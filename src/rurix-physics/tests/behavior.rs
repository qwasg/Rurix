//! A7 行为测试(RFC-0017 §4.A7 host 单测锚 + §4.A4 query 与 step 并发机验判据;
//! 真 Jolt 后端,default(= jolt)档运行;`--no-default-features` 档本文件整体
//! cfg 出局,后端无关 API 锚定见 tests/api.rs)。
//!
//! 测试名关键字对齐 ci/physics_core_smoke.py(步骤 88)§4.A7 清单:determin /
//! stack|settl / sleep|wake / batch / concurren / contact|drain / budget|saturat。
//!
//! 并发纪律说明(诚实边界):safe API 相位门 = step/add/remove/drain 取 `&mut self`、
//! cast 查询与快照读取 `&self`(§4.A4 Q-B)——「step 相位内直读世界」类型面不暴露,
//! 故并发烟测的机验形态 = 交替期(&self 相位)真多线程并发读一致 + 互斥交替下主步
//! 延迟有界;不存在也不允许绕过借用规则的相位内并发读路径。

#![cfg(feature = "jolt")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

use rurix_physics::{
    BodyDesc, BodyKind, ContactEvent, ContactPhase, MassProps, PhysicsTransform, PhysicsWorld,
    QueryRay, QueryShape, ShapeDesc, SyncBudget, WorldDesc,
};

const DT: f32 = 1.0 / 60.0;
const ALL_LAYERS: u64 = u64::MAX;
const IDENTITY_ROT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
/// C-6 批插判据下限:1 帧 @ 60Hz(阈值 = max(基线 mean+3σ, 此下限),见测试 4)。
const ONE_FRAME: Duration = Duration::from_micros(16_667);

fn world_desc(job_threads: Option<u32>, contact_capacity: u32) -> WorldDesc {
    WorldDesc {
        gravity: [0.0, -9.81, 0.0],
        layer_count: 4,
        max_bodies: 1024,
        job_threads,
        dt_fixed: DT,
        contact_capacity,
        ..Default::default()
    }
}

/// 地面:静态箱,顶面 y = 0(半高 0.5,中心 y = -0.5)。
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

/// 动态球(半径 0.5,球心 (x, y, z))。
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

/// 变换 → 位级键(§4.0-4(a) 逐位相等口径的比较基元)。
fn transform_bits(t: &PhysicsTransform) -> [u32; 7] {
    let p = t.translation;
    let r = t.rotation;
    [
        p[0].to_bits(),
        p[1].to_bits(),
        p[2].to_bits(),
        r[0].to_bits(),
        r[1].to_bits(),
        r[2].to_bits(),
        r[3].to_bits(),
    ]
}

/// 当前 active 变换快照的位级表示(active_transforms 返回已按 BodyId 升序)。
fn snapshot_bits(w: &PhysicsWorld) -> Vec<(u64, [u32; 7])> {
    w.active_transforms()
        .iter()
        .map(|(id, t)| (id.to_bits(), transform_bits(t)))
        .collect()
}

/// 接触事件规范序键(§4.A5:(min(a,b), max(a,b), phase))。
fn canon_key(e: &ContactEvent) -> (u64, u64, u8) {
    let (lo, hi) = if e.a <= e.b { (e.a, e.b) } else { (e.b, e.a) };
    (lo.to_bits(), hi.to_bits(), e.phase as u8)
}

fn mean_sigma(samples: &[Duration]) -> (f64, f64) {
    let n = samples.len() as f64;
    let mean = samples.iter().map(|d| d.as_secs_f64()).sum::<f64>() / n;
    let var = samples
        .iter()
        .map(|d| (d.as_secs_f64() - mean).powi(2))
        .sum::<f64>()
        / n;
    (mean, var.sqrt())
}

// ---------------------------------------------------------------------------
// (1) 固定步确定性(§4.A7;§4.0-4(a) 默认口径:同二进制同平台重放逐位一致)
// ---------------------------------------------------------------------------

