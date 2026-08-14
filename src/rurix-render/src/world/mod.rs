//! 世界分区与场景数据模型(G9.5 大世界×专项波;RFC-0025 D4 伞形 §4.A)。
//!
//! - [`partition`] = M110 世界分区数据模型与流送预算契约(RXS-0363:单一持久
//!   世界 schema + 2D cell + 三项流送预算契约逐帧 evidence + 预算违约注入必
//!   排队降级 + cell 四事件序列逐字 golden + Data Layer 掩码位只预留不接线 +
//!   大世界 soak hitch p99 measured 阈值)。
//! - [`hlod`] = M111 HLOD 运行时互斥切换面(RXS-0364:screen-size 阈值互斥
//!   切换 + 运行时零合并断言 + cell 事件总线接线 + 层级序列 golden)。
//! - [`atmosphere`] = M112 Froxel 大气前端(RXS-0365:Froxel 统一基础设施 +
//!   雾前端高度雾解析项 + weather map 资产化 + 时序上采样默认路径 + 计数面
//!   逐帧 evidence)。
//! - [`terrain`] = M116 地形(RXS-0367:chunk ≡ cell 禁第二套分格 + 全 compute
//!   LOD/剔除/缝合产 indirect draw + toroidal 环形窗口复用 + 零 SVT 依赖断言 +
//!   邻级 LOD 差>1 缝合裂缝 RED)。

pub mod atmosphere;
pub mod hlod;
pub mod partition;
pub mod terrain;
