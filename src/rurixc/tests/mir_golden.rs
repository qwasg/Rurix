//! MIR 文本 golden guardrail(M3.3 WP6;14 §2 常驻集,M3 CI_GATES §4 第 2 项)。
//!
//! M3.2 预评估(M3 CI_GATES v1.2 §4)裁决的形态落地:
//! - 基线 = 全管线 → `mir::pretty` 文本逐字节(与 `rurixc --emit=mir` 同源);
//! - 语料 `tests/mir/**/*.rx`,golden = 同名 `.mir`;三类形态代表
//!   (无 drop / drop 顺序 / 条件初始化 drop flag);
//! - **bless 是审批动作**:`RURIX_BLESS=1` 重写 `.mir`;`.mir` 变更必须伴随
//!   `tests/mir/bless_log.md` 追加记录(ci/check_guardrails.py `check_mir_bless` 核对)。
//!
//! MIR 形态在 M3.2(Move/Drop 落地)后定型,M3.3 NLL 借用检查为只读 pass 不改形态,
//! 故 golden 基线在 WP6 安全入库。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn mir_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/mir")
}

fn rx_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![mir_dir()];
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

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/// 全管线产出 MIR 文本(drop elaboration 已在 mir_crate 内落定;借用检查只读不改形态)。
fn mir_text(src: &str) -> String {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    let mut text = String::new();
    if !diag.has_errors() {
        cx.check_crate_patterns();
        if !diag.has_errors() {
            let bodies = cx.mir_crate();
            let res = cx.resolutions();
            cx.check_moves();
            if !diag.has_errors() {
                cx.check_borrows();
            }
            // 与 `rurixc --emit=mir` 逐字节同源:body 间无额外分隔(pretty 各自以
            // `}\n` 收尾,直接拼接 = CLI `for b { print!("{}", pretty(b)) }` 形态)。
            text = bodies
                .iter()
                .map(|b| rurixc::mir::pretty(b, &res))
                .collect::<String>();
        }
    }
    let codes: Vec<u16> = diag
        .emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect();
    assert!(
        codes.is_empty(),
        "MIR golden 语料须 0 诊断(代表程序均为 accept),实得: {codes:?}"
    );
    text
}

fn bless_mode() -> bool {
    std::env::var("RURIX_BLESS").is_ok_and(|v| v == "1")
}

/// 语料 ≥3(三类形态代表;M3 CI_GATES §4 第 2 项基线范围)。
#[test]
fn mir_corpus_is_not_empty() {
    let n = rx_files().len();
    assert!(
        n >= 3,
        "MIR golden 语料过少: {n} 个(基线三代表:无 drop / drop 顺序 / 条件初始化 drop flag)"
    );
}

/// `.mir` golden 逐字节比对;`RURIX_BLESS=1` 时重写(受控审批动作,对齐 UI bless)。
#[test]
fn mir_golden_snapshots_match() {
    let bless = bless_mode();
    let mut mismatches = Vec::new();
    for path in rx_files() {
        let src = normalize_newlines(&fs::read_to_string(&path).expect("读取语料失败"));
        let text = mir_text(&src);
        let golden_path = path.with_extension("mir");
        if bless {
            fs::write(&golden_path, &text).expect("bless 写入失败");
            continue;
        }
        let expected = match fs::read_to_string(&golden_path) {
            Ok(s) => normalize_newlines(&s),
            Err(_) => {
                mismatches.push(format!(
                    "{}: 缺 .mir golden(新语料需经审批 bless:RURIX_BLESS=1 + bless_log.md 留痕)",
                    golden_path.display()
                ));
                continue;
            }
        };
        if expected != text {
            mismatches.push(format!(
                "{}: MIR golden 漂移\n--- expected ---\n{expected}\n--- actual ---\n{text}",
                golden_path.display()
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "MIR golden 比对失败({} 处):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// 每个语料携带条款锚定(traceability,对齐 ui_golden / borrowck_corpus)。
#[test]
fn mir_files_carry_spec_anchor() {
    for path in rx_files() {
        let src = fs::read_to_string(&path).expect("读取语料失败");
        let first = src.lines().next().unwrap_or("");
        assert!(
            first.starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头(//@ spec: RXS-####)",
            path.display()
        );
    }
}
