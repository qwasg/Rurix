//! rurix-basis-sys — M83 纹理 codec FFI 边界(G8.3,RFC-0020 §4.8)。
//!
//! 真实 `basis_universal`(BinomialLLC 1.16.4)encoder + transcoder,经 `cc` 编
//! 显式 .cpp 清单(禁 cmake / 禁 zstd supercompression;见 VENDOR.md)。
//! 能力面:UASTC→KTX2、ETC1S→真实 `.basis`、容器→BCn/ASTC 真 transcode。
//! unsafe 集中地:`unsafe-audit/rurix-basis-sys.md` U44~U46。

mod ffi;

use std::ffi::CStr;
use std::fmt;
use std::ptr;

/// VENDOR.md / `rurix_basis_version()` 字面锚定(smoke `real_codec_identity`)。
///
/// 真实上游 pin:tag 1.16.4 @ commit 900e40fb5d25(见 `vendor/basis_universal/
/// vendor_manifest.json`)。过渡串 `rurix-basis-transitional/*` 已废除。
pub const VENDOR_VERSION: &str = "basis_universal/1.16.4+g900e40fb5d25";

/// 上游 pin(VENDOR.md / SBOM.md 复核锚)。
pub const UPSTREAM_TAG: &str = "1.16.4";
pub const UPSTREAM_COMMIT: &str = "900e40fb5d2502927360fe2f31762bdbb624455f";

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
    // SAFETY: (U45)`rurix_basis_version` 返回指向静态只读 C 字符串的指针,
    // 生命周期 = 进程;非 null;UTF-8 字面由本 crate shim 保证。
    unsafe {
        let p = ffi::rurix_basis_version();
        debug_assert!(!p.is_null());
        CStr::from_ptr(p).to_str().unwrap_or("invalid-version")
    }
}

fn take_buf(mut buf: ffi::RurixBasisBuf) -> Result<Vec<u8>, BasisError> {
    if buf.data.is_null() {
        return Err(BasisError::NullBuffer);
    }
    // SAFETY: (U46)`data` 指向 encoder 经 C++ `new[]` 分配的 `len` 字节,
    // 在 `rurix_basis_buf_free` 前可读;`len` 为有效字节数。
    // 跨分配器纪律:先拷贝进 Rust `Vec`,再由 shim `delete[]` 释放——
    // 禁止 `Vec::from_raw_parts` 接管 C++ 堆。
    let vec = unsafe { std::slice::from_raw_parts(buf.data, buf.len).to_vec() };
    // SAFETY: (U46)配对 `new[]`/`delete[]`;此后不得再读 `data`。
    unsafe {
        ffi::rurix_basis_buf_free(&mut buf);
    }
    Ok(vec)
}

/// 容器编码模式(对齐 `rurix_basis_wrap.h` 字面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerMode {
    /// UASTC 4×4 → `.ktx2`(supercompressionScheme=0)。
    UastcKtx2,
    /// ETC1S → 真实 `.basis`。
    Etc1sBasis,
}

impl ContainerMode {
    fn raw(self) -> i32 {
        match self {
            ContainerMode::UastcKtx2 => ffi::MODE_UASTC_KTX2,
            ContainerMode::Etc1sBasis => ffi::MODE_ETC1S_BASIS,
        }
    }
}

/// transcode 源容器种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrcKind {
    Basis,
    Ktx2,
}

impl SrcKind {
    fn raw(self) -> i32 {
        match self {
            SrcKind::Basis => ffi::SRC_BASIS,
            SrcKind::Ktx2 => ffi::SRC_KTX2,
        }
    }
}

/// transcode 目标格式(数值 == `basist::transcoder_texture_format` 字面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFormat {
    Bc4R,
    Bc5Rg,
    Bc7Rgba,
    Astc4x4,
}

impl TargetFormat {
    fn raw(self) -> i32 {
        match self {
            TargetFormat::Bc4R => ffi::TF_BC4_R,
            TargetFormat::Bc5Rg => ffi::TF_BC5_RG,
            TargetFormat::Bc7Rgba => ffi::TF_BC7_RGBA,
            TargetFormat::Astc4x4 => ffi::TF_ASTC_4X4,
        }
    }

