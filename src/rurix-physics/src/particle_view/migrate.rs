//! M68 damage journal → particle_view consumer 迁移器(RFC-0024 §4.A 末条;
//! 骨架期加性迁移,源面 0-byte)。
//!
//! 语义(判据事实源 = MAP M121 行「M68 damage journal 迁移为首个 consumer
//! 后,迁移前后逐 tick digest 与 golden 一致、journal 全消费无损」):
//! - **迁移前后同语义**:旧路径 = `FracturePipeline::apply_damage` 直推;
//!   新路径 = 同一 `DamageCommand` 经 [`migrate_damage_command`] 记为
//!   `ParticleViewCommand`(dest_ref = damage 点距最近 chunk 的稳定 ref,
//!   确定域语义),再经 [`replay_migrated`] 逐 tick 重放到 **全新**
//!   `FracturePipeline`;两路径逐 tick `state_hash` 必须逐位一致
//!   (golden 面 = conformance/physics/particle_view/m68_migration/golden.json)。
//! - **journal 全消费无损**:迁移记账 = 仅 `damage` 行(消费者 = 本视图,
//!   管线自身记账行 `break_edge:*`/`activate_body:*` 不参与——它们不是
//!   damage consumer 的输入);`damage` 行数 == 迁移命令数 == 重放消费数,
//!   零 leftover。
//! - 单向事实源:迁移器只**读**旧 journal 行 + 只经 `apply_damage` 公共
//!   口写新管线;源 `destruction/runtime.rs` 0-byte 改动。

use std::collections::BTreeMap;

use rurix_pkg::sha256::{digest, hex};

use crate::capture::canonical::CaptureError;
use crate::destruction::{DamageCommand, DestructionCookedArtifact, FracturePipeline};

use super::PhysicsParticleRef;
use super::destruction_adapter::DestructionChunkAdapter;

/// 迁移后的 consumer 命令(dest_ref = 距 damage 点最近 chunk 的 particle
/// ref;域语义确定——同距按 chunk_id 字典序取小)。
#[derive(Debug, Clone)]
pub struct ParticleViewCommand {
    /// 原 damage 命令(逐字保留,无损)。
    pub damage: DamageCommand,
    /// 解析的目标 particle ref(DestructionChunk 域)。
    pub dest_ref: PhysicsParticleRef,
    /// 解析时距离(诊断面)。
    pub resolved_dist: f32,
}

/// 迁移报告(门脚本 JSON 面)。
#[derive(Debug, Clone)]
pub struct MigrationReport {
    /// 迁移命令序列(tick 升序)。
    pub commands: Vec<ParticleViewCommand>,
    /// 旧路径逐 tick state_hash(tick → hash)。
    pub legacy_hashes: BTreeMap<u64, String>,
    /// 新路径(迁移 consumer 重放)逐 tick state_hash。
    pub migrated_hashes: BTreeMap<u64, String>,
    /// 迁移前后逐 tick digest 一致。
    pub digest_equal: bool,
    /// journal 全消费(damage 行数 == 迁移命令数 == 重放消费数)。
    pub journal_fully_consumed: bool,
    /// 迁移命令 digest(canonical 序列 sha256;golden 锚)。
    pub migration_digest: String,
    /// 旧 journal 行数(damage:*)。
    pub damage_line_count: usize,
    /// 重放消费数。
    pub replayed_count: usize,
}

/// DamageCommand → ParticleViewCommand(确定域解析:最近 chunk,同距字典序)。
pub fn migrate_damage_command(
    cooked: &DestructionCookedArtifact,
    cmd: &DamageCommand,
) -> Result<ParticleViewCommand, CaptureError> {
    if cooked.chunks.is_empty() {
        return Err(CaptureError::Rejected(
            "migrate: cooked artifact has zero chunks".into(),
        ));
    }
    let mut best: Option<(&str, f32)> = None;
    for c in &cooked.chunks {
        let d = dist3(cmd.point, c.center);
        best = match best {
            None => Some((c.chunk_id.as_str(), d)),
            Some((id, bd)) => {
                if d < bd || (d == bd && c.chunk_id.as_str() < id) {
                    Some((c.chunk_id.as_str(), d))
                } else {
                    best
                }
            }
        };
    }
    let (chunk_id, dist) = best.expect("non-empty chunks");
    Ok(ParticleViewCommand {
        damage: cmd.clone(),
        dest_ref: DestructionChunkAdapter::ref_of_chunk(chunk_id),
        resolved_dist: dist,
    })
}

