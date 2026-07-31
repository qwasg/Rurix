//! 渲染合流桥行为测试(RFC-0017 §4.B,G6.3;真后端,default(= jolt)档运行;
//! `--no-default-features` 档本文件整体 cfg 出局,后端无关单测见 src/bridge/ 模块内)。
//!
//! 测试名逐字对齐 G6.3 契约清单(CI 步骤 89 physics_bridge_smoke.py 按关键字 grep)。
//!
//! 并发纪律说明(诚实边界,镜像 tests/behavior.rs):safe API 相位门 = step/add/remove
//! 取 `&mut self`、变换快照读 `&self`——「卸载与物理写并发」的机验形态 = 互斥交替下
//! Barrier 编排的「写 → 卸 → 写」时序脚本(真线程注入);Rust 借用规则下不存在、
//! 也不允许绕过相位纪律的并发写路径。

#![cfg(feature = "jolt")]

use std::collections::HashSet;
use std::sync::{Barrier, Mutex};

use rurix_physics::{
    BodyDesc, BodyKind, FrameSyncReport, MassProps, PageKey, PhysicsBridge, PhysicsError,
    PhysicsTransform, PhysicsWorld, RemovalReceipt, ShapeDesc, StreamingBridge, SyncBudget,
    WorldDesc, compose_transform_3x4,
};
use rurix_render::geometry::gpu_scene::{DirtyRange, GpuScene, IDENTITY_3X4};

const DT: f32 = 1.0 / 60.0;
const IDENTITY_ROT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn world_desc(job_threads: Option<u32>) -> WorldDesc {
    WorldDesc {
        gravity: [0.0, -9.81, 0.0],
        layer_count: 4,
        max_bodies: 1024,
        job_threads,
        dt_fixed: DT,
        contact_capacity: 64,
        ..Default::default()
    }
}

/// 地面:静态箱,顶面 y = 0(半高 0.5,中心 y = -0.5;镜像 tests/behavior.rs)。
fn ground_desc() -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Static,
        shape: ShapeDesc::Box {
            half_extents: [20.0, 0.5, 20.0],
        },
        layer: 0,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [0.0, -0.5, 0.0],
            rotation: IDENTITY_ROT,
        },
    }
}

/// 动态球(半径 0.5,球心 (x, y, z);allow_sleep 默认开)。
fn dyn_sphere(x: f32, y: f32, z: f32) -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Sphere { radius: 0.5 },
        layer: 1,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [x, y, z],
            rotation: IDENTITY_ROT,
        },
    }
}

/// 动态箱(半长 half,中心 (x, y, z))。
fn dyn_box(x: f32, y: f32, z: f32, half: f32) -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Box {
            half_extents: [half, half, half],
        },
        layer: 1,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [x, y, z],
            rotation: IDENTITY_ROT,
        },
    }
}

/// 静态箱(半长 half,中心 (x, y, z))。
fn static_box(x: f32, y: f32, z: f32, half: f32) -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Static,
        shape: ShapeDesc::Box {
            half_extents: [half, half, half],
        },
        layer: 0,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [x, y, z],
            rotation: IDENTITY_ROT,
        },
    }
}

/// 场景:n 个共享单位盒 mesh 的实例(恒等变换),返回 (scene, instance ids);
/// 建实例的初始脏先行 flush(断言只面向 sync_frame 结算的脏集)。
fn fresh_scene(n: u32) -> (GpuScene, Vec<u32>) {
    let mut scene = GpuScene::new();
    let mesh = scene.add_mesh(0, 1, [-0.5; 3], [0.5; 3]);
    let insts: Vec<u32> = (0..n)
        .map(|_| scene.add_instance(mesh, IDENTITY_3X4, 0, 0))
        .collect();
    scene.flush_dirty();
    (scene, insts)
}

fn approx34(a: &[[f32; 4]; 3], b: &[[f32; 4]; 3]) -> bool {
    (0..3).all(|i| (0..4).all(|j| (a[i][j] - b[i][j]).abs() <= 1e-6))
}

