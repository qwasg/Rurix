//! CharacterInner 域 adapter(M121 骨架期;M71 角色内部状态最小面)。
//!
//! 骨架期边界:M71 `RurixCharacter` 是版本化状态块(canonical 捕获面),
//! 非完整角色控制器运行时;adapter 读面 = 状态块 transform/velocity,
//! 写面 = impulse 记账到 `linear_velocity`(骨架期记账语义,角色内部状态
//! 的最小外力注入形态;非 transform 直写——position 不经写面变更)。

use crate::capture::canonical::CaptureError;
use crate::character::RurixCharacter;

use super::{
    CharacterStableId, ImpulseWrite, NO_SUCH_PARTICLE_LITERAL, ParticleAdapter, ParticleDomain,
    ParticleSleepState, PhysicsParticleRef, expect_domain,
};

/// Character 域 adapter(持有角色状态块借用)。
pub struct CharacterAdapter<'c> {
    character: &'c mut RurixCharacter,
}

impl<'c> CharacterAdapter<'c> {
    /// 绑定角色(character_id 即稳定 ID 位表示)。
    pub fn new(character: &'c mut RurixCharacter) -> Self {
        Self { character }
    }

    /// 角色 ref(character_id → 域句柄)。
    pub fn ref_of(character_id: u64) -> PhysicsParticleRef {
        PhysicsParticleRef::CharacterInner(CharacterStableId(character_id))
    }

    fn check(&self, particle: PhysicsParticleRef) -> Result<(), CaptureError> {
        expect_domain(particle, ParticleDomain::CharacterInner)?;
        let PhysicsParticleRef::CharacterInner(id) = particle else {
            return Err(CaptureError::Rejected(NO_SUCH_PARTICLE_LITERAL.into()));
        };
        if id.to_bits() != self.character.state.character_id {
            return Err(CaptureError::Rejected(format!(
                "{NO_SUCH_PARTICLE_LITERAL}: character bits mismatch"
            )));
        }
        Ok(())
    }
}

impl ParticleAdapter for CharacterAdapter<'_> {
    fn mass(&self, particle: PhysicsParticleRef) -> Result<f32, CaptureError> {
        self.check(particle)?;
        // 骨架期诚实面:M71 状态块无质量字段;单位质量记账。
        Ok(1.0)
    }

    fn position(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        self.check(particle)?;
        Ok(self.character.state.transform.translation)
    }

    fn velocity(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        self.check(particle)?;
        Ok(self.character.state.linear_velocity)
    }

    fn set_force_impulse(
        &mut self,
        particle: PhysicsParticleRef,
        write: ImpulseWrite,
    ) -> Result<(), CaptureError> {
        self.check(particle)?;
        let impulse = match write {
            ImpulseWrite::Linear(v) | ImpulseWrite::Force(v) => v,
        };
        // 骨架期记账:impulse → linear_velocity 累加(单位质量);
        // position 不经写面变更(非 transform 直写)。
        for (k, v) in impulse.iter().enumerate() {
            self.character.state.linear_velocity[k] += v;
        }
        Ok(())
    }

    fn sleep_state(
        &self,
        particle: PhysicsParticleRef,
    ) -> Result<ParticleSleepState, CaptureError> {
        self.check(particle)?;
        let v = self.character.state.linear_velocity;
        let still = v.iter().all(|c| *c == 0.0);
        Ok(if still {
            ParticleSleepState::Sleeping
        } else {
            ParticleSleepState::Awake
        })
    }

    fn skeleton_boundary(&self) -> &'static str {
        "state_block(M71);unit_mass;velocity_ledger_write_only"
    }
}
