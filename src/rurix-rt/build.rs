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

/// 嵌入的 device kernel 列表(M4.4 SAXPY + M5.3 gpu 并行基元)。每项 (kernel 文件名
/// 干名 = module 名 = 常量前缀小写)产 `$OUT_DIR/{name}.ptx` + `{name}_meta.rs`
/// (常量 `{UPPER}_KERNEL` = ptx_kernel 入口符号名;降级时为空)。
const KERNELS: &[&str] = &["saxpy", "reduce", "scan", "transpose", "gemm_tile"];

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-env-changed=RURIXC_CLANG");
    println!("cargo:rerun-if-env-changed=CUDA_PATH");

    for name in KERNELS {
        let kernel_rx = manifest.join("kernels").join(format!("{name}.rx"));
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
                // 降级:空哨兵 PTX + 空入口名(bin/test 运行时据空 SKIP)
                println!(
                    "cargo:warning=rurix-rt: {name} device codegen unavailable, embedded PTX skipped ({reason})"
                );
                std::fs::write(&ptx_out, "")
                    .unwrap_or_else(|e| panic!("write sentinel {name}.ptx: {e}"));
                std::fs::write(&meta_out, format!("pub const {upper}_KERNEL: &str = \"\";\n"))
                    .unwrap_or_else(|e| panic!("write sentinel {name}_meta.rs: {e}"));
            }
        }
    }
}

/// `<name>.rx` → PTX(写 `ptx_out`;含 libdevice 链接 RXS-0082),返回 `ptx_kernel`
/// 入口符号名。
fn gen_ptx(
    kernel_rx: &std::path::Path,
    ptx_out: &std::path::Path,
    module: &str,
) -> Result<String, String> {
    let src = std::fs::read_to_string(kernel_rx)
        .map_err(|e| format!("cannot read {}: {e}", kernel_rx.display()))?;

    // 全量静态检查(typeck → 着色 → launch → 穷尽性 → views 不相交 → shared+barrier
    // 一致性),再 device codegen(kernel 为根)。M5.3 kernel 含 shared/2D/数学。
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

    // libdevice 链接裁决(RXS-0082):用到 `__nv_*` 但 bc 缺失 → 降级(Err → 哨兵)
    if matches!(
        rurixc::toolchain::libdevice_link_for(&ir),
        rurixc::toolchain::LibdeviceLink::MissingSkip
    ) {
        return Err("libdevice.10.bc not found (no CUDA toolchain)".to_owned());
    }
    // IR → PTX(pin 的 clang NVPTX 后端 + libdevice 链接,RXS-0070/0082)
    let ptx = rurixc::toolchain::ir_to_ptx(&ir, ptx_out)?;

    // ptxas 干验证关卡(strict-only,RXS-0073);ptxas 缺失 → SKIP(关卡不阻断 build)
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

/// 从 NVPTX IR(`define ptx_kernel void @<name>(`)解析入口符号名,PTX
/// (`.entry <name>`)兜底核对。
fn parse_entry(ir: &str, ptx: &str) -> Option<String> {
    let from_ir = ir.find("define ptx_kernel").and_then(|pos| {
        // 自 `define ptx_kernel` 处向后找 ` @<name>(`(否则会先命中 define 之前的
        // `declare i32 @llvm.nvvm.read.ptx.sreg.*()` intrinsic 声明而取错名)。
        let after_def = &ir[pos..];
        let at = after_def.find(" @")?;
        let rest = &after_def[at + 2..];
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
