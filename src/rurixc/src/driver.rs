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
const E_LINK_ATTR: ErrorCode = ErrorCode(7022); // RX7022
const E_GPU_EMBED: ErrorCode = ErrorCode(6025); // RX6025(RXS-0192)
const E_RT_CABI: ErrorCode = ErrorCode(7021); // RX7021(RXS-0195)

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
    let mut ast = parse(&src, tokens, id, Edition::Rx0, &diag);
    prof.record(
        "parse",
        t,
        &[("tokens", n_tokens), ("items", ast.items.len() as u64)],
    );
    // out-of-line 模块装配(RXS-0196):parse 后、resolve 前;`mod name;` 按输入
    // 文件同目录 name.rx 加载 splice 为内联 mod 等价形态(resolve/typeck 零改动;
    // 缺失/IO/循环 → RX1005,经下方阶段化关卡中止)。
    let module_srcs = crate::mod_assembly::assemble_out_of_line_mods(
        &mut ast,
        &input_path,
        &mut sm,
        Edition::Rx0,
        &diag,
    );
    // #[link(name = "x")] 接线(RXS-0195):extern 块属性收集,链接段追加 x.lib;
    // 属性形态非法 → RX7022(编译期,同经阶段化关卡中止)。
    let mut link_libs: Vec<String> = Vec::new();
    collect_link_libs(&ast.items, &sm, &diag, &mut link_libs);
    let mut cx = QueryCtx::from_ast(ast, &src, id, &diag);
    for (fid, fsrc) in module_srcs {
        cx.add_module_src(fid, fsrc);
    }

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

    // 宿主 GPU 编排(MS1.2,RXS-0192/0195):MIR 含 rxrt_* 调用 = 本单元使用 gpu
    // 宿主 API → device 路产 single-source 嵌入产物(PTX 必存 + 可选 sm_89 cubin;
    // 工具链缺失/无 kernel 按哨兵纪律)+ 链接段追加 rurix-rt-cabi。host-only 程序
    // 链接线零漂移。
    let exe = out
        .clone()
        .unwrap_or_else(|| input_path.with_extension("exe"));
    let uses_gpu = mir_bodies.iter().any(|b| {
        b.blocks.iter().any(|bb| {
            matches!(
                &bb.terminator.kind,
                mir::TerminatorKind::Call {
                    target: mir::CallTarget::Rt { .. },
                    ..
                }
            )
        })
    });
    let gpu_artifacts = if uses_gpu {
        match build_gpu_artifacts(&diag, &sm, &cx, &stem, &exe) {
            Ok(a) => Some(a),
            Err(code) => return code,
        }
    } else {
        None
    };

    let t = Instant::now();
    let krate = cx.hir_crate();
    let lang_items = cx.resolutions().lang_items;
    let ir = codegen::emit_llvm_ir(
        &mir_bodies,
        &krate,
        &sm,
        &CodegenOpts {
            module_name: &stem,
            file_name: &file_name,
            directory: &directory,
            lang_items,
            gpu_artifacts,
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
    // #[link(name = "x")] 追加 x.lib(RXS-0195:最小策略,仅追加参数,定位依赖
    // /libpath 序;定位失败经下方退出码归因 RX7022)
    for lib in &link_libs {
        cmd.arg(format!("{lib}.lib"));
    }
    // 宿主 GPU 编排链接接线(RXS-0195):rurix_rt_cabi.lib(crt-static 构建,与
    // 基础集 libcmt 系一致)+ Rust staticlib 系统库固定集(`cargo rustc --print
    // native-static-libs` 实测 pin;kernel32 已在基础集)。定位/构建失败 → RX7021。
    if uses_gpu {
        match locate_or_build_rt_cabi() {
            Ok(lib) => {
                cmd.arg(&lib);
                for l in ["ntdll.lib", "userenv.lib", "ws2_32.lib", "dbghelp.lib"] {
                    cmd.arg(l);
                }
            }
            Err(detail) => {
                diag.struct_error(E_RT_CABI, "link.rt_cabi_failure")
                    .arg("detail", detail)
                    .emit();
                eprint!(
                    "{}",
                    render_diagnostics(&diag.emitted(), &sm, diag.messages())
                );
                return 1;
            }
        }
    }
    if opts.reproducible {
        cmd.arg("/Brepro");
    } else {
        cmd.arg("/debug:full");
    }
    for p in &libpaths {
        cmd.arg(format!("/libpath:{}", p.display()));
    }
    if let Err(e) = run_tool(&mut cmd, "link.exe") {
        // #[link] 追加库在场时链接失败归因 RX7022(RXS-0195:库定位失败只能事后
        // 从 link.exe 退出码判,包一层 #[link] 上下文提示);无追加库维持 RX7001。
        if link_libs.is_empty() {
            toolchain_err(&diag, &sm, e);
        } else {
            let libs = link_libs
                .iter()
                .map(|l| format!("{l}.lib"))
                .collect::<Vec<_>>()
                .join(", ");
            diag.struct_error(E_LINK_ATTR, "link.native_lib_failure")
                .arg(
                    "detail",
                    format!("link.exe failed with `#[link]` libraries appended ({libs}): {e}"),
                )
                .emit();
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), &sm, diag.messages())
            );
        }
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

