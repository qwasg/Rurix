//! uc03_demo — UC-03 旗舰验收 demo 单 EXE 入口(M7.4,契约 G-M7-1;01 §6 旗舰用例)。
//!
//! 用法:`uc03_demo <out_dir>`
//!   从**固定初值**起跑确定性 SPH 仿真,逐帧经 G0 软光栅管线(binning → tile 光栅 →
//!   深度 → tonemap,RXS-0118~0121)渲染,经 image-io PPM P6 确定编码(RXS-0114~0117)
//!   落盘为图像序列到 `<out_dir>`,逐行打印 `<frame_file> <byte_len>`(stdout)。
//!   ci/uc03_demo_smoke.py 在两个目录各跑一次,对落盘字节计算 content SHA-256 核对逐帧
//!   逐字节一致(固定输入/种子两次运行 → 确定性图像序列,G-M7-1)。
//!
//! 全 safe(`unsafe_code=deny`);无随机量 / 时间戳 / 平台相关字节,纯确定性。

use image_io::{ImageFormat, ImageSequence};
use std::path::PathBuf;
use std::process::ExitCode;
use uc03_demo::render_sequence;

/// 固定帧数(确定性序列;跨运行逐字节一致)。
const FRAMES: u32 = 12;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    // G1.1 实时窗口呈现模式（RFC-0001 / RXS-0142~0143）：`uc03_demo --present`。
    // 默认离屏 PPM 序列路径不变（向后兼容,G-M7-1）。
    if args.iter().any(|a| a == "--present") {
        return run_present_mode();
    }
    let Some(out_dir) = args.get(1) else {
        eprintln!("用法: uc03_demo <out_dir>            # 离屏 PPM 图像序列（默认）");
        eprintln!(
            "      uc03_demo --present            # 实时窗口呈现（需 --features d3d12-present-real，G1.1）"
        );
        return ExitCode::from(2);
    };
    let dir = PathBuf::from(out_dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("uc03_demo: 无法创建目录 {out_dir}: {e}");
        return ExitCode::from(1);
    }

    let frames = render_sequence(FRAMES);
    let mut seq = ImageSequence::new(&dir);
    for frame in &frames {
        match seq.push_frame(frame, ImageFormat::Ppm) {
            Ok(rec) => println!("{} {}", rec.file_name, rec.byte_len),
            Err(e) => {
                eprintln!("uc03_demo: 落盘失败: {e}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}

/// 实时窗口呈现模式（feature `d3d12-present`）。无该 feature → 明确报错（不静默）。
#[cfg(feature = "d3d12-present")]
fn run_present_mode() -> ExitCode {
    use soft_raster::{HEIGHT, WIDTH};
    // render = 软光栅帧尺寸（G0 kernel 写共享 backbuffer）;window = 呈现窗口（nearest 放大）。
    match uc03_demo::present::run_present(0, [WIDTH, HEIGHT], [1024, 768]) {
        Ok(report) => {
            println!(
                "UC03_PRESENT: ok frames={} scene=uc03_sph lit_pixels={} first_checksum={:016x} animated={}",
                report.frames,
                report.first_frame_lit_pixels,
                report.first_frame_checksum,
                report.animation_changed
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            // stub shim / 无 GPU / 非交互桌面：interop 不可用（确定性错误,非崩溃）。
            eprintln!(
                "UC03_PRESENT: skip — interop 不可用（需 --features d3d12-present-real + 交互桌面 + GPU + Windows SDK D3D12）: {e:?}"
            );
            ExitCode::from(3)
        }
    }
}

#[cfg(not(feature = "d3d12-present"))]
fn run_present_mode() -> ExitCode {
    eprintln!(
        "uc03_demo --present 需 `--features d3d12-present`（或 d3d12-present-real 真跑）构建（G1.1 实时窗口呈现，RFC-0001 / RXS-0142~0143）"
    );
    ExitCode::from(2)
}
