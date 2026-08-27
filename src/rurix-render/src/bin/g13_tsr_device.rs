//! G13.3 M-b(M168) 自研 TSR device 化 harness(门 g13.p0.m_b.tsr_device_kernel;
//! G13_CONTRACT §4.2 M-b 行逐字 / G-G13-5;G13_ACCEPTANCE_MAP §1 M-b 行;
//! RFC-0016 §4.H2/H3;RXS-0387/0388 口径继承;BENCH_PROTOCOL §3 50×3 trimmed
//! mean 协议沿 M141/M165 字面)。
//!
//! ## 集成路径
//!
//! `TsrDeviceBackend` = **bin-local adapter** 实现 [`UpscaleBackend`] 冻结面
//! (RFC-0016 §4.0-3),逐帧经 `rurix_rt::vk::run_compute`(G12 PT megakernel 车道
//! 同一 compute 派发面)驱动 .rx kernel 双腿——`kernels/g13_tsr_resample.rx`
//! (jitter 对齐 Catmull-Rom 重采样 × exposure 转显示域 + 抗振铃 4×4 min/max
//! 钳制 + 深度最近邻上采样 + 亮度导出)与 `kernels/g13_tsr_resolve.rx`(闪烁
//! 时域分析 EMA + MV 最近邻上采样 + 历史双线性重投影 + 深度相对差验证 +
//! YCoCg 3×3 AABB 闪烁松弛钳制 + reactive 优先 alpha 调制混合)——公式面与
//! host 金标准 `temporal::tsr::TsrUpscaler` 逐字同源,`temporal/` 底座与 trait
//! 签名面 0-byte(目录级 git diff vs G13.0 不可变 ref 8c5dc5ee 机核)。
//!
//! ## 判据面
//!
//! - device vs host 金标准**同输入逐帧对拍**:三档(50% 640×360 / 67% 858×482 /
//!   100% 1280×720,统一输出 1280×720)× 32 帧 Halton jitter 静态收敛序列,逐帧
//!   逐像素最大绝对差 ≤ 标定容差(g13_budget 标定条目,threshold = measured
//!   × 2.0 冻结 k,标定腿两跑位级一致程序产,禁手写 P-09);超容差静默即 RED;
//! - 三档质量/帧时 measured 对照:质量 = 终帧 SSIM deficit(1−SSIM,RXS-0387
//!   LDR 8×8 窗口径)对拍 4×4 超采样参照;帧时 = host Instant 墙钟 around
//!   逐帧 device 全链路(打包 + 双 dispatch + 回读同步 + 状态轮换),warmup 10
//!   + timed 150(3 块 × 50,M141/M165 冻结统计口径);全入 g13_budget
//!   measured_local 零 estimated——**回归守护语义,不构成超分画质/帧率对标
//!   通过线**(G13 不设画质通过线归 G15;正式帧率对标锚定 G14);
//! - 固定 seed 位级确定性协议:同档同参双跑 digest 位级一致;
//! - RURIX_VK_VALIDATION=1:层在跑,CI 捕获 stderr 扫 VUID/Validation Error
//!   token 计数 = 0(覆盖口径 evidence 登记:run_compute 车道层在跑默认
//!   stderr 汇聚面扫描)。
//!
//! ## RED 臂(契约判据字面)
//!
//! - `kernel-bias`:device kernel 输出面加性偏置(params[17])——device vs host
//!   对拍必超容差检出(超容差静默即 RED 的机器兑现);
//! - `seed-change`:jitter 序列改 seed(相位偏移 0.13/0.29)——终帧 digest 与
//!   诚实跑必异检出(确定性协议漂移检出面);
//! - estimated 冒充 measured:evidence/budget 面 estimated 注入必拒(CI 脚本
//!   selftest 合成红臂承载)。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备 → device 腿 `SKIP DEV_ENV_DEGRADE`(退 0,非 fake
//! pass;`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层裁决);host 腿
//! 恒跑。判据不符 / RED 臂失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g13_tsr_device --spv-resample <a.spv> --spv-resolve <b.spv> --tol <F>
//!     [--band-deficit-50 F --band-deficit-67 F --band-deficit-100 F]
//! g13_tsr_device --calibrate maxdiff|quality --spv-resample .. --spv-resolve ..
//! g13_tsr_device --bench 50|67|100 --spv-resample .. --spv-resolve .. [--warmup 10 --frames 150]
//! g13_tsr_device --red-arm kernel-bias|seed-change --spv-resample .. --spv-resolve .. --tol <F>
//! g13_tsr_device --host-only
//! ```

#![forbid(unsafe_code)]

