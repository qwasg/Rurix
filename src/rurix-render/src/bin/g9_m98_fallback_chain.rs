//! G9.4 M98 四级追踪降级链 device harness(RXS-0359;门 `g9.p0.m98.tracing_fallback_chain`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §2 M98 行 + spec/global_illumination.md RXS-0359)
//!
//! - **L1 Screen Trace → L2 SWRT → L3 HWRT(含 hit lighting 档)→ L4 Far Field
//!   四级命中率/耗时计数非空且逐帧 evidence**:场景 = M96 冻结 cornell fixture;
//!   主光线像素中心 GBuffer 预传递(host 输入产线),二次射线(冻结 seed PCG32
//!   流)经选档器逐像素选档;L1 device kernel 真跑(纯 compute;host 参照对拍
//!   = hit flags/march 步数结构域精确硬判据 + rgb/t 浮点残差信息项,G7.5b
//!   「FMA 残差不进判据」先例)+ L2 host 解析场景暴力求值(BVH 金标准
//!   对拍)+ L3 device RayQuery 双着色档真跑(U30 run_ray_query_effects);
//!   耗时 = 确定性代理计数(L1 march 步数 / L2 三角测试数 / L3 有效射线查询
//!   发行量 1+hit)+ host 壁钟 ns 信息项(口径显式登记);双帧(同输入)逐帧导出;
//! - **逐级强制关闭后回归差异必须可检测(强关后输出仍同 golden 即 RED)**:
//!   产物 digest = sha256(rgb‖flags),flags 携带实际服务级别 ⇒ 级别转移必然
//!   改变 digest(结构性保证);每级强关臂断言 digest 分叉 + 转移日志含
//!   ForcedOff;反向 sabotage 探针(golden vs golden)必判 RED(能红证明);
//! - **实际使用级别必须显式记录,禁静默回退**:逐像素 flags + 逐边界
//!   TransitionRecord(Miss/OutOfRange/ForcedOff)入 evidence;
//!   `fallback_chain::audit` 独立重算比对——静默回退注入 variant(抑日志)
//!   必 fail-closed Err(RED 臂独立有效);
//! - **L4 Far Field 依赖 HLOD 接口未就绪 ⇒ 登记 SKIP=not-triggered 不充绿**:
//!   `check_l4_trigger` 判 not-triggered 显式登记;`l4_serve` 服务请求必
//!   typed Err;L4 行计数面恒零(显式,不充绿);
//! - **各档按匹配深度对 M96 golden**:1 次间接弹射 ⇒ 匹配深度 2;四档 solo
//!   + 全链双着色档共 6 条目 rel_dev ≤ 冻结带(`milestones/g9/
//!   g9_m98_depth_band.json`;带 = measured × M98_BAND_MARGIN,禁手写 P-09);
//!   M96 深度 2 digest 与 M97 冻结带 `m96_cornell` 条目逐字相等(门序消费锚)。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/W3 能力链缺失 → `G9_M98_FB: SKIP DEV_ENV_DEGRADE`
//! (退 0,非 fake pass;`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层
//! 裁决);判据不符 / RED 轴失效 → FAIL 退 1。`RURIX_VK_VALIDATION=1`:vk.rs
//! lane 内 fail-closed;evidence 记 validation 模式。
//!
//! ## 用法
//!
//! ```text
//! g9_m98_fallback_chain --spv-l1 <l1.spv> --spv-l3 <l3.spv> --spv-m96 <m96.spv>
//!     [--band <path>] [--m97-band <path>] [--evidence <path>] [--work-dir <dir>]
//! g9_m98_fallback_chain --freeze --spv-l1 .. --spv-l3 .. --spv-m96 .. [--band-out <path>]
//! g9_m98_fallback_chain --red-arm force-off-l1|force-off-l2|force-off-l3 --spv-l1 .. --spv-l3 ..
//! g9_m98_fallback_chain --red-arm silent-demotion   # 纯 host 臂(无 device 依赖)
//! ```

use rurix_render::gi::fallback_chain::{self as fb, ChainSwitches, L3ShadeMode, LegSample};
use rurix_render::gi::path_trace::{self, PtConfig, PtImage};
use rurix_render::gi::surface_cache;
use rurix_rt::render_exec::{self, KernelWave};
use rurix_rt::vk::{
    self, RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc,
};

