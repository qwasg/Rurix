//! MIR 数据流框架(M3_PLAN §2 任务 2;07 §4)。
//!
//! 通用位向量 worklist 不动点骨架:前向/后向方向、gen/kill 由具体分析经
//! [`Analysis`] 转移函数提供、汇合默认按位或(may 分析;must 分析覆写
//! [`Analysis::join`] 与 [`Analysis::bottom`])。首个消费者 = move/init
//! 分析(RXS-0054);M3.3 borrowck 的逐点 in-scope borrows 与 M4 device
//! 扩展 pass 复用本骨架(M3 契约 §2.2 pass 结构可扩展口径)。

use crate::mir::{Body, Statement, Terminator, TerminatorKind};

// ---------------------------------------------------------------------------
// 位向量
// ---------------------------------------------------------------------------

/// 定长位集(数据流状态载体)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BitSet {
    words: Vec<u64>,
    bits: usize,
}

impl BitSet {
    pub fn new(bits: usize) -> Self {
        BitSet {
            words: vec![0; bits.div_ceil(64)],
            bits,
        }
    }

    pub fn filled(bits: usize) -> Self {
        let mut s = Self::new(bits);
        s.fill();
        s
    }

    pub fn insert(&mut self, i: usize) {
        debug_assert!(i < self.bits);
        self.words[i / 64] |= 1u64 << (i % 64);
    }

    pub fn remove(&mut self, i: usize) {
        debug_assert!(i < self.bits);
        self.words[i / 64] &= !(1u64 << (i % 64));
    }

    pub fn contains(&self, i: usize) -> bool {
        debug_assert!(i < self.bits);
        self.words[i / 64] & (1u64 << (i % 64)) != 0
    }

    pub fn fill(&mut self) {
        for w in &mut self.words {
            *w = !0;
        }
        self.truncate_tail();
    }

    pub fn clear(&mut self) {
        for w in &mut self.words {
            *w = 0;
        }
    }

    /// 按位或;返回是否有变化(worklist 收敛判据)。
    pub fn union(&mut self, other: &BitSet) -> bool {
        let mut changed = false;
        for (a, b) in self.words.iter_mut().zip(&other.words) {
            let new = *a | b;
            changed |= new != *a;
            *a = new;
        }
        changed
    }

    /// 按位与;返回是否有变化(must 分析汇合)。
    pub fn intersect(&mut self, other: &BitSet) -> bool {
        let mut changed = false;
        for (a, b) in self.words.iter_mut().zip(&other.words) {
            let new = *a & b;
            changed |= new != *a;
            *a = new;
        }
        changed
    }

    fn truncate_tail(&mut self) {
        let rem = self.bits % 64;
        if rem != 0
            && let Some(last) = self.words.last_mut()
        {
            *last &= (1u64 << rem) - 1;
        }
    }
}

// ---------------------------------------------------------------------------
// 分析接口
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Forward,
    Backward,
}

/// 程序点:`stmt < stmts.len()` 指语句,`== stmts.len()` 指终结子。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Location {
    pub block: usize,
    pub stmt: usize,
}

/// 数据流分析定义(转移函数 + 边界/汇合)。
pub trait Analysis {
    /// 状态位宽(通常 = locals 数)。
    fn bits(&self, body: &Body) -> usize;

    fn direction(&self) -> Direction {
        Direction::Forward
    }

    /// 入口边界状态(前向 = bb0 入口;后向 = exit 块出口)。
    fn boundary(&self, body: &Body, state: &mut BitSet);

    /// 非边界块的初始格底(默认全 0 = may 分析;must 分析覆写为全 1)。
    fn bottom(&self, body: &Body) -> BitSet {
        BitSet::new(self.bits(body))
    }

    /// 汇合(默认按位或 = may;must 分析覆写为按位与)。
    fn join(&self, into: &mut BitSet, from: &BitSet) -> bool {
        into.union(from)
    }

    fn stmt_effect(&self, state: &mut BitSet, stmt: &Statement, loc: Location);

    fn term_effect(&self, state: &mut BitSet, term: &Terminator, loc: Location);
}

// ---------------------------------------------------------------------------
// CFG 边
// ---------------------------------------------------------------------------

/// 终结子后继(CFG 出边)。
pub fn successors(term: &TerminatorKind) -> Vec<usize> {
    match term {
        TerminatorKind::Goto(b) => vec![b.0 as usize],
        TerminatorKind::SwitchBool { then, else_, .. } => {
            if then == else_ {
                vec![then.0 as usize]
            } else {
                vec![then.0 as usize, else_.0 as usize]
            }
        }
        TerminatorKind::Call { next, .. } => vec![next.0 as usize],
        TerminatorKind::Return | TerminatorKind::Unreachable => Vec::new(),
    }
}

