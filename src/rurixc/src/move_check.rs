//! move/init 数据流检查(RXS-0053/RXS-0054;M3_PLAN §2 任务 3,4xxx 首批)。
//!
//! 基于 [`crate::dataflow`] 前向 may 分析,按 **local 粒度**(M3.2 保守口径,
//! RXS-0054:move 出字段 = 整 local 视为 moved;字段级精度随 M3.3+ 只追加):
//! - 状态位 `[0, n)` = maybe-uninit(可能未初始化);
//! - 状态位 `[n, 2n)` = maybe-moved(可能已移出);
//! - 整体赋值 kill 两组位(重新初始化,RXS-0054);
//! - `Operand::Move` gen maybe-moved(经解引用 move → RX4003,不改状态)。
//!
//! 诊断 = 不动点后逐块重放,在每个使用点对照状态(07 §4 先正确性后诊断,
//! 措辞保守粗糙;同 local 同码去重防噪)。检查时点 = MIR 构造后、codegen 前
//! (RXS-0054 实现要求),对全部单态化 body 强制。

use std::collections::HashSet;

use crate::dataflow::{Analysis, BitSet, Location, Results, iterate_to_fixpoint};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::mir::{
    Body, LocalIdx, Operand, Place, ProjElem, Rvalue, Statement, StatementKind, Terminator,
    TerminatorKind,
};
use crate::span::Span;

pub const E_USE_AFTER_MOVE: ErrorCode = ErrorCode(4001); // RX4001
pub const E_USE_BEFORE_INIT: ErrorCode = ErrorCode(4002); // RX4002
pub const E_MOVE_OUT_OF_REF: ErrorCode = ErrorCode(4003); // RX4003

/// 对单个 MIR body 跑 move/init 检查(诊断经 `diag`)。
pub fn check_body(diag: &DiagCtxt, body: &Body) {
    let analysis = InitMove::new(body);
    let results: Results = iterate_to_fixpoint(body, &analysis);
    let mut rep = Reporter {
        diag,
        body,
        n: body.locals.len(),
        reported: HashSet::new(),
    };
    for b in 0..body.blocks.len() {
        results.visit_block(body, &analysis, b, |state, loc: Location| {
            let bb = &body.blocks[loc.block];
            if loc.stmt < bb.stmts.len() {
                rep.check_stmt(state, &bb.stmts[loc.stmt]);
            } else {
                rep.check_term(state, &bb.terminator);
            }
        });
    }
}

/// rvalue 的 operand 读集(Ref/Discriminant 的 place 读单列)。
pub(crate) fn rvalue_operands(rv: &Rvalue) -> Vec<&Operand> {
    match rv {
        Rvalue::Use(o) | Rvalue::UnaryOp(_, o) | Rvalue::Cast(o, _) => vec![o],
        Rvalue::BinaryOp(_, a, b) => vec![a, b],
        Rvalue::Aggregate(_, ops) | Rvalue::VariantAggregate { ops, .. } => ops.iter().collect(),
        Rvalue::Ref(..) | Rvalue::Discriminant(_) => Vec::new(),
    }
}

pub(crate) fn place_has_deref(p: &Place) -> bool {
    p.proj.iter().any(|e| matches!(e, ProjElem::Deref))
}

// ---------------------------------------------------------------------------
// 数据流转移(纯函数;诊断在重放期)
// ---------------------------------------------------------------------------

pub(crate) struct InitMove {
    /// locals 数(位宽 = 2n)。
    pub(crate) n: usize,
    arg_count: usize,
    /// use-before-init 跟踪豁免的 local(按 LocalIdx 下标)。device 存储收紧
    /// (RXS-0054/RXS-0079):仅"经 Index 元素级写入的 device 存储"(`shared let`
    /// addrspace(3) 缓冲 / `[T; N]` 数组缓冲)豁免——其元素写入常受 thread-id
    /// 边界守卫条件化(如 tiled transpose `if x<w { tile[..]=.. }`),may-uninit
    /// 数据流在分支汇合处会保守判其仍可能未初始化,但 codegen 背书该背景存储,
    /// 故按 body 级"存在元素写"豁免(非流敏感)。真正未经元素写的标量
    /// (`shared let acc: f32;` 未写先读)不豁免,仍报 RX4002。
    exempt: Vec<bool>,
}

