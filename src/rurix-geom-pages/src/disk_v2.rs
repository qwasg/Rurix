//! 磁盘页 RXPD **major=2**（cluster 流送 P4-1；RD-039 cluster P4 分项；G31+ 波 C
//! Task C11）——RXPL major=2 逻辑页映像的 RXPZ-LZ1 压缩磁盘面。
//!
//! 演进面（加性新版本面，v1 全文件 0-byte 不动）：
//! - v1（`disk.rs`，RXS-0339/0341）：payload = RXPZ-LZ1(RXPM major=1 内存页映像)，
//!   disk→memory 映射表仅 {(RXPD,1)→(RXPM,1)}——G8.3 冻结面，本模块零触碰；
//! - v2（本模块）：payload = RXPZ-LZ1(**RXPL major=2 逻辑页映像**)
//!   （`logical_v2::encode_logical_page_v2` 原字节）——cluster 流送的消费面是
//!   逻辑页本身（host 驻留调度读簇记录/层级/误差做 cut 与父级回退；device 经
//!   host 重建的元数据面消费），不经 RXPM 内存页 ABI。映射表加性新增一行
//!   {(RXPD,2)→(RXPL,2)}（[`mapping_allows_v2`]），旧行 0-byte。
//!
//! 封套布局与 v1 逐字段同构（148B header：magic/format_id/major/minor/endian/
//! header_size/section_dir_count/codec_id/codec_version/uncompressed_size/
//! compressed_size/logical_page_id/schema_digest/payload_checksum/
//! dependency_digest）——唯一差异 = major=2 与 schema_digest 的 v2 域分离
//! preimage（`b"RXPD-SCHEMA-V2\0"` 起首，冻结目标面 = RXPL logical major=2）。
//!
//! 拒录律沿 v1 口径（消费前/分配前 fail-closed）：bad magic → unknown major →
//! header 字段 → schema_digest → 截断（解压分配前）→ 尾字节 → 巨大分配守卫 →
//! payload checksum → 解压 → RXPL major=2 分发校验（篡改逻辑页 digest 由
//! `decode_logical_page_v2` 确定性拒绝）。

use rurix_pkg::sha256;

use crate::codec::{self, CODEC_ID_RXPZ_LZ1, CODEC_VERSION, CodecError};
use crate::logical::{PageDecodeError, STREAM_PAGE_SIZE};
use crate::logical_v2::{
    LOGICAL_MAJOR_V2, LogicalPageV2, decode_logical_page_v2, encode_logical_page_v2,
};

/// v2 磁盘主版本号（与 v1 `disk::DISK_MAJOR=1` 不同且冻结）。
pub const DISK_MAJOR_V2: u16 = 2;
/// v2 磁盘次版本号。
pub const DISK_MINOR_V2: u16 = 0;
/// 封套魔数（与 v1 同容器族；major 分发布局）。
pub const RXPD_MAGIC: [u8; 4] = *b"RXPD";
/// 封套 format_id（与 v1 同值；版本演进走 major，沿 RXPL v1/v2 同 FORMAT_ID 先例）。
pub const FORMAT_ID: u32 = 3;
/// header 尺寸（与 v1 逐字段同构 148B）。
pub const HEADER_SIZE_V2: u16 = 148;
/// 小端标记（与 v1 同值）。
pub const ENDIAN_LE: u8 = 1;

/// v2 解码 typed 错误（fail-closed；无静默、无猜测布局解析）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskV2Error {
    /// 魔数不符。
    BadMagic,
    /// major 未知（v1 文件进 v2 解码器同此拒——major 分发面）。
    UnsupportedVersion { major: u16, minor: u16 },
    /// header 字段破损（字段名自述）。
    BadHeader(&'static str),
    /// 截断（阶段自述；payload 截断判定发生在解压分配前）。
    Truncated(&'static str),
    /// payload_checksum（压缩流 sha256）不匹配。
    ChecksumMismatch,
    /// 未知 codec_id。
    UnknownCodec(u32),
    /// schema_digest 不匹配。
    DigestMismatch,
    /// 解压失败。
    Codec(CodecError),
    /// RXPL major=2 逻辑页解码失败（digest 篡改/布局破损确定性拒绝）。
    Logical(PageDecodeError),
}

impl std::fmt::Display for DiskV2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiskV2Error::BadMagic => write!(f, "RXPDv2 魔数不符"),
            DiskV2Error::UnsupportedVersion { major, minor } => {
                write!(f, "RXPDv2 版本不支持:{major}.{minor}")
            }
            DiskV2Error::BadHeader(s) => write!(f, "RXPDv2 header 损坏:{s}"),
            DiskV2Error::Truncated(s) => write!(f, "RXPDv2 截断:{s}"),
            DiskV2Error::ChecksumMismatch => write!(f, "RXPDv2 payload_checksum 不匹配"),
            DiskV2Error::UnknownCodec(id) => write!(f, "RXPDv2 未知 codec_id:{id}"),
            DiskV2Error::DigestMismatch => write!(f, "RXPDv2 schema_digest 不匹配"),
            DiskV2Error::Codec(e) => write!(f, "RXPDv2 codec:{e}"),
            DiskV2Error::Logical(e) => write!(f, "RXPDv2 逻辑页:{e:?}"),
        }
    }
}

