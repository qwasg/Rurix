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
const KERNELS: &[&str] = &[
    "saxpy",
    "reduce",
    "scan",
    "transpose",
    "gemm_tile",
    // M7.3 G0 软光栅 kernel(binning/tile 光栅/深度/tonemap,全 safe,atomics-free;
    // spec/softraster.md RXS-0118~0121,D-M7-3)
    "sr_binning",
    "sr_raster_tile",
    "sr_depth",
    "sr_tonemap",
];

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-env-changed=RURIXC_CLANG");
    println!("cargo:rerun-if-env-changed=CUDA_PATH");
    // MR-0011(RD-027 护栏):compile_cubin 读 RURIXC_PTXAS_OPT 注入 ptxas -O<n> 旗标;
    // 缺此行则改 env 后 cargo 不重跑 build.rs → 旧 cubin 被复用 → 护栏静默失效。
    println!("cargo:rerun-if-env-changed=RURIXC_PTXAS_OPT");

    for name in KERNELS {
        let kernel_rx = manifest.join("kernels").join(format!("{name}.rx"));
        println!("cargo:rerun-if-changed={}", kernel_rx.display());
        let ptx_out = out_dir.join(format!("{name}.ptx"));
        let meta_out = out_dir.join(format!("{name}_meta.rs"));
        // 按架构预编 cubin(RXS-0150,G1.5;sm_89 基线)。始终写文件(空哨兵兜底)使
        // 消费侧 `include_bytes!` 在无 ptxas / 降级时也编译通过(保守 PTX fallback)。
        let cubin_out = out_dir.join(format!("{name}.{CUBIN_ARCH}.cubin"));
        let upper = name.to_uppercase();
        match gen_ptx(&kernel_rx, &ptx_out, name) {
            Ok(kernel) => {
                std::fs::write(
                    &meta_out,
                    format!("pub const {upper}_KERNEL: &str = \"{kernel}\";\n"),
                )
                .unwrap_or_else(|e| panic!("write {name}_meta.rs: {e}"));
                // PTX 已过 dry_gate(RXS-0073);按架构预编 cubin 并保留字节(脱离 PTX-only,
                // D-207)。无 ptxas / 拒绝 → 空 cubin 哨兵(降级仅 PTX fallback,保守兜底)。
                let cubin = gen_cubin(&ptx_out, name);
                std::fs::write(&cubin_out, &cubin)
                    .unwrap_or_else(|e| panic!("write {name}.{CUBIN_ARCH}.cubin: {e}"));
            }
            Err(reason) => {
                // 降级:空哨兵 PTX + 空入口名 + 空 cubin(bin/test 运行时据空 SKIP)
                println!(
                    "cargo:warning=rurix-rt: {name} device codegen unavailable, embedded PTX skipped ({reason})"
                );
                std::fs::write(&ptx_out, "")
                    .unwrap_or_else(|e| panic!("write sentinel {name}.ptx: {e}"));
                std::fs::write(
                    &meta_out,
                    format!("pub const {upper}_KERNEL: &str = \"\";\n"),
                )
                .unwrap_or_else(|e| panic!("write sentinel {name}_meta.rs: {e}"));
                std::fs::write(&cubin_out, b"")
                    .unwrap_or_else(|e| panic!("write sentinel {name}.{CUBIN_ARCH}.cubin: {e}"));
            }
        }
    }

    // RXS-0208 marshalling anchor 支撑:saxpy 经 vulkan_codegen(MIR→SPIR-V,feature
    // build-dep `vulkan-backend`)产**真** `.spv`,供 `vk.rs` 单测解析 `OpDecorate Binding`
    // / `OpMemberDecorate Offset` 装饰值,核对与运行时 descriptor-binding 构造序位**单一
    // 事实源**一致(codegen RXS-0203 IR ↔ vk.rs 运行时绑定,非各自约定的两份拷贝)。
    // 复现命令等价:`rurixc --target vulkan src/rurix-rt/kernels/saxpy.rx`。
    // 纯 Rust codegen(无外部工具)→ 始终产;检查失败/降级 → 空哨兵,vk 测试据空 SKIP。
    {
        let saxpy_rx = manifest.join("kernels").join("saxpy.rx");
        println!("cargo:rerun-if-changed={}", saxpy_rx.display());
        let spv_out = out_dir.join("saxpy.spv");
        let bytes = gen_spirv(&saxpy_rx).unwrap_or_default();
        std::fs::write(&spv_out, &bytes).unwrap_or_else(|e| panic!("write saxpy.spv: {e}"));
    }

    // mb1 W7 Android present demo:三角形 vertex/fragment 着色阶段经同一 `vulkan_codegen` 纯
    // Rust MIR→SPIR-V(graphics 阶段走 dxil_spirv::emit_spirv_body_vulkan,方案 B 去 provenance)
    // 产 `tri_vs.spv`/`tri_fs.spv` 嵌入 EXE/cdylib(`vk::demo_shaders_spv` 消费),复现等价
    // `rurixc --target vulkan conformance/vulkan/accept/vk_tri_{vs,fs}.rx`。**镜像 saxpy.spv 机制**
    // (同 gen_spirv 全静态检查 + build_and_emit_vulkan);build.rs 常在 host 跑,交叉构建 android
    // 时亦然,rurixc 解析与 target 无关(单一事实源)。降级 → 空哨兵,demo 据空 SKIP。
    let accept = manifest // src/rurix-rt → src → repo root → conformance/vulkan/accept
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repo root(CARGO_MANIFEST_DIR 上二级)")
        .join("conformance")
        .join("vulkan")
        .join("accept");
    for (rx_name, spv_name) in [
        ("vk_tri_vs.rx", "tri_vs.spv"),
        ("vk_tri_fs.rx", "tri_fs.spv"),
    ] {
        let rx = accept.join(rx_name);
        println!("cargo:rerun-if-changed={}", rx.display());
        let spv_out = out_dir.join(spv_name);
        let bytes = gen_spirv(&rx).unwrap_or_default();
        std::fs::write(&spv_out, &bytes).unwrap_or_else(|e| panic!("write {spv_name}: {e}"));
    }
}