/// dirty_ranges 覆盖的实例集合(半开区间展开)。
fn covered_instances(ranges: &[DirtyRange]) -> HashSet<u32> {
    ranges.iter().flat_map(|r| r.start..r.end).collect()
}

/// 静置箱 step 至睡眠(allow_sleep 默认开;上限 400 步,镜像 behavior.rs sleep 锚),
/// 期间每帧同步桥;返回是否入睡。
fn settle_until_asleep(
    w: &mut PhysicsWorld,
    bridge: &mut PhysicsBridge,
    scene: &mut GpuScene,
    body: rurix_physics::BodyId,
) -> bool {
    for _ in 0..400u32 {
        w.step(DT).unwrap();
        bridge.sync_frame(w, scene, &mut SyncBudget::new(16, 0, 0));
        if !w.is_active(body).unwrap() {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// (1) 单向同步:只写 active 动态体(§4.B1-1/§4.B2)
// ---------------------------------------------------------------------------

#[test]
fn one_way_sync_writes_active_dynamic_only() {
    let mut w = PhysicsWorld::new(world_desc(None)).unwrap();
    let ground = w.add_bodies_batch(&[ground_desc()]).unwrap()[0];
    let ball = w.add_bodies_batch(&[dyn_sphere(0.0, 5.0, 0.0)]).unwrap()[0];
    let (mut scene, inst) = fresh_scene(2);
    // 静态实例初值 = 地面初始变换(此后任何帧不得被改写)。
    let ground_initial = compose_transform_3x4(&ground_desc().transform);
    scene.update_transform(inst[0], ground_initial);
    scene.flush_dirty();
    let mut bridge = PhysicsBridge::new();
    bridge.register(ground, inst[0], BodyKind::Static);
    bridge.register(ball, inst[1], BodyKind::Dynamic);

    w.step(DT).unwrap();
    let report = bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(16, 0, 0));

    // 只有动态 active 体被写;静态体零写(§4.B2 静态零脏写)。
    assert_eq!(report.bodies_seen, 1, "active 快照只含动态体");
    assert_eq!(report.bodies_written, 1);
    assert_eq!(report.writes_truncated, 0);
    assert_eq!(report.dirty_instances, vec![inst[1]]);
    // GpuScene 实例变换 == compose(物理变换)。
    let phys = w.body_transform(ball).unwrap();
    assert!(
        approx34(
            &scene.instances()[inst[1] as usize].transform,
            &compose_transform_3x4(&phys)
        ),
        "实例变换须等于 compose(物理变换)"
    );
    // 静态实例逐位未动,且不在任何 dirty 区间。
    assert_eq!(
        scene.instances()[inst[0] as usize].transform,
        ground_initial,
        "静态体零脏写(逐位)"
    );
    assert!(
        report
            .dirty_ranges
            .iter()
            .all(|r| inst[0] < r.start || inst[0] >= r.end),
        "静态实例不得出现在 dirty_ranges"
    );
}

// ---------------------------------------------------------------------------
// (2) 睡眠体零写零 MV(§4.A3/§4.B2/§4.B3)
// ---------------------------------------------------------------------------

#[test]
fn sleeping_body_zero_write_zero_mv() {
    let mut w = PhysicsWorld::new(world_desc(None)).unwrap();
    let ground = w.add_bodies_batch(&[ground_desc()]).unwrap()[0];
    let rest = w
        .add_bodies_batch(&[dyn_box(0.0, 0.452, 0.0, 0.45)])
        .unwrap()[0];
    let (mut scene, inst) = fresh_scene(2);
    let mut bridge = PhysicsBridge::new();
    bridge.register(ground, inst[0], BodyKind::Static);
    bridge.register(rest, inst[1], BodyKind::Dynamic);

    // 静置至睡眠,记录睡眠前最后一次写入基线。
    let mut last_written = None;
    for _ in 0..400u32 {
        w.step(DT).unwrap();
        bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(16, 0, 0));
        if let Some(h) = bridge.motion_hints().iter().find(|h| h.instance == inst[1]) {
            last_written = Some(h.cur_transform);
        }
        if !w.is_active(rest).unwrap() {
            break;
        }
    }
    assert!(!w.is_active(rest).unwrap(), "静置箱应在 400 步内入睡");
    let last_written = last_written.expect("入睡前必有写入基线");

    // 睡眠后:bodies_seen == 0、零写、零 MV、dirty_ranges 为空。
    w.step(DT).unwrap();
    let report = bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(16, 0, 0));
    assert_eq!(report.bodies_seen, 0, "睡眠体不进 active 快照");
    assert_eq!(report.bodies_written, 0, "睡眠体零脏写");
    assert_eq!(report.writes_truncated, 0);
    assert!(report.dirty_instances.is_empty());
    assert!(report.dirty_ranges.is_empty(), "零写帧 dirty 为空");
    assert!(bridge.motion_hints().is_empty(), "睡眠体零 MV");

    // 唤醒连续性:prev = 睡眠前最后一次写入(睡眠期未动,不产生假 MV)。
    w.apply_impulse(rest, [0.0, 3.0, 0.0]).unwrap();
    w.step(DT).unwrap();
    let report = bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(16, 0, 0));
    assert_eq!(report.bodies_written, 1);
    let hints = bridge.motion_hints();
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].instance, inst[1]);
    assert_eq!(
        hints[0].prev_transform, last_written,
        "唤醒后 hint.prev = 睡眠前最后写入基线(逐位;睡眠期未动 → 无假 MV)"
    );
}

