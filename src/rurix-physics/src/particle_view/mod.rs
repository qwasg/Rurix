//! G9.2 M121 统一 physics particle view(RFC-0024 §4.A;骨架期)。
//!
//! 冻结面(RFC-0024 §4.A,判据事实源 = `G9_ACCEPTANCE_MAP.md` M121 行):
//! - [`PhysicsParticleRef`] = `(domain, stable_id, element_index)` 名义类型;
//!   `domain ∈ {RigidBody, ClothVertex, DestructionChunk, RagdollNode, CharacterInner}`;
//!   `stable_id` 复用 generation 语义(RFC-0021 §3.4),**绝不暴露 arena index**
//!   (ref 无 index 读取器;域内解析经 generation 校验的 arena 门禁,失效句柄
//!   确定性 `Err(NoSuchParticle)`,P-01 不悬垂)。
//! - 每域实现 [`ParticleAdapter`] trait;**写路径只允许 impulse/force 语义,
//!   不允许直接改写 transform**——类型层结构性保证:trait 不提供任何
//!   transform/position 直写方法(纪律 1 单向事实源 0-byte;渲染桥仍只读
//!   已提交变换)。
//! - 视图只覆盖 CPU 权威世界内的对象;GPU 副轨粒子不进本抽象。
//!
//! 骨架期边界(诚实登记,非完整语义;禁 stub 冒充运行时):
//! - **RagdollNode 域**:G8 终态 ragdoll 运行时面缺失(M69 只落
//!   `asset::PhysicsAsset` 资产层 + bone→body 映射,无运行时 ragdoll 实例
//!   类型);骨架期 adapter = 资产层只读视图(`set_force_impulse` 确定性
//!   `Err(SchemaOnlyAdapter(RagdollNode))`,读面 = 资产静态数据);
//!   运行时权威 adapter 归 --phase g9.6 完整期。
//! - **ClothVertex 域**:M72 `ClothSolver::positions` 语义为演示轨道
//!   (demo);adapter 按现状接线并登记 `demo_track=true`;生产布料顶点轨
//!   接真实 XPBD 状态后 0-byte 适配面。
//! - 名义类型跨域/同域别名混用的编译期隔离断言 =
//!   `tests/particle_view_isolation.rs`(探针源期望编译失败,门脚本以
//!   typeck 红绿双证;不引入 dev-dependency)。
//! - M68 damage journal 迁移器 = 加性迁移(源面 0-byte),见 `migrate` 子模块。

use std::fmt;

use crate::capture::canonical::CaptureError;
use crate::id::BodyId;

/// 五域枚举(RFC-0024 §4.A 冻结字面;`ALL` 为门脚本的完备性枚举源)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParticleDomain {
    /// 刚体(`PhysicsWorld` BodyId 域)。
    RigidBody,
    /// 布料顶点(M72 cloth;骨架期 = demo 轨道,见模块文档)。
    ClothVertex,
    /// 破坏碎块(M68 chunk 稳定 ID 域)。
    DestructionChunk,
    /// Ragdoll 节点(M69 资产层;骨架期 = 资产只读视图)。
    RagdollNode,
    /// Character 内部状态(M71 角色)。
    CharacterInner,
}

impl ParticleDomain {
    /// 冻结五域完备集(顺序 = 声明序,canonical 面)。
    pub const ALL: [ParticleDomain; 5] = [
        ParticleDomain::RigidBody,
        ParticleDomain::ClothVertex,
        ParticleDomain::DestructionChunk,
        ParticleDomain::RagdollNode,
        ParticleDomain::CharacterInner,
    ];

