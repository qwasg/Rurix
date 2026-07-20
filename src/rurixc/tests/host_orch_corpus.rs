//! conformance/host_orch 前端机械面语料批跑(MS1.2,RFC-0009 §4.2;
//! RXS-0195/RXS-0196)。
//!
//! 覆盖三件:
//! - `mod name;` out-of-line 模块装配(accept/mod_file + reject/mod_missing +
//!   reject/mod_cycle,RX1005,RXS-0196);
//! - extern "C" 无 body fn 符号保名(accept/extern_link IR 断言:字面名 declare、
//!   不走 mangle,RXS-0195);
//! - `#[link(name = "x")]` 接线(collect_link_libs 收集 + 属性形态非法 RX7022,
//!   RXS-0195;真实链接段追加由 ci 冒烟 / 手动 `rx build` 全链路见证)。
//!
//! 管线镜像 driver:lex/parse → mod 装配([`rurixc::mod_assembly`])→ resolve →
//! typeck → 着色/launch/穷尽性/consteval → MIR → move/borrow → codegen IR。
//! reject 体例:根文件 main.rx 带 `//@ expect-error: RX####` 头,断言"产生诊断
//! 且全部为预期码"(反例全拦截口径,对齐 launch_corpus)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::lexer::lex;
use rurixc::mod_assembly::assemble_out_of_line_mods;
use rurixc::parser::parse;
use rurixc::query::QueryCtx;
use rurixc::source_map::SourceMap;
use rurixc::span::Edition;

/// reject 预设类别(目录即类别;根文件 = `<类别>/main.rx`,环/辅助文件不作根)。
/// gpu 语料四类(MS1.2):elem_infer(RX2010,RXS-0190)/ gpu_in_kernel(RX3015,
/// RXS-0189)/ launch_arg_subset(RX6024,RXS-0191)/ buffer_move(move 后再用,
/// 既有 RX4001 拦,RXS-0189)。present 语料两类(MS1.2b):present_out_of_order
/// (typestate 错序 = move 违例,既有 RX4001 拦,RXS-0197)/ present_in_kernel
/// (RX3015,RXS-0197)。bindless 语料一类(G3.4):table_in_kernel(TextureTable
/// 宿主注册面 kernel 体内 → RX3015,RXS-0235 L2 承 RXS-0189 同点位)。
// render graph 语料一类(G3.5):graph_in_kernel(Graph 宿主构造/方法 kernel 体内 →
// RX3015,RXS-0236 承 RXS-0189 同点位)。
// EI1.3 Part B UC-05 RHI 语料独立成 `conformance/uc05/`(见 tests/uc05_corpus.rs);不混入
// host_orch(RFC-0014 §4.B / spec/rhi.md,路径与 spec 锚对齐)。
const REJECT_CATEGORIES: [&str; 10] = [
    "mod_missing",
    "mod_cycle",
    "elem_infer",
    "gpu_in_kernel",
    "launch_arg_subset",
    "buffer_move",
    "present_out_of_order",
    "present_in_kernel",
    "table_in_kernel",
    "graph_in_kernel",
];

fn host_orch_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/host_orch")
        .join(sub)
}

fn rx_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", d.display())) {
            let p = e.expect("读取目录项失败").path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rx") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// 镜像 driver 管线跑一个根文件(装配 + 全量静态检查 + MIR/codegen IR)。
