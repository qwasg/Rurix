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
//! - 标量形参(`f32`/`u32`/`usize`/`i64`/`u64`)→ **push constant** 块(`Block` + `Offset`);
//! - `ThreadCtx.global_id()`(DeviceIntrinsic)→ `GlobalInvocationId` builtin;
//! - 结构化 `if`(SwitchBool)→ `OpSelectionMerge` + `OpBranchConditional`。
//!
//! 首期子集(RXS-0203):compute builtins(GlobalId/ThreadIndex/BlockIndex/Barrier)+
//! 存储缓冲 + 标量算术/比较 + 结构化 `if`;子集外(BlockDim / device fn 调用 / 数学
//! intrinsic→GLSL.std.450〔RXS-0205〕/ 循环 / 非标量 / F64)→ `RX6026`。下游
//! (`.spv` → `spirv-val` clean)见 [`crate::toolchain`];真实红绿:篡改 `.spv` 字节 →
//! spirv-val 拒(红),复原绿(RFC-0011 §6)。**本片不碰** 🔒 launch marshalling FFI
//! ABI(RFC-0011 §4.7)/ Backend trait(§4.5)/ 纹理内存模型映射(06 §4.2)。

use std::collections::HashMap;

use crate::ast::BinOp;
use crate::ast::FnColor;
use crate::diag::ErrorCode;
use crate::hir::{AtomicOp, DeviceIntrinsic, MeshIntrinsic, PrimTy, TaskIntrinsic};
use crate::mir::{
    BasicBlock, Body, CallTarget, Const, LocalIdx, MeshEntryMeta, Operand, Place, ProjElem,
    ResourceMethod, Rvalue, StatementKind, TerminatorKind,
};
use crate::query::QueryCtx;
use crate::resolve::Resolutions;
use crate::span::Span;
use crate::ty::Ty;

// ───────────────────────── SPIR-V 常量(核心规范取值) ─────────────────────────

const SPIRV_MAGIC: u32 = 0x0723_0203;
const SPIRV_VERSION_1_0: u32 = 0x0001_0000;
/// SPIR-V 1.4(RFC-0013 §4.E6,Q-M-SpirvVersion:per-entry 版本轴)。mesh/RT 入口
/// **硬性要求** 1.4(`VK_KHR_ray_tracing_pipeline` 依赖 `VK_KHR_spirv_1_4`;1.4 起
/// `OpEntryPoint` interface 须枚举全部被引用全局变量)。既有 compute/vertex/fragment 入口
/// 维持 [`SPIRV_VERSION_1_0`] emit(产物字节零漂移;既有 vulkan golden 不重 bless、DXIL B 路
/// 消费的 SPIR-V 字节不变)——分叉在**发射函数级**:compute [`assemble`] 恒 1.0,mesh/task/RT
/// [`assemble_ext_module`] 恒 1.4。
const SPIRV_VERSION_1_4: u32 = 0x0001_0400;
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
const OP_TYPE_IMAGE: u16 = 25;
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
const OP_COMPOSITE_CONSTRUCT: u16 = 80;
const OP_IMAGE_READ: u16 = 98;
const OP_IMAGE_WRITE: u16 = 99;
const OP_CONVERT_F_TO_U: u16 = 109;
const OP_CONVERT_F_TO_S: u16 = 110;
const OP_CONVERT_S_TO_F: u16 = 111;
const OP_CONVERT_U_TO_F: u16 = 112;
const OP_UCONVERT: u16 = 113;
const OP_SCONVERT: u16 = 114;
const OP_BITCAST: u16 = 124;
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
const OP_SHIFT_RIGHT_LOGICAL: u16 = 194;
const OP_SHIFT_RIGHT_ARITHMETIC: u16 = 195;
const OP_SHIFT_LEFT_LOGICAL: u16 = 196;
const OP_BITWISE_OR: u16 = 197;
const OP_BITWISE_XOR: u16 = 198;
const OP_BITWISE_AND: u16 = 199;
const OP_CONTROL_BARRIER: u16 = 224;
const OP_ATOMIC_EXCHANGE: u16 = 229;
const OP_ATOMIC_COMPARE_EXCHANGE: u16 = 230;
const OP_ATOMIC_IADD: u16 = 234;
const OP_ATOMIC_ISUB: u16 = 235;
const OP_ATOMIC_SMIN: u16 = 236;
const OP_ATOMIC_UMIN: u16 = 237;
const OP_ATOMIC_SMAX: u16 = 238;
pub(crate) const OP_ATOMIC_UMAX: u16 = 239;
const OP_ATOMIC_AND: u16 = 240;
const OP_ATOMIC_OR: u16 = 241;
const OP_ATOMIC_XOR: u16 = 242;
const OP_SELECTION_MERGE: u16 = 247;
const OP_LABEL: u16 = 248;
const OP_BRANCH: u16 = 249;
const OP_BRANCH_CONDITIONAL: u16 = 250;
const OP_LOOP_MERGE: u16 = 246;
const OP_RETURN: u16 = 253;
const OP_UNREACHABLE: u16 = 255;

// ── SPV_KHR_ray_query 指令 / 类型(G7.2 W3a,RXS-0300)。
// 取值逐一核自本机 SPIR-V 头 `spirv-headers/spirv.core.grammar.json`
// (Vulkan SDK 1.3.296.0),非凭记忆;capability 承载均为 `RayQueryKHR`
// (`OpTypeAccelerationStructureKHR` 另有 RayTracingNV/RayTracingKHR 承载路径,
// compute 面唯一取 RayQueryKHR,RXS-0300 逐字)。
const OP_TYPE_RAY_QUERY_KHR: u16 = 4472;
const OP_RAY_QUERY_INITIALIZE_KHR: u16 = 4473;
const OP_RAY_QUERY_TERMINATE_KHR: u16 = 4474;
const OP_RAY_QUERY_PROCEED_KHR: u16 = 4477;
const OP_RAY_QUERY_GET_INTERSECTION_TYPE_KHR: u16 = 4479;
// `OP_TYPE_ACCELERATION_STRUCTURE_KHR`(=5341)已由 RT 腿(RXS-0247)在下方
// 常量块定义,此处复用同一常量(单一事实源,不重复定义)。
const OP_RAY_QUERY_GET_INTERSECTION_T_KHR: u16 = 6018;
const OP_RAY_QUERY_GET_INTERSECTION_INSTANCE_ID_KHR: u16 = 6020;
const OP_RAY_QUERY_GET_INTERSECTION_GEOMETRY_INDEX_KHR: u16 = 6022;
const OP_RAY_QUERY_GET_INTERSECTION_PRIMITIVE_INDEX_KHR: u16 = 6023;
const OP_RAY_QUERY_GET_INTERSECTION_BARYCENTRICS_KHR: u16 = 6024;

// 枚举取值。
const CAP_SHADER: u32 = 1;
// CAP_INT64/CAP_INT64_ATOMICS/OP_ATOMIC_UMAX/SCOPE_DEVICE/MEM_SEM_RELAXED 自 G7.5b 起
// `pub(crate)`:图形扩展路(dxil_spirv ExtendedBodyLowerer)的 u64 原子发射段与 compute
// 路**逐字同值**(RXS-0302 L2「scope/semantics 常量与 compute 路发射段逐字同值」;
// 仅改可见性不改 compute 发射序)。
pub(crate) const CAP_INT64: u32 = 11;
pub(crate) const CAP_INT64_ATOMICS: u32 = 12;
/// `RayQueryKHR` capability(=4472;与 `OpTypeRayQueryKHR` opcode 同值但属不同
/// 枚举空间)。compute inline ray query 与 compute 面 `OpTypeAccelerationStructureKHR`
/// 的唯一承载(RXS-0300)。
const CAP_RAY_QUERY_KHR: u32 = 4472;
/// `SPV_KHR_ray_query` extension 名。声明条件 = 模块含 `OpTypeRayQueryKHR` 或
/// `OpTypeAccelerationStructureKHR`(与升版并集判定同源,RXS-0300)。
const EXT_SPV_KHR_RAY_QUERY: &str = "SPV_KHR_ray_query";
// `OpRayQueryInitializeKHR` 的 RayFlags / CullMask 实参:首期恒 `OpaqueKHR`(=1)
// 与 `0xFF`(RXS-0298 钉死,与 RXS-0245 `trace_ray` 同一纪律)→ 复用下方 RT 腿
// 既有 `RAY_FLAG_OPAQUE` / `CULL_MASK_ALL` 常量(单一事实源)。二者为 `IdRef`,
// 须经 `const_uint` 物化为 `OpConstant`。
/// `RayQueryIntersection` 枚举:committed 侧(=1)。首期只读 committed
/// (candidate 面 RXS-0298 首期不开放)。
const RAY_QUERY_COMMITTED_INTERSECTION_KHR: u32 = 1;
/// `RayQueryCommittedIntersectionType` 枚举:`None`(=0)。`has_committed` =
/// `OpRayQueryGetIntersectionTypeKHR(committed) != None`(RXS-0300)。
const RAY_QUERY_COMMITTED_INTERSECTION_NONE_KHR: u32 = 0;
const ADDR_MODEL_LOGICAL: u32 = 0;
const MEM_MODEL_GLSL450: u32 = 1;
const EXEC_MODEL_GLCOMPUTE: u32 = 5;
const EXEC_MODE_LOCAL_SIZE: u32 = 17;
const FUNCTION_CONTROL_NONE: u32 = 0;
const SELECTION_CONTROL_NONE: u32 = 0;
/// `OpLoopMerge` 的 Loop Control 掩码:`None`(不请求 Unroll/DontUnroll 等)。
const LOOP_CONTROL_NONE: u32 = 0;

// 存储类。
const STORAGE_INPUT: u32 = 1;
const STORAGE_UNIFORM_CONSTANT: u32 = 0;
const STORAGE_UNIFORM: u32 = 2;
const STORAGE_FUNCTION: u32 = 7;
const STORAGE_PUSH_CONSTANT: u32 = 9;
/// `StorageBuffer` 存储类(=12)。SPIR-V 1.3 起为核心;1.4 的 SSBO 唯一合法形态
/// (`Block` + `StorageBuffer`),因 `BufferBlock` 装饰在 1.4 被移除(G7.2 W3a)。
const STORAGE_STORAGE_BUFFER: u32 = 12;

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

// storage image 类型。
const DIM_2D: u32 = 1;
const IMAGE_SAMPLED_STORAGE: u32 = 2;
const IMAGE_FORMAT_RGBA32F: u32 = 1;
const IMAGE_FORMAT_RGBA32I: u32 = 21;
const IMAGE_FORMAT_RGBA32UI: u32 = 30;

// barrier scope / memory semantics(OpControlBarrier)。
const SCOPE_WORKGROUP: u32 = 2;
pub(crate) const SCOPE_DEVICE: u32 = 1;
pub(crate) const MEM_SEM_RELAXED: u32 = 0;
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
    /// `TextureRw2D<F>` → format-qualified storage image(set 0,binding = 序)。
    Image { binding: u32, elem: PrimTy },
    /// `AccelStruct`(G7.2 W3a,RXS-0297 修订行)→ `OpTypeAccelerationStructureKHR`
    /// descriptor(SRV 轴,UniformConstant;set 0,binding = 序)。
    AccelStruct { binding: u32 },
    /// 标量形参 → push constant 块成员(member idx = 序)。
    Scalar { member: u32, prim: PrimTy },
    /// `ThreadCtx`(ZST)→ 不产物化。
    ThreadCtx,
}

/// SPIR-V 模块构造器(compute)。分节累积,末尾按 SPIR-V logical layout 组装。
struct Builder<'a> {
    res: &'a Resolutions,
    allow_int64: bool,
    uses_int64: bool,
    uses_int64_atomics: bool,
    /// 模块**实际发射**了 `OpTypeRayQueryKHR` / `OpTypeAccelerationStructureKHR`
    /// (由两个懒发点置位)。仅作 [`emit_1_4`](Self::emit_1_4) 的**一致性自检**:
    /// 发射了 ray query 类型却未升版 = codegen 内部不一致(assemble 处断言)。
    uses_ray_query: bool,
    /// 本入口是否 emit SPIR-V **1.4**(G7.2 W3a,RXS-0300 per-entry 升版判定)。
    ///
    /// **必须在发射任何指令之前定下**(而非懒发现):1.4 移除了 `BufferBlock`
    /// 装饰,SSBO 形态在 1.0 与 1.4 下不同(`BufferBlock`+`Uniform` vs
    /// `Block`+`StorageBuffer`),故描述符发射需要预先知道版本。判定只读 MIR
    /// (见 [`needs_spirv_1_4`]),同 MIR 同版本轴 → 结果确定性(RXS-0300 逐字)。
    emit_1_4: bool,
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
    type_ulong: Option<u32>,
    type_long: Option<u32>,
    type_float: Option<u32>,
    type_v3uint: Option<u32>,
    /// `OpTypeAccelerationStructureKHR`(懒发,G7.2 W3a,RXS-0300)。
    type_accel: Option<u32>,
    /// `OpTypeRayQueryKHR`(懒发,G7.2 W3a,RXS-0300)。
    type_ray_query: Option<u32>,
    vector_types: HashMap<(PrimTy, u32), u32>,
    ptr_cache: HashMap<(u32, u32), u32>, // (storage, pointee) → ptr type id
    const_u32: HashMap<u32, u32>,
    const_f32: HashMap<u32, u32>, // bits → id
    // builtin 变量(懒发)。
    builtin_vars: HashMap<u32, u32>, // builtin enum → var id
    // local idx → Function OpVariable id(标量/临时);buffer 形参不入此表。
    local_var: HashMap<u32, u32>,
    // buffer 形参 local idx → (描述符变量 id, 元素 PrimTy)。
    buffer_var: HashMap<u32, (u32, PrimTy)>,
    // storage image 形参 local idx → (变量 id,OpTypeImage id,分量类型)。
    image_var: HashMap<u32, (u32, u32, PrimTy)>,
    /// `AccelStruct` 形参 local idx → descriptor 变量 id(G7.2 W3a,RXS-0300)。
    accel_var: HashMap<u32, u32>,
    /// `RayQuery` local idx → Function-storage `OpVariable` id(G7.2 W3a;
    /// RXS-0297 Function-only 收窄)。
    ray_query_var: HashMap<u32, u32>,
    /// 循环头块下标 → (merge 块下标, latch 块下标)(G7.2 W3a;结构化循环)。
    loop_info: HashMap<usize, (usize, usize)>,
    /// latch 块下标 → **合成 continue 块** label id(G7.2 W3a)。
    ///
    /// 为何需要合成:MIR 把 `while c { if d { .. } }` 的内层 `if` 的 merge 块与
    /// 循环 latch **复用同一块**。SPIR-V 结构化规则下 continue 块属 *continue
    /// construct*、不属 *loop construct*,内层 selection 的 merge 块必须落在
    /// loop construct 内 → 复用即被 spirv-val 拒(`Header block ... is contained
    /// in the loop construct ... but its merge block is not`)。故在 latch 与
    /// 循环头之间**插入一个只含 `OpBranch <header>` 的合成块**作 continue target,
    /// 原 latch 块回归纯 selection merge 角色。
    loop_continue_label: HashMap<usize, u32>,
    /// SPIR-V 1.4 `OpEntryPoint` interface **全量枚举**所需的全部全局变量 id
    /// (RXS-0300;1.4 起 interface 须列全被引用全局变量,与 mesh/RT 同律
    /// RXS-0247)。1.0 路径不消费本表(`entry_interface` 只含 Input/Output),
    /// 既有 `assemble` 字节零漂移。
    global_vars: Vec<u32>,
    // block idx → label id。
    block_label: HashMap<usize, u32>,
    main_id: u32,
}

impl<'a> Builder<'a> {
    fn new(res: &'a Resolutions, allow_int64: bool) -> Self {
        Builder {
            res,
            allow_int64,
            uses_int64: false,
            uses_int64_atomics: false,
            uses_ray_query: false,
            emit_1_4: false,
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
            type_ulong: None,
            type_long: None,
            type_float: None,
            type_v3uint: None,
            type_accel: None,
            type_ray_query: None,
            vector_types: HashMap::new(),
            ptr_cache: HashMap::new(),
            const_u32: HashMap::new(),
            const_f32: HashMap::new(),
            builtin_vars: HashMap::new(),
            local_var: HashMap::new(),
            buffer_var: HashMap::new(),
            image_var: HashMap::new(),
            accel_var: HashMap::new(),
            ray_query_var: HashMap::new(),
            loop_info: HashMap::new(),
            loop_continue_label: HashMap::new(),
            global_vars: Vec::new(),
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

    fn t_ulong(&mut self) -> u32 {
        self.uses_int64 = true;
        if let Some(id) = self.type_ulong {
            return id;
        }
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_INT, &[id, 64, 0]);
        self.type_ulong = Some(id);
        id
    }

    fn t_long(&mut self) -> u32 {
        self.uses_int64 = true;
        if let Some(id) = self.type_long {
            return id;
        }
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_INT, &[id, 64, 1]);
        self.type_long = Some(id);
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

    /// `OpTypeAccelerationStructureKHR`(懒发;G7.2 W3a,RXS-0300)。发射即置
    /// `uses_ray_query` —— capability/extension/1.4 升版三者同源于此并集。
    fn t_accel_struct(&mut self) -> u32 {
        self.uses_ray_query = true;
        if let Some(id) = self.type_accel {
            return id;
        }
        let id = self.fresh();
        emit(
            &mut self.types_globals,
            OP_TYPE_ACCELERATION_STRUCTURE_KHR,
            &[id],
        );
        self.type_accel = Some(id);
        id
    }

    /// `OpTypeRayQueryKHR`(懒发;G7.2 W3a,RXS-0300)。同上置 `uses_ray_query`。
    fn t_ray_query(&mut self) -> u32 {
        self.uses_ray_query = true;
        if let Some(id) = self.type_ray_query {
            return id;
        }
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_RAY_QUERY_KHR, &[id]);
        self.type_ray_query = Some(id);
        id
    }

    /// 标量 PrimTy → SPIR-V 类型 id。compute 路径放行 i64/u64 并按需记录 Int64；
    /// mesh 路径仍拒绝 64 位整数，f64 在两条路径均为 RX6026。
    fn prim_type(&mut self, p: PrimTy, span: Span) -> Result<u32, VulkanCodegenError> {
        match p {
            PrimTy::F32 => Ok(self.t_float()),
            PrimTy::Usize | PrimTy::U32 | PrimTy::U16 | PrimTy::U8 => Ok(self.t_uint()),
            PrimTy::I32 | PrimTy::I16 | PrimTy::I8 => Ok(self.t_int()),
            PrimTy::U64 if self.allow_int64 => Ok(self.t_ulong()),
            PrimTy::I64 if self.allow_int64 => Ok(self.t_long()),
            // bool 在内存中以 u32(0/1)表示(镜像 NVPTX i8);SSA 比较结果为 OpTypeBool,
            // 经 OpSelect 转 u32 存回(见 emit_assign 比较分支 / SwitchBool)。
            PrimTy::Bool => Ok(self.t_uint()),
            other => Err(VulkanCodegenError::unsupported(
                span,
                format!(
                    "Vulkan 当前入口不支持类型 {other:?}(compute 仅新增 I64/U64；F64 仍需 Float64 capability)"
                ),
            )),
        }
    }

