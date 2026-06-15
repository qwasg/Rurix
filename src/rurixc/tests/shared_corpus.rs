//! conformance/shared 语料批跑(M5.2,契约 D-M5-2 / G-M5-3;shared+barrier 一致性
//! 数据流违例全拦截与 accept 正例 0 诊断,RXS-0079)。
//!
//! 管线:resolve → typeck(含 `shared let` 存储 / `block.sync()` barrier 定型)→
//! 着色骨架(RX3001/RX3003)→ views 不相交(RX3007/RX3008)→ shared+barrier 一致性
//! device 借用扩展数据流(RX3009);HIR 层,无需 MIR(device 上下文 body 不在 host
//! `main` 可达 MIR 内,07 §4)。reject 体例:`reject/<category>/*.rx`,文件头次行
//! `//@ expect-error: RX####`;批跑断言"产生诊断且全部为预期码"(反例全拦截口径)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn shared_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/shared")
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

/// resolve → typeck → 着色 → launch → views → shared+barrier 一致性(HIR 层,无 MIR)。
/// 阶段化:前段有错即停(防级联),shared 在 typeck/着色/launch/views 干净后跑。
/// `check_launch` 介于着色与 views 之间是为消除 corpus 顺序漂移(driver 顺序为
/// coloring→launch→…→views→shared):含 launch 契约违例(RX3004/3005/3006)的样例
/// 在 driver 会先被 launch 抢报,corpus 须同口径,而非直达 views/shared。
fn run_pipeline(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if !diag.has_errors() {
        cx.check_coloring();
    }
    if !diag.has_errors() {
        cx.check_launch();
    }
    if !diag.has_errors() {
        cx.check_views();
    }
    if !diag.has_errors() {
        cx.check_shared_barrier();
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

#[test]
fn accept_corpus_is_diagnostic_free() {
    let files = rx_files(&shared_dir("accept"));
    assert!(!files.is_empty(), "shared accept 正例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let codes = run_pipeline(&src);
        assert!(
            codes.is_empty(),
            "{} 产生诊断: {codes:?}(accept 正例须 0 诊断)",
            f.display()
        );
    }
}

#[test]
fn reject_corpus_all_intercepted() {
    let files = rx_files(&shared_dir("reject"));
    assert!(!files.is_empty(), "shared reject 反例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let expected: u16 = src
            .lines()
            .find_map(|l| l.trim().strip_prefix("//@ expect-error: RX"))
            .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", f.display()))
            .trim()
            .parse()
            .expect("expect-error 码格式非法");
        let codes = run_pipeline(&src);
        assert!(
            !codes.is_empty(),
            "{} 未被拦截(反例全拦截口径)",
            f.display()
        );
        assert!(
            codes.iter().all(|c| *c == expected),
            "{} 诊断码偏离预期 RX{expected}: {codes:?}",
            f.display()
        );
    }
}

/// 反例覆盖预设类别(shared+barrier 一致性;目录即类别)。
#[test]
fn reject_has_expected_categories() {
    let reject = shared_dir("reject");
    for cat in ["unsynced_cross_lane_read", "barrier_too_late"] {
        let d = reject.join(cat);
        assert!(
            d.is_dir() && !rx_files(&d).is_empty(),
            "缺类别目录或为空: shared/reject/{cat}/"
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for sub in ["accept", "reject"] {
        for f in rx_files(&shared_dir(sub)) {
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
