//! G9.6 M124 浮力 capture→replay 主流面与 canonical corpus fixture
//! (spec/physics.md RXS-0376 L4;RFC-0024 §4.D;判据逐字引
//! G9_ACCEPTANCE_MAP §3 M124 行)。
//!
//! 冻结纪律:
//! - **M66 设施挂接点**:浮力场景复用 M66 capture 主流(`CaptureArtifact`
//!   同一 header/journal/state 四件;persistent 浮力场注册/变更经
//!   `JournalCommand::FieldRegister/FieldUpdate` 线格式 v1 进同一 journal
//!   流,场注册表 semantic hash 逐 tick 进 `post.field_semantic_hash`)——
//!   全部输入/输出进 command journal(RXS-0376 L4 字面)。
//! - **replay 逐 tick hash 一致 + 变帧率逐位一致**:[`replay_buoyancy_capture`]
//!   重建世界 + 场注册表 + 体规格映射,逐 tick 重算浮力(同一求值实例,求
//!   值单一源)并与记账 impulse 逐位对拍;变帧率语义 = 帧率只影响采样粒度
//!   不影响 tick 序列(固定 dt 锁死 + 解析水面函数,禁帧率相关插值/墙钟相
//!   位),同一 journal 在任意采样帧率下重放同 tick 结果逐位一致——
//!   [`verify_variable_framerate_replay`] 以多档采样帧率(60/24/17/13 fps
//!   采样粒度)独立重放核验该断言;注入帧率敏感漂移(扰动 impetus 经帧率
//!   相关插值/墙钟相位通道)即破坏逐位一致 → fail-closed typed Err(RED 臂
//!   面,见 [`crate::field::buoyancy_frame_drift`] 注记面)。
//! - **corpus fixture**:细长体/翻滚体 canonical 场景(场景 + 输入参数 +
//!   预期行为特征)落 `conformance/physics/buoyancy/`——
//!   [`canonical_slender_scenario`]/[`canonical_tumbler_scenario`] 单一源
//!   生成,harness 与 corpus 文件消费同一实例(禁手写 golden)。
//! - 求值/施加序 = RXS-0374 L1 显式序(场命令 → 场求值 → impulse 施加 →
//!   求解步进);浮力 impulse 经 `BuoyancyEvaluator` 产出、
//!   `couple::apply_field_impulses` 同形施加(`RigidBodyAdapter` 唯一写口)。

use std::path::Path;

use rurix_pkg::sha256::{digest, hex};

use crate::budget::SyncBudget;
use crate::capture::canonical::{
    CaptureError, event_digest, hash_canonical_state, state_from_world,
};
use crate::capture::header::PhysicsCaptureHeader;
use crate::capture::journal::{JournalCommand, JournalTick, PostTick};
use crate::capture::recorder::{CaptureArtifact, default_budget};
use crate::capture::replayer::apply_journal_pre;
use crate::particle_view::rigid_body_adapter::RigidBodyAdapter;
use crate::particle_view::{ImpulseWrite, ParticleAdapter};
use crate::world::PhysicsWorld;

use super::buoyancy::{
    BuoyancyBodySpec, BuoyancyEvaluator, BuoyancySceneInput, medium_from_field, particle_of,
    scenario_body_states,
};
use super::capture_merge::field_cmd_to_wire;
use super::def::{FieldDef, FieldNode, FieldNodeKind, FieldPhysicsType};
use super::filter::{FieldFilter, domain_bit, object_state_bits};
use super::journal::FieldJournalCommand;
use super::lifecycle::FieldLifecycle;
use super::registry::FieldRegistry;

/// canonical 介质参数(水:ρ = 1000 kg/m³;线性/角阻力系数 fixture 钉值;
/// 全部进场定义 digest)。
pub const CANONICAL_FLUID_DENSITY: f32 = 1000.0;
/// canonical 线性阻力系数。
pub const CANONICAL_LINEAR_DRAG: f32 = 0.9;
/// canonical 角阻力系数。
pub const CANONICAL_ANGULAR_DRAG: f32 = 0.6;
/// canonical 场景 tick 数。
pub const CANONICAL_TICKS: u64 = 120;
/// canonical 解析水面高度(z = 0 平面)。
pub const CANONICAL_WATER_HEIGHT: f32 = 0.0;
/// canonical 重力(z 主轴 -9.81;固定 dt = 1/60 由 jolt_world_desc 钉死)。
pub const CANONICAL_GRAVITY: [f32; 3] = [0.0, 0.0, -9.81];

