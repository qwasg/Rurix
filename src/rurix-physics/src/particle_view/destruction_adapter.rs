//! DestructionChunk 域 adapter(M121 骨架期;M68 chunk 只读映射 + 激活状态)。
//!
//! chunk 状态(质量/中心/半宽)活在 cooked artifact(静态);激活状态活在
//! `FracturePipeline`(运行时)。写路径 = 记账 impulse 缓存(骨架期不直接
//! 驱动 Jolt 权威体——M68 激活体由宿主 pipeline 显式 spawn,本视图只记账,
//! 完整期接宿主 spawn 通道;骨架期诚实登记)。

use std::collections::BTreeMap;

use crate::capture::canonical::CaptureError;
use crate::destruction::{DestructionCookedArtifact, FracturePipeline};

use super::{
    ChunkStableId, ImpulseWrite, NO_SUCH_PARTICLE_LITERAL, ParticleAdapter, ParticleDomain,
    ParticleSleepState, PhysicsParticleRef, chunk_stable_bits, expect_domain,
};

/// DestructionChunk 域 adapter(cooked 静态面 + pipeline 运行时面双引用)。
pub struct DestructionChunkAdapter<'p> {
    cooked: &'p DestructionCookedArtifact,
    pipeline: Option<&'p FracturePipeline>,
    /// 记账 impulse 缓存(骨架期不驱权威体;键 = chunk 位表示)。
    impulse_ledger: BTreeMap<u64, [f32; 3]>,
}

impl<'p> DestructionChunkAdapter<'p> {
    /// cooked-only 构造(骨架期最小面;无 pipeline 时 sleep_state = 资产态)。
    pub fn cooked_only(cooked: &'p DestructionCookedArtifact) -> Self {
        Self {
            cooked,
            pipeline: None,
            impulse_ledger: BTreeMap::new(),
        }
    }

    /// 完整构造(cooked + 运行时 pipeline;激活状态只读查询)。
    pub fn with_pipeline(
        cooked: &'p DestructionCookedArtifact,
        pipeline: &'p FracturePipeline,
    ) -> Self {
        Self {
            cooked,
            pipeline: Some(pipeline),
            impulse_ledger: BTreeMap::new(),
        }
    }

    fn chunk_of(
        &self,
        particle: PhysicsParticleRef,
    ) -> Result<&'p crate::destruction::ChunkDesc, CaptureError> {
        expect_domain(particle, ParticleDomain::DestructionChunk)?;
        let PhysicsParticleRef::DestructionChunk(id) = particle else {
            return Err(CaptureError::Rejected(NO_SUCH_PARTICLE_LITERAL.into()));
        };
        self.cooked
            .chunks
            .iter()
            .find(|c| chunk_stable_bits(&c.chunk_id) == id.to_bits())
            .ok_or_else(|| {
                CaptureError::Rejected(format!(
                    "{NO_SUCH_PARTICLE_LITERAL}: chunk bits {:016x}",
                    id.to_bits()
                ))
            })
    }

    /// chunk 稳定 ref(cooked artifact 内按 chunk_id 派生)。
    pub fn ref_of_chunk(chunk_id: &str) -> PhysicsParticleRef {
        PhysicsParticleRef::DestructionChunk(ChunkStableId(chunk_stable_bits(chunk_id)))
    }

    /// 记账 impulse 只读回查(骨架期迁移断言面)。
    pub fn ledger_impulse(&self, particle: PhysicsParticleRef) -> Option<[f32; 3]> {
        self.impulse_ledger.get(&particle.stable_bits()).copied()
    }
}

impl ParticleAdapter for DestructionChunkAdapter<'_> {
    fn mass(&self, particle: PhysicsParticleRef) -> Result<f32, CaptureError> {
        let c = self.chunk_of(particle)?;
        Ok(c.mass)
    }

    fn position(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        let c = self.chunk_of(particle)?;
        Ok(c.center)
    }

    fn velocity(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        self.chunk_of(particle)?;
        // 骨架期诚实面:chunk 激活后的权威速度在宿主 Jolt 体,不在本视图;
        // 未激活 chunk 零速度(锚定)。
        Ok([0.0; 3])
    }

    fn set_force_impulse(
        &mut self,
        particle: PhysicsParticleRef,
        write: ImpulseWrite,
    ) -> Result<(), CaptureError> {
        let c = self.chunk_of(particle)?;
        let bits = chunk_stable_bits(&c.chunk_id);
        let impulse = match write {
            ImpulseWrite::Linear(v) | ImpulseWrite::Force(v) => v,
        };
        // 骨架期记账(不直写 transform;不驱权威体——完整期接宿主 spawn)。
        let e = self.impulse_ledger.entry(bits).or_insert([0.0; 3]);
        for (k, v) in impulse.iter().enumerate() {
            e[k] += v;
        }
        Ok(())
    }

    fn sleep_state(
        &self,
        particle: PhysicsParticleRef,
    ) -> Result<ParticleSleepState, CaptureError> {
        let c = self.chunk_of(particle)?;
        let Some(pipeline) = self.pipeline else {
            return Ok(ParticleSleepState::Static);
        };
        // 激活判定:任一激活 body 的 chunk 集包含本 chunk。
        let activated = pipeline
            .activated_bodies()
            .iter()
            .any(|b| b.chunk_ids.iter().any(|id| id == &c.chunk_id));
        Ok(if activated {
            ParticleSleepState::Awake
        } else {
            ParticleSleepState::Static
        })
    }

    fn skeleton_boundary(&self) -> &'static str {
        "cooked_static+pipeline_activation_read;impulse=ledger_only_no_authoritative_drive"
    }
}