use rurix_render::temporal::common::jitter_sequence;
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::ssim::ssim;
use rurix_render::temporal::tsr::{TsrParams, TsrUpscaler};
use rurix_render::temporal::upscale::{UpscaleBackend, UpscaleInputs};
use rurix_rt::vk;

const TAG: &str = "[g13_tsr_device]";
const OUT_W: u32 = 1280;
const OUT_H: u32 = 720;
/// 三档内部分辨率(50%/67%/100%,统一输出 1280×720;契约 M-b 行字面)。
const TIERS: [(&str, u32, u32); 3] = [("50", 640, 360), ("67", 858, 482), ("100", 1280, 720)];
const CONVERGE_FRAMES: u32 = 32;
/// bench 协议(M141/M165 冻结):warmup 10 + timed 150 = 3 块 × 50。
const BENCH_WARMUP: u32 = 10;
const BENCH_TIMED: u32 = 150;
/// RED 臂注入幅(kernel-bias 输出面加性偏置;远超任何容差带)。
const RED_BIAS: f32 = 0.05;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 静态合成场景(与 tsr.rs 静态收敛门禁/M-a harness 同 shade 面——同一事实源)
// ---------------------------------------------------------------------------

fn shade(fx: f32, fy: f32) -> [f32; 3] {
    let check = (((fx + 3.7) / 8.0).floor() as i32 + ((fy + 3.7) / 8.0).floor() as i32) & 1;
    let mut base = 0.2 + 0.55 * check as f32;
    if fx + fy > 84.0 {
        base = 1.0 - base;
    }
    let line = (fx + 0.3) % 6.0 < 1.0;
    let v = if line { base * 0.35 } else { base };
    let grad = 0.08 * (fx * 0.05).sin() * (fy * 0.07).cos();
    [
        (v + grad).clamp(0.0, 1.0),
        (0.85 * v + 0.6 * grad).clamp(0.0, 1.0),
        (0.7 * v - grad).clamp(0.0, 1.0),
    ]
}

fn render_input(w: u32, h: u32, scale: f32, jitter: [f32; 2]) -> ImageF32 {
    ImageF32::from_fn(w, h, 3, |x, y, ch| {
        shade(
            (x as f32 + 0.5 + jitter[0]) * scale,
            (y as f32 + 0.5 + jitter[1]) * scale,
        )[ch as usize]
    })
}

/// 参照:输出分辨率 4×4 超采样(收敛对拍金标准,报告7 §5 / RXS-0387 口径)。
fn render_reference(w: u32, h: u32) -> ImageF32 {
    ImageF32::from_fn(w, h, 3, |x, y, ch| {
        let mut acc = 0.0f32;
        for sy in 0..4 {
            for sx in 0..4 {
                acc += shade(
                    x as f32 + (sx as f32 + 0.5) / 4.0,
                    y as f32 + (sy as f32 + 0.5) / 4.0,
                )[ch as usize];
            }
        }
        acc / 16.0
    })
}

fn const_depth(w: u32, h: u32) -> ImageF32 {
    ImageF32::from_fn(w, h, 1, |_, _, _| 0.5)
}

fn zero_mv(w: u32, h: u32) -> ImageF32 {
    ImageF32::new(w, h, 2)
}

fn inputs_for<'a>(
    color: &'a ImageF32,
    depth: &'a ImageF32,
    mv: &'a ImageF32,
    out_size: (u32, u32),
    jitter: [f32; 2],
    frame_index: u32,
    reset: bool,
) -> UpscaleInputs<'a> {
    UpscaleInputs {
        color,
        depth,
        mv,
        reactive: None,
        exposure: 1.0,
        jitter,
        output_size: out_size,
        frame_index,
        reset,
    }
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
// device backend(bin-local adapter;UpscaleBackend 冻结面;公式面 = tsr.rs 同源)
// ---------------------------------------------------------------------------

/// TSR kernel 参数面打包(与 .rx 双腿参数面逐字同源;32 f32 位级编码)。
#[allow(clippy::too_many_arguments)]
fn pack_tsr_params(
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
    jitter: [f32; 2],
    exposure: f32,
    has_history: bool,
    has_reactive: bool,
    red_bias: f32,
    red_passthrough: bool,
) -> Vec<f32> {
    let p = TsrParams::default();
    let mut v = vec![
        (ow * oh) as f32,
        ow as f32,
        oh as f32,
        iw as f32,
        ih as f32,
        jitter[0],
        jitter[1],
        exposure,
        if has_history { 1.0 } else { 0.0 },
        p.base_alpha,
        p.min_alpha,
        2.0 / (p.flicker_window_frames as f32 + 1.0),
        p.flicker_tighten,
        p.flicker_deadzone_abs,
        p.flicker_deadzone_rel,
        p.depth_rel_tol,
        if has_reactive { 1.0 } else { 0.0 },
        red_bias,
        if red_passthrough { 1.0 } else { 0.0 },
    ];
    v.resize(32, 0.0);
    v
}

