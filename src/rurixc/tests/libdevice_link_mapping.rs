//! libdevice 链接流程与 device 数学函数映射真跑测试(M5.3,RXS-0081/0082)。
//!
//! 覆盖三段(对齐 M5 契约 D-M5-4):
//! 1. **device 数学 intrinsic codegen**(环境无关):`f32` 数学方法 →
//!    保留的外部 libdevice 符号 `__nv_*` declare + call(RXS-0081);
//! 2. **NVVMReflect 精确路径留痕**:用到 libdevice 时发 `nvvm-reflect-ftz=0`
//!    模块 flag(RXS-0081/0082);
//! 3. **libdevice 链接 + ptxas 干验证**(需 clang 22.1.x + libdevice.10.bc +
//!    ptxas):`__nv_*` → clang `-mlink-builtin-bitcode` 链接 → PTX → ptxas 过
//!    (RXS-0082);任一工具缺失 → SKIP(开发环境降级,真实红绿在带 CUDA 的 runner)。

use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

/// device 数学 intrinsic 正例(方法形;`__nv_sqrtf`/`__nv_fmaf`/`__nv_expf`)。
const MATH_KERNEL: &str = r#"
device fn gaussian(x: f32, sigma: f32) -> f32 {
    let z = x / sigma;
    let e = z.fma(z, 0.0) * -0.5;
    e.exp()
}

device fn vlen(x: f32, y: f32) -> f32 {
    x.fma(x, y * y).sqrt()
}

kernel fn map_math(t: ThreadCtx<1>, src: View<global, f32>, dst: ViewMut<global, f32>, n: usize) {
    let i = t.global_id();
    if i < n {
        let v = src[i];
        dst[i] = gaussian(vlen(v, 1.0), 2.0);
    }
}

fn main() {}
"#;

/// 全管线产 device NVPTX IR(`kernel fn` 为根;断言 0 诊断)。
fn nvptx_ir(src: &str, module: &str) -> String {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    assert!(!diag.has_errors(), "typeck 应 0 诊断");
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(!diag.has_errors(), "前端检查应 0 诊断");
    let ir = rurixc::device_codegen::build_and_emit(&cx, module).expect("应产 device IR");
    assert!(!diag.has_errors(), "device codegen 应 0 诊断");
    ir
}

/// 段 1+2:device 数学 intrinsic → `__nv_*` 外部符号 + NVVMReflect 精确路径留痕。
#[test]
fn device_math_intrinsics_lower_to_nv_symbols() {
    let ir = nvptx_ir(MATH_KERNEL, "libdevice_math");
    for sym in ["@__nv_sqrtf(", "@__nv_fmaf(", "@__nv_expf("] {
        assert!(
            ir.contains(&format!("declare float {sym}")),
            "device 数学 intrinsic 应保留外部 declare `{sym}`(RXS-0081),IR:\n{ir}"
        );
        assert!(
            ir.contains(&format!("call float {sym}")),
            "device 数学 intrinsic 应 call `{sym}`(RXS-0081)"
        );
    }
    assert!(
        ir.contains("nvvm-reflect-ftz"),
        "用到 libdevice 时应发 NVVMReflect 精确路径模块 flag(RXS-0081/0082)"
    );
}

