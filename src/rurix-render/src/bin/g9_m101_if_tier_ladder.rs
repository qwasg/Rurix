//! G9.4 M101 IF 体素网格 + 档位阶梯 device harness(RXS-0362;门
//! `g9.p1.m101.if_tier_ladder`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M101 行 + spec/global_illumination.md RXS-0362)
//!
//! - **IF 档位阶梯 L0~L3 共享 probe 着色与八面体编码内核、只换空间索引**:
//!   L0 屏幕空间 probe(SPG 完整形态)/ L1 clipmap 体积 probe(八面体
//!   irradiance 8×8 + visibility 16×16 防漏光优先 + 每帧轮换更新摊销)/
//!   L2 空间哈希缓存 / L3 per-pixel 参考档;共享内核同一函数实例断言
//!   (`fn_addr_eq` 机器锚);L1 oct 图经 device kernel
//!   `g9_m101_probe_oct.rx`(RayQuery,irr/vis 双 dispatch)真跑,双跑位级
//!   一致;档间产物 digest 各自冻结(对拍可归因到索引结构);
//! - **每档强制含 AS 更新预算行且档位切换判据消费 AsStats 计数面(D2-Q10),
//!   超 AS 更新预算必须强制降档**:真实 `BlasCache::stats()` 消费锚 + 合成
//!   超预算序列 ⇒ 逐级强制降档,每步显式 DemotionRecord;`audit_demotions`
//!   独立重算逐条比对——静默降档注入(抑记录)/超限未降档注入必
//!   fail-closed(RED 臂独立有效);
//! - **八面体编码为线性域(SRGB 编码注入即 RED)**:SRGB 域注入 variant 的
//!   L1 出图对线性 golden rel_dev ≥ 冻结阈 + digest 分叉;sabotage 探针
//!   (线性 vs 线性)必不触发;
//! - **档位切换对同输入确定(双运行逐位一致)**:选档器双跑逐位一致 + 四档
//!   产物双跑 digest 相等;
//! - **按匹配深度对 M96 golden + 门序硬约束**:匹配深度 2;M96 cornell
//!   深度 2 实跑 digest 与 M97 冻结带条目逐字相等(D2-Q7 门序消费锚)。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/W3 能力链缺失 → `G9_M101_IF: SKIP DEV_ENV_DEGRADE`
//! (退 0,非 fake pass;`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层
//! 裁决);判据不符 / RED 轴失效 → FAIL 退 1。`RURIX_VK_VALIDATION=1`:vk.rs
//! lane 内 fail-closed;evidence 记 validation 模式。
//!
//! ## 用法
//!
//! ```text
//! g9_m101_if_tier_ladder --spv-m101 <m101.spv> --spv-m96 <m96.spv>
//!     [--band <path>] [--m97-band <path>] [--evidence <path>] [--work-dir <dir>]
//! g9_m101_if_tier_ladder --freeze --spv-m101 .. --spv-m96 .. [--band-out <path>]
//! g9_m101_if_tier_ladder --red-arm srgb-encode --spv-m101 ..
//! g9_m101_if_tier_ladder --red-arm budget-no-demote   # 纯 host 臂(无 device 依赖)
//! ```

use rurix_render::gi::fallback_chain as fb;
use rurix_render::gi::if_tier::{self as it, EncodeDomain, IfTier, OctTraceMode, ProbeOctMaps};
use rurix_render::gi::path_trace::{self, PtConfig, PtImage};
use rurix_render::gi::surface_cache;
use rurix_render::rt::as_manager::{AsStats, BlasCache, DynamicPolicy};
use rurix_rt::render_exec::{self, KernelWave};
use rurix_rt::vk::{
    self, RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc,
};

const TAG: &str = "G9_M101_IF";

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

