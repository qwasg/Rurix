//! views 不相交证明 — MIR 借用检查的 device 扩展 pass(spec 条款 RXS-0078,
//! spec/device.md;07 §4 保守先行)。
//!
//! 实现裁决:本 pass 在 **HIR 层**遍历(仿 [`crate::coloring`] / [`crate::launch_check`]
//! 的 device 检查体例),仅对 **device 上下文 body**(`kernel`/`device` 着色)实施,
//! 消费 M4 着色(RXS-0066)/ 地址空间(RXS-0067)边界信息(子 view 继承父 view 的
//! space 与着色)。spec「MIR 借用检查 device 扩展 pass / 在 host 借用检查之后运行」
//! 以**管线顺序**满足([`crate::query::QueryCtx::check_views`] 在 `check_borrows` 之后、
//! device codegen 之前接入);kernel/device body 不在 host `main` 可达 MIR 内,故不能
//! 复用 host MIR 借用检查的 loan 模型,改以 HIR provenance 跟踪。
//!
//! - **结构性不相交放行**(RXS-0078):`let (lo, hi) = v.split_at(mid)` 产 [0, mid)
//!   与 [mid, len) 两个静态可证不相交子 view,二者可变写并存合法。
//! - **重叠/别名可变借用**(`RX3007`):
//!   - `windows(n)` 子 view 相邻窗口步长 1 恒重叠 → 对其可变写保守拒绝;
//!   - 同一父 view 的两个非结构性不相交子 view(不同 split 组 / 父子别名)同时被
//!     可变写 → 拒绝(证不出不相交的保守拒绝复用本通道)。
//! - **view 划分越界**(`RX3008`):`chunks(0)`/`windows(0)` 零尺寸;以及子 view
//!   长度静态可知时(`split_at` 低半 view 长度 = 字面 `mid`)划分点 / 窗口大小超界。
//!
//! 保守上界(07 §4),按通道区分(checker 为 **conservative reject**,非"证不出不报"):
//! - **RX3008 越界**:仅在子 view 长度/划分点**静态可证**超界时报(证不出长度则放行,
//!   不臆测);
//! - **RX3007 重叠/别名**:对同根子 view 的可变写,**证不出不相交即保守拒绝并报**
//!   (windows 恒重叠、不同 split 组 / 父子别名 / 藏进 aggregate 的别名 view 均拒绝)。
//!
//! `unsafe` 块内豁免(承担 P-03 验证义务,对齐 [`crate::coloring`] barrier 骨架豁免)。
//! 完整区间/别名求解器随真实 kernel 需求扩展(经 conformance 类别留痕);shared+barrier
//! 一致性 / scoped atomics 见 [`crate::shared_check`] / RXS-0080。

use std::collections::{HashMap, HashSet};

use crate::ast::{FnColor, LitKind};
use crate::diag::ErrorCode;
use crate::hir::{
    self, Body, BodyId, Crate, DefId, Expr, ExprKind, LocalId, Pat, PatKind, Res, Stmt, ViewOp,
};
use crate::query::QueryCtx;
use crate::span::Span;
use crate::ty::Ty;
use crate::typeck::TypeckResults;

pub const E_VIEWS_OVERLAPPING_MUT: ErrorCode = ErrorCode(3007); // RX3007(RXS-0078)
pub const E_VIEWS_OUT_OF_BOUNDS: ErrorCode = ErrorCode(3008); // RX3008(RXS-0078)

