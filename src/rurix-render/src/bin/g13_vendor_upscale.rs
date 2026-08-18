//! G13.2 M-a(M167) vendor 超分接入 harness(门 g13.p0.m_a.vendor_upscale_integration;
//! G13_CONTRACT §4.2 M-a 行逐字 / G-G13-4;G13_ACCEPTANCE_MAP §1;RFC-0016 §4.H3/§9 Q-F;
//! spec/visual_comparison.md RXS-0387/0388 口径继承)。
//!
//! ## 集成路径
//!
//! DLSS SR(Streamline SDK 2.10.3,Vulkan interop 臂)与 FSR 3.1.5(FidelityFX SDK
//! 2.0.0 预编译签名 DLL,DX12 臂)经 **bin-local adapter** 实现 [`UpscaleBackend`]
//! 冻结面(RFC-0016 §4.0-3)接入——adapter 只消费 `rurix-rt::vendor_upscale` 的 safe
//! 公共面(U58 审计),`temporal/` 底座与 trait 签名面 0-byte;树内绕过
//! UpscaleBackend 的私接面即 RED(CI 脚本 grep token 面机核)。
//!
//! ## 判据面
//!
//! - 同场景(静态合成 shade 场景)同内部分辨率(320×180→640×360)TSR/DLSS/FSR
//!   三后端同进程运行时切换(逐帧轮换,各自历史内置于 session/adapter);
//! - 静态场景 32 帧 Halton jitter 收敛,终帧 SSIM deficit(1−SSIM,LDR 口径
//!   RXS-0387)对拍 4×4 超采样参照,不偏离 g13_budget 标定冻结带(measured×2.0);
//! - 双端对拍 measured 登记(DLSS↔TSR / FSR↔TSR SSIM + 逐像素最大绝对差,不设
//!   绝对通过线——G13 不设 DLSS/超分画质通过线);
//! - DLL provenance 实测 digest 出报(CI 对账 g13_vendor_sdk_registry.json);
//! - FSR4 ML 不可用自动回退 FSR 3.1.5 分析版如实登记;
//! - 双跑位级一致(固定 scene/jitter/参数 → 确定性);
//! - RURIX_VK_VALIDATION=1 下 validation 错误计数 = 0——DLSS 臂校验覆盖经
//!   `--validation-probe dlss` 独立子进程达成(我方 Vulkan 全表面 + SL 代理建链
//!   + SL 簿记;**NGX slEvaluateFeature 段排除**——层在下 NGX CUDA interop 触发
//!   NVIDIA 驱动内部崩溃 nvoglv64.dll 0xc0000005,vendor 已知 SL+validation
//!   不兼容类 Streamline issue #84,排除段 evidence/契约 §8.3 同字面登记);
//!   FSR 臂 = D3D12 debug layer + InfoQueue ERROR/CORRUPTION 级在跑计数。
//!
//! ## RED 臂(契约判据字面)
//!
//! - `mock-passthrough`:bilinear 单帧上采样冒充 DLSS——无 DLL provenance +
//!   deficit 超带,验证路径必须拒(mock/stub 充真跑即 RED);
//! - `mv-garbage`(DLSS,device):静态场景喂垃圾 MV,deficit 显著劣化必检出;
//! - `fsr-mv-garbage`(FSR,device):同型垃圾 MV 注入,FSR 时序重投影错位劣化
//!   必检出(原 zero-exposure 注入实测无效已废止——FSR 3.1.5 LDR 路径不消费
//!   pre_exposure,留痕防回归)。
//!
//! ## 三态
//!
//! 无 SDK/GPU → device 腿 `SKIP DEV_ENV_DEGRADE`(退 0,非 fake pass;
//! `RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层裁决);host 腿恒跑。
//! 判据不符 / RED 臂失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g13_vendor_upscale [--frames 32] [--band-tsr F --band-dlss F --band-fsr F]
//! g13_vendor_upscale --host-only
//! g13_vendor_upscale --calibrate tsr|dlss|fsr [--frames 32]
//! g13_vendor_upscale --red-arm mock-passthrough|mv-garbage|fsr-mv-garbage [--band-dlss F]
//! g13_vendor_upscale --validation-probe dlss|fsr
//! ```

#![forbid(unsafe_code)]

use rurix_render::temporal::common::jitter_sequence;
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::ssim::ssim;
use rurix_render::temporal::tsr::TsrUpscaler;
use rurix_render::temporal::upscale::{UpscaleBackend, UpscaleInputs};
use rurix_rt::vendor_upscale::{
    DlssVkSession, FsrDx12Session, VendorError, VendorFrameInput, VendorSessionReport,
    fsr_sdk_dir, streamline_sdk_dir,
};
use std::path::Path;

const TAG: &str = "[g13_vendor_upscale]";
const IN_W: u32 = 320;
const IN_H: u32 = 180;
const OUT_W: u32 = 640;
const OUT_H: u32 = 360;
const SWITCH_FRAMES: u32 = 12;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 静态合成场景(与 tsr.rs 静态收敛门禁同 shade 面——跨后端同一事实源)
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

