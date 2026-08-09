//! 权威世界:逐 tick 应用全量输入,按节奏发 snapshot。

use std::collections::HashMap;

use crate::bridge::StreamingBridge;
use crate::budget::SyncBudget;
use crate::capture::canonical::{CanonicalPhysicsState, hash_canonical_state, state_from_world};
use crate::capture::header::BudgetProfile;
use crate::capture::journal::JournalCommand;
use crate::capture::replayer::apply_journal_pre;
use crate::types::{ContactEvent, WorldDesc};
use crate::world::PhysicsWorld;

use super::NetError;
use super::frame::{FrameDomainMap, NetworkPhysicsFrameId, PhysicsTickId};
use super::history::HistoryRing;
use super::rollback::TickInput;

#[derive(Debug, Clone, PartialEq)]
pub struct AuthoritativeSnapshot {
    pub net_frame: NetworkPhysicsFrameId,
    pub physics_tick: PhysicsTickId,
    pub semantic_state_hash: String,
    pub schema_digest: String,
    pub build_digest: String,
    pub state: CanonicalPhysicsState,
    /// 权威输入视图(0..=tick),供客户端 correction。
    pub inputs_through: Vec<TickInput>,
}

pub struct ServerWorld {
    world: PhysicsWorld,
    streaming: StreamingBridge,
    constraints: HashMap<u64, u64>,
    budget: BudgetProfile,
    world_desc: WorldDesc,
    tick: u64,
    input_history: HistoryRing<TickInput>,
    schema_digest: String,
    build_digest: String,
    domain_maps: Vec<FrameDomainMap>,
    last_contacts: Vec<ContactEvent>,
}

impl ServerWorld {
    pub fn new(
        world_desc: WorldDesc,
        budget: BudgetProfile,
        history_capacity: usize,
        schema_digest: impl Into<String>,
        build_digest: impl Into<String>,
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
            input_history: HistoryRing::new(history_capacity)?,
            schema_digest: schema_digest.into(),
            build_digest: build_digest.into(),
            domain_maps: Vec::new(),
            last_contacts: Vec::new(),
        })
    }

    pub fn tick(&self) -> PhysicsTickId {
        PhysicsTickId(self.tick)
    }

    pub fn world(&self) -> &PhysicsWorld {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut PhysicsWorld {
        &mut self.world
    }

    pub fn domain_maps(&self) -> &[FrameDomainMap] {
        &self.domain_maps
    }

    pub fn last_contacts(&self) -> &[ContactEvent] {
        &self.last_contacts
    }

    pub fn schema_digest(&self) -> &str {
        &self.schema_digest
    }

    pub fn build_digest(&self) -> &str {
        &self.build_digest
    }

    pub fn world_desc(&self) -> &WorldDesc {
        &self.world_desc
    }

    pub fn budget(&self) -> &BudgetProfile {
        &self.budget
    }

    pub fn all_inputs_through(&self, through: PhysicsTickId) -> Vec<TickInput> {
        self.input_history
            .iter()
            .filter(|(f, _)| f.0 <= through.0)
            .map(|(_, v)| v.clone())
            .collect()
    }

    pub fn step(&mut self, commands: Vec<JournalCommand>) -> Result<String, NetError> {
        let frame = NetworkPhysicsFrameId(self.tick);
        let input = TickInput {
            frame,
            commands: commands.clone(),
        };
        // 权威端 ring 保留从 0 起的完整窗口;溢出 → hard correction 显式。
        self.input_history
            .push(frame, input, Some(NetworkPhysicsFrameId(0)))?;
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
        self.last_contacts = self.world.drain_contacts(&mut sb).collect();
        let state = state_from_world(&self.world, self.tick)
            .map_err(|e| NetError::Backend(e.to_string()))?;
        let hash = hash_canonical_state(&state).map_err(|e| NetError::Backend(e.to_string()))?;
        let map = FrameDomainMap::rigid_only(frame, PhysicsTickId(self.tick));
        map.validate()?;
        self.domain_maps.push(map);
        self.tick += 1;
        Ok(hash)
    }

    pub fn emit_snapshot(&self) -> Result<AuthoritativeSnapshot, NetError> {
        if self.tick == 0 {
            return Err(NetError::Rejected("no ticks stepped yet".into()));
        }
        let physics_tick = PhysicsTickId(self.tick - 1);
        let state = state_from_world(&self.world, physics_tick.0)
            .map_err(|e| NetError::Backend(e.to_string()))?;
        let semantic_state_hash =
            hash_canonical_state(&state).map_err(|e| NetError::Backend(e.to_string()))?;
        Ok(AuthoritativeSnapshot {
            net_frame: NetworkPhysicsFrameId(physics_tick.0),
            physics_tick,
            semantic_state_hash,
            schema_digest: self.schema_digest.clone(),
            build_digest: self.build_digest.clone(),
            state,
            inputs_through: self.all_inputs_through(physics_tick),
        })
    }
}
