//! device MIR → DXIL DirectX 三元组 LLVM IR 文本(G2.2 PR-C2 分片1,RXS-0157;
//! RFC-0003 §4.1/§4.2,D-131=A)。
//!
//! 本模块 gate 于 cargo feature `dxil-backend`(RFC-0003 §9 Q-Gate);未启用时整模块
//! 不编入 rurixc,PTX 路径(D-207)不受影响。target 分发在 MIR 之后分叉:DXIL 后端与
//! NVPTX 后端(`device_codegen`)并列、各自从 MIR 独立降级,不共享后端 lowering
//! (RFC-0003 §4.5)。
//!
//! **最小 compute 子集(分片1)**:仅支持 compute 着色入口(`kernel fn`,RXS-0153
//! compute-via-kernel 着色)的最小子集——无 ABI 形参、平凡(空)体 → DXIL `void` 入口
//! (`dxil-unknown-shadermodel6.0-compute` 三元组 + `hlsl.shader`="compute" /
//! `hlsl.numthreads` 入口属性,对齐 LLVM DirectX 后端 emit 形态)。子集外构造
//! (View/资源句柄形参、非平凡体——需绑定布局推导 G2.3 / FFI ABI 禁区)→ `RX6007`。
//!
//! 下游(IR → patched llc -filetype=obj → DXIL 容器 → dxc validator)见
//! [`crate::toolchain::ir_to_dxil`];golden 取文本反汇编经 validator 验证(RFC-0003
//! §9 Q-Golden)。**本片不碰** 🔒 纹理内存模型映射(06 §4.2)/ FFI ABI 二进制布局
//! (RFC-0003 §4.6)/ 绑定布局推导(G2.3,P-11)。

use std::fmt::Write as _;

use crate::ast::FnColor;
use crate::diag::ErrorCode;
use crate::mir::{Body, Const, Operand, Rvalue, StatementKind, TerminatorKind};
use crate::query::QueryCtx;
use crate::span::Span;

/// DXIL codegen 失败(RX6007;目标不可用 / 子集外构造 / 降级失败,RXS-0157 L1~L3)。
#[derive(Debug, Clone)]
pub struct DxilCodegenError {
    pub span: Span,
    pub detail: String,
}

impl DxilCodegenError {
    fn unsupported(span: Span, detail: impl Into<String>) -> Self {
        DxilCodegenError {
            span,
            detail: detail.into(),
        }
    }
}

/// 驱动 / 测试入口:构建 device MIR(`kernel fn` 为根)+ DXIL 最小 compute codegen。
/// 无 kernel → `None`(无 device 产物);子集外 / 降级失败 → 经 `cx.diag()` 落
/// `RX6007` 结构化诊断并返回 `None`;成功 → `Some(DirectX 三元组 LLVM IR 文本)`。
/// patched llc → DXIL 容器 + dxc validator 由驱动在产 IR 后另行实施(RXS-0157 IR2)。
pub fn build_and_emit_dxil(cx: &QueryCtx<'_>, module_name: &str) -> Option<String> {
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
    match emit_dxil_ir(entry, module_name) {
        Ok(ir) => Some(ir),
        Err(e) => {
            cx.diag()
                .struct_error(ErrorCode(6007), "codegen.dxil_unsupported")
                .arg("detail", e.detail.clone())
                .span_label(e.span, "in DXIL compute entry")
                .emit();
            None
        }
    }
}

/// 单个 compute kernel body → DXIL DirectX 三元组 LLVM IR 文本(最小子集)。
/// 子集校验(RXS-0157 L2):无 ABI 形参 + 平凡体(块内零语句,终结子仅 Goto/Return/
/// Unreachable);违例 → `DxilCodegenError`(上层映射 RX6007)。
pub fn emit_dxil_ir(body: &Body, module_name: &str) -> Result<String, DxilCodegenError> {
    if body.arg_count != 0 {
        return Err(DxilCodegenError::unsupported(
            body.span,
            "DXIL 最小 compute 子集暂不支持带形参的 compute 入口(View/资源句柄绑定布局推导属 G2.3,FFI ABI 属禁区)",
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
                    "DXIL 最小 compute 子集暂不支持非平凡 compute 体(分片1 仅空体入口,语句降级随后续分片)",
                ));
            };
        }
        match bb.terminator.kind {
            TerminatorKind::Goto(_) | TerminatorKind::Return | TerminatorKind::Unreachable => {}
            _ => {
                return Err(DxilCodegenError::unsupported(
                    bb.terminator.span,
                    "DXIL 最小 compute 子集暂不支持该控制流终结子(分片1 仅空体入口)",
                ));
            }
        }
    }
    Ok(render_dxil_module(&body.symbol, module_name))
}

/// DirectX 三元组 LLVM IR 文本(最小空体 compute 入口)。形态对齐 LLVM DirectX 后端
/// emit 期望(shadermodel6.0-compute 三元组 + DXIL 数据布局 + `hlsl.shader`/
/// `hlsl.numthreads` 入口属性);经 patched llc -filetype=obj 产 DXIL 容器、dxc
/// validator 接受(round-8 recipe 验证)。numthreads 取最小 `1,1,1`(分片1 无 launch
/// bounds 降级)。确定性:给定符号名输出字节确定。
fn render_dxil_module(entry_symbol: &str, module_name: &str) -> String {
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
    let _ = writeln!(out, "define void @{entry_symbol}() #0 {{");
    out.push_str("entry:\n");
    out.push_str("  ret void\n");
    out.push_str("}\n");
    out.push('\n');
    out.push_str(
        "attributes #0 = { noinline nounwind \"hlsl.numthreads\"=\"1,1,1\" \"hlsl.shader\"=\"compute\" }\n",
    );
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
}
