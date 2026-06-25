//! device MIR → DXIL DirectX 三元组 LLVM IR 文本(G2.2 PR-C2;RXS-0157 分片1 +
//! RXS-0158 分片2,RFC-0003 §4.1/§4.2,D-131=A)。
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
//! §9 Q-Golden)。**本片不碰** 🔒 纹理内存模型映射(06 §4.2)/ 阶段 I/O 签名 SV_*
//! (RXS-0159)/ FFI ABI 二进制布局(RFC-0003 §4.6)/ 绑定布局推导(G2.3,P-11)。

use std::fmt::Write as _;

use crate::ast::{FnColor, ShaderStage};
use crate::diag::ErrorCode;
use crate::hir::{self, DefId};
use crate::mir::{Body, Const, Operand, Rvalue, StatementKind, TerminatorKind};
use crate::query::QueryCtx;
use crate::span::Span;

/// DXIL codegen 失败(RXS-0157/0158)。`code` 区分诊断类别:
/// - `6007`(`codegen.dxil_unsupported`):目标不可用 / 子集外构造 / 降级失败
///   (RXS-0157 L1~L3);
/// - `6008`(`codegen.dxil_stage_unsupported`):着色阶段降级暂未支持(RXS-0158 L2,
///   mesh/task/RT 阶段 deferred RD-012)。
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
    match emit_dxil_ir(entry, stage, module_name) {
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

/// 单个着色阶段 body → DXIL DirectX 三元组 LLVM IR 文本(RXS-0158 阶段→着色器类型)。
/// 先裁阶段降级目标(deferred 阶段 → RX6008,RXS-0158 L2);再校验最小子集(RXS-0157
/// L1:无 ABI 形参 + 平凡体);违例 → `DxilCodegenError`(上层映射 RX6007/RX6008)。
pub fn emit_dxil_ir(
    body: &Body,
    stage: Option<ShaderStage>,
    module_name: &str,
) -> Result<String, DxilCodegenError> {
    // RXS-0158 L2:deferred 阶段(mesh/task/RT)先行裁定 → RX6008(优先于子集校验,
    // 阶段不可降级是首要可行动诊断)。
    let target = stage_target(stage, body.span)?;
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
    Ok(render_dxil_module(&body.symbol, module_name, &target))
}

/// DirectX 三元组 LLVM IR 文本(最小空体着色阶段入口,按阶段产对应 shader 类型)。
/// 形态对齐 LLVM DirectX 后端 emit 期望(`shadermodel<sm>-<env>` 三元组 + DXIL 数据
/// 布局 + `hlsl.shader`〔+ compute/mesh/task 的 `hlsl.numthreads`〕入口属性);经
/// patched llc -filetype=obj 产 DXIL 容器、dxc validator 接受(RXS-0158 IR1/IR3)。
/// numthreads 取最小 `1,1,1`。确定性:给定符号名 + 阶段目标输出字节确定。
fn render_dxil_module(entry_symbol: &str, module_name: &str, target: &DxilStageTarget) -> String {
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
    out
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
}
