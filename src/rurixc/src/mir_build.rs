//! typed HIR body → MIR lowering + 单态化收集(D-111;M2.3 host 子集)。
//!
//! 入口 [`build_crate`]:自根模块 `main` 起沿 [`crate::typeck::TypeckResults`]
//! 的调用点做可达性收集,每个 (DefId, 泛型实参) 实例独立 lowering(全单态化)。
//!
//! 作用面外构造(closure/`match`/方法调用/索引等)报 RX6001(M2_PLAN v1.3
//! 留痕;`for`/`?` 自 M3.1 在 lower 层 desugar,本层不再见到;match/方法调用
//! 经 TBIR 收口,M3_PLAN §1 任务 4)。

use std::collections::HashSet;
use std::rc::Rc;

use crate::ast::{BinOp, LitKind, LitSuffix, UnOp};
use crate::diag::ErrorCode;
use crate::hir::{self, DefId, LocalId, PrimTy, Res};
use crate::mir::{
    BasicBlock, BlockIdx, Body, CallTarget, Const, Local, LocalIdx, Operand, Place, ProjElem,
    Rvalue, Statement, StatementKind, Terminator, TerminatorKind, mangle,
};
use crate::query::QueryCtx;
use crate::resolve::Resolutions;
use crate::span::Span;
use crate::ty::Ty;
use crate::typeck::TypeckResults;

pub const E_UNSUPPORTED: ErrorCode = ErrorCode(6001); // RX6001

/// MIR 构建入口:单态化收集(自 `main` 可达)+ 逐实例 lowering。
///
/// 根模块无 `main` 时返回空集(是否成错由驱动裁决;库形态随 M3+)。
pub fn build_crate(cx: &QueryCtx<'_>) -> Vec<Body> {
    let krate = cx.hir_crate();
    let main = krate.root_items.iter().copied().find(|d| {
        let it = krate.item(*d);
        it.name == "main"
            && matches!(&it.kind, hir::ItemKind::Fn(decl)
                if decl.body.is_some() && decl.generic_params.is_empty())
    });
    let Some(main) = main else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut worklist: Vec<(DefId, Vec<Ty>)> = vec![(main, Vec::new())];
    visited.insert(mangle(&krate.item(main).name, main, &[]));
    while let Some((def, args)) = worklist.pop() {
        let (body, callees) = build_body(cx, def, args);
        out.push(body);
        for (d, a) in callees {
            let sym = mangle(&krate.item(d).name, d, &a);
            if visited.insert(sym) {
                worklist.push((d, a));
            }
        }
    }
    // 收集序稳定化(worklist 为 DFS;按符号名排序,main 恒首位)
    out.sort_by(|a, b| {
        (a.symbol != "main")
            .cmp(&(b.symbol != "main"))
            .then_with(|| a.symbol.cmp(&b.symbol))
    });
    out
}

