//! Strain 断键 + hierarchical cluster 激活(RFC-0021 §4.C2;CPU host)。
//!
//! strain 源 = journal damage 命令(+ 点距衰减),不读求解器 impulse(VENDOR 恒 0)。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use rurix_pkg::sha256::{digest, hex};

use super::cache::{CacheTickRecord, DestructionCache};
use super::schema::DestructionCookedArtifact;
use super::vfx::{FractureEvent, VfxBridge};

#[derive(Debug)]
pub enum RuntimeError {
    BadCommand(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadCommand(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DamageCommand {
    pub tick: u64,
    pub point: [f32; 3],
    pub radius: f32,
    pub magnitude: f32,
}

#[derive(Debug, Clone)]
pub struct ActivatedBodyRecord {
    pub body_stable_id: String,
    pub cluster_id: String,
    pub chunk_ids: Vec<String>,
    pub tick: u64,
}

#[derive(Debug, Clone, Default)]
pub struct FractureTickReport {
    pub tick: u64,
    pub broken_edge_ids: Vec<String>,
    pub activated_cluster_ids: Vec<String>,
    pub new_events: Vec<FractureEvent>,
}

#[derive(Debug)]
pub struct FracturePipeline {
    cooked: DestructionCookedArtifact,
    /// edge_id → accumulated strain
    strain: BTreeMap<String, f32>,
    broken: BTreeSet<String>,
    activated_clusters: BTreeSet<String>,
    activated_bodies: Vec<ActivatedBodyRecord>,
    journal: Vec<String>,
    pending_commands: BTreeMap<u64, Vec<DamageCommand>>,
    cache_records: Vec<CacheTickRecord>,
    vfx: VfxBridge,
    fracture_events: Vec<FractureEvent>,
    event_ordinal: u32,
}

impl FracturePipeline {
    pub fn new(cooked: DestructionCookedArtifact) -> Self {
        let mut strain = BTreeMap::new();
        for e in &cooked.edges {
            strain.insert(e.edge_id.clone(), 0.0);
        }
        Self {
            cooked,
            strain,
            broken: BTreeSet::new(),
            activated_clusters: BTreeSet::new(),
            activated_bodies: Vec::new(),
            journal: Vec::new(),
            pending_commands: BTreeMap::new(),
            cache_records: Vec::new(),
            vfx: VfxBridge::new(),
            fracture_events: Vec::new(),
            event_ordinal: 0,
        }
    }

    pub fn apply_damage(&mut self, cmd: DamageCommand) -> Result<(), RuntimeError> {
        if cmd.magnitude < 0.0 || cmd.radius < 0.0 {
            return Err(RuntimeError::BadCommand("negative damage".into()));
        }
        self.journal.push(format!(
            "damage:tick={}:mag={:.6}:r={:.6}",
            cmd.tick, cmd.magnitude, cmd.radius
        ));
        self.pending_commands.entry(cmd.tick).or_default().push(cmd);
        Ok(())
    }

    pub fn step(&mut self, tick: u64) -> Result<FractureTickReport, RuntimeError> {
        let cmds = self.pending_commands.remove(&tick).unwrap_or_default();
        // 按 edge_id 升序累积 strain(确定性)
        let mut edge_order: Vec<_> = self.cooked.edges.iter().collect();
        edge_order.sort_by(|a, b| a.edge_id.cmp(&b.edge_id));

        for cmd in &cmds {
            for e in &edge_order {
                if self.broken.contains(&e.edge_id) {
                    continue;
                }
                let ca = self.cooked.chunks.iter().find(|c| c.chunk_id == e.chunk_a);
                let cb = self.cooked.chunks.iter().find(|c| c.chunk_id == e.chunk_b);
                let (Some(a), Some(b)) = (ca, cb) else {
                    continue;
                };
                let mid = [
                    (a.center[0] + b.center[0]) * 0.5,
                    (a.center[1] + b.center[1]) * 0.5,
                    (a.center[2] + b.center[2]) * 0.5,
                ];
                let dist = dist3(cmd.point, mid);
                if dist > cmd.radius {
                    continue;
                }
                let falloff = 1.0 - dist / cmd.radius;
                let add = cmd.magnitude * falloff * e.contact_area;
                *self.strain.entry(e.edge_id.clone()).or_default() += add;
            }
        }

        let mut broken_now = Vec::new();
        for e in &edge_order {
            if self.broken.contains(&e.edge_id) {
                continue;
            }
            let s = *self.strain.get(&e.edge_id).unwrap_or(&0.0);
            if s > e.strength {
                self.broken.insert(e.edge_id.clone());
                broken_now.push(e.edge_id.clone());
                self.journal
                    .push(format!("break_edge:{}:tick={}", e.edge_id, tick));
            }
        }

        let activated_now = self.recompute_activation(tick, &broken_now);

        let mut new_events = Vec::new();
        for (i, edge_id) in broken_now.iter().enumerate() {
            let edge = self
                .cooked
                .edges
                .iter()
                .find(|e| e.edge_id == *edge_id)
                .expect("edge");
            let cluster_id = activated_now
                .first()
                .cloned()
                .unwrap_or_else(|| "root".into());
            let ordinal = self.event_ordinal + i as u32;
            let ev = FractureEvent {
                event_id: FractureEvent::derive_id(tick, edge_id, ordinal),
                tick,
                edge_id: edge_id.clone(),
                chunks: vec![edge.chunk_a.clone(), edge.chunk_b.clone()],
                cluster_id,
            };
            new_events.push(ev);
        }
        self.event_ordinal += broken_now.len() as u32;
        self.fracture_events.extend(new_events.clone());
        self.vfx.try_commit(&new_events);

        let state_hash = self.state_hash();
        self.cache_records.push(CacheTickRecord {
            tick,
            commands: cmds,
            events: new_events.clone(),
            state_hash,
        });

        Ok(FractureTickReport {
            tick,
            broken_edge_ids: broken_now,
            activated_cluster_ids: activated_now,
            new_events,
        })
    }

    /// union-find 重算连通;脱离锚接分量按 cluster_tree 激活。
    fn recompute_activation(&mut self, tick: u64, newly_broken: &[String]) -> Vec<String> {
        if newly_broken.is_empty() {
            return Vec::new();
        }
        let chunk_ids: Vec<String> = self
            .cooked
            .chunks
            .iter()
            .map(|c| c.chunk_id.clone())
            .collect();
        let mut parent: BTreeMap<String, String> = chunk_ids
            .iter()
            .map(|id| (id.clone(), id.clone()))
            .collect();

        fn find(p: &mut BTreeMap<String, String>, x: &str) -> String {
            let par = p.get(x).cloned().unwrap_or_else(|| x.to_string());
            if par == x {
                return par;
            }
            let root = find(p, &par);
            p.insert(x.to_string(), root.clone());
            root
        }
        fn union(p: &mut BTreeMap<String, String>, a: &str, b: &str) {
            let ra = find(p, a);
            let rb = find(p, b);
            if ra != rb {
                // 稳定:字典序小者为根
                if ra < rb {
                    p.insert(rb, ra);
                } else {
                    p.insert(ra, rb);
                }
            }
        }

        for e in &self.cooked.edges {
            if self.broken.contains(&e.edge_id) {
                continue;
            }
            union(&mut parent, &e.chunk_a, &e.chunk_b);
        }

        let anchored: BTreeSet<String> = self
            .cooked
            .anchors
            .iter()
            .filter(|a| a.world_static)
            .map(|a| find(&mut parent, &a.chunk_id))
            .collect();

        // 按 activation_depth 降序考虑 cluster(深层先激活)
        let mut clusters: Vec<_> = self.cooked.clusters.iter().collect();
        clusters.sort_by(|a, b| {
            b.activation_depth
                .cmp(&a.activation_depth)
                .then_with(|| a.cluster_id.cmp(&b.cluster_id))
        });

        let mut activated_now = Vec::new();
        for c in clusters {
            if c.leaf_chunks.is_empty() {
                continue;
            }
            if self.activated_clusters.contains(&c.cluster_id) {
                continue;
            }
            // 若任一 leaf 不在锚定分量 → 该 cluster 可激活
            let mut free = false;
            for leaf in &c.leaf_chunks {
                let root = find(&mut parent, leaf);
                if !anchored.contains(&root) {
                    free = true;
                    break;
                }
            }
            if !free {
                continue;
            }
            // 仅当 newly broken 触及该 cluster 的 leaf 时激活(避免阈下误激活)
            let leaves: BTreeSet<_> = c.leaf_chunks.iter().cloned().collect();
            let touched = newly_broken.iter().any(|eid| {
                self.cooked
                    .edges
                    .iter()
                    .find(|e| e.edge_id == *eid)
                    .map(|e| leaves.contains(&e.chunk_a) || leaves.contains(&e.chunk_b))
                    .unwrap_or(false)
            });
            if !touched {
                continue;
            }
            self.activated_clusters.insert(c.cluster_id.clone());
            activated_now.push(c.cluster_id.clone());
            let body_id = format!("body:{}:{}", c.cluster_id, tick);
            self.journal.push(format!(
                "activate_body:{}:cluster={}",
                body_id, c.cluster_id
            ));
            self.activated_bodies.push(ActivatedBodyRecord {
                body_stable_id: body_id,
                cluster_id: c.cluster_id.clone(),
                chunk_ids: c.leaf_chunks.clone(),
                tick,
            });
        }
        activated_now.sort();
        activated_now
    }

    pub fn broken_edges(&self) -> &BTreeSet<String> {
        &self.broken
    }

    pub fn activated_cluster_ids(&self) -> Vec<String> {
        self.activated_clusters.iter().cloned().collect()
    }

    pub fn activated_bodies(&self) -> &[ActivatedBodyRecord] {
        &self.activated_bodies
    }

    pub fn journal_lines(&self) -> &[String] {
        &self.journal
    }

    pub fn cache_records(&self) -> &[CacheTickRecord] {
        &self.cache_records
    }

    pub fn export_cache(&self) -> DestructionCache {
        DestructionCache::from_pipeline(self)
    }

    pub fn vfx_commit_count(&self) -> usize {
        self.vfx.commit_count()
    }

    pub fn vfx_duplicate_count(&self) -> u64 {
        self.vfx.duplicate_count()
    }

    pub fn fracture_event_count(&self) -> usize {
        self.fracture_events.len()
    }

    pub fn event_sequence_digest(&self) -> String {
        self.vfx.sequence_digest()
    }

    pub fn recommit_vfx_for_tick(&mut self, tick: u64) {
        let evs: Vec<_> = self
            .fracture_events
            .iter()
            .filter(|e| e.tick == tick)
            .cloned()
            .collect();
        self.vfx.try_commit(&evs);
    }

    pub fn state_hash(&self) -> String {
        let mut buf = Vec::new();
        for (k, v) in &self.strain {
            buf.extend_from_slice(k.as_bytes());
            buf.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        for e in &self.broken {
            buf.extend_from_slice(e.as_bytes());
        }
        for c in &self.activated_clusters {
            buf.extend_from_slice(c.as_bytes());
        }
        for b in &self.activated_bodies {
            buf.extend_from_slice(b.body_stable_id.as_bytes());
        }
        hex(&digest(&buf))
    }
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}
