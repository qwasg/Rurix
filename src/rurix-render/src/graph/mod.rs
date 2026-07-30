//! 渲染调度底座(报告5 P0–P2;RFC-0016 章 A)。
//!
//! 形态:Frostbite 式声明式——应用指定 pass 线性序 + 逐 pass 显式读写声明;
//! 编译四趟(剔除/生命周期/屏障/车道)独立于声明面;EB 三轴屏障为后端中立
//! 内部规范形式;transient 池化别名;编译期校验(漏声明/越期句柄/读写冲突
//! 确定性拒);图 dump JSON。冻结契约见 [`types`]。
//!
//! 模块划分:
//! - [`graph`]:声明面([`RenderGraph`] / [`CmdRecorder`],execute 闭包与声明分离)
//! - [`resources`]:资源节点与 transient 池分桶
//! - [`sync`]:EB 三轴映射 + 逐资源 AccessTracker(屏障推导内核)
//! - [`compile`]:四趟编译、五类校验与 [`CompiledGraph`] 产物(下游唯一消费面)
//! - [`transient`]:transient 池(区间着色别名 + 峰值审计)
//! - [`dump`]:图 dump JSON(观测/CI;报告5 §5 验证方法第四层)

pub mod compile;
pub mod dump;
#[allow(clippy::module_inception)] // 任务规定的文件布局:graph/graph.rs 为声明面本体
pub mod graph;
pub mod resources;
pub mod sync;
pub mod transient;
pub mod types;

pub use compile::{CompileOptions, CompiledGraph, CompiledPass, CompiledResource, GraphError};
pub use graph::{CmdRecorder, CommandKind, CommandLog, PassExecute, RecordedCommand, RenderGraph};
pub use sync::{access_mask_of, image_layout_of, sync_stage_of};
pub use transient::TransientPool;
