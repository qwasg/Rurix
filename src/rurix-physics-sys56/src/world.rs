//! `SysWorld` 内部实现(RFC-0017 §4.C):JoltC 句柄所有权、相位门、接触回调、查询面。
//!
//! 所有权单向:world 拥有 body/shape(§4.C3);shape 引用计数纪律 = Create 得 1 引用,
//! body 销毁后 `JPC56_Shape_Release`(Jolt `Body` 不持有 shape 引用,实测
//! `Body::SetShapeInternal`,登记 VENDOR56.md §4)。销毁序见 `Inner::drop`。
//!
//! 相位门(§4.A4/§4.C3):变更路径 = `&mut self`,只读查询 = `&self`;
//! Jolt job 线程只活在 `JPC56_PhysicsSystem_Update` 调用内;contact 回调在该窗口内
//! 多线程触发,事件经 `Mutex` 收集(归一化在 safe 层,§4.A5),回调内不 panic。

use std::collections::{HashMap, VecDeque};
use std::ffi::{CStr, c_void};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, Once};
use std::time::Instant;

use crate::ffi::*;
use crate::{
    SysBodyDesc, SysBodyKind, SysContactEvent, SysContactPhase, SysError, SysErrorCode, SysHit,
    SysRay, SysShapeParams, SysTransform, SysWorldDesc,
};

/// Jolt object layer 上限:16 位位宽,0xFFFF = `cObjectLayerInvalid` 保留(VENDOR56.md §2)。
const MAX_OBJECT_LAYERS: u32 = 65535;
/// `PhysicsSystem::Init` 的 body pair / contact constraint 池上限(Jolt HelloWorld 画像;
/// 溢出 → `Update` 返回错误位 → 确定性 `Err(PoolExhausted)`,P-01)。
const MAX_BODY_PAIRS: u32 = 65536;
const MAX_CONTACT_CONSTRAINTS: u32 = 10240;
/// TempAllocator 预算(Jolt HelloWorld 画像)。
const TEMP_ALLOCATOR_BYTES: u32 = 10 * 1024 * 1024;
/// cast_ray 排除循环安全阀(防异常输入下死循环;正常远达不到)。
const MAX_RAY_HITS: usize = 4096;

fn err(code: SysErrorCode, message: impl Into<String>) -> SysError {
    SysError {
        code,
        message: message.into(),
    }
}

/// 进程级 Jolt 初始化(一次注册、进程常驻,镜像 U1 loader 不卸载纪律;U33 登记)。
fn ensure_jolt_initialized() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // SAFETY: 进程级一次性注册(Once 保证);Jolt 约定三调用按此序、进程生命期内
        // 不配对 UnregisterTypes/FactoryDelete(常驻语义,避免多 world 反复注册竞态)。
        unsafe {
            JPC56_RegisterDefaultAllocator();
            JPC56_FactoryInit();
            JPC56_RegisterTypes();
        }
    });
}

/// 接触事件收集器(contact listener 的 user_data;Box 固定堆地址,生命周期 ≥ 注册窗口)。
pub(crate) struct ContactSink {
    queue: Mutex<ContactQueue>,
    /// 本步已收事件数(仅 `step` 相位统计;step 外只增不读)。
    pushed_this_step: AtomicU32,
}

struct ContactQueue {
    deque: VecDeque<SysContactEvent>,
    cap: usize,
    dropped_since_drain: u32,
}

impl ContactSink {
    fn new(capacity: u32) -> Self {
        Self {
            queue: Mutex::new(ContactQueue {
                deque: VecDeque::new(),
                cap: capacity as usize,
                dropped_since_drain: 0,
            }),
            pushed_this_step: AtomicU32::new(0),
        }
    }

    fn push(&self, ev: SysContactEvent) {
        self.pushed_this_step.fetch_add(1, Ordering::Relaxed);
        let mut q = lock_unpoison(&self.queue);
        if q.cap == 0 {
            q.dropped_since_drain = q.dropped_since_drain.saturating_add(1);
            return;
        }
        if q.deque.len() == q.cap {
            // ring 满 → 确定性丢最旧 + 计数(§4.A5 溢出语义,不 panic)
            q.deque.pop_front();
            q.dropped_since_drain = q.dropped_since_drain.saturating_add(1);
        }
        q.deque.push_back(ev);
    }

    fn dropped(&self) -> u32 {
        lock_unpoison(&self.queue).dropped_since_drain
    }

    fn drain(&self) -> (Vec<SysContactEvent>, u32) {
        let mut q = lock_unpoison(&self.queue);
        let dropped = std::mem::take(&mut q.dropped_since_drain);
        (q.deque.drain(..).collect(), dropped)
    }
}

/// 回调与 step 均不 panic → 锁不中毒;`into_inner` 兜底保证 FFI 边界永不 panic。
fn lock_unpoison(m: &Mutex<ContactQueue>) -> MutexGuard<'_, ContactQueue> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// 回调(user_data 纪律:listener 指向 Inner 持有的 Box<ContactSink>;查询过滤器/
// 收集器指向调用帧栈上状态,生命周期严格短于注册窗口 — 沿 U27 栈纪律)
// ---------------------------------------------------------------------------

unsafe extern "C" fn bp_get_num_layers(_self: *const c_void) -> u32 {
    // 首版单 broadphase layer(VENDOR56.md §3 收窄登记)
    1
}

unsafe extern "C" fn bp_get_layer(
    _self: *const c_void,
    _layer: JpcObjectLayer,
) -> JpcBroadPhaseLayer {
    0
}

unsafe extern "C" fn filter_true_olp(
    _self: *const c_void,
    _l1: JpcObjectLayer,
    _l2: JpcObjectLayer,
) -> bool {
    true
}

unsafe extern "C" fn filter_true_ovb(
    _self: *const c_void,
    _l1: JpcObjectLayer,
    _l2: JpcBroadPhaseLayer,
) -> bool {
    true
}

unsafe extern "C" fn ol_should_collide(self_: *const c_void, layer: JpcObjectLayer) -> bool {
    // SAFETY: self_ 指向调用帧栈上 u64 layer_mask(单次查询调用内有效,栈纪律);
    // 仅读 8 字节 POD,不 panic。
    let mask = unsafe { *self_.cast::<u64>() };
    layer < 64 && (mask >> layer) & 1 == 1
}

unsafe extern "C" fn ray_body_should_collide(self_: *const c_void, id: JpcBodyId) -> bool {
    // SAFETY: self_ 指向调用帧栈上 Vec<u32> 已命中排除集(单次 cast_ray 内有效)。
    !unsafe { &*self_.cast::<Vec<u32>>() }.contains(&id)
}

unsafe extern "C" fn ray_body_should_collide_locked(
    self_: *const c_void,
    body: *const JpcBody,
) -> bool {
    // SAFETY: body 由 Jolt 在查询内向 narrow phase 保证有效(锁定只读访问);
    // 排除集栈纪律同 ray_body_should_collide。
    let id = unsafe { JPC56_Body_GetID(body) };
    // SAFETY: 同 ray_body_should_collide(栈上 Vec<u32>,单次 cast_ray 内有效)。
    !unsafe { &*self_.cast::<Vec<u32>>() }.contains(&id)
}

unsafe extern "C" fn on_contact_added(
    self_: *mut c_void,
    body1: *const JpcBody,
    body2: *const JpcBody,
    manifold: *const JpcContactManifold,
    _settings: *mut JpcContactSettings,
) {
    // SAFETY: JoltC 在 Update 相位内以有效 body/manifold 指针回调;self_ 指向
    // Inner 持有的 Box<ContactSink>(生命周期 ≥ 注册窗口,Inner::drop 先摘除监听器)。
    // 仅读 POD + 锁内 push,不 panic(FFI 边界不回抛)。
    let (sink, a, b, m) = unsafe {
        (
            &*self_.cast::<ContactSink>(),
            JPC56_Body_GetID(body1),
            JPC56_Body_GetID(body2),
            &*manifold,
        )
    };
    sink.push(manifold_event(a, b, SysContactPhase::Begin, m));
}

unsafe extern "C" fn on_contact_persisted(
    self_: *mut c_void,
    body1: *const JpcBody,
    body2: *const JpcBody,
    manifold: *const JpcContactManifold,
    _settings: *mut JpcContactSettings,
) {
    // SAFETY: 同 on_contact_added。
    let (sink, a, b, m) = unsafe {
        (
            &*self_.cast::<ContactSink>(),
            JPC56_Body_GetID(body1),
            JPC56_Body_GetID(body2),
            &*manifold,
        )
    };
    sink.push(manifold_event(a, b, SysContactPhase::Persist, m));
}

