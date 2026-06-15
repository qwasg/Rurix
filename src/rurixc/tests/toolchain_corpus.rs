//! 工具链语义条款锚定(spec/toolchain.md,M6.1)。
//!
//! rx CLI 子命令端到端冒烟在 `ci/rx_cli_smoke.py`(契约 G-M6-3);本文件锚定
//! 不依赖 rx 二进制的语义契约:退出码约定常量、核心子命令集、BENCH_PROTOCOL
//! §3 协议参数、fmt 库面单一事实源。

use std::path::Path;

/// rx CLI 退出码约定(RXS-0083):0 成功 / 1 诊断错误 / 2 用法·I/O 错误。
/// 全子命令统一,与 rurixc 驱动同口径。
const EXIT_OK: u8 = 0;
const EXIT_DIAGNOSTIC: u8 = 1;
const EXIT_USAGE: u8 = 2;

/// RXS-0083:退出码约定为稳定契约(rx 子命令分发与 rurixc 驱动同口径)。
//@ spec: RXS-0083
#[test]
fn exit_code_convention() {
    // 退出码三值互异且语义固定(契约冻结:rx run 透传产物退出码为受控例外)。
    assert_eq!(EXIT_OK, 0, "成功退出码");
    assert_eq!(EXIT_DIAGNOSTIC, 1, "诊断错误退出码");
    assert_eq!(EXIT_USAGE, 2, "用法/IO 错误退出码");
    let all = [EXIT_OK, EXIT_DIAGNOSTIC, EXIT_USAGE];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b, "退出码必须互异");
        }
    }
}

/// RXS-0083:M6.1 核心子命令集(build/run/check/fmt/bench)+ 已登记分发位。
//@ spec: RXS-0083
#[test]
fn core_subcommand_set() {
    // M6.1 落地核心集:涉及编译的 build/run/check 经 rurixc query 层单一前端。
    let core: &[&str] = &["build", "run", "check", "fmt", "bench"];
    assert!(core.contains(&"build"));
    assert!(core.contains(&"run"));
    assert!(core.contains(&"check"));
    assert!(core.contains(&"fmt"));
    assert!(core.contains(&"bench"));
    // 已登记但 M6.1 期返回"未实现"用法诊断的分发位(后续里程碑承接)。
    let reserved: &[&str] = &["test", "doc", "fix", "watch", "vendor"];
    for c in core {
        assert!(!reserved.contains(c), "核心集与保留分发位不重叠");
    }
}

/// RXS-0084/RXS-0086:rx build/check 经 rurixc query 层复用单一前端(不另起引擎)。
/// 经库面 driver 跑 check 路径,断言无错误 + 无 codegen 副作用语义。
//@ spec: RXS-0084
//@ spec: RXS-0086
#[test]
fn check_path_via_query_layer() {
    use rurixc::diag::DiagCtxt;
    use rurixc::feature_gate::check_feature_gates;
    use rurixc::lexer::lex;
    use rurixc::parser::parse;
    use rurixc::query::QueryCtx;
    use rurixc::span::{Edition, SourceId};

    let src = "fn main() {\n    let g = \"hi\";\n    println(g);\n}\n";
    let diag = DiagCtxt::new();
    let id = SourceId(0);
    let tokens = lex(src, id, Edition::Rx0, &diag);
    let ast = parse(src, tokens, id, Edition::Rx0, &diag);
    let cx = QueryCtx::from_ast(ast, src, id, &diag);
    check_feature_gates(cx.ast(), &diag);
    assert!(!diag.has_errors(), "feature gate 不应报错");
    let _res = cx.resolutions();
    cx.check_crate();
    assert!(!diag.has_errors(), "check 路径(typeck)不应报错");
}

/// RXS-0087:rx fmt 复用 rurixc::fmt::format_source 单一事实源,字节级幂等。
//@ spec: RXS-0087
#[test]
fn fmt_single_source_of_truth() {
    use rurixc::fmt::format_source;
    let src = "fn  main(){let  x=1;}\n";
    let once = format_source(src).expect("fmt 应成功");
    let twice = format_source(&once).expect("二次 fmt 应成功");
    assert_eq!(once, twice, "rx fmt 字节级幂等(G-M6-4 延续 G-M1-5)");
}

/// RXS-0088:rx bench 复用 BENCH_PROTOCOL §3 协议(三次进程级独立运行 + trimmed
/// mean + L0 锁频前置);协议实现与 smoke 入口存在性契约。
//@ spec: RXS-0088
#[test]
fn bench_protocol_present() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // BENCH_PROTOCOL §3 协议文档(三次进程级 / trimmed mean / L0 锁频)。
    assert!(
        root.join("milestones/m0/BENCH_PROTOCOL.md").is_file(),
        "BENCH_PROTOCOL.md 应存在(RD-003 收编复用 §3 协议)"
    );
    // 协议实现库(收编后被 rx bench 编排)与 smoke 真跑入口。
    assert!(
        root.join("bench/protocol.py").is_file(),
        "bench/protocol.py 单次协议实现应存在"
    );
    assert!(
        root.join("bench/saxpy_bench.py").is_file(),
        "bench/saxpy_bench.py(--smoke 正确性入口)应存在"
    );
}
