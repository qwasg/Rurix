//! TBIR → MIR lowering + 单态化收集(D-111/D-202;M3.1 管线重排)。
//!
//! 入口 [`build_crate`]:自根模块 `main` 起沿调用点做可达性收集,每个
//! (DefId, 泛型实参) 实例独立 lowering(全单态化)。每实例**即建即用**
//! 一份 TBIR([`crate::tbir_build`]),MIR 构造完成后立即释放(D-202
//! 峰值内存纪律);TBIR 构造耗时/计数经 [`QueryCtx::note_tbir`] 汇总到
//! self-profile `tbir` 阶段(M3.1 出口判据:TBIR 可观测)。
//!
//! M3.1 收口:match 降级(判别测试链 + 绑定提取,先正确性后优化)、enum
//! 变体构造(扁平布局,[`crate::mir::enum_variant_layout`])、方法调用
//! (TBIR 已显式化为直调)。剩余作用面外构造(closure/索引/数组/独立
//! 区间/fn 指针间接调用/带值 break/解构 let 等)报 RX6001,清单留痕
//! M3_PLAN §1 修订行。

use std::collections::HashSet;
use std::rc::Rc;
use std::time::Instant;

use crate::ast::{BinOp, LitKind, LitSuffix, UnOp};
use crate::diag::ErrorCode;
use crate::hir::{self, DefId, LocalId, PrimTy};
use crate::mir::{
    BasicBlock, BlockIdx, Body, CallTarget, Const, Local, LocalIdx, Operand, Place, ProjElem,
    Rvalue, Statement, StatementKind, Terminator, TerminatorKind, enum_variant_layout, mangle,
};
use crate::query::QueryCtx;
use crate::resolve::Resolutions;
use crate::span::Span;
use crate::tbir;
use crate::ty::Ty;

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

    // TBIR 窄门:即建即用(本函数返回前释放,D-202)
    let t = Instant::now();
    let tb = crate::tbir_build::build(&krate, &res, &tcr, hir_body);
    cx.note_tbir(tb.scopes.len() as u64, t.elapsed());

    let mut b = Builder {
        cx,
        krate: Rc::clone(&krate),
        res: Rc::clone(&res),
        substs: generic_args.clone(),
        locals: vec![Local {
            ty: sig.output.subst(&generic_args),
            name: None,
            span: item.span,
        }],
        blocks: Vec::new(),
        local_map: vec![None; tb.locals.len()],
        cur: BlockIdx(0),
        loops: Vec::new(),
        callees: Vec::new(),
    };
    b.new_block();

    // 参数:绑定模式直接落位 _1..=_n(复杂模式作用面外)
    let mut arg_count = 0;
    for p in &tb.params {
        match &p.kind {
            tbir::PatKind::Binding { local, sub: None } => {
                let idx = b.declare_local(*local, &tb);
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
    for i in 0..tb.locals.len() {
        if b.local_map[i].is_none() {
            b.declare_local(LocalId(i as u32), &tb);
        }
    }

    let v = b.op_of(&tb.value);
    let span = tb.value.span;
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
    /// 本实例的单态化实参(类型代入点)。
    substs: Vec<Ty>,
    locals: Vec<Local>,
    blocks: Vec<BlockBuf>,
    /// TBIR LocalId → MIR local。
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

    fn declare_local(&mut self, l: LocalId, tb: &tbir::Body) -> LocalIdx {
        let decl = &tb.locals[l.0 as usize];
        let ty = decl.ty.subst(&self.substs);
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

    fn ty_of(&self, e: &tbir::Expr) -> Ty {
        e.ty.subst(&self.substs)
    }

    fn unsupported(&mut self, span: Span, construct: &str) -> Operand {
        self.cx
            .diag()
            .struct_error(E_UNSUPPORTED, "codegen.unsupported_construct")
            .arg("construct", construct)
            .span_label(span, "not supported by MIR lowering yet (M3.1 host subset)")
            .emit();
        Operand::Const(Const::Unit)
    }

    // -- 字面量取值(源文本切片) ----------------------------------------------

    fn lit_text(&self, span: Span) -> &str {
        &self.cx.src()[span.lo.0 as usize..span.hi.0 as usize]
    }

    fn const_of_lit(&mut self, ty: &Ty, l: &crate::ast::Lit, span: Span) -> Operand {
        let text = self.lit_text(l.span).to_owned();
        let c = match l.kind {
            LitKind::Bool(v) => Const::Bool(v),
            LitKind::Int => {
                let prim = match ty {
                    Ty::Prim(p) => *p,
                    _ => PrimTy::I32,
                };
                match parse_int(&text, l.suffix) {
                    Some(v) => Const::Int(v, prim),
                    None => return self.unsupported(span, "integer literal form"),
                }
            }
            LitKind::Float => {
                let prim = match ty {
                    Ty::Prim(p) => *p,
                    _ => PrimTy::F64,
                };
                match parse_float(&text, l.suffix) {
                    Some(v) => Const::Float(v, prim),
                    None => return self.unsupported(span, "float literal form"),
                }
            }
            LitKind::Str => match unescape(text.trim_start_matches('"').trim_end_matches('"')) {
                Some(s) => Const::Str(s),
                None => return self.unsupported(span, "string escape form"),
            },
            LitKind::Char => {
                let inner = text.trim_start_matches('\'').trim_end_matches('\'');
                match unescape(inner).and_then(|s| s.chars().next()) {
                    Some(c) => Const::Char(c),
                    None => return self.unsupported(span, "char literal form"),
                }
            }
        };
        Operand::Const(c)
    }

    // -- place 路径 -------------------------------------------------------------

    /// 表达式的 place 形态(局部/字段/解引用);非 place 形态返回 None。
    fn place_of(&mut self, e: &tbir::Expr) -> Option<Place> {
        match &e.kind {
            tbir::ExprKind::Local(l) => {
                let idx = self.local_map.get(l.0 as usize).copied().flatten()?;
                Some(Place::local(idx))
            }
            tbir::ExprKind::Field { base, index } => {
                let mut p = self.place_of_or_temp(base);
                p.proj.push(ProjElem::Field(*index));
                Some(p)
            }
            tbir::ExprKind::Unary {
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
    fn place_of_or_temp(&mut self, e: &tbir::Expr) -> Place {
        if let Some(p) = self.place_of(e) {
            return p;
        }
        let ty = self.ty_of(e);
        let op = self.op_of(e);
        let t = self.temp(ty, e.span);
        self.assign(Place::local(t), Rvalue::Use(op), e.span);
        Place::local(t)
    }

    /// rvalue 物化到 temp 并以 Copy 返回。
    fn rvalue_to_op(&mut self, rv: Rvalue, ty: Ty, span: Span) -> Operand {
        let t = self.temp(ty, span);
        self.assign(Place::local(t), rv, span);
        Operand::Copy(Place::local(t))
    }

    // -- 表达式 lowering ---------------------------------------------------------

    fn op_of(&mut self, e: &tbir::Expr) -> Operand {
        match &e.kind {
            tbir::ExprKind::Lit(l) => {
                let ty = self.ty_of(e);
                self.const_of_lit(&ty, l, e.span)
            }
            // desugar 合成推进步(RXS-0049):值内置,不经源文本切片
            tbir::ExprKind::SynthInt(v) => {
                let prim = match self.ty_of(e) {
                    Ty::Prim(p) => p,
                    _ => PrimTy::I32,
                };
                Operand::Const(Const::Int(*v, prim))
            }
            tbir::ExprKind::Local(_) => match self.place_of(e) {
                Some(p) => Operand::Copy(p),
                None => self.unsupported(e.span, "unresolved local"),
            },
            tbir::ExprKind::Def(_) => {
                // 裸 fn 引用/const 引用:fn 指针与 const eval 随 M3.2+/M3.4
                self.unsupported(e.span, "value path (const/fn reference)")
            }
            tbir::ExprKind::Unary {
                op: UnOp::Deref, ..
            } => match self.place_of(e) {
                Some(p) => Operand::Copy(p),
                None => self.unsupported(e.span, "deref of non-place"),
            },
            tbir::ExprKind::Unary { op, expr } => {
                let ty = self.ty_of(e);
                let o = self.op_of(expr);
                self.rvalue_to_op(Rvalue::UnaryOp(*op, o), ty, e.span)
            }
            tbir::ExprKind::Borrow { expr, .. } => {
                let ty = self.ty_of(e);
                let p = self.place_of_or_temp(expr);
                self.rvalue_to_op(Rvalue::Ref(p), ty, e.span)
            }
            tbir::ExprKind::Binary {
                op: op @ (BinOp::And | BinOp::Or),
                lhs,
                rhs,
            } => self.lower_short_circuit(*op, lhs, rhs, e.span),
            tbir::ExprKind::Binary { op, lhs, rhs } => {
                let ty = self.ty_of(e);
                let a = self.op_of(lhs);
                let b = self.op_of(rhs);
                self.rvalue_to_op(Rvalue::BinaryOp(*op, a, b), ty, e.span)
            }
            tbir::ExprKind::Assign { op, lhs, rhs } => {
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
            tbir::ExprKind::Cast(expr) => {
                let target = self.ty_of(e);
                let o = self.op_of(expr);
                self.rvalue_to_op(Rvalue::Cast(o, target.clone()), target, e.span)
            }
            tbir::ExprKind::Call {
                def,
                generic_args,
                args,
            } => self.lower_call(e, *def, generic_args, args),
            tbir::ExprKind::CallIndirect { .. } => {
                self.unsupported(e.span, "indirect call (fn pointer)")
            }
            tbir::ExprKind::Field { .. } => match self.place_of(e) {
                Some(p) => Operand::Copy(p),
                None => self.unsupported(e.span, "field access on this type"),
            },
            tbir::ExprKind::Tuple(elems) => {
                let ty = self.ty_of(e);
                let ops: Vec<Operand> = elems.iter().map(|x| self.op_of(x)).collect();
                if elems.is_empty() {
                    return Operand::Const(Const::Unit);
                }
                self.rvalue_to_op(Rvalue::Aggregate(ty.clone(), ops), ty, e.span)
            }
            tbir::ExprKind::Aggregate { def, fields } => self.lower_aggregate(e, *def, fields),
            tbir::ExprKind::Block(b) => self.lower_block(b),
            tbir::ExprKind::If { cond, then, else_ } => self.lower_if(e, cond, then, else_),
            tbir::ExprKind::While { cond, body } => {
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
            tbir::ExprKind::Loop { body } => {
                let head = self.new_block();
                let exit = self.new_block();
                self.terminate(TerminatorKind::Goto(head), e.span);
                self.switch_to(head);
                self.loops.push((head, exit));
                let _ = self.lower_block(body);
                self.loops.pop();
                self.terminate(TerminatorKind::Goto(head), e.span);
                self.switch_to(exit);
                // break 值随 M3.2+(typeck 同口径容忍);loop 作 () 用
                Operand::Const(Const::Unit)
            }
            tbir::ExprKind::Match { scrutinee, arms } => self.lower_match(e, scrutinee, arms),
            tbir::ExprKind::Return(op) => {
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
            tbir::ExprKind::Break => {
                if let Some(&(_, exit)) = self.loops.last() {
                    self.terminate(TerminatorKind::Goto(exit), e.span);
                    let dead = self.new_block();
                    self.switch_to(dead);
                    Operand::Const(Const::Unit)
                } else {
                    self.unsupported(e.span, "break outside loop")
                }
            }
            tbir::ExprKind::Continue => {
                if let Some(&(head, _)) = self.loops.last() {
                    self.terminate(TerminatorKind::Goto(head), e.span);
                    let dead = self.new_block();
                    self.switch_to(dead);
                    Operand::Const(Const::Unit)
                } else {
                    self.unsupported(e.span, "continue outside loop")
                }
            }
            // ---- M3.1 作用面外(RX6001;清单留痕 M3_PLAN §1 修订行) ----
            tbir::ExprKind::BreakValue(_) => self.unsupported(e.span, "break with value"),
            tbir::ExprKind::Index { .. } => self.unsupported(e.span, "indexing"),
            tbir::ExprKind::Array(_) | tbir::ExprKind::Repeat { .. } => {
                self.unsupported(e.span, "array expression")
            }
            tbir::ExprKind::Range { .. } => self.unsupported(e.span, "range expression"),
            tbir::ExprKind::Closure => self.unsupported(e.span, "closure"),
            tbir::ExprKind::Err => self.unsupported(e.span, "erroneous expression"),
        }
    }

    fn lower_short_circuit(
        &mut self,
        op: BinOp,
        lhs: &tbir::Expr,
        rhs: &tbir::Expr,
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

    fn lower_call(
        &mut self,
        e: &tbir::Expr,
        def: DefId,
        generic_args: &[Ty],
        args: &[tbir::Expr],
    ) -> Operand {
        let gargs: Vec<Ty> = generic_args.iter().map(|t| t.subst(&self.substs)).collect();
        let target = if let Some(b) = self.res.builtins.get(&def) {
            CallTarget::Builtin(*b)
        } else {
            let item = self.krate.item(def);
            let has_body = matches!(&item.kind, hir::ItemKind::Fn(decl) if decl.body.is_some());
            let symbol = mangle(&item.name, def, &gargs);
            if has_body {
                self.callees.push((def, gargs.clone()));
            } else if !gargs.is_empty() {
                return self.unsupported(e.span, "generic extern function");
            }
            CallTarget::Fn { def, symbol }
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
        if ret_ty.is_unit() {
            Operand::Const(Const::Unit)
        } else {
            Operand::Copy(Place::local(dest))
        }
    }

    /// struct / 元组结构体 / enum 变体构造(TBIR 已按定义序重排齐全)。
    fn lower_aggregate(&mut self, e: &tbir::Expr, def: DefId, fields: &[tbir::Expr]) -> Operand {
        let ty = self.ty_of(e);
        match &self.krate.item(def).kind {
            hir::ItemKind::Variant { .. } => {
                let Some((_, tag, base)) = self.variant_layout(def) else {
                    return self.unsupported(e.span, "enum variant construction");
                };
                let ops: Vec<Operand> = fields.iter().map(|f| self.op_of(f)).collect();
                self.rvalue_to_op(
                    Rvalue::VariantAggregate {
                        ty: ty.clone(),
                        tag,
                        base,
                        ops,
                    },
                    ty,
                    e.span,
                )
            }
            hir::ItemKind::Struct { .. } => {
                let ops: Vec<Operand> = fields.iter().map(|f| self.op_of(f)).collect();
                self.rvalue_to_op(Rvalue::Aggregate(ty.clone(), ops), ty, e.span)
            }
            _ => self.unsupported(e.span, "unresolved struct literal"),
        }
    }

    /// 变体的 (enum_def, 判别下标, 载荷布局基址)。
    fn variant_layout(&self, variant: DefId) -> Option<(DefId, u32, u32)> {
        let enum_def = *self.res.variant_parents.get(&variant)?;
        let layout = enum_variant_layout(&self.krate, enum_def);
        let (idx, (_, base)) = layout
            .iter()
            .enumerate()
            .find(|(_, (v, _))| *v == variant)?;
        Some((enum_def, idx as u32, *base))
    }

    fn lower_if(
        &mut self,
        e: &tbir::Expr,
        cond: &tbir::Expr,
        then: &tbir::Block,
        else_: &Option<Box<tbir::Expr>>,
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

    fn lower_block(&mut self, b: &tbir::Block) -> Operand {
        for stmt in &b.stmts {
            match stmt {
                tbir::Stmt::Let { pat, init } => match (&pat.kind, init) {
                    (tbir::PatKind::Binding { local, sub: None }, Some(init)) => {
                        let v = self.op_of(init);
                        let Some(idx) = self.local_map.get(local.0 as usize).copied().flatten()
                        else {
                            let _ = self.unsupported(pat.span, "unresolved binding");
                            continue;
                        };
                        self.assign(Place::local(idx), Rvalue::Use(v), init.span);
                    }
                    (tbir::PatKind::Binding { sub: None, .. }, None) => {} // 延迟绑定:首次赋值落位
                    (tbir::PatKind::Wild, Some(init)) => {
                        let _ = self.op_of(init); // 求值后丢弃(副作用保留)
                    }
                    (tbir::PatKind::Wild, None) => {}
                    _ => {
                        // 解构 let:不可反驳性条款随 M3.2(spec/borrow.md RXS-0051 留痕)
                        let _ = self.unsupported(pat.span, "destructuring `let` pattern");
                    }
                },
                tbir::Stmt::Expr(e) => {
                    let _ = self.op_of(e);
                }
            }
        }
        match &b.tail {
            Some(t) => self.op_of(t),
            None => Operand::Const(Const::Unit),
        }
    }

    // -- match 降级(M3.1:顺序候选测试链,先正确性后优化) ----------------------

    fn lower_match(
        &mut self,
        e: &tbir::Expr,
        scrutinee: &tbir::Expr,
        arms: &[tbir::Arm],
    ) -> Operand {
        let ty = self.ty_of(e);
        let produces_value = !ty.is_unit() && !ty.is_err();
        let dest = if produces_value {
            Some(self.temp(ty.clone(), e.span))
        } else {
            None
        };
        let scrut_place = self.place_of_or_temp(scrutinee);
        let join = self.new_block();

        // 候选 = (模式, 所属臂);or-pattern 展开为多候选(臂体按候选重复
        // lowering,M3.1 取舍——共享体的块复用随诊断/优化期评估)
        for arm in arms {
            for pat in &arm.pats {
                let fail = self.new_block();
                self.lower_pat_test(pat, scrut_place.clone(), fail);
                if let Some(g) = &arm.guard {
                    let gv = self.op_of(g);
                    let ok = self.new_block();
                    self.terminate(
                        TerminatorKind::SwitchBool {
                            discr: gv,
                            then: ok,
                            else_: fail,
                        },
                        g.span,
                    );
                    self.switch_to(ok);
                }
                let v = self.op_of(&arm.body);
                if let Some(d) = dest {
                    self.assign(Place::local(d), Rvalue::Use(v), arm.body.span);
                }
                self.terminate(TerminatorKind::Goto(join), e.span);
                self.switch_to(fail);
            }
        }
        // 末候选失败:穷尽性由 TBIR 窄门静态把关(RXS-0051),此处死路封口
        self.terminate(TerminatorKind::Unreachable, e.span);
        self.switch_to(join);
        match dest {
            Some(d) => Operand::Copy(Place::local(d)),
            None => Operand::Const(Const::Unit),
        }
    }

    /// 模式测试 + 绑定提取:测试失败跳 `fail`,成功落入当前块尾。
    fn lower_pat_test(&mut self, pat: &tbir::Pat, place: Place, fail: BlockIdx) {
        match &pat.kind {
            tbir::PatKind::Wild => {}
            tbir::PatKind::Binding { local, sub } => {
                if let Some(idx) = self.local_map.get(local.0 as usize).copied().flatten() {
                    self.assign(
                        Place::local(idx),
                        Rvalue::Use(Operand::Copy(place.clone())),
                        pat.span,
                    );
                } else {
                    let _ = self.unsupported(pat.span, "unresolved binding");
                }
                if let Some(sub) = sub {
                    self.lower_pat_test(sub, place, fail);
                }
            }
            tbir::PatKind::Lit { negated, lit } => {
                let ty = pat.ty.subst(&self.substs);
                let c = self.const_of_lit(&ty, lit, pat.span);
                let c = match (negated, c) {
                    (true, Operand::Const(Const::Int(v, p))) => {
                        Operand::Const(Const::Int(-v, p))
                    }
                    (true, Operand::Const(Const::Float(v, p))) => {
                        Operand::Const(Const::Float(-v, p))
                    }
                    (_, other) => other,
                };
                if matches!(c, Operand::Const(Const::Str(_))) {
                    let _ = self.unsupported(pat.span, "string literal pattern");
                    return;
                }
                let t = self.temp(Ty::Prim(PrimTy::Bool), pat.span);
                self.assign(
                    Place::local(t),
                    Rvalue::BinaryOp(BinOp::Eq, Operand::Copy(place), c),
                    pat.span,
                );
                self.branch_on(t, fail, pat.span);
            }
            tbir::PatKind::Deref(sub) => {
                let mut p = place;
                p.proj.push(ProjElem::Deref);
                self.lower_pat_test(sub, p, fail);
            }
            tbir::PatKind::Tuple(elems) => {
                for (i, sub) in elems.iter().enumerate() {
                    let mut p = place.clone();
                    p.proj.push(ProjElem::Field(i as u32));
                    self.lower_pat_test(sub, p, fail);
                }
            }
            tbir::PatKind::Struct { fields, .. } => {
                for (idx, sub) in fields {
                    let mut p = place.clone();
                    p.proj.push(ProjElem::Field(*idx));
                    self.lower_pat_test(sub, p, fail);
                }
            }
            tbir::PatKind::Variant {
                variant,
                index,
                fields,
                ..
            } => {
                let Some((_, _, base)) = self.variant_layout(*variant) else {
                    let _ = self.unsupported(pat.span, "enum variant pattern");
                    return;
                };
                // 判别测试
                let d = self.temp(Ty::Prim(PrimTy::I32), pat.span);
                self.assign(
                    Place::local(d),
                    Rvalue::Discriminant(place.clone()),
                    pat.span,
                );
                let t = self.temp(Ty::Prim(PrimTy::Bool), pat.span);
                self.assign(
                    Place::local(t),
                    Rvalue::BinaryOp(
                        BinOp::Eq,
                        Operand::Copy(Place::local(d)),
                        Operand::Const(Const::Int(*index as i128, PrimTy::I32)),
                    ),
                    pat.span,
                );
                self.branch_on(t, fail, pat.span);
                // 载荷字段递归
                for (fidx, sub) in fields {
                    let mut p = place.clone();
                    p.proj.push(ProjElem::VariantField {
                        variant: *variant,
                        base,
                        field: *fidx,
                    });
                    self.lower_pat_test(sub, p, fail);
                }
            }
            // 作用面外模式(区间/slice/const 模式):RX6001 留痕
            tbir::PatKind::Range => {
                let _ = self.unsupported(pat.span, "range pattern");
            }
            tbir::PatKind::Slice(_) => {
                let _ = self.unsupported(pat.span, "slice pattern");
            }
            tbir::PatKind::Err => {
                let _ = self.unsupported(pat.span, "unsupported pattern");
            }
        }
    }

    /// `t` 为真落入新 cont 块,否则跳 `fail`。
    fn branch_on(&mut self, t: LocalIdx, fail: BlockIdx, span: Span) {
        let cont = self.new_block();
        self.terminate(
            TerminatorKind::SwitchBool {
                discr: Operand::Copy(Place::local(t)),
                then: cont,
                else_: fail,
            },
            span,
        );
        self.switch_to(cont);
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

    /// match 降级(M3.1):判别读取 + 测试链 + 变体构造(RXS-0051 通路)。
    //@ spec: RXS-0051
    #[test]
    fn match_lowers_to_discriminant_tests() {
        let src = "enum E {\n    A,\n    B(i32),\n}\nfn main() {\n    let e = E::B(7);\n    let _v = match e {\n        E::A => 0,\n        E::B(x) => x,\n    };\n}";
        let (text, codes) = mir_text(src);
        assert!(codes.is_empty(), "意外诊断: {codes:?}");
        assert!(text.contains("discriminant("), "缺判别读取:\n{text}");
        assert!(text.contains("#1 {"), "缺变体构造(tag 1):\n{text}");
        assert!(text.contains(".v["), "缺载荷投影:\n{text}");
    }

    /// 方法调用经 TBIR 显式化为直调(RXS-0046/0048;receiver autoref)。
    //@ spec: RXS-0048
    #[test]
    fn method_call_lowers_to_explicit_call() {
        let src = "struct C {\n    v: i32,\n}\nimpl C {\n    fn get(&self) -> i32 {\n        self.v\n    }\n}\nfn main() {\n    let c = C { v: 3 };\n    let _x = c.get();\n}";
        let (text, codes) = mir_text(src);
        assert!(codes.is_empty(), "意外诊断: {codes:?}");
        assert!(text.contains("rx_get_"), "方法实例缺失:\n{text}");
        assert!(text.contains("= &_"), "receiver autoref 缺失:\n{text}");
    }

    /// for-range desugar 全管线:无 RX6001(RXS-0049 出口判据通路)。
    //@ spec: RXS-0049
    #[test]
    fn for_range_lowers_without_diagnostics() {
        let src = "fn main() {\n    let mut acc = 0;\n    for i in 0..4 {\n        if i == 2 {\n            continue;\n        }\n        acc += i;\n    }\n    let _r = acc;\n}";
        let (text, codes) = mir_text(src);
        assert!(codes.is_empty(), "意外诊断: {codes:?}");
        assert!(text.contains("discriminant("), "desugar match 缺失:\n{text}");
    }

    /// `?` desugar 全管线:无 RX6001(RXS-0050 出口判据通路)。
    //@ spec: RXS-0050
    #[test]
    fn question_mark_lowers_without_diagnostics() {
        let src = "fn half(x: i32) -> Result<i32, i32> {\n    if x % 2 == 0 { Ok(x / 2) } else { Err(x) }\n}\nfn main() {\n    let _r = match half(6) {\n        Ok(v) => v,\n        Err(e) => e,\n    };\n}";
        let (text, codes) = mir_text(src);
        assert!(codes.is_empty(), "意外诊断: {codes:?}");
        assert!(text.contains("rx_half_"), "callee 实例缺失:\n{text}");
    }

    /// 作用面外构造 → RX6001(M3.1 口径,不级联 ICE)。
    #[test]
    fn out_of_scope_construct_is_rx6001() {
        let src = "fn main() {\n    let _x = [1, 2, 3];\n}";
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
