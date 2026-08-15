//! G9.6 M125 Jolt 5.3→5.6 升级 A/B 评估夹具(spec/physics.md RXS-0377;RFC-0024
//! §4.E1;判据逐字引 G9_ACCEPTANCE_MAP §3 M125 行,gate
//! `g9.p1.m125.jolt_56_ab_evaluation`)。
//!
//! 冻结纪律(RXS-0377 L1~L6):
//! - **七步程序逐字**(RFC-0021 §4.A4)——① 冻结 5.3 基线(conformance/physics
//!   全树 digest 清单 + 既有 replay corpus 五轴〔CCD/contact/query 等〕5.3 重跑
//!   PASS + canonical 场景 measured baseline);② 5.6 独立 vendor/ABI 构建
//!   (rurix-physics-sys56,`JPC56_`/`JPH56` 符号隔离,**不覆盖 5.3 基线**——
//!   覆盖注入即 RED);③ 两版本各自证明同版本 capture/replay 逐 tick 一致
//!   (M66 主流 recorder/replayer,各自 backend header 锚);④ 相同 canonical
//!   source asset/input journal A/B(输入 digest 两臂逐位相等为机核断言);
//!   ⑤ 性能阈值只从真实采样写入 budget(本批零 budget counter——评估不升格、
//!   无阈值写入,版本锚按实测 tag/commit 登记);⑥ **失败臂**——任一硬门失败
//!   正式钉住 5.3、记录失败证据、不得伪写 5.6 PASS;⑦ **采纳臂三件事**(corpus
//!   显式迁移保留 5.3 基线 artifact + replay 门新版本重跑落 evidence + 判据
//!   字面经修订后才改版本号)——本评估**不升格默认**,⑦三件登记 not-triggered。
//!   七步执行记录完整(逐步留痕,见 [`SevenStepRecord`])。
//! - **独立 vendor 并存(RED 臂独立有效)**:5.3 基线 vendor 标记(5.3.0 版本
//!   宏 + `JPC_` 符号面)机核在位;5.6 标记(5.6.0 宏 + `JPH56` 命名空间 +
//!   `JPC56_` 符号面)机核在位;[`check_baseline_vendor_markers`] 对覆盖/替换
//!   注入 fail-closed `Err(BaselineVendorTampered)`。
//! - **新摩擦模型重点实测**:5.6 新摩擦模型(平均接触点——上游 v5.6.0 release
//!   notes:Pyramid 测试快 15%/省 40% 内存/消除首接触点序偏向;摩擦 = 2 线性
//!   约束 + 1 角约束,线性用 `μ·Σcontact_impulse`、角用
//!   `μ·Σ(distance·contact_impulse)`)为 A/B 重点项;求解器语义变化**逐字段
//!   exact / tolerance / invariant 分类**([`FieldClass`] 实测驱动,未分类字段
//!   不得默认同性)。
//! - **GPU compute 只评估不接权威(RED 臂独立有效)**:Jolt 5.6 GPU compute
//!   shader 接口编译期整体关闭(sys56 build.rs 四开关 OFF),JoltC C 面零 GPU
//!   导出,任何接入提案一律 [`connect_gpu_compute_authority`] fail-closed typed
//!   Err;接入须 RD-043 + 矩阵 §12 + 独立 Full RFC(GPU 主刚体禁止线 0-byte)。
//! - **layout 探针工具化**:`src/rurix-physics-sys56/tools/layout_dump56.cpp`
//!   入库(所有消费面 `*Settings` 结构 sizeof/offsetof 重测),探针数值进
//!   sys56 `ffi_layout_anchors` 编译期断言。
//! - **两臂诚实登记**:采纳与失败都是正式终态——本夹具 verdict 只产
//!   `maintain_5_3_default`(评估完成不升格)或 `pinned_5_3_on_failure`(硬门
//!   失败钉 5.3),**禁写 5.6 PASS 伪绿**([`validate_report_honesty`] 对伪写
//!   fail-closed);G9 契约判据字面若再钉「Jolt 5.3」,同样须修订后才可改字面。

use std::fmt;
use std::time::Instant;

use rurix_pkg::sha256::{digest, hex};

use crate::capture::canonical::{hash_canonical_state, state_from_world};
use crate::capture::header::PhysicsCaptureHeader;
use crate::capture::journal::JournalCommand;
use crate::capture::recorder::{CaptureArtifact, CaptureRecorder, default_budget};
use crate::capture::replayer::{ReplayVerdict, replay_artifact};
use crate::types::{
    BackendKind, BodyDesc, BodyKind, MassProps, PhysicsTransform, ShapeDesc, WorldDesc,
};
use crate::world::PhysicsWorld;

/// M125 域错误(fail-closed 单一出口;harness RED 臂锚字面)。
#[derive(Debug, Clone, PartialEq)]
pub enum AbError {
    /// Jolt56 后端未编译(feature `jolt56` off)——A/B 缺臂 fail-closed,
    /// 不静默退化为单臂绿。
    Jolt56BackendNotCompiled(String),
    /// 5.3 基线 vendor 标记漂移(5.6 覆盖/替换注入)——RED 臂字面。
    BaselineVendorTampered(String),
    /// 5.6 评估臂 vendor 标记缺失(独立 vendor 线不完整)。
    Vendor56MarkerMissing(String),
    /// GPU compute 接权威提案/接线——一律拒绝(RXS-0377 L4;接入须
    /// RD-043 + 矩阵 §12 + 独立 Full RFC)。
    GpuComputeAuthorityUsurpation(String),
    /// 同后端双跑位级不一致(自身确定性破坏)。
    DeterminismViolation(String),
    /// capture/replay 同版本逐 tick 不一致(七步③硬门)。
    ReplayMismatch(String),
    /// 伪写 5.6 PASS / 采纳登记造假(失败臂伪绿)——门 FAIL 面。
    FakePassAttempt(String),
    /// 输入/场景非法。
    InvalidInput(String),
}