    /// 每块字节数(BC4=8,BC5/BC7/ASTC 4×4=16)。
    pub fn bytes_per_block(self) -> usize {
        match self {
            TargetFormat::Bc4R => 8,
            TargetFormat::Bc5Rg | TargetFormat::Bc7Rgba | TargetFormat::Astc4x4 => 16,
        }
    }
}

/// 转码结果:GPU 块字节 + 上游报告的原始尺寸。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcoded {
    pub blocks: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// RGBA8 → 真实 basis_universal 容器字节(`.ktx2` 或 `.basis`)。
///
/// `swizzle_rg` = true 时按 XY normal 流铺陈(R→RGB、G→A),使后续 BC5 腿
/// X=R / Y=G 语义成立。
pub fn encode_container(
    rgba: &[u8],
    width: u32,
    height: u32,
    mode: ContainerMode,
    swizzle_rg: bool,
) -> Result<Vec<u8>, BasisError> {
    if width == 0 || height == 0 {
        return Err(BasisError::InvalidDims);
    }
    let need = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if rgba.len() < need {
        return Err(BasisError::InvalidDims);
    }
    let mut buf = ffi::RurixBasisBuf {
        data: ptr::null_mut(),
        len: 0,
    };
    // SAFETY: (U44)`rgba` 在调用期间有效且长度 ≥ w*h*4;`out` 指向本栈 POD;
    // 失败时 wrap 不写非 null data;成功后由 `take_buf` 接管所有权。
    let rc = unsafe {
        ffi::rurix_basis_encode_container(
            rgba.as_ptr(),
            width,
            height,
            mode.raw(),
            i32::from(swizzle_rg),
            &mut buf,
        )
    };
    if rc != 0 {
        // SAFETY: (U46)失败路径仍可能部分分配——交 free 兜底。
        unsafe {
            ffi::rurix_basis_buf_free(&mut buf);
        }
        return Err(BasisError::EncodeFailed(rc));
    }
    take_buf(buf)
}

/// 真实容器 → GPU 块字节(真 transcode,非重打包)。
pub fn transcode(
    container: &[u8],
    src: SrcKind,
    target: TargetFormat,
) -> Result<Transcoded, BasisError> {
    if container.is_empty() {
        return Err(BasisError::InvalidDims);
    }
    let mut buf = ffi::RurixBasisBuf {
        data: ptr::null_mut(),
        len: 0,
    };
    let mut w: u32 = 0;
    let mut h: u32 = 0;
    // SAFETY: (U44)`container` 在调用期间有效且长度 == len;`out`/`w`/`h` 均指向本栈 POD;
    // 失败时 wrap 不写非 null data;成功后由 `take_buf` 接管所有权。
    let rc = unsafe {
        ffi::rurix_basis_transcode(
            container.as_ptr(),
            container.len(),
            src.raw(),
            target.raw(),
            &mut buf,
            &mut w,
            &mut h,
        )
    };
    if rc != 0 {
        // SAFETY: (U46)失败路径仍可能部分分配——交 free 兜底。
        unsafe {
            ffi::rurix_basis_buf_free(&mut buf);
        }
        return Err(BasisError::EncodeFailed(rc));
    }
    let blocks = take_buf(buf)?;
    Ok(Transcoded {
        blocks,
        width: w,
        height: h,
    })
}

/// `.basis` 文件签名(上游 `basis_file_header::cBASISSigValue` = 'B'|'s'<<8 LE)。
/// 真实 `.basis` 文件签名字节(上游 `basis_file_header::m_sig`)。
///
/// 上游值 `cBASISSigValue = ('B' << 8) | 's'` = 0x4273,经 `packed_uint<2>`
/// **小端**落盘 → 磁盘字节序 = `s`,`B`(非 `B`,`s`)。
pub const BASIS_SIG: [u8; 2] = [b's', b'B'];