/// 确定性脚本:箱塔(3) + 球 + 冲量脚本(§4.A7「同输入序列:初始变换 + 外力脚本」)。
/// 返回逐步 active_transforms 位级快照(N=100 帧)。
fn run_determinism_replay(job_threads: Option<u32>) -> Vec<Vec<(u64, [u32; 7])>> {
    let mut w = PhysicsWorld::new(world_desc(job_threads, 4096)).unwrap();
    w.add_bodies_batch(&[ground_desc()]).unwrap();
    let tower: Vec<BodyDesc> = (0..3)
        .map(|i| dyn_box(0.0, 0.45 + i as f32 * 0.901, 0.0, 0.45))
        .collect();
    let boxes = w.add_bodies_batch(&tower).unwrap();
    let ball = w.add_bodies_batch(&[dyn_sphere(3.0, 2.0, 0.0)]).unwrap()[0];
    let mut frames = Vec::new();
    for step in 0..100u32 {
        // 外力脚本(含对睡眠体的唤醒冲量):重放两次位级一致。
        if step == 10 {
            w.apply_impulse(ball, [0.0, 4.0, 0.0]).unwrap();
        }
        if step == 40 {
            w.apply_impulse(boxes[2], [1.5, 0.0, 3.0]).unwrap();
        }
        w.step(DT).unwrap();
        frames.push(snapshot_bits(&w));
    }
    frames
}

#[test]
fn fixed_step_determinism_replay_100_steps_bitwise() {
    // job 线程口径(诚实登记,本切片集成轮探针实测 2026-07-31,Windows 11 x64,
    // dev profile + Jolt Release):同机实测 MT job 池(job_threads=None 硬件并行度
    // /Some(2)/Some(4))在 72 动态体大场景(3×8 箱塔 + 48 球 + 冲量脚本)下 100 步
    // 重放**亦逐位一致**(探针 4 轮全同),即本机未观测到 MT 引入的逐位偏差;但
    // vendor 构建 CROSS_PLATFORM_DETERMINISTIC=OFF(VENDOR.md §2),上游在该配置下
    // 不承诺 MT 调度序逐位稳定(异机/异核数/高负载下可能漂移)——故门测试以
    // job_threads=Some(1) 钉住单线程 job,保证 §4.0-4(a)「同二进制同平台重放逐位
    // 一致」判据跨机不 flaky;MT 逐位确定性不作为冻结承诺(跨平台 bit 级选项 (b)
    // 同样未启用,后续波次按需启用并写 evidence)。
    let run_a = run_determinism_replay(Some(1));
    let run_b = run_determinism_replay(Some(1));
    assert_eq!(run_a.len(), 100, "N=100 固定步");
    for (step, (fa, fb)) in run_a.iter().zip(run_b.iter()).enumerate() {
        assert_eq!(fa, fb, "第 {step} 步 active_transforms 全量逐位不一致");
    }
    // 场景锚定有效:全程存在 active 体(快照非全空)。
    assert!(
        run_a.iter().any(|f| !f.is_empty()),
        "确定性脚本应存在活动体"
    );
}

// ---------------------------------------------------------------------------
// (2) 堆叠沉降(§4.A7:≥5 箱塔静置收敛;末速 ≈ 0 / 末变换稳定,容差断言)
// ---------------------------------------------------------------------------

#[test]
fn box_tower_stack_settling_converges() {
    let mut w = PhysicsWorld::new(world_desc(None, 4096)).unwrap();
    w.add_bodies_batch(&[ground_desc()]).unwrap();
    const N: usize = 6;
    const HALF: f32 = 0.45;
    let descs: Vec<BodyDesc> = (0..N)
        .map(|i| dyn_box(0.0, HALF + i as f32 * (2.0 * HALF + 0.001), 0.0, HALF))
        .collect();
    let boxes = w.add_bodies_batch(&descs).unwrap();

    // 静置至全体入睡(上限 900 步 = 15 s,实测 ~2-4 s 内入睡)。
    let mut asleep_at = None;
    for step in 0..900u32 {
        w.step(DT).unwrap();
        if step >= 60 && step % 30 == 0 && boxes.iter().all(|b| !w.is_active(*b).unwrap()) {
            asleep_at = Some(step);
            break;
        }
    }
    let asleep_at = asleep_at.expect("6 箱塔应在 900 步内全体入睡");

    // 末速 ≈ 0 代理:入睡后再走 30 步,末变换逐位不变(睡眠体零脏写,§4.A3)。
    let before: Vec<PhysicsTransform> = boxes
        .iter()
        .map(|b| w.body_transform(*b).unwrap())
        .collect();
    for _ in 0..30 {
        w.step(DT).unwrap();
    }
    let after: Vec<PhysicsTransform> = boxes
        .iter()
        .map(|b| w.body_transform(*b).unwrap())
        .collect();
    for (i, (t0, t1)) in before.iter().zip(after.iter()).enumerate() {
        assert_eq!(
            transform_bits(t0),
            transform_bits(t1),
            "箱 {i} 入睡后末变换漂移(入睡于第 {asleep_at} 步)"
        );
        // 沉降位置容差:竖直叠放,不滑移、不倾倒。
        let expected_y = HALF + i as f32 * 2.0 * HALF;
        assert!(
            (t1.translation[1] - expected_y).abs() < 0.05,
            "箱 {i} 沉降高度 {} 应 ≈ {expected_y}",
            t1.translation[1]
        );
        assert!(
            t1.translation[0].abs() < 0.1 && t1.translation[2].abs() < 0.1,
            "箱 {i} 水平漂移过大(倾倒?):{:?}",
            t1.translation
        );
    }
}