impl fmt::Display for AbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jolt56BackendNotCompiled(s) => write!(f, "Jolt56BackendNotCompiled({s})"),
            Self::BaselineVendorTampered(s) => write!(f, "BaselineVendorTampered({s})"),
            Self::Vendor56MarkerMissing(s) => write!(f, "Vendor56MarkerMissing({s})"),
            Self::GpuComputeAuthorityUsurpation(s) => {
                write!(f, "GpuComputeAuthorityUsurpation({s})")
            }
            Self::DeterminismViolation(s) => write!(f, "DeterminismViolation({s})"),
            Self::ReplayMismatch(s) => write!(f, "ReplayMismatch({s})"),
            Self::FakePassAttempt(s) => write!(f, "FakePassAttempt({s})"),
            Self::InvalidInput(s) => write!(f, "InvalidInput({s})"),
        }
    }
}

impl std::error::Error for AbError {}

/// GPU compute 评估留档字面(RFC-0024 §4.E1 分项处置行消费;报告/evidence 面
/// 唯一合法字面)。Jolt 5.6 GPU compute shader 接口(DX12/Vulkan/Metal 三实现 +
/// CPU 参考)在本 vendor 线**编译期整体关闭**(`JPH_USE_DX12/VK/MTL/CPU_COMPUTE
/// =OFF`,Jolt/Compute/** 与 GPU 毛发 Jolt/Shaders/** 不参与构建),JoltC C 面
/// 从未导出 GPU compute 入口——接口在本进程**结构性不可达**;评估留档,接入须
/// RD-043 + 矩阵 §12 + 独立 Full RFC(GPU 主刚体禁止线 0-byte)。
pub const GPU_COMPUTE_EVALUATION_NOTE: &str = "Jolt 5.6 新增 GPU compute shader 接口(上游 v5.6.0:DX12/Vulkan/Metal 实现 + JPH_USE_DX12/VK/MTL/CPU_COMPUTE cmake 开关;GPU strand 毛发 Cosserat 杆上游自标 work-in-progress)——本 vendor 线编译期四开关 OFF 整体排除(结构性不可达),JoltC C 面零 GPU 导出;只评估不接权威:任何把 GPU compute 接为权威求解路径的提案/接线一律 fail-closed typed Err;接入须 RD-043 + 矩阵 §12 + 独立 Full RFC(GPU 主刚体禁止线 0-byte,RD-043 观察维持);GPU strand 毛发仅登记为非权威装饰副轨候选(async-decorative 通道维持 M123 判档门前不启用)。";

/// 新摩擦模型上游语义留档(上游 v5.6.0 release notes 实测转述;A/B 重点项分类基线)。
pub const FRICTION_MODEL_56_NOTE: &str = "Jolt 5.6 新摩擦模型(平均接触点):摩擦不再逐接触点施加——改算平均接触点单点施加,消除首接触点序偏向;摩擦 = 2 线性约束 + 1 角约束,线性约束上限 μ·Σ(contact_impulse),角约束上限 μ·Σ(distance·contact_impulse);上游口径 Pyramid 测试快 15%/省 40% 内存。ABI 面印记:CollisionEstimationResult 重排(逐点 Impulse{Contact,Friction1,Friction2} 删除 → 聚合摩擦 FrictionPoint/Tangent1/Tangent2 + FrictionImpulse1/2 + AngularFrictionImpulse + 逐点 ContactImpulse float 数组)。求解器语义变化 → canonical A/B 逐字段 exact/tolerance/invariant 实测分类(未分类字段不得默认同性)。";

/// canonical A/B 场景(七步④:相同 canonical source asset / input journal——
/// 静态地面 + 箱堆叠(接触 manifold/摩擦承压)+ 滑块(初切向速度,摩擦减速
/// 直射新摩擦模型);两臂同一实例,输入 digest 逐位相等为机核断言)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanonicalAbSpec {
    /// 堆叠层数(≥ 2)。
    pub layers: u32,
    /// 箱半长(m)。
    pub box_half: f32,
    /// 层间初始缝(m)。
    pub layer_gap: f32,
    /// 材质摩擦(堆叠与滑块同参——摩擦模型直射面)。
    pub friction: f32,
    /// 恢复系数。
    pub restitution: f32,
    /// 滑块初切向速度(m/s,世界系;直射摩擦减速)。
    pub slider_velocity: [f32; 3],
    /// tick 数。
    pub ticks: u64,
}

impl Default for CanonicalAbSpec {
    fn default() -> Self {
        Self {
            layers: 4,
            box_half: 0.45,
            layer_gap: 0.001,
            friction: 0.6,
            restitution: 0.0,
            slider_velocity: [2.5, 0.0, 0.0],
            ticks: 120,
        }
    }
}

impl CanonicalAbSpec {
    /// 校验(fail-closed)。
    pub fn validate(&self) -> Result<(), AbError> {
        if self.layers < 2
            || !self.box_half.is_finite()
            || self.box_half <= 0.0
            || !self.layer_gap.is_finite()
            || self.layer_gap < 0.0
            || !self.friction.is_finite()
            || !self.restitution.is_finite()
            || !self.slider_velocity.iter().all(|v| v.is_finite())
            || self.ticks < 2
        {
            return Err(AbError::InvalidInput("canonical ab spec".into()));
        }
        Ok(())
    }

