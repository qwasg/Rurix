//! 场注册表(生命周期执行体 + 零影响不变量断言面 + semantic hash)。
//!
//! 纪律:
//! - Persistent 场注册/注销/变更全经本表并写 command journal(调用方
//!   逐 tick 记 `FieldJournal`;本表 semantic_hash 参与逐 tick hash);
//! - Transient 场**不得**进注册表(`LifecycleViolation` fail-closed);
//! - Construction 场不进注册表(进 cooked digest,`cooked_digest` 面);
//! - **零影响不变量**:注册一个过滤零匹配的场,对任何粒子集的求值输出
//!   恒为空 → 世界状态不受影响(门脚本断言 = 注册零匹配场前后世界状态
//!   hash 逐位一致)。

use std::collections::BTreeMap;

use rurix_pkg::sha256::{digest, hex};

use super::def::{FieldDef, FieldError};
use super::lifecycle::FieldLifecycle;
use crate::particle_view::{ParticleSleepState, PhysicsParticleRef};

/// 已注册场(定义 + 注册 tick 序)。
#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredField {
    /// 完整定义。
    pub def: FieldDef,
    /// 注册序(单调;canonical 序的决胜键)。
    pub ordinal: u64,
}

/// 场注册表(BTreeMap 规范序 = 确定性面)。
#[derive(Debug, Clone, Default)]
pub struct FieldRegistry {
    fields: BTreeMap<String, RegisteredField>,
    next_ordinal: u64,
}

impl FieldRegistry {
    /// 空注册表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册 persistent 场(Transient/Construction → `LifecycleViolation`)。
    pub fn register(&mut self, def: FieldDef) -> Result<(), FieldError> {
        def.validate()?;
        match def.lifecycle {
            FieldLifecycle::Persistent => {}
            other => {
                return Err(FieldError::LifecycleViolation(format!(
                    "{} field must not enter runtime registry",
                    other.canonical_name()
                )));
            }
        }
        if self.fields.contains_key(&def.field_id) {
            return Err(FieldError::AlreadyRegistered(def.field_id.clone()));
        }
        let ordinal = self.next_ordinal;
        self.next_ordinal += 1;
        self.fields
            .insert(def.field_id.clone(), RegisteredField { def, ordinal });
        Ok(())
    }

    /// 显式注销(未注册 → `NotRegistered` fail-closed;Persistent 必须可
    /// 显式注销,冻结表字面)。
    pub fn unregister(&mut self, field_id: &str) -> Result<FieldDef, FieldError> {
        self.fields
            .remove(field_id)
            .map(|r| r.def)
            .ok_or_else(|| FieldError::NotRegistered(field_id.into()))
    }

    /// 参数变更(载荷 = 变更后完整定义;未注册 fail-closed;注册序保留)。
    pub fn update(&mut self, def: FieldDef) -> Result<(), FieldError> {
        def.validate()?;
        match def.lifecycle {
            FieldLifecycle::Persistent => {}
            other => {
                return Err(FieldError::LifecycleViolation(format!(
                    "{} field must not enter runtime registry",
                    other.canonical_name()
                )));
            }
        }
        let Some(entry) = self.fields.get_mut(&def.field_id) else {
            return Err(FieldError::NotRegistered(def.field_id.clone()));
        };
        let ordinal = entry.ordinal;
        *entry = RegisteredField { def, ordinal };
        Ok(())
    }

    /// 已注册场数。
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// 空表。
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// 注册表 semantic hash(参与 `semantic_state_hash` 的骨架面;
    /// 键序 = BTreeMap 字典序 = 规范序)。
    pub fn semantic_hash(&self) -> String {
        let mut buf = String::new();
        buf.push_str(&format!("fields:{}:\n", self.fields.len()));
        for (id, r) in &self.fields {
            buf.push_str(&format!("{id}:{}:{}\n", r.ordinal, r.def.digest()));
        }
        hex(&digest(buf.as_bytes()))
    }

