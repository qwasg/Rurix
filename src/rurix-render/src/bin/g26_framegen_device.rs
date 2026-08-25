//! G26.2 M-a/M-b FG/MFG device kernel harness(门 g26.p0.m_a.framegen_device_kernel
//! + g26.p0.m_b.framegen_device_bench_accounting;G26_CONTRACT §4.2 M-a/M-b 行逐字;
//! RFC-0043 §1/§2 判据事实源;g13_tsr_device 同模)。
//!
//! ## 集成路径
//!
//! bin-local 全部逻辑:`DeviceFramegen` 逐帧经 `rurix_rt::vk::run_compute`
//! (G12/G13 compute 派发面同车道)驱动 `kernels/g26_framegen.rx` 单 kernel。
//! 公式面与 host 金标准 `temporal::framegen::interpolate` 逐字同源;
//! **temporal/ 整目录 0-byte 不接线**(RFC-0043 §1.7 目录级冻结机核归 CI),
//! host 参考臂(interpolate/mfg_between)与 SSIM(temporal::ssim)只消费不改写。
//!
//! ## 合成运动场景(framegen.rs pure_translation 单测同模)
//!
//! 恒速 uv 平移:`render(k)` 以相位 k(帧单位)解析渲染任意时刻真值帧——
//! 相邻真渲帧对 (render(j), render(j+1)) 之间任意 t 的 GT = render(j+t);
//! mv 场 = 恒定平移 [shift_u, 0](prev→cur uv 位移,2 px/帧水平)。
//!
//! ## 判据面(RFC-0043 §1.4)
//!
//! - device vs host `interpolate` 同输入逐帧对拍:×2/×3/×4 三档 × 16 对帧序列
//!   逐插入帧逐像素最大绝对差 p100 ≤ 标定容差(标定腿程序产 threshold =
//!   measured × 2.0,禁手写;量化兜底断言 tol < RED_BIAS×0.5 归 CI);
//! - SSIM(device_interp, GT) > SSIM(frame_hold, GT) 逐插入帧程序产对照
//!   (temporal::ssim::ssim;frame_hold = 复制最近真渲帧 prev);
//! - device 双跑位级一致(同设备同驱动窗口固定输入两跑 digest 位级相等);
//! - host 参考臂对照恒跑(对拍基线即 host interpolate 本体)。
//!
//! ## RED 臂(RFC-0043 §1.4)
//!
//! - `kernel-bias`:params[5] 注入 RED_BIAS=0.05 输出面加性偏置 → 对拍必超容差;
//! - `seed-change`:合成场景相位偏移(输入流改动等价面)→ 末帧 digest 必异。
//!
//! ## M-b bench 腿(RFC-0043 §2)
//!
//! warmup + timed 逐帧墙钟(host Instant around 逐帧 device 全链路:打包 +
//! dispatch + 回读同步),×2/×3/×4 三档;`FgAccounting` 两口径类型面核验
//! (F9 双恒等式:presented = real + generated;real_render_fps 与 generated
//! 无关)——回归守护语义,不构成帧率对标通过线。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备 → `skipped_dev_env` JSON 退 0(非 fake pass;
//! `RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决);host 腿恒跑;
//! 判据不符 / RED 臂失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g26_framegen_device --spv <k.spv> --tol <F>            # 全档验证(默认)
//! g26_framegen_device --calibrate maxdiff --spv <k.spv>  # 标定腿
//! g26_framegen_device --bench x2|x3|x4 --spv <k.spv> [--warmup 10 --frames 150]
//! g26_framegen_device --red-arm kernel-bias|seed-change --spv <k.spv> [--tol <F>]
//! g26_framegen_device --probe --spv <k.spv> --tol <F> [--out <path>]  # soak 快车道
//! g26_framegen_device --host-only
//! ```

#![forbid(unsafe_code)]

use rurix_render::temporal::framegen::{mfg_between, FgAccounting, FrameGenParams};
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::ssim::ssim;
use rurix_rt::vk;

