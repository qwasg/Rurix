//! CaptureRecorder:逐步记 journal + canonical hash。

use std::fs;
use std::path::Path;

use super::canonical::{
    CanonicalPhysicsState, CaptureError, event_digest, hash_canonical_state, state_from_world,
};
use super::header::{BudgetProfile, PhysicsCaptureHeader};
use super::journal::{JournalCommand, JournalTick, PostTick};
use crate::budget::SyncBudget;
use crate::types::{ContactEvent, WorldDesc};
use crate::world::PhysicsWorld;

#[derive(Debug, Clone)]
pub struct CaptureArtifact {
    pub header: PhysicsCaptureHeader,
    pub ticks: Vec<JournalTick>,
    pub state0: CanonicalPhysicsState,
    pub state_final: CanonicalPhysicsState,
}

pub struct CaptureRecorder {
    header: PhysicsCaptureHeader,
    ticks: Vec<JournalTick>,
    state0: Option<CanonicalPhysicsState>,
    pending_pre: Vec<JournalCommand>,
    budget: BudgetProfile,
}

impl CaptureRecorder {
    pub fn begin(
        scenario_id: &str,
        tick_count: u64,
        world_desc: &WorldDesc,
        build_fingerprint: &str,
        abi_digest: &str,
        budget: BudgetProfile,
    ) -> Self {
        CaptureRecorder {
            header: PhysicsCaptureHeader::new_jolt_53(
                scenario_id,
                tick_count,
                world_desc,
                build_fingerprint,
                abi_digest,
                budget.clone(),
            ),
            ticks: Vec::with_capacity(tick_count as usize),
            state0: None,
            pending_pre: Vec::new(),
            budget,
        }
    }

    pub fn header(&self) -> &PhysicsCaptureHeader {
        &self.header
    }

    pub fn push_command(&mut self, cmd: JournalCommand) {
        self.pending_pre.push(cmd);
    }

    pub fn take_pending(&mut self) -> Vec<JournalCommand> {
        std::mem::take(&mut self.pending_pre)
    }

    /// step 后封存本 tick:drain 接触 → hash → journal 行。
    pub fn seal_tick(
        &mut self,
        world: &mut PhysicsWorld,
        tick: u64,
        pre: Vec<JournalCommand>,
        contacts_emitted: u32,
        contacts_dropped: u64,
    ) -> Result<(), CaptureError> {
        let mut budget = SyncBudget::new(
            self.budget.max_body_writes,
            self.budget.max_contact_events,
            self.budget.max_query_casts,
        );
        let events: Vec<ContactEvent> = world.drain_contacts(&mut budget).collect();
        let state = state_from_world(world, tick)?;
        if tick == 0 && self.state0.is_none() {
            self.state0 = Some(state.clone());
        }
        let semantic_state_hash = hash_canonical_state(&state)?;
        let event_d = event_digest(&events)?;
        let sat = world.budget_saturation();
        self.ticks.push(JournalTick {
            tick,
            pre,
            post: PostTick {
                semantic_state_hash,
                event_digest: event_d,
                contacts_emitted,
                contacts_dropped,
                ring_backlog: world.contact_ring_len() as u32,
                saturation_query_casts: sat.query_casts,
                saturation_contact_events: sat.contact_events,
                saturation_body_writes: sat.body_writes,
            },
        });
        Ok(())
    }

    pub fn finish(mut self, world: &PhysicsWorld) -> Result<CaptureArtifact, CaptureError> {
        let state0 = self
            .state0
            .take()
            .ok_or_else(|| CaptureError::Mismatch("missing state0".into()))?;
        let final_tick = self.header.tick_count.saturating_sub(1);
        let state_final = state_from_world(world, final_tick)?;
        Ok(CaptureArtifact {
            header: self.header,
            ticks: self.ticks,
            state0,
            state_final,
        })
    }
}

impl CaptureArtifact {
    pub fn persist(&self, dir: &Path) -> Result<(), CaptureError> {
        fs::create_dir_all(dir).map_err(|e| CaptureError::Io(e.to_string()))?;
        fs::write(dir.join("header.json"), self.header.to_canonical_json()?)
            .map_err(|e| CaptureError::Io(e.to_string()))?;
        let mut journal = String::new();
        for t in &self.ticks {
            journal.push_str(&t.to_json_line()?);
            journal.push('\n');
        }
        fs::write(dir.join("journal.jsonl"), journal)
            .map_err(|e| CaptureError::Io(e.to_string()))?;
        fs::write(dir.join("state0.json"), self.state0.to_diagnostic_json()?)
            .map_err(|e| CaptureError::Io(e.to_string()))?;
        fs::write(
            dir.join("state_final.json"),
            self.state_final.to_diagnostic_json()?,
        )
        .map_err(|e| CaptureError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn load(dir: &Path) -> Result<Self, CaptureError> {
        let header_text = fs::read_to_string(dir.join("header.json"))
            .map_err(|e| CaptureError::Io(e.to_string()))?;
        let header = PhysicsCaptureHeader::parse_json(&header_text)?;
        header.validate_complete()?;
        let journal_text = fs::read_to_string(dir.join("journal.jsonl"))
            .map_err(|e| CaptureError::Io(e.to_string()))?;
        let mut ticks = Vec::new();
        for (i, line) in journal_text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            ticks.push(
                JournalTick::parse_json_line(line)
                    .map_err(|e| CaptureError::Parse(format!("journal line {}: {e}", i + 1)))?,
            );
        }
        let state0_text = fs::read_to_string(dir.join("state0.json"))
            .map_err(|e| CaptureError::Io(e.to_string()))?;
        let state_final_text = fs::read_to_string(dir.join("state_final.json"))
            .map_err(|e| CaptureError::Io(e.to_string()))?;
        let state0_tick = parse_state_tick(&state0_text)?;
        let state_final_tick = parse_state_tick(&state_final_text)?;
        Ok(CaptureArtifact {
            header,
            ticks,
            state0: super::canonical::empty_state(state0_tick),
            state_final: super::canonical::empty_state(state_final_tick),
        })
    }
}

fn parse_state_tick(text: &str) -> Result<u64, CaptureError> {
    let key = "\"tick\"";
    let i = text
        .find(key)
        .ok_or_else(|| CaptureError::Parse("state tick".into()))?;
    let rest =
        text[i + key.len()..].trim_start_matches(|c: char| c == ' ' || c == ':' || c == '\n');
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end]
        .parse()
        .map_err(|e| CaptureError::Parse(format!("state tick: {e}")))
}

/// 便利:按 contact_capacity 构造默认 budget 画像。
pub fn default_budget_profile(contact_capacity: u32) -> BudgetProfile {
    BudgetProfile {
        contact_capacity,
        max_query_casts: 4096,
        max_contact_events: contact_capacity.max(1),
        max_body_writes: 65_536,
    }
}

pub fn default_budget(world: &WorldDesc) -> BudgetProfile {
    default_budget_profile(world.contact_capacity)
}
