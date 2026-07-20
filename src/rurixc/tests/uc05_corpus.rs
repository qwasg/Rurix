//! conformance/uc05 EI1.3 Part B UC-05 最小 RHI 语料批跑(RFC-0014 §4.B;RXS-0256~0265)。
//!
//! 覆盖三档(裁决 1,spec/rhi.md RXS-0263):
//! - **accept**(0 诊断 + lowering 落 `rxrt_rhi_*` 字面符号);
//! - **reject 编译期**(I1/I2/I6/I7/I8;`//@ expect-error: RX####` 头,反例全拦截口径)——
//!   res_use_after_move / res_double_move(RX4001,readback 按值 move-out)/ rhi_double_submit
//!   (RX4001,1-submit typestate)/ rhi_cross_brand(RX3006,per-instance brand)/ rhi_in_kernel
//!   (RX3015,着色合法性);
//! - **assembly**(I3/I5;`//@ assembly-reject:` 头,**编译期 CLEAN**——图装配期性质,`--emit=check`
//!   不拦,由 ci/uc05_rhi_smoke.py 步骤 72 编译成 EXE 真跑 red-green + rhi.rs 库单测纯 host 见证)。
//!
//! 管线镜像 driver:lex/parse → resolve → typeck → 着色/launch/穷尽性/consteval → MIR →
//! move/borrow → codegen IR(uc05 语料为单文件 flat,无 out-of-line mod)。
//!
//! I1~I10 不变量矩阵 ↔ reject/assembly 语料实存 ↔ 对照报告三方一致性机核(RXS-0263/0264)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::lexer::lex;
use rurixc::parser::parse;
use rurixc::query::QueryCtx;
use rurixc::source_map::SourceMap;
use rurixc::span::Edition;

/// 编译期 reject 预设文件(I1/I2/I6/I7/I8;裁决 1 编译期档)。
const COMPILE_REJECTS: [&str; 5] = [
    "res_use_after_move",
    "res_double_move",
    "rhi_double_submit",
    "rhi_cross_brand",
    "rhi_in_kernel",
];

/// 装配期 reject 预设文件(I3/I5 + 生命周期;编译期 CLEAN,submit 装配期确定性拦)。
const ASSEMBLY_REJECTS: [&str; 3] = ["graph_cycle", "graph_write_write", "graph_empty"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn uc05_dir(sub: &str) -> PathBuf {
    repo_root().join("conformance/uc05").join(sub)
}

fn rx_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return out;
    }
    for e in fs::read_dir(root).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", root.display()))
    {
        let p = e.expect("读取目录项失败").path();
        if p.extension().is_some_and(|x| x == "rx") {
            out.push(p);
        }
    }
    out.sort();
    out
}

/// 镜像 driver 管线跑一个根文件(全量静态检查 + MIR/codegen IR)。
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
    let ast = parse(&src, tokens, id, Edition::Rx0, &diag);
    let cx = QueryCtx::from_ast(ast, &src, id, &diag);
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
                ir = rurixc::codegen::emit_llvm_ir(
                    &m,
                    &cx.hir_crate(),
                    &sm,
                    &rurixc::codegen::CodegenOpts {
                        module_name: "uc05",
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

// ---------------------------------------------------------------------------
// accept:lowering 落 rxrt_rhi_* 字面符号(RXS-0256~0260)
// ---------------------------------------------------------------------------

/// accept/rhi_min(RXS-0256):Rhi / Res / Pass / Queue 编译器已知签名 0 诊断,compute-pass
/// 声明 lowering 落 `rxrt_rhi_*` 字面符号 declare(create 图根 / resource 资源 / pass pass /
/// declare 三参访问 / submit 装配核验);`reads`/`writes` 取 `&Res` 借用(同资源二次借用合法),
/// submit 消费式 typestate;声明序 = 提交序,用户不写 barrier。
//@ spec: RXS-0256, RXS-0257, RXS-0258, RXS-0260
#[test]
fn accept_rhi_min_lowers_to_rxrt_rhi() {
    let root = uc05_dir("accept").join("rhi_min.rx");
    let (codes, ir) = run_root(&root);
    assert!(codes.is_empty(), "accept/rhi_min 产生诊断: {codes:?}");
    assert!(
        ir.contains("declare i64 @rxrt_rhi_create(i64)"),
        "rxrt_rhi_create declare 形态(ctx 句柄入,图根句柄出,RXS-0256)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i64 @rxrt_rhi_resource(i64)"),
        "rxrt_rhi_resource declare 形态(图根句柄入,资源句柄出)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i64 @rxrt_rhi_pass(i64)"),
        "rxrt_rhi_pass declare 形态(图根句柄入,pass 句柄出)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("declare i32 @rxrt_rhi_declare(i64, i64, i32)"),
        "rxrt_rhi_declare declare 形态(pass + resource 句柄 + access tag)\nIR:\n{ir}"
    );
    for sym in ["rxrt_rhi_submit", "rxrt_trap"] {
        assert!(
            ir.contains(&format!("@{sym}(")),
            "缺 RHI 符号 @{sym}(RXS-0256~0260)\nIR:\n{ir}"
        );
    }
}

