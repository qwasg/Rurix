//! device MIR → SPIR-V Vulkan 跨端后端 codegen(mb1,RXS-0200~0203;RFC-0011）。
//!
//! 本模块 gate 于 cargo feature `vulkan-backend`(RFC-0011 §6;未启用时整模块不编入
//! rurixc,PTX/DXIL 路径不受影响)。target 分发在 MIR 之后分叉:Vulkan 后端与 NVPTX
//! (`device_codegen`)/ DXIL(`dxil_codegen`)后端**并列**、各自从 MIR 独立降级,不共享
//! 后端 lowering(RFC-0003 §4.5 口径)。SPIR-V 是唯一中间产物:AMD 桌面驱动与 Android
//! `libvulkan.so` 都消费同一份 `.spv`(RFC-0011 §1)。
//!
//! **compute lowering(RXS-0201~0203)**:镜像 NVPTX 后端的**内存式** local 模型
//! (Function-storage `OpVariable` + `OpLoad`/`OpStore`,规避 SSA/phi 构造)。
//! - `View`/`ViewMut<global,T>` 形参 → **StorageBuffer 描述符**(SPIR-V 1.0 SSBO:
//!   `OpTypeStruct{OpTypeRuntimeArray T}` + `BufferBlock` + `DescriptorSet`/`Binding`;
//!   索引 `buf[i]` → `OpAccessChain`);
//! - 标量形参(`f32`/`u32`/`usize`)→ **push constant** 块(`Block` + `Offset`);
//! - `ThreadCtx.global_id()`(DeviceIntrinsic)→ `GlobalInvocationId` builtin;
//! - 结构化 `if`(SwitchBool)→ `OpSelectionMerge` + `OpBranchConditional`。
//!
//! 首期子集(RXS-0203):compute builtins(GlobalId/ThreadIndex/BlockIndex/Barrier)+
//! 存储缓冲 + 标量算术/比较 + 结构化 `if`;子集外(BlockDim / device fn 调用 / 数学
//! intrinsic→GLSL.std.450〔RXS-0205〕/ 循环 / 非标量 / F64·I64)→ `RX6026`。下游
//! (`.spv` → `spirv-val` clean)见 [`crate::toolchain`];真实红绿:篡改 `.spv` 字节 →
//! spirv-val 拒(红),复原绿(RFC-0011 §6)。**本片不碰** 🔒 launch marshalling FFI
//! ABI(RFC-0011 §4.7)/ Backend trait(§4.5)/ 纹理内存模型映射(06 §4.2)。

use std::collections::HashMap;

use crate::ast::BinOp;
use crate::ast::FnColor;
use crate::diag::ErrorCode;
use crate::hir::{DeviceIntrinsic, PrimTy};
use crate::mir::{
    BasicBlock, Body, CallTarget, Const, LocalIdx, Operand, Place, ProjElem, Rvalue, StatementKind,
    TerminatorKind,
};
use crate::query::QueryCtx;
use crate::resolve::Resolutions;
use crate::span::Span;
use crate::ty::Ty;

// ───────────────────────── SPIR-V 常量(核心规范取值) ─────────────────────────

const SPIRV_MAGIC: u32 = 0x0723_0203;
const SPIRV_VERSION_1_0: u32 = 0x0001_0000;
const SPIRV_GENERATOR: u32 = 0;
const SPIRV_SCHEMA: u32 = 0;

// opcodes(SPIR-V core 规范)。
const OP_EXT_INST_IMPORT: u16 = 11;
const OP_EXT_INST: u16 = 12;
const OP_MEMORY_MODEL: u16 = 14;
const OP_ENTRY_POINT: u16 = 15;
const OP_EXECUTION_MODE: u16 = 16;
const OP_CAPABILITY: u16 = 17;
const OP_TYPE_VOID: u16 = 19;
const OP_TYPE_BOOL: u16 = 20;
const OP_TYPE_INT: u16 = 21;
const OP_TYPE_FLOAT: u16 = 22;
const OP_TYPE_VECTOR: u16 = 23;
const OP_TYPE_RUNTIME_ARRAY: u16 = 29;
const OP_TYPE_STRUCT: u16 = 30;
const OP_TYPE_POINTER: u16 = 32;
const OP_TYPE_FUNCTION: u16 = 33;
const OP_CONSTANT: u16 = 43;
const OP_FUNCTION: u16 = 54;
const OP_FUNCTION_END: u16 = 56;
const OP_VARIABLE: u16 = 59;
const OP_LOAD: u16 = 61;
const OP_STORE: u16 = 62;
const OP_ACCESS_CHAIN: u16 = 65;
const OP_DECORATE: u16 = 71;
const OP_MEMBER_DECORATE: u16 = 72;
const OP_COMPOSITE_EXTRACT: u16 = 81;
const OP_SELECT: u16 = 169;
const OP_IADD: u16 = 128;
const OP_FADD: u16 = 129;
const OP_ISUB: u16 = 130;
const OP_FSUB: u16 = 131;
const OP_IMUL: u16 = 132;
const OP_FMUL: u16 = 133;
const OP_UDIV: u16 = 134;
const OP_SDIV: u16 = 135;
const OP_FDIV: u16 = 136;
const OP_UMOD: u16 = 137;
const OP_SREM: u16 = 139;
const OP_FREM: u16 = 140;
const OP_IEQUAL: u16 = 170;
const OP_INOTEQUAL: u16 = 171;
const OP_UGREATERTHAN: u16 = 172;
const OP_SGREATERTHAN: u16 = 173;
const OP_UGREATERTHANEQUAL: u16 = 174;
const OP_SGREATERTHANEQUAL: u16 = 175;
const OP_ULESSTHAN: u16 = 176;
const OP_SLESSTHAN: u16 = 177;
const OP_ULESSTHANEQUAL: u16 = 178;
const OP_SLESSTHANEQUAL: u16 = 179;
const OP_FORDEQUAL: u16 = 180;
const OP_FORDNOTEQUAL: u16 = 182;
const OP_FORDLESSTHAN: u16 = 184;
const OP_FORDGREATERTHAN: u16 = 186;
const OP_FORDLESSTHANEQUAL: u16 = 188;
const OP_FORDGREATERTHANEQUAL: u16 = 190;
const OP_CONTROL_BARRIER: u16 = 224;
const OP_SELECTION_MERGE: u16 = 247;
const OP_LABEL: u16 = 248;
const OP_BRANCH: u16 = 249;
const OP_BRANCH_CONDITIONAL: u16 = 250;
const OP_RETURN: u16 = 253;
const OP_UNREACHABLE: u16 = 255;