impl InitMove {
    pub(crate) fn new(body: &Body) -> InitMove {
        let mut indexed_write = vec![false; body.locals.len()];
        let mut note = |place: &Place| {
            if place.proj.iter().any(|e| matches!(e, ProjElem::Index(_))) {
                indexed_write[place.local.0 as usize] = true;
            }
        };
        for bb in &body.blocks {
            for stmt in &bb.stmts {
                let StatementKind::Assign(dest, _) = &stmt.kind;
                note(dest);
            }
            if let TerminatorKind::Call { dest, .. } = &bb.terminator.kind {
                note(dest);
            }
        }
        let exempt = body
            .locals
            .iter()
            .enumerate()
            .map(|(l, d)| (d.shared || d.array_len.is_some()) && indexed_write[l])
            .collect();
        InitMove {
            n: body.locals.len(),
            arg_count: body.arg_count,
            exempt,
        }
    }

    fn apply_move_op(&self, state: &mut BitSet, op: &Operand) {
        if let Operand::Move(p) = op {
            // 经解引用的 move 是 RX4003 违例(重放期报),不污染 base 状态;
            // 其余(整 local 或字段投影)按 RXS-0054 保守置整 local moved
            if !place_has_deref(p) {
                state.insert(self.n + p.local.0 as usize);
            }
        }
    }

    fn apply_dest(&self, state: &mut BitSet, dest: &Place) {
        // 整体赋值 =(重新)初始化;投影写不改 base 状态(保守)
        if dest.proj.is_empty() {
            state.remove(dest.local.0 as usize);
            state.remove(self.n + dest.local.0 as usize);
        }
    }
}

impl Analysis for InitMove {
    fn bits(&self, body: &Body) -> usize {
        2 * body.locals.len()
    }

    fn boundary(&self, _body: &Body, state: &mut BitSet) {
        // 入口:参数已初始化;返回槽与其余局部 maybe-uninit(RXS-0054)。
        // device 扩展收紧:仅"经 Index 元素级写入的 device 存储"(`shared let` /
        // `[T; N]` 数组缓冲,见 [`InitMove::exempt`])由 codegen 背书其存储而不纳入
        // use-before-init 跟踪;未经元素写的标量(含 `shared let` 标量)仍跟踪,
        // 真正未写先读报 RX4002。
        state.insert(0);
        for l in (self.arg_count + 1)..self.n {
            if self.exempt[l] {
                continue;
            }
            state.insert(l);
        }
    }

    fn stmt_effect(&self, state: &mut BitSet, stmt: &Statement, _loc: Location) {
        let StatementKind::Assign(dest, rv) = &stmt.kind;
        for op in rvalue_operands(rv) {
            self.apply_move_op(state, op);
        }
        self.apply_dest(state, dest);
    }

    fn term_effect(&self, state: &mut BitSet, term: &Terminator, _loc: Location) {
        if let TerminatorKind::Call { args, dest, .. } = &term.kind {
            for op in args {
                self.apply_move_op(state, op);
            }
            self.apply_dest(state, dest);
        }
    }
}

// ---------------------------------------------------------------------------
// 重放报告器
// ---------------------------------------------------------------------------