    /// canonical 名称(canonical_json / journal 面唯一合法字面)。
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::RigidBody => "RigidBody",
            Self::ClothVertex => "ClothVertex",
            Self::DestructionChunk => "DestructionChunk",
            Self::RagdollNode => "RagdollNode",
            Self::CharacterInner => "CharacterInner",
        }
    }

    /// 自 canonical 名还原;未知名 fail-closed(非法枚举旁路 = 骨架期 RED 臂面)。
    pub fn from_canonical_name(s: &str) -> Option<Self> {
        match s {
            "RigidBody" => Some(Self::RigidBody),
            "ClothVertex" => Some(Self::ClothVertex),
            "DestructionChunk" => Some(Self::DestructionChunk),
            "RagdollNode" => Some(Self::RagdollNode),
            "CharacterInner" => Some(Self::CharacterInner),
            _ => None,
        }
    }
}

/// 五域域句柄(名义类型隔离的执行体)。
///
/// 每域一个独立类型,**不可跨域互转**(无 From/Borrow;域错误配在类型面消灭)。
/// 元组字段 `pub(crate)` = 稳定 ID 位表示;稳定 ID 从 arena index 派生但
/// **类型面无 index 读取器**——「绝不暴露 arena index」由本模块唯一构造口
/// 结构性执行。
macro_rules! domain_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub(crate) u64);

        impl $name {
            /// 自位表示构造(journal/canonical 还原口;**位表示非 arena
            /// index**——域内解析侧做存活校验,伪造位表示在解析侧
            /// fail-closed)。
            pub fn from_bits(bits: u64) -> Self {
                Self(bits)
            }

            /// u64 位表示(journal/canonical 边界唯一出口;非 arena index)。
            pub fn to_bits(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{:#x}", stringify!($name), self.0)
            }
        }
    };
}

domain_id! {
    /// 刚体稳定 ID(由 `BodyId` 位表示派生;generation 语义经 arena 门禁)。
    RigidBodyStableId
}
domain_id! {
    /// 布料稳定 ID(布料资产 stable 位表示)。
    ClothStableId
}
domain_id! {
    /// 碎块稳定 ID(chunk 字符串 ID 的 digest 位表示)。
    ChunkStableId
}
domain_id! {
    /// Ragdoll 资产稳定 ID(资产 digest 派生)。
    RagdollAssetStableId
}
domain_id! {
    /// Character 稳定 ID(character_id 位表示)。
    CharacterStableId
}

/// chunk 字符串 ID → 稳定位表示(sha256 截位 digest,**非 arena index**;
/// journal/canonical 面只过位表示)。
pub(crate) fn chunk_stable_bits(chunk_id: &str) -> u64 {
    let d = rurix_pkg::sha256::digest(chunk_id.as_bytes());
    u64::from_le_bytes([d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]])
}

/// 统一 physics particle ref 名义类型(RFC-0024 §4.A 冻结三元组)。
///
/// 名义隔离:`domain` 变体载荷为各域独立句柄类型——不同域的 ref 在类型面
/// 不可互换;同域 `stable_id` 只经本模块构造口派生,调用方无法从 arena
/// index 伪造(generation 校验在域内解析侧)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhysicsParticleRef {
    /// 刚体域(单 body = 单 particle;element_index 恒 0)。
    RigidBody(RigidBodyStableId),
    /// 布料顶点(stable_id = 布料资产;element_index = 顶点序)。
    ClothVertex {
        /// 布料资产稳定 ID。
        stable_id: ClothStableId,
        /// 顶点序(0 起;非 arena index,纯逻辑序)。
        element_index: u32,
    },
    /// 破坏碎块(单 chunk = 单 particle;element_index 恒 0)。
    DestructionChunk(ChunkStableId),
    /// Ragdoll 节点(stable_id = 资产;element_index = bone 映射序)。
    RagdollNode {
        /// ragdoll 资产稳定 ID。
        stable_id: RagdollAssetStableId,
        /// bone 映射序(`PhysicsAsset::bones` 序;非 arena index)。
        element_index: u32,
    },
    /// Character 内部状态(单 character = 单 particle;element_index 恒 0)。
    CharacterInner(CharacterStableId),
}

