//! conformance/coloring + conformance/addrspace 语料批跑(M4.1,契约 D-M4-1;
//! 着色/地址空间反例全拦截与 accept 正例 0 诊断,RXS-0066/0067/0068)。
//!
//! 管线:resolve → typeck(含地址空间一致性 RX3002)→ 着色/barrier 骨架检查
//! (RX3001/RX3003);无需 MIR(着色/地址空间在 HIR/typeck 层,07 §3)。
//! reject 体例:`reject/<category>/*.rx`,文件头次行 `//@ expect-error: RX####`;
//! 批跑断言"产生诊断且全部为预期码"(反例全拦截口径)。

use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

mod common;
use common::{assert_spec_anchor, conformance_dir, expect_error_code, read_source, rx_files};

/// 两个语料根(着色 + 地址空间)。
const ROOTS: [&str; 2] = ["coloring", "addrspace"];

fn root_dir(root: &str, sub: &str) -> PathBuf {
    conformance_dir(Path::new("")).join(root).join(sub)
}

/// resolve → typeck → 着色/barrier 检查(HIR 层,无 MIR),返回错误码序列。
fn run_pipeline(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    // 着色/barrier(RX3001/RX3003)在 typeck 干净后跑;地址空间(RX3002)已在
    // typeck 内裁决(阶段化:前段有错即停,防级联)
    if !diag.has_errors() {
        cx.check_coloring();
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

#[test]
fn accept_corpus_is_diagnostic_free() {
    let mut total = 0;
    for root in ROOTS {
        let files = rx_files(&root_dir(root, "accept"));
        for f in files {
            total += 1;
            let src = read_source(&f);
            let codes = run_pipeline(&src);
            assert!(
                codes.is_empty(),
                "{} 产生诊断: {codes:?}(accept 正例须 0 诊断)",
                f.display()
            );
        }
    }
    assert!(total > 0, "coloring/addrspace accept 正例集为空");
}

#[test]
fn reject_corpus_all_intercepted() {
    let mut total = 0;
    for root in ROOTS {
        let files = rx_files(&root_dir(root, "reject"));
        for f in files {
            total += 1;
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
    assert!(total > 0, "coloring/addrspace reject 反例集为空");
}

/// 反例覆盖契约预设类别(coloring 三类 + addrspace 一类;目录即类别)。
#[test]
fn reject_has_expected_categories() {
    let coloring_reject = root_dir("coloring", "reject");
    for cat in [
        "host_in_device",
        "direct_kernel_call",
        "barrier_non_uniform",
    ] {
        let d = coloring_reject.join(cat);
        assert!(
            d.is_dir() && !rx_files(&d).is_empty(),
            "缺类别目录或为空: coloring/reject/{cat}/"
        );
    }
    let addrspace_reject = root_dir("addrspace", "reject").join("space_mismatch");
    assert!(
        addrspace_reject.is_dir() && !rx_files(&addrspace_reject).is_empty(),
        "缺类别目录或为空: addrspace/reject/space_mismatch/"
    );
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for root in ROOTS {
        for f in rx_files(&root_dir(root, "")) {
            let src = read_source(&f);
            assert_spec_anchor(&src, &f);
        }
    }
}