unsafe extern "C" fn on_contact_removed(self_: *mut c_void, pair: *const JpcSubShapeIdPair) {
    // SAFETY: pair 为 JoltC 桥内有效栈对象;其余同 on_contact_added。
    let (sink, p) = unsafe { (&*self_.cast::<ContactSink>(), &*pair) };
    sink.push(SysContactEvent {
        a: p.body1_id as u64,
        b: p.body2_id as u64,
        phase: SysContactPhase::End,
        point: [0.0; 3],
        normal: [0.0; 3],
        impulse: 0.0,
    });
}

/// manifold → 事件:点 = BaseOffset + 首个 RelativeContactPointsOn1(无点则 BaseOffset);
/// 法线 = WorldSpaceNormal(Jolt 约定:把 body2 推出碰撞的最短方向);impulse 首版恒 0
/// (JoltC 回调不含求解后冲量,VENDOR56.md §3 缺口收窄登记)。
fn manifold_event(
    a: JpcBodyId,
    b: JpcBodyId,
    phase: SysContactPhase,
    m: &JpcContactManifold,
) -> SysContactEvent {
    let point = if m.relative_contact_points_on1.length > 0 {
        let p = m.relative_contact_points_on1.points[0];
        [
            m.base_offset.x + p.x,
            m.base_offset.y + p.y,
            m.base_offset.z + p.z,
        ]
    } else {
        [m.base_offset.x, m.base_offset.y, m.base_offset.z]
    };
    SysContactEvent {
        a: a as u64,
        b: b as u64,
        phase,
        point,
        normal: [
            m.world_space_normal.x,
            m.world_space_normal.y,
            m.world_space_normal.z,
        ],
        impulse: 0.0,
    }
}

struct CastCollectCtx {
    hits: Vec<SysHit>,
    t_max: f32,
}

unsafe extern "C" fn cast_shape_add_hit(
    self_: *mut c_void,
    _base: *mut JpcCastShapeCollector,
    result: *const JpcShapeCastResult,
) {
    // SAFETY: self_ 指向调用帧栈上 CastCollectCtx(collector 生命周期严格短于单次
    // CastShape 调用,且 CastShape 在调用线程同步执行);result 调用内有效;不 panic。
    let (ctx, r) = unsafe { (&mut *self_.cast::<CastCollectCtx>(), &*result) };
    let axis = r.base.penetration_axis;
    ctx.hits.push(SysHit {
        body: r.base.body_id2 as u64,
        t: r.fraction * ctx.t_max,
        position: [
            r.base.contact_point_on2.x,
            r.base.contact_point_on2.y,
            r.base.contact_point_on2.z,
        ],
        // 命中面法线 ≈ -penetration_axis 归一化(axis 指向把 shape2 推出方向)
        normal: neg_normalized([axis.x, axis.y, axis.z]),
    });
}

struct OverlapCollectCtx {
    ids: Vec<u64>,
}

unsafe extern "C" fn collide_shape_add_hit(
    self_: *mut c_void,
    _base: *mut JpcCollideShapeCollector,
    result: *const JpcCollideShapeResult,
) {
    // SAFETY: 同 cast_shape_add_hit(单次 CollideShape 调用内栈纪律)。
    let (ctx, r) = unsafe { (&mut *self_.cast::<OverlapCollectCtx>(), &*result) };
    let id = r.body_id2 as u64;
    if !ctx.ids.contains(&id) {
        ctx.ids.push(id);
    }
}

// ---------------------------------------------------------------------------
// 数学助手(quat 约定 xyzw;矩阵列主序,与 Jolt Mat44::sRotationTranslation 同构)
// ---------------------------------------------------------------------------

fn quat_normalized(q: [f32; 4]) -> [f32; 4] {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if !len.is_finite() || len < 1e-12 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
}

fn quat_rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let tx = 2.0 * (y * v[2] - z * v[1]);
    let ty = 2.0 * (z * v[0] - x * v[2]);
    let tz = 2.0 * (x * v[1] - y * v[0]);
    [
        v[0] + w * tx + (y * tz - z * ty),
        v[1] + w * ty + (z * tx - x * tz),
        v[2] + w * tz + (x * ty - y * tx),
    ]
}

fn mat44_from_rot_trans(q: [f32; 4], t: [f32; 3]) -> JpcMat44 {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let c0 = [
        1.0 - 2.0 * (y * y + z * z),
        2.0 * (x * y + w * z),
        2.0 * (x * z - w * y),
    ];
    let c1 = [
        2.0 * (x * y - w * z),
        1.0 - 2.0 * (x * x + z * z),
        2.0 * (y * z + w * x),
    ];
    let c2 = [
        2.0 * (x * z + w * y),
        2.0 * (y * z - w * x),
        1.0 - 2.0 * (x * x + y * y),
    ];
    JpcMat44 {
        col: [
            JpcVec4 {
                x: c0[0],
                y: c0[1],
                z: c0[2],
                w: 0.0,
            },
            JpcVec4 {
                x: c1[0],
                y: c1[1],
                z: c1[2],
                w: 0.0,
            },
            JpcVec4 {
                x: c2[0],
                y: c2[1],
                z: c2[2],
                w: 0.0,
            },
        ],
        col3: JpcVec3::new(t[0], t[1], t[2]),
    }
}

fn neg_normalized(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if !len.is_finite() || len < 1e-12 {
        return [0.0; 3];
    }
    [-v[0] / len, -v[1] / len, -v[2] / len]
}

// ---------------------------------------------------------------------------
// Inner:JoltC 句柄所有权与全部世界操作
// ---------------------------------------------------------------------------

struct BodyRec {
    shape: *mut JpcShape,
    kind: SysBodyKind,
    kinematic_target: Option<SysTransform>,
}

struct ConstraintRec {
    /// `JPC56_HingeConstraint*` 作 `JPC56_Constraint*` 使用(JoltC 继承布局)。
    ptr: *mut JpcConstraint,
    body_a: JpcBodyId,
    body_b: JpcBodyId,
    motor_state: u32,
}

pub(crate) struct Inner {
    ps: *mut JpcPhysicsSystem,
    bi: *mut JpcBodyInterface,
    npq: *const JpcNarrowPhaseQuery,
    bli: *const JpcBodyLockInterface,
    temp: *mut JpcTempAllocatorImpl,
    job: *mut JpcJobSystemThreadPool,
    bp: *mut JpcBroadPhaseLayerInterface,
    ovb: *mut JpcObjectVsBroadPhaseLayerFilter,
    olp: *mut JpcObjectLayerPairFilter,
    listener: *mut JpcContactListener,
    cb: Box<ContactSink>,
    bodies: HashMap<JpcBodyId, BodyRec>,
    /// constraint token(u64 自增键,从 1 起)→ 句柄。
    constraints: HashMap<u64, ConstraintRec>,
    next_constraint_token: u64,
    layer_count: u32,
    max_bodies: u32,
}

/// create 半成品守卫:任一步失败 → 逆序销毁已建句柄(fail-closed,无泄漏)。
struct CreateGuard {
    temp: *mut JpcTempAllocatorImpl,
    job: *mut JpcJobSystemThreadPool,
    bp: *mut JpcBroadPhaseLayerInterface,
    ovb: *mut JpcObjectVsBroadPhaseLayerFilter,
    olp: *mut JpcObjectLayerPairFilter,
    ps: *mut JpcPhysicsSystem,
    listener: *mut JpcContactListener,
}

impl CreateGuard {
    fn new() -> Self {
        Self {
            temp: std::ptr::null_mut(),
            job: std::ptr::null_mut(),
            bp: std::ptr::null_mut(),
            ovb: std::ptr::null_mut(),
            olp: std::ptr::null_mut(),
            ps: std::ptr::null_mut(),
            listener: std::ptr::null_mut(),
        }
    }
}