/// 段 3:libdevice 链接 + ptxas 干验证真跑(缺工具链 → SKIP)。
#[test]
fn libdevice_link_and_ptxas_gate() {
    let ir = nvptx_ir(MATH_KERNEL, "libdevice_math");

    // clang 缺失(或非 pin 22.1.x)→ SKIP(开发环境降级)。
    if rurixc::toolchain::locate_clang().is_err() {
        eprintln!("SKIP: clang 22.1.x 未就位(libdevice 链接真跑延到带工具链 runner)");
        return;
    }
    // libdevice bc 缺失 → SKIP(RXS-0082 开发环境降级,不报 RX7002)。
    if matches!(
        rurixc::toolchain::libdevice_link_for(&ir),
        rurixc::toolchain::LibdeviceLink::MissingSkip
    ) {
        eprintln!("SKIP: libdevice.10.bc 未就位(无 CUDA 工具链)");
        return;
    }

    // 链接 + IR→PTX:`__nv_*` 应被 libdevice 解析(链接产物不再含外部 __nv_* 调用)。
    let ptx_out: PathBuf = std::env::temp_dir().join("rurix_libdevice_link_test.ptx");
    let ptx = rurixc::toolchain::ir_to_ptx(&ir, &ptx_out)
        .expect("libdevice 链接 + IR→PTX 应成功(RXS-0082)");
    assert!(
        ptx.contains("sqrt.rn.f32"),
        "精确路径应产 sqrt.rn.f32(NVVMReflect prec-sqrt=1,RXS-0081),PTX 片段缺失"
    );
    assert!(
        !ptx.contains("__nv_sqrtf"),
        "libdevice 链接后不应残留未解析的外部 __nv_* 调用"
    );

    // ptxas 干验证(缺 ptxas → SKIP;拒绝 → 失败,对齐 RXS-0073 真跑铁律)。
    match rurixc::ptxas::dry_gate(&ptx, "libdevice_math") {
        rurixc::ptxas::PtxasOutcome::Pass => {}
        rurixc::ptxas::PtxasOutcome::Skipped => {
            eprintln!("SKIP: ptxas 未就位(libdevice 链接产物 ptxas 干验证延到带 CUDA runner)");
        }
        rurixc::ptxas::PtxasOutcome::Rejected(r) => {
            panic!("libdevice 链接产物被 ptxas 拒绝(RXS-0073/0082):{r}")
        }
        rurixc::ptxas::PtxasOutcome::Toolchain(e) => {
            eprintln!("SKIP: ptxas 工具链问题:{e}");
        }
    }
    let _ = std::fs::remove_file(&ptx_out);
    let _ = Path::new("");
}

const F64_MATH: &str = r#"
kernel fn sqrt_f64(t: ThreadCtx<1>, src: View<global, f64>, dst: ViewMut<global, f64>, n: usize) {
    let i = t.global_id();
    if i < n {
        dst[i] = src[i].sqrt();
    }
}
fn main() {}
"#;

#[test]
fn device_math_f64_lowers_to_nv_sqrt() {
    let ir = nvptx_ir(F64_MATH, "libdevice_f64");
    assert!(
        ir.contains("@__nv_sqrt(") || ir.contains("declare double @__nv_sqrt"),
        "f64 sqrt 应保留 __nv_sqrt(RXS-0081),IR:\n{ir}"
    );
}

//@ spec: RXS-0081
//@ spec: RXS-0082
#[test]
fn device_math_libdevice_clauses_anchored() {
    // 锚定占位:RXS-0081/0082 由本文件上方真跑测试覆盖(traceability)。
}

/// 位扫描/位计数 intrinsic 正例(MR-0006,RXS-0183;NVPTX 双后端同落面)。
const BIT_KERNEL: &str = r#"
kernel fn bit_scan(t: ThreadCtx<1>, words: View<global, u32>, dst: ViewMut<global, u32>, n: usize) {
    let i = t.global_id();
    if i < n {
        let w = words[i];
        dst[i] = w.find_lsb() + w.find_msb() + w.popcount();
    }
}
fn main() {}
"#;

/// MR-0006(RXS-0183):位扫描/位计数 intrinsic NVPTX 下译 = libdevice 符号组合
/// (`__nv_ffs - 1` / `31 - __nv_clz` / `__nv_popc`;零输入回绕即 HLSL 形
/// 0xFFFF_FFFF,与 DXIL 路 FirstbitLo/FirstbitHi 正规化语义一致,判档 O-1)。
//@ spec: RXS-0183
#[test]
fn device_bit_intrinsics_lower_to_nv_symbols() {
    let ir = nvptx_ir(BIT_KERNEL, "libdevice_bits");
    for sym in ["@__nv_ffs(", "@__nv_clz(", "@__nv_popc("] {
        assert!(
            ir.contains(&format!("declare i32 {sym}")),
            "位 intrinsic 应保留外部 declare `{sym}`(RXS-0183),IR:\n{ir}"
        );
        assert!(
            ir.contains(&format!("call i32 {sym}")),
            "位 intrinsic 应 call `{sym}`(RXS-0183)"
        );
    }
    // find_lsb = ffs - 1;find_msb = 31 - clz(组合 sub 存在性)。
    assert!(
        ir.contains("sub i32 31,"),
        "find_msb 应 emit `31 - clz` 组合(RXS-0183),IR:\n{ir}"
    );
}