/// 单个 (DefId, 泛型实参) 实例的 lowering;返回 (body, 发现的被调用实例)。
fn build_body(
    cx: &QueryCtx<'_>,
    def: DefId,
    generic_args: Vec<Ty>,
) -> (Body, Vec<(DefId, Vec<Ty>)>) {
    let krate = cx.hir_crate();
    let res = cx.resolutions();
    let item = krate.item(def);
    let hir::ItemKind::Fn(decl) = &item.kind else {
        unreachable!("MIR lowering 只对 fn 实例调用")
    };
    let body_id = decl.body.expect("无 body 的 fn 不入收集");
    let hir_body = krate.body(body_id);
    let tcr = cx.check_body(body_id);
    let sig = cx.fn_sig(def);

    let mut b = Builder {
        cx,
        krate: Rc::clone(&krate),
        res: Rc::clone(&res),
        tcr,
        substs: generic_args.clone(),
        locals: vec![Local {
            ty: sig.output.subst(&generic_args),
            name: None,
            span: item.span,
        }],
        blocks: Vec::new(),
        local_map: vec![None; hir_body.locals.len()],
        cur: BlockIdx(0),
        loops: Vec::new(),
        callees: Vec::new(),
    };
    b.new_block();

    // 参数:body.params 的绑定模式直接落位 _1..=_n(复杂模式作用面外)
    let mut arg_count = 0;
    for p in &hir_body.params {
        match &p.kind {
            hir::PatKind::Binding { local } => {
                let idx = b.declare_local(*local, hir_body);
                arg_count += 1;
                debug_assert_eq!(idx.0 as usize, arg_count);
            }
            _ => {
                b.unsupported(p.span, "non-binding parameter pattern");
                arg_count += 1;
            }
        }
    }
    // 其余局部(let 绑定)按声明序落位
    for i in 0..hir_body.locals.len() {
        if b.local_map[i].is_none() {
            b.declare_local(LocalId(i as u32), hir_body);
        }
    }

    let v = b.op_of(&hir_body.value);
    let span = hir_body.value.span;
    b.assign(Place::local(LocalIdx(0)), Rvalue::Use(v), span);
    b.terminate(TerminatorKind::Return, span);

    let blocks = b
        .blocks
        .into_iter()
        .map(|bb| BasicBlock {
            stmts: bb.stmts,
            terminator: bb.term.unwrap_or(Terminator {
                kind: TerminatorKind::Unreachable,
                span: item.span,
            }),
        })
        .collect();
    (
        Body {
            def,
            symbol: mangle(&item.name, def, &generic_args),
            generic_args,
            locals: b.locals,
            arg_count,
            blocks,
            span: item.span,
        },
        b.callees,
    )
}

/// 构建中的基本块(终结子后置)。
struct BlockBuf {
    stmts: Vec<Statement>,
    term: Option<Terminator>,
}

struct Builder<'a, 'q> {
    cx: &'a QueryCtx<'q>,
    krate: Rc<hir::Crate>,
    res: Rc<Resolutions>,
    tcr: Rc<TypeckResults>,
    /// 本实例的单态化实参(类型代入点)。
    substs: Vec<Ty>,
    locals: Vec<Local>,
    blocks: Vec<BlockBuf>,
    /// HIR LocalId → MIR local。
    local_map: Vec<Option<LocalIdx>>,
    cur: BlockIdx,
    /// (continue 目标, break 目标) 栈。
    loops: Vec<(BlockIdx, BlockIdx)>,
    /// 发现的被调用实例(单态化收集输出)。
    callees: Vec<(DefId, Vec<Ty>)>,
}

impl Builder<'_, '_> {
    // -- 基础设施 --------------------------------------------------------------

    fn new_block(&mut self) -> BlockIdx {
        let id = BlockIdx(self.blocks.len() as u32);
        self.blocks.push(BlockBuf {
            stmts: Vec::new(),
            term: None,
        });
        id
    }

    fn declare_local(&mut self, l: LocalId, hir_body: &hir::Body) -> LocalIdx {
        let decl = &hir_body.locals[l.0 as usize];
        let ty = self
            .tcr
            .local_ty
            .get(l.0 as usize)
            .cloned()
            .unwrap_or(Ty::Err)
            .subst(&self.substs);
        let idx = LocalIdx(self.locals.len() as u32);
        self.locals.push(Local {
            ty,
            name: Some(decl.name.clone()),
            span: decl.span,
        });
        self.local_map[l.0 as usize] = Some(idx);
        idx
    }

    fn temp(&mut self, ty: Ty, span: Span) -> LocalIdx {
        let idx = LocalIdx(self.locals.len() as u32);
        self.locals.push(Local {
            ty,
            name: None,
            span,
        });
        idx
    }

    fn assign(&mut self, place: Place, rv: Rvalue, span: Span) {
        self.blocks[self.cur.0 as usize].stmts.push(Statement {
            kind: StatementKind::Assign(place, rv),
            span,
        });
    }

    /// 封口当前块(已封口则丢弃——发散后的死代码路径)。
    fn terminate(&mut self, kind: TerminatorKind, span: Span) {
        let buf = &mut self.blocks[self.cur.0 as usize];
        if buf.term.is_none() {
            buf.term = Some(Terminator { kind, span });
        }
    }

