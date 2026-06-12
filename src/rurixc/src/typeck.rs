//! 类型检查 host 子集(spec 条款 RXS-0039 ~ RXS-0047,spec/types.md;07 §3)。
//!
//! - 收集:`fn_sig` / `type_of` 经 [`crate::query::QueryCtx`](provider 在本模块);
//! - 推断:body 内 HM 合一(union-find 推断变量 + 字面量数值类约束,RXS-0041);
//!   body 检查结束时按 RXS-0039 默认化(i32 / f64);
//! - 检查面 = host 子集(函数/struct/enum/泛型单态化雏形,11 §3);
//!   trait bound 仅记录不求解、方法查找仅 inherent、内建运算符不经 trait
//!   (RXS-0045/0046 的 M2.2 口径);
//! - **Err 容忍不级联**(RXS-0047):任一参与类型为 [`Ty::Err`] 时静默通过;
//!   `for`(非区间迭代器)/`?`/闭包/`loop` 值等未定语义容忍为 Err。

use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{BinOp, LitKind, LitSuffix, UnOp};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::hir::{self, BodyId, DefId, DefKind, HirId, LocalId, PrimTy, Res};
use crate::query::QueryCtx;
use crate::resolve::Resolutions;
use crate::span::Span;
use crate::ty::{FnSig, Ty, TyVid};

pub const E_MISMATCHED_TYPES: ErrorCode = ErrorCode(2001); // RX2001
pub const E_BAD_FIELD: ErrorCode = ErrorCode(2002); // RX2002
pub const E_ARG_COUNT: ErrorCode = ErrorCode(2003); // RX2003
pub const E_UNKNOWN_METHOD: ErrorCode = ErrorCode(2004); // RX2004
pub const E_NOT_CALLABLE: ErrorCode = ErrorCode(2005); // RX2005
pub const E_BAD_OPERAND: ErrorCode = ErrorCode(2006); // RX2006

// ---------------------------------------------------------------------------
// typeck 结果物化(M2.3:MIR lowering 的输入)
// ---------------------------------------------------------------------------

/// 单个 body 的类型检查产物(`check_body` query 的 memo 值)。
///
/// 全部类型在 body 检查结束时经推断引擎深度 resolve 并默认化;残留的
/// 未约束推断变量收敛为 [`Ty::Err`](容忍区,MIR lowering 按不支持处理)。
#[derive(Debug, Default)]
pub struct TypeckResults {
    /// 表达式节点 → 定型结果。
    pub expr_ty: HashMap<HirId, Ty>,
    /// 模式节点 → 绑定时的被匹配类型。
    pub pat_ty: HashMap<HirId, Ty>,
    /// 局部绑定(LocalId 索引)→ 定型结果(未绑定/容忍区为 Err)。
    pub local_ty: Vec<Ty>,
    /// 调用点(Call/MethodCall 表达式节点)→ (目标 DefId, 泛型实参)。
    /// 单态化收集的输入(D-111);非 fn-item 调用(fn 指针)不入表。
    pub call_targets: HashMap<HirId, (DefId, Vec<Ty>)>,
}

// ---------------------------------------------------------------------------
// 推断引擎(RXS-0041)
// ---------------------------------------------------------------------------

/// 数值类约束(无后缀字面量,RXS-0039)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NumClass {
    Int,
    Float,
}

#[derive(Clone, Debug)]
enum VarState {
    Unbound(Option<NumClass>),
    Bound(Ty),
}

#[derive(Default)]
struct InferCtxt {
    vars: Vec<VarState>,
}

impl InferCtxt {
    fn fresh(&mut self, class: Option<NumClass>) -> Ty {
        let id = TyVid(self.vars.len() as u32);
        self.vars.push(VarState::Unbound(class));
        Ty::Infer(id)
    }

    /// 追链到非绑定形态(浅)。
    fn shallow(&self, t: &Ty) -> Ty {
        let mut cur = t.clone();
        while let Ty::Infer(v) = cur {
            match &self.vars[v.0 as usize] {
                VarState::Bound(b) => cur = b.clone(),
                VarState::Unbound(_) => return Ty::Infer(v),
            }
        }
        cur
    }

    /// 深度解析:绑定替换;未定数值类按 RXS-0039 默认化;其余保持。
    fn resolve(&self, t: &Ty) -> Ty {
        let t = self.shallow(t);
        match t {
            Ty::Infer(v) => match self.vars[v.0 as usize] {
                VarState::Unbound(Some(NumClass::Int)) => Ty::Prim(PrimTy::I32),
                VarState::Unbound(Some(NumClass::Float)) => Ty::Prim(PrimTy::F64),
                _ => Ty::Infer(v),
            },
            Ty::Adt(d, args) => Ty::Adt(d, args.iter().map(|a| self.resolve(a)).collect()),
            Ty::Tuple(v) => Ty::Tuple(v.iter().map(|a| self.resolve(a)).collect()),
            Ty::Ref(t, m) => Ty::Ref(Box::new(self.resolve(&t)), m),
            Ty::RawPtr(t, m) => Ty::RawPtr(Box::new(self.resolve(&t)), m),
            Ty::Array(t) => Ty::Array(Box::new(self.resolve(&t))),
            Ty::Slice(t) => Ty::Slice(Box::new(self.resolve(&t))),
            Ty::FnPtr(ps, r) => Ty::FnPtr(
                ps.iter().map(|a| self.resolve(a)).collect(),
                Box::new(self.resolve(&r)),
            ),
            other => other,
        }
    }

    fn class_compatible(class: NumClass, t: &Ty) -> bool {
        match class {
            NumClass::Int => t.is_int(),
            NumClass::Float => t.is_float(),
        }
    }

    fn bind(&mut self, v: TyVid, t: Ty) -> bool {
        if let VarState::Unbound(class) = self.vars[v.0 as usize].clone() {
            if let Some(c) = class {
                match &t {
                    Ty::Infer(o) => {
                        // 合并数值类约束到另一变量
                        if let VarState::Unbound(oc) = &mut self.vars[o.0 as usize] {
                            match oc {
                                None => *oc = Some(c),
                                Some(other) if *other != c => return false,
                                _ => {}
                            }
                        }
                    }
                    _ if !Self::class_compatible(c, &t) => return false,
                    _ => {}
                }
            }
            self.vars[v.0 as usize] = VarState::Bound(t);
            true
        } else {
            unreachable!("bind 只对 unbound 变量调用")
        }
    }

