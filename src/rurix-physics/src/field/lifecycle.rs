//! 三生命周期(RFC-0024 §4.B2 冻结表;骨架期语义面)。
//!
//! | 生命周期 | 语义 | 确定性规则 |
//! |---|---|---|
//! | Transient | 单 tick 内求值即弃 | **不进 journal**;结果经命令规范化进 journal |
//! | Construction | cook/关卡构建期烘焙 | **进 cooked artifact digest** |
//! | Persistent | 跨 tick 存活 | **必须可显式注销**;注册/注销/参数变更全部写 command journal,参与 `semantic_state_hash`,replay 逐 tick hash 一致为硬门 |

use super::def::FieldError;

/// 场生命周期(冻结三态;canonical 名进 schema/journal 面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldLifecycle {
    /// 单 tick 求值即弃(不进 journal)。
    Transient,
    /// cook 期烘焙(进 cooked digest;运行时不注册)。
    Construction,
    /// 跨 tick 存活(注册/注销/变更全 journal 化;可显式注销)。
    Persistent,
}

impl FieldLifecycle {
    /// canonical 名(schema/journal 唯一字面)。
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Transient => "Transient",
            Self::Construction => "Construction",
            Self::Persistent => "Persistent",
        }
    }

    /// 自 canonical 名还原(未知名 fail-closed)。
    pub fn parse(s: &str) -> Result<Self, FieldError> {
        Ok(match s {
            "Transient" => Self::Transient,
            "Construction" => Self::Construction,
            "Persistent" => Self::Persistent,
            other => return Err(FieldError::InvalidDef(format!("unknown lifecycle {other}"))),
        })
    }

    /// 是否进 command journal(冻结表字面的机核面)。
    pub fn enters_journal(self) -> bool {
        matches!(self, Self::Persistent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_membership_matches_frozen_table() {
        assert!(!FieldLifecycle::Transient.enters_journal());
        assert!(!FieldLifecycle::Construction.enters_journal());
        assert!(FieldLifecycle::Persistent.enters_journal());
        for name in ["Transient", "Construction", "Persistent"] {
            assert!(FieldLifecycle::parse(name).is_ok());
        }
        assert!(FieldLifecycle::parse("Ephemeral").is_err());
    }
}
