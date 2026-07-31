//! `PhysicsWorld`(RFC-0017 §4.A1~A6 冻结接口实现)。
//!
//! 结构纪律:
//! - 公共方法 = 前置纯校验(两构建档一致)+ 后端分派(`*_inner`,cfg 按 feature);
//!   `--no-default-features` 档 `new` 全路径确定性 `Err(BackendNotCompiled)`,
//!   世界类型面不可构造,inner 桩为全函数 Err,零 panic 路径(P-01)。
//! - safe 层自维护:`BodyId`↔Jolt token 映射、body→`ShapeId` 记录(`QueryHit.shape`
//!   回填源)、接触事件归一化与有界 ring、预算饱和计数;sys 层只过 u64 token(§4.C3)。
//! - 相位纪律(§4.A4 Q-B):`step`/`add_*`/`remove_*`/`apply_impulse`/`drain_contacts`
//!   取 `&mut self`(step 相位);cast 查询/变换读取 `&self`,step 外多线程全并发
//!   (每线程持自己的 `SyncBudget`,`&mut` 不跨线程共享)。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "jolt")]
use rurix_physics_sys::{
    SysBodyDesc, SysBodyKind, SysContactEvent, SysContactPhase, SysError, SysErrorCode, SysHit,
    SysRay, SysShapeParams, SysTransform, SysWorld, SysWorldDesc,
};

use crate::arena::GenArena;
use crate::budget::SyncBudget;
use crate::error::PhysicsError;
use crate::events::ContactRing;
#[cfg(feature = "jolt")]
use crate::events::normalize_contacts;
use crate::id::{BodyId, ShapeId};
#[cfg(feature = "jolt")]
use crate::order::{sort_overlap_hits, sort_query_hits};
use crate::types::{
    BackendKind, BodyDesc, ContactEvent, OverlapHit, PhysicsTransform, QueryHit, QueryRay,
    QueryShape, ShapeDesc, StepStats, WorldDesc,
};
#[cfg(feature = "jolt")]
use crate::types::{BodyKind, ContactPhase};

/// 预算饱和计数(单调累计快照;§4.A6「饱和计数上报」出口,计数进 evidence 不进硬门)。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BudgetSaturation {
    /// 因 `max_query_casts` 耗尽被确定性截断(返回空结果)的 cast 次数。
    pub query_casts: u64,
    /// 因 `max_contact_events` 耗尽被 drain 截断的接触事件条数。
    pub contact_events: u64,
    /// 因 `max_body_writes` 耗尽被截断的 body 写次数(消费方 = G6.3 同步桥,本切片保留)。
    pub body_writes: u64,
}

/// body 槽位负载:Jolt token(safe ↔ sys 映射键)+ body→shape 记录。
#[derive(Debug)]
struct BodyEntry {
    token: u64,
    shape: ShapeId,
}

/// 一对已占槽位:(body (index, generation), shape (index, generation))。
#[cfg(feature = "jolt")]
type BodyShapeSlots = ((u32, u32), (u32, u32));

/// 物理世界(冻结接口 §4.A1;宿主只握此类型与不透明句柄,永不见原生指针)。
///
/// 线程纪律:`&self` 查询面可 step 外多线程并发(`Sync`);`&mut self` 面由宿主
/// 串行驱动(accumulator 在宿主,§3)。
pub struct PhysicsWorld {
    desc: WorldDesc,
    bodies: GenArena<BodyEntry>,
    shapes: GenArena<()>,
    token_map: HashMap<u64, BodyId>,
    ring: ContactRing,
    sat_query_casts: AtomicU64,
    sat_contact_events: u64,
    sat_body_writes: u64,
    #[cfg(feature = "jolt")]
    sys: SysWorld,
}

impl std::fmt::Debug for PhysicsWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhysicsWorld")
            .field("backend", &self.desc.backend)
            .field("max_bodies", &self.desc.max_bodies)
            .finish_non_exhaustive()
    }
}