    fn switch_to(&mut self, b: BlockIdx) {
        self.cur = b;
    }

    fn ty_of(&self, e: &hir::Expr) -> Ty {
        self.tcr
            .expr_ty
            .get(&e.hir_id)
            .cloned()
            .unwrap_or(Ty::Err)
            .subst(&self.substs)
    }

    fn unsupported(&mut self, span: Span, construct: &str) -> Operand {
        self.cx
            .diag()
            .struct_error(E_UNSUPPORTED, "codegen.unsupported_construct")
            .arg("construct", construct)
            .span_label(span, "not supported in M2.3 codegen")
            .emit();
        Operand::Const(Const::Unit)
    }

    // -- 字面量取值(源文本切片) ----------------------------------------------

    fn lit_text(&self, span: Span) -> &str {
        &self.cx.src()[span.lo.0 as usize..span.hi.0 as usize]
    }

    fn const_of_lit(&mut self, e: &hir::Expr, l: &crate::ast::Lit) -> Operand {
        let text = self.lit_text(l.span).to_owned();
        let c = match l.kind {
            LitKind::Bool(v) => Const::Bool(v),
            LitKind::Int => {
                let prim = match self.ty_of(e) {
                    Ty::Prim(p) => p,
                    _ => PrimTy::I32,
                };
                match parse_int(&text, l.suffix) {
                    Some(v) => Const::Int(v, prim),
                    None => return self.unsupported(e.span, "integer literal form"),
                }
            }
            LitKind::Float => {
                let prim = match self.ty_of(e) {
                    Ty::Prim(p) => p,
                    _ => PrimTy::F64,
                };
                match parse_float(&text, l.suffix) {
                    Some(v) => Const::Float(v, prim),
                    None => return self.unsupported(e.span, "float literal form"),
                }
            }
            LitKind::Str => match unescape(text.trim_start_matches('"').trim_end_matches('"')) {
                Some(s) => Const::Str(s),
                None => return self.unsupported(e.span, "string escape form"),
            },
            LitKind::Char => {
                let inner = text.trim_start_matches('\'').trim_end_matches('\'');
                match unescape(inner).and_then(|s| s.chars().next()) {
                    Some(c) => Const::Char(c),
                    None => return self.unsupported(e.span, "char literal form"),
                }
            }
        };
        Operand::Const(c)
    }

    // -- place 路径 -------------------------------------------------------------

    /// 表达式的 place 形态(局部/字段/解引用);非 place 形态返回 None。
    fn place_of(&mut self, e: &hir::Expr) -> Option<Place> {
        match &e.kind {
            hir::ExprKind::Res(Res::Local(l)) => {
                let idx = self.local_map.get(l.0 as usize).copied().flatten()?;
                Some(Place::local(idx))
            }
            hir::ExprKind::Field { expr, field } => {
                let base_ty = self.ty_of(expr);
                let idx = self.field_index(&base_ty, field)?;
                let mut p = self.place_of_or_temp(expr);
                p.proj.push(ProjElem::Field(idx));
                Some(p)
            }
            hir::ExprKind::TupleField { expr, index } => {
                let mut p = self.place_of_or_temp(expr);
                p.proj.push(ProjElem::Field(*index));
                Some(p)
            }
            hir::ExprKind::Unary {
                op: UnOp::Deref,
                expr,
            } => {
                let mut p = self.place_of_or_temp(expr);
                p.proj.push(ProjElem::Deref);
                Some(p)
            }
            _ => None,
        }
    }

    /// place 形态;否则物化到 temp(rvalue 提升)。
    fn place_of_or_temp(&mut self, e: &hir::Expr) -> Place {
        if let Some(p) = self.place_of(e) {
            return p;
        }
        let ty = self.ty_of(e);
        let op = self.op_of(e);
        let t = self.temp(ty, e.span);
        self.assign(Place::local(t), Rvalue::Use(op), e.span);
        Place::local(t)
    }

