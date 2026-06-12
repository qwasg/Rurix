//! UI golden 测试通道(契约 D-M1-4 / G-M1-2;14 §6 受控 bless)。
//!
//! compiletest 风格:
//! - `tests/ui/**/*.rx` 逐文件跑 lex + parse + feature gate,渲染诊断;
//! - `//~ ERROR RX####` 行注释比对:注释所在行必须有同码诊断(主 span 行号对齐),
//!   且 error 级诊断数与注释数一致(防漏标/多标);
//! - `.stderr` snapshot 字节比对(路径规范化为 `$DIR/...`,LF 行尾);
//! - **bless 是审批动作**:`RURIX_BLESS=1` 重写 snapshot;`.stderr` 变更必须
//!   伴随 `tests/ui/bless_log.md` 追加记录(ci/check_guardrails.py 核对)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::{DiagCtxt, Level};
use rurixc::feature_gate::check_feature_gates;
use rurixc::lexer::lex;
use rurixc::parser::parse;
use rurixc::query::QueryCtx;
use rurixc::render::render_diagnostics;
use rurixc::source_map::SourceMap;
use rurixc::span::Edition;

fn ui_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/ui")
}

fn collect_rx_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("读取 tests/ui 失败") {
        let path = entry.expect("读取目录项失败").path();
        if path.is_dir() {
            collect_rx_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rx") {
            out.push(path);
        }
    }
}

fn ui_tests() -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rx_files(&ui_dir(), &mut files);
    files.sort();
    files
}

/// 规范化文件名:`$DIR/<tests/ui 下相对路径,正斜杠>`。
fn normalized_name(path: &Path) -> String {
    let rel = path
        .strip_prefix(ui_dir())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    format!("$DIR/{rel}")
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/// 提取 `//~ ERROR RX####` 行注释:(1-based 行号, 错误码文本)。
fn expected_annotations(src: &str) -> Vec<(u32, String)> {
    let mut anns = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let mut rest = line;
        while let Some(pos) = rest.find("//~ ERROR ") {
            let after = &rest[pos + "//~ ERROR ".len()..];
            let code: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            assert!(
                code.starts_with("RX") && code.len() == 6,
                "{}: 第 {} 行注释错误码格式非法: {code:?}",
                i + 1,
                i + 1
            );
            anns.push((i as u32 + 1, code));
            rest = after;
        }
    }
    anns
}

struct CaseResult {
    rendered: String,
    /// (行号, 错误码) — error 级诊断的主 span 行号
    errors: Vec<(u32, String)>,
}

fn run_case(path: &Path, src: &str) -> CaseResult {
    let name = normalized_name(path);
    let diag = DiagCtxt::new();
    let mut sm = SourceMap::new();
    let id = sm.add_file(name, src, Edition::Rx0);
    let tokens = lex(src, id, Edition::Rx0, &diag);
    let ast = parse(src, tokens, id, Edition::Rx0, &diag);
    check_feature_gates(&ast, &diag);
    // 阶段化(对齐 rustc:前一阶段有错即停,防级联污染 snapshot):
    // parse/gate 干净 → 名称解析(M2.1,1xxx);resolve 干净 → typeck(M2.2,2xxx)
    if !diag.has_errors() {
        let cx = QueryCtx::from_ast(ast, src, id, &diag);
        let _ = cx.resolutions();
        if !diag.has_errors() {
            cx.check_crate();
        }
    }
    let emitted = diag.emitted();
    let rendered = render_diagnostics(&emitted, &sm, diag.messages());
    let errors = emitted
        .iter()
        .filter(|d| d.level == Level::Error)
        .map(|d| {
            let line = d
                .labels
                .first()
                .map(|l| sm.lookup(l.span.file, l.span.lo).line)
                .unwrap_or(0);
            let code = d.code.map(|c| c.to_string()).unwrap_or_default();
            (line, code)
        })
        .collect();
    CaseResult { rendered, errors }
}

fn bless_mode() -> bool {
    std::env::var("RURIX_BLESS").is_ok_and(|v| v == "1")
}

#[test]
fn ui_corpus_is_not_empty() {
    let n = ui_tests().len();
    assert!(
        n >= 10,
        "UI golden 样例过少: {n} 个(G-M1-2 / m1.counter.ui_golden_path1_snapshots: >=10)"
    );
}

/// `//~ ERROR RX####` 注释比对:逐行逐码对齐,计数一致。
#[test]
fn ui_error_annotations_match() {
    for path in ui_tests() {
        let src = normalize_newlines(&fs::read_to_string(&path).expect("读取样例失败"));
        let case = run_case(&path, &src);
        let mut expected = expected_annotations(&src);
        let mut actual = case.errors.clone();
        expected.sort();
        actual.sort();
        assert_eq!(
            expected,
            actual,
            "{}: 注释与诊断不匹配\n  expected(行,码): {expected:?}\n  actual(行,码): {actual:?}\n  rendered:\n{}",
            path.display(),
            case.rendered
        );
        assert!(
            !expected.is_empty(),
            "{}: UI 测试必须至少标注一条 //~ ERROR(黄金路径 1 = 解析错误)",
            path.display()
        );
    }
}

/// `.stderr` snapshot 比对;`RURIX_BLESS=1` 时重写(受控审批动作,14 §6)。
#[test]
fn ui_stderr_snapshots_match() {
    let bless = bless_mode();
    let mut mismatches = Vec::new();
    for path in ui_tests() {
        let src = normalize_newlines(&fs::read_to_string(&path).expect("读取样例失败"));
        let case = run_case(&path, &src);
        let stderr_path = path.with_extension("stderr");
        if bless {
            fs::write(&stderr_path, &case.rendered).expect("bless 写入失败");
            continue;
        }
        let expected = match fs::read_to_string(&stderr_path) {
            Ok(s) => normalize_newlines(&s),
            Err(_) => {
                mismatches.push(format!(
                    "{}: 缺 .stderr snapshot(新测试需经审批 bless:RURIX_BLESS=1 + bless_log.md 留痕)",
                    stderr_path.display()
                ));
                continue;
            }
        };
        if expected != case.rendered {
            mismatches.push(format!(
                "{}: snapshot 不匹配\n--- expected ---\n{expected}\n--- actual ---\n{}",
                stderr_path.display(),
                case.rendered
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "UI snapshot 比对失败({} 处):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// 每个 UI 样例携带条款锚定(traceability,G-M1-4)。
#[test]
fn ui_files_carry_spec_anchor() {
    for path in ui_tests() {
        let src = fs::read_to_string(&path).expect("读取样例失败");
        let first = src.lines().next().unwrap_or("");
        assert!(
            first.starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头(//@ spec: RXS-####)",
            path.display()
        );
    }
}
