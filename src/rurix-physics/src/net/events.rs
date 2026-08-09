//! PhysicsEventId 派生 + commit boundary 去重(RFC-0021 Q2 / §4.B1)。

use std::collections::BTreeSet;

use rurix_pkg::sha256::{digest, hex};

use crate::types::{ContactEvent, ContactPhase};

use super::frame::PhysicsTickId;

/// 事件类型标签(v1:contact phases + gameplay cue 占位)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PhysicsEventKind {
    ContactBegin = 1,
    ContactPersist = 2,
    ContactEnd = 3,
    GameplayCue = 4,
}

impl PhysicsEventKind {
    pub fn from_phase(p: ContactPhase) -> Self {
        match p {
            ContactPhase::Begin => Self::ContactBegin,
            ContactPhase::Persist => Self::ContactPersist,
            ContactPhase::End => Self::ContactEnd,
        }
    }
}

/// 稳定事件 ID(类型 + 权威 tick + 参与方 generation 排序 + tick 内 ordinal)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicsEventId(pub String);

pub fn derive_physics_event_id(
    kind: PhysicsEventKind,
    tick: PhysicsTickId,
    participant_gens: &[u64],
    ordinal: u32,
) -> PhysicsEventId {
    let mut gens = participant_gens.to_vec();
    gens.sort_unstable();
    let mut buf = Vec::with_capacity(32 + gens.len() * 8);
    buf.push(kind as u8);
    buf.extend_from_slice(&tick.0.to_le_bytes());
    buf.extend_from_slice(&ordinal.to_le_bytes());
    for g in gens {
        buf.extend_from_slice(&g.to_le_bytes());
    }
    PhysicsEventId(hex(&digest(&buf)))
}

pub fn event_id_for_contact(
    tick: PhysicsTickId,
    ev: &ContactEvent,
    ordinal: u32,
) -> PhysicsEventId {
    let a = ev.a.to_bits();
    let b = ev.b.to_bits();
    derive_physics_event_id(
        PhysicsEventKind::from_phase(ev.phase),
        tick,
        &[a, b],
        ordinal,
    )
}

/// 提交桥:仅当 tick ≤ server_confirmed 时对外发布;按 PhysicsEventId 恰好一次。
#[derive(Debug, Default)]
pub struct EventCommitBridge {
    committed: BTreeSet<PhysicsEventId>,
    published: Vec<PhysicsEventId>,
}

impl EventCommitBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// 尝试提交一批内部事件;已提交或未越过边界的跳过。
    pub fn try_commit(
        &mut self,
        server_confirmed_tick: PhysicsTickId,
        pending: &[(PhysicsTickId, PhysicsEventId)],
    ) -> Vec<PhysicsEventId> {
        let mut newly = Vec::new();
        for (tick, id) in pending {
            if *tick > server_confirmed_tick {
                continue;
            }
            if self.committed.insert(id.clone()) {
                self.published.push(id.clone());
                newly.push(id.clone());
            }
        }
        newly
    }

    pub fn published(&self) -> &[PhysicsEventId] {
        &self.published
    }

    pub fn published_count(&self) -> usize {
        self.published.len()
    }

    pub fn is_committed(&self, id: &PhysicsEventId) -> bool {
        self.committed.contains(id)
    }
}
