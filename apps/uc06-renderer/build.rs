use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

const KERNELS: &[&str] = &[
    "cull",
    "visbuffer_sw_u64",
    "classify_resolve",
    "vsm_page_mark",
    "taa",
];

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    for stem in KERNELS {
        let source = manifest.join("kernels").join(format!("{stem}.rx"));
        println!("cargo:rerun-if-changed={}", source.display());
        let target = out.join(format!("{stem}.spv"));
        if std::env::var_os("CARGO_FEATURE_VULKAN").is_some() {
            let bytes = compile(&source, stem);
            std::fs::write(&target, bytes)
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        } else {
            std::fs::write(&target, [])
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        }
    }
}

fn compile(source: &Path, stem: &str) -> Vec<u8> {
    let src = std::fs::read_to_string(source)
        .unwrap_or_else(|e| panic!("read {}: {e}", source.display()));
    assert!(
        !src.contains('\r'),
        "{} must use LF line endings",
        source.display()
    );
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_launch();
    cx.check_crate_patterns();
    cx.check_views();
    cx.check_shared_barrier();
    cx.check_consteval();
    assert!(
        !diag.has_errors(),
        "{} frontend diagnostics: {:?}",
        source.display(),
        diag.emitted()
    );
    let words = rurixc::vulkan_codegen::build_and_emit_vulkan(&cx, stem)
        .unwrap_or_else(|| panic!("{} Vulkan codegen: {:?}", source.display(), diag.emitted()));
    assert!(
        !diag.has_errors(),
        "{} Vulkan diagnostics: {:?}",
        source.display(),
        diag.emitted()
    );
    rurixc::vulkan_codegen::words_to_bytes(&words)
}
