//! 材质与 GPU 场景(报告6 P0;RFC-0016 章 G 前半)。
//!
//! 单层 principled 材质闭合(32B 定长,pack/unpack 往返单测)+ GPU scene
//! 扁平化实例表 + PSO precache(预测式预编译;运行时编译告警计数,验收
//! 归零)。闭合布局冻结在 [`crate::graph::types::MaterialClosure`]。
//! [`side_table`] = G9.5 起 RFC-0025 §4.L 🔒 修订行登记的资产化侧表扩展
//! 通道(M115 皮肤 Burley profile / M114 毛发 Marschner 参数集,按材质槽 ID
//! 索引,32B 布局 0-byte,缺省侧表 ≡ 既有输出逐位不变,禁静默扩)。

pub mod closure;
pub mod pso_cache;
pub mod side_table;
// G22.2 波（RFC-0039）：Substrate 类双层 slab 能量守恒闭合 host 参考臂——
// RD-041 slab 分层材质分项语义参考面（closure 单层面 0-byte 不接线）。
pub mod slab;
pub mod table;

pub use closure::{
    EMISSIVE_RGBE_EXP_BIAS, MATERIAL_FLAG_ALPHA_BLEND, MATERIAL_FLAG_DOUBLE_SIDED, MaterialParams,
    unpack,
};
pub use pso_cache::{
    BlendMode, CullMode, PassShaderTemplate, PsoCache, PsoDesc, predict_precache_list,
};
pub use table::MaterialTable;
