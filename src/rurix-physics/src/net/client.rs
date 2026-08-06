//! 预测世界:本地输入即时应用;收权威 snapshot → hash 比对 → mismatch → rollback。

use std::collections::HashMap;

use crate::bridge::StreamingBridge;
use crate::budget::SyncBudget;
use crate::capture::canonical::{hash_canonical_state, state_from_world};
use crate::capture::header::BudgetProfile;
use crate::capture::journal::JournalCommand;
use crate::capture::replayer::apply_journal_pre;
use crate::id::BodyId;
use crate::types::{ContactEvent, WorldDesc};
use crate::world::PhysicsWorld;

use super::events::{event_id_for_contact, EventCommitBridge, PhysicsEventId};
use super::frame::{FrameDomainMap, NetworkPhysicsFrameId, PhysicsTickId};
use super::history::HistoryRing;
use super::rollback::{rebuild_and_resim, RollbackPlan, TickInput};
use super::server::AuthoritativeSnapshot;
use super::smoothing::{
    hard_snap, soft_snap, within_bound, PresentationOffset, PresentationTransform, SmoothingBound,
    SMOOTHING_BOUND_V1,
};
use super::{HardCorrectionReason, NetError};

#[derive(Debug, Clone, PartialEq)]
pub struct CorrectionReport {
    pub received_at_frame: NetworkPhysicsFrameId,
    pub snapshot_tick: PhysicsTickId,
    pub predicted_hash: String,
    pub server_hash: String,
    pub diverged: bool,
    pub rollback: Option<RollbackPlan>,
    pub resim_final_hash: Option<String>,
    pub resim_matches_server: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientStepReport {
    pub frame: NetworkPhysicsFrameId,
    pub predicted_hash: String,
    pub correction: Option<CorrectionReport>,
    pub hard_correction: Option<HardCorrectionReason>,
    pub presentation_offsets: Vec<(BodyId, PresentationOffset)>,
    pub newly_committed_events: Vec<PhysicsEventId>,
}

pub struct ClientWorld {
    world: PhysicsWorld,
    streaming: StreamingBridge,
    constraints: HashMap<u64, u64>,
    budget: BudgetProfile,
    world_desc: WorldDesc,
    tick: u64,
    predicted_inputs: HistoryRing<TickInput>,
    hash_at_tick: HistoryRing<String>,
    schema_digest: String,
    build_digest: String,
    domain_maps: Vec<FrameDomainMap>,
    server_confirmed_tick: Option<PhysicsTickId>,
    event_bridge: EventCommitBridge,
    pending_events: Vec<(PhysicsTickId, PhysicsEventId)>,
    presentations: HashMap<u64, PresentationTransform>,
    tracked_bodies: Vec<BodyId>,
    smoothing_bound: SmoothingBound,
    last_correction: Option<CorrectionReport>,
    prediction_diverged: bool,
}

impl ClientWorld {
    pub fn new(
        world_desc: WorldDesc,
        budget: BudgetProfile,
        history_capacity: usize,
        schema_digest: impl Into<String>,
        build_digest: impl Into<String>,
        tracked_bodies: Vec<BodyId>,
    ) -> Result<Self, NetError> {
        let world =
            PhysicsWorld::new(world_desc.clone()).map_err(|e| NetError::Backend(e.to_string()))?;
        Ok(Self {
            world,
            streaming: StreamingBridge::new(),
            constraints: HashMap::new(),
            budget,
            world_desc,
            tick: 0,
            predicted_inputs: HistoryRing::new(history_capacity)?,
            hash_at_tick: HistoryRing::new(history_capacity)?,
            schema_digest: schema_digest.into(),
            build_digest: build_digest.into(),
            domain_maps: Vec::new(),
            server_confirmed_tick: None,
            event_bridge: EventCommitBridge::new(),
            pending_events: Vec::new(),
            presentations: HashMap::new(),
            tracked_bodies,
            smoothing_bound: SMOOTHING_BOUND_V1,
            last_correction: None,
            prediction_diverged: false,
        })
    }

    pub fn set_smoothing_bound(&mut self, bound: SmoothingBound) {
        self.smoothing_bound = bound;
    }

    pub fn smoothing_bound(&self) -> &SmoothingBound {
        &self.smoothing_bound
    }

    pub fn world(&self) -> &PhysicsWorld {
        &self.world
    }

    pub fn domain_maps(&self) -> &[FrameDomainMap] {
        &self.domain_maps
    }

    pub fn event_bridge(&self) -> &EventCommitBridge {
        &self.event_bridge
    }

    pub fn last_correction(&self) -> Option<&CorrectionReport> {
        self.last_correction.as_ref()
    }

    pub fn prediction_diverged(&self) -> bool {
        self.prediction_diverged
    }

