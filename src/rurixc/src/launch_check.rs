//! launch 类型契约检查(spec 条款 RXS-0074/RXS-0075,spec/device.md;07 §3
//! HIR 层,typeck 后、MIR 前,无数据流)。
//!
//! 识别 `Stream` 接收者的 `launch(kernel, GridDim(..), BlockDim(..), (args..))`
//! 调用(typeck 在该形态上容忍,见 [`crate::typeck`]),按契约四类裁决:
//!
//! - **着色契约**(`RX3004`):`launch` 的 kernel 引用必须解析到 `kernel` 着色
//!   函数;对 host/device/const 着色函数发起 launch 即违例。
//! - **维度契约**(`RX3005`):`GridDim` 与 `BlockDim` 维数(实参个数)须一致。
//! - **参数契约**(`RX2001` 复用 / `RX3002` 复用):参数元组各元素与 `kernel fn`
//!   形参(剔除 `ThreadCtx` 句柄)对应;host `Buffer<Ctx, T>` 满足 device
//!   `View<space, T>`/`ViewMut<space, T>` 形参当且仅当元素类型 `T` 可合一。
//! - **context-brand 契约**(`RX3006`):携带 brand 的资源实参(`Buffer<Ctx, T>`)
//!   的 brand 须与发起 launch 的 `Stream<Ctx>` 的 brand 一致。
//!
//! 不级联(RXS-0075):launch 形态不完整 / 接收者非 `Stream` / 参与类型为 `Err`
//! 时不触发;每个 launch 调用按优先序(着色 → 维度 → 参数/brand)报首个违例。

use crate::ast::FnColor;
use crate::diag::{DiagCtxt, ErrorCode};
use crate::hir::{self, BodyId, Crate, DefId, Expr, ExprKind, Res};
use crate::query::QueryCtx;
use crate::resolve::Resolutions;
use crate::ty::Ty;
use crate::typeck::TypeckResults;

pub const E_LAUNCH_NON_KERNEL: ErrorCode = ErrorCode(3004); // RX3004(RXS-0074)
pub const E_LAUNCH_DIM_MISMATCH: ErrorCode = ErrorCode(3005); // RX3005(RXS-0074)
pub const E_LAUNCH_CONTEXT_BRAND: ErrorCode = ErrorCode(3006); // RX3006(RXS-0074)
pub const E_MISMATCHED_TYPES: ErrorCode = ErrorCode(2001); // RX2001(复用,RXS-0074 参数契约)

const LAUNCH_METHOD: &str = "launch";

/// 全 crate launch 类型契约检查入口(provider:[`QueryCtx::check_launch`])。
pub fn check_crate(cx: &QueryCtx<'_>) {
    let krate = cx.hir_crate();
    let res = cx.resolutions();
    for i in 0..krate.bodies.len() {
        let body_id = BodyId(i as u32);
        let tcr = cx.check_body(body_id);
        let body = krate.body(body_id);
        let walker = Walker {
            cx,
            krate: &krate,
            res: &res,
            tcr: &tcr,
        };
        walker.walk_expr(&body.value);
    }
}

struct Walker<'a, 'q> {
    cx: &'a QueryCtx<'q>,
    krate: &'a Crate,
    res: &'a Resolutions,
    tcr: &'a TypeckResults,
}

impl Walker<'_, '_> {
    fn diag(&self) -> &DiagCtxt {
        self.cx.diag()
    }

    /// 接收者类型解一层引用后的基类型(RXS-0074;`Stream` 识别)。
    fn receiver_base(&self, receiver: &Expr) -> Option<Ty> {
        let t = self.tcr.expr_ty.get(&receiver.hir_id)?;
        Some(match t {
            Ty::Ref(inner, _) => (**inner).clone(),
            other => other.clone(),
        })
    }

