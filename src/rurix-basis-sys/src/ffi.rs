//! 手写 `extern "C"` 声明 + POD 镜像(U45)。
//! 签名对齐 `ffi/rurix_basis_wrap.h`(真实 basis_universal 1.16.4 包装)。

#![allow(non_camel_case_types)]

use std::os::raw::c_char;

#[repr(C)]
pub struct RurixBasisBuf {
    pub data: *mut u8,
    pub len: usize,
}

/// 容器编码模式(== wrap 头 `RURIX_BASIS_MODE_*` 字面)。
pub const MODE_UASTC_KTX2: i32 = 0;
pub const MODE_ETC1S_BASIS: i32 = 1;

/// transcode 源容器种类(== wrap 头 `RURIX_BASIS_SRC_*` 字面)。
pub const SRC_BASIS: i32 = 0;
pub const SRC_KTX2: i32 = 1;

/// transcode 目标(== 上游 `basist::transcoder_texture_format` 字面)。
pub const TF_BC4_R: i32 = 4;
pub const TF_BC5_RG: i32 = 5;
pub const TF_BC7_RGBA: i32 = 6;
pub const TF_ASTC_4X4: i32 = 10;

unsafe extern "C" {
    pub fn rurix_basis_version() -> *const c_char;
    pub fn rurix_basis_buf_free(buf: *mut RurixBasisBuf);
    pub fn rurix_basis_encode_container(
        rgba: *const u8,
        width: u32,
        height: u32,
        mode: i32,
        swizzle_rg: i32,
        out: *mut RurixBasisBuf,
    ) -> i32;
    // `rurix_basis_transcode`(level 0 旧入口)C 符号在 wrap 侧保留(C 消费面,
    // 见 VENDOR.md §3);Rust 侧统一经 `rurix_basis_transcode_level`(level=0 同义),
    // 故此处不再重复声明(避免死声明面)。
    pub fn rurix_basis_transcode_level(
        data: *const u8,
        len: usize,
        src_kind: i32,
        target: i32,
        level: u32,
        out: *mut RurixBasisBuf,
        out_width: *mut u32,
        out_height: *mut u32,
    ) -> i32;
}

#[cfg(test)]
mod layout {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn ffi_layout_anchors() {
        assert_eq!(
            size_of::<RurixBasisBuf>(),
            size_of::<*mut u8>() + size_of::<usize>()
        );
        assert_eq!(align_of::<RurixBasisBuf>(), align_of::<*mut u8>());
    }
}
