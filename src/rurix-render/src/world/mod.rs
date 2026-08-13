//! 世界分区与场景数据模型(G9.5 大世界×专项波;RFC-0025 D4 伞形 §4.A)。
//!
//! - [`partition`] = M110 世界分区数据模型与流送预算契约(RXS-0363:单一持久
//!   世界 schema + 2D cell + 三项流送预算契约逐帧 evidence + 预算违约注入必
//!   排队降级 + cell 四事件序列逐字 golden + Data Layer 掩码位只预留不接线 +
//!   大世界 soak hitch p99 measured 阈值)。

pub mod partition;
