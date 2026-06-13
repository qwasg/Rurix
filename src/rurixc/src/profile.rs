//! self-profile:阶段计时 + 计数器(D-M2-6,契约 G-M2-4;07 §6 `-Z self-profile` 式)。
//!
//! 输出形态 = JSON 行(机器可解析,G-M2-4):每阶段一行
//! `{"stage":"parse","wall_ms":1.234,"counters":{"tokens":42,"items":1}}`。
//! 阶段集 = parse(含 lex)/resolve/typeck/mir/codegen/link(M2_PLAN §4),
//! 末行 `total` 携带 query memo 汇总(数据源 [`crate::query::QueryCtx`] 的
//! memo_hits/memo_misses,M2.2 布点的消费端)。
//!
//! 计数器纪律:D-235"合入后 2 个里程碑内非零真实证据"——CI 对 hello-world
//! 编译断言全部阶段计数器非零(`ci/hello_smoke.py self-profile`,M2 CI_GATES §2)。

use std::cell::RefCell;
use std::time::Instant;

/// 单阶段记录:墙钟毫秒 + 计数器集(名字为静态标识符,无需 JSON 转义)。
pub struct StageRecord {
    pub stage: &'static str,
    pub wall_ms: f64,
    pub counters: Vec<(&'static str, u64)>,
}

/// 阶段计时收集器。`&self` 接口(内部 `RefCell`),与驱动的既有借用结构并存;
/// 不进 query 层(query 维持纯函数纪律,D-203),只在驱动外层包裹阶段。
#[derive(Default)]
pub struct Profiler {
    stages: RefCell<Vec<StageRecord>>,
}

impl Profiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一个阶段:`started` 为阶段起点墙钟,`counters` 为该阶段计数器。
    pub fn record(&self, stage: &'static str, started: Instant, counters: &[(&'static str, u64)]) {
        self.record_ms(stage, started.elapsed().as_secs_f64() * 1e3, counters);
    }

    /// 以累计毫秒记录阶段(非连续区段的聚合计时,如 TBIR 逐 body 即建即用)。
    pub fn record_ms(&self, stage: &'static str, wall_ms: f64, counters: &[(&'static str, u64)]) {
        self.stages.borrow_mut().push(StageRecord {
            stage,
            wall_ms,
            counters: counters.to_vec(),
        });
    }

    /// 已记录阶段数(单测与驱动自检用)。
    pub fn len(&self) -> usize {
        self.stages.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.stages.borrow().is_empty()
    }

    /// JSON 行序列化(手写:rurixc 零外部 Rust 依赖纪律,M1.1)。
    pub fn to_json_lines(&self) -> String {
        let mut out = String::new();
        for r in self.stages.borrow().iter() {
            out.push_str(&format!(
                "{{\"stage\":\"{}\",\"wall_ms\":{:.3},\"counters\":{{",
                r.stage, r.wall_ms
            ));
            for (i, (k, v)) in r.counters.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&format!("\"{k}\":{v}"));
            }
            out.push_str("}}\n");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    #[test]
    fn json_lines_shape() {
        let p = Profiler::new();
        let t = Instant::now();
        p.record("parse", t, &[("tokens", 7), ("items", 1)]);
        p.record("resolve", t, &[("defs", 2)]);
        let text = p.to_json_lines();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("{\"stage\":\"parse\",\"wall_ms\":"));
        assert!(lines[0].ends_with("\"counters\":{\"tokens\":7,\"items\":1}}"));
        assert!(lines[1].contains("\"counters\":{\"defs\":2}}"));
    }

    /// 驱动同款分阶段布点跑到 MIR:全部计数器非零(G-M2-4 的进程内子集;
    /// codegen/link 阶段经 ci/hello_smoke.py self-profile 在 CI 全管线断言)。
    #[test]
    fn stage_counters_nonzero_through_mir() {
        let diag = DiagCtxt::new();
        let src = "fn main() {\n    let greeting = \"hello, rurix\";\n    println(greeting);\n}\n";
        let prof = Profiler::new();

        let t = Instant::now();
        let tokens = lex(src, SourceId(0), Edition::Rx0, &diag);
        let n_tokens = tokens.len() as u64;
        let ast = parse(src, tokens, SourceId(0), Edition::Rx0, &diag);
        prof.record(
            "parse",
            t,
            &[("tokens", n_tokens), ("items", ast.items.len() as u64)],
        );
        let cx = QueryCtx::from_ast(ast, src, SourceId(0), &diag);

        let t = Instant::now();
        let res = cx.resolutions();
        prof.record("resolve", t, &[("defs", res.defs.len() as u64)]);

        let t = Instant::now();
        cx.check_crate();
        prof.record(
            "typeck",
            t,
            &[("bodies_checked", cx.hir_crate().bodies.len() as u64)],
        );

        let t = Instant::now();
        let mir = cx.mir_crate();
        prof.record("mir", t, &[("mir_bodies", mir.len() as u64)]);
        assert!(
            diag.emitted().is_empty(),
            "前置阶段诊断: {:?}",
            diag.emitted()
        );

        let text = prof.to_json_lines();
        assert_eq!(text.lines().count(), 4);
        for r in prof.stages.borrow().iter() {
            for (k, v) in &r.counters {
                assert!(*v > 0, "阶段 {} 计数器 {k} 为零(G-M2-4 非零判据)", r.stage);
            }
        }
    }
}
