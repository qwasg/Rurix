//! 编译驱动库面(M6.1:从 `bin/rurixc.rs` 抽出,供 rurixc 驱动与 rx CLI 复用)。
//!
//! 单一前端纪律(07 §2,D-203):rurixc 驱动与 `rx build/run/check` 子命令都经本
//! 模块 [`compile`] 复用同一 query 层管线,**不另起引擎**。行为相对既有 rurixc
//! 驱动零语义漂移(既有 golden / hello-world 冒烟不变)。
//!
//! 管线:lex → parse → feature gate → resolve → typeck → 着色/launch/穷尽性 →
//! const eval → MIR(单态化收集)→ move/borrow/views/shared → 文本 LLVM IR →
//! clang(pin 22.1.x,D-205)→ COFF .obj → link.exe(D-209)。
//! 阶段化中止:前一阶段有 error 即停(与 UI 通道同口径)。
//!
//! `--emit=check`(G-M3-3):跑全量静态检查后即返回,不产 codegen/link 产物。
//! device 通道(`--emit=nvptx-ir|ptx`,RXS-0070~0073/RXS-0082):以 `kernel fn`
//! 为根,经 pin 的 clang NVPTX 后端 + ptxas 干验证关卡。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::codegen::{self, CodegenOpts};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::feature_gate::check_feature_gates;
use crate::lexer::lex;
use crate::mir;
use crate::parser::parse;
use crate::profile::Profiler;
use crate::query::QueryCtx;
use crate::render::render_diagnostics;
use crate::source_map::SourceMap;
use crate::span::Edition;

const E_MISSING_MAIN: ErrorCode = ErrorCode(6002); // RX6002
const E_TOOLCHAIN: ErrorCode = ErrorCode(7001); // RX7001

/// 编译选项(rurixc 驱动 argv 与 rx 子命令分发都构造此结构后调 [`compile`])。
pub struct CompileOptions {
    /// 输入 `.rx` 源文件路径。
    pub input: PathBuf,
    /// 输出产物路径(`-o`);None 时默认与输入同目录同名(EXE/PTX 各自扩展名)。
    pub out: Option<PathBuf>,
    /// emit 目标:`check`/`mir`/`llvm-ir`/`nvptx-ir`/`ptx`;None = 默认 host EXE。
    pub emit: Option<String>,
    /// self-profile JSON 行输出路径(`--self-profile`);None 时不落盘。
    pub profile_out: Option<PathBuf>,
    /// 可复现 host 产物模式(RXS-0097):关闭 debug link/PDB 并启用链接器
    /// `/Brepro`;普通 debug build 保持原 PDB/source path 语义。
    pub reproducible: bool,
    /// 诊断输出格式:`json` 时输出 07 §5 结构化 JSON(RXS-0099);默认文本。
    pub error_format: Option<String>,
    /// codegen 目标后端(RXS-0157,RFC-0003 §9 Q-CLI):`Some("dxil")` 选 DXIL
    /// 第二后端(MIR 之后 target 分叉,gate `dxil-backend`);`None`/`Some("ptx")`
    /// 维持现状默认 host/PTX 通道(零语义漂移)。
    pub target: Option<String>,
}