    /// `field` 在 base 类型(Adt,经一层自动解引用)定义序中的下标。
    fn field_index(&self, base_ty: &Ty, field: &str) -> Option<u32> {
        let inner = match base_ty {
            Ty::Ref(t, _) => t.as_ref(),
            t => t,
        };
        let Ty::Adt(d, _) = inner else { return None };
        let hir::ItemKind::Struct { fields } = &self.krate.item(*d).kind else {
            return None;
        };
        fields
            .iter()
            .position(|f| f.name == field)
            .map(|i| i as u32)
    }

    /// rvalue 物化到 temp 并以 Copy 返回。
    fn rvalue_to_op(&mut self, rv: Rvalue, ty: Ty, span: Span) -> Operand {
        let t = self.temp(ty, span);
        self.assign(Place::local(t), rv, span);
        Operand::Copy(Place::local(t))
    }

    // -- 表达式 lowering ---------------------------------------------------------

    fn op_of(&mut self, e: &hir::Expr) -> Operand {
        match &e.kind {
            hir::ExprKind::Lit(l) => self.const_of_lit(e, l),
            // desugar 合成推进步(RXS-0049):值内置,不经源文本切片
            hir::ExprKind::SynthInt(v) => {
                let prim = match self.ty_of(e) {
                    Ty::Prim(p) => p,
                    _ => PrimTy::I32,
                };
                Operand::Const(Const::Int(*v, prim))
            }
            hir::ExprKind::Res(Res::Local(_)) => match self.place_of(e) {
                Some(p) => Operand::Copy(p),
                None => self.unsupported(e.span, "unresolved local"),
            },
            hir::ExprKind::Res(_) => {
                // 裸 fn 引用/const 引用/单元变体:fn 指针与 const eval 随 M3
                self.unsupported(e.span, "value path (const/fn reference)")
            }
            hir::ExprKind::Unary {
                op: UnOp::Deref, ..
            } => match self.place_of(e) {
                Some(p) => Operand::Copy(p),
                None => self.unsupported(e.span, "deref of non-place"),
            },
            hir::ExprKind::Unary { op, expr } => {
                let ty = self.ty_of(e);
                let o = self.op_of(expr);
                self.rvalue_to_op(Rvalue::UnaryOp(*op, o), ty, e.span)
            }
            hir::ExprKind::Borrow { expr, .. } => {
                let ty = self.ty_of(e);
                let p = self.place_of_or_temp(expr);
                self.rvalue_to_op(Rvalue::Ref(p), ty, e.span)
            }
            hir::ExprKind::Binary {
                op: op @ (BinOp::And | BinOp::Or),
                lhs,
                rhs,
            } => self.lower_short_circuit(*op, lhs, rhs, e.span),
            hir::ExprKind::Binary { op, lhs, rhs } => {
                let ty = self.ty_of(e);
                let a = self.op_of(lhs);
                let b = self.op_of(rhs);
                self.rvalue_to_op(Rvalue::BinaryOp(*op, a, b), ty, e.span)
            }
            hir::ExprKind::Assign { op, lhs, rhs } => {
                let Some(p) = self.place_of(lhs) else {
                    return self.unsupported(lhs.span, "assignment to non-place");
                };
                let r = self.op_of(rhs);
                let rv = match op {
                    None => Rvalue::Use(r),
                    Some(o) => Rvalue::BinaryOp(*o, Operand::Copy(p.clone()), r),
                };
                self.assign(p, rv, e.span);
                Operand::Const(Const::Unit)
            }
            hir::ExprKind::Cast { expr, .. } => {
                let target = self.ty_of(e);
                let o = self.op_of(expr);
                self.rvalue_to_op(Rvalue::Cast(o, target.clone()), target, e.span)
            }
            hir::ExprKind::Call { callee, args } => self.lower_call(e, callee, args),
            hir::ExprKind::Field { .. } | hir::ExprKind::TupleField { .. } => {
                match self.place_of(e) {
                    Some(p) => Operand::Copy(p),
                    None => self.unsupported(e.span, "field access on this type"),
                }
            }
            hir::ExprKind::Tuple(elems) => {
                let ty = self.ty_of(e);
                let ops: Vec<Operand> = elems.iter().map(|x| self.op_of(x)).collect();
                if elems.is_empty() {
                    return Operand::Const(Const::Unit);
                }
                self.rvalue_to_op(Rvalue::Aggregate(ty.clone(), ops), ty, e.span)
            }
            hir::ExprKind::StructLit { res, fields } => self.lower_struct_lit(e, res, fields),
            hir::ExprKind::Block(b) | hir::ExprKind::Unsafe(b) => self.lower_block(b),
            hir::ExprKind::If { cond, then, else_ } => self.lower_if(e, cond, then, else_),
            hir::ExprKind::While { cond, body } => {
                let head = self.new_block();
                let body_bb = self.new_block();
                let exit = self.new_block();
                self.terminate(TerminatorKind::Goto(head), e.span);
                self.switch_to(head);
                let c = self.op_of(cond);
                self.terminate(
                    TerminatorKind::SwitchBool {
                        discr: c,
                        then: body_bb,
                        else_: exit,
                    },
                    cond.span,
                );
                self.switch_to(body_bb);
                self.loops.push((head, exit));
                let _ = self.lower_block(body);
                self.loops.pop();
                self.terminate(TerminatorKind::Goto(head), e.span);
                self.switch_to(exit);
                Operand::Const(Const::Unit)
            }
            hir::ExprKind::Loop { body } => {
                let head = self.new_block();
                let exit = self.new_block();
                self.terminate(TerminatorKind::Goto(head), e.span);
                self.switch_to(head);
                self.loops.push((head, exit));
                let _ = self.lower_block(body);
                self.loops.pop();
                self.terminate(TerminatorKind::Goto(head), e.span);
                self.switch_to(exit);
                // break 值随 M3(typeck 同口径容忍);loop 作 () 用
                Operand::Const(Const::Unit)
            }
            hir::ExprKind::Return(op) => {
                let v = match op {
                    Some(x) => self.op_of(x),
                    None => Operand::Const(Const::Unit),
                };
                self.assign(Place::local(LocalIdx(0)), Rvalue::Use(v), e.span);
                self.terminate(TerminatorKind::Return, e.span);
                let dead = self.new_block();
                self.switch_to(dead);
                Operand::Const(Const::Unit)
            }
            hir::ExprKind::Break(None) => {
                if let Some(&(_, exit)) = self.loops.last() {
                    self.terminate(TerminatorKind::Goto(exit), e.span);
                    let dead = self.new_block();
                    self.switch_to(dead);
                    Operand::Const(Const::Unit)
                } else {
                    self.unsupported(e.span, "break outside loop")
                }
            }
            hir::ExprKind::Continue => {
                if let Some(&(head, _)) = self.loops.last() {
                    self.terminate(TerminatorKind::Goto(head), e.span);
                    let dead = self.new_block();
                    self.switch_to(dead);
                    Operand::Const(Const::Unit)
                } else {
                    self.unsupported(e.span, "continue outside loop")
                }
            }
            // ---- M2.3 作用面外(RX6001;desugar 推迟项随 M3 收口) ----
            hir::ExprKind::Break(Some(_)) => self.unsupported(e.span, "break with value"),
            hir::ExprKind::MethodCall { .. } => self.unsupported(e.span, "method call"),
            hir::ExprKind::Index { .. } => self.unsupported(e.span, "indexing"),
            hir::ExprKind::Array(_) | hir::ExprKind::Repeat { .. } => {
                self.unsupported(e.span, "array expression")
            }
            hir::ExprKind::Range { .. } => self.unsupported(e.span, "range expression"),
            hir::ExprKind::Match { .. } => self.unsupported(e.span, "`match` expression"),
            hir::ExprKind::Closure { .. } => self.unsupported(e.span, "closure"),
            hir::ExprKind::Err => self.unsupported(e.span, "erroneous expression"),
        }
    }

