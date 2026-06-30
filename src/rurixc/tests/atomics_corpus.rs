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

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn atomics_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/atomics")
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
    let files = rx_files(&atomics_dir("reject"));
    assert!(!files.is_empty(), "atomics reject 反例集为空");
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
