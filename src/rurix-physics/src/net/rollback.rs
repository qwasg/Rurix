//! rollback = journal 从 t0 重建至权威 tick + 重放 [T+1..now] 输入(§1 (c))。

use std::collections::HashMap;

use crate::bridge::StreamingBridge;
use crate::budget::SyncBudget;
use crate::capture::canonical::{hash_canonical_state, state_from_world};
use crate::capture::header::BudgetProfile;
use crate::capture::journal::JournalCommand;
use crate::capture::replayer::apply_journal_pre;
use crate::types::{ContactEvent, WorldDesc};
use crate::world::PhysicsWorld;

use super::frame::{NetworkPhysicsFrameId, PhysicsTickId};
use super::NetError;

#[derive(Debug, Clone, PartialEq)]
pub struct TickInput {
    pub frame: NetworkPhysicsFrameId,
    pub commands: Vec<JournalCommand>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RollbackPlan {
    pub start_tick: PhysicsTickId,
    pub input_sequence: Vec<TickInput>,
    pub resim_end_tick: PhysicsTickId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RollbackResult {
    pub plan: RollbackPlan,
    pub final_hash: String,
    pub contacts_by_tick: Vec<(PhysicsTickId, Vec<ContactEvent>)>,
}

/// 从 t0 按完整输入序列重建至 `resim_end`(含),权威段为 `0..=start`,其后为重放输入。
pub fn rebuild_and_resim(
    world_desc: &WorldDesc,
    budget: &BudgetProfile,
    // tick 0..resim_end 每个 tick 的 pre 命令(权威纠正后的完整视图)。
    full_inputs: &[TickInput],
    start_tick: PhysicsTickId,
    resim_end: PhysicsTickId,
) -> Result<RollbackResult, NetError> {
    if full_inputs.len() as u64 != resim_end.0 + 1 {
        return Err(NetError::Rejected(format!(
            "full_inputs len {} != resim_end+1 {}",
            full_inputs.len(),
            resim_end.0 + 1
        )));
    }
    let mut world =
        PhysicsWorld::new(world_desc.clone()).map_err(|e| NetError::Backend(e.to_string()))?;
    let mut streaming = StreamingBridge::new();
    let mut constraints = HashMap::new();
    let mut contacts_by_tick = Vec::new();
    let mut replay_seq = Vec::new();

    for tick in 0..=resim_end.0 {
        let input = &full_inputs[tick as usize];
        if input.frame.0 != tick {
            return Err(NetError::Rejected(format!(
                "input frame {} != tick {}",
                input.frame.0, tick
            )));
        }
        if tick > start_tick.0 {
            replay_seq.push(input.clone());
        }
        apply_journal_pre(
            &mut world,
            &mut streaming,
            &mut constraints,
            budget,
            &input.commands,
        )
        .map_err(|e| NetError::Backend(e.to_string()))?;
        world
            .step(world_desc.dt_fixed)
            .map_err(|e| NetError::Backend(e.to_string()))?;
        let mut sb = SyncBudget::new(
            budget.max_body_writes,
            budget.max_contact_events,
            budget.max_query_casts,
        );
        let events: Vec<ContactEvent> = world.drain_contacts(&mut sb).collect();
        contacts_by_tick.push((PhysicsTickId(tick), events));
    }

    let state = state_from_world(&world, resim_end.0).map_err(|e| NetError::Backend(e.to_string()))?;
    let final_hash = hash_canonical_state(&state).map_err(|e| NetError::Backend(e.to_string()))?;

    Ok(RollbackResult {
        plan: RollbackPlan {
            start_tick,
            input_sequence: replay_seq,
            resim_end_tick: resim_end,
        },
        final_hash,
        contacts_by_tick,
    })
}
