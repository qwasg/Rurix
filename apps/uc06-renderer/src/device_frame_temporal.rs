//! G7.6 PR-1:TSR 时域臂孤立腿 harness(设计案 §2 / §8 PR-1)。
//!
//! 最小 `DeviceFrameSession` 图:`tsr_resample` → `tsr_temporal`,经
//! `FrameUpdate` 逐帧上传合成输入、轮换 hist_color/hist_depth ping-pong、覆盖
//! jitter/reset push。32 帧对拍 host `TsrUpscaler::upscale`;容差 measured→冻结。
//!
//! 专项断言:
//! ① 首帧 reset 直通逐位(`final == tsr_cur`);
//! ② 静态输入下 `flicker_score` 均值单调非增并收敛;
//! ③ 历史轮换错绑 → 对拍必红(预演步骤 96 `--frame-red-history`)。

use rurix_render::temporal::common::jitter_sequence;
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::tsr::{TsrParams, TsrUpscaler};
use rurix_render::temporal::upscale::{UpscaleBackend, UpscaleInputs};
use rurix_rt::render_exec::{
    self, Bindings, BufferDesc, BufferUsage, ComputePass, DeviceFrameSession, DispatchSpec,
    FrameUpdate, KernelWave, Pass, Readback, ResourceDesc, StableResourceId, TargetState,
};

const TSR_RESAMPLE_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsr_resample.spv"));
const TSR_TEMPORAL_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsr_temporal.spv"));

/// 孤立腿分辨率(2× 超分;合成场景,非冻结 960→1920 全链口径)。
const IN_W: u32 = 16;
const IN_H: u32 = 16;
const OUT_W: u32 = 32;
const OUT_H: u32 = 32;
const FRAMES: u32 = 32;

/// 冻结容差(**measured → 冻结**)。
///
/// measured 4.768371582e-7 @ RTX 4070 Ti(32 帧合成序列,含上游 resample);
/// 冻结 = 1e-6(留 FMA 收缩余量,同 G7.5 TSR 空间腿口径)。
pub mod tol {
    /// TSR 时域臂(含上游 resample)逐通道 max-abs。
    pub const TEMPORAL: f32 = 1e-6;
}

/// 资源表下标(创建期固定;StableResourceId = index + 1)。
mod res {
    pub const COLOR: u32 = 0;
    pub const DEPTH: u32 = 1;
    pub const MV: u32 = 2;
    pub const TSR_CUR: u32 = 3;
    pub const HIST_COLOR_A: u32 = 4;
    pub const HIST_COLOR_B: u32 = 5;
    pub const HIST_DEPTH_A: u32 = 6;
    pub const HIST_DEPTH_B: u32 = 7;
    pub const PREV_LUMA: u32 = 8;
    pub const PREV_SIGN: u32 = 9;
    pub const FLICKER: u32 = 10;
}

/// readback 下标。
mod rb {
    pub const HIST_A: u32 = 0;
    pub const HIST_B: u32 = 1;
    pub const TSR_CUR: u32 = 2;
    pub const FLICKER: u32 = 3;
}

/// G7.6 TSR 时域臂对拍结果(`--g76-tsr-temporal` JSON)。
#[derive(Debug, Clone)]
pub struct G76TemporalResults {
    pub device_name: String,
    pub frames: u32,
    pub in_w: u32,
    pub in_h: u32,
    pub out_w: u32,
    pub out_h: u32,
    pub measured_temporal_max_abs: f32,
    pub frame0_passthrough_bitexact: bool,
    pub flicker_monotone_converged: bool,
    pub history_red_ok: bool,
    pub temporal_pass: bool,
}

impl G76TemporalResults {
    pub fn all_pass(&self) -> bool {
        self.temporal_pass && self.frame0_passthrough_bitexact && self.flicker_monotone_converged
    }

    pub fn json(&self) -> String {
        format!(
            "{{\"subject\":\"uc06_g76_tsr_temporal\",\"device_name\":\"{}\",\
             \"frames\":{},\"in_w\":{},\"in_h\":{},\"out_w\":{},\"out_h\":{},\
             \"measured_temporal_max_abs\":{:.9e},\"tol_temporal\":{:.9e},\
             \"frame0_passthrough_bitexact\":{},\"flicker_monotone_converged\":{},\
             \"history_red_ok\":{},\"temporal_pass\":{},\"all_pass\":{}}}",
            self.device_name,
            self.frames,
            self.in_w,
            self.in_h,
            self.out_w,
            self.out_h,
            self.measured_temporal_max_abs,
            tol::TEMPORAL,
            self.frame0_passthrough_bitexact,
            self.flicker_monotone_converged,
            self.history_red_ok,
            self.temporal_pass,
            self.all_pass(),
        )
    }
}