// ---------------------------------------------------------------------------
// (3) 睡眠唤醒(§4.A7:静置入睡 is_active=false + apply_impulse 唤醒)
// ---------------------------------------------------------------------------

#[test]
fn sleep_then_impulse_wake() {
    let mut w = PhysicsWorld::new(world_desc(None, 4096)).unwrap();
    w.add_bodies_batch(&[ground_desc()]).unwrap();
    let b = w
        .add_bodies_batch(&[dyn_box(0.0, 0.452, 0.0, 0.45)])
        .unwrap()[0];

    let mut slept = false;
    for _ in 0..400u32 {
        w.step(DT).unwrap();
        if !w.is_active(b).unwrap() {
            slept = true;
            break;
        }
    }
    assert!(slept, "静置箱应在 400 步内入睡(is_active = false)");
    let p0 = w.body_transform(b).unwrap();

    w.apply_impulse(b, [0.0, 3.0, 0.0]).unwrap();
    w.step(DT).unwrap();
    assert!(
        w.is_active(b).unwrap(),
        "冲量应唤醒睡眠体(is_active = true)"
    );
    let p1 = w.body_transform(b).unwrap();
    assert!(
        p1.translation[1] > p0.translation[1] + 0.005,
        "唤醒后位置应变化(上升):{} -> {}",
        p0.translation[1],
        p1.translation[1]
    );
}

// ---------------------------------------------------------------------------
// (4) 批插体不锁死主步(§4.A7 C-6 量化判据:批插期间主步延迟 ≤ 1 帧)
// ---------------------------------------------------------------------------

