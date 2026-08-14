//! 过滤一等公民(RFC-0024 §4.B3;默认空集匹配 = 零影响不变量)。
//!
//! `FieldFilter = (object_state_mask × domain_mask × layer_mask ×
//! explicit_include/exclude)`;**默认空集匹配 = 无影响**,拒绝「默认全
//! 影响」语义——`FieldFilter::default()` 的 `matches_any` 恒 false。
//! filter 是场定义的一部分,进 digest(canonical_json 进 `FieldDef` 前像)。

use crate::particle_view::{ParticleDomain, PhysicsParticleRef};

/// 对象状态掩码位(骨架期三态;睡眠/静态语义对齐 `ParticleSleepState`)。
pub mod object_state_bits {
    /// 活跃。
    pub const AWAKE: u64 = 1 << 0;
    /// 睡眠。
    pub const SLEEPING: u64 = 1 << 1;
    /// 静态/锚定。
    pub const STATIC: u64 = 1 << 2;
}

/// 对象状态掩码 newtype(canonical 面位字面)。
pub type ObjectStateMask = u64;

/// 域掩码位(五域一一对应;位序 = `ParticleDomain::ALL` 声明序)。
pub fn domain_bit(domain: ParticleDomain) -> u64 {
    match domain {
        ParticleDomain::RigidBody => 1 << 0,
        ParticleDomain::ClothVertex => 1 << 1,
        ParticleDomain::DestructionChunk => 1 << 2,
        ParticleDomain::RagdollNode => 1 << 3,
        ParticleDomain::CharacterInner => 1 << 4,
    }
}

/// 域掩码合法位(五域闭集;骨架期 domain_mask 只允许低 5 位)。
pub const DOMAIN_MASK_VALID: u64 = 0b1_1111;

/// `ParticleSleepState` → 对象状态掩码位(完整期耦合面与骨架期 `evaluate`
/// 共用同一映射,RXS-0374 L1)。
pub fn sleep_state_bits(state: crate::particle_view::ParticleSleepState) -> u64 {
    match state {
        crate::particle_view::ParticleSleepState::Awake => object_state_bits::AWAKE,
        crate::particle_view::ParticleSleepState::Sleeping => object_state_bits::SLEEPING,
        crate::particle_view::ParticleSleepState::Static => object_state_bits::STATIC,
    }
}

/// 场过滤(四元组;`Default` = 全空集 = 零匹配零影响,RFC-0024 §4.B3
/// 拒绝「默认全影响」语义的类型面落地)。
#[derive(Debug, Clone, PartialEq)]
pub struct FieldFilter {
    /// 对象状态掩码(0 = 不匹配任何状态)。
    pub object_state_mask: ObjectStateMask,
    /// 域掩码(0 = 不匹配任何域;只允许低 5 位,越位 schema 校验拒)。
    pub domain_mask: u64,
    /// 层掩码(0 = 不匹配任何层;对物理 body layer 语义)。
    pub layer_mask: u64,
    /// 显式包含(particle canonical_text 字面;掩码未命中时的强包含)。
    pub explicit_include: Vec<String>,
    /// 显式排除(优先于一切包含;canonical_text 字面)。
    pub explicit_exclude: Vec<String>,
}

impl Default for FieldFilter {
    /// **默认空集匹配 = 无影响**:全掩码 0、无显式包含——`matches` 恒
    /// false(零影响不变量的默认值面,RFC-0024 §4.B3 冻结语义)。
    fn default() -> Self {
        Self {
            object_state_mask: 0,
            domain_mask: 0,
            layer_mask: 0,
            explicit_include: Vec::new(),
            explicit_exclude: Vec::new(),
        }
    }
}

impl FieldFilter {
    /// 域掩码合法性(只允许五域低 5 位;schema 校验面)。
    pub fn domain_mask_valid(&self) -> bool {
        self.domain_mask & !DOMAIN_MASK_VALID == 0
    }

