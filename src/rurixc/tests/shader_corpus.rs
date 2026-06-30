//! conformance/shader 着色阶段语料批跑(G2.1,RFC-0002;cargo feature
//! `shader-stages`)。着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例 reject 全拦截
//! 与 accept 正例 0 诊断(RXS-0153~0156)。
//!
//! 管线:着色阶段类型面检查(RX3011~3013;AST 层)→ resolve → typeck → 着色/barrier
//! (RX3001 复用,着色阶段入口直接调用)。镜像 driver:句柄位置违例先于 typeck
//! body↔返回类型匹配裁决(避免 RX2001 掩盖 RX3013)。纯 host/CPU-only(着色阶段类型面为
//! 编译期,无 device)。reject 体例:`reject/<category>/*.rx`,文件头次行
//! `//@ expect-error: RX####`。
#![cfg(feature = "shader-stages")]

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn shader_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/shader")
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

/// 着色阶段类型面检查(AST 层,RX3011~3013)→ resolve → typeck → 着色/barrier
/// (RX3001 复用)。阶段化镜像 driver(`driver.rs`):资源句柄位置违例须在 typeck
/// body↔返回类型匹配前裁决,否则非法句柄返回类型先触 RX2001 掩盖 spec 强制的
/// RX3013(RXS-0156)。前段有错即停,防级联。返回错误码序列。
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
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

#[test]
fn accept_corpus_is_diagnostic_free() {
    let files = rx_files(&shader_dir("accept"));
    assert!(!files.is_empty(), "conformance/shader/accept 正例集为空");
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
    let files = rx_files(&shader_dir("reject"));
    assert!(!files.is_empty(), "conformance/shader/reject 反例集为空");
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

/// reject 覆盖四类(着色阶段误用 / I/O 标注 / 接口不匹配 / 资源句柄;目录即类别)。
#[test]
fn reject_has_expected_categories() {
    for cat in [
        "stage_misuse",
        "io_annotation",
        "interface_mismatch",
        "resource_handle",
    ] {
        let d = shader_dir("reject").join(cat);
        assert!(
            d.is_dir() && !rx_files(&d).is_empty(),
            "缺类别目录或为空: conformance/shader/reject/{cat}/"
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for f in rx_files(&shader_dir("")) {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let first = src.lines().next().unwrap_or("");
        assert!(
            first.starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头(//@ spec: RXS-####)",
            f.display()
        );
    }
}
