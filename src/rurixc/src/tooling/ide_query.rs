//! IDE 语义查询(completion / definition / references / highlight / rename,RXS-0100~0103)。

use std::collections::HashSet;

use crate::diag::{DiagCtxt, ErrorCode};
use crate::hir::{DefId, LocalId, Res};
use crate::query::QueryCtx;
use crate::resolve::Resolutions;
use crate::source_map::SourceMap;
use crate::span::{SourceId, Span};

/// 光标处符号目标(MVP 单文件)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolTarget {
    Def(DefId),
    Local { body: Span, id: LocalId },
}

#[derive(Clone, Debug)]
pub struct LspRange {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

#[derive(Clone, Debug)]
pub struct TextEdit {
    pub range: LspRange,
    pub new_text: String,
}

fn span_to_range(sm: &SourceMap, span: Span) -> LspRange {
    let (sl, sc) = sm.to_lsp_position(span);
    let end_lc = sm.lookup(span.file, span.hi);
    LspRange {
        start_line: sl,
        start_character: sc,
        end_line: end_lc.line - 1,
        end_character: end_lc.col - 1,
    }
}

fn offset_from_lsp(sm: &SourceMap, file: SourceId, line: u32, character: u32) -> u32 {
    sm.from_lsp_position(file, line, character).0
}

fn span_contains(span: Span, off: u32) -> bool {
    span.lo.0 <= off && off < span.hi.0
}

fn smallest_at(spans: impl Iterator<Item = Span>, off: u32) -> Option<Span> {
    spans
        .filter(|s| span_contains(*s, off))
        .min_by_key(|s| s.len())
}

fn body_for_span(res: &Resolutions, span: Span) -> Option<Span> {
    res.body_locals
        .keys()
        .copied()
        .filter(|body| body.file == span.file && body.lo.0 <= span.lo.0 && span.hi.0 <= body.hi.0)
        .min_by_key(|body| body.len())
}

/// 解析光标处符号。
pub fn symbol_at(
    res: &Resolutions,
    file: SourceId,
    sm: &SourceMap,
    line: u32,
    character: u32,
) -> Option<(SymbolTarget, Span)> {
    let off = offset_from_lsp(sm, file, line, character);
    if let Some(span) = smallest_at(res.path_res.keys().copied(), off) {
        return match res.path_res.get(&span)? {
            Res::Def(d) => Some((SymbolTarget::Def(*d), span)),
            Res::Local(l) => Some((
                SymbolTarget::Local {
                    body: body_for_span(res, span)?,
                    id: *l,
                },
                span,
            )),
            _ => None,
        };
    }
    if let Some(span) = smallest_at(res.bindings.keys().copied(), off) {
        let local = *res.bindings.get(&span)?;
        return Some((
            SymbolTarget::Local {
                body: body_for_span(res, span)?,
                id: local,
            },
            span,
        ));
    }
    if let Some(span) = smallest_at(res.item_defs.keys().copied(), off) {
        let def = *res.item_defs.get(&span)?;
        return Some((SymbolTarget::Def(def), span));
    }
    let _ = file;
    None
}

fn def_span(res: &Resolutions, def: DefId) -> Option<Span> {
    res.defs.get(def.0 as usize).map(|d| d.span)
}

fn spans_for_target(res: &Resolutions, target: &SymbolTarget) -> Vec<Span> {
    let mut spans = Vec::new();
    match target {
        SymbolTarget::Def(def) => {
            if let Some(s) = def_span(res, *def) {
                spans.push(s);
            }
            for (span, r) in &res.path_res {
                if matches!(r, Res::Def(d) if *d == *def) {
                    spans.push(*span);
                }
            }
            for (span, d) in &res.item_defs {
                if *d == *def {
                    spans.push(*span);
                }
            }
        }
        SymbolTarget::Local { body, id } => {
            for (span, l) in &res.bindings {
                if *l == *id && body_for_span(res, *span) == Some(*body) {
                    spans.push(*span);
                }
            }
            for (span, r) in &res.path_res {
                if matches!(r, Res::Local(l) if *l == *id)
                    && body_for_span(res, *span) == Some(*body)
                {
                    spans.push(*span);
                }
            }
        }
    }
    spans.sort_by_key(|s| (s.lo.0, s.hi.0));
    spans.dedup_by(|a, b| a.lo == b.lo && a.hi == b.hi);
    spans
}

pub fn definition_at(
    cx: &QueryCtx<'_>,
    sm: &SourceMap,
    file: SourceId,
    line: u32,
    character: u32,
) -> Option<LspRange> {
    let res = cx.resolutions();
    let (target, _) = symbol_at(&res, file, sm, line, character)?;
    let span = match &target {
        SymbolTarget::Def(d) => def_span(&res, *d)?,
        SymbolTarget::Local { body, id } => *res
            .bindings
            .iter()
            .find(|(span, local)| **local == *id && body_for_span(&res, **span) == Some(*body))
            .map(|(s, _)| s)?,
    };
    Some(span_to_range(sm, span))
}

pub fn references_at(
    cx: &QueryCtx<'_>,
    sm: &SourceMap,
    file: SourceId,
    line: u32,
    character: u32,
) -> Vec<LspRange> {
    let res = cx.resolutions();
    let Some((target, _)) = symbol_at(&res, file, sm, line, character) else {
        return Vec::new();
    };
    spans_for_target(&res, &target)
        .into_iter()
        .map(|s| span_to_range(sm, s))
        .collect()
}

pub fn highlights_at(
    cx: &QueryCtx<'_>,
    sm: &SourceMap,
    file: SourceId,
    line: u32,
    character: u32,
) -> Vec<LspRange> {
    references_at(cx, sm, file, line, character)
}

const KEYWORDS: &[&str] = &[
    "fn", "let", "if", "else", "return", "struct", "enum", "impl",
];

pub fn completions_at(cx: &QueryCtx<'_>, prefix: &str) -> Vec<String> {
    let res = cx.resolutions();
    let mut names: HashSet<String> = HashSet::new();
    for d in &res.defs {
        if d.name.starts_with(prefix) {
            names.insert(d.name.clone());
        }
    }
    for decls in res.body_locals.values() {
        for l in decls {
            if l.name.starts_with(prefix) {
                names.insert(l.name.clone());
            }
        }
    }
    for kw in KEYWORDS {
        if kw.starts_with(prefix) {
            names.insert((*kw).to_string());
        }
    }
    let mut out: Vec<_> = names.into_iter().collect();
    out.sort();
    out
}

pub fn rename_at(
    cx: &QueryCtx<'_>,
    sm: &SourceMap,
    file: SourceId,
    line: u32,
    character: u32,
    new_name: &str,
) -> Result<Vec<TextEdit>, &'static str> {
    rename_at_checked(cx, sm, file, line, character, new_name, None)
}

