//! rurix-physics-sys — JoltC FFI 边界(G6.2,RFC-0017 §4.C)。
//!
//! 本 crate 是物理 unsafe 唯一集中地(`unsafe_code = "allow"` 块级豁免,
//! 注册 `unsafe-audit/rurix-physics-sys.md`,U33 起续号;每块 `// SAFETY:`)。
//!
//! 边界契约(RFC-0017 §4.C3 ABI 纪律 + §4.0-1 crate 布局):
//! - 对外只露 safe Rust 类型与 u64 句柄(token),不露原生 Jolt/JoltC 指针与类型名;
//! - 跨 FFI 只过 `#[repr(C)]` POD 与 u64;所有权单向(world 拥有 body/shape);
//! - 本文件 `SysWorld` 公共签名 = sys ↔ safe crate 边界**契约**,实现内部可自由,
//!   签名不得漂移(确需修订 = 追加式并在交付摘要登记,由汇装层裁决)。

mod ffi;
mod world;

use std::fmt;

/// sys 层错误码(库层状态值,零新 RX 码;RFC-0017 §5.1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysErrorCode {
    /// 描述非法(尺寸越界 / layer 超上限 / 动态 StaticMesh 等,P-01 确定性 Err)
    InvalidDesc,
    /// body token 无效(未创建 / 已移除;不悬垂,§4.C3)
    InvalidBody,
    /// body 池耗尽(`max_bodies` 上限)
    PoolExhausted,
    /// FFI 层不可用(JoltC 构建缺失 / 初始化失败;fail-closed)
    BackendUnavailable,
}

/// sys 层错误(确定性 Err,不 panic,P-01)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysError {
    pub code: SysErrorCode,
    pub message: String,
}

impl fmt::Display for SysError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for SysError {}

/// 世界描述(镜像 RFC-0017 §4.A1 `WorldDesc` 的 sys 投影;`dt_fixed` 校验在 safe 层)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SysWorldDesc {
    pub gravity: [f32; 3],
    /// object layer 数(Jolt ObjectLayer 位宽约束内,上限随实现登记)
    pub layer_count: u32,
    pub max_bodies: u32,
    /// job 线程数;0 = 可用并行度(默认库内线程池,§4.A3 job 适配层)
    pub job_threads: u32,
    /// 接触事件 ring 容量(溢出确定性丢最旧 + 计数,§4.A5)
    pub contact_capacity: u32,
}

/// 体类型(§4.A2 `BodyDesc.kind`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SysBodyKind {
    Static = 0,
    Kinematic = 1,
    Dynamic = 2,
}

/// 形状参数(safe Rust 枚举;内部转 JoltC shape 创建调用,FFI 只过 repr(C) 展开)。
/// `StaticMesh` 仅 Static 体(动态 mesh → `InvalidDesc`,§4.A2)。
#[derive(Debug, Clone, PartialEq)]
pub enum SysShapeParams {
    Sphere {
        radius: f32,
    },
    Box {
        half_extents: [f32; 3],
    },
    Capsule {
        half_height: f32,
        radius: f32,
    },
    ConvexHull {
        points: Vec<[f32; 3]>,
    },
    /// 三角形汤(顶点 + 三角形索引),仅 Static 体
    StaticMesh {
        vertices: Vec<[f32; 3]>,
        triangles: Vec<[u32; 3]>,
    },
}

/// 体描述(§4.A2 `BodyDesc` 的 sys 投影 + 初始变换)。
#[derive(Debug, Clone, PartialEq)]
pub struct SysBodyDesc {
    pub kind: SysBodyKind,
    pub shape: SysShapeParams,
    pub layer: u32,
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub ccd: bool,
    pub allow_sleep: bool,
    pub translation: [f32; 3],
    /// xyzw quat
    pub rotation: [f32; 4],
}

/// 刚体变换(与 `PhysicsTransform` 同构;sys 层独立类型,边界不共享内存)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SysTransform {
    pub translation: [f32; 3],
    /// xyzw quat
    pub rotation: [f32; 4],
}

/// 单步统计(§4.A1 `StepStats` sys 投影;`step_time_secs` 仅供 evidence 不进硬门)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SysStepStats {
    pub active_bodies: u32,
    pub slept_this_step: u32,
    pub contacts_emitted: u32,
    pub contacts_dropped: u32,
    pub step_time_secs: f64,
}

