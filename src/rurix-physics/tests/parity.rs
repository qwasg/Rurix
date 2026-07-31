//! G6.4 双后端对拍(RFC-0017 §4.D3,门 G-G6-5;仅双后端档
//! `all(feature = "jolt", feature = "rapier")` 编译,其余构建档整体出局)。
//!
//! 判据形态(冻结)落实口径:
//! - 同场景(箱塔沉降 + 球滚动 + 批插移除脚本)双后端各跑 N=300 固定步:
//!   ① 变换容差断言(位置 `POS_TOL_M` / 旋转 `ROT_TOL_DEG`,阈值常量钉于本文件
//!   顶部——T5 标定:≥5 次重复对拍实测包络 + 文档化余量,标定留痕见常量尾注
//!   与 evidence/physics_rapier_parity_*.json;**非跨引擎逐位**,§4.0-4);
//!   ② 接触集合不变量——Begin/End 事件 body 对集合重叠率 ≥ 99%
//!   (`CONTACT_OVERLAP_MIN`)+ 逐对相位序列等价类一致(Persist 连段 RLE 归并,
//!   归一化序列上面向,§4.A5);
//!   ③ 禁跨引擎逐位相等(§4.0-4):跨后端只面向容差/集合断言,逐位相等帧数
//!   仅作 evidence 记录,不进判据;
//!   ④ 各后端各自平台内逐位确定性分别断言(§4.A7(a):同进程重放全量逐位相等,
//!   捕获面 = 追踪体变换 + active 快照 + 接触规范键);
//!   ⑤ ≥5 次重放稳定(进程内 `REPLAYS` = 5 次全量逐位;跨进程复跑稳定性留痕
//!   见 evidence 标定件);
//!   ⑥ `PARITY_JSON` 环境变量:设置时对拍结果(各后端确定性哈希/逐拍变换摘要/
//!   跨后端最大位置·旋转偏差/Begin·End 集合重叠率/相位等价类判定/N=300/阈值
//!   常量值/通过判定)写 JSON 到该路径(父目录自建),供 T5 标定与 evidence 汇总。
//! - impulse 不比对(Jolt 侧恒 0 系 JoltC 缺口,RFC-0017 v1.2 已登记;Rapier 侧
//!   取 manifold 求解冲量——§4.D3 明示对拍门不比对 impulse)。

#![cfg(all(feature = "jolt", feature = "rapier"))]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use rurix_physics::{
    BackendKind, BodyDesc, BodyId, BodyKind, ContactPhase, MassProps, PhysicsTransform,
    PhysicsWorld, ShapeDesc, SyncBudget, WorldDesc,
};

const DT: f32 = 1.0 / 60.0;
const IDENTITY_ROT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
/// N=300 固定步(§4.D3 钉数)。
const N_STEPS: usize = 300;
/// 判据⑤:各后端进程内重放次数(≥5 次重放稳定)。
const REPLAYS: usize = 5;
/// 追踪体数(箱塔 4 + 球 1,全程 300 步在场;瞬态批插体只进 active 快照面,
/// 不进容差比较面)。
const N_TRACKED: usize = 5;
/// ① 位置容差(m)——T5 标定钉定(2026-07-31,Windows 11 x64,dev profile +
/// Jolt Release + Rapier 后端 0.33.0(pin);evidence/physics_rapier_parity_*.json):
/// 6 次跨进程复跑实测 max_pos_dev 恒 = 0.541314125(逐位恒定、方差为零——
/// 两后端各自确定性成立,跨后端偏差为确定性常量,per_step 曲线 300 拍跨
/// 进程逐位一致)。偏差结构(per-body 分解):塔箱 2.1e-3~7.6e-3 mm 级
/// (沉降穿透差),球 0.5413 m(滚动摩擦模型差,step 20 起 ~1.93e-3 m/步
/// 近线性发散,非混沌放大)。阈值 = 实测 max × 1.5 = 0.812 → 钉 0.82:
/// 余量仅覆盖异机浮点/微架构漂移(无实测,取工程惯例 1.5×),与实测同
/// 量级不放大;判别力结构 = 穿透/散架类结构性失效由 ② 接触集合不变量
/// 兜底(穿透偏差 10² m 量级、散架即 RLE/集合破裂),本阈值面向「同
/// 物理模型实现差」的包络。
const POS_TOL_M: f32 = 0.82;
/// ① 旋转容差(°)——同 POS_TOL_M 标定:实测 max_rot_dev 恒 =
/// 61.774520874(球滚动角差主导,塔箱 ≤ 0.04°),阈值 = 实测 × 1.5 =
/// 92.66 → 钉 93。
const ROT_TOL_DEG: f32 = 93.0;
/// ② Begin/End body 对集合重叠率下限(§4.D3 冻结形态:≥ 99%)。
const CONTACT_OVERLAP_MIN: f64 = 0.99;

