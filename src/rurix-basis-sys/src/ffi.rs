//! 手写 `extern "C"` 声明 + POD 镜像(U45)。
//! 签名对齐 `vendor/rurix_basis_shim/rurix_basis_shim.h`。

#![allow(non_camel_case_types)]

use std::os::raw::c_char;

#[repr(C)]
pub struct RurixBasisBuf {
    pub data: *mut u8,
    pub len: usize,
}

unsafe extern "C" {
    pub fn rurix_basis_version() -> *const c_char;
    pub fn rurix_basis_buf_free(buf: *mut RurixBasisBuf);
    pub fn rurix_basis_encode_bc7_rgba8(
        rgba: *const u8,
        width: u32,
        height: u32,
        out: *mut RurixBasisBuf,
    ) -> i32;
    pub fn rurix_basis_encode_bc1_rgba8(
        rgba: *const u8,
        width: u32,
        height: u32,
        out: *mut RurixBasisBuf,
    ) -> i32;
    pub fn rurix_basis_encode_astc4x4_rgba8(
        rgba: *const u8,
        width: u32,
        height: u32,
        out: *mut RurixBasisBuf,
    ) -> i32;
}

#[cfg(test)]
mod layout {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn ffi_layout_anchors() {
        assert_eq!(size_of::<RurixBasisBuf>(), size_of::<*mut u8>() + size_of::<usize>());
        assert_eq!(align_of::<RurixBasisBuf>(), align_of::<*mut u8>());
    }
}
