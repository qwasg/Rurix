//! External plain-data 域 adapter(G35-6 particle_view GPU↔host 双向桥
//! 方向 A;门 g35.wave6.events,RFC-0049 §4.9;RFC-0024 统一 particle view
//! 语义面引用不扩)。
//!
//! 消费面:渲染侧 GPU 粒子 readback 子集(rurix-render
//! `particles::events::GpuParticleSnapshot`)拆成 **plain 三流**
//! (positions/velocities/ids)喂本 adapter——**本 crate 不新增对渲染 crate
//! device 面的依赖,适配器只吃 plain 数据**(依赖方向纪律 RFC-0017 §4.B1-1
//! 不动)。roundtrip 判别(门 fact):GPU 九流 readback → snapshot → 本
//! 视图逐粒子读 == readback 原值**位级**(反打 Niagara GPU↔CPU 互读静默
//! 失败——失效句柄/域错配/越界一律确定性 `Err(NoSuchParticle)`,禁伪值)。
//!
//! ## v1 演示域登记(诚实边界;禁 stub 冒充运行时)
//! - RFC-0024 §4.A 五域枚举冻结(`GpuParticle` 域不在闭集,mod.rs 头注
//!   「GPU 副轨粒子不进本抽象」为骨架期边界);G35-6 v1 以 **ClothVertex
//!   域名义寻址**外部集合:`stable_id` = 调用方集合位表示(collection
//!   bits,与真实布料资产 digest 域隔离由调用方保证)、`element_index` =
//!   快照数组下标(「纯逻辑序,非 arena index」语义逐字吻合)。生产期
//!   六域扩枚举(ExternalParticle 域)走 RFC-0024 契约修订,本 adapter
//!   届时 0-byte 换 ref 构造口。
//! - `mass()` 诚实 `Err(Rejected("mass_not_in_snapshot(External)…"))`
//!   ——plain 快照无质量字段,不伪造有限质量(rigid_body_adapter 同律)。
//! - `set_force_impulse` = **记账台账**(v1):外部数据无求解器,写路径
//!   记 `(element_index, ImpulseWrite)` 序列由消费方 [`Self::drain_impulses`]
//!   取走折算(方向 B 装配输入:台账 → host 事件 → EventQueue → GPU 发射;
//!   G35-6 方向 B 演示用合成事件,不真接物理世界);**不改写快照 pos/vel**
//!   (快照 = 只读事实源,trait 无 transform 直写面的结构性保证维持)。

use std::collections::HashSet;

use crate::capture::canonical::CaptureError;

use super::{
    ClothStableId, ImpulseWrite, NO_SUCH_PARTICLE_LITERAL, ParticleAdapter, ParticleDomain,
    ParticleSleepState, PhysicsParticleRef, expect_domain,
};

/// External plain-data adapter(持有 plain 三流 Vec;写 = impulse 记账台账)。
pub struct ExternalParticlesAdapter {
    positions: Vec<[f32; 3]>,
    velocities: Vec<[f32; 3]>,
    ids: Vec<u32>,
    collection_bits: u64,
    impulse_ledger: Vec<(u32, ImpulseWrite)>,
}

impl ExternalParticlesAdapter {
    /// 自 plain 三流构造(渲染侧快照拆流喂入;边界校验 fail-closed:
    /// 三流等长 + ids 无重复——重复 pid 的定址歧义在构造口消灭,禁静默
    /// 首匹配充解析)。
    pub fn new(
        positions: Vec<[f32; 3]>,
        velocities: Vec<[f32; 3]>,
        ids: Vec<u32>,
        collection_bits: u64,
    ) -> Result<Self, CaptureError> {
        if positions.len() != ids.len() || velocities.len() != ids.len() {
            return Err(CaptureError::Rejected(format!(
                "external adapter: 三流长度不一致(pos {} / vel {} / ids {})",
                positions.len(),
                velocities.len(),
                ids.len()
            )));
        }
        let mut seen: HashSet<u32> = HashSet::with_capacity(ids.len());
        for &pid in &ids {
            if !seen.insert(pid) {
                return Err(CaptureError::Rejected(format!(
                    "external adapter: 重复 pid {pid}(定址歧义 fail-closed)"
                )));
            }
        }
        Ok(Self {
            positions,
            velocities,
            ids,
            collection_bits,
            impulse_ledger: Vec::new(),
        })
    }

