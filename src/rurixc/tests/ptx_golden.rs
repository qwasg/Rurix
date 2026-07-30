//! NVPTX IR 文本 golden guardrail(M4.2;14 §2 常驻集 / 07 §11,M4 CI_GATES §4 第 3 项)。
//!
//! 形态(M4 CI_GATES §4.3 在激活 PR 中裁决):
//! - 基线 = device MIR(`kernel fn` 为根)→ `device_codegen::build_and_emit` 产出的
//!   **NVPTX 约束 LLVM IR 文本**(rurixc 自有产物,确定性、无外部工具依赖;PTX 为
//!   下游 clang/NVPTX 后端汇编产物,其字节稳定性绑定工具链版本,故 golden 取 IR 层);
//! - 语料 `tests/ptx/**/*.rx`,golden = 同名 `.nvptx`;
//! - **bless 是审批动作**:`RURIX_BLESS=1` 重写 `.nvptx`;变更必须伴随
//!   `tests/ptx/bless_log.md` 追加记录(ci/check_guardrails.py `check_ptx_bless` 核对)。
//!
//! NVPTX IR 形态随 M4.2 device codegen 定型(RXS-0070~0072),M4.3 launch/装载为
//! 上层不改 codegen 形态,故 golden 基线在 M4.2 安全入库。clang IR→PTX→ptxas 真跑
//! 关卡由 `rurixc --emit=ptx`(PR Smoke 步骤 17)覆盖。

use std::fs;
use std::path::PathBuf;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

mod common;
use common::{assert_spec_anchor, bless_mode, check_golden, read_corpus_normalized, tests_dir};

fn ptx_dir() -> PathBuf {
    tests_dir("ptx")
}

fn rx_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![ptx_dir()];
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

/// 全管线产出 device NVPTX IR 文本(`kernel fn` 为根;0 诊断断言)。
fn nvptx_text(src: &str, module_name: &str) -> String {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    let mut text = String::new();
    if !diag.has_errors() {
        cx.check_coloring();
        if !diag.has_errors() {
            cx.check_crate_patterns();
            if !diag.has_errors() {
                cx.check_consteval();
            }
            if !diag.has_errors()
                && let Some(ir) = rurixc::device_codegen::build_and_emit(&cx, module_name)
            {
                text = ir;
            }
        }
    }
    let codes: Vec<u16> = diag
        .emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect();
    assert!(
        codes.is_empty(),
        "PTX golden 语料须 0 诊断(代表 kernel 均为 accept),实得: {codes:?}"
    );
    text
}

/// 语料 ≥2(SAXPY 雏形 + 线程索引代表;M4 CI_GATES §4.3 起步范围)。
#[test]
fn ptx_corpus_is_not_empty() {
    let n = rx_files().len();
    assert!(
        n >= 2,
        "PTX golden 语料过少: {n} 个(起步:SAXPY 雏形 + 线程索引代表)"
    );
}

/// `.nvptx` golden 逐字节比对;`RURIX_BLESS=1` 时重写(受控审批动作,对齐 MIR bless)。
#[test]
fn ptx_golden_snapshots_match() {
    let bless = bless_mode();
    let mut mismatches = Vec::new();
    for path in rx_files() {
        let src = read_corpus_normalized(&path);
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let text = nvptx_text(&src, &stem);
        let golden_path = path.with_extension("nvptx");
        mismatches.extend(check_golden(&golden_path, &text, bless, "NVPTX IR"));
    }
    assert!(
        mismatches.is_empty(),
        "PTX golden 比对失败({} 处):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// 每个语料携带条款锚定(traceability,对齐 mir_golden / ui_golden)。
#[test]
fn ptx_files_carry_spec_anchor() {
    for path in rx_files() {
        let src = fs::read_to_string(&path).expect("读取语料失败");
        assert_spec_anchor(&src, &path);
    }
}
