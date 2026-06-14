//! 工具链定位与 device IR→PTX(M4.2 抽出复用,M4.4;clang pin 22.1.x,D-205)。
//!
//! 驱动 `--emit=ptx`(bin)与 `rurix-rt` 的 `build.rs`(嵌入 PTX 单产物)复用同一
//! IR→PTX 路径(单一事实源):NVPTX 约束 LLVM IR 文本 → pin 的 clang
//! `--target=nvptx64-nvidia-cuda` NVPTX 后端 → PTX。ptxas 干验证关卡见
//! [`crate::ptxas`](strict-only,RXS-0073)。

use std::path::{Path, PathBuf};
use std::process::Command;

/// clang 定位 + pin 22.1.x 断言(D-205;M2_PLAN v1.3 选型留痕)。
///
/// `RURIXC_CLANG` 环境变量 > `C:\Program Files\LLVM\bin\clang.exe` > PATH;
/// 版本非 22.1.x(违例 = pin 纪律,上层映射 RX7001)。
pub fn locate_clang() -> Result<PathBuf, String> {
    let candidates: Vec<PathBuf> = [
        std::env::var("RURIXC_CLANG").ok(),
        Some("C:\\Program Files\\LLVM\\bin\\clang.exe".to_owned()),
        Some("clang".to_owned()),
    ]
    .into_iter()
    .flatten()
    .map(PathBuf::from)
    .collect();
    for c in candidates {
        let Ok(out) = Command::new(&c).arg("--version").output() else {
            continue;
        };
        if !out.status.success() {
            continue;
        }
        let ver = String::from_utf8_lossy(&out.stdout);
        if ver.contains("clang version 22.1.") {
            return Ok(c);
        }
        return Err(format!(
            "clang at {} is not the pinned 22.1.x (D-205): {}",
            c.display(),
            ver.lines().next().unwrap_or("")
        ));
    }
    Err("clang not found (install LLVM 22.1.x or set RURIXC_CLANG)".to_owned())
}

/// libdevice 链接裁决(M5.3,RXS-0082):IR 是否用到 libdevice `__nv_*` 数学
/// 符号 + bc 是否可定位。
pub enum LibdeviceLink {
    /// IR 无 `__nv_*` 符号引用:按原路径直接 IR→PTX(无需 libdevice)。
    NotNeeded,
    /// 用到 libdevice 且已定位 `libdevice.10.bc`(链 bc → internalize → DCE →
    /// NVVMReflect,clang `-mlink-builtin-bitcode` 内置流程)。
    Linked(PathBuf),
    /// 用到 libdevice 但 bc 缺失(无 CUDA 工具链):开发环境降级 SKIP(真实红绿
    /// 在带 CUDA 的 CI runner,RXS-0082;不报 RX7002)。
    MissingSkip,
}

/// IR 是否引用 libdevice `__nv_*` 数学符号(device_codegen 保留为外部 declare)。
pub fn ir_needs_libdevice(ir: &str) -> bool {
    ir.contains("@__nv_")
}

/// 定位 `libdevice.10.bc`(RXS-0082;禁硬编码版本路径,沿用 ptxas 定位纪律 r6):
/// `RURIXC_LIBDEVICE` > `CUDA_PATH\nvvm\libdevice\libdevice.10.bc`。
pub fn locate_libdevice() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RURIXC_LIBDEVICE") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    if let Ok(cuda) = std::env::var("CUDA_PATH") {
        let pb = PathBuf::from(cuda)
            .join("nvvm")
            .join("libdevice")
            .join("libdevice.10.bc");
        if pb.is_file() {
            return Some(pb);
        }
    }
    None
}

/// 对给定 IR 裁决 libdevice 链接路径(RXS-0082)。
pub fn libdevice_link_for(ir: &str) -> LibdeviceLink {
    if !ir_needs_libdevice(ir) {
        return LibdeviceLink::NotNeeded;
    }
    match locate_libdevice() {
        Some(bc) => LibdeviceLink::Linked(bc),
        None => LibdeviceLink::MissingSkip,
    }
}