    /// 粒子数。
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// 是否空。
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// 集合位表示(构造口传入的 collection bits)。
    pub fn collection_bits(&self) -> u64 {
        self.collection_bits
    }

    /// 下标 → ref 构造口(element_index = 快照数组下标,纯逻辑序;越界 ref
    /// 在解析侧确定性 `Err(NoSuchParticle)`)。
    pub fn ref_of_index(&self, index: u32) -> PhysicsParticleRef {
        PhysicsParticleRef::ClothVertex {
            stable_id: ClothStableId::from_bits(self.collection_bits),
            element_index: index,
        }
    }

    /// pid → ref 解析口(线性稳定扫;构造口保证 pid 无重复 ⇒ 解析确定;
    /// 未知 pid → `None`,不伪造)。
    pub fn ref_of_pid(&self, pid: u32) -> Option<PhysicsParticleRef> {
        self.ids
            .iter()
            .position(|&p| p == pid)
            .map(|i| self.ref_of_index(i as u32))
    }

    /// impulse 记账台账取走(方向 B 装配输入;记账序 = 写调用序)。
    pub fn drain_impulses(&mut self) -> Vec<(u32, ImpulseWrite)> {
        std::mem::take(&mut self.impulse_ledger)
    }

    /// ref → 快照下标解析(域门禁 + 集合位表示门禁 + 越界门禁,一律
    /// fail-closed `NoSuchParticle`)。
    fn index_of(&self, particle: PhysicsParticleRef) -> Result<usize, CaptureError> {
        expect_domain(particle, ParticleDomain::ClothVertex)?;
        let PhysicsParticleRef::ClothVertex {
            stable_id,
            element_index,
        } = particle
        else {
            return Err(CaptureError::Rejected(NO_SUCH_PARTICLE_LITERAL.into()));
        };
        if stable_id.to_bits() != self.collection_bits {
            return Err(CaptureError::Rejected(format!(
                "{NO_SUCH_PARTICLE_LITERAL}: external collection bits mismatch"
            )));
        }
        let i = element_index as usize;
        if i >= self.ids.len() {
            return Err(CaptureError::Rejected(format!(
                "{NO_SUCH_PARTICLE_LITERAL}: external element {element_index} out of {}",
                self.ids.len()
            )));
        }
        Ok(i)
    }
}

impl ParticleAdapter for ExternalParticlesAdapter {
    fn mass(&self, particle: PhysicsParticleRef) -> Result<f32, CaptureError> {
        // 诚实边界:plain 快照无质量字段;不伪造(rigid_body_adapter 同律)。
        let _i = self.index_of(particle)?;
        Err(CaptureError::Rejected(
            "mass_not_in_snapshot(External): plain snapshot has no mass field; \
             external particles carry pos/vel/id only"
                .into(),
        ))
    }

    fn position(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        let i = self.index_of(particle)?;
        Ok(self.positions[i])
    }

    fn velocity(&self, particle: PhysicsParticleRef) -> Result<[f32; 3], CaptureError> {
        let i = self.index_of(particle)?;
        Ok(self.velocities[i])
    }

    fn set_force_impulse(
        &mut self,
        particle: PhysicsParticleRef,
        write: ImpulseWrite,
    ) -> Result<(), CaptureError> {
        let i = self.index_of(particle)?;
        // v1 记账台账(模块头注):不改写快照 pos/vel(只读事实源),写
        // 序列由消费方 drain 折算为 host 事件(方向 B 装配输入)。
        self.impulse_ledger.push((i as u32, write));
        Ok(())
    }

