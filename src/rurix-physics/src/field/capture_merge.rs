//! 场 journal 并入 M66 capture 主流(spec/physics.md RXS-0374 L3;RFC-0024
//! §4.B2 + RFC-0021 §4.A1;判据逐字引 G9_ACCEPTANCE_MAP §2 M121/M122 行)。
//!
//! 冻结纪律:
//! - **同一 capture 目录与 journal 流**:persistent field 注册/注销/参数变更
//!   以 [`JournalCommand::FieldRegister`/`FieldUnregister`/`FieldUpdate`] 线
//!   格式进 M66 journal.jsonl 主流,不单开第二通道;逐 tick 场注册表
//!   semantic hash 记入 `post.field_semantic_hash`(legacy capture 恒 None,
//!   既有 corpus 字节 0-byte)。
//! - **往返兼容**:合并 journal 经 encode→decode 往返无损
//!   ([`field_cmd_to_wire`]/[`field_cmd_from_wire`] + canonical 字节回写相等
//!   + digest 锚);线格式版本化([`crate::capture::FIELD_COMMAND_WIRE_VERSION`],
//!     未知版本 fail-closed,显式迁移而非静默重解释)。
//! - **replay 逐 tick hash 一致**:[`replay_field_capture`] 重建世界 +
//!   `FieldRegistry`,逐 tick 核验 semantic_state_hash/event_digest/场 hash;
//!   场驱动 impulse 以 `ApplyImpulse` 记账,replay 经**同一求值实例**重算并
//!   逐位对拍(求值单一源机核面),再由耦合面施加而非盲目重放。
//! - journal 缺失/乱序/篡改注入即 fail-closed(typed `Err`,不静默充绿)。

use std::path::Path;

use rurix_pkg::sha256::{digest, hex};

use crate::budget::SyncBudget;
use crate::capture::canonical::{CaptureError, event_digest, hash_canonical_state, state_from_world};
use crate::capture::header::PhysicsCaptureHeader;
use crate::capture::journal::{JournalCommand, JournalTick, PostTick};
use crate::capture::recorder::{CaptureArtifact, default_budget};
use crate::capture::replayer::apply_journal_pre;
use crate::particle_view::rigid_body_adapter::RigidBodyAdapter;
use crate::particle_view::{ParticleSleepState, PhysicsParticleRef, rigid_body_ref};
use crate::types::{BodyDesc, BodyKind, MassProps, PhysicsTransform, ShapeDesc};
use crate::world::PhysicsWorld;

use super::couple::apply_field_impulses;
use super::def::FieldDef;
use super::eval::FieldEvaluator;
use super::journal::FieldJournalCommand;
use super::registry::FieldRegistry;

/// 场命令 → 主流线命令(载荷 = 完整定义**线格式 v1 JSON**(well-formed;
/// 骨架期冻结 canonical 字节 0-byte 不动)+ digest 锚〔冻结 canonical 前像〕)。
pub fn field_cmd_to_wire(cmd: &FieldJournalCommand) -> JournalCommand {
    match cmd {
        FieldJournalCommand::Register { field_id, def } => JournalCommand::FieldRegister {
            field_id: field_id.clone(),
            def_digest: def.digest(),
            def_json: def.wire_json(),
        },
        FieldJournalCommand::Unregister { field_id } => JournalCommand::FieldUnregister {
            field_id: field_id.clone(),
        },
        FieldJournalCommand::Update { field_id, def } => JournalCommand::FieldUpdate {
            field_id: field_id.clone(),
            def_digest: def.digest(),
            def_json: def.wire_json(),
        },
    }
}