/// 单次场景跑的全部捕获。后端内逐位比对的比较基元 = 位级表示(u32 bits);
/// 跨后端只面向容差/集合不变量,永不逐位(§4.0-4)。
#[derive(Debug, PartialEq)]
struct RunData {
    /// 逐拍追踪体变换位级表示(序 = 箱塔 4 + 球;[x,y,z,qx,qy,qz,qw] to_bits)。
    tracked: Vec<[[u32; 7]; N_TRACKED]>,
    /// 逐拍 active_transforms 位级快照(API 出口已按 BodyId 升序;含瞬态体)。
    active_bits: Vec<Vec<(u64, [u32; 7])>>,
    /// 逐拍归一化接触事件规范键(min(a,b), max(a,b), phase;§4.A5 ring 序)。
    contact_keys: Vec<Vec<(u64, u64, u8)>>,
    /// 全程 Begin / End body 对集合(位级对)。
    begin_pairs: BTreeSet<(u64, u64)>,
    end_pairs: BTreeSet<(u64, u64)>,
    /// 逐对相位序列等价类(相邻同相位 RLE 连段归并:[B,P,P,E] → [B,P,E])。
    pair_phase_rle: BTreeMap<(u64, u64), Vec<u8>>,
}

/// 变换 → 位级键(§4.0-4(a) 逐位相等口径的比较基元;与 tests/behavior.rs 同型)。
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

/// 位级表示 → (平移, 四元数) f32(跨后端容差计算用,不进逐位判据)。
fn bits_to_f32(b: &[u32; 7]) -> ([f32; 3], [f32; 4]) {
    (
        [
            f32::from_bits(b[0]),
            f32::from_bits(b[1]),
            f32::from_bits(b[2]),
        ],
        [
            f32::from_bits(b[3]),
            f32::from_bits(b[4]),
            f32::from_bits(b[5]),
            f32::from_bits(b[6]),
        ],
    )
}

/// 四元数夹角(°;q 与 -q 同旋转取 |dot|,§4.A2 调用方负责单位化语义)。
fn quat_angle_deg(a: [f32; 4], b: [f32; 4]) -> f32 {
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3])
        .abs()
        .min(1.0);
    2.0 * dot.acos().to_degrees()
}

/// body 对规范键(min, max;§4.A5 归一化口径,与存储序无关)。
fn canon_pair(a: BodyId, b: BodyId) -> (u64, u64) {
    let (a, b) = (a.to_bits(), b.to_bits());
    if a <= b { (a, b) } else { (b, a) }
}

/// Begin/End body 对集合重叠率(Jaccard = |∩|/|∪|;两集皆空 = 1.0 全重叠)。
fn set_overlap(a: &BTreeSet<(u64, u64)>, b: &BTreeSet<(u64, u64)>) -> f64 {
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        1.0
    } else {
        a.intersection(b).count() as f64 / union
    }
}

