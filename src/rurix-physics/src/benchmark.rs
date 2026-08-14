//! G9.6 M126 Rapier 深造对标基准 A/B 夹具(spec/physics.md RXS-0378;RFC-0024
//! §4.E2;判据逐字引 G9_ACCEPTANCE_MAP §3 M126 行,gate
//! `g9.p1.m126.rapier_benchmark_ab`)。
//!
//! 冻结纪律:
//! - **同场景同输入同 determinism 画像 A/B**:Jolt(生产默认后端)vs
//!   Rapier(feature `rapier` 快路径第二后端)在同一 canonical 大堆叠场景、
//!   同一输入 journal(同一体创建命令集 + 同一 tick 数)、同一
//!   determinism 画像(固定 dt 锁死、单线程、睡眠策略钉值、零 IO)下 A/B;
//!   输入 digest 两臂逐位相等为机核断言。
//! - **三面测量**:逐 tick world 状态摘要链 digest(确定性面)+ 接触事件
//!   计数 + 求解耗时(wall-clock,measured_local 真实采样,禁 estimated)。
//! - **determinism 画像**:各自后端同后端双跑位级一致(逐 tick hash 链 +
//!   接触计数 + 末态逐位),各自确定性成立为硬断言;跨后端差异如实记录,
//!   跨 solver 不承诺逐位(RFC-0021 §7 备选 D——只作不变量/容差对拍),差
//!   异非判据、画像记录。
//! - **基准不作 replay oracle(RED 臂独立有效)**:基准输出不得充当
//!   capture/replay 的逐位对拍 oracle——[`compare_as_replay_oracle`] 一律
//!   fail-closed typed Err;replay 对拍唯一权威 = 同 solver 同版本
//!   capture/replay 逐 tick hash(RFC-0021 §4.A1 字面不变)。
//! - **RD-044 字面不变**:「快路径被真实 workload 采用时」0-byte——基准
//!   显示 D5 真实 workload 上 measured 优势才按 RD-044 程序申请深造判档,
//!   否则维持 no-go 留档;本模块只产基准报告,不升格深造、不作验收依赖与
//!   生产默认。无 measured 数据的判档申请 = fail-closed
//!   ([`validate_rd044_application`])。
//! - **glam 迁移兼容留档**:Rapier 0.32+ glam 化 API 冲击评估与兼容层设计
//!   = [`GLAM_MIGRATION_NOTE`],不承诺 bitwise 不变。

use std::fmt;
use std::time::Instant;

use rurix_pkg::sha256::{digest, hex};

use crate::capture::canonical::{hash_canonical_state, state_from_world};
use crate::types::{
    BackendKind, BodyDesc, BodyKind, MassProps, PhysicsTransform, ShapeDesc, WorldDesc,
};
use crate::world::PhysicsWorld;

/// M126 域错误(fail-closed 单一出口;harness RED 臂锚字面)。
#[derive(Debug, Clone, PartialEq)]
pub enum BenchmarkError {
    /// Rapier 后端未编译(feature `rapier` off)——A/B 缺臂 fail-closed,
    /// 不静默退化为单臂绿。
    RapierBackendNotCompiled(String),
    /// 基准冒充 replay oracle(跨 solver 逐位对拍僭越)——RED 臂字面。
    ReplayOracleUsurpation(String),
    /// 无 measured 数据的深造判档申请——fail-closed。
    MeasuredDataMissing(String),
    /// 同后端双跑位级不一致(自身确定性破坏)。
    DeterminismViolation(String),
    /// 输入/场景非法。
    InvalidInput(String),
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RapierBackendNotCompiled(s) => write!(f, "RapierBackendNotCompiled({s})"),
            Self::ReplayOracleUsurpation(s) => write!(f, "ReplayOracleUsurpation({s})"),
            Self::MeasuredDataMissing(s) => write!(f, "MeasuredDataMissing({s})"),
            Self::DeterminismViolation(s) => write!(f, "DeterminismViolation({s})"),
            Self::InvalidInput(s) => write!(f, "InvalidInput({s})"),
        }
    }
}

impl std::error::Error for BenchmarkError {}

