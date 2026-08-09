//! 内存页 RXPM encode/decode（RXS-0338）。

use crate::logical::{FLAG_ROOT, LogicalPage};
use rurix_pkg::sha256;

pub const RXPM_MAGIC: [u8; 4] = *b"RXPM";
pub const FORMAT_ID: u32 = 2;
pub const MEMORY_MAJOR: u16 = 1;
pub const MEMORY_MINOR: u16 = 0;
pub const ENDIAN_LE: u8 = 1;
pub const HEADER_SIZE: u16 = 48;
pub const SECTION_DIR_ENTRY_SIZE: usize = 16;
pub const SECTION_COUNT_V1: u32 = 4;

pub const KIND_POS_Q16: u32 = 1;
pub const KIND_INDICES_U8: u32 = 2;
pub const KIND_CLUSTER_META: u32 = 3;
pub const KIND_QUANT_PARAMS: u32 = 4;

pub const POS_RECORD_SIZE: usize = 8;
pub const META_RECORD_SIZE: usize = 32;
pub const QUANT_PARAMS_SIZE: usize = 32;

/// 解码后的内存页逻辑视图。
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPage {
    pub logical_page_id: u64,
    pub flags: u8,
    pub clusters: Vec<MemCluster>,
    pub indices: Vec<u8>,
    pub bounds: [f32; 6],
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemCluster {
    pub cluster_id: u32,
    pub qx: u16,
    pub qy: u16,
    pub qz: u16,
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub level: u32,
    pub group: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    BadMagic,
    UnsupportedVersion { major: u16, minor: u16 },
    BadHeader(&'static str),
    Truncated(&'static str),
    Inconsistent(&'static str),
    SectionOverlap,
    SectionOob,
    DigestMismatch,
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::BadMagic => write!(f, "RXPM 魔数不符"),
            MemoryError::UnsupportedVersion { major, minor } => {
                write!(f, "RXPM 版本不支持:{major}.{minor}")
            }
            MemoryError::BadHeader(s) => write!(f, "RXPM header 损坏:{s}"),
            MemoryError::Truncated(s) => write!(f, "RXPM 截断:{s}"),
            MemoryError::Inconsistent(s) => write!(f, "RXPM 不一致:{s}"),
            MemoryError::SectionOverlap => write!(f, "RXPM section 重叠"),
            MemoryError::SectionOob => write!(f, "RXPM section 越界"),
            MemoryError::DigestMismatch => write!(f, "RXPM schema_digest 不匹配"),
        }
    }
}

impl std::error::Error for MemoryError {}

/// 冻结 schema digest（完整 32B）。
pub fn schema_digest() -> [u8; 32] {
    let mut pre = Vec::with_capacity(32);
    pre.extend_from_slice(b"RXPM-SCHEMA-V1\0");
    put_u16(&mut pre, MEMORY_MAJOR);
    put_u16(&mut pre, MEMORY_MINOR);
    put_u32(&mut pre, FORMAT_ID);
    put_u16(&mut pre, HEADER_SIZE);
    put_u32(&mut pre, SECTION_COUNT_V1);
    sha256::digest(&pre)
}

/// 自逻辑页构造内存页视图。
pub fn from_logical(page: &LogicalPage) -> MemoryPage {
    MemoryPage {
        logical_page_id: page.page_id,
        flags: page.flags,
        clusters: page
            .clusters
            .iter()
            .map(|c| MemCluster {
                cluster_id: c.cluster_id,
                qx: c.qx,
                qy: c.qy,
                qz: c.qz,
                vertex_offset: c.vertex_offset,
                triangle_offset: c.triangle_offset,
                vertex_count: c.vertex_count,
                triangle_count: c.triangle_count,
                level: c.level,
                group: c.group,
            })
            .collect(),
        indices: page.indices.clone(),
        bounds: page.bounds,
    }
}

fn align_up(v: usize, a: usize) -> usize {
    v.div_ceil(a) * a
}

/// 编码 RXPM image（确定性）。
pub fn encode_memory_page(page: &MemoryPage) -> Vec<u8> {
    let n = page.clusters.len();
    let pos_size = n * POS_RECORD_SIZE;
    let idx_raw = page.indices.len();
    let idx_size = align_up(idx_raw, 4);
    let meta_size = n * META_RECORD_SIZE;
    let quant_size = QUANT_PARAMS_SIZE;

    let dir_bytes = SECTION_COUNT_V1 as usize * SECTION_DIR_ENTRY_SIZE;
    let body_start = HEADER_SIZE as usize + dir_bytes;

    let mut cursor = body_start;
    let pos_off = align_up(cursor, 16);
    cursor = pos_off + pos_size;
    let idx_off = align_up(cursor, 4);
    cursor = idx_off + idx_size;
    let meta_off = align_up(cursor, 16);
    cursor = meta_off + meta_size;
    let quant_off = align_up(cursor, 16);
    let total = quant_off + quant_size;

    let mut out = vec![0u8; total];
    // header
    out[0..4].copy_from_slice(&RXPM_MAGIC);
    put_u32_at(&mut out, 4, FORMAT_ID);
    put_u16_at(&mut out, 8, MEMORY_MAJOR);
    put_u16_at(&mut out, 10, MEMORY_MINOR);
    out[12] = ENDIAN_LE;
    out[13] = page.flags;
    put_u16_at(&mut out, 14, HEADER_SIZE);
    put_u64_at(&mut out, 16, page.logical_page_id);
    put_u32_at(&mut out, 24, SECTION_COUNT_V1);
    put_u32_at(&mut out, 28, 0);
    let digest = schema_digest();
    out[32..48].copy_from_slice(&digest[..16]);

    // section dir (kind 升序 1..4)
    write_dir(&mut out, 0, KIND_POS_Q16, pos_off, pos_size, 16);
    write_dir(&mut out, 1, KIND_INDICES_U8, idx_off, idx_size, 4);
    write_dir(&mut out, 2, KIND_CLUSTER_META, meta_off, meta_size, 16);
    write_dir(&mut out, 3, KIND_QUANT_PARAMS, quant_off, quant_size, 16);

    // POS
    for (i, c) in page.clusters.iter().enumerate() {
        let o = pos_off + i * POS_RECORD_SIZE;
        put_u16_at(&mut out, o, c.qx);
        put_u16_at(&mut out, o + 2, c.qy);
        put_u16_at(&mut out, o + 4, c.qz);
        put_u16_at(&mut out, o + 6, 0);
    }
    // INDICES
    out[idx_off..idx_off + idx_raw].copy_from_slice(&page.indices);
    // META
    for (i, c) in page.clusters.iter().enumerate() {
        let o = meta_off + i * META_RECORD_SIZE;
        put_u32_at(&mut out, o, c.cluster_id);
        put_u32_at(&mut out, o + 4, c.vertex_offset);
        put_u32_at(&mut out, o + 8, c.triangle_offset);
        put_u32_at(&mut out, o + 12, c.vertex_count);
        put_u32_at(&mut out, o + 16, c.triangle_count);
        put_u32_at(&mut out, o + 20, c.level);
        put_u32_at(&mut out, o + 24, c.group);
        put_u32_at(&mut out, o + 28, 0);
    }
    // QUANT
    for (i, &b) in page.bounds.iter().enumerate() {
        put_u32_at(&mut out, quant_off + i * 4, b.to_bits());
    }
    out
}

fn write_dir(out: &mut [u8], idx: usize, kind: u32, off: usize, size: usize, align: u32) {
    let base = HEADER_SIZE as usize + idx * SECTION_DIR_ENTRY_SIZE;
    put_u32_at(out, base, kind);
    put_u32_at(out, base + 4, off as u32);
    put_u32_at(out, base + 8, size as u32);
    put_u32_at(out, base + 12, align);
}

/// 解码 RXPM（先 magic/major，再 section 校验）。
pub fn decode_memory_page(bytes: &[u8]) -> Result<MemoryPage, MemoryError> {
    if bytes.len() < 12 {
        if bytes.len() >= 4 && bytes[0..4] != RXPM_MAGIC {
            return Err(MemoryError::BadMagic);
        }
        return Err(MemoryError::Truncated("header"));
    }
    if bytes[0..4] != RXPM_MAGIC {
        return Err(MemoryError::BadMagic);
    }
    let major = u16::from_le_bytes([bytes[8], bytes[9]]);
    let minor = u16::from_le_bytes([bytes[10], bytes[11]]);
    if major != MEMORY_MAJOR {
        return Err(MemoryError::UnsupportedVersion { major, minor });
    }
    if bytes.len() < HEADER_SIZE as usize {
        return Err(MemoryError::Truncated("header"));
    }
    let format_id = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if format_id != FORMAT_ID {
        return Err(MemoryError::BadHeader("format_id"));
    }
    if bytes[12] != ENDIAN_LE {
        return Err(MemoryError::BadHeader("endian"));
    }
    let flags = bytes[13];
    if flags & !FLAG_ROOT != 0 {
        return Err(MemoryError::BadHeader("flags"));
    }
    let header_size = u16::from_le_bytes([bytes[14], bytes[15]]);
    if header_size != HEADER_SIZE {
        return Err(MemoryError::BadHeader("header_size"));
    }
    let logical_page_id = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    let section_count = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    if section_count != SECTION_COUNT_V1 {
        return Err(MemoryError::BadHeader("section_count"));
    }
    let reserved = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
    if reserved != 0 {
        return Err(MemoryError::BadHeader("reserved"));
    }
    let digest = schema_digest();
    if bytes[32..48] != digest[..16] {
        return Err(MemoryError::DigestMismatch);
    }

    let dir_bytes = section_count as usize * SECTION_DIR_ENTRY_SIZE;
    let dir_end = HEADER_SIZE as usize + dir_bytes;
    if bytes.len() < dir_end {
        return Err(MemoryError::Truncated("section_dir"));
    }

    let mut sections = Vec::with_capacity(section_count as usize);
    for i in 0..section_count as usize {
        let base = HEADER_SIZE as usize + i * SECTION_DIR_ENTRY_SIZE;
        let kind = u32::from_le_bytes(bytes[base..base + 4].try_into().unwrap());
        let off = u32::from_le_bytes(bytes[base + 4..base + 8].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(bytes[base + 8..base + 12].try_into().unwrap()) as usize;
        let align = u32::from_le_bytes(bytes[base + 12..base + 16].try_into().unwrap()) as usize;
        if align == 0 || !off.is_multiple_of(align) {
            return Err(MemoryError::Inconsistent("align"));
        }
        if off < dir_end {
            return Err(MemoryError::SectionOob);
        }
        if off
            .checked_add(size)
            .map(|e| e > bytes.len())
            .unwrap_or(true)
        {
            return Err(MemoryError::SectionOob);
        }
        sections.push((kind, off, size, align));
    }
    // kind 升序
    for w in sections.windows(2) {
        if w[0].0 >= w[1].0 {
            return Err(MemoryError::Inconsistent("kind_order"));
        }
    }
    // overlap
    let mut ranges: Vec<(usize, usize)> = sections.iter().map(|s| (s.1, s.1 + s.2)).collect();
    ranges.sort_by_key(|r| r.0);
    for w in ranges.windows(2) {
        if w[0].1 > w[1].0 {
            return Err(MemoryError::SectionOverlap);
        }
    }
    // 空洞恒 0
    let mut covered = vec![false; bytes.len()];
    for c in covered.iter_mut().take(dir_end) {
        *c = true;
    }
    for &(_, off, size, _) in &sections {
        for b in &mut covered[off..off + size] {
            *b = true;
        }
    }
    for (i, &c) in covered.iter().enumerate() {
        if !c && bytes[i] != 0 {
            return Err(MemoryError::Inconsistent("hole_nonzero"));
        }
    }

    let pos = sections
        .iter()
        .find(|s| s.0 == KIND_POS_Q16)
        .ok_or(MemoryError::Inconsistent("missing_pos"))?;
    let idx = sections
        .iter()
        .find(|s| s.0 == KIND_INDICES_U8)
        .ok_or(MemoryError::Inconsistent("missing_idx"))?;
    let meta = sections
        .iter()
        .find(|s| s.0 == KIND_CLUSTER_META)
        .ok_or(MemoryError::Inconsistent("missing_meta"))?;
    let quant = sections
        .iter()
        .find(|s| s.0 == KIND_QUANT_PARAMS)
        .ok_or(MemoryError::Inconsistent("missing_quant"))?;

    if pos.2 % POS_RECORD_SIZE != 0 || meta.2 % META_RECORD_SIZE != 0 {
        return Err(MemoryError::Inconsistent("record_size"));
    }
    if pos.2 / POS_RECORD_SIZE != meta.2 / META_RECORD_SIZE {
        return Err(MemoryError::Inconsistent("cluster_count"));
    }
    if quant.2 != QUANT_PARAMS_SIZE {
        return Err(MemoryError::Inconsistent("quant_size"));
    }
    let n = pos.2 / POS_RECORD_SIZE;
    let mut clusters = Vec::with_capacity(n);
    for i in 0..n {
        let po = pos.1 + i * POS_RECORD_SIZE;
        let mo = meta.1 + i * META_RECORD_SIZE;
        let qx = u16::from_le_bytes(bytes[po..po + 2].try_into().unwrap());
        let qy = u16::from_le_bytes(bytes[po + 2..po + 4].try_into().unwrap());
        let qz = u16::from_le_bytes(bytes[po + 4..po + 6].try_into().unwrap());
        let pad = u16::from_le_bytes(bytes[po + 6..po + 8].try_into().unwrap());
        if pad != 0 {
            return Err(MemoryError::Inconsistent("pos_pad"));
        }
        let cluster_id = u32::from_le_bytes(bytes[mo..mo + 4].try_into().unwrap());
        let vertex_offset = u32::from_le_bytes(bytes[mo + 4..mo + 8].try_into().unwrap());
        let triangle_offset = u32::from_le_bytes(bytes[mo + 8..mo + 12].try_into().unwrap());
        let vertex_count = u32::from_le_bytes(bytes[mo + 12..mo + 16].try_into().unwrap());
        let triangle_count = u32::from_le_bytes(bytes[mo + 16..mo + 20].try_into().unwrap());
        let level = u32::from_le_bytes(bytes[mo + 20..mo + 24].try_into().unwrap());
        let group = u32::from_le_bytes(bytes[mo + 24..mo + 28].try_into().unwrap());
        let reserved = u32::from_le_bytes(bytes[mo + 28..mo + 32].try_into().unwrap());
        if reserved != 0 {
            return Err(MemoryError::Inconsistent("meta_reserved"));
        }
        clusters.push(MemCluster {
            cluster_id,
            qx,
            qy,
            qz,
            vertex_offset,
            triangle_offset,
            vertex_count,
            triangle_count,
            level,
            group,
        });
    }

    // indices：去掉尾部 pad 0，但保留三角形需要的字节——以 max(triangle_offset+3*triangle_count) 截。
    let mut need = 0usize;
    for c in &clusters {
        let end = c.triangle_offset as usize + c.triangle_count as usize * 3;
        need = need.max(end);
    }
    if need > idx.2 {
        return Err(MemoryError::SectionOob);
    }
    let indices = bytes[idx.1..idx.1 + need].to_vec();
    for &b in &bytes[idx.1 + need..idx.1 + idx.2] {
        if b != 0 {
            return Err(MemoryError::Inconsistent("idx_pad"));
        }
    }

    let mut bounds = [0f32; 6];
    for i in 0..6 {
        let bits = u32::from_le_bytes(
            bytes[quant.1 + i * 4..quant.1 + i * 4 + 4]
                .try_into()
                .unwrap(),
        );
        bounds[i] = f32::from_bits(bits);
    }
    let pad0 = u32::from_le_bytes(bytes[quant.1 + 24..quant.1 + 28].try_into().unwrap());
    let pad1 = u32::from_le_bytes(bytes[quant.1 + 28..quant.1 + 32].try_into().unwrap());
    if pad0 != 0 || pad1 != 0 {
        return Err(MemoryError::Inconsistent("quant_pad"));
    }

    Ok(MemoryPage {
        logical_page_id,
        flags,
        clusters,
        indices,
        bounds,
    })
}

fn put_u16(b: &mut Vec<u8>, v: u16) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_u32(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_u16_at(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn put_u32_at(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn put_u64_at(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logical::{PageClusterRecord, quantize_center};

    fn sample() -> MemoryPage {
        let bounds = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let center = [0.25, -0.5, 0.75];
        let (qx, qy, qz) = quantize_center(center, bounds);
        from_logical(&LogicalPage {
            page_id: 0,
            flags: FLAG_ROOT,
            lod_level_min: 0,
            lod_level_max: 0,
            bounds,
            clusters: vec![PageClusterRecord {
                cluster_id: 7,
                qx,
                qy,
                qz,
                center,
                radius: 1.0,
                cone_axis: [0.0, 1.0, 0.0],
                cone_cutoff: 0.0,
                error: 0.0,
                parent_error: 1.0,
                vertex_offset: 0,
                triangle_offset: 0,
                vertex_count: 3,
                triangle_count: 1,
                level: 0,
                group: 0,
            }],
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            dependency_page_ids: vec![],
            dag_links: vec![],
        })
    }

    //@ spec: RXS-0338
    #[test]
    fn roundtrip() {
        let p = sample();
        let bytes = encode_memory_page(&p);
        assert_eq!(&bytes[0..4], b"RXPM");
        let back = decode_memory_page(&bytes).unwrap();
        assert_eq!(back.clusters[0].cluster_id, 7);
        assert_eq!(back.indices, vec![0, 1, 2]);
        assert_eq!(encode_memory_page(&back), bytes);
    }

    //@ spec: RXS-0338
    #[test]
    fn rejects_overlap() {
        let mut bytes = encode_memory_page(&sample());
        // 把 INDICES offset 改到与 POS 重叠。
        let base = HEADER_SIZE as usize + SECTION_DIR_ENTRY_SIZE;
        put_u32_at(&mut bytes, base + 4, 112); // 可能与 pos 重叠
        // 强制两段同 offset
        let pos_off = u32::from_le_bytes(
            bytes[HEADER_SIZE as usize + 4..HEADER_SIZE as usize + 8]
                .try_into()
                .unwrap(),
        );
        put_u32_at(&mut bytes, base + 4, pos_off);
        assert_eq!(decode_memory_page(&bytes), Err(MemoryError::SectionOverlap));
    }
}