#[test]
fn batch_insert_no_stall_main_step() {
    // 相位纪律(诚实边界):add/remove/step 均取 `&mut self`,Rust 借用规则下两线程
    // 只能互斥交替——这正是 §4.A7 C-6 的「prepare 在 step 外交替期执行、finalize 单点
    // 提交」形态:批插操作落在主步步间交替期,断言批插活跃窗口内主步单步耗时 ≤ 1 帧。
    let world = Arc::new(Mutex::new(
        PhysicsWorld::new(world_desc(None, 4096)).unwrap(),
    ));
    world
        .lock()
        .unwrap()
        .add_bodies_batch(&[ground_desc()])
        .unwrap();
    {
        let descs: Vec<BodyDesc> = (0..4)
            .map(|i| dyn_box(i as f32 * 1.2 - 2.4, 0.45, 0.0, 0.45))
            .collect();
        world.lock().unwrap().add_bodies_batch(&descs).unwrap();
    }

    // 阈值标定:批插线程启动前 90 步基线取 mean+3σ,下限 1 帧(16.667 ms)。
    // 本切片实测标定(2026-07-31,Windows 11 x64,dev profile + Jolt Release,≥5 次
    // 重复取包络):baseline mean ≈ 29~48 µs、σ ≈ 42~71 µs → mean+3σ ≈ 155~261 µs,
    // 恒低于 1 帧下限,故阈值实际 = 16.667 ms;批插活跃窗口内 max_step ≈ 1.29~1.52 ms
    // (≈ 帧预算的 8~9%),主步无锁死。
    let mut baseline = Vec::new();
    for _ in 0..90 {
        let t0 = Instant::now();
        world.lock().unwrap().step(DT).unwrap();
        baseline.push(t0.elapsed());
    }
    let (mean, sigma) = mean_sigma(&baseline);
    let threshold = (mean + 3.0 * sigma).max(ONE_FRAME.as_secs_f64());

    // 另一线程:反复批插 16 球 + 批移除(交替期互斥执行)。
    let ops = Arc::new(AtomicUsize::new(0));
    let worker = {
        let world = Arc::clone(&world);
        let ops = Arc::clone(&ops);
        std::thread::spawn(move || {
            for round in 0..25u32 {
                let descs: Vec<BodyDesc> = (0..16)
                    .map(|k| {
                        dyn_sphere(
                            (k % 4) as f32 * 1.5 - 3.0,
                            8.0 + round as f32,
                            (k / 4) as f32 * 1.5 - 3.0,
                        )
                    })
                    .collect();
                let ids = world.lock().unwrap().add_bodies_batch(&descs).unwrap();
                ops.fetch_add(1, Ordering::Relaxed);
                world.lock().unwrap().remove_bodies_batch(&ids).unwrap();
                ops.fetch_add(1, Ordering::Relaxed);
                std::thread::yield_now();
            }
        })
    };

    // 主线程:批插活跃窗口内继续固定步循环,逐步计时。
    let mut step_times = Vec::new();
    let mut iter_walls = Vec::new();
    for _ in 0..210 {
        let w0 = Instant::now();
        let t0 = Instant::now();
        world.lock().unwrap().step(DT).unwrap();
        step_times.push(t0.elapsed());
        iter_walls.push(w0.elapsed());
    }
    worker.join().unwrap();
    assert_eq!(
        ops.load(Ordering::Relaxed),
        50,
        "批插线程应完成 25 轮 × (add + remove)(真线程注入)"
    );

    let max_step = step_times
        .iter()
        .map(|d| d.as_secs_f64())
        .fold(0.0, f64::max);
    let max_wall = iter_walls
        .iter()
        .map(|d| d.as_secs_f64())
        .fold(0.0, f64::max);
    eprintln!(
        "[batch_insert_no_stall] baseline mean={mean:.6}s sigma={sigma:.6}s \
         threshold={threshold:.6}s max_step={max_step:.6}s max_iter_wall={max_wall:.6}s"
    );
    assert!(
        max_step <= threshold,
        "批插期间主步单步耗时超 1 帧:max={max_step:.6}s threshold={threshold:.6}s"
    );
    assert!(
        max_wall <= threshold,
        "批插期间主步迭代墙钟(含交替期批插等待)超 1 帧:max={max_wall:.6}s threshold={threshold:.6}s"
    );
}

// ---------------------------------------------------------------------------
// (5) query 与 step 并发烟测(§4.A4 机验判据:step 外 ≥2 线程并发 cast,
//     排序后结果与单线程一致;变换快照读与 step 完成后一致;真并发经线程注入)
// ---------------------------------------------------------------------------