impl Drop for CreateGuard {
    fn drop(&mut self) {
        // SAFETY: 各句柄若非 null 均为本守卫创建成功且未销毁过;逆创建序销毁。
        unsafe {
            if !self.listener.is_null() {
                JPC56_ContactListener_delete(self.listener);
            }
            if !self.ps.is_null() {
                JPC56_PhysicsSystem_delete(self.ps);
            }
            if !self.olp.is_null() {
                JPC56_ObjectLayerPairFilter_delete(self.olp);
            }
            if !self.ovb.is_null() {
                JPC56_ObjectVsBroadPhaseLayerFilter_delete(self.ovb);
            }
            if !self.bp.is_null() {
                JPC56_BroadPhaseLayerInterface_delete(self.bp);
            }
            if !self.job.is_null() {
                JPC56_JobSystemThreadPool_delete(self.job);
            }
            if !self.temp.is_null() {
                JPC56_TempAllocatorImpl_delete(self.temp);
            }
        }
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        // SAFETY: 全部 JoltC 句柄为本 Inner 独占拥有、未提前释放;此时无 Update 在飞
        // (Rust 借用规则:drop 需独占 &mut)。销毁序 = 摘除监听器 → 摘除/释放约束 →
        // PhysicsSystem(连带 body)→ 逐 body 释放 shape → 过滤器/层接口 → job → temp。
        unsafe {
            JPC56_PhysicsSystem_SetContactListener(self.ps, std::ptr::null_mut());
            for rec in self.constraints.values() {
                // RemoveConstraint 释放 manager 的 Ref;再 Release 本 registry 的 AddRef(U52)。
                JPC56_PhysicsSystem_RemoveConstraint(self.ps, rec.ptr);
                JPC56_Constraint_Release(rec.ptr);
            }
            self.constraints.clear();
            JPC56_PhysicsSystem_delete(self.ps);
            for rec in self.bodies.values() {
                JPC56_Shape_Release(rec.shape);
            }
            JPC56_ContactListener_delete(self.listener);
            JPC56_ObjectLayerPairFilter_delete(self.olp);
            JPC56_ObjectVsBroadPhaseLayerFilter_delete(self.ovb);
            JPC56_BroadPhaseLayerInterface_delete(self.bp);
            JPC56_JobSystemThreadPool_delete(self.job);
            JPC56_TempAllocatorImpl_delete(self.temp);
        }
    }
}

