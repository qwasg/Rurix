//! conformance/stdlib 语料批跑(M7.1,契约 D-M7-1;core 数学库 Vec/Mat/swizzle 类型面,
//! spec/stdlib.md RXS-0104 ~ RXS-0109)。
//!
//! - `host/`:具体 f32 结构体 + inherent `device fn` 方法的类型面正例(host 路径),
//!   经检查管线(check → coloring → patterns → consteval)须全程 0 诊断。host 真跑
//!   断言由 `ci/stdlib_math_smoke.py` 覆盖(本测试只断言 0 诊断)。
//! - `device/`:语义同义的标量分量 `device fn` 原语 + kernel(device 路径),经
//!   device codegen 须产 NVPTX IR 且 0 诊断(聚合值类型 device codegen 为后续扩展,
//!   现以标量子集实现,spec/stdlib.md §5)。
//! - `reject/<cat>/`:误用须命中既有 2xxx 类型类诊断(`//@ expect-error: RX####`,
//!   非法 swizzle → RX2004 / 维度不匹配 → RX2001)。
//!
//! 全部 `.rx` 须携带条款锚定头(`//@ spec: RXS-####`,trace_matrix 消费)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn stdlib_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/stdlib")
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

/// 检查管线(check → coloring → patterns → consteval);返回诊断码集合(不跑 device codegen)。
fn check_codes(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if !diag.has_errors() {
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

/// 全管线 + device codegen(与 libdevice 语料同口径);返回诊断码集合。
fn device_codes(src: &str, module: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if !diag.has_errors() {
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        if !diag.has_errors() {
            let _ = rurixc::device_codegen::build_and_emit(&cx, module);
        }
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

/// 解析 `//@ expect-error: RX####` 头中的错误码数字。
fn expected_code(src: &str) -> Option<u16> {
    for line in src.lines() {
        let s = line.trim();
        if let Some(rest) = s.strip_prefix("//@ expect-error:") {
            let t = rest.trim();
            let digits: String = t
                .trim_start_matches("RX")
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            return digits.parse::<u16>().ok();
        }
    }
    None
}

#[test]
fn host_accept_is_diagnostic_free() {
    let files = rx_files(&stdlib_dir("host"));
    assert!(!files.is_empty(), "conformance/stdlib/host 正例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let codes = check_codes(&src);
        assert!(
            codes.is_empty(),
            "{} 产生诊断: {codes:?}(host 正例须检查管线 0 诊断)",
            f.display()
        );
    }
}

#[test]
fn device_accept_emits_ir() {
    let files = rx_files(&stdlib_dir("device"));
    assert!(!files.is_empty(), "conformance/stdlib/device 正例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        let codes = device_codes(&src, &stem);
        assert!(
            codes.is_empty(),
            "{} 产生诊断: {codes:?}(device 标量正例须全管线 + device codegen 0 诊断)",
            f.display()
        );
    }
}

#[test]
fn reject_hits_expected_code() {
    let files = rx_files(&stdlib_dir("reject"));
    assert!(!files.is_empty(), "conformance/stdlib/reject 反例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let want = expected_code(&src)
            .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", f.display()));
        let codes = check_codes(&src);
        assert!(
            codes.contains(&want),
            "{} 期待诊断 RX{want:04} 未命中: 实得 {codes:?}",
            f.display()
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for sub in ["host", "device", "reject"] {
        for f in rx_files(&stdlib_dir(sub)) {
            let src = fs::read_to_string(&f).expect("读取样例失败");
            let first = src.lines().next().unwrap_or("");
            assert!(
                first.starts_with("//@ spec: RXS-"),
                "{} 缺条款锚定头(//@ spec: RXS-####)",
                f.display()
            );
        }
    }
}