/// KTX2 12 字节标识。
pub const KTX2_IDENTIFIER: [u8; 12] = [
    0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n',
];

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

    fn gradient(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let on = ((x / 4) + (y / 4)) % 2 == 0;
                if on {
                    v.extend_from_slice(&[220, 40, 40, 255]);
                } else {
                    v.extend_from_slice(&[40, 40, 220, 200]);
                }
            }
        }
        v
    }

    #[test]
    fn version_matches_vendor_pin() {
        assert_eq!(version_string(), VENDOR_VERSION);
        assert!(
            VENDOR_VERSION.starts_with("basis_universal/"),
            "版本串须锚定真实上游,不得为过渡串: {VENDOR_VERSION}"
        );
    }

    #[test]
    fn ktx2_container_is_real_and_deterministic() {
        let rgba = gradient(16, 16);
        let a = encode_container(&rgba, 16, 16, ContainerMode::UastcKtx2, false).unwrap();
        let b = encode_container(&rgba, 16, 16, ContainerMode::UastcKtx2, false).unwrap();
        assert_eq!(a, b, "同输入两次编码须逐字节相等");
        assert!(a.starts_with(&KTX2_IDENTIFIER), "须为真实 KTX2 容器");
        // supercompressionScheme(offset 12 + 8*4)恒 0
        let ss = u32::from_le_bytes(a[44..48].try_into().unwrap());
        assert_eq!(ss, 0, "禁 supercompression");
    }

    #[test]
    fn basis_container_is_real() {
        let rgba = gradient(16, 16);
        let a = encode_container(&rgba, 16, 16, ContainerMode::Etc1sBasis, false).unwrap();
        let b = encode_container(&rgba, 16, 16, ContainerMode::Etc1sBasis, false).unwrap();
        assert_eq!(a, b);
        assert_eq!(&a[..2], &BASIS_SIG, "须为真实 .basis 签名,非 RXBS 冒充");
        assert!(!a.starts_with(b"RXBS"), "禁 RXBS 充当 .basis");
    }

    #[test]
    fn transcode_bc7_from_ktx2() {
        let rgba = gradient(16, 16);
        let ktx2 = encode_container(&rgba, 16, 16, ContainerMode::UastcKtx2, false).unwrap();
        let t = transcode(&ktx2, SrcKind::Ktx2, TargetFormat::Bc7Rgba).unwrap();
        assert_eq!((t.width, t.height), (16, 16));
        assert_eq!(t.blocks.len(), 4 * 4 * 16, "4x4 blocks * 16B");
        assert!(!t.blocks.iter().all(|&x| x == 0));
    }

    #[test]
    fn transcode_astc_and_bc4_bc5_shapes() {
        let rgba = gradient(16, 16);
        let ktx2 = encode_container(&rgba, 16, 16, ContainerMode::UastcKtx2, false).unwrap();
        let astc = transcode(&ktx2, SrcKind::Ktx2, TargetFormat::Astc4x4).unwrap();
        assert_eq!(astc.blocks.len(), 4 * 4 * 16);
        assert!(!astc.blocks.iter().all(|&x| x == 0));
        let bc4 = transcode(&ktx2, SrcKind::Ktx2, TargetFormat::Bc4R).unwrap();
        assert_eq!(bc4.blocks.len(), 4 * 4 * 8, "BC4 = 8B/block");
        let bc5 = transcode(&ktx2, SrcKind::Ktx2, TargetFormat::Bc5Rg).unwrap();
        assert_eq!(bc5.blocks.len(), 4 * 4 * 16, "BC5 = 16B/block");
    }

    #[test]
    fn transcode_rejects_garbage_container() {
        let junk = vec![0u8; 256];
        assert!(transcode(&junk, SrcKind::Basis, TargetFormat::Bc7Rgba).is_err());
        assert!(transcode(&junk, SrcKind::Ktx2, TargetFormat::Bc7Rgba).is_err());
    }

    #[test]
    fn invalid_dims_rejected() {
        let rgba = solid(4, 4, [1, 2, 3, 255]);
        assert_eq!(
            encode_container(&rgba, 0, 4, ContainerMode::UastcKtx2, false),
            Err(BasisError::InvalidDims)
        );
        assert_eq!(
            encode_container(&rgba[..8], 4, 4, ContainerMode::UastcKtx2, false),
            Err(BasisError::InvalidDims)
        );
    }
}
