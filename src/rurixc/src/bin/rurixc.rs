//! rurixc 驱动:`.rx` → EXE + PDB 的端到端 host 编译闭环(M2.3,契约 G-M2-1)。
//!
//! 管线:lex → parse → feature gate → resolve → typeck → MIR(单态化收集)
//! → 文本 LLVM IR → clang(pin 22.1.x,D-205)→ COFF .obj → link.exe(D-209)。
//! 阶段化中止:前一阶段有 error 即停(与 UI 通道同口径,M2_PLAN v1.2)。
//!
//! 工具链定位:
//! - clang:`RURIXC_CLANG` 环境变量 > `C:\Program Files\LLVM\bin\clang.exe` > PATH;
//!   版本断言 22.1.x(违例 = RX7001,pin 纪律)。
//! - link.exe:`RURIXC_LINK` > vswhere 定位 VS BuildTools;MSVC/SDK 库目录自动发现。
//!
//! 用法:`rurixc <input.rx> [-o <out.exe>] [--emit=mir|llvm-ir]`

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use rurixc::codegen::{self, CodegenOpts};
use rurixc::diag::{DiagCtxt, ErrorCode};
use rurixc::feature_gate::check_feature_gates;
use rurixc::mir;
use rurixc::query::QueryCtx;
use rurixc::render::render_diagnostics;
use rurixc::source_map::SourceMap;
use rurixc::span::Edition;

const E_MISSING_MAIN: ErrorCode = ErrorCode(6002); // RX6002
const E_TOOLCHAIN: ErrorCode = ErrorCode(7001); // RX7001

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<String> = None;
    let mut out: Option<String> = None;
    let mut emit: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                out = args.get(i).cloned();
            }
            s if s.starts_with("--emit=") => emit = Some(s["--emit=".len()..].to_owned()),
            s if !s.starts_with('-') && input.is_none() => input = Some(s.to_owned()),
            s => {
                eprintln!("rurixc: unknown argument `{s}`");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let Some(input) = input else {
        eprintln!("usage: rurixc <input.rx> [-o <out.exe>] [--emit=mir|llvm-ir]");
        return ExitCode::from(2);
    };
    let input_path = PathBuf::from(&input);
    let src = match std::fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rurixc: cannot read {input}: {e}");
            return ExitCode::from(2);
        }
    };
    let file_name = input_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| input.clone());
    let stem = input_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_owned());
    let directory = input_path
        .canonicalize()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        // Windows canonicalize 产生 \\?\ 长路径前缀;剥除以保 PDB 源路径
        // 可被 cdb/WinDbg 源行断点匹配(D-237)
        .map(|d| d.trim_start_matches("\\\\?\\").to_owned())
        .unwrap_or_else(|| ".".to_owned());

    let diag = DiagCtxt::new();
    let mut sm = SourceMap::new();
    let id = sm.add_file(file_name.clone(), src.as_str(), Edition::Rx0);
    let cx = QueryCtx::new(&src, id, Edition::Rx0, &diag);

    // 阶段化:parse/gate → resolve → typeck → mir(前段有错即停)
    check_feature_gates(cx.ast(), &diag);
    let mir_bodies = if diag.has_errors() {
        None
    } else {
        let _ = cx.resolutions();
        if diag.has_errors() {
            None
        } else {
            cx.check_crate();
            if diag.has_errors() {
                None
            } else {
                let m = cx.mir_crate();
                if m.is_empty() {
                    diag.struct_error(E_MISSING_MAIN, "codegen.missing_main")
                        .emit();
                }
                Some(m)
            }
        }
    };
    if diag.has_errors() {
        eprint!(
            "{}",
            render_diagnostics(&diag.emitted(), &sm, diag.messages())
        );
        return ExitCode::from(1);
    }
    let mir_bodies = mir_bodies.expect("无错误则 MIR 存在");

    if emit.as_deref() == Some("mir") {
        let res = cx.resolutions();
        for b in mir_bodies.iter() {
            print!("{}", mir::pretty(b, &res));
        }
        return ExitCode::SUCCESS;
    }

    let krate = cx.hir_crate();
    let ir = codegen::emit_llvm_ir(
        &mir_bodies,
        &krate,
        &sm,
        &CodegenOpts {
            module_name: &stem,
            file_name: &file_name,
            directory: &directory,
        },
    );
    if emit.as_deref() == Some("llvm-ir") {
        print!("{ir}");
        return ExitCode::SUCCESS;
    }

    // 产物路径:exe 由 -o 指定(默认与输入同目录同名);.ll/.obj 随 exe 落同目录
    let exe = out
        .map(PathBuf::from)
        .unwrap_or_else(|| input_path.with_extension("exe"));
    let ll = exe.with_extension("ll");
    let obj = exe.with_extension("obj");
    if let Err(e) = std::fs::write(&ll, &ir) {
        toolchain_err(&diag, &sm, format!("cannot write {}: {e}", ll.display()));
        return ExitCode::from(1);
    }

    // clang(pin 22.1.x 核对,D-205)
    let clang = match locate_clang() {
        Ok(c) => c,
        Err(e) => {
            toolchain_err(&diag, &sm, e);
            return ExitCode::from(1);
        }
    };
    if let Err(e) = run_tool(
        Command::new(&clang)
            .arg("-c")
            .arg(&ll)
            .arg("-o")
            .arg(&obj)
            .arg("-g")
            .arg("--target=x86_64-pc-windows-msvc"),
        "clang",
    ) {
        toolchain_err(&diag, &sm, e);
        return ExitCode::from(1);
    }

    // link.exe(D-209:默认 link.exe;PDB 经 /debug:full)
    let (link, libpaths) = match locate_link() {
        Ok(v) => v,
        Err(e) => {
            toolchain_err(&diag, &sm, e);
            return ExitCode::from(1);
        }
    };
    let mut cmd = Command::new(&link);
    cmd.arg("/nologo")
        .arg("/subsystem:console")
        .arg("/debug:full")
        .arg(format!("/out:{}", exe.display()))
        .arg(&obj)
        .arg("libcmt.lib")
        .arg("libucrt.lib")
        .arg("libvcruntime.lib")
        .arg("kernel32.lib");
    for p in &libpaths {
        cmd.arg(format!("/libpath:{}", p.display()));
    }
    if let Err(e) = run_tool(&mut cmd, "link.exe") {
        toolchain_err(&diag, &sm, e);
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn toolchain_err(diag: &DiagCtxt, sm: &SourceMap, reason: String) {
    diag.struct_error(E_TOOLCHAIN, "link.toolchain_failure")
        .arg("reason", reason)
        .emit();
    eprint!(
        "{}",
        render_diagnostics(&diag.emitted(), sm, diag.messages())
    );
}

fn run_tool(cmd: &mut Command, name: &str) -> Result<(), String> {
    match cmd.output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!(
            "{name} exited with {}: {}{}",
            o.status,
            String::from_utf8_lossy(&o.stdout).trim(),
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("cannot spawn {name}: {e}")),
    }
}

