//! device MIR → DXIL DirectX 三元组 LLVM IR 文本(G2.2 PR-C2;RXS-0157 分片1 +
//! RXS-0158 分片2 + RXS-0159 分片3,RFC-0003 §4.1/§4.2,D-131=A)。
//!
//! 本模块 gate 于 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate);未启用时整模块
//! 不编入 rurixc,PTX 路径(D-207)不受影响。target 分发在 MIR 之后分叉:DXIL 后端与
//! NVPTX 后端(`device_codegen`)并列、各自从 MIR 独立降级,不共享后端 lowering
//! (RFC-0003 §4.5)。DXIL 收集根(`build_dxil_crate`)扩到含着色阶段入口,PTX 收集根
//! (`build_device_crate`)维持排除着色阶段(D-207)。
//!
//! **阶段 → DXIL 着色器类型降级(RXS-0158)**:按 [`stage_target`] 将 RXS-0153 着色
//! 阶段映射为 DXIL 着色器类型 + shader profile——`compute`(及 `kernel`,
//! compute-via-kernel)→ compute shader(`shadermodel6.0-compute` + `hlsl.numthreads`)、
//! `vertex` → vertex shader(`shadermodel6.0-vertex`)、`fragment` → pixel shader
//! (`shadermodel6.0-pixel`)。本片落地阶段沿 RXS-0157 最小子集(无 ABI 形参、平凡空
//! 体 → DXIL `void` 入口);mesh/task/RT 阶段映射已登记(RXS-0158 对应表)但实现
//! deferred(RD-012)→ `RX6008`。子集外构造(I/O 签名形参——RXS-0159 / 非平凡体)
//! → `RX6007`。
//!
//! 下游(IR → patched llc -filetype=obj → DXIL 容器 → dxc validator)见
//! [`crate::toolchain::ir_to_dxil`];golden 取文本反汇编经 validator 验证(RFC-0003
//! §9 Q-Golden)。
//!
//! **阶段 I/O → DXIL 签名/系统值语义降级(RXS-0159,分片3,类型面)**:
//! vertex/fragment 入口的 `#[builtin]` 内建变量 → DXIL 系统值语义名(SV_Position /
//! SV_Target / SV_VertexID / ...,按阶段 + 方向裁定,见 [`sv_semantic`]),
//! `#[interpolate]` 插值限定 → DXIL 插值限定符(见 [`interp_modifier`]);vertex out /
//! fragment in/out 签名结构(按语义元素,非二进制布局)以**类型面签名元数据**
//! (`!rurix.dxil.sig.in` / `.out`)emit 入 IR。不可映射的内建变量 / 非法插值组合
//! (整数 varying 非 flat)→ `RX6009`(strict-only)。**Rurix 侧只设语义名 / 插值,
//! 不算签名元素的寄存器打包 / 字节偏移 / component mask**——后者属 RFC-0003 §4.6 /
//! §9 Q-Builtin 🔒 FFI ABI 禁区,由 LLVM DirectX 后端 emit,不在本文件定义/冻结。
//! 带 I/O 签名的入口 body 数据流降级 deferred(// STUB(RD-013),本片仅签名 + void stub)。
//!
//! **本片不碰** 🔒 纹理内存模型映射(06 §4.2)/ 内建变量·签名二进制 ABI 布局
//! (RFC-0003 §4.6)/ 绑定布局推导(G2.3,P-11)/ 阶段间接口链接核对(RXS-0160)。

use std::fmt::Write as _;

use crate::ast::{self, FnColor, ShaderStage};
use crate::diag::ErrorCode;
use crate::hir::{self, DefId};
use crate::mir::{Body, Const, Operand, Rvalue, StatementKind, TerminatorKind};
use crate::query::QueryCtx;
use crate::span::Span;

/// DXIL codegen 失败(RXS-0157/0158/0159)。`code` 区分诊断类别:
/// - `6007`(`codegen.dxil_unsupported`):目标不可用 / 子集外构造 / 降级失败
///   (RXS-0157 L1~L3);
/// - `6008`(`codegen.dxil_stage_unsupported`):着色阶段降级暂未支持(RXS-0158 L2,
///   mesh/task/RT 阶段 deferred RD-012);
/// - `6009`(`codegen.dxil_signature_unsupported`):阶段 I/O → DXIL 签名/系统值语义
///   降级失败(RXS-0159,不可映射的内建变量→SV_* / 非法插值组合,类型面)。
#[derive(Debug, Clone)]
pub struct DxilCodegenError {
    pub span: Span,
    pub detail: String,
    pub code: u16,
    pub message_key: &'static str,
}

impl DxilCodegenError {
    fn unsupported(span: Span, detail: impl Into<String>) -> Self {
        DxilCodegenError {
            span,
            detail: detail.into(),
            code: 6007,
            message_key: "codegen.dxil_unsupported",
        }
    }

