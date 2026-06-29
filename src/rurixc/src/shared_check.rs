//! shared+barrier 一致性 — MIR 借用检查的 device 扩展 pass 的数据流分析(spec 条款
//! RXS-0079,spec/device.md;06 §2.2 D-123 / 07 §4 保守先行)。
//!
//! 实现裁决:与 [`crate::views_check`] 同层,本 pass 在 **HIR 层**遍历(kernel/device
//! body 不在 host `main` 可达 MIR 内),仅对 **device 上下文 body**(`kernel`/`device`
//! 着色)实施;以**管线顺序**满足 spec「MIR 借用检查 device 扩展 / 在 host 借用检查
//! 之后运行」([`crate::query::QueryCtx::check_shared_barrier`] 在 `check_views` 之后、
//! device codegen 之前接入)。把 M4.1 的 barrier uniform 保守骨架(RXS-0068,HIR)
//! **完整化为 `shared let` 读写与 `block.sync()` barrier 的一致性数据流判定**。
//!
//! 一致性规则(RXS-0079):对同一 `shared` 位置,某 lane 的写入对**他 lane** 的读取
//! 可见当且仅当写读之间隔有 block 级 barrier(`block.sync()`)同步;否则为 shared
//! 数据竞争。
//!
//! - **写后未同步跨 lane 读**(`RX3009`):`shared` 位置写后未过 `block.sync()` 即读
//!   **他 lane** 写入(下标证不出同 lane)→ 一致性违例。
//! - **本 lane 自读自写**:同一下标的写后自读无须 barrier(数据流内即可见),不报。
//!
//! 保守上界(07 §4):能证写读间隔 barrier 且为同 lane 才放行;证不出(下标不可判 /
//! 跨 lane 别名不可判)保守拒绝(误拒边界情形,措辞容许粗糙)。`unsafe` 块内豁免
//! (承担 P-03 验证义务,对齐 [`crate::views_check`] / [`crate::coloring`] 骨架豁免)。
//! MVP 作用面 = 下标化 `shared` 数组访问(`tile[idx]`);裸标量 `shared` 跨 lane 判定
//! 随真实 kernel 需求扩展(经 conformance 类别留痕)。scoped atomics 类型契约见
//! RXS-0080([`crate::typeck`]);PTX 映射为 D-406 / RD-008 高敏面(deferred,agent 可落笔、agent 自主落地)。

use std::collections::{HashMap, HashSet};

use crate::ast::{FnColor, LitKind};
use crate::diag::ErrorCode;
use crate::hir::{self, Body, BodyId, Crate, DefId, Expr, ExprKind, LocalId, PatKind, Res, Stmt};
use crate::query::QueryCtx;
use crate::span::Span;
use crate::ty::Ty;

pub const E_SHARED_BARRIER: ErrorCode = ErrorCode(3009); // RX3009(RXS-0079)
pub const E_DEVICE_CONSTRAINT: ErrorCode = ErrorCode(6005); // RX6005(RXS-0071/0079)

/// block barrier 方法名(`block.sync()`,RXS-0072/0079;对齐 [`crate::coloring`])。
const BARRIER_METHOD: &str = "sync";

/// 全 crate shared+barrier 一致性入口(provider:[`QueryCtx::check_shared_barrier`])。
pub fn check_crate(cx: &QueryCtx<'_>) {
    let krate = cx.hir_crate();
    for i in 0..krate.bodies.len() {
        let body_id = BodyId(i as u32);
        let body = krate.body(body_id);
        // device 扩展:仅 device 上下文(kernel/device 着色)实施(消费着色信息)。
        if !is_device_ctx(context_color(&krate, body.owner)) {
            continue;
        }
        let mut shared: HashSet<u32> = HashSet::new();
        collect_shared(&body.value, &mut shared);
        check_shared_array_shapes(cx, body_id, body, &shared);
        if shared.is_empty() {
            continue; // 无 `shared let`:本 body 无一致性义务。
        }
        let checker = Checker { cx, body, shared };
        let mut state: State = HashMap::new();
        checker.walk_expr(&body.value, &mut state, false);
    }
}

