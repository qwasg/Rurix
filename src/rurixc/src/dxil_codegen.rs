//! device MIR → DXIL DirectX 三元组 LLVM IR 文本(G2.2 PR-C2 分片1,RXS-0157;
//! RFC-0003 §4.1/§4.2,D-131=A)。
//!
//! 本模块 gate 于 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate);未启用时整模块
//! 不编入 rurixc,PTX 路径(D-207)不受影响。target 分发在 MIR 之后分叉:DXIL 后端与
//! NVPTX 后端(`device_codegen`)并列、各自从 MIR 独立降级,不共享后端 lowering
//! (RFC-0003 §4.5)。
//!
//! **最小 compute 子集(分片1 + GRX-009 segment 3a 增量)**:支持 compute 着色入口
//! (`kernel fn`,RXS-0153 compute-via-kernel 着色)的空入口与最小 body lowering
//! 子集:简单 `let`/赋值、`let mut` mutable scalar local、`usize`/`f32` 常量与二元
//! 算术、整数 `%`(`srem i64`)、简单比较 + 标量 select(无语句 if 表达式)、
//! `ThreadCtx<1>.global_id()`、no-else statement if(语句位 + 函数体 tail 位,then
//! block 复用语句 lowering)、最小 `while cond { stmts }`、`View<global, f32>`
//! load、`ViewMut<global, f32>` store、i64 动态资源索引。资源常量索引仅支持 0,
//! 非 0 常量索引缺少资源边界 evidence → strict `RX6007`。f32 `%` 取模/else 与
//! else-if/break/continue/dynamic dispatch shape 等子集外构造继续 strict `RX6007`,
//! 禁止静默降级为 entry shell。
//!
//! 下游(IR → patched llc -filetype=obj → DXIL 容器 → dxc validator)见
//! [`crate::toolchain::ir_to_dxil`];golden 取文本反汇编经 validator 验证(RFC-0003
//! §9 Q-Golden)。**本片不碰** 🔒 Godot resource mapping/纹理内存模型映射(06 §4.2)/
//! FFI ABI 二进制布局(RFC-0003 §4.6)。

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::ast::{BinOp, FnColor, LitKind, LitSuffix, ShaderStage};
use crate::binding_layout;
use crate::diag::{DiagCtxt, ErrorCode};
use crate::dxil_sig_gate::signature_gate;
use crate::dxil_spirv::{self, DxilError};
use crate::mir::{Body, IoDir, IoSigElem, IoSigKind, ResourceBinding};
use crate::query::QueryCtx;
use crate::span::Span;
use crate::toolchain::{self, DxilSignatures};

/// DXIL codegen 失败(默认 RX6007:目标不可用 / 子集外构造 / 降级失败,RXS-0157
/// L1~L3;数学 intrinsic 首期覆盖外取 RX6006——RXS-0081 既有语义精确适用,RXS-0184)。
#[derive(Debug, Clone)]
pub struct DxilCodegenError {
    pub span: Span,
    pub detail: String,
    /// 诊断码(默认 6007;RXS-0184 首期覆盖外数学 intrinsic → 6006)。
    pub code: u16,
    /// 诊断 message-key(与 `code` 配对;默认 `codegen.dxil_unsupported`)。
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

    /// 数学 intrinsic 首期覆盖外(RXS-0184;RX6006 = RXS-0081「不支持的元素类型
    /// 组合 / 超覆盖」既有语义,零新码)。
    fn math_unsupported(span: Span, detail: impl Into<String>) -> Self {
        DxilCodegenError {
            span,
            detail: detail.into(),
            code: 6006,
            message_key: "codegen.device_math_unsupported",
        }
    }
}

/// compute kernel 形参推导出的离线绑定布局(GRX-009 segment 3a.1;最小支持)。
///
/// 从受支持的 compute 入口形参(`View`/`ViewMut` 资源视图 + `usize`/`f32` 标量 root
/// constant + `ThreadCtx` 线程内建)经 [`binding_layout`] safe 推导得到资源绑定 +
/// root signature 形态,供离线 artifact(RTS0 root signature 字节 + descriptor
/// layout JSON)落盘。body lowering 仅覆盖 RD-013 slice 1 straight-line 子集。
#[derive(Debug, Clone)]
pub struct ComputeLayout {
    /// 声明序资源绑定(`View`→SRV / `ViewMut`→UAV)。
    pub resources: Vec<ResourceBinding>,
    /// root signature 形态(经 `binding_layout::infer_root_signature`)。
    pub root_signature: binding_layout::RootSignature,
    /// 标量 root constant layout(按 compute 入口声明序)。
    pub root_constants: Vec<binding_layout::RootConstant>,
}

/// DXIL compute 入口的离线 artifact 三元组(GRX-009 segment 3a.1)。
///
/// - `ir`:DirectX 三元组 LLVM IR 文本(空入口或 slice 1 straight-line body)。
/// - `root_signature`:RTS0 容器字节(`binding_layout::serialize_rts0`)。
/// - `descriptor_layout_json`:descriptor layout 意图的确定性 JSON(host/safe)。
#[derive(Debug, Clone)]
pub struct DxilComputeArtifacts {
    /// DirectX 三元组 LLVM IR 文本。
    pub ir: String,
    /// RTS0 root signature 容器字节。
    pub root_signature: Vec<u8>,
    /// descriptor layout 意图 JSON(确定性序列化)。
    pub descriptor_layout_json: String,
}

/// 驱动 / 测试入口:构建 device MIR(`kernel fn` 为根)+ DXIL 最小 compute codegen。
/// 无 kernel → `None`(无 device 产物);子集外 / 降级失败 → 经 `cx.diag()` 落
/// `RX6007` 结构化诊断并返回 `None`;成功 → `Some(DirectX 三元组 LLVM IR 文本)`。
/// patched llc → DXIL 容器 + dxc validator 由驱动在产 IR 后另行实施(RXS-0157 IR2)。
///
/// GRX-009 segment 3a.1:受支持的 compute 入口形参(View/ViewMut/标量/ThreadCtx)不再
/// 硬拒,经 [`derive_compute_bindings`] 进入绑定布局推导(strict-only:未知形参类型仍
/// RX6007);body 仅支持 RD-013 slice 1,子集外 strict reject,不能降级为 entry-shell
/// compile success。
pub fn build_and_emit_dxil(cx: &QueryCtx<'_>, module_name: &str) -> Option<String> {
    build_dxil_compute(cx, module_name).map(|b| b.ir)
}

/// 驱动入口(GRX-009 segment 3a.1):构建 device MIR + DXIL compute codegen,产出
/// 离线 artifact 三元组(DXIL IR / RTS0 root signature / descriptor layout JSON)。
/// 无 kernel → `None`;子集外(未知形参类型 / unsupported body / 绑定推导失败)→ 落
/// `RX6007`(或 binding_layout 的 6xxx)并返回 `None`。strict-only,无 fallback。
pub fn build_and_emit_dxil_artifacts(
    cx: &QueryCtx<'_>,
    module_name: &str,
) -> Option<DxilComputeArtifacts> {
    let built = build_dxil_compute(cx, module_name)?;
    let root_signature = binding_layout::serialize_rts0(&built.layout.root_signature);
    let descriptor_layout_json = render_descriptor_layout_json(module_name, &built.layout);
    Some(DxilComputeArtifacts {
        ir: built.ir,
        root_signature,
        descriptor_layout_json,
    })
}

struct BuiltDxilCompute {
    ir: String,
    layout: ComputeLayout,
}

fn build_dxil_compute(cx: &QueryCtx<'_>, module_name: &str) -> Option<BuiltDxilCompute> {
    let bodies = cx.device_mir_crate();
    if bodies.is_empty() {
        return None;
    }
    // device MIR 构建已报错 → 不级联 codegen(防一错多报,对齐 device_codegen)。
    if cx.diag().has_errors() {
        return None;
    }
    // compute 入口 = kernel 着色 body(RXS-0153 compute-via-kernel);取首个为最小入口。
    let entry = bodies.iter().find(|b| b.color == FnColor::Kernel)?;
    let krate = cx.hir_crate();
    let fn_name = krate.item(entry.def).name.clone();
    let Some(kernel_fn) = find_kernel_fn(&cx.ast().items, &fn_name) else {
        cx.diag()
            .struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
            .arg(
                "detail",
                "无法在 AST 定位 compute kernel 声明(内部不一致)".to_owned(),
            )
            .span_label(entry.span, "in DXIL compute entry")
            .emit();
        return None;
    };
    // 从 AST 形参推导绑定布局(strict-only:未知形参类型 → RX6007)。
    let layout = match derive_compute_bindings(cx, entry) {
        Ok(layout) => layout,
        Err(e) => {
            emit_dxil_codegen_error(cx, &e);
            return None;
        }
    };
    let lowered_body = match lower_compute_body_slice1(cx, entry, kernel_fn) {
        Ok(lowered_body) => lowered_body,
        Err(e) => {
            emit_dxil_codegen_error(cx, &e);
            return None;
        }
    };
    let ir = match emit_dxil_ir_with_body(entry, module_name, &lowered_body) {
        Ok(ir) => ir,
        Err(e) => {
            emit_dxil_codegen_error(cx, &e);
            return None;
        }
    };
    Some(BuiltDxilCompute { ir, layout })
}

/// 结构化 emit [`DxilCodegenError`](按错误携带的码 / message-key;默认 RX6007
/// `codegen.dxil_unsupported`,RXS-0184 首期覆盖外数学 intrinsic → RX6006
/// `codegen.device_math_unsupported`)。
fn emit_dxil_codegen_error(cx: &QueryCtx<'_>, e: &DxilCodegenError) {
    cx.diag()
        .struct_error(ErrorCode(e.code), e.message_key)
        .arg("detail", e.detail.clone())
        .span_label(e.span, "in DXIL compute entry")
        .emit();
}

/// GRX-009 segment 3a.1:从 compute kernel 的 **AST 形参**推导离线绑定布局
/// (strict-only)。compute 入口的 `body.resources` 恒空(仅图形阶段填充,见
/// mir_build),故本函数直接读 AST 形参类型分类:
/// - `View<_, T>`(只读视图)→ SRV(`StructuredBuffer{read_only:true}`)。
/// - `ViewMut<_, T>`(可写视图)→ UAV(`StructuredBuffer{read_only:false}`)。
/// - `usize`/`f32`(标量)→ root constant 计数(不入 descriptor)。
/// - `ThreadCtx<N>`→ 线程内建(跳过,不产资源 / root constant)。
/// - 其它任何类型 → [`DxilCodegenError`](上层映射 RX6007,无 fallback)。
///
/// 资源绑定经 [`binding_layout::infer_root_signature`] 推导 root signature(其内部
/// 跑 register/space 分配 + 冲突门 + 64-DWORD 上限门,失败继续 strict-only 报错)。
fn derive_compute_bindings(
    cx: &QueryCtx<'_>,
    entry: &Body,
) -> Result<ComputeLayout, DxilCodegenError> {
    // 经 HIR item 名 + AST 定位该 kernel 的形参声明(compute 入口 stage 恒 None)。
    let krate = cx.hir_crate();
    let fn_name = krate.item(entry.def).name.clone();
    let ast = cx.ast();
    let Some(f) = find_kernel_fn(&ast.items, &fn_name) else {
        // 前端已建 MIR,理应可在 AST 定位;定位失败 = 内部不一致 → strict-only 停手。
        return Err(DxilCodegenError::unsupported(
            entry.span,
            "无法在 AST 定位 compute kernel 形参声明(内部不一致)",
        ));
    };
    let params = collect_compute_param_sigs(cx, f, ComputeParamMode::Entry)?;
    let resources = compute_resource_bindings(&params);
    let root_constant_params = params
        .iter()
        .filter_map(|p| match p {
            ComputeParamSig::Scalar { name, ty, .. } => Some((name.clone(), *ty)),
            _ => None,
        })
        .collect();

    let root_constants = binding_layout::pack_root_constants(root_constant_params);
    let mut root_signature = binding_layout::infer_root_signature(&resources).map_err(|e| {
        DxilCodegenError::unsupported(entry.span, format!("绑定布局推导失败(strict-only): {e}"))
    })?;
    if !root_constants.is_empty() {
        root_signature.parameters.insert(
            0,
            binding_layout::RootParameter::RootConstants {
                constants: root_constants.clone(),
            },
        );
        let dwords = binding_layout::root_signature_cost_dwords(&root_signature);
        if dwords > binding_layout::ROOT_SIGNATURE_DWORD_LIMIT {
            return Err(DxilCodegenError::unsupported(
                entry.span,
                format!(
                    "绑定布局推导失败(strict-only): root signature 推导超上限: {dwords} DWORD > {} DWORD",
                    binding_layout::ROOT_SIGNATURE_DWORD_LIMIT
                ),
            ));
        }
    }

    Ok(ComputeLayout {
        resources,
        root_signature,
        root_constants,
    })
}

#[derive(Debug, Clone, Copy)]
enum ComputeParamMode {
    Entry,
    Body,
}

#[derive(Debug, Clone)]
enum ComputeParamSig {
    Resource {
        name: String,
        res: crate::mir::MirResourceType,
        kind: ComputeParamKind,
        span: Span,
    },
    Scalar {
        name: String,
        ty: binding_layout::RootConstantType,
        lowered_ty: LoweredScalarTy,
        span: Span,
    },
    ThreadCtx {
        name: String,
        span: Span,
    },
    IgnoredScalar {
        name: String,
        span: Span,
    },
}

fn collect_compute_param_sigs(
    cx: &QueryCtx<'_>,
    f: &crate::ast::FnItem,
    mode: ComputeParamMode,
) -> Result<Vec<ComputeParamSig>, DxilCodegenError> {
    use crate::ast::{ParamKind, TyKind};

    let mut out = Vec::with_capacity(f.params.len());
    for p in &f.params {
        let ParamKind::Typed { pat, ty } = &p.kind else {
            let detail = match mode {
                ComputeParamMode::Entry => "DXIL compute 入口不支持 self 形参",
                ComputeParamMode::Body => "DXIL compute body lowering slice 1 不支持 self 形参",
            };
            return Err(DxilCodegenError::unsupported(p.span, detail));
        };
        let Some(name) = ast_pat_binding_name(pat) else {
            let detail = match mode {
                ComputeParamMode::Entry => "DXIL compute 入口仅支持简单绑定形参",
                ComputeParamMode::Body => "DXIL compute body lowering slice 1 仅支持简单绑定形参",
            };
            return Err(DxilCodegenError::unsupported(pat.span, detail));
        };
        let head = ast_ty_head_name(ty).unwrap_or("");
        let sig = match head {
            "View" => {
                if matches!(mode, ComputeParamMode::Body) {
                    require_texture_or_view_global_elem(ty)?;
                }
                // MR-0006(RXS-0181):元素类型扩到 f32|u32|i32(Body 模式已经
                // require 校验;Entry 模式沿既有容忍——布局推导元素类型无关,
                // 子集外元素在 body lowering 收口 strict RX6007)。
                ComputeParamSig::Resource {
                    name,
                    res: crate::mir::MirResourceType::StructuredBuffer { read_only: true },
                    kind: match view_elem_head(ty) {
                        Some("u32") => ComputeParamKind::ViewU32,
                        Some("i32") => ComputeParamKind::ViewI32,
                        _ => ComputeParamKind::ViewF32,
                    },
                    span: ty.span,
                }
            }
            "ViewMut" => {
                if matches!(mode, ComputeParamMode::Body) {
                    require_texture_or_view_global_elem(ty)?;
                }
                ComputeParamSig::Resource {
                    name,
                    res: crate::mir::MirResourceType::StructuredBuffer { read_only: false },
                    kind: match view_elem_head(ty) {
                        Some("u32") => ComputeParamKind::ViewMutU32,
                        Some("i32") => ComputeParamKind::ViewMutI32,
                        _ => ComputeParamKind::ViewMutF32,
                    },
                    span: ty.span,
                }
            }
            "Texture2D" => {
                if matches!(mode, ComputeParamMode::Body) {
                    require_texture_or_view_global_elem(ty)?;
                }
                ComputeParamSig::Resource {
                    name,
                    res: crate::mir::MirResourceType::Texture2D(crate::hir::PrimTy::F32),
                    kind: ComputeParamKind::Texture2DF32,
                    span: ty.span,
                }
            }
            "RWTexture2D" => {
                if matches!(mode, ComputeParamMode::Body) {
                    require_texture_or_view_global_elem(ty)?;
                }
                ComputeParamSig::Resource {
                    name,
                    res: crate::mir::MirResourceType::RWTexture2D(crate::hir::PrimTy::F32),
                    kind: ComputeParamKind::RWTexture2DF32,
                    span: ty.span,
                }
            }
            "ThreadCtx" => {
                if matches!(mode, ComputeParamMode::Body) {
                    require_threadctx_1d(cx, ty)?;
                }
                ComputeParamSig::ThreadCtx {
                    name,
                    span: ty.span,
                }
            }
            "f32" => ComputeParamSig::Scalar {
                name,
                ty: binding_layout::RootConstantType::F32,
                lowered_ty: LoweredScalarTy::F32,
                span: ty.span,
            },
            "usize" | "u32" | "i32" | "u64" | "i64" | "u16" | "i16" | "u8" | "i8" => {
                ComputeParamSig::Scalar {
                    name,
                    ty: binding_layout::RootConstantType::I64,
                    lowered_ty: LoweredScalarTy::I64,
                    span: ty.span,
                }
            }
            "bool" => ComputeParamSig::Scalar {
                name,
                ty: binding_layout::RootConstantType::Bool,
                lowered_ty: LoweredScalarTy::Bool,
                span: ty.span,
            },
            "f64" if matches!(mode, ComputeParamMode::Body) => ComputeParamSig::IgnoredScalar {
                name,
                span: ty.span,
            },
            _ if matches!(mode, ComputeParamMode::Body) => ComputeParamSig::IgnoredScalar {
                name,
                span: ty.span,
            },
            other => {
                let detail = if other.is_empty() {
                    "DXIL compute 入口含不支持的形参类型(strict-only,仅支持 View/ViewMut/Texture2D/RWTexture2D/标量/ThreadCtx)".to_owned()
                } else {
                    format!(
                        "DXIL compute 入口不支持形参类型 `{other}`(strict-only,仅支持 View/ViewMut/Texture2D/RWTexture2D/标量/ThreadCtx;其余绑定布局属禁区)"
                    )
                };
                let ty_kind_span = match &ty.kind {
                    TyKind::Path(pth) => pth.span,
                    _ => ty.span,
                };
                return Err(DxilCodegenError::unsupported(ty_kind_span, detail));
            }
        };
        out.push(sig);
    }
    Ok(out)
}

fn compute_resource_bindings(params: &[ComputeParamSig]) -> Vec<ResourceBinding> {
    params
        .iter()
        .filter_map(|p| match p {
            ComputeParamSig::Resource { name, res, .. } => Some(ResourceBinding {
                name: name.clone(),
                res: *res,
                count: crate::mir::ResourceCount::One,
            }),
            _ => None,
        })
        .collect()
}

fn lowered_resource_bindings(resources: &[LoweredComputeResource]) -> Vec<ResourceBinding> {
    resources
        .iter()
        .map(|r| ResourceBinding {
            name: r.name.clone(),
            res: r.res,
            count: crate::mir::ResourceCount::One,
        })
        .collect()
}

