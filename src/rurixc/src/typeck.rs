//! 类型检查 host 子集(spec 条款 RXS-0039 ~ RXS-0047,spec/types.md;07 §3)。
//!
//! - 收集:`fn_sig` / `type_of` 经 [`crate::query::QueryCtx`](provider 在本模块);
//! - 推断:body 内 HM 合一(union-find 推断变量 + 字面量数值类约束,RXS-0041);
//!   body 检查结束时按 RXS-0039 默认化(i32 / f64);
//! - 检查面 = host 子集(函数/struct/enum/泛型单态化雏形,11 §3);
//!   trait bound 仅记录不求解、方法查找仅 inherent、内建运算符不经 trait
//!   (RXS-0045/0046 的 M2.2 口径);
//! - **Err 容忍不级联**(RXS-0047):任一参与类型为 [`Ty::Err`] 时静默通过;
//!   闭包/`loop` 值等未定语义容忍为 Err。`for`/`?` 自 M3.1 在 lower 层
//!   desugar(RXS-0049/0050),本层只见展开后的 loop+match/match 形态。

use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{BinOp, FnColor, LitKind, LitSuffix, UnOp};
use crate::diag::{DiagCtxt, ErrorCode};
use crate::hir::{self, BodyId, DefId, DefKind, HirId, LocalId, PrimTy, Res};
use crate::query::QueryCtx;
use crate::resolve::Resolutions;
use crate::span::Span;
use crate::ty::{FnSig, Ty, TyVid, thread_ctx_dim};

pub const E_MISMATCHED_TYPES: ErrorCode = ErrorCode(2001); // RX2001
pub const E_BAD_FIELD: ErrorCode = ErrorCode(2002); // RX2002
pub const E_ARG_COUNT: ErrorCode = ErrorCode(2003); // RX2003
pub const E_UNKNOWN_METHOD: ErrorCode = ErrorCode(2004); // RX2004
pub const E_ATOMICS_SCOPE: ErrorCode = ErrorCode(3010); // RX3010(RXS-0080)
pub const E_SAMPLE_EXPR: ErrorCode = ErrorCode(3014); // RX3014(RXS-0174,RFC-0007)
pub const E_NONUNIFORM_MISSING: ErrorCode = ErrorCode(3016); // RX3016(RXS-0232,RFC-0013 §4.C1)
pub const E_RHI_CROSS_BRAND: ErrorCode = ErrorCode(3006); // RX3006(复用,RXS-0256 I7;跨 Rhi 实例 brand 误用)
pub const E_NOT_CALLABLE: ErrorCode = ErrorCode(2005); // RX2005
pub const E_BAD_OPERAND: ErrorCode = ErrorCode(2006); // RX2006
pub const E_BAD_DERIVE_COPY: ErrorCode = ErrorCode(2008); // RX2008
pub const E_BAD_DROP_IMPL: ErrorCode = ErrorCode(2009); // RX2009
pub const E_ADDRSPACE_MISMATCH: ErrorCode = ErrorCode(3002); // RX3002(RXS-0067)
pub const E_DEVICE_MATH_UNSUPPORTED: ErrorCode = ErrorCode(6006); // RX6006(RXS-0081)
pub const E_DEVICE_CONSTRAINT: ErrorCode = ErrorCode(6005); // RX6005(RXS-0072)
pub const E_GPU_ELEM_INFER: ErrorCode = ErrorCode(2010); // RX2010(RXS-0190)
pub const E_GPU_LAUNCH_ARG_SUBSET: ErrorCode = ErrorCode(6024); // RX6024(RXS-0191)

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
    /// device intrinsic 调用点(M4.2,RXS-0072):MethodCall 节点 → intrinsic
    /// (接收者为 `ThreadCtx` lang item 时识别);tbir/MIR/codegen 消费。
    pub device_calls: HashMap<HirId, crate::hir::DeviceIntrinsic>,
    /// device 数学 intrinsic 调用点(M5.3,RXS-0081):MethodCall 节点 →
    /// (数学函数, 元素类型 f32/f64);接收者为 `f32`/`f64` 时识别,tbir/MIR/
    /// codegen 消费(下译为 libdevice `__nv_*` 外部符号)。
    pub device_math_calls: HashMap<HirId, (crate::hir::DeviceMathFn, PrimTy)>,
    /// 纹理采样调用点(G2.4,RXS-0174;RFC-0007):MethodCall 节点 → 采样标记
    /// (接收者为 `Texture2D<F>` lang item + 方法 `sample` 时识别;tbir/MIR/codegen
    /// 消费,降为 `Rvalue::ResourceSample` → `OpImageSampleExplicitLod`)。
    pub sample_calls: std::collections::HashSet<HirId>,
    /// 采样方法族调用点(G3.3,RXS-0223;RFC-0013 §4.B1):MethodCall 节点 →
    /// [`crate::mir::ResourceMethod`](接收者为 `Texture2D<F>`/`TextureRw2D<F>`
    /// lang item + 方法 ∈ 方法族时识别;`sample` 本名走既有 [`Self::sample_calls`]
    /// 路,byte-preserving,Q-S-SampleName)。tbir/MIR/codegen 消费,降为
    /// `Rvalue::ResourceSample{method, extra}` → SPIR-V opcode 全家(RXS-0226)。
    pub sample_family_calls: HashMap<HirId, crate::mir::ResourceMethod>,
    /// bindless 无界表动态索引采样调用点(G3.4,RXS-0232;RFC-0013 §4.C1):
    /// MethodCall 节点(receiver = `table[nonuniform(idx)]`,base = `[Texture2D<F>]`
    /// 无界表形参)→ 记录,供 tbir_build 抽取索引值(降为
    /// `Rvalue::ResourceSample{table_index: Some(..)}` → `OpAccessChain` runtime array
    /// + `NonUniform` 装饰 + clamp,RXS-0234)。`sample`/方法族两路共用本标记。
    pub bindless_index_calls: std::collections::HashSet<HirId>,
    /// 宿主 GPU 编排调用点(MS1.2,RXS-0189~0191):Call/MethodCall 节点 → 已知
    /// 操作(接收者为 std::gpu lang item 句柄时识别;用户同名 impl 优先遮蔽)。
    /// tbir/mir_build 消费降级为 `rxrt_*` 调用;coloring 消费裁决宿主 API 着色
    /// 合法性(kernel/device 内出现 → RX3015,RXS-0189)。
    pub gpu_calls: HashMap<HirId, crate::hir::GpuHostOp>,
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
            // const 泛型实参 / per-instance opaque brand(RHI Res<C> 的 `C` = Ty::Const(call_id),
            // RXS-0256):结构相等即数值相等(line 448 口径)。缺此自反臂时,同一 brand 的 RHI 句柄
            // 经 if/else / match 分支合流(demand→unify)会落 `_ => false` 被自相矛盾误拒
            // (「expected Res<K>, found Res<K>」)。跨 brand(distinct Const)仍判异,隔离不弱化。
            (Ty::Const(m), Ty::Const(n)) => m == n,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// HIR 类型降级
// ---------------------------------------------------------------------------

/// HIR 类型 → `Ty`;`infer` 回调裁决 `_` 占位(签名给 Err 容忍,body 给 fresh)。
fn lower_hir_ty(t: &hir::Ty, infer: &mut dyn FnMut() -> Ty) -> Ty {
    lower_hir_ty_with_cx(t, infer, None)
}

/// 带源文本的 HIR 类型 lowering(M5.3:解析 `ConstLit` → [`Ty::Const`])。
fn lower_hir_ty_with_cx(
    t: &hir::Ty,
    infer: &mut dyn FnMut() -> Ty,
    cx: Option<&QueryCtx<'_>>,
) -> Ty {
    match &t.kind {
        hir::TyKind::ConstLit { span } => cx
            .and_then(|c| parse_const_lit_span(c, *span))
            .map(Ty::Const)
            .unwrap_or(Ty::Err),
        hir::TyKind::Res(res, args) => match res {
            Res::PrimTy(p) => Ty::Prim(*p),
            Res::Def(d) => Ty::Adt(
                *d,
                args.iter()
                    .map(|a| lower_hir_ty_with_cx(a, infer, cx))
                    .collect(),
            ),
            Res::GenericParam(i) => Ty::Param(*i),
            // SelfTy/Local/Err:M2.2 容忍(SelfTy 展开随 M2.3)
            _ => Ty::Err,
        },
        hir::TyKind::Ref { mutable, inner } => {
            Ty::Ref(Box::new(lower_hir_ty_with_cx(inner, infer, cx)), *mutable)
        }
        hir::TyKind::RawPtr { mutable, inner } => {
            Ty::RawPtr(Box::new(lower_hir_ty_with_cx(inner, infer, cx)), *mutable)
        }
        hir::TyKind::Tuple(v) => Ty::Tuple(
            v.iter()
                .map(|x| lower_hir_ty_with_cx(x, infer, cx))
                .collect(),
        ),
        hir::TyKind::Array { elem, .. } => {
            Ty::Array(Box::new(lower_hir_ty_with_cx(elem, infer, cx)))
        }
        hir::TyKind::Slice(inner) => Ty::Slice(Box::new(lower_hir_ty_with_cx(inner, infer, cx))),
        hir::TyKind::FnPtr { params, ret } => Ty::FnPtr(
            params
                .iter()
                .map(|x| lower_hir_ty_with_cx(x, infer, cx))
                .collect(),
            Box::new(
                ret.as_ref()
                    .map(|r| lower_hir_ty_with_cx(r, infer, cx))
                    .unwrap_or_else(Ty::unit),
            ),
        ),
        hir::TyKind::Infer => infer(),
        hir::TyKind::Err => Ty::Err,
    }
}

fn parse_const_lit_span(cx: &QueryCtx<'_>, span: Span) -> Option<u64> {
    // 多文件感知切片(RXS-0196):span.file 归属 out-of-line 模块文件时取其源
    let text = cx.snippet(span)?.trim().replace('_', "");
    text.parse().ok()
}

// ---------------------------------------------------------------------------
// query providers(D-203:provider 只经 QueryCtx 互访)
// ---------------------------------------------------------------------------

/// ADT 字段语义类型(定义序,泛型实参已代入;MIR/codegen 布局消费)。
pub fn adt_field_tys(krate: &hir::Crate, def: DefId, args: &[Ty]) -> Vec<Ty> {
    let (hir::ItemKind::Struct { fields } | hir::ItemKind::Variant { fields }) =
        &krate.item(def).kind
    else {
        return Vec::new();
    };
    let mut sig_infer = || Ty::Err;
    fields
        .iter()
        .map(|f| lower_hir_ty(&f.ty, &mut sig_infer).subst(args))
        .collect()
}

/// 宿主 GPU 编排已知方法识别(MS1.2,RXS-0189/0190;`Context::create` 走
/// check_call、`Stream::launch` 走既有 launch 分支,均不在本表)。
fn gpu_host_method(
    li: &crate::resolve::LangItems,
    d: DefId,
    method: &str,
) -> Option<crate::hir::GpuHostOp> {
    use crate::hir::GpuHostOp as Op;
    if li.is_context(d) {
        return match method {
            "stream" => Some(Op::CtxStream),
            "alloc" => Some(Op::CtxAlloc),
            "alloc_pinned" => Some(Op::CtxAllocPinned),
            "sync" => Some(Op::CtxSync),
            // G3.4 bindless(RXS-0235):无界纹理表句柄构造。
            "texture_table" => Some(Op::CtxTextureTable),
            _ => None,
        };
    }
    // G3.4 bindless(RXS-0235):TextureTable 注册面(注册序即索引 / 已注册计数)。
    if li.is_texture_table(d) {
        return match method {
            "register" => Some(Op::TableRegister),
            "len" => Some(Op::TableLen),
            _ => None,
        };
    }
    // G3.5 render graph(RXS-0236):Graph 图本体方法族(资源创建 / pass 声明 / readback /
    // execute)+ PassBuilder 访问声明方法族(五类访问)。
    if li.is_graph(d) {
        return match method {
            "color_target" => Some(Op::GraphColorTarget),
            "depth_target" => Some(Op::GraphDepthTarget),
            "pass" => Some(Op::GraphPass),
            "readback" => Some(Op::GraphReadback),
            "execute" => Some(Op::GraphExecute),
            _ => None,
        };
    }
    if li.is_pass_builder(d) {
        return match method {
            "writes_rt" => Some(Op::PassWritesRt),
            "writes_depth" => Some(Op::PassWritesDepth),
            "reads" => Some(Op::PassReads),
            "reads_writes_uav" => Some(Op::PassReadsWritesUav),
            _ => None,
        };
    }
    // EI1.3 Part B UC-05 RHI(RXS-0256/0257):`Rhi` 图根方法族(资源创建 / pass 声明 /
    // submit)+ `Pass` 访问声明方法族(读 / 写)。与 G3.5 `Graph`/`PassBuilder` 平行的
    // 不同 lang items(compute-pass 面,RFC-0014 §7-2);方法名 `reads`/`writes` 与 graph
    // `PassBuilder::reads` 语义相邻但由接收者 lang item 区分分发。
    if li.is_rhi(d) {
        return match method {
            "resource" => Some(Op::RhiResource),
            "pass" => Some(Op::RhiPass),
            "submit" => Some(Op::RhiSubmit),
            _ => None,
        };
    }
    // EI1.4(RXS-0259/0261):`readback` **归 `Queue<C>`**——即 submit 之后。EI1.3 期 readback
    // 挂在 `Rhi` 上(纯 host 图安全,无数值);EI1.4 派发真实发生在 submit 内,故读回点必须在
    // submit 之后才可能看到计算结果。`submit(self) -> Queue<C>` 的消费式 typestate 使这一执行
    // 序**由类型强制**(submit 前无 `Queue` 可读回,submit 后 `Rhi` 已被消费),1-submit 不变。
    if li.is_rhi_queue(d) && method == "readback" {
        return Some(Op::RhiReadback);
    }
    if li.is_rhi_pass(d) {
        return match method {
            "reads" => Some(Op::RhiPassReads),
            "writes" => Some(Op::RhiPassWrites),
            _ => None,
        };
    }
    if li.is_buffer(d) {
        return match method {
            "upload" => Some(Op::BufUpload),
            "download" => Some(Op::BufDownload),
            "len" => Some(Op::BufLen),
            _ => None,
        };
    }
    if li.is_pinned_buffer(d) {
        return match method {
            "get" => Some(Op::PinnedGet),
            "set" => Some(Op::PinnedSet),
            "len" => Some(Op::PinnedLen),
            _ => None,
        };
    }
    if li.is_stream(d) && method == "sync" {
        return Some(Op::StreamSync);
    }
    // present 宿主 typestate 面(MS1.2b,RXS-0197/0198):帧状态句柄的编译器已知
    // 方法集;消费式转移(ready/wait/signal/present)接收者按值 move(mir_build),
    // 错序 = 编译期 move 违例(RXS-0054,零新码)。
    if Some(d) == li.present && method == "ready" {
        return Some(Op::PresentReady);
    }
    if Some(d) == li.present_ready && method == "wait" {
        return Some(Op::PresentWait);
    }
    if Some(d) == li.present_acquired {
        return match method {
            "backbuffer" => Some(Op::PresentBackbuffer),
            "signal" => Some(Op::PresentSignal),
            _ => None,
        };
    }
    if Some(d) == li.present_presentable {
        return match method {
            "pump" => Some(Op::PresentPump),
            "present" => Some(Op::PresentPresent),
            _ => None,
        };
    }
    None
}

