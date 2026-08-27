//! G31+ 波 C Task C12 HLOD L4 Far Field 档 device harness(RXS-0359 四级链
//! L4 档;门 `g31.waveC.hlodl4`;G30 承接锚 M98-l4 行「HLOD proxy 追踪 device
//! 腿 + L4 计数器接入」合取两半兑现面)。
//!
//! ## 判据面
//!
//! - **scene = canonical 远场契约**(`fallback_chain::m98_l4_far_field_scene`:
//!   近场地板 + 中央盒 + 顶置光源,开放空间二次射线可逸出;远场 HLOD proxy
//!   五件最近面均 > M98_VIEW_DIST)——golden 必须真实消费全部四级(L1/L2/L3/
//!   L4 served 全 > 0,级别覆盖充分性,否则强关臂空转 = RED)。
//! - **L4 device 腿真跑**(`kernels/g31_hlod_l4_proxy_trace.rx` 纯 compute,
//!   L1 同构面不消费 TLAS):双跑位级一致 + 对 host 镜像
//!   (`fallback_chain::l4_leg_host`,逐字同源)**结构域精确对拍**(hit flags /
//!   proxy 下标 / 扫描计数逐像素硬判据)+ **rgb 位级全等硬判据**(烘焙辐射度
//!   纯数据搬运)+ t 残差信息项(除法 2.5 ULP 口径,G7.5b 先例)。
//! - **L4 计数器接入**(第二半):L4 槽位真实计数(attempted=链内像素/proxy
//!   命中数/服务像素数/扫描耗时代理)+ 可见 proxy 数(逐像素下标回读去重,
//!   契约 = proxies_total 逐件 ≥1)+ 切换次数(至 L4 转移按因分列)+ 覆盖率
//!   (hit_rate)——三处 fail-closed 入口解锁核验(check_l4_trigger=Ready /
//!   l4_serve=Ok / 计数非零)。
//! - **L4 on/off(L3 截断)对照**:on = 四级链 golden;off = L4Leg 强关
//!   (ForcedOff 记录 + L4 槽位归零 + 逸出像素截断天空);legacy = None 旧
//!   三级链。on/off digest 必分叉(proxy 贡献真实进入画面);off 与 legacy
//!   产物 digest 位级相等(截断语义等价)如实登记。
//! - **禁静默回退**(RXS-0359 L4):抑日志注入 variant 审计必 fail-closed
//!   Err;**逐级强关回归可检测**(L3 同律):强关 L4 digest 必分叉 +
//!   ForcedOff 记录;sabotage 探针(golden vs golden)必判不可检测(能红
//!   证明);**空接线冒充判红**:空 proxy 集接入 ⇒ InvalidConfig。
//! - **frame_ms measured**:完整帧管(host 腿 + L4 device 派发 + 装配)
//!   on/off 各 N 帧平均壁钟 + L4 kernel 纯派发壁钟,真实数字如实登记不设
//!   通过线(G6 无硬门纪律)。
//! - **确定性双跑位级一致**:GBuffer/双腿/golden 帧全部双跑逐位一致。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/W1(compute)能力链缺失 → 打印
//! `G31_HLOD_L4: SKIP DEV_ENV_DEGRADE` + `{"state":"skipped_dev_env",...}`
//! (退 0,非 fake pass;`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本
//! 层裁决);判据不符 / RED 臂失效 → FAIL 退 1。`RURIX_VK_VALIDATION=1`:
//! vk.rs lane 内 fail-closed;evidence 记 validation 模式。
//!
//! ## 用法
//!
//! ```text
//! g31_hlod_l4_far_field --spv-l4 <l4.spv> [--evidence <path>] [--frames 32]
//! g31_hlod_l4_far_field --red-arm silent-demotion|force-off-l4|tamper-proxy|empty-proxy
//! ```
//! (--red-arm 为纯 host 臂,无 device 依赖;退 0 = 检出成立)

use rurix_render::gi::fallback_chain as fb;
use rurix_render::gi::path_trace;
use rurix_rt::render_exec::{self, KernelWave};
use rurix_rt::vk;

