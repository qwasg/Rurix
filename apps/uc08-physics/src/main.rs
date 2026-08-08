//! uc08-physics — 物理×渲染合流 demo(G6.3,RFC-0017 §4.B;工程形态照 uc06)。
//!
//! host 全管线(默认 feature,零 GPU 依赖):Jolt 刚体场景 → PhysicsBridge 单向
//! 同步 GpuScene(变换单向 physics→GpuScene,渲染不回写物理)→ 动态体 MV 供
//! 时域底座(静态/睡眠零 MV)→ TLAS 增量标脏 + rebuild_if_dirty(刚体 BLAS
//! Static 零 refit)→ G5 十五 pass(host 参考执行:流送 → 物理步 → 同步桥 →
//! 两级剔除 → VisBuffer → classify/resolve → GBuffer → MV → VSM → GI →
//! RTAO+硬阴影 → 延迟着色 → TAA → TSR)。流送剧本:远场景页初始不请求,
//! K 帧起提交,驻留沿批插 body;M 帧剧本化卸载——先卸 body 凭 RemovalReceipt
//! 再放页(编译期凭证:release 按值消耗 receipt,无 receipt 不可放页)。
//!
//! device 腿(feature `vulkan` + `--device`):经 rurix-rt `render_exec` 真 draw
//! 对拍「物理驱动变换到达 device」(P 步物理+sync → draw;Q 步+sync → 再 draw;
//! 两帧 readback 像素差异非平凡)。RURIX_REQUIRE_REAL=1 时任何 device 失败硬红;
//! 仅「Vulkan loader 缺失」且非 REQUIRE_REAL 才可 device:null 降级退 0;
//! 对拍/断言失败永远硬红。
//!
//! CLI:`uc08-physics [--frames N=96] [--size WxH=128x72] [--device] [--json]`
//! G8.8a soak:`--soak --min-seconds S --min-frames F`(双阈值同时满足;全程真实帧循环,
//! 禁 sleep 充墙钟,subject=host-soak 无 device 零错字面量,RSS 未门禁)。
//! `--json` 输出单行 JSON(smoke 脚本消费,字段集冻结);exit 0 仅当全部断言过;
//! exit 1 = 断言红/运行错;exit 2 = CLI 错或无 vulkan feature 传 --device。

#[cfg(feature = "vulkan")]
mod device;
mod graph_setup;
mod pipeline;
mod scene;
mod shading;

use std::time::{Duration, Instant};

use pipeline::{RenderConfig, Uc08Summary, run_frame};

/// CLI 参数(解析确定性;未知参数 = Err)。
#[derive(Debug, Clone)]
struct Cli {
    frames: u32,
    width: u32,
    height: u32,
    device: bool,
    json: bool,
    soak: bool,
    min_seconds: u64,
    min_frames: u32,
}

impl Default for Cli {
    fn default() -> Self {
        Cli {
            frames: 96,
            width: 128,
            height: 72,
            device: false,
            json: false,
            soak: false,
            min_seconds: 1800,
            min_frames: 10000,
        }
    }
}

fn parse_cli(args: &[String]) -> Result<Cli, String> {
    let mut c = Cli::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--frames" => {
                i += 1;
                c.frames = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .filter(|&n: &u32| n >= 1)
                    .ok_or("--frames 需要 ≥1 整数")?;
            }
            "--size" => {
                i += 1;
                let s = args.get(i).ok_or("--size 需要 WxH")?;
                let (w, h) = s.split_once('x').ok_or("--size 形如 128x72")?;
                c.width = w.parse().map_err(|_| "--size 宽非整数")?;
                c.height = h.parse().map_err(|_| "--size 高非整数")?;
                if c.width == 0 || c.height == 0 || c.width > 4096 || c.height > 4096 {
                    return Err("--size 越界(1..=4096)".to_owned());
                }
            }
            "--device" => c.device = true,
            "--json" => c.json = true,
            "--soak" => c.soak = true,
            "--min-seconds" => {
                i += 1;
                c.min_seconds = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .filter(|&n: &u64| n >= 1)
                    .ok_or("--min-seconds 需要 ≥1")?;
            }
            "--min-frames" => {
                i += 1;
                c.min_frames = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .filter(|&n: &u32| n >= 1)
                    .ok_or("--min-frames 需要 ≥1")?;
            }
            other => return Err(format!("未知参数 {other}")),
        }
        i += 1;
    }
    Ok(c)
}

/// 主流程(host 全管线;断言失败 → Err 串,exit 非零)。
fn run(cli: &Cli) -> Result<Uc08Summary, String> {
    let cfg = RenderConfig {
        out_w: cli.width,
        out_h: cli.height,
        frames: cli.frames,
        ..Default::default()
    };

    let scene = scene::build_scene();
    let mut st = pipeline::PipelineState::new(&scene, &cfg);

    let mut reports = Vec::with_capacity(cli.frames as usize);
    for frame in 0..cli.frames {
        let r = run_frame(&scene, &mut st, &cfg, frame);
        reports.push(r);
    }

    let mut summary = pipeline::assemble_summary(&st, &reports, cli.device)?;
    summary.width = cli.width;
    summary.height = cli.height;
    summary.internal_width = cfg.internal_w();
    summary.internal_height = cfg.internal_h();
    Ok(summary)
}

