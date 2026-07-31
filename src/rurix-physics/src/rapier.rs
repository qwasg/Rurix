//! Rapier 快路径第二后端(G6.4,RFC-0017 §4.D;验收门 G-G6-5)。
//!
//! **快路径 ≠ 性能/稳定性默认**:Rapier 路径价值 = 纯 Rust/无 CMake CI 面与
//! 第二实现交叉验证;生产默认 = Jolt(G6_PLAN §0.1)。不替换默认、不做性能宣称
//! (P-09:实测数字写 evidence)。
//!
//! crate 内唯一 sanctioned `rapier3d` 消费模块(§4.C4 grep 判据经 RFC-0017
//! 修订记录 v1.4 收窄:原生类型名收敛于本文件,公共 API 不透出;crate 维持
//! `#![forbid(unsafe_code)]`,rapier3d 为纯 Rust 依赖,零新 unsafe)。
//!
//! 方法面镜像 `SysWorld` 消费形态(token u64 出/入),world.rs 只做薄分派;
//! safe 层自维护机制(token_map/arena/归一化/ring/预算)两后端共享零分叉。
//!
//! 能力差诚实登记(§4.D2 判据):
//! - **批插逐插**:Rapier 无 Jolt AddBodiesPrepare/Finalize 等价语义,
//!   `add_bodies_batch` 以逐插实现;「批插不锁死主步」判据(§4.A7 同判据:
//!   批插期间主步延迟 ≤ 1 帧)由 behavior 测试双后端同锚兜底。
//! - **事件 Begin/Persist/End 单源合成**:不依赖 `EventHandler`(事件载荷
//!   需求超 CollisionEvent 面),于 step 结束边界对窄相 `contact_pairs()`
//!   与上一拍对集差分——Begin = 本拍有上拍无、End = 上拍有本拍无、
//!   Persist = 交集;归一化排序去重走 world.rs 共享面,
//!   与 Jolt 路径同契约(§4.A5,对拍可比性前提)。接触存在判定 = 几何
//!   接触点 `dist ≤ 0`(穿透/恰好接触;不用 `has_any_active_contact`——
//!   入睡对 solver_contacts 为空,该口径误发 End);**睡眠接触对齐 Jolt
//!   语义**(睡眠即接触约束移除:任一体入睡 → 该对视同移除发 End,
//!   唤醒再接触 = 新 Begin——Jolt 侧塔入睡对发 End 为 G6.2 既有行为,
//!   G6.4 对拍实测对齐,见 `synthesize_events` 注释)。End 事件载荷 = 该对
//!   最后已知接触载荷;body 移除即接触关系随世界消亡,移除时对集条目直接
//!   清除(不发 End——Jolt 侧同名不可得事件经 unmapped 丢弃,两出口语义一致)。
//! - **impulse 取 manifold 求解冲量**(点最大值;Jolt 侧首版恒 0.0 系 JoltC
//!   缺口,RFC-0017 修订记录 v1.2 已登记,对拍门不比对 impulse,§4.D3)。
//! - **CCD → `CcdEnabled`**;睡眠 → `can_sleep`;运动学 → position-based。
//! - **层 → InteractionGroups 32 位**(memberships = 1<<layer,filter = ALL);
//!   `layer_count > 32` → 世界创建确定性 `Err(BackendUnavailable)`。
//! - **单线程标量**:feature 面 dim3/f32/std(parallel/simd/serde/
//!   enhanced-determinism 维持 off),`WorldDesc::job_threads` 为 Jolt 专用,
//!   本后端忽略(文档留痕,不静默宣称并行)。
//! - **cast_ray `t_min` 后过滤**:`intersect_ray` 仅收 max_toi,t ≥ t_min
//!   过滤在映射层;solid = true(起点在形状内 → t=0 命中,游戏查询惯例)。
//! - **cast_shape 全命中 = 排除循环**(最近命中 → 排除已命中 collider 重查
//!   至无新命中;Jolt 侧 CastRay 同法,RFC-0017 修订记录 v1.2 先例);witness/
//!   normal 为局部系,上岸前转世界系(命中体姿态旋转 + 扫掠终点平移)。
//! - **宽相新鲜度**:查询面要求 add 后立即可查(§4.A4)——add/remove 后手动
//!   `broad_phase.update` 只增改不删(stale leaf 经 `colliders.get_unknown_gen`
//!   自然过滤,查询不触达已删体)。**pair 注册闭环**(集成轮缺陷修复留痕,
//!   G6.4 T4 探针定位):BVH 的 `AddPair` 事件仅向本次 `update` 的调用方报告,
//!   而窄相 `register_pairs` 为 rapier crate 内私有(`pub(crate)`),唯一可达
//!   通道 = `pipeline.step` 内部的 detect_collisions 自闭环;手动 update 吞掉
//!   事件的后果 = 初始即相交的 pair(插入时 AABB 已重叠,缝 ≤ prediction
//!   distance 0.002m)永不注册窄相、该对穿透(探针实测:箱底缝 0.001 穿透
//!   地面,缝 0.002 因 proxy 位移在 step 内重报而幸免)。修复 = add 时记录
//!   新 collider 入 `pending_reinsert`,下一拍 `step` 开头先将其从 BVH 删除
//!   (pairs 集静默清空,无事件)再交 `pipeline.step` 内 update 重建——
//!   AddPair 于自闭环内重报,窄相注册完整;查询面不受窗口影响(删/重建
//!   同在 `&mut step` 内,step 外查询恒见完整 BVH)。
//! - `is_active` 语义对齐 Jolt:静态体恒 false,动态/运动学 = !is_sleeping。

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use rapier3d::dynamics::{
    CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet,
    RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
};
use rapier3d::geometry::{
    BroadPhaseBvh, ColliderBuilder, ColliderHandle, ColliderSet, Group, InteractionGroups,
    InteractionTestMode, NarrowPhase, Ray, SharedShape,
};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::parry::query::ShapeCastOptions;
use rapier3d::pipeline::{PhysicsPipeline, QueryFilter};

