//! RagdollNode 域 adapter(M121 骨架期;**资产层只读视图,非运行时权威**)。
//!
//! 骨架期诚实边界(判据硬要求,禁 stub 冒充运行时):G8 终态 ragdoll
//! 运行时面缺失——M69 只落 `asset::PhysicsAsset` 资产层 + bone→body
//! 映射 + 约束五件套 schema,**无运行时 ragdoll 实例类型**。骨架期
//! adapter = 资产层只读视图:
//! - 读面 = 资产静态数据(bone 映射 / 资产 digest);
//! - **写面确定性 `Err(SchemaOnlyAdapter(RagdollNode))`**——不存在
//!   运行时 ragdoll 实例可写,诚实拒绝;运行时权威 adapter 归
//!   --phase g9.6 完整期。

use crate::asset::PhysicsAsset;
use crate::capture::canonical::CaptureError;

use super::{
    ImpulseWrite, NO_SUCH_PARTICLE_LITERAL, ParticleAdapter, ParticleDomain, ParticleSleepState,
    PhysicsParticleRef, RAGDOLL_SCHEMA_ONLY_LITERAL, expect_domain,
};

/// RagdollNode 域 adapter(资产层只读视图;写面确定性 SchemaOnlyAdapter 拒绝)。
pub struct RagdollNodeAdapter<'a> {
    asset: &'a PhysicsAsset,
    asset_bits: u64,
}

impl<'a> RagdollNodeAdapter<'a> {
    /// 绑定资产(asset_bits = 资产 digest 截位,调用侧派生;非 arena index)。
    pub fn new(asset: &'a PhysicsAsset, asset_bits: u64) -> Self {
        Self { asset, asset_bits }
    }

    fn node_of(&self, particle: PhysicsParticleRef) -> Result<u32, CaptureError> {
        expect_domain(particle, ParticleDomain::RagdollNode)?;
        let PhysicsParticleRef::RagdollNode {
            stable_id,
            element_index,
        } = particle
        else {
            return Err(CaptureError::Rejected(NO_SUCH_PARTICLE_LITERAL.into()));
        };
        if stable_id.to_bits() != self.asset_bits {
            return Err(CaptureError::Rejected(format!(
                "{NO_SUCH_PARTICLE_LITERAL}: ragdoll asset bits mismatch"
            )));
        }
        if element_index as usize >= self.asset.bones.len() {
            return Err(CaptureError::Rejected(format!(
                "{NO_SUCH_PARTICLE_LITERAL}: ragdoll node {element_index} out of {}",
                self.asset.bones.len()
            )));
        }
        Ok(element_index)
    }
}

impl ParticleAdapter for RagdollNodeAdapter<'_> {
    fn mass(&self, particle: PhysicsParticleRef) -> Result<f32, CaptureError> {
        let _idx = self.node_of(particle)?;
        // 资产层无质量面(质量归 collider_role 运行时实例化);诚实拒绝。
        Err(CaptureError::Rejected(format!(
            "{RAGDOLL_SCHEMA_ONLY_LITERAL}: mass is runtime-instance face"
        )))
    }

    fn position(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        let _idx = self.node_of(particle)?;
        // 资产层无运行时位置;诚实拒绝(不返 [0;3] 伪装)。
        Err(CaptureError::Rejected(format!(
            "{RAGDOLL_SCHEMA_ONLY_LITERAL}: position is runtime-instance face"
        )))
    }

    fn velocity(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        let _idx = self.node_of(particle)?;
        Err(CaptureError::Rejected(format!(
            "{RAGDOLL_SCHEMA_ONLY_LITERAL}: velocity is runtime-instance face"
        )))
    }

    fn set_force_impulse(
        &mut self,
        particle: PhysicsParticleRef,
        _write: ImpulseWrite,
    ) -> Result<(), CaptureError> {
        let _idx = self.node_of(particle)?;
        // 骨架期硬边界:写面确定性拒绝——无运行时 ragdoll 实例可写;
        // 不 stub 冒充运行时权威。
        Err(CaptureError::Rejected(format!(
            "{RAGDOLL_SCHEMA_ONLY_LITERAL}: write path reserved for g9.6 runtime adapter"
        )))
    }

    fn sleep_state(
        &self,
        particle: PhysicsParticleRef,
    ) -> Result<ParticleSleepState, CaptureError> {
        let _idx = self.node_of(particle)?;
        Err(CaptureError::Rejected(format!(
            "{RAGDOLL_SCHEMA_ONLY_LITERAL}: sleep_state is runtime-instance face"
        )))
    }

    fn skeleton_boundary(&self) -> &'static str {
        "schema_layer_readonly(no_runtime_ragdoll_instance);write=SchemaOnlyAdapter"
    }
}