/// clang 定位 + pin 22.1.x 断言(D-205;M2_PLAN v1.3 选型留痕)。
fn locate_clang() -> Result<PathBuf, String> {
    let candidates: Vec<PathBuf> = [
        std::env::var("RURIXC_CLANG").ok(),
        Some("C:\\Program Files\\LLVM\\bin\\clang.exe".to_owned()),
        Some("clang".to_owned()),
    ]
    .into_iter()
    .flatten()
    .map(PathBuf::from)
    .collect();
    for c in candidates {
        let Ok(out) = Command::new(&c).arg("--version").output() else {
            continue;
        };
        if !out.status.success() {
            continue;
        }
        let ver = String::from_utf8_lossy(&out.stdout);
        if ver.contains("clang version 22.1.") {
            return Ok(c);
        }
        return Err(format!(
            "clang at {} is not the pinned 22.1.x (D-205): {}",
            c.display(),
            ver.lines().next().unwrap_or("")
        ));
    }
    Err("clang not found (install LLVM 22.1.x or set RURIXC_CLANG)".to_owned())
}

/// link.exe 与 MSVC/SDK 库目录定位(vswhere;无 vcvars 依赖)。
fn locate_link() -> Result<(PathBuf, Vec<PathBuf>), String> {
    // 库目录:MSVC lib + SDK ucrt/um
    let mut libpaths = Vec::new();
    let vs_root = if let Ok(p) = std::env::var("RURIXC_VS_ROOT") {
        PathBuf::from(p)
    } else {
        let vswhere = PathBuf::from(
            "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe",
        );
        if !vswhere.exists() {
            return Err("vswhere.exe not found (install VS Build Tools)".to_owned());
        }
        let out = Command::new(&vswhere)
            .args([
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-property",
                "installationPath",
            ])
            .output()
            .map_err(|e| format!("cannot run vswhere: {e}"))?;
        let p = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        if p.is_empty() {
            return Err("no VS installation with VC tools found".to_owned());
        }
        PathBuf::from(p)
    };
    let msvc_root = vs_root.join("VC\\Tools\\MSVC");
    let msvc_ver = newest_subdir(&msvc_root)
        .ok_or_else(|| format!("no MSVC toolset under {}", msvc_root.display()))?;
    let link = if let Ok(p) = std::env::var("RURIXC_LINK") {
        PathBuf::from(p)
    } else {
        msvc_ver.join("bin\\Hostx64\\x64\\link.exe")
    };
    if !link.exists() {
        return Err(format!("link.exe not found at {}", link.display()));
    }
    libpaths.push(msvc_ver.join("lib\\x64"));

    let sdk_lib = PathBuf::from("C:\\Program Files (x86)\\Windows Kits\\10\\Lib");
    let sdk_ver = newest_subdir(&sdk_lib)
        .ok_or_else(|| format!("no Windows SDK libs under {}", sdk_lib.display()))?;
    libpaths.push(sdk_ver.join("ucrt\\x64"));
    libpaths.push(sdk_ver.join("um\\x64"));
    for p in &libpaths {
        if !p.exists() {
            return Err(format!("library path missing: {}", p.display()));
        }
    }
    Ok((link, libpaths))
}

/// 目录下按名称排序最大的子目录(MSVC/SDK 版本目录形态)。
fn newest_subdir(dir: &Path) -> Option<PathBuf> {
    let mut subs: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    subs.sort();
    subs.pop()
}