/// `saxpy.rx` → Vulkan SPIR-V 字节(RXS-0208 anchor 支撑;`build_and_emit_vulkan` 纯 Rust
/// MIR→SPIR-V,无外部工具)。全静态检查失败 / 降级不可用 → `None`(消费侧据空 SKIP,对齐
/// PTX 降级纪律)。
fn gen_spirv(kernel_rx: &std::path::Path) -> Option<Vec<u8>> {
    let src = std::fs::read_to_string(kernel_rx).ok()?;
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if diag.has_errors() {
        return None;
    }
    cx.check_coloring();
    cx.check_launch();
    cx.check_crate_patterns();
    cx.check_views();
    cx.check_shared_barrier();
    if diag.has_errors() {
        return None;
    }
    let words = rurixc::vulkan_codegen::build_and_emit_vulkan(&cx, "saxpy")?;
    if diag.has_errors() {
        return None;
    }
    Some(rurixc::vulkan_codegen::words_to_bytes(&words))
}

/// 按架构预编 cubin 目标(基线 `sm_89`,07 §7 / D-207;多架构矩阵 defer RD-010)。
const CUBIN_ARCH: &str = "sm_89";

/// PTX → 按架构预编 cubin 字节(RXS-0150;`ptxas -arch=sm_89` 保留字节)。无 ptxas /
/// 空 PTX / 拒绝 → 空 cubin(降级仅 PTX fallback,保守兜底前向兼容,D-207;不阻断 build)。
///
/// MR-0011(RD-027 护栏):`RURIXC_PTXAS_OPT` 非法值 → 构建硬红(fail-closed,同 driver 预检;
/// 静默回落会让护栏假生效——用户显式设置了坏值,不应悄悄当没设)。
fn gen_cubin(ptx_out: &std::path::Path, module: &str) -> Vec<u8> {
    let Ok(ptx) = std::fs::read_to_string(ptx_out) else {
        return Vec::new();
    };
    if ptx.trim().is_empty() {
        return Vec::new();
    }
    // MR-0011(RD-027 护栏):非法值必须构建拒——与 driver.rs 预检同源 fail-closed。
    if let Err(e) = rurixc::ptxas::opt_flag_from_env(
        std::env::var("RURIXC_PTXAS_OPT").ok().as_deref(),
    ) {
        panic!("rurixc: error: {e}");
    }
    match rurixc::ptxas::compile_cubin(&ptx, module, CUBIN_ARCH) {
        rurixc::ptxas::CubinOutcome::Compiled(bytes) => bytes,
        // Skipped(无 ptxas)/ Rejected / Toolchain → 空 cubin 哨兵(降级仅 PTX fallback)。
        _ => Vec::new(),
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
