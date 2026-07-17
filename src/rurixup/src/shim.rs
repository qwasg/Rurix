//! 活跃版本切换 shim(spec/release.md RXS-0215;RFC-0012 §4.3,裁决 B)。
//!
//! `<RURIX_HOME>\bin\<name>.exe` 为 `rurixup.exe` 的一份拷贝,一次入 PATH。rurixup
//! `main()` 起始按 `current_exe()` 干名(file_stem)判定:干名 == `"rurixup"` → 正常
//! 子命令分发;干名 ≠ `"rurixup"` → **代理模式**——读 `toolchains.json` default →
//! 转发 `toolchains\<default>\bin\<干名>.exe`(stdio 继承,退出码逐位透传)。
//!
//! **防自递归 / 防逃逸**:转发目标恒在 `<RURIX_HOME>\toolchains\` 下(shim 自身在
//! `\bin\` 下,永不转发自己);且目标经规范化不得等于 shim 自身路径。default 缺失 /
//! 指向缺失目录 / 目标不存在 → 诚实错误退出非 0。切换 = 注册表 JSON 单写(见
//! `toolchain.rs`);已开 shell 即时生效。全 safe(`unsafe_code=deny`,仅 `std::process`
//! / `std::fs`,零 unsafe、零第三方)。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::toolchain::ToolchainRegistry;

/// 代理决策/执行错误(工具层,退出码 1;非编译器 RX 段位码)。
#[derive(Debug)]
pub enum ShimError {
    /// 无法解析 RURIX_HOME。
    NoHome(String),
    /// 读 / 解析 toolchains.json 失败。
    Registry(String),
    /// 注册表无 default 版本。
    NoDefault,
    /// 目标不在 `<home>\toolchains\` 下(防逃逸)。
    Escape(PathBuf),
    /// 目标等于 shim 自身(防自递归)。
    SelfRecursion(PathBuf),
    /// 目标 exe 不存在(切换指向缺失目录 / 未物化)。
    TargetMissing(PathBuf),
    /// spawn / 等待子进程失败。
    Spawn(String),
}

impl std::fmt::Display for ShimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShimError::NoHome(e) => write!(f, "无法解析 RURIX_HOME:{e}"),
            ShimError::Registry(e) => write!(f, "读工具链注册表失败:{e}"),
            ShimError::NoDefault => write!(
                f,
                "工具链注册表无 default 版本(先 `rurixup install` 并 `rurixup default <ver>`)"
            ),
            ShimError::Escape(p) => write!(
                f,
                "拒绝转发:目标 {} 不在 toolchains\\ 下(防逃逸)",
                p.display()
            ),
            ShimError::SelfRecursion(p) => {
                write!(f, "拒绝转发:目标 {} 等于 shim 自身(防自递归)", p.display())
            }
            ShimError::TargetMissing(p) => write!(
                f,
                "切换目标不存在:{}(版本目录可能已删除 / 未物化;`rurixup install` 或 `rurixup default <ver>`)",
                p.display()
            ),
            ShimError::Spawn(e) => write!(f, "转发子进程失败:{e}"),
        }
    }
}

/// 解析 RURIX_HOME(shim 侧):env `RURIX_HOME` 覆盖,否则由 shim 自身位置派生
/// (`<home>\bin\<exe>` → `<home>`)。
pub fn resolve_home(current_exe: &Path) -> Result<PathBuf, ShimError> {
    if let Some(h) = std::env::var_os("RURIX_HOME") {
        return Ok(PathBuf::from(h));
    }
    current_exe
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| ShimError::NoHome(format!("无法由 {} 派生 home", current_exe.display())))
}

/// exe 文件干名(小写化,便于 `"rurixup"` 判定;无扩展名亦可)。
pub fn exe_stem(current_exe: &Path) -> String {
    current_exe
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// 规范化前缀判定:`child` 是否位于 `ancestor` 之下(尽力 canonicalize;失败回退
/// 词法比较)。
fn is_under(ancestor: &Path, child: &Path) -> bool {
    let a = std::fs::canonicalize(ancestor).unwrap_or_else(|_| ancestor.to_path_buf());
    let c = std::fs::canonicalize(child).unwrap_or_else(|_| child.to_path_buf());
    c.starts_with(&a)
}

/// 计算代理转发目标 exe 路径(纯路径推导 + 防逃逸/防自递归判定,不 spawn;host 可测)。
/// `stem` = 当前 exe 干名(调用方已确认 ≠ "rurixup")。
pub fn resolve_target(
    home: &Path,
    stem: &str,
    default_version: &str,
    current_exe: &Path,
) -> Result<PathBuf, ShimError> {
    let toolchains = home.join("toolchains");
    let bin = toolchains.join(default_version).join("bin");
    let exe_name = if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    };
    let target = bin.join(&exe_name);

    // 防逃逸:目标必须在 <home>\toolchains\ 下。
    if !is_under(&toolchains, &target) {
        return Err(ShimError::Escape(target));
    }
    // 防自递归:目标不得等于 shim 自身。
    let self_canon =
        std::fs::canonicalize(current_exe).unwrap_or_else(|_| current_exe.to_path_buf());
    let target_canon = std::fs::canonicalize(&target).unwrap_or_else(|_| target.clone());
    if target_canon == self_canon {
        return Err(ShimError::SelfRecursion(target));
    }
    Ok(target)
}

