//! 完整期场求值器(spec/physics.md RXS-0374 L1;RFC-0024 §4.A/§4.B)。
//!
//! **单一源纪律**:host 参照与一切消费面(求解器耦合 `couple`、主流
//! capture replay 重算 `capture_merge`、World-Field 提交采样)复用本模块
//! 同一求值实例——禁第二套场求值实现。求值纯 host f32 定序运算,同输入
//! 双跑位级一致(确定性面)。

use super::def::{FieldDef, FieldPhysicsType};

/// 场求值输出(力/力矩;经 `ParticleAdapter` 写路径以 impulse/force 语义
/// 耦合进 lockstep 求解器输入的载荷面)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldEvaluation {
    /// 标量采样(场强)。
    pub sample: f32,
    /// 场梯度(analytic-surface 闭集 = 解析梯度;其余基元 = 固定 eps 数值
    /// 梯度,见 `FieldNode::gradient`)。
    pub gradient: [f32; 3],
    /// 力输出(LinearForce 语义;其余枚举零)。
    pub force: [f32; 3],
    /// 力矩输出(Torque 语义;其余枚举零)。
    pub torque: [f32; 3],
}

/// 完整期场求值器(无状态;`new` 得同一实例语义,全部求值路径复用)。
#[derive(Debug, Default, Clone, Copy)]
pub struct FieldEvaluator;

impl FieldEvaluator {
    /// 构造求值器(单一源;无内部状态,实例语义唯一)。
    pub fn new() -> Self {
        Self
    }

    /// 标量采样(与 `FieldNode::sample` 同一面)。
    pub fn sample(&self, def: &FieldDef, p: [f32; 3]) -> f32 {
        def.root.sample(p)
    }

    /// 场梯度(与 `FieldNode::gradient` 同一面)。
    pub fn gradient(&self, def: &FieldDef, p: [f32; 3]) -> [f32; 3] {
        def.root.gradient(p)
    }

    /// 完整求值:`FieldPhysicsType::LinearForce` → `force = gradient ×
    /// sample`;`Torque` → `torque = gradient × sample`;其余六枚举零力/零
    /// 力矩(完整期 M121 耦合面消费 LinearForce;Buoyancy 归 M124 专用
    /// 求值面,不在本通用面产出)。
    pub fn evaluate(&self, def: &FieldDef, p: [f32; 3]) -> FieldEvaluation {
        let sample = self.sample(def, p);
        let gradient = self.gradient(def, p);
        let scaled = [
            gradient[0] * sample,
            gradient[1] * sample,
            gradient[2] * sample,
        ];
        let (force, torque) = match def.physics_type {
            FieldPhysicsType::LinearForce => (scaled, [0.0; 3]),
            FieldPhysicsType::Torque => ([0.0; 3], scaled),
            _ => ([0.0; 3], [0.0; 3]),
        };
        FieldEvaluation {
            sample,
            gradient,
            force,
            torque,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::def::{FieldNode, FieldNodeKind};
    use crate::field::filter::FieldFilter;
    use crate::field::lifecycle::FieldLifecycle;

    fn def_of(kind: FieldNodeKind, ty: FieldPhysicsType) -> FieldDef {
        FieldDef::new(
            "f",
            FieldNode {
                node_id: "n".into(),
                kind,
                weight: 1.0,
                children: vec![],
            },
            ty,
            FieldLifecycle::Transient,
            FieldFilter::default(),
        )
    }

    //@ spec: RXS-0374
    #[test]
    fn evaluator_single_source_deterministic_and_typed_output() {
        let ev = FieldEvaluator::new();
        let def = def_of(
            FieldNodeKind::RadialFalloff {
                center: [0.0; 3],
                radius: 10.0,
            },
            FieldPhysicsType::LinearForce,
        );
        let p = [1.0, 0.0, 0.0];
        let a = ev.evaluate(&def, p);
        let b = ev.evaluate(&def, p);
        assert_eq!(a, b, "同输入双跑位级一致");
        // LinearForce:力 = 梯度×场强;径向衰减在 +x 侧梯度指向 -x。
        assert!(a.force[0] < 0.0 && a.force[1] == 0.0 && a.force[2] == 0.0);
        assert_eq!(a.torque, [0.0; 3]);
        assert!(a.sample > 0.0);

        // Torque 语义:力矩通道输出,力通道零。
        let tq = def_of(
            FieldNodeKind::RadialFalloff {
                center: [0.0; 3],
                radius: 10.0,
            },
            FieldPhysicsType::Torque,
        );
        let t = ev.evaluate(&tq, p);
        assert_eq!(t.force, [0.0; 3]);
        assert!(t.torque[0] < 0.0);

        // 其余六枚举零输出(完整期不在通用耦合面产力)。
        for ty in [
            FieldPhysicsType::Velocity,
            FieldPhysicsType::Sleeping,
            FieldPhysicsType::Disabled,
            FieldPhysicsType::CollisionGroup,
            FieldPhysicsType::Strain,
            FieldPhysicsType::Buoyancy,
        ] {
            let d = def_of(
                FieldNodeKind::Sphere {
                    center: [0.0; 3],
                    radius: 5.0,
                },
                ty,
            );
            let e = ev.evaluate(&d, p);
            assert_eq!(e.force, [0.0; 3]);
            assert_eq!(e.torque, [0.0; 3]);
        }
    }
}