    fn stage_deferred(span: Span, detail: impl Into<String>) -> Self {
        DxilCodegenError {
            span,
            detail: detail.into(),
            code: 6008,
            message_key: "codegen.dxil_stage_unsupported",
        }
    }

    /// RXS-0159:阶段 I/O → DXIL 签名/系统值语义降级失败(类型面,RX6009)。
    fn signature(span: Span, detail: impl Into<String>) -> Self {
        DxilCodegenError {
            span,
            detail: detail.into(),
            code: 6009,
            message_key: "codegen.dxil_signature_unsupported",
        }
    }
}

/// 阶段 → DXIL 着色器类型降级目标(RXS-0158 着色器类型对应表)。
struct DxilStageTarget {
    /// DirectX 三元组的 shader-stage 环境分量(`dxil-unknown-shadermodel<sm>-<env>`)。
    triple_env: &'static str,
    /// 入口 `hlsl.shader` 属性值。
    hlsl_shader: &'static str,
    /// shader model(三元组分量,如 `6.0` → `shadermodel6.0`)。
    sm: &'static str,
    /// 是否附 `hlsl.numthreads` 入口属性(compute/mesh/task)。
    needs_numthreads: bool,
}

/// 着色阶段 → DXIL 着色器类型降级目标裁定(RXS-0158)。`stage == None` 取 compute
/// (RXS-0153 compute-via-kernel:普通 `kernel fn`)。mesh/task/RT 阶段的合规降级
/// 越出阶段→着色器类型类型面(线程组/DispatchMesh/输出拓扑或 library 多入口 + I/O
/// 签名 ABI),本片仅登记映射、不实现 → `stage_deferred`(RX6008,RD-012)。
fn stage_target(
    stage: Option<ShaderStage>,
    span: Span,
) -> Result<DxilStageTarget, DxilCodegenError> {
    let t = |triple_env, hlsl_shader, sm, needs_numthreads| DxilStageTarget {
        triple_env,
        hlsl_shader,
        sm,
        needs_numthreads,
    };
    Ok(match stage {
        None | Some(ShaderStage::Compute) => t("compute", "compute", "6.0", true),
        Some(ShaderStage::Vertex) => t("vertex", "vertex", "6.0", false),
        Some(ShaderStage::Fragment) => t("pixel", "pixel", "6.0", false),
        Some(ShaderStage::Mesh) => {
            return Err(DxilCodegenError::stage_deferred(
                span,
                "mesh 着色阶段(→ DXIL mesh shader,SM6.5)的合规降级需线程组维度 + 输出拓扑声明,越出阶段→着色器类型类型面;映射已登记 RXS-0158 对应表,实现 deferred(RD-012)",
            ));
        }
        Some(ShaderStage::Task) => {
            return Err(DxilCodegenError::stage_deferred(
                span,
                "task 着色阶段(→ DXIL amplification shader,SM6.5)的合规降级需线程组维度 + DispatchMesh 声明,越出阶段→着色器类型类型面;映射已登记 RXS-0158 对应表,实现 deferred(RD-012)",
            ));
        }
        Some(
            ShaderStage::RayGen | ShaderStage::ClosestHit | ShaderStage::AnyHit | ShaderStage::Miss,
        ) => {
            return Err(DxilCodegenError::stage_deferred(
                span,
                "RT 着色阶段(raygen/closesthit/anyhit/miss → DXIL library 多入口,SM6.3)为 library 形态,越出阶段→着色器类型类型面;映射已登记 RXS-0158 对应表,实现 deferred(RD-012)",
            ));
        }
    })
}

/// 取 MIR body 对应 HIR 函数的着色阶段标记(RXS-0153;`None` = 普通 kernel/compute)。
fn stage_of(cx: &QueryCtx<'_>, def: DefId) -> Option<ShaderStage> {
    match &cx.hir_crate().item(def).kind {
        hir::ItemKind::Fn(decl) => decl.stage,
        _ => None,
    }
}

// ===========================================================================
// RXS-0159 阶段 I/O → DXIL 签名 / 系统值语义降级(类型面)
// ===========================================================================

/// 签名方向(input = 形参 I/O 结构体;output = 返回 I/O 结构体)。
#[derive(Clone, Copy, PartialEq, Eq)]
enum SigDir {
    In,
    Out,
}

