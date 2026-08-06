//! 磁盘页 RXPD encode/decode + disk→memory 映射（RXS-0339 / RXS-0341）。

use crate::codec::{self, CODEC_ID_RXPZ_LZ1, CODEC_VERSION, CodecError};
use crate::memory::{self, MEMORY_MAJOR, MemoryError, MemoryPage, encode_memory_page};
use rurix_pkg::sha256;

pub const RXPD_MAGIC: [u8; 4] = *b"RXPD";
pub const FORMAT_ID: u32 = 3;
pub const DISK_MAJOR: u16 = 1;
pub const DISK_MINOR: u16 = 0;
pub const ENDIAN_LE: u8 = 1;
pub const HEADER_SIZE: u16 = 148;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskError {
    BadMagic,
    UnsupportedVersion { major: u16, minor: u16 },
    BadHeader(&'static str),
    Truncated(&'static str),
    ChecksumMismatch,
    UnknownCodec(u32),
    MappingRejected,
    Codec(CodecError),
    Memory(MemoryError),
    DigestMismatch,
}

impl std::fmt::Display for DiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiskError::BadMagic => write!(f, "RXPD 魔数不符"),
            DiskError::UnsupportedVersion { major, minor } => {
                write!(f, "RXPD 版本不支持:{major}.{minor}")
            }
            DiskError::BadHeader(s) => write!(f, "RXPD header 损坏:{s}"),
            DiskError::Truncated(s) => write!(f, "RXPD 截断:{s}"),
            DiskError::ChecksumMismatch => write!(f, "RXPD payload_checksum 不匹配"),
            DiskError::UnknownCodec(id) => write!(f, "RXPD 未知 codec_id:{id}"),
            DiskError::MappingRejected => write!(f, "RXPD→RXPM 映射拒绝"),
            DiskError::Codec(e) => write!(f, "RXPD codec:{e}"),
            DiskError::Memory(e) => write!(f, "RXPD memory:{e}"),
            DiskError::DigestMismatch => write!(f, "RXPD schema_digest 不匹配"),
        }
    }
}

impl std::error::Error for DiskError {}

impl From<CodecError> for DiskError {
    fn from(e: CodecError) -> Self {
        DiskError::Codec(e)
    }
}

impl From<MemoryError> for DiskError {
    fn from(e: MemoryError) -> Self {
        DiskError::Memory(e)
    }
}

/// 冻结映射表：仅 (RXPD,major=1)→(RXPM,major=1)。
pub fn mapping_allows(disk_major: u16, mem_major: u16) -> bool {
    disk_major == DISK_MAJOR && mem_major == MEMORY_MAJOR
}

pub fn schema_digest() -> [u8; 32] {
    let mut pre = Vec::with_capacity(40);
    pre.extend_from_slice(b"RXPD-SCHEMA-V1\0");
    put_u16(&mut pre, DISK_MAJOR);
    put_u16(&mut pre, DISK_MINOR);
    put_u32(&mut pre, FORMAT_ID);
    put_u16(&mut pre, HEADER_SIZE);
    put_u32(&mut pre, CODEC_ID_RXPZ_LZ1);
    put_u32(&mut pre, CODEC_VERSION);
    sha256::digest(&pre)
}

pub fn dependency_digest(deps: &[u64]) -> [u8; 32] {
    let mut pre = Vec::with_capacity(deps.len() * 8);
    for &d in deps {
        pre.extend_from_slice(&d.to_le_bytes());
    }
    sha256::digest(&pre)
}

/// 编码磁盘页：MemoryPage → RXPD bytes。
pub fn encode_disk_page(page: &MemoryPage, dependency_page_ids: &[u64]) -> Vec<u8> {
    let mem = encode_memory_page(page);
    let compressed = codec::compress(&mem);
    let mut out = Vec::with_capacity(HEADER_SIZE as usize + compressed.len());
    out.extend_from_slice(&RXPD_MAGIC);
    put_u32(&mut out, FORMAT_ID);
    put_u16(&mut out, DISK_MAJOR);
    put_u16(&mut out, DISK_MINOR);
    out.push(ENDIAN_LE);
    out.push(0);
    put_u16(&mut out, HEADER_SIZE);
    put_u32(&mut out, 0); // section_dir_count
    put_u32(&mut out, CODEC_ID_RXPZ_LZ1);
    put_u32(&mut out, CODEC_VERSION);
    put_u64(&mut out, mem.len() as u64);
    put_u64(&mut out, compressed.len() as u64);
    put_u64(&mut out, page.logical_page_id);
    out.extend_from_slice(&schema_digest());
    out.extend_from_slice(&sha256::digest(&compressed));
    out.extend_from_slice(&dependency_digest(dependency_page_ids));
    debug_assert_eq!(out.len(), HEADER_SIZE as usize);
    out.extend_from_slice(&compressed);
    out
}