/// 返回 (错误码序列, LLVM IR——有错时为空串)。
fn run_root(root: &Path) -> (Vec<u16>, String) {
    let src = fs::read_to_string(root).expect("读取语料失败");
    let diag = DiagCtxt::new();
    let mut sm = SourceMap::new();
    let file_name = root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .expect("根文件名");
    let id = sm.add_file(file_name.clone(), src.as_str(), Edition::Rx0);
    let tokens = lex(&src, id, Edition::Rx0, &diag);
    let mut ast = parse(&src, tokens, id, Edition::Rx0, &diag);
    // out-of-line 模块装配(RXS-0196):parse 后、resolve 前(镜像 driver)
    let module_srcs = assemble_out_of_line_mods(&mut ast, root, &mut sm, Edition::Rx0, &diag);
    let mut cx = QueryCtx::from_ast(ast, &src, id, &diag);
    for (fid, fsrc) in module_srcs {
        cx.add_module_src(fid, fsrc);
    }
    let mut ir = String::new();
    if !diag.has_errors() {
        let _ = cx.resolutions();
        if !diag.has_errors() {
            cx.check_crate();
        }
        if !diag.has_errors() {
            cx.check_coloring();
            cx.check_launch();
            cx.check_crate_patterns();
        }
        if !diag.has_errors() {
            cx.check_consteval();
        }
        if !diag.has_errors() {
            let m = cx.mir_crate();
            if !diag.has_errors() {
                cx.check_moves();
            }
            if !diag.has_errors() {
                cx.check_borrows();
            }
            if !diag.has_errors() {
                // gpu 语料 IR 断言口径:嵌入产物不喂真实 PTX(None → 哨兵空表),
                // 只验 declare rxrt_* / @__rx_gpu_artifacts 结构在位(RXS-0192 的
                // 真实红绿在 ci 冒烟 / rx build 全链路)。
                ir = rurixc::codegen::emit_llvm_ir(
                    &m,
                    &cx.hir_crate(),
                    &sm,
                    &rurixc::codegen::CodegenOpts {
                        module_name: "host_orch",
                        file_name: &file_name,
                        directory: ".",
                        lang_items: cx.resolutions().lang_items,
                        gpu_artifacts: None,
                    },
                );
            }
        }
    }
    let codes = diag
        .emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect();
    (codes, ir)
}

/// 内联源:parse 后收集 `#[link]`(RX7022 形态检查单测入口)。
fn collect_from(src: &str) -> (Vec<u16>, Vec<String>) {
    let diag = DiagCtxt::new();
    let mut sm = SourceMap::new();
    let id = sm.add_file("inline.rx", src, Edition::Rx0);
    let tokens = lex(src, id, Edition::Rx0, &diag);
    let ast = parse(src, tokens, id, Edition::Rx0, &diag);
    assert!(!diag.has_errors(), "inline 语料 parse 不应有错: {src}");
    let mut libs = Vec::new();
    rurixc::driver::collect_link_libs(&ast.items, &sm, &diag, &mut libs);
    let codes = diag
        .emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect();
    (codes, libs)
}

// ---------------------------------------------------------------------------
// `mod name;` out-of-line 模块装配(RXS-0196)
// ---------------------------------------------------------------------------

/// accept/mod_file:装配后与内联 mod 无差别——0 诊断 + 模块内 fn 正常单态
/// 化(mangled 符号进 IR)+ 跨文件字面量取值正确(util.rx 的 40 进常量)。
#[test]
fn accept_mod_file_assembles_and_compiles() {
    let root = host_orch_dir("accept/mod_file").join("main.rx");
    let (codes, ir) = run_root(&root);
    assert!(codes.is_empty(), "accept/mod_file 产生诊断: {codes:?}");
    assert!(
        ir.contains("@rx_add_"),
        "模块内 fn 应以 mangled 符号参与 codegen(与 extern 字面名保名区分)"
    );
}

/// reject 反例全拦截:根文件 `//@ expect-error: RX####` 头,产生诊断且全为预期码。
#[test]
fn reject_corpus_all_intercepted() {
    for cat in REJECT_CATEGORIES {
        let root = host_orch_dir("reject").join(cat).join("main.rx");
        let src = fs::read_to_string(&root)
            .unwrap_or_else(|e| panic!("缺 reject 根文件 {}: {e}", root.display()));
        let expected: u16 = src
            .lines()
            .find_map(|l| l.trim().strip_prefix("//@ expect-error: RX"))
            .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", root.display()))
            .trim()
            .parse()
            .expect("expect-error 码格式非法");
        let (codes, _) = run_root(&root);
        assert!(
            !codes.is_empty(),
            "{} 未被拦截(反例全拦截口径)",
            root.display()
        );
        assert!(
            codes.iter().all(|c| *c == expected),
            "{} 诊断码偏离预期 RX{expected}: {codes:?}",
            root.display()
        );
    }
}

// ---------------------------------------------------------------------------
// std::gpu 宿主编排单源语料(MS1.2,RXS-0189~0193)
// ---------------------------------------------------------------------------