// 枚举取值。
const CAP_SHADER: u32 = 1;
const ADDR_MODEL_LOGICAL: u32 = 0;
const MEM_MODEL_GLSL450: u32 = 1;
const EXEC_MODEL_GLCOMPUTE: u32 = 5;
const EXEC_MODE_LOCAL_SIZE: u32 = 17;
const FUNCTION_CONTROL_NONE: u32 = 0;
const SELECTION_CONTROL_NONE: u32 = 0;

// 存储类。
const STORAGE_INPUT: u32 = 1;
const STORAGE_UNIFORM: u32 = 2;
const STORAGE_FUNCTION: u32 = 7;
const STORAGE_PUSH_CONSTANT: u32 = 9;

// decoration 取值。
const DECORATION_BLOCK: u32 = 2;
const DECORATION_BUFFER_BLOCK: u32 = 3;
const DECORATION_ARRAY_STRIDE: u32 = 6;
const DECORATION_BUILTIN: u32 = 11;
const DECORATION_BINDING: u32 = 33;
const DECORATION_DESCRIPTOR_SET: u32 = 34;
const DECORATION_OFFSET: u32 = 35;

// BuiltIn 枚举取值。
const BUILTIN_WORKGROUP_ID: u32 = 26;
const BUILTIN_LOCAL_INVOCATION_ID: u32 = 27;
const BUILTIN_GLOBAL_INVOCATION_ID: u32 = 28;

// barrier scope / memory semantics(OpControlBarrier)。
const SCOPE_WORKGROUP: u32 = 2;
const MEM_SEM_ACQUIRE_RELEASE: u32 = 0x8;
const MEM_SEM_WORKGROUP_MEMORY: u32 = 0x100;

// GLSL.std.450 扩展指令集与 ext-inst 编号(RXS-0205:__nv_* 数学 intrinsic → ext-inst)。
const EXT_GLSL_STD_450: &str = "GLSL.std.450";
const GLSL_ROUND_EVEN: u32 = 2;
const GLSL_TRUNC: u32 = 3;
const GLSL_FABS: u32 = 4;
const GLSL_FLOOR: u32 = 8;
const GLSL_CEIL: u32 = 9;
const GLSL_SIN: u32 = 13;
const GLSL_COS: u32 = 14;
const GLSL_TAN: u32 = 15;
const GLSL_POW: u32 = 26;
const GLSL_EXP: u32 = 27;
const GLSL_LOG: u32 = 28;
const GLSL_EXP2: u32 = 29;
const GLSL_LOG2: u32 = 30;
const GLSL_SQRT: u32 = 31;
const GLSL_INVERSE_SQRT: u32 = 32;
const GLSL_FMIN: u32 = 37;
const GLSL_FMAX: u32 = 40;
const GLSL_FMA: u32 = 50;

/// mb1 Vulkan codegen 目标不可用 / 暂不支持的构造 / 降级失败错误码(6xxx codegen 段;
/// 跳 RX6024/RX6025 = MS1.2b 在途占用避撞,RFC-0011 §5)。
const E_VULKAN_UNSUPPORTED: ErrorCode = ErrorCode(6026);

// ───────────────────────── 编码器 ─────────────────────────

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
    bytes.push(0);
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

/// 一个形参的分类(compute 降级模型)。
enum ParamKind {
    /// `View`/`ViewMut<space,T>` → StorageBuffer 描述符(set 0,binding = 序)。
    Buffer { binding: u32, elem: PrimTy },
    /// 标量形参 → push constant 块成员(member idx = 序)。
    Scalar { member: u32, prim: PrimTy },
    /// `ThreadCtx`(ZST)→ 不产物化。
    ThreadCtx,
}

/// SPIR-V 模块构造器(compute)。分节累积,末尾按 SPIR-V logical layout 组装。
struct Builder<'a> {
    res: &'a Resolutions,
    next_id: u32,
    // 分节字流。
    decorations: Vec<u32>,
    types_globals: Vec<u32>,
    func_vars: Vec<u32>, // Function-storage OpVariable(须列于 entry block 首)
    func_body: Vec<u32>, // entry 前导 + 各 block
    entry_interface: Vec<u32>, // OpEntryPoint 的 Input/Output 变量 id(SPIR-V 1.0)
    ext_imports: Vec<u32>, // OpExtInstImport(GLSL.std.450;layout 在 memory-model 前)
    ext_glsl: Option<u32>, // GLSL.std.450 ext-inst-set id(懒发)
    // 类型 / 常量缓存。
    type_void: Option<u32>,
    type_bool: Option<u32>,
    type_uint: Option<u32>,
    type_int: Option<u32>,
    type_float: Option<u32>,
    type_v3uint: Option<u32>,
    ptr_cache: HashMap<(u32, u32), u32>, // (storage, pointee) → ptr type id
    const_u32: HashMap<u32, u32>,
    const_f32: HashMap<u32, u32>, // bits → id
    // builtin 变量(懒发)。
    builtin_vars: HashMap<u32, u32>, // builtin enum → var id
    // local idx → Function OpVariable id(标量/临时);buffer 形参不入此表。
    local_var: HashMap<u32, u32>,
    // buffer 形参 local idx → (描述符变量 id, 元素 PrimTy)。
    buffer_var: HashMap<u32, (u32, PrimTy)>,
    // block idx → label id。
    block_label: HashMap<usize, u32>,
    main_id: u32,
}

impl<'a> Builder<'a> {
    fn new(res: &'a Resolutions) -> Self {
        Builder {
            res,
            next_id: 1,
            decorations: Vec::new(),
            types_globals: Vec::new(),
            func_vars: Vec::new(),
            func_body: Vec::new(),
            entry_interface: Vec::new(),
            ext_imports: Vec::new(),
            ext_glsl: None,
            type_void: None,
            type_bool: None,
            type_uint: None,
            type_int: None,
            type_float: None,
            type_v3uint: None,
            ptr_cache: HashMap::new(),
            const_u32: HashMap::new(),
            const_f32: HashMap::new(),
            builtin_vars: HashMap::new(),
            local_var: HashMap::new(),
            buffer_var: HashMap::new(),
            block_label: HashMap::new(),
            main_id: 0,
        }
    }

    fn fresh(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// GLSL.std.450 ext-inst-set(懒发 `OpExtInstImport`,RXS-0205)。
    fn ext_glsl_set(&mut self) -> u32 {
        if let Some(id) = self.ext_glsl {
            return id;
        }
        let id = self.fresh();
        let mut operands = vec![id];
        push_string(&mut operands, EXT_GLSL_STD_450);
        emit(&mut self.ext_imports, OP_EXT_INST_IMPORT, &operands);
        self.ext_glsl = Some(id);
        id
    }

    // -- 类型 / 常量(懒发 + 缓存) --------------------------------------------

    fn t_void(&mut self) -> u32 {
        if let Some(id) = self.type_void {
            return id;
        }
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_VOID, &[id]);
        self.type_void = Some(id);
        id
    }