/// accept/graph_three_pass(RXS-0258/0259):三 compute pass RAW 建序 + `rhi.readback(c)` 按值
/// move-out lowering 落 `rxrt_rhi_readback(rhi, res)`(资源实参 move,I1/I2 消费锚);0 诊断。
//@ spec: RXS-0258, RXS-0259
#[test]
fn accept_graph_three_pass_lowers_readback() {
    let root = uc05_dir("accept").join("graph_three_pass.rx");
    let (codes, ir) = run_root(&root);
    assert!(
        codes.is_empty(),
        "accept/graph_three_pass 产生诊断: {codes:?}"
    );
    assert!(
        ir.contains("declare i32 @rxrt_rhi_readback(i64, i64)"),
        "rxrt_rhi_readback declare 形态(rhi + 资源句柄入,rc 出;资源按值消费 RXS-0259)\nIR:\n{ir}"
    );
    assert!(
        ir.contains("@rxrt_rhi_readback("),
        "readback 应接线(RXS-0259;负值 rc → rxrt_trap)\nIR:\n{ir}"
    );
}

/// accept/pass_declared(RXS-0257)+ single_submit(RXS-0260):pass 声明面 + 单 submit typestate
/// 正例 0 诊断。
//@ spec: RXS-0257, RXS-0260
#[test]
fn accept_pass_declared_and_single_submit() {
    for (name, tag) in [("pass_declared", "RXS-0257"), ("single_submit", "RXS-0260")] {
        let root = uc05_dir("accept").join(format!("{name}.rx"));
        let (codes, ir) = run_root(&root);
        assert!(codes.is_empty(), "accept/{name}({tag}) 产生诊断: {codes:?}");
        assert!(
            ir.contains("@rxrt_rhi_submit("),
            "accept/{name} submit 应接线(RXS-0260)\nIR:\n{ir}"
        );
    }
}

/// 采纳判据 C ABI 成熟面(RXS-0265):RHI 宿主面 lowering 全落 `rxrt_rhi_*` 稳定 C ABI 符号
/// (端到端 C ABI 承载即成熟度腿之一;增量 check <5s 双口径 bench measured 随 EI1.5 回填,
/// SKIP 不充绿,不进 CI 硬门)。
//@ spec: RXS-0265
#[test]
fn adoption_c_abi_surface_present() {
    let root = uc05_dir("accept").join("rhi_min.rx");
    let (codes, ir) = run_root(&root);
    assert!(codes.is_empty(), "accept/rhi_min 产生诊断: {codes:?}");
    for sym in [
        "rxrt_rhi_create",
        "rxrt_rhi_resource",
        "rxrt_rhi_pass",
        "rxrt_rhi_declare",
        "rxrt_rhi_submit",
    ] {
        assert!(
            ir.contains(&format!("@{sym}(")),
            "采纳判据 C ABI 面缺 @{sym}(RXS-0265 C ABI 成熟腿)\nIR:\n{ir}"
        );
    }
}

// ---------------------------------------------------------------------------
// reject 编译期(I1/I2/I6/I7/I8):反例全拦截口径
// ---------------------------------------------------------------------------

/// reject 编译期反例全拦截:根文件 `//@ expect-error: RX####` 头,产生诊断且全为预期码。
/// 覆盖 I1(res_use_after_move RX4001)/ I2(res_double_move RX4001)/ I6(rhi_double_submit
/// RX4001)/ I7(rhi_cross_brand RX3006)/ I8(rhi_in_kernel RX3015)。装配期 I3/I5 不在此(编译期
/// CLEAN)——由 ci/uc05_rhi_smoke.py 步骤 72 EXE 真跑断言。
//@ spec: RXS-0256, RXS-0259, RXS-0260
#[test]
fn reject_compile_time_all_intercepted() {
    for cat in COMPILE_REJECTS {
        let root = uc05_dir("reject").join(format!("{cat}.rx"));
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
        assert!(!codes.is_empty(), "reject/{cat} 未被拦截(反例全拦截口径)");
        assert!(
            codes.iter().all(|c| *c == expected),
            "reject/{cat} 诊断码偏离预期 RX{expected}: {codes:?}"
        );
    }
}