/// body 的上下文着色(fn 取其着色;const/static 初始化器视为 host)。
fn context_color(krate: &Crate, owner: DefId) -> FnColor {
    match &krate.item(owner).kind {
        hir::ItemKind::Fn(decl) => decl.color,
        _ => FnColor::Host,
    }
}

fn is_device_ctx(c: FnColor) -> bool {
    matches!(c, FnColor::Device | FnColor::Kernel)
}

/// 预扫描:收集 `shared let` 引入的局部(addrspace 3 块作用域存储,RXS-0071)。
fn collect_shared(e: &Expr, out: &mut HashSet<u32>) {
    walk_blocks(e, &mut |stmt| {
        if let Stmt::Let {
            pat, shared: true, ..
        } = stmt
            && let PatKind::Binding { local } = &pat.kind
        {
            out.insert(local.0);
        }
    });
}

/// `shared let` 必须为固定长度数组(M5.3 review fix;标量 shared silent 错误 lowering)。
fn check_shared_array_shapes(
    cx: &QueryCtx<'_>,
    body_id: BodyId,
    body: &Body,
    shared: &HashSet<u32>,
) {
    if shared.is_empty() {
        return;
    }
    let tcr = cx.check_body(body_id);
    walk_blocks(&body.value, &mut |stmt| {
        let Stmt::Let {
            pat, shared: true, ..
        } = stmt
        else {
            return;
        };
        let PatKind::Binding { local } = &pat.kind else {
            return;
        };
        if !shared.contains(&local.0) {
            return;
        }
        let Some(ty) = tcr.local_ty.get(local.0 as usize) else {
            return;
        };
        if !matches!(ty, Ty::Array(_)) {
            cx.diag()
                .struct_error(E_DEVICE_CONSTRAINT, "codegen.device_constraint")
                .arg(
                    "detail",
                    "shared let requires a fixed-size array type (RXS-0071/0079)",
                )
                .span_label(pat.span, "device codegen constraint violated")
                .emit();
        }
    });
}

/// 一个 `shared` 位置的同步状态(数据流格)。
#[derive(Clone, PartialEq)]
enum WriteState {
    /// 无未同步写(已过 barrier / 从未写)。
    Clean,
    /// 写后未过 barrier;`Some(key)` 为写下标的静态键(可判同 lane),`None` 为
    /// 不可判下标 / 分支合并后歧义(任意读保守视作跨 lane)。
    Pending(Option<String>),
}

/// 数据流状态:shared 局部 → 同步状态(缺省项视作 [`WriteState::Clean`])。
type State = HashMap<u32, WriteState>;

struct Checker<'a, 'q> {
    cx: &'a QueryCtx<'q>,
    body: &'a Body,
    /// `shared let` 局部集(LocalId.0)。
    shared: HashSet<u32>,
}

impl Checker<'_, '_> {
    fn local_name(&self, l: LocalId) -> String {
        match self.body.locals.get(l.0 as usize) {
            Some(d) if !d.name.is_empty() => format!("`{}`", d.name),
            _ => "this shared location".to_owned(),
        }
    }

    /// 下标表达式的静态键(`Res::Local` / 整数字面量可判;复杂表达式不可判 → None)。
    fn index_key(&self, e: &Expr) -> Option<String> {
        match &e.kind {
            ExprKind::Res(Res::Local(l)) => Some(format!("L{}", l.0)),
            ExprKind::SynthInt(v) => Some(format!("#{v}")),
            ExprKind::Lit(l) if l.kind == LitKind::Int => {
                let text = self
                    .cx
                    .src()
                    .get(l.span.lo.0 as usize..l.span.hi.0 as usize)?;
                Some(format!("#{}", text.trim()))
            }
            _ => None,
        }
    }

    /// `expr[..]` 的基址若为 `shared` 局部则返回其 LocalId。
    fn shared_index_base(&self, base: &Expr) -> Option<LocalId> {
        if let ExprKind::Res(Res::Local(l)) = &base.kind
            && self.shared.contains(&l.0)
        {
            Some(*l)
        } else {
            None
        }
    }