    fn t_bool(&mut self) -> u32 {
        if let Some(id) = self.type_bool {
            return id;
        }
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_BOOL, &[id]);
        self.type_bool = Some(id);
        id
    }

    fn t_uint(&mut self) -> u32 {
        if let Some(id) = self.type_uint {
            return id;
        }
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_INT, &[id, 32, 0]);
        self.type_uint = Some(id);
        id
    }

    fn t_int(&mut self) -> u32 {
        if let Some(id) = self.type_int {
            return id;
        }
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_INT, &[id, 32, 1]);
        self.type_int = Some(id);
        id
    }

    fn t_float(&mut self) -> u32 {
        if let Some(id) = self.type_float {
            return id;
        }
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_FLOAT, &[id, 32]);
        self.type_float = Some(id);
        id
    }

    fn t_v3uint(&mut self) -> u32 {
        if let Some(id) = self.type_v3uint {
            return id;
        }
        let u = self.t_uint();
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_VECTOR, &[id, u, 3]);
        self.type_v3uint = Some(id);
        id
    }

    /// 标量 PrimTy → SPIR-V 类型 id。usize/u* → u32;i* → i32;f32 → float。
    /// F64/I64/U64/bool/char → 子集外(RX6026),需 Int64/Float64 cap 或非标量语义。
    fn prim_type(&mut self, p: PrimTy, span: Span) -> Result<u32, VulkanCodegenError> {
        match p {
            PrimTy::F32 => Ok(self.t_float()),
            PrimTy::Usize | PrimTy::U32 | PrimTy::U16 | PrimTy::U8 => Ok(self.t_uint()),
            PrimTy::I32 | PrimTy::I16 | PrimTy::I8 => Ok(self.t_int()),
            // bool 在内存中以 u32(0/1)表示(镜像 NVPTX i8);SSA 比较结果为 OpTypeBool,
            // 经 OpSelect 转 u32 存回(见 emit_assign 比较分支 / SwitchBool)。
            PrimTy::Bool => Ok(self.t_uint()),
            other => Err(VulkanCodegenError::unsupported(
                span,
                format!(
                    "Vulkan compute 首期标量子集暂不支持类型 {other:?}(F64/I64/U64 需 Float64/Int64 capability,后续分片)"
                ),
            )),
        }
    }

    fn ptr_type(&mut self, storage: u32, pointee: u32) -> u32 {
        if let Some(&id) = self.ptr_cache.get(&(storage, pointee)) {
            return id;
        }
        let id = self.fresh();
        emit(
            &mut self.types_globals,
            OP_TYPE_POINTER,
            &[id, storage, pointee],
        );
        self.ptr_cache.insert((storage, pointee), id);
        id
    }

    fn const_uint(&mut self, v: u32) -> u32 {
        if let Some(&id) = self.const_u32.get(&v) {
            return id;
        }
        let ty = self.t_uint();
        let id = self.fresh();
        emit(&mut self.types_globals, OP_CONSTANT, &[ty, id, v]);
        self.const_u32.insert(v, id);
        id
    }

    fn const_float_bits(&mut self, bits: u32) -> u32 {
        if let Some(&id) = self.const_f32.get(&bits) {
            return id;
        }
        let ty = self.t_float();
        let id = self.fresh();
        emit(&mut self.types_globals, OP_CONSTANT, &[ty, id, bits]);
        self.const_f32.insert(bits, id);
        id
    }

    /// builtin 变量(Input storage,`v3uint`)懒发 + 装饰 + 入 entry interface。
    fn builtin_var(&mut self, builtin: u32) -> u32 {
        if let Some(&id) = self.builtin_vars.get(&builtin) {
            return id;
        }
        let v3 = self.t_v3uint();
        let ptr = self.ptr_type(STORAGE_INPUT, v3);
        let id = self.fresh();
        emit(
            &mut self.types_globals,
            OP_VARIABLE,
            &[ptr, id, STORAGE_INPUT],
        );
        emit(
            &mut self.decorations,
            OP_DECORATE,
            &[id, DECORATION_BUILTIN, builtin],
        );
        self.entry_interface.push(id);
        self.builtin_vars.insert(builtin, id);
        id
    }
}

/// device intrinsic(index 类)→ (builtin 枚举, 分量索引)。BlockDim / Barrier 另处。
fn intrinsic_builtin(intr: DeviceIntrinsic) -> Option<(u32, u32)> {
    match intr {
        DeviceIntrinsic::GlobalIdX => Some((BUILTIN_GLOBAL_INVOCATION_ID, 0)),
        DeviceIntrinsic::GlobalIdY => Some((BUILTIN_GLOBAL_INVOCATION_ID, 1)),
        DeviceIntrinsic::GlobalIdZ => Some((BUILTIN_GLOBAL_INVOCATION_ID, 2)),
        DeviceIntrinsic::ThreadIndexX => Some((BUILTIN_LOCAL_INVOCATION_ID, 0)),
        DeviceIntrinsic::ThreadIndexY => Some((BUILTIN_LOCAL_INVOCATION_ID, 1)),
        DeviceIntrinsic::ThreadIndexZ => Some((BUILTIN_LOCAL_INVOCATION_ID, 2)),
        DeviceIntrinsic::BlockIndexX => Some((BUILTIN_WORKGROUP_ID, 0)),
        DeviceIntrinsic::BlockIndexY => Some((BUILTIN_WORKGROUP_ID, 1)),
        DeviceIntrinsic::BlockIndexZ => Some((BUILTIN_WORKGROUP_ID, 2)),
        _ => None,
    }
}

/// 前向可达块集(从 `start`,不跨回边;结构化 Rust MIR 为 DAG-ish)。
fn forward_reachable(body: &Body, start: usize) -> Vec<usize> {
    let mut seen = vec![false; body.blocks.len()];
    let mut stack = vec![start];
    let mut out = Vec::new();
    while let Some(b) = stack.pop() {
        if b >= body.blocks.len() || seen[b] {
            continue;
        }
        seen[b] = true;
        out.push(b);
        for succ in block_succs(&body.blocks[b]) {
            if !seen[succ] {
                stack.push(succ);
            }
        }
    }
    out
}

fn block_succs(bb: &BasicBlock) -> Vec<usize> {
    match &bb.terminator.kind {
        TerminatorKind::Goto(t) => vec![t.0 as usize],
        TerminatorKind::SwitchBool { then, else_, .. } => vec![then.0 as usize, else_.0 as usize],
        TerminatorKind::Call { next, .. } => vec![next.0 as usize],
        TerminatorKind::Drop { next, .. } => vec![next.0 as usize],
        TerminatorKind::Return | TerminatorKind::Unreachable => vec![],
    }
}

// ───────────────────────── 主降级入口 ─────────────────────────