/// 主流线命令 → 场命令(非场命令 → `Ok(None)`;场命令 = 线格式还原 +
/// digest 锚校验〔还原后重算冻结 canonical digest 对拍〕+ 注册表面
/// fail-closed)。
pub fn field_cmd_from_wire(
    cmd: &JournalCommand,
) -> Result<Option<FieldJournalCommand>, CaptureError> {
    fn parse_def(def_json: &str, def_digest: &str) -> Result<Box<FieldDef>, CaptureError> {
        let def = FieldDef::parse_wire_json(def_json)
            .map_err(|e| CaptureError::Parse(format!("field def wire: {e}")))?;
        if def.digest() != def_digest {
            return Err(CaptureError::Mismatch(format!(
                "field def digest anchor: {} != {def_digest}",
                def.digest()
            )));
        }
        Ok(Box::new(def))
    }
    match cmd {
        JournalCommand::FieldRegister {
            field_id,
            def_digest,
            def_json,
        } => Ok(Some(FieldJournalCommand::Register {
            field_id: field_id.clone(),
            def: parse_def(def_json, def_digest)?,
        })),
        JournalCommand::FieldUnregister { field_id } => {
            Ok(Some(FieldJournalCommand::Unregister {
                field_id: field_id.clone(),
            }))
        }
        JournalCommand::FieldUpdate {
            field_id,
            def_digest,
            def_json,
        } => Ok(Some(FieldJournalCommand::Update {
            field_id: field_id.clone(),
            def: parse_def(def_json, def_digest)?,
        })),
        _ => Ok(None),
    }
}

/// 场参与模式(canonical 录制场景臂)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldPresence {
    /// 活性场(权重 1.0 注册,ticks/2 变更 ×2,末 tick 注销)。
    Active,
    /// 场置零退化臂(注册同形但权重恒 0 → 零 impulse,世界演化须与
    /// 无场基线逐位一致)。
    Zeroed,
    /// 零匹配臂(注册同形但 filter = 默认空集 → 过滤默认空匹配 = 零影响
    /// 完整期重验面,RXS-0375 L2;场命令/场 hash 链照常进 journal)。
    NoMatch,
    /// 无场基线(零场命令;`field_semantic_hash` 恒 None)。
    Absent,
}

/// 完整期录制参数(canonical 耦合场景:单动态球 + 单 persistent 场)。
#[derive(Debug, Clone)]
pub struct FieldCaptureSpec {
    /// 场景 ID(进 header)。
    pub scenario_id: String,
    /// tick 数(≥2;canonical 门场景 = 8)。
    pub ticks: u64,
    /// 场参与模式。
    pub presence: FieldPresence,
}

/// 录制产出(artifact + 对拍锚 digest 组)。
#[derive(Debug, Clone)]
pub struct FieldCaptureOutcome {
    /// 录制 artifact(可 persist 落主流 capture 目录)。
    pub artifact: CaptureArtifact,
    /// 逐 tick semantic_state_hash 链 digest(门 golden 锚)。
    pub world_digest: String,
    /// journal.jsonl 全文 digest(主流并入往返锚)。
    pub journal_digest: String,
    /// 实际施加的场驱动 impulse 总数(非零贡献机核面)。
    pub applied_impulse_count: usize,
    /// 逐 tick 场注册表 hash 链 digest(场缺失臂对拍锚;无场 = 空链 digest)。
    pub field_chain_digest: String,
}

/// canonical 驱动场定义(径向衰减中心偏置 +x,原点处梯度非零;权重由臂定)。
fn canonical_drive_field(weight: f32) -> FieldDef {
    canonical_drive_field_filtered(
        weight,
        super::filter::FieldFilter {
            object_state_mask: super::filter::object_state_bits::AWAKE,
            domain_mask: super::filter::domain_bit(crate::particle_view::ParticleDomain::RigidBody),
            layer_mask: 1,
            explicit_include: vec![],
            explicit_exclude: vec![],
        },
    )
}

/// canonical 驱动场定义(过滤显式给;NoMatch 臂 = 默认空集 filter)。
fn canonical_drive_field_filtered(weight: f32, filter: super::filter::FieldFilter) -> FieldDef {
    FieldDef::new(
        "drive",
        crate::field::def::FieldNode {
            node_id: "drive_root".into(),
            kind: crate::field::def::FieldNodeKind::RadialFalloff {
                center: [1.0, 0.0, 0.0],
                radius: 50.0,
            },
            weight,
            children: vec![],
        },
        crate::field::def::FieldPhysicsType::LinearForce,
        super::lifecycle::FieldLifecycle::Persistent,
        filter,
    )
}

/// canonical 场景体(单动态球,layer 0,禁睡眠——过滤 AWAKE 匹配稳定)。
fn canonical_body_desc() -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Sphere { radius: 0.5 },
        layer: 0,
        mass_props: MassProps {
            mass: 1.0,
            friction: 0.5,
            restitution: 0.0,
            allow_sleep: false,
        },
        ccd: false,
        transform: PhysicsTransform::IDENTITY,
    }
}