/// 按名定位顶层 / 嵌套 mod 内的 `kernel fn`(compute 入口;`FnColor::Kernel`)。
fn find_kernel_fn<'a>(items: &'a [crate::ast::Item], name: &str) -> Option<&'a crate::ast::FnItem> {
    use crate::ast::{FnColor, ItemKind};
    for it in items {
        match &it.kind {
            ItemKind::Fn(f) if f.color == FnColor::Kernel && f.name.name == name => {
                return Some(f);
            }
            ItemKind::Mod(m) => {
                if let Some(found) = find_kernel_fn(&m.items, name) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// AST 类型头名(剥 `&T`/`*T`/`(T)`;取路径末段 ident)。
fn ast_ty_head_name(ty: &crate::ast::Ty) -> Option<&str> {
    use crate::ast::TyKind;
    match &ty.kind {
        TyKind::Path(p) => p.segments.last().map(|s| s.ident.name.as_str()),
        TyKind::Paren(inner) | TyKind::Ref { inner, .. } | TyKind::RawPtr { inner, .. } => {
            ast_ty_head_name(inner)
        }
        _ => None,
    }
}

/// 简单绑定形参名(`name: Ty` → "name");非简单绑定 → None。
fn ast_pat_binding_name(pat: &crate::ast::Pat) -> Option<String> {
    match &pat.kind {
        crate::ast::PatKind::Binding { name, .. } => Some(name.name.clone()),
        _ => None,
    }
}

fn ast_pat_binding_mutability(pat: &crate::ast::Pat) -> Option<bool> {
    match &pat.kind {
        crate::ast::PatKind::Binding { mutable, .. } => Some(*mutable),
        _ => None,
    }
}

fn ast_scalar_ty(ty: &crate::ast::Ty) -> Option<LoweredScalarTy> {
    match ast_ty_head_name(ty)? {
        "f32" => Some(LoweredScalarTy::F32),
        // MR-0006(RXS-0181/0182):u32/i32 local 注解定型为 32 位整型(位运算
        // 工作集与整型视图元素同域);usize/i64 族维持 slice 3a I64。
        "u32" => Some(LoweredScalarTy::U32),
        "i32" => Some(LoweredScalarTy::I32),
        "usize" | "u64" | "i64" | "u16" | "i16" | "u8" | "i8" => Some(LoweredScalarTy::I64),
        "bool" => Some(LoweredScalarTy::Bool),
        _ => None,
    }
}

/// descriptor layout 意图的确定性 JSON 序列化(host/safe;GRX-009 离线 artifact)。
/// 手写 JSON(无 serde 依赖),字段序固定 → 相同输入字节确定。**非 stable ABI**:
/// register/space 数值为实现确定、gate 后产物,不冻结为语言保证。
fn render_descriptor_layout_json(module_name: &str, layout: &ComputeLayout) -> String {
    use std::fmt::Write as _;
    let assignments =
        binding_layout::infer_register_assignments(&layout.resources).unwrap_or_default();
    let mut out = String::new();
    out.push_str("{\n");
    let _ = writeln!(out, "  \"module\": \"{}\",", json_escape(module_name));
    let _ = writeln!(
        out,
        "  \"root_constants\": {},",
        layout.root_constants.len()
    );
    out.push_str("  \"root_constant_layout\": [");
    for (i, c) in layout.root_constants.iter().enumerate() {
        if i == 0 {
            out.push('\n');
        }
        let _ = write!(
            out,
            "    {{ \"name\": \"{}\", \"type\": \"{}\", \"order\": {}, \"root_parameter_index\": 0, \"dword_offset\": {}, \"dword_size\": {} }}",
            json_escape(&c.name),
            c.ty.as_str(),
            c.order,
            c.dword_offset,
            c.dword_size
        );
        if i + 1 < layout.root_constants.len() {
            out.push(',');
        }
        out.push('\n');
    }
    if layout.root_constants.is_empty() {
        out.push_str("],\n");
    } else {
        out.push_str("  ],\n");
    }
    out.push_str("  \"resources\": [");
    // GRX-009:`assignments` 与 `layout.resources` 同序(均按声明序),zip 取对应 `res`
    // 用于 `binding_kind` 字段。空 resources 走 `]` 短路。
    for (i, (rb, a)) in layout.resources.iter().zip(assignments.iter()).enumerate() {
        if i == 0 {
            out.push('\n');
        }
        let _ = write!(
            out,
            "    {{ \"name\": \"{}\", \"class\": \"{}\", \"register\": {}, \"space\": {}, \"count\": {}, \"binding_kind\": \"{}\" }}",
            json_escape(&a.name),
            a.axis_prefix(),
            a.register,
            a.space,
            a.span,
            binding_kind_str(&rb.res),
        );
        if i + 1 < assignments.len() {
            out.push(',');
        }
        out.push('\n');
    }
    if assignments.is_empty() {
        out.push_str("],\n");
    } else {
        out.push_str("  ],\n");
    }
    let _ = writeln!(
        out,
        "  \"root_signature_parameters\": {}",
        layout.root_signature.parameters.len()
    );
    out.push_str("}\n");
    out
}

/// GRX-009 descriptor layout `binding_kind` 字段串。spec ADDDED Requirements
/// 「Descriptor Layout binding_kind Field」固定取值,与 bridge `runtime_resource_binding_kind`
/// 映射对齐(`RXGD_RESOURCE_TEXTURE → "texture2d"`、`RXGD_RESOURCE_BUFFER → "raw_buffer_view"`)。
fn binding_kind_str(res: &crate::mir::MirResourceType) -> &'static str {
    match res {
        crate::mir::MirResourceType::Texture2D(_) => "texture2d",
        crate::mir::MirResourceType::RWTexture2D(_) => "rwtexture2d",
        // View/ViewMut → StructuredBuffer(SRV/UAV):均映射为 `raw_buffer_view`
        // (bridge `runtime_resource_binding_kind` 按 RXGD_RESOURCE_BUFFER 取此值)。
        crate::mir::MirResourceType::StructuredBuffer { .. } => "raw_buffer_view",
        crate::mir::MirResourceType::Sampler => "sampler",
        crate::mir::MirResourceType::ConstantBuffer => "constant_buffer",
    }
}

/// JSON 字符串转义:host/safe 离线 artifact 序列化专用(GRX-009)。
///
/// 仅转义 JSON 必需的 5 个字符(`"` `\` `\n` `\r` `\t`),其余字节(含 UTF-8 多字节
/// 序列)原样批量复制。`NEEDS_ESCAPE` 查找表把 per-byte 判定降到 O(1),并为未来
/// SIMD 向量化扫锚;字节级遍历避免 `chars()` 的 UTF-8 解码开销。
///
/// 切片点只在 ASCII 单字节字符处出现 → 必然落在 UTF-8 char 边界,因此 `s.get()`
/// 返回 `Some` 仅需 O(1) char-boundary 检查,无需完整 UTF-8 再校验。`expect` 不变式:
/// 切片起点/终点始终紧跟一个 ASCII 字节之后。
#[inline]
fn json_escape(s: &str) -> String {
    const NEEDS_ESCAPE: [bool; 256] = {
        let mut t = [false; 256];
        t[b'"' as usize] = true;
        t[b'\\' as usize] = true;
        t[b'\n' as usize] = true;
        t[b'\r' as usize] = true;
        t[b'\t' as usize] = true;
        t
    };

    let bytes = s.as_bytes();
    // 最坏情况:全为需转义字符(每个 1→2 字节),2× 上界避免再分配。
    let mut out = String::with_capacity(bytes.len() * 2);

    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if NEEDS_ESCAPE[b as usize] {
            if start < i {
                out.push_str(s.get(start..i).expect("ASCII 切片点必在 UTF-8 char 边界"));
            }
            out.push_str(match b {
                b'"' => "\\\"",
                b'\\' => "\\\\",
                b'\n' => "\\n",
                b'\r' => "\\r",
                b'\t' => "\\t",
                _ => unreachable!(),
            });
            start = i + 1;
        }
        i += 1;
    }
    if start < bytes.len() {
        out.push_str(s.get(start..).expect("ASCII 切片点必在 UTF-8 char 边界"));
    }
    out
}

#[derive(Debug, Clone, Default)]
struct LoweredComputeBody {
    resources: Vec<LoweredComputeResource>,
    scalar_params: Vec<LoweredScalarParam>,
    ops: Vec<LoweredComputeOp>,
}

#[derive(Debug, Clone)]
struct LoweredComputeResource {
    name: String,
    /// 资源 MIR 类型(GRX-009:区分 raw-buffer StructuredBuffer 与 Texture2D/RWTexture2D
    /// 三种;mutable 由 `res.class() == ResourceClass::Uav` 派生)。
    res: crate::mir::MirResourceType,
    /// 元素标量类型(MR-0006/RXS-0181:raw-buffer 视图 f32|u32|i32,按元素类型
    /// 选 rawbuffer intrinsic 重载;纹理路径恒 F32)。
    elem: LoweredScalarTy,
}

#[derive(Debug, Clone)]
struct LoweredScalarParam {
    name: String,
    ty: LoweredScalarTy,
}

#[derive(Debug, Clone)]
enum LoweredLocal {
    Immutable(LoweredValue),
    Mutable { slot: String, ty: LoweredScalarTy },
}

#[derive(Debug, Clone)]
enum LoweredComputeOp {
    Load {
        dst: String,
        resource: String,
        index: LoweredResourceIndex,
        /// GRX-009 段 stage 3:纹理 2D texel 坐标 `(x, y)`(方法调用 `tex.load(x,y)`
        /// 形)。`Some` → 纹理 load 用 `(x,y)`;`None` → 1D `index` 派生 `(idx,0)`
        /// (`tex[idx]` 形 / raw-buffer 路径)。raw-buffer load 恒 `None`。
        tex_coords_2d: Option<(LoweredValue, LoweredValue)>,
    },
    Store {
        /// GEP 结果 SSA 名(经 [`ComputeLowerCx::temp`] 分配,渲染为 `%{ptr}.ptr`;
        /// 重复 store 同一资源/索引时保持定义名唯一)。
        ptr: String,
        resource: String,
        index: LoweredResourceIndex,
        value: LoweredValue,
        /// GRX-009 段 stage 3:纹理 2D texel 坐标 `(x, y)`(方法调用
        /// `dst.store(x,y,v)` 形)。语义同 `Load::tex_coords_2d`。
        tex_coords_2d: Option<(LoweredValue, LoweredValue)>,
    },
    LocalAlloca {
        slot: String,
        ty: LoweredScalarTy,
    },
    LocalLoad {
        dst: String,
        slot: String,
        ty: LoweredScalarTy,
    },
    LocalStore {
        slot: String,
        ty: LoweredScalarTy,
        value: LoweredValue,
    },
    Binary {
        dst: String,
        op: BinOp,
        lhs: LoweredValue,
        rhs: LoweredValue,
    },
    ScalarParam {
        dst: String,
        name: String,
        ty: LoweredScalarTy,
    },
    ThreadGlobalId {
        dst: String,
    },
    Compare {
        dst: String,
        op: BinOp,
        lhs: LoweredValue,
        rhs: LoweredValue,
    },
    Select {
        dst: String,
        cond: LoweredValue,
        then_value: LoweredValue,
        else_value: LoweredValue,
    },
    /// 位扫描/位计数 intrinsic(MR-0006,RXS-0183):`u32` 值 → `u32`。
    /// DXIL 侧拼写(probe 实测 2026-07-12,pinned llc ×8 字节稳定 + dxv 接受):
    /// find_lsb → `@llvm.dx.firstbitlow`(dx.op FirstbitLo(32),零输入 -1 = HLSL 形);
    /// find_msb → `@llvm.dx.firstbituhigh`(FirstbitHi(33))+ dxc 同款正规化
    /// `select(raw == -1, -1, 31 - raw)`(O-7 golden 锚,LSB=0 位序 + 零输入 HLSL 形);
    /// popcount → `@llvm.ctpop.i32`(Countbits(31))。
    /// `llvm.cttz/ctlz.i32` 被 pinned llc DXILBitcodeWriter 拒(「Unsupported
    /// intrinsic … for DXIL lowering」),不采用。
    BitScan {
        dst: String,
        op: crate::hir::DeviceBitFn,
        value: LoweredValue,
    },
    /// device 数学 intrinsic f32 首期四函数(MR-0007,RXS-0184):
    /// sqrt → `@llvm.sqrt.f32`(dx.op.unary Sqrt(24))/ rsqrt → `@llvm.dx.rsqrt.f32`
    /// (Rsqrt(25),上游直达 intrinsic,无须 1/sqrt 组合)/ sin → `@llvm.sin.f32`
    /// (Sin(13))/ cos → `@llvm.cos.f32`(Cos(12))。probe 实测 2026-07-12:四者
    /// pinned llc emit ×8 字节稳定 + dxv `Validation succeeded`。
    MathUnary {
        dst: String,
        op: crate::hir::DeviceMathFn,
        value: LoweredValue,
    },
    /// 最小 no-else statement if(GRX-009 segment 3a):`br i1 cond` 分叉到
    /// `if.then.{id}` / `if.end.{id}` 两 label(`id` 经 [`ComputeLowerCx::block_id`]
    /// 单调分配,deterministic 且全 body 唯一)。then_ops 复用同一 temp 计数器,
    /// SSA 名跨块唯一;else / else-if 仍 strict RX6007。
    If {
        id: u32,
        cond: LoweredValue,
        then_ops: Vec<LoweredComputeOp>,
    },
    While {
        id: u32,
        cond_ops: Vec<LoweredComputeOp>,
        cond: LoweredValue,
        body_ops: Vec<LoweredComputeOp>,
    },
}

#[derive(Debug, Clone)]
enum LoweredResourceIndex {
    ConstZero,
    Dynamic(LoweredValue),
}

#[derive(Debug, Clone)]
struct LoweredValue {
    ty: LoweredScalarTy,
    repr: LoweredValueRepr,
}

#[derive(Debug, Clone)]
enum LoweredValueRepr {
    Const(String),
    Temp(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoweredScalarTy {
    F32,
    /// 整型 raw buffer 视图元素 / 位运算工作集(MR-0006,RXS-0181;LLVM IR 层
    /// `i32`,无符号语义由运算指令侧承载:udiv/urem/lshr/无符号 icmp)。
    U32,
    /// 有符号 32 位整型视图元素(MR-0006,RXS-0181;`i32`,ashr/sdiv/srem)。
    I32,
    I64,
    Bool,
}

impl LoweredScalarTy {
    /// 32 位整型元素(u32/i32;raw buffer i32 重载 + 位运算/移位掩码域)。
    fn is_int32(self) -> bool {
        matches!(self, LoweredScalarTy::U32 | LoweredScalarTy::I32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComputeParamKind {
    ViewF32,
    ViewMutF32,
    /// MR-0006(RXS-0181):`View<global, u32>`(SRV,i32 rawbuffer 重载)。
    ViewU32,
    /// MR-0006(RXS-0181):`ViewMut<global, u32>`(UAV,i32 rawbuffer 重载)。
    ViewMutU32,
    /// MR-0006(RXS-0181):`View<global, i32>`(SRV)。
    ViewI32,
    /// MR-0006(RXS-0181):`ViewMut<global, i32>`(UAV)。
    ViewMutI32,
    /// GRX-009:compute-kernel SRV 纹理(`Texture2D<f32>` → texelLoad 路径)。
    Texture2DF32,
    /// GRX-009:compute-kernel UAV 纹理(`RWTexture2D<f32>` → texelStore 路径)。
    RWTexture2DF32,
    Scalar,
    ThreadCtx,
}

impl ComputeParamKind {
    /// 只读视图(load 源)的元素类型;非只读视图 → None。
    fn view_elem(self) -> Option<LoweredScalarTy> {
        match self {
            ComputeParamKind::ViewF32 => Some(LoweredScalarTy::F32),
            ComputeParamKind::ViewU32 => Some(LoweredScalarTy::U32),
            ComputeParamKind::ViewI32 => Some(LoweredScalarTy::I32),
            _ => None,
        }
    }

    /// 可写视图(store 目标)的元素类型;非可写视图 → None。
    fn view_mut_elem(self) -> Option<LoweredScalarTy> {
        match self {
            ComputeParamKind::ViewMutF32 => Some(LoweredScalarTy::F32),
            ComputeParamKind::ViewMutU32 => Some(LoweredScalarTy::U32),
            ComputeParamKind::ViewMutI32 => Some(LoweredScalarTy::I32),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct ComputeParamInfo {
    kind: ComputeParamKind,
    scalar_ty: Option<LoweredScalarTy>,
    span: Span,
}

#[derive(Debug, Default)]
struct ComputeLowerCx {
    params: std::collections::HashMap<String, ComputeParamInfo>,
    locals: std::collections::HashMap<String, LoweredLocal>,
    resources: Vec<LoweredComputeResource>,
    scalar_params: Vec<LoweredScalarParam>,
    ops: Vec<LoweredComputeOp>,
    next_temp: u32,
    next_block: u32,
    next_local: u32,
}

fn lower_compute_body_slice1(
    cx: &QueryCtx<'_>,
    entry: &Body,
    f: &crate::ast::FnItem,
) -> Result<LoweredComputeBody, DxilCodegenError> {
    let mut lcx = ComputeLowerCx::new(cx, f)?;
    let Some(body) = f.body.as_ref() else {
        return Err(DxilCodegenError::unsupported(
            entry.span,
            "DXIL compute 入口缺少 body",
        ));
    };
    for stmt in &body.stmts {
        lower_compute_stmt_slice1(cx, &mut lcx, stmt)?;
    }
    if let Some(tail) = &body.tail {
        lower_unit_tail_expr_slice3a(cx, &mut lcx, tail)?;
    }
    Ok(LoweredComputeBody {
        resources: lcx.resources,
        scalar_params: lcx.scalar_params,
        ops: lcx.ops,
    })
}

impl ComputeLowerCx {
    fn new(cx: &QueryCtx<'_>, f: &crate::ast::FnItem) -> Result<Self, DxilCodegenError> {
        let mut params = std::collections::HashMap::new();
        let mut resources = Vec::new();
        let mut scalar_params = Vec::new();
        for sig in collect_compute_param_sigs(cx, f, ComputeParamMode::Body)? {
            match sig {
                ComputeParamSig::Resource {
                    name,
                    res,
                    kind,
                    span,
                } => {
                    resources.push(LoweredComputeResource {
                        name: name.clone(),
                        res,
                        elem: kind
                            .view_elem()
                            .or_else(|| kind.view_mut_elem())
                            .unwrap_or(LoweredScalarTy::F32),
                    });
                    params.insert(
                        name,
                        ComputeParamInfo {
                            kind,
                            scalar_ty: None,
                            span,
                        },
                    );
                }
                ComputeParamSig::Scalar {
                    name,
                    lowered_ty,
                    span,
                    ..
                } => {
                    scalar_params.push(LoweredScalarParam {
                        name: name.clone(),
                        ty: lowered_ty,
                    });
                    params.insert(
                        name,
                        ComputeParamInfo {
                            kind: ComputeParamKind::Scalar,
                            scalar_ty: Some(lowered_ty),
                            span,
                        },
                    );
                }
                ComputeParamSig::ThreadCtx { name, span } => {
                    params.insert(
                        name,
                        ComputeParamInfo {
                            kind: ComputeParamKind::ThreadCtx,
                            scalar_ty: None,
                            span,
                        },
                    );
                }
                ComputeParamSig::IgnoredScalar { name, span } => {
                    params.insert(
                        name,
                        ComputeParamInfo {
                            kind: ComputeParamKind::Scalar,
                            scalar_ty: None,
                            span,
                        },
                    );
                }
            }
        }
        Ok(Self {
            params,
            locals: std::collections::HashMap::new(),
            resources,
            scalar_params,
            ops: Vec::new(),
            next_temp: 0,
            next_block: 0,
            next_local: 0,
        })
    }

    fn temp(&mut self) -> String {
        let name = format!("v{}", self.next_temp);
        self.next_temp += 1;
        name
    }

    fn block_id(&mut self) -> u32 {
        let id = self.next_block;
        self.next_block += 1;
        id
    }

    fn local_slot(&mut self, name: &str) -> String {
        let id = self.next_local;
        self.next_local += 1;
        format!("local.{}.{id}", sanitize_local_name(name))
    }
}

fn lower_compute_stmt_slice1(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    stmt: &crate::ast::Stmt,
) -> Result<(), DxilCodegenError> {
    use crate::ast::StmtKind;

    match &stmt.kind {
        StmtKind::Let(let_stmt) => {
            if let_stmt.shared {
                return Err(DxilCodegenError::unsupported(
                    stmt.span,
                    "DXIL compute body lowering slice 1 不支持 shared let",
                ));
            }
            let Some(name) = ast_pat_binding_name(&let_stmt.pat) else {
                return Err(DxilCodegenError::unsupported(
                    let_stmt.pat.span,
                    "DXIL compute body lowering slice 1 仅支持简单 let 绑定",
                ));
            };
            let mutable = ast_pat_binding_mutability(&let_stmt.pat).ok_or_else(|| {
                DxilCodegenError::unsupported(
                    let_stmt.pat.span,
                    "DXIL compute body lowering slice 1 仅支持简单 let 绑定",
                )
            })?;
            let Some(init) = &let_stmt.init else {
                return Err(DxilCodegenError::unsupported(
                    stmt.span,
                    "DXIL compute body lowering slice 1 要求 let 带初始化表达式",
                ));
            };
            let mut value = lower_scalar_expr_slice3a(cx, lcx, init)?;
            if let Some(ty) = let_stmt.ty.as_ref().and_then(ast_scalar_ty) {
                // 注解定型:类型一致 / F32 常量收窄 / u32|i32 整型常量收窄(MR-0006)
                // 之外 → strict 拒绝。
                value = coerce_scalar_value(
                    value,
                    ty,
                    let_stmt.ty.as_ref().expect("guard 已确保注解存在").span,
                )
                .map_err(|e| {
                    DxilCodegenError::unsupported(
                        e.span,
                        "DXIL compute body lowering slice 3a let 标量类型与初始化表达式不一致",
                    )
                })?;
            } else if let Some(ty) = &let_stmt.ty {
                return Err(DxilCodegenError::unsupported(
                    ty.span,
                    "DXIL compute body lowering slice 3a 仅支持 f32/u32/i32/i64/bool 标量 local",
                ));
            }
            if mutable {
                let ty = let_stmt
                    .ty
                    .as_ref()
                    .and_then(ast_scalar_ty)
                    .unwrap_or(value.ty);
                let value = value.with_ty(ty);
                let slot = lcx.local_slot(&name);
                lcx.ops.push(LoweredComputeOp::LocalAlloca {
                    slot: slot.clone(),
                    ty,
                });
                lcx.ops.push(LoweredComputeOp::LocalStore {
                    slot: slot.clone(),
                    ty,
                    value,
                });
                lcx.locals.insert(name, LoweredLocal::Mutable { slot, ty });
            } else {
                lcx.locals.insert(name, LoweredLocal::Immutable(value));
            }
            Ok(())
        }
        StmtKind::Expr { expr, semi: _ } => lower_compute_expr_stmt_slice1(cx, lcx, expr),
        StmtKind::Empty => Ok(()),
        StmtKind::Item(_) => Err(DxilCodegenError::unsupported(
            stmt.span,
            "DXIL compute body lowering slice 1 不支持嵌套 item",
        )),
    }
}

fn lower_compute_expr_stmt_slice1(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    expr: &crate::ast::Expr,
) -> Result<(), DxilCodegenError> {
    use crate::ast::ExprKind;

    match &expr.kind {
        ExprKind::Assign { op: None, lhs, rhs } => lower_assignment_slice3a(cx, lcx, lhs, rhs),
        ExprKind::Assign { op: Some(_), .. } => Err(DxilCodegenError::unsupported(
            expr.span,
            "DXIL compute body lowering slice 1 不支持复合赋值",
        )),
        // GRX-009 段 stage 3:compute texel 2D store —— `dst.store(x, y, v)`(接收者
        // 为 `RWTexture2D<f32>` 形参)。x/y usize/i64 texel 坐标 + f32 value,降为本地
        // patch `@llvm.dx.resource.store.texture` 的 `<2 x i32>` coords `(x, y)` +
        // 标量 float。typeck 已定型(store→unit)。
        ExprKind::MethodCall {
            receiver,
            method,
            generic_args,
            args,
        } if method.name == "store"
            && generic_args.is_none()
            && args.len() == 3
            && path_expr_name(receiver).is_some_and(|name| {
                lcx.params
                    .get(name)
                    .is_some_and(|info| info.kind == ComputeParamKind::RWTexture2DF32)
            }) =>
        {
            let resource = path_expr_name(receiver)
                .expect("guard 已确保接收者为路径形参")
                .to_owned();
            let x = lower_texture_coord_slice3(cx, lcx, &args[0])?;
            let y = lower_texture_coord_slice3(cx, lcx, &args[1])?;
            let value = lower_f32_expr_slice1(cx, lcx, &args[2])?;
            let ptr = lcx.temp();
            lcx.ops.push(LoweredComputeOp::Store {
                ptr,
                resource,
                index: LoweredResourceIndex::ConstZero,
                value,
                tex_coords_2d: Some((x, y)),
            });
            Ok(())
        }
        ExprKind::While { cond, body } => lower_while_stmt_slice3a(cx, lcx, cond, body),
        ExprKind::Break(_) | ExprKind::Continue => Err(DxilCodegenError::unsupported(
            expr.span,
            "DXIL compute body lowering slice 3a 不支持 break/continue",
        )),
        ExprKind::If { cond, then, else_ } => lower_if_stmt_slice3a(cx, lcx, cond, then, else_),
        ExprKind::For { .. } | ExprKind::Loop { .. } | ExprKind::Match { .. } => {
            Err(DxilCodegenError::unsupported(
                expr.span,
                "DXIL compute body lowering slice 1 不支持复杂控制流",
            ))
        }
        _ => Err(unsupported_expr(expr, "表达式语句")),
    }
}

fn lower_assignment_slice3a(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    lhs: &crate::ast::Expr,
    rhs: &crate::ast::Expr,
) -> Result<(), DxilCodegenError> {
    if let Some(name) = path_expr_name(lhs) {
        let Some(local) = lcx.locals.get(name).cloned() else {
            return Err(DxilCodegenError::unsupported(
                lhs.span,
                "DXIL compute body lowering slice 3a local assignment 目标必须是已声明 mutable scalar local",
            ));
        };
        let LoweredLocal::Mutable { slot, ty } = local else {
            return Err(DxilCodegenError::unsupported(
                lhs.span,
                "DXIL compute body lowering slice 3a 不支持给不可变 local 赋值",
            ));
        };
        let value = lower_scalar_expr_slice3a(cx, lcx, rhs)?;
        let value = coerce_scalar_value(value, ty, rhs.span)?;
        lcx.ops
            .push(LoweredComputeOp::LocalStore { slot, ty, value });
        return Ok(());
    }
    lower_store_slice1(cx, lcx, lhs, rhs)
}

fn lower_store_slice1(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    lhs: &crate::ast::Expr,
    rhs: &crate::ast::Expr,
) -> Result<(), DxilCodegenError> {
    let (resource, index) = lower_index_lvalue_slice1(cx, lcx, lhs)?;
    // store 目标:可写视图(f32|u32|i32 元素,MR-0006/RXS-0181)或 UAV 纹理;
    // store 值类型 = 视图元素类型(整型常量随目标收窄,越界/异型 strict 拒)。
    let elem = match lcx.params.get(&resource) {
        Some(info) if info.kind.view_mut_elem().is_some() => {
            info.kind.view_mut_elem().expect("guard 已确保可写视图")
        }
        Some(info) if info.kind == ComputeParamKind::RWTexture2DF32 => LoweredScalarTy::F32,
        Some(info) => {
            return Err(DxilCodegenError::unsupported(
                info.span,
                "DXIL compute body lowering store 目标必须是 ViewMut<global, f32|u32|i32> 或 RWTexture2D<f32>(RXS-0181/GRX-009)",
            ));
        }
        None => {
            return Err(DxilCodegenError::unsupported(
                lhs.span,
                "DXIL compute body lowering slice 1 store 目标必须是资源形参",
            ));
        }
    };
    let value = lower_scalar_expr_slice3a(cx, lcx, rhs)?;
    let value = coerce_scalar_value(value, elem, rhs.span).map_err(|_| {
        DxilCodegenError::unsupported(
            rhs.span,
            "DXIL compute body lowering store 值类型须与视图元素类型一致(RXS-0181)",
        )
    })?;
    // ptr temp 在 rhs 之后分配,保持 IR 文本内 temp 编号单调。
    let ptr = lcx.temp();
    lcx.ops.push(LoweredComputeOp::Store {
        ptr,
        resource,
        index,
        value,
        tex_coords_2d: None,
    });
    Ok(())
}

fn lower_f32_expr_slice1(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    expr: &crate::ast::Expr,
) -> Result<LoweredValue, DxilCodegenError> {
    let value = lower_scalar_expr_slice3a(cx, lcx, expr)?;
    if value.ty != LoweredScalarTy::F32 {
        return Err(DxilCodegenError::unsupported(
            expr.span,
            "DXIL compute body lowering slice 3a store 仅支持 f32 值",
        ));
    }
    Ok(value)
}

fn lower_scalar_expr_slice3a(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    expr: &crate::ast::Expr,
) -> Result<LoweredValue, DxilCodegenError> {
    use crate::ast::ExprKind;

    match &expr.kind {
        ExprKind::Paren(inner) => lower_scalar_expr_slice3a(cx, lcx, inner),
        ExprKind::Lit(lit) if lit.kind == LitKind::Float => Ok(LoweredValue::constant(
            LoweredScalarTy::F32,
            parse_f32_lit_text(cx, lit)?,
        )),
        // 整数字面量:`u32`/`i32` 后缀直接定型为对应 32 位常量(MR-0006 位运算
        // 掩码常量域;越界 strict 拒);其余(无后缀 / i64 族)沿 slice 3a I64。
        ExprKind::Lit(lit) if lit.kind == LitKind::Int => {
            let v = parse_int_lit_value(cx, lit)?;
            let (ty, lo, hi) = match lit.suffix {
                Some(LitSuffix::U32) => (LoweredScalarTy::U32, 0, u32::MAX as i128),
                Some(LitSuffix::I32) => (LoweredScalarTy::I32, i32::MIN as i128, i32::MAX as i128),
                _ => (LoweredScalarTy::I64, i64::MIN as i128, i64::MAX as i128),
            };
            if v < lo || v > hi {
                return Err(DxilCodegenError::unsupported(
                    lit.span,
                    "DXIL compute body lowering 整数常量超出目标类型范围(strict-only)",
                ));
            }
            Ok(LoweredValue::constant(ty, v.to_string()))
        }
        ExprKind::Lit(lit) => match lit.kind {
            LitKind::Bool(value) => Ok(LoweredValue::constant(
                LoweredScalarTy::Bool,
                if value { "true" } else { "false" }.to_owned(),
            )),
            _ => Err(unsupported_expr(expr, "标量字面量")),
        },
        ExprKind::Path(path) => {
            let Some(name) = single_path_name(path) else {
                return Err(unsupported_expr(expr, "复杂路径"));
            };
            if let Some(local) = lcx.locals.get(name).cloned() {
                return match local {
                    LoweredLocal::Immutable(value) => Ok(value),
                    LoweredLocal::Mutable { slot, ty } => {
                        let dst = lcx.temp();
                        lcx.ops.push(LoweredComputeOp::LocalLoad {
                            dst: dst.clone(),
                            slot,
                            ty,
                        });
                        Ok(LoweredValue::temp(ty, dst))
                    }
                };
            }
            match lcx.params.get(name) {
                Some(info) if info.kind == ComputeParamKind::ThreadCtx => {
                    Err(DxilCodegenError::unsupported(
                        info.span,
                        "DXIL compute body lowering slice 1 不支持 ThreadCtx body lowering",
                    ))
                }
                Some(info) if info.kind == ComputeParamKind::Scalar => {
                    let Some(ty) = info.scalar_ty else {
                        return Err(DxilCodegenError::unsupported(
                            info.span,
                            "DXIL compute body lowering slice 3a 不支持该标量形参类型",
                        ));
                    };
                    let dst = lcx.temp();
                    lcx.ops.push(LoweredComputeOp::ScalarParam {
                        dst: dst.clone(),
                        name: name.to_owned(),
                        ty,
                    });
                    Ok(LoweredValue::temp(ty, dst))
                }
                Some(_) => Err(DxilCodegenError::unsupported(
                    expr.span,
                    "DXIL compute body lowering slice 3a 不支持直接读取资源句柄",
                )),
                None => Err(DxilCodegenError::unsupported(
                    expr.span,
                    "DXIL compute body lowering slice 1 遇到未知局部",
                )),
            }
        }
        ExprKind::Index { expr: base, index } => lower_load_slice1(cx, lcx, base, index),
        ExprKind::MethodCall {
            receiver,
            method,
            generic_args,
            args,
        } if path_expr_name(receiver).is_some_and(|name| {
            lcx.params
                .get(name)
                .is_some_and(|info| info.kind == ComputeParamKind::ThreadCtx)
        }) && method.name == "global_id"
            && generic_args.is_none()
            && args.is_empty() =>
        {
            let dst = lcx.temp();
            lcx.ops
                .push(LoweredComputeOp::ThreadGlobalId { dst: dst.clone() });
            Ok(LoweredValue::temp(LoweredScalarTy::I64, dst))
        }
        // GRX-009 段 stage 3:compute texel 2D load —— `tex.load(x, y)`(接收者为
        // `Texture2D<f32>` 形参)。x/y 为 usize/i64 texel 坐标,降为上游
        // `@llvm.dx.resource.load.level` 的 `<2 x i32>` coords `(x, y)`(mip=0、
        // offsets=zeroinitializer),产 f32。typeck 已定型(load→f32);此处按 AST 降级。
        ExprKind::MethodCall {
            receiver,
            method,
            generic_args,
            args,
        } if method.name == "load"
            && generic_args.is_none()
            && args.len() == 2
            && path_expr_name(receiver).is_some_and(|name| {
                lcx.params
                    .get(name)
                    .is_some_and(|info| info.kind == ComputeParamKind::Texture2DF32)
            }) =>
        {
            let resource = path_expr_name(receiver)
                .expect("guard 已确保接收者为路径形参")
                .to_owned();
            let x = lower_texture_coord_slice3(cx, lcx, &args[0])?;
            let y = lower_texture_coord_slice3(cx, lcx, &args[1])?;
            let dst = lcx.temp();
            lcx.ops.push(LoweredComputeOp::Load {
                dst: dst.clone(),
                resource,
                index: LoweredResourceIndex::ConstZero,
                tex_coords_2d: Some((x, y)),
            });
            Ok(LoweredValue::temp(LoweredScalarTy::F32, dst))
        }
        // 位扫描/位计数 intrinsic(MR-0006,RXS-0183):`w.find_lsb()` /
        // `w.find_msb()` / `w.popcount()`,u32 接收者(typeck 已按 RXS-0183 签名
        // 契约裁决;此处按 AST 方法调用形态降级,镜像 `global_id`/`tex.load`)。
        ExprKind::MethodCall {
            receiver,
            method,
            generic_args,
            args,
        } if crate::hir::DeviceBitFn::from_method(&method.name).is_some()
            && generic_args.is_none()
            && args.is_empty() =>
        {
            let op = crate::hir::DeviceBitFn::from_method(&method.name)
                .expect("guard 已确保位 intrinsic 存在");
            let value = lower_scalar_expr_slice3a(cx, lcx, receiver)?;
            if value.ty != LoweredScalarTy::U32 {
                return Err(DxilCodegenError::unsupported(
                    expr.span,
                    "DXIL compute body lowering 位扫描/位计数 intrinsic 首期仅支持 u32 接收者(RXS-0183)",
                ));
            }
            let dst = lcx.temp();
            lcx.ops.push(LoweredComputeOp::BitScan {
                dst: dst.clone(),
                op,
                value,
            });
            Ok(LoweredValue::temp(LoweredScalarTy::U32, dst))
        }
        // device 数学 intrinsic(MR-0007,RXS-0184):f32 首期四函数
        // sqrt/rsqrt/sin/cos;首期覆盖外(f64 任意 / 其余 RXS-0081 集合成员)→
        // RX6006(RXS-0081「不支持的元素类型组合/超覆盖」既有语义,strict-only
        // 不静默近似、不 fallback,P-01)。
        ExprKind::MethodCall {
            receiver,
            method,
            generic_args,
            args,
        } if crate::hir::DeviceMathFn::from_method(&method.name).is_some()
            && generic_args.is_none() =>
        {
            let op = crate::hir::DeviceMathFn::from_method(&method.name)
                .expect("guard 已确保数学 intrinsic 存在");
            // f64 首期外:f64 后缀浮点字面量接收者显式 RX6006(其余 f64 值源在
            // 本路结构上不可达——f64 形参在 compute 入口分类即 RX6007)。
            if float_lit_suffix_is_f64(receiver) {
                return Err(DxilCodegenError::math_unsupported(
                    expr.span,
                    "DXIL 路 device 数学 intrinsic 首期仅覆盖 f32(RXS-0184);f64 维持 strict 拒绝",
                ));
            }
            let value = lower_scalar_expr_slice3a(cx, lcx, receiver)?;
            if value.ty != LoweredScalarTy::F32 {
                return Err(DxilCodegenError::math_unsupported(
                    expr.span,
                    "DXIL 路 device 数学 intrinsic 首期仅覆盖 f32 接收者(RXS-0184)",
                ));
            }
            {
                use crate::hir::DeviceMathFn as MF;
                if !matches!(op, MF::Sqrt | MF::Rsqrt | MF::Sin | MF::Cos) || !args.is_empty() {
                    return Err(DxilCodegenError::math_unsupported(
                        expr.span,
                        format!(
                            "DXIL 路 device 数学 intrinsic 首期仅覆盖 f32 sqrt/rsqrt/sin/cos(RXS-0184);`{}` 超出首期覆盖,strict-only 无静默近似",
                            method.name
                        ),
                    ));
                }
            }
            let dst = lcx.temp();
            lcx.ops.push(LoweredComputeOp::MathUnary {
                dst: dst.clone(),
                op,
                value,
            });
            Ok(LoweredValue::temp(LoweredScalarTy::F32, dst))
        }
        ExprKind::Binary { op, lhs, rhs } => {
            if matches!(
                op,
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge
            ) {
                let (lhs, rhs) = lower_compatible_binary_operands(cx, lcx, lhs, rhs)?;
                if lhs.ty == LoweredScalarTy::Bool {
                    return Err(DxilCodegenError::unsupported(
                        expr.span,
                        "DXIL compute body lowering slice 3a 不支持 bool 比较",
                    ));
                }
                let dst = lcx.temp();
                lcx.ops.push(LoweredComputeOp::Compare {
                    dst: dst.clone(),
                    op: *op,
                    lhs,
                    rhs,
                });
                return Ok(LoweredValue::temp(LoweredScalarTy::Bool, dst));
            }
            let is_bitop = matches!(
                op,
                BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr
            );
            if !matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem
            ) && !is_bitop
            {
                return Err(DxilCodegenError::unsupported(
                    expr.span,
                    "DXIL compute body lowering 仅支持标量 + - * / %、位运算 & | ^ << >> 与简单比较",
                ));
            }
            let (lhs, rhs) = lower_compatible_binary_operands(cx, lcx, lhs, rhs)?;
            if lhs.ty == LoweredScalarTy::Bool {
                return Err(DxilCodegenError::unsupported(
                    expr.span,
                    "DXIL compute body lowering slice 3a 不支持 bool 算术",
                ));
            }
            if is_bitop {
                // 位运算仅整数(types.md 位运算条款,浮点违例在 typeck 落 RX2006;
                // 此处 strict 防御)。移位首期仅 u32/i32(RXS-0182):slice 3a 的
                // I64 lowering 把 usize/u64/i64 归并同底,`>>` 的 lshr/ashr 符号性
                // 不可判 → 子集外 strict 拒绝优于错码(P-01)。
                if !matches!(
                    lhs.ty,
                    LoweredScalarTy::U32 | LoweredScalarTy::I32 | LoweredScalarTy::I64
                ) {
                    return Err(DxilCodegenError::unsupported(
                        expr.span,
                        "DXIL compute body lowering 位运算仅支持整数操作数(RXS-0182)",
                    ));
                }
                if matches!(op, BinOp::Shl | BinOp::Shr) && !lhs.ty.is_int32() {
                    return Err(DxilCodegenError::unsupported(
                        expr.span,
                        "DXIL compute body lowering 移位首期仅支持 u32/i32 操作数(RXS-0182;i64/usize 移位子集外 strict 拒绝)",
                    ));
                }
            } else if *op == BinOp::Rem
                && !matches!(
                    lhs.ty,
                    LoweredScalarTy::I64 | LoweredScalarTy::U32 | LoweredScalarTy::I32
                )
            {
                return Err(DxilCodegenError::unsupported(
                    expr.span,
                    "DXIL compute body lowering slice 3a 仅支持整数 `%` 取模(modulo)",
                ));
            }
            let ty = lhs.ty;
            let dst = lcx.temp();
            lcx.ops.push(LoweredComputeOp::Binary {
                dst: dst.clone(),
                op: *op,
                lhs,
                rhs,
            });
            Ok(LoweredValue::temp(ty, dst))
        }
        ExprKind::While { .. } => Err(DxilCodegenError::unsupported(
            expr.span,
            "DXIL compute body lowering slice 1 不支持 while",
        )),
        ExprKind::Break(_) | ExprKind::Continue => Err(DxilCodegenError::unsupported(
            expr.span,
            "DXIL compute body lowering slice 3a 不支持 break/continue",
        )),
        ExprKind::If { cond, then, else_ } => {
            lower_select_expr_slice3a(cx, lcx, expr.span, cond, then, else_)
        }
        ExprKind::For { .. } | ExprKind::Loop { .. } | ExprKind::Match { .. } => {
            Err(DxilCodegenError::unsupported(
                expr.span,
                "DXIL compute body lowering slice 1 不支持复杂控制流",
            ))
        }
        _ => Err(unsupported_expr(expr, "f32 表达式")),
    }
}

/// 无后缀整型常量(I64 typed const)→ U32/I32 目标类型收窄(MR-0006:掩码/
/// 字面量常量域;越界不收窄 → None,上层落 strict 拒绝)。
fn retag_int_const(value: &LoweredValue, ty: LoweredScalarTy) -> Option<LoweredValue> {
    if !ty.is_int32() || value.ty != LoweredScalarTy::I64 {
        return None;
    }
    let LoweredValueRepr::Const(text) = &value.repr else {
        return None;
    };
    let v: i128 = text.parse().ok()?;
    let in_range = match ty {
        LoweredScalarTy::U32 => (0..=u32::MAX as i128).contains(&v),
        LoweredScalarTy::I32 => ((i32::MIN as i128)..=(i32::MAX as i128)).contains(&v),
        _ => false,
    };
    in_range.then(|| value.clone().with_ty(ty))
}

fn coerce_scalar_value(
    value: LoweredValue,
    ty: LoweredScalarTy,
    span: Span,
) -> Result<LoweredValue, DxilCodegenError> {
    if value.ty == ty {
        return Ok(value);
    }
    if ty == LoweredScalarTy::F32 && matches!(value.repr, LoweredValueRepr::Const(_)) {
        return Ok(value.with_ty(LoweredScalarTy::F32));
    }
    if let Some(v) = retag_int_const(&value, ty) {
        return Ok(v);
    }
    Err(DxilCodegenError::unsupported(
        span,
        "DXIL compute body lowering slice 3a 不支持类型不一致的标量赋值",
    ))
}

impl LoweredValue {
    fn constant(ty: LoweredScalarTy, value: String) -> Self {
        Self {
            ty,
            repr: LoweredValueRepr::Const(value),
        }
    }

    fn temp(ty: LoweredScalarTy, name: String) -> Self {
        Self {
            ty,
            repr: LoweredValueRepr::Temp(name),
        }
    }
}

fn lower_compatible_binary_operands(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    lhs: &crate::ast::Expr,
    rhs: &crate::ast::Expr,
) -> Result<(LoweredValue, LoweredValue), DxilCodegenError> {
    let span = lhs.span.to(rhs.span);
    let lhs = lower_scalar_expr_slice3a(cx, lcx, lhs)?;
    let rhs = lower_scalar_expr_slice3a(cx, lcx, rhs)?;
    coerce_binary_values(lhs, rhs, span)
}

fn coerce_binary_values(
    lhs: LoweredValue,
    rhs: LoweredValue,
    span: Span,
) -> Result<(LoweredValue, LoweredValue), DxilCodegenError> {
    if lhs.ty == rhs.ty {
        return Ok((lhs, rhs));
    }
    if lhs.ty == LoweredScalarTy::F32 && matches!(rhs.repr, LoweredValueRepr::Const(_)) {
        return Ok((lhs, rhs.with_ty(LoweredScalarTy::F32)));
    }
    if rhs.ty == LoweredScalarTy::F32 && matches!(lhs.repr, LoweredValueRepr::Const(_)) {
        return Ok((lhs.with_ty(LoweredScalarTy::F32), rhs));
    }
    // MR-0006:无后缀整型常量随另一操作数收窄到 u32/i32(镜像 F32 常量收窄;
    // 越界常量不收窄 → strict 拒绝)。
    if let Some(rhs2) = retag_int_const(&rhs, lhs.ty) {
        return Ok((lhs, rhs2));
    }
    if let Some(lhs2) = retag_int_const(&lhs, rhs.ty) {
        return Ok((lhs2, rhs));
    }
    Err(DxilCodegenError::unsupported(
        span,
        "DXIL compute body lowering slice 3a 不支持类型不一致的标量表达式",
    ))
}

impl LoweredValue {
    fn with_ty(mut self, ty: LoweredScalarTy) -> Self {
        self.ty = ty;
        self
    }
}

fn lower_select_expr_slice3a(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    span: Span,
    cond: &crate::ast::Expr,
    then: &crate::ast::Block,
    else_: &Option<Box<crate::ast::Expr>>,
) -> Result<LoweredValue, DxilCodegenError> {
    if !then.stmts.is_empty() {
        return Err(DxilCodegenError::unsupported(
            then.span,
            "DXIL compute body lowering slice 3a 仅支持无语句 if 表达式 then block",
        ));
    }
    let Some(then_tail) = &then.tail else {
        return Err(DxilCodegenError::unsupported(
            then.span,
            "DXIL compute body lowering slice 3a 要求 if then block 带标量尾表达式",
        ));
    };
    let Some(else_expr) = else_ else {
        return Err(DxilCodegenError::unsupported(
            span,
            "DXIL compute body lowering slice 3a 要求 if 表达式带 else",
        ));
    };
    let else_tail = match &else_expr.kind {
        crate::ast::ExprKind::Block(block) if block.stmts.is_empty() => {
            block.tail.as_deref().ok_or_else(|| {
                DxilCodegenError::unsupported(
                    block.span,
                    "DXIL compute body lowering slice 3a 要求 if else block 带标量尾表达式",
                )
            })?
        }
        crate::ast::ExprKind::Block(block) => {
            return Err(DxilCodegenError::unsupported(
                block.span,
                "DXIL compute body lowering slice 3a 仅支持无语句 if 表达式 else block",
            ));
        }
        crate::ast::ExprKind::If { .. } => {
            return Err(DxilCodegenError::unsupported(
                else_expr.span,
                "DXIL compute body lowering slice 3a 不支持 else-if 复杂控制流",
            ));
        }
        _ => else_expr.as_ref(),
    };
    let cond_value = lower_scalar_expr_slice3a(cx, lcx, cond)?;
    if cond_value.ty != LoweredScalarTy::Bool {
        return Err(DxilCodegenError::unsupported(
            cond.span,
            "DXIL compute body lowering slice 3a if 条件必须是 bool 比较结果",
        ));
    }
    let then_value = lower_scalar_expr_slice3a(cx, lcx, then_tail)?;
    let else_value = lower_scalar_expr_slice3a(cx, lcx, else_tail)?;
    let (then_value, else_value) = coerce_binary_values(then_value, else_value, span)?;
    if then_value.ty == LoweredScalarTy::Bool {
        return Err(DxilCodegenError::unsupported(
            span,
            "DXIL compute body lowering slice 3a 不支持 bool select 结果",
        ));
    }
    let ty = then_value.ty;
    let dst = lcx.temp();
    lcx.ops.push(LoweredComputeOp::Select {
        dst: dst.clone(),
        cond: cond_value,
        then_value,
        else_value,
    });
    Ok(LoweredValue::temp(ty, dst))
}

/// 最小 no-else statement if lowering(GRX-009 segment 3a):`if cond { stmts }` 作
/// unit 语句(语句位 + 函数体 tail 位共用)。cond 须现有 bool compare lowering 结果;
/// then block 逐句复用 [`lower_compute_stmt_slice1`](modulo / while / mutable
/// assignment / dynamic index 等子集外构造继续 strict RX6007);else / else-if 与
/// then block 尾表达式继续 strict RX6007,禁止静默降级。
fn lower_if_stmt_slice3a(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    cond: &crate::ast::Expr,
    then: &crate::ast::Block,
    else_: &Option<Box<crate::ast::Expr>>,
) -> Result<(), DxilCodegenError> {
    if let Some(else_expr) = else_ {
        return Err(DxilCodegenError::unsupported(
            else_expr.span,
            "DXIL compute body lowering slice 3a statement if 不支持 else / else-if",
        ));
    }
    let cond_value = lower_scalar_expr_slice3a(cx, lcx, cond)?;
    if cond_value.ty != LoweredScalarTy::Bool {
        return Err(DxilCodegenError::unsupported(
            cond.span,
            "DXIL compute body lowering slice 3a if 条件必须是 bool 比较结果",
        ));
    }
    // then block 换缓冲收集 ops;locals 快照/恢复(then 内 let 不外泄,块外引用
    // then 定义会违反 SSA 支配关系,strict 归「未知局部」)。temp 计数器共享,
    // SSA 名跨块唯一。
    let saved_locals = lcx.locals.clone();
    let outer_ops = std::mem::take(&mut lcx.ops);
    let mut lowered = Ok(());
    for stmt in &then.stmts {
        lowered = lower_compute_stmt_slice1(cx, lcx, stmt);
        if lowered.is_err() {
            break;
        }
    }
    if lowered.is_ok()
        && let Some(tail) = &then.tail
    {
        lowered = lower_unit_tail_expr_slice3a(cx, lcx, tail);
    }
    let then_ops = std::mem::replace(&mut lcx.ops, outer_ops);
    lcx.locals = saved_locals;
    lowered?;
    let id = lcx.block_id();
    lcx.ops.push(LoweredComputeOp::If {
        id,
        cond: cond_value,
        then_ops,
    });
    Ok(())
}

fn lower_while_stmt_slice3a(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    cond: &crate::ast::Expr,
    body: &crate::ast::Block,
) -> Result<(), DxilCodegenError> {
    let outer_ops = std::mem::take(&mut lcx.ops);
    let cond_value = lower_scalar_expr_slice3a(cx, lcx, cond);
    let cond_ops = std::mem::replace(&mut lcx.ops, outer_ops);
    let cond_value = cond_value?;
    if cond_value.ty != LoweredScalarTy::Bool {
        return Err(DxilCodegenError::unsupported(
            cond.span,
            "DXIL compute body lowering slice 3a while 条件必须是 bool 比较结果",
        ));
    }

    let saved_locals = lcx.locals.clone();
    let outer_ops = std::mem::take(&mut lcx.ops);
    let mut lowered = Ok(());
    for stmt in &body.stmts {
        lowered = lower_compute_stmt_slice1(cx, lcx, stmt);
        if lowered.is_err() {
            break;
        }
    }
    if lowered.is_ok()
        && let Some(tail) = &body.tail
    {
        lowered = lower_unit_tail_expr_slice3a(cx, lcx, tail);
    }
    let body_ops = std::mem::replace(&mut lcx.ops, outer_ops);
    lcx.locals = saved_locals;
    lowered?;

    let id = lcx.block_id();
    lcx.ops.push(LoweredComputeOp::While {
        id,
        cond_ops,
        cond: cond_value,
        body_ops,
    });
    Ok(())
}

fn lower_unit_tail_expr_slice3a(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    expr: &crate::ast::Expr,
) -> Result<(), DxilCodegenError> {
    match &expr.kind {
        crate::ast::ExprKind::If { cond, then, else_ } => {
            lower_if_stmt_slice3a(cx, lcx, cond, then, else_)
        }
        crate::ast::ExprKind::While { cond, body } => lower_while_stmt_slice3a(cx, lcx, cond, body),
        _ => Err(unsupported_expr(expr, "尾表达式")),
    }
}

fn lower_load_slice1(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    base: &crate::ast::Expr,
    index: &crate::ast::Expr,
) -> Result<LoweredValue, DxilCodegenError> {
    let Some(resource) = path_expr_name(base) else {
        return Err(unsupported_expr(base, "资源索引基址"));
    };
    // load 源:只读视图(f32|u32|i32 元素,MR-0006/RXS-0181)或 SRV 纹理;
    // 结果标量类型 = 视图元素类型。
    let elem = match lcx.params.get(resource) {
        Some(info) if info.kind.view_elem().is_some() => {
            info.kind.view_elem().expect("guard 已确保只读视图")
        }
        Some(info) if info.kind == ComputeParamKind::Texture2DF32 => LoweredScalarTy::F32,
        Some(info) if info.kind == ComputeParamKind::ThreadCtx => {
            return Err(DxilCodegenError::unsupported(
                info.span,
                "DXIL compute body lowering slice 1 不支持 ThreadCtx body lowering",
            ));
        }
        Some(info) => {
            return Err(DxilCodegenError::unsupported(
                info.span,
                "DXIL compute body lowering load 源必须是 View<global, f32|u32|i32> 或 Texture2D<f32>(RXS-0181/GRX-009)",
            ));
        }
        None => {
            return Err(DxilCodegenError::unsupported(
                base.span,
                "DXIL compute body lowering slice 1 load 源必须是资源形参",
            ));
        }
    };
    let index = lower_resource_index_slice1(cx, lcx, index)?;
    let dst = lcx.temp();
    lcx.ops.push(LoweredComputeOp::Load {
        dst: dst.clone(),
        resource: resource.to_owned(),
        index,
        tex_coords_2d: None,
    });
    Ok(LoweredValue::temp(elem, dst))
}

fn lower_index_lvalue_slice1(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    expr: &crate::ast::Expr,
) -> Result<(String, LoweredResourceIndex), DxilCodegenError> {
    use crate::ast::ExprKind;

    match &expr.kind {
        ExprKind::Index { expr: base, index } => {
            let Some(resource) = path_expr_name(base) else {
                return Err(unsupported_expr(base, "store 资源基址"));
            };
            Ok((
                resource.to_owned(),
                lower_resource_index_slice1(cx, lcx, index)?,
            ))
        }
        _ => Err(unsupported_expr(expr, "store 左值")),
    }
}

fn lower_resource_index_slice1(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    expr: &crate::ast::Expr,
) -> Result<LoweredResourceIndex, DxilCodegenError> {
    if let Some(index) = lower_usize_const_slice1(cx, expr)? {
        if index != 0 {
            return Err(DxilCodegenError::unsupported(
                expr.span,
                format!(
                    "DXIL compute body lowering slice 1 仅支持资源常量索引 0(索引 {index} 缺少资源边界 evidence)"
                ),
            ));
        }
        return Ok(LoweredResourceIndex::ConstZero);
    }
    let index = lower_scalar_expr_slice3a(cx, lcx, expr)?;
    if index.ty != LoweredScalarTy::I64 {
        return Err(DxilCodegenError::unsupported(
            expr.span,
            "DXIL compute body lowering slice 3a 资源动态索引必须是 i64/usize 标量表达式",
        ));
    }
    Ok(LoweredResourceIndex::Dynamic(index))
}

/// GRX-009 段 stage 3:纹理 2D texel 坐标标量降级(`tex.load(x,y)` / `dst.store(x,y,v)`
/// 的 x/y)。坐标须是 usize/i64 标量表达式(与动态资源索引同域);常量与动态均走
/// `lower_scalar_expr_slice3a`(常量 0 也保留为 i64 值,render 层统一 trunc 到 i32)。
fn lower_texture_coord_slice3(
    cx: &QueryCtx<'_>,
    lcx: &mut ComputeLowerCx,
    expr: &crate::ast::Expr,
) -> Result<LoweredValue, DxilCodegenError> {
    let v = lower_scalar_expr_slice3a(cx, lcx, expr)?;
    if v.ty != LoweredScalarTy::I64 {
        return Err(DxilCodegenError::unsupported(
            expr.span,
            "DXIL compute body lowering 段 stage 3 纹理 texel 坐标必须是 usize/i64 标量表达式",
        ));
    }
    Ok(v)
}

fn lower_usize_const_slice1(
    cx: &QueryCtx<'_>,
    expr: &crate::ast::Expr,
) -> Result<Option<u64>, DxilCodegenError> {
    use crate::ast::ExprKind;

    match &expr.kind {
        ExprKind::Paren(inner) => lower_usize_const_slice1(cx, inner),
        ExprKind::Lit(lit) if lit.kind == LitKind::Int => parse_usize_lit_text(cx, lit).map(Some),
        ExprKind::Binary { op, lhs, rhs } => {
            if !matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div) {
                return Ok(None);
            }
            let Some(lhs) = lower_usize_const_slice1(cx, lhs)? else {
                return Ok(None);
            };
            let Some(rhs) = lower_usize_const_slice1(cx, rhs)? else {
                return Ok(None);
            };
            match op {
                BinOp::Add => lhs.checked_add(rhs),
                BinOp::Sub => lhs.checked_sub(rhs),
                BinOp::Mul => lhs.checked_mul(rhs),
                BinOp::Div if rhs != 0 => Some(lhs / rhs),
                _ => None,
            }
            .map(Some)
            .ok_or_else(|| {
                DxilCodegenError::unsupported(
                    expr.span,
                    "DXIL compute body lowering slice 1 不支持该 usize 常量算术",
                )
            })
        }
        _ => Ok(None),
    }
}

/// View/ViewMut 第二类型实参(元素类型)头名(`View<space, T>` 的 `T`)。
fn view_elem_head(ty: &crate::ast::Ty) -> Option<&str> {
    let args = ast_ty_path_args(ty)?;
    if args.len() != 2 {
        return None;
    }
    match &args[1] {
        crate::ast::GenericArg::Type(t) => ast_ty_head_name(t),
        _ => None,
    }
}

/// MR-0006(RXS-0181):View/ViewMut 元素类型集扩到 `{f32, u32, i32}`(4 字节
/// 天然对齐标量,同 rawbuffer intrinsic 元素类型重载;probe 实测 2026-07-12
/// pinned llc i32 重载 emit ×8 字节稳定 + dxv `Validation succeeded`)。地址空间
/// 维持 `global` strict;子集外元素(f64/bool/聚合等)维持 RX6007 strict 拒绝
/// (RXS-0157 L2 边界收窄,不新造码)。
fn require_view_global_elem(ty: &crate::ast::Ty) -> Result<(), DxilCodegenError> {
    let Some(args) = ast_ty_path_args(ty) else {
        return Err(DxilCodegenError::unsupported(
            ty.span,
            "DXIL compute body lowering 要求 View/ViewMut 带 <global, f32|u32|i32>(RXS-0181)",
        ));
    };
    if args.len() != 2
        || !generic_arg_is_type_path(&args[0], "global")
        || !(generic_arg_is_type_path(&args[1], "f32")
            || generic_arg_is_type_path(&args[1], "u32")
            || generic_arg_is_type_path(&args[1], "i32"))
    {
        return Err(DxilCodegenError::unsupported(
            ty.span,
            "DXIL compute body lowering 仅支持 View/ViewMut<global, f32|u32|i32>(RXS-0181;子集外元素类型 strict 拒绝)",
        ));
    }
    Ok(())
}

/// GRX-009 texture-capable kernel artifact + MR-0006 整型视图:同时接受
/// `Texture2D<f32>`/`RWTexture2D<f32>`(compute-kernel SRV/UAV 纹理)与
/// `View/ViewMut<global, f32|u32|i32>`(raw-buffer 路径,RXS-0181)。
/// strict-only:纹理仍仅 f32 元素(`Texture2D<i32>` 不进 descriptor layout);
/// View/ViewMut 仍要求 `global` 地址空间(其他地址空间不接受)。
fn require_texture_or_view_global_elem(ty: &crate::ast::Ty) -> Result<(), DxilCodegenError> {
    let head = ast_ty_head_name(ty).unwrap_or("");
    match head {
        "Texture2D" | "RWTexture2D" => {
            let Some(args) = ast_ty_path_args(ty) else {
                return Err(DxilCodegenError::unsupported(
                    ty.span,
                    "DXIL compute body lowering GRX-009 要求 Texture2D/RWTexture2D 带 <f32>",
                ));
            };
            if args.len() != 1 || !generic_arg_is_type_path(&args[0], "f32") {
                return Err(DxilCodegenError::unsupported(
                    ty.span,
                    "DXIL compute body lowering GRX-009 仅支持 Texture2D<f32> / RWTexture2D<f32>(strict-only,非 f32 元素类型不接受)",
                ));
            }
            Ok(())
        }
        "View" | "ViewMut" => require_view_global_elem(ty),
        _ => Err(DxilCodegenError::unsupported(
            ty.span,
            "DXIL compute body lowering 仅接受 View/ViewMut<global, f32|u32|i32>/Texture2D<f32>/RWTexture2D<f32>",
        )),
    }
}

fn require_threadctx_1d(cx: &QueryCtx<'_>, ty: &crate::ast::Ty) -> Result<(), DxilCodegenError> {
    let Some(args) = ast_ty_path_args(ty) else {
        return Err(DxilCodegenError::unsupported(
            ty.span,
            "DXIL compute body lowering slice 3a 仅支持 ThreadCtx<1>",
        ));
    };
    if args.len() != 1 || !generic_arg_is_int(cx, args.first().unwrap(), "1") {
        return Err(DxilCodegenError::unsupported(
            ty.span,
            "DXIL compute body lowering slice 3a 仅支持 ThreadCtx<1>",
        ));
    }
    Ok(())
}

fn ast_ty_path_args(ty: &crate::ast::Ty) -> Option<&[crate::ast::GenericArg]> {
    use crate::ast::TyKind;

    match &ty.kind {
        TyKind::Path(p) => p
            .segments
            .last()?
            .args
            .as_ref()
            .map(|args| args.args.as_slice()),
        TyKind::Paren(inner) | TyKind::Ref { inner, .. } | TyKind::RawPtr { inner, .. } => {
            ast_ty_path_args(inner)
        }
        _ => None,
    }
}

fn generic_arg_is_type_path(arg: &crate::ast::GenericArg, name: &str) -> bool {
    matches!(arg, crate::ast::GenericArg::Type(ty) if ast_ty_head_name(ty) == Some(name))
}

fn generic_arg_is_int(cx: &QueryCtx<'_>, arg: &crate::ast::GenericArg, expected: &str) -> bool {
    match arg {
        crate::ast::GenericArg::Type(ty) => ty_const_arg_lit(ty).is_some_and(|lit| {
            lit.kind == LitKind::Int
                && strip_int_suffix(lit_text(cx, lit.span).trim()).trim() == expected
        }),
        crate::ast::GenericArg::Const(expr) => expr_int_lit(expr).is_some_and(|lit| {
            lit.kind == LitKind::Int
                && strip_int_suffix(lit_text(cx, lit.span).trim()).trim() == expected
        }),
        crate::ast::GenericArg::Lifetime(_) => false,
    }
}

fn ty_const_arg_lit(ty: &crate::ast::Ty) -> Option<&crate::ast::Lit> {
    match &ty.kind {
        crate::ast::TyKind::ConstArg(lit) => Some(lit),
        crate::ast::TyKind::Paren(inner) => ty_const_arg_lit(inner),
        _ => None,
    }
}

fn expr_int_lit(expr: &crate::ast::Expr) -> Option<&crate::ast::Lit> {
    match &expr.kind {
        crate::ast::ExprKind::Lit(lit) => Some(lit),
        crate::ast::ExprKind::Paren(inner) => expr_int_lit(inner),
        _ => None,
    }
}

fn parse_usize_lit_text(cx: &QueryCtx<'_>, lit: &crate::ast::Lit) -> Result<u64, DxilCodegenError> {
    let text = lit_text(cx, lit.span).replace('_', "");
    let digits = strip_int_suffix(&text);
    parse_int_digits(digits)
        .and_then(|v| u64::try_from(v).ok())
        .ok_or_else(|| {
            DxilCodegenError::unsupported(
                lit.span,
                "DXIL compute body lowering slice 1 无法解析 usize 常量",
            )
        })
}

/// 整数字面量数字体 → i128(radix 感知:`0x`/`0o`/`0b` 前缀,MR-0006 位运算
/// 语料的十六进制掩码常量域;下划线与后缀须已剥)。
fn parse_int_digits(digits: &str) -> Option<i128> {
    let (radix, body) = if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        (16, hex)
    } else if let Some(oct) = digits
        .strip_prefix("0o")
        .or_else(|| digits.strip_prefix("0O"))
    {
        (8, oct)
    } else if let Some(bin) = digits
        .strip_prefix("0b")
        .or_else(|| digits.strip_prefix("0B"))
    {
        (2, bin)
    } else {
        (10, digits)
    };
    i128::from_str_radix(body, radix).ok()
}

fn parse_int_lit_value(cx: &QueryCtx<'_>, lit: &crate::ast::Lit) -> Result<i128, DxilCodegenError> {
    let text = lit_text(cx, lit.span).replace('_', "");
    let digits = strip_int_suffix(&text);
    parse_int_digits(digits).ok_or_else(|| {
        DxilCodegenError::unsupported(
            lit.span,
            "DXIL compute body lowering slice 3a 无法解析整数常量",
        )
    })
}

fn parse_f32_lit_text(
    cx: &QueryCtx<'_>,
    lit: &crate::ast::Lit,
) -> Result<String, DxilCodegenError> {
    let text = lit_text(cx, lit.span).replace('_', "");
    let value = match lit.kind {
        LitKind::Int => strip_int_suffix(&text).parse::<f32>(),
        LitKind::Float => strip_float_suffix(&text, lit.suffix).parse::<f32>(),
        _ => unreachable!(),
    }
    .map_err(|_| {
        DxilCodegenError::unsupported(
            lit.span,
            "DXIL compute body lowering slice 1 无法解析 f32 常量",
        )
    })?;
    Ok(format_f32_const(value))
}

fn lit_text(cx: &QueryCtx<'_>, span: Span) -> String {
    let src = cx.src();
    let lo = span.lo.0 as usize;
    let hi = span.hi.0 as usize;
    src.get(lo..hi.min(src.len()))
        .unwrap_or("")
        .trim()
        .to_owned()
}

fn strip_int_suffix(text: &str) -> &str {
    for suffix in [
        "usize", "u64", "u32", "u16", "u8", "i64", "i32", "i16", "i8",
    ] {
        if let Some(stripped) = text.strip_suffix(suffix) {
            return stripped;
        }
    }
    text
}

fn strip_float_suffix(text: &str, suffix: Option<LitSuffix>) -> &str {
    match suffix {
        Some(LitSuffix::F32) => text.strip_suffix("f32").unwrap_or(text),
        Some(LitSuffix::F64) => text.strip_suffix("f64").unwrap_or(text),
        _ => text,
    }
}

fn format_f32_const(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}

fn path_expr_name(expr: &crate::ast::Expr) -> Option<&str> {
    use crate::ast::ExprKind;

    match &expr.kind {
        ExprKind::Path(path) => single_path_name(path),
        ExprKind::Paren(inner) => path_expr_name(inner),
        _ => None,
    }
}

/// 浮点字面量(剥括号)是否带 `f64` 后缀(RXS-0184 f64 首期外显式拒的判据)。
fn float_lit_suffix_is_f64(expr: &crate::ast::Expr) -> bool {
    match &expr.kind {
        crate::ast::ExprKind::Paren(inner) => float_lit_suffix_is_f64(inner),
        crate::ast::ExprKind::Lit(lit) => {
            lit.kind == LitKind::Float && lit.suffix == Some(LitSuffix::F64)
        }
        _ => false,
    }
}

fn sanitize_local_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unnamed".to_owned()
    } else {
        out
    }
}

fn single_path_name(path: &crate::ast::Path) -> Option<&str> {
    if path.segments.len() == 1 {
        Some(path.segments[0].ident.name.as_str())
    } else {
        None
    }
}

fn unsupported_expr(expr: &crate::ast::Expr, context: &str) -> DxilCodegenError {
    DxilCodegenError::unsupported(
        expr.span,
        format!("DXIL compute body lowering slice 1 不支持{context}"),
    )
}

/// 单个 compute kernel body → DXIL DirectX 三元组 LLVM IR 文本(空入口)。
/// artifact 路径使用 [`emit_dxil_ir_with_body`] 承载 RD-013 slice 1 body lowering;
/// 此公开入口保留给既有 A 路分发/单测的最小空体渲染。
pub fn emit_dxil_ir(body: &Body, module_name: &str) -> Result<String, DxilCodegenError> {
    Ok(render_dxil_module(
        &body.symbol,
        module_name,
        &LoweredComputeBody::default(),
        &DxilBindingPlan {
            resources: Vec::new(),
            scalars: Vec::new(),
        },
    ))
}

fn emit_dxil_ir_with_body(
    body: &Body,
    module_name: &str,
    lowered_body: &LoweredComputeBody,
) -> Result<String, DxilCodegenError> {
    let plan = plan_dxil_bindings(body, lowered_body)?;
    Ok(render_dxil_module(
        &body.symbol,
        module_name,
        lowered_body,
        &plan,
    ))
}

/// 渲染期绑定计划:资源 register/space + root constant 的 cbuffer 字节偏移。
/// 与 descriptor layout JSON 同源([`binding_layout::infer_register_assignments`] /
/// [`binding_layout::pack_root_constants`]),防 IR 与离线 artifact 漂移。
struct DxilBindingPlan {
    /// 资源名 → (res, 元素类型, register, space);声明序与 `lowered_body.resources`
    /// 一致。`res` 为 MIR 资源类型(GRX-009:区分 raw-buffer StructuredBuffer 与
    /// Texture2D/RWTexture2D;mutable 由 `res.class() == Uav` 派生);元素类型
    /// (MR-0006/RXS-0181)选 rawbuffer intrinsic 重载(f32|i32)。
    resources: Vec<(
        String,
        crate::mir::MirResourceType,
        LoweredScalarTy,
        u32,
        u32,
    )>,
    /// root constant 名 → (类型, cbuffer 字节偏移 = dword_offset×4)。
    scalars: Vec<(String, LoweredScalarTy, u32)>,
}

impl DxilBindingPlan {
    #[allow(clippy::type_complexity)]
    fn resource(
        &self,
        name: &str,
    ) -> Option<&(
        String,
        crate::mir::MirResourceType,
        LoweredScalarTy,
        u32,
        u32,
    )> {
        self.resources.iter().find(|(n, ..)| n == name)
    }

    fn scalar(&self, name: &str) -> Option<&(String, LoweredScalarTy, u32)> {
        self.scalars.iter().find(|(n, ..)| n == name)
    }
}

fn plan_dxil_bindings(
    body: &Body,
    lowered_body: &LoweredComputeBody,
) -> Result<DxilBindingPlan, DxilCodegenError> {
    // 资源 register/space:重建声明序 ResourceBinding(与 derive_compute_bindings
    // 同构)走同一分配函数,保证与 descriptor layout / RTS0 同源。GRX-009:直接
    // 透传 `r.res`(Texture2D/RWTexture2D 不再被回退为 StructuredBuffer)。
    let bindings = lowered_resource_bindings(&lowered_body.resources);
    let assignments = binding_layout::infer_register_assignments(&bindings).map_err(|e| {
        DxilCodegenError::unsupported(body.span, format!("绑定布局推导失败(strict-only): {e}"))
    })?;
    let resources = lowered_body
        .resources
        .iter()
        .zip(assignments.iter())
        .map(|(r, a)| (r.name.clone(), r.res, r.elem, a.register, a.space))
        .collect();

    // root constant → cbuffer 字节偏移(密排 dword × 4;与 pack_root_constants 同源)。
    // cbuffer 行规则门(strict-only):i64 须 8 字节对齐(偶数 dword),否则 16 字节
    // 行内寻址与 root constant 密排 dword 布局不相容 → 拒绝,不产不一致 IR。
    let packed = binding_layout::pack_root_constants(
        lowered_body
            .scalar_params
            .iter()
            .map(|s| {
                let ty = match s.ty {
                    LoweredScalarTy::F32 => binding_layout::RootConstantType::F32,
                    // 标量形参分类不产 U32/I32(整型 root constant 维持 I64 打包);
                    // 防御性归 I64。
                    LoweredScalarTy::U32 | LoweredScalarTy::I32 | LoweredScalarTy::I64 => {
                        binding_layout::RootConstantType::I64
                    }
                    LoweredScalarTy::Bool => binding_layout::RootConstantType::Bool,
                };
                (s.name.clone(), ty)
            })
            .collect(),
    );
    let mut scalars = Vec::with_capacity(packed.len());
    for (s, c) in lowered_body.scalar_params.iter().zip(packed.iter()) {
        if s.ty == LoweredScalarTy::I64 && c.dword_offset % 2 != 0 {
            return Err(DxilCodegenError::unsupported(
                body.span,
                format!(
                    "root constant `{}`(i64)落在奇数 dword 偏移 {},与 cbuffer 行对齐规则不相容(strict-only)",
                    s.name, c.dword_offset
                ),
            ));
        }
        scalars.push((s.name.clone(), s.ty, c.dword_offset * 4));
    }
    Ok(DxilBindingPlan { resources, scalars })
}

/// 资源句柄 SSA 名(entry 头部 `handlefrombinding` 结果;体内 load/store 复用)。
fn resource_handle_name(name: &str) -> String {
    format!("rx_h_{name}")
}

/// `target("dx.RawBuffer", <elem>, is_uav, 0)` 目标类型文本(StructuredBuffer 忠实
/// 形,对齐 MIR `StructuredBuffer{read_only}` 与 RTS0 root descriptor 可绑定性)。
/// 元素类型按 MR-0006/RXS-0181 元素类型重载:f32 → `float`;u32/i32 → `i32`
/// (LLVM IR 层同为 `i32`,有/无符号语义由运算指令侧承载;probe 实测 2026-07-12
/// pinned llc i32 重载 + 混合 f32/i32 模块均 emit ×8 字节稳定 + dxv 接受)。
fn rawbuffer_target_ty(mutable: bool, elem: LoweredScalarTy) -> String {
    let elem_ty = match elem {
        LoweredScalarTy::U32 | LoweredScalarTy::I32 => "i32",
        _ => "float",
    };
    format!(
        "target(\"dx.RawBuffer\", {elem_ty}, {}, 0)",
        if mutable { 1 } else { 0 }
    )
}

/// rawbuffer 元素的 LLVM IR 类型文本(intrinsic 重载轴:f32 → `float`,
/// u32/i32 → `i32`;MR-0006/RXS-0181)。
fn rawbuffer_elem_ir_ty(elem: LoweredScalarTy) -> &'static str {
    match elem {
        LoweredScalarTy::U32 | LoweredScalarTy::I32 => "i32",
        _ => "float",
    }
}

/// GRX-009 texture-capable kernel artifact:纹理目标类型文本(**上游 DirectX target
/// ext 拼写**,LLVM PR #193343 texture load.level + 本地 store.texture patch)。
/// 形态:`target("dx.Texture", <ElemTy>, IsWriteable, IsROV, IsSigned, Dimension)`。
/// Texture2D<f32>(SRV)→ `target("dx.Texture", float, 0, 0, 0, 2)`;
/// RWTexture2D<f32>(UAV)→ `target("dx.Texture", float, 1, 0, 0, 2)`
/// (`IsWriteable=1`)。Dimension=2 = Texture2D;ROV=0、Signed=0(float)。此拼写被
/// patched llc(`llvm.dx.resource.load.level`/`store.texture` → dx.op.textureLoad(66)/
/// textureStore(67))识别并降级;旧自造拼写 `target("dx.Texture2D<float>", 0, 0)` 已
/// 淘汰(任何 llc 均按名拒绝,见 texture_intrinsic_toolchain_blocker.json)。
fn texture_target_ty(mutable: bool) -> String {
    if mutable {
        r#"target("dx.Texture", float, 1, 0, 0, 2)"#.to_string()
    } else {
        r#"target("dx.Texture", float, 0, 0, 0, 2)"#.to_string()
    }
}

/// GRX-009:按 MIR 资源类型选 `rawbuffer_target_ty` vs `texture_target_ty`,
/// 并返回 (target_ty 文本, is_texture 标志)。raw-buffer 路径(`View`/`ViewMut`)
/// 沿用 `rawbuffer_target_ty`(元素类型重载,MR-0006/RXS-0181);
/// `Texture2D`/`RWTexture2D` 走 `texture_target_ty`。
fn resource_target_ty(res: crate::mir::MirResourceType, elem: LoweredScalarTy) -> (String, bool) {
    match res {
        crate::mir::MirResourceType::Texture2D(_) => (texture_target_ty(false), true),
        crate::mir::MirResourceType::RWTexture2D(_) => (texture_target_ty(true), true),
        _ => {
            let mutable = res.class() == crate::mir::ResourceClass::Uav;
            (rawbuffer_target_ty(mutable, elem), false)
        }
    }
}

/// cbuffer 布局 LLVM 类型名(镜像 clang HLSL `__cblayout_<name>` 约定)。
fn cblayout_type_name(module_name: &str) -> String {
    format!("%__cblayout_{module_name}")
}

/// cbuffer 字段的 LLVM 存储类型(bool 按 D3D12 root constant 单 dword = i32 落存,
/// 读取处再比零还原 i1;f32/i64 与标量类型一致)。
fn cbuffer_field_ty(ty: LoweredScalarTy) -> &'static str {
    match ty {
        LoweredScalarTy::F32 => "float",
        // 标量形参分类不产 U32/I32(整型 root constant 维持 I64 打包,布局 0-byte);
        // 防御性映射到 i32。
        LoweredScalarTy::U32 | LoweredScalarTy::I32 => "i32",
        LoweredScalarTy::I64 => "i64",
        LoweredScalarTy::Bool => "i32",
    }
}

/// DirectX 三元组 LLVM IR 文本(空入口或 RD-013 slice 1 body)。形态对齐 LLVM DirectX
/// 后端 emit 期望(shadermodel6.0-compute 三元组 + DXIL 数据布局 + `hlsl.shader`/
/// `hlsl.numthreads` 入口属性)。资源/线程内建/root constant 一律走上游 `llvm.dx.*`
/// intrinsic(`handlefrombinding`/`load.rawbuffer`/`store.rawbuffer`/`getpointer`/
/// `thread.id`),由 patched llc 的 DirectX 后端降为 dx.op + 资源元数据——手搓
/// external global / 自造 intrinsic 均过不了 dxv(External declaration unused /
/// not a DXIL function)。当前 body IR 仍不等同于 Godot resource mapping。
fn render_dxil_module(
    entry_symbol: &str,
    module_name: &str,
    lowered_body: &LoweredComputeBody,
    plan: &DxilBindingPlan,
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
        "target triple = \"dxil-unknown-shadermodel6.0-compute\""
    );
    out.push('\n');
    let cblayout = cblayout_type_name(module_name);
    if !plan.scalars.is_empty() {
        let fields: Vec<&'static str> = plan
            .scalars
            .iter()
            .map(|(_, ty, _)| cbuffer_field_ty(*ty))
            .collect();
        let _ = writeln!(out, "{cblayout} = type <{{ {} }}>", fields.join(", "));
        out.push('\n');
    }
    let _ = writeln!(out, "define void @{entry_symbol}() #0 {{");
    out.push_str("entry:\n");
    // entry 头部:cbuffer / 资源句柄(register/space 与 descriptor layout 同源)。
    // root constants 经 RTS0 以 b0 绑定 → cbuffer 句柄 (space 0, b0, range 1)。
    if !plan.scalars.is_empty() {
        let _ = writeln!(
            out,
            "  %rx_cb = call target(\"dx.CBuffer\", {cblayout}) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)"
        );
    }
    for (name, res, elem, register, space) in &plan.resources {
        // GRX-009:按 MIR 资源类型选 rawbuffer vs texture target ty。`handlefrombinding`
        // 调用形态保持一致(spec ADDDED Requirements「Texture-Capable DXIL Lowering」)。
        // 第 4 实参 = **range 内相对 index**(单资源 range 恒 0)——MR-0006 probe 实测
        // (2026-07-12,pinned llc):llc 以 `lowerBound + index` 计算 createHandle 的
        // 绝对 register index,旧拼写传 register(register>0 时 `2×register` 越 range)
        // → dxv `Constant values must be in-range for operation` 拒;相对 0 → 接受。
        // register 0 资源(既有全部 golden/语料)两种拼写字节一致,0-byte。
        let (target_ty, _is_texture) = resource_target_ty(*res, *elem);
        let _ = writeln!(
            out,
            "  %{handle} = call {ty} @llvm.dx.resource.handlefrombinding(i32 {space}, i32 {register}, i32 1, i32 0, ptr null)",
            handle = resource_handle_name(name),
            ty = target_ty,
        );
    }
    render_lowered_compute_ops(&mut out, lowered_body, module_name, plan);
    out.push_str("  ret void\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str(
        "attributes #0 = { noinline nounwind \"hlsl.numthreads\"=\"1,1,1\" \"hlsl.shader\"=\"compute\" }\n",
    );
    // `!dx.valver`:llc DXContainer 写出器恒产 PSV0 v3(52 字节);模块不声明
    // validator version 时 dxv 按默认版本期望 24 字节 PSV0 → 容器级
    // `PSVRuntimeInfoSize` mismatch 拒绝。声明 1.8 与 pinned 签名 validator
    // (dxc-round7,1.9)实测相容。
    out.push('\n');
    out.push_str("!dx.valver = !{!0}\n");
    out.push('\n');
    out.push_str("!0 = !{i32 1, i32 8}\n");
    out
}

fn render_lowered_compute_ops(
    out: &mut String,
    body: &LoweredComputeBody,
    module_name: &str,
    plan: &DxilBindingPlan,
) {
    render_local_allocas(out, &body.ops);
    render_lowered_ops(out, &body.ops, module_name, plan);
}

fn render_local_allocas(out: &mut String, ops: &[LoweredComputeOp]) {
    for op in ops {
        match op {
            LoweredComputeOp::LocalAlloca { slot, ty } => {
                let _ = writeln!(
                    out,
                    "  %{slot}.addr = alloca {ty}",
                    ty = render_scalar_ty(*ty)
                );
            }
            LoweredComputeOp::If { then_ops, .. } => render_local_allocas(out, then_ops),
            LoweredComputeOp::While {
                cond_ops, body_ops, ..
            } => {
                render_local_allocas(out, cond_ops);
                render_local_allocas(out, body_ops);
            }
            _ => {}
        }
    }
}

/// rawbuffer intrinsic 的 i32 元素索引操作数:常量 0 直用;动态 i64 索引先 trunc
/// (`%{temp_prefix}.idx`)。intrinsic 索引位宽即 i32(dx.op.bufferLoad 同形)。
fn render_rawbuffer_index(
    out: &mut String,
    index: &LoweredResourceIndex,
    temp_prefix: &str,
) -> String {
    match index {
        LoweredResourceIndex::ConstZero => "0".to_owned(),
        LoweredResourceIndex::Dynamic(value) => {
            let _ = writeln!(
                out,
                "  %{temp_prefix}.idx = trunc i64 {} to i32",
                render_lowered_value(value)
            );
            format!("%{temp_prefix}.idx")
        }
    }
}

/// GRX-009 纹理 texel 坐标(render 层内部表示):1D 线性索引(打通形)或 2D `(x,y)`。
#[derive(Debug, Clone)]
enum LoweredTextureCoords {
    /// 1D 线性索引 → texel `(idx, 0)`(段 stage 2 打通形,非 2D 语义等价)。
    Coords1D(LoweredResourceIndex),
    /// 2D texel 坐标 `(x, y)`(段 stage 3,与 HLSL bridge `Load(int3(sx,sy,0))` 一致)。
    Coords2D { x: LoweredValue, y: LoweredValue },
}

/// 由 Load/Store 的 `index` + 可选 2D 坐标字段决定纹理 texel 坐标形态:2D 坐标存在则
/// 取 `Coords2D`(方法调用 `tex.load(x,y)` 形),否则从 1D `index` 派生 `Coords1D`。
fn texture_coords(
    index: &LoweredResourceIndex,
    tex_coords_2d: &Option<(LoweredValue, LoweredValue)>,
) -> LoweredTextureCoords {
    match tex_coords_2d {
        Some((x, y)) => LoweredTextureCoords::Coords2D {
            x: x.clone(),
            y: y.clone(),
        },
        None => LoweredTextureCoords::Coords1D(index.clone()),
    }
}

/// GRX-009 texture texel 坐标:把资源索引组装为 `<2 x i32>` 坐标操作数,供上游
/// `llvm.dx.resource.load.level` / `store.texture` 使用。返回可直接嵌入 intrinsic 调用
/// 的 `<2 x i32> ...` 文本。
///
/// - `Coords1D`(段 stage 2 打通形):单一线性索引 → texel `(idx, 0)`。ConstZero →
///   `<2 x i32> zeroinitializer`;Dynamic(i64)→ trunc 到 i32 后 insertelement 进
///   (idx, 0)。**注意**:此形非 2D 语义等价(texel `(idx,0)` ≠ `(x,y)`),仅为
///   toolchain 打通里程碑;真实 2D 坐标由 `Coords2D` 承载(段 stage 3)。
/// - `Coords2D`:两个 i64 标量 `(x, y)` 各 trunc 到 i32 后 insertelement 进 `(x, y)`,
///   与 HLSL bridge `Load(int3(sx, sy, 0))` 的 texel 寻址一致。
fn render_texture_coords(
    out: &mut String,
    coords: &LoweredTextureCoords,
    temp_prefix: &str,
) -> String {
    match coords {
        LoweredTextureCoords::Coords1D(LoweredResourceIndex::ConstZero) => {
            "<2 x i32> zeroinitializer".to_owned()
        }
        LoweredTextureCoords::Coords1D(LoweredResourceIndex::Dynamic(value)) => {
            let _ = writeln!(
                out,
                "  %{temp_prefix}.idx = trunc i64 {} to i32",
                render_lowered_value(value)
            );
            let _ = writeln!(
                out,
                "  %{temp_prefix}.c0 = insertelement <2 x i32> poison, i32 %{temp_prefix}.idx, i32 0"
            );
            let _ = writeln!(
                out,
                "  %{temp_prefix}.coords = insertelement <2 x i32> %{temp_prefix}.c0, i32 0, i32 1"
            );
            format!("<2 x i32> %{temp_prefix}.coords")
        }
        LoweredTextureCoords::Coords2D { x, y } => {
            let _ = writeln!(
                out,
                "  %{temp_prefix}.x = trunc i64 {} to i32",
                render_lowered_value(x)
            );
            let _ = writeln!(
                out,
                "  %{temp_prefix}.y = trunc i64 {} to i32",
                render_lowered_value(y)
            );
            let _ = writeln!(
                out,
                "  %{temp_prefix}.c0 = insertelement <2 x i32> poison, i32 %{temp_prefix}.x, i32 0"
            );
            let _ = writeln!(
                out,
                "  %{temp_prefix}.coords = insertelement <2 x i32> %{temp_prefix}.c0, i32 %{temp_prefix}.y, i32 1"
            );
            format!("<2 x i32> %{temp_prefix}.coords")
        }
    }
}

fn render_lowered_ops(
    out: &mut String,
    ops: &[LoweredComputeOp],
    module_name: &str,
    plan: &DxilBindingPlan,
) {
    for op in ops {
        match op {
            LoweredComputeOp::Load {
                dst,
                resource,
                index,
                tex_coords_2d,
            } => {
                // GRX-009:按 MIR 资源类型分 rawbuffer vs texture load 路径。
                // rawbuffer 路径按元素类型选 intrinsic 重载(MR-0006/RXS-0181:
                // f32 → `{float,i1}`,u32/i32 → `{i32,i1}`);`Texture2D<f32>` 走上游
                // `@llvm.dx.resource.load.level`(返回元素类型本身,无 `{float,i1}`;
                // coords 为 `<2 x i32>`,mip=0、offsets=zeroinitializer)。
                let (res, elem) = plan
                    .resource(resource)
                    .map(|(_, r, e, ..)| (*r, *e))
                    .unwrap_or((
                        crate::mir::MirResourceType::StructuredBuffer { read_only: true },
                        LoweredScalarTy::F32,
                    ));
                match res {
                    crate::mir::MirResourceType::Texture2D(_) => {
                        let ty = texture_target_ty(false);
                        let coords =
                            render_texture_coords(out, &texture_coords(index, tex_coords_2d), dst);
                        let _ = writeln!(
                            out,
                            "  %{dst} = call float @llvm.dx.resource.load.level({ty} %{handle}, {coords}, i32 0, <2 x i32> zeroinitializer)",
                            handle = resource_handle_name(resource),
                        );
                    }
                    _ => {
                        let idx = render_rawbuffer_index(out, index, dst);
                        let ty = rawbuffer_target_ty(
                            res.class() == crate::mir::ResourceClass::Uav,
                            elem,
                        );
                        let ret = rawbuffer_elem_ir_ty(elem);
                        let _ = writeln!(
                            out,
                            "  %{dst}.ld = call {{ {ret}, i1 }} @llvm.dx.resource.load.rawbuffer({ty} %{handle}, i32 {idx}, i32 0)",
                            handle = resource_handle_name(resource),
                        );
                        let _ =
                            writeln!(out, "  %{dst} = extractvalue {{ {ret}, i1 }} %{dst}.ld, 0");
                    }
                }
            }
            LoweredComputeOp::Store {
                ptr,
                resource,
                index,
                value,
                tex_coords_2d,
            } => {
                // GRX-009:按 MIR 资源类型分 rawbuffer vs texture store 路径。
                // rawbuffer 路径按元素类型选 intrinsic 重载(MR-0006/RXS-0181);
                // `RWTexture2D<f32>` 走本地 patch `@llvm.dx.resource.store.texture`
                // (coords 为 `<2 x i32>`,value 为标量 float;降为 dx.op.textureStore(67),
                // mask=15 标量 splat 4 份,见 DXILOpLowering::lowerTextureStore)。
                let (res, elem) = plan
                    .resource(resource)
                    .map(|(_, r, e, ..)| (*r, *e))
                    .unwrap_or((
                        crate::mir::MirResourceType::StructuredBuffer { read_only: false },
                        LoweredScalarTy::F32,
                    ));
                match res {
                    crate::mir::MirResourceType::RWTexture2D(_) => {
                        let ty = texture_target_ty(true);
                        let coords =
                            render_texture_coords(out, &texture_coords(index, tex_coords_2d), ptr);
                        let _ = writeln!(
                            out,
                            "  call void @llvm.dx.resource.store.texture({ty} %{handle}, {coords}, float {})",
                            render_lowered_value(value),
                            handle = resource_handle_name(resource),
                        );
                    }
                    _ => {
                        let idx = render_rawbuffer_index(out, index, ptr);
                        let ty = rawbuffer_target_ty(
                            res.class() == crate::mir::ResourceClass::Uav,
                            elem,
                        );
                        let vt = rawbuffer_elem_ir_ty(elem);
                        let _ = writeln!(
                            out,
                            "  call void @llvm.dx.resource.store.rawbuffer({ty} %{handle}, i32 {idx}, i32 0, {vt} {})",
                            render_lowered_value(value),
                            handle = resource_handle_name(resource),
                        );
                    }
                }
            }
            LoweredComputeOp::LocalAlloca { .. } => {}
            LoweredComputeOp::LocalLoad { dst, slot, ty } => {
                let _ = writeln!(
                    out,
                    "  %{dst} = load {ty}, ptr %{slot}.addr",
                    ty = render_scalar_ty(*ty)
                );
            }
            LoweredComputeOp::LocalStore { slot, ty, value } => {
                let _ = writeln!(
                    out,
                    "  store {ty} {}, ptr %{slot}.addr",
                    render_lowered_value(value),
                    ty = render_scalar_ty(*ty)
                );
            }
            LoweredComputeOp::Binary { dst, op, lhs, rhs } => {
                // 移位:移位量按位宽取模(MR-0006 判档 O-2/RXS-0182)——emit 显式
                // 掩码 `and i32 amount, 31`(消除 LLVM 移位越界 poison;u32 → lshr,
                // i32 → ashr;首期仅 32 位,lowering 侧已 strict 收口)。
                if matches!(op, BinOp::Shl | BinOp::Shr) {
                    let inst = match (*op, lhs.ty) {
                        (BinOp::Shl, _) => "shl",
                        (BinOp::Shr, LoweredScalarTy::U32) => "lshr",
                        (BinOp::Shr, _) => "ashr",
                        _ => unreachable!("外层已限定 Shl/Shr"),
                    };
                    let _ = writeln!(
                        out,
                        "  %{dst}.shamt = and i32 {}, 31",
                        render_lowered_value(rhs)
                    );
                    let _ = writeln!(
                        out,
                        "  %{dst} = {inst} i32 {}, %{dst}.shamt",
                        render_lowered_value(lhs)
                    );
                    continue;
                }
                let opcode = match (lhs.ty, *op) {
                    (LoweredScalarTy::F32, BinOp::Add) => "fadd",
                    (LoweredScalarTy::F32, BinOp::Sub) => "fsub",
                    (LoweredScalarTy::F32, BinOp::Mul) => "fmul",
                    (LoweredScalarTy::F32, BinOp::Div) => "fdiv",
                    (
                        LoweredScalarTy::I64 | LoweredScalarTy::U32 | LoweredScalarTy::I32,
                        BinOp::Add,
                    ) => "add",
                    (
                        LoweredScalarTy::I64 | LoweredScalarTy::U32 | LoweredScalarTy::I32,
                        BinOp::Sub,
                    ) => "sub",
                    (
                        LoweredScalarTy::I64 | LoweredScalarTy::U32 | LoweredScalarTy::I32,
                        BinOp::Mul,
                    ) => "mul",
                    // 有/无符号语义由指令侧承载(RXS-0181):u32 → udiv/urem。
                    (LoweredScalarTy::U32, BinOp::Div) => "udiv",
                    (LoweredScalarTy::U32, BinOp::Rem) => "urem",
                    (LoweredScalarTy::I64 | LoweredScalarTy::I32, BinOp::Div) => "sdiv",
                    (LoweredScalarTy::I64 | LoweredScalarTy::I32, BinOp::Rem) => "srem",
                    // 位运算(RXS-0182):按位与/或/异或为符号无关平凡指令。
                    (
                        LoweredScalarTy::I64 | LoweredScalarTy::U32 | LoweredScalarTy::I32,
                        BinOp::BitAnd,
                    ) => "and",
                    (
                        LoweredScalarTy::I64 | LoweredScalarTy::U32 | LoweredScalarTy::I32,
                        BinOp::BitOr,
                    ) => "or",
                    (
                        LoweredScalarTy::I64 | LoweredScalarTy::U32 | LoweredScalarTy::I32,
                        BinOp::BitXor,
                    ) => "xor",
                    _ => unreachable!(),
                };
                let _ = writeln!(
                    out,
                    "  %{dst} = {opcode} {ty} {}, {}",
                    render_lowered_value(lhs),
                    render_lowered_value(rhs),
                    ty = render_scalar_ty(lhs.ty)
                );
            }
            LoweredComputeOp::ScalarParam { dst, name, ty } => {
                let byte_offset = plan.scalar(name).map(|(_, _, off)| *off).unwrap_or(0);
                let cblayout = cblayout_type_name(module_name);
                let _ = writeln!(
                    out,
                    "  %{dst}.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target(\"dx.CBuffer\", {cblayout}) %rx_cb, i32 {byte_offset})"
                );
                match ty {
                    LoweredScalarTy::Bool => {
                        // bool root constant 落存单 dword(i32),读取处比零还原 i1。
                        let _ =
                            writeln!(out, "  %{dst}.i32 = load i32, ptr addrspace(2) %{dst}.ptr");
                        let _ = writeln!(out, "  %{dst} = icmp ne i32 %{dst}.i32, 0");
                    }
                    _ => {
                        let _ = writeln!(
                            out,
                            "  %{dst} = load {ty}, ptr addrspace(2) %{dst}.ptr",
                            ty = render_scalar_ty(*ty)
                        );
                    }
                }
            }
            LoweredComputeOp::ThreadGlobalId { dst } => {
                let _ = writeln!(out, "  %{dst}.u32 = call i32 @llvm.dx.thread.id(i32 0)");
                let _ = writeln!(out, "  %{dst} = zext i32 %{dst}.u32 to i64");
            }
            LoweredComputeOp::Compare { dst, op, lhs, rhs } => {
                let pred = render_compare_predicate(lhs.ty, *op);
                let _ = writeln!(
                    out,
                    "  %{dst} = {cmp} {pred} {ty} {}, {}",
                    render_lowered_value(lhs),
                    render_lowered_value(rhs),
                    cmp = if lhs.ty == LoweredScalarTy::F32 {
                        "fcmp"
                    } else {
                        "icmp"
                    },
                    ty = render_scalar_ty(lhs.ty)
                );
            }
            LoweredComputeOp::Select {
                dst,
                cond,
                then_value,
                else_value,
            } => {
                let _ = writeln!(
                    out,
                    "  %{dst} = select i1 {}, {ty} {}, {ty} {}",
                    render_lowered_value(cond),
                    render_lowered_value(then_value),
                    render_lowered_value(else_value),
                    ty = render_scalar_ty(then_value.ty)
                );
            }
            LoweredComputeOp::BitScan { dst, op, value } => {
                let v = render_lowered_value(value);
                match op {
                    // find_lsb → FirstbitLo(32):op 本身即 HLSL 形(零输入 -1),
                    // 与 dxc `firstbitlow` 产物同形(直发,无正规化;probe 锚)。
                    crate::hir::DeviceBitFn::FindLsb => {
                        let _ = writeln!(out, "  %{dst} = call i32 @llvm.dx.firstbitlow(i32 {v})");
                    }
                    // find_msb → FirstbitHi(33) + dxc 同款正规化(O-7 golden 锚):
                    // `select(raw == -1, -1, 31 - raw)` → LSB=0 位序 + 零输入 HLSL 形。
                    crate::hir::DeviceBitFn::FindMsb => {
                        let _ = writeln!(
                            out,
                            "  %{dst}.raw = call i32 @llvm.dx.firstbituhigh(i32 {v})"
                        );
                        let _ = writeln!(out, "  %{dst}.norm = sub i32 31, %{dst}.raw");
                        let _ = writeln!(out, "  %{dst}.isz = icmp eq i32 %{dst}.raw, -1");
                        let _ = writeln!(
                            out,
                            "  %{dst} = select i1 %{dst}.isz, i32 -1, i32 %{dst}.norm"
                        );
                    }
                    crate::hir::DeviceBitFn::Popcount => {
                        let _ = writeln!(out, "  %{dst} = call i32 @llvm.ctpop.i32(i32 {v})");
                    }
                }
            }
            LoweredComputeOp::MathUnary { dst, op, value } => {
                let intr = match op {
                    crate::hir::DeviceMathFn::Sqrt => "llvm.sqrt.f32",
                    crate::hir::DeviceMathFn::Rsqrt => "llvm.dx.rsqrt.f32",
                    crate::hir::DeviceMathFn::Sin => "llvm.sin.f32",
                    crate::hir::DeviceMathFn::Cos => "llvm.cos.f32",
                    // 首期覆盖外在 lowering 侧已 RX6006 strict 收口(RXS-0184)。
                    _ => unreachable!("RXS-0184 首期覆盖外不产 MathUnary"),
                };
                let _ = writeln!(
                    out,
                    "  %{dst} = call float @{intr}(float {})",
                    render_lowered_value(value)
                );
            }
            LoweredComputeOp::If { id, cond, then_ops } => {
                let _ = writeln!(
                    out,
                    "  br i1 {}, label %if.then.{id}, label %if.end.{id}",
                    render_lowered_value(cond)
                );
                let _ = writeln!(out, "if.then.{id}:");
                render_lowered_ops(out, then_ops, module_name, plan);
                let _ = writeln!(out, "  br label %if.end.{id}");
                let _ = writeln!(out, "if.end.{id}:");
            }
            LoweredComputeOp::While {
                id,
                cond_ops,
                cond,
                body_ops,
            } => {
                let _ = writeln!(out, "  br label %while.cond.{id}");
                let _ = writeln!(out, "while.cond.{id}:");
                render_lowered_ops(out, cond_ops, module_name, plan);
                let _ = writeln!(
                    out,
                    "  br i1 {}, label %while.body.{id}, label %while.end.{id}",
                    render_lowered_value(cond)
                );
                let _ = writeln!(out, "while.body.{id}:");
                render_lowered_ops(out, body_ops, module_name, plan);
                let _ = writeln!(out, "  br label %while.cond.{id}");
                let _ = writeln!(out, "while.end.{id}:");
            }
        }
    }
}

fn render_lowered_value(value: &LoweredValue) -> String {
    match &value.repr {
        LoweredValueRepr::Const(v) => match value.ty {
            LoweredScalarTy::F32 => format_f32_const(v.parse::<f32>().unwrap_or(0.0)),
            LoweredScalarTy::U32
            | LoweredScalarTy::I32
            | LoweredScalarTy::I64
            | LoweredScalarTy::Bool => v.clone(),
        },
        LoweredValueRepr::Temp(v) => format!("%{v}"),
    }
}

fn render_scalar_ty(ty: LoweredScalarTy) -> &'static str {
    match ty {
        LoweredScalarTy::F32 => "float",
        LoweredScalarTy::U32 | LoweredScalarTy::I32 => "i32",
        LoweredScalarTy::I64 => "i64",
        LoweredScalarTy::Bool => "i1",
    }
}

