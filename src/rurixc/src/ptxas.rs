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

/// ASCII 临时目录选路(纯函数,可测):`%TEMP%` 纯 ASCII → 直用其下 `rurixc_ptx`
/// (本就按用户隔离);否则(如中文用户名的 profile 路径)落
/// `C:\Windows\Temp\rurixc_ptx_<userprofile FNV-1a 哈希 8 hex>`——仍 ASCII、
/// **按用户唯一**、同用户重入稳定。
///
/// 按用户唯一是硬要求:曾用跨用户共享的固定 `C:\Windows\Temp\rurixc_ptx`,用户 A
/// 创建后其 creator-owner ACL 挡住用户 B 的 `create_dir_all` 存在性探测,盲创建
/// 撞 `ERROR_ALREADY_EXISTS`(os error 183)——EA1 冷启动 B 段干净账户实测抓获。
fn ascii_temp_dir_for(std_tmp: &Path, userprofile: &str) -> PathBuf {
    if std_tmp.to_str().is_some_and(|s| s.is_ascii()) {
        return std_tmp.join("rurixc_ptx");
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in userprofile.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let tag = ((h >> 32) as u32) ^ (h as u32);
    PathBuf::from("C:\\Windows\\Temp").join(format!("rurixc_ptx_{tag:08x}"))
}

/// 建 ASCII 临时目录(幂等:`AlreadyExists` 视同成功)。失败 → `Err(报文)`(上层
/// 映射 RX7001,语义不变)。
fn ensure_ascii_temp_dir() -> Result<PathBuf, String> {
    let dir = ascii_temp_dir_for(
        &std::env::temp_dir(),
        &std::env::var("USERPROFILE").unwrap_or_default(),
    );
    match std::fs::create_dir_all(&dir) {
        Ok(()) => Ok(dir),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(dir),
        Err(e) => Err(format!("cannot create ascii temp dir: {e}")),
    }
}

/// 对给定 PTX 文本跑 `ptxas -arch=sm_89` 干验证关卡(RXS-0073,G-M4-4)。
///
/// 始终写入 ASCII 临时目录(非 ASCII 路径防御);产 cubin 至临时文件后即删除
/// (strict-only,不留存)。`stem` 仅用于临时文件名(经 ASCII 清洗)。
pub fn dry_gate(ptx: &str, stem: &str) -> PtxasOutcome {
    let Some(ptxas) = locate_ptxas() else {
        return PtxasOutcome::Skipped;
    };
    let dir = match ensure_ascii_temp_dir() {
        Ok(d) => d,
        Err(e) => return PtxasOutcome::Toolchain(e),
    };
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

/// `RURIXC_PTXAS_OPT` 环境开关 → ptxas `-O<n>` 旗标(MR-0011,RD-027 护栏)。
///
/// 纯函数(可测):`None` = 不注入(ptxas 默认 -O3,行为 0-byte);`Some("0"~"3")` =
/// 注入对应 `-O<n>`;其余值 = 确定性拒(工具层报文,不静默回落——RD-027 spike 实证
/// `-O0` 下毒径构型正确终止而 `-O1+` SASS 死锁,误写档位静默回默认会让护栏假生效)。
pub fn opt_flag_from_env(val: Option<&str>) -> Result<Option<String>, String> {
    match val {
        None => Ok(None),
        // 空串/纯空白 = 视同未设(MR-0011 §7.1 F1:`RURIXC_PTXAS_OPT=` 置空是
        // CI/shell 常见「清空」写法,语义归「用默认」而非「误写」)。
        Some(v) if v.trim().is_empty() => Ok(None),
        Some(v) if matches!(v, "0" | "1" | "2" | "3") => Ok(Some(format!("-O{v}"))),
        Some(v) => Err(format!(
            "RURIXC_PTXAS_OPT must be one of 0|1|2|3, got {v:?} (RD-027 guardrail; \
             refusing to silently fall back to default opt level)"
        )),
    }
}

/// 对给定 PTX 文本按架构 `ptxas -arch=<arch>` **预编并保留** cubin 字节(RXS-0150,G1.5)。
///
/// 区别于 [`dry_gate`](干验证后丢弃 cubin,RXS-0073):本函数**保留** cubin 用于「按架构预编
/// 分发」(脱离 M8 PTX-only,D-207);无 `ptxas` → [`CubinOutcome::Skipped`](降级仅 PTX
/// fallback,保守兜底前向兼容)。`arch` 形如 `"sm_89"`(基线);非 ASCII 路径防御同 [`dry_gate`]
/// (始终写 ASCII 临时目录)。预编 cubin 由 [`crate`] 嵌入产物并经 lockfile `[[artifact]]`
/// 内容寻址锁定(RXS-0152)。
///
/// `RURIXC_PTXAS_OPT`(MR-0011,RD-027 护栏):见 [`opt_flag_from_env`];缺省不注入 0-byte。
pub fn compile_cubin(ptx: &str, stem: &str, arch: &str) -> CubinOutcome {
    let opt_flag = match opt_flag_from_env(std::env::var("RURIXC_PTXAS_OPT").ok().as_deref()) {
        Ok(f) => f,
        Err(e) => return CubinOutcome::Toolchain(e),
    };
    let Some(ptxas) = locate_ptxas() else {
        return CubinOutcome::Skipped;
    };
    let dir = match ensure_ascii_temp_dir() {
        Ok(d) => d,
        Err(e) => return CubinOutcome::Toolchain(e),
    };
    let base = sanitize_ascii(stem);
    let arch_clean = sanitize_ascii(arch);
    let ptx_path = dir.join(format!("{base}_{arch_clean}_{}.ptx", std::process::id()));
    if let Err(e) = std::fs::write(&ptx_path, ptx) {
        return CubinOutcome::Toolchain(format!("cannot write ptx: {e}"));
    }
    let cubin = ptx_path.with_extension("cubin");
    let mut cmd = Command::new(&ptxas);
    if let Some(flag) = &opt_flag {
        cmd.arg(flag);
    }
    let out = cmd
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

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0073
    #[test]
    fn ascii_temp_dir_uses_per_user_temp_when_ascii() {
        let d = ascii_temp_dir_for(
            Path::new(r"C:\Users\rurixtest\AppData\Local\Temp"),
            r"C:\Users\rurixtest",
        );
        assert_eq!(
            d,
            PathBuf::from(r"C:\Users\rurixtest\AppData\Local\Temp\rurixc_ptx")
        );
        assert!(d.to_str().unwrap().is_ascii());
    }

    //@ spec: RXS-0073
    #[test]
    fn ascii_temp_dir_hashes_per_user_when_profile_non_ascii() {
        let tmp = Path::new(r"C:\Users\苍\AppData\Local\Temp");
        let a = ascii_temp_dir_for(tmp, r"C:\Users\苍");
        let b = ascii_temp_dir_for(tmp, r"C:\Users\苍");
        let other = ascii_temp_dir_for(tmp, r"C:\Users\别人");
        // ASCII、同用户稳定、异用户不同名(修 os error 183 跨用户 ACL 碰撞的关键不变量)。
        assert!(a.to_str().unwrap().is_ascii());
        assert!(
            a.to_str()
                .unwrap()
                .starts_with(r"C:\Windows\Temp\rurixc_ptx_")
        );
        assert_eq!(a, b);
        assert_ne!(a, other);
    }

    // MR-0011(RD-027 护栏):RURIXC_PTXAS_OPT → -O 旗标注入,非法值确定性拒。
    #[test]
    fn opt_flag_accepts_levels_0_to_3_and_defaults_to_none() {
        assert_eq!(opt_flag_from_env(None).unwrap(), None);
        // 空串/纯空白 = 视同未设(评审 F1:CI 置空写法不应硬红)。
        assert_eq!(opt_flag_from_env(Some("")).unwrap(), None);
        assert_eq!(opt_flag_from_env(Some("  ")).unwrap(), None);
        for (v, want) in [("0", "-O0"), ("1", "-O1"), ("2", "-O2"), ("3", "-O3")] {
            assert_eq!(opt_flag_from_env(Some(v)).unwrap().as_deref(), Some(want));
        }
    }

    // MR-0011(RD-027 护栏):误写档位不得静默回落默认(护栏假生效即毒径复挂)。
    #[test]
    fn opt_flag_rejects_invalid_levels_deterministically() {
        for bad in ["4", "-1", "O0", "fast", "00"] {
            let err = opt_flag_from_env(Some(bad)).unwrap_err();
            assert!(
                err.contains("RURIXC_PTXAS_OPT"),
                "err carries env name: {err}"
            );
            assert!(
                err.contains("RD-027"),
                "err carries guardrail anchor: {err}"
            );
        }
    }

    //@ spec: RXS-0073
    #[test]
    fn ensure_ascii_temp_dir_is_idempotent() {
        let first = ensure_ascii_temp_dir().expect("first create");
        let second = ensure_ascii_temp_dir().expect("second create tolerates existing");
        assert_eq!(first, second);
        assert!(first.is_dir());
    }
}
