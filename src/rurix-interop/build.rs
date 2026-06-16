//! rurix-interop 构建脚本(M8.1,D-M8-1):复用 M5 自研 kernel
//! (`../rurix-rt/kernels/{saxpy,reduce,gemm_tile}.rx`)全管线产 PTX 嵌入 staticlib
//! (单一事实源,对齐 rurix-rt build.rs)。UC-01 算子替换零拷贝接入 PyTorch 经这三
//! 个 kernel(SAXPY / Reduction / GEMM 类瓶颈算子)。
//!
//! 工具链缺失优雅降级:rurixc 检查/clang 任一失败 → 写空哨兵 PTX + 空入口名,
//! FFI 据空在运行时返回失败(真红绿在带 clang+GPU 的 self-hosted runner)。

use std::path::PathBuf;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

/// UC-01 复用的 M5 kernel(干名 = module 名 = 常量前缀小写)。每项产
/// `$OUT_DIR/{name}.ptx` + `{name}_meta.rs`(常量 `{UPPER}_KERNEL` = 入口符号名)。
const KERNELS: &[&str] = &["saxpy", "reduce", "gemm_tile"];

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // M5 kernel 源在 rurix-rt crate(单一事实源,不复制)。
    let kernels_dir = manifest.join("..").join("rurix-rt").join("kernels");
    println!("cargo:rerun-if-env-changed=RURIXC_CLANG");
    println!("cargo:rerun-if-env-changed=CUDA_PATH");

    for name in KERNELS {
        let kernel_rx = kernels_dir.join(format!("{name}.rx"));
        println!("cargo:rerun-if-changed={}", kernel_rx.display());
        let ptx_out = out_dir.join(format!("{name}.ptx"));
        let meta_out = out_dir.join(format!("{name}_meta.rs"));
        let upper = name.to_uppercase();
        match gen_ptx(&kernel_rx, &ptx_out, name) {
            Ok(kernel) => {
                std::fs::write(
                    &meta_out,
                    format!("pub const {upper}_KERNEL: &str = \"{kernel}\";\n"),
                )
                .unwrap_or_else(|e| panic!("write {name}_meta.rs: {e}"));
            }
            Err(reason) => {
                println!(
                    "cargo:warning=rurix-interop: {name} device codegen unavailable, embedded PTX skipped ({reason})"
                );
                std::fs::write(&ptx_out, "")
                    .unwrap_or_else(|e| panic!("write sentinel {name}.ptx: {e}"));
                std::fs::write(
                    &meta_out,
                    format!("pub const {upper}_KERNEL: &str = \"\";\n"),
                )
                .unwrap_or_else(|e| panic!("write sentinel {name}_meta.rs: {e}"));
            }
        }
    }
}

/// `<name>.rx` → PTX(写 `ptx_out`),返回 `ptx_kernel` 入口符号名(对齐 rurix-rt
/// build.rs 管线:全量静态检查 → device codegen → libdevice 关卡 → IR→PTX → ptxas 干验证)。
fn gen_ptx(
    kernel_rx: &std::path::Path,
    ptx_out: &std::path::Path,
    module: &str,
) -> Result<String, String> {
    let src = std::fs::read_to_string(kernel_rx)
        .map_err(|e| format!("cannot read {}: {e}", kernel_rx.display()))?;

    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if diag.has_errors() {
        return Err("typeck reported errors".to_owned());
    }
    cx.check_coloring();
    cx.check_launch();
    cx.check_crate_patterns();
    cx.check_views();
    cx.check_shared_barrier();
    if diag.has_errors() {
        return Err("coloring/views/shared checks reported errors".to_owned());
    }
    let ir = rurixc::device_codegen::build_and_emit(&cx, module)
        .ok_or_else(|| "no device IR (kernel codegen failed)".to_owned())?;
    if diag.has_errors() {
        return Err("device codegen reported errors".to_owned());
    }
    if matches!(
        rurixc::toolchain::libdevice_link_for(&ir),
        rurixc::toolchain::LibdeviceLink::MissingSkip
    ) {
        return Err("libdevice.10.bc not found (no CUDA toolchain)".to_owned());
    }
    let ptx = rurixc::toolchain::ir_to_ptx(&ir, ptx_out)?;
    match rurixc::ptxas::dry_gate(&ptx, module) {
        rurixc::ptxas::PtxasOutcome::Pass | rurixc::ptxas::PtxasOutcome::Skipped => {}
        rurixc::ptxas::PtxasOutcome::Rejected(reason) => {
            return Err(format!(
                "ptxas -arch=sm_89 rejected generated PTX: {reason}"
            ));
        }
        rurixc::ptxas::PtxasOutcome::Toolchain(e) => {
            return Err(format!("ptxas toolchain error: {e}"));
        }
    }
    parse_entry(&ir, &ptx).ok_or_else(|| "cannot locate ptx_kernel entry symbol".to_owned())
}

/// 从 NVPTX IR(`define ptx_kernel void @<name>(`)解析入口符号名,PTX 兜底核对。
fn parse_entry(ir: &str, ptx: &str) -> Option<String> {
    let from_ir = ir.find("define ptx_kernel").and_then(|pos| {
        let after_def = &ir[pos..];
        let at = after_def.find(" @")?;
        let rest = &after_def[at + 2..];
        let end = rest.find('(').unwrap_or(rest.len());
        let name = rest[..end].trim();
        (!name.is_empty()).then(|| name.to_owned())
    });
    if let Some(name) = from_ir
        && ptx.contains(&name)
    {
        return Some(name);
    }
    let idx = ptx.find(".entry")?;
    let rest = ptx[idx + ".entry".len()..].trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
        .collect();
    (!name.is_empty()).then_some(name)
}
