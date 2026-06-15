//! rurixc 驱动:`.rx` → EXE + PDB 的端到端 host 编译闭环(M2.3,契约 G-M2-1)。
//!
//! M6.1:编译管线抽到 [`rurixc::driver`](库面,供 rurixc 驱动与 rx CLI 复用单一
//! 前端,07 §2);本 bin 仅负责 argv 解析后委托 [`rurixc::driver::compile`],
//! 行为相对既有驱动零语义漂移(既有 golden / hello-world 冒烟不变)。
//!
//! 工具链定位:
//! - clang:`RURIXC_CLANG` 环境变量 > `C:\Program Files\LLVM\bin\clang.exe` > PATH;
//!   版本断言 22.1.x(违例 = RX7001,pin 纪律)。
//! - link.exe:`RURIXC_LINK` > vswhere 定位 VS BuildTools;MSVC/SDK 库目录自动发现。
//!
//! 用法:`rurixc <input.rx> [-o <out.exe>] [--emit=check|mir|llvm-ir|nvptx-ir|ptx] [--self-profile=<file.json>]`

use std::path::PathBuf;
use std::process::ExitCode;

use rurixc::driver::{self, CompileOptions};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<String> = None;
    let mut out: Option<String> = None;
    let mut emit: Option<String> = None;
    let mut profile_out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                out = args.get(i).cloned();
            }
            s if s.starts_with("--emit=") => emit = Some(s["--emit=".len()..].to_owned()),
            s if s.starts_with("--self-profile=") => {
                profile_out = Some(PathBuf::from(&s["--self-profile=".len()..]));
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
            "usage: rurixc <input.rx> [-o <out.exe>] [--emit=check|mir|llvm-ir] [--self-profile=<file.json>]"
        );
        return ExitCode::from(2);
    };
    ExitCode::from(driver::compile(&CompileOptions {
        input: PathBuf::from(input),
        out: out.map(PathBuf::from),
        emit,
        profile_out,
    }))
}
