//! ptxas 干验证关卡(spec 条款 RXS-0073,spec/device.md;07 §7 strict-only,
//! 契约 G-M4-4)。
//!
//! device codegen 产出的 PTX **必须过 `ptxas -arch=sm_89` 干验证**(语法/语义
//! 校验,产 cubin 至临时文件后即弃,不留存);ptxas 拒绝(退出非零)→ 上层报
//! `RX6004`(对齐真跑铁律:注入非法 PTX / 破坏 codegen 必须红)。工具链定位经
//! 运行时探测(`RURIXC_PTXAS` > `CUDA_PATH\bin` > PATH,**禁硬编码版本文件名**,
//! r6/07 §10);ptxas 缺失(无 CUDA 工具链)→ [`PtxasOutcome::Skipped`](开发环境
//! 降级,真红绿在带 CUDA 的 CI runner)。非 ASCII 路径防御(ptxas 崩溃先例,r6):
//! 始终写入 ASCII 临时目录再交 ptxas。

use std::path::{Path, PathBuf};
use std::process::Command;

/// ptxas 干验证结果(RXS-0073)。
#[derive(Debug)]
pub enum PtxasOutcome {
    /// PTX 过 `ptxas -arch=sm_89` 干验证。
    Pass,
    /// ptxas 拒绝 PTX(携 stderr 摘要)→ 上层映射 `RX6004`。
    Rejected(String),
    /// 无 CUDA 工具链(ptxas 缺失):关卡 SKIP(开发环境降级)。
    Skipped,
    /// 工具链失败(ptxas 定位/spawn 失败)→ 上层映射 `RX7001`。
    Toolchain(String),
}