impl Inner {
    pub(crate) fn create(desc: &SysWorldDesc) -> Result<Inner, SysError> {
        if desc.layer_count == 0 || desc.layer_count > MAX_OBJECT_LAYERS {
            return Err(err(
                SysErrorCode::InvalidDesc,
                format!(
                    "layer_count {} 超出 [1, {MAX_OBJECT_LAYERS}]",
                    desc.layer_count
                ),
            ));
        }
        if desc.max_bodies == 0 {
            return Err(err(SysErrorCode::InvalidDesc, "max_bodies 必须 ≥ 1"));
        }
        if !desc.gravity.iter().all(|g| g.is_finite()) {
            return Err(err(SysErrorCode::InvalidDesc, "gravity 必须有限"));
        }

        ensure_jolt_initialized();

        let mut g = CreateGuard::new();
        // SAFETY: 以下为 JoltC 构造调用链,参数为字面量/合法回调表;每步 null 校验,
        // 失败经 CreateGuard 逆序清理(fail-closed)。回调 user_data 纪律:层接口与过滤
        // 器传 null(回调不读 self);contact listener 的 self 指向 Box<ContactSink>
        // 堆地址(稳定,生命周期 ≥ 注册窗口)。
        unsafe {
            g.temp = JPC56_TempAllocatorImpl_new(TEMP_ALLOCATOR_BYTES);
            if g.temp.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "JPC56_TempAllocatorImpl_new 返回 null",
                ));
            }
            let threads = if desc.job_threads == 0 {
                -1 // Jolt 默认 = 硬件并行度(§4.A3:job_threads 或可用并行度)
            } else {
                desc.job_threads as i32
            };
            g.job = JPC56_JobSystemThreadPool_new3(
                JPC56_MAX_PHYSICS_JOBS,
                JPC56_MAX_PHYSICS_BARRIERS,
                threads,
            );
            if g.job.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "JPC56_JobSystemThreadPool_new3 返回 null",
                ));
            }
            g.bp = JPC56_BroadPhaseLayerInterface_new(
                std::ptr::null(),
                JpcBroadPhaseLayerInterfaceFns {
                    get_num_broad_phase_layers: Some(bp_get_num_layers),
                    get_broad_phase_layer: Some(bp_get_layer),
                },
            );
            if g.bp.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "JPC56_BroadPhaseLayerInterface_new 返回 null",
                ));
            }
            g.ovb = JPC56_ObjectVsBroadPhaseLayerFilter_new(
                std::ptr::null(),
                JpcObjectVsBroadPhaseLayerFilterFns {
                    should_collide: Some(filter_true_ovb),
                },
            );
            if g.ovb.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "JPC56_ObjectVsBroadPhaseLayerFilter_new 返回 null",
                ));
            }
            g.olp = JPC56_ObjectLayerPairFilter_new(
                std::ptr::null(),
                JpcObjectLayerPairFilterFns {
                    should_collide: Some(filter_true_olp),
                },
            );
            if g.olp.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "JPC56_ObjectLayerPairFilter_new 返回 null",
                ));
            }
            g.ps = JPC56_PhysicsSystem_new();
            if g.ps.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "JPC56_PhysicsSystem_new 返回 null",
                ));
            }
            JPC56_PhysicsSystem_Init(
                g.ps,
                desc.max_bodies,
                0, // num_body_mutexes:0 = Jolt 默认
                MAX_BODY_PAIRS,
                MAX_CONTACT_CONSTRAINTS,
                g.bp,
                g.ovb,
                g.olp,
            );
            JPC56_PhysicsSystem_SetGravity(
                g.ps,
                JpcVec3::new(desc.gravity[0], desc.gravity[1], desc.gravity[2]),
            );

            let cb = Box::new(ContactSink::new(desc.contact_capacity));
            g.listener = JPC56_ContactListener_new(
                (&*cb as *const ContactSink).cast_mut().cast(),
                JpcContactListenerFns {
                    on_contact_validate: None, // null = Jolt 默认(全接受)
                    on_contact_added: Some(on_contact_added),
                    on_contact_persisted: Some(on_contact_persisted),
                    on_contact_removed: Some(on_contact_removed),
                },
            );
            if g.listener.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "JPC56_ContactListener_new 返回 null",
                ));
            }
            JPC56_PhysicsSystem_SetContactListener(g.ps, g.listener);

            let bi = JPC56_PhysicsSystem_GetBodyInterface(g.ps);
            let npq = JPC56_PhysicsSystem_GetNarrowPhaseQuery(g.ps);
            let bli = JPC56_PhysicsSystem_GetBodyLockInterface(g.ps);
            if bi.is_null() || npq.is_null() || bli.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "PhysicsSystem 子接口获取失败",
                ));
            }

            let inner = Inner {
                ps: g.ps,
                bi,
                npq,
                bli,
                temp: g.temp,
                job: g.job,
                bp: g.bp,
                ovb: g.ovb,
                olp: g.olp,
                listener: g.listener,
                cb,
                bodies: HashMap::new(),
                constraints: HashMap::new(),
                next_constraint_token: 1,
                layer_count: desc.layer_count,
                max_bodies: desc.max_bodies,
            };
            // 成功:句柄所有权移交 Inner,守卫解除。
            g.ps = std::ptr::null_mut();
            g.listener = std::ptr::null_mut();
            g.olp = std::ptr::null_mut();
            g.ovb = std::ptr::null_mut();
            g.bp = std::ptr::null_mut();
            g.job = std::ptr::null_mut();
            g.temp = std::ptr::null_mut();
            Ok(inner)
        }
    }

    fn validate_token(&self, token: u64) -> Result<JpcBodyId, SysError> {
        if token > u32::MAX as u64 || token == JPC56_BODY_ID_INVALID as u64 {
            return Err(err(
                SysErrorCode::InvalidBody,
                format!("token {token:#x} 非合法 body 句柄"),
            ));
        }
        let id = token as u32;
        if !self.bodies.contains_key(&id) {
            return Err(err(
                SysErrorCode::InvalidBody,
                format!("token {token:#x} 未创建或已移除(不悬垂,§4.C3)"),
            ));
        }
        Ok(id)
    }

    fn body_transform_of(&self, id: JpcBodyId) -> SysTransform {
        let mut pos = JpcVec3::ZERO;
        let mut rot = JpcQuat::IDENTITY;
        // SAFETY: bi 有效;id 已校验在册;body 存活;出参为栈上 POD。
        // &self 相位 = step 外,BodyInterface 只读路径线程安全(§4.A4)。
        unsafe {
            JPC56_BodyInterface_GetPositionAndRotation(self.bi, id, &mut pos, &mut rot);
        }
        SysTransform {
            translation: [pos.x, pos.y, pos.z],
            rotation: [rot.x, rot.y, rot.z, rot.w],
        }
    }

    pub(crate) fn step(&mut self, dt: f32) -> Result<crate::SysStepStats, SysError> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(err(
                SysErrorCode::InvalidDesc,
                format!("dt {dt} 必须为正有限值(固定步一致性校验在 safe 层)"),
            ));
        }

        // 运动学目标:step 前单点应用(下一固定步生效;Jolt MoveKinematic 需激活)
        let mut targets: Vec<(JpcBodyId, SysTransform)> = Vec::new();
        for (id, rec) in self.bodies.iter_mut() {
            if let Some(t) = rec.kinematic_target.take() {
                targets.push((*id, t));
            }
        }
        for (id, t) in targets {
            let q = quat_normalized(t.rotation);
            // SAFETY: id 在册、body 存活;step 相位 &mut 独占。
            unsafe {
                JPC56_BodyInterface_MoveKinematic(
                    self.bi,
                    id,
                    JpcVec3::new(t.translation[0], t.translation[1], t.translation[2]),
                    JpcQuat::new(q[0], q[1], q[2], q[3]),
                    dt,
                );
                JPC56_BodyInterface_ActivateBody(self.bi, id);
            }
        }

        // 睡眠统计(JoltC 无 activation listener,轮询差分 — VENDOR56.md §3 收窄登记)
        let dyn_ids: Vec<JpcBodyId> = self
            .bodies
            .iter()
            .filter(|(_, r)| r.kind == SysBodyKind::Dynamic)
            .map(|(id, _)| *id)
            .collect();
        let mut pre_active: Vec<bool> = Vec::with_capacity(dyn_ids.len());
        for &id in &dyn_ids {
            // SAFETY: id 在册;body 存活;step 相位 &mut 独占。
            pre_active.push(unsafe { JPC56_BodyInterface_IsActive(self.bi, id) });
        }

        self.cb.pushed_this_step.store(0, Ordering::Relaxed);
        let dropped0 = self.cb.dropped();
        let t0 = Instant::now();
        // SAFETY: 全部句柄有效;Update 为 step 相位入口(&mut 独占),job 线程只活在
        // 该调用内;contact 回调经 ContactSink Mutex 收集。
        let update_err = unsafe {
            JPC56_PhysicsSystem_Update(self.ps, dt, 1, self.temp, self.job.cast::<JpcJobSystem>())
        };
        let step_time_secs = t0.elapsed().as_secs_f64();
        if update_err != JPC56_PHYSICS_UPDATE_ERROR_NONE {
            return Err(err(
                SysErrorCode::PoolExhausted,
                format!(
                    "Jolt Update 错误位 {update_err:#x}(manifold/body-pair/contact-constraints 池满)"
                ),
            ));
        }

        let mut active_bodies = 0u32;
        let mut post_active: Vec<bool> = Vec::with_capacity(dyn_ids.len());
        for (&id, rec) in &self.bodies {
            // SAFETY: 同 pre 轮询。
            let active = unsafe { JPC56_BodyInterface_IsActive(self.bi, id) };
            if active {
                active_bodies += 1;
            }
            if rec.kind == SysBodyKind::Dynamic {
                post_active.push(active);
            }
        }
        let slept = pre_active
            .iter()
            .zip(post_active.iter())
            .filter(|(pre, post)| **pre && !**post)
            .count() as u32;

        Ok(crate::SysStepStats {
            active_bodies,
            slept_this_step: slept,
            contacts_emitted: self.cb.pushed_this_step.load(Ordering::Relaxed),
            contacts_dropped: self.cb.dropped() - dropped0,
            step_time_secs,
        })
    }

    pub(crate) fn add_bodies_batch(&mut self, descs: &[SysBodyDesc]) -> Result<Vec<u64>, SysError> {
        if descs.is_empty() {
            return Ok(Vec::new());
        }
        if self.bodies.len() + descs.len() > self.max_bodies as usize {
            return Err(err(
                SysErrorCode::PoolExhausted,
                format!(
                    "body 池耗尽:{} + {} > max_bodies {}",
                    self.bodies.len(),
                    descs.len(),
                    self.max_bodies
                ),
            ));
        }
        // 全量预校验(P-01:失败零副作用)
        for (i, d) in descs.iter().enumerate() {
            validate_body_desc(d, self.layer_count).map_err(|mut e| {
                e.message = format!("descs[{i}]: {}", e.message);
                e
            })?;
        }

        // 形状创建(失败 → 已建形状全释放,零副作用)
        let mut shapes: Vec<*mut JpcShape> = Vec::with_capacity(descs.len());
        for d in descs {
            match create_shape(d) {
                Ok(s) => shapes.push(s),
                Err(e) => {
                    // SAFETY: shapes 内均为本批 Create 成功所得、未释放过。
                    unsafe {
                        for s in &shapes {
                            JPC56_Shape_Release(*s);
                        }
                    }
                    return Err(e);
                }
            }
        }

        // 体创建(未加入世界;失败 → 销毁已建体 + 释放全部形状)
        let mut settings: Vec<JpcBodyCreationSettings> = Vec::with_capacity(descs.len());
        for (d, &shape) in descs.iter().zip(shapes.iter()) {
            settings.push(make_body_settings(d, shape));
        }
        let mut ids: Vec<JpcBodyId> = Vec::with_capacity(descs.len());
        for s in &settings {
            // SAFETY: settings 布局锚定、shape 引用存活;bi 有效;step 外交替期。
            let body = unsafe { JPC56_BodyInterface_CreateBody(self.bi, s) };
            if body.is_null() {
                // SAFETY: ids 内体已 Create 未 Add,DestroyBody 合法;shapes 未释放过。
                unsafe {
                    for &id in &ids {
                        JPC56_BodyInterface_DestroyBody(self.bi, id);
                    }
                    for s in &shapes {
                        JPC56_Shape_Release(*s);
                    }
                }
                return Err(err(
                    SysErrorCode::PoolExhausted,
                    "Jolt CreateBody 返回 null(body 池满)",
                ));
            }
            // SAFETY: body 为刚创建的有效指针。
            ids.push(unsafe { JPC56_Body_GetID(body) });
        }

        // 批插提交:prepare/finalize 单点(§4.A3;两调用间无可失败操作,故无需 Abort)
        let n = ids.len() as i32;
        // Jolt AddBodiesPrepare 原地重排 ioBodies(vendor 直通,文档明确允许;批 > 32
        // 走 Hoare 划分 QuickSort,排序键全等亦非恒等;≤ 32 走稳定 InsertionSort 恰为
        // 恒等)。重排仅置换顺序、不改 id 值——快照原始序,激活/登记/返回一律按
        // 原始序与 descs/shapes 配对(契约:返回序与输入一一对应)。
        let ids_orig = ids.clone();
        // SAFETY: ids 指向 n 个已创建未加入的 body;prepare 返回的 AddState 立即交回
        // finalize(之间数组不经他手,满足 Jolt「unmodified 回传」约定);DONT_ACTIVATE
        // 后按 ids_orig 原始序按类逐个激活(批量 finalize 仅接受单一激活模式)。
        unsafe {
            let state = JPC56_BodyInterface_AddBodiesPrepare(self.bi, ids.as_mut_ptr(), n);
            JPC56_BodyInterface_AddBodiesFinalize(
                self.bi,
                ids.as_mut_ptr(),
                n,
                state,
                JPC56_ACTIVATION_DONT_ACTIVATE,
            );
            for (&id, d) in ids_orig.iter().zip(descs.iter()) {
                if d.kind != SysBodyKind::Static {
                    JPC56_BodyInterface_ActivateBody(self.bi, id);
                }
            }
        }

        for (&id, (d, &shape)) in ids_orig.iter().zip(descs.iter().zip(shapes.iter())) {
            self.bodies.insert(
                id,
                BodyRec {
                    shape,
                    kind: d.kind,
                    kinematic_target: None,
                },
            );
        }
        Ok(ids_orig.into_iter().map(|id| id as u64).collect())
    }

    pub(crate) fn remove_bodies_batch(&mut self, tokens: &[u64]) -> Result<(), SysError> {
        // 全量预校验(P-01:任一无效 → 零移除)
        let mut ids: Vec<JpcBodyId> = Vec::with_capacity(tokens.len());
        for &t in tokens {
            ids.push(self.validate_token(t)?);
        }
        if ids.is_empty() {
            return Ok(());
        }
        let n = ids.len() as i32;
        // SAFETY: ids 全部在册且已加入世界;先 RemoveBodies 再逐 DestroyBody(Jolt 序;
        // DestroyBodies 在 JoltC impl 被上游注释,WIP 缺口处置 (c) = Rust 循环,VENDOR56.md
        // §3);之后释放各 body 持有的 shape 引用并出册(token 失效)。
        unsafe {
            JPC56_BodyInterface_RemoveBodies(self.bi, ids.as_mut_ptr(), n);
            for &id in &ids {
                JPC56_BodyInterface_DestroyBody(self.bi, id);
            }
        }
        for id in ids {
            if let Some(rec) = self.bodies.remove(&id) {
                // SAFETY: shape 引用为 add 时所得、仅此一次释放。
                unsafe { JPC56_Shape_Release(rec.shape) };
            }
        }
        Ok(())
    }

    pub(crate) fn body_transform(&self, token: u64) -> Result<SysTransform, SysError> {
        let id = self.validate_token(token)?;
        Ok(self.body_transform_of(id))
    }

    pub(crate) fn active_transforms(&self) -> Vec<(u64, SysTransform)> {
        let mut out = Vec::new();
        for (&id, rec) in &self.bodies {
            if rec.kind == SysBodyKind::Static {
                continue;
            }
            // SAFETY: id 在册;step 外只读路径线程安全(§4.A4)。
            if unsafe { JPC56_BodyInterface_IsActive(self.bi, id) } {
                out.push((id as u64, self.body_transform_of(id)));
            }
        }
        out
    }

    pub(crate) fn set_kinematic_target(
        &mut self,
        token: u64,
        target: &SysTransform,
    ) -> Result<(), SysError> {
        let id = self.validate_token(token)?;
        if !target.translation.iter().all(|v| v.is_finite())
            || !target.rotation.iter().all(|v| v.is_finite())
        {
            return Err(err(SysErrorCode::InvalidDesc, "目标变换必须有限"));
        }
        let rec = self.bodies.get_mut(&id).expect("token 已校验在册");
        if rec.kind != SysBodyKind::Kinematic {
            return Err(err(
                SysErrorCode::InvalidDesc,
                format!("token {token:#x} 非运动学体"),
            ));
        }
        rec.kinematic_target = Some(*target);
        Ok(())
    }

    pub(crate) fn apply_impulse(&mut self, token: u64, impulse: [f32; 3]) -> Result<(), SysError> {
        let id = self.validate_token(token)?;
        if !impulse.iter().all(|v| v.is_finite()) {
            return Err(err(SysErrorCode::InvalidDesc, "impulse 必须有限"));
        }
        // SAFETY: id 在册;先激活(睡眠体冲量需激活方生效 — §4.A7 睡眠唤醒锚)再施加。
        unsafe {
            JPC56_BodyInterface_ActivateBody(self.bi, id);
            JPC56_BodyInterface_AddImpulse(
                self.bi,
                id,
                JpcVec3::new(impulse[0], impulse[1], impulse[2]),
            );
        }
        Ok(())
    }

    pub(crate) fn add_force_at_point(
        &mut self,
        token: u64,
        force: [f32; 3],
        point: [f32; 3],
    ) -> Result<(), SysError> {
        let id = self.validate_token(token)?;
        if !force.iter().chain(point.iter()).all(|v| v.is_finite()) {
            return Err(err(SysErrorCode::InvalidDesc, "force/point 必须有限"));
        }
        // SAFETY: id 在册;先激活(睡眠体受力需激活方生效,同 apply_impulse
        // 的 §4.A7 唤醒锚)再于世界系点施力。DOUBLE_PRECISION=OFF 档
        // JPC56_RVec3 == JPC56_Vec3,point 布局等价。
        unsafe {
            JPC56_BodyInterface_ActivateBody(self.bi, id);
            JPC56_BodyInterface_AddForceAtPoint(
                self.bi,
                id,
                JpcVec3::new(force[0], force[1], force[2]),
                JpcVec3::new(point[0], point[1], point[2]),
            );
        }
        Ok(())
    }

    pub(crate) fn is_active(&self, token: u64) -> Result<bool, SysError> {
        let id = self.validate_token(token)?;
        // SAFETY: id 在册;只读路径。
        Ok(unsafe { JPC56_BodyInterface_IsActive(self.bi, id) })
    }

    pub(crate) fn body_velocities(&self, token: u64) -> Result<([f32; 3], [f32; 3]), SysError> {
        let id = self.validate_token(token)?;
        // SAFETY: id 在册;BodyInterface 速度只读路径 step 外线程安全。
        unsafe {
            let lin = JPC56_BodyInterface_GetLinearVelocity(self.bi, id);
            let ang = JPC56_BodyInterface_GetAngularVelocity(self.bi, id);
            Ok(([lin.x, lin.y, lin.z], [ang.x, ang.y, ang.z]))
        }
    }

    pub(crate) fn set_linear_velocity(
        &mut self,
        token: u64,
        linear: [f32; 3],
    ) -> Result<(), SysError> {
        let id = self.validate_token(token)?;
        if !linear.iter().all(|v| v.is_finite()) {
            return Err(err(SysErrorCode::InvalidDesc, "linear velocity 必须有限"));
        }
        // SAFETY: id 在册;step 相位 &mut 独占;不附带激活(与 injection 纪律一致)。
        unsafe {
            JPC56_BodyInterface_SetLinearVelocity(
                self.bi,
                id,
                JpcVec3::new(linear[0], linear[1], linear[2]),
            );
        }
        Ok(())
    }

    pub(crate) fn set_angular_velocity(
        &mut self,
        token: u64,
        angular: [f32; 3],
    ) -> Result<(), SysError> {
        let id = self.validate_token(token)?;
        if !angular.iter().all(|v| v.is_finite()) {
            return Err(err(SysErrorCode::InvalidDesc, "angular velocity 必须有限"));
        }
        // SAFETY: 同 set_linear_velocity。
        unsafe {
            JPC56_BodyInterface_SetAngularVelocity(
                self.bi,
                id,
                JpcVec3::new(angular[0], angular[1], angular[2]),
            );
        }
        Ok(())
    }

    /// 写入位姿且不激活(注入面;`JPC56_ACTIVATION_DONT_ACTIVATE`)。
    pub(crate) fn set_position_rotation_dont_activate(
        &mut self,
        token: u64,
        transform: &SysTransform,
    ) -> Result<(), SysError> {
        let id = self.validate_token(token)?;
        if !transform.translation.iter().all(|v| v.is_finite())
            || !transform.rotation.iter().all(|v| v.is_finite())
        {
            return Err(err(SysErrorCode::InvalidDesc, "transform 必须有限"));
        }
        // SAFETY: id 在册;DontActivate 不附带激活副作用(F-12)。
        unsafe {
            JPC56_BodyInterface_SetPositionAndRotation(
                self.bi,
                id,
                JpcVec3::new(
                    transform.translation[0],
                    transform.translation[1],
                    transform.translation[2],
                ),
                JpcQuat::new(
                    transform.rotation[0],
                    transform.rotation[1],
                    transform.rotation[2],
                    transform.rotation[3],
                ),
                JPC56_ACTIVATION_DONT_ACTIVATE,
            );
        }
        Ok(())
    }

    pub(crate) fn set_position_rotation_and_velocity(
        &mut self,
        token: u64,
        transform: &SysTransform,
        linear: [f32; 3],
        angular: [f32; 3],
    ) -> Result<(), SysError> {
        let id = self.validate_token(token)?;
        if !transform.translation.iter().all(|v| v.is_finite())
            || !transform.rotation.iter().all(|v| v.is_finite())
            || !linear.iter().all(|v| v.is_finite())
            || !angular.iter().all(|v| v.is_finite())
        {
            return Err(err(SysErrorCode::InvalidDesc, "pose/velocity 必须有限"));
        }
        // SAFETY: id 在册;step 相位 &mut 独占。
        unsafe {
            JPC56_BodyInterface_SetPositionRotationAndVelocity(
                self.bi,
                id,
                JpcVec3::new(
                    transform.translation[0],
                    transform.translation[1],
                    transform.translation[2],
                ),
                JpcQuat::new(
                    transform.rotation[0],
                    transform.rotation[1],
                    transform.rotation[2],
                    transform.rotation[3],
                ),
                JpcVec3::new(linear[0], linear[1], linear[2]),
                JpcVec3::new(angular[0], angular[1], angular[2]),
            );
        }
        Ok(())
    }

    /// 世界空间铰链约束(两体 + 铰链点/轴);返回 constraint token。
    pub(crate) fn add_hinge_constraint(
        &mut self,
        body_a: u64,
        body_b: u64,
        point: [f32; 3],
        hinge_axis: [f32; 3],
        normal_axis: [f32; 3],
    ) -> Result<u64, SysError> {
        let id_a = self.validate_token(body_a)?;
        let id_b = self.validate_token(body_b)?;
        if id_a == id_b {
            return Err(err(SysErrorCode::InvalidDesc, "hinge 两体不得相同"));
        }
        if !point
            .iter()
            .chain(hinge_axis.iter())
            .chain(normal_axis.iter())
            .all(|v| v.is_finite())
        {
            return Err(err(SysErrorCode::InvalidDesc, "hinge 参数必须有限"));
        }
        // SAFETY: BodyLockMultiWrite 成对加锁;GetBody 仅锁期内用于 Create;
        // Create 后 refcount=0 → AddRef(registry) → AddConstraint(系统 AddRef);
        // Drop/remove: RemoveConstraint + Release。
        unsafe {
            use crate::ffi::{
                JPC56_BodyLockMultiWrite_GetBody, JPC56_BodyLockMultiWrite_delete,
                JPC56_BodyLockMultiWrite_new,
            };
            let mut ids = [id_a, id_b];
            if ids[0] > ids[1] {
                ids.swap(0, 1);
            }
            let multi = JPC56_BodyLockMultiWrite_new(self.bli, ids.as_ptr(), 2);
            if multi.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "BodyLockMultiWrite_new 失败",
                ));
            }
            let body_lo = JPC56_BodyLockMultiWrite_GetBody(multi, 0);
            let body_hi = JPC56_BodyLockMultiWrite_GetBody(multi, 1);
            if body_lo.is_null() || body_hi.is_null() {
                JPC56_BodyLockMultiWrite_delete(multi);
                return Err(err(SysErrorCode::InvalidBody, "hinge body 锁定失败"));
            }
            let (body_ptr_a, body_ptr_b) = if id_a <= id_b {
                (body_lo, body_hi)
            } else {
                (body_hi, body_lo)
            };
            let mut settings = std::mem::zeroed::<JpcHingeConstraintSettings>();
            JPC56_HingeConstraintSettings_default(&mut settings);
            settings.space = JPC56_CONSTRAINT_SPACE_WORLD_SPACE;
            settings.point1 = JpcVec3::new(point[0], point[1], point[2]);
            settings.point2 = JpcVec3::new(point[0], point[1], point[2]);
            settings.hinge_axis1 = JpcVec3::new(hinge_axis[0], hinge_axis[1], hinge_axis[2]);
            settings.hinge_axis2 = JpcVec3::new(hinge_axis[0], hinge_axis[1], hinge_axis[2]);
            settings.normal_axis1 = JpcVec3::new(normal_axis[0], normal_axis[1], normal_axis[2]);
            settings.normal_axis2 = JpcVec3::new(normal_axis[0], normal_axis[1], normal_axis[2]);
            let hinge = JPC56_HingeConstraintSettings_Create(&settings, body_ptr_a, body_ptr_b);
            JPC56_BodyLockMultiWrite_delete(multi);
            if hinge.is_null() {
                return Err(err(
                    SysErrorCode::BackendUnavailable,
                    "HingeConstraintSettings_Create 返回 null",
                ));
            }
            let constraint = hinge as *mut JpcConstraint;
            JPC56_Constraint_AddRef(constraint);
            JPC56_PhysicsSystem_AddConstraint(self.ps, constraint);
            let token = self.next_constraint_token;
            self.next_constraint_token = self.next_constraint_token.saturating_add(1);
            self.constraints.insert(
                token,
                ConstraintRec {
                    ptr: constraint,
                    body_a: id_a,
                    body_b: id_b,
                    motor_state: JPC56_MOTOR_STATE_OFF,
                },
            );
            Ok(token)
        }
    }

    pub(crate) fn remove_constraint(&mut self, token: u64) -> Result<(), SysError> {
        let Some(rec) = self.constraints.remove(&token) else {
            return Err(err(
                SysErrorCode::InvalidDesc,
                format!("无效 constraint token {token:#x}"),
            ));
        };
        // SAFETY: ptr 为本 world 创建并 AddConstraint 过;Remove 释 manager Ref,
        // Release 释 registry AddRef(Create 后 AddRef 配对,缺一则泄漏或双重释放)。
        unsafe {
            JPC56_PhysicsSystem_RemoveConstraint(self.ps, rec.ptr);
            JPC56_Constraint_Release(rec.ptr);
        }
        Ok(())
    }

    pub(crate) fn set_hinge_motor(
        &mut self,
        token: u64,
        state: u32,
        target_angular_velocity: f32,
    ) -> Result<(), SysError> {
        let Some(rec) = self.constraints.get_mut(&token) else {
            return Err(err(
                SysErrorCode::InvalidDesc,
                format!("无效 constraint token {token:#x}"),
            ));
        };
        if !target_angular_velocity.is_finite() {
            return Err(err(SysErrorCode::InvalidDesc, "motor target 必须有限"));
        }
        // SAFETY: ptr 为有效 HingeConstraint*;step 相位 &mut 独占。
        unsafe {
            let hinge = rec.ptr as *mut JpcHingeConstraint;
            JPC56_HingeConstraint_SetMotorState(hinge, state);
            JPC56_HingeConstraint_SetTargetAngularVelocity(hinge, target_angular_velocity);
        }
        rec.motor_state = state;
        Ok(())
    }

    pub(crate) fn constraint_snapshot(&self) -> Vec<(u64, u64, u64, bool, u32)> {
        let mut out: Vec<(u64, u64, u64, bool, u32)> = self
            .constraints
            .iter()
            .map(|(&token, rec)| {
                // SAFETY: ptr 在册;GetEnabled 只读。
                let enabled = unsafe { JPC56_Constraint_GetEnabled(rec.ptr) };
                (
                    token,
                    rec.body_a as u64,
                    rec.body_b as u64,
                    enabled,
                    rec.motor_state,
                )
            })
            .collect();
        out.sort_by_key(|(t, _, _, _, _)| *t);
        out
    }

    pub(crate) fn num_bodies(&self) -> u32 {
        self.bodies.len() as u32
    }

    pub(crate) fn cast_ray(&self, ray: &SysRay) -> Vec<SysHit> {
        let dir_len =
            (ray.dir[0] * ray.dir[0] + ray.dir[1] * ray.dir[1] + ray.dir[2] * ray.dir[2]).sqrt();
        if !ray.origin.iter().all(|v| v.is_finite())
            || !ray.dir.iter().all(|v| v.is_finite())
            || !ray.t_min.is_finite()
            || !ray.t_max.is_finite()
            || dir_len < 1e-12
            || ray.t_min < 0.0
            || ray.t_max <= ray.t_min
        {
            return Vec::new(); // 非法射线:无 Result 通道,返回空(契约签名所限,文档登记)
        }

        let span = ray.t_max - ray.t_min;
        let origin = [
            ray.origin[0] + ray.dir[0] * ray.t_min,
            ray.origin[1] + ray.dir[1] * ray.t_min,
            ray.origin[2] + ray.dir[2] * ray.t_min,
        ];
        let direction = [ray.dir[0] * span, ray.dir[1] * span, ray.dir[2] * span];

        let mask: u64 = ray.layer_mask;
        let mut excluded: Vec<u32> = Vec::new();
        // SAFETY: 过滤器 self 指向本帧栈上 mask/excluded,过滤器对象经 *_new/*_delete
        // 配对,生命周期严格短于本调用(U27 栈纪律);npq 只读路径 step 外并发安全。
        let olf = unsafe {
            JPC56_ObjectLayerFilter_new(
                (&mask as *const u64).cast(),
                JpcObjectLayerFilterFns {
                    should_collide: Some(ol_should_collide),
                },
            )
        };
        // SAFETY: 同 olf — self 指向本帧栈上 excluded,配对 delete 在本函数末。
        let bf = unsafe {
            JPC56_BodyFilter_new(
                (&excluded as *const Vec<u32>).cast(),
                JpcBodyFilterFns {
                    should_collide: Some(ray_body_should_collide),
                    should_collide_locked: Some(ray_body_should_collide_locked),
                },
            )
        };
        let mut hits = Vec::new();
        if !olf.is_null() && !bf.is_null() {
            // JoltC CastRay = 仅最近命中(impl ClosestHitCollisionCollector)——排除循环
            // 实现契约「全命中返回」(零 C++ 补丁,VENDOR56.md §3 计划外缺口登记)
            loop {
                if excluded.len() >= MAX_RAY_HITS {
                    break;
                }
                let mut args = JpcCastRayArgs {
                    ray: JpcRRayCast {
                        origin: JpcVec3::new(origin[0], origin[1], origin[2]),
                        direction: JpcVec3::new(direction[0], direction[1], direction[2]),
                    },
                    result: JpcRayCastResult::default(),
                    broad_phase_layer_filter: std::ptr::null(),
                    object_layer_filter: olf,
                    body_filter: bf,
                    shape_filter: std::ptr::null(),
                };
                // SAFETY: args 为栈上布局锚定 POD;过滤器有效;npq 有效。
                let hit = unsafe { JPC56_NarrowPhaseQuery_CastRay(self.npq, &mut args) };
                if !hit {
                    break;
                }
                let id = args.result.body_id;
                let t = ray.t_min + args.result.fraction * span;
                let position = [
                    ray.origin[0] + ray.dir[0] * t,
                    ray.origin[1] + ray.dir[1] * t,
                    ray.origin[2] + ray.dir[2] * t,
                ];
                let normal = self.surface_normal(id, args.result.sub_shape_id2, position);
                hits.push(SysHit {
                    body: id as u64,
                    t,
                    position,
                    normal,
                });
                excluded.push(id);
            }
        }
        // SAFETY: 与上方 *_new 配对,过滤器此后不再被引用。
        unsafe {
            if !bf.is_null() {
                JPC56_BodyFilter_delete(bf);
            }
            if !olf.is_null() {
                JPC56_ObjectLayerFilter_delete(olf);
            }
        }
        hits
    }

    /// 命中面法线回填:BodyLockRead + GetWorldSpaceSurfaceNormal(JoltC CastRay 结果
    /// 不含法线 — 既有 API 组合,无补丁;锁失败 → [0,0,0] 登记)。
    fn surface_normal(
        &self,
        id: JpcBodyId,
        sub_shape: JpcSubShapeId,
        position: [f32; 3],
    ) -> [f32; 3] {
        // SAFETY: bli 有效;id 刚被查询命中(存活);锁配对 new/delete;GetBody 仅在
        // Succeeded 后调用,返回指针锁期内有效。
        unsafe {
            let lock = JPC56_BodyLockRead_new(self.bli, id);
            if lock.is_null() {
                return [0.0; 3];
            }
            let out = if JPC56_BodyLockRead_Succeeded(lock) {
                let body = JPC56_BodyLockRead_GetBody(lock);
                if body.is_null() {
                    [0.0; 3]
                } else {
                    let n = JPC56_Body_GetWorldSpaceSurfaceNormal(
                        body,
                        sub_shape,
                        JpcVec3::new(position[0], position[1], position[2]),
                    );
                    [n.x, n.y, n.z]
                }
            } else {
                [0.0; 3]
            };
            JPC56_BodyLockRead_delete(lock);
            out
        }
    }

    pub(crate) fn cast_shape(
        &self,
        shape: &SysShapeParams,
        start: &SysTransform,
        dir: [f32; 3],
        t_max: f32,
        layer_mask: u64,
    ) -> Vec<SysHit> {
        if !start.translation.iter().all(|v| v.is_finite())
            || !start.rotation.iter().all(|v| v.is_finite())
            || !dir.iter().all(|v| v.is_finite())
            || !t_max.is_finite()
            || t_max <= 0.0
        {
            return Vec::new();
        }
        let query_shape = match create_query_shape(shape) {
            Some(s) => s,
            None => return Vec::new(),
        };

        // CenterOfMassStart:平移 = start.translation + start.rotation * COM(形状质心偏移)
        let q = quat_normalized(start.rotation);
        // SAFETY: query_shape 为 create_query_shape 成功所得,本调用内持有。
        let com = unsafe { JPC56_Shape_GetCenterOfMass(query_shape) };
        let com_world = quat_rotate(q, [com.x, com.y, com.z]);
        let t = [
            start.translation[0] + com_world[0],
            start.translation[1] + com_world[1],
            start.translation[2] + com_world[2],
        ];

        let mask = layer_mask;
        let mut ctx = CastCollectCtx {
            hits: Vec::new(),
            t_max,
        };
        // SAFETY: collector/过滤器 self 指向本帧栈上 ctx/mask,生命周期严格短于本
        // 调用;CastShape 在调用线程同步执行;settings 经 _default 填充。
        unsafe {
            let collector = JPC56_CastShapeCollector_new(
                (&mut ctx as *mut CastCollectCtx).cast(),
                JpcCastShapeCollectorFns {
                    reset: None,
                    add_hit: Some(cast_shape_add_hit),
                },
            );
            let olf = JPC56_ObjectLayerFilter_new(
                (&mask as *const u64).cast(),
                JpcObjectLayerFilterFns {
                    should_collide: Some(ol_should_collide),
                },
            );
            if !collector.is_null() && !olf.is_null() {
                let mut settings = std::mem::zeroed::<JpcShapeCastSettings>();
                JPC56_ShapeCastSettings_default(&mut settings);
                let mut args = JpcCastShapeArgs {
                    shape_cast: JpcRShapeCast {
                        shape: query_shape,
                        scale: JpcVec3::new(1.0, 1.0, 1.0),
                        center_of_mass_start: mat44_from_rot_trans(q, t),
                        direction: JpcVec3::new(dir[0] * t_max, dir[1] * t_max, dir[2] * t_max),
                    },
                    settings,
                    base_offset: JpcVec3::ZERO,
                    collector,
                    broad_phase_layer_filter: std::ptr::null(),
                    object_layer_filter: olf,
                    body_filter: std::ptr::null(),
                    shape_filter: std::ptr::null(),
                };
                JPC56_NarrowPhaseQuery_CastShape(self.npq, &mut args);
            }
            if !olf.is_null() {
                JPC56_ObjectLayerFilter_delete(olf);
            }
            if !collector.is_null() {
                JPC56_CastShapeCollector_delete(collector);
            }
            JPC56_Shape_Release(query_shape);
        }
        ctx.hits
    }

    pub(crate) fn overlap_shape(
        &self,
        shape: &SysShapeParams,
        transform: &SysTransform,
        layer_mask: u64,
    ) -> Vec<u64> {
        if !transform.translation.iter().all(|v| v.is_finite())
            || !transform.rotation.iter().all(|v| v.is_finite())
        {
            return Vec::new();
        }
        let query_shape = match create_query_shape(shape) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let q = quat_normalized(transform.rotation);
        // SAFETY: 同 cast_shape 的 COM 偏移与栈纪律。
        let com = unsafe { JPC56_Shape_GetCenterOfMass(query_shape) };
        let com_world = quat_rotate(q, [com.x, com.y, com.z]);
        let t = [
            transform.translation[0] + com_world[0],
            transform.translation[1] + com_world[1],
            transform.translation[2] + com_world[2],
        ];

        let mask = layer_mask;
        let mut ctx = OverlapCollectCtx { ids: Vec::new() };
        // SAFETY: 同 cast_shape。
        unsafe {
            let collector = JPC56_CollideShapeCollector_new(
                (&mut ctx as *mut OverlapCollectCtx).cast(),
                JpcCollideShapeCollectorFns {
                    reset: None,
                    add_hit: Some(collide_shape_add_hit),
                },
            );
            let olf = JPC56_ObjectLayerFilter_new(
                (&mask as *const u64).cast(),
                JpcObjectLayerFilterFns {
                    should_collide: Some(ol_should_collide),
                },
            );
            if !collector.is_null() && !olf.is_null() {
                let mut settings = std::mem::zeroed::<JpcCollideShapeSettings>();
                JPC56_CollideShapeSettings_default(&mut settings);
                let mut args = JpcCollideShapeArgs {
                    shape: query_shape,
                    shape_scale: JpcVec3::new(1.0, 1.0, 1.0),
                    center_of_mass_transform: mat44_from_rot_trans(q, t),
                    settings,
                    base_offset: JpcVec3::ZERO,
                    collector,
                    broad_phase_layer_filter: std::ptr::null(),
                    object_layer_filter: olf,
                    body_filter: std::ptr::null(),
                    shape_filter: std::ptr::null(),
                };
                JPC56_NarrowPhaseQuery_CollideShape(self.npq, &mut args);
            }
            if !olf.is_null() {
                JPC56_ObjectLayerFilter_delete(olf);
            }
            if !collector.is_null() {
                JPC56_CollideShapeCollector_delete(collector);
            }
            JPC56_Shape_Release(query_shape);
        }
        ctx.ids
    }

    pub(crate) fn drain_contacts(&mut self) -> (Vec<SysContactEvent>, u32) {
        self.cb.drain()
    }
}