/// 收集 extern 块上的 `#[link(name = "x")]`(RXS-0195):链接段追加 `x.lib` 参数
/// (最小策略:仅追加参数,定位交给 link.exe 的 /libpath 序)。属性形态非法
/// (非 list 形态 / 缺 name / 空名 / 非字符串 / 重复或未知键)→ **RX7022** 编译期
/// 诊断;库定位失败无法在编译期精确归因,由 [`compile`] 链接段按 link.exe 退出码
/// 事后归因(同码 RX7022)。挂在非 extern 块 item 上的 `#[link]` 不生效(不收集,
/// 与其余未知属性同样静默,MVP 属性纪律)。
//@ spec: RXS-0195
pub fn collect_link_libs(
    items: &[crate::ast::Item],
    sm: &SourceMap,
    diag: &DiagCtxt,
    libs: &mut Vec<String>,
) {
    use crate::ast::{ItemKind, LitKind, MetaInner, MetaKind};
    for item in items {
        match &item.kind {
            ItemKind::Mod(m) => collect_link_libs(&m.items, sm, diag, libs),
            ItemKind::ExternBlock(_) => {
                for attr in &item.attrs {
                    let is_link = !attr.inner
                        && attr.meta.path.segments.len() == 1
                        && attr.meta.path.segments[0].ident.name == "link";
                    if !is_link {
                        continue;
                    }
                    let bad = |detail: String| {
                        diag.struct_error(E_LINK_ATTR, "link.native_lib_failure")
                            .arg("detail", detail)
                            .span_label(attr.span, "invalid `#[link]` attribute form")
                            .emit();
                    };
                    let MetaKind::List(inner) = &attr.meta.kind else {
                        bad("expected `#[link(name = \"...\")]`".to_owned());
                        continue;
                    };
                    let mut name: Option<String> = None;
                    let mut ok = true;
                    for entry in inner {
                        let MetaInner::Meta(mi) = entry else {
                            bad("expected `name = \"...\"` inside `#[link(...)]`".to_owned());
                            ok = false;
                            break;
                        };
                        let is_name =
                            mi.path.segments.len() == 1 && mi.path.segments[0].ident.name == "name";
                        if !is_name {
                            bad(format!(
                                "unknown `#[link]` key `{}` (only `name = \"...\"` is supported)",
                                mi.path
                                    .segments
                                    .iter()
                                    .map(|s| s.ident.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join("::")
                            ));
                            ok = false;
                            break;
                        }
                        if name.is_some() {
                            bad("duplicate `name` key in `#[link(...)]`".to_owned());
                            ok = false;
                            break;
                        }
                        let MetaKind::NameValue(lit) = &mi.kind else {
                            bad("`#[link]` `name` must be a string literal".to_owned());
                            ok = false;
                            break;
                        };
                        if lit.kind != LitKind::Str {
                            bad("`#[link]` `name` must be a string literal".to_owned());
                            ok = false;
                            break;
                        }
                        let value = sm.snippet(lit.span).trim_matches('"').to_owned();
                        if value.is_empty() {
                            bad("`#[link]` `name` must not be empty".to_owned());
                            ok = false;
                            break;
                        }
                        name = Some(value);
                    }
                    if !ok {
                        continue;
                    }
                    match name {
                        Some(n) => {
                            if !libs.contains(&n) {
                                libs.push(n);
                            }
                        }
                        None => bad("missing `name = \"...\"` in `#[link(...)]`".to_owned()),
                    }
                }
            }
            _ => {}
        }
    }
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

/// single-source 嵌入产物构建(MS1.2,RXS-0192):host 编译单元使用 gpu 宿主 API
/// 时走 device 路产 PTX(复用 [`emit_ptx_and_gate`] 纪律:ptxas 干验证 RXS-0073 +
/// libdevice RXS-0082)+ `ptxas` 在位时预编 sm_89 cubin(RXS-0150)。
///
/// 哨兵纪律(RXS-0192/0193,不静默降级):
/// - 无 `kernel fn` 的编译单元 → 哨兵空表(编译不拒;运行期 `rxrt_ctx_create`
///   解析确定性拒 + 终止);
/// - 工具链缺失(libdevice SKIP)→ 哨兵空表 + note(对齐既有 SKIP 纪律);
/// - device 路自身错误 → 既有码(ptxas 拒 RX6004 / libdevice RX7002 / 工具链
///   RX7001);产物无法安全打包(空 PTX / 含 NUL 字节破坏 NUL 终止嵌入)→ RX6025。
fn build_gpu_artifacts(
    diag: &DiagCtxt,
    sm: &SourceMap,
    cx: &QueryCtx<'_>,
    stem: &str,
    exe: &Path,
) -> Result<codegen::GpuArtifacts, u8> {
    let ir = crate::device_codegen::build_and_emit(cx, stem);
    if diag.has_errors() {
        eprint!(
            "{}",
            render_diagnostics(&diag.emitted(), sm, diag.messages())
        );
        return Err(1);
    }
    let Some(ir) = ir else {
        eprintln!(
            "rurixc: note: no `kernel fn` in this unit; embedding sentinel GPU artifacts \
             (first gpu op fails deterministically at run time, RXS-0192)"
        );
        return Ok(codegen::GpuArtifacts::default());
    };
    let ptx_out = exe.with_extension("ptx");
    let embed = |ptx: String, cubin: Vec<u8>| -> Result<codegen::GpuArtifacts, u8> {
        if ptx.is_empty() || ptx.contains('\0') {
            diag.struct_error(E_GPU_EMBED, "codegen.gpu_embed_failure")
                .arg(
                    "detail",
                    if ptx.is_empty() {
                        "device path produced empty PTX for a unit with `kernel fn`"
                    } else {
                        "PTX text contains an interior NUL byte (cannot embed NUL-terminated)"
                    },
                )
                .emit();
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), sm, diag.messages())
            );
            return Err(1);
        }
        Ok(codegen::GpuArtifacts { ptx, cubin })
    };
    match emit_ptx_and_gate(&ir, stem, &ptx_out) {
        Ok(PtxGate::Ok(ptx)) => {
            // ptxas 在位:预编 sm_89 cubin 一并入描述表(RXS-0150;失败降级仅
            // PTX fallback,保守兜底)。
            let cubin = match crate::ptxas::compile_cubin(&ptx, stem, "sm_89") {
                crate::ptxas::CubinOutcome::Compiled(bytes) => bytes,
                _ => Vec::new(),
            };
            embed(ptx, cubin)
        }
        Ok(PtxGate::SkippedNoPtxas(ptx)) => {
            eprintln!(
                "rurixc: note: ptxas not found; embedding PTX-only artifacts \
                 (ptxas dry-gate SKIPPED, RXS-0073)"
            );
            embed(ptx, Vec::new())
        }
        Ok(PtxGate::SkippedNoLibdevice) => {
            eprintln!(
                "rurixc: note: libdevice.10.bc not found; embedding sentinel GPU artifacts \
                 (first gpu op fails deterministically at run time, RXS-0082/RXS-0192)"
            );
            Ok(codegen::GpuArtifacts::default())
        }
        Err(PtxError::LibdeviceLink(reason)) => {
            diag.struct_error(ErrorCode(7002), "link.libdevice_failure")
                .arg("reason", reason)
                .emit();
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), sm, diag.messages())
            );
            Err(1)
        }
        Err(PtxError::Rejected { reason }) => {
            diag.struct_error(ErrorCode(6004), "codegen.ptxas_rejected")
                .arg("reason", reason)
                .emit();
            eprint!(
                "{}",
                render_diagnostics(&diag.emitted(), sm, diag.messages())
            );
            Err(1)
        }
        Err(PtxError::Toolchain(e)) => {
            toolchain_err(diag, sm, e);
            Err(1)
        }
    }
}

