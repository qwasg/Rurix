//! conformance/libdevice 语料批跑(M5.3,契约 D-M5-4;device 数学 intrinsic 正例
//! 全管线 0 诊断,RXS-0081/0082)。
//!
//! 管线:resolve → typeck(`f32`/`f64` 数学方法识别,RXS-0081)→ coloring →
//! device codegen(`__nv_*` 外部符号 declare/call,RXS-0081)。accept 正例须全程
//! 0 诊断且能产 device IR。libdevice bc 链接 + ptxas 真跑由
//! `libdevice_link_mapping.rs` 覆盖(缺工具链 SKIP)。

use std::path::PathBuf;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

mod common;
use common::{assert_spec_anchor, conformance_dir, read_source, rx_files};

fn libdevice_dir(sub: &str) -> PathBuf {
    conformance_dir("libdevice").join(sub)
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
        let src = read_source(&f);
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
        let src = read_source(&f);
        assert_spec_anchor(&src, &f);
    }
}
