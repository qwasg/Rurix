//! conformance/launch 语料批跑(M4.3,契约 D-M4-6 / G-M4-2;launch 类型契约
//! 四类反例全拦截与 accept 正例 0 诊断,RXS-0074/0075)。
//!
//! 管线:resolve → typeck → 着色检查 → launch 类型契约检查(HIR 层,07 §3);
//! 无需 MIR。reject 体例:`reject/<category>/*.rx`,文件头含 `//@ expect-error:
//! RX####`;批跑断言"产生诊断且全部为预期码"(反例全拦截口径,对齐 coloring)。
//!
//! 四类(契约 §4.2):launch_non_kernel(RX3004)/ dim_mismatch(RX3005)/
//! arg_type_mismatch(RX2001 复用)/ context_brand_mismatch(RX3006)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

/// 契约预设四类(目录即类别,数量为 m4.counter.launch_conformance_categories 计数对象)。
const REJECT_CATEGORIES: [&str; 4] = [
    "launch_non_kernel",
    "dim_mismatch",
    "arg_type_mismatch",
    "context_brand_mismatch",
];

fn launch_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/launch")
        .join(sub)
}

fn rx_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return out;
    }
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

/// resolve → typeck → 着色 → launch 检查(HIR 层,无 MIR),返回错误码序列。
fn run_pipeline(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    // launch 类型契约在 typeck/着色干净后跑(阶段化:前段有错即停,防级联)
    if !diag.has_errors() {
        cx.check_coloring();
    }
    if !diag.has_errors() {
        cx.check_launch();
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

#[test]
fn accept_corpus_is_diagnostic_free() {
    let files = rx_files(&launch_dir("accept"));
    assert!(!files.is_empty(), "launch accept 正例集为空");
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

#[test]
fn reject_corpus_all_intercepted() {
    let files = rx_files(&launch_dir("reject"));
    assert!(!files.is_empty(), "launch reject 反例集为空");
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
        assert!(!codes.is_empty(), "{} 未被拦截(反例全拦截口径)", f.display());
        assert!(
            codes.iter().all(|c| *c == expected),
            "{} 诊断码偏离预期 RX{expected}: {codes:?}",
            f.display()
        );
    }
}

/// 反例覆盖契约预设四类(目录即类别;m4.counter.launch_conformance_categories ≥4)。
#[test]
fn reject_has_expected_categories() {
    let reject = launch_dir("reject");
    for cat in REJECT_CATEGORIES {
        let d = reject.join(cat);
        assert!(
            d.is_dir() && !rx_files(&d).is_empty(),
            "缺类别目录或为空: launch/reject/{cat}/"
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for sub in ["accept", "reject"] {
        for f in rx_files(&launch_dir(sub)) {
            let src = fs::read_to_string(&f).expect("读取样例失败");
            let first = src.lines().next().unwrap_or("");
            assert!(
                first.starts_with("//@ spec: RXS-"),
                "{} 缺条款锚定头(//@ spec: RXS-####)",
                f.display()
            );
        }
    }
}