    /// canonical 输入 journal 文本(体创建命令集 + 滑块初速 + tick 数;两臂同一
    /// 实例,digest 逐位相等 = 「同一输入 journal」机核面)。
    pub fn input_journal_text(&self) -> String {
        let mut s = String::from("create_static_ground\n");
        for l in 0..self.layers {
            s.push_str(&format!("create_dyn_box layer={l}\n"));
        }
        s.push_str(&format!(
            "create_slider v=({:08x},{:08x},{:08x})\n",
            self.slider_velocity[0].to_bits(),
            self.slider_velocity[1].to_bits(),
            self.slider_velocity[2].to_bits()
        ));
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

    /// 体创建命令集(静态地面 + 逐层动态箱 + 滑块;两臂同一构建面)。
    pub fn body_descs(&self) -> Vec<BodyDesc> {
        let mut out = Vec::with_capacity(self.layers as usize + 2);
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
        // 滑块(摩擦减速直射面;偏离堆叠 x 轴向 3 m 防干涉)。
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
                allow_sleep: false,
            },
            ccd: false,
            transform: PhysicsTransform {
                translation: [-6.0, 0.0, self.box_half],
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
        });
        out
    }

    /// determinism 画像世界 desc(两臂同一画像:固定 dt 锁死、单线程 declared、
    /// 睡眠策略钉值、零 IO;与 5.3 基线 M66 画像逐项一致)。
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

    /// 滑块在 body_descs 中的下标(摩擦模型专项记录面)。
    pub fn slider_index(&self) -> usize {
        self.layers as usize + 1
    }
}

/// 末态逐体快照元组(translation, rotation, linvel, angvel, is_dynamic;规范序 =
/// body 位表示升序;逐字段分类源)。
pub type BodyFinalState = ([f32; 3], [f32; 4], [f32; 3], [f32; 3], bool);

/// 单臂测量产出(三面:逐 tick world 状态摘要链 + 接触事件计数 + 求解耗时;
/// 外加同版本 capture/replay 逐 tick 一致断言面)。
#[derive(Debug, Clone)]
pub struct ArmOutcome {
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
    /// 末态逐体快照摘要(canonical 文本 digest;跨版本偏差统计面)。
    pub final_state_digest: String,
    /// 末态逐体 (translation, rotation, linvel, angvel, is_dynamic) 展平序列
    /// (规范序 = body 位表示升序;逐字段分类源)。
    pub final_states: Vec<BodyFinalState>,
    /// 七步③:capture→replay 同版本逐 tick 一致(ticks_ok == tick_count 且
    /// journal 全消费且 verdict == Pass)。
    pub replay_ticks_ok: u64,
    /// replay journal 全消费标记。
    pub replay_journal_fully_consumed: bool,
}

impl ArmOutcome {
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

    /// 七步③断言面:同版本 capture/replay 逐 tick 一致成立。
    pub fn replay_consistent(&self) -> bool {
        self.replay_ticks_ok == self.ticks && self.replay_journal_fully_consumed
    }
}

/// 单臂执行(同 determinism 画像;后端未编译 → 确定性 typed Err 不静默)。
/// 逐 tick 计时 + world 摘要链 + 接触计数;末态逐体快照供逐字段分类。
pub fn run_arm(backend: BackendKind, spec: &CanonicalAbSpec) -> Result<ArmOutcome, AbError> {
    spec.validate()?;
    let desc = spec.world_desc(backend);
    let mut world = PhysicsWorld::new(desc.clone()).map_err(|e| match e {
        crate::error::PhysicsError::BackendNotCompiled(BackendKind::Jolt56) => {
            AbError::Jolt56BackendNotCompiled(
                "feature `jolt56` 未编译——A/B 缺臂 fail-closed(不静默单臂充绿)".into(),
            )
        }
        other => AbError::InvalidInput(format!("world create: {other}")),
    })?;
    let descs = spec.body_descs();
    let ids = world
        .add_bodies_batch(&descs)
        .map_err(|e| AbError::InvalidInput(format!("add bodies: {e}")))?;
    // 滑块初速(输入 journal 一部分;replay 路径由 journal SetVelocity 重放)。
    world
        .set_linear_velocity(ids[spec.slider_index()], spec.slider_velocity)
        .map_err(|e| AbError::InvalidInput(format!("slider velocity: {e}")))?;
    let dt = desc.dt_fixed;
    let mut chain = String::from("world\n");
    let mut contact_events_total = 0u64;
    let mut step_ns = Vec::with_capacity(spec.ticks as usize);
    for tick in 0..spec.ticks {
        let t0 = Instant::now();
        world
            .step(dt)
            .map_err(|e| AbError::InvalidInput(format!("step: {e}")))?;
        step_ns.push(t0.elapsed().as_nanos() as u64);
        let mut budget = crate::budget::SyncBudget::new(1 << 20, 1 << 20, 1 << 20);
        let events: Vec<_> = world.drain_contacts(&mut budget).collect();
        contact_events_total += events.len() as u64;
        let state = state_from_world(&world, tick)
            .map_err(|e| AbError::InvalidInput(format!("state: {e}")))?;
        let h = hash_canonical_state(&state)
            .map_err(|e| AbError::InvalidInput(format!("hash: {e}")))?;
        chain.push_str(&format!("{tick}:{h}\n"));
    }
    let mut sem = world
        .body_semantic_snapshot()
        .map_err(|e| AbError::InvalidInput(format!("snapshot: {e}")))?;
    sem.sort_by_key(|s| s.body_id.to_bits());
    let mut final_states = Vec::with_capacity(sem.len());
    let mut final_text = String::new();
    for s in &sem {
        final_states.push((
            s.transform.translation,
            s.transform.rotation,
            s.linvel,
            s.angvel,
            s.kind == crate::types::BodyKind::Dynamic,
        ));
        final_text.push_str(&format!(
            "{}:t={:?}:r={:?}:v={:?}:w={:?}:a={}\n",
            s.body_id.to_bits(),
            s.transform.translation,
            s.transform.rotation,
            s.linvel,
            s.angvel,
            s.is_active
        ));
    }
    Ok(ArmOutcome {
        backend,
        ticks: spec.ticks,
        input_digest: spec.input_digest(),
        world_digest: hex(&digest(chain.as_bytes())),
        contact_events_total,
        step_ns,
        final_state_digest: hex(&digest(final_text.as_bytes())),
        final_states,
        replay_ticks_ok: 0,
        replay_journal_fully_consumed: false,
    })
}