const TAG: &str = "G9_M98_FB";
/// 逐帧 evidence 帧数(同输入双帧 = 双跑位级一致 + 逐帧计数面)。
const FRAMES: usize = 2;

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

/// L1 device 真跑(纯 compute 屏幕 march;无 TLAS——D2-Q9 射线流纪律)。
fn run_l1_device(
    scene: &path_trace::PtScene,
    gb: &fb::GBuffer,
    spv: &[u32],
    entry: &str,
) -> Result<(Vec<LegSample>, u64), String> {
    let pixel_count = (gb.width * gb.height) as usize;
    let mut bufs: Vec<Vec<u8>> = vec![
        bytes_f32(&gb.depth),
        bytes_f32(&gb.nrm),
        bytes_f32(&gb.alb),
        bytes_f32(&gb.sec_o),
        bytes_f32(&gb.sec_d),
        bytes_f32(&fb::pack_l1_params(scene, gb)),
        vec![0u8; pixel_count * 12],
        vec![0u8; pixel_count * 16],
    ];
    let t0 = std::time::Instant::now();
    vk::run_compute(spv, entry, &mut bufs, &[], [pixel_count as u32, 1, 1])?;
    let wall_ns = t0.elapsed().as_nanos() as u64;
    let rgb = read_f32(&bufs[6]);
    let state = read_f32(&bufs[7]);
    let mut out = Vec::with_capacity(pixel_count);
    for i in 0..pixel_count {
        out.push(LegSample {
            hit: state[i * 4] >= 0.5,
            t: state[i * 4 + 1],
            rgb: [rgb[i * 3], rgb[i * 3 + 1], rgb[i * 3 + 2]],
            work: state[i * 4 + 2] as u32,
        });
    }
    Ok((out, wall_ns))
}

/// L3 device 真跑(RayQuery 对 TLAS;双着色档)。
fn run_l3_device(
    scene: &path_trace::PtScene,
    gb: &fb::GBuffer,
    mode: L3ShadeMode,
    spv: &[u32],
    entry: &str,
) -> Result<(Vec<LegSample>, u64), String> {
    let tris = scene.blas_triangles();
    let (blas_refs, instances) = scene_desc_of(&tris);
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let pixel_count = (gb.width * gb.height) as usize;
    let rays_o_b = bytes_f32(&gb.sec_o);
    let rays_d_b = bytes_f32(&gb.sec_d);
    let rng_b = bytes_f32(&gb.stream);
    let mats_b = bytes_f32(&path_trace::pack_mats(scene));
    let tris_b = bytes_f32(&tris);
    let params_b = bytes_f32(&fb::pack_l3_params(scene, mode));
    let buffers = [
        RayQueryBufferDesc::Input(&rays_o_b),
        RayQueryBufferDesc::Input(&rays_d_b),
        RayQueryBufferDesc::Input(&rng_b),
        RayQueryBufferDesc::Input(&mats_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&params_b),
        RayQueryBufferDesc::Output(pixel_count * 12),
        RayQueryBufferDesc::Output(pixel_count * 16),
    ];
    let t0 = std::time::Instant::now();
    let out = vk::run_ray_query_effects(
        &scene_desc,
        &[RayQueryDispatchDesc {
            name: "g9_m98_hwrt",
            spv,
            entry,
            buffers: &buffers,
            push_constants: &[],
            groups: [pixel_count as u32, 1, 1],
        }],
    )?;
    let wall_ns = t0.elapsed().as_nanos() as u64;
    let rb = out
        .readbacks
        .into_iter()
        .next()
        .ok_or("单 dispatch 缺回读")?;
    if rb.len() != 2 {
        return Err(format!("回读路数 {} ≠ 2", rb.len()));
    }
    let rgb = read_f32(&rb[0]);
    let state = read_f32(&rb[1]);
    let mut leg = Vec::with_capacity(pixel_count);
    for i in 0..pixel_count {
        leg.push(LegSample {
            hit: state[i * 4] >= 0.5,
            t: state[i * 4 + 1],
            rgb: [rgb[i * 3], rgb[i * 3 + 1], rgb[i * 3 + 2]],
            work: state[i * 4 + 2] as u32,
        });
    }
    Ok((leg, wall_ns))
}