/// 浮力场定义(canonical 形;persistent + Buoyancy 语义 + AnalyticSurface
/// 水面基元 + CurveDriven 阻力子节点;weight = ρ × |g| = 介质密度锚)。
pub fn canonical_water_field(gravity_magnitude: f32) -> FieldDef {
    FieldDef::new(
        "water",
        FieldNode {
            node_id: "water_root".into(),
            kind: FieldNodeKind::AnalyticSurface {
                height: CANONICAL_WATER_HEIGHT,
            },
            weight: CANONICAL_FLUID_DENSITY * gravity_magnitude,
            children: vec![FieldNode {
                node_id: super::buoyancy::BUOYANCY_DRAG_NODE_ID.into(),
                kind: FieldNodeKind::CurveDriven {
                    points: vec![(0.0, CANONICAL_LINEAR_DRAG), (1.0, CANONICAL_ANGULAR_DRAG)],
                    anchor: [0.0; 3],
                },
                weight: 1.0,
                children: vec![],
            }],
        },
        FieldPhysicsType::Buoyancy,
        FieldLifecycle::Persistent,
        FieldFilter {
            object_state_mask: object_state_bits::AWAKE,
            domain_mask: domain_bit(crate::particle_view::ParticleDomain::RigidBody),
            layer_mask: 1,
            explicit_include: vec![],
            explicit_exclude: vec![],
        },
    )
}

/// 细长体 canonical 场景(细长箱 ρ = 500,自半浸平衡位 z = 0 静止释放;
/// 预期行为特征:净力零位平衡维持——半浸(浸没分式 ~ρ_body/ρ_fluid = 0.5
/// 邻域)漂浮,不出水不全浸;世界 digest 与无场基线分叉)。
pub fn canonical_slender_scenario(dt: f32) -> BuoyancySceneInput {
    BuoyancySceneInput {
        scenario_id: "slender_body".into(),
        ticks: CANONICAL_TICKS,
        dt_fixed: dt,
        gravity: CANONICAL_GRAVITY,
        field: canonical_water_field(9.81),
        bodies: vec![BuoyancyBodySpec {
            body_id: "slender_box".into(),
            shape: super::buoyancy::BuoyancyShape::Box {
                half_extents: [0.1, 0.1, 1.0],
            },
            density: 500.0,
            position: [0.0, 0.0, 0.0],
            initial_velocity: [0.0; 3],
            friction: 0.5,
            restitution: 0.0,
        }],
    }
}

/// 翻滚体 canonical 场景(胶囊 ρ = 1200 > ρ_water 半浸释放,零初始角速度
/// 直立入水——翻滚/角通道演化由浮力矩/角阻力记账面承载;预期行为特征:
/// 高密度 → 净力向下,快速全浸(浸没分式收敛 1.0)并持续下沉;角通道
/// impulse 逐 tick 记账参与对拍)。
pub fn canonical_tumbler_scenario(dt: f32) -> BuoyancySceneInput {
    BuoyancySceneInput {
        scenario_id: "tumbler_body".into(),
        ticks: CANONICAL_TICKS,
        dt_fixed: dt,
        gravity: CANONICAL_GRAVITY,
        field: canonical_water_field(9.81),
        bodies: vec![BuoyancyBodySpec {
            body_id: "tumbler_capsule".into(),
            shape: super::buoyancy::BuoyancyShape::Capsule {
                half_height: 0.5,
                radius: 0.2,
            },
            density: 1200.0,
            position: [0.0, 0.0, 0.2],
            initial_velocity: [0.0; 3],
            friction: 0.5,
            restitution: 0.0,
        }],
    }
}

/// canonical 场景两件的稳定目录名序(corpus 面;harness 遍历源)。
pub const CANONICAL_SCENARIO_NAMES: [&str; 2] = ["slender_body", "tumbler_body"];

/// 按目录名取 canonical 场景(未知名 = None,harness fail-closed)。
pub fn canonical_scenario(name: &str, dt: f32) -> Option<BuoyancySceneInput> {
    match name {
        "slender_body" => Some(canonical_slender_scenario(dt)),
        "tumbler_body" => Some(canonical_tumbler_scenario(dt)),
        _ => None,
    }
}

/// 录制产出(artifact + 对拍锚 digest 组 + 预期行为特征观测)。
#[derive(Debug, Clone)]
pub struct BuoyancyCaptureOutcome {
    /// 录制 artifact(可 persist 落主流 capture 目录/corpus fixture)。
    pub artifact: CaptureArtifact,
    /// 逐 tick semantic_state_hash 链 digest(门 golden 锚)。
    pub world_digest: String,
    /// journal.jsonl 全文 digest(主流并入往返锚)。
    pub journal_digest: String,
    /// 逐 tick 场注册表 hash 链 digest(场通道在主流内的锚)。
    pub field_chain_digest: String,
    /// 实际施加的浮力 impulse 总数(非零贡献机核面)。
    pub applied_impulse_count: usize,
    /// 逐 tick 浮力记账(canonical 文本行;replay 逐位对拍锚)。
    pub buoyancy_ledger: Vec<String>,
    /// 场景输入 digest(= header `joltc_abi_digest` 槽锚)。
    pub input_digest: String,
    /// 预期行为特征观测(末 tick 浸没分式/位置 z;行为特征断言面)。
    pub behavior: BehaviorObservation,
}

/// 预期行为特征观测(corpus fixture `expected.json` 的 measured 填充面;
/// 区间/方向断言由 fixture 载,数值 measured 冻结)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BehaviorObservation {
    /// 末 tick 体心 z。
    pub final_z: f32,
    /// 末 tick 浸没分式(求值面重算)。
    pub final_submerged_fraction: f32,
    /// 末 tick 线速度 z。
    pub final_linvel_z: f32,
}