/// 七步③:单臂 capture 录制(M66 主流;header 按臂版本锚登记——5.3 臂
/// `new_jolt_53` / 5.6 臂 `new_jolt_56`,版本锚按实测 tag/commit)。
pub fn record_arm_capture(
    backend: BackendKind,
    spec: &CanonicalAbSpec,
) -> Result<CaptureArtifact, AbError> {
    spec.validate()?;
    let desc = spec.world_desc(backend);
    let budget = default_budget(&desc);
    let input_digest = spec.input_digest();
    let header = match backend {
        BackendKind::Jolt56 => PhysicsCaptureHeader::new_jolt_56(
            "g96_m125_canonical_ab",
            spec.ticks,
            &desc,
            "g9.6-jolt56-ab-harness",
            &input_digest,
            budget,
        ),
        _ => PhysicsCaptureHeader::new_jolt_53(
            "g96_m125_canonical_ab",
            spec.ticks,
            &desc,
            "g9.6-jolt56-ab-harness",
            &input_digest,
            budget,
        ),
    };
    let dt = desc.dt_fixed;
    let mut world = PhysicsWorld::new(desc.clone()).map_err(|e| match e {
        crate::error::PhysicsError::BackendNotCompiled(BackendKind::Jolt56) => {
            AbError::Jolt56BackendNotCompiled("feature `jolt56` 未编译".into())
        }
        other => AbError::InvalidInput(format!("world create: {other}")),
    })?;
    let mut recorder = CaptureRecorder::begin_with_header(header);
    let descs = spec.body_descs();
    for tick in 0..spec.ticks {
        let mut pre: Vec<JournalCommand> = Vec::new();
        if tick == 0 {
            let ids = world
                .add_bodies_batch(&descs)
                .map_err(|e| AbError::InvalidInput(format!("add bodies: {e}")))?;
            pre.push(JournalCommand::CreateBodies {
                descs: descs.clone(),
                assigned_ids: ids.iter().map(|b| b.to_bits()).collect(),
            });
            let slider = ids[spec.slider_index()];
            world
                .set_linear_velocity(slider, spec.slider_velocity)
                .map_err(|e| AbError::InvalidInput(format!("slider velocity: {e}")))?;
            world
                .set_angular_velocity(slider, [0.0, 0.0, 0.0])
                .map_err(|e| AbError::InvalidInput(format!("slider angvel: {e}")))?;
            pre.push(JournalCommand::SetVelocity {
                body: slider.to_bits(),
                linear: spec.slider_velocity,
                angular: [0.0, 0.0, 0.0],
            });
        }
        let stats = world
            .step(dt)
            .map_err(|e| AbError::InvalidInput(format!("step: {e}")))?;
        let dropped = u64::from(stats.contacts_dropped);
        recorder
            .seal_tick(&mut world, tick, pre, stats.contacts_emitted, dropped)
            .map_err(|e| AbError::InvalidInput(format!("seal: {e}")))?;
    }
    recorder
        .finish(&world)
        .map_err(|e| AbError::InvalidInput(format!("finish: {e}")))
}

/// 七步③断言:同版本 capture→replay 逐 tick 一致(fail-closed;verdict 必须
/// Pass 且 ticks_ok == tick_count 且 journal 全消费)。
pub fn assert_arm_capture_replay_consistent(
    backend: BackendKind,
    spec: &CanonicalAbSpec,
) -> Result<(u64, bool), AbError> {
    let artifact = record_arm_capture(backend, spec)?;
    let report = replay_artifact(&artifact, None)
        .map_err(|e| AbError::ReplayMismatch(format!("replay err: {e}")))?;
    if report.verdict != ReplayVerdict::Pass {
        return Err(AbError::ReplayMismatch(format!(
            "{backend} capture/replay verdict = {:?}(非 Pass)",
            report.verdict
        )));
    }
    if report.ticks_ok != spec.ticks || !report.journal_fully_consumed {
        return Err(AbError::ReplayMismatch(format!(
            "{backend} replay ticks_ok={} journal_consumed={} (期望 {}/{})",
            report.ticks_ok, report.journal_fully_consumed, spec.ticks, true
        )));
    }
    Ok((report.ticks_ok, report.journal_fully_consumed))
}

