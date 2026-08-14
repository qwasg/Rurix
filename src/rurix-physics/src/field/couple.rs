//! 场求值与求解器耦合(力场积分;spec/physics.md RXS-0374 L1,判据逐字引
//! G9_ACCEPTANCE_MAP §2 M121 行;RFC-0024 §4.A/§4.B)。
//!
//! 冻结纪律:
//! - **tick 内显式序 = 场求值 → impulse 施加 → 求解步进**;本模块承载前两
//!   步,求解步进由调用方紧随其后(`PhysicsWorld::step`)。
//! - **写路径仅 impulse/force**:场输出只经 [`ParticleAdapter::set_force_impulse`]
//!   耦合——任何 transform/速度/位置直写旁路在类型面不可表达(骨架期结构
//!   断言字面 0-byte 维持)。
//! - **求值单一源**:一切求值经 [`FieldEvaluator`] 同一实例语义(同输入
//!   双运行逐位一致)。

use crate::capture::canonical::CaptureError;
use crate::particle_view::{
    ImpulseWrite, ParticleAdapter, ParticleSleepState, PhysicsParticleRef,
};

use super::eval::FieldEvaluator;
use super::filter::sleep_state_bits;
use super::registry::FieldRegistry;

/// 一场对一粒子的耦合中间量(求值结果 + 位置;确定性记账面)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldCouplingSample {
    /// 粒子运行时位置(适配器只读面快照)。
    pub position: [f32; 3],
    /// 场求值输出(力/力矩)。
    pub force: [f32; 3],
    /// 力矩输出(完整期记账面;M121 写路径仅线性 impulse/force——力矩
    /// 施加需 `PhysicsWorld` 角冲量口,不在 RFC-0017 §4.A 冻结 API 内,
    /// 本波诚实登记不接线)。
    pub torque: [f32; 3],
}

