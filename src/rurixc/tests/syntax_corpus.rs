//! conformance/syntax 样例集跑批(契约 G-M1-1 通道,M1 CI_GATES §2 步骤 9)。
//!
//! M1.2 形态:全量样例 0 词法错误;M1.3 升级为 100% 解析。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::lexer::lex;
use rurixc::span::{Edition, SourceId};

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/syntax")
}

fn collect_rx_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("读取样例目录失败") {
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
fn corpus_is_not_empty() {
    let n = corpus().len();
    assert!(
        n >= 20,
        "语法样例集过小: {n} 个(M1.2 起步 ~26,M1.3 扩到 >=100)"
    );
}

#[test]
fn corpus_lexes_with_zero_diagnostics() {
    for file in corpus() {
        let src = fs::read_to_string(&file).expect("读取样例失败");
        let diag = DiagCtxt::new();
        let tokens = lex(&src, SourceId(0), Edition::Rx0, &diag);
        assert!(
            diag.emitted().is_empty(),
            "{} 产生词法诊断: {:?}",
            file.display(),
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message_key.clone()))
                .collect::<Vec<_>>()
        );
        assert!(tokens.len() > 1, "{} 未产出 token", file.display());
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    // traceability 锚定注释(spec/README.md §2;M1.4 矩阵工具收割)
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
