//! device MIR → SPIR-V Vulkan 跨端后端 codegen(mb1,RXS-0200/0201;RFC-0011）。
//!
//! 本模块 gate 于 cargo feature `vulkan-backend`(RFC-0011 §6;未启用时整模块不编入
//! rurixc,PTX/DXIL 路径不受影响)。target 分发在 MIR 之后分叉:Vulkan 后端与 NVPTX
//! (`device_codegen`)/ DXIL(`dxil_codegen`)后端**并列**、各自从 MIR 独立降级,不共享
//! 后端 lowering(RFC-0003 §4.5 口径)。SPIR-V 是唯一中间产物:AMD 桌面驱动与 Android
//! `libvulkan.so` 都消费同一份 `.spv`(RFC-0011 §1)。
//!
//! **最小 compute 子集(MB1.1 walking skeleton,RXS-0201)**:仅支持 compute 着色入口
//! (`kernel fn`,RXS-0153 compute-via-kernel 着色)的最小子集——无 ABI 形参、平凡(空)
//! 体 → SPIR-V `GLCompute` 入口(`OpEntryPoint GLCompute` + `OpExecutionMode LocalSize`,
//! void `main`)。子集外构造(View/资源句柄形参、非平凡体——需存储缓冲降级 / 绑定布局
//! 推导 / 控制流,属后续分片 RXS-0202/0203)→ `RX6026`。
//!
//! 下游(`.spv` → `spirv-val` clean)见 [`crate::toolchain`];真实红绿:篡改 `.spv` 字节
//! → spirv-val 拒(红),复原绿(RFC-0011 §6)。**本片不碰** 🔒 launch marshalling FFI
//! ABI(RFC-0011 §4.7)/ Backend trait(§4.5)/ 纹理内存模型映射(06 §4.2);compute
//! builtins / 存储缓冲 / 控制流 / 数学 intrinsic→GLSL.std.450 属后续分片。

use crate::ast::FnColor;
use crate::diag::ErrorCode;
use crate::mir::{Body, Const, Operand, Rvalue, StatementKind, TerminatorKind};
use crate::query::QueryCtx;
use crate::span::Span;

// ───────────────────────── SPIR-V 常量(核心规范取值) ─────────────────────────
// 与 `dxil_spirv` 的图形编码器共享同一套 SPIR-V core 常量(取值同源于 SPIR-V 规范);
// compute 面新增 GLCompute 执行模型 + LocalSize 执行模式。抽取为共享 `spirv` 模块的
// 泛化随 compute+graphics 面成熟(RFC-0011 §4.2)——首片自包含以最小化耦合。

/// SPIR-V magic number(字流首字)。
const SPIRV_MAGIC: u32 = 0x0723_0203;
/// SPIR-V 版本字(1.0 = `0x0001_0000`;与广泛 spirv-val target-env / Vulkan 1.0+ 兼容)。
const SPIRV_VERSION_1_0: u32 = 0x0001_0000;
/// generator magic(未注册工具用 0;spirv-val 忽略)。
const SPIRV_GENERATOR: u32 = 0;
/// header schema 字(保留,恒 0)。
const SPIRV_SCHEMA: u32 = 0;

// opcodes(SPIR-V core 规范)。
const OP_MEMORY_MODEL: u16 = 14;
const OP_ENTRY_POINT: u16 = 15;
const OP_EXECUTION_MODE: u16 = 16;
const OP_CAPABILITY: u16 = 17;
const OP_TYPE_VOID: u16 = 19;
const OP_TYPE_FUNCTION: u16 = 33;
const OP_FUNCTION: u16 = 54;
const OP_FUNCTION_END: u16 = 56;
const OP_LABEL: u16 = 248;
const OP_RETURN: u16 = 253;

// 枚举取值。
const CAP_SHADER: u32 = 1;
const ADDR_MODEL_LOGICAL: u32 = 0;
const MEM_MODEL_GLSL450: u32 = 1;
/// `GLCompute` 执行模型(compute 着色入口;`dxil_spirv` 仅有 Vertex=0/Fragment=4)。
const EXEC_MODEL_GLCOMPUTE: u32 = 5;
/// `LocalSize` 执行模式(workgroup 维度;首片最小 `1,1,1`,launch bounds 降级属后续)。
const EXEC_MODE_LOCAL_SIZE: u32 = 17;
const FUNCTION_CONTROL_NONE: u32 = 0;

/// mb1 Vulkan codegen 目标不可用 / 暂不支持的构造 / 降级失败错误码(6xxx codegen 段;
/// 跳 RX6024/RX6025 = MS1.2b 在途占用避撞,RFC-0011 §5)。
const E_VULKAN_UNSUPPORTED: ErrorCode = ErrorCode(6026);