fn render_compare_predicate(ty: LoweredScalarTy, op: BinOp) -> &'static str {
    match (ty, op) {
        (LoweredScalarTy::F32, BinOp::Eq) => "oeq",
        (LoweredScalarTy::F32, BinOp::Ne) => "one",
        (LoweredScalarTy::F32, BinOp::Lt) => "olt",
        (LoweredScalarTy::F32, BinOp::Gt) => "ogt",
        (LoweredScalarTy::F32, BinOp::Le) => "ole",
        (LoweredScalarTy::F32, BinOp::Ge) => "oge",
        // u32 无符号比较谓词(MR-0006/RXS-0181:无符号语义由指令侧承载)。
        (LoweredScalarTy::U32, BinOp::Eq) => "eq",
        (LoweredScalarTy::U32, BinOp::Ne) => "ne",
        (LoweredScalarTy::U32, BinOp::Lt) => "ult",
        (LoweredScalarTy::U32, BinOp::Gt) => "ugt",
        (LoweredScalarTy::U32, BinOp::Le) => "ule",
        (LoweredScalarTy::U32, BinOp::Ge) => "uge",
        (LoweredScalarTy::I32 | LoweredScalarTy::I64, BinOp::Eq) => "eq",
        (LoweredScalarTy::I32 | LoweredScalarTy::I64, BinOp::Ne) => "ne",
        (LoweredScalarTy::I32 | LoweredScalarTy::I64, BinOp::Lt) => "slt",
        (LoweredScalarTy::I32 | LoweredScalarTy::I64, BinOp::Gt) => "sgt",
        (LoweredScalarTy::I32 | LoweredScalarTy::I64, BinOp::Le) => "sle",
        (LoweredScalarTy::I32 | LoweredScalarTy::I64, BinOp::Ge) => "sge",
        _ => unreachable!(),
    }
}

