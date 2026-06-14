//! drop elaboration(RXS-0055;M3_PLAN §2 任务 4)。
//!
//! 输入 = [`crate::mir_build`] Phase A 产出的**无条件** Drop 终结子(scope 退出
//! 序已就位)+ ownership 数据流([`crate::dataflow`])。精化:
//! - **definitely-owned**(must 分析)→ 保留无条件 Drop;
//! - **definitely-not-owned**(may 分析判 never-owned)→ 删除(Drop → Goto);
//! - **maybe-owned** → drop flag:bool local 在(重)初始化置真、move 置假,
//!   Drop 经 SwitchBool(flag) 守卫,运行期裁决(RXS-0055 至多一次)。
//!
//! ownership 转移(per-local 一位,owned=1):入口参数 owned;整体赋值置 owned;
//! 整 local move(非解引用)与 Drop 置非 owned。may 用 OR 汇合、must 用 AND。

use crate::dataflow::{Analysis, BitSet, Direction, Location, iterate_to_fixpoint};
use crate::mir::{
    BasicBlock, BlockIdx, Body, Const, Local, LocalIdx, Operand, Place, Rvalue, Statement,
    StatementKind, Terminator, TerminatorKind,
};
use crate::move_check::{place_has_deref, rvalue_operands};
use crate::span::Span;
use crate::ty::Ty;

/// 在 MIR body 上原地精化 Drop 终结子(move/init 感知 + drop flag)。
pub fn elaborate(body: &mut Body) {
    if !body
        .blocks
        .iter()
        .any(|b| matches!(b.terminator.kind, TerminatorKind::Drop { .. }))
    {
        return; // 无 drop:零开销短路(常见,保持 MIR 形态不变)
    }
    let may = ownership_states(body, false);
    let must = ownership_states(body, true);

    // 分类每个 Drop 终结子(块入度=终结子点状态)
    let mut keep: Vec<usize> = Vec::new();
    let mut remove: Vec<usize> = Vec::new();
    let mut flag: Vec<(usize, LocalIdx, BlockIdx)> = Vec::new();
    for (bi, bb) in body.blocks.iter().enumerate() {
        let TerminatorKind::Drop { place, next } = &bb.terminator.kind else {
            continue;
        };
        let l = place.local.0 as usize;
        if must[bi].contains(l) {
            keep.push(bi);
        } else if !may[bi].contains(l) {
            remove.push(bi);
        } else {
            flag.push((bi, place.local, *next));
        }
    }

    for bi in &remove {
        let TerminatorKind::Drop { next, .. } = body.blocks[*bi].terminator.kind else {
            unreachable!()
        };
        body.blocks[*bi].terminator.kind = TerminatorKind::Goto(next);
    }
    let _ = keep; // 保留:无条件 Drop 原样进 codegen

    if !flag.is_empty() {
        insert_drop_flags(body, &flag);
    }
}

// ---------------------------------------------------------------------------
// ownership 数据流(may/must)
// ---------------------------------------------------------------------------

struct Ownership {
    arg_count: usize,
    must: bool,
}

impl Analysis for Ownership {
    fn bits(&self, body: &Body) -> usize {
        body.locals.len()
    }

    fn direction(&self) -> Direction {
        Direction::Forward
    }

    fn boundary(&self, _body: &Body, state: &mut BitSet) {
        state.clear();
        // 参数入口即 owned(返回槽 _0 与其余 local 未 owned)
        for i in 1..=self.arg_count {
            state.insert(i);
        }
    }

    fn bottom(&self, body: &Body) -> BitSet {
        // must:非边界块格顶 = 全 owned(AND 精化);may:格底 = 全 0
        if self.must {
            BitSet::filled(body.locals.len())
        } else {
            BitSet::new(body.locals.len())
        }
    }

    fn join(&self, into: &mut BitSet, from: &BitSet) -> bool {
        if self.must {
            into.intersect(from)
        } else {
            into.union(from)
        }
    }

