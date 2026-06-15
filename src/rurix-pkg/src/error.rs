//! 包管理类型化错误(RXS-0089~0094;rx CLI 映射 7xxx 段位错误码)。
//!
//! 7xxx 链接/工具链段位续接(RX7001~RX7004 之后,分配制递增、含义冻结,
//! registry/error_codes.json 唯一事实源)。本 crate 保持纯净不发诊断,
//! 返回 [`PkgError`];rx CLI 取 [`PkgError::code`] 与 [`Display`] 落 stderr。

use std::fmt;

/// 包管理诊断错误(每变体对应一个冻结的 7xxx 错误码,RXS-0089~0094)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PkgError {
    /// RX7005 — rurix.toml 清单解析/校验错误(缺字段 / 非法值 / build 非声明式
    /// / TOML 子集违例 / 清单不可读)。
    ManifestInvalid(String),
    /// RX7006 — 依赖解析冲突(来源/pin 不相容 / feature 引用不存在)。
    ResolutionConflict(String),
    /// RX7007 — rurix.lock 不一致(--locked 下重解析图 ≠ 入库 lock)。
    LockMismatch(String),
    /// RX7008 — 内容树 digest 不符(vendor 内容 SHA-256 ≠ lock 记录)。
    DigestMismatch(String),
    /// RX7009 — 依赖来源不可达(--offline 需网无缓存 / path 目标缺失)。
    SourceUnreachable(String),
}

impl PkgError {
    /// 冻结的 7xxx 错误码(registry/error_codes.json,10 §6)。
    pub fn code(&self) -> &'static str {
        match self {
            PkgError::ManifestInvalid(_) => "RX7005",
            PkgError::ResolutionConflict(_) => "RX7006",
            PkgError::LockMismatch(_) => "RX7007",
            PkgError::DigestMismatch(_) => "RX7008",
            PkgError::SourceUnreachable(_) => "RX7009",
        }
    }

    /// 诊断细节(措辞允许保守粗糙,07 §4 先正确性后诊断)。
    pub fn detail(&self) -> &str {
        match self {
            PkgError::ManifestInvalid(d)
            | PkgError::ResolutionConflict(d)
            | PkgError::LockMismatch(d)
            | PkgError::DigestMismatch(d)
            | PkgError::SourceUnreachable(d) => d,
        }
    }
}

impl fmt::Display for PkgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error[{}]: {}", self.code(), self.detail())
    }
}

impl std::error::Error for PkgError {}

/// 包管理操作统一结果别名。
pub type PkgResult<T> = Result<T, PkgError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_frozen_7xxx() {
        // 段位与编号冻结(10 §6;registry/error_codes.json 同步)。
        assert_eq!(PkgError::ManifestInvalid(String::new()).code(), "RX7005");
        assert_eq!(PkgError::ResolutionConflict(String::new()).code(), "RX7006");
        assert_eq!(PkgError::LockMismatch(String::new()).code(), "RX7007");
        assert_eq!(PkgError::DigestMismatch(String::new()).code(), "RX7008");
        assert_eq!(PkgError::SourceUnreachable(String::new()).code(), "RX7009");
    }

    #[test]
    fn display_carries_code_and_detail() {
        let e = PkgError::ManifestInvalid("missing name".to_owned());
        assert_eq!(e.to_string(), "error[RX7005]: missing name");
    }
}