/// 单臂完整面:双跑位级断言(自身确定性)+ capture/replay 一致断言(七步③)。
pub fn run_arm_full(backend: BackendKind, spec: &CanonicalAbSpec) -> Result<ArmOutcome, AbError> {
    let a = run_arm(backend, spec)?;
    let b = run_arm(backend, spec)?;
    if a.world_digest != b.world_digest
        || a.contact_events_total != b.contact_events_total
        || a.final_state_digest != b.final_state_digest
    {
        return Err(AbError::DeterminismViolation(format!(
            "{backend} 双跑位级不一致(world/contact/final 三面)"
        )));
    }
    let (ticks_ok, consumed) = assert_arm_capture_replay_consistent(backend, spec)?;
    let mut out = a;
    out.replay_ticks_ok = ticks_ok;
    out.replay_journal_fully_consumed = consumed;
    Ok(out)
}

/// 逐字段分类(RXS-0377 L3:exact / tolerance / invariant;未分类字段不得默认
/// 同性)。实测驱动:位级相等 → exact;容差内 → tolerance;其余 → invariant
/// (物理不变量断言面:动态体末态不穿地、全分量有限)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldClass {
    /// 位级一致。
    Exact,
    /// 容差内(数值漂移可解释)。
    Tolerance,
    /// 仅不变量成立(语义面分叉如实记录)。
    Invariant,
}

impl FieldClass {
    /// canonical 名(报告/evidence 面唯一合法字面)。
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Tolerance => "tolerance",
            Self::Invariant => "invariant",
        }
    }
}

/// 字段级分类容差钉值(堆叠尺度 ~半长 0.45 的 1/9;沿 M126 容差面)。
pub const FIELD_TOLERANCE_TRANSLATION_M: f32 = 0.05;
/// 旋转四元数逐分量容差。
pub const FIELD_TOLERANCE_ROTATION: f32 = 0.01;
/// 速度逐分量容差(m/s 与 rad/s)。
pub const FIELD_TOLERANCE_VELOCITY: f32 = 0.05;

fn classify_diff(max_abs_diff: f32, tol: f32, bitwise_equal: bool) -> FieldClass {
    if bitwise_equal {
        FieldClass::Exact
    } else if max_abs_diff <= tol {
        FieldClass::Tolerance
    } else {
        FieldClass::Invariant
    }
}

/// 跨版本 A/B 偏差画像(七步④;差异如实记录非判据)+ 逐字段分类(L3)+
/// 新摩擦模型专项(滑块行程/堆叠沉降/接触计数三分面)。
#[derive(Debug, Clone, PartialEq)]
pub struct CrossVersionDeviation {
    /// 逐 tick world hash 链逐位相等(false = 已分叉;画像记录,非判据——
    /// 新摩擦模型求解器语义变化下分叉为预期)。
    pub world_chain_bitwise_equal: bool,
    /// 末态平移逐元最大绝对差(m)。
    pub max_translation_abs_diff: f32,
    /// 末态平移逐元平均绝对差(m)。
    pub mean_translation_abs_diff: f32,
    /// 末态旋转四元数逐元最大绝对差。
    pub max_rotation_abs_diff: f32,
    /// 末态线速度逐元最大绝对差(m/s)。
    pub max_linvel_abs_diff: f32,
    /// 末态角速度逐元最大绝对差(rad/s)。
    pub max_angvel_abs_diff: f32,
    /// 接触事件计数差(|a - b|;画像记录)。
    pub contact_events_abs_diff: u64,
    /// 不变量:两臂末态动态体均在地面之上(z ≥ -tol)且全分量有限。
    pub rest_above_ground_invariant: bool,
    /// 逐字段分类(RXS-0377 L3;实测驱动)。
    pub class_translation: FieldClass,
    pub class_rotation: FieldClass,
    pub class_linvel: FieldClass,
    pub class_angvel: FieldClass,
    pub class_contact_events: FieldClass,
    pub class_world_chain: FieldClass,
    /// 新摩擦模型专项:滑块末态 x 行程差(m;两臂滑块 x 位移差绝对值)。
    pub friction_slider_travel_abs_diff: f32,
    /// 新摩擦模型专项:堆叠体(动态非滑块)末态 z 最大绝对差(m)。
    pub friction_stack_z_abs_diff: f32,
}

