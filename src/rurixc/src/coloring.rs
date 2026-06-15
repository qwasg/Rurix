//! 函数着色与 barrier uniform 可达性检查(spec 条款 RXS-0066/RXS-0068/RXS-0069,
//! spec/device.md;07 §3 着色检查在 HIR 层,无需数据流;3xxx 着色/地址空间首批)。
//!
//! - **跨着色调用合法性**(RXS-0066,`RX3001`):调用点所在 body 的着色(上下文
//!   着色)决定其可调用集——device 上下文(device/kernel 体)调用 host 着色函数、
//!   或任意上下文直接调用 `kernel fn` 即违例。调用目标取自 typeck `call_targets`
//!   (调用点 HirId → 目标 DefId);目标着色不可判定(内建/构造器/非函数项)不触发
//!   (RXS-0069 不级联)。
//! - **barrier uniform 可达性保守骨架**(RXS-0068,`RX3003`):device 上下文内真实
//!   block barrier(typeck 记录的 `DeviceIntrinsic::Barrier`,非按 `.sync()` 方法名
//!   猜测)出现在依赖 thread id 的分支内且未置于 `unsafe` 即违例。thread-id 依赖经
//!   **taint 传播**判定:分支条件直接含线程索引 device intrinsic,或含一个其值经
//!   `let` 绑定从线程索引传播而来的局部(`let i = t.thread_index(); if i < 10 {...}`)。
//!   完整 uniform 控制流分析随后续(07 §4)。
//!
//! 地址空间一致性(RXS-0067,`RX3002`)在 typeck 合一处裁决(见 [`crate::typeck`]),
//! 不在本模块。

use std::collections::HashSet;

use crate::ast::FnColor;
use crate::diag::{DiagCtxt, ErrorCode};
use crate::hir::{
    self, BodyId, Crate, DefId, DeviceIntrinsic, Expr, ExprKind, LocalId, PatKind, Res,
};
use crate::query::QueryCtx;
use crate::typeck::TypeckResults;

pub const E_CROSS_COLOR_CALL: ErrorCode = ErrorCode(3001); // RX3001(RXS-0066)
pub const E_BARRIER_NON_UNIFORM: ErrorCode = ErrorCode(3003); // RX3003(RXS-0068)
pub const E_DEVICE_MATH_UNSUPPORTED: ErrorCode = ErrorCode(6006); // RX6006(RXS-0081)

/// 线程索引类 device intrinsic(thread/block 索引、global id):其值随线程发散,
/// 是 barrier uniform 可达性判定的 thread-id 源(RXS-0068)。判定来自 typeck 记录的
/// 真实 `device_calls`(`ThreadCtx`/`block` 方法识别),而非方法名字符串猜测——用户
/// 自定义同名方法不会被误判为线程索引/barrier。block_dim(block 内 uniform)与
/// barrier 自身不计入 thread-id 源。
fn is_thread_id_intrinsic(intr: DeviceIntrinsic) -> bool {
    matches!(
        intr,
        DeviceIntrinsic::ThreadIndexX
            | DeviceIntrinsic::ThreadIndexY
            | DeviceIntrinsic::ThreadIndexZ
            | DeviceIntrinsic::BlockIndexX
            | DeviceIntrinsic::BlockIndexY
            | DeviceIntrinsic::BlockIndexZ
            | DeviceIntrinsic::GlobalIdX
            | DeviceIntrinsic::GlobalIdY
            | DeviceIntrinsic::GlobalIdZ
    )
}

/// 全 crate 着色 + barrier 骨架检查入口(provider:[`QueryCtx::check_coloring`])。
pub fn check_crate(cx: &QueryCtx<'_>) {
    let krate = cx.hir_crate();
    for i in 0..krate.bodies.len() {
        let body_id = BodyId(i as u32);
        let tcr = cx.check_body(body_id);
        let body = krate.body(body_id);
        let ctx = context_color(&krate, body.owner);
        let mut walker = Walker {
            diag: cx.diag(),
            krate: &krate,
            tcr: &tcr,
            ctx,
            tid_taint: HashSet::new(),
        };
        walker.walk_expr(&body.value, false, false);
    }
}

/// body 的上下文着色:fn 取其着色;const/static 初始化器视为 host 上下文。
fn context_color(krate: &Crate, owner: DefId) -> FnColor {
    match &krate.item(owner).kind {
        hir::ItemKind::Fn(decl) => decl.color,
        _ => FnColor::Host,
    }
}

fn is_device_ctx(c: FnColor) -> bool {
    matches!(c, FnColor::Device | FnColor::Kernel)
}

struct Walker<'a> {
    diag: &'a DiagCtxt,
    krate: &'a Crate,
    tcr: &'a TypeckResults,
    ctx: FnColor,
    /// thread-id 污染的局部(其值经 `let` 从线程索引 intrinsic 传播而来,RXS-0068)。
    tid_taint: HashSet<LocalId>,
}

