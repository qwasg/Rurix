//! query 骨架(07 §2 / D-203:"接口第一天、存储最后一天")。
//!
//! 第一天形态:
//! - 全部语义分析 API 写成 **query 风格纯函数**:同输入同输出,provider 之间
//!   只经 [`QueryCtx`] 互访,无全局可变状态;
//! - **进程内 memoization**:每个 query 首算后缓存,命中计数暴露
//!   ([`QueryCtx::memo_hits`])供单测与未来 self-profile(M2.4)消费;
//! - 不做:跨会话红绿增量、并行前端(D-203 Phase 2+,显式 out-of-scope)。
//!
//! 首批 query:`resolutions` / `hir_crate` / `def_kind` / `fn_sig` / `type_of` /
//! `check_body`(后三者由 [`crate::typeck`] 提供 provider)。

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast;
use crate::const_eval::{ConstError, ConstVal};
use crate::diag::DiagCtxt;
use crate::hir::{self, BodyId, DefId, DefKind};
use crate::lexer::lex;
use crate::lower::lower;
use crate::parser::parse;
use crate::resolve::{Resolutions, resolve};
use crate::span::{Edition, SourceId};
use crate::ty::{FnSig, Ty};
use crate::typeck::TypeckResults;

/// query 上下文:输入(源文本 → AST)+ 各 query 的 memo 存储。
pub struct QueryCtx<'a> {
    diag: &'a DiagCtxt,
    /// 源文本(MIR 字面量取值;span 切片)。
    src: String,
    ast: ast::SourceFile,
    src_file: SourceId,
    // ---- memo 存储(进程内,D-203 MVP) ----
    resolutions: OnceCell<Rc<Resolutions>>,
    hir: OnceCell<Rc<hir::Crate>>,
    fn_sigs: RefCell<HashMap<DefId, Rc<FnSig>>>,
    type_of: RefCell<HashMap<DefId, Ty>>,
    checked_bodies: RefCell<HashMap<BodyId, Rc<TypeckResults>>>,
    /// 模式穷尽性已检 body 集(RXS-0051;memo 防重复诊断)。
    checked_patterns: RefCell<std::collections::HashSet<BodyId>>,
    /// 定义处检查已跑标记(RXS-0053/RXS-0055;memo 防重复诊断)。
    checked_defs: Cell<bool>,
    /// 着色/barrier 骨架检查已跑标记(RXS-0066/0068;memo 防重复诊断)。
    checked_coloring: Cell<bool>,
    /// move/init 检查已跑标记(RXS-0054;memo 防重复诊断)。
    checked_moves: Cell<bool>,
    /// 借用检查已跑标记(RXS-0057~0061;memo 防重复诊断)。
    checked_borrows: Cell<bool>,
    /// const 求值结果 memo(RXS-0062;DefId → 值/错误)。
    const_vals: RefCell<HashMap<DefId, Result<ConstVal, ConstError>>>,
    /// const 求值进行中集(环引用检测,RXS-0063)。
    const_in_progress: RefCell<std::collections::HashSet<DefId>>,
    /// const 求值强制检查已跑标记(RXS-0065;memo 防重复诊断)。
    checked_consteval: Cell<bool>,
    mir: OnceCell<Rc<Vec<crate::mir::Body>>>,
    // ---- 计量(self-profile 布点,07 §6) ----
    hits: Cell<u64>,
    misses: Cell<u64>,
    // ---- TBIR 窄门计量(M3.1:即建即用层,不入 memo;07 §1 D-202) ----
    tbir_bodies: Cell<u64>,
    tbir_scopes: Cell<u64>,
    tbir_nanos: Cell<u64>,
}

impl<'a> QueryCtx<'a> {
    /// 从源文本构建(lex + parse 为输入阶段;诊断经 `diag`)。
    pub fn new(src: &str, file: SourceId, edition: Edition, diag: &'a DiagCtxt) -> Self {
        let tokens = lex(src, file, edition, diag);
        let ast = parse(src, tokens, file, edition, diag);
        Self::from_ast(ast, src, file, diag)
    }

