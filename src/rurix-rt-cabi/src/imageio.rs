//! rxio 宿主图像落盘桥(MS1.2b,RFC-0009 §4.7;spec/host_orchestration.md RXS-0199)。
//!
//! host `.rx` 的 `Image::write_ppm` 面降级为本模块 [`rxio_write_ppm`]:桥
//! `image-io` crate 既有确定性 PPM P6 序列化(RXS-0114~0117 语义 **0-byte 复用**——
//! header 规范化 / 行主序 / 通道序 R,G,B / `f32→u8` 量化口径对齐 `sr_quantize`,
//! RXS-0116),同一输入逐字节一致。补齐离线渲染「出图落盘」的 `.rx` 面(UC-07
//! 离线入口零 `.rs`,G5 缺口)。
//!
//! 失败语义(RXS-0193 / RFC-0009 §4.5):校验(null / 非 UTF-8 / 零尺寸 /
//! `n != w*h*3`)与写盘失败一律 stderr 确定性诊断 `RXRT: error op=...` + 负值,
//! 是否终止由调用方(编译器注入检查 → [`crate::rxrt_trap`])裁决;host-only 纯
//! 序列化,不触 GPU、不涉 poisoned。

use std::ffi::CStr;

use image_io::{ImageBuffer, ImageFormat, Rgb, encode};

use crate::{RXRT_FAIL, diag};

/// C ABI:确定性 PPM 落盘(RFC-0009 §4.7)。`path` = NUL 终止 UTF-8 路径;`data` =
/// 行主序 RGB 三元组 `f32` 数组(上→下、左→右,通道序 R,G,B——对齐共享 f32 RGB
/// 布局,RXS-0143/RXS-0121);`n` 须精确等于 `w*h*3`(不匹配 = 触盘前确定性拒绝)。
/// `0` 成功;失败 → 诊断 + 负值。
//@ spec: RXS-0199
#[allow(clippy::not_unsafe_ptr_arg_deref)] // C ABI 入口:指针契约由调用方 codegen 保证(U25)
#[unsafe(no_mangle)]
pub extern "C" fn rxio_write_ppm(path: *const u8, w: u32, h: u32, data: *const f32, n: u64) -> i32 {
    const OP: &str = "rxio_write_ppm";
    if path.is_null() {
        diag(OP, "null path pointer");
        return RXRT_FAIL;
    }
    // SAFETY: (U25):`path` 非 null(上方已检),调用方(codegen 发射的 write_ppm 调用)
    // 保证其为 NUL 终止字符串且调用期存活(RFC-0009 §4.3 指针契约);借用不越出本函数。
    let path = unsafe { CStr::from_ptr(path.cast()) };
    let Ok(path) = path.to_str() else {
        diag(OP, "path is not valid UTF-8");
        return RXRT_FAIL;
    };
    if w == 0 || h == 0 {
        diag(OP, format!("zero-dimension image {w}x{h}"));
        return RXRT_FAIL;
    }
    // n == w*h*3 精确匹配(checked:w*h*3 溢出 u64 时必不等,确定性拒绝)。
    let expected = (u64::from(w))
        .checked_mul(u64::from(h))
        .and_then(|px| px.checked_mul(3));
    if expected != Some(n) {
        diag(
            OP,
            format!(
                "length mismatch: expected w*h*3 = {}x{}x3 elements, got {n}",
                w, h
            ),
        );
        return RXRT_FAIL;
    }
    if data.is_null() {
        diag(OP, "null data pointer");
        return RXRT_FAIL;
    }
    // SAFETY: (U25):`data` 非 null(上方已检),调用方保证其指向 `n` 个 `f32` 有效
    // 可读主机内存且调用期存活(`n` 已与 w*h*3 精确核对);借用不越出本函数。
    let data = unsafe { core::slice::from_raw_parts(data, n as usize) };

    // 行主序 RGB 三元组 → ImageBuffer<Rgb>(量化/编码全权委托 image-io,RXS-0116)。
    let mut buf = ImageBuffer::new(w, h, Rgb::new(0.0, 0.0, 0.0));
    for (i, px) in data.chunks_exact(3).enumerate() {
        let x = (i % w as usize) as u32;
        let y = (i / w as usize) as u32;
        buf.set(x, y, Rgb::new(px[0], px[1], px[2]));
    }
    let bytes = match encode(&buf, ImageFormat::Ppm) {
        Ok(bytes) => bytes,
        Err(e) => {
            diag(OP, e);
            return RXRT_FAIL;
        }
    };
    if let Err(e) = std::fs::write(path, &bytes) {
        diag(OP, format!("write '{path}' failed: {e}"));
        return RXRT_FAIL;
    }
    0
}