/// 单个签名元素的语义(类型面;**无寄存器/偏移/component mask**——二进制布局属
/// RFC-0003 §4.6 🔒 FFI ABI 禁区,由 LLVM DirectX 后端 emit)。
enum SigSem {
    /// 系统值语义名(`#[builtin]` → SV_*)。
    Sv(&'static str),
    /// 用户 varying 的 DXIL 插值限定符(`#[interpolate]` → linear/nointerpolation/...)。
    Interp(&'static str),
}

/// 一条签名元素:字段名 + 语义(类型面)。
struct SigElem {
    field: String,
    sem: SigSem,
}

/// 阶段 I/O 签名(类型面:输入/输出元素按语义,无二进制布局)。
struct StageSig {
    inputs: Vec<SigElem>,
    outputs: Vec<SigElem>,
}

impl StageSig {
    fn is_empty(&self) -> bool {
        self.inputs.is_empty() && self.outputs.is_empty()
    }
}

/// `#[builtin(name)]` + 阶段 + 方向 → DXIL 系统值语义名(SV_*)。类型面映射:仅语义
/// 名,**不涉寄存器/偏移**。该阶段/方向无对应 DXIL 系统值 → `None`(上层 → RX6009)。
/// 内建变量集来自 RXS-0154(已由 shader_stages 校验已知性);本表只裁其在 DXIL
/// 签名中的 SV 对应。
fn sv_semantic(stage: ShaderStage, dir: SigDir, builtin: &str) -> Option<&'static str> {
    use ShaderStage::{Fragment, Vertex};
    match (stage, dir, builtin) {
        // vertex 输入:顶点取数系统值。
        (Vertex, SigDir::In, "vertex_id") => Some("SV_VertexID"),
        (Vertex, SigDir::In, "instance_id") => Some("SV_InstanceID"),
        // vertex 输出:裁剪空间位置。
        (Vertex, SigDir::Out, "position") => Some("SV_Position"),
        // fragment 输入:片元坐标 / 朝向 / 图元 id。
        (Fragment, SigDir::In, "frag_coord") => Some("SV_Position"),
        (Fragment, SigDir::In, "front_facing") => Some("SV_IsFrontFace"),
        (Fragment, SigDir::In, "primitive_id") => Some("SV_PrimitiveID"),
        // fragment 输出:深度系统值(颜色经 #[interpolate] → SV_Target,见 map_field)。
        (Fragment, SigDir::Out, "depth") => Some("SV_Depth"),
        // 其余阶段/方向组合无对应 DXIL 系统值(thread_id 属 compute、错向 builtin 等)。
        _ => None,
    }
}

/// RXS-0154 插值限定 → DXIL 插值限定符(HLSL 关键字形态)。
fn interp_modifier(mode: &str) -> Option<&'static str> {
    Some(match mode {
        // 透视校正(HLSL `linear` = 透视校正,无独立 perspective 关键字)。
        "perspective" | "linear" => "linear",
        "noperspective" => "noperspective",
        "flat" => "nointerpolation",
        "centroid" => "centroid",
        "sample" => "sample",
        _ => return None, // RXS-0154 已校验已知性;防御性
    })
}

/// 类型文本头名是否为整数(DXIL:整数 varying 必须 flat/nointerpolation)。
fn is_integer_ty(ty_head: &str) -> bool {
    matches!(
        ty_head,
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize" | "bool"
    )
}

/// 字段 I/O 标注(首个 `#[builtin(..)]` / `#[interpolate(..)]`;对齐 shader_stages)。
enum FieldAnno {
    Builtin(String),
    Interpolate(String),
    None,
}

fn field_anno(f: &ast::FieldDef) -> FieldAnno {
    for attr in &f.attrs {
        let [seg] = attr.meta.path.segments.as_slice() else {
            continue;
        };
        let key = seg.ident.name.as_str();
        if key != "builtin" && key != "interpolate" {
            continue;
        }
        let arg = match &attr.meta.kind {
            ast::MetaKind::List(inner) => inner.iter().find_map(|mi| match mi {
                ast::MetaInner::Meta(m) => m.path.segments.last().map(|s| s.ident.name.clone()),
                ast::MetaInner::Lit(_) => None,
            }),
            _ => None,
        }
        .unwrap_or_default();
        return if key == "builtin" {
            FieldAnno::Builtin(arg)
        } else {
            FieldAnno::Interpolate(arg)
        };
    }
    FieldAnno::None
}

/// 类型头名(`Vec4<f32>` → "Vec4";引用取内层;非路径 → None)。
fn ty_head_name(ty: &ast::Ty) -> Option<&str> {
    match &ty.kind {
        ast::TyKind::Path(p) => p.segments.last().map(|s| s.ident.name.as_str()),
        ast::TyKind::Paren(inner)
        | ast::TyKind::Ref { inner, .. }
        | ast::TyKind::RawPtr { inner, .. } => ty_head_name(inner),
        _ => None,
    }
}

/// 在 AST(含嵌套 mod)按名定位命名字段结构体的字段集。
fn struct_fields<'a>(items: &'a [ast::Item], name: &str) -> Option<&'a [ast::FieldDef]> {
    for it in items {
        match &it.kind {
            ast::ItemKind::Struct(s) if s.name.name == name => {
                if let ast::VariantBody::Named(fields) = &s.body {
                    return Some(fields);
                }
            }
            ast::ItemKind::Mod(m) => {
                if let Some(f) = struct_fields(&m.items, name) {
                    return Some(f);
                }
            }
            _ => {}
        }
    }
    None
}

