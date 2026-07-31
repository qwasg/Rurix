//! 公共 API 锚定测试(后端无关面,两构建档恒跑;真 Jolt 行为测试属集成轮,不在此)。
//!
//! 覆盖(§4.A7 host 单测锚的后端无关子集):BackendNotCompiled 两路径、WorldDesc
//! 校验、SyncBudget 公共消耗语义、冻结接口类型面字面锚、PhysicsError Display/Error。

use rurix_physics::{
    BackendKind, BodyDesc, BodyId, BodyKind, ContactEvent, ContactPhase, MassProps, OverlapHit,
    PhysicsError, PhysicsTransform, PhysicsWorld, QueryHit, QueryRay, QueryShape, ShapeDesc,
    ShapeId, StepStats, SyncBudget, WorldDesc,
};

#[test]
fn rapier_backend_always_not_compiled() {
    let desc = WorldDesc {
        backend: BackendKind::Rapier,
        ..Default::default()
    };
    assert_eq!(
        PhysicsWorld::new(desc).unwrap_err(),
        PhysicsError::BackendNotCompiled(BackendKind::Rapier),
        "Rapier 在 G6.4 实现前一律 Err(BackendNotCompiled),含 default(=jolt) 档"
    );
}

#[cfg(not(feature = "jolt"))]
#[test]
fn jolt_backend_not_compiled_without_feature() {
    assert_eq!(
        PhysicsWorld::new(WorldDesc::default()).unwrap_err(),
        PhysicsError::BackendNotCompiled(BackendKind::Jolt),
        "--no-default-features 零 C++ 依赖:Jolt 未编译 → 确定性 Err,不静默回退"
    );
}

#[test]
fn world_desc_default_shape_matches_rfc() {
    let d = WorldDesc::default();
    assert_eq!(d.backend, BackendKind::Jolt);
    assert_eq!(d.gravity, [0.0, -9.81, 0.0]);
    assert_eq!(d.dt_fixed, 1.0 / 60.0);
    assert!(d.layer_count >= 1 && d.max_bodies >= 1 && d.job_threads.is_none());
}

#[test]
fn invalid_world_desc_is_deterministic_err() {
    // 校验先于后端分派:无后端档同样命中 InvalidDesc(P-01 确定性 Err)。
    let bad = WorldDesc {
        dt_fixed: -1.0,
        ..Default::default()
    };
    assert!(matches!(
        PhysicsWorld::new(bad),
        Err(PhysicsError::InvalidDesc(_))
    ));
}

#[test]
fn sync_budget_three_axis_consumption_and_reset() {
    let mut b = SyncBudget {
        max_body_writes: 1,
        max_contact_events: 2,
        max_query_casts: 1,
    };
    // 三轴独立消耗,余量归零即停。
    assert!(b.try_consume_body_write());
    assert!(!b.try_consume_body_write());
    assert_eq!(
        b.consume_contact_events(5),
        2,
        "批量申请实发 = min(剩余, 请求)"
    );
    assert!(!b.try_consume_contact_event());
    assert!(b.try_consume_query_cast());
    assert!(!b.try_consume_query_cast());
    // 重置 = 宿主每帧重新构造(§4.A6)。
    let mut next_frame = SyncBudget::new(1, 2, 1);
    assert!(next_frame.try_consume_query_cast());
}

#[test]
fn frozen_type_faces_constructible() {
    // 冻结接口字面锚(§4.0-3):字段名/类型漂移 = 编译失败。
    let ray = QueryRay {
        origin: [0.0; 3],
        dir: [0.0, -1.0, 0.0],
        t_min: 0.0,
        t_max: 50.0,
        layer_mask: 0b1,
    };
    let _ = ray;
    let body = BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Capsule {
            half_height: 0.25,
            radius: 0.5,
        },
        layer: 0,
        mass_props: MassProps {
            mass: 80.0,
            friction: 0.5,
            restitution: 0.0,
            allow_sleep: true,
        },
        ccd: true,
        transform: PhysicsTransform::IDENTITY,
    };
    let _ = body;
    let stats = StepStats {
        active_bodies: 0,
        slept_this_step: 0,
        contacts_emitted: 0,
        contacts_dropped: 0,
        step_time: std::time::Duration::ZERO,
    };
    let _ = stats;
    let shape_query = QueryShape {
        shape: ShapeDesc::Sphere { radius: 1.0 },
        start: PhysicsTransform {
            translation: [0.0, 10.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
        dir: [0.0, -1.0, 0.0],
        t_max: 20.0,
        layer_mask: u64::MAX,
    };
    let _ = shape_query;
    let hull = ShapeDesc::ConvexHull {
        points: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };
    let _ = hull;
    let mesh = ShapeDesc::StaticMesh {
        vertices: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![[0, 1, 2]],
    };
    let _ = mesh;
    // 事件/命中类型面(字段锚;实例构造需 BodyId,经世界产出,此锚编译期字段)。
    assert!(
        ContactPhase::Begin < ContactPhase::Persist && ContactPhase::Persist < ContactPhase::End,
        "相位声明序 = 规范序相位序(§4.A5)"
    );
    let _e: fn(ContactEvent) -> (ContactPhase, f32) = |ev| (ev.phase, ev.impulse);
    let _h: fn(QueryHit) -> (BodyId, ShapeId, f32) = |h| (h.body, h.shape, h.t);
    let _o: fn(OverlapHit) -> ShapeId = |h| h.shape;
}

#[test]
fn physics_error_display_and_std_error() {
    let errs = [
        PhysicsError::BackendNotCompiled(BackendKind::Jolt),
        PhysicsError::BackendNotCompiled(BackendKind::Rapier),
        PhysicsError::BackendUnavailable("x".into()),
        PhysicsError::PoolExhausted,
        PhysicsError::InvalidDesc("y".into()),
        PhysicsError::FixedStepMismatch {
            expected: 1.0 / 60.0,
            got: 0.1,
        },
        PhysicsError::BudgetSaturated,
    ];
    for e in &errs {
        assert!(!format!("{e}").is_empty());
        let _: &dyn std::error::Error = e;
    }
}
