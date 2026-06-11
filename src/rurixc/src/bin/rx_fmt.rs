//! `rx fmt` 雏形二进制(契约 D-M1-5;M6 收编 rx CLI 前形态自由,RD-005)。
//!
//! 用法:
//!   rx_fmt <file>                      格式化结果写 stdout
//!   rx_fmt --check <file>              已格式化则退出 0,否则 1
//!   rx_fmt --check-idempotent <file>   核对 fmt(fmt(x)) == fmt(x),违例退出 1(G-M1-5)

use std::process::ExitCode;

use rurixc::fmt::format_source;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (mode, path) = match args.get(1).map(String::as_str) {
        Some("--check") => ("check", args.get(2)),
        Some("--check-idempotent") => ("idem", args.get(2)),
        Some(_) => ("fmt", args.get(1)),
        None => ("fmt", None),
    };
    let Some(path) = path else {
        eprintln!("usage: rx_fmt [--check|--check-idempotent] <file>");
        return ExitCode::from(2);
    };
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rx_fmt: 读取 {path} 失败: {e}");
            return ExitCode::from(2);
        }
    };
    let once = match format_source(&src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rx_fmt: {path}: {e}");
            return ExitCode::from(2);
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
                eprintln!("rx_fmt: {path}: 未格式化");
                ExitCode::from(1)
            }
        }
        _ => {
            // --check-idempotent:G-M1-5 字节级判据
            let twice = match format_source(&once) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("rx_fmt: {path}: 二次 fmt 失败(输出词法不洁): {e}");
                    return ExitCode::from(1);
                }
            };
            if once == twice {
                ExitCode::SUCCESS
            } else {
                eprintln!("rx_fmt: {path}: fmt(fmt(x)) != fmt(x)");
                ExitCode::from(1)
            }
        }
    }
}