// ===========================================================================
// 图形=B 路:stage 分发 + B 链接线(G2.2 PR-D2 分片 2/3,RXS-0161/0162;任务4)。
//
// 分发规则(按 `body.stage`,RFC-0004 §4.1):
//   None(host / compute via kernel) → A 路 [`emit_dxil_ir`](RXS-0157,完全不改)。
//   Some(Vertex|Fragment)           → B 路 [`emit_dxil_b`](本任务新增)。
//   Some(Mesh|Task|RayGen|...)       → STUB(RD-012)「暂不支持」显式 6xxx 停手。
//
// B 链(本任务到 `parse_dxil_signatures` 产出 [`DxilSignatures`] 为止):
//   dxil_spirv::emit_spirv(stage,&io_sig) -> Vec<u32>          (任务2)
//     └─ 写临时 .spv(u32 小端字节,纯 safe)
//        └─ toolchain::spirv_cross_to_hlsl(..) -> HLSL          (分片1)
//           └─ toolchain::dxc_hlsl_to_dxil(..) -> DXIL 容器      (分片1)
//              └─ toolchain::dxc_disasm(..) -> 反汇编文本         (分片1)
//                 └─ toolchain::parse_dxil_signatures(text) -> DxilSignatures
//                    └─ // TODO(task 5): signature_gate::check(..)(校验门接缝)
//
// strict-only(R6.1):B 链任一**语言层**失败(编码器不可映射 / 工具运行后拒绝)
//   → 6xxx,禁止静默 fallback/降级。**工具链缺失**(定位失败 / spawn 失败)→ SKIP
//   (非 6xxx,环境降级,对齐 RXS-0073 ptxas 干验证 / RXS-0157 validator SKIP)。
//
// 🔒 禁区(R1.10 / R6.3):B 路输入 `io_sig`(`MirIoType` 仅标量/向量)结构上无法
//   表达资源句柄/描述符/采样器,故纹理访问语义(描述符编码 / 采样 opcode / 缓存 /
//   LOD / 导数 / 越界)在本层不可达;一旦未来类型面扩展触及,`emit_spirv` 将在映射
//   处发 [`DxilError::Unmappable`] 并标「需升档」,本层只透传、不发明 lowering /
//   ABI 二进制布局 / UB 契约(RFC-0004 §4.6)。
// ===========================================================================