// ---------------------------------------------------------------------------
// 描述校验与形状/体创建
// ---------------------------------------------------------------------------

fn validate_body_desc(d: &SysBodyDesc, layer_count: u32) -> Result<(), SysError> {
    if d.layer >= layer_count {
        return Err(err(
            SysErrorCode::InvalidDesc,
            format!("layer {} 超出 layer_count {}", d.layer, layer_count),
        ));
    }
    if !d.translation.iter().all(|v| v.is_finite())
        || !d.rotation.iter().all(|v| v.is_finite())
        || !d.mass.is_finite()
        || d.mass < 0.0
        || !d.friction.is_finite()
        || d.friction < 0.0
        || !d.restitution.is_finite()
        || d.restitution < 0.0
    {
        return Err(err(
            SysErrorCode::InvalidDesc,
            "变换/mass/friction/restitution 非法(须有限,mass/friction/restitution ≥ 0)",
        ));
    }
    match &d.shape {
        SysShapeParams::Sphere { radius } => positive(*radius, "radius"),
        SysShapeParams::Box { half_extents } => half_extents
            .iter()
            .try_for_each(|v| positive(*v, "half_extents")),
        SysShapeParams::Capsule {
            half_height,
            radius,
        } => positive(*half_height, "half_height").and_then(|_| positive(*radius, "radius")),
        SysShapeParams::ConvexHull { points } => {
            if points.len() < 4 || !points.iter().flatten().all(|v| v.is_finite()) {
                return Err(err(SysErrorCode::InvalidDesc, "ConvexHull 需 ≥ 4 个有限点"));
            }
            Ok(())
        }
        SysShapeParams::StaticMesh {
            vertices,
            triangles,
        } => {
            if d.kind != SysBodyKind::Static {
                return Err(err(
                    SysErrorCode::InvalidDesc,
                    "StaticMesh 仅 Static 体(§4.A2)",
                ));
            }
            if vertices.is_empty()
                || triangles.is_empty()
                || !vertices.iter().flatten().all(|v| v.is_finite())
                || triangles
                    .iter()
                    .flatten()
                    .any(|&i| i as usize >= vertices.len())
            {
                return Err(err(
                    SysErrorCode::InvalidDesc,
                    "StaticMesh 顶点/索引非法(空网格或索引越界)",
                ));
            }
            Ok(())
        }
    }
}