/// G8.8a soak(诚实语义,见 milestones/g8/G8_8A_SOAK_SEMANTICS_NOTES.md):
/// 双阈值(frames ≥ min_frames 且墙钟 ≥ min_seconds)全部由真实帧循环产出——
/// 帧数不够补真实帧、墙钟不够继续跑真实帧,本函数无任何 sleep(sleep_seconds 恒 0,
/// 构造保证)。host soak 无 Vulkan validation/device-lost 面:不输出字面量 0 充
/// device 零错门,改标 soak_subject="host-soak";RSS 未门禁(Windows 无采样器,
/// notes 声明,不再报恒 0 假采样)。
fn run_soak(cli: &Cli) -> Result<String, String> {
    let t0 = Instant::now();
    let target_frames = cli.min_frames.max(1);
    let need = Duration::from_secs(cli.min_seconds);
    let cfg = RenderConfig {
        out_w: cli.width,
        out_h: cli.height,
        frames: target_frames,
        ..Default::default()
    };
    let scene = scene::build_scene();
    let mut st = pipeline::PipelineState::new(&scene, &cfg);
    let mut frames: u64 = 0;
    loop {
        // 帧索引按 target_frames 回绕:超 min_frames 的补帧仍是每帧真实物理步+
        // 全管线,只是剧本事件(K/M 阈值)按周期重演。
        let frame_idx = (frames % u64::from(target_frames)) as u32;
        let _ = run_frame(&scene, &mut st, &cfg, frame_idx);
        frames += 1;
        if frames >= u64::from(target_frames) && t0.elapsed() >= need {
            break;
        }
    }
    let seconds = t0.elapsed().as_secs_f64();
    Ok(format!(
        "{{\"ok\":true,\"soak\":true,\"soak_subject\":\"host-soak\",\"soak_frames\":{frames},\"frames\":{frames},\"soak_seconds\":{seconds:.3},\"active_frame_seconds\":{seconds:.3},\"sleep_seconds\":0.0,\"min_frames\":{},\"min_seconds\":{}}}",
        cli.min_frames, cli.min_seconds
    ))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match parse_cli(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("uc08-physics: {e}");
            std::process::exit(2);
        }
    };

    #[cfg(not(feature = "vulkan"))]
    if cli.device {
        eprintln!(
            "uc08-physics: --device 需要 feature vulkan(cargo run -p uc08-physics --features vulkan)"
        );
        std::process::exit(2);
    }

    if cli.soak {
        match run_soak(&cli) {
            Ok(json) => {
                println!("{json}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("uc08-physics soak: {e}");
                std::process::exit(1);
            }
        }
    }

    match run(&cli) {
        Ok(summary) => {
            // device 腿(feature + flag 双开)。
            #[cfg(feature = "vulkan")]
            let device_json = if cli.device {
                match device::run_device_leg() {
                    Ok(j) => Some(j),
                    Err(e) => {
                        let require_real =
                            std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
                        let environment_missing = e.contains("Vulkan loader")
                            || e.contains("no-vulkan")
                            || e.contains("vulkan loader");
                        if require_real || !environment_missing {
                            eprintln!(
                                "uc08-physics: device 腿失败(回归硬红;仅 loader 缺失可降级): {e}"
                            );
                            std::process::exit(1);
                        }
                        eprintln!("uc08-physics: device 腿降级(dev-env degrade,不充绿): {e}");
                        None
                    }
                }
            } else {
                None
            };
            #[cfg(not(feature = "vulkan"))]
            let device_json: Option<pipeline::DeviceLeg> = None;

            let json = pipeline::summary_json(&summary, device_json.as_ref(), cli.device);
            if cli.json {
                println!("{json}");
            } else {
                println!("uc08-physics OK: {}", summary.one_line());
                println!("{json}");
            }
            if !summary.all_asserts_pass(device_json.as_ref()) {
                eprintln!("uc08-physics: 断言未全过(见 JSON asserts)");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("uc08-physics: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parse_defaults_and_overrides() {
        let c = parse_cli(&[]).unwrap();
        assert_eq!((c.frames, c.width, c.height), (96, 128, 72));
        assert!(!c.device && !c.json);
        let c = parse_cli(&[
            "--frames".into(),
            "4".into(),
            "--size".into(),
            "64x64".into(),
            "--json".into(),
        ])
        .unwrap();
        assert_eq!((c.frames, c.width, c.height), (4, 64, 64));
        assert!(c.json);
        let c = parse_cli(&["--device".into()]).unwrap();
        assert!(c.device);
    }

    #[test]
    fn cli_parse_rejects_bad_input() {
        assert!(parse_cli(&["--frames".into(), "0".into()]).is_err());
        assert!(parse_cli(&["--size".into(), "0x10".into()]).is_err());
        assert!(parse_cli(&["--size".into(), "10".into()]).is_err());
        assert!(parse_cli(&["--bogus".into()]).is_err());
        assert!(parse_cli(&["--frames".into()]).is_err());
    }

    #[test]
    fn soak_reports_honest_fields_no_sleep_no_device_literals() {
        // 小阈值 soak:输出必须带 honesty 字段,且无 sleep/无 device 字面量 0/无假 RSS。
        let cli = parse_cli(&[
            "--soak".into(),
            "--min-seconds".into(),
            "1".into(),
            "--min-frames".into(),
            "2".into(),
        ])
        .unwrap();
        let json = run_soak(&cli).unwrap();
        assert!(json.contains("\"soak_subject\":\"host-soak\""), "{json}");
        assert!(json.contains("\"sleep_seconds\":0.0"), "{json}");
        assert!(json.contains("\"active_frame_seconds\":"), "{json}");
        assert!(!json.contains("validation_messages"), "{json}");
        assert!(!json.contains("device_lost_count"), "{json}");
        assert!(!json.contains("rss"), "{json}");
        // 双阈值真实满足:墙钟 ≥1s 由帧循环产出(无 sleep,故帧数随时间自然增长)。
        assert!(json.contains("\"min_seconds\":1"), "{json}");
    }
}