impl PhysicsParticleRef {
    /// 所属域(canonical 分流键)。
    pub fn domain(self) -> ParticleDomain {
        match self {
            Self::RigidBody(_) => ParticleDomain::RigidBody,
            Self::ClothVertex { .. } => ParticleDomain::ClothVertex,
            Self::DestructionChunk(_) => ParticleDomain::DestructionChunk,
            Self::RagdollNode { .. } => ParticleDomain::RagdollNode,
            Self::CharacterInner(_) => ParticleDomain::CharacterInner,
        }
    }

    /// 稳定 ID 位表示(journal 面;**非 arena index**——无 index 读取器)。
    pub fn stable_bits(self) -> u64 {
        match self {
            Self::RigidBody(id) => id.to_bits(),
            Self::ClothVertex { stable_id, .. } => stable_id.to_bits(),
            Self::DestructionChunk(id) => id.to_bits(),
            Self::RagdollNode { stable_id, .. } => stable_id.to_bits(),
            Self::CharacterInner(id) => id.to_bits(),
        }
    }

    /// 逻辑元素序(布料顶点/bone 序;单元素域恒 0)。
    pub fn element_index(self) -> u32 {
        match self {
            Self::RigidBody(_) | Self::DestructionChunk(_) | Self::CharacterInner(_) => 0,
            Self::ClothVertex { element_index, .. } => element_index,
            Self::RagdollNode { element_index, .. } => element_index,
        }
    }

    /// canonical 文本(canonical_json / journal 迁移器唯一字面形态)。
    pub fn canonical_text(self) -> String {
        match self {
            Self::RigidBody(id) => format!("RigidBody:{:016x}", id.to_bits()),
            Self::ClothVertex {
                stable_id,
                element_index,
            } => format!("ClothVertex:{:016x}:{element_index}", stable_id.to_bits()),
            Self::DestructionChunk(id) => format!("DestructionChunk:{:016x}", id.to_bits()),
            Self::RagdollNode {
                stable_id,
                element_index,
            } => format!("RagdollNode:{:016x}:{element_index}", stable_id.to_bits()),
            Self::CharacterInner(id) => format!("CharacterInner:{:016x}", id.to_bits()),
        }
    }
}

/// 睡眠状态(域无关三态;骨架期最小面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleSleepState {
    /// 活跃。
    Awake,
    /// 睡眠(域内语义:刚体 = `!is_active`;cloth = lod 冻结;character = 静止)。
    Sleeping,
    /// 静态/锚定(永不求解;写 impulse 语义 = 确定性 no-op)。
    Static,
}

/// impulse/force 写载荷(写路径唯一合法形态;骨架期 impulse 与 force 同
/// 语义面——都是线动量类输入,不经本视图改写任何 transform)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImpulseWrite {
    /// 线 impulse。
    Linear([f32; 3]),
    /// 力(force;按 dt 折算由域内语义决定,骨架期记账 = impulse 等价)。
    Force([f32; 3]),
}

impl ImpulseWrite {
    /// canonical 文本(canonical_json 面;NaN fail-closed 在调用侧经
    /// `canon_f32_bits_at` 执行)。
    pub fn canonical_text(&self) -> String {
        match self {
            Self::Linear(v) => format!("linear:{v:?}"),
            Self::Force(v) => format!("force:{v:?}"),
        }
    }
}

/// 统一 particle 读写适配面(RFC-0024 §4.A 冻结 trait 集)。
///
/// **写路径结构性保证**:本 trait 只声明 impulse/force 写口
/// ([`set_force_impulse`](Self::set_force_impulse));不存在
/// `set_transform`/`set_position`/`teleport` 任何形式的 transform 直写方法——
/// 旁路写注入在类型面不可表达(编译期消灭),门脚本以「旁路探针源编译期
/// 拒绝」+「域面 transform 直写扫描」双机械核验。
pub trait ParticleAdapter {
    /// 质量(kg;静态/锚定域返回 `f32::INFINITY` 由调用侧解释——骨架期
    /// 诚实面:不伪造有限质量)。
    fn mass(&self, particle: PhysicsParticleRef) -> Result<f32, CaptureError>;

