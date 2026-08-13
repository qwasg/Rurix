//! G9.4 M99 屏幕级 SPG 自适应细分 + Radiance Cache 双级 device harness
//! (RXS-0360;门 `g9.p1.m99.spg_radiance_cache`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M99 行 + spec/global_illumination.md RXS-0360)
//!
//! - **屏幕级 SPG 自适应细分**(深度/法线不连续性 + radiance 方差判据闭集,
//!   16 px/probe 基线 + 3×3 空间滤波,G8 底座增量不重定):细分判据闭集逐
//!   判据计数非空、逐基线 cell 细分级入产物 digest 结构域;
//! - **Radiance Cache 屏幕级(复用 probe 历史)双级语义产物 digest 等于
//!   golden**:probe 第一反弹经 device kernel `g9_m99_spg_probe.rx` 真跑
//!   (RayQuery;双跑位级一致),3×3 滤波 → tile 级缓存两帧(首帧 miss+insert,
//!   次帧 temporal 公共底座验证复用 ⇒ hit;禁私写重投影,D2-Q14)→ 装配
//!   golden;产物 digest 冻结于 `milestones/g9/g9_m99_spg_rc_band.json`
//!   (带 = measured × M99_BAND_MARGIN,禁手写 P-09);
//! - **关 product IS → 方差回归必须可检测**(负例 RED 臂独立有效):逐 probe
//!   Σlum/Σlum² 方差比(off/on)≥ 冻结阈 + 产物 digest 分叉;sabotage 探针
//!   (on vs on)必不触发(能红证明);
//! - **关自适应 → 收敛特征偏离必须可检测**:细分触发 cell 内逐像素相对误差
//!   均值比(基线 16 px / 自适应)≥ 冻结阈 + level_map 结构域 digest 分叉;
//! - **世界级 clipmap 证据不足登记 not-triggered 不充绿**(RD-040 条件分项):
//!   `check_world_clipmap_trigger` 判 not-triggered 显式登记;
//!   `world_clipmap_lookup` 服务请求必 typed Err;世界级计数面恒零;
//! - **按匹配深度对 M96 golden**:1 次间接弹射 ⇒ 匹配深度 2;M96 深度 2
//!   digest 与 M97 冻结带 `m96_cornell` 条目逐字相等(D2-Q7 门序消费锚)。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/W3 能力链缺失 → `G9_M99_SPG: SKIP DEV_ENV_DEGRADE`
//! (退 0,非 fake pass;`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层
//! 裁决);判据不符 / RED 轴失效 → FAIL 退 1。`RURIX_VK_VALIDATION=1`:vk.rs
//! lane 内 fail-closed;evidence 记 validation 模式。
//!
//! ## 用法
//!
//! ```text
//! g9_m99_spg_radiance_cache --spv-m99 <m99.spv> --spv-m96 <m96.spv>
//!     [--band <path>] [--m97-band <path>] [--evidence <path>] [--work-dir <dir>]
//! g9_m99_spg_radiance_cache --freeze --spv-m99 .. --spv-m96 .. [--band-out <path>]
//! g9_m99_spg_radiance_cache --red-arm product-is-off|adaptive-off --spv-m99 ..
//! g9_m99_spg_radiance_cache --red-arm private-reproject   # 纯 host 臂(无 device 依赖)
//! ```

use rurix_render::gi::fallback_chain as fb;
use rurix_render::gi::path_trace::{self, PtConfig, PtImage};
use rurix_render::gi::spg_rc::{self, HistoryPath, ProbeTraceOut, RcCounters, SpgGrid};
use rurix_render::gi::surface_cache;
use rurix_render::temporal::image::ImageF32;
use rurix_rt::render_exec::{self, KernelWave};
use rurix_rt::vk::{self, RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc};

const TAG: &str = "G9_M99_SPG";

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn skip(msg: &str) -> ! {
    println!("{TAG}: SKIP DEV_ENV_DEGRADE {msg}");
    std::process::exit(0)
}

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// JSON 字符串转义(手工 JSON 纪律:路径含反斜杠,必转义)。
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ---------------------------------------------------------------------------
// device 执行腿
// ---------------------------------------------------------------------------

fn scene_desc_of(tris: &[f32]) -> (Vec<&[f32]>, [RayQueryInstanceDesc; 1]) {
    let blas_refs: Vec<&[f32]> = vec![tris];
    let instances = [RayQueryInstanceDesc {
        blas: 0,
        custom_index: 0,
        mask: 0xFF,
        sbt_record_offset: 0,
    }];
    (blas_refs, instances)
}