/// device NVPTX 约束 LLVM IR 文本 → PTX(clang NVPTX 后端;RXS-0070;libdevice
/// 链接 RXS-0082)。
///
/// 目标基线 compute_89/sm_89:nvptx 后端经 `-Xclang -target-cpu sm_89` 设 GPU
/// 架构(clang 驱动 nvptx target 不接受 `-mcpu=`);`+ptx78` 设 PTX ISA 版本
/// (sm_89 要求 ≥ 7.8;默认 4.2 不支持)。`-O2` 优化:NVPTX 后端 `-O0` 对 i64
/// 索引的 lowering 产出错误地址(`ld.local.b32` 入 64 位寄存器高位未定义 → 越界
/// 访存),且 device 代码须打满带宽(G-M4-1 ≥ 手写基线 95%);IR golden 在 IR 层
/// (CI_GATES §4.3),clang 优化级不影响 golden。
///
/// **libdevice 链接(RXS-0082)**:IR 用到 `__nv_*` 数学符号且 bc 可定位时,经
/// clang `-mlink-builtin-bitcode <libdevice.10.bc>` 链接(clang NVPTX 后端内置
/// internalize/DCE/NVVMReflect 流程,精确路径由 IR 的 `nvvm-reflect-ftz=0` 模块
/// flag 留痕)。bc 缺失(`MissingSkip`)应由调用方先行 SKIP,不应进入本函数。
///
/// 中间 `.dev.ll` 落 `ptx_out` 同名旁路,返回 PTX 文本(失败 = 工具链错误串,
/// 上层映射 RX7001;libdevice 链接语境失败映射 RX7002)。
pub fn ir_to_ptx(ir: &str, ptx_out: &Path) -> Result<String, String> {
    let clang = locate_clang()?;
    let ll = ptx_out.with_extension("dev.ll");
    std::fs::write(&ll, ir).map_err(|e| format!("cannot write {}: {e}", ll.display()))?;
    let mut cmd = Command::new(&clang);
    cmd.arg("--target=nvptx64-nvidia-cuda")
        .arg("-Xclang")
        .arg("-target-cpu")
        .arg("-Xclang")
        .arg("sm_89")
        .arg("-Xclang")
        .arg("-target-feature")
        .arg("-Xclang")
        .arg("+ptx78");
    // libdevice 链接(RXS-0082):保留外部 `__nv_*` 符号 → 链 libdevice bc →
    // internalize → DCE → NVVMReflect(clang 内置流程)。
    if let LibdeviceLink::Linked(bc) = libdevice_link_for(ir) {
        cmd.arg("-Xclang")
            .arg("-mlink-builtin-bitcode")
            .arg("-Xclang")
            .arg(&bc)
            // NVVMReflect 裁决(RXS-0081 默认精确路径):ftz=0(模块 flag 已留痕)
            // + prec-sqrt=1 / prec-div=1 经 `-mllvm -nvvm-reflect-add` 显式置值
            // (模块 flag 仅 ftz 被 NVVMReflect 识别,prec-* 须经 reflect-add)。
            .arg("-mllvm")
            .arg("-nvvm-reflect-add=__CUDA_PREC_SQRT=1")
            .arg("-mllvm")
            .arg("-nvvm-reflect-add=__CUDA_PREC_DIV=1");
    }
    let out = cmd
        .arg("-O2")
        .arg("-S")
        .arg(&ll)
        .arg("-o")
        .arg(ptx_out)
        .output();
    match out {
        Ok(o) if o.status.success() => std::fs::read_to_string(ptx_out)
            .map_err(|e| format!("cannot read {}: {e}", ptx_out.display())),
        Ok(o) => Err(format!(
            "clang (nvptx) exited with {}: {}{}",
            o.status,
            String::from_utf8_lossy(&o.stdout).trim(),
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("cannot spawn clang (nvptx): {e}")),
    }
}