/// `&Res<C>` / `Res<C>` → 内层 brand 类型实参(EI1.3 Part B,RXS-0256;I7 brand 核验源)。
/// 剥可选引用后,若为 `Res` lang item 的 ADT 则取首类型实参(brand);非 `Res` 形态 → `None`
/// (调用方按类型失配 RX2001 裁决)。
fn rhi_res_brand(ty: &Ty, res_def: DefId) -> Option<Ty> {
    let peeled = match ty {
        Ty::Ref(inner, _) => inner.as_ref(),
        other => other,
    };
    match peeled {
        Ty::Adt(d, args) if *d == res_def => Some(args.first().cloned().unwrap_or(Ty::Err)),
        _ => None,
    }
}

/// per-instance brand 相容判定(EI1.3 Part B,RXS-0256;I7)。`Err`/`Infer` 容忍侧一律相容
/// (不级联,承 RXS-0047);其余按结构相等(brand = `Ty::Const` 时即数值相等)。
fn brands_compatible(a: &Ty, b: &Ty) -> bool {
    matches!(a, Ty::Err | Ty::Infer(_)) || matches!(b, Ty::Err | Ty::Infer(_)) || a == b
}

/// 内建函数签名(M2.3 最小 prelude)。
fn builtin_sig(b: hir::Builtin) -> FnSig {
    match b {
        hir::Builtin::Println => FnSig {
            generics_count: 0,
            has_self: false,
            inputs: vec![Ty::Ref(Box::new(Ty::Prim(PrimTy::Str)), false)],
            output: Ty::unit(),
        },
    }
}