pub fn rename_at_checked(
    cx: &QueryCtx<'_>,
    sm: &SourceMap,
    file: SourceId,
    line: u32,
    character: u32,
    new_name: &str,
    diag: Option<&DiagCtxt>,
) -> Result<Vec<TextEdit>, &'static str> {
    let res = cx.resolutions();
    let Some((target, at_span)) = symbol_at(&res, file, sm, line, character) else {
        return Ok(Vec::new());
    };
    if new_name.is_empty() || !is_ident(new_name) || KEYWORDS.contains(&new_name) {
        emit_rename_invalid(diag, at_span);
        return Err("invalid rename target");
    }
    let conflict = match target {
        SymbolTarget::Def(d) => res
            .defs
            .iter()
            .enumerate()
            .any(|(i, x)| i as u32 != d.0 && x.name == new_name),
        SymbolTarget::Local { body, id } => res.body_locals.get(&body).is_some_and(|locals| {
            locals
                .iter()
                .enumerate()
                .any(|(i, l)| i as u32 != id.0 && l.name == new_name)
        }),
    };
    if conflict {
        emit_rename_invalid(diag, at_span);
        return Err("rename conflict");
    }
    Ok(spans_for_target(&res, &target)
        .into_iter()
        .map(|s| TextEdit {
            range: span_to_range(sm, s),
            new_text: new_name.to_string(),
        })
        .collect())
}