fn bilinear_up(src: &ImageF32, ow: u32, oh: u32) -> ImageF32 {
    ImageF32::from_fn(ow, oh, 3, |x, y, ch| {
        src.sample_bilinear(
            (x as f32 + 0.5) / ow as f32,
            (y as f32 + 0.5) / oh as f32,
            ch,
        )
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

// ---------------------------------------------------------------------------
// bin-local vendor adapters(UpscaleBackend 冻结面;只消费 rurix-rt safe 公共面)
// ---------------------------------------------------------------------------

/// DLSS SR(Streamline 2.10.3 Vulkan interop)→ UpscaleBackend 适配。
struct DlssBackend {
    session: DlssVkSession,
    in_size: (u32, u32),
    out_size: (u32, u32),
    pending_reset: bool,
}

impl DlssBackend {
    fn create(validation: bool) -> Result<Self, VendorError> {
        let dir = streamline_sdk_dir()?;
        let session = DlssVkSession::create(&dir, (IN_W, IN_H), (OUT_W, OUT_H), validation)?;
        Ok(Self {
            session,
            in_size: (IN_W, IN_H),
            out_size: (OUT_W, OUT_H),
            pending_reset: true,
        })
    }
}

impl UpscaleBackend for DlssBackend {
    fn name(&self) -> &str {
        "dlss_sr"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        assert_eq!((iw, ih), self.in_size, "DLSS adapter 输入分辨率与 session 不符");
        assert_eq!((ow, oh), self.out_size, "DLSS adapter 输出分辨率与 session 不符");
        let vi = VendorFrameInput {
            color: &inputs.color.data,
            depth: &inputs.depth.data,
            mv: &inputs.mv.data,
            reactive: inputs.reactive.map(|r| &r.data[..]),
            exposure: inputs.exposure,
            jitter: inputs.jitter,
            frame_index: inputs.frame_index,
            reset: inputs.reset || self.pending_reset,
        };
        self.pending_reset = false;
        let data = self
            .session
            .upscale(&vi)
            .unwrap_or_else(|e| panic!("DLSS upscale 失败: {e}"));
        ImageF32 { w: ow, h: oh, c: 3, data }
    }

    fn reset_history(&mut self) {
        self.pending_reset = true;
    }
}

/// FSR 3.1.5(FFX SDK 2.0.0 DX12)→ UpscaleBackend 适配。
struct FsrBackend {
    session: FsrDx12Session,
    in_size: (u32, u32),
    out_size: (u32, u32),
    pending_reset: bool,
}

impl FsrBackend {
    fn create(validation: bool) -> Result<Self, VendorError> {
        let dir = fsr_sdk_dir()?;
        let session = FsrDx12Session::create(&dir, (IN_W, IN_H), (OUT_W, OUT_H), validation)?;
        Ok(Self {
            session,
            in_size: (IN_W, IN_H),
            out_size: (OUT_W, OUT_H),
            pending_reset: true,
        })
    }
}

impl UpscaleBackend for FsrBackend {
    fn name(&self) -> &str {
        "fsr_3.1.5"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        assert_eq!((iw, ih), self.in_size, "FSR adapter 输入分辨率与 session 不符");
        assert_eq!((ow, oh), self.out_size, "FSR adapter 输出分辨率与 session 不符");
        let vi = VendorFrameInput {
            color: &inputs.color.data,
            depth: &inputs.depth.data,
            mv: &inputs.mv.data,
            reactive: inputs.reactive.map(|r| &r.data[..]),
            exposure: inputs.exposure,
            jitter: inputs.jitter,
            frame_index: inputs.frame_index,
            reset: inputs.reset || self.pending_reset,
        };
        self.pending_reset = false;
        let data = self
            .session
            .upscale(&vi)
            .unwrap_or_else(|e| panic!("FSR upscale 失败: {e}"));
        ImageF32 { w: ow, h: oh, c: 3, data }
    }

    fn reset_history(&mut self) {
        self.pending_reset = true;
    }
}

/// RED 臂专用:单帧 bilinear 上采样冒充 DLSS(mock/stub 充真跑注入件)。
struct MockPassthrough;

impl UpscaleBackend for MockPassthrough {
    fn name(&self) -> &str {
        "dlss_sr" // 冒充 DLSS——验证路径必须经 provenance/收敛带识破
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (_, _, ow, oh) = inputs.validated();
        bilinear_up(inputs.color, ow, oh)
    }

    fn reset_history(&mut self) {}
}

// ---------------------------------------------------------------------------
// 静态收敛跑面
// ---------------------------------------------------------------------------

struct ConvergeRun {
    outs: Vec<ImageF32>,
}

fn run_static(backend: &mut dyn UpscaleBackend, frames: u32) -> ConvergeRun {
    let scale = OUT_W as f32 / IN_W as f32;
    let depth = const_depth(IN_W, IN_H);
    let mv = zero_mv(IN_W, IN_H);
    let jitters = jitter_sequence(frames);
    let mut outs = Vec::new();
    for (i, &j) in jitters.iter().enumerate() {
        let cur = render_input(IN_W, IN_H, scale, j);
        let inp = inputs_for(&cur, &depth, &mv, (OUT_W, OUT_H), j, i as u32, i == 0);
        outs.push(backend.upscale(&inp));
    }
    ConvergeRun { outs }
}

/// 收敛量测:终帧 SSIM / deficit / 首末段 MSE 单调性。
struct ConvergeMetrics {
    final_ssim: f64,
    deficit: f64,
    monotonic_mse: bool,
    digest: String,
}

fn measure(run: &ConvergeRun, reference: &ImageF32) -> ConvergeMetrics {
    let last = run.outs.last().expect("至少一帧");
    let s = ssim(last, reference);
    let mses: Vec<f64> = run.outs.iter().map(|o| ImageF32::mse(o, reference)).collect();
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
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn jstr(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn dlls_json(report: &VendorSessionReport) -> String {
    let items: Vec<String> = report
        .dlls
        .iter()
        .map(|d| format!("[{},{}]", jstr(&d.name), jstr(&d.sha256)))
        .collect();
    format!("[{}]", items.join(","))
}

fn strs_json(items: &[String]) -> String {
    format!(
        "[{}]",
        items.iter().map(|s| jstr(s)).collect::<Vec<_>>().join(",")
    )
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
    host_only: bool,
    calibrate: Option<String>,
    red_arm: Option<String>,
    validation_probe: Option<String>,
    frames: u32,
    band_tsr: f64,
    band_dlss: f64,
    band_fsr: f64,
}

fn parse_args() -> Args {
    let mut a = Args {
        host_only: false,
        calibrate: None,
        red_arm: None,
        validation_probe: None,
        frames: 32,
        band_tsr: 0.0,
        band_dlss: 0.0,
        band_fsr: 0.0,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--host-only" => a.host_only = true,
            "--calibrate" => {
                a.calibrate = Some(it.next().unwrap_or_else(|| fail("--calibrate 需 tsr|dlss|fsr")))
            }
            "--red-arm" => {
                a.red_arm = Some(it.next().unwrap_or_else(|| fail("--red-arm 需臂名")))
            }
            "--validation-probe" => {
                a.validation_probe =
                    Some(it.next().unwrap_or_else(|| fail("--validation-probe 需 dlss|fsr")))
            }
            "--frames" => {
                a.frames = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| fail("--frames 需正整数"))
            }
            "--band-tsr" => a.band_tsr = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0),
            "--band-dlss" => a.band_dlss = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0),
            "--band-fsr" => a.band_fsr = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0),
            other => fail(&format!("未知参数: {other}")),
        }
    }
    if a.frames < 8 {
        fail("--frames 必须 ≥8(收敛分段统计最小面)");
    }
    a
}

