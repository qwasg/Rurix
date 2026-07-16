//! 嵌入产物描述表解析(RFC-0009 §4.4 `@__rx_gpu_artifacts`):编译器 codegen 侧发射、
//! [`rxrt_ctx_create`](crate::rxrt_ctx_create) 消费的二进制布局(v1,little-endian,
//! Windows x64 唯一 ABI,D-113)。
//!
//! # 描述表布局(v1,共 [`DESC_LEN`] = 48 字节;与编译器侧约定)
//!
//! | 偏移 | 类型 | 字段 | 含义 |
//! |---|---|---|---|
//! | 0 | `u32` | `version` | 描述表版本,本版 = `1`(其余值确定性拒绝) |
//! | 4 | `u32` | `reserved` | 保留(忽略) |
//! | 8 | `u64` | `ptx_ptr` | PTX fallback 文本首字节**绝对地址**(必存,RXS-0150) |
//! | 16 | `u64` | `ptx_len` | PTX 字节数(> 0;UTF-8 文本,无需 NUL 终止) |
//! | 24 | `u64` | `cubin_ptr` | 可选预编 cubin 首字节绝对地址(`cubin_len = 0` 时忽略) |
//! | 32 | `u64` | `cubin_len` | cubin 字节数(`0` = 无 cubin,仅 PTX fallback) |
//! | 40 | `u8[8]` | `sm_key` | cubin 架构键,NUL 填充(如 `"sm_89\0\0\0"`;无 cubin 时忽略) |
//!
//! 指针字段为**绝对地址**:codegen 侧以同产物常量段(`@__rx_gpu_ptx` /
//! `@__rx_gpu_cubin_sm89`)的全局常量地址填入,进程生命期有效。解析即拷贝为 owned
//! (`String` / `Vec<u8>`),不持外部指针越出调用(U25)。

use rurix_rt::fatbin::ArchKey;

/// 描述表总长(v1,字节)。
pub(crate) const DESC_LEN: usize = 48;

/// 解析结果:PTX fallback(必存)+ 可选按架构预编 cubin。
pub(crate) struct ParsedArtifacts {
    pub(crate) ptx: String,
    pub(crate) cubin: Option<(ArchKey, Vec<u8>)>,
}

/// 固定偏移取 `u32`(little-endian;偏移由本模块布局常量约束,不越界)。
fn u32_at(raw: &[u8; DESC_LEN], off: usize) -> u32 {
    u32::from_le_bytes(raw[off..off + 4].try_into().expect("固定偏移切片长度 4"))
}

/// 固定偏移取 `u64`(little-endian)。
fn u64_at(raw: &[u8; DESC_LEN], off: usize) -> u64 {
    u64::from_le_bytes(raw[off..off + 8].try_into().expect("固定偏移切片长度 8"))
}

/// 解析嵌入产物描述表(畸形形态一律确定性拒绝,错误文本进 `RXRT:` 诊断 detail,
/// RXS-0193)。
///
/// # Safety
///
/// `desc` 须为 null 或指向 ≥ [`DESC_LEN`] 字节可读描述表;其 `ptx_ptr`/`cubin_ptr`
/// 字段(对应 `*_len` > 0 时)须指向该长度的有效可读字节(codegen 侧以同产物常量段
/// 地址填入,进程生命期有效,RFC-0009 §4.4)。null 与字段级畸形(版本不符 / 缺 PTX /
/// 坏 sm 键 / 非 UTF-8)在解引用载荷指针**之前**确定性拒绝。
pub(crate) unsafe fn parse(desc: *const u8) -> Result<ParsedArtifacts, String> {
    if desc.is_null() {
        return Err("null artifacts descriptor".to_owned());
    }
    let mut raw = [0u8; DESC_LEN];
    // SAFETY: (U25):调用方契约(见 fn 文档):`desc` 非 null 时指向 ≥ DESC_LEN 字节
    // 可读描述表;目标为本栈数组,长度精确 DESC_LEN,不重叠。
    unsafe { core::ptr::copy_nonoverlapping(desc, raw.as_mut_ptr(), DESC_LEN) };

    let version = u32_at(&raw, 0);
    if version != 1 {
        return Err(format!(
            "unsupported artifacts descriptor version {version} (expected 1)"
        ));
    }
    let ptx_ptr = u64_at(&raw, 8);
    let ptx_len = u64_at(&raw, 16);
    let cubin_ptr = u64_at(&raw, 24);
    let cubin_len = u64_at(&raw, 32);
    if ptx_ptr == 0 || ptx_len == 0 {
        return Err("missing PTX fallback (ptx_ptr/ptx_len must be non-zero, RXS-0150)".to_owned());
    }
    // SAFETY: (U25):调用方契约:`ptx_ptr` 指向 `ptx_len` 字节有效可读常量段
    // (进程生命期);随即拷贝为 owned String,借用不越出本函数。
    let ptx_bytes = unsafe { core::slice::from_raw_parts(ptx_ptr as *const u8, ptx_len as usize) };
    let Ok(ptx) = core::str::from_utf8(ptx_bytes) else {
        return Err("PTX fallback is not valid UTF-8".to_owned());
    };

    let cubin = if cubin_len == 0 {
        None
    } else {
        if cubin_ptr == 0 {
            return Err("cubin_len > 0 but cubin_ptr is null".to_owned());
        }
        let sm_raw = &raw[40..48];
        let end = sm_raw.iter().position(|b| *b == 0).unwrap_or(sm_raw.len());
        let Some(sm) = core::str::from_utf8(&sm_raw[..end])
            .ok()
            .and_then(ArchKey::parse)
        else {
            return Err(format!(
                "bad sm key {sm_raw:?} (expected e.g. \"sm_89\" NUL-padded)"
            ));
        };
        // SAFETY: (U25):调用方契约:`cubin_ptr` 指向 `cubin_len` 字节有效可读常量段;
        // 随即拷贝为 owned Vec,借用不越出本函数。
        let bytes =
            unsafe { core::slice::from_raw_parts(cubin_ptr as *const u8, cubin_len as usize) };
        Some((sm, bytes.to_vec()))
    };
    Ok(ParsedArtifacts {
        ptx: ptx.to_owned(),
        cubin,
    })
}

/// 构造 v1 描述表字节(单测辅助:指针字段填调用方切片的**绝对地址**,故 `ptx`/`cubin`
/// 缓冲须在解析期间存活;`cubin` 为空 = 无 cubin 变体,`cubin_ptr` 填 0)。
#[cfg(test)]
pub(crate) fn make_artifacts_blob(ptx: &[u8], cubin: &[u8], sm_key: &[u8; 8]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(DESC_LEN);
    blob.extend_from_slice(&1u32.to_le_bytes()); // version = 1
    blob.extend_from_slice(&0u32.to_le_bytes()); // reserved
    blob.extend_from_slice(&(ptx.as_ptr() as u64).to_le_bytes());
    blob.extend_from_slice(&(ptx.len() as u64).to_le_bytes());
    let cubin_ptr = if cubin.is_empty() {
        0u64
    } else {
        cubin.as_ptr() as u64
    };
    blob.extend_from_slice(&cubin_ptr.to_le_bytes());
    blob.extend_from_slice(&(cubin.len() as u64).to_le_bytes());
    blob.extend_from_slice(sm_key);
    blob
}