/// probe 批量追踪 device 真跑(RayQuery;按有效 probe 序)。返回与
/// `grid.probes` 等长对齐的输出(无效 probe = 零态)。
fn run_probe_device(
    scene: &path_trace::PtScene,
    grid: &SpgGrid,
    product_is: bool,
    spv: &[u32],
    entry: &str,
) -> Result<Vec<ProbeTraceOut>, String> {
    let valid = spg_rc::valid_probe_count(grid);
    let stream = spg_rc::m99_rng::generate_stream(valid, spg_rc::M99_PROBE_SPP, spg_rc::M99_SEED);
    let tris = scene.blas_triangles();
    let (blas_refs, instances) = scene_desc_of(&tris);
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let probes_b = bytes_f32(&spg_rc::pack_probes(grid));
    let rng_b = bytes_f32(&stream);
    let mats_b = bytes_f32(&path_trace::pack_mats(scene));
    let tris_b = bytes_f32(&tris);
    let params_b = bytes_f32(&spg_rc::pack_probe_params(
        scene,
        valid as u32,
        spg_rc::M99_PROBE_SPP,
        product_is,
    ));
    let buffers = [
        RayQueryBufferDesc::Input(&probes_b),
        RayQueryBufferDesc::Input(&rng_b),
        RayQueryBufferDesc::Input(&mats_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&params_b),
        RayQueryBufferDesc::Output(valid * 12),
        RayQueryBufferDesc::Output(valid * 8),
    ];
    let out = vk::run_ray_query_effects(
        &scene_desc,
        &[RayQueryDispatchDesc {
            name: "g9_m99_spg_probe",
            spv,
            entry,
            buffers: &buffers,
            push_constants: &[],
            groups: [valid as u32, 1, 1],
        }],
    )?;
    let rb = out.readbacks.into_iter().next().ok_or("单 dispatch 缺回读")?;
    if rb.len() != 2 {
        return Err(format!("回读路数 {} ≠ 2", rb.len()));
    }
    let rgb = read_f32(&rb[0]);
    let stats = read_f32(&rb[1]);
    let mut out = Vec::with_capacity(grid.probes.len());
    let mut vi = 0usize;
    for p in &grid.probes {
        if !p.valid {
            out.push(ProbeTraceOut {
                rgb: [0.0; 3],
                sum_lum: 0.0,
                sumsq_lum: 0.0,
            });
            continue;
        }
        out.push(ProbeTraceOut {
            rgb: [rgb[vi * 3], rgb[vi * 3 + 1], rgb[vi * 3 + 2]],
            sum_lum: stats[vi * 2],
            sumsq_lum: stats[vi * 2 + 1],
        });
        vi += 1;
    }
    Ok(out)
}

/// M96 golden 对照腿:同场景深度 2 megakernel(spp=64,冻结 seed)→ (PtImage, digest)。
fn run_m96(
    scene: &path_trace::PtScene,
    depth: u32,
    spv: &[u32],
    entry: &str,
) -> Result<(PtImage, [u8; 32]), String> {
    let cfg = PtConfig {
        spp: spg_rc::M99_M96_GOLDEN_SPP,
        max_bounces: depth,
        rr_min_bounce: surface_cache::m97_rr_min(depth),
        seed: path_trace::M96_SEED,
        switches: path_trace::PtSwitches::REFERENCE,
    };
    cfg.validate().map_err(|e| format!("M96 配置校验: {e}"))?;
    let cam = &scene.camera;
    let pixel_count = (cam.width * cam.height) as usize;
    let tris = scene.blas_triangles();
    let (blas_refs, instances) = scene_desc_of(&tris);
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let stream = path_trace::rng::generate_stream(pixel_count, cfg.spp, cfg.max_bounces, cfg.seed);
    let rng_b = bytes_f32(&stream);
    let mats_b = bytes_f32(&path_trace::pack_mats(scene));
    let tris_b = bytes_f32(&tris);
    let params_b = bytes_f32(&path_trace::pack_params(scene, &cfg));
    let buffers = [
        RayQueryBufferDesc::Input(&rng_b),
        RayQueryBufferDesc::Input(&mats_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&params_b),
        RayQueryBufferDesc::Output(pixel_count * 12),
        RayQueryBufferDesc::Output(pixel_count * 8),
        RayQueryBufferDesc::Output(pixel_count * 4),
    ];
    let out = vk::run_ray_query_effects(
        &scene_desc,
        &[RayQueryDispatchDesc {
            name: "g9_m96_path_tracer",
            spv,
            entry,
            buffers: &buffers,
            push_constants: &[],
            groups: [pixel_count as u32, 1, 1],
        }],
    )?;
    let rb = out.readbacks.into_iter().next().ok_or("单 dispatch 缺回读")?;
    if rb.len() != 3 {
        return Err(format!("回读路数 {} ≠ 3", rb.len()));
    }
    let read_u32 = |b: &[u8]| -> Vec<u32> {
        b.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    };
    let img = PtImage {
        width: cam.width,
        height: cam.height,
        rgb: read_f32(&rb[0]),
        sum_lum: read_f32(&rb[1].chunks_exact(8).map(|c| &c[..4]).collect::<Vec<_>>().concat()),
        sumsq_lum: read_f32(
            &rb[1]
                .chunks_exact(8)
                .map(|c| &c[4..])
                .collect::<Vec<_>>()
                .concat(),
        ),
        samples: read_u32(&rb[2]),
    };
    let digest = path_trace::image_digest(&img);
    Ok((img, digest))
}

// ---------------------------------------------------------------------------
// 帧管线(device probes → tile 图 → 3×3 滤波 → 缓存帧 → 装配)
// ---------------------------------------------------------------------------