// ---------------------------------------------------------------------------
// (3) SyncBudget 截断确定性(§4.A6 + §4.0-4)
// ---------------------------------------------------------------------------

#[test]
fn budget_truncation_deterministic() {
    fn run_once() -> Vec<FrameSyncReport> {
        // 单线程 job 钉确定性(镜像 behavior.rs fixed_step 口径)。
        let mut w = PhysicsWorld::new(world_desc(Some(1))).unwrap();
        // 三个自由落体球:全程 active,无接触无睡眠。
        let bodies = w
            .add_bodies_batch(&[
                dyn_sphere(-3.0, 50.0, 0.0),
                dyn_sphere(0.0, 55.0, 0.0),
                dyn_sphere(3.0, 60.0, 0.0),
            ])
            .unwrap();
        let (mut scene, inst) = fresh_scene(3);
        let mut bridge = PhysicsBridge::new();
        for (i, &b) in bodies.iter().enumerate() {
            bridge.register(b, inst[i], BodyKind::Dynamic);
        }

        // 帧 1:额度 1 → written 1 / truncated 2(确定性:BodyId 升序首体获写)。
        w.step(DT).unwrap();
        let r1 = bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(1, 0, 0));
        assert_eq!(
            (r1.bodies_seen, r1.bodies_written, r1.writes_truncated),
            (3, 1, 2),
            "额度 1:确定性截断"
        );
        assert_eq!(r1.dirty_instances, vec![inst[0]], "升序首体获写");
        assert_eq!(bridge.writes_saturated_total(), 2);

        // 帧 2:新预算(每帧重置,§4.A6)足额 → 续写全部 active 体。
        w.step(DT).unwrap();
        let r2 = bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(3, 0, 0));
        assert_eq!(
            (r2.bodies_seen, r2.bodies_written, r2.writes_truncated),
            (3, 3, 0),
            "新预算续写剩余"
        );
        assert_eq!(r2.dirty_instances, inst);
        assert_eq!(
            bridge.writes_saturated_total(),
            2,
            "饱和计数单调累计,不随预算重置清零"
        );

        // 帧 3:额度 0 → 全截断,累计 2 + 3。
        w.step(DT).unwrap();
        let r3 = bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(0, 0, 0));
        assert_eq!((r3.bodies_written, r3.writes_truncated), (0, 3));
        assert!(r3.dirty_instances.is_empty());
        assert!(r3.dirty_ranges.is_empty(), "零写帧 dirty 为空");
        assert_eq!(bridge.writes_saturated_total(), 5);

        vec![r1, r2, r3]
    }

    let run_a = run_once();
    let run_b = run_once();
    assert_eq!(run_a, run_b, "同输入同输出(确定性截断重放一致)");
}