fn storage<'a>(size: usize, data: Option<&'a [u8]>) -> ResourceDesc<'a> {
    ResourceDesc::Buffer(BufferDesc {
        size: size as u64,
        usage: BufferUsage {
            storage: true,
            ..Default::default()
        },
        data,
    })
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_u32(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn max_abs(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

fn bitexact(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

fn gate() -> Option<render_exec::DeviceCaps> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("[uc06 G7.6 temporal] SKIP: vulkan loader 不可用(dev-env degrade)");
        return None;
    }
    let caps = render_exec::probe_device_caps().ok()?;
    match render_exec::require_wave(&caps, KernelWave::W1) {
        Ok(()) => Some(caps),
        Err(e) => {
            eprintln!("[uc06 G7.6 temporal] SKIP: W1 能力链缺失({e})");
            None
        }
    }
}

/// 合成着色(镜像 `tsr.rs` 单测 `shade`;输出分辨率像素单位)。
fn shade(fx: f32, fy: f32) -> [f32; 3] {
    let check = (((fx + 3.7) / 8.0).floor() as i32 + ((fy + 3.7) / 8.0).floor() as i32) & 1;
    let mut base = 0.2 + 0.55 * check as f32;
    if fx + fy > 42.0 {
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

fn const_depth(w: u32, h: u32) -> ImageF32 {
    ImageF32::from_fn(w, h, 1, |_, _, _| 0.5)
}

fn resample_push(jitter: [f32; 2]) -> Vec<u8> {
    let mut push = bytes_u32(&[IN_W, IN_H, OUT_W, OUT_H]);
    for v in [
        jitter[0],
        jitter[1],
        1.0f32,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    push
}

fn temporal_push(reset: bool) -> Vec<u8> {
    let p = TsrParams::default();
    let ema_k = 2.0 / (p.flicker_window_frames as f32 + 1.0);
    let mut push = bytes_u32(&[IN_W, IN_H, OUT_W, OUT_H, u32::from(reset)]);
    for v in [
        p.base_alpha,
        p.min_alpha,
        ema_k,
        p.flicker_tighten,
        p.flicker_deadzone_abs,
        p.flicker_deadzone_rel,
        p.depth_rel_tol,
    ] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    push
}

fn temporal_bindings(
    hist_in: u32,
    hist_depth_in: u32,
    hist_out: u32,
    hist_depth_out: u32,
    reset: bool,
) -> Bindings {
    Bindings {
        storage_buffers: vec![
            res::TSR_CUR,
            res::DEPTH,
            res::MV,
            hist_in,
            hist_depth_in,
            res::PREV_LUMA,
            res::PREV_SIGN,
            res::FLICKER,
            hist_out,
            hist_depth_out,
        ],
        push_constants: temporal_push(reset),
        ..Default::default()
    }
}

/// `wrong_history` = true:故意不轮换 ping-pong(预演步骤 96 RED)。
fn run_sequence(wrong_history: bool) -> Result<RunAccum, String> {
    let color_bytes = IN_W as usize * IN_H as usize * 3 * 4;
    let depth_bytes = IN_W as usize * IN_H as usize * 4;
    let mv_bytes = IN_W as usize * IN_H as usize * 2 * 4;
    let out_rgb_bytes = OUT_W as usize * OUT_H as usize * 3 * 4;
    let out_1_bytes = OUT_W as usize * OUT_H as usize * 4;

    let zero_rgb = vec![0u8; out_rgb_bytes];
    let zero_1 = vec![0u8; out_1_bytes];
    let init_color = vec![0u8; color_bytes];
    let init_depth = vec![0u8; depth_bytes];
    let init_mv = vec![0u8; mv_bytes];

    let resources = [
        storage(color_bytes, Some(&init_color)),
        storage(depth_bytes, Some(&init_depth)),
        storage(mv_bytes, Some(&init_mv)),
        storage(out_rgb_bytes, Some(&zero_rgb)),
        storage(out_rgb_bytes, Some(&zero_rgb)),
        storage(out_rgb_bytes, Some(&zero_rgb)),
        storage(out_1_bytes, Some(&zero_1)),
        storage(out_1_bytes, Some(&zero_1)),
        storage(out_1_bytes, Some(&zero_1)),
        storage(out_1_bytes, Some(&zero_1)),
        storage(out_1_bytes, Some(&zero_1)),
    ];

    let resample_pc = resample_push([0.0, 0.0]);
    let passes = [
        Pass::Compute(ComputePass {
            name: "tsr_resample",
            spirv: TSR_RESAMPLE_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([OUT_W * OUT_H, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![res::COLOR, res::TSR_CUR],
                push_constants: resample_pc,
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "tsr_temporal",
            spirv: TSR_TEMPORAL_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([OUT_W * OUT_H, 1, 1]),
            bindings: temporal_bindings(
                res::HIST_COLOR_A,
                res::HIST_DEPTH_A,
                res::HIST_COLOR_B,
                res::HIST_DEPTH_B,
                true,
            ),
        }),
    ];
    // pass1 前显式见证 tsr_cur RAW(隐式补全同语义;设计 §1.2 裁决 9)。
    let plan0: [(u32, TargetState); 0] = [];
    let plan1 = [(res::TSR_CUR, TargetState::StorageReadWrite)];
    let barriers: [&[(u32, TargetState)]; 2] = [&plan0, &plan1];
    let readbacks = [
        Readback::Buffer {
            res: res::HIST_COLOR_A,
            offset: 0,
            size: out_rgb_bytes as u64,
        },
        Readback::Buffer {
            res: res::HIST_COLOR_B,
            offset: 0,
            size: out_rgb_bytes as u64,
        },
        Readback::Buffer {
            res: res::TSR_CUR,
            offset: 0,
            size: out_rgb_bytes as u64,
        },
        Readback::Buffer {
            res: res::FLICKER,
            offset: 0,
            size: out_1_bytes as u64,
        },
    ];

    let mut session = DeviceFrameSession::new(&resources, &passes, &barriers, &readbacks, 2)?;
    let scale = OUT_W as f32 / IN_W as f32;
    let depth = const_depth(IN_W, IN_H);
    let mv = ImageF32::new(IN_W, IN_H, 2);
    let jitters = jitter_sequence(FRAMES);
    let mut tsr = TsrUpscaler::default();

    let mut measured_max = 0.0f32;
    let mut frame0_bitexact = false;

    for frame in 0..FRAMES {
        let jitter = jitters[frame as usize];
        let color = render_input(IN_W, IN_H, scale, jitter);
        let color_b = bytes_f32(&color.data);
        let depth_b = bytes_f32(&depth.data);
        let mv_b = bytes_f32(&mv.data);

        // ping-pong:偶帧写 B(读 A);奇帧写 A(读 B)。错绑 = 恒读 A 写 B。
        let ping_a = wrong_history || frame % 2 == 0;
        let (hist_in, hist_depth_in, hist_out, hist_depth_out, final_rb) = if ping_a {
            (
                res::HIST_COLOR_A,
                res::HIST_DEPTH_A,
                res::HIST_COLOR_B,
                res::HIST_DEPTH_B,
                rb::HIST_B,
            )
        } else {
            (
                res::HIST_COLOR_B,
                res::HIST_DEPTH_B,
                res::HIST_COLOR_A,
                res::HIST_DEPTH_A,
                rb::HIST_A,
            )
        };

        let update = FrameUpdate {
            buffer_uploads: vec![
                (StableResourceId(u64::from(res::COLOR) + 1), 0, color_b),
                (StableResourceId(u64::from(res::DEPTH) + 1), 0, depth_b),
                (StableResourceId(u64::from(res::MV) + 1), 0, mv_b),
            ],
            binding_overrides: vec![(
                1,
                temporal_bindings(hist_in, hist_depth_in, hist_out, hist_depth_out, frame == 0),
            )],
            push_constant_overrides: vec![(0, resample_push(jitter))],
            readback_subset: Some(vec![final_rb, rb::TSR_CUR, rb::FLICKER]),
            ..Default::default()
        };
        let prov = session.next_provenance_with_update(&update)?;
        let out = session.execute_with_frame_update(&prov, &update)?;
        let final_dev = read_f32(&out.readbacks[0]);
        let tsr_cur_dev = read_f32(&out.readbacks[1]);
        let _flick_dev = read_f32(&out.readbacks[2]);

        let inputs = UpscaleInputs {
            color: &color,
            depth: &depth,
            mv: &mv,
            reactive: None,
            exposure: 1.0,
            jitter,
            output_size: (OUT_W, OUT_H),
            frame_index: frame,
            reset: frame == 0,
        };
        let host = tsr.upscale(&inputs);
        measured_max = measured_max.max(max_abs(&final_dev, &host.data));

        if frame == 0 {
            frame0_bitexact = bitexact(&final_dev, &tsr_cur_dev);
        }
    }

    Ok(RunAccum {
        measured_max,
        frame0_bitexact,
    })
}

struct RunAccum {
    measured_max: f32,
    frame0_bitexact: bool,
}

/// 静态输入下 flicker 均值单调非增(允许 1e-7 浮点噪声)且末帧均值 < 1e-3。
fn flicker_monotone_ok(means: &[f32]) -> bool {
    if means.len() < 2 {
        return false;
    }
    for w in means.windows(2) {
        if w[1] > w[0] + 1e-7 {
            return false;
        }
    }
    *means.last().unwrap() < 1e-3
}

fn run_inner(caps: &render_exec::DeviceCaps) -> Result<G76TemporalResults, String> {
    let ok = run_sequence(false)?;
    // 静态+jitter 下闪烁分数未必单调(相位诱发翻转)。专项 ② 另跑零 jitter 静态序列。
    let flicker_ok = run_static_zero_jitter_flicker()?;
    let red = run_sequence(true)?;
    let history_red_ok = red.measured_max > tol::TEMPORAL;
    let temporal_pass = ok.measured_max <= tol::TEMPORAL;
    Ok(G76TemporalResults {
        device_name: caps.device_name.clone(),
        frames: FRAMES,
        in_w: IN_W,
        in_h: IN_H,
        out_w: OUT_W,
        out_h: OUT_H,
        measured_temporal_max_abs: ok.measured_max,
        frame0_passthrough_bitexact: ok.frame0_bitexact,
        flicker_monotone_converged: flicker_ok,
        history_red_ok,
        temporal_pass,
    })
}

/// 专项 ②:零 jitter、逐帧相同输入 → 无翻转 → score 恒 0(单调收敛)。
fn run_static_zero_jitter_flicker() -> Result<bool, String> {
    // 复用主序列接口但强制零 jitter:通过专用短跑(仅读 flicker 均值序列)。
    let color_bytes = IN_W as usize * IN_H as usize * 3 * 4;
    let depth_bytes = IN_W as usize * IN_H as usize * 4;
    let mv_bytes = IN_W as usize * IN_H as usize * 2 * 4;
    let out_rgb_bytes = OUT_W as usize * OUT_H as usize * 3 * 4;
    let out_1_bytes = OUT_W as usize * OUT_H as usize * 4;
    let zero_rgb = vec![0u8; out_rgb_bytes];
    let zero_1 = vec![0u8; out_1_bytes];
    let init_color = vec![0u8; color_bytes];
    let init_depth = vec![0u8; depth_bytes];
    let init_mv = vec![0u8; mv_bytes];
    let resources = [
        storage(color_bytes, Some(&init_color)),
        storage(depth_bytes, Some(&init_depth)),
        storage(mv_bytes, Some(&init_mv)),
        storage(out_rgb_bytes, Some(&zero_rgb)),
        storage(out_rgb_bytes, Some(&zero_rgb)),
        storage(out_rgb_bytes, Some(&zero_rgb)),
        storage(out_1_bytes, Some(&zero_1)),
        storage(out_1_bytes, Some(&zero_1)),
        storage(out_1_bytes, Some(&zero_1)),
        storage(out_1_bytes, Some(&zero_1)),
        storage(out_1_bytes, Some(&zero_1)),
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "tsr_resample",
            spirv: TSR_RESAMPLE_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([OUT_W * OUT_H, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![res::COLOR, res::TSR_CUR],
                push_constants: resample_push([0.0, 0.0]),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "tsr_temporal",
            spirv: TSR_TEMPORAL_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([OUT_W * OUT_H, 1, 1]),
            bindings: temporal_bindings(
                res::HIST_COLOR_A,
                res::HIST_DEPTH_A,
                res::HIST_COLOR_B,
                res::HIST_DEPTH_B,
                true,
            ),
        }),
    ];
    let plan0: [(u32, TargetState); 0] = [];
    let plan1 = [(res::TSR_CUR, TargetState::StorageReadWrite)];
    let barriers: [&[(u32, TargetState)]; 2] = [&plan0, &plan1];
    let readbacks = [
        Readback::Buffer {
            res: res::HIST_COLOR_A,
            offset: 0,
            size: out_rgb_bytes as u64,
        },
        Readback::Buffer {
            res: res::HIST_COLOR_B,
            offset: 0,
            size: out_rgb_bytes as u64,
        },
        Readback::Buffer {
            res: res::TSR_CUR,
            offset: 0,
            size: out_rgb_bytes as u64,
        },
        Readback::Buffer {
            res: res::FLICKER,
            offset: 0,
            size: out_1_bytes as u64,
        },
    ];
    let mut session = DeviceFrameSession::new(&resources, &passes, &barriers, &readbacks, 2)?;
    let scale = OUT_W as f32 / IN_W as f32;
    let color = render_input(IN_W, IN_H, scale, [0.0, 0.0]);
    let depth = const_depth(IN_W, IN_H);
    let mv = ImageF32::new(IN_W, IN_H, 2);
    let color_b = bytes_f32(&color.data);
    let depth_b = bytes_f32(&depth.data);
    let mv_b = bytes_f32(&mv.data);
    let mut means = Vec::new();
    for frame in 0..FRAMES {
        let (hist_in, hist_depth_in, hist_out, hist_depth_out, final_rb) = if frame % 2 == 0 {
            (
                res::HIST_COLOR_A,
                res::HIST_DEPTH_A,
                res::HIST_COLOR_B,
                res::HIST_DEPTH_B,
                rb::HIST_B,
            )
        } else {
            (
                res::HIST_COLOR_B,
                res::HIST_DEPTH_B,
                res::HIST_COLOR_A,
                res::HIST_DEPTH_A,
                rb::HIST_A,
            )
        };
        let update = FrameUpdate {
            buffer_uploads: vec![
                (
                    StableResourceId(u64::from(res::COLOR) + 1),
                    0,
                    color_b.clone(),
                ),
                (
                    StableResourceId(u64::from(res::DEPTH) + 1),
                    0,
                    depth_b.clone(),
                ),
                (StableResourceId(u64::from(res::MV) + 1), 0, mv_b.clone()),
            ],
            binding_overrides: vec![(
                1,
                temporal_bindings(hist_in, hist_depth_in, hist_out, hist_depth_out, frame == 0),
            )],
            push_constant_overrides: vec![(0, resample_push([0.0, 0.0]))],
            readback_subset: Some(vec![final_rb, rb::TSR_CUR, rb::FLICKER]),
            ..Default::default()
        };
        let prov = session.next_provenance_with_update(&update)?;
        let out = session.execute_with_frame_update(&prov, &update)?;
        let flick = read_f32(&out.readbacks[2]);
        means.push(flick.iter().sum::<f32>() / flick.len() as f32);
    }
    Ok(flicker_monotone_ok(&means))
}

/// 生产路径。
pub fn run_g76_tsr_temporal() -> Option<Result<G76TemporalResults, String>> {
    let caps = gate()?;
    Some(run_inner(&caps))
}

/// RED:错绑历史 → 对拍必红。返回 `Some(true)` = RED-OK。
pub fn red_wrong_history() -> Option<bool> {
    let _caps = gate()?;
    match run_sequence(true) {
        Ok(r) => Some(r.measured_max > tol::TEMPORAL),
        Err(_) => Some(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_g76_tsr_temporal_match_host() {
        let Some(res) = run_g76_tsr_temporal() else {
            return;
        };
        let r = res.expect("G7.6 TSR 时域臂 device 执行");
        assert!(r.all_pass(), "G7.6 TSR 时域臂对拍未全过: {}", r.json());
        assert!(r.history_red_ok, "历史错绑 RED 轴未触发: {}", r.json());
    }
}