struct Reporter<'a> {
    diag: &'a DiagCtxt,
    body: &'a Body,
    n: usize,
    /// (local, 错误码) 去重(循环/重复使用防噪)。
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

    /// place 读取合法性(RXS-0054):全部到达路径已初始化且未被 move。
    fn check_read(&mut self, state: &BitSet, place: &Place, span: Span, is_move: bool) {
        let l = place.local;
        if is_move && place_has_deref(place) {
            self.report(
                E_MOVE_OUT_OF_REF,
                "borrowck.move_out_of_ref",
                l,
                span,
                "cannot move out of a reference (RXS-0053)",
            );
            return;
        }
        if state.contains(l.0 as usize) {
            self.report(
                E_USE_BEFORE_INIT,
                "borrowck.use_before_init",
                l,
                span,
                "used here but it is possibly-uninitialized",
            );
        } else if state.contains(self.n + l.0 as usize) {
            self.report(
                E_USE_AFTER_MOVE,
                "borrowck.use_after_move",
                l,
                span,
                "value used here after move",
            );
        }
    }

    fn check_op(&mut self, state: &BitSet, op: &Operand, span: Span) {
        match op {
            Operand::Copy(p) => self.check_read(state, p, span, false),
            Operand::Move(p) => self.check_read(state, p, span, true),
            Operand::Const(_) => {}
        }
    }

    fn check_stmt(&mut self, state: &BitSet, stmt: &Statement) {
        let StatementKind::Assign(dest, rv) = &stmt.kind;
        for op in rvalue_operands(rv) {
            self.check_op(state, op, stmt.span);
        }
        match rv {
            // 取引用 / 判别读取 = 使用(须已初始化且未 move,RXS-0054)
            Rvalue::Ref(_, p) | Rvalue::Discriminant(p) => {
                self.check_read(state, p, stmt.span, false);
            }
            _ => {}
        }
        // 投影写(字段/解引用):base 须已初始化(RXS-0054)
        if !dest.proj.is_empty() {
            self.check_read(state, &Place::local(dest.local), stmt.span, false);
        }
    }

    fn check_term(&mut self, state: &BitSet, term: &Terminator) {
        match &term.kind {
            TerminatorKind::SwitchBool { discr, .. } => {
                self.check_op(state, discr, term.span);
            }
            TerminatorKind::Call { args, dest, .. } => {
                for op in args {
                    self.check_op(state, op, term.span);
                }
                if !dest.proj.is_empty() {
                    self.check_read(state, &Place::local(dest.local), term.span, false);
                }
            }
            // Drop 为编译器插入(RXS-0055),不构成用户可见 use,不诊断
            TerminatorKind::Drop { .. }
            | TerminatorKind::Goto(_)
            | TerminatorKind::Return
            | TerminatorKind::Unreachable => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    fn check(src: &str) -> Vec<u16> {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "前置诊断: {:?}", diag.emitted());
        let _ = cx.mir_crate();
        cx.check_moves();
        diag.emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect()
    }

    //@ spec: RXS-0054
    #[test]
    fn use_after_move_detected() {
        let codes = check(
            "struct T { id: i32 }\nfn eat(t: T) -> i32 { t.id }\nfn main() {\n    let t = T { id: 1 };\n    let _a = eat(t);\n    let _b = eat(t);\n}",
        );
        assert_eq!(codes, vec![4001]);
    }

    //@ spec: RXS-0054
    #[test]
    fn maybe_moved_on_branch_detected() {
        let codes = check(
            "struct T { id: i32 }\nfn eat(t: T) -> i32 { t.id }\nfn main() {\n    let t = T { id: 1 };\n    let f = true;\n    if f {\n        let _a = eat(t);\n    }\n    let _b = eat(t);\n}",
        );
        assert_eq!(codes, vec![4001]);
    }

    //@ spec: RXS-0054
    #[test]
    fn use_before_init_detected() {
        let codes = check("fn main() {\n    let x: i32;\n    let _y = x + 1;\n}");
        assert_eq!(codes, vec![4002]);
    }

    //@ spec: RXS-0054
    #[test]
    fn maybe_uninit_on_loop_path_detected() {
        let codes = check(
            "fn main() {\n    let x: i32;\n    let mut i = 0;\n    while i < 3 {\n        x = i;\n        i += 1;\n    }\n    let _y = x;\n}",
        );
        assert_eq!(codes, vec![4002]);
    }

    //@ spec: RXS-0053
    #[test]
    fn move_out_of_reference_detected() {
        let codes = check(
            "struct T { id: i32 }\nfn peek(r: &T) -> T {\n    *r\n}\nfn main() {\n    let t = T { id: 1 };\n    let _u = peek(&t);\n}",
        );
        assert_eq!(codes, vec![4003]);
    }

    //@ spec: RXS-0054
    #[test]
    fn reinit_after_move_is_clean() {
        let codes = check(
            "struct T { id: i32 }\nfn eat(t: T) -> i32 { t.id }\nfn main() {\n    let mut t = T { id: 1 };\n    let a = eat(t);\n    t = T { id: a };\n    let _b = eat(t);\n}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0054
    #[test]
    fn branch_converged_init_is_clean() {
        let codes = check(
            "fn main() {\n    let f = true;\n    let c: i32;\n    if f {\n        c = 1;\n    } else {\n        c = 2;\n    }\n    let _u = c + 1;\n}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0053
    #[test]
    fn copy_types_do_not_move() {
        let codes = check(
            "#[derive(Copy)]\nstruct P { x: i32 }\nfn take(p: P) -> i32 { p.x }\nfn main() {\n    let p = P { x: 1 };\n    let _a = take(p);\n    let _b = take(p);\n}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0054
    #[test]
    fn move_out_of_field_marks_whole_local() {
        // M3.2 保守粒度:move 出字段 → 整 local moved
        let codes = check(
            "struct T { id: i32 }\nstruct W { a: T, b: T }\nfn eat(t: T) -> i32 { t.id }\nfn main() {\n    let w = W { a: T { id: 1 }, b: T { id: 2 } };\n    let _x = eat(w.a);\n    let _y = eat(w.b);\n}",
        );
        assert_eq!(codes, vec![4001]);
    }

    //@ spec: RXS-0054
    #[test]
    fn borrow_of_uninit_rejected_and_init_borrow_clean() {
        let codes = check("fn main() {\n    let x: i32;\n    let _r = &x;\n}");
        assert_eq!(codes, vec![4002]);
        let codes =
            check("fn main() {\n    let x: i32 = 1;\n    let _r = &x;\n    let _v = *_r;\n}");
        assert!(codes.is_empty(), "{codes:?}");
    }

    // -- device MIR 安全门(kernel/device fn use-after-move,M3 安全检查 device 扩展) --

    /// 经 device 安全门(check_device_safety)对 device MIR 跑 move/init。
    fn check_device(src: &str) -> Vec<u16> {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        assert!(diag.emitted().is_empty(), "前置诊断: {:?}", diag.emitted());
        cx.check_device_safety();
        let mut codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        codes.sort_unstable();
        codes
    }

    //@ spec: RXS-0054
    #[test]
    fn kernel_use_after_move_detected() {
        // kernel 体内 use-after-move → RX4001(device MIR 安全门)
        let codes = check_device(
            "struct T { id: i32 }\ndevice fn eat(t: T) -> i32 { t.id }\nkernel fn k() {\n    let v = T { id: 1 };\n    let _a = eat(v);\n    let _b = eat(v);\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![4001]);
    }

    //@ spec: RXS-0054
    #[test]
    fn kernel_shared_buffer_not_uninit() {
        // shared let 缓冲元素级写入不误报 use-before-init(device 存储,RX4002 豁免)
        let codes = check_device(
            "kernel fn k(t: ThreadCtx<1>, dst: ViewMut<global, f32>) {\n    shared let tile: [f32; 64];\n    let i = t.thread_index();\n    tile[i] = 1.0;\n    dst[i] = tile[i];\n}\nfn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0054
    #[test]
    fn kernel_shared_buffer_conditional_write_not_uninit() {
        // thread-id 边界守卫下的条件化元素写(tiled transpose 模式):may-uninit
        // 数据流在分支汇合处保守判其可能未初始化,但 device 存储经 body 级"存在
        // 元素写"豁免(非流敏感),不误报 RX4002(收紧后仍豁免)。
        let codes = check_device(
            "kernel fn k(t: ThreadCtx<1>, src: View<global, f32>, dst: ViewMut<global, f32>, w: usize) {\n    shared let tile: [f32; 64];\n    let i = t.thread_index();\n    if i < w {\n        tile[i] = src[i];\n    }\n    block.sync();\n    dst[i] = tile[i];\n}\nfn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0054
    #[test]
    fn kernel_shared_scalar_use_before_init_detected() {
        // device 存储收紧:未经元素写的 `shared let` 标量(无 Index 写)不再整体
        // 豁免;未写先读 → RX4002(对真正未初始化标量仍报)。
        let codes = check_device(
            "kernel fn k(t: ThreadCtx<1>, dst: ViewMut<global, f32>) {\n    shared let acc: f32;\n    let i = t.thread_index();\n    dst[i] = acc;\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![4002]);
    }

    //@ spec: RXS-0054
    #[test]
    fn kernel_local_scalar_use_before_init_detected() {
        // 普通(非 shared/非数组)device 标量未写先读 → RX4002(收紧不波及既有口径)。
        let codes = check_device(
            "kernel fn k(t: ThreadCtx<1>, dst: ViewMut<global, f32>) {\n    let x: f32;\n    let i = t.thread_index();\n    dst[i] = x + 1.0;\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![4002]);
    }

    //@ spec: RXS-0054
    #[test]
    fn host_error_defers_kernel_safety_but_does_not_hide_it() {
        use crate::diag::DiagCtxt;
        use crate::query::QueryCtx;
        use crate::span::{Edition, SourceId};

        // 顺序约束(driver.rs / query.rs::check_device_safety):device 安全门仅在
        // 前序(含 host move/borrow)无错时构建 device MIR。混合场景——host fn
        // use-after-move(RX4001)+ kernel use-after-move(RX4001):host 检查先报错,
        // has_errors 置位 → driver 阶段化跳过 device 安全门(防 device lowering 噪声),
        // 程序整体仍被拒(非误放行);host 错误一旦修复,kernel 错误经 device 安全门浮现。
        let mixed = "struct T { id: i32 }\ndevice fn eat(t: T) -> i32 { t.id }\nfn heat(t: T) -> i32 { t.id }\nkernel fn kk() {\n    let v = T { id: 2 };\n    let _c = eat(v);\n    let _d = eat(v);\n}\nfn main() {\n    let h = T { id: 1 };\n    let _a = heat(h);\n    let _b = heat(h);\n}";

        // 复刻 driver 阶段化:host move 检查先行。
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(mixed, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        assert!(!diag.has_errors(), "前置应干净: {:?}", diag.emitted());
        let _ = cx.mir_crate();
        cx.check_moves();
        assert!(diag.has_errors(), "host use-after-move 应报");
        // 此处 has_errors → driver 跳过 check_device_safety(阶段化中止):程序已被
        // 拒,kernel 错误顺延到 host 修复后,非漏报。

        // host 修复后(同 kernel),kernel use-after-move 经 device 安全门浮现。
        let fixed = "struct T { id: i32 }\ndevice fn eat(t: T) -> i32 { t.id }\nkernel fn kk() {\n    let v = T { id: 2 };\n    let _c = eat(v);\n    let _d = eat(v);\n}\nfn main() {}";
        let codes = check_device(fixed);
        assert_eq!(
            codes,
            vec![4001],
            "kernel use-after-move 须经 device 安全门捕获"
        );
    }
}
