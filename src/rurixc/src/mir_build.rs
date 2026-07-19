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
    //
    // 着色阶段根收集口径(RXS-0161,R1.3 / R1.2 / R6.7):
    // - 默认(非 `dxil-backend`)构建:仅收非着色阶段 kernel 根(`stage == None`),
    //   图形/RT 着色阶段一律不收 —— PTX 后端行为与既有测试零漂移。
    // - cargo feature `dxil-backend` 启用:额外收 vertex / fragment 图形阶段根
    //   (B 路 DXIL codegen 入口),并携 AST 层 I/O 意图签名进 MIR。mesh/task/RT
    //   不在此收集(deferred,任务 15 stub);compute 阶段沿用排除(走既有 A 路)。
    // 收集判定见 [`collectable_stage`](feature 门控,零漂移)。
    for item in &krate.items {
        if let hir::ItemKind::Fn(decl) = &item.kind
            && decl.color == crate::ast::FnColor::Kernel
            && collectable_stage(decl.stage)
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
        // 图形阶段根:携 stage 类别 + AST I/O 意图签名进 MIR(仅 `dxil-backend`;
        // 默认构建为 no-op,`stage`/`io_sig` 维持 build_body 的 None/空,零漂移)。
        attach_graphics_io_sig(cx, &krate, def, &mut body);
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

/// device codegen 收集根判定(RXS-0161,R1.2/R1.3/R6.7):默认仅非着色阶段
/// kernel 根(`stage == None`),vertex/fragment 图形阶段根仅在 `dxil-backend`
/// feature 下额外收纳(B 路 DXIL 入口)。mesh/task/RT 与 compute 不在此收。
#[cfg(any(feature = "dxil-backend", feature = "vulkan-backend"))]
fn collectable_stage(stage: Option<crate::ast::ShaderStage>) -> bool {
    use crate::ast::ShaderStage;
    matches!(
        stage,
        None | Some(ShaderStage::Vertex | ShaderStage::Fragment)
    )
}

/// device codegen 收集根判定(默认 / 非 `dxil-backend`):仅非着色阶段 kernel
/// 根。图形/RT 着色阶段一律排除 —— PTX 路径行为与既有测试逐一致(零漂移)。
#[cfg(not(any(feature = "dxil-backend", feature = "vulkan-backend")))]
fn collectable_stage(stage: Option<crate::ast::ShaderStage>) -> bool {
    stage.is_none()
}

/// 为 vertex / fragment 图形阶段根携 stage 类别 + AST I/O 意图签名(RXS-0161,
/// R1.3):`dxil-backend` 下从 AST `shader_stages` 形参/返回位置的 I/O 结构体
/// 字段标注提取 [`crate::mir::IoSigElem`] 表,置入 `body`。非图形阶段(含全部
/// device fn callee)为 no-op。
#[cfg(any(feature = "dxil-backend", feature = "vulkan-backend"))]
fn attach_graphics_io_sig(cx: &QueryCtx<'_>, krate: &hir::Crate, def: DefId, body: &mut Body) {
    use crate::ast::ShaderStage;
    let hir::ItemKind::Fn(decl) = &krate.item(def).kind else {
        return;
    };
    let Some(stage @ (ShaderStage::Vertex | ShaderStage::Fragment)) = decl.stage else {
        return;
    };
    body.stage = Some(stage);
    body.io_sig = dxil_io::io_sig_for(cx.ast(), &krate.item(def).name, stage);
    // PR-E2b 生产接线(RXS-0163):同序提取资源句柄形参绑定声明,作 host 侧
    // 绑定布局推导(binding_layout)的确定性输入(io_sig 与 resources 互不交叠:
    // 命名 I/O 结构体 → io_sig;资源句柄形参 → resources)。
    body.resources = dxil_io::resources_for(cx.ast(), &krate.item(def).name, stage);
}

/// 默认 / 非 `dxil-backend`:图形阶段根不收集,`Body` 的 stage/io_sig 维持
/// build_body 的中立默认(`None`/空),保证 PTX 路径零漂移(R1.2/R6.7)。
#[cfg(not(any(feature = "dxil-backend", feature = "vulkan-backend")))]
fn attach_graphics_io_sig(_cx: &QueryCtx<'_>, _krate: &hir::Crate, _def: DefId, _body: &mut Body) {}

/// AST → MIR 图形阶段 I/O 意图签名提取(RXS-0161,仅 `dxil-backend`)。
///
/// HIR `FieldDef` 不携 `#[builtin(..)]`/`#[interpolate(..)]` 属性(那是 AST 面),
/// 故 I/O 签名意图须自 AST 提取。本模块**只读** AST(`cx.ast()`),按图形阶段
/// 函数的形参(`In`)/返回(`Out`)位置可达的 I/O 结构体字段,逐字段携带源码
/// 字段名 / builtin·interpolate·varying 种类 / 已建模类型 / 方向四维度。
///
/// 类型映射(R1.9 边界):标量 prim → [`MirIoType::Scalar`]、向量约定名 →
/// [`MirIoType::Vector`];超出已建模子集的类型**不在此静默丢弃**——元素仍
/// 进 io_sig(字段名/种类/方向保真),不可映射的 6xxx 拒绝由 B 路编码器
/// (任务 2/4)裁决。资源句柄(`Texture2D`/`Sampler`)非命名 I/O 结构体,
/// 自然不入 io_sig(opaque handle 形态,RFC-0004 §4.6(b))。
#[cfg(any(feature = "dxil-backend", feature = "vulkan-backend"))]
mod dxil_io {
    use std::collections::HashMap;

    use crate::ast::{self, MetaInner, MetaKind, ShaderStage, TyKind};
    use crate::hir::PrimTy;
    use crate::mir::{
        IoDir, IoSigElem, IoSigKind, MirIoType, MirResourceType, ResourceBinding, ResourceCount,
    };

    /// 提取指定图形阶段函数(名 + 阶段匹配)的 I/O 意图签名表。
    pub(super) fn io_sig_for(
        file: &ast::SourceFile,
        fn_name: &str,
        stage: ShaderStage,
    ) -> Vec<IoSigElem> {
        let mut structs: HashMap<String, &[ast::FieldDef]> = HashMap::new();
        collect_named_structs(&file.items, &mut structs);

        let mut out = Vec::new();
        let Some(f) = find_stage_fn(&file.items, fn_name, stage) else {
            return out;
        };
        // 形参 → In 方向(资源句柄等非命名 I/O 结构体自然跳过)。
        for p in &f.params {
            if let ast::ParamKind::Typed { ty, .. } = &p.kind
                && let Some(fields) = io_struct_fields(ty, &structs)
            {
                for fld in fields {
                    out.push(field_to_elem(fld, IoDir::In));
                }
            }
        }
        // 返回类型 → Out 方向。
        if let Some(ret) = &f.ret
            && let Some(fields) = io_struct_fields(ret, &structs)
        {
            for fld in fields {
                out.push(field_to_elem(fld, IoDir::Out));
            }
        }
        out
    }

    /// 提取指定图形阶段函数的资源句柄形参绑定声明(RXS-0163;PR-E2b 生产接线)。
    ///
    /// 按**声明序**扫描阶段函数形参,命中资源句柄类型(RXS-0156 首批:`Texture2D<F>`
    /// → SRV / `Sampler` → Sampler)者落 [`ResourceBinding`](源码形参名保名 + 资源
    /// 类型 + 基数)。命名 I/O 结构体形参(varying)与原生类型形参不入(由
    /// [`io_sig_for`] 各管其责)。首批无数组语法 → 基数恒 [`ResourceCount::One`]。
    pub(super) fn resources_for(
        file: &ast::SourceFile,
        fn_name: &str,
        stage: ShaderStage,
    ) -> Vec<ResourceBinding> {
        let mut out = Vec::new();
        let Some(f) = find_stage_fn(&file.items, fn_name, stage) else {
            return out;
        };
        for p in &f.params {
            if let ast::ParamKind::Typed { pat, ty } = &p.kind
                && let Some((res, count)) = ast_ty_to_resource(ty)
            {
                out.push(ResourceBinding {
                    name: pat_binding_name(pat).unwrap_or_default(),
                    res,
                    count,
                });
            }
        }
        out
    }

    /// 简单绑定形参名(`name: Ty` → "name");非简单绑定模式 → None。
    fn pat_binding_name(pat: &ast::Pat) -> Option<String> {
        match &pat.kind {
            ast::PatKind::Binding { name, .. } => Some(name.name.clone()),
            _ => None,
        }
    }

    /// AST 类型 → 资源句柄建模 + 绑定基数(RXS-0156 `Texture2D<F>`/`Sampler`;
    /// RXS-0223 扩 `TextureRw2D<F>`/`SamplerCmp`;G3.4 RXS-0231 扩无界句柄数组
    /// `[Texture2D<F>]` → [`ResourceCount::Unbounded`]);非资源句柄类型 → None。
    fn ast_ty_to_resource(ty: &ast::Ty) -> Option<(MirResourceType, ResourceCount)> {
        let ty = unwrap_ty(ty);
        // G3.4 无界句柄数组 `[Texture2D<F>]`(RXS-0231;切片样式文法,无新 token)→
        // 无界基数(binding 推导 RXS-0233 自 Unmappable 翻转;首期无界仅 SRV 纹理,
        // 非-SRV-纹理无界维持 RX6013,binding_layout 兜底)。
        if let TyKind::Slice(inner) = &ty.kind {
            let res = scalar_resource(unwrap_ty(inner))?;
            return Some((res, ResourceCount::Unbounded));
        }
        Some((scalar_resource(ty)?, ResourceCount::One))
    }

    /// 标量(单)资源句柄类型建模。`Texture2D`/`TextureRw2D` 取首个类型实参的头
    /// prim 作分量类型(缺省 `f32`);非资源句柄类型 → None。
    fn scalar_resource(ty: &ast::Ty) -> Option<MirResourceType> {
        let head = ty_head_name(ty)?;
        let elem_prim = || {
            if let TyKind::Path(p) = &ty.kind {
                p.segments
                    .last()
                    .and_then(vector_elem_prim)
                    .unwrap_or(PrimTy::F32)
            } else {
                PrimTy::F32
            }
        };
        match head {
            "Texture2D" => Some(MirResourceType::Texture2D(elem_prim())),
            "Sampler" => Some(MirResourceType::Sampler),
            // RXS-0223:storage image(UAV 轴)+ 比较采样器(Sampler 轴)。
            "TextureRw2D" => Some(MirResourceType::TextureRw2D(elem_prim())),
            "SamplerCmp" => Some(MirResourceType::SamplerCmp),
            _ => None,
        }
    }

    /// 收集全 crate(含嵌套 mod)命名字段结构体 → 字段切片(按名;同名取首个)。
    fn collect_named_structs<'a>(
        items: &'a [ast::Item],
        out: &mut HashMap<String, &'a [ast::FieldDef]>,
    ) {
        for it in items {
            match &it.kind {
                ast::ItemKind::Struct(s) => {
                    if let ast::VariantBody::Named(fields) = &s.body {
                        out.entry(s.name.name.clone()).or_insert(fields.as_slice());
                    }
                }
                ast::ItemKind::Mod(m) => collect_named_structs(&m.items, out),
                _ => {}
            }
        }
    }

    /// 按名 + 阶段查找图形阶段函数(含嵌套 mod)。
    fn find_stage_fn<'a>(
        items: &'a [ast::Item],
        name: &str,
        stage: ShaderStage,
    ) -> Option<&'a ast::FnItem> {
        for it in items {
            match &it.kind {
                ast::ItemKind::Fn(f) if f.stage == Some(stage) && f.name.name == name => {
                    return Some(f);
                }
                ast::ItemKind::Mod(m) => {
                    if let Some(found) = find_stage_fn(&m.items, name, stage) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// 类型若命中命名结构体(I/O varying 结构体)→ 其字段切片;否则 None
    /// (资源句柄 / 原生类型等非 I/O 结构体)。
    fn io_struct_fields<'a>(
        ty: &ast::Ty,
        structs: &HashMap<String, &'a [ast::FieldDef]>,
    ) -> Option<&'a [ast::FieldDef]> {
        let head = ty_head_name(ty)?;
        structs.get(head).copied()
    }

    /// 单个 AST I/O 字段 → MIR 意图签名元素(四维度保真)。
    fn field_to_elem(f: &ast::FieldDef, dir: IoDir) -> IoSigElem {
        IoSigElem {
            field_name: f.name.name.clone(),
            kind: field_anno_kind(f),
            ty: ast_ty_to_mir_io(&f.ty),
            dir,
        }
    }

    /// 字段标注 → I/O 种类(首个 `#[builtin(..)]`/`#[interpolate(..)]`;无标注
    /// 落 [`IoSigKind::Varying`])。与 [`crate::shader_stages`] 的 `field_anno`
    /// 同口径(builtin/interpolate 取列表首个 meta 名)。
    fn field_anno_kind(f: &ast::FieldDef) -> IoSigKind {
        for attr in &f.attrs {
            let [seg] = attr.meta.path.segments.as_slice() else {
                continue;
            };
            let key = seg.ident.name.as_str();
            if key != "builtin" && key != "interpolate" {
                continue;
            }
            let arg = match &attr.meta.kind {
                MetaKind::List(inner) => inner.iter().find_map(|mi| match mi {
                    MetaInner::Meta(m) => m.path.segments.last().map(|s| s.ident.name.clone()),
                    MetaInner::Lit(_) => None,
                }),
                _ => None,
            }
            .unwrap_or_default();
            return if key == "builtin" {
                IoSigKind::Builtin(arg)
            } else {
                IoSigKind::Interpolate(arg)
            };
        }
        IoSigKind::Varying
    }

    /// AST 类型 → 已建模 MIR I/O 类型(标量 / 向量)。超出子集的类型不在此
    /// 静默丢弃(元素仍携),保守落 [`MirIoType::Scalar`] 占位 —— 真正的不可
    /// 映射 6xxx 拒绝由 B 路编码器(任务 2/4)裁决(strict-only,R1.9)。
    fn ast_ty_to_mir_io(ty: &ast::Ty) -> MirIoType {
        let ty = unwrap_ty(ty);
        if let TyKind::Path(p) = &ty.kind
            && let Some(seg) = p.segments.last()
        {
            let name = seg.ident.name.as_str();
            if let Some(prim) = PrimTy::from_name(name) {
                return MirIoType::Scalar(prim);
            }
            if let Some(n) = vector_arity(name) {
                let elem = vector_elem_prim(seg).unwrap_or(PrimTy::F32);
                return MirIoType::Vector(elem, n);
            }
        }
        // 不可映射类型占位:意图侧字段名/种类/方向已保真,类型由编码器复核。
        MirIoType::Scalar(PrimTy::F32)
    }

    /// 向量约定名 → 分量数(`vec2/vec3/vec4`,2..=4;非向量名返回 None)。
    fn vector_arity(name: &str) -> Option<u8> {
        match name {
            "vec2" => Some(2),
            "vec3" => Some(3),
            "vec4" => Some(4),
            _ => None,
        }
    }

    /// 向量分量 prim(末段 `<elem>` 首个类型实参的头 prim;缺省 None)。
    fn vector_elem_prim(seg: &ast::PathSegment) -> Option<PrimTy> {
        let args = seg.args.as_ref()?;
        for a in &args.args {
            if let ast::GenericArg::Type(t) = a {
                return ty_head_name(t).and_then(PrimTy::from_name);
            }
        }
        None
    }

    /// 类型头名(`Texture2D<f32>` → "Texture2D";`&T`/`*T`/`(T)` 取内层头;
    /// 非路径类型 → None)。
    fn ty_head_name(ty: &ast::Ty) -> Option<&str> {
        match &ty.kind {
            TyKind::Path(p) => p.segments.last().map(|s| s.ident.name.as_str()),
            TyKind::Paren(inner) | TyKind::Ref { inner, .. } | TyKind::RawPtr { inner, .. } => {
                ty_head_name(inner)
            }
            _ => None,
        }
    }

    /// 剥 `&T`/`*T`/`(T)` 外层,取内层类型(用于类型映射)。
    fn unwrap_ty(ty: &ast::Ty) -> &ast::Ty {
        match &ty.kind {
            TyKind::Paren(inner) | TyKind::Ref { inner, .. } | TyKind::RawPtr { inner, .. } => {
                unwrap_ty(inner)
            }
            _ => ty,
        }
    }
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
            // G2.2 图形=B(RXS-0161):本构建路径为默认(host/compute/kernel,
            // 含 PTX device 收集)——恒不携着色阶段意图(`stage = None`、`io_sig`
            // 空),保证默认路径 MIR 构造与既有测试零漂移(R1.2/R6.7)。图形阶段
            // 根收集与 I/O 签名携带由后续分片在 `dxil-backend` feature 下接线。
            stage: None,
            io_sig: Vec::new(),
            // PR-E2b 生产接线(RXS-0163):资源句柄绑定声明亦由图形阶段根收集
            // (`attach_graphics_io_sig`)在 `dxil-backend` 下携带;默认路径恒空,
            // 行为零漂移(R1.2/R6.7)。
            resources: Vec::new(),
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
        // 多文件感知(RXS-0196):out-of-line 模块文件的字面量按 span.file 归属
        // 其自身源文本切片;越界退化为空串(经 parse_int 失败走 unsupported)。
        self.cx.snippet(span).unwrap_or("")
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
            // 宿主 GPU 编排(MS1.2,RXS-0191~0193):rxrt_* 字面符号直降 + 失败
            // 终止检查;launch 走 🔒 slot+kinds marshalling。
            tbir::ExprKind::GpuCall { op, args } => self.lower_gpu_call(e, *op, args),
            tbir::ExprKind::GpuLaunch {
                stream,
                kernel,
                grid,
                block,
                args,
            } => self.lower_gpu_launch(e, stream, *kernel, grid, block, args),
            tbir::ExprKind::ResourceSample {
                texture,
                sampler,
                coord,
            } => {
                // 纹理采样(G2.4,RXS-0175;RFC-0007):receiver/sampler 须为资源句柄
                // 形参的裸 local 引用(句柄非值,无投影);coord 为 vec2<f32> 值。
                let ty = self.ty_of(e);
                let Some(tex_p) = self.place_of(texture) else {
                    return self
                        .unsupported(texture.span, "texture sample receiver must be a handle");
                };
                let Some(samp_p) = self.place_of(sampler) else {
                    return self.unsupported(sampler.span, "sampler argument must be a handle");
                };
                if !tex_p.proj.is_empty() || !samp_p.proj.is_empty() {
                    return self.unsupported(
                        e.span,
                        "texture/sampler must be direct resource handle parameter references \
                         (RXS-0174)",
                    );
                }
                let coord_op = self.op_of(coord);
                // RXS-0223 语义升级(Q-S-SampleName):既有 `.sample()` = 显式 LOD 0,
                // 现由 `sample_lod(s, uv, 0.0)` 同一 lowering 路径逐字节承接 →
                // `ResourceMethod::SampleLod` 空 extra(codegen 默认 LOD 0),既有
                // uc04 golden(`dx.op.sampleLevel`)0-byte。新方法族(隐式 sample /
                // grad / bias / load / cmp / gather / store)的前端 typeck→tbir 接线为
                // 后续里程碑,codegen 方法族已就位(RXS-0226)。
                self.rvalue_to_op(
                    Rvalue::ResourceSample {
                        texture_local: tex_p.local,
                        sampler_local: Some(samp_p.local),
                        table_index: None,
                        method: crate::mir::ResourceMethod::SampleLod,
                        coord: coord_op,
                        extra: Vec::new(),
                    },
                    ty,
                    e.span,
                )
            }
            tbir::ExprKind::ResourceMethodCall {
                method,
                texture,
                sampler,
                table_index,
                coord,
                extra,
            } => {
                // 采样方法族(G3.3,RXS-0223/0226):receiver/sampler 须为资源句柄
                // 形参的裸 local 引用(句柄非值,无投影,承 RXS-0175 L4);coord /
                // extra 为值 operand,按 [`crate::mir::ResourceMethod`] 形态携带,
                // codegen 方法族分发消费(dxil_spirv `lower_resource_op`)。
                // G3.4 bindless(RXS-0232/0234):`table_index = Some` 时 `texture` 为
                // `[Texture2D<F>]` 无界表形参裸 local,`table_index` 为动态索引值 operand
                // (codegen `OpAccessChain` runtime array + `NonUniform` + clamp);句柄
                // **不物化中间 local**(RXS-0175 内联形态)。
                let ty = self.ty_of(e);
                let Some(tex_p) = self.place_of(texture) else {
                    return self
                        .unsupported(texture.span, "texture method receiver must be a handle");
                };
                if !tex_p.proj.is_empty() {
                    return self.unsupported(
                        e.span,
                        "texture/sampler must be direct resource handle parameter references \
                         (RXS-0223)",
                    );
                }
                // G3.4:动态索引值物化为 operand(无界表元素采样);单句柄 = None。
                let table_index_op = table_index.as_ref().map(|idx| self.op_of(idx));
                let sampler_local = match sampler {
                    Some(s) => {
                        let Some(samp_p) = self.place_of(s) else {
                            return self.unsupported(s.span, "sampler argument must be a handle");
                        };
                        if !samp_p.proj.is_empty() {
                            return self.unsupported(
                                e.span,
                                "texture/sampler must be direct resource handle parameter \
                                 references (RXS-0223)",
                            );
                        }
                        Some(samp_p.local)
                    }
                    None => None,
                };
                let coord_op = self.op_of(coord);
                let extra_ops: Vec<Operand> = extra.iter().map(|x| self.op_of(x)).collect();
                self.rvalue_to_op(
                    Rvalue::ResourceSample {
                        texture_local: tex_p.local,
                        sampler_local,
                        table_index: table_index_op,
                        method: *method,
                        coord: coord_op,
                        extra: extra_ops,
                    },
                    ty,
                    e.span,
                )
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
            // extern "C" 无 body fn 符号保名(RXS-0195):以字面名参与 codegen/
            // 链接,不走 mangle()——字面名与 `rx_` 前缀单态符号天然不撞;`main` 等
            // CRT 保留名冲突交由链接器报错(不新增诊断,RFC-0009 §4.2)。
            //@ spec: RXS-0195
            let symbol = if has_body {
                mangle(&item.name, def, &gargs)
            } else {
                item.name.clone()
            };
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

    // -- 宿主 GPU 编排 lowering(MS1.2,RXS-0191/0192/0193)------------------------

    /// gpu 句柄表达式按**读取**消费(RXS-0189/0191:方法接收者与实参语义 = 调用期
    /// 短借用,不 move——句柄后续仍可用;move 后再用由既有 move_check 对 Copy 读
    /// 裁决 RX4001)。
    fn gpu_handle_op(&mut self, e: &tbir::Expr) -> Operand {
        let p = self.place_of_or_temp(e);
        Operand::Copy(p)
    }

    /// present 消费式转移的接收者按值 move 物化(MS1.2b,RXS-0197):receiver
    /// move 进转移结果 temp(句柄同为单 u64 标量,再定名零开销)——错序重用由
    /// 既有 move 检查裁决(RX4001;经引用消费 → RX4003,RXS-0054),编译期拦截。
    fn gpu_consume_receiver(&mut self, e: &tbir::Expr, ret: &Ty, span: Span) -> LocalIdx {
        let p = self.place_of_or_temp(e);
        let carried = self.temp(ret.clone(), span);
        self.assign(Place::local(carried), Rvalue::Use(Operand::Move(p)), span);
        carried
    }

    /// rxrt_* 调用终结子(RXS-0191/0194:`CallTarget::Rt` 字面符号,不走
    /// `mangle()`);dest 新建 temp 并返回。
    fn emit_rt_call(&mut self, symbol: &str, args: Vec<Operand>, ret: Ty, span: Span) -> LocalIdx {
        let dest = self.temp(ret, span);
        let next = self.new_block();
        self.terminate(
            TerminatorKind::Call {
                target: CallTarget::Rt {
                    symbol: symbol.to_owned(),
                },
                args,
                dest: Place::local(dest),
                next,
            },
            span,
        );
        self.switch_to(next);
        dest
    }

    /// 运行期失败终止检查(RXS-0193):`cond` 为真 → 可选诊断行(println)后
    /// `rxrt_trap()`(确定性诊断已由 cabi 落 stderr,trap 直接 abort);否则续行。
    fn emit_gpu_guard(&mut self, cond: LocalIdx, msg: Option<&str>, span: Span) {
        let trap = self.new_block();
        let cont = self.new_block();
        self.terminate(
            TerminatorKind::SwitchBool {
                discr: Operand::Copy(Place::local(cond)),
                then: trap,
                else_: cont,
            },
            span,
        );
        self.switch_to(trap);
        if let Some(m) = msg {
            let pdest = self.temp(Ty::unit(), span);
            let pnext = self.new_block();
            self.terminate(
                TerminatorKind::Call {
                    target: CallTarget::Builtin(crate::hir::Builtin::Println),
                    args: vec![Operand::Const(Const::Str(m.to_owned()))],
                    dest: Place::local(pdest),
                    next: pnext,
                },
                span,
            );
            self.switch_to(pnext);
        }
        let dest = self.temp(Ty::unit(), span);
        let dead = self.new_block();
        self.terminate(
            TerminatorKind::Call {
                target: CallTarget::Rt {
                    symbol: "rxrt_trap".to_owned(),
                },
                args: Vec::new(),
                dest: Place::local(dest),
                next: dead,
            },
            span,
        );
        self.switch_to(dead);
        self.terminate(TerminatorKind::Unreachable, span);
        self.switch_to(cont);
    }

    /// 句柄返回值 == 0 → 终止(RXS-0193/0194:句柄 `0` = cabi 失败值)。
    fn guard_handle_zero(&mut self, h: LocalIdx, span: Span) {
        let c = self.temp(Ty::Prim(PrimTy::Bool), span);
        self.assign(
            Place::local(c),
            Rvalue::BinaryOp(
                BinOp::Eq,
                Operand::Copy(Place::local(h)),
                Operand::Const(Const::Int(0, PrimTy::U64)),
            ),
            span,
        );
        self.emit_gpu_guard(c, None, span);
    }

    /// i32 返回值 < 0 → 终止(RXS-0193:负值 = cabi 失败诊断已落 stderr)。
    fn guard_rc_negative(&mut self, rc: LocalIdx, span: Span) {
        let c = self.temp(Ty::Prim(PrimTy::Bool), span);
        self.assign(
            Place::local(c),
            Rvalue::BinaryOp(
                BinOp::Lt,
                Operand::Copy(Place::local(rc)),
                Operand::Const(Const::Int(0, PrimTy::I32)),
            ),
            span,
        );
        self.emit_gpu_guard(c, None, span);
    }

    /// gpu 缓冲容器元素字节数(RXS-0190 首期子集 {f32,i32,u32} 恒 4;此点元素已
    /// 定型——RX2010 未过则编译早停,防御性兜底 4)。
    fn gpu_elem_size(&self, container: &Ty) -> u64 {
        match container {
            Ty::Adt(_, args) => match args.get(1) {
                Some(Ty::Prim(p)) => u64::from(crate::codegen::prim_width(*p) / 8),
                _ => 4,
            },
            _ => 4,
        }
    }

    /// `n(元素数) * sizeof(T)` 字节数物化(u64 temp)。
    fn gpu_bytes_of(&mut self, n: Operand, elem_size: u64, span: Span) -> LocalIdx {
        let bytes = self.temp(Ty::Prim(PrimTy::U64), span);
        self.assign(
            Place::local(bytes),
            Rvalue::BinaryOp(
                BinOp::Mul,
                n,
                Operand::Const(Const::Int(elem_size as i128, PrimTy::U64)),
            ),
            span,
        );
        bytes
    }

    /// pinned 元素地址物化(RXS-0191:`rxrt_pinned_ptr` + 越界检查 + 指针算术;
    /// 每次调用取 ptr,正确优先)。返回指向元素的 `*mut T` local(经 Deref 读写)。
    fn gpu_pinned_elem_ptr(
        &mut self,
        hp: Operand,
        idx: Operand,
        elem: &Ty,
        elem_size: u64,
        span: Span,
    ) -> LocalIdx {
        // off = i * sizeof(T)
        let off = self.gpu_bytes_of(idx, elem_size, span);
        // 越界 → 诊断行 + 终止(RXS-0193 无 UB:off ≥ bytes 即越界;元素尺寸整除
        // 分配字节数,off < bytes ⟺ off + sizeof(T) ≤ bytes)。poisoned/未知句柄时
        // rxrt_pinned_len 诊断 + 返回 0,同样命中本检查。
        let pb = self.emit_rt_call(
            "rxrt_pinned_len",
            vec![hp.clone()],
            Ty::Prim(PrimTy::U64),
            span,
        );
        let oob = self.temp(Ty::Prim(PrimTy::Bool), span);
        self.assign(
            Place::local(oob),
            Rvalue::BinaryOp(
                BinOp::Ge,
                Operand::Copy(Place::local(off)),
                Operand::Copy(Place::local(pb)),
            ),
            span,
        );
        self.emit_gpu_guard(
            oob,
            Some("RXRT: error op=pinned_index detail=index out of bounds (RXS-0193)"),
            span,
        );
        let p = self.emit_rt_call(
            "rxrt_pinned_ptr",
            vec![hp],
            Ty::RawPtr(Box::new(Ty::Prim(PrimTy::U8)), true),
            span,
        );
        // 元素地址 = (ptr as u64) + off,再回 *mut T(ptrtoint / inttoptr)。
        let pi = self.temp(Ty::Prim(PrimTy::U64), span);
        self.assign(
            Place::local(pi),
            Rvalue::Cast(Operand::Copy(Place::local(p)), Ty::Prim(PrimTy::U64)),
            span,
        );
        let addr = self.temp(Ty::Prim(PrimTy::U64), span);
        self.assign(
            Place::local(addr),
            Rvalue::BinaryOp(
                BinOp::Add,
                Operand::Copy(Place::local(pi)),
                Operand::Copy(Place::local(off)),
            ),
            span,
        );
        let ep_ty = Ty::RawPtr(Box::new(elem.clone()), true);
        let ep = self.temp(ep_ty.clone(), span);
        self.assign(
            Place::local(ep),
            Rvalue::Cast(Operand::Copy(Place::local(addr)), ep_ty),
            span,
        );
        ep
    }

    /// 宿主 GPU 编排调用 lowering(RXS-0191/0192:gpu 方法 → `rxrt_*` 字面符号 +
    /// 失败终止检查 RXS-0193)。`args[0]` = receiver 句柄(CtxCreate 无)。
    fn lower_gpu_call(
        &mut self,
        e: &tbir::Expr,
        op: crate::hir::GpuHostOp,
        args: &[tbir::Expr],
    ) -> Operand {
        use crate::hir::GpuHostOp as Op;
        let span = e.span;
        match op {
            Op::CtxCreate => {
                // 注册即传参(RXS-0192):@__rx_gpu_artifacts 嵌入描述表地址;
                // 无 kernel 编译单元 = 哨兵空表,运行期 cabi 解析确定性拒 + 终止。
                let ret = self.ty_of(e);
                let dest = self.emit_rt_call(
                    "rxrt_ctx_create",
                    vec![Operand::Const(Const::GlobalAddr(
                        "__rx_gpu_artifacts".to_owned(),
                    ))],
                    ret.clone(),
                    span,
                );
                self.guard_handle_zero(dest, span);
                self.consume(Place::local(dest), &ret)
            }
            Op::CtxStream | Op::CtxAlloc | Op::CtxAllocPinned => {
                let h = self.gpu_handle_op(&args[0]);
                let ret = self.ty_of(e);
                let dest = match op {
                    Op::CtxStream => {
                        self.emit_rt_call("rxrt_stream_create", vec![h], ret.clone(), span)
                    }
                    _ => {
                        let n = self.op_of(&args[1]);
                        let esz = self.gpu_elem_size(&ret);
                        let bytes = self.gpu_bytes_of(n, esz, span);
                        let symbol = if op == Op::CtxAlloc {
                            "rxrt_buf_alloc"
                        } else {
                            "rxrt_pinned_alloc"
                        };
                        self.emit_rt_call(
                            symbol,
                            vec![h, Operand::Copy(Place::local(bytes))],
                            ret.clone(),
                            span,
                        )
                    }
                };
                self.guard_handle_zero(dest, span);
                self.consume(Place::local(dest), &ret)
            }
            Op::CtxSync | Op::StreamSync => {
                let h = self.gpu_handle_op(&args[0]);
                let symbol = if op == Op::CtxSync {
                    "rxrt_ctx_sync"
                } else {
                    "rxrt_stream_sync"
                };
                let rc = self.emit_rt_call(symbol, vec![h], Ty::Prim(PrimTy::I32), span);
                self.guard_rc_negative(rc, span);
                Operand::Const(Const::Unit)
            }
            Op::BufLen | Op::PinnedLen => {
                let recv_ty = self.ty_of(&args[0]);
                let h = self.gpu_handle_op(&args[0]);
                let symbol = if op == Op::BufLen {
                    "rxrt_buf_len"
                } else {
                    "rxrt_pinned_len"
                };
                let bytes = self.emit_rt_call(symbol, vec![h], Ty::Prim(PrimTy::U64), span);
                let esz = self.gpu_elem_size(&recv_ty);
                let dest = self.temp(Ty::Prim(PrimTy::Usize), span);
                self.assign(
                    Place::local(dest),
                    Rvalue::BinaryOp(
                        BinOp::Div,
                        Operand::Copy(Place::local(bytes)),
                        Operand::Const(Const::Int(esz as i128, PrimTy::U64)),
                    ),
                    span,
                );
                self.consume(Place::local(dest), &Ty::Prim(PrimTy::Usize))
            }
            Op::BufUpload | Op::BufDownload => {
                // pinned 侧取指针 + 字节数 = 锁页分配字节(cabi 核对与设备缓冲
                // 精确一致,不一致 = 确定性诊断 + 负值 → 终止,RXS-0193/0194)。
                let hb = self.gpu_handle_op(&args[0]);
                let hp = self.gpu_handle_op(&args[1]);
                let p = self.emit_rt_call(
                    "rxrt_pinned_ptr",
                    vec![hp.clone()],
                    Ty::RawPtr(Box::new(Ty::Prim(PrimTy::U8)), true),
                    span,
                );
                let pb =
                    self.emit_rt_call("rxrt_pinned_len", vec![hp], Ty::Prim(PrimTy::U64), span);
                let symbol = if op == Op::BufUpload {
                    "rxrt_buf_upload"
                } else {
                    "rxrt_buf_download"
                };
                let rc = self.emit_rt_call(
                    symbol,
                    vec![
                        hb,
                        Operand::Copy(Place::local(p)),
                        Operand::Copy(Place::local(pb)),
                    ],
                    Ty::Prim(PrimTy::I32),
                    span,
                );
                self.guard_rc_negative(rc, span);
                Operand::Const(Const::Unit)
            }
            Op::PinnedGet | Op::PinnedSet => {
                let recv_ty = self.ty_of(&args[0]);
                let elem = self.ty_of(e);
                let elem = if op == Op::PinnedSet {
                    match &recv_ty {
                        Ty::Adt(_, a) => a.get(1).cloned().unwrap_or(Ty::Err),
                        _ => Ty::Err,
                    }
                } else {
                    elem
                };
                let esz = self.gpu_elem_size(&recv_ty);
                let hp = self.gpu_handle_op(&args[0]);
                let idx = self.op_of(&args[1]);
                let ep = self.gpu_pinned_elem_ptr(hp, idx, &elem, esz, span);
                let place = Place {
                    local: ep,
                    proj: vec![ProjElem::Deref],
                };
                if op == Op::PinnedGet {
                    self.consume(place, &elem)
                } else {
                    let v = self.op_of(&args[2]);
                    self.assign(place, Rvalue::Use(v), span);
                    Operand::Const(Const::Unit)
                }
            }
            // present 宿主 typestate 转移(MS1.2b,RXS-0197):消费式转移的
            // 接收者按值 move 进转移结果;`ready()` 纯类型面转移(Present →
            // Ready 同句柄再定名)不落运行时符号。
            Op::PresentReady => {
                let ret = self.ty_of(e);
                let carried = self.gpu_consume_receiver(&args[0], &ret, span);
                self.consume(Place::local(carried), &ret)
            }
            // wait / signal / present → rxp_*(消费式;负值 rc → 终止,
            // RXS-0193;fence 偶/奇协议单一事实源留 interop 帧机,RXS-0142)。
            Op::PresentWait | Op::PresentSignal | Op::PresentPresent => {
                let ret = self.ty_of(e);
                let carried = self.gpu_consume_receiver(&args[0], &ret, span);
                let symbol = match op {
                    Op::PresentWait => "rxp_wait",
                    Op::PresentSignal => "rxp_signal",
                    _ => "rxp_present",
                };
                let rc = self.emit_rt_call(
                    symbol,
                    vec![Operand::Copy(Place::local(carried))],
                    Ty::Prim(PrimTy::I32),
                    span,
                );
                self.guard_rc_negative(rc, span);
                self.consume(Place::local(carried), &ret)
            }
            // backbuffer 借用句柄(RXS-0198):接收者非消费(借用读取);
            // 句柄 0 → 终止;产 Buffer<C, f32>(drop 不释放,运行时侧 no-op)。
            Op::PresentBackbuffer => {
                let h = self.gpu_handle_op(&args[0]);
                let ret = self.ty_of(e);
                let dest = self.emit_rt_call("rxp_backbuffer", vec![h], ret.clone(), span);
                self.guard_handle_zero(dest, span);
                self.consume(Place::local(dest), &ret)
            }
            // pump(RXS-0197):负值 → 终止;非负 rc != 0 → bool(关闭请求)。
            Op::PresentPump => {
                let h = self.gpu_handle_op(&args[0]);
                let rc = self.emit_rt_call("rxp_pump", vec![h], Ty::Prim(PrimTy::I32), span);
                self.guard_rc_negative(rc, span);
                let b = self.temp(Ty::Prim(PrimTy::Bool), span);
                self.assign(
                    Place::local(b),
                    Rvalue::BinaryOp(
                        BinOp::Ne,
                        Operand::Copy(Place::local(rc)),
                        Operand::Const(Const::Int(0, PrimTy::I32)),
                    ),
                    span,
                );
                self.consume(Place::local(b), &Ty::Prim(PrimTy::Bool))
            }
            // present 会话构造(RXS-0197):rxp_create(ctx, rw, rh, ww, wh);
            // 句柄 0 → 终止(RXS-0193)。
            Op::PresentCreate => {
                let ret = self.ty_of(e);
                let h = self.gpu_handle_op(&args[0]);
                let mut call_args = vec![h];
                for a in &args[1..] {
                    call_args.push(self.op_of(a));
                }
                let dest = self.emit_rt_call("rxp_create", call_args, ret.clone(), span);
                self.guard_handle_zero(dest, span);
                self.consume(Place::local(dest), &ret)
            }
            // 宿主图像落盘桥(RXS-0199):指针/元素数自锁页句柄物化(RXS-0191
            // 同机制);n ≠ w·h·3 失配 / IO 失败由 cabi 确定性拒(负值 → 终止,
            // RXS-0193);PPM 序列化语义 = RXS-0114~0117(运行时桥接 image-io)。
            Op::WritePpm => {
                let path = self.op_of(&args[0]);
                let w = self.op_of(&args[1]);
                let h = self.op_of(&args[2]);
                let hp = self.gpu_handle_op(&args[3]);
                let p = self.emit_rt_call(
                    "rxrt_pinned_ptr",
                    vec![hp.clone()],
                    Ty::RawPtr(Box::new(Ty::Prim(PrimTy::U8)), true),
                    span,
                );
                let pb =
                    self.emit_rt_call("rxrt_pinned_len", vec![hp], Ty::Prim(PrimTy::U64), span);
                let n = self.temp(Ty::Prim(PrimTy::U64), span);
                self.assign(
                    Place::local(n),
                    Rvalue::BinaryOp(
                        BinOp::Div,
                        Operand::Copy(Place::local(pb)),
                        Operand::Const(Const::Int(4, PrimTy::U64)),
                    ),
                    span,
                );
                let rc = self.emit_rt_call(
                    "rxio_write_ppm",
                    vec![
                        path,
                        w,
                        h,
                        Operand::Copy(Place::local(p)),
                        Operand::Copy(Place::local(n)),
                    ],
                    Ty::Prim(PrimTy::I32),
                    span,
                );
                self.guard_rc_negative(rc, span);
                Operand::Const(Const::Unit)
            }
            // G3.4 bindless(RXS-0235):TextureTable 宿主注册面 → `rxrt_table_*`
            // 字面符号(镜像 `rxrt_buf_*`;RXS-0194 只追加符号族)。
            Op::CtxTextureTable => {
                let h = self.gpu_handle_op(&args[0]);
                let ret = self.ty_of(e);
                let dest = self.emit_rt_call("rxrt_table_create", vec![h], ret.clone(), span);
                self.guard_handle_zero(dest, span);
                self.consume(Place::local(dest), &ret)
            }
            // register(注册序即索引):失败哨兵 `u32::MAX`(cabi 诊断已落 stderr)→
            // 终止(RXS-0193 失败返回值检查纪律;buf 实参非消费,镜像 launch Buffer
            // 实参 gpu_handle_op 纪律)。
            Op::TableRegister => {
                let ht = self.gpu_handle_op(&args[0]);
                let hx = self.gpu_handle_op(&args[1]);
                let dest = self.emit_rt_call(
                    "rxrt_table_register",
                    vec![ht, hx],
                    Ty::Prim(PrimTy::U32),
                    span,
                );
                let bad = self.temp(Ty::Prim(PrimTy::Bool), span);
                self.assign(
                    Place::local(bad),
                    Rvalue::BinaryOp(
                        BinOp::Eq,
                        Operand::Copy(Place::local(dest)),
                        Operand::Const(Const::Int(i128::from(u32::MAX), PrimTy::U32)),
                    ),
                    span,
                );
                self.emit_gpu_guard(bad, None, span);
                self.consume(Place::local(dest), &Ty::Prim(PrimTy::U32))
            }
            // len(已注册计数;0 为合法值,不设 guard)。
            Op::TableLen => {
                let h = self.gpu_handle_op(&args[0]);
                let dest =
                    self.emit_rt_call("rxrt_table_len", vec![h], Ty::Prim(PrimTy::U32), span);
                self.consume(Place::local(dest), &Ty::Prim(PrimTy::U32))
            }
            Op::Launch => unreachable!("launch 走 GpuLaunch 节点(RXS-0191)"),
        }
    }

    /// launch 宿主 lowering(🔒 RXS-0191 marshalling):实参元组物化为栈上
    /// `[u64; n]` slot(Tuple[u64;n] 布局 = 连续 8 字节槽)+ `[u8; n]` kinds
    /// (0 = Buffer 句柄,cabi 换设备指针;1 = 标量按位样式存槽低位——f32/i32
    /// 直接 store 进槽低 4 字节,LE 下 cuLaunchKernel 按形参尺寸读取);kernel
    /// 符号 = device MIR 同源 `mangle(name, def, &[])`(单一事实源)的 NUL 终止
    /// 字符串常量。物化纪律复刻 interop.rs `AcquiredFrame::launch`。
    fn lower_gpu_launch(
        &mut self,
        e: &tbir::Expr,
        stream: &tbir::Expr,
        kernel: DefId,
        grid: &[tbir::Expr],
        block: &[tbir::Expr],
        args: &[tbir::Expr],
    ) -> Operand {
        let span = e.span;
        let s = self.gpu_handle_op(stream);
        let entry = mangle(&self.krate.item(kernel).name, kernel, &[]);

        // 维度分量 → u32(缺轴补 1,RXS-0074 维度契约已裁决 grid/block 同维)。
        let mut dims: Vec<Operand> = Vec::with_capacity(6);
        for comps in [grid, block] {
            for i in 0..3 {
                dims.push(self.gpu_dim_op(comps, i));
            }
        }

        // slot/kinds 物化(空实参仍物化 1 槽哨兵,n_args = 0 时 cabi 不读)。
        let n = args.len();
        let slot_count = n.max(1);
        let mut slot_ops: Vec<Operand> = Vec::with_capacity(slot_count);
        let mut kind_ops: Vec<Operand> = Vec::with_capacity(slot_count);
        for a in args {
            let ty = self.ty_of(a);
            match &ty {
                Ty::Adt(d, _) if self.res.lang_items.is_buffer(*d) => {
                    slot_ops.push(self.gpu_handle_op(a));
                    kind_ops.push(Operand::Const(Const::Int(0, PrimTy::U8)));
                }
                _ => {
                    slot_ops.push(self.op_of(a));
                    kind_ops.push(Operand::Const(Const::Int(1, PrimTy::U8)));
                }
            }
        }
        if n == 0 {
            slot_ops.push(Operand::Const(Const::Int(0, PrimTy::U64)));
            kind_ops.push(Operand::Const(Const::Int(0, PrimTy::U8)));
        }
        let slots_ty = Ty::Tuple(vec![Ty::Prim(PrimTy::U64); slot_count]);
        let slots = self.temp(slots_ty.clone(), span);
        self.assign(
            Place::local(slots),
            Rvalue::Aggregate(slots_ty.clone(), slot_ops),
            span,
        );
        let kinds_ty = Ty::Tuple(vec![Ty::Prim(PrimTy::U8); slot_count]);
        let kinds = self.temp(kinds_ty.clone(), span);
        self.assign(
            Place::local(kinds),
            Rvalue::Aggregate(kinds_ty.clone(), kind_ops),
            span,
        );
        // slot/kinds 地址稳定至 rxrt_launch 返回(cuLaunchKernel 调用内拷贝参数)。
        let slots_ref = self.temp(Ty::Ref(Box::new(slots_ty), false), span);
        self.assign(
            Place::local(slots_ref),
            Rvalue::Ref(BorrowKind::Shared, Place::local(slots)),
            span,
        );
        let kinds_ref = self.temp(Ty::Ref(Box::new(kinds_ty), false), span);
        self.assign(
            Place::local(kinds_ref),
            Rvalue::Ref(BorrowKind::Shared, Place::local(kinds)),
            span,
        );

        let mut call_args = vec![s, Operand::Const(Const::Str(entry))];
        call_args.extend(dims);
        call_args.push(Operand::Copy(Place::local(slots_ref)));
        call_args.push(Operand::Copy(Place::local(kinds_ref)));
        call_args.push(Operand::Const(Const::Int(n as i128, PrimTy::U64)));
        let rc = self.emit_rt_call("rxrt_launch", call_args, Ty::Prim(PrimTy::I32), span);
        self.guard_rc_negative(rc, span);
        Operand::Const(Const::Unit)
    }

    /// launch 维度分量 → u32 值(缺轴补常量 1)。
    fn gpu_dim_op(&mut self, comps: &[tbir::Expr], i: usize) -> Operand {
        match comps.get(i) {
            Some(c) => {
                let o = self.op_of(c);
                let t = self.temp(Ty::Prim(PrimTy::U32), c.span);
                self.assign(
                    Place::local(t),
                    Rvalue::Cast(o, Ty::Prim(PrimTy::U32)),
                    c.span,
                );
                Operand::Copy(Place::local(t))
            }
            None => Operand::Const(Const::Int(1, PrimTy::U32)),
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

/// 整型字面量文本解析(pub(crate):typeck gather 分量 0..=3 字面量核验复用,
/// RXS-0223;与 MIR 常量物化同一事实源)。
pub(crate) fn parse_int(text: &str, suffix: Option<LitSuffix>) -> Option<i128> {
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

    /// 默认路径零漂移(R1.2/R6.7):`build_crate` 产出的每个 `Body` 的图形阶段
    /// 意图字段恒中立(`stage = None`、`io_sig` 空)——`mir::Body` 扩展不改默认
    /// (host/PTX)路径 MIR 构造行为(RXS-0161;图形阶段根收集属后续分片)。
    #[test]
    fn default_path_bodies_carry_neutral_shader_signature() {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(
            "fn pick<T>(a: T, b: T) -> T { a }\n\
             fn main() {\n    let _i = pick(1i64, 2);\n    let _x = 3 + 4;\n}",
            SourceId(0),
            Edition::Rx0,
            &diag,
        );
        cx.check_crate();
        assert!(diag.emitted().is_empty(), "前置诊断: {:?}", diag.emitted());
        let mir = cx.mir_crate();
        assert!(!mir.is_empty(), "应至少收集到 main 实例");
        for body in mir.iter() {
            assert_eq!(
                body.stage, None,
                "默认路径 body `{}` 不应携着色阶段",
                body.symbol
            );
            assert!(
                body.io_sig.is_empty(),
                "默认路径 body `{}` 不应携 I/O 意图签名",
                body.symbol
            );
        }
    }

    /// `dxil-backend` 下 `build_device_crate` 收 vertex/fragment 图形阶段根,且
    /// 自 AST `shader_stages` 携 I/O 意图签名进 MIR(RXS-0161,R1.3):图形根
    /// 进入 device MIR、`stage` 置 `Some(Vertex|Fragment)`、`io_sig` 非空且逐
    /// 元素四维度(字段名 / builtin·interpolate·varying 种类 / 类型 / in|out
    /// 方向)保真;资源句柄(`Texture2D`/`Sampler`)非命名 I/O 结构体,不入
    /// io_sig(opaque handle 形态)。
    //@ spec: RXS-0161
    #[cfg(all(feature = "dxil-backend", feature = "shader-stages"))]
    #[test]
    fn dxil_backend_collects_graphics_roots_with_io_sig() {
        use crate::ast::ShaderStage;
        use crate::hir::PrimTy;
        use crate::mir::{IoDir, IoSigKind, MirIoType};

        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(
            "struct VsOut {\n\
            \x20   #[builtin(position)] pos: f32,\n\
            \x20   #[interpolate(perspective)] uv: f32,\n\
            \x20   #[interpolate(flat)] mat_id: u32,\n\
             }\n\
             vertex fn vs_main() -> VsOut {\n\
            \x20   VsOut { pos: 0.0, uv: 0.0, mat_id: 0 }\n\
             }\n\
             fragment fn fs_main(inp: VsOut, tex: Texture2D<f32>, samp: Sampler) -> VsOut {\n\
            \x20   inp\n\
             }\n\
             fn main() {}",
            SourceId(0),
            Edition::Rx0,
            &diag,
        );
        let device = cx.device_mir_crate();

        // 图形阶段根入 device MIR(vertex + fragment 各一)。
        let vs = device
            .iter()
            .find(|b| b.stage == Some(ShaderStage::Vertex))
            .expect("vertex 图形阶段根应进入 device MIR");
        let fs = device
            .iter()
            .find(|b| b.stage == Some(ShaderStage::Fragment))
            .expect("fragment 图形阶段根应进入 device MIR");

        // vertex:返回 VsOut → 3 个 Out 元素(builtin / interpolate ×2)。
        assert_eq!(vs.io_sig.len(), 3, "vertex io_sig: {:?}", vs.io_sig);
        assert!(
            vs.io_sig.iter().all(|e| e.dir == IoDir::Out),
            "vertex 返回位置 I/O 应全为 Out 方向"
        );
        let pos = &vs.io_sig[0];
        assert_eq!(pos.field_name, "pos");
        assert_eq!(pos.kind, IoSigKind::Builtin("position".to_owned()));
        assert_eq!(pos.ty, MirIoType::Scalar(PrimTy::F32));
        assert_eq!(
            vs.io_sig[1].kind,
            IoSigKind::Interpolate("perspective".to_owned())
        );
        assert_eq!(vs.io_sig[2].kind, IoSigKind::Interpolate("flat".to_owned()));
        assert_eq!(vs.io_sig[2].ty, MirIoType::Scalar(PrimTy::U32));

        // fragment:形参 VsOut(3 个 In)+ 返回 VsOut(3 个 Out);资源句柄不入。
        assert_eq!(fs.io_sig.len(), 6, "fragment io_sig: {:?}", fs.io_sig);
        let ins = fs.io_sig.iter().filter(|e| e.dir == IoDir::In).count();
        let outs = fs.io_sig.iter().filter(|e| e.dir == IoDir::Out).count();
        assert_eq!((ins, outs), (3, 3), "fragment In/Out 计数");
        assert!(
            !fs.io_sig
                .iter()
                .any(|e| e.field_name == "tex" || e.field_name == "samp"),
            "资源句柄不应进入 io_sig"
        );
    }

    /// G3.3 采样方法族 lowering(RXS-0223/0226;RFC-0013 §4.B1):新方法自 `.rx`
    /// 源经 typeck(`sample_family_calls`)→ tbir `ResourceMethodCall` → MIR
    /// `Rvalue::ResourceSample{method, extra}`(方法判别 + sampler 携带 + extra
    /// 形态);既有 `.sample()` 路持续产 `SampleLod` 空 extra(byte-preserving,
    /// Q-S-SampleName,uc04 golden 0-byte 由 dxil_golden 硬门另证)。
    //@ spec: RXS-0223, RXS-0226
    #[cfg(all(feature = "dxil-backend", feature = "shader-stages"))]
    #[test]
    fn sample_family_lowering_carries_method_and_extra() {
        use crate::ast::ShaderStage;
        use crate::mir::{ResourceMethod, Rvalue, StatementKind};

        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(
            "struct V {\n\
            \x20   #[interpolate(perspective)] uv: vec2<f32>,\n\
            \x20   #[interpolate(flat)] px: vec2<u32>,\n\
            \x20   #[interpolate(perspective)] val: vec4<f32>,\n\
             }\n\
             struct O {\n\
            \x20   a: vec4<f32>,\n\
            \x20   b: vec4<f32>,\n\
            \x20   c: vec4<f32>,\n\
            \x20   d: f32,\n\
             }\n\
             fragment fn fs_family(\n\
            \x20   inp: V,\n\
            \x20   tex: Texture2D<f32>,\n\
            \x20   samp: Sampler,\n\
            \x20   scmp: SamplerCmp,\n\
            \x20   img: TextureRw2D<f32>,\n\
             ) -> O {\n\
            \x20   img.store(inp.px, inp.val);\n\
            \x20   O {\n\
            \x20       a: tex.sample(samp, inp.uv),\n\
            \x20       b: tex.gather(samp, inp.uv, 2),\n\
            \x20       c: img.load(inp.px),\n\
            \x20       d: tex.sample_cmp(scmp, inp.uv, 0.5),\n\
            \x20   }\n\
             }\n\
             fn main() {}",
            SourceId(0),
            Edition::Rx0,
            &diag,
        );
        cx.check_crate();
        assert!(
            !diag.has_errors(),
            "方法族语料应 0 诊断(实得 {:?})",
            diag.emitted()
                .iter()
                .filter_map(|d| d.code)
                .collect::<Vec<_>>()
        );
        let device = cx.device_mir_crate();
        let fs = device
            .iter()
            .find(|b| b.stage == Some(ShaderStage::Fragment))
            .expect("fragment 图形阶段根应进入 device MIR");

        // (方法, 携 sampler?, extra 元数) 三元组集(块序无关断言)。
        let mut seen: Vec<(ResourceMethod, bool, usize)> = Vec::new();
        for bb in &fs.blocks {
            for st in &bb.stmts {
                let StatementKind::Assign(_, rv) = &st.kind;
                if let Rvalue::ResourceSample {
                    method,
                    sampler_local,
                    extra,
                    ..
                } = rv
                {
                    seen.push((*method, sampler_local.is_some(), extra.len()));
                }
            }
        }
        for want in [
            // 既有 `.sample()` → SampleLod 空 extra(byte-preserving 承接)。
            (ResourceMethod::SampleLod, true, 0),
            // gather → 分量字面量入 extra。
            (ResourceMethod::Gather, true, 1),
            // rw store → 无 sampler + store 值入 extra(唯一写者 🔒 RXS-0229)。
            (ResourceMethod::Store, false, 1),
            // rw load → 无 sampler 无 extra。
            (ResourceMethod::StorageLoad, false, 0),
            // sample_cmp → dref 入 extra。
            (ResourceMethod::SampleCmp, true, 1),
        ] {
            assert!(
                seen.contains(&want),
                "MIR 应含方法族 rvalue {want:?}(实得 {seen:?})"
            );
        }
    }

    /// PR-E2b E2b-1 端到端(闭合 assumed-1):着色阶段签名资源句柄形参 →
    /// `Body.resources` 收集(RXS-0163)→ `emit_spirv` 资源绑定装饰 emit
    /// (`DescriptorSet`/`Binding` + opaque 资源类型)→ `infer_root_signature` +
    /// `serialize_rts0` 产 RTS0 容器(RXS-0165)。证明 MIR→SPIR-V 资源绑定不再
    /// 「结构上不可达」:资源句柄端到端贯通 emit 与 root signature 推导。
    //@ spec: RXS-0163, RXS-0165
    #[cfg(all(feature = "dxil-backend", feature = "shader-stages"))]
    #[test]
    fn e2b1_resources_flow_into_spirv_and_root_signature() {
        use crate::ast::ShaderStage;
        use crate::hir::PrimTy;
        use crate::mir::{MirResourceType, ResourceClass, ResourceCount};

        // SPIR-V 解码常量(core 规范)。
        const OP_DECORATE: u16 = 71;
        const OP_TYPE_IMAGE: u16 = 25;
        const OP_TYPE_SAMPLER: u16 = 26;
        const DECORATION_BINDING: u32 = 33;
        const DECORATION_DESCRIPTOR_SET: u32 = 34;

        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(
            "struct FsOut {\n\
            \x20   color: f32,\n\
             }\n\
             fragment fn fs_main(tex: Texture2D<f32>, samp: Sampler) -> FsOut {\n\
            \x20   FsOut { color: 0.0 }\n\
             }\n\
             fn main() {}",
            SourceId(0),
            Edition::Rx0,
            &diag,
        );
        assert!(
            !diag.has_errors(),
            "资源句柄 fragment 前段应 0 诊断(实得 {:?})",
            diag.emitted()
                .iter()
                .filter_map(|d| d.code)
                .collect::<Vec<_>>()
        );
        let device = cx.device_mir_crate();
        let fs = device
            .iter()
            .find(|b| b.stage == Some(ShaderStage::Fragment))
            .expect("fragment 图形阶段根应进入 device MIR");

        // 1) 资源句柄形参按声明序进 `Body.resources`(io_sig 与 resources 不交叠)。
        assert_eq!(fs.resources.len(), 2, "resources: {:?}", fs.resources);
        assert_eq!(fs.resources[0].name, "tex");
        assert_eq!(fs.resources[0].res, MirResourceType::Texture2D(PrimTy::F32));
        assert_eq!(fs.resources[0].res.class(), ResourceClass::Srv);
        assert_eq!(fs.resources[0].count, ResourceCount::One);
        assert_eq!(fs.resources[1].name, "samp");
        assert_eq!(fs.resources[1].res, MirResourceType::Sampler);
        assert_eq!(fs.resources[1].res.class(), ResourceClass::Sampler);

        // 2) emit_spirv 携资源 → 资源绑定装饰 + opaque 资源类型(闭合 assumed-1)。
        let spv = crate::dxil_spirv::emit_spirv(ShaderStage::Fragment, &fs.io_sig, &fs.resources)
            .expect("带资源的 fragment emit 应 Ok");
        // 手动遍历指令(跳过 5 字 header),统计资源装饰 / opaque 类型。
        let mut sets = Vec::new();
        let mut bindings = Vec::new();
        let mut has_image = false;
        let mut has_sampler = false;
        let mut i = 5;
        while i < spv.len() {
            let word = spv[i];
            let wc = (word >> 16) as usize;
            let op = (word & 0xFFFF) as u16;
            if wc == 0 || i + wc > spv.len() {
                break;
            }
            let ops = &spv[i + 1..i + wc];
            match op {
                OP_DECORATE if ops.get(1) == Some(&DECORATION_DESCRIPTOR_SET) => sets.push(ops[2]),
                OP_DECORATE if ops.get(1) == Some(&DECORATION_BINDING) => bindings.push(ops[2]),
                OP_TYPE_IMAGE => has_image = true,
                OP_TYPE_SAMPLER => has_sampler = true,
                _ => {}
            }
            i += wc;
        }
        assert!(has_image, "Texture2D 应 emit OpTypeImage");
        assert!(has_sampler, "Sampler 应 emit OpTypeSampler");
        assert_eq!(sets, vec![0, 0], "首期单 set,两资源 DescriptorSet 恒 0");
        assert_eq!(
            bindings,
            vec![0, 0],
            "Binding 按种类轴 per-class 从 0(tex=SRV t0, samp=Sampler s0;RXS-0164 与 RTS0 同口径)"
        );
        // 确定性:同输入二次 emit 字节全等。
        assert_eq!(
            spv,
            crate::dxil_spirv::emit_spirv(ShaderStage::Fragment, &fs.io_sig, &fs.resources)
                .unwrap()
        );

        // 3) root signature 形态推导 + RTS0 容器序列化(RXS-0165;确定性)。
        let rs = crate::binding_layout::infer_root_signature(&fs.resources)
            .expect("Texture2D/Sampler 应可推导 root signature");
        let rts0 = crate::binding_layout::serialize_rts0(&rs);
        assert_eq!(&rts0[0..4], b"DXBC", "RTS0 外层 DXBC 容器 fourcc");
        assert!(
            rts0.windows(4).any(|w| w == b"RTS0"),
            "容器应含 RTS0 part fourcc"
        );
        assert_eq!(
            rts0,
            crate::binding_layout::serialize_rts0(&rs),
            "RTS0 序列化应确定性(同输入字节全等)"
        );
    }
}