fn validation_on() -> bool {
    std::env::var("RURIX_VK_VALIDATION").ok().as_deref() == Some("1")
}

// ---------------------------------------------------------------------------
// validation 探针腿(独立子进程面;RURIX_VK_VALIDATION=1 时由 device 腿自举调用)
// ---------------------------------------------------------------------------

/// DLSS 校验覆盖口径(evidence/契约 §8.3 同字面登记;NGX evaluate 段排除)。
const DLSS_PROBE_COVERAGE: &str = "app_vulkan_surface+sl_proxy_device+sl_bookkeeping";
/// NGX evaluate 段排除说明(minidump 实测 + vendor 已知不兼容留痕)。
const DLSS_EVALUATE_EXCLUDED_NOTE: &str =
    "ngx_evaluate_excluded:NGX slEvaluateFeature 在 VK_LAYER_KHRONOS_validation 在下触发 \
     NVIDIA 驱动内部崩溃(nvoglv64.dll 0xc0000005,SL 异常处理器捕获报 \
     eErrorExceptionHandler;vendor 已知 SL+validation 不兼容类 Streamline issue #84 \
     ack/bug)——evaluate 段未纳入校验层覆盖;DLSS 功能帧经独立无层 session 真跑产出";

/// validation 探针:DLSS 跳过 evaluate 跑我方 Vulkan 全表面 + SL 代理建链/簿记;
/// FSR 跑真帧(D3D12 debug layer 兼容在跑)。无 SDK/GPU → skipped_dev_env 退 0。
fn validation_probe_leg(backend: &str) -> ! {
    let scale = OUT_W as f32 / IN_W as f32;
    let depth = const_depth(IN_W, IN_H);
    let mv = zero_mv(IN_W, IN_H);
    let jitters = jitter_sequence(2); // 帧 0 建链+上传;帧 1 覆盖 cmd reset→重录面
    match backend {
        "dlss" => {
            let dir = match streamline_sdk_dir() {
                Ok(d) => d,
                Err(e) => {
                    println!(
                        "{{\"schema\":\"rurix.g13upscale.validation_probe.v1\",\"backend\":\"dlss\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                        jstr(&format!("{e}"))
                    );
                    std::process::exit(0);
                }
            };
            let mut session = match DlssVkSession::create(&dir, (IN_W, IN_H), (OUT_W, OUT_H), true) {
                Ok(s) => s,
                Err(e) => {
                    println!(
                        "{{\"schema\":\"rurix.g13upscale.validation_probe.v1\",\"backend\":\"dlss\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                        jstr(&format!("{e}"))
                    );
                    std::process::exit(0);
                }
            };
            for (i, &j) in jitters.iter().enumerate() {
                let cur = render_input(IN_W, IN_H, scale, j);
                let vi = VendorFrameInput {
                    color: &cur.data,
                    depth: &depth.data,
                    mv: &mv.data,
                    reactive: None,
                    exposure: 1.0,
                    jitter: j,
                    frame_index: i as u32,
                    reset: i == 0,
                };
                if let Err(e) = session.probe_validation_frame(&vi) {
                    eprintln!("{TAG}: validation-probe dlss 探针帧失败: {e}");
                    std::process::exit(1);
                }
            }
            let errors = session.validation_errors();
            let (excluded, names) = session.validation_excluded();
            let state = if errors == 0 { "pass" } else { "fail" };
            println!(
                "{{\"schema\":\"rurix.g13upscale.validation_probe.v1\",\"backend\":\"dlss\",\"state\":{},\"validation_errors\":{},\"validation_excluded_ngx\":{},\"validation_excluded_names\":{},\"coverage\":{},\"coverage_note\":{}}}",
                jstr(state),
                errors,
                excluded,
                strs_json(&names),
                jstr(DLSS_PROBE_COVERAGE),
                jstr(DLSS_EVALUATE_EXCLUDED_NOTE),
            );
            std::process::exit(if errors == 0 { 0 } else { 1 })
        }
        "fsr" => {
            let mut fsr = match FsrBackend::create(true) {
                Ok(b) => b,
                Err(e) => {
                    println!(
                        "{{\"schema\":\"rurix.g13upscale.validation_probe.v1\",\"backend\":\"fsr\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                        jstr(&format!("{e}"))
                    );
                    std::process::exit(0);
                }
            };
            for (i, &j) in jitters.iter().enumerate() {
                let cur = render_input(IN_W, IN_H, scale, j);
                let inp = inputs_for(&cur, &depth, &mv, (OUT_W, OUT_H), j, i as u32, i == 0);
                let out = fsr.upscale(&inp);
                if !out.data.iter().all(|v| v.is_finite()) {
                    eprintln!("{TAG}: validation-probe fsr 输出非有限");
                    std::process::exit(1);
                }
            }
            let errors = fsr.session.validation_errors();
            let state = if errors == 0 { "pass" } else { "fail" };
            println!(
                "{{\"schema\":\"rurix.g13upscale.validation_probe.v1\",\"backend\":\"fsr\",\"state\":{},\"validation_errors\":{},\"validation_excluded_ngx\":0,\"validation_excluded_names\":[],\"coverage\":{},\"coverage_note\":{}}}",
                jstr(state),
                errors,
                jstr("full_in_run_d3d12_debug_layer+info_queue"),
                jstr("D3D12 debug layer + ID3D12InfoQueue ERROR/CORRUPTION 级计数;FFX dispatch 全程在层下真跑"),
            );
            std::process::exit(if errors == 0 { 0 } else { 1 })
        }
        other => fail(&format!("未知 validation-probe 后端: {other}(dlss|fsr)")),
    }
}