/// 自研 TSR device 后端:逐帧经 vk::run_compute 双 dispatch 驱动 .rx 双腿,
/// 历史状态(颜色/深度/亮度/翻转符号/闪烁分数,全部输出分辨率)host 侧双缓冲
/// 轮换(G12 PT megakernel 车道 host 簿记同模)。
struct TsrDeviceBackend {
    spv_resample: Vec<u32>,
    spv_resolve: Vec<u32>,
    entry_resample: String,
    entry_resolve: String,
    output_size: Option<(u32, u32)>,
    hist_color: Vec<f32>,
    hist_depth: Vec<f32>,
    prev_luma: Vec<f32>,
    prev_sign: Vec<f32>,
    prev_score: Vec<f32>,
    red_bias: f32,
    red_passthrough: bool,
}

impl TsrDeviceBackend {
    fn create(spv_resample: Vec<u32>, spv_resolve: Vec<u32>) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let entry_resample =
            vk::entry_point_name(&spv_resample).ok_or("resample SPV 无 OpEntryPoint")?;
        let entry_resolve =
            vk::entry_point_name(&spv_resolve).ok_or("resolve SPV 无 OpEntryPoint")?;
        Ok(Self {
            spv_resample,
            spv_resolve,
            entry_resample,
            entry_resolve,
            output_size: None,
            hist_color: Vec::new(),
            hist_depth: Vec::new(),
            prev_luma: Vec::new(),
            prev_sign: Vec::new(),
            prev_score: Vec::new(),
            red_bias: 0.0,
            red_passthrough: false,
        })
    }

    fn clear_state(&mut self) {
        self.output_size = None;
        self.hist_color = Vec::new();
        self.hist_depth = Vec::new();
        self.prev_luma = Vec::new();
        self.prev_sign = Vec::new();
        self.prev_score = Vec::new();
    }
}

impl UpscaleBackend for TsrDeviceBackend {
    fn name(&self) -> &str {
        "tsr_device"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        // 输出分辨率变化 → 自动丢弃历史(接口契约,host 金标准同语义)。
        if self.output_size != Some((ow, oh)) {
            self.clear_state();
            self.output_size = Some((ow, oh));
        }
        let pc = (ow * oh) as usize;
        let has_history = !inputs.reset && !self.hist_color.is_empty();
        let params = pack_tsr_params(
            iw,
            ih,
            ow,
            oh,
            inputs.jitter,
            inputs.exposure,
            has_history,
            inputs.reactive.is_some(),
            self.red_bias,
            self.red_passthrough,
        );
        let zero_in = vec![0.0f32; (iw * ih) as usize];
        let reactive = inputs.reactive.map(|r| &r.data[..]).unwrap_or(&zero_in);
        // ── 腿 ①:重采样(当前帧 → 输出网格显示域 + 亮度 + 深度最近邻)──
        let mut bufs = vec![
            bytes_f32(&inputs.color.data),
            bytes_f32(&inputs.depth.data),
            bytes_f32(&params),
            vec![0u8; pc * 12],
            vec![0u8; pc * 4],
            vec![0u8; pc * 4],
        ];
        vk::run_compute(
            &self.spv_resample,
            &self.entry_resample,
            &mut bufs,
            &[],
            [ow * oh, 1, 1],
        )
        .unwrap_or_else(|e| panic!("TSR resample dispatch 失败: {e}"));
        let cur_rgb = read_f32(&bufs[3]);
        let cur_luma = read_f32(&bufs[4]);
        let depth_hi = read_f32(&bufs[5]);
        // ── 腿 ②:resolve(闪烁 EMA + 重投影 + 验证 + AABB + 混合)──
        // 首帧历史槽位传当前帧面(has_history=0,kernel 不消费)。
        let hist_color = if has_history {
            self.hist_color.clone()
        } else {
            cur_rgb.clone()
        };
        let hist_depth = if has_history {
            self.hist_depth.clone()
        } else {
            depth_hi.clone()
        };
        let prev_luma = if has_history {
            self.prev_luma.clone()
        } else {
            cur_luma.clone()
        };
        let prev_sign = if has_history {
            self.prev_sign.clone()
        } else {
            vec![0.0f32; pc]
        };
        let prev_score = if has_history {
            self.prev_score.clone()
        } else {
            vec![0.0f32; pc]
        };
        let mut bufs2 = vec![
            bytes_f32(&cur_rgb),
            bytes_f32(&cur_luma),
            bytes_f32(&depth_hi),
            bytes_f32(&inputs.mv.data),
            bytes_f32(reactive),
            bytes_f32(&hist_color),
            bytes_f32(&hist_depth),
            bytes_f32(&prev_luma),
            bytes_f32(&prev_sign),
            bytes_f32(&prev_score),
            bytes_f32(&params),
            vec![0u8; pc * 12],
            vec![0u8; pc * 4],
            vec![0u8; pc * 4],
        ];
        vk::run_compute(
            &self.spv_resolve,
            &self.entry_resolve,
            &mut bufs2,
            &[],
            [ow * oh, 1, 1],
        )
        .unwrap_or_else(|e| panic!("TSR resolve dispatch 失败: {e}"));
        let out_color = read_f32(&bufs2[11]);
        let out_sign = read_f32(&bufs2[12]);
        let out_score = read_f32(&bufs2[13]);
        // 双缓冲:本帧输出即下帧历史(host 金标准双缓冲语义同字面)。
        self.hist_color = out_color.clone();
        self.hist_depth = depth_hi;
        self.prev_luma = cur_luma;
        self.prev_sign = out_sign;
        self.prev_score = out_score;
        ImageF32 {
            w: ow,
            h: oh,
            c: 3,
            data: out_color,
        }
    }

