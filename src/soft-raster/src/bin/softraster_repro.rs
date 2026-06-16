//! softraster_repro — G0 软光栅确定性帧像素离线复现冒烟驱动(M7.3,spec/softraster.md
//! RXS-0118~0121)。
//!
//! 用法:`softraster_repro <out_dir>`
//!   把**固定输入**场景经软光栅管线(binning → tile 光栅 → 深度 → tonemap)渲染为
//!   确定性帧序列,经 image-io PPM P6 确定编码落盘到 `<out_dir>`,逐行打印
//!   `<frame_file> <byte_len>`(stdout)。ci/soft_raster_smoke.py 在两个目录各跑一次,
//!   对落盘字节计算 SHA-256 核对逐帧逐字节一致(固定输入 → 逐字节确定帧像素)。
//!
//! 全 safe(unsafe_code=deny);无随机量 / 时间戳 / 平台相关字节,纯确定性。

use image_io::{ImageFormat, ImageSequence};
use soft_raster::render_sequence;
use std::path::PathBuf;
use std::process::ExitCode;

/// 固定帧数(确定性序列;跨运行逐字节一致)。
const FRAMES: u32 = 6;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(out_dir) = args.get(1) else {
        eprintln!("用法: softraster_repro <out_dir>");
        return ExitCode::from(2);
    };
    let dir = PathBuf::from(out_dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("softraster_repro: 无法创建目录 {out_dir}: {e}");
        return ExitCode::from(1);
    }

    let frames = render_sequence(FRAMES);
    let mut seq = ImageSequence::new(&dir);
    for frame in &frames {
        match seq.push_frame(frame, ImageFormat::Ppm) {
            Ok(rec) => println!("{} {}", rec.file_name, rec.byte_len),
            Err(e) => {
                eprintln!("softraster_repro: 落盘失败: {e}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}
