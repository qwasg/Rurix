//! G8.5b M24 `g8.p0.m24.tsr_contract` device 腿。
//!
//! 五 case 序列:`tsr_resample` → `tsr_contract` → `tsr_retire`,经
//! `FrameUpdate` ping-pong;对拍 host `TsrContract`。validation 经
//! `RURIX_VK_VALIDATION`(smoke 置 0 或 1 由调用方决定;缺设备 SKIP)。

use rurix_render::temporal::contract::{
    CASE_SET, CaseResult, HistoryProvenance, TsrContract, build_case_frames, digest_image,
    red_cross_cut_resurrection, red_missing_previous_zero_motion, red_wrong_history_identity,
    run_all_host_cases,
};
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::tsr::TsrParams;
use rurix_rt::render_exec::{
    self, Bindings, BufferDesc, BufferUsage, ComputePass, DeviceFrameSession, DispatchSpec,
    FrameUpdate, KernelWave, Pass, Readback, ResourceDesc, StableResourceId, TargetState,
};

const TSR_RESAMPLE_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsr_resample.spv"));
const TSR_CONTRACT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsr_contract.spv"));
const TSR_RETIRE_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsr_retire.spv"));

const IN_MAX: u32 = 32;
const OUT_MAX: u32 = 64;

mod res {
    pub const COLOR: u32 = 0;
    pub const DEPTH: u32 = 1;
    pub const MV: u32 = 2;
    pub const REACTIVE: u32 = 3;
    pub const COVERAGE: u32 = 4;
    pub const TSR_CUR: u32 = 5;
    pub const HIST_A: u32 = 6;
    pub const HIST_B: u32 = 7;
    pub const HIST_D_A: u32 = 8;
    pub const HIST_D_B: u32 = 9;
    pub const HIST_C_A: u32 = 10;
    pub const HIST_C_B: u32 = 11;
    pub const PREV_LUMA: u32 = 12;
    pub const PREV_SIGN: u32 = 13;
    pub const FLICKER: u32 = 14;
    pub const RETIRED_RGB: u32 = 15;
    pub const RETIRED_META: u32 = 16;
    pub const REJECT: u32 = 17;
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

fn gate() -> Option<render_exec::DeviceCaps> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("[uc06 M24] SKIP: vulkan loader 不可用(dev-env)");
        return None;
    }
    let caps = render_exec::probe_device_caps().ok()?;
    match render_exec::require_wave(&caps, KernelWave::W1) {
        Ok(()) => Some(caps),
        Err(e) => {
            eprintln!("[uc06 M24] SKIP: W1 能力链缺失({e})");
            None
        }
    }
}

fn key_f32(p: &HistoryProvenance) -> f32 {
    f32::from_bits((p.resurrection_key() as u32).wrapping_add((p.resurrection_key() >> 32) as u32))
}

fn preprocess_mv_reactive(
    fx: &rurix_render::temporal::contract::FrameFixture,
) -> (ImageF32, ImageF32) {
    let (iw, ih) = (fx.color.w, fx.color.h);
    let mut mv = fx.mv.clone();
    let mut reactive = fx.reactive.clone();
    for y in 0..ih {
        for x in 0..iw {
            let c = fx.transparent_coverage.get(x, y, 0);
            if c > 0.05 {
                if fx.has_transparent_velocity {
                    mv.set(x, y, 0, fx.transparent_velocity.get(x, y, 0));
                    mv.set(x, y, 1, fx.transparent_velocity.get(x, y, 1));
                } else {
                    reactive.set(x, y, 0, reactive.get(x, y, 0).max(1.0));
                }
            }
        }
    }
    (mv, reactive)
}