/// MR-0006(RXS-0183):位 intrinsic libdevice 链接 + ptxas 干验证真跑
/// (`__nv_ffs`/`__nv_clz`/`__nv_popc` 经 libdevice.bc 解析;缺工具链 → SKIP,
/// 对齐 RXS-0082 开发环境降级纪律)。
//@ spec: RXS-0183
#[test]
fn device_bit_intrinsics_link_and_ptxas_gate() {
    let ir = nvptx_ir(BIT_KERNEL, "libdevice_bits");
    if rurixc::toolchain::locate_clang().is_err() {
        eprintln!("SKIP: clang 22.1.x 未就位(位 intrinsic libdevice 链接真跑延到带工具链 runner)");
        return;
    }
    if matches!(
        rurixc::toolchain::libdevice_link_for(&ir),
        rurixc::toolchain::LibdeviceLink::MissingSkip
    ) {
        eprintln!("SKIP: libdevice.10.bc 未就位(无 CUDA 工具链)");
        return;
    }
    let ptx_out: PathBuf = std::env::temp_dir().join("rurix_libdevice_bits_test.ptx");
    let ptx = rurixc::toolchain::ir_to_ptx(&ir, &ptx_out)
        .expect("位 intrinsic libdevice 链接 + IR→PTX 应成功(RXS-0183/0082)");
    for sym in ["__nv_ffs", "__nv_clz", "__nv_popc"] {
        assert!(
            !ptx.contains(sym),
            "libdevice 链接后不应残留未解析的外部 {sym} 调用"
        );
    }
    assert!(
        ptx.contains("popc.b32"),
        "popcount 链接产物应含 PTX popc.b32(RXS-0183),PTX 片段缺失"
    );
    match rurixc::ptxas::dry_gate(&ptx, "libdevice_bits") {
        rurixc::ptxas::PtxasOutcome::Pass => {}
        rurixc::ptxas::PtxasOutcome::Skipped => {
            eprintln!("SKIP: ptxas 未就位(位 intrinsic ptxas 干验证延到带 CUDA runner)");
        }
        rurixc::ptxas::PtxasOutcome::Rejected(r) => {
            panic!("位 intrinsic 链接产物被 ptxas 拒绝(RXS-0073/0183):{r}")
        }
        rurixc::ptxas::PtxasOutcome::Toolchain(e) => {
            eprintln!("SKIP: ptxas 工具链问题:{e}");
        }
    }
    let _ = std::fs::remove_file(&ptx_out);
}

/// 移位量取模掩码 kernel(MR-0006,RXS-0182 / 判档 O-2 双后端显式掩码)。
const SHIFT_KERNEL: &str = r#"
kernel fn shift_mask(t: ThreadCtx<1>, words: View<global, u32>, dst: ViewMut<global, u32>, n: usize) {
    let i = t.global_id();
    if i < n {
        let w = words[i];
        dst[i] = (w << 3u32) >> 1u32;
    }
}
fn main() {}
"#;

/// MR-0006(RXS-0182):NVPTX 侧移位量按位宽取模——device codegen emit 显式
/// 掩码 `and i32 amount, 31`(消除 LLVM 越界 poison;u32 右移取 lshr)。
//@ spec: RXS-0182
#[test]
fn device_shift_amount_masked_on_nvptx() {
    let ir = nvptx_ir(SHIFT_KERNEL, "nvptx_shift_mask");
    assert!(
        ir.contains("and i32 3, 31") || ir.matches("and i32").count() >= 2,
        "移位量应 emit 显式 `& 31` 掩码(RXS-0182/O-2),IR:\n{ir}"
    );
    for inst in ["shl i32", "lshr i32"] {
        assert!(ir.contains(inst), "u32 移位应产 {inst}(RXS-0182),IR:\n{ir}");
    }
}