/// oct 图 device 真跑(RayQuery;irr/vis 双 dispatch 合并一次提交)。返回全
/// 网格 oct 图(线性域;编码域注入在装配侧,RED 臂)。
fn run_oct_device(
    scene: &path_trace::PtScene,
    grid: &it::IfVoxelGrid,
    spv: &[u32],
    entry: &str,
) -> Result<Vec<ProbeOctMaps>, String> {
    let probe_count = grid.probe_count();
    let tris = scene.blas_triangles();
    let (blas_refs, instances) = scene_desc_of(&tris);
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let probes_b = bytes_f32(&it::pack_probe_positions(grid));
    let mats_b = bytes_f32(&path_trace::pack_mats(scene));
    let tris_b = bytes_f32(&tris);
    let params_irr = bytes_f32(&it::pack_oct_params(
        scene,
        probe_count as u32,
        OctTraceMode::Irradiance,
    ));
    let params_vis = bytes_f32(&it::pack_oct_params(
        scene,
        probe_count as u32,
        OctTraceMode::Visibility,
    ));
    let irr_texels = OctTraceMode::Irradiance.texels() as usize;
    let vis_texels = OctTraceMode::Visibility.texels() as usize;
    // 两 dispatch 各自的输出缓冲尺寸按本 dispatch texel 数定(非本 mode 缓冲
    // 写 0,不回读消费)。
    let bufs_irr = [
        RayQueryBufferDesc::Input(&probes_b),
        RayQueryBufferDesc::Input(&mats_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&params_irr),
        RayQueryBufferDesc::Output(probe_count * irr_texels * 12),
        RayQueryBufferDesc::Output(probe_count * irr_texels * 4),
    ];
    let bufs_vis = [
        RayQueryBufferDesc::Input(&probes_b),
        RayQueryBufferDesc::Input(&mats_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&params_vis),
        RayQueryBufferDesc::Output(probe_count * vis_texels * 12),
        RayQueryBufferDesc::Output(probe_count * vis_texels * 4),
    ];
    let out = vk::run_ray_query_effects(
        &scene_desc,
        &[
            RayQueryDispatchDesc {
                name: "g9_m101_probe_oct_irr",
                spv,
                entry,
                buffers: &bufs_irr,
                push_constants: &[],
                groups: [(probe_count * irr_texels) as u32, 1, 1],
            },
            RayQueryDispatchDesc {
                name: "g9_m101_probe_oct_vis",
                spv,
                entry,
                buffers: &bufs_vis,
                push_constants: &[],
                groups: [(probe_count * vis_texels) as u32, 1, 1],
            },
        ],
    )?;
    if out.readbacks.len() != 2 {
        return Err(format!("回读批数 {} ≠ 2", out.readbacks.len()));
    }
    let mut it_out = out.readbacks.into_iter();
    let rb_irr = it_out.next().ok_or("缺 irr 回读")?;
    let rb_vis = it_out.next().ok_or("缺 vis 回读")?;
    let irr = read_f32(&rb_irr[0]);
    let vis = read_f32(&rb_vis[1]);
    let mut maps = Vec::with_capacity(probe_count);
    for p in 0..probe_count {
        let mut irr_map = vec![[0.0f32; 3]; irr_texels];
        for t in 0..irr_texels {
            let b = (p * irr_texels + t) * 3;
            irr_map[t] = [irr[b], irr[b + 1], irr[b + 2]];
        }
        let vis_map = vis[p * vis_texels..(p + 1) * vis_texels].to_vec();
        maps.push(ProbeOctMaps {
            irr: irr_map,
            vis: vis_map,
        });
    }
    Ok(maps)
}

