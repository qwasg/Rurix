//! corpus↔driver 阶段一致性回归(M0–M6 审查·corpus↔driver 阶段顺序一致性审计)。
//!
//! 各 `*_corpus.rs` 用窄口径 `run_pipeline` 跑各自语料;本测试用单一
//! [`driver_check_codes`] 复刻 driver `--emit=check` 的**完整静态检查顺序**
//! (driver.rs::compile,至 `check_device_safety` 止,不跑 codegen),把 driver
//! 口径钉死到各 conformance reject 样例的 `//@ expect-error` 文档化期望:
//!
//!   driver `--emit=check` 首报码 == 样例 expect-error == 各 corpus 窄管线报码
//!
//! 三段相等 ⇒ corpus 口径 == driver 口径。另对历史已知漂移点(shared 标量 vs
//! 数组、launch+views 组合)做显式交叉断言,锁定"前序阶段抢报"的同口径裁决。
//!
//! 复刻顺序须与 [`rurixc::driver`] 的 `compile` 严格一致(本文件不抽公共函数,避免
//! 架构重构;若 driver 顺序变更,本 helper 须同步)。

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::feature_gate::check_feature_gates;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

/// 复刻 driver `--emit=check` 完整静态检查顺序(driver.rs::compile):
/// gate → resolve → typeck → coloring → launch → patterns → consteval → mir →
/// moves → borrows → views → shared → device_safety,逐段前段有错即停;返回
/// 全部诊断错误码序列(不跑 codegen,与 `--emit=check` 同口径)。
fn driver_check_codes(src: &str) -> Vec<u16> {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    // gate(driver 在 resolve 前跑 check_feature_gates(ast))
    check_feature_gates(cx.ast(), &diag);
    if !diag.has_errors() {
        let _ = cx.resolutions();
        if !diag.has_errors() {
            cx.check_crate(); // typeck(含 resolve memo)
            if !diag.has_errors() {
                cx.check_coloring();
            }
            if !diag.has_errors() {
                cx.check_launch();
            }
            if !diag.has_errors() {
                cx.check_crate_patterns();
            }
            if !diag.has_errors() {
                cx.check_consteval();
            }
            if !diag.has_errors() {
                let _ = cx.mir_crate();
                cx.check_moves();
            }
            if !diag.has_errors() {
                cx.check_borrows();
            }
            if !diag.has_errors() {
                cx.check_views();
            }
            if !diag.has_errors() {
                cx.check_shared_barrier();
            }
            if !diag.has_errors() {
                cx.check_device_safety();
            }
        }
    }
    diag.emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect()
}

fn conformance_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance")
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

fn expect_error(src: &str, path: &Path) -> u16 {
    src.lines()
        .find_map(|l| l.trim().strip_prefix("//@ expect-error: RX"))
        .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", path.display()))
        .trim()
        .parse()
        .expect("expect-error 码格式非法")
}

/// 核心一致性回归:遍历各域 conformance reject 样例,断言 driver `--emit=check`
/// 顺序首报码序列与样例 `//@ expect-error` 完全一致(反例全拦截 + 同码,不被前序
/// 阶段抢报为异码)。覆盖全部 3xxx/4xxx/6xxx reject 语料域(九域):
/// borrowck(4xxx)/ shared(6xxx)/ views(3xxx)/ launch(2xxx/3xxx)/ atomics(3xxx)/
/// coloring(RX3001/3003,check_coloring 首报)/ addrspace(RX3002,typeck 首报)/
/// device(RX6005,typeck 维度契约首报)/ libdevice(RX6006,typeck legality 首报)。
#[test]
fn driver_check_matches_expect_error_across_domains() {
    let domains = [
        "borrowck/reject",
        "shared/reject",
        "views/reject",
        "launch/reject",
        "atomics/reject",
        "coloring/reject",
        "addrspace/reject",
        "device/reject",
        "libdevice/reject",
    ];
    let mut total = 0usize;
    for dom in domains {
        let files = rx_files(&conformance_dir(dom));
        assert!(!files.is_empty(), "{dom} reject 反例集为空");
        for f in files {
            let src = fs::read_to_string(&f).expect("读取样例失败");
            let expected = expect_error(&src, &f);
            let codes = driver_check_codes(&src);
            assert!(
                !codes.is_empty(),
                "{} 在 driver --emit=check 顺序下未被拦截",
                f.display()
            );
            assert!(
                codes.iter().all(|c| *c == expected),
                "{} driver --emit=check 顺序报码偏离 expect-error RX{expected}: {codes:?}\
                 (corpus↔driver 阶段漂移)",
                f.display()
            );
            total += 1;
        }
    }
    assert!(total >= 10, "一致性回归覆盖样例过少: {total}");
}