/// 全量迁移:逐命令解析 + canonical digest(damage 行序 = 记账序)。
pub fn migrate_journal(
    cooked: &DestructionCookedArtifact,
    cmds: &[DamageCommand],
) -> Result<Vec<ParticleViewCommand>, CaptureError> {
    cmds.iter()
        .map(|c| migrate_damage_command(cooked, c))
        .collect()
}

/// 迁移命令 canonical 序列 digest(golden 锚;行序 = 输入序)。
pub fn migration_digest(cmds: &[ParticleViewCommand]) -> String {
    let mut buf = String::new();
    for c in cmds {
        buf.push_str(&format!(
            "tick={}:ref={}:mag={:.6}:r={:.6}:dist={:.6}\n",
            c.damage.tick,
            c.dest_ref.canonical_text(),
            c.damage.magnitude,
            c.damage.radius,
            c.resolved_dist
        ));
    }
    hex(&digest(buf.as_bytes()))
}

/// 新路径重放:迁移命令逐 tick 推入全新管线;返回逐 tick state_hash。
///
/// 语义等价执行体 = `FracturePipeline::apply_damage`(迁移不改命令语义,
/// 只改寻址面);「迁移 consumer」在骨架期 = 命令经 ref 解析记账后原样
/// 重放,完整期 ref 直驱 strain 面。
pub fn replay_migrated(
    cooked: &DestructionCookedArtifact,
    cmds: &[ParticleViewCommand],
    tick_count: u64,
) -> Result<BTreeMap<u64, String>, CaptureError> {
    let mut pipe = FracturePipeline::new(cooked.clone());
    let mut hashes = BTreeMap::new();
    for tick in 0..tick_count {
        for c in cmds.iter().filter(|c| c.damage.tick == tick) {
            pipe.apply_damage(c.damage.clone())
                .map_err(|e| CaptureError::Rejected(format!("migrate replay: {e}")))?;
        }
        pipe.step(tick)
            .map_err(|e| CaptureError::Rejected(format!("migrate step: {e}")))?;
        hashes.insert(tick, pipe.state_hash());
    }
    Ok(hashes)
}

/// 端到端迁移门:旧路径直推 vs 迁移 consumer 重放,逐 tick digest 一致 +
/// journal 全消费无损。
///
/// `damage_line_count` 由调用方从旧管线 `journal_lines()` 统计
/// (`damage:` 前缀行);骨架期门脚本以 `apply_damage` 输入数对拍。
pub fn run_migration_gate(
    cooked: &DestructionCookedArtifact,
    cmds: &[DamageCommand],
    tick_count: u64,
) -> Result<MigrationReport, CaptureError> {
    // ① 旧路径:直推管线。
    let mut legacy = FracturePipeline::new(cooked.clone());
    let mut legacy_hashes = BTreeMap::new();
    for tick in 0..tick_count {
        for c in cmds.iter().filter(|c| c.tick == tick) {
            legacy
                .apply_damage(c.clone())
                .map_err(|e| CaptureError::Rejected(format!("legacy: {e}")))?;
        }
        legacy
            .step(tick)
            .map_err(|e| CaptureError::Rejected(format!("legacy step: {e}")))?;
        legacy_hashes.insert(tick, legacy.state_hash());
    }
    let damage_line_count = legacy
        .journal_lines()
        .iter()
        .filter(|l| l.starts_with("damage:"))
        .count();

    // ② 迁移:damage journal → particle_view consumer 命令。
    let migrated_cmds = migrate_journal(cooked, cmds)?;
    let digest = migration_digest(&migrated_cmds);

    // ③ 新路径:迁移命令重放到全新管线。
    let migrated_hashes = replay_migrated(cooked, &migrated_cmds, tick_count)?;

    let digest_equal = legacy_hashes == migrated_hashes;
    let journal_fully_consumed =
        damage_line_count == cmds.len() && migrated_cmds.len() == cmds.len();

    Ok(MigrationReport {
        commands: migrated_cmds,
        legacy_hashes,
        migrated_hashes,
        digest_equal,
        journal_fully_consumed,
        migration_digest: digest,
        damage_line_count,
        replayed_count: cmds.len(),
    })
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}