    fn sleep_state(
        &self,
        particle: PhysicsParticleRef,
    ) -> Result<ParticleSleepState, CaptureError> {
        let _i = self.index_of(particle)?;
        // 外部快照 = 活跃 GPU 粒子子集(死亡粒子已被稳定压缩剔除)。
        Ok(ParticleSleepState::Awake)
    }

    fn skeleton_boundary(&self) -> &'static str {
        "external_plain_data(v1_demo_domain=ClothVertex/collection_bits);\
         mass=none;impulse=ledger_v1;snapshot_readonly"
    }
}

#[cfg(test)]
mod tests {
    use super::super::ChunkStableId;
    use super::*;

    /// 夹具:非平凡位型(含 −0.0/极小次正规邻域值,位级面强判据)。
    fn fixture() -> ExternalParticlesAdapter {
        ExternalParticlesAdapter::new(
            vec![
                [1.5, -0.0, 3.25e-7],
                [-2.75, 4.125, 0.0],
                [9.0e-30, -8.5, 0.015625],
            ],
            vec![
                [0.5, -1.25, 2.0],
                [-0.0, 0.75, -3.5],
                [1.0e-20, 0.0, -0.125],
            ],
            vec![41, 7, 900_001],
            0xE35_6_BEEF,
        )
        .expect("夹具构造必绿")
    }

    /// ① roundtrip 位级:逐下标 ref 读 position/velocity == 原 plain 流
    /// (to_bits 全等,−0.0 面强于 ==)。
    #[test]
    fn roundtrip_bitexact_via_index_refs() {
        let a = fixture();
        let want_pos = [
            [1.5f32, -0.0, 3.25e-7],
            [-2.75, 4.125, 0.0],
            [9.0e-30, -8.5, 0.015625],
        ];
        let want_vel = [
            [0.5f32, -1.25, 2.0],
            [-0.0, 0.75, -3.5],
            [1.0e-20, 0.0, -0.125],
        ];
        for i in 0..a.len() {
            let r = a.ref_of_index(i as u32);
            let p = a.position(r).expect("位置读必绿");
            let v = a.velocity(r).expect("速度读必绿");
            for k in 0..3 {
                assert_eq!(p[k].to_bits(), want_pos[i][k].to_bits(), "pos[{i}][{k}] 非位级");
                assert_eq!(v[k].to_bits(), want_vel[i][k].to_bits(), "vel[{i}][{k}] 非位级");
            }
        }
    }

    /// ② pid 定址:命中/未知 None;canonical_text 走 ClothVertex 字面
    /// (v1 演示域登记面)。
    #[test]
    fn pid_addressing_resolves_and_unknown_is_none() {
        let a = fixture();
        let r = a.ref_of_pid(900_001).expect("池内 pid 必命中");
        assert_eq!(a.position(r).unwrap()[1].to_bits(), (-8.5f32).to_bits());
        assert_eq!(r.element_index(), 2);
        assert_eq!(r.domain(), ParticleDomain::ClothVertex);
        assert!(r.canonical_text().starts_with("ClothVertex:"));
        assert_eq!(a.ref_of_pid(123_456), None, "未知 pid 必 None(不伪造)");
    }

    /// ③ fail-closed 三门:域错配 / 集合位表示错配 / 越界 element ——
    /// 一律确定性 NoSuchParticle(反打互读静默失败)。
    #[test]
    fn fail_closed_wrong_domain_collection_and_range() {
        let a = fixture();
        let wrong_domain = PhysicsParticleRef::DestructionChunk(ChunkStableId::from_bits(41));
        let e = a.position(wrong_domain).unwrap_err();
        assert!(e.to_string().contains(NO_SUCH_PARTICLE_LITERAL));
        let wrong_collection = PhysicsParticleRef::ClothVertex {
            stable_id: ClothStableId::from_bits(0xDEAD),
            element_index: 0,
        };
        let e = a.velocity(wrong_collection).unwrap_err();
        assert!(e.to_string().contains(NO_SUCH_PARTICLE_LITERAL));
        let out_of_range = a.ref_of_index(3);
        let e = a.position(out_of_range).unwrap_err();
        assert!(e.to_string().contains(NO_SUCH_PARTICLE_LITERAL));
    }