/// accept/saxpy_single_source:完整单源程序(kernel saxpy + host main 编排)
/// 0 诊断,且宿主 lowering 落 rxrt_* 字面符号 declare(RXS-0191/0194)、嵌入
/// 描述表常量在位(RXS-0192)、kernel 入口 = device MIR 同源 mangle 符号常量、
/// 失败终止检查接线 rxrt_trap(RXS-0193)。
#[test]
fn accept_saxpy_single_source_lowers_to_rxrt() {
    let root = host_orch_dir("accept/saxpy_single_source").join("main.rx");
    let (codes, ir) = run_root(&root);
    assert!(codes.is_empty(), "accept/saxpy 产生诊断: {codes:?}");
    for sym in [
        "rxrt_ctx_create",
        "rxrt_stream_create",
        "rxrt_buf_alloc",
        "rxrt_pinned_alloc",
        "rxrt_pinned_ptr",
        "rxrt_pinned_len",
        "rxrt_buf_upload",
        "rxrt_buf_download",
        "rxrt_launch",
        "rxrt_stream_sync",
        "rxrt_ctx_sync",
        "rxrt_trap",
    ] {
        assert!(
            ir.contains(&format!("@{sym}(")),
            "缺 rxrt 符号 @{sym}(RXS-0191/0193/0194)\nIR:\n{ir}"
        );
    }
    assert!(
        ir.contains("declare i64 @rxrt_ctx_create(ptr)"),
        "rxrt_ctx_create declare 形态(句柄 u64 + 描述表 ptr)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("@__rx_gpu_artifacts"),
        "嵌入描述表常量缺失(RXS-0192)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("rx_saxpy_"),
        "kernel 入口应为 device MIR 同源 mangle 符号常量(RXS-0191)\nIR:\n{ir}"
    );
    // marshalling 形态(🔒 RXS-0191):slots/kinds 平行数组指针 + n_args。
    assert!(
        ir.contains("call i32 @rxrt_launch(i64"),
        "rxrt_launch 调用形态\nIR:\n{ir}"
    );
}

/// accept/bindless_table(G3.4,RXS-0235):`ctx.texture_table()` / `register` / `len`
/// 编译器已知签名 0 诊断,lowering 落 `rxrt_table_*` 字面符号 declare(create 产 u64
/// 句柄 / register 双 u64 入 u32 出〔失败哨兵 u32::MAX → rxrt_trap〕/ len u64 入 u32
/// 出);注册非消费——注册后句柄仍作 launch 实参(镜像 RXS-0191 Buffer 实参纪律)。
#[test]
fn accept_bindless_table_lowers_to_rxrt_table() {
    let root = host_orch_dir("accept/bindless_table").join("main.rx");
    let (codes, ir) = run_root(&root);
    assert!(
        codes.is_empty(),
        "accept/bindless_table 产生诊断: {codes:?}"
    );
    assert!(
        ir.contains("declare i64 @rxrt_table_create(i64)"),
        "rxrt_table_create declare 形态(ctx 句柄入,table 句柄出,RXS-0235)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i32 @rxrt_table_register(i64, i64)"),
        "rxrt_table_register declare 形态(table + 资源句柄入,注册序索引 u32 出)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i32 @rxrt_table_len(i64)"),
        "rxrt_table_len declare 形态(table 句柄入,已注册计数 u32 出)\nIR:\n{ir}"
    );
    // 失败终止检查接线(RXS-0193:register 失败哨兵 u32::MAX → rxrt_trap)。
    assert!(
        ir.contains("@rxrt_trap("),
        "register 失败哨兵检查应接线 rxrt_trap(RXS-0193)\nIR:\n{ir}"
    );
    // 注册非消费:注册后句柄仍可作 launch 实参(RXS-0191 同纪律)。
    assert!(
        ir.contains("call i32 @rxrt_launch(i64"),
        "注册后句柄应仍可作 launch 实参(RXS-0235/RXS-0191)\nIR:\n{ir}"
    );
}