fn emit_rename_invalid(diag: Option<&DiagCtxt>, span: Span) {
    if let Some(diag) = diag {
        diag.struct_error(ErrorCode(7012), "toolchain.lsp_rename_invalid")
            .span_label(span, "rename target")
            .emit();
    }
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::span::{BytePos, Edition};

    //@ spec: RXS-0100
    #[test]
    fn completions_include_local_binding() {
        let src = "fn main() { let foo = 1; }";
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let file = sm.add_file("t.rx", src, Edition::Rx0);
        let cx = QueryCtx::new(src, file, Edition::Rx0, &diag);
        let items = completions_at(&cx, "fo");
        assert!(items.iter().any(|s| s == "foo"));
    }

    //@ spec: RXS-0101
    #[test]
    fn definition_finds_fn() {
        let src = "fn helper() -> i32 { 1 }\nfn main() { helper(); }";
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let file = sm.add_file("t.rx", src, Edition::Rx0);
        let cx = QueryCtx::new(src, file, Edition::Rx0, &diag);
        let helper_use = src.find("helper();").unwrap() + 1;
        let lc = sm.lookup(file, BytePos(helper_use as u32));
        let def = definition_at(&cx, &sm, file, lc.line - 1, lc.col - 1);
        assert!(def.is_some());
    }

    //@ spec: RXS-0103
    #[test]
    fn rename_produces_edits() {
        let src = "fn main() { let foo = 1; let bar = foo; }";
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let file = sm.add_file("t.rx", src, Edition::Rx0);
        let cx = QueryCtx::new(src, file, Edition::Rx0, &diag);
        let foo_off = src.find("foo").unwrap();
        let lc = sm.lookup(file, BytePos(foo_off as u32));
        let edits = rename_at(&cx, &sm, file, lc.line - 1, lc.col - 1, "baz").unwrap();
        assert!(edits.len() >= 2);
    }

    //@ spec: RXS-0101, RXS-0102, RXS-0103
    #[test]
    fn local_references_are_limited_to_body() {
        let src = "fn a() { let x = 1; let _ = x; }\nfn b() { let x = 2; let _ = x; }";
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let file = sm.add_file("t.rx", src, Edition::Rx0);
        let cx = QueryCtx::new(src, file, Edition::Rx0, &diag);
        let b_use = src.rfind("x;").unwrap();
        let lc = sm.lookup(file, BytePos(b_use as u32));
        let refs = references_at(&cx, &sm, file, lc.line - 1, lc.col - 1);
        assert_eq!(refs.len(), 2, "只应包含 b 内 x 的定义与使用");
        let edits = rename_at(&cx, &sm, file, lc.line - 1, lc.col - 1, "y").unwrap();
        assert_eq!(edits.len(), 2, "rename 不应跨函数体编辑 a::x");
    }

    //@ spec: RXS-0103
    #[test]
    fn rename_invalid_emits_rx7012() {
        let src = "fn main() { let foo = 1; foo; }";
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let file = sm.add_file("t.rx", src, Edition::Rx0);
        let cx = QueryCtx::new(src, file, Edition::Rx0, &diag);
        let foo_off = src.find("foo").unwrap();
        let lc = sm.lookup(file, BytePos(foo_off as u32));
        let err = rename_at_checked(&cx, &sm, file, lc.line - 1, lc.col - 1, "fn", Some(&diag));
        assert!(err.is_err());
        assert!(
            diag.emitted()
                .iter()
                .any(|d| d.code == Some(crate::diag::ErrorCode(7012)))
        );
    }
}