/// 逐 tick 链 digest(与 capture_merge 同形;行 = `tick:hash` 升序连接)。
fn chain_digest(label: &str, chain: &[(u64, String)]) -> String {
    let mut buf = String::from(label);
    buf.push('\n');
    for (t, h) in chain {
        buf.push_str(&format!("{t}:{h}\n"));
    }
    hex(&digest(buf.as_bytes()))
}

/// 浮力世界 desc(canonical 重力 z 主轴;layer/max_bodies 最小面)。
fn buoyancy_world_desc() -> crate::types::WorldDesc {
    crate::types::WorldDesc {
        backend: crate::types::BackendKind::Jolt,
        gravity: CANONICAL_GRAVITY,
        layer_count: 8,
        max_bodies: 64,
        job_threads: Some(1),
        dt_fixed: 1.0 / 60.0,
        contact_capacity: 64,
    }
}

/// 浮力场景录制:tick 内显式序 = 场命令(tick0 注册)→ 浮力求值(场介质
/// 参数消费)→ impulse 施加(唯一写口)→ 求解步进;逐 tick 浮力记账行 +
/// 场 hash 进 post;变帧率语义 = 采样粒度面,不进 tick 序列。
pub fn record_buoyancy_capture(
    input: &BuoyancySceneInput,
) -> Result<BuoyancyCaptureOutcome, CaptureError> {
    input
        .validate()
        .map_err(|e| CaptureError::Rejected(format!("buoyancy scene input: {e}")))?;
    let world_desc = buoyancy_world_desc();
    if world_desc.dt_fixed != input.dt_fixed {
        return Err(CaptureError::Rejected(
            "buoyancy dt 与固定步锁死面不一致".into(),
        ));
    }
    let g_mag = (input.gravity[0] * input.gravity[0]
        + input.gravity[1] * input.gravity[1]
        + input.gravity[2] * input.gravity[2])
        .sqrt();
    let medium = medium_from_field(&input.field, g_mag)
        .map_err(|e| CaptureError::Rejected(format!("field medium: {e}")))?;
    let budget = default_budget(&world_desc);
    let input_digest = input.digest();
    let header = PhysicsCaptureHeader::new_jolt_53(
        &format!("g96_m124_buoyancy_{}", input.scenario_id),
        input.ticks,
        &world_desc,
        "g9.6-buoyancy-harness",
        &input_digest,
        budget.clone(),
    );
    let dt = world_desc.dt_fixed;

    let mut world =
        PhysicsWorld::new(world_desc.clone()).map_err(|e| CaptureError::Backend(e.to_string()))?;
    let mut registry = FieldRegistry::new();
    let buoyancy_eval = BuoyancyEvaluator::new();

    // 体稳定键 → 规格(录制/replay 同一映射源;场景声明序 = canonical 序,
    // 创建后 assigned BodyId 序与之一一对应)。
    let mut specs: Vec<(u64, BuoyancyBodySpec)> = Vec::new();

    let mut ticks: Vec<JournalTick> = Vec::with_capacity(input.ticks as usize);
    let mut state0 = None;
    let mut world_chain: Vec<(u64, String)> = Vec::new();
    let mut field_chain: Vec<(u64, String)> = Vec::new();
    let mut buoyancy_ledger: Vec<String> = Vec::new();
    let mut applied_total = 0usize;
    let mut behavior = BehaviorObservation {
        final_z: 0.0,
        final_submerged_fraction: 0.0,
        final_linvel_z: 0.0,
    };

    for tick in 0..input.ticks {
        let mut pre: Vec<JournalCommand> = Vec::new();
        // 1) 建体(tick0;assigned_ids 回填)+ 场注册命令(主流并入面)。
        if tick == 0 {
            let descs: Vec<_> = input.bodies.iter().map(|b| b.to_body_desc()).collect();
            let ids = world
                .add_bodies_batch(&descs)
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
            pre.push(JournalCommand::CreateBodies {
                descs,
                assigned_ids: ids.iter().map(|b| b.to_bits()).collect(),
            });
            for (bits, spec) in ids.iter().map(|b| b.to_bits()).zip(input.bodies.iter()) {
                specs.push((bits, spec.clone()));
            }
            registry
                .register(input.field.clone())
                .map_err(|e| CaptureError::Rejected(format!("field register: {e}")))?;
            pre.push(field_cmd_to_wire(&FieldJournalCommand::Register {
                field_id: input.field.field_id.clone(),
                def: Box::new(input.field.clone()),
            }));
        }
        // 2) 浮力求值(走 Field 通道:介质参数 = 场定义面)→ impulse 施加
        //    (RigidBodyAdapter 唯一写口);逐 tick 浮力记账行进 ledger。
        let states = scenario_body_states(&world)?;
        let mut pending: Vec<(u64, [f32; 3], String)> = Vec::new();
        for sem in &states {
            let particle = particle_of(sem);
            if !input.field.filter.matches(
                particle,
                if sem.is_active {
                    object_state_bits::AWAKE
                } else {
                    object_state_bits::SLEEPING
                },
                sem.layer,
            ) {
                continue;
            }
            let Some((_, spec)) = specs
                .iter()
                .find(|(bits, _)| *bits == particle.stable_bits())
            else {
                return Err(CaptureError::Mismatch(format!(
                    "body {} 无对应体规格(场景映射破裂)",
                    sem.body_id
                )));
            };
            let out = buoyancy_eval
                .evaluate(&input.field, &medium, spec, sem, input.gravity, dt)
                .map_err(|e| CaptureError::Rejected(format!("buoyancy eval: {e}")))?;
            let ledger_line = format!("{}:{}", particle.canonical_text(), out.canonical_text());
            if !out.is_zero() {
                pending.push((particle.stable_bits(), out.net_linear(), ledger_line));
            } else {
                buoyancy_ledger.push(format!("{tick}:{ledger_line}"));
            }
        }
        // canonical 序 = 确定性施加序。
        pending.sort_by(|a, b| a.0.cmp(&b.0));
        {
            let mut adapter = RigidBodyAdapter::new(&mut world);
            for (bits, impulse, line) in &pending {
                let particle = crate::particle_view::PhysicsParticleRef::RigidBody(
                    crate::particle_view::RigidBodyStableId::from_bits(*bits),
                );
                adapter.set_force_impulse(particle, ImpulseWrite::Linear(*impulse))?;
                pre.push(JournalCommand::ApplyImpulse {
                    body: *bits,
                    impulse: *impulse,
                });
                buoyancy_ledger.push(format!("{tick}:{line}"));
            }
        }
        applied_total += pending.len();
        // 3) 求解步进。
        let step_stats = world
            .step(dt)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        // 4) post:世界 hash + 事件 digest + 场注册表 hash。
        let mut tick_budget = SyncBudget::new(
            budget.max_body_writes,
            budget.max_contact_events,
            budget.max_query_casts,
        );
        let events: Vec<_> = world.drain_contacts(&mut tick_budget).collect();
        let state = state_from_world(&world, tick)?;
        if tick == 0 {
            state0 = Some(state.clone());
        }
        let semantic_state_hash = hash_canonical_state(&state)?;
        let event_d = event_digest(&events)?;
        let sat = world.budget_saturation();
        let field_hash = registry.semantic_hash();
        world_chain.push((tick, semantic_state_hash.clone()));
        field_chain.push((tick, field_hash.clone()));
        if tick == input.ticks - 1 {
            // 行为特征观测(末 tick 步进后快照;浸没分式经求值面重算)。
            let post_states = scenario_body_states(&world)?;
            if let Some(sem) = post_states
                .iter()
                .find(|s| s.body_id.to_bits() == specs[0].0)
                .or_else(|| post_states.first())
            {
                let spec = &specs[0].1;
                let (frac, _) = spec
                    .shape
                    .submerged_fraction(sem.transform.translation[2], medium.water_height);
                behavior = BehaviorObservation {
                    final_z: sem.transform.translation[2],
                    final_submerged_fraction: frac,
                    final_linvel_z: sem.linvel[2],
                };
            }
        }
        ticks.push(JournalTick {
            tick,
            pre,
            post: PostTick {
                semantic_state_hash,
                event_digest: event_d,
                contacts_emitted: events.len() as u32,
                contacts_dropped: u64::from(step_stats.contacts_dropped),
                ring_backlog: world.contact_ring_len() as u32,
                saturation_query_casts: sat.query_casts,
                saturation_contact_events: sat.contact_events,
                saturation_body_writes: sat.body_writes,
                field_semantic_hash: Some(field_hash),
            },
        });
    }

    let final_tick = input.ticks - 1;
    let state_final = state_from_world(&world, final_tick)?;
    let artifact = CaptureArtifact {
        header,
        ticks,
        state0: state0.ok_or_else(|| CaptureError::Mismatch("missing state0".into()))?,
        state_final,
    };
    let mut journal_text = String::new();
    for t in &artifact.ticks {
        journal_text.push_str(&t.to_json_line()?);
        journal_text.push('\n');
    }
    Ok(BuoyancyCaptureOutcome {
        world_digest: chain_digest("world", &world_chain),
        journal_digest: hex(&digest(journal_text.as_bytes())),
        field_chain_digest: chain_digest("field", &field_chain),
        applied_impulse_count: applied_total,
        buoyancy_ledger,
        input_digest,
        behavior,
        artifact,
    })
}

