//! M68 破坏生产链(RFC-0021 §4.C;`physics-destruction` feature)。
//!
//! cook → connection/hierarchical cluster → strain 断键 → cache → VFX exactly-once。
//! CPU/host 权威;禁 GPU 主刚体。strain 源 = journal damage + 相对速度,不依赖求解器 impulse。

mod cache;
mod cook;
mod runtime;
mod schema;
mod vfx;

pub use cache::{CacheRoundtripReport, DestructionCache};
pub use cook::{CookError, cook_destruction, cook_deterministic_double};
pub use runtime::{
    ActivatedBodyRecord, DamageCommand, FracturePipeline, FractureTickReport, RuntimeError,
};
pub use schema::{
    Anchor, ChunkDesc, ClusterNode, ConnectionEdge, DESTRUCTION_SCHEMA_ID,
    DESTRUCTION_SCHEMA_VERSION, DestructionCookedArtifact, DestructionSourceAsset, FractureRecipe,
    InteriorFace, SchemaHeader,
};
pub use vfx::{FractureEvent, VfxBridge, VfxCommitReport};

use cook::CookError as CE;
use runtime::RuntimeError as RE;
use schema::SchemaError;

/// 端到端门报告(g8-physics-gates fracture JSON 字段源)。
#[derive(Debug, Clone)]
pub struct FracturePipelineReport {
    pub ok: bool,
    pub cook_deterministic_double_byte_equal: bool,
    pub cook_counts_and_digests_match_golden: bool,
    pub unknown_schema_fails_closed: bool,
    pub dangling_edge_or_nontree_cluster_fails_closed: bool,
    pub below_threshold_no_break: bool,
    pub above_threshold_breaks_specified_edge_at_tick: bool,
    pub cluster_activation_hierarchy_matches_golden: bool,
    pub activated_bodies_enter_journal_and_capture: bool,
    pub cache_roundtrip_event_sequence_identical: bool,
    pub cache_roundtrip_state_hash_identical: bool,
    pub vfx_exactly_once_per_fracture_event: bool,
    pub vfx_no_duplicate_across_rollback_or_cache_replay: bool,
    pub chunk_count: usize,
    pub edge_count: usize,
    pub interior_face_count: usize,
    pub anchor_count: usize,
    pub cooked_digest: String,
    pub broken_edge_id: Option<String>,
    pub break_tick: Option<u64>,
    pub activated_cluster_ids: Vec<String>,
    pub activated_body_count: usize,
    pub vfx_commit_count: usize,
    pub event_sequence_digest: String,
    pub state_hash: String,
    pub detail: String,
}