    fn reset_history(&mut self) {
        self.clear_state();
    }
}

// ---------------------------------------------------------------------------
// 静态收敛跑面(device/host 同帧序同输入)
// ---------------------------------------------------------------------------

struct ConvergeRun {
    outs: Vec<ImageF32>,
}

fn run_static(backend: &mut dyn UpscaleBackend, in_w: u32, in_h: u32, frames: u32) -> ConvergeRun {
    let scale = OUT_W as f32 / in_w as f32;
    let depth = const_depth(in_w, in_h);
    let mv = zero_mv(in_w, in_h);
    let jitters = jitter_sequence(frames);
    let mut outs = Vec::new();
    for (i, &j) in jitters.iter().enumerate() {
        let cur = render_input(in_w, in_h, scale, j);
        let inp = inputs_for(&cur, &depth, &mv, (OUT_W, OUT_H), j, i as u32, i == 0);
        outs.push(backend.upscale(&inp));
    }
    ConvergeRun { outs }
}

struct ConvergeMetrics {
    final_ssim: f64,
    deficit: f64,
    monotonic_mse: bool,
    digest: String,
}

fn measure(run: &ConvergeRun, reference: &ImageF32) -> ConvergeMetrics {
    let last = run.outs.last().expect("至少一帧");
    let s = ssim(last, reference);
    let mses: Vec<f64> = run
        .outs
        .iter()
        .map(|o| ImageF32::mse(o, reference))
        .collect();
    let n = mses.len();
    let seg = (n / 4).max(1);
    let first_avg = mses[..seg].iter().sum::<f64>() / seg as f64;
    let last_avg = mses[n - seg..].iter().sum::<f64>() / seg as f64;
    ConvergeMetrics {
        final_ssim: s,
        deficit: 1.0 - s,
        monotonic_mse: last_avg < first_avg,
        digest: sha256_frame(last),
    }
}

/// device vs host 同输入逐帧对拍:逐帧最大绝对差序列 + 全局 p100。
fn per_frame_maxdiff(a: &ConvergeRun, b: &ConvergeRun) -> (Vec<f64>, f64) {
    assert_eq!(a.outs.len(), b.outs.len());
    let mut per = Vec::new();
    let mut p100 = 0.0f64;
    for (fa, fb) in a.outs.iter().zip(b.outs.iter()) {
        let d = max_abs_diff(fa, fb);
        p100 = p100.max(d);
        per.push(d);
    }
    (per, p100)
}

// ---------------------------------------------------------------------------
// JSON 出报(手写,零新依赖)
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

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

struct Args {
    spv_resample: Option<String>,
    spv_resolve: Option<String>,
    tol: f64,
    band_deficit: [f64; 3],
    calibrate: Option<String>,
    bench: Option<String>,
    red_arm: Option<String>,
    host_only: bool,
    warmup: u32,
    frames: u32,
}