/// ptxas 定位(运行时探测;禁硬编码版本文件名,r6/07 §10)。
/// `RURIXC_PTXAS` > `CUDA_PATH\bin\ptxas.exe` > PATH `ptxas`;缺失 → None(SKIP)。
pub fn locate_ptxas() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("RURIXC_PTXAS") {
        candidates.push(PathBuf::from(p));
    }
    if let Ok(cuda) = std::env::var("CUDA_PATH") {
        candidates.push(PathBuf::from(cuda).join("bin").join("ptxas.exe"));
    }
    candidates.push(PathBuf::from("ptxas"));
    candidates.into_iter().find(|c| {
        Command::new(c)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

/// 对给定 PTX 文本跑 `ptxas -arch=sm_89` 干验证关卡(RXS-0073,G-M4-4)。
///
/// 始终写入 ASCII 临时目录(非 ASCII 路径防御);产 cubin 至临时文件后即删除
/// (strict-only,不留存)。`stem` 仅用于临时文件名(经 ASCII 清洗)。
pub fn dry_gate(ptx: &str, stem: &str) -> PtxasOutcome {
    let Some(ptxas) = locate_ptxas() else {
        return PtxasOutcome::Skipped;
    };
    let dir = PathBuf::from("C:\\Windows\\Temp").join("rurixc_ptx");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return PtxasOutcome::Toolchain(format!("cannot create ascii temp dir: {e}"));
    }
    let base = sanitize_ascii(stem);
    let ptx_path = dir.join(format!("{base}_{}.ptx", std::process::id()));
    if let Err(e) = std::fs::write(&ptx_path, ptx) {
        return PtxasOutcome::Toolchain(format!("cannot write ptx: {e}"));
    }
    let cubin = ptx_path.with_extension("cubin");
    let out = Command::new(&ptxas)
        .arg("-arch=sm_89")
        .arg(&ptx_path)
        .arg("-o")
        .arg(&cubin)
        .output();
    let _ = std::fs::remove_file(&cubin);
    let _ = std::fs::remove_file(&ptx_path);
    match out {
        Ok(o) if o.status.success() => PtxasOutcome::Pass,
        Ok(o) => {
            // ptxas 诊断可能落 stdout(非 stderr);两路合并取非空摘要(RX6004 输入)
            let stderr = String::from_utf8_lossy(&o.stderr);
            let stdout = String::from_utf8_lossy(&o.stdout);
            let reason = if stderr.trim().is_empty() {
                stdout.trim().to_owned()
            } else {
                stderr.trim().to_owned()
            };
            PtxasOutcome::Rejected(reason)
        }
        Err(e) => PtxasOutcome::Toolchain(format!("cannot spawn ptxas: {e}")),
    }
}

/// 按架构预编 cubin 结果(RXS-0150,G1.5/MR-0005)。
#[derive(Debug)]
pub enum CubinOutcome {
    /// 预编成功:cubin 字节(`cuModuleLoadData` 装载输入,首启免 JIT)。
    Compiled(Vec<u8>),
    /// ptxas 拒绝 PTX(携 stderr 摘要)。
    Rejected(String),
    /// 无 CUDA 工具链(ptxas 缺失):降级**仅 PTX fallback**(保守兜底,开发环境 / 无 ptxas)。
    Skipped,
    /// 工具链失败(ptxas 定位/spawn/IO 失败)。
    Toolchain(String),
}

/// 对给定 PTX 文本按架构 `ptxas -arch=<arch>` **预编并保留** cubin 字节(RXS-0150,G1.5)。
///
/// 区别于 [`dry_gate`](干验证后丢弃 cubin,RXS-0073):本函数**保留** cubin 用于「按架构预编
/// 分发」(脱离 M8 PTX-only,D-207);无 `ptxas` → [`CubinOutcome::Skipped`](降级仅 PTX
/// fallback,保守兜底前向兼容)。`arch` 形如 `"sm_89"`(基线);非 ASCII 路径防御同 [`dry_gate`]
/// (始终写 ASCII 临时目录)。预编 cubin 由 [`crate`] 嵌入产物并经 lockfile `[[artifact]]`
/// 内容寻址锁定(RXS-0152)。
pub fn compile_cubin(ptx: &str, stem: &str, arch: &str) -> CubinOutcome {
    let Some(ptxas) = locate_ptxas() else {
        return CubinOutcome::Skipped;
    };
    let dir = PathBuf::from("C:\\Windows\\Temp").join("rurixc_ptx");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return CubinOutcome::Toolchain(format!("cannot create ascii temp dir: {e}"));
    }
    let base = sanitize_ascii(stem);
    let arch_clean = sanitize_ascii(arch);
    let ptx_path = dir.join(format!("{base}_{arch_clean}_{}.ptx", std::process::id()));
    if let Err(e) = std::fs::write(&ptx_path, ptx) {
        return CubinOutcome::Toolchain(format!("cannot write ptx: {e}"));
    }
    let cubin = ptx_path.with_extension("cubin");
    let out = Command::new(&ptxas)
        .arg(format!("-arch={arch}"))
        .arg(&ptx_path)
        .arg("-o")
        .arg(&cubin)
        .output();
    let result = match out {
        Ok(o) if o.status.success() => match std::fs::read(&cubin) {
            Ok(bytes) if !bytes.is_empty() => CubinOutcome::Compiled(bytes),
            Ok(_) => CubinOutcome::Toolchain("ptxas produced empty cubin".to_owned()),
            Err(e) => CubinOutcome::Toolchain(format!("cannot read cubin: {e}")),
        },
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let stdout = String::from_utf8_lossy(&o.stdout);
            let reason = if stderr.trim().is_empty() {
                stdout.trim().to_owned()
            } else {
                stderr.trim().to_owned()
            };
            CubinOutcome::Rejected(reason)
        }
        Err(e) => CubinOutcome::Toolchain(format!("cannot spawn ptxas: {e}")),
    };
    let _ = std::fs::remove_file(&cubin);
    let _ = std::fs::remove_file(&ptx_path);
    result
}

/// 文件名 ASCII 清洗(非 ASCII 路径防御,RXS-0073)。
pub fn sanitize_ascii(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "kernel".to_owned()
    } else {
        cleaned
    }
}

/// `Path` 是否纯 ASCII(非 ASCII 时 ptxas 有崩溃先例,r6)。
pub fn is_ascii_path(p: &Path) -> bool {
    p.to_string_lossy().is_ascii()
}