/// accept/graph_deferred_three_pass(G3.5,RXS-0236):Graph/PassBuilder/GraphResource
/// 编译器已知签名 0 诊断,deferred 三 pass 图声明 lowering 落 `rxrt_graph_*` 字面符号 declare
/// (create 产 u64 图句柄 / resource 产 u64 资源句柄 / pass 产 u64 pass 句柄 / declare 三参
/// 声明访问 / readback / execute 装配核验);声明序 = 提交序,用户不写 barrier。
#[test]
fn accept_graph_deferred_three_pass_lowers_to_rxrt_graph() {
    let root = host_orch_dir("accept/graph_deferred_three_pass").join("main.rx");
    let (codes, ir) = run_root(&root);
    assert!(
        codes.is_empty(),
        "accept/graph_deferred_three_pass 产生诊断: {codes:?}"
    );
    assert!(
        ir.contains("declare i64 @rxrt_graph_create(i64)"),
        "rxrt_graph_create declare 形态(ctx 句柄入,图句柄出,RXS-0241)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i64 @rxrt_graph_resource(i64, i32)"),
        "rxrt_graph_resource declare 形态(图句柄 + class 入,资源句柄出)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i64 @rxrt_graph_pass(i64)"),
        "rxrt_graph_pass declare 形态(图句柄入,pass 句柄出)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i32 @rxrt_graph_declare(i64, i64, i32)"),
        "rxrt_graph_declare declare 形态(pass + resource 句柄 + access tag)\nIR:\n{ir}"
    );
    for sym in ["rxrt_graph_readback", "rxrt_graph_execute", "rxrt_trap"] {
        assert!(
            ir.contains(&format!("@{sym}(")),
            "缺 render graph 符号 @{sym}(RXS-0241)\nIR:\n{ir}"
        );
    }
    // 装配核验失败终止检查接线(RXS-0193:execute 负值 rc → rxrt_trap)。
    assert!(
        ir.contains("@rxrt_graph_execute("),
        "execute 装配核验应接线(RXS-0241/0193)\nIR:\n{ir}"
    );
}

// ---------------------------------------------------------------------------
// present 宿主 typestate 面 + 宿主图像落盘桥(MS1.2b,RXS-0197~0199)
// ---------------------------------------------------------------------------

/// accept/present_loop:完整帧循环 0 诊断,消费式转移与借用句柄 lowering 落
/// rxp_* 字面符号 declare(RXS-0197/0198/0194);`ready()` 纯类型面转移不落
/// 运行时符号;失败终止检查接线 rxrt_trap(RXS-0193)。
#[test]
fn accept_present_loop_lowers_to_rxp() {
    let root = host_orch_dir("accept/present_loop").join("main.rx");
    let (codes, ir) = run_root(&root);
    assert!(codes.is_empty(), "accept/present_loop 产生诊断: {codes:?}");
    for sym in [
        "rxp_create",
        "rxp_wait",
        "rxp_backbuffer",
        "rxp_signal",
        "rxp_pump",
        "rxp_present",
        "rxrt_trap",
    ] {
        assert!(
            ir.contains(&format!("@{sym}(")),
            "缺 present 符号 @{sym}(RXS-0197/0198)\nIR:\n{ir}"
        );
    }
    assert!(
        ir.contains("declare i64 @rxp_create(i64, i32, i32, i32, i32)"),
        "rxp_create declare 形态(ctx 句柄 + rw/rh/ww/wh u32)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i64 @rxp_backbuffer(i64)"),
        "rxp_backbuffer declare 形态(借用句柄 u64,RXS-0198)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i32 @rxp_pump(i64)"),
        "rxp_pump declare 形态(rc → bool,RXS-0197)\nIR:\n{ir}"
    );
    assert!(
        !ir.contains("rxp_ready"),
        "`ready()` 为纯类型面转移,不得落运行时符号(RXS-0197)"
    );
    // backbuffer 借用句柄作 launch 实参(RXS-0198):blit kernel 经 rxrt_launch。
    assert!(
        ir.contains("call i32 @rxrt_launch(i64"),
        "backbuffer 应可作 launch 实参(RXS-0198/0191)\nIR:\n{ir}"
    );
}

/// accept/imageio_write:pinned 填充 + write_ppm 0 诊断,lowering 落
/// rxio_write_ppm 字面符号(路径 NUL 终止字符串 + rxrt_pinned_ptr/len 物化
/// 指针与元素数,RXS-0199/0194);真跑(退出 0 + PPM 字节)随 cabi 落位由
/// ci 冒烟 / 手动 `rx build` 全链路见证。
#[test]
fn accept_imageio_write_lowers_to_rxio() {
    let root = host_orch_dir("accept/imageio_write").join("main.rx");
    let (codes, ir) = run_root(&root);
    assert!(codes.is_empty(), "accept/imageio_write 产生诊断: {codes:?}");
    assert!(
        ir.contains("declare i32 @rxio_write_ppm(ptr, i32, i32, ptr, i64)"),
        "rxio_write_ppm declare 形态(path ptr + w/h u32 + data ptr + n u64,RXS-0199)\nIR:\n{ir}"
    );
    for sym in ["rxrt_pinned_ptr", "rxrt_pinned_len"] {
        assert!(
            ir.contains(&format!("@{sym}(")),
            "缺锁页物化符号 @{sym}(RXS-0199 经 RXS-0191 同机制)\nIR:\n{ir}"
        );
    }
}

