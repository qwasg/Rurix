//! rx fmt 雏形(契约 D-M1-5 / G-M1-5;完整工具化 → RD-005,M6)。
//!
//! 形态:**token 流重排印,保留注释**——
//! - 词法干净的源文本才可 fmt(有词法诊断即拒绝);
//! - 保留原换行结构(同行 token 仍同行;≥1 个空行归一为恰好 1 个空行);
//! - 缩进 = 当前未闭合定界符深度 × 4 空格(行首为闭定界符时少一层);
//! - token 对水平间距按规则表归一;**歧义 token**(`<` `>` `>>` `|` `||`,
//!   泛型/比较与闭包/位或在 token 层不可判)保留作者原有间距(sticky 规则,
//!   幂等性由"保留"本身保证);
//! - 注释保留:独立行注释按当前缩进排印,行尾注释与 token 间留单空格,
//!   块注释内部逐字保留;
//! - 行尾空白清除,文件尾恰好一个换行;CRLF 归一为 LF。
//!
//! 幂等性(G-M1-5):输出落在上述规则的不动点上——换行结构/sticky 间距在
//! 输出中被原样保留,缩进与非 sticky 间距为输入无关的确定函数。

use crate::diag::DiagCtxt;
use crate::lexer::{Keyword, Token, TokenKind as Tk, lex};
use crate::span::{Edition, SourceId};

#[derive(Debug, PartialEq, Eq)]
pub enum FmtError {
    /// 源文本有词法错误(诊断条数);fmt 拒绝处理。
    LexErrors(usize),
}

impl std::fmt::Display for FmtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FmtError::LexErrors(n) => write!(f, "源文本含 {n} 条词法诊断,拒绝格式化"),
        }
    }
}

/// 格式化入口:返回格式化文本(LF 行尾,尾部单换行)。
pub fn format_source(src: &str) -> Result<String, FmtError> {
    let src = src.replace("\r\n", "\n");
    let diag = DiagCtxt::new();
    let tokens = lex(&src, SourceId(0), Edition::Rx0, &diag);
    let n_diags = diag.emitted().len();
    if n_diags > 0 {
        return Err(FmtError::LexErrors(n_diags));
    }
    let elems = collect_elements(&src, &tokens);
    Ok(emit(&src, &elems))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ElemKind {
    Token(Tk),
    LineComment,
    BlockComment,
}

#[derive(Clone, Copy, Debug)]
struct Elem {
    kind: ElemKind,
    /// 源文本字节区间。
    lo: usize,
    hi: usize,
    /// 与前一元素之间的原始换行数(0 = 同行)。
    lines_before: u32,
    /// 与前一元素之间原始是否有空白/注释间隔(sticky 规则数据源)。
    space_before: bool,
}

/// 扫描 token 间隙提取注释,与 token 合并为有序元素流。
fn collect_elements(src: &str, tokens: &[Token]) -> Vec<Elem> {
    let mut elems: Vec<Elem> = Vec::new();
    let mut cursor = 0usize;

    let scan_gap = |elems: &mut Vec<Elem>, from: usize, to: usize| {
        let gap = &src[from..to];
        let mut i = 0;
        let bytes = gap.as_bytes();
        // lines / seen_sep:自上一元素(token 或注释)以来的换行数 / 是否有任何间隔
        let mut lines = 0u32;
        let mut seen_sep = false;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => {
                    lines += 1;
                    seen_sep = true;
                    i += 1;
                }
                b' ' | b'\t' | b'\r' => {
                    seen_sep = true;
                    i += 1;
                }
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                    let start = from + i;
                    let end_rel = gap[i..].find('\n').map_or(gap.len(), |p| i + p);
                    let mut end = from + end_rel;
                    // 剥除行注释尾部 \r(防 CRLF 残留)
                    while end > start && src.as_bytes()[end - 1] == b'\r' {
                        end -= 1;
                    }
                    elems.push(Elem {
                        kind: ElemKind::LineComment,
                        lo: start,
                        hi: end,
                        lines_before: lines,
                        space_before: seen_sep,
                    });
                    lines = 0;
                    seen_sep = true;
                    i = end_rel;
                }
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                    // 嵌套块注释(RXS-0003;词法已验证配平),内部逐字保留
                    let start = from + i;
                    let mut depth = 1u32;
                    let mut j = i + 2;
                    while j < bytes.len() && depth > 0 {
                        if bytes[j] == b'/' && j + 1 < bytes.len() && bytes[j + 1] == b'*' {
                            depth += 1;
                            j += 2;
                        } else if bytes[j] == b'*' && j + 1 < bytes.len() && bytes[j + 1] == b'/' {
                            depth -= 1;
                            j += 2;
                        } else {
                            j += 1;
                        }
                    }
                    elems.push(Elem {
                        kind: ElemKind::BlockComment,
                        lo: start,
                        hi: from + j,
                        lines_before: lines,
                        space_before: seen_sep,
                    });
                    lines = 0;
                    seen_sep = true;
                    i = j;
                }
                _ => {
                    // 词法干净前提下不可达;防御性跳过
                    seen_sep = true;
                    i += 1;
                }
            }
        }
        (lines, seen_sep)
    };

    for tok in tokens {
        if tok.kind == Tk::Eof {
            // EOF 前的尾随注释
            scan_gap(&mut elems, cursor, src.len());
            break;
        }
        let lo = tok.span.lo.0 as usize;
        let hi = tok.span.hi.0 as usize;
        let (lines, spaced) = scan_gap(&mut elems, cursor, lo);
        elems.push(Elem {
            kind: ElemKind::Token(tok.kind),
            lo,
            hi,
            lines_before: lines,
            space_before: spaced,
        });
        cursor = hi;
    }
    elems
}

