//! 嵌入产物描述表解析(RFC-0009 §4.4 `@__rx_gpu_artifacts`):编译器 codegen 侧发射、
//! [`rxrt_ctx_create`](crate::rxrt_ctx_create) 消费的二进制布局(v1/v2,little-endian,
//! Windows x64 唯一 ABI,D-113)。
//!
//! # 描述表布局(v1,共 [`DESC_LEN`] = 48 字节;与编译器侧约定)
//!
//! | 偏移 | 类型 | 字段 | 含义 |
//! |---|---|---|---|
//! | 0 | `u32` | `version` | 描述表版本(1 或 2;其余值确定性拒绝) |
//! | 4 | `u32` | `reserved` | 保留(忽略) |
//! | 8 | `u64` | `ptx_ptr` | PTX fallback 文本首字节**绝对地址**(必存,RXS-0150) |
//! | 16 | `u64` | `ptx_len` | PTX 字节数(> 0;UTF-8 文本,无需 NUL 终止) |
//! | 24 | `u64` | `cubin_ptr` | 可选预编 cubin 首字节绝对地址(`cubin_len = 0` 时忽略) |
//! | 32 | `u64` | `cubin_len` | cubin 字节数(`0` = 无 cubin,仅 PTX fallback) |
//! | 40 | `u8[8]` | `sm_key` | cubin 架构键,NUL 填充(如 `"sm_89\0\0\0"`;无 cubin 时忽略) |
//!
//! # 描述表布局(v2,共 [`DESC_LEN_V2`] = 64 字节;RXS-0290,v1 48B 前缀原位不变)
//!
//! | 偏移 | 类型 | 字段 | 含义 |
//! |---|---|---|---|
//! | 48 | `u64` | `spirv_count` | SPIR-V 入口表项数(`0` 合法 = 空表) |
//! | 56 | `u64` | `spirv_entries_ptr` | 入口表首项绝对地址(`spirv_count > 0` 时必非空) |
//!
//! v2 入口表项 = 40 字节 packed:`name_ptr:u64`(@0)/ `name_len:u64`(@8)/
//! `stage_tag:u32`(@16,`ShaderStage` 枚举声明序 0..=10)/ `reserved:u32`(@20)/
//! `spv_ptr:u64`(@24)/ `spv_len:u64`(@32);表 = 连续 `spirv_count` 项。
//!
//! 指针字段为**绝对地址**:codegen 侧以同产物常量段(`@__rx_gpu_ptx` /
//! `@__rx_gpu_cubin_sm89` / `@__rx_gpu_spirv*`)的全局常量地址填入,进程生命期有效。
//! 解析即拷贝为 owned(`String` / `Vec<u8>`),不持外部指针越出调用(U25/U31)。

use rurix_rt::fatbin::ArchKey;

/// 描述表总长(v1,字节)。
pub(crate) const DESC_LEN: usize = 48;
/// 描述表总长(v2,字节;RXS-0290:v1 48B 前缀 + `spirv_count`@48 + `spirv_entries_ptr`@56)。
pub(crate) const DESC_LEN_V2: usize = 64;
/// v2 入口表项长(字节,packed;RXS-0290)。
pub(crate) const ENTRY_LEN_V2: usize = 40;
/// v2 入口表项数合理上界(防越界读;超出即畸形确定性拒,RXS-0290 Legality)。
pub(crate) const MAX_SPIRV_ENTRIES: u64 = 4096;
/// `stage_tag` 上界(`ShaderStage` 枚举声明序 0..=10,单一事实源在编译器侧 ast.rs;
/// 越界 = 畸形确定性拒,RXS-0290)。
pub(crate) const MAX_STAGE_TAG: u32 = 10;

/// 解析结果:PTX fallback(必存)+ 可选按架构预编 cubin + v2 SPIR-V 入口表(v1 恒空)。
pub(crate) struct ParsedArtifacts {
    pub(crate) ptx: String,
    pub(crate) cubin: Option<(ArchKey, Vec<u8>)>,
    /// v2 SPIR-V 入口表(RXS-0292;v1 解析恒空,既有消费面 0-byte)。
    pub(crate) spirv_entries: Vec<ParsedSpirvEntry>,
}