/// glam 迁移兼容留档(Rapier 0.32+ glam 化 API 冲击评估与兼容层设计;
/// RXS-0378 L4「不承诺 bitwise 不变」)。
pub const GLAM_MIGRATION_NOTE: &str = "Rapier 0.33 数学层 = glam 化 nalgebra 后继(Pose/Rotation/Vector = glam 基);对既有快路径封装的 API 冲击 = 收敛于 src/rapier.rs 单模块类型映射面(pose_of/transform_of:PhysicsTransform xyzw quat ↔ glam Quat 直映零重排,§4.C4 原生类型名不透出);兼容层设计 = 映射函数单点维护,公共 API 零 glam 类型透出;glam 化不承诺 bitwise 不变(跨版本浮点语义漂移如实登记,跨 solver 只作不变量/容差对拍)。";

/// RD-044 触发条件字面(registry/deferred.json RD-044;字面 0-byte 引用)。
pub const RD044_CONDITION_LITERAL: &str =
    "Rapier 深造在快路径被真实 workload 采用时(对拍门判据形态不变,阈值实测标定口径不变)";

/// canonical 大堆叠场景参数(A/B 两臂同一实例 = 同场景断言面)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanonicalStackSpec {
    /// 堆叠层数(≥ 2)。
    pub layers: u32,
    /// 箱半长(m)。
    pub box_half: f32,
    /// 层间初始缝(m;小于后端 prediction distance ⇒ 初拍即接触)。
    pub layer_gap: f32,
    /// 材质摩擦。
    pub friction: f32,
    /// 恢复系数。
    pub restitution: f32,
    /// tick 数。
    pub ticks: u64,
}

impl Default for CanonicalStackSpec {
    fn default() -> Self {
        Self {
            layers: 6,
            box_half: 0.45,
            layer_gap: 0.001,
            friction: 0.6,
            restitution: 0.0,
            ticks: 90,
        }
    }
}

impl CanonicalStackSpec {
    /// 校验(fail-closed)。
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        if self.layers < 2
            || !self.box_half.is_finite()
            || self.box_half <= 0.0
            || !self.layer_gap.is_finite()
            || self.layer_gap < 0.0
            || !self.friction.is_finite()
            || !self.restitution.is_finite()
            || self.ticks < 2
        {
            return Err(BenchmarkError::InvalidInput("canonical stack spec".into()));
        }
        Ok(())
    }

    /// canonical 输入 journal(体创建命令集文本;两臂同一实例,digest 逐位
    /// 相等 = 「同一输入 journal」机核面)。
    pub fn input_journal_text(&self) -> String {
        let mut s = String::from("create_static_ground\n");
        for l in 0..self.layers {
            s.push_str(&format!("create_dyn_box layer={l}\n"));
        }
        s.push_str(&format!("run ticks={}\n", self.ticks));
        s
    }

    /// 输入 digest(A/B 同输入锚)。
    pub fn input_digest(&self) -> String {
        let mut buf = format!(
            "layers={}:half={:08x}:gap={:08x}:fric={:08x}:rest={:08x}\n",
            self.layers,
            self.box_half.to_bits(),
            self.layer_gap.to_bits(),
            self.friction.to_bits(),
            self.restitution.to_bits()
        );
        buf.push_str(&self.input_journal_text());
        hex(&digest(buf.as_bytes()))
    }

    /// 体创建命令集(静态地面 + 逐层动态箱;两臂同一构建面)。
    pub fn body_descs(&self) -> Vec<BodyDesc> {
        let mut out = Vec::with_capacity(self.layers as usize + 1);
        out.push(BodyDesc {
            kind: BodyKind::Static,
            shape: ShapeDesc::Box {
                half_extents: [10.0, 10.0, 0.5],
            },
            layer: 0,
            mass_props: MassProps {
                mass: 1.0,
                friction: self.friction,
                restitution: self.restitution,
                allow_sleep: false,
            },
            ccd: false,
            transform: PhysicsTransform {
                translation: [0.0, 0.0, -0.5],
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
        });
        let pitch = 2.0 * self.box_half + self.layer_gap;
        for l in 0..self.layers {
            out.push(BodyDesc {
                kind: BodyKind::Dynamic,
                shape: ShapeDesc::Box {
                    half_extents: [self.box_half; 3],
                },
                layer: 0,
                mass_props: MassProps {
                    mass: 1.0,
                    friction: self.friction,
                    restitution: self.restitution,
                    allow_sleep: true,
                },
                ccd: false,
                transform: PhysicsTransform {
                    translation: [0.0, 0.0, self.box_half + pitch * l as f32],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                },
            });
        }
        out
    }

    /// determinism 画像世界 desc(两臂同一画像:固定 dt 锁死、单线程
    /// declared、睡眠策略钉值、零 IO;`job_threads` 为 Jolt 专用,Rapier
    /// 臂忽略——rapier.rs 模块头诚实登记字面不变)。
    pub fn world_desc(&self, backend: BackendKind) -> WorldDesc {
        WorldDesc {
            backend,
            gravity: [0.0, 0.0, -9.81],
            layer_count: 8,
            max_bodies: 1024,
            job_threads: Some(1),
            dt_fixed: 1.0 / 60.0,
            contact_capacity: 4096,
        }
    }
}

