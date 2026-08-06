//! FractureEvent VFX 桥:按 PhysicsEventId 恰好提交一次(RFC-0021 §4.C2)。

use std::collections::BTreeSet;

use rurix_pkg::sha256::{digest, hex};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FractureEvent {
    pub event_id: String,
    pub tick: u64,
    pub edge_id: String,
    pub chunks: Vec<String>,
    pub cluster_id: String,
}

impl FractureEvent {
    pub fn derive_id(tick: u64, edge_id: &str, ordinal: u32) -> String {
        let mut buf = Vec::new();
        buf.push(0xFEu8); // fracture kind tag
        buf.extend_from_slice(&tick.to_le_bytes());
        buf.extend_from_slice(&ordinal.to_le_bytes());
        buf.extend_from_slice(edge_id.as_bytes());
        hex(&digest(&buf))
    }
}

#[derive(Debug, Default)]
pub struct VfxBridge {
    committed: BTreeSet<String>,
    published: Vec<FractureEvent>,
    duplicate_attempts: u64,
}

#[derive(Debug, Clone)]
pub struct VfxCommitReport {
    pub newly_committed: usize,
    pub total_committed: usize,
    pub duplicate_attempts: u64,
}

impl VfxBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// 尝试提交;已见 event_id 计 duplicate,不二次发布。
    pub fn try_commit(&mut self, events: &[FractureEvent]) -> VfxCommitReport {
        let mut newly = 0usize;
        for ev in events {
            if self.committed.contains(&ev.event_id) {
                self.duplicate_attempts += 1;
                continue;
            }
            self.committed.insert(ev.event_id.clone());
            self.published.push(ev.clone());
            newly += 1;
        }
        VfxCommitReport {
            newly_committed: newly,
            total_committed: self.committed.len(),
            duplicate_attempts: self.duplicate_attempts,
        }
    }

    pub fn published(&self) -> &[FractureEvent] {
        &self.published
    }

    pub fn duplicate_count(&self) -> u64 {
        self.duplicate_attempts
    }

    pub fn commit_count(&self) -> usize {
        self.committed.len()
    }

    pub fn sequence_digest(&self) -> String {
        let mut buf = Vec::new();
        for ev in &self.published {
            buf.extend_from_slice(ev.event_id.as_bytes());
            buf.push(b'|');
            buf.extend_from_slice(&ev.tick.to_le_bytes());
            buf.push(b'|');
            buf.extend_from_slice(ev.edge_id.as_bytes());
            buf.push(b';');
        }
        hex(&digest(&buf))
    }
}