fn parse_args() -> Args {
    let mut a = Args {
        spv_resample: None,
        spv_resolve: None,
        tol: 0.0,
        band_deficit: [0.0; 3],
        calibrate: None,
        bench: None,
        red_arm: None,
        host_only: false,
        warmup: BENCH_WARMUP,
        frames: BENCH_TIMED,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            "--spv-resample" => a.spv_resample = it.next(),
            "--spv-resolve" => a.spv_resolve = it.next(),
            "--tol" => {
                a.tol = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --tol 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--tol 非 f64"))
            }
            "--band-deficit-50" => {
                a.band_deficit[0] = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --band-deficit-50 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--band-deficit-50 非 f64"))
            }
            "--band-deficit-67" => {
                a.band_deficit[1] = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --band-deficit-67 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--band-deficit-67 非 f64"))
            }
            "--band-deficit-100" => {
                a.band_deficit[2] = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --band-deficit-100 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--band-deficit-100 非 f64"))
            }
            "--calibrate" => a.calibrate = it.next(),
            "--bench" => a.bench = it.next(),
            "--red-arm" => a.red_arm = it.next(),
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
            other => fail(&format!("未知参数: {other}")),
        }
    }
    a
}

fn device_backend(args: &Args) -> Result<TsrDeviceBackend, String> {
    let spv_a = load_spv(
        args.spv_resample
            .as_deref()
            .unwrap_or_else(|| fail("缺 --spv-resample")),
    );
    let spv_b = load_spv(
        args.spv_resolve
            .as_deref()
            .unwrap_or_else(|| fail("缺 --spv-resolve")),
    );
    TsrDeviceBackend::create(spv_a, spv_b)
}

// ---------------------------------------------------------------------------
// 标定腿(deterministic 面:device vs host 容差 + 三档 deficit;两跑位级一致
// 由 CI 裁决。帧时面不走本腿——墙钟测量由 --bench 腿 + CI 统计面承载)
// ---------------------------------------------------------------------------

fn calibrate_leg(what: &str, args: &Args) -> ! {
    let reference = render_reference(OUT_W, OUT_H);
    let mut dev = match device_backend(args) {
        Ok(b) => b,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g13tsrdevice.calibration_skip.v1\",\"what\":{},\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(what),
                jstr(&e)
            );
            std::process::exit(0);
        }
    };
    match what {
        // device vs host 金标准同输入逐帧对拍容差:三档 × 32 帧逐帧逐像素
        // 最大绝对差 p100(统计面 = 全帧全集 p100;threshold = measured × 2.0)。
        "maxdiff" => {
            let mut p100 = 0.0f64;
            let mut cells = Vec::new();
            for (name, iw, ih) in TIERS {
                let mut host = TsrUpscaler::default();
                let host_run = run_static(&mut host, iw, ih, CONVERGE_FRAMES);
                dev.reset_history();
                let dev_run = run_static(&mut dev, iw, ih, CONVERGE_FRAMES);
                let (_per, cell_p100) = per_frame_maxdiff(&dev_run, &host_run);
                cells.push(format!("\"{}\":{:.15e}", name, cell_p100));
                p100 = p100.max(cell_p100);
            }
            let protocol = format!(
                "device vs host 金标准同输入逐帧对拍容差(三档 50% 640×360/67% 858×482/100% 1280×720 → 1280×720,各 {CONVERGE_FRAMES} 帧 Halton jitter 静态收敛序列,逐帧逐像素最大绝对差 p100;threshold = measured × 2.0 冻结 k,方向 max)"
            );
            println!(
                "{{\"schema\":\"rurix.g13tsrdevice.calibration_entry.v1\",\"entry_id\":\"g13.tsr_device.host_device_maxdiff_tol\",\"results\":{{\"trimmed_mean\":{:.15e}}},\"protocol\":{},\"sample_manifest\":{{\"count\":{},\"digest\":{}}},\"provenance\":{{\"gpu\":\"device\",\"backend\":\"tsr_device\",\"base_commit\":{}}},\"cells\":{{{}}},\"timestamp\":{}}}",
                p100,
                jstr(&protocol),
                TIERS.len() * CONVERGE_FRAMES as usize,
                jstr(&format!("sha256:{}", cells_digest(&cells))),
                jstr(&base_commit()),
                cells.join(","),
                jstr(&utc_now()),
            );
        }
        // 三档质量标定:终帧 SSIM deficit(对拍 4×4 超采样参照)per tier。
        "quality" => {
            let mut entries = Vec::new();
            for (name, iw, ih) in TIERS {
                dev.reset_history();
                let run = run_static(&mut dev, iw, ih, CONVERGE_FRAMES);
                let m = measure(&run, &reference);
                entries.push(format!("\"{}\":{:.15e}", name, m.deficit));
            }
            let protocol = format!(
                "三档质量 measured(device TSR 终帧 SSIM deficit 1−SSIM 对拍 4×4 超采样参照,RXS-0387 LDR 8×8 窗口径;50% 640×360/67% 858×482/100% 1280×720 → 1280×720 各 {CONVERGE_FRAMES} 帧 Halton jitter 静态收敛;threshold = measured × 2.0 冻结 k,方向 max;回归守护语义不构成画质通过线)"
            );
            println!(
                "{{\"schema\":\"rurix.g13tsrdevice.calibration_entry.v1\",\"entry_id\":\"g13.tsr_device.tier_ssim_deficit\",\"results\":{{\"trimmed_mean\":{:.15e}}},\"protocol\":{},\"sample_manifest\":{{\"count\":{},\"digest\":{}}},\"provenance\":{{\"gpu\":\"device\",\"backend\":\"tsr_device\",\"base_commit\":{}}},\"cells\":{{{}}},\"timestamp\":{}}}",
                entries
                    .iter()
                    .filter_map(|e| e.split(':').nth(1)?.parse::<f64>().ok())
                    .fold(0.0f64, f64::max),
                jstr(&protocol),
                TIERS.len(),
                jstr(&format!("sha256:{}", cells_digest(&entries))),
                jstr(&base_commit()),
                entries.join(","),
                jstr(&utc_now()),
            );
        }
        other => fail(&format!("未知标定面: {other}(maxdiff|quality)")),
    }
    std::process::exit(0)
}