/// 射线查询(§4.A4 `QueryRay`)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SysRay {
    pub origin: [f32; 3],
    pub dir: [f32; 3],
    pub t_min: f32,
    pub t_max: f32,
    pub layer_mask: u64,
}

/// 查询命中(§4.A4 `QueryHit` sys 投影;`shape` 由 safe 层按 body 记录回填)。
/// 返回顺序**不保证**(Jolt collector 顺序可变);规范序排序在 safe 层(§4.A4 C-2)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SysHit {
    /// Jolt 侧 body token(u64 widened)
    pub body: u64,
    pub t: f32,
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// 接触事件相位(§4.A5)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SysContactPhase {
    Begin = 0,
    Persist = 1,
    End = 2,
}

/// 接触事件(原始回调序;归一化排序去重在 safe 层,§4.A5)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SysContactEvent {
    pub a: u64,
    pub b: u64,
    pub phase: SysContactPhase,
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub impulse: f32,
}

/// Jolt 世界拥有句柄(safe wrapper;内部持 JoltC 原生指针,所有权单向,Drop 销毁)。
///
/// 线程纪律(§4.C3):`step`/`add_*`/`remove_*`/`drain_contacts` = `&mut`(step 相位);
/// `cast_*`/`overlap_*`/`body_transform`/`active_transforms`/`is_active` = `&self`,
/// step 外多线程全并发(Jolt NarrowPhaseQuery 只读路径线程安全,§4.A4 Q-B)。
pub struct SysWorld {
    // 内部:JoltC PhysicsSystem / BodyInterface / JobSystem / shape registry / contact ring。
    inner: world::Inner,
}

impl SysWorld {
    /// 创建世界(JoltC 初始化 + PhysicsSystem + 层过滤 + 默认 job 池)。
    pub fn create(desc: &SysWorldDesc) -> Result<SysWorld, SysError> {
        Ok(SysWorld {
            inner: world::Inner::create(desc)?,
        })
    }

    /// 固定步推进(dt 一致性校验在 safe 层;step 相位内 Jolt job 线程活动)。
    pub fn step(&mut self, dt: f32) -> Result<SysStepStats, SysError> {
        self.inner.step(dt)
    }

    /// 批插体(AddBodiesPrepare/AddBodiesFinalize;prepare 在 step 外交替期,
    /// finalize 单点提交,批插期间主步延迟 ≤ 1 帧,§4.A3/§4.A7 C-6)。
    /// 返回 Jolt 侧 body token(u64),顺序与输入一一对应。
    pub fn add_bodies_batch(&mut self, descs: &[SysBodyDesc]) -> Result<Vec<u64>, SysError> {
        self.inner.add_bodies_batch(descs)
    }

    /// 批移除(移除后 token 失效,二次使用 → `InvalidBody`,§4.C3)。
    pub fn remove_bodies_batch(&mut self, tokens: &[u64]) -> Result<(), SysError> {
        self.inner.remove_bodies_batch(tokens)
    }

    /// 读 body 当前变换。
    pub fn body_transform(&self, token: u64) -> Result<SysTransform, SysError> {
        self.inner.body_transform(token)
    }

    /// 上一拍变换快照:step 结束边界提交的 active 动态/运动体变换浅拷贝
    /// (§4.A4 变换读;仅数组,不复制加速结构)。
    pub fn active_transforms(&self) -> Vec<(u64, SysTransform)> {
        self.inner.active_transforms()
    }

    /// 运动学体目标变换(Jolt MoveKinematic;下一固定步生效)。
    pub fn set_kinematic_target(
        &mut self,
        token: u64,
        target: &SysTransform,
    ) -> Result<(), SysError> {
        self.inner.set_kinematic_target(token, target)
    }

    /// 冲量唤醒(睡眠体冲量 → 激活,§4.A7 睡眠唤醒单测锚)。
    pub fn apply_impulse(&mut self, token: u64, impulse: [f32; 3]) -> Result<(), SysError> {
        self.inner.apply_impulse(token, impulse)
    }

    /// 在世界系指定点施力(M70 悬挂/驱动/侧向力;力在下一 step 内积分)。
    pub fn add_force_at_point(
        &mut self,
        token: u64,
        force: [f32; 3],
        point: [f32; 3],
    ) -> Result<(), SysError> {
        self.inner.add_force_at_point(token, force, point)
    }

    /// body 是否激活(未睡眠)。
    pub fn is_active(&self, token: u64) -> Result<bool, SysError> {
        self.inner.is_active(token)
    }

