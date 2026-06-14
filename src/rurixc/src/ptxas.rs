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