/// 跨版本对拍(逐元 abs diff + 不变量 + 逐字段分类;**非逐位**——差异如实
/// 记录;`slider_idx` = 滑块在规范序中的下标)。
pub fn cross_version_deviation(
    arm_53: &ArmOutcome,
    arm_56: &ArmOutcome,
    slider_idx: usize,
) -> Result<CrossVersionDeviation, AbError> {
    if arm_53.final_states.len() != arm_56.final_states.len() {
        return Err(AbError::InvalidInput(
            "两臂末态体数不一致(场景映射破裂)".into(),
        ));
    }
    let mut max_t = 0.0f32;
    let mut sum_t = 0.0f32;
    let mut max_r = 0.0f32;
    let mut max_v = 0.0f32;
    let mut max_w = 0.0f32;
    let mut rest_ok = true;
    let mut bitwise_translation = true;
    let mut bitwise_rotation = true;
    let mut bitwise_linvel = true;
    let mut bitwise_angvel = true;
    let mut stack_z = 0.0f32;
    let n_bodies = arm_53.final_states.len();
    for (i, (a, b)) in arm_53
        .final_states
        .iter()
        .zip(arm_56.final_states.iter())
        .enumerate()
    {
        for k in 0..3 {
            let dt = (a.0[k] - b.0[k]).abs();
            max_t = max_t.max(dt);
            sum_t += dt;
            max_v = max_v.max((a.2[k] - b.2[k]).abs());
            max_w = max_w.max((a.3[k] - b.3[k]).abs());
            if a.0[k].to_bits() != b.0[k].to_bits() {
                bitwise_translation = false;
            }
            if a.2[k].to_bits() != b.2[k].to_bits() {
                bitwise_linvel = false;
            }
            if a.3[k].to_bits() != b.3[k].to_bits() {
                bitwise_angvel = false;
            }
            if !a.0[k].is_finite()
                || !b.0[k].is_finite()
                || !a.2[k].is_finite()
                || !b.2[k].is_finite()
            {
                rest_ok = false;
            }
        }
        for k in 0..4 {
            max_r = max_r.max((a.1[k] - b.1[k]).abs());
            if a.1[k].to_bits() != b.1[k].to_bits() {
                bitwise_rotation = false;
            }
        }
        // 不变量:动态体末态箱心在地面之上(地面顶 z=0;容差内允许接触微沉;
        // 静态地面体心 z=-0.5 不参与本断言)。
        if a.4 && (a.0[2] < -FIELD_TOLERANCE_TRANSLATION_M || b.0[2] < -FIELD_TOLERANCE_TRANSLATION_M)
        {
            rest_ok = false;
        }
        // 堆叠体 z 偏差(滑块与静态地面除外)。
        if a.4 && i != slider_idx {
            stack_z = stack_z.max((a.0[2] - b.0[2]).abs());
        }
    }
    let n = (n_bodies * 3) as f32;
    let slider_travel = if slider_idx < n_bodies {
        (arm_53.final_states[slider_idx].0[0] - arm_56.final_states[slider_idx].0[0]).abs()
    } else {
        return Err(AbError::InvalidInput("slider index 越界".into()));
    };
    let contact_diff = arm_53.contact_events_total.abs_diff(arm_56.contact_events_total);
    Ok(CrossVersionDeviation {
        world_chain_bitwise_equal: arm_53.world_digest == arm_56.world_digest,
        max_translation_abs_diff: max_t,
        mean_translation_abs_diff: sum_t / n.max(1.0),
        max_rotation_abs_diff: max_r,
        max_linvel_abs_diff: max_v,
        max_angvel_abs_diff: max_w,
        contact_events_abs_diff: contact_diff,
        rest_above_ground_invariant: rest_ok,
        class_translation: classify_diff(max_t, FIELD_TOLERANCE_TRANSLATION_M, bitwise_translation),
        class_rotation: classify_diff(max_r, FIELD_TOLERANCE_ROTATION, bitwise_rotation),
        class_linvel: classify_diff(max_v, FIELD_TOLERANCE_VELOCITY, bitwise_linvel),
        class_angvel: classify_diff(max_w, FIELD_TOLERANCE_VELOCITY, bitwise_angvel),
        class_contact_events: if contact_diff == 0 {
            FieldClass::Exact
        } else {
            FieldClass::Invariant
        },
        class_world_chain: if arm_53.world_digest == arm_56.world_digest {
            FieldClass::Exact
        } else {
            FieldClass::Invariant
        },
        friction_slider_travel_abs_diff: slider_travel,
        friction_stack_z_abs_diff: stack_z,
    })
}

/// 5.3 基线 vendor 标记核验(七步② RED 臂锚:`Core.h` 版本宏 5.3.0 +
/// `JPH_NAMESPACE_BEGIN` 未改名 + `Functions.h` 符号面 `JPC_` 未改名;任一
/// 漂移 = 5.6 覆盖/替换注入,fail-closed `BaselineVendorTampered`)。
pub fn check_baseline_vendor_markers(core_h: &str, functions_h: &str) -> Result<(), AbError> {
    let version_ok = core_h.contains("#define JPH_VERSION_MAJOR 5")
        && core_h.contains("#define JPH_VERSION_MINOR 3")
        && core_h.contains("#define JPH_VERSION_PATCH 0");
    if !version_ok {
        return Err(AbError::BaselineVendorTampered(
            "5.3 基线 Core.h 版本宏漂移(覆盖注入信号)".into(),
        ));
    }
    if core_h.contains("namespace JPH56") || core_h.contains("JPH56::") {
        return Err(AbError::BaselineVendorTampered(
            "5.3 基线 Core.h 出现 5.6 线命名空间标记(覆盖注入信号)".into(),
        ));
    }
    if !functions_h.contains("JPC_PhysicsSystem_new") {
        return Err(AbError::BaselineVendorTampered(
            "5.3 基线 Functions.h JPC_ 符号面缺失(覆盖注入信号)".into(),
        ));
    }
    if functions_h.contains("JPC56_PhysicsSystem_new") {
        return Err(AbError::BaselineVendorTampered(
            "5.3 基线 Functions.h 出现 JPC56_ 符号(覆盖注入信号)".into(),
        ));
    }
    Ok(())
}

/// 5.6 评估臂 vendor 标记核验(独立 vendor 线完整性:`Core.h` 5.6.0 宏 +
/// `namespace JPH56` + `Functions.h` `JPC56_` 符号面)。
pub fn check_vendor56_markers(core_h: &str, functions_h: &str) -> Result<(), AbError> {
    let version_ok = core_h.contains("#define JPH_VERSION_MAJOR 5")
        && core_h.contains("#define JPH_VERSION_MINOR 6")
        && core_h.contains("#define JPH_VERSION_PATCH 0");
    if !version_ok {
        return Err(AbError::Vendor56MarkerMissing(
            "5.6 线 Core.h 版本宏非 5.6.0".into(),
        ));
    }
    if !core_h.contains("namespace JPH56") {
        return Err(AbError::Vendor56MarkerMissing(
            "5.6 线 Core.h 缺 JPH56 命名空间标记(符号隔离面缺失)".into(),
        ));
    }
    if !functions_h.contains("JPC56_PhysicsSystem_new") {
        return Err(AbError::Vendor56MarkerMissing(
            "5.6 线 Functions.h 缺 JPC56_ 符号面(符号隔离面缺失)".into(),
        ));
    }
    Ok(())
}