/// tick 前场求值 + impulse 施加(RXS-0374 L1 显式序前两步)。
///
/// 返回实际施加的 `(particle, impulse)` 列表(canonical 序;零冲量不施加
/// 也不记账——场置零/零匹配时对求解输入零扰动,退化基线逐位一致断言面)。
pub fn apply_field_impulses<A: ParticleAdapter>(
    adapter: &mut A,
    registry: &FieldRegistry,
    evaluator: &FieldEvaluator,
    particles: &[(PhysicsParticleRef, ParticleSleepState, u32)],
    dt: f32,
) -> Result<Vec<(PhysicsParticleRef, [f32; 3])>, CaptureError> {
    // 1) 场求值(位置经适配器只读面;求值器单一源)。
    let mut pending: Vec<(PhysicsParticleRef, [f32; 3])> = Vec::new();
    for (p, state, layer) in particles {
        let position = adapter.position(*p)?;
        let state_bits = sleep_state_bits(*state);
        let mut total_force = [0.0f32; 3];
        for (_id, reg) in registry.iter() {
            if !reg.def.filter.matches(*p, state_bits, *layer) {
                continue;
            }
            let eval = evaluator.evaluate(&reg.def, position);
            total_force[0] += eval.force[0];
            total_force[1] += eval.force[1];
            total_force[2] += eval.force[2];
        }
        // force → impulse(固定步 dt 折算;调用方锁死 dt = dt_fixed)。
        let impulse = [
            total_force[0] * dt,
            total_force[1] * dt,
            total_force[2] * dt,
        ];
        if impulse != [0.0; 3] {
            pending.push((*p, impulse));
        }
    }
    // canonical 序 = 确定性施加序。
    pending.sort_by(|a, b| a.0.canonical_text().cmp(&b.0.canonical_text()));
    // 2) impulse 施加(写路径唯一口;域外/失效 ref fail-closed)。
    for (p, impulse) in &pending {
        adapter.set_force_impulse(*p, ImpulseWrite::Linear(*impulse))?;
    }
    Ok(pending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::jolt_world_desc;
    use crate::field::def::{FieldDef, FieldNode, FieldNodeKind, FieldPhysicsType};
    use crate::field::filter::{FieldFilter, domain_bit, object_state_bits};
    use crate::field::lifecycle::FieldLifecycle;
    use crate::field::registry::FieldRegistry;
    use crate::particle_view::rigid_body_adapter::RigidBodyAdapter;
    use crate::particle_view::{ParticleDomain, rigid_body_ref};
    use crate::{BodyDesc, BodyKind, MassProps, PhysicsTransform, PhysicsWorld, ShapeDesc};

    fn drive_field(id: &str) -> FieldDef {
        FieldDef::new(
            id,
            FieldNode {
                node_id: format!("{id}_root"),
                kind: FieldNodeKind::RadialFalloff {
                    // 中心偏置 +x:原点处场梯度非零(正心对称点梯度为零,
                    // 非零贡献断言需要偏置探针)。
                    center: [1.0, 0.0, 0.0],
                    radius: 50.0,
                },
                weight: 1.0,
                children: vec![],
            },
            FieldPhysicsType::LinearForce,
            FieldLifecycle::Persistent,
            FieldFilter {
                object_state_mask: object_state_bits::AWAKE,
                domain_mask: domain_bit(ParticleDomain::RigidBody),
                layer_mask: 1,
                explicit_include: vec![],
                explicit_exclude: vec![],
            },
        )
    }

    fn world_with_body() -> (PhysicsWorld, crate::id::BodyId) {
        let mut w = PhysicsWorld::new(jolt_world_desc(16)).expect("jolt world");
        let id = w
            .add_bodies_batch(&[BodyDesc {
                kind: BodyKind::Dynamic,
                shape: ShapeDesc::Sphere { radius: 0.5 },
                layer: 0,
                mass_props: MassProps {
                    mass: 1.0,
                    friction: 0.5,
                    restitution: 0.0,
                    allow_sleep: false,
                },
                ccd: false,
                transform: PhysicsTransform::IDENTITY,
            }])
            .expect("add body")
            .remove(0);
        (w, id)
    }

    //@ spec: RXS-0374
    #[test]
    fn coupling_applies_nonzero_impulse_via_impulse_write_path() {
        let (mut world, body) = world_with_body();
        let mut registry = FieldRegistry::new();
        registry.register(drive_field("drive")).expect("register");
        let evaluator = FieldEvaluator::new();
        let particles = vec![(rigid_body_ref(body), ParticleSleepState::Awake, 0u32)];
        let dt = jolt_world_desc(16).dt_fixed;
        let applied = {
            let mut ad = RigidBodyAdapter::new(&mut world);
            apply_field_impulses(&mut ad, &registry, &evaluator, &particles, dt).expect("couple")
        };
        assert_eq!(applied.len(), 1, "唯一动态体匹配 → 一条 impulse");
        assert!(applied[0].1.iter().any(|c| *c != 0.0), "场贡献非零");
        // impulse 已进求解器输入(速度位非零;位置只经 step 演化)。
        let ad = RigidBodyAdapter::new(&mut world);
        let v = ad.velocity(rigid_body_ref(body)).expect("velocity");
        assert!(v.iter().any(|c| *c != 0.0));
    }

    //@ spec: RXS-0374
    #[test]
    fn coupling_zero_field_and_zero_match_are_bitwise_noop() {
        // 场置零(权重 0)= 零 impulse 施加;零匹配 = 空列表——两臂均不触
        // 写路径(退化基线逐位一致的前提断言)。
        let (mut world, body) = world_with_body();
        let evaluator = FieldEvaluator::new();
        let particles = vec![(rigid_body_ref(body), ParticleSleepState::Awake, 0u32)];
        let dt = jolt_world_desc(16).dt_fixed;

        let mut zeroed = drive_field("zeroed");
        zeroed.root.weight = 0.0;
        let mut registry = FieldRegistry::new();
        registry.register(zeroed).expect("register zeroed");
        let applied_zeroed = {
            let mut ad = RigidBodyAdapter::new(&mut world);
            apply_field_impulses(&mut ad, &registry, &evaluator, &particles, dt).expect("couple")
        };
        assert!(applied_zeroed.is_empty(), "场置零 → 零 impulse 施加");

        let mut registry2 = FieldRegistry::new();
        let mut no_match = drive_field("no_match");
        no_match.filter = FieldFilter::default();
        registry2.register(no_match).expect("register no_match");
        let applied_nomatch = {
            let mut ad = RigidBodyAdapter::new(&mut world);
            apply_field_impulses(&mut ad, &registry2, &evaluator, &particles, dt).expect("couple")
        };
        assert!(applied_nomatch.is_empty(), "零匹配 → 零 impulse 施加");
    }
}