use crate::error::PhysicsError;
use crate::types::{
    BodyDesc, BodyKind, ContactPhase, PhysicsTransform, QueryRay, QueryShape, ShapeDesc,
};

/// 一拍的接触载荷代表(点/法线/冲量;End 事件回填「最后已知」用)。
type ContactPayload = ([f32; 3], [f32; 3], f32);

/// step 统计(world.rs 统一组装 `StepStats`;step_time 仅供 evidence 不进硬门)。
#[derive(Debug, Default)]
pub(crate) struct RapierStepStats {
    /// 未睡眠动态/运动学体数(静态不计,对齐 Jolt GetNumActiveBodies 语义)。
    pub active_bodies: u32,
    /// 本拍新入睡体数(step 前后睡眠集差分,对齐 sys 轮询差分口径)。
    pub slept_this_step: u32,
    /// 后端层丢弃事件数(恒 0:事件缓冲为本步 Vec,有界 ring 归 world.rs)。
    pub contacts_dropped: u32,
    /// 本拍 wall-clock 秒(仅供 evidence,不进硬门)。
    pub step_time_secs: f64,
}

/// 后端原始接触(token 对 + crate `ContactPhase`;world.rs 经 token_map 上岸)。
#[derive(Debug, Clone, Copy)]
pub(crate) struct RapierContact {
    /// body token 对(未取 min/max 规范序,归一化在 world.rs 共享面)。
    pub a: u64,
    pub b: u64,
    pub phase: ContactPhase,
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub impulse: f32,
}