    fn vector_type(
        &mut self,
        prim: PrimTy,
        len: u32,
        span: Span,
    ) -> Result<u32, VulkanCodegenError> {
        if let Some(&id) = self.vector_types.get(&(prim, len)) {
            return Ok(id);
        }
        let elem = self.prim_type(prim, span)?;
        let id = self.fresh();
        emit(&mut self.types_globals, OP_TYPE_VECTOR, &[id, elem, len]);
        self.vector_types.insert((prim, len), id);
        Ok(id)
    }

    /// SSBO 存储类:1.0 路 = `Uniform`(配 `BufferBlock` 装饰,pre-1.3 SSBO 惯用形态,
    /// 既有 W1/W2 golden 字节依赖);1.4 路 = `StorageBuffer`(G7.2 W3a)。
    ///
    /// **为何必须分叉**:`BufferBlock` 装饰在 SPIR-V **1.4 起被移除**
    /// (spirv-val:`operand BufferBlock(3) requires SPIR-V version 1.3 or earlier`),
    /// 1.4 的 SSBO 唯一合法形态是 `Block` 装饰 + `StorageBuffer` 存储类
    /// (该存储类自 1.3 起为核心,无需 `SPV_KHR_storage_buffer_storage_class`)。
    /// 分叉只作用于升 1.4 的入口;1.0 入口逐字节不变(零漂移门)。
    fn ssbo_storage(&self) -> u32 {
        if self.emit_1_4 {
            STORAGE_STORAGE_BUFFER
        } else {
            STORAGE_UNIFORM
        }
    }

