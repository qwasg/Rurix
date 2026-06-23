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
    BasicBlock, BlockIdx, Body, BorrowKind, CallTarget, Const, Local, LocalIdx, Operand, Place,
    ProjElem, Rvalue, Statement, StatementKind, Terminator, TerminatorKind, enum_variant_layout,
    mangle,
};
use crate::query::QueryCtx;
use crate::resolve::Resolutions;
use crate::span::Span;
use crate::tbir;
use crate::ty::Ty;

pub const E_UNSUPPORTED: ErrorCode = ErrorCode(6001); // RX6001

/// 单态化收集中发现的被调用实例 (DefId, 泛型实参)。
type Callees = Vec<(DefId, Vec<Ty>)>;
/// [`build_body`] 产物:(body, 被调用实例, const 引用求值首错)。
type BuildOutput = (Body, Callees, Option<crate::const_eval::ConstError>);

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
        let (mut body, callees, _const_err) = build_body(cx, def, args);
        // drop elaboration(RXS-0055):move/init 感知精化 + drop flag
        crate::drop_elab::elaborate(&mut body);
        // drop glue 需要的 Drop::drop 单态化实例并入收集
        let drop_callees = crate::drop_elab::collect_drop_callees(&krate, &body);
        out.push(body);
        for (d, a) in callees.into_iter().chain(drop_callees) {
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

/// device MIR 构建入口(M4.2,RXS-0070):以 `kernel fn` 为收集根(不依赖
/// host `main` 可达性),沿 device 调用图收集 `device fn`;产物供 device
/// codegen(MIR→NVPTX IR→PTX)消费。host 收集(`build_crate`)不受影响。
pub fn build_device_crate(cx: &QueryCtx<'_>) -> Vec<Body> {
    let krate = cx.hir_crate();
    let mut out = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut worklist: Vec<(DefId, Vec<Ty>)> = Vec::new();
    // 根 = 全部 `kernel fn`(有 body、无泛型参数;泛型 kernel 随单态化扩展 M4+)。
    // 着色阶段函数(`decl.stage.is_some()`)虽取 kernel 着色,但本里程碑仅类型面,
    // 不进 device codegen 收集根(DXIL codegen 属 G2.2,RFC-0002 §8;PTX 后端不收集
    // 图形/RT 着色阶段)。
    for item in &krate.items {
        if let hir::ItemKind::Fn(decl) = &item.kind
            && decl.color == crate::ast::FnColor::Kernel
            && decl.stage.is_none()
            && decl.body.is_some()
            && decl.generic_params.is_empty()
            && visited.insert(mangle(&item.name, item.def_id, &[]))
        {
            worklist.push((item.def_id, Vec::new()));
        }
    }
    while let Some((def, args)) = worklist.pop() {
        let (mut body, callees, _const_err) = build_body(cx, def, args);
        crate::drop_elab::elaborate(&mut body);
        let drop_callees = crate::drop_elab::collect_drop_callees(&krate, &body);
        out.push(body);
        for (d, a) in callees.into_iter().chain(drop_callees) {
            let sym = mangle(&krate.item(d).name, d, &a);
            if visited.insert(sym) {
                worklist.push((d, a));
            }
        }
    }
    out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    out
}

/// const 求值专用单实例构建(M3.4,RXS-0062):构建 const item / const fn 的
/// MIR body 供 [`crate::const_eval`] 解释,不入 `main` 可达性收集、不跑 drop
/// elaboration(标量 const 无 needs-drop)。body 内对其他 const 的引用在构建期
/// 即经 [`QueryCtx::eval_const`] 内联;若引用求值失败(含环引用),首个错误经
/// `Err` 上抛(RXS-0063/RXS-0065)。
pub fn build_for_const_eval(
    cx: &QueryCtx<'_>,
    def: DefId,
    generic_args: Vec<Ty>,
) -> Result<Body, crate::const_eval::ConstError> {
    let (body, _callees, const_err) = build_body(cx, def, generic_args);
    match const_err {
        Some(e) => Err(e),
        None => Ok(body),
    }
}

/// 单个 (DefId, 泛型实参) 实例的 lowering;返回 (body, 发现的被调用实例,
/// const 引用求值首错)。const_err 仅在 const 求值路径([`build_for_const_eval`])
/// 被消费;运行期收集路径(`main` halt 于 const 错误前)忽略之。
fn build_body(cx: &QueryCtx<'_>, def: DefId, generic_args: Vec<Ty>) -> BuildOutput {
    let krate = cx.hir_crate();
    let res = cx.resolutions();
    let item = krate.item(def);
    let (body_id, output_ty) = match &item.kind {
        hir::ItemKind::Fn(decl) => {
            let bid = decl.body.expect("无 body 的 fn 不入收集");
            (bid, cx.fn_sig(def).output.subst(&generic_args))
        }
        hir::ItemKind::Const { body, .. } | hir::ItemKind::Static { body, .. } => {
            (*body, cx.type_of(def).subst(&generic_args))
        }
        _ => unreachable!("MIR lowering 只对 fn/const/static 实例调用"),
    };
    let hir_body = krate.body(body_id);
    let tcr = cx.check_body(body_id);

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
            ty: output_ty,
            name: None,
            span: item.span,
            shared: false,
            array_len: None,
        }],
        blocks: Vec::new(),
        local_map: vec![None; tb.locals.len()],
        cur: BlockIdx(0),
        loops: Vec::new(),
        drop_scopes: Vec::new(),
        callees: Vec::new(),
        const_err: None,
    };
    b.new_block();
    // 根 scope:参数归此(RXS-0052;函数退出时 drop)
    b.push_scope();

    // 参数:绑定模式直接落位 _1..=_n(复杂模式作用面外)
    let mut arg_count = 0;
    for p in &tb.params {
        match &p.kind {
            tbir::PatKind::Binding { local, sub: None } => {
                let idx = b.declare_local(*local, &tb);
                arg_count += 1;
                debug_assert_eq!(idx.0 as usize, arg_count);
                b.register_drop(idx);
            }
            _ => {
                b.unsupported(p.span, "non-binding parameter pattern");
                arg_count += 1;
            }
        }
    }
    // 其余局部(let 绑定)按声明序落位(归属经 register 在 let 降级时入 scope)
    for i in 0..tb.locals.len() {
        if b.local_map[i].is_none() {
            b.declare_local(LocalId(i as u32), &tb);
        }
    }

    let v = b.op_of(&tb.value);
    let span = tb.value.span;
    b.assign(Place::local(LocalIdx(0)), Rvalue::Use(v), span);
    // 根 scope drop(参数;返回值已 move 入 _0,其 drop 经 elaboration 消去)
    b.pop_scope_and_drop(span);
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
    let color = match &item.kind {
        hir::ItemKind::Fn(decl) => decl.color,
        _ => crate::ast::FnColor::Host,
    };
    (
        Body {
            def,
            symbol: mangle(&item.name, def, &generic_args),
            color,
            generic_args,
            locals: b.locals,
            arg_count,
            blocks,
            span: item.span,
        },
        b.callees,
        b.const_err,
    )
}