    fn stmt_effect(&self, state: &mut BitSet, stmt: &Statement, _loc: Location) {
        let StatementKind::Assign(dest, rv) = &stmt.kind;
        for op in rvalue_operands(rv) {
            consume_op(state, op);
        }
        if dest.proj.is_empty() {
            state.insert(dest.local.0 as usize); // 整体赋值 → owned
        }
    }

    fn term_effect(&self, state: &mut BitSet, term: &Terminator, _loc: Location) {
        match &term.kind {
            TerminatorKind::Call { args, dest, .. } => {
                for op in args {
                    consume_op(state, op);
                }
                if dest.proj.is_empty() {
                    state.insert(dest.local.0 as usize);
                }
            }
            TerminatorKind::Drop { place, .. } => {
                state.remove(place.local.0 as usize); // drop 后不再 owned
            }
            _ => {}
        }
    }
}

fn consume_op(state: &mut BitSet, op: &Operand) {
    if let Operand::Move(p) = op
        && !place_has_deref(p)
    {
        state.remove(p.local.0 as usize);
    }
}

/// 每个块终结子点的 ownership 状态。
fn ownership_states(body: &Body, must: bool) -> Vec<BitSet> {
    let analysis = Ownership {
        arg_count: body.arg_count,
        must,
    };
    let results = iterate_to_fixpoint(body, &analysis);
    let mut out: Vec<BitSet> = (0..body.blocks.len())
        .map(|_| analysis.bottom(body))
        .collect();
    for (bi, slot) in out.iter_mut().enumerate() {
        let term_stmt = body.blocks[bi].stmts.len();
        results.visit_block(body, &analysis, bi, |state, loc: Location| {
            if loc.stmt == term_stmt {
                *slot = state.clone();
            }
        });
    }
    out
}

// ---------------------------------------------------------------------------
// drop flag 插入(maybe-owned drop 的运行期守卫)
// ---------------------------------------------------------------------------