fn positive(v: f32, name: &str) -> Result<(), SysError> {
    if !v.is_finite() || v <= 0.0 {
        return Err(err(
            SysErrorCode::InvalidDesc,
            format!("{name} 必须为正有限值"),
        ));
    }
    Ok(())
}

/// mass 语义:dynamic 且 mass > 0 → 密度 = mass / 体积(两遍创建);mass == 0 → Jolt
/// 默认密度(1000 kg/m³);其余非法已在 validate 拦截。
fn create_shape(d: &SysBodyDesc) -> Result<*mut JpcShape, SysError> {
    let first = build_shape(&d.shape, 1000.0)?;
    if d.kind == SysBodyKind::Dynamic && d.mass > 0.0 {
        // SAFETY: first 为 Create 成功所得引用,读取体积后即释放。
        let volume = unsafe { JPC56_Shape_GetVolume(first) };
        // SAFETY: 同上,仅此一次释放。
        unsafe { JPC56_Shape_Release(first) };
        if !volume.is_finite() || volume <= 0.0 {
            return Err(err(
                SysErrorCode::InvalidDesc,
                "形状体积非法(无法映射显式 mass)",
            ));
        }
        build_shape(&d.shape, d.mass / volume)
    } else {
        Ok(first)
    }
}

/// 查询用临时形状(默认密度;调用方持有引用并负责 Release)。
fn create_query_shape(params: &SysShapeParams) -> Option<*mut JpcShape> {
    build_shape(params, 1000.0).ok()
}

