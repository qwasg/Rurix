//! conformance/borrowck 语料批跑(M3.2 出口判据:契约 §4 类别 1/2 反例
//! 全拦截 + accept 正例 0 诊断;CI 步骤 15 的 cargo 侧先行形态,工作流
//! 接入随 M3.3,M3 CI_GATES §2)。
//!
//! reject 体例:`reject/<category>/*.rx`,文件头 `//@ expect-error: RX####`
//! 声明预期错误码;批跑断言"产生诊断且全部为预期码"(反例全拦截口径)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/borrowck").join(sub)
}

fn rx_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", d.display())) {
            let p = e.expect("读取目录项失败").path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rx") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// 全管线(含 move/init 检查)跑单文件,返回错误码序列。
fn run_pipeline(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if !diag.has_errors() {
        cx.check_crate_patterns();
        let _ = cx.mir_crate();
        cx.check_moves();
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

//@ spec: RXS-0054
#[test]
fn accept_corpus_is_diagnostic_free() {
    let files = rx_files(&dir("accept"));
    assert!(!files.is_empty(), "accept 正例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let codes = run_pipeline(&src);
        assert!(
            codes.is_empty(),
            "{} 产生诊断: {codes:?}(accept 正例须 0 诊断)",
            f.display()
        );
    }
}

//@ spec: RXS-0054
#[test]
fn reject_corpus_all_intercepted() {
    let files = rx_files(&dir("reject"));
    assert!(
        files.len() >= 4,
        "reject 反例集过小: {} 个(类别 1/2 各 ≥2,M3.2 出口判据)",
        files.len()
    );
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let expected: u16 = src
            .lines()
            .find_map(|l| l.trim().strip_prefix("//@ expect-error: RX"))
            .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", f.display()))
            .trim()
            .parse()
            .expect("expect-error 码格式非法");
        let codes = run_pipeline(&src);
        assert!(
            !codes.is_empty(),
            "{} 未被拦截(反例全拦截口径)",
            f.display()
        );
        assert!(
            codes.iter().all(|c| *c == expected),
            "{} 诊断码偏离预期 RX{expected}: {codes:?}",
            f.display()
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for f in rx_files(&dir("")) {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let first = src.lines().next().unwrap_or("");
        assert!(
            first.starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头(//@ spec: RXS-####)",
            f.display()
        );
    }
}