// ---------------------------------------------------------------------------
// (4) dirty_ranges 与 dirty_instances 同帧同源(§4.B2/§4.B5)
// ---------------------------------------------------------------------------

#[test]
fn flush_dirty_ranges_match_dirty_instances() {
    let mut w = PhysicsWorld::new(world_desc(None)).unwrap();
    // 静态/动态分批插入(诚实边界:add_bodies_batch 对混 broadphase 层批存在
    // Jolt prepare 重排 ioBodies 引发的 id↔desc 错位缺陷——G6.3 测试期发现,
    // 已上报待 G6.4 前修复;本测试断言面(桥写脏集)与该缺陷无关,分两批规避)。
    let statics = w
        .add_bodies_batch(&[
            static_box(0.0, -0.5, 0.0, 0.5),  // inst0 静态
            static_box(10.0, -0.5, 0.0, 0.5), // inst3 静态
            static_box(20.0, -0.5, 0.0, 0.5), // inst5 静态
        ])
        .unwrap();
    // 动态体远置自由落体(无接触,1 步内必 active)。
    let dyns = w
        .add_bodies_batch(&[
            dyn_sphere(50.0, 50.0, 0.0), // inst1 动态
            dyn_sphere(60.0, 55.0, 0.0), // inst2 动态
            dyn_sphere(70.0, 60.0, 0.0), // inst4 动态
        ])
        .unwrap();
    let (mut scene, inst) = fresh_scene(6);
    let mut bridge = PhysicsBridge::new();
    bridge.register(statics[0], inst[0], BodyKind::Static);
    bridge.register(dyns[0], inst[1], BodyKind::Dynamic);
    bridge.register(dyns[1], inst[2], BodyKind::Dynamic);
    bridge.register(statics[1], inst[3], BodyKind::Static);
    bridge.register(dyns[2], inst[4], BodyKind::Dynamic);
    bridge.register(statics[2], inst[5], BodyKind::Static);

    w.step(DT).unwrap();
    let report = bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(16, 0, 0));

    // 本测试无桥外写者(setup 已 flush),两集合恰相等。
    assert_eq!(report.bodies_written, 3);
    assert_eq!(report.dirty_instances, vec![inst[1], inst[2], inst[4]]);
    assert_eq!(
        report.dirty_ranges,
        vec![
            DirtyRange {
                start: inst[1],
                end: inst[2] + 1,
            },
            DirtyRange {
                start: inst[4],
                end: inst[4] + 1,
            },
        ],
        "相邻合并:{{1,2}} → [1,3),{{4}} → [4,5)"
    );
    assert_eq!(
        covered_instances(&report.dirty_ranges),
        report
            .dirty_instances
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
        "dirty_ranges 覆盖实例集合 == dirty_instances 集合(同帧同源,§4.B5)"
    );
    // dirty_instances 升序(AS 脏信号消费序)。
    let mut sorted = report.dirty_instances.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, report.dirty_instances, "dirty_instances 升序");
}

// ---------------------------------------------------------------------------
// (5) motion hint 差分(prev/cur == 实际位移;同帧睡眠/静态体无 hint,§4.B3)
// ---------------------------------------------------------------------------

