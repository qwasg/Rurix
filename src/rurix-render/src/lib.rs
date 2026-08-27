//! rurix-render — Rurix 原生引擎渲染器库(G5,RFC-0016)。
//!
//! 依渲染器调研七报告 P0–P2 主线落地(报告号 ↔ 模块):
//! - 报告5 调度/RenderGraph/异步计算 → [`graph`]
//! - 报告1 几何/Nanite 类虚拟化几何 → [`geometry`]
//! - 报告3 阴影/VSM → [`shadow`]
//! - 报告2 GI/屏幕探针 → [`gi`]
//! - 报告4 实时光追/AS 管理 → [`rt`]
//! - 报告6 材质/场景/流送 → [`material`] + [`streaming`]
//! - 报告7 时域重建/超分 → [`temporal`]
//! - G9.5 大世界数据模型(RFC-0025 D4 伞形 §4.A) → [`world`]
//! - G9.5 显示管线 view transform 插件面与交换链路径(RFC-0025 §4.I,RXS-0369)
//!   → [`display`]
//! - G9.5 OIT benchmark harness 测量面(RFC-0025 §4.K,RXS-0371,仅测量不定档)
//!   → [`oit`]
//! - G31+ 波 C 设备兼容矩阵与能力降级链系统化(G31_PLUS §5 #50) →
//!   [`capability_matrix`](六链降级闭集 fail-closed 裁决 + 登记表 digest)
//! - G35 GPU 粒子系统(RFC-0049,对标并超越 UE5 Niagara 五轴) → [`particles`]
//!   (确定性 GPU 粒子 host 金标准:分段稳定 scan/排序基元/池与发射;
//!   device 面 = kernels/g35_*.rx)
//!
//! 架构纪律(RFC-0016 跨章一致性约定):
//! - 渲染器是库不进语言(06 §8.3);host 侧全 safe(`forbid(unsafe_code)`)纯 Rust 可单测;
//!   device 执行经 rurix-rt vk 底座,`vulkan` feature gate(uc04-demo real-shim 先例)。
//! - 跨帧资源(TAA 历史/VSM 页表/GI 探针历史)一律外部资源 import,不入 transient;
//!   流送屏障图外 acquire/release。
//! - 冻结契约类型单源 = [`graph::types`],子系统间只经该契约交换数据。

#![forbid(unsafe_code)]

pub mod capability_matrix;
pub mod display;
pub mod geometry;
pub mod gi;
pub mod graph;
pub mod material;
pub mod oit;
pub mod particles;
pub mod rt;
pub mod shadow;
pub mod streaming;
pub mod temporal;
pub mod world;