/// v2 单入口解析结果(按名索引的独立 SPIR-V 模块,RXS-0291/0292)。
pub(crate) struct ParsedSpirvEntry {
    /// 入口名(PTX launch 同名内核标识)。
    pub(crate) name: String,
    /// `ShaderStage` 枚举声明序(0..=10,RXS-0290)。
    pub(crate) stage_tag: u32,
    /// SPIR-V 模块小端字节(owned 拷贝)。
    pub(crate) spv: Vec<u8>,
}

/// 固定偏移取 `u32`(little-endian;偏移由本模块布局常量约束,不越界)。
fn u32_at(raw: &[u8; DESC_LEN], off: usize) -> u32 {
    u32::from_le_bytes(raw[off..off + 4].try_into().expect("固定偏移切片长度 4"))
}

/// 固定偏移取 `u64`(little-endian)。
fn u64_at(raw: &[u8; DESC_LEN], off: usize) -> u64 {
    u64::from_le_bytes(raw[off..off + 8].try_into().expect("固定偏移切片长度 8"))
}

/// 切片固定偏移取 `u32`(little-endian;v2 尾部/入口表项)。
fn u32_le(raw: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(raw[off..off + 4].try_into().expect("固定偏移切片长度 4"))
}

/// 切片固定偏移取 `u64`(little-endian;v2 尾部/入口表项)。
fn u64_le(raw: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(raw[off..off + 8].try_into().expect("固定偏移切片长度 8"))
}

/// 解析嵌入产物描述表(畸形形态一律确定性拒绝,错误文本进 `RXRT:` 诊断 detail,
/// RXS-0193)。版本分支(RXS-0290/0292):`version == 1` → v1 解析(行为 0-byte);
/// `version == 2` → v1 字段 + SPIR-V 入口表逐项核 RXS-0290 畸形判据;其余 → 拒绝。
///
/// # Safety
///
/// `desc` 须为 null 或指向 ≥ [`DESC_LEN`] 字节可读描述表(`version == 2` 时 ≥
/// [`DESC_LEN_V2`]);其 `ptx_ptr`/`cubin_ptr`/v2 `spirv_entries_ptr` 与表项
/// `name_ptr`/`spv_ptr` 字段(对应长度 > 0 时)须指向该长度的有效可读字节
/// (codegen 侧以同产物常量段地址填入,进程生命期有效,RFC-0009 §4.4 / RXS-0290)。
/// null 与字段级畸形(版本不符 / 缺 PTX / 坏 sm 键 / 非 UTF-8 / 表项畸形)在解引用
/// 载荷指针**之前**确定性拒绝。
//@ spec: RXS-0292
pub(crate) unsafe fn parse(desc: *const u8) -> Result<ParsedArtifacts, String> {
    if desc.is_null() {
        return Err("null artifacts descriptor".to_owned());
    }
    let mut raw = [0u8; DESC_LEN];
    // SAFETY: (U25):调用方契约(见 fn 文档):`desc` 非 null 时指向 ≥ DESC_LEN 字节
    // 可读描述表;目标为本栈数组,长度精确 DESC_LEN,不重叠。
    unsafe { core::ptr::copy_nonoverlapping(desc, raw.as_mut_ptr(), DESC_LEN) };

    let version = u32_at(&raw, 0);
    match version {
        // v1 解析路径 0-byte(RXS-0292 Legality)。
        1 => parse_v1_fields(&raw).map(|(ptx, cubin)| ParsedArtifacts {
            ptx,
            cubin,
            spirv_entries: Vec::new(),
        }),
        2 => {
            let mut tail = [0u8; DESC_LEN_V2 - DESC_LEN];
            // SAFETY: (U31):调用方契约:v2 描述表 ≥ DESC_LEN_V2 字节可读(codegen
            // `@__rx_gpu_artifacts` v2 常量段);源 = desc + 48(已读前缀之后),
            // 目标为本栈数组,长度精确 16,不重叠。
            unsafe {
                core::ptr::copy_nonoverlapping(
                    desc.add(DESC_LEN),
                    tail.as_mut_ptr(),
                    DESC_LEN_V2 - DESC_LEN,
                )
            };
            parse_v2(&raw, &tail)
        }
        v => Err(format!(
            "unsupported artifacts descriptor version {v} (expected 1 or 2)"
        )),
    }
}