    /// 记录对 shared 位置的写(写后置 Pending,等待 barrier 同步)。
    fn record_write(&self, l: LocalId, index: &Expr, state: &mut State) {
        state.insert(l.0, WriteState::Pending(self.index_key(index)));
    }

    /// 裁决对 shared 位置的读(RX3009;写后未同步跨 lane 读)。
    fn check_read(&self, l: LocalId, index: &Expr, span: Span, state: &mut State, in_unsafe: bool) {
        if in_unsafe {
            return;
        }
        let Some(WriteState::Pending(write_key)) = state.get(&l.0) else {
            return; // Clean:写读间已隔 barrier,或本位置从未写。
        };
        let read_key = self.index_key(index);
        let same_lane = matches!((write_key, &read_key), (Some(w), Some(r)) if w == r);
        if !same_lane {
            self.cx
                .diag()
                .struct_error(E_SHARED_BARRIER, "shared.barrier_consistency")
                .arg(
                    "detail",
                    format!(
                        "read of shared {} may observe another lane's write not synchronized by `block.sync()`",
                        self.local_name(l)
                    ),
                )
                .span_label(span, "shared read not synchronized by a block barrier")
                .emit();
        }
    }

    fn walk_block(&self, b: &hir::Block, state: &mut State, in_unsafe: bool) {
        for s in &b.stmts {
            match s {
                Stmt::Item(_) => {} // 嵌套 item body 经 check_crate 全集遍历
                Stmt::Let { init, .. } => {
                    if let Some(e) = init {
                        self.walk_expr(e, state, in_unsafe);
                    }
                }
                Stmt::Expr(e) => self.walk_expr(e, state, in_unsafe),
            }
        }
        if let Some(t) = &b.tail {
            self.walk_expr(t, state, in_unsafe);
        }
    }

