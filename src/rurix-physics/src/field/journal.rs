//! Persistent 场 command journal(RFC-0024 §4.B2 硬门:注册/注销/参数变更
//! 全部写 journal,参与 `semantic_state_hash`,replay 逐 tick hash 一致)。
//!
//! 骨架期面:命令变体 + canonical 文本行 + 逐 tick hash 链 +
//! replay 全消费(fail-closed leftover/missing)。完整期:场求值结果经
//! 命令规范化并入 M66 capture 主流(骨架期独立链,不反向改写 M66 格式)。

use rurix_pkg::sha256::{digest, hex};

use super::def::{FieldDef, FieldError};

/// Persistent 场命令(注册/注销/变更三态;Transient/Construction 不产生
/// 命令——冻结表「不进 journal / 进 cooked digest」的类型面执行)。
#[derive(Debug, Clone, PartialEq)]
pub enum FieldJournalCommand {
    /// 注册 persistent 场(载荷 = 完整定义 canonical digest + 定义体;
    /// digest 前置 = replay 校验锚)。
    Register {
        /// 场稳定 ID。
        field_id: String,
        /// 完整定义(canonical 序列化面 = replay 重建源)。
        def: Box<FieldDef>,
    },
    /// 显式注销(Persistent 必须可显式注销,冻结表字面)。
    Unregister {
        /// 场稳定 ID。
        field_id: String,
    },
    /// 参数变更(载荷 = 变更后完整定义;字段级 diff 归完整期)。
    Update {
        /// 场稳定 ID。
        field_id: String,
        /// 变更后完整定义。
        def: Box<FieldDef>,
    },
}

impl FieldJournalCommand {
    /// canonical 文本行(journal 行唯一字面;digest 前像)。
    pub fn canonical_text(&self) -> String {
        match self {
            Self::Register { field_id, def } => format!(
                "register:{}:def_digest={}:def={}",
                field_id,
                def.digest(),
                def.canonical_json()
            ),
            Self::Unregister { field_id } => format!("unregister:{field_id}"),
            Self::Update { field_id, def } => format!(
                "update:{}:def_digest={}:def={}",
                field_id,
                def.digest(),
                def.canonical_json()
            ),
        }
    }
}

/// 逐 tick journal 行(命令集 + tick 末 semantic hash)。
#[derive(Debug, Clone, PartialEq)]
pub struct FieldJournalTick {
    /// tick 序。
    pub tick: u64,
    /// 本 tick 命令(序 = 记账序)。
    pub commands: Vec<FieldJournalCommand>,
    /// tick 末场注册表 semantic hash(参与 semantic_state_hash 的骨架面)。
    pub semantic_hash: String,
}

/// Persistent 场 journal(逐 tick 行 + replay 校验面)。
#[derive(Debug, Clone, Default)]
pub struct FieldJournal {
    /// 逐 tick 行(序 = tick 升序)。
    pub ticks: Vec<FieldJournalTick>,
}

impl FieldJournal {
    /// 空 journal。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加 tick 行。
    pub fn push_tick(&mut self, tick: FieldJournalTick) {
        self.ticks.push(tick);
    }

    /// 全 journal canonical digest(逐行连接后 sha256;golden 锚)。
    pub fn digest(&self) -> String {
        let mut buf = String::new();
        for t in &self.ticks {
            buf.push_str(&format!("tick:{}:\n", t.tick));
            for c in &t.commands {
                buf.push_str(&c.canonical_text());
                buf.push('\n');
            }
            buf.push_str(&format!("hash:{}\n", t.semantic_hash));
        }
        hex(&digest(buf.as_bytes()))
    }

    /// 命令总数(replay 全消费断言面)。
    pub fn command_count(&self) -> usize {
        self.ticks.iter().map(|t| t.commands.len()).sum()
    }
}

/// replay 全消费执行体(骨架期):逐 tick 重放命令到全新注册表,逐 tick
/// hash 与 journal 记录一致;leftover 命令/tick 缺失 fail-closed。
///
/// 返回逐 tick hash 序列(调用方对拍 golden)。
pub fn replay_journal(journal: &FieldJournal) -> Result<Vec<(u64, String)>, FieldError> {
    let mut registry = super::registry::FieldRegistry::new();
    let mut out = Vec::new();
    for t in &journal.ticks {
        for c in &t.commands {
            match c {
                FieldJournalCommand::Register { def, .. } => registry.register((**def).clone())?,
                FieldJournalCommand::Unregister { field_id } => {
                    registry.unregister(field_id)?;
                }
                FieldJournalCommand::Update { def, .. } => registry.update((**def).clone())?,
            }
        }
        let h = registry.semantic_hash();
        if h != t.semantic_hash {
            return Err(FieldError::InvalidDef(format!(
                "replay tick {} hash mismatch: expected {} got {}",
                t.tick, t.semantic_hash, h
            )));
        }
        out.push((t.tick, h));
    }
    Ok(out)
}