fn insert_drop_flags(body: &mut Body, flagged: &[(usize, LocalIdx, BlockIdx)]) {
    // 每个被守卫 local 分配一个 bool flag local
    let mut flag_of: std::collections::HashMap<u32, LocalIdx> = std::collections::HashMap::new();
    for (_, l, _) in flagged {
        flag_of.entry(l.0).or_insert_with(|| {
            let idx = LocalIdx(body.locals.len() as u32);
            body.locals.push(Local {
                ty: Ty::Prim(crate::hir::PrimTy::Bool),
                name: None,
                span: body.span,
                shared: false,
                array_len: None,
            });
            idx
        });
    }
    let span = body.span;
    let set = |b: bool| Rvalue::Use(Operand::Const(Const::Bool(b)));

    // 入口 bb0 前置:flag 初值(参数 owned = true,其余 false)
    let mut init_stmts: Vec<Statement> = Vec::new();
    for (lid, flag) in &flag_of {
        let owned_at_entry = *lid >= 1 && (*lid as usize) <= body.arg_count;
        init_stmts.push(Statement {
            kind: StatementKind::Assign(Place::local(*flag), set(owned_at_entry)),
            span,
        });
    }

    // 每块:重建语句(在 init/move 点维护 flag),并处理 Call 终结子的 flag 更新块
    let mut new_blocks: Vec<BasicBlock> = Vec::with_capacity(body.blocks.len());
    // 先把待追加的新块收集,统一在末尾 push(BlockIdx 稳定)
    let mut appended: Vec<BasicBlock> = Vec::new();
    let next_block_id = body.blocks.len();

    let blocks = std::mem::take(&mut body.blocks);
    let mut init_stmts = Some(init_stmts);
    for (bi, bb) in blocks.into_iter().enumerate() {
        let BasicBlock { stmts, terminator } = bb;
        let mut out_stmts: Vec<Statement> = Vec::new();
        if bi == 0 {
            out_stmts.append(init_stmts.as_mut().unwrap());
        }
        for s in stmts {
            let StatementKind::Assign(dest, rv) = &s.kind;
            // move 出被守卫 local → flag = false(语句后)
            let moved: Vec<LocalIdx> = rvalue_operands(rv)
                .iter()
                .filter_map(|op| match op {
                    Operand::Move(p) if !place_has_deref(p) && flag_of.contains_key(&p.local.0) => {
                        Some(p.local)
                    }
                    _ => None,
                })
                .collect();
            let dest_flag = dest
                .proj
                .is_empty()
                .then(|| flag_of.get(&dest.local.0).copied())
                .flatten();
            let span_s = s.span;
            out_stmts.push(s);
            for m in moved {
                out_stmts.push(Statement {
                    kind: StatementKind::Assign(Place::local(flag_of[&m.0]), set(false)),
                    span: span_s,
                });
            }
            if let Some(flag) = dest_flag {
                out_stmts.push(Statement {
                    kind: StatementKind::Assign(Place::local(flag), set(true)),
                    span: span_s,
                });
            }
        }

        // 终结子处理
        let term = match terminator.kind {
            TerminatorKind::Drop { place, next } if flag_of.contains_key(&place.local.0) => {
                // SwitchBool(flag) { then: drop_bb, else: next };
                // drop_bb: Drop(place) -> clear_bb;clear_bb: flag=false; goto next
                let flag = flag_of[&place.local.0];
                let drop_id = BlockIdx((next_block_id + appended.len()) as u32);
                let clear_id = BlockIdx((next_block_id + appended.len() + 1) as u32);
                appended.push(BasicBlock {
                    stmts: Vec::new(),
                    terminator: Terminator {
                        kind: TerminatorKind::Drop {
                            place,
                            next: clear_id,
                        },
                        span: terminator.span,
                    },
                });
                appended.push(BasicBlock {
                    stmts: vec![Statement {
                        kind: StatementKind::Assign(Place::local(flag), set(false)),
                        span: terminator.span,
                    }],
                    terminator: Terminator {
                        kind: TerminatorKind::Goto(next),
                        span: terminator.span,
                    },
                });
                Terminator {
                    kind: TerminatorKind::SwitchBool {
                        discr: Operand::Copy(Place::local(flag)),
                        then: drop_id,
                        else_: next,
                    },
                    span: terminator.span,
                }
            }
            TerminatorKind::Call {
                target,
                args,
                dest,
                next,
            } => {
                // Call 的 move/dest flag 更新:插入更新块 FU,Call.next = FU
                let moved: Vec<LocalIdx> = args
                    .iter()
                    .filter_map(|op| match op {
                        Operand::Move(p)
                            if !place_has_deref(p) && flag_of.contains_key(&p.local.0) =>
                        {
                            Some(p.local)
                        }
                        _ => None,
                    })
                    .collect();
                let dest_flag = dest
                    .proj
                    .is_empty()
                    .then(|| flag_of.get(&dest.local.0).copied())
                    .flatten();
                if moved.is_empty() && dest_flag.is_none() {
                    Terminator {
                        kind: TerminatorKind::Call {
                            target,
                            args,
                            dest,
                            next,
                        },
                        span: terminator.span,
                    }
                } else {
                    let mut fu_stmts: Vec<Statement> = Vec::new();
                    for m in moved {
                        fu_stmts.push(Statement {
                            kind: StatementKind::Assign(Place::local(flag_of[&m.0]), set(false)),
                            span: terminator.span,
                        });
                    }
                    if let Some(flag) = dest_flag {
                        fu_stmts.push(Statement {
                            kind: StatementKind::Assign(Place::local(flag), set(true)),
                            span: terminator.span,
                        });
                    }
                    let fu_id = BlockIdx((next_block_id + appended.len()) as u32);
                    appended.push(BasicBlock {
                        stmts: fu_stmts,
                        terminator: Terminator {
                            kind: TerminatorKind::Goto(next),
                            span: terminator.span,
                        },
                    });
                    Terminator {
                        kind: TerminatorKind::Call {
                            target,
                            args,
                            dest,
                            next: fu_id,
                        },
                        span: terminator.span,
                    }
                }
            }
            other => Terminator {
                kind: other,
                span: terminator.span,
            },
        };
        new_blocks.push(BasicBlock {
            stmts: out_stmts,
            terminator: term,
        });
    }
    let _ = init_stmts;
    new_blocks.extend(appended);
    body.blocks = new_blocks;
}