/// 构建中的基本块(终结子后置)。
struct BlockBuf {
    stmts: Vec<Statement>,
    term: Option<Terminator>,
}

/// drop scope(RXS-0052/RXS-0055:块 scope 与语句临时 scope;退出时按
/// 声明逆序对登记的 needs-drop local/temp 落 Drop 终结子,Phase A 无条件,
/// move/init 感知由 [`crate::drop_elab`] 在 MIR 上精化)。
struct DropScope {
    /// 本 scope 登记的 needs-drop local(声明序;emit 时逆序)。
    locals: Vec<LocalIdx>,
}

/// 循环帧:continue/break 目标 + 进入时的 drop scope 栈深(break/continue
/// 跨出的 scope 在转移前 drop,RXS-0052)。
struct LoopFrame {
    cont: BlockIdx,
    brk: BlockIdx,
    scope_depth: usize,
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
    /// 循环帧栈(continue/break 目标 + scope 深度)。
    loops: Vec<LoopFrame>,
    /// drop scope 栈(RXS-0052;栈顶 = 当前最内 scope)。
    drop_scopes: Vec<DropScope>,
    /// 发现的被调用实例(单态化收集输出)。
    callees: Vec<(DefId, Vec<Ty>)>,
    /// const 引用求值首错(M3.4;仅 const 求值路径消费,RXS-0063)。
    const_err: Option<crate::const_eval::ConstError>,
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
        let shared = decl.shared;
        // 数组长度字面量 span → u64(M5.3;device shared/array codegen 定形)。
        let array_len = decl.array_len.and_then(|sp| {
            let text = self.lit_text(sp).to_owned();
            parse_int(&text, None).and_then(|v| u64::try_from(v).ok())
        });
        let idx = LocalIdx(self.locals.len() as u32);
        self.locals.push(Local {
            ty,
            name: Some(decl.name.clone()),
            span: decl.span,
            shared,
            array_len,
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
            shared: false,
            array_len: None,
        });
        // needs-drop 临时归当前(语句)scope(RXS-0056;move 出者由 elaboration
        // 消去 drop)
        self.register_drop(idx);
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