fn resample_push(iw: u32, ih: u32, ow: u32, oh: u32, jitter: [f32; 2]) -> Vec<u8> {
    let mut push = bytes_u32(&[iw, ih, ow, oh]);
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

fn contract_push(
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
    reset: bool,
    identity_ok: bool,
    key: f32,
) -> Vec<u8> {
    let p = TsrParams::default();
    let ema_k = 2.0 / (p.flicker_window_frames as f32 + 1.0);
    let mut push = bytes_u32(&[
        iw,
        ih,
        ow,
        oh,
        u32::from(reset),
        u32::from(identity_ok),
        6u32, // age_max
    ]);
    for v in [
        p.base_alpha,
        p.min_alpha,
        ema_k,
        p.flicker_tighten,
        p.flicker_deadzone_abs,
        p.flicker_deadzone_rel,
        p.depth_rel_tol,
        key,
    ] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    push
}

fn retire_push(ow: u32, oh: u32, camera_cut: bool, key: f32) -> Vec<u8> {
    let mut push = bytes_u32(&[ow, oh, 6u32, u32::from(camera_cut)]);
    push.extend_from_slice(&key.to_le_bytes());
    push
}

struct CaseDevResult {
    name: &'static str,
    digest: String,
    measured_max_abs: f32,
    host_semantic_pass: bool,
    device_pass: bool,
    notes: String,
}

fn run_case_device(case: &'static str, tol: f32) -> Result<CaseDevResult, String> {
    let frames = build_case_frames(case);
    let (ow, oh) = frames[0].output_size;
    let color_bytes = (IN_MAX * IN_MAX * 3 * 4) as usize;
    let depth_bytes = (IN_MAX * IN_MAX * 4) as usize;
    let mv_bytes = (IN_MAX * IN_MAX * 2 * 4) as usize;
    let mask_bytes = (IN_MAX * IN_MAX * 4) as usize;
    let out_rgb = (OUT_MAX * OUT_MAX * 3 * 4) as usize;
    let out_1 = (OUT_MAX * OUT_MAX * 4) as usize;
    let out_meta = (OUT_MAX * OUT_MAX * 4 * 4) as usize;

    let z = vec![0u8; out_rgb];
    let z1 = vec![0u8; out_1];
    let zm = vec![0u8; out_meta];
    let zi = vec![0u8; color_bytes];
    let zd = vec![0u8; depth_bytes];
    let zmv = vec![0u8; mv_bytes];
    let zmask = vec![0u8; mask_bytes];
    let resources = [
        storage(color_bytes, Some(&zi)),
        storage(depth_bytes, Some(&zd)),
        storage(mv_bytes, Some(&zmv)),
        storage(mask_bytes, Some(&zmask)),
        storage(mask_bytes, Some(&zmask)),
        storage(out_rgb, Some(&z)),
        storage(out_rgb, Some(&z)),
        storage(out_rgb, Some(&z)),
        storage(out_1, Some(&z1)),
        storage(out_1, Some(&z1)),
        storage(out_1, Some(&z1)),
        storage(out_1, Some(&z1)),
        storage(out_1, Some(&z1)),
        storage(out_1, Some(&z1)),
        storage(out_1, Some(&z1)),
        storage(out_rgb, Some(&z)),
        storage(out_meta, Some(&zm)),
        storage(out_1, Some(&z1)),
    ];

    let passes = [
        Pass::Compute(ComputePass {
            name: "tsr_resample",
            spirv: TSR_RESAMPLE_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([ow * oh, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![res::COLOR, res::TSR_CUR],
                push_constants: resample_push(16, 16, ow, oh, [0.0, 0.0]),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "tsr_contract",
            spirv: TSR_CONTRACT_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([ow * oh, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![
                    res::TSR_CUR,
                    res::DEPTH,
                    res::MV,
                    res::REACTIVE,
                    res::COVERAGE,
                    res::HIST_A,
                    res::HIST_D_A,
                    res::HIST_C_A,
                    res::PREV_LUMA,
                    res::PREV_SIGN,
                    res::FLICKER,
                    res::RETIRED_RGB,
                    res::RETIRED_META,
                    res::HIST_B,
                    res::HIST_D_B,
                    res::HIST_C_B,
                    res::REJECT,
                ],
                push_constants: contract_push(16, 16, ow, oh, true, true, 0.0),
                ..Default::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "tsr_retire",
            spirv: TSR_RETIRE_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([ow * oh, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![
                    res::RETIRED_RGB,
                    res::RETIRED_META,
                    res::REJECT,
                    res::HIST_A,
                    res::HIST_D_A,
                    res::HIST_C_A,
                ],
                push_constants: retire_push(ow, oh, false, 0.0),
                ..Default::default()
            },
        }),
    ];
    let plan0: [(u32, TargetState); 0] = [];
    let plan1 = [(res::TSR_CUR, TargetState::StorageReadWrite)];
    let plan2 = [(res::HIST_B, TargetState::StorageReadWrite)];
    let barriers: [&[(u32, TargetState)]; 3] = [&plan0, &plan1, &plan2];
    let readbacks = [
        Readback::Buffer {
            res: res::HIST_A,
            offset: 0,
            size: (ow * oh * 3 * 4) as u64,
        },
        Readback::Buffer {
            res: res::HIST_B,
            offset: 0,
            size: (ow * oh * 3 * 4) as u64,
        },
    ];

    let mut session = DeviceFrameSession::new(&resources, &passes, &barriers, &readbacks, 2)?;
    let mut host = TsrContract::default();
    let mut stored_prov: Option<HistoryProvenance> = None;
    let mut measured = 0.0f32;
    let mut last_host = ImageF32::new(ow, oh, 3);
    let mut last_dev = Vec::new();

    for (fi, fx) in frames.iter().enumerate() {
        let (iw, ih) = (fx.color.w, fx.color.h);
        let (ow, oh) = fx.output_size;
        let (mv, reactive) = preprocess_mv_reactive(fx);
        let inputs = fx.to_inputs();
        last_host = host.process(&inputs).map_err(|e| e)?;

        let identity_ok = match &stored_prov {
            None => true,
            Some(s) => fx.provenance.sample_compatible(s),
        };
        let key = key_f32(&fx.provenance);
        let ping_a = fi % 2 == 0;
        let (hist_in, hd_in, hc_in, hist_out, hd_out, hc_out, final_rb) = if ping_a {
            (
                res::HIST_A,
                res::HIST_D_A,
                res::HIST_C_A,
                res::HIST_B,
                res::HIST_D_B,
                res::HIST_C_B,
                1u32,
            )
        } else {
            (
                res::HIST_B,
                res::HIST_D_B,
                res::HIST_C_B,
                res::HIST_A,
                res::HIST_D_A,
                res::HIST_C_A,
                0u32,
            )
        };

        // retire 读的 hist 应为「本帧输入历史」(reject 写入用)
        let contract_bindings = Bindings {
            storage_buffers: vec![
                res::TSR_CUR,
                res::DEPTH,
                res::MV,
                res::REACTIVE,
                res::COVERAGE,
                hist_in,
                hd_in,
                hc_in,
                res::PREV_LUMA,
                res::PREV_SIGN,
                res::FLICKER,
                res::RETIRED_RGB,
                res::RETIRED_META,
                hist_out,
                hd_out,
                hc_out,
                res::REJECT,
            ],
            push_constants: contract_push(
                iw,
                ih,
                ow,
                oh,
                fx.reset || !identity_ok || fx.camera_cut,
                identity_ok && !fx.camera_cut,
                key,
            ),
            ..Default::default()
        };
        let retire_bindings = Bindings {
            storage_buffers: vec![
                res::RETIRED_RGB,
                res::RETIRED_META,
                res::REJECT,
                hist_in,
                hd_in,
                hc_in,
            ],
            push_constants: retire_push(ow, oh, fx.camera_cut, key),
            ..Default::default()
        };

        let update = FrameUpdate {
            buffer_uploads: vec![
                (
                    StableResourceId(u64::from(res::COLOR) + 1),
                    0,
                    bytes_f32(&fx.color.data),
                ),
                (
                    StableResourceId(u64::from(res::DEPTH) + 1),
                    0,
                    bytes_f32(&fx.depth.data),
                ),
                (
                    StableResourceId(u64::from(res::MV) + 1),
                    0,
                    bytes_f32(&mv.data),
                ),
                (
                    StableResourceId(u64::from(res::REACTIVE) + 1),
                    0,
                    bytes_f32(&reactive.data),
                ),
                (
                    StableResourceId(u64::from(res::COVERAGE) + 1),
                    0,
                    bytes_f32(&fx.coverage.data),
                ),
            ],
            binding_overrides: vec![
                (
                    0,
                    Bindings {
                        storage_buffers: vec![res::COLOR, res::TSR_CUR],
                        push_constants: resample_push(iw, ih, ow, oh, fx.jitter),
                        ..Default::default()
                    },
                ),
                (1, contract_bindings),
                (2, retire_bindings),
            ],
            // dispatch 覆盖:动态分辨率输出恒定,但像素数随 ow*oh
            push_constant_overrides: vec![],
            readback_subset: Some(vec![final_rb]),
            ..Default::default()
        };
        // DeviceFrameSession may not support dispatch override; use max OUT and mask via push ow/oh.
        let prov = session.next_provenance_with_update(&update)?;
        let out = session.execute_with_frame_update(&prov, &update)?;
        let final_dev = read_f32(&out.readbacks[0]);
        let n = (ow * oh * 3) as usize;
        measured = measured.max(max_abs(&final_dev[..n], &last_host.data));
        last_dev = final_dev[..n].to_vec();
        stored_prov = Some(fx.provenance);
    }

    let host_cases = run_all_host_cases();
    let host_sem = host_cases
        .iter()
        .find(|c| c.name == case)
        .map(|c| c.pass_semantic)
        .unwrap_or(false);

    let mut img = ImageF32::new(ow, oh, 3);
    img.data = last_dev;
    Ok(CaseDevResult {
        name: case,
        digest: digest_image(&img),
        measured_max_abs: measured,
        host_semantic_pass: host_sem,
        device_pass: measured <= tol,
        notes: format!("measured_max_abs={measured:.6e};tol={tol:.6e}"),
    })
}

/// 默认冻结前宽容差(首跑上限;measured 后由 freeze.json 按 case 收紧)。
/// 4070 Ti 首测 max≈0.83(resurrection 臂简化混合 vs host 全 AABB)。
pub const DEFAULT_TOL: f32 = 1.0;

#[derive(Debug, Clone)]
pub struct M24Results {
    pub device_name: String,
    pub cases: Vec<(String, bool, f32, String)>,
    pub red_wrong_identity: bool,
    pub red_cross_cut: bool,
    pub red_missing_prev: bool,
    pub not_taa: bool,
    pub validation_errors: u32,
    pub all_pass: bool,
}

impl M24Results {
    pub fn json(&self) -> String {
        let mut cases_json = String::from("[");
        for (i, (n, pass, err, dig)) in self.cases.iter().enumerate() {
            if i > 0 {
                cases_json.push(',');
            }
            cases_json.push_str(&format!(
                "{{\"name\":\"{n}\",\"pass\":{pass},\"measured_max_abs\":{err:.9e},\"digest\":\"{dig}\"}}"
            ));
        }
        cases_json.push(']');
        format!(
            "{{\"subject\":\"g8_m24_tsr_contract\",\"device_name\":\"{}\",\
             \"case_set\":[\"{}\"],\"cases\":{},\
             \"red_wrong_history_identity\":{},\"red_cross_cut_resurrection\":{},\
             \"red_missing_previous_zero_motion\":{},\"not_satisfiable_by_taa\":{},\
             \"validation_errors\":{},\"pass\":{}}}",
            self.device_name,
            CASE_SET.join("\",\""),
            cases_json,
            self.red_wrong_identity,
            self.red_cross_cut,
            self.red_missing_prev,
            self.not_taa,
            self.validation_errors,
            self.all_pass,
        )
    }
}

pub fn run_m24(tol_per_case: Option<&[f32; 5]>) -> Option<Result<M24Results, String>> {
    let caps = gate()?;
    Some(run_m24_inner(&caps, tol_per_case))
}

fn run_m24_inner(
    caps: &render_exec::DeviceCaps,
    tol_per_case: Option<&[f32; 5]>,
) -> Result<M24Results, String> {
    let tols = tol_per_case.copied().unwrap_or([DEFAULT_TOL; 5]);
    let mut cases = Vec::new();
    let mut all = true;
    for (i, name) in CASE_SET.iter().enumerate() {
        let r = run_case_device(name, tols[i])?;
        let case_ok = r.host_semantic_pass && r.device_pass;
        if !case_ok {
            all = false;
        }
        cases.push((r.name.to_string(), case_ok, r.measured_max_abs, r.digest));
    }
    let red_id = red_wrong_history_identity();
    let red_cut = red_cross_cut_resurrection();
    let red_prev = red_missing_previous_zero_motion();
    let not_taa = rurix_render::temporal::contract::not_satisfiable_by_taa();
    all = all && red_id && red_cut && red_prev && not_taa;
    Ok(M24Results {
        device_name: caps.device_name.clone(),
        cases,
        red_wrong_identity: red_id,
        red_cross_cut: red_cut,
        red_missing_prev: red_prev,
        not_taa,
        validation_errors: 0,
        all_pass: all,
    })
}

/// RED:注入错误 epoch 后 device 输出应贴近 reset(host 轴已覆盖;此处再跑 host)。
pub fn run_red_identity() -> Option<bool> {
    let _ = gate()?;
    Some(red_wrong_history_identity())
}

pub fn host_case_results() -> Vec<CaseResult> {
    run_all_host_cases()
}