impl std::error::Error for DiskV2Error {}

impl From<CodecError> for DiskV2Error {
    fn from(e: CodecError) -> Self {
        DiskV2Error::Codec(e)
    }
}

impl From<PageDecodeError> for DiskV2Error {
    fn from(e: PageDecodeError) -> Self {
        DiskV2Error::Logical(e)
    }
}

/// 加性映射行：仅 (RXPD,major=2)→(RXPL logical,major=2)。
///
/// v1 行 {(1)→(RXPM,1)} 由 `disk::mapping_allows` 承载（0-byte）；两行并存，
/// 未列组合一律拒（沿 RXS-0341 冻结映射表口径的加性扩展）。
pub fn mapping_allows_v2(disk_major: u16, logical_major: u16) -> bool {
    disk_major == DISK_MAJOR_V2 && logical_major == LOGICAL_MAJOR_V2
}

/// v2 冻结 schema_digest（域分离 preimage；目标面 = RXPL logical major=2 字面进
/// preimage，防与 v1 面互替）。
pub fn schema_digest_v2() -> [u8; 32] {
    let mut pre = Vec::with_capacity(48);
    pre.extend_from_slice(b"RXPD-SCHEMA-V2\0");
    put_u16(&mut pre, DISK_MAJOR_V2);
    put_u16(&mut pre, DISK_MINOR_V2);
    put_u32(&mut pre, FORMAT_ID);
    put_u16(&mut pre, HEADER_SIZE_V2);
    put_u32(&mut pre, CODEC_ID_RXPZ_LZ1);
    put_u32(&mut pre, CODEC_VERSION);
    // 目标面标识：payload = RXPL logical major=2 映像（映射行进 digest）。
    put_u16(&mut pre, LOGICAL_MAJOR_V2);
    sha256::digest(&pre)
}

/// 依赖页 digest（与 v1 同律：逐 u64 LE 拼接 sha256；单源复用 v1 实现）。
pub fn dependency_digest_v2(deps: &[u64]) -> [u8; 32] {
    crate::disk::dependency_digest(deps)
}

/// 编码磁盘页 v2：LogicalPageV2 → RXPD major=2 bytes（确定性；同输入同输出）。
pub fn encode_disk_page_v2(page: &LogicalPageV2) -> Vec<u8> {
    let image = encode_logical_page_v2(page);
    debug_assert!(image.len() <= STREAM_PAGE_SIZE as usize);
    let compressed = codec::compress(&image);
    let mut out = Vec::with_capacity(HEADER_SIZE_V2 as usize + compressed.len());
    out.extend_from_slice(&RXPD_MAGIC);
    put_u32(&mut out, FORMAT_ID);
    put_u16(&mut out, DISK_MAJOR_V2);
    put_u16(&mut out, DISK_MINOR_V2);
    out.push(ENDIAN_LE);
    out.push(0);
    put_u16(&mut out, HEADER_SIZE_V2);
    put_u32(&mut out, 0); // section_dir_count（v1 同律恒 0）
    put_u32(&mut out, CODEC_ID_RXPZ_LZ1);
    put_u32(&mut out, CODEC_VERSION);
    put_u64(&mut out, image.len() as u64);
    put_u64(&mut out, compressed.len() as u64);
    put_u64(&mut out, page.base.page_id);
    out.extend_from_slice(&schema_digest_v2());
    out.extend_from_slice(&sha256::digest(&compressed));
    out.extend_from_slice(&dependency_digest_v2(&page.base.dependency_page_ids));
    debug_assert_eq!(out.len(), HEADER_SIZE_V2 as usize);
    out.extend_from_slice(&compressed);
    out
}