    /// 线速度 + 角速度(世界系)。
    pub fn body_velocities(&self, token: u64) -> Result<([f32; 3], [f32; 3]), SysError> {
        self.inner.body_velocities(token)
    }

    /// 写线速度(不附带激活)。
    pub fn set_linear_velocity(&mut self, token: u64, linear: [f32; 3]) -> Result<(), SysError> {
        self.inner.set_linear_velocity(token, linear)
    }

    /// 写角速度(不附带激活)。
    pub fn set_angular_velocity(&mut self, token: u64, angular: [f32; 3]) -> Result<(), SysError> {
        self.inner.set_angular_velocity(token, angular)
    }

    /// 写位姿且 DontActivate(注入白名单面)。
    pub fn set_position_rotation_dont_activate(
        &mut self,
        token: u64,
        transform: &SysTransform,
    ) -> Result<(), SysError> {
        self.inner
            .set_position_rotation_dont_activate(token, transform)
    }

    /// 写位姿 + 速度。
    pub fn set_position_rotation_and_velocity(
        &mut self,
        token: u64,
        transform: &SysTransform,
        linear: [f32; 3],
        angular: [f32; 3],
    ) -> Result<(), SysError> {
        self.inner
            .set_position_rotation_and_velocity(token, transform, linear, angular)
    }

    /// 世界空间铰链约束;返回 constraint token。
    pub fn add_hinge_constraint(
        &mut self,
        body_a: u64,
        body_b: u64,
        point: [f32; 3],
        hinge_axis: [f32; 3],
        normal_axis: [f32; 3],
    ) -> Result<u64, SysError> {
        self.inner
            .add_hinge_constraint(body_a, body_b, point, hinge_axis, normal_axis)
    }

    pub fn remove_constraint(&mut self, token: u64) -> Result<(), SysError> {
        self.inner.remove_constraint(token)
    }

    pub fn set_hinge_motor(
        &mut self,
        token: u64,
        state: u32,
        target_angular_velocity: f32,
    ) -> Result<(), SysError> {
        self.inner
            .set_hinge_motor(token, state, target_angular_velocity)
    }

    /// `(token, body_a, body_b, enabled, motor_state)` 按 token 升序。
    pub fn constraint_snapshot(&self) -> Vec<(u64, u64, u64, bool, u32)> {
        self.inner.constraint_snapshot()
    }

    /// 当前 body 数。
    pub fn num_bodies(&self) -> u32 {
        self.inner.num_bodies()
    }

    /// 射线 cast(step 外并发;全命中返回,顺序未规范化 — 排序在 safe 层)。
    /// 非法射线(零方向 / 非有限 / 空区间)→ 空 Vec(签名无 Result 通道,登记)。
    pub fn cast_ray(&self, ray: &SysRay) -> Vec<SysHit> {
        self.inner.cast_ray(ray)
    }

    /// 形状 cast(同 cast_ray 并发纪律;`dir` 为位移方向,扫掠 [0, t_max])。
    /// 非法输入(非有限 / t_max ≤ 0 / 形状创建失败)→ 空 Vec(同上登记)。
    pub fn cast_shape(
        &self,
        shape: &SysShapeParams,
        start: &SysTransform,
        dir: [f32; 3],
        t_max: f32,
        layer_mask: u64,
    ) -> Vec<SysHit> {
        self.inner.cast_shape(shape, start, dir, t_max, layer_mask)
    }

    /// 形状 overlap(同 cast_ray 并发纪律;返回命中 body token 集,已去重)。
    /// 非法输入 → 空 Vec(同上登记)。
    pub fn overlap_shape(
        &self,
        shape: &SysShapeParams,
        transform: &SysTransform,
        layer_mask: u64,
    ) -> Vec<u64> {
        self.inner.overlap_shape(shape, transform, layer_mask)
    }

    /// 取走本步原始接触事件(回调收集,未归一化)+ 自上次 drain 以来溢出丢弃计数。
    /// 归一化(规范序排序去重)在 safe 层(§4.A5)。
    pub fn drain_contacts(&mut self) -> (Vec<SysContactEvent>, u32) {
        self.inner.drain_contacts()
    }
}