const TAG: &str = "G31_HLOD_L4";

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn skip(msg: &str) -> ! {
    println!("{TAG}: SKIP DEV_ENV_DEGRADE {msg}");
    println!(
        "{{\"state\":\"skipped_dev_env\",\"reason\":\"{}\"}}",
        json_escape(msg)
    );
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
// host 腿装配(远场契约;全确定性)
// ---------------------------------------------------------------------------

struct HostWorld {
    scene: path_trace::PtScene,
    proxies: fb::L4ProxySet,
    gb: fb::GBuffer,
    l1: Vec<fb::LegSample>,
    l2: Vec<fb::LegSample>,
    l3: Vec<fb::LegSample>,
    l4: Vec<fb::LegSample>,
    l4_idx: Vec<u32>,
}

fn host_world() -> HostWorld {
    let (scene, proxies) = fb::m98_l4_far_field_scene();
    scene.validate().expect("远场契约场景校验");
    let gb = fb::gbuffer_prepass(&scene);
    let gb2 = fb::gbuffer_prepass(&scene);
    if gb != gb2 {
        fail("GBuffer 预传递双跑分叉(确定性协议违例)");
    }
    let l1 = fb::l1_leg_host(&scene, &gb);
    let l2 = fb::l2_leg_host(&scene, &gb.sec_o, &gb.sec_d);
    let l3 = fb::l3_leg_host(&scene, &gb, fb::L3ShadeMode::Simple);
    let (l4, l4_idx) = fb::l4_leg_host(&gb, &proxies);
    let (l4b, l4_idx_b) = fb::l4_leg_host(&gb, &proxies);
    if l4 != l4b || l4_idx != l4_idx_b {
        fail("L4 host 镜像双跑分叉(确定性协议违例)");
    }
    HostWorld {
        scene,
        proxies,
        gb,
        l1,
        l2,
        l3,
        l4,
        l4_idx,
    }
}

fn leg_on<'a>(w: &'a HostWorld) -> fb::L4Leg<'a> {
    fb::L4Leg {
        proxies: &w.proxies,
        samples: &w.l4,
        enabled: true,
    }
}

fn golden_host(w: &HostWorld) -> fb::ChainFrame {
    match fb::assemble_l4(
        &w.gb,
        fb::ChainSwitches::ALL_ON,
        Some(leg_on(w)),
        &w.l1,
        &w.l2,
        &w.l3,
        true,
    ) {
        Ok(f) => f,
        Err(e) => fail(&format!("四级 golden 装配: {e}")),
    }
}

// ---------------------------------------------------------------------------
// RED 臂(独立有效;纯 host;退 0 = 检出成立)
// ---------------------------------------------------------------------------

/// RED 臂 silent-demotion:抑日志注入 ⇒ 四级审计必 fail-closed SilentFallback。
fn red_arm_silent_demotion() -> bool {
    let w = host_world();
    let golden = golden_host(&w);
    let injected = fb::assemble_l4(
        &w.gb,
        fb::ChainSwitches::ALL_ON,
        Some(leg_on(&w)),
        &w.l1,
        &w.l2,
        &w.l3,
        false,
    );
    let detected = matches!(injected, Err(fb::FbError::SilentFallback(_)))
        && !golden.transitions.is_empty()
        && golden
            .transitions
            .iter()
            .any(|r| r.to == fb::TraceLevel::L4FarField);
    println!("{TAG}: RED 臂 silent-demotion(注入拒={detected})");
    detected
}

/// 强关可检测判定(digest 分叉 ∧ ForcedOff 记录;sabotage 探针同帧自比必
/// 判不可检测 = 能红证明)。
fn force_off_detectable(golden: &fb::ChainFrame, off: &fb::ChainFrame) -> bool {
    golden.product_digest() != off.product_digest()
        && off
            .transitions
            .iter()
            .any(|r| r.to == fb::TraceLevel::L4FarField && r.cause == fb::TransitionCause::ForcedOff)
}

/// RED 臂 force-off-l4:强关 L4 ⇒ 回归可检测;探针能红。
fn red_arm_force_off_l4() -> bool {
    let w = host_world();
    let golden = golden_host(&w);
    let leg_off = fb::L4Leg {
        enabled: false,
        ..leg_on(&w)
    };
    let off = match fb::assemble_l4(
        &w.gb,
        fb::ChainSwitches::ALL_ON,
        Some(leg_off),
        &w.l1,
        &w.l2,
        &w.l3,
        true,
    ) {
        Ok(f) => f,
        Err(e) => fail(&format!("强关 L4 装配: {e}")),
    };
    let detectable = force_off_detectable(&golden, &off);
    let probe_red = !force_off_detectable(&golden, &golden);
    let slot_zero =
        off.counters[fb::TraceLevel::L4FarField.slot()] == fb::LevelCounters::default();
    println!(
        "{TAG}: RED 臂 force-off-l4(digest 分叉={} ForcedOff 记录={} 探针红={probe_red} 槽位归零={slot_zero})",
        golden.product_digest() != off.product_digest(),
        off.transitions
            .iter()
            .any(|r| r.cause == fb::TransitionCause::ForcedOff),
    );
    detectable && probe_red && slot_zero
}