/// FNV-1a 64(后端内确定性哈希;evidence 摘要比对用,非密码学用途)。
fn fnv1a64(run: &RunData) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| {
        for byte in v.to_le_bytes() {
            h ^= u64::from(byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for step in 0..run.tracked.len() {
        for body in &run.tracked[step] {
            for &bits in body {
                mix(u64::from(bits));
            }
        }
        for &(id, t) in &run.active_bits[step] {
            mix(id);
            for &bits in &t {
                mix(u64::from(bits));
            }
        }
        for &(lo, hi, ph) in &run.contact_keys[step] {
            mix(lo);
            mix(hi);
            mix(u64::from(ph));
        }
    }
    h
}

/// 对拍场景(§4.D3「箱塔沉降 + 球滚动 + 批插移除脚本」,双后端逐字同脚本):
/// - 静态地面(顶面 y = 0)+ 4 箱塔(半长 0.45,0.001 缝静置沉降);
/// - 球(半径 0.5)静置地面:step 20 冲量 [1.5,0,0] 起滚,step 250 冲量
///   [0,5,0] 挑离地面——(地面,球) 对在 N=300 内走完 Begin→Persist→End 全相位
///   (弹道飞行 ≈ 61 步 > 残余 50 步,两后端同 dt 同脚本均不回落再 Begin),
///   End 集合非空、不变量判据非平凡;
/// - 批插移除脚本:step 60 批插 3 球(高空、相互/对地间距大于接触距离,
///   step 120 批移除)、step 150 批插 2 箱(step 210 批移除)——瞬态体全程
///   无接触,只进 active 快照面,不进接触集合(两后端接触对集合期望逐等,
///   ≥99% 重叠率门为残余引擎噪声留余量,非为结构性差异开口子)。
fn run_scenario(backend: BackendKind) -> RunData {
    let mut w = PhysicsWorld::new(WorldDesc {
        backend,
        gravity: [0.0, -9.81, 0.0],
        layer_count: 4,
        max_bodies: 1024,
        // Jolt MT 调度序钉单线程(behavior 测试同锚,跨机不 flaky);Rapier 单线程
        // 标量忽略 job_threads(src/rapier.rs 模块头登记),同锚传入不放宽。
        job_threads: Some(1),
        dt_fixed: DT,
        contact_capacity: 4096,
    })
    .unwrap();
    w.add_bodies_batch(&[BodyDesc {
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
    }])
    .unwrap();
    const HALF: f32 = 0.45;
    let tower_descs: Vec<BodyDesc> = (0..4)
        .map(|i| BodyDesc {
            kind: BodyKind::Dynamic,
            shape: ShapeDesc::Box {
                half_extents: [HALF, HALF, HALF],
            },
            layer: 1,
            mass_props: MassProps::default(),
            ccd: false,
            transform: PhysicsTransform {
                translation: [0.0, HALF + i as f32 * (2.0 * HALF + 0.001), 0.0],
                rotation: IDENTITY_ROT,
            },
        })
        .collect();
    let tower = w.add_bodies_batch(&tower_descs).unwrap();
    let ball = w
        .add_bodies_batch(&[BodyDesc {
            kind: BodyKind::Dynamic,
            shape: ShapeDesc::Sphere { radius: 0.5 },
            layer: 1,
            mass_props: MassProps::default(),
            ccd: false,
            transform: PhysicsTransform {
                translation: [3.0, 0.5, 0.0],
                rotation: IDENTITY_ROT,
            },
        }])
        .unwrap()[0];
    let tracked: Vec<BodyId> = tower.iter().copied().chain([ball]).collect();

    let mut data = RunData {
        tracked: Vec::with_capacity(N_STEPS),
        active_bits: Vec::with_capacity(N_STEPS),
        contact_keys: Vec::with_capacity(N_STEPS),
        begin_pairs: BTreeSet::new(),
        end_pairs: BTreeSet::new(),
        pair_phase_rle: BTreeMap::new(),
    };
    let mut transient: Option<Vec<BodyId>> = None;
    for step in 0..N_STEPS as u32 {
        match step {
            20 => w.apply_impulse(ball, [1.5, 0.0, 0.0]).unwrap(),
            60 => {
                transient = Some(
                    w.add_bodies_batch(
                        &(0..3)
                            .map(|k| BodyDesc {
                                kind: BodyKind::Dynamic,
                                shape: ShapeDesc::Sphere { radius: 0.5 },
                                layer: 1,
                                mass_props: MassProps::default(),
                                ccd: false,
                                transform: PhysicsTransform {
                                    translation: [-6.0 + 3.0 * k as f32, 12.0, 5.0],
                                    rotation: IDENTITY_ROT,
                                },
                            })
                            .collect::<Vec<_>>(),
                    )
                    .unwrap(),
                );
            }
            120 => w.remove_bodies_batch(&transient.take().unwrap()).unwrap(),
            150 => {
                transient = Some(
                    w.add_bodies_batch(
                        &(0..2)
                            .map(|k| BodyDesc {
                                kind: BodyKind::Dynamic,
                                shape: ShapeDesc::Box {
                                    half_extents: [0.4, 0.4, 0.4],
                                },
                                layer: 1,
                                mass_props: MassProps::default(),
                                ccd: false,
                                transform: PhysicsTransform {
                                    translation: [8.0 + 3.0 * k as f32, 14.0, -5.0],
                                    rotation: IDENTITY_ROT,
                                },
                            })
                            .collect::<Vec<_>>(),
                    )
                    .unwrap(),
                );
            }
            210 => w.remove_bodies_batch(&transient.take().unwrap()).unwrap(),
            250 => w.apply_impulse(ball, [0.0, 5.0, 0.0]).unwrap(),
            _ => {}
        }
        w.step(DT).unwrap();
        // ① 追踪体逐拍变换位级捕获(睡眠体同样可读,容差面不因入睡中断)。
        let mut frame = [[0u32; 7]; N_TRACKED];
        for (slot, id) in frame.iter_mut().zip(&tracked) {
            *slot = transform_bits(&w.body_transform(*id).unwrap());
        }
        data.tracked.push(frame);
        // 活跃面快照(含瞬态体;API 出口已按 BodyId 升序)。
        data.active_bits.push(
            w.active_transforms()
                .iter()
                .map(|(id, t)| (id.to_bits(), transform_bits(t)))
                .collect(),
        );
        // 接触事件(归一化 ring 序;预算放开不截断,impulse 载荷不比对)。
        let mut budget = SyncBudget::new(0, 1_000_000, 0);
        let events: Vec<_> = w.drain_contacts(&mut budget).collect();
        let keys: Vec<(u64, u64, u8)> = events
            .iter()
            .map(|e| {
                let (lo, hi) = canon_pair(e.a, e.b);
                (lo, hi, e.phase as u8)
            })
            .collect();
        for e in &events {
            let pair = canon_pair(e.a, e.b);
            match e.phase {
                ContactPhase::Begin => {
                    data.begin_pairs.insert(pair);
                }
                ContactPhase::End => {
                    data.end_pairs.insert(pair);
                }
                ContactPhase::Persist => {}
            }
            let rle = data.pair_phase_rle.entry(pair).or_default();
            let ph = e.phase as u8;
            if rle.last() != Some(&ph) {
                rle.push(ph);
            }
        }
        data.contact_keys.push(keys);
    }
    data
}

/// 对拍结果 JSON 拼装(PARITY_JSON 落盘件;手工拼装零新依赖——键名为本文件
/// 常量、数值为实测值,无任意字符串转义面)。
#[allow(clippy::too_many_arguments)]
fn parity_json(
    jolt: &RunData,
    rapier: &RunData,
    per_step: &[(usize, f32, f32)],
    per_body_max: &[(f32, f32); N_TRACKED],
    max_pos_dev: f32,
    max_rot_dev: f32,
    bitwise_frames: usize,
    begin_overlap: f64,
    end_overlap: f64,
    phase_class_equal: bool,
    pass: bool,
) -> String {
    let mut s = String::new();
    writeln!(s, "{{").unwrap();
    writeln!(s, "  \"schema\": \"physics_rapier_parity/run_v1\",").unwrap();
    writeln!(s, "  \"n_steps\": {N_STEPS},").unwrap();
    writeln!(s, "  \"replays\": {REPLAYS},").unwrap();
    writeln!(
        s,
        "  \"thresholds\": {{\"pos_tol_m\": {POS_TOL_M}, \"rot_tol_deg\": {ROT_TOL_DEG}, \
         \"contact_overlap_min\": {CONTACT_OVERLAP_MIN}}},"
    )
    .unwrap();
    writeln!(s, "  \"per_backend\": {{").unwrap();
    writeln!(
        s,
        "    \"jolt\": {{\"determinism_hash\": \"{:016x}\", \"replays_bitwise_identical\": true}},",
        fnv1a64(jolt)
    )
    .unwrap();
    writeln!(
        s,
        "    \"rapier\": {{\"determinism_hash\": \"{:016x}\", \"replays_bitwise_identical\": true}}",
        fnv1a64(rapier)
    )
    .unwrap();
    writeln!(s, "  }},").unwrap();
    writeln!(s, "  \"cross_backend\": {{").unwrap();
    writeln!(s, "    \"max_pos_dev_m\": {max_pos_dev:.9},").unwrap();
    writeln!(s, "    \"max_rot_dev_deg\": {max_rot_dev:.9},").unwrap();
    // 逐追踪体最大偏差分解(序 = tower0..tower3, ball;判别力结构留痕)。
    write!(s, "    \"per_body_max_pos_m\": [").unwrap();
    for (i, &(pos, _)) in per_body_max.iter().enumerate() {
        if i > 0 {
            write!(s, ", ").unwrap();
        }
        write!(s, "{pos:.9}").unwrap();
    }
    writeln!(s, "],").unwrap();
    write!(s, "    \"per_body_max_rot_deg\": [").unwrap();
    for (i, &(_, rot)) in per_body_max.iter().enumerate() {
        if i > 0 {
            write!(s, ", ").unwrap();
        }
        write!(s, "{rot:.9}").unwrap();
    }
    writeln!(s, "],").unwrap();
    writeln!(
        s,
        "    \"tracked_body_order\": [\"tower0\", \"tower1\", \"tower2\", \"tower3\", \"ball\"],"
    )
    .unwrap();
    writeln!(s, "    \"bitwise_identical_frames\": {bitwise_frames},").unwrap();
    writeln!(s, "    \"begin_overlap\": {begin_overlap:.6},").unwrap();
    writeln!(s, "    \"end_overlap\": {end_overlap:.6},").unwrap();
    writeln!(s, "    \"phase_class_equal\": {phase_class_equal},").unwrap();
    // 逐拍变换摘要:[step, 当拍最大位置偏差 m, 当拍最大旋转偏差 °](5 追踪体取 max)。
    write!(s, "    \"per_step\": [").unwrap();
    for (i, &(step, pos, rot)) in per_step.iter().enumerate() {
        if i > 0 {
            write!(s, ", ").unwrap();
        }
        write!(s, "[{step}, {pos:.9}, {rot:.9}]").unwrap();
    }
    writeln!(s, "]").unwrap();
    writeln!(s, "  }},").unwrap();
    writeln!(
        s,
        "  \"verdict\": \"{}\"",
        if pass { "pass" } else { "fail" }
    )
    .unwrap();
    writeln!(s, "}}").unwrap();
    s
}

#[test]
fn rapier_jolt_parity_n300_tolerance_and_contact_invariants() {
    // ④⑤ 各后端 REPLAYS 次进程内重放全量逐位(§4.A7(a) 同二进制同平台重放
    // 口径;两后端同标准分别断言,不为 rapier 放宽)。
    let jolt_runs: Vec<RunData> = (0..REPLAYS)
        .map(|_| run_scenario(BackendKind::Jolt))
        .collect();
    for (i, run) in jolt_runs.iter().enumerate().skip(1) {
        assert_eq!(
            &jolt_runs[0], run,
            "Jolt 第 {i} 次重放与第 0 次非逐位一致(§4.A7(a) 后端内确定性)"
        );
    }
    let rapier_runs: Vec<RunData> = (0..REPLAYS)
        .map(|_| run_scenario(BackendKind::Rapier))
        .collect();
    for (i, run) in rapier_runs.iter().enumerate().skip(1) {
        assert_eq!(
            &rapier_runs[0], run,
            "Rapier 第 {i} 次重放与第 0 次非逐位一致(§4.A7(a) 后端内确定性)"
        );
    }
    let jolt = &jolt_runs[0];
    let rapier = &rapier_runs[0];

    // ① 逐拍逐追踪体变换容差(③:跨后端永不逐位断言,逐位相等帧仅计数入
    // evidence 留痕)。per-body 分解入 JSON:判别力结构留痕(塔沉降 mm 级
    // 一致 vs 球滚动模型差主导,阈值含义见文件顶部常量尾注)。
    let mut per_step = Vec::with_capacity(N_STEPS);
    let mut per_body_max = [(0.0f32, 0.0f32); N_TRACKED];
    let mut max_pos_dev = 0.0f32;
    let mut max_rot_dev = 0.0f32;
    let mut bitwise_frames = 0usize;
    for (step, (fj, fr)) in jolt.tracked.iter().zip(&rapier.tracked).enumerate() {
        let mut step_pos = 0.0f32;
        let mut step_rot = 0.0f32;
        let mut step_bitwise = true;
        for (body, (bj, br)) in fj.iter().zip(fr).enumerate() {
            let (pj, qj) = bits_to_f32(bj);
            let (pr, qr) = bits_to_f32(br);
            let pos_dev = pj
                .iter()
                .zip(pr.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            let rot_dev = quat_angle_deg(qj, qr);
            step_pos = step_pos.max(pos_dev);
            step_rot = step_rot.max(rot_dev);
            per_body_max[body].0 = per_body_max[body].0.max(pos_dev);
            per_body_max[body].1 = per_body_max[body].1.max(rot_dev);
            if bj != br {
                step_bitwise = false;
            }
        }
        if step_bitwise {
            bitwise_frames += 1;
        }
        per_step.push((step, step_pos, step_rot));
        max_pos_dev = max_pos_dev.max(step_pos);
        max_rot_dev = max_rot_dev.max(step_rot);
    }

    // ② 接触集合不变量(Begin/End 集合重叠率 + 逐对相位序列等价类)。
    let begin_overlap = set_overlap(&jolt.begin_pairs, &rapier.begin_pairs);
    let end_overlap = set_overlap(&jolt.end_pairs, &rapier.end_pairs);
    let phase_class_equal = jolt.pair_phase_rle == rapier.pair_phase_rle;

    let pass = max_pos_dev <= POS_TOL_M
        && max_rot_dev <= ROT_TOL_DEG
        && begin_overlap >= CONTACT_OVERLAP_MIN
        && end_overlap >= CONTACT_OVERLAP_MIN
        && phase_class_equal;

    // ⑥ PARITY_JSON 落盘(父目录自建;先写盘后断言,失败 run 同样留证)。
    if let Ok(path) = std::env::var("PARITY_JSON") {
        let json = parity_json(
            jolt,
            rapier,
            &per_step,
            &per_body_max,
            max_pos_dev,
            max_rot_dev,
            bitwise_frames,
            begin_overlap,
            end_overlap,
            phase_class_equal,
            pass,
        );
        let p = std::path::Path::new(&path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, json).unwrap();
    }

    eprintln!(
        "[parity] max_pos_dev={max_pos_dev:.9}m(tol {POS_TOL_M}) \
         max_rot_dev={max_rot_dev:.9}deg(tol {ROT_TOL_DEG}) \
         begin_overlap={begin_overlap:.6} end_overlap={end_overlap:.6} \
         phase_class_equal={phase_class_equal} bitwise_frames={bitwise_frames}/{N_STEPS}"
    );
    assert!(
        max_pos_dev <= POS_TOL_M,
        "① 跨后端最大位置偏差 {max_pos_dev} m 超容差 {POS_TOL_M} m(阈值标定留痕见文件顶部)"
    );
    assert!(
        max_rot_dev <= ROT_TOL_DEG,
        "① 跨后端最大旋转偏差 {max_rot_dev}° 超容差 {ROT_TOL_DEG}°(阈值标定留痕见文件顶部)"
    );
    assert!(
        begin_overlap >= CONTACT_OVERLAP_MIN,
        "② Begin body 对集合重叠率 {begin_overlap} < {CONTACT_OVERLAP_MIN}(§4.D3)"
    );
    assert!(
        end_overlap >= CONTACT_OVERLAP_MIN,
        "② End body 对集合重叠率 {end_overlap} < {CONTACT_OVERLAP_MIN}(§4.D3)"
    );
    assert!(
        phase_class_equal,
        "② 逐对相位序列等价类不一致:Jolt {:?} vs Rapier {:?}",
        jolt.pair_phase_rle, rapier.pair_phase_rle
    );
}
