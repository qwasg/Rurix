//! rx — Rurix 工具链 CLI 总入口(M6.1,08 §7 D-239;spec/toolchain.md RXS-0083~0088)。
//!
//! 单一前端纪律(07 §2,RXS-0083):涉及编译的子命令(build/run/check)经
//! [`rurixc::driver`] 复用 rurixc query 层管线,**不另起引擎**;rx 是子命令分发
//! 与产物编排层。fmt 收编 M1 雏形格式器(RD-005,RXS-0087);bench 作统一入口
//! 编排既有 BENCH_PROTOCOL 协议(RD-003,RXS-0088)。
//!
//! 包管理(M6.2,RXS-0089~0094):`rx vendor` 与 `rx build` 的 manifest 解析前段
//! 经 [`rurix_pkg`] 子系统(rurix.toml 三来源解析 + 依赖解析图 + rurix.lock +
//! 内容树 SHA-256 + 离线路径);编译仍经 [`rurixc::driver`] 单一前端,不另起引擎。
//!
//! 退出码约定(RXS-0083):0 成功 / 1 诊断错误 / 2 用法·I/O 错误。
//! `rx run` 透传产物退出码为受控例外(RXS-0085)。
//!
//! 工具链诊断错误码(7xxx 链接/工具链段位,registry/error_codes.json):
//! RX7003 子命令用法错误 / RX7004 rx run 产物执行失败 / RX7005~RX7009 包管理
//! (清单/解析冲突/lock 不一致/digest 不符/来源不可达,RXS-0089~0094)。

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use rurix_pkg::PkgError;
use rurixc::driver::{self, CompileOptions};
use rurixc::fmt::format_source;

const USAGE: &str =
    "usage: rx <build|run|check|fmt|bench|vendor> ...\n  (test|doc|fix|watch 后续小里程碑承接)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(sub) = args.first() else {
        usage_error("缺子命令");
        return ExitCode::from(2);
    };
    let rest = &args[1..];
    match sub.as_str() {
        "build" => cmd_build(rest),
        "check" => cmd_check(rest),
        "run" => cmd_run(rest),
        "fmt" => cmd_fmt(rest),
        "bench" => cmd_bench(rest),
        "vendor" => cmd_vendor(rest),
        // 已登记分发位,M6.1/M6.2 期返回"未实现"用法诊断(RXS-0083;后续里程碑承接)
        "test" | "doc" | "fix" | "watch" => {
            usage_error(&format!("子命令 `{sub}` 尚未实现(后续小里程碑承接)"));
            ExitCode::from(2)
        }
        other => {
            usage_error(&format!("未知子命令 `{other}`"));
            ExitCode::from(2)
        }
    }
}

/// RX7003 用法诊断(RXS-0083;7xxx 链接/工具链段位 rx CLI 首批)。
fn usage_error(detail: &str) {
    eprintln!("rx: error[RX7003]: {detail}");
    eprintln!("{USAGE}");
}

/// 包管理诊断映射(RXS-0089~0094):RX7005~RX7009 inline 落 stderr,退出码 1
/// (诊断错误,RXS-0083 退出码约定);沿用 M6.1 rx 以 inline eprintln 发 7xxx 码。
fn report_pkg_error(e: PkgError) -> ExitCode {
    eprintln!("rx: {e}");
    ExitCode::from(1)
}

/// `<input.rx> [-o <out>]` 解析(build/run 共用)。Err 串为用法错误细节。
fn parse_input_out(args: &[String]) -> Result<(PathBuf, Option<PathBuf>), String> {
    let mut input: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or("`-o` 缺输出路径参数")?));
            }
            s if !s.starts_with('-') && input.is_none() => input = Some(PathBuf::from(s)),
            s => return Err(format!("无法识别的参数 `{s}`")),
        }
        i += 1;
    }
    let input = input.ok_or("缺输入 `.rx` 源文件")?;
    Ok((input, out))
}

/// rx build 解析后的参数(M6.1 单文件 + M6.2 包上下文)。
struct BuildArgs {
    input: Option<PathBuf>,
    out: Option<PathBuf>,
    manifest_path: Option<PathBuf>,
    locked: bool,
    offline: bool,
}

/// `rx build [<input.rx>] [-o <out>] [--manifest-path <p>] [--locked] [--offline]`。
fn parse_build_args(args: &[String]) -> Result<BuildArgs, String> {
    let mut b = BuildArgs {
        input: None,
        out: None,
        manifest_path: None,
        locked: false,
        offline: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                b.out = Some(PathBuf::from(args.get(i).ok_or("`-o` 缺输出路径参数")?));
            }
            "--manifest-path" => {
                i += 1;
                b.manifest_path = Some(PathBuf::from(
                    args.get(i).ok_or("`--manifest-path` 缺路径参数")?,
                ));
            }
            "--locked" => b.locked = true,
            "--offline" => b.offline = true,
            s if !s.starts_with('-') && b.input.is_none() => b.input = Some(PathBuf::from(s)),
            s => return Err(format!("无法识别的参数 `{s}`")),
        }
        i += 1;
    }
    Ok(b)
}

