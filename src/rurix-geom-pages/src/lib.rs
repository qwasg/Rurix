//! rurix-geom-pages — 几何页格式（G8.3；RFC-0020 §4.9；spec/geometry_pages.md）。
//!
//! M01：逻辑页 RXPL（未压缩 builder artifact）encode/decode。
//! M04：磁盘 RXPD / 内存 RXPM 双 ABI + RXPZ-LZ1 + 整数域展开 digest。
//! M91：RXPL **major=2** 深化段（簇误差/包围球随 v1 记录面 + 骨骼元数据 +
//! CLAS 输入段；RXS-0344 / spec/virtual_geometry.md RXS-0345）+ major 分发共存。
//!
//! 依赖：仅 [`rurix_pkg`]（sha256）。`#![forbid(unsafe_code)]`。

#![forbid(unsafe_code)]

pub mod codec;
pub mod disk;
pub mod expand;
pub mod expand_v2;
pub mod logical;
pub mod logical_v2;
pub mod memory;

pub use codec::{CODEC_ID_RXPZ_LZ1, CODEC_VERSION, CodecError, compress, decompress};
pub use disk::{
    DISK_MAJOR, DISK_MINOR, DiskError, RXPD_MAGIC, decode_disk_page, dependency_digest,
    encode_disk_page, mapping_allows,
};
pub use expand::{expand_memory_page, expand_u32_count, expanded_digest};
pub use expand_v2::{expand_logical_page_v2, expand_u32_count_v2, expanded_digest_v2};
pub use logical::{
    FLAG_ROOT, FORMAT_ID, HEADER_SIZE, LOGICAL_MAJOR, LOGICAL_MINOR, LogicalPage, PACKING_ALGO_ID,
    PageClusterRecord, PageDecodeError, RECORD_SIZE, RXPL_MAGIC, STREAM_PAGE_SIZE,
    decode_logical_page, encode_logical_page, quantize_center, schema_digest,
};
pub use logical_v2::{
    CLAS_RECORD_SIZE, HEADER_SIZE_V2, LOGICAL_MAJOR_V2, LOGICAL_MINOR_V2, LogicalPageV2,
    SKIN_RECORD_SIZE, V2ClusterExt, decode_logical_page_any, decode_logical_page_v2,
    encode_logical_page_v2, encoded_len_v2, schema_digest_v2,
};
pub use memory::{
    MEMORY_MAJOR, MEMORY_MINOR, MemCluster, MemoryError, MemoryPage, RXPM_MAGIC,
    decode_memory_page, encode_memory_page, from_logical,
};
