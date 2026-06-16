//! imgio_repro — image-io 确定性离线复现冒烟驱动(M7.2,spec/imageio.md RXS-0117)。
//!
//! 用法:`imgio_repro <out_dir>`
//!   把**固定输入**帧序列经 PPM P6 确定编码落盘到 `<out_dir>`,逐行打印
//!   `<frame_file> <byte_len>`(stdout)。ci/image_io_smoke.py 在两个目录各跑一次,
//!   对落盘字节计算 SHA-256 核对逐帧逐字节一致(固定输入 → 逐字节确定字节流)。
//!
//! 全 safe(unsafe_code=deny);无随机量 / 时间戳 / 平台相关字节,纯确定性。

use image_io::{ImageBuffer, ImageFormat, ImageSequence, Rgb};
use std::path::PathBuf;
use std::process::ExitCode;

/// 固定输入帧序列(确定性):`frames` 帧,每帧 `w × h` 渐变。
fn build_fixed_frames(frames: u32, w: u32, h: u32) -> Vec<ImageBuffer<Rgb>> {
    let mut out = Vec::new();
    for f in 0..frames {
        let mut buf = ImageBuffer::new(w, h, Rgb::new(0.0, 0.0, 0.0));
        for y in 0..h {
            for x in 0..w {
                let r = (x as f32) / ((w.max(2) - 1) as f32);
                let g = (y as f32) / ((h.max(2) - 1) as f32);
                let b = (f as f32) / ((frames.max(2) - 1) as f32);
                buf.set(x, y, Rgb::new(r, g, b));
            }
        }
        out.push(buf);
    }
    out
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(out_dir) = args.get(1) else {
        eprintln!("用法: imgio_repro <out_dir>");
        return ExitCode::from(2);
    };
    let dir = PathBuf::from(out_dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("imgio_repro: 无法创建目录 {out_dir}: {e}");
        return ExitCode::from(1);
    }

    // 固定输入:8 帧 16x12 渐变(确定性,跨运行逐字节一致)。
    let frames = build_fixed_frames(8, 16, 12);
    let mut seq = ImageSequence::new(&dir);
    for frame in &frames {
        match seq.push_frame(frame, ImageFormat::Ppm) {
            Ok(rec) => println!("{} {}", rec.file_name, rec.byte_len),
            Err(e) => {
                eprintln!("imgio_repro: 落盘失败: {e}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}