/// 在 AST(含嵌套 mod)按名 + 阶段定位着色阶段函数(RXS-0153)。
fn find_stage_fn<'a>(
    items: &'a [ast::Item],
    name: &str,
    stage: ShaderStage,
) -> Option<&'a ast::FnItem> {
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) if f.name.name == name && f.stage == Some(stage) => {
                return Some(f);
            }
            ast::ItemKind::Mod(m) => {
                if let Some(f) = find_stage_fn(&m.items, name, stage) {
                    return Some(f);
                }
            }
            _ => {}
        }
    }
    None
}

/// 单字段 → DXIL 签名语义(类型面)。不可映射内建变量 / 非法插值组合 → RX6009。
fn map_field(
    stage: ShaderStage,
    dir: SigDir,
    f: &ast::FieldDef,
) -> Result<SigSem, DxilCodegenError> {
    let fname = &f.name.name;
    match field_anno(f) {
        FieldAnno::Builtin(name) => sv_semantic(stage, dir, &name).map(SigSem::Sv).ok_or_else(|| {
            DxilCodegenError::signature(
                f.span,
                format!(
                    "内建变量 `{name}`(字段 `{fname}`)在该着色阶段/方向无对应 DXIL 系统值语义(不可映射,类型面)"
                ),
            )
        }),
        FieldAnno::Interpolate(mode) => {
            // fragment 输出 varying = 渲染目标颜色输出 → SV_Target(插值模式对输出无意义)。
            if stage == ShaderStage::Fragment && dir == SigDir::Out {
                return Ok(SigSem::Sv("SV_Target"));
            }
            let modifier = interp_modifier(&mode).ok_or_else(|| {
                DxilCodegenError::signature(
                    f.span,
                    format!("未知插值限定 `{mode}`(字段 `{fname}`)"),
                )
            })?;
            // DXIL:整数 varying 必须 nointerpolation(flat);非 flat 整数插值非法。
            let int_ty = ty_head_name(&f.ty).is_some_and(is_integer_ty);
            if int_ty && modifier != "nointerpolation" {
                return Err(DxilCodegenError::signature(
                    f.span,
                    format!(
                        "整数 varying `{fname}` 的插值限定 `{mode}` 非法:整数 varying 必须 flat(DXIL nointerpolation)"
                    ),
                ));
            }
            Ok(SigSem::Interp(modifier))
        }
        FieldAnno::None => Err(DxilCodegenError::signature(
            f.span,
            format!("着色阶段 I/O 字段 `{fname}` 无 `#[builtin]`/`#[interpolate]` 标注,无法映射签名元素"),
        )),
    }
}

/// 一组形参/返回 I/O 结构体字段 → 签名元素(方向);非 I/O 结构体类型 → RX6007(子集外)。
fn sig_from_ty(
    items: &[ast::Item],
    ty: &ast::Ty,
    stage: ShaderStage,
    dir: SigDir,
    out: &mut Vec<SigElem>,
) -> Result<(), DxilCodegenError> {
    let head = ty_head_name(ty).unwrap_or("");
    let Some(fields) = struct_fields(items, head) else {
        return Err(DxilCodegenError::unsupported(
            ty.span,
            "DXIL 阶段 I/O 最小子集仅支持以命名 I/O 结构体表达签名(标量/资源句柄形参属绑定布局推导 G2.3 / FFI ABI 禁区,不在本片)",
        ));
    };
    for f in fields {
        out.push(SigElem {
            field: f.name.name.clone(),
            sem: map_field(stage, dir, f)?,
        });
    }
    Ok(())
}

/// 构建阶段 I/O 签名(RXS-0159,类型面)。compute/None → 空签名(无图形签名,
/// 形参合法性由最小子集校验裁 RX6007)。vertex/fragment → 从 AST 签名(形参 = 输入
/// I/O 结构体、返回 = 输出 I/O 结构体)提取 SV 语义名 / 插值限定符。
fn stage_signature(
    cx: &QueryCtx<'_>,
    def: DefId,
    stage: Option<ShaderStage>,
) -> Result<StageSig, DxilCodegenError> {
    let mut sig = StageSig {
        inputs: Vec::new(),
        outputs: Vec::new(),
    };
    let Some(stage @ (ShaderStage::Vertex | ShaderStage::Fragment)) = stage else {
        return Ok(sig); // compute / 其他阶段:无图形 I/O 签名
    };
    let krate = cx.hir_crate();
    let name = krate.item(def).name.clone();
    let Some(f) = find_stage_fn(&cx.ast().items, &name, stage) else {
        return Ok(sig); // 定位不到(理论不达)→ 空签名,交最小子集校验
    };
    for p in &f.params {
        if let ast::ParamKind::Typed { ty, .. } = &p.kind {
            sig_from_ty(&cx.ast().items, ty, stage, SigDir::In, &mut sig.inputs)?;
        }
    }
    if let Some(ret) = &f.ret {
        sig_from_ty(&cx.ast().items, ret, stage, SigDir::Out, &mut sig.outputs)?;
    }
    Ok(sig)
}