/// 可以终结一个表达式的 token(一元/二元歧义判定数据源)。
fn ends_expr(kind: Tk) -> bool {
    matches!(
        kind,
        Tk::Ident
            | Tk::Underscore
            | Tk::Lifetime
            | Tk::IntLit { .. }
            | Tk::FloatLit { .. }
            | Tk::StrLit
            | Tk::CharLit
            | Tk::Kw(Keyword::True)
            | Tk::Kw(Keyword::False)
            | Tk::CloseParen
            | Tk::CloseBracket
            | Tk::CloseBrace
            | Tk::Question
    )
}

/// 歧义 token:泛型尖括号 / 闭包竖线在 token 层不可判,保留作者间距。
fn is_sticky(kind: Tk) -> bool {
    matches!(kind, Tk::Lt | Tk::Gt | Tk::Shr | Tk::Or | Tk::OrOr)
}

/// 一元前缀候选:`-` `!` `*` `&` `&&`。
fn is_prefix_candidate(kind: Tk) -> bool {
    matches!(kind, Tk::Minus | Tk::Not | Tk::Star | Tk::And | Tk::AndAnd)
}

/// 同行相邻 token 的间距规则(prev_prefix = prev 已被判为一元前缀)。
fn space_between(prev: Tk, prev_prefix: bool, next: Tk, orig_space: bool) -> bool {
    use Tk::*;
    if is_sticky(prev) || is_sticky(next) {
        return orig_space;
    }
    // 区间运算符两侧紧贴(Rust 风格 `0..n`)
    if matches!(next, DotDot | DotDotEq) || matches!(prev, DotDot | DotDotEq) {
        return false;
    }
    // next 侧禁止空格
    if matches!(
        next,
        Comma | Semi | Question | Dot | PathSep | CloseParen | CloseBracket | Colon
    ) {
        return false;
    }
    // prev 侧禁止空格
    if matches!(prev, Dot | PathSep | OpenParen | OpenBracket | Pound) {
        return false;
    }
    if prev_prefix {
        return false;
    }
    match next {
        // 调用/声明/fn 指针紧贴;表达式上下文的 `(` 留空格
        OpenParen => !(ends_expr(prev) || prev == Kw(Keyword::Fn)),
        // 索引紧贴;数组字面量/属性体留空格(`#[` 已被 prev=Pound 规则覆盖)
        OpenBracket => !ends_expr(prev),
        // `{}` 紧贴;`{ x }` 内侧留空
        CloseBrace => prev != OpenBrace,
        _ => true,
    }
}