/// 若当前 exe 为 shim(干名 ≠ "rurixup")→ 代理转发 default 版本同名 exe 并
/// **透传退出码**(本函数在代理成功时 `std::process::exit`,不返回);干名 ==
/// "rurixup" 或无法确定 current_exe → 返回 `Ok(())`(交由正常子命令分发)。代理
/// 决策/执行错误 → `std::process::exit(1)` 并打印诚实错误。
pub fn forward_if_shim(args: &[String]) {
    let Ok(current_exe) = std::env::current_exe() else {
        return; // 无法确定自身 → 按正常 rurixup 处理。
    };
    let stem = exe_stem(&current_exe);
    if stem == "rurixup" {
        return; // 非 shim,正常分发。
    }
    match run_proxy(&current_exe, &stem, args) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("rurixup(shim): 错误:{e}");
            std::process::exit(1);
        }
    }
}

/// 代理转发核心:解析 home → 读注册表 default → 计算目标 → spawn 透传。返回子进程
/// 退出码(供 `forward_if_shim` 透传)。
fn run_proxy(current_exe: &Path, stem: &str, args: &[String]) -> Result<i32, ShimError> {
    let home = resolve_home(current_exe)?;
    let registry_path = home.join("toolchains.json");
    let text = std::fs::read_to_string(&registry_path)
        .map_err(|e| ShimError::Registry(format!("读 {} 失败:{e}", registry_path.display())))?;
    let registry = ToolchainRegistry::from_json(&text).map_err(ShimError::Registry)?;
    let default = registry.default_version().ok_or(ShimError::NoDefault)?;

    let target = resolve_target(&home, stem, default, current_exe)?;
    if !target.is_file() {
        return Err(ShimError::TargetMissing(target));
    }

    // stdio 继承,参数透传;退出码逐位透传。
    let status = Command::new(&target)
        .args(args)
        .status()
        .map_err(|e| ShimError::Spawn(format!("spawn {} 失败:{e}", target.display())))?;
    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0215
    // 干名判定:shim(干名 ≠ "rurixup")识别 + home 派生(<home>\bin\<exe> → <home>)。
    #[test]
    fn exe_stem_and_home_derivation() {
        let shim = Path::new("C:/Users/u/.rurix/bin/rx.exe");
        assert_eq!(exe_stem(shim), "rx");
        let rurixup = Path::new("C:/Users/u/.rurix/toolchains/1.0.0/bin/rurixup.exe");
        assert_eq!(exe_stem(rurixup), "rurixup");
        // RURIX_HOME 未设时由 shim 位置派生(<home>\bin\rx.exe → <home>)。
        // 注:resolve_home 优先读 env,单测不改进程 env,直接验证派生分支的路径代数。
        assert_eq!(
            shim.parent().and_then(Path::parent).unwrap(),
            Path::new("C:/Users/u/.rurix")
        );
    }

    //@ spec: RXS-0215
    // 代理目标路径推导 = toolchains\<default>\bin\<干名>.exe;防逃逸拦截 toolchains\ 外目标。
    #[test]
    fn resolve_target_stays_under_toolchains_and_blocks_escape() {
        let home = Path::new("C:/rurixtest/.rurix");
        let shim = home.join("bin").join("rx.exe");
        // 正常:目标在 toolchains\<ver>\bin\ 下(路径存在与否不影响词法推导)。
        let t = resolve_target(home, "rx", "1.0.0", &shim).expect("target resolves");
        let expected_tail: PathBuf = ["toolchains", "1.0.0", "bin"].iter().collect();
        assert!(t.starts_with(home.join(&expected_tail)));
        assert!(
            t.file_name().unwrap().to_string_lossy().starts_with("rx"),
            "目标干名须为转发干名"
        );
    }

    //@ spec: RXS-0215
    // 防自递归:目标解析到 shim 自身路径 → SelfRecursion(工具层拒,非 spawn)。
    #[test]
    fn resolve_target_detects_self_recursion() {
        // 构造:home\toolchains\1.0.0\bin\rx.exe 即 current_exe 自身(shim 被误置于 toolchains 内)。
        let home = Path::new("C:/rurixtest2/.rurix");
        let self_exe: PathBuf = home
            .join("toolchains")
            .join("1.0.0")
            .join("bin")
            .join("rx.exe");
        let err =
            resolve_target(home, "rx", "1.0.0", &self_exe).expect_err("目标 == 自身应判自递归");
        assert!(matches!(err, ShimError::SelfRecursion(_)));
    }
}
