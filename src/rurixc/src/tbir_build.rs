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

use crate::ast::{LitKind, UnOp};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::hir::{self, DefId, LocalId, PrimTy, Res};
use crate::resolve::Resolutions;
use crate::tbir::{self, ScopeId};
use crate::ty::Ty;
use crate::typeck::TypeckResults;

pub const E_NON_EXHAUSTIVE: ErrorCode = ErrorCode(2007); // RX2007

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
                shared: false,
                array_len: None,
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
            hir::PatKind::Tuple(v) => tbir::PatKind::Tuple(v.iter().map(|x| self.pat(x)).collect()),
            hir::PatKind::Slice(v) => tbir::PatKind::Slice(v.iter().map(|x| self.pat(x)).collect()),
            hir::PatKind::Res(r) => match r {
                Res::Def(d)
                    if matches!(self.krate.item(*d).kind, hir::ItemKind::Variant { .. }) =>
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
                hir::Stmt::Let {
                    pat,
                    init,
                    ty,
                    shared,
                } => {
                    // `shared let`(M5.3,RXS-0079)与数组长度标注(`[T; N]`)透传到
                    // 绑定的 LocalDecl,供 device codegen 定 addrspace 3 / `[N x T]`。
                    let array_len = match ty {
                        Some(t) => match &t.kind {
                            hir::TyKind::Array { len, .. } => *len,
                            _ => None,
                        },
                        None => None,
                    };
                    if (*shared || array_len.is_some())
                        && let hir::PatKind::Binding { local, .. } = &pat.kind
                    {
                        let li = local.0 as usize;
                        if li < self.locals.len() {
                            self.locals[li].shared = *shared;
                            self.locals[li].array_len = array_len;
                        }
                    }
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
                // 宿主 GPU 上下文构造(MS1.2,RXS-0189):`Context::create()` →
                // GpuCall(无 receiver;MIR 降级为 rxrt_ctx_create,RXS-0192)。
                // present 会话构造 / 宿主图像落盘桥(MS1.2b,RXS-0197/0199):
                // `Present::create(&ctx, ..)` 的 `&ctx`、`write_ppm(.., &pinned)`
                // 的 `&pinned` 借用实参剥壳为句柄表达式(镜像 upload/download)。
                if let Some(op) = self.tcr.gpu_calls.get(&e.hir_id).copied() {
                    let unborrow_at = match op {
                        crate::hir::GpuHostOp::PresentCreate => Some(0),
                        crate::hir::GpuHostOp::WritePpm => Some(3),
                        _ => None,
                    };
                    let lowered = args
                        .iter()
                        .enumerate()
                        .map(|(i, a)| {
                            if unborrow_at == Some(i) {
                                self.gpu_unborrow(a)
                            } else {
                                self.expr(a)
                            }
                        })
                        .collect();
                    return tbir::Expr {
                        ty,
                        span,
                        kind: tbir::ExprKind::GpuCall { op, args: lowered },
                    };
                }
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
                // device intrinsic(M4.2,RXS-0072):`ThreadCtx` 方法 → 无副作用
                // 的 sreg/barrier 取值,接收者(零尺寸句柄)不下放。
                if let Some(intr) = self.tcr.device_calls.get(&e.hir_id) {
                    return tbir::Expr {
                        ty,
                        span,
                        kind: tbir::ExprKind::DeviceCall(*intr),
                    };
                }
                // device 数学 intrinsic(M5.3,RXS-0081):receiver 作 args[0],
                // 后续为方法实参 → libdevice `__nv_*` 调用。
                if let Some((op, elem)) = self.tcr.device_math_calls.get(&e.hir_id) {
                    let mut all = vec![self.expr(receiver)];
                    all.extend(args.iter().map(|a| self.expr(a)));
                    return tbir::Expr {
                        ty,
                        span,
                        kind: tbir::ExprKind::DeviceMathCall {
                            op: *op,
                            is_f32: matches!(elem, crate::hir::PrimTy::F32),
                            args: all,
                        },
                    };
                }
                // 纹理采样(G2.4,RXS-0174/0175;RFC-0007):`tex.sample(samp, coord)`
                // → 采样表达式。typeck 已核对 receiver=Texture2D / args=(Sampler, coord)
                // / fragment 阶段(违例 RX3014);此处仅当恰 2 实参时产采样节点,否则容忍
                // 区兜底(typeck 已发诊断)。
                if self.tcr.sample_calls.contains(&e.hir_id) && args.len() == 2 {
                    let texture = Box::new(self.expr(receiver));
                    let sampler = Box::new(self.expr(&args[0]));
                    let coord = Box::new(self.expr(&args[1]));
                    return tbir::Expr {
                        ty,
                        span,
                        kind: tbir::ExprKind::ResourceSample {
                            texture,
                            sampler,
                            coord,
                        },
                    };
                }
                // 宿主 GPU 编排(MS1.2,RXS-0189/0191):launch → GpuLaunch(kernel
                // 编译期绑定 + 维度分量 + 实参元组);其余已知方法 → GpuCall
                // (receiver 作 args[0];upload/download 的 `&pinned` 借用剥壳为
                // 句柄表达式)。形态残缺落 Err(MIR 报 RX6001,既有口径)。
                if let Some(op) = self.tcr.gpu_calls.get(&e.hir_id).copied() {
                    if op == crate::hir::GpuHostOp::Launch {
                        return self.gpu_launch(receiver, args, ty, span);
                    }
                    let recv = self.expr(receiver);
                    let mut all = vec![self.gpu_deref(recv)];
                    let unborrow = matches!(
                        op,
                        crate::hir::GpuHostOp::BufUpload | crate::hir::GpuHostOp::BufDownload
                    );
                    for a in args {
                        let lowered = if unborrow {
                            self.gpu_unborrow(a)
                        } else {
                            self.expr(a)
                        };
                        all.push(lowered);
                    }
                    return tbir::Expr {
                        ty,
                        span,
                        kind: tbir::ExprKind::GpuCall { op, args: all },
                    };
                }
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

    /// 宿主 GPU 句柄一层 autoderef 显式化(MS1.2,RXS-0189:receiver / 实参可为
    /// `&Handle` 形态;句柄按引用语义读取,MIR 侧不 move)。
    fn gpu_deref(&self, e: tbir::Expr) -> tbir::Expr {
        self.autoderef(e)
    }

    /// upload/download 实参剥壳(MS1.2,RXS-0191):`&pinned` / `&mut pinned`
    /// 借用形态取内层句柄表达式;已是引用值的实参落显式 deref。
    fn gpu_unborrow(&mut self, a: &hir::Expr) -> tbir::Expr {
        match &a.kind {
            hir::ExprKind::Borrow { expr, .. } => {
                let inner = self.expr(expr);
                self.gpu_deref(inner)
            }
            _ => {
                let e = self.expr(a);
                self.gpu_deref(e)
            }
        }
    }

    /// launch 结构化提取(MS1.2,RXS-0191):`stream.launch(kernel, GridDim(..),
    /// BlockDim(..), (args..))` → [`tbir::ExprKind::GpuLaunch`]。kernel 引用须为
    /// fn item、维度须为 GridDim/BlockDim 构造(≤ 3 轴)、实参须为元组;形态
    /// 残缺落 Err(MIR 报 RX6001)。
    fn gpu_launch(
        &mut self,
        receiver: &hir::Expr,
        args: &[hir::Expr],
        ty: Ty,
        span: crate::span::Span,
    ) -> tbir::Expr {
        let structural = 'form: {
            if args.len() != 4 {
                break 'form None;
            }
            let hir::ExprKind::Res(Res::Def(kernel)) = &args[0].kind else {
                break 'form None;
            };
            if !matches!(self.krate.item(*kernel).kind, hir::ItemKind::Fn(_)) {
                break 'form None;
            }
            let Some(grid) = self.gpu_dim_components(&args[1], true) else {
                break 'form None;
            };
            let Some(block) = self.gpu_dim_components(&args[2], false) else {
                break 'form None;
            };
            let hir::ExprKind::Tuple(elems) = &args[3].kind else {
                break 'form None;
            };
            Some((*kernel, grid, block, elems))
        };
        let Some((kernel, grid, block, elems)) = structural else {
            return tbir::Expr {
                ty,
                span,
                kind: tbir::ExprKind::Err,
            };
        };
        let stream = {
            let recv = self.expr(receiver);
            Box::new(self.gpu_deref(recv))
        };
        let grid = grid.iter().map(|c| self.expr(c)).collect();
        let block = block.iter().map(|c| self.expr(c)).collect();
        let largs = elems.iter().map(|a| self.expr(a)).collect();
        tbir::Expr {
            ty,
            span,
            kind: tbir::ExprKind::GpuLaunch {
                stream,
                kernel,
                grid,
                block,
                args: largs,
            },
        }
    }

    /// `GridDim(..)` / `BlockDim(..)` 构造的维度分量表达式(≤ 3 轴;非该构造 /
    /// 超轴 → None,launch 形态残缺口径)。
    fn gpu_dim_components<'e>(&self, e: &'e hir::Expr, grid: bool) -> Option<&'e [hir::Expr]> {
        let hir::ExprKind::Call { callee, args } = &e.kind else {
            return None;
        };
        let hir::ExprKind::Res(Res::Def(d)) = &callee.kind else {
            return None;
        };
        let ok = if grid {
            self.res.lang_items.is_grid_dim(*d)
        } else {
            self.res.lang_items.is_block_dim(*d)
        };
        (ok && args.len() <= 3).then_some(args.as_slice())
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