/// RED 臂 tamper-proxy:篡改单件 proxy 辐射度 ⇒ golden digest 必分叉
/// (构造性注入:proxy 0 有命中像素,消费路径命中有保证)。
fn red_arm_tamper_proxy() -> bool {
    let w = host_world();
    let golden = golden_host(&w);
    // 篡改:proxy 0 辐射度 r 通道 +0.01(契约内 proxy 0 投影覆盖非空 ⇒
    // 篡改进画面)。
    let mut forged = w.proxies.proxies.clone();
    forged[0].radiance[0] += 0.01;
    let forged_set = fb::L4ProxySet::new(forged).expect("篡改集合法");
    let (l4_f, _idx) = fb::l4_leg_host(&w.gb, &forged_set);
    let leg_f = fb::L4Leg {
        proxies: &forged_set,
        samples: &l4_f,
        enabled: true,
    };
    let golden_f = match fb::assemble_l4(
        &w.gb,
        fb::ChainSwitches::ALL_ON,
        Some(leg_f),
        &w.l1,
        &w.l2,
        &w.l3,
        true,
    ) {
        Ok(f) => f,
        Err(e) => fail(&format!("篡改装配: {e}")),
    };
    let detected = golden.product_digest() != golden_f.product_digest();
    let probe_red = golden.product_digest() == golden.product_digest();
    println!("{TAG}: RED 臂 tamper-proxy(digest 分叉={detected} 探针红={probe_red})");
    detected && probe_red
}

/// RED 臂 empty-proxy(空接线冒充判红):空 proxy 集接入 ⇒ 三处入口全部
/// fail-closed(NotTriggered / Err / InvalidConfig)。
fn red_arm_empty_proxy() -> bool {
    let w = host_world();
    let empty = fb::L4ProxySet::default();
    let trig_not_ready = matches!(
        fb::check_l4_trigger(Some(&empty)),
        fb::L4TriggerState::NotTriggered { .. }
    );
    let sample = fb::LegSample {
        hit: false,
        t: 0.0,
        rgb: fb::M98_SKY,
        work: 0,
    };
    let serve_rejected = matches!(
        fb::l4_serve(Some(&empty), &sample),
        Err(fb::FbError::L4InterfaceNotReady(_))
    );
    let leg_empty = fb::L4Leg {
        proxies: &empty,
        samples: &w.l4,
        enabled: true,
    };
    let assemble_rejected = matches!(
        fb::assemble_l4(
            &w.gb,
            fb::ChainSwitches::ALL_ON,
            Some(leg_empty),
            &w.l1,
            &w.l2,
            &w.l3,
            true,
        ),
        Err(fb::FbError::InvalidConfig(_))
    );
    let detected = trig_not_ready && serve_rejected && assemble_rejected;
    println!(
        "{TAG}: RED 臂 empty-proxy(触发 NotTriggered={trig_not_ready} 服务拒={serve_rejected} 装配拒={assemble_rejected})"
    );
    detected
}

// ---------------------------------------------------------------------------
// device 执行腿
// ---------------------------------------------------------------------------

/// L4 device 真跑(纯 compute proxy 追踪;无 TLAS——D2-Q9 射线流纪律)。
/// 返回 (腿样本, 逐像素 proxy 下标, 壁钟 ns)。
fn run_l4_device(
    gb: &fb::GBuffer,
    proxies: &fb::L4ProxySet,
    spv: &[u32],
    entry: &str,
) -> Result<(Vec<fb::LegSample>, Vec<u32>, u64), String> {
    let pixel_count = (gb.width * gb.height) as usize;
    let mut bufs: Vec<Vec<u8>> = vec![
        bytes_f32(&gb.sec_o),
        bytes_f32(&gb.sec_d),
        bytes_f32(&fb::pack_l4_proxies(proxies)),
        bytes_f32(&fb::pack_l4_params(pixel_count as u32, proxies.len() as u32)),
        vec![0u8; pixel_count * 12],
        vec![0u8; pixel_count * 16],
    ];
    let t0 = std::time::Instant::now();
    vk::run_compute(spv, entry, &mut bufs, &[], [pixel_count as u32, 1, 1])?;
    let wall_ns = t0.elapsed().as_nanos() as u64;
    let rgb = read_f32(&bufs[4]);
    let state = read_f32(&bufs[5]);
    let mut out = Vec::with_capacity(pixel_count);
    let mut idx = Vec::with_capacity(pixel_count);
    for i in 0..pixel_count {
        out.push(fb::LegSample {
            hit: state[i * 4] >= 0.5,
            t: state[i * 4 + 1],
            rgb: [rgb[i * 3], rgb[i * 3 + 1], rgb[i * 3 + 2]],
            work: state[i * 4 + 2] as u32,
        });
        idx.push(state[i * 4 + 3] as u32);
    }
    Ok((out, idx, wall_ns))
}