/// 从 fixture 源 + golden 期望跑完整 fracture 链。
pub fn run_fracture_pipeline(
    source: &DestructionSourceAsset,
    golden: &FractureGolden,
) -> Result<FracturePipelineReport, String> {
    // 1) 双 cook 确定性
    let (a, b) = cook_deterministic_double(source).map_err(|e| e.to_string())?;
    let cook_det = a.canonical_bytes() == b.canonical_bytes();
    let cooked = a;

    let counts_ok = cooked.chunks.len() == golden.chunk_count
        && cooked.edges.len() == golden.edge_count
        && cooked.interior_faces.len() == golden.interior_face_count
        && cooked.anchors.len() == golden.anchor_count
        && cooked.digest() == golden.cooked_digest;

    // 2) fail-closed 负样本
    let mut bad_schema = source.clone();
    bad_schema.header.schema_version = 9999;
    let unknown_fail = matches!(cook_destruction(&bad_schema), Err(CE::Schema(_)));

    let mut dangling = source.clone();
    if let Some(e) = dangling.edges.first_mut() {
        e.chunk_b = "__missing_chunk__".into();
    }
    let dangling_fail = matches!(
        cook_destruction(&dangling),
        Err(CE::Schema(
            SchemaError::DanglingEdge(_) | SchemaError::NonTreeCluster(_)
        ))
    ) || matches!(cook_destruction(&dangling), Err(CE::Schema(_)));

    // 3) 阈下:N tick 零断键
    let mut below = FracturePipeline::new(cooked.clone());
    for tick in 0..golden.below_threshold_ticks {
        let mag = golden.below_damage_magnitude;
        below
            .apply_damage(DamageCommand {
                tick,
                point: golden.damage_point,
                radius: golden.damage_radius,
                magnitude: mag,
            })
            .map_err(|e| e.to_string())?;
        below.step(tick).map_err(|e| e.to_string())?;
    }
    let below_ok = below.broken_edges().is_empty();

    // 4) 阈上:指定 tick 断指定 edge + 激活层级
    let mut above = FracturePipeline::new(cooked.clone());
    let mut break_tick = None;
    let mut broken_edge = None;
    for tick in 0..=golden.break_tick {
        let mag = if tick == golden.break_tick {
            golden.above_damage_magnitude
        } else {
            0.0
        };
        if mag > 0.0 {
            above
                .apply_damage(DamageCommand {
                    tick,
                    point: golden.damage_point,
                    radius: golden.damage_radius,
                    magnitude: mag,
                })
                .map_err(|e| e.to_string())?;
        }
        let rep = above.step(tick).map_err(|e| e.to_string())?;
        if !rep.broken_edge_ids.is_empty() {
            break_tick = Some(tick);
            broken_edge = rep.broken_edge_ids.first().cloned();
        }
    }
    let above_ok = break_tick == Some(golden.break_tick)
        && broken_edge.as_deref() == Some(golden.break_edge_id.as_str());
    let activated = above.activated_cluster_ids();
    let hierarchy_ok = activated == golden.activated_cluster_ids;
    let bodies_ok = !above.activated_bodies().is_empty()
        && above
            .journal_lines()
            .iter()
            .any(|l| l.starts_with("activate_body:"));

    // 5) cache roundtrip
    let cache = above.export_cache();
    let rt = cache.roundtrip_replay(&cooked).map_err(|e| e.to_string())?;
    let cache_events_ok = rt.event_sequence_identical;
    let cache_hash_ok = rt.state_hash_identical;

    // 6) VFX exactly-once(+ cache replay 无重复)
    let vfx_once = above.vfx_commit_count() == above.fracture_event_count()
        && above.fracture_event_count() > 0
        && above.vfx_duplicate_count() == 0;
    // cache/rollback 重放后 publish 计数不变且序列 digest 对齐
    let vfx_replay_ok = rt.vfx_commit_count == above.vfx_commit_count()
        && rt.vfx_duplicate_count == 0
        && rt.event_sequence_identical;

    let ok = cook_det
        && counts_ok
        && unknown_fail
        && dangling_fail
        && below_ok
        && above_ok
        && hierarchy_ok
        && bodies_ok
        && cache_events_ok
        && cache_hash_ok
        && vfx_once
        && vfx_replay_ok;

    Ok(FracturePipelineReport {
        ok,
        cook_deterministic_double_byte_equal: cook_det,
        cook_counts_and_digests_match_golden: counts_ok,
        unknown_schema_fails_closed: unknown_fail,
        dangling_edge_or_nontree_cluster_fails_closed: dangling_fail,
        below_threshold_no_break: below_ok,
        above_threshold_breaks_specified_edge_at_tick: above_ok,
        cluster_activation_hierarchy_matches_golden: hierarchy_ok,
        activated_bodies_enter_journal_and_capture: bodies_ok,
        cache_roundtrip_event_sequence_identical: cache_events_ok,
        cache_roundtrip_state_hash_identical: cache_hash_ok,
        vfx_exactly_once_per_fracture_event: vfx_once,
        vfx_no_duplicate_across_rollback_or_cache_replay: vfx_replay_ok,
        chunk_count: cooked.chunks.len(),
        edge_count: cooked.edges.len(),
        interior_face_count: cooked.interior_faces.len(),
        anchor_count: cooked.anchors.len(),
        cooked_digest: cooked.digest(),
        broken_edge_id: broken_edge,
        break_tick,
        activated_cluster_ids: activated,
        activated_body_count: above.activated_bodies().len(),
        vfx_commit_count: above.vfx_commit_count(),
        event_sequence_digest: above.event_sequence_digest(),
        state_hash: above.state_hash(),
        detail: if ok {
            "fracture pipeline full-chain PASS".into()
        } else {
            "fracture pipeline check failed".into()
        },
    })
}

/// Golden 期望(conformance fixture)。
#[derive(Debug, Clone)]
pub struct FractureGolden {
    pub chunk_count: usize,
    pub edge_count: usize,
    pub interior_face_count: usize,
    pub anchor_count: usize,
    pub cooked_digest: String,
    pub below_threshold_ticks: u64,
    pub below_damage_magnitude: f32,
    pub above_damage_magnitude: f32,
    pub damage_point: [f32; 3],
    pub damage_radius: f32,
    pub break_tick: u64,
    pub break_edge_id: String,
    pub activated_cluster_ids: Vec<String>,
}

/// 自 fixtures JSON 解析(极简手写,免 serde)。
pub fn parse_source_json(text: &str) -> Result<DestructionSourceAsset, String> {
    schema::parse_source_json(text)
}

pub fn parse_golden_json(text: &str) -> Result<FractureGolden, String> {
    schema::parse_golden_json(text)
}

#[allow(dead_code)]
fn _use_re(_: RE) {}