/// 端到端编译(单一前端,07 §2)。返回退出码(`u8`,供调用方 [`std::process::ExitCode::from`]
/// 与 rx run 成功判定复用):0 成功 / 1 诊断或工具链错误 / 2 I/O 错误(输入不可读 /
/// device 通道无 kernel)。
pub fn compile(opts: &CompileOptions) -> u8 {
    let input_path = opts.input.clone();
    let input = input_path.to_string_lossy().into_owned();
    let out = opts.out.clone();
    let emit = opts.emit.clone();
    let profile_out = opts.profile_out.clone();
    let json_out = opts.error_format.as_deref() == Some("json");
    let target = opts.target.clone();

    let src = match std::fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rurixc: cannot read {input}: {e}");
            return 2;
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
            // 着色阶段类型面(G2.1,RXS-0153~0156):AST 层,cargo feature
            // `shader-stages`;着色阶段误用 / 阶段间接口 / 资源句柄 100% 编译期拦截
            // (RX3011~3013)。**resolve 后、typeck 前**:资源句柄位置违例须在 typeck
            // body↔返回类型匹配前裁决——否则非法句柄返回类型(`-> Texture2D<F>`)会先触
            // 类型不匹配 RX2001 掩盖 spec 强制的 RX3013(RXS-0156)。直接调用着色阶段入口
            // 复用 RX3001,经下方 check_coloring(typeck 后)。
            cx.check_shader_stages();
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
                        // device emit 通道(`--emit=nvptx-ir|ptx|pyd`)以 `kernel fn` 为根,
                        // 不要求 host `main`(RXS-0070 / 互操作 PYD RXS-0122);其余缺 main → RX6002。
                        let device_emit = matches!(
                            emit.as_deref(),
                            Some("nvptx-ir") | Some("ptx") | Some("pyd")
                        ) || target.as_deref() == Some("dxil");
                        if m.is_empty() && !device_emit {
                            diag.struct_error(E_MISSING_MAIN, "codegen.missing_main")
                                .emit();
                        }
                        Some(m)
                    }
                }
            }
        }
    };
    if diag.has_errors() {
        if json_out {
            println!(
                "{}",
                crate::tooling::diag_json::diags_to_json(&diag.emitted(), &sm, diag.messages())
            );
        } else {
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), &sm, diag.messages())
            );
        }
        return 1;
    }
    let mir_bodies = mir_bodies.expect("无错误则 MIR 存在");

    // --emit=check:全量静态检查闭环(resolve/typeck/穷尽性/const eval/MIR/
    // move/borrow 均已跑),不产 codegen/link 产物——check 延迟计时口径(G-M3-3)
    if emit.as_deref() == Some("check") {
        if json_out {
            println!(
                "{}",
                crate::tooling::diag_json::diags_to_json(&diag.emitted(), &sm, diag.messages())
            );
        }
        if let Err(e) = finish_profile(&prof, &cx, t_start, profile_out.as_deref()) {
            toolchain_err(&diag, &sm, e);
            return 1;
        }
        return 0;
    }

    if emit.as_deref() == Some("mir") {
        let res = cx.resolutions();
        for b in mir_bodies.iter() {
            print!("{}", mir::pretty(b, &res));
        }
        if let Err(e) = finish_profile(&prof, &cx, t_start, profile_out.as_deref()) {
            toolchain_err(&diag, &sm, e);
            return 1;
        }
        return 0;
    }

    // DXIL 第二后端 target 分发(G2.2,RXS-0157;RFC-0003 §4.1 MIR 之后分叉)。
    // `--target dxil`:device MIR(kernel 根)→ DirectX 三元组 LLVM IR → patched llc
    // -filetype=obj → DXIL 容器 → dxc validator accept。target 分叉不改 PTX 路径
    // (D-207,§4.5)。feature `dxil-backend` 未启用 → RX6007(L1 后端不可用)。
    if target.as_deref() == Some("dxil") {
        return compile_dxil_target(&diag, &sm, &cx, &stem, out.as_deref(), &input_path);
    }

    // device codegen 通道(M4.2,RXS-0070~0073):`--emit=nvptx-ir` / `--emit=ptx`。
    if emit.as_deref() == Some("nvptx-ir") || emit.as_deref() == Some("ptx") {
        let mode = emit.as_deref().unwrap();
        let ir = crate::device_codegen::build_and_emit(&cx, &stem);
        if diag.has_errors() {
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), &sm, diag.messages())
            );
            return 1;
        }
        let Some(ir) = ir else {
            eprintln!("rurixc: no `kernel fn` found; nothing to emit for --emit={mode}");
            return 2;
        };
        if mode == "nvptx-ir" {
            print!("{ir}");
            return 0;
        }
        // --emit=ptx:IR → PTX(clang NVPTX 后端)+ ptxas 干验证关卡
        let ptx_out = out
            .clone()
            .unwrap_or_else(|| input_path.with_extension("ptx"));
        match emit_ptx_and_gate(&ir, &stem, &ptx_out) {
            Ok(PtxGate::Ok(ptx)) => {
                print!("{ptx}");
                return 0;
            }
            Ok(PtxGate::SkippedNoPtxas(ptx)) => {
                print!("{ptx}");
                eprintln!(
                    "rurixc: note: ptxas not found (no CUDA toolchain); ptxas -arch=sm_89 dry-gate SKIPPED (RXS-0073)"
                );
                return 0;
            }
            Ok(PtxGate::SkippedNoLibdevice) => {
                eprintln!(
                    "rurixc: note: libdevice.10.bc not found (no CUDA toolchain); no PTX emitted (libdevice unavailable); libdevice link + PTX emit SKIPPED (RXS-0082)"
                );
                return 0;
            }
            Err(PtxError::LibdeviceLink(reason)) => {
                // libdevice bc 链接失败 = RX7002 编译期诊断(RXS-0082)
                diag.struct_error(ErrorCode(7002), "link.libdevice_failure")
                    .arg("reason", reason)
                    .emit();
                eprint!(
                    "{}",
                    render_diagnostics(&diag.emitted(), &sm, diag.messages())
                );
                return 1;
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
                return 1;
            }
            Err(PtxError::Toolchain(e)) => {
                toolchain_err(&diag, &sm, e);
                return 1;
            }
        }
    }

    // 互操作 PYD 通道(M8.1,RXS-0122):`--emit=pyd`。编译器侧把输入 device
    // `kernel fn` 全管线产 PTX(与 --emit=ptx 同管线:device codegen + ptxas 干验证),
    // 写入 staging PTX 文件供 PYD 打包消费;PYD 打包(nanobind + scikit-build-core,
    // 链接 rurix-interop 运行时)由 `rx build --emit=pyd` 编排。输入无 `kernel fn`
    // → 互操作协议不支持 RX7013(无可导出零拷贝算子源,RXS-0122/0125)。
    if emit.as_deref() == Some("pyd") {
        let ir = crate::device_codegen::build_and_emit(&cx, &stem);
        if diag.has_errors() {
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), &sm, diag.messages())
            );
            return 1;
        }
        let Some(ir) = ir else {
            diag.struct_error(ErrorCode(7013), "interop.unsupported_protocol")
                .arg(
                    "detail",
                    "输入无 `kernel fn`;--emit=pyd 需 device kernel 作为零拷贝算子源",
                )
                .emit();
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), &sm, diag.messages())
            );
            return 1;
        };
        let ptx_out = out
            .clone()
            .unwrap_or_else(|| input_path.with_extension("ptx"));
        match emit_ptx_and_gate(&ir, &stem, &ptx_out) {
            Ok(PtxGate::Ok(_)) | Ok(PtxGate::SkippedNoPtxas(_)) => {
                eprintln!(
                    "rurixc: --emit=pyd: device kernel 编译为 PTX 完成({});PYD 打包(nanobind + scikit-build-core,链接 rurix-interop)由 rx build 编排(RXS-0122)",
                    ptx_out.display()
                );
                return 0;
            }
            Ok(PtxGate::SkippedNoLibdevice) => {
                eprintln!(
                    "rurixc: note: libdevice.10.bc not found (no CUDA toolchain); --emit=pyd device PTX SKIPPED (RXS-0082)"
                );
                return 0;
            }
            Err(PtxError::LibdeviceLink(reason)) => {
                diag.struct_error(ErrorCode(7002), "link.libdevice_failure")
                    .arg("reason", reason)
                    .emit();
                eprint!(
                    "{}",
                    render_diagnostics(&diag.emitted(), &sm, diag.messages())
                );
                return 1;
            }
            Err(PtxError::Rejected { reason }) => {
                diag.struct_error(ErrorCode(6004), "codegen.ptxas_rejected")
                    .arg("reason", reason)
                    .emit();
                eprint!(
                    "{}",
                    render_diagnostics(&diag.emitted(), &sm, diag.messages())
                );
                return 1;
            }
            Err(PtxError::Toolchain(e)) => {
                toolchain_err(&diag, &sm, e);
                return 1;
            }
        }
    }

    // 未知 emit 目标拒绝(M8.1:避免未知 --emit 静默落入 host EXE 路径)。
    if let Some(target) = emit.as_deref()
        && !matches!(
            target,
            "check" | "mir" | "nvptx-ir" | "ptx" | "llvm-ir" | "pyd"
        )
    {
        toolchain_err(
            &diag,
            &sm,
            format!("未知 --emit 目标 `{target}`(合法:check/mir/llvm-ir/nvptx-ir/ptx/pyd)"),
        );
        return 1;
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
            return 1;
        }
        return 0;
    }

    // 产物路径:exe 由 -o 指定(默认与输入同目录同名);.ll/.obj 随 exe 落同目录
    let exe = out
        .clone()
        .unwrap_or_else(|| input_path.with_extension("exe"));
    let ll = exe.with_extension("ll");
    let obj = exe.with_extension("obj");
    if let Err(e) = std::fs::write(&ll, &ir) {
        toolchain_err(&diag, &sm, format!("cannot write {}: {e}", ll.display()));
        return 1;
    }

    // clang(pin 22.1.x 核对,D-205)
    let clang = match crate::toolchain::locate_clang() {
        Ok(c) => c,
        Err(e) => {
            toolchain_err(&diag, &sm, e);
            return 1;
        }
    };
    let mut clang_cmd = Command::new(&clang);
    clang_cmd
        .arg("-c")
        .arg(&ll)
        .arg("-o")
        .arg(&obj)
        .arg("--target=x86_64-pc-windows-msvc");
    if !opts.reproducible {
        clang_cmd.arg("-g");
    }
    if let Err(e) = run_tool(&mut clang_cmd, "clang") {
        toolchain_err(&diag, &sm, e);
        return 1;
    }
    // codegen 阶段 = LLVM IR 生成 + .ll 落盘 + clang → .obj(文本 IR 通道,M2_PLAN v1.3)
    prof.record("codegen", t, &[("ir_bytes", ir.len() as u64)]);

    // link.exe(D-209:默认 link.exe;PDB 经 /debug:full)
    let t = Instant::now();
    let (link, libpaths) = match locate_link() {
        Ok(v) => v,
        Err(e) => {
            toolchain_err(&diag, &sm, e);
            return 1;
        }
    };
    let mut cmd = Command::new(&link);
    cmd.arg("/nologo")
        .arg("/subsystem:console")
        .arg(format!("/out:{}", exe.display()))
        .arg(&obj)
        .arg("libcmt.lib")
        .arg("libucrt.lib")
        .arg("libvcruntime.lib")
        .arg("kernel32.lib");
    if opts.reproducible {
        cmd.arg("/Brepro");
    } else {
        cmd.arg("/debug:full");
    }
    for p in &libpaths {
        cmd.arg(format!("/libpath:{}", p.display()));
    }
    if let Err(e) = run_tool(&mut cmd, "link.exe") {
        toolchain_err(&diag, &sm, e);
        return 1;
    }
    let pdb = exe.with_extension("pdb");
    let artifacts = [&exe, &pdb].iter().filter(|p| p.exists()).count() as u64;
    prof.record("link", t, &[("artifacts", artifacts)]);

    if let Err(e) = finish_profile(&prof, &cx, t_start, profile_out.as_deref()) {
        toolchain_err(&diag, &sm, e);
        return 1;
    }
    0
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
    /// IR 用到 libdevice `__nv_*` 但 bc 缺失(无 CUDA 工具链):链接 + 产 PTX
    /// SKIP(开发环境降级,真实红绿在带 CUDA 的 CI runner,RXS-0082)。
    SkippedNoLibdevice,
}