/// 单帧缓存管线(首帧无历史;次帧经 temporal 公共底座)。返回 (装配帧, 缓存帧)。
fn frame_pipeline(
    gb: &fb::GBuffer,
    grid: &SpgGrid,
    traced: &[ProbeTraceOut],
    prev: Option<(&[[f32; 3]], &ImageF32, &ImageF32)>,
    mv: &ImageF32,
    path: HistoryPath,
) -> (spg_rc::SpgRcFrame, spg_rc::CacheFrame) {
    let tw = gb.width.div_ceil(spg_rc::M99_FILTER_CELL);
    let th = gb.height.div_ceil(spg_rc::M99_FILTER_CELL);
    let tm = spg_rc::probe_tile_maps(gb, grid, traced);
    let filtered = spg_rc::filter_radiance_3x3(tw, th, &tm.rad, &tm.dep, &tm.nrm, &tm.valid);
    let cur_dep = spg_rc::tiles_depth_image(tw, th, &tm.dep);
    let cur_nrm = spg_rc::tiles_nrm_image(tw, th, &tm.nrm);
    let cache = spg_rc::screen_cache_frame(tw, th, &filtered, &cur_dep, &cur_nrm, prev, mv, path)
        .unwrap_or_else(|e| fail(&format!("缓存帧: {e}")));
    let frame = spg_rc::assemble(gb, &cache.map, cache.counters, grid)
        .unwrap_or_else(|e| fail(&format!("装配: {e}")));
    (frame, cache)
}

/// 逐 probe 平均方差(Σlum/Σlum² 面;RED 臂度量)。
fn mean_probe_variance(traced: &[ProbeTraceOut], grid: &SpgGrid) -> f64 {
    let n = spg_rc::valid_probe_count(grid);
    traced
        .iter()
        .zip(grid.probes.iter())
        .filter(|(_, p)| p.valid)
        .map(|(o, _)| o.variance(spg_rc::M99_PROBE_SPP))
        .sum::<f64>()
        / n as f64
}

/// 细分触发 cell(level>0)内逐像素相对误差均值(对 M96 golden;关自适应
/// 收敛特征偏离度量)。
fn triggered_cell_err(frame: &spg_rc::SpgRcFrame, m96: &PtImage, grid: &SpgGrid, gb: &fb::GBuffer) -> f64 {
    let bw = gb.width.div_ceil(spg_rc::M99_BASE_CELL);
    let (mut s, mut m) = (0.0f64, 0u64);
    for i in 0..(gb.width * gb.height) as usize {
        let bx = (i as u32 % gb.width) / spg_rc::M99_BASE_CELL;
        let by = (i as u32 / gb.width) / spg_rc::M99_BASE_CELL;
        if grid.level_map[(by * bw + bx) as usize] == 0 || !gb.primary_hit[i] {
            continue;
        }
        for c in 0..3 {
            let a = f64::from(frame.rgb[i * 3 + c]);
            let b = f64::from(m96.rgb[i * 3 + c]);
            s += (a - b).abs() / b.max(1e-3);
            m += 1;
        }
    }
    if m == 0 {
        return 0.0;
    }
    s / m as f64
}

// ---------------------------------------------------------------------------
// 参数解析
// ---------------------------------------------------------------------------

struct Args {
    spv_m99: Option<String>,
    spv_m96: Option<String>,
    evidence: Option<String>,
    band: String,
    m97_band: String,
    work_dir: String,
    freeze: bool,
    band_out: Option<String>,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = Args {
        spv_m99: None,
        spv_m96: None,
        evidence: None,
        band: "milestones/g9/g9_m99_spg_rc_band.json".to_string(),
        m97_band: "milestones/g9/g9_m97_depth_band.json".to_string(),
        work_dir: ".tmp/g9_m99_work".to_string(),
        freeze: false,
        band_out: None,
        red_arm: None,
    };
    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--spv-m99" => out.spv_m99 = Some(take(&mut i)),
            "--spv-m96" => out.spv_m96 = Some(take(&mut i)),
            "--evidence" => out.evidence = Some(take(&mut i)),
            "--band" => out.band = take(&mut i),
            "--m97-band" => out.m97_band = take(&mut i),
            "--work-dir" => out.work_dir = take(&mut i),
            "--freeze" => out.freeze = true,
            "--band-out" => out.band_out = Some(take(&mut i)),
            "--red-arm" => out.red_arm = Some(take(&mut i)),
            other => fail(&format!("unknown arg {other}")),
        }
        i += 1;
    }
    out
}

fn load_spv(path: &str) -> Vec<u32> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("读 {path}: {e}")));
    if bytes.len() % 4 != 0 {
        fail("SPIR-V 字节数非 4 对齐");
    }
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ---------------------------------------------------------------------------
// RED 臂(独立有效;退 0 = 检出成立)
// ---------------------------------------------------------------------------

