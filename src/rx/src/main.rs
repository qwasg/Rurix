//! rx — Rurix 工具链 CLI 总入口(M6,08 §7 D-239;spec/toolchain.md RXS-0083~0097)。
//!
//! 单一前端纪律(07 §2,RXS-0083):涉及编译的子命令(build/run/check)经
//! [`rurixc::driver`] 复用 rurixc query 层管线,**不另起引擎**;rx 是子命令分发
//! 与产物编排层。fmt 收编 M1 雏形格式器(RD-005,RXS-0087);bench 作统一入口
//! 编排既有 BENCH_PROTOCOL 协议(RD-003,RXS-0088)。
//!
//! 包管理(M6.2,RXS-0089~0094):`rx vendor` 与 `rx build` 的 manifest 解析前段
//! 经 [`rurix_pkg`] 子系统(rurix.toml 三来源解析 + 依赖解析图 + rurix.lock +
//! 内容树 SHA-256 + 离线路径);编译仍经 [`rurixc::driver`] 单一前端,不另起引擎。
//! `rx test`(M6.3,RXS-0095):发现顶层 `#[test]`/`#[test(gpu)]`,逐测试渲染临时
//! harness 并以子进程隔离运行;GPU 测试崩溃不连坐父 harness(14 §6)。
//!
//! 退出码约定(RXS-0083):0 成功 / 1 诊断错误 / 2 用法·I/O 错误。
//! `rx run` 透传产物退出码为受控例外(RXS-0085)。
//!
//! 工具链诊断错误码(7xxx 链接/工具链段位,registry/error_codes.json):
//! RX7003 子命令用法错误 / RX7004 rx run 产物执行失败 / RX7005~RX7009 包管理
//! (清单/解析冲突/lock 不一致/digest 不符/来源不可达,RXS-0089~0094) /
//! RX7010~RX7011 rx test 发现与子进程执行诊断。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use rurix_pkg::PkgError;
use rurixc::driver::{self, CompileOptions};
use rurixc::fmt::format_source;
use rurixc::test_harness::{self, TestKind};

mod doc;

const USAGE: &str =
    "usage: rx <build|run|check|test|fmt|bench|vendor|doc> ...\n  (fix|watch 后续小里程碑承接)";

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
        "test" => cmd_test(rest),
        // rx doc(M8.6,D-M8-6 / G-M8-6):从既有单一事实源确定性生成文档站(RXS-0083 分发位兑现)。
        "doc" => doc::run(rest),
        // 仍保留的分发位,返回"未实现"用法诊断(RXS-0083;后续里程碑承接)
        "fix" | "watch" => {
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
    /// `--emit=<target>`(透传 rurixc:check/mir/llvm-ir/nvptx-ir/ptx;`pyd` = M8.1
    /// 互操作 PYD 产出,RXS-0122)。None = 默认 host EXE。
    emit: Option<String>,
    /// `--target <ptx|dxil>`(RXS-0157,RFC-0003 §9 Q-CLI):`dxil` 选 DXIL 第二
    /// 后端(gate `dxil-backend`);None/`ptx` 维持现状默认通道。
    target: Option<String>,
}

/// `rx build --emit` 透传给 rurixc 的合法目标(host/device codegen 通道)。
const RURIXC_EMIT_TARGETS: &[&str] = &["check", "mir", "llvm-ir", "nvptx-ir", "ptx"];

/// `rx build [<input.rx>] [-o <out>] [--manifest-path <p>] [--locked] [--offline]`。
fn parse_build_args(args: &[String]) -> Result<BuildArgs, String> {
    let mut b = BuildArgs {
        input: None,
        out: None,
        manifest_path: None,
        locked: false,
        offline: false,
        emit: None,
        target: None,
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
            "--target" => {
                i += 1;
                let t = args.get(i).ok_or("`--target` 缺目标参数(合法:ptx/dxil/vulkan)")?;
                if t != "ptx" && t != "dxil" && t != "vulkan" {
                    return Err(format!("无法识别的 --target 目标 `{t}`(合法:ptx/dxil/vulkan)"));
                }
                b.target = Some(t.clone());
            }
            s if s.starts_with("--target=") => {
                let t = s["--target=".len()..].to_owned();
                if t != "ptx" && t != "dxil" && t != "vulkan" {
                    return Err(format!("无法识别的 --target 目标 `{t}`(合法:ptx/dxil/vulkan)"));
                }
                b.target = Some(t);
            }
            s if s.starts_with("--emit=") => {
                let target = s["--emit=".len()..].to_owned();
                if target != "pyd" && !RURIXC_EMIT_TARGETS.contains(&target.as_str()) {
                    return Err(format!(
                        "无法识别的 --emit 目标 `{target}`(合法:check/mir/llvm-ir/nvptx-ir/ptx/pyd)"
                    ));
                }
                b.emit = Some(target);
            }
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

    // M8.1 互操作 PYD 产出(RXS-0122):rx 编排 nanobind + scikit-build-core 打包。
    if b.emit.as_deref() == Some("pyd") {
        return build_pyd(&b);
    }

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
            emit: b.emit.clone(),
            profile_out: None,
            reproducible: b.locked && b.offline,
            error_format: None,
            target: b.target.clone(),
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
        emit: b.emit.clone(),
        profile_out: None,
        reproducible: false,
        error_format: None,
        target: b.target.clone(),
    }))
}