    fn walk_expr(&self, e: &Expr, state: &mut State, in_unsafe: bool) {
        match &e.kind {
            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                self.walk_expr(receiver, state, in_unsafe);
                for a in args {
                    self.walk_expr(a, state, in_unsafe);
                }
                // `block.sync()` barrier:同步点 → 全 shared 位置转 Clean。
                if method == BARRIER_METHOD {
                    state.clear();
                }
            }
            ExprKind::Assign { lhs, rhs, .. } => {
                // 求值序:先 rhs(读)后 lhs(写)。
                self.walk_expr(rhs, state, in_unsafe);
                if let ExprKind::Index { expr, index } = &lhs.kind {
                    self.walk_expr(index, state, in_unsafe);
                    if let Some(l) = self.shared_index_base(expr) {
                        self.record_write(l, index, state);
                    } else {
                        self.walk_expr(expr, state, in_unsafe);
                    }
                } else {
                    self.walk_expr(lhs, state, in_unsafe);
                }
            }
            ExprKind::Index { expr, index } => {
                self.walk_expr(index, state, in_unsafe);
                if let Some(l) = self.shared_index_base(expr) {
                    self.check_read(l, index, e.span, state, in_unsafe);
                } else {
                    self.walk_expr(expr, state, in_unsafe);
                }
            }
            ExprKind::Call { callee, args } => {
                self.walk_expr(callee, state, in_unsafe);
                for a in args {
                    self.walk_expr(a, state, in_unsafe);
                }
            }
            ExprKind::Unary { expr, .. }
            | ExprKind::Borrow { expr, .. }
            | ExprKind::Cast { expr, .. }
            | ExprKind::Field { expr, .. }
            | ExprKind::TupleField { expr, .. } => self.walk_expr(expr, state, in_unsafe),
            ExprKind::Binary { lhs, rhs, .. }
            | ExprKind::Range {
                lo: lhs, hi: rhs, ..
            } => {
                self.walk_expr(lhs, state, in_unsafe);
                self.walk_expr(rhs, state, in_unsafe);
            }
            ExprKind::Tuple(v) | ExprKind::Array(v) => {
                for x in v {
                    self.walk_expr(x, state, in_unsafe);
                }
            }
            ExprKind::Repeat { elem, len } => {
                self.walk_expr(elem, state, in_unsafe);
                self.walk_expr(len, state, in_unsafe);
            }
            ExprKind::StructLit { fields, .. } => {
                for (_, v) in fields {
                    if let Some(x) = v {
                        self.walk_expr(x, state, in_unsafe);
                    }
                }
            }
            ExprKind::Block(b) => self.walk_block(b, state, in_unsafe),
            ExprKind::Unsafe(b) => self.walk_block(b, state, true),
            ExprKind::If { cond, then, else_ } => {
                self.walk_expr(cond, state, in_unsafe);
                let s0 = state.clone();
                self.walk_block(then, state, in_unsafe);
                let s_then = std::mem::replace(state, s0);
                if let Some(eb) = else_ {
                    self.walk_expr(eb, state, in_unsafe);
                }
                *state = merge(&s_then, state);
            }
            ExprKind::While { cond, body } => {
                self.walk_expr(cond, state, in_unsafe);
                let s0 = state.clone();
                self.walk_block(body, state, in_unsafe);
                *state = merge(&s0, state);
            }
            ExprKind::Loop { body } => {
                let s0 = state.clone();
                self.walk_block(body, state, in_unsafe);
                *state = merge(&s0, state);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee, state, in_unsafe);
                let s0 = state.clone();
                let mut merged: Option<State> = None;
                for arm in arms {
                    let mut arm_state = s0.clone();
                    if let Some(g) = &arm.guard {
                        self.walk_expr(g, &mut arm_state, in_unsafe);
                    }
                    self.walk_expr(&arm.body, &mut arm_state, in_unsafe);
                    merged = Some(match merged {
                        Some(m) => merge(&m, &arm_state),
                        None => arm_state,
                    });
                }
                if let Some(m) = merged {
                    *state = m;
                }
            }
            ExprKind::Return(op) | ExprKind::Break(op) => {
                if let Some(x) = op {
                    self.walk_expr(x, state, in_unsafe);
                }
            }
            ExprKind::Closure { body, .. } => self.walk_expr(body, state, in_unsafe),
            ExprKind::Lit(_)
            | ExprKind::SynthInt(_)
            | ExprKind::Res(_)
            | ExprKind::Continue
            | ExprKind::Err => {}
        }
    }
}

/// 控制流合并(保守上界):任一路径存在未同步写则合并后仍 Pending;两路径写下标
/// 不一致 → 下标歧义(None,任意读保守视作跨 lane)。
fn merge(a: &State, b: &State) -> State {
    let mut out: State = HashMap::new();
    let keys: HashSet<u32> = a.keys().chain(b.keys()).copied().collect();
    for k in keys {
        let wa = a.get(&k).cloned().unwrap_or(WriteState::Clean);
        let wb = b.get(&k).cloned().unwrap_or(WriteState::Clean);
        let merged = match (wa, wb) {
            (WriteState::Clean, WriteState::Clean) => WriteState::Clean,
            (WriteState::Pending(x), WriteState::Clean)
            | (WriteState::Clean, WriteState::Pending(x)) => WriteState::Pending(x),
            (WriteState::Pending(x), WriteState::Pending(y)) => {
                WriteState::Pending(if x == y { x } else { None })
            }
        };
        if merged != WriteState::Clean {
            out.insert(k, merged);
        }
    }
    out
}