fn predecessors(body: &Body) -> Vec<Vec<usize>> {
    let mut preds = vec![Vec::new(); body.blocks.len()];
    for (i, bb) in body.blocks.iter().enumerate() {
        for s in successors(&bb.terminator.kind) {
            preds[s].push(i);
        }
    }
    preds
}

// ---------------------------------------------------------------------------
// 不动点引擎
// ---------------------------------------------------------------------------

/// 不动点产物:每块的入边状态(前向 = 块入口;后向 = 块出口)。
pub struct Results {
    pub entry: Vec<BitSet>,
}

impl Results {
    /// 重放单块转移,在每个程序点(转移**前**)回调观察器。
    /// 前向按语句序、后向按逆序;`loc.stmt == stmts.len()` 为终结子点。
    pub fn visit_block<A: Analysis>(
        &self,
        body: &Body,
        analysis: &A,
        block: usize,
        mut before: impl FnMut(&BitSet, Location),
    ) {
        let bb = &body.blocks[block];
        let mut state = self.entry[block].clone();
        let term_loc = Location {
            block,
            stmt: bb.stmts.len(),
        };
        match analysis.direction() {
            Direction::Forward => {
                for (i, stmt) in bb.stmts.iter().enumerate() {
                    let loc = Location { block, stmt: i };
                    before(&state, loc);
                    analysis.stmt_effect(&mut state, stmt, loc);
                }
                before(&state, term_loc);
            }
            Direction::Backward => {
                before(&state, term_loc);
                let mut state = state.clone();
                analysis.term_effect(&mut state, &bb.terminator, term_loc);
                for (i, stmt) in bb.stmts.iter().enumerate().rev() {
                    let loc = Location { block, stmt: i };
                    before(&state, loc);
                    analysis.stmt_effect(&mut state, stmt, loc);
                }
            }
        }
    }
}