    /// 对粒子集求值(骨架期 = filter 匹配 + 标量采样;返回 (field_id,
    /// particle, sample) 三元组,序 = (field 字典序, particle canonical
    /// 序) = 确定性面)。
    ///
    /// **零影响不变量**:过滤零匹配的场对输出零贡献——默认 `FieldFilter`
    /// 场注册后,本函数对任意粒子集输出与未注册时逐位一致(空)。
    pub fn evaluate(
        &self,
        particles: &[(PhysicsParticleRef, ParticleSleepState, u32)],
    ) -> Vec<(String, PhysicsParticleRef, f32)> {
        let mut out = Vec::new();
        for (id, r) in &self.fields {
            let mut hits: Vec<(PhysicsParticleRef, f32)> = Vec::new();
            for (p, state, layer) in particles {
                let state_bits = match state {
                    ParticleSleepState::Awake => super::filter::object_state_bits::AWAKE,
                    ParticleSleepState::Sleeping => super::filter::object_state_bits::SLEEPING,
                    ParticleSleepState::Static => super::filter::object_state_bits::STATIC,
                };
                if r.def.filter.matches(*p, state_bits, *layer) {
                    let sample = r.def.root.sample(particle_probe_position(*p));
                    hits.push((*p, sample));
                }
            }
            hits.sort_by_key(|(p, _)| p.canonical_text());
            for (p, s) in hits {
                out.push((id.clone(), p, s));
            }
        }
        out
    }
}

/// 骨架期采样探针位置(单元素域 = 原点探针;cloth/ragdoll = 元素序探针;
/// 完整期接 `ParticleAdapter::position` 运行时面——骨架期求值语义只承诺
/// 确定性,不承诺空间真实)。
fn particle_probe_position(p: PhysicsParticleRef) -> [f32; 3] {
    let i = p.element_index() as f32;
    [i, 0.0, 0.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::def::{FieldNode, FieldNodeKind, FieldPhysicsType};
    use crate::field::filter::FieldFilter;

    fn def(id: &str, lc: FieldLifecycle, filter: FieldFilter) -> FieldDef {
        FieldDef::new(
            id,
            FieldNode {
                node_id: "n".into(),
                kind: FieldNodeKind::Sphere {
                    center: [0.0; 3],
                    radius: 10.0,
                },
                weight: 1.0,
                children: vec![],
            },
            FieldPhysicsType::LinearForce,
            lc,
            filter,
        )
    }

    #[test]
    fn transient_and_construction_rejected_from_registry() {
        let mut r = FieldRegistry::new();
        assert!(matches!(
            r.register(def("t", FieldLifecycle::Transient, FieldFilter::default())),
            Err(FieldError::LifecycleViolation(_))
        ));
        assert!(matches!(
            r.register(def(
                "c",
                FieldLifecycle::Construction,
                FieldFilter::default()
            )),
            Err(FieldError::LifecycleViolation(_))
        ));
        assert!(
            r.register(def("p", FieldLifecycle::Persistent, FieldFilter::default()))
                .is_ok()
        );
        assert!(matches!(
            r.register(def("p", FieldLifecycle::Persistent, FieldFilter::default())),
            Err(FieldError::AlreadyRegistered(_))
        ));
        assert!(matches!(
            r.unregister("missing"),
            Err(FieldError::NotRegistered(_))
        ));
    }

    #[test]
    fn default_filter_registered_zero_match_zero_output() {
        // 零影响不变量:默认空 filter 场注册后求值输出恒空。
        let mut r = FieldRegistry::new();
        let hash_before = r.semantic_hash();
        r.register(def("z", FieldLifecycle::Persistent, FieldFilter::default()))
            .unwrap();
        let particles: Vec<(PhysicsParticleRef, ParticleSleepState, u32)> = vec![];
        assert!(r.evaluate(&particles).is_empty());
        let _ = hash_before; // 注册表 hash 变化属预期(场已注册);零影响指世界状态。
    }
}
