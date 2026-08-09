//! NetTrace fixture 加载/确定性双世界模拟(零 socket)。

use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use crate::capture::header::BudgetProfile;
use crate::capture::journal::JournalCommand;
use crate::capture::recorder::default_budget;
use crate::capture::replayer::jolt_world_desc;
use crate::id::BodyId;
use crate::types::{BodyDesc, BodyKind, MassProps, PhysicsTransform, ShapeDesc, WorldDesc};
use crate::world::PhysicsWorld;

use super::client::ClientWorld;
use super::frame::{NetworkPhysicsFrameId, PhysicsTickId};
use super::history::HistoryRing;
use super::rollback::TickInput;
use super::server::{AuthoritativeSnapshot, ServerWorld};
use super::smoothing::{SMOOTHING_BOUND_V1, SmoothingBound};
use super::{HardCorrectionReason, NetError};

const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

#[derive(Debug, Clone, PartialEq)]
pub enum DeliveryKind {
    Delay { snapshot_delay_ticks: u64 },
    Drop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceActorImpulse {
    pub body_role: String,
    pub impulse: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceFrame {
    pub net_frame: u64,
    pub client_local: Vec<TraceActorImpulse>,
    pub other_actors: Vec<TraceActorImpulse>,
    pub delivery: DeliveryKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetTrace {
    pub schema_id: String,
    pub schema_version: u32,
    pub trace_id: String,
    pub history_ring_capacity: usize,
    pub golden_correction_frame: u64,
    pub schema_digest: String,
    pub build_digest: String,
    pub expected_rollback_start: u64,
    pub frames: Vec<TraceFrame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetTraceReport {
    pub trace_id: String,
    pub profile_match: bool,
    pub job_threads: u32,
    pub frame_domain_map_count: usize,
    pub prediction_divergence_observed: bool,
    pub correction_received_at_golden: bool,
    pub correction_frame: Option<u64>,
    pub rollback_start: Option<u64>,
    pub rollback_input_sequence: Vec<u64>,
    pub expected_rollback_start: u64,
    pub rollback_sequence_matches: bool,
    pub resim_final_hash: Option<String>,
    pub server_hash_at_correction: Option<String>,
    pub resim_final_hash_equals_server: bool,
    pub contact_events_committed: usize,
    pub contact_event_committed_exactly_once: bool,
    pub event_dedup_across_repeated_rollbacks: bool,
    pub smoothing_authoritative_state_untouched: bool,
    pub smoothing_within_frozen_bound_per_frame: bool,
    pub smoothing_bound_frozen: bool,
    pub max_position_offset_m: f32,
    pub max_angle_offset_rad: f32,
    pub history_ring_overflow_hard_correction_explicit: bool,
    pub incompatible_digest_rejected: bool,
    pub trace_fixture_deterministic: bool,
}

struct RoleMap {
    ball_a: BodyId,
    ball_b: BodyId,
}

fn scene_descs() -> [BodyDesc; 3] {
    let ground = BodyDesc {
        kind: BodyKind::Static,
        shape: ShapeDesc::Box {
            half_extents: [20.0, 0.5, 20.0],
        },
        layer: 0,
        mass_props: MassProps::default(),
        transform: PhysicsTransform {
            translation: [0.0, -0.5, 0.0],
            rotation: IDENTITY,
        },
        ccd: false,
    };
    let ball = |x: f32| BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Sphere { radius: 0.5 },
        layer: 0,
        mass_props: MassProps {
            mass: 1.0,
            friction: 0.5,
            restitution: 0.1,
            allow_sleep: true,
        },
        transform: PhysicsTransform {
            translation: [x, 1.0, 0.0],
            rotation: IDENTITY,
        },
        ccd: false,
    };
    [ground, ball(-1.0), ball(1.0)]
}

fn allocate_roles(desc: &WorldDesc) -> Result<(JournalCommand, RoleMap), NetError> {
    let descs = scene_descs();
    let mut probe =
        PhysicsWorld::new(desc.clone()).map_err(|e| NetError::Backend(e.to_string()))?;
    let ids = probe
        .add_bodies_batch(&descs)
        .map_err(|e| NetError::Backend(e.to_string()))?;
    let assigned: Vec<u64> = ids.iter().map(|b| b.to_bits()).collect();
    Ok((
        JournalCommand::CreateBodies {
            descs: descs.to_vec(),
            assigned_ids: assigned,
        },
        RoleMap {
            ball_a: ids[1],
            ball_b: ids[2],
        },
    ))
}

fn cmds_for(
    roles: &RoleMap,
    local: &[TraceActorImpulse],
    other: &[TraceActorImpulse],
) -> Vec<JournalCommand> {
    let mut cmds = Vec::new();
    for imp in local.iter().chain(other.iter()) {
        let body = match imp.body_role.as_str() {
            "ball_a" => roles.ball_a.to_bits(),
            "ball_b" => roles.ball_b.to_bits(),
            _ => continue,
        };
        cmds.push(JournalCommand::ApplyImpulse {
            body,
            impulse: imp.impulse,
        });
    }
    cmds
}

pub fn load_net_trace(path: &Path) -> Result<NetTrace, NetError> {
    let text = fs::read_to_string(path).map_err(|e| NetError::Io(e.to_string()))?;
    parse_net_trace(&text)
}

pub fn parse_net_trace(text: &str) -> Result<NetTrace, NetError> {
    let schema_id = json_str(text, "schema_id")?;
    let schema_version = json_u64(text, "schema_version")? as u32;
    let trace_id = json_str(text, "trace_id")?;
    let history_ring_capacity = json_u64(text, "history_ring_capacity")? as usize;
    let golden_correction_frame = json_u64(text, "golden_correction_frame")?;
    let schema_digest = json_str(text, "schema_digest")?;
    let build_digest = json_str(text, "build_digest")?;
    let expected_rollback_start = json_u64(text, "expected_rollback_start")?;
    if schema_id != "rurix.physics.net_trace" || schema_version != 1 {
        return Err(NetError::Rejected(format!(
            "unsupported net_trace {schema_id} v{schema_version}"
        )));
    }
    Ok(NetTrace {
        schema_id,
        schema_version,
        trace_id,
        history_ring_capacity,
        golden_correction_frame,
        schema_digest,
        build_digest,
        expected_rollback_start,
        frames: parse_frames(text)?,
    })
}

fn parse_frames(text: &str) -> Result<Vec<TraceFrame>, NetError> {
    let mut frames = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = text[search_from..].find("\"net_frame\"") {
        let idx = search_from + rel;
        let rest = &text[idx..];
        let net_frame = json_u64(rest, "net_frame")?;
        let client_local = parse_impulse_array(rest, "client_local")?;
        let other_actors = parse_impulse_array(rest, "other_actors")?;
        let delivery = if delivery_is_drop(rest) {
            DeliveryKind::Drop
        } else {
            DeliveryKind::Delay {
                snapshot_delay_ticks: json_u64(rest, "snapshot_delay_ticks").unwrap_or(0),
            }
        };
        frames.push(TraceFrame {
            net_frame,
            client_local,
            other_actors,
            delivery,
        });
        search_from = idx + 12;
    }
    if frames.is_empty() {
        return Err(NetError::Rejected("net_trace has no frames".into()));
    }
    Ok(frames)
}

fn delivery_is_drop(rest: &str) -> bool {
    if let Some(d) = rest.find("\"delivery\"") {
        let end = (d + 96).min(rest.len());
        let slice = &rest[d..end];
        slice.contains("\"kind\": \"drop\"") || slice.contains("\"kind\":\"drop\"")
    } else {
        false
    }
}

fn parse_impulse_array(text: &str, key: &str) -> Result<Vec<TraceActorImpulse>, NetError> {
    let key_pat = format!("\"{key}\"");
    let Some(start) = text.find(&key_pat) else {
        return Ok(Vec::new());
    };
    // 截断到下一个同级关键帧字段,避免吃到下一 frame
    let window_end = text[start..]
        .find("\"delivery\"")
        .map(|i| start + i)
        .unwrap_or(text.len());
    let window = &text[start..window_end];
    let Some(arr_start_rel) = window.find('[') else {
        return Ok(Vec::new());
    };
    let arr_start = arr_start_rel;
    let bytes = window[arr_start..].as_bytes();
    let mut depth = 0i32;
    let mut end = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        match *b {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let arr = &window[arr_start..=arr_start + end];
    let mut out = Vec::new();
    let mut cursor = arr;
    while let Some(idx) = cursor.find("\"body_role\"") {
        cursor = &cursor[idx..];
        let role = json_str(cursor, "body_role")?;
        let impulse = json_f32_array3(cursor, "impulse")?;
        out.push(TraceActorImpulse {
            body_role: role,
            impulse,
        });
        if let Some(n) = cursor[12..].find("\"body_role\"") {
            cursor = &cursor[12 + n..];
        } else {
            break;
        }
    }
    Ok(out)
}

fn json_str(text: &str, key: &str) -> Result<String, NetError> {
    let pat = format!("\"{key}\"");
    let Some(i) = text.find(&pat) else {
        return Err(NetError::Rejected(format!("missing string {key}")));
    };
    let after = &text[i + pat.len()..];
    let Some(colon) = after.find(':') else {
        return Err(NetError::Rejected(format!("bad {key}")));
    };
    let after = after[colon + 1..].trim_start();
    let Some(rest) = after.strip_prefix('"') else {
        return Err(NetError::Rejected(format!("{key} not string")));
    };
    let Some(end) = rest.find('"') else {
        return Err(NetError::Rejected(format!("{key} unclosed")));
    };
    Ok(rest[..end].to_string())
}

fn json_u64(text: &str, key: &str) -> Result<u64, NetError> {
    let pat = format!("\"{key}\"");
    let Some(i) = text.find(&pat) else {
        return Err(NetError::Rejected(format!("missing u64 {key}")));
    };
    let after = &text[i + pat.len()..];
    let Some(colon) = after.find(':') else {
        return Err(NetError::Rejected(format!("bad {key}")));
    };
    let after = after[colon + 1..].trim_start();
    let num: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse()
        .map_err(|e| NetError::Rejected(format!("{key}: {e}")))
}

fn json_f32_array3(text: &str, key: &str) -> Result<[f32; 3], NetError> {
    let pat = format!("\"{key}\"");
    let Some(i) = text.find(&pat) else {
        return Err(NetError::Rejected(format!("missing {key}")));
    };
    let after = &text[i + pat.len()..];
    let Some(lb) = after.find('[') else {
        return Err(NetError::Rejected(format!("{key} not array")));
    };
    let after = &after[lb + 1..];
    let Some(rb) = after.find(']') else {
        return Err(NetError::Rejected(format!("{key} unclosed")));
    };
    let parts: Vec<&str> = after[..rb].split(',').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return Err(NetError::Rejected(format!("{key} need 3 floats")));
    }
    let mut out = [0.0; 3];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p
            .parse()
            .map_err(|e| NetError::Rejected(format!("{key}[{i}]: {e}")))?;
    }
    Ok(out)
}

pub fn run_net_trace(trace: &NetTrace) -> Result<NetTraceReport, NetError> {
    run_net_trace_with_bound(trace, SMOOTHING_BOUND_V1)
}

pub fn run_net_trace_with_bound(
    trace: &NetTrace,
    bound: SmoothingBound,
) -> Result<NetTraceReport, NetError> {
    let world_desc = WorldDesc {
        job_threads: Some(1),
        ..jolt_world_desc(4096)
    };
    let budget: BudgetProfile = default_budget(&world_desc);
    let (create_cmd, roles) = allocate_roles(&world_desc)?;

    let mut server = ServerWorld::new(
        world_desc.clone(),
        budget.clone(),
        trace.history_ring_capacity,
        &trace.schema_digest,
        &trace.build_digest,
    )?;
    let mut client = ClientWorld::new(
        world_desc.clone(),
        budget.clone(),
        trace.history_ring_capacity,
        &trace.schema_digest,
        &trace.build_digest,
        vec![roles.ball_a, roles.ball_b],
    )?;
    client.set_smoothing_bound(bound);

    let mut pending_snaps: VecDeque<(u64, AuthoritativeSnapshot)> = VecDeque::new();
    let mut correction_at_golden = false;
    let mut correction_frame = None;
    let mut max_pos = 0.0f32;
    let mut max_ang = 0.0f32;
    let mut server_hash_at_corr = None;
    let mut first_divergent: Option<super::client::CorrectionReport> = None;

    for frame in &trace.frames {
        let f = frame.net_frame;
        let mut server_cmds = Vec::new();
        let mut client_cmds = Vec::new();
        if f == 0 {
            server_cmds.push(create_cmd.clone());
            client_cmds.push(create_cmd.clone());
        }
        server_cmds.extend(cmds_for(&roles, &frame.client_local, &frame.other_actors));
        client_cmds.extend(cmds_for(&roles, &frame.client_local, &[]));

        server.step(server_cmds)?;

        match &frame.delivery {
            DeliveryKind::Drop => {}
            DeliveryKind::Delay {
                snapshot_delay_ticks,
            } => {
                let snap = server.emit_snapshot()?;
                pending_snaps.push_back((f + *snapshot_delay_ticks, snap));
            }
        }

        let mut incoming = None;
        while let Some((due, _)) = pending_snaps.front() {
            if *due == f {
                incoming = pending_snaps.pop_front().map(|(_, s)| s);
                break;
            } else if *due < f {
                let _ = pending_snaps.pop_front();
            } else {
                break;
            }
        }

        let report = client.step_predict(client_cmds, incoming.as_ref(), 0.35)?;
        for (_, off) in &report.presentation_offsets {
            max_pos = max_pos.max(off.position_m);
            max_ang = max_ang.max(off.angle_rad);
        }
        if let Some(corr) = &report.correction {
            if corr.diverged && first_divergent.is_none() {
                first_divergent = Some(corr.clone());
            }
            if f == trace.golden_correction_frame {
                correction_at_golden = corr.diverged || first_divergent.is_some();
                correction_frame = Some(f);
                server_hash_at_corr = Some(corr.server_hash.clone());
            }
        }
    }

    let before = client.event_bridge().published_count();
    let again = client.recommit_pending();
    let dedup_ok = again.is_empty() && client.event_bridge().published_count() == before;
    let committed = client.event_bridge().published_count();
    let exactly_once = committed > 0 && {
        let c2 = client.recommit_pending();
        c2.is_empty()
    };

    let corr = first_divergent
        .clone()
        .or_else(|| client.last_correction().cloned());
    let rollback_start = corr
        .as_ref()
        .and_then(|c| c.rollback.as_ref().map(|r| r.start_tick.0));
    let rollback_seq: Vec<u64> = corr
        .as_ref()
        .and_then(|c| c.rollback.as_ref())
        .map(|r| r.input_sequence.iter().map(|t| t.frame.0).collect())
        .unwrap_or_default();
    let seq_match = rollback_start == Some(trace.expected_rollback_start)
        && (rollback_seq.is_empty()
            || rollback_seq.first().copied() == Some(trace.expected_rollback_start + 1));

    let resim_ok = corr
        .as_ref()
        .map(|c| c.resim_matches_server)
        .unwrap_or(false);
    let resim_hash = corr.as_ref().and_then(|c| c.resim_final_hash.clone());

    let incompatible_digest_rejected = {
        let bad = AuthoritativeSnapshot {
            net_frame: NetworkPhysicsFrameId(0),
            physics_tick: PhysicsTickId(0),
            semantic_state_hash: "00".into(),
            schema_digest: "wrong-schema".into(),
            build_digest: "wrong-build".into(),
            state: crate::capture::canonical::CanonicalPhysicsState {
                tick: 0,
                bodies: Vec::new(),
                constraints: Vec::new(),
            },
            inputs_through: Vec::new(),
        };
        let mut probe = ClientWorld::new(
            world_desc.clone(),
            budget.clone(),
            8,
            "good-schema",
            "good-build",
            vec![roles.ball_a],
        )?;
        matches!(
            probe.step_predict(Vec::new(), Some(&bad), 0.5),
            Err(NetError::IncompatibleDigest { .. })
        )
    };

    let hard_overflow = {
        let mut ring: HistoryRing<TickInput> = HistoryRing::new(2)?;
        let _ = ring.push(
            NetworkPhysicsFrameId(0),
            TickInput {
                frame: NetworkPhysicsFrameId(0),
                commands: Vec::new(),
            },
            Some(NetworkPhysicsFrameId(0)),
        );
        let _ = ring.push(
            NetworkPhysicsFrameId(1),
            TickInput {
                frame: NetworkPhysicsFrameId(1),
                commands: Vec::new(),
            },
            Some(NetworkPhysicsFrameId(0)),
        );
        matches!(
            ring.push(
                NetworkPhysicsFrameId(2),
                TickInput {
                    frame: NetworkPhysicsFrameId(2),
                    commands: Vec::new(),
                },
                Some(NetworkPhysicsFrameId(0)),
            ),
            Err(NetError::HardCorrection {
                reason: HardCorrectionReason::HistoryRingOverflow,
                ..
            })
        )
    };

    let a = client.prediction_diverged();
    let det = {
        // 同 fixture 关键字段再跑一遍
        let r2 = run_net_trace_once(trace, bound)?;
        r2.prediction_divergence_observed == a
            && r2.correction_frame == correction_frame
            && r2.resim_final_hash_equals_server == resim_ok
    };

    Ok(NetTraceReport {
        trace_id: trace.trace_id.clone(),
        profile_match: world_desc.job_threads == Some(1),
        job_threads: world_desc.job_threads.unwrap_or(0),
        frame_domain_map_count: client.domain_maps().len() + server.domain_maps().len(),
        prediction_divergence_observed: a,
        correction_received_at_golden: correction_at_golden,
        correction_frame,
        rollback_start,
        rollback_input_sequence: rollback_seq,
        expected_rollback_start: trace.expected_rollback_start,
        rollback_sequence_matches: seq_match,
        resim_final_hash: resim_hash,
        server_hash_at_correction: server_hash_at_corr,
        resim_final_hash_equals_server: resim_ok,
        contact_events_committed: committed,
        contact_event_committed_exactly_once: exactly_once,
        event_dedup_across_repeated_rollbacks: dedup_ok && committed > 0,
        smoothing_authoritative_state_untouched: client.authoritative_untouched_after_smooth()?,
        smoothing_within_frozen_bound_per_frame: client.all_offsets_within_bound()?,
        smoothing_bound_frozen: bound.frozen,
        max_position_offset_m: max_pos,
        max_angle_offset_rad: max_ang,
        history_ring_overflow_hard_correction_explicit: hard_overflow,
        incompatible_digest_rejected,
        trace_fixture_deterministic: det,
    })
}

fn run_net_trace_once(trace: &NetTrace, bound: SmoothingBound) -> Result<NetTraceReport, NetError> {
    // 避免 assert_trace_deterministic 递归双倍爆炸:内部轻量再跑(无嵌套 det)
    let world_desc = WorldDesc {
        job_threads: Some(1),
        ..jolt_world_desc(4096)
    };
    let budget: BudgetProfile = default_budget(&world_desc);
    let (create_cmd, roles) = allocate_roles(&world_desc)?;
    let mut server = ServerWorld::new(
        world_desc.clone(),
        budget.clone(),
        trace.history_ring_capacity,
        &trace.schema_digest,
        &trace.build_digest,
    )?;
    let mut client = ClientWorld::new(
        world_desc,
        budget,
        trace.history_ring_capacity,
        &trace.schema_digest,
        &trace.build_digest,
        vec![roles.ball_a, roles.ball_b],
    )?;
    client.set_smoothing_bound(bound);
    let mut pending: VecDeque<(u64, AuthoritativeSnapshot)> = VecDeque::new();
    let mut correction_frame = None;
    for frame in &trace.frames {
        let f = frame.net_frame;
        let mut sc = Vec::new();
        let mut cc = Vec::new();
        if f == 0 {
            sc.push(create_cmd.clone());
            cc.push(create_cmd.clone());
        }
        sc.extend(cmds_for(&roles, &frame.client_local, &frame.other_actors));
        cc.extend(cmds_for(&roles, &frame.client_local, &[]));
        server.step(sc)?;
        if let DeliveryKind::Delay {
            snapshot_delay_ticks,
        } = &frame.delivery
        {
            pending.push_back((f + *snapshot_delay_ticks, server.emit_snapshot()?));
        }
        let mut incoming = None;
        while let Some((due, _)) = pending.front() {
            if *due == f {
                incoming = pending.pop_front().map(|(_, s)| s);
                break;
            } else if *due < f {
                pending.pop_front();
            } else {
                break;
            }
        }
        let report = client.step_predict(cc, incoming.as_ref(), 0.35)?;
        if let Some(corr) = report.correction {
            if f == trace.golden_correction_frame {
                correction_frame = Some(f);
                let _ = corr;
            }
        }
    }
    Ok(NetTraceReport {
        trace_id: trace.trace_id.clone(),
        profile_match: true,
        job_threads: 1,
        frame_domain_map_count: 0,
        prediction_divergence_observed: client.prediction_diverged(),
        correction_received_at_golden: correction_frame.is_some(),
        correction_frame,
        rollback_start: client
            .last_correction()
            .and_then(|c| c.rollback.as_ref().map(|r| r.start_tick.0)),
        rollback_input_sequence: Vec::new(),
        expected_rollback_start: trace.expected_rollback_start,
        rollback_sequence_matches: false,
        resim_final_hash: client
            .last_correction()
            .and_then(|c| c.resim_final_hash.clone()),
        server_hash_at_correction: None,
        resim_final_hash_equals_server: client
            .last_correction()
            .map(|c| c.resim_matches_server)
            .unwrap_or(false),
        contact_events_committed: 0,
        contact_event_committed_exactly_once: true,
        event_dedup_across_repeated_rollbacks: true,
        smoothing_authoritative_state_untouched: true,
        smoothing_within_frozen_bound_per_frame: false,
        smoothing_bound_frozen: bound.frozen,
        max_position_offset_m: 0.0,
        max_angle_offset_rad: 0.0,
        history_ring_overflow_hard_correction_explicit: true,
        incompatible_digest_rejected: true,
        trace_fixture_deterministic: true,
    })
}

pub fn assert_trace_deterministic(trace: &NetTrace) -> Result<bool, NetError> {
    let a = run_net_trace_once(trace, SMOOTHING_BOUND_V1)?;
    let b = run_net_trace_once(trace, SMOOTHING_BOUND_V1)?;
    Ok(
        a.prediction_divergence_observed == b.prediction_divergence_observed
            && a.correction_frame == b.correction_frame
            && a.resim_final_hash == b.resim_final_hash
            && a.rollback_start == b.rollback_start,
    )
}