const TAG: &str = "[g26_framegen_device]";
/// 合成场景分辨率(host 对拍 + SSIM 逐帧程序产对照的固定栅格)。
const SCENE_W: u32 = 128;
const SCENE_H: u32 = 72;
/// 恒速平移:每帧 2 像素水平(uv 域 2/W;framegen.rs pure_translation 同模)。
const SHIFT_PX: f32 = 2.0;
/// 全档验证/标定腿:每档 16 对真渲帧序列(N=16)。
const PAIRS: u32 = 16;
/// probe/seed-change 快车道:8 对帧序列。
const PROBE_PAIRS: u32 = 8;
/// MFG 三档(×2/×3/×4 → inserted_per_pair 1/2/3;RFC-0043 §1.2 闭集)。
const TIERS: [(&str, u32); 3] = [("x2", 2), ("x3", 3), ("x4", 4)];
/// bench 协议(M141/M165 冻结):warmup 10 + timed 150。
const BENCH_WARMUP: u32 = 10;
const BENCH_TIMED: u32 = 150;
/// RED 臂注入幅(RFC-0043 §1.4 F4:RED_BIAS=0.05,g13 同值;标定容差绝对上界
/// = RED_BIAS × 0.5 由 CI 断言)。
const RED_BIAS: f32 = 0.05;
/// seed-change 臂:合成场景相位偏移(输入流改动等价面)。
const SEED_PHASE_OFFSET: f32 = 0.37;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 合成运动场景(恒速 uv 平移;render(k) 解析出任意相位真值帧)
// ---------------------------------------------------------------------------

fn shift_u() -> f32 {
    SHIFT_PX / SCENE_W as f32
}

/// 相位 k(帧单位)处解析渲染(framegen.rs pure_translation 单测 render 同式)。
fn render_phase(k: f32) -> ImageF32 {
    let su = shift_u();
    ImageF32::from_fn(SCENE_W, SCENE_H, 3, |x, y, ch| {
        let fx = (x as f32 + 0.5) / SCENE_W as f32 - k * su;
        let fy = (y as f32 + 0.5) / SCENE_H as f32;
        let base = 0.5
            + 0.35 * ((fx * 6.0) * std::f32::consts::PI).sin()
                * ((fy * 4.0) * std::f32::consts::PI).cos();
        (base + 0.05 * ch as f32).clamp(0.0, 1.0)
    })
}

/// 恒定平移 mv 场(prev→cur uv 位移;pure_translation 同模)。
fn const_mv() -> ImageF32 {
    let su = shift_u();
    ImageF32::from_fn(SCENE_W, SCENE_H, 2, |_, _, ch| if ch == 0 { su } else { 0.0 })
}

fn max_abs_diff(a: &ImageF32, b: &ImageF32) -> f64 {
    assert!(a.same_shape(b));
    a.data
        .iter()
        .zip(b.data.iter())
        .map(|(&x, &y)| (x - y).abs() as f64)
        .fold(0.0, f64::max)
}