// -- tests ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    //@ spec: RXS-0199
    #[test]
    fn write_ppm_bytes_match_image_io_exactly() {
        let dir = std::env::temp_dir().join(format!(
            "rurix_cabi_imgio_{}_{}",
            std::process::id(),
            "bridge"
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rxio_bridge.ppm");
        // 2x2 RGB(含钳制/NaN 量化边界样本,RXS-0116 口径对齐)。
        let data: [f32; 12] = [
            0.0,
            0.5,
            1.0,
            -1.0,
            2.0,
            f32::NAN,
            0.25,
            0.75,
            0.125,
            1.0,
            0.0,
            0.5,
        ];
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(
            rxio_write_ppm(cpath.as_ptr().cast(), 2, 2, data.as_ptr(), 12),
            0,
            "write_ppm 成功"
        );
        let on_disk = std::fs::read(&path).unwrap();

        // image-io 直调对照:同输入逐字节一致(RXS-0114~0117 语义 0-byte 复用)。
        let mut buf = ImageBuffer::new(2, 2, Rgb::new(0.0, 0.0, 0.0));
        for y in 0..2u32 {
            for x in 0..2u32 {
                let i = ((y * 2 + x) * 3) as usize;
                buf.set(x, y, Rgb::new(data[i], data[i + 1], data[i + 2]));
            }
        }
        let direct = encode(&buf, ImageFormat::Ppm).unwrap();
        assert_eq!(on_disk, direct, "rxio_write_ppm 与 image-io 直调逐字节一致");

        // 两次落盘逐字节一致(确定性,RXS-0116)。
        let path2 = dir.join("rxio_bridge_again.ppm");
        let cpath2 = CString::new(path2.to_str().unwrap()).unwrap();
        assert_eq!(
            rxio_write_ppm(cpath2.as_ptr().cast(), 2, 2, data.as_ptr(), 12),
            0
        );
        assert_eq!(std::fs::read(&path2).unwrap(), on_disk);

        let _ = std::fs::remove_dir_all(&dir);
    }

    //@ spec: RXS-0193, RXS-0199
    #[test]
    fn write_ppm_rejects_malformed_deterministically() {
        let data = [0.0f32; 3];
        // null path。
        assert!(rxio_write_ppm(core::ptr::null(), 1, 1, data.as_ptr(), 3) < 0);
        // 以下形态均在触盘前确定性拒绝(路径不落盘)。
        let cpath = CString::new("rurix_rxio_never_written.ppm").unwrap();
        // n != w*h*3。
        assert!(rxio_write_ppm(cpath.as_ptr().cast(), 1, 1, data.as_ptr(), 4) < 0);
        assert!(rxio_write_ppm(cpath.as_ptr().cast(), 2, 1, data.as_ptr(), 3) < 0);
        // w*h*3 溢出 u64 → 必不等,确定性拒绝(checked_mul)。
        assert!(rxio_write_ppm(cpath.as_ptr().cast(), u32::MAX, u32::MAX, data.as_ptr(), 3) < 0);
        // 零尺寸。
        assert!(rxio_write_ppm(cpath.as_ptr().cast(), 0, 1, data.as_ptr(), 0) < 0);
        assert!(rxio_write_ppm(cpath.as_ptr().cast(), 1, 0, data.as_ptr(), 0) < 0);
        // null data(n > 0)。
        assert!(rxio_write_ppm(cpath.as_ptr().cast(), 1, 1, core::ptr::null(), 3) < 0);
        assert!(
            !std::path::Path::new("rurix_rxio_never_written.ppm").exists(),
            "校验拒绝不落盘"
        );
        // 写入失败(父目录不存在)→ 诊断 + 负值,不 panic。
        let bad = std::env::temp_dir()
            .join(format!("rurix_no_such_dir_{}", std::process::id()))
            .join("out.ppm");
        let cbad = CString::new(bad.to_str().unwrap()).unwrap();
        assert!(rxio_write_ppm(cbad.as_ptr().cast(), 1, 1, data.as_ptr(), 3) < 0);
    }
}