/// stage 分发路由(任务4分发点的判定结果)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageRoute {
    /// `None`(host / compute via kernel)→ A 路 [`emit_dxil_ir`](RXS-0157,不改)。
    PathA,
    /// `Some(Vertex|Fragment)`→ B 路 [`emit_dxil_b`]。
    PathB(ShaderStage),
    /// `Some(Mesh|Task|RayGen|ClosestHit|AnyHit|Miss)`→ STUB(RD-012)「暂不支持」。
    Stub(ShaderStage),
}

/// 按 `stage` 分发 codegen 路由(RFC-0004 §4.1;R6.7 A 路零漂移)。
fn classify_stage(stage: Option<ShaderStage>) -> StageRoute {
    match stage {
        // 非着色阶段(host / compute via kernel,kernel 入口 stage 常为 None)→ A 路。
        None => StageRoute::PathA,
        // compute 着色阶段亦走 A 路(D-131 compute=A);防御性归 A,保 A 路零漂移。
        Some(ShaderStage::Compute) => StageRoute::PathA,
        // 图形着色阶段 → B 路。
        Some(s @ (ShaderStage::Vertex | ShaderStage::Fragment)) => StageRoute::PathB(s),
        // mesh/task/RT 等 → STUB(RD-012)(registry 落条目归任务15/owner;本层 stub 接缝)。
        Some(s) => StageRoute::Stub(s),
    }
}

/// B 路产出(B 链译后签名 + host 侧推导的 RTS0 root signature 容器字节)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DxilBOutcome {
    /// B 链跑通,得译后签名(任务5 `signature_gate::check` 的意图比对输入)+ host 侧
    /// 绑定布局推导序列化出的 RTS0 root signature 容器字节(RXS-0165;PR-E2b 生产
    /// 接线,供 device PSO 创建消费,G-G2-3)。
    Produced {
        /// B 链译后签名(ISG1/OSG1)。
        sigs: DxilSignatures,
        /// host 侧推导序列化的 RTS0 root signature 容器(确定性;非 stable ABI)。
        root_signature: Vec<u8>,
    },
    /// 工具链不可用(定位失败 / spawn 失败 / 临时文件失败)→ SKIP(非 6xxx,
    /// 环境降级,对齐 RXS-0073);携带 SKIP 原因供 note 展示。
    Skipped(String),
}

/// 统一的 B 路编译产物包。
///
/// 单次 B 链执行可同时拿到译后签名、host 侧推导的 root signature、经校验门接受的
/// dumpbin 反汇编文本与 DXIL 容器字节。公开包装器按各自用途从此结构取字段,避免
/// 再各自重跑或重写同一条生产链。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxilBArtifacts {
    /// 该次编译对应的图形阶段。
    pub stage: ShaderStage,
    /// B 链译后签名(ISG1/OSG1)。
    pub sigs: DxilSignatures,
    /// host 侧推导序列化的 RTS0 root signature 容器。
    pub root_signature: Vec<u8>,
    /// 经校验门接受的 dumpbin 反汇编文本。
    pub disasm: String,
    /// 经校验门接受的 DXIL 容器字节。
    pub dxil: Vec<u8>,
}

/// 统一 B 路编译入口的结果:成功产统一产物包或因工具链不可用而 SKIP。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DxilBCompileResult {
    /// B 链跑通,产出完整 artifacts。
    Produced(DxilBArtifacts),
    /// 工具链不可用(定位失败 / spawn 失败 / 临时文件失败)→ SKIP(非 6xxx)。
    Skipped(String),
}

/// B 链跑链体内部结果(译后签名 / 文本 / 容器 / SKIP;RTS0 由上层统一装配入
/// [`DxilBArtifacts`])。
enum BChainResult {
    /// B 链跑通,得译后签名 + 经校验门接受的 dumpbin 反汇编文本。
    ///
    /// `disasm` 是 `signature_gate` 实际取数、实际验过的**同一份** dumpbin 产物
    /// (步骤6),不另起手搓链——golden 据此即「校验门所验产物」单一真相源
    /// ([`emit_dxil_b_disasm`])。生产签名/RTS0 出口忽略它,行为零漂移。
    Sigs {
        sigs: DxilSignatures,
        disasm: String,
        /// 经校验门接受的 DXIL 容器字节(`stage.dxil`,dxc 产);供 D3D12 PSO 创建消费
        /// (G2.4 device 真跑,[`emit_dxil_b_container`])。生产签名/RTS0/golden 出口忽略它。
        dxil: Vec<u8>,
    },
    /// 工具链不可用 → SKIP(携带原因)。
    Skipped(String),
}

/// B 路 strict-only 失败(任务7 落码 RX6010~RX6013;G2.3 PR-E2b-2 续接绑定布局推导
/// 失败 RX6013/6015/6016/6017;emit 点见 [`emit_b_error`])。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DxilBError {
    /// MIR→SPIR-V 编码器不可映射(透传任务2 [`DxilError::Unmappable`];含未来纹理访问
    /// 语义触及的 🔒 升档点)→ `RX6013` `codegen.dxil_unmappable`。
    Spirv(DxilError),
    /// B 链外部工具阶段运行后拒绝(spirv-cross / dxc / dumpbin exit != 0)→
    /// `RX6010` `codegen.dxil_b_transpile_failed`。`step` 为失败阶段,`reason` 为工具
    /// 错误串。(工具缺失/spawn 失败为 SKIP,非 6xxx。)
    Toolchain {
        /// 失败的 B 链阶段名(诊断用)。
        step: String,
        /// 工具错误串(诊断用)。
        reason: String,
    },
    /// 强制签名一致性校验门拒绝 → `RX6011` `codegen.dxil_sig_mismatch`(输出未保真)/
    /// `RX6012` `codegen.dxil_sig_dropped_input`(声明输入被消除)。honor deferred.json
    /// RX6009=RD-013 故不复用 RX6009。不可裁剪、无旁路(R2.5 / Property 5):校验失败
    /// 的入口绝不返回 `Produced`、绝不产 golden。
    SigGate(signature_gate::SigGateError),
    /// 绑定布局推导失败(RXS-0163~0166;G2.3 PR-E2b-2 按变体专属落码,
    /// [`emit_b_error`] 分派):`Unmappable` → `RX6013` `codegen.dxil_unmappable`
    /// (bindless / unbounded descriptor array RD-018 defer,复用既有不可映射码,owner
    /// 已裁不新开);`RegisterConflict` → `RX6015` `codegen.dxil_register_conflict`;
    /// `RootSignatureTooLarge` → `RX6016` `codegen.dxil_root_signature_too_large`;
    /// `Psv0Mismatch` → `RX6017` `codegen.dxil_psv0_mismatch`。strict-only,无运行期
    /// fallback。🔒 诊断只描述失败类别,不落 register/space/packing 物理布局值。
    Binding(binding_layout::BindingInferError),
}

impl std::fmt::Display for DxilBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DxilBError::Spirv(e) => write!(f, "MIR→SPIR-V 不可映射: {e}"),
            DxilBError::Toolchain { step, reason } => {
                write!(f, "B 链 {step} 转译失败: {reason}")
            }
            DxilBError::SigGate(e) => write!(f, "{e}"),
            DxilBError::Binding(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DxilBError {}

/// 创建一个唯一的临时工作目录(进程 id + 纳秒戳;清理由调用方 `remove_dir_all`)。
fn scratch_dir() -> std::io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "rurix_dxil_b_codegen_{}_{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 区分 B 链工具失败语义(strict-only 的 SKIP↔6xxx 判定边界):
/// - **spawn 失败**(工具实际不存在 / 不可执行)= SKIP(环境问题,非 6xxx,对齐
///   RXS-0073 ptxas 干验证纪律)。分片1 驱动以 `cannot spawn` 前缀标记 spawn 失败。
/// - **工具运行后拒绝**(exit != 0)= B 链转译失败 → 6xxx(strict-only,R6.1)。
///
/// (分片1 工具链驱动只读复用、勿改,故据其错误串前缀判别 spawn↔exit。)
fn classify_tool_failure(step: &str, reason: String) -> Result<BChainResult, DxilBError> {
    if reason.contains("cannot spawn") {
        Ok(BChainResult::Skipped(format!(
            "{step} 不可执行(spawn 失败): {reason}"
        )))
    } else {
        Err(DxilBError::Toolchain {
            step: step.to_owned(),
            reason,
        })
    }
}

/// 统一校验图形=B body lowering 支持的阶段,并复用既有不可映射错误形态。
fn graphics_body_stage(body: &Body) -> Result<ShaderStage, DxilBError> {
    match body.stage {
        Some(stage @ (ShaderStage::Vertex | ShaderStage::Fragment)) => Ok(stage),
        _ => Err(DxilBError::Spirv(DxilError::Unmappable {
            what: "stage".to_owned(),
            detail: format!("Body stage {:?} 不在图形=B body lowering 范围", body.stage),
        })),
    }
}

/// 图形=B 链对 dxc profile 的阶段映射。
fn dxil_b_profile(stage: ShaderStage) -> Option<&'static str> {
    match stage {
        ShaderStage::Vertex => Some("vs_6_0"),
        ShaderStage::Fragment => Some("ps_6_0"),
        _ => None,
    }
}

/// 从 vertex 阶段 I/O 意图签名导出 spirv-cross **顶点输入**语义保名旗标
/// (`--set-hlsl-vertex-input-semantic <location> <semantic>`,RFC-0004 §4.4 机制①)。
///
/// **机制(实测,贴 evidence/dxil_b_strict_only_report.md §3 + 本任务报告)**:spirv-cross
/// HLSL 后端默认把顶点输入语义按 location 写为通用 `TEXCOORD#`;`--set-hlsl-vertex-input-
/// semantic <location> <semantic>` 按 **location** 覆盖回用户语义名。[`dxil_spirv::emit_spirv`]
/// 对 Input 方向 varying/interpolate **按 io_sig 顺序递增分配 `Location`**(builtin 取
/// `BuiltIn` 装饰、**不**占 location),故此处按同一顺序复算 `location → field_name` 映射,
/// **经 io_sig 导出、非硬编码**(与 `emit_io_elem` 的 `next_in_location` 严格对齐)。
///
/// 实测要点:spirv-cross **不**消费 SPIR-V `UserSemantic` 装饰为 HLSL 语义(机制是
/// **location**,非 UserSemantic);`--set-hlsl-named-vertex-input-semantic` 按变量
/// `OpName` 匹配,而 `emit_spirv` 不 emit `OpName`,故按 location 覆盖是 Rust-emit SPIR-V
/// 路径下可复现的保名通道(本机 dxc 1.8.0.4739 / spirv-cross vulkan-sdk 实测 ISG1
/// `POSITION`/`NORMAL` 存活、不退化)。
///
/// **边界(实测)**:本机制仅覆盖 **vertex 阶段输入**用户语义名(机制①,按 location
/// 覆盖旗标)。**输出 varying** 与 **fragment 输入 varying** 的保名经 **RXS-0172**
/// (选项① HLSL 边界改写,[`restore_varying_semantics`])在 spirv-cross 产 HLSL 与 dxc
/// 之间复原(spirv-cross HLSL 后端无输出/片元语义旗标、不消费 UserSemantic);保名失败
/// 仍经 strict-only 校验门 **RX6011** 拒(不放宽门,P-01 / Property 5)。
fn vertex_input_semantic_flags(stage: ShaderStage, io_sig: &[IoSigElem]) -> Vec<String> {
    if stage != ShaderStage::Vertex {
        // fragment 输入 varying 不经顶点输入旗标(spirv-cross 无片元输入语义旗标);其保名
        // 由 RXS-0172 `restore_varying_semantics` 在 HLSL 边界复原(RD-017)。
        return Vec::new();
    }
    let mut flags = Vec::new();
    let mut location: u32 = 0;
    for elem in io_sig {
        if !matches!(elem.dir, IoDir::In) {
            continue;
        }
        match &elem.kind {
            // builtin 输入取 BuiltIn 装饰、**不**占 location(对齐 emit_spirv::emit_io_elem)。
            IoSigKind::Builtin(_) => {}
            // 非 builtin 输入按 io_sig 顺序占 location;有用户语义名 → emit 保名旗标。
            IoSigKind::Varying | IoSigKind::Interpolate(_) => {
                if !elem.field_name.is_empty() {
                    flags.push("--set-hlsl-vertex-input-semantic".to_owned());
                    flags.push(location.to_string());
                    flags.push(elem.field_name.clone());
                }
                location += 1;
            }
        }
    }
    flags
}

/// RXS-0172:输出 varying / fragment 输入 varying 用户语义名保名(选项①,HLSL 边界改写)。
///
/// 在 spirv-cross 产 HLSL 与 dxc 之间施加:把退化为通用 `TEXCOORD<location>` 的 varying
/// 语义 token 按 `io_sig` 的 **location→用户语义名** provenance 改回用户名。provenance
/// 与 [`vertex_input_semantic_flags`] / [`dxil_spirv::emit_spirv`] **同源**(varying 按
/// 方向各自递增 `Location`,`#[builtin]` 不占 location)。`dir == In` 映射 spirv-cross
/// 输入 struct(entry 形参类型),`dir == Out` 映射输出 struct(entry 返回类型)。
///
/// **边界(RXS-0172 L3,ABI 中立)**:只替换 HLSL struct field 的 semantic token,不动
/// 类型 / 字段名 / 行结构 / 寄存器 packing,不冻结 register/mask/packing/byte layout/
/// 稳定 `Location`。
///
/// **fail-closed(RXS-0172 L4)**:仅在 provenance 明确(目标 struct 内存在对应
/// `TEXCOORD<loc>` field)时改写;不匹配则保留退化名,经末端 `signature_gate` RX6011
/// 拒(不放宽门,Property 5;RXS-0172 L2)。vertex 阶段输入经机制①(顶点输入保名旗标)
/// 已非 `TEXCOORD#`,本改写对其 In 方向自然 no-op。
fn restore_varying_semantics(io_sig: &[IoSigElem], hlsl: &str) -> String {
    let structs = collect_struct_names(hlsl);
    let (in_struct, out_struct) = find_entry_io_structs(hlsl, &structs);
    let mut text = hlsl.to_string();
    if let Some(s) = in_struct {
        text = rewrite_struct_varyings(&text, &s, &varying_provenance(io_sig, true));
    }
    if let Some(s) = out_struct {
        text = rewrite_struct_varyings(&text, &s, &varying_provenance(io_sig, false));
    }
    text
}

/// 按方向(`want_in`:In/Out)导出 varying 的 location→用户语义名 provenance。
/// 与 [`emit_io_elem`](dxil_spirv) 同源:builtin 不占 location,varying/interpolate
/// 按方向各自从 0 递增;空 `field_name` 不参与(无可恢复名)。
fn varying_provenance(io_sig: &[IoSigElem], want_in: bool) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut location: u32 = 0;
    for elem in io_sig {
        if matches!(elem.dir, IoDir::In) != want_in {
            continue;
        }
        match &elem.kind {
            IoSigKind::Builtin(_) => {}
            IoSigKind::Varying | IoSigKind::Interpolate(_) => {
                if !elem.field_name.is_empty() {
                    out.push((location, elem.field_name.clone()));
                }
                location += 1;
            }
        }
    }
    out
}