fn take_create_error(err_str: *mut JpcString, what: &str) -> SysError {
    if err_str.is_null() {
        return err(
            SysErrorCode::InvalidDesc,
            format!("{what} 形状创建失败(无错误串)"),
        );
    }
    // SAFETY: err_str 为 Create 失败路径返回的有效 JPC56_String;读完即 delete。
    let msg = unsafe {
        let c = JPC56_String_c_str(err_str);
        let s = if c.is_null() {
            String::new()
        } else {
            CStr::from_ptr(c).to_string_lossy().into_owned()
        };
        JPC56_String_delete(err_str);
        s
    };
    err(
        SysErrorCode::InvalidDesc,
        format!("{what} 形状创建失败:{msg}"),
    )
}

/// 形状创建统一出口:default 填充 → 覆写本切片消费字段 → Create(成功 = 引用计数 1)。
fn build_shape(params: &SysShapeParams, density: f32) -> Result<*mut JpcShape, SysError> {
    let mut shape: *mut JpcShape = std::ptr::null_mut();
    let mut err_str: *mut JpcString = std::ptr::null_mut();
    match params {
        SysShapeParams::Sphere { radius } => {
            // SAFETY: settings 布局锚定;out 参数为栈上指针;失败经 take_create_error 清理。
            unsafe {
                let mut s = std::mem::zeroed::<JpcSphereShapeSettings>();
                JPC56_SphereShapeSettings_default(&mut s);
                s.radius = *radius;
                s.density = density;
                if !JPC56_SphereShapeSettings_Create(&s, &mut shape, &mut err_str) {
                    return Err(take_create_error(err_str, "Sphere"));
                }
            }
        }
        SysShapeParams::Box { half_extents } => {
            // SAFETY: 同上。
            unsafe {
                let mut s = std::mem::zeroed::<JpcBoxShapeSettings>();
                JPC56_BoxShapeSettings_default(&mut s);
                s.half_extent = JpcVec3::new(half_extents[0], half_extents[1], half_extents[2]);
                s.density = density;
                if !JPC56_BoxShapeSettings_Create(&s, &mut shape, &mut err_str) {
                    return Err(take_create_error(err_str, "Box"));
                }
            }
        }
        SysShapeParams::Capsule {
            half_height,
            radius,
        } => {
            // SAFETY: 同上。
            unsafe {
                let mut s = std::mem::zeroed::<JpcCapsuleShapeSettings>();
                JPC56_CapsuleShapeSettings_default(&mut s);
                s.radius = *radius;
                s.half_height_of_cylinder = *half_height;
                s.density = density;
                if !JPC56_CapsuleShapeSettings_Create(&s, &mut shape, &mut err_str) {
                    return Err(take_create_error(err_str, "Capsule"));
                }
            }
        }
        SysShapeParams::ConvexHull { points } => {
            let pts: Vec<JpcVec3> = points
                .iter()
                .map(|p| JpcVec3::new(p[0], p[1], p[2]))
                .collect();
            // SAFETY: pts 在 Create 调用期存活(同步调用,拷贝语义);其余同上。
            unsafe {
                let mut s = std::mem::zeroed::<JpcConvexHullShapeSettings>();
                JPC56_ConvexHullShapeSettings_default(&mut s);
                s.points = pts.as_ptr();
                s.points_len = pts.len();
                s.density = density;
                if !JPC56_ConvexHullShapeSettings_Create(&s, &mut shape, &mut err_str) {
                    return Err(take_create_error(err_str, "ConvexHull"));
                }
            }
        }
        SysShapeParams::StaticMesh {
            vertices,
            triangles,
        } => {
            let mut verts: Vec<JpcFloat3> = vertices
                .iter()
                .map(|v| JpcFloat3 {
                    x: v[0],
                    y: v[1],
                    z: v[2],
                })
                .collect();
            let mut tris: Vec<JpcIndexedTriangle> = triangles
                .iter()
                .map(|t| JpcIndexedTriangle {
                    idx: *t,
                    material_index: 0,
                    user_data: 0,
                })
                .collect();
            // SAFETY: verts/tris 在 Create 调用期存活(同步调用,拷贝语义);其余同上。
            unsafe {
                let mut s = std::mem::zeroed::<JpcMeshShapeSettings>();
                JPC56_MeshShapeSettings_default(&mut s);
                s.triangle_vertices = verts.as_mut_ptr();
                s.triangle_vertices_len = verts.len();
                s.indexed_triangles = tris.as_mut_ptr();
                s.indexed_triangles_len = tris.len();
                if !JPC56_MeshShapeSettings_Create(&s, &mut shape, &mut err_str) {
                    return Err(take_create_error(err_str, "StaticMesh"));
                }
            }
        }
    }
    if shape.is_null() {
        return Err(err(
            SysErrorCode::BackendUnavailable,
            "形状 Create 成功但返回 null",
        ));
    }
    Ok(shape)
}