    fn lower_short_circuit(
        &mut self,
        op: BinOp,
        lhs: &hir::Expr,
        rhs: &hir::Expr,
        span: Span,
    ) -> Operand {
        let t = self.temp(Ty::Prim(PrimTy::Bool), span);
        let a = self.op_of(lhs);
        self.assign(Place::local(t), Rvalue::Use(a), lhs.span);
        let rhs_bb = self.new_block();
        let join = self.new_block();
        let (then, else_) = match op {
            BinOp::And => (rhs_bb, join),
            _ => (join, rhs_bb),
        };
        self.terminate(
            TerminatorKind::SwitchBool {
                discr: Operand::Copy(Place::local(t)),
                then,
                else_,
            },
            span,
        );
        self.switch_to(rhs_bb);
        let b = self.op_of(rhs);
        self.assign(Place::local(t), Rvalue::Use(b), rhs.span);
        self.terminate(TerminatorKind::Goto(join), span);
        self.switch_to(join);
        Operand::Copy(Place::local(t))
    }

    fn lower_call(&mut self, e: &hir::Expr, callee: &hir::Expr, args: &[hir::Expr]) -> Operand {
        // fn item / 关联 fn(typeck 已记录调用点)
        if let Some((d, gargs)) = self.tcr.call_targets.get(&e.hir_id).cloned() {
            let gargs: Vec<Ty> = gargs.iter().map(|t| t.subst(&self.substs)).collect();
            let target = if let Some(b) = self.res.builtins.get(&d) {
                CallTarget::Builtin(*b)
            } else {
                let item = self.krate.item(d);
                let has_body = matches!(&item.kind, hir::ItemKind::Fn(decl) if decl.body.is_some());
                let symbol = mangle(&item.name, d, &gargs);
                if has_body {
                    self.callees.push((d, gargs.clone()));
                } else if !gargs.is_empty() {
                    return self.unsupported(e.span, "generic extern function");
                }
                CallTarget::Fn { def: d, symbol }
            };
            let ops: Vec<Operand> = args.iter().map(|a| self.op_of(a)).collect();
            let ret_ty = self.ty_of(e);
            let dest = self.temp(ret_ty.clone(), e.span);
            let next = self.new_block();
            self.terminate(
                TerminatorKind::Call {
                    target,
                    args: ops,
                    dest: Place::local(dest),
                    next,
                },
                e.span,
            );
            self.switch_to(next);
            return if ret_ty.is_unit() {
                Operand::Const(Const::Unit)
            } else {
                Operand::Copy(Place::local(dest))
            };
        }
        // 元组结构体构造器直调 → Aggregate
        if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
            && matches!(self.krate.item(*d).kind, hir::ItemKind::Struct { .. })
        {
            let ty = self.ty_of(e);
            let ops: Vec<Operand> = args.iter().map(|a| self.op_of(a)).collect();
            return self.rvalue_to_op(Rvalue::Aggregate(ty.clone(), ops), ty, e.span);
        }
        self.unsupported(e.span, "indirect call (fn pointer)")
    }