/// rx build(RXS-0084 + RXS-0094):经 rurixc query 层产 host EXE(默认)。
///
/// 包上下文(`--manifest-path`,RXS-0094):先跑 manifest 解析前段(`--locked`
/// 校验入库 lock 一致 + vendor 内容树 digest;否则解析校验清单/解析图),再经
/// [`rurixc::driver`] 单一前端编译根入口(默认 `<base>/src/main.rx`,**不另起引擎**)。
fn cmd_build(args: &[String]) -> ExitCode {
    let b = match parse_build_args(args) {
        Ok(v) => v,
        Err(e) => {
            usage_error(&e);
            return ExitCode::from(2);
        }
    };

    if let Some(mp) = &b.manifest_path {
        let base = mp.parent().unwrap_or(Path::new(".")).to_path_buf();
        let front = if b.locked {
            rurix_pkg::vendor::verify_locked(&base, b.offline)
        } else {
            rurix_pkg::vendor::resolve_workspace(&base, b.offline)
        };
        if let Err(e) = front {
            return report_pkg_error(e);
        }
        // 根入口:显式 input 优先,否则包默认 <base>/src/main.rx。
        let entry = b
            .input
            .clone()
            .unwrap_or_else(|| base.join("src").join("main.rx"));
        return ExitCode::from(driver::compile(&CompileOptions {
            input: entry,
            out: b.out,
            emit: None,
            profile_out: None,
        }));
    }

    // 无 manifest:M6.1 单文件编译路径(向后兼容)。
    if b.locked || b.offline {
        usage_error("--locked/--offline 仅在 --manifest-path 包上下文下有效");
        return ExitCode::from(2);
    }
    let Some(input) = b.input else {
        usage_error("缺输入 `.rx` 源文件或 --manifest-path");
        return ExitCode::from(2);
    };
    ExitCode::from(driver::compile(&CompileOptions {
        input,
        out: b.out,
        emit: None,
        profile_out: None,
    }))
}