/// 全 crate views 不相交证明入口(provider:[`QueryCtx::check_views`])。
pub fn check_crate(cx: &QueryCtx<'_>) {
    let krate = cx.hir_crate();
    for i in 0..krate.bodies.len() {
        let body_id = BodyId(i as u32);
        let body = krate.body(body_id);
        // device 扩展:仅 device 上下文(kernel/device 着色)实施(消费着色信息)。
        if !is_device_ctx(context_color(&krate, body.owner)) {
            continue;
        }
        let tcr = cx.check_body(body_id);
        let mut checker = Checker {
            cx,
            body,
            tcr: &tcr,
            prov: HashMap::new(),
            agg_fields: HashMap::new(),
            writes: Vec::new(),
        };
        checker.walk_expr(&body.value, false);
        checker.finish();
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

/// 子 view 的区间标签(结构性不相交证明的依据)。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tag {
    /// 整父 view 区间(根 view / chunks 代表 / 别名再绑定;与同根任意子区间重叠)。
    Whole,
    /// `split_at` 低半 [0, mid):与同一 split 调用的高半互证不相交。
    SplitLo(u32),
    /// `split_at` 高半 [mid, len):与同一 split 调用的低半互证不相交。
    SplitHi(u32),
    /// `windows(n)` 滑动窗口:相邻窗口恒重叠(步长 1),不可证不相交。
    Overlap,
}

/// 一个 view 局部的来源(provenance):根 view + 区间标签 + 静态可知长度。
#[derive(Clone, Copy)]
struct Prov {
    root: LocalId,
    tag: Tag,
    /// 静态可知的元素长度(`split_at` 低半 = `mid`;不可知为 None)。
    len: Option<u64>,
}

/// view 算子调用点的纯 provenance 信息(无诊断)。
#[derive(Clone, Copy)]
struct ViewOpInfo {
    op: ViewOp,
    /// 接收者(父 view)的根 local(接收者非简单 local 时 None)。
    recv_root: Option<LocalId>,
    /// 接收者静态可知长度。
    recv_len: Option<u64>,
    /// 划分实参(`mid`/`n`)字面量值。
    arg_lit: Option<u64>,
}

/// 对一个 view 局部的可变写(index-assign);`in_unsafe` 标记豁免。
struct Write {
    /// 解析出的源 view 局部(provenance 判定与命名用)。
    local: LocalId,
    /// 写"位置"的唯一键:直接局部 `L{n}`,投影 `L{agg}.{field}`。两次写
    /// 同一 place key 视作同一访问路径(不互判别名),不同 place key 才进入冲突判定。
    place_key: String,
    span: Span,
    in_unsafe: bool,
}

struct Checker<'a, 'q> {
    cx: &'a QueryCtx<'q>,
    body: &'a Body,
    tcr: &'a TypeckResults,
    /// view 局部 → provenance(未登记者按整父 view 自根处理)。
    prov: HashMap<u32, Prov>,
    /// 聚合(元组/结构体)局部的字段 → 源 view 局部(`let pair = (v, v)` 后
    /// `pair.0`/`pair.1` 经此解析其别名根,RXS-0078:引用藏进 aggregate 仍追踪)。
    /// 键 = (聚合局部 id, 字段键:元组位置串 / 结构体字段名)。
    agg_fields: HashMap<(u32, String), LocalId>,
    writes: Vec<Write>,
}

impl Checker<'_, '_> {
    // -- provenance / 类型辅助 ------------------------------------------------

    /// 局部是否为可变 view(`ViewMut`);非 view / 只读 view 返回 false。
    fn is_view_mut_local(&self, l: LocalId) -> bool {
        matches!(
            self.tcr.local_ty.get(l.0 as usize),
            Some(Ty::Adt(d, _)) if self.tcr_view_mutable(*d) == Some(true)
        )
    }

    /// 局部是否为 view 族(`View` 或 `ViewMut`)。
    fn is_any_view_local(&self, l: LocalId) -> bool {
        matches!(
            self.tcr.local_ty.get(l.0 as usize),
            Some(Ty::Adt(d, _)) if self.tcr_view_mutable(*d).is_some()
        )
    }

    /// 表达式定型是否为 `ViewMut`(消费 typeck `expr_ty`;用于无 provenance 登记
    /// 的投影写——参数聚合 / 嵌套投影——的 view 判定)。
    fn is_view_mut_expr(&self, e: &Expr) -> bool {
        matches!(
            self.tcr.expr_ty.get(&e.hir_id),
            Some(Ty::Adt(d, _)) if self.tcr_view_mutable(*d) == Some(true)
        )
    }

    fn tcr_view_mutable(&self, d: DefId) -> Option<bool> {
        self.cx.resolutions().lang_items.view_mutable(d)
    }

    fn prov_of(&self, l: LocalId) -> Prov {
        self.prov.get(&l.0).copied().unwrap_or(Prov {
            root: l,
            tag: Tag::Whole,
            len: None,
        })
    }

    fn root_of(&self, l: LocalId) -> LocalId {
        self.prov.get(&l.0).map(|p| p.root).unwrap_or(l)
    }

    fn local_name(&self, l: LocalId) -> String {
        match self.body.locals.get(l.0 as usize) {
            Some(d) if !d.name.is_empty() => format!("`{}`", d.name),
            _ => "this view".to_owned(),
        }
    }

    /// 表达式是否为局部引用(`Res::Local`)。
    fn expr_local(e: &Expr) -> Option<LocalId> {
        match &e.kind {
            ExprKind::Res(Res::Local(l)) => Some(*l),
            _ => None,
        }
    }

    /// 整数字面量取值(`split_at`/`chunks`/`windows` 划分实参常量判定)。
    fn lit_u64(&self, e: &Expr) -> Option<u64> {
        match &e.kind {
            ExprKind::SynthInt(v) => u64::try_from(*v).ok(),
            ExprKind::Lit(l) if l.kind == LitKind::Int => {
                let text = self
                    .cx
                    .src()
                    .get(l.span.lo.0 as usize..l.span.hi.0 as usize)?;
                parse_uint(text)
            }
            _ => None,
        }
    }

    // -- RX3008 越界 + provenance 记录(view 算子调用点) ----------------------

    /// 纯 provenance 信息(无诊断):view 算子 + 接收者 root + 接收者静态长度。
    /// 接收者非 view 族返回 None(消费 typeck 定型;不级联)。
    fn view_op_info(&self, receiver: &Expr, method: &str, args: &[Expr]) -> Option<ViewOpInfo> {
        let op = ViewOp::from_method(method)?;
        let recv_local = Self::expr_local(receiver);
        let recv_is_view = recv_local.is_some_and(|l| {
            matches!(
                self.tcr.local_ty.get(l.0 as usize),
                Some(Ty::Adt(d, _)) if self.tcr_view_mutable(*d).is_some()
            )
        });
        if !recv_is_view {
            return None;
        }
        Some(ViewOpInfo {
            op,
            recv_root: recv_local.map(|l| self.root_of(l)),
            recv_len: recv_local.and_then(|l| self.prov_of(l).len),
            arg_lit: args.first().and_then(|a| self.lit_u64(a)),
        })
    }

    /// 在 view 算子调用点裁决越界(RX3008);`unsafe` 块内豁免(RXS-0078)。
    fn check_view_op_bounds(
        &self,
        call: &Expr,
        receiver: &Expr,
        method: &str,
        args: &[Expr],
        in_unsafe: bool,
    ) {
        if in_unsafe {
            return;
        }
        let Some(info) = self.view_op_info(receiver, method, args) else {
            return;
        };
        let (recv_len, arg_lit) = (info.recv_len, info.arg_lit);
        let detail = match info.op {
            ViewOp::Chunks if arg_lit == Some(0) => {
                Some("chunk size must be at least 1".to_owned())
            }
            ViewOp::Windows if arg_lit == Some(0) => {
                Some("window size must be at least 1".to_owned())
            }
            ViewOp::Windows => match (arg_lit, recv_len) {
                (Some(n), Some(len)) if n > len => {
                    Some(format!("window size {n} exceeds view length {len}"))
                }
                _ => None,
            },
            ViewOp::SplitAt => match (arg_lit, recv_len) {
                (Some(mid), Some(len)) if mid > len => {
                    Some(format!("split point {mid} exceeds view length {len}"))
                }
                _ => None,
            },
            ViewOp::Chunks => None,
        };
        if let Some(detail) = detail {
            self.cx
                .diag()
                .struct_error(E_VIEWS_OUT_OF_BOUNDS, "views.out_of_bounds")
                .arg("detail", detail)
                .span_label(call.span, "view split is out of bounds")
                .emit();
        }
    }

    /// 记录 `let` 绑定引入的子 view provenance(split/chunks/windows/别名)。
    /// 越界诊断(RX3008)已在 [`Self::walk_expr`] 的 MethodCall 处发出,本函数纯记录。
    fn record_let(&mut self, pat: &Pat, init: &Expr) {
        // 形态 1:别名再绑定 `let a = src;`(src 为 view 局部)→ 继承 provenance。
        if let Some(src) = Self::expr_local(init)
            && matches!(
                self.tcr.local_ty.get(src.0 as usize),
                Some(Ty::Adt(d, _)) if self.tcr_view_mutable(*d).is_some()
            )
            && let PatKind::Binding { local } = &pat.kind
        {
            let p = self.prov_of(src);
            self.prov.insert(local.0, p);
            return;
        }

        // 形态 3:聚合(元组 / 结构体)字段含 view 局部 → 记录字段 → 源 view 映射,
        // 供 `pair.0[i] = ..` / `s.field[i] = ..` 投影写解析其别名根(RXS-0078)。
        if let PatKind::Binding { local: agg } = &pat.kind {
            match &init.kind {
                ExprKind::Tuple(elems) => {
                    for (idx, elem) in elems.iter().enumerate() {
                        if let Some(src) = Self::expr_local(elem)
                            && self.is_any_view_local(src)
                        {
                            self.agg_fields.insert((agg.0, idx.to_string()), src);
                        }
                    }
                    return;
                }
                ExprKind::StructLit { fields, .. } => {
                    for (name, val) in fields {
                        if let Some(e) = val
                            && let Some(src) = Self::expr_local(e)
                            && self.is_any_view_local(src)
                        {
                            self.agg_fields.insert((agg.0, name.clone()), src);
                        }
                    }
                    return;
                }
                _ => {}
            }
        }

        // 形态 2:view 算子 `let .. = recv.op(args);`。
        let ExprKind::MethodCall {
            receiver,
            method,
            args,
        } = &init.kind
        else {
            return;
        };
        let Some(info) = self.view_op_info(receiver, method, args) else {
            return;
        };
        let Some(root) = info.recv_root else { return };
        let (recv_len, arg_lit) = (info.recv_len, info.arg_lit);

        match info.op {
            ViewOp::SplitAt => {
                // (lo, hi):lo = [0, mid) 长 = mid;hi = [mid, len) 长 = len - mid。
                let PatKind::Tuple(elems) = &pat.kind else {
                    return;
                };
                if elems.len() != 2 {
                    return;
                }
                let group = init.hir_id.0;
                if let PatKind::Binding { local } = &elems[0].kind {
                    self.prov.insert(
                        local.0,
                        Prov {
                            root,
                            tag: Tag::SplitLo(group),
                            len: arg_lit,
                        },
                    );
                }
                if let PatKind::Binding { local } = &elems[1].kind {
                    let hi_len = match (recv_len, arg_lit) {
                        (Some(l), Some(m)) if l >= m => Some(l - m),
                        _ => None,
                    };
                    self.prov.insert(
                        local.0,
                        Prov {
                            root,
                            tag: Tag::SplitHi(group),
                            len: hi_len,
                        },
                    );
                }
            }
            ViewOp::Windows => {
                if let PatKind::Binding { local } = &pat.kind {
                    self.prov.insert(
                        local.0,
                        Prov {
                            root,
                            tag: Tag::Overlap,
                            len: None,
                        },
                    );
                }
            }
            ViewOp::Chunks => {
                if let PatKind::Binding { local } = &pat.kind {
                    self.prov.insert(
                        local.0,
                        Prov {
                            root,
                            tag: Tag::Whole,
                            len: None,
                        },
                    );
                }
            }
        }
    }

    // -- RX3007 重叠/别名(全 body walk 后裁决) ------------------------------

    /// 两个不同 view 局部的可变写是否冲突(同根且证不出不相交)。
    fn conflicts(&self, a: LocalId, b: LocalId) -> bool {
        let pa = self.prov_of(a);
        let pb = self.prov_of(b);
        if pa.root != pb.root {
            return false; // 不同父 view(MVP local 粒度):不相交
        }
        // windows 重叠由 overlap 通道单独裁决,避免重复诊断。
        if matches!(pa.tag, Tag::Overlap) || matches!(pb.tag, Tag::Overlap) {
            return false;
        }
        !disjoint_siblings(pa.tag, pb.tag)
    }

    fn finish(&mut self) {
        // RX3007(a):windows 子 view 可变写(相邻窗口重叠,保守拒绝)。
        let mut overlap_seen: HashSet<u32> = HashSet::new();
        for w in &self.writes {
            if w.in_unsafe {
                continue;
            }
            if matches!(self.prov_of(w.local).tag, Tag::Overlap) && overlap_seen.insert(w.local.0) {
                self.cx
                    .diag()
                    .struct_error(E_VIEWS_OVERLAPPING_MUT, "views.overlapping_mut")
                    .arg("place", self.local_name(w.local))
                    .span_label(w.span, "overlapping `windows` sub-view written here")
                    .emit();
            }
        }

        // RX3007(b):同根别名子 view 跨 view 写冲突(证不出不相交的保守拒绝)。
        let mut pair_seen: HashSet<(u32, u32)> = HashSet::new();
        for i in 0..self.writes.len() {
            for j in (i + 1)..self.writes.len() {
                let (wi, wj) = (&self.writes[i], &self.writes[j]);
                // 同一访问路径(place key 相同)= 同一 view 同一写法,不互判别名;
                // 不同 place key(含 `pair.0` vs `pair.1` 等投影别名)才进入冲突判定。
                if wi.in_unsafe || wj.in_unsafe || wi.place_key == wj.place_key {
                    continue;
                }
                if !self.conflicts(wi.local, wj.local) {
                    continue;
                }
                let key = if wi.local.0 <= wj.local.0 {
                    (wi.local.0, wj.local.0)
                } else {
                    (wj.local.0, wi.local.0)
                };
                if !pair_seen.insert(key) {
                    continue;
                }
                let root = self.root_of(wi.local);
                self.cx
                    .diag()
                    .struct_error(E_VIEWS_OVERLAPPING_MUT, "views.overlapping_mut")
                    .arg("place", self.local_name(root))
                    .span_label(wj.span, "aliased mutable sub-view written here")
                    .emit();
            }
        }
    }

    // -- walk -----------------------------------------------------------------

    fn record_write(&mut self, lhs: &Expr, in_unsafe: bool) {
        let ExprKind::Index { expr: base, .. } = &lhs.kind else {
            return;
        };
        // 直接 view 局部:`v[i] = ..`。
        if let Some(l) = Self::expr_local(base)
            && self.is_view_mut_local(l)
        {
            self.writes.push(Write {
                local: l,
                place_key: format!("L{}", l.0),
                span: lhs.span,
                in_unsafe,
            });
            return;
        }
        // 聚合字段投影:`pair.0[i] = ..` / `s.field[i] = ..`(藏进 tuple/struct 的
        // view 别名);解析到源 view 局部,以投影路径为 place key 参与别名冲突判定。
        if let Some((src, key)) = self.resolve_field_view(base) {
            if self.is_view_mut_local(src) {
                self.writes.push(Write {
                    local: src,
                    place_key: key,
                    span: lhs.span,
                    in_unsafe,
                });
            }
            return;
        }

        // 无 provenance 登记的 view 投影写(参数为 view 结构体/元组,或嵌套投影):
        // 聚合非 `let` 绑定故 `agg_fields` 无记录,证不出各字段子 view 不相交。
        // 按 RXS-0078 保守口径(证不出不相交即拒绝)纳入冲突判定:以聚合局部为
        // 根、投影路径为 place key,同一聚合的不同字段投影写互判别名 → RX3007;
        // 单字段写 / 同字段同路径写不构成冲突(确为不相交则放行)。
        if let Some((agg, key)) = self.unknown_view_projection(base) {
            self.writes.push(Write {
                local: agg,
                place_key: key,
                span: lhs.span,
                in_unsafe,
            });
        }
    }

    /// 无 provenance 登记的 view 字段投影:`agg.field` 定型为 `ViewMut` 且 `agg`
    /// 为简单局部(参数聚合 / 未经 `let` 记录的聚合)→ (聚合局部, 投影 place key)。
    fn unknown_view_projection(&self, base: &Expr) -> Option<(LocalId, String)> {
        if !self.is_view_mut_expr(base) {
            return None;
        }
        match &base.kind {
            ExprKind::TupleField { expr, index } => {
                let agg = Self::expr_local(expr)?;
                Some((agg, format!("L{}.{}", agg.0, index)))
            }
            ExprKind::Field { expr, field } => {
                let agg = Self::expr_local(expr)?;
                Some((agg, format!("L{}.{}", agg.0, field)))
            }
            _ => None,
        }
    }

    /// 解析投影基址 `agg.field` 到其源 view 局部 + 唯一 place key(经 `agg_fields`)。
    fn resolve_field_view(&self, base: &Expr) -> Option<(LocalId, String)> {
        match &base.kind {
            ExprKind::TupleField { expr, index } => {
                let agg = Self::expr_local(expr)?;
                let src = self.agg_fields.get(&(agg.0, index.to_string())).copied()?;
                Some((src, format!("L{}.{}", agg.0, index)))
            }
            ExprKind::Field { expr, field } => {
                let agg = Self::expr_local(expr)?;
                let src = self.agg_fields.get(&(agg.0, field.clone())).copied()?;
                Some((src, format!("L{}.{}", agg.0, field)))
            }
            _ => None,
        }
    }

    fn walk_expr(&mut self, e: &Expr, in_unsafe: bool) {
        match &e.kind {
            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                // view 算子调用点越界裁决(provenance 记录在 `let` 处)。
                self.check_view_op_bounds(e, receiver, method, args, in_unsafe);
                self.walk_expr(receiver, in_unsafe);
                for a in args {
                    self.walk_expr(a, in_unsafe);
                }
            }
            ExprKind::Assign { lhs, rhs, .. } => {
                self.record_write(lhs, in_unsafe);
                self.walk_expr(lhs, in_unsafe);
                self.walk_expr(rhs, in_unsafe);
            }
            ExprKind::Call { callee, args } => {
                self.walk_expr(callee, in_unsafe);
                for a in args {
                    self.walk_expr(a, in_unsafe);
                }
            }
            ExprKind::Unary { expr, .. }
            | ExprKind::Borrow { expr, .. }
            | ExprKind::Cast { expr, .. }
            | ExprKind::Field { expr, .. }
            | ExprKind::TupleField { expr, .. } => self.walk_expr(expr, in_unsafe),
            ExprKind::Binary { lhs, rhs, .. }
            | ExprKind::Range {
                lo: lhs, hi: rhs, ..
            }
            | ExprKind::Index {
                expr: lhs,
                index: rhs,
            } => {
                self.walk_expr(lhs, in_unsafe);
                self.walk_expr(rhs, in_unsafe);
            }
            ExprKind::Tuple(v) | ExprKind::Array(v) => {
                for x in v {
                    self.walk_expr(x, in_unsafe);
                }
            }
            ExprKind::Repeat { elem, len } => {
                self.walk_expr(elem, in_unsafe);
                self.walk_expr(len, in_unsafe);
            }
            ExprKind::StructLit { fields, .. } => {
                for (_, v) in fields {
                    if let Some(x) = v {
                        self.walk_expr(x, in_unsafe);
                    }
                }
            }
            ExprKind::Block(b) => self.walk_block(b, in_unsafe),
            ExprKind::Unsafe(b) => self.walk_block(b, true),
            ExprKind::If { cond, then, else_ } => {
                self.walk_expr(cond, in_unsafe);
                self.walk_block(then, in_unsafe);
                if let Some(eb) = else_ {
                    self.walk_expr(eb, in_unsafe);
                }
            }
            ExprKind::While { cond, body } => {
                self.walk_expr(cond, in_unsafe);
                self.walk_block(body, in_unsafe);
            }
            ExprKind::Loop { body } => self.walk_block(body, in_unsafe),
            ExprKind::Match { scrutinee, arms } => {
                self.walk_expr(scrutinee, in_unsafe);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        self.walk_expr(g, in_unsafe);
                    }
                    self.walk_expr(&arm.body, in_unsafe);
                }
            }
            ExprKind::Return(op) | ExprKind::Break(op) => {
                if let Some(x) = op {
                    self.walk_expr(x, in_unsafe);
                }
            }
            ExprKind::Closure { body, .. } => self.walk_expr(body, in_unsafe),
            ExprKind::Lit(_)
            | ExprKind::SynthInt(_)
            | ExprKind::Res(_)
            | ExprKind::Continue
            | ExprKind::Err => {}
        }
    }

    fn walk_block(&mut self, b: &hir::Block, in_unsafe: bool) {
        for s in &b.stmts {
            match s {
                Stmt::Item(_) => {} // 嵌套 item body 经 check_crate 全集遍历
                Stmt::Let { pat, init, .. } => {
                    if let Some(e) = init {
                        self.walk_expr(e, in_unsafe);
                        self.record_let(pat, e);
                    }
                }
                Stmt::Expr(e) => self.walk_expr(e, in_unsafe),
            }
        }
        if let Some(t) = &b.tail {
            self.walk_expr(t, in_unsafe);
        }
    }
}