/// 解码 RXPD major=2 → LogicalPageV2（拒录发生在解压/大分配前）。
pub fn decode_disk_page_v2(bytes: &[u8]) -> Result<LogicalPageV2, DiskV2Error> {
    // 消费前：至少读到 major。
    if bytes.len() < 12 {
        if bytes.len() >= 4 && bytes[0..4] != RXPD_MAGIC {
            return Err(DiskV2Error::BadMagic);
        }
        return Err(DiskV2Error::Truncated("header"));
    }
    if bytes[0..4] != RXPD_MAGIC {
        return Err(DiskV2Error::BadMagic);
    }
    let major = u16::from_le_bytes([bytes[8], bytes[9]]);
    let minor = u16::from_le_bytes([bytes[10], bytes[11]]);
    if major != DISK_MAJOR_V2 {
        return Err(DiskV2Error::UnsupportedVersion { major, minor });
    }
    if bytes.len() < HEADER_SIZE_V2 as usize {
        return Err(DiskV2Error::Truncated("header"));
    }

    let format_id = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if format_id != FORMAT_ID {
        return Err(DiskV2Error::BadHeader("format_id"));
    }
    if bytes[12] != ENDIAN_LE {
        return Err(DiskV2Error::BadHeader("endian"));
    }
    if bytes[13] != 0 {
        return Err(DiskV2Error::BadHeader("reserved0"));
    }
    let header_size = u16::from_le_bytes([bytes[14], bytes[15]]);
    if header_size != HEADER_SIZE_V2 {
        return Err(DiskV2Error::BadHeader("header_size"));
    }
    let section_dir_count = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    if section_dir_count != 0 {
        return Err(DiskV2Error::BadHeader("section_dir_count"));
    }
    let codec_id = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
    let codec_version = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    if codec_id != CODEC_ID_RXPZ_LZ1 {
        return Err(DiskV2Error::UnknownCodec(codec_id));
    }
    if codec_version != CODEC_VERSION {
        return Err(DiskV2Error::BadHeader("codec_version"));
    }
    let uncompressed_size = u64::from_le_bytes(bytes[28..36].try_into().unwrap()) as usize;
    let compressed_size = u64::from_le_bytes(bytes[36..44].try_into().unwrap()) as usize;
    let _logical_page_id = u64::from_le_bytes(bytes[44..52].try_into().unwrap());
    let mut schema = [0u8; 32];
    schema.copy_from_slice(&bytes[52..84]);
    let mut checksum = [0u8; 32];
    checksum.copy_from_slice(&bytes[84..116]);
    // dependency_digest 保留在 116..148，decode 不强制复核（编码侧写入；v1 同律）。

    if schema != schema_digest_v2() {
        return Err(DiskV2Error::DigestMismatch);
    }

    // 截断：在解压分配前检查（v1 同律）。
    let need = HEADER_SIZE_V2 as usize + compressed_size;
    if bytes.len() < need {
        return Err(DiskV2Error::Truncated("payload"));
    }
    if bytes.len() != need {
        return Err(DiskV2Error::BadHeader("trailing"));
    }
    // 防巨大分配：RXPL v2 逻辑页契约 ≤ STREAM_PAGE_SIZE（宽松顶 ×2 沿 v1）。
    if uncompressed_size > STREAM_PAGE_SIZE as usize * 2 {
        return Err(DiskV2Error::BadHeader("uncompressed_too_large"));
    }

    let payload = &bytes[HEADER_SIZE_V2 as usize..need];
    if sha256::digest(payload) != checksum {
        return Err(DiskV2Error::ChecksumMismatch);
    }

    let image = codec::decompress(payload, uncompressed_size)?;
    // 映射行核验：解压产物必须是 RXPL major=2（major 分发面；v1/v2 互不解析）。
    if image.len() < 12 {
        return Err(DiskV2Error::BadHeader("image_header"));
    }
    let logical_major = u16::from_le_bytes([image[8], image[9]]);
    if !mapping_allows_v2(major, logical_major) {
        return Err(DiskV2Error::UnsupportedVersion {
            major,
            minor: logical_major,
        });
    }
    Ok(decode_logical_page_v2(&image)?)
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
    use crate::logical_v2::V2ClusterExt;

    fn sample_page() -> LogicalPageV2 {
        let bounds = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let center = [0.1, 0.2, 0.3];
        let (qx, qy, qz) = quantize_center(center, bounds);
        LogicalPageV2 {
            base: LogicalPage {
                page_id: 7,
                flags: FLAG_ROOT,
                lod_level_min: 0,
                lod_level_max: 1,
                bounds,
                clusters: vec![
                    PageClusterRecord {
                        cluster_id: 0,
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
                    },
                    PageClusterRecord {
                        cluster_id: 1,
                        qx,
                        qy,
                        qz,
                        center,
                        radius: 2.0,
                        cone_axis: [0.0, 1.0, 0.0],
                        cone_cutoff: 1.0,
                        error: 1.0,
                        parent_error: f32::INFINITY,
                        vertex_offset: 0,
                        triangle_offset: 0,
                        vertex_count: 3,
                        triangle_count: 1,
                        level: 1,
                        group: 0,
                    },
                ],
                vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                indices: vec![0, 1, 2],
                dependency_page_ids: vec![3],
                dag_links: vec![(1, 0)],
            },
            ext: vec![
                V2ClusterExt::unskinned([-1.0; 3], [1.0; 3]),
                V2ClusterExt::unskinned([-2.0; 3], [2.0; 3]),
            ],
        }
    }

    /// roundtrip：encode→decode 全等 + 双编码逐字节相等 + 映像 ≤128KB 契约。
    #[test]
    fn v2_disk_roundtrip_byte_equal() {
        let page = sample_page();
        let a = encode_disk_page_v2(&page);
        let b = encode_disk_page_v2(&page);
        assert_eq!(a, b);
        assert_eq!(&a[0..4], b"RXPD");
        assert_eq!(a.len(), HEADER_SIZE_V2 as usize + (a.len() - HEADER_SIZE_V2 as usize));
        let back = decode_disk_page_v2(&a).unwrap();
        assert_eq!(back, page);
        assert_eq!(encode_disk_page_v2(&back), a);
    }

    /// 封套字段字面值锚定（major=2 / 同构 148B / logical_page_id / 依赖 digest 位）。
    #[test]
    fn v2_header_layout_literal() {
        let page = sample_page();
        let a = encode_disk_page_v2(&page);
        assert_eq!(u16::from_le_bytes([a[8], a[9]]), DISK_MAJOR_V2);
        assert_eq!(u16::from_le_bytes([a[10], a[11]]), DISK_MINOR_V2);
        assert_eq!(u32::from_le_bytes(a[4..8].try_into().unwrap()), FORMAT_ID);
        assert_eq!(u16::from_le_bytes([a[14], a[15]]), HEADER_SIZE_V2);
        assert_eq!(u64::from_le_bytes(a[44..52].try_into().unwrap()), 7);
        assert_eq!(&a[52..84], schema_digest_v2().as_slice());
        assert_eq!(&a[116..148], dependency_digest_v2(&[3]).as_slice());
    }

    /// 双编码压缩流逐字节相等（codec 确定性经磁盘面复核）。
    #[test]
    fn v2_compress_twice_byte_equal() {
        let page = sample_page();
        let a = encode_disk_page_v2(&page);
        let b = encode_disk_page_v2(&page);
        assert_eq!(a[HEADER_SIZE_V2 as usize..], b[HEADER_SIZE_V2 as usize..]);
    }

    /// v1 磁盘页进 v2 解码器 = 未知 major 拒（major 分发面；v1 面 0-byte 互证）。
    #[test]
    fn v1_disk_rejected_by_v2_decoder() {
        use crate::logical::LogicalPage as V1Page;
        use crate::memory::from_logical;
        let bounds = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let center = [0.1, 0.2, 0.3];
        let (qx, qy, qz) = quantize_center(center, bounds);
        let v1 = from_logical(&V1Page {
            page_id: 0,
            flags: FLAG_ROOT,
            lod_level_min: 0,
            lod_level_max: 0,
            bounds,
            clusters: vec![PageClusterRecord {
                cluster_id: 0,
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
            vertices: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            dependency_page_ids: vec![],
            dag_links: vec![],
        });
        let bytes = crate::disk::encode_disk_page(&v1, &[]);
        assert!(matches!(
            decode_disk_page_v2(&bytes),
            Err(DiskV2Error::UnsupportedVersion { major: 1, .. })
        ));
    }

    /// 四类损坏各自 fail-closed：bad magic / 截断（分配前）/ checksum 翻转 /
    /// 未知 codec / 未知 major / schema digest 篡改 / 尾字节。
    #[test]
    fn v2_corruption_fail_closed() {
        let page = sample_page();
        let good = encode_disk_page_v2(&page);
        // bad magic
        let mut b = good.clone();
        b[0] = b'X';
        assert_eq!(decode_disk_page_v2(&b), Err(DiskV2Error::BadMagic));
        // header 截断
        assert!(matches!(
            decode_disk_page_v2(&good[..100]),
            Err(DiskV2Error::Truncated("header"))
        ));
        // payload 截断（分配前拒）
        let cut = &good[..good.len() - 1];
        assert!(matches!(
            decode_disk_page_v2(cut),
            Err(DiskV2Error::Truncated("payload")) | Err(DiskV2Error::BadHeader("trailing"))
        ));
        // checksum 翻转（payload 区）
        let mut b = good.clone();
        let n = b.len();
        b[n - 1] ^= 0x01;
        assert_eq!(decode_disk_page_v2(&b), Err(DiskV2Error::ChecksumMismatch));
        // 未知 codec
        let mut b = good.clone();
        b[20] = 99;
        assert_eq!(decode_disk_page_v2(&b), Err(DiskV2Error::UnknownCodec(99)));
        // 未知 major
        let mut b = good.clone();
        b[8] = 9;
        assert!(matches!(
            decode_disk_page_v2(&b),
            Err(DiskV2Error::UnsupportedVersion { major: 9, .. })
        ));
        // schema digest 篡改
        let mut b = good.clone();
        b[52] ^= 0x01;
        assert_eq!(decode_disk_page_v2(&b), Err(DiskV2Error::DigestMismatch));
        // 尾字节
        let mut b = good.clone();
        b.push(0);
        assert_eq!(decode_disk_page_v2(&b), Err(DiskV2Error::BadHeader("trailing")));
    }

    /// 逻辑页 digest 篡改（压缩流整体重打包经合法 codec）：解码层确定性拒绝——
    /// 与 checksum 臂互补（本臂 checksum 重算合法、内容 digest 非法）。
    #[test]
    fn v2_logical_tamper_rejected() {
        let page = sample_page();
        let mut image = encode_logical_page_v2(&page);
        // 篡改 section_digest 字段（header 内 100..132）。
        image[100] ^= 0x01;
        let compressed = codec::compress(&image);
        let mut out = Vec::new();
        out.extend_from_slice(&RXPD_MAGIC);
        put_u32(&mut out, FORMAT_ID);
        put_u16(&mut out, DISK_MAJOR_V2);
        put_u16(&mut out, DISK_MINOR_V2);
        out.push(ENDIAN_LE);
        out.push(0);
        put_u16(&mut out, HEADER_SIZE_V2);
        put_u32(&mut out, 0);
        put_u32(&mut out, CODEC_ID_RXPZ_LZ1);
        put_u32(&mut out, CODEC_VERSION);
        put_u64(&mut out, image.len() as u64);
        put_u64(&mut out, compressed.len() as u64);
        put_u64(&mut out, 7);
        out.extend_from_slice(&schema_digest_v2());
        out.extend_from_slice(&sha256::digest(&compressed));
        out.extend_from_slice(&dependency_digest_v2(&[3]));
        out.extend_from_slice(&compressed);
        assert!(matches!(
            decode_disk_page_v2(&out),
            Err(DiskV2Error::Logical(_))
        ));
    }

    /// 映射表：v2 行存在且未列组合拒；v1 行（disk::mapping_allows）0-byte 互证。
    #[test]
    fn v2_mapping_row_additive() {
        assert!(mapping_allows_v2(2, 2));
        assert!(!mapping_allows_v2(1, 2));
        assert!(!mapping_allows_v2(2, 1));
        assert!(!mapping_allows_v2(2, 9));
        assert!(crate::disk::mapping_allows(1, 1));
        assert!(!crate::disk::mapping_allows(2, 2));
    }
}
