//! conformance/resolve 语义正例跑批(契约 D-M2-1;M2_PLAN §1 出口判据,
//! 作用面调整留痕见 M2_PLAN 修订记录 v1.1)。
//!
//! 门作用面:`conformance/resolve/` 全量(自包含程序,无悬空引用)——
//! lex + parse + resolve 0 诊断且产出 HIR;`conformance/syntax/` 维持 parse 门
//! (其样例是含草图引用的语法正例,且 names_duplicates.rx 是故意的 resolve 反例)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::lexer::lex;
use rurixc::lower::lower;
use rurixc::parser::parse;
use rurixc::resolve::resolve;
use rurixc::span::{Edition, SourceId};

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/resolve")
}

fn collect_rx_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("读取语义样例目录失败") {
        let path = entry.expect("读取目录项失败").path();
        if path.is_dir() {
            collect_rx_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rx") {
            out.push(path);
        }
    }
}

fn corpus() -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rx_files(&corpus_dir(), &mut files);
    files.sort();
    files
}

#[test]
fn resolve_corpus_is_not_empty() {
    let n = corpus().len();
    assert!(n >= 10, "语义正例集过小: {n} 个(M2_PLAN §1:>=10)");
}

/// M2.1 出口判据:语义正例全量 0 诊断(lex + parse + resolve)且可降级 HIR。
#[test]
fn resolve_corpus_is_diagnostic_free() {
    for file in corpus() {
        let src = fs::read_to_string(&file).expect("读取样例失败");
        let diag = DiagCtxt::new();
        let tokens = lex(&src, SourceId(0), Edition::Rx0, &diag);
        let ast = parse(&src, tokens, SourceId(0), Edition::Rx0, &diag);
        let res = resolve(&ast, &diag);
        assert!(
            diag.emitted().is_empty(),
            "{} 产生诊断: {:?}",
            file.display(),
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message(diag.messages())))
                .collect::<Vec<_>>()
        );
        let krate = lower(&ast, &res);
        assert!(
            !krate.root_items.is_empty(),
            "{} 未产出 HIR item",
            file.display()
        );
    }
}

#[test]
fn resolve_corpus_files_carry_spec_anchor() {
    for file in corpus() {
        let src = fs::read_to_string(&file).expect("读取样例失败");
        let first = src.lines().next().unwrap_or("");
        assert!(
            first.starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头(//@ spec: RXS-####)",
            file.display()
        );
    }
}
