//! 通用页式流送运行时(报告6 P1–P2 §2.4「先建通用流送运行时,再接资源类型」;
//! RFC-0016 §4.G4)。
//!
//! 资源类型无关的页式驻留栈:128KB 页固定槽池 + LRU(root 页钉住常驻)+
//! 渲染反馈 `PageRequest` 驱动 + 每帧三预算(io/transcode/upload 各自独立计量、
//! 每帧重置、超支即停滚入下帧)+ pop-in 度量埋点。几何页与纹理页是注册进去
//! 的两种资源类型(本模块测试内提供合成实现),未来 SVT 页同栈留口(报告6
//! §2.5 P3 按需)。
//!
//! 纪律与边界:
//! - **staging 图外**:本运行时全部工作(IO 读取/转码/入池)发生在 render
//!   graph 图外;页数据经 staging 上传后,消费点以 acquire 屏障接入图内
//!   (RFC-0016 §4.0-3「流送屏障图外 acquire/release」)。host 波不实做屏障
//!   推导,只承诺语义边界。
//! - **转码留口**:[`PagedResource::transcode`] 默认恒等;真 KTX2/BasisU → BC
//!   转码器接入归 RD-037+ 存续(RFC-0016 §9.1 R-4 裁决:本期页 payload 为未
//!   压缩/简单打包档),接口按确定性转换设计(同输入同输出,host 单测逐字节
//!   锚定)。
//! - **度量埋点**:[`TickReport`] 各计数与 pop-in 统计只进 evidence 不进硬门
//!   (RFC-0016 §4.0-4 P-09 口径)。
//! - 冻结契约 [`PageRequest`]/[`StreamingBudget`]/[`STREAM_PAGE_SIZE`] 单源 =
//!   [`crate::graph::types`],本模块只消费不重定义(G5_PLAN §2)。

mod engine;
mod feedback;
mod pool;
mod resource;

pub mod cluster;
pub mod svt;

pub use engine::{StreamingEngine, TickReport};
pub use feedback::{FEEDBACK_BASE_GEOMETRY_LOD, FEEDBACK_BASE_TEXTURE_MISS, FeedbackBuilder};
pub use pool::{InsertOutcome, PagePool};
pub use resource::PagedResource;

// 冻结契约便利再导出(单源仍在 graph::types,此处不新增定义)。
pub use crate::graph::types::{PageRequest, STREAM_PAGE_SIZE, StreamingBudget};
