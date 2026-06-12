//! lexer 吞吐基准探针(M1 契约 D-M1-6;协议侧驱动在 bench/lexer_bench.py)。
//!
//! 用法:`lex_bench <corpus-file> <iters>`
//! 输出:首行 `{"tokens":N,"bytes":B}` 哨兵,随后每迭代一行 `{"iter":i,"ns":t,"bytes":B}`。
//! 基准语料必须 0 词法诊断,否则退出码 2(测错误路径的数字无效)。

use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

use rurixc::diag::DiagCtxt;
use rurixc::lexer::lex;
use rurixc::span::{Edition, SourceId};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(iters)) = (args.get(1), args.get(2)) else {
        eprintln!("usage: lex_bench <corpus-file> <iters>");
        return ExitCode::from(1);
    };
    let Ok(iters) = iters.parse::<u32>() else {
        eprintln!("lex_bench: iters 必须是非负整数");
        return ExitCode::from(1);
    };
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("lex_bench: 读取 {path} 失败: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes = src.len();

    // 正确性哨兵:语料必须干净(单次 lex 校验)
    let diag = DiagCtxt::new();
    let tokens = lex(&src, SourceId(0), Edition::Rx0, &diag);
    if !diag.emitted().is_empty() {
        eprintln!(
            "lex_bench: 语料含 {} 条词法诊断,拒绝采样",
            diag.emitted().len()
        );
        return ExitCode::from(2);
    }
    println!("{{\"tokens\":{},\"bytes\":{bytes}}}", tokens.len());
    drop(tokens);

    for i in 0..iters {
        let t0 = Instant::now();
        let toks = lex(&src, SourceId(0), Edition::Rx0, &diag);
        let ns = t0.elapsed().as_nanos();
        black_box(&toks);
        println!("{{\"iter\":{i},\"ns\":{ns},\"bytes\":{bytes}}}");
    }
    ExitCode::SUCCESS
}
