//! rurix-pkg — Rurix 包管理子系统(M6.2,09 §7.1/§7.2)。
//!
//! 条款:spec/toolchain.md RXS-0089 ~ RXS-0094(rurix.toml 清单格式 / 三来源
//! path·git·archive 解析 / 依赖解析图与 feature additive-v1 合一 / rurix.lock
//! 精确解析图 / 内容树规范化 SHA-256 / vendor 与离线路径)。
//!
//! 纪律:**零外部依赖**(手写 TOML 子集解析 + 手写 SHA-256),纯函数、确定性
//! ——同一内容树在不同机器/时刻哈希一致,为 M6.3 三包离线重建逐字节复现门
//! (G-M6-1)铺底。本 crate 不直接发诊断,以 [`PkgError`] 类型化错误返回,由
//! rx CLI 映射 RX7005 ~ RX7009(沿用 M6.1 rx 以 inline eprintln 发 7xxx 码)。

pub mod content_tree;
pub mod error;
pub mod lock;
pub mod manifest;
pub mod resolve;
pub mod sha256;
pub mod toml;
pub mod vendor;

pub use error::PkgError;