// ───────────────────────── 编码器本体 ─────────────────────────

/// Vulkan/SPIR-V codegen 错误(上层映射 `RX6026`)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VulkanCodegenError {
    pub span: Span,
    pub detail: String,
}

impl VulkanCodegenError {
    fn unsupported(span: Span, detail: impl Into<String>) -> Self {
        VulkanCodegenError {
            span,
            detail: detail.into(),
        }
    }
}

/// 一条 SPIR-V 指令写入字流:首字 = `(word_count << 16) | opcode`,`word_count` 含首字。
fn emit(stream: &mut Vec<u32>, opcode: u16, operands: &[u32]) {
    let word_count = (operands.len() + 1) as u32;
    stream.push((word_count << 16) | u32::from(opcode));
    stream.extend_from_slice(operands);
}

/// SPIR-V 字面串:UTF-8 + NUL 终止 + 4 字节对齐(little-endian 打包)。
fn push_string(operands: &mut Vec<u32>, s: &str) {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0); // NUL 终止
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
    for chunk in bytes.chunks(4) {
        let mut w = 0u32;
        for (i, &b) in chunk.iter().enumerate() {
            w |= u32::from(b) << (8 * i);
        }
        operands.push(w);
    }
}

/// 驱动 / 测试入口:构建 device MIR(`kernel fn` 为根)+ SPIR-V 最小 compute codegen。
/// 无 kernel → `None`(无 device 产物);子集外 / 降级失败 → 经 `cx.diag()` 落 `RX6026`
/// 结构化诊断并返回 `None`;成功 → `Some(SPIR-V 字流)`。`.spv` 落盘 + `spirv-val` gate 由
/// 驱动在产字流后另行实施(RFC-0011 §4.2)。
pub fn build_and_emit_vulkan(cx: &QueryCtx<'_>, module_name: &str) -> Option<Vec<u32>> {
    let bodies = cx.device_mir_crate();
    if bodies.is_empty() {
        return None;
    }
    // device MIR 构建已报错 → 不级联 codegen(防一错多报,对齐 device_codegen / dxil_codegen)。
    if cx.diag().has_errors() {
        return None;
    }
    // compute 入口 = kernel 着色 body(RXS-0153 compute-via-kernel);取首个为最小入口。
    let entry = bodies.iter().find(|b| b.color == FnColor::Kernel)?;
    match emit_spirv_compute(entry, module_name) {
        Ok(words) => Some(words),
        Err(e) => {
            cx.diag()
                .struct_error(E_VULKAN_UNSUPPORTED, "codegen.vulkan_unsupported")
                .arg("detail", e.detail.clone())
                .span_label(e.span, "in Vulkan compute entry")
                .emit();
            None
        }
    }
}

/// 单个 compute kernel body → SPIR-V 字流(最小子集,RXS-0201）。
/// 子集校验(walking skeleton):无 ABI 形参 + 平凡体(块内仅隐式 unit 返回赋值,终结子
/// 仅 Goto/Return/Unreachable);违例 → `VulkanCodegenError`(上层映射 `RX6026`)。
pub fn emit_spirv_compute(body: &Body, module_name: &str) -> Result<Vec<u32>, VulkanCodegenError> {
    if body.arg_count != 0 {
        return Err(VulkanCodegenError::unsupported(
            body.span,
            "Vulkan 最小 compute 子集暂不支持带形参的 compute 入口(存储缓冲绑定 / 描述符布局属 RXS-0203,后续分片)",
        ));
    }
    for bb in &body.blocks {
        for st in &bb.stmts {
            // 最小子集仅容忍隐式 unit 返回赋值(`_0 = ()`,空体语义);其余语句
            // (真实计算 / 内存写 / 调用)需 body lowering + 可能存储缓冲,属后续分片。
            let StatementKind::Assign(_, Rvalue::Use(Operand::Const(Const::Unit))) = &st.kind
            else {
                return Err(VulkanCodegenError::unsupported(
                    st.span,
                    "Vulkan 最小 compute 子集暂不支持非平凡 compute 体(walking skeleton 仅空体入口,body lowering 随 RXS-0202/0203)",
                ));
            };
        }
        match bb.terminator.kind {
            TerminatorKind::Goto(_) | TerminatorKind::Return | TerminatorKind::Unreachable => {}
            _ => {
                return Err(VulkanCodegenError::unsupported(
                    bb.terminator.span,
                    "Vulkan 最小 compute 子集暂不支持该控制流终结子(walking skeleton 仅空体入口,结构化控制流随 RXS-0203)",
                ));
            }
        }
    }
    Ok(render_compute_module(&body.symbol, module_name))
}