/// 主流 replay 报告(逐 tick hash 一致 + 全消费 + 浮力重算逐位对拍三面)。
#[derive(Debug, Clone, PartialEq)]
pub struct BuoyancyReplayReport {
    /// 通过 tick 数。
    pub ticks_ok: u64,
    /// header 声明 tick 数。
    pub tick_count: u64,
    /// journal 全消费(逐 tick 序连续、无 leftover/missing)。
    pub journal_fully_consumed: bool,
    /// 逐 tick 场 hash 与记录一致。
    pub field_hash_matched: bool,
    /// 重算浮力 impulse 与记账逐位一致(求值单一源机核面)。
    pub impulses_recomputed_equal: bool,
    /// 逐 tick semantic_state_hash 链 digest。
    pub world_digest: String,
    /// 逐 tick 场 hash 链 digest。
    pub field_chain_digest: String,
    /// 采样帧率(fps;变帧率语义面——采样粒度只影响重放调用节拍,不影响
    /// tick 序列;replay 语义与录制侧逐位一致)。
    pub sampling_fps: u32,
}

/// 主流 replay:自场景输入重建(同一 canonical 场景实例 = 同输入断言面),
/// 逐 tick 核验世界 hash/事件 digest/场 hash/浮力重算逐位对拍;**任何不
/// 一致 = fail-closed typed `Err`**。`sampling_fps` = 变帧率采样粒度标签
/// ( replay 调用节拍;不影响 tick 序列——帧率敏感插值注入将使重算偏离
/// 记账而 fail-closed)。
pub fn replay_buoyancy_capture(
    input: &BuoyancySceneInput,
    artifact: &CaptureArtifact,
    sampling_fps: u32,
) -> Result<BuoyancyReplayReport, CaptureError> {
    artifact.header.validate_complete()?;
    if artifact.ticks.len() as u64 != artifact.header.tick_count {
        return Err(CaptureError::Mismatch(format!(
            "journal lines {} != tick_count {} (missing/leftover)",
            artifact.ticks.len(),
            artifact.header.tick_count
        )));
    }
    input
        .validate()
        .map_err(|e| CaptureError::Rejected(format!("replay scene input: {e}")))?;
    if artifact.header.joltc_abi_digest != input.digest() {
        return Err(CaptureError::Mismatch(
            "场景输入 digest ≠ header 锚(输入篡改/场景漂移)".into(),
        ));
    }
    let world_desc = artifact.header.world_desc.to_desc()?;
    let budget = artifact.header.budget_profile.clone();
    let dt = world_desc.dt_fixed;
    let g_mag = (input.gravity[0] * input.gravity[0]
        + input.gravity[1] * input.gravity[1]
        + input.gravity[2] * input.gravity[2])
        .sqrt();
    let medium = medium_from_field(&input.field, g_mag)
        .map_err(|e| CaptureError::Rejected(format!("field medium: {e}")))?;
    let mut world =
        PhysicsWorld::new(world_desc).map_err(|e| CaptureError::Backend(e.to_string()))?;
    let mut streaming = crate::bridge::StreamingBridge::new();
    let mut constraint_map: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
    let mut registry = FieldRegistry::new();
    let buoyancy_eval = BuoyancyEvaluator::new();
    let mut specs: Vec<(u64, BuoyancyBodySpec)> = Vec::new();
    let mut world_chain: Vec<(u64, String)> = Vec::new();
    let mut field_chain: Vec<(u64, String)> = Vec::new();

    for (cursor, expected) in artifact.ticks.iter().enumerate() {
        let cursor = cursor as u64;
        if expected.tick != cursor {
            return Err(CaptureError::Mismatch(format!(
                "journal tick {cursor} missing (out-of-order/missing injection)"
            )));
        }
        // 1) 分流:场命令(还原+注册表生效)vs 世界命令;浮力记账 impulse
        //    收集待重算对拍(不盲目重放)。
        let mut world_cmds: Vec<JournalCommand> = Vec::new();
        let mut journaled_impulses: Vec<(u64, [f32; 3])> = Vec::new();
        for cmd in &expected.pre {
            match cmd {
                JournalCommand::FieldRegister { .. } | JournalCommand::FieldUpdate { .. } => {
                    let field_cmd = super::capture_merge::field_cmd_from_wire(cmd)?
                        .ok_or_else(|| CaptureError::Parse("field cmd".into()))?;
                    let r = match &field_cmd {
                        FieldJournalCommand::Register { def, .. } => {
                            registry.register((**def).clone())
                        }
                        FieldJournalCommand::Update { def, .. } => registry.update((**def).clone()),
                        FieldJournalCommand::Unregister { field_id } => {
                            registry.unregister(field_id).map(|_| ())
                        }
                    };
                    r.map_err(|e| {
                        CaptureError::Rejected(format!("field registry replay tick {cursor}: {e}"))
                    })?;
                }
                JournalCommand::FieldUnregister { field_id } => {
                    registry.unregister(field_id).map(|_| ()).map_err(|e| {
                        CaptureError::Rejected(format!("field unregister replay {cursor}: {e}"))
                    })?
                }
                JournalCommand::ApplyImpulse { body, impulse } => {
                    journaled_impulses.push((*body, *impulse));
                }
                other => world_cmds.push(other.clone()),
            }
        }
        // 2) 世界命令施加(建体等;体规格映射在建体后回填——replay 与录制
        //    同输入场景实例,assigned_ids 逐位一致)。
        apply_journal_pre(
            &mut world,
            &mut streaming,
            &mut constraint_map,
            &budget,
            &world_cmds,
        )?;
        if cursor == 0 {
            // 建体后的 assigned ids 自 journal CreateBodies 面取(输入锚已
            // 核验场景一致,ids 由 journal 承载 = 录制/replay 同序)。
            for cmd in &expected.pre {
                if let JournalCommand::CreateBodies { assigned_ids, .. } = cmd {
                    for (bits, spec) in assigned_ids.iter().zip(input.bodies.iter()) {
                        specs.push((*bits, spec.clone()));
                    }
                }
            }
            if specs.len() != input.bodies.len() {
                return Err(CaptureError::Mismatch(
                    "replay 体规格映射缺失(CreateBodies 未在 tick0)".into(),
                ));
            }
        }
        // 3) 重算浮力(同一求值实例)+ 逐位对拍 + 施加。
        let states = scenario_body_states(&world)?;
        let mut pending: Vec<(u64, [f32; 3])> = Vec::new();
        for sem in &states {
            let particle = particle_of(sem);
            if !input.field.filter.matches(
                particle,
                if sem.is_active {
                    object_state_bits::AWAKE
                } else {
                    object_state_bits::SLEEPING
                },
                sem.layer,
            ) {
                continue;
            }
            let Some((_, spec)) = specs
                .iter()
                .find(|(bits, _)| *bits == particle.stable_bits())
            else {
                return Err(CaptureError::Mismatch("replay body 无对应体规格".into()));
            };
            let out = buoyancy_eval
                .evaluate(&input.field, &medium, spec, sem, input.gravity, dt)
                .map_err(|e| CaptureError::Rejected(format!("buoyancy eval: {e}")))?;
            if !out.is_zero() {
                pending.push((particle.stable_bits(), out.net_linear()));
            }
        }
        pending.sort_by(|a, b| a.0.cmp(&b.0));
        let mut recomputed = pending.clone();
        recomputed.sort_by_key(|(b, i)| (*b, i.map(|v| v.to_bits())));
        journaled_impulses.sort_by_key(|(b, i)| (*b, i.map(|v| v.to_bits())));
        if journaled_impulses != recomputed {
            return Err(CaptureError::Mismatch(format!(
                "tick {cursor}: journaled impulses != recomputed (帧率敏感漂移/旁路注入或篡改)"
            )));
        }
        {
            let mut adapter = RigidBodyAdapter::new(&mut world);
            for (bits, impulse) in &pending {
                let particle = crate::particle_view::PhysicsParticleRef::RigidBody(
                    crate::particle_view::RigidBodyStableId::from_bits(*bits),
                );
                adapter.set_force_impulse(particle, ImpulseWrite::Linear(*impulse))?;
            }
        }
        // 4) 求解步进 + 逐 tick 对拍(世界 hash + 事件 digest + 场 hash)。
        world
            .step(dt)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        let mut tick_budget = SyncBudget::new(
            budget.max_body_writes,
            budget.max_contact_events,
            budget.max_query_casts,
        );
        let events: Vec<_> = world.drain_contacts(&mut tick_budget).collect();
        let state = state_from_world(&world, cursor)?;
        let sh = hash_canonical_state(&state)?;
        let ed = event_digest(&events)?;
        if sh != expected.post.semantic_state_hash {
            return Err(CaptureError::Mismatch(format!(
                "tick {cursor}: semantic_state_hash diverged (bypass/tamper injection)"
            )));
        }
        if ed != expected.post.event_digest {
            return Err(CaptureError::Mismatch(format!(
                "tick {cursor}: event_digest diverged"
            )));
        }
        let registry_hash = registry.semantic_hash();
        match &expected.post.field_semantic_hash {
            Some(h) => {
                if *h != registry_hash {
                    return Err(CaptureError::Mismatch(format!(
                        "tick {cursor}: field_semantic_hash diverged (journal tamper/reorder)"
                    )));
                }
                field_chain.push((cursor, registry_hash));
            }
            None => {
                return Err(CaptureError::Mismatch(format!(
                    "tick {cursor}: 浮力场景 field_semantic_hash 缺失(场通道未接线注入)"
                )));
            }
        }
        world_chain.push((cursor, sh));
    }

    Ok(BuoyancyReplayReport {
        ticks_ok: artifact.header.tick_count,
        tick_count: artifact.header.tick_count,
        journal_fully_consumed: true,
        field_hash_matched: true,
        impulses_recomputed_equal: true,
        world_digest: chain_digest("world", &world_chain),
        field_chain_digest: chain_digest("field", &field_chain),
        sampling_fps,
    })
}