/// 单臂测量产出(三面:逐 tick world 状态摘要链 + 接触事件计数 + 求解耗时)。
#[derive(Debug, Clone)]
pub struct BenchmarkArmOutcome {
    /// 后端。
    pub backend: BackendKind,
    /// tick 数。
    pub ticks: u64,
    /// 输入 digest(两臂逐位相等断言面)。
    pub input_digest: String,
    /// 逐 tick world 状态摘要链 digest(确定性面)。
    pub world_digest: String,
    /// 接触事件累计数(Begin/Persist/End 归一化序列总长)。
    pub contact_events_total: u64,
    /// 逐 tick 求解耗时 ns(wall-clock 真实采样;measured_local)。
    pub step_ns: Vec<u64>,
    /// 末态逐体快照摘要(canonical 文本 digest;跨后端偏差统计面)。
    pub final_state_digest: String,
    /// 末态逐体 (translation, linvel, is_dynamic) 展平序列(跨后端逐元对拍
    /// 源;规范序 = body 位表示升序;不变量只对动态体断言)。
    pub final_states: Vec<([f32; 3], [f32; 3], bool)>,
}

impl BenchmarkArmOutcome {
    /// 求解耗时中位数 ns(排序中元;measured 统计面)。
    pub fn step_ns_median(&self) -> u64 {
        let mut v = self.step_ns.clone();
        v.sort_unstable();
        v[v.len() / 2]
    }

    /// 求解耗时最小值 ns。
    pub fn step_ns_min(&self) -> u64 {
        self.step_ns.iter().copied().min().unwrap_or(0)
    }

    /// 求解耗时总 ns。
    pub fn step_ns_total(&self) -> u64 {
        self.step_ns.iter().sum()
    }
}