/// 驱动 / 测试入口:构建 device MIR(`kernel fn` 为根)+ SPIR-V compute codegen。
/// 无 kernel → `None`;子集外 / 降级失败 → 经 `cx.diag()` 落 `RX6026` 并返回 `None`;
/// 成功 → `Some(SPIR-V 字流)`。`.spv` 落盘 + `spirv-val` gate 由驱动另行实施。
pub fn build_and_emit_vulkan(cx: &QueryCtx<'_>, _module_name: &str) -> Option<Vec<u32>> {
    let bodies = cx.device_mir_crate();
    if bodies.is_empty() {
        return None;
    }
    if cx.diag().has_errors() {
        return None;
    }
    let res = cx.resolutions();
    let entry = bodies.iter().find(|b| b.color == FnColor::Kernel)?;
    // 图形阶段(vertex/fragment,`stage=Some`)→ 复用 dxil_spirv SPIR-V 编码器
    // (RXS-0204;RFC-0004 种子,Vulkan 原生消费,去 B 路 SPIRV-Cross→HLSL→dxc 转译链)。
    // compute(`stage=None`,color=Kernel)→ compute lowerer(RXS-0201~0203)。
    if let Some(stage) = entry.stage {
        // Vulkan 原生消费入口(RXS-0210 方案 B):去 UserSemantic/SPV_GOOGLE provenance
        // (保名仅 B 路 HLSL 转译需要)→ `.spv` 免 device 扩展依赖直喂 vkCreateShaderModule
        // (修 VUID-VkShaderModuleCreateInfo-pCode-08742)。DXIL 路 emit_spirv_body 字节不变。
        return match crate::dxil_spirv::emit_spirv_body_vulkan(stage, entry) {
            Ok(words) => Some(words),
            Err(e) => {
                cx.diag()
                    .struct_error(E_VULKAN_UNSUPPORTED, "codegen.vulkan_unsupported")
                    .arg("detail", format!("graphics 阶段 MIR→SPIR-V 降级: {e}"))
                    .span_label(entry.span, "in Vulkan graphics entry")
                    .emit();
                None
            }
        };
    }
    match lower_compute(entry, &res) {
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

/// 单个 compute kernel body → SPIR-V 字流(RXS-0201~0203)。
pub fn lower_compute(body: &Body, res: &Resolutions) -> Result<Vec<u32>, VulkanCodegenError> {
    let mut b = Builder::new(res);
    b.main_id = b.fresh();

    // 形参分类(locals 1..=arg_count):buffer / scalar / ThreadCtx。
    let mut params: Vec<(LocalIdx, ParamKind)> = Vec::new();
    let mut next_binding = 0u32;
    let mut next_member = 0u32;
    for i in 1..=body.arg_count {
        let li = LocalIdx(i as u32);
        let ty = &body.locals[i].ty;
        let span = body.locals[i].span;
        let kind = classify_param(&mut b, ty, span, &mut next_binding, &mut next_member)?;
        params.push((li, kind));
    }

    // 描述符 / push-constant 全局变量发射。
    emit_buffer_descriptors(&mut b, &params, body)?;
    let pc_var = emit_push_constants(&mut b, &params, body)?;

    // 预分配 block label id。
    for bi in 0..body.blocks.len() {
        let id = b.fresh();
        b.block_label.insert(bi, id);
    }

    // Function-storage local 变量(非 ZST、非 buffer 形参、非 ret slot〔kernel void〕)。
    // scalar 形参也建 Function local(entry 处从 push-constant 拷入),body 统一按 local 处理。
    for (i, l) in body.locals.iter().enumerate() {
        if i == 0 {
            continue; // ret slot(kernel = void)
        }
        if b.buffer_var.contains_key(&(i as u32)) {
            continue; // buffer 形参 → 描述符,不建 Function local
        }
        if is_zst(res, &l.ty) {
            continue;
        }
        let elem = prim_of(&l.ty).ok_or_else(|| {
            VulkanCodegenError::unsupported(
                l.span,
                "Vulkan compute local 首期仅支持标量类型(非标量 local 属后续分片)",
            )
        })?;
        let ty_id = b.prim_type(elem, l.span)?;
        let ptr = b.ptr_type(STORAGE_FUNCTION, ty_id);
        let var = b.fresh();
        emit(&mut b.func_vars, OP_VARIABLE, &[ptr, var, STORAGE_FUNCTION]);
        b.local_var.insert(i as u32, var);
    }

    // entry 前导:scalar 形参从 push-constant 拷入其 Function local。
    for (li, kind) in &params {
        if let ParamKind::Scalar { member, prim } = kind {
            let pc = pc_var.expect("有 scalar 形参则 push-constant 块已建");
            let ty_id = b.prim_type(*prim, body.locals[li.0 as usize].span)?;
            let ptr_pc = b.ptr_type(STORAGE_PUSH_CONSTANT, ty_id);
            let midx = b.const_uint(*member);
            let acc = b.fresh();
            emit(&mut b.func_body, OP_ACCESS_CHAIN, &[ptr_pc, acc, pc, midx]);
            let val = b.fresh();
            emit(&mut b.func_body, OP_LOAD, &[ty_id, val, acc]);
            let local = b.local_var[&li.0];
            emit(&mut b.func_body, OP_STORE, &[local, val]);
        }
    }
    // entry → bb0。
    let bb0 = b.block_label[&0];
    emit(&mut b.func_body, OP_BRANCH, &[bb0]);

    // 各 block 降级。
    for (bi, bb) in body.blocks.iter().enumerate() {
        let label = b.block_label[&bi];
        emit(&mut b.func_body, OP_LABEL, &[label]);
        for st in &bb.stmts {
            let StatementKind::Assign(place, rv) = &st.kind;
            emit_assign(&mut b, body, place, rv)?;
        }
        emit_terminator(&mut b, body, bi)?;
    }

    Ok(assemble(&mut b, &body.symbol))
}

/// 形参分类 + buffer binding / scalar member 计数递增。
fn classify_param(
    b: &mut Builder,
    ty: &Ty,
    span: Span,
    next_binding: &mut u32,
    next_member: &mut u32,
) -> Result<ParamKind, VulkanCodegenError> {
    if is_zst(b.res, ty) {
        return Ok(ParamKind::ThreadCtx);
    }
    if let Ty::Adt(d, args) = ty
        && b.res.lang_items.view_mutable(*d).is_some()
    {
        let elem = args.get(1).and_then(prim_of).ok_or_else(|| {
            VulkanCodegenError::unsupported(
                span,
                "Vulkan compute 存储缓冲元素首期仅支持标量类型(View<space,T> 的 T)",
            )
        })?;
        let binding = *next_binding;
        *next_binding += 1;
        return Ok(ParamKind::Buffer { binding, elem });
    }
    if let Some(p) = prim_of(ty) {
        let member = *next_member;
        *next_member += 1;
        return Ok(ParamKind::Scalar { member, prim: p });
    }
    Err(VulkanCodegenError::unsupported(
        span,
        "Vulkan compute 形参首期仅支持 View/ViewMut<space,T> 存储缓冲、标量、ThreadCtx",
    ))
}

/// 每个 buffer 形参 → StorageBuffer 描述符(SPIR-V 1.0 SSBO)。
fn emit_buffer_descriptors(
    b: &mut Builder,
    params: &[(LocalIdx, ParamKind)],
    body: &Body,
) -> Result<(), VulkanCodegenError> {
    for (li, kind) in params {
        if let ParamKind::Buffer { binding, elem } = kind {
            let elem_ty = b.prim_type(*elem, body.locals[li.0 as usize].span)?;
            let stride = 4u32; // f32/i32/u32 均 4 字节。
            // OpTypeRuntimeArray T(ArrayStride)。
            let rarr = b.fresh();
            emit(
                &mut b.types_globals,
                OP_TYPE_RUNTIME_ARRAY,
                &[rarr, elem_ty],
            );
            emit(
                &mut b.decorations,
                OP_DECORATE,
                &[rarr, DECORATION_ARRAY_STRIDE, stride],
            );
            // OpTypeStruct { rarr }(BufferBlock,member 0 Offset 0)。
            let st = b.fresh();
            emit(&mut b.types_globals, OP_TYPE_STRUCT, &[st, rarr]);
            emit(
                &mut b.decorations,
                OP_MEMBER_DECORATE,
                &[st, 0, DECORATION_OFFSET, 0],
            );
            emit(
                &mut b.decorations,
                OP_DECORATE,
                &[st, DECORATION_BUFFER_BLOCK],
            );
            // 变量(Uniform storage,set 0 / binding)。
            let ptr = b.ptr_type(STORAGE_UNIFORM, st);
            let var = b.fresh();
            emit(
                &mut b.types_globals,
                OP_VARIABLE,
                &[ptr, var, STORAGE_UNIFORM],
            );
            emit(
                &mut b.decorations,
                OP_DECORATE,
                &[var, DECORATION_DESCRIPTOR_SET, 0],
            );
            emit(
                &mut b.decorations,
                OP_DECORATE,
                &[var, DECORATION_BINDING, *binding],
            );
            b.buffer_var.insert(li.0, (var, *elem));
        }
    }
    Ok(())
}

/// 若有 scalar 形参 → 单个 push constant 块(`Block` + member `Offset`)。返回其变量 id。
fn emit_push_constants(
    b: &mut Builder,
    params: &[(LocalIdx, ParamKind)],
    body: &Body,
) -> Result<Option<u32>, VulkanCodegenError> {
    let scalars: Vec<(u32, PrimTy)> = params
        .iter()
        .filter_map(|(_, k)| match k {
            ParamKind::Scalar { member, prim } => Some((*member, *prim)),
            _ => None,
        })
        .collect();
    if scalars.is_empty() {
        return Ok(None);
    }
    let mut member_tys = Vec::new();
    for (_, p) in &scalars {
        member_tys.push(b.prim_type(*p, body.span)?);
    }
    let st = b.fresh();
    let mut operands = vec![st];
    operands.extend_from_slice(&member_tys);
    emit(&mut b.types_globals, OP_TYPE_STRUCT, &operands);
    // 成员 Offset(均 4 字节标量,顺排)。
    for (i, _) in scalars.iter().enumerate() {
        emit(
            &mut b.decorations,
            OP_MEMBER_DECORATE,
            &[st, i as u32, DECORATION_OFFSET, (i as u32) * 4],
        );
    }
    emit(&mut b.decorations, OP_DECORATE, &[st, DECORATION_BLOCK]);
    let ptr = b.ptr_type(STORAGE_PUSH_CONSTANT, st);
    let var = b.fresh();
    emit(
        &mut b.types_globals,
        OP_VARIABLE,
        &[ptr, var, STORAGE_PUSH_CONSTANT],
    );
    Ok(Some(var))
}

// ───────────────────────── 语句 / place / operand ─────────────────────────

/// place 解析 → (指针 id, 元素 SPIR-V 类型 id, 元素 PrimTy)。
/// - buffer 形参 + `[Index(idx)]` → `OpAccessChain(var, uint_0, idx)`(StorageBuffer 元素);
/// - Function local(无投影)→ 其 OpVariable id;
/// - 其余 → RX6026。
fn place_ptr(
    b: &mut Builder,
    body: &Body,
    p: &Place,
) -> Result<(u32, u32, PrimTy), VulkanCodegenError> {
    let span = body.locals[p.local.0 as usize].span;
    if p.proj.is_empty() {
        // Function local(标量/临时/scalar 形参 copy)。
        let prim = prim_of(&body.locals[p.local.0 as usize].ty)
            .ok_or_else(|| VulkanCodegenError::unsupported(span, "非标量 local 访问属后续分片"))?;
        let ty_id = b.prim_type(prim, span)?;
        let var = *b.local_var.get(&p.local.0).ok_or_else(|| {
            VulkanCodegenError::unsupported(
                span,
                "对未建 Function 变量的 local 访问(可能是 buffer 形参裸引用,子集外)",
            )
        })?;
        return Ok((var, ty_id, prim));
    }
    if let [ProjElem::Index(idx_local)] = p.proj.as_slice()
        && let Some((var, elem)) = b.buffer_var.get(&p.local.0).copied()
    {
        let elem_ty = b.prim_type(elem, span)?;
        let ptr_elem = b.ptr_type(STORAGE_UNIFORM, elem_ty);
        let idx_val = load_local(b, body, *idx_local)?;
        let member0 = b.const_uint(0);
        let acc = b.fresh();
        emit(
            &mut b.func_body,
            OP_ACCESS_CHAIN,
            &[ptr_elem, acc, var, member0, idx_val],
        );
        return Ok((acc, elem_ty, elem));
    }
    Err(VulkanCodegenError::unsupported(
        span,
        "Vulkan compute place 首期仅支持标量 local 与 buffer[index](数组/字段/deref 投影属后续分片)",
    ))
}

/// 载入一个标量 Function local 的值 → SPIR-V id。
fn load_local(b: &mut Builder, body: &Body, l: LocalIdx) -> Result<u32, VulkanCodegenError> {
    let span = body.locals[l.0 as usize].span;
    let prim = prim_of(&body.locals[l.0 as usize].ty)
        .ok_or_else(|| VulkanCodegenError::unsupported(span, "非标量 local 载入属后续分片"))?;
    let ty_id = b.prim_type(prim, span)?;
    let var = *b.local_var.get(&l.0).ok_or_else(|| {
        VulkanCodegenError::unsupported(span, "对未建 Function 变量的 local 载入")
    })?;
    let id = b.fresh();
    emit(&mut b.func_body, OP_LOAD, &[ty_id, id, var]);
    Ok(id)
}

/// operand → (值 id, 元素 SPIR-V 类型 id, PrimTy);unit/ZST → None。
fn operand(
    b: &mut Builder,
    body: &Body,
    o: &Operand,
) -> Result<Option<(u32, u32, PrimTy)>, VulkanCodegenError> {
    match o {
        Operand::Copy(p) | Operand::Move(p) => {
            let (ptr, ty_id, prim) = place_ptr(b, body, p)?;
            let id = b.fresh();
            emit(&mut b.func_body, OP_LOAD, &[ty_id, id, ptr]);
            Ok(Some((id, ty_id, prim)))
        }
        Operand::Const(c) => match c {
            Const::Unit => Ok(None),
            Const::Int(v, p) => {
                let ty_id = b.prim_type(*p, body.span)?;
                let val = (*v as i64) as u32; // 32-bit 截断(usize/u32/i32 子集)
                // 无符号走 u32 常量缓存;i32 单独发(位型同但结果类型不同,不复用缓存)。
                let id = if is_signed_prim(*p) {
                    let idn = b.fresh();
                    emit(&mut b.types_globals, OP_CONSTANT, &[ty_id, idn, val]);
                    idn
                } else {
                    b.const_uint(val)
                };
                Ok(Some((id, ty_id, *p)))
            }
            Const::Float(v, p) => {
                if !matches!(p, PrimTy::F32) {
                    return Err(VulkanCodegenError::unsupported(
                        body.span,
                        "Vulkan compute 首期浮点仅 f32(F64 需 Float64 capability)",
                    ));
                }
                let bits = (*v as f32).to_bits();
                let id = b.const_float_bits(bits);
                let ty_id = b.t_float();
                Ok(Some((id, ty_id, PrimTy::F32)))
            }
            Const::Bool(_) | Const::Char(_) | Const::Str(_) => {
                Err(VulkanCodegenError::unsupported(
                    body.span,
                    "Vulkan compute 首期常量仅标量整数/f32(bool/char/str 属后续分片)",
                ))
            }
            Const::GlobalAddr(_) => Err(VulkanCodegenError::unsupported(
                body.span,
                "Vulkan device codegen 不含全局常量地址(@__rx_gpu_artifacts 描述表指针属 MS1.2 host 编排 codegen,非 device compute/graphics 作用面)",
            )),
        },
    }
}

fn emit_assign(
    b: &mut Builder,
    body: &Body,
    place: &Place,
    rv: &Rvalue,
) -> Result<(), VulkanCodegenError> {
    match rv {
        Rvalue::Use(o) => {
            let Some((val, _, _)) = operand(b, body, o)? else {
                return Ok(()); // unit 赋值 no-op(空体语义)。
            };
            let (ptr, _, _) = place_ptr(b, body, place)?;
            emit(&mut b.func_body, OP_STORE, &[ptr, val]);
            Ok(())
        }
        Rvalue::BinaryOp(op, a, c) => {
            let Some((va, ty_id, prim)) = operand(b, body, a)? else {
                return Ok(());
            };
            let Some((vc, _, _)) = operand(b, body, c)? else {
                return Ok(());
            };
            let is_float = matches!(prim, PrimTy::F32);
            let is_signed = is_signed_prim(prim);
            let (opcode, result_bool) = binop_opcode(*op, is_float, is_signed, body.span)?;
            if result_bool {
                // 比较 → OpTypeBool 结果 → OpSelect 为 u32(0/1)存入 place(镜像 NVPTX i8)。
                let bool_ty = b.t_bool();
                let cmp = b.fresh();
                emit(&mut b.func_body, opcode, &[bool_ty, cmp, va, vc]);
                let (ptr, dst_ty, _) = place_ptr(b, body, place)?;
                let one = b.const_uint(1);
                let zero = b.const_uint(0);
                let sel = b.fresh();
                emit(&mut b.func_body, OP_SELECT, &[dst_ty, sel, cmp, one, zero]);
                emit(&mut b.func_body, OP_STORE, &[ptr, sel]);
            } else {
                let res = b.fresh();
                emit(&mut b.func_body, opcode, &[ty_id, res, va, vc]);
                let (ptr, _, _) = place_ptr(b, body, place)?;
                emit(&mut b.func_body, OP_STORE, &[ptr, res]);
            }
            Ok(())
        }
        _ => Err(VulkanCodegenError::unsupported(
            body.span,
            "Vulkan compute 首期 rvalue 仅 Use / BinaryOp(Cast/UnaryOp/Ref/Aggregate/纹理采样属后续分片)",
        )),
    }
}

/// BinOp → (SPIR-V opcode, 结果是否 bool)。
fn binop_opcode(
    op: BinOp,
    is_float: bool,
    is_signed: bool,
    span: Span,
) -> Result<(u16, bool), VulkanCodegenError> {
    let oc = match op {
        BinOp::Add => (if is_float { OP_FADD } else { OP_IADD }, false),
        BinOp::Sub => (if is_float { OP_FSUB } else { OP_ISUB }, false),
        BinOp::Mul => (if is_float { OP_FMUL } else { OP_IMUL }, false),
        BinOp::Div => (
            if is_float {
                OP_FDIV
            } else if is_signed {
                OP_SDIV
            } else {
                OP_UDIV
            },
            false,
        ),
        BinOp::Rem => (
            if is_float {
                OP_FREM
            } else if is_signed {
                OP_SREM
            } else {
                OP_UMOD
            },
            false,
        ),
        BinOp::Eq => (if is_float { OP_FORDEQUAL } else { OP_IEQUAL }, true),
        BinOp::Ne => (
            if is_float {
                OP_FORDNOTEQUAL
            } else {
                OP_INOTEQUAL
            },
            true,
        ),
        BinOp::Lt => (
            cmp_op(
                is_float,
                is_signed,
                OP_FORDLESSTHAN,
                OP_SLESSTHAN,
                OP_ULESSTHAN,
            ),
            true,
        ),
        BinOp::Gt => (
            cmp_op(
                is_float,
                is_signed,
                OP_FORDGREATERTHAN,
                OP_SGREATERTHAN,
                OP_UGREATERTHAN,
            ),
            true,
        ),
        BinOp::Le => (
            cmp_op(
                is_float,
                is_signed,
                OP_FORDLESSTHANEQUAL,
                OP_SLESSTHANEQUAL,
                OP_ULESSTHANEQUAL,
            ),
            true,
        ),
        BinOp::Ge => (
            cmp_op(
                is_float,
                is_signed,
                OP_FORDGREATERTHANEQUAL,
                OP_SGREATERTHANEQUAL,
                OP_UGREATERTHANEQUAL,
            ),
            true,
        ),
        BinOp::BitAnd
        | BinOp::BitOr
        | BinOp::BitXor
        | BinOp::Shl
        | BinOp::Shr
        | BinOp::And
        | BinOp::Or => {
            return Err(VulkanCodegenError::unsupported(
                span,
                "Vulkan compute 首期算术仅 +−*/% 与比较(位运算/逻辑属后续分片)",
            ));
        }
    };
    Ok(oc)
}

fn cmp_op(is_float: bool, is_signed: bool, f: u16, s: u16, u: u16) -> u16 {
    if is_float {
        f
    } else if is_signed {
        s
    } else {
        u
    }
}

// ───────────────────────── 终结子 / 调用 / intrinsic ─────────────────────────

fn emit_terminator(b: &mut Builder, body: &Body, bi: usize) -> Result<(), VulkanCodegenError> {
    let bb = &body.blocks[bi];
    match &bb.terminator.kind {
        TerminatorKind::Goto(t) => {
            let lbl = b.block_label[&(t.0 as usize)];
            emit(&mut b.func_body, OP_BRANCH, &[lbl]);
        }
        TerminatorKind::Return => {
            emit(&mut b.func_body, OP_RETURN, &[]);
        }
        TerminatorKind::Unreachable => {
            emit(&mut b.func_body, OP_UNREACHABLE, &[]);
        }
        TerminatorKind::Drop { next, .. } => {
            let lbl = b.block_label[&(next.0 as usize)];
            emit(&mut b.func_body, OP_BRANCH, &[lbl]);
        }
        TerminatorKind::Call {
            target,
            args,
            dest,
            next,
        } => {
            emit_call(b, body, target, args, dest, bb.terminator.span)?;
            let lbl = b.block_label[&(next.0 as usize)];
            emit(&mut b.func_body, OP_BRANCH, &[lbl]);
        }
        TerminatorKind::SwitchBool { discr, then, else_ } => {
            // discr(u32 0/1)载入 → INotEqual 0 → OpTypeBool。
            let Some((dv, _, _)) = operand(b, body, discr)? else {
                return Err(VulkanCodegenError::unsupported(
                    bb.terminator.span,
                    "switch on zero-sized value",
                ));
            };
            let bool_ty = b.t_bool();
            let zero = b.const_uint(0);
            let cond = b.fresh();
            emit(&mut b.func_body, OP_INOTEQUAL, &[bool_ty, cond, dv, zero]);
            // 结构化 merge 块。
            let then_i = then.0 as usize;
            let else_i = else_.0 as usize;
            let merge = structured_merge(body, then_i, else_i).ok_or_else(|| {
                VulkanCodegenError::unsupported(
                    bb.terminator.span,
                    "Vulkan compute 首期仅支持结构化 if(分支须收敛于唯一 merge 块;循环/提前 return 属后续分片)",
                )
            })?;
            let merge_lbl = b.block_label[&merge];
            let then_lbl = b.block_label[&then_i];
            let else_lbl = b.block_label[&else_i];
            emit(
                &mut b.func_body,
                OP_SELECTION_MERGE,
                &[merge_lbl, SELECTION_CONTROL_NONE],
            );
            emit(
                &mut b.func_body,
                OP_BRANCH_CONDITIONAL,
                &[cond, then_lbl, else_lbl],
            );
        }
    }
    Ok(())
}

/// 结构化 if 的 merge 块 = 前向可达(then)∩ 前向可达(else),取最小块下标。
fn structured_merge(body: &Body, then_i: usize, else_i: usize) -> Option<usize> {
    let rt = forward_reachable(body, then_i);
    let re = forward_reachable(body, else_i);
    rt.iter().filter(|x| re.contains(x)).copied().min()
}

/// libdevice `__nv_*` 数学符号 → (GLSL.std.450 ext-inst 编号, arity)。RXS-0205 首期覆盖
/// 20 个 `DeviceMathFn` 中的 1:1 可映射项;`cbrt`/`log10`(需 Pow/Log 组合)→ None(后续
/// 分片)。符号形态:`__nv_<base>` (f64) / `__nv_<base>f` (f32);base 无一以 'f' 结尾,
/// 故 strip 尾 'f' 唯一恢复 base(ext-inst 按操作数类型分发,f32/f64 同一编号)。
fn glsl_ext_op(nv_symbol: &str) -> Option<(u32, usize)> {
    let s = nv_symbol.strip_prefix("__nv_")?;
    let base = s.strip_suffix('f').unwrap_or(s);
    let m = match base {
        "sqrt" => (GLSL_SQRT, 1),
        "rsqrt" => (GLSL_INVERSE_SQRT, 1),
        "exp" => (GLSL_EXP, 1),
        "exp2" => (GLSL_EXP2, 1),
        "log" => (GLSL_LOG, 1),
        "log2" => (GLSL_LOG2, 1),
        "sin" => (GLSL_SIN, 1),
        "cos" => (GLSL_COS, 1),
        "tan" => (GLSL_TAN, 1),
        "floor" => (GLSL_FLOOR, 1),
        "ceil" => (GLSL_CEIL, 1),
        "trunc" => (GLSL_TRUNC, 1),
        "round" => (GLSL_ROUND_EVEN, 1),
        "fabs" => (GLSL_FABS, 1),
        "pow" => (GLSL_POW, 2),
        "fmin" => (GLSL_FMIN, 2),
        "fmax" => (GLSL_FMAX, 2),
        "fma" => (GLSL_FMA, 3),
        _ => return None, // cbrt / log10 需组合 → 后续分片
    };
    Some(m)
}

fn emit_call(
    b: &mut Builder,
    body: &Body,
    target: &CallTarget,
    args: &[Operand],
    dest: &Place,
    span: Span,
) -> Result<(), VulkanCodegenError> {
    match target {
        CallTarget::DeviceIntrinsic(intr) => {
            if let DeviceIntrinsic::Barrier = intr {
                let scope = b.const_uint(SCOPE_WORKGROUP);
                let sem = b.const_uint(MEM_SEM_ACQUIRE_RELEASE | MEM_SEM_WORKGROUP_MEMORY);
                // OpControlBarrier ExecScope MemScope Semantics(均 Workgroup)。
                emit(&mut b.func_body, OP_CONTROL_BARRIER, &[scope, scope, sem]);
                return Ok(());
            }
            let Some((builtin, comp)) = intrinsic_builtin(*intr) else {
                return Err(VulkanCodegenError::unsupported(
                    span,
                    "Vulkan compute 首期 device intrinsic 支持 global_id/thread_index/block_index/sync(block_dim 属后续分片)",
                ));
            };
            let var = b.builtin_var(builtin);
            let v3 = b.t_v3uint();
            let loaded = b.fresh();
            emit(&mut b.func_body, OP_LOAD, &[v3, loaded, var]);
            let uint = b.t_uint();
            let elem = b.fresh();
            emit(
                &mut b.func_body,
                OP_COMPOSITE_EXTRACT,
                &[uint, elem, loaded, comp],
            );
            let (ptr, _, _) = place_ptr(b, body, dest)?;
            emit(&mut b.func_body, OP_STORE, &[ptr, elem]);
            Ok(())
        }
        CallTarget::Libdevice { symbol } => {
            // 数学 intrinsic `__nv_*` → GLSL.std.450 ext-inst(RXS-0205)。首期 f32,
            // 结果类型 = float;操作数经 operand 载入。cbrt/log10 需组合表达 → 后续分片。
            let Some((glsl_op, arity)) = glsl_ext_op(symbol) else {
                return Err(VulkanCodegenError::unsupported(
                    span,
                    format!(
                        "Vulkan compute 数学 intrinsic `{symbol}` 未映射(cbrt/log10 需 GLSL.std.450 组合表达,后续分片)"
                    ),
                ));
            };
            if args.len() != arity {
                return Err(VulkanCodegenError::unsupported(
                    span,
                    format!(
                        "数学 intrinsic `{symbol}` 期望 {arity} 实参,得 {}",
                        args.len()
                    ),
                ));
            }
            let set = b.ext_glsl_set();
            let float_ty = b.t_float();
            let result = b.fresh();
            // OpExtInst = [result_type, result_id, set, instruction, arg0, ...]。
            let mut operands = vec![float_ty, result, set, glsl_op];
            for a in args {
                let Some((v, _, _)) = operand(b, body, a)? else {
                    return Err(VulkanCodegenError::unsupported(
                        span,
                        "数学 intrinsic 零尺寸实参",
                    ));
                };
                operands.push(v);
            }
            emit(&mut b.func_body, OP_EXT_INST, &operands);
            let (ptr, _, _) = place_ptr(b, body, dest)?;
            emit(&mut b.func_body, OP_STORE, &[ptr, result]);
            Ok(())
        }
        CallTarget::Fn { .. } => Err(VulkanCodegenError::unsupported(
            span,
            "Vulkan compute device fn 调用(内联)属后续分片",
        )),
        CallTarget::Builtin(_) => Err(VulkanCodegenError::unsupported(
            span,
            "host builtin 调用不在 device compute codegen 作用面",
        )),
        CallTarget::Rt { .. } => Err(VulkanCodegenError::unsupported(
            span,
            "宿主 GPU 编排运行时符号 rxrt_* 调用(MS1.2,host-only)不在 device compute/graphics codegen 作用面",
        )),
    }
}

// ───────────────────────── 模块组装 ─────────────────────────

/// 按 SPIR-V logical layout 组装最终字流。
fn assemble(b: &mut Builder, entry_name: &str) -> Vec<u32> {
    let void_id = b.t_void();
    let fn_ty = {
        let id = b.fresh();
        emit(&mut b.types_globals, OP_TYPE_FUNCTION, &[id, void_id]);
        id
    };
    let entry_label = b.fresh();
    let bound = b.next_id;

    let mut m: Vec<u32> = vec![
        SPIRV_MAGIC,
        SPIRV_VERSION_1_0,
        SPIRV_GENERATOR,
        bound,
        SPIRV_SCHEMA,
    ];
    emit(&mut m, OP_CAPABILITY, &[CAP_SHADER]);
    // OpExtInstImport(GLSL.std.450 等)layout 在 memory-model 之前。
    m.extend_from_slice(&b.ext_imports);
    emit(
        &mut m,
        OP_MEMORY_MODEL,
        &[ADDR_MODEL_LOGICAL, MEM_MODEL_GLSL450],
    );
    // OpEntryPoint GLCompute %main "<entry>" <interface...>。
    let mut ep = vec![EXEC_MODEL_GLCOMPUTE, b.main_id];
    push_string(&mut ep, entry_name);
    ep.extend_from_slice(&b.entry_interface);
    emit(&mut m, OP_ENTRY_POINT, &ep);
    emit(
        &mut m,
        OP_EXECUTION_MODE,
        &[b.main_id, EXEC_MODE_LOCAL_SIZE, 1, 1, 1],
    );
    // decorations。
    m.extend_from_slice(&b.decorations);
    // types / consts / global vars。
    m.extend_from_slice(&b.types_globals);
    // function。
    emit(
        &mut m,
        OP_FUNCTION,
        &[void_id, b.main_id, FUNCTION_CONTROL_NONE, fn_ty],
    );
    emit(&mut m, OP_LABEL, &[entry_label]);
    m.extend_from_slice(&b.func_vars);
    m.extend_from_slice(&b.func_body);
    emit(&mut m, OP_FUNCTION_END, &[]);
    m
}

// ───────────────────────── 类型辅助 ─────────────────────────

/// 零尺寸(unit 或 ThreadCtx)。
fn is_zst(res: &Resolutions, ty: &Ty) -> bool {
    match ty {
        Ty::Tuple(v) => v.is_empty(),
        Ty::Adt(d, _) => res.lang_items.is_thread_ctx(*d),
        _ => false,
    }
}

/// 标量 PrimTy(非标量 → None)。
fn prim_of(ty: &Ty) -> Option<PrimTy> {
    match ty {
        Ty::Prim(p) => Some(*p),
        _ => None,
    }
}

fn is_signed_prim(p: PrimTy) -> bool {
    matches!(p, PrimTy::I8 | PrimTy::I16 | PrimTy::I32 | PrimTy::I64)
}

/// SPIR-V 字流 → 小端字节序 `.spv`。
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
    fn instruction_word_count_encoding() {
        let mut s = Vec::new();
        emit(&mut s, OP_CAPABILITY, &[CAP_SHADER]);
        // OpCapability = 1 operand + 1 首字 = word_count 2。
        assert_eq!(s[0] >> 16, 2);
        assert_eq!(s[0] & 0xffff, u32::from(OP_CAPABILITY));
        assert_eq!(s[1], CAP_SHADER);
    }

    //@ spec: RXS-0201
    #[test]
    fn string_is_nul_terminated_and_padded() {
        let mut ops = Vec::new();
        push_string(&mut ops, "main");
        // "main\0\0\0\0" = 8 字节 = 2 字。
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0], u32::from_le_bytes([b'm', b'a', b'i', b'n']));
        assert_eq!(ops[1], 0);
    }

    //@ spec: RXS-0201
    #[test]
    fn bytes_are_little_endian() {
        let b = words_to_bytes(&[SPIRV_MAGIC]);
        assert_eq!(b, vec![0x03, 0x02, 0x23, 0x07]);
    }
}
