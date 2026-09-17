//! conformance/borrowck 语料批跑(契约 §4 / G-M3-1:7 类错误类别反例全拦截
//! 与 accept 正例 0 诊断;CI 步骤 15 = `cargo test -p rurixc --test borrowck_corpus`,
//! M3.3 WP4 接入 pr-smoke 工作流,M3 CI_GATES §2)。
//!
//! reject 体例:`reject/<category>/*.rx`,文件头 `//@ expect-error: RX####`
//! 声明预期错误码;批跑断言"产生诊断且全部为预期码"(反例全拦截口径)。
//! 类别覆盖面(目录数)由 `m3.counter.borrowck_conformance_categories` 核对(≥7)。

use std::path::PathBuf;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

mod common;
use common::{assert_spec_anchor, conformance_dir, expect_error_code, read_source, rx_files};

fn dir(sub: &str) -> PathBuf {
    conformance_dir("borrowck").join(sub)
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
        if !diag.has_errors() {
            cx.check_borrows();
        }
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
        let src = read_source(&f);
        let codes = run_pipeline(&src);
        assert!(
            codes.is_empty(),
            "{} 产生诊断: {codes:?}(accept 正例须 0 诊断)",
            f.display()
        );
    }
}

/// 契约 §4 预设的 7 类错误类别(reject/ 下子目录名)。
const REJECT_CATEGORIES: [&str; 7] = [
    "use_after_move",
    "use_before_init",
    "double_mut_borrow",
    "shared_mut_conflict",
    "move_while_borrowed",
    "assign_while_borrowed",
    "dangling_reference",
];

//@ spec: RXS-0058
#[test]
fn reject_has_all_seven_categories() {
    let reject = dir("reject");
    for cat in REJECT_CATEGORIES {
        let d = reject.join(cat);
        assert!(
            d.is_dir() && !rx_files(&d).is_empty(),
            "缺类别目录或为空: reject/{cat}/(契约 §4 七类,G-M3-1)"
        );
    }
    let present: usize = std::fs::read_dir(&reject)
        .expect("读取 reject/ 失败")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .count();
    assert!(
        present >= 7,
        "reject 类别目录数 {present} < 7(m3.counter.borrowck_conformance_categories)"
    );
}

//@ spec: RXS-0054
#[test]
fn reject_corpus_all_intercepted() {
    let files = rx_files(&dir("reject"));
    assert!(
        files.len() >= REJECT_CATEGORIES.len(),
        "reject 反例集过小: {} 个(契约 §4 七类各 ≥1)",
        files.len()
    );
    for f in files {
        let src = read_source(&f);
        let expected: u16 = expect_error_code(&src, &f);
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
        let src = read_source(&f);
        assert_spec_anchor(&src, &f);
    }
}
