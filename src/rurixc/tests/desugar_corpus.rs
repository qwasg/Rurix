//! conformance/desugar 正例全管线批跑(M3.1 出口判据,契约 D-M3-1):
//! desugar 后正例经 lex + parse + resolve + typeck + 模式穷尽性(TBIR 窄门)
//! + MIR 构建全程 **0 诊断**且产出非空 MIR(可执行真跑经
//! `py -3 ci/hello_smoke.py desugar-smoke`,对齐步骤 12 形态)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn corpus() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/desugar");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("读取 conformance/desugar 失败")
        .filter_map(|e| {
            let p = e.expect("读取目录项失败").path();
            (p.extension().is_some_and(|x| x == "rx")).then_some(p)
        })
        .collect();
    files.sort();
    files
}

#[test]
fn desugar_corpus_is_not_empty() {
    let n = corpus().len();
    assert!(n >= 6, "desugar 正例集过小: {n} 个(M3.1 五条款锚定面)");
}

/// M3.1 出口判据:for/`?` 用例在内的 desugar 正例全管线 0 诊断。
#[test]
fn desugar_corpus_full_pipeline_diagnostic_free() {
    for file in corpus() {
        let src = fs::read_to_string(&file).expect("读取样例失败");
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_crate_patterns(); // TBIR 窄门(RXS-0051)
        let mir = cx.mir_crate(); // 单态化收集 + TBIR→MIR(RX6001 面)
        cx.check_moves(); // move/init 数据流(M3.2,RXS-0054)
        assert!(
            diag.emitted().is_empty(),
            "{} 产生诊断: {:?}",
            file.display(),
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message(diag.messages())))
                .collect::<Vec<_>>()
        );
        assert!(!mir.is_empty(), "{} 未产出 MIR body", file.display());
    }
}

#[test]
fn desugar_corpus_files_carry_spec_anchor() {
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
