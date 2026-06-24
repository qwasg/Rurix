//! rurixc 驱动:`.rx` → EXE + PDB 的端到端 host 编译闭环(M2.3,契约 G-M2-1)。
//!
//! M6.1:编译管线抽到 [`rurixc::driver`](库面,供 rurixc 驱动与 rx CLI 复用单一
//! 前端,07 §2);本 bin 仅负责 argv 解析后委托 [`rurixc::driver::compile`],
//! 行为相对既有驱动零语义漂移(既有 golden / hello-world 冒烟不变)。
//!
//! M6.4:`--tooling-server` 常驻 LSP 进程;`--tooling-smoke` 能力面冒烟(JSON stdout)。
//!
//! 工具链定位:
//! - clang:`RURIXC_CLANG` 环境变量 > `C:\Program Files\LLVM\bin\clang.exe` > PATH;
//!   版本断言 22.1.x(违例 = RX7001,pin 纪律)。
//! - link.exe:`RURIXC_LINK` > vswhere 定位 VS BuildTools;MSVC/SDK 库目录自动发现。
//!
//! 用法:
//! - `rurixc <input.rx> [-o <out.exe>] [--emit=check|mir|llvm-ir|nvptx-ir|ptx] [--error-format=json] [--self-profile=<file.json>]`
//! - `rurixc --tooling-server [--stdio]`
//! - `rurixc --tooling-smoke <sample.rx>`

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

    let mut input: Option<String> = None;
    let mut out: Option<String> = None;
    let mut emit: Option<String> = None;
    let mut target: Option<String> = None;
    let mut profile_out: Option<PathBuf> = None;
    let mut error_format: Option<String> = None;
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
            s if s.starts_with("--self-profile=") => {
                profile_out = Some(PathBuf::from(&s["--self-profile=".len()..]));
            }
            s if s.starts_with("--error-format=") => {
                error_format = Some(s["--error-format=".len()..].to_owned());
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
            "usage: rurixc <input.rx> [-o <out.exe>] [--emit=check|mir|llvm-ir] [--error-format=json] [--self-profile=<file.json>]\n       rurixc --tooling-server\n       rurixc --tooling-smoke <sample.rx>"
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
    }))
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