/// M96 golden 对照腿:同场景深度 2 megakernel(spp=64,冻结 seed)→ (PtImage, digest)。
fn run_m96(
    scene: &path_trace::PtScene,
    depth: u32,
    spv: &[u32],
    entry: &str,
) -> Result<(PtImage, [u8; 32]), String> {
    let cfg = PtConfig {
        spp: fb::M98_M96_GOLDEN_SPP,
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
    spv_l1: Option<String>,
    spv_l3: Option<String>,
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
        spv_l1: None,
        spv_l3: None,
        spv_m96: None,
        evidence: None,
        band: "milestones/g9/g9_m98_depth_band.json".to_string(),
        m97_band: "milestones/g9/g9_m97_depth_band.json".to_string(),
        work_dir: ".tmp/g9_m98_work".to_string(),
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
            "--spv-l1" => out.spv_l1 = Some(take(&mut i)),
            "--spv-l3" => out.spv_l3 = Some(take(&mut i)),
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

/// 强关可检测判定:产物 digest 分叉 且 转移日志含 ForcedOff ⇒ true(检出);
/// 强关后输出仍同 golden ⇒ false(回归不可检测 = RED,RXS-0359 L3)。
fn force_off_detectable(golden: &fb::ChainFrame, off: &fb::ChainFrame) -> bool {
    golden.product_digest() != off.product_digest()
        && off
            .transitions
            .iter()
            .any(|r| r.cause == fb::TransitionCause::ForcedOff)
}

/// RED 臂①:静默回退注入(抑日志 variant)⇒ 审计必 fail-closed SilentFallback。
/// 纯 host(host 三腿即可承载;对接 conformance 负例语料臂①)。
fn red_arm_silent_demotion() -> bool {
    let scene = path_trace::m96_cornell_scene();
    scene.validate().expect("场景校验");
    let gb = fb::gbuffer_prepass(&scene);
    let l1 = fb::l1_leg_host(&scene, &gb);
    let l2 = fb::l2_leg_host(&scene, &gb.sec_o, &gb.sec_d);
    let l3 = fb::l3_leg_host(&scene, &gb, L3ShadeMode::Simple);
    // 正例:记录开 ⇒ 装配过。
    let ok_frame = fb::assemble(&gb, ChainSwitches::ALL_ON, &l1, &l2, &l3, true);
    // 注入:静默回退(抑日志)⇒ 必 Err。
    let injected = fb::assemble(&gb, ChainSwitches::ALL_ON, &l1, &l2, &l3, false);
    let detected = matches!(injected, Err(fb::FbError::SilentFallback(_))) && ok_frame.is_ok();
    // 反向锚:全关 Unserved 终端仍须日志齐备(ForcedOff×3/px)可审计 ⇒ 非静默。
    let all_off = fb::assemble(
        &gb,
        ChainSwitches {
            l1: false,
            l2: false,
            l3: false,
        },
        &l1,
        &l2,
        &l3,
        true,
    );
    let terminal_logged = matches!(&all_off, Ok(f) if !f.transitions.is_empty()
        && f.transitions.iter().all(|r| r.cause == fb::TransitionCause::ForcedOff)
        && f.flags.iter().all(|&x| x == fb::FLAG_UNSERVED));
    println!(
        "{TAG}: RED 臂 silent-demotion(注入拒={} 正例过={} Unserved 终端日志齐={terminal_logged})",
        matches!(injected, Err(fb::FbError::SilentFallback(_))),
        ok_frame.is_ok()
    );
    detected && terminal_logged
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    println!(
        "[g9_m98_fallback_chain] G9.4 M98 四级追踪降级链 device harness(RXS-0359;门 g9.p0.m98.tracing_fallback_chain)"
    );
    let args = parse_args();

    // ── 步骤 0:host 预传递(无 device 依赖)──
    let scene = path_trace::m96_cornell_scene();
    scene
        .validate()
        .unwrap_or_else(|e| fail(&format!("场景校验: {e}")));
    let gb = fb::gbuffer_prepass(&scene);
    let gb2 = fb::gbuffer_prepass(&scene);
    if gb != gb2 {
        fail("GBuffer 预传递双跑分叉(确定性协议违例)");
    }
    let chain_pixels = gb.primary_hit.iter().filter(|&&b| b).count();
    let t0 = std::time::Instant::now();
    let l1_host = fb::l1_leg_host(&scene, &gb);
    let l2_host = fb::l2_leg_host(&scene, &gb.sec_o, &gb.sec_d);
    let l2_wall_ns = t0.elapsed().as_nanos() as u64;
    println!(
        "{TAG}: host 预传递 pixels={} 链内={} (L1 参照 + L2 SWRT 腿就绪)",
        gb.width * gb.height,
        chain_pixels
    );

    // --red-arm silent-demotion:纯 host 臂(静默回退注入 ⇒ 审计必拒)。
    if args.red_arm.as_deref() == Some("silent-demotion") {
        if red_arm_silent_demotion() {
            println!("{TAG}: PASS red-arm silent-demotion(独立检出:注入 variant 审计必拒)");
            std::process::exit(0);
        }
        fail("red-arm silent-demotion 未检出");
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
    let spv_l1_path = args
        .spv_l1
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-l1 <l1.spv>"));
    let spv_l3_path = args
        .spv_l3
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-l3 <l3.spv>"));
    let spv_l1 = load_spv(&spv_l1_path);
    let spv_l3 = load_spv(&spv_l3_path);
    let entry_l1 = vk::entry_point_name(&spv_l1).unwrap_or_else(|| fail("L1 SPV 无 OpEntryPoint"));
    let entry_l3 = vk::entry_point_name(&spv_l3).unwrap_or_else(|| fail("L3 SPV 无 OpEntryPoint"));
    println!("{TAG}: kernel entry l1=`{entry_l1}` l3=`{entry_l3}`");
    let work = std::path::PathBuf::from(&args.work_dir);
    std::fs::create_dir_all(&work).unwrap_or_else(|e| fail(&format!("建 work-dir: {e}")));
    let mut failures: Vec<String> = Vec::new();

    // ── 步骤 2:device 腿(L1 双跑位级一致 + host 参照对拍;L3 双档)──
    let (l1_dev, l1_wall) = match run_l1_device(&scene, &gb, &spv_l1, &entry_l1) {
        Ok(v) => v,
        Err(e) => fail(&format!("L1 device: {e}")),
    };
    let (l1_dev_b, _) = match run_l1_device(&scene, &gb, &spv_l1, &entry_l1) {
        Ok(v) => v,
        Err(e) => fail(&format!("L1 device 双跑: {e}")),
    };
    let l1_device_bitexact = l1_dev == l1_dev_b;
    // host 参照对拍:结构域精确(hit flags + march 步数 = 整数/判定域,硬判据);
    // rgb/t 浮点残差 = 信息项(单 texel 锁存邻域 + 设备 FMA 残差口径,G7.5b
    // 「FMA 残差不进判据」先例;实测值进 evidence,禁手写阈值)。
    let mut l1_parity = true;
    let mut max_diff = 0.0f32;
    for (i, (d, h)) in l1_dev.iter().zip(l1_host.iter()).enumerate() {
        if d.hit != h.hit || d.work != h.work {
            l1_parity = false;
            println!("{TAG}: L1 结构对拍分叉 px={i} dev={d:?} host={h:?}");
            break;
        }
        for c in 0..3 {
            max_diff = max_diff.max((d.rgb[c] - h.rgb[c]).abs());
        }
        max_diff = max_diff.max((d.t - h.t).abs());
    }
    if !l1_parity {
        failures.push("L1 device/host 结构对拍分叉(hit flags / march 步数不一致)".into());
    }
    println!(
        "{TAG}: L1 device 双跑位级一致={} 结构对拍={} 浮点残差 max|Δ|={:.3e}(信息项)",
        l1_device_bitexact, l1_parity, max_diff
    );
    let (l3_simple, l3s_wall) =
        match run_l3_device(&scene, &gb, L3ShadeMode::Simple, &spv_l3, &entry_l3) {
            Ok(v) => v,
            Err(e) => fail(&format!("L3 simple device: {e}")),
        };
    let (l3_simple_b, _) = match run_l3_device(&scene, &gb, L3ShadeMode::Simple, &spv_l3, &entry_l3)
    {
        Ok(v) => v,
        Err(e) => fail(&format!("L3 simple device 双跑: {e}")),
    };
    let (l3_hl, l3hl_wall) =
        match run_l3_device(&scene, &gb, L3ShadeMode::HitLighting, &spv_l3, &entry_l3) {
            Ok(v) => v,
            Err(e) => fail(&format!("L3 hit-lighting device: {e}")),
        };
    let l3_device_bitexact = l3_simple == l3_simple_b;

    // ── 步骤 3:装配(golden 双帧 + hit lighting 全链)──
    let mut frames: Vec<fb::ChainFrame> = Vec::new();
    for _ in 0..FRAMES {
        match fb::assemble(
            &gb,
            ChainSwitches::ALL_ON,
            &l1_dev,
            &l2_host,
            &l3_simple,
            true,
        ) {
            Ok(f) => frames.push(f),
            Err(e) => fail(&format!("golden 装配: {e}")),
        }
    }
    let double_run_bitexact = frames[0] == frames[1] && l1_device_bitexact && l3_device_bitexact;
    if !double_run_bitexact {
        failures.push("双跑位级一致破坏(帧/device 腿 digest 分叉)".into());
    }
    let frame_hl = match fb::assemble(&gb, ChainSwitches::ALL_ON, &l1_dev, &l2_host, &l3_hl, true) {
        Ok(f) => f,
        Err(e) => fail(&format!("hit lighting 全链装配: {e}")),
    };
    // 壁钟信息项(非判据;双跑相等性断言后回填)。
    for f in frames.iter_mut() {
        f.counters[fb::TraceLevel::L1ScreenTrace.slot()].wall_ns = l1_wall;
        f.counters[fb::TraceLevel::L2Swrt.slot()].wall_ns = l2_wall_ns;
        f.counters[fb::TraceLevel::L3Hwrt.slot()].wall_ns = l3s_wall;
    }
    let golden = frames[0].clone();
    // 级别覆盖充分性:golden 必须真实消费 L1/L2/L3 三级(否则强关臂空转 = RED)。
    let mut coverage_ok = true;
    for level in fb::TraceLevel::SELECTABLE {
        let served = golden.counters[level.slot()].pixels_served;
        if served == 0 {
            coverage_ok = false;
            failures.push(format!(
                "{} golden 服务像素数 = 0(级别覆盖不足,强关臂空转)",
                level.name()
            ));
        }
        let c = golden.counters[level.slot()];
        println!(
            "{TAG}: golden {} attempted={} hit={} served={} work={} hit_rate={:.4}",
            level.name(),
            c.rays_attempted,
            c.rays_hit,
            served,
            c.work_count,
            c.hit_rate()
        );
    }
    // ── 步骤 4:逐级强关双臂(回归可检测 = digest 必分叉 + ForcedOff 日志)──
    let mut force_off_results: Vec<(&str, bool)> = Vec::new();
    for (name, sw) in [
        (
            "force-off-l1",
            ChainSwitches {
                l1: false,
                l2: true,
                l3: true,
            },
        ),
        (
            "force-off-l2",
            ChainSwitches {
                l1: true,
                l2: false,
                l3: true,
            },
        ),
        (
            "force-off-l3",
            ChainSwitches {
                l1: true,
                l2: true,
                l3: false,
            },
        ),
    ] {
        let off = match fb::assemble(&gb, sw, &l1_dev, &l2_host, &l3_simple, true) {
            Ok(f) => f,
            Err(e) => fail(&format!("强关装配 {name}: {e}")),
        };
        let detectable = force_off_detectable(&golden, &off);
        // 反向 sabotage 探针:golden vs golden 必判「不可检测」(能红证明)。
        let sabotage_probe_red = !force_off_detectable(&golden, &golden);
        println!(
            "{TAG}: 强关臂 {name}: digest 分叉={} ForcedOff 日志={} sabotage 探针红={sabotage_probe_red}",
            golden.product_digest() != off.product_digest(),
            off.transitions
                .iter()
                .any(|r| r.cause == fb::TransitionCause::ForcedOff)
        );
        force_off_results.push((name, detectable && sabotage_probe_red));
        if args.red_arm.as_deref() == Some(name) {
            if detectable && sabotage_probe_red {
                println!("{TAG}: PASS red-arm {name}(独立检出:强关回归可检测 + 探针能红)");
                std::process::exit(0);
            }
            fail(&format!("red-arm {name} 失效(回归不可检测或探针不红)"));
        }
        if !(detectable && sabotage_probe_red) {
            failures.push(format!(
                "强关臂 {name} 失效(detectable={detectable} probe_red={sabotage_probe_red})"
            ));
        }
    }

    // ── 步骤 5:静默回退注入臂(门内需独立有效)──
    let silent_arm_ok = red_arm_silent_demotion();
    if !silent_arm_ok {
        failures.push("静默回退注入臂失效:抑日志 variant 未被审计拒".into());
    }
    if let Some(arm) = &args.red_arm {
        fail(&format!("unknown --red-arm {arm}"));
    }

    // ── 步骤 6:L4 not-triggered 登记(显式;不充绿)──
    // G31+ C12 注:三处入口已按锚字面参数化解锁——cornell 冻结 fixture 无
    // proxy 集装载(None)⇒ 本登记面维持 not-triggered/typed Err/计数恒零,
    // 门语义不变(半齐保护;四级链世界见 g31_hlod_l4_far_field harness)。
    let l4_reason = match fb::check_l4_trigger(None) {
        fb::L4TriggerState::NotTriggered { reason } => reason,
        fb::L4TriggerState::Ready { .. } => fail("cornell 无 proxy 集装载,L4 触发核验不得 Ready"),
    };
    let l4_no_sample = LegSample {
        hit: false,
        t: 0.0,
        rgb: fb::M98_SKY,
        work: 0,
    };
    let l4_serve_rejected = matches!(
        fb::l4_serve(None, &l4_no_sample),
        Err(fb::FbError::L4InterfaceNotReady(_))
    );
    let l4_counters_zero =
        golden.counters[fb::TraceLevel::L4FarField.slot()] == fb::LevelCounters::default();
    println!("{TAG}: L4 Far Field 登记 not-triggered({l4_reason});服务请求拒={l4_serve_rejected}");
    if !l4_serve_rejected || !l4_counters_zero {
        failures.push("L4 登记破坏:服务请求未拒或计数面非零".into());
    }

    // ── 步骤 7:M96 golden 对照腿(匹配深度 2)+ 门序消费锚 ──
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
        .entry(fb::M98_MATCHED_DEPTH)
        .unwrap_or_else(|e| fail(&format!("M97 深度带缺锚条目: {e}")))
        .m96_digest
        .clone();
    let (m96_img, m96_digest) = match run_m96(&scene, fb::M98_MATCHED_DEPTH, &spv_m96, &entry_m96) {
        Ok(v) => v,
        Err(e) => fail(&format!("M96 对照腿: {e}")),
    };
    let m96_cross_anchor = hex(&m96_digest) == m97_anchor;
    println!(
        "{TAG}: M96 depth={} digest={} 门序锚(M97 带)={}",
        fb::M98_MATCHED_DEPTH,
        hex(&m96_digest),
        m96_cross_anchor
    );
    if !m96_cross_anchor {
        failures.push(format!(
            "门序消费锚破坏:M96 depth={} digest {} ≠ M97 冻结带条目 {}",
            fb::M98_MATCHED_DEPTH,
            hex(&m96_digest),
            m97_anchor
        ));
    }

    // ── 步骤 8:六档产物 + rel_dev(solo×4 + 全链双档;匹配深度对 M96 golden)──
    let solo = |sw: ChainSwitches, l3: &[LegSample]| -> fb::ChainFrame {
        match fb::assemble(&gb, sw, &l1_dev, &l2_host, l3, true) {
            Ok(f) => f,
            Err(e) => fail(&format!("solo 装配: {e}")),
        }
    };
    let tier_frames: Vec<(&str, fb::ChainFrame)> = vec![
        (
            "l1_solo",
            solo(
                ChainSwitches {
                    l1: true,
                    l2: false,
                    l3: false,
                },
                &l3_simple,
            ),
        ),
        (
            "l2_solo",
            solo(
                ChainSwitches {
                    l1: false,
                    l2: true,
                    l3: false,
                },
                &l3_simple,
            ),
        ),
        (
            "l3_simple_solo",
            solo(
                ChainSwitches {
                    l1: false,
                    l2: false,
                    l3: true,
                },
                &l3_simple,
            ),
        ),
        (
            "l3_hit_lighting_solo",
            solo(
                ChainSwitches {
                    l1: false,
                    l2: false,
                    l3: true,
                },
                &l3_hl,
            ),
        ),
        ("chain_simple", golden.clone()),
        ("chain_hit_lighting", frame_hl.clone()),
    ];
    let mut measured: Vec<fb::M98BandEntry> = Vec::new();
    for (tier, frame) in &tier_frames {
        let dev = path_trace::rel_dev(&frame.rgb, &m96_img.rgb).expect("rel_dev 计算");
        println!(
            "{TAG}: tier={tier} digest={} rel_dev={dev:.6e}",
            hex(&frame.product_digest())
        );
        measured.push(fb::M98BandEntry {
            tier: tier.to_string(),
            chain_digest: hex(&frame.product_digest()),
            m96_digest: hex(&m96_digest),
            band_rel_dev: dev * fb::M98_BAND_MARGIN,
            measured_rel_dev: dev,
        });
    }

    // ── 步骤 9:freeze(写带)或 gate(比对带)──
    let mut digests_match = true;
    let mut depth_band_within = true;
    if args.freeze {
        let band = fb::M98DepthBand {
            frozen_at_utc: utc_now(),
            device_name: caps.device_name.clone(),
            scene: scene.name.to_string(),
            m96_anchor_digest: m97_anchor.clone(),
            entries: measured.clone(),
        };
        let out = args.band_out.clone().unwrap_or(args.band.clone());
        std::fs::write(&out, band.to_json()).unwrap_or_else(|e| fail(&format!("写带 {out}: {e}")));
        println!(
            "{TAG}: FREEZE 深度容差带已写 {out}(measured × {};provenance 全字段)",
            fb::M98_BAND_MARGIN
        );
    } else {
        let band_text = std::fs::read_to_string(&args.band)
            .unwrap_or_else(|e| fail(&format!("读深度容差带 {}: {e}", args.band)));
        let band = fb::M98DepthBand::parse(&band_text)
            .unwrap_or_else(|e| fail(&format!("深度容差带解析: {e}")));
        if m97_anchor != band.m96_anchor_digest {
            digests_match = false;
            failures.push("M97 门序锚条目与冻结带漂移".into());
        }
        for m in &measured {
            match band.check(&m.tier, &m.chain_digest, &m.m96_digest, m.measured_rel_dev) {
                Ok(()) => {}
                Err(e) => {
                    digests_match = false;
                    depth_band_within = false;
                    failures.push(e.to_string());
                }
            }
        }
        if digests_match && depth_band_within {
            println!("{TAG}: 深度带对照在带内(六档产物 digest 全等 + rel_dev ≤ 冻结带)");
        }
    }

    // ── 步骤 10:evidence(rurix.g9m98.fallback_chain.v1)──
    let silent_detected = silent_arm_ok;
    let checks: [(&str, bool); 12] = [
        ("double_run_bitexact", double_run_bitexact),
        ("l1_device_host_parity", l1_parity),
        ("level_coverage_all_used", coverage_ok),
        ("counters_non_empty_per_frame", coverage_ok),
        ("force_off_l1_detectable", force_off_results[0].1),
        ("force_off_l2_detectable", force_off_results[1].1),
        ("force_off_l3_detectable", force_off_results[2].1),
        ("silent_demotion_detected", silent_detected),
        (
            "l4_not_triggered_registered",
            l4_serve_rejected && l4_counters_zero,
        ),
        ("m96_cross_anchor", m96_cross_anchor),
        ("depth_band_within", digests_match && depth_band_within),
        ("validation_zero", true), // vk.rs lane 内 fail-closed:到此即零 ERROR
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    // 逐帧四级计数面(命中率/射线量/耗时代理 + 壁钟信息项)。
    let mut frames_json: Vec<String> = Vec::new();
    for (fi, f) in frames.iter().enumerate() {
        let mut levels_json: Vec<String> = Vec::new();
        for level in fb::TraceLevel::ALL {
            let c = f.counters[level.slot()];
            levels_json.push(format!(
                "\"{}\": {{\"rays_attempted\": {}, \"rays_hit\": {}, \"pixels_served\": {}, \"work_count\": {}, \"hit_rate\": \"{:.6e}\", \"wall_ns\": {}}}",
                level.name(),
                c.rays_attempted,
                c.rays_hit,
                c.pixels_served,
                c.work_count,
                c.hit_rate(),
                c.wall_ns
            ));
        }
        frames_json.push(format!(
            "{{\"frame\": {fi}, \"levels\": {{{}}}, \"transitions\": {}, \"usage_log_digest\": \"{}\"}}",
            levels_json.join(", "),
            f.transitions.len(),
            hex(&f.usage_log_digest())
        ));
    }
    let transitions_digest = hex(&golden.usage_log_digest());
    let cause_count =
        |c: fb::TransitionCause| golden.transitions.iter().filter(|r| r.cause == c).count();
    let mut digests_json: Vec<String> = vec![
        format!(
            "\"m96_depth{}\": \"{}\"",
            fb::M98_MATCHED_DEPTH,
            hex(&m96_digest)
        ),
        format!("\"usage_log\": \"{transitions_digest}\""),
    ];
    for m in &measured {
        digests_json.push(format!("\"{}\": \"{}\"", m.tier, m.chain_digest));
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
    let arms_json = force_off_results
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect::<Vec<_>>()
        .join(", ");
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m98.fallback_chain.v1\",\n  \
         \"subject\": \"g9_m98_fallback_chain\",\n  \
         \"spec_anchor\": \"RXS-0359\",\n  \
         \"assertion_id\": \"g9.p0.m98.tracing_fallback_chain\",\n  \
         \"milestone\": \"M98\",\n  \"wave\": \"G9.4\",\n  \
         \"status\": \"{status}\",\n  \
         \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \"require_real\": {}}},\n  \
         \"determinism_protocol\": {{\"seed_chain\": \"{}\", \"rng\": \"PCG32 单一流按索引寻址(rt::ref_tracer::Pcg32 同一实例;流为输入非结果,G7.4 先例)\", \
         \"primary_rays\": \"像素中心无 jitter;GBuffer 为 host 预传递输入\", \
         \"accumulation\": \"逐像素独立顺序累加(禁 atomic)\", \
         \"digest_domain\": \"sha256(rgb‖flags / 转移日志编码)\"}},\n  \
         \"chain_config\": {{\"l1_range\": {}, \"l2_range\": {}, \"l1_max_steps\": {}, \
         \"l1_depth_bias\": {}, \"matched_depth\": {}, \"m96_golden_spp\": {}, \
         \"time_counter_semantics\": \"work_count=确定性代理(L1 march 步数/L2 三角测试数/L3 有效射线查询发行量 1+hit);wall_ns=host 壁钟信息项(非判据,口径显式登记)\"}},\n  \
         \"frames\": [{}],\n  \
         \"level_usage\": {{\"transitions_total\": {}, \"cause_miss\": {}, \"cause_out_of_range\": {}, \"cause_forced_off\": {}, \
         \"no_silent_fallback\": true, \"audit\": \"fallback_chain::audit 独立重算逐条比对,无记录降级即 fail-closed Err\"}},\n  \
         \"l4_registration\": {{\"status\": \"not-triggered\", \"trigger_condition\": \"hlod_interface_ready\", \
         \"trigger_met\": false, \"reason\": \"{}\", \"serve_request_rejected\": {}, \"counters_zero\": {}}},\n  \
         \"force_off_arms\": {{{}}},\n  \
         \"l3_hit_lighting\": {{\"tier_digest\": \"{}\", \"mode\": \"hit_lighting(NEE 流采样+Lambert+阴影)\", \"wall_ns\": {}}},\n  \
         \"l1_host_parity\": {{\"structural_exact\": {}, \"float_residual_max_abs\": \"{:e}\", \
         \"semantics\": \"hit flags/march 步数 = 结构域精确硬判据;rgb/t 残差 = 单 texel 锁存邻域+设备 FMA 残差信息项(G7.5b 口径)\"}},\n  \
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
        fb::M98_SEED,
        fb::M98_L1_RANGE,
        fb::M98_L2_RANGE,
        fb::M98_L1_MAX_STEPS,
        fb::M98_L1_DEPTH_BIAS,
        fb::M98_MATCHED_DEPTH,
        fb::M98_M96_GOLDEN_SPP,
        frames_json.join(", "),
        golden.transitions.len(),
        cause_count(fb::TransitionCause::Miss),
        cause_count(fb::TransitionCause::OutOfRange),
        cause_count(fb::TransitionCause::ForcedOff),
        json_escape(l4_reason),
        l4_serve_rejected,
        l4_counters_zero,
        arms_json,
        hex(&frame_hl.product_digest()),
        l3hl_wall,
        l1_parity,
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
            "{TAG}: PASS 双跑位级一致 + 四级计数逐帧非空 + 强关双臂可检测 + 静默回退注入拒 + L4 not-triggered 登记 + 六档深度带内(validation={})",
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
