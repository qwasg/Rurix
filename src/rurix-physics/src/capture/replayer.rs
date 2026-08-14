//! CaptureReplayer:只读 artifact,从 t0 journal 确定性重建并逐 tick 比对 hash。

use std::collections::HashMap;
use std::path::Path;

use super::canonical::{CaptureError, event_digest, hash_canonical_state, state_from_world};
use super::divergence::{DivergenceLocate, FieldDiff, locate_divergence};
use super::header::{PhysicsCaptureHeader, RECOVERY_LAYER_V1, SCHEMA_ID};
use super::inject::{InjectRequest, inject_before_tick};
use super::journal::{JournalCommand, JournalTick};
use super::recorder::{CaptureArtifact, default_budget};
use crate::bridge::{PageKey, StreamingBridge};
use crate::budget::SyncBudget;
use crate::error::PhysicsError;
use crate::id::BodyId;
use crate::types::{ContactEvent, QueryRay, WorldDesc};
use crate::world::PhysicsWorld;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayVerdict {
    Pass,
    HashMismatch {
        tick: u64,
        field: String,
        expected: String,
        actual: String,
    },
    JournalLeftover {
        tick: u64,
        detail: String,
    },
    JournalMissing {
        tick: u64,
    },
    AssignedIdMismatch {
        expected: u64,
        actual: u64,
    },
    HeaderInvalid(String),
    InjectionDivergence {
        tick: u64,
        diffs: Vec<FieldDiff>,
    },
    Backend(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayReport {
    pub verdict: ReplayVerdict,
    pub ticks_ok: u64,
    pub tick_count: u64,
    pub journal_fully_consumed: bool,
    pub recovery_layer: String,
    pub scenario_id: String,
    pub first_divergence: Option<DivergenceLocate>,
}

/// 从 capture 目录独立重建并逐 tick 校验 hash;可选注入路径(注入 tick 后允许 hash 偏离)。
pub fn replay_capture_dir(
    dir: &Path,
    injection: Option<&InjectRequest>,
) -> Result<ReplayReport, CaptureError> {
    let artifact = CaptureArtifact::load(dir)?;
    replay_artifact(&artifact, injection)
}

pub fn replay_artifact(
    artifact: &CaptureArtifact,
    injection: Option<&InjectRequest>,
) -> Result<ReplayReport, CaptureError> {
    validate_header(&artifact.header)?;
    if artifact.ticks.len() as u64 != artifact.header.tick_count {
        let lines = artifact.ticks.len() as u64;
        let expected = artifact.header.tick_count;
        let verdict = if lines > expected {
            ReplayVerdict::JournalLeftover {
                tick: expected,
                detail: format!("journal lines {lines} != tick_count {expected}"),
            }
        } else {
            ReplayVerdict::JournalMissing { tick: lines }
        };
        return Ok(fail_report(artifact, verdict, 0, false, None));
    }
    let world_desc = artifact.header.world_desc.to_desc()?;
    let mut world =
        PhysicsWorld::new(world_desc.clone()).map_err(|e| CaptureError::Backend(e.to_string()))?;
    let budget_profile = artifact.header.budget_profile.clone();
    let mut streaming = StreamingBridge::new();
    let mut constraint_map: HashMap<u64, u64> = HashMap::new();
    let mut ticks_ok = 0u64;
    let mut first_divergence = None;

    for expected in &artifact.ticks {
        if expected.tick != ticks_ok {
            return Ok(fail_report(
                artifact,
                ReplayVerdict::JournalMissing { tick: ticks_ok },
                ticks_ok,
                false,
                first_divergence,
            ));
        }
        apply_journal_pre(
            &mut world,
            &mut streaming,
            &mut constraint_map,
            &budget_profile,
            &expected.pre,
        )?;
        if let Some(inj) = injection {
            inject_before_tick(&mut world, inj, expected.tick)?;
        }
        world
            .step(world_desc.dt_fixed)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        let mut budget = SyncBudget::new(
            budget_profile.max_body_writes,
            budget_profile.max_contact_events,
            budget_profile.max_query_casts,
        );
        let events: Vec<ContactEvent> = world.drain_contacts(&mut budget).collect();
        let state = state_from_world(&world, expected.tick)?;
        let sh = hash_canonical_state(&state)?;
        let ed = event_digest(&events)?;

        let injected_at = injection.map(|i| i.tick == expected.tick).unwrap_or(false);
        if !injected_at {
            if sh != expected.post.semantic_state_hash {
                return Ok(fail_report(
                    artifact,
                    ReplayVerdict::HashMismatch {
                        tick: expected.tick,
                        field: "semantic_state_hash".into(),
                        expected: expected.post.semantic_state_hash.clone(),
                        actual: sh,
                    },
                    ticks_ok,
                    false,
                    first_divergence,
                ));
            }
            if ed != expected.post.event_digest {
                return Ok(fail_report(
                    artifact,
                    ReplayVerdict::HashMismatch {
                        tick: expected.tick,
                        field: "event_digest".into(),
                        expected: expected.post.event_digest.clone(),
                        actual: ed,
                    },
                    ticks_ok,
                    false,
                    first_divergence,
                ));
            }
        } else if first_divergence.is_none() {
            // 注入 replay:clean 路径须先自证(此处仅 injected 单轨,由 inject 子命令双轨比对)。
            if sh != expected.post.semantic_state_hash {
                first_divergence = Some(DivergenceLocate {
                    first_divergence_tick: expected.tick,
                    diffs: vec![FieldDiff {
                        path: "semantic_state_hash".into(),
                        stable_id: String::new(),
                        expected_bits: 0,
                        actual_bits: 0,
                    }],
                });
            }
        }
        ticks_ok += 1;
    }

    if ticks_ok != artifact.header.tick_count {
        return Ok(fail_report(
            artifact,
            ReplayVerdict::JournalMissing { tick: ticks_ok },
            ticks_ok,
            false,
            first_divergence,
        ));
    }

    Ok(ReplayReport {
        verdict: ReplayVerdict::Pass,
        ticks_ok,
        tick_count: artifact.header.tick_count,
        journal_fully_consumed: true,
        recovery_layer: artifact.header.recovery_layer.clone(),
        scenario_id: artifact.header.scenario_id.clone(),
        first_divergence,
    })
}

/// clean vs injected 锁步;返回首个 canonical 不等 tick。
pub fn locate_injection_divergence(
    dir: &Path,
    injection: &InjectRequest,
) -> Result<DivergenceLocate, CaptureError> {
    let artifact = CaptureArtifact::load(dir)?;
    validate_header(&artifact.header)?;
    let world_desc = artifact.header.world_desc.to_desc()?;
    let budget_profile = artifact.header.budget_profile.clone();

    let mut clean =
        PhysicsWorld::new(world_desc.clone()).map_err(|e| CaptureError::Backend(e.to_string()))?;
    let mut dirty =
        PhysicsWorld::new(world_desc).map_err(|e| CaptureError::Backend(e.to_string()))?;
    let mut stream_c = StreamingBridge::new();
    let mut stream_d = StreamingBridge::new();
    let mut map_c = HashMap::new();
    let mut map_d = HashMap::new();

    for expected in &artifact.ticks {
        apply_journal_pre(
            &mut clean,
            &mut stream_c,
            &mut map_c,
            &budget_profile,
            &expected.pre,
        )?;
        apply_journal_pre(
            &mut dirty,
            &mut stream_d,
            &mut map_d,
            &budget_profile,
            &expected.pre,
        )?;
        inject_before_tick(&mut dirty, injection, expected.tick)?;
        clean
            .step(clean.desc().dt_fixed)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        dirty
            .step(dirty.desc().dt_fixed)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        let mut b1 = SyncBudget::new(
            budget_profile.max_body_writes,
            budget_profile.max_contact_events,
            budget_profile.max_query_casts,
        );
        let mut b2 = SyncBudget::new(
            budget_profile.max_body_writes,
            budget_profile.max_contact_events,
            budget_profile.max_query_casts,
        );
        let _: Vec<_> = clean.drain_contacts(&mut b1).collect();
        let _: Vec<_> = dirty.drain_contacts(&mut b2).collect();

        let s_clean = state_from_world(&clean, expected.tick)?;
        let s_dirty = state_from_world(&dirty, expected.tick)?;
        let sh = hash_canonical_state(&s_clean)?;
        if sh != expected.post.semantic_state_hash {
            return Err(CaptureError::Mismatch(format!(
                "clean replay hash mismatch tick {}",
                expected.tick
            )));
        }
        if let Some(div) = locate_divergence(&s_clean, &s_dirty)? {
            return Ok(div);
        }
    }
    Err(CaptureError::Mismatch(
        "injection produced no divergence".into(),
    ))
}

/// journal 行数超出 tick_count → fail-closed 探测(不改 header)。
pub fn replay_with_extra_journal_line(dir: &Path) -> Result<ReplayVerdict, CaptureError> {
    let mut artifact = CaptureArtifact::load(dir)?;
    if artifact.ticks.is_empty() {
        return Err(CaptureError::Parse("empty journal".into()));
    }
    let last = artifact.ticks.last().cloned().unwrap();
    artifact.ticks.push(JournalTick {
        tick: last.tick + 1,
        pre: Vec::new(),
        post: last.post,
    });
    let report = replay_artifact(&artifact, None)?;
    Ok(report.verdict)
}

/// 删除末行 journal 但 header tick_count 不变 → fail-closed 探测。
pub fn replay_with_missing_journal_line(dir: &Path) -> Result<ReplayVerdict, CaptureError> {
    let mut artifact = CaptureArtifact::load(dir)?;
    if artifact.ticks.is_empty() {
        return Err(CaptureError::Parse("empty journal".into()));
    }
    artifact.ticks.pop();
    let report = replay_artifact(&artifact, None)?;
    Ok(report.verdict)
}

fn fail_report(
    artifact: &CaptureArtifact,
    verdict: ReplayVerdict,
    ticks_ok: u64,
    consumed: bool,
    first_divergence: Option<DivergenceLocate>,
) -> ReplayReport {
    ReplayReport {
        verdict,
        ticks_ok,
        tick_count: artifact.header.tick_count,
        journal_fully_consumed: consumed,
        recovery_layer: artifact.header.recovery_layer.clone(),
        scenario_id: artifact.header.scenario_id.clone(),
        first_divergence,
    }
}

fn validate_header(h: &PhysicsCaptureHeader) -> Result<(), CaptureError> {
    h.validate_complete()?;
    if h.schema_id != SCHEMA_ID {
        return Err(CaptureError::Parse(format!("schema_id {}", h.schema_id)));
    }
    if h.recovery_layer != RECOVERY_LAYER_V1 {
        return Err(CaptureError::Parse(format!(
            "recovery_layer {}",
            h.recovery_layer
        )));
    }
    if h.jolt_version != "5.3.0" {
        return Err(CaptureError::Parse(format!(
            "jolt_version {}",
            h.jolt_version
        )));
    }
    Ok(())
}

/// 回放/录制共用:执行 tick 前 journal 命令。
pub fn apply_journal_pre(
    world: &mut PhysicsWorld,
    streaming: &mut StreamingBridge,
    constraint_map: &mut HashMap<u64, u64>,
    budget_profile: &super::header::BudgetProfile,
    cmds: &[JournalCommand],
) -> Result<(), CaptureError> {
    for cmd in cmds {
        match cmd {
            JournalCommand::CreateBodies {
                descs,
                assigned_ids,
            } => {
                let ids = world
                    .add_bodies_batch(descs)
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
                for (want, got) in assigned_ids.iter().zip(ids.iter()) {
                    if *want != got.to_bits() {
                        return Err(CaptureError::Mismatch(format!(
                            "assigned id {want:016x} != {:#016x}",
                            got.to_bits()
                        )));
                    }
                }
            }
            JournalCommand::RemoveBodies { ids } => {
                let bodies: Vec<BodyId> = ids.iter().copied().map(BodyId::from_bits).collect();
                world
                    .remove_bodies_batch(&bodies)
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
            }
            JournalCommand::ApplyImpulse { body, impulse } => {
                world
                    .apply_impulse(BodyId::from_bits(*body), *impulse)
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
            }
            JournalCommand::SetVelocity {
                body,
                linear,
                angular,
            } => {
                let id = BodyId::from_bits(*body);
                world
                    .set_linear_velocity(id, *linear)
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
                world
                    .set_angular_velocity(id, *angular)
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
            }
            JournalCommand::MoveKinematic { body, transform } => {
                world
                    .set_kinematic_target(BodyId::from_bits(*body), *transform)
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
            }
            JournalCommand::PageResident {
                page_resource,
                page,
                descs,
                assigned_ids,
            } => {
                let page_key = PageKey {
                    resource: *page_resource,
                    page: *page,
                };
                let ids = streaming
                    .insert_page(world, page_key, descs)
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
                for (want, got) in assigned_ids.iter().zip(ids.iter()) {
                    if *want != got.to_bits() {
                        return Err(CaptureError::Mismatch(format!(
                            "page assigned id {want:016x} != {:#016x}",
                            got.to_bits()
                        )));
                    }
                }
            }
            JournalCommand::PageUnload {
                page_resource,
                page,
                receipt_bodies,
            } => {
                let page_key = PageKey {
                    resource: *page_resource,
                    page: *page,
                };
                let receipt = streaming
                    .remove_page(world, page_key)
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
                let got: Vec<u64> = receipt
                    .removed_bodies()
                    .iter()
                    .map(|b| b.to_bits())
                    .collect();
                if &got != receipt_bodies {
                    return Err(CaptureError::Mismatch(format!(
                        "receipt order mismatch: {got:?} vs {receipt_bodies:?}"
                    )));
                }
            }
            JournalCommand::AddConstraint {
                ctype,
                body_a,
                body_b,
                point,
                hinge_axis,
                normal_axis,
                assigned_id,
            } => {
                let token = world
                    .add_hinge_constraint(
                        BodyId::from_bits(*body_a),
                        BodyId::from_bits(*body_b),
                        *point,
                        *hinge_axis,
                        *normal_axis,
                    )
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
                constraint_map.insert(*assigned_id, token);
                let _ = ctype;
            }
            JournalCommand::RemoveConstraint { id } => {
                if let Some(token) = constraint_map.remove(id) {
                    world
                        .remove_constraint(token)
                        .map_err(|e| CaptureError::Backend(e.to_string()))?;
                }
            }
            JournalCommand::SetMotor { id, state, target } => {
                if let Some(token) = constraint_map.get(id) {
                    world
                        .set_hinge_motor(*token, *state, *target)
                        .map_err(|e| CaptureError::Backend(e.to_string()))?;
                }
            }
            JournalCommand::QueryRay {
                origin,
                dir,
                t_min,
                t_max,
                layer_mask,
                expected_hits,
            } => {
                let mut budget = SyncBudget::new(
                    budget_profile.max_body_writes,
                    budget_profile.max_contact_events,
                    budget_profile.max_query_casts,
                );
                let hits = world
                    .cast_ray(
                        &QueryRay {
                            origin: *origin,
                            dir: *dir,
                            t_min: *t_min,
                            t_max: *t_max,
                            layer_mask: *layer_mask,
                        },
                        &mut budget,
                    )
                    .map_err(|e| CaptureError::Backend(e.to_string()))?;
                if hits.len() != expected_hits.len() {
                    return Err(CaptureError::Mismatch(format!(
                        "query hit_count {} != {}",
                        hits.len(),
                        expected_hits.len()
                    )));
                }
                for (hit, (exp_body, exp_t_bits)) in hits.iter().zip(expected_hits.iter()) {
                    if hit.body.to_bits() != *exp_body {
                        return Err(CaptureError::Mismatch(format!(
                            "query body {:016x} != {exp_body:016x}",
                            hit.body.to_bits()
                        )));
                    }
                    if hit.t.to_bits() != *exp_t_bits {
                        return Err(CaptureError::Mismatch(format!(
                            "query t {:08x} != {exp_t_bits:08x}",
                            hit.t.to_bits()
                        )));
                    }
                }
            }
            // RXS-0374 L3:场命令需场感知 replay(`field::capture_merge::
            // replay_field_capture` 重建 FieldRegistry 并逐 tick 核验场
            // hash);legacy 纯世界 replay 不静默吞掉场命令——fail-closed
            // 拒绝(防止场参与 capture 被半校验充绿)。
            JournalCommand::FieldRegister { .. }
            | JournalCommand::FieldUnregister { .. }
            | JournalCommand::FieldUpdate { .. } => {
                return Err(CaptureError::Rejected(
                    "field journal command requires field-aware replay \
                     (rurix_physics::field::capture_merge::replay_field_capture)"
                        .into(),
                ));
            }
        }
    }
    Ok(())
}

pub fn jolt_world_desc(contact_capacity: u32) -> WorldDesc {
    WorldDesc {
        backend: crate::types::BackendKind::Jolt,
        gravity: [0.0, -9.81, 0.0],
        layer_count: 8,
        max_bodies: 4096,
        job_threads: Some(1),
        dt_fixed: 1.0 / 60.0,
        contact_capacity,
    }
}

pub fn budget_for(desc: &WorldDesc) -> super::header::BudgetProfile {
    default_budget(desc)
}

/// 映射 PhysicsError 供 CLI。
pub fn capture_err_from_physics(e: PhysicsError) -> CaptureError {
    CaptureError::Backend(e.to_string())
}