#[test]
fn motion_hint_tracks_prev_cur() {
    let mut w = PhysicsWorld::new(world_desc(None)).unwrap();
    let ground = w.add_bodies_batch(&[ground_desc()]).unwrap()[0];
    let rest = w
        .add_bodies_batch(&[dyn_box(0.0, 0.452, 0.0, 0.45)])
        .unwrap()[0];
    let (mut scene, inst) = fresh_scene(3);
    let mut bridge = PhysicsBridge::new();
    bridge.register(ground, inst[0], BodyKind::Static);
    bridge.register(rest, inst[1], BodyKind::Dynamic);
    // 静置箱入睡(静态/睡眠体同帧锚定)。
    assert!(
        settle_until_asleep(&mut w, &mut bridge, &mut scene, rest),
        "静置箱应在 400 步内入睡"
    );

    // 落体进场(避开睡眠箱上方,自由落体两帧)。
    let ball = w.add_bodies_batch(&[dyn_sphere(5.0, 10.0, 0.0)]).unwrap()[0];
    bridge.register(ball, inst[2], BodyKind::Dynamic);

    // 帧 A:首写 → hint prev == cur(零位移基线,不产生假 MV)。
    w.step(DT).unwrap();
    let t0 = w.body_transform(ball).unwrap();
    let ra = bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(16, 0, 0));
    assert_eq!(ra.bodies_seen, 1, "同帧睡眠/静态体不进入 active 快照");
    assert_eq!(ra.bodies_written, 1);
    let ha = bridge.motion_hints();
    assert_eq!(ha.len(), 1);
    assert_eq!(ha[0].instance, inst[2]);
    assert_eq!(
        ha[0].prev_transform, ha[0].cur_transform,
        "首写帧零位移基线(逐位)"
    );

    // 帧 B:hint.prev/cur 差 == 实际位移(方向 + 量级)。
    w.step(DT).unwrap();
    let t1 = w.body_transform(ball).unwrap();
    let rb = bridge.sync_frame(&w, &mut scene, &mut SyncBudget::new(16, 0, 0));
    assert_eq!(rb.bodies_written, 1);
    let hb = bridge.motion_hints();
    assert_eq!(hb.len(), 1, "同一帧内睡眠/静态体无 hint");
    assert_eq!(hb[0].instance, inst[2]);
    assert!(
        approx34(&hb[0].prev_transform, &compose_transform_3x4(&t0)),
        "hint.prev == 帧 A 写入变换"
    );
    assert!(
        approx34(&hb[0].cur_transform, &compose_transform_3x4(&t1)),
        "hint.cur == 帧 B 写入变换"
    );
    let dy = t1.translation[1] - t0.translation[1];
    assert!(dy < 0.0, "落体位移方向向下:dy = {dy}");
    assert!(
        dy.abs() > 1e-4 && dy.abs() < 0.2,
        "单帧自由落体量级带内:dy = {dy}"
    );
    let hint_dy = hb[0].cur_transform[1][3] - hb[0].prev_transform[1][3];
    assert!((hint_dy - dy).abs() <= 1e-6, "hint 位移 == 实际位移(容差)");
}

// ---------------------------------------------------------------------------
// (6) 流送批插/批移除 + RemovalReceipt(§4.B4)
// ---------------------------------------------------------------------------

#[test]
fn streaming_insert_on_residency_and_remove_receipt() {
    let mut w = PhysicsWorld::new(world_desc(None)).unwrap();
    let mut streaming = StreamingBridge::new();
    let page = PageKey {
        resource: 42,
        page: 7,
    };
    let descs = [
        static_box(0.0, 0.0, 0.0, 0.5),
        static_box(2.0, 0.0, 0.0, 0.5),
        static_box(4.0, 0.0, 0.0, 0.5),
    ];

    // 页驻留 → 批插;返回 id 序与 descs 一一对应。
    let ids = streaming.insert_page(&mut w, page, &descs).unwrap();
    assert_eq!(ids.len(), 3);
    assert_eq!(streaming.watched_count(), 1);
    assert_eq!(streaming.bodies_of(page), Some(ids.as_slice()));
    for (i, &id) in ids.iter().enumerate() {
        assert_eq!(streaming.page_of(id), Some(page));
        let t = w.body_transform(id).unwrap();
        assert!(
            (t.translation[0] - 2.0 * i as f32).abs() <= 1e-6,
            "id 序对应 descs 序(x = 2·i)"
        );
    }

    // 重复驻留同页 → 确定性 Err(InvalidDesc),原映射不动。
    assert!(
        matches!(
            streaming.insert_page(&mut w, page, &descs),
            Err(PhysicsError::InvalidDesc(_))
        ),
        "重复 insert_page 同页 → Err(InvalidDesc)"
    );
    assert_eq!(streaming.bodies_of(page).map(<[_]>::len), Some(3));
    assert_eq!(streaming.watched_count(), 1);

    // 页卸载 → 先卸 body,receipt 与页绑定。
    let receipt = streaming.remove_page(&mut w, page).unwrap();
    assert_eq!(receipt.page(), page);
    assert_eq!(receipt.removed_bodies(), ids.as_slice());
    assert_eq!(streaming.watched_count(), 0);
    assert!(streaming.bodies_of(page).is_none());
    for &id in &ids {
        assert!(streaming.page_of(id).is_none());
        assert_eq!(
            w.body_transform(id),
            Err(PhysicsError::InvalidBody(id)),
            "移除后 body 全部失效"
        );
    }

    // 未知页卸载 → 确定性 Err(InvalidDesc)。
    assert!(matches!(
        streaming.remove_page(
            &mut w,
            PageKey {
                resource: 9,
                page: 9,
            }
        ),
        Err(PhysicsError::InvalidDesc(_))
    ));

    // receipt 移动语义单次消耗(编译期:无 pub ctor/不可 Clone);按值 drop。
    drop(receipt);
}