/// M96 golden 对照腿:同场景深度 2 megakernel(spp=64,冻结 seed)→ (PtImage, digest)。
fn run_m96(
    scene: &path_trace::PtScene,
    depth: u32,
    spv: &[u32],
    entry: &str,
) -> Result<(PtImage, [u8; 32]), String> {
    let cfg = PtConfig {
        spp: it::M101_M96_GOLDEN_SPP,
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
    let rb = out
        .readbacks
        .into_iter()
        .next()
        .ok_or("单 dispatch 缺回读")?;
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
        sum_lum: read_f32(
            &rb[1]
                .chunks_exact(8)
                .map(|c| &c[..4])
                .collect::<Vec<_>>()
                .concat(),
        ),
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
// 参数解析
// ---------------------------------------------------------------------------

struct Args {
    spv_m101: Option<String>,
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
        spv_m101: None,
        spv_m96: None,
        evidence: None,
        band: "milestones/g9/g9_m101_if_tier_band.json".to_string(),
        m97_band: "milestones/g9/g9_m97_depth_band.json".to_string(),
        work_dir: ".tmp/g9_m101_work".to_string(),
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
            "--spv-m101" => out.spv_m101 = Some(take(&mut i)),
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

/// RED 臂②:超预算静默降档注入(抑记录)⇒ 审计必 fail-closed;超限未降档
/// 注入(服务档伪造)⇒ 必拒;正例(显式记录链)必过。纯 host。
fn red_arm_budget_no_demote() -> bool {
    let hot = AsStats {
        blas_builds: 0,
        refits: 99,
        tlas_rebuilds: 0,
    };
    // 正例:超预算 ⇒ 逐级强制降档,记录链显式 ⇒ 审计过。
    let (served, recs) = it::select_tier(IfTier::L3PerPixel, &hot, true);
    let ok = served == IfTier::L0ScreenProbe
        && recs.len() == 3
        && it::audit_demotions(IfTier::L3PerPixel, &hot, served, &recs).is_ok();
    // 注入①:静默降档(抑记录)⇒ 必拒。
    let (served_silent, recs_silent) = it::select_tier(IfTier::L3PerPixel, &hot, false);
    let rej_silent = matches!(
        it::audit_demotions(IfTier::L3PerPixel, &hot, served_silent, &recs_silent),
        Err(it::IfError::SilentDemotion(_))
    );
    // 注入②:超限未降档(服务档伪造 L3)⇒ 必拒。
    let rej_held = matches!(
        it::audit_demotions(IfTier::L3PerPixel, &hot, IfTier::L3PerPixel, &recs),
        Err(it::IfError::SilentDemotion(_))
    );
    println!(
        "{TAG}: RED 臂 budget-no-demote(正例过={ok} 静默注入拒={rej_silent} 未降档注入拒={rej_held})"
    );
    ok && rej_silent && rej_held
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    println!(
        "[g9_m101_if_tier_ladder] G9.4 M101 IF 体素网格 + 档位阶梯 device harness(RXS-0362;门 g9.p1.m101.if_tier_ladder)"
    );
    let args = parse_args();

    // ── 步骤 0:host 预传递(无 device 依赖)──
    let scene = path_trace::m96_cornell_scene();
    scene
        .validate()
        .unwrap_or_else(|e| fail(&format!("场景校验: {e}")));
    let gb = fb::gbuffer_prepass(&scene);
    let mut failures: Vec<String> = Vec::new();
    // 共享内核同一函数实例断言(D2-Q5 机器锚)。
    let shared_kernel_ok = it::assert_shared_kernel_instance();
    if !shared_kernel_ok {
        failures.push("共享内核同一函数实例断言失败(各档复制实现即 RED)".into());
    }
    // 真实 AsStats 消费锚:BlasCache 建 cornell BLAS ⇒ stats 快照供选档。
    let mut blas_cache = BlasCache::new();
    let _blas = blas_cache.get_or_build(&scene.positions, &scene.indices, DynamicPolicy::Static);
    let as_stats_calm = blas_cache.stats();
    // 轮换更新摊销:两帧游标推进确定性,预算非空。
    let mut grid = it::IfVoxelGrid::new();
    let rot0 = grid.rotate_update();
    let rot1 = grid.rotate_update();
    let rotation_ok = rot0.len() == it::M101_L1_UPDATE_BUDGET as usize
        && rot1.len() == it::M101_L1_UPDATE_BUDGET as usize
        && rot0
            .iter()
            .all(|a| !rot1.contains(a) || rot1.len() as u64 * 2 > grid.probe_count() as u64);
    println!(
        "{TAG}: host 预传递 pixels={} probes={} 共享内核单实例={} AsStats(calm)={:?} 轮换帧0={} 帧1={}",
        gb.width * gb.height,
        grid.probe_count(),
        shared_kernel_ok,
        as_stats_calm,
        rot0.len(),
        rot1.len()
    );

    // --red-arm budget-no-demote:纯 host 臂(静默降档注入 ⇒ 审计必拒)。
    if args.red_arm.as_deref() == Some("budget-no-demote") {
        if red_arm_budget_no_demote() {
            println!("{TAG}: PASS red-arm budget-no-demote(独立检出:静默/未降档注入必拒)");
            std::process::exit(0);
        }
        fail("red-arm budget-no-demote 未检出");
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
    let spv_m101_path = args
        .spv_m101
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-m101 <m101.spv>"));
    let spv_m101 = load_spv(&spv_m101_path);
    let entry_m101 =
        vk::entry_point_name(&spv_m101).unwrap_or_else(|| fail("M101 SPV 无 OpEntryPoint"));
    let work = std::path::PathBuf::from(&args.work_dir);
    std::fs::create_dir_all(&work).unwrap_or_else(|e| fail(&format!("建 work-dir: {e}")));

    // ── 步骤 2:device 腿(oct 图双跑位级一致 + host 镜像对拍)──
    let maps_dev = match run_oct_device(&scene, &grid, &spv_m101, &entry_m101) {
        Ok(v) => v,
        Err(e) => fail(&format!("oct device: {e}")),
    };
    let maps_dev_b = match run_oct_device(&scene, &grid, &spv_m101, &entry_m101) {
        Ok(v) => v,
        Err(e) => fail(&format!("oct device 双跑: {e}")),
    };
    let device_bitexact = maps_dev == maps_dev_b;
    if !device_bitexact {
        failures.push("oct device 双跑位级一致破坏".into());
    }
    // host 镜像对拍:visibility 遮蔽分类(<1.0 ⇒ 有遮蔽)结构域精确硬判据;
    // irradiance/遮蔽距离浮点残差 = 信息项(设备 FMA 残差口径,G7.5b 先例;
    // 实测值进 evidence,禁手写阈值)。
    let maps_host = it::oct_probe_trace_host(&scene, &grid, EncodeDomain::Linear);
    let mut vis_parity = true;
    let mut irr_max_diff = 0.0f32;
    let mut vis_max_diff = 0.0f32;
    for (d, h) in maps_dev.iter().zip(maps_host.iter()) {
        for (dv, hv) in d.vis.iter().zip(h.vis.iter()) {
            if (*dv < 1.0) != (*hv < 1.0) {
                vis_parity = false;
            }
            vis_max_diff = vis_max_diff.max((dv - hv).abs());
        }
        for (di, hi) in d.irr.iter().zip(h.irr.iter()) {
            for c in 0..3 {
                irr_max_diff = irr_max_diff.max((di[c] - hi[c]).abs());
            }
        }
    }
    if !vis_parity {
        failures.push("visibility 图 device/host 结构对拍分叉(遮蔽分类不一致)".into());
    }
    println!(
        "{TAG}: oct device 双跑位级一致={device_bitexact} vis 分类对拍={vis_parity} vis 残差={vis_max_diff:.3e} irr 残差 max|Δ|={irr_max_diff:.3e}(信息项)"
    );

    // ── 步骤 3:四档产物(L1 用 device oct 图;L0/L2/L3 host 评估;双跑)──
    let frame_l1 = it::eval_tier(IfTier::L1ClipmapVolume, &scene, &gb, &grid, &maps_dev)
        .unwrap_or_else(|e| fail(&format!("L1 评估: {e}")));
    let frame_l1_b = it::eval_tier(IfTier::L1ClipmapVolume, &scene, &gb, &grid, &maps_dev_b)
        .unwrap_or_else(|e| fail(&format!("L1 双跑: {e}")));
    let frame_l0 = it::eval_l0_screen_probe(&scene, &gb);
    let frame_l0_b = it::eval_l0_screen_probe(&scene, &gb);
    let frame_l2 = it::eval_l2_spatial_hash(&scene, &gb);
    let frame_l2_b = it::eval_l2_spatial_hash(&scene, &gb);
    let frame_l3 = it::eval_l3_per_pixel(&scene, &gb);
    let frame_l3_b = it::eval_l3_per_pixel(&scene, &gb);
    let tier_double_run = frame_l1 == frame_l1_b
        && frame_l0 == frame_l0_b
        && frame_l2 == frame_l2_b
        && frame_l3 == frame_l3_b;
    if !tier_double_run {
        failures.push("档位产物双跑位级一致破坏(档位切换/评估确定性违例)".into());
    }
    let tier_frames: Vec<it::TierFrame> = vec![frame_l0, frame_l1, frame_l2, frame_l3];

    // ── 步骤 4:选档器(消费 AsStats;强制降档 + 审计 + 静默注入臂)──
    let (served_calm, recs_calm) = it::select_tier(IfTier::L3PerPixel, &as_stats_calm, true);
    let calm_ok = served_calm == IfTier::L3PerPixel
        && recs_calm.is_empty()
        && it::audit_demotions(IfTier::L3PerPixel, &as_stats_calm, served_calm, &recs_calm).is_ok();
    let selector_double_run = it::select_tier(IfTier::L3PerPixel, &as_stats_calm, true)
        == (served_calm, recs_calm.clone());
    let hot = AsStats {
        blas_builds: 0,
        refits: 99,
        tlas_rebuilds: 0,
    };
    let (served_hot, recs_hot) = it::select_tier(IfTier::L3PerPixel, &hot, true);
    let demote_ok = served_hot == IfTier::L0ScreenProbe
        && recs_hot.len() == 3
        && it::audit_demotions(IfTier::L3PerPixel, &hot, served_hot, &recs_hot).is_ok();
    let selector_checks_ok = calm_ok && selector_double_run && demote_ok;
    println!(
        "{TAG}: 选档器 calm→{} (记录 {}) hot→{} (记录 {}) 双跑一致={selector_double_run}",
        served_calm.name(),
        recs_calm.len(),
        served_hot.name(),
        recs_hot.len()
    );
    if !selector_checks_ok {
        failures.push("选档器判据破坏(calm 降档/双跑分叉/hot 降档链或审计失败)".into());
    }
    // 静默降档注入臂(门内需独立有效)。
    let silent_arm_ok = red_arm_budget_no_demote();
    if !silent_arm_ok {
        failures.push("静默降档/未降档注入臂失效".into());
    }

    // ── 步骤 5:M96 golden 对照腿(匹配深度 2)+ 门序消费锚 ──
    let spv_m96_path = args
        .spv_m96
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-m96 <m96.spv>"));
    let spv_m96 = load_spv(&spv_m96_path);
    let entry_m96 =
        vk::entry_point_name(&spv_m96).unwrap_or_else(|| fail("M96 SPV 无 OpEntryPoint"));
    let m97_band_text = std::fs::read_to_string(&args.m97_band).unwrap_or_else(|e| {
        fail(&format!(
            "读 M97 深度带 {}: {e}(门序消费锚前置)",
            args.m97_band
        ))
    });
    let m97_band = surface_cache::DepthBand::parse(&m97_band_text)
        .unwrap_or_else(|e| fail(&format!("M97 深度带解析: {e}")));
    let m97_anchor = m97_band
        .entry(it::M101_MATCHED_DEPTH)
        .unwrap_or_else(|e| fail(&format!("M97 深度带缺锚条目: {e}")))
        .m96_digest
        .clone();
    let (m96_img, m96_digest) = match run_m96(&scene, it::M101_MATCHED_DEPTH, &spv_m96, &entry_m96)
    {
        Ok(v) => v,
        Err(e) => fail(&format!("M96 对照腿: {e}")),
    };
    let m96_cross_anchor = hex(&m96_digest) == m97_anchor;
    println!(
        "{TAG}: M96 depth={} digest={} 门序锚(M97 带)={}",
        it::M101_MATCHED_DEPTH,
        hex(&m96_digest),
        m96_cross_anchor
    );
    if !m96_cross_anchor {
        failures.push(format!(
            "门序消费锚破坏:M96 depth=2 digest {} ≠ M97 冻结带条目 {}",
            hex(&m96_digest),
            m97_anchor
        ));
    }

    // ── 步骤 6:四档 rel_dev + RED 臂①(SRGB 编码注入)──
    let mut measured: Vec<it::M101BandEntry> = Vec::new();
    for frame in &tier_frames {
        let dev = path_trace::rel_dev(&frame.rgb, &m96_img.rgb).expect("rel_dev 计算");
        println!(
            "{TAG}: tier={} digest={} rel_dev={dev:.6e}",
            frame.tier.name(),
            hex(&frame.product_digest())
        );
        measured.push(it::M101BandEntry {
            tier: frame.tier.name().to_string(),
            product_digest: hex(&frame.product_digest()),
            m96_digest: hex(&m96_digest),
            band_rel_dev: dev * it::M101_BAND_MARGIN,
            measured_rel_dev: dev,
        });
    }
    // 臂①:SRGB 编码注入 ⇒ L1 出图对线性 golden rel_dev ≥ 阈 + digest 分叉;
    // sabotage 探针(线性 vs 线性)= 0 必不触发。
    let maps_srgb: Vec<ProbeOctMaps> = maps_dev
        .iter()
        .map(|m| ProbeOctMaps {
            irr: m
                .irr
                .iter()
                .map(|&rgb| it::apply_domain(rgb, EncodeDomain::SrgbInjected))
                .collect(),
            vis: m.vis.clone(),
        })
        .collect();
    let frame_srgb = it::eval_tier(IfTier::L1ClipmapVolume, &scene, &gb, &grid, &maps_srgb)
        .unwrap_or_else(|e| fail(&format!("SRGB 注入评估: {e}")));
    let frame_l1_golden = &tier_frames[1];
    let srgb_rel =
        path_trace::rel_dev(&frame_srgb.rgb, &frame_l1_golden.rgb).expect("rel_dev 计算");
    let srgb_sabotage =
        path_trace::rel_dev(&frame_l1_golden.rgb, &frame_l1_golden.rgb).expect("rel_dev 计算");
    let srgb_detectable = srgb_rel >= it::M101_SRGB_REL_DEV_MIN
        && frame_srgb.product_digest() != frame_l1_golden.product_digest()
        && srgb_sabotage < it::M101_SRGB_REL_DEV_MIN;
    println!(
        "{TAG}: RED 臂 srgb-encode:rel_dev={srgb_rel:.6e}(阈 {})digest 分叉={} sabotage={srgb_sabotage:.3}(应<阈)",
        it::M101_SRGB_REL_DEV_MIN,
        frame_srgb.product_digest() != frame_l1_golden.product_digest()
    );
    if args.red_arm.as_deref() == Some("srgb-encode") {
        if srgb_detectable {
            println!("{TAG}: PASS red-arm srgb-encode(独立检出:编码域错误可检测 + 探针能红)");
            std::process::exit(0);
        }
        fail("red-arm srgb-encode 失效(编码域错误不可检测或探针不红)");
    }
    if !srgb_detectable {
        failures.push(format!(
            "SRGB 编码注入臂失效(rel_dev={srgb_rel:.6e} 或 digest 未分叉或探针误检)"
        ));
    }
    if let Some(arm) = &args.red_arm {
        fail(&format!("unknown --red-arm {arm}"));
    }

    // ── 步骤 7:freeze(写带)或 gate(比对带)──
    let mut digests_match = true;
    let mut depth_band_within = true;
    if args.freeze {
        let band = it::M101Band {
            frozen_at_utc: utc_now(),
            device_name: caps.device_name.clone(),
            scene: scene.name.to_string(),
            m96_anchor_digest: m97_anchor.clone(),
            srgb_encode_rel_dev: srgb_rel,
            entries: measured.clone(),
        };
        let out = args.band_out.clone().unwrap_or(args.band.clone());
        std::fs::write(&out, band.to_json()).unwrap_or_else(|e| fail(&format!("写带 {out}: {e}")));
        println!(
            "{TAG}: FREEZE 容差带已写 {out}(measured × {};provenance 全字段)",
            it::M101_BAND_MARGIN
        );
    } else {
        let band_text = std::fs::read_to_string(&args.band)
            .unwrap_or_else(|e| fail(&format!("读容差带 {}: {e}", args.band)));
        let band =
            it::M101Band::parse(&band_text).unwrap_or_else(|e| fail(&format!("容差带解析: {e}")));
        if m97_anchor != band.m96_anchor_digest {
            digests_match = false;
            failures.push("M97 门序锚条目与冻结带漂移".into());
        }
        for m in &measured {
            match band.check(
                &m.tier,
                &m.product_digest,
                &m.m96_digest,
                m.measured_rel_dev,
            ) {
                Ok(()) => {}
                Err(e) => {
                    digests_match = false;
                    depth_band_within = false;
                    failures.push(e.to_string());
                }
            }
        }
        if digests_match && depth_band_within {
            println!("{TAG}: 深度带对照在带内(四档产物 digest 全等 + rel_dev ≤ 冻结带)");
        }
    }

    // ── 步骤 8:evidence(rurix.g9m101.if_tier.v1)──
    let checks: [(&str, bool); 12] = [
        ("double_run_bitexact", device_bitexact && tier_double_run),
        ("shared_kernel_single_instance", shared_kernel_ok),
        ("vis_device_host_parity", vis_parity),
        ("rotation_amortization_non_empty", rotation_ok),
        (
            "budget_row_per_tier_present",
            IfTier::ALL
                .iter()
                .all(|t| it::tier_def(*t).as_budget.max_refits > 0),
        ),
        ("as_stats_consumed_calm_no_demote", calm_ok),
        ("forced_demote_with_records", demote_ok),
        ("selector_double_run_bitexact", selector_double_run),
        ("silent_demotion_detected", silent_arm_ok),
        ("srgb_encode_detectable", srgb_detectable),
        ("m96_cross_anchor", m96_cross_anchor),
        ("depth_band_within", digests_match && depth_band_within),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let mut digests_json: Vec<String> = vec![format!(
        "\"m96_depth{}\": \"{}\"",
        it::M101_MATCHED_DEPTH,
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
    let budget_json: Vec<String> = IfTier::ALL
        .iter()
        .map(|t| {
            let d = it::tier_def(*t);
            format!(
                "\"{}\": {{\"index_kind\": \"{}\", \"max_blas_builds\": {}, \"max_refits\": {}, \"max_tlas_rebuilds\": {}}}",
                t.name(),
                d.index_kind,
                d.as_budget.max_blas_builds,
                d.as_budget.max_refits,
                d.as_budget.max_tlas_rebuilds
            )
        })
        .collect();
    let demote_json: Vec<String> = recs_hot
        .iter()
        .map(|r| format!("\"{}→{}\"", r.from.name(), r.to.name()))
        .collect();
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m101.if_tier.v1\",\n  \
         \"subject\": \"g9_m101_if_tier_ladder\",\n  \
         \"spec_anchor\": \"RXS-0362\",\n  \
         \"assertion_id\": \"g9.p1.m101.if_tier_ladder\",\n  \
         \"milestone\": \"M101\",\n  \"wave\": \"G9.4\",\n  \
         \"status\": \"{status}\",\n  \
         \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \"require_real\": {}}},\n  \
         \"determinism_protocol\": {{\"seed_chain\": \"{}\", \"rng\": \"PCG32 单一流按索引寻址(rt::ref_tracer::Pcg32 同一实例;流为输入非结果,G7.4 先例)\", \
         \"accumulation\": \"逐实体独立顺序累加(禁 atomic);L2 缓存 = BTreeMap 迭代序确定\", \
         \"digest_domain\": \"sha256(rgb‖tier)——服务档入键,档位转移必分叉\"}},\n  \
         \"oct_encoding\": {{\"domain\": \"linear(线性域;SRGB 注入即 RED)\", \"irr_res\": {}, \"vis_res\": {}, \
         \"visibility_priority\": \"vis 16×16 > irr 8×8(防漏光优先于提 irradiance 分辨率,RXS-0362 L1 逐字)\", \
         \"roundtrip_bound_irr_rad\": {}, \"roundtrip_bound_vis_rad\": {}, \
         \"single_source\": \"gi::if_tier::oct(host)/kernels/g9_m101_probe_oct.rx(device)逐字同源\"}},\n  \
         \"voxel_grid\": {{\"dims\": {:?}, \"cell\": {}, \"probes\": {}, \"rotation_budget_per_frame\": {}, \
         \"rotation_frame0\": {}, \"rotation_frame1\": {}, \"ddgi_resampling\": \"演进项非首版(未做)\"}},\n  \
         \"tier_ladder\": {{\"tiers\": {:?}, \"budget_rows\": {{{}}}, \
         \"shared_kernel_single_instance\": {}, \"shared_kernel_assert\": \"std::ptr::fn_addr_eq 四档 dispatch 两两相等\"}},\n  \
         \"as_budget_contract\": {{\"calm_stats\": {{\"blas_builds\": {}, \"refits\": {}, \"tlas_rebuilds\": {}}}, \
         \"calm_served\": \"{}\", \"hot_stats_refits\": 99, \"hot_served\": \"{}\", \
         \"demotion_chain\": [{}], \"audit\": \"audit_demotions 独立重算逐条比对,静默降档 fail-closed\"}},\n  \
         \"red_arm_metrics\": {{\"srgb_encode_rel_dev\": \"{:e}\", \"srgb_threshold\": \"{:e}\", \"sabotage_probe_srgb\": \"{:e}\"}},\n  \
         \"host_device_parity\": {{\"vis_structural_exact\": {}, \"vis_float_residual_max_abs\": \"{:e}\", \"irr_float_residual_max_abs\": \"{:e}\", \
         \"semantics\": \"visibility 遮蔽分类(<1.0)结构域精确硬判据;irr/遮蔽距离残差 = 设备 FMA 残差信息项(G7.5b 口径)\"}},\n  \
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
        it::M101_SEED,
        it::M101_IRR_RES,
        it::M101_VIS_RES,
        it::OCT_ROUNDTRIP_BOUND_IRR,
        it::OCT_ROUNDTRIP_BOUND_VIS,
        it::M101_GRID_DIMS,
        it::M101_GRID_CELL,
        grid.probe_count(),
        it::M101_L1_UPDATE_BUDGET,
        rot0.len(),
        rot1.len(),
        IfTier::ALL.iter().map(|t| t.name()).collect::<Vec<_>>(),
        budget_json.join(", "),
        shared_kernel_ok,
        as_stats_calm.blas_builds,
        as_stats_calm.refits,
        as_stats_calm.tlas_rebuilds,
        served_calm.name(),
        served_hot.name(),
        demote_json.join(", "),
        srgb_rel,
        it::M101_SRGB_REL_DEV_MIN,
        srgb_sabotage,
        vis_parity,
        vis_max_diff,
        irr_max_diff,
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
            "{TAG}: PASS 双跑位级一致 + 共享内核单实例 + 预算行闭集 + 强制降档显式记录 + 静默降档注入拒 + SRGB 注入可检测 + 四档深度带内(validation={})",
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
