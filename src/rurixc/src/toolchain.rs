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

/// device NVPTX 约束 LLVM IR 文本 → PTX(clang NVPTX 后端;RXS-0070)。
///
/// 目标基线 compute_89/sm_89:nvptx 后端经 `-Xclang -target-cpu sm_89` 设 GPU
/// 架构(clang 驱动 nvptx target 不接受 `-mcpu=`);`+ptx78` 设 PTX ISA 版本
/// (sm_89 要求 ≥ 7.8;默认 4.2 不支持)。`-O2` 优化:NVPTX 后端 `-O0` 对 i64
/// 索引的 lowering 产出错误地址(`ld.local.b32` 入 64 位寄存器高位未定义 → 越界
/// 访存),且 device 代码须打满带宽(G-M4-1 ≥ 手写基线 95%);IR golden 在 IR 层
/// (CI_GATES §4.3),clang 优化级不影响 golden。中间 `.dev.ll` 落 `ptx_out` 同名
/// 旁路,返回 PTX 文本(失败 = 工具链错误串,上层映射 RX7001)。
pub fn ir_to_ptx(ir: &str, ptx_out: &Path) -> Result<String, String> {
    let clang = locate_clang()?;
    let ll = ptx_out.with_extension("dev.ll");
    std::fs::write(&ll, ir).map_err(|e| format!("cannot write {}: {e}", ll.display()))?;
    let out = Command::new(&clang)
        .arg("--target=nvptx64-nvidia-cuda")
        .arg("-Xclang")
        .arg("-target-cpu")
        .arg("-Xclang")
        .arg("sm_89")
        .arg("-Xclang")
        .arg("-target-feature")
        .arg("-Xclang")
        .arg("+ptx78")
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
