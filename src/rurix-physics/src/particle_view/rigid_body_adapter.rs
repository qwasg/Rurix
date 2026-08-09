//! RigidBody 域 adapter(M121 骨架期;运行时权威,直接挂 `PhysicsWorld`)。
//!
//! 解析路径:`RigidBodyStableId` 位表示 → `BodyId::from_bits` → arena
//! generation 门禁(`world.body_transform` 等既有面,失效句柄确定性
//! `Err(InvalidBody)` → 映射 `NoSuchParticle`);**类型面只过位表示,
//! arena index 永不外露**。
//!
//! 骨架期诚实面:`BodySemantic` 快照(RFC-0017 §4.A6)不含质量字段,
//! 质量走宿主登记的 desc 通道由调用方自查;本 adapter `mass()` 诚实
//! `Err(Rejected("mass_not_in_snapshot"))`,不伪造有限质量(禁 stub 充绿)。

use crate::capture::canonical::CaptureError;
use crate::error::PhysicsError;
use crate::id::BodyId;
use crate::world::PhysicsWorld;

use super::{
    ImpulseWrite, NO_SUCH_PARTICLE_LITERAL, ParticleAdapter, ParticleDomain, ParticleSleepState,
    PhysicsParticleRef, expect_domain,
};

/// RigidBody 域 adapter(持有 `PhysicsWorld` 借用;写 = `apply_impulse` 唯一面)。
pub struct RigidBodyAdapter<'w> {
    world: &'w mut PhysicsWorld,
}

impl<'w> RigidBodyAdapter<'w> {
    /// 包装既有世界(零所有权变更;step 相位纪律由调用方维持,§4.A4 Q-B)。
    pub fn new(world: &'w mut PhysicsWorld) -> Self {
        Self { world }
    }

    fn body_of(particle: PhysicsParticleRef) -> Result<BodyId, CaptureError> {
        expect_domain(particle, ParticleDomain::RigidBody)?;
        let PhysicsParticleRef::RigidBody(id) = particle else {
            return Err(CaptureError::Rejected(NO_SUCH_PARTICLE_LITERAL.into()));
        };
        Ok(BodyId::from_bits(id.to_bits()))
    }

    fn map_err(e: PhysicsError) -> CaptureError {
        // 失效句柄(含 generation 失配)= 单一 NoSuchParticle 字面;其余
        // 后端错误原样上抛(不静默)。
        match e {
            PhysicsError::InvalidBody(_) => CaptureError::Rejected(NO_SUCH_PARTICLE_LITERAL.into()),
            other => CaptureError::Backend(other.to_string()),
        }
    }
}

impl ParticleAdapter for RigidBodyAdapter<'_> {
    fn mass(&self, particle: PhysicsParticleRef) -> Result<f32, CaptureError> {
        // 骨架期诚实边界:BodySemantic 快照无质量字段;不伪造(P-01/P-09)。
        let _body = Self::body_of(particle)?;
        Err(CaptureError::Rejected(
            "mass_not_in_snapshot(RigidBody): BodySemantic has no mass field; use host BodyDesc channel".into(),
        ))
    }

    fn position(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        let body = Self::body_of(particle)?;
        self.world
            .body_transform(body)
            .map(|t| t.translation)
            .map_err(Self::map_err)
    }

    fn velocity(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        let body = Self::body_of(particle)?;
        self.world
            .body_velocities(body)
            .map(|(lin, _ang)| lin)
            .map_err(Self::map_err)
    }

    fn set_force_impulse(
        &mut self,
        particle: PhysicsParticleRef,
        write: ImpulseWrite,
    ) -> Result<(), CaptureError> {
        let body = Self::body_of(particle)?;
        // 写路径唯一面:apply_impulse(§4.A 冻结既有面);骨架期 impulse/force
        // 同语义记账,不允许 transform 直写(类型面消灭)。
        let impulse = match write {
            ImpulseWrite::Linear(v) | ImpulseWrite::Force(v) => v,
        };
        self.world
            .apply_impulse(body, impulse)
            .map_err(Self::map_err)
    }

    fn sleep_state(
        &self,
        particle: PhysicsParticleRef,
    ) -> Result<ParticleSleepState, CaptureError> {
        let body = Self::body_of(particle)?;
        let active = self.world.is_active(body).map_err(Self::map_err)?;
        Ok(if active {
            ParticleSleepState::Awake
        } else {
            ParticleSleepState::Sleeping
        })
    }

    fn skeleton_boundary(&self) -> &'static str {
        "runtime_authoritative(jolt_world);mass=host_desc_channel"
    }
}
