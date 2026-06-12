//! HIR(typed)→ TBIR 构造(D-202 窄门;M3.1)。
//!
//! 显式化动作(见 [`crate::tbir`] 模块注释):方法糖 → 显式直调(receiver
//! 按 `self` 形态 autoref/autoderef,RXS-0046)、字段名/构造字段 → 定义序
//! 下标、struct 字面量按定义序重排、单元变体值 → 空构造、drop scope 树
//! (RXS-0052)。模式穷尽性检查(RXS-0051)挂在本阶段(match 节点构造处)。
//!
//! 输入要求:本阶段假定 typeck 无错误(管线阶段化中止);容忍区残留
//! (`Ty::Err` / 未记录调用点)落为 [`tbir::ExprKind::Err`],由 MIR 报
//! RX6001 作用面诊断。

use crate::ast::UnOp;
use crate::hir::{self, DefId, LocalId, Res};
use crate::resolve::Resolutions;
use crate::tbir::{self, ScopeId};
use crate::ty::Ty;
use crate::typeck::TypeckResults;

/// TBIR 构造入口(逐 body;产物即建即用,MIR 构造后释放,D-202)。
pub fn build(
    krate: &hir::Crate,
    res: &Resolutions,
    tcr: &TypeckResults,
    body: &hir::Body,
) -> tbir::Body {
    let mut b = Builder {
        krate,
        res,
        tcr,
        scopes: vec![tbir::Scope {
            parent: None,
            span: body.value.span,
        }],
        cur_scope: ScopeId(0),
        locals: body
            .locals
            .iter()
            .enumerate()
            .map(|(i, l)| tbir::LocalDecl {
                name: l.name.clone(),
                mutable: l.mutable,
                ty: tcr.local_ty.get(i).cloned().unwrap_or(Ty::Err),
                span: l.span,
                scope: ScopeId(0),
            })
            .collect(),
    };
    let params: Vec<tbir::Pat> = body.params.iter().map(|p| b.pat(p)).collect();
    let value = b.expr(&body.value);
    tbir::Body {
        owner: body.owner,
        locals: b.locals,
        params,
        value,
        scopes: b.scopes,
    }
}

struct Builder<'a> {
    krate: &'a hir::Crate,
    res: &'a Resolutions,
    tcr: &'a TypeckResults,
    scopes: Vec<tbir::Scope>,
    cur_scope: ScopeId,
    locals: Vec<tbir::LocalDecl>,
}

