//! conformance/libdevice 语料批跑(M5.3,契约 D-M5-4;device 数学 intrinsic 正例
//! 全管线 0 诊断,RXS-0081/0082)。
//!
//! 管线:resolve → typeck(`f32`/`f64` 数学方法识别,RXS-0081)→ coloring →
//! device codegen(`__nv_*` 外部符号 declare/call,RXS-0081)。accept 正例须全程
//! 0 诊断且能产 device IR。libdevice bc 链接 + ptxas 真跑由
//! `libdevice_link_mapping.rs` 覆盖(缺工具链 SKIP)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn libdevice_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/libdevice")
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

/// 全管线(typeck → coloring → patterns → consteval → device codegen);返回诊断码。
fn run_pipeline(src: &str, module: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if !diag.has_errors() {
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        if !diag.has_errors() {
            let _ = rurixc::device_codegen::build_and_emit(&cx, module);
        }
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

#[test]
fn accept_corpus_is_diagnostic_free() {
    let files = rx_files(&libdevice_dir("accept"));
    assert!(!files.is_empty(), "libdevice accept 正例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        let codes = run_pipeline(&src, &stem);
        assert!(
            codes.is_empty(),
            "{} 产生诊断: {codes:?}(accept 正例须全管线 0 诊断)",
            f.display()
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for f in rx_files(&libdevice_dir("accept")) {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let first = src.lines().next().unwrap_or("");
        assert!(
            first.starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头(//@ spec: RXS-####)",
            f.display()
        );
    }
}