    /// 位置(只读事实源快照;读不改写)。
    fn position(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError>;

    /// 线速度(只读)。
    fn velocity(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError>;

    /// impulse/force 写路径(**唯一写口**;失效/域外 ref → 确定性
    /// `Err(NoSuchParticle)`;骨架期 RagdollNode → `Err(SchemaOnlyAdapter)`)。
    fn set_force_impulse(
        &mut self,
        particle: PhysicsParticleRef,
        write: ImpulseWrite,
    ) -> Result<(), CaptureError>;

    /// 睡眠状态(只读)。
    fn sleep_state(&self, particle: PhysicsParticleRef)
    -> Result<ParticleSleepState, CaptureError>;

    /// 骨架期边界登记(evidence notes 机械源;禁 stub 冒充运行时)。
    fn skeleton_boundary(&self) -> &'static str;
}

/// 骨架期 ragdoll 资产层 adapter 的写路径诚实错误字面(门 RED 臂锚)。
pub const RAGDOLL_SCHEMA_ONLY_LITERAL: &str = "SchemaOnlyAdapter(RagdollNode)";

/// 域错误配/失效句柄的统一拒绝(canonical 字面,门脚本锚定)。
pub const NO_SUCH_PARTICLE_LITERAL: &str = "NoSuchParticle";

/// ref 域校验工具:拒绝非本域 ref(单一字面,fail-closed)。
pub(crate) fn expect_domain(
    particle: PhysicsParticleRef,
    domain: ParticleDomain,
) -> Result<(), CaptureError> {
    if particle.domain() != domain {
        return Err(CaptureError::Rejected(format!(
            "{NO_SUCH_PARTICLE_LITERAL}: expect {} got {}",
            domain.canonical_name(),
            particle.domain().canonical_name()
        )));
    }
    Ok(())
}

/// BodyId → 刚体 ref(唯一构造口;BodyId 位表示即稳定 ID,generation 校验
/// 由 `PhysicsWorld` arena 门禁在解析侧执行)。
pub fn rigid_body_ref(body: BodyId) -> PhysicsParticleRef {
    PhysicsParticleRef::RigidBody(RigidBodyStableId(body.to_bits()))
}

pub mod rigid_body_adapter;

#[cfg(feature = "physics-character")]
pub mod character_adapter;
#[cfg(feature = "physics-cloth")]
pub mod cloth_adapter;
#[cfg(feature = "physics-destruction")]
pub mod destruction_adapter;
#[cfg(feature = "physics-destruction")]
pub mod migrate;
#[cfg(feature = "physics-character")]
pub mod ragdoll_adapter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_domains_closed_set() {
        assert_eq!(ParticleDomain::ALL.len(), 5);
        for d in ParticleDomain::ALL {
            assert_eq!(
                ParticleDomain::from_canonical_name(d.canonical_name()),
                Some(d),
                "canonical roundtrip {d:?}"
            );
        }
        assert_eq!(ParticleDomain::from_canonical_name("GpuParticle"), None);
    }

    #[test]
    fn refs_are_nominally_distinct_and_index_free() {
        let a = rigid_body_ref(BodyId::new(7, 3));
        let b = PhysicsParticleRef::DestructionChunk(ChunkStableId(7));
        assert_ne!(a.domain(), b.domain());
        assert_ne!(a, b);
        assert_eq!(a.stable_bits(), BodyId::new(7, 3).to_bits());
        assert_eq!(a.element_index(), 0);
        // 类型面无 index 读取器:只能从位表示读回(无 arena index 出口)。
        assert_eq!(
            a.canonical_text(),
            format!("RigidBody:{:016x}", a.stable_bits())
        );
    }

    #[test]
    fn domain_mismatch_fails_closed_single_literal() {
        let e = expect_domain(
            PhysicsParticleRef::DestructionChunk(ChunkStableId(1)),
            ParticleDomain::RigidBody,
        )
        .unwrap_err();
        assert!(e.to_string().contains(NO_SUCH_PARTICLE_LITERAL));
    }
}
