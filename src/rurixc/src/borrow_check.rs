//! NLL 借用检查(RXS-0057 ~ RXS-0061;M3_PLAN §3 任务 2,4xxx 借用主体)。
//!
//! 复用 [`crate::dataflow`] 骨架,按 **local 粒度**保守口径(RXS-0057:同 local
//! 即视为可能重叠;字段级不相交精度随 M3.3+ 只追加):
//! - **借用收集**:每个 `Rvalue::Ref(kind, borrowed)` 登记一笔 loan
//!   (borrowed local + 种类 + holder ref local + 创建点)。
//! - **NLL 活跃期(RXS-0059)**:loan 的活跃 = 其 holder 引用值仍活(后向
//!   liveness),非词法块边界;holder 被重定义即 kill(前向 reaching)。
//! - **报错 walk**(不动点后逐点重放):
//!   - RXS-0058 别名 XOR 可变 → `RX4004`(double_mut / shared_mut_conflict);
//!   - RXS-0060 借用期间所有者 move/写 → `RX4005`(move/assign_while_borrowed);
//!   - RXS-0061 悬垂引用(局部引用经返回槽逃逸)→ `RX4006`(dangling_reference)。
//!
//! 07 §4 先正确性后诊断:保守允许放大,误报登记为已知限制;措辞粗糙。
//! 检查时点 = MIR 构造后、codegen 前,move/init 之后(NLL 前置 pass);
//! 经 [`crate::query::QueryCtx::check_borrows`] query 接入管线(M3.3 WP3)。

use std::collections::{HashMap, HashSet};

use crate::dataflow::{Analysis, BitSet, Direction, Location, iterate_to_fixpoint};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::mir::{
    Body, BorrowKind, LocalIdx, Operand, Place, ProjElem, Rvalue, Statement, StatementKind,
    Terminator, TerminatorKind,
};
use crate::span::Span;

pub const E_BORROW_CONFLICT: ErrorCode = ErrorCode(4004); // RX4004
pub const E_BORROW_WHILE_BORROWED: ErrorCode = ErrorCode(4005); // RX4005
pub const E_DANGLING_REFERENCE: ErrorCode = ErrorCode(4006); // RX4006

/// 单笔借用(loan)。
#[derive(Clone, Copy, Debug)]
struct Loan {
    /// 被借路径的 local(整 local 重叠口径,RXS-0057)。
    borrowed: LocalIdx,
    kind: BorrowKind,
    /// 承载该借用的引用值所在 local(NLL 活跃期取其 liveness)。
    holder: LocalIdx,
    created: Location,
}

/// 对单个 MIR body 跑借用检查(诊断经 `diag`)。
pub fn check_body(diag: &DiagCtxt, body: &Body) {
    let loans = collect_loans(body);
    if loans.is_empty() {
        return;
    }

    // 创建点 → loan id;holder 列表(killset 用)。
    let mut loc_loan: HashMap<(usize, usize), usize> = HashMap::new();
    let holders: Vec<LocalIdx> = loans.iter().map(|l| l.holder).collect();
    for (id, l) in loans.iter().enumerate() {
        loc_loan.insert((l.created.block, l.created.stmt), id);
    }

    // 前向 reaching:逐点到达的活跃 loan 集(尚未被 holder 重定义 kill)。
    let reach = InScope {
        n_loans: loans.len(),
        holders: holders.clone(),
        loc_loan: loc_loan.clone(),
    };
    let rf = iterate_to_fixpoint(body, &reach);
    let mut reach_in: HashMap<(usize, usize), BitSet> = HashMap::new();
    for b in 0..body.blocks.len() {
        rf.visit_block(body, &reach, b, |state, loc| {
            reach_in.insert((loc.block, loc.stmt), state.clone());
        });
    }

    // 后向 liveness:逐点 holder 的 live-out(NLL 活跃期判据)。
    let live = Liveness {
        n: body.locals.len(),
    };
    let lr = iterate_to_fixpoint(body, &live);
    let mut live_out: HashMap<(usize, usize), BitSet> = HashMap::new();
    for b in 0..body.blocks.len() {
        lr.visit_block(body, &live, b, |state, loc| {
            live_out.insert((loc.block, loc.stmt), state.clone());
        });
    }

    // holder 拷贝闭包:引用值经 `dst = use(src)`(Copy/Move)流向的全部 local。
    // loan 活跃期 = 其闭包内任一 local 仍活(NLL,RXS-0059;承接 `&T` 可复制)。
    let holder_sets = holder_closures(body, &loans);

    // 悬垂:经返回槽 `_0` 逃逸的、指向本函数真局部的借用(RXS-0061)。
    let escaped = escapes_to_return(body);

    let mut rep = Reporter {
        diag,
        body,
        loans: &loans,
        holder_sets: &holder_sets,
        reported: HashSet::new(),
    };

    // RXS-0061:逐 loan 一次性裁决(创建点报告)。
    for l in loans.iter() {
        if is_local_referent(body, l.borrowed) && escaped.contains(l.holder.0 as usize) {
            let span = body.blocks[l.created.block].stmts[l.created.stmt].span;
            rep.report(
                E_DANGLING_REFERENCE,
                "borrowck.dangling_reference",
                l.borrowed,
                span,
                "reference to local outlives its referent",
            );
        }
    }

    // RXS-0058 / RXS-0060:逐点 walk。
    for b in 0..body.blocks.len() {
        let bb = &body.blocks[b];
        for i in 0..=bb.stmts.len() {
            let key = (b, i);
            let Some(r) = reach_in.get(&key) else {
                continue;
            };
            let Some(lo) = live_out.get(&key) else {
                continue;
            };
            let active = |id: usize| -> bool {
                r.contains(id) && holder_sets[id].iter().any(|h| lo.contains(h.0 as usize))
            };
            if i < bb.stmts.len() {
                rep.check_stmt(&bb.stmts[i], key, &active, lo);
            } else {
                rep.check_term(&bb.terminator, &active);
            }
        }
    }
}

