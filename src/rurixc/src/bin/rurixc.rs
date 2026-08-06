//! rurixc 驱动:`.rx` → EXE + PDB 的端到端 host 编译闭环(M2.3,契约 G-M2-1)。
//!
//! M6.1:编译管线抽到 [`rurixc::driver`](库面,供 rurixc 驱动与 rx CLI 复用单一
//! 前端,07 §2);本 bin 仅负责 argv 解析后委托 [`rurixc::driver::compile`],
//! 行为相对既有驱动零语义漂移(既有 golden / hello-world 冒烟不变)。
//!
//! M6.4:`--tooling-server` 常驻 LSP 进程;`--tooling-smoke` 能力面冒烟(JSON stdout)。
//!
//! G8.2 M85:工具模式 `--merge-manifests` / `--assemble-manifest`(不走编译管线)。
//!
//! 工具链定位:
//! - clang:`RURIXC_CLANG` 环境变量 > `C:\Program Files\LLVM\bin\clang.exe` > PATH;
//!   版本断言 22.1.x(违例 = RX7001,pin 纪律)。
//! - link.exe:`RURIXC_LINK` > vswhere 定位 VS BuildTools;MSVC/SDK 库目录自动发现。
//!
//! 用法:
//! - `rurixc <input.rx> [-o <out.exe>] [--emit=check|mir|reflection|llvm-ir|nvptx-ir|ptx] [--error-format=json] [--self-profile=<file.json>]`
//! - `rurixc --tooling-server [--stdio]`
//! - `rurixc --tooling-smoke <sample.rx>`
//! - `rurixc --merge-manifests -o <merged.json> <a.json> <b.json> ...`
//! - `rurixc --assemble-manifest -o <unit.json> --reflection <r.json> [--permutations <p.json>] [--collector <c.json>]`

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use rurixc::driver::{self, CompileOptions};
use rurixc::tooling::{run_smoke, run_stdio_server};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--tooling-server") {
        if let Err(e) = run_stdio_server() {
            eprintln!("rurixc: tooling-server error: {e}");
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }
    if args.first().is_some_and(|a| a == "--tooling-smoke") {
        return tooling_smoke(&args[1..]);
    }
    if args.first().is_some_and(|a| a == "--merge-manifests") {
        return merge_manifests_cli(&args[1..]);
    }
    if args.first().is_some_and(|a| a == "--assemble-manifest") {
        return assemble_manifest_cli(&args[1..]);
    }

    let mut input: Option<String> = None;
    let mut out: Option<String> = None;
    let mut emit: Option<String> = None;
    let mut target: Option<String> = None;
    let mut profile_out: Option<PathBuf> = None;
    let mut error_format: Option<String> = None;
    let mut permutation_budget: Option<u32> = None;
    let mut permutation_select: Option<String> = None;
    let mut profile: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                out = args.get(i).cloned();
            }
            s if s.starts_with("--emit=") => emit = Some(s["--emit=".len()..].to_owned()),
            "--target" => {
                i += 1;
                target = args.get(i).cloned();
            }
            s if s.starts_with("--target=") => target = Some(s["--target=".len()..].to_owned()),
            "--profile" => {
                i += 1;
                profile = args.get(i).cloned();
            }
            s if s.starts_with("--profile=") => {
                profile = Some(s["--profile=".len()..].to_owned());
            }
            s if s.starts_with("--self-profile=") => {
                profile_out = Some(PathBuf::from(&s["--self-profile=".len()..]));
            }
            s if s.starts_with("--error-format=") => {
                error_format = Some(s["--error-format=".len()..].to_owned());
            }
            s if s.starts_with("--permutation-budget=") => {
                let text = &s["--permutation-budget=".len()..];
                match text.parse::<u32>() {
                    Ok(v) if v > 0 => permutation_budget = Some(v),
                    _ => {
                        eprintln!(
                            "rurixc: invalid --permutation-budget=`{text}`(须为正整数 u32,RXS-0310)"
                        );
                        return ExitCode::from(2);
                    }
                }
            }
            s if s.starts_with("--permutation-select=") => {
                permutation_select = Some(s["--permutation-select=".len()..].to_owned());
            }
            s if !s.starts_with('-') && input.is_none() => input = Some(s.to_owned()),
            s => {
                eprintln!("rurixc: unknown argument `{s}`");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let Some(input) = input else {
        eprintln!(
            "usage: rurixc <input.rx> [-o <out.exe>] [--emit=check|mir|reflection|permutations|capabilities|llvm-ir] [--profile <profile.json>] [--permutation-budget=N] [--permutation-select=KEY] [--error-format=json] [--self-profile=<file.json>]\n       rurixc --tooling-server\n       rurixc --tooling-smoke <sample.rx>\n       rurixc --merge-manifests -o <merged.json> <a.json> <b.json> ...\n       rurixc --assemble-manifest -o <unit.json> --reflection <r.json> [--permutations <p.json>] [--collector <c.json>]"
        );
        return ExitCode::from(2);
    };
    ExitCode::from(driver::compile(&CompileOptions {
        input: PathBuf::from(input),
        out: out.map(PathBuf::from),
        emit,
        profile_out,
        reproducible: false,
        error_format,
        target,
        permutation_budget,
        permutation_select,
        profile: profile.map(PathBuf::from),
    }))
}

