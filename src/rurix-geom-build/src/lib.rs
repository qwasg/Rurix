//! rurix-geom-build — 离线几何构建器(G5,RFC-0016 章 C;报告1 P0)。
//!
//! 管线:三角网格 → meshlet 化(≤128 tri / ≤64 vert,贪心邻接生长)→ 分组
//! 简化层级 DAG(组边界锁定裂缝保护;误差包围球 parent_error ≥ error 单调)
//! → [`rurix_render::graph::types::ClusterRecord`] 序列化(预留 page_id)。
//! 附 CPU 参照剔除器(视锥/背面锥逐簇蛮力),作 GPU 剔除 device 对拍金标准。
//!
//! 模块:
//! - [`mesh`]:输入网格模型 + 立方体/UV 球/平面生成器 + 共享边邻接;
//! - [`cluster`]:贪心簇化 + Ritter 包围球 + meshopt 口径背面锥;
//! - [`dag`]:Morton 分组 → 边界锁定边收缩 → 再簇化的层级 DAG(误差单调);
//! - [`serialize`]:RXGB 二进制格式(手写 LE,零依赖,页表字段预留);
//! - [`cull_ref`]:CPU 参照剔除器(接口冻结 = GPU 剔除对拍契约)。
//!
//! 与工业实现的已知差距(P0 取舍,正确性不变量不受影响):
//! - 分组用簇邻接共享边加权贪心,非 meshopt_partitionClusters 的完整图分区;
//! - **默认**简化用最短边贪心收缩(端点保持),非 QEM 最优位置收缩——
//!   G31+ #66 起 [`qem`] 提供 QEM 加性第二实现(经 [`dag::SimplifyKind::Qem`]
//!   显式选用;默认面 0-byte,m90 DAG digest golden 不漂移,替换事实源走
//!   #66 立项对照臂纪律);
//! - link-condition 拓扑校验未做;QEM 腿含 fold-over 拒绝(法线翻转检测),
//!   最短边腿极端输入可能产生 fold-over(误差上界仍保守);
//! - Ritter 包围球非最优球(偏大 ~20% 以内,剔除保守方向);
//! - 跨层顶点焊接按精确位置相等,「不同 id 同位置」的输入顶点会被合并
//!   (内置生成器已规避;外接网格建议先焊接);
//! - G31+ #96 属性保持简化为加性第二链([`dag::build_dag_attrs`] /
//!   [`qem::simplify_free_mesh_attrs`]):位置 QEM 主导 + 收缩点线段插值,
//!   非 meshopt 属性加权四次型(最优属性求解留后续质量档);UV 接缝顶点
//!   保守锁定(两侧逐位保持,不做位置重映射协动简化)。

#![forbid(unsafe_code)]

pub mod cluster;
pub mod cull_ref;
pub mod dag;
pub mod lod_bounds;
pub mod mesh;
pub mod qem;
pub mod serialize;
mod vecmath;

pub use cluster::{Cluster, MAX_TRIS, MAX_VERTS, clusterize};
pub use cull_ref::{CullStats, CullView, Mat4, cull_clusters, lod_cut_select};
pub use dag::{
    ClasBakeInput, ClusterDag, ClusterDagAttrs, ClusterDagV2, ClusterSkinMeta, DagAsset,
    DagBuildParams, DagError, DagLevel, DagNode, MAX_BONE_INFLUENCES, SimplifyKind, SkinWeights,
    SkinnedClusterData, build_asset_dag, build_asset_dag_kind, build_asset_dag_params, build_dag,
    build_dag_attrs, build_dag_kind, build_dag_params, build_dag_v2, canonical_bytes,
    clas_bake_input_of, derive_skin_metadata, skinned_cluster_runtime_data,
    validate_monotonicity,
};
pub use mesh::{AttrMeshError, AttrTriMesh, TriMesh, TriMeshAttrs, build_face_adjacency};
pub use serialize::{RXGB_VERSION, RxgbError, read_dag, write_dag};

// 冻结契约单源转引(64B 簇记录与簇上限;rurix-render graph::types)。
pub use rurix_render::graph::types::{ClusterRecord, MAX_TRIS_PER_CLUSTER, MAX_VERTS_PER_CLUSTER};