impl Builder<'_> {
    fn expr_ty(&self, e: &hir::Expr) -> Ty {
        self.tcr.expr_ty.get(&e.hir_id).cloned().unwrap_or(Ty::Err)
    }

    fn pat_ty(&self, p: &hir::Pat) -> Ty {
        self.tcr.pat_ty.get(&p.hir_id).cloned().unwrap_or(Ty::Err)
    }

    fn fields_of(&self, def: DefId) -> Option<&[hir::FieldDef]> {
        match &self.krate.item(def).kind {
            hir::ItemKind::Struct { fields } | hir::ItemKind::Variant { fields } => Some(fields),
            _ => None,
        }
    }

    /// 变体在父 enum 定义序中的 (enum_def, 下标)。
    fn variant_pos(&self, variant: DefId) -> Option<(DefId, u32)> {
        let enum_def = *self.res.variant_parents.get(&variant)?;
        let hir::ItemKind::Enum { variants } = &self.krate.item(enum_def).kind else {
            return None;
        };
        let idx = variants.iter().position(|v| *v == variant)? as u32;
        Some((enum_def, idx))
    }

    fn err_expr(&self, span: crate::span::Span) -> tbir::Expr {
        tbir::Expr {
            ty: Ty::Err,
            span,
            kind: tbir::ExprKind::Err,
        }
    }

    // -- scope(RXS-0052) -------------------------------------------------------

    fn enter_scope(&mut self, span: crate::span::Span) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        self.scopes.push(tbir::Scope {
            parent: Some(self.cur_scope),
            span,
        });
        std::mem::replace(&mut self.cur_scope, id)
    }

    fn leave_scope(&mut self, prev: ScopeId) {
        self.cur_scope = prev;
    }

    // -- 模式 --------------------------------------------------------------------

    fn pat(&mut self, p: &hir::Pat) -> tbir::Pat {
        let ty = self.pat_ty(p);
        let kind = match &p.kind {
            hir::PatKind::Wild => tbir::PatKind::Wild,
            hir::PatKind::Binding { local } => {
                self.attach_local(*local);
                tbir::PatKind::Binding {
                    local: *local,
                    sub: None,
                }
            }
            hir::PatKind::At { local, pat } => {
                self.attach_local(*local);
                tbir::PatKind::Binding {
                    local: *local,
                    sub: Some(Box::new(self.pat(pat))),
                }
            }
            hir::PatKind::Lit { negated, lit } => tbir::PatKind::Lit {
                negated: *negated,
                lit: lit.clone(),
            },
            hir::PatKind::Range => tbir::PatKind::Range,
            hir::PatKind::Ref { pat } => tbir::PatKind::Deref(Box::new(self.pat(pat))),
            hir::PatKind::Tuple(v) => {
                tbir::PatKind::Tuple(v.iter().map(|x| self.pat(x)).collect())
            }
            hir::PatKind::Slice(v) => {
                tbir::PatKind::Slice(v.iter().map(|x| self.pat(x)).collect())
            }
            hir::PatKind::Res(r) => match r {
                Res::Def(d) if matches!(self.krate.item(*d).kind, hir::ItemKind::Variant { .. }) =>
                {
                    match self.variant_pos(*d) {
                        Some((enum_def, index)) => tbir::PatKind::Variant {
                            enum_def,
                            variant: *d,
                            index,
                            fields: Vec::new(),
                        },
                        None => tbir::PatKind::Err,
                    }
                }
                // const 模式等:M3.1 作用面外
                _ => tbir::PatKind::Err,
            },
            hir::PatKind::TupleStruct { res, elems } => {
                let fields: Vec<(u32, tbir::Pat)> = elems
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (i as u32, self.pat(x)))
                    .collect();
                self.ctor_pat_kind(res, fields)
            }
            hir::PatKind::Struct { res, fields, .. } => {
                let named: Vec<(u32, tbir::Pat)> = fields
                    .iter()
                    .filter_map(|(name, sub)| {
                        let sub = sub.as_ref()?;
                        let idx = match res {
                            Res::Def(d) => self
                                .fields_of(*d)
                                .and_then(|fs| fs.iter().position(|f| f.name == *name)),
                            _ => None,
                        }?;
                        Some((idx as u32, self.pat(sub)))
                    })
                    .collect();
                self.ctor_pat_kind(res, named)
            }
            hir::PatKind::Err => tbir::PatKind::Err,
        };
        tbir::Pat {
            ty,
            span: p.span,
            kind,
        }
    }

    fn attach_local(&mut self, l: LocalId) {
        if let Some(decl) = self.locals.get_mut(l.0 as usize) {
            decl.scope = self.cur_scope;
        }
    }

    fn ctor_pat_kind(&self, res: &Res, fields: Vec<(u32, tbir::Pat)>) -> tbir::PatKind {
        let Res::Def(d) = res else {
            return tbir::PatKind::Err;
        };
        match self.krate.item(*d).kind {
            hir::ItemKind::Variant { .. } => match self.variant_pos(*d) {
                Some((enum_def, index)) => tbir::PatKind::Variant {
                    enum_def,
                    variant: *d,
                    index,
                    fields,
                },
                None => tbir::PatKind::Err,
            },
            hir::ItemKind::Struct { .. } => tbir::PatKind::Struct { def: *d, fields },
            _ => tbir::PatKind::Err,
        }
    }

    // -- 块 / 语句 -----------------------------------------------------------------

    fn block(&mut self, b: &hir::Block) -> tbir::Block {
        let prev = self.enter_scope(b.span);
        let scope = self.cur_scope;
        let mut stmts = Vec::new();
        for s in &b.stmts {
            match s {
                hir::Stmt::Item(_) => {} // 嵌套 item 经调用点收集(MIR 同口径)
                hir::Stmt::Let { pat, init, .. } => {
                    let init = init.as_ref().map(|e| self.expr(e));
                    let pat = self.pat(pat);
                    stmts.push(tbir::Stmt::Let { pat, init });
                }
                hir::Stmt::Expr(e) => stmts.push(tbir::Stmt::Expr(self.expr(e))),
            }
        }
        let tail = b.tail.as_ref().map(|t| Box::new(self.expr(t)));
        self.leave_scope(prev);
        tbir::Block {
            scope,
            stmts,
            tail,
            span: b.span,
        }
    }

    // -- 表达式 --------------------------------------------------------------------

    fn expr(&mut self, e: &hir::Expr) -> tbir::Expr {
        let ty = self.expr_ty(e);
        let span = e.span;
        let kind = match &e.kind {
            hir::ExprKind::Lit(l) => tbir::ExprKind::Lit(l.clone()),
            hir::ExprKind::SynthInt(v) => tbir::ExprKind::SynthInt(*v),
            hir::ExprKind::Res(r) => match r {
                Res::Local(l) => tbir::ExprKind::Local(*l),
                Res::Def(d) => match &self.krate.item(*d).kind {
                    // 单元变体值 → 空构造(RXS-0048:普通 enum 规则)
                    hir::ItemKind::Variant { fields } if fields.is_empty() => {
                        tbir::ExprKind::Aggregate {
                            def: *d,
                            fields: Vec::new(),
                        }
                    }
                    _ => tbir::ExprKind::Def(*d),
                },
                _ => tbir::ExprKind::Err,
            },
            hir::ExprKind::Unary { op, expr } => tbir::ExprKind::Unary {
                op: *op,
                expr: Box::new(self.expr(expr)),
            },
            hir::ExprKind::Borrow { mutable, expr } => tbir::ExprKind::Borrow {
                mutable: *mutable,
                expr: Box::new(self.expr(expr)),
            },
            hir::ExprKind::Binary { op, lhs, rhs } => tbir::ExprKind::Binary {
                op: *op,
                lhs: Box::new(self.expr(lhs)),
                rhs: Box::new(self.expr(rhs)),
            },
            hir::ExprKind::Assign { op, lhs, rhs } => tbir::ExprKind::Assign {
                op: *op,
                lhs: Box::new(self.expr(lhs)),
                rhs: Box::new(self.expr(rhs)),
            },
            hir::ExprKind::Cast { expr, .. } => tbir::ExprKind::Cast(Box::new(self.expr(expr))),
            hir::ExprKind::Range { lo, hi, .. } => tbir::ExprKind::Range {
                lo: Box::new(self.expr(lo)),
                hi: Box::new(self.expr(hi)),
            },
            hir::ExprKind::Call { callee, args } => {
                if let Some((def, gargs)) = self.tcr.call_targets.get(&e.hir_id) {
                    tbir::ExprKind::Call {
                        def: *def,
                        generic_args: gargs.clone(),
                        args: args.iter().map(|a| self.expr(a)).collect(),
                    }
                } else if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
                    && matches!(
                        self.krate.item(*d).kind,
                        hir::ItemKind::Struct { .. } | hir::ItemKind::Variant { .. }
                    )
                {
                    // 元组结构体 / 变体构造器直调 → 构造节点
                    tbir::ExprKind::Aggregate {
                        def: *d,
                        fields: args.iter().map(|a| self.expr(a)).collect(),
                    }
                } else {
                    tbir::ExprKind::CallIndirect {
                        callee: Box::new(self.expr(callee)),
                        args: args.iter().map(|a| self.expr(a)).collect(),
                    }
                }
            }
            hir::ExprKind::MethodCall { receiver, args, .. } => {
                match self.tcr.call_targets.get(&e.hir_id) {
                    Some((def, gargs)) => {
                        let (def, gargs) = (*def, gargs.clone());
                        let recv = self.expr(receiver);
                        let recv = self.adjust_receiver(recv, def, span);
                        let mut all = vec![recv];
                        all.extend(args.iter().map(|a| self.expr(a)));
                        tbir::ExprKind::Call {
                            def,
                            generic_args: gargs,
                            args: all,
                        }
                    }
                    // 容忍区(receiver Err 等):MIR 作用面诊断兜底
                    None => tbir::ExprKind::Err,
                }
            }
            hir::ExprKind::Field { expr, field } => {
                let raw = self.expr_to_owned(expr);
                let base = self.autoderef(raw);
                let idx = match &base.ty {
                    Ty::Adt(d, _) => self
                        .fields_of(*d)
                        .and_then(|fs| fs.iter().position(|f| f.name == *field)),
                    _ => None,
                };
                match idx {
                    Some(i) => tbir::ExprKind::Field {
                        base: Box::new(base),
                        index: i as u32,
                    },
                    None => tbir::ExprKind::Err,
                }
            }
            hir::ExprKind::TupleField { expr, index } => {
                let raw = self.expr_to_owned(expr);
                let base = self.autoderef(raw);
                tbir::ExprKind::Field {
                    base: Box::new(base),
                    index: *index,
                }
            }
            hir::ExprKind::Index { expr, index } => tbir::ExprKind::Index {
                base: Box::new(self.expr(expr)),
                index: Box::new(self.expr(index)),
            },
            hir::ExprKind::Tuple(v) => {
                tbir::ExprKind::Tuple(v.iter().map(|x| self.expr(x)).collect())
            }
            hir::ExprKind::Array(v) => {
                tbir::ExprKind::Array(v.iter().map(|x| self.expr(x)).collect())
            }
            hir::ExprKind::Repeat { elem, len } => tbir::ExprKind::Repeat {
                elem: Box::new(self.expr(elem)),
                len: Box::new(self.expr(len)),
            },
            hir::ExprKind::StructLit { res, fields } => self.struct_lit(span, res, fields),
            hir::ExprKind::Block(b) | hir::ExprKind::Unsafe(b) => {
                tbir::ExprKind::Block(self.block(b))
            }
            hir::ExprKind::If { cond, then, else_ } => tbir::ExprKind::If {
                cond: Box::new(self.expr(cond)),
                then: self.block(then),
                else_: else_.as_ref().map(|x| Box::new(self.expr(x))),
            },
            hir::ExprKind::While { cond, body } => tbir::ExprKind::While {
                cond: Box::new(self.expr(cond)),
                body: self.block(body),
            },
            hir::ExprKind::Loop { body } => tbir::ExprKind::Loop {
                body: self.block(body),
            },
            hir::ExprKind::Match { scrutinee, arms } => tbir::ExprKind::Match {
                scrutinee: Box::new(self.expr(scrutinee)),
                arms: arms
                    .iter()
                    .map(|a| tbir::Arm {
                        pats: a.pats.iter().map(|p| self.pat(p)).collect(),
                        guard: a.guard.as_ref().map(|g| self.expr(g)),
                        body: self.expr(&a.body),
                    })
                    .collect(),
            },
            hir::ExprKind::Return(op) => {
                tbir::ExprKind::Return(op.as_ref().map(|x| Box::new(self.expr(x))))
            }
            hir::ExprKind::Break(None) => tbir::ExprKind::Break,
            hir::ExprKind::Break(Some(x)) => tbir::ExprKind::BreakValue(Box::new(self.expr(x))),
            hir::ExprKind::Continue => tbir::ExprKind::Continue,
            hir::ExprKind::Closure { .. } => tbir::ExprKind::Closure,
            hir::ExprKind::Err => tbir::ExprKind::Err,
        };
        tbir::Expr { ty, span, kind }
    }

    fn expr_to_owned(&mut self, e: &hir::Expr) -> tbir::Expr {
        self.expr(e)
    }

    /// 一层 autoderef 显式化(字段访问/方法接收者,RXS-0046)。
    fn autoderef(&self, e: tbir::Expr) -> tbir::Expr {
        match &e.ty {
            Ty::Ref(inner, _) => {
                let inner = (**inner).clone();
                let span = e.span;
                tbir::Expr {
                    ty: inner,
                    span,
                    kind: tbir::ExprKind::Unary {
                        op: UnOp::Deref,
                        expr: Box::new(e),
                    },
                }
            }
            _ => e,
        }
    }

    /// 方法接收者按 `self` 形态调整(RXS-0046 方法糖显式化):
    /// `&self`/`&mut self` 且接收者为值 → 显式 autoref;
    /// `self` 按值且接收者为引用 → 显式 autoderef。
    fn adjust_receiver(&self, recv: tbir::Expr, def: DefId, span: crate::span::Span) -> tbir::Expr {
        let self_kind = match &self.krate.item(def).kind {
            hir::ItemKind::Fn(decl) => decl.self_kind,
            _ => None,
        };
        let Some(sk) = self_kind else { return recv };
        let is_ref = matches!(recv.ty, Ty::Ref(..));
        if sk.by_ref {
            if is_ref {
                recv // 一层引用直接传递(&&T 形态在 M2.2 容忍口径外)
            } else {
                let ty = Ty::Ref(Box::new(recv.ty.clone()), sk.mutable);
                tbir::Expr {
                    ty,
                    span,
                    kind: tbir::ExprKind::Borrow {
                        mutable: sk.mutable,
                        expr: Box::new(recv),
                    },
                }
            }
        } else {
            self.autoderef(recv)
        }
    }

    fn struct_lit(
        &mut self,
        span: crate::span::Span,
        res: &Res,
        fields: &[(String, Option<hir::Expr>)],
    ) -> tbir::ExprKind {
        let Res::Def(d) = res else {
            return tbir::ExprKind::Err;
        };
        let Some(defs) = self.fields_of(*d) else {
            return tbir::ExprKind::Err;
        };
        // 按定义序重排(typeck 已保证齐全;缺失/重复仅出现在错误恢复路径)
        let order: Vec<String> = defs.iter().map(|f| f.name.clone()).collect();
        let mut ordered = Vec::with_capacity(order.len());
        for name in &order {
            let Some((_, Some(v))) = fields.iter().find(|(n, v)| n == name && v.is_some()) else {
                return tbir::ExprKind::Err;
            };
            ordered.push(self.expr(v));
        }
        let _ = span;
        tbir::ExprKind::Aggregate {
            def: *d,
            fields: ordered,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    fn tbir_for(src: &str, body_idx: u32) -> tbir::Body {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "前置诊断: {:?}", diag.emitted());
        let krate = cx.hir_crate();
        let res = cx.resolutions();
        let tcr = cx.check_body(crate::hir::BodyId(body_idx));
        build(&krate, &res, &tcr, krate.body(crate::hir::BodyId(body_idx)))
    }

    //@ spec: RXS-0052
    #[test]
    fn scope_tree_tracks_block_nesting_and_locals() {
        let tb = tbir_for(
            "fn f() {\n    let a = 1;\n    {\n        let b = 2;\n        let _c = a + b;\n    }\n}",
            0,
        );
        // 根 + fn 体块 + 内层块 = 3 个 scope,父链成树(RXS-0052)
        assert_eq!(tb.scopes.len(), 3);
        assert_eq!(tb.scopes[0].parent, None);
        assert_eq!(tb.scopes[1].parent, Some(ScopeId(0)));
        assert_eq!(tb.scopes[2].parent, Some(ScopeId(1)));
        let scope_of = |name: &str| tb.locals.iter().find(|l| l.name == name).unwrap().scope;
        assert_eq!(scope_of("a"), ScopeId(1), "a 归属 fn 体块");
        assert_eq!(scope_of("b"), ScopeId(2), "b 归属内层块");
    }

    //@ spec: RXS-0046, RXS-0048
    #[test]
    fn method_sugar_explicit_with_autoref_and_field_index() {
        let src = "struct C {\n    a: i32,\n    v: i32,\n}\nimpl C {\n    fn get(&self) -> i32 {\n        self.v\n    }\n}\nfn f(c: C) -> i32 {\n    c.get()\n}";
        // f 的 body(BodyId 1):c.get() → 显式 Call,receiver autoref 作首实参
        let tb = tbir_for(src, 1);
        let tbir::ExprKind::Block(b) = &tb.value.kind else {
            panic!()
        };
        let tail = b.tail.as_ref().unwrap();
        let tbir::ExprKind::Call { args, .. } = &tail.kind else {
            panic!("期待显式 Call,实得 {:?}", tail.kind)
        };
        assert!(
            matches!(args[0].kind, tbir::ExprKind::Borrow { .. }),
            "receiver 应 autoref: {:?}",
            args[0].kind
        );
        // get 的 body(BodyId 0):self.v → 显式 deref + 定义序下标 1
        let tb = tbir_for(src, 0);
        let tbir::ExprKind::Block(b) = &tb.value.kind else {
            panic!()
        };
        let tail = b.tail.as_ref().unwrap();
        let tbir::ExprKind::Field { base, index } = &tail.kind else {
            panic!("期待下标字段访问,实得 {:?}", tail.kind)
        };
        assert_eq!(*index, 1);
        assert!(
            matches!(
                base.kind,
                tbir::ExprKind::Unary {
                    op: crate::ast::UnOp::Deref,
                    ..
                }
            ),
            "autoderef 应显式化: {:?}",
            base.kind
        );
    }

    //@ spec: RXS-0048, RXS-0051
    #[test]
    fn variant_patterns_carry_discriminant_index() {
        let src = "fn f(o: Option<i32>) -> i32 {\n    match o {\n        None => 0,\n        Some(v) => v,\n    }\n}";
        let tb = tbir_for(src, 0);
        let tbir::ExprKind::Block(b) = &tb.value.kind else {
            panic!()
        };
        let tail = b.tail.as_ref().unwrap();
        let tbir::ExprKind::Match { arms, .. } = &tail.kind else {
            panic!("{:?}", tail.kind)
        };
        let tbir::PatKind::Variant { index: i0, .. } = &arms[0].pats[0].kind else {
            panic!()
        };
        let tbir::PatKind::Variant {
            index: i1, fields, ..
        } = &arms[1].pats[0].kind
        else {
            panic!()
        };
        // None = 变体 0,Some = 变体 1(RXS-0048 定义序)
        assert_eq!((*i0, *i1), (0, 1));
        assert!(matches!(
            fields[0].1.kind,
            tbir::PatKind::Binding { .. }
        ));
    }
}