    fn lower_struct_lit(
        &mut self,
        e: &hir::Expr,
        res: &Res,
        fields: &[(String, Option<hir::Expr>)],
    ) -> Operand {
        let ty = self.ty_of(e);
        let Res::Def(d) = res else {
            return self.unsupported(e.span, "unresolved struct literal");
        };
        let hir::ItemKind::Struct { fields: defs } = &self.krate.item(*d).kind else {
            return self.unsupported(e.span, "enum variant construction");
        };
        // 按定义序重排(typeck 已保证齐全;缺失即作用面外兜底)
        let order: Vec<String> = defs.iter().map(|f| f.name.clone()).collect();
        let mut ops = Vec::with_capacity(order.len());
        for name in &order {
            let Some((_, Some(v))) = fields.iter().find(|(n, v)| n == name && v.is_some()) else {
                return self.unsupported(e.span, "struct literal with missing fields");
            };
            ops.push(self.op_of(v));
        }
        self.rvalue_to_op(Rvalue::Aggregate(ty.clone(), ops), ty, e.span)
    }

    fn lower_if(
        &mut self,
        e: &hir::Expr,
        cond: &hir::Expr,
        then: &hir::Block,
        else_: &Option<Box<hir::Expr>>,
    ) -> Operand {
        let ty = self.ty_of(e);
        let produces_value = !ty.is_unit() && !ty.is_err();
        let dest = if produces_value {
            Some(self.temp(ty.clone(), e.span))
        } else {
            None
        };
        let c = self.op_of(cond);
        let then_bb = self.new_block();
        let else_bb = self.new_block();
        let join = self.new_block();
        self.terminate(
            TerminatorKind::SwitchBool {
                discr: c,
                then: then_bb,
                else_: else_bb,
            },
            cond.span,
        );
        self.switch_to(then_bb);
        let tv = self.lower_block(then);
        if let Some(d) = dest {
            self.assign(Place::local(d), Rvalue::Use(tv), then.span);
        }
        self.terminate(TerminatorKind::Goto(join), e.span);
        self.switch_to(else_bb);
        if let Some(eb) = else_ {
            let ev = self.op_of(eb);
            if let Some(d) = dest {
                self.assign(Place::local(d), Rvalue::Use(ev), eb.span);
            }
        }
        self.terminate(TerminatorKind::Goto(join), e.span);
        self.switch_to(join);
        match dest {
            Some(d) => Operand::Copy(Place::local(d)),
            None => Operand::Const(Const::Unit),
        }
    }