/// GPU compute 接权威注入的 fail-closed 面(RED 臂单一出口;RXS-0377 L4):
/// 任何把 Jolt 5.6 GPU compute shader 接口接为权威求解路径的提案/接线一律
/// `GpuComputeAuthorityUsurpation`——只评估不接权威(GPU 主刚体禁止线
/// 0-byte);接入须 RD-043 + 矩阵 §12 + 独立 Full RFC。
pub fn connect_gpu_compute_authority(_proposal: &str) -> Result<(), AbError> {
    Err(AbError::GpuComputeAuthorityUsurpation(
        "GPU compute 只评估不接权威(GPU 主刚体禁止线 0-byte;接入须 RD-043 + 矩阵 §12 + 独立 Full RFC)".into(),
    ))
}

/// 两臂终态(诚实登记闭集;RXS-0377 L6)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbVerdict {
    /// 评估完成,5.3 维持生产默认(不升格 5.6;采纳臂⑦三件事 not-triggered
    /// 登记——corpus 迁移/replay 门重跑/判据字面修订均未触发,采纳归后续
    /// 治理裁决)。
    Maintain53Default,
    /// 硬门失败,正式钉住 5.3(失败证据记录;不得伪写 5.6 PASS)。
    Pinned53OnFailure,
}

impl AbVerdict {
    /// canonical 名(报告/evidence 面唯一合法字面)。
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Maintain53Default => "maintain_5_3_default",
            Self::Pinned53OnFailure => "pinned_5_3_on_failure",
        }
    }
}

/// 伪写 5.6 PASS 拒绝面(失败臂伪绿 → 门 FAIL;RXS-0377 L6):任何把「5.6 臂
/// 已采纳/5.6 PASS 为生产默认」字面写进报告的尝试一律 `FakePassAttempt`;
/// 合法 verdict 仅 [`AbVerdict`] 闭集两字面。
pub fn validate_report_honesty(verdict_literal: &str) -> Result<(), AbError> {
    match verdict_literal {
        "maintain_5_3_default" | "pinned_5_3_on_failure" => Ok(()),
        other => Err(AbError::FakePassAttempt(format!(
            "非法 verdict 字面 {other:?}(伪写 5.6 PASS 面;合法闭集 = maintain_5_3_default | pinned_5_3_on_failure)"
        ))),
    }
}

/// 七步执行记录(逐步留痕;每步 status + 证据字面进报告)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SevenStepRecord {
    /// ① 冻结 5.3 基线:conformance/physics 全树 digest 清单 + 既有 replay
    /// corpus 5.3 重跑全 PASS + canonical measured baseline 在位。
    pub step1_baseline_frozen: bool,
    /// ② 5.6 独立 vendor/ABI 构建不覆盖 5.3(标记三面核验 + 同进程并存实例化)。
    pub step2_independent_vendor: bool,
    /// ③ 两版本各自 capture/replay 逐 tick 一致。
    pub step3_replay_each_consistent: bool,
    /// ④ 相同 canonical source asset/input journal A/B(输入 digest 逐位相等)。
    pub step4_canonical_ab: bool,
    /// ⑤ measured 真实采样 + 版本锚实测登记(零 budget counter 写入登记)。
    pub step5_measured_budget_discipline: bool,
    /// ⑥ 失败臂语义在位(硬门失败 → pinned_5_3_on_failure + 证据,不伪绿)。
    pub step6_failure_arm_honest: bool,
    /// ⑦ 采纳臂三件事登记(本评估不升格 → 三件 not-triggered 如实登记)。
    pub step7_adoption_items_registered: bool,
}

/// 完整 A/B 报告(双臂 measured + determinism 画像 + 跨版本偏差与逐字段分类 +
/// 七步记录 + verdict;harness 序列化落 milestones/g9/g9_m125_jolt56_ab.json)。
#[derive(Debug, Clone)]
pub struct AbReport {
    /// canonical 场景 spec。
    pub spec: CanonicalAbSpec,
    /// 5.3 基线臂(measured + replay 一致)。
    pub arm_53: ArmOutcome,
    /// 5.6 评估臂(measured + replay 一致)。
    pub arm_56: ArmOutcome,
    /// 跨版本偏差画像与逐字段分类。
    pub deviation: CrossVersionDeviation,
    /// 两臂终态(诚实登记)。
    pub verdict: AbVerdict,
    /// 七步执行记录。
    pub steps: SevenStepRecord,
}