/// 驱动 / 测试入口:构建 device MIR(`kernel fn` / 着色阶段入口为根)+ DXIL 按阶段
/// codegen(RXS-0158)。无入口 → `None`;子集外 / 降级失败 → `RX6007`,deferred 阶段
/// → `RX6008`(经 `cx.diag()` 落结构化诊断并返回 `None`);成功 → `Some(DirectX 三元组
/// LLVM IR 文本)`。patched llc → DXIL 容器 + dxc validator 由驱动在产 IR 后另行实施。
pub fn build_and_emit_dxil(cx: &QueryCtx<'_>, module_name: &str) -> Option<String> {
    let bodies = cx.dxil_mir_crate();
    if bodies.is_empty() {
        return None;
    }
    // device MIR 构建已报错 → 不级联 codegen(防一错多报,对齐 device_codegen)。
    if cx.diag().has_errors() {
        return None;
    }
    // 入口 = 首个 kernel 着色 body(含着色阶段入口,RXS-0158;取首个为最小入口)。
    let entry = bodies.iter().find(|b| b.color == FnColor::Kernel)?;
    let stage = stage_of(cx, entry.def);
    // RXS-0159:先从 AST 签名提取阶段 I/O → DXIL 签名/系统值语义(类型面);
    // deferred 阶段(mesh/task/RT)返回空签名,RX6008 由 emit_dxil_ir 的 stage_target 裁。
    let result = stage_signature(cx, entry.def, stage)
        .and_then(|sig| emit_dxil_ir(entry, stage, &sig, module_name));
    match result {
        Ok(ir) => Some(ir),
        Err(e) => {
            cx.diag()
                .struct_error(ErrorCode(e.code), e.message_key)
                .arg("detail", e.detail.clone())
                .span_label(e.span, "in DXIL shader entry")
                .emit();
            None
        }
    }
}

/// 单个着色阶段 body → DXIL DirectX 三元组 LLVM IR 文本(RXS-0158 阶段→着色器类型 +
/// RXS-0159 阶段 I/O 签名)。先裁阶段降级目标(deferred 阶段 → RX6008,RXS-0158 L2);
/// 无 I/O 签名时(compute / 空体 vertex·fragment)校验最小子集(RXS-0157 L1:无 ABI
/// 形参 + 平凡体);带 I/O 签名时(RXS-0159)以类型面签名元数据 + void 入口 stub emit
/// (body 数据流降级 deferred,// STUB(RD-013))。违例 → `DxilCodegenError`
/// (RX6007/RX6008/RX6009)。
fn emit_dxil_ir(
    body: &Body,
    stage: Option<ShaderStage>,
    sig: &StageSig,
    module_name: &str,
) -> Result<String, DxilCodegenError> {
    // RXS-0158 L2:deferred 阶段(mesh/task/RT)先行裁定 → RX6008(优先于子集/签名校验)。
    let target = stage_target(stage, body.span)?;
    if sig.is_empty() {
        // 无 I/O 签名(compute / 空体 vertex·fragment):承 RXS-0157/0158 最小子集——
        // 无 ABI 形参 + 平凡(空)体 → DXIL `void` 入口;违例 → RX6007。
        check_trivial_subset(body)?;
    }
    // else(RXS-0159):带 I/O 签名的 vertex/fragment 入口——签名(SV_* / 插值)经
    // 类型面元数据 emit;入口 body 数据流降级 deferred(// STUB(RD-013),本片仅签名 +
    // void 入口 stub,不降级输入读取/输出写入语句)。
    Ok(render_dxil_module(&body.symbol, module_name, &target, sig))
}

/// RXS-0157/0158 最小子集校验:无 ABI 形参 + 平凡(空/隐式 unit)体 → DXIL void 入口;
/// 违例 → RX6007(子集外构造)。
fn check_trivial_subset(body: &Body) -> Result<(), DxilCodegenError> {
    if body.arg_count != 0 {
        return Err(DxilCodegenError::unsupported(
            body.span,
            "DXIL 最小子集暂不支持带形参的着色阶段入口(I/O 签名属 RXS-0159、View/资源句柄绑定布局推导属 G2.3,FFI ABI 属禁区)",
        ));
    }
    for bb in &body.blocks {
        for st in &bb.stmts {
            // 最小子集仅容忍隐式 unit 返回赋值(`_0 = ()`,空体语义);其余语句
            // (真实计算 / 内存写 / 调用)需 codegen 降级 + 可能绑定布局,属后续分片。
            let StatementKind::Assign(_, Rvalue::Use(Operand::Const(Const::Unit))) = &st.kind
            else {
                return Err(DxilCodegenError::unsupported(
                    st.span,
                    "DXIL 最小子集暂不支持非平凡着色阶段体(本片仅空体入口,语句降级随后续分片)",
                ));
            };
        }
        match bb.terminator.kind {
            TerminatorKind::Goto(_) | TerminatorKind::Return | TerminatorKind::Unreachable => {}
            _ => {
                return Err(DxilCodegenError::unsupported(
                    bb.terminator.span,
                    "DXIL 最小子集暂不支持该控制流终结子(本片仅空体入口)",
                ));
            }
        }
    }
    Ok(())
}