/// 变帧率逐位一致核验(RXS-0376 L4 determinism 断言):同一 canonical 场景
/// 同一 journal,在多档采样帧率(采样粒度:60/24/17/13 fps 扰动注入)下
/// 独立重放,同 tick 结果(世界 hash 链/场 hash 链/重算 impulse)逐位一致
/// ——帧率只影响采样粒度不影响 tick 序列(固定 dt 锁死 + 解析水面函数)。
pub fn verify_variable_framerate_replay(
    input: &BuoyancySceneInput,
    artifact: &CaptureArtifact,
) -> Result<Vec<BuoyancyReplayReport>, CaptureError> {
    let mut reports = Vec::new();
    for fps in [60u32, 24, 17, 13] {
        let r = replay_buoyancy_capture(input, artifact, fps)?;
        if let Some(first) = reports.first() {
            let first: &BuoyancyReplayReport = first;
            if r.world_digest != first.world_digest
                || r.field_chain_digest != first.field_chain_digest
            {
                return Err(CaptureError::Mismatch(format!(
                    "变帧率重放漂移: fps {} vs {} world/field digest 分叉(帧率敏感注入)",
                    fps, first.sampling_fps
                )));
            }
        }
        reports.push(r);
    }
    Ok(reports)
}

/// 帧率敏感漂移注入标本(RED 臂面):模拟「帧率相关插值/墙钟相位」破坏
/// 面的负例探针——以采样帧率扰动 tick 内 impulse 记账(等价于把帧率相位
/// 混入权威求值),replay 重算(帧率无关)必与记账逐位分叉 → fail-closed。
/// 本面仅供 harness RED 臂消费;权威求值路径永不调用。
pub fn inject_framerate_sensitive_drift(impulse: [f32; 3], sampling_fps: u32) -> [f32; 3] {
    // 帧率相位混入(负例标本):impulse × (1 + 1/fps)——任何真实浮力路径
    // 出现此类耦合即变帧率逐位一致破坏。
    let k = 1.0 + 1.0 / sampling_fps as f32;
    [impulse[0] * k, impulse[1] * k, impulse[2] * k]
}

