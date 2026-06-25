//! conformance/dxil DXIL 第二后端语料批跑(G2.2 PR-C2 分片1,RFC-0003;cargo
//! feature `dxil-backend`)。RXS-0157:codegen target 分发与 DXIL 后端分叉——
//! accept(合法最小 compute kernel 经 DXIL 后端产 DirectX 三元组 LLVM IR,0 诊断)+
//! reject(子集外构造 / target 不支持 → RX6007,strict-only 无 fallback)。
//!
//! 管线:resolve → typeck → 着色/barrier → 穷尽性 → const eval → `dxil_codegen::
//! build_and_emit_dxil`(device MIR kernel 根 → DXIL IR)。纯 host/CPU-only(本测试
//! 仅到 IR emit;patched llc → DXIL 容器 → dxc validator 真跑由 `dxil_golden` /
//! `rx build --target dxil` 工具链关卡覆盖)。reject 体例:文件次行
//! `//@ expect-error: RX####`。
#![cfg(feature = "dxil-backend")]

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn dxil_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/dxil")
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

/// resolve → typeck → 着色 → 穷尽性 → const eval → DXIL codegen(阶段化:前段有错
/// 即停)。返回 (DXIL IR 文本 Option, 错误码序列)。
fn run_dxil(src: &str, module: &str) -> (Option<String>, Vec<u16>) {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if !diag.has_errors() {
        cx.check_coloring();
    }
    if !diag.has_errors() {
        cx.check_crate_patterns();
    }
    if !diag.has_errors() {
        cx.check_consteval();
    }
    let ir = if !diag.has_errors() {
        rurixc::dxil_codegen::build_and_emit_dxil(&cx, module)
    } else {
        None
    };
    let codes = diag
        .emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect();
    (ir, codes)
}

/// accept 正例:0 诊断 + 产出 DirectX 三元组 DXIL IR(compute shader 形态)。
#[test]
fn accept_corpus_emits_dxil() {
    let files = rx_files(&dxil_dir("accept"));
    assert!(!files.is_empty(), "conformance/dxil/accept 正例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        let (ir, codes) = run_dxil(&src, &stem);
        assert!(
            codes.is_empty(),
            "{} 产生诊断: {codes:?}(accept 须 0 诊断)",
            f.display()
        );
        let ir = ir.unwrap_or_else(|| panic!("{} 未产出 DXIL IR", f.display()));
        assert!(
            ir.contains("target triple = \"dxil-unknown-shadermodel6.0-compute\""),
            "{} DXIL IR 缺 DirectX 三元组",
            f.display()
        );
        assert!(
            ir.contains("\"hlsl.shader\"=\"compute\""),
            "{} DXIL IR 缺 hlsl.shader=compute 入口属性",
            f.display()
        );
    }
}

/// reject 反例:全拦截到 `//@ expect-error` 声明的码(RX6007 全覆盖)。
#[test]
fn reject_corpus_all_intercepted() {
    let files = rx_files(&dxil_dir("reject"));
    assert!(!files.is_empty(), "conformance/dxil/reject 反例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let expected: u16 = src
            .lines()
            .find_map(|l| l.trim().strip_prefix("//@ expect-error: RX"))
            .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", f.display()))
            .trim()
            .parse()
            .expect("expect-error 码格式非法");
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        let (ir, codes) = run_dxil(&src, &stem);
        assert!(ir.is_none(), "{} 不应产出 DXIL IR(reject)", f.display());
        assert!(
            codes.contains(&expected),
            "{} 未拦截到 RX{expected}: {codes:?}",
            f.display()
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for f in rx_files(&dxil_dir("")) {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let first = src.lines().next().unwrap_or("");
        assert!(
            first.starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头(//@ spec: RXS-####)",
            f.display()
        );
    }
}