fn sha256_frame(img: &ImageF32) -> String {
    let mut bytes = Vec::with_capacity(img.data.len() * 4);
    for &v in &img.data {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    rurix_pkg::sha256::hex_digest(&bytes)
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
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
// device 插值臂(bin-local;经 vk::run_compute 逐帧派发)
// ---------------------------------------------------------------------------

/// kernel 参数面打包(与 g26_framegen.rx 参数面逐字同源;16 f32 位级编码)。
fn pack_params(t: f32, inv_sigma2: f32, red_bias: f32) -> Vec<f32> {
    let mut v = vec![
        (SCENE_W * SCENE_H) as f32,
        SCENE_W as f32,
        SCENE_H as f32,
        t,
        inv_sigma2,
        red_bias,
    ];
    v.resize(16, 0.0);
    v
}

/// FG/MFG device 臂:逐帧经 vk::run_compute 单 dispatch 驱动 g26_framegen.rx。
struct DeviceFramegen {
    spv: Vec<u32>,
    entry: String,
    red_bias: f32,
}

impl DeviceFramegen {
    fn create(spv: Vec<u32>) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let entry = vk::entry_point_name(&spv).ok_or("SPV 无 OpEntryPoint")?;
        Ok(Self {
            spv,
            entry,
            red_bias: 0.0,
        })
    }

    /// device 单帧插值(host interpolate 同签名面;t 由调用侧按 host mfg 同式
    /// 算好传入——kernel 内不重算,F7;inv_sigma2 = 1/(σ·σ) host 同式预算)。
    fn interpolate(
        &self,
        prev: &ImageF32,
        cur: &ImageF32,
        mv: &ImageF32,
        t: f32,
        params: &FrameGenParams,
    ) -> ImageF32 {
        let inv_sigma2 = 1.0 / (params.consistency_sigma * params.consistency_sigma);
        let packed = pack_params(t, inv_sigma2, self.red_bias);
        let pc = (SCENE_W * SCENE_H) as usize;
        let mut bufs = vec![
            bytes_f32(&prev.data),
            bytes_f32(&cur.data),
            bytes_f32(&mv.data),
            bytes_f32(&packed),
            vec![0u8; pc * 12],
        ];
        vk::run_compute(
            &self.spv,
            &self.entry,
            &mut bufs,
            &[],
            [SCENE_W * SCENE_H, 1, 1],
        )
        .unwrap_or_else(|e| panic!("framegen dispatch 失败: {e}"));
        ImageF32 {
            w: SCENE_W,
            h: SCENE_H,
            c: 3,
            data: read_f32(&bufs[4]),
        }
    }

    /// device MFG:t_i = i/(n+1)(i=1..=n)host `mfg_between` 同式同序。
    fn mfg_between(
        &self,
        prev: &ImageF32,
        cur: &ImageF32,
        mv: &ImageF32,
        params: &FrameGenParams,
    ) -> Vec<ImageF32> {
        let n = params.inserted_per_pair;
        assert!((1..=3).contains(&n), "inserted_per_pair 闭集 1..=3");
        (1..=n)
            .map(|i| self.interpolate(prev, cur, mv, i as f32 / (n + 1) as f32, params))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// 序列跑面(device/host 同帧序同输入;phase_offset = seed-change 注入点)
// ---------------------------------------------------------------------------

/// 一档序列跑结果:逐插入帧(pair 序 × t 序)+ 逐帧相位(GT 解析用)。
struct TierRun {
    frames: Vec<ImageF32>,
    phases: Vec<f32>,
    prev_index: Vec<u32>,
}

enum Arm<'a> {
    Host,
    Device(&'a DeviceFramegen),
}

fn run_tier(arm: &Arm, mode_x: u32, pairs: u32, phase_offset: f32) -> TierRun {
    let n = mode_x - 1;
    let params = FrameGenParams {
        inserted_per_pair: n,
        ..Default::default()
    };
    let mv = const_mv();
    let mut frames = Vec::new();
    let mut phases = Vec::new();
    let mut prev_index = Vec::new();
    for j in 0..pairs {
        let prev = render_phase(j as f32 + phase_offset);
        let cur = render_phase((j + 1) as f32 + phase_offset);
        let inserted = match arm {
            Arm::Host => mfg_between(&prev, &cur, &mv, &params),
            Arm::Device(dev) => dev.mfg_between(&prev, &cur, &mv, &params),
        };
        for (idx, f) in inserted.into_iter().enumerate() {
            let t = (idx as u32 + 1) as f32 / (n + 1) as f32;
            frames.push(f);
            phases.push(j as f32 + t + phase_offset);
            prev_index.push(j);
        }
    }
    TierRun {
        frames,
        phases,
        prev_index,
    }
}

/// 序列 digest(逐帧 digest 串接再 sha256;双跑位级判据面)。
fn run_digest(run: &TierRun) -> String {
    let joined: String = run.frames.iter().map(sha256_frame).collect::<Vec<_>>().join(",");
    rurix_pkg::sha256::hex_digest(joined.as_bytes())
}

/// device vs host 逐帧对拍 p100(全插入帧最大绝对差)。
fn parity_p100(dev: &TierRun, host: &TierRun) -> f64 {
    assert_eq!(dev.frames.len(), host.frames.len());
    dev.frames
        .iter()
        .zip(host.frames.iter())
        .map(|(a, b)| max_abs_diff(a, b))
        .fold(0.0, f64::max)
}

/// SSIM(interp, GT) > SSIM(frame_hold, GT) 逐插入帧程序产对照;返回
/// (全帧通过?, 最小裕量 min(ssim_interp − ssim_hold))。frame_hold = 复制 prev。
fn ssim_beats_frame_hold(run: &TierRun, phase_offset: f32) -> (bool, f64) {
    let mut all_ok = true;
    let mut min_margin = f64::INFINITY;
    for ((frame, &phase), &pj) in run.frames.iter().zip(run.phases.iter()).zip(run.prev_index.iter()) {
        let gt = render_phase(phase);
        let hold = render_phase(pj as f32 + phase_offset);
        let s_interp = ssim(frame, &gt);
        let s_hold = ssim(&hold, &gt);
        let margin = s_interp - s_hold;
        if s_interp <= s_hold {
            all_ok = false;
        }
        if margin < min_margin {
            min_margin = margin;
        }
    }
    (all_ok, min_margin)
}

// ---------------------------------------------------------------------------
// JSON 出报(手写,零新依赖;g13_tsr_device 同模)
// ---------------------------------------------------------------------------

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

fn jstr(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn strs_json(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| jstr(s)).collect();
    format!("[{}]", inner.join(","))
}

fn base_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// 提交时刻(git log -1 %cI;同 commit 内确定——标定腿两跑位级一致的时间戳面)。
fn utc_now() -> String {
    std::process::Command::new("git")
        .args(["log", "-1", "--format=%cI"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn cells_digest(cells: &[String]) -> String {
    rurix_pkg::sha256::hex_digest(cells.join(",").as_bytes())
}

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

struct Args {
    spv: Option<String>,
    tol: f64,
    calibrate: Option<String>,
    bench: Option<String>,
    red_arm: Option<String>,
    probe: bool,
    host_only: bool,
    warmup: u32,
    frames: u32,
    out: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        spv: None,
        tol: 0.0,
        calibrate: None,
        bench: None,
        red_arm: None,
        probe: false,
        host_only: false,
        warmup: BENCH_WARMUP,
        frames: BENCH_TIMED,
        out: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            "--spv" => a.spv = it.next(),
            "--tol" => {
                a.tol = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --tol 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--tol 非 f64"))
            }
            "--calibrate" => a.calibrate = it.next(),
            "--bench" => a.bench = it.next(),
            "--red-arm" => a.red_arm = it.next(),
            "--probe" => a.probe = true,
            "--host-only" => a.host_only = true,
            "--warmup" => {
                a.warmup = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --warmup 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--warmup 非 u32"))
            }
            "--frames" => {
                a.frames = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --frames 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--frames 非 u32"))
            }
            "--out" => a.out = it.next(),
            other => fail(&format!("未知参数: {other}")),
        }
    }
    a
}

fn device_arm(args: &Args) -> Result<DeviceFramegen, String> {
    let spv = load_spv(args.spv.as_deref().unwrap_or_else(|| fail("缺 --spv")));
    DeviceFramegen::create(spv)
}

// ---------------------------------------------------------------------------
// 标定腿(device vs host 全档全帧 p100;两跑位级一致由 CI 裁决)
// ---------------------------------------------------------------------------

fn calibrate_leg(what: &str, args: &Args) -> ! {
    if what != "maxdiff" {
        fail(&format!("未知标定面: {what}(maxdiff)"));
    }
    let dev = match device_arm(args) {
        Ok(d) => d,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g26framegen.calibration_skip.v1\",\"what\":{},\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(what),
                jstr(&e)
            );
            std::process::exit(0);
        }
    };
    let mut p100 = 0.0f64;
    let mut cells = Vec::new();
    let mut total = 0usize;
    for (name, mode_x) in TIERS {
        let host_run = run_tier(&Arm::Host, mode_x, PAIRS, 0.0);
        let dev_run = run_tier(&Arm::Device(&dev), mode_x, PAIRS, 0.0);
        let cell = parity_p100(&dev_run, &host_run);
        total += dev_run.frames.len();
        cells.push(format!("\"{name}\":{cell:.15e}"));
        p100 = p100.max(cell);
    }
    let protocol = format!(
        "FG/MFG device vs host 金标准同输入逐帧对拍容差(×2/×3/×4 三档 × {PAIRS} 对恒速平移合成场景 {SCENE_W}×{SCENE_H},t_i=i/(n+1) host 同式传参,逐插入帧逐像素最大绝对差 p100;threshold = measured × 2.0 冻结 k,方向 max,禁手写;RFC-0043 §1.4)"
    );
    println!(
        "{{\"schema\":\"rurix.g26framegen.calibration_entry.v1\",\"entry_id\":\"g26.framegen_device.host_device_maxdiff_tol\",\"results\":{{\"trimmed_mean\":{:.15e}}},\"protocol\":{},\"sample_manifest\":{{\"count\":{},\"digest\":{}}},\"provenance\":{{\"gpu\":\"device\",\"backend\":\"framegen_device\",\"base_commit\":{}}},\"cells\":{{{}}},\"timestamp\":{}}}",
        p100,
        jstr(&protocol),
        total,
        jstr(&format!("sha256:{}", cells_digest(&cells))),
        jstr(&base_commit()),
        cells.join(","),
        jstr(&utc_now()),
    );
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// bench 腿(M-b:warmup+timed 逐帧墙钟 device 全链路 + FgAccounting 类型面核验)
// ---------------------------------------------------------------------------

fn bench_leg(tier: &str, args: &Args) -> ! {
    let Some((name, mode_x)) = TIERS.iter().find(|t| t.0 == tier) else {
        fail(&format!("未知档位: {tier}(x2|x3|x4)"))
    };
    let dev = match device_arm(args) {
        Ok(d) => d,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g26framegen.bench_skip.v1\",\"tier\":{},\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(name),
                jstr(&e)
            );
            std::process::exit(0);
        }
    };
    let n = mode_x - 1;
    let params = FrameGenParams {
        inserted_per_pair: n,
        ..Default::default()
    };
    let mv = const_mv();
    let total_gen = args.warmup + args.frames;
    let mut warmup_ms: Vec<f64> = Vec::new();
    let mut frame_ms: Vec<f64> = Vec::new();
    let mut generated: u64 = 0;
    // 真渲臂:host 解析渲染即本 harness 的真渲帧来源(墙钟入 real 口径)。
    let mut real_frames: u64 = 0;
    let mut real_render_seconds: f64 = 0.0;
    let mut generation_seconds: f64 = 0.0;
    let t0r = std::time::Instant::now();
    let mut prev = render_phase(0.0);
    real_render_seconds += t0r.elapsed().as_secs_f64();
    real_frames += 1;
    let mut first_digest = String::new();
    let mut digests = std::collections::BTreeSet::new();
    let mut pair: u32 = 0;
    'outer: loop {
        let t1r = std::time::Instant::now();
        let cur = render_phase((pair + 1) as f32);
        real_render_seconds += t1r.elapsed().as_secs_f64();
        real_frames += 1;
        for i in 1..=n {
            let t = i as f32 / (n + 1) as f32;
            let t0 = std::time::Instant::now();
            let out = dev.interpolate(&prev, &cur, &mv, t, &params);
            let el = t0.elapsed().as_secs_f64();
            generation_seconds += el;
            if generated < u64::from(args.warmup) {
                warmup_ms.push(el * 1000.0);
            } else {
                frame_ms.push(el * 1000.0);
            }
            let d = sha256_frame(&out);
            if generated == 0 {
                first_digest = d.clone();
            }
            digests.insert(d);
            generated += 1;
            if generated >= u64::from(total_gen) {
                break 'outer;
            }
        }
        prev = cur;
        pair += 1;
    }
    // FgAccounting 类型面核验(F9 双恒等式;temporal::framegen::FgAccounting 本体)。
    let acc = FgAccounting {
        real_frames,
        generated_frames: generated,
        real_render_seconds,
        generation_seconds,
    };
    let identity_presented = acc.presented_frames() == acc.real_frames + acc.generated_frames;
    let fps = acc.real_render_fps();
    let identity_fps_recompute = fps == acc.real_frames as f64 / acc.real_render_seconds;
    let perturbed = FgAccounting {
        generated_frames: acc.generated_frames + 997,
        ..acc
    };
    let identity_fps_isolated = perturbed.real_render_fps() == fps;
    let samples: Vec<String> = frame_ms.iter().map(|v| format!("{v:.6}")).collect();
    let warms: Vec<String> = warmup_ms.iter().map(|v| format!("{v:.6}")).collect();
    println!(
        "{{\"schema\":\"rurix.g26framegen.bench.v1\",\"tier\":{},\"width\":{},\"height\":{},\"inserted_per_pair\":{},\"warmup_count\":{},\"timed_count\":{},\"frame_ms\":[{}],\"warmup_ms\":[{}],\"accounting\":{{\"real_frames\":{},\"generated_frames\":{},\"real_render_seconds\":{:.17e},\"generation_seconds\":{:.17e},\"real_render_fps\":{:.17e},\"presented_fps\":{:.17e},\"presented_frames\":{}}},\"identity_presented_ok\":{},\"identity_real_fps_recompute_ok\":{},\"identity_real_fps_isolated_ok\":{},\"first_frame_digest\":{},\"distinct_frame_digests\":{},\"timer\":\"host Instant 墙钟 around 逐帧 device 全链路(打包 + dispatch + 回读同步);真渲口径 = host 解析渲染墙钟,两口径类型面分离永不混算\",\"base_commit\":{}}}",
        jstr(name),
        SCENE_W,
        SCENE_H,
        n,
        args.warmup,
        args.frames,
        samples.join(","),
        warms.join(","),
        acc.real_frames,
        acc.generated_frames,
        acc.real_render_seconds,
        acc.generation_seconds,
        fps,
        acc.presented_fps(),
        acc.presented_frames(),
        identity_presented,
        identity_fps_recompute,
        identity_fps_isolated,
        jstr(&first_digest),
        digests.len(),
        jstr(&base_commit()),
    );
    if !(identity_presented && identity_fps_recompute && identity_fps_isolated) {
        fail("FgAccounting 恒等式核验失败(F9)");
    }
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// RED 臂
// ---------------------------------------------------------------------------

fn red_arm_kernel_bias(args: &Args) -> Result<String, String> {
    // 输出面加性偏置注入 → device vs host 对拍必超容差(超容差静默即 RED 的
    // 机器兑现;tol 由 g26_budget 标定条目经 CI 传入)。
    let (_name, mode_x) = TIERS[0];
    let host_run = run_tier(&Arm::Host, mode_x, PAIRS, 0.0);
    let honest = device_arm(args)?;
    let honest_run = run_tier(&Arm::Device(&honest), mode_x, PAIRS, 0.0);
    let honest_p100 = parity_p100(&honest_run, &host_run);
    let mut tampered = device_arm(args)?;
    tampered.red_bias = RED_BIAS;
    let tampered_run = run_tier(&Arm::Device(&tampered), mode_x, PAIRS, 0.0);
    let tampered_p100 = parity_p100(&tampered_run, &host_run);
    let detected = if args.tol > 0.0 {
        tampered_p100 > args.tol
    } else {
        tampered_p100 > honest_p100 + f64::from(RED_BIAS) * 0.5
    };
    if !detected {
        return Err(format!(
            "kernel-bias 漏检:tampered p100={tampered_p100:.6e} vs tol={:.6e} honest={honest_p100:.6e}",
            args.tol
        ));
    }
    Ok(format!(
        "honest p100={honest_p100:.6e} tampered p100={tampered_p100:.6e} tol={:.6e}",
        args.tol
    ))
}

fn red_arm_seed_change(args: &Args) -> Result<String, String> {
    // 合成场景相位偏移(输入流改动等价面)→ 末帧 digest 必异(确定性协议
    // 漂移检出面;digest 比对机制必须能分辨输入流改动)。
    let (_name, mode_x) = TIERS[0];
    let honest = device_arm(args)?;
    let honest_run = run_tier(&Arm::Device(&honest), mode_x, PROBE_PAIRS, 0.0);
    let digest_a = sha256_frame(honest_run.frames.last().expect("至少一帧"));
    let shifted_run = run_tier(&Arm::Device(&honest), mode_x, PROBE_PAIRS, SEED_PHASE_OFFSET);
    let digest_b = sha256_frame(shifted_run.frames.last().expect("至少一帧"));
    if digest_a == digest_b {
        return Err("seed-change 漏检:场景相位偏移后末帧 digest 未变".into());
    }
    Ok(format!("末帧 digest 可分辨({} ≠ {})", &digest_a[..12], &digest_b[..12]))
}

// ---------------------------------------------------------------------------
// probe(soak 快车道:×2 档 8 帧序列对拍 + 双跑位级;单行 JSON)
// ---------------------------------------------------------------------------

fn probe_leg(args: &Args) -> ! {
    let dev = match device_arm(args) {
        Ok(d) => d,
        Err(e) => {
            let line = format!(
                "{{\"schema\":\"rurix.g26framegen.probe.v1\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(&e)
            );
            emit_probe(&line, args);
            std::process::exit(0);
        }
    };
    let (_name, mode_x) = TIERS[0];
    let host_run = run_tier(&Arm::Host, mode_x, PROBE_PAIRS, 0.0);
    let dev_run_a = run_tier(&Arm::Device(&dev), mode_x, PROBE_PAIRS, 0.0);
    let dev_run_b = run_tier(&Arm::Device(&dev), mode_x, PROBE_PAIRS, 0.0);
    let p100 = parity_p100(&dev_run_a, &host_run);
    let digest_a = run_digest(&dev_run_a);
    let digest_b = run_digest(&dev_run_b);
    let bitexact = digest_a == digest_b;
    let in_tol = args.tol > 0.0 && p100 <= args.tol;
    let state = if in_tol && bitexact { "pass" } else { "fail" };
    let line = format!(
        "{{\"schema\":\"rurix.g26framegen.probe.v1\",\"state\":{},\"tier\":\"x2\",\"pairs\":{},\"p100_vs_host\":{:.15e},\"tol\":{:.15e},\"in_tol\":{},\"bitexact\":{},\"digest\":{},\"last_frame_digest\":{},\"base_commit\":{}}}",
        jstr(state),
        PROBE_PAIRS,
        p100,
        args.tol,
        in_tol,
        bitexact,
        jstr(&digest_a),
        jstr(&sha256_frame(dev_run_a.frames.last().expect("至少一帧"))),
        jstr(&base_commit()),
    );
    emit_probe(&line, args);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

fn emit_probe(line: &str, args: &Args) {
    println!("{line}");
    if let Some(path) = &args.out {
        std::fs::write(path, format!("{line}\n"))
            .unwrap_or_else(|e| fail(&format!("写 --out {path}: {e}")));
    }
}

// ---------------------------------------------------------------------------
// host 腿(金标准锚;恒跑——host 参考臂自身 SSIM 程序产对照)
// ---------------------------------------------------------------------------

fn host_leg() -> (bool, f64) {
    let (_name, mode_x) = TIERS[0];
    let host_run = run_tier(&Arm::Host, mode_x, PROBE_PAIRS, 0.0);
    let (ok, margin) = ssim_beats_frame_hold(&host_run, 0.0);
    eprintln!("{TAG}: host 参考臂(×2 {PROBE_PAIRS} 对) ssim_beats_frame_hold={ok} min_margin={margin:.6}");
    (ok, margin)
}

// ---------------------------------------------------------------------------
// main(默认 = 全档验证)
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    if let Some(what) = &args.calibrate {
        calibrate_leg(what, &args);
    }
    if let Some(tier) = &args.bench {
        bench_leg(tier, &args);
    }
    if args.probe {
        probe_leg(&args);
    }
    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "kernel-bias" => red_arm_kernel_bias(&args),
            "seed-change" => red_arm_seed_change(&args),
            other => fail(&format!("未知 RED 臂: {other}(kernel-bias|seed-change)")),
        };
        match r {
            Ok(detail) => {
                eprintln!("{TAG}: red-arm {arm} 检出 — {detail}");
                println!(
                    "{{\"schema\":\"rurix.g26framegen.red_arm.v1\",\"arm\":{},\"detected\":true,\"detail\":{}}}",
                    jstr(arm),
                    jstr(&detail)
                );
                std::process::exit(0);
            }
            Err(e) if e.contains("不可用") || e.contains("DEV_ENV") => {
                println!(
                    "{{\"schema\":\"rurix.g26framegen.red_arm.v1\",\"arm\":{},\"detected\":false,\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                    jstr(arm),
                    jstr(&e)
                );
                std::process::exit(0);
            }
            Err(e) => fail(&format!("red-arm {arm} 失效(漏检): {e}")),
        }
    }

    // host 腿恒跑(host 参考臂 SSIM 程序产对照锚)。
    let (host_ssim_ok, host_margin) = host_leg();

    if args.host_only {
        let state = if host_ssim_ok { "pass" } else { "fail" };
        println!(
            "{{\"schema\":\"rurix.g26framegen.harness.v1\",\"mode\":\"host-only\",\"state\":{},\"host\":{{\"ssim_beats_frame_hold\":{},\"min_margin\":{:.10}}}}}",
            jstr(state),
            host_ssim_ok,
            host_margin
        );
        std::process::exit(if host_ssim_ok { 0 } else { 1 });
    }

    let dev = match device_arm(&args) {
        Ok(d) => d,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g26framegen.harness.v1\",\"mode\":\"device\",\"state\":\"skipped_dev_env\",\"skip_reason\":{},\"host\":{{\"ssim_beats_frame_hold\":{},\"min_margin\":{:.10}}}}}",
                jstr(&e),
                host_ssim_ok,
                host_margin
            );
            return;
        }
    };

    // ── 全档验证:三档 × 16 对帧,对拍 + SSIM 程序产对照 + 双跑位级 ──
    let mut problems: Vec<String> = Vec::new();
    let mut tier_json: Vec<String> = Vec::new();
    for (name, mode_x) in TIERS {
        let host_run = run_tier(&Arm::Host, mode_x, PAIRS, 0.0);
        let dev_run = run_tier(&Arm::Device(&dev), mode_x, PAIRS, 0.0);
        let p100 = parity_p100(&dev_run, &host_run);
        let (ssim_ok, margin) = ssim_beats_frame_hold(&dev_run, 0.0);
        let digest_a = run_digest(&dev_run);
        let dev_run_b = run_tier(&Arm::Device(&dev), mode_x, PAIRS, 0.0);
        let digest_b = run_digest(&dev_run_b);
        let bitexact = digest_a == digest_b;
        let in_tol = args.tol <= 0.0 || p100 <= args.tol;
        if !in_tol {
            problems.push(format!(
                "tier {name} device vs host p100={p100:.6e} 超容差 {:.6e}",
                args.tol
            ));
        }
        if !ssim_ok {
            problems.push(format!("tier {name} SSIM(interp,GT) 未严格优于 frame-hold"));
        }
        if !bitexact {
            problems.push(format!("tier {name} device 双跑非位级一致"));
        }
        tier_json.push(format!(
            "{}:{{\"pairs\":{},\"inserted\":{},\"p100_vs_host\":{:.15e},\"in_tol\":{},\"ssim_all_beat_frame_hold\":{},\"ssim_min_margin\":{:.10},\"bitexact\":{},\"digest\":{}}}",
            jstr(name),
            PAIRS,
            dev_run.frames.len(),
            p100,
            in_tol,
            ssim_ok,
            margin,
            bitexact,
            jstr(&digest_a),
        ));
        eprintln!(
            "{TAG}: tier {name} {SCENE_W}×{SCENE_H} inserted={} p100_vs_host={p100:.6e} ssim_ok={ssim_ok} margin={margin:.6} bitexact={bitexact}",
            dev_run.frames.len()
        );
    }
    if !host_ssim_ok {
        problems.push("host 参考臂 SSIM 对照异常(金标准面异常)".into());
    }
    let state = if problems.is_empty() { "pass" } else { "fail" };
    println!(
        "{{\"schema\":\"rurix.g26framegen.harness.v1\",\"mode\":\"device\",\"state\":{},\"problems\":{},\"host\":{{\"ssim_beats_frame_hold\":{},\"min_margin\":{:.10}}},\"tiers\":{{{}}},\"tol\":{:.15e},\"scene\":{{\"w\":{},\"h\":{},\"pairs\":{},\"shift_px_per_frame\":{}}}}}",
        jstr(state),
        strs_json(&problems),
        host_ssim_ok,
        host_margin,
        tier_json.join(","),
        args.tol,
        SCENE_W,
        SCENE_H,
        PAIRS,
        SHIFT_PX,
    );
    if !problems.is_empty() {
        std::process::exit(1);
    }
}