/// rurix_rt_cabi.lib 定位序(RXS-0195,RX7021):env `RURIX_RT_CABI_LIB` →
/// rx.exe 旁 `lib/` → workspace `target/crt-static/release/`(缺则编排
/// `cargo build -p rurix-rt-cabi --release`,先例 rx build_pyd)。
///
/// CRT 口径(RFC-0009 §9 Q-Link 实测定案):cabi 以
/// `RUSTFLAGS=-C target-feature=+crt-static` 构建(静态 CRT = libcmt 系,与
/// 本 driver 链接基础集 libcmt.lib 一致,避免 /defaultlib:msvcrt 冲突);
/// `--target-dir target/crt-static` 与普通 target/release 缓存隔离。
fn locate_or_build_rt_cabi() -> Result<PathBuf, String> {
    const LIB: &str = "rurix_rt_cabi.lib";
    if let Ok(p) = std::env::var("RURIX_RT_CABI_LIB") {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Ok(pb);
        }
        return Err(format!("RURIX_RT_CABI_LIB points to a missing file: {p}"));
    }
    if let Ok(me) = std::env::current_exe()
        && let Some(dir) = me.parent()
    {
        let pb = dir.join("lib").join(LIB);
        if pb.is_file() {
            return Ok(pb);
        }
    }
    let Some(root) = find_workspace_with_rt_cabi() else {
        return Err(format!(
            "cannot find a workspace containing src/rurix-rt-cabi (searched upward from the \
             current directory and the compiler executable); set RURIX_RT_CABI_LIB to a \
             prebuilt {LIB}"
        ));
    };
    let lib = root
        .join("target")
        .join("crt-static")
        .join("release")
        .join(LIB);
    if lib.is_file() {
        return Ok(lib);
    }
    eprintln!(
        "rurixc: building rurix-rt-cabi (cargo --release, crt-static; one-time per workspace)…"
    );
    let out = Command::new("cargo")
        .args([
            "build",
            "-p",
            "rurix-rt-cabi",
            "--release",
            "--target-dir",
            "target/crt-static",
        ])
        .env("RUSTFLAGS", "-C target-feature=+crt-static")
        .current_dir(&root)
        .output()
        .map_err(|e| format!("cannot spawn cargo: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo build -p rurix-rt-cabi exited with {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    if lib.is_file() {
        Ok(lib)
    } else {
        Err(format!(
            "cargo build succeeded but {} is missing",
            lib.display()
        ))
    }
}