/// v1 字段解析产物:PTX fallback + 可选 (架构键, cubin 字节)。
type PtxAndCubin = (String, Option<(ArchKey, Vec<u8>)>);

/// v1 字段解析(version 已核;v1/v2 共用 = v1 前缀兼容,RXS-0290):PTX fallback +
/// 可选 cubin,语义与既有 v1 解析逐字节一致(0-byte)。
fn parse_v1_fields(raw: &[u8; DESC_LEN]) -> Result<PtxAndCubin, String> {
    let ptx_ptr = u64_at(raw, 8);
    let ptx_len = u64_at(raw, 16);
    let cubin_ptr = u64_at(raw, 24);
    let cubin_len = u64_at(raw, 32);
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
    Ok((ptx.to_owned(), cubin))
}

/// v2 SPIR-V 入口表解析(RXS-0292;v1 字段已由 [`parse_v1_fields`] 核过)。逐项核
/// RXS-0290 畸形判据,全部确定性拒:`spirv_count > 0` 而表指针空;表项
/// `name_ptr`/`spv_ptr` 空而对应 `len > 0`;`stage_tag` 越界(> 10);入口名重复;
/// `spirv_count` 越出合理上界(防越界读)。
fn parse_v2(
    raw: &[u8; DESC_LEN],
    tail: &[u8; DESC_LEN_V2 - DESC_LEN],
) -> Result<ParsedArtifacts, String> {
    let (ptx, cubin) = parse_v1_fields(raw)?;
    let spirv_count = u64_le(tail, 0);
    let entries_ptr = u64_le(tail, 8);
    let mut spirv_entries = Vec::new();
    if spirv_count > 0 {
        if entries_ptr == 0 {
            return Err("spirv_count > 0 but spirv_entries_ptr is null (RXS-0290)".to_owned());
        }
        if spirv_count > MAX_SPIRV_ENTRIES {
            return Err(format!(
                "spirv_count {spirv_count} exceeds sanity bound {MAX_SPIRV_ENTRIES} (RXS-0290)"
            ));
        }
        let table_len = spirv_count as usize * ENTRY_LEN_V2;
        let mut table = vec![0u8; table_len];
        // SAFETY: (U31):调用方契约:`entries_ptr` 指向 `spirv_count` 项连续 40B 入口表
        // (codegen `@__rx_gpu_spirv_entries` 常量段,进程生命期);目标为自有 Vec,
        // 长度精确 table_len,不重叠。
        unsafe {
            core::ptr::copy_nonoverlapping(entries_ptr as *const u8, table.as_mut_ptr(), table_len)
        };
        for i in 0..spirv_count as usize {
            let rec = &table[i * ENTRY_LEN_V2..(i + 1) * ENTRY_LEN_V2];
            let name_ptr = u64_le(rec, 0);
            let name_len = u64_le(rec, 8);
            let stage_tag = u32_le(rec, 16);
            let spv_ptr = u64_le(rec, 24);
            let spv_len = u64_le(rec, 32);
            if name_ptr == 0 && name_len > 0 {
                return Err("entry name_ptr is null but name_len > 0 (RXS-0290)".to_owned());
            }
            if spv_ptr == 0 && spv_len > 0 {
                return Err("entry spv_ptr is null but spv_len > 0 (RXS-0290)".to_owned());
            }
            if stage_tag > MAX_STAGE_TAG {
                return Err(format!(
                    "entry stage_tag {stage_tag} out of range 0..={MAX_STAGE_TAG} (RXS-0290)"
                ));
            }
            let name_bytes: &[u8] = if name_len == 0 {
                &[]
            } else {
                // SAFETY: (U31):调用方契约:`name_ptr` 指向 `name_len` 字节有效可读常量段;
                // 借用不越出随即的 owned 拷贝点。
                unsafe { core::slice::from_raw_parts(name_ptr as *const u8, name_len as usize) }
            };
            let Ok(name) = core::str::from_utf8(name_bytes) else {
                return Err("entry name is not valid UTF-8 (RXS-0290)".to_owned());
            };
            if spirv_entries
                .iter()
                .any(|e: &ParsedSpirvEntry| e.name == name)
            {
                return Err(format!("duplicate SPIR-V entry name `{name}` (RXS-0290)"));
            }
            let spv: Vec<u8> = if spv_len == 0 {
                Vec::new()
            } else {
                // SAFETY: (U31):调用方契约:`spv_ptr` 指向 `spv_len` 字节有效可读常量段;
                // 随即 `to_vec` 拷贝为 owned,借用不越出本函数。
                unsafe { core::slice::from_raw_parts(spv_ptr as *const u8, spv_len as usize) }
                    .to_vec()
            };
            spirv_entries.push(ParsedSpirvEntry {
                name: name.to_owned(),
                stage_tag,
                spv,
            });
        }
    }
    Ok(ParsedArtifacts {
        ptx,
        cubin,
        spirv_entries,
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

/// v2 描述表 + 入口表(单测辅助;两缓冲均须存活至解析完成——`desc` 的
/// `spirv_entries_ptr` 指向 `table`)。
#[cfg(test)]
pub(crate) struct V2Blob {
    /// 64B v2 描述表(RXS-0290)。
    pub(crate) desc: Vec<u8>,
    /// 连续 40B 入口表项(`desc` 的 `spirv_entries_ptr` 指向其首字节)。
    pub(crate) table: Vec<u8>,
}

/// 构造 v2 描述表字节(单测辅助,RXS-0290;`entries` = `(name, stage_tag, spv)` 三元组,
/// 指针字段填调用方切片的**绝对地址**,故全部缓冲须在解析期间存活;空 `entries` →
/// `spirv_count = 0` + `spirv_entries_ptr = 0` 合法空表)。
#[cfg(test)]
pub(crate) fn make_artifacts_blob_v2(
    ptx: &[u8],
    cubin: &[u8],
    sm_key: &[u8; 8],
    entries: &[(&[u8], u32, &[u8])],
) -> V2Blob {
    let mut desc = Vec::with_capacity(DESC_LEN_V2);
    desc.extend_from_slice(&2u32.to_le_bytes()); // version = 2
    desc.extend_from_slice(&0u32.to_le_bytes()); // reserved
    desc.extend_from_slice(&(ptx.as_ptr() as u64).to_le_bytes());
    desc.extend_from_slice(&(ptx.len() as u64).to_le_bytes());
    let cubin_ptr = if cubin.is_empty() {
        0u64
    } else {
        cubin.as_ptr() as u64
    };
    desc.extend_from_slice(&cubin_ptr.to_le_bytes());
    desc.extend_from_slice(&(cubin.len() as u64).to_le_bytes());
    desc.extend_from_slice(sm_key);
    desc.extend_from_slice(&(entries.len() as u64).to_le_bytes()); // spirv_count @48
    let mut table = Vec::with_capacity(entries.len() * ENTRY_LEN_V2);
    for (name, stage_tag, spv) in entries {
        table.extend_from_slice(&(name.as_ptr() as u64).to_le_bytes()); // name_ptr @0
        table.extend_from_slice(&(name.len() as u64).to_le_bytes()); // name_len @8
        table.extend_from_slice(&stage_tag.to_le_bytes()); // stage_tag @16
        table.extend_from_slice(&0u32.to_le_bytes()); // reserved @20
        table.extend_from_slice(&(spv.as_ptr() as u64).to_le_bytes()); // spv_ptr @24
        table.extend_from_slice(&(spv.len() as u64).to_le_bytes()); // spv_len @32
    }
    let entries_ptr = if entries.is_empty() {
        0u64
    } else {
        table.as_ptr() as u64
    };
    desc.extend_from_slice(&entries_ptr.to_le_bytes()); // spirv_entries_ptr @56
    V2Blob { desc, table }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SM89_KEY: &[u8; 8] = b"sm_89\0\0\0";
    const NO_CUBIN_KEY: &[u8; 8] = b"\0\0\0\0\0\0\0\0";

    /// v2 双入口回环(RXS-0290/0292):v1 前缀字段 + 入口表逐项(名字/stage_tag/模块
    /// 字节)owned 拷贝;解析为 owned 后源缓冲可安全释放(EXE/DLL 双形态共用同一份
    /// 常量段重定位语义,RXS-0290 Dynamic Semantics —— 指针字段 = 链接期重定位,
    /// 单测以栈/堆缓冲绝对地址模拟,与 v1 同构)。
    //@ spec: RXS-0292
    #[test]
    fn v2_parse_roundtrip_two_entries() {
        let ptx = b".version 8.0\n.target sm_89\n";
        let cubin = [0xDEu8, 0xAD];
        let spv_a = [0x03u8, 0x02, 0x23, 0x07, 0xAA];
        let spv_b = [0x03u8, 0x02, 0x23, 0x07, 0xBB, 0xCC];
        let blob = make_artifacts_blob_v2(
            ptx,
            &cubin,
            SM89_KEY,
            &[(b"rx_k_3", 2, &spv_a), (b"rx_vs_5", 0, &spv_b)],
        );
        assert_eq!(blob.desc.len(), DESC_LEN_V2);
        assert_eq!(blob.table.len(), 2 * ENTRY_LEN_V2);
        // SAFETY: `desc`/`table`/`ptx`/`cubin`/入口 name·spv 缓冲均本栈存活至解析完成;
        // 布局由 make_artifacts_blob_v2 按 RXS-0290 构造。
        let parsed = unsafe { parse(blob.desc.as_ptr()) }.expect("解析 v2 双入口描述表");
        assert_eq!(parsed.ptx.as_bytes(), ptx);
        let (sm, bytes) = parsed.cubin.expect("cubin 变体在位");
        assert_eq!(sm.as_str(), "sm_89");
        assert_eq!(bytes, cubin);
        assert_eq!(parsed.spirv_entries.len(), 2);
        assert_eq!(parsed.spirv_entries[0].name, "rx_k_3");
        assert_eq!(parsed.spirv_entries[0].stage_tag, 2);
        assert_eq!(parsed.spirv_entries[0].spv, spv_a);
        assert_eq!(parsed.spirv_entries[1].name, "rx_vs_5");
        assert_eq!(parsed.spirv_entries[1].stage_tag, 0);
        assert_eq!(parsed.spirv_entries[1].spv, spv_b);
    }

    /// v2 空表合法(RXS-0290:`spirv_count == 0`,`entries_ptr` 空);v1 blob 解析
    /// SPIR-V 表恒空(v1 路径 0-byte,RXS-0292)。
    //@ spec: RXS-0290
    #[test]
    fn v2_empty_table_legal_and_v1_has_no_spirv() {
        let ptx = b".version 8.0\n";
        let blob = make_artifacts_blob_v2(ptx, &[], NO_CUBIN_KEY, &[]);
        // SAFETY: 空表 v2 描述表(64B),本栈存活。
        let parsed = unsafe { parse(blob.desc.as_ptr()) }.expect("解析 v2 空表");
        assert_eq!(parsed.ptx.as_bytes(), ptx);
        assert!(parsed.cubin.is_none());
        assert!(parsed.spirv_entries.is_empty());

        let blob_v1 = make_artifacts_blob(ptx, &[], NO_CUBIN_KEY);
        // SAFETY: 48B v1 描述表,本栈存活;v1 路径不解引 v2 字段。
        let parsed_v1 = unsafe { parse(blob_v1.as_ptr()) }.expect("解析 v1 描述表");
        assert!(parsed_v1.spirv_entries.is_empty());
    }

    /// v2 畸形类逐条确定性拒(RXS-0290 Legality;诊断文本进 RXS-0193 detail)。
    //@ spec: RXS-0290, RXS-0292
    #[test]
    fn v2_parse_rejects_each_malformed_class() {
        let ptx = b".version 8.0\n";
        let spv = [0x03u8, 0x02, 0x23, 0x07];

        // 版本不符(非 1 非 2;0 与 3 均拒)。
        let mut blob = make_artifacts_blob_v2(ptx, &[], NO_CUBIN_KEY, &[]);
        blob.desc[0] = 3;
        // SAFETY: 版本检查在任何载荷解引前拒绝。
        assert!(unsafe { parse(blob.desc.as_ptr()) }.is_err());
        blob.desc[0] = 0;
        // SAFETY: 同上。
        assert!(unsafe { parse(blob.desc.as_ptr()) }.is_err());

        // spirv_count > 0 而 spirv_entries_ptr 空。
        let mut blob = make_artifacts_blob_v2(ptx, &[], NO_CUBIN_KEY, &[(b"rx_k_3", 2, &spv)]);
        blob.desc[56..64].fill(0);
        // SAFETY: 畸形判据在解引 entries_ptr 前拒绝。
        assert!(unsafe { parse(blob.desc.as_ptr()) }.is_err());

        // spirv_count 越合理上界(防越界读)。
        let mut blob = make_artifacts_blob_v2(ptx, &[], NO_CUBIN_KEY, &[(b"rx_k_3", 2, &spv)]);
        blob.desc[48..56].copy_from_slice(&(MAX_SPIRV_ENTRIES + 1).to_le_bytes());
        // SAFETY: 上界判据在拷贝入口表前拒绝。
        assert!(unsafe { parse(blob.desc.as_ptr()) }.is_err());

        // 表项 name_ptr 空而 name_len > 0。
        let mut blob = make_artifacts_blob_v2(ptx, &[], NO_CUBIN_KEY, &[(b"rx_k_3", 2, &spv)]);
        blob.table[0..8].fill(0); // name_ptr @0
        // SAFETY: 畸形判据在解引 name_ptr 前拒绝(原 name_len = 6 > 0)。
        assert!(unsafe { parse(blob.desc.as_ptr()) }.is_err());

        // 表项 spv_ptr 空而 spv_len > 0。
        let mut blob = make_artifacts_blob_v2(ptx, &[], NO_CUBIN_KEY, &[(b"rx_k_3", 2, &spv)]);
        blob.table[24..32].fill(0); // spv_ptr @24
        // SAFETY: 畸形判据在解引 spv_ptr 前拒绝(原 spv_len = 4 > 0)。
        assert!(unsafe { parse(blob.desc.as_ptr()) }.is_err());

        // stage_tag 越界(> 10)。
        let mut blob = make_artifacts_blob_v2(ptx, &[], NO_CUBIN_KEY, &[(b"rx_k_3", 2, &spv)]);
        blob.table[16..20].copy_from_slice(&11u32.to_le_bytes()); // stage_tag @16
        // SAFETY: 越界判据确定性拒绝。
        assert!(unsafe { parse(blob.desc.as_ptr()) }.is_err());

        // 入口名重复。
        let blob = make_artifacts_blob_v2(
            ptx,
            &[],
            NO_CUBIN_KEY,
            &[(b"rx_k_3", 2, &spv), (b"rx_k_3", 0, &spv)],
        );
        // SAFETY: 重名判据在第二项入表前确定性拒绝。
        assert!(unsafe { parse(blob.desc.as_ptr()) }.is_err());

        // 入口名非 UTF-8。
        let bad_name = [0xFFu8, 0xFE];
        let blob = make_artifacts_blob_v2(ptx, &[], NO_CUBIN_KEY, &[(&bad_name, 2, &spv)]);
        // SAFETY: UTF-8 校验确定性拒绝。
        assert!(unsafe { parse(blob.desc.as_ptr()) }.is_err());
    }
}
