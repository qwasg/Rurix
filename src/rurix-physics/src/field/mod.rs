//! G9.2 M122 Gameplay Field 系统(RFC-0024 §4.B,R-3/R-7/R-10 🔒;骨架期)。
//!
//! 冻结面(判据事实源 = `G9_ACCEPTANCE_MAP.md` M122 行):
//! - **三层解耦**:场定义层(Field Nodes,`FieldNodeKind` 六基元)→
//!   作用对象层(M121 `PhysicsParticleRef` 寻址)→ 目标语义层
//!   (`FieldPhysicsType` 首期八枚举)。三层各自 canonical 序列化 + digest,
//!   合成 `FieldDef::digest`(图 schema 版本化 + cook 确定性,承 RFC-0021 §5.1)。
//! - **八枚举首期冻结**:`FieldPhysicsType::{LinearForce, Strain, Velocity,
//!   Torque, Sleeping, Disabled, CollisionGroup, Buoyancy}`;非法枚举
//!   fail-closed(`FieldError::IllegalPhysicsType`);扩枚举须先经两个真实
//!   用户(destruction damage + 浮力)验证(RFC-0024 §4.B1 冻结句)。
//! - **三生命周期**:Transient(不进 journal,结果经命令规范化进
//!   journal)/ Construction(进 cooked artifact digest)/ Persistent
//!   (跨 tick,**注册/注销/参数变更全部写 command journal** 且参与
//!   `semantic_state_hash`,replay 逐 tick hash 一致为硬门)。
//! - **过滤一等公民**:`FieldFilter = (object_state_mask × domain_mask ×
//!   layer_mask × explicit_include/exclude)`;**默认空集匹配 = 无影响**,
//!   拒绝「默认全影响」语义;filter 是场定义的一部分,进 digest。
//! - **World-Field 唯一出口**:GpuScene 只读 buffer 提交口(见
//!   `crate::field::world_egress`;渲染侧零回写,R-10 🔒)。
//!
//! 骨架期范围(诚实登记,完整期归 --phase g9.6):
//! - 场求值 = 确定性标量采样(基元函数解析求值);力场积分/求解器耦合
//!   归完整期;骨架期断言面 = schema/digest/journal/replay/过滤/出口纪律。
//! - `WorldFieldSampleSet` 时间域归属 `RenderFrameId` + `FrameDomainMap`
//!   显式映射(R-4 🔒)= 类型层登记(record 结构),渲染侧消费归完整期。
//! - noise 基元骨架期 = 确定性 hash-noise(整数格点);curve-driven =
//!   分段线性;analytic-surface 预留(浮力水面函数,M124 共用求值管线)。

pub mod capture_merge;
pub mod couple;
pub mod def;
pub mod eval;
pub mod filter;
pub mod journal;
pub mod lifecycle;
pub mod registry;
pub mod world_egress;

pub use def::{
    AnalyticSurfacePrimitive, FIELD_SCHEMA_ID, FIELD_SCHEMA_VERSION, FieldDef, FieldError,
    FieldNode, FieldNodeKind, FieldPhysicsType,
};
pub use filter::{FieldFilter, ObjectStateMask};
pub use journal::{FieldJournal, FieldJournalCommand, FieldJournalTick};
pub use lifecycle::FieldLifecycle;
pub use registry::{FieldRegistry, RegisteredField};
pub use world_egress::{WorldFieldBuffer, WorldFieldSampleSet, WorldFieldSubmitter};
