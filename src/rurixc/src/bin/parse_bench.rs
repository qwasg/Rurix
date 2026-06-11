//! parser 吞吐基准探针(M1 契约 D-M1-6;协议侧驱动在 bench/parser_bench.py)。
//!
//! 用法:`parse_bench <corpus-file> <iters>`
//! 输出:首行 `{"items":N,"loc":L,"bytes":B}` 哨兵,随后每迭代一行
//! `{"iter":i,"ns":t,"loc":L}`。
//!
//! 计时仅覆盖 parse(token 流在计时区外预先 lex 并逐迭代 clone);
//! 基准语料必须 0 诊断(lex + parse + feature gate),否则退出码 2。

use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

use rurixc::diag::DiagCtxt;
use rurixc::feature_gate::check_feature_gates;
use rurixc::lexer::lex;
use rurixc::parser::parse;
use rurixc::span::{Edition, SourceId};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(iters)) = (args.get(1), args.get(2)) else {
        eprintln!("usage: parse_bench <corpus-file> <iters>");
        return ExitCode::from(1);
    };
    let Ok(iters) = iters.parse::<u32>() else {
        eprintln!("parse_bench: iters 必须是非负整数");
        return ExitCode::from(1);
    };
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("parse_bench: 读取 {path} 失败: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes = src.len();
    let loc = src.lines().count();

    // 正确性哨兵:语料必须干净(lex + parse + feature gate 各零诊断)
    let diag = DiagCtxt::new();
    let tokens = lex(&src, SourceId(0), Edition::Rx0, &diag);
    if !diag.emitted().is_empty() {
        eprintln!(
            "parse_bench: 语料含 {} 条词法诊断,拒绝采样",
            diag.emitted().len()
        );
        return ExitCode::from(2);
    }
    let ast = parse(&src, tokens.clone(), SourceId(0), Edition::Rx0, &diag);
    check_feature_gates(&ast, &diag);
    if !diag.emitted().is_empty() {
        eprintln!(
            "parse_bench: 语料含 {} 条语法/gate 诊断,拒绝采样",
            diag.emitted().len()
        );
        return ExitCode::from(2);
    }
    let items = ast.items.len();
    if items == 0 {
        eprintln!("parse_bench: 语料未产出 item,拒绝采样");
        return ExitCode::from(2);
    }
    println!("{{\"items\":{items},\"loc\":{loc},\"bytes\":{bytes}}}");
    drop(ast);

    for i in 0..iters {
        // token clone 在计时区外:计时仅覆盖 parse 本体
        let toks = tokens.clone();
        let t0 = Instant::now();
        let ast = parse(&src, toks, SourceId(0), Edition::Rx0, &diag);
        let ns = t0.elapsed().as_nanos();
        black_box(&ast);
        // 基准语料零诊断已由哨兵保证;防御性断言防止采样期意外报错累积
        assert!(diag.emitted().is_empty(), "采样期产生诊断,数字无效");
        println!("{{\"iter\":{i},\"ns\":{ns},\"loc\":{loc}}}");
    }
    ExitCode::SUCCESS
}