// ---------------------------------------------------------------------------
// match 穷尽性检查(RXS-0051;检查时点 = TBIR 窄门:typeck 后、MIR 前)
// ---------------------------------------------------------------------------

/// 对一个 TBIR body 内全部 `match` 做穷尽性检查(违例 → RX2007)。
///
/// 算法 = 简化 usefulness(Maranget 特化矩阵):判定域见 RXS-0051——
/// enum 变体 / bool / 元组 / struct / 引用递归;数值/char/str 字面量与
/// 区间、slice 模式不做值域完备性分析(无穷域,必须通配/绑定兜底);
/// 带 guard 的臂不计入;or-pattern 为并集;`x @ p` 按子模式。
pub fn check_exhaustiveness(
    krate: &hir::Crate,
    res: &Resolutions,
    diag: &DiagCtxt,
    body: &tbir::Body,
) {
    let cx = ExhaustCx { krate, res, diag };
    cx.walk_expr(&body.value);
}

struct ExhaustCx<'a> {
    krate: &'a hir::Crate,
    res: &'a Resolutions,
    diag: &'a DiagCtxt,
}

/// 归一化模式(usefulness 输入;or-pattern 已按行展开)。
#[derive(Clone, Debug)]
enum SP {
    Wild,
    Ctor(Key, Vec<SP>),
}