    // -- drop scope(RXS-0052/RXS-0055)----------------------------------------

    fn push_scope(&mut self) {
        self.drop_scopes.push(DropScope { locals: Vec::new() });
    }

    /// 登记一个 MIR local 到当前最内 scope(仅 needs-drop 类型参与;
    /// move/init 感知由 elaboration 精化,Phase A 无条件)。
    fn register_drop(&mut self, idx: LocalIdx) {
        let ty = self.locals[idx.0 as usize].ty.clone();
        if crate::ty::needs_drop(&self.krate, &ty)
            && let Some(s) = self.drop_scopes.last_mut()
        {
            s.locals.push(idx);
        }
    }

    /// 在当前块落一条 Drop 终结子(place 整体)并切到后继块。
    fn emit_drop(&mut self, idx: LocalIdx, span: Span) {
        let next = self.new_block();
        self.terminate(
            TerminatorKind::Drop {
                place: Place::local(idx),
                next,
            },
            span,
        );
        self.switch_to(next);
    }

    /// 对一组 local 按逆序落 Drop(scope 退出序)。
    fn emit_drops(&mut self, locals: &[LocalIdx], span: Span) {
        for &l in locals.iter().rev() {
            self.emit_drop(l, span);
        }
    }

    /// 弹出当前 scope 并落其 drop(正常块/函数退出)。
    fn pop_scope_and_drop(&mut self, span: Span) {
        let scope = self.drop_scopes.pop().expect("drop scope 栈非空");
        self.emit_drops(&scope.locals, span);
    }

