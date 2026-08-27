// Assisted-by: Cursor Agent（G19.2 M-a 实现波）
//! G19.2 M-a FG/MFG 帧生成独立层 host 参考臂 probe（门
//! `g19.p0.m_a.frame_generation_host_realization`；RFC-0036；G13-N7 兑现）。
//!
//! ## 职责闭集
//!
//! 1. **确定性程序化动画序列**（解析式 ground truth 全帧率渲染）：平移背景
//!    （低频正弦，双线性友好）+ 自运动软边圆盘 sprite——每帧均可解析求值，
//!    构成插帧质量的无争议 GT。
//! 2. **三档 MFG 车道**（×2/×3/×4）：真渲帧 = 步长 N+1 的子采样；中间帧由
//!    `rurix_render::temporal::framegen` host 参考臂生成（mv 场解析产出，
//!    与渲染器几何 pass 产 mv 同语义）。
//! 3. **程序产对照阈（禁手写）**：逐生成帧 `SSIM(interp, GT) >
//!    SSIM(frame_hold, GT)`——frame-hold = 复制最近真渲帧的零成本下界，
//!    插帧必须逐帧严格优于（RFC-0036 §1.2）。
//! 4. **两口径账目（G13-N7 字面 0-byte）**：`real_render_fps` 只由真渲帧构成；
//!    `presented_fps`（真渲 + 生成）独立登记面，永不混算——probe 内重算
//!    恒等式核验。
//! 5. **双跑位级确定性**：全部生成帧字节 sha256（rurix-pkg 同源实现）双跑
//!    比对。
//!
//! ## 用法
//!
//! ```text
//! g19_frame_gen_probe --out evidence/g19_frame_gen_probe_<UTC>.json
//! ```

#![forbid(unsafe_code)]

use rurix_render::temporal::framegen::{
    FgAccounting, FrameGenParams, interpolate, mfg_inserted_frames,
};
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::ssim::ssim;
use std::time::Instant;

const W: u32 = 160;
const H: u32 = 96;
/// 全帧率 GT 帧数（0..=48：被 2/3/4 步长同时整除的收口索引 48）。
const FULL_RATE_FRAMES: u32 = 49;
/// 背景 uv 速度（每全帧率帧）。
const BG_VEL: [f32; 2] = [1.6 / W as f32, 0.9 / H as f32];
/// sprite uv 速度（每全帧率帧，自运动与背景不同向）。
const SPRITE_VEL: [f32; 2] = [-2.2 / W as f32, 1.4 / H as f32];
const SPRITE_R: f32 = 0.11;
const SPRITE_SOFT: f32 = 0.02;

fn sprite_center(k: f32) -> [f32; 2] {
    [0.30 + k * SPRITE_VEL[0], 0.35 + k * SPRITE_VEL[1]]
}

/// 解析式场景渲染（时间 k 连续可求值 → 任意 t 的 GT 存在）。
fn render_frame(k: f32) -> ImageF32 {
    let c = sprite_center(k);
    ImageF32::from_fn(W, H, 3, |x, y, ch| {
        let u = (x as f32 + 0.5) / W as f32;
        let v = (y as f32 + 0.5) / H as f32;
        // 平移背景（低频正弦，双线性友好）
        let bu = u - k * BG_VEL[0];
        let bv = v - k * BG_VEL[1];
        let base = 0.45
            + 0.30
                * ((bu * 5.0) * std::f32::consts::PI).sin()
                * ((bv * 3.0) * std::f32::consts::PI).cos()
            + 0.04 * ch as f32;
        // 软边圆盘 sprite（自运动）
        let d = ((u - c[0]) * (u - c[0]) + (v - c[1]) * (v - c[1])).sqrt();
        let cover = if d <= SPRITE_R {
            1.0
        } else if d >= SPRITE_R + SPRITE_SOFT {
            0.0
        } else {
            1.0 - (d - SPRITE_R) / SPRITE_SOFT
        };
        let sprite = [0.85f32, 0.25, 0.15][ch as usize];
        (base * (1.0 - cover) + sprite * cover).clamp(0.0, 1.0)
    })
}