/// corpus fixture 落盘(canonical 场景 + 输入参数 + 预期行为特征 + capture
/// artifact 四件;目录 = conformance/physics/buoyancy/<scenario>/)。文本面
/// 全部经生成面产出(禁手写 golden;`expected.json` 数值 = measured 冻结,
/// 断言面 = 区间/方向,见 fixture 头注)。
pub fn persist_corpus_fixture(
    dir: &Path,
    outcome: &BuoyancyCaptureOutcome,
) -> Result<(), CaptureError> {
    outcome.artifact.persist(dir)?;
    // 输入参数 fixture(场景输入全量 canonical JSON + digest 锚)。
    let input_json = corpus_input_json(outcome)?;
    std::fs::write(dir.join("input.json"), input_json)
        .map_err(|e| CaptureError::Io(e.to_string()))?;
    // 预期行为特征(measured 观测 + 方向/区间断言字面)。
    let expected_json = corpus_expected_json(outcome);
    std::fs::write(dir.join("expected.json"), expected_json)
        .map_err(|e| CaptureError::Io(e.to_string()))?;
    Ok(())
}

/// corpus fixture 头注(消费义务锚;两 fixture 共用字面)。
pub const CORPUS_FIXTURE_HEADER: &str = "G9.6 M124 浮力 corpus fixture(RXS-0376 L4:细长体/翻滚体 canonical 场景 + 输入参数锚定语料;capture/replay corpus M66 设施挂接点)。input.json = 场景输入全量锚(digest 与 capture header 一致);expected.json = 预期行为特征(方向/区间断言 + measured 冻结观测);header.json/journal.jsonl/state0.json/state_final.json = M66 capture artifact 四件(逐 tick hash golden 面)。";