enum PtxError {
    /// ptxas 拒绝 PTX(RX6004)。
    Rejected { reason: String },
    /// libdevice bc 链接失败(bc 在却 clang 链接退出非零;RX7002,RXS-0082)。
    LibdeviceLink(String),
    /// 工具链失败(clang 定位/版本/退出非零;ptxas 定位失败归 RX7001)。
    Toolchain(String),
}

/// device IR → PTX(clang NVPTX 后端)+ ptxas -arch=sm_89 干验证关卡(RXS-0073)。
///
/// IR→PTX 经 [`crate::toolchain::ir_to_ptx`](pin 的 clang `--target=nvptx64`,
/// driver 与 `rurix-rt` build.rs 复用单一事实源)。ptxas 缺失 → 关卡 SKIP(M4
/// CI_GATES §1:真实红绿延到带 CUDA 的 CI runner)。非 ASCII 路径防御(r6 教训)
/// 在 [`crate::ptxas::dry_gate`] 内(ASCII 临时目录)。
fn emit_ptx_and_gate(ir: &str, stem: &str, ptx_out: &Path) -> Result<PtxGate, PtxError> {
    // libdevice 链接裁决(RXS-0082):用到 `__nv_*` 但 bc 缺失 → 开发环境降级 SKIP
    // (不报 RX7002);bc 在却 clang 链接失败 → RX7002。
    let needs_libdevice = match crate::toolchain::libdevice_link_for(ir) {
        crate::toolchain::LibdeviceLink::MissingSkip => return Ok(PtxGate::SkippedNoLibdevice),
        crate::toolchain::LibdeviceLink::Linked(_) => true,
        crate::toolchain::LibdeviceLink::NotNeeded => false,
    };
    let ptx = crate::toolchain::ir_to_ptx(ir, ptx_out).map_err(|e| {
        if needs_libdevice {
            PtxError::LibdeviceLink(e)
        } else {
            PtxError::Toolchain(e)
        }
    })?;

    // ptxas 干验证关卡(strict-only;RXS-0073,关卡逻辑在 crate::ptxas,供红绿单测复用)
    match crate::ptxas::dry_gate(&ptx, stem) {
        crate::ptxas::PtxasOutcome::Pass => Ok(PtxGate::Ok(ptx)),
        crate::ptxas::PtxasOutcome::Skipped => Ok(PtxGate::SkippedNoPtxas(ptx)),
        crate::ptxas::PtxasOutcome::Rejected(reason) => Err(PtxError::Rejected { reason }),
        crate::ptxas::PtxasOutcome::Toolchain(e) => Err(PtxError::Toolchain(e)),
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

/// `--target dxil` 端到端(RXS-0157;feature `dxil-backend` 启用)。device MIR
/// (kernel 根)→ DirectX 三元组 LLVM IR(`dxil_codegen`)→ patched llc -filetype=obj
/// → DXIL 容器 → dxc validator accept。无 kernel → 退出码 2;子集外 / 降级失败 →
/// RX6007;patched llc / validator 缺失 → SKIP(开发环境降级,真实红绿在带工具链环境,
/// 对齐 RXS-0073 ptxas 干验证 SKIP 纪律)。
#[cfg(feature = "dxil-backend")]
fn compile_dxil_target(
    diag: &DiagCtxt,
    sm: &SourceMap,
    cx: &QueryCtx<'_>,
    stem: &str,
    out: Option<&Path>,
    input_path: &Path,
) -> u8 {
    let artifacts = crate::dxil_codegen::build_and_emit_dxil_artifacts(cx, stem);
    if diag.has_errors() {
        eprint!(
            "{}",
            render_diagnostics(&diag.emitted(), sm, diag.messages())
        );
        return 1;
    }
    let Some(artifacts) = artifacts else {
        eprintln!("rurixc: no compute `kernel fn` found; nothing to emit for --target dxil");
        return 2;
    };
    let ir = artifacts.ir;
    // DXIL 主产物路径(`-o`;compile_offline 传 `<pass>.dxil`)。llc 可用 → 容器 obj
    // 落此;llc SKIP → 直接落 DirectX 三元组 LLVM IR 文本(GRX-009:离线 artifact 须
    // 真实存在但仍按 artifact_kind/semantic_status 区分 debug evidence 与 compile-ready
    // container;probe/schema 不得把 IR 文本误判为 validator 通过的 DXIL 容器)。
    let obj_out = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| input_path.with_extension("dxc"));

    // 派生 root signature(RTS0)与 descriptor layout JSON 路径(与主产物同目录、
    // 同 file stem;compile_offline 期望 `<pass>.rts0.bin` / `<pass>_descriptor_layout.json`)。
    let file_stem = obj_out
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| stem.to_owned());
    let rts0_out = obj_out.with_file_name(format!("{file_stem}.rts0.bin"));
    let descriptor_out = obj_out.with_file_name(format!("{file_stem}_descriptor_layout.json"));

    // 先落 root signature + descriptor layout(host/safe 推导产物,无需工具链)。
    if let Err(e) = std::fs::write(&rts0_out, &artifacts.root_signature) {
        diag.struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
            .arg(
                "detail",
                format!("写 root signature {} 失败: {e}", rts0_out.display()),
            )
            .emit();
        eprint!(
            "{}",
            render_diagnostics(&diag.emitted(), sm, diag.messages())
        );
        return 1;
    }
    if let Err(e) = std::fs::write(&descriptor_out, artifacts.descriptor_layout_json.as_bytes()) {
        diag.struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
            .arg(
                "detail",
                format!(
                    "写 descriptor layout {} 失败: {e}",
                    descriptor_out.display()
                ),
            )
            .emit();
        eprint!(
            "{}",
            render_diagnostics(&diag.emitted(), sm, diag.messages())
        );
        return 1;
    }

    let Some(llc) = crate::toolchain::locate_llc() else {
        // llc 缺失:落 DXIL LLVM IR 文本作调试证据(SKIP 容器 emit + validator)。
        if let Err(e) = std::fs::write(&obj_out, ir.as_bytes()) {
            diag.struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
                .arg(
                    "detail",
                    format!("写 DXIL IR {} 失败: {e}", obj_out.display()),
                )
                .emit();
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), sm, diag.messages())
            );
            return 1;
        }
        eprintln!(
            "rurixc: note: patched llc not found (set RURIX_LLC to dev DXIL llc, RD-011); wrote artifact_kind=dxil_ir_text + RTS0 root signature + descriptor layout as debug/blocker evidence, semantic_status=dxil_ir_only, but DXIL container emit + dxc validator gate SKIPPED (RXS-0157); this is not current compile-ready evidence and not a validated DXIL container"
        );
        return 0;
    };
    if let Err(e) = crate::toolchain::llc_emit_dxil(&llc, &ir, &obj_out) {
        diag.struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
            .arg("detail", format!("patched llc DXIL emit failed: {e}"))
            .emit();
        eprint!(
            "{}",
            render_diagnostics(&diag.emitted(), sm, diag.messages())
        );
        return 1;
    }
    let Some(dxc_dir) = crate::toolchain::locate_dxc_dir() else {
        eprintln!(
            "rurixc: note: dxc validator suite not found or incomplete (set RURIX_DXC_DIR or RURIX_DXC_NEW_DIR to a directory containing dxc.exe, dxv.exe, and dxil.dll); DXIL emitted at {} (+ RTS0 + descriptor layout) but validator gate SKIPPED (RXS-0157)",
            obj_out.display()
        );
        return 0;
    };
    match crate::toolchain::dxv_validate(&dxc_dir, &obj_out) {
        Ok(result) if result.success => {
            eprintln!(
                "rurixc: --target dxil: DXIL container emitted + dxc validator accepted ({})",
                obj_out.display()
            );
            0
        }
        Ok(result) => {
            diag.struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
                .arg(
                    "detail",
                    "dxc validator rejected emitted DXIL container".to_owned(),
                )
                .emit();
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), sm, diag.messages())
            );
            eprintln!("rurixc: dxv validator argv begin");
            for arg in &result.argv {
                eprintln!("{arg}");
            }
            eprintln!("rurixc: dxv validator argv end");
            match result.exit_code {
                Some(code) => eprintln!("rurixc: dxv validator exit_code: {code}"),
                None => eprintln!("rurixc: dxv validator exit_code: unavailable"),
            }
            eprintln!("rurixc: dxv validator stdout begin");
            eprint!("{}", result.stdout);
            if !result.stdout.ends_with('\n') {
                eprintln!();
            }
            eprintln!("rurixc: dxv validator stdout end");
            eprintln!("rurixc: dxv validator stderr begin");
            eprint!("{}", result.stderr);
            if !result.stderr.ends_with('\n') {
                eprintln!();
            }
            eprintln!("rurixc: dxv validator stderr end");
            1
        }
        Err(e) => {
            toolchain_err(diag, sm, e);
            1
        }
    }
}

/// `--target dxil` 但 feature `dxil-backend` 未启用(RXS-0157 L1):DXIL 后端不参与
/// 编译 → RX6007(P-01 strict-only,不降级 host/PTX)。
#[cfg(not(feature = "dxil-backend"))]
fn compile_dxil_target(
    diag: &DiagCtxt,
    sm: &SourceMap,
    _cx: &QueryCtx<'_>,
    _stem: &str,
    _out: Option<&Path>,
    _input_path: &Path,
) -> u8 {
    diag.struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
        .arg(
            "detail",
            "`--target dxil` 需启用 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate;未启用时 DXIL 后端不参与编译,PTX 路径不受影响)"
                .to_owned(),
        )
        .emit();
    eprint!(
        "{}",
        render_diagnostics(&diag.emitted(), sm, diag.messages())
    );
    1
}