    /// launch 调用裁决(按优先序报首个违例,RXS-0075 不级联)。
    fn check_launch_call(&self, receiver: &Expr, args: &[Expr]) {
        let Some(base) = self.receiver_base(receiver) else {
            return;
        };
        let Ty::Adt(d, brand_args) = &base else {
            return;
        };
        if !self.res.lang_items.is_stream(*d) {
            return;
        }
        // 形态:launch(kernel, GridDim, BlockDim, (args..));不完整即容忍
        if args.len() != 4 {
            return;
        }
        let kernel_ref = &args[0];
        let grid = &args[1];
        let block = &args[2];
        let arg_tuple = &args[3];

        // 1. 着色契约(RX3004):kernel 引用须为 `kernel` 着色函数
        let Some(kernel_def) = self.fn_ref(kernel_ref) else {
            return; // 引用不可判定(非函数项 / Err):不级联
        };
        let color = self.fn_color(kernel_def);
        if color != Some(FnColor::Kernel) {
            let callee = match color {
                Some(FnColor::Host) => "a host function",
                Some(FnColor::Device) => "a `device` function",
                Some(FnColor::Const) => "a `const` function",
                _ => "this function",
            };
            self.diag()
                .struct_error(E_LAUNCH_NON_KERNEL, "launch.non_kernel")
                .arg("callee", callee)
                .span_label(kernel_ref.span, "only a `kernel` function can be launched")
                .emit();
            return;
        }

        // 2. 维度契约(RX3005):GridDim 与 BlockDim 维数须一致
        if let (Some(g), Some(b)) = (self.grid_arity(grid), self.block_arity(block))
            && g != b
        {
            self.diag()
                .struct_error(E_LAUNCH_DIM_MISMATCH, "launch.dim_mismatch")
                .arg("grid", g.to_string())
                .arg("block", b.to_string())
                .span_label(block.span, "block dimensions disagree with grid")
                .emit();
            return;
        }

        // 3/4. 参数契约 + context-brand 契约
        let ExprKind::Tuple(elems) = &arg_tuple.kind else {
            return;
        };
        let stream_brand = brand_args.first().cloned();
        let sig = self.cx.fn_sig(kernel_def);
        // kernel 形参剔除 `ThreadCtx` 句柄形参(RXS-0074 参数契约)
        let params: Vec<Ty> = sig
            .inputs
            .iter()
            .filter(|t| !self.is_thread_ctx_ty(t))
            .cloned()
            .collect();
        for (param, elem) in params.iter().zip(elems.iter()) {
            let Some(arg_ty) = self.tcr.expr_ty.get(&elem.hir_id) else {
                continue;
            };
            if self.check_arg(param, arg_ty, stream_brand.as_ref(), elem) {
                return; // 报首个违例(RXS-0075)
            }
        }
    }

    /// 单个 launch 实参裁决;返回 true 表示已报违例(调用方据此停止)。
    fn check_arg(&self, param: &Ty, arg_ty: &Ty, stream_brand: Option<&Ty>, elem: &Expr) -> bool {
        // host `Buffer<Ctx, T>` 满足 device `View<space, T>`/`ViewMut<space, T>`
        // 形参:元素类型 T 可合一(Buffer 提供 view);brand 与 Stream 一致。
        if let (Ty::Adt(pd, pargs), Ty::Adt(ad, aargs)) = (param, arg_ty)
            && self.res.lang_items.view_mutable(*pd).is_some()
            && self.res.lang_items.is_buffer(*ad)
        {
            let param_elem = pargs.get(1);
            let arg_elem = aargs.get(1);
            if let (Some(pe), Some(ae)) = (param_elem, arg_elem)
                && !ty_compat(pe, ae)
            {
                self.emit_mismatch(elem.span, param, arg_ty);
                return true;
            }
            if let (Some(sb), Some(ab)) = (stream_brand, aargs.first())
                && !ty_compat(sb, ab)
            {
                self.diag()
                    .struct_error(E_LAUNCH_CONTEXT_BRAND, "launch.context_brand")
                    .arg("what", "this launch argument")
                    .span_label(elem.span, "belongs to a different context")
                    .emit();
                return true;
            }
            return false;
        }
        // 标量 / 同形态实参:类型须合一(RXS-0074 参数契约)
        if !ty_compat(param, arg_ty) {
            self.emit_mismatch(elem.span, param, arg_ty);
            return true;
        }
        false
    }

    fn emit_mismatch(&self, span: crate::span::Span, expected: &Ty, found: &Ty) {
        self.diag()
            .struct_error(E_MISMATCHED_TYPES, "typeck.mismatched_types")
            .arg("expected", expected.render(self.res))
            .arg("found", found.render(self.res))
            .span_label(span, format!("expected {}", expected.render(self.res)))
            .emit();
    }

    /// 表达式是否为 fn item 的值引用(launch kernel 引用判定)。
    fn fn_ref(&self, e: &Expr) -> Option<DefId> {
        match &e.kind {
            ExprKind::Res(Res::Def(d)) => {
                matches!(self.krate.item(*d).kind, hir::ItemKind::Fn(_)).then_some(*d)
            }
            _ => None,
        }
    }