    /// 匹配判定(显式排除 > 显式包含 > 掩码交;**掩码全 0 且无显式包含
    /// = 恒 false** = 默认零影响)。
    ///
    /// `object_state_bits`: 该 particle 当前状态位;`layer`: 域内层
    /// (刚体 = body layer;其余域骨架期恒 0)。
    pub fn matches(
        &self,
        particle: PhysicsParticleRef,
        object_state_bits: u64,
        layer: u32,
    ) -> bool {
        let canonical = particle.canonical_text();
        if self.explicit_exclude.iter().any(|e| e == &canonical) {
            return false;
        }
        if self.explicit_include.iter().any(|e| e == &canonical) {
            return true;
        }
        // 掩码路径:三掩码全须命中(与语义);任一掩码为 0 = 该维度拒绝
        // ——默认全 0 → 零匹配。
        if self.object_state_mask == 0 || self.domain_mask == 0 || self.layer_mask == 0 {
            return false;
        }
        (self.object_state_mask & object_state_bits) != 0
            && (self.domain_mask & domain_bit(particle.domain())) != 0
            && (self.layer_mask & (1u64 << (layer as u64 % 64))) != 0
    }

    /// canonical JSON(进 FieldDef digest 前像;显式集合排序去重 = 规范序)。
    pub fn canonical_json(&self) -> String {
        let mut inc = self.explicit_include.clone();
        inc.sort();
        inc.dedup();
        let mut exc = self.explicit_exclude.clone();
        exc.sort();
        exc.dedup();
        let mut s = format!(
            "{{\"object_state_mask\":{},\"domain_mask\":{},\"layer_mask\":{},\"include\":[",
            self.object_state_mask, self.domain_mask, self.layer_mask
        );
        for (i, e) in inc.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "\"{}\"",
                e.replace('\\', "\\\\").replace('"', "\\\"")
            ));
        }
        s.push_str("],\"exclude\":[");
        for (i, e) in exc.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "\"{}\"",
                e.replace('\\', "\\\\").replace('"', "\\\"")
            ));
        }
        s.push_str("]}");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particle_view::ChunkStableId;

    fn chunk_ref() -> PhysicsParticleRef {
        PhysicsParticleRef::DestructionChunk(ChunkStableId(42))
    }

    #[test]
    fn default_empty_matches_nothing_zero_impact() {
        let f = FieldFilter::default();
        // 默认空集匹配 = 无影响(冻结语义):任意状态/层/域恒 false。
        assert!(!f.matches(chunk_ref(), object_state_bits::AWAKE, 0));
        assert!(!f.matches(chunk_ref(), u64::MAX, 63));
    }

    #[test]
    fn explicit_exclude_beats_include() {
        let c = chunk_ref();
        let mut f = FieldFilter {
            object_state_mask: object_state_bits::AWAKE,
            domain_mask: domain_bit(ParticleDomain::DestructionChunk),
            layer_mask: u64::MAX,
            explicit_include: vec![c.canonical_text()],
            explicit_exclude: vec![c.canonical_text()],
        };
        assert!(!f.matches(c, object_state_bits::AWAKE, 0));
        f.explicit_exclude.clear();
        assert!(f.matches(c, object_state_bits::AWAKE, 0));
        // 域掩码错位 → 拒。
        f.domain_mask = domain_bit(ParticleDomain::RigidBody);
        assert!(
            f.matches(c, object_state_bits::AWAKE, 0),
            "explicit include 仍命中"
        );
        f.explicit_include.clear();
        assert!(!f.matches(c, object_state_bits::AWAKE, 0));
    }

    #[test]
    fn mask_outside_five_domains_invalid() {
        let mut f = FieldFilter {
            domain_mask: 1 << 5,
            ..FieldFilter::default()
        };
        assert!(!f.domain_mask_valid());
        f.domain_mask = DOMAIN_MASK_VALID;
        assert!(f.domain_mask_valid());
    }
}
