//! rurix-basis-sys — M83 纹理 codec FFI 边界(G8.3,RFC-0020 §4.8)。
//!
//! 过渡真实 codec(手写 BC1/BC7 + ASTC void-extent);完整 basis_universal 待合入
//! (VENDOR.md)。unsafe 集中地:`unsafe-audit/rurix-basis-sys.md` U44~U46。

mod ffi;

use std::ffi::CStr;
use std::fmt;
use std::ptr;

/// VENDOR.md / `rurix_basis_version()` 字面锚定(smoke `real_codec_identity`)。
pub const VENDOR_VERSION: &str = "rurix-basis-transitional/0.1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BasisError {
    EncodeFailed(i32),
    NullBuffer,
    InvalidDims,
}

impl fmt::Display for BasisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BasisError::EncodeFailed(c) => write!(f, "basis encode failed: {c}"),
            BasisError::NullBuffer => write!(f, "basis encoder returned null buffer"),
            BasisError::InvalidDims => write!(f, "width/height must be > 0"),
        }
    }
}

impl std::error::Error for BasisError {}

/// FFI 报告的 encoder 版本串(静态存储,无分配)。
pub fn version_string() -> &'static str {
    // SAFETY(U45):`rurix_basis_version` 返回指向静态只读 C 字符串的指针,
    // 生命周期 = 进程;非 null;UTF-8 字面由本 crate shim 保证。
    unsafe {
        let p = ffi::rurix_basis_version();
        debug_assert!(!p.is_null());
        CStr::from_ptr(p)
            .to_str()
            .unwrap_or("invalid-version")
    }
}

fn take_buf(mut buf: ffi::RurixBasisBuf) -> Result<Vec<u8>, BasisError> {
    if buf.data.is_null() {
        return Err(BasisError::NullBuffer);
    }
    // SAFETY(U46):`data` 指向 encoder 经 C++ `new[]` 分配的 `len` 字节,
    // 在 `rurix_basis_buf_free` 前可读;`len` 为有效字节数。
    // 跨分配器纪律:先拷贝进 Rust `Vec`,再由 shim `delete[]` 释放——
    // 禁止 `Vec::from_raw_parts` 接管 C++ 堆。
    let vec = unsafe { std::slice::from_raw_parts(buf.data, buf.len).to_vec() };
    // SAFETY(U46):配对 `new[]`/`delete[]`;此后不得再读 `data`。
    unsafe {
        ffi::rurix_basis_buf_free(&mut buf);
    }
    Ok(vec)
}

fn encode_with(
    rgba: &[u8],
    width: u32,
    height: u32,
    f: unsafe extern "C" fn(*const u8, u32, u32, *mut ffi::RurixBasisBuf) -> i32,
) -> Result<Vec<u8>, BasisError> {
    if width == 0 || height == 0 {
        return Err(BasisError::InvalidDims);
    }
    let need = (width as usize).saturating_mul(height as usize).saturating_mul(4);
    if rgba.len() < need {
        return Err(BasisError::InvalidDims);
    }
    let mut buf = ffi::RurixBasisBuf {
        data: ptr::null_mut(),
        len: 0,
    };
    // SAFETY(U44):`rgba` 在调用期间有效且长度 ≥ w*h*4;`out` 指向本栈 POD;
    // 失败时 shim 不写非 null data;成功后由 `take_buf` 接管所有权。
    let rc = unsafe { f(rgba.as_ptr(), width, height, &mut buf) };
    if rc != 0 {
        // SAFETY(U46):失败路径仍可能部分分配——交 free 兜底。
        unsafe {
            ffi::rurix_basis_buf_free(&mut buf);
        }
        return Err(BasisError::EncodeFailed(rc));
    }
    take_buf(buf)
}

/// RGBA8 → BC7_UNORM 块字节(确定性;非全零占位)。
pub fn encode_bc7_rgba8(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, BasisError> {
    encode_with(rgba, width, height, ffi::rurix_basis_encode_bc7_rgba8)
}

/// RGBA8 → BC1_RGB 块字节。
pub fn encode_bc1_rgba8(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, BasisError> {
    encode_with(rgba, width, height, ffi::rurix_basis_encode_bc1_rgba8)
}

/// RGBA8 → ASTC 4×4(void-extent 实块)。
pub fn encode_astc4x4_rgba8(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, BasisError> {
    encode_with(rgba, width, height, ffi::rurix_basis_encode_astc4x4_rgba8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            v.extend_from_slice(&rgba);
        }
        v
    }

    #[test]
    fn version_matches_vendor_pin() {
        assert_eq!(version_string(), VENDOR_VERSION);
    }

    #[test]
    fn bc7_not_all_zero_and_deterministic() {
        let rgba = solid(8, 8, [32, 64, 128, 255]);
        let a = encode_bc7_rgba8(&rgba, 8, 8).unwrap();
        let b = encode_bc7_rgba8(&rgba, 8, 8).unwrap();
        assert_eq!(a, b);
        assert!(!a.iter().all(|&x| x == 0), "BC7 must not be all-zero placeholder");
        assert_eq!(a.len(), 4 * 16); // 2x2 blocks * 16
    }

    #[test]
    fn astc_void_extent_nonzero() {
        let rgba = solid(4, 4, [200, 10, 10, 255]);
        let a = encode_astc4x4_rgba8(&rgba, 4, 4).unwrap();
        assert_eq!(a.len(), 16);
        assert_ne!(a, vec![0u8; 16]);
    }
}