    pub fn from_ast(ast: ast::SourceFile, src: &str, file: SourceId, diag: &'a DiagCtxt) -> Self {
        Self {
            diag,
            src: src.to_owned(),
            ast,
            src_file: file,
            resolutions: OnceCell::new(),
            hir: OnceCell::new(),
            fn_sigs: RefCell::new(HashMap::new()),
            type_of: RefCell::new(HashMap::new()),
            checked_bodies: RefCell::new(HashMap::new()),
            checked_patterns: RefCell::new(std::collections::HashSet::new()),
            checked_defs: Cell::new(false),
            checked_coloring: Cell::new(false),
            checked_moves: Cell::new(false),
            checked_borrows: Cell::new(false),
            const_vals: RefCell::new(HashMap::new()),
            const_in_progress: RefCell::new(std::collections::HashSet::new()),
            checked_consteval: Cell::new(false),
            mir: OnceCell::new(),
            hits: Cell::new(0),
            misses: Cell::new(0),
            tbir_bodies: Cell::new(0),
            tbir_scopes: Cell::new(0),
            tbir_nanos: Cell::new(0),
        }
    }

    pub fn diag(&self) -> &DiagCtxt {
        self.diag
    }

    pub fn ast(&self) -> &ast::SourceFile {
        &self.ast
    }

    pub fn src(&self) -> &str {
        &self.src
    }

    pub fn src_file(&self) -> SourceId {
        self.src_file
    }

    /// memo 命中/未命中计数(单测断言与 self-profile 数据源)。
    pub fn memo_hits(&self) -> u64 {
        self.hits.get()
    }

    pub fn memo_misses(&self) -> u64 {
        self.misses.get()
    }

    fn hit(&self) {
        self.hits.set(self.hits.get() + 1);
    }

    fn miss(&self) {
        self.misses.set(self.misses.get() + 1);
    }

    /// TBIR 即建即用计量(mir_build 逐实例上报;self-profile `tbir` 阶段数据源)。
    pub fn note_tbir(&self, scopes: u64, elapsed: std::time::Duration) {
        self.tbir_bodies.set(self.tbir_bodies.get() + 1);
        self.tbir_scopes.set(self.tbir_scopes.get() + scopes);
        self.tbir_nanos
            .set(self.tbir_nanos.get() + elapsed.as_nanos() as u64);
    }

    /// (TBIR body 数, scope 总数, 累计构造毫秒)。
    pub fn tbir_stats(&self) -> (u64, u64, f64) {
        (
            self.tbir_bodies.get(),
            self.tbir_scopes.get(),
            self.tbir_nanos.get() as f64 / 1e6,
        )
    }

    // ---- 首批 query ---------------------------------------------------------

    /// 名称解析结果(provider:[`crate::resolve::resolve`])。
    pub fn resolutions(&self) -> Rc<Resolutions> {
        if let Some(r) = self.resolutions.get() {
            self.hit();
            return Rc::clone(r);
        }
        self.miss();
        let r = Rc::new(resolve(&self.ast, self.diag));
        let _ = self.resolutions.set(Rc::clone(&r));
        r
    }

    /// HIR(provider:[`crate::lower::lower`],经 `resolutions` query 互访)。
    pub fn hir_crate(&self) -> Rc<hir::Crate> {
        if let Some(k) = self.hir.get() {
            self.hit();
            return Rc::clone(k);
        }
        self.miss();
        let res = self.resolutions();
        let k = Rc::new(lower(&self.ast, &res));
        let _ = self.hir.set(Rc::clone(&k));
        k
    }

    /// 定义类别(轻量查表,经 `resolutions`)。
    pub fn def_kind(&self, def: DefId) -> Option<DefKind> {
        let res = self.resolutions();
        res.defs.get(def.0 as usize).map(|d| d.kind)
    }

    /// 函数签名(provider:[`crate::typeck::fn_sig_provider`])。
    pub fn fn_sig(&self, def: DefId) -> Rc<FnSig> {
        if let Some(sig) = self.fn_sigs.borrow().get(&def) {
            self.hit();
            return Rc::clone(sig);
        }
        self.miss();
        let sig = Rc::new(crate::typeck::fn_sig_provider(self, def));
        self.fn_sigs.borrow_mut().insert(def, Rc::clone(&sig));
        sig
    }

    /// 定义的类型(struct/enum 自身、const/static 标注;provider 在 typeck)。
    pub fn type_of(&self, def: DefId) -> Ty {
        if let Some(ty) = self.type_of.borrow().get(&def) {
            self.hit();
            return ty.clone();
        }
        self.miss();
        let ty = crate::typeck::type_of_provider(self, def);
        self.type_of.borrow_mut().insert(def, ty.clone());
        ty
    }