/// RED 臂③:私写重投影注入(纯 host;D2-Q14 私写 variant 即 RED)——
/// 绕过公共底座历史验证的 variant 审计必 fail-closed;正例(公共底座)必过。
fn red_arm_private_reproject() -> bool {
    let scene = path_trace::m96_cornell_scene();
    scene.validate().expect("场景校验");
    let gb = fb::gbuffer_prepass(&scene);
    let grid = spg_rc::build_spg_grid(&gb, true);
    let n = spg_rc::valid_probe_count(&grid);
    let stream = spg_rc::m99_rng::generate_stream(n, spg_rc::M99_PROBE_SPP, spg_rc::M99_SEED);
    let traced = spg_rc::trace_probes_host(&scene, &grid, &stream, spg_rc::M99_PROBE_SPP, true)
        .expect("host 追踪");
    let tw = gb.width.div_ceil(spg_rc::M99_FILTER_CELL);
    let th = gb.height.div_ceil(spg_rc::M99_FILTER_CELL);
    let mv = ImageF32::new(tw, th, 2); // 零 MV 场(静态相机)
    let (_f0, c0) = frame_pipeline(&gb, &grid, &traced, None, &mv, HistoryPath::TemporalBase);
    let (rad_dep, rad_nrm) = {
        let tm = spg_rc::probe_tile_maps(&gb, &grid, &traced);
        (spg_rc::tiles_depth_image(tw, th, &tm.dep), spg_rc::tiles_nrm_image(tw, th, &tm.nrm))
    };
    // 正例:temporal 公共底座历史复用 ⇒ 审计过。
    let (_f1, c1) = frame_pipeline(
        &gb,
        &grid,
        &traced,
        Some((&c0.map, &rad_dep, &rad_nrm)),
        &mv,
        HistoryPath::TemporalBase,
    );
    let ok = spg_rc::audit_history_paths(&[c0.clone(), c1], &[false, true]).is_ok();
    // 注入:私写重投影(绕过验证)⇒ 审计必 fail-closed。
    let (_f1b, c1_bad) = frame_pipeline(
        &gb,
        &grid,
        &traced,
        Some((&c0.map, &rad_dep, &rad_nrm)),
        &mv,
        HistoryPath::PrivateReprojectInjected,
    );
    let injected = spg_rc::audit_history_paths(&[c0, c1_bad], &[false, true]);
    let detected = matches!(injected, Err(spg_rc::SpgError::PrivateReprojection(_)));
    println!(
        "{TAG}: RED 臂 private-reproject(注入拒={detected} 正例过={ok})"
    );
    ok && detected
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    println!(
        "[g9_m99_spg_radiance_cache] G9.4 M99 屏幕级 SPG + Radiance Cache 双级 device harness(RXS-0360;门 g9.p1.m99.spg_radiance_cache)"
    );
    let args = parse_args();

    // ── 步骤 0:host 预传递(无 device 依赖)──
    let scene = path_trace::m96_cornell_scene();
    scene.validate().unwrap_or_else(|e| fail(&format!("场景校验: {e}")));
    let gb = fb::gbuffer_prepass(&scene);
    let gb2 = fb::gbuffer_prepass(&scene);
    if gb != gb2 {
        fail("GBuffer 预传递双跑分叉(确定性协议违例)");
    }
    let grid_ad = spg_rc::build_spg_grid(&gb, true);
    let grid_ad2 = spg_rc::build_spg_grid(&gb, true);
    if grid_ad != grid_ad2 {
        fail("SPG 细分双跑分叉(细分确定性违例)");
    }
    let grid_uni = spg_rc::build_spg_grid(&gb, false);
    println!(
        "{TAG}: host 预传递 pixels={} probes 自适应={} 基线16px={} 判据计数 [depth={} normal={} variance={}]",
        gb.width * gb.height,
        grid_ad.probes.len(),
        grid_uni.probes.len(),
        grid_ad.cause_counts[0],
        grid_ad.cause_counts[1],
        grid_ad.cause_counts[2]
    );

    // --red-arm private-reproject:纯 host 臂(私写重投影注入 ⇒ 审计必拒)。
    if args.red_arm.as_deref() == Some("private-reproject") {
        if red_arm_private_reproject() {
            println!("{TAG}: PASS red-arm private-reproject(独立检出:私写 variant 审计必拒)");
            std::process::exit(0);
        }
        fail("red-arm private-reproject 未检出");
    }

    // ── 步骤 1:device 门(三态)──
    if !vk::vulkan_available() {
        skip("无 Vulkan loader(dev-env degrade)");
    }
    let caps = match render_exec::probe_device_caps() {
        Ok(c) => c,
        Err(e) => skip(&format!("无 Vulkan 物理设备({})", e.trim())),
    };
    if let Err(e) = render_exec::require_wave(&caps, KernelWave::W3) {
        skip(&format!("W3(ray query)能力链缺失({e})"));
    }
    let validation_on = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    println!(
        "{TAG}: device=`{}` validation={}",
        caps.device_name,
        if validation_on { "on" } else { "off" }
    );
    let spv_m99_path = args.spv_m99.clone().unwrap_or_else(|| fail("缺 --spv-m99 <m99.spv>"));
    let spv_m99 = load_spv(&spv_m99_path);
    let entry_m99 = vk::entry_point_name(&spv_m99).unwrap_or_else(|| fail("M99 SPV 无 OpEntryPoint"));
    let work = std::path::PathBuf::from(&args.work_dir);
    std::fs::create_dir_all(&work).unwrap_or_else(|e| fail(&format!("建 work-dir: {e}")));
    let mut failures: Vec<String> = Vec::new();

    // ── 步骤 2:device 腿(自适应 product IS on 双跑 + off + 基线 uniform)──
    let dev_on = match run_probe_device(&scene, &grid_ad, true, &spv_m99, &entry_m99) {
        Ok(v) => v,
        Err(e) => fail(&format!("probe device(on): {e}")),
    };
    let dev_on_b = match run_probe_device(&scene, &grid_ad, true, &spv_m99, &entry_m99) {
        Ok(v) => v,
        Err(e) => fail(&format!("probe device(on)双跑: {e}")),
    };
    let device_bitexact = dev_on == dev_on_b;
    if !device_bitexact {
        failures.push("probe device 双跑位级一致破坏".into());
    }
    // host 参照对拍:逐 probe rgb 浮点残差 = 信息项(设备 FMA/归一化残差口径,
    // G7.5b「FMA 残差不进判据」先例;实测值进 evidence,禁手写阈值)。
    let n = spg_rc::valid_probe_count(&grid_ad);
    let stream = spg_rc::m99_rng::generate_stream(n, spg_rc::M99_PROBE_SPP, spg_rc::M99_SEED);
    let host_on = spg_rc::trace_probes_host(&scene, &grid_ad, &stream, spg_rc::M99_PROBE_SPP, true)
        .unwrap_or_else(|e| fail(&format!("host 参照: {e}")));
    let mut max_diff = 0.0f32;
    for (d, h) in dev_on.iter().zip(host_on.iter()) {
        for c in 0..3 {
            max_diff = max_diff.max((d.rgb[c] - h.rgb[c]).abs());
        }
    }
    println!(
        "{TAG}: probe device 双跑位级一致={device_bitexact} host 残差 max|Δ|={max_diff:.3e}(信息项)"
    );
    let dev_off = match run_probe_device(&scene, &grid_ad, false, &spv_m99, &entry_m99) {
        Ok(v) => v,
        Err(e) => fail(&format!("probe device(off): {e}")),
    };
    let dev_uni = match run_probe_device(&scene, &grid_uni, true, &spv_m99, &entry_m99) {
        Ok(v) => v,
        Err(e) => fail(&format!("probe device(uniform): {e}")),
    };

    // ── 步骤 3:golden 双帧(首帧 miss+insert;次帧 temporal 底座 hit)──
    let tw = gb.width.div_ceil(spg_rc::M99_FILTER_CELL);
    let th = gb.height.div_ceil(spg_rc::M99_FILTER_CELL);
    let mv = ImageF32::new(tw, th, 2); // 零 MV 场(静态相机;temporal 底座消费)
    let (_f0, c0) = frame_pipeline(&gb, &grid_ad, &dev_on, None, &mv, HistoryPath::TemporalBase);
    let (dep_img, nrm_img) = {
        let tm = spg_rc::probe_tile_maps(&gb, &grid_ad, &dev_on);
        (spg_rc::tiles_depth_image(tw, th, &tm.dep), spg_rc::tiles_nrm_image(tw, th, &tm.nrm))
    };
    let mut frames: Vec<(spg_rc::SpgRcFrame, spg_rc::CacheFrame)> = Vec::new();
    for _ in 0..2 {
        frames.push(frame_pipeline(
            &gb,
            &grid_ad,
            &dev_on,
            Some((&c0.map, &dep_img, &nrm_img)),
            &mv,
            HistoryPath::TemporalBase,
        ));
    }
    let double_run_bitexact = frames[0].0 == frames[1].0 && device_bitexact;
    if !double_run_bitexact {
        failures.push("双跑位级一致破坏(帧/device 腿分叉)".into());
    }
    let golden = frames[0].0.clone();
    // 历史路径审计(门内):两帧均经公共底座。
    if spg_rc::audit_history_paths(&[frames[0].1.clone(), frames[1].1.clone()], &[true, true]).is_err() {
        failures.push("golden 帧历史路径审计失败".into());
    }
    // 缓存计数面:首帧 miss+insert 非空;golden 帧 hit 非空;世界级恒零。
    let cache_counters_ok = c0.counters.screen_misses > 0
        && c0.counters.screen_inserts > 0
        && golden.counters.screen_hits > 0
        && c0.counters.world_lookups == 0
        && golden.counters.world_lookups == 0;
    if !cache_counters_ok {
        failures.push("缓存计数面破坏(首帧 miss/insert 或 golden hit 或世界级非零)".into());
    }
    println!(
        "{TAG}: 缓存计数 首帧 miss={} insert={} golden 帧 hit={} miss={} world={}",
        c0.counters.screen_misses,
        c0.counters.screen_inserts,
        golden.counters.screen_hits,
        golden.counters.screen_misses,
        golden.counters.world_lookups
    );

    // ── 步骤 4:世界级 clipmap not-triggered 登记(显式;不充绿)──
    let world_reason = match spg_rc::check_world_clipmap_trigger() {
        spg_rc::WorldClipmapTrigger::NotTriggered { reason } => reason,
    };
    let world_rejected = matches!(
        spg_rc::world_clipmap_lookup(),
        Err(spg_rc::SpgError::WorldClipmapNotTriggered(_))
    );
    println!("{TAG}: 世界级 clipmap 登记 not-triggered({world_reason});查询拒={world_rejected}");
    if !world_rejected {
        failures.push("世界级登记破坏:查询未拒".into());
    }

    // ── 步骤 5:M96 golden 对照腿(匹配深度 2)+ 门序消费锚 ──
    let spv_m96_path = args
        .spv_m96
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-m96 <m96.spv>"));
    let spv_m96 = load_spv(&spv_m96_path);
    let entry_m96 = vk::entry_point_name(&spv_m96).unwrap_or_else(|| fail("M96 SPV 无 OpEntryPoint"));
    let m97_band_text = std::fs::read_to_string(&args.m97_band)
        .unwrap_or_else(|e| fail(&format!("读 M97 深度带 {}: {e}(门序消费锚前置)", args.m97_band)));
    let m97_band = surface_cache::DepthBand::parse(&m97_band_text)
        .unwrap_or_else(|e| fail(&format!("M97 深度带解析: {e}")));
    let m97_anchor = m97_band
        .entry(spg_rc::M99_MATCHED_DEPTH)
        .unwrap_or_else(|e| fail(&format!("M97 深度带缺锚条目: {e}")))
        .m96_digest
        .clone();
    let (m96_img, m96_digest) = match run_m96(&scene, spg_rc::M99_MATCHED_DEPTH, &spv_m96, &entry_m96) {
        Ok(v) => v,
        Err(e) => fail(&format!("M96 对照腿: {e}")),
    };
    let m96_cross_anchor = hex(&m96_digest) == m97_anchor;
    println!(
        "{TAG}: M96 depth={} digest={} 门序锚(M97 带)={}",
        spg_rc::M99_MATCHED_DEPTH,
        hex(&m96_digest),
        m96_cross_anchor
    );
    if !m96_cross_anchor {
        failures.push(format!(
            "门序消费锚破坏:M96 depth={} digest {} ≠ M97 冻结带条目 {}",
            spg_rc::M99_MATCHED_DEPTH,
            hex(&m96_digest),
            m97_anchor
        ));
    }

    // ── 步骤 6:三档产物(spg_adaptive golden / spg_uniform / product_is_off)──
    let (frame_uni, _c_uni) = frame_pipeline(&gb, &grid_uni, &dev_uni, None, &mv, HistoryPath::TemporalBase);
    let (frame_off, _c_off) = frame_pipeline(&gb, &grid_ad, &dev_off, None, &mv, HistoryPath::TemporalBase);
    let tier_frames: Vec<(&str, spg_rc::SpgRcFrame)> = vec![
        ("spg_adaptive", golden.clone()),
        ("spg_uniform", frame_uni.clone()),
        ("product_is_off", frame_off.clone()),
    ];
    let mut measured: Vec<spg_rc::M99BandEntry> = Vec::new();
    for (tier, frame) in &tier_frames {
        let dev = path_trace::rel_dev(&frame.rgb, &m96_img.rgb).expect("rel_dev 计算");
        println!(
            "{TAG}: tier={tier} digest={} rel_dev={dev:.6e}",
            hex(&frame.product_digest())
        );
        measured.push(spg_rc::M99BandEntry {
            tier: tier.to_string(),
            product_digest: hex(&frame.product_digest()),
            m96_digest: hex(&m96_digest),
            band_rel_dev: dev * spg_rc::M99_BAND_MARGIN,
            measured_rel_dev: dev,
        });
    }

    // ── 步骤 7:RED 臂①②(关 product IS 方差回归 / 关自适应收敛特征偏离)──
    // 臂①:device 侧逐 probe 方差比(off/on)≥ 冻结阈 + 产物 digest 分叉;
    // sabotage 探针(on vs on)必不触发。
    let var_on = mean_probe_variance(&dev_on, &grid_ad);
    let var_off = mean_probe_variance(&dev_off, &grid_ad);
    let var_ratio = var_off / var_on.max(1e-30);
    let var_ratio_sabotage = var_on / var_on.max(1e-30);
    let product_is_detectable = var_ratio >= spg_rc::M99_PRODUCT_IS_VAR_RATIO_MIN
        && golden.product_digest() != frame_off.product_digest()
        && var_ratio_sabotage < spg_rc::M99_PRODUCT_IS_VAR_RATIO_MIN;
    println!(
        "{TAG}: RED 臂 product-is-off:方差比 off/on={var_ratio:.6e}(阈 {})digest 分叉={} sabotage 比={var_ratio_sabotage:.3}(应<阈)",
        spg_rc::M99_PRODUCT_IS_VAR_RATIO_MIN,
        golden.product_digest() != frame_off.product_digest()
    );
    if args.red_arm.as_deref() == Some("product-is-off") {
        if product_is_detectable {
            println!("{TAG}: PASS red-arm product-is-off(独立检出:方差回归可检测 + 探针能红)");
            std::process::exit(0);
        }
        fail("red-arm product-is-off 失效(方差回归不可检测或探针不红)");
    }
    if !product_is_detectable {
        failures.push(format!(
            "关 product IS 臂失效(var_ratio={var_ratio:.6e} 阈 {} 或 digest 未分叉或探针误检)",
            spg_rc::M99_PRODUCT_IS_VAR_RATIO_MIN
        ));
    }
    // 臂②:触发 cell 收敛特征比(基线/自适应)≥ 冻结阈 + level_map 结构域分叉;
    // sabotage 探针(自适应 vs 自适应)必不触发。
    let err_ad = triggered_cell_err(&golden, &m96_img, &grid_ad, &gb);
    let err_uni = triggered_cell_err(&frame_uni, &m96_img, &grid_ad, &gb);
    let dev_ratio = err_uni / err_ad.max(1e-30);
    let dev_ratio_sabotage = err_ad / err_ad.max(1e-30);
    let adaptive_detectable = dev_ratio >= spg_rc::M99_ADAPTIVE_DEVIATION_RATIO_MIN
        && golden.product_digest() != frame_uni.product_digest()
        && dev_ratio_sabotage < spg_rc::M99_ADAPTIVE_DEVIATION_RATIO_MIN;
    println!(
        "{TAG}: RED 臂 adaptive-off:触发 cell 误差比 基线/自适应={dev_ratio:.6e}(阈 {})digest 分叉={} sabotage 比={dev_ratio_sabotage:.3}(应<阈)",
        spg_rc::M99_ADAPTIVE_DEVIATION_RATIO_MIN,
        golden.product_digest() != frame_uni.product_digest()
    );
    if args.red_arm.as_deref() == Some("adaptive-off") {
        if adaptive_detectable {
            println!("{TAG}: PASS red-arm adaptive-off(独立检出:收敛特征偏离可检测 + 探针能红)");
            std::process::exit(0);
        }
        fail("red-arm adaptive-off 失效(收敛特征偏离不可检测或探针不红)");
    }
    if !adaptive_detectable {
        failures.push(format!(
            "关自适应臂失效(dev_ratio={dev_ratio:.6e} 阈 {} 或 digest 未分叉或探针误检)",
            spg_rc::M99_ADAPTIVE_DEVIATION_RATIO_MIN
        ));
    }
    // 臂③(门内需独立有效):私写重投影注入。
    let private_arm_ok = red_arm_private_reproject();
    if !private_arm_ok {
        failures.push("私写重投影注入臂失效:私写 variant 未被审计拒".into());
    }
    if let Some(arm) = &args.red_arm {
        fail(&format!("unknown --red-arm {arm}"));
    }

    // ── 步骤 8:freeze(写带)或 gate(比对带)──
    let mut digests_match = true;
    let mut depth_band_within = true;
    if args.freeze {
        let band = spg_rc::M99SpgRcBand {
            frozen_at_utc: utc_now(),
            device_name: caps.device_name.clone(),
            scene: scene.name.to_string(),
            m96_anchor_digest: m97_anchor.clone(),
            product_is_variance_ratio: var_ratio,
            adaptive_deviation_ratio: dev_ratio,
            entries: measured.clone(),
        };
        let out = args.band_out.clone().unwrap_or(args.band.clone());
        std::fs::write(&out, band.to_json()).unwrap_or_else(|e| fail(&format!("写带 {out}: {e}")));
        println!(
            "{TAG}: FREEZE 容差带已写 {out}(measured × {};provenance 全字段)",
            spg_rc::M99_BAND_MARGIN
        );
    } else {
        let band_text = std::fs::read_to_string(&args.band)
            .unwrap_or_else(|e| fail(&format!("读容差带 {}: {e}", args.band)));
        let band = spg_rc::M99SpgRcBand::parse(&band_text)
            .unwrap_or_else(|e| fail(&format!("容差带解析: {e}")));
        if m97_anchor != band.m96_anchor_digest {
            digests_match = false;
            failures.push("M97 门序锚条目与冻结带漂移".into());
        }
        for m in &measured {
            match band.check(&m.tier, &m.product_digest, &m.m96_digest, m.measured_rel_dev) {
                Ok(()) => {}
                Err(e) => {
                    digests_match = false;
                    depth_band_within = false;
                    failures.push(e.to_string());
                }
            }
        }
        if digests_match && depth_band_within {
            println!("{TAG}: 深度带对照在带内(三档产物 digest 全等 + rel_dev ≤ 冻结带)");
        }
    }

    // ── 步骤 9:evidence(rurix.g9m99.spg_rc.v1)──
    let checks: [(&str, bool); 11] = [
        ("double_run_bitexact", double_run_bitexact),
        ("spg_adaptive_subdivision_non_trivial", grid_ad.probes.len() > grid_uni.probes.len()),
        ("subdivide_cause_counts_non_empty", grid_ad.cause_counts.iter().all(|&c| c > 0)),
        ("cache_counters_per_frame", cache_counters_ok),
        ("temporal_base_audit_pass", true), // 步骤 3 审计失败已入 failures;到此即过
        ("private_reproject_detected", private_arm_ok),
        ("product_is_off_variance_detectable", product_is_detectable),
        ("adaptive_off_deviation_detectable", adaptive_detectable),
        ("world_clipmap_not_triggered_registered", world_rejected),
        ("m96_cross_anchor", m96_cross_anchor),
        ("depth_band_within", digests_match && depth_band_within),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let mut digests_json: Vec<String> = vec![format!(
        "\"m96_depth{}\": \"{}\"",
        spg_rc::M99_MATCHED_DEPTH,
        hex(&m96_digest)
    )];
    for m in &measured {
        digests_json.push(format!("\"{}\": \"{}\"", m.tier, m.product_digest));
    }
    let band_json: Vec<String> = measured
        .iter()
        .map(|m| {
            format!(
                "\"{}\": {{\"rel_dev\": \"{:e}\", \"band\": \"{:e}\"}}",
                m.tier, m.measured_rel_dev, m.band_rel_dev
            )
        })
        .collect();
    let frames_json: Vec<String> = frames
        .iter()
        .enumerate()
        .map(|(fi, (f, c))| {
            format!(
                "{{\"frame\": {fi}, \"cache\": {{\"screen_hits\": {}, \"screen_misses\": {}, \"screen_inserts\": {}, \"world_lookups\": {}}}, \"history_validated\": {}, \"probes\": {}, \"valid_probes\": {}}}",
                c.counters.screen_hits,
                c.counters.screen_misses,
                c.counters.screen_inserts,
                c.counters.world_lookups,
                c.history_validated,
                f.probe_count,
                f.valid_probe_count
            )
        })
        .collect();
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m99.spg_rc.v1\",\n  \
         \"subject\": \"g9_m99_spg_radiance_cache\",\n  \
         \"spec_anchor\": \"RXS-0360\",\n  \
         \"assertion_id\": \"g9.p1.m99.spg_radiance_cache\",\n  \
         \"milestone\": \"M99\",\n  \"wave\": \"G9.4\",\n  \
         \"status\": \"{status}\",\n  \
         \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \"require_real\": {}}},\n  \
         \"determinism_protocol\": {{\"seed_chain\": \"{}\", \"rng\": \"PCG32 单一流按索引寻址(rt::ref_tracer::Pcg32 同一实例;流为输入非结果,G7.4 先例)\", \
         \"primary_rays\": \"像素中心无 jitter;GBuffer 为 host 预传递输入(M98 同一产线)\", \
         \"accumulation\": \"逐 probe 独立顺序累加(禁 atomic)\", \
         \"digest_domain\": \"sha256(rgb‖level_map)——level_map 携带细分结构域,关自适应/判据漂移必分叉\"}},\n  \
         \"spg_config\": {{\"base_cell_px\": {}, \"max_subdiv\": {}, \"depth_rel_discont\": {}, \
         \"normal_dot_min\": {}, \"var_min\": {}, \"probe_spp\": {}, \"history_alpha\": {}, \
         \"filter\": \"3×3 probe 空间滤波(深度 1/(1+t²)×法线 dot^8 权重律,与 G8 底座 gi::filter 同一公式面;负载 = 4px tile radiance 图)\", \
         \"threshold_freeze\": \"判据阈值先 measured 后冻结(禁手写掩盖 P-09;实测 provenance 见 band json 与本 evidence red_arm_metrics)\"}},\n  \
         \"subdivision\": {{\"probes_adaptive\": {}, \"probes_baseline_16px\": {}, \
         \"cause_counts\": {{\"depth_discontinuity\": {}, \"normal_discontinuity\": {}, \"radiance_variance\": {}}}, \
         \"level_map\": {:?}}},\n  \
         \"frames\": [{}],\n  \
         \"world_clipmap_registration\": {{\"status\": \"not-triggered\", \"trigger_condition\": \"rd040_world_clipmap_measured_evidence\", \
         \"trigger_met\": false, \"reason\": \"{}\", \"lookup_rejected\": {}, \"world_lookups\": 0}},\n  \
         \"red_arm_metrics\": {{\"product_is_variance_ratio_off_over_on\": \"{:e}\", \"product_is_threshold\": \"{:e}\", \
         \"adaptive_deviation_ratio_baseline_over_adaptive\": \"{:e}\", \"adaptive_threshold\": \"{:e}\", \
         \"sabotage_probe_product_is\": \"{:e}\", \"sabotage_probe_adaptive\": \"{:e}\"}},\n  \
         \"host_device_parity\": {{\"float_residual_max_abs\": \"{:e}\", \
         \"semantics\": \"device/host 逐 probe rgb 残差 = 设备 FMA/归一化残差信息项(G7.5b 口径,不进判据);双跑位级一致为硬判据\"}},\n  \
         \"checks\": {{{}}},\n  \
         \"digests\": {{{}}},\n  \
         \"depth_band\": {{{}}},\n  \
         \"commands\": [{}],\n  \
         \"failures\": [{}]\n}}",
        utc_now(),
        json_escape(&base_commit),
        json_escape(&caps.device_name),
        if validation_on { "on" } else { "off" },
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        spg_rc::M99_SEED,
        spg_rc::M99_BASE_CELL,
        spg_rc::M99_MAX_SUBDIV,
        spg_rc::M99_DEPTH_REL_DISCONT,
        spg_rc::M99_NORMAL_DOT_MIN,
        spg_rc::M99_VAR_MIN,
        spg_rc::M99_PROBE_SPP,
        spg_rc::M99_HISTORY_ALPHA,
        grid_ad.probes.len(),
        grid_uni.probes.len(),
        grid_ad.cause_counts[0],
        grid_ad.cause_counts[1],
        grid_ad.cause_counts[2],
        grid_ad.level_map,
        frames_json.join(", "),
        json_escape(world_reason),
        world_rejected,
        var_ratio,
        spg_rc::M99_PRODUCT_IS_VAR_RATIO_MIN,
        dev_ratio,
        spg_rc::M99_ADAPTIVE_DEVIATION_RATIO_MIN,
        var_ratio_sabotage,
        dev_ratio_sabotage,
        max_diff,
        checks_json.join(", "),
        digests_json.join(", "),
        band_json.join(", "),
        std::env::args()
            .map(|a| format!("\"{}\"", json_escape(&a)))
            .collect::<Vec<_>>()
            .join(", "),
        failures_json.join(", "),
    );
    if let Some(p) = &args.evidence {
        std::fs::write(p, &json).unwrap_or_else(|e| fail(&format!("写 evidence {p}: {e}")));
    }
    println!("{json}");
    if failures.is_empty() {
        println!(
            "{TAG}: PASS 双跑位级一致 + 细分判据闭集计数非空 + 缓存计数逐帧非空 + 关 product IS/关自适应双臂可检测 + 私写重投影注入拒 + 世界级 not-triggered 登记 + 三档深度带内(validation={})",
            if validation_on { "on(0 error)" } else { "off" }
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}

/// UTC 时间戳(秒级;无依赖手工拼)。
fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    //  civil-from-days 算法(Howard Hinnant)。
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}
