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
    let Some(out_dir) = args.get(1) else {
        eprintln!("用法: uc03_demo <out_dir>");
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
