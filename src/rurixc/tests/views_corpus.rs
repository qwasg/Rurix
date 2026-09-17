//! conformance/views 语料批跑(M5.1,契约 D-M5-1 / G-M5-2;views 不相交证明
//! 反例全拦截与 accept 正例 0 诊断,RXS-0078)。
//!
//! 管线:resolve → typeck(含 View 族算子 split_at/chunks/windows 定型)→ 着色
//! 骨架(RX3001/RX3003)→ views 不相交 device 借用扩展(RX3007/RX3008);HIR 层,
//! 无需 MIR(device 上下文 body 不在 host `main` 可达 MIR 内,07 §4)。
//! reject 体例:`reject/<category>/*.rx`,文件头次行 `//@ expect-error: RX####`;
//! 批跑断言"产生诊断且全部为预期码"(反例全拦截口径,对齐 coloring/launch 语料)。

use std::path::PathBuf;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

mod common;
use common::{assert_spec_anchor, conformance_dir, expect_error_code, read_source, rx_files};

fn views_dir(sub: &str) -> PathBuf {
    conformance_dir("views").join(sub)
}

/// resolve → typeck → 着色 → views 不相交检查(HIR 层,无 MIR),返回错误码序列。
/// 阶段化:前段有错即停(防级联),views 在 typeck/着色干净后跑。
fn run_pipeline(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if !diag.has_errors() {
        cx.check_coloring();
    }
    if !diag.has_errors() {
        cx.check_views();
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

#[test]
fn accept_corpus_is_diagnostic_free() {
    let files = rx_files(&views_dir("accept"));
    assert!(!files.is_empty(), "views accept 正例集为空");
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

#[test]
fn reject_corpus_all_intercepted() {
    let files = rx_files(&views_dir("reject"));
    assert!(!files.is_empty(), "views reject 反例集为空");
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

/// 反例覆盖契约预设类别(M5.1 纯 views 四类;目录即类别,m5.counter
/// .views_conformance_categories ≥4)。
#[test]
fn reject_has_expected_categories() {
    let reject = views_dir("reject");
    for cat in [
        "overlap_windows",
        "alias_mut_write",
        "split_out_of_bounds",
        "chunk_zero_or_window_oversize",
    ] {
        let d = reject.join(cat);
        assert!(
            d.is_dir() && !rx_files(&d).is_empty(),
            "缺类别目录或为空: views/reject/{cat}/"
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for sub in ["accept", "reject"] {
        for f in rx_files(&views_dir(sub)) {
            let src = read_source(&f);
            assert_spec_anchor(&src, &f);
        }
    }
}