/// 向上查找含 `src/rurix-rt-cabi/Cargo.toml` 的 workspace 根(先当前目录、
/// 后编译器可执行所在目录;先例 rx `find_workspace_with_pyd`)。
fn find_workspace_with_rt_cabi() -> Option<PathBuf> {
    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(me) = std::env::current_exe()
        && let Some(dir) = me.parent()
    {
        starts.push(dir.to_path_buf());
    }
    for start in starts {
        let mut dir = start;
        loop {
            if dir
                .join("src")
                .join("rurix-rt-cabi")
                .join("Cargo.toml")
                .is_file()
            {
                return Some(dir);
            }
            if !dir.pop() {
                break;
            }
        }
    }
    None
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
    let ir = crate::dxil_codegen::build_and_emit_dxil(cx, stem);
    if diag.has_errors() {
        eprint!(
            "{}",
            render_diagnostics(&diag.emitted(), sm, diag.messages())
        );
        return 1;
    }
    let Some(ir) = ir else {
        eprintln!("rurixc: no compute `kernel fn` found; nothing to emit for --target dxil");
        return 2;
    };
    let obj_out = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| input_path.with_extension("dxc"));
    let Some(llc) = crate::toolchain::locate_llc() else {
        eprintln!(
            "rurixc: note: patched llc not found (set RURIX_LLC to dev DXIL llc, RD-011); DXIL emit + dxc validator gate SKIPPED (RXS-0157)"
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
            "rurixc: note: dxc validator not found (set RURIX_DXC_DIR); DXIL emitted at {} but validator gate SKIPPED (RXS-0157)",
            obj_out.display()
        );
        return 0;
    };
    match crate::toolchain::dxv_validate(&dxc_dir, &obj_out) {
        Ok(true) => {
            eprintln!(
                "rurixc: --target dxil: DXIL container emitted + dxc validator accepted ({})",
                obj_out.display()
            );
            0
        }
        Ok(false) => {
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