/// rx vendor(RXS-0094,收编保留分发位):解析图 → 落 vendor/<name>(path 依赖)
/// → 写 rurix.lock(含每包内容树 SHA-256)。CPU-only 无 codegen(供 CI 离线冒烟)。
///   rx vendor [--manifest-path <p>] [--offline]            写 lock + vendor
///   rx vendor --locked [--manifest-path <p>] [--offline]   只校验不重写
///     (入库 lock 与重解析图一致 RX7007 + vendor 内容树 digest 一致 RX7008)
fn cmd_vendor(args: &[String]) -> ExitCode {
    let mut manifest_path: Option<PathBuf> = None;
    let mut offline = false;
    let mut locked = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--manifest-path" => {
                i += 1;
                match args.get(i) {
                    Some(p) => manifest_path = Some(PathBuf::from(p)),
                    None => {
                        usage_error("rx vendor: `--manifest-path` 缺路径参数");
                        return ExitCode::from(2);
                    }
                }
            }
            "--offline" => offline = true,
            "--locked" => locked = true,
            s => {
                usage_error(&format!("rx vendor 无法识别的参数 `{s}`"));
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let base = match &manifest_path {
        Some(mp) => mp.parent().unwrap_or(Path::new(".")).to_path_buf(),
        None => PathBuf::from("."),
    };
    if locked {
        // 只校验:lock 一致(RX7007)+ vendor 内容树 digest 一致(RX7008),不重写。
        return match rurix_pkg::vendor::verify_locked(&base, offline) {
            Ok(graph) => {
                let n = graph.nodes.len().saturating_sub(1);
                println!("rx vendor --locked: {n} 个依赖 lock/digest 校验通过");
                ExitCode::SUCCESS
            }
            Err(e) => report_pkg_error(e),
        };
    }
    match rurix_pkg::vendor::run_vendor(&base, offline) {
        Ok(graph) => {
            let n = graph.nodes.len().saturating_sub(1);
            println!(
                "rx vendor: 解析 {n} 个依赖,写 {}",
                base.join("rurix.lock").display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => report_pkg_error(e),
    }
}

/// rx check(RXS-0086):仅前端全量静态检查,不产 codegen/link 产物。
fn cmd_check(args: &[String]) -> ExitCode {
    let (input, _out) = match parse_input_out(args) {
        Ok(v) => v,
        Err(e) => {
            usage_error(&e);
            return ExitCode::from(2);
        }
    };
    ExitCode::from(driver::compile(&CompileOptions {
        input,
        out: None,
        emit: Some("check".to_owned()),
        profile_out: None,
    }))
}

/// rx run(RXS-0085):build 成功后执行产物并**透传产物退出码**(受控例外)。
fn cmd_run(args: &[String]) -> ExitCode {
    let (input, out) = match parse_input_out(args) {
        Ok(v) => v,
        Err(e) => {
            usage_error(&e);
            return ExitCode::from(2);
        }
    };
    // 先 build host EXE(emit None);build 失败则停于 build 退出码语义。
    let exe = out.clone().unwrap_or_else(|| input.with_extension("exe"));
    let build_code = driver::compile(&CompileOptions {
        input,
        out,
        emit: None,
        profile_out: None,
    });
    if build_code != 0 {
        return ExitCode::from(build_code);
    }
    if !exe.exists() {
        eprintln!(
            "rx run: error[RX7004]: build 成功但产物缺失: {}",
            exe.display()
        );
        return ExitCode::from(1);
    }
    // 执行产物,透传产物进程退出码(RXS-0085 受控例外;可超 u8 → process::exit)。
    match Command::new(&exe).status() {
        Ok(status) => {
            let code = status.code().unwrap_or(1);
            std::process::exit(code);
        }
        Err(e) => {
            eprintln!("rx run: error[RX7004]: 无法启动产物 {}: {e}", exe.display());
            ExitCode::from(1)
        }
    }
}

/// rx fmt(RXS-0087,收编 RD-005):复用 rurixc::fmt::format_source 单一事实源。
///   rx fmt <file>                     格式化写 stdout
///   rx fmt --check <file>             已格式化 → 0,否则 1
///   rx fmt --check-idempotent <file>  fmt(fmt(x)) == fmt(x) → 0,否则 1
fn cmd_fmt(args: &[String]) -> ExitCode {
    let (mode, path) = match args.first().map(String::as_str) {
        Some("--check") => ("check", args.get(1)),
        Some("--check-idempotent") => ("idem", args.get(1)),
        Some(s) if !s.starts_with('-') => ("fmt", args.first()),
        Some(s) => {
            usage_error(&format!("rx fmt 无法识别的参数 `{s}`"));
            return ExitCode::from(2);
        }
        None => ("fmt", None),
    };
    let Some(path) = path else {
        usage_error("rx fmt 缺输入文件(usage: rx fmt [--check|--check-idempotent] <file>)");
        return ExitCode::from(2);
    };
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rx fmt: 读取 {path} 失败: {e}");
            return ExitCode::from(2);
        }
    };
    // 词法不洁 → 退出码 1(任务级失败,RXS-0087)。
    let once = match format_source(&src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rx fmt: {path}: {e}");
            return ExitCode::from(1);
        }
    };
    match mode {
        "fmt" => {
            print!("{once}");
            ExitCode::SUCCESS
        }
        "check" => {
            if src.replace("\r\n", "\n") == once {
                ExitCode::SUCCESS
            } else {
                eprintln!("rx fmt: {path}: 未格式化");
                ExitCode::from(1)
            }
        }
        _ => {
            // --check-idempotent:字节级幂等判据(G-M6-4 延续 G-M1-5)
            let twice = match format_source(&once) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("rx fmt: {path}: 二次 fmt 失败(输出词法不洁): {e}");
                    return ExitCode::from(1);
                }
            };
            if once == twice {
                ExitCode::SUCCESS
            } else {
                eprintln!("rx fmt: {path}: fmt(fmt(x)) != fmt(x)");
                ExitCode::from(1)
            }
        }
    }
}

/// rx bench(RXS-0088,收编 RD-003):统一入口编排既有 BENCH_PROTOCOL 协议
/// (`bench/*.py`),口径/证据格式完全不变。
///   rx bench [<name>] [--smoke] [extra...]   默认 name=saxpy
///
/// 解析 python 解释器(`RX_PYTHON` > `py -3` > `python`),从当前工作目录定位
/// `bench/<name>_bench.py` 并透传退出码(L0 锁频前置 / 三次进程级独立运行 /
/// trimmed mean 等协议纪律由被编排的脚本承担)。
fn cmd_bench(args: &[String]) -> ExitCode {
    let mut name = "saxpy".to_owned();
    let mut passthrough: Vec<String> = Vec::new();
    for a in args {
        if a.starts_with('-') {
            passthrough.push(a.clone());
        } else {
            name = a.clone();
        }
    }
    let script = PathBuf::from("bench").join(format!("{name}_bench.py"));
    if !script.is_file() {
        usage_error(&format!(
            "rx bench:未知基准 `{name}`(缺协议脚本 {})",
            script.display()
        ));
        return ExitCode::from(2);
    }
    // python 解释器候选:RX_PYTHON > `py -3`(Windows launcher)> `python`。
    let (prog, mut pre_args): (String, Vec<String>) = if let Ok(p) = std::env::var("RX_PYTHON") {
        (p, Vec::new())
    } else if cfg!(windows) {
        ("py".to_owned(), vec!["-3".to_owned()])
    } else {
        ("python3".to_owned(), Vec::new())
    };
    let mut cmd = Command::new(&prog);
    pre_args.push(script.to_string_lossy().into_owned());
    cmd.args(&pre_args).args(&passthrough);
    match cmd.status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("rx bench: 无法启动 python 协议编排(`{prog}`): {e}");
            ExitCode::from(1)
        }
    }
}
