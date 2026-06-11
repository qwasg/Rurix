//! 诊断文本渲染(07 §5;UI golden 通道的输出形态,M1 契约 D-M1-4)。
//!
//! 设计约束:**确定性输出**——同输入字节级同输出(snapshot 比对前提);
//! 路径规范化由调用方负责(UI harness 以 `$DIR/...` 形态注册文件名,
//! 渲染器原样使用 [`SourceMap`] 中的文件名,不做环境相关变换)。
//!
//! 形态对齐 rustc 风格(annotate-snippets 的完整接入随后续里程碑演进,
//! 本模块是其确定性 MVP):
//!
//! ```text
//! error[RX0008]: expected `;`, found `let`
//!   --> $DIR/missing_semi.rx:3:5
//!    |
//!  3 |     let b = 2;
//!    |     ^^^ expected `;`
//!    |
//!    = help: ...
//! ```

use std::fmt::Write as _;

use crate::diag::{DiagData, Level};
use crate::messages::MessageTable;
use crate::source_map::SourceMap;
use crate::span::Span;

/// 渲染全部诊断为 snapshot 文本(诊断间以空行分隔,末尾单换行;无诊断返回空串)。
pub fn render_diagnostics(diags: &[DiagData], sm: &SourceMap, table: &MessageTable) -> String {
    let mut out = String::new();
    for (i, d) in diags.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        render_one(&mut out, d, sm, table);
    }
    out
}

fn render_one(out: &mut String, d: &DiagData, sm: &SourceMap, table: &MessageTable) {
    let level = match d.level {
        Level::Error => "error",
        Level::Warning => "warning",
    };
    match d.code {
        Some(code) => {
            let _ = writeln!(out, "{level}[{code}]: {}", d.message(table));
        }
        None => {
            let _ = writeln!(out, "{level}: {}", d.message(table));
        }
    }

    // 行号 gutter 宽度:取全部 label 行号的最大十进制宽度
    let gutter = d
        .labels
        .iter()
        .map(|l| {
            let lc = sm.lookup(l.span.file, l.span.lo);
            lc.line.to_string().len()
        })
        .max()
        .unwrap_or(1);

    for (i, label) in d.labels.iter().enumerate() {
        render_label(out, label.span, &label.message, sm, gutter, i == 0);
    }

    let pad = " ".repeat(gutter);
    for note in &d.notes {
        let _ = writeln!(out, "{pad} = note: {note}");
    }
    for help in &d.helps {
        let _ = writeln!(out, "{pad} = help: {help}");
    }
    for sug in &d.suggestions {
        let _ = writeln!(out, "{pad} = help: {}: `{}`", sug.message, sug.replacement);
    }
}

fn render_label(
    out: &mut String,
    span: Span,
    message: &str,
    sm: &SourceMap,
    gutter: usize,
    primary: bool,
) {
    let file = sm.file(span.file);
    let lc = sm.lookup(span.file, span.lo);
    let pad = " ".repeat(gutter);
    if primary {
        let _ = writeln!(out, "{pad}--> {}:{}:{}", file.name, lc.line, lc.col);
    } else {
        let _ = writeln!(out, "{pad}::: {}:{}:{}", file.name, lc.line, lc.col);
    }
    let _ = writeln!(out, "{pad} |");
    let line_text = file.line_text(lc.line);
    let _ = writeln!(out, "{:>gutter$} | {line_text}", lc.line);

    // caret 宽度:span 在本行内覆盖的字符数(跨行截断到行尾;至少 1)
    let line_start_col = lc.col as usize - 1;
    let span_text = sm.snippet(span);
    let first_line = span_text.split('\n').next().unwrap_or("");
    let width = first_line.chars().count().max(1);
    let carets = "^".repeat(width);
    let _ = writeln!(
        out,
        "{pad} | {}{carets} {message}",
        " ".repeat(line_start_col)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::span::{Edition, SourceId};

    fn render_src(name: &str, src: &str) -> String {
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let id = sm.add_file(name, src, Edition::Rx0);
        assert_eq!(id, SourceId(0));
        let tokens = lex(src, id, Edition::Rx0, &diag);
        let _ = parse(src, tokens, id, Edition::Rx0, &diag);
        render_diagnostics(&diag.emitted(), &sm, diag.messages())
    }

    #[test]
    fn clean_source_renders_empty() {
        assert_eq!(render_src("$DIR/ok.rx", "fn f() {}"), "");
    }

    #[test]
    fn parse_error_renders_header_location_and_caret() {
        let out = render_src("$DIR/bad.rx", "fn f() {\n    let a = 1 let b = 2;\n}");
        assert!(out.contains("error[RX0008]:"), "{out}");
        assert!(out.contains("--> $DIR/bad.rx:2:15"), "{out}");
        assert!(out.contains("let a = 1 let b = 2;"), "{out}");
        assert!(out.contains("^^^"), "{out}");
        assert!(out.ends_with('\n'), "{out:?}");
    }

    #[test]
    fn rendering_is_deterministic() {
        let src = "fn f( {}\nstruct 1\n";
        let a = render_src("$DIR/x.rx", src);
        let b = render_src("$DIR/x.rx", src);
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn multiple_diags_separated_by_blank_line() {
        let out = render_src("$DIR/multi.rx", "fn a( {}\nfn b() { let x = 1 < 2 < 3; }\n");
        let blocks: Vec<&str> = out.split("\n\n").collect();
        assert!(blocks.len() >= 2, "{out}");
    }

    #[test]
    fn help_lines_render() {
        // gated 闭包诊断携带 help(feature_gate)
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let src = "fn f() { let g = || 0; }";
        let id = sm.add_file("$DIR/gate.rx", src, Edition::Rx0);
        let tokens = lex(src, id, Edition::Rx0, &diag);
        let ast = parse(src, tokens, id, Edition::Rx0, &diag);
        crate::feature_gate::check_feature_gates(&ast, &diag);
        let out = render_diagnostics(&diag.emitted(), &sm, diag.messages());
        assert!(out.contains("error[RX0010]:"), "{out}");
        assert!(out.contains("= help: add `#![feature(closures)]`"), "{out}");
    }
}
