//! rx fmt 幂等性跑批(契约 G-M1-5 的 cargo test 侧通道;close-out 判据脚本为
//! ci/check_fmt_idempotent.py,二者同判据:字节级 fmt(fmt(x)) == fmt(x))。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::fmt::format_source;
use rurixc::lexer::lex;
use rurixc::parser::parse;
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

/// G-M1-5:语法样例集全量 fmt(fmt(x)) == fmt(x)(字节级)。
#[test]
fn fmt_is_idempotent_on_syntax_corpus() {
    let mut files = Vec::new();
    collect_rx_files(&corpus_dir(), &mut files);
    files.sort();
    assert!(files.len() >= 100, "语料缺失");
    for file in files {
        let src = fs::read_to_string(&file).expect("读取样例失败");
        let once =
            format_source(&src).unwrap_or_else(|e| panic!("{} fmt 失败: {e}", file.display()));
        let twice = format_source(&once)
            .unwrap_or_else(|e| panic!("{} 二次 fmt 失败: {e}", file.display()));
        assert_eq!(once, twice, "{} fmt 不幂等", file.display());
    }
}

/// fmt 输出仍可被 parser 接受(0 诊断;防 fmt 产出破坏语法的文本)。
#[test]
fn fmt_output_still_parses_clean() {
    let mut files = Vec::new();
    collect_rx_files(&corpus_dir(), &mut files);
    files.sort();
    for file in files {
        let src = fs::read_to_string(&file).expect("读取样例失败");
        let formatted =
            format_source(&src).unwrap_or_else(|e| panic!("{} fmt 失败: {e}", file.display()));
        let diag = DiagCtxt::new();
        let tokens = lex(&formatted, SourceId(0), Edition::Rx0, &diag);
        let _ = parse(&formatted, tokens, SourceId(0), Edition::Rx0, &diag);
        assert!(
            diag.emitted().is_empty(),
            "{} fmt 输出解析失败: {:?}",
            file.display(),
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message_key.clone()))
                .collect::<Vec<_>>()
        );
    }
}