    /// body 类型检查(诊断经 DiagCtxt 产出;memo 防重复检查)。
    ///
    /// M2.3 起返回按节点物化的 [`TypeckResults`](MIR lowering 的输入)。
    pub fn check_body(&self, body: BodyId) -> Rc<TypeckResults> {
        if let Some(r) = self.checked_bodies.borrow().get(&body) {
            self.hit();
            return Rc::clone(r);
        }
        self.miss();
        let r = Rc::new(crate::typeck::check_body_provider(self, body));
        self.checked_bodies.borrow_mut().insert(body, Rc::clone(&r));
        r
    }

    /// 定义处检查(M3.2:derive(Copy) 合法性 + Drop impl 形状,
    /// RXS-0053/RXS-0055;provider:[`crate::typeck::check_defs_provider`])。
    pub fn check_defs(&self) {
        if self.checked_defs.replace(true) {
            self.hit();
            return;
        }
        self.miss();
        crate::typeck::check_defs_provider(self);
    }

    /// 全 crate 类型检查入口(定义处检查 + 遍历全部 body)。
    pub fn check_crate(&self) {
        self.check_defs();
        let krate = self.hir_crate();
        for i in 0..krate.bodies.len() {
            let _ = self.check_body(BodyId(i as u32));
        }
    }

    /// 着色 + barrier 骨架检查(RXS-0066/0068;HIR 层,typeck 后、MIR 前;
    /// provider:[`crate::coloring::check_crate`])。地址空间一致性(RXS-0067)
    /// 在 typeck 合一处裁决,不在此 query。memo 防重复诊断。
    pub fn check_coloring(&self) {
        if self.checked_coloring.replace(true) {
            self.hit();
            return;
        }
        self.miss();
        crate::coloring::check_crate(self);
    }

    /// 模式穷尽性检查(RXS-0051;TBIR 窄门时点 = typeck 后、MIR 前)。
    ///
    /// TBIR 即建即检即弃(D-202);memo 防同 body 重复诊断(单态化多实例)。
    pub fn check_patterns(&self, body: BodyId) {
        if !self.checked_patterns.borrow_mut().insert(body) {
            self.hit();
            return;
        }
        self.miss();
        let krate = self.hir_crate();
        let res = self.resolutions();
        let tcr = self.check_body(body);
        let tb = crate::tbir_build::build(&krate, &res, &tcr, krate.body(body));
        crate::tbir_build::check_exhaustiveness(&krate, &res, self.diag, &tb);
    }

    /// 全 crate 模式穷尽性检查(覆盖不可达 body,与 MIR 可达性收集解耦)。
    pub fn check_crate_patterns(&self) {
        let krate = self.hir_crate();
        for i in 0..krate.bodies.len() {
            self.check_patterns(BodyId(i as u32));
        }
    }

    /// move/init 数据流检查(RXS-0053/RXS-0054;MIR 后、codegen 前强制;
    /// provider:[`crate::move_check::check_body`] 对全部单态化实例)。
    pub fn check_moves(&self) {
        if self.checked_moves.replace(true) {
            self.hit();
            return;
        }
        self.miss();
        let mir = self.mir_crate();
        for body in mir.iter() {
            crate::move_check::check_body(self.diag, body);
        }
    }

    /// NLL 借用检查(RXS-0057~0061;MIR 后、codegen 前,move/init 之后强制;
    /// provider:[`crate::borrow_check::check_body`] 对全部单态化实例)。
    pub fn check_borrows(&self) {
        if self.checked_borrows.replace(true) {
            self.hit();
            return;
        }
        self.miss();
        let mir = self.mir_crate();
        for body in mir.iter() {
            crate::borrow_check::check_body(self.diag, body);
        }
    }

    /// const item 求值(RXS-0062/0063;provider:[`crate::const_eval::eval_const_item`])。
    ///
    /// memo 化(同 const 跨引用点共享值);环引用经 in-progress 集检出报
    /// 非 const 操作错误(RXS-0063)。
    pub fn eval_const(&self, def: DefId) -> Result<ConstVal, ConstError> {
        if let Some(r) = self.const_vals.borrow().get(&def) {
            self.hit();
            return r.clone();
        }
        self.miss();
        if !self.const_in_progress.borrow_mut().insert(def) {
            // 求值期间再次进入同一 const = 环(RXS-0063);不入 memo,由各引用点报告
            return Err(ConstError::NonConst {
                span: self.hir_crate().item(def).span,
                what: "cyclic constant reference".to_owned(),
            });
        }
        let r = crate::const_eval::eval_const_item(self, def);
        self.const_in_progress.borrow_mut().remove(&def);
        self.const_vals.borrow_mut().insert(def, r.clone());
        r
    }

