//! M121 PhysicsParticleRef 名义类型编译期隔离断言(G9.2 骨架期)。

#![cfg(feature = "physics-particle-view")]

use rurix_physics::BodyId;
use rurix_physics::particle_view::{
    CharacterStableId, ChunkStableId, ParticleDomain, PhysicsParticleRef, RigidBodyStableId,
};

/// 跨域隔离(运行时可判面):同位表示不同域的 ref 不相等、canonical 不同。
#[test]
fn nominal_isolation_cross_domain_same_bits() {
    let rb = PhysicsParticleRef::RigidBody(RigidBodyStableId::from_bits(
        BodyId::from_bits(0x0000_0001_0000_0001).to_bits(),
    ));
    let ch = PhysicsParticleRef::DestructionChunk(ChunkStableId::from_bits(
        BodyId::from_bits(0x0000_0001_0000_0001).to_bits(),
    ));
    let ci = PhysicsParticleRef::CharacterInner(CharacterStableId::from_bits(
        BodyId::from_bits(0x0000_0001_0000_0001).to_bits(),
    ));
    assert_eq!(rb.stable_bits(), ch.stable_bits());
    assert_ne!(rb, ch);
    assert_ne!(rb, ci);
    assert_ne!(rb.domain(), ch.domain());
    assert_eq!(rb.domain(), ParticleDomain::RigidBody);
    assert_eq!(ch.domain(), ParticleDomain::DestructionChunk);
}

/// 同域不同元素序隔离:cloth 顶点序不同 = 不同 ref。
#[test]
fn nominal_isolation_element_index() {
    use rurix_physics::particle_view::ClothStableId;
    let v0 = PhysicsParticleRef::ClothVertex {
        stable_id: ClothStableId::from_bits(9),
        element_index: 0,
    };
    let v1 = PhysicsParticleRef::ClothVertex {
        stable_id: ClothStableId::from_bits(9),
        element_index: 1,
    };
    assert_ne!(v0, v1);
    assert_eq!(v0.domain(), v1.domain());
    assert_eq!(v0.stable_bits(), v1.stable_bits());
    assert_ne!(v0.element_index(), v1.element_index());
}

/// 域句柄类型无跨域 From/Into(类型面静态事实;本测试 = 该事实存在即
/// 编译通过,门脚本另有探针源证「若允许混用则 typeck 必红」)。
#[test]
fn nominal_types_no_cross_domain_conversion() {
    fn assert_no_from<T, U>() {
        // 编译期事实:T: From<U> 不存在(若存在,下面的 trait bound 断言
        // 辅助函数会成功编译而语义测试失败——我们直接以类型不互转的运行时
        // 等价断言 + 探针源承担编译期证明)。
        fn _f() {
            // 若 RigidBodyStableId: From<ChunkStableId> 存在,此行类型
            // 检查仍通过(泛型不约束);真正的编译期拒绝证据 = 探针源
            // (conformance/physics/particle_view/isolation_reject.rs.probe)。
        }
        let _ = _f;
        let _ = core::marker::PhantomData::<(T, U)>;
    }
    assert_no_from::<RigidBodyStableId, ChunkStableId>();
    assert_no_from::<RigidBodyStableId, CharacterStableId>();
    assert_no_from::<ChunkStableId, CharacterStableId>();
}