/// DirectX 三元组 LLVM IR 文本(最小空体着色阶段入口,按阶段产对应 shader 类型)。
/// 形态对齐 LLVM DirectX 后端 emit 期望(`shadermodel<sm>-<env>` 三元组 + DXIL 数据
/// 布局 + `hlsl.shader`〔+ compute/mesh/task 的 `hlsl.numthreads`〕入口属性);经
/// patched llc -filetype=obj 产 DXIL 容器、dxc validator 接受(RXS-0158 IR1/IR3)。
/// numthreads 取最小 `1,1,1`。确定性:给定符号名 + 阶段目标输出字节确定。
fn render_dxil_module(
    entry_symbol: &str,
    module_name: &str,
    target: &DxilStageTarget,
    sig: &StageSig,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "; ModuleID = '{module_name}'");
    let _ = writeln!(out, "source_filename = \"{module_name}\"");
    let _ = writeln!(
        out,
        "target datalayout = \"e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64\""
    );
    let _ = writeln!(
        out,
        "target triple = \"dxil-unknown-shadermodel{}-{}\"",
        target.sm, target.triple_env
    );
    out.push('\n');
    let _ = writeln!(out, "define void @{entry_symbol}() #0 {{");
    out.push_str("entry:\n");
    out.push_str("  ret void\n");
    out.push_str("}\n");
    out.push('\n');
    if target.needs_numthreads {
        let _ = writeln!(
            out,
            "attributes #0 = {{ noinline nounwind \"hlsl.numthreads\"=\"1,1,1\" \"hlsl.shader\"=\"{}\" }}",
            target.hlsl_shader
        );
    } else {
        let _ = writeln!(
            out,
            "attributes #0 = {{ noinline nounwind \"hlsl.shader\"=\"{}\" }}",
            target.hlsl_shader
        );
    }
    render_signature_metadata(&mut out, sig);
    out
}

/// RXS-0159:阶段 I/O 签名 → 类型面签名元数据(SV_* 语义名 / 插值限定符,**无寄存器/
/// 偏移/component mask**——二进制布局属 RFC-0003 §4.6 / §9 Q-Builtin 🔒 FFI ABI 禁区,
/// 由 LLVM DirectX 后端 emit)。仅签名非空时 emit(空签名保持既有 cs/vs/ps_noop golden
/// 字节不变)。元素按声明序线性编号;`!N = !{!"<field>", !"<semantic>"}`(Sv 直出 SV 名、
/// Interp 出 `interp:<modifier>`),命名元数据 `!rurix.dxil.sig.in` / `.out` 引用之。
fn render_signature_metadata(out: &mut String, sig: &StageSig) {
    if sig.is_empty() {
        return;
    }
    out.push('\n');
    out.push_str(
        "; RXS-0159 stage I/O signature (type-face: SV semantic names + interpolation modifiers\n\
         ; only; signature element register / offset / component-mask binary layout is RFC-0003\n\
         ; §4.6 / §9 Q-Builtin FFI-ABI 禁区, emitted by the LLVM DirectX backend, not by Rurix).\n",
    );
    let mut idx = 0usize;
    let mut in_ids = Vec::new();
    let mut out_ids = Vec::new();
    let mut nodes = String::new();
    for e in &sig.inputs {
        in_ids.push(format!("!{idx}"));
        let _ = writeln!(nodes, "{}", sig_node(idx, e));
        idx += 1;
    }
    for e in &sig.outputs {
        out_ids.push(format!("!{idx}"));
        let _ = writeln!(nodes, "{}", sig_node(idx, e));
        idx += 1;
    }
    if !in_ids.is_empty() {
        let _ = writeln!(out, "!rurix.dxil.sig.in = !{{{}}}", in_ids.join(", "));
    }
    if !out_ids.is_empty() {
        let _ = writeln!(out, "!rurix.dxil.sig.out = !{{{}}}", out_ids.join(", "));
    }
    out.push_str(&nodes);
}