/// 解析 mv 场（prev→cur 的 uv 位移，按 cur 帧栅格；sprite 掩码内 = sprite
/// 位移，否则背景位移——与渲染器几何 pass 产 mv 同语义）。
fn mv_field(cur_k: f32, pair_frames: f32) -> ImageF32 {
    let c = sprite_center(cur_k);
    ImageF32::from_fn(W, H, 2, |x, y, ch| {
        let u = (x as f32 + 0.5) / W as f32;
        let v = (y as f32 + 0.5) / H as f32;
        let d = ((u - c[0]) * (u - c[0]) + (v - c[1]) * (v - c[1])).sqrt();
        let vel = if d <= SPRITE_R + SPRITE_SOFT {
            SPRITE_VEL
        } else {
            BG_VEL
        };
        vel[ch as usize] * pair_frames
    })
}

struct LaneReport {
    mode_x: u32,
    generated: u32,
    ssim_interp_mean: f64,
    ssim_hold_mean: f64,
    ssim_interp_min: f64,
    min_margin: f64,
    all_frames_interp_gt_hold: bool,
    acc: FgAccounting,
    digest: String,
}

fn run_lane(mode_x: u32, gt: &[ImageF32]) -> LaneReport {
    let n = mfg_inserted_frames(mode_x);
    let step = mode_x; // 真渲帧步长 = N+1
    let params = FrameGenParams {
        inserted_per_pair: n,
        ..Default::default()
    };
    let mut acc = FgAccounting::default();
    let mut interp_scores: Vec<f64> = Vec::new();
    let mut hold_scores: Vec<f64> = Vec::new();
    let mut gen_bytes: Vec<u8> = Vec::new();

    // 真渲帧耗时（独立重渲计时，GT 预渲不计入口径）
    let t0 = Instant::now();
    let mut real_indices: Vec<u32> = Vec::new();
    let mut k = 0u32;
    while k < FULL_RATE_FRAMES {
        let _ = render_frame(k as f32);
        real_indices.push(k);
        k += step;
    }
    acc.real_frames = real_indices.len() as u64;
    acc.real_render_seconds = t0.elapsed().as_secs_f64();

    let tg = Instant::now();
    for pair in real_indices.windows(2) {
        let (pk, ck) = (pair[0], pair[1]);
        let prev = &gt[pk as usize];
        let cur = &gt[ck as usize];
        let mv = mv_field(ck as f32, (ck - pk) as f32);
        for i in 1..=n {
            let t = i as f32 / (n + 1) as f32;
            let generated = interpolate(prev, cur, &mv, t, &params);
            let gt_idx = pk + i * (ck - pk) / (n + 1);
            let s_interp = ssim(&generated, &gt[gt_idx as usize]);
            let s_hold = ssim(prev, &gt[gt_idx as usize]);
            interp_scores.push(s_interp);
            hold_scores.push(s_hold);
            for &f in &generated.data {
                gen_bytes.extend_from_slice(&f.to_le_bytes());
            }
            acc.generated_frames += 1;
        }
    }
    acc.generation_seconds = tg.elapsed().as_secs_f64();

    let margins: Vec<f64> = interp_scores
        .iter()
        .zip(hold_scores.iter())
        .map(|(a, b)| a - b)
        .collect();
    LaneReport {
        mode_x,
        generated: acc.generated_frames as u32,
        ssim_interp_mean: interp_scores.iter().sum::<f64>() / interp_scores.len() as f64,
        ssim_hold_mean: hold_scores.iter().sum::<f64>() / hold_scores.len() as f64,
        ssim_interp_min: interp_scores.iter().cloned().fold(f64::INFINITY, f64::min),
        min_margin: margins.iter().cloned().fold(f64::INFINITY, f64::min),
        all_frames_interp_gt_hold: margins.iter().all(|&m| m > 0.0),
        acc,
        digest: rurix_pkg::sha256::hex_digest(&gen_bytes),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut out_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                out_path = Some(args[i + 1].clone());
                i += 2;
            }
            other => {
                eprintln!("g19_frame_gen_probe: FAIL 未知参数 {other}");
                std::process::exit(2);
            }
        }
    }
    let out_path = out_path.unwrap_or_else(|| {
        eprintln!("g19_frame_gen_probe: FAIL 缺 --out <evidence.json>");
        std::process::exit(2);
    });

    // GT 全帧率序列（解析式，一次预渲供全部车道对照）
    let gt: Vec<ImageF32> = (0..FULL_RATE_FRAMES)
        .map(|k| render_frame(k as f32))
        .collect();

    // 三档车道 run1
    let lanes: Vec<LaneReport> = [2u32, 3, 4].iter().map(|&m| run_lane(m, &gt)).collect();
    // 双跑位级确定性（run2 只比对 digest）
    let lanes_run2: Vec<LaneReport> = [2u32, 3, 4].iter().map(|&m| run_lane(m, &gt)).collect();
    let double_run_bitexact = lanes
        .iter()
        .zip(lanes_run2.iter())
        .all(|(a, b)| a.digest == b.digest);

    // 口径恒等式重算核验（真实渲染帧率禁计生成帧）
    let caliber_ok = lanes.iter().all(|l| {
        let manual_real = l.acc.real_frames as f64 / l.acc.real_render_seconds;
        (l.acc.real_render_fps() - manual_real).abs() < 1e-9
            && l.acc.presented_frames() == l.acc.real_frames + l.acc.generated_frames
    });

    let all_quality = lanes.iter().all(|l| l.all_frames_interp_gt_hold);

    let mut lanes_json = String::new();
    for (idx, l) in lanes.iter().enumerate() {
        if idx > 0 {
            lanes_json.push(',');
        }
        lanes_json.push_str(&format!(
            "{{\"mode_x\":{},\"real_frames\":{},\"generated_frames\":{},\
             \"ssim_interp_mean\":{:.6},\"ssim_hold_mean\":{:.6},\"ssim_interp_min\":{:.6},\
             \"min_margin\":{:.6},\"all_frames_interp_gt_hold\":{},\
             \"real_render_seconds\":{:.6},\"generation_seconds\":{:.6},\
             \"real_render_fps\":{:.3},\"presented_fps\":{:.3},\"presented_frames\":{},\
             \"generated_digest\":\"sha256:{}\"}}",
            l.mode_x,
            l.acc.real_frames,
            l.acc.generated_frames,
            l.ssim_interp_mean,
            l.ssim_hold_mean,
            l.ssim_interp_min,
            l.min_margin,
            l.all_frames_interp_gt_hold,
            l.acc.real_render_seconds,
            l.acc.generation_seconds,
            l.acc.real_render_fps(),
            l.acc.presented_fps(),
            l.acc.presented_frames(),
            l.digest,
        ));
        println!(
            "[g19_fg_probe] x{}: gen={} interp_mean={:.4} hold_mean={:.4} min_margin={:+.4} \
             real_fps={:.1} presented_fps={:.1} all_gt_hold={}",
            l.mode_x,
            l.generated,
            l.ssim_interp_mean,
            l.ssim_hold_mean,
            l.min_margin,
            l.acc.real_render_fps(),
            l.acc.presented_fps(),
            l.all_frames_interp_gt_hold,
        );
    }

    let payload = format!(
        "{{\"schema_version\":1,\"subject\":\"g19_frame_gen_probe\",\
         \"resolution\":[{W},{H}],\"full_rate_frames\":{FULL_RATE_FRAMES},\
         \"lanes\":[{lanes_json}],\
         \"all_lanes_quality_pass\":{all_quality},\
         \"double_run_bitexact\":{double_run_bitexact},\
         \"real_fps_caliber_invariant\":{caliber_ok},\
         \"notes\":\"FG/MFG host 参考臂 probe：程序产对照阈 = 逐帧 SSIM(interp)>SSIM(frame-hold)；\
真实渲染帧率口径 0-byte（生成帧禁入），presented 口径独立登记；解析式 GT 全帧率序列\"}}"
    );
    if let Some(parent) = std::path::Path::new(&out_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out_path, payload + "\n").expect("写 evidence 失败");
    println!("[g19_fg_probe] evidence → {out_path}");

    if !(all_quality && double_run_bitexact && caliber_ok) {
        eprintln!(
            "[g19_fg_probe] FAIL quality={all_quality} bitexact={double_run_bitexact} caliber={caliber_ok}"
        );
        std::process::exit(1);
    }
    println!("[g19_fg_probe] PASS");
}