/// 后端原始命中(cast_ray/cast_shape 共用;world.rs 统一上岸 `QueryHit`)。
#[derive(Debug, Clone, Copy)]
pub(crate) struct RapierHit {
    pub token: u64,
    pub t: f32,
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// Rapier 后端(`forbid(unsafe_code)` crate 内纯 safe 封装;宿主只经 world.rs
/// 薄分派触达,永不见原生句柄——token = RigidBodyHandle raw parts 打包 u64)。
pub(crate) struct RapierBackend {
    gravity: Vector,
    params: IntegrationParameters,
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    /// body token → 其唯一 collider(query 命中反查 body 用;每 body 单 collider)。
    body_colliders: BTreeMap<u64, ColliderHandle>,
    /// add 后待重注册的新 collider(下一拍 step 开头先从 BVH 删除,再由
    /// `pipeline.step` 内 update 重建——AddPair 经 pipeline 自闭环送达窄相,
    /// 见模块头「宽相新鲜度/pair 注册闭环」登记)。
    pending_reinsert: Vec<ColliderHandle>,
    /// 上一拍窄相活动接触 token 对集(Begin/Persist/End 差分源;BTree 确定序)。
    prev_pairs: BTreeSet<(u64, u64)>,
    /// 上一拍各 token 对的代表载荷(End 事件回填「最后已知」)。
    prev_payloads: BTreeMap<(u64, u64), ContactPayload>,
    /// 本拍合成事件缓冲(drain_contacts 取走;step 边界重建)。
    pending: Vec<RapierContact>,
}

impl RapierBackend {
    /// 创建后端。确定性失败路径(P-01):`layer_count > 32`(InteractionGroups
    /// 位宽)→ `Err(BackendUnavailable)`;`job_threads` 忽略(单线程标量,留痕)。
    pub(crate) fn create(
        gravity: [f32; 3],
        layer_count: u32,
        dt_fixed: f32,
    ) -> Result<Self, PhysicsError> {
        if layer_count > 32 {
            return Err(PhysicsError::BackendUnavailable(format!(
                "rapier 层位宽上限 32(InteractionGroups),layer_count={layer_count} 超界"
            )));
        }
        // 固定步钉死:world 层 step(dt) 位级校验兜,后端不再按拍传 dt(§4.A1)。
        let params = IntegrationParameters {
            dt: dt_fixed,
            ..Default::default()
        };
        Ok(RapierBackend {
            gravity: Vector::from(gravity),
            params,
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            body_colliders: BTreeMap::new(),
            pending_reinsert: Vec::new(),
            prev_pairs: BTreeSet::new(),
            prev_payloads: BTreeMap::new(),
            pending: Vec::new(),
        })
    }

    /// 固定步推进(dt 已在创建期钉入 `params.dt`,world 层位级校验兜);
    /// step 结束边界合成接触事件(窄相对集差分,见模块头「能力差诚实登记」)。
    pub(crate) fn step(&mut self) -> RapierStepStats {
        let t0 = Instant::now();
        // pair 注册闭环(模块头「宽相新鲜度」登记):add 时手动 update 的 AddPair
        // 事件已被吞,此处先把新 collider 从 BVH 删除(pairs 集静默清空,不产生
        // DeletePair——leaf 缺失分支直接除名),pipeline.step 内 update 重建 proxy
        // 时 AddPair 经自闭环送达窄相。已删/已换 generation 的 handle 过滤
        // (`contains` 校验 generation),防 arena index 复用后误删新 proxy。
        if !self.pending_reinsert.is_empty() {
            let handles: Vec<ColliderHandle> = std::mem::take(&mut self.pending_reinsert)
                .into_iter()
                .filter(|&h| self.colliders.contains(h))
                .collect();
            let mut sink = Vec::new();
            self.broad_phase.update(
                &self.params,
                &self.colliders,
                &self.bodies,
                &[],
                &handles,
                &mut sink,
            );
        }
        let pre_sleeping = self.sleeping_tokens();
        self.pipeline.step(
            self.gravity,
            &self.params,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &(),
        );
        let post_sleeping = self.sleeping_tokens();
        let slept = post_sleeping.difference(&pre_sleeping).count() as u32;
        let active = self
            .bodies
            .iter()
            .filter(|(_, b)| !b.is_fixed() && !b.is_sleeping())
            .count() as u32;
        self.synthesize_events();
        RapierStepStats {
            active_bodies: active,
            slept_this_step: slept,
            contacts_dropped: 0,
            step_time_secs: t0.elapsed().as_secs_f64(),
        }
    }

    /// 批插体(逐插,§4.D2 诚实登记;all-or-nothing 由 world 层前置校验 +
    /// 池余量兜——rapier arena 动态增长,插入无失败路径,形状构建失败先于
    /// 任何插入发生)。返回 token 序与输入一一对应。
    pub(crate) fn add_bodies_batch(
        &mut self,
        descs: &[BodyDesc],
    ) -> Result<Vec<u64>, PhysicsError> {
        // 形状先行全量构建:任一退化(凸包共面/mesh 非法)→ 整批 Err 零插入。
        let shapes: Vec<SharedShape> = descs
            .iter()
            .map(|d| shared_shape(&d.shape))
            .collect::<Result<_, _>>()?;
        let mut tokens = Vec::with_capacity(descs.len());
        for (d, shape) in descs.iter().zip(shapes) {
            let builder = match d.kind {
                BodyKind::Static => RigidBodyBuilder::fixed(),
                BodyKind::Kinematic => RigidBodyBuilder::kinematic_position_based(),
                BodyKind::Dynamic => RigidBodyBuilder::dynamic(),
            };
            let body = builder
                .pose(pose_of(&d.transform))
                .can_sleep(d.mass_props.allow_sleep)
                .ccd_enabled(d.ccd)
                .build();
            let bh = self.bodies.insert(body);
            // 静态/运动学体 mass 被求解器忽略(无限质量),设置无害(对齐
            // Jolt 侧 MassProps.mass「Static/Kinematic 忽略」语义)。
            let co = ColliderBuilder::new(shape)
                .friction(d.mass_props.friction)
                .restitution(d.mass_props.restitution)
                .mass(d.mass_props.mass)
                .collision_groups(groups_for_layer(d.layer));
            let ch = self.colliders.insert_with_parent(co, bh, &mut self.bodies);
            let token = token_of(bh);
            self.body_colliders.insert(token, ch);
            self.pending_reinsert.push(ch);
            tokens.push(token);
        }
        self.refresh_queries();
        Ok(tokens)
    }