// ---- 历史已知漂移点的显式交叉断言 ----

/// 漂移点 1:`shared let` **标量**(非数组)。device 安全门(check_device_safety)
/// 会把"先读未写"报为 use-before-init(RX4002),但 driver 顺序中 check_shared_barrier
/// 在 device_safety **之前**,先报 shared 形状违例 RX6005。借用门未带 shared 检查的
/// 旧 borrowck corpus 曾报 RX4002 → 与 driver 漂移。此处锁定:driver 顺序报 RX6005,
/// 不是 RX4002。
#[test]
fn shared_scalar_driver_reports_rx6005_not_rx4002() {
    let src = "\
kernel fn k(t: ThreadCtx<1>, dst: ViewMut<global, f32>) {
    shared let x: f32;
    let i = t.thread_index();
    dst[i] = x;
}
fn main() {}
";
    let codes = driver_check_codes(src);
    assert!(
        codes.contains(&6005),
        "shared 标量应由 check_shared_barrier 报 RX6005: {codes:?}"
    );
    assert!(
        !codes.contains(&4002),
        "shared 标量不应被 device 安全门抢报 RX4002(顺序漂移): {codes:?}"
    );
}

/// 漂移点 1 对照:`shared let` **数组**未经元素写先读。shared 形状合法(数组),
/// check_shared_barrier 不报;由 device 安全门报 use-before-init RX4002。证实 RX6005
/// 仅因"标量形状"而非"未写先读"——两者在 driver 顺序下分别归位、不混淆。
#[test]
fn shared_array_unwritten_driver_reports_rx4002() {
    let src = "\
kernel fn k(t: ThreadCtx<1>, dst: ViewMut<global, f32>) {
    shared let tile: [f32; 64];
    let i = t.thread_index();
    dst[i] = tile[i];
}
fn main() {}
";
    let codes = driver_check_codes(src);
    assert!(
        codes.contains(&4002),
        "shared 数组未写先读应由 device 安全门报 RX4002: {codes:?}"
    );
    assert!(
        !codes.contains(&6005),
        "合法 shared 数组形状不应报 RX6005: {codes:?}"
    );
}

/// 漂移点 2:同一源同时含 launch 契约违例(launch 非 kernel,RX3004)与 views 重叠
/// 可变写(RX3007)。driver 顺序为 coloring→launch→…→views,launch 在 views 之前,
/// 故首报 RX3004。views/shared corpus 未插 check_launch 时会先报 RX3007 → 漂移。
/// 此处锁定 driver 顺序报 RX3004(launch 抢在 views 前),并交叉断言"着色→launch→
/// views"窄复刻同码,证实 corpus 插 launch 后口径一致。
#[test]
fn launch_preempts_views_driver_reports_rx3004() {
    let src = "\
device fn helper(out: ViewMut<global, f32>) {}

kernel fn windows_overlap(v: ViewMut<global, f32>, i: usize, t: ThreadCtx<1>) {
    let w = v.windows(2);
    w[i] = 1.0;
}

fn run<C>(stream: Stream<C>, out: Buffer<C, f32>) {
    stream.launch(helper, GridDim(1), BlockDim(1), (out,));
}

fn main() {}
";
    let codes = driver_check_codes(src);
    assert_eq!(
        codes,
        vec![3004],
        "launch 应抢在 views 之前报 RX3004(防 views 抢报 RX3007): {codes:?}"
    );

    // 着色→launch→views 窄复刻(shared/views corpus 插 launch 后的顺序)同口径。
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
    let narrow: Vec<u16> = diag
        .emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect();
    assert_eq!(narrow, codes, "窄复刻(着色→launch→views)须与 driver 同口径");
}
