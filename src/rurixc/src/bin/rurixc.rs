//! rurixc 驱动:`.rx` → EXE + PDB 的端到端 host 编译闭环(M2.3,契约 G-M2-1)。
//!
//! 管线:lex → parse → feature gate → resolve → typeck → TBIR 窄门(逐实例
//! 即建即用,D-202)→ MIR(单态化收集)→ 文本 LLVM IR → clang(pin 22.1.x,
//! D-205)→ COFF .obj → link.exe(D-209)。
//! 阶段化中止:前一阶段有 error 即停(与 UI 通道同口径,M2_PLAN v1.2)。
//!
//! 工具链定位:
//! - clang:`RURIXC_CLANG` 环境变量 > `C:\Program Files\LLVM\bin\clang.exe` > PATH;
//!   版本断言 22.1.x(违例 = RX7001,pin 纪律)。
//! - link.exe:`RURIXC_LINK` > vswhere 定位 VS BuildTools;MSVC/SDK 库目录自动发现。
//!
//! 用法:`rurixc <input.rx> [-o <out.exe>] [--emit=check|mir|llvm-ir] [--self-profile=<file.json>]`
//!
//! `--emit=check`(M3.4):跑全量静态检查(resolve→typeck→穷尽性→const eval→
//! MIR→move/borrow)后即返回,不产 codegen/link 产物——预算 check 延迟计时口径
//! (契约 G-M3-3)。
//!
//! self-profile(D-M2-6,契约 G-M2-4):`--self-profile=<path>` 输出 JSON 行
//! 阶段计时(parse/resolve/typeck/mir/codegen/link + total/memo 汇总,07 §6)。

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use rurixc::codegen::{self, CodegenOpts};
use rurixc::diag::{DiagCtxt, ErrorCode};
use rurixc::feature_gate::check_feature_gates;
use rurixc::lexer::lex;
use rurixc::mir;
use rurixc::parser::parse;
use rurixc::profile::Profiler;
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

    // self-profile 布点(D-M2-6):计时在驱动外层包裹阶段,query 层维持纯函数纪律
    let prof = Profiler::new();
    let t_start = Instant::now();

    let t = Instant::now();
    let tokens = lex(&src, id, Edition::Rx0, &diag);
    let n_tokens = tokens.len() as u64;
    let ast = parse(&src, tokens, id, Edition::Rx0, &diag);
    prof.record(
        "parse",
        t,
        &[("tokens", n_tokens), ("items", ast.items.len() as u64)],
    );
    let cx = QueryCtx::from_ast(ast, &src, id, &diag);

    // 阶段化:parse/gate → resolve → typeck → mir(前段有错即停)
    check_feature_gates(cx.ast(), &diag);
    let mir_bodies = if diag.has_errors() {
        None
    } else {
        let t = Instant::now();
        let res = cx.resolutions();
        prof.record("resolve", t, &[("defs", res.defs.len() as u64)]);
        if diag.has_errors() {
            None
        } else {
            let t = Instant::now();
            cx.check_crate();
            prof.record(
                "typeck",
                t,
                &[("bodies_checked", cx.hir_crate().bodies.len() as u64)],
            );
            if diag.has_errors() {
                None
            } else {
                // 着色 + barrier 骨架(M4.1,RXS-0066/0068):HIR 层,typeck 后、
                // MIR 前;地址空间一致性(RXS-0067)已在 typeck 合一处裁决
                cx.check_coloring();
                // launch 类型契约(M4.3,RXS-0074/0075):同着色层(typeck 后、MIR 前)
                cx.check_launch();
                // 模式穷尽性(RXS-0051):TBIR 窄门时点(typeck 后、MIR 前),
                // 全 body 覆盖(含 MIR 可达性外的 body)
                cx.check_crate_patterns();
                // const 求值强制检查(M3.4,RXS-0062~0065):typeck 后、MIR 前
                if !diag.has_errors() {
                    cx.check_consteval();
                }
                if diag.has_errors() {
                    None
                } else {
                    let t = Instant::now();
                    let m = cx.mir_crate();
                    prof.record("mir", t, &[("mir_bodies", m.len() as u64)]);
                    // TBIR 窄门(M3.1):逐实例即建即用,聚合计时/计数经 QueryCtx 上报
                    let (tb_bodies, tb_scopes, tb_ms) = cx.tbir_stats();
                    prof.record_ms(
                        "tbir",
                        tb_ms,
                        &[("tbir_bodies", tb_bodies), ("tbir_scopes", tb_scopes)],
                    );
                    // move/init 数据流(M3.2,RXS-0054):MIR 后、codegen 前强制
                    if !diag.has_errors() {
                        cx.check_moves();
                    }
                    // NLL 借用检查(M3.3,RXS-0057~0061):move/init 之后强制
                    if !diag.has_errors() {
                        cx.check_borrows();
                    }
                    // views 不相交证明(M5.1,RXS-0078):device 借用扩展,host
                    // 借用检查之后、device codegen 之前(仅 device 上下文 body)
                    if !diag.has_errors() {
                        cx.check_views();
                    }
                    // shared+barrier 一致性(M5.2,RXS-0079):device 借用扩展的
                    // 数据流分析,views 不相交之后、device codegen 之前(仅 device
                    // 上下文 body)
                    if !diag.has_errors() {
                        cx.check_shared_barrier();
                    }
                    // device emit 通道(`--emit=nvptx-ir|ptx`)以 `kernel fn` 为根,
                    // 不要求 host `main`(RXS-0070);其余目标缺 main → RX6002。
                    let device_emit = matches!(emit.as_deref(), Some("nvptx-ir") | Some("ptx"));
                    if m.is_empty() && !device_emit {
                        diag.struct_error(E_MISSING_MAIN, "codegen.missing_main")
                            .emit();
                    }
                    Some(m)
                }
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

    // --emit=check:全量静态检查闭环(resolve/typeck/穷尽性/const eval/MIR/
    // move/borrow 均已跑),不产 codegen/link 产物——check 延迟计时口径(G-M3-3)
    if emit.as_deref() == Some("check") {
        if let Err(e) = finish_profile(&prof, &cx, t_start, profile_out.as_deref()) {
            toolchain_err(&diag, &sm, e);
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    if emit.as_deref() == Some("mir") {
        let res = cx.resolutions();
        for b in mir_bodies.iter() {
            print!("{}", mir::pretty(b, &res));
        }
        if let Err(e) = finish_profile(&prof, &cx, t_start, profile_out.as_deref()) {
            toolchain_err(&diag, &sm, e);
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    // device codegen 通道(M4.2,RXS-0070~0073):`--emit=nvptx-ir`(NVPTX
    // LLVM IR 文本)/ `--emit=ptx`(经 pin 的 clang `--target=nvptx64` 产 PTX +
    // ptxas -arch=sm_89 干验证关卡)。device MIR 以 `kernel fn` 为根收集(独立于
    // host `main`);codegen 失败 → RX6003/RX6005 诊断。
    if emit.as_deref() == Some("nvptx-ir") || emit.as_deref() == Some("ptx") {
        let mode = emit.as_deref().unwrap();
        let ir = rurixc::device_codegen::build_and_emit(&cx, &stem);
        if diag.has_errors() {
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), &sm, diag.messages())
            );
            return ExitCode::from(1);
        }
        let Some(ir) = ir else {
            eprintln!("rurixc: no `kernel fn` found; nothing to emit for --emit={mode}");
            return ExitCode::from(2);
        };
        if mode == "nvptx-ir" {
            print!("{ir}");
            return ExitCode::SUCCESS;
        }
        // --emit=ptx:IR → PTX(clang NVPTX 后端)+ ptxas 干验证关卡
        let ptx_out = out
            .map(PathBuf::from)
            .unwrap_or_else(|| input_path.with_extension("ptx"));
        match emit_ptx_and_gate(&ir, &stem, &ptx_out) {
            Ok(PtxGate::Ok(ptx)) => {
                print!("{ptx}");
                return ExitCode::SUCCESS;
            }
            Ok(PtxGate::SkippedNoPtxas(ptx)) => {
                print!("{ptx}");
                eprintln!(
                    "rurixc: note: ptxas not found (no CUDA toolchain); ptxas -arch=sm_89 dry-gate SKIPPED (RXS-0073)"
                );
                return ExitCode::SUCCESS;
            }
            Err(PtxError::Rejected { reason }) => {
                // ptxas 拒绝 = RX6004 编译期诊断(RXS-0073,G-M4-4)
                diag.struct_error(ErrorCode(6004), "codegen.ptxas_rejected")
                    .arg("reason", reason)
                    .emit();
                eprint!(
                    "{}",
                    render_diagnostics(&diag.emitted(), &sm, diag.messages())
                );
                return ExitCode::from(1);
            }
            Err(PtxError::Toolchain(e)) => {
                toolchain_err(&diag, &sm, e);
                return ExitCode::from(1);
            }
        }
    }

    let t = Instant::now();
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
        prof.record("codegen", t, &[("ir_bytes", ir.len() as u64)]);
        print!("{ir}");
        if let Err(e) = finish_profile(&prof, &cx, t_start, profile_out.as_deref()) {
            toolchain_err(&diag, &sm, e);
            return ExitCode::from(1);
        }
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
    let clang = match rurixc::toolchain::locate_clang() {
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
    // codegen 阶段 = LLVM IR 生成 + .ll 落盘 + clang → .obj(文本 IR 通道,M2_PLAN v1.3)
    prof.record("codegen", t, &[("ir_bytes", ir.len() as u64)]);

    // link.exe(D-209:默认 link.exe;PDB 经 /debug:full)
    let t = Instant::now();
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
    let pdb = exe.with_extension("pdb");
    let artifacts = [&exe, &pdb].iter().filter(|p| p.exists()).count() as u64;
    prof.record("link", t, &[("artifacts", artifacts)]);

    if let Err(e) = finish_profile(&prof, &cx, t_start, profile_out.as_deref()) {
        toolchain_err(&diag, &sm, e);
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// 收尾 self-profile:追加 `total` 行(memo 汇总)并按需落盘 JSON 行文件。
fn finish_profile(
    prof: &Profiler,
    cx: &QueryCtx<'_>,
    t_start: Instant,
    path: Option<&Path>,
) -> Result<(), String> {
    prof.record(
        "total",
        t_start,
        &[
            ("memo_hits", cx.memo_hits()),
            ("memo_misses", cx.memo_misses()),
        ],
    );
    if let Some(p) = path {
        std::fs::write(p, prof.to_json_lines())
            .map_err(|e| format!("cannot write self-profile {}: {e}", p.display()))?;
    }
    Ok(())
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

/// ptxas 干验证关卡结果(RXS-0073,G-M4-4)。
enum PtxGate {
    /// PTX 过 `ptxas -arch=sm_89` 干验证(或无 ptxas 时仅产 PTX 不验证)。
    Ok(String),
    /// 无 CUDA 工具链(ptxas 缺失):关卡 SKIP(开发环境降级,真实红绿在 CI runner)。
    SkippedNoPtxas(String),
}

enum PtxError {
    /// ptxas 拒绝 PTX(RX6004)。
    Rejected { reason: String },
    /// 工具链失败(clang 定位/版本/退出非零;ptxas 定位失败归 RX7001)。
    Toolchain(String),
}

/// device IR → PTX(clang NVPTX 后端)+ ptxas -arch=sm_89 干验证关卡(RXS-0073)。
///
/// IR→PTX 经 [`rurixc::toolchain::ir_to_ptx`](pin 的 clang `--target=nvptx64`,
/// bin 与 `rurix-rt` build.rs 复用单一事实源)。ptxas 缺失 → 关卡 SKIP(M4
/// CI_GATES §1:真实红绿延到带 CUDA 的 CI runner)。非 ASCII 路径防御(r6 教训)
/// 在 [`rurixc::ptxas::dry_gate`] 内(ASCII 临时目录)。
fn emit_ptx_and_gate(ir: &str, stem: &str, ptx_out: &Path) -> Result<PtxGate, PtxError> {
    let ptx = rurixc::toolchain::ir_to_ptx(ir, ptx_out).map_err(PtxError::Toolchain)?;

    // ptxas 干验证关卡(strict-only;RXS-0073,关卡逻辑在 rurixc::ptxas,供红绿单测复用)
    match rurixc::ptxas::dry_gate(&ptx, stem) {
        rurixc::ptxas::PtxasOutcome::Pass => Ok(PtxGate::Ok(ptx)),
        rurixc::ptxas::PtxasOutcome::Skipped => Ok(PtxGate::SkippedNoPtxas(ptx)),
        rurixc::ptxas::PtxasOutcome::Rejected(reason) => Err(PtxError::Rejected { reason }),
        rurixc::ptxas::PtxasOutcome::Toolchain(e) => Err(PtxError::Toolchain(e)),
    }
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