fn merge_manifests_cli(args: &[String]) -> ExitCode {
    #[cfg(not(feature = "shader-stages"))]
    {
        let _ = args;
        eprintln!("rurixc: --merge-manifests 需要 feature `shader-stages`");
        return ExitCode::from(2);
    }
    #[cfg(feature = "shader-stages")]
    {
        let mut out: Option<PathBuf> = None;
        let mut inputs: Vec<PathBuf> = Vec::new();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-o" => {
                    i += 1;
                    match args.get(i) {
                        Some(p) => out = Some(PathBuf::from(p)),
                        None => {
                            eprintln!("rurixc: --merge-manifests `-o` 缺路径");
                            return ExitCode::from(2);
                        }
                    }
                }
                s if !s.starts_with('-') => inputs.push(PathBuf::from(s)),
                s => {
                    eprintln!("rurixc: --merge-manifests unknown argument `{s}`");
                    return ExitCode::from(2);
                }
            }
            i += 1;
        }
        let Some(out) = out else {
            eprintln!(
                "usage: rurixc --merge-manifests -o <merged.json> <a.json> <b.json> ..."
            );
            return ExitCode::from(2);
        };
        match rurixc::manifest::merge_manifest_files(&inputs, &out) {
            Ok(m) => {
                eprintln!(
                    "rurixc: --merge-manifests: {} (manifest_digest={})",
                    out.display(),
                    m.digest_hex()
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("rurixc: --merge-manifests: {e}");
                ExitCode::from(1)
            }
        }
    }
}

fn assemble_manifest_cli(args: &[String]) -> ExitCode {
    #[cfg(not(feature = "shader-stages"))]
    {
        let _ = args;
        eprintln!("rurixc: --assemble-manifest 需要 feature `shader-stages`");
        return ExitCode::from(2);
    }
    #[cfg(feature = "shader-stages")]
    {
        let mut out: Option<PathBuf> = None;
        let mut reflection: Option<PathBuf> = None;
        let mut permutations: Option<PathBuf> = None;
        let mut collector: Option<PathBuf> = None;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-o" => {
                    i += 1;
                    out = args.get(i).map(PathBuf::from);
                }
                "--reflection" => {
                    i += 1;
                    reflection = args.get(i).map(PathBuf::from);
                }
                s if s.starts_with("--reflection=") => {
                    reflection = Some(PathBuf::from(&s["--reflection=".len()..]));
                }
                "--permutations" => {
                    i += 1;
                    permutations = args.get(i).map(PathBuf::from);
                }
                s if s.starts_with("--permutations=") => {
                    permutations = Some(PathBuf::from(&s["--permutations=".len()..]));
                }
                "--collector" => {
                    i += 1;
                    collector = args.get(i).map(PathBuf::from);
                }
                s if s.starts_with("--collector=") => {
                    collector = Some(PathBuf::from(&s["--collector=".len()..]));
                }
                s => {
                    eprintln!("rurixc: --assemble-manifest unknown argument `{s}`");
                    return ExitCode::from(2);
                }
            }
            i += 1;
        }
        let (Some(out), Some(reflection)) = (out, reflection) else {
            eprintln!(
                "usage: rurixc --assemble-manifest -o <unit.json> --reflection <r.json> [--permutations <p.json>] [--collector <c.json>]"
            );
            return ExitCode::from(2);
        };
        match rurixc::manifest::assemble_manifest_files(
            &reflection,
            permutations.as_deref(),
            collector.as_deref(),
            &out,
        ) {
            Ok(m) => {
                eprintln!(
                    "rurixc: --assemble-manifest: {} (manifest_digest={})",
                    out.display(),
                    m.digest_hex()
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("rurixc: --assemble-manifest: {e}");
                ExitCode::from(1)
            }
        }
    }
}

fn tooling_smoke(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: rurixc --tooling-smoke <sample.rx>");
        return ExitCode::from(2);
    };
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rurixc: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let uri = format!("file:///{}", path.replace('\\', "/"));
    let result = run_smoke(&src, &uri);
    let caps_json: String = result
        .capabilities_passed
        .iter()
        .map(|c| format!("\"{}\"", c))
        .collect::<Vec<_>>()
        .join(",");
    let fail_json: String = result
        .failures
        .iter()
        .map(|f| format!("\"{}\"", f.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(",");
    let out = format!(
        "{{\"capabilities_passed\":[{}],\"failures\":[{}],\"ok\":{}}}",
        caps_json,
        fail_json,
        result.failures.is_empty()
    );
    let _ = std::io::stdout().write_all(out.as_bytes());
    if result.failures.is_empty() && result.capabilities_passed.len() >= 5 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