    /// ④ mass 诚实边界:合法 ref 亦 Err(mass_not_in_snapshot),不伪造
    /// 有限质量;失效 ref 走 NoSuchParticle 先行。
    #[test]
    fn mass_is_honest_error() {
        let a = fixture();
        let e = a.mass(a.ref_of_index(0)).unwrap_err();
        assert!(e.to_string().contains("mass_not_in_snapshot(External)"));
        let e2 = a.mass(a.ref_of_index(99)).unwrap_err();
        assert!(e2.to_string().contains(NO_SUCH_PARTICLE_LITERAL));
    }

    /// ⑤ impulse 记账台账:写序保序累计、drain 取走清空、失效 ref 写
    /// Err 且台账 0 污染;快照 pos/vel 0 改写(只读事实源)。
    #[test]
    fn impulse_ledger_ordered_and_snapshot_untouched() {
        let mut a = fixture();
        let p0_before = a.position(a.ref_of_index(0)).unwrap();
        a.set_force_impulse(a.ref_of_index(1), ImpulseWrite::Linear([1.0, 0.0, 0.0]))
            .unwrap();
        a.set_force_impulse(a.ref_of_index(0), ImpulseWrite::Force([0.0, -2.0, 0.5]))
            .unwrap();
        let bad = a.ref_of_index(7);
        assert!(a.set_force_impulse(bad, ImpulseWrite::Linear([9.0; 3])).is_err());
        let ledger = a.drain_impulses();
        assert_eq!(ledger.len(), 2, "失效写不入台账");
        assert_eq!(ledger[0].0, 1);
        assert_eq!(ledger[1].0, 0);
        assert!(matches!(ledger[0].1, ImpulseWrite::Linear([x, _, _]) if x == 1.0));
        assert!(matches!(ledger[1].1, ImpulseWrite::Force([_, y, _]) if y == -2.0));
        assert!(a.drain_impulses().is_empty(), "drain 取走清空");
        let p0_after = a.position(a.ref_of_index(0)).unwrap();
        for k in 0..3 {
            assert_eq!(
                p0_after[k].to_bits(),
                p0_before[k].to_bits(),
                "写路径不改写快照(只读事实源)"
            );
        }
    }

    /// ⑥ 构造口 fail-closed:三流长度不一致 / 重复 pid 必拒(定址歧义
    /// 在构造口消灭)。
    #[test]
    fn constructor_rejects_mismatch_and_duplicate_ids() {
        assert!(
            ExternalParticlesAdapter::new(vec![[0.0; 3]; 2], vec![[0.0; 3]; 3], vec![1, 2], 9)
                .is_err(),
            "三流长度不一致必拒"
        );
        assert!(
            ExternalParticlesAdapter::new(
                vec![[0.0; 3]; 2],
                vec![[0.0; 3]; 2],
                vec![5, 5],
                9
            )
            .is_err(),
            "重复 pid 必拒"
        );
    }

    /// ⑦ sleep/boundary 登记面:合法 ref 恒 Awake;骨架边界字面在档。
    #[test]
    fn sleep_awake_and_boundary_registered() {
        let a = fixture();
        assert_eq!(
            a.sleep_state(a.ref_of_index(1)).unwrap(),
            ParticleSleepState::Awake
        );
        assert!(a.sleep_state(a.ref_of_index(9)).is_err());
        let b = a.skeleton_boundary();
        assert!(b.contains("external_plain_data"));
        assert!(b.contains("impulse=ledger_v1"));
        assert!(!a.is_empty());
        assert_eq!(a.collection_bits(), 0xE35_6_BEEF);
    }
}