/// `rx build --emit=pyd <kernel.rx> [-o <out_dir>]`(M8.1,D-M8-1,RXS-0122):产
/// Python 扩展模块(`.pyd`)经 **nanobind + scikit-build-core**(09 §6),链接
/// `rurix-interop` 运行时(C ABI + 复用 M5 自研 kernel),供 PyTorch 经
/// `__cuda_array_interface__` v3 / DLPack 双协议零拷贝接入。
///
/// 编排:(1) rurixc 把输入 device kernel 全管线产 PTX(`--emit=pyd` 编译校验,
/// 无 kernel → RX7013);(2) `cargo build -p rurix-interop --release` 产 staticlib;
/// (3) `pip install <pyd 工程> --target` 经 scikit-build-core 产 `.pyd`(注入
/// `RURIX_INTEROP_LIB`);(4) 拷贝 `.pyd`(保留 ABI 标记名)到输出目录。
fn build_pyd(b: &BuildArgs) -> ExitCode {
    let Some(input) = b.input.clone() else {
        usage_error("--emit=pyd 缺输入 `.rx` kernel 源");
        return ExitCode::from(2);
    };
    let out_dir = b
        .out
        .clone()
        .unwrap_or_else(|| input.parent().unwrap_or(Path::new(".")).to_path_buf());

    // (1) 编译校验输入 device kernel → PTX(genuine 编译通道;无 kernel → RX7013)。
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("kernel")
        .to_owned();
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!(
            "rx build --emit=pyd: 无法创建输出目录 {}: {e}",
            out_dir.display()
        );
        return ExitCode::from(1);
    }
    let ptx_stage = out_dir.join(format!("{stem}.ptx"));
    let rc = driver::compile(&CompileOptions {
        input: input.clone(),
        out: Some(ptx_stage),
        emit: Some("pyd".to_owned()),
        profile_out: None,
        reproducible: false,
        error_format: None,
        target: None,
    });
    if rc != 0 {
        return ExitCode::from(rc);
    }

    // (2) 定位 workspace 根(含互操作 PYD 工程)。
    let Some(root) = find_workspace_with_pyd() else {
        eprintln!(
            "rx build --emit=pyd: 未找到含 src/rurix-interop/pyd/pyproject.toml 的 workspace 根(从当前目录向上)"
        );
        return ExitCode::from(1);
    };
    let pyd_proj = root.join("src").join("rurix-interop").join("pyd");
    let staticlib = root
        .join("target")
        .join("release")
        .join("rurix_interop.lib");

    // (3) cargo build staticlib(单一事实源:复用 M5 kernel 嵌入 PTX)。
    eprintln!("rx build --emit=pyd: 构建 rurix-interop staticlib(cargo --release)…");
    if !run_inherit(
        "cargo",
        &["build", "-p", "rurix-interop", "--release"],
        &root,
    ) {
        eprintln!("rx build --emit=pyd: cargo build -p rurix-interop 失败");
        return ExitCode::from(1);
    }
    if !staticlib.exists() {
        eprintln!(
            "rx build --emit=pyd: staticlib 未产出: {}",
            staticlib.display()
        );
        return ExitCode::from(1);
    }

    // (4) scikit-build-core 产 .pyd(pip install --target,注入 RURIX_INTEROP_LIB)。
    let stage = out_dir.join(".rx-pyd-stage");
    let _ = std::fs::remove_dir_all(&stage);
    let (py, mut py_args) = python_command();
    let lib_define = format!(
        "cmake.define.RURIX_INTEROP_LIB={}",
        staticlib.display().to_string().replace('\\', "/")
    );
    let proj_s = pyd_proj.display().to_string();
    let stage_s = stage.display().to_string();
    py_args.extend(
        [
            "-m",
            "pip",
            "install",
            proj_s.as_str(),
            "--target",
            stage_s.as_str(),
            "--no-deps",
            "--no-build-isolation",
            "--upgrade",
            "--config-settings",
            lib_define.as_str(),
        ]
        .map(str::to_owned),
    );
    let py_args_ref: Vec<&str> = py_args.iter().map(String::as_str).collect();
    eprintln!("rx build --emit=pyd: scikit-build-core 打包 PYD(nanobind)…");
    if !run_inherit(&py, &py_args_ref, &root) {
        eprintln!("rx build --emit=pyd: scikit-build-core 打包失败");
        return ExitCode::from(1);
    }

    // (5) 拷贝产出的 .pyd(保留模块名 rurix_uc01.*.pyd)到输出目录。
    let mut produced: Option<PathBuf> = None;
    if let Ok(entries) = std::fs::read_dir(&stage) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("pyd") {
                let dest = out_dir.join(p.file_name().unwrap());
                if let Err(err) = std::fs::copy(&p, &dest) {
                    eprintln!("rx build --emit=pyd: 拷贝 .pyd 失败: {err}");
                    return ExitCode::from(1);
                }
                produced = Some(dest);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&stage);
    match produced {
        Some(p) => {
            eprintln!("rx build --emit=pyd: PYD 产出完成 → {}", p.display());
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("rx build --emit=pyd: 未在打包产物中找到 .pyd");
            ExitCode::from(1)
        }
    }
}