/// BodyCreationSettings:default 填充后覆写;CCD → MotionQuality::LinearCast(§4.A3);
/// 睡眠默认开由 desc.allow_sleeping 透传(宿主可关)。
fn make_body_settings(d: &SysBodyDesc, shape: *const JpcShape) -> JpcBodyCreationSettings {
    // SAFETY: settings 布局锚定 POD;zeroed 后经 _default 填充合法默认值。
    let mut s = unsafe {
        let mut s = std::mem::zeroed::<JpcBodyCreationSettings>();
        JPC56_BodyCreationSettings_default(&mut s);
        s
    };
    let q = quat_normalized(d.rotation);
    s.position = JpcVec3::new(d.translation[0], d.translation[1], d.translation[2]);
    s.rotation = JpcQuat::new(q[0], q[1], q[2], q[3]);
    s.object_layer = d.layer as JpcObjectLayer;
    s.motion_type = match d.kind {
        SysBodyKind::Static => JPC56_MOTION_TYPE_STATIC,
        SysBodyKind::Kinematic => JPC56_MOTION_TYPE_KINEMATIC,
        SysBodyKind::Dynamic => JPC56_MOTION_TYPE_DYNAMIC,
    };
    s.allowed_dofs = JPC56_ALLOWED_DOFS_ALL;
    s.allow_dynamic_or_kinematic = u8::from(d.kind != SysBodyKind::Static);
    s.motion_quality = if d.ccd {
        JPC56_MOTION_QUALITY_LINEAR_CAST
    } else {
        JPC56_MOTION_QUALITY_DISCRETE
    };
    s.allow_sleeping = u8::from(d.allow_sleep);
    s.friction = d.friction;
    s.restitution = d.restitution;
    s.override_mass_properties = JPC56_OVERRIDE_MASS_PROPS_CALC_MASS_INERTIA;
    s.shape = shape;
    s
}