    /// 跨出转移(return/break/continue):对 `[keep_depth, top]` 区间的 scope
    /// 按由内向外逆序落 drop(不弹栈——后续切到死块,词法 scope 仍在)。
    fn emit_unwind_drops(&mut self, keep_depth: usize, span: Span) {
        let locals: Vec<LocalIdx> = self.drop_scopes[keep_depth..]
            .iter()
            .rev()
            .flat_map(|s| s.locals.iter().rev().copied())
            .collect();
        self.emit_drops(&locals, span);
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
            // `View`/`ViewMut` 容器索引(M4.2,RXS-0071):base 为地址空间指针,
            // 偏移 index(usize)得元素 place。数组(`shared let [T; N]` 等,M5.3)
            // 索引同样产 place(device codegen 落 addrspace 3/5 数组 gep)。其余非
            // 容器索引作用面外(op_of 报 RX6001)。
            tbir::ExprKind::Index { base, index } => {
                let bt = self.ty_of(base);
                if !self.is_view_ty(&bt) && !matches!(bt, Ty::Array(_)) {
                    return None;
                }
                let mut p = self.place_of_or_temp(base);
                let idx_local = self.index_local(index);
                p.proj.push(ProjElem::Index(idx_local));
                Some(p)
            }
            _ => None,
        }
    }

    /// 类型是否为 `View`/`ViewMut` 族容器(M4.2,RXS-0071;索引 place 化判定)。
    fn is_view_ty(&self, ty: &Ty) -> bool {
        matches!(ty, Ty::Adt(d, _) if self.res.lang_items.view_mutable(*d).is_some())
    }

    /// 索引下标物化为 usize local(`ProjElem::Index` 载荷)。
    fn index_local(&mut self, index: &tbir::Expr) -> LocalIdx {
        let op = self.op_of(index);
        if let Operand::Copy(p) | Operand::Move(p) = &op
            && p.proj.is_empty()
        {
            return p.local;
        }
        let t = self.temp(Ty::Prim(PrimTy::Usize), index.span);
        self.assign(Place::local(t), Rvalue::Use(op), index.span);
        t
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

    /// 类型零值常量(const 求值失败时的占位;错误经 const_err 上报)。
    fn const_zero(&self, ty: &Ty) -> Operand {
        match ty {
            Ty::Prim(p)
                if matches!(
                    p,
                    PrimTy::I8
                        | PrimTy::I16
                        | PrimTy::I32
                        | PrimTy::I64
                        | PrimTy::U8
                        | PrimTy::U16
                        | PrimTy::U32
                        | PrimTy::U64
                        | PrimTy::Usize
                ) =>
            {
                Operand::Const(Const::Int(0, *p))
            }
            Ty::Prim(p @ (PrimTy::F32 | PrimTy::F64)) => Operand::Const(Const::Float(0.0, *p)),
            Ty::Prim(PrimTy::Bool) => Operand::Const(Const::Bool(false)),
            _ => Operand::Const(Const::Unit),
        }
    }

    /// 按值使用 place(RXS-0053 move 时点):Copy 类型复制,非 Copy move。
    fn consume(&self, place: Place, ty: &Ty) -> Operand {
        if crate::ty::is_copy(&self.krate, ty) {
            Operand::Copy(place)
        } else {
            Operand::Move(place)
        }
    }

    /// rvalue 物化到 temp 并按值返回(Copy/Move 按类型裁决,RXS-0053)。
    fn rvalue_to_op(&mut self, rv: Rvalue, ty: Ty, span: Span) -> Operand {
        let t = self.temp(ty.clone(), span);
        self.assign(Place::local(t), rv, span);
        self.consume(Place::local(t), &ty)
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
                Some(p) => {
                    let ty = self.ty_of(e);
                    self.consume(p, &ty)
                }
                None => self.unsupported(e.span, "unresolved local"),
            },
            tbir::ExprKind::Def(d) => {
                // const item / 关联 const:经 const 求值内联为常量(RXS-0062/0063);
                // 裸 fn 引用 / static 引用仍作用面外(fn 指针/全局随 M4+)。
                let kind = self.res.defs.get(d.0 as usize).map(|i| i.kind);
                if matches!(
                    kind,
                    Some(hir::DefKind::Const) | Some(hir::DefKind::AssocConst)
                ) {
                    let ty = self.ty_of(e);
                    match self.cx.eval_const(*d) {
                        Ok(v) => Operand::Const(v.to_mir_const()),
                        Err(err) => {
                            // 错误经 const_err 上报(const 求值路径);运行期路径下
                            // driver 已在 mir 前 halt,占位不被消费
                            if self.const_err.is_none() {
                                self.const_err = Some(err);
                            }
                            self.const_zero(&ty)
                        }
                    }
                } else {
                    self.unsupported(e.span, "value path (fn/static reference)")
                }
            }
            tbir::ExprKind::Unary {
                op: UnOp::Deref, ..
            } => match self.place_of(e) {
                // 非 Copy 经解引用按值使用 → Move 算子落 MIR,合法性由
                // move/init 检查裁决(RXS-0053 RX4003)
                Some(p) => {
                    let ty = self.ty_of(e);
                    self.consume(p, &ty)
                }
                None => self.unsupported(e.span, "deref of non-place"),
            },
            tbir::ExprKind::Unary { op, expr } => {
                let ty = self.ty_of(e);
                let o = self.op_of(expr);
                self.rvalue_to_op(Rvalue::UnaryOp(*op, o), ty, e.span)
            }
            tbir::ExprKind::Borrow { mutable, expr } => {
                let ty = self.ty_of(e);
                let p = self.place_of_or_temp(expr);
                let kind = if *mutable {
                    BorrowKind::Mut
                } else {
                    BorrowKind::Shared
                };
                self.rvalue_to_op(Rvalue::Ref(kind, p), ty, e.span)
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
            tbir::ExprKind::DeviceCall(intr) => self.lower_device_call(e, *intr),
            tbir::ExprKind::DeviceMathCall { op, is_f32, args } => {
                self.lower_device_math_call(e, *op, *is_f32, args)
            }
            tbir::ExprKind::Field { .. } => match self.place_of(e) {
                Some(p) => {
                    let ty = self.ty_of(e);
                    self.consume(p, &ty)
                }
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
                self.loops.push(LoopFrame {
                    cont: head,
                    brk: exit,
                    scope_depth: self.drop_scopes.len(),
                });
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
                self.loops.push(LoopFrame {
                    cont: head,
                    brk: exit,
                    scope_depth: self.drop_scopes.len(),
                });
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
                // 跨出全部活动 scope(返回值已 move 入 _0,其 drop 经 elaboration 消去)
                self.emit_unwind_drops(0, e.span);
                self.terminate(TerminatorKind::Return, e.span);
                let dead = self.new_block();
                self.switch_to(dead);
                Operand::Const(Const::Unit)
            }
            tbir::ExprKind::Break => {
                if let Some(frame) = self.loops.last() {
                    let (exit, depth) = (frame.brk, frame.scope_depth);
                    self.emit_unwind_drops(depth, e.span);
                    self.terminate(TerminatorKind::Goto(exit), e.span);
                    let dead = self.new_block();
                    self.switch_to(dead);
                    Operand::Const(Const::Unit)
                } else {
                    self.unsupported(e.span, "break outside loop")
                }
            }
            tbir::ExprKind::Continue => {
                if let Some(frame) = self.loops.last() {
                    let (head, depth) = (frame.cont, frame.scope_depth);
                    self.emit_unwind_drops(depth, e.span);
                    self.terminate(TerminatorKind::Goto(head), e.span);
                    let dead = self.new_block();
                    self.switch_to(dead);
                    Operand::Const(Const::Unit)
                } else {
                    self.unsupported(e.span, "continue outside loop")
                }
            }
            // `View`/`ViewMut` 索引(M4.2,RXS-0071):place 化后按值读;非 View
            // 容器索引 place_of 返回 None → RX6001(host 数组索引作用面外)。
            tbir::ExprKind::Index { .. } => match self.place_of(e) {
                Some(p) => {
                    let ty = self.ty_of(e);
                    self.consume(p, &ty)
                }
                None => self.unsupported(e.span, "indexing"),
            },
            // ---- M3.1 作用面外(RX6001;清单留痕 M3_PLAN §1 修订行) ----
            tbir::ExprKind::BreakValue(_) => self.unsupported(e.span, "break with value"),
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
            self.consume(Place::local(dest), &ret_ty)
        }
    }

    /// device 线程上下文 intrinsic(M4.2,RXS-0072):落 `CallTarget::DeviceIntrinsic`
    /// 终结子(无实参;返回 usize / unit),device codegen 展开为 sreg/barrier。
    fn lower_device_call(&mut self, e: &tbir::Expr, intr: crate::hir::DeviceIntrinsic) -> Operand {
        let ret_ty = self.ty_of(e);
        let dest = self.temp(ret_ty.clone(), e.span);
        let next = self.new_block();
        self.terminate(
            TerminatorKind::Call {
                target: CallTarget::DeviceIntrinsic(intr),
                args: Vec::new(),
                dest: Place::local(dest),
                next,
            },
            e.span,
        );
        self.switch_to(next);
        if ret_ty.is_unit() {
            Operand::Const(Const::Unit)
        } else {
            self.consume(Place::local(dest), &ret_ty)
        }
    }

    /// device 数学 intrinsic(M5.3,RXS-0081/0082):落 `CallTarget::Libdevice`
    /// 终结子(args = receiver + 方法实参;返回元素类型),device codegen 展开为
    /// 对保留的外部符号 `__nv_*` 的 `call`,经 libdevice bc 链接解析。
    fn lower_device_math_call(
        &mut self,
        e: &tbir::Expr,
        op: crate::hir::DeviceMathFn,
        is_f32: bool,
        args: &[tbir::Expr],
    ) -> Operand {
        let symbol = op.nv_symbol(is_f32);
        let ops: Vec<Operand> = args.iter().map(|a| self.op_of(a)).collect();
        let ret_ty = self.ty_of(e);
        let dest = self.temp(ret_ty.clone(), e.span);
        let next = self.new_block();
        self.terminate(
            TerminatorKind::Call {
                target: CallTarget::Libdevice { symbol },
                args: ops,
                dest: Place::local(dest),
                next,
            },
            e.span,
        );
        self.switch_to(next);
        self.consume(Place::local(dest), &ret_ty)
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
            Some(d) => self.consume(Place::local(d), &ty),
            None => Operand::Const(Const::Unit),
        }
    }

    fn lower_block(&mut self, b: &tbir::Block) -> Operand {
        // 块 scope(RXS-0052):let 绑定归此,块退出时逆序 drop
        self.push_scope();
        for stmt in &b.stmts {
            // 语句临时 scope(RXS-0056):语句内物化的 needs-drop 临时,语句末 drop
            self.push_scope();
            match stmt {
                tbir::Stmt::Let { pat, init } => match (&pat.kind, init) {
                    (tbir::PatKind::Binding { local, sub: None }, Some(init)) => {
                        let v = self.op_of(init);
                        let Some(idx) = self.local_map.get(local.0 as usize).copied().flatten()
                        else {
                            let _ = self.unsupported(pat.span, "unresolved binding");
                            self.pop_scope_and_drop(pat.span);
                            continue;
                        };
                        self.assign(Place::local(idx), Rvalue::Use(v), init.span);
                        // 绑定归块 scope(语句临时 scope 之下一层)
                        self.register_drop_in_block(idx);
                    }
                    (tbir::PatKind::Binding { local, sub: None }, None) => {
                        // 延迟绑定:首次赋值落位;归块 scope(条件初始化由 elaboration
                        // 经 drop flag 裁决)
                        if let Some(idx) = self.local_map.get(local.0 as usize).copied().flatten() {
                            self.register_drop_in_block(idx);
                        }
                    }
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
            // 语句末:drop 本语句临时(RXS-0056)
            self.pop_scope_and_drop(stmt_span(stmt));
        }
        // 尾表达式:临时归独立语句 scope(块值 move 出者由 elaboration 消去)
        let tail = match &b.tail {
            Some(t) => {
                self.push_scope();
                let v = self.op_of(t);
                self.pop_scope_and_drop(t.span);
                v
            }
            None => Operand::Const(Const::Unit),
        };
        // 块退出:drop 块 scope 的 let 绑定(声明逆序,RXS-0052)
        self.pop_scope_and_drop(b.span);
        tail
    }

    /// 把 local 登记到块 scope(当前栈顶是语句临时 scope,块 scope 在其下)。
    fn register_drop_in_block(&mut self, idx: LocalIdx) {
        let ty = self.locals[idx.0 as usize].ty.clone();
        if crate::ty::needs_drop(&self.krate, &ty) {
            let n = self.drop_scopes.len();
            debug_assert!(n >= 2, "块 scope + 语句 scope 应同时在栈");
            self.drop_scopes[n - 2].locals.push(idx);
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
            Some(d) => self.consume(Place::local(d), &ty),
            None => Operand::Const(Const::Unit),
        }
    }

    /// 模式测试 + 绑定提取:测试失败跳 `fail`,成功落入当前块尾。
    fn lower_pat_test(&mut self, pat: &tbir::Pat, place: Place, fail: BlockIdx) {
        match &pat.kind {
            tbir::PatKind::Wild => {}
            tbir::PatKind::Binding { local, sub } => {
                if let Some(idx) = self.local_map.get(local.0 as usize).copied().flatten() {
                    // 绑定提取 = 按值使用(RXS-0053:非 Copy 从 scrutinee move 出)
                    let ty = pat.ty.subst(&self.substs);
                    let v = self.consume(place.clone(), &ty);
                    self.assign(Place::local(idx), Rvalue::Use(v), pat.span);
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
                    (true, Operand::Const(Const::Int(v, p))) => Operand::Const(Const::Int(-v, p)),
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

/// 语句 span(语句临时 scope drop 的诊断锚点)。
fn stmt_span(stmt: &tbir::Stmt) -> Span {
    match stmt {
        tbir::Stmt::Let { pat, init } => init.as_ref().map(|e| e.span).unwrap_or(pat.span),
        tbir::Stmt::Expr(e) => e.span,
    }
}

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
        assert!(
            text.contains("discriminant("),
            "desugar match 缺失:\n{text}"
        );
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

    /// drop elaboration:scope 退出按声明逆序落 Drop;move 出者消去
    /// (RXS-0052/RXS-0055)。
    //@ spec: RXS-0055
    #[test]
    fn drop_elaboration_orders_and_elides() {
        let src = "struct A {}\nimpl Drop for A {\n    fn drop(&mut self) {}\n}\nstruct B {}\nimpl Drop for B {\n    fn drop(&mut self) {}\n}\nfn eat(a: A) {}\nfn main() {\n    let a = A {};\n    let b = B {};\n    eat(a);\n}";
        let (text, codes) = mir_text(src);
        assert!(codes.is_empty(), "意外诊断: {codes:?}");
        // main:a move 入 eat → a 的 drop 消去;b 保留无条件 drop
        let main = text
            .split("fn ")
            .find(|s| s.starts_with("main("))
            .expect("main body");
        assert_eq!(
            main.matches("drop(").count(),
            1,
            "main 仅 b 应 drop:\n{main}"
        );
        // eat:参数 a 在函数退出 drop(definitely-owned)
        let eat = text
            .split("fn ")
            .find(|s| s.starts_with("rx_eat_"))
            .expect("eat body");
        assert_eq!(eat.matches("drop(").count(), 1, "eat 参数应 drop:\n{eat}");
    }

    /// 作用面外构造 → RX6001(M3.1 口径,不级联 ICE)。
    #[test]
    fn out_of_scope_construct_is_rx6001() {
        let src = "fn main() {\n    let _x = [1, 2, 3];\n}";
        let (_, codes) = mir_text(src);
        assert_eq!(codes, vec![6001]);
    }

    /// const item 引用经 const 求值内联为常量(M3.4,RXS-0062/0063 集成点)。
    //@ spec: RXS-0062
    #[test]
    fn const_reference_inlines_to_constant() {
        let src = "const fn sq(n: i32) -> i32 { n * n }\n\
                   const K: i32 = sq(6);\n\
                   fn main() {\n    let _x = K;\n}";
        let (text, codes) = mir_text(src);
        assert!(codes.is_empty(), "意外诊断: {codes:?}");
        // K = sq(6) = 36 编译期内联;main 无对 K 的运行期调用,直接用常量 36
        assert!(text.contains("const 36i32"), "const 未内联:\n{text}");
        assert!(
            !text.contains("rx_sq_"),
            "const fn 不应进运行期收集:\n{text}"
        );
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