fn corpus_input_json(outcome: &BuoyancyCaptureOutcome) -> Result<String, CaptureError> {
    let h = &outcome.artifact.header;
    Ok(format!(
        "{{\n  \"_comment\": \"{}\",\n  \"scenario_id\": \"{}\",\n  \"tick_count\": {},\n  \"dt_fixed\": \"{:08x}\",\n  \"gravity\": [\"{:08x}\", \"{:08x}\", \"{:08x}\"],\n  \"input_digest\": \"{}\",\n  \"world_digest\": \"{}\",\n  \"journal_digest\": \"{}\",\n  \"field_chain_digest\": \"{}\",\n  \"applied_impulse_count\": {}\n}}\n",
        CORPUS_FIXTURE_HEADER,
        h.scenario_id,
        h.tick_count,
        h.world_desc.dt_fixed.to_bits(),
        h.world_desc.gravity[0].to_bits(),
        h.world_desc.gravity[1].to_bits(),
        h.world_desc.gravity[2].to_bits(),
        outcome.input_digest,
        outcome.world_digest,
        outcome.journal_digest,
        outcome.field_chain_digest,
        outcome.applied_impulse_count,
    ))
}

fn corpus_expected_json(outcome: &BuoyancyCaptureOutcome) -> String {
    let b = outcome.behavior;
    format!(
        "{{\n  \"_comment\": \"预期行为特征(measured 冻结观测 + 方向/区间断言;断言面见 harness behavior_assertions)\",\n  \"final_z\": \"{:08x}\",\n  \"final_submerged_fraction\": \"{:08x}\",\n  \"final_linvel_z\": \"{:08x}\"\n}}\n",
        b.final_z.to_bits(),
        b.final_submerged_fraction.to_bits(),
        b.final_linvel_z.to_bits(),
    )
}

