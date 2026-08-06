//! rurix-physics — Rurix 引擎物理库(G6.2,RFC-0017 §4.A 物理库边界章)。
//!
//! 依 RFC-0017 §4.A 冻结接口落地(§4.0-3:RFC Approved 字面冻结,实现不得漂移):
//! - [`PhysicsWorld`] 固定步 `step(dt_fixed)`(accumulator 在宿主;变步长 → 确定性
//!   `Err(FixedStepMismatch)`);后端 [`BackendKind::Jolt`] 生产默认(feature `jolt`,
//!   经 rurix-physics-sys vendor 构建)/ [`BackendKind::Rapier`] 快路径第二后端
//!   (feature `rapier`,默认 off,G6.4 §4.D;同 `PhysicsWorld` 抽象同 API);
//! - [`BodyId`]/[`ShapeId`] 不透明句柄(index 32b + generation 32b,generation
//!   单调递增、耗尽槽位退休;index 池耗尽 → `Err(PoolExhausted)`);
//! - [`PhysicsTransform`] `{ translation, rotation /* xyzw quat */ }` 为渲染唯一
//!   桥接输入,库内不出现 4×4/3×4 矩阵类型(单源,P-11);
//! - [`ContactEvent`] 有界 ring + step 结束边界归一化(规范序排序去重,§4.A5);
//! - [`QueryRay`]/[`QueryHit`]/[`QueryShape`]/[`OverlapHit`] step 外并发查询,
//!   cast 结果 `(t, BodyId)` 规范序 = 确定性面(§4.A4 C-2);
//! - [`SyncBudget`] 每帧重置,三轴耗尽 → 对应面确定性截断 + 饱和计数(§4.A6)。
//! - [`bridge`] 渲染合流桥(G6.3,RFC-0017 §4.B):[`PhysicsBridge`] 物理 → `GpuScene`
//!   单向变换同步 + [`StreamingBridge`] 流送批插移除(`RemovalReceipt` 先卸后放)。
//!
//! **快路径 ≠ 性能/稳定性默认**(§4.D4):Rapier 路径价值 = 纯 Rust/无 CMake
//! CI 面与第二实现交叉验证;生产默认 = Jolt(G6_PLAN §0.1)。不替换默认、不做
//! 性能宣称(P-09:实测数字写 evidence)。
//!
//! 架构纪律(RFC-0017 §4.0 跨章一致性约定):
//! - 物理是引擎库不进语言(06 §8.3);本 crate 全 safe(`forbid(unsafe_code)`),
//!   FFI/unsafe 唯一集中地 = `rurix-physics-sys`(§4.C);
//! - `--no-default-features` 构建零 C++ 依赖恒绿:无后端编译,`PhysicsWorld::new`
//!   全路径确定性 `Err(BackendNotCompiled)`,不静默回退、不 panic(P-01);
//! - 宿主只握不透明句柄,永不见原生 Jolt/Rapier 指针(§4.C4 审计判据);
//! - 归一化/预算/arena 为纯函数/纯类型(模块 events/budget/arena),后端无关可单测。

#![forbid(unsafe_code)]
// 无后端构建档(非默认):世界不可构造,后端消费路径仅经单测触达——豁免 dead_code
// (镜像 rurix-physics-sys `#![cfg_attr(not(test), allow(dead_code))]` 先例);
// default(= jolt)档保持全量 dead_code 检查。
#![cfg_attr(not(feature = "jolt"), allow(dead_code))]
// 零后端档(G6.4 起):统一分派的 match 臂全 cfg 出局,绑定/后续语句不可达——
// 窄域豁免仅该档;jolt/rapier/双后端档全量 lint 维持(G6.4 落地后仍零告警)。
#![cfg_attr(
    not(any(feature = "jolt", feature = "rapier")),
    allow(unused_variables, unreachable_code)
)]

mod arena;
pub mod bridge;
mod budget;
#[cfg(feature = "physics-capture")]
pub mod capture;
#[cfg(feature = "physics-character")]
pub mod asset;
#[cfg(feature = "physics-character")]
pub mod character;
#[cfg(feature = "physics-destruction")]
pub mod destruction;
#[cfg(feature = "physics-cloth")]
pub mod cloth;
#[cfg(feature = "physics-vehicle")]
pub mod vehicle;
mod error;
mod events;
mod id;
#[cfg(feature = "network-physics")]
pub mod net;
mod order;
#[cfg(feature = "rapier")]
mod rapier;
mod types;
mod world;

pub use bridge::{
    FrameSyncReport, MotionHint, PageKey, PhysicsBridge, RemovalReceipt, StreamingBridge,
    compose_transform_3x4,
};
pub use budget::SyncBudget;
pub use error::PhysicsError;
pub use id::{BodyId, ShapeId};
pub use types::{
    BackendKind, BodyDesc, BodyKind, BodySemantic, ContactEvent, ContactPhase, MassProps,
    OverlapHit, PhysicsTransform, QueryHit, QueryRay, QueryShape, ShapeDesc, StepStats, WorldDesc,
};
pub use world::{BudgetSaturation, PhysicsWorld};