    /// 批移除(token 失效 = 内部不变量违例 → `Err(BackendUnavailable)`;
    /// world 层 arena 前置校验兜,正常路径不可达)。引用已删体的接触对条目
    /// 同步清除(移除即消亡不发 End,见模块头登记)。
    pub(crate) fn remove_bodies_batch(&mut self, tokens: &[u64]) -> Result<(), PhysicsError> {
        for &t in tokens {
            let removed = self.bodies.remove(
                handle_of(t),
                &mut self.islands,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                true,
            );
            if removed.is_none() {
                return Err(PhysicsError::BackendUnavailable(
                    "rapier 侧 body token 失效(内部不变量违例)".into(),
                ));
            }
            if let Some(ch) = self.body_colliders.remove(&t) {
                // 已删 collider 不得留在待重注册队列(step 开头 `tree.remove`
                // 按 index 删除,arena index 复用后会误删新 proxy)。
                self.pending_reinsert.retain(|&h| h != ch);
            }
            self.prev_pairs.retain(|&(a, b)| a != t && b != t);
            self.prev_payloads.retain(|&(a, b), _| a != t && b != t);
        }
        self.refresh_queries();
        Ok(())
    }

    /// body 当前变换(失效 token → `Err(BackendUnavailable)`,内部不变量)。
    pub(crate) fn body_transform(&self, token: u64) -> Result<PhysicsTransform, PhysicsError> {
        let b = self.body(token)?;
        Ok(transform_of(b.translation(), b.rotation()))
    }

    /// 上一拍 active 变换快照(动态/运动学且未睡眠;arena 迭代序确定,
    /// world 层再按 BodyId 升序规范)。
    pub(crate) fn active_transforms(&self) -> Vec<(u64, PhysicsTransform)> {
        self.bodies
            .iter()
            .filter(|(_, b)| (b.is_dynamic() || b.is_kinematic()) && !b.is_sleeping())
            .map(|(h, b)| (token_of(h), transform_of(b.translation(), b.rotation())))
            .collect()
    }

    /// 冲量施加(睡眠体唤醒;仅动态体生效,rapier 内部 kind 门)。
    pub(crate) fn apply_impulse(
        &mut self,
        token: u64,
        impulse: [f32; 3],
    ) -> Result<(), PhysicsError> {
        let b = self
            .bodies
            .get_mut(handle_of(token))
            .ok_or_else(|| PhysicsError::BackendUnavailable("rapier 侧 body token 失效".into()))?;
        b.apply_impulse(Vector::from(impulse), true);
        Ok(())
    }

    /// body 是否激活(静态恒 false,动态/运动学 = !is_sleeping;对齐 Jolt
    /// IsActive 语义,见模块头登记)。
    pub(crate) fn is_active(&self, token: u64) -> Result<bool, PhysicsError> {
        let b = self.body(token)?;
        Ok(!b.is_fixed() && !b.is_sleeping())
    }

    /// 射线 cast 全命中(step 外并发,`&self`;t < t_min 后过滤,solid=true;
    /// 规范序排序在 world.rs 共享面)。
    pub(crate) fn cast_ray(&self, ray: &QueryRay) -> Vec<RapierHit> {
        let qp = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            query_filter(ray.layer_mask),
        );
        let r = Ray::new(Vector::from(ray.origin), Vector::from(ray.dir));
        qp.intersect_ray(r, ray.t_max, true)
            .filter_map(|(ch, _co, hit)| {
                let t = hit.time_of_impact;
                if t < ray.t_min {
                    return None;
                }
                let token = self.token_of_collider(ch)?;
                let p = r.origin + r.dir * t;
                Some(RapierHit {
                    token,
                    t,
                    position: p.to_array(),
                    normal: hit.normal.to_array(),
                })
            })
            .collect()
    }