/// 收集 HLSL 顶层 `struct <Name>` 声明名(供 entry I/O struct 识别)。
fn collect_struct_names(hlsl: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in hlsl.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("struct ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.push(name);
            }
        }
    }
    names
}

/// 解析 entry(` main(`)签名,识别输入 struct(形参类型)与输出 struct(返回类型)。
/// 返回 `(输入 struct, 输出 struct)`;非 struct(如 PS 直返 `float4 : SV_Target`)为 `None`。
fn find_entry_io_structs(hlsl: &str, structs: &[String]) -> (Option<String>, Option<String>) {
    let known = |t: &str| structs.iter().any(|s| s == t);
    for line in hlsl.lines() {
        let Some(mpos) = line.find(" main(") else {
            continue;
        };
        let out_struct = line[..mpos]
            .split_whitespace()
            .last()
            .map(str::to_owned)
            .filter(|t| known(t));
        let mut in_struct = None;
        if let Some(op) = line[mpos..].find('(') {
            let start = mpos + op + 1;
            if let Some(cp) = line[start..].find(')') {
                let params = &line[start..start + cp];
                for tok in params.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
                    if !tok.is_empty() && known(tok) {
                        in_struct = Some(tok.to_owned());
                        break;
                    }
                }
            }
        }
        return (in_struct, out_struct);
    }
    (None, None)
}