    /// SSBO 块装饰:1.0 路 = `BufferBlock`;1.4 路 = `Block`(同上)。
    fn ssbo_block_decoration(&self) -> u32 {
        if self.emit_1_4 {
            DECORATION_BLOCK
        } else {
            DECORATION_BUFFER_BLOCK
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

/// (G7.5b 起 `pub(crate)`:图形扩展路的回边预扫描〔负面清单「循环」轴〕与
/// `structured_merge` 复用同一后继表。)
pub(crate) fn block_succs(bb: &BasicBlock) -> Vec<usize> {
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
        // Vulkan 原生消费入口(RXS-0210 方案 B + G7.5b RXS-0301 两遍编译):去
        // UserSemantic/SPV_GOOGLE provenance(保名仅 B 路 HLSL 转译需要)→ `.spv` 免
        // device 扩展依赖直喂 vkCreateShaderModule(修 VUID-...-08742)。DXIL 路
        // emit_spirv_body 字节不变。第一遍 Unmappable → ExtendedBodyLowerer(RXS-0301
        // 白名单),仍失败 → RX6026(负面清单诊断)。
        return match crate::dxil_spirv::emit_spirv_body_vulkan(stage, entry, &res) {
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
    let mut b = Builder::new(res, true);
    // per-entry 版本轴判定**先于一切发射**(RXS-0300;见 `Builder::emit_1_4`)。
    b.emit_1_4 = needs_spirv_1_4(body, res);
    b.main_id = b.fresh();

    // 形参分类(locals 1..=arg_count):AccelStruct / buffer / scalar / ThreadCtx。
    let mut params: Vec<(LocalIdx, ParamKind)> = Vec::new();
    let mut next_binding = 0u32;
    let mut next_member = 0u32;
    // G7.2 W3a(RXS-0297「compute 签名 AccelStruct 至多一个」单 TLAS 纪律):
    // shader_stages 已在 AST 层预校验(第 2 个起 RX3013);此处为 codegen 侧
    // **防御性**复核 —— 前端若被绕过,不静默产多 AS 模块。
    if body.accel_params.len() > 1 {
        return Err(VulkanCodegenError::unsupported(
            body.span,
            "compute 签名中 `AccelStruct` 至多一个(单 TLAS 纪律,RXS-0297)",
        ));
    }
    for i in 1..=body.arg_count {
        let li = LocalIdx(i as u32);
        let ty = &body.locals[i].ty;
        let span = body.locals[i].span;
        let is_accel = body.accel_params.contains(&(i as u32));
        let kind = classify_param(
            &mut b,
            ty,
            span,
            is_accel,
            &mut next_binding,
            &mut next_member,
        )?;
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
    // 结构化循环分析(G7.2 W3a):循环头 → (merge, latch),并为每个 latch 预分配
    // 合成 continue 块 label(见 `Builder::loop_continue_label` 逐字留痕)。
    for bi in 0..body.blocks.len() {
        if let Some((merge_i, latch_i)) = loop_merge_targets(body, bi) {
            b.loop_info.insert(bi, (merge_i, latch_i));
            let lbl = b.fresh();
            b.loop_continue_label.insert(latch_i, lbl);
        }
    }

    // Function-storage local 变量(非 ZST、非 buffer 形参、非 ret slot〔kernel void〕)。
    // scalar 形参也建 Function local(entry 处从 push-constant 拷入),body 统一按 local 处理。
    for (i, l) in body.locals.iter().enumerate() {
        if i == 0 {
            continue; // ret slot(kernel = void)
        }
        if b.buffer_var.contains_key(&(i as u32))
            || b.image_var.contains_key(&(i as u32))
            || b.accel_var.contains_key(&(i as u32))
        {
            continue; // 资源形参 → 描述符,不建 Function local
        }
        if is_zst(res, &l.ty) {
            continue;
        }
        // G7.2 W3a(RXS-0297 Function-only 收窄):`RayQuery` local 不在此建
        // Function 变量,也**不**入 `local_var`(非值:`OpLoad`/`OpStore` 对
        // RayQuery 类型指针在 SPV_KHR_ray_query 规范上禁用,非 Copy 纪律
        // by-construction)。变量改由 `ray_query_var_for` 在 `initialize` 落点
        // **按需建**;MIR 的 `let mut rq = ray_query_initialize(..)` 会产生
        // 「temp → 用户 local」的 move,该 move 降级为**别名**(同一遍历器对象,
        // 零指令),故按需建可避免产生死变量。
        if matches!(&l.ty, Ty::Adt(d, _) if res.lang_items.is_ray_query(*d)) {
            continue;
        }
        let kind = value_kind(&l.ty).ok_or_else(|| {
            VulkanCodegenError::unsupported(
                l.span,
                "Vulkan compute local 仅支持标量与 2/4 分量同型元组向量",
            )
        })?;
        let ty_id = value_type(&mut b, kind, l.span)?;
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

    // per-entry 版本轴分叉(RXS-0300,升版判定**并集**钉死):使用 RayQuery
    // (MIR 体存在 RayQuery local / ray query intrinsic)**或** compute 签名含
    // `AccelStruct` 形参 → 升 SPIR-V 1.4 + interface 全量枚举。二者任一即触发:
    // 仅看 RayQuery local 会把「AS 形参在、RayQuery 不在」的 kernel 留在 1.0 且
    // 不声明 capability,致 `OpTypeAccelerationStructureKHR` 无 capability 承载、
    // spirv-val 必拒。`uses_ray_query` 由 `t_accel_struct`/`t_ray_query` 两个
    // 懒发点共同置位,已是该并集。
    //
    // W1/W2 零漂移:不含二者的 compute entry 走既有 `assemble`(1.0)**原路**,
    // 既有五 kernel 与全部既有 vulkan golden 字节不变(分叉落发射函数级)。
    // 一致性自检:实际发射了 ray query 类型 ⇒ 必已判定升 1.4。二者不一致即
    // codegen 内部 bug(会产 capability 无承载 / BufferBlock@1.4 等必拒模块),
    // 宁可确定性报错也不产可疑模块。
    if b.uses_ray_query && !b.emit_1_4 {
        return Err(VulkanCodegenError::unsupported(
            body.span,
            "内部不一致:发射了 ray query 类型但未判定升 SPIR-V 1.4(RXS-0300 并集判定)",
        ));
    }
    if b.emit_1_4 {
        Ok(assemble_ray_query(&mut b, &body.symbol))
    } else {
        Ok(assemble(&mut b, &body.symbol))
    }
}

/// per-entry SPIR-V 1.4 升版判定(G7.2 W3a,RXS-0300 **并集**钉死)。
///
/// 触发条件(任一即升):
/// 1. compute 签名含 `AccelStruct` 形参([`Body::accel_params`] 非空);
/// 2. MIR 体存在 `RayQuery` 类型 local;
/// 3. MIR 体存在 ray query intrinsic(`RayQueryInitialize` / `RayQueryMethod`)。
///
/// 二者任一即触发的理由(RXS-0300 逐字):仅看 RayQuery local 会把「AS 形参在、
/// RayQuery 不在」的 kernel 留在 1.0 且不声明 capability,致
/// `OpTypeAccelerationStructureKHR` 无 capability 承载、spirv-val 必拒。
///
/// 只读 MIR,无副作用 → 同 MIR 同版本轴结果确定性。W1/W2 五 kernel 无 AS 形参、
/// 无 RayQuery 面 → 恒 `false`,零漂移。
fn needs_spirv_1_4(body: &Body, res: &Resolutions) -> bool {
    if !body.accel_params.is_empty() {
        return true;
    }
    if body
        .locals
        .iter()
        .any(|l| matches!(&l.ty, Ty::Adt(d, _) if res.lang_items.is_ray_query(*d)))
    {
        return true;
    }
    body.blocks.iter().any(|bb| {
        bb.stmts.iter().any(|st| {
            let StatementKind::Assign(_, rv) = &st.kind;
            matches!(
                rv,
                Rvalue::RayQueryInitialize { .. } | Rvalue::RayQueryMethod { .. }
            )
        })
    })
}

/// mesh 阶段 MIR → SPIR-V lowering(G4.2,RXS-0275)。
///
/// 镜像 [`lower_compute`] 的 MIR 降级流程(形参分类 → 描述符/push-constant →
/// block 降级),但组装为 **MeshEXT** 执行模型 + SPIR-V 1.4 header(per-entry
/// 分叉,RXS-0275),并自 `body.mesh_meta` 发射 `LocalSize`/`OutputVertices`/
/// `OutputPrimitivesEXT`/`OutputTrianglesEXT` execution modes。
/// `mesh_set_outputs` intrinsic → `OpSetMeshOutputsEXT`(经 [`emit_call`] 分发)。
/// task 条件臂首期不开放(RXS-0270),task 阶段根不收集,本函数不被 task 入口调用。
pub fn lower_mesh(body: &Body, res: &Resolutions) -> Result<Vec<u32>, VulkanCodegenError> {
    let mesh_meta = body.mesh_meta.as_ref().ok_or_else(|| {
        VulkanCodegenError::unsupported(
            body.span,
            "mesh 入口缺 mesh_meta(RXS-0275;attach_graphics_io_sig 未填充)",
        )
    })?;

    let mut b = Builder::new(res, false);
    b.main_id = b.fresh();

    // 形参分类(locals 1..=arg_count):buffer / scalar / ThreadCtx。
    let mut params: Vec<(LocalIdx, ParamKind)> = Vec::new();
    let mut next_binding = 0u32;
    let mut next_member = 0u32;
    for i in 1..=body.arg_count {
        let li = LocalIdx(i as u32);
        let ty = &body.locals[i].ty;
        let span = body.locals[i].span;
        // mesh 阶段无 compute 签名 AccelStruct 面(`accel_params` 恒空,
        // `attach_accel_params` 只对 compute 根携带)→ `is_accel = false`。
        let kind = classify_param(&mut b, ty, span, false, &mut next_binding, &mut next_member)?;
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

    // Function-storage local 变量(非 ZST、非 buffer 形参、非 ret slot)。
    for (i, l) in body.locals.iter().enumerate() {
        if i == 0 {
            continue; // ret slot(mesh = void)
        }
        if b.buffer_var.contains_key(&(i as u32)) || b.image_var.contains_key(&(i as u32)) {
            continue;
        }
        if is_zst(res, &l.ty) {
            continue;
        }
        let kind = value_kind(&l.ty).ok_or_else(|| {
            VulkanCodegenError::unsupported(
                l.span,
                "Vulkan mesh local 首期仅支持标量类型(非标量 local 属后续分片)",
            )
        })?;
        let ty_id = value_type(&mut b, kind, l.span)?;
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
    let bb0 = b.block_label[&0];
    emit(&mut b.func_body, OP_BRANCH, &[bb0]);

    // 各 block 降级(复用 compute 的 emit_assign / emit_terminator;
    // MeshIntrinsic 经 emit_call 分发 → OpSetMeshOutputsEXT)。
    for (bi, bb) in body.blocks.iter().enumerate() {
        let label = b.block_label[&bi];
        emit(&mut b.func_body, OP_LABEL, &[label]);
        for st in &bb.stmts {
            let StatementKind::Assign(place, rv) = &st.kind;
            emit_assign(&mut b, body, place, rv)?;
        }
        emit_terminator(&mut b, body, bi)?;
    }

    Ok(assemble_mesh(&mut b, &body.symbol, mesh_meta))
}

/// task 阶段 MIR → SPIR-V lowering(G4.2,RXS-0275)。
///
/// task 条件臂**首期不开放**(RXS-0270):task 阶段根不收集(`collectable_stage`
/// 排除 Task),本函数不被调用。类型面预留(`TaskIntrinsic` 枚举 + `emit_call`
/// 分支),待 Q-RTArm/Q-MeshScope 评估窗评估后兑现。
pub fn lower_task(_body: &Body, _res: &Resolutions) -> Result<Vec<u32>, VulkanCodegenError> {
    Err(VulkanCodegenError::unsupported(
        _body.span,
        "task 阶段条件臂首期不开放(RXS-0270);lower_task 待评估窗兑现",
    ))
}

/// 形参分类 + buffer binding / scalar member 计数递增。
///
/// `is_accel` = 本形参是否为 `AccelStruct`(G7.2 W3a,RXS-0297/0300)。判定**不**在
/// 本函数内以 `ty` 反推——`AccelStruct` 形参 ty 落容忍位 `Ty::Err`,反推会把拼错的
/// 未知类型名误绑成 AS descriptor;事实源 = [`crate::mir::Body::accel_params`]
/// (单一事实源 = AST 层 `shader_stages::is_accel_struct`)。
fn classify_param(
    b: &mut Builder,
    ty: &Ty,
    span: Span,
    is_accel: bool,
    next_binding: &mut u32,
    next_member: &mut u32,
) -> Result<ParamKind, VulkanCodegenError> {
    if is_accel {
        let binding = *next_binding;
        *next_binding += 1;
        return Ok(ParamKind::AccelStruct { binding });
    }
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
    if let Ty::Adt(d, args) = ty
        && let Some(is_view) = b.res.lang_items.atomic_kind(*d)
    {
        let elem_idx = usize::from(is_view);
        let elem = args.get(elem_idx).and_then(prim_of).ok_or_else(|| {
            VulkanCodegenError::unsupported(
                span,
                "Vulkan 原子形参仅支持 Atomic/AtomicView 的 i32/u32/i64/u64 元素",
            )
        })?;
        if !(matches!(elem, PrimTy::I32 | PrimTy::U32)
            || b.allow_int64 && matches!(elem, PrimTy::I64 | PrimTy::U64))
        {
            return Err(VulkanCodegenError::unsupported(
                span,
                "Vulkan compute 原子仅支持 i32/u32/i64/u64；其他入口维持 i32/u32",
            ));
        }
        let binding = *next_binding;
        *next_binding += 1;
        return Ok(ParamKind::Buffer { binding, elem });
    }
    if let Ty::Adt(d, args) = ty
        && b.res.lang_items.is_texture_rw2d(*d)
    {
        let elem = args.first().and_then(prim_of).unwrap_or(PrimTy::F32);
        if !matches!(elem, PrimTy::F32 | PrimTy::I32 | PrimTy::U32) {
            return Err(VulkanCodegenError::unsupported(
                span,
                "TextureRw2D storage image 仅支持 f32/i32/u32 分量",
            ));
        }
        let binding = *next_binding;
        *next_binding += 1;
        return Ok(ParamKind::Image { binding, elem });
    }
    if let Some(p) = prim_of(ty) {
        let member = *next_member;
        *next_member += 1;
        return Ok(ParamKind::Scalar { member, prim: p });
    }
    Err(VulkanCodegenError::unsupported(
        span,
        "Vulkan compute 形参仅支持 View/ViewMut/Atomic/AtomicView、TextureRw2D、标量、ThreadCtx",
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
            let stride = prim_layout(*elem).1;
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
            let block_deco = b.ssbo_block_decoration();
            emit(&mut b.decorations, OP_DECORATE, &[st, block_deco]);
            // 变量(1.0:Uniform+BufferBlock / 1.4:StorageBuffer+Block;set 0 / binding)。
            let storage = b.ssbo_storage();
            let ptr = b.ptr_type(storage, st);
            let var = b.fresh();
            emit(&mut b.types_globals, OP_VARIABLE, &[ptr, var, storage]);
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
            b.global_vars.push(var);
        } else if let ParamKind::Image { binding, elem } = kind {
            let sampled_ty = b.prim_type(*elem, body.locals[li.0 as usize].span)?;
            let format = match elem {
                PrimTy::F32 => IMAGE_FORMAT_RGBA32F,
                PrimTy::I32 => IMAGE_FORMAT_RGBA32I,
                PrimTy::U32 => IMAGE_FORMAT_RGBA32UI,
                _ => unreachable!("classify_param 已限制 storage image 分量"),
            };
            let image_ty = b.fresh();
            emit(
                &mut b.types_globals,
                OP_TYPE_IMAGE,
                &[
                    image_ty,
                    sampled_ty,
                    DIM_2D,
                    0,
                    0,
                    0,
                    IMAGE_SAMPLED_STORAGE,
                    format,
                ],
            );
            let ptr = b.ptr_type(STORAGE_UNIFORM_CONSTANT, image_ty);
            let var = b.fresh();
            emit(
                &mut b.types_globals,
                OP_VARIABLE,
                &[ptr, var, STORAGE_UNIFORM_CONSTANT],
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
            b.image_var.insert(li.0, (var, image_ty, *elem));
            b.global_vars.push(var);
        } else if let ParamKind::AccelStruct { binding } = kind {
            // G7.2 W3a(RXS-0297 修订行 + RXS-0300):`AccelStruct` → SRV 轴
            // `OpTypeAccelerationStructureKHR` descriptor,存储类 UniformConstant
            // (与 `Texture2D`/storage image 同轴先例);set 0 / binding = 形参
            // 出现序(与 RXS-0208 marshalling 单一事实源同源)。
            let accel_ty = b.t_accel_struct();
            let ptr = b.ptr_type(STORAGE_UNIFORM_CONSTANT, accel_ty);
            let var = b.fresh();
            emit(
                &mut b.types_globals,
                OP_VARIABLE,
                &[ptr, var, STORAGE_UNIFORM_CONSTANT],
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
            b.accel_var.insert(li.0, var);
            b.global_vars.push(var);
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
    // 成员按自然标量对齐顺排；i64/u64 为 8 字节对齐。
    let mut offset = 0u32;
    for (i, (_, prim)) in scalars.iter().enumerate() {
        let (align, size) = prim_layout(*prim);
        offset = align_up(offset, align);
        emit(
            &mut b.decorations,
            OP_MEMBER_DECORATE,
            &[st, i as u32, DECORATION_OFFSET, offset],
        );
        offset += size;
    }
    emit(&mut b.decorations, OP_DECORATE, &[st, DECORATION_BLOCK]);
    let ptr = b.ptr_type(STORAGE_PUSH_CONSTANT, st);
    let var = b.fresh();
    emit(
        &mut b.types_globals,
        OP_VARIABLE,
        &[ptr, var, STORAGE_PUSH_CONSTANT],
    );
    b.global_vars.push(var);
    Ok(Some(var))
}

// ───────────────────────── 语句 / place / operand ─────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ValueKind {
    Scalar(PrimTy),
    Vector(PrimTy, u32),
}

fn value_type(b: &mut Builder, kind: ValueKind, span: Span) -> Result<u32, VulkanCodegenError> {
    match kind {
        ValueKind::Scalar(prim) => b.prim_type(prim, span),
        ValueKind::Vector(prim, len) => b.vector_type(prim, len, span),
    }
}

/// place 解析 → (指针 id,SPIR-V 类型 id,值种类)。
/// - buffer 形参 + `[Index(idx)]` → `OpAccessChain(var, uint_0, idx)`(StorageBuffer 元素);
/// - Function local(无投影)→ 其 OpVariable id;
/// - **向量 Function local + `[Field(k)]`** → `OpAccessChain(var, const k)` 标量分量指针
///   (G7.4 W3c 路 A 实现兑现,见下);
/// - 其余 → RX6026。
///
/// # 向量分量投影(G7.4 W3c;W3a 章 B 实现面回填,spec 面 0-byte)
///
/// RXS-0298 已冻结 `committed_barycentric(&RayQuery) -> vec2<f32>`、RXS-0300 已发射
/// `OpRayQueryGetIntersectionBarycentricsKHR`,但先前 place 子集无字段投影 →
/// 该 vec2 的分量在语言面**不可消费**(值只能绑定不能读),已冻结条款成死条款。
/// 本支为其**实现兑现**:`v.k` → 对 Function 存储类向量变量的 `OpAccessChain`
/// 标量指针,读(`OpLoad`)与写(`OpStore`)经同一指针自然覆盖。
///
/// 定性 = **纯实现侧回填,不改既有条款语义**——先例口径见 spec/softraster.md §4
/// 「聚合值类型 device codegen(后续扩展,**非禁区**)……其接通**不改本文件既有
/// 条款语义**(纯实现侧回填)」与 spec/stdlib.md §5 同句,及 RX6026 原拒绝消息
/// 「属后续**分片**」字面(登记为后续分片,非禁止)。零新 RXS。
///
/// **作用面钉死**(超出即维持既有拒绝,不扩张):仅**单层** `[Field(k)]` 且 base 为
/// `ValueKind::Vector` 的 Function local。不做数组投影、`Deref`、嵌套投影
/// (`.0.1`)、buffer 元素字段投影;destructuring `let` 维持 RX6001 现状。
/// 分量下标越界(如 `vec2` 取 `.2`)在此确定性拒(typeck 层通常先拦,本处为
/// codegen 侧防御性复核,误判方向恒为拒)。
fn place_ptr(
    b: &mut Builder,
    body: &Body,
    p: &Place,
) -> Result<(u32, u32, ValueKind), VulkanCodegenError> {
    let span = body.locals[p.local.0 as usize].span;
    if p.proj.is_empty() {
        // Function local(标量/临时/scalar 形参 copy)。
        let kind = value_kind(&body.locals[p.local.0 as usize].ty).ok_or_else(|| {
            VulkanCodegenError::unsupported(span, "local 类型不是标量或受支持向量")
        })?;
        let ty_id = value_type(b, kind, span)?;
        let var = *b.local_var.get(&p.local.0).ok_or_else(|| {
            VulkanCodegenError::unsupported(
                span,
                "对未建 Function 变量的 local 访问(可能是 buffer 形参裸引用,子集外)",
            )
        })?;
        return Ok((var, ty_id, kind));
    }
    if let [ProjElem::Index(idx_local)] = p.proj.as_slice()
        && let Some((var, elem)) = b.buffer_var.get(&p.local.0).copied()
    {
        let elem_ty = b.prim_type(elem, span)?;
        let ptr_elem = b.ptr_type(b.ssbo_storage(), elem_ty);
        let idx_val = load_local(b, body, *idx_local)?;
        let member0 = b.const_uint(0);
        let acc = b.fresh();
        emit(
            &mut b.func_body,
            OP_ACCESS_CHAIN,
            &[ptr_elem, acc, var, member0, idx_val],
        );
        return Ok((acc, elem_ty, ValueKind::Scalar(elem)));
    }
    // 向量分量投影(单层 Field;见函数文档「向量分量投影」节)。
    if let [ProjElem::Field(k)] = p.proj.as_slice()
        && let Some(ValueKind::Vector(prim, len)) = value_kind(&body.locals[p.local.0 as usize].ty)
    {
        if *k >= len {
            return Err(VulkanCodegenError::unsupported(
                span,
                "向量分量下标越界(分量数外的字段投影)",
            ));
        }
        let var = *b.local_var.get(&p.local.0).ok_or_else(|| {
            VulkanCodegenError::unsupported(
                span,
                "对未建 Function 变量的向量 local 分量访问(子集外)",
            )
        })?;
        let comp_ty = b.prim_type(prim, span)?;
        let ptr_comp = b.ptr_type(STORAGE_FUNCTION, comp_ty);
        // 向量内下标须为常量(SPIR-V 逻辑寻址;本支恒常量,by-construction 满足)。
        let kidx = b.const_uint(*k);
        let acc = b.fresh();
        emit(
            &mut b.func_body,
            OP_ACCESS_CHAIN,
            &[ptr_comp, acc, var, kidx],
        );
        return Ok((acc, comp_ty, ValueKind::Scalar(prim)));
    }
    Err(VulkanCodegenError::unsupported(
        span,
        "Vulkan compute place 首期仅支持标量 local、向量 local 单层分量投影与 buffer[index](数组/deref/嵌套投影/buffer 元素字段投影属后续分片)",
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

/// operand → (值 id,SPIR-V 类型 id,值种类);unit/ZST → None。
fn operand(
    b: &mut Builder,
    body: &Body,
    o: &Operand,
) -> Result<Option<(u32, u32, ValueKind)>, VulkanCodegenError> {
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
                let bits = *v as u64;
                let val = bits as u32;
                // 无符号走 u32 常量缓存;i32 单独发(位型同但结果类型不同,不复用缓存)。
                let id = if is_64bit_prim(*p) {
                    let idn = b.fresh();
                    emit(
                        &mut b.types_globals,
                        OP_CONSTANT,
                        &[ty_id, idn, val, (bits >> 32) as u32],
                    );
                    idn
                } else if is_signed_prim(*p) {
                    let idn = b.fresh();
                    emit(&mut b.types_globals, OP_CONSTANT, &[ty_id, idn, val]);
                    idn
                } else {
                    b.const_uint(val)
                };
                Ok(Some((id, ty_id, ValueKind::Scalar(*p))))
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
                Ok(Some((id, ty_id, ValueKind::Scalar(PrimTy::F32))))
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
            // G7.2 W3a:`RayQuery` local 间的 move(`let mut rq =
            // ray_query_initialize(..)` 的 MIR temp → 用户 local)降级为**别名**
            // —— 同一遍历器对象,零指令。RayQuery 非 Copy(RXS-0297)故 move 是
            // 唯一形态,别名语义与之一致;规范禁用 `OpLoad`/`OpStore`/`OpCopyMemory`
            // 于 RayQuery 类型指针,别名亦是唯一可编码形态。
            if let Operand::Copy(src) | Operand::Move(src) = o
                && src.proj.is_empty()
                && place.proj.is_empty()
                && let Some(&src_var) = b.ray_query_var.get(&src.local.0)
            {
                b.ray_query_var.insert(place.local.0, src_var);
                return Ok(());
            }
            let Some((val, _, _)) = operand(b, body, o)? else {
                return Ok(()); // unit 赋值 no-op(空体语义)。
            };
            let (ptr, _, _) = place_ptr(b, body, place)?;
            emit(&mut b.func_body, OP_STORE, &[ptr, val]);
            Ok(())
        }
        Rvalue::BinaryOp(op, a, c) => {
            let Some((va, ty_id, kind)) = operand(b, body, a)? else {
                return Ok(());
            };
            let Some((vc, _, _)) = operand(b, body, c)? else {
                return Ok(());
            };
            let ValueKind::Scalar(prim) = kind else {
                return Err(VulkanCodegenError::unsupported(
                    body.span,
                    "Vulkan compute 向量算术不在 W1 作用面",
                ));
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
        Rvalue::Cast(o, target) => {
            let Some((val, src_ty_id, src_kind)) = operand(b, body, o)? else {
                return Ok(());
            };
            let ValueKind::Scalar(src_prim) = src_kind else {
                return Err(VulkanCodegenError::unsupported(
                    body.span,
                    "Vulkan compute 向量 Cast 不在 W1 作用面",
                ));
            };
            let dst_prim = prim_of(target).ok_or_else(|| {
                VulkanCodegenError::unsupported(body.span, "Cast 目标非标量(子集外)")
            })?;
            let dst_ty_id = b.prim_type(dst_prim, body.span)?;
            let result = if src_ty_id == dst_ty_id {
                // 同 SPIR-V 类型(如 Usize→U32 均映射 t_uint;F32→F32)→ identity,零转换指令。
                val
            } else {
                let opcode = cast_opcode(src_prim, dst_prim)?;
                let res = b.fresh();
                emit(&mut b.func_body, opcode, &[dst_ty_id, res, val]);
                res
            };
            let (ptr, _, _) = place_ptr(b, body, place)?;
            emit(&mut b.func_body, OP_STORE, &[ptr, result]);
            Ok(())
        }
        Rvalue::Aggregate(ty, ops) => {
            let kind = value_kind(ty).ok_or_else(|| {
                VulkanCodegenError::unsupported(
                    body.span,
                    "Vulkan compute aggregate 仅支持 2/4 分量同型元组向量",
                )
            })?;
            let ty_id = value_type(b, kind, body.span)?;
            let mut operands = vec![ty_id, b.fresh()];
            for op in ops {
                let Some((value, _, ValueKind::Scalar(_))) = operand(b, body, op)? else {
                    return Err(VulkanCodegenError::unsupported(
                        body.span,
                        "向量构造分量必须是标量",
                    ));
                };
                operands.push(value);
            }
            emit(&mut b.func_body, OP_COMPOSITE_CONSTRUCT, &operands);
            let (ptr, _, _) = place_ptr(b, body, place)?;
            emit(&mut b.func_body, OP_STORE, &[ptr, operands[1]]);
            Ok(())
        }
        Rvalue::Atomic {
            op,
            target_local,
            index,
            value,
            compare,
            // compute 路忽略 scope(恒 Device scope + Relaxed 映射,W1 既有口径;
            // 图形扩展路的 RXS-0302 L2 scope 判定见 dxil_spirv ExtendedBodyLowerer)。
            scope: _,
        } => emit_atomic(
            b,
            body,
            place,
            *op,
            *target_local,
            index.as_ref(),
            value,
            compare.as_ref(),
        ),
        Rvalue::ResourceSample {
            texture_local,
            method,
            coord,
            extra,
            ..
        } => emit_storage_image_op(b, body, place, *texture_local, *method, coord, extra),
        Rvalue::RayQueryInitialize {
            tlas_local,
            origin,
            t_min,
            dir,
            t_max,
        } => emit_ray_query_initialize(b, body, place, *tlas_local, origin, t_min, dir, t_max),
        Rvalue::RayQueryMethod { op, rq_local } => {
            emit_ray_query_method(b, body, place, *op, *rq_local)
        }
        _ => Err(VulkanCodegenError::unsupported(
            body.span,
            "Vulkan compute rvalue 不在当前 W1 子集",
        )),
    }
}

/// `Rvalue::RayQueryInitialize` → `OpRayQueryInitializeKHR`(G7.2 W3a,RXS-0300)。
///
/// 操作数序**逐一对齐** SPIR-V 规范(`spirv.core.grammar.json` 核实):
/// `RayQuery, Accel, RayFlags, CullMask, RayOrigin, RayTMin, RayDirection, RayTMax`
/// —— 注意 `RayTMin` 在 `RayOrigin` **之后**、`RayDirection` 在 `RayTMin` **之后**
/// (与语言面 `ray_query_initialize(tlas, origin, t_min, dir, t_max)` 的实参序同形,
/// 非字母序)。flags 恒 `OpaqueKHR`、mask 恒 `0xFF`(RXS-0298 钉死),二者为
/// `IdRef` 故须物化为 `OpConstant`。
///
/// `place` = 目标 RayQuery local。`OpRayQueryInitializeKHR` 无结果 id:遍历器
/// 状态直接写入该 Function 变量所指对象(不经 `OpStore`,非 Copy 纪律
/// by-construction)。
#[allow(clippy::too_many_arguments)]
fn emit_ray_query_initialize(
    b: &mut Builder,
    body: &Body,
    place: &Place,
    tlas_local: LocalIdx,
    origin: &Operand,
    t_min: &Operand,
    dir: &Operand,
    t_max: &Operand,
) -> Result<(), VulkanCodegenError> {
    let span = body.locals[tlas_local.0 as usize].span;
    let accel_var = *b.accel_var.get(&tlas_local.0).ok_or_else(|| {
        VulkanCodegenError::unsupported(
            span,
            "ray query tlas 未绑定 AccelStruct descriptor(须为 compute 签名 \
             `AccelStruct` 形参,RXS-0297)",
        )
    })?;
    let rq_var = ray_query_var_for(b, body, place)?;
    let accel_ty = b.t_accel_struct();
    let accel = b.fresh();
    emit(&mut b.func_body, OP_LOAD, &[accel_ty, accel, accel_var]);

    let origin_id = vec3_f32_operand(b, body, origin, span, "ray origin")?;
    let dir_id = vec3_f32_operand(b, body, dir, span, "ray direction")?;
    let t_min_id = f32_operand(b, body, t_min, span, "ray t_min")?;
    let t_max_id = f32_operand(b, body, t_max, span, "ray t_max")?;
    let flags = b.const_uint(RAY_FLAG_OPAQUE);
    let mask = b.const_uint(CULL_MASK_ALL);
    emit(
        &mut b.func_body,
        OP_RAY_QUERY_INITIALIZE_KHR,
        &[
            rq_var, accel, flags, mask, origin_id, t_min_id, dir_id, t_max_id,
        ],
    );
    Ok(())
}

/// `Rvalue::RayQueryMethod` → proceed / terminate / has_committed / committed 五查询
/// (G7.2 W3a,RXS-0300)。committed 查询族的 `Intersection` 实参恒
/// `RayQueryCommittedIntersectionKHR`(candidate 面 RXS-0298 首期不开放)。
fn emit_ray_query_method(
    b: &mut Builder,
    body: &Body,
    place: &Place,
    op: crate::hir::RayQueryOp,
    rq_local: LocalIdx,
) -> Result<(), VulkanCodegenError> {
    use crate::hir::RayQueryOp as Rq;
    let span = body.locals[rq_local.0 as usize].span;
    let rq_var = *b.ray_query_var.get(&rq_local.0).ok_or_else(|| {
        VulkanCodegenError::unsupported(span, "ray query 接收者未绑定 RayQuery Function 变量")
    })?;
    match op {
        // 无结果值:遍历器早退。`place` 为 unit local(MIR 侧 `Ty::unit()`),
        // 不建 Function 变量、不写回。
        Rq::Terminate => {
            emit(&mut b.func_body, OP_RAY_QUERY_TERMINATE_KHR, &[rq_var]);
            Ok(())
        }
        // bool 结果:SPIR-V 侧为 `OpTypeBool`,内存式 local 以 u32(0/1)建模
        // (镜像既有比较分支 `OpSelect` 口径)。
        Rq::Proceed | Rq::HasCommitted => {
            let bool_ty = b.t_bool();
            let cond = b.fresh();
            if matches!(op, Rq::Proceed) {
                emit(
                    &mut b.func_body,
                    OP_RAY_QUERY_PROCEED_KHR,
                    &[bool_ty, cond, rq_var],
                );
            } else {
                // `has_committed` = committed intersection type ≠ None
                // (RXS-0300:映射 `OpRayQueryGetIntersectionTypeKHR`)。
                let uint_ty = b.t_uint();
                let committed = b.const_uint(RAY_QUERY_COMMITTED_INTERSECTION_KHR);
                let ity = b.fresh();
                emit(
                    &mut b.func_body,
                    OP_RAY_QUERY_GET_INTERSECTION_TYPE_KHR,
                    &[uint_ty, ity, rq_var, committed],
                );
                let none = b.const_uint(RAY_QUERY_COMMITTED_INTERSECTION_NONE_KHR);
                emit(&mut b.func_body, OP_INOTEQUAL, &[bool_ty, cond, ity, none]);
            }
            let (ptr, dst_ty, _) = place_ptr(b, body, place)?;
            let one = b.const_uint(1);
            let zero = b.const_uint(0);
            let sel = b.fresh();
            emit(&mut b.func_body, OP_SELECT, &[dst_ty, sel, cond, one, zero]);
            emit(&mut b.func_body, OP_STORE, &[ptr, sel]);
            Ok(())
        }
        // committed 查询族:结果类型逐 op 固定(RXS-0298 签名),取 committed 交点。
        Rq::CommittedT
        | Rq::CommittedBarycentric
        | Rq::CommittedInstanceIndex
        | Rq::CommittedPrimitiveIndex
        | Rq::CommittedGeometryIndex => {
            let (opcode, result_ty) = match op {
                Rq::CommittedT => (OP_RAY_QUERY_GET_INTERSECTION_T_KHR, b.t_float()),
                Rq::CommittedBarycentric => (
                    OP_RAY_QUERY_GET_INTERSECTION_BARYCENTRICS_KHR,
                    b.vector_type(PrimTy::F32, 2, span)?,
                ),
                // 「instance index」= TLAS 内实例下标 → `InstanceId`
                // (RXS-0300 逐字列 `InstanceIdKHR`;非 `InstanceCustomIndex`)。
                Rq::CommittedInstanceIndex => {
                    (OP_RAY_QUERY_GET_INTERSECTION_INSTANCE_ID_KHR, b.t_uint())
                }
                Rq::CommittedPrimitiveIndex => (
                    OP_RAY_QUERY_GET_INTERSECTION_PRIMITIVE_INDEX_KHR,
                    b.t_uint(),
                ),
                Rq::CommittedGeometryIndex => {
                    (OP_RAY_QUERY_GET_INTERSECTION_GEOMETRY_INDEX_KHR, b.t_uint())
                }
                _ => unreachable!("外层 match 已收窄为 committed 查询族"),
            };
            let committed = b.const_uint(RAY_QUERY_COMMITTED_INTERSECTION_KHR);
            let raw = b.fresh();
            emit(
                &mut b.func_body,
                opcode,
                &[result_ty, raw, rq_var, committed],
            );
            let (ptr, dst_ty, _) = place_ptr(b, body, place)?;
            // index 三族的 SPIR-V 结果类型规范只要求「32 位整数标量」(未强制
            // 符号性),此处取**无符号**与语言面 `u32` 签名(RXS-0298)同型,
            // 常态零转换;若目标 local 位型同宽而符号性不同则 `OpBitcast`
            // 零代价换类型(防御性,不改数值位)。
            let value = if dst_ty == result_ty {
                raw
            } else {
                let cast = b.fresh();
                emit(&mut b.func_body, OP_BITCAST, &[dst_ty, cast, raw]);
                cast
            };
            emit(&mut b.func_body, OP_STORE, &[ptr, value]);
            Ok(())
        }
        Rq::Initialize => Err(VulkanCodegenError::unsupported(
            span,
            "`ray_query_initialize` 经 Rvalue::RayQueryInitialize 降级,不入方法族",
        )),
    }
}

/// `initialize` 目标 place → RayQuery Function 变量 id(按需建)。
///
/// 目标须为无投影的 `RayQuery` local(RXS-0297 位置纪律 + Function-only 收窄)。
/// 变量落 `func_vars` 流(entry block 首,SPIR-V 要求 Function 变量前置)。
fn ray_query_var_for(
    b: &mut Builder,
    body: &Body,
    place: &Place,
) -> Result<u32, VulkanCodegenError> {
    let span = body.locals[place.local.0 as usize].span;
    if !place.proj.is_empty() {
        return Err(VulkanCodegenError::unsupported(
            span,
            "RayQuery 目标须为无投影 local(禁止逃逸/投影,RXS-0297)",
        ));
    }
    if !matches!(&body.locals[place.local.0 as usize].ty,
        Ty::Adt(d, _) if b.res.lang_items.is_ray_query(*d))
    {
        return Err(VulkanCodegenError::unsupported(
            span,
            "`ray_query_initialize` 目标 local 类型非 `RayQuery`",
        ));
    }
    if let Some(&var) = b.ray_query_var.get(&place.local.0) {
        return Ok(var);
    }
    let rq_ty = b.t_ray_query();
    let ptr = b.ptr_type(STORAGE_FUNCTION, rq_ty);
    let var = b.fresh();
    emit(&mut b.func_vars, OP_VARIABLE, &[ptr, var, STORAGE_FUNCTION]);
    b.ray_query_var.insert(place.local.0, var);
    Ok(var)
}

/// 取 f32 标量操作数值 id(非 f32 → RX6026)。
fn f32_operand(
    b: &mut Builder,
    body: &Body,
    o: &Operand,
    span: Span,
    what: &str,
) -> Result<u32, VulkanCodegenError> {
    match operand(b, body, o)? {
        Some((id, _, ValueKind::Scalar(PrimTy::F32))) => Ok(id),
        _ => Err(VulkanCodegenError::unsupported(
            span,
            format!("{what} 必须是 f32 标量(RXS-0298 签名)"),
        )),
    }
}

/// 取 3 分量 f32 向量操作数值 id(非 vec3&lt;f32&gt; → RX6026)。
fn vec3_f32_operand(
    b: &mut Builder,
    body: &Body,
    o: &Operand,
    span: Span,
    what: &str,
) -> Result<u32, VulkanCodegenError> {
    match operand(b, body, o)? {
        Some((id, _, ValueKind::Vector(PrimTy::F32, 3))) => Ok(id),
        _ => Err(VulkanCodegenError::unsupported(
            span,
            format!("{what} 必须是 3 分量 f32 向量(RXS-0298 签名 `vec3<f32>`)"),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_atomic(
    b: &mut Builder,
    body: &Body,
    dest: &Place,
    op: AtomicOp,
    target_local: LocalIdx,
    index: Option<&Operand>,
    value: &Operand,
    compare: Option<&Operand>,
) -> Result<(), VulkanCodegenError> {
    let span = body.locals[target_local.0 as usize].span;
    let (var, prim) = b
        .buffer_var
        .get(&target_local.0)
        .copied()
        .ok_or_else(|| VulkanCodegenError::unsupported(span, "atomic target 不是 SSBO 形参"))?;
    if !matches!(prim, PrimTy::I32 | PrimTy::U32 | PrimTy::I64 | PrimTy::U64) {
        return Err(VulkanCodegenError::unsupported(
            span,
            "Vulkan compute 原子仅支持 i32/u32/i64/u64",
        ));
    }
    if is_64bit_prim(prim) {
        b.uses_int64_atomics = true;
    }
    let elem_ty = b.prim_type(prim, span)?;
    let ptr_elem = b.ptr_type(b.ssbo_storage(), elem_ty);
    let idx = if let Some(index) = index {
        let Some((id, _, ValueKind::Scalar(_))) = operand(b, body, index)? else {
            return Err(VulkanCodegenError::unsupported(
                span,
                "atomic index 必须是标量",
            ));
        };
        id
    } else {
        b.const_uint(0)
    };
    let member0 = b.const_uint(0);
    let ptr = b.fresh();
    emit(
        &mut b.func_body,
        OP_ACCESS_CHAIN,
        &[ptr_elem, ptr, var, member0, idx],
    );
    let Some((value_id, _, ValueKind::Scalar(value_prim))) = operand(b, body, value)? else {
        return Err(VulkanCodegenError::unsupported(
            span,
            "atomic value 必须是标量",
        ));
    };
    if b.prim_type(value_prim, span)? != elem_ty {
        return Err(VulkanCodegenError::unsupported(
            span,
            "atomic value 与目标元素类型不一致",
        ));
    }
    let scope = b.const_uint(SCOPE_DEVICE);
    let semantics = b.const_uint(MEM_SEM_RELAXED);
    let result = b.fresh();
    if op == AtomicOp::CompareExchange {
        let compare = compare.ok_or_else(|| {
            VulkanCodegenError::unsupported(span, "compare_exchange 缺 expected 实参")
        })?;
        let Some((compare_id, compare_ty, ValueKind::Scalar(_))) = operand(b, body, compare)?
        else {
            return Err(VulkanCodegenError::unsupported(
                span,
                "compare_exchange expected 必须是标量",
            ));
        };
        if compare_ty != elem_ty {
            return Err(VulkanCodegenError::unsupported(
                span,
                "compare_exchange expected 类型与目标不一致",
            ));
        }
        emit(
            &mut b.func_body,
            OP_ATOMIC_COMPARE_EXCHANGE,
            &[
                elem_ty, result, ptr, scope, semantics, semantics, value_id, compare_id,
            ],
        );
    } else {
        let opcode = match op {
            AtomicOp::FetchAdd => OP_ATOMIC_IADD,
            AtomicOp::FetchSub => OP_ATOMIC_ISUB,
            AtomicOp::FetchMin if is_signed_prim(prim) => OP_ATOMIC_SMIN,
            AtomicOp::FetchMin => OP_ATOMIC_UMIN,
            AtomicOp::FetchMax if is_signed_prim(prim) => OP_ATOMIC_SMAX,
            AtomicOp::FetchMax => OP_ATOMIC_UMAX,
            AtomicOp::FetchAnd => OP_ATOMIC_AND,
            AtomicOp::FetchOr => OP_ATOMIC_OR,
            AtomicOp::FetchXor => OP_ATOMIC_XOR,
            AtomicOp::Exchange => OP_ATOMIC_EXCHANGE,
            AtomicOp::CompareExchange => unreachable!(),
        };
        emit(
            &mut b.func_body,
            opcode,
            &[elem_ty, result, ptr, scope, semantics, value_id],
        );
    }
    let (dest_ptr, dest_ty, _) = place_ptr(b, body, dest)?;
    if dest_ty != elem_ty {
        return Err(VulkanCodegenError::unsupported(
            span,
            "atomic 返回值类型与目标 local 不一致",
        ));
    }
    emit(&mut b.func_body, OP_STORE, &[dest_ptr, result]);
    Ok(())
}

fn emit_storage_image_op(
    b: &mut Builder,
    body: &Body,
    dest: &Place,
    texture_local: LocalIdx,
    method: ResourceMethod,
    coord: &Operand,
    extra: &[Operand],
) -> Result<(), VulkanCodegenError> {
    let span = body.locals[texture_local.0 as usize].span;
    let (var, image_ty, prim) =
        b.image_var.get(&texture_local.0).copied().ok_or_else(|| {
            VulkanCodegenError::unsupported(span, "storage image receiver 未绑定")
        })?;
    let Some((coord_id, _, ValueKind::Vector(coord_prim, 2))) = operand(b, body, coord)? else {
        return Err(VulkanCodegenError::unsupported(
            span,
            "TextureRw2D 坐标必须是 2 分量向量",
        ));
    };
    if !matches!(coord_prim, PrimTy::U32 | PrimTy::Usize) {
        return Err(VulkanCodegenError::unsupported(
            span,
            "TextureRw2D 坐标分量必须是 u32",
        ));
    }
    let image = b.fresh();
    emit(&mut b.func_body, OP_LOAD, &[image_ty, image, var]);
    match method {
        ResourceMethod::Store => {
            let [value] = extra else {
                return Err(VulkanCodegenError::unsupported(
                    span,
                    "TextureRw2D.store 需要一个 value",
                ));
            };
            let Some((value_id, _, ValueKind::Vector(value_prim, 4))) = operand(b, body, value)?
            else {
                return Err(VulkanCodegenError::unsupported(
                    span,
                    "TextureRw2D.store value 必须是 4 分量向量",
                ));
            };
            if value_prim != prim {
                return Err(VulkanCodegenError::unsupported(
                    span,
                    "TextureRw2D.store value 分量类型不匹配",
                ));
            }
            emit(
                &mut b.func_body,
                OP_IMAGE_WRITE,
                &[image, coord_id, value_id],
            );
            Ok(())
        }
        ResourceMethod::StorageLoad => {
            let result_ty = b.vector_type(prim, 4, span)?;
            let result = b.fresh();
            emit(
                &mut b.func_body,
                OP_IMAGE_READ,
                &[result_ty, result, image, coord_id],
            );
            let (dest_ptr, dest_ty, _) = place_ptr(b, body, dest)?;
            if dest_ty != result_ty {
                return Err(VulkanCodegenError::unsupported(
                    span,
                    "TextureRw2D.load 结果 local 类型不匹配",
                ));
            }
            emit(&mut b.func_body, OP_STORE, &[dest_ptr, result]);
            Ok(())
        }
        _ => Err(VulkanCodegenError::unsupported(
            span,
            "compute TextureRw2D 仅支持 load/store",
        )),
    }
}

/// Cast → SPIR-V 转换 opcode(compute 标量子集含 i64/u64)。
/// 调用方应先判 src_ty_id == dst_ty_id 走 identity;本函数仅处理不同 SPIR-V 类型的转换。
/// `as` 数值 cast 映射表(compute 路事实源;G7.5b 起 `pub(crate)` 与图形扩展路
/// 双路共享——仅改可见性不改 compute 发射序,RXS-0301「表级复用语义中性」)。
/// **f64 目标不在本表裁决**:调用方的标量类型映射先拒 F64(RXS-0203 L1 /
/// RXS-0301 L3),本表仅接受已过类型面的 prim 对。
pub(crate) fn cast_opcode(src: PrimTy, dst: PrimTy) -> Result<u16, VulkanCodegenError> {
    let is_src_int = !matches!(src, PrimTy::F32);
    let is_dst_int = !matches!(dst, PrimTy::F32);
    Ok(match (src, dst) {
        // int → f32:unsigned 走 OpConvertUToF,signed 走 OpConvertSToF
        (_, PrimTy::F32) if is_src_int => {
            if is_signed_prim(src) {
                OP_CONVERT_S_TO_F
            } else {
                OP_CONVERT_U_TO_F
            }
        }
        // f32 → int:unsigned 走 OpConvertFToU,signed 走 OpConvertFToS
        (PrimTy::F32, _) if is_dst_int => {
            if is_signed_prim(dst) {
                OP_CONVERT_F_TO_S
            } else {
                OP_CONVERT_F_TO_U
            }
        }
        // 整数扩窄按源操作数符号解释；同位宽 signedness 变化为位型重解释。
        _ if is_src_int && is_dst_int && int_width(src) != int_width(dst) => {
            if is_signed_prim(src) {
                OP_SCONVERT
            } else {
                OP_UCONVERT
            }
        }
        _ if is_src_int && is_dst_int => OP_BITCAST,
        _ => OP_BITCAST,
    })
}

/// BinOp → (SPIR-V opcode, 结果是否 bool)。compute 路事实源;G7.5b 起 `pub(crate)`
/// 与图形扩展路双路共享(含比较/位运算;仅改可见性不改 compute 发射序,RXS-0301)。
pub(crate) fn binop_opcode(
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
        BinOp::BitAnd if !is_float => (OP_BITWISE_AND, false),
        BinOp::BitOr if !is_float => (OP_BITWISE_OR, false),
        BinOp::BitXor if !is_float => (OP_BITWISE_XOR, false),
        BinOp::Shl if !is_float => (OP_SHIFT_LEFT_LOGICAL, false),
        BinOp::Shr if !is_float && is_signed => (OP_SHIFT_RIGHT_ARITHMETIC, false),
        BinOp::Shr if !is_float => (OP_SHIFT_RIGHT_LOGICAL, false),
        BinOp::BitAnd
        | BinOp::BitOr
        | BinOp::BitXor
        | BinOp::Shl
        | BinOp::Shr
        | BinOp::And
        | BinOp::Or => {
            return Err(VulkanCodegenError::unsupported(
                span,
                "Vulkan compute 逻辑与/或及浮点位运算不在当前子集",
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
            // G7.2 W3a:本块若是循环 latch(回边源)→ 改跳**合成 continue 块**,
            // 并就地发射该合成块(`OpLabel` + `OpBranch <header>`),把真实回边
            // 移到合成块上;原块回归纯 selection merge 角色(见
            // `Builder::loop_continue_label`)。
            if let Some(&cont_lbl) = b.loop_continue_label.get(&bi) {
                let header_lbl = b.block_label[&(t.0 as usize)];
                emit(&mut b.func_body, OP_BRANCH, &[cont_lbl]);
                emit(&mut b.func_body, OP_LABEL, &[cont_lbl]);
                emit(&mut b.func_body, OP_BRANCH, &[header_lbl]);
                return Ok(());
            }
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
            let then_i = then.0 as usize;
            let else_i = else_.0 as usize;
            let then_lbl = b.block_label[&then_i];
            let else_lbl = b.block_label[&else_i];
            // G7.2 W3a:本块若是**循环头**(存在回边指向它)→ 发 `OpLoopMerge`
            // (而非 `OpSelectionMerge`)。SPIR-V 结构化控制流要求回边目标必须是
            // 声明了 `OpLoopMerge` 的循环头;沿用 selection 会被 spirv-val 拒。
            // 形态 = while 循环的规范降级:条件指令在头块内、`OpLoopMerge` 紧邻
            // 终结子之前、`OpBranchConditional` 一臂为 body、另一臂为 merge。
            if let Some(&(merge_i, latch_i)) = b.loop_info.get(&bi) {
                let merge_lbl = b.block_label[&merge_i];
                let continue_lbl = b.loop_continue_label[&latch_i];
                emit(
                    &mut b.func_body,
                    OP_LOOP_MERGE,
                    &[merge_lbl, continue_lbl, LOOP_CONTROL_NONE],
                );
                emit(
                    &mut b.func_body,
                    OP_BRANCH_CONDITIONAL,
                    &[cond, then_lbl, else_lbl],
                );
                return Ok(());
            }
            // 结构化 selection merge 块。
            let merge = structured_merge(body, then_i, else_i).ok_or_else(|| {
                VulkanCodegenError::unsupported(
                    bb.terminator.span,
                    "Vulkan compute 首期仅支持结构化 if 与 while(分支须收敛于唯一 merge 块;提前 return 属后续分片)",
                )
            })?;
            let merge_lbl = b.block_label[&merge];
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

/// 循环头判定 + `OpLoopMerge` 的 (merge, continue) 目标(G7.2 W3a)。
///
/// `header` 为带 `SwitchBool` 终结子的块。返回 `Some((merge, continue))` 当且仅当
/// 存在**回边**(某块的后继指向 `header` 且该块下标 ≥ `header`)——即 `header` 是
/// 循环头。
///
/// - `continue` = 回边源块(latch)。多个回边源 → `None`(非单 latch 结构化循环,
///   保守拒,不产可能非法的模块)。
/// - `merge` = `SwitchBool` 两臂中**到不了 latch** 的那一臂(循环出口);另一臂为
///   循环体。以「绕过 header 的可达性」判定,不假定 then/else 极性(误判方向恒为
///   拒:两臂皆可达或皆不可达 latch → `None`)。
fn loop_merge_targets(body: &Body, header: usize) -> Option<(usize, usize)> {
    let TerminatorKind::SwitchBool { then, else_, .. } = &body.blocks[header].terminator.kind
    else {
        return None;
    };
    // 回边源:后继含 header 且自身下标 ≥ header(MIR 块按降级序编号,循环体块
    // 下标大于循环头)。
    let mut latches = (0..body.blocks.len())
        .filter(|&i| i >= header && block_succs(&body.blocks[i]).contains(&header));
    let latch = latches.next()?;
    if latches.next().is_some() {
        return None; // 多 latch:非单回边结构化循环,保守拒
    }
    // 绕过 header 的可达性(header 作屏障:循环体内到 latch 无须再经 header)。
    let reaches_latch = |start: usize| -> bool {
        let mut seen = vec![false; body.blocks.len()];
        let mut work = vec![start];
        seen[start] = true;
        while let Some(cur) = work.pop() {
            if cur == latch {
                return true;
            }
            for s in block_succs(&body.blocks[cur]) {
                if s != header && !seen[s] {
                    seen[s] = true;
                    work.push(s);
                }
            }
        }
        false
    };
    let then_i = then.0 as usize;
    let else_i = else_.0 as usize;
    match (reaches_latch(then_i), reaches_latch(else_i)) {
        (true, false) => Some((else_i, latch)),
        (false, true) => Some((then_i, latch)),
        // 两臂皆达 / 皆不达 latch:出口不唯一或形态非预期 → 保守拒。
        _ => None,
    }
}

/// 结构化 if 的 merge 块 = 两臂最近共同可达块。不能按 MIR block 下标最小值取：
/// 嵌套 if 的外层 merge 往往编号更小，会造成多个 header 复用同一 merge，触发
/// `Block is already a merge block for another header`。
/// (G7.5b 起 `pub(crate)`:图形扩展路复用同一前向可达交汇算法,RXS-0301 IR 逐字。)
pub(crate) fn structured_merge(body: &Body, then_i: usize, else_i: usize) -> Option<usize> {
    let distance = |start: usize| {
        let mut dist = vec![usize::MAX; body.blocks.len()];
        dist[start] = 0;
        let mut work = vec![start];
        while let Some(block) = work.pop() {
            let next_distance = dist[block].saturating_add(1);
            for succ in block_succs(&body.blocks[block]) {
                if next_distance < dist[succ] {
                    dist[succ] = next_distance;
                    work.push(succ);
                }
            }
        }
        dist
    };
    let then_distance = distance(then_i);
    let else_distance = distance(else_i);
    (0..body.blocks.len())
        .filter(|&block| then_distance[block] != usize::MAX && else_distance[block] != usize::MAX)
        .min_by_key(|&block| {
            (
                then_distance[block].max(else_distance[block]),
                then_distance[block] + else_distance[block],
                block,
            )
        })
}

/// libdevice `__nv_*` 数学符号 → (GLSL.std.450 ext-inst 编号, arity)。RXS-0205 首期覆盖
/// 20 个 `DeviceMathFn` 中的 1:1 可映射项;`cbrt`/`log10`(需 Pow/Log 组合)→ None(后续
/// 分片)。符号形态:`__nv_<base>` (f64) / `__nv_<base>f` (f32);base 无一以 'f' 结尾,
/// 故 strip 尾 'f' 唯一恢复 base(ext-inst 按操作数类型分发,f32/f64 同一编号)。
/// (G7.5b 起 `pub(crate)` 与图形扩展路双路共享——`round`→`RoundEven` 同表,
/// RXS-0301「首批仅 round,与 compute 路 `glsl_ext_op` 同表」;仅改可见性。)
pub(crate) fn glsl_ext_op(nv_symbol: &str) -> Option<(u32, usize)> {
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
        // G4.2,RXS-0275:mesh intrinsic → OpSetMeshOutputsEXT。
        // mesh 阶段(body.stage == Mesh)lowering;compute 路径防御拒。
        CallTarget::MeshIntrinsic(mesh_intr) => match mesh_intr {
            MeshIntrinsic::SetMeshOutputs => {
                if body.stage != Some(crate::ast::ShaderStage::Mesh) {
                    return Err(VulkanCodegenError::unsupported(
                        span,
                        "mesh intrinsic 在 compute codegen 路径不可达(走 lower_mesh,RXS-0275)",
                    ));
                }
                // mesh_set_outputs(vertex_count, primitive_count) → OpSetMeshOutputsEXT。
                if args.len() != 2 {
                    return Err(VulkanCodegenError::unsupported(
                        span,
                        "mesh_set_outputs 期望 2 实参(vertex_count, primitive_count)",
                    ));
                }
                let Some((vc, _, _)) = operand(b, body, &args[0])? else {
                    return Err(VulkanCodegenError::unsupported(
                        span,
                        "mesh_set_outputs vertex_count 实参为零尺寸值",
                    ));
                };
                let Some((pc, _, _)) = operand(b, body, &args[1])? else {
                    return Err(VulkanCodegenError::unsupported(
                        span,
                        "mesh_set_outputs primitive_count 实参为零尺寸值",
                    ));
                };
                emit(&mut b.func_body, OP_SET_MESH_OUTPUTS_EXT, &[vc, pc]);
                Ok(())
            }
        },
        // G4.2,RXS-0275:task intrinsic → OpEmitMeshTasksEXT。
        // task 条件臂首期不开放(RXS-0270);compute 路径防御拒。
        CallTarget::TaskIntrinsic(task_intr) => match task_intr {
            TaskIntrinsic::EmitMeshTasks => {
                if body.stage != Some(crate::ast::ShaderStage::Task) {
                    return Err(VulkanCodegenError::unsupported(
                        span,
                        "task intrinsic 在 compute codegen 路径不可达(走 lower_task,RXS-0275)",
                    ));
                }
                // task 条件臂首期不开放(RXS-0270):类型面预留,lowering 防御拒。
                Err(VulkanCodegenError::unsupported(
                    span,
                    "task 阶段条件臂首期不开放(RXS-0270);emit_mesh_tasks lowering 待评估窗兑现",
                ))
            }
        },
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
    if b.uses_int64 {
        emit(&mut m, OP_CAPABILITY, &[CAP_INT64]);
    }
    if b.uses_int64_atomics {
        emit(&mut m, OP_CAPABILITY, &[CAP_INT64_ATOMICS]);
    }
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

/// compute RayQuery 模块组装(G7.2 W3a,RXS-0300):SPIR-V **1.4** header +
/// `RayQueryKHR` capability + `OpExtension "SPV_KHR_ray_query"` + `OpEntryPoint`
/// interface **全量枚举**。
///
/// 与既有 [`assemble`](恒 1.0)**并列**、不改其一字节 —— 分叉落发射函数级
/// (承 RXS-0247 既有机制),故 W1/W2 五 kernel 与全部既有 vulkan golden 字节零漂移。
/// 与 [`assemble_mesh`] 的差异仅在 capability/extension/执行模型三处;其余 logical
/// layout 逐节同序。
fn assemble_ray_query(b: &mut Builder, entry_name: &str) -> Vec<u32> {
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
        SPIRV_VERSION_1_4,
        SPIRV_GENERATOR,
        bound,
        SPIRV_SCHEMA,
    ];
    emit(&mut m, OP_CAPABILITY, &[CAP_SHADER]);
    if b.uses_int64 {
        emit(&mut m, OP_CAPABILITY, &[CAP_INT64]);
    }
    if b.uses_int64_atomics {
        emit(&mut m, OP_CAPABILITY, &[CAP_INT64_ATOMICS]);
    }
    // capability 只按真实使用声明(承 Int64/Int64Atomics 先例):本发射路径的
    // 进入条件即 `uses_ray_query`,故此处恒声明 `RayQueryKHR`。
    emit(&mut m, OP_CAPABILITY, &[CAP_RAY_QUERY_KHR]);
    // OpExtension(layout:capability 之后、OpExtInstImport 之前)。
    let mut ext_ops = Vec::new();
    push_string(&mut ext_ops, EXT_SPV_KHR_RAY_QUERY);
    emit(&mut m, OP_EXTENSION, &ext_ops);
    m.extend_from_slice(&b.ext_imports);
    emit(
        &mut m,
        OP_MEMORY_MODEL,
        &[ADDR_MODEL_LOGICAL, MEM_MODEL_GLSL450],
    );
    // OpEntryPoint GLCompute %main "<entry>" <interface...>。
    // SPIR-V 1.4 起 interface 须枚举**全部**被引用全局变量(不再限 Input/Output),
    // 与 mesh/RT 同律(RXS-0247/0300)→ builtin(Input)+ 描述符/push-constant。
    let mut ep = vec![EXEC_MODEL_GLCOMPUTE, b.main_id];
    push_string(&mut ep, entry_name);
    ep.extend_from_slice(&b.entry_interface);
    ep.extend_from_slice(&b.global_vars);
    emit(&mut m, OP_ENTRY_POINT, &ep);
    emit(
        &mut m,
        OP_EXECUTION_MODE,
        &[b.main_id, EXEC_MODE_LOCAL_SIZE, 1, 1, 1],
    );
    m.extend_from_slice(&b.decorations);
    m.extend_from_slice(&b.types_globals);
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

/// mesh 模块组装(G4.2,RXS-0275):SPIR-V 1.4 + MeshEXT 执行模型 +
/// SPV_EXT_mesh_shader 扩展 + mesh_meta 派生的 execution modes。
/// 镜像 [`assemble`] 的 logical layout,但 header 版本 1.4(per-entry 分叉)、
/// capability = MeshShadingEXT、OpEntryPoint = MeshEXT、execution modes 含
/// LocalSize/OutputVertices/OutputPrimitivesEXT/OutputTrianglesEXT。
fn assemble_mesh(b: &mut Builder, entry_name: &str, meta: &MeshEntryMeta) -> Vec<u32> {
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
        SPIRV_VERSION_1_4,
        SPIRV_GENERATOR,
        bound,
        SPIRV_SCHEMA,
    ];
    emit(&mut m, OP_CAPABILITY, &[CAP_MESH_SHADING_EXT]);
    // SPV_EXT_mesh_shader extension(layout 在 memory-model 之前)。
    let mut ext_ops = Vec::new();
    push_string(&mut ext_ops, EXT_MESH_SHADER);
    emit(&mut m, OP_EXTENSION, &ext_ops);
    m.extend_from_slice(&b.ext_imports);
    emit(
        &mut m,
        OP_MEMORY_MODEL,
        &[ADDR_MODEL_LOGICAL, MEM_MODEL_GLSL450],
    );
    // OpEntryPoint MeshEXT %main "<entry>" <interface...>。
    let mut ep = vec![EXEC_MODEL_MESH_EXT, b.main_id];
    push_string(&mut ep, entry_name);
    ep.extend_from_slice(&b.entry_interface);
    emit(&mut m, OP_ENTRY_POINT, &ep);
    // execution modes 自 mesh_meta(RXS-0275)。
    emit(
        &mut m,
        OP_EXECUTION_MODE,
        &[
            b.main_id,
            EXEC_MODE_LOCAL_SIZE,
            meta.numthreads.0,
            meta.numthreads.1,
            meta.numthreads.2,
        ],
    );
    emit(
        &mut m,
        OP_EXECUTION_MODE,
        &[b.main_id, EXEC_MODE_OUTPUT_VERTICES, meta.max_vertices],
    );
    emit(
        &mut m,
        OP_EXECUTION_MODE,
        &[
            b.main_id,
            EXEC_MODE_OUTPUT_PRIMITIVES_EXT,
            meta.max_primitives,
        ],
    );
    emit(
        &mut m,
        OP_EXECUTION_MODE,
        &[b.main_id, EXEC_MODE_OUTPUT_TRIANGLES_EXT],
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

fn value_kind(ty: &Ty) -> Option<ValueKind> {
    if let Some(prim) = prim_of(ty) {
        return Some(ValueKind::Scalar(prim));
    }
    let Ty::Tuple(elems) = ty else {
        return None;
    };
    let len = u32::try_from(elems.len()).ok()?;
    // G7.2 W3a(RXS-0300):放行 3 分量(`vec3<f32>` ray origin/direction 的结构性
    // 元组表示,`OpRayQueryInitializeKHR` 的 RayOrigin/RayDirection 操作数为
    // 3 分量向量)。**纯加性**:既有 W1/W2 五 kernel 无 3 元组 local,先前落
    // RX6026 的 3 元组自此可编码,方向为「原拒→现受」,零既有 golden 影响。
    if !matches!(len, 2..=4) {
        return None;
    }
    let prim = elems.first().and_then(prim_of)?;
    elems
        .iter()
        .all(|elem| prim_of(elem) == Some(prim))
        .then_some(ValueKind::Vector(prim, len))
}

fn is_signed_prim(p: PrimTy) -> bool {
    matches!(p, PrimTy::I8 | PrimTy::I16 | PrimTy::I32 | PrimTy::I64)
}

fn is_64bit_prim(p: PrimTy) -> bool {
    matches!(p, PrimTy::I64 | PrimTy::U64)
}

fn int_width(p: PrimTy) -> u32 {
    if is_64bit_prim(p) { 64 } else { 32 }
}

fn prim_layout(p: PrimTy) -> (u32, u32) {
    if is_64bit_prim(p) { (8, 8) } else { (4, 4) }
}

fn align_up(value: u32, align: u32) -> u32 {
    value.div_ceil(align) * align
}

/// SPIR-V 字流 → 小端字节序 `.spv`。
pub fn words_to_bytes(words: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(words.len() * 4);
    for w in words {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

// ═══════════════════════ G3.6 mesh/task/RT SPIR-V 编码(RFC-0013 §4.E5/E6) ═══════════════════════
//
// mesh/task 复用本模块 workgroup/LocalSize 基建(E-4 采纳,§4.E5 钉死落点);RT 六执行模型
// 亦落本模块(§4.E6「dxil_spirv.rs 或 vulkan_codegen.rs 钉落点」——归 workgroup 发射器同址)。
// **per-entry SPIR-V 1.4 分叉**:本节全部产物 header 恒 [`SPIRV_VERSION_1_4`],`OpEntryPoint`
// interface 枚举全部被引用全局变量;既有 compute [`assemble`](1.0)/ vertex·fragment
// [`crate::dxil_spirv::emit_spirv_inner`](1.0)字节零漂移(零回归门)。
//
// **首期语料形态**(§8 accept-only 边界):mesh = 单三角形非空输出;task = payload 写 +
// EmitMeshTasks;raygen/miss/closesthit = 三件套最小形态。这些发射器产**固定最小合规模块**
// (库级见证,镜像 dxil_corpus / sampling_vulkan_spirv_val 无 CLI 见证);从真实 `.rx` MIR
// 体(SetMeshOutputs / EmitMeshTasks / TraceRay 的 mir_build intrinsic 降级)接线归后续 PR。
//
// 数值取值 = 参考 glslang 产物字节解证(build/spike-sampling-probe 探针 mesh.spv/rg.spv 反汇编)。

// 追加 SPIR-V 常量(本模块既有集之外)。
const OP_EXTENSION: u16 = 10;
const OP_TYPE_ARRAY: u16 = 28;
const OP_CONSTANT_COMPOSITE: u16 = 44;

// 执行模型(RFC-0013 §4.E5/E6)。
const EXEC_MODEL_MESH_EXT: u32 = 5365;
const EXEC_MODEL_TASK_EXT: u32 = 5364;
const EXEC_MODEL_RAY_GENERATION_KHR: u32 = 5313;
const EXEC_MODEL_INTERSECTION_KHR: u32 = 5314;
const EXEC_MODEL_ANY_HIT_KHR: u32 = 5315;
const EXEC_MODEL_CLOSEST_HIT_KHR: u32 = 5316;
const EXEC_MODEL_MISS_KHR: u32 = 5317;
const EXEC_MODEL_CALLABLE_KHR: u32 = 5318;

// capability。
const CAP_MESH_SHADING_EXT: u32 = 5283;
const CAP_RAY_TRACING_KHR: u32 = 4479;

// 扩展字符串。
const EXT_MESH_SHADER: &str = "SPV_EXT_mesh_shader";
const EXT_RAY_TRACING: &str = "SPV_KHR_ray_tracing";

// execution modes(mesh)。
const EXEC_MODE_OUTPUT_VERTICES: u32 = 26;
const EXEC_MODE_OUTPUT_PRIMITIVES_EXT: u32 = 5270;
const EXEC_MODE_OUTPUT_TRIANGLES_EXT: u32 = 5298;

// 存储类(mesh/RT)。
const STORAGE_OUTPUT: u32 = 3;
const STORAGE_RAY_PAYLOAD_KHR: u32 = 5338;
const STORAGE_HIT_ATTRIBUTE_KHR: u32 = 5339;
const STORAGE_INCOMING_RAY_PAYLOAD_KHR: u32 = 5342;
const STORAGE_INCOMING_CALLABLE_DATA_KHR: u32 = 5329;
const STORAGE_TASK_PAYLOAD_WORKGROUP_EXT: u32 = 5402;

// builtin(mesh)。
const BUILTIN_POSITION: u32 = 0;
const BUILTIN_PRIMITIVE_TRIANGLE_INDICES_EXT: u32 = 5296;

// RT / mesh 专属指令。
const OP_TYPE_ACCELERATION_STRUCTURE_KHR: u16 = 5341;
const OP_TRACE_RAY_KHR: u16 = 4445;
const OP_REPORT_INTERSECTION_KHR: u16 = 5334;
const OP_SET_MESH_OUTPUTS_EXT: u16 = 5295;
const OP_EMIT_MESH_TASKS_EXT: u16 = 5294;

// storage image 写出面(RT raygen payload → UAV;§4.E8 device 见证落点)。opcode 取值 =
// SPIR-V core 规范,与 glslang rg.spv 反汇编逐字核对(build/spike-sampling-probe)。
const OP_VECTOR_SHUFFLE: u16 = 79;

// RT launch builtin(BuiltIn LaunchIdKHR/LaunchSizeKHR;取自 glslang rg.spv `OpDecorate … BuiltIn`)。
const BUILTIN_LAUNCH_ID_KHR: u32 = 5319;
const BUILTIN_LAUNCH_SIZE_KHR: u32 = 5320;

// OpTypeImage Dim(2D=1)+ storage image 显式 format(§4.B5「OpTypeImage 带显式 format」纪律)。
// Rgba8(=4)↔ vk.rs `run_rt_inner` storage image 的 `VK_FORMAT_R8G8B8A8_UNORM`(UAV 回读逐纹素
// 4B);format-qualified write 须与 image view 格式一致,故取 Rgba8 而非探针的 Rgba32f。
const IMAGE_FORMAT_RGBA8: u32 = 4;

// RayFlags / cull mask(§4.E4 已知签名固定:opaque / 0xFF / SBT 恒 0)。
const RAY_FLAG_OPAQUE: u32 = 1;
const CULL_MASK_ALL: u32 = 0xFF;

/// 固定最小合规 SPIR-V 模块构造器(mesh/task/RT;E5/E6)。分节累积,末尾按 SPIR-V logical
/// layout 组装,header 恒 1.4(per-entry 版本轴)。
struct ExtBuilder {
    next_id: u32,
    caps: Vec<u32>,
    exts: Vec<&'static str>,
    exec_modes: Vec<u32>,
    decorations: Vec<u32>,
    types: Vec<u32>,
    body: Vec<u32>,
    interface: Vec<u32>,
    void_id: u32,
    fn_ty_id: u32,
    main_id: u32,
    entry_label: u32,
}

impl ExtBuilder {
    fn new(caps: Vec<u32>, exts: Vec<&'static str>) -> Self {
        let mut b = ExtBuilder {
            next_id: 1,
            caps,
            exts,
            exec_modes: Vec::new(),
            decorations: Vec::new(),
            types: Vec::new(),
            body: Vec::new(),
            interface: Vec::new(),
            void_id: 0,
            fn_ty_id: 0,
            main_id: 0,
            entry_label: 0,
        };
        b.void_id = b.type_result(OP_TYPE_VOID, &[]);
        b.fn_ty_id = b.type_result(OP_TYPE_FUNCTION, &[b.void_id]);
        b.main_id = b.id();
        b.entry_label = b.id();
        b
    }

    fn id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// 结果型指令(结果 id 为首操作数)→ types 段;返回结果 id。
    fn type_result(&mut self, opcode: u16, tail: &[u32]) -> u32 {
        let id = self.id();
        let mut ops = vec![id];
        ops.extend_from_slice(tail);
        emit(&mut self.types, opcode, &ops);
        id
    }

    /// `OpConstant <ty> <id> <value>`。
    fn constant(&mut self, ty: u32, value: u32) -> u32 {
        let id = self.id();
        emit(&mut self.types, OP_CONSTANT, &[ty, id, value]);
        id
    }

    /// `OpConstantComposite <ty> <id> <comps...>`。
    fn const_composite(&mut self, ty: u32, comps: &[u32]) -> u32 {
        let id = self.id();
        let mut ops = vec![ty, id];
        ops.extend_from_slice(comps);
        emit(&mut self.types, OP_CONSTANT_COMPOSITE, &ops);
        id
    }

    /// `OpVariable <ptr_ty> <id> <storage>` → types 段(全局变量);返回变量 id。
    /// `in_interface` = 是否登记入 `OpEntryPoint` interface(1.4:全部被引用全局变量)。
    fn global_var(&mut self, ptr_ty: u32, storage: u32, in_interface: bool) -> u32 {
        let id = self.id();
        emit(&mut self.types, OP_VARIABLE, &[ptr_ty, id, storage]);
        if in_interface {
            self.interface.push(id);
        }
        id
    }

    fn decorate(&mut self, target: u32, deco: u32, args: &[u32]) {
        let mut ops = vec![target, deco];
        ops.extend_from_slice(args);
        emit(&mut self.decorations, OP_DECORATE, &ops);
    }

    fn member_decorate(&mut self, target: u32, member: u32, deco: u32, args: &[u32]) {
        let mut ops = vec![target, member, deco];
        ops.extend_from_slice(args);
        emit(&mut self.decorations, OP_MEMBER_DECORATE, &ops);
    }

    fn exec_mode(&mut self, mode: u32, args: &[u32]) {
        let mut ops = vec![self.main_id, mode];
        ops.extend_from_slice(args);
        emit(&mut self.exec_modes, OP_EXECUTION_MODE, &ops);
    }

    /// 按 SPIR-V logical layout 组装(header 恒 1.4)。`body_terminated` = 函数体已自带
    /// 终结子(task 的 `OpEmitMeshTasksEXT`)→ 不追加 `OpReturn`。
    fn finish(self, exec_model: u32, body_terminated: bool) -> Vec<u32> {
        let mut m: Vec<u32> = vec![
            SPIRV_MAGIC,
            SPIRV_VERSION_1_4,
            SPIRV_GENERATOR,
            self.next_id,
            SPIRV_SCHEMA,
        ];
        for &c in &self.caps {
            emit(&mut m, OP_CAPABILITY, &[c]);
        }
        for e in &self.exts {
            let mut ops = Vec::new();
            push_string(&mut ops, e);
            emit(&mut m, OP_EXTENSION, &ops);
        }
        emit(
            &mut m,
            OP_MEMORY_MODEL,
            &[ADDR_MODEL_LOGICAL, MEM_MODEL_GLSL450],
        );
        let mut ep = vec![exec_model, self.main_id];
        push_string(&mut ep, "main");
        ep.extend_from_slice(&self.interface);
        emit(&mut m, OP_ENTRY_POINT, &ep);
        m.extend_from_slice(&self.exec_modes);
        m.extend_from_slice(&self.decorations);
        m.extend_from_slice(&self.types);
        emit(
            &mut m,
            OP_FUNCTION,
            &[
                self.void_id,
                self.main_id,
                FUNCTION_CONTROL_NONE,
                self.fn_ty_id,
            ],
        );
        emit(&mut m, OP_LABEL, &[self.entry_label]);
        m.extend_from_slice(&self.body);
        if !body_terminated {
            emit(&mut m, OP_RETURN, &[]);
        }
        emit(&mut m, OP_FUNCTION_END, &[]);
        m
    }
}

/// mesh 阶段最小合规 SPIR-V(§4.E5:MeshEXT + SPV_EXT_mesh_shader + LocalSize/OutputVertices/
/// OutputPrimitivesEXT/OutputTrianglesEXT + OpSetMeshOutputsEXT + Position/PrimitiveTriangleIndicesEXT
/// 输出 = 单三角形非空输出)。
pub fn emit_mesh_min() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_MESH_SHADING_EXT], vec![EXT_MESH_SHADER]);
    // 类型 / 常量。
    let uint = b.type_result(OP_TYPE_INT, &[32, 0]);
    let float = b.type_result(OP_TYPE_FLOAT, &[32]);
    let v4float = b.type_result(OP_TYPE_VECTOR, &[float, 4]);
    let v3uint = b.type_result(OP_TYPE_VECTOR, &[uint, 3]);
    let int = b.type_result(OP_TYPE_INT, &[32, 1]);
    let uint_1 = b.constant(uint, 1);
    let uint_3 = b.constant(uint, 3);
    let uint_0c = b.constant(uint, 0);
    let uint_2 = b.constant(uint, 2);
    let float_0 = b.constant(float, 0.0f32.to_bits());
    let float_1 = b.constant(float, 1.0f32.to_bits());
    // 覆盖屏幕中心的三角形 NDC 顶点(非退化,使 mesh 管线 device 判据 covered>0;
    // G3.6 步骤 66 像素见证)。0.7 / -0.7 对称,三顶点互异。
    let float_p7 = b.constant(float, 0.7f32.to_bits());
    let float_n7 = b.constant(float, (-0.7f32).to_bits());
    let int_0 = b.constant(int, 0);
    let int_1 = b.constant(int, 1);
    let int_2 = b.constant(int, 2);
    // per-vertex Block { vec4 Position }(单成员最小合规)。
    let per_vertex = b.type_result(OP_TYPE_STRUCT, &[v4float]);
    b.decorate(per_vertex, DECORATION_BLOCK, &[]);
    b.member_decorate(per_vertex, 0, DECORATION_BUILTIN, &[BUILTIN_POSITION]);
    let arr_pv = b.type_result(OP_TYPE_ARRAY, &[per_vertex, uint_3]);
    let ptr_out_arr_pv = b.type_result(OP_TYPE_POINTER, &[STORAGE_OUTPUT, arr_pv]);
    let verts = b.global_var(ptr_out_arr_pv, STORAGE_OUTPUT, true);
    // primitive triangle indices 输出(uvec3[max_primitives])。
    let arr_idx = b.type_result(OP_TYPE_ARRAY, &[v3uint, uint_1]);
    let ptr_out_arr_idx = b.type_result(OP_TYPE_POINTER, &[STORAGE_OUTPUT, arr_idx]);
    let prims = b.global_var(ptr_out_arr_idx, STORAGE_OUTPUT, true);
    b.decorate(
        prims,
        DECORATION_BUILTIN,
        &[BUILTIN_PRIMITIVE_TRIANGLE_INDICES_EXT],
    );
    // 元素指针型 + 常量。
    let ptr_out_v4f = b.type_result(OP_TYPE_POINTER, &[STORAGE_OUTPUT, v4float]);
    let ptr_out_v3u = b.type_result(OP_TYPE_POINTER, &[STORAGE_OUTPUT, v3uint]);
    // 三互异顶点(v0 上 / v1 左下 / v2 右下),覆盖屏幕中心。
    let pos0 = b.const_composite(v4float, &[float_0, float_n7, float_0, float_1]);
    let pos1 = b.const_composite(v4float, &[float_n7, float_p7, float_0, float_1]);
    let pos2 = b.const_composite(v4float, &[float_p7, float_p7, float_0, float_1]);
    let tri = b.const_composite(v3uint, &[uint_0c, uint_1, uint_2]);
    // execution modes。
    b.exec_mode(EXEC_MODE_LOCAL_SIZE, &[1, 1, 1]);
    b.exec_mode(EXEC_MODE_OUTPUT_VERTICES, &[3]);
    b.exec_mode(EXEC_MODE_OUTPUT_PRIMITIVES_EXT, &[1]);
    b.exec_mode(EXEC_MODE_OUTPUT_TRIANGLES_EXT, &[]);
    // 函数体:SetMeshOutputs(3,1) + 三顶点 Position 写 + 单三角形索引写。
    emit(&mut b.body, OP_SET_MESH_OUTPUTS_EXT, &[uint_3, uint_1]);
    for &(vi, vpos) in &[(int_0, pos0), (int_1, pos1), (int_2, pos2)] {
        let acc = b.id();
        emit(
            &mut b.body,
            OP_ACCESS_CHAIN,
            &[ptr_out_v4f, acc, verts, vi, int_0],
        );
        emit(&mut b.body, OP_STORE, &[acc, vpos]);
    }
    let acc_idx = b.id();
    emit(
        &mut b.body,
        OP_ACCESS_CHAIN,
        &[ptr_out_v3u, acc_idx, prims, int_0],
    );
    emit(&mut b.body, OP_STORE, &[acc_idx, tri]);
    b.finish(EXEC_MODEL_MESH_EXT, false)
}

/// task 阶段最小合规 SPIR-V(§4.E5:TaskEXT + LocalSize + TaskPayloadWorkgroupEXT payload 写 +
/// OpEmitMeshTasksEXT〔终结子〕)。
pub fn emit_task_min() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_MESH_SHADING_EXT], vec![EXT_MESH_SHADER]);
    let uint = b.type_result(OP_TYPE_INT, &[32, 0]);
    let int = b.type_result(OP_TYPE_INT, &[32, 1]);
    let uint_1 = b.constant(uint, 1);
    let int_0 = b.constant(int, 0);
    // task payload = struct { uint v }(TaskPayloadWorkgroupEXT)。
    let payload_struct = b.type_result(OP_TYPE_STRUCT, &[uint]);
    let ptr_tp_struct = b.type_result(
        OP_TYPE_POINTER,
        &[STORAGE_TASK_PAYLOAD_WORKGROUP_EXT, payload_struct],
    );
    let payload = b.global_var(ptr_tp_struct, STORAGE_TASK_PAYLOAD_WORKGROUP_EXT, true);
    let ptr_tp_uint = b.type_result(OP_TYPE_POINTER, &[STORAGE_TASK_PAYLOAD_WORKGROUP_EXT, uint]);
    b.exec_mode(EXEC_MODE_LOCAL_SIZE, &[1, 1, 1]);
    // payload.v = 1u; EmitMeshTasksEXT(1,1,1)(终结子,无 OpReturn)。
    let acc = b.id();
    emit(
        &mut b.body,
        OP_ACCESS_CHAIN,
        &[ptr_tp_uint, acc, payload, int_0],
    );
    emit(&mut b.body, OP_STORE, &[acc, uint_1]);
    emit(
        &mut b.body,
        OP_EMIT_MESH_TASKS_EXT,
        &[uint_1, uint_1, uint_1],
    );
    b.finish(EXEC_MODEL_TASK_EXT, true)
}

/// raygen 阶段最小合规 SPIR-V(§4.E6/E8:RayGenerationKHR、SPV_KHR_ray_tracing、AccelStruct SRV、
/// RayPayloadKHR、OpTraceRayKHR〔已知签名固定:opaque/0xFF/SBT 恒 0/递归恒 1〕、storage image
/// 写出〔RXS-0247〕)。
///
/// **device 可判据落点**(RXS-0247):此前 raygen 无 storage image 落点 → device 回读全 clear
/// 黑、`bin/vk_rt` 中心/角落像素判据无法判(MISS)。本形态补齐两半:
/// 1. **per-pixel 光线原点** = `(gl_LaunchIDEXT.xy / gl_LaunchSizeEXT.xy) * 2 - 1`(→ NDC 式
///    `[-1,1]` 平面坐标,z=-1、方向 +z)——每像素独立命中判定,使「中心命中 / 角落 miss」及
///    「移动顶点 → 命中区移动」空间判据有信号(定原点恒定则全像素同判、判据无法辨);
/// 2. **`OpImageWrite`**:TraceRay 返回后把 payload(命中色 / miss 色)写落 UAV storage image
///    的 `ivec2(launchid.xy)` 纹素。
///
/// storage image = `OpTypeImage f32 2D 0 0 0 Sampled=2 Rgba8`,UAV 轴 **set1/binding0** = vk.rs
/// `run_rt_inner` 的 `set_layouts[1]=dsl_img`(TLAS SRV = set0/binding0 = `set_layouts[0]`);
/// format `Rgba8` 对齐 `VK_FORMAT_R8G8B8A8_UNORM`。精确原点映射 / 命中·miss 阈值 = owner device
/// 校准(`bin/vk_rt`,步骤 67)。
pub fn emit_raygen_min() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    let uint = b.type_result(OP_TYPE_INT, &[32, 0]);
    let int = b.type_result(OP_TYPE_INT, &[32, 1]);
    let float = b.type_result(OP_TYPE_FLOAT, &[32]);
    let v2uint = b.type_result(OP_TYPE_VECTOR, &[uint, 2]);
    let v3uint = b.type_result(OP_TYPE_VECTOR, &[uint, 3]);
    let v2int = b.type_result(OP_TYPE_VECTOR, &[int, 2]);
    let v2float = b.type_result(OP_TYPE_VECTOR, &[float, 2]);
    let v3float = b.type_result(OP_TYPE_VECTOR, &[float, 3]);
    let v4float = b.type_result(OP_TYPE_VECTOR, &[float, 4]);
    let uint_0 = b.constant(uint, 0);
    let ray_flags = b.constant(uint, RAY_FLAG_OPAQUE);
    let cull_mask = b.constant(uint, CULL_MASK_ALL);
    let float_0 = b.constant(float, 0.0f32.to_bits());
    let float_1 = b.constant(float, 1.0f32.to_bits());
    let float_2 = b.constant(float, 2.0f32.to_bits());
    let float_n1 = b.constant(float, (-1.0f32).to_bits());
    let float_100 = b.constant(float, 100.0f32.to_bits());
    let two_v2 = b.const_composite(v2float, &[float_2, float_2]);
    let one_v2 = b.const_composite(v2float, &[float_1, float_1]);
    let dir = b.const_composite(v3float, &[float_0, float_0, float_1]);
    let zero_v4 = b.const_composite(v4float, &[float_0, float_0, float_0, float_0]);
    // gl_LaunchIDEXT / gl_LaunchSizeEXT(Input v3uint;BuiltIn LaunchIdKHR / LaunchSizeKHR)。
    let ptr_in_v3uint = b.type_result(OP_TYPE_POINTER, &[STORAGE_INPUT, v3uint]);
    let launch_id = b.global_var(ptr_in_v3uint, STORAGE_INPUT, true);
    b.decorate(launch_id, DECORATION_BUILTIN, &[BUILTIN_LAUNCH_ID_KHR]);
    let launch_size = b.global_var(ptr_in_v3uint, STORAGE_INPUT, true);
    b.decorate(launch_size, DECORATION_BUILTIN, &[BUILTIN_LAUNCH_SIZE_KHR]);
    // AccelStruct SRV(UniformConstant;set0/binding0,承 RXS-0163 SRV 轴 = run_rt_inner
    // set_layouts[0]=dsl_tlas)。
    let accel_ty = b.type_result(OP_TYPE_ACCELERATION_STRUCTURE_KHR, &[]);
    let ptr_uc_accel = b.type_result(OP_TYPE_POINTER, &[STORAGE_UNIFORM_CONSTANT, accel_ty]);
    let tlas = b.global_var(ptr_uc_accel, STORAGE_UNIFORM_CONSTANT, true);
    b.decorate(tlas, DECORATION_DESCRIPTOR_SET, &[0]);
    b.decorate(tlas, DECORATION_BINDING, &[0]);
    // storage image UAV(OpTypeImage f32 2D 0 0 0 Sampled=2 Rgba8;set1/binding0 = run_rt_inner
    // set_layouts[1]=dsl_img)。
    let image_ty = b.type_result(
        OP_TYPE_IMAGE,
        &[float, DIM_2D, 0, 0, 0, 2, IMAGE_FORMAT_RGBA8],
    );
    let ptr_uc_image = b.type_result(OP_TYPE_POINTER, &[STORAGE_UNIFORM_CONSTANT, image_ty]);
    let out_image = b.global_var(ptr_uc_image, STORAGE_UNIFORM_CONSTANT, true);
    b.decorate(out_image, DECORATION_DESCRIPTOR_SET, &[1]);
    b.decorate(out_image, DECORATION_BINDING, &[0]);
    // ray payload(RayPayloadKHR vec4)。
    let ptr_payload = b.type_result(OP_TYPE_POINTER, &[STORAGE_RAY_PAYLOAD_KHR, v4float]);
    let payload = b.global_var(ptr_payload, STORAGE_RAY_PAYLOAD_KHR, true);

    // ── 函数体 ──
    // uv = launchid.xy / launchsize.xy ∈ [0,1);origin.xy = uv*2-1 ∈ [-1,1)(三角形居中平面)。
    let li = b.id();
    emit(&mut b.body, OP_LOAD, &[v3uint, li, launch_id]);
    let li_xy = b.id();
    emit(
        &mut b.body,
        OP_VECTOR_SHUFFLE,
        &[v2uint, li_xy, li, li, 0, 1],
    );
    let li_f = b.id();
    emit(&mut b.body, OP_CONVERT_U_TO_F, &[v2float, li_f, li_xy]);
    let ls = b.id();
    emit(&mut b.body, OP_LOAD, &[v3uint, ls, launch_size]);
    let ls_xy = b.id();
    emit(
        &mut b.body,
        OP_VECTOR_SHUFFLE,
        &[v2uint, ls_xy, ls, ls, 0, 1],
    );
    let ls_f = b.id();
    emit(&mut b.body, OP_CONVERT_U_TO_F, &[v2float, ls_f, ls_xy]);
    let uv = b.id();
    emit(&mut b.body, OP_FDIV, &[v2float, uv, li_f, ls_f]);
    let scaled = b.id();
    emit(&mut b.body, OP_FMUL, &[v2float, scaled, uv, two_v2]);
    let centered = b.id();
    emit(&mut b.body, OP_FSUB, &[v2float, centered, scaled, one_v2]);
    let ox = b.id();
    emit(&mut b.body, OP_COMPOSITE_EXTRACT, &[float, ox, centered, 0]);
    let oy = b.id();
    emit(&mut b.body, OP_COMPOSITE_EXTRACT, &[float, oy, centered, 1]);
    let origin = b.id();
    emit(
        &mut b.body,
        OP_COMPOSITE_CONSTRUCT,
        &[v3float, origin, ox, oy, float_n1],
    );
    // payload = 0;acc = load tlas;TraceRay(payload 落 hit/miss 色)。
    emit(&mut b.body, OP_STORE, &[payload, zero_v4]);
    let acc = b.id();
    emit(&mut b.body, OP_LOAD, &[accel_ty, acc, tlas]);
    emit(
        &mut b.body,
        OP_TRACE_RAY_KHR,
        &[
            acc, ray_flags, cull_mask, uint_0, uint_0, uint_0, origin, float_0, dir, float_100,
            payload,
        ],
    );
    // imageStore(out_image, ivec2(launchid.xy), payload):命中/miss 色写落 storage image。
    let pv = b.id();
    emit(&mut b.body, OP_LOAD, &[v4float, pv, payload]);
    let img = b.id();
    emit(&mut b.body, OP_LOAD, &[image_ty, img, out_image]);
    let coord = b.id();
    emit(&mut b.body, OP_BITCAST, &[v2int, coord, li_xy]);
    emit(&mut b.body, OP_IMAGE_WRITE, &[img, coord, pv]);
    b.finish(EXEC_MODEL_RAY_GENERATION_KHR, false)
}

/// miss 阶段最小合规 SPIR-V(§4.E6:MissKHR + IncomingRayPayloadKHR 写 miss 色)。
///
/// miss 色 = **黑 (0,0,0,1)**(RXS-0247):device 端 raygen 把本 payload 写落 storage image →
/// 未命中像素 `(0,0,0,255)`,`bin/vk_rt::expect_miss`(`r,g,b ≤ 8` 近黑)判 miss。取黑而非蓝
/// 是为与既有 `expect_miss` 近黑判据同相(vk_rt 判据阈值 = owner device 域,本片不动);全部
/// launch 像素均被 raygen 写落(命中→红 / miss→黑),无未写「clear」像素与黑 miss 混淆之虞。
pub fn emit_miss_min() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    let float = b.type_result(OP_TYPE_FLOAT, &[32]);
    let v4float = b.type_result(OP_TYPE_VECTOR, &[float, 4]);
    let float_0 = b.constant(float, 0.0f32.to_bits());
    let float_1 = b.constant(float, 1.0f32.to_bits());
    let miss_color = b.const_composite(v4float, &[float_0, float_0, float_0, float_1]);
    let ptr_payload = b.type_result(
        OP_TYPE_POINTER,
        &[STORAGE_INCOMING_RAY_PAYLOAD_KHR, v4float],
    );
    let payload = b.global_var(ptr_payload, STORAGE_INCOMING_RAY_PAYLOAD_KHR, true);
    emit(&mut b.body, OP_STORE, &[payload, miss_color]);
    b.finish(EXEC_MODEL_MISS_KHR, false)
}

/// closesthit 阶段最小合规 SPIR-V(§4.E6:ClosestHitKHR + IncomingRayPayloadKHR 写命中色)。
///
/// 命中色 = **红 (1,0,0,1)**(RXS-0247):device 端 raygen 把本 payload 写落 storage image →
/// 命中像素 `(255,0,0,255)`,`bin/vk_rt::expect_hit`(`r>8`)判命中。固定高对比色规避重心坐标
/// 近顶点时 r/g→0 的假 miss(此前 payload=vec4(bary.x,bary.y,0,1) 在近顶点处退化为近黑)。
pub fn emit_closesthit_min() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    let float = b.type_result(OP_TYPE_FLOAT, &[32]);
    let v4float = b.type_result(OP_TYPE_VECTOR, &[float, 4]);
    let float_0 = b.constant(float, 0.0f32.to_bits());
    let float_1 = b.constant(float, 1.0f32.to_bits());
    let hit_color = b.const_composite(v4float, &[float_1, float_0, float_0, float_1]);
    let ptr_payload = b.type_result(
        OP_TYPE_POINTER,
        &[STORAGE_INCOMING_RAY_PAYLOAD_KHR, v4float],
    );
    let payload = b.global_var(ptr_payload, STORAGE_INCOMING_RAY_PAYLOAD_KHR, true);
    emit(&mut b.body, OP_STORE, &[payload, hit_color]);
    b.finish(EXEC_MODEL_CLOSEST_HIT_KHR, false)
}

/// anyhit 阶段最小合规 SPIR-V(§4.E6:AnyHitKHR + IncomingRayPayloadKHR 写)。
pub fn emit_anyhit_min() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    let float = b.type_result(OP_TYPE_FLOAT, &[32]);
    let v4float = b.type_result(OP_TYPE_VECTOR, &[float, 4]);
    let float_0 = b.constant(float, 0.0f32.to_bits());
    let float_1 = b.constant(float, 1.0f32.to_bits());
    let color = b.const_composite(v4float, &[float_1, float_0, float_0, float_1]);
    let ptr_payload = b.type_result(
        OP_TYPE_POINTER,
        &[STORAGE_INCOMING_RAY_PAYLOAD_KHR, v4float],
    );
    let payload = b.global_var(ptr_payload, STORAGE_INCOMING_RAY_PAYLOAD_KHR, true);
    emit(&mut b.body, OP_STORE, &[payload, color]);
    b.finish(EXEC_MODEL_ANY_HIT_KHR, false)
}

/// intersection 阶段最小合规 SPIR-V(§4.E6:IntersectionKHR + HitAttributeKHR 写 +
/// OpReportIntersectionKHR〔hit-t / hit-kind → bool〕)。首期 accept-only(§8)。
pub fn emit_intersection_min() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    let bool_ty = b.type_result(OP_TYPE_BOOL, &[]);
    let uint = b.type_result(OP_TYPE_INT, &[32, 0]);
    let float = b.type_result(OP_TYPE_FLOAT, &[32]);
    let v2float = b.type_result(OP_TYPE_VECTOR, &[float, 2]);
    let uint_0 = b.constant(uint, 0);
    let float_half = b.constant(float, 0.5f32.to_bits());
    let float_1 = b.constant(float, 1.0f32.to_bits());
    let attr = b.const_composite(v2float, &[float_half, float_half]);
    let ptr_attr = b.type_result(OP_TYPE_POINTER, &[STORAGE_HIT_ATTRIBUTE_KHR, v2float]);
    let bary = b.global_var(ptr_attr, STORAGE_HIT_ATTRIBUTE_KHR, true);
    emit(&mut b.body, OP_STORE, &[bary, attr]);
    let reported = b.id();
    emit(
        &mut b.body,
        OP_REPORT_INTERSECTION_KHR,
        &[bool_ty, reported, float_1, uint_0],
    );
    b.finish(EXEC_MODEL_INTERSECTION_KHR, false)
}

/// callable 阶段最小合规 SPIR-V(§4.E6:CallableKHR + IncomingCallableDataKHR 写)。
/// 首期 accept-only(§8)。
pub fn emit_callable_min() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    let float = b.type_result(OP_TYPE_FLOAT, &[32]);
    let v4float = b.type_result(OP_TYPE_VECTOR, &[float, 4]);
    let float_1 = b.constant(float, 1.0f32.to_bits());
    let data_val = b.const_composite(v4float, &[float_1, float_1, float_1, float_1]);
    let ptr_data = b.type_result(
        OP_TYPE_POINTER,
        &[STORAGE_INCOMING_CALLABLE_DATA_KHR, v4float],
    );
    let data = b.global_var(ptr_data, STORAGE_INCOMING_CALLABLE_DATA_KHR, true);
    emit(&mut b.body, OP_STORE, &[data, data_val]);
    b.finish(EXEC_MODEL_CALLABLE_KHR, false)
}

/// mesh/task/RT 六执行模型的库级见证集(阶段名 → 发射器)。device 端 mesh/raygen/miss/
/// closesthit 三件套见证归主循环(vk 运行时);intersection/anyhit/callable 首期 accept-only
/// (§8;类型面 + spirv-val 全量,device 端到端见证 defer RD-034)。所有产物过 spirv-val
/// `--target-env vulkan1.2` / `spv1.4`(见 tests/mesh_rt_vulkan_spirv_val.rs)。

// ═══════════════════════ G8.2 M50 RT 增量 SPIR-V(RXS-0325;非 emit_*_min) ═══════════════════════
const STORAGE_CALLABLE_DATA_KHR: u32 = 5328;
const STORAGE_SHADER_RECORD_BUFFER_KHR: u32 = 5343;
const OP_IGNORE_INTERSECTION_KHR: u16 = 5335;
const OP_EXECUTE_CALLABLE_KHR: u16 = 4446;

/// M50 增量 raygen:ExecuteCallable(0)+TraceRay+ImageWrite(RXS-0325)。
//@ spec: RXS-0325
pub fn emit_m50_raygen() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    let uint = b.type_result(OP_TYPE_INT, &[32, 0]);
    let int = b.type_result(OP_TYPE_INT, &[32, 1]);
    let float = b.type_result(OP_TYPE_FLOAT, &[32]);
    let v2uint = b.type_result(OP_TYPE_VECTOR, &[uint, 2]);
    let v3uint = b.type_result(OP_TYPE_VECTOR, &[uint, 3]);
    let v2int = b.type_result(OP_TYPE_VECTOR, &[int, 2]);
    let v2float = b.type_result(OP_TYPE_VECTOR, &[float, 2]);
    let v3float = b.type_result(OP_TYPE_VECTOR, &[float, 3]);
    let v4float = b.type_result(OP_TYPE_VECTOR, &[float, 4]);
    let uint_0 = b.constant(uint, 0);
    let ray_flags = b.constant(uint, RAY_FLAG_OPAQUE);
    let cull_mask = b.constant(uint, CULL_MASK_ALL);
    let float_0 = b.constant(float, 0.0f32.to_bits());
    let float_1 = b.constant(float, 1.0f32.to_bits());
    let float_2 = b.constant(float, 2.0f32.to_bits());
    let float_n1 = b.constant(float, (-1.0f32).to_bits());
    let float_100 = b.constant(float, 100.0f32.to_bits());
    let two_v2 = b.const_composite(v2float, &[float_2, float_2]);
    let one_v2 = b.const_composite(v2float, &[float_1, float_1]);
    let dir = b.const_composite(v3float, &[float_0, float_0, float_1]);
    let zero_v4 = b.const_composite(v4float, &[float_0, float_0, float_0, float_0]);
    let ptr_in_v3uint = b.type_result(OP_TYPE_POINTER, &[STORAGE_INPUT, v3uint]);
    let launch_id = b.global_var(ptr_in_v3uint, STORAGE_INPUT, true);
    b.decorate(launch_id, DECORATION_BUILTIN, &[BUILTIN_LAUNCH_ID_KHR]);
    let launch_size = b.global_var(ptr_in_v3uint, STORAGE_INPUT, true);
    b.decorate(launch_size, DECORATION_BUILTIN, &[BUILTIN_LAUNCH_SIZE_KHR]);
    let accel_ty = b.type_result(OP_TYPE_ACCELERATION_STRUCTURE_KHR, &[]);
    let ptr_uc_accel = b.type_result(OP_TYPE_POINTER, &[STORAGE_UNIFORM_CONSTANT, accel_ty]);
    let tlas = b.global_var(ptr_uc_accel, STORAGE_UNIFORM_CONSTANT, true);
    b.decorate(tlas, DECORATION_DESCRIPTOR_SET, &[0]);
    b.decorate(tlas, DECORATION_BINDING, &[0]);
    let image_ty =
        b.type_result(OP_TYPE_IMAGE, &[float, DIM_2D, 0, 0, 0, 2, IMAGE_FORMAT_RGBA8]);
    let ptr_uc_image = b.type_result(OP_TYPE_POINTER, &[STORAGE_UNIFORM_CONSTANT, image_ty]);
    let out_image = b.global_var(ptr_uc_image, STORAGE_UNIFORM_CONSTANT, true);
    b.decorate(out_image, DECORATION_DESCRIPTOR_SET, &[1]);
    b.decorate(out_image, DECORATION_BINDING, &[0]);
    let ptr_payload = b.type_result(OP_TYPE_POINTER, &[STORAGE_RAY_PAYLOAD_KHR, v4float]);
    let payload = b.global_var(ptr_payload, STORAGE_RAY_PAYLOAD_KHR, true);
    let ptr_call = b.type_result(OP_TYPE_POINTER, &[STORAGE_CALLABLE_DATA_KHR, uint]);
    let call_data = b.global_var(ptr_call, STORAGE_CALLABLE_DATA_KHR, true);

    let li = b.id();
    emit(&mut b.body, OP_LOAD, &[v3uint, li, launch_id]);
    let li_xy = b.id();
    emit(&mut b.body, OP_VECTOR_SHUFFLE, &[v2uint, li_xy, li, li, 0, 1]);
    let li_f = b.id();
    emit(&mut b.body, OP_CONVERT_U_TO_F, &[v2float, li_f, li_xy]);
    let ls = b.id();
    emit(&mut b.body, OP_LOAD, &[v3uint, ls, launch_size]);
    let ls_xy = b.id();
    emit(&mut b.body, OP_VECTOR_SHUFFLE, &[v2uint, ls_xy, ls, ls, 0, 1]);
    let ls_f = b.id();
    emit(&mut b.body, OP_CONVERT_U_TO_F, &[v2float, ls_f, ls_xy]);
    let uv = b.id();
    emit(&mut b.body, OP_FDIV, &[v2float, uv, li_f, ls_f]);
    let scaled = b.id();
    emit(&mut b.body, OP_FMUL, &[v2float, scaled, uv, two_v2]);
    let centered = b.id();
    emit(&mut b.body, OP_FSUB, &[v2float, centered, scaled, one_v2]);
    let ox = b.id();
    emit(&mut b.body, OP_COMPOSITE_EXTRACT, &[float, ox, centered, 0]);
    let oy = b.id();
    emit(&mut b.body, OP_COMPOSITE_EXTRACT, &[float, oy, centered, 1]);
    let origin = b.id();
    emit(
        &mut b.body,
        OP_COMPOSITE_CONSTRUCT,
        &[v3float, origin, ox, oy, float_n1],
    );
    emit(&mut b.body, OP_STORE, &[call_data, uint_0]);
    emit(&mut b.body, OP_EXECUTE_CALLABLE_KHR, &[uint_0, call_data]);
    emit(&mut b.body, OP_STORE, &[payload, zero_v4]);
    let acc = b.id();
    emit(&mut b.body, OP_LOAD, &[accel_ty, acc, tlas]);
    emit(
        &mut b.body,
        OP_TRACE_RAY_KHR,
        &[
            acc, ray_flags, cull_mask, uint_0, uint_0, uint_0, origin, float_0, dir, float_100,
            payload,
        ],
    );
    let pv = b.id();
    emit(&mut b.body, OP_LOAD, &[v4float, pv, payload]);
    let img = b.id();
    emit(&mut b.body, OP_LOAD, &[image_ty, img, out_image]);
    let coord = b.id();
    emit(&mut b.body, OP_BITCAST, &[v2int, coord, li_xy]);
    emit(&mut b.body, OP_IMAGE_WRITE, &[img, coord, pv]);
    b.finish(EXEC_MODEL_RAY_GENERATION_KHR, false)
}

//@ spec: RXS-0325
pub fn emit_m50_miss() -> Vec<u32> {
    emit_miss_min()
}

/// M50 closesthit:ShaderRecordBufferKHR `{u32 id; f32 r,g,b}` → IncomingRayPayload。
//@ spec: RXS-0325
pub fn emit_m50_closesthit() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    let uint = b.type_result(OP_TYPE_INT, &[32, 0]);
    let float = b.type_result(OP_TYPE_FLOAT, &[32]);
    let v4float = b.type_result(OP_TYPE_VECTOR, &[float, 4]);
    let rec_ty = b.type_result(OP_TYPE_STRUCT, &[uint, float, float, float]);
    b.decorate(rec_ty, DECORATION_BLOCK, &[]);
    b.member_decorate(rec_ty, 0, DECORATION_OFFSET, &[0]);
    b.member_decorate(rec_ty, 1, DECORATION_OFFSET, &[4]);
    b.member_decorate(rec_ty, 2, DECORATION_OFFSET, &[8]);
    b.member_decorate(rec_ty, 3, DECORATION_OFFSET, &[12]);
    let ptr_rec =
        b.type_result(OP_TYPE_POINTER, &[STORAGE_SHADER_RECORD_BUFFER_KHR, rec_ty]);
    let rec = b.global_var(ptr_rec, STORAGE_SHADER_RECORD_BUFFER_KHR, true);
    let ptr_payload =
        b.type_result(OP_TYPE_POINTER, &[STORAGE_INCOMING_RAY_PAYLOAD_KHR, v4float]);
    let payload = b.global_var(ptr_payload, STORAGE_INCOMING_RAY_PAYLOAD_KHR, true);
    let ptr_u = b.type_result(OP_TYPE_POINTER, &[STORAGE_SHADER_RECORD_BUFFER_KHR, uint]);
    let ptr_f = b.type_result(OP_TYPE_POINTER, &[STORAGE_SHADER_RECORD_BUFFER_KHR, float]);
    let c0 = b.constant(uint, 0);
    let c1 = b.constant(uint, 1);
    let c2 = b.constant(uint, 2);
    let c3 = b.constant(uint, 3);
    let a_id = b.id();
    emit(&mut b.body, OP_ACCESS_CHAIN, &[ptr_u, a_id, rec, c0]);
    let mid = b.id();
    emit(&mut b.body, OP_LOAD, &[uint, mid, a_id]);
    let mid_f = b.id();
    emit(&mut b.body, OP_CONVERT_U_TO_F, &[float, mid_f, mid]);
    let ar = b.id();
    emit(&mut b.body, OP_ACCESS_CHAIN, &[ptr_f, ar, rec, c1]);
    let rv = b.id();
    emit(&mut b.body, OP_LOAD, &[float, rv, ar]);
    let ag = b.id();
    emit(&mut b.body, OP_ACCESS_CHAIN, &[ptr_f, ag, rec, c2]);
    let gv = b.id();
    emit(&mut b.body, OP_LOAD, &[float, gv, ag]);
    let ab = b.id();
    emit(&mut b.body, OP_ACCESS_CHAIN, &[ptr_f, ab, rec, c3]);
    let bv = b.id();
    emit(&mut b.body, OP_LOAD, &[float, bv, ab]);
    let color = b.id();
    emit(
        &mut b.body,
        OP_COMPOSITE_CONSTRUCT,
        &[v4float, color, rv, gv, bv, mid_f],
    );
    emit(&mut b.body, OP_STORE, &[payload, color]);
    b.finish(EXEC_MODEL_CLOSEST_HIT_KHR, false)
}

/// M50 anyhit:record.u32 ignore!=0 → OpIgnoreIntersectionKHR。
//@ spec: RXS-0325
pub fn emit_m50_anyhit() -> Vec<u32> {
    // 首期简化:无条件 Ignore(masked 组专用模块);keep 组不挂 anyhit。
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    emit(&mut b.body, OP_IGNORE_INTERSECTION_KHR, &[]);
    b.finish(EXEC_MODEL_ANY_HIT_KHR, true)
}

//@ spec: RXS-0325
pub fn emit_m50_intersection() -> Vec<u32> {
    emit_intersection_min()
}

//@ spec: RXS-0325
pub fn emit_m50_callable() -> Vec<u32> {
    let mut b = ExtBuilder::new(vec![CAP_RAY_TRACING_KHR], vec![EXT_RAY_TRACING]);
    let uint = b.type_result(OP_TYPE_INT, &[32, 0]);
    let v42 = b.constant(uint, 42);
    let ptr =
        b.type_result(OP_TYPE_POINTER, &[STORAGE_INCOMING_CALLABLE_DATA_KHR, uint]);
    let data = b.global_var(ptr, STORAGE_INCOMING_CALLABLE_DATA_KHR, true);
    emit(&mut b.body, OP_STORE, &[data, v42]);
    b.finish(EXEC_MODEL_CALLABLE_KHR, false)
}

/// M50 增量语料(非 emit_*_min;供 rurix-rt build.rs 嵌入)。
//@ spec: RXS-0325
pub fn m50_incremental_corpus() -> Vec<(&'static str, Vec<u32>)> {
    vec![
        ("m50_raygen", emit_m50_raygen()),
        ("m50_miss", emit_m50_miss()),
        ("m50_closesthit", emit_m50_closesthit()),
        ("m50_anyhit", emit_m50_anyhit()),
        ("m50_intersection", emit_m50_intersection()),
        ("m50_callable", emit_m50_callable()),
    ]
}

pub fn mesh_rt_witness_corpus() -> Vec<(&'static str, Vec<u32>)> {
    vec![
        ("mesh", emit_mesh_min()),
        ("task", emit_task_min()),
        ("raygen", emit_raygen_min()),
        ("miss", emit_miss_min()),
        ("closesthit", emit_closesthit_min()),
        ("anyhit", emit_anyhit_min()),
        ("intersection", emit_intersection_min()),
        ("callable", emit_callable_min()),
    ]
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

    /// per-entry 版本轴(RFC-0013 §4.E6):mesh/task/RT 六执行模型入口恒 emit 1.4 +
    /// interface 全量;既有 compute [`assemble`] 恒 1.0(SPIRV_VERSION_1_0 常量不变 =
    /// 字节零漂移锚点,跨两发射器零回归门)。
    //@ spec: RXS-0247
    #[test]
    fn mesh_rt_entries_emit_1_4_and_full_interface() {
        for (name, words) in mesh_rt_witness_corpus() {
            assert_eq!(words[1], SPIRV_VERSION_1_4, "{name} 入口须 emit SPIR-V 1.4");
        }
        // compute 路版本常量维持 1.0(assemble 恒引用,既有 GLCompute golden 字节不动)。
        assert_eq!(SPIRV_VERSION_1_0, 0x0001_0000);
        assert_ne!(SPIRV_VERSION_1_0, SPIRV_VERSION_1_4);
    }

    /// mesh 入口 = MeshEXT 执行模型 + SPV_EXT_mesh_shader + OutputTrianglesEXT(§4.E5)。
    /// RXS-0275 mesh MIR→SPIR-V lowering(lower_mesh)产出的执行模型/extension/mode 结构同此 golden。
    //@ spec: RXS-0246
    //@ spec: RXS-0275
    #[test]
    fn mesh_entry_point_is_mesh_ext_model() {
        let words = emit_mesh_min();
        // 定位 OpEntryPoint(op=15):首操作数 = 执行模型。
        let mut i = 5;
        let mut found = None;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            let op = (words[i] & 0xffff) as u16;
            if wc == 0 {
                break;
            }
            if op == OP_ENTRY_POINT {
                found = Some(words[i + 1]);
                break;
            }
            i += wc;
        }
        assert_eq!(
            found,
            Some(EXEC_MODEL_MESH_EXT),
            "mesh 入口执行模型须 MeshEXT"
        );
    }

    /// raygen 入口 = RayGenerationKHR 执行模型(§4.E6)。
    //@ spec: RXS-0247
    #[test]
    fn raygen_entry_point_is_ray_generation_khr() {
        let words = emit_raygen_min();
        let mut i = 5;
        let mut model = None;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            let op = (words[i] & 0xffff) as u16;
            if wc == 0 {
                break;
            }
            if op == OP_ENTRY_POINT {
                model = Some(words[i + 1]);
                break;
            }
            i += wc;
        }
        assert_eq!(model, Some(EXEC_MODEL_RAY_GENERATION_KHR));
    }

    // ───────── G7.2 W3a:compute RayQuery 编码锚定(RXS-0297~0300) ─────────

    /// `.rx` 源 → compute SPIR-V 字流(真实前端管线,非手编见证)。
    fn compile_compute(src: &str) -> Vec<u32> {
        let diag = crate::diag::DiagCtxt::new();
        let cx = crate::query::QueryCtx::new(
            src,
            crate::span::SourceId(0),
            crate::span::Edition::Rx0,
            &diag,
        );
        cx.check_shader_stages();
        cx.check_crate();
        cx.check_coloring();
        assert!(
            !diag.has_errors(),
            "语料应无前端诊断: {:?}",
            diag.emitted().iter().map(|d| d.code).collect::<Vec<_>>()
        );
        let bodies = cx.device_mir_crate();
        let res = cx.resolutions();
        let entry = bodies
            .iter()
            .find(|b| b.color == FnColor::Kernel && b.stage.is_none())
            .expect("须有 compute kernel 根");
        lower_compute(entry, &res).expect("compute lowering 应成功")
    }

    /// 指令流扫描:收集全部 opcode(跳 5 字 header)。
    fn opcodes(words: &[u32]) -> Vec<u16> {
        let mut out = Vec::new();
        let mut i = 5;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            if wc == 0 {
                break;
            }
            out.push((words[i] & 0xffff) as u16);
            i += wc;
        }
        out
    }

    /// 取首条指定 opcode 的操作数切片。
    fn first_inst(words: &[u32], opcode: u16) -> Option<&[u32]> {
        let mut i = 5;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            if wc == 0 {
                break;
            }
            if (words[i] & 0xffff) as u16 == opcode {
                return Some(&words[i + 1..i + wc]);
            }
            i += wc;
        }
        None
    }

    /// 模块内 `OpExtension` 名集合。
    fn extensions(words: &[u32]) -> Vec<String> {
        let mut out = Vec::new();
        let mut i = 5;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            if wc == 0 {
                break;
            }
            if (words[i] & 0xffff) as u16 == OP_EXTENSION {
                let mut bytes = Vec::new();
                for w in &words[i + 1..i + wc] {
                    bytes.extend_from_slice(&w.to_le_bytes());
                }
                while bytes.last() == Some(&0) {
                    bytes.pop();
                }
                out.push(String::from_utf8_lossy(&bytes).into_owned());
            }
            i += wc;
        }
        out
    }

    /// RXS-0297~0300 全流程语料:compute 签名 `AccelStruct` + 体内 `RayQuery`
    /// (initialize → while proceed → if has_committed → committed 五查询 → terminate)。
    /// 与 `conformance/rayquery/accept/ray_query_basic.rx` 同形。
    const RQ_FULL: &str = "kernel fn rq(tlas: AccelStruct, t: ThreadCtx<1>) {\n\
         \x20   let mut rq = ray_query_initialize(tlas, (0.0, 0.0, 0.0), 0.0, (0.0, 0.0, 1.0), 100.0);\n\
         \x20   while rq.proceed() {\n\
         \x20       if rq.has_committed() {\n\
         \x20           let a = rq.committed_t();\n\
         \x20           let b = rq.committed_barycentric();\n\
         \x20           let c = rq.committed_instance_index();\n\
         \x20           let d = rq.committed_primitive_index();\n\
         \x20           let e = rq.committed_geometry_index();\n\
         \x20       }\n\
         \x20   }\n\
         \x20   rq.terminate();\n\
         }\n";

    /// per-entry 升版 + capability/extension 按需声明(RXS-0300 Legality 首两条)。
    //@ spec: RXS-0300
    #[test]
    fn ray_query_compute_emits_1_4_with_capability_and_extension() {
        let words = compile_compute(RQ_FULL);
        assert_eq!(
            words[1], SPIRV_VERSION_1_4,
            "含 RayQuery/AccelStruct 的 compute entry 须升 SPIR-V 1.4"
        );
        let caps: Vec<u32> = {
            let mut v = Vec::new();
            let mut i = 5;
            while i < words.len() {
                let wc = (words[i] >> 16) as usize;
                if wc == 0 {
                    break;
                }
                if (words[i] & 0xffff) as u16 == OP_CAPABILITY {
                    v.push(words[i + 1]);
                }
                i += wc;
            }
            v
        };
        assert!(
            caps.contains(&CAP_RAY_QUERY_KHR),
            "须声明 RayQueryKHR capability: {caps:?}"
        );
        assert!(
            caps.contains(&CAP_SHADER),
            "Shader capability 不得丢失: {caps:?}"
        );
        assert_eq!(
            extensions(&words),
            vec![EXT_SPV_KHR_RAY_QUERY.to_owned()],
            "须且仅须声明 SPV_KHR_ray_query"
        );
    }

    /// 反汇编 golden 最小集(G-G7-4 逐字):关键指令逐条锚定。
    //@ spec: RXS-0300
    #[test]
    fn ray_query_golden_anchors_key_instructions() {
        let words = compile_compute(RQ_FULL);
        let ops = opcodes(&words);
        for (opcode, name) in [
            (
                OP_TYPE_ACCELERATION_STRUCTURE_KHR,
                "OpTypeAccelerationStructureKHR",
            ),
            (OP_TYPE_RAY_QUERY_KHR, "OpTypeRayQueryKHR"),
            (OP_RAY_QUERY_INITIALIZE_KHR, "OpRayQueryInitializeKHR"),
            (OP_RAY_QUERY_PROCEED_KHR, "OpRayQueryProceedKHR"),
            (OP_RAY_QUERY_TERMINATE_KHR, "OpRayQueryTerminateKHR"),
            (
                OP_RAY_QUERY_GET_INTERSECTION_TYPE_KHR,
                "OpRayQueryGetIntersectionTypeKHR",
            ),
            (
                OP_RAY_QUERY_GET_INTERSECTION_T_KHR,
                "OpRayQueryGetIntersectionTKHR",
            ),
            (
                OP_RAY_QUERY_GET_INTERSECTION_BARYCENTRICS_KHR,
                "OpRayQueryGetIntersectionBarycentricsKHR",
            ),
            (
                OP_RAY_QUERY_GET_INTERSECTION_INSTANCE_ID_KHR,
                "OpRayQueryGetIntersectionInstanceIdKHR",
            ),
            (
                OP_RAY_QUERY_GET_INTERSECTION_PRIMITIVE_INDEX_KHR,
                "OpRayQueryGetIntersectionPrimitiveIndexKHR",
            ),
            (
                OP_RAY_QUERY_GET_INTERSECTION_GEOMETRY_INDEX_KHR,
                "OpRayQueryGetIntersectionGeometryIndexKHR",
            ),
            // `while rq.proceed()` 的结构化循环(RXS-0299 守卫形态 ③)。
            (OP_LOOP_MERGE, "OpLoopMerge"),
        ] {
            assert!(ops.contains(&opcode), "反汇编 golden 缺 {name}");
        }
    }

    /// `OpRayQueryInitializeKHR` 操作数序 + 冻结的 flags/mask(RXS-0298/0300)。
    /// 序 = RayQuery, Accel, RayFlags, CullMask, RayOrigin, RayTMin, RayDirection, RayTMax。
    //@ spec: RXS-0298
    #[test]
    fn ray_query_initialize_operand_shape_and_frozen_flags() {
        let words = compile_compute(RQ_FULL);
        let ops = first_inst(&words, OP_RAY_QUERY_INITIALIZE_KHR).expect("须有 initialize");
        assert_eq!(ops.len(), 8, "initialize 恰 8 个操作数: {ops:?}");
        // flags(idx 2)/ mask(idx 3)为 IdRef → 物化常量;经 OpConstant 表反查其值。
        let const_value = |id: u32| -> Option<u32> {
            let mut i = 5;
            while i < words.len() {
                let wc = (words[i] >> 16) as usize;
                if wc == 0 {
                    break;
                }
                if (words[i] & 0xffff) as u16 == OP_CONSTANT && words[i + 2] == id {
                    return Some(words[i + 3]);
                }
                i += wc;
            }
            None
        };
        assert_eq!(
            const_value(ops[2]),
            Some(RAY_FLAG_OPAQUE),
            "ray flags 首期恒 Opaque(RXS-0298)"
        );
        assert_eq!(
            const_value(ops[3]),
            Some(CULL_MASK_ALL),
            "cull mask 首期恒 0xFF(RXS-0298)"
        );
        // RayQuery 操作数(idx 0)= Function 存储类变量(RXS-0297 Function-only 收窄)。
        let rq_var = ops[0];
        let mut i = 5;
        let mut storage = None;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            if wc == 0 {
                break;
            }
            if (words[i] & 0xffff) as u16 == OP_VARIABLE && words[i + 2] == rq_var {
                storage = Some(words[i + 3]);
            }
            i += wc;
        }
        assert_eq!(
            storage,
            Some(STORAGE_FUNCTION),
            "RayQuery 变量须 Function 存储类"
        );
    }

    /// SPIR-V 1.4 `OpEntryPoint` interface **全量枚举**:AS descriptor(UniformConstant)
    /// 须在 interface 中(1.4 起不再限 Input/Output;与 mesh/RT 同律)。
    //@ spec: RXS-0300
    #[test]
    fn ray_query_entry_interface_enumerates_accel_descriptor() {
        let words = compile_compute(RQ_FULL);
        let ep = first_inst(&words, OP_ENTRY_POINT).expect("须有 OpEntryPoint");
        // ops: [exec_model, main_id, name words.., interface..]。名串以 NUL 结尾字对齐,
        // 逐字扫过名串后剩余即 interface。
        assert_eq!(ep[0], EXEC_MODEL_GLCOMPUTE);
        let mut k = 2;
        while k < ep.len() {
            let w = ep[k];
            k += 1;
            if w.to_le_bytes().contains(&0) {
                break;
            }
        }
        let interface = &ep[k..];
        // 取 AS descriptor 变量 id = OpTypeAccelerationStructureKHR 指针类型的
        // UniformConstant 变量。
        let accel_ty =
            first_inst(&words, OP_TYPE_ACCELERATION_STRUCTURE_KHR).expect("须有 AS 类型")[0];
        let mut accel_ptr = None;
        let mut i = 5;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            if wc == 0 {
                break;
            }
            if (words[i] & 0xffff) as u16 == OP_TYPE_POINTER && words[i + 3] == accel_ty {
                accel_ptr = Some(words[i + 1]);
            }
            i += wc;
        }
        let accel_ptr = accel_ptr.expect("须有 AS 指针类型");
        let mut accel_var = None;
        let mut i = 5;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            if wc == 0 {
                break;
            }
            if (words[i] & 0xffff) as u16 == OP_VARIABLE && words[i + 1] == accel_ptr {
                accel_var = Some(words[i + 2]);
            }
            i += wc;
        }
        let accel_var = accel_var.expect("须有 AS descriptor 变量");
        assert!(
            interface.contains(&accel_var),
            "1.4 interface 须全量枚举 AS descriptor: interface={interface:?} var={accel_var}"
        );
    }

    /// 向量 Function local 的**单层分量投影** → `OpAccessChain` 标量指针
    /// (G7.4 W3c 路 A 实现兑现;W3a 章 B 实现面回填,spec 面 0-byte,零新 RXS)。
    ///
    /// 判据:`committed_barycentric()` 的 vec2 结果经 `bary.0`/`bary.1` 真实写出 →
    /// 模块须含 `OpRayQueryGetIntersectionBarycentricsKHR` **且** ≥2 条以
    /// `Function` 存储类 `float` 指针为结果类型、**单索引**的 `OpAccessChain`
    /// (索引 = 常量 0 / 1)。此前该 vec2 分量不可读,RXS-0298 条款为死条款。
    //@ spec: RXS-0300
    #[test]
    fn ray_query_barycentric_components_lower_to_function_access_chain() {
        let src = "kernel fn rqb(tlas: AccelStruct, t: ThreadCtx<1>, out: ViewMut<global, f32>) {\n\
             \x20   let i = t.global_id();\n\
             \x20   let mut rq = ray_query_initialize(tlas, (0.0, 0.0, 0.0), 0.0, (0.0, 0.0, 1.0), 100.0);\n\
             \x20   while rq.proceed() {\n\
             \x20   }\n\
             \x20   if rq.has_committed() {\n\
             \x20       let bary = rq.committed_barycentric();\n\
             \x20       out[i * 2] = bary.0;\n\
             \x20       out[i * 2 + 1] = bary.1;\n\
             \x20   }\n\
             }\n";
        let words = compile_compute(src);
        assert!(
            opcodes(&words).contains(&OP_RAY_QUERY_GET_INTERSECTION_BARYCENTRICS_KHR),
            "须发射 OpRayQueryGetIntersectionBarycentricsKHR"
        );
        // f32 类型 id + Function 存储类 float 指针类型 id 集合。
        let mut f32_ty: Option<u32> = None;
        let mut fn_float_ptrs: Vec<u32> = Vec::new();
        let mut i = 5;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            if wc == 0 {
                break;
            }
            let op = (words[i] & 0xffff) as u16;
            if op == OP_TYPE_FLOAT && words[i + 2] == 32 {
                f32_ty = Some(words[i + 1]);
            }
            if op == OP_TYPE_POINTER
                && words[i + 2] == STORAGE_FUNCTION
                && Some(words[i + 3]) == f32_ty
            {
                fn_float_ptrs.push(words[i + 1]);
            }
            i += wc;
        }
        assert!(
            f32_ty.is_some() && !fn_float_ptrs.is_empty(),
            "须有 Function 存储类 f32 指针类型(向量分量指针)"
        );
        // 单索引 OpAccessChain(wc = 5:result-ty, result, base, index)且结果类型为
        // Function float 指针 —— 即向量分量指针。
        let mut comp_chains = 0usize;
        let mut i = 5;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            if wc == 0 {
                break;
            }
            if (words[i] & 0xffff) as u16 == OP_ACCESS_CHAIN
                && wc == 5
                && fn_float_ptrs.contains(&words[i + 1])
            {
                comp_chains += 1;
            }
            i += wc;
        }
        assert!(
            comp_chains >= 2,
            "两个分量各须一条向量分量 OpAccessChain,实得 {comp_chains}"
        );
    }

    /// 向量分量投影是**通用面**而非 RayQuery 特例:纯元组分量读写的 compute entry
    /// 维持 SPIR-V 1.0 + 零 ray query capability(与
    /// `conformance/vulkan/accept/vk_vec_component.rx` 同形;零漂移方向锚点)。
    //@ spec: RXS-0300
    #[test]
    fn vector_component_projection_without_ray_query_stays_1_0() {
        let src = "kernel fn vc(t: ThreadCtx<1>, x: View<global, f32>, out: ViewMut<global, f32>) {\n\
             \x20   let i = t.global_id();\n\
             \x20   let mut p = (x[i], x[i] * 2.0);\n\
             \x20   p.0 = p.0 + 1.0;\n\
             \x20   out[i * 2] = p.0;\n\
             \x20   out[i * 2 + 1] = p.1;\n\
             }\n";
        let words = compile_compute(src);
        assert_eq!(
            words[1], SPIRV_VERSION_1_0,
            "无 ray query 面的向量分量投影须维持 1.0"
        );
        assert!(
            extensions(&words).is_empty(),
            "向量分量投影不得引入 OpExtension"
        );
        assert!(
            opcodes(&words).contains(&OP_ACCESS_CHAIN),
            "分量投影须经 OpAccessChain"
        );
    }

    /// W1/W2 零漂移锚点(RXS-0300 Dynamic Semantics):**不含** RayQuery/AccelStruct 的
    /// compute entry 维持 1.0 emit 且**零新 capability**(走既有 `assemble` 原路)。
    //@ spec: RXS-0300
    #[test]
    fn compute_without_ray_query_stays_1_0_and_declares_no_ray_capability() {
        let src = "kernel fn k(out: ViewMut<global, f32>, t: ThreadCtx<1>) {\n\
                   \x20   let i = t.global_id();\n\
                   \x20   out[i] = 1.0;\n\
                   }\n";
        let words = compile_compute(src);
        assert_eq!(words[1], SPIRV_VERSION_1_0, "无 ray query 面须维持 1.0");
        let mut i = 5;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            if wc == 0 {
                break;
            }
            if (words[i] & 0xffff) as u16 == OP_CAPABILITY {
                assert_ne!(
                    words[i + 1],
                    CAP_RAY_QUERY_KHR,
                    "无 ray query 面不得声明 RayQueryKHR"
                );
            }
            i += wc;
        }
        assert!(
            extensions(&words).is_empty(),
            "无 ray query 面不得声明 OpExtension"
        );
    }

    /// `spirv-val` **双口径**(`--target-env vulkan1.2` 与 `spv1.4`)皆 accept
    /// (RXS-0300 校验轴;退出码判定,缺工具 → Skipped dev-env degrade 不充绿)。
    //@ spec: RXS-0300
    #[test]
    fn ray_query_module_passes_spirv_val_dual_target_env() {
        use crate::toolchain::{SpirvValGate, spirv_val_gate_env};
        let words = compile_compute(RQ_FULL);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("rurix_rxs0300_{nanos}.spv"));
        std::fs::write(&path, words_to_bytes(&words)).expect("写临时 .spv");
        let mut skipped = false;
        for env in ["vulkan1.2", "spv1.4"] {
            match spirv_val_gate_env(&path, Some(env)) {
                SpirvValGate::Accepted => {}
                SpirvValGate::Rejected(why) => {
                    let _ = std::fs::remove_file(&path);
                    panic!("spirv-val --target-env {env} 拒绝 compute RayQuery 模块: {why}");
                }
                // 缺 Vulkan SDK:dev-env degrade(RXS-0212 三态),不 fake pass。
                SpirvValGate::Skipped => skipped = true,
            }
        }
        let _ = std::fs::remove_file(&path);
        if skipped {
            eprintln!("spirv-val 不可用 → SKIP(dev-env degrade,非通过性证据)");
        }
    }
}