// ---------------------------------------------------------------------------
// extern "C" 符号保名 + `#[link]` 接线(RXS-0195)
// ---------------------------------------------------------------------------

/// accept/extern_link:extern "C" 无 body fn 以字面名 declare 进 IR(不走
/// mangle);`#[link(name = "user32")]` 收集为链接段追加库。
#[test]
fn accept_extern_link_preserves_symbol_and_collects_lib() {
    let root = host_orch_dir("accept/extern_link").join("main.rx");
    let (codes, ir) = run_root(&root);
    assert!(codes.is_empty(), "accept/extern_link 产生诊断: {codes:?}");
    assert!(
        ir.contains("@GetSystemMetrics("),
        "extern fn 应以字面名参与 codegen(RXS-0195 符号保名)\nIR:\n{ir}"
    );
    assert!(
        !ir.contains("rx_GetSystemMetrics"),
        "extern fn 不得走 mangle()(RXS-0195)"
    );

    let src = fs::read_to_string(&root).expect("读取语料失败");
    let (codes, libs) = collect_from(&src);
    assert!(codes.is_empty(), "合法 #[link] 不应产生诊断: {codes:?}");
    assert_eq!(libs, vec!["user32".to_owned()], "#[link] 库名收集");
}

/// `#[link]` 属性形态非法 → RX7022(name 缺失/空/非字符串/重复或未知键;
/// 编译期 emit 点,RFC-0009 §4.2)。
#[test]
fn link_attr_bad_forms_rejected() {
    const EXTERN_TAIL: &str = "extern \"C\" {\n    fn f();\n}\nfn main() -> i32 { 0 }\n";
    let bad_cases = [
        format!("#[link]\n{EXTERN_TAIL}"),              // 非 list 形态
        format!("#[link(name = 1)]\n{EXTERN_TAIL}"),    // 非字符串
        format!("#[link(name = \"\")]\n{EXTERN_TAIL}"), // 空名
        format!("#[link(kind = \"static\")]\n{EXTERN_TAIL}"), // 未知键
        format!("#[link(name = \"a\", name = \"b\")]\n{EXTERN_TAIL}"), // 重复键
        format!("#[link()]\n{EXTERN_TAIL}"),            // 缺 name
    ];
    for src in &bad_cases {
        let (codes, libs) = collect_from(src);
        assert_eq!(codes, vec![7022], "非法 #[link] 形态应报 RX7022: {src}");
        assert!(libs.is_empty(), "非法形态不得收集库名: {src}");
    }
}

/// 同名库跨 extern 块去重;多库保序收集。
#[test]
fn link_attr_dedups_and_preserves_order() {
    let src = "#[link(name = \"user32\")]\nextern \"C\" {\n    fn a();\n}\n\
               #[link(name = \"winmm\")]\n#[link(name = \"user32\")]\nextern \"C\" {\n    fn b();\n}\n\
               fn main() -> i32 { 0 }\n";
    let (codes, libs) = collect_from(src);
    assert!(codes.is_empty(), "合法 #[link] 不应产生诊断: {codes:?}");
    assert_eq!(libs, vec!["user32".to_owned(), "winmm".to_owned()]);
}

// ---------------------------------------------------------------------------
// 语料纪律(traceability)
// ---------------------------------------------------------------------------

/// 每个语料文件携带条款锚定头(//@ spec: RXS-####,对齐 launch_corpus)。
#[test]
fn corpus_files_carry_spec_anchor() {
    let mut n = 0;
    for sub in ["accept", "reject"] {
        for f in rx_files(&host_orch_dir(sub)) {
            let src = fs::read_to_string(&f).expect("读取语料失败");
            let first = src.lines().next().unwrap_or("");
            assert!(
                first.starts_with("//@ spec: RXS-"),
                "{} 缺条款锚定头(//@ spec: RXS-####)",
                f.display()
            );
            n += 1;
        }
    }
    assert!(n >= 11, "host_orch 语料过少: {n} 个");
}

/// reject 覆盖预设类别(目录即类别)。
#[test]
fn reject_has_expected_categories() {
    let reject = host_orch_dir("reject");
    for cat in REJECT_CATEGORIES {
        let d = reject.join(cat);
        assert!(
            d.is_dir() && !rx_files(&d).is_empty(),
            "缺类别目录或为空: host_orch/reject/{cat}/"
        );
    }
}
