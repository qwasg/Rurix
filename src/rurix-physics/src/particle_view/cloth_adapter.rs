//! ClothVertex 域 adapter(M121 骨架期;M72 demo 轨道诚实接线)。
//!
//! 骨架期边界(诚实登记,非完整语义):M72 `ClothSolver::positions` 为演示
//! 轨道(demo garment),adapter 按现状接线;`set_force_impulse` 对 cloth
//! 顶点 = 位置增量(delta = impulse / 单位质量,骨架期记账语义,**非**
//! transform 直写——cloth 顶点无 transform 面,位置增量即 XPBD 外力注入的
//! 最小骨架形态)。生产布料顶点轨接真实 XPBD 外力积累器后,本面 0-byte
//! 适配,签名不变。

use crate::capture::canonical::CaptureError;
use crate::cloth::ClothSolver;

use super::{
    ImpulseWrite, NO_SUCH_PARTICLE_LITERAL, ParticleAdapter, ParticleDomain, ParticleSleepState,
    PhysicsParticleRef, expect_domain,
};

/// Cloth 顶点 adapter(持有 solver 借用;stable_id = 布料资产位表示)。
pub struct ClothVertexAdapter<'s> {
    solver: &'s mut ClothSolver,
    cloth_bits: u64,
}

impl<'s> ClothVertexAdapter<'s> {
    /// 绑定 demo solver(cloth 资产稳定位表示由调用方给;骨架期 demo 轨道
    /// 单资产,位表示 = `ClothAsset::digest` 截位由调用侧派生)。
    pub fn new(solver: &'s mut ClothSolver, cloth_bits: u64) -> Self {
        Self { solver, cloth_bits }
    }

    fn vertex_of(&self, particle: PhysicsParticleRef) -> Result<u32, CaptureError> {
        expect_domain(particle, ParticleDomain::ClothVertex)?;
        let PhysicsParticleRef::ClothVertex {
            stable_id,
            element_index,
        } = particle
        else {
            return Err(CaptureError::Rejected(NO_SUCH_PARTICLE_LITERAL.into()));
        };
        if stable_id.to_bits() != self.cloth_bits {
            return Err(CaptureError::Rejected(format!(
                "{NO_SUCH_PARTICLE_LITERAL}: cloth asset bits mismatch"
            )));
        }
        Ok(element_index)
    }

    fn check_index(&self, idx: u32) -> Result<usize, CaptureError> {
        let i = idx as usize;
        if i >= self.solver.positions.len() {
            return Err(CaptureError::Rejected(format!(
                "{NO_SUCH_PARTICLE_LITERAL}: cloth vertex {idx} out of {}",
                self.solver.positions.len()
            )));
        }
        Ok(i)
    }
}

impl ParticleAdapter for ClothVertexAdapter<'_> {
    fn mass(&self, particle: PhysicsParticleRef) -> Result<f32, CaptureError> {
        let idx = self.vertex_of(particle)?;
        self.check_index(idx)?;
        // demo 轨道单位质量(骨架期记账语义;fabric_json density 未接线)。
        Ok(1.0)
    }

    fn position(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        let idx = self.vertex_of(particle)?;
        let i = self.check_index(idx)?;
        Ok(self.solver.positions[i])
    }

    fn velocity(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        let idx = self.vertex_of(particle)?;
        self.check_index(idx)?;
        // demo 轨道无速度面(隐式欧拉位置态);骨架期诚实零。
        Ok([0.0; 3])
    }

    fn set_force_impulse(
        &mut self,
        particle: PhysicsParticleRef,
        write: ImpulseWrite,
    ) -> Result<(), CaptureError> {
        let idx = self.vertex_of(particle)?;
        let i = self.check_index(idx)?;
        let impulse = match write {
            ImpulseWrite::Linear(v) | ImpulseWrite::Force(v) => v,
        };
        // 骨架期记账:impulse → 位置增量(单位质量);cloth 顶点无 transform
        // 面,位置增量是 XPBD 外力注入的最小骨架形态(非 transform 直写)。
        for (k, v) in impulse.iter().enumerate() {
            self.solver.positions[i][k] += v;
        }
        Ok(())
    }

    fn sleep_state(
        &self,
        particle: PhysicsParticleRef,
    ) -> Result<ParticleSleepState, CaptureError> {
        let idx = self.vertex_of(particle)?;
        self.check_index(idx)?;
        // demo 轨道恒活跃(lod 冻结面未接线)。
        Ok(ParticleSleepState::Awake)
    }

    fn skeleton_boundary(&self) -> &'static str {
        "demo_track(M72 positions);unit_mass;no_velocity_face"
    }
}