/// `fn_sig` provider(RXS-0040/0042)。
pub fn fn_sig_provider(cx: &QueryCtx<'_>, def: DefId) -> FnSig {
    if let Some(b) = cx.resolutions().builtins.get(&def) {
        return builtin_sig(*b);
    }
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
        .map(|t| lower_hir_ty_with_cx(t, &mut sig_infer, Some(cx)))
        .collect();
    FnSig {
        generics_count: decl.generic_params.len() as u32,
        has_self: decl.params.iter().any(|p| p.ty.is_none()),
        inputs,
        output: decl
            .ret
            .as_ref()
            .map(|t| lower_hir_ty_with_cx(t, &mut sig_infer, Some(cx)))
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

/// 定义处检查(M3.2):`#[derive(Copy)]` 合法性(RXS-0053,RX2008)+
/// `impl Drop for T` 形状(RXS-0055,RX2009)。诊断经 DiagCtxt,无产物。
pub fn check_defs_provider(cx: &QueryCtx<'_>) {
    let krate = cx.hir_crate();
    let res = cx.resolutions();
    check_drop_impls(cx, &krate, &res);
    check_copy_derives(cx, &krate, &res);
}

/// `impl Drop for T` 形状校验(RXS-0055):目标为本包 struct/enum、
/// 不重复、impl 体恰一个 `fn drop(&mut self)`(无其余参数,返回 `()`)。
fn check_drop_impls(cx: &QueryCtx<'_>, krate: &hir::Crate, res: &Resolutions) {
    let mut seen: std::collections::HashSet<DefId> = std::collections::HashSet::new();
    for di in &krate.drop_impls {
        let hir::ItemKind::Impl { items, .. } = &krate.item(di.impl_def).kind else {
            continue;
        };
        let emit = |ty: String, reason: &str| {
            cx.diag()
                .struct_error(E_BAD_DROP_IMPL, "typeck.bad_drop_impl")
                .arg("ty", ty)
                .arg("reason", reason)
                .span_label(di.span, reason.to_owned())
                .emit();
        };
        let adt = di.adt.filter(|d| {
            matches!(
                krate.item(*d).kind,
                hir::ItemKind::Struct { .. } | hir::ItemKind::Enum { .. }
            )
        });
        let Some(adt) = adt else {
            emit(
                "this type".to_owned(),
                "`Drop` can only be implemented for a local struct or enum",
            );
            continue;
        };
        let ty_name = format!("`{}`", res.defs[adt.0 as usize].name);
        if !seen.insert(adt) {
            emit(ty_name, "duplicate `Drop` impl for the same type");
            continue;
        }
        let shape_ok = items.len() == 1 && {
            let it = krate.item(items[0]);
            it.name == "drop"
                && matches!(&it.kind, hir::ItemKind::Fn(decl)
                    if matches!(decl.self_kind, Some(hir::SelfKind { by_ref: true, mutable: true }))
                        && decl.params.len() == 1
                        && ret_is_unit(&decl.ret))
        };
        if !shape_ok {
            emit(
                ty_name,
                "a `Drop` impl must contain exactly one `fn drop(&mut self)`",
            );
        }
    }
}

fn ret_is_unit(ret: &Option<hir::Ty>) -> bool {
    match ret {
        None => true,
        Some(t) => matches!(&t.kind, hir::TyKind::Tuple(v) if v.is_empty()),
    }
}

/// `#[derive(Copy)]` 合法性(RXS-0053):全字段 Copy;字段类型引用泛型
/// 参数保守拒绝;与 Drop impl 冲突。
fn check_copy_derives(cx: &QueryCtx<'_>, krate: &hir::Crate, res: &Resolutions) {
    let mut targets: Vec<(DefId, Span)> =
        krate.copy_derives.iter().map(|(d, s)| (*d, *s)).collect();
    targets.sort_by_key(|(d, _)| d.0);
    for (def, span) in targets {
        let ty_name = format!("`{}`", res.defs[def.0 as usize].name);
        let emit = |reason: String| {
            cx.diag()
                .struct_error(E_BAD_DERIVE_COPY, "typeck.bad_derive_copy")
                .arg("ty", ty_name.clone())
                .arg("reason", reason.clone())
                .span_label(span, reason)
                .emit();
        };
        if krate.drop_impl_of(def).is_some() {
            emit("the type also implements `Drop`".to_owned());
            continue;
        }
        let component_defs: Vec<DefId> = match &krate.item(def).kind {
            hir::ItemKind::Struct { .. } => vec![def],
            hir::ItemKind::Enum { variants } => variants.clone(),
            _ => Vec::new(),
        };
        'adt: for cd in component_defs {
            let (hir::ItemKind::Struct { fields } | hir::ItemKind::Variant { fields }) =
                &krate.item(cd).kind
            else {
                continue;
            };
            let mut sig_infer = || Ty::Err;
            for f in fields {
                let ft = lower_hir_ty(&f.ty, &mut sig_infer);
                if mentions_param(&ft) {
                    emit(format!(
                        "field `{}` has a generic type (conservatively rejected)",
                        f.name
                    ));
                    break 'adt;
                }
                if !crate::ty::is_copy(krate, &ft) {
                    emit(format!("field `{}` is not Copy", f.name));
                    break 'adt;
                }
            }
        }
    }
}

/// 类型是否引用泛型参数(RXS-0053 保守拒绝判定)。
fn mentions_param(t: &Ty) -> bool {
    match t {
        Ty::Param(_) => true,
        Ty::Adt(_, args) => args.iter().any(mentions_param),
        Ty::Tuple(v) => v.iter().any(mentions_param),
        Ty::Ref(x, _) | Ty::RawPtr(x, _) | Ty::Array(x) | Ty::Slice(x) => mentions_param(x),
        Ty::FnPtr(ps, r) => ps.iter().any(mentions_param) || mentions_param(r),
        Ty::Const(_) => false,
        _ => false,
    }
}

/// `check_body` provider:对单个 body 做推断与检查,诊断经 DiagCtxt;
/// 产物 [`TypeckResults`] 按节点物化(M2.3,MIR lowering 消费)。
pub fn check_body_provider(cx: &QueryCtx<'_>, body_id: BodyId) -> TypeckResults {
    let krate = cx.hir_crate();
    let res = cx.resolutions();
    let body = krate.body(body_id);
    let owner = krate.item(body.owner);

    let ctx_color = match &owner.kind {
        hir::ItemKind::Fn(decl) => decl.color,
        _ => FnColor::Host,
    };
    let ctx_stage = match &owner.kind {
        hir::ItemKind::Fn(decl) => decl.stage,
        _ => None,
    };

    let mut tck = Tck {
        cx,
        krate: Rc::clone(&krate),
        res: Rc::clone(&res),
        infcx: InferCtxt::default(),
        locals: vec![None; body.locals.len()],
        ret_ty: Ty::Err,
        results: TypeckResults::default(),
        ctx_color,
        ctx_stage,
        gpu_allocs: Vec::new(),
        gpu_launch_args: Vec::new(),
    };

    // 期望返回类型与参数绑定
    match &owner.kind {
        hir::ItemKind::Fn(decl) => {
            let sig = cx.fn_sig(body.owner);
            tck.ret_ty = sig.output.clone();
            // self 接收者:反查所属 inherent impl 的 self 类型;`&self`/`&mut self`
            // 绑定为引用类型(M3.1 收紧——TBIR 方法糖显式化的 autoderef 依据)
            let self_ty = if sig.has_self {
                let base = tck.impl_self_ty(body.owner);
                match decl.self_kind {
                    Some(sk) if sk.by_ref && !base.is_err() => Ty::Ref(Box::new(base), sk.mutable),
                    _ => base,
                }
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

    // 宿主 GPU 编排收尾裁决(MS1.2):元素定型 RX2010(RXS-0190)+ launch 实参
    // 子集 RX6024(RXS-0191)——body 全部使用点约束收齐后统一定型检查。
    tck.check_gpu_deferred();

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
    /// 当前 body 的上下文着色(RXS-0066/0081;device 数学 intrinsic 门禁)。
    ctx_color: FnColor,
    /// 当前 body 的着色阶段(RXS-0174;采样表达式阶段可用性门禁,RFC-0007;
    /// `None` = 普通/非着色阶段函数)。
    ctx_stage: Option<crate::ast::ShaderStage>,
    /// 宿主 GPU 缓冲分配点(MS1.2,RXS-0190):`(alloc 调用 span, 元素类型)`;
    /// body 收尾统一定型检查,不可定型 / 超出首期子集 {f32,i32,u32} → RX2010。
    gpu_allocs: Vec<(Span, Ty)>,
    /// launch 实参记录(MS1.2,RXS-0191):`(实参 span, 类型)`;body 收尾统一
    /// 子集检查,超出 Buffer + {i32,u32,f32,usize} → RX6024。
    gpu_launch_args: Vec<(Span, Ty)>,
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

    fn err_addrspace(&self, span: Span, expected: &str, found: &str) {
        self.diag()
            .struct_error(E_ADDRSPACE_MISMATCH, "addrspace.mismatch")
            .arg("expected", format!("`{expected}`"))
            .arg("found", format!("`{found}`"))
            .span_label(span, format!("expected address space `{expected}`"))
            .emit();
    }

    fn err_device_math_unsupported(&self, span: Span, detail: &str) {
        self.diag()
            .struct_error(E_DEVICE_MATH_UNSUPPORTED, "codegen.device_math_unsupported")
            .arg("detail", detail)
            .span_label(span, "unsupported device math intrinsic")
            .emit();
    }

    fn err_device_constraint(&self, span: Span, detail: &str) {
        self.diag()
            .struct_error(E_DEVICE_CONSTRAINT, "codegen.device_constraint")
            .arg("detail", detail)
            .span_label(span, "device codegen constraint violated")
            .emit();
    }

    /// 采样表达式违例(RXS-0174;RFC-0007):非 fragment 阶段 / sampler 实参非
    /// `Sampler` / 元数不符 → RX3014(strict-only,首期收敛子集外)。
    fn err_sample_expr(&self, span: Span, detail: &str) {
        self.diag()
            .struct_error(E_SAMPLE_EXPR, "shader.sample_expr_invalid")
            .arg("detail", detail)
            .span_label(span, "invalid texture sampling expression")
            .emit();
    }

    /// 采样方法族类型检查(G3.3,RXS-0223;RFC-0013 §4.B1 签名 × 阶段矩阵)。
    /// 违例一律 **RX3014 扩类别**(strict-only,不新增 3xxx 码):接收者 × 方法轴 /
    /// 阶段矩阵 / 元数与实参形态 / 元素 F 分方法限定。`elem` = 接收者首类型实参
    /// (`Texture2D<F>`/`TextureRw2D<F>` 的 F,已 resolve);`rw` = 接收者为
    /// `TextureRw2D<F>`。`sample` 本名走既有 RXS-0174 分支(byte-preserving),
    /// 不进本函数。
    fn check_sample_family(
        &mut self,
        span: Span,
        call_id: HirId,
        method: &str,
        elem: Option<Ty>,
        args: &[hir::Expr],
        rw: bool,
    ) -> Ty {
        use crate::mir::ResourceMethod as M;
        // 实参逐一定型(expr_ty 物化;核验基于定型结果)。
        let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(a)).collect();

        // 接收者 × 方法轴(RXS-0223 矩阵):族内方法配错接收者 → RX3014。
        let m = if rw {
            match method {
                "load" => M::StorageLoad,
                "store" => M::Store,
                _ => {
                    self.err_sample_expr(
                        span,
                        &format!(
                            "`{method}` is not available on `TextureRw2D<F>` (storage image \
                             supports only `load`/`store`; RXS-0223)"
                        ),
                    );
                    return Ty::Err;
                }
            }
        } else if method == "store" {
            self.err_sample_expr(
                span,
                "`store` requires a `TextureRw2D<F>` storage image receiver (`Texture2D<F>` \
                 is a read-only SRV handle; RXS-0223)",
            );
            return Ty::Err;
        } else {
            texture2d_family_method(method).expect("guard 已确保方法族方法")
        };

        // 阶段 × 合法性矩阵(RXS-0223;`TextureRw2D` 阶段列 = fragment + raygen,
        // §4.0-2 一次性钉死;后续阶段扩展点集中于 [`family_stage_note`] / 本 match)。
        let stage_ok = match m {
            // 隐式 LOD 族(quad 导数,🔒 RXS-0227)仅 fragment。
            M::Sample | M::SampleBias => {
                matches!(self.ctx_stage, Some(crate::ast::ShaderStage::Fragment))
            }
            M::SampleLod | M::SampleGrad | M::Load | M::LoadLod | M::SampleCmp | M::Gather => {
                matches!(
                    self.ctx_stage,
                    Some(crate::ast::ShaderStage::Fragment | crate::ast::ShaderStage::Vertex)
                )
            }
            M::StorageLoad | M::Store => matches!(
                self.ctx_stage,
                Some(crate::ast::ShaderStage::Fragment | crate::ast::ShaderStage::RayGen)
            ),
        };
        if !stage_ok {
            self.err_sample_expr(
                span,
                &format!(
                    "`{method}` is not available in this shader stage (RXS-0223 stage matrix: \
                     {})",
                    family_stage_note(m)
                ),
            );
        }

        // 元素 F 分方法限定(Q-S-Element):sample 族限 `f32`(过滤仅对浮点定义);
        // `load`/`store` 族支持 {f32, u32, i32}。无类型实参 = 默认 f32(mir_build
        // `ast_ty_to_resource` 同口径);`Ty::Err` 容忍区。
        let sample_only_f32 = matches!(
            m,
            M::Sample | M::SampleLod | M::SampleGrad | M::SampleBias | M::SampleCmp | M::Gather
        );
        match &elem {
            None | Some(Ty::Err) | Some(Ty::Infer(_)) | Some(Ty::Prim(PrimTy::F32)) => {}
            Some(Ty::Prim(PrimTy::U32 | PrimTy::I32)) if !sample_only_f32 => {}
            Some(other) => {
                let allowed = if sample_only_f32 {
                    "the sample family is limited to `f32` elements"
                } else {
                    "`load`/`store` support {f32, u32, i32} elements"
                };
                self.err_sample_expr(
                    span,
                    &format!(
                        "element type `{}` is not supported for `{method}` ({allowed}; \
                         RXS-0223)",
                        self.render(other)
                    ),
                );
            }
        }

        // 元数 + 实参形态(RFC-0013 §4.B1 签名表)。向量位实参(coord / ddx / ddy /
        // store 值):vec2/vec4 非真实 typeck 类型(承 RXS-0174 名约定,结构性
        // `Ty::Err` 容忍区),故仅拒已定型的非向量实参,精确向量类型由 codegen 层
        // 裁决(RXS-0226/0228 strict-only);标量位实参(lod/bias/dref)经推断合一。
        let (sampler_cmp_kind, arity, sig_note) = match m {
            // `sample` 防御项:本函数不承接(既有 RXS-0174 分支)。
            M::Sample => (Some(false), 2, "(sampler, coord)"),
            M::SampleLod => (Some(false), 3, "(sampler, coord, lod: f32)"),
            M::SampleGrad => (
                Some(false),
                4,
                "(sampler, coord, ddx: vec2<f32>, ddy: vec2<f32>)",
            ),
            M::SampleBias => (Some(false), 3, "(sampler, coord, bias: f32)"),
            M::SampleCmp => (Some(true), 3, "(sampler_cmp, coord, dref: f32)"),
            M::Gather => (Some(false), 3, "(sampler, coord, component: 0..=3 literal)"),
            M::Load | M::StorageLoad => (None, 1, "(coord: vec2<u32>)"),
            M::LoadLod => (None, 2, "(coord: vec2<u32>, lod: u32)"),
            M::Store => (None, 2, "(coord: vec2<u32>, value: vec4<F>)"),
        };
        if args.len() != arity {
            self.err_sample_expr(
                span,
                &format!("`{method}` expects exactly {sig_note} arguments (RXS-0223)"),
            );
        } else {
            if let Some(want_cmp) = sampler_cmp_kind {
                let samp_ty = self.infcx.resolve(&self.autoderef(&arg_tys[0]));
                let ok = match &samp_ty {
                    Ty::Adt(sd, _) => {
                        if want_cmp {
                            self.res.lang_items.is_sampler_cmp(*sd)
                        } else {
                            self.res.lang_items.is_sampler(*sd)
                        }
                    }
                    Ty::Err => true,
                    _ => false,
                };
                if !ok {
                    let want = if want_cmp { "SamplerCmp" } else { "Sampler" };
                    self.err_sample_expr(
                        args[0].span,
                        &format!(
                            "first argument to `{method}` must be a `{want}` handle (RXS-0223)"
                        ),
                    );
                }
            }
            let coord_idx = if sampler_cmp_kind.is_some() { 1 } else { 0 };
            let coord_want = if sample_only_f32 {
                "vec2<f32>"
            } else {
                "vec2<u32>"
            };
            self.expect_vec_arg(
                args[coord_idx].span,
                &arg_tys[coord_idx],
                method,
                "coord",
                coord_want,
            );
            match m {
                M::SampleLod => {
                    self.expect_scalar_arg(args[2].span, &arg_tys[2], PrimTy::F32, method, "lod");
                }
                M::SampleBias => {
                    self.expect_scalar_arg(args[2].span, &arg_tys[2], PrimTy::F32, method, "bias");
                }
                M::SampleCmp => {
                    self.expect_scalar_arg(args[2].span, &arg_tys[2], PrimTy::F32, method, "dref");
                }
                M::LoadLod => {
                    self.expect_scalar_arg(args[1].span, &arg_tys[1], PrimTy::U32, method, "lod");
                }
                M::SampleGrad => {
                    self.expect_vec_arg(args[2].span, &arg_tys[2], method, "ddx", "vec2<f32>");
                    self.expect_vec_arg(args[3].span, &arg_tys[3], method, "ddy", "vec2<f32>");
                }
                M::Gather => self.check_gather_component(&args[2]),
                M::Store => {
                    self.expect_vec_arg(args[1].span, &arg_tys[1], method, "value", "vec4<F>");
                }
                M::Sample | M::Load | M::StorageLoad => {}
            }
        }

        self.results.sample_family_calls.insert(call_id, m);
        match m {
            // 结果标量 f32(RXS-0223:`sample_cmp` 恒 depth 比较,非 vec4)。
            M::SampleCmp => Ty::Prim(PrimTy::F32),
            // `store` 无结果(唯一写者 storage 写,🔒 RXS-0229)。
            M::Store => Ty::unit(),
            // vec4<F> 非真实类型(承 vec2/vec4 名约定结构性);返回容忍区。
            _ => Ty::Err,
        }
    }

    /// 向量位实参核验(RXS-0223 容忍口径):vec2/vec4 非真实 typeck 类型(结构性
    /// `Ty::Err`),故仅拒**已定型的非向量**实参(标量/bool/ADT 等);精确向量
    /// 类型由 codegen 层裁决(RXS-0226/0228 strict-only)。
    fn expect_vec_arg(&self, span: Span, t: &Ty, method: &str, what: &str, want: &str) {
        let r = self.infcx.resolve(t);
        if !matches!(r, Ty::Err | Ty::Infer(_)) {
            self.err_sample_expr(
                span,
                &format!(
                    "`{what}` of `{method}` must be `{want}` (found {}; RXS-0223)",
                    self.render(t)
                ),
            );
        }
    }

    /// 标量位实参核验(lod/bias/dref: f32;`load_lod` 的 lod: u32):与期望原生
    /// 类型推断合一(无后缀字面量经数值类约束绑定);不可合一 → RX3014(违例归
    /// 采样表达式类别,不落 RX2001,RXS-0223)。
    fn expect_scalar_arg(&mut self, span: Span, t: &Ty, want: PrimTy, method: &str, what: &str) {
        if !self.infcx.unify(&Ty::Prim(want), t) {
            self.err_sample_expr(
                span,
                &format!(
                    "`{what}` of `{method}` must be `{}` (found {}; RXS-0223)",
                    self.render(&Ty::Prim(want)),
                    self.render(t)
                ),
            );
        }
    }

    /// gather 分量实参(RXS-0223):须 0..=3 **整型字面量**(非字面量 / 越界 →
    /// RX3014;codegen 层 `gather_component` 再核常量形态,双保险)。
    fn check_gather_component(&self, arg: &hir::Expr) {
        let ok = match &arg.kind {
            hir::ExprKind::Lit(l) if l.kind == LitKind::Int => self
                .cx
                .snippet(l.span)
                .and_then(|t| crate::mir_build::parse_int(t, l.suffix))
                .is_some_and(|v| (0..=3).contains(&v)),
            _ => false,
        };
        if !ok {
            self.err_sample_expr(
                arg.span,
                "`component` of `gather` must be an integer literal in 0..=3 (RXS-0223)",
            );
        }
    }

    fn is_device_ctx(&self) -> bool {
        matches!(self.ctx_color, FnColor::Device | FnColor::Kernel)
    }

    /// 地址空间不一致检测(RXS-0067):两侧为同一 `View` 族容器(同可变性)
    /// 而首类型实参(地址空间标记)不同 → `RX3002` 特化诊断(优先于 RX2001)。
    fn try_addrspace_mismatch(&self, expected: &Ty, found: &Ty) -> Option<(String, String)> {
        let e = self.infcx.resolve(expected);
        let f = self.infcx.resolve(found);
        let (Ty::Adt(de, ae), Ty::Adt(df, af)) = (&e, &f) else {
            return None;
        };
        let li = &self.res.lang_items;
        if de != df || li.view_mutable(*de).is_none() {
            return None;
        }
        let space_name = |args: &[Ty]| -> Option<&'static str> {
            match args.first() {
                Some(Ty::Adt(d, _)) => li.addr_space_name(*d),
                _ => None,
            }
        };
        match (space_name(ae), space_name(af)) {
            (Some(a), Some(b)) if a != b => Some((a.to_owned(), b.to_owned())),
            _ => None,
        }
    }

    /// 合一并按 RX2001 报错(Err 容忍内建于 unify);`View` 族地址空间不一致
    /// 特化为 RX3002(RXS-0067,先于通用类型不匹配诊断)。
    fn demand(&mut self, span: Span, expected: &Ty, found: &Ty) {
        if let Some((exp, fnd)) = self.try_addrspace_mismatch(expected, found) {
            self.err_addrspace(span, &exp, &fnd);
            return;
        }
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
            if let hir::ItemKind::Impl {
                self_res, items, ..
            } = &item.kind
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

    /// ADT 的泛型实例化槽位数(MVP 推定):struct 取自身字段;enum 取**全部
    /// 变体字段的最大值**(单变体字段不必提满参数,如 `Result` 的 `Ok(T)`);
    /// variant 归并到父 enum 口径——保证同一 enum 的各变体构造出一致的实参数。
    fn adt_slots(&self, def: DefId) -> u32 {
        match &self.krate.item(def).kind {
            hir::ItemKind::Struct { fields } => self.generic_slots(fields),
            hir::ItemKind::Enum { variants } => variants
                .iter()
                .map(|v| match &self.krate.item(*v).kind {
                    hir::ItemKind::Variant { fields } => self.generic_slots(fields),
                    _ => 0,
                })
                .max()
                .unwrap_or(0),
            hir::ItemKind::Variant { fields } => self
                .res
                .variant_parents
                .get(&def)
                .map(|e| self.adt_slots(*e))
                .unwrap_or_else(|| self.generic_slots(fields)),
            _ => 0,
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

    /// 构造器模式与被匹配类型的相容性(RXS-0050/0051 前置):模式的 ADT
    /// (实例化 fresh 槽位)与 scrutinee 合一 → 违例 RX2001;Err 容忍内建。
    /// 副作用:把未定型 scrutinee 推到正确的 ADT 形态(字段类型分解的前提)。
    fn pat_ctor_compat(&mut self, pat: &hir::Pat, res: &Res, ty: &Ty) {
        let Res::Def(d) = res else { return };
        let kind = self.res.defs[d.0 as usize].kind;
        if !matches!(kind, DefKind::Variant | DefKind::Struct) {
            return;
        }
        let slots = self.adt_slots(*d);
        let fresh: Vec<Ty> = (0..slots).map(|_| self.infcx.fresh(None)).collect();
        let expect = self.ctor_result(*d, fresh);
        self.demand(pat.span, &expect, ty);
    }

    fn bind_pat(&mut self, pat: &hir::Pat, ty: &Ty) {
        self.results.pat_ty.insert(pat.hir_id, ty.clone());
        match &pat.kind {
            hir::PatKind::Binding { local } => self.set_local(*local, ty.clone()),
            hir::PatKind::Wild
            | hir::PatKind::Lit { .. }
            | hir::PatKind::Range
            | hir::PatKind::Err => {}
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
            hir::PatKind::Res(r) => self.pat_ctor_compat(pat, r, ty),
            hir::PatKind::TupleStruct { res, elems } => {
                self.pat_ctor_compat(pat, res, ty);
                let field_tys = self.ctor_field_tys(res, ty);
                for (i, p) in elems.iter().enumerate() {
                    self.bind_pat(p, field_tys.get(i).unwrap_or(&Ty::Err));
                }
            }
            hir::PatKind::Struct { res, fields, .. } => {
                self.pat_ctor_compat(pat, res, ty);
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
            // desugar 合成推进步(RXS-0049):同无后缀整数字面量
            hir::ExprKind::SynthInt(_) => self.infcx.fresh(Some(NumClass::Int)),
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
                let based = self.autoderef(&bt);
                // G3.4 逃逸(RXS-0232;RFC-0013 §4.C1):无界纹理表索引 `table[idx]` 产临时
                // 句柄,**仅立即 sample-family receiver 合法**(该位由 [`Self::check_method`]
                // 拦截,不达此臂)。出现在此臂 = 句柄逃逸至 let / 实参 / 字段等非 receiver
                // 位 → RX3014 扩类别(句柄非值纪律 RXS-0156/0174 不破)。不 demand usize
                // (索引 u32),避免二次 RX2001 级联。
                if let Ty::Slice(inner) = &based
                    && matches!(self.infcx.resolve(inner), Ty::Adt(d, _)
                        if self.res.lang_items.is_texture2d(d))
                {
                    let _ = self.check_expr(index);
                    self.err_sample_expr(
                        e.span,
                        "bindless table index handle may only be the immediate receiver of a \
                         sample-family method (it cannot be let-bound, passed, or stored; RXS-0232)",
                    );
                    return Ty::Err;
                }
                let it = self.check_expr(index);
                self.demand(index.span, &Ty::Prim(PrimTy::Usize), &it);
                match based {
                    Ty::Array(t) | Ty::Slice(t) => *t,
                    // `View<space, T, ..>` / `ViewMut<space, T, ..>` 索引(M4.2,
                    // RXS-0071):元素类型 = 第二类型实参(args[0] = 地址空间标记)。
                    Ty::Adt(d, args)
                        if self.res.lang_items.view_mutable(d).is_some() && args.len() >= 2 =>
                    {
                        args[1].clone()
                    }
                    // 其余 Adt 索引(运算符 trait 形态)M2.2 容忍
                    _ => Ty::Err,
                }
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
                DefKind::Variant => {
                    // 单元变体值:按父 enum 槽位实例化 fresh(`None` 可与
                    // `Option<i32>` 等标注合一,RXS-0048/0044)
                    let slots = self.adt_slots(*d);
                    let fresh: Vec<Ty> = (0..slots).map(|_| self.infcx.fresh(None)).collect();
                    self.ctor_result(*d, fresh)
                }
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
        // G3.4 bindless(RXS-0232):`nonuniform(idx)` 身份标注——返回实参类型
        // (`table[nonuniform(idx)]` 采样 receiver 位由 [`Self::check_method`] 拦截并
        // 抽取内层,不达此处;此臂仅兜底 nonuniform 用于非索引位的容忍定型)。
        if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
            && self.res.lang_items.is_nonuniform(*d)
        {
            if args.len() != 1 {
                self.err_arg_count(span, 1, args.len());
            }
            let mut ty = Ty::Err;
            for (i, a) in args.iter().enumerate() {
                let t = self.check_expr(a);
                if i == 0 {
                    ty = t;
                }
            }
            return ty;
        }
        // launch 维度构造器(M4.3,RXS-0074):`GridDim(..)`/`BlockDim(..)` 变维数
        // 容忍——维数 = 实参个数(launch_check 结构化读取);typeck 仅核对实参可
        // 定型,不按 0 字段 struct 构造器报 arity(防 RX2003 误报)。
        if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
            && self.res.lang_items.is_launch_dim(*d)
        {
            for a in args {
                let _ = self.check_expr(a);
            }
            return Ty::Adt(*d, Vec::new());
        }
        // 宿主 GPU 上下文构造(MS1.2,RXS-0189/0190):`Context::create()` 编译器
        // 已知关联函数——0 实参,产 `Context` 句柄。brand 取单 brand 方案(RFC-0009
        // §9 Q-Brand 降级路):`Context` 自身即 brand 类型,跨 context 资源误用由
        // cabi 运行期 ctx-id 校验确定性拦截(RXS-0194);泛型签名 brand 契约
        // (RX3006,RXS-0074)不受影响。
        if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
            && Some(*d) == self.res.lang_items.context_create
        {
            if !args.is_empty() {
                self.err_arg_count(span, 0, args.len());
                for a in args {
                    let _ = self.check_expr(a);
                }
            }
            self.results
                .gpu_calls
                .insert(call_id, crate::hir::GpuHostOp::CtxCreate);
            let ctx_def = self
                .res
                .lang_items
                .context
                .expect("Context lang item 在 resolve 入口注入");
            return Ty::Adt(ctx_def, Vec::new());
        }
        // present 会话构造(MS1.2b,RXS-0197):`Present::create(&ctx, rw, rh,
        // ww, wh)` 编译器已知关联函数——首实参 `&Context`(单 brand 方案沿
        // RXS-0189),四维度实参 u32,产 `Present` 句柄。
        if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
            && Some(*d) == self.res.lang_items.present_create
        {
            self.results
                .gpu_calls
                .insert(call_id, crate::hir::GpuHostOp::PresentCreate);
            let ctx_def = self
                .res
                .lang_items
                .context
                .expect("Context lang item 在 resolve 入口注入");
            let present_def = self
                .res
                .lang_items
                .present
                .expect("Present lang item 在 resolve 入口注入");
            let u32t = Ty::Prim(PrimTy::U32);
            let expected = [
                Ty::Ref(Box::new(Ty::Adt(ctx_def, Vec::new())), false),
                u32t.clone(),
                u32t.clone(),
                u32t.clone(),
                u32t,
            ];
            return self.check_args(span, &expected, args, Ty::Adt(present_def, Vec::new()));
        }
        // render graph 图构造(G3.5,RXS-0236):`Graph::create(&ctx)` 编译器已知关联函数——
        // 首实参 `&Context`(单 brand 方案沿 RXS-0189),产 `Graph<C>` 句柄(非 Copy affine)。
        if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
            && Some(*d) == self.res.lang_items.graph_create
        {
            self.results
                .gpu_calls
                .insert(call_id, crate::hir::GpuHostOp::GraphCreate);
            let ctx_def = self
                .res
                .lang_items
                .context
                .expect("Context lang item 在 resolve 入口注入");
            let graph_def = self
                .res
                .lang_items
                .graph
                .expect("Graph lang item 在 resolve 入口注入");
            let brand = Ty::Adt(ctx_def, Vec::new());
            let expected = [Ty::Ref(Box::new(brand.clone()), false)];
            return self.check_args(span, &expected, args, Ty::Adt(graph_def, vec![brand]));
        }
        // UC-05 RHI 图根构造(EI1.3 Part B,RXS-0256):`Rhi::create(&ctx)` 编译器已知关联
        // 函数——首实参 `&Context`,产 `Rhi<C>` 句柄(非 Copy affine)。**per-instance 新鲜
        // opaque brand 类型 `C`**:brand 取本调用点 `call_id` 派生的 `Ty::Const`(每 `Rhi::create`
        // 调用点唯一 → 跨 `Rhi` 实例的资源/pass 携不同 brand,`reads`/`writes` 处编译期 RX3006
        // 拦截,I7,RFC-0014 §4.B1;区别于 G3.5 `Graph` 单 brand + 运行期 ctx-id 校验路)。
        if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
            && Some(*d) == self.res.lang_items.rhi_create
        {
            self.results
                .gpu_calls
                .insert(call_id, crate::hir::GpuHostOp::RhiCreate);
            let ctx_def = self
                .res
                .lang_items
                .context
                .expect("Context lang item 在 resolve 入口注入");
            let rhi_def = self
                .res
                .lang_items
                .rhi
                .expect("Rhi lang item 在 resolve 入口注入");
            // per-instance 新鲜 brand:调用点 HirId → Const(编译期唯一,类型层记账,不入 codegen)。
            let brand = Ty::Const(u64::from(call_id.0));
            let expected = [Ty::Ref(Box::new(Ty::Adt(ctx_def, Vec::new())), false)];
            return self.check_args(span, &expected, args, Ty::Adt(rhi_def, vec![brand]));
        }
        // 宿主图像落盘桥(MS1.2b,RXS-0199):`write_ppm(path, w, h, &pinned)`
        // 编译器已知自由函数;data 形参位为元素类型使用点约束(RXS-0190:
        // 未定元素在此定型 f32)。
        if let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
            && Some(*d) == self.res.lang_items.write_ppm
        {
            self.results
                .gpu_calls
                .insert(call_id, crate::hir::GpuHostOp::WritePpm);
            let ctx_def = self
                .res
                .lang_items
                .context
                .expect("Context lang item 在 resolve 入口注入");
            let pinned_def = self
                .res
                .lang_items
                .pinned_buffer
                .expect("PinnedBuffer lang item 在 resolve 入口注入");
            let u32t = Ty::Prim(PrimTy::U32);
            let expected = [
                Ty::Ref(Box::new(Ty::Prim(PrimTy::Str)), false),
                u32t.clone(),
                u32t,
                Ty::Ref(
                    Box::new(Ty::Adt(
                        pinned_def,
                        vec![Ty::Adt(ctx_def, Vec::new()), Ty::Prim(PrimTy::F32)],
                    )),
                    false,
                ),
            ];
            return self.check_args(span, &expected, args, Ty::unit());
        }
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
                    // 先收集字段类型(owned),再生成 fresh 槽位(借用解耦);
                    // 槽位按 ADT 口径(variant 归并父 enum,RXS-0048/0045)
                    let collected = self.fields_of(*d).map(|fields| {
                        let mut sig_infer = || Ty::Err;
                        let raw: Vec<Ty> = fields
                            .iter()
                            .map(|f| lower_hir_ty(&f.ty, &mut sig_infer))
                            .collect();
                        let slots = self.adt_slots(*d);
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

    /// G3.4 bindless(RXS-0232;RFC-0013 §4.C1):无界表动态索引 `table[<idx>]` 的
    /// nonuniform 标注校验 + 索引值 u32 定型 + bindless 标记记录。`<idx>` 须以
    /// `nonuniform(expr)` 包裹(唯一豁免 = 整型字面量常量索引);缺失 → RX3016
    /// strict-only(不做 uniformity 推断,保守全标合法,Q-B-Uniformity)。
    fn check_bindless_index(&mut self, call_id: HirId, index: &hir::Expr) {
        let value = match nonuniform_inner(index, &self.res.lang_items) {
            // `nonuniform(idx)`:取内层索引值。
            Some(inner) => inner,
            // 整型字面量常量索引:豁免标注(波内恒均匀,SPIR-V 合法)。
            None if is_int_literal(index) => index,
            // 缺失标注:strict-only 拒(RX3016)。仍定型索引(单错单报,继续族校验)。
            None => {
                self.diag()
                    .struct_error(E_NONUNIFORM_MISSING, "shader.nonuniform_annotation_missing")
                    .arg(
                        "detail",
                        "bindless table dynamic index must be wrapped in `nonuniform(...)` \
                         (only integer-literal constant indices are exempt; RXS-0232)",
                    )
                    .span_label(index.span, "un-annotated non-uniform bindless index")
                    .emit();
                index
            }
        };
        // 索引值 : u32(RFC-0013 §4.C1)。
        let it = self.check_expr(value);
        self.demand(value.span, &Ty::Prim(PrimTy::U32), &it);
        self.results.bindless_index_calls.insert(call_id);
    }

    fn check_method(
        &mut self,
        span: Span,
        call_id: HirId,
        receiver: &hir::Expr,
        method: &str,
        args: &[hir::Expr],
    ) -> Ty {
        // G3.4 bindless(RXS-0232;RFC-0013 §4.C1):无界表动态索引临时句柄仅立即
        // receiver——`table[nonuniform(idx)].sample(...)`。在此对 Index-receiver 特判:
        // base 定型恰一次(避免二次定型),元素句柄类型 `Texture2D<F>` 作 receiver 走
        // 下方既有采样方法族 / `.sample()` 臂;bindless 标记 + nonuniform 校验单独记录。
        // 逃逸(非立即 receiver 的 `table[idx]`)由通用 `ExprKind::Index` 臂拒(RX3014)。
        let rt = if let hir::ExprKind::Index { expr: base, index } = &receiver.kind {
            let bt = self.check_expr(base);
            let based = self.infcx.resolve(&self.autoderef(&bt));
            // G3.4 bindless:base = `[Texture2D<F>]` 无界表 → 元素句柄 receiver + 校验;
            // 其余索引 receiver(View/ViewMut/Array/Slice/运算符)= 复用既有 Index
            // 元素类型规则(**不含逃逸检查**——receiver 位是唯一合法位)。
            if let Ty::Slice(inner) = &based
                && matches!(self.infcx.resolve(inner), Ty::Adt(d, _)
                    if self.res.lang_items.is_texture2d(d))
            {
                self.check_bindless_index(call_id, index);
                self.infcx.resolve(inner)
            } else {
                let it = self.check_expr(index);
                self.demand(index.span, &Ty::Prim(PrimTy::Usize), &it);
                match based {
                    Ty::Array(t) | Ty::Slice(t) => *t,
                    // `View`/`ViewMut<space, T, ..>` 索引(M4.2,RXS-0071):元素 = args[1]。
                    Ty::Adt(d, args)
                        if self.res.lang_items.view_mutable(d).is_some() && args.len() >= 2 =>
                    {
                        args[1].clone()
                    }
                    _ => Ty::Err,
                }
            }
        } else {
            self.check_expr(receiver)
        };
        // 数值类未定变量按 RXS-0039 默认化后再查方法(原生类型无 inherent
        // 方法 → RX2004;无类约束的推断变量维持容忍)
        let base = self.infcx.resolve(&self.autoderef(&rt));
        match &base {
            // device 线程上下文 intrinsic(M4.2,RXS-0072):`ThreadCtx` 方法 →
            // sreg/barrier intrinsic(用户同名定义优先 = 先查 assoc_items,
            // 命中则不走 intrinsic 路径;此处仅在无用户 impl 时兜底)。
            Ty::Adt(d, _)
                if self.res.lang_items.is_thread_ctx(*d)
                    && self
                        .res
                        .assoc_items
                        .get(d)
                        .is_none_or(|items| !items.iter().any(|(n, _)| n == method))
                    && crate::hir::DeviceIntrinsic::from_method(method).is_some() =>
            {
                for a in args {
                    let _ = self.check_expr(a);
                }
                let intr = crate::hir::DeviceIntrinsic::from_method(method)
                    .expect("guard 已确保 intrinsic 存在");
                if let Some(dim) = thread_ctx_dim(&base, &self.res.lang_items) {
                    let need = intr.min_dim();
                    if dim < need {
                        self.err_device_constraint(
                            span,
                            &format!(
                                "ThreadCtx<{dim}> does not provide axis required by `{method}` \
                                 (needs DIM >= {need}, RXS-0072)"
                            ),
                        );
                    }
                }
                self.results.device_calls.insert(call_id, intr);
                if intr.returns_unit() {
                    Ty::unit()
                } else {
                    Ty::Prim(PrimTy::Usize)
                }
            }
            // 宿主 GPU 编排编译器已知签名(MS1.2,RXS-0189/0190):`Context` /
            // `Buffer` / `PinnedBuffer` / `Stream` 句柄的方法集(先例 ThreadCtx/
            // Atomic 分支;用户同名 impl 优先遮蔽 = 先查 assoc_items)。launch 类型
            // 契约维持下方既有分支。
            Ty::Adt(d, adt_args)
                if gpu_host_method(&self.res.lang_items, *d, method).is_some()
                    && self
                        .res
                        .assoc_items
                        .get(d)
                        .is_none_or(|items| !items.iter().any(|(n, _)| n == method)) =>
            {
                let op = gpu_host_method(&self.res.lang_items, *d, method)
                    .expect("guard 已确保已知 gpu 方法");
                let adt_args = adt_args.clone();
                self.check_gpu_method(span, call_id, op, &adt_args, args)
            }
            // launch 类型契约(M4.3,RXS-0074):`Stream` 接收者的 `launch` 方法
            // 由 launch_check 结构化裁决(着色/维度/参数/brand);typeck 容忍
            // (不报方法未找到),递归核对实参可定型,返回 unit。
            Ty::Adt(d, _) if self.res.lang_items.is_stream(*d) && method == "launch" => {
                for a in args {
                    let _ = self.check_expr(a);
                }
                // 宿主执行语义标记 + 元素推断增强(MS1.2,RXS-0190/0191):launch 为
                // gpu 宿主 API(kernel/device 内出现 → RX3015,RXS-0189);形态完整时
                // 把**未定**元素类型的实参与 kernel 形参静默合一(已定型不动——
                // RX2001 仍由 launch_check 裁决,RXS-0074/0075 零漂移),并记录实参
                // 供收尾子集检查(RX6024)。
                self.results
                    .gpu_calls
                    .insert(call_id, crate::hir::GpuHostOp::Launch);
                self.gpu_launch_infer(args);
                Ty::unit()
            }
            // device block barrier(M5.2,RXS-0079):`block.sync()` 兜底识别为
            // block 级 barrier intrinsic(用户同名定义优先 = 先查 assoc_items)。
            // barrier 的 uniform 可达延续 RXS-0068(coloring),shared+barrier 一致性
            // 数据流由 [`crate::shared_check`] 裁决;此处仅定型 + 记录 intrinsic。
            Ty::Adt(d, _)
                if self.res.lang_items.is_block_ctx(*d)
                    && self
                        .res
                        .assoc_items
                        .get(d)
                        .is_none_or(|items| !items.iter().any(|(n, _)| n == method))
                    && crate::hir::DeviceIntrinsic::from_method(method)
                        == Some(crate::hir::DeviceIntrinsic::Barrier) =>
            {
                for a in args {
                    let _ = self.check_expr(a);
                }
                self.results
                    .device_calls
                    .insert(call_id, crate::hir::DeviceIntrinsic::Barrier);
                Ty::unit()
            }
            // scoped atomics 类型契约(M5.2,RXS-0080):`Atomic<T,Scope>` /
            // `AtomicView<space,T,Shape>` 的原子读改写方法(用户同名定义优先遮蔽,
            // 先查 assoc_items)。裁决 scope 误用(RX3010);PTX atom 映射为 D-406
            // 禁区(本分支不实现映射,仅类型契约 + 元素类型回填)。
            Ty::Adt(d, adt_args)
                if self.res.lang_items.atomic_kind(*d).is_some()
                    && self
                        .res
                        .assoc_items
                        .get(d)
                        .is_none_or(|items| !items.iter().any(|(n, _)| n == method))
                    && crate::hir::AtomicOp::from_method(method).is_some() =>
            {
                let is_view = self
                    .res
                    .lang_items
                    .atomic_kind(*d)
                    .expect("guard 已确保 atomic 容器");
                let adt_args = adt_args.clone();
                self.check_atomic_op(is_view, &adt_args, args);
                // 元素类型:`AtomicView<space,T,..>` → args[1];`Atomic<T,..>` → args[0]。
                let elem_idx = if is_view { 1 } else { 0 };
                adt_args
                    .get(elem_idx)
                    .cloned()
                    .unwrap_or_else(|| self.infcx.fresh(None))
            }
            // device views 算子(M5.1,RXS-0078):`View`/`ViewMut` 的子 view 划分
            // 方法(split_at/chunks/windows;用户同名 impl 优先,故先查 assoc_items)。
            // 返回子 view 类型(与接收者同 space/elem/可变性);不相交性由 views
            // 不相交 device 借用扩展 pass(见 [`crate::views_check`])裁决。
            Ty::Adt(d, _)
                if self.res.lang_items.view_mutable(*d).is_some()
                    && self
                        .res
                        .assoc_items
                        .get(d)
                        .is_none_or(|items| !items.iter().any(|(n, _)| n == method))
                    && crate::hir::ViewOp::from_method(method).is_some() =>
            {
                let op = crate::hir::ViewOp::from_method(method).expect("guard 已确保算子存在");
                // 划分实参(mid / n)须为 usize(RXS-0078;`mid`/`n` 下标域)。
                for a in args {
                    let at = self.check_expr(a);
                    self.demand(a.span, &Ty::Prim(PrimTy::Usize), &at);
                }
                let sub_view = base.clone();
                match op {
                    // split_at → (lo, hi) 两个子 view;chunks/windows → 单一代表
                    // 子 view 形态(序列容器留后续,RXS-0078 MVP)。
                    crate::hir::ViewOp::SplitAt => Ty::Tuple(vec![sub_view.clone(), sub_view]),
                    crate::hir::ViewOp::Chunks | crate::hir::ViewOp::Windows => sub_view,
                }
            }
            // device 数学函数 intrinsic(M5.3,RXS-0081):`f32`/`f64` 接收者的
            // 数学方法(sqrt/exp/fma/...)→ libdevice `__nv_*` 外部符号。原生类型
            // 无用户 inherent impl(无遮蔽问题);device-only(host 数学走 M7 标准库,
            // 本识别面记录后由 device codegen 消费,host codegen 不产出)。
            Ty::Prim(p @ (PrimTy::F32 | PrimTy::F64))
                if crate::hir::DeviceMathFn::from_method(method).is_some() =>
            {
                let op = crate::hir::DeviceMathFn::from_method(method)
                    .expect("guard 已确保数学 intrinsic 存在");
                if !self.is_device_ctx() {
                    self.err_device_math_unsupported(
                        span,
                        "device math intrinsics require device or kernel context (RXS-0081)",
                    );
                    return Ty::Prim(*p);
                }
                let elem = *p;
                // 实参与 receiver 同浮点元素类型(RXS-0081 签名契约);元数 = arity-1。
                for a in args {
                    let at = self.check_expr(a);
                    self.demand(a.span, &Ty::Prim(elem), &at);
                }
                if args.len() + 1 != op.arity() {
                    self.err_arg_count(span, op.arity() - 1, args.len());
                }
                self.results.device_math_calls.insert(call_id, (op, elem));
                Ty::Prim(elem)
            }
            // 纹理采样 intrinsic(G2.4,RXS-0174;RFC-0007):`Texture2D<F>` 接收者的
            // `sample(samp, coord)` 方法 → 采样表达式,产 vec4<F>。原生 lang item
            // 句柄无用户 inherent impl(无遮蔽问题);首期仅 fragment 阶段可采样、
            // samp 实参须 `Sampler`、恰 2 实参;违例 RX3014(strict-only)。vec4<F>
            // 非真实类型(承 vec 名约定,结构性 Ty::Err),codegen 层裁决类型/越界。
            Ty::Adt(d, _) if self.res.lang_items.is_texture2d(*d) && method == "sample" => {
                let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(a)).collect();
                // 阶段可用性:首期仅 fragment 可采样(RXS-0174)。
                if self.ctx_stage != Some(crate::ast::ShaderStage::Fragment) {
                    self.err_sample_expr(
                        span,
                        "texture sampling is only available in `fragment` shader stage \
                         (RXS-0174; first-phase convergent subset)",
                    );
                }
                // 元数 + sampler 实参类型核对。
                if arg_tys.len() != 2 {
                    self.err_sample_expr(
                        span,
                        "`sample` expects exactly (sampler, coord) arguments (RXS-0174)",
                    );
                } else {
                    let samp_ty = self.infcx.resolve(&self.autoderef(&arg_tys[0]));
                    let is_samp =
                        matches!(&samp_ty, Ty::Adt(sd, _) if self.res.lang_items.is_sampler(*sd));
                    // coord(arg[1])为 vec2<f32>(结构性 Ty::Err 容忍,codegen 层裁决)。
                    if !is_samp && !matches!(samp_ty, Ty::Err) {
                        self.err_sample_expr(
                            span,
                            "first argument to `sample` must be a `Sampler` handle (RXS-0174)",
                        );
                    }
                }
                self.results.sample_calls.insert(call_id);
                // vec4<F> 非真实类型(承 vec2/vec4 名约定结构性);返回容忍区。
                Ty::Err
            }
            // 采样方法族(G3.3,RXS-0223;RFC-0013 §4.B1):`Texture2D<F>` 接收者的
            // 新方法(sample_lod/sample_grad/sample_bias/load/load_lod/sample_cmp/
            // gather;`sample` 本名走上方既有 RXS-0174 分支,byte-preserving,
            // Q-S-SampleName)。`store` 配 Texture2D(只读 SRV 轴)= 族内违例 →
            // RX3014。原生 lang item 句柄无用户 inherent impl(无遮蔽问题)。
            Ty::Adt(d, adt_args)
                if self.res.lang_items.is_texture2d(*d)
                    && (texture2d_family_method(method).is_some() || method == "store") =>
            {
                let elem = adt_args.first().map(|t| self.infcx.resolve(t));
                self.check_sample_family(span, call_id, method, elem, args, false)
            }
            // 采样方法族(G3.3,RXS-0223):`TextureRw2D<F>` storage image 接收者的
            // `load`/`store`(阶段列 fragment + raygen,§4.0-2);sample 族方法配
            // rw 接收者 = 族内违例 → RX3014。
            Ty::Adt(d, adt_args)
                if self.res.lang_items.is_texture_rw2d(*d) && is_sample_family_name(method) =>
            {
                let elem = adt_args.first().map(|t| self.infcx.resolve(t));
                self.check_sample_family(span, call_id, method, elem, args, true)
            }
            Ty::Adt(d, _adt_args) => {
                let found = self
                    .res
                    .assoc_items
                    .get(d)
                    .and_then(|items| items.iter().find(|(n, _)| n == method))
                    .map(|(_, m)| *m)
                    // Drop::drop 不可显式调用(RXS-0055;查找面自然拒绝 → RX2004)
                    .filter(|m| !self.krate.is_drop_fn(*m));
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

    /// scoped atomics scope 类型契约裁决(M5.2,RXS-0080;RX3010)。保守上界:
    /// 仅在 scope 实参可静态判定(`Scope::*` 字面变体)时裁决,scope 不可判 /
    /// 参与类型容忍区 `Err` → 不报(防一错多报,口径同 RXS-0069/0075)。
    /// PTX `atom.{order}.{scope}` 映射为 D-406 禁区,本函数不实现映射语义。
    fn check_atomic_op(&mut self, is_view: bool, adt_args: &[Ty], args: &[hir::Expr]) {
        // 实参定型(不级联:scope 实参为 `Scope` 封闭枚举值)。
        for a in args {
            let _ = self.check_expr(a);
        }
        // scope 实参:首个解析为 `Scope::*` 变体的实参(scope 位通常居末)。
        let used = args.iter().find_map(|a| {
            if let hir::ExprKind::Res(Res::Def(d)) = &a.kind {
                self.res.lang_items.scope_rank(*d).map(|r| (r, a.span))
            } else {
                None
            }
        });
        let Some((used_rank, scope_span)) = used else {
            return; // scope 不可静态判定:保守容忍(不误报)。
        };
        // 规则 A(与地址空间不相容):`AtomicView<shared, ..>`(addrspace 3)为
        // block 本地存储,使用宽于 `Scope::Block` 的作用域 → 越权 / 不相容。
        if is_view
            && let Some(space) = adt_args.first()
            && let Ty::Adt(sd, _) = space
            && self.res.lang_items.addr_spaces[1] == Some(*sd)
            && used_rank > 0
        {
            self.diag()
                .struct_error(E_ATOMICS_SCOPE, "atomics.scope_misuse")
                .arg(
                    "detail",
                    format!(
                        "`shared` atomics are block-local; scope `Scope::{}` exceeds `Scope::Block`",
                        scope_name(used_rank)
                    ),
                )
                .span_label(scope_span, "scope incompatible with `shared` address space")
                .emit();
            return;
        }
        // 规则 B(越权作用域):`Atomic<T, Scope::G>` 的 scope brand 由第二类型实参
        // 携带;原子操作使用宽于 brand 的作用域 → 越权未授予的作用域。
        if !is_view
            && let Some(Ty::Adt(brand, _)) = adt_args.get(1)
            && let Some(brand_rank) = self.res.lang_items.scope_rank(*brand)
            && used_rank > brand_rank
        {
            self.diag()
                .struct_error(E_ATOMICS_SCOPE, "atomics.scope_misuse")
                .arg(
                    "detail",
                    format!(
                        "atomic grants scope `Scope::{}`, but operation uses wider `Scope::{}`",
                        scope_name(brand_rank),
                        scope_name(used_rank)
                    ),
                )
                .span_label(scope_span, "scope exceeds the granted atomic scope")
                .emit();
        }
    }

    /// 宿主 GPU 编排已知签名核对与定型(MS1.2,RXS-0190):接收者判定已由调用方
    /// guard 完成;实参/返回按编译器已知签名经 [`Self::check_args`] 合一(元数不符
    /// 走既有 RX2003、类型不符走既有 RX2001,不另立新码)。
    fn check_gpu_method(
        &mut self,
        span: Span,
        call_id: HirId,
        op: crate::hir::GpuHostOp,
        adt_args: &[Ty],
        args: &[hir::Expr],
    ) -> Ty {
        use crate::hir::GpuHostOp as Op;
        self.results.gpu_calls.insert(call_id, op);
        let ctx_def = self
            .res
            .lang_items
            .context
            .expect("Context lang item 在 resolve 入口注入");
        // 单 brand 方案(RFC-0009 §9 Q-Brand):Context 自身即 brand 类型。
        let brand = Ty::Adt(ctx_def, Vec::new());
        match op {
            Op::CtxStream => {
                let stream = self
                    .res
                    .lang_items
                    .stream
                    .expect("Stream lang item 在 resolve 入口注入");
                self.check_args(span, &[], args, Ty::Adt(stream, vec![brand]))
            }
            Op::CtxAlloc | Op::CtxAllocPinned => {
                // 元素类型经使用点推断合一(RXS-0190):此处合成 fresh 变量并登记
                // 分配点,body 收尾定型检查(不可定型 / 超出子集 → RX2010)。
                let elem = self.infcx.fresh(None);
                self.gpu_allocs.push((span, elem.clone()));
                let container = if matches!(op, Op::CtxAlloc) {
                    self.res.lang_items.buffer
                } else {
                    self.res.lang_items.pinned_buffer
                }
                .expect("gpu 缓冲 lang item 在 resolve 入口注入");
                self.check_args(
                    span,
                    &[Ty::Prim(PrimTy::Usize)],
                    args,
                    Ty::Adt(container, vec![brand, elem]),
                )
            }
            Op::CtxSync | Op::StreamSync => self.check_args(span, &[], args, Ty::unit()),
            Op::BufUpload | Op::BufDownload => {
                // 形参 = `&PinnedBuffer<C, T>` / `&mut PinnedBuffer<C, T>`(brand 与
                // 元素取接收者实参位,合一即传播推断,RXS-0190)。
                let pinned = self
                    .res
                    .lang_items
                    .pinned_buffer
                    .expect("PinnedBuffer lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand);
                let elem = adt_args
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| self.infcx.fresh(None));
                let expected = Ty::Ref(
                    Box::new(Ty::Adt(pinned, vec![b, elem])),
                    matches!(op, Op::BufDownload),
                );
                self.check_args(span, &[expected], args, Ty::unit())
            }
            Op::BufLen | Op::PinnedLen => self.check_args(span, &[], args, Ty::Prim(PrimTy::Usize)),
            Op::PinnedGet => {
                let elem = adt_args.get(1).cloned().unwrap_or(Ty::Err);
                self.check_args(span, &[Ty::Prim(PrimTy::Usize)], args, elem)
            }
            Op::PinnedSet => {
                let elem = adt_args.get(1).cloned().unwrap_or(Ty::Err);
                self.check_args(span, &[Ty::Prim(PrimTy::Usize), elem], args, Ty::unit())
            }
            // present 宿主 typestate 转移(MS1.2b,RXS-0197):全部 0 实参;
            // 消费/借用语义在 mir_build 侧表达(消费式 = 接收者按值 move)。
            Op::PresentReady | Op::PresentPresent => {
                let ready = self
                    .res
                    .lang_items
                    .present_ready
                    .expect("Ready lang item 在 resolve 入口注入");
                self.check_args(span, &[], args, Ty::Adt(ready, Vec::new()))
            }
            Op::PresentWait => {
                let acquired = self
                    .res
                    .lang_items
                    .present_acquired
                    .expect("Acquired lang item 在 resolve 入口注入");
                self.check_args(span, &[], args, Ty::Adt(acquired, Vec::new()))
            }
            Op::PresentSignal => {
                let presentable = self
                    .res
                    .lang_items
                    .present_presentable
                    .expect("Presentable lang item 在 resolve 入口注入");
                self.check_args(span, &[], args, Ty::Adt(presentable, Vec::new()))
            }
            // backbuffer 借用句柄(RXS-0198):产 `Buffer<C, f32>`(元素定型 f32,
            // 不经 RXS-0190 推断;单 brand = Context)。
            Op::PresentBackbuffer => {
                let buffer = self
                    .res
                    .lang_items
                    .buffer
                    .expect("Buffer lang item 在 resolve 入口注入");
                self.check_args(
                    span,
                    &[],
                    args,
                    Ty::Adt(buffer, vec![brand, Ty::Prim(PrimTy::F32)]),
                )
            }
            Op::PresentPump => self.check_args(span, &[], args, Ty::Prim(PrimTy::Bool)),
            // G3.4 bindless(RXS-0235):`ctx.texture_table() -> TextureTable<C>`
            // (单 brand 方案沿 RFC-0009 §9 Q-Brand;非 Copy affine 沿 RXS-0189)。
            Op::CtxTextureTable => {
                let tt = self
                    .res
                    .lang_items
                    .texture_table
                    .expect("TextureTable lang item 在 resolve 入口注入");
                self.check_args(span, &[], args, Ty::Adt(tt, vec![brand]))
            }
            // `table.register(buf) -> u32`(注册序即索引;首期宿主可注册资源 =
            // `Buffer<C, T>` 句柄——std::gpu 唯一宿主资源面,格式擦除 host↔shader
            // 形态错配 = 运行期确定性 Err,RXS-0235 L3;实参非消费镜像 launch Buffer
            // 实参纪律,RXS-0191)。
            Op::TableRegister => {
                let buffer = self
                    .res
                    .lang_items
                    .buffer
                    .expect("Buffer lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand);
                let elem = self.infcx.fresh(None);
                self.check_args(
                    span,
                    &[Ty::Adt(buffer, vec![b, elem])],
                    args,
                    Ty::Prim(PrimTy::U32),
                )
            }
            // `table.len() -> u32`(已注册计数 = clamp 表长源,RXS-0235)。
            Op::TableLen => self.check_args(span, &[], args, Ty::Prim(PrimTy::U32)),
            // G3.5 render graph(RXS-0236):`g.color_target(w, h)` / `g.depth_target(w, h)`
            // → `GraphResource<C>`(单 brand 方案沿 RFC-0009 §9 Q-Brand;非 Copy affine 沿
            // RXS-0189)。w/h 为 u32 维度(执行器消费,推导本体不用)。
            Op::GraphColorTarget | Op::GraphDepthTarget => {
                let gr = self
                    .res
                    .lang_items
                    .graph_resource
                    .expect("GraphResource lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand);
                let u32t = Ty::Prim(PrimTy::U32);
                self.check_args(span, &[u32t.clone(), u32t], args, Ty::Adt(gr, vec![b]))
            }
            // `g.pass() -> PassBuilder<C>`(声明序 = 提交序;非消费接收者)。
            Op::GraphPass => {
                let pb = self
                    .res
                    .lang_items
                    .pass_builder
                    .expect("PassBuilder lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand);
                self.check_args(span, &[], args, Ty::Adt(pb, vec![b]))
            }
            // `g.readback(t: GraphResource<C>)`(源 CopySrc + 自动 readback 目的 buffer;
            // 非消费接收者与实参)。
            Op::GraphReadback => {
                let gr = self
                    .res
                    .lang_items
                    .graph_resource
                    .expect("GraphResource lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand);
                self.check_args(span, &[Ty::Adt(gr, vec![b])], args, Ty::unit())
            }
            // `g.execute()`(装配核验 + 状态推导;非消费接收者;返回 unit)。
            Op::GraphExecute => self.check_args(span, &[], args, Ty::unit()),
            // `pb.writes_rt(t)` / `writes_depth(t)` / `reads(t)` / `reads_writes_uav(t)`
            // → `PassBuilder<C>`(消费接收者并返回〔builder 链〕;资源实参 t 非消费,镜像
            // launch/register Buffer 实参纪律)。五类访问声明方法(封闭枚举 AccessKind)。
            Op::PassWritesRt | Op::PassWritesDepth | Op::PassReads | Op::PassReadsWritesUav => {
                let pb = self
                    .res
                    .lang_items
                    .pass_builder
                    .expect("PassBuilder lang item 在 resolve 入口注入");
                let gr = self
                    .res
                    .lang_items
                    .graph_resource
                    .expect("GraphResource lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand);
                self.check_args(
                    span,
                    &[Ty::Adt(gr, vec![b.clone()])],
                    args,
                    Ty::Adt(pb, vec![b]),
                )
            }
            // EI1.4 UC-05 RHI(RXS-0257):`rhi.resource(n) -> Res<C, T>`(owned affine 资源句柄;
            // 非消费接收者;n 为 u32 元素数)。**元素类型 `T` 经使用点推断合一**(镜像
            // `ctx.alloc`,RXS-0190):合成 fresh 变量并登记分配点,body 收尾定型检查(不可定型 /
            // 超出首期子集 → RX2010)。`T` 定型后 mir_build 以 `n * sizeof(T)` 下发真设备分配
            // (EI1.4 兑现;EI1.3 期 n 不下发)。brand 取接收者 `Rhi<C>` 的 per-instance 新鲜 brand。
            Op::RhiResource => {
                let res = self
                    .res
                    .lang_items
                    .rhi_res
                    .expect("Res lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand);
                let elem = self.infcx.fresh(None);
                self.gpu_allocs.push((span, elem.clone()));
                self.check_args(
                    span,
                    &[Ty::Prim(PrimTy::U32)],
                    args,
                    Ty::Adt(res, vec![b, elem]),
                )
            }
            // EI1.4 `rhi.pass(kernel, GridDim(..), BlockDim(..), (args..)) -> Pass<C>`(RXS-0257/0261):
            // **pass 绑 kernel**——形态与 `Stream::launch` 逐位同构(kernel 引用 + 维度 + 实参元组),
            // 类型契约由 [`crate::launch_check`] 结构化裁决(着色 RX3004 / 维度 RX3005 / 实参 RX2001
            // / brand RX3006,**零新码全复用**);typeck 侧仅递归定型 + 元素推断增强(kernel `View`
            // 形参元素 ← `Res` 未定元素静默合一)+ 实参登记(收尾 RX6024 子集检查)。
            //
            // **I4 反射喂入源**:实参中类型为 `Res<C, T>` 者 = 该 pass 的 kernel **实际触碰资源集**
            // (reflected 集;由 launch_check 核对其确落在 kernel 的 `View`/`ViewMut` 形参位)→
            // mir_build 以 kind-2 槽下发 `rxrt_rhi_bind` → `PassSpec::with_reflection`(RXS-0257)。
            Op::RhiPass => {
                let pass = self
                    .res
                    .lang_items
                    .rhi_pass
                    .expect("Pass lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand);
                for a in args {
                    let _ = self.check_expr(a);
                }
                if args.len() != 4 {
                    self.err_arg_count(span, 4, args.len());
                }
                self.gpu_launch_infer(args);
                Ty::Adt(pass, vec![b])
            }
            // `pass.reads(&res)` / `pass.writes(&res) -> Pass<C>`(消费接收者并返回〔builder 链〕;
            // 资源实参 `&Res` **借用非消费**——保 `.reads(&a).reads(&a)` 二次借用可编译,RFC-0014
            // §4.B1)。**per-instance brand 核验(I7)**:资源 brand 与 pass brand 不一致(跨 `Rhi`
            // 实例误用)→ **RX3006**(复用 launch brand 裁决 RXS-0074/0189,非 RX2001);非 `&Res`
            // 形态 → RX2001(demand)。
            Op::RhiPassReads | Op::RhiPassWrites => {
                let pass = self
                    .res
                    .lang_items
                    .rhi_pass
                    .expect("Pass lang item 在 resolve 入口注入");
                let res = self
                    .res
                    .lang_items
                    .rhi_res
                    .expect("Res lang item 在 resolve 入口注入");
                let pass_brand = adt_args.first().cloned().unwrap_or(brand);
                if args.len() != 1 {
                    self.err_arg_count(span, 1, args.len());
                }
                for (i, a) in args.iter().enumerate() {
                    let at = self.check_expr(a);
                    if i != 0 {
                        continue;
                    }
                    match rhi_res_brand(&at, res) {
                        // `&Res<C'>`:per-instance brand 相等核验(I7)。
                        Some(res_brand) => {
                            if !brands_compatible(&pass_brand, &res_brand) {
                                self.diag()
                                    .struct_error(E_RHI_CROSS_BRAND, "rhi.cross_brand")
                                    .arg("what", "this RHI resource")
                                    .span_label(a.span, "belongs to a different `Rhi` instance")
                                    .emit();
                            }
                        }
                        // 非 `&Res` 形态:类型失配 RX2001。
                        None => {
                            let expected =
                                Ty::Ref(Box::new(Ty::Adt(res, vec![pass_brand.clone()])), false);
                            self.demand(a.span, &expected, &at);
                        }
                    }
                }
                Ty::Adt(pass, vec![pass_brand])
            }
            // `rhi.submit() -> Queue<C>`(装配核验 I3/I4/I5 + 纯函数状态推导;**消费式接收者**,
            // 1-submit typestate 镜像 RXS-0197 present;二次 submit = RX4001,由 mir_build move-out
            // + move 检查裁决,RXS-0258/0260)。
            Op::RhiSubmit => {
                let queue = self
                    .res
                    .lang_items
                    .rhi_queue
                    .expect("Queue lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand);
                self.check_args(span, &[], args, Ty::Adt(queue, vec![b]))
            }
            // `rhi.readback(res: Res<C, T>, dst: &mut PinnedBuffer<Ctx, T>)`(RXS-0259;EI1.4 兑现
            // 真 D2H)。资源实参**按值消费**(`Res<C, T>` 非引用)——非消费接收者 `rhi`,消费实参
            // `res`;`dst` 为**锁页主机缓冲可变借用**(镜像 `buf.download(&mut pinned)` 的落地面,
            // RXS-0190),元素 `T` 与资源元素合一(不一致 → RX2001 by demand)。返回 unit。
            // 实际 move-out(I1 use-after-free / I2 double-free 的 RX4001 拦截)由 mir_build 对
            // 实参发射 `Operand::Move` + move 检查裁决(镜像 submit 消费式接收者纪律)。
            Op::RhiReadback => {
                let res = self
                    .res
                    .lang_items
                    .rhi_res
                    .expect("Res lang item 在 resolve 入口注入");
                let pinned = self
                    .res
                    .lang_items
                    .pinned_buffer
                    .expect("PinnedBuffer lang item 在 resolve 入口注入");
                let b = adt_args.first().cloned().unwrap_or(brand.clone());
                let elem = self.infcx.fresh(None);
                self.check_args(
                    span,
                    &[
                        Ty::Adt(res, vec![b, elem.clone()]),
                        Ty::Ref(Box::new(Ty::Adt(pinned, vec![brand, elem])), true),
                    ],
                    args,
                    Ty::unit(),
                )
            }
            Op::CtxCreate
            | Op::Launch
            | Op::PresentCreate
            | Op::WritePpm
            | Op::GraphCreate
            | Op::RhiCreate => {
                unreachable!(
                    "CtxCreate/PresentCreate/GraphCreate/RhiCreate/WritePpm 走 check_call;launch 走既有 launch 分支"
                )
            }
        }
    }

    /// launch 元素推断增强 + 实参记录(MS1.2,RXS-0190/0191)。仅形态完整
    /// (kernel 引用 + 4 实参 + 元组)时生效:Buffer 实参**未定**元素与 kernel
    /// `View`/`ViewMut` 形参元素静默合一、未定数值类标量与标量形参静默合一
    /// (合一失败不发诊断——类型契约裁决权在 launch_check,RXS-0074/0075
    /// 零漂移);全部元组实参登记供收尾子集检查(RX6024)。
    fn gpu_launch_infer(&mut self, args: &[hir::Expr]) {
        if args.len() != 4 {
            return;
        }
        let hir::ExprKind::Tuple(elems) = &args[3].kind else {
            return;
        };
        for el in elems {
            if let Some(t) = self.results.expr_ty.get(&el.hir_id) {
                self.gpu_launch_args.push((el.span, t.clone()));
            }
        }
        let hir::ExprKind::Res(Res::Def(k)) = &args[0].kind else {
            return;
        };
        let is_kernel = matches!(&self.krate.item(*k).kind,
            hir::ItemKind::Fn(decl) if decl.color == FnColor::Kernel);
        if !is_kernel {
            return;
        }
        let sig = self.cx.fn_sig(*k);
        // kernel 形参剔除 `ThreadCtx` 句柄形参(RXS-0074 参数契约同口径)。
        let params: Vec<Ty> = sig
            .inputs
            .iter()
            .filter(|t| !matches!(t, Ty::Adt(d, _) if self.res.lang_items.is_thread_ctx(*d)))
            .cloned()
            .collect();
        for (param, el) in params.iter().zip(elems.iter()) {
            let Some(arg_ty) = self.results.expr_ty.get(&el.hir_id).cloned() else {
                continue;
            };
            let shallow = self.infcx.shallow(&arg_ty);
            match (param, &shallow) {
                // `Buffer<C, T>`(launch)/ `Res<C, T>`(EI1.4 `rhi.pass` 绑 kernel,RXS-0257)
                // 均以 `View`/`ViewMut` 形参承载:未定元素静默合一(裁决权仍在 launch_check)。
                (Ty::Adt(pd, pargs), Ty::Adt(ad, aargs))
                    if self.res.lang_items.view_mutable(*pd).is_some()
                        && (self.res.lang_items.is_buffer(*ad)
                            || self.res.lang_items.is_rhi_res(*ad)) =>
                {
                    if let (Some(pe), Some(ae)) = (pargs.get(1), aargs.get(1))
                        && matches!(self.infcx.shallow(ae), Ty::Infer(_))
                    {
                        let _ = self.infcx.unify(ae, pe);
                    }
                }
                (Ty::Prim(_), Ty::Infer(_)) => {
                    let _ = self.infcx.unify(&arg_ty, param);
                }
                _ => {}
            }
        }
    }

    /// 宿主 GPU 收尾裁决(MS1.2;body 全部使用点约束收齐后统一定型):
    /// - RX2010(RXS-0190):缓冲元素不可定型 / 定型超出首期子集 {f32,i32,u32};
    /// - RX6024(RXS-0191):launch 实参超出 Buffer + {i32,u32,f32,usize} 子集。
    ///
    /// Err 容忍不级联(RXS-0075 同口径);诊断 span 指向 alloc 调用点 / 违例实参。
    fn check_gpu_deferred(&mut self) {
        let allocs = std::mem::take(&mut self.gpu_allocs);
        for (span, elem) in allocs {
            let r = self.infcx.resolve(&elem);
            match &r {
                Ty::Prim(PrimTy::F32 | PrimTy::I32 | PrimTy::U32) => {}
                Ty::Err => {} // 容忍区不级联
                Ty::Infer(_) => {
                    self.err_gpu_elem_infer(span, "no use site constrains the element type");
                }
                other => {
                    let rendered = other.render(&self.res);
                    self.err_gpu_elem_infer(span, &format!("element type resolves to {rendered}"));
                }
            }
        }
        let launch_args = std::mem::take(&mut self.gpu_launch_args);
        for (span, ty) in launch_args {
            let r = self.infcx.resolve(&ty);
            match &r {
                Ty::Prim(PrimTy::F32 | PrimTy::I32 | PrimTy::U32 | PrimTy::Usize) => {}
                Ty::Adt(d, _) if self.res.lang_items.is_buffer(*d) => {}
                // EI1.4(RXS-0257):`Res<C, T>` 为 `rhi.pass` 绑 kernel 的资源实参(marshalling
                // kind-2 槽,submit 期换设备指针),与 Buffer 平行落在首期子集内。
                Ty::Adt(d, _) if self.res.lang_items.is_rhi_res(*d) => {}
                Ty::Err | Ty::Infer(_) => {} // 容忍区/未定不级联(定型违例另有归位)
                _ => self.err_gpu_launch_arg(span, &r),
            }
        }
    }

    /// RX2010(RXS-0190):宿主 GPU 缓冲元素不可定型 / 超出首期子集。
    fn err_gpu_elem_infer(&self, span: Span, detail: &str) {
        self.diag()
            .struct_error(E_GPU_ELEM_INFER, "gpu.elem_infer")
            .arg("detail", detail)
            .span_label(span, "cannot type this GPU buffer element")
            .emit();
    }

    /// RX6024(RXS-0191):launch 实参超出首期 marshalling 子集。
    fn err_gpu_launch_arg(&self, span: Span, ty: &Ty) {
        self.diag()
            .struct_error(E_GPU_LAUNCH_ARG_SUBSET, "gpu.launch_arg_subset")
            .arg("ty", ty.render(&self.res))
            .span_label(span, "launch argument outside the first-phase subset")
            .emit();
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
            let slots = self.adt_slots(*d);
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

/// `Scope` 包含序 → 展示名(RXS-0080;RX3010 诊断渲染)。
fn scope_name(rank: u8) -> &'static str {
    match rank {
        0 => "Block",
        1 => "Gpu",
        _ => "System",
    }
}

/// G3.4 bindless(RXS-0232):若 `expr` 为 `nonuniform(inner)` 调用(callee 解析为
/// `nonuniform` lang item + 恰 1 实参),返回内层索引表达式 `inner`(否则 `None`)。
fn nonuniform_inner<'e>(
    expr: &'e hir::Expr,
    lang_items: &crate::resolve::LangItems,
) -> Option<&'e hir::Expr> {
    if let hir::ExprKind::Call { callee, args } = &expr.kind
        && let hir::ExprKind::Res(Res::Def(d)) = &callee.kind
        && lang_items.is_nonuniform(*d)
        && args.len() == 1
    {
        Some(&args[0])
    } else {
        None
    }
}

/// G3.4 bindless(RXS-0232):`expr` 是否为整型字面量常量(nonuniform 标注唯一豁免)。
fn is_int_literal(expr: &hir::Expr) -> bool {
    matches!(&expr.kind, hir::ExprKind::Lit(l) if l.kind == LitKind::Int)
}

/// 采样方法族名 → [`crate::mir::ResourceMethod`](G3.3,RXS-0223;`Texture2D<F>`
/// 接收者的新方法)。`sample` 本名不在此表——走既有 RXS-0174 分支(`sample_calls`
/// → `SampleLod` 空 extra,byte-preserving,Q-S-SampleName)。
fn texture2d_family_method(name: &str) -> Option<crate::mir::ResourceMethod> {
    use crate::mir::ResourceMethod as M;
    Some(match name {
        "sample_lod" => M::SampleLod,
        "sample_grad" => M::SampleGrad,
        "sample_bias" => M::SampleBias,
        "load" => M::Load,
        "load_lod" => M::LoadLod,
        "sample_cmp" => M::SampleCmp,
        "gather" => M::Gather,
        _ => return None,
    })
}

/// 方法名 ∈ 采样方法族(含 `sample`/`store`;RXS-0223)。`TextureRw2D` 接收者
/// 分支守卫:族内方法在 rw 接收者上或合法(load/store)或 RX3014,均归采样类别。
fn is_sample_family_name(name: &str) -> bool {
    matches!(
        name,
        "sample"
            | "sample_lod"
            | "sample_grad"
            | "sample_bias"
            | "load"
            | "load_lod"
            | "sample_cmp"
            | "gather"
            | "store"
    )
}

/// 方法 → 阶段列展示(RXS-0223 阶段 × 合法性矩阵;RX3014 诊断渲染。后续阶段
/// 扩展〔如 mesh/RT 其余阶段〕集中改此处与 `check_sample_family` 的 stage match)。
fn family_stage_note(m: crate::mir::ResourceMethod) -> &'static str {
    use crate::mir::ResourceMethod as M;
    match m {
        M::Sample | M::SampleBias => "`fragment` only",
        M::SampleLod | M::SampleGrad | M::Load | M::LoadLod | M::SampleCmp | M::Gather => {
            "`fragment` + `vertex`"
        }
        M::StorageLoad | M::Store => "`fragment` + `raygen`",
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

    //@ spec: RXS-0067, RXS-0069
    #[test]
    fn addrspace_mismatch_is_rx3002() {
        // device fn 形参要求 constant 空间,kernel 传入 global view → RX3002
        let (codes, _) = check(
            "device fn consume(v: View<constant, f32>) {}\nkernel fn k(g: View<global, f32>) {\n    consume(g);\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![3002]);
    }

    //@ spec: RXS-0067
    #[test]
    fn matching_addrspace_is_clean() {
        check_clean(
            "device fn consume(v: View<global, f32>) {}\nkernel fn k(g: View<global, f32>) {\n    consume(g);\n}\nfn main() {}",
        );
    }

    //@ spec: RXS-0067
    #[test]
    fn addrspace_mismatch_on_let_annotation_is_rx3002() {
        // 同可变性 View,空间不符的 let 标注 → RX3002
        let (codes, _) = check(
            "kernel fn k(g: View<global, f32>) {\n    let _s: View<constant, f32> = g;\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![3002]);
    }

    //@ spec: RXS-0050
    #[test]
    fn question_mark_propagates_and_unwraps() {
        check_clean(
            "fn half(x: i32) -> Result<i32, i32> {\n    if x % 2 == 0 { Ok(x / 2) } else { Err(x) }\n}\nfn quarter(x: i32) -> Result<i32, i32> {\n    let h = half(x)?;\n    let q = half(h)?;\n    Ok(q)\n}",
        );
    }

    //@ spec: RXS-0050
    #[test]
    fn question_mark_requires_result_scrutinee() {
        let (codes, _) =
            check("fn f() -> Result<i32, i32> {\n    let x = 1;\n    let y = x?;\n    Ok(y)\n}");
        assert!(codes.contains(&2001), "{codes:?}");
    }

    //@ spec: RXS-0050
    #[test]
    fn question_mark_requires_result_return_type() {
        let (codes, _) = check(
            "fn half(x: i32) -> Result<i32, i32> {\n    Ok(x)\n}\nfn f(x: i32) -> i32 {\n    let h = half(x)?;\n    h\n}",
        );
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0048
    #[test]
    fn builtin_option_result_are_plain_generic_enums() {
        check_clean(
            "fn f() {\n    let x: Option<i32> = None;\n    let y: Option<i32> = Some(3);\n    let z: Result<bool, i32> = Ok(true);\n    let _p = (x, y, z);\n}",
        );
        let (codes, _) = check("fn f() {\n    let _x: Option<i32> = Some(true);\n}");
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0049
    #[test]
    fn for_over_inherent_iterator_binds_element() {
        let base = "struct C {\n    n: i32,\n}\nimpl C {\n    fn make(n: i32) -> C {\n        C { n }\n    }\n    fn next(&mut self) -> Option<i32> {\n        if self.n > 0 {\n            self.n -= 1;\n            Some(self.n)\n        } else {\n            None\n        }\n    }\n}\n";
        check_clean(&format!(
            "{base}fn f() -> i32 {{\n    let mut acc = 0;\n    for v in C::make(3) {{\n        acc += v;\n    }}\n    acc\n}}"
        ));
        let (codes, _) = check(&format!(
            "{base}fn g() {{\n    for v in C::make(1) {{\n        let _x: bool = v;\n    }}\n}}"
        ));
        assert_eq!(codes, vec![2001]);
    }

    //@ spec: RXS-0049
    #[test]
    fn for_over_non_iterator_is_rx2004() {
        let (codes, _) = check("fn f() {\n    for _x in 5 {\n    }\n}");
        assert_eq!(codes, vec![2004]);
    }

    //@ spec: RXS-0050, RXS-0044
    #[test]
    fn pattern_ctor_must_match_scrutinee_adt() {
        let (codes, _) = check(
            "enum E {\n    A,\n}\nstruct S {\n    v: i32,\n}\nfn f(s: S) -> i32 {\n    match s {\n        E::A => 1,\n    }\n}",
        );
        assert_eq!(codes, vec![2001]);
    }

    // M2.3:内建 println 签名(最小 prelude)
    #[test]
    fn builtin_println_signature() {
        check_clean("fn main() {\n    println(\"hello\");\n}");
        let (codes, _) = check("fn main() {\n    println(1);\n}");
        assert_eq!(codes, vec![2001]);
        let (codes, _) = check("fn main() {\n    println(\"a\", \"b\");\n}");
        assert_eq!(codes, vec![2003]);
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

    // ---- M3.2:Copy 判定 / derive(Copy) / Drop impl(RXS-0053/RXS-0055) ----

    //@ spec: RXS-0053
    #[test]
    fn copy_judgment_matrix() {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(
            "#[derive(Copy)]\nstruct P { x: i32 }\nstruct M { x: i32 }\nfn main() {}",
            SourceId(0),
            Edition::Rx0,
            &diag,
        );
        let krate = cx.hir_crate();
        let res = cx.resolutions();
        let def = |n: &str| DefId(res.defs.iter().position(|d| d.name == n).unwrap() as u32);
        let p = Ty::Adt(def("P"), Vec::new());
        let m = Ty::Adt(def("M"), Vec::new());
        use crate::ty::is_copy;
        // 标量 / 共享引用 / 裸指针 / fn 指针:内建 Copy
        assert!(is_copy(&krate, &Ty::Prim(PrimTy::I32)));
        assert!(is_copy(&krate, &Ty::Prim(PrimTy::Bool)));
        assert!(is_copy(&krate, &Ty::Ref(Box::new(m.clone()), false)));
        assert!(is_copy(&krate, &Ty::RawPtr(Box::new(m.clone()), true)));
        assert!(is_copy(
            &krate,
            &Ty::FnPtr(Vec::new(), Box::new(Ty::unit()))
        ));
        // &mut T 与未标注 ADT:move
        assert!(!is_copy(&krate, &Ty::Ref(Box::new(p.clone()), true)));
        assert!(!is_copy(&krate, &m));
        // derive(Copy) ADT:Copy
        assert!(is_copy(&krate, &p));
        // 元组/数组:逐组件
        assert!(is_copy(
            &krate,
            &Ty::Tuple(vec![Ty::Prim(PrimTy::I32), p.clone()])
        ));
        assert!(!is_copy(
            &krate,
            &Ty::Tuple(vec![Ty::Prim(PrimTy::I32), m.clone()])
        ));
        assert!(is_copy(&krate, &Ty::Array(Box::new(p))));
        assert!(!is_copy(&krate, &Ty::Array(Box::new(m))));
        // Err 容忍为 Copy(不级联 move 诊断)
        assert!(is_copy(&krate, &Ty::Err));
    }

    //@ spec: RXS-0055
    #[test]
    fn needs_drop_is_transitive() {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(
            "struct R { x: i32 }\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nstruct W { r: R }\nenum E { A, B(R) }\nstruct C { x: i32 }\nfn main() {}",
            SourceId(0),
            Edition::Rx0,
            &diag,
        );
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "{:?}", diag.emitted());
        let krate = cx.hir_crate();
        let res = cx.resolutions();
        let adt = |n: &str| {
            Ty::Adt(
                DefId(res.defs.iter().position(|d| d.name == n).unwrap() as u32),
                Vec::new(),
            )
        };
        use crate::ty::needs_drop;
        assert!(needs_drop(&krate, &adt("R")), "自身携带 Drop impl");
        assert!(needs_drop(&krate, &adt("W")), "字段传递");
        assert!(needs_drop(&krate, &adt("E")), "变体载荷传递");
        assert!(!needs_drop(&krate, &adt("C")));
        assert!(!needs_drop(&krate, &Ty::Prim(PrimTy::I32)));
        assert!(
            !needs_drop(&krate, &Ty::Ref(Box::new(adt("R")), true)),
            "引用不拥有"
        );
        assert!(needs_drop(&krate, &Ty::Tuple(vec![adt("R")])));
        assert!(needs_drop(&krate, &Ty::Array(Box::new(adt("R")))));
    }

    //@ spec: RXS-0053
    #[test]
    fn derive_copy_requires_all_fields_copy() {
        let (codes, _) =
            check("struct M { x: i32 }\n#[derive(Copy)]\nstruct B { m: M }\nfn main() {}");
        assert_eq!(codes, vec![2008]);
        check_clean(
            "#[derive(Copy)]\nstruct P { x: i32, y: bool }\n#[derive(Copy)]\nstruct Q { p: P, t: (i32, char) }\nfn main() {}",
        );
    }

    //@ spec: RXS-0053
    #[test]
    fn derive_copy_rejects_generic_fields_conservatively() {
        let (codes, _) = check("#[derive(Copy)]\nstruct G<T> { v: T }\nfn main() {}");
        assert_eq!(codes, vec![2008]);
    }

    //@ spec: RXS-0053
    #[test]
    fn derive_copy_conflicts_with_drop_impl() {
        let (codes, _) = check(
            "#[derive(Copy)]\nstruct R { x: i32 }\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![2008]);
    }

    //@ spec: RXS-0055
    #[test]
    fn drop_impl_shape_violations() {
        // 接收者非 &mut self
        let (codes, _) =
            check("struct R { x: i32 }\nimpl Drop for R {\n    fn drop(&self) {}\n}\nfn main() {}");
        assert_eq!(codes, vec![2009]);
        // 多余参数
        let (codes, _) = check(
            "struct R { x: i32 }\nimpl Drop for R {\n    fn drop(&mut self, n: i32) {}\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![2009]);
        // impl 体多余项
        let (codes, _) = check(
            "struct R { x: i32 }\nimpl Drop for R {\n    fn drop(&mut self) {}\n    fn extra(&self) {}\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![2009]);
        // 目标非本包 struct/enum
        let (codes, _) = check("impl Drop for i32 {\n    fn drop(&mut self) {}\n}\nfn main() {}");
        assert_eq!(codes, vec![2009]);
        // 合法形状
        check_clean(
            "struct R { x: i32 }\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nfn main() {}",
        );
    }

    //@ spec: RXS-0055
    #[test]
    fn duplicate_drop_impl_rejected() {
        let (codes, _) = check(
            "struct R { x: i32 }\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nfn main() {}",
        );
        // resolve 报关联项重名(RX1002)+ 定义处检查报重复 impl(RX2009)
        assert!(codes.contains(&2009), "{codes:?}");
    }

    //@ spec: RXS-0055
    #[test]
    fn drop_fn_not_explicitly_callable() {
        let (codes, _) = check(
            "struct R { x: i32 }\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nfn main() {\n    let mut r = R { x: 1 };\n    r.drop();\n}",
        );
        assert_eq!(codes, vec![2004]);
    }

    //@ spec: RXS-0055
    #[test]
    fn user_shadowed_drop_trait_not_recognized() {
        // 用户遮蔽 Drop:impl 绑定到用户 trait,不入识别面(形状不校验)
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(
            "trait Drop {\n    fn drop(&mut self);\n}\nstruct R { x: i32 }\nimpl Drop for R {\n    fn drop(&mut self) {}\n}\nfn main() {}",
            SourceId(0),
            Edition::Rx0,
            &diag,
        );
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "{:?}", diag.emitted());
        assert!(cx.hir_crate().drop_impls.is_empty());
    }

    // -- scoped atomics scope 类型契约(M5.2,RXS-0080;RX3010) -----------------

    //@ spec: RXS-0080
    #[test]
    fn scoped_atomics_legal_scope_is_clean() {
        // global AtomicView + Scope::Block(窄于地址空间可见性)+ Atomic brand 同 scope。
        check_clean(
            "kernel fn k(t: ThreadCtx<1>, g: AtomicView<global, u32, (16,)>, a: Atomic<u32, Scope::Gpu>) {\n    let i = t.thread_index();\n    g.fetch_add(i, 1, Scope::Block);\n    a.fetch_max(1, Scope::Block);\n}\nfn main() {}",
        );
    }

    //@ spec: RXS-0080
    #[test]
    fn shared_atomic_wide_scope_is_rx3010() {
        // AtomicView<shared, ..>(block 本地)用 Scope::System → 与地址空间不相容。
        let (codes, _) = check(
            "kernel fn k(t: ThreadCtx<1>, c: AtomicView<shared, u32, (16,)>) {\n    let i = t.thread_index();\n    c.fetch_add(i, 1, Scope::System);\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![3010]);
    }

    //@ spec: RXS-0080
    #[test]
    fn atomic_scope_overreach_is_rx3010() {
        // Atomic<u32, Scope::Block> brand 仅授予 block;操作用 Scope::Gpu → 越权。
        let (codes, _) = check(
            "kernel fn k(a: Atomic<u32, Scope::Block>) {\n    a.fetch_add(1, Scope::Gpu);\n}\nfn main() {}",
        );
        assert_eq!(codes, vec![3010]);
    }

    //@ spec: RXS-0080
    #[test]
    fn atomic_narrower_scope_than_brand_is_clean() {
        // brand = System,操作用更窄的 Scope::Block → 未越权,0 诊断。
        check_clean(
            "kernel fn k(a: Atomic<u32, Scope::System>) {\n    a.fetch_add(1, Scope::Block);\n}\nfn main() {}",
        );
    }
}