// ---------------------------------------------------------------------------
// 参数解析
// ---------------------------------------------------------------------------

struct Args {
    spv_l4: Option<String>,
    evidence: Option<String>,
    frames: u32,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = Args {
        spv_l4: None,
        evidence: None,
        frames: 32,
        red_arm: None,
    };
    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--spv-l4" => out.spv_l4 = Some(take(&mut i)),
            "--evidence" => out.evidence = Some(take(&mut i)),
            "--frames" => out.frames = take(&mut i).parse().unwrap_or_else(|_| fail("--frames 非整数")),
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
// main
// ---------------------------------------------------------------------------

fn main() {
    println!(
        "[g31_hlod_l4_far_field] G31+ 波 C Task C12 HLOD L4 Far Field 档 device harness(RXS-0359;门 g31.waveC.hlodl4)"
    );
    let args = parse_args();

    // ── RED 臂(纯 host,先行;退 0 = 检出成立)──
    if let Some(arm) = args.red_arm.as_deref() {
        let detected = match arm {
            "silent-demotion" => red_arm_silent_demotion(),
            "force-off-l4" => red_arm_force_off_l4(),
            "tamper-proxy" => red_arm_tamper_proxy(),
            "empty-proxy" => red_arm_empty_proxy(),
            other => fail(&format!("unknown --red-arm {other}")),
        };
        if detected {
            println!("{TAG}: PASS red-arm {arm}(独立检出成立)");
            std::process::exit(0);
        }
        fail(&format!("red-arm {arm} 未检出"));
    }

    // ── 步骤 0:host 预传递(无 device 依赖)──
    let w = host_world();
    let chain_pixels = w.gb.primary_hit.iter().filter(|&&b| b).count();
    println!(
        "{TAG}: host 预传递 pixels={} 链内={} proxies={}",
        w.gb.width * w.gb.height,
        chain_pixels,
        w.proxies.len()
    );
    let golden = golden_host(&w);
    let golden2 = golden_host(&w);
    if golden != golden2 {
        fail("四级 golden 装配双跑分叉(确定性协议违例)");
    }

    // ── 步骤 1:device 门(三态)──
    if !vk::vulkan_available() {
        skip("无 Vulkan loader(dev-env degrade)");
    }
    let caps = match render_exec::probe_device_caps() {
        Ok(c) => c,
        Err(e) => skip(&format!("无 Vulkan 物理设备({})", e.trim())),
    };
    if let Err(e) = render_exec::require_wave(&caps, KernelWave::W1) {
        skip(&format!("W1(compute)能力链缺失({e})"));
    }
    let validation_on = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    println!(
        "{TAG}: device=`{}` validation={}",
        caps.device_name,
        if validation_on { "on" } else { "off" }
    );
    let spv_l4_path = args
        .spv_l4
        .clone()
        .unwrap_or_else(|| fail("缺 --spv-l4 <l4.spv>"));
    let spv_l4 = load_spv(&spv_l4_path);
    let entry_l4 = vk::entry_point_name(&spv_l4).unwrap_or_else(|| fail("L4 SPV 无 OpEntryPoint"));
    println!("{TAG}: kernel entry l4=`{entry_l4}`");
    let mut failures: Vec<String> = Vec::new();

    // ── 步骤 2:L4 device 腿(双跑位级 + host 镜像对拍)──
    let (l4_dev, idx_dev, l4_wall) = match run_l4_device(&w.gb, &w.proxies, &spv_l4, &entry_l4) {
        Ok(v) => v,
        Err(e) => fail(&format!("L4 device: {e}")),
    };
    let (l4_dev_b, idx_dev_b, _) = match run_l4_device(&w.gb, &w.proxies, &spv_l4, &entry_l4) {
        Ok(v) => v,
        Err(e) => fail(&format!("L4 device 双跑: {e}")),
    };
    let device_leg_bitexact = l4_dev == l4_dev_b && idx_dev == idx_dev_b;
    if !device_leg_bitexact {
        failures.push("L4 device 双腿双跑位级分叉".into());
    }
    // 结构域精确对拍:hit flags / proxy 下标 / 扫描计数逐像素硬判据;
    // rgb 位级全等硬判据(纯数据搬运);t 残差信息项(除法 2.5 ULP 口径)。
    let mut parity_structural = true;
    let mut rgb_bitexact = true;
    let mut t_residual_max = 0.0f32;
    let mut t_residual_rel_max = 0.0f32;
    for (i, (d, h)) in l4_dev.iter().zip(w.l4.iter()).enumerate() {
        if d.hit != h.hit || d.work != h.work || idx_dev[i] != w.l4_idx[i] {
            parity_structural = false;
            println!(
                "{TAG}: L4 结构对拍分叉 px={i} dev=({},{},{}) host=({},{},{})",
                d.hit, d.work, idx_dev[i], h.hit, h.work, w.l4_idx[i]
            );
            break;
        }
        if d.rgb != h.rgb {
            rgb_bitexact = false;
            println!("{TAG}: L4 rgb 对拍分叉 px={i} dev={:?} host={:?}", d.rgb, h.rgb);
            break;
        }
        t_residual_max = t_residual_max.max((d.t - h.t).abs());
        if h.t > 0.0 {
            t_residual_rel_max = t_residual_rel_max.max((d.t - h.t).abs() / h.t);
        }
        if std::env::var("RURIX_L4_DEBUG").as_deref() == Ok("1") && (d.t - h.t).abs() > 1.0 {
            let o = [w.gb.sec_o[i * 3], w.gb.sec_o[i * 3 + 1], w.gb.sec_o[i * 3 + 2]];
            let dd = [w.gb.sec_d[i * 3], w.gb.sec_d[i * 3 + 1], w.gb.sec_d[i * 3 + 2]];
            println!(
                "{TAG}: DEBUG t 残差 px={i} idx={} dev_t={} host_t={} o={o:?} d={dd:?}",
                idx_dev[i], d.t, h.t
            );
        }
    }
    if !parity_structural {
        failures.push("L4 device/host 结构对拍分叉(hit/下标/计数不一致)".into());
    }
    if !rgb_bitexact {
        failures.push("L4 device/host rgb 位级分叉(纯数据搬运破坏)".into());
    }
    println!(
        "{TAG}: L4 device 双跑位级一致={} 结构对拍={} rgb 位级={} t 残差 max|Δ|={:.3e} rel={:.3e}(信息项)",
        device_leg_bitexact, parity_structural, rgb_bitexact, t_residual_max, t_residual_rel_max
    );

    // ── 步骤 3:四级 golden(device 腿)双跑位级 + 计数/覆盖核验 ──
    let leg_dev = fb::L4Leg {
        proxies: &w.proxies,
        samples: &l4_dev,
        enabled: true,
    };
    let golden_dev = match fb::assemble_l4(
        &w.gb,
        fb::ChainSwitches::ALL_ON,
        Some(leg_dev),
        &w.l1,
        &w.l2,
        &w.l3,
        true,
    ) {
        Ok(f) => f,
        Err(e) => fail(&format!("device 腿四级 golden 装配: {e}")),
    };
    let golden_dev_b = match fb::assemble_l4(
        &w.gb,
        fb::ChainSwitches::ALL_ON,
        Some(leg_dev),
        &w.l1,
        &w.l2,
        &w.l3,
        true,
    ) {
        Ok(f) => f,
        Err(e) => fail(&format!("device 腿四级双跑装配: {e}")),
    };
    let golden_bitexact = golden_dev == golden_dev_b;
    if !golden_bitexact {
        failures.push("四级 golden(device 腿)双跑位级分叉".into());
    }
    // device 腿 golden vs host 镜像 golden 位级(结构对拍 + rgb 位级蕴含
    // 装配位级——flags/rgb 全等;登记为最强对拍形)。
    let dev_eq_host_golden = golden_dev == golden;
    if !dev_eq_host_golden {
        failures.push("device 腿 golden ≠ host 镜像 golden(装配级分叉)".into());
    }
    // 级别覆盖充分性:golden 必须真实消费全部四级(否则强关臂空转 = RED)。
    let mut coverage_ok = true;
    for level in fb::TraceLevel::ALL {
        let served = golden_dev.counters[level.slot()].pixels_served;
        if served == 0 {
            coverage_ok = false;
            failures.push(format!(
                "{} golden 服务像素数 = 0(级别覆盖不足,强关臂空转)",
                level.name()
            ));
        }
        let c = golden_dev.counters[level.slot()];
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
    // L4 计数面(第二半):真实计数 + 切换次数 + 覆盖率 + 可见 proxy 数。
    let c4 = golden_dev.counters[fb::TraceLevel::L4FarField.slot()];
    let to_l4 = golden_dev
        .transitions
        .iter()
        .filter(|r| r.to == fb::TraceLevel::L4FarField)
        .count() as u64;
    let to_l4_miss = golden_dev
        .transitions
        .iter()
        .filter(|r| r.to == fb::TraceLevel::L4FarField && r.cause == fb::TransitionCause::Miss)
        .count() as u64;
    let to_l4_oor = golden_dev
        .transitions
        .iter()
        .filter(|r| {
            r.to == fb::TraceLevel::L4FarField && r.cause == fb::TransitionCause::OutOfRange
        })
        .count() as u64;
    // 可见 proxy 数(逐像素下标回读去重;链内命中像素)与逐件命中计数。
    let mut per_proxy_hits = vec![0u64; w.proxies.len()];
    for (i, s) in l4_dev.iter().enumerate() {
        if w.gb.primary_hit[i] && s.hit {
            per_proxy_hits[idx_dev[i] as usize] += 1;
        }
    }
    let proxies_visible = per_proxy_hits.iter().filter(|&&n| n > 0).count() as u64;
    let counters_wired = c4.rays_attempted == chain_pixels as u64
        && c4.rays_hit > 0
        && c4.pixels_served > 0
        && c4.work_count == chain_pixels as u64 * w.proxies.len() as u64
        && to_l4 == c4.pixels_served
        && proxies_visible == w.proxies.len() as u64;
    if !counters_wired {
        failures.push(format!(
            "L4 计数面破坏:attempted={} hit={} served={} work={} to_l4={} visible={proxies_visible}",
            c4.rays_attempted, c4.rays_hit, c4.pixels_served, c4.work_count, to_l4
        ));
    }
    // 三处 fail-closed 入口解锁核验(两半全齐 ⇒ Ready/Ok/计数非零)。
    let trigger_ready = matches!(
        fb::check_l4_trigger(Some(&w.proxies)),
        fb::L4TriggerState::Ready { proxies: 5, .. }
    );
    let sample0 = fb::LegSample {
        hit: false,
        t: 0.0,
        rgb: fb::M98_SKY,
        work: 0,
    };
    let serve_ok = fb::l4_serve(Some(&w.proxies), &sample0).is_ok();
    let counters_non_zero = c4.rays_attempted > 0 && c4.pixels_served > 0;
    if !(trigger_ready && serve_ok && counters_non_zero) {
        failures.push(format!(
            "三处入口解锁核验破坏:Ready={trigger_ready} Ok={serve_ok} 计数非零={counters_non_zero}"
        ));
    }

    // ── 步骤 4:L4 on/off(L3 截断)对照 ──
    let leg_off = fb::L4Leg {
        enabled: false,
        ..leg_dev
    };
    let off = match fb::assemble_l4(
        &w.gb,
        fb::ChainSwitches::ALL_ON,
        Some(leg_off),
        &w.l1,
        &w.l2,
        &w.l3,
        true,
    ) {
        Ok(f) => f,
        Err(e) => fail(&format!("off(L4 强关)装配: {e}")),
    };
    let legacy = match fb::assemble(&w.gb, fb::ChainSwitches::ALL_ON, &w.l1, &w.l2, &w.l3, true) {
        Ok(f) => f,
        Err(e) => fail(&format!("legacy 三级装配: {e}")),
    };
    let on_off_differs = golden_dev.product_digest() != off.product_digest();
    let off_forced_off = off
        .transitions
        .iter()
        .filter(|r| r.to == fb::TraceLevel::L4FarField && r.cause == fb::TransitionCause::ForcedOff)
        .count() as u64;
    let off_l4_served = off.counters[fb::TraceLevel::L4FarField.slot()].pixels_served;
    let legacy_l4_served = legacy.counters[fb::TraceLevel::L4FarField.slot()].pixels_served;
    let off_eq_legacy = off.product_digest() == legacy.product_digest();
    let off_flags_no_l4 = !off.flags.iter().any(|&f| f == fb::TraceLevel::L4FarField.flag());
    let on_off_ok = on_off_differs
        && off_forced_off > 0
        && off_l4_served == 0
        && legacy_l4_served == 0
        && off_eq_legacy
        && off_flags_no_l4;
    if !on_off_ok {
        failures.push(format!(
            "on/off 对照破坏:分叉={on_off_differs} ForcedOff={off_forced_off} off_served={off_l4_served} legacy_served={legacy_l4_served} off==legacy={off_eq_legacy} flags 无 L4={off_flags_no_l4}"
        ));
    }
    println!(
        "{TAG}: on/off 对照:on={} off={} legacy={} 分叉={} ForcedOff={} off==legacy(截断等价)={}",
        hex(&golden_dev.product_digest()),
        hex(&off.product_digest()),
        hex(&legacy.product_digest()),
        on_off_differs,
        off_forced_off,
        off_eq_legacy
    );

    // ── 步骤 5:静默回退注入臂 + 强关臂 + 篡改臂 + 空集臂(门内需独立有效)──
    let silent_ok = red_arm_silent_demotion();
    if !silent_ok {
        failures.push("静默回退注入臂失效".into());
    }
    let force_off_ok = red_arm_force_off_l4();
    if !force_off_ok {
        failures.push("强关 L4 臂失效".into());
    }
    let tamper_ok = red_arm_tamper_proxy();
    if !tamper_ok {
        failures.push("篡改 proxy 臂失效".into());
    }
    let empty_ok = red_arm_empty_proxy();
    if !empty_ok {
        failures.push("空 proxy 集 fail-closed 臂失效".into());
    }

    // ── 步骤 6:frame_ms measured(on/off 完整帧管 + L4 纯派发;如实登记)──
    let frames = args.frames.max(2);
    // L4 kernel 纯派发壁钟(同输入 K 次;device 派发成本独立口径)。
    let mut dispatch_ns: u128 = 0;
    for _ in 0..frames {
        let (_, _, ns) = match run_l4_device(&w.gb, &w.proxies, &spv_l4, &entry_l4) {
            Ok(v) => v,
            Err(e) => fail(&format!("L4 measured 派发: {e}")),
        };
        dispatch_ns += ns as u128;
    }
    let l4_dispatch_ms = dispatch_ns as f64 / 1e6 / f64::from(frames);
    // 完整帧管(GBuffer + host 三腿 + L4 host 镜像 + 装配)on vs off:
    // on = 四级链;off = legacy 三级(None 委托,L3 截断)。host 壁钟信息项
    // (非确定性判据;同机同窗 measured_local,如实登记不设通过线)。
    let mut on_ns: u128 = 0;
    let mut off_ns: u128 = 0;
    for _ in 0..frames {
        let t0 = std::time::Instant::now();
        let gb = fb::gbuffer_prepass(&w.scene);
        let l1 = fb::l1_leg_host(&w.scene, &gb);
        let l2 = fb::l2_leg_host(&w.scene, &gb.sec_o, &gb.sec_d);
        let l3 = fb::l3_leg_host(&w.scene, &gb, fb::L3ShadeMode::Simple);
        let (l4h, _i) = fb::l4_leg_host(&gb, &w.proxies);
        let leg = fb::L4Leg {
            proxies: &w.proxies,
            samples: &l4h,
            enabled: true,
        };
        let _f = fb::assemble_l4(&gb, fb::ChainSwitches::ALL_ON, Some(leg), &l1, &l2, &l3, true)
            .expect("on 帧装配");
        on_ns += t0.elapsed().as_nanos();
        let t1 = std::time::Instant::now();
        let gb = fb::gbuffer_prepass(&w.scene);
        let l1 = fb::l1_leg_host(&w.scene, &gb);
        let l2 = fb::l2_leg_host(&w.scene, &gb.sec_o, &gb.sec_d);
        let l3 = fb::l3_leg_host(&w.scene, &gb, fb::L3ShadeMode::Simple);
        let _f = fb::assemble(&gb, fb::ChainSwitches::ALL_ON, &l1, &l2, &l3, true)
            .expect("off 帧装配");
        off_ns += t1.elapsed().as_nanos();
    }
    let frame_on_ms = on_ns as f64 / 1e6 / f64::from(frames);
    let frame_off_ms = off_ns as f64 / 1e6 / f64::from(frames);
    println!(
        "{TAG}: frame_ms measured:L4 纯派发={l4_dispatch_ms:.4}ms 帧管 on={frame_on_ms:.4}ms off={frame_off_ms:.4}ms(各 {frames} 帧,如实登记)"
    );

    // ── 步骤 7:evidence(rurix.g31.hlod_l4_harness.v1)──
    let checks: [(&str, bool); 16] = [
        ("host_mirror_double_run", true), // host_world() 内双跑断言先行
        ("device_double_run_bitexact", device_leg_bitexact),
        ("device_host_parity_structural", parity_structural),
        ("device_host_rgb_bitexact", rgb_bitexact),
        ("golden_double_run_bitexact", golden_bitexact),
        ("device_golden_eq_host_golden", dev_eq_host_golden),
        ("level_coverage_all_four_used", coverage_ok),
        ("l4_counters_wired", counters_wired),
        ("unlock_trigger_ready", trigger_ready),
        ("unlock_serve_ok", serve_ok),
        ("unlock_counters_non_zero", counters_non_zero),
        ("on_off_relation_ok", on_off_ok),
        ("silent_demotion_detected", silent_ok),
        ("force_off_l4_detectable", force_off_ok),
        ("tamper_proxy_detected", tamper_ok),
        ("empty_proxy_fail_closed", empty_ok),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let per_proxy_json: Vec<String> = per_proxy_hits.iter().map(u64::to_string).collect();
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\n  \"schema\": \"rurix.g31.hlod_l4_harness.v1\",\n  \
         \"subject\": \"g31_hlod_l4_far_field\",\n  \
         \"spec_anchor\": \"RXS-0359\",\n  \
         \"assertion_id\": \"g31.waveC.hlodl4\",\n  \
         \"wave\": \"G31+.C\",\n  \
         \"status\": \"{status}\",\n  \
         \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \
         \"environment\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \"require_real\": {}}},\n  \
         \"scene\": {{\"name\": \"{}\", \"pixels\": {}, \"chain_pixels\": {}, \"proxies_total\": {}}},\n  \
         \"determinism_protocol\": {{\"seed_chain\": \"{}\", \"rng\": \"PCG32 单一流按索引寻址(流为输入非结果)\", \
         \"primary_rays\": \"像素中心无 jitter;GBuffer 为 host 预传递输入\", \
         \"accumulation\": \"逐像素独立顺序累加(禁 atomic)\", \
         \"digest_domain\": \"sha256(rgb‖flags / 转移日志编码)\"}},\n  \
         \"l4\": {{\"trigger_ready\": {}, \"serve_ok\": {}, \
         \"counters\": {{\"rays_attempted\": {}, \"rays_hit\": {}, \"pixels_served\": {}, \"work_count\": {}, \"hit_rate\": \"{:.6e}\", \"wall_ns\": {}}}, \
         \"transitions_to_l4\": {}, \"transitions_to_l4_miss\": {}, \"transitions_to_l4_out_of_range\": {}, \
         \"proxies_visible\": {}, \"per_proxy_hits\": [{}], \"coverage\": \"{:.6e}\"}},\n  \
         \"parity\": {{\"structural_exact\": {}, \"rgb_bitexact\": {}, \"t_residual_max_abs\": \"{:e}\", \"t_residual_rel_max\": \"{:e}\", \
         \"semantics\": \"hit flags/proxy 下标/扫描计数 = 整数判定域精确硬判据;rgb = 纯数据搬运位级硬判据;t = 除法 2.5 ULP 残差信息项(G7.5b 口径;近平行命中 t ~ 1e9 量级,相对残差为判读口径)\"}},\n  \
         \"double_run\": {{\"device_leg_bitexact\": {}, \"golden_bitexact\": {}, \"device_golden_eq_host_golden\": {}}},\n  \
         \"on_off\": {{\"on_digest\": \"{}\", \"off_digest\": \"{}\", \"legacy_digest\": \"{}\", \
         \"digest_differs\": {}, \"off_forced_off_records\": {}, \"off_l4_served\": {}, \"legacy_l4_served\": {}, \
         \"off_eq_legacy_product\": {}}},\n  \
         \"frame_ms\": {{\"l4_device_dispatch_ms\": \"{:.6e}\", \"frame_on_ms\": \"{:.6e}\", \"frame_off_ms\": \"{:.6e}\", \
         \"frames\": {}, \"note\": \"完整帧管(GBuffer+host 三腿+L4 镜像+装配)on/off 各 N 帧平均壁钟 + L4 kernel 纯派发壁钟;同机同窗 measured_local,如实登记不设通过线(G6 无硬门纪律)\"}},\n  \
         \"red_arms\": {{\"silent_demotion\": {}, \"force_off_l4\": {}, \"tamper_proxy\": {}, \"empty_proxy\": {}}},\n  \
         \"checks\": {{{}}},\n  \
         \"commands\": [{}],\n  \
         \"failures\": [{}]\n}}",
        utc_now(),
        json_escape(&base_commit),
        json_escape(&caps.device_name),
        if validation_on { "on" } else { "off" },
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        w.scene.name,
        w.gb.width * w.gb.height,
        chain_pixels,
        w.proxies.len(),
        fb::M98_SEED,
        trigger_ready,
        serve_ok,
        c4.rays_attempted,
        c4.rays_hit,
        c4.pixels_served,
        c4.work_count,
        c4.hit_rate(),
        l4_wall,
        to_l4,
        to_l4_miss,
        to_l4_oor,
        proxies_visible,
        per_proxy_json.join(", "),
        c4.hit_rate(),
        parity_structural,
        rgb_bitexact,
        t_residual_max,
        t_residual_rel_max,
        device_leg_bitexact,
        golden_bitexact,
        dev_eq_host_golden,
        hex(&golden_dev.product_digest()),
        hex(&off.product_digest()),
        hex(&legacy.product_digest()),
        on_off_differs,
        off_forced_off,
        off_l4_served,
        legacy_l4_served,
        off_eq_legacy,
        l4_dispatch_ms,
        frame_on_ms,
        frame_off_ms,
        frames,
        silent_ok,
        force_off_ok,
        tamper_ok,
        empty_ok,
        checks_json.join(", "),
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
            "{TAG}: PASS 双跑位级 + L4 device/host 结构对拍 + rgb 位级 + 四级覆盖全消费 + 计数接入 + 三入口解锁 + on/off 分叉 + 四 RED 臂(validation={})",
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