/// A/B 夹具执行(七步③④机核:两臂各自双跑位级断言 + capture/replay 一致
/// 断言 + 同输入 digest 断言 + 偏差画像与逐字段分类;verdict 由调用方按硬门
/// 全绿性登记——本函数全绿 ⇒ `Maintain53Default` 评估完成不升格)。
pub fn run_ab_evaluation(spec: &CanonicalAbSpec) -> Result<AbReport, AbError> {
    let arm_53 = run_arm_full(BackendKind::Jolt, spec)?;
    let arm_56 = run_arm_full(BackendKind::Jolt56, spec)?;
    if arm_53.input_digest != arm_56.input_digest {
        return Err(AbError::InvalidInput(
            "两臂输入 digest 不一致(同输入断言破裂)".into(),
        ));
    }
    if !arm_53.replay_consistent() || !arm_56.replay_consistent() {
        return Err(AbError::ReplayMismatch(
            "七步③ 同版本 capture/replay 一致断言破裂".into(),
        ));
    }
    let deviation = cross_version_deviation(&arm_53, &arm_56, spec.slider_index())?;
    Ok(AbReport {
        spec: *spec,
        arm_53,
        arm_56,
        deviation,
        verdict: AbVerdict::Maintain53Default,
        steps: SevenStepRecord {
            step1_baseline_frozen: false,
            step2_independent_vendor: false,
            step3_replay_each_consistent: true,
            step4_canonical_ab: true,
            step5_measured_budget_discipline: false,
            step6_failure_arm_honest: false,
            step7_adoption_items_registered: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0377
    #[test]
    fn coexistence_replay_consistency_and_canonical_ab() {
        // 七步②并存断言:双后端同进程各自实例化(链接即符号隔离证明)。
        let spec = CanonicalAbSpec::default();
        spec.validate().expect("spec");
        let d53 = spec.world_desc(BackendKind::Jolt);
        let d56 = spec.world_desc(BackendKind::Jolt56);
        let w53 = PhysicsWorld::new(d53.clone()).expect("5.3 基线世界");
        let w56 = PhysicsWorld::new(d56.clone()).expect("5.6 评估臂世界");
        assert_eq!(w53.desc().backend, BackendKind::Jolt);
        assert_eq!(w56.desc().backend, BackendKind::Jolt56);
        // 同 determinism 画像(除后端外 desc 全字段逐位)。
        assert_eq!(d53.gravity, d56.gravity);
        assert_eq!(d53.job_threads, d56.job_threads);
        assert_eq!(d53.dt_fixed.to_bits(), d56.dt_fixed.to_bits());
        drop(w53);
        drop(w56);

        // 七步③④:两臂各自双跑位级 + capture/replay 逐 tick 一致 + 同输入 +
        // 偏差画像逐字段分类闭集。
        let report = run_ab_evaluation(&spec).expect("ab");
        assert_eq!(report.arm_53.input_digest, report.arm_56.input_digest);
        assert!(report.arm_53.replay_consistent());
        assert!(report.arm_56.replay_consistent());
        assert!(report.arm_53.step_ns_median() > 0);
        assert!(report.arm_56.step_ns_median() > 0);
        // 不变量:两臂末态均在地面之上(沉降不穿地)。
        assert!(report.deviation.rest_above_ground_invariant);
        // 逐字段分类全分类(闭集成员;未分类不得默认同性)。
        for c in [
            report.deviation.class_translation,
            report.deviation.class_rotation,
            report.deviation.class_linvel,
            report.deviation.class_angvel,
            report.deviation.class_contact_events,
            report.deviation.class_world_chain,
        ] {
            assert!(matches!(
                c,
                FieldClass::Exact | FieldClass::Tolerance | FieldClass::Invariant
            ));
        }
        // 两臂终态:评估完成不升格(5.3 维持默认)。
        assert_eq!(report.verdict, AbVerdict::Maintain53Default);
        assert!(report.steps.step3_replay_each_consistent);
        assert!(report.steps.step4_canonical_ab);
    }

    //@ spec: RXS-0377
    #[test]
    fn vendor_overwrite_gpu_authority_and_fake_pass_fail_closed() {
        // RED 臂 1:5.6 覆盖 5.3 基线 vendor 注入——篡改标记必须 fail-closed。
        let good_core = "#define JPH_VERSION_MAJOR 5\n#define JPH_VERSION_MINOR 3\n#define JPH_VERSION_PATCH 0\nnamespace JPH {}\n";
        let good_funcs = "JPC_API void JPC_PhysicsSystem_new();\n";
        assert!(check_baseline_vendor_markers(good_core, good_funcs).is_ok());
        let tampered_core = good_core.replace("MINOR 3", "MINOR 6");
        let e = check_baseline_vendor_markers(&tampered_core, good_funcs).unwrap_err();
        assert!(matches!(e, AbError::BaselineVendorTampered(_)));
        let renamed_core = format!("{good_core}// namespace JPH56 {{}}\n");
        let e = check_baseline_vendor_markers(&renamed_core, good_funcs).unwrap_err();
        assert!(matches!(e, AbError::BaselineVendorTampered(_)));
        let renamed_funcs = good_funcs.replace("JPC_PhysicsSystem_new", "JPC56_PhysicsSystem_new");
        let e = check_baseline_vendor_markers(good_core, &renamed_funcs).unwrap_err();
        assert!(matches!(e, AbError::BaselineVendorTampered(_)));
        // 5.6 线标记核验(独立 vendor 完整性)。
        let core56 = "#define JPH_VERSION_MAJOR 5\n#define JPH_VERSION_MINOR 6\n#define JPH_VERSION_PATCH 0\nnamespace JPH56 {}\n";
        let funcs56 = "JPC56_API void JPC56_PhysicsSystem_new();\n";
        assert!(check_vendor56_markers(core56, funcs56).is_ok());
        assert!(matches!(
            check_vendor56_markers(good_core, funcs56).unwrap_err(),
            AbError::Vendor56MarkerMissing(_)
        ));
        // RED 臂 2:GPU compute 接权威提案——一律 typed Err。
        let e = connect_gpu_compute_authority("proposal: gpu rigid body authority").unwrap_err();
        assert!(matches!(e, AbError::GpuComputeAuthorityUsurpation(_)));
        // RED 臂 3:伪写 5.6 PASS/采纳字面——fail-closed;合法闭集放行。
        let e = validate_report_honesty("adopted_5_6_pass").unwrap_err();
        assert!(matches!(e, AbError::FakePassAttempt(_)));
        assert!(validate_report_honesty("maintain_5_3_default").is_ok());
        assert!(validate_report_honesty("pinned_5_3_on_failure").is_ok());
    }
}