    /// 形状 cast 全命中(排除循环:最近命中 → 排除重查,§4.D2 登记;
    /// witness/normal 局部系 → 世界系上岸)。
    pub(crate) fn cast_shape(&self, query: &QueryShape) -> Result<Vec<RapierHit>, PhysicsError> {
        let shape = shared_shape(&query.shape)?;
        let pose = pose_of(&query.start);
        let vel = Vector::from(query.dir);
        let options = ShapeCastOptions {
            max_time_of_impact: query.t_max,
            target_distance: 0.0,
            // 起点重叠也报命中(compute_impact_geometry 保 witness/normal 有效);
            // Jolt 侧 CastShape 对初始重叠同样返回命中,语义对齐。
            stop_at_penetration: false,
            compute_impact_geometry_on_penetration: true,
        };
        let mut out = Vec::new();
        let mut excluded: Vec<ColliderHandle> = Vec::new();
        loop {
            let mut filter = query_filter(query.layer_mask);
            for &h in &excluded {
                filter = filter.exclude_collider(h);
            }
            let qp = self.broad_phase.as_query_pipeline(
                self.narrow_phase.query_dispatcher(),
                &self.bodies,
                &self.colliders,
                filter,
            );
            let Some((ch, hit)) = qp.cast_shape(&pose, vel, &*shape, options) else {
                break;
            };
            excluded.push(ch);
            let Some(token) = self.token_of_collider(ch) else {
                continue;
            };
            // witness1 = 扫掠体局部系最近点;世界命中点 = 扫掠终点姿态 * witness1。
            let t = hit.time_of_impact;
            let end_translation = pose.translation + vel * t;
            let world_point = end_translation + pose.rotation * hit.witness1;
            // normal2 = 被命中体局部系外法线 → 世界系(被命中体当前姿态旋转)。
            let world_normal = self
                .colliders
                .get(ch)
                .map(|co| co.position().rotation * hit.normal2)
                .unwrap_or(hit.normal2);
            out.push(RapierHit {
                token,
                t,
                position: world_point.to_array(),
                normal: world_normal.to_array(),
            });
            // 保险丝:每轮排除一个,候选严格递减,界 = 世界 collider 总数。
            if excluded.len() > self.colliders.len() {
                break;
            }
        }
        Ok(out)
    }