    fn lower_block(&mut self, b: &hir::Block) -> Operand {
        for stmt in &b.stmts {
            match stmt {
                hir::Stmt::Item(_) => {} // 嵌套 item 经调用点收集
                hir::Stmt::Let { pat, init, .. } => match (&pat.kind, init) {
                    (hir::PatKind::Binding { local }, Some(init)) => {
                        let v = self.op_of(init);
                        let Some(idx) = self.local_map.get(local.0 as usize).copied().flatten()
                        else {
                            let _ = self.unsupported(pat.span, "unresolved binding");
                            continue;
                        };
                        self.assign(Place::local(idx), Rvalue::Use(v), init.span);
                    }
                    (hir::PatKind::Binding { .. }, None) => {} // 延迟绑定:首次赋值落位
                    (hir::PatKind::Wild, Some(init)) => {
                        let _ = self.op_of(init); // 求值后丢弃(副作用保留)
                    }
                    (hir::PatKind::Wild, None) => {}
                    _ => {
                        let _ = self.unsupported(pat.span, "destructuring `let` pattern");
                    }
                },
                hir::Stmt::Expr(e) => {
                    let _ = self.op_of(e);
                }
            }
        }
        match &b.tail {
            Some(t) => self.op_of(t),
            None => Operand::Const(Const::Unit),
        }
    }
}

// ---------------------------------------------------------------------------
// 字面量取值(词法已验证合法性;此处只做值转换)
// ---------------------------------------------------------------------------

fn strip_suffix_text(text: &str, suffix: Option<LitSuffix>) -> &str {
    let Some(s) = suffix else { return text };
    let name = match s {
        LitSuffix::I8 => "i8",
        LitSuffix::I16 => "i16",
        LitSuffix::I32 => "i32",
        LitSuffix::I64 => "i64",
        LitSuffix::U8 => "u8",
        LitSuffix::U16 => "u16",
        LitSuffix::U32 => "u32",
        LitSuffix::U64 => "u64",
        LitSuffix::Usize => "usize",
        LitSuffix::F32 => "f32",
        LitSuffix::F64 => "f64",
    };
    text.strip_suffix(name).unwrap_or(text)
}

fn parse_int(text: &str, suffix: Option<LitSuffix>) -> Option<i128> {
    let t = strip_suffix_text(text, suffix).replace('_', "");
    let (radix, digits) = if let Some(d) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        (16, d)
    } else if let Some(d) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        (8, d)
    } else if let Some(d) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        (2, d)
    } else {
        (10, t.as_str())
    };
    i128::from_str_radix(digits, radix).ok()
}

fn parse_float(text: &str, suffix: Option<LitSuffix>) -> Option<f64> {
    strip_suffix_text(text, suffix)
        .replace('_', "")
        .parse()
        .ok()
}