    fn fn_color(&self, def: DefId) -> Option<FnColor> {
        match &self.krate.item(def).kind {
            hir::ItemKind::Fn(decl) => Some(decl.color),
            _ => None,
        }
    }

    fn is_thread_ctx_ty(&self, t: &Ty) -> bool {
        matches!(t, Ty::Adt(d, _) if self.res.lang_items.is_thread_ctx(*d))
    }

    /// `GridDim(..)` 构造调用的维数(实参个数);非该构造 → None。
    fn grid_arity(&self, e: &Expr) -> Option<usize> {
        self.dim_arity(e, true)
    }

    fn block_arity(&self, e: &Expr) -> Option<usize> {
        self.dim_arity(e, false)
    }

    fn dim_arity(&self, e: &Expr, grid: bool) -> Option<usize> {
        let ExprKind::Call { callee, args } = &e.kind else {
            return None;
        };
        let ExprKind::Res(Res::Def(d)) = &callee.kind else {
            return None;
        };
        let ok = if grid {
            self.res.lang_items.is_grid_dim(*d)
        } else {
            self.res.lang_items.is_block_dim(*d)
        };
        ok.then_some(args.len())
    }

    fn walk_expr(&self, e: &Expr) {
        match &e.kind {
            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                if method == LAUNCH_METHOD {
                    self.check_launch_call(receiver, args);
                }
                self.walk_expr(receiver);
                for a in args {
                    self.walk_expr(a);
                }
            }
            ExprKind::Call { callee, args } => {
                self.walk_expr(callee);
                for a in args {
                    self.walk_expr(a);
                }
            }
            ExprKind::Unary { expr, .. }
            | ExprKind::Borrow { expr, .. }
            | ExprKind::Cast { expr, .. }
            | ExprKind::Field { expr, .. }
            | ExprKind::TupleField { expr, .. } => self.walk_expr(expr),
            ExprKind::Binary { lhs, rhs, .. }
            | ExprKind::Assign { lhs, rhs, .. }
            | ExprKind::Range {
                lo: lhs, hi: rhs, ..
            }
            | ExprKind::Index {
                expr: lhs,
                index: rhs,
            } => {
                self.walk_expr(lhs);
                self.walk_expr(rhs);
            }
            ExprKind::Tuple(v) | ExprKind::Array(v) => {
                for x in v {
                    self.walk_expr(x);
                }
            }
            ExprKind::Repeat { elem, len } => {
                self.walk_expr(elem);
                self.walk_expr(len);
            }
            ExprKind::StructLit { fields, .. } => {
                for (_, v) in fields {
                    if let Some(x) = v {
                        self.walk_expr(x);
                    }
                }
            }
            ExprKind::Block(b) | ExprKind::Unsafe(b) => self.walk_block(b),
            ExprKind::If { cond, then, else_ } => {
                self.walk_expr(cond);
                self.walk_block(then);
                if let Some(eb) = else_ {
                    self.walk_expr(eb);
                }
            }
            ExprKind::While { cond, body } => {
                self.walk_expr(cond);
                self.walk_block(body);
            }
            ExprKind::Loop { body } => self.walk_block(body),
            ExprKind::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        self.walk_expr(g);
                    }
                    self.walk_expr(&arm.body);
                }
            }
            ExprKind::Return(op) | ExprKind::Break(op) => {
                if let Some(x) = op {
                    self.walk_expr(x);
                }
            }
            ExprKind::Closure { body, .. } => self.walk_expr(body),
            ExprKind::Lit(_)
            | ExprKind::SynthInt(_)
            | ExprKind::Res(_)
            | ExprKind::Continue
            | ExprKind::Err => {}
        }
    }

    fn walk_block(&self, b: &hir::Block) {
        for s in &b.stmts {
            match s {
                hir::Stmt::Item(_) => {}
                hir::Stmt::Let { init, .. } => {
                    if let Some(e) = init {
                        self.walk_expr(e);
                    }
                }
                hir::Stmt::Expr(e) => self.walk_expr(e),
            }
        }
        if let Some(t) = &b.tail {
            self.walk_expr(t);
        }
    }
}