    /// 合一(RXS-0041);Err 容忍(RXS-0047)。
    fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let a = self.shallow(a);
        let b = self.shallow(b);
        match (&a, &b) {
            (Ty::Err, _) | (_, Ty::Err) => true,
            (Ty::Infer(v), Ty::Infer(w)) if v == w => true,
            (Ty::Infer(v), other) => self.bind(*v, other.clone()),
            (other, Ty::Infer(v)) => self.bind(*v, other.clone()),
            (Ty::Prim(p), Ty::Prim(q)) => p == q,
            (Ty::Adt(d, xs), Ty::Adt(e, ys)) => {
                d == e
                    && xs.len() == ys.len()
                    && xs
                        .clone()
                        .iter()
                        .zip(ys.clone().iter())
                        .all(|(x, y)| self.unify(x, y))
            }
            (Ty::Tuple(xs), Ty::Tuple(ys)) => {
                xs.len() == ys.len()
                    && xs
                        .clone()
                        .iter()
                        .zip(ys.clone().iter())
                        .all(|(x, y)| self.unify(x, y))
            }
            (Ty::Ref(x, m), Ty::Ref(y, n)) => m == n && self.unify(&x.clone(), &y.clone()),
            (Ty::RawPtr(x, m), Ty::RawPtr(y, n)) => m == n && self.unify(&x.clone(), &y.clone()),
            (Ty::Array(x), Ty::Array(y)) | (Ty::Slice(x), Ty::Slice(y)) => {
                self.unify(&x.clone(), &y.clone())
            }
            (Ty::FnPtr(xs, xr), Ty::FnPtr(ys, yr)) => {
                xs.len() == ys.len()
                    && xs
                        .clone()
                        .iter()
                        .zip(ys.clone().iter())
                        .all(|(x, y)| self.unify(x, y))
                    && self.unify(&xr.clone(), &yr.clone())
            }
            (Ty::Param(i), Ty::Param(j)) => i == j,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// HIR 类型降级
// ---------------------------------------------------------------------------

/// HIR 类型 → `Ty`;`infer` 回调裁决 `_` 占位(签名给 Err 容忍,body 给 fresh)。
fn lower_hir_ty(t: &hir::Ty, infer: &mut dyn FnMut() -> Ty) -> Ty {
    match &t.kind {
        hir::TyKind::Res(res, args) => match res {
            Res::PrimTy(p) => Ty::Prim(*p),
            Res::Def(d) => Ty::Adt(*d, args.iter().map(|a| lower_hir_ty(a, infer)).collect()),
            Res::GenericParam(i) => Ty::Param(*i),
            // SelfTy/Local/Err:M2.2 容忍(SelfTy 展开随 M2.3)
            _ => Ty::Err,
        },
        hir::TyKind::Ref { mutable, inner } => {
            Ty::Ref(Box::new(lower_hir_ty(inner, infer)), *mutable)
        }
        hir::TyKind::RawPtr { mutable, inner } => {
            Ty::RawPtr(Box::new(lower_hir_ty(inner, infer)), *mutable)
        }
        hir::TyKind::Tuple(v) => Ty::Tuple(v.iter().map(|x| lower_hir_ty(x, infer)).collect()),
        hir::TyKind::Array { elem } => Ty::Array(Box::new(lower_hir_ty(elem, infer))),
        hir::TyKind::Slice(inner) => Ty::Slice(Box::new(lower_hir_ty(inner, infer))),
        hir::TyKind::FnPtr { params, ret } => Ty::FnPtr(
            params.iter().map(|x| lower_hir_ty(x, infer)).collect(),
            Box::new(
                ret.as_ref()
                    .map(|r| lower_hir_ty(r, infer))
                    .unwrap_or_else(Ty::unit),
            ),
        ),
        hir::TyKind::Infer => infer(),
        hir::TyKind::Err => Ty::Err,
    }
}

// ---------------------------------------------------------------------------
// query providers(D-203:provider 只经 QueryCtx 互访)
// ---------------------------------------------------------------------------

/// `fn_sig` provider(RXS-0040/0042)。
pub fn fn_sig_provider(cx: &QueryCtx<'_>, def: DefId) -> FnSig {
    let krate = cx.hir_crate();
    let item = krate.item(def);
    let hir::ItemKind::Fn(decl) = &item.kind else {
        return FnSig {
            generics_count: 0,
            has_self: false,
            inputs: Vec::new(),
            output: Ty::Err,
        };
    };
    let mut sig_infer = || Ty::Err; // 签名中 `_` 容忍(RXS-0040 全标注,违例语义层延后)
    let inputs = decl
        .params
        .iter()
        .filter_map(|p| p.ty.as_ref())
        .map(|t| lower_hir_ty(t, &mut sig_infer))
        .collect();
    FnSig {
        generics_count: decl.generic_params.len() as u32,
        has_self: decl.params.iter().any(|p| p.ty.is_none()),
        inputs,
        output: decl
            .ret
            .as_ref()
            .map(|t| lower_hir_ty(t, &mut sig_infer))
            .unwrap_or_else(Ty::unit),
    }
}

/// `type_of` provider(const/static 标注、ADT 自身、变体归属)。
pub fn type_of_provider(cx: &QueryCtx<'_>, def: DefId) -> Ty {
    let krate = cx.hir_crate();
    let res = cx.resolutions();
    let mut sig_infer = || Ty::Err;
    match &krate.item(def).kind {
        hir::ItemKind::Const { ty, .. } | hir::ItemKind::Static { ty, .. } => {
            lower_hir_ty(ty, &mut sig_infer)
        }
        hir::ItemKind::Struct { .. } | hir::ItemKind::Enum { .. } => Ty::Adt(def, Vec::new()),
        hir::ItemKind::Variant { .. } => res
            .variant_parents
            .get(&def)
            .map(|e| Ty::Adt(*e, Vec::new()))
            .unwrap_or(Ty::Err),
        hir::ItemKind::TypeAlias { ty } => lower_hir_ty(ty, &mut sig_infer),
        _ => Ty::Err,
    }
}

/// `check_body` provider:对单个 body 做推断与检查,诊断经 DiagCtxt;
/// 产物 [`TypeckResults`] 按节点物化(M2.3,MIR lowering 消费)。
pub fn check_body_provider(cx: &QueryCtx<'_>, body_id: BodyId) -> TypeckResults {
    let krate = cx.hir_crate();
    let res = cx.resolutions();
    let body = krate.body(body_id);
    let owner = krate.item(body.owner);

    let mut tck = Tck {
        cx,
        krate: Rc::clone(&krate),
        res: Rc::clone(&res),
        infcx: InferCtxt::default(),
        locals: vec![None; body.locals.len()],
        ret_ty: Ty::Err,
        results: TypeckResults::default(),
    };

    // 期望返回类型与参数绑定
    match &owner.kind {
        hir::ItemKind::Fn(decl) => {
            let sig = cx.fn_sig(body.owner);
            tck.ret_ty = sig.output.clone();
            // self 接收者:反查所属 inherent impl 的 self 类型(M2.2 宽松)
            let self_ty = if sig.has_self {
                tck.impl_self_ty(body.owner)
            } else {
                Ty::Err
            };
            let mut input_iter = sig.inputs.iter();
            for (i, p) in decl.params.iter().enumerate() {
                let ty = if p.ty.is_none() {
                    self_ty.clone()
                } else {
                    input_iter.next().cloned().unwrap_or(Ty::Err)
                };
                if let Some(pat) = body.params.get(i) {
                    tck.bind_pat(pat, &ty);
                }
            }
        }
        hir::ItemKind::Const { ty, .. } | hir::ItemKind::Static { ty, .. } => {
            let mut sig_infer = || Ty::Err;
            tck.ret_ty = lower_hir_ty(ty, &mut sig_infer);
        }
        _ => {}
    }

    let found = tck.check_expr(&body.value);
    let ret = tck.ret_ty.clone();
    tck.demand(body.value.span, &ret, &found);

    // 物化:全部记录类型经推断引擎 resolve(含数值类默认化),残留推断变量收敛为 Err
    let infcx = tck.infcx;
    let finalize = |t: &Ty| -> Ty { strip_infer(&infcx.resolve(t)) };
    let mut results = tck.results;
    for t in results.expr_ty.values_mut() {
        *t = finalize(t);
    }
    for t in results.pat_ty.values_mut() {
        *t = finalize(t);
    }
    results.local_ty = tck
        .locals
        .iter()
        .map(|t| t.as_ref().map(&finalize).unwrap_or(Ty::Err))
        .collect();
    for (_, args) in results.call_targets.values_mut() {
        for t in args.iter_mut() {
            *t = finalize(t);
        }
    }
    results
}

/// 残留未约束推断变量 → Err(物化收敛,RXS-0047 容忍区)。
fn strip_infer(t: &Ty) -> Ty {
    match t {
        Ty::Infer(_) => Ty::Err,
        Ty::Adt(d, args) => Ty::Adt(*d, args.iter().map(strip_infer).collect()),
        Ty::Tuple(v) => Ty::Tuple(v.iter().map(strip_infer).collect()),
        Ty::Ref(x, m) => Ty::Ref(Box::new(strip_infer(x)), *m),
        Ty::RawPtr(x, m) => Ty::RawPtr(Box::new(strip_infer(x)), *m),
        Ty::Array(x) => Ty::Array(Box::new(strip_infer(x))),
        Ty::Slice(x) => Ty::Slice(Box::new(strip_infer(x))),
        Ty::FnPtr(ps, r) => Ty::FnPtr(
            ps.iter().map(strip_infer).collect(),
            Box::new(strip_infer(r)),
        ),
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// body 检查器
// ---------------------------------------------------------------------------

struct Tck<'a, 'q> {
    cx: &'a QueryCtx<'q>,
    krate: Rc<hir::Crate>,
    res: Rc<Resolutions>,
    infcx: InferCtxt,
    locals: Vec<Option<Ty>>,
    ret_ty: Ty,
    results: TypeckResults,
}

impl Tck<'_, '_> {
    fn diag(&self) -> &DiagCtxt {
        self.cx.diag()
    }

    fn render(&self, t: &Ty) -> String {
        self.infcx.resolve(t).render(&self.res)
    }

    // -- 诊断(RXS-0047) ------------------------------------------------------

    fn err_mismatch(&self, span: Span, expected: &Ty, found: &Ty) {
        self.diag()
            .struct_error(E_MISMATCHED_TYPES, "typeck.mismatched_types")
            .arg("expected", self.render(expected))
            .arg("found", self.render(found))
            .span_label(span, format!("expected {}", self.render(expected)))
            .emit();
    }

    fn err_bad_field(&self, span: Span, kind: &str, field: &str, ty: &Ty) {
        self.diag()
            .struct_error(E_BAD_FIELD, "typeck.bad_field")
            .arg("kind", kind)
            .arg("field", format!("`{field}`"))
            .arg("ty", self.render(ty))
            .span_label(span, format!("{kind} field `{field}`"))
            .emit();
    }

    fn err_arg_count(&self, span: Span, expected: usize, found: usize) {
        self.diag()
            .struct_error(E_ARG_COUNT, "typeck.arg_count_mismatch")
            .arg("expected", expected.to_string())
            .arg("found", found.to_string())
            .span_label(span, format!("expected {expected} argument(s)"))
            .emit();
    }

    fn err_unknown_method(&self, span: Span, method: &str, ty: &Ty) {
        self.diag()
            .struct_error(E_UNKNOWN_METHOD, "typeck.unknown_method")
            .arg("method", format!("`{method}`"))
            .arg("ty", self.render(ty))
            .span_label(span, "method not found")
            .emit();
    }

    fn err_not_callable(&self, span: Span, ty: &Ty) {
        self.diag()
            .struct_error(E_NOT_CALLABLE, "typeck.not_callable")
            .arg("ty", self.render(ty))
            .span_label(span, "not callable")
            .emit();
    }

    fn err_bad_operand(&self, span: Span, op: &str, ty: &Ty) {
        self.diag()
            .struct_error(E_BAD_OPERAND, "typeck.bad_operand")
            .arg("op", format!("`{op}`"))
            .arg("ty", self.render(ty))
            .span_label(span, "invalid operand type")
            .emit();
    }

    /// 合一并按 RX2001 报错(Err 容忍内建于 unify)。
    fn demand(&mut self, span: Span, expected: &Ty, found: &Ty) {
        if !self.infcx.unify(expected, found) {
            self.err_mismatch(span, expected, found);
        }
    }

    // -- 辅助 -----------------------------------------------------------------

    fn ty_from_hir(&mut self, t: &hir::Ty) -> Ty {
        let infcx = &mut self.infcx;
        lower_hir_ty(t, &mut || infcx.fresh(None))
    }

    /// 反查 owner(AssocFn)所属 inherent impl 的 self 类型。
    fn impl_self_ty(&self, owner: DefId) -> Ty {
        for item in &self.krate.items {
            if let hir::ItemKind::Impl { self_res, items } = &item.kind
                && items.contains(&owner)
            {
                if let Res::Def(d) = self_res {
                    return Ty::Adt(*d, Vec::new());
                }
                return Ty::Err;
            }
        }
        Ty::Err
    }

    fn fields_of(&self, def: DefId) -> Option<&[hir::FieldDef]> {
        match &self.krate.item(def).kind {
            hir::ItemKind::Struct { fields } | hir::ItemKind::Variant { fields } => Some(fields),
            _ => None,
        }
    }

    /// ADT 构造结果类型:struct → 自身;variant → 父 enum。
    fn ctor_result(&self, def: DefId, args: Vec<Ty>) -> Ty {
        match self.krate.item(def).kind {
            hir::ItemKind::Variant { .. } => self
                .res
                .variant_parents
                .get(&def)
                .map(|e| Ty::Adt(*e, args))
                .unwrap_or(Ty::Err),
            _ => Ty::Adt(def, args),
        }
    }

    /// 字段表中 Param 的最大序号 + 1(泛型 ADT 的实例化槽位数,MVP 推定)。
    fn generic_slots(&self, fields: &[hir::FieldDef]) -> u32 {
        fn max_param(t: &Ty, cur: &mut u32) {
            match t {
                Ty::Param(i) => *cur = (*cur).max(*i + 1),
                Ty::Adt(_, v) | Ty::Tuple(v) | Ty::FnPtr(v, _) => {
                    for x in v {
                        max_param(x, cur);
                    }
                    if let Ty::FnPtr(_, r) = t {
                        max_param(r, cur);
                    }
                }
                Ty::Ref(x, _) | Ty::RawPtr(x, _) | Ty::Array(x) | Ty::Slice(x) => max_param(x, cur),
                _ => {}
            }
        }
        let mut n = 0;
        let mut sig_infer = || Ty::Err;
        for f in fields {
            max_param(&lower_hir_ty(&f.ty, &mut sig_infer), &mut n);
        }
        n
    }

    /// 返回 (实例化后形参, 返回类型, 泛型实参槽位)——槽位供调用点记录(单态化,D-111)。
    fn instantiate_sig(&mut self, sig: &FnSig) -> (Vec<Ty>, Ty, Vec<Ty>) {
        if sig.generics_count == 0 {
            return (sig.inputs.clone(), sig.output.clone(), Vec::new());
        }
        let fresh: Vec<Ty> = (0..sig.generics_count)
            .map(|_| self.infcx.fresh(None))
            .collect();
        (
            sig.inputs.iter().map(|t| t.subst(&fresh)).collect(),
            sig.output.subst(&fresh),
            fresh,
        )
    }

    /// 解一层引用(字段访问/方法接收者,RXS-0044/0046)。
    fn autoderef(&self, t: &Ty) -> Ty {
        match self.infcx.shallow(t) {
            Ty::Ref(inner, _) => self.infcx.shallow(&inner),
            other => other,
        }
    }

    fn numeric_guard(&mut self, span: Span, op: &str, t: &Ty, ints_only: bool) {
        let r = self.infcx.resolve(t);
        match &r {
            Ty::Err | Ty::Infer(_) | Ty::Param(_) => {}
            _ if r.is_int() => {}
            _ if r.is_float() && !ints_only => {}
            _ => self.err_bad_operand(span, op, &r),
        }
    }

    // -- 模式绑定(参数 / let / match 臂) --------------------------------------

    fn bind_pat(&mut self, pat: &hir::Pat, ty: &Ty) {
        self.results.pat_ty.insert(pat.hir_id, ty.clone());
        match &pat.kind {
            hir::PatKind::Binding { local } => self.set_local(*local, ty.clone()),
            hir::PatKind::Wild | hir::PatKind::Lit | hir::PatKind::Range | hir::PatKind::Err => {}
            hir::PatKind::At { local, pat } => {
                self.set_local(*local, ty.clone());
                self.bind_pat(pat, ty);
            }
            hir::PatKind::Ref { pat } => {
                let inner = match self.infcx.shallow(ty) {
                    Ty::Ref(t, _) => *t,
                    _ => Ty::Err,
                };
                self.bind_pat(pat, &inner);
            }
            hir::PatKind::Tuple(pats) => {
                let elems = match self.infcx.shallow(ty) {
                    Ty::Tuple(v) if v.len() == pats.len() => v,
                    _ => vec![Ty::Err; pats.len()],
                };
                for (p, t) in pats.iter().zip(elems) {
                    self.bind_pat(p, &t);
                }
            }
            hir::PatKind::Slice(pats) => {
                let elem = match self.infcx.shallow(ty) {
                    Ty::Array(t) | Ty::Slice(t) => *t,
                    _ => Ty::Err,
                };
                for p in pats {
                    self.bind_pat(p, &elem);
                }
            }
            hir::PatKind::Res(_) => {}
            hir::PatKind::TupleStruct { res, elems } => {
                let field_tys = self.ctor_field_tys(res, ty);
                for (i, p) in elems.iter().enumerate() {
                    self.bind_pat(p, field_tys.get(i).unwrap_or(&Ty::Err));
                }
            }
            hir::PatKind::Struct { res, fields, .. } => {
                let named = self.named_field_tys(res, ty);
                for (name, sub) in fields {
                    let t = named
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, t)| t.clone())
                        .unwrap_or(Ty::Err);
                    if let Some(p) = sub {
                        self.bind_pat(p, &t);
                    }
                }
            }
        }
    }

    fn set_local(&mut self, local: LocalId, ty: Ty) {
        if let Some(slot) = self.locals.get_mut(local.0 as usize) {
            *slot = Some(ty);
        }
    }

    /// 模式中构造器字段类型(以被匹配值的 Adt 实参实例化)。
    fn ctor_field_tys(&mut self, res: &Res, scrutinee: &Ty) -> Vec<Ty> {
        let Res::Def(d) = res else { return Vec::new() };
        let Some(fields) = self.fields_of(*d) else {
            return Vec::new();
        };
        let args = match self.infcx.shallow(scrutinee) {
            Ty::Adt(_, args) => args,
            _ => Vec::new(),
        };
        let mut sig_infer = || Ty::Err;
        fields
            .iter()
            .map(|f| lower_hir_ty(&f.ty, &mut sig_infer).subst(&args))
            .collect()
    }

    fn named_field_tys(&mut self, res: &Res, scrutinee: &Ty) -> Vec<(String, Ty)> {
        let Res::Def(d) = res else { return Vec::new() };
        let Some(fields) = self.fields_of(*d) else {
            return Vec::new();
        };
        let args = match self.infcx.shallow(scrutinee) {
            Ty::Adt(_, args) => args,
            _ => Vec::new(),
        };
        let mut sig_infer = || Ty::Err;
        fields
            .iter()
            .map(|f| {
                (
                    f.name.clone(),
                    lower_hir_ty(&f.ty, &mut sig_infer).subst(&args),
                )
            })
            .collect()
    }

    // -- 表达式检查(RXS-0042 ~ RXS-0046) --------------------------------------

    fn check_block(&mut self, b: &hir::Block) -> Ty {
        let mut diverged = false;
        for stmt in &b.stmts {
            match stmt {
                hir::Stmt::Item(_) => {} // 嵌套 item 的 body 经 check_crate 全集遍历
                hir::Stmt::Let { pat, ty, init, .. } => {
                    let ann = ty.as_ref().map(|t| self.ty_from_hir(t));
                    let init_ty = init.as_ref().map(|e| (e.span, self.check_expr(e)));
                    let bound = match (ann, init_ty) {
                        (Some(a), Some((span, i))) => {
                            self.demand(span, &a, &i);
                            a
                        }
                        (Some(a), None) => a,
                        (None, Some((_, i))) => i,
                        (None, None) => self.infcx.fresh(None),
                    };
                    self.bind_pat(pat, &bound);
                }
                hir::Stmt::Expr(e) => {
                    let _ = self.check_expr(e);
                    // 发散语句后的块值容忍(never 形态随 M2.3 评估)
                    if matches!(
                        e.kind,
                        hir::ExprKind::Return(_)
                            | hir::ExprKind::Break(_)
                            | hir::ExprKind::Continue
                    ) {
                        diverged = true;
                    }
                }
            }
        }
        match &b.tail {
            Some(t) => self.check_expr(t),
            None if diverged => Ty::Err,
            None => Ty::unit(),
        }
    }

    fn check_expr(&mut self, e: &hir::Expr) -> Ty {
        let t = self.check_expr_kind(e);
        // 物化:按节点落表(含推断变量,body 收尾统一 resolve)
        self.results.expr_ty.insert(e.hir_id, t.clone());
        t
    }

    fn check_expr_kind(&mut self, e: &hir::Expr) -> Ty {
        match &e.kind {
            hir::ExprKind::Lit(l) => self.lit_ty(l),
            hir::ExprKind::Res(r) => self.res_value_ty(r),
            hir::ExprKind::Unary { op, expr } => {
                let t = self.check_expr(expr);
                match op {
                    UnOp::Neg => {
                        self.numeric_guard(e.span, "-", &t, false);
                        t
                    }
                    UnOp::Not => {
                        let r = self.infcx.resolve(&t);
                        match &r {
                            Ty::Prim(PrimTy::Bool) | Ty::Err | Ty::Infer(_) => {}
                            _ if r.is_int() => {}
                            _ => self.err_bad_operand(e.span, "!", &r),
                        }
                        t
                    }
                    UnOp::Deref => match self.infcx.shallow(&t) {
                        Ty::Ref(inner, _) | Ty::RawPtr(inner, _) => *inner,
                        Ty::Err | Ty::Infer(_) => Ty::Err,
                        other => {
                            self.err_bad_operand(e.span, "*", &other);
                            Ty::Err
                        }
                    },
                }
            }
            hir::ExprKind::Borrow { mutable, expr } => {
                let t = self.check_expr(expr);
                Ty::Ref(Box::new(t), *mutable)
            }
            hir::ExprKind::Binary { op, lhs, rhs } => self.check_binary(e.span, *op, lhs, rhs),
            hir::ExprKind::Assign { op, lhs, rhs } => {
                let lt = self.check_expr(lhs);
                let rt = self.check_expr(rhs);
                match op {
                    None => self.demand(rhs.span, &lt, &rt),
                    Some(o) => {
                        if !self.infcx.unify(&lt, &rt) {
                            self.err_mismatch(rhs.span, &lt, &rt);
                        }
                        let ints_only = matches!(
                            o,
                            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr
                        );
                        self.numeric_guard(e.span, binop_text(*o), &lt, ints_only);
                    }
                }
                Ty::unit()
            }
            hir::ExprKind::Cast { expr, ty } => {
                let st = self.check_expr(expr);
                let target = self.ty_from_hir(ty);
                let s = self.infcx.resolve(&st);
                let t = self.infcx.resolve(&target);
                let src_ok = s.is_numeric()
                    || matches!(s, Ty::Prim(PrimTy::Bool | PrimTy::Char))
                    || matches!(s, Ty::Err | Ty::Infer(_) | Ty::Param(_));
                let dst_ok = t.is_numeric() || matches!(t, Ty::Err | Ty::Infer(_) | Ty::Param(_));
                // bool/char 仅可 → 整数(RXS-0046)
                let pair_ok = match (&s, &t) {
                    (Ty::Prim(PrimTy::Bool | PrimTy::Char), tt) if !tt.is_int() => {
                        matches!(tt, Ty::Err | Ty::Infer(_) | Ty::Param(_))
                    }
                    _ => true,
                };
                if !(src_ok && dst_ok && pair_ok) {
                    self.err_mismatch(e.span, &target, &st);
                }
                target
            }
            hir::ExprKind::Range { lo, hi, .. } => {
                let lt = self.check_expr(lo);
                let rt = self.check_expr(hi);
                self.demand(hi.span, &lt, &rt);
                self.numeric_guard(e.span, "..", &lt, true);
                // Range 自身类型未定义(库面随 M3+):容忍
                Ty::Err
            }
            hir::ExprKind::Call { callee, args } => self.check_call(e.span, e.hir_id, callee, args),
            hir::ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => self.check_method(e.span, e.hir_id, receiver, method, args),
            hir::ExprKind::Field { expr, field } => {
                let t = self.check_expr(expr);
                let base = self.autoderef(&t);
                match &base {
                    Ty::Adt(d, adt_args) => {
                        if let Some(fields) = self.fields_of(*d)
                            && let Some(f) = fields.iter().find(|f| f.name == *field)
                        {
                            let mut sig_infer = || Ty::Err;
                            return lower_hir_ty(&f.ty, &mut sig_infer).subst(adt_args);
                        }
                        self.err_bad_field(e.span, "unknown", field, &base);
                        Ty::Err
                    }
                    Ty::Err | Ty::Infer(_) | Ty::Param(_) => Ty::Err,
                    _ => {
                        self.err_bad_field(e.span, "unknown", field, &base);
                        Ty::Err
                    }
                }
            }
            hir::ExprKind::TupleField { expr, index } => {
                let t = self.check_expr(expr);
                let base = self.autoderef(&t);
                match &base {
                    Ty::Tuple(v) => v.get(*index as usize).cloned().unwrap_or_else(|| {
                        self.err_bad_field(e.span, "unknown", &index.to_string(), &base);
                        Ty::Err
                    }),
                    Ty::Adt(d, adt_args) => {
                        if let Some(fields) = self.fields_of(*d)
                            && let Some(f) = fields.get(*index as usize)
                        {
                            let mut sig_infer = || Ty::Err;
                            return lower_hir_ty(&f.ty, &mut sig_infer).subst(adt_args);
                        }
                        self.err_bad_field(e.span, "unknown", &index.to_string(), &base);
                        Ty::Err
                    }
                    Ty::Err | Ty::Infer(_) | Ty::Param(_) => Ty::Err,
                    _ => {
                        self.err_bad_field(e.span, "unknown", &index.to_string(), &base);
                        Ty::Err
                    }
                }
            }
            hir::ExprKind::Index { expr, index } => {
                let bt = self.check_expr(expr);
                let it = self.check_expr(index);
                self.demand(index.span, &Ty::Prim(PrimTy::Usize), &it);
                match self.autoderef(&bt) {
                    Ty::Array(t) | Ty::Slice(t) => *t,
                    // Adt 索引(views 等)经运算符 trait,M2.2 容忍
                    _ => Ty::Err,
                }
            }
            hir::ExprKind::Try(inner) => {
                let _ = self.check_expr(inner);
                Ty::Err // `?` desugar 随 M2.3(Result lang-item)
            }
            hir::ExprKind::Tuple(elems) => {
                Ty::Tuple(elems.iter().map(|x| self.check_expr(x)).collect())
            }
            hir::ExprKind::Array(elems) => {
                let mut iter = elems.iter();
                let first = iter
                    .next()
                    .map(|x| self.check_expr(x))
                    .unwrap_or_else(|| self.infcx.fresh(None));
                for x in iter {
                    let t = self.check_expr(x);
                    self.demand(x.span, &first, &t);
                }
                Ty::Array(Box::new(first))
            }
            hir::ExprKind::Repeat { elem, len } => {
                let t = self.check_expr(elem);
                let lt = self.check_expr(len);
                self.demand(len.span, &Ty::Prim(PrimTy::Usize), &lt);
                Ty::Array(Box::new(t))
            }
            hir::ExprKind::StructLit { res, fields } => self.check_struct_lit(e.span, res, fields),
            hir::ExprKind::Block(b) | hir::ExprKind::Unsafe(b) => self.check_block(b),
            hir::ExprKind::If { cond, then, else_ } => {
                let ct = self.check_expr(cond);
                self.demand(cond.span, &Ty::Prim(PrimTy::Bool), &ct);
                let tt = self.check_block(then);
                match else_ {
                    Some(eb) => {
                        let et = self.check_expr(eb);
                        self.demand(eb.span, &tt, &et);
                        tt
                    }
                    None => {
                        // 无 else 的 if 为 ()(RXS-0044)
                        self.demand(then.span, &Ty::unit(), &tt);
                        Ty::unit()
                    }
                }
            }
            hir::ExprKind::While { cond, body } => {
                let ct = self.check_expr(cond);
                self.demand(cond.span, &Ty::Prim(PrimTy::Bool), &ct);
                let _ = self.check_block(body);
                Ty::unit()
            }
            hir::ExprKind::For { pat, iter, body } => {
                // 区间迭代特判(RXS-0043);其余迭代器协议随 M2.3+
                let bind = if let hir::ExprKind::Range { lo, hi, .. } = &iter.kind {
                    let lt = self.check_expr(lo);
                    let rt = self.check_expr(hi);
                    self.demand(hi.span, &lt, &rt);
                    self.numeric_guard(iter.span, "..", &lt, true);
                    lt
                } else {
                    let _ = self.check_expr(iter);
                    Ty::Err
                };
                self.bind_pat(pat, &bind);
                let _ = self.check_block(body);
                Ty::unit()
            }
            hir::ExprKind::Loop { body } => {
                let _ = self.check_block(body);
                Ty::Err // break 值合一随 M2.3
            }
            hir::ExprKind::Match { scrutinee, arms } => {
                let st = self.check_expr(scrutinee);
                let mut result: Option<Ty> = None;
                for arm in arms {
                    for p in &arm.pats {
                        self.bind_pat(p, &st);
                    }
                    if let Some(g) = &arm.guard {
                        let gt = self.check_expr(g);
                        self.demand(g.span, &Ty::Prim(PrimTy::Bool), &gt);
                    }
                    let at = self.check_expr(&arm.body);
                    match &result {
                        None => result = Some(at),
                        Some(r) => {
                            let r = r.clone();
                            self.demand(arm.body.span, &r, &at);
                        }
                    }
                }
                result.unwrap_or_else(Ty::unit)
            }
            hir::ExprKind::Return(op) => {
                let t = op
                    .as_ref()
                    .map(|x| self.check_expr(x))
                    .unwrap_or_else(Ty::unit);
                let span = op.as_ref().map(|x| x.span).unwrap_or(e.span);
                let ret = self.ret_ty.clone();
                self.demand(span, &ret, &t);
                Ty::Err // never 形态容忍
            }
            hir::ExprKind::Break(op) => {
                if let Some(x) = op {
                    let _ = self.check_expr(x);
                }
                Ty::Err
            }
            hir::ExprKind::Continue => Ty::Err,
            hir::ExprKind::Closure { params, body } => {
                for p in params {
                    self.bind_pat(p, &Ty::Err); // 闭包类型随 M2.3+(容忍)
                }
                let _ = self.check_expr(body);
                Ty::Err
            }
            hir::ExprKind::Err => Ty::Err,
        }
    }

    fn lit_ty(&mut self, l: &crate::ast::Lit) -> Ty {
        match (&l.kind, &l.suffix) {
            (LitKind::Int, Some(s)) | (LitKind::Float, Some(s)) => Ty::Prim(suffix_prim(*s)),
            (LitKind::Int, None) => self.infcx.fresh(Some(NumClass::Int)),
            (LitKind::Float, None) => self.infcx.fresh(Some(NumClass::Float)),
            (LitKind::Str, _) => Ty::Ref(Box::new(Ty::Prim(PrimTy::Str)), false),
            (LitKind::Char, _) => Ty::Prim(PrimTy::Char),
            (LitKind::Bool(_), _) => Ty::Prim(PrimTy::Bool),
        }
    }

    /// 值位置的 Res 类型(RXS-0034 重分类后的消费侧)。
    fn res_value_ty(&mut self, r: &Res) -> Ty {
        match r {
            Res::Local(l) => self
                .locals
                .get(l.0 as usize)
                .and_then(|t| t.clone())
                .unwrap_or(Ty::Err),
            Res::Def(d) => match self.res.defs[d.0 as usize].kind {
                DefKind::Const | DefKind::Static | DefKind::AssocConst => self.cx.type_of(*d),
                DefKind::Fn | DefKind::AssocFn => {
                    let sig = self.cx.fn_sig(*d);
                    if sig.generics_count > 0 {
                        Ty::Err // 泛型 fn 裸引用:单态化点缺失,容忍
                    } else {
                        Ty::FnPtr(sig.inputs.clone(), Box::new(sig.output.clone()))
                    }
                }
                DefKind::Variant => self.ctor_result(*d, Vec::new()),
                DefKind::Struct => Ty::Adt(*d, Vec::new()),
                _ => Ty::Err,
            },
            // const 泛型参数值/Self 等:容忍(RXS-0045 M2.2 口径)
            _ => Ty::Err,
        }
    }

    fn check_binary(&mut self, span: Span, op: BinOp, lhs: &hir::Expr, rhs: &hir::Expr) -> Ty {
        let lt = self.check_expr(lhs);
        let rt = self.check_expr(rhs);
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                if !self.infcx.unify(&lt, &rt) {
                    self.err_mismatch(rhs.span, &lt, &rt);
                    return Ty::Err; // 毒化:防级联(RXS-0047)
                }
                self.numeric_guard(span, binop_text(op), &lt, false);
                lt
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                if !self.infcx.unify(&lt, &rt) {
                    self.err_mismatch(rhs.span, &lt, &rt);
                    return Ty::Err;
                }
                self.numeric_guard(span, binop_text(op), &lt, true);
                lt
            }
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                if !self.infcx.unify(&lt, &rt) {
                    self.err_mismatch(rhs.span, &lt, &rt);
                }
                // 可比较类:数值/bool/char(RXS-0043);Adt 比较经 trait,M2.2 报 RX2006
                let r = self.infcx.resolve(&lt);
                match &r {
                    Ty::Err | Ty::Infer(_) | Ty::Param(_) => {}
                    _ if r.is_numeric() => {}
                    Ty::Prim(PrimTy::Bool | PrimTy::Char) => {}
                    _ => self.err_bad_operand(span, binop_text(op), &r),
                }
                Ty::Prim(PrimTy::Bool)
            }
            BinOp::And | BinOp::Or => {
                self.demand(lhs.span, &Ty::Prim(PrimTy::Bool), &lt);
                self.demand(rhs.span, &Ty::Prim(PrimTy::Bool), &rt);
                Ty::Prim(PrimTy::Bool)
            }
        }
    }

    fn check_call(
        &mut self,
        span: Span,
        call_id: HirId,
        callee: &hir::Expr,
        args: &[hir::Expr],
    ) -> Ty {
        // fn item / 构造器直调(含泛型实例化,RXS-0042/0045)
        if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind {
            let kind = self.res.defs[d.0 as usize].kind;
            match kind {
                DefKind::Fn | DefKind::AssocFn => {
                    let sig = self.cx.fn_sig(*d);
                    let (inputs, output, generic_args) = self.instantiate_sig(&sig);
                    self.results
                        .call_targets
                        .insert(call_id, (*d, generic_args));
                    return self.check_args(span, &inputs, args, output);
                }
                DefKind::Struct | DefKind::Variant => {
                    // 先收集字段类型(owned),再生成 fresh 槽位(借用解耦)
                    let collected = self.fields_of(*d).map(|fields| {
                        let mut sig_infer = || Ty::Err;
                        let raw: Vec<Ty> = fields
                            .iter()
                            .map(|f| lower_hir_ty(&f.ty, &mut sig_infer))
                            .collect();
                        let slots = self.generic_slots(fields);
                        (raw, slots)
                    });
                    if let Some((raw, slots)) = collected {
                        let fresh: Vec<Ty> = (0..slots).map(|_| self.infcx.fresh(None)).collect();
                        let inputs: Vec<Ty> = raw.iter().map(|t| t.subst(&fresh)).collect();
                        let out = self.ctor_result(*d, fresh);
                        return self.check_args(span, &inputs, args, out);
                    }
                }
                _ => {}
            }
        }
        let ct = self.check_expr(callee);
        // resolve 含数值类默认化:AnyInt 变量等可判定为不可调用
        match self.infcx.resolve(&ct) {
            Ty::FnPtr(inputs, output) => self.check_args(span, &inputs, args, *output),
            Ty::Err | Ty::Infer(_) | Ty::Param(_) => {
                for a in args {
                    let _ = self.check_expr(a);
                }
                Ty::Err
            }
            other => {
                for a in args {
                    let _ = self.check_expr(a);
                }
                self.err_not_callable(span, &other);
                Ty::Err
            }
        }
    }

    fn check_args(&mut self, span: Span, inputs: &[Ty], args: &[hir::Expr], output: Ty) -> Ty {
        if args.len() != inputs.len() {
            self.err_arg_count(span, inputs.len(), args.len());
        }
        for (a, expected) in args.iter().zip(inputs.iter()) {
            let at = self.check_expr(a);
            self.demand(a.span, expected, &at);
        }
        for a in args.iter().skip(inputs.len()) {
            let _ = self.check_expr(a);
        }
        output
    }

    fn check_method(
        &mut self,
        span: Span,
        call_id: HirId,
        receiver: &hir::Expr,
        method: &str,
        args: &[hir::Expr],
    ) -> Ty {
        let rt = self.check_expr(receiver);
        let base = self.autoderef(&rt);
        match &base {
            Ty::Adt(d, _adt_args) => {
                let found = self
                    .res
                    .assoc_items
                    .get(d)
                    .and_then(|items| items.iter().find(|(n, _)| n == method))
                    .map(|(_, m)| *m);
                match found {
                    Some(m) => {
                        let sig = self.cx.fn_sig(m);
                        let (inputs, output, generic_args) = self.instantiate_sig(&sig);
                        self.results.call_targets.insert(call_id, (m, generic_args));
                        self.check_args(span, &inputs, args, output)
                    }
                    None => {
                        for a in args {
                            let _ = self.check_expr(a);
                        }
                        self.err_unknown_method(span, method, &base);
                        Ty::Err
                    }
                }
            }
            Ty::Err | Ty::Infer(_) | Ty::Param(_) => {
                for a in args {
                    let _ = self.check_expr(a);
                }
                Ty::Err
            }
            _ => {
                for a in args {
                    let _ = self.check_expr(a);
                }
                self.err_unknown_method(span, method, &base);
                Ty::Err
            }
        }
    }

    fn check_struct_lit(
        &mut self,
        span: Span,
        res: &Res,
        fields: &[(String, Option<hir::Expr>)],
    ) -> Ty {
        let Res::Def(d) = res else {
            for (_, v) in fields {
                if let Some(e) = v {
                    let _ = self.check_expr(e);
                }
            }
            return Ty::Err;
        };
        // 先收集字段名/类型(owned),再生成 fresh 槽位(借用解耦)
        let collected = self.fields_of(*d).map(|fdefs| {
            let mut sig_infer = || Ty::Err;
            let named_raw: Vec<(String, Ty)> = fdefs
                .iter()
                .map(|f| (f.name.clone(), lower_hir_ty(&f.ty, &mut sig_infer)))
                .collect();
            let slots = self.generic_slots(fdefs);
            (named_raw, slots)
        });
        let Some((named_raw, slots)) = collected else {
            for (_, v) in fields {
                if let Some(e) = v {
                    let _ = self.check_expr(e);
                }
            }
            return Ty::Err;
        };
        let fresh: Vec<Ty> = (0..slots).map(|_| self.infcx.fresh(None)).collect();
        let named: Vec<(String, Ty)> = named_raw
            .into_iter()
            .map(|(n, t)| (n, t.subst(&fresh)))
            .collect();
        let result = self.ctor_result(*d, fresh);

        let mut provided: Vec<&str> = Vec::new();
        for (name, value) in fields {
            let expected = named
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, t)| t.clone());
            let vt = value
                .as_ref()
                .map(|e| (e.span, self.check_expr(e)))
                .unwrap_or((span, Ty::Err));
            match expected {
                Some(t) => {
                    if provided.contains(&name.as_str()) {
                        let r = result.clone();
                        self.err_bad_field(span, "duplicate", name, &r);
                    } else {
                        self.demand(vt.0, &t, &vt.1);
                        provided.push(name);
                    }
                }
                None => {
                    let r = result.clone();
                    self.err_bad_field(vt.0, "unknown", name, &r);
                }
            }
        }
        for (n, _) in &named {
            if !provided.contains(&n.as_str()) {
                let r = result.clone();
                self.err_bad_field(span, "missing", n, &r);
            }
        }
        result
    }
}