#[test]
fn concurrent_query_cast_matches_single_thread() {
    let mut w = PhysicsWorld::new(world_desc(None, 4096)).unwrap();
    w.add_bodies_batch(&[ground_desc()]).unwrap();
    let tower: Vec<BodyDesc> = (0..3)
        .map(|i| dyn_box(0.0, 0.45 + i as f32 * 0.901, 0.0, 0.45))
        .collect();
    w.add_bodies_batch(&tower).unwrap();
    w.add_bodies_batch(&[dyn_sphere(2.5, 0.5, 0.0)]).unwrap();
    // 下落体:使 active_transforms 快照在查询窗口内非空(非平凡断言)。
    w.add_bodies_batch(&[dyn_sphere(0.0, 20.0, 5.0)]).unwrap();
    for _ in 0..90 {
        w.step(DT).unwrap();
    }

    let ray_down = QueryRay {
        origin: [0.0, 10.0, 0.0],
        dir: [0.0, -1.0, 0.0],
        t_min: 0.0,
        t_max: 100.0,
        layer_mask: ALL_LAYERS,
    };
    let ray_angled = QueryRay {
        origin: [8.0, 8.0, 8.0],
        dir: [-0.64, -0.56, -0.64],
        t_min: 0.0,
        t_max: 100.0,
        layer_mask: ALL_LAYERS,
    };
    let shape_cast = QueryShape {
        shape: ShapeDesc::Sphere { radius: 0.5 },
        start: PhysicsTransform {
            translation: [0.0, 6.0, 0.0],
            rotation: IDENTITY_ROT,
        },
        dir: [0.0, -1.0, 0.0],
        t_max: 20.0,
        layer_mask: ALL_LAYERS,
    };
    let overlap_shape = ShapeDesc::Box {
        half_extents: [2.5, 2.5, 2.5],
    };
    let overlap_at = PhysicsTransform {
        translation: [0.0, 1.5, 0.0],
        rotation: IDENTITY_ROT,
    };

    // 单线程参考序列(API 出口已按 (t, BodyId) / BodyId 规范序,§4.A4 C-2)。
    let mut budget = SyncBudget::new(0, 0, 1000);
    let ref_ray_down = w.cast_ray(&ray_down, &mut budget).unwrap();
    let ref_ray_angled = w.cast_ray(&ray_angled, &mut budget).unwrap();
    let ref_cast = w.cast_shape(&shape_cast, &mut budget).unwrap();
    let ref_overlap = w
        .overlap(&overlap_shape, &overlap_at, ALL_LAYERS, &mut budget)
        .unwrap();
    let ref_snapshot = w.active_transforms();
    assert!(
        !ref_ray_down.is_empty() && !ref_cast.is_empty() && !ref_overlap.is_empty(),
        "参考查询须非空(场景锚定有效)"
    );
    assert!(!ref_snapshot.is_empty(), "下落体使 active 快照非空");

    // ≥2 线程真并发:Barrier 齐射 + 在飞计数(≥2 同时在查询窗 = 真并发证据)。
    let barrier = Arc::new(Barrier::new(4));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));
    let reports: Vec<_> = std::thread::scope(|s| {
        let mut handles = Vec::new();
        for _ in 0..4 {
            let barrier = Arc::clone(&barrier);
            let in_flight = Arc::clone(&in_flight);
            let max_in_flight = Arc::clone(&max_in_flight);
            let world = &w;
            let (ray_down, ray_angled, shape_cast, overlap_shape, overlap_at) = (
                &ray_down,
                &ray_angled,
                &shape_cast,
                &overlap_shape,
                &overlap_at,
            );
            handles.push(s.spawn(move || {
                barrier.wait();
                let mut local = Vec::new();
                for _ in 0..25 {
                    let cur = in_flight.fetch_add(1, Ordering::Relaxed) + 1;
                    max_in_flight.fetch_max(cur, Ordering::Relaxed);
                    // 每线程持自己的 SyncBudget(&mut 不跨线程共享,§4.A4/A6)。
                    let mut b = SyncBudget::new(0, 0, 1000);
                    let r1 = world.cast_ray(ray_down, &mut b).unwrap();
                    let r2 = world.cast_ray(ray_angled, &mut b).unwrap();
                    let r3 = world.cast_shape(shape_cast, &mut b).unwrap();
                    let r4 = world
                        .overlap(overlap_shape, overlap_at, ALL_LAYERS, &mut b)
                        .unwrap();
                    // step 相位内直读类型面不暴露(&mut 独占)——交替期快照并发读。
                    let snap = world.active_transforms();
                    in_flight.fetch_sub(1, Ordering::Relaxed);
                    local.push((r1, r2, r3, r4, snap));
                }
                local
            }));
        }
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    for thread in &reports {
        assert_eq!(thread.len(), 25);
        for (r1, r2, r3, r4, snap) in thread {
            assert_eq!(r1, &ref_ray_down, "并发 cast_ray(down) 与单线程不一致");
            assert_eq!(r2, &ref_ray_angled, "并发 cast_ray(angled) 与单线程不一致");
            assert_eq!(r3, &ref_cast, "并发 cast_shape 与单线程不一致");
            assert_eq!(r4, &ref_overlap, "并发 overlap 与单线程不一致");
            assert_eq!(
                snap, &ref_snapshot,
                "交替期 active_transforms 并发快照读与 step 完成后读不一致"
            );
        }
    }
    assert!(
        max_in_flight.load(Ordering::Relaxed) >= 2,
        "真并发证据:查询窗内须 ≥2 线程同时在飞(实测 max={})",
        max_in_flight.load(Ordering::Relaxed)
    );
}

// ---------------------------------------------------------------------------
// (6) ContactEvent 有界 drain(§4.A5:Begin→Persist→End 相位序列;归一化序;
//     ring 容量截断丢最旧 + contacts_dropped;drain 受 budget 截断)
// ---------------------------------------------------------------------------

#[test]
fn contact_events_bounded_drain_and_overflow() {
    // —— A: Begin→Persist→End 相位序列(默认容量,逐步 drain)——
    {
        let mut w = PhysicsWorld::new(world_desc(None, 4096)).unwrap();
        let ground = w.add_bodies_batch(&[ground_desc()]).unwrap()[0];
        let ball = w.add_bodies_batch(&[dyn_sphere(0.0, 1.5, 0.0)]).unwrap()[0];
        let mut phases: Vec<ContactPhase> = Vec::new();
        let mut launched = false;
        for step in 0..600u32 {
            w.step(DT).unwrap();
            let mut b = SyncBudget::new(0, 1_000_000, 0);
            let batch: Vec<ContactEvent> = w.drain_contacts(&mut b).collect();
            // 每步批次 = 归一化序列:(min,max,phase) 规范序。
            let keys: Vec<(u64, u64, u8)> = batch.iter().map(canon_key).collect();
            let mut sorted = keys.clone();
            sorted.sort();
            assert_eq!(keys, sorted, "step {step} drain 批次须为规范序");
            for e in &batch {
                let involves = (e.a == ball && e.b == ground) || (e.a == ground && e.b == ball);
                if involves {
                    phases.push(e.phase);
                }
            }
            // 接触稳定(≥30 条)后上抛球 → 接触断开 → End。
            if !launched && phases.len() >= 30 {
                w.apply_impulse(ball, [0.0, 6.0, 0.0]).unwrap();
                launched = true;
            }
            if launched && phases.contains(&ContactPhase::End) {
                break;
            }
        }
        assert_eq!(
            phases.first(),
            Some(&ContactPhase::Begin),
            "相位序列须以 Begin 开头"
        );
        assert_eq!(
            phases.last(),
            Some(&ContactPhase::End),
            "相位序列须以 End 结尾"
        );
        assert_eq!(
            phases.iter().filter(|p| **p == ContactPhase::Begin).count(),
            1,
            "Begin 恰好一次"
        );
        assert_eq!(
            phases.iter().filter(|p| **p == ContactPhase::End).count(),
            1,
            "End 恰好一次"
        );
        assert!(
            phases.contains(&ContactPhase::Persist),
            "Begin 与 End 之间须有 Persist"
        );
    }

    // —— B: ring 溢出确定性丢最旧 + contacts_dropped 计数(小容量 + 不 drain 累积)——
    {
        let mut w = PhysicsWorld::new(world_desc(Some(1), 8)).unwrap();
        w.add_bodies_batch(&[ground_desc()]).unwrap();
        let descs: Vec<BodyDesc> = [-6.0f32, -2.0, 2.0, 6.0]
            .iter()
            .map(|&x| dyn_sphere(x, 2.0, 0.0))
            .collect();
        w.add_bodies_batch(&descs).unwrap();
        let mut emitted = 0u64;
        let mut dropped = 0u64;
        let mut contact_step = None;
        for step in 0..120u32 {
            let stats = w.step(DT).unwrap();
            emitted += u64::from(stats.contacts_emitted);
            dropped += u64::from(stats.contacts_dropped);
            if contact_step.is_none() && stats.contacts_emitted > 0 {
                contact_step = Some(step);
            }
            if let Some(cs) = contact_step
                && step >= cs + 8
            {
                break;
            }
        }
        assert!(contact_step.is_some(), "4 球应落地产生接触");
        let mut b = SyncBudget::new(0, 1_000_000, 0);
        let rest: Vec<ContactEvent> = w.drain_contacts(&mut b).collect();
        assert_eq!(rest.len(), 8, "ring 只留容量 8 条");
        assert_eq!(
            dropped,
            emitted - 8,
            "溢出确定性丢弃计数 = 入队总量 - ring 容量"
        );
        assert!(
            rest.iter().all(|e| e.phase == ContactPhase::Persist),
            "最旧事件(Begin)被确定性丢弃,残余全为 Persist"
        );
    }

    // —— C: drain_contacts 受 budget.max_contact_events 截断 + 饱和计数 ——
    {
        let mut w = PhysicsWorld::new(world_desc(None, 4096)).unwrap();
        w.add_bodies_batch(&[ground_desc()]).unwrap();
        w.add_bodies_batch(&[dyn_sphere(0.0, 1.5, 0.0)]).unwrap();
        // 落地后不 drain 累积 3 步(Begin + Persist + Persist)。
        let mut seen = 0u32;
        for _ in 0..120u32 {
            let stats = w.step(DT).unwrap();
            seen += stats.contacts_emitted;
            if seen >= 3 {
                break;
            }
        }
        assert_eq!(seen, 3, "ring 内应累积 3 条事件");
        let mut small = SyncBudget::new(0, 2, 0);
        let first: Vec<ContactEvent> = w.drain_contacts(&mut small).collect();
        assert_eq!(first.len(), 2, "drain 受 max_contact_events=2 确定性截断");
        assert_eq!(
            w.budget_saturation().contact_events,
            1,
            "截断的 1 条计入饱和计数(§4.A6)"
        );
        let mut full = SyncBudget::new(0, 100, 0);
        let rest: Vec<ContactEvent> = w.drain_contacts(&mut full).collect();
        assert_eq!(rest.len(), 1, "截断未消费部分留在 ring 不丢");
        assert_eq!(w.budget_saturation().contact_events, 1);
        // ring 序 = 归一化入队序:Begin → Persist → Persist。
        assert_eq!(first[0].phase, ContactPhase::Begin);
        assert_eq!(first[1].phase, ContactPhase::Persist);
        assert_eq!(rest[0].phase, ContactPhase::Persist);
    }
}

// ---------------------------------------------------------------------------
// (7) SyncBudget 每帧重置与饱和(default 档行为级;§4.A6)
// ---------------------------------------------------------------------------

#[test]
fn sync_budget_reset_and_query_saturation_behavior() {
    let mut w = PhysicsWorld::new(world_desc(None, 4096)).unwrap();
    w.add_bodies_batch(&[ground_desc()]).unwrap();
    w.add_bodies_batch(&[dyn_sphere(0.0, 0.5, 0.0)]).unwrap();
    let ray = QueryRay {
        origin: [0.0, 5.0, 0.0],
        dir: [0.0, -1.0, 0.0],
        t_min: 0.0,
        t_max: 50.0,
        layer_mask: ALL_LAYERS,
    };
    let shape_cast = QueryShape {
        shape: ShapeDesc::Sphere { radius: 0.5 },
        start: PhysicsTransform {
            translation: [0.0, 5.0, 0.0],
            rotation: IDENTITY_ROT,
        },
        dir: [0.0, -1.0, 0.0],
        t_max: 50.0,
        layer_mask: ALL_LAYERS,
    };

    // query 轴额度 2:两次正常命中,第三次起确定性截断(空结果,非 Err)+ 饱和计数。
    let mut b1 = SyncBudget::new(0, 0, 2);
    assert!(!w.cast_ray(&ray, &mut b1).unwrap().is_empty());
    assert!(!w.cast_ray(&ray, &mut b1).unwrap().is_empty());
    assert!(
        w.cast_ray(&ray, &mut b1).unwrap().is_empty(),
        "query 轴饱和后 cast 确定性截断为空"
    );
    assert_eq!(w.budget_saturation().query_casts, 1);
    assert!(w.cast_ray(&ray, &mut b1).unwrap().is_empty());
    assert_eq!(w.budget_saturation().query_casts, 2);
    // cast_shape / overlap 共享同一 query 轴额度。
    assert!(w.cast_shape(&shape_cast, &mut b1).unwrap().is_empty());
    assert!(
        w.overlap(
            &ShapeDesc::Box {
                half_extents: [1.0; 3]
            },
            &PhysicsTransform::IDENTITY,
            ALL_LAYERS,
            &mut b1
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(w.budget_saturation().query_casts, 4);

    // 每帧重置 = 重新构造:额度恢复,cast 恢复命中;饱和计数单调累计不清零。
    let mut b2 = SyncBudget::new(0, 0, 2);
    assert!(
        !w.cast_ray(&ray, &mut b2).unwrap().is_empty(),
        "预算重置后额度恢复"
    );
    assert_eq!(
        w.budget_saturation().query_casts,
        4,
        "饱和计数随世界单调累计,不随预算重置清零"
    );
}

// ---------------------------------------------------------------------------
// (8) 混层批插序保持(G6.2 sys 缺陷回归:Jolt AddBodiesPrepare 原地重排 ioBodies
//     —— vendor 直通、文档允许(BodyInterface.h:127「array unmodified」即承认
//     prepare 会改写)。本 sys 层单 broadphase 层(bp_get_layer ≡ 0),排序键全等:
//     批 ≤ 32 走稳定 InsertionSort → 恒等(G6.2 既有测试全绿之因);批 > 32 走
//     Hoare 划分 QuickSort,等键也成对交换 → 非恒等。故回归批取 35 体 > 32,
//     sys 层激活/登记/返回必须按 prepare 前原始序配对)
// ---------------------------------------------------------------------------

#[test]
fn mixed_layer_batch_insert_order_preserved() {
    let mut w = PhysicsWorld::new(world_desc(None, 4096)).unwrap();
    // 35 体同批(> 32 触发 QuickSort 重排):1 静态地面插在非首槽(index 3),
    // 34 动态箱悬空网格(y = 8,间距 1.5 > 2×0.45 互不接触),后续下落不碰地。
    let mut descs: Vec<BodyDesc> = (0..34)
        .map(|i| {
            dyn_box(
                (i % 7) as f32 * 1.5 - 4.5,
                8.0,
                (i / 7) as f32 * 1.5 - 3.0,
                0.45,
            )
        })
        .collect();
    descs.insert(3, ground_desc());
    let ids = w.add_bodies_batch(&descs).unwrap();
    assert_eq!(ids.len(), descs.len());

    // ① 返回 ids 与 descs 序一一对应:逐 body_transform 位级比对初始位姿。
    for (i, (id, d)) in ids.iter().zip(descs.iter()).enumerate() {
        let t = w.body_transform(*id).unwrap();
        assert_eq!(
            transform_bits(&t),
            transform_bits(&d.transform),
            "descs[{i}] 返回 id 位姿错配(批内重排未按原始序还原)"
        );
    }

    let dyn_ids: Vec<rurix_physics::BodyId> = ids
        .iter()
        .zip(descs.iter())
        .filter(|(_, d)| d.kind == BodyKind::Dynamic)
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(dyn_ids.len(), 34, "场景锚定:34 动态 + 1 静态");

    // ② 动态体全部激活:插入后即 is_active;step 后 active_transforms 收齐。
    for id in &dyn_ids {
        assert!(
            w.is_active(*id).unwrap(),
            "动态体 {id} 批插后应立即激活(DONT_ACTIVATE + 按类逐个激活)"
        );
    }
    let y0: Vec<f32> = dyn_ids
        .iter()
        .map(|id| w.body_transform(*id).unwrap().translation[1])
        .collect();
    w.step(DT).unwrap();
    let active: Vec<u64> = w
        .active_transforms()
        .iter()
        .map(|(id, _)| id.to_bits())
        .collect();
    for id in &dyn_ids {
        assert!(
            active.contains(&id.to_bits()),
            "step 后 active_transforms 应收齐动态体 {id}(kind 错配为 Static 会永久漏收)"
        );
    }

    // ③ 动态体 kind 不错配:30 步后确实下落(未激活/错配则悬停于 y0)。
    for _ in 0..30 {
        w.step(DT).unwrap();
    }
    for (id, y_init) in dyn_ids.iter().zip(y0.iter()) {
        let y = w.body_transform(*id).unwrap().translation[1];
        assert!(
            y < y_init - 0.1,
            "动态体 {id} 应下落:y0 = {y_init},y = {y}(未激活或 kind 错配则悬停)"
        );
    }
}