/// 世界 → 粒子集(录制/replay 同一重建面;layer/is_active 取自
/// `BodySemantic` 快照,规范序 = 确定性面)。
fn scenario_particles(
    world: &PhysicsWorld,
) -> Result<Vec<(PhysicsParticleRef, ParticleSleepState, u32)>, CaptureError> {
    let snap = world
        .body_semantic_snapshot()
        .map_err(|e| CaptureError::Backend(e.to_string()))?;
    let mut out = Vec::with_capacity(snap.len());
    for sem in &snap {
        out.push((
            rigid_body_ref(sem.body_id),
            if sem.is_active {
                ParticleSleepState::Awake
            } else {
                ParticleSleepState::Sleeping
            },
            sem.layer,
        ));
    }
    Ok(out)
}

/// 逐 tick 链 digest(对拍锚;行 = `tick:hash` 升序连接)。
fn chain_digest(label: &str, chain: &[(u64, String)]) -> String {
    let mut buf = String::from(label);
    buf.push('\n');
    for (t, h) in chain {
        buf.push_str(&format!("{t}:{h}\n"));
    }
    hex(&digest(buf.as_bytes()))
}

/// 完整期录制:tick 内显式序 = 场命令(注册/变更/注销)→ 场求值 → impulse
/// 施加 → 求解步进;场命令与 impulse 同 journal 主流记账,场注册表 hash 逐
/// tick 进 post。
pub fn record_field_capture(spec: &FieldCaptureSpec) -> Result<FieldCaptureOutcome, CaptureError> {
    if spec.ticks < 2 {
        return Err(CaptureError::Rejected("field capture ticks >= 2".into()));
    }
    let world_desc = crate::capture::jolt_world_desc(16);
    let budget = default_budget(&world_desc);
    let header = PhysicsCaptureHeader::new_jolt_53(
        &spec.scenario_id,
        spec.ticks,
        &world_desc,
        "g9.6-field-coupling-harness",
        "g9.6-field-coupling-harness",
        budget.clone(),
    );
    let dt = world_desc.dt_fixed;
    let update_tick = spec.ticks / 2;
    let unregister_tick = spec.ticks - 1;

    let mut world =
        PhysicsWorld::new(world_desc.clone()).map_err(|e| CaptureError::Backend(e.to_string()))?;
    let mut registry = FieldRegistry::new();
    let evaluator = FieldEvaluator::new();

    let mut ticks: Vec<JournalTick> = Vec::with_capacity(spec.ticks as usize);
    let mut state0 = None;
    let mut world_chain: Vec<(u64, String)> = Vec::new();
    let mut field_chain: Vec<(u64, String)> = Vec::new();
    let mut applied_total = 0usize;

    for tick in 0..spec.ticks {
        let mut pre: Vec<JournalCommand> = Vec::new();
        // 1) 场命令(注册/变更/注销)先记账并生效。
        if tick == 0 {
            pre.push(JournalCommand::CreateBodies {
                descs: vec![canonical_body_desc()],
                assigned_ids: vec![], // assigned_ids 在创建后回填(见下)
            });
        }
        // 体创建须在耦合前完成——本 tick 先落 CreateBodies,再补场命令。
        if tick == 0 {
            let JournalCommand::CreateBodies { assigned_ids, .. } = &mut pre[0] else {
                return Err(CaptureError::Mismatch("create bodies slot".into()));
            };
            let ids = world
                .add_bodies_batch(&[canonical_body_desc()])
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
            *assigned_ids = ids.iter().map(|b| b.to_bits()).collect();
        }
        match spec.presence {
            FieldPresence::Active | FieldPresence::Zeroed | FieldPresence::NoMatch => {
                let base_weight = match spec.presence {
                    FieldPresence::Active | FieldPresence::NoMatch => 1.0,
                    FieldPresence::Zeroed => 0.0,
                    FieldPresence::Absent => unreachable!(),
                };
                let make_def = |w: f32| match spec.presence {
                    FieldPresence::NoMatch => {
                        canonical_drive_field_filtered(w, super::filter::FieldFilter::default())
                    }
                    _ => canonical_drive_field(w),
                };
                if tick == 0 {
                    let def = make_def(base_weight);
                    registry
                        .register(def.clone())
                        .map_err(|e| CaptureError::Rejected(format!("field register: {e}")))?;
                    pre.push(field_cmd_to_wire(&FieldJournalCommand::Register {
                        field_id: def.field_id.clone(),
                        def: Box::new(def),
                    }));
                } else if tick == update_tick {
                    let def = make_def(base_weight * 2.0);
                    registry
                        .update(def.clone())
                        .map_err(|e| CaptureError::Rejected(format!("field update: {e}")))?;
                    pre.push(field_cmd_to_wire(&FieldJournalCommand::Update {
                        field_id: def.field_id.clone(),
                        def: Box::new(def),
                    }));
                } else if tick == unregister_tick {
                    registry
                        .unregister("drive")
                        .map_err(|e| CaptureError::Rejected(format!("field unregister: {e}")))?;
                    pre.push(field_cmd_to_wire(&FieldJournalCommand::Unregister {
                        field_id: "drive".into(),
                    }));
                }
            }
            FieldPresence::Absent => {}
        }
        // 2) 场求值 → impulse 施加(写路径仅 impulse/force;置零/零匹配
        //    不产生 impulse,不记账)。
        let particles = scenario_particles(&world)?;
        let applied = {
            let mut adapter = RigidBodyAdapter::new(&mut world);
            apply_field_impulses(&mut adapter, &registry, &evaluator, &particles, dt)?
        };
        applied_total += applied.len();
        for (p, impulse) in &applied {
            pre.push(JournalCommand::ApplyImpulse {
                body: p.stable_bits(),
                impulse: *impulse,
            });
        }
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
        let field_hash = match spec.presence {
            FieldPresence::Absent => None,
            _ => Some(registry.semantic_hash()),
        };
        world_chain.push((tick, semantic_state_hash.clone()));
        if let Some(h) = &field_hash {
            field_chain.push((tick, h.clone()));
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
                field_semantic_hash: field_hash,
            },
        });
    }

    let final_tick = spec.ticks - 1;
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
    Ok(FieldCaptureOutcome {
        world_digest: chain_digest("world", &world_chain),
        journal_digest: hex(&digest(journal_text.as_bytes())),
        applied_impulse_count: applied_total,
        field_chain_digest: chain_digest("field", &field_chain),
        artifact,
    })
}

