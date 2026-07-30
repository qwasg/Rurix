//! conformance/atomics 语料批跑(M5.2,契约 D-M5-3 / G-M5-3;scoped atomics scope
//! 类型契约违例全拦截与 accept 正例 0 诊断,RXS-0080)。
//!
//! 管线:resolve → typeck(含 `Atomic`/`AtomicView` 原子方法识别 + scope 类型契约
//! 裁决,RX3010);scope 误用为编译期 typeck 层裁决,不依赖数据流(RXS-0080
//! Implementation Requirements)。reject 体例:`reject/<category>/*.rx`,文件头次行
//! `//@ expect-error: RX####`;批跑断言"产生诊断且全部为预期码"(反例全拦截口径)。
//!
//! 注:PTX `atom.{order}.{scope}` 映射为 D-406 / RD-008 高敏面(deferred,agent 可落笔、经 owner
//! 批准后落地),本语料只覆盖 safe
//! 层 scope 类型契约,不涉映射真跑(映射真跑随承接 PR + Compute Sanitizer,G-M5-4)。

use std::path::PathBuf;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

mod common;
use common::{assert_spec_anchor, conformance_dir, expect_error_code, read_source, rx_files};

fn atomics_dir(sub: &str) -> PathBuf {
    conformance_dir("atomics").join(sub)
}

/// resolve → typeck(scoped atomics scope 类型契约,RX3010)。scope 误用在 typeck
/// 层即裁决,无需后续 device 借用扩展。
fn run_pipeline(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

#[test]
fn accept_corpus_is_diagnostic_free() {
    let files = rx_files(&atomics_dir("accept"));
    assert!(!files.is_empty(), "atomics accept 正例集为空");
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
    let files = rx_files(&atomics_dir("reject"));
    assert!(!files.is_empty(), "atomics reject 反例集为空");
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

/// 反例覆盖预设类别(scoped atomics scope 误用;目录即类别)。
#[test]
fn reject_has_expected_categories() {
    let reject = atomics_dir("reject");
    for cat in ["scope_addrspace_incompat", "scope_overreach"] {
        let d = reject.join(cat);
        assert!(
            d.is_dir() && !rx_files(&d).is_empty(),
            "缺类别目录或为空: atomics/reject/{cat}/"
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for sub in ["accept", "reject"] {
        for f in rx_files(&atomics_dir(sub)) {
            let src = read_source(&f);
            assert_spec_anchor(&src, &f);
        }
    }
}