/// 字符串/字符字面量体的转义还原(词法已验证;`\u{…}` / `\xNN` / 单字符转义)。
pub(crate) fn unescape(body: &str) -> Option<String> {
    let mut out = String::with_capacity(body.len());
    let mut it = body.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next()? {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            '\'' => out.push('\''),
            '0' => out.push('\0'),
            'x' => {
                let h: String = [it.next()?, it.next()?].iter().collect();
                out.push(u8::from_str_radix(&h, 16).ok()? as char);
            }
            'u' => {
                if it.next()? != '{' {
                    return None;
                }
                let mut h = String::new();
                for c in it.by_ref() {
                    if c == '}' {
                        break;
                    }
                    h.push(c);
                }
                out.push(char::from_u32(u32::from_str_radix(&h, 16).ok()?)?);
            }
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    fn mir_text(src: &str) -> (String, Vec<u16>) {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        assert!(
            diag.emitted().is_empty(),
            "前置阶段诊断: {:?}",
            diag.emitted()
        );
        let mir = cx.mir_crate();
        let res = cx.resolutions();
        let text = mir
            .iter()
            .map(|b| crate::mir::pretty(b, &res))
            .collect::<Vec<_>>()
            .join("\n");
        let codes = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        (text, codes)
    }

    /// hello-world body 的 MIR 文本快照(M2_PLAN §3 任务 1 验证方式)。
    #[test]
    fn hello_world_mir_snapshot() {
        let src = "fn main() {\n    let greeting = \"hello, rurix\";\n    println(greeting);\n}";
        let (text, codes) = mir_text(src);
        assert!(codes.is_empty(), "意外诊断: {codes:?}");
        let expected = "\
fn main() -> () {
    let _0: ();
    let _1: &str; // greeting
    let _2: ();
bb0:
    _1 = const \"hello, rurix\";
    _2 = builtin println(_1) -> bb1;
bb1:
    _0 = const ();
    return;
}
";
        assert_eq!(text, expected, "MIR 快照漂移:\n{text}");
    }

    /// 单态化收集:同一泛型 fn 的不同实参 → 独立实例(D-111)。
    #[test]
    fn monomorphization_collects_distinct_instances() {
        let src = "fn pick<T>(a: T, b: T) -> T { a }\n\
                   fn main() {\n    let _i = pick(1i64, 2);\n    let _f = pick(1.0, 2.0);\n}";
        let (text, codes) = mir_text(src);
        assert!(codes.is_empty(), "意外诊断: {codes:?}");
        assert!(text.contains("__i64("), "缺 i64 实例:\n{text}");
        assert!(text.contains("__f64("), "缺 f64 实例:\n{text}");
    }

    /// 控制流 CFG 化:if/while 产生多块结构。
    #[test]
    fn control_flow_builds_cfg() {
        let src = "fn main() {\n    let mut n = 0;\n    while n < 3 {\n        n += 1;\n    }\n    if n == 3 {\n        println(\"done\");\n    }\n}";
        let (text, codes) = mir_text(src);
        assert!(codes.is_empty(), "意外诊断: {codes:?}");
        assert!(text.contains("switch("), "缺条件分支:\n{text}");
        assert!(text.contains("goto -> "), "缺回边:\n{text}");
    }

    /// 作用面外构造 → RX6001(M2.3 口径,不级联 ICE)。
    #[test]
    fn out_of_scope_construct_is_rx6001() {
        let src = "fn main() {\n    let _x = match 1 {\n        _ => 2,\n    };\n}";
        let (_, codes) = mir_text(src);
        assert_eq!(codes, vec![6001]);
    }

    #[test]
    fn literal_value_parsing() {
        assert_eq!(super::parse_int("0xff", None), Some(255));
        assert_eq!(super::parse_int("1_000", None), Some(1000));
        assert_eq!(
            super::parse_int("255u8", Some(crate::ast::LitSuffix::U8)),
            Some(255)
        );
        assert_eq!(super::parse_float("1.5", None), Some(1.5));
        assert_eq!(
            super::unescape("a\\n\\x41\\u{42}"),
            Some("a\nAB".to_owned())
        );
    }
}
