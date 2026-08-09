//! rurix-geom-pages — 几何页格式（G8.3；RFC-0020 §4.9；spec/geometry_pages.md）。
//!
//! M01：逻辑页 RXPL（未压缩 builder artifact）encode/decode。
//! M04：磁盘 RXPD / 内存 RXPM 双 ABI + RXPZ-LZ1 + 整数域展开 digest。
//!
//! 依赖：仅 [`rurix_pkg`]（sha256）。`#![forbid(unsafe_code)]`。

#![forbid(unsafe_code)]

pub mod codec;
pub mod disk;
pub mod expand;
pub mod logical;
pub mod memory;

pub use codec::{CODEC_ID_RXPZ_LZ1, CODEC_VERSION, CodecError, compress, decompress};
pub use disk::{
    DISK_MAJOR, DISK_MINOR, DiskError, RXPD_MAGIC, decode_disk_page, dependency_digest,
    encode_disk_page, mapping_allows,
};
pub use expand::{expand_memory_page, expand_u32_count, expanded_digest};
pub use logical::{
    FLAG_ROOT, FORMAT_ID, HEADER_SIZE, LOGICAL_MAJOR, LOGICAL_MINOR, LogicalPage, PACKING_ALGO_ID,
    PageClusterRecord, PageDecodeError, RECORD_SIZE, RXPL_MAGIC, STREAM_PAGE_SIZE,
    decode_logical_page, encode_logical_page, quantize_center, schema_digest,
};
pub use memory::{
    MEMORY_MAJOR, MEMORY_MINOR, MemCluster, MemoryError, MemoryPage, RXPM_MAGIC,
    decode_memory_page, encode_memory_page, from_logical,
};