    /// const 求值强制检查(RXS-0065):对全部可 ground 求值的 const item 强制
    /// 求值,失败即报 5xxx;时点 = typeck 后、MIR 前(对全部 const 求值上下文
    /// 强制,即便未被运行期引用)。memo 防重复诊断。
    pub fn check_consteval(&self) {
        if self.checked_consteval.replace(true) {
            self.hit();
            return;
        }
        self.miss();
        let krate = self.hir_crate();
        for (i, item) in krate.items.iter().enumerate() {
            if !matches!(item.kind, hir::ItemKind::Const { .. }) {
                continue;
            }
            let def = DefId(i as u32);
            // 仅强制可 ground 求值者(类型无 Param/Infer/Err);泛型上下文 assoc
            // const 随 M4+(标量优先,登记已知限制)
            if !ty_is_ground(&self.type_of(def)) {
                continue;
            }
            if let Err(e) = self.eval_const(def) {
                e.emit(self.diag);
            }
        }
    }

    /// MIR(单态化实例集,自 `main` 可达;provider:[`crate::mir_build::build_crate`])。
    pub fn mir_crate(&self) -> Rc<Vec<crate::mir::Body>> {
        if let Some(m) = self.mir.get() {
            self.hit();
            return Rc::clone(m);
        }
        self.miss();
        let m = Rc::new(crate::mir_build::build_crate(self));
        let _ = self.mir.set(Rc::clone(&m));
        m
    }

    /// device MIR(M4.2,RXS-0070;`kernel fn` 为根的 device 调用图收集;
    /// provider:[`crate::mir_build::build_device_crate`])。不缓存(device
    /// codegen 单次消费;host `main` 可达性收集与之独立)。
    pub fn device_mir_crate(&self) -> Vec<crate::mir::Body> {
        crate::mir_build::build_device_crate(self)
    }
}

/// 类型是否完全 ground(无 Param/Infer/Err;const 强制求值的前置)。
fn ty_is_ground(ty: &Ty) -> bool {
    match ty {
        Ty::Param(_) | Ty::Infer(_) | Ty::Err => false,
        Ty::Prim(_) => true,
        Ty::Adt(_, a) => a.iter().all(ty_is_ground),
        Ty::Tuple(v) => v.iter().all(ty_is_ground),
        Ty::Ref(t, _) | Ty::RawPtr(t, _) | Ty::Array(t) | Ty::Slice(t) => ty_is_ground(t),
        Ty::FnPtr(ps, r) => ps.iter().all(ty_is_ground) && ty_is_ground(r),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::{Edition, SourceId};

    fn ctx_for<'a>(src: &str, diag: &'a DiagCtxt) -> QueryCtx<'a> {
        QueryCtx::new(src, SourceId(0), Edition::Rx0, diag)
    }

    #[test]
    fn memo_hits_on_second_call() {
        let diag = DiagCtxt::new();
        let cx = ctx_for("fn f() -> i32 { 1 }", &diag);
        let _ = cx.resolutions();
        let misses_after_first = cx.memo_misses();
        let _ = cx.resolutions();
        let _ = cx.resolutions();
        assert_eq!(cx.memo_misses(), misses_after_first, "二次调用零重算");
        assert!(cx.memo_hits() >= 2);
    }

    #[test]
    fn hir_provider_goes_through_resolutions_query() {
        let diag = DiagCtxt::new();
        let cx = ctx_for("fn f() {}", &diag);
        // 直接要 HIR:resolutions 作为依赖被 query 化拉起(miss 2 次)
        let _ = cx.hir_crate();
        assert_eq!(cx.memo_misses(), 2);
        // 再要 resolutions:命中
        let _ = cx.resolutions();
        assert_eq!(cx.memo_misses(), 2);
        assert!(cx.memo_hits() >= 1);
    }

    #[test]
    fn def_kind_lookup() {
        let diag = DiagCtxt::new();
        let cx = ctx_for("struct S { x: i32 }\nfn f() {}", &diag);
        let res = cx.resolutions();
        let s = res.defs.iter().position(|d| d.name == "S").unwrap();
        assert_eq!(
            cx.def_kind(crate::hir::DefId(s as u32)),
            Some(DefKind::Struct)
        );
    }
}