/// 解码 RXPD → MemoryPage（拒录发生在大分配前）。
pub fn decode_disk_page(bytes: &[u8]) -> Result<MemoryPage, DiskError> {
    // 消费前：至少读到 major。
    if bytes.len() < 12 {
        if bytes.len() >= 4 && bytes[0..4] != RXPD_MAGIC {
            return Err(DiskError::BadMagic);
        }
        return Err(DiskError::Truncated("header"));
    }
    if bytes[0..4] != RXPD_MAGIC {
        return Err(DiskError::BadMagic);
    }
    let major = u16::from_le_bytes([bytes[8], bytes[9]]);
    let minor = u16::from_le_bytes([bytes[10], bytes[11]]);
    if major != DISK_MAJOR {
        return Err(DiskError::UnsupportedVersion { major, minor });
    }
    if bytes.len() < HEADER_SIZE as usize {
        return Err(DiskError::Truncated("header"));
    }

    let format_id = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if format_id != FORMAT_ID {
        return Err(DiskError::BadHeader("format_id"));
    }
    if bytes[12] != ENDIAN_LE {
        return Err(DiskError::BadHeader("endian"));
    }
    if bytes[13] != 0 {
        return Err(DiskError::BadHeader("reserved0"));
    }
    let header_size = u16::from_le_bytes([bytes[14], bytes[15]]);
    if header_size != HEADER_SIZE {
        return Err(DiskError::BadHeader("header_size"));
    }
    let section_dir_count = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    if section_dir_count != 0 {
        return Err(DiskError::BadHeader("section_dir_count"));
    }
    let codec_id = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
    let codec_version = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    if codec_id != CODEC_ID_RXPZ_LZ1 {
        return Err(DiskError::UnknownCodec(codec_id));
    }
    if codec_version != CODEC_VERSION {
        return Err(DiskError::BadHeader("codec_version"));
    }
    let uncompressed_size = u64::from_le_bytes(bytes[28..36].try_into().unwrap()) as usize;
    let compressed_size = u64::from_le_bytes(bytes[36..44].try_into().unwrap()) as usize;
    let _logical_page_id = u64::from_le_bytes(bytes[44..52].try_into().unwrap());
    let mut schema = [0u8; 32];
    schema.copy_from_slice(&bytes[52..84]);
    let mut checksum = [0u8; 32];
    checksum.copy_from_slice(&bytes[84..116]);
    // dependency_digest 保留在 116..148，decode 不强制复核（编码侧写入）。

    if schema != schema_digest() {
        return Err(DiskError::DigestMismatch);
    }

    // 截断：在解压分配前检查。
    let need = HEADER_SIZE as usize + compressed_size;
    if bytes.len() < need {
        return Err(DiskError::Truncated("payload"));
    }
    if bytes.len() != need {
        return Err(DiskError::BadHeader("trailing"));
    }
    // 防巨大分配：上限 = STREAM_PAGE_SIZE * 2（内存页含对齐，宽松顶）。
    if uncompressed_size > crate::logical::STREAM_PAGE_SIZE as usize * 2 {
        return Err(DiskError::BadHeader("uncompressed_too_large"));
    }

    let payload = &bytes[HEADER_SIZE as usize..need];
    if sha256::digest(payload) != checksum {
        return Err(DiskError::ChecksumMismatch);
    }

    let mem_bytes = codec::decompress(payload, uncompressed_size)?;
    // 映射表
    if mem_bytes.len() < 12 {
        return Err(DiskError::MappingRejected);
    }
    if mem_bytes[0..4] != memory::RXPM_MAGIC {
        return Err(DiskError::MappingRejected);
    }
    let mem_major = u16::from_le_bytes([mem_bytes[8], mem_bytes[9]]);
    if !mapping_allows(major, mem_major) {
        return Err(DiskError::MappingRejected);
    }
    Ok(memory::decode_memory_page(&mem_bytes)?)
}

/// 仅返回压缩流（供 compress_twice 腿）。
pub fn compress_memory_image(mem_image: &[u8]) -> Vec<u8> {
    codec::compress(mem_image)
}

fn put_u16(b: &mut Vec<u8>, v: u16) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_u32(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_u64(b: &mut Vec<u8>, v: u64) {
    b.extend_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logical::{FLAG_ROOT, LogicalPage, PageClusterRecord, quantize_center};
    use crate::memory::from_logical;

    fn sample_mem() -> MemoryPage {
        let bounds = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let center = [0.1, 0.2, 0.3];
        let (qx, qy, qz) = quantize_center(center, bounds);
        from_logical(&LogicalPage {
            page_id: 0,
            flags: FLAG_ROOT,
            lod_level_min: 0,
            lod_level_max: 0,
            bounds,
            clusters: vec![PageClusterRecord {
                cluster_id: 1,
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

    //@ spec: RXS-0339
    #[test]
    fn disk_roundtrip() {
        let mem = sample_mem();
        let a = encode_disk_page(&mem, &[]);
        let b = encode_disk_page(&mem, &[]);
        assert_eq!(a, b);
        assert_eq!(&a[0..4], b"RXPD");
        let back = decode_disk_page(&a).unwrap();
        assert_eq!(back.clusters[0].cluster_id, 1);
    }

    //@ spec: RXS-0341
    #[test]
    fn unknown_codec_before_alloc() {
        let mut bytes = encode_disk_page(&sample_mem(), &[]);
        bytes[20] = 99;
        bytes[21] = 0;
        bytes[22] = 0;
        bytes[23] = 0;
        // checksum/schema 可能仍过？codec 在 checksum 后检查——先修 schema 不相关。
        // 未知 codec 在 checksum 之后？按实现：codec 在 checksum 前检查。
        assert_eq!(decode_disk_page(&bytes), Err(DiskError::UnknownCodec(99)));
    }

    //@ spec: RXS-0341
    #[test]
    fn truncation_before_decompress() {
        let bytes = encode_disk_page(&sample_mem(), &[]);
        let cut = &bytes[..HEADER_SIZE as usize + 2];
        assert!(matches!(
            decode_disk_page(cut),
            Err(DiskError::Truncated("payload"))
        ));
    }
}
