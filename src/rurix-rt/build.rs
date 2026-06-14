//! rurix-rt 构建脚本(M4.4,契约 D-M4-5):Rurix `kernel fn saxpy` 全管线产 PTX
//! 并嵌入 host EXE data 段(06 §5.2)。
//!
//! 管线(复用 rurixc 库,单一事实源):`kernels/saxpy.rx` → 着色检查 →
//! `device_codegen::build_and_emit`(NVPTX 约束 LLVM IR)→ `toolchain::ir_to_ptx`
//! (pin 的 clang `--target=nvptx64-nvidia-cuda`)→ PTX →(有 ptxas 则)ptxas
//! `-arch=sm_89` 干验证关卡(RXS-0073)。产物写 `$OUT_DIR/saxpy.ptx` + 入口符号名
//! `$OUT_DIR/saxpy_meta.rs`(`rx_saxpy_<defid>` 从 IR/PTX 解析,不硬编码)。
//!
//! **工具链缺失优雅降级**:rurixc 检查/clang 任一失败 → 写空哨兵 PTX + 空入口名,
//! bin/test 据 `SAXPY_PTX.is_empty()` 运行时 SKIP(对齐 ptxas SKIP 纪律,真红绿在带
//! clang+GPU 的 self-hosted runner;保 `cargo build` 全平台绿)。

use std::path::PathBuf;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let kernel_rx = manifest.join("kernels").join("saxpy.rx");
    println!("cargo:rerun-if-changed={}", kernel_rx.display());
    println!("cargo:rerun-if-env-changed=RURIXC_CLANG");

    let ptx_out = out_dir.join("saxpy.ptx");
    let meta_out = out_dir.join("saxpy_meta.rs");

    match gen_ptx(&kernel_rx, &ptx_out) {
        Ok(kernel) => {
            std::fs::write(
                &meta_out,
                format!("pub const SAXPY_KERNEL: &str = \"{kernel}\";\n"),
            )
            .expect("write saxpy_meta.rs");
        }
        Err(reason) => {
            // 降级:空哨兵 PTX + 空入口名(bin/test 运行时据空 SKIP)
            println!("cargo:warning=rurix-rt: SAXPY device codegen unavailable, embedded PTX skipped ({reason})");
            std::fs::write(&ptx_out, "").expect("write sentinel saxpy.ptx");
            std::fs::write(&meta_out, "pub const SAXPY_KERNEL: &str = \"\";\n")
                .expect("write sentinel saxpy_meta.rs");
        }
    }
}

/// `saxpy.rx` → PTX(写 `ptx_out`),返回 `ptx_kernel` 入口符号名。
fn gen_ptx(kernel_rx: &std::path::Path, ptx_out: &std::path::Path) -> Result<String, String> {
    let src = std::fs::read_to_string(kernel_rx)
        .map_err(|e| format!("cannot read {}: {e}", kernel_rx.display()))?;

    // 全量静态检查(typeck → 着色 → 穷尽性),再 device codegen(kernel 为根)
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if diag.has_errors() {
        return Err("typeck reported errors".to_owned());
    }
    cx.check_coloring();
    cx.check_launch();
    cx.check_crate_patterns();
    if diag.has_errors() {
        return Err("coloring/pattern checks reported errors".to_owned());
    }
    let ir = rurixc::device_codegen::build_and_emit(&cx, "saxpy")
        .ok_or_else(|| "no device IR (kernel codegen failed)".to_owned())?;
    if diag.has_errors() {
        return Err("device codegen reported errors".to_owned());
    }

    // IR → PTX(pin 的 clang NVPTX 后端,RXS-0070)
    let ptx = rurixc::toolchain::ir_to_ptx(&ir, ptx_out)?;

    // ptxas 干验证关卡(strict-only,RXS-0073);ptxas 缺失 → SKIP(关卡不阻断 build)
    match rurixc::ptxas::dry_gate(&ptx, "saxpy") {
        rurixc::ptxas::PtxasOutcome::Pass | rurixc::ptxas::PtxasOutcome::Skipped => {}
        rurixc::ptxas::PtxasOutcome::Rejected(reason) => {
            return Err(format!("ptxas -arch=sm_89 rejected generated PTX: {reason}"));
        }
        rurixc::ptxas::PtxasOutcome::Toolchain(e) => {
            return Err(format!("ptxas toolchain error: {e}"));
        }
    }

    parse_entry(&ir, &ptx).ok_or_else(|| "cannot locate ptx_kernel entry symbol".to_owned())
}

/// 从 NVPTX IR(`define ptx_kernel void @<name>(`)解析入口符号名,PTX
/// (`.entry <name>`)兜底核对。
fn parse_entry(ir: &str, ptx: &str) -> Option<String> {
    let from_ir = ir.find("define ptx_kernel").and_then(|_| {
        let at = ir.find(" @")?;
        let rest = &ir[at + 2..];
        let end = rest.find('(').unwrap_or(rest.len());
        let name = rest[..end].trim();
        (!name.is_empty()).then(|| name.to_owned())
    });
    if let Some(name) = from_ir {
        // PTX 兜底核对:.entry 后应含同名(clang 不改符号名)
        if ptx.contains(&name) {
            return Some(name);
        }
    }
    // PTX 直解析(`.entry <name>`)
    let idx = ptx.find(".entry")?;
    let rest = ptx[idx + ".entry".len()..].trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
        .collect();
    (!name.is_empty()).then_some(name)
}