/// SPIR-V 字流(最小空体 GLCompute 入口)。形态对齐 Vulkan `vkCreateShaderModule` 消费
/// 期望(`GLCompute` 执行模型 + `LocalSize 1,1,1` + void `main`);经 `spirv-val` 接受。
/// LocalSize 取最小 `1,1,1`(walking skeleton 无 launch bounds 降级)。确定性:给定符号名
/// 输出字节确定(两次产出逐字节一致)。
fn render_compute_module(entry_symbol: &str, _module_name: &str) -> Vec<u32> {
    // 结果 id 静态分配(单入口空体:main / void / fn_type / label)。
    let main_id: u32 = 1;
    let void_id: u32 = 2;
    let fn_type_id: u32 = 3;
    let label_id: u32 = 4;
    let bound: u32 = 5; // = 最大 id + 1

    // header(5 字):magic / version / generator / bound / schema。
    let mut m: Vec<u32> = vec![
        SPIRV_MAGIC,
        SPIRV_VERSION_1_0,
        SPIRV_GENERATOR,
        bound,
        SPIRV_SCHEMA,
    ];

    // OpCapability Shader。
    emit(&mut m, OP_CAPABILITY, &[CAP_SHADER]);
    // OpMemoryModel Logical GLSL450。
    emit(
        &mut m,
        OP_MEMORY_MODEL,
        &[ADDR_MODEL_LOGICAL, MEM_MODEL_GLSL450],
    );
    // OpEntryPoint GLCompute %main "<entry>"(空 interface:无 I/O 变量)。
    let mut ep = vec![EXEC_MODEL_GLCOMPUTE, main_id];
    push_string(&mut ep, entry_symbol);
    emit(&mut m, OP_ENTRY_POINT, &ep);
    // OpExecutionMode %main LocalSize 1 1 1。
    emit(
        &mut m,
        OP_EXECUTION_MODE,
        &[main_id, EXEC_MODE_LOCAL_SIZE, 1, 1, 1],
    );
    // %void = OpTypeVoid。
    emit(&mut m, OP_TYPE_VOID, &[void_id]);
    // %fn_type = OpTypeFunction %void。
    emit(&mut m, OP_TYPE_FUNCTION, &[fn_type_id, void_id]);
    // %main = OpFunction %void None %fn_type。
    emit(
        &mut m,
        OP_FUNCTION,
        &[void_id, main_id, FUNCTION_CONTROL_NONE, fn_type_id],
    );
    // %label = OpLabel。
    emit(&mut m, OP_LABEL, &[label_id]);
    // OpReturn。
    emit(&mut m, OP_RETURN, &[]);
    // OpFunctionEnd。
    emit(&mut m, OP_FUNCTION_END, &[]);

    m
}

/// SPIR-V 字流 → 小端字节序 `.spv`(每字 4 字节 little-endian,R1.4)。
pub fn words_to_bytes(words: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(words.len() * 4);
    for w in words {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0201
    #[test]
    fn minimal_compute_module_header_shape() {
        let m = render_compute_module("k", "mod");
        // header 首字 = magic;bound 字(索引 3)= 5。
        assert_eq!(m[0], SPIRV_MAGIC);
        assert_eq!(m[1], SPIRV_VERSION_1_0);
        assert_eq!(m[3], 5, "bound = 最大 id + 1");
        // 含 GLCompute 入口 + LocalSize 执行模式(扫指令流)。
        let mut saw_glcompute = false;
        let mut saw_localsize = false;
        let mut i = 5;
        while i < m.len() {
            let wc = (m[i] >> 16) as usize;
            let op = (m[i] & 0xffff) as u16;
            if op == OP_ENTRY_POINT && m[i + 1] == EXEC_MODEL_GLCOMPUTE {
                saw_glcompute = true;
            }
            if op == OP_EXECUTION_MODE && m[i + 2] == EXEC_MODE_LOCAL_SIZE {
                saw_localsize = true;
            }
            i += wc.max(1);
        }
        assert!(saw_glcompute, "OpEntryPoint GLCompute 存在");
        assert!(saw_localsize, "OpExecutionMode LocalSize 存在");
    }

    //@ spec: RXS-0201
    #[test]
    fn bytes_are_little_endian() {
        let b = words_to_bytes(&[SPIRV_MAGIC]);
        assert_eq!(b, vec![0x03, 0x02, 0x23, 0x07], "magic 小端字节");
    }
}