fn emit(src: &str, elems: &[Elem]) -> String {
    let mut out = String::new();
    let mut depth: u32 = 0;
    let mut prev: Option<Elem> = None;
    // 注释不改变表达式上下文:单独记录最近的 token 及其前缀判定
    let mut last_tok: Option<Tk> = None;
    let mut last_tok_prefix = false;

    for &elem in elems {
        let text = &src[elem.lo..elem.hi];
        let is_close = matches!(
            elem.kind,
            ElemKind::Token(Tk::CloseBrace | Tk::CloseParen | Tk::CloseBracket)
        );
        if is_close {
            depth = depth.saturating_sub(1);
        }

        match prev {
            None => {} // 文件首元素,无前导
            Some(p) if elem.lines_before == 0 && p.kind != ElemKind::LineComment => {
                let need_space = match (p.kind, elem.kind) {
                    (_, ElemKind::LineComment | ElemKind::BlockComment) => true,
                    (ElemKind::BlockComment, _) => true,
                    (ElemKind::Token(pt), ElemKind::Token(nt)) => {
                        space_between(pt, last_tok_prefix, nt, elem.space_before)
                    }
                    (ElemKind::LineComment, _) => unreachable!("行注释后必换行"),
                };
                if need_space {
                    out.push(' ');
                }
            }
            Some(_) => {
                out.push('\n');
                if elem.lines_before >= 2 {
                    out.push('\n'); // ≥1 个空行归一为恰好 1 个
                }
                out.push_str(&"    ".repeat(depth as usize));
            }
        }

        out.push_str(text);

        if let ElemKind::Token(kind) = elem.kind {
            if matches!(kind, Tk::OpenBrace | Tk::OpenParen | Tk::OpenBracket) {
                depth += 1;
            }
            // 一元前缀判定:前一 token 不能终结表达式 → 本 token 是前缀
            last_tok_prefix = is_prefix_candidate(kind) && !last_tok.is_some_and(ends_expr);
            last_tok = Some(kind);
        }
        prev = Some(elem);
    }
    // 文件尾恰好一个换行(空文件输出空串)
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str) -> String {
        format_source(src).expect("fmt 失败")
    }

    fn assert_idempotent(src: &str) -> String {
        let once = fmt(src);
        let twice = fmt(&once);
        assert_eq!(
            once, twice,
            "fmt 不幂等\n--- once ---\n{once}\n--- twice ---\n{twice}"
        );
        once
    }

    #[test]
    fn normalizes_spacing_basics() {
        assert_eq!(
            fmt("fn  add( a:i32,b :i32 )->i32{ a+b }"),
            "fn add(a: i32, b: i32) -> i32 { a + b }\n"
        );
    }

    #[test]
    fn indentation_follows_depth() {
        let out = fmt("fn f() {\nlet a = 1;\nif a > 0 {\nreturn;\n}\n}");
        assert_eq!(
            out,
            "fn f() {\n    let a = 1;\n    if a > 0 {\n        return;\n    }\n}\n"
        );
    }

    #[test]
    fn preserves_comments_and_blank_lines() {
        let out = fmt(
            "// head\nfn f() {\n    let a = 1; // trailing\n\n\n    /* block */ let b = 2;\n}\n",
        );
        assert_eq!(
            out,
            "// head\nfn f() {\n    let a = 1; // trailing\n\n    /* block */ let b = 2;\n}\n"
        );
    }

    #[test]
    fn sticky_generics_and_closures_keep_author_spacing() {
        // 泛型尖括号:作者紧贴保持紧贴;比较运算的空格保持空格
        assert_eq!(
            fmt("fn f(v: Vec<Vec<f32>>) {}"),
            "fn f(v: Vec<Vec<f32>>) {}\n"
        );
        assert_eq!(
            fmt("fn f() { let c = a < b; }"),
            "fn f() { let c = a < b; }\n"
        );
        // 闭包竖线紧贴保持
        let out = assert_idempotent("fn f() { let g = |x: i32| x; }");
        assert_eq!(out, "fn f() { let g = |x: i32| x; }\n");
    }

    #[test]
    fn unary_vs_binary_disambiguation() {
        assert_eq!(
            fmt(
                "fn f() { let a = - 1; let b = a- 1; let c = & buf; let d = a&b; *out = * v * 2.0; }"
            ),
            "fn f() { let a = -1; let b = a - 1; let c = &buf; let d = a & b; *out = *v * 2.0; }\n"
        );
        assert_eq!(
            fmt("fn f(p: * mut T, r: &mut T) {}"),
            "fn f(p: *mut T, r: &mut T) {}\n"
        );
    }

    #[test]
    fn ranges_paths_attrs_tight() {
        assert_eq!(
            fmt("fn f() { for i in 0 .. n { } let p = std :: mem :: size_of; }"),
            "fn f() { for i in 0..n {} let p = std::mem::size_of; }\n"
        );
        assert_eq!(
            fmt("# [derive(Copy , Clone)]\nstruct P;"),
            "#[derive(Copy, Clone)]\nstruct P;\n"
        );
        assert_eq!(
            fmt("#![feature(closures)]\nfn f() { let g = || 0; }"),
            "#![feature(closures)]\nfn f() { let g = || 0; }\n"
        );
    }

    #[test]
    fn crlf_and_trailing_newline_normalized() {
        assert_eq!(
            fmt("fn f() {}\r\n\r\n\r\nfn g() {}\r\n"),
            "fn f() {}\n\nfn g() {}\n"
        );
        assert_eq!(fmt("fn f() {}"), "fn f() {}\n");
    }

    #[test]
    fn rejects_lex_errors() {
        assert!(matches!(
            format_source("fn f() { let s = \"unterminated; }"),
            Err(FmtError::LexErrors(_))
        ));
    }

    #[test]
    fn idempotent_on_representative_sources() {
        for src in [
            "kernel fn saxpy(grid: Grid<(1024,)>, a: f32, x: View<global, f32, (N,)>) {\n    let i = grid.thread_index();\n}\n",
            "enum Shape {\n    Circle { radius: f32 },\n    Point,\n}\n\nfn area(s: Shape) -> f32 {\n    match s {\n        Shape::Circle { radius } => 3.14 * radius * radius,\n        Shape::Point => 0.0,\n    }\n}\n",
            "fn ops() {\n    let mut f = 0;\n    f <<= 1;\n    f ^= 0b101;\n    let r = 0..=255;\n    let q = some()?;\n}\n",
            "/* leading /* nested */ block */\nfn f() {\n    // line\n    let s = \"multi\nline string\";\n}\n",
        ] {
            assert_idempotent(src);
        }
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(fmt(""), "");
        assert_eq!(fmt("\n\n"), "");
    }
}
