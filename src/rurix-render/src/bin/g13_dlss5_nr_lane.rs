//! DLSS 5 NR 后置增强 lane harness(artifacts/day_0830_dlss5nr Phase 2):
//! `tsr→nr` / `dlss_sr→nr` 两条链——上采样器(自研 TSR / DLSS SR Vulkan interop)
//! 产出目标分辨率 color,再喂 [`NrDx12Session`] 做 NR 神经增强(in==out)。
//!
//! **加性纪律**:本 bin 对冻结的 g14_3_pipeline_perf / g14_3_lane_body 车道**字面
//! 零改动**(独立 harness),经 rurix-render 公共上采样面 + rurix-rt vendor_upscale
//! 公共会话面串链;NR 特性硬件限定 Blackwell,本机 RTX 4070 Ti(Ada)NR 段
//! create() fail-closed(上采样段真跑不受影响,如实登记每段)。
//!
//! 用法:
//!   g13_dlss5_nr_lane [--out <path>] [--chain tsr|dlss_sr|both]
//!   RURIX_REQUIRE_REAL=1  # 任一链 NR 段不可用 → 退 1(硬红)
//!
//! 数字纪律:上采样段真跑真出帧;NR 段真跑真裁决(fail-closed 非 mock)。

use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::tsr::{TsrParams, TsrUpscaler};
use rurix_render::temporal::upscale::{UpscaleBackend, UpscaleInputs};
use rurix_rt::vendor_upscale::{
    DlssVkSession, NrDx12Session, VendorFrameInput, dlss5nr_sdk_dir, streamline_sdk_dir,
};

const IN_W: u32 = 960;
const IN_H: u32 = 540;
const OUT_W: u32 = 1920;
const OUT_H: u32 = 1080;

const REPORT_SCHEMA: &str = "rurix.dlss5nr.lane.v1";

/// 合成输入帧(确定性梯度;跨链同一事实源)。
fn render_input(w: u32, h: u32, jitter: [f32; 2]) -> ImageF32 {
    ImageF32::from_fn(w, h, 3, |x, y, ch| {
        let u = (x as f32 + jitter[0]) / w as f32;
        let v = (y as f32 + jitter[1]) / h as f32;
        match ch {
            0 => u,
            1 => v,
            _ => 0.5 * (u + v),
        }
    })
}

fn const_depth(w: u32, h: u32) -> ImageF32 {
    ImageF32::from_fn(w, h, 1, |_, _, _| 0.5)
}

fn zero_mv(w: u32, h: u32) -> ImageF32 {
    ImageF32::new(w, h, 2)
}

