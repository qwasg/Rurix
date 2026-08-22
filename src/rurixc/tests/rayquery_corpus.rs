//! conformance/rayquery 语料批跑(G7.2 W3a,体例沿 tests/shared_corpus.rs;
//! RXS-0297~0300,RFC-0018 §3.A5)。
//!
//! 管线(driver.rs 同序,前段有错即停防级联):shader-stages AST 层位置纪律
//! (RX3013,resolve 后、typeck 前)→ typeck → 着色骨架 → views 不相交 →
//! shared+barrier 一致性 → RayQuery 状态机(S2 前向 may-terminated 数据流 +
//! S3 committed_* 守卫支配,MIR 层,RX3018)。reject 体例:`reject/*.rx`,文件头
//! `//@ expect-error: RX####` 行;批跑断言"产生诊断且全部为预期码"(反例全
//! 拦截口径)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn rayquery_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/rayquery")
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

/// shader-stages → typeck → 着色 → views → shared+barrier → RayQuery 状态机
/// (driver.rs 同序;前段有错即停防级联),返回诊断码序列。
fn run_pipeline(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_shader_stages();
    if !diag.has_errors() {
        cx.check_crate();
    }
    if !diag.has_errors() {
        cx.check_coloring();
    }
    if !diag.has_errors() {
        cx.check_views();
    }
    if !diag.has_errors() {
        cx.check_shared_barrier();
    }
    if !diag.has_errors() {
        cx.check_ray_query();
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

#[test]
fn accept_corpus_is_diagnostic_free() {
    let files = rx_files(&rayquery_dir("accept"));
    assert!(!files.is_empty(), "rayquery accept 正例集为空");
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
    let files = rx_files(&rayquery_dir("reject"));
    assert!(!files.is_empty(), "rayquery reject 反例集为空");
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

/// 语料防静默丢失锚定:accept/reject 各含预设文件。
#[test]
fn corpus_contains_expected_files() {
    let accept = rx_files(&rayquery_dir("accept"));
    for name in [
        "ray_query_basic.rx",
        // first-hit 早退构造内建(RFC-0030 §4.6)。
        "ray_query_first_hit.rx",
    ] {
        assert!(
            accept
                .iter()
                .any(|f| f.file_name().is_some_and(|n| n == name)),
            "rayquery/accept 缺 {name}"
        );
    }
    let reject = rx_files(&rayquery_dir("reject"));
    for name in [
        "ray_query_escape.rx",
        "ray_query_after_terminate.rx",
        "committed_unguarded.rx",
        // first-hit 变体的 S3 协议反例(RFC-0030 §4.6)。
        "first_hit_committed_unguarded.rx",
    ] {
        assert!(
            reject
                .iter()
                .any(|f| f.file_name().is_some_and(|n| n == name)),
            "rayquery/reject 缺 {name}"
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for sub in ["accept", "reject"] {
        for f in rx_files(&rayquery_dir(sub)) {
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