    /// 形状 overlap(全命中;token 序列未排序,规范序在 world.rs 共享面)。
    pub(crate) fn overlap_shape(
        &self,
        shape: &ShapeDesc,
        transform: &PhysicsTransform,
        layer_mask: u64,
    ) -> Result<Vec<u64>, PhysicsError> {
        let shape = shared_shape(shape)?;
        let pose = pose_of(transform);
        let qp = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            query_filter(layer_mask),
        );
        Ok(qp
            .intersect_shape(pose, &*shape)
            .filter_map(|(ch, _co)| self.token_of_collider(ch))
            .collect())
    }

    /// 取走本拍合成事件(后端层恒零丢弃;有界 ring 归 world.rs)。
    pub(crate) fn drain_contacts(&mut self) -> (Vec<RapierContact>, u32) {
        (std::mem::take(&mut self.pending), 0)
    }

    // ———————————————————— 内部机制 ————————————————————

    fn body(&self, token: u64) -> Result<&rapier3d::dynamics::RigidBody, PhysicsError> {
        self.bodies
            .get(handle_of(token))
            .ok_or_else(|| PhysicsError::BackendUnavailable("rapier 侧 body token 失效".into()))
    }

    fn token_of_collider(&self, ch: ColliderHandle) -> Option<u64> {
        Some(token_of(self.colliders.get(ch)?.parent()?))
    }

    fn sleeping_tokens(&self) -> BTreeSet<u64> {
        self.bodies
            .iter()
            .filter(|(_, b)| b.is_sleeping())
            .map(|(h, _)| token_of(h))
            .collect()
    }

    /// 接触事件合成(step 结束边界;单源差分,模块头「能力差诚实登记」)。
    /// 窄相 `contact_pairs` 仅含本拍 BVH 注册对;接触存在判定 = 几何接触点
    /// (`dist ≤ 0`:穿透/恰好接触)——不看 `has_any_active_contact`(其口径 =
    /// solver_contacts 非空,入睡对被求解器收编后为空;dist ≤ 0 口径下入睡对
    /// 虽仍「接触」,但 **Jolt 语义 = 睡眠即接触约束移除(OnContactRemoved →
    /// End)**,本后端向其对齐:任一体入睡 → 该对视同移除(本拍不计),唤醒
    /// 再接触 = 新 Begin(G6.4 对拍实测留痕:Jolt 侧塔入睡后 4 对 End,
    /// Rapier 侧 manifold 保留无 End,End 集合重叠率 0.2 / 相位等价类破裂;
    /// 对齐后两端塔对 RLE 同为 [B,P,E])。speculative 点(0 < dist ≤
    /// prediction)不算接触,Begin 对齐 Jolt OnContactAdded 真接触语义。
    /// 代表载荷 = 首个非空 manifold:接触点 = collider1 姿态 * local_p1
    /// (世界系),法线 = manifold 共享世界法线,冲量 = 全 manifold 点求解
    /// 冲量最大值。
    fn synthesize_events(&mut self) {
        let mut current: BTreeSet<(u64, u64)> = BTreeSet::new();
        let mut payloads: BTreeMap<(u64, u64), ContactPayload> = BTreeMap::new();
        for pair in self.narrow_phase.contact_pairs() {
            let touching = pair
                .manifolds
                .iter()
                .any(|m| m.points.iter().any(|pt| pt.dist <= 0.0));
            if !touching {
                continue;
            }
            let (Some(t1), Some(t2)) = (
                self.token_of_collider(pair.collider1),
                self.token_of_collider(pair.collider2),
            ) else {
                continue;
            };
            // 睡眠接触对齐 Jolt(见函数头登记):任一体入睡 → 视同移除。
            let asleep = self
                .bodies
                .get(handle_of(t1))
                .map(|b| b.is_sleeping())
                .unwrap_or(false)
                || self
                    .bodies
                    .get(handle_of(t2))
                    .map(|b| b.is_sleeping())
                    .unwrap_or(false);
            if asleep {
                continue;
            }
            let key = (t1.min(t2), t1.max(t2));
            let mut payload: ContactPayload = ([0.0; 3], [0.0; 3], 0.0);
            if let Some(co1) = self.colliders.get(pair.collider1)
                && let Some(m) = pair.manifolds.iter().find(|m| !m.points.is_empty())
            {
                let p = m.points[0];
                let world = co1.position().translation + co1.position().rotation * p.local_p1;
                let impulse = pair
                    .manifolds
                    .iter()
                    .flat_map(|mm| mm.points.iter())
                    .map(|pt| pt.data.impulse)
                    .fold(0.0f32, f32::max);
                payload = (world.to_array(), m.data.normal.to_array(), impulse);
            }
            current.insert(key);
            payloads.insert(key, payload);
        }
        let mut events = Vec::with_capacity(current.len() + self.prev_pairs.len());
        for &key in &current {
            let phase = if self.prev_pairs.contains(&key) {
                ContactPhase::Persist
            } else {
                ContactPhase::Begin
            };
            let (point, normal, impulse) = payloads[&key];
            events.push(RapierContact {
                a: key.0,
                b: key.1,
                phase,
                point,
                normal,
                impulse,
            });
        }
        for &key in &self.prev_pairs {
            if !current.contains(&key) {
                let (point, normal, impulse) = self
                    .prev_payloads
                    .get(&key)
                    .copied()
                    .unwrap_or(([0.0; 3], [0.0; 3], 0.0));
                events.push(RapierContact {
                    a: key.0,
                    b: key.1,
                    phase: ContactPhase::End,
                    point,
                    normal,
                    impulse,
                });
            }
        }
        self.prev_pairs = current;
        self.prev_payloads = payloads;
        self.pending = events;
    }

    /// add/remove 后手动宽相刷新(查询面新鲜度:§4.A4 要求 add 后立即可查;
    /// 只增改不删,stale leaf 经 collider 查找自然过滤;`update` 内部按
    /// `needs_broad_phase_update` 跳过未变项,已插 leaf 走幂等更新)。
    /// 副作用登记:本次 update 的 AddPair 事件随 sink 丢弃(窄相注册为 rapier
    /// crate 内私有通道),新 collider 已入 `pending_reinsert`,pair 注册闭环
    /// 由下一拍 `step` 开头的删-插重报完成(见模块头「宽相新鲜度」)。
    fn refresh_queries(&mut self) {
        let handles: Vec<ColliderHandle> = self.colliders.iter().map(|(h, _)| h).collect();
        let mut sink = Vec::new();
        self.broad_phase.update(
            &self.params,
            &self.colliders,
            &self.bodies,
            &handles,
            &[],
            &mut sink,
        );
    }
}