impl PhysicsWorld {
    /// 创建世界(§4.A1)。确定性失败路径(P-01,不静默回退):
    /// 描述非法 → `Err(InvalidDesc)`;后端未编译(Jolt 无 feature / Rapier 在
    /// G6.4 前)→ `Err(BackendNotCompiled)`;后端初始化失败 → `Err(BackendUnavailable)`。
    pub fn new(desc: WorldDesc) -> Result<Self, PhysicsError> {
        desc.validate()?;
        match desc.backend {
            BackendKind::Rapier => Err(PhysicsError::BackendNotCompiled(BackendKind::Rapier)),
            BackendKind::Jolt => Self::new_jolt(desc),
        }
    }

    #[cfg(feature = "jolt")]
    fn new_jolt(desc: WorldDesc) -> Result<Self, PhysicsError> {
        let sys = SysWorld::create(&SysWorldDesc {
            gravity: desc.gravity,
            layer_count: desc.layer_count,
            max_bodies: desc.max_bodies,
            job_threads: desc.job_threads.unwrap_or(0),
            contact_capacity: desc.contact_capacity,
        })
        .map_err(physics_error_from_sys)?;
        Ok(Self::assemble(desc, sys))
    }

    #[cfg(not(feature = "jolt"))]
    fn new_jolt(_desc: WorldDesc) -> Result<Self, PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(BackendKind::Jolt))
    }

    #[cfg(feature = "jolt")]
    fn assemble(desc: WorldDesc, sys: SysWorld) -> Self {
        let bodies = GenArena::with_capacity(desc.max_bodies);
        let shapes = GenArena::with_capacity(desc.max_bodies);
        let ring = ContactRing::with_capacity(desc.contact_capacity);
        PhysicsWorld {
            desc,
            bodies,
            shapes,
            token_map: HashMap::new(),
            ring,
            sat_query_casts: AtomicU64::new(0),
            sat_contact_events: 0,
            sat_body_writes: 0,
            sys,
        }
    }

    /// 世界配置(宿主锚 `dt_fixed`/layer 数等只读回访)。
    pub fn desc(&self) -> &WorldDesc {
        &self.desc
    }

    /// 固定步推进(§4.A1):`dt_fixed` 与 `WorldDesc::dt_fixed` 位级不一致 →
    /// 确定性 `Err(FixedStepMismatch)`(accumulator 在宿主,库内拒绝变步长)。
    /// step 结束边界:drain sys 原始事件 → 归一化(规范序排序去重)→ 入有界 ring。
    pub fn step(&mut self, dt_fixed: f32) -> Result<StepStats, PhysicsError> {
        validate_fixed_dt(self.desc.dt_fixed, dt_fixed)?;
        self.step_inner(dt_fixed)
    }

    #[cfg(feature = "jolt")]
    fn step_inner(&mut self, dt_fixed: f32) -> Result<StepStats, PhysicsError> {
        let stats = self.sys.step(dt_fixed).map_err(physics_error_from_sys)?;
        let (raw, sys_dropped) = self.sys.drain_contacts();
        let mut unmapped = 0u32;
        let mut events = Vec::with_capacity(raw.len());
        for e in raw {
            match self.map_sys_contact(e) {
                Some(ev) => events.push(ev),
                // 事件引用未知 token(如本步内已移除 body):无法命名,确定性丢弃计数。
                None => unmapped = unmapped.saturating_add(1),
            }
        }
        let normalized = normalize_contacts(events);
        let emitted = u32::try_from(normalized.len()).unwrap_or(u32::MAX);
        let dropped_ring = self.ring.push_normalized(normalized);
        Ok(StepStats {
            active_bodies: stats.active_bodies,
            slept_this_step: stats.slept_this_step,
            contacts_emitted: emitted,
            contacts_dropped: stats
                .contacts_dropped
                .saturating_add(sys_dropped)
                .saturating_add(dropped_ring)
                .saturating_add(unmapped),
            step_time: duration_from_secs(stats.step_time_secs),
        })
    }

    #[cfg(not(feature = "jolt"))]
    fn step_inner(&mut self, _dt_fixed: f32) -> Result<StepStats, PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(self.desc.backend))
    }

    /// 批插体(§4.A3:AddBodiesPrepare/Finalize 映射在 sys 层)。all-or-nothing:
    /// 任一描述非法 → `Err(InvalidDesc)`;池余量不足 → `Err(PoolExhausted)`;
    /// sys 失败 → 已占槽位回滚,无悬挂(P-01)。返回顺序与输入一一对应。
    pub fn add_bodies_batch(&mut self, descs: &[BodyDesc]) -> Result<Vec<BodyId>, PhysicsError> {
        for d in descs {
            d.validate(self.desc.layer_count)?;
        }
        self.add_bodies_inner(descs)
    }

    #[cfg(feature = "jolt")]
    fn add_bodies_inner(&mut self, descs: &[BodyDesc]) -> Result<Vec<BodyId>, PhysicsError> {
        let n = u32::try_from(descs.len())
            .map_err(|_| PhysicsError::InvalidDesc("批插数量超 u32 上限".into()))?;
        if self.bodies.remaining_capacity() < n || self.shapes.remaining_capacity() < n {
            return Err(PhysicsError::PoolExhausted);
        }
        // 先占槽(句柄不外泄),sys 成功后回填 token;任一失败整批回滚。
        let mut slots = Vec::with_capacity(descs.len());
        for _ in descs {
            match self.alloc_body_slots() {
                Ok(parts) => slots.push(parts),
                Err(e) => {
                    self.rollback_body_slots(&slots);
                    return Err(e);
                }
            }
        }
        let sys_descs: Vec<SysBodyDesc> = descs.iter().map(sys_body_desc).collect();
        let tokens = match self.sys.add_bodies_batch(&sys_descs) {
            Ok(t) => t,
            Err(e) => {
                self.rollback_body_slots(&slots);
                return Err(physics_error_from_sys(e));
            }
        };
        // sys 契约:返回 token 与输入一一对应(sys/lib.rs 边界注释)。
        debug_assert_eq!(tokens.len(), slots.len());
        let mut ids = Vec::with_capacity(descs.len());
        for ((body_parts, _), token) in slots.into_iter().zip(tokens) {
            let id = BodyId::new(body_parts.0, body_parts.1);
            if let Some(entry) = self.bodies.get_mut(body_parts.0, body_parts.1) {
                entry.token = token;
            }
            self.token_map.insert(token, id);
            ids.push(id);
        }
        Ok(ids)
    }

    #[cfg(not(feature = "jolt"))]
    fn add_bodies_inner(&mut self, _descs: &[BodyDesc]) -> Result<Vec<BodyId>, PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(self.desc.backend))
    }

    /// 批移除(§4.A2;失效句柄二次使用 → `Err(InvalidBody)`,all-or-nothing 不悬垂,
    /// §4.C3)。输入去重(重复 id 幂等,保持首现序)。`RemovalReceipt` 流送纪律
    /// (§4.B4)属 G6.3 合流层,本切片返回 `()`。
    pub fn remove_bodies_batch(&mut self, bodies: &[BodyId]) -> Result<(), PhysicsError> {
        let mut seen = HashSet::with_capacity(bodies.len());
        let mut unique = Vec::with_capacity(bodies.len());
        for id in bodies {
            self.body_token(*id)?;
            if seen.insert(*id) {
                unique.push(*id);
            }
        }
        self.remove_bodies_inner(&unique)
    }

    #[cfg(feature = "jolt")]
    fn remove_bodies_inner(&mut self, bodies: &[BodyId]) -> Result<(), PhysicsError> {
        let tokens: Vec<u64> = bodies
            .iter()
            .map(|id| self.body_token(*id))
            .collect::<Result<_, _>>()?;
        self.sys
            .remove_bodies_batch(&tokens)
            .map_err(physics_error_from_sys)?;
        for id in bodies {
            if let Some(entry) = self.bodies.remove(id.index(), id.generation()) {
                self.token_map.remove(&entry.token);
                self.shapes
                    .remove(entry.shape.index(), entry.shape.generation());
            }
        }
        Ok(())
    }

    #[cfg(not(feature = "jolt"))]
    fn remove_bodies_inner(&mut self, _bodies: &[BodyId]) -> Result<(), PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(self.desc.backend))
    }

    /// 读 body 当前变换(§4.A2;失效句柄 → `Err(InvalidBody)`)。
    pub fn body_transform(&self, body: BodyId) -> Result<PhysicsTransform, PhysicsError> {
        let token = self.body_token(body)?;
        self.body_transform_inner(token)
    }

    #[cfg(feature = "jolt")]
    fn body_transform_inner(&self, token: u64) -> Result<PhysicsTransform, PhysicsError> {
        self.sys
            .body_transform(token)
            .map_err(physics_error_from_sys)
            .map(transform_from_sys)
    }

    #[cfg(not(feature = "jolt"))]
    fn body_transform_inner(&self, _token: u64) -> Result<PhysicsTransform, PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(self.desc.backend))
    }

    /// 上一拍变换快照(§4.A4:step 结束边界提交的 active 动态/运动体变换浅拷贝;
    /// 仅数组,不复制加速结构)。返回按 `BodyId` 升序(确定性面);与 `render_exec`
    /// 同帧可读(G6_PLAN §2.3-2)。
    pub fn active_transforms(&self) -> Vec<(BodyId, PhysicsTransform)> {
        self.active_transforms_inner()
    }

    #[cfg(feature = "jolt")]
    fn active_transforms_inner(&self) -> Vec<(BodyId, PhysicsTransform)> {
        let mut out: Vec<(BodyId, PhysicsTransform)> = self
            .sys
            .active_transforms()
            .into_iter()
            .filter_map(|(token, t)| {
                self.token_map
                    .get(&token)
                    .map(|id| (*id, transform_from_sys(t)))
            })
            .collect();
        out.sort_by_key(|(id, _)| *id);
        out
    }

    #[cfg(not(feature = "jolt"))]
    fn active_transforms_inner(&self) -> Vec<(BodyId, PhysicsTransform)> {
        Vec::new()
    }

    /// 冲量施加(睡眠体冲量 → 唤醒,§4.A7 睡眠唤醒单测锚;`impulse` 须有限)。
    pub fn apply_impulse(&mut self, body: BodyId, impulse: [f32; 3]) -> Result<(), PhysicsError> {
        if !impulse.iter().all(|c| c.is_finite()) {
            return Err(PhysicsError::InvalidDesc("impulse 分量须有限".into()));
        }
        let token = self.body_token(body)?;
        self.apply_impulse_inner(token, impulse)
    }

    #[cfg(feature = "jolt")]
    fn apply_impulse_inner(&mut self, token: u64, impulse: [f32; 3]) -> Result<(), PhysicsError> {
        self.sys
            .apply_impulse(token, impulse)
            .map_err(physics_error_from_sys)
    }

    #[cfg(not(feature = "jolt"))]
    fn apply_impulse_inner(&mut self, _token: u64, _impulse: [f32; 3]) -> Result<(), PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(self.desc.backend))
    }

    /// body 是否激活(未睡眠;§4.A7 单测锚)。
    pub fn is_active(&self, body: BodyId) -> Result<bool, PhysicsError> {
        let token = self.body_token(body)?;
        self.is_active_inner(token)
    }

    #[cfg(feature = "jolt")]
    fn is_active_inner(&self, token: u64) -> Result<bool, PhysicsError> {
        self.sys.is_active(token).map_err(physics_error_from_sys)
    }

    #[cfg(not(feature = "jolt"))]
    fn is_active_inner(&self, _token: u64) -> Result<bool, PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(self.desc.backend))
    }

    /// 射线 cast(§4.A4:step 外并发,`&self`;全命中按 `(t, BodyId)` 规范序返回)。
    /// 每次调用消耗 1 次 `SyncBudget::max_query_casts`;耗尽 → 确定性截断(空结果)
    /// + 饱和计数,不 panic(P-01)。
    pub fn cast_ray(
        &self,
        ray: &QueryRay,
        budget: &mut SyncBudget,
    ) -> Result<Vec<QueryHit>, PhysicsError> {
        ray.validate()?;
        if !self.consume_query_budget(budget) {
            return Ok(Vec::new());
        }
        self.cast_ray_inner(ray)
    }

    #[cfg(feature = "jolt")]
    fn cast_ray_inner(&self, ray: &QueryRay) -> Result<Vec<QueryHit>, PhysicsError> {
        let hits = self.sys.cast_ray(&SysRay {
            origin: ray.origin,
            dir: ray.dir,
            t_min: ray.t_min,
            t_max: ray.t_max,
            layer_mask: ray.layer_mask,
        });
        Ok(self.map_sys_hits(hits))
    }

    #[cfg(not(feature = "jolt"))]
    fn cast_ray_inner(&self, _ray: &QueryRay) -> Result<Vec<QueryHit>, PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(self.desc.backend))
    }

    /// 形状 cast(§4.A4;并发/预算/规范序纪律同 `cast_ray`)。
    pub fn cast_shape(
        &self,
        query: &QueryShape,
        budget: &mut SyncBudget,
    ) -> Result<Vec<QueryHit>, PhysicsError> {
        query.validate()?;
        if !self.consume_query_budget(budget) {
            return Ok(Vec::new());
        }
        self.cast_shape_inner(query)
    }

    #[cfg(feature = "jolt")]
    fn cast_shape_inner(&self, query: &QueryShape) -> Result<Vec<QueryHit>, PhysicsError> {
        let hits = self.sys.cast_shape(
            &sys_shape(&query.shape),
            &sys_transform(query.start),
            query.dir,
            query.t_max,
            query.layer_mask,
        );
        Ok(self.map_sys_hits(hits))
    }

    #[cfg(not(feature = "jolt"))]
    fn cast_shape_inner(&self, _query: &QueryShape) -> Result<Vec<QueryHit>, PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(self.desc.backend))
    }

    /// 形状 overlap(§4.A4;并发/预算纪律同 `cast_ray`,规范序 = `BodyId` 升序)。
    pub fn overlap(
        &self,
        shape: &ShapeDesc,
        transform: &PhysicsTransform,
        layer_mask: u64,
        budget: &mut SyncBudget,
    ) -> Result<Vec<OverlapHit>, PhysicsError> {
        shape.validate_dims()?;
        if !transform
            .translation
            .iter()
            .chain(transform.rotation.iter())
            .all(|c| c.is_finite())
        {
            return Err(PhysicsError::InvalidDesc(
                "overlap transform 分量须有限".into(),
            ));
        }
        if !self.consume_query_budget(budget) {
            return Ok(Vec::new());
        }
        self.overlap_inner(shape, transform, layer_mask)
    }

    #[cfg(feature = "jolt")]
    fn overlap_inner(
        &self,
        shape: &ShapeDesc,
        transform: &PhysicsTransform,
        layer_mask: u64,
    ) -> Result<Vec<OverlapHit>, PhysicsError> {
        let tokens =
            self.sys
                .overlap_shape(&sys_shape(shape), &sys_transform(*transform), layer_mask);
        let mut out: Vec<OverlapHit> = tokens
            .into_iter()
            .filter_map(|t| {
                let body = *self.token_map.get(&t)?;
                let shape = self.bodies.get(body.index(), body.generation())?.shape;
                Some(OverlapHit { body, shape })
            })
            .collect();
        sort_overlap_hits(&mut out);
        Ok(out)
    }

    #[cfg(not(feature = "jolt"))]
    fn overlap_inner(
        &self,
        _shape: &ShapeDesc,
        _transform: &PhysicsTransform,
        _layer_mask: u64,
    ) -> Result<Vec<OverlapHit>, PhysicsError> {
        Err(PhysicsError::BackendNotCompiled(self.desc.backend))
    }

    /// 每帧 drain 接触事件(§4.A5 冻结签名):迭代器受
    /// `SyncBudget::max_contact_events` 截断(§4.A6),截断条数计入饱和计数。
    /// 序列 = step 边界归一化后的 ring 序(确定性面)。
    pub fn drain_contacts<'a>(
        &'a mut self,
        budget: &mut SyncBudget,
    ) -> impl Iterator<Item = ContactEvent> + use<'a> {
        let total = self.ring.len();
        let requested = u32::try_from(total).unwrap_or(u32::MAX);
        let granted = budget.consume_contact_events(requested) as usize;
        self.sat_contact_events = self
            .sat_contact_events
            .saturating_add((total - granted) as u64);
        self.ring.drain_n(granted)
    }

    /// 预算饱和计数快照(单调累计;evidence 埋点源,不进硬门)。
    pub fn budget_saturation(&self) -> BudgetSaturation {
        BudgetSaturation {
            query_casts: self.sat_query_casts.load(Ordering::Relaxed),
            contact_events: self.sat_contact_events,
            body_writes: self.sat_body_writes,
        }
    }

    /// 查询预算消耗(&self 并发面:耗尽可能多线程同时发生,计数走原子)。
    fn consume_query_budget(&self, budget: &mut SyncBudget) -> bool {
        if budget.try_consume_query_cast() {
            true
        } else {
            self.sat_query_casts.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    /// 句柄 → Jolt token(失效句柄二次使用 → `Err(InvalidBody)`,§4.C3 不悬垂)。
    fn body_token(&self, id: BodyId) -> Result<u64, PhysicsError> {
        self.bodies
            .get(id.index(), id.generation())
            .map(|e| e.token)
            .ok_or(PhysicsError::InvalidBody(id))
    }

    /// 占 body+shape 槽位(add 批内单条;返回 (body 部件, shape 部件))。
    #[cfg(feature = "jolt")]
    fn alloc_body_slots(&mut self) -> Result<BodyShapeSlots, PhysicsError> {
        let shape_parts = self.shapes.alloc(())?;
        match self.bodies.alloc(BodyEntry {
            token: 0,
            shape: ShapeId::new(shape_parts.0, shape_parts.1),
        }) {
            Ok(body_parts) => Ok((body_parts, shape_parts)),
            Err(e) => {
                self.shapes.remove(shape_parts.0, shape_parts.1);
                Err(e)
            }
        }
    }

    /// 回滚一批已占槽位(sys 失败路径;句柄从未外泄,无悬挂)。
    #[cfg(feature = "jolt")]
    fn rollback_body_slots(&mut self, slots: &[BodyShapeSlots]) {
        for &((bi, bg), (si, sg)) in slots {
            self.bodies.remove(bi, bg);
            self.shapes.remove(si, sg);
        }
    }

    /// sys 命中 → 公共 `QueryHit`(token→BodyId 映射 + body→shape 回填;
    /// 未知 token 确定性丢弃),按 `(t, BodyId)` 规范序(C-2)。
    #[cfg(feature = "jolt")]
    fn map_sys_hits(&self, hits: Vec<SysHit>) -> Vec<QueryHit> {
        let mut out: Vec<QueryHit> = hits
            .into_iter()
            .filter_map(|h| {
                let body = *self.token_map.get(&h.body)?;
                let shape = self.bodies.get(body.index(), body.generation())?.shape;
                Some(QueryHit {
                    body,
                    t: h.t,
                    position: h.position,
                    normal: h.normal,
                    shape,
                })
            })
            .collect();
        sort_query_hits(&mut out);
        out
    }

    /// sys 原始接触事件 → 公共 `ContactEvent`(token→BodyId;未知 token → `None`)。
    #[cfg(feature = "jolt")]
    fn map_sys_contact(&self, e: SysContactEvent) -> Option<ContactEvent> {
        let a = *self.token_map.get(&e.a)?;
        let b = *self.token_map.get(&e.b)?;
        Some(ContactEvent {
            a,
            b,
            phase: phase_from_sys(e.phase),
            contact_point: e.point,
            normal: e.normal,
            impulse: e.impulse,
        })
    }
}

/// 固定步校验(§4.A1):位级精确比较——同二进制同平台重放逐位一致(§4.0-4(a))
/// 的前提是宿主每拍传入位级相同的 dt;NaN/变步长 → 确定性 `Err`。
pub(crate) fn validate_fixed_dt(expected: f32, got: f32) -> Result<(), PhysicsError> {
    if expected.to_bits() == got.to_bits() {
        Ok(())
    } else {
        Err(PhysicsError::FixedStepMismatch { expected, got })
    }
}

/// sys 层错误上岸映射(§4.C4 v1.2:sys 类型消费收敛于本模块,错误码逐类归并上岸;
/// sys 类型不进 safe 公共 API)。
#[cfg(feature = "jolt")]
fn physics_error_from_sys(e: SysError) -> PhysicsError {
    match e.code {
        SysErrorCode::InvalidDesc => PhysicsError::InvalidDesc(e.message),
        SysErrorCode::InvalidBody => {
            PhysicsError::BackendUnavailable(format!("sys 侧 body token 失效:{}", e.message))
        }
        SysErrorCode::PoolExhausted => PhysicsError::PoolExhausted,
        SysErrorCode::BackendUnavailable => PhysicsError::BackendUnavailable(e.message),
    }
}

#[cfg(feature = "jolt")]
fn transform_from_sys(t: SysTransform) -> PhysicsTransform {
    PhysicsTransform {
        translation: t.translation,
        rotation: t.rotation,
    }
}

#[cfg(feature = "jolt")]
fn sys_transform(t: PhysicsTransform) -> SysTransform {
    SysTransform {
        translation: t.translation,
        rotation: t.rotation,
    }
}

#[cfg(feature = "jolt")]
fn sys_shape(shape: &ShapeDesc) -> SysShapeParams {
    match shape {
        ShapeDesc::Sphere { radius } => SysShapeParams::Sphere { radius: *radius },
        ShapeDesc::Box { half_extents } => SysShapeParams::Box {
            half_extents: *half_extents,
        },
        ShapeDesc::Capsule {
            half_height,
            radius,
        } => SysShapeParams::Capsule {
            half_height: *half_height,
            radius: *radius,
        },
        ShapeDesc::ConvexHull { points } => SysShapeParams::ConvexHull {
            points: points.clone(),
        },
        ShapeDesc::StaticMesh {
            vertices,
            triangles,
        } => SysShapeParams::StaticMesh {
            vertices: vertices.clone(),
            triangles: triangles.clone(),
        },
    }
}

#[cfg(feature = "jolt")]
fn sys_body_desc(d: &BodyDesc) -> SysBodyDesc {
    SysBodyDesc {
        kind: match d.kind {
            BodyKind::Static => SysBodyKind::Static,
            BodyKind::Kinematic => SysBodyKind::Kinematic,
            BodyKind::Dynamic => SysBodyKind::Dynamic,
        },
        shape: sys_shape(&d.shape),
        layer: d.layer,
        mass: d.mass_props.mass,
        friction: d.mass_props.friction,
        restitution: d.mass_props.restitution,
        ccd: d.ccd,
        allow_sleep: d.mass_props.allow_sleep,
        translation: d.transform.translation,
        rotation: d.transform.rotation,
    }
}

#[cfg(feature = "jolt")]
fn phase_from_sys(p: SysContactPhase) -> ContactPhase {
    match p {
        SysContactPhase::Begin => ContactPhase::Begin,
        SysContactPhase::Persist => ContactPhase::Persist,
        SysContactPhase::End => ContactPhase::End,
    }
}

/// f64 秒 → `Duration`(非有限/非正 → ZERO;`from_secs_f64` 对负/NaN 会 panic,
/// P-01 下不允许)。
#[cfg(feature = "jolt")]
fn duration_from_secs(secs: f64) -> std::time::Duration {
    if secs.is_finite() && secs > 0.0 {
        std::time::Duration::from_secs_f64(secs)
    } else {
        std::time::Duration::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BodyKind;

    #[test]
    fn physics_world_is_send_sync() {
        // §4.A4 并发查询前提:PhysicsWorld 必须 Send + Sync(sys 契约同责)。
        fn check<T: Send + Sync>() {}
        check::<PhysicsWorld>();
    }

    fn sphere_desc(kind: BodyKind) -> BodyDesc {
        BodyDesc {
            kind,
            shape: ShapeDesc::Sphere { radius: 0.5 },
            layer: 0,
            mass_props: crate::types::MassProps::default(),
            ccd: false,
            transform: PhysicsTransform::IDENTITY,
        }
    }

    #[test]
    fn fixed_dt_bit_exact() {
        let dt = 1.0 / 60.0;
        assert!(validate_fixed_dt(dt, dt).is_ok());
        assert_eq!(
            validate_fixed_dt(dt, 0.016),
            Err(PhysicsError::FixedStepMismatch {
                expected: dt,
                got: 0.016
            })
        );
        // NaN 变步长 → 确定性 Err。
        assert!(validate_fixed_dt(dt, f32::NAN).is_err());
    }

    #[test]
    fn rapier_always_backend_not_compiled() {
        let desc = WorldDesc {
            backend: BackendKind::Rapier,
            ..Default::default()
        };
        assert_eq!(
            PhysicsWorld::new(desc).unwrap_err(),
            PhysicsError::BackendNotCompiled(BackendKind::Rapier)
        );
    }

    #[cfg(not(feature = "jolt"))]
    #[test]
    fn jolt_backend_not_compiled_without_feature() {
        assert_eq!(
            PhysicsWorld::new(WorldDesc::default()).unwrap_err(),
            PhysicsError::BackendNotCompiled(BackendKind::Jolt)
        );
    }

    #[test]
    fn invalid_world_desc_rejected_before_backend() {
        let base = WorldDesc::default();
        for bad in [
            WorldDesc {
                dt_fixed: 0.0,
                ..base.clone()
            },
            WorldDesc {
                dt_fixed: f32::NAN,
                ..base.clone()
            },
            WorldDesc {
                layer_count: 0,
                ..base.clone()
            },
            WorldDesc {
                max_bodies: 0,
                ..base.clone()
            },
            WorldDesc {
                gravity: [0.0, f32::INFINITY, 0.0],
                ..base.clone()
            },
        ] {
            assert!(matches!(
                PhysicsWorld::new(bad),
                Err(PhysicsError::InvalidDesc(_))
            ));
        }
    }

    #[test]
    fn body_desc_validation_rules() {
        // StaticMesh 仅 Static 体(§4.A2)。
        let mesh = ShapeDesc::StaticMesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[0, 1, 2]],
        };
        let mut d = BodyDesc {
            kind: BodyKind::Dynamic,
            shape: mesh.clone(),
            layer: 0,
            mass_props: crate::types::MassProps::default(),
            ccd: false,
            transform: PhysicsTransform::IDENTITY,
        };
        assert!(d.validate(8).is_err());
        d.kind = BodyKind::Static;
        assert!(d.validate(8).is_ok());
        // layer 越界。
        let mut d = sphere_desc(BodyKind::Static);
        d.layer = 8;
        assert!(d.validate(8).is_err());
        // Dynamic 质量非正。
        let mut d = sphere_desc(BodyKind::Dynamic);
        d.mass_props.mass = 0.0;
        assert!(d.validate(8).is_err());
        // 形状尺寸非法。
        let mut d = sphere_desc(BodyKind::Dynamic);
        d.shape = ShapeDesc::Box {
            half_extents: [1.0, -1.0, 1.0],
        };
        assert!(d.validate(8).is_err());
        // mesh 索引越界。
        let mut d = sphere_desc(BodyKind::Static);
        d.shape = ShapeDesc::StaticMesh {
            vertices: vec![[0.0, 0.0, 0.0]],
            triangles: vec![[0, 1, 2]],
        };
        assert!(d.validate(8).is_err());
    }

    #[test]
    fn query_validation_rules() {
        let ray = QueryRay {
            origin: [0.0; 3],
            dir: [0.0, -1.0, 0.0],
            t_min: 2.0,
            t_max: 1.0,
            layer_mask: 1,
        };
        assert!(ray.validate().is_err());
        let ray = QueryRay {
            t_min: 0.0,
            t_max: f32::NAN,
            ..ray
        };
        assert!(ray.validate().is_err());
    }
}