/// 装配期语料**编译期 CLEAN**(图装配期性质,`--emit=check` 不拦;裁决 1 装配期档):
/// graph_cycle(I3)/ graph_write_write(I5)/ graph_empty(生命周期)编译 0 诊断——违例在 submit
/// 装配期由 rhi.rs 确定性拦(库单测纯 host 见证 + 步骤 72 EXE 真跑 red-green)。
//@ spec: RXS-0258
#[test]
fn assembly_rejects_compile_clean() {
    for cat in ASSEMBLY_REJECTS {
        let root = uc05_dir("assembly").join(format!("{cat}.rx"));
        let (codes, _) = run_root(&root);
        assert!(
            codes.is_empty(),
            "assembly/{cat} 应编译期 CLEAN(图装配期性质,--emit=check 不拦): {codes:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 语料纪律 + I1~I10 矩阵三方一致性(traceability;RXS-0263/0264)
// ---------------------------------------------------------------------------

/// 每个 uc05 语料文件携带条款锚定头(//@ spec: RXS-####)。
//@ spec: RXS-0256
#[test]
fn corpus_files_carry_spec_anchor() {
    let mut n = 0;
    for sub in ["accept", "reject", "assembly"] {
        for f in rx_files(&uc05_dir(sub)) {
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
    assert!(
        n >= 12,
        "uc05 语料过少: {n} 个(4 accept + 5 reject + 3 assembly)"
    );
}

/// I1~I10 不变量矩阵 ↔ reject/assembly 语料实存 ↔ 对照报告三方一致性(RXS-0263/0264,防
/// YAML-only):矩阵 json 每条 compile_time/assembly_time 不变量的 `corpus` 路径必须实存;每条
/// 的诊断码 / 语料路径必须出现在 evidence/uc05_comparison_report.md;I4 = lib_tested(无 corpus,
/// .rx 接线 EI1.4);I9/I10 = report_only(无诊断码)。
//@ spec: RXS-0263, RXS-0264
#[test]
fn invariant_matrix_three_way_consistency() {
    let root = repo_root();
    let matrix_text = fs::read_to_string(root.join("evidence/uc05_invariant_matrix.json"))
        .expect("缺 evidence/uc05_invariant_matrix.json");
    let report = fs::read_to_string(root.join("evidence/uc05_comparison_report.md"))
        .expect("缺 evidence/uc05_comparison_report.md");

    // 报告顶部醒目标注(RXS-0264 redline F3)。
    assert!(
        report.contains(
            "historical counters unavailable in-repo, non-reproducible, no fabricated figures"
        ),
        "对照报告缺顶部 historical counters 标注(RXS-0264)"
    );

    // 轻量解析矩阵 json(无第三方依赖:按 id/tier/corpus/diagnostic 行提取,断言一致性)。
    // 断言 I1~I10 全在矩阵;compile_time/assembly_time 的 corpus 实存 + 出现在报告。
    for inv in ["I1", "I2", "I3", "I4", "I5", "I6", "I7", "I8", "I9", "I10"] {
        let needle = format!("\"id\": \"{inv}\"");
        assert!(matrix_text.contains(&needle), "矩阵 json 缺不变量 {inv}");
    }
    // compile_time / assembly_time 组:corpus 路径实存 + 在报告出现。
    let corpora = [
        ("I1", "conformance/uc05/reject/res_use_after_move.rx"),
        ("I2", "conformance/uc05/reject/res_double_move.rx"),
        ("I3", "conformance/uc05/assembly/graph_cycle.rx"),
        ("I5", "conformance/uc05/assembly/graph_write_write.rx"),
        ("I6", "conformance/uc05/reject/rhi_double_submit.rx"),
        ("I7", "conformance/uc05/reject/rhi_cross_brand.rx"),
        ("I8", "conformance/uc05/reject/rhi_in_kernel.rx"),
    ];
    for (inv, path) in corpora {
        assert!(
            matrix_text.contains(path),
            "{inv} 矩阵 corpus 路径缺 {path}"
        );
        assert!(
            root.join(path).is_file(),
            "{inv} corpus 语料文件不存在: {path}"
        );
        assert!(
            report.contains(path),
            "{inv} corpus 路径未在对照报告出现(三方一致性,RXS-0264): {path}"
        );
    }
    // I4 = lib_tested(诚实收窄):矩阵标注 rx_wiring:EI1.4,不锚 .rx corpus。
    assert!(
        matrix_text.contains("\"rx_wiring\": \"EI1.4") || matrix_text.contains(".rx_wiring:EI1.4"),
        "I4 应诚实标注 .rx 反射喂入随 EI1.4(RXS-0257 收窄)"
    );
    // 报告三档划界措辞在位(裁决 1)。
    for tier in ["编译期", "装配期", "report_only"] {
        assert!(report.contains(tier), "对照报告缺三档措辞 {tier}(裁决 1)");
    }
}
