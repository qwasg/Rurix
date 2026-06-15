//! conformance/resolve + conformance/typeck 语义正例跑批(契约 D-M2-1/D-M2-4;
//! 作用面留痕见 M2_PLAN 修订记录 v1.1/v1.2)。
//!
//! 门作用面(M2.2 起升级):`conformance/resolve/` 与 `conformance/typeck/`
//! 全量(自包含程序)—— lex + parse + resolve + typeck 0 诊断且产出 HIR;
//! `conformance/syntax/` 维持 parse 门(含草图引用与故意的语义反例)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn corpus_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");
    vec![root.join("resolve"), root.join("typeck")]
}

fn collect_rx_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("读取语义样例目录失败") {
        let path = entry.expect("读取目录项失败").path();
        if path.is_dir() {
            collect_rx_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rx") {
            out.push(path);
        }
    }
}

fn corpus() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in corpus_dirs() {
        collect_rx_files(&dir, &mut files);
    }
    files.sort();
    files
}

#[test]
fn semantic_corpus_is_not_empty() {
    let n = corpus().len();
    assert!(n >= 20, "语义正例集过小: {n} 个(M2_PLAN §1/§2:合计 >=20)");
}

/// M2.1/M2.2 出口判据:语义正例全量 0 诊断(lex + parse + resolve + typeck)。
///
/// corpus↔driver 阶段顺序一致性(M0–M6 审查):本门为 **accept-only**,契约即
/// "任一阶段产生任何诊断即失败"(`diag.emitted().is_empty()`)——比 `*_corpus.rs`
/// reject 用的 fail-fast(`if !has_errors()` 钉死首报码、防级联抢报)语义**更强**,
/// 且 driver 对干净输入本就跑完整前缀。故阶段间不补 fail-fast 判定:0 诊断契约下
/// 它纯属无操作,且会暗示并不存在的 reject 处理(见 `pipeline_consistency.rs` 文件头)。
#[test]
fn semantic_corpus_is_diagnostic_free() {
    for file in corpus() {
        let src = fs::read_to_string(&file).expect("读取样例失败");
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate(); // 经 query 通道:resolve + lower + typeck
        cx.check_crate_patterns(); // M3.1:模式穷尽性同入正例门(RXS-0051)
        assert!(
            diag.emitted().is_empty(),
            "{} 产生诊断: {:?}",
            file.display(),
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message(diag.messages())))
                .collect::<Vec<_>>()
        );
        assert!(
            !cx.hir_crate().root_items.is_empty(),
            "{} 未产出 HIR item",
            file.display()
        );
    }
}

#[test]
fn semantic_corpus_files_carry_spec_anchor() {
    for file in corpus() {
        let src = fs::read_to_string(&file).expect("读取样例失败");
        let first = src.lines().next().unwrap_or("");
        assert!(
            first.starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头(//@ spec: RXS-####)",
            file.display()
        );
    }
}