impl Walker<'_> {
    /// 被调函数的着色(仅 fn item 有色;构造器/内建/非函数项返回 None)。
    fn callee_color(&self, def: DefId) -> Option<FnColor> {
        match &self.krate.item(def).kind {
            hir::ItemKind::Fn(decl) => Some(decl.color),
            _ => None,
        }
    }

    /// 跨着色调用合法性裁决(RXS-0066)。
    fn check_call_target(&self, call_id: hir::HirId, span: crate::span::Span) {
        let Some((def, _)) = self.tcr.call_targets.get(&call_id) else {
            return;
        };
        let Some(callee) = self.callee_color(*def) else {
            return; // 目标着色不可判定:不级联(RXS-0069)
        };
        let illegal = match callee {
            // kernel fn 不可直接调用(任何上下文须经 launch,RXS-0066)
            FnColor::Kernel => true,
            // host-only 函数在 device 上下文不可达(RXS-0066)
            FnColor::Host => is_device_ctx(self.ctx),
            FnColor::Device | FnColor::Const => false,
        };
        if illegal {
            let callee_desc = match callee {
                FnColor::Kernel => "a `kernel` function",
                FnColor::Host => "a host function",
                _ => unreachable!(),
            };
            let ctx_desc = if is_device_ctx(self.ctx) {
                "device"
            } else {
                "host"
            };
            self.diag
                .struct_error(E_CROSS_COLOR_CALL, "coloring.cross_color_call")
                .arg("callee", callee_desc)
                .arg("context", ctx_desc)
                .span_label(span, format!("cannot be called from a {ctx_desc} context"))
                .emit();
        }
    }

    /// barrier 骨架违例(RXS-0068):device 上下文 + thread-id 依赖分支 + 非 unsafe。
    fn check_barrier(&self, span: crate::span::Span, in_tid_branch: bool, in_unsafe: bool) {
        if is_device_ctx(self.ctx) && in_tid_branch && !in_unsafe {
            self.diag
                .struct_error(E_BARRIER_NON_UNIFORM, "coloring.barrier_non_uniform")
                .span_label(span, "barrier inside a thread-id-dependent branch")
                .emit();
        }
    }

    fn walk_expr(&mut self, e: &Expr, in_tid_branch: bool, in_unsafe: bool) {
        match &e.kind {
            ExprKind::Call { callee, args } => {
                self.check_call_target(e.hir_id, e.span);
                self.walk_expr(callee, in_tid_branch, in_unsafe);
                for a in args {
                    self.walk_expr(a, in_tid_branch, in_unsafe);
                }
            }
            ExprKind::MethodCall { receiver, args, .. } => {
                self.check_call_target(e.hir_id, e.span);
                if self.tcr.device_math_calls.contains_key(&e.hir_id) && !is_device_ctx(self.ctx) {
                    self.diag
                        .struct_error(E_DEVICE_MATH_UNSUPPORTED, "codegen.device_math_unsupported")
                        .arg(
                            "detail",
                            "device math intrinsics require device or kernel context (RXS-0081)",
                        )
                        .span_label(e.span, "unsupported device math intrinsic")
                        .emit();
                }
                // 真实 barrier 判定:typeck 记录的 DeviceIntrinsic::Barrier
                // (ThreadCtx/block 的 sync()),非按方法名猜测——用户自定义 sync()
                // 不会被识别为 barrier(无 device_calls 记录)。
                if matches!(
                    self.tcr.device_calls.get(&e.hir_id),
                    Some(DeviceIntrinsic::Barrier)
                ) {
                    self.check_barrier(e.span, in_tid_branch, in_unsafe);
                }
                self.walk_expr(receiver, in_tid_branch, in_unsafe);
                for a in args {
                    self.walk_expr(a, in_tid_branch, in_unsafe);
                }
            }
            ExprKind::If { cond, then, else_ } => {
                self.walk_expr(cond, in_tid_branch, in_unsafe);
                // 分支内 thread-id 依赖:本分支条件含线程索引(或其 taint 传播局部),
                // 或已处于此类分支内
                let branch_tid = in_tid_branch || self.mentions_tid(cond);
                self.walk_block(then, branch_tid, in_unsafe);
                if let Some(eb) = else_ {
                    self.walk_expr(eb, branch_tid, in_unsafe);
                }
            }
            ExprKind::While { cond, body } => {
                self.walk_expr(cond, in_tid_branch, in_unsafe);
                let branch_tid = in_tid_branch || self.mentions_tid(cond);
                self.walk_block(body, branch_tid, in_unsafe);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee, in_tid_branch, in_unsafe);
                let branch_tid = in_tid_branch || self.mentions_tid(scrutinee);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        self.walk_expr(g, branch_tid, in_unsafe);
                    }
                    self.walk_expr(&arm.body, branch_tid, in_unsafe);
                }
            }
            ExprKind::Loop { body } => self.walk_block(body, in_tid_branch, in_unsafe),
            ExprKind::Block(b) => self.walk_block(b, in_tid_branch, in_unsafe),
            ExprKind::Unsafe(b) => self.walk_block(b, in_tid_branch, true),
            ExprKind::Unary { expr, .. }
            | ExprKind::Borrow { expr, .. }
            | ExprKind::Cast { expr, .. }
            | ExprKind::Field { expr, .. }
            | ExprKind::TupleField { expr, .. } => self.walk_expr(expr, in_tid_branch, in_unsafe),
            ExprKind::Binary { lhs, rhs, .. }
            | ExprKind::Assign { lhs, rhs, .. }
            | ExprKind::Range {
                lo: lhs, hi: rhs, ..
            }
            | ExprKind::Index {
                expr: lhs,
                index: rhs,
            } => {
                self.walk_expr(lhs, in_tid_branch, in_unsafe);
                self.walk_expr(rhs, in_tid_branch, in_unsafe);
            }
            ExprKind::Tuple(v) | ExprKind::Array(v) => {
                for x in v {
                    self.walk_expr(x, in_tid_branch, in_unsafe);
                }
            }
            ExprKind::Repeat { elem, len } => {
                self.walk_expr(elem, in_tid_branch, in_unsafe);
                self.walk_expr(len, in_tid_branch, in_unsafe);
            }
            ExprKind::StructLit { fields, .. } => {
                for (_, v) in fields {
                    if let Some(x) = v {
                        self.walk_expr(x, in_tid_branch, in_unsafe);
                    }
                }
            }
            ExprKind::Return(op) | ExprKind::Break(op) => {
                if let Some(x) = op {
                    self.walk_expr(x, in_tid_branch, in_unsafe);
                }
            }
            ExprKind::Closure { body, .. } => self.walk_expr(body, in_tid_branch, in_unsafe),
            ExprKind::Lit(_)
            | ExprKind::SynthInt(_)
            | ExprKind::Res(_)
            | ExprKind::Continue
            | ExprKind::Err => {}
        }
    }

    fn walk_block(&mut self, b: &hir::Block, in_tid_branch: bool, in_unsafe: bool) {
        for s in &b.stmts {
            match s {
                hir::Stmt::Item(_) => {} // 嵌套 item 的 body 经 check_crate 全集遍历
                hir::Stmt::Let { pat, init, .. } => {
                    if let Some(e) = init {
                        self.walk_expr(e, in_tid_branch, in_unsafe);
                        // thread-id taint 传播:`let i = <含线程索引的表达式>` → 污染 i,
                        // 使后续 `if i < 10 { t.sync(); }` 间接判定为 thread-id 依赖分支。
                        if let PatKind::Binding { local } = pat.kind
                            && self.mentions_tid(e)
                        {
                            self.tid_taint.insert(local);
                        }
                    }
                }
                hir::Stmt::Expr(e) => self.walk_expr(e, in_tid_branch, in_unsafe),
            }
        }
        if let Some(t) = &b.tail {
            self.walk_expr(t, in_tid_branch, in_unsafe);
        }
    }

    /// 表达式(子树)是否依赖 thread id(RXS-0068 保守 thread-id 判定):含线程索引
    /// device intrinsic(typeck `device_calls`),或引用一个 thread-id taint 传播的局部。
    fn mentions_tid(&self, e: &Expr) -> bool {
        let mut found = false;
        self.walk_for_tid(e, &mut found);
        found
    }

    fn walk_for_tid(&self, e: &Expr, found: &mut bool) {
        if *found {
            return;
        }
        match &e.kind {
            // taint 传播的局部(`let i = t.thread_index()` 后的 `i`)。
            ExprKind::Res(Res::Local(l)) => {
                if self.tid_taint.contains(l) {
                    *found = true;
                }
            }
            ExprKind::MethodCall { receiver, args, .. } => {
                // 真实线程索引 intrinsic 判定来自 typeck device_calls,非方法名猜测。
                if let Some(intr) = self.tcr.device_calls.get(&e.hir_id)
                    && is_thread_id_intrinsic(*intr)
                {
                    *found = true;
                    return;
                }
                self.walk_for_tid(receiver, found);
                for a in args {
                    self.walk_for_tid(a, found);
                }
            }
            ExprKind::Call { callee, args } => {
                self.walk_for_tid(callee, found);
                for a in args {
                    self.walk_for_tid(a, found);
                }
            }
            ExprKind::Unary { expr, .. }
            | ExprKind::Borrow { expr, .. }
            | ExprKind::Cast { expr, .. }
            | ExprKind::Field { expr, .. }
            | ExprKind::TupleField { expr, .. } => self.walk_for_tid(expr, found),
            ExprKind::Binary { lhs, rhs, .. }
            | ExprKind::Assign { lhs, rhs, .. }
            | ExprKind::Range {
                lo: lhs, hi: rhs, ..
            }
            | ExprKind::Index {
                expr: lhs,
                index: rhs,
            } => {
                self.walk_for_tid(lhs, found);
                self.walk_for_tid(rhs, found);
            }
            ExprKind::Tuple(v) | ExprKind::Array(v) => {
                for x in v {
                    self.walk_for_tid(x, found);
                }
            }
            ExprKind::Repeat { elem, len } => {
                self.walk_for_tid(elem, found);
                self.walk_for_tid(len, found);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// 跑 typeck + 着色检查,返回着色/barrier 诊断码序列。
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
        diag.emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect()
    }

    //@ spec: RXS-0066, RXS-0069
    #[test]
    fn device_context_calling_host_only_is_rx3001() {
        let codes = check("fn host_only() {}\ndevice fn d() {\n    host_only();\n}\nfn main() {}");
        assert_eq!(codes, vec![3001]);
    }

    //@ spec: RXS-0066
    #[test]
    fn kernel_context_calling_host_only_is_rx3001() {
        let codes = check("fn host_only() {}\nkernel fn k() {\n    host_only();\n}\nfn main() {}");
        assert_eq!(codes, vec![3001]);
    }

    //@ spec: RXS-0066
    #[test]
    fn direct_kernel_call_is_rx3001() {
        let codes = check("kernel fn k() {}\nfn main() {\n    k();\n}");
        assert_eq!(codes, vec![3001]);
    }

    //@ spec: RXS-0066
    #[test]
    fn host_calling_device_is_clean() {
        // 单向可达:device ⊂ host 可调用集(RXS-0066)
        let codes = check("device fn d() {}\nfn main() {\n    d();\n}");
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0066
    #[test]
    fn device_calling_device_is_clean() {
        let codes = check("device fn a() {}\ndevice fn b() {\n    a();\n}\nfn main() {}");
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0066
    #[test]
    fn kernel_calling_device_is_clean() {
        let codes = check("device fn d() {}\nkernel fn k() {\n    d();\n}\nfn main() {}");
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0068
    #[test]
    fn barrier_in_tid_branch_is_rx3003() {
        let codes = check(
            "device fn d() {}\nkernel fn k(t: ThreadCtx<1>) {\n    if t.global_id() < 10 {\n        t.sync();\n    }\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![3003]);
    }

    //@ spec: RXS-0068
    #[test]
    fn uniform_barrier_is_clean() {
        let codes = check("kernel fn k(t: ThreadCtx<1>) {\n    t.sync();\n}\nfn main() {}");
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0068
    #[test]
    fn barrier_in_unsafe_tid_branch_is_exempt() {
        let codes = check(
            "kernel fn k(t: ThreadCtx<1>) {\n    if t.global_id() < 10 {\n        unsafe {\n            t.sync();\n        }\n    }\n}\nfn main() {}",
        );
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0066
    #[test]
    fn host_calling_host_is_clean() {
        let codes = check("fn a() {}\nfn b() {\n    a();\n}\nfn main() {}");
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0068
    #[test]
    fn barrier_in_indirect_tid_branch_is_rx3003() {
        // thread-id 经 let 局部传播后的分支条件:barrier 仍判为非 uniform(taint 传播)
        let codes = check(
            "kernel fn k(t: ThreadCtx<1>) {\n    let i = t.thread_index();\n    if i < 10 {\n        t.sync();\n    }\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![3003]);
    }

    //@ spec: RXS-0068
    #[test]
    fn barrier_in_transitively_tainted_branch_is_rx3003() {
        // 多跳 taint 传播:i ← thread_index,j ← i + 1,分支用 j
        let codes = check(
            "kernel fn k(t: ThreadCtx<1>) {\n    let i = t.thread_index();\n    let j = i + 1;\n    if j < 10 {\n        t.sync();\n    }\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![3003]);
    }

    //@ spec: RXS-0068
    #[test]
    fn user_defined_sync_is_not_barrier() {
        // 用户自定义 sync()(非 ThreadCtx/block 的真实 barrier)在 tid 分支内不报 RX3003:
        // barrier 判定来自 typeck device_calls,而非方法名字符串匹配。
        let codes = check(
            "struct Gate {}\nimpl Gate {\n    device fn sync(&self) {}\n}\nkernel fn k(t: ThreadCtx<1>, g: Gate) {\n    if t.global_id() < 10 {\n        g.sync();\n    }\n}\nfn main() {}",
        );
        assert!(!codes.contains(&3003), "{codes:?}");
    }
}