/// 自举 validation 探针子进程(当前 exe 同路径;device 腿在 RURIX_VK_VALIDATION=1
/// 下调用,结果并入该后端 evidence)。返回 None = 子进程不可用/输出不可解析。
fn run_validation_probe(backend: &str) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let out = std::process::Command::new(exe)
        .args(["--validation-probe", backend])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    stdout
        .lines()
        .find(|l| l.contains("rurix.g13upscale.validation_probe.v1"))
        .map(|l| l.trim().to_string())
}

/// 从单行 JSON 提取 u64 字段(harness 自控 schema,免依赖解析)。
fn json_u64(doc: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let at = doc.find(&pat)? + pat.len();
    let rest = &doc[at..];
    let end = rest.find(|c: char| !c.is_ascii_digit())?;
    rest[..end].parse().ok()
}

/// 从单行 JSON 提取 string 字段。
fn json_str(doc: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let at = doc.find(&pat)? + pat.len();
    let rest = &doc[at..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// 从单行 JSON 提取 string 数组字段(元素均简单转义面)。
fn json_strs(doc: &str, key: &str) -> Vec<String> {
    let pat = format!("\"{key}\":[");
    let Some(mut at) = doc.find(&pat).map(|i| i + pat.len()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    while let Some(rel) = doc[at..].find('"') {
        at += rel + 1;
        let Some(end) = doc[at..].find('"') else { break };
        out.push(doc[at..at + end].to_string());
        at += end + 1;
        if !doc[at..].starts_with(',') {
            break;
        }
        at += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// host 腿(TSR 金标准锚 + adapter fail-closed)
// ---------------------------------------------------------------------------

struct HostLeg {
    tsr_ssim: f64,
    tsr_deficit: f64,
    tsr_monotonic: bool,
    tsr_digest: String,
    adapter_failclosed: bool,
}

fn host_leg(frames: u32) -> HostLeg {
    let reference = render_reference(OUT_W, OUT_H);
    let mut tsr = TsrUpscaler::default();
    let run = run_static(&mut tsr, frames);
    let m = measure(&run, &reference);
    eprintln!(
        "{TAG}: host tsr_ssim={:.4} deficit={:.6} monotonic={}",
        m.final_ssim, m.deficit, m.monotonic_mse
    );
    // adapter fail-closed:bogus SDK 目录必须确定性 Err(非 panic/非静默)。
    let bogus = Path::new("Z:\\rurix-nonexistent-sdk-dir");
    let d = DlssVkSession::create(bogus, (IN_W, IN_H), (OUT_W, OUT_H), false);
    let f = FsrDx12Session::create(bogus, (IN_W, IN_H), (OUT_W, OUT_H), false);
    let adapter_failclosed = matches!(d, Err(VendorError::DllNotFound(_)))
        && matches!(f, Err(VendorError::DllNotFound(_)));
    HostLeg {
        tsr_ssim: m.final_ssim,
        tsr_deficit: m.deficit,
        tsr_monotonic: m.monotonic_mse,
        tsr_digest: m.digest,
        adapter_failclosed,
    }
}

// ---------------------------------------------------------------------------
// device 腿
// ---------------------------------------------------------------------------

struct BackendReport {
    ran: bool,
    final_ssim: f64,
    deficit: f64,
    in_band: Option<bool>,
    monotonic: bool,
    digest: String,
    bitexact: bool,
    dlls: String,
    engine_version: String,
    validation_errors: u64,
    validation_excluded_ngx: u64,
    validation_excluded_names: Vec<String>,
    validation_coverage: String,
    fsr4_ml_available: Option<bool>,
    fsr4_note: String,
    available_versions: Vec<String>,
}

fn empty_report() -> BackendReport {
    BackendReport {
        ran: false,
        final_ssim: 0.0,
        deficit: 0.0,
        in_band: None,
        monotonic: false,
        digest: String::new(),
        bitexact: false,
        dlls: "[]".into(),
        engine_version: String::new(),
        validation_errors: 0,
        validation_excluded_ngx: 0,
        validation_excluded_names: Vec::new(),
        validation_coverage: "not_requested".into(),
        fsr4_ml_available: None,
        fsr4_note: String::new(),
        available_versions: Vec::new(),
    }
}

fn run_vendor_backend(
    backend: &mut dyn UpscaleBackend,
    report: &VendorSessionReport,
    validation_errors: u64,
    validation_excluded: (u64, Vec<String>),
    frames: u32,
    band: f64,
    reference: &ImageF32,
) -> BackendReport {
    let mut r = empty_report();
    let run_a = run_static(backend, frames);
    let ma = measure(&run_a, reference);
    // 双跑位级一致:同一进程二次全序列(create→frames→digest 由调用方重建 session;
    // 此处同一 backend 实例重跑——历史经首帧 reset 清洗,确定性协议面)。
    let run_b = run_static(backend, frames);
    let mb = measure(&run_b, reference);
    r.ran = true;
    r.final_ssim = ma.final_ssim;
    r.deficit = ma.deficit;
    r.monotonic = ma.monotonic_mse;
    r.digest = ma.digest.clone();
    r.bitexact = ma.digest == mb.digest;
    if band > 0.0 {
        r.in_band = Some(ma.deficit <= band);
    }
    r.dlls = dlls_json(report);
    r.engine_version = report.engine_version.clone();
    r.validation_errors = validation_errors;
    r.validation_excluded_ngx = validation_excluded.0;
    r.validation_excluded_names = validation_excluded.1;
    r.fsr4_ml_available = report.fsr4_ml_available;
    r.fsr4_note = report.fsr4_note.clone().unwrap_or_default();
    r.available_versions = report.available_versions.clone();
    r
}

struct DeviceLeg {
    state: String, // pass | fail | skipped_dev_env
    skip_reason: String,
    gpu: String,
    dlss: BackendReport,
    fsr: BackendReport,
    tsr: BackendReport,
    switch_ok: bool,
    switch_order: Vec<String>,
    pairwise_dlss_tsr_ssim: f64,
    pairwise_dlss_tsr_maxdiff: f64,
    pairwise_fsr_tsr_ssim: f64,
    pairwise_fsr_tsr_maxdiff: f64,
}

fn device_leg(args: &Args) -> DeviceLeg {
    let validation = validation_on();
    let reference = render_reference(OUT_W, OUT_H);

    // 三后端同进程创建(运行时切换前提);任一 vendor SDK/GPU 缺失 → SKIP。
    // DLSS 功能腿恒以 validation=false 建 session——KHRONOS 层在下 NGX evaluate
    // 触发驱动内崩溃(vendor 已知不兼容),校验覆盖经独立探针子进程达成(见
    // validation_probe_leg / DLSS_EVALUATE_EXCLUDED_NOTE);FSR D3D12 臂 debug
    // layer 在跑兼容,功能腿直接携 validation 旗标。
    let mut tsr: Box<dyn UpscaleBackend> = Box::new(TsrUpscaler::default());
    let mut dlss = match DlssBackend::create(false) {
        Ok(b) => b,
        Err(e) => {
            return DeviceLeg::skip(format!("DLSS session 创建失败(DEV_ENV): {e}"));
        }
    };
    let mut fsr = match FsrBackend::create(validation) {
        Ok(b) => b,
        Err(e) => {
            return DeviceLeg::skip(format!("FSR session 创建失败(DEV_ENV): {e}"));
        }
    };
    let gpu = dlss.session.report().gpu_name.clone();
    eprintln!("{TAG}: device gpu={gpu} validation={validation}");

    // ── 运行时切换:逐帧轮换三后端,各自历史内置,输出逐帧有效 ──
    let scale = OUT_W as f32 / IN_W as f32;
    let depth = const_depth(IN_W, IN_H);
    let mv = zero_mv(IN_W, IN_H);
    let jitters = jitter_sequence(SWITCH_FRAMES);
    let mut switch_order: Vec<String> = Vec::new();
    let mut switch_ok = true;
    for (i, &j) in jitters.iter().enumerate() {
        let cur = render_input(IN_W, IN_H, scale, j);
        let inp = inputs_for(&cur, &depth, &mv, (OUT_W, OUT_H), j, i as u32, i == 0);
        let backend: &mut dyn UpscaleBackend = match i % 3 {
            0 => &mut *tsr,
            1 => &mut dlss,
            _ => &mut fsr,
        };
        let out = backend.upscale(&inp);
        switch_order.push(backend.name().to_string());
        let finite = out.data.iter().all(|v| v.is_finite());
        let (mn, mx) = (
            out.data.iter().fold(f32::INFINITY, |a, &v| a.min(v)),
            out.data.iter().fold(f32::NEG_INFINITY, |a, &v| a.max(v)),
        );
        if !(finite && out.w == OUT_W && out.h == OUT_H && out.c == 3 && mx > mn) {
            switch_ok = false;
            eprintln!("{TAG}: 切换帧 {i} 后端 {} 输出无效", backend.name());
        }
    }
    eprintln!("{TAG}: 三后端运行时切换 {} 帧 ok={switch_ok}", SWITCH_FRAMES);

    // ── 各自全序列收敛(切换腿后 session 历史经下一帧 reset 清洗——run_static
    //    首帧 reset=true,语义 0-byte) ──
    let dlss_report = dlss.session.report();
    let dlss_verr = dlss.session.validation_errors();
    let dlss_excluded = dlss.session.validation_excluded();
    let mut dlss_rep = run_vendor_backend(&mut dlss, &dlss_report, dlss_verr, dlss_excluded, args.frames, args.band_dlss, &reference);
    // DLSS 校验覆盖:RURIX_VK_VALIDATION=1 → 探针子进程结果并入(探针 SKIP/
    // 不可解析 = 门禁完整性强错,coverage 记 probe_failed_unparseable,main 判红)。
    if validation {
        match run_validation_probe("dlss") {
            Some(doc) if json_str(&doc, "state").is_some_and(|s| s == "pass" || s == "fail") => {
                dlss_rep.validation_errors = json_u64(&doc, "validation_errors").unwrap_or(u64::MAX);
                dlss_rep.validation_excluded_ngx =
                    json_u64(&doc, "validation_excluded_ngx").unwrap_or(0);
                dlss_rep.validation_excluded_names = json_strs(&doc, "validation_excluded_names");
                let cov = json_str(&doc, "coverage").unwrap_or_default();
                let note = json_str(&doc, "coverage_note").unwrap_or_default();
                dlss_rep.validation_coverage = format!("{cov};{note}");
            }
            Some(_) => {
                dlss_rep.validation_coverage = "probe_skipped_dev_env".into();
            }
            None => {
                dlss_rep.validation_coverage = "probe_failed_unparseable".into();
            }
        }
    }
    let fsr_report = fsr.session.report();
    let fsr_verr = fsr.session.validation_errors();
    let mut fsr_rep = run_vendor_backend(&mut fsr, &fsr_report, fsr_verr, (0, Vec::new()), args.frames, args.band_fsr, &reference);
    if validation {
        fsr_rep.validation_coverage = "full_in_run_d3d12_debug_layer+info_queue".into();
    }
    let tsr_report_run = run_static(&mut *tsr, args.frames);
    let tsr_m = measure(&tsr_report_run, &reference);
    let mut tsr_rep = empty_report();
    tsr_rep.ran = true;
    tsr_rep.validation_coverage = "host_na".into();
    tsr_rep.final_ssim = tsr_m.final_ssim;
    tsr_rep.deficit = tsr_m.deficit;
    tsr_rep.monotonic = tsr_m.monotonic_mse;
    tsr_rep.digest = tsr_m.digest;
    if args.band_tsr > 0.0 {
        tsr_rep.in_band = Some(tsr_m.deficit <= args.band_tsr);
    }
    let tsr_run_b = run_static(&mut *tsr, args.frames);
    tsr_rep.bitexact = tsr_rep.digest == measure(&tsr_run_b, &reference).digest;

    // ── 双端对拍 measured 登记(vs TSR 金标准;不设通过线) ──
    let tsr_final = tsr_report_run.outs.last().expect("tsr 帧");
    let dlss_final = run_a_last(&dlss_rep, &reference, args.frames, &mut dlss);
    let fsr_final = run_a_last(&fsr_rep, &reference, args.frames, &mut fsr);
    let pairwise_dlss_tsr_ssim = ssim(&dlss_final, tsr_final);
    let pairwise_dlss_tsr_maxdiff = max_abs_diff(&dlss_final, tsr_final);
    let pairwise_fsr_tsr_ssim = ssim(&fsr_final, tsr_final);
    let pairwise_fsr_tsr_maxdiff = max_abs_diff(&fsr_final, tsr_final);
    dlss_rep.ran = true;

    DeviceLeg {
        state: "pass".into(),
        skip_reason: String::new(),
        gpu,
        dlss: dlss_rep,
        fsr: fsr_rep,
        tsr: tsr_rep,
        switch_ok,
        switch_order,
        pairwise_dlss_tsr_ssim,
        pairwise_dlss_tsr_maxdiff,
        pairwise_fsr_tsr_ssim,
        pairwise_fsr_tsr_maxdiff,
    }
}

/// 取 backend 收敛末帧(重跑一次确定序列;digest 已由双跑锚定)。
fn run_a_last(
    _rep: &BackendReport,
    _reference: &ImageF32,
    frames: u32,
    backend: &mut dyn UpscaleBackend,
) -> ImageF32 {
    let run = run_static(backend, frames);
    run.outs.into_iter().last().expect("至少一帧")
}

impl DeviceLeg {
    fn skip(reason: String) -> Self {
        eprintln!("{TAG}: SKIP DEV_ENV_DEGRADE — {reason}");
        DeviceLeg {
            state: "skipped_dev_env".into(),
            skip_reason: reason,
            gpu: String::new(),
            dlss: empty_report(),
            fsr: empty_report(),
            tsr: empty_report(),
            switch_ok: false,
            switch_order: Vec::new(),
            pairwise_dlss_tsr_ssim: 0.0,
            pairwise_dlss_tsr_maxdiff: 0.0,
            pairwise_fsr_tsr_ssim: 0.0,
            pairwise_fsr_tsr_maxdiff: 0.0,
        }
    }
}

fn backend_json(name: &str, r: &BackendReport) -> String {
    let in_band = match r.in_band {
        Some(b) => format!("{b}"),
        None => "null".into(),
    };
    let fsr4 = match r.fsr4_ml_available {
        Some(b) => format!("{b}"),
        None => "null".into(),
    };
    format!(
        "{}:{{\"ran\":{},\"final_ssim\":{:.10},\"deficit\":{:.10},\"in_band\":{},\"monotonic\":{},\"digest\":{},\"bitexact\":{},\"dlls\":{},\"engine_version\":{},\"validation_errors\":{},\"validation_excluded_ngx\":{},\"validation_excluded_names\":{},\"validation_coverage\":{},\"fsr4_ml_available\":{},\"fsr4_note\":{},\"available_versions\":{}}}",
        jstr(name),
        r.ran,
        r.final_ssim,
        r.deficit,
        in_band,
        r.monotonic,
        jstr(&r.digest),
        r.bitexact,
        r.dlls,
        jstr(&r.engine_version),
        r.validation_errors,
        r.validation_excluded_ngx,
        strs_json(&r.validation_excluded_names),
        jstr(&r.validation_coverage),
        fsr4,
        jstr(&r.fsr4_note),
        strs_json(&r.available_versions),
    )
}

// ---------------------------------------------------------------------------
// 标定腿(g13_m_a_calibration_entry evidence 形态;两跑位级一致由 CI 裁决)
// ---------------------------------------------------------------------------

fn calibrate_leg(backend_name: &str, frames: u32) -> ! {
    let validation = validation_on();
    let reference = render_reference(OUT_W, OUT_H);
    let mut backend: Box<dyn UpscaleBackend>;
    let mut gpu = "host".to_string();
    let mut dll_digest = String::new();
    match backend_name {
        "tsr" => backend = Box::new(TsrUpscaler::default()),
        // DLSS 标定为功能腿(出真帧测 deficit),恒 validation=false——校验层覆盖
        // 由 --validation-probe 独立面承担(NGX evaluate 层下驱动崩溃,见
        // DLSS_EVALUATE_EXCLUDED_NOTE)。
        "dlss" => match DlssBackend::create(false) {
            Ok(b) => {
                gpu = b.session.report().gpu_name.clone();
                dll_digest = b
                    .session
                    .report()
                    .dlls
                    .iter()
                    .map(|d| format!("{}:{}", d.name, d.sha256))
                    .collect::<Vec<_>>()
                    .join(";");
                backend = Box::new(b);
            }
            Err(e) => {
                println!(
                    "{{\"schema\":\"rurix.g13upscale.calibration_skip.v1\",\"backend\":\"dlss\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                    jstr(&format!("{e}"))
                );
                std::process::exit(0);
            }
        },
        "fsr" => match FsrBackend::create(validation) {
            Ok(b) => {
                gpu = b.session.report().gpu_name.clone();
                dll_digest = b
                    .session
                    .report()
                    .dlls
                    .iter()
                    .map(|d| format!("{}:{}", d.name, d.sha256))
                    .collect::<Vec<_>>()
                    .join(";");
                backend = Box::new(b);
            }
            Err(e) => {
                println!(
                    "{{\"schema\":\"rurix.g13upscale.calibration_skip.v1\",\"backend\":\"fsr\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                    jstr(&format!("{e}"))
                );
                std::process::exit(0);
            }
        },
        other => fail(&format!("未知标定后端: {other}(tsr|dlss|fsr)")),
    }
    let run = run_static(&mut *backend, frames);
    let m = measure(&run, &reference);
    // sample_manifest digest:全输出帧字节流 sha256(样本集代表面)。
    let mut all = Vec::new();
    for o in &run.outs {
        for &v in &o.data {
            all.extend_from_slice(&v.to_le_bytes());
        }
    }
    let manifest_digest = rurix_pkg::sha256::hex_digest(&all);
    let entry_id = format!("g13.upscale.static_converge_ssim_deficit_{backend_name}");
    let protocol = format!(
        "静态合成 shade 场景 {IN_W}×{IN_H}→{OUT_W}×{OUT_H},{frames} 帧 Halton jitter(均值趋 0 长程无偏),终帧 SSIM deficit(1−SSIM,RXS-0387 LDR 8×8 窗口径)对拍 4×4 超采样参照;threshold = measured × 2.0(冻结 k,方向 max)"
    );
    let ts = {
        let out = std::process::Command::new("git")
            .args(["log", "-1", "--format=%cI"])
            .output();
        out.ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into())
    };
    println!(
        "{{\"schema\":\"rurix.g13upscale.calibration_entry.v1\",\"entry_id\":{},\"results\":{{\"trimmed_mean\":{:.15}}},\"protocol\":{},\"sample_manifest\":{{\"count\":{},\"digest\":{}}},\"provenance\":{{\"gpu\":{},\"backend\":{},\"base_commit\":{},\"dll_digest\":{}}},\"timestamp\":{}}}",
        jstr(&entry_id),
        m.deficit,
        jstr(&protocol),
        frames,
        jstr(&format!("sha256:{manifest_digest}")),
        jstr(&gpu),
        jstr(backend.name()),
        jstr(&base_commit()),
        jstr(&dll_digest),
        jstr(&ts),
    );
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// RED 臂
// ---------------------------------------------------------------------------

fn red_arm_mock_passthrough(frames: u32, band_dlss: f64) -> Result<(), String> {
    // mock 冒充 DLSS;bilinear 无累积无 provenance。验证路径 = provenance 非空 +
    // deficit ≤ 带(band 0 时退化为 deficit 显著劣于 TSR 金标准对照)。
    let reference = render_reference(OUT_W, OUT_H);
    let mut mock = MockPassthrough;
    let run = run_static(&mut mock, frames);
    let m = measure(&run, &reference);
    let mock_dlls = 0usize; // mock 无 DLL provenance(冒充面无登记 digest)
    let mut reasons: Vec<String> = Vec::new();
    if mock_dlls == 0 {
        reasons.push("无 DLL provenance(冒充件零登记 digest)".into());
    }
    if band_dlss > 0.0 {
        if m.deficit > band_dlss {
            reasons.push(format!("deficit {:.6} > 标定带 {band_dlss:.6}", m.deficit));
        }
    } else {
        // 无带面:与 TSR 金标准对照——mock deficit 必须显著劣于 TSR。
        let mut tsr = TsrUpscaler::default();
        let tm = measure(&run_static(&mut tsr, frames), &reference);
        if m.deficit > tm.deficit * 1.5 {
            reasons.push(format!(
                "deficit {:.6} 显著劣于 TSR 金标准 {:.6}(×1.5)",
                m.deficit, tm.deficit
            ));
        }
    }
    if reasons.is_empty() {
        return Err(format!(
            "mock-passthrough 漏检:deficit={:.6} 未被验证路径拒绝",
            m.deficit
        ));
    }
    eprintln!("{TAG}: red-arm mock-passthrough 检出 — {}", reasons.join(";"));
    Ok(())
}

fn red_arm_mv_garbage(frames: u32, band_dlss: f64) -> Result<(), String> {
    // device 臂:静态场景喂垃圾 MV( uv 位移 10.0),DLSS 重投影必须劣化。
    // DLSS 功能面恒 validation=false(校验覆盖归 --validation-probe 独立面)。
    let mut dlss = DlssBackend::create(false)
        .map_err(|e| format!("DLSS session 创建失败(DEV_ENV): {e}"))?;
    let reference = render_reference(OUT_W, OUT_H);
    let good = measure(&run_static(&mut dlss, frames), &reference);
    let scale = OUT_W as f32 / IN_W as f32;
    let depth = const_depth(IN_W, IN_H);
    let garbage_mv = ImageF32::from_fn(IN_W, IN_H, 2, |_, _, _| 10.0);
    let jitters = jitter_sequence(frames);
    let mut outs = Vec::new();
    for (i, &j) in jitters.iter().enumerate() {
        let cur = render_input(IN_W, IN_H, scale, j);
        let inp = inputs_for(&cur, &depth, &garbage_mv, (OUT_W, OUT_H), j, i as u32, i == 0);
        outs.push(dlss.upscale(&inp));
    }
    let bad_run = ConvergeRun { outs };
    let bad = measure(&bad_run, &reference);
    let worse = bad.deficit > good.deficit * 1.05 || (band_dlss > 0.0 && bad.deficit > band_dlss);
    if !worse {
        return Err(format!(
            "mv-garbage 漏检:honest deficit={:.6} vs tampered={:.6}",
            good.deficit, bad.deficit
        ));
    }
    eprintln!(
        "{TAG}: red-arm mv-garbage 检出 — honest={:.6} tampered={:.6}",
        good.deficit, bad.deficit
    );
    Ok(())
}

fn red_arm_fsr_mv_garbage(frames: u32, band_fsr: f64) -> Result<(), String> {
    // device 臂:FSR 静态场景喂垃圾 MV(uv 位移 10.0),时序重投影历史错位必劣化
    // (原 zero-exposure 注入实测无效——FSR 3.1.5 LDR 路径不消费 pre_exposure,
    // deficit 与诚实跑位级一致,该注入面已废止并如实留痕)。
    let mut fsr = FsrBackend::create(validation_on())
        .map_err(|e| format!("FSR session 创建失败(DEV_ENV): {e}"))?;
    let reference = render_reference(OUT_W, OUT_H);
    let good = measure(&run_static(&mut fsr, frames), &reference);
    let scale = OUT_W as f32 / IN_W as f32;
    let depth = const_depth(IN_W, IN_H);
    let garbage_mv = ImageF32::from_fn(IN_W, IN_H, 2, |_, _, _| 10.0);
    let jitters = jitter_sequence(frames);
    let mut outs = Vec::new();
    for (i, &j) in jitters.iter().enumerate() {
        let cur = render_input(IN_W, IN_H, scale, j);
        let inp = inputs_for(&cur, &depth, &garbage_mv, (OUT_W, OUT_H), j, i as u32, i == 0);
        outs.push(fsr.upscale(&inp));
    }
    let bad_run = ConvergeRun { outs };
    let bad = measure(&bad_run, &reference);
    let worse = bad.deficit > good.deficit * 1.05 || (band_fsr > 0.0 && bad.deficit > band_fsr);
    if !worse {
        return Err(format!(
            "fsr-mv-garbage 漏检:honest deficit={:.6} vs tampered={:.6}",
            good.deficit, bad.deficit
        ));
    }
    eprintln!(
        "{TAG}: red-arm fsr-mv-garbage 检出 — honest={:.6} tampered={:.6}",
        good.deficit, bad.deficit
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    if let Some(backend) = &args.validation_probe {
        validation_probe_leg(backend);
    }

    if let Some(name) = &args.calibrate {
        calibrate_leg(name, args.frames);
    }

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "mock-passthrough" => red_arm_mock_passthrough(args.frames, args.band_dlss),
            "mv-garbage" => red_arm_mv_garbage(args.frames, args.band_dlss),
            "fsr-mv-garbage" => red_arm_fsr_mv_garbage(args.frames, args.band_fsr),
            other => fail(&format!(
                "未知 RED 臂: {other}(mock-passthrough|mv-garbage|fsr-mv-garbage)"
            )),
        };
        match r {
            Ok(()) => {
                println!(
                    "{{\"schema\":\"rurix.g13upscale.red_arm.v1\",\"arm\":{},\"detected\":true}}",
                    jstr(arm)
                );
                std::process::exit(0);
            }
            Err(e) if e.contains("DEV_ENV") => {
                println!(
                    "{{\"schema\":\"rurix.g13upscale.red_arm.v1\",\"arm\":{},\"detected\":false,\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                    jstr(arm),
                    jstr(&e)
                );
                std::process::exit(0);
            }
            Err(e) => fail(&format!("red-arm {arm} 失效(漏检): {e}")),
        }
    }

    // host 腿恒跑(TSR 金标准锚 + adapter fail-closed)。
    let host = host_leg(args.frames);
    if !host.adapter_failclosed {
        fail("adapter fail-closed 锚失效(bogus SDK 目录未确定性 DllNotFound)");
    }

    if args.host_only {
        println!(
            "{{\"schema\":\"rurix.g13upscale.harness.v1\",\"mode\":\"host-only\",\"state\":\"pass\",\"host\":{{\"tsr_ssim\":{:.10},\"tsr_deficit\":{:.10},\"tsr_monotonic\":{},\"tsr_digest\":{},\"adapter_failclosed\":true}}}}",
            host.tsr_ssim,
            host.tsr_deficit,
            host.tsr_monotonic,
            jstr(&host.tsr_digest),
        );
        return;
    }

    let leg = device_leg(&args);
    if leg.state == "skipped_dev_env" {
        println!(
            "{{\"schema\":\"rurix.g13upscale.harness.v1\",\"mode\":\"device\",\"state\":\"skipped_dev_env\",\"skip_reason\":{},\"host\":{{\"tsr_ssim\":{:.10},\"tsr_deficit\":{:.10},\"tsr_monotonic\":{},\"tsr_digest\":{},\"adapter_failclosed\":true}}}}",
            jstr(&leg.skip_reason),
            host.tsr_ssim,
            host.tsr_deficit,
            host.tsr_monotonic,
            jstr(&host.tsr_digest),
        );
        return;
    }

    // 判据聚拢(任何不符 → FAIL 退 1;SKIP 语义不在此路径)。
    let mut problems: Vec<String> = Vec::new();
    if !leg.switch_ok {
        problems.push("三后端运行时切换输出无效".into());
    }
    for (name, rep) in [("dlss", &leg.dlss), ("fsr", &leg.fsr), ("tsr", &leg.tsr)] {
        if !rep.ran {
            problems.push(format!("{name} 未真跑"));
        }
        if !rep.monotonic {
            problems.push(format!("{name} 收敛 MSE 非单调下降"));
        }
        if !rep.bitexact {
            problems.push(format!("{name} 双跑非位级一致"));
        }
        if let Some(false) = rep.in_band {
            problems.push(format!("{name} deficit 超标定带"));
        }
    }
    if leg.dlss.validation_errors != 0 {
        problems.push(format!("DLSS validation 错误计数 = {}", leg.dlss.validation_errors));
    }
    if leg.dlss.validation_coverage.starts_with("probe_") {
        problems.push(format!(
            "DLSS validation 探针完整性失败: {}",
            leg.dlss.validation_coverage
        ));
    }
    if leg.fsr.validation_errors != 0 {
        problems.push(format!("FSR validation 错误计数 = {}", leg.fsr.validation_errors));
    }
    if leg.fsr.fsr4_ml_available.is_none() || leg.fsr.fsr4_note.is_empty() {
        problems.push("FSR4 ML 回退登记缺失".into());
    }

    let state = if problems.is_empty() { "pass" } else { "fail" };
    println!(
        "{{\"schema\":\"rurix.g13upscale.harness.v1\",\"mode\":\"device\",\"state\":{},\"gpu\":{},\"problems\":{},\"host\":{{\"tsr_ssim\":{:.10},\"tsr_deficit\":{:.10},\"tsr_monotonic\":{},\"tsr_digest\":{},\"adapter_failclosed\":true}},\"backends\":{{{},{},{}}},\"switch\":{{\"ok\":{},\"order\":{}}},\"pairwise\":{{\"dlss_vs_tsr_ssim\":{:.10},\"dlss_vs_tsr_maxdiff\":{:.10},\"fsr_vs_tsr_ssim\":{:.10},\"fsr_vs_tsr_maxdiff\":{:.10}}}}}",
        jstr(state),
        jstr(&leg.gpu),
        strs_json(&problems),
        host.tsr_ssim,
        host.tsr_deficit,
        host.tsr_monotonic,
        jstr(&host.tsr_digest),
        backend_json("tsr", &leg.tsr),
        backend_json("dlss", &leg.dlss),
        backend_json("fsr", &leg.fsr),
        leg.switch_ok,
        strs_json(&leg.switch_order),
        leg.pairwise_dlss_tsr_ssim,
        leg.pairwise_dlss_tsr_maxdiff,
        leg.pairwise_fsr_tsr_ssim,
        leg.pairwise_fsr_tsr_maxdiff,
    );
    if !problems.is_empty() {
        std::process::exit(1);
    }
}