/// 别名 XOR 可变:两笔可能重叠借用是否冲突(RXS-0058)。
fn conflicts(a: BorrowKind, b: BorrowKind) -> bool {
    matches!(a, BorrowKind::Mut) || matches!(b, BorrowKind::Mut)
}

fn collect_loans(body: &Body) -> Vec<Loan> {
    let mut loans = Vec::new();
    for (b, bb) in body.blocks.iter().enumerate() {
        for (i, stmt) in bb.stmts.iter().enumerate() {
            let StatementKind::Assign(dest, rv) = &stmt.kind;
            if let Rvalue::Ref(kind, borrowed) = rv {
                loans.push(Loan {
                    borrowed: borrowed.local,
                    kind: *kind,
                    holder: dest.local,
                    created: Location { block: b, stmt: i },
                });
            }
        }
    }
    loans
}

/// 真局部(非返回槽/非参数):其指代物不活过本次调用(RXS-0061)。
fn is_local_referent(body: &Body, l: LocalIdx) -> bool {
    (l.0 as usize) > body.arg_count
}

/// 每笔 loan 的 holder 拷贝闭包:引用值经 `dst = use(src)`(Copy/Move,整体)
/// 流向的 local 集(含创建 holder)。NLL 活跃期取闭包内任一 local 的 liveness。
fn holder_closures(body: &Body, loans: &[Loan]) -> Vec<Vec<LocalIdx>> {
    let mut sets: Vec<HashSet<u32>> = loans
        .iter()
        .map(|l| {
            let mut s = HashSet::new();
            s.insert(l.holder.0);
            s
        })
        .collect();
    loop {
        let mut changed = false;
        for bb in &body.blocks {
            for stmt in &bb.stmts {
                let StatementKind::Assign(dest, rv) = &stmt.kind;
                if !dest.proj.is_empty() {
                    continue;
                }
                if let Rvalue::Use(op) = rv
                    && let Some(p) = op.place()
                    && p.proj.is_empty()
                {
                    for s in sets.iter_mut() {
                        if s.contains(&p.local.0) && s.insert(dest.local.0) {
                            changed = true;
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    sets.into_iter()
        .map(|s| s.into_iter().map(LocalIdx).collect())
        .collect()
}

/// 经返回槽 `_0` 逃逸的 local 集(直接落 `_0` 或经 `_0 = use(p)` 一/多跳传播)。
fn escapes_to_return(body: &Body) -> BitSet {
    let mut set = BitSet::new(body.locals.len());
    set.insert(0);
    // 不动点:把"以 use 形式整体流入逃逸 local"的源 local 纳入。
    loop {
        let mut changed = false;
        for bb in &body.blocks {
            for stmt in &bb.stmts {
                let StatementKind::Assign(dest, rv) = &stmt.kind;
                if !dest.proj.is_empty() || !set.contains(dest.local.0 as usize) {
                    continue;
                }
                if let Rvalue::Use(op) = rv
                    && let Some(p) = op.place()
                    && p.proj.is_empty()
                    && !set.contains(p.local.0 as usize)
                {
                    set.insert(p.local.0 as usize);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    set
}

// ---------------------------------------------------------------------------
// 数据流:前向 reaching loans
// ---------------------------------------------------------------------------

struct InScope {
    n_loans: usize,
    holders: Vec<LocalIdx>,
    loc_loan: HashMap<(usize, usize), usize>,
}

impl InScope {
    fn kill_holder(&self, state: &mut BitSet, def: LocalIdx) {
        for (id, h) in self.holders.iter().enumerate() {
            if *h == def {
                state.remove(id);
            }
        }
    }
}

impl Analysis for InScope {
    fn bits(&self, _body: &Body) -> usize {
        self.n_loans
    }

    fn boundary(&self, _body: &Body, _state: &mut BitSet) {}

    fn stmt_effect(&self, state: &mut BitSet, stmt: &Statement, loc: Location) {
        let StatementKind::Assign(dest, _) = &stmt.kind;
        if dest.proj.is_empty() {
            self.kill_holder(state, dest.local);
        }
        if let Some(id) = self.loc_loan.get(&(loc.block, loc.stmt)) {
            state.insert(*id);
        }
    }

    fn term_effect(&self, state: &mut BitSet, term: &Terminator, _loc: Location) {
        if let TerminatorKind::Call { dest, .. } = &term.kind
            && dest.proj.is_empty()
        {
            self.kill_holder(state, dest.local);
        }
    }
}

// ---------------------------------------------------------------------------
// 数据流:后向 liveness(全 local)
// ---------------------------------------------------------------------------

struct Liveness {
    n: usize,
}

impl Analysis for Liveness {
    fn bits(&self, _body: &Body) -> usize {
        self.n
    }

    fn direction(&self) -> Direction {
        Direction::Backward
    }

    fn boundary(&self, _body: &Body, _state: &mut BitSet) {}

    fn stmt_effect(&self, state: &mut BitSet, stmt: &Statement, _loc: Location) {
        let StatementKind::Assign(dest, rv) = &stmt.kind;
        // live_in = uses ∪ (live_out − def);先 kill def 再 gen uses。
        if dest.proj.is_empty() {
            state.remove(dest.local.0 as usize);
        } else {
            state.insert(dest.local.0 as usize); // 字段/解引用写 = 读基址
        }
        for r in rvalue_read_locals(rv) {
            state.insert(r.0 as usize);
        }
    }

    fn term_effect(&self, state: &mut BitSet, term: &Terminator, _loc: Location) {
        match &term.kind {
            TerminatorKind::Return => {
                state.insert(0); // 返回槽 _0 在出口被读
            }
            TerminatorKind::Call { args, dest, .. } => {
                if dest.proj.is_empty() {
                    state.remove(dest.local.0 as usize);
                } else {
                    state.insert(dest.local.0 as usize);
                }
                for a in args {
                    if let Some(p) = a.place() {
                        state.insert(p.local.0 as usize);
                    }
                }
            }
            TerminatorKind::SwitchBool { discr, .. } => {
                if let Some(p) = discr.place() {
                    state.insert(p.local.0 as usize);
                }
            }
            // Drop 编译器插入,不视为延长 holder 活跃的源(避免词法放大)
            TerminatorKind::Drop { .. } | TerminatorKind::Goto(_) | TerminatorKind::Unreachable => {
            }
        }
    }
}

/// rvalue 中被读取的 local(取址 Ref 的 borrowed 计为读)。
fn rvalue_read_locals(rv: &Rvalue) -> Vec<LocalIdx> {
    let mut v = Vec::new();
    let mut push = |op: &Operand| {
        if let Some(p) = op.place() {
            v.push(p.local);
        }
    };
    match rv {
        Rvalue::Use(o) | Rvalue::UnaryOp(_, o) | Rvalue::Cast(o, _) => push(o),
        Rvalue::BinaryOp(_, a, b) => {
            push(a);
            push(b);
        }
        Rvalue::Aggregate(_, ops) | Rvalue::VariantAggregate { ops, .. } => {
            for o in ops {
                push(o);
            }
        }
        Rvalue::Ref(_, p) | Rvalue::Discriminant(p) => v.push(p.local),
        // 采样方法族(RXS-0175/0223):coord + extra 读 + texture/sampler 句柄 local 均计为活跃读。
        Rvalue::ResourceSample {
            coord,
            texture_local,
            sampler_local,
            extra,
            ..
        } => {
            push(coord);
            for op in extra {
                push(op);
            }
            v.push(*texture_local);
            if let Some(s) = sampler_local {
                v.push(*s);
            }
        }
    }
    v
}

fn place_has_deref(p: &Place) -> bool {
    p.proj.iter().any(|e| matches!(e, ProjElem::Deref))
}

// ---------------------------------------------------------------------------
// 报告器
// ---------------------------------------------------------------------------

struct Reporter<'a> {
    diag: &'a DiagCtxt,
    body: &'a Body,
    loans: &'a [Loan],
    holder_sets: &'a [Vec<LocalIdx>],
    /// (borrowed local, 错误码) 去重。
    reported: HashSet<(u32, u16)>,
}

impl Reporter<'_> {
    fn place_desc(&self, l: LocalIdx) -> String {
        match &self.body.local(l).name {
            Some(n) => format!("`{n}`"),
            None => "(temporary)".to_owned(),
        }
    }

    fn report(&mut self, code: ErrorCode, key: &str, l: LocalIdx, span: Span, label: &str) {
        if !self.reported.insert((l.0, code.0)) {
            return;
        }
        self.diag
            .struct_error(code, key)
            .arg("place", self.place_desc(l))
            .span_label(span, label.to_owned())
            .emit();
    }

    fn report_kv(
        &mut self,
        code: ErrorCode,
        key: &str,
        kv: (&str, String),
        l: LocalIdx,
        span: Span,
        label: &str,
    ) {
        if !self.reported.insert((l.0, code.0)) {
            return;
        }
        self.diag
            .struct_error(code, key)
            .arg("place", self.place_desc(l))
            .arg(kv.0, kv.1)
            .span_label(span, label.to_owned())
            .emit();
    }

    /// 当前点已有的活跃借用中,与 `borrowed` 可能重叠者的 (id)。
    fn overlapping(&self, borrowed: LocalIdx, active: &dyn Fn(usize) -> bool) -> Vec<usize> {
        (0..self.loans.len())
            .filter(|&id| active(id) && self.loans[id].borrowed == borrowed)
            .collect()
    }

    fn check_stmt(
        &mut self,
        stmt: &Statement,
        key: (usize, usize),
        active: &dyn Fn(usize) -> bool,
        live_out: &BitSet,
    ) {
        let StatementKind::Assign(dest, rv) = &stmt.kind;

        // RXS-0058:新借用与既有活跃借用的别名 XOR 可变冲突。
        if let Rvalue::Ref(kind, borrowed) = rv {
            let created = Location {
                block: key.0,
                stmt: key.1,
            };
            let new_id = self.loans.iter().position(|l| l.created == created);
            // 新借用自身须活(holder 闭包 live-out),否则即生即死无冲突。
            if let Some(nid) = new_id
                && self.holder_sets[nid]
                    .iter()
                    .any(|h| live_out.contains(h.0 as usize))
            {
                for other in self.overlapping(borrowed.local, active) {
                    if conflicts(self.loans[other].kind, *kind) {
                        let label = match (self.loans[other].kind, kind) {
                            (BorrowKind::Mut, BorrowKind::Mut) => "two `&mut` borrows overlap",
                            _ => "`&` and `&mut` borrows overlap",
                        };
                        self.report_kv(
                            E_BORROW_CONFLICT,
                            "borrowck.borrow_conflict",
                            ("kind", label.to_owned()),
                            borrowed.local,
                            stmt.span,
                            label,
                        );
                    }
                }
            }
        }

        // RXS-0060:借用活跃期间写入被借所有者(整体或字段写)。
        if let Some(others) = self.owner_write_target(dest)
            && !self.overlapping(others, active).is_empty()
        {
            self.report_kv(
                E_BORROW_WHILE_BORROWED,
                "borrowck.borrow_while_borrowed",
                ("action", "assign to".to_owned()),
                others,
                stmt.span,
                "assigned here while borrowed",
            );
        }

        // RXS-0060:借用活跃期间 move 被借所有者。
        self.check_owner_moves(rv, key, active, stmt.span);
    }

    /// 被整体/字段写入的所有者 local(创建借用的 holder 写入除外)。
    fn owner_write_target(&self, dest: &Place) -> Option<LocalIdx> {
        // 解引用写不视为所有者写(经引用进行);整体写或字段写 = 写所有者。
        if place_has_deref(dest) {
            None
        } else {
            Some(dest.local)
        }
    }

    fn check_owner_moves(
        &mut self,
        rv: &Rvalue,
        _key: (usize, usize),
        active: &dyn Fn(usize) -> bool,
        span: Span,
    ) {
        let mut check = |op: &Operand| {
            if let Operand::Move(p) = op
                && !place_has_deref(p)
                && !self.overlapping(p.local, active).is_empty()
            {
                self.report_kv(
                    E_BORROW_WHILE_BORROWED,
                    "borrowck.borrow_while_borrowed",
                    ("action", "move out of".to_owned()),
                    p.local,
                    span,
                    "moved here while borrowed",
                );
            }
        };
        for op in operands_of_rvalue(rv) {
            check(op);
        }
    }

    fn check_term(&mut self, term: &Terminator, active: &dyn Fn(usize) -> bool) {
        if let TerminatorKind::Call { args, .. } = &term.kind {
            for op in args {
                if let Operand::Move(p) = op
                    && !place_has_deref(p)
                    && !self.overlapping(p.local, active).is_empty()
                {
                    self.report_kv(
                        E_BORROW_WHILE_BORROWED,
                        "borrowck.borrow_while_borrowed",
                        ("action", "move out of".to_owned()),
                        p.local,
                        term.span,
                        "moved here while borrowed",
                    );
                }
            }
        }
    }
}

fn operands_of_rvalue(rv: &Rvalue) -> Vec<&Operand> {
    match rv {
        Rvalue::Use(o) | Rvalue::UnaryOp(_, o) | Rvalue::Cast(o, _) => vec![o],
        Rvalue::BinaryOp(_, a, b) => vec![a, b],
        Rvalue::Aggregate(_, ops) | Rvalue::VariantAggregate { ops, .. } => ops.iter().collect(),
        Rvalue::Ref(..) | Rvalue::Discriminant(_) => Vec::new(),
        // 采样方法族(RXS-0175/0223):coord + extra 为读 operand;texture/sampler 句柄非 operand。
        Rvalue::ResourceSample { coord, extra, .. } => {
            let mut ops = vec![coord];
            ops.extend(extra.iter());
            ops
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// 经全 query 通道建 MIR(前置无诊断)后跑 `check_borrows` query,回收错误码。
    fn check(src: &str) -> Vec<u16> {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "前置诊断: {:?}", diag.emitted());
        cx.check_borrows();
        let mut codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        codes.sort_unstable();
        codes
    }

    // -- RXS-0058 别名 XOR 可变(RX4004) --------------------------------------

    //@ spec: RXS-0058
    #[test]
    fn double_mut_borrow_detected() {
        let codes = check(
            "fn use2(a: &mut i32, b: &mut i32) -> i32 { *a + *b }\nfn main() {\n    let mut x = 1;\n    let r1 = &mut x;\n    let r2 = &mut x;\n    let _z = use2(r1, r2);\n}",
        );
        assert_eq!(codes, vec![4004]);
    }

    //@ spec: RXS-0058
    #[test]
    fn shared_mut_conflict_detected() {
        let codes = check(
            "fn use2(a: &i32, b: &mut i32) -> i32 { *a + *b }\nfn main() {\n    let mut x = 1;\n    let r1 = &x;\n    let r2 = &mut x;\n    let _z = use2(r1, r2);\n}",
        );
        assert_eq!(codes, vec![4004]);
    }

    //@ spec: RXS-0058
    #[test]
    fn two_shared_borrows_are_clean() {
        let codes = check(
            "fn use2(a: &i32, b: &i32) -> i32 { *a + *b }\nfn main() {\n    let x = 1;\n    let r1 = &x;\n    let r2 = &x;\n    let _z = use2(r1, r2);\n}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0059
    #[test]
    fn nll_released_mut_reborrow_is_clean() {
        // r1 最后一次使用在 r2 创建之前 → NLL 已释放,不冲突
        let codes = check(
            "fn use1(a: &mut i32) -> i32 { *a }\nfn main() {\n    let mut x = 1;\n    let r1 = &mut x;\n    let _a = use1(r1);\n    let r2 = &mut x;\n    let _b = use1(r2);\n}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    // -- RXS-0060 借用期间所有者操作(RX4005) --------------------------------

    //@ spec: RXS-0060
    #[test]
    fn move_while_borrowed_detected() {
        let codes = check(
            "struct T { id: i32 }\nfn eat(t: T) -> i32 { t.id }\nfn peek(r: &T) -> i32 { r.id }\nfn main() {\n    let t = T { id: 1 };\n    let r = &t;\n    let _a = eat(t);\n    let _b = peek(r);\n}",
        );
        assert_eq!(codes, vec![4005]);
    }

    //@ spec: RXS-0060
    #[test]
    fn assign_while_borrowed_detected() {
        let codes = check(
            "fn peek(r: &i32) -> i32 { *r }\nfn main() {\n    let mut x = 1;\n    let r = &x;\n    x = 5;\n    let _b = peek(r);\n}",
        );
        assert_eq!(codes, vec![4005]);
    }

    //@ spec: RXS-0060
    #[test]
    fn owner_op_after_borrow_released_is_clean() {
        let codes = check(
            "fn peek(r: &i32) -> i32 { *r }\nfn main() {\n    let mut x = 1;\n    let _a = peek(&x);\n    x = 5;\n    let _b = x;\n}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    // -- RXS-0061 悬垂引用(RX4006) ------------------------------------------

    //@ spec: RXS-0061
    #[test]
    fn dangling_local_reference_detected() {
        let codes = check(
            "fn dangle() -> &i32 {\n    let x = 1;\n    &x\n}\nfn main() {\n    let _r = dangle();\n}",
        );
        assert_eq!(codes, vec![4006]);
    }

    //@ spec: RXS-0061
    #[test]
    fn reference_to_param_is_not_dangling() {
        // 返回参数引用:指代物活过本次调用,非悬垂
        let codes = check(
            "fn id(r: &i32) -> &i32 { r }\nfn main() {\n    let x = 1;\n    let _r = id(&x);\n}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    // -- query 接入:memo 命中/纯函数纪律 ------------------------------------

    //@ spec: RXS-0058
    #[test]
    fn check_borrows_query_memoized() {
        // 同 QueryCtx 连调两次 check_borrows:第二次走 memo,不重复诊断、不重算。
        let diag = DiagCtxt::new();
        let src = "fn use2(a: &mut i32, b: &mut i32) -> i32 { *a + *b }\nfn main() {\n    let mut x = 1;\n    let r1 = &mut x;\n    let r2 = &mut x;\n    let _z = use2(r1, r2);\n}";
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "前置诊断: {:?}", diag.emitted());
        cx.check_borrows();
        let after_first = diag.emitted().len();
        assert_eq!(after_first, 1, "首次借用检查应恰报一笔 RX4004");
        let misses_after_first = cx.memo_misses();
        let hits_before = cx.memo_hits();
        cx.check_borrows();
        assert_eq!(diag.emitted().len(), after_first, "二次调用不得重复诊断");
        assert_eq!(cx.memo_misses(), misses_after_first, "二次调用零重算");
        assert!(cx.memo_hits() > hits_before, "二次调用应命中 memo");
    }

    // -- 中间产物:reaching in-scope borrows --------------------------------

    //@ spec: RXS-0059
    #[test]
    fn inscope_borrow_reaches_later_use() {
        let diag = DiagCtxt::new();
        let src = "fn peek(r: &i32) -> i32 { *r }\nfn main() {\n    let x = 1;\n    let r = &x;\n    let _b = peek(r);\n}";
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "{:?}", diag.emitted());
        let mir = cx.mir_crate();
        let main = mir.iter().find(|b| b.symbol == "main").expect("main body");
        let loans = collect_loans(main);
        assert_eq!(loans.len(), 1, "main 应恰有一笔借用 &x");

        let mut loc_loan = HashMap::new();
        for (id, l) in loans.iter().enumerate() {
            loc_loan.insert((l.created.block, l.created.stmt), id);
        }
        let reach = InScope {
            n_loans: loans.len(),
            holders: loans.iter().map(|l| l.holder).collect(),
            loc_loan,
        };
        let rf = iterate_to_fixpoint(main, &reach);
        // 借用在创建后,到达后续(终结子调用 peek 的)程序点仍 in-scope
        let mut reached_after_creation = false;
        for b in 0..main.blocks.len() {
            rf.visit_block(main, &reach, b, |state, loc| {
                let created = loans[0].created;
                let after = loc.block > created.block
                    || (loc.block == created.block && loc.stmt > created.stmt);
                if after && state.contains(0) {
                    reached_after_creation = true;
                }
            });
        }
        assert!(reached_after_creation, "借用应在创建后保持 in-scope");
    }
}