/// worklist 不动点迭代(单调格 + 有限位宽 ⇒ 必然收敛)。
pub fn iterate_to_fixpoint<A: Analysis>(body: &Body, analysis: &A) -> Results {
    let n = body.blocks.len();
    let dir = analysis.direction();
    let mut entry: Vec<BitSet> = (0..n).map(|_| analysis.bottom(body)).collect();

    // 边界:前向 = bb0;后向 = 全部无后继块(Return/Unreachable)
    let mut worklist: Vec<usize> = Vec::new();
    match dir {
        Direction::Forward => {
            if n > 0 {
                analysis.boundary(body, &mut entry[0]);
                worklist.push(0);
            }
        }
        Direction::Backward => {
            for (i, bb) in body.blocks.iter().enumerate() {
                if successors(&bb.terminator.kind).is_empty() {
                    analysis.boundary(body, &mut entry[i]);
                }
                worklist.push(i);
            }
        }
    }
    let preds = predecessors(body);
    let mut on_list = vec![true; n];
    for (i, v) in on_list.iter_mut().enumerate() {
        *v = worklist.contains(&i);
    }

    while let Some(b) = worklist.pop() {
        on_list[b] = false;
        let bb = &body.blocks[b];
        let mut state = entry[b].clone();
        let term_loc = Location {
            block: b,
            stmt: bb.stmts.len(),
        };
        // 块内转移
        match dir {
            Direction::Forward => {
                for (i, stmt) in bb.stmts.iter().enumerate() {
                    analysis.stmt_effect(&mut state, stmt, Location { block: b, stmt: i });
                }
                analysis.term_effect(&mut state, &bb.terminator, term_loc);
            }
            Direction::Backward => {
                analysis.term_effect(&mut state, &bb.terminator, term_loc);
                for (i, stmt) in bb.stmts.iter().enumerate().rev() {
                    analysis.stmt_effect(&mut state, stmt, Location { block: b, stmt: i });
                }
            }
        }
        // 出边传播
        let targets: Vec<usize> = match dir {
            Direction::Forward => successors(&bb.terminator.kind),
            Direction::Backward => preds[b].clone(),
        };
        for t in targets {
            if analysis.join(&mut entry[t], &state) && !on_list[t] {
                on_list[t] = true;
                worklist.push(t);
            }
        }
    }
    Results { entry }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::mir::StatementKind;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    fn with_mir(src: &str, f: impl FnOnce(&[crate::mir::Body])) {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "{:?}", diag.emitted());
        let mir = cx.mir_crate();
        assert!(!mir.is_empty());
        f(&mir);
    }

    /// 前向 may 分析:被赋值过的 local 集(gen = Assign 目标,无 kill)。
    struct EverAssigned;

    impl Analysis for EverAssigned {
        fn bits(&self, body: &Body) -> usize {
            body.locals.len()
        }

        fn boundary(&self, body: &Body, state: &mut BitSet) {
            for i in 1..=body.arg_count {
                state.insert(i);
            }
        }

        fn stmt_effect(&self, state: &mut BitSet, stmt: &Statement, _loc: Location) {
            let StatementKind::Assign(p, _) = &stmt.kind;
            if p.proj.is_empty() {
                state.insert(p.local.0 as usize);
            }
        }

        fn term_effect(&self, state: &mut BitSet, term: &Terminator, _loc: Location) {
            if let TerminatorKind::Call { dest, .. } = &term.kind
                && dest.proj.is_empty()
            {
                state.insert(dest.local.0 as usize);
            }
        }
    }

    /// 后向 may 分析:可达 Return 的程序点(bit 0)。
    struct ReachesReturn;

    impl Analysis for ReachesReturn {
        fn bits(&self, _body: &Body) -> usize {
            1
        }

        fn direction(&self) -> Direction {
            Direction::Backward
        }

        fn boundary(&self, _body: &Body, _state: &mut BitSet) {}

        fn stmt_effect(&self, _state: &mut BitSet, _stmt: &Statement, _loc: Location) {}

        fn term_effect(&self, state: &mut BitSet, term: &Terminator, _loc: Location) {
            if matches!(term.kind, TerminatorKind::Return) {
                state.insert(0);
            }
        }
    }

    //@ spec: RXS-0054
    #[test]
    fn forward_fixpoint_converges_on_loop_cfg() {
        // while 循环 + 条件分支:回边存在下收敛,且循环体内可见循环前赋值
        with_mir(
            "fn main() {\n    let mut n = 0;\n    while n < 3 {\n        n += 1;\n    }\n    let _done = n;\n}",
            |bodies| {
                let body = &bodies[0];
                let results = iterate_to_fixpoint(body, &EverAssigned);
                // n(_1)在 bb0 赋值;所有非入口可达块的入口处 _1 已置位
                for (i, bb) in body.blocks.iter().enumerate() {
                    if i == 0 || matches!(bb.terminator.kind, TerminatorKind::Unreachable) {
                        continue;
                    }
                    assert!(
                        results.entry[i].contains(1),
                        "bb{i} 入口缺 _1 的赋值事实: {:?}",
                        results.entry[i]
                    );
                }
            },
        );
    }

    //@ spec: RXS-0054
    #[test]
    fn backward_direction_propagates_against_edges() {
        with_mir(
            "fn main() {\n    let mut n = 0;\n    while n < 3 {\n        n += 1;\n    }\n}",
            |bodies| {
                let body = &bodies[0];
                let results = iterate_to_fixpoint(body, &ReachesReturn);
                // 入口块出口可达 return(经循环退出路径)
                assert!(results.entry[0].contains(0), "{:?}", results.entry[0]);
            },
        );
    }

    #[test]
    fn bitset_ops() {
        let mut a = BitSet::new(70);
        a.insert(0);
        a.insert(69);
        assert!(a.contains(0) && a.contains(69) && !a.contains(35));
        let mut b = BitSet::new(70);
        b.insert(35);
        assert!(a.union(&b));
        assert!(!a.union(&b), "重复 union 无变化");
        assert!(a.contains(35));
        let mut f = BitSet::filled(70);
        assert!(f.contains(69));
        assert!(f.intersect(&a));
        assert_eq!(f, a);
        a.remove(69);
        assert!(!a.contains(69));
    }

    /// visit_block 重放:语句点状态 = 入口态 + 前缀转移(前向)。
    #[test]
    fn visit_block_replays_transfer() {
        with_mir(
            "fn main() {\n    let a = 1;\n    let b = a + 1;\n    let _c = b;\n}",
            |bodies| {
                let body = &bodies[0];
                let results = iterate_to_fixpoint(body, &EverAssigned);
                let mut seen_states: Vec<(Location, Vec<usize>)> = Vec::new();
                results.visit_block(body, &EverAssigned, 0, |state, loc| {
                    let set: Vec<usize> = (0..body.locals.len())
                        .filter(|i| state.contains(*i))
                        .collect();
                    seen_states.push((loc, set));
                });
                // 状态单调增长(本分析无 kill)
                for w in seen_states.windows(2) {
                    assert!(w[0].1.len() <= w[1].1.len(), "{seen_states:?}");
                }
            },
        );
    }
}