/// 两个区间标签是否互证不相交(同一 split 调用的低/高半)。
fn disjoint_siblings(a: Tag, b: Tag) -> bool {
    matches!(
        (a, b),
        (Tag::SplitLo(g1), Tag::SplitHi(g2)) | (Tag::SplitHi(g1), Tag::SplitLo(g2)) if g1 == g2
    )
}

/// 无符号整数字面量解析(十进制 / `0x` 十六进制;容忍后缀与下划线)。
fn parse_uint(text: &str) -> Option<u64> {
    let t: String = text.chars().filter(|c| *c != '_').collect();
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        let digits: String = h.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        if digits.is_empty() {
            return None;
        }
        u64::from_str_radix(&digits, 16).ok()
    } else {
        let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        digits.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// 跑 typeck + 着色 + views 不相交检查,返回 views 诊断码序列。
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
        cx.check_views();
        let mut codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        codes.sort_unstable();
        codes
    }

    const HEAD: &str = "kernel fn k(v: ViewMut<global, f32>, i: usize, t: ThreadCtx<1>) {\n";

    //@ spec: RXS-0078
    #[test]
    fn split_disjoint_halves_are_clean() {
        let src = format!(
            "{HEAD}    let (lo, hi) = v.split_at(i);\n    lo[i] = 1.0;\n    hi[0] = 2.0;\n}}\nfn main() {{}}"
        );
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0078
    #[test]
    fn windows_mut_write_is_rx3007() {
        let src = format!("{HEAD}    let w = v.windows(2);\n    w[i] = 1.0;\n}}\nfn main() {{}}");
        assert_eq!(check(&src), vec![3007]);
    }

    //@ spec: RXS-0078
    #[test]
    fn alias_parent_and_subview_write_is_rx3007() {
        let src = format!(
            "{HEAD}    let (lo, hi) = v.split_at(4);\n    lo[i] = 1.0;\n    v[i] = 2.0;\n}}\nfn main() {{}}"
        );
        assert_eq!(check(&src), vec![3007]);
    }

    //@ spec: RXS-0078
    #[test]
    fn two_distinct_splits_alias_is_rx3007() {
        let src = format!(
            "{HEAD}    let (a, b) = v.split_at(4);\n    let (c, d) = v.split_at(6);\n    a[i] = 1.0;\n    c[i] = 2.0;\n}}\nfn main() {{}}"
        );
        assert_eq!(check(&src), vec![3007]);
    }

    //@ spec: RXS-0078
    #[test]
    fn tuple_field_aliased_mut_write_is_rx3007() {
        // 引用藏进元组:pair.0 与 pair.1 同根别名 v,两路投影可变写 → RX3007
        let src = format!(
            "{HEAD}    let pair = (v, v);\n    pair.0[i] = 1.0;\n    pair.1[i] = 2.0;\n}}\nfn main() {{}}"
        );
        assert_eq!(check(&src), vec![3007]);
    }

    //@ spec: RXS-0078
    #[test]
    fn param_struct_field_aliased_mut_write_is_rx3007() {
        // 参数为 view 结构体(非 let 绑定):p.a / p.b 无 provenance 登记,证不出
        // 两子 view 不相交 → 保守拒绝 RX3007(覆盖此前漏检面,RXS-0078)。
        let src = "struct Pair { a: ViewMut<global, f32>, b: ViewMut<global, f32> }\nkernel fn k(p: Pair, i: usize) {\n    p.a[i] = 1.0;\n    p.b[i] = 2.0;\n}\nfn main() {}";
        assert_eq!(check(src), vec![3007]);
    }

    //@ spec: RXS-0078
    #[test]
    fn param_tuple_field_aliased_mut_write_is_rx3007() {
        // 参数为 view 元组(非 let 绑定):pair.0 / pair.1 无 provenance,保守拒绝。
        let src = "kernel fn k(pair: (ViewMut<global, f32>, ViewMut<global, f32>), i: usize) {\n    pair.0[i] = 1.0;\n    pair.1[i] = 2.0;\n}\nfn main() {}";
        assert_eq!(check(src), vec![3007]);
    }

    //@ spec: RXS-0078
    #[test]
    fn param_struct_single_field_write_is_clean() {
        // 参数 view 结构体仅写单字段:无第二路冲突写 → 放行(不过度拒绝)。
        let src = "struct Pair { a: ViewMut<global, f32>, b: ViewMut<global, f32> }\nkernel fn k(p: Pair, i: usize) {\n    p.a[i] = 1.0;\n}\nfn main() {}";
        assert!(check(src).is_empty(), "{:?}", check(src));
    }

    //@ spec: RXS-0078
    #[test]
    fn disjoint_sources_tuple_field_write_is_clean() {
        // 聚合由两个 distinct view 参数构成(`let pair = (a, b)`):pair.0/pair.1
        // 根不同 → 确为不相交 → 放行(RXS-0078 结构性不相交)。
        let src = "kernel fn k(a: ViewMut<global, f32>, b: ViewMut<global, f32>, i: usize) {\n    let pair = (a, b);\n    pair.0[i] = 1.0;\n    pair.1[i] = 2.0;\n}\nfn main() {}";
        assert!(check(src).is_empty(), "{:?}", check(src));
    }

    //@ spec: RXS-0078
    #[test]
    fn chunks_zero_is_rx3008() {
        let src = format!("{HEAD}    let c = v.chunks(0);\n}}\nfn main() {{}}");
        assert_eq!(check(&src), vec![3008]);
    }

    //@ spec: RXS-0078
    #[test]
    fn split_oob_on_known_low_half_is_rx3008() {
        // lo = v.split_at(4) 的低半,静态长 4;lo.split_at(8) → 8 > 4 越界
        let src = format!(
            "{HEAD}    let (lo, hi) = v.split_at(4);\n    let (a, b) = lo.split_at(8);\n}}\nfn main() {{}}"
        );
        assert_eq!(check(&src), vec![3008]);
    }

    //@ spec: RXS-0078
    #[test]
    fn window_oversize_on_known_low_half_is_rx3008() {
        let src = format!(
            "{HEAD}    let (lo, hi) = v.split_at(4);\n    let w = lo.windows(8);\n}}\nfn main() {{}}"
        );
        assert_eq!(check(&src), vec![3008]);
    }

    //@ spec: RXS-0078
    #[test]
    fn windows_mut_write_in_unsafe_is_exempt() {
        let src = format!(
            "{HEAD}    let w = v.windows(2);\n    unsafe {{\n        w[i] = 1.0;\n    }}\n}}\nfn main() {{}}"
        );
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0078
    #[test]
    fn single_chunk_write_is_clean() {
        let src = format!("{HEAD}    let c = v.chunks(8);\n    c[i] = 1.0;\n}}\nfn main() {{}}");
        assert!(check(&src).is_empty(), "{:?}", check(&src));
    }

    //@ spec: RXS-0078
    #[test]
    fn host_context_views_not_checked() {
        // host fn 非 device 上下文:views 扩展 pass 不实施(消费着色)
        let src = "fn h(v: ViewMut<global, f32>, i: usize) {\n    let w = v.windows(2);\n    w[i] = 1.0;\n}\nfn main() {}";
        assert!(check(src).is_empty(), "{:?}", check(src));
    }
}