// ---------------------------------------------------------------------------
// (7) R-G6-4 竞态注入:驻留 → 批插 → 卸载并发物理写 → 无悬挂(§4.B4)
// ---------------------------------------------------------------------------

/// 模拟页缓存(G6.3 流送层镜像):放页唯一路径按值消耗 receipt。
#[derive(Debug, Default)]
struct PageCache {
    released: Vec<PageKey>,
}

impl PageCache {
    /// 放页(§4.B4 先卸 body 再放页):`RemovalReceipt` 按值消耗 = 单次放页权;
    /// 无 receipt 的放页路径编译期不可构造(receipt 无 pub 构造器、不可 Clone)。
    fn release(&mut self, streaming: &StreamingBridge, receipt: RemovalReceipt) {
        // 运行时断言双保险:放页时刻页不得仍有映射 body。
        debug_assert!(
            streaming.bodies_of(receipt.page()).is_none(),
            "先卸 body 再放页违反:页 {:?} 仍有映射 body",
            receipt.page()
        );
        self.released.push(receipt.page());
    }
}

#[test]
fn unload_race_injection_no_dangling() {
    const ROUNDS: u32 = 3;

    /// 写者/流送共享态(单锁,无锁序问题;相位纪律诚实边界见文件头)。
    struct Shared {
        world: PhysicsWorld,
        streaming: StreamingBridge,
    }

    let shared = Mutex::new(Shared {
        world: PhysicsWorld::new(world_desc(None)).unwrap(),
        streaming: StreamingBridge::new(),
    });
    let bar = Barrier::new(2);
    let mut scene = GpuScene::new();
    let mesh = scene.add_mesh(0, 1, [-0.5; 3], [0.5; 3]);
    scene.flush_dirty();
    let mut bridge = PhysicsBridge::new();
    let mut cache = PageCache::default();
    let page_descs = [dyn_sphere(-1.5, 50.0, 0.0), dyn_sphere(1.5, 50.0, 0.0)];

    // Barrier 编排「写 → 卸 → 写」交替(#1 驻留批插完成 / #2 写相 1 完成 /
    // #3 卸载放页完成;两线程每轮各 3 wait,真线程注入时序脚本)。
    // 每轮报告:(写相 1, 写相 2, 页 body, 页实例)。
    let rounds = std::thread::scope(|s| {
        let writer = s.spawn(|| {
            let mut out = Vec::new();
            for round in 0..ROUNDS {
                let page = PageKey {
                    resource: 7,
                    page: round,
                };
                bar.wait(); // #1
                let ids = {
                    let sh = shared.lock().unwrap();
                    sh.streaming.bodies_of(page).unwrap().to_vec()
                };
                // 场景实例装配(渲染侧驻留动作镜像)+ 注册映射。
                let insts: Vec<u32> = page_descs
                    .iter()
                    .map(|d| scene.add_instance(mesh, compose_transform_3x4(&d.transform), 0, 0))
                    .collect();
                for (i, &id) in ids.iter().enumerate() {
                    bridge.register(id, insts[i], BodyKind::Dynamic);
                }
                // 写相 1:页 body 在场 → 全量写入。
                let write1 = {
                    let mut sh = shared.lock().unwrap();
                    sh.world.step(DT).unwrap();
                    bridge.sync_frame(&sh.world, &mut scene, &mut SyncBudget::new(16, 0, 0))
                };
                assert_eq!(write1.bodies_seen, 2, "写相 1:页 body 应 active");
                assert_eq!(write1.bodies_written, 2);
                assert_eq!(
                    write1.dirty_instances, insts,
                    "写相 1 写入实例集合 == 页实例(升序)"
                );
                assert_eq!(
                    covered_instances(&write1.dirty_ranges),
                    insts.iter().copied().collect::<HashSet<_>>(),
                    "写相 1 dirty_ranges 覆盖 == 页实例(建实例脏 + 写脏同帧合并)"
                );
                assert_eq!(bridge.motion_hints().len(), 2);
                bar.wait(); // #2
                bar.wait(); // #3
                // 写相 2:页 body 已卸 → 零写入、脏集为空(无悬挂写)。
                let write2 = {
                    let mut sh = shared.lock().unwrap();
                    sh.world.step(DT).unwrap();
                    bridge.sync_frame(&sh.world, &mut scene, &mut SyncBudget::new(16, 0, 0))
                };
                assert_eq!(write2.bodies_seen, 0, "remove_page 后页 body 不再 active");
                assert_eq!(write2.bodies_written, 0, "remove_page 后对页 body 零写入");
                assert!(write2.dirty_instances.is_empty());
                assert!(
                    write2.dirty_ranges.is_empty(),
                    "GpuScene 无悬挂引用已释放页形状的写(脏集为空)"
                );
                assert!(bridge.motion_hints().is_empty());
                // 注销映射,页生命周期闭环。
                for (&id, &inst) in ids.iter().zip(&insts) {
                    assert_eq!(bridge.unregister(id), Some(inst));
                }
                assert_eq!(bridge.tracked_count(), 0);
                out.push((write1, write2, ids, insts));
            }
            out
        });
        let streamer = s.spawn(|| {
            for round in 0..ROUNDS {
                let page = PageKey {
                    resource: 7,
                    page: round,
                };
                // 页驻留 → 批插。
                {
                    let mut guard = shared.lock().unwrap();
                    // 先整体解引用再字段拆分(MutexGuard 的 DerefMut 不支持字段拆分)。
                    let sh = &mut *guard;
                    sh.streaming
                        .insert_page(&mut sh.world, page, &page_descs)
                        .unwrap();
                }
                bar.wait(); // #1
                bar.wait(); // #2
                // 页卸载 → 先卸 body,receipt 按值消耗放页。
                {
                    let mut guard = shared.lock().unwrap();
                    let sh = &mut *guard;
                    let receipt = sh.streaming.remove_page(&mut sh.world, page).unwrap();
                    cache.release(&sh.streaming, receipt);
                }
                bar.wait(); // #3
            }
        });
        let rounds = writer.join().unwrap();
        streamer.join().unwrap();
        rounds
    });

    // receipt 先于放页:逐页恰好一次,页序与驻留序一致。
    let expected_pages: Vec<PageKey> = (0..ROUNDS)
        .map(|round| PageKey {
            resource: 7,
            page: round,
        })
        .collect();
    assert_eq!(cache.released, expected_pages);
    assert_eq!(rounds.len(), ROUNDS as usize);
    assert_eq!(bridge.tracked_count(), 0, "全部映射已注销");
    assert_eq!(
        scene.instance_count(),
        2 * ROUNDS as usize,
        "实例表只增(桥不做悬挂移除)"
    );
    let Shared { world, streaming } = shared.into_inner().unwrap();
    assert_eq!(streaming.watched_count(), 0, "全部页已卸载");
    for (_write1, _write2, ids, _insts) in &rounds {
        for &id in ids {
            assert_eq!(
                world.body_transform(id),
                Err(PhysicsError::InvalidBody(id)),
                "卸载后 body 全部失效,无悬挂 body 引用已释放页形状"
            );
            assert!(streaming.page_of(id).is_none());
        }
    }
}