/// 单臂执行(同 determinism 画像;后端未编译 → 确定性 typed Err 不静默)。
pub fn run_benchmark_arm(
    backend: BackendKind,
    spec: &CanonicalStackSpec,
) -> Result<BenchmarkArmOutcome, BenchmarkError> {
    spec.validate()?;
    let desc = spec.world_desc(backend);
    let mut world = PhysicsWorld::new(desc.clone()).map_err(|e| match e {
        crate::error::PhysicsError::BackendNotCompiled(BackendKind::Rapier) => {
            BenchmarkError::RapierBackendNotCompiled(
                "feature `rapier` 未编译——A/B 缺臂 fail-closed(不静默单臂充绿)".into(),
            )
        }
        other => BenchmarkError::InvalidInput(format!("world create: {other}")),
    })?;
    world
        .add_bodies_batch(&spec.body_descs())
        .map_err(|e| BenchmarkError::InvalidInput(format!("add bodies: {e}")))?;
    let dt = desc.dt_fixed;
    let mut chain = String::from("world\n");
    let mut contact_events_total = 0u64;
    let mut step_ns = Vec::with_capacity(spec.ticks as usize);
    for tick in 0..spec.ticks {
        let t0 = Instant::now();
        world
            .step(dt)
            .map_err(|e| BenchmarkError::InvalidInput(format!("step: {e}")))?;
        step_ns.push(t0.elapsed().as_nanos() as u64);
        let mut budget = crate::budget::SyncBudget::new(1 << 20, 1 << 20, 1 << 20);
        let events: Vec<_> = world.drain_contacts(&mut budget).collect();
        contact_events_total += events.len() as u64;
        let state = state_from_world(&world, tick)
            .map_err(|e| BenchmarkError::InvalidInput(format!("state: {e}")))?;
        let h = hash_canonical_state(&state)
            .map_err(|e| BenchmarkError::InvalidInput(format!("hash: {e}")))?;
        chain.push_str(&format!("{tick}:{h}\n"));
    }
    // 末态逐体快照(规范序 = body 位表示升序;跨后端逐元对拍源)。
    let mut sem = world
        .body_semantic_snapshot()
        .map_err(|e| BenchmarkError::InvalidInput(format!("snapshot: {e}")))?;
    sem.sort_by_key(|s| s.body_id.to_bits());
    let mut final_states = Vec::with_capacity(sem.len());
    let mut final_text = String::new();
    for s in &sem {
        final_states.push((
            s.transform.translation,
            s.linvel,
            s.kind == crate::types::BodyKind::Dynamic,
        ));
        final_text.push_str(&format!(
            "{}:t={:?}:v={:?}:a={}\n",
            s.body_id.to_bits(),
            s.transform.translation,
            s.linvel,
            s.is_active
        ));
    }
    Ok(BenchmarkArmOutcome {
        backend,
        ticks: spec.ticks,
        input_digest: spec.input_digest(),
        world_digest: hex(&digest(chain.as_bytes())),
        contact_events_total,
        step_ns,
        final_state_digest: hex(&digest(final_text.as_bytes())),
        final_states,
    })
}

/// 同后端双跑位级一致断言(determinism 画像硬断言;逐 tick hash 链 +
/// 接触计数 + 末态 digest 三面逐位;耗时为 wall-clock 不位冻)。
pub fn assert_double_run_bitwise(
    backend: BackendKind,
    spec: &CanonicalStackSpec,
) -> Result<BenchmarkArmOutcome, BenchmarkError> {
    let a = run_benchmark_arm(backend, spec)?;
    let b = run_benchmark_arm(backend, spec)?;
    if a.world_digest != b.world_digest
        || a.contact_events_total != b.contact_events_total
        || a.final_state_digest != b.final_state_digest
    {
        return Err(BenchmarkError::DeterminismViolation(format!(
            "{backend} 双跑位级不一致(world/contact/final 三面)"
        )));
    }
    Ok(a)
}

/// 跨 solver 确定性偏差统计(只作不变量/容差对拍;跨 solver 不承诺逐位)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossSolverDeviation {
    /// 逐 tick world hash 链逐位相等(false = 已分叉;画像记录,非判据——
    /// 跨 solver 分叉为预期)。
    pub world_chain_bitwise_equal: bool,
    /// 末态平移逐元最大绝对差(m)。
    pub max_translation_abs_diff: f32,
    /// 末态平移逐元平均绝对差(m)。
    pub mean_translation_abs_diff: f32,
    /// 末态线速度逐元最大绝对差(m/s)。
    pub max_linvel_abs_diff: f32,
    /// 接触事件计数差(|a - b|;画像记录)。
    pub contact_events_abs_diff: u64,
    /// 不变量:两臂末态均在地面之上(z ≥ -tol;沉降不穿地的物理不变量)。
    pub rest_above_ground_invariant: bool,
    /// 容差对拍:最大平移偏差 ≤ 冻结容差(堆叠尺度 ~半长 0.45 的 1/9)。
    pub within_tolerance: bool,
}

/// 跨后端末态偏差统计(不变量/容差对拍面;容差钉值 0.05 m)。
pub const CROSS_SOLVER_TOLERANCE_M: f32 = 0.05;