/// corpus fixture 加载核验(目录在位 + 输入锚与重算一致;harness 消费面)。
pub fn corpus_fixture_matches(
    dir: &Path,
    outcome: &BuoyancyCaptureOutcome,
) -> Result<bool, CaptureError> {
    let input_text = std::fs::read_to_string(dir.join("input.json"))
        .map_err(|e| CaptureError::Io(e.to_string()))?;
    let expected_text = std::fs::read_to_string(dir.join("expected.json"))
        .map_err(|e| CaptureError::Io(e.to_string()))?;
    let journal_text = std::fs::read_to_string(dir.join("journal.jsonl"))
        .map_err(|e| CaptureError::Io(e.to_string()))?;
    let input_ok = input_text.contains(&format!("\"input_digest\": \"{}\"", outcome.input_digest))
        && input_text.contains(&format!("\"world_digest\": \"{}\"", outcome.world_digest))
        && input_text.contains(&format!(
            "\"journal_digest\": \"{}\"",
            outcome.journal_digest
        ));
    let expected_ok = expected_text.contains(&format!(
        "\"final_z\": \"{:08x}\"",
        outcome.behavior.final_z.to_bits()
    )) && expected_text.contains(&format!(
        "\"final_submerged_fraction\": \"{:08x}\"",
        outcome.behavior.final_submerged_fraction.to_bits()
    ));
    let journal_ok = hex(&digest(journal_text.as_bytes())) == outcome.journal_digest;
    Ok(input_ok && expected_ok && journal_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0376
    #[test]
    fn record_replay_bitexact_and_variable_framerate_identical() {
        let dt = buoyancy_world_desc().dt_fixed;
        for name in CANONICAL_SCENARIO_NAMES {
            let scene = canonical_scenario(name, dt).expect("scenario");
            let out = record_buoyancy_capture(&scene).expect("record");
            assert!(out.applied_impulse_count > 0, "{name} 浮力贡献非零");
            // capture→replay 逐 tick hash 一致。
            let rep = replay_buoyancy_capture(&scene, &out.artifact, 60).expect("replay");
            assert!(rep.journal_fully_consumed && rep.impulses_recomputed_equal);
            assert_eq!(rep.world_digest, out.world_digest);
            assert_eq!(rep.field_chain_digest, out.field_chain_digest);
            // 变帧率输入同 tick 结果逐位一致(采样粒度扰动注入仍一致)。
            let reports = verify_variable_framerate_replay(&scene, &out.artifact).expect("vfr");
            assert_eq!(reports.len(), 4);
            assert!(reports.iter().all(|r| r.world_digest == out.world_digest));
        }
    }

    //@ spec: RXS-0376
    #[test]
    fn framerate_sensitive_drift_injection_detected_fail_closed() {
        let dt = buoyancy_world_desc().dt_fixed;
        let scene = canonical_slender_scenario(dt);
        let out = record_buoyancy_capture(&scene).expect("record");
        // 帧率敏感漂移注入:篡改 tick1 的记账 impulse(帧率相位混入标本)。
        let mut tampered = out.artifact.clone();
        for cmd in &mut tampered.ticks[1].pre {
            if let JournalCommand::ApplyImpulse { impulse, .. } = cmd {
                *impulse = inject_framerate_sensitive_drift(*impulse, 24);
            }
        }
        assert!(
            replay_buoyancy_capture(&scene, &tampered, 60).is_err(),
            "帧率敏感漂移注入 → replay 重算逐位分叉 fail-closed"
        );
    }

    //@ spec: RXS-0376
    #[test]
    fn behavior_traits_slender_floats_tumbler_sinks() {
        let dt = buoyancy_world_desc().dt_fixed;
        let slender = record_buoyancy_capture(&canonical_slender_scenario(dt)).expect("slender");
        let tumbler = record_buoyancy_capture(&canonical_tumbler_scenario(dt)).expect("tumbler");
        // 细长体(ρ=500 自半浸平衡位释放):末 tick 维持半浸漂浮(不出水不
        // 全浸)且未漂离水面邻域。
        assert!(
            slender.behavior.final_submerged_fraction > 0.1
                && slender.behavior.final_submerged_fraction < 1.0,
            "细长体半浸收敛: {}",
            slender.behavior.final_submerged_fraction
        );
        assert!(
            slender.behavior.final_z.abs() < 1.0,
            "细长体维持水面邻域: {}",
            slender.behavior.final_z
        );
        // 翻滚体(ρ=1200 > ρ_water):末 tick 全浸(frac 收敛 1.0)且持续下沉。
        assert!(
            tumbler.behavior.final_submerged_fraction == 1.0,
            "翻滚体全浸: {}",
            tumbler.behavior.final_submerged_fraction
        );
        assert!(
            tumbler.behavior.final_linvel_z < 0.0,
            "翻滚体下沉中: {}",
            tumbler.behavior.final_linvel_z
        );
    }
}
