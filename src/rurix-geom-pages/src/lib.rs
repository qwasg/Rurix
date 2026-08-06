//! rurix-geom-pages — 几何页格式（G8.3；RFC-0020 §4.9；spec/geometry_pages.md）。
//!
//! M01：逻辑页 RXPL（未压缩 builder artifact）encode/decode。
//! M04（后续）：磁盘/内存双 ABI + RXPZ-LZ1。
//!
//! 依赖：仅 [`rurix_pkg`]（sha256）。`#![forbid(unsafe_code)]`。

#![forbid(unsafe_code)]

pub mod logical;

pub use logical::{
    FLAG_ROOT, FORMAT_ID, HEADER_SIZE, LOGICAL_MAJOR, LOGICAL_MINOR, PACKING_ALGO_ID,
    LogicalPage, PageClusterRecord, PageDecodeError, RECORD_SIZE, RXPL_MAGIC, STREAM_PAGE_SIZE,
    decode_logical_page, encode_logical_page, quantize_center, schema_digest,
};