/// 跨 solver 对拍(逐元 abs diff + 不变量;**非逐位**——差异如实记录)。
pub fn cross_solver_deviation(
    jolt: &BenchmarkArmOutcome,
    rapier: &BenchmarkArmOutcome,
) -> Result<CrossSolverDeviation, BenchmarkError> {
    if jolt.final_states.len() != rapier.final_states.len() {
        return Err(BenchmarkError::InvalidInput(
            "两臂末态体数不一致(场景映射破裂)".into(),
        ));
    }
    let mut max_t = 0.0f32;
    let mut sum_t = 0.0f32;
    let mut max_v = 0.0f32;
    let mut rest_ok = true;
    for ((jt, jv, jdyn), (rt, rv, _rdyn)) in
        jolt.final_states.iter().zip(rapier.final_states.iter())
    {
        for k in 0..3 {
            let dt = (jt[k] - rt[k]).abs();
            max_t = max_t.max(dt);
            sum_t += dt;
            max_v = max_v.max((jv[k] - rv[k]).abs());
        }
        // 不变量:动态箱末态箱心在地面之上(地面顶 z=0;容差内允许接触微沉;
        // 静态地面体心 z=-0.5 不参与本断言)。
        if *jdyn && (jt[2] < -CROSS_SOLVER_TOLERANCE_M || rt[2] < -CROSS_SOLVER_TOLERANCE_M) {
            rest_ok = false;
        }
    }
    let n = (jolt.final_states.len() * 3) as f32;
    Ok(CrossSolverDeviation {
        world_chain_bitwise_equal: jolt.world_digest == rapier.world_digest,
        max_translation_abs_diff: max_t,
        mean_translation_abs_diff: sum_t / n.max(1.0),
        max_linvel_abs_diff: max_v,
        contact_events_abs_diff: jolt
            .contact_events_total
            .abs_diff(rapier.contact_events_total),
        rest_above_ground_invariant: rest_ok,
        within_tolerance: max_t <= CROSS_SOLVER_TOLERANCE_M,
    })
}

/// 基准冒充 replay oracle 注入的 fail-closed 面(RED 臂单一出口;RXS-0378
/// L3):任何把基准输出(A/B 任一臂)当 capture/replay 逐位对拍 golden 消费
/// 的请求一律 `ReplayOracleUsurpation`——replay 对拍唯一权威 = 同 solver
/// 同版本 capture/replay 逐 tick hash(RFC-0021 §4.A1 字面不变)。
pub fn compare_as_replay_oracle(
    _baseline: &BenchmarkArmOutcome,
    _candidate: &BenchmarkArmOutcome,
) -> Result<(), BenchmarkError> {
    Err(BenchmarkError::ReplayOracleUsurpation(
        "基准输出不得充当 replay 逐位对拍 oracle(跨 solver 不承诺逐位;replay 权威 = 同 solver 同版本 capture/replay 逐 tick hash)".into(),
    ))
}

/// RD-044 判档申请校验(无 measured 数据的申请 = fail-closed;RXS-0378
/// Implementation Requirements「无 measured 数据的深造判档申请 →
/// fail-closed」字面)。
pub fn validate_rd044_application(measured_report_present: bool) -> Result<(), BenchmarkError> {
    if !measured_report_present {
        return Err(BenchmarkError::MeasuredDataMissing(
            "无 measured 数据的 RD-044 深造判档申请 fail-closed".into(),
        ));
    }
    Ok(())
}

/// RD-044 判档结论(基准报告消费面;字面不变——仅登记,不升格深造)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rd044Verdict {
    /// 维持 no-go 留档(基准未显示 measured 优势,或画像容差对拍未过)。
    MaintainNoGo,
    /// 可申请判档(基准显示 measured 优势且不变量/容差对拍通过;申请动
    /// 作 = 登记,深造不升格、不作验收依赖与生产默认)。
    EligibleToApply,
}

/// RD-044 判档裁决(条件 = Rapier 臂求解耗时中位数 measured 优势 + 不变
/// 量/容差对拍通过;D5 真实 workload 采用面留档——本基准 = canonical A/B
/// 夹具 measured 报告,真实 workload 采用证据归 D5 后续面)。
pub fn rd044_verdict(
    jolt: &BenchmarkArmOutcome,
    rapier: &BenchmarkArmOutcome,
    deviation: &CrossSolverDeviation,
) -> Rd044Verdict {
    let advantage = rapier.step_ns_median() < jolt.step_ns_median();
    if advantage && deviation.rest_above_ground_invariant && deviation.within_tolerance {
        Rd044Verdict::EligibleToApply
    } else {
        Rd044Verdict::MaintainNoGo
    }
}