#[derive(Clone, PartialEq, Debug)]
enum Key {
    Bool(bool),
    /// enum 变体(定义序下标)。
    Variant(u32),
    /// 单构造子域(元组 / struct / 引用)。
    Single,
    /// 无穷域字面量/区间/slice(覆盖力为点集,不参与完备判定;
    /// 序号仅保证键互异)。
    Opaque(u32),
}

/// 列构造子描述(完备集元素)。
struct CtorInfo {
    key: Key,
    arg_tys: Vec<Ty>,
    shape: CtorShape,
}

enum CtorShape {
    Text(String),
    Tuple,
    StructBraces(String),
    Ref,
}

impl ExhaustCx<'_> {
    // -- TBIR 走查 ---------------------------------------------------------------

    fn walk_expr(&self, e: &tbir::Expr) {
        match &e.kind {
            tbir::ExprKind::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee);
                for a in arms {
                    if let Some(g) = &a.guard {
                        self.walk_expr(g);
                    }
                    self.walk_expr(&a.body);
                }
                self.check_match(e.span, &scrutinee.ty, arms);
            }
            tbir::ExprKind::Unary { expr, .. }
            | tbir::ExprKind::Borrow { expr, .. }
            | tbir::ExprKind::Cast(expr)
            | tbir::ExprKind::Return(Some(expr))
            | tbir::ExprKind::BreakValue(expr) => self.walk_expr(expr),
            tbir::ExprKind::ResourceSample {
                texture,
                sampler,
                coord,
            } => {
                self.walk_expr(texture);
                self.walk_expr(sampler);
                self.walk_expr(coord);
            }
            tbir::ExprKind::Binary { lhs, rhs, .. }
            | tbir::ExprKind::Assign { lhs, rhs, .. }
            | tbir::ExprKind::Range { lo: lhs, hi: rhs }
            | tbir::ExprKind::Index {
                base: lhs,
                index: rhs,
            }
            | tbir::ExprKind::Repeat {
                elem: lhs,
                len: rhs,
            } => {
                self.walk_expr(lhs);
                self.walk_expr(rhs);
            }
            tbir::ExprKind::Call { args, .. }
            | tbir::ExprKind::DeviceMathCall { args, .. }
            | tbir::ExprKind::GpuCall { args, .. } => {
                for a in args {
                    self.walk_expr(a);
                }
            }
            // 宿主 GPU launch(MS1.2,RXS-0191):stream/维度/实参子树走查。
            tbir::ExprKind::GpuLaunch {
                stream,
                grid,
                block,
                args,
                ..
            } => {
                self.walk_expr(stream);
                for x in grid.iter().chain(block.iter()).chain(args.iter()) {
                    self.walk_expr(x);
                }
            }
            tbir::ExprKind::CallIndirect { callee, args } => {
                self.walk_expr(callee);
                for a in args {
                    self.walk_expr(a);
                }
            }
            tbir::ExprKind::Field { base, .. } => self.walk_expr(base),
            tbir::ExprKind::Tuple(v)
            | tbir::ExprKind::Array(v)
            | tbir::ExprKind::Aggregate { fields: v, .. } => {
                for x in v {
                    self.walk_expr(x);
                }
            }
            tbir::ExprKind::Block(b) => self.walk_block(b),
            tbir::ExprKind::If { cond, then, else_ } => {
                self.walk_expr(cond);
                self.walk_block(then);
                if let Some(x) = else_ {
                    self.walk_expr(x);
                }
            }
            tbir::ExprKind::While { cond, body } => {
                self.walk_expr(cond);
                self.walk_block(body);
            }
            tbir::ExprKind::Loop { body } => self.walk_block(body),
            tbir::ExprKind::Lit(_)
            | tbir::ExprKind::SynthInt(_)
            | tbir::ExprKind::Local(_)
            | tbir::ExprKind::Def(_)
            | tbir::ExprKind::DeviceCall(_)
            | tbir::ExprKind::Return(None)
            | tbir::ExprKind::Break
            | tbir::ExprKind::Continue
            | tbir::ExprKind::Closure
            | tbir::ExprKind::Err => {}
        }
    }

    fn walk_block(&self, b: &tbir::Block) {
        for s in &b.stmts {
            match s {
                tbir::Stmt::Let { init, .. } => {
                    if let Some(e) = init {
                        self.walk_expr(e);
                    }
                }
                tbir::Stmt::Expr(e) => self.walk_expr(e),
            }
        }
        if let Some(t) = &b.tail {
            self.walk_expr(t);
        }
    }

    // -- 检查 ---------------------------------------------------------------------

    fn check_match(&self, span: crate::span::Span, scrut_ty: &Ty, arms: &[tbir::Arm]) {
        if scrut_ty.is_err() {
            return; // 容忍区不级联(RXS-0047 口径)
        }
        let mut opaque = 0u32;
        let matrix: Vec<Vec<SP>> = arms
            .iter()
            .filter(|a| a.guard.is_none()) // 带 guard 的臂不计入(RXS-0051)
            .flat_map(|a| a.pats.iter())
            .map(|p| vec![self.normalize(p, &mut opaque)])
            .collect();
        if let Some(w) = self.useful(std::slice::from_ref(scrut_ty), &matrix) {
            let witness = w.into_iter().next().unwrap_or_else(|| "_".to_owned());
            self.diag
                .struct_error(E_NON_EXHAUSTIVE, "typeck.non_exhaustive_match")
                .arg("witness", format!("`{witness}`"))
                .span_label(span, "non-exhaustive match")
                .emit();
        }
    }

    fn normalize(&self, p: &tbir::Pat, opaque: &mut u32) -> SP {
        match &p.kind {
            tbir::PatKind::Wild => SP::Wild,
            // 错误恢复模式按通配处理(防级联)
            tbir::PatKind::Err => SP::Wild,
            tbir::PatKind::Binding { sub: None, .. } => SP::Wild,
            tbir::PatKind::Binding { sub: Some(s), .. } => self.normalize(s, opaque),
            tbir::PatKind::Lit { lit, .. } => match lit.kind {
                LitKind::Bool(v) => SP::Ctor(Key::Bool(v), Vec::new()),
                _ => {
                    *opaque += 1;
                    SP::Ctor(Key::Opaque(*opaque), Vec::new())
                }
            },
            tbir::PatKind::Range | tbir::PatKind::Slice(_) => {
                *opaque += 1;
                SP::Ctor(Key::Opaque(*opaque), Vec::new())
            }
            tbir::PatKind::Deref(sub) => SP::Ctor(Key::Single, vec![self.normalize(sub, opaque)]),
            tbir::PatKind::Tuple(elems) => SP::Ctor(
                Key::Single,
                elems.iter().map(|x| self.normalize(x, opaque)).collect(),
            ),
            tbir::PatKind::Struct { def, fields } => {
                let arity = self.arity_of(*def);
                SP::Ctor(Key::Single, self.positional(fields, arity, opaque))
            }
            tbir::PatKind::Variant {
                variant,
                index,
                fields,
                ..
            } => {
                let arity = self.arity_of(*variant);
                SP::Ctor(Key::Variant(*index), self.positional(fields, arity, opaque))
            }
        }
    }

    fn arity_of(&self, def: DefId) -> usize {
        match &self.krate.item(def).kind {
            hir::ItemKind::Struct { fields } | hir::ItemKind::Variant { fields } => fields.len(),
            _ => 0,
        }
    }

    /// 字段子模式按定义序展开为全量位置序(缺位补通配)。
    fn positional(&self, fields: &[(u32, tbir::Pat)], arity: usize, opaque: &mut u32) -> Vec<SP> {
        let mut out = vec![SP::Wild; arity];
        for (i, sub) in fields {
            if let Some(slot) = out.get_mut(*i as usize) {
                *slot = self.normalize(sub, opaque);
            }
        }
        out
    }

    /// 列类型的完备构造子集;无穷/未知域(数值/char/str/Param/Err/数组等)
    /// 返回 None(必须通配兜底,RXS-0051)。
    fn complete_ctors(&self, ty: &Ty) -> Option<Vec<CtorInfo>> {
        match ty {
            Ty::Prim(PrimTy::Bool) => Some(vec![
                CtorInfo {
                    key: Key::Bool(false),
                    arg_tys: Vec::new(),
                    shape: CtorShape::Text("false".to_owned()),
                },
                CtorInfo {
                    key: Key::Bool(true),
                    arg_tys: Vec::new(),
                    shape: CtorShape::Text("true".to_owned()),
                },
            ]),
            Ty::Adt(d, args) => match &self.krate.item(*d).kind {
                hir::ItemKind::Enum { variants } => {
                    let enum_name = &self.res.defs[d.0 as usize].name;
                    Some(
                        variants
                            .iter()
                            .enumerate()
                            .map(|(i, v)| {
                                let vname = &self.res.defs[v.0 as usize].name;
                                CtorInfo {
                                    key: Key::Variant(i as u32),
                                    arg_tys: crate::typeck::adt_field_tys(self.krate, *v, args),
                                    shape: CtorShape::Text(format!("{enum_name}::{vname}")),
                                }
                            })
                            .collect(),
                    )
                }
                hir::ItemKind::Struct { .. } => Some(vec![CtorInfo {
                    key: Key::Single,
                    arg_tys: crate::typeck::adt_field_tys(self.krate, *d, args),
                    shape: CtorShape::StructBraces(self.res.defs[d.0 as usize].name.clone()),
                }]),
                _ => None,
            },
            Ty::Tuple(v) => Some(vec![CtorInfo {
                key: Key::Single,
                arg_tys: v.clone(),
                shape: CtorShape::Tuple,
            }]),
            Ty::Ref(inner, _) => Some(vec![CtorInfo {
                key: Key::Single,
                arg_tys: vec![(**inner).clone()],
                shape: CtorShape::Ref,
            }]),
            _ => None,
        }
    }

    fn render_ctor(&self, info: &CtorInfo, args: &[String]) -> String {
        match &info.shape {
            CtorShape::Text(base) => {
                if args.is_empty() {
                    base.clone()
                } else {
                    format!("{base}({})", args.join(", "))
                }
            }
            CtorShape::Tuple => format!("({})", args.join(", ")),
            CtorShape::StructBraces(name) => format!("{name} {{ .. }}"),
            CtorShape::Ref => format!("&{}", args.first().cloned().unwrap_or_default()),
        }
    }

    /// usefulness:全通配向量对矩阵是否有用;有用 → 返回逐列见证(非穷尽)。
    fn useful(&self, tys: &[Ty], matrix: &[Vec<SP>]) -> Option<Vec<String>> {
        let Some(col_ty) = tys.first() else {
            return if matrix.is_empty() {
                Some(Vec::new())
            } else {
                None
            };
        };
        let ctors = self.complete_ctors(col_ty);
        let heads: Vec<&Key> = matrix
            .iter()
            .filter_map(|r| match &r[0] {
                SP::Ctor(k, _) => Some(k),
                SP::Wild => None,
            })
            .collect();
        match &ctors {
            Some(all) if all.iter().all(|c| heads.contains(&&c.key)) => {
                // 构造子完备且全在场:逐构造子特化递归
                for c in all {
                    let m2 = specialize(matrix, &c.key, c.arg_tys.len());
                    let mut t2 = c.arg_tys.clone();
                    t2.extend_from_slice(&tys[1..]);
                    if let Some(w) = self.useful(&t2, &m2) {
                        let (args, rest) = w.split_at(c.arg_tys.len());
                        let mut out = vec![self.render_ctor(c, args)];
                        out.extend_from_slice(rest);
                        return Some(out);
                    }
                }
                None
            }
            _ => {
                // 构造子缺位 / 无穷域:default 矩阵(仅通配行)递归
                let m2: Vec<Vec<SP>> = matrix
                    .iter()
                    .filter(|r| matches!(r[0], SP::Wild))
                    .map(|r| r[1..].to_vec())
                    .collect();
                let w = self.useful(&tys[1..], &m2)?;
                let head = match &ctors {
                    Some(all) => all
                        .iter()
                        .find(|c| !heads.contains(&&c.key))
                        .map(|c| self.render_ctor(c, &vec!["_".to_owned(); c.arg_tys.len()]))
                        .unwrap_or_else(|| "_".to_owned()),
                    None => "_".to_owned(),
                };
                let mut out = vec![head];
                out.extend(w);
                Some(out)
            }
        }
    }
}