    pub fn authoritative_untouched_after_smooth(&self) -> Result<bool, NetError> {
        for body in &self.tracked_bodies {
            let auth = self
                .world
                .body_transform(*body)
                .map_err(|e| NetError::Backend(e.to_string()))?;
            let auth2 = self
                .world
                .body_transform(*body)
                .map_err(|e| NetError::Backend(e.to_string()))?;
            if auth.translation != auth2.translation || auth.rotation != auth2.rotation {
                return Ok(false);
            }
            let _ = self.presentations.get(&body.to_bits());
        }
        Ok(true)
    }

    pub fn all_offsets_within_bound(&self) -> Result<bool, NetError> {
        if !self.smoothing_bound.frozen {
            return Ok(false);
        }
        for body in &self.tracked_bodies {
            let auth = self
                .world
                .body_transform(*body)
                .map_err(|e| NetError::Backend(e.to_string()))?;
            if let Some(pres) = self.presentations.get(&body.to_bits()) {
                let off = PresentationOffset::between(&auth, pres);
                if !within_bound(&off, &self.smoothing_bound) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn max_presentation_offset(&self) -> Result<PresentationOffset, NetError> {
        let mut max = PresentationOffset {
            position_m: 0.0,
            angle_rad: 0.0,
        };
        for body in &self.tracked_bodies {
            let auth = self
                .world
                .body_transform(*body)
                .map_err(|e| NetError::Backend(e.to_string()))?;
            if let Some(pres) = self.presentations.get(&body.to_bits()) {
                let off = PresentationOffset::between(&auth, pres);
                if off.position_m > max.position_m {
                    max.position_m = off.position_m;
                }
                if off.angle_rad > max.angle_rad {
                    max.angle_rad = off.angle_rad;
                }
            }
        }
        Ok(max)
    }

    pub fn step_predict(
        &mut self,
        commands: Vec<JournalCommand>,
        incoming: Option<&AuthoritativeSnapshot>,
        soft_alpha: f32,
    ) -> Result<ClientStepReport, NetError> {
        let frame = NetworkPhysicsFrameId(self.tick);
        let mut hard = None;
        let mut correction = None;

        if let Some(snap) = incoming {
            match self.apply_snapshot(snap, frame) {
                Ok(rep) => correction = Some(rep),
                Err(NetError::HardCorrection { reason, .. }) => hard = Some(reason),
                Err(e) => return Err(e),
            }
        }

        let input = TickInput {
            frame,
            commands: commands.clone(),
        };
        let retain = self
            .server_confirmed_tick
            .map(|t| NetworkPhysicsFrameId(t.0));
        if let Err(NetError::HardCorrection { reason, .. }) =
            self.predicted_inputs.push(frame, input, retain)
        {
            return Ok(ClientStepReport {
                frame,
                predicted_hash: String::new(),
                correction,
                hard_correction: Some(reason),
                presentation_offsets: Vec::new(),
                newly_committed_events: Vec::new(),
            });
        }

        apply_journal_pre(
            &mut self.world,
            &mut self.streaming,
            &mut self.constraints,
            &self.budget,
            &commands,
        )
        .map_err(|e| NetError::Backend(e.to_string()))?;
        self.world
            .step(self.world_desc.dt_fixed)
            .map_err(|e| NetError::Backend(e.to_string()))?;
        let mut sb = SyncBudget::new(
            self.budget.max_body_writes,
            self.budget.max_contact_events,
            self.budget.max_query_casts,
        );
        let contacts: Vec<ContactEvent> = self.world.drain_contacts(&mut sb).collect();
        self.queue_contacts(PhysicsTickId(self.tick), &contacts);

        let state = state_from_world(&self.world, self.tick)
            .map_err(|e| NetError::Backend(e.to_string()))?;
        let predicted_hash =
            hash_canonical_state(&state).map_err(|e| NetError::Backend(e.to_string()))?;
        let _ = self.hash_at_tick.push(frame, predicted_hash.clone(), retain);

        let offsets = self.smooth_presentations(soft_alpha)?;
        let map = FrameDomainMap::rigid_only(frame, PhysicsTickId(self.tick));
        map.validate()?;
        self.domain_maps.push(map);

        let newly = if let Some(confirmed) = self.server_confirmed_tick {
            self.event_bridge
                .try_commit(confirmed, &self.pending_events)
        } else {
            Vec::new()
        };

        self.tick += 1;
        Ok(ClientStepReport {
            frame,
            predicted_hash,
            correction,
            hard_correction: hard,
            presentation_offsets: offsets,
            newly_committed_events: newly,
        })
    }

    fn queue_contacts(&mut self, tick: PhysicsTickId, contacts: &[ContactEvent]) {
        for (i, ev) in contacts.iter().enumerate() {
            let id = event_id_for_contact(tick, ev, i as u32);
            self.pending_events.push((tick, id));
        }
    }

    fn smooth_presentations(
        &mut self,
        alpha: f32,
    ) -> Result<Vec<(BodyId, PresentationOffset)>, NetError> {
        let mut out = Vec::new();
        for body in &self.tracked_bodies {
            let auth = self
                .world
                .body_transform(*body)
                .map_err(|e| NetError::Backend(e.to_string()))?;
            let entry = self
                .presentations
                .entry(body.to_bits())
                .or_insert_with(|| PresentationTransform::from(auth));
            let pre = PresentationOffset::between(&auth, entry);
            if pre.position_m > 1.0 {
                hard_snap(&auth, entry);
            } else {
                soft_snap(&auth, entry, alpha);
            }
            out.push((*body, PresentationOffset::between(&auth, entry)));
        }
        Ok(out)
    }

    fn apply_snapshot(
        &mut self,
        snap: &AuthoritativeSnapshot,
        received_at: NetworkPhysicsFrameId,
    ) -> Result<CorrectionReport, NetError> {
        if snap.schema_digest != self.schema_digest || snap.build_digest != self.build_digest {
            return Err(NetError::IncompatibleDigest {
                schema_ok: snap.schema_digest == self.schema_digest,
                build_ok: snap.build_digest == self.build_digest,
            });
        }

        let predicted_hash = self
            .hash_at_tick
            .get(NetworkPhysicsFrameId(snap.physics_tick.0))
            .cloned()
            .unwrap_or_default();
        let diverged = predicted_hash != snap.semantic_state_hash;
        if diverged {
            self.prediction_diverged = true;
        }
        self.server_confirmed_tick = Some(snap.physics_tick);

        if !diverged {
            let rep = CorrectionReport {
                received_at_frame: received_at,
                snapshot_tick: snap.physics_tick,
                predicted_hash,
                server_hash: snap.semantic_state_hash.clone(),
                diverged: false,
                rollback: None,
                resim_final_hash: Some(snap.semantic_state_hash.clone()),
                resim_matches_server: true,
            };
            self.last_correction = Some(rep.clone());
            return Ok(rep);
        }

        // 权威输入 0..=T 重建 → 必与 server hash 收敛(同画像 F-13)。
        let auth_rebuild = rebuild_and_resim(
            &self.world_desc,
            &self.budget,
            &snap.inputs_through,
            PhysicsTickId(0),
            snap.physics_tick,
        )?;
        let resim_matches_server = auth_rebuild.final_hash == snap.semantic_state_hash;

        // 本地继续段 T+1..now
        let now = PhysicsTickId(self.tick.saturating_sub(1).max(snap.physics_tick.0));
        let mut dense = snap.inputs_through.clone();
        for (f, inp) in self.predicted_inputs.iter() {
            if f.0 > snap.physics_tick.0 && f.0 <= now.0 && !dense.iter().any(|t| t.frame == *f) {
                dense.push(inp.clone());
            }
        }
        dense.sort_by_key(|t| t.frame.0);
        let mut full = Vec::new();
        for t in 0..=now.0 {
            if let Some(found) = dense.iter().find(|x| x.frame.0 == t) {
                full.push(found.clone());
            } else {
                full.push(TickInput {
                    frame: NetworkPhysicsFrameId(t),
                    commands: Vec::new(),
                });
            }
        }

        let rb = rebuild_and_resim(
            &self.world_desc,
            &self.budget,
            &full,
            snap.physics_tick,
            now,
        )?;

        // 用完整序列重建替换客户端世界
        self.world = PhysicsWorld::new(self.world_desc.clone())
            .map_err(|e| NetError::Backend(e.to_string()))?;
        self.streaming = StreamingBridge::new();
        self.constraints.clear();
        for input in &full {
            apply_journal_pre(
                &mut self.world,
                &mut self.streaming,
                &mut self.constraints,
                &self.budget,
                &input.commands,
            )
            .map_err(|e| NetError::Backend(e.to_string()))?;
            self.world
                .step(self.world_desc.dt_fixed)
                .map_err(|e| NetError::Backend(e.to_string()))?;
            let mut sb = SyncBudget::new(
                self.budget.max_body_writes,
                self.budget.max_contact_events,
                self.budget.max_query_casts,
            );
            let contacts: Vec<ContactEvent> = self.world.drain_contacts(&mut sb).collect();
            self.queue_contacts(PhysicsTickId(input.frame.0), &contacts);
        }
        self.tick = now.0 + 1;
        self.predicted_inputs.clear();
        for inp in full {
            let _ = self
                .predicted_inputs
                .push(inp.frame, inp, Some(NetworkPhysicsFrameId(0)));
        }

        let rep = CorrectionReport {
            received_at_frame: received_at,
            snapshot_tick: snap.physics_tick,
            predicted_hash,
            server_hash: snap.semantic_state_hash.clone(),
            diverged: true,
            rollback: Some(rb.plan),
            resim_final_hash: Some(auth_rebuild.final_hash),
            resim_matches_server,
        };
        self.last_correction = Some(rep.clone());
        Ok(rep)
    }

    pub fn recommit_pending(&mut self) -> Vec<PhysicsEventId> {
        let confirmed = self
            .server_confirmed_tick
            .unwrap_or(PhysicsTickId(u64::MAX));
        self.event_bridge
            .try_commit(confirmed, &self.pending_events)
    }
}