/// 完整期 replay 报告(逐 tick hash 一致 + 全消费 + 重算对拍三面)。
#[derive(Debug, Clone, PartialEq)]
pub struct FieldReplayReport {
    /// 通过 tick 数。
    pub ticks_ok: u64,
    /// header 声明 tick 数。
    pub tick_count: u64,
    /// journal 全消费(逐 tick 序连续、无 leftover/missing)。
    pub journal_fully_consumed: bool,
    /// 逐 tick 场 hash 与记录一致。
    pub field_hash_matched: bool,
    /// 重算 impulse 与记账 impulse 逐位一致(求值单一源机核面)。
    pub impulses_recomputed_equal: bool,
    /// 逐 tick semantic_state_hash 链 digest。
    pub world_digest: String,
    /// 逐 tick 场 hash 链 digest(无场 = 空链)。
    pub field_chain_digest: String,
}

/// 主流 replay:重建世界 + FieldRegistry,逐 tick 核验;**任何不一致 =
/// fail-closed typed `Err`**(缺失/乱序/篡改/重算偏离全红)。
pub fn replay_field_capture(dir: &Path) -> Result<FieldReplayReport, CaptureError> {
    let artifact = CaptureArtifact::load(dir)?;
    replay_field_artifact(&artifact)
}

/// artifact 内存面 replay(`replay_field_capture` 同语义;RED 臂注入用)。
pub fn replay_field_artifact(
    artifact: &CaptureArtifact,
) -> Result<FieldReplayReport, CaptureError> {
    artifact.header.validate_complete()?;
    if artifact.ticks.len() as u64 != artifact.header.tick_count {
        let lines = artifact.ticks.len() as u64;
        return Err(CaptureError::Mismatch(format!(
            "journal lines {lines} != tick_count {} (missing/leftover)",
            artifact.header.tick_count
        )));
    }
    let world_desc = artifact.header.world_desc.to_desc()?;
    let budget = artifact.header.budget_profile.clone();
    let dt = world_desc.dt_fixed;
    let mut world =
        PhysicsWorld::new(world_desc).map_err(|e| CaptureError::Backend(e.to_string()))?;
    let mut streaming = crate::bridge::StreamingBridge::new();
    let mut constraint_map: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
    let mut registry = FieldRegistry::new();
    let evaluator = FieldEvaluator::new();
    let mut world_chain: Vec<(u64, String)> = Vec::new();
    let mut field_chain: Vec<(u64, String)> = Vec::new();

    for (cursor, expected) in artifact.ticks.iter().enumerate() {
        let cursor = cursor as u64;
        if expected.tick != cursor {
            return Err(CaptureError::Mismatch(format!(
                "journal tick {} missing (out-of-order/missing injection)",
                cursor
            )));
        }
        // 1) 分流:场命令(还原+注册表生效)vs 世界命令。
        let mut world_cmds: Vec<JournalCommand> = Vec::new();
        let mut journaled_impulses: Vec<(u64, [f32; 3])> = Vec::new();
        let mut saw_field_cmd = false;
        for cmd in &expected.pre {
            if let Some(field_cmd) = field_cmd_from_wire(cmd)? {
                saw_field_cmd = true;
                let r = match &field_cmd {
                    FieldJournalCommand::Register { def, .. } => registry.register((**def).clone()),
                    FieldJournalCommand::Unregister { field_id } => {
                        registry.unregister(field_id).map(|_| ())
                    }
                    FieldJournalCommand::Update { def, .. } => registry.update((**def).clone()),
                };
                r.map_err(|e| {
                    CaptureError::Rejected(format!("field registry replay tick {cursor}: {e}"))
                })?;
            } else if let JournalCommand::ApplyImpulse { body, impulse } = cmd {
                // 场驱动 impulse 记账面:replay 不盲目施加,先收集与重算对拍。
                journaled_impulses.push((*body, *impulse));
            } else {
                world_cmds.push(cmd.clone());
            }
        }
        // 2) 世界命令(CreateBodies 等)先施加——耦合求值需要本 tick 起点
        //    世界态(录制侧同时序:建体 → 场命令 → 耦合 → 步进)。
        apply_journal_pre(
            &mut world,
            &mut streaming,
            &mut constraint_map,
            &budget,
            &world_cmds,
        )?;
        // 3) 重算耦合(同一求值实例)+ impulse 施加(写路径唯一口)。
        let particles = scenario_particles(&world)?;
        let recomputed = {
            let mut adapter = RigidBodyAdapter::new(&mut world);
            apply_field_impulses(&mut adapter, &registry, &evaluator, &particles, dt)?
        };
        let mut recomputed_pairs: Vec<(u64, [f32; 3])> = recomputed
            .iter()
            .map(|(p, i)| (p.stable_bits(), *i))
            .collect();
        // canonical 序对拍:f32 以位表示排序(位级一致断言面)。
        recomputed_pairs.sort_by_key(|(b, i)| (*b, i.map(|v| v.to_bits())));
        journaled_impulses.sort_by_key(|(b, i)| (*b, i.map(|v| v.to_bits())));
        if journaled_impulses != recomputed_pairs {
            return Err(CaptureError::Mismatch(format!(
                "tick {cursor}: journaled impulses != recomputed (single-source violation or tamper)"
            )));
        }
        // 4) 求解步进。
        world
            .step(dt)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        // 5) 逐 tick 对拍:世界 hash + 事件 digest + 场 hash。
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
                if saw_field_cmd {
                    return Err(CaptureError::Mismatch(format!(
                        "tick {cursor}: field command without field_semantic_hash"
                    )));
                }
                if !registry.is_empty() {
                    return Err(CaptureError::Mismatch(format!(
                        "tick {cursor}: registry non-empty but field hash absent"
                    )));
                }
            }
        }
        world_chain.push((cursor, sh));
    }

    Ok(FieldReplayReport {
        ticks_ok: artifact.header.tick_count,
        tick_count: artifact.header.tick_count,
        journal_fully_consumed: true,
        // 到达此处 = 逐 tick 场 hash 全等对拍通过、重算 impulse 逐位相等
        // (任一偏离已在循环内 fail-closed 返回)。
        field_hash_matched: true,
        impulses_recomputed_equal: true,
        world_digest: chain_digest("world", &world_chain),
        field_chain_digest: chain_digest("field", &field_chain),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(presence: FieldPresence) -> FieldCaptureSpec {
        FieldCaptureSpec {
            scenario_id: "g96_field_coupling_test".into(),
            ticks: 8,
            presence,
        }
    }

    //@ spec: RXS-0374
    #[test]
    fn wire_roundtrip_lossless_and_digest_anchored() {
        let def = canonical_drive_field(1.0);
        for cmd in [
            FieldJournalCommand::Register {
                field_id: def.field_id.clone(),
                def: Box::new(def.clone()),
            },
            FieldJournalCommand::Update {
                field_id: def.field_id.clone(),
                def: Box::new(def.clone()),
            },
            FieldJournalCommand::Unregister {
                field_id: def.field_id.clone(),
            },
        ] {
            let wire = field_cmd_to_wire(&cmd);
            let back = field_cmd_from_wire(&wire).expect("decode").expect("field cmd");
            assert_eq!(back, cmd, "encode→decode 往返无损");
        }
        // digest 锚篡改 fail-closed。
        let mut wire = field_cmd_to_wire(&FieldJournalCommand::Register {
            field_id: def.field_id.clone(),
            def: Box::new(def.clone()),
        });
        if let JournalCommand::FieldRegister { def_digest, .. } = &mut wire {
            *def_digest = "0".repeat(64);
        }
        assert!(field_cmd_from_wire(&wire).is_err(), "digest 锚篡改即 RED");
    }

    //@ spec: RXS-0374
    #[test]
    fn record_replay_mainstream_bitexact_and_degenerate_baseline() {
        // 主流并入:record → replay 逐 tick hash 一致;场置零退化臂世界
        // digest 与无场基线逐位一致;活性臂 ≠ 基线(场贡献非零)。
        let active = record_field_capture(&spec(FieldPresence::Active)).expect("record active");
        let zeroed = record_field_capture(&spec(FieldPresence::Zeroed)).expect("record zeroed");
        let absent = record_field_capture(&spec(FieldPresence::Absent)).expect("record absent");
        let nomatch = record_field_capture(&spec(FieldPresence::NoMatch)).expect("record nomatch");

        assert!(active.applied_impulse_count > 0, "场贡献非零");
        assert_eq!(zeroed.applied_impulse_count, 0, "场置零 → 零 impulse");
        assert_ne!(
            active.world_digest, absent.world_digest,
            "活性场必驱动力学响应"
        );
        assert_eq!(
            zeroed.world_digest, absent.world_digest,
            "场置零退化基线逐位一致"
        );
        // RXS-0375 L2 完整期重验:过滤默认空匹配 = 零影响(场注册但零
        // 匹配时世界状态 hash 与无 field 基线逐位一致;场命令/hash 链照常
        // 进 journal 主流)。
        assert_eq!(nomatch.applied_impulse_count, 0, "零匹配 → 零 impulse");
        assert_eq!(
            nomatch.world_digest, absent.world_digest,
            "过滤默认空匹配 = 零影响"
        );
        assert_ne!(
            nomatch.field_chain_digest, absent.field_chain_digest,
            "零匹配场仍全 journal 化(注册表 hash 链非空)"
        );

        for out in [&active, &zeroed, &absent, &nomatch] {
            let report = replay_field_artifact(&out.artifact).expect("replay");
            assert!(report.journal_fully_consumed);
            assert_eq!(report.world_digest, out.world_digest);
            assert_eq!(report.field_chain_digest, out.field_chain_digest);
        }
        assert!(replay_field_artifact(&active.artifact)
            .expect("replay")
            .impulses_recomputed_equal);
    }

    //@ spec: RXS-0374
    #[test]
    fn journal_missing_reorder_tamper_injections_fail_closed() {
        let active = record_field_capture(&spec(FieldPresence::Active)).expect("record");
        // 缺失注入:删末行 → RED。
        let mut missing = active.artifact.clone();
        missing.ticks.pop();
        assert!(replay_field_artifact(&missing).is_err());
        // 乱序注入:交换 tick4(Update)与 tick7(Unregister)的 pre → RED。
        let mut reordered = active.artifact.clone();
        reordered.ticks[4].pre = active.artifact.ticks[7].pre.clone();
        reordered.ticks[7].pre = active.artifact.ticks[4].pre.clone();
        assert!(replay_field_artifact(&reordered).is_err());
        // 篡改注入:翻 def_digest 一位 → RED。
        let mut tampered = active.artifact.clone();
        for cmd in &mut tampered.ticks[0].pre {
            if let JournalCommand::FieldRegister { def_digest, .. } = cmd {
                *def_digest = "f".repeat(64);
            }
        }
        assert!(replay_field_artifact(&tampered).is_err());
    }
}