impl Rd044Verdict {
    /// canonical 名(报告/evidence 面唯一合法字面)。
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::MaintainNoGo => "maintain_no_go",
            Self::EligibleToApply => "eligible_to_apply",
        }
    }
}

/// 完整 A/B 报告(两臂 measured + determinism 画像 + 跨 solver 偏差统计 +
/// RD-044 裁决;harness 序列化落 milestones/g9/g9_m126_rapier_benchmark.json)。
#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    /// canonical 场景 spec。
    pub spec: CanonicalStackSpec,
    /// Jolt 臂(measured)。
    pub jolt: BenchmarkArmOutcome,
    /// Rapier 臂(measured)。
    pub rapier: BenchmarkArmOutcome,
    /// 跨 solver 偏差统计。
    pub deviation: CrossSolverDeviation,
    /// RD-044 裁决。
    pub verdict: Rd044Verdict,
}

/// A/B 夹具执行(两臂各自双跑位级断言 + 同输入 digest 断言 + 偏差统计 +
/// 裁决;Rapier 未编译 → 双臂缺席 fail-closed)。
pub fn run_ab_benchmark(spec: &CanonicalStackSpec) -> Result<BenchmarkReport, BenchmarkError> {
    let jolt = assert_double_run_bitwise(BackendKind::Jolt, spec)?;
    let rapier = assert_double_run_bitwise(BackendKind::Rapier, spec)?;
    if jolt.input_digest != rapier.input_digest {
        return Err(BenchmarkError::InvalidInput(
            "两臂输入 digest 不一致(同输入断言破裂)".into(),
        ));
    }
    let deviation = cross_solver_deviation(&jolt, &rapier)?;
    let verdict = rd044_verdict(&jolt, &rapier, &deviation);
    Ok(BenchmarkReport {
        spec: *spec,
        jolt,
        rapier,
        deviation,
        verdict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0378
    #[test]
    fn ab_fixture_same_input_double_run_bitwise_and_deviation_recorded() {
        let spec = CanonicalStackSpec::default();
        let report = run_ab_benchmark(&spec).expect("ab");
        // 同输入断言(input digest 两臂逐位一致)。
        assert_eq!(report.jolt.input_digest, report.rapier.input_digest);
        // 各自 determinism 画像:双跑位级一致由 assert_double_run_bitwise
        // 内部断言(到达此处 = 两臂自身确定性成立)。
        assert!(report.jolt.step_ns_median() > 0);
        assert!(report.rapier.step_ns_median() > 0);
        // 跨 solver 不承诺逐位:world digest 分叉如实记录(差异非判据)。
        assert_ne!(
            report.jolt.world_digest, report.rapier.world_digest,
            "跨 solver 逐位一致不构成判据(分叉为预期画像)"
        );
        // 不变量:两臂末态均在地面之上(沉降不穿地)。
        assert!(report.deviation.rest_above_ground_invariant);
    }

    //@ spec: RXS-0378
    #[test]
    fn benchmark_as_replay_oracle_fail_closed_and_rd044_requires_measured() {
        let spec = CanonicalStackSpec::default();
        let report = run_ab_benchmark(&spec).expect("ab");
        // 基准冒充 replay oracle 一律 typed Err(RED 臂面)。
        let e = compare_as_replay_oracle(&report.jolt, &report.rapier).unwrap_err();
        assert!(matches!(e, BenchmarkError::ReplayOracleUsurpation(_)));
        let e2 = compare_as_replay_oracle(&report.rapier, &report.jolt).unwrap_err();
        assert!(matches!(e2, BenchmarkError::ReplayOracleUsurpation(_)));
        // 无 measured 数据的判档申请 fail-closed;有 measured = 合规面。
        assert!(matches!(
            validate_rd044_application(false),
            Err(BenchmarkError::MeasuredDataMissing(_))
        ));
        assert!(validate_rd044_application(true).is_ok());
        // 裁决面:两臂 measured 均非零耗时 + 裁决为闭集成员。
        assert!(matches!(
            report.verdict,
            Rd044Verdict::MaintainNoGo | Rd044Verdict::EligibleToApply
        ));
    }
}