// 线程纪律(§4.C3):Send/Sync 显式实现 + SAFETY 论证。
// SAFETY: SysWorld 独占拥有全部 JoltC 句柄(PhysicsSystem/BodyInterface/JobSystem/
// 监听器与过滤器),所有权单向;跨线程 move 后旧线程不再触碰。Jolt PhysicsSystem 的
// 非线程安全面由库内相位门封闭(§4.A4 Q-B):step/add/remove/drain/set_kinematic/
// apply_impulse = &mut self(Rust 借用规则编译期保证独占,step 相位内 Jolt job 线程
// 只活在 Update 调用内);cast_*/overlap_*/body_transform/active_transforms/is_active
// = &self,对应 Jolt NarrowPhaseQuery/BodyInterface 只读路径(step 外多线程并发安全,
// Jolt 上游文档口径),与 step 相位类型面互斥;contact 回调经 Mutex<Vec> 收集,与并发
// 查询无共享可变状态。故 SysWorld 可安全 Send/Sync。
unsafe impl Send for SysWorld {}
// SAFETY: 同上(&self 并发路径全为 Jolt 只读查询 + 不可变注册表读;可变路径被 &mut 独占)。
unsafe impl Sync for SysWorld {}

// ---------------------------------------------------------------------------
// in-crate 单测(G6.2 PR-A 锚;§4.A7 host 单测的 sys 层子集)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;
    const ALL_LAYERS: u64 = u64::MAX;

    fn desc(job_threads: u32) -> SysWorldDesc {
        SysWorldDesc {
            gravity: [0.0, -9.81, 0.0],
            layer_count: 4,
            max_bodies: 1024,
            job_threads,
            contact_capacity: 256,
        }
    }

    fn identity() -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    /// 地面:静态箱,顶面 y = 0(半高 0.5,中心 y = -0.5)。
    fn ground_desc() -> SysBodyDesc {
        SysBodyDesc {
            kind: SysBodyKind::Static,
            shape: SysShapeParams::Box {
                half_extents: [20.0, 0.5, 20.0],
            },
            layer: 0,
            mass: 0.0,
            friction: 0.5,
            restitution: 0.0,
            ccd: false,
            allow_sleep: true,
            translation: [0.0, -0.5, 0.0],
            rotation: identity(),
        }
    }

    fn sphere_desc(x: f32, y: f32, z: f32) -> SysBodyDesc {
        SysBodyDesc {
            kind: SysBodyKind::Dynamic,
            shape: SysShapeParams::Sphere { radius: 0.5 },
            layer: 1,
            mass: 1.0,
            friction: 0.5,
            restitution: 0.0,
            ccd: false,
            allow_sleep: true,
            translation: [x, y, z],
            rotation: identity(),
        }
    }

    /// 1) 世界创建/销毁(+ 单线程 job 可用 + Send/Sync 相位门静态断言)。
    #[test]
    fn world_create_destroy() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<SysWorld>();

        let world = SysWorld::create(&desc(0)).expect("多线程 job 世界创建");
        assert_eq!(world.num_bodies(), 0);
        drop(world);

        let mut world = SysWorld::create(&desc(1)).expect("单线程 job 世界创建");
        world.step(DT).expect("单线程 job step");
        drop(world);

        // 非法描述 → 确定性 Err(P-01)
        let mut bad = desc(0);
        bad.layer_count = 0;
        assert_eq!(
            SysWorld::create(&bad)
                .err()
                .expect("layer_count=0 应失败")
                .code,
            SysErrorCode::InvalidDesc
        );
        let mut bad = desc(0);
        bad.max_bodies = 0;
        assert_eq!(
            SysWorld::create(&bad)
                .err()
                .expect("max_bodies=0 应失败")
                .code,
            SysErrorCode::InvalidDesc
        );
    }

    /// 2) 球体重力下落:自由落体 → 落地停在地面顶面(y ≈ 0.5)。
    #[test]
    fn sphere_falls_under_gravity() {
        let mut world = SysWorld::create(&desc(0)).unwrap();
        world.add_bodies_batch(&[ground_desc()]).unwrap();
        let ball = world
            .add_bodies_batch(&[sphere_desc(0.0, 5.0, 0.0)])
            .unwrap()[0];

        let t0 = world.body_transform(ball).unwrap();
        assert!(
            (t0.translation[1] - 5.0).abs() < 1e-4,
            "初始变换 round-trip"
        );

        let mut saw_active = false;
        let mut y_mid = 0.0;
        for i in 0..300 {
            let stats = world.step(DT).unwrap();
            saw_active |= stats.active_bodies > 0;
            if i == 29 {
                y_mid = world.body_transform(ball).unwrap().translation[1];
            }
        }
        assert!(saw_active, "下落期 active_bodies > 0");
        assert!(y_mid < 5.0 - 0.3, "30 步后应明显下落,y = {y_mid}");

        let tf = world.body_transform(ball).unwrap();
        assert!(
            (tf.translation[1] - 0.5).abs() < 0.1,
            "沉降后球心应停在地面顶面 y ≈ 0.5,实际 {}",
            tf.translation[1]
        );
    }

    /// 3) 批插 N 体(AddBodiesPrepare/Finalize)+ 批移除 + 池/描述校验。
    #[test]
    fn batch_insert_and_remove() {
        let mut world = SysWorld::create(&desc(0)).unwrap();
        world.add_bodies_batch(&[ground_desc()]).unwrap();

        let descs: Vec<SysBodyDesc> = (0..64)
            .map(|i| sphere_desc((i % 8) as f32 * 1.5 - 5.25, 3.0 + (i / 8) as f32 * 1.5, 0.0))
            .collect();
        let tokens = world.add_bodies_batch(&descs).unwrap();
        assert_eq!(tokens.len(), 64);
        assert_eq!(world.num_bodies(), 65);
        let unique: std::collections::HashSet<u64> = tokens.iter().copied().collect();
        assert_eq!(unique.len(), 64, "token 唯一");

        for _ in 0..5 {
            world.step(DT).unwrap();
        }
        assert!(
            !world.active_transforms().is_empty(),
            "step 后 active 动态体快照非空"
        );

        world.remove_bodies_batch(&tokens).unwrap();
        assert_eq!(world.num_bodies(), 1);
        assert_eq!(
            world.body_transform(tokens[0]).unwrap_err().code,
            SysErrorCode::InvalidBody,
            "移除后 token 失效(§4.C3)"
        );

        // 池耗尽 → 确定性 Err(P-01)
        let mut small = desc(0);
        small.max_bodies = 2;
        let mut w2 = SysWorld::create(&small).unwrap();
        w2.add_bodies_batch(&[ground_desc()]).unwrap();
        assert_eq!(
            w2.add_bodies_batch(&descs).unwrap_err().code,
            SysErrorCode::PoolExhausted
        );
        // 非法 layer → 确定性 Err(InvalidDesc)
        let mut bad = sphere_desc(0.0, 1.0, 0.0);
        bad.layer = 99;
        assert_eq!(
            world.add_bodies_batch(&[bad]).unwrap_err().code,
            SysErrorCode::InvalidDesc
        );
    }

    /// 4) 射线命中:先穿球(近)再中地面(远),t 升序;法线朝上;layer_mask 过滤。
    #[test]
    fn ray_hits_ground() {
        let mut world = SysWorld::create(&desc(0)).unwrap();
        let ground = world.add_bodies_batch(&[ground_desc()]).unwrap()[0];
        let ball = world
            .add_bodies_batch(&[sphere_desc(0.0, 2.5, 0.0)])
            .unwrap()[0];

        let hits = world.cast_ray(&SysRay {
            origin: [0.0, 5.0, 0.0],
            dir: [0.0, -1.0, 0.0],
            t_min: 0.0,
            t_max: 100.0,
            layer_mask: ALL_LAYERS,
        });
        assert_eq!(hits.len(), 2, "应命中球 + 地面:{hits:?}");
        assert!(hits[0].t < hits[1].t, "近命中在前");
        assert_eq!(hits[0].body, ball);
        assert!(
            (hits[0].t - 2.0).abs() < 1e-3,
            "球面 t ≈ 2,实际 {}",
            hits[0].t
        );
        assert_eq!(hits[1].body, ground);
        assert!(
            (hits[1].t - 5.0).abs() < 1e-3,
            "地面顶面 t ≈ 5,实际 {}",
            hits[1].t
        );
        assert!((hits[1].position[1]).abs() < 1e-3, "命中点 y ≈ 0");
        assert!(
            hits[1].normal[1] > 0.99,
            "地面法线朝上:{:?}",
            hits[1].normal
        );

        // layer_mask 不含球层(1)与地面层(0)→ 空
        assert!(
            world
                .cast_ray(&SysRay {
                    origin: [0.0, 5.0, 0.0],
                    dir: [0.0, -1.0, 0.0],
                    t_min: 0.0,
                    t_max: 100.0,
                    layer_mask: 1 << 3,
                })
                .is_empty(),
            "mask 过滤后无命中"
        );
        // 朝天射线 → 空
        assert!(
            world
                .cast_ray(&SysRay {
                    origin: [0.0, 5.0, 0.0],
                    dir: [0.0, 1.0, 0.0],
                    t_min: 0.0,
                    t_max: 100.0,
                    layer_mask: ALL_LAYERS,
                })
                .is_empty()
        );
    }

    /// 5) 接触事件产生与 drain:落地产生 Begin/Persist;原始序返回;容量内零丢弃。
    #[test]
    fn contact_events_drained() {
        let mut world = SysWorld::create(&desc(0)).unwrap();
        let ground = world.add_bodies_batch(&[ground_desc()]).unwrap()[0];
        let ball = world
            .add_bodies_batch(&[sphere_desc(0.0, 2.0, 0.0)])
            .unwrap()[0];

        // 起始 drain 清空
        let (ev0, dropped0) = world.drain_contacts();
        assert!(ev0.is_empty() && dropped0 == 0);

        let mut events = Vec::new();
        let mut emitted_seen = false;
        for _ in 0..600 {
            let stats = world.step(DT).unwrap();
            emitted_seen |= stats.contacts_emitted > 0;
            let (ev, dropped) = world.drain_contacts();
            assert_eq!(dropped, 0, "容量 256 内零丢弃");
            events.extend(ev);
            if !events.is_empty() {
                break;
            }
        }
        assert!(emitted_seen, "StepStats.contacts_emitted 见到非零");
        assert!(!events.is_empty(), "落地应产生接触事件");
        let involves_pair = events.iter().any(|e| {
            matches!(e.phase, SysContactPhase::Begin | SysContactPhase::Persist)
                && ((e.a == ball && e.b == ground) || (e.a == ground && e.b == ball))
        });
        assert!(involves_pair, "事件应关联球/地面 body 对:{events:?}");
        // impulse 首版恒 0(VENDOR.md §3 缺口收窄登记)
        assert!(events.iter().all(|e| e.impulse == 0.0));
    }

    /// 6) 无效 token → 确定性 Err(InvalidBody)(不悬垂,§4.C3)。
    #[test]
    fn invalid_token_errors() {
        let mut world = SysWorld::create(&desc(0)).unwrap();
        const BOGUS: u64 = 0xDEAD;
        const WIDE: u64 = u32::MAX as u64 + 1;

        for token in [BOGUS, WIDE] {
            assert_eq!(
                world.body_transform(token).unwrap_err().code,
                SysErrorCode::InvalidBody
            );
            assert_eq!(
                world.is_active(token).unwrap_err().code,
                SysErrorCode::InvalidBody
            );
            assert_eq!(
                world
                    .apply_impulse(token, [0.0, 1.0, 0.0])
                    .unwrap_err()
                    .code,
                SysErrorCode::InvalidBody
            );
            assert_eq!(
                world.remove_bodies_batch(&[token]).unwrap_err().code,
                SysErrorCode::InvalidBody
            );
        }

        // 合法创建 → 移除 → 二次使用失效
        let ball = world
            .add_bodies_batch(&[sphere_desc(0.0, 3.0, 0.0)])
            .unwrap()[0];
        world.remove_bodies_batch(&[ball]).unwrap();
        assert_eq!(
            world.body_transform(ball).unwrap_err().code,
            SysErrorCode::InvalidBody
        );
    }

    #[test]
    fn hinge_constraint_step_smoke() {
        fn box_desc(kind: SysBodyKind, x: f32, y: f32, z: f32, half: [f32; 3]) -> SysBodyDesc {
            SysBodyDesc {
                kind,
                shape: SysShapeParams::Box { half_extents: half },
                layer: if kind == SysBodyKind::Static { 0 } else { 1 },
                mass: if kind == SysBodyKind::Dynamic { 1.0 } else { 0.0 },
                friction: 0.5,
                restitution: 0.0,
                ccd: false,
                allow_sleep: true,
                translation: [x, y, z],
                rotation: identity(),
            }
        }
        let mut world = SysWorld::create(&desc(1)).unwrap();
        let _ground = world
            .add_bodies_batch(&[ground_desc()])
            .unwrap()[0];
        let anchor = world
            .add_bodies_batch(&[box_desc(SysBodyKind::Static, 0.0, 4.0, 0.0, [0.3, 0.3, 0.3])])
            .unwrap()[0];
        let bar = world
            .add_bodies_batch(&[box_desc(SysBodyKind::Dynamic, 0.0, 2.0, 0.0, [0.25, 0.25, 0.25])])
            .unwrap()[0];
        let token = world
            .add_hinge_constraint(anchor, bar, [0.0, 4.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0])
            .unwrap();
        assert!(token > 0);
    }
}
