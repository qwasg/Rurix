//! 源位置基础类型(07 §5 第 1 条:基础设施先于 lexer)。
//!
//! `Span` 携带 edition(D-404:span 层第一天预埋,未来 edition 迁移不需重构)。

/// 源文件在 [`crate::source_map::SourceMap`] 中的句柄。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SourceId(pub u32);

/// 文件内字节偏移(u32:单文件 4GiB 上限,与 rustc 同量级取舍)。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BytePos(pub u32);

/// 语言 edition(D-404 / 10 §5)。
///
/// MVP 期唯一变体;1.0 后引入年度 edition 窗口承载破坏性迁移。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Edition {
    /// 0.x(MVP)期的唯一 edition。
    #[default]
    Rx0,
}

/// 半开区间 `[lo, hi)` 的源码位置。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    pub file: SourceId,
    pub lo: BytePos,
    pub hi: BytePos,
    pub edition: Edition,
}

impl Span {
    pub fn new(file: SourceId, lo: u32, hi: u32, edition: Edition) -> Self {
        debug_assert!(lo <= hi, "Span lo > hi: {lo} > {hi}");
        Self {
            file,
            lo: BytePos(lo),
            hi: BytePos(hi),
            edition,
        }
    }

    pub fn len(&self) -> u32 {
        self.hi.0 - self.lo.0
    }

    pub fn is_empty(&self) -> bool {
        self.lo == self.hi
    }

    /// 合并两个同文件 span 为最小覆盖区间(诊断 label 常用)。
    pub fn to(self, other: Span) -> Span {
        debug_assert_eq!(self.file, other.file, "跨文件 Span 合并");
        Span {
            file: self.file,
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            edition: self.edition,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_len_and_empty() {
        let f = SourceId(0);
        let s = Span::new(f, 3, 7, Edition::Rx0);
        assert_eq!(s.len(), 4);
        assert!(!s.is_empty());
        assert!(Span::new(f, 5, 5, Edition::Rx0).is_empty());
    }

    #[test]
    fn span_to_merges_min_cover() {
        let f = SourceId(1);
        let a = Span::new(f, 10, 14, Edition::Rx0);
        let b = Span::new(f, 2, 6, Edition::Rx0);
        let m = a.to(b);
        assert_eq!((m.lo.0, m.hi.0), (2, 14));
        assert_eq!(m.file, f);
    }

    #[test]
    fn edition_default_is_rx0() {
        assert_eq!(Edition::default(), Edition::Rx0);
    }
}
