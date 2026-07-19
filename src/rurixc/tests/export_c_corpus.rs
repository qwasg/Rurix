//! conformance/export_c 语料批跑（EI1.2,RFC-0014 Part A,RXS-0250~0255）：
//! `#[export(c)]` C ABI 导出的编译期挂载合法性 / C 兼容子集 v1 签名 / 导出体无
//! panic 面守门——reject 反例全拦截 + accept 正例 0 export_c 诊断。
//!
//! 管线（与 coloring/addrspace 不同）：export_c 诊断在 **收集/校验阶段**（lex →
//! parse → `collect_c_exports`）产出,不经 resolve/typeck/MIR——RXS-0250（挂载对象）
//! /RXS-0251（签名子集）/RXS-0255（体 panic 面）三门皆为 AST 层结构性检查
//! （export_c.rs `collect_c_exports`）。故本批跑直接驱动收集管线,隔离于宿主
//! 目标（main/EXE）语义,accept 无 `main` 亦不污染（driver 层 `--emit=dll` 通道）。
//!
//! reject 体例:文件头次行 `//@ expect-error: RX####`（镜像 conformance/coloring/reject）;
//! 批跑断言"收集管线产生诊断且全部为预期码"（反例全拦截口径）。
//! 空导出集 RX6032 = driver 层 `emit_dll` 发射（非 `collect_c_exports`),不入本语料,
//! 改由 ci/export_c_smoke.py 步骤 71 host 段覆盖。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::export_c::collect_c_exports;
use rurixc::lexer::lex;
use rurixc::parser::parse;
use rurixc::source_map::SourceMap;
use rurixc::span::Edition;

/// 本语料覆盖的条款全集（accept/reject 合并须每条 ≥1 锚,RFC-0014 Part A）。
const CLAUSES: [&str; 6] = [
    "RXS-0250", "RXS-0251", "RXS-0252", "RXS-0253", "RXS-0254", "RXS-0255",
];

fn root_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/export_c")
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

/// lex → parse → `collect_c_exports`,返回收集管线发射的错误码序列（RX6031/6033;
/// 空导出集 RX6032 属 driver 层,不在此路径）。
fn run(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let mut sm = SourceMap::new();
    let id = sm.add_file("t.rx", src, Edition::Rx0);
    let toks = lex(src, id, Edition::Rx0, &diag);
    let ast = parse(src, toks, id, Edition::Rx0, &diag);
    collect_c_exports(&ast.items, &sm, &diag);
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

#[test]
fn accept_corpus_is_export_c_diagnostic_free() {
    let files = rx_files(&root_dir("accept"));
    assert!(!files.is_empty(), "export_c accept 正例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let codes = run(&src);
        assert!(
            codes.is_empty(),
            "{} 产生 export_c 诊断: {codes:?}（accept 正例须 0 诊断）",
            f.display()
        );
    }
}

#[test]
fn reject_corpus_all_intercepted() {
    let files = rx_files(&root_dir("reject"));
    assert!(!files.is_empty(), "export_c reject 反例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let expected: u16 = src
            .lines()
            .find_map(|l| l.trim().strip_prefix("//@ expect-error: RX"))
            .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", f.display()))
            .trim()
            .parse()
            .expect("expect-error 码格式非法");
        let codes = run(&src);
        assert!(
            !codes.is_empty(),
            "{} 未被拦截（反例全拦截口径）",
            f.display()
        );
        assert!(
            codes.iter().all(|c| *c == expected),
            "{} 诊断码偏离预期 RX{expected}: {codes:?}",
            f.display()
        );
    }
}

/// 每 `.rx` 首行 `//@ spec: RXS-####` 条款锚（trace_matrix 扫此锚）。
#[test]
fn corpus_files_carry_spec_anchor() {
    for sub in ["accept", "reject"] {
        for f in rx_files(&root_dir(sub)) {
            let src = fs::read_to_string(&f).expect("读取样例失败");
            let first = src.lines().next().unwrap_or("");
            assert!(
                first.starts_with("//@ spec: RXS-"),
                "{} 缺条款锚定头（//@ spec: RXS-####）",
                f.display()
            );
        }
    }
}

/// accept/reject 合并须覆盖 RXS-0250~0255 全部 6 条（每条 ≥1 锚,RFC-0014 Part A）。
#[test]
fn corpus_covers_all_six_clauses() {
    let mut seen = String::new();
    for sub in ["accept", "reject"] {
        for f in rx_files(&root_dir(sub)) {
            seen.push_str(&fs::read_to_string(&f).expect("读取样例失败"));
        }
    }
    for clause in CLAUSES {
        assert!(
            seen.contains(clause),
            "语料缺 {clause} 锚（RXS-0250~0255 每条须 ≥1 锚）"
        );
    }
}
