//! 虚拟化几何运行时(报告1 P1–P2;RFC-0016 章 C)。
//!
//! GPU 实例/簇两级剔除(视锥/背面锥;HZB 两阶段 P3 预留)→ 64 位 VisBuffer
//! (depth30|cluster27|tri7,SW atomicMax u64 / HW 间接绘制双路)→ 材质
//! classify/resolve。簇记录与位格式冻结在 [`crate::graph::types`];离线构建
//! 在 `rurix-geom-build` crate。本模块 host 侧提供剔除/光栅的 CPU 参照实现
//! (device 对拍金标准)与 GPU 场景表装配。
//!
//! 本波(G5 W2-G host 波)新增:[`cull`](两级剔除 + LOD cut + SW/HW 分箱)、
//! [`visbuffer`](CPU 光栅参考,reverse-Z 30 位量化 + atomicMax 语义)、
//! [`material_pass`](tile×材质 classify + 16 位窄缓冲 resolve)、
//! [`gpu_layout`](GPU 数据编组字节面);device shader 由 W3 统一接线,以
//! 上述 host 参考为逐簇/逐像素对拍金标准。

pub mod cull;
pub mod gpu_layout;
// G20.2 波（RFC-0037）：HZB 层级深度金字塔遮挡剔除 host 参考臂——兑现本头注
// 「HZB 两阶段 P3 预留」第一阶段 host 面（保守零假阳性硬不变量 + 双跑位级）。
pub mod gpu_scene;
pub mod hzb;
pub mod material_pass;
// G9.3 波:蒙皮 host 参照(M92 RXS-0353)+ VisibleClusterSet 单源真相
// (M93 RXS-0350 / M95 RXS-0352)。M92 device 腿:蒙皮 kernel 手编 SPV 构建
// (纯 host 数据构造)+ M95 device 腿 SW/HW 双腿 SPV 构建。
pub mod skin_kernel;
pub mod skinning;
pub mod visbuffer;
pub mod visbuffer_swhw_spv;
pub mod visible_cluster_set;