/// 单签名元素元数据节点文本(`!N = !{!"field", !"semantic"}`)。
fn sig_node(idx: usize, e: &SigElem) -> String {
    let sem = match &e.sem {
        SigSem::Sv(s) => (*s).to_owned(),
        SigSem::Interp(m) => format!("interp:{m}"),
    };
    format!("!{idx} = !{{!\"{}\", !\"{sem}\"}}", e.field)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::query::QueryCtx;
    use crate::span::{Edition, SourceId};

    /// RXS-0157:空体 compute kernel(`kernel fn` 无形参)→ DirectX 三元组 DXIL IR。
    //@ spec: RXS-0157
    #[test]
    fn empty_compute_kernel_emits_dxil_directx_triple() {
        let src = "kernel fn cs_noop() {}\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        assert!(!diag.has_errors(), "空体 compute kernel 应 0 诊断");
        let ir = build_and_emit_dxil(&cx, "cs_noop").expect("应产出 DXIL IR");
        assert!(ir.contains("target triple = \"dxil-unknown-shadermodel6.0-compute\""));
        assert!(ir.contains("\"hlsl.shader\"=\"compute\""));
        assert!(ir.contains("\"hlsl.numthreads\"=\"1,1,1\""));
        assert!(ir.contains("ret void"));
    }

    /// RXS-0157 L2:带 ABI 形参的 kernel(View 形参)→ 子集外 → RX6007。
    //@ spec: RXS-0157
    #[test]
    fn kernel_with_view_param_is_rx6007() {
        let src = "kernel fn k(out: ViewMut<global, f32>) {}\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        let ir = build_and_emit_dxil(&cx, "k");
        assert!(ir.is_none(), "带形参 compute 入口应被拒(子集外)");
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(codes.contains(&6007), "应发 RX6007,实得 {codes:?}");
    }

    #[cfg(feature = "shader-stages")]
    fn emit_ok(src: &str, module: &str) -> String {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        assert!(!diag.has_errors(), "{src:?} 应 0 诊断");
        build_and_emit_dxil(&cx, module).expect("应产出 DXIL IR")
    }

    #[cfg(feature = "shader-stages")]
    fn emit_codes(src: &str, module: &str) -> Vec<u16> {
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        if !diag.has_errors() {
            cx.check_coloring();
        }
        let _ = build_and_emit_dxil(&cx, module);
        diag.emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect()
    }

    /// RXS-0158:vertex 着色阶段(空体)→ DXIL vertex shader 三元组 + hlsl.shader=vertex。
    //@ spec: RXS-0158
    #[cfg(feature = "shader-stages")]
    #[test]
    fn vertex_stage_emits_dxil_vertex_shader() {
        let ir = emit_ok("vertex fn vs_noop() {}\n", "vs_noop");
        assert!(ir.contains("target triple = \"dxil-unknown-shadermodel6.0-vertex\""));
        assert!(ir.contains("\"hlsl.shader\"=\"vertex\""));
        assert!(!ir.contains("hlsl.numthreads"), "vertex 不附 numthreads");
    }

    /// RXS-0158:fragment 着色阶段(空体)→ DXIL pixel shader 三元组 + hlsl.shader=pixel。
    //@ spec: RXS-0158
    #[cfg(feature = "shader-stages")]
    #[test]
    fn fragment_stage_emits_dxil_pixel_shader() {
        let ir = emit_ok("fragment fn ps_noop() {}\n", "ps_noop");
        assert!(ir.contains("target triple = \"dxil-unknown-shadermodel6.0-pixel\""));
        assert!(ir.contains("\"hlsl.shader\"=\"pixel\""));
        assert!(!ir.contains("hlsl.numthreads"), "pixel 不附 numthreads");
    }

    /// RXS-0158:compute 着色阶段(`compute fn`,空体)→ DXIL compute shader(与
    /// `kernel fn` 同产,compute-via-kernel)。
    //@ spec: RXS-0158
    #[cfg(feature = "shader-stages")]
    #[test]
    fn compute_stage_emits_dxil_compute_shader() {
        let ir = emit_ok("compute fn cs_noop() {}\n", "cs_noop");
        assert!(ir.contains("target triple = \"dxil-unknown-shadermodel6.0-compute\""));
        assert!(ir.contains("\"hlsl.shader\"=\"compute\""));
        assert!(ir.contains("\"hlsl.numthreads\"=\"1,1,1\""));
    }

    /// RXS-0158 L2:mesh 着色阶段降级 deferred(RD-012)→ RX6008。
    //@ spec: RXS-0158
    #[cfg(feature = "shader-stages")]
    #[test]
    fn mesh_stage_is_rx6008_deferred() {
        let codes = emit_codes("mesh fn ms_noop() {}\n", "ms_noop");
        assert!(codes.contains(&6008), "mesh 应发 RX6008,实得 {codes:?}");
    }

    /// RXS-0158 L2:task 着色阶段降级 deferred(RD-012)→ RX6008。
    //@ spec: RXS-0158
    #[cfg(feature = "shader-stages")]
    #[test]
    fn task_stage_is_rx6008_deferred() {
        let codes = emit_codes("task fn as_noop() {}\n", "as_noop");
        assert!(codes.contains(&6008), "task 应发 RX6008,实得 {codes:?}");
    }

    /// RXS-0158 L2:RT raygen 着色阶段降级 deferred(RD-012)→ RX6008。
    //@ spec: RXS-0158
    #[cfg(feature = "shader-stages")]
    #[test]
    fn raygen_stage_is_rx6008_deferred() {
        let codes = emit_codes("raygen fn rg_noop() {}\n", "rg_noop");
        assert!(codes.contains(&6008), "raygen 应发 RX6008,实得 {codes:?}");
    }

    /// RXS-0159:vertex I/O → DXIL 签名(SV_VertexID 输入 / SV_Position 输出 + 透视
    /// 插值 varying);类型面签名元数据 emit,无寄存器/偏移。
    //@ spec: RXS-0159
    #[cfg(feature = "shader-stages")]
    #[test]
    fn vertex_io_lowers_to_dxil_signature_semantics() {
        let ir = emit_ok(
            "struct VsIn { #[builtin(vertex_id)] vid: u32 }\n\
             struct VsOut { #[builtin(position)] pos: f32, #[interpolate(perspective)] uv: f32 }\n\
             vertex fn vs_io(inp: VsIn) -> VsOut { VsOut { pos: 0.0, uv: 0.0 } }\n\
             fn main() {}",
            "vs_io",
        );
        assert!(ir.contains("target triple = \"dxil-unknown-shadermodel6.0-vertex\""));
        assert!(ir.contains("!rurix.dxil.sig.in ="), "缺输入签名元数据");
        assert!(ir.contains("!rurix.dxil.sig.out ="), "缺输出签名元数据");
        assert!(ir.contains("!\"SV_VertexID\""), "vertex_id → SV_VertexID");
        assert!(ir.contains("!\"SV_Position\""), "position → SV_Position");
        assert!(
            ir.contains("!\"interp:linear\""),
            "perspective → linear 插值限定符"
        );
        // 类型面边界:签名元数据不含寄存器号/字节偏移等二进制 ABI 布局(只语义名/插值)。
        assert!(!ir.contains("Register"), "不得含寄存器布局列");
        assert!(!ir.contains("byteoffset"), "不得含字节偏移");
    }

    /// RXS-0159:fragment I/O → SV_Position 输入(frag_coord)+ SV_Target 颜色输出。
    //@ spec: RXS-0159
    #[cfg(feature = "shader-stages")]
    #[test]
    fn fragment_io_lowers_sv_target_output() {
        let ir = emit_ok(
            "struct FsIn { #[builtin(frag_coord)] coord: f32, #[interpolate(perspective)] uv: f32 }\n\
             struct FsOut { #[interpolate(perspective)] color: f32 }\n\
             fragment fn fs_io(inp: FsIn) -> FsOut { FsOut { color: 0.0 } }\n\
             fn main() {}",
            "fs_io",
        );
        assert!(ir.contains("target triple = \"dxil-unknown-shadermodel6.0-pixel\""));
        assert!(ir.contains("!\"SV_Position\""), "frag_coord → SV_Position");
        assert!(
            ir.contains("!\"SV_Target\""),
            "fragment 输出 varying → SV_Target"
        );
    }

    /// RXS-0159:不可映射内建变量(thread_id 在 vertex 入口无对应 DXIL SV)→ RX6009。
    //@ spec: RXS-0159
    #[cfg(feature = "shader-stages")]
    #[test]
    fn unmappable_builtin_is_rx6009() {
        let codes = emit_codes(
            "struct VsIn { #[builtin(thread_id)] tid: u32 }\n\
             vertex fn vs_bad(inp: VsIn) {}\n\
             fn main() {}",
            "vs_bad",
        );
        assert!(
            codes.contains(&6009),
            "不可映射 builtin 应发 RX6009,实得 {codes:?}"
        );
    }

    /// RXS-0159:整数 varying 非 flat 插值(DXIL 要求 nointerpolation)→ RX6009。
    //@ spec: RXS-0159
    #[cfg(feature = "shader-stages")]
    #[test]
    fn integer_varying_non_flat_is_rx6009() {
        let codes = emit_codes(
            "struct VsOut { #[builtin(position)] pos: f32, #[interpolate(perspective)] id: u32 }\n\
             vertex fn vs_bad() -> VsOut { VsOut { pos: 0.0, id: 0 } }\n\
             fn main() {}",
            "vs_bad",
        );
        assert!(
            codes.contains(&6009),
            "整数非 flat 插值应发 RX6009,实得 {codes:?}"
        );
    }

    /// RXS-0159:整数 varying flat(nointerpolation)合法 → 产签名,无诊断。
    //@ spec: RXS-0159
    #[cfg(feature = "shader-stages")]
    #[test]
    fn integer_varying_flat_is_ok() {
        let ir = emit_ok(
            "struct VsOut { #[builtin(position)] pos: f32, #[interpolate(flat)] id: u32 }\n\
             vertex fn vs_ok() -> VsOut { VsOut { pos: 0.0, id: 0 } }\n\
             fn main() {}",
            "vs_ok",
        );
        assert!(
            ir.contains("!\"interp:nointerpolation\""),
            "flat → nointerpolation"
        );
    }
}
