//! Presentation-only soft/hard snap;权威状态 0 影响(RFC-0021 §4.B1)。

use crate::types::PhysicsTransform;

/// 表现层变换(可被 smoothing 改写;不得回写权威 world)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresentationTransform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}

impl From<PhysicsTransform> for PresentationTransform {
    fn from(t: PhysicsTransform) -> Self {
        Self {
            translation: t.translation,
            rotation: t.rotation,
        }
    }
}

impl PresentationTransform {
    pub fn to_physics(&self) -> PhysicsTransform {
        PhysicsTransform {
            translation: self.translation,
            rotation: self.rotation,
        }
    }
}

/// 逐帧偏移度量(采样/bound 判据用)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresentationOffset {
    pub position_m: f32,
    pub angle_rad: f32,
}

impl PresentationOffset {
    pub fn between(auth: &PhysicsTransform, pres: &PresentationTransform) -> Self {
        let dx = pres.translation[0] - auth.translation[0];
        let dy = pres.translation[1] - auth.translation[1];
        let dz = pres.translation[2] - auth.translation[2];
        let position_m = (dx * dx + dy * dy + dz * dz).sqrt();
        // 简易角差:四元数点积 → 角
        let dot = (auth.rotation[0] * pres.rotation[0]
            + auth.rotation[1] * pres.rotation[1]
            + auth.rotation[2] * pres.rotation[2]
            + auth.rotation[3] * pres.rotation[3])
            .clamp(-1.0, 1.0);
        let angle_rad = 2.0 * dot.abs().acos();
        Self {
            position_m,
            angle_rad,
        }
    }
}

/// RFC-0021 §6.5.1 冻结 bound(实现期采样后写入;未冻结时 smoke 如实 RED)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmoothingBound {
    pub max_position_m: f32,
    pub max_angle_rad: f32,
    pub max_convergence_frames: u32,
    pub frozen: bool,
    pub reference: &'static str,
}

/// 默认未冻结表。smoke `--force-freeze-bound` 用 measured ceiling 本地冻结;
/// RFC-0021 §6.5.1 + `g8_budget.json` 由 Gov materialize 同 PR 回填后改 frozen=true。
/// measured_local(mispredict_impulse_delay): max_pos≈0.215m max_ang≈0.031rad。
pub const SMOOTHING_BOUND_V1: SmoothingBound = SmoothingBound {
    max_position_m: 0.269,
    max_angle_rad: 0.039,
    max_convergence_frames: 30,
    frozen: false,
    reference: "RFC-0021 §6.5.1 (pending Gov freeze; measured_local ceiling ready)",
};

pub fn soft_snap(
    authoritative: &PhysicsTransform,
    presentation: &mut PresentationTransform,
    alpha: f32,
) {
    let a = alpha.clamp(0.0, 1.0);
    for i in 0..3 {
        presentation.translation[i] =
            presentation.translation[i] * (1.0 - a) + authoritative.translation[i] * a;
    }
    for i in 0..4 {
        presentation.rotation[i] =
            presentation.rotation[i] * (1.0 - a) + authoritative.rotation[i] * a;
    }
    // 归一化 quat
    let n = (presentation.rotation[0] * presentation.rotation[0]
        + presentation.rotation[1] * presentation.rotation[1]
        + presentation.rotation[2] * presentation.rotation[2]
        + presentation.rotation[3] * presentation.rotation[3])
        .sqrt();
    if n > 1e-8 {
        for i in 0..4 {
            presentation.rotation[i] /= n;
        }
    }
}

pub fn hard_snap(authoritative: &PhysicsTransform, presentation: &mut PresentationTransform) {
    *presentation = PresentationTransform::from(*authoritative);
}

pub fn within_bound(offset: &PresentationOffset, bound: &SmoothingBound) -> bool {
    if !bound.frozen {
        return false;
    }
    offset.position_m <= bound.max_position_m && offset.angle_rad <= bound.max_angle_rad
}