/// 类型相容(RXS-0074 参数/brand 契约):`Err` 容忍区任一侧 → 相容(不级联,
/// RXS-0075);其余按结构相等(`Param` 同序号、`Adt` 同 DefId + 实参逐一相容)。
fn ty_compat(a: &Ty, b: &Ty) -> bool {
    match (a, b) {
        (Ty::Err, _) | (_, Ty::Err) => true,
        (Ty::Prim(p), Ty::Prim(q)) => p == q,
        (Ty::Param(i), Ty::Param(j)) => i == j,
        (Ty::Adt(d, xs), Ty::Adt(e, ys)) => {
            d == e && xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| ty_compat(x, y))
        }
        (Ty::Tuple(xs), Ty::Tuple(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| ty_compat(x, y))
        }
        (Ty::Ref(x, m), Ty::Ref(y, n)) | (Ty::RawPtr(x, m), Ty::RawPtr(y, n)) => {
            m == n && ty_compat(x, y)
        }
        (Ty::Array(x), Ty::Array(y)) | (Ty::Slice(x), Ty::Slice(y)) => ty_compat(x, y),
        (Ty::FnPtr(xs, xr), Ty::FnPtr(ys, yr)) => {
            xs.len() == ys.len()
                && xs.iter().zip(ys).all(|(x, y)| ty_compat(x, y))
                && ty_compat(xr, yr)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// 跑 typeck + 着色 + launch 检查,返回 launch 诊断码序列。
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
        cx.check_launch();
        diag.emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect()
    }

    const KERNEL: &str = "kernel fn saxpy(out: ViewMut<global, f32>, x: View<global, f32>, n: usize, t: ThreadCtx<1>) {\n    let i = t.global_id();\n    if i < n {\n        out[i] = x[i];\n    }\n}\n";

    //@ spec: RXS-0074
    #[test]
    fn valid_launch_is_clean() {
        let src = format!(
            "{KERNEL}fn run<C>(s: Stream<C>, out: Buffer<C, f32>, x: Buffer<C, f32>, n: usize) {{\n    s.launch(saxpy, GridDim(n), BlockDim(n), (out, x, n));\n}}\nfn main() {{}}"
        );
        let codes = check(&src);
        assert!(codes.is_empty(), "{codes:?}");
    }

    //@ spec: RXS-0074, RXS-0075
    #[test]
    fn launch_non_kernel_is_rx3004() {
        let src = "device fn helper(out: ViewMut<global, f32>, x: View<global, f32>, n: usize) {}\nfn run<C>(s: Stream<C>, out: Buffer<C, f32>, x: Buffer<C, f32>, n: usize) {\n    s.launch(helper, GridDim(n), BlockDim(n), (out, x, n));\n}\nfn main() {}";
        assert_eq!(check(src), vec![3004]);
    }

    //@ spec: RXS-0074, RXS-0075
    #[test]
    fn launch_dim_mismatch_is_rx3005() {
        let src = format!(
            "{KERNEL}fn run<C>(s: Stream<C>, out: Buffer<C, f32>, x: Buffer<C, f32>, n: usize) {{\n    s.launch(saxpy, GridDim(n, n), BlockDim(n), (out, x, n));\n}}\nfn main() {{}}"
        );
        assert_eq!(check(&src), vec![3005]);
    }

    //@ spec: RXS-0074, RXS-0075
    #[test]
    fn launch_arg_type_mismatch_is_rx2001() {
        // out 元素类型 i32 与 kernel 形参 ViewMut<global, f32> 不符
        let src = format!(
            "{KERNEL}fn run<C>(s: Stream<C>, out: Buffer<C, i32>, x: Buffer<C, f32>, n: usize) {{\n    s.launch(saxpy, GridDim(n), BlockDim(n), (out, x, n));\n}}\nfn main() {{}}"
        );
        assert_eq!(check(&src), vec![2001]);
    }

    //@ spec: RXS-0074, RXS-0075
    #[test]
    fn launch_context_brand_mismatch_is_rx3006() {
        // bad 的 brand D 与 Stream<C> 的 brand C 不一致
        let src = format!(
            "{KERNEL}fn run<C, D>(s: Stream<C>, bad: Buffer<D, f32>, x: Buffer<C, f32>, n: usize) {{\n    s.launch(saxpy, GridDim(n), BlockDim(n), (bad, x, n));\n}}\nfn main() {{}}"
        );
        assert_eq!(check(&src), vec![3006]);
    }

    //@ spec: RXS-0075
    #[test]
    fn non_launch_method_is_ignored() {
        // 接收者非 Stream:不触发 launch 检查(不级联)
        let src = "struct S {}\nimpl S {\n    fn launch(&self) {}\n}\nfn run(s: S) {\n    s.launch();\n}\nfn main() {}";
        assert!(check(src).is_empty());
    }
}