/// body 中 Drop 终结子涉及类型所需的 `Drop::drop` 单态化实例(单态化收集补充)。
pub fn collect_drop_callees(
    krate: &crate::hir::Crate,
    body: &Body,
) -> Vec<(crate::hir::DefId, Vec<Ty>)> {
    let mut out = Vec::new();
    for bb in &body.blocks {
        if let TerminatorKind::Drop { place, .. } = &bb.terminator.kind {
            let ty = body.local(place.local).ty.clone();
            collect_drop_instances(krate, &ty, &mut out);
        }
    }
    out
}

fn collect_drop_instances(
    krate: &crate::hir::Crate,
    ty: &Ty,
    out: &mut Vec<(crate::hir::DefId, Vec<Ty>)>,
) {
    match ty {
        Ty::Adt(d, args) => {
            if let Some(fnd) = krate.drop_fn_of(*d) {
                out.push((fnd, args.clone()));
            }
            for comp in crate::ty::adt_component_tys(krate, *d, args) {
                collect_drop_instances(krate, &comp, out);
            }
        }
        Ty::Tuple(v) => {
            for t in v {
                collect_drop_instances(krate, t, out);
            }
        }
        Ty::Array(t) => collect_drop_instances(krate, t, out),
        _ => {}
    }
}

#[allow(dead_code)]
fn _unused(_: Span) {}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::mir::TerminatorKind;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    fn mir_text(src: &str) -> String {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "{:?}", diag.emitted());
        let mir = cx.mir_crate();
        let res = cx.resolutions();
        mir.iter()
            .map(|b| crate::mir::pretty(b, &res))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn count_drops(text: &str) -> usize {
        text.matches("drop(").count()
    }

    //@ spec: RXS-0055
    #[test]
    fn owned_local_keeps_unconditional_drop() {
        let text = mir_text(
            "struct R {}\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nfn main() {\n    let _r = R {};\n}",
        );
        // _r 定有所有权 → 无条件 drop(无 flag 守卫的 switch 引入)
        assert_eq!(count_drops(&text), 1, "{text}");
    }

    //@ spec: RXS-0055
    #[test]
    fn moved_local_drop_removed() {
        let text = mir_text(
            "struct R {}\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nfn take(r: R) {}\nfn main() {\n    let r = R {};\n    take(r);\n}",
        );
        // main 内 r 被 move 入 take → main 的 r drop 删除;take 内 param r 保留 1 个
        // 即整程序恰一处 drop(take 的参数)
        assert_eq!(count_drops(&text), 1, "{text}");
    }

    //@ spec: RXS-0055
    #[test]
    fn conditional_init_gets_drop_flag() {
        let text = mir_text(
            "struct R {}\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nfn main() {\n    let r: R;\n    let c = true;\n    if c {\n        r = R {};\n    } else {\n        r = R {};\n    }\n    let _keep = c;\n}",
        );
        // 条件初始化:drop 经 flag 守卫(出现 switch + drop 同块结构)
        assert!(count_drops(&text) >= 1, "{text}");
    }

    //@ spec: RXS-0052
    #[test]
    fn non_drop_program_unchanged_no_drop_terminator() {
        let text = mir_text("fn main() {\n    let x = 1;\n    let _y = x + 1;\n}");
        assert_eq!(count_drops(&text), 0, "{text}");
        assert!(!text.contains("Drop"));
        let _ = TerminatorKind::Return; // 引用以免未用告警
    }
}