/// 从当前目录向上查找含 `src/rurix-interop/pyd/pyproject.toml` 的 workspace 根。
fn find_workspace_with_pyd() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir
            .join("src")
            .join("rurix-interop")
            .join("pyd")
            .join("pyproject.toml")
            .is_file()
        {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Python 解释器命令(沿 rx bench 先例:`RX_PYTHON` env 覆盖,否则 Windows `py -3`
/// / 其余 `python3`)。
fn python_command() -> (String, Vec<String>) {
    if let Ok(p) = std::env::var("RX_PYTHON") {
        (p, Vec::new())
    } else if cfg!(windows) {
        ("py".to_owned(), vec!["-3".to_owned()])
    } else {
        ("python3".to_owned(), Vec::new())
    }
}

/// 运行外部命令(继承 stdout/stderr),返回是否成功退出(status.success())。
fn run_inherit(prog: &str, args: &[&str], cwd: &Path) -> bool {
    std::process::Command::new(prog)
        .args(args)
        .current_dir(cwd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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

struct TestArgs {
    input: Option<PathBuf>,
    filter: Option<String>,
    gpu: bool,
    manifest_path: Option<PathBuf>,
    locked: bool,
    offline: bool,
}

/// `rx test [<file.rx>] [--filter <substring>] [--gpu] [--manifest-path <p>] [--locked] [--offline]`。
fn parse_test_args(args: &[String]) -> Result<TestArgs, String> {
    let mut t = TestArgs {
        input: None,
        filter: None,
        gpu: false,
        manifest_path: None,
        locked: false,
        offline: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--filter" => {
                i += 1;
                t.filter = Some(args.get(i).ok_or("`--filter` 缺过滤字符串参数")?.clone());
            }
            "--gpu" => t.gpu = true,
            "--manifest-path" => {
                i += 1;
                t.manifest_path = Some(PathBuf::from(
                    args.get(i).ok_or("`--manifest-path` 缺路径参数")?,
                ));
            }
            "--locked" => t.locked = true,
            "--offline" => t.offline = true,
            s if !s.starts_with('-') && t.input.is_none() => t.input = Some(PathBuf::from(s)),
            s => return Err(format!("rx test 无法识别的参数 `{s}`")),
        }
        i += 1;
    }
    Ok(t)
}

/// rx test(RXS-0095):发现顶层 `#[test]`/`#[test(gpu)]`,逐测试生成临时
/// `main` 并以子进程运行。GPU 分类测试用 `--gpu` 选择,同样经子进程隔离。
fn cmd_test(args: &[String]) -> ExitCode {
    let t = match parse_test_args(args) {
        Ok(v) => v,
        Err(e) => {
            usage_error(&e);
            return ExitCode::from(2);
        }
    };

    let input = if let Some(mp) = &t.manifest_path {
        let base = mp.parent().unwrap_or(Path::new(".")).to_path_buf();
        let front = if t.locked {
            rurix_pkg::vendor::verify_locked(&base, t.offline)
        } else {
            rurix_pkg::vendor::resolve_workspace(&base, t.offline)
        };
        if let Err(e) = front {
            return report_pkg_error(e);
        }
        t.input
            .clone()
            .unwrap_or_else(|| base.join("src").join("test.rx"))
    } else {
        if t.locked || t.offline {
            usage_error("--locked/--offline 仅在 --manifest-path 包上下文下有效");
            return ExitCode::from(2);
        }
        let Some(input) = t.input.clone() else {
            usage_error("rx test 缺输入 `.rx` 源文件或 --manifest-path");
            return ExitCode::from(2);
        };
        input
    };

    let src = match fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            usage_error(&format!("rx test 无法读取 {}: {e}", input.display()));
            return ExitCode::from(2);
        }
    };
    let discovered = match test_harness::discover_tests(&src) {
        Ok(v) => v,
        Err(e) => {
            report_rx_test_discovery(e.detail());
            return ExitCode::from(1);
        }
    };
    let want_kind = if t.gpu { TestKind::Gpu } else { TestKind::Host };
    let tests: Vec<_> = discovered
        .into_iter()
        .filter(|case| case.kind == want_kind)
        .filter(|case| {
            t.filter
                .as_ref()
                .is_none_or(|needle| case.name.contains(needle))
        })
        .collect();
    if tests.is_empty() {
        let kind = if t.gpu { "#[test(gpu)]" } else { "#[test]" };
        report_rx_test_discovery(&format!("未发现匹配的 {kind} 测试"));
        return ExitCode::from(1);
    }

    let temp = unique_temp_dir("rx_test");
    if let Err(e) = fs::create_dir_all(&temp) {
        report_rx_test_exec_failure(&format!(
            "创建临时 harness 目录失败 {}: {e}",
            temp.display()
        ));
        return ExitCode::from(1);
    }

    let mut failed = 0usize;
    for (idx, case) in tests.iter().enumerate() {
        println!("rx test: {} ...", case.name);
        let harness_src = test_harness::render_harness(&src, case);
        let stem = format!("{idx:04}_{}", sanitize_stem(&case.name));
        let harness_path = temp.join(format!("{stem}.rx"));
        let exe_path = temp.join(format!("{stem}.exe"));
        if let Err(e) = fs::write(&harness_path, harness_src) {
            report_rx_test_exec_failure(&format!("{}: 写临时 harness 失败: {e}", case.name));
            failed += 1;
            continue;
        }
        let compile_code = driver::compile(&CompileOptions {
            input: harness_path,
            out: Some(exe_path.clone()),
            emit: None,
            profile_out: None,
            reproducible: false,
            error_format: None,
            target: None,
        });
        if compile_code != 0 {
            report_rx_test_exec_failure(&format!(
                "{}: harness 编译失败(exit {compile_code})",
                case.name
            ));
            failed += 1;
            continue;
        }
        match Command::new(&exe_path).status() {
            Ok(status) if status.success() => {
                println!("rx test: {} ... ok", case.name);
            }
            Ok(status) => {
                report_rx_test_exec_failure(&format!("{}: 子进程退出 {}", case.name, status));
                failed += 1;
            }
            Err(e) => {
                report_rx_test_exec_failure(&format!(
                    "{}: 无法启动子进程 {}: {e}",
                    case.name,
                    exe_path.display()
                ));
                failed += 1;
            }
        }
    }
    let _ = fs::remove_dir_all(&temp);

    let total = tests.len();
    if failed == 0 {
        println!("rx test: PASS {total}/{total}");
        ExitCode::SUCCESS
    } else {
        eprintln!("rx test: FAIL {} passed; {failed} failed", total - failed);
        ExitCode::from(1)
    }
}

fn report_rx_test_discovery(detail: &str) {
    eprintln!("rx test: error[RX7010]: {detail}");
}

fn report_rx_test_exec_failure(detail: &str) {
    eprintln!("rx test: error[RX7011]: {detail}");
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nanos))
}

fn sanitize_stem(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "test".to_owned()
    } else {
        out
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
        reproducible: false,
        error_format: None,
        target: None,
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
        reproducible: false,
        error_format: None,
        target: None,
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