/// 遍历表达式树,对每条 block 语句回调(预扫描 `shared let` 收集)。
fn walk_blocks(e: &Expr, f: &mut impl FnMut(&Stmt)) {
    match &e.kind {
        ExprKind::Block(b) | ExprKind::Unsafe(b) => {
            for s in &b.stmts {
                f(s);
                if let Stmt::Let { init: Some(e), .. } = s {
                    walk_blocks(e, f);
                }
                if let Stmt::Expr(e) = s {
                    walk_blocks(e, f);
                }
            }
            if let Some(t) = &b.tail {
                walk_blocks(t, f);
            }
        }
        ExprKind::If { cond, then, else_ } => {
            walk_blocks(cond, f);
            for s in &then.stmts {
                f(s);
                stmt_walk(s, f);
            }
            if let Some(t) = &then.tail {
                walk_blocks(t, f);
            }
            if let Some(eb) = else_ {
                walk_blocks(eb, f);
            }
        }
        ExprKind::While { cond, body } => {
            walk_blocks(cond, f);
            for s in &body.stmts {
                f(s);
                stmt_walk(s, f);
            }
            if let Some(t) = &body.tail {
                walk_blocks(t, f);
            }
        }
        ExprKind::Loop { body } => {
            for s in &body.stmts {
                f(s);
                stmt_walk(s, f);
            }
            if let Some(t) = &body.tail {
                walk_blocks(t, f);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            walk_blocks(scrutinee, f);
            for arm in arms {
                walk_blocks(&arm.body, f);
            }
        }
        _ => {}
    }
}

fn stmt_walk(s: &Stmt, f: &mut impl FnMut(&Stmt)) {
    match s {
        Stmt::Let { init: Some(e), .. } | Stmt::Expr(e) => walk_blocks(e, f),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// 跑 typeck + 着色 + shared+barrier 检查,返回 shared 诊断码序列。
    fn check(src: &str) -> Vec<u16> {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(
            diag.emitted().is_empty(),
            "前置类型诊断: {:?}",
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message(diag.messages())))
                .collect::<Vec<_>>()
        );
        cx.check_coloring();
        assert!(
            diag.emitted().is_empty(),
            "前置着色诊断: {:?}",
            diag.emitted().iter().map(|d| d.code).collect::<Vec<_>>()
        );
        cx.check_shared_barrier();
        let mut codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        codes.sort_unstable();
        codes
    }

    const HEAD: &str =
        "kernel fn k(t: ThreadCtx<1>, src: View<global, f32>, dst: ViewMut<global, f32>) {\n";

    //@ spec: RXS-0079
    #[test]
    fn write_barrier_then_cross_lane_read_is_clean() {
        let src = format!(
            "{HEAD}    shared let tile: [f32; 256];\n    let i = t.thread_index();\n    tile[i] = src[i];\n    block.sync();\n    dst[0] = tile[1];\n}}\nfn main() {{}}"
        );
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0079
    #[test]
    fn write_then_unsynced_cross_lane_read_is_rx3009() {
        let src = format!(
            "{HEAD}    shared let tile: [f32; 256];\n    let i = t.thread_index();\n    tile[i] = src[i];\n    dst[0] = tile[1];\n}}\nfn main() {{}}"
        );
        assert_eq!(check(&src), vec![3009]);
    }

    //@ spec: RXS-0079
    #[test]
    fn write_then_self_lane_read_is_clean() {
        // 同一下标 `i` 的写后自读:数据流内可见,无须 barrier(RXS-0079)。
        let src = format!(
            "{HEAD}    shared let tile: [f32; 256];\n    let i = t.thread_index();\n    tile[i] = src[i];\n    dst[i] = tile[i];\n}}\nfn main() {{}}"
        );
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0079
    #[test]
    fn unsynced_cross_lane_read_in_unsafe_is_exempt() {
        let src = format!(
            "{HEAD}    shared let tile: [f32; 256];\n    let i = t.thread_index();\n    tile[i] = src[i];\n    unsafe {{\n        dst[0] = tile[1];\n    }}\n}}\nfn main() {{}}"
        );
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0079
    #[test]
    fn host_context_shared_not_checked() {
        // host fn 非 device 上下文:shared+barrier 扩展 pass 不实施(消费着色)。
        let src = "fn h(i: usize) {\n    shared let tile: [f32; 256];\n    tile[i] = 1.0;\n    let _ = tile[1];\n}\nfn main() {}";
        assert!(check(src).is_empty(), "{:?}", check(src));
    }
}