fn cells_digest(cells: &[String]) -> String {
    rurix_pkg::sha256::hex_digest(cells.join(",").as_bytes())
}

fn utc_now() -> String {
    std::process::Command::new("git")
        .args(["log", "-1", "--format=%cI"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

// ---------------------------------------------------------------------------
// bench 腿(帧时采样面:warmup + timed 逐帧 host Instant 墙钟 around device
// 全链路;原始样本出报,统计面 = CI block_stats M141/M165 冻结口径)
// ---------------------------------------------------------------------------

fn bench_leg(tier: &str, args: &Args) -> ! {
    let Some((name, iw, ih)) = TIERS.iter().find(|t| t.0 == tier) else {
        fail(&format!("未知档位: {tier}(50|67|100)"))
    };
    let mut dev = match device_backend(args) {
        Ok(b) => b,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g13tsrdevice.bench_skip.v1\",\"tier\":{},\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(name),
                jstr(&e)
            );
            std::process::exit(0);
        }
    };
    let scale = OUT_W as f32 / *iw as f32;
    let depth = const_depth(*iw, *ih);
    let mv = zero_mv(*iw, *ih);
    let total = args.warmup + args.frames;
    let jitters = jitter_sequence(total);
    let mut warmup_ms: Vec<f64> = Vec::new();
    let mut frame_ms: Vec<f64> = Vec::new();
    let mut first_digest = String::new();
    let mut digests = std::collections::BTreeSet::new();
    for (i, &j) in jitters.iter().enumerate() {
        let cur = render_input(*iw, *ih, scale, j);
        let inp = inputs_for(&cur, &depth, &mv, (OUT_W, OUT_H), j, i as u32, i == 0);
        let t0 = std::time::Instant::now();
        let out = dev.upscale(&inp);
        let el = t0.elapsed().as_secs_f64() * 1000.0;
        if (i as u32) < args.warmup {
            warmup_ms.push(el);
        } else {
            frame_ms.push(el);
        }
        let d = sha256_frame(&out);
        if i == 0 {
            first_digest = d.clone();
        }
        digests.insert(d);
    }
    let samples: Vec<String> = frame_ms.iter().map(|v| format!("{v:.6}")).collect();
    let warms: Vec<String> = warmup_ms.iter().map(|v| format!("{v:.6}")).collect();
    println!(
        "{{\"schema\":\"rurix.g13tsrdevice.bench.v1\",\"tier\":{},\"in_size\":[{},{}],\"out_size\":[{},{}],\"warmup_count\":{},\"timed_count\":{},\"frame_ms\":[{}],\"warmup_ms\":[{}],\"first_frame_digest\":{},\"distinct_frame_digests\":{},\"timer\":\"host Instant 墙钟 around 逐帧 device 全链路(打包 + 双 dispatch + 回读同步 + 状态轮换)\",\"base_commit\":{}}}",
        jstr(name),
        iw,
        ih,
        OUT_W,
        OUT_H,
        args.warmup,
        args.frames,
        samples.join(","),
        warms.join(","),
        jstr(&first_digest),
        digests.len(),
        jstr(&base_commit()),
    );
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// RED 臂
// ---------------------------------------------------------------------------

fn red_arm_kernel_bias(args: &Args) -> Result<(), String> {
    // device kernel 输出面加性偏置 → device vs host 对拍必超容差(超容差静默
    // 即 RED 的机器兑现;tol 由 g13_budget 标定条目经 CI 传入)。
    let (name, iw, ih) = TIERS[0];
    let mut host = TsrUpscaler::default();
    let host_run = run_static(&mut host, iw, ih, CONVERGE_FRAMES);
    let mut honest = device_backend(args)?;
    let honest_run = run_static(&mut honest, iw, ih, CONVERGE_FRAMES);
    let (_hp, honest_p100) = per_frame_maxdiff(&honest_run, &host_run);
    let mut tampered = device_backend(args)?;
    tampered.red_bias = RED_BIAS;
    let tampered_run = run_static(&mut tampered, iw, ih, CONVERGE_FRAMES);
    let (_tp, tampered_p100) = per_frame_maxdiff(&tampered_run, &host_run);
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
    eprintln!(
        "{TAG}: red-arm kernel-bias 检出 — tier {name} honest p100={honest_p100:.6e} tampered p100={tampered_p100:.6e} tol={:.6e}",
        args.tol
    );
    Ok(())
}

fn red_arm_seed_change(args: &Args) -> Result<(), String> {
    // jitter 序列相位偏移(改 seed 等价面)→ 终帧 digest 必异(确定性协议漂移
    // 检出面;digest 比对机制必须能分辨输入流改动)。
    let (_name, iw, ih) = TIERS[0];
    let mut honest = device_backend(args)?;
    let scale = OUT_W as f32 / iw as f32;
    let depth = const_depth(iw, ih);
    let mv = zero_mv(iw, ih);
    let jitters = jitter_sequence(CONVERGE_FRAMES);
    let mut outs_a = Vec::new();
    for (i, &j) in jitters.iter().enumerate() {
        let cur = render_input(iw, ih, scale, j);
        let inp = inputs_for(&cur, &depth, &mv, (OUT_W, OUT_H), j, i as u32, i == 0);
        outs_a.push(honest.upscale(&inp));
    }
    let digest_a = sha256_frame(outs_a.last().expect("至少一帧"));
    let mut tampered = device_backend(args)?;
    let mut outs_b = Vec::new();
    for (i, &j) in jitters.iter().enumerate() {
        let j2 = [j[0] + 0.13, j[1] + 0.29];
        let cur = render_input(iw, ih, scale, j2);
        let inp = inputs_for(&cur, &depth, &mv, (OUT_W, OUT_H), j2, i as u32, i == 0);
        outs_b.push(tampered.upscale(&inp));
    }
    let digest_b = sha256_frame(outs_b.last().expect("至少一帧"));
    if digest_a == digest_b {
        return Err("seed-change 漏检:jitter 相位偏移后终帧 digest 未变".into());
    }
    eprintln!("{TAG}: red-arm seed-change 检出 — digest 偏移可分辨");
    Ok(())
}

// ---------------------------------------------------------------------------
// host 腿(金标准锚;恒跑)
// ---------------------------------------------------------------------------

fn host_leg() -> (f64, bool) {
    let reference = render_reference(OUT_W, OUT_H);
    let mut tsr = TsrUpscaler::default();
    let run = run_static(&mut tsr, TIERS[0].1, TIERS[0].2, CONVERGE_FRAMES);
    let m = measure(&run, &reference);
    eprintln!(
        "{TAG}: host tsr(tier50) ssim={:.4} deficit={:.6} monotonic={}",
        m.final_ssim, m.deficit, m.monotonic_mse
    );
    (m.deficit, m.monotonic_mse)
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    if let Some(what) = &args.calibrate {
        calibrate_leg(what, &args);
    }
    if let Some(tier) = &args.bench {
        bench_leg(tier, &args);
    }
    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "kernel-bias" => red_arm_kernel_bias(&args),
            "seed-change" => red_arm_seed_change(&args),
            other => fail(&format!("未知 RED 臂: {other}(kernel-bias|seed-change)")),
        };
        match r {
            Ok(()) => {
                println!(
                    "{{\"schema\":\"rurix.g13tsrdevice.red_arm.v1\",\"arm\":{},\"detected\":true}}",
                    jstr(arm)
                );
                std::process::exit(0);
            }
            Err(e) if e.contains("不可用") || e.contains("DEV_ENV") => {
                println!(
                    "{{\"schema\":\"rurix.g13tsrdevice.red_arm.v1\",\"arm\":{},\"detected\":false,\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                    jstr(arm),
                    jstr(&e)
                );
                std::process::exit(0);
            }
            Err(e) => fail(&format!("red-arm {arm} 失效(漏检): {e}")),
        }
    }

    // host 腿恒跑(TSR 金标准锚 + 收敛单调性)。
    let (host_deficit, host_monotonic) = host_leg();

    if args.host_only {
        println!(
            "{{\"schema\":\"rurix.g13tsrdevice.harness.v1\",\"mode\":\"host-only\",\"state\":\"pass\",\"host\":{{\"tsr_deficit\":{:.10},\"tsr_monotonic\":{}}}}}",
            host_deficit, host_monotonic
        );
        return;
    }

    let mut dev = match device_backend(&args) {
        Ok(b) => b,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g13tsrdevice.harness.v1\",\"mode\":\"device\",\"state\":\"skipped_dev_env\",\"skip_reason\":{},\"host\":{{\"tsr_deficit\":{:.10},\"tsr_monotonic\":{}}}}}",
                jstr(&e),
                host_deficit,
                host_monotonic
            );
            return;
        }
    };

    // ── device 全档:三档 × 32 帧收敛 + device vs host 逐帧对拍 + 双跑位级 ──
    let reference = render_reference(OUT_W, OUT_H);
    let mut problems: Vec<String> = Vec::new();
    let mut tier_json: Vec<String> = Vec::new();
    for (idx, (name, iw, ih)) in TIERS.iter().enumerate() {
        let mut host = TsrUpscaler::default();
        let host_run = run_static(&mut host, *iw, *ih, CONVERGE_FRAMES);
        dev.reset_history();
        let dev_run = run_static(&mut dev, *iw, *ih, CONVERGE_FRAMES);
        let m = measure(&dev_run, &reference);
        let (_per, p100) = per_frame_maxdiff(&dev_run, &host_run);
        // 双跑位级一致(固定 scene/jitter/参数 → 确定性协议面)。
        dev.reset_history();
        let dev_run_b = run_static(&mut dev, *iw, *ih, CONVERGE_FRAMES);
        let mb = measure(&dev_run_b, &reference);
        let bitexact = m.digest == mb.digest;
        let in_tol = args.tol <= 0.0 || p100 <= args.tol;
        let in_band = args.band_deficit[idx] <= 0.0 || m.deficit <= args.band_deficit[idx];
        if !m.monotonic_mse {
            problems.push(format!("tier{name} 收敛 MSE 非单调下降"));
        }
        if !bitexact {
            problems.push(format!("tier{name} 双跑非位级一致"));
        }
        if !in_tol {
            problems.push(format!(
                "tier{name} device vs host p100={p100:.6e} 超容差 {:.6e}",
                args.tol
            ));
        }
        if !in_band {
            problems.push(format!(
                "tier{name} deficit {:.6} 超标定带 {:.6}",
                m.deficit, args.band_deficit[idx]
            ));
        }
        tier_json.push(format!(
            "{}:{{\"in_size\":[{},{}],\"out_size\":[{},{}],\"final_ssim\":{:.10},\"deficit\":{:.10},\"monotonic\":{},\"digest\":{},\"bitexact\":{},\"host_device_maxdiff_p100\":{:.15e},\"in_tol\":{},\"in_band\":{}}}",
            jstr(name),
            iw,
            ih,
            OUT_W,
            OUT_H,
            m.final_ssim,
            m.deficit,
            m.monotonic_mse,
            jstr(&m.digest),
            bitexact,
            p100,
            in_tol,
            in_band,
        ));
        eprintln!(
            "{TAG}: tier {name} {iw}×{ih}→{OUT_W}×{OUT_H} ssim={:.4} deficit={:.6} p100_vs_host={:.6e} bitexact={bitexact}",
            m.final_ssim, m.deficit, p100
        );
    }
    if !host_monotonic {
        problems.push("host 金标准收敛非单调(金标准面异常)".into());
    }
    let state = if problems.is_empty() { "pass" } else { "fail" };
    println!(
        "{{\"schema\":\"rurix.g13tsrdevice.harness.v1\",\"mode\":\"device\",\"state\":{},\"problems\":{},\"host\":{{\"tsr_deficit\":{:.10},\"tsr_monotonic\":{}}},\"tiers\":{{{}}},\"tol\":{:.15e}}}",
        jstr(state),
        strs_json(&problems),
        host_deficit,
        host_monotonic,
        tier_json.join(","),
        args.tol,
    );
    if !problems.is_empty() {
        std::process::exit(1);
    }
}
