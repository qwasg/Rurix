//! rurix-asset — G8.3 资产管线(RFC-0020)。
//!
//! - M01：[`geom_build`]（`ClusterDag → Vec<LogicalPage>` + `rxgb_to_pages`）
//! - M79：[`canon`] / [`graph`] / [`cook`] / [`verify`]（AP-CANON/GRAPH + 双构建）
//! - M81：[`schema`]（AP-SCHEMA）+ [`gltf`]（AP-GLTF）+ `rxcook import-gltf`
//! - M83：[`texture`] / [`bcdec`] / [`ktx2`]
//!
//! 后续批次追加 DDC。lib 部分 `#![forbid(unsafe_code)]`。

#![forbid(unsafe_code)]

pub mod bcdec;
pub mod canon;
pub mod cook;
pub mod ddc;
pub mod error;
pub mod geom_build;
pub mod geom_build_v2;
pub mod gltf;
pub mod graph;
pub mod hlod;
pub mod ktx2;
pub mod schema;
pub mod texture;
pub mod verify;

pub use error::{AssetError, ErrorKind, Result};
pub use geom_build::{PackError, concatenate_pages, pack_cluster_dag, rxgb_to_pages};
pub use geom_build_v2::{
    check_v2_pages_within_contract, concatenate_pages_v2, logical_v2_from_parts,
    pack_cluster_dag_v2,
};
pub use texture::{CookProfile, CookReport, TextureSemantics, cook_texture};