fn suffix_prim(s: LitSuffix) -> PrimTy {
    match s {
        LitSuffix::I8 => PrimTy::I8,
        LitSuffix::I16 => PrimTy::I16,
        LitSuffix::I32 => PrimTy::I32,
        LitSuffix::I64 => PrimTy::I64,
        LitSuffix::U8 => PrimTy::U8,
        LitSuffix::U16 => PrimTy::U16,
        LitSuffix::U32 => PrimTy::U32,
        LitSuffix::U64 => PrimTy::U64,
        LitSuffix::Usize => PrimTy::Usize,
        LitSuffix::F32 => PrimTy::F32,
        LitSuffix::F64 => PrimTy::F64,
    }
}

fn binop_text(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::{Edition, SourceId};

    fn check(src: &str) -> (Vec<u16>, DiagCtxt) {
        let diag = DiagCtxt::new();
        let codes = {
            let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
            assert!(
                diag.emitted().is_empty(),
                "测试源含前置诊断: {:?}",
                diag.emitted()
            );
            cx.check_crate();
            diag.emitted()
                .iter()
                .filter_map(|d| d.code.map(|c| c.0))
                .collect()
        };
        (codes, diag)
    }

    fn check_clean(src: &str) {
        let (codes, diag) = check(src);
        assert!(
            codes.is_empty(),
            "意外类型诊断: {:?}\n源:\n{src}",
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message(diag.messages())))
                .collect::<Vec<_>>()
        );
    }

    //@ spec: RXS-0039
    #[test]
    fn literal_defaults_and_suffixes() {
        check_clean(
            "fn f() -> i32 { 1 }\nfn g() -> f64 { 1.5 }\nfn h() -> u8 { 255u8 }\nfn s() -> f32 { 2.0f32 }\nfn b() -> bool { true }\nfn c() -> char { 'x' }",
        );
    }

    //@ spec: RXS-0039
    #[test]
    fn int_literal_cannot_be_float() {
        let (codes, _) = check("fn f() -> f32 { 1 }");
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0040
    #[test]
    fn const_init_checked_against_annotation() {
        let (codes, _) = check("const K: i32 = true;");
        assert_eq!(codes, vec![2001]);
        check_clean("const K: i32 = 41 + 1;\nstatic S: bool = false;");
    }

    //@ spec: RXS-0041
    #[test]
    fn let_annotation_and_inference() {
        check_clean(
            "fn f() {\n    let a: i64 = 7;\n    let b = a;\n    let c: i64 = b;\n    let _k = c;\n}",
        );
        let (codes, _) = check("fn f() {\n    let a: bool = 1;\n}");
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0041
    #[test]
    fn deferred_binding_fixed_by_first_assignment() {
        check_clean("fn f() {\n    let v;\n    v = 3usize;\n    let _w: usize = v;\n}");
    }

    //@ spec: RXS-0042
    #[test]
    fn call_arity_and_types() {
        let (codes, _) = check("fn add(a: i32, b: i32) -> i32 { a + b }\nfn f() -> i32 { add(1) }");
        assert_eq!(codes, vec![2003]);
        let (codes, _) =
            check("fn add(a: i32, b: i32) -> i32 { a + b }\nfn f() -> i32 { add(1, true) }");
        assert_eq!(codes, vec![2001]);
        check_clean("fn add(a: i32, b: i32) -> i32 { a + b }\nfn f() -> i32 { add(1, 2) }");
    }

    //@ spec: RXS-0042
    #[test]
    fn not_callable_is_rx2005() {
        let (codes, _) = check("fn f() {\n    let x = 1;\n    let _y = x(2);\n}");
        assert_eq!(codes, vec![2005]);
    }

    //@ spec: RXS-0042
    #[test]
    fn return_must_match_signature() {
        let (codes, _) = check("fn f() -> i32 {\n    return true;\n}");
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0043
    #[test]
    fn operator_rules() {
        check_clean(
            "fn f(a: i32, b: i32, x: f32) -> bool {\n    let _s = a + b * 2;\n    let _m = x / 2.0;\n    let _bits = a & b | (a ^ b);\n    let _sh = a << 2;\n    (a < b) && !(a == b) || false\n}",
        );
        let (codes, _) = check("fn f(a: i32, x: f32) -> f32 { a + x }");
        assert_eq!(codes, vec![2001]);
        let (codes, _) = check("fn f(p: bool, q: bool) -> bool { p + q }");
        assert_eq!(codes, vec![2006]);
        let (codes, _) = check("fn f(x: f32) -> f32 { x << 2.0 }");
        assert!(codes.contains(&2006));
        let (codes, _) = check("fn f(a: i32) -> bool { a && true }");
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0043
    #[test]
    fn conditions_must_be_bool() {
        let (codes, _) = check("fn f(n: i32) {\n    if n { }\n}");
        assert_eq!(codes, vec![2001]);
        let (codes, _) = check("fn f(n: i32) {\n    while n + 1 { }\n}");
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0044
    #[test]
    fn struct_literal_field_rules() {
        let base = "struct P {\n    x: f32,\n    y: f32,\n}\n";
        check_clean(&format!("{base}fn f() -> P {{ P {{ x: 1.0, y: 2.0 }} }}"));
        let (codes, _) = check(&format!("{base}fn f() -> P {{ P {{ x: 1.0, z: 2.0 }} }}"));
        assert!(codes.contains(&2002)); // 未知 z + 缺失 y
        let (codes, _) = check(&format!("{base}fn f() -> P {{ P {{ x: 1.0 }} }}"));
        assert_eq!(codes, vec![2002]); // 缺失 y
        let (codes, _) = check(&format!("{base}fn f() -> P {{ P {{ x: true, y: 2.0 }} }}"));
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0044
    #[test]
    fn field_access_and_match_arm_unification() {
        let base = "struct P {\n    x: f32,\n}\n";
        check_clean(&format!("{base}fn f(p: P) -> f32 {{ p.x }}"));
        let (codes, _) = check(&format!("{base}fn f(p: P) -> f32 {{ p.z }}"));
        assert_eq!(codes, vec![2002]);
        let (codes, _) = check(
            "fn f(n: i32) -> i32 {\n    match n {\n        0 => 1,\n        _ => true,\n    }\n}",
        );
        assert_eq!(codes, vec![2001]);
        let (codes, _) = check("fn f(c: bool) -> i32 { if c { 1 } else { false } }");
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0045
    #[test]
    fn generic_monomorphization_draft() {
        check_clean(
            "fn pick<T>(a: T, b: T) -> T { a }\nfn f() -> i64 { pick(1i64, 2) }\nstruct Holder<T> {\n    inner: T,\n}\nfn g() -> i32 {\n    let h = Holder { inner: 5 };\n    h.inner\n}",
        );
        let (codes, _) =
            check("fn pick<T>(a: T, b: T) -> T { a }\nfn f() -> i64 { pick(1i64, true) }");
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0045
    #[test]
    fn bounds_recorded_not_solved() {
        // bound 不满足不产生诊断(M2.2 口径)
        check_clean(
            "trait Marker {}\nfn constrained<T: Marker>(t: T) -> T { t }\nfn f() -> i32 { constrained(1) }",
        );
    }

    //@ spec: RXS-0046
    #[test]
    fn inherent_methods_and_casts() {
        let base = "struct C {\n    v: u32,\n}\nimpl C {\n    fn new() -> C {\n        C { v: 0 }\n    }\n    fn get(&self) -> u32 {\n        self.v\n    }\n}\n";
        check_clean(&format!(
            "{base}fn f() -> u32 {{\n    let c = C::new();\n    c.get()\n}}"
        ));
        let (codes, _) = check(&format!(
            "{base}fn f() -> u32 {{\n    let c = C::new();\n    c.missing()\n}}"
        ));
        assert_eq!(codes, vec![2004]);
        check_clean("fn f(x: i32) -> f64 { x as f64 }\nfn g(b: bool) -> u8 { b as u8 }");
        let (codes, _) = check("fn f(b: bool) -> f32 { b as f32 }");
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0047
    #[test]
    fn err_tolerance_does_not_cascade() {
        // 草图类型(Grid/View 容忍区)参与的检查全部静默通过
        check_clean(
            "kernel fn k(grid: Grid<(64,)>, out: ViewMut<global, f32, (N,)>) {\n    let i = grid.thread_index();\n    out[i] = 1.0;\n}",
        );
        // for/?/closure 容忍
        check_clean(
            "fn f(n: i32) -> i32 {\n    let mut acc = 0;\n    for i in 0..n {\n        acc += i;\n    }\n    acc\n}",
        );
    }

    // M2.3-B:typeck 结果物化(MIR lowering 输入面)
    #[test]
    fn typeck_results_materialize_node_types() {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(
            "fn f(x: i32) -> i32 {\n    let y = x + 1;\n    y\n}",
            SourceId(0),
            Edition::Rx0,
            &diag,
        );
        let results = cx.check_body(crate::hir::BodyId(0));
        assert!(diag.emitted().is_empty());
        // 局部 x / y 均定型为 i32(数值类默认化已生效)
        assert_eq!(results.local_ty.len(), 2);
        assert!(
            results.local_ty.iter().all(|t| *t == Ty::Prim(PrimTy::I32)),
            "{:?}",
            results.local_ty
        );
        // 表达式与模式节点均落表,且无残留推断变量
        assert!(!results.expr_ty.is_empty());
        assert!(!results.pat_ty.is_empty());
        assert!(
            results
                .expr_ty
                .values()
                .chain(results.pat_ty.values())
                .all(|t| !matches!(t, Ty::Infer(_)))
        );
    }

    // M2.3-B:调用点记录(单态化收集输入,D-111)
    #[test]
    fn typeck_results_record_call_targets_with_substs() {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(
            "fn pick<T>(a: T, b: T) -> T { a }\nfn f() -> i64 { pick(1i64, 2) }",
            SourceId(0),
            Edition::Rx0,
            &diag,
        );
        let res = cx.resolutions();
        let pick = res.defs.iter().position(|d| d.name == "pick").unwrap();
        cx.check_crate();
        assert!(diag.emitted().is_empty());
        // f 的 body(BodyId 1)内有对 pick 的调用,泛型实参定型为 i64
        let results = cx.check_body(crate::hir::BodyId(1));
        let target = results
            .call_targets
            .values()
            .find(|(d, _)| d.0 as usize == pick)
            .expect("调用点已记录");
        assert_eq!(target.1, vec![Ty::Prim(PrimTy::I64)]);
    }

    //@ spec: RXS-0047
    #[test]
    fn mismatch_renders_expected_and_found() {
        let (_, diag) = check("fn f() -> i32 { true }");
        let emitted = diag.emitted();
        let msg = emitted[0].message(diag.messages());
        assert!(msg.contains("i32") && msg.contains("bool"), "{msg}");
    }
}