/// 特化矩阵:保留首列匹配 `key` 的行(展开其子模式)与通配行(补 `arity` 通配)。
fn specialize(matrix: &[Vec<SP>], key: &Key, arity: usize) -> Vec<Vec<SP>> {
    let mut out = Vec::new();
    for row in matrix {
        match &row[0] {
            SP::Ctor(k, args) if k == key => {
                let mut r = args.clone();
                r.extend_from_slice(&row[1..]);
                out.push(r);
            }
            SP::Wild => {
                let mut r = vec![SP::Wild; arity];
                r.extend_from_slice(&row[1..]);
                out.push(r);
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn pattern_codes(src: &str) -> (Vec<u16>, DiagCtxt) {
        let diag = DiagCtxt::new();
        let codes = {
            let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
            cx.check_crate();
            assert!(
                diag.emitted().is_empty(),
                "前置阶段诊断: {:?}",
                diag.emitted()
            );
            cx.check_crate_patterns();
            diag.emitted()
                .iter()
                .filter_map(|d| d.code.map(|c| c.0))
                .collect()
        };
        (codes, diag)
    }

    //@ spec: RXS-0051
    #[test]
    fn non_exhaustive_enum_match_is_rx2007_with_witness() {
        let (codes, diag) = pattern_codes(
            "enum E {\n    A,\n    B(i32),\n}\nfn f(e: E) -> i32 {\n    match e {\n        E::A => 0,\n    }\n}",
        );
        assert_eq!(codes, vec![2007]);
        let msg = diag.emitted()[0].message(diag.messages());
        assert!(msg.contains("E::B"), "见证应指向缺失变体: {msg}");
    }

    //@ spec: RXS-0051
    #[test]
    fn exhaustive_matches_are_clean() {
        let (codes, _) = pattern_codes(
            "enum E {\n    A,\n    B(i32),\n}\nfn f(e: E, b: bool, p: (bool, i32)) -> i32 {\n    let x = match e {\n        E::A => 0,\n        E::B(v) => v,\n    };\n    let y = match b {\n        true => 1,\n        false => 0,\n    };\n    let z = match p {\n        (true, n) => n,\n        (false, _) => 0,\n    };\n    let w = match x {\n        0 => 0,\n        other => other,\n    };\n    x + y + z + w\n}",
        );
        assert_eq!(codes, Vec::<u16>::new());
    }

    //@ spec: RXS-0051
    #[test]
    fn nested_payload_non_exhaustive_is_rx2007() {
        let (codes, diag) = pattern_codes(
            "fn f(o: Option<bool>) -> i32 {\n    match o {\n        None => 0,\n        Some(true) => 1,\n    }\n}",
        );
        assert_eq!(codes, vec![2007]);
        let msg = diag.emitted()[0].message(diag.messages());
        assert!(msg.contains("Some(false)"), "{msg}");
    }

    //@ spec: RXS-0051
    #[test]
    fn guarded_arms_do_not_count() {
        let (codes, _) = pattern_codes(
            "fn f(b: bool, x: bool) -> i32 {\n    match b {\n        true => 1,\n        false if x => 0,\n    }\n}",
        );
        assert_eq!(codes, vec![2007]);
    }

    //@ spec: RXS-0051
    #[test]
    fn int_scrutinee_needs_wildcard_fallback() {
        let (codes, _) = pattern_codes(
            "fn f(n: i32) -> i32 {\n    match n {\n        0 => 0,\n        1 => 1,\n    }\n}",
        );
        assert_eq!(codes, vec![2007]);
    }

    //@ spec: RXS-0051
    #[test]
    fn or_patterns_union_coverage() {
        let (codes, _) = pattern_codes(
            "enum E {\n    A,\n    B,\n    C,\n}\nfn f(e: E) -> i32 {\n    match e {\n        E::A | E::B => 0,\n        E::C => 1,\n    }\n}",
        );
        assert_eq!(codes, Vec::<u16>::new());
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
        assert!(matches!(fields[0].1.kind, tbir::PatKind::Binding { .. }));
    }
}