/// 廉价内容 digest(FNV-1a over f32 le bytes;lane 报告对账用,非 canonical 锚)。
fn fnv_digest(data: &[f32]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &f in data {
        for b in f.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

fn jesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

struct StageResult {
    ok: bool,
    detail: String,
    digest: String,
}

/// 上采样段:自研 TSR(CPU UpscaleBackend)暖 8 帧 → 目标分辨率 color。
fn upscale_tsr() -> Result<ImageF32, String> {
    let mut tsr = TsrUpscaler::new(TsrParams::default());
    let depth = const_depth(IN_W, IN_H);
    let mv = zero_mv(IN_W, IN_H);
    let mut last: Option<ImageF32> = None;
    for i in 0..8u32 {
        let cur = render_input(IN_W, IN_H, [0.0, 0.0]);
        let inp = UpscaleInputs {
            color: &cur,
            depth: &depth,
            mv: &mv,
            reactive: None,
            exposure: 1.0,
            jitter: [0.0, 0.0],
            output_size: (OUT_W, OUT_H),
            frame_index: i,
            reset: i == 0,
        };
        last = Some(tsr.upscale(&inp));
    }
    last.ok_or_else(|| "TSR 零帧".to_owned())
}

/// 上采样段:DLSS SR(Streamline 2.10.3 Vulkan interop)真建 session → 目标分辨率 color。
fn upscale_dlss() -> Result<ImageF32, String> {
    let dir = streamline_sdk_dir().map_err(|e| format!("streamline_sdk_dir: {e}"))?;
    let mut s = DlssVkSession::create(&dir, (IN_W, IN_H), (OUT_W, OUT_H), false)
        .map_err(|e| format!("DlssVkSession::create: {e}"))?;
    let color = render_input(IN_W, IN_H, [0.0, 0.0]);
    let depth = const_depth(IN_W, IN_H);
    let mv = zero_mv(IN_W, IN_H);
    let vi = VendorFrameInput {
        color: &color.data,
        depth: &depth.data,
        mv: &mv.data,
        reactive: None,
        exposure: 1.0,
        jitter: [0.0, 0.0],
        frame_index: 0,
        reset: true,
    };
    let out = s.upscale(&vi).map_err(|e| format!("DlssVkSession::upscale: {e}"))?;
    Ok(ImageF32 {
        w: OUT_W,
        h: OUT_H,
        c: 3,
        data: out,
    })
}

/// NR 段:NrDx12Session(in==out=目标分辨率)消费上采样 color → NR 增强 → digest。
/// 本机 Ada:create() fail-closed(NR 特性硬件限定 Blackwell)。
fn nr_stage(color: &ImageF32) -> StageResult {
    match NrDx12Session::create((OUT_W, OUT_H)) {
        Ok(mut nr) => {
            let rep = nr.report();
            let depth = vec![0.5f32; (OUT_W * OUT_H) as usize];
            let mv = vec![0.0f32; (OUT_W * OUT_H * 2) as usize];
            let vi = VendorFrameInput {
                color: &color.data,
                depth: &depth,
                mv: &mv,
                reactive: None,
                exposure: 1.0,
                jitter: [0.0, 0.0],
                frame_index: 0,
                reset: true,
            };
            let mut out = vec![0f32; (OUT_W * OUT_H * 3) as usize];
            match nr.evaluate(&vi, &mut out) {
                Ok(()) => StageResult {
                    ok: true,
                    detail: format!(
                        "NR evaluate OK({}x{};snippet={};gpu={})",
                        OUT_W, OUT_H, rep.engine_version, rep.gpu_name
                    ),
                    digest: fnv_digest(&out),
                },
                Err(e) => StageResult {
                    ok: false,
                    detail: format!("NR evaluate Err: {e}"),
                    digest: String::new(),
                },
            }
        }
        Err(e) => StageResult {
            ok: false,
            detail: format!("NrDx12Session::create Err: {e}"),
            digest: String::new(),
        },
    }
}

fn run_chain(chain: &str) -> (StageResult, StageResult) {
    let up = match chain {
        "tsr" => upscale_tsr(),
        "dlss_sr" => upscale_dlss(),
        _ => Err(format!("未知链 {chain}")),
    };
    match up {
        Ok(color) => {
            let up_res = StageResult {
                ok: true,
                detail: format!("{chain} 上采样 OK({IN_W}x{IN_H}→{OUT_W}x{OUT_H})"),
                digest: fnv_digest(&color.data),
            };
            let nr_res = nr_stage(&color);
            (up_res, nr_res)
        }
        Err(e) => (
            StageResult {
                ok: false,
                detail: format!("{chain} 上采样 Err: {e}"),
                digest: String::new(),
            },
            StageResult {
                ok: false,
                detail: "上采样段失败,NR 段跳过".to_owned(),
                digest: String::new(),
            },
        ),
    }
}

fn main() {
    let mut out_path: Option<String> = None;
    let mut chain_sel = "both".to_owned();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out_path = Some(args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("DLSS5NR_LANE: FAIL --out 缺路径");
                    std::process::exit(2)
                }));
            }
            "--chain" => {
                i += 1;
                chain_sel = args.get(i).cloned().unwrap_or_else(|| {
                    eprintln!("DLSS5NR_LANE: FAIL --chain 缺值");
                    std::process::exit(2)
                });
            }
            other => {
                eprintln!("DLSS5NR_LANE: FAIL 未知参数 {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
    // snippet 在树前置校验(缺 → 环境降级如实登记)。
    let snippet_ok = dlss5nr_sdk_dir().is_ok();

    let chains: Vec<&str> = match chain_sel.as_str() {
        "tsr" => vec!["tsr"],
        "dlss_sr" => vec!["dlss_sr"],
        "both" => vec!["tsr", "dlss_sr"],
        other => {
            eprintln!("DLSS5NR_LANE: FAIL --chain 闭集 [tsr|dlss_sr|both],得 {other}");
            std::process::exit(2);
        }
    };

    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"schema\": \"{REPORT_SCHEMA}\",\n"));
    s.push_str(&format!("  \"snippet_present\": {snippet_ok},\n"));
    s.push_str("  \"chains\": [\n");
    let mut any_nr_ok = false;
    for (k, chain) in chains.iter().enumerate() {
        let (up, nr) = run_chain(chain);
        any_nr_ok |= nr.ok;
        s.push_str(&format!(
            "    {{\"chain\": \"{}→nr\", \"upscale_stage\": {{\"ok\": {}, \"detail\": \"{}\", \"digest\": \"{}\"}}, \"nr_stage\": {{\"ok\": {}, \"detail\": \"{}\", \"digest\": \"{}\"}}}}{}\n",
            chain,
            up.ok,
            jesc(&up.detail),
            up.digest,
            nr.ok,
            jesc(&nr.detail),
            nr.digest,
            if k + 1 < chains.len() { "," } else { "" }
        ));
    }
    s.push_str("  ],\n");
    let verdict = if any_nr_ok {
        "nr_available"
    } else {
        "nr_unavailable_hw_gated_blackwell"
    };
    s.push_str(&format!("  \"verdict\": \"{verdict}\"\n"));
    s.push_str("}\n");

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(&p, &s) {
                eprintln!("DLSS5NR_LANE: FAIL 落盘 {p}: {e}");
                std::process::exit(1);
            }
            println!("DLSS5NR_LANE: verdict={verdict} chains={} out={p}", chains.len());
        }
        None => print!("{s}"),
    }
    if require_real && !any_nr_ok {
        eprintln!("DLSS5NR_LANE: FAIL RURIX_REQUIRE_REAL=1 但 NR 段全链不可用(硬件限定 Blackwell)");
        std::process::exit(1);
    }
}
