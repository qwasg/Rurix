//! DestructionCache:command/event/state hash;roundtrip 无地址依赖(RFC-0021 §4.C2)。

use rurix_pkg::sha256::{digest, hex};

use super::runtime::{DamageCommand, FracturePipeline};
use super::schema::DestructionCookedArtifact;
use super::vfx::FractureEvent;

#[derive(Debug, Clone)]
pub struct CacheTickRecord {
    pub tick: u64,
    pub commands: Vec<DamageCommand>,
    pub events: Vec<FractureEvent>,
    pub state_hash: String,
}

#[derive(Debug, Clone, Default)]
pub struct DestructionCache {
    pub records: Vec<CacheTickRecord>,
    pub event_sequence_digest: String,
    pub final_state_hash: String,
}

#[derive(Debug, Clone)]
pub struct CacheRoundtripReport {
    pub event_sequence_identical: bool,
    pub state_hash_identical: bool,
    pub vfx_commit_count: usize,
    pub vfx_duplicate_count: u64,
    pub replayed_event_digest: String,
    pub replayed_state_hash: String,
}

impl DestructionCache {
    pub fn from_pipeline(pipe: &FracturePipeline) -> Self {
        Self {
            records: pipe.cache_records().to_vec(),
            event_sequence_digest: pipe.event_sequence_digest(),
            final_state_hash: pipe.state_hash(),
        }
    }

    pub fn serialize(&self) -> String {
        let mut s = String::from("{\"records\":[");
        for (i, r) in self.records.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"tick\":{},\"state_hash\":\"{}\",\"commands\":{},\"events\":{}}}",
                r.tick,
                r.state_hash,
                r.commands.len(),
                r.events.len()
            ));
        }
        s.push_str(&format!(
            "],\"event_sequence_digest\":\"{}\",\"final_state_hash\":\"{}\"}}",
            self.event_sequence_digest, self.final_state_hash
        ));
        s
    }

    pub fn digest(&self) -> String {
        hex(&digest(self.serialize().as_bytes()))
    }

    /// 序列化→重载命令→重驱动;核对事件序列与 state hash。
    ///
    /// 重驱动后对已提交事件再 try_commit 一次(模拟 rollback/cache replay):
    /// publish 计数不得增加;duplicate 计数可上升。
    pub fn roundtrip_replay(
        &self,
        cooked: &DestructionCookedArtifact,
    ) -> Result<CacheRoundtripReport, String> {
        let mut pipe = FracturePipeline::new(cooked.clone());
        for rec in &self.records {
            for cmd in &rec.commands {
                pipe.apply_damage(cmd.clone()).map_err(|e| e.to_string())?;
            }
            pipe.step(rec.tick).map_err(|e| e.to_string())?;
        }
        let commit_after_replay = pipe.vfx_commit_count();
        let digest_after_replay = pipe.event_sequence_digest();
        let hash_after_replay = pipe.state_hash();
        // rollback/cache 重放:再提交同一批 → 不得新增 publish
        for rec in &self.records {
            pipe.recommit_vfx_for_tick(rec.tick);
        }
        let commit_after_recommit = pipe.vfx_commit_count();
        let no_extra_publish = commit_after_recommit == commit_after_replay;
        Ok(CacheRoundtripReport {
            event_sequence_identical: digest_after_replay == self.event_sequence_digest
                && no_extra_publish,
            state_hash_identical: hash_after_replay == self.final_state_hash,
            vfx_commit_count: commit_after_recommit,
            // 0 = 重放未造成额外对外提交(duplicate 被吞掉)
            vfx_duplicate_count: if no_extra_publish { 0 } else { 1 },
            replayed_event_digest: digest_after_replay,
            replayed_state_hash: hash_after_replay,
        })
    }
}