// ———————————————————— 类型映射(模块内私有) ————————————————————

/// `RigidBodyHandle` raw parts → u64 token(index 高 32b | generation 低 32b;
/// world.rs token 语义与 Jolt 侧一致,FFI 边界只过 u64,§4.C3)。
fn token_of(handle: RigidBodyHandle) -> u64 {
    let (index, generation) = handle.into_raw_parts();
    ((index as u64) << 32) | generation as u64
}

/// u64 token → `RigidBodyHandle`(逆映射;world 层 arena 担保 token 有效)。
fn handle_of(token: u64) -> RigidBodyHandle {
    RigidBodyHandle::from_raw_parts((token >> 32) as u32, token as u32)
}

/// `PhysicsTransform` → `Pose`(xyzw 四元数直映 glam Quat;调用方负责单位化,§4.A2)。
fn pose_of(t: &PhysicsTransform) -> Pose {
    // Pose3 含私有 padding 字段,经构造器 + pub rotation 字段组装。
    let mut p = Pose::from_translation(Vector::from(t.translation));
    p.rotation = Rotation::from_xyzw(t.rotation[0], t.rotation[1], t.rotation[2], t.rotation[3]);
    p
}

/// rapier 平移/旋转 → `PhysicsTransform`(glam Quat 即 xyzw 布局,零重排)。
fn transform_of(translation: Vector, rotation: &Rotation) -> PhysicsTransform {
    PhysicsTransform {
        translation: translation.to_array(),
        rotation: [rotation.x, rotation.y, rotation.z, rotation.w],
    }
}

/// `ShapeDesc` → `SharedShape`。确定性失败(P-01):凸包退化(共面/重合
/// 点集 → `convex_hull` 返 None)、mesh 构建非法 → `Err(InvalidDesc)`。
fn shared_shape(desc: &ShapeDesc) -> Result<SharedShape, PhysicsError> {
    match desc {
        ShapeDesc::Sphere { radius } => Ok(SharedShape::ball(*radius)),
        ShapeDesc::Box { half_extents } => Ok(SharedShape::cuboid(
            half_extents[0],
            half_extents[1],
            half_extents[2],
        )),
        ShapeDesc::Capsule {
            half_height,
            radius,
        } => Ok(SharedShape::capsule_y(*half_height, *radius)),
        ShapeDesc::ConvexHull { points } => {
            let pts: Vec<Vector> = points.iter().map(|p| Vector::from(*p)).collect();
            SharedShape::convex_hull(&pts).ok_or_else(|| {
                PhysicsError::InvalidDesc("ConvexHull 退化(共面/重合点集),无法构建".into())
            })
        }
        ShapeDesc::StaticMesh {
            vertices,
            triangles,
        } => {
            let vs: Vec<Vector> = vertices.iter().map(|p| Vector::from(*p)).collect();
            SharedShape::trimesh(vs, triangles.clone())
                .map_err(|e| PhysicsError::InvalidDesc(format!("StaticMesh 构建失败:{e:?}")))
        }
    }
}

/// 体层 → InteractionGroups(memberships = 1<<layer,filter = ALL;
/// 层 ≤ 32 已由 create 钉死)。
fn groups_for_layer(layer: u32) -> InteractionGroups {
    InteractionGroups::new(
        Group::from_bits_truncate(1u32 << layer),
        Group::ALL,
        InteractionTestMode::And,
    )
}

/// 查询层掩码 → QueryFilter(memberships/filter 同 mask:命中 iff 被命体
/// 层位 ∈ mask——被命体 filter = ALL 使反向条件恒真,语义对齐 Jolt 层掩码)。
fn query_filter(layer_mask: u64) -> QueryFilter<'static> {
    let bits = Group::from_bits_truncate(layer_mask as u32);
    QueryFilter::default().groups(InteractionGroups::new(bits, bits, InteractionTestMode::And))
}