/// 在指定 struct 块内,把 field 的 `TEXCOORD<loc>` 语义按 provenance 改回用户名。
/// 只动 semantic token(`:` 后的标识符),前缀(类型/字段名/冒号)与后缀(`;`/packing)
/// 逐字节保留(RXS-0172 L3 ABI 中立)。
fn rewrite_struct_varyings(hlsl: &str, struct_name: &str, prov: &[(u32, String)]) -> String {
    if prov.is_empty() {
        return hlsl.to_owned();
    }
    let header = format!("struct {struct_name}");
    let mut in_block = false;
    let mut out_lines: Vec<String> = Vec::new();
    for line in hlsl.lines() {
        let trimmed = line.trim_start();
        if !in_block {
            let is_header = trimmed.strip_prefix(&header).is_some_and(|tail| {
                tail.chars()
                    .next()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
            });
            if is_header {
                in_block = true;
            }
            out_lines.push(line.to_owned());
            continue;
        }
        if trimmed.starts_with('}') {
            in_block = false;
            out_lines.push(line.to_owned());
            continue;
        }
        out_lines.push(rewrite_field_semantic(line, prov));
    }
    let mut s = out_lines.join("\n");
    if hlsl.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// 改写单个 field 行的 semantic token:若 `:` 后语义为 `TEXCOORD<loc>` 且 provenance
/// 命中,替换为用户名;否则原样返回(fail-closed,RXS-0172 L4)。
fn rewrite_field_semantic(line: &str, prov: &[(u32, String)]) -> String {
    let Some(colon) = line.rfind(':') else {
        return line.to_owned();
    };
    let after = &line[colon + 1..];
    let lead = after.len() - after.trim_start().len();
    let sem_start = colon + 1 + lead;
    let rest = &line[sem_start..];
    let sem_len = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .count();
    if sem_len == 0 {
        return line.to_owned();
    }
    let sem = &rest[..sem_len];
    let digits = match sem
        .strip_prefix("TEXCOORD")
        .or_else(|| sem.strip_prefix("texcoord"))
    {
        Some(d) => d,
        None => return line.to_owned(),
    };
    let Ok(loc) = digits.parse::<u32>() else {
        return line.to_owned();
    };
    for (l, field) in prov {
        if *l == loc {
            let mut s = String::with_capacity(line.len());
            s.push_str(&line[..sem_start]);
            s.push_str(field);
            s.push_str(&line[sem_start + sem_len..]);
            return s;
        }
    }
    line.to_owned()
}

/// B 链跑链体(步骤 3~7):写临时 `.spv` → spirv-cross → dxc → dumpbin →
/// `parse_dxil_signatures`。临时目录由调用方 [`emit_dxil_b`] 创建并统一清理。
#[allow(clippy::too_many_arguments)] // B 链参数面(spv/工具/profile/io_sig/extra/stage)各为不同关注点,聚合为 struct 反损可读性。
fn run_b_chain(
    spv: &[u32],
    spvx: &Path,
    dxc: &Path,
    profile: &str,
    dir: &Path,
    io_sig: &[IoSigElem],
    extra: &[String],
    stage: ShaderStage,
) -> Result<BChainResult, DxilBError> {
    // 3) 写临时 `.spv`:`&[u32]` 小端 → `&[u8]`(纯 safe,`u32::to_le_bytes`,R1.11)。
    let spv_path = dir.join("stage.spv");
    let mut bytes = Vec::with_capacity(spv.len() * 4);
    for w in spv {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    if let Err(e) = std::fs::write(&spv_path, &bytes) {
        return Ok(BChainResult::Skipped(format!("写临时 .spv 失败: {e}")));
    }

    // 4) spirv-cross:SPIR-V → HLSL(SM 6.0)。`extra` = 顶点输入语义保名旗标
    //    (`--set-hlsl-vertex-input-semantic <loc> <semantic>`,经 io_sig 导出,
    //    [`vertex_input_semantic_flags`];RFC-0004 §4.4 机制①,实测顶点输入名存活)。
    let hlsl_path = dir.join("stage.hlsl");
    if let Err(e) = toolchain::spirv_cross_to_hlsl(spvx, &spv_path, &hlsl_path, 60, extra) {
        return classify_tool_failure("spirv-cross", e);
    }

    // 4.5) RXS-0172 输出 varying / fragment 输入 varying 用户语义名保名(选项①):在
    //     spirv-cross 产 HLSL 与 dxc 之间,按 io_sig location provenance 把退化的
    //     `TEXCOORD#` 改回用户语义名(`restore_varying_semantics`,与机制① 同源 provenance)。
    //     只动 semantic token(ABI 中立,RXS-0172 L3);fail-closed(provenance 不明确不改写,
    //     RXS-0172 L4);保名失败由步骤 8 signature_gate RX6011 闭合、不放宽门(Property 5)。
    match std::fs::read_to_string(&hlsl_path) {
        Ok(src) => {
            let restored = restore_varying_semantics(io_sig, &src);
            if restored != src
                && let Err(e) = std::fs::write(&hlsl_path, restored.as_bytes())
            {
                return Ok(BChainResult::Skipped(format!("写回保名 HLSL 失败: {e}")));
            }
        }
        Err(e) => return Ok(BChainResult::Skipped(format!("读回译 HLSL 失败: {e}"))),
    }

    // 5) dxc:HLSL → DXIL 容器(profile vs_6_0/ps_6_0,entry "main")。
    let dxil_path = dir.join("stage.dxil");
    if let Err(e) = toolchain::dxc_hlsl_to_dxil(dxc, &hlsl_path, profile, "main", &dxil_path) {
        return classify_tool_failure("dxc", e);
    }

    // 6) dxc -dumpbin:DXIL → 反汇编文本(`dxc_disasm` 吃 dxc **所在目录**,内部
    //    join dxc.exe;故由 dxc 可执行本体取 `.parent()`)。
    let dxc_dir = dxc.parent().map(Path::to_path_buf).unwrap_or_default();
    let disasm = match toolchain::dxc_disasm(&dxc_dir, &dxil_path) {
        Ok(d) => d,
        Err(e) => return classify_tool_failure("dxc -dumpbin", e),
    };

    // 7) 解析 DXIL ISG1/OSG1 签名 part(校验门取数基础)。
    let sigs = toolchain::parse_dxil_signatures(&disasm);

    // 8) 强制签名一致性校验门(任务5,不可裁剪 / 无旁路,R2.5 / Property 5):
    //    比对译后签名与 MIR 意图签名(用户语义名 / 系统值 / 被用输入 / 阶段间
    //    location 链接键),缺失 / 改名 / 错配 / 「声明但未用输入被消除」→ strict-only
    //    失败 → 6xxx,**终止该入口产物**(不返回 Produced、不产 golden)。
    //    fragment 输出 varying 按 RXS-0173 以 SV_Target# 渲染目标系统值忠实核对(阶段
    //    上下文 `stage`),vertex 输出维持 RXS-0172 用户语义名保名。
    signature_gate::check_with_stage(&sigs, io_sig, stage).map_err(DxilBError::SigGate)?;

    // 9) 回读经校验门接受的 DXIL 容器字节(步骤5 dxc 产 `stage.dxil`),供 device
    //    PSO 创建消费(G2.4)。读失败按环境降级 SKIP(非 6xxx;校验门已过故文件应在)。
    let dxil = match std::fs::read(&dxil_path) {
        Ok(b) => b,
        Err(e) => {
            return Ok(BChainResult::Skipped(format!(
                "读回 DXIL 容器字节失败: {e}"
            )));
        }
    };

    Ok(BChainResult::Sigs { sigs, disasm, dxil })
}

/// B 路 codegen:着色阶段(`stage` ∈ {Vertex, Fragment})+ I/O 意图签名(`io_sig`)
/// 与资源句柄绑定(`resources`)→ MIR→SPIR-V(含资源 `DescriptorSet`/`Binding` 装饰)
/// →spirv-cross→dxc→dumpbin→`parse_dxil_signatures`→`signature_gate::check`
/// → [`DxilSignatures`] 与 host 侧推导序列化的 RTS0 root signature 容器
/// ([`DxilBOutcome::Produced`])。
///
/// 强制签名一致性校验门(`signature_gate::check`,任务5)在 B 链末尾(步骤8)运行,
/// 不可裁剪、无旁路:译后签名与 `io_sig` 不一致 → strict-only 失败,绝不返回
/// [`DxilBOutcome::Produced`]。
///
/// RTS0 推导(RXS-0165;PR-E2b 生产接线):`binding_layout::infer_root_signature`
/// → `serialize_rts0`,纯 host 推导(工具链无关);`emit_spirv` 已先拒 bindless/
/// unbounded(RD-018),故生产侧资源(`Texture2D<F>`/`Sampler`)恒可推导。
///
/// # Errors
/// - 编码器不可映射构造(非 vertex·fragment 阶段 / 不可映射类型 / 未建模 builtin /
///   builtin 类型不符 / 越界向量宽度 / bindless 资源)→ [`DxilBError::Spirv`]
///   (strict-only,6xxx)。
/// - B 链外部工具运行后拒绝 → [`DxilBError::Toolchain`](6xxx)。
/// - 签名一致性校验门拒绝(语义名 / 系统值未保真 / 声明输入被消除)→
///   [`DxilBError::SigGate`](6xxx,任务5)。
///
/// 工具链不可用(定位失败 / spawn 失败 / 临时文件失败)→ `Ok(`[`DxilBOutcome::Skipped`]`)`
/// (非 6xxx,环境降级,真实红绿在带工具链的 dev/owner 环境)。
pub fn emit_dxil_b(
    stage: ShaderStage,
    io_sig: &[IoSigElem],
    resources: &[ResourceBinding],
) -> Result<DxilBOutcome, DxilBError> {
    // 1) MIR→SPIR-V(任务2 编码器 + RXS-0163 资源绑定装饰);不可映射 → 透传 6xxx
    //    (strict-only,不静默降级)。资源句柄绑定的 `DescriptorSet`/`Binding` 由
    //    host 侧 `binding_layout::infer_spirv_bindings` 确定性推导(见 emit_spirv)。
    let spv = dxil_spirv::emit_spirv(stage, io_sig, resources).map_err(DxilBError::Spirv)?;
    match compile_dxil_b_from_spv(stage, io_sig, resources, spv)? {
        DxilBCompileResult::Produced(artifacts) => Ok(DxilBOutcome::Produced {
            sigs: artifacts.sigs,
            root_signature: artifacts.root_signature,
        }),
        DxilBCompileResult::Skipped(why) => Ok(DxilBOutcome::Skipped(why)),
    }
}

/// B 路统一生产入口:消费完整 MIR [`Body`] 并按 RXS-0171 降级图形 body I/O 数据流,
/// 返回完整 [`DxilBArtifacts`] 或工具链 SKIP。
///
/// 现有 `emit_dxil_b_body` / `emit_dxil_b_disasm` / `emit_dxil_b_container` 都应经此
/// 单一真相源取结果,避免重复编排阶段校验、SPIR-V 降级、profile 选择、工具链定位与
/// 临时目录生命周期。
pub fn compile_dxil_b_body(body: &Body) -> Result<DxilBCompileResult, DxilBError> {
    let stage = graphics_body_stage(body)?;
    let spv = dxil_spirv::emit_spirv_body(stage, body).map_err(DxilBError::Spirv)?;
    compile_dxil_b_from_spv(stage, &body.io_sig, &body.resources, spv)
}

/// B 路签名/root-signature 兼容包装器:公开语义保持不变,内部改为复用
/// [`compile_dxil_b_body`] 的统一产物编译入口。
pub fn emit_dxil_b_body(body: &Body) -> Result<DxilBOutcome, DxilBError> {
    match compile_dxil_b_body(body)? {
        DxilBCompileResult::Produced(artifacts) => Ok(DxilBOutcome::Produced {
            sigs: artifacts.sigs,
            root_signature: artifacts.root_signature,
        }),
        DxilBCompileResult::Skipped(why) => Ok(DxilBOutcome::Skipped(why)),
    }
}

fn compile_dxil_b_from_spv(
    stage: ShaderStage,
    io_sig: &[IoSigElem],
    resources: &[ResourceBinding],
    spv: Vec<u32>,
) -> Result<DxilBCompileResult, DxilBError> {
    // emit_spirv 成功即保证 stage ∈ {Vertex, Fragment};据此取 dxc profile。
    let profile = match dxil_b_profile(stage) {
        Some(profile) => profile,
        // 不可达(非图形阶段已在 emit_spirv 被拒);防御性 SKIP,不 panic。
        None => return Ok(DxilBCompileResult::Skipped("非图形阶段(不可达)".to_owned())),
    };

    // 1b) root signature 形态推导 + RTS0 容器序列化(RXS-0165;纯 host,工具链无关)。
    //     emit_spirv 已先拒 bindless/unbounded → 生产侧资源恒可推导。
    let root_signature = serialize_root_signature(resources)?;

    // 2) 工具链定位:缺失 → SKIP(非 6xxx,环境降级)。
    let Some(spvx) = toolchain::locate_spirv_cross() else {
        return Ok(DxilBCompileResult::Skipped(
            "spirv-cross 不可定位".to_owned(),
        ));
    };
    let Some(dxc) = toolchain::locate_dxc() else {
        return Ok(DxilBCompileResult::Skipped("dxc 不可定位".to_owned()));
    };

    // 顶点输入语义保名旗标(经 io_sig 导出,非硬编码;RFC-0004 §4.4 机制①,实测)。
    // fragment / 无命名输入 → 空(behavior 不变)。
    let extra = vertex_input_semantic_flags(stage, io_sig);

    // 3~7) 临时工作目录内跑链;无论成败统一清理。
    let dir = match scratch_dir() {
        Ok(d) => d,
        Err(e) => {
            return Ok(DxilBCompileResult::Skipped(format!(
                "临时目录创建失败: {e}"
            )));
        }
    };
    let result = run_b_chain(&spv, &spvx, &dxc, profile, &dir, io_sig, &extra, stage);
    let _ = std::fs::remove_dir_all(&dir);
    match result {
        Ok(BChainResult::Sigs { sigs, disasm, dxil }) => {
            Ok(DxilBCompileResult::Produced(DxilBArtifacts {
                stage,
                sigs,
                root_signature,
                disasm,
                dxil,
            }))
        }
        Ok(BChainResult::Skipped(why)) => Ok(DxilBCompileResult::Skipped(why)),
        Err(e) => Err(e),
    }
}

/// 生产忠实 B 链反汇编(golden 单一真相源;RXS-0162 golden + RXS-0171 body 降级 +
/// RXS-0172 varying 保名)。
///
/// 驱动与 [`emit_dxil_b_body`] **同一条**生产链:`emit_spirv_body`(RXS-0171 入口
/// body I/O 数据流降级)→ 顶点输入保名旗标([`vertex_input_semantic_flags`])→
/// RXS-0172 输出/片元 varying 用户语义名保名([`restore_varying_semantics`],在
/// `run_b_chain` 内 HLSL 边界)→ dxc → dumpbin → `parse_dxil_signatures` → 强制
/// `signature_gate::check`。返回**经校验门接受**的规范化反汇编文本。
///
/// golden 比对此返回值,故 golden 字节 = 校验门所验产物本身——不再有第二条手搓
/// 链(签名-only `emit_spirv` + 空旗标 + 跳过保名)与生产链漂移。规范化仅抹平
/// 工具版本噪声行(shader hash / dxc ident),不动语言相关结构。
///
/// # Returns
/// - `Ok(Some(disasm))`:工具链可用、链跑通且校验门通过。
/// - `Ok(None)`:工具链不可用(spirv-cross / dxc 定位或 spawn 失败 / 临时目录失败)
///   → 环境降级(非 6xxx;真实红绿在带 pin B 工具链的 dev/owner 环境)。
///
/// # Errors
/// 同 [`emit_dxil_b_body`]:不可映射构造 / 工具运行后拒绝 / 校验门拒绝 → 6xxx,
/// 绝不静默成功。
pub fn emit_dxil_b_disasm(body: &Body) -> Result<Option<String>, DxilBError> {
    match compile_dxil_b_body(body)? {
        DxilBCompileResult::Produced(artifacts) => Ok(Some(normalize_b_disasm(&artifacts.disasm))),
        DxilBCompileResult::Skipped(_) => Ok(None),
    }
}

/// B 路生产入口:消费完整 MIR [`Body`] 产出**经校验门接受的 DXIL 容器字节**(供 D3D12
/// PSO 创建消费;G2.4 UC-04 device 真跑)。与 [`emit_dxil_b_body`] / [`emit_dxil_b_disasm`]
/// **同一条**生产链:`emit_spirv_body`(RXS-0171 body I/O 数据流降级)→ 顶点输入保名旗标
/// → RXS-0172 输出/片元输入 varying 用户语义名保名 → dxc(产 DXIL 容器)→ dumpbin →
/// `parse_dxil_signatures` → 强制 `signature_gate::check_with_stage`(RXS-0173 fragment
/// 输出 SV_Target# 忠实核对)。返回的字节 = 校验门所验产物本身(无第二条手搓链)。
///
/// **G-G2-4 防降级**:device PSO 消费的 DXIL 来自本入口(Rurix 源经图形=B 链),非手写
/// HLSL/DXIL;校验门失败的入口绝不返回字节(`?` 终止落 6xxx)。
///
/// # Returns
/// - `Ok(Some(dxil))`:工具链可用、链跑通且校验门通过 → DXIL 容器字节。
/// - `Ok(None)`:工具链不可用(spirv-cross / dxc 定位或 spawn 失败 / 临时目录失败)→
///   环境降级(非 6xxx;真实产出在带 pin B 工具链的 dev/CI 环境)。
///
/// # Errors
/// 同 [`emit_dxil_b_body`]:不可映射构造 / 工具运行后拒绝 / 校验门拒绝 → 6xxx,绝不
/// 静默成功。
pub fn emit_dxil_b_container(body: &Body) -> Result<Option<Vec<u8>>, DxilBError> {
    match compile_dxil_b_body(body)? {
        DxilBCompileResult::Produced(artifacts) => Ok(Some(artifacts.dxil)),
        DxilBCompileResult::Skipped(_) => Ok(None),
    }
}

/// 规范化 dxc 反汇编中的版本噪声行(shader hash 内容/版本派生 + dxc ident 构建串),
/// 使 golden 聚焦语言相关结构(签名表 / 入口 / 着色器类型),不写死工具版本。
fn normalize_b_disasm(s: &str) -> String {
    let mut lines = Vec::new();
    for raw in s.replace("\r\n", "\n").lines() {
        let t = raw.trim_start();
        if t.starts_with("; shader hash:") {
            lines.push("; shader hash: <OWNER-BLESSED-NORMALIZED>".to_owned());
        } else if raw.contains("dxc(private)") || raw.contains("dxcoob ") {
            // 保留 metadata id 前缀(如 `!0 = `),仅规范化版本串。
            let id = raw.split('=').next().unwrap_or("").trim_end();
            lines.push(format!("{id} = !{{!\"dxc <OWNER-BLESSED-NORMALIZED>\"}}"));
        } else {
            lines.push(raw.to_owned());
        }
    }
    lines.join("\n")
}

/// root signature 形态推导 + RTS0 容器序列化(RXS-0165;PR-E2b 生产接线)。
///
/// **G2.3 PR-E2b-2(本片落码)**:绑定推导失败按失败类别经 [`DxilBError::Binding`]
/// 携带真实 [`binding_layout::BindingInferError`] 变体上抛,[`emit_b_error`] 分派专属
/// 码——`Unmappable` → `RX6013`(复用)/ `RegisterConflict` → `RX6015` / `RootSignature
/// TooLarge` → `RX6016` / `Psv0Mismatch` → `RX6017`(替换 E2b-1 经 `RX6013` 一律透传的
/// interim)。`emit_spirv` 在本函数前已先拒 bindless/unbounded(RD-018),生产侧资源
/// (`Texture2D<F>`/`Sampler`,基数 One)恒可推导,故 `Err` 分支当前仍主要作 strict-only
/// 防御(绝不静默产出空 root signature),失败类别的真实红绿由 [`binding_layout`] host
/// 单测 + [`emit_b_error`] 分派单测保证。
fn serialize_root_signature(resources: &[ResourceBinding]) -> Result<Vec<u8>, DxilBError> {
    match binding_layout::infer_root_signature(resources) {
        Ok(rs) => Ok(binding_layout::serialize_rts0(&rs)),
        Err(e) => Err(DxilBError::Binding(e)),
    }
}

/// 单个 device [`Body`] 的 DXIL codegen 分发产出(任务4分发点的整体结果)。
#[derive(Debug)]
pub enum DispatchOutcome {
    /// `None`(compute/kernel)→ A 路 DirectX 三元组 LLVM IR 文本(RXS-0157)。
    PathAIr(String),
    /// Vertex/Fragment → B 路译后签名(任务5校验门接缝输入)+ RTS0 root signature。
    PathBSignatures {
        /// B 链译后签名(ISG1/OSG1)。
        sigs: DxilSignatures,
        /// host 侧推导序列化的 RTS0 root signature 容器(RXS-0165;PR-E2b)。
        root_signature: Vec<u8>,
    },
    /// Vertex/Fragment → B 路工具链 SKIP(非 6xxx;携带原因)。
    SkippedB(String),
    /// 已发诊断(A 路 RX6007 子集外 / B 路 strict-only 6xxx / mesh·task·RT stub 6xxx);
    /// 无产物。
    Diagnosed,
}

/// B 路 strict-only 失败 → 按真实可达类别落 6xxx 结构化诊断(任务7 只追加新码)。
///
/// 编号映射(honor `registry/deferred.json`:RX6008=mesh/task/RT RD-012、
/// RX6009=阶段 I/O body 数据流降级 RD-013,均留给既有引用不改派;本片真实可达类别
/// 自 RX6010 起):
/// - [`DxilBError::Toolchain`] → `RX6010` `codegen.dxil_b_transpile_failed`
///   (spirv-cross / dxc / dumpbin 运行后 exit≠0;工具缺失/spawn 失败为 SKIP 非本码);
/// - [`SigGateError::SigMismatch`] → `RX6011` `codegen.dxil_sig_mismatch`
///   (输出方向用户语义名 / 系统值未保真);
/// - [`SigGateError::SigDroppedInput`] → `RX6012` `codegen.dxil_sig_dropped_input`
///   (声明的外部输入被消除且不可等价保留);
/// - [`DxilBError::Spirv`](`DxilError::Unmappable`)→ `RX6013` `codegen.dxil_unmappable`
///   (MIR→SPIR-V 编码器最小子集外构造);
/// - [`DxilBError::Binding`] → 按 [`binding_layout::BindingInferError`] 变体分派
///   (G2.3 PR-E2b-2):`Unmappable` → `RX6013`(复用,bindless/unbounded RD-018)/
///   `RegisterConflict` → `RX6015` / `RootSignatureTooLarge` → `RX6016` /
///   `Psv0Mismatch` → `RX6017`。
///
/// 🔒 纹理访问语义结构上不可达(`io_sig` 仅标量/向量),**不造码**(R3.6 不预造)。
/// 🔒 绑定布局诊断只描述失败类别,**不**落 register/space/packing 物理布局值。
fn emit_b_error(diag: &DiagCtxt, span: Span, err: &DxilBError) {
    use crate::binding_layout::BindingInferError;
    use crate::dxil_sig_gate::signature_gate::SigGateError;
    match err {
        DxilBError::Toolchain { step, reason } => {
            diag.struct_error(ErrorCode(6010), "codegen.dxil_b_transpile_failed")
                .arg("step", step.clone())
                .arg("reason", reason.clone())
                .span_label(span, "in DXIL graphics entry")
                .emit();
        }
        DxilBError::SigGate(SigGateError::SigMismatch { detail }) => {
            diag.struct_error(ErrorCode(6011), "codegen.dxil_sig_mismatch")
                .arg("detail", detail.clone())
                .span_label(span, "in DXIL graphics entry")
                .emit();
        }
        DxilBError::SigGate(SigGateError::SigDroppedInput { detail }) => {
            diag.struct_error(ErrorCode(6012), "codegen.dxil_sig_dropped_input")
                .arg("detail", detail.clone())
                .span_label(span, "in DXIL graphics entry")
                .emit();
        }
        DxilBError::Spirv(e) => {
            // 采样首期收敛子集外 → RX6023(RXS-0175);其余 MIR→SPIR-V 不可映射 → RX6013。
            match e {
                DxilError::SampleUnsupported { .. } => {
                    diag.struct_error(ErrorCode(6023), "codegen.dxil_sample_unsupported")
                        .arg("detail", e.to_string())
                        .span_label(span, "in DXIL graphics entry")
                        .emit();
                }
                DxilError::Unmappable { .. } => {
                    diag.struct_error(ErrorCode(6013), "codegen.dxil_unmappable")
                        .arg("detail", e.to_string())
                        .span_label(span, "in DXIL graphics entry")
                        .emit();
                }
            }
        }
        DxilBError::Binding(e) => {
            // 绑定布局推导失败按类别分派专属码(RXS-0163~0166;owner 已裁:
            // Unmappable 复用 RX6013,其余新开 RX6015/6016/6017)。诊断载荷只取失败
            // 类别描述(BindingInferError::Display),🔒 不含 register/space 物理值。
            let (code, key) = match e {
                BindingInferError::Unmappable { .. } => (6013, "codegen.dxil_unmappable"),
                BindingInferError::RegisterConflict { .. } => {
                    (6015, "codegen.dxil_register_conflict")
                }
                BindingInferError::RootSignatureTooLarge { .. } => {
                    (6016, "codegen.dxil_root_signature_too_large")
                }
                BindingInferError::Psv0Mismatch { .. } => (6017, "codegen.dxil_psv0_mismatch"),
            };
            diag.struct_error(ErrorCode(code), key)
                .arg("detail", e.to_string())
                .span_label(span, "in DXIL graphics entry")
                .emit();
        }
    }
}

/// 按 `body.stage` 分发 codegen 并落诊断(任务4分发点)。
///
/// - `None`(compute/kernel)→ A 路 [`emit_dxil_ir`](完全不改);成功 →
///   [`DispatchOutcome::PathAIr`],子集外 → RX6007 + [`DispatchOutcome::Diagnosed`]。
/// - `Some(Vertex|Fragment)`→ B 路 [`emit_dxil_b`];产出 →
///   [`DispatchOutcome::PathBSignatures`],SKIP → note +
///   [`DispatchOutcome::SkippedB`],strict-only 失败 → 6xxx +
///   [`DispatchOutcome::Diagnosed`]。
/// - mesh/task/RT 等 → STUB(RD-012)「暂不支持」6xxx + [`DispatchOutcome::Diagnosed`]。
pub fn dispatch_and_emit(diag: &DiagCtxt, body: &Body, module_name: &str) -> DispatchOutcome {
    match classify_stage(body.stage) {
        // ── A 路(compute/kernel):完全复用既有 emit_dxil_ir,零漂移(R6.7)。 ──
        StageRoute::PathA => match emit_dxil_ir(body, module_name) {
            Ok(ir) => DispatchOutcome::PathAIr(ir),
            Err(e) => {
                diag.struct_error(ErrorCode(e.code), e.message_key)
                    .arg("detail", e.detail.clone())
                    .span_label(e.span, "in DXIL compute entry")
                    .emit();
                DispatchOutcome::Diagnosed
            }
        },
        // ── B 路(vertex/fragment):MIR→SPIR-V→…→parse_dxil_signatures。 ──
        StageRoute::PathB(_stage) => match emit_dxil_b_body(body) {
            Ok(DxilBOutcome::Produced {
                sigs,
                root_signature,
            }) => {
                // 校验门已在 B 链内部(run_b_chain 步骤8)强制通过:能到此即译后签名
                // 与 MIR 意图签名一致(用户语义名/系统值/被用输入/链接键保真)。校验
                // 失败的入口绝不到此(已转 Err 分支落 6xxx),Property 5 不旁路由此保证。
                DispatchOutcome::PathBSignatures {
                    sigs,
                    root_signature,
                }
            }
            Ok(DxilBOutcome::Skipped(why)) => {
                eprintln!(
                    "rurixc: note: [SKIP] DXIL B 链工具链不可用({why});转译 + 签名校验 \
                     SKIPPED(开发环境降级,非 6xxx,对齐 RXS-0073;真实红绿在带工具链环境)"
                );
                DispatchOutcome::SkippedB(why)
            }
            Err(e) => {
                emit_b_error(diag, body.span, &e);
                DispatchOutcome::Diagnosed
            }
        },
        // ── STUB(RD-012):mesh/task/RT 着色器类型降级未实现 → 显式 6xxx 停手。 ──
        StageRoute::Stub(stage) => {
            // STUB(RD-012): mesh/task/RT 着色器类型 DXIL 降级 deferred。任务7 核查:
            // registry/deferred.json RD-012 已引用 RX6008 作此类降级码,honor 既有引用
            // 不改派;但 RX6008 的 registry 落条目 + status 翻转归后续里程碑/owner(非
            // 任务7 真实可达类别),故本层暂续用既有 RX6007 通道发显式「暂不支持」
            // 6xxx,不静默降级(strict-only,R6.1)。RX6008 落码后由 owner 改接此点。
            diag.struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
                .arg(
                    "detail",
                    format!(
                        "着色器类型 {stage:?} 暂不支持 DXIL 降级(mesh/task/RT;\
                         STUB(RD-012),待后续里程碑回填)"
                    ),
                )
                .span_label(body.span, "in DXIL graphics entry")
                .emit();
            DispatchOutcome::Diagnosed
        }
    }
}

/// vertex+fragment 多阶段联编点的链接核对结果(RXS-0160 IR2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageLinkOutcome {
    /// 无 vertex+fragment 配对(单阶段编译 / 缺一阶段)→ 无链接核对(behavior 不变,
    /// 单阶段 / A 路零漂移,RXS-0157 R6.7)。
    NoPair,
    /// vertex out ↔ fragment in 链接一致(语义名 / 类型 / 插值全配对)。
    Linked,
    /// 错链(strict-only;经 [`emit_stage_link_error`] 落 `RX6014`
    /// `codegen.dxil_stage_link_mismatch`,agent 裁定方案 B 新开码、不复用 RX6011,
    /// 见 [`signature_gate::StageLinkError`])。
    LinkError(signature_gate::StageLinkError),
}

/// vertex+fragment 多阶段联编点接缝(RXS-0160 IR2):从 device MIR body 集合中收集
/// vertex / fragment 两阶段的 `io_sig`,汇集到链接核对入口
/// [`signature_gate::check_stage_link`]。
///
/// 由单着色阶段编译([`dispatch_and_emit`] 逐 body)扩到 **vertex+fragment 配对**的
/// 链接核对:取首个 vertex 阶段 body 与首个 fragment 阶段 body,以 vertex 输出方向 +
/// fragment 输入方向的 `io_sig` 核实跨阶段 varying 链接键(语义名 / 类型 / 插值)。
/// 无 vertex+fragment 配对(单阶段编译 / 缺一阶段)→ [`StageLinkOutcome::NoPair`]
/// (behavior 不变,零漂移)。
///
/// **错误码(G2.3 PR-E2b-2 已落,agent 裁定方案 B)**:错链返回
/// [`StageLinkOutcome::LinkError`],经 [`emit_stage_link_error`] 落 `RX6014`
/// `codegen.dxil_stage_link_mismatch`——agent 裁定**新开 RX6014**(不复用 RX6011 单阶段
/// 签名不一致语义;spec §2 RXS-0160 IR3)。strict-only 语义由 `check_stage_link` 保证
/// (错链必 Err,绝不静默通过)。
pub fn link_graphics_stages(bodies: &[Body]) -> StageLinkOutcome {
    let vs = bodies
        .iter()
        .find(|b| matches!(b.stage, Some(ShaderStage::Vertex)));
    let fs = bodies
        .iter()
        .find(|b| matches!(b.stage, Some(ShaderStage::Fragment)));
    match (vs, fs) {
        (Some(v), Some(f)) => match signature_gate::check_stage_link(&v.io_sig, &f.io_sig) {
            Ok(()) => StageLinkOutcome::Linked,
            Err(e) => StageLinkOutcome::LinkError(e),
        },
        _ => StageLinkOutcome::NoPair,
    }
}

/// 阶段间接口错链 → `RX6014` `codegen.dxil_stage_link_mismatch` 结构化诊断(RXS-0160;
/// G2.3 PR-E2b-2,agent 裁定方案 B 新开码)。
///
/// 两类 [`signature_gate::StageLinkError`](`Unlinked` 缺链接键 / `LinkMismatch`
/// 类型·插值失配)均落同一 `RX6014`(同属阶段间接口错链失败类别,RXS-0160 L2/L3);
/// strict-only,无运行期 fallback。🔒 诊断只描述错链失败类别,**不**落 location /
/// register / mask 物理布局值(ABI 中立,RFC-0004 §4.6(a))。
pub fn emit_stage_link_error(diag: &DiagCtxt, span: Span, err: &signature_gate::StageLinkError) {
    diag.struct_error(ErrorCode(6014), "codegen.dxil_stage_link_mismatch")
        .arg("detail", err.to_string())
        .span_label(span, "in DXIL graphics stage link")
        .emit();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::UnOp;
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

    /// GRX-009 segment 3a.1:受支持 compute 形参进入离线绑定布局推导。
    //@ spec: RXS-0157
    #[test]
    fn kernel_with_supported_compute_params_emits_artifacts() {
        let src =
            "kernel fn k(src: View<global, f32>, out: ViewMut<global, f32>, t: ThreadCtx<1>) {}\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        let artifacts = build_and_emit_dxil_artifacts(&cx, "k").expect("应产出 DXIL artifact");
        assert!(
            artifacts
                .ir
                .contains("target triple = \"dxil-unknown-shadermodel6.0-compute\"")
        );
        assert!(!artifacts.root_signature.is_empty());
        assert!(
            artifacts
                .descriptor_layout_json
                .contains("\"name\": \"src\"")
        );
        assert!(
            artifacts
                .descriptor_layout_json
                .contains("\"name\": \"out\"")
        );
        assert!(
            artifacts
                .descriptor_layout_json
                .contains("\"root_constants\": 0")
        );
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(
            codes.is_empty(),
            "受支持 compute 形参不应发诊断,实得 {codes:?}"
        );
    }

    /// GRX-009:texture-capable kernel artifact round —— `Texture2D<f32>` /
    /// `RWTexture2D<f32>` 入参产出 DXIL/RTS0/layout 三件套,layout 中 `binding_kind`
    /// 为 `texture2d`/`rwtexture2d`、`class` 为 `t`/`u`,DXIL IR 含**上游** target ext
    /// 拼写 `target("dx.Texture", float, 0, 0, 0, 2)`(SRV)/
    /// `target("dx.Texture", float, 1, 0, 0, 2)`(UAV)与上游/本地 patch intrinsic
    /// `@llvm.dx.resource.load.level` / `@llvm.dx.resource.store.texture`。
    //@ spec: GRX-009 Texture-Capable DXIL Lowering
    #[test]
    fn kernel_with_texture2d_params_emits_artifacts() {
        let src = "kernel fn k(tex: Texture2D<f32>, dst: RWTexture2D<f32>, t: ThreadCtx<1>) {\n    dst[0] = tex[0];\n}\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        let artifacts = build_and_emit_dxil_artifacts(&cx, "k");
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(
            codes.is_empty(),
            "texture-capable kernel 不应发诊断,实得 {codes:?}"
        );
        let artifacts = artifacts.expect("应产出 texture-capable DXIL artifact 三件套");
        // DXIL IR:DirectX 三元组 + texture target types + load/store intrinsics。
        assert!(
            artifacts
                .ir
                .contains("target triple = \"dxil-unknown-shadermodel6.0-compute\"")
        );
        assert!(
            artifacts
                .ir
                .contains(r#"target("dx.Texture", float, 0, 0, 0, 2)"#),
            "DXIL IR 缺上游 Texture2D SRV target ty: {}",
            artifacts.ir
        );
        assert!(
            artifacts
                .ir
                .contains(r#"target("dx.Texture", float, 1, 0, 0, 2)"#),
            "DXIL IR 缺上游 RWTexture2D UAV target ty: {}",
            artifacts.ir
        );
        assert!(
            artifacts.ir.contains("@llvm.dx.resource.load.level("),
            "DXIL IR 缺上游 texture load.level intrinsic: {}",
            artifacts.ir
        );
        assert!(
            artifacts.ir.contains("@llvm.dx.resource.store.texture("),
            "DXIL IR 缺 texture store.texture intrinsic: {}",
            artifacts.ir
        );
        // RTS0:非空(SRV+UAV descriptor table 至少一项)。
        assert!(
            !artifacts.root_signature.is_empty(),
            "RTS0 root signature 应非空"
        );
        // descriptor layout JSON:`binding_kind` + `class` 字段。
        assert!(
            artifacts
                .descriptor_layout_json
                .contains("\"name\": \"tex\""),
            "缺 tex 资源记录: {}",
            artifacts.descriptor_layout_json
        );
        assert!(
            artifacts
                .descriptor_layout_json
                .contains("\"name\": \"dst\""),
            "缺 dst 资源记录: {}",
            artifacts.descriptor_layout_json
        );
        // tex(Texture2D SRV)→ class=t + binding_kind=texture2d。
        let tex_idx = artifacts
            .descriptor_layout_json
            .find("\"name\": \"tex\"")
            .expect("tex 记录已断言存在");
        let tex_slice_end = (tex_idx + 200).min(artifacts.descriptor_layout_json.len());
        let tex_record = &artifacts.descriptor_layout_json[tex_idx..tex_slice_end];
        let tex_end = tex_record.find('}').expect("tex 记录应有 }");
        let tex_record = &tex_record[..tex_end];
        assert!(
            tex_record.contains("\"class\": \"t\""),
            "tex 应 class=t(SRV): {tex_record}"
        );
        assert!(
            tex_record.contains("\"binding_kind\": \"texture2d\""),
            "tex 应 binding_kind=texture2d: {tex_record}"
        );
        // dst(RWTexture2D UAV)→ class=u + binding_kind=rwtexture2d。
        let dst_idx = artifacts
            .descriptor_layout_json
            .find("\"name\": \"dst\"")
            .expect("dst 记录已断言存在");
        let dst_slice_end = (dst_idx + 200).min(artifacts.descriptor_layout_json.len());
        let dst_record = &artifacts.descriptor_layout_json[dst_idx..dst_slice_end];
        let dst_end = dst_record.find('}').expect("dst 记录应有 }");
        let dst_record = &dst_record[..dst_end];
        assert!(
            dst_record.contains("\"class\": \"u\""),
            "dst 应 class=u(UAV): {dst_record}"
        );
        assert!(
            dst_record.contains("\"binding_kind\": \"rwtexture2d\""),
            "dst 应 binding_kind=rwtexture2d: {dst_record}"
        );
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(
            codes.is_empty(),
            "texture-capable kernel 不应发诊断,实得 {codes:?}"
        );
    }

    /// GRX-009:`binding_kind` 字段对所有资源种类(View/ViewMut/Texture2D/RWTexture2D)
    /// 均正确输出。spec ADDDED Requirements「Descriptor Layout binding_kind Field」。
    //@ spec: GRX-009 Descriptor Layout binding_kind Field
    #[test]
    fn descriptor_layout_records_binding_kind_for_all_resource_kinds() {
        // View → raw_buffer_view / ViewMut → raw_buffer_view。
        let view_src =
            "kernel fn k(src: View<global, f32>, out: ViewMut<global, f32>) { out[0] = src[0]; }\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(view_src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        let view_artifacts =
            build_and_emit_dxil_artifacts(&cx, "k").expect("View/ViewMut kernel 应产 artifact");
        assert!(
            view_artifacts
                .descriptor_layout_json
                .contains("\"binding_kind\": \"raw_buffer_view\""),
            "View/ViewMut 资源应 binding_kind=raw_buffer_view: {}",
            view_artifacts.descriptor_layout_json
        );
        assert!(
            view_artifacts
                .descriptor_layout_json
                .contains("\"class\": \"t\"")
                && view_artifacts
                    .descriptor_layout_json
                    .contains("\"class\": \"u\""),
            "View/ViewMut 资源应同时含 class=t 和 class=u: {}",
            view_artifacts.descriptor_layout_json
        );

        // Texture2D / RWTexture2D → texture2d / rwtexture2d。
        let tex_src =
            "kernel fn k(tex: Texture2D<f32>, dst: RWTexture2D<f32>) { dst[0] = tex[0]; }\n";
        let diag2 = DiagCtxt::new();
        let cx2 = QueryCtx::new(tex_src, SourceId(0), Edition::Rx0, &diag2);
        cx2.check_crate();
        cx2.check_coloring();
        cx2.check_crate_patterns();
        cx2.check_consteval();
        let tex_artifacts = build_and_emit_dxil_artifacts(&cx2, "k")
            .expect("Texture2D/RWTexture2D kernel 应产 artifact");
        assert!(
            tex_artifacts
                .descriptor_layout_json
                .contains("\"binding_kind\": \"texture2d\""),
            "Texture2D 资源应 binding_kind=texture2d: {}",
            tex_artifacts.descriptor_layout_json
        );
        assert!(
            tex_artifacts
                .descriptor_layout_json
                .contains("\"binding_kind\": \"rwtexture2d\""),
            "RWTexture2D 资源应 binding_kind=rwtexture2d: {}",
            tex_artifacts.descriptor_layout_json
        );
        // 不应再含 raw_buffer_view(纯 texture kernel)。
        assert!(
            !tex_artifacts
                .descriptor_layout_json
                .contains("\"binding_kind\": \"raw_buffer_view\""),
            "纯 texture kernel 不应含 raw_buffer_view: {}",
            tex_artifacts.descriptor_layout_json
        );
    }

    /// scalar root constants 进入 artifact layout,保留 name/type/order/offset/size。
    //@ spec: RXS-0157
    #[test]
    fn kernel_with_scalar_root_constants_emits_layout_artifacts() {
        let src = "kernel fn k(src: View<global, f32>, out: ViewMut<global, f32>, w: usize, gain: f32, t: ThreadCtx<1>) {\n    out[0] = src[0] * gain;\n}\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        let ir =
            build_and_emit_dxil(&cx, "k").expect("含 scalar root constant 应仍产 IR-only evidence");
        // root constants → cbuffer b0:布局类型 + 句柄 + getpointer 读取
        // (w:i64@byte0,gain:f32@byte8 = 密排 dword×4)。
        assert!(ir.contains("%__cblayout_k = type <{ i64, float }>"), "{ir}");
        assert!(
            ir.contains("%rx_cb = call target(\"dx.CBuffer\", %__cblayout_k) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)"),
            "{ir}"
        );
        assert!(
            ir.contains(
                "@llvm.dx.resource.getpointer(target(\"dx.CBuffer\", %__cblayout_k) %rx_cb, i32 8)"
            ),
            "{ir}"
        );
        assert!(ir.contains("target triple = \"dxil-unknown-shadermodel6.0-compute\""));

        let artifacts = build_and_emit_dxil_artifacts(&cx, "k")
            .expect("应产出 scalar root constants artifact layout");
        assert!(!artifacts.root_signature.is_empty());
        for needle in [
            "\"root_constants\": 2",
            "\"name\": \"w\"",
            "\"type\": \"i64\"",
            "\"order\": 0",
            "\"dword_offset\": 0",
            "\"dword_size\": 2",
            "\"name\": \"gain\"",
            "\"type\": \"f32\"",
            "\"order\": 1",
            "\"dword_offset\": 2",
            "\"dword_size\": 1",
            "\"root_signature_parameters\": 2",
        ] {
            assert!(
                artifacts.descriptor_layout_json.contains(needle),
                "descriptor layout 缺 scalar root constant layout 证据 {needle}: {}",
                artifacts.descriptor_layout_json
            );
        }
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(
            codes.is_empty(),
            "scalar root constants layout 不应发诊断,实得 {codes:?}"
        );
    }

    /// strict-only:scalar root constants 超 64 DWORD 仍 fail closed。
    //@ spec: RXS-0157
    #[test]
    fn kernel_with_too_many_scalar_root_constants_is_rx6007() {
        let mut src = String::from("kernel fn k(out: ViewMut<global, f32>");
        for i in 0..33 {
            let _ = write!(src, ", p{i}: usize");
        }
        src.push_str(") {}\n");
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        let artifacts = build_and_emit_dxil_artifacts(&cx, "k");
        assert!(
            artifacts.is_none(),
            "超 64 DWORD root constants 必须 fail closed"
        );
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(codes.contains(&6007), "应发 RX6007,实得 {codes:?}");
    }

    /// strict-only:未知 compute 形参类型仍拒绝,不 silent fallback。
    //@ spec: RXS-0157
    #[test]
    fn kernel_with_unknown_compute_param_is_rx6007() {
        let src = "struct Params { x: f32 }\nkernel fn k(p: Params) {}\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        let ir = build_and_emit_dxil(&cx, "k");
        assert!(ir.is_none(), "未知 compute 形参类型必须 strict-only 拒绝");
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(codes.contains(&6007), "应发 RX6007,实得 {codes:?}");
    }

    /// GRX-009 segment 3a:最小 no-else tail-if statement lowering → 真实 branch/label
    /// (`br i1` 分叉 + `if.then.0`/`if.end.0` deterministic label,store 落 then 块内)。
    //@ spec: RXS-0157
    #[test]
    fn tail_if_statement_lowers_to_branch_labels() {
        let src = "kernel fn k(t: ThreadCtx<1>, dst: ViewMut<global, f32>, len: usize) {\n    let gid = t.global_id();\n    if gid < len {\n        dst[0] = 1.0;\n    }\n}\n";
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        assert!(!diag.has_errors(), "tail-if kernel 应 0 前段诊断");
        let ir = build_and_emit_dxil(&cx, "k").expect("应产出 DXIL IR");
        let br_pos = ir.find("br i1 ").expect("应含条件分支 br i1");
        let then_pos = ir.find("if.then.0:").expect("应含 if.then.0 label");
        let store_pos = ir
            .find("@llvm.dx.resource.store.rawbuffer")
            .expect("then 块应含资源 store");
        let end_pos = ir.find("if.end.0:").expect("应含 if.end.0 label");
        assert!(
            br_pos < then_pos && then_pos < store_pos && store_pos < end_pos,
            "分支结构次序应为 br i1 < if.then.0 < store < if.end.0"
        );
        assert!(
            ir.contains("br label %if.end.0"),
            "then 块应以 br label %if.end.0 收束"
        );
    }

    // ───────────────── 任务4:stage 分发 + B 链 单测 ─────────────────

    use crate::hir::{DefId, PrimTy};
    use crate::mir::{
        BasicBlock, Const, IoDir, IoSigKind, Local, LocalIdx, MirIoType, Operand, Place, Rvalue,
        Statement, StatementKind, Terminator, TerminatorKind,
    };
    use crate::ty::Ty;

    fn dummy_span() -> Span {
        Span::new(SourceId(0), 0, 0, Edition::Rx0)
    }

    /// 便捷构造一个图形阶段 [`IoSigElem`]。
    fn io(name: &str, kind: IoSigKind, ty: MirIoType, dir: IoDir) -> IoSigElem {
        IoSigElem {
            field_name: name.to_owned(),
            kind,
            ty,
            dir,
        }
    }

    /// 最小图形阶段 vertex I/O:builtin position(out) + 一个 varying(out)+
    /// builtin vertex_index(in)。
    fn vertex_io() -> Vec<IoSigElem> {
        vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "vertex_index",
                IoSigKind::Builtin("vertex_index".to_owned()),
                MirIoType::Scalar(PrimTy::U32),
                IoDir::In,
            ),
        ]
    }

    /// 最小图形阶段 fragment I/O:varying(in)+ builtin frag_coord(in)+
    /// varying(out)。
    fn fragment_io() -> Vec<IoSigElem> {
        vec![
            io(
                "in_color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "out_color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
        ]
    }

    /// RXS-0172 provenance:varying 按方向各自从 0 递增 location,builtin 不占,空名跳过
    /// (与 `vertex_input_semantic_flags` / `emit_io_elem` 同源)。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_varying_provenance_matches_location_assignment() {
        let io_sig = vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
        ];
        // Out 方向:builtin position 不占 location,normal→0,uv→1。
        assert_eq!(
            varying_provenance(&io_sig, false),
            vec![(0, "normal".to_owned()), (1, "uv".to_owned())]
        );
        // In 方向:仅 builtin,无 varying → 空。
        assert!(varying_provenance(&io_sig, true).is_empty());
    }

    /// 模拟 spirv-cross 回译 HLSL(vertex 输出 struct 的 varying 退化为 TEXCOORD#)。
    fn vs_degraded_hlsl() -> &'static str {
        "struct SPIRV_Cross_Input\n\
         {\n\
         \x20   float3 in_var_POSITION : POSITION;\n\
         };\n\
         \n\
         struct SPIRV_Cross_Output\n\
         {\n\
         \x20   float3 out_var_NORMAL : TEXCOORD0;\n\
         \x20   float2 out_var_UV : TEXCOORD1;\n\
         \x20   float4 gl_Position : SV_Position;\n\
         };\n\
         \n\
         SPIRV_Cross_Output main(SPIRV_Cross_Input stage_input)\n\
         {\n\
         \x20   SPIRV_Cross_Output stage_output;\n\
         \x20   return stage_output;\n\
         }\n"
    }

    /// RXS-0172 L1/L3:vertex 输出 varying 退化名按 location provenance 复原为用户语义名,
    /// 只动 semantic token(类型/字段名/`;`/行数不变),SV_Position 与输入 struct 不动。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_output_varying_semantics_restored() {
        let io_sig = vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
        ];
        let src = vs_degraded_hlsl();
        let out = restore_varying_semantics(&io_sig, src);
        // 保名:输出 struct 的 TEXCOORD0/1 → normal/uv。
        assert!(out.contains("float3 out_var_NORMAL : normal;"), "{out}");
        assert!(out.contains("float2 out_var_UV : uv;"), "{out}");
        // SV_Position 不动;退化 TEXCOORD# 已消失(输出 struct)。
        assert!(out.contains("float4 gl_Position : SV_Position;"));
        assert!(!out.contains(": TEXCOORD"));
        // ABI 中立:类型/字段名保留;行数不变。
        assert!(out.contains("float3 out_var_NORMAL :"));
        assert!(out.contains("float2 out_var_UV :"));
        assert_eq!(src.lines().count(), out.lines().count());
    }

    /// RXS-0172 L4:fail-closed —— provenance 不覆盖的 location 退化名保留(经末端门拒)。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_unmapped_location_is_left_degraded_fail_closed() {
        // 仅给 location 0 的 provenance;location 1(uv)无对应 → 保留 TEXCOORD1。
        let io_sig = vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::Out,
            ),
        ];
        let out = restore_varying_semantics(&io_sig, vs_degraded_hlsl());
        assert!(out.contains("float3 out_var_NORMAL : normal;"), "{out}");
        // 无 provenance 的 TEXCOORD1 不被改写(fail-closed,留给 RX6011)。
        assert!(out.contains("float2 out_var_UV : TEXCOORD1;"), "{out}");
    }

    /// RXS-0172 L1:fragment 输入 varying 退化名按 location provenance 复原。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_fragment_input_varying_restored() {
        let src = "struct SPIRV_Cross_Input\n\
                   {\n\
                   \x20   float3 in_var_NORMAL : TEXCOORD0;\n\
                   \x20   float2 in_var_UV : TEXCOORD1;\n\
                   };\n\
                   \n\
                   float4 main(SPIRV_Cross_Input stage_input) : SV_Target\n\
                   {\n\
                   \x20   return 0.0.xxxx;\n\
                   }\n";
        let io_sig = vec![
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::In,
            ),
            io(
                "uv",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::In,
            ),
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
        ];
        let out = restore_varying_semantics(&io_sig, src);
        assert!(out.contains("float3 in_var_NORMAL : normal;"), "{out}");
        assert!(out.contains("float2 in_var_UV : uv;"), "{out}");
        assert!(!out.contains(": TEXCOORD"));
    }

    /// RXS-0172 L2:保名标准不放宽 —— 退化名经强制校验门 RX6011 拒,复原后等价名过。
    /// 用生产 `signature_gate::check`(不放宽)对模拟的 dxc 译后签名核验。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_gate_not_relaxed_degraded_rejected_restored_accepted() {
        use crate::toolchain::{DxilSignatures, SigElement};
        let intent = vec![
            io(
                "normal",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
        ];
        let mk = |name: &str, idx: u32| SigElement {
            name: name.to_owned(),
            index: idx,
            register: idx.to_string(),
            sysvalue: "NONE".to_owned(),
            used: true,
        };
        // 退化(TEXCOORD#):门拒(RX6011 SigMismatch)。
        let degraded = DxilSignatures {
            input: Vec::new(),
            output: vec![mk("TEXCOORD", 0), mk("TEXCOORD", 1)],
        };
        assert!(
            signature_gate::check(&degraded, &intent).is_err(),
            "退化 TEXCOORD# 必须被 RX6011 拒(门不放宽)"
        );
        // 复原(用户名):门过,无需放宽。
        let restored = DxilSignatures {
            input: Vec::new(),
            output: vec![mk("normal", 0), mk("uv", 0)],
        };
        assert!(
            signature_gate::check(&restored, &intent).is_ok(),
            "复原用户语义名后校验门应过(不放宽)"
        );
    }

    /// RXS-0172:entry I/O struct 识别(返回类型=输出 struct;形参类型=输入 struct)。
    //@ spec: RXS-0172
    #[test]
    fn rxs0172_entry_io_struct_identification() {
        let structs = collect_struct_names(vs_degraded_hlsl());
        let (in_s, out_s) = find_entry_io_structs(vs_degraded_hlsl(), &structs);
        assert_eq!(in_s.as_deref(), Some("SPIRV_Cross_Input"));
        assert_eq!(out_s.as_deref(), Some("SPIRV_Cross_Output"));
    }

    /// 构造一个最小平凡 [`Body`](空体 + 单 Return 块);`stage`/`io_sig` 由调用方设。
    fn make_body(stage: Option<ShaderStage>, io_sig: Vec<IoSigElem>) -> Body {
        let sp = dummy_span();
        Body {
            def: DefId(0),
            symbol: "main".to_owned(),
            color: FnColor::Kernel,
            generic_args: Vec::new(),
            locals: vec![Local {
                ty: Ty::unit(),
                name: None,
                span: sp,
                shared: false,
                array_len: None,
            }],
            arg_count: 0,
            blocks: vec![BasicBlock {
                stmts: Vec::new(),
                terminator: Terminator {
                    kind: TerminatorKind::Return,
                    span: sp,
                },
            }],
            span: sp,
            stage,
            io_sig,
            resources: Vec::new(),
        }
    }

    fn emitted_codes(diag: &DiagCtxt) -> Vec<u16> {
        diag.emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect()
    }

    /// 分发恒跑(工具无关):None→A 路;Vertex/Fragment→B 路;mesh/task/RT→stub。
    /// 阶段→着色器类型/路由分类(含 mesh/task/RT deferred→stub→RX6007)即 RXS-0158 主旨。
    //@ spec: RXS-0158, RXS-0161
    #[test]
    fn classify_stage_routes_by_category() {
        // None(host / compute via kernel)→ A 路。
        assert_eq!(classify_stage(None), StageRoute::PathA);
        // compute 着色阶段亦归 A(D-131 compute=A)。
        assert_eq!(
            classify_stage(Some(ShaderStage::Compute)),
            StageRoute::PathA
        );
        // 图形阶段 → B 路。
        assert_eq!(
            classify_stage(Some(ShaderStage::Vertex)),
            StageRoute::PathB(ShaderStage::Vertex)
        );
        assert_eq!(
            classify_stage(Some(ShaderStage::Fragment)),
            StageRoute::PathB(ShaderStage::Fragment)
        );
        // mesh/task/RT 等 → STUB(RD-012)。
        for s in [
            ShaderStage::Mesh,
            ShaderStage::Task,
            ShaderStage::RayGen,
            ShaderStage::ClosestHit,
            ShaderStage::AnyHit,
            ShaderStage::Miss,
        ] {
            assert_eq!(
                classify_stage(Some(s)),
                StageRoute::Stub(s),
                "{s:?} 应 stub"
            );
        }
    }

    /// 分发:compute/kernel body(stage None,空体)→ A 路,产 DirectX 三元组 IR,
    /// 不进 B 路(A 路用例不回归)。
    //@ spec: RXS-0161
    #[test]
    fn dispatch_compute_body_goes_path_a() {
        let diag = DiagCtxt::new();
        let body = make_body(None, Vec::new());
        match dispatch_and_emit(&diag, &body, "cs_noop") {
            DispatchOutcome::PathAIr(ir) => {
                assert!(
                    ir.contains("target triple = \"dxil-unknown-shadermodel6.0-compute\""),
                    "A 路应产 compute 三元组 IR"
                );
            }
            other => panic!("compute body 应走 A 路,实得 {other:?}"),
        }
        assert!(!diag.has_errors(), "空体 compute A 路应 0 诊断");
    }

    /// 分发:vertex/fragment body → B 路分支(非 A 路)。任务5 校验门接入后,带工具链
    /// 真跑时 trivial passthrough 被 DCE → 校验门如期拒绝 → `Diagnosed`(6xxx,经
    /// 既有 DXIL 诊断通道);工具链缺失 → `SkippedB`;均**不**得 `PathAIr`(零漂移)。
    /// 关键不变式:图形阶段恒走 B 路,绝不误入 A 路。
    //@ spec: RXS-0161
    #[test]
    fn dispatch_graphics_body_goes_path_b_not_a() {
        for (stage, io_sig) in [
            (ShaderStage::Vertex, vertex_io()),
            (ShaderStage::Fragment, fragment_io()),
        ] {
            let diag = DiagCtxt::new();
            let body = make_body(Some(stage), io_sig);
            match dispatch_and_emit(&diag, &body, "gfx") {
                // 校验门通过(签名保真)。
                DispatchOutcome::PathBSignatures { .. } => {}
                // 工具链不可用 → SKIP(非 6xxx,环境降级)。
                DispatchOutcome::SkippedB(_) => {}
                // 带工具链真跑:trivial passthrough DCE 消除声明输入 → 校验门
                // strict-only 如期拒绝 → 新 6xxx(RX6012 声明输入被消除 / RX6011 签名
                // 不一致;设计决策1 红例域,非误入 A 路,绝不复用 A 路 RX6007)。
                DispatchOutcome::Diagnosed => {
                    let codes = emitted_codes(&diag);
                    assert!(
                        codes.iter().any(|c| (6010..=6013).contains(c)),
                        "{stage:?} B 路校验门拒绝应经新 B 路 6xxx 码(RX6010~6013),实得 {codes:?}"
                    );
                    assert!(
                        !codes.contains(&6007),
                        "{stage:?} B 路失败绝不复用 A 路 RX6007(零漂移),实得 {codes:?}"
                    );
                }
                DispatchOutcome::PathAIr(_) => panic!("{stage:?} 误入 A 路"),
            }
        }
    }

    /// mesh/task/RT stub:发「暂不支持」6xxx 诊断、不产物(STUB(RD-012))。
    //@ spec: RXS-0161
    #[test]
    fn dispatch_mesh_task_rt_stub_diagnoses_no_artifact() {
        for s in [
            ShaderStage::Mesh,
            ShaderStage::Task,
            ShaderStage::RayGen,
            ShaderStage::ClosestHit,
            ShaderStage::AnyHit,
            ShaderStage::Miss,
        ] {
            let diag = DiagCtxt::new();
            let body = make_body(Some(s), Vec::new());
            let outcome = dispatch_and_emit(&diag, &body, "gfx");
            assert!(
                matches!(outcome, DispatchOutcome::Diagnosed),
                "{s:?} 应 stub 诊断不产物,实得 {outcome:?}"
            );
            assert!(
                emitted_codes(&diag).contains(&6007),
                "{s:?} stub 应发 6xxx(本任务用既有 RX6007 通道),实得 {:?}",
                emitted_codes(&diag)
            );
        }
    }

    /// strict-only:不可映射构造(f64 标量)→ emit_dxil_b 返回 [`DxilBError::Spirv`]
    /// (透传任务2 编码器),绝不静默成功。工具无关恒跑。
    //@ spec: RXS-0161
    #[test]
    fn emit_dxil_b_unmappable_is_error_not_silent() {
        let io = vec![io(
            "weird",
            IoSigKind::Varying,
            MirIoType::Scalar(PrimTy::F64),
            IoDir::Out,
        )];
        let r = emit_dxil_b(ShaderStage::Vertex, &io, &[]);
        assert!(
            matches!(r, Err(DxilBError::Spirv(DxilError::Unmappable { .. }))),
            "f64 应透传不可映射 6xxx,实得 {r:?}"
        );
    }

    /// 统一 artifacts 入口对非图形阶段保持与既有包装器一致的不可映射错误。
    #[test]
    fn compile_dxil_b_body_rejects_non_graphics_stage() {
        let body = make_body(Some(ShaderStage::Mesh), Vec::new());
        let r = compile_dxil_b_body(&body);
        assert!(
            matches!(r, Err(DxilBError::Spirv(DxilError::Unmappable { .. }))),
            "统一 artifacts 入口应拒绝非图形阶段,实得 {r:?}"
        );
    }

    /// strict-only:不可映射构造经分发 → 6xxx 诊断、不产物(走既有 RX6007 通道)。
    //@ spec: RXS-0161
    #[test]
    fn dispatch_unmappable_graphics_body_diagnoses() {
        let io = vec![io(
            "weird",
            IoSigKind::Varying,
            MirIoType::Scalar(PrimTy::F64),
            IoDir::Out,
        )];
        let diag = DiagCtxt::new();
        let body = make_body(Some(ShaderStage::Vertex), io);
        let outcome = dispatch_and_emit(&diag, &body, "gfx");
        assert!(
            matches!(outcome, DispatchOutcome::Diagnosed),
            "不可映射应诊断不产物,实得 {outcome:?}"
        );
        assert!(
            emitted_codes(&diag).contains(&6013),
            "应发 RX6013 不可映射构造"
        );
    }

    /// RXS-0171 strict-only:白名单外 body rvalue 经生产分发映射为 RX6013。
    //@ spec: RXS-0171
    #[test]
    fn dispatch_unsupported_body_rvalue_diagnoses_rx6013() {
        let io = vec![io(
            "out_luma",
            IoSigKind::Varying,
            MirIoType::Scalar(PrimTy::F32),
            IoDir::Out,
        )];
        let mut body = make_body(Some(ShaderStage::Fragment), io);
        let sp = dummy_span();
        body.locals = vec![Local {
            ty: Ty::Adt(DefId(9017), Vec::new()),
            name: None,
            span: sp,
            shared: false,
            array_len: None,
        }];
        body.blocks[0].stmts.push(Statement {
            kind: StatementKind::Assign(
                Place::local(LocalIdx(0)),
                Rvalue::UnaryOp(UnOp::Neg, Operand::Const(Const::Float(1.0, PrimTy::F32))),
            ),
            span: sp,
        });

        let diag = DiagCtxt::new();
        let outcome = dispatch_and_emit(&diag, &body, "gfx");
        assert!(
            matches!(outcome, DispatchOutcome::Diagnosed),
            "unsupported body rvalue 应诊断不产物,实得 {outcome:?}"
        );
        assert!(
            emitted_codes(&diag).contains(&6013),
            "unsupported body rvalue 应发 RX6013,实得 {:?}",
            emitted_codes(&diag)
        );
    }

    // 🔒 禁区说明(纹理访问语义 → 6xxx):IoSigElem/MirIoType 仅可表达已建模标量/
    // 向量,**结构上无法**表达资源句柄/描述符/采样器,故纹理访问语义(描述符编码/
    // 采样 opcode/缓存/LOD/导数/越界)在本层不可构造、不可达(任务2 即如此);该路径
    // 由后续绑定布局分片(G2.3,P-11)覆盖,本层保留 emit_dxil_b 的 DxilBError::Spirv
    // 透传接缝 + 模块顶注「需升档」标注。故本任务无纹理 6xxx 单测(输入不可达)。

    /// B 链端到端(带工具链 → 真跑直到 `signature_gate::check`;缺失 → SKIP 不 fail)。
    /// vertex + fragment 各一例。
    ///
    /// 任务5 接缝接入后的真实行为(设计决策1):任务2 最小子集 emit 的是 trivial
    /// passthrough `main`,**不读写 I/O**,dxc 会把未用的 builtin/varying DCE 消除
    /// (B 链 vertex 例实测得 `input:[]`)→ 校验门按 strict-only **如期拒绝**
    /// (`SigDroppedInput`:声明输入被消除)。这是 R2.4 预期红例域,**不是 bug**,
    /// 更**不**为让测试通过而旁路校验门(Property 5)。故接受的真跑结局为:
    /// - `Skipped`(工具链不可用)→ SKIP;
    /// - `Err(SigGate(SigDroppedInput))`(DCE 消除声明输入)→ 校验门如期红;
    /// - `Err(SigGate(SigMismatch))`(语义名/系统值未保真)→ 校验门如期红;
    /// - `Produced`(若译后签名恰好保真)→ 校验门绿。
    ///
    /// 编码器不可映射 / 工具转译失败仍判为测试失败(最小子集不应触发)。
    //@ spec: RXS-0162
    #[test]
    fn emit_dxil_b_end_to_end_or_skip() {
        for (tag, stage, io_sig) in [
            ("vertex", ShaderStage::Vertex, vertex_io()),
            ("fragment", ShaderStage::Fragment, fragment_io()),
        ] {
            match emit_dxil_b(stage, &io_sig, &[]) {
                Ok(DxilBOutcome::Produced { sigs, .. }) => {
                    // 校验门已强制通过:译后签名与意图签名保真。
                    eprintln!("[OK] {tag} B 链产签名且校验门通过: {sigs:?}");
                }
                Ok(DxilBOutcome::Skipped(why)) => {
                    eprintln!("[SKIP] {tag} B 链工具链不可用: {why}");
                }
                Err(DxilBError::SigGate(e)) => {
                    // strict-only 如期拒绝(trivial passthrough DCE 消除声明输入/
                    // 未保真),非 bug、非旁路。
                    eprintln!("[GATE-REJECT] {tag} 校验门如期拒绝 DCE/未保真产物: {e}");
                }
                Err(e) => panic!(
                    "[{tag}] B 链最小子集不应因编码器/工具失败而红(校验门拒绝走 SigGate): {e}"
                ),
            }
        }
    }

    /// **Property 5(校验门不旁路)**:校验门失败是 B 路 strict-only 失败的一种,经
    /// **唯一**出口 [`emit_b_error`] 落 6xxx 结构化诊断,**绝不**静默通过、绝不产物。
    /// 两类 [`SigGateError`] 分别落 `RX6011`(SigMismatch)/ `RX6012`(SigDroppedInput)。
    ///
    /// 代码层佐证(无需运行):`run_b_chain` 步骤8 以 `signature_gate::check(..)
    /// .map_err(DxilBError::SigGate)?` 在返回 [`DxilBOutcome::Produced`] **之前**以 `?`
    /// 终止——校验失败的入口不可能到达 `Produced` 分支;且 `check` 签名仅 `(actual,
    /// intent)`,无任何 skip / 开关 / env 参数(类型层即无旁路面)。
    //@ spec: RXS-0162
    #[test]
    fn property5_siggate_failure_routes_to_6xxx_never_silent() {
        use crate::dxil_sig_gate::signature_gate::SigGateError;
        let cases = [
            (
                DxilBError::SigGate(SigGateError::SigMismatch {
                    detail: "语义名未保真".to_owned(),
                }),
                6011u16,
            ),
            (
                DxilBError::SigGate(SigGateError::SigDroppedInput {
                    detail: "声明输入被消除".to_owned(),
                }),
                6012u16,
            ),
        ];
        for (err, expected) in cases {
            let diag = DiagCtxt::new();
            emit_b_error(&diag, dummy_span(), &err);
            assert!(diag.has_errors(), "校验门失败必落诊断(strict-only,不静默)");
            assert!(
                emitted_codes(&diag).contains(&expected),
                "校验门失败必经新 6xxx 码 RX{expected}(不旁路、不复用 RX6007),实得 {:?}",
                emitted_codes(&diag)
            );
            assert!(
                !emitted_codes(&diag).contains(&6007),
                "校验门失败绝不再落 A 路 RX6007(零漂移),实得 {:?}",
                emitted_codes(&diag)
            );
        }
    }

    /// B 链转译失败(`DxilBError::Toolchain`,spirv-cross/dxc/dumpbin exit≠0)经
    /// [`emit_b_error`] 落 `RX6010` `codegen.dxil_b_transpile_failed`,strict-only 不静默。
    /// (SKIP——工具缺失/spawn 失败——在 `classify_tool_failure` 即转 `Skipped`,不到此。)
    //@ spec: RXS-0157
    #[test]
    fn emit_b_error_toolchain_routes_to_rx6010() {
        let err = DxilBError::Toolchain {
            step: "dxc".to_owned(),
            reason: "exit 1: validation error".to_owned(),
        };
        let diag = DiagCtxt::new();
        emit_b_error(&diag, dummy_span(), &err);
        assert!(
            emitted_codes(&diag).contains(&6010),
            "B 链转译失败应发 RX6010,实得 {:?}",
            emitted_codes(&diag)
        );
        assert!(
            !emitted_codes(&diag).contains(&6007),
            "B 链转译失败绝不复用 A 路 RX6007(零漂移),实得 {:?}",
            emitted_codes(&diag)
        );
    }

    /// 不可映射构造(`DxilBError::Spirv(Unmappable)`)经 [`emit_b_error`] 落 `RX6013`
    /// `codegen.dxil_unmappable`,strict-only 不静默。
    //@ spec: RXS-0157
    #[test]
    fn emit_b_error_unmappable_routes_to_rx6013() {
        let err = DxilBError::Spirv(DxilError::Unmappable {
            what: "scalar-type".to_owned(),
            detail: "f64 不在已建模标量子集".to_owned(),
        });
        let diag = DiagCtxt::new();
        emit_b_error(&diag, dummy_span(), &err);
        assert!(
            emitted_codes(&diag).contains(&6013),
            "不可映射构造应发 RX6013,实得 {:?}",
            emitted_codes(&diag)
        );
    }

    /// 绑定布局推导失败经 [`emit_b_error`] 按变体分派专属码(RXS-0163~0166;
    /// G2.3 PR-E2b-2,owner 已裁:Unmappable 复用 RX6013,其余新开 RX6015/6016/6017)。
    //@ spec: RXS-0163, RXS-0164, RXS-0165, RXS-0166
    #[test]
    fn emit_b_error_binding_routes_to_dedicated_codes() {
        use crate::binding_layout::BindingInferError;
        let cases: &[(DxilBError, u16)] = &[
            (
                DxilBError::Binding(BindingInferError::Unmappable {
                    detail: "bindless 资源(RD-018 defer)".to_owned(),
                }),
                6013,
            ),
            (
                DxilBError::Binding(BindingInferError::RegisterConflict {
                    detail: "t0 区间重叠".to_owned(),
                }),
                6015,
            ),
            (
                DxilBError::Binding(BindingInferError::RootSignatureTooLarge {
                    dwords: 66,
                    limit: 64,
                }),
                6016,
            ),
            (
                DxilBError::Binding(BindingInferError::Psv0Mismatch {
                    detail: "反射资源数与意图不一致".to_owned(),
                }),
                6017,
            ),
        ];
        for (err, expected) in cases {
            let diag = DiagCtxt::new();
            emit_b_error(&diag, dummy_span(), err);
            let codes = emitted_codes(&diag);
            assert!(
                codes.contains(expected),
                "{err:?} 应发 RX{expected},实得 {codes:?}"
            );
            // 零漂移:绑定布局失败绝不复用 RX6007(A 路)/ RX6011~6012(签名校验门)。
            assert!(
                !codes.contains(&6007),
                "绑定布局失败绝不复用 A 路 RX6007,实得 {codes:?}"
            );
        }
    }

    /// 阶段间接口错链经 [`emit_stage_link_error`] 落 `RX6014`
    /// `codegen.dxil_stage_link_mismatch`(RXS-0160;agent 裁定方案 B 新开码,
    /// 不复用 RX6011),两类错链(`Unlinked` / `LinkMismatch`)同落 RX6014。
    //@ spec: RXS-0160
    #[test]
    fn emit_stage_link_error_routes_to_rx6014() {
        let errs = [
            signature_gate::StageLinkError::Unlinked {
                detail: "fragment 输入 `extra` 无上游链接键".to_owned(),
            },
            signature_gate::StageLinkError::LinkMismatch {
                detail: "链接键 `color` 两端类型失配".to_owned(),
            },
        ];
        for err in &errs {
            let diag = DiagCtxt::new();
            emit_stage_link_error(&diag, dummy_span(), err);
            let codes = emitted_codes(&diag);
            assert!(
                codes.contains(&6014),
                "阶段间接口错链应发 RX6014,实得 {codes:?}"
            );
            // 零漂移:错链新开码 RX6014,绝不复用单阶段签名不一致 RX6011。
            assert!(
                !codes.contains(&6011),
                "错链 owner 裁定新开 RX6014,绝不复用 RX6011,实得 {codes:?}"
            );
        }
    }

    /// 顶点输入语义保名旗标导出(工具无关,恒跑):[`vertex_input_semantic_flags`] 按
    /// io_sig 顺序复算 location → field_name(与 emit_spirv 的 next_in_location 对齐),
    /// 经 io_sig 导出、**非硬编码**(RFC-0004 §4.4 机制①,实测顶点输入名存活)。
    //@ spec: RXS-0159
    #[test]
    fn vertex_input_semantic_flags_derive_from_io_sig() {
        // vertex:命名输入 POSITION(loc0)/ NORMAL(loc1)+ builtin vertex_index(不占
        // location)+ 命名输出(不取输入旗标)。
        let io_sig = vec![
            io(
                "POSITION",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::In,
            ),
            io(
                "vertex_index",
                IoSigKind::Builtin("vertex_index".to_owned()),
                MirIoType::Scalar(PrimTy::U32),
                IoDir::In,
            ),
            io(
                "NORMAL",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::In,
            ),
            io(
                "color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
        ];
        let flags = vertex_input_semantic_flags(ShaderStage::Vertex, &io_sig);
        // POSITION→loc0(builtin 不占 location)、NORMAL→loc1;输出 color 不取旗标。
        assert_eq!(
            flags,
            vec![
                "--set-hlsl-vertex-input-semantic".to_owned(),
                "0".to_owned(),
                "POSITION".to_owned(),
                "--set-hlsl-vertex-input-semantic".to_owned(),
                "1".to_owned(),
                "NORMAL".to_owned(),
            ],
            "顶点输入保名旗标应按 io_sig 顺序复算 location(builtin 不占位),非硬编码"
        );

        // fragment:顶点输入保名旗标机制不适用(无顶点输入语义旗标)→ 空(边界;fragment
        // 输入 varying 保名经 RXS-0172 `restore_varying_semantics`,见该函数单测)。
        assert!(
            vertex_input_semantic_flags(ShaderStage::Fragment, &fragment_io()).is_empty(),
            "fragment 阶段不导出顶点输入保名旗标(spirv-cross 无片元输入语义旗标;保名走 RXS-0172)"
        );

        // vertex 无命名输入(仅 builtin 输入 / 命名输出)→ 空(行为不变)。
        assert!(
            vertex_input_semantic_flags(ShaderStage::Vertex, &vertex_io()).is_empty(),
            "无命名顶点输入 → 无保名旗标(行为不变)"
        );
    }

    // ───────────────── RXS-0160:vertex+fragment 多阶段联编点接缝 ─────────────────

    /// 链接一致的 vertex 输出(position builtin out + uv interpolate out)。
    fn vs_link_io() -> Vec<IoSigElem> {
        vec![
            io(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
        ]
    }

    /// 与 [`vs_link_io`] 链接一致的 fragment 输入(frag_coord builtin in + uv
    /// interpolate in + out_color varying out)。
    fn fs_link_io() -> Vec<IoSigElem> {
        vec![
            io(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            io(
                "uv",
                IoSigKind::Interpolate("perspective".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::In,
            ),
            io(
                "out_color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
        ]
    }

    /// accept:vertex+fragment 配对 + 链接一致 → `Linked`(多阶段联编点核对通过)。
    //@ spec: RXS-0160
    #[test]
    fn link_graphics_stages_consistent_pair_is_linked() {
        let bodies = vec![
            make_body(Some(ShaderStage::Vertex), vs_link_io()),
            make_body(Some(ShaderStage::Fragment), fs_link_io()),
        ];
        assert_eq!(
            link_graphics_stages(&bodies),
            StageLinkOutcome::Linked,
            "vertex+fragment 链接一致应 Linked"
        );
    }

    /// reject:fragment 输入 varying(`extra`)在 vertex 输出无链接键 → `LinkError`
    /// (错链;strict-only)→ 经 [`emit_stage_link_error`] 落 `RX6014`(agent 裁定方案 B
    /// 新开码,G2.3 PR-E2b-2)。
    //@ spec: RXS-0160
    #[test]
    fn link_graphics_stages_mismatched_pair_is_link_error() {
        let fs = vec![io(
            "extra",
            IoSigKind::Varying,
            MirIoType::Vector(PrimTy::F32, 4),
            IoDir::In,
        )];
        let bodies = vec![
            make_body(Some(ShaderStage::Vertex), vs_link_io()),
            make_body(Some(ShaderStage::Fragment), fs),
        ];
        let outcome = link_graphics_stages(&bodies);
        let StageLinkOutcome::LinkError(err) = outcome else {
            panic!("错链应 LinkError,实得 {outcome:?}");
        };
        // 错链经生产 emit 接缝落真实码 RX6014(替换 agent 裁码前的占位「6xxx」)。
        let diag = DiagCtxt::new();
        emit_stage_link_error(&diag, dummy_span(), &err);
        assert!(
            emitted_codes(&diag).contains(&6014),
            "错链应发 RX6014,实得 {:?}",
            emitted_codes(&diag)
        );
    }

    /// 单阶段编译(仅 vertex,缺 fragment)→ `NoPair`(无链接核对,零漂移)。
    //@ spec: RXS-0160
    #[test]
    fn link_graphics_stages_single_stage_is_no_pair() {
        let bodies = vec![make_body(Some(ShaderStage::Vertex), vs_link_io())];
        assert_eq!(
            link_graphics_stages(&bodies),
            StageLinkOutcome::NoPair,
            "缺 fragment 阶段应 NoPair(单阶段编译零漂移)"
        );
    }

    /// 无图形阶段(compute/kernel,stage None)→ `NoPair`(A 路 / 单阶段零漂移)。
    //@ spec: RXS-0160
    #[test]
    fn link_graphics_stages_no_graphics_is_no_pair() {
        let bodies = vec![make_body(None, Vec::new())];
        assert_eq!(
            link_graphics_stages(&bodies),
            StageLinkOutcome::NoPair,
            "无图形阶段(compute)应 NoPair(零漂移)"
        );
    }

    /// `json_escape` 边界与等价性:空串、纯 ASCII、5 个转义字符、UTF-8 多字节、
    /// 头尾转义、连续转义、超长输入。回归边界确保字节级批量复制不损坏 UTF-8。
    #[test]
    fn json_escape_empty_yields_empty() {
        assert_eq!(json_escape(""), "");
    }

    #[test]
    fn json_escape_plain_ascii_unchanged() {
        assert_eq!(json_escape("hello world 123"), "hello world 123");
    }

    #[test]
    fn json_escape_escapes_all_five_special_chars() {
        assert_eq!(json_escape("\""), "\\\"");
        assert_eq!(json_escape("\\"), "\\\\");
        assert_eq!(json_escape("\n"), "\\n");
        assert_eq!(json_escape("\r"), "\\r");
        assert_eq!(json_escape("\t"), "\\t");
    }

    #[test]
    fn json_escape_preserves_non_escape_control_bytes() {
        // 0x01..0x08、0x0b、0x0c、0x0e..0x1f 不在 JSON 必需转义集合内,
        // 当前实现原样输出(与原 chars() 版本行为一致)。
        let s = "\u{0001}\u{0008}\u{000b}\u{000c}\u{001f}";
        assert_eq!(json_escape(s), s);
    }

    #[test]
    fn json_escape_mixed_special_and_plain() {
        let input = "a\"b\\c\nd\re\tf";
        let expected = "a\\\"b\\\\c\\nd\\re\\tf";
        assert_eq!(json_escape(input), expected);
    }

    #[test]
    fn json_escape_escape_at_head_and_tail() {
        assert_eq!(json_escape("\"abc"), "\\\"abc");
        assert_eq!(json_escape("abc\""), "abc\\\"");
        assert_eq!(json_escape("\""), "\\\"");
    }

    #[test]
    fn json_escape_consecutive_escapes() {
        assert_eq!(json_escape("\"\"\""), "\\\"\\\"\\\"");
        assert_eq!(json_escape("\n\n"), "\\n\\n");
    }

    #[test]
    fn json_escape_preserves_utf8_multibyte() {
        // 中文 + emoji + 带转义字符混合:验证字节级批量复制不切断多字节序列。
        let input = "你好\"world\\🎉\n";
        let expected = "你好\\\"world\\\\🎉\\n";
        assert_eq!(json_escape(input), expected);
    }

    #[test]
    fn json_escape_long_string_linear() {
        // 1 KiB 普通字符 + 间隔转义:验证批量复制路径与上界容量分配。
        let mut input = String::with_capacity(1024);
        for i in 0..64 {
            input.push_str(&format!("seg{:03}_", i));
            if i % 8 == 0 {
                input.push('"');
            }
        }
        let out = json_escape(&input);
        // 每个 `"` → `\"`,数量等于 i%8==0 的 i(0,8,...,56)→ 8 个。
        let quote_count = (0..64).filter(|i| i % 8 == 0).count();
        assert_eq!(
            out.matches("\\\"").count(),
            quote_count,
            "转义引号计数应匹配"
        );
        // 未转义部分应能复原(去掉 `\"` 中的反斜杠)。
        assert_eq!(out.replace("\\\"", "\""), input);
    }

    #[test]
    fn json_escape_only_escape_chars() {
        let s = "\"\\\n\r\t\"\\\n\r\t";
        let expected = "\\\"\\\\\\n\\r\\t\\\"\\\\\\n\\r\\t";
        assert_eq!(json_escape(s), expected);
    }
}
