//! `dxil_spirv` — 图形=B 后端的 MIR→SPIR-V 字流编码器(RFC-0004 §4.1;RXS-0161)。
//!
//! 本模块为 G2.2 PR-D2 分片 2 的最高风险点:把着色阶段(vertex/fragment)的
//! I/O 意图签名降级为**合法、spirv-val 干净**的 SPIR-V 二进制字流,作为 B 转译链
//! (SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL)的第二中间表示输入。
//!
//! 设计与约束(严格遵循 RFC-0004 §4.1/§4.6 与本规格 Requirement 1/6)。
//!
//! 纯 safe(R1.11):仅以 `Vec<u32>` 累积字流 + 单调递增 result-id 计数器,无任何
//! `unsafe` 块(crate `unsafe_code = "deny"`)。
//!
//! 最小子集(R1.4~R1.7):`Capability Shader`、`OpMemoryModel(Logical, GLSL450)`、
//! `OpEntryPoint(Vertex|Fragment)`、`OpExecutionMode(OriginUpperLeft)`(fragment)、
//! 按需类型指令(`OpTypeVoid`/`OpTypeFloat`/`OpTypeInt`/`OpTypeVector`/`OpTypePointer`/
//! `OpTypeFunction`)、Input/Output 变量、`Location`/`BuiltIn` 装饰、`UserSemantic`
//! 保名、以及平凡 passthrough `main`。
//!
//! by-construction 保名(R1.6):对每个有用户语义名的 I/O,emit
//! `OpDecorate <var> UserSemantic "<field_name>"`(经 `SPV_GOOGLE_hlsl_functionality1`
//! 扩展启用),使 SPIR-V→HLSL 段经反射端到端保名。
//!
//! strict-only(R1.9 / R6.1):最小子集外的构造(不可映射类型、未建模 builtin 名、
//! 非 vertex·fragment 阶段、越界向量宽度等)→ 返回 [`DxilError::Unmappable`],
//! 严禁静默产出降级 SPIR-V。
//!
//! 🔒 禁区(R1.10 / R6.3~R6.5):本编码器的输入 [`crate::mir::IoSigElem`] 仅可表达
//! 已建模标量/向量([`crate::mir::MirIoType`]),无法表达资源句柄/描述符/采样器,
//! 故纹理访问语义(描述符编码/采样 opcode/缓存/LOD/导数/越界)在本层结构上不可达;
//! 一旦未来类型面扩展触及,应在映射处停手发 [`DxilError::Unmappable`] 并标「需人工
//! 升档」,不在此发明 SPIR-V 纹理访问语义或 ABI 布局。
//!
//! 本任务不接 MIR codegen 主链(那是任务 4):对外只暴露 [`emit_spirv`],直接吃
//! `stage + &[IoSigElem]`(均为任务 1 已落地的公开类型),由 `#[cfg(test)]` 单测/
//! PBT 直接构造 I/O 元素喂编码器并以本机 spirv-val 独立验证(R1.8,Property 1)。

use crate::ast::{BinOp, ShaderStage};
use crate::binding_layout::{self, BindingInferError};
use crate::hir::PrimTy;
use crate::mir::{
    Body, Const, IoDir, IoSigElem, IoSigKind, LocalIdx, MirIoType, MirResourceType, Operand, Place,
    ProjElem, ResourceBinding, ResourceMethod, Rvalue, StatementKind, TerminatorKind,
};

use std::collections::HashMap;
use std::fmt;

// ───────────────────────── 错误类型 ─────────────────────────

/// 图形=B 编码器/降级面的错误(strict-only;变体→6xxx registry 落码是任务 4,
/// 本任务只定义枚举与携带诊断信息,不动 `registry/error_codes.json`)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DxilError {
    /// 最小子集外的不可映射构造(不可映射类型 / 未建模 builtin 名 / 非
    /// vertex·fragment 阶段 / 越界向量宽度 / builtin 类型不符等)。
    ///
    /// strict-only:遇此即失败,**不**静默产出降级 SPIR-V(任务7 映射 RX6013
    /// `codegen.dxil_unmappable`,经 `DxilBError::Spirv` 透传)。`what` 为不可映射构造
    /// 的分类,`detail` 为携带的诊断上下文。
    Unmappable {
        /// 不可映射构造的分类(供后续 6xxx 诊断与人工排查)。
        what: String,
        /// 诊断上下文(字段名 / 阶段 / 方向 / 类型等)。
        detail: String,
    },
    /// 纹理采样首期收敛子集外(RXS-0175;RFC-0007):隐式 LOD / 非 `Texture2D<f32>` /
    /// coord 非 `vec2<f32>` / texel fetch / 比较采样 / 多分量纹理等。
    ///
    /// strict-only:遇此即失败(任务映射 `RX6023` `codegen.dxil_sample_unsupported`,
    /// 经 `DxilBError::Spirv` 透传;区别于 `Unmappable` → RX6013 通用不可映射)。
    SampleUnsupported {
        /// 采样子集外构造的诊断上下文。
        detail: String,
    },
}

impl DxilError {
    /// 构造一个 [`DxilError::Unmappable`](内部便捷构造)。
    fn unmappable(what: impl Into<String>, detail: impl Into<String>) -> Self {
        DxilError::Unmappable {
            what: what.into(),
            detail: detail.into(),
        }
    }

    /// 构造一个 [`DxilError::SampleUnsupported`](采样子集外,RX6023)。
    fn sample_unsupported(detail: impl Into<String>) -> Self {
        DxilError::SampleUnsupported {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for DxilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DxilError::Unmappable { what, detail } => {
                write!(f, "unmappable SPIR-V construct ({what}): {detail}")
            }
            DxilError::SampleUnsupported { detail } => {
                write!(f, "texture sampling outside first-phase subset: {detail}")
            }
        }
    }
}

impl std::error::Error for DxilError {}

/// host 侧绑定推导失败 → 编码器错误映射(RXS-0163)。[`binding_layout::infer_spirv_bindings`]
/// 仅产 [`BindingInferError::Unmappable`](bindless/unbounded RD-018 / 非法基数);
/// 其余绑定推导失败类(register 冲突 / root signature 超限 / PSV0 失配)不在
/// SPIR-V 资源装饰 emit 阶段触达(归 codegen 层的 root signature 推导,PR-E2b)。
fn map_binding_err(e: BindingInferError) -> DxilError {
    DxilError::unmappable("binding-layout", e.to_string())
}

// ───────────────────────── SPIR-V 常量(核心规范取值) ─────────────────────────

/// SPIR-V magic number(字流首字,R1.4)。
const SPIRV_MAGIC: u32 = 0x0723_0203;
/// SPIR-V 版本字(1.0 = `0x0001_0000`;最小子集与广泛 spirv-val target-env 兼容)。
const SPIRV_VERSION_1_0: u32 = 0x0001_0000;
/// generator magic(未注册工具用 0;spirv-val 忽略此字段)。
const SPIRV_GENERATOR: u32 = 0;
/// header schema 字(保留,恒 0)。
const SPIRV_SCHEMA: u32 = 0;

// opcodes(SPIR-V core 规范)。
const OP_EXTENSION: u16 = 10;
const OP_MEMORY_MODEL: u16 = 14;
const OP_ENTRY_POINT: u16 = 15;
const OP_EXECUTION_MODE: u16 = 16;
const OP_CAPABILITY: u16 = 17;
const OP_TYPE_VOID: u16 = 19;
const OP_TYPE_INT: u16 = 21;
const OP_TYPE_FLOAT: u16 = 22;
const OP_TYPE_VECTOR: u16 = 23;
const OP_TYPE_IMAGE: u16 = 25;
const OP_TYPE_SAMPLER: u16 = 26;
const OP_TYPE_SAMPLED_IMAGE: u16 = 27;
const OP_TYPE_POINTER: u16 = 32;
const OP_TYPE_FUNCTION: u16 = 33;
const OP_CONSTANT: u16 = 43;
const OP_VARIABLE: u16 = 59;
const OP_LOAD: u16 = 61;
const OP_STORE: u16 = 62;
const OP_EXT_INST_IMPORT: u16 = 11;
const OP_EXT_INST: u16 = 12;
const OP_CONSTANT_COMPOSITE: u16 = 44;
/// `OpSampledImage`(组合 image + sampler 为采样图像,RXS-0175;RFC-0007)。
const OP_SAMPLED_IMAGE: u16 = 86;
/// `OpImageSampleImplicitLod`(隐式 LOD 采样,quad 导数;`sample`/`sample_bias`,RXS-0226/0227)。
const OP_IMAGE_SAMPLE_IMPLICIT_LOD: u16 = 87;
/// `OpImageSampleExplicitLod`(显式 LOD 采样;`sample_lod`(Lod)/`sample_grad`(Grad),RXS-0226)。
const OP_IMAGE_SAMPLE_EXPLICIT_LOD: u16 = 88;
/// `OpImageSampleDrefExplicitLod`(比较采样,shadow;`sample_cmp`,Lod 0,RXS-0226)。
const OP_IMAGE_SAMPLE_DREF_EXPLICIT_LOD: u16 = 90;
/// `OpImageFetch`(无过滤整型取址;`load`/`load_lod`,RXS-0226/0228)。
const OP_IMAGE_FETCH: u16 = 95;
/// `OpImageGather`(基层 2×2 单分量聚合;`gather`,RXS-0226)。
const OP_IMAGE_GATHER: u16 = 96;
/// `OpImageRead`(storage image 读;`TextureRw2D.load`,RXS-0226)。
const OP_IMAGE_READ: u16 = 98;
/// `OpImageWrite`(storage image 写;`TextureRw2D.store` 唯一写者,RXS-0226/0229)。
const OP_IMAGE_WRITE: u16 = 99;
/// `OpImageQuerySizeLod`(取 mip 层尺寸;texel fetch 越界钳制序列,RXS-0228)。
/// SPIR-V 核心 opcode = **103**(104 = `OpImageQuerySize`〔无 LOD 操作数,4 字〕;先前误置
/// 104 令带 LOD 操作数〔5 字〕的产物被 spirv-val 拒——由 PR-S3 采样模式 spirv-val 门捕获,
/// tests/sampling_vulkan_spirv_val.rs)。
const OP_IMAGE_QUERY_SIZE_LOD: u16 = 103;
const OP_DECORATE: u16 = 71;
const OP_FUNCTION: u16 = 54;
const OP_IADD: u16 = 128;
const OP_FADD: u16 = 129;
const OP_ISUB: u16 = 130;
const OP_FSUB: u16 = 131;
const OP_IMUL: u16 = 132;
const OP_FMUL: u16 = 133;
const OP_UDIV: u16 = 134;
const OP_SDIV: u16 = 135;
const OP_FDIV: u16 = 136;
const OP_LABEL: u16 = 248;
const OP_RETURN: u16 = 253;
const OP_FUNCTION_END: u16 = 56;

// 枚举取值。
const CAP_SHADER: u32 = 1;
const ADDR_MODEL_LOGICAL: u32 = 0;
const MEM_MODEL_GLSL450: u32 = 1;
const EXEC_MODEL_VERTEX: u32 = 0;
const EXEC_MODEL_FRAGMENT: u32 = 4;
const EXEC_MODE_ORIGIN_UPPER_LEFT: u32 = 7;
const STORAGE_INPUT: u32 = 1;
const STORAGE_OUTPUT: u32 = 3;
/// `UniformConstant`(opaque 资源:image/sampler 全局变量存储类)。
const STORAGE_UNIFORM_CONSTANT: u32 = 0;
const FUNCTION_CONTROL_NONE: u32 = 0;

// decoration 取值。
const DECORATION_BUILTIN: u32 = 11;
/// `NoPerspective`(线性/屏幕空间插值,`#[interpolate(noperspective)]`)。
const DECORATION_NO_PERSPECTIVE: u32 = 13;
/// `Flat`(无插值;Vulkan 强约束:整型/双精度 fragment 输入**须** Flat,
/// VUID-StandaloneSpirv-Flat-04744;`#[interpolate(flat)]`)。
const DECORATION_FLAT: u32 = 14;
const DECORATION_LOCATION: u32 = 30;
/// `Binding`(SPIR-V 资源绑定装饰:轴内绑定号)。
const DECORATION_BINDING: u32 = 33;
/// `DescriptorSet`(SPIR-V 资源绑定装饰:descriptor set 号)。
const DECORATION_DESCRIPTOR_SET: u32 = 34;
/// `UserSemantic`(= `HlslSemanticGOOGLE`,由 `SPV_GOOGLE_hlsl_functionality1` 启用)。
const DECORATION_USER_SEMANTIC: u32 = 5635;

/// 保名所依赖的 Google HLSL functionality 扩展(spirv-val 接受;启用
/// `UserSemantic` 装饰,R1.6)。
const EXT_HLSL_FUNCTIONALITY1: &str = "SPV_GOOGLE_hlsl_functionality1";

// BuiltIn 枚举取值(已建模子集)。
const BUILTIN_POSITION: u32 = 0;
const BUILTIN_POINT_SIZE: u32 = 1;
const BUILTIN_FRAG_COORD: u32 = 15;
const BUILTIN_FRAG_DEPTH: u32 = 22;
const BUILTIN_VERTEX_INDEX: u32 = 42;
const BUILTIN_INSTANCE_INDEX: u32 = 43;

// 资源(opaque)类型枚举取值(SPIR-V core 规范)。
/// `OpTypeImage` Dim = 2D。
const DIM_2D: u32 = 1;
/// `OpTypeImage` ImageFormat = Unknown(分离纹理 + 采样器,HLSL 形态)。
const IMAGE_FORMAT_UNKNOWN: u32 = 0;
/// `OpTypeImage` Sampled = 1(与采样器配合使用的采样图像)。
const IMAGE_SAMPLED_WITH_SAMPLER: u32 = 1;
/// `OpTypeImage` Sampled = 2(无采样器读写 storage image,`TextureRw2D`,RXS-0226)。
const IMAGE_SAMPLED_STORAGE: u32 = 2;
/// `ImageOperands` `Bias` bit(0x1;`sample_bias`,RXS-0226)。
const IMAGE_OPERANDS_BIAS: u32 = 0x1;
/// `ImageOperands` `Lod` bit(0x2;显式 LOD 采样 / texel fetch,RXS-0175/0226)。
const IMAGE_OPERANDS_LOD: u32 = 0x2;
/// `ImageOperands` `Grad` bit(0x4;`sample_grad` ddx/ddy,RXS-0226)。
const IMAGE_OPERANDS_GRAD: u32 = 0x4;
/// `Capability ImageQuery`(50;`OpImageQuerySizeLod` texel fetch 越界钳制,RXS-0228)。
const CAP_IMAGE_QUERY: u32 = 50;
/// `ImageFormat Rgba32f`(1;`TextureRw2D<f32>` 显式 format,RXS-0226 L2)。
const IMAGE_FORMAT_RGBA32F: u32 = 1;
/// `ImageFormat Rgba32i`(21;`TextureRw2D<i32>`)。
const IMAGE_FORMAT_RGBA32I: u32 = 21;
/// `ImageFormat Rgba32ui`(30;`TextureRw2D<u32>`)。
const IMAGE_FORMAT_RGBA32UI: u32 = 30;
/// GLSL.std.450 ext-inst `UMin`(38;texel fetch 越界钳制 `min(coord, size-1)`,RXS-0228)。
const GLSL_STD_450_UMIN: u32 = 38;
/// GLSL.std.450 ext-inst 集合名。
const EXT_GLSL_STD_450: &str = "GLSL.std.450";

// ── G3.4 bindless descriptor indexing(RXS-0234;RFC-0013 §4.C3;Vulkan 1.2 core)──
/// `OpTypeRuntimeArray`(29;无界 descriptor 数组元素类型)。
const OP_TYPE_RUNTIME_ARRAY: u16 = 29;
/// `OpAccessChain`(65;runtime array 动态索引 → 元素指针)。
const OP_ACCESS_CHAIN: u16 = 65;
/// `OpTypeStruct`(30;push-constant 块 `table_len` 尾槽,RXS-0208)。
const OP_TYPE_STRUCT: u16 = 30;
/// `OpMemberDecorate`(72;push-constant 成员 Offset)。
const OP_MEMBER_DECORATE: u16 = 72;
/// `Decoration NonUniform`(5300;非均匀索引临时句柄标注,ShaderNonUniform 依赖)。
const DECORATION_NON_UNIFORM: u32 = 5300;
/// `Decoration Block`(2;push-constant 块)。
const DECORATION_BLOCK: u32 = 2;
/// `Decoration Offset`(35;push-constant 成员字节偏移)。
const DECORATION_OFFSET: u32 = 35;
/// `Capability ShaderNonUniform`(5301;NonUniform 装饰依赖,Vk1.2 core)。
const CAP_SHADER_NON_UNIFORM: u32 = 5301;
/// `Capability RuntimeDescriptorArray`(5302;无界 descriptor 数组,Vk1.2 core)。
const CAP_RUNTIME_DESCRIPTOR_ARRAY: u32 = 5302;
/// `StorageClass PushConstant`(9;`table_len` 尾槽块,RXS-0208)。
const STORAGE_PUSH_CONSTANT: u32 = 9;
/// SPIR-V 扩展名 `SPV_EXT_descriptor_indexing`(Vulkan 1.2 core promoted)。
const EXT_DESCRIPTOR_INDEXING: &str = "SPV_EXT_descriptor_indexing";

// ───────────────────────── 编码器本体 ─────────────────────────

/// 已建模 builtin 的 SPIR-V 映射结果:`BuiltIn` 枚举 + 该 builtin 要求的类型。
struct BuiltinMapping {
    builtin: u32,
    expected: MirIoType,
}

/// 已 emit 的 I/O 变量记录。RXS-0171 只把源码层 I/O 元素绑定到 SPIR-V
/// Input/Output 变量,不暴露或冻结 Location/register/mask/packing 等 ABI 数值。
#[derive(Clone, Copy, Debug)]
struct IoVar {
    dir: IoDir,
    ty: MirIoType,
    var_id: u32,
}

/// 已 emit 的资源句柄变量记录(RXS-0175;采样 body lowering 消费)。`type_id` =
/// 该资源的 SPIR-V 类型 id(`OpTypeImage` for texture / `OpTypeSampler` for sampler);
/// `sampled_prim` = 纹理分量类型(texture 用,sampler 占位 f32)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ResourceVarKind {
    /// `Texture2D<F>` → 采样图像(Sampled=1,与采样器配合)。
    SampledImage,
    /// `TextureRw2D<F>` → storage image(Sampled=2,显式 format,读写,RXS-0226/0229)。
    StorageImage,
    /// `Sampler` → 采样器。
    Sampler,
    /// `SamplerCmp` → 比较采样器(与 depth 图像配合,RXS-0226)。
    SamplerCmp,
}

#[derive(Clone, Debug)]
struct ResourceVarInfo {
    /// 源码形参名(保名依据;BodyLowerer 按 MIR local 名匹配解析此变量)。
    name: String,
    /// SPIR-V 全局变量 id(`UniformConstant` 存储类)。
    var_id: u32,
    /// 资源 SPIR-V 类型 id(image / sampler)。
    type_id: u32,
    /// 资源种类(采样/storage 图像 / 采样器 / 比较采样器,RXS-0226)。
    kind: ResourceVarKind,
    /// 纹理分量类型(image 用;sampler 占位)。
    sampled_prim: PrimTy,
    /// G3.4 bindless(RXS-0234):`true` = 无界表(`[Texture2D<F>]`)——`var_id` 为
    /// `OpTypeRuntimeArray(image)` 的 `UniformConstant` 变量,`type_id` = 元素 image
    /// 类型(动态索引经 `OpAccessChain`→`OpLoad` 取元素);`false` = 单句柄。
    runtime_array: bool,
}

/// 把源码 builtin 名(在给定 `stage`/`dir` 下)映射到 SPIR-V `BuiltIn` 枚举与其
/// 要求的类型。超出已建模集合(未知名 / 阶段·方向不符)→ `None`(调用方发
/// [`DxilError::Unmappable`],strict-only)。
///
/// spirv-val 对 builtin 变量的类型有强约束(如 `Position`/`FragCoord` 须 vec4
/// float、`VertexIndex` 须 32-bit int 标量),故此处一并给出期望类型,由调用方校验,
/// 类型不符即不可映射(不产无效 SPIR-V)。
fn builtin_mapping(name: &str, stage: ShaderStage, dir: IoDir) -> Option<BuiltinMapping> {
    let vec4f = MirIoType::Vector(PrimTy::F32, 4);
    let f32s = MirIoType::Scalar(PrimTy::F32);
    let i32s = MirIoType::Scalar(PrimTy::I32);
    match (name, stage, dir) {
        // 顶点裁剪空间位置(vertex 输出)。
        ("position", ShaderStage::Vertex, IoDir::Out) => Some(BuiltinMapping {
            builtin: BUILTIN_POSITION,
            expected: vec4f,
        }),
        // 片元窗口空间坐标(fragment 输入)。
        ("position" | "frag_coord", ShaderStage::Fragment, IoDir::In) => Some(BuiltinMapping {
            builtin: BUILTIN_FRAG_COORD,
            expected: vec4f,
        }),
        // 顶点点尺寸(vertex 输出)。
        ("point_size", ShaderStage::Vertex, IoDir::Out) => Some(BuiltinMapping {
            builtin: BUILTIN_POINT_SIZE,
            expected: f32s,
        }),
        // 片元深度(fragment 输出)。
        ("frag_depth" | "depth", ShaderStage::Fragment, IoDir::Out) => Some(BuiltinMapping {
            builtin: BUILTIN_FRAG_DEPTH,
            expected: f32s,
        }),
        // 顶点/实例索引(vertex 输入,32-bit int 标量)。
        ("vertex_index", ShaderStage::Vertex, IoDir::In) => Some(BuiltinMapping {
            builtin: BUILTIN_VERTEX_INDEX,
            expected: i32s,
        }),
        ("instance_index", ShaderStage::Vertex, IoDir::In) => Some(BuiltinMapping {
            builtin: BUILTIN_INSTANCE_INDEX,
            expected: i32s,
        }),
        _ => None,
    }
}

/// builtin 类型符合性:`VertexIndex`/`InstanceIndex` 接受 `i32`/`u32`(均为
/// 32-bit int 标量,spirv-val 接受);其余 builtin 要求精确等于期望类型。
fn builtin_type_ok(expected: MirIoType, actual: MirIoType) -> bool {
    match expected {
        MirIoType::Scalar(PrimTy::I32) => {
            matches!(actual, MirIoType::Scalar(PrimTy::I32 | PrimTy::U32))
        }
        other => other == actual,
    }
}

/// SPIR-V 字流构造器:持有单调递增 result-id 计数器与各分节缓冲(纯 safe)。
struct Builder {
    /// 下一个可分配的 result-id(从 1 起;0 保留)。
    next_id: u32,
    /// 注解节(`OpDecorate`)。
    decorations: Vec<u32>,
    /// 类型/常量/全局变量节中的**类型**指令(按依赖序先于变量)。
    types: Vec<u32>,
    /// 全局**变量**指令(`OpVariable`,Input/Output 存储类)。
    variables: Vec<u32>,
    /// 入口接口变量 id 列表(`OpEntryPoint` 的 interface 段)。
    interface: Vec<u32>,
    /// 是否用到 `UserSemantic`(决定是否 emit `SPV_GOOGLE_hlsl_functionality1`)。
    used_user_semantic: bool,
    /// 是否 emit provenance 装饰(`UserSemantic` → `SPV_GOOGLE_hlsl_functionality1`)。
    /// DXIL 路 `true`(保名供 B 路 SPIRV-Cross→HLSL→dxc 边界改回用户语义名,字节不变);
    /// Vulkan 原生路 `false`(SPIR-V 即终产物,保名无消费者,去装饰免 device 扩展
    /// `VK_GOOGLE_hlsl_functionality1` 依赖 → 跨 ICD `vkCreateShaderModule` 直喂)。RXS-0210。
    emit_provenance: bool,
    /// 下一个 Input 方向 varying 的 `Location`(按方向各自递增分配)。
    next_in_location: u32,
    /// 下一个 Output 方向 varying 的 `Location`(按方向各自递增分配)。
    next_out_location: u32,
    // 类型去重缓存(小规模线性查找即可)。
    scalar_cache: Vec<(PrimTy, u32)>,
    vector_cache: Vec<(PrimTy, u8, u32)>,
    pointer_cache: Vec<(u32, u32, u32)>,
    /// 已 emit 的资源句柄变量(RXS-0175;采样 body lowering 按声明序消费)。
    resource_vars: Vec<ResourceVarInfo>,
    /// `OpTypeSampledImage` 去重缓存(image_type_id → sampled_image_type_id)。
    sampled_image_cache: Vec<(u32, u32)>,
    /// 追加能力(`Capability Shader` 之外;RXS-0228 texel fetch 需 `ImageQuery`)。
    extra_caps: Vec<u32>,
    /// `OpExtInstImport "GLSL.std.450"` 指令(懒 emit;texel fetch 越界钳制 UMin,RXS-0228)。
    ext_imports: Vec<u32>,
    /// GLSL.std.450 ext-inst 集合 result-id(懒分配)。
    glsl_ext_id: Option<u32>,
    /// `OpConstantComposite` 去重缓存(vec2<u32> {1,1} 钳制序列复用,RXS-0228)。
    const_composite_cache: Vec<(u32, u32)>,
    /// G3.4 bindless(RXS-0234):是否用到 descriptor indexing(决定 emit
    /// `SPV_EXT_descriptor_indexing` 扩展;capability 走 `extra_caps`)。
    used_descriptor_indexing: bool,
    /// G3.4 bindless(RXS-0208/0234):`table_len` push-constant 块的 `OpVariable` id
    /// (懒 emit,单块;成员 0 = `u32` 表长)。
    table_len_pc_var: Option<u32>,
}

impl Builder {
    fn new() -> Self {
        Builder {
            next_id: 1,
            decorations: Vec::new(),
            types: Vec::new(),
            variables: Vec::new(),
            interface: Vec::new(),
            used_user_semantic: false,
            // 默认保名(DXIL 路字节不变);Vulkan 路由经 emit_spirv_body_vulkan 置 false。
            emit_provenance: true,
            next_in_location: 0,
            next_out_location: 0,
            scalar_cache: Vec::new(),
            vector_cache: Vec::new(),
            pointer_cache: Vec::new(),
            resource_vars: Vec::new(),
            sampled_image_cache: Vec::new(),
            extra_caps: Vec::new(),
            ext_imports: Vec::new(),
            glsl_ext_id: None,
            const_composite_cache: Vec::new(),
            used_descriptor_indexing: false,
            table_len_pc_var: None,
        }
    }

    /// 登记追加能力(去重;RXS-0228)。
    fn add_capability(&mut self, cap: u32) {
        if !self.extra_caps.contains(&cap) {
            self.extra_caps.push(cap);
        }
    }

    /// G3.4 bindless(RXS-0208/0234):取/造 `table_len` push-constant 块的 `OpVariable`
    /// id(懒 emit,单块)。块 = `struct { u32 table_len; }`,`Block` 装饰、成员 Offset 0、
    /// `PushConstant` 存储类。运行时经既有 marshalling 通道于 push-constant 尾槽下发表长
    /// (宿主 TextureTable 已注册计数,RXS-0235)。成员 0 = `u32` 表长(clamp 上界源)。
    fn push_constant_table_len_var(&mut self) -> u32 {
        if let Some(v) = self.table_len_pc_var {
            return v;
        }
        let u32_ty = self.scalar_type(PrimTy::U32).expect("u32 标量类型");
        let struct_ty = self.alloc_id();
        Self::emit(&mut self.types, OP_TYPE_STRUCT, &[struct_ty, u32_ty]);
        // Block 装饰 + 成员 0 Offset 0(push-constant 块布局,RXS-0208)。
        Self::emit(
            &mut self.decorations,
            OP_DECORATE,
            &[struct_ty, DECORATION_BLOCK],
        );
        Self::emit(
            &mut self.decorations,
            OP_MEMBER_DECORATE,
            &[struct_ty, 0, DECORATION_OFFSET, 0],
        );
        let ptr_ty = self.pointer_type(STORAGE_PUSH_CONSTANT, struct_ty);
        let var = self.alloc_id();
        Self::emit(
            &mut self.variables,
            OP_VARIABLE,
            &[ptr_ty, var, STORAGE_PUSH_CONSTANT],
        );
        self.table_len_pc_var = Some(var);
        var
    }

    /// 取/造 GLSL.std.450 ext-inst 集合 result-id(懒 emit `OpExtInstImport`;RXS-0228)。
    fn glsl_ext_inst(&mut self) -> u32 {
        if let Some(id) = self.glsl_ext_id {
            return id;
        }
        let id = self.alloc_id();
        let mut operands = vec![id];
        Self::push_string(&mut operands, EXT_GLSL_STD_450);
        Self::emit(&mut self.ext_imports, OP_EXT_INST_IMPORT, &operands);
        self.glsl_ext_id = Some(id);
        id
    }

    /// 取/造 vec2<u32> `OpConstantComposite {a, a}`(钳制序列复用,RXS-0228)。
    fn const_uvec2_splat(&mut self, comp_id: u32, vec_ty: u32) -> u32 {
        if let Some(&(_, id)) = self
            .const_composite_cache
            .iter()
            .find(|&&(v, _)| v == vec_ty)
        {
            let _ = comp_id;
            return id;
        }
        let id = self.alloc_id();
        Self::emit(
            &mut self.types,
            OP_CONSTANT_COMPOSITE,
            &[vec_ty, id, comp_id, comp_id],
        );
        self.const_composite_cache.push((vec_ty, id));
        id
    }

    /// 分配下一个 result-id。
    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// 把一条指令(opcode + operands)写入指定分节缓冲。word0 = (wc<<16)|opcode。
    fn emit(section: &mut Vec<u32>, opcode: u16, operands: &[u32]) {
        let wc = (operands.len() + 1) as u32;
        section.push((wc << 16) | u32::from(opcode));
        section.extend_from_slice(operands);
    }

    /// 把字面串按 SPIR-V 规则(UTF-8、null 结尾、4 字节字对齐零填充)追加到 operands。
    fn push_string(operands: &mut Vec<u32>, s: &str) {
        let mut word = 0u32;
        let mut shift = 0u32;
        for &b in s.as_bytes() {
            word |= u32::from(b) << shift;
            shift += 8;
            if shift == 32 {
                operands.push(word);
                word = 0;
                shift = 0;
            }
        }
        // 始终追加一个尾字:承载 null 结尾与高位零填充(shift==0 时即纯 null 字)。
        operands.push(word);
    }

    /// 取/造 SPIR-V 标量类型 id(已建模子集:f32/i32/u32);其余 → 不可映射。
    fn scalar_type(&mut self, prim: PrimTy) -> Result<u32, DxilError> {
        if let Some(&(_, id)) = self.scalar_cache.iter().find(|&&(p, _)| p == prim) {
            return Ok(id);
        }
        let id = self.alloc_id();
        match prim {
            PrimTy::F32 => Self::emit(&mut self.types, OP_TYPE_FLOAT, &[id, 32]),
            // OpTypeInt: width=32, signedness(1=signed i32 / 0=unsigned u32)。
            PrimTy::I32 => Self::emit(&mut self.types, OP_TYPE_INT, &[id, 32, 1]),
            PrimTy::U32 => Self::emit(&mut self.types, OP_TYPE_INT, &[id, 32, 0]),
            other => {
                return Err(DxilError::unmappable(
                    "scalar-type",
                    format!("primitive {other:?} 不在已建模 SPIR-V 标量子集(f32/i32/u32)内"),
                ));
            }
        }
        self.scalar_cache.push((prim, id));
        Ok(id)
    }

    /// 取/造 SPIR-V 向量类型 id(分量数须 2..=4);否则 → 不可映射。
    fn vector_type(&mut self, prim: PrimTy, count: u8) -> Result<u32, DxilError> {
        if !(2..=4).contains(&count) {
            return Err(DxilError::unmappable(
                "vector-width",
                format!("向量分量数 {count} 越界(已建模 2..=4)"),
            ));
        }
        if let Some(&(_, _, id)) = self
            .vector_cache
            .iter()
            .find(|&&(p, c, _)| p == prim && c == count)
        {
            return Ok(id);
        }
        let comp = self.scalar_type(prim)?;
        let id = self.alloc_id();
        Self::emit(
            &mut self.types,
            OP_TYPE_VECTOR,
            &[id, comp, u32::from(count)],
        );
        self.vector_cache.push((prim, count, id));
        Ok(id)
    }

    /// 取/造一个 [`MirIoType`] 对应的 SPIR-V 值类型 id。
    fn value_type(&mut self, ty: MirIoType) -> Result<u32, DxilError> {
        match ty {
            MirIoType::Scalar(p) => self.scalar_type(p),
            MirIoType::Vector(p, n) => self.vector_type(p, n),
        }
    }

    /// 取/造 SPIR-V 指针类型 id(storage_class, 指向 base_type)。
    fn pointer_type(&mut self, storage: u32, base: u32) -> u32 {
        if let Some(&(_, _, id)) = self
            .pointer_cache
            .iter()
            .find(|&&(s, b, _)| s == storage && b == base)
        {
            return id;
        }
        let id = self.alloc_id();
        Self::emit(&mut self.types, OP_TYPE_POINTER, &[id, storage, base]);
        self.pointer_cache.push((storage, base, id));
        id
    }

    /// emit 一个 I/O 元素:全局 `OpVariable` + 装饰(`Location`/`BuiltIn` +
    /// `UserSemantic` 保名),并登记入口接口列表。
    fn emit_io_elem(&mut self, elem: &IoSigElem, stage: ShaderStage) -> Result<IoVar, DxilError> {
        let storage = match elem.dir {
            IoDir::In => STORAGE_INPUT,
            IoDir::Out => STORAGE_OUTPUT,
        };

        // builtin 元素:类型须符合 spirv-val 对该 builtin 的强约束。
        let builtin = match &elem.kind {
            IoSigKind::Builtin(name) => {
                let Some(m) = builtin_mapping(name, stage, elem.dir) else {
                    return Err(DxilError::unmappable(
                        "builtin",
                        format!(
                            "未建模 builtin `{name}`(stage={stage:?}, dir={:?})",
                            elem.dir
                        ),
                    ));
                };
                if !builtin_type_ok(m.expected, elem.ty) {
                    return Err(DxilError::unmappable(
                        "builtin-type",
                        format!(
                            "builtin `{name}` 类型 {:?} 与期望 {:?} 不符",
                            elem.ty, m.expected
                        ),
                    ));
                }
                Some(m.builtin)
            }
            IoSigKind::Interpolate(_) | IoSigKind::Varying => None,
        };

        let base = self.value_type(elem.ty)?;
        let ptr = self.pointer_type(storage, base);
        let var = self.alloc_id();
        Self::emit(&mut self.variables, OP_VARIABLE, &[ptr, var, storage]);
        self.interface.push(var);

        // 装饰:builtin → BuiltIn;varying/interpolate → Location(方向各自递增)。
        match builtin {
            Some(b) => Self::emit(
                &mut self.decorations,
                OP_DECORATE,
                &[var, DECORATION_BUILTIN, b],
            ),
            None => {
                let loc = match elem.dir {
                    IoDir::In => &mut self.next_in_location,
                    IoDir::Out => &mut self.next_out_location,
                };
                let n = *loc;
                *loc += 1;
                Self::emit(
                    &mut self.decorations,
                    OP_DECORATE,
                    &[var, DECORATION_LOCATION, n],
                );
                // 插值限定装饰(仅**跨阶段光栅化接口** = fragment 输入 / vertex 输出;
                // vertex 输入〔顶点缓冲侧〕与 fragment 输出〔颜色附件〕不承载,否则
                // spirv-val 拒 VUID-StandaloneSpirv-Flat-06202)。`#[interpolate(flat)]`
                // → `Flat`(Vulkan 强约束:整型/双精度 fragment 输入须 Flat,
                // VUID-StandaloneSpirv-Flat-04744;flat varying 两阶段限定须一致)/
                // `noperspective` → `NoPerspective`;`perspective`/裸 `Varying` = 默认透视无装饰。
                // 整型纹素坐标 varying(load/storage 模式)据此过 spirv-val。
                let interstage_varying = matches!(
                    (stage, elem.dir),
                    (ShaderStage::Fragment, IoDir::In) | (ShaderStage::Vertex, IoDir::Out)
                );
                if interstage_varying && let IoSigKind::Interpolate(mode) = &elem.kind {
                    let deco = match mode.as_str() {
                        "flat" => Some(DECORATION_FLAT),
                        "noperspective" => Some(DECORATION_NO_PERSPECTIVE),
                        _ => None,
                    };
                    if let Some(d) = deco {
                        Self::emit(&mut self.decorations, OP_DECORATE, &[var, d]);
                    }
                }
            }
        }

        // by-construction provenance:对有用户语义名的 I/O emit UserSemantic(SPIR-V 层
        // provenance,经 spirv-val 干净保留)。**spirv-cross 不消费**它为 HLSL 语义(实测)。
        // 保名通道:vertex 输入经 `dxil_codegen::vertex_input_semantic_flags` 的 location
        // 覆盖旗标(机制①,RXS-0159 IR1(a));**输出 varying / fragment 输入 varying** 经
        // **RXS-0172** `dxil_codegen::restore_varying_semantics` 在 spirv-cross→dxc 的 HLSL
        // 边界按 location provenance 改回用户名(RD-017,选项①);保名失败仍经校验门 RX6011
        // strict-only 拒(不放宽门,Property 5)。
        // provenance gate(RXS-0210):Vulkan 原生路(`emit_provenance=false`)不 emit
        // UserSemantic → `used_user_semantic` 保持 false → `SPV_GOOGLE` 自然不 emit。
        if self.emit_provenance && !elem.field_name.is_empty() {
            let mut operands = vec![var, DECORATION_USER_SEMANTIC];
            Self::push_string(&mut operands, &elem.field_name);
            Self::emit(&mut self.decorations, OP_DECORATE, &operands);
            self.used_user_semantic = true;
        }

        Ok(IoVar {
            dir: elem.dir,
            ty: elem.ty,
            var_id: var,
        })
    }

    /// emit 一个资源句柄绑定(RXS-0163;PR-E2b 生产接线):opaque 资源类型
    /// (`OpTypeImage`/`OpTypeSampler`)+ `UniformConstant` 全局 `OpVariable` +
    /// `DescriptorSet`/`Binding` 装饰。`set`/`binding` 由 host 侧推导
    /// ([`binding_layout::infer_spirv_bindings`])给定,本编码器**机械落字节、不
    /// 自创编号**。资源变量不入 `OpEntryPoint` interface(SPIR-V 1.0:interface 仅
    /// Input/Output 变量)。
    ///
    /// # Errors
    /// 编码器最小资源子集(`Texture2D<F>`/`Sampler`)外的资源类型 →
    /// [`DxilError::Unmappable`](strict-only;CBV/structured buffer 的 SPIR-V 降级
    /// 为后续扩展,源侧首批不可达)。
    fn emit_resource(
        &mut self,
        res: &ResourceBinding,
        set: u32,
        binding: u32,
    ) -> Result<(), DxilError> {
        let (res_type, kind, sampled_prim) = match res.res {
            MirResourceType::Texture2D(prim) => {
                let sampled_type = self.scalar_type(prim)?;
                let id = self.alloc_id();
                // OpTypeImage: sampled_type, Dim2D, depth=0, arrayed=0, ms=0,
                // sampled=1(与采样器配合), format=Unknown(分离纹理形态)。
                Self::emit(
                    &mut self.types,
                    OP_TYPE_IMAGE,
                    &[
                        id,
                        sampled_type,
                        DIM_2D,
                        0,
                        0,
                        0,
                        IMAGE_SAMPLED_WITH_SAMPLER,
                        IMAGE_FORMAT_UNKNOWN,
                    ],
                );
                (id, ResourceVarKind::SampledImage, prim)
            }
            MirResourceType::Sampler => {
                let id = self.alloc_id();
                Self::emit(&mut self.types, OP_TYPE_SAMPLER, &[id]);
                (id, ResourceVarKind::Sampler, PrimTy::F32)
            }
            // RXS-0226 L2:storage image = Sampled=2 + 显式 format(规避
            // `shaderStorageImageWriteWithoutFormat` capability 依赖)。
            MirResourceType::TextureRw2D(prim) => {
                let sampled_type = self.scalar_type(prim)?;
                let format = match prim {
                    PrimTy::F32 => IMAGE_FORMAT_RGBA32F,
                    PrimTy::I32 => IMAGE_FORMAT_RGBA32I,
                    PrimTy::U32 => IMAGE_FORMAT_RGBA32UI,
                    other => {
                        return Err(DxilError::sample_unsupported(format!(
                            "TextureRw2D 分量类型 {other:?} 无显式 storage image format\
                             (首期 {{f32,u32,i32}},RXS-0226 L2)"
                        )));
                    }
                };
                let id = self.alloc_id();
                Self::emit(
                    &mut self.types,
                    OP_TYPE_IMAGE,
                    &[
                        id,
                        sampled_type,
                        DIM_2D,
                        0,
                        0,
                        0,
                        IMAGE_SAMPLED_STORAGE,
                        format,
                    ],
                );
                (id, ResourceVarKind::StorageImage, prim)
            }
            // RXS-0226 L2:比较采样器与 depth 图像配合 → OpTypeSampler(depth-image
            // 由采样 lowering 在 OpTypeImage Depth=1 上组合)。
            MirResourceType::SamplerCmp => {
                let id = self.alloc_id();
                Self::emit(&mut self.types, OP_TYPE_SAMPLER, &[id]);
                (id, ResourceVarKind::SamplerCmp, PrimTy::F32)
            }
            other => {
                return Err(DxilError::unmappable(
                    "resource-type",
                    format!(
                        "资源 `{}` 类型 {other:?} 不在 B 路编码器资源最小子集\
                         (Texture2D<F>/Sampler/TextureRw2D<F>/SamplerCmp)内\
                         (CBV/structured buffer SPIR-V 降级为后续扩展)",
                        res.name
                    ),
                ));
            }
        };

        // G3.4 bindless(RXS-0234;RFC-0013 §4.C3):无界表 `[Texture2D<F>]` 的变量类型
        // 为 `OpTypeRuntimeArray(image)`(元素 = image 类型)+ `RuntimeDescriptorArray`
        // capability + `SPV_EXT_descriptor_indexing`(Vulkan 1.2 core)。`type_id` 存元素
        // image 类型(动态索引 `OpAccessChain`→`OpLoad` 取元素句柄)。
        let is_unbounded = matches!(res.count, crate::mir::ResourceCount::Unbounded);
        let (var_type, runtime_array) = if is_unbounded {
            let rt_array = self.alloc_id();
            Self::emit(
                &mut self.types,
                OP_TYPE_RUNTIME_ARRAY,
                &[rt_array, res_type],
            );
            self.add_capability(CAP_RUNTIME_DESCRIPTOR_ARRAY);
            self.add_capability(CAP_SHADER_NON_UNIFORM);
            self.used_descriptor_indexing = true;
            (rt_array, true)
        } else {
            (res_type, false)
        };

        let ptr = self.pointer_type(STORAGE_UNIFORM_CONSTANT, var_type);
        let var = self.alloc_id();
        Self::emit(
            &mut self.variables,
            OP_VARIABLE,
            &[ptr, var, STORAGE_UNIFORM_CONSTANT],
        );

        // 资源绑定装饰:DescriptorSet + Binding(host 推导给定,机械落字节)。
        Self::emit(
            &mut self.decorations,
            OP_DECORATE,
            &[var, DECORATION_DESCRIPTOR_SET, set],
        );
        Self::emit(
            &mut self.decorations,
            OP_DECORATE,
            &[var, DECORATION_BINDING, binding],
        );

        // by-construction 保名:资源句柄亦 emit UserSemantic provenance(源码形参名)。
        // provenance gate(RXS-0210):Vulkan 原生路不 emit(同 I/O 元素路径)。
        if self.emit_provenance && !res.name.is_empty() {
            let mut operands = vec![var, DECORATION_USER_SEMANTIC];
            Self::push_string(&mut operands, &res.name);
            Self::emit(&mut self.decorations, OP_DECORATE, &operands);
            self.used_user_semantic = true;
        }

        // 登记资源变量(RXS-0175/0226;采样方法族 body lowering 按名匹配 MIR local 解析)。
        self.resource_vars.push(ResourceVarInfo {
            name: res.name.clone(),
            var_id: var,
            type_id: res_type,
            kind,
            sampled_prim,
            runtime_array,
        });

        Ok(())
    }

    /// 取/造 `OpTypeSampledImage`(组合采样图像类型;RXS-0175)。
    fn sampled_image_type(&mut self, image_type: u32) -> u32 {
        if let Some(&(_, id)) = self
            .sampled_image_cache
            .iter()
            .find(|&&(img, _)| img == image_type)
        {
            return id;
        }
        let id = self.alloc_id();
        Self::emit(&mut self.types, OP_TYPE_SAMPLED_IMAGE, &[id, image_type]);
        self.sampled_image_cache.push((image_type, id));
        id
    }
}

#[derive(Clone, Copy, Debug)]
struct SpirvValue {
    id: u32,
    ty: MirIoType,
}

#[derive(Clone, Debug)]
enum LocalValue {
    Unit,
    Value(SpirvValue),
    Aggregate(Vec<SpirvValue>),
}

/// RXS-0171 最小 body lowering:只支持 straight-line 的 Use / Const / 标量或向量
/// 算术 BinaryOp,并把输出 I/O 聚合返回值机械分解为逐元素 OpStore。
struct BodyLowerer<'a> {
    body: &'a Body,
    input_vars: Vec<IoVar>,
    output_vars: Vec<IoVar>,
    local_values: HashMap<u32, LocalValue>,
    output_written: Vec<bool>,
    ops: Vec<u32>,
    /// 已 emit 的资源句柄变量(RXS-0175;采样 lowering 按 MIR local 名匹配解析)。
    resource_vars: Vec<ResourceVarInfo>,
}

impl<'a> BodyLowerer<'a> {
    fn new(body: &'a Body, io_vars: &'a [IoVar], resource_vars: Vec<ResourceVarInfo>) -> Self {
        let input_vars = io_vars
            .iter()
            .copied()
            .filter(|v| v.dir == IoDir::In)
            .collect();
        let output_vars: Vec<IoVar> = io_vars
            .iter()
            .copied()
            .filter(|v| v.dir == IoDir::Out)
            .collect();
        let output_written = vec![false; output_vars.len()];
        BodyLowerer {
            body,
            input_vars,
            output_vars,
            local_values: HashMap::new(),
            output_written,
            ops: Vec::new(),
            resource_vars,
        }
    }

    fn lower(mut self, b: &mut Builder) -> Result<Vec<u32>, DxilError> {
        let mut block = 0usize;
        let mut seen = vec![false; self.body.blocks.len()];
        loop {
            let Some(bb) = self.body.blocks.get(block) else {
                return Err(DxilError::unmappable(
                    "body-control-flow",
                    format!("basic block bb{block} 越界"),
                ));
            };
            if seen[block] {
                return Err(DxilError::unmappable(
                    "body-control-flow",
                    "RXS-0171 最小切片不支持循环或重复进入 basic block",
                ));
            }
            seen[block] = true;

            for stmt in &bb.stmts {
                match &stmt.kind {
                    StatementKind::Assign(place, rv) => self.lower_assign(b, place, rv)?,
                }
            }

            match &bb.terminator.kind {
                TerminatorKind::Return => break,
                TerminatorKind::Goto(next) => {
                    block = next.0 as usize;
                }
                other => {
                    return Err(DxilError::unmappable(
                        "body-terminator",
                        format!(
                            "RXS-0171 最小切片仅支持 straight-line Goto/Return, 实得 {other:?}"
                        ),
                    ));
                }
            }
        }

        if !self.output_vars.is_empty() && !self.output_written.iter().all(|w| *w) {
            return Err(DxilError::unmappable(
                "output-return",
                "着色 body 未写出所有声明的 Output I/O 元素",
            ));
        }

        Ok(self.ops)
    }

    fn lower_assign(
        &mut self,
        b: &mut Builder,
        place: &Place,
        rv: &Rvalue,
    ) -> Result<(), DxilError> {
        if place.local == LocalIdx(0) {
            if let Some(index) = single_field_projection(place)? {
                let expected = self.output_ty(index)?;
                let value = self.lower_rvalue_value(b, rv, Some(expected))?;
                return self.store_output(index, value);
            }
            let value = self.lower_rvalue_any(b, rv)?;
            return self.store_return_value(value);
        }

        if !place.proj.is_empty() {
            return Err(DxilError::unmappable(
                "body-destination",
                format!("RXS-0171 最小切片不支持写入投影 place `{place:?}`"),
            ));
        }

        let value = self.lower_rvalue_any(b, rv)?;
        self.local_values.insert(place.local.0, value);
        Ok(())
    }

    fn lower_rvalue_any(&mut self, b: &mut Builder, rv: &Rvalue) -> Result<LocalValue, DxilError> {
        match rv {
            Rvalue::Use(op) => self.lower_operand_any(b, op, None),
            Rvalue::BinaryOp(op, lhs, rhs) => {
                Ok(LocalValue::Value(self.lower_binary_op(b, *op, lhs, rhs)?))
            }
            Rvalue::Aggregate(ty, ops) => self.lower_output_aggregate(b, ty, ops),
            Rvalue::ResourceSample {
                texture_local,
                sampler_local,
                table_index,
                method,
                coord,
                extra,
            } => self.lower_resource_op(
                b,
                *method,
                texture_local.0,
                sampler_local.map(|s| s.0),
                table_index.as_ref(),
                coord,
                extra,
            ),
            other => Err(DxilError::unmappable(
                "body-rvalue",
                format!("RXS-0171 最小切片不支持 rvalue `{other:?}`"),
            )),
        }
    }

    fn lower_rvalue_value(
        &mut self,
        b: &mut Builder,
        rv: &Rvalue,
        expected: Option<MirIoType>,
    ) -> Result<SpirvValue, DxilError> {
        match rv {
            Rvalue::Use(op) => self.lower_operand_value(b, op, expected),
            Rvalue::BinaryOp(op, lhs, rhs) => self.lower_binary_op(b, *op, lhs, rhs),
            Rvalue::Aggregate(..) => Err(DxilError::unmappable(
                "body-rvalue",
                "输出字段写入需要标量/向量值,不能直接写聚合",
            )),
            other => Err(DxilError::unmappable(
                "body-rvalue",
                format!("RXS-0171 最小切片不支持 rvalue `{other:?}`"),
            )),
        }
    }

    fn lower_output_aggregate(
        &mut self,
        b: &mut Builder,
        ty: &crate::ty::Ty,
        operands: &[Operand],
    ) -> Result<LocalValue, DxilError> {
        if self.output_vars.is_empty() {
            return Err(DxilError::unmappable(
                "aggregate",
                "无 Output I/O 签名时不允许聚合返回值降级",
            ));
        }
        if ty != self.body.ret_ty() || operands.len() != self.output_vars.len() {
            return Err(DxilError::unmappable(
                "aggregate",
                format!(
                    "仅允许声明的输出 I/O 聚合返回值机械分解; ret_ty={:?}, aggregate_ty={ty:?}, fields={}, outs={}",
                    self.body.ret_ty(),
                    operands.len(),
                    self.output_vars.len()
                ),
            ));
        }

        let mut values = Vec::with_capacity(operands.len());
        for (idx, op) in operands.iter().enumerate() {
            values.push(self.lower_operand_value(b, op, Some(self.output_ty(idx)?))?);
        }
        Ok(LocalValue::Aggregate(values))
    }

    fn lower_operand_any(
        &mut self,
        b: &mut Builder,
        op: &Operand,
        expected: Option<MirIoType>,
    ) -> Result<LocalValue, DxilError> {
        match op {
            Operand::Const(Const::Unit) => Ok(LocalValue::Unit),
            Operand::Const(c) => Ok(LocalValue::Value(self.lower_const(b, c, expected)?)),
            Operand::Copy(place) | Operand::Move(place) => {
                if place.proj.is_empty()
                    && let Some(v) = self.lower_place_aggregate(b, place)?
                {
                    return Ok(LocalValue::Aggregate(v));
                }
                Ok(LocalValue::Value(self.lower_place_value(b, place)?))
            }
        }
    }

    fn lower_operand_value(
        &mut self,
        b: &mut Builder,
        op: &Operand,
        expected: Option<MirIoType>,
    ) -> Result<SpirvValue, DxilError> {
        match self.lower_operand_any(b, op, expected)? {
            LocalValue::Value(v) => Ok(v),
            LocalValue::Unit => Err(DxilError::unmappable(
                "operand",
                "unit 常量不能作为 SPIR-V 标量/向量值",
            )),
            LocalValue::Aggregate(_) => Err(DxilError::unmappable(
                "operand",
                "聚合值只能用于输出 I/O 聚合返回分解",
            )),
        }
    }

    fn lower_place_value(
        &mut self,
        b: &mut Builder,
        place: &Place,
    ) -> Result<SpirvValue, DxilError> {
        if let Some(field) = single_field_projection(place)? {
            if place.local.0 >= 1 && (place.local.0 as usize) <= self.body.arg_count {
                return self.load_input_field(b, field);
            }
            let local = self
                .local_values
                .get(&place.local.0)
                .cloned()
                .ok_or_else(|| {
                    DxilError::unmappable(
                        "place",
                        format!("local _{} 尚未在 RXS-0171 白名单中物化", place.local.0),
                    )
                })?;
            return match local {
                LocalValue::Aggregate(fields) => fields.get(field).copied().ok_or_else(|| {
                    DxilError::unmappable(
                        "place-field",
                        format!("local _{} 字段 {field} 越界", place.local.0),
                    )
                }),
                LocalValue::Value(_) | LocalValue::Unit => Err(DxilError::unmappable(
                    "place-field",
                    format!("local _{} 不是可投影聚合", place.local.0),
                )),
            };
        }

        if !place.proj.is_empty() {
            return Err(DxilError::unmappable(
                "place-projection",
                format!("RXS-0171 最小切片不支持 projection `{place:?}`"),
            ));
        }

        let local = self
            .local_values
            .get(&place.local.0)
            .cloned()
            .ok_or_else(|| {
                DxilError::unmappable(
                    "place",
                    format!("local _{} 尚未在 RXS-0171 白名单中物化", place.local.0),
                )
            })?;
        match local {
            LocalValue::Value(v) => Ok(v),
            LocalValue::Unit | LocalValue::Aggregate(_) => Err(DxilError::unmappable(
                "place",
                format!("local _{} 不是标量/向量值", place.local.0),
            )),
        }
    }

    fn lower_place_aggregate(
        &mut self,
        b: &mut Builder,
        place: &Place,
    ) -> Result<Option<Vec<SpirvValue>>, DxilError> {
        if !place.proj.is_empty() {
            return Ok(None);
        }
        if place.local.0 >= 1 && (place.local.0 as usize) <= self.body.arg_count {
            let mut values = Vec::with_capacity(self.input_vars.len());
            for idx in 0..self.input_vars.len() {
                values.push(self.load_input_field(b, idx)?);
            }
            return Ok(Some(values));
        }
        Ok(match self.local_values.get(&place.local.0) {
            Some(LocalValue::Aggregate(fields)) => Some(fields.clone()),
            _ => None,
        })
    }

    fn lower_const(
        &mut self,
        b: &mut Builder,
        c: &Const,
        expected: Option<MirIoType>,
    ) -> Result<SpirvValue, DxilError> {
        let (ty, literal) = match c {
            Const::Int(v, prim @ (PrimTy::I32 | PrimTy::U32)) => {
                let ty = MirIoType::Scalar(*prim);
                if let Some(expected) = expected
                    && expected != ty
                {
                    return Err(DxilError::unmappable(
                        "constant-type",
                        format!("常量类型 {ty:?} 与期望 {expected:?} 不符"),
                    ));
                }
                let word = match prim {
                    PrimTy::I32 => i32::try_from(*v).map(|x| x as u32).map_err(|_| {
                        DxilError::unmappable("constant", format!("i32 常量 {v} 越界"))
                    })?,
                    PrimTy::U32 => u32::try_from(*v).map_err(|_| {
                        DxilError::unmappable("constant", format!("u32 常量 {v} 越界"))
                    })?,
                    _ => unreachable!(),
                };
                (ty, word)
            }
            Const::Float(v, PrimTy::F32) => {
                let ty = MirIoType::Scalar(PrimTy::F32);
                if let Some(expected) = expected
                    && expected != ty
                {
                    return Err(DxilError::unmappable(
                        "constant-type",
                        format!("常量类型 {ty:?} 与期望 {expected:?} 不符"),
                    ));
                }
                (ty, (*v as f32).to_bits())
            }
            other => {
                return Err(DxilError::unmappable(
                    "constant",
                    format!("RXS-0171 最小切片仅支持 f32/i32/u32 常量, 实得 {other:?}"),
                ));
            }
        };

        let ty_id = b.value_type(ty)?;
        let id = b.alloc_id();
        Builder::emit(&mut b.types, OP_CONSTANT, &[ty_id, id, literal]);
        Ok(SpirvValue { id, ty })
    }

    fn lower_binary_op(
        &mut self,
        b: &mut Builder,
        op: BinOp,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<SpirvValue, DxilError> {
        let a = self.lower_operand_value(b, lhs, None)?;
        let bval = self.lower_operand_value(b, rhs, Some(a.ty))?;
        if a.ty != bval.ty {
            return Err(DxilError::unmappable(
                "binary-op-type",
                format!("二元操作左右类型不一致: {:?} vs {:?}", a.ty, bval.ty),
            ));
        }
        let prim = mir_io_prim(a.ty);
        let opcode = match (op, prim) {
            (BinOp::Add, PrimTy::F32) => OP_FADD,
            (BinOp::Sub, PrimTy::F32) => OP_FSUB,
            (BinOp::Mul, PrimTy::F32) => OP_FMUL,
            (BinOp::Div, PrimTy::F32) => OP_FDIV,
            (BinOp::Add, PrimTy::I32 | PrimTy::U32) => OP_IADD,
            (BinOp::Sub, PrimTy::I32 | PrimTy::U32) => OP_ISUB,
            (BinOp::Mul, PrimTy::I32 | PrimTy::U32) => OP_IMUL,
            (BinOp::Div, PrimTy::I32) => OP_SDIV,
            (BinOp::Div, PrimTy::U32) => OP_UDIV,
            _ => {
                return Err(DxilError::unmappable(
                    "binary-op",
                    format!("RXS-0171 最小切片仅支持 f32/i32/u32 加减乘除, 实得 {op:?}/{prim:?}"),
                ));
            }
        };

        let ty_id = b.value_type(a.ty)?;
        let id = b.alloc_id();
        Builder::emit(&mut self.ops, opcode, &[ty_id, id, a.id, bval.id]);
        Ok(SpirvValue { id, ty: a.ty })
    }

    /// 解析 MIR local 下标 → 已 emit 的资源句柄变量(按 local 名匹配 `resource_vars`,
    /// RXS-0175;句柄非值,不进 `local_values`)。
    fn resource_for_local(&self, local: u32) -> Result<ResourceVarInfo, DxilError> {
        let name = self
            .body
            .locals
            .get(local as usize)
            .and_then(|l| l.name.as_deref())
            .ok_or_else(|| {
                DxilError::sample_unsupported(format!(
                    "采样句柄 local _{local} 无源码名,无法解析资源绑定"
                ))
            })?;
        self.resource_vars
            .iter()
            .find(|r| r.name == name)
            .cloned()
            .ok_or_else(|| {
                DxilError::sample_unsupported(format!(
                    "采样句柄 `{name}`(local _{local})未在资源绑定声明中(RXS-0163/0175)"
                ))
            })
    }

    /// 采样方法族 lowering(RXS-0223/0226;RFC-0013 §4.B6):按 [`ResourceMethod`]
    /// 分发 SPIR-V opcode 全家。sample 族(`OpImageSample{Implicit,Explicit}Lod` /
    /// `OpImageSampleDrefExplicitLod` / `OpImageGather`)产 `vec4<F>` 或 `sample_cmp`
    /// 的标量 `f32`;texel fetch(`OpImageFetch` + 越界钳制序列,RXS-0228)产 `vec4<F>`;
    /// storage image(`OpImageRead`/`OpImageWrite`,RXS-0229 唯一写者)产 `vec4<F>` / `()`。
    /// 子集外 → [`DxilError::SampleUnsupported`](RX6023)。
    #[allow(clippy::too_many_arguments)] // G3.4 table_index 参数(bindless);采样面自然多参
    fn lower_resource_op(
        &mut self,
        b: &mut Builder,
        method: ResourceMethod,
        texture_local: u32,
        sampler_local: Option<u32>,
        table_index: Option<&Operand>,
        coord: &Operand,
        extra: &[Operand],
    ) -> Result<LocalValue, DxilError> {
        use ResourceMethod as M;
        // G3.4(RXS-0234):首期无界表动态索引仅承 sample 族(`[Texture2D<F>]` = SRV
        // 纹理表);load/store 表元素越出首期(§8),诚实拒绝而非静默取元素 0。
        if table_index.is_some()
            && !matches!(
                method,
                M::Sample | M::SampleLod | M::SampleGrad | M::SampleBias | M::SampleCmp | M::Gather
            )
        {
            return Err(DxilError::sample_unsupported(format!(
                "bindless 无界表动态索引首期仅支持 sample 族,`{}` 越出首期(RXS-0234/§8)",
                method.name()
            )));
        }
        match method {
            // ── storage image(TextureRw2D):无 sampler,坐标 vec2<u32>,RXS-0229 ──
            M::StorageLoad => Ok(LocalValue::Value(self.lower_storage_load(
                b,
                texture_local,
                coord,
            )?)),
            M::Store => {
                self.lower_storage_store(b, texture_local, coord, extra)?;
                Ok(LocalValue::Unit)
            }
            // ── texel fetch(Texture2D):无 sampler,坐标 vec2<u32> + 越界钳制,RXS-0228 ──
            M::Load | M::LoadLod => Ok(LocalValue::Value(self.lower_texel_fetch(
                b,
                method,
                texture_local,
                coord,
                extra,
            )?)),
            // ── sample 族(Texture2D + Sampler/SamplerCmp):坐标 vec2<f32> ──
            M::Sample | M::SampleLod | M::SampleGrad | M::SampleBias | M::SampleCmp | M::Gather => {
                let sampler_local = sampler_local.ok_or_else(|| {
                    DxilError::sample_unsupported(format!(
                        "采样方法 `{}` 缺 sampler 实参",
                        method.name()
                    ))
                })?;
                self.lower_sample_family(
                    b,
                    method,
                    texture_local,
                    sampler_local,
                    table_index,
                    coord,
                    extra,
                )
            }
        }
    }

    /// sample 族 lowering(sample/sample_lod/sample_grad/sample_bias/sample_cmp/gather)。
    /// G3.4 bindless(RXS-0234):`table_index = Some` 时 `texture_local` 指向无界表
    /// (`OpTypeRuntimeArray`),经 `OpAccessChain`(clamp 后索引)→`OpLoad` 取元素 image
    /// 句柄 + `NonUniform` 装饰(不物化中间句柄 local)。
    #[allow(clippy::too_many_arguments)] // G3.4 table_index 参数(bindless);采样面自然多参
    fn lower_sample_family(
        &mut self,
        b: &mut Builder,
        method: ResourceMethod,
        texture_local: u32,
        sampler_local: u32,
        table_index: Option<&Operand>,
        coord: &Operand,
        extra: &[Operand],
    ) -> Result<LocalValue, DxilError> {
        use ResourceMethod as M;
        // coord 须为 vec2<f32>(归一化 UV;首期子集,RXS-0175/0223)。
        let coord_val = self.lower_operand_value(b, coord, None)?;
        if coord_val.ty != MirIoType::Vector(PrimTy::F32, 2) {
            return Err(DxilError::sample_unsupported(format!(
                "采样坐标类型 {:?} 非 vec2<f32>(首期收敛子集)",
                coord_val.ty
            )));
        }

        let tex = self.resource_for_local(texture_local)?;
        let samp = self.resource_for_local(sampler_local)?;
        if tex.kind != ResourceVarKind::SampledImage {
            return Err(DxilError::sample_unsupported(format!(
                "采样 receiver `{}` 非 Texture2D 采样图像句柄",
                tex.name
            )));
        }
        // sample_cmp 须 SamplerCmp;其余 sample 族须 Sampler(RXS-0223)。
        let want_cmp = method == M::SampleCmp;
        let samp_ok = if want_cmp {
            samp.kind == ResourceVarKind::SamplerCmp
        } else {
            samp.kind == ResourceVarKind::Sampler
        };
        if !samp_ok {
            return Err(DxilError::sample_unsupported(format!(
                "采样方法 `{}` 的 sampler 实参 `{}` 种类不符({:?})",
                method.name(),
                samp.name,
                samp.kind
            )));
        }
        if tex.sampled_prim != PrimTy::F32 {
            return Err(DxilError::sample_unsupported(format!(
                "sample 族首期仅支持 Texture2D<f32>(实得分量类型 {:?})",
                tex.sampled_prim
            )));
        }

        // OpLoad 纹理句柄:单句柄 = 直接 OpLoad var;G3.4 无界表(RXS-0234)= 先
        // clamp 索引 → OpAccessChain(runtime array 元素指针)→ OpLoad + NonUniform 装饰。
        let img_id = match table_index {
            Some(idx) if tex.runtime_array => self.lower_bindless_image_load(b, &tex, idx)?,
            Some(_) => {
                return Err(DxilError::sample_unsupported(format!(
                    "采样 receiver `{}` 带动态索引但非无界表(RXS-0234)",
                    tex.name
                )));
            }
            None => {
                let img_id = b.alloc_id();
                Builder::emit(&mut self.ops, OP_LOAD, &[tex.type_id, img_id, tex.var_id]);
                img_id
            }
        };
        let samp_id = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_LOAD,
            &[samp.type_id, samp_id, samp.var_id],
        );

        // OpSampledImage 组合。
        let si_ty = b.sampled_image_type(tex.type_id);
        let si_id = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_SAMPLED_IMAGE,
            &[si_ty, si_id, img_id, samp_id],
        );

        match method {
            // sample_lod:显式任意层 → OpImageSampleExplicitLod(Lod)。空 extra = LOD 0
            // (既有 `.sample()` 迁移,byte-preserving,RXS-0223 Q-S-SampleName)。
            M::SampleLod => {
                let lod = self.lod_operand(b, extra)?;
                let (result_ty, result_mir) = self.vec4f(b)?;
                let result_id = b.alloc_id();
                Builder::emit(
                    &mut self.ops,
                    OP_IMAGE_SAMPLE_EXPLICIT_LOD,
                    &[
                        result_ty,
                        result_id,
                        si_id,
                        coord_val.id,
                        IMAGE_OPERANDS_LOD,
                        lod,
                    ],
                );
                Ok(LocalValue::Value(SpirvValue {
                    id: result_id,
                    ty: result_mir,
                }))
            }
            // sample:隐式 LOD(quad 导数,🔒 RXS-0227)→ OpImageSampleImplicitLod(无 operands)。
            M::Sample => {
                let (result_ty, result_mir) = self.vec4f(b)?;
                let result_id = b.alloc_id();
                Builder::emit(
                    &mut self.ops,
                    OP_IMAGE_SAMPLE_IMPLICIT_LOD,
                    &[result_ty, result_id, si_id, coord_val.id],
                );
                Ok(LocalValue::Value(SpirvValue {
                    id: result_id,
                    ty: result_mir,
                }))
            }
            // sample_bias:隐式 + bias → OpImageSampleImplicitLod(Bias)。
            M::SampleBias => {
                let bias = self.f32_operand(b, extra, "sample_bias 缺 bias:f32")?;
                let (result_ty, result_mir) = self.vec4f(b)?;
                let result_id = b.alloc_id();
                Builder::emit(
                    &mut self.ops,
                    OP_IMAGE_SAMPLE_IMPLICIT_LOD,
                    &[
                        result_ty,
                        result_id,
                        si_id,
                        coord_val.id,
                        IMAGE_OPERANDS_BIAS,
                        bias,
                    ],
                );
                Ok(LocalValue::Value(SpirvValue {
                    id: result_id,
                    ty: result_mir,
                }))
            }
            // sample_grad:显式梯度 → OpImageSampleExplicitLod(Grad ddx, ddy)。
            M::SampleGrad => {
                if extra.len() != 2 {
                    return Err(DxilError::sample_unsupported(
                        "sample_grad 需 ddx/ddy 两个 vec2<f32> 梯度".to_owned(),
                    ));
                }
                let ddx = self.lower_operand_value(b, &extra[0], None)?;
                let ddy = self.lower_operand_value(b, &extra[1], None)?;
                for g in [&ddx, &ddy] {
                    if g.ty != MirIoType::Vector(PrimTy::F32, 2) {
                        return Err(DxilError::sample_unsupported(
                            "sample_grad 梯度须 vec2<f32>".to_owned(),
                        ));
                    }
                }
                let (result_ty, result_mir) = self.vec4f(b)?;
                let result_id = b.alloc_id();
                Builder::emit(
                    &mut self.ops,
                    OP_IMAGE_SAMPLE_EXPLICIT_LOD,
                    &[
                        result_ty,
                        result_id,
                        si_id,
                        coord_val.id,
                        IMAGE_OPERANDS_GRAD,
                        ddx.id,
                        ddy.id,
                    ],
                );
                Ok(LocalValue::Value(SpirvValue {
                    id: result_id,
                    ty: result_mir,
                }))
            }
            // sample_cmp:恒显式 LOD 0 比较采样(shadow)→ OpImageSampleDrefExplicitLod。结果标量 f32。
            M::SampleCmp => {
                let dref = self.f32_operand(b, extra, "sample_cmp 缺 dref:f32")?;
                let f32_ty = b.scalar_type(PrimTy::F32)?;
                let lod0 = self.const_f32(b, 0.0);
                let result_id = b.alloc_id();
                Builder::emit(
                    &mut self.ops,
                    OP_IMAGE_SAMPLE_DREF_EXPLICIT_LOD,
                    &[
                        f32_ty,
                        result_id,
                        si_id,
                        coord_val.id,
                        dref,
                        IMAGE_OPERANDS_LOD,
                        lod0,
                    ],
                );
                Ok(LocalValue::Value(SpirvValue {
                    id: result_id,
                    ty: MirIoType::Scalar(PrimTy::F32),
                }))
            }
            // gather:基层 2×2 单分量聚合 → OpImageGather(component 0..=3 常量)。
            M::Gather => {
                let comp = self.gather_component(b, extra)?;
                let (result_ty, result_mir) = self.vec4f(b)?;
                let result_id = b.alloc_id();
                Builder::emit(
                    &mut self.ops,
                    OP_IMAGE_GATHER,
                    &[result_ty, result_id, si_id, coord_val.id, comp],
                );
                Ok(LocalValue::Value(SpirvValue {
                    id: result_id,
                    ty: result_mir,
                }))
            }
            other => Err(DxilError::sample_unsupported(format!(
                "sample 族分发意外方法 {other:?}"
            ))),
        }
    }

    /// G3.4 bindless(RXS-0234;RFC-0013 §4.C3):从无界表 `tex`(`OpTypeRuntimeArray`)
    /// 按动态索引 `idx` 取元素 image 句柄。序列:① clamp `UMin(idx, table_len-1)`
    /// (`table_len` 经 push-constant 尾槽,RXS-0208/0235;越界索引结果**实现定义但有
    /// 界**,访问恒有界于已注册表段,无 UB 措辞);② `OpAccessChain`(runtime array
    /// 元素指针)+ `NonUniform` 装饰;③ `OpLoad` image + `NonUniform`。**不物化中间
    /// 句柄 local**(镜像 RXS-0175 内联形态)。返回加载出的 image result-id。
    fn lower_bindless_image_load(
        &mut self,
        b: &mut Builder,
        tex: &ResourceVarInfo,
        idx: &Operand,
    ) -> Result<u32, DxilError> {
        // 动态索引值(u32;typeck 已 demand u32,RXS-0232)。
        let idx_val = self.lower_operand_value(b, idx, None)?;
        let u32_ty = b.scalar_type(PrimTy::U32)?;

        // clamp 上界 = table_len - 1(table_len 经 push-constant 尾槽成员 0,RXS-0208/0235)。
        let pc_var = b.push_constant_table_len_var();
        let pc_ptr = b.pointer_type(STORAGE_PUSH_CONSTANT, u32_ty);
        let zero = self.const_u32(b, 0);
        let len_ptr = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_ACCESS_CHAIN,
            &[pc_ptr, len_ptr, pc_var, zero],
        );
        let len_id = b.alloc_id();
        Builder::emit(&mut self.ops, OP_LOAD, &[u32_ty, len_id, len_ptr]);
        let one = self.const_u32(b, 1);
        let len_minus1 = b.alloc_id();
        Builder::emit(&mut self.ops, OP_ISUB, &[u32_ty, len_minus1, len_id, one]);
        let glsl = b.glsl_ext_inst();
        let clamped = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_EXT_INST,
            &[
                u32_ty,
                clamped,
                glsl,
                GLSL_STD_450_UMIN,
                idx_val.id,
                len_minus1,
            ],
        );

        // OpAccessChain(runtime array 元素指针)→ OpLoad(image)。NonUniform 装饰
        // 施于访问链指针与加载对象(descriptor indexing 波内正确采样,RXS-0234;
        // Q-B-Uniformity 保守全标合法)。
        let elem_ptr_ty = b.pointer_type(STORAGE_UNIFORM_CONSTANT, tex.type_id);
        let chain = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_ACCESS_CHAIN,
            &[elem_ptr_ty, chain, tex.var_id, clamped],
        );
        Builder::emit(
            &mut b.decorations,
            OP_DECORATE,
            &[chain, DECORATION_NON_UNIFORM],
        );
        let img_id = b.alloc_id();
        Builder::emit(&mut self.ops, OP_LOAD, &[tex.type_id, img_id, chain]);
        Builder::emit(
            &mut b.decorations,
            OP_DECORATE,
            &[img_id, DECORATION_NON_UNIFORM],
        );
        Ok(img_id)
    }

    /// texel fetch(load/load_lod)lowering:`OpImageFetch` + 越界钳制序列
    /// (`OpImageQuerySizeLod` → `OpISub`(size-1)→ GLSL.std.450 `UMin`,RXS-0228)。
    fn lower_texel_fetch(
        &mut self,
        b: &mut Builder,
        method: ResourceMethod,
        texture_local: u32,
        coord: &Operand,
        extra: &[Operand],
    ) -> Result<SpirvValue, DxilError> {
        // coord 须为 vec2<u32>(非归一化整型纹素坐标,RXS-0228)。
        let coord_val = self.lower_operand_value(b, coord, None)?;
        if coord_val.ty != MirIoType::Vector(PrimTy::U32, 2) {
            return Err(DxilError::sample_unsupported(format!(
                "texel fetch 坐标类型 {:?} 非 vec2<u32>(RXS-0228)",
                coord_val.ty
            )));
        }
        let tex = self.resource_for_local(texture_local)?;
        if tex.kind != ResourceVarKind::SampledImage {
            return Err(DxilError::sample_unsupported(format!(
                "texel fetch receiver `{}` 非 Texture2D 采样图像句柄",
                tex.name
            )));
        }
        // lod:load = 0、load_lod = extra[0]:u32(RXS-0228)。
        let lod_id = if method == ResourceMethod::LoadLod {
            self.u32_operand(b, extra, "load_lod 缺 lod:u32")?
        } else {
            self.const_u32(b, 0)
        };

        let img_id = b.alloc_id();
        Builder::emit(&mut self.ops, OP_LOAD, &[tex.type_id, img_id, tex.var_id]);

        // 越界钳制序列 min(coord, size-1)(RXS-0228 DS2;两后端同源确定性,零 feature 依赖)。
        b.add_capability(CAP_IMAGE_QUERY);
        let uvec2_ty = b.value_type(MirIoType::Vector(PrimTy::U32, 2))?;
        let size_id = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_IMAGE_QUERY_SIZE_LOD,
            &[uvec2_ty, size_id, img_id, lod_id],
        );
        let u32_ty = b.scalar_type(PrimTy::U32)?;
        let one = self.const_u32(b, 1);
        let ones = b.const_uvec2_splat(one, uvec2_ty);
        let _ = u32_ty;
        let size_minus1 = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_ISUB,
            &[uvec2_ty, size_minus1, size_id, ones],
        );
        let glsl = b.glsl_ext_inst();
        let clamped = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_EXT_INST,
            &[
                uvec2_ty,
                clamped,
                glsl,
                GLSL_STD_450_UMIN,
                coord_val.id,
                size_minus1,
            ],
        );

        // OpImageFetch(image, clamped_coord, Lod, lod)。
        let (result_ty, result_mir) = self.vec4_of(b, tex.sampled_prim)?;
        let result_id = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_IMAGE_FETCH,
            &[
                result_ty,
                result_id,
                img_id,
                clamped,
                IMAGE_OPERANDS_LOD,
                lod_id,
            ],
        );
        Ok(SpirvValue {
            id: result_id,
            ty: result_mir,
        })
    }

    /// storage image 读(TextureRw2D.load)→ `OpImageRead`(RXS-0226/0229)。
    fn lower_storage_load(
        &mut self,
        b: &mut Builder,
        texture_local: u32,
        coord: &Operand,
    ) -> Result<SpirvValue, DxilError> {
        let coord_val = self.lower_operand_value(b, coord, None)?;
        if coord_val.ty != MirIoType::Vector(PrimTy::U32, 2) {
            return Err(DxilError::sample_unsupported(format!(
                "storage image load 坐标 {:?} 非 vec2<u32>(RXS-0228)",
                coord_val.ty
            )));
        }
        let tex = self.resource_for_local(texture_local)?;
        if tex.kind != ResourceVarKind::StorageImage {
            return Err(DxilError::sample_unsupported(format!(
                "storage image load receiver `{}` 非 TextureRw2D storage image",
                tex.name
            )));
        }
        let img_id = b.alloc_id();
        Builder::emit(&mut self.ops, OP_LOAD, &[tex.type_id, img_id, tex.var_id]);
        let (result_ty, result_mir) = self.vec4_of(b, tex.sampled_prim)?;
        let result_id = b.alloc_id();
        Builder::emit(
            &mut self.ops,
            OP_IMAGE_READ,
            &[result_ty, result_id, img_id, coord_val.id],
        );
        Ok(SpirvValue {
            id: result_id,
            ty: result_mir,
        })
    }

    /// storage image 写(TextureRw2D.store)→ `OpImageWrite`(唯一写者纪律,RXS-0229)。
    /// **唯一写者 codegen 强制**:store 坐标须为本 invocation 位置标识 identity 映射
    /// (frag_coord / launch_id);非 identity、可产生多写者的坐标派生 → strict-only 拒。
    fn lower_storage_store(
        &mut self,
        b: &mut Builder,
        texture_local: u32,
        coord: &Operand,
        extra: &[Operand],
    ) -> Result<(), DxilError> {
        // RXS-0229 IR1:唯一写者纪律——store 坐标须直接来自本 invocation 位置标识
        // (identity 映射),不做别名分析,保守近似:仅接受直接读自输入 I/O place 的
        // 坐标(frag_coord 派生 / launch_id 派生);常量 / 算术派生坐标可产生多写者 → 拒。
        if !self.is_identity_coord(coord) {
            return Err(DxilError::sample_unsupported(
                "TextureRw2D.store 坐标非本 invocation 位置标识 identity 映射(唯一写者纪律,\
                 RXS-0229;可竞写模式登 RD-034+ 另 Full RFC)"
                    .to_owned(),
            ));
        }
        let coord_val = self.lower_operand_value(b, coord, None)?;
        if coord_val.ty != MirIoType::Vector(PrimTy::U32, 2) {
            return Err(DxilError::sample_unsupported(format!(
                "storage image store 坐标 {:?} 非 vec2<u32>(RXS-0228)",
                coord_val.ty
            )));
        }
        let tex = self.resource_for_local(texture_local)?;
        if tex.kind != ResourceVarKind::StorageImage {
            return Err(DxilError::sample_unsupported(format!(
                "storage image store receiver `{}` 非 TextureRw2D storage image",
                tex.name
            )));
        }
        if extra.len() != 1 {
            return Err(DxilError::sample_unsupported(
                "TextureRw2D.store 需 value:vec4<F> 一个写入值".to_owned(),
            ));
        }
        let value = self.lower_operand_value(b, &extra[0], None)?;
        if value.ty != MirIoType::Vector(tex.sampled_prim, 4) {
            return Err(DxilError::sample_unsupported(format!(
                "storage image store 值类型 {:?} 与纹理分量 vec4<{:?}> 不符",
                value.ty, tex.sampled_prim
            )));
        }
        let img_id = b.alloc_id();
        Builder::emit(&mut self.ops, OP_LOAD, &[tex.type_id, img_id, tex.var_id]);
        // OpImageWrite(image, coord, value):无结果 id。
        Builder::emit(
            &mut self.ops,
            OP_IMAGE_WRITE,
            &[img_id, coord_val.id, value.id],
        );
        Ok(())
    }

    /// 唯一写者纪律保守近似(RXS-0229 IR1):坐标是否为本 invocation 位置标识 identity
    /// 映射。保守判据:坐标 operand 直接读自**输入 I/O place**(frag_coord / launch_id
    /// 派生形参字段),不做别名分析;常量 / 算术派生 → 非 identity(可产生多写者)→ 拒。
    fn is_identity_coord(&self, coord: &Operand) -> bool {
        match coord {
            // 直接读输入 I/O 形参字段(frag_coord / launch_id 位置标识)= identity。
            Operand::Copy(p) | Operand::Move(p) => {
                p.local.0 >= 1 && (p.local.0 as usize) <= self.body.arg_count
            }
            // 常量坐标 → 所有 invocation 写同一 texel = 多写者 → 非 identity。
            Operand::Const(_) => false,
        }
    }

    /// LOD operand(sample_lod):空 extra = 显式 LOD 0 常量(既有 `.sample()` byte-preserving);
    /// 否则 extra[0]:f32(RXS-0223 Q-S-SampleName)。
    fn lod_operand(&mut self, b: &mut Builder, extra: &[Operand]) -> Result<u32, DxilError> {
        if extra.is_empty() {
            // 显式 LOD 0 常量(既有路承接,RFC-0007 §4.6)。
            let f32_ty = b.scalar_type(PrimTy::F32)?;
            let lod0 = b.alloc_id();
            Builder::emit(&mut b.types, OP_CONSTANT, &[f32_ty, lod0, 0.0f32.to_bits()]);
            Ok(lod0)
        } else {
            self.f32_operand(b, extra, "sample_lod 缺 lod:f32")
        }
    }

    /// f32 标量 operand(extra[0]),类型校验。
    fn f32_operand(
        &mut self,
        b: &mut Builder,
        extra: &[Operand],
        msg: &str,
    ) -> Result<u32, DxilError> {
        let op = extra
            .first()
            .ok_or_else(|| DxilError::sample_unsupported(msg.to_owned()))?;
        let v = self.lower_operand_value(b, op, Some(MirIoType::Scalar(PrimTy::F32)))?;
        Ok(v.id)
    }

    /// u32 标量 operand(extra[0]),类型校验。
    fn u32_operand(
        &mut self,
        b: &mut Builder,
        extra: &[Operand],
        msg: &str,
    ) -> Result<u32, DxilError> {
        let op = extra
            .first()
            .ok_or_else(|| DxilError::sample_unsupported(msg.to_owned()))?;
        let v = self.lower_operand_value(b, op, Some(MirIoType::Scalar(PrimTy::U32)))?;
        Ok(v.id)
    }

    /// gather component(0..=3 字面量常量,RXS-0223)。
    fn gather_component(&mut self, b: &mut Builder, extra: &[Operand]) -> Result<u32, DxilError> {
        let op = extra.first().ok_or_else(|| {
            DxilError::sample_unsupported("gather 缺 component 字面量".to_owned())
        })?;
        match op {
            Operand::Const(Const::Int(v, _)) if (0..=3).contains(v) => {
                Ok(self.const_u32(b, *v as u32))
            }
            _ => Err(DxilError::sample_unsupported(
                "gather component 须 0..=3 字面量常量".to_owned(),
            )),
        }
    }

    /// vec4<f32> 类型 id + MirIoType(sample 族结果)。
    fn vec4f(&mut self, b: &mut Builder) -> Result<(u32, MirIoType), DxilError> {
        self.vec4_of(b, PrimTy::F32)
    }

    /// vec4<prim> 类型 id + MirIoType。
    fn vec4_of(&mut self, b: &mut Builder, prim: PrimTy) -> Result<(u32, MirIoType), DxilError> {
        let mir = MirIoType::Vector(prim, 4);
        Ok((b.value_type(mir)?, mir))
    }

    /// f32 常量 id(去缓存,直接 emit;顺序敏感处调用)。
    fn const_f32(&mut self, b: &mut Builder, v: f32) -> u32 {
        let f32_ty = b.scalar_type(PrimTy::F32).expect("f32 scalar type");
        let id = b.alloc_id();
        Builder::emit(&mut b.types, OP_CONSTANT, &[f32_ty, id, v.to_bits()]);
        id
    }

    /// u32 常量 id。
    fn const_u32(&mut self, b: &mut Builder, v: u32) -> u32 {
        let u32_ty = b.scalar_type(PrimTy::U32).expect("u32 scalar type");
        let id = b.alloc_id();
        Builder::emit(&mut b.types, OP_CONSTANT, &[u32_ty, id, v]);
        id
    }

    fn load_input_field(&mut self, b: &mut Builder, field: usize) -> Result<SpirvValue, DxilError> {
        let var = self.input_vars.get(field).copied().ok_or_else(|| {
            DxilError::unmappable("input-field", format!("输入 I/O 字段 {field} 越界"))
        })?;
        let ty_id = b.value_type(var.ty)?;
        let id = b.alloc_id();
        Builder::emit(&mut self.ops, OP_LOAD, &[ty_id, id, var.var_id]);
        Ok(SpirvValue { id, ty: var.ty })
    }

    fn store_return_value(&mut self, value: LocalValue) -> Result<(), DxilError> {
        match value {
            LocalValue::Unit if self.output_vars.is_empty() => Ok(()),
            LocalValue::Aggregate(fields) => self.store_output_aggregate(&fields),
            LocalValue::Value(v) if self.output_vars.len() == 1 => self.store_output(0, v),
            LocalValue::Unit => Err(DxilError::unmappable(
                "output-return",
                "声明了 Output I/O 时不能返回 unit",
            )),
            LocalValue::Value(_) => Err(DxilError::unmappable(
                "output-return",
                "多字段 Output I/O 必须以输出结构体聚合返回",
            )),
        }
    }

    fn store_output_aggregate(&mut self, fields: &[SpirvValue]) -> Result<(), DxilError> {
        if fields.len() != self.output_vars.len() {
            return Err(DxilError::unmappable(
                "output-return",
                format!(
                    "输出聚合字段数 {} 与 Output I/O 元素数 {} 不一致",
                    fields.len(),
                    self.output_vars.len()
                ),
            ));
        }
        for (idx, value) in fields.iter().copied().enumerate() {
            self.store_output(idx, value)?;
        }
        Ok(())
    }

    fn store_output(&mut self, index: usize, value: SpirvValue) -> Result<(), DxilError> {
        let out = self.output_vars.get(index).copied().ok_or_else(|| {
            DxilError::unmappable("output-field", format!("输出 I/O 字段 {index} 越界"))
        })?;
        if out.ty != value.ty {
            return Err(DxilError::unmappable(
                "output-type",
                format!(
                    "输出字段 {index} 类型 {:?} 与值类型 {:?} 不符",
                    out.ty, value.ty
                ),
            ));
        }
        Builder::emit(&mut self.ops, OP_STORE, &[out.var_id, value.id]);
        if let Some(w) = self.output_written.get_mut(index) {
            *w = true;
        }
        Ok(())
    }

    fn output_ty(&self, index: usize) -> Result<MirIoType, DxilError> {
        self.output_vars
            .get(index)
            .map(|v| v.ty)
            .ok_or_else(|| DxilError::unmappable("output-field", format!("字段 {index} 越界")))
    }
}

fn single_field_projection(place: &Place) -> Result<Option<usize>, DxilError> {
    match place.proj.as_slice() {
        [] => Ok(None),
        [ProjElem::Field(idx)] => Ok(Some(*idx as usize)),
        _ => Err(DxilError::unmappable(
            "place-projection",
            format!("RXS-0171 最小切片仅支持单层 Field 投影, 实得 {place:?}"),
        )),
    }
}

fn mir_io_prim(ty: MirIoType) -> PrimTy {
    match ty {
        MirIoType::Scalar(p) | MirIoType::Vector(p, _) => p,
    }
}

/// 把一个着色阶段(`stage`)与其 I/O 意图签名(`io_sig`)编码为合法 SPIR-V
/// 二进制字流(`Vec<u32>`)。
///
/// 覆盖 vertex/fragment 最小子集(R1.4~R1.7):header + `Capability Shader` +
/// `OpMemoryModel(Logical, GLSL450)` + `OpEntryPoint` + `OpExecutionMode`
/// (fragment `OriginUpperLeft`)+ 按需类型指令 + Input/Output 变量 +
/// `Location`/`BuiltIn` 装饰 + `UserSemantic` 保名 + 平凡 passthrough `main`。
///
/// # Errors
/// 遇最小子集外构造(非 vertex·fragment 阶段、不可映射类型、未建模 builtin 名、
/// builtin 类型不符、越界向量宽度)→ [`DxilError::Unmappable`](strict-only,
/// **不**静默产出降级 SPIR-V,R1.9)。
///
/// 注:本函数接 `stage + &[IoSigElem] + &[ResourceBinding]`(均为公开类型);
/// 资源句柄绑定由 host 侧 [`binding_layout::infer_spirv_bindings`] 确定性推导出
/// `DescriptorSet`/`Binding`,本编码器机械落对应装饰(PR-E2b 生产接线,RXS-0163)。
pub fn emit_spirv(
    stage: ShaderStage,
    io_sig: &[IoSigElem],
    resources: &[ResourceBinding],
) -> Result<Vec<u32>, DxilError> {
    emit_spirv_inner(stage, io_sig, resources, None, /*provenance=*/ true)
}

/// 把完整图形着色阶段 [`Body`] 编码为 SPIR-V。相较 [`emit_spirv`] 的签名-only
/// 兼容入口,本函数按 RXS-0171 降级最小 body 数据流:Input place → `OpLoad`,
/// f32/i32/u32 常量 → `OpConstant`,白名单算术 → SPIR-V 算术 op,输出 I/O 聚合返回
/// → 逐 Output 元素 `OpStore`。
pub fn emit_spirv_body(stage: ShaderStage, body: &Body) -> Result<Vec<u32>, DxilError> {
    emit_spirv_inner(
        stage,
        &body.io_sig,
        &body.resources,
        Some(body),
        /*provenance=*/ true,
    )
}

/// Vulkan 原生消费入口(RXS-0210 + G7.5b RXS-0301 两遍编译):与 [`emit_spirv_body`]
/// 同 io/资源面,但 **不 emit** provenance 装饰(`UserSemantic` →
/// `SPV_GOOGLE_hlsl_functionality1`)。保名仅 B 路 SPIRV-Cross→HLSL→dxc 边界需要
/// (Vulkan 原生按 `Location`/`BuiltIn` 消费,永不需要);去装饰后 `.spv` 对所有
/// Vulkan ICD 零扩展依赖直喂 `vkCreateShaderModule`(免 `VK_GOOGLE_hlsl_functionality1`,
/// VUID-...-08742)。DXIL 路(`emit_spirv_body`,provenance=true)保名字节不变、零回归。
///
/// **两遍编译(RXS-0301 Dynamic Semantics,RFC-0018 §E3 授权)**:
/// - 第一遍 = 现行最小 [`BodyLowerer`](RXS-0171 L4 白名单):成功即**原样输出**——
///   既有全部图形 accept 语料/RHI demo 不经任何新代码,输出字节零漂移 by construction;
/// - 第一遍 [`DxilError::Unmappable`] 且 provenance=false → 第二遍
///   [`ext::emit_spirv_ext`](RXS-0301 扩展白名单,全新 Builder 纯函数式重发射;
///   仅 feature `vulkan-backend` 编入——DXIL-only 构建维持单遍,`--target vulkan`
///   本就不可达);
/// - 第二遍仍失败 → 错误上抛,driver/`build_and_emit_vulkan` 落 **RX6026**
///   (诊断收窄为 RXS-0301 L3 负面清单命中项)。
///
/// `res`:名称解析产物(扩展遍的形参分类事实源——`View`/`ViewMut`/`AtomicView`
/// lang item 判定,RXS-0302 L1;第一遍不消费)。
pub fn emit_spirv_body_vulkan(
    stage: ShaderStage,
    body: &Body,
    res: &crate::resolve::Resolutions,
) -> Result<Vec<u32>, DxilError> {
    let first = emit_spirv_inner(
        stage,
        &body.io_sig,
        &body.resources,
        Some(body),
        /*provenance=*/ false,
    );
    #[cfg(feature = "vulkan-backend")]
    {
        match first {
            // 仅 Unmappable 触发第二遍(SampleUnsupported 维持第一遍采样子集语义原样上抛)。
            Err(DxilError::Unmappable { .. }) => ext::emit_spirv_ext(stage, body, res),
            other => other,
        }
    }
    #[cfg(not(feature = "vulkan-backend"))]
    {
        let _ = res;
        first
    }
}

fn emit_spirv_inner(
    stage: ShaderStage,
    io_sig: &[IoSigElem],
    resources: &[ResourceBinding],
    body: Option<&Body>,
    provenance: bool,
) -> Result<Vec<u32>, DxilError> {
    // 仅 vertex/fragment 走 B 路最小子集;compute 走既有 A 路、mesh/task/RT 为
    // STUB(RD-012),均不在本编码器范围 → 不可映射(strict-only)。
    let exec_model = match stage {
        ShaderStage::Vertex => EXEC_MODEL_VERTEX,
        ShaderStage::Fragment => EXEC_MODEL_FRAGMENT,
        other => {
            return Err(DxilError::unmappable(
                "stage",
                format!("着色阶段 {other:?} 不在 B 路编码器最小子集(vertex/fragment)内"),
            ));
        }
    };

    let mut b = Builder::new();
    // provenance 路由(RXS-0210):DXIL 路 true(保名字节不变)/ Vulkan 原生路 false
    // (去 UserSemantic → OpExtension SPV_GOOGLE 自然不 emit)。
    b.emit_provenance = provenance;

    // void 与 fn 类型(`void()`)先于一切(供 OpFunction 引用)。
    let void_id = b.alloc_id();
    Builder::emit(&mut b.types, OP_TYPE_VOID, &[void_id]);
    let fn_type_id = b.alloc_id();
    Builder::emit(&mut b.types, OP_TYPE_FUNCTION, &[fn_type_id, void_id]);

    // 逐 I/O 元素:类型/指针/变量/装饰/接口登记。
    let mut io_vars = Vec::with_capacity(io_sig.len());
    for elem in io_sig {
        io_vars.push(b.emit_io_elem(elem, stage)?);
    }

    // 资源句柄绑定(RXS-0163;PR-E2b 生产接线):host 侧确定性推导
    // `DescriptorSet`/`Binding`(按声明序),逐资源 emit opaque 类型 + 变量 + 装饰。
    // bindless / unbounded → `BindingInferError::Unmappable` → 透传 `DxilError::Unmappable`
    // (strict-only,RD-018,不发明 descriptor heap 编码)。
    //
    // 绑定 set 装饰按目标选择两套策略(RXS-0230/E-3,RFC-0013 §4.B7,provenance 旗标承载):
    // DXIL/B 链路(provenance=true)维持硬编码 set0 装饰**字节不动**(零 golden 重 bless);
    // Vulkan 原生路(provenance=false,`emit_spirv_body_vulkan` / `--target vulkan`)切
    // Vk-native set-per-class(0=CBV/1=SRV/2=UAV/3=Sampler),令原生 vkCreateShaderModule
    // 消费下四类轴各占独立 set,与 `run_graphics_offscreen_v2` 的 `plan_descriptor_sets`
    // (rurix-rt 侧镜像同一分配律)对齐。**binding 号两策略同一事实源**(per-class 递增),
    // 仅 set 分配策略切换——非「一处推导两形态」的含糊。
    let spirv_bindings = if provenance {
        binding_layout::infer_spirv_bindings(resources)
    } else {
        binding_layout::infer_spirv_bindings_vk_native(resources)
    }
    .map_err(map_binding_err)?;
    for (res, b_intent) in resources.iter().zip(spirv_bindings.iter()) {
        b.emit_resource(res, b_intent.set, b_intent.binding)?;
    }

    // 入口函数与首基本块 id(forward-ref:OpEntryPoint/OpExecutionMode 先于定义引用)。
    let main_id = b.alloc_id();
    let label_id = b.alloc_id();
    let body_ops = match body {
        Some(body) => BodyLowerer::new(body, &io_vars, b.resource_vars.clone()).lower(&mut b)?,
        None => Vec::new(),
    };

    // ── 组装最终模块(严格遵守 SPIR-V 逻辑分节序) ──
    let mut module: Vec<u32> = Vec::new();

    // 1) header(bound 末填)。
    module.push(SPIRV_MAGIC);
    module.push(SPIRV_VERSION_1_0);
    module.push(SPIRV_GENERATOR);
    let bound_index = module.len();
    module.push(0); // bound 占位,最后回填。
    module.push(SPIRV_SCHEMA);

    // 2) capability(Shader + 方法族追加能力,如 texel fetch 的 ImageQuery,RXS-0228)。
    Builder::emit(&mut module, OP_CAPABILITY, &[CAP_SHADER]);
    for &cap in &b.extra_caps {
        Builder::emit(&mut module, OP_CAPABILITY, &[cap]);
    }

    // 3) extension(仅当用到 UserSemantic 保名)。
    if b.used_user_semantic {
        let mut operands = Vec::new();
        Builder::push_string(&mut operands, EXT_HLSL_FUNCTIONALITY1);
        Builder::emit(&mut module, OP_EXTENSION, &operands);
    }
    // 3.1) G3.4 bindless(RXS-0234):descriptor indexing 扩展(Vulkan 1.2 core;
    //      RuntimeDescriptorArray/ShaderNonUniform capability 走 extra_caps 段)。
    if b.used_descriptor_indexing {
        let mut operands = Vec::new();
        Builder::push_string(&mut operands, EXT_DESCRIPTOR_INDEXING);
        Builder::emit(&mut module, OP_EXTENSION, &operands);
    }

    // 3.5) ext-inst import(GLSL.std.450,texel fetch 越界钳制 UMin;须在 memory model 前)。
    module.extend_from_slice(&b.ext_imports);

    // 4) memory model。
    Builder::emit(
        &mut module,
        OP_MEMORY_MODEL,
        &[ADDR_MODEL_LOGICAL, MEM_MODEL_GLSL450],
    );

    // 5) entry point:execution model + main + "main" + 接口变量 id 列表。
    {
        let mut operands = vec![exec_model, main_id];
        Builder::push_string(&mut operands, "main");
        operands.extend_from_slice(&b.interface);
        Builder::emit(&mut module, OP_ENTRY_POINT, &operands);
    }

    // 6) execution mode:fragment 至少 OriginUpperLeft。
    if stage == ShaderStage::Fragment {
        Builder::emit(
            &mut module,
            OP_EXECUTION_MODE,
            &[main_id, EXEC_MODE_ORIGIN_UPPER_LEFT],
        );
    }

    // 7) 注解(decorations)。
    module.extend_from_slice(&b.decorations);

    // 8) 类型/常量/全局变量(类型先于变量,依赖序已在构造时保证)。
    module.extend_from_slice(&b.types);
    module.extend_from_slice(&b.variables);

    // 9) main:body-aware 入口会先 emit 降级后的 OpLoad/OpStore/算术;签名-only
    //    兼容入口保持平凡 OpReturn。
    Builder::emit(
        &mut module,
        OP_FUNCTION,
        &[void_id, main_id, FUNCTION_CONTROL_NONE, fn_type_id],
    );
    Builder::emit(&mut module, OP_LABEL, &[label_id]);
    module.extend_from_slice(&body_ops);
    Builder::emit(&mut module, OP_RETURN, &[]);
    Builder::emit(&mut module, OP_FUNCTION_END, &[]);

    // 10) 回填 bound = 末 id + 1(已分配 id 范围 1..next_id)。
    module[bound_index] = b.next_id;

    Ok(module)
}

// ───────────────── 第二遍:ExtendedBodyLowerer(G7.5b,RXS-0301~0303) ─────────────────

/// Vulkan 原生图形 body 扩展 lowerer(**第二遍**;RXS-0301 白名单,RFC-0018 §E3 授权;
/// 仅 feature `vulkan-backend` 编入——第一遍 [`BodyLowerer`] 与 DXIL 路 0-byte 不经此处)。
///
/// 结构镜像 [`crate::vulkan_codegen`] compute 路成熟做法(RXS-0301 IR 逐字):
/// - **内存式 local**:MIR 预扫描判「多次赋值或跨分支活跃」→ Function 存储
///   `OpVariable` + Load/Store;单赋值直线 local 维持 SSA 值;
/// - **结构化 `if`**:`SwitchBool` → `OpSelectionMerge` + 条件分支,merge 复用
///   [`crate::vulkan_codegen::structured_merge`] 前向可达交汇算法;**循环不实现**
///   (回边预扫描恒拒,RXS-0301 L3 负面清单);
/// - **表级复用**:[`crate::vulkan_codegen::binop_opcode`](含比较/位运算)/
///   [`crate::vulkan_codegen::cast_opcode`] / [`crate::vulkan_codegen::glsl_ext_op`]
///   (round→RoundEven)与原子发射段常量(`OP_ATOMIC_UMAX` + Device scope + Relaxed
///   semantics + `CAP_INT64`/`CAP_INT64_ATOMICS`)与 compute 路**逐字同值**双路共享
///   (仅可见性提升,compute 发射序 0-byte,W1/W2 manifest 零漂移门维持);
/// - **资源分类**(RXS-0302 L1,与 compute 同一分配律):图形入口签名中
///   `View`/`ViewMut`/`AtomicView<global,..>` buffer 形参按声明序 → `set=0, binding=n`
///   (StorageBuffer,std430,SPIR-V 1.0 形态 = `BufferBlock`+`Uniform`);标量形参按
///   声明序聚合单 push constant 块(Offset 4 字节对齐顺排;u64 标量 8 字节对齐同
///   compute `prim_layout` 律);
/// - **SPIR-V 版本恒 1.0**(RXS-0302 L4);capability 按需:u64 类型 → `Int64`,
///   u64 原子 → 追加 `Int64Atomics`(不用不发)。
#[cfg(feature = "vulkan-backend")]
mod ext {
    use super::*;
    use crate::hir::AtomicOp;
    use crate::mir::CallTarget;
    use crate::resolve::Resolutions;
    use crate::ty::Ty;
    use crate::vulkan_codegen::{
        CAP_INT64, CAP_INT64_ATOMICS, MEM_SEM_RELAXED, OP_ATOMIC_UMAX, SCOPE_DEVICE, binop_opcode,
        block_succs, cast_opcode, glsl_ext_op, structured_merge,
    };

    // 本遍新增消费的 SPIR-V core 常量(第一遍未触及的枚举/opcode;取值同
    // vulkan_codegen 同名常量,SPIR-V core 规范数值)。
    const OP_TYPE_BOOL: u16 = 20;
    const OP_COMPOSITE_CONSTRUCT: u16 = 80;
    const OP_COMPOSITE_EXTRACT: u16 = 81;
    const OP_SELECT: u16 = 169;
    const OP_INOTEQUAL: u16 = 171;
    const OP_SELECTION_MERGE: u16 = 247;
    const OP_BRANCH: u16 = 249;
    const OP_BRANCH_CONDITIONAL: u16 = 250;
    const OP_UNREACHABLE: u16 = 255;
    const STORAGE_FUNCTION: u32 = 7;
    const STORAGE_UNIFORM: u32 = 2;
    const DECORATION_ARRAY_STRIDE: u32 = 6;
    const DECORATION_BUFFER_BLOCK: u32 = 3;
    const SELECTION_CONTROL_NONE: u32 = 0;

    /// 值形状(SPIR-V 已建模标量/向量;prim 恒为**归一化**后的建模集
    /// {F32,I32,U32,I64,U64,} —— `Usize/U16/U8/Bool → U32`、`I16/I8 → I32`,
    /// 与 compute 路 `prim_type` 的 32 位收编同律)。
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum EKind {
        Scalar(PrimTy),
        Vector(PrimTy, u8),
    }

    /// 已物化 SPIR-V 值。
    #[derive(Clone, Copy, Debug)]
    struct EVal {
        id: u32,
        kind: EKind,
    }

    /// 求值结果(unit / 标量·向量值 / 输出结构体聚合)。
    #[derive(Clone, Debug)]
    enum EOperand {
        Unit,
        Value(EVal),
        Agg(Vec<EVal>),
    }

    /// 每个 MIR local 的降级槽位(预扫描 + 形参分类产物)。
    #[derive(Clone, Debug)]
    enum Slot {
        /// 返回槽 `_0`(输出装配面)。
        Ret,
        /// 唯一 IO 结构体形参(Field 投影 → Input 变量)。
        IoStruct,
        /// buffer 形参(`View`/`ViewMut`/`AtomicView<global,..>` → SSBO 描述符)。
        Buffer { var: u32, elem: PrimTy },
        /// 纹理/采样器句柄形参(资源变量由第一遍同款 `emit_resource` 发射;
        /// body 内非采样引用恒拒)。
        Resource,
        /// 内存式 local(Function 存储 `OpVariable`)。
        Var { var: u32, kind: EKind },
        /// 单赋值直线 local(SSA 值,赋值时物化进 `ssa` 表)。
        Ssa,
        /// 输出结构体聚合临时(字段值向量,赋值时物化进 `agg` 表)。
        Agg,
        /// 零尺寸(unit / ThreadCtx):不物化。
        Zst,
    }

    /// 归一化到 SPIR-V 建模标量集(compute `prim_type` 32 位收编同律;bool 内存
    /// 以 u32(0/1) 表示,镜像 compute `OpSelect` 口径)。f64 → `None`(负面清单)。
    fn norm_prim(p: PrimTy) -> Option<PrimTy> {
        Some(match p {
            PrimTy::F32 => PrimTy::F32,
            PrimTy::I32 | PrimTy::I16 | PrimTy::I8 => PrimTy::I32,
            PrimTy::U32 | PrimTy::U16 | PrimTy::U8 | PrimTy::Usize | PrimTy::Bool => PrimTy::U32,
            PrimTy::I64 => PrimTy::I64,
            PrimTy::U64 => PrimTy::U64,
            _ => return None,
        })
    }

    fn is_signed(p: PrimTy) -> bool {
        matches!(p, PrimTy::I8 | PrimTy::I16 | PrimTy::I32 | PrimTy::I64)
    }

    /// MIR 值类型 → 形状(标量 / 2..=4 同型元组向量;归一化;其余 `None`)。
    fn ekind_of(ty: &Ty) -> Option<EKind> {
        match ty {
            Ty::Prim(p) => Some(EKind::Scalar(norm_prim(*p)?)),
            Ty::Tuple(el) if (2..=4).contains(&el.len()) => {
                let Some(Ty::Prim(first)) = el.first() else {
                    return None;
                };
                if !el.iter().all(|t| matches!(t, Ty::Prim(p) if p == first)) {
                    return None;
                }
                Some(EKind::Vector(norm_prim(*first)?, el.len() as u8))
            }
            _ => None,
        }
    }

    /// I/O 元素类型 → 形状(io 面已建模 f32/i32/u32 标量与向量,恒可归一)。
    fn ekind_of_io(ty: MirIoType) -> EKind {
        match ty {
            MirIoType::Scalar(p) => EKind::Scalar(norm_prim(p).unwrap_or(PrimTy::U32)),
            MirIoType::Vector(p, n) => EKind::Vector(norm_prim(p).unwrap_or(PrimTy::U32), n),
        }
    }

    fn unmappable(what: &str, detail: impl Into<String>) -> DxilError {
        DxilError::unmappable(what, detail)
    }

    /// CFG 环检测(迭代 DFS 三色法:灰点边 = 回边 = 循环)。后继表复用
    /// [`crate::vulkan_codegen::block_succs`](与 `structured_merge` 同源)。
    /// 不可达死块(return 降级产物)一并扫描(保守方向恒为拒)。
    fn has_cycle(body: &Body) -> bool {
        let n = body.blocks.len();
        let succs: Vec<Vec<usize>> = body.blocks.iter().map(block_succs).collect();
        // 0 = 未访问 / 1 = 在栈(灰) / 2 = 完成(黑)。
        let mut state = vec![0u8; n];
        let mut stack: Vec<(usize, usize)> = Vec::new();
        for start in 0..n {
            if state[start] != 0 {
                continue;
            }
            state[start] = 1;
            stack.push((start, 0));
            while let Some(top) = stack.last_mut() {
                let (node, i) = (top.0, top.1);
                if i < succs[node].len() {
                    top.1 += 1;
                    let s = succs[node][i];
                    match state[s] {
                        0 => {
                            state[s] = 1;
                            stack.push((s, 0));
                        }
                        1 => return true,
                        _ => {}
                    }
                } else {
                    state[node] = 2;
                    stack.pop();
                }
            }
        }
        false
    }

    /// 扩展遍入口(纯函数式:全新 [`Builder`] 重发射;失败原因 = RXS-0301 L3
    /// 负面清单命中或扩展白名单外构造,上层落 RX6026)。
    pub(super) fn emit_spirv_ext(
        stage: ShaderStage,
        body: &Body,
        res: &Resolutions,
    ) -> Result<Vec<u32>, DxilError> {
        let exec_model = match stage {
            ShaderStage::Vertex => EXEC_MODEL_VERTEX,
            ShaderStage::Fragment => EXEC_MODEL_FRAGMENT,
            other => {
                return Err(unmappable(
                    "ext-stage",
                    format!("着色阶段 {other:?} 不在图形扩展编码器(vertex/fragment)内"),
                ));
            }
        };
        // 负面清单预扫描(RXS-0301 L3):① CFG 环(DFS 灰点回边)= 循环,恒拒
        // (`while`/`for`/`loop` 的 MIR 降级必产回边;结构化 if 无环——MIR 块编号
        // 非拓扑序,不能按「目标下标 ≤ 源下标」误判,须按 DFS 祖先边判)。
        if has_cycle(body) {
            return Err(unmappable(
                "ext-negative-loop",
                "循环(while/for)在图形 body 恒拒(RXS-0301 L3 负面清单,\
                 拒绝面 grow-only;回边不构成结构化 if)",
            ));
        }
        // ② `shared` 存储(负面清单:共享内存——fragment/vertex 无 workgroup 语义)。
        if body.locals.iter().any(|l| l.shared) {
            return Err(unmappable(
                "ext-negative-shared",
                "共享内存(`shared`/Workgroup 存储)在图形 body 恒拒(RXS-0301 L3 负面清单)",
            ));
        }

        let mut b = Builder::new();
        b.emit_provenance = false;

        // void 与 fn 类型(供 OpFunction 引用;同第一遍序)。
        let void_id = b.alloc_id();
        Builder::emit(&mut b.types, OP_TYPE_VOID, &[void_id]);
        let fn_type_id = b.alloc_id();
        Builder::emit(&mut b.types, OP_TYPE_FUNCTION, &[fn_type_id, void_id]);

        // I/O 元素(与第一遍同一 `emit_io_elem`:builtin/Location/Flat 装饰同形)。
        let mut io_vars = Vec::with_capacity(body.io_sig.len());
        for elem in &body.io_sig {
            io_vars.push(b.emit_io_elem(elem, stage)?);
        }
        // 纹理/采样器句柄资源(与第一遍同一 vk-native 分配律;本遍 body 不放行采样,
        // 但资源面保持同形,签名合法性不因走第二遍而漂移)。
        let spirv_bindings = binding_layout::infer_spirv_bindings_vk_native(&body.resources)
            .map_err(map_binding_err)?;
        for (r, intent) in body.resources.iter().zip(spirv_bindings.iter()) {
            b.emit_resource(r, intent.set, intent.binding)?;
        }

        let main_id = b.alloc_id();
        let entry_label = b.alloc_id();

        let mut lo = ExtLowerer::new(body, res, &io_vars);
        lo.classify_params(&mut b)?;
        lo.prescan();
        lo.infer_kinds();
        lo.alloc_locals(&mut b)?;
        lo.emit_preamble(&mut b)?;
        lo.lower_blocks(&mut b)?;
        if !lo.output_vars.is_empty() && !lo.output_written.iter().all(|w| *w) {
            return Err(unmappable(
                "ext-output-return",
                "着色 body 未写出所有声明的 Output I/O 元素",
            ));
        }

        // ── 组装(SPIR-V 逻辑分节序;版本恒 1.0,RXS-0302 L4) ──
        let mut module: Vec<u32> = Vec::new();
        module.push(SPIRV_MAGIC);
        module.push(SPIRV_VERSION_1_0);
        module.push(SPIRV_GENERATOR);
        let bound_index = module.len();
        module.push(0);
        module.push(SPIRV_SCHEMA);

        // capability:基线 Shader;u64 类型 → Int64;u64 原子 → Int64Atomics
        // (按需声明,不用不发;常量与 compute 路逐字同值,RXS-0302 L3)。
        Builder::emit(&mut module, OP_CAPABILITY, &[CAP_SHADER]);
        if lo.uses_int64 {
            Builder::emit(&mut module, OP_CAPABILITY, &[CAP_INT64]);
        }
        if lo.uses_int64_atomics {
            Builder::emit(&mut module, OP_CAPABILITY, &[CAP_INT64_ATOMICS]);
        }
        for &cap in &b.extra_caps {
            Builder::emit(&mut module, OP_CAPABILITY, &[cap]);
        }
        // extension:provenance=false → 无 UserSemantic/SPV_GOOGLE;descriptor
        // indexing 面(无界表)与第一遍同形透传。
        if b.used_descriptor_indexing {
            let mut operands = Vec::new();
            Builder::push_string(&mut operands, EXT_DESCRIPTOR_INDEXING);
            Builder::emit(&mut module, OP_EXTENSION, &operands);
        }
        module.extend_from_slice(&b.ext_imports);
        Builder::emit(
            &mut module,
            OP_MEMORY_MODEL,
            &[ADDR_MODEL_LOGICAL, MEM_MODEL_GLSL450],
        );
        // entry point:interface 仅 Input/Output(SPIR-V 1.0;SSBO/push-constant
        // 变量不入 interface)。入口名恒 "main"(与第一遍/运行时 pName 同约定)。
        {
            let mut operands = vec![exec_model, main_id];
            Builder::push_string(&mut operands, "main");
            operands.extend_from_slice(&b.interface);
            Builder::emit(&mut module, OP_ENTRY_POINT, &operands);
        }
        if stage == ShaderStage::Fragment {
            Builder::emit(
                &mut module,
                OP_EXECUTION_MODE,
                &[main_id, EXEC_MODE_ORIGIN_UPPER_LEFT],
            );
        }
        module.extend_from_slice(&b.decorations);
        module.extend_from_slice(&b.types);
        module.extend_from_slice(&b.variables);

        // 函数体:entry block(Function 变量前置 + push-constant 拷入 + 跳 bb0)
        // + 各 MIR block(结构化码,终结子自带;不追加尾 OpReturn)。
        Builder::emit(
            &mut module,
            OP_FUNCTION,
            &[void_id, main_id, FUNCTION_CONTROL_NONE, fn_type_id],
        );
        Builder::emit(&mut module, OP_LABEL, &[entry_label]);
        module.extend_from_slice(&lo.func_vars);
        module.extend_from_slice(&lo.preamble);
        module.extend_from_slice(&lo.code);
        Builder::emit(&mut module, OP_FUNCTION_END, &[]);

        module[bound_index] = b.next_id;
        Ok(module)
    }

    /// 扩展 body lowerer 状态机(见模块文档;全字段生命周期 = 单次 `emit_spirv_ext`)。
    pub(super) struct ExtLowerer<'a> {
        body: &'a Body,
        res: &'a Resolutions,
        input_vars: Vec<IoVar>,
        output_vars: Vec<IoVar>,
        output_written: Vec<bool>,
        /// local 下标 → 槽位(下标 0 = 返回槽;1..=arg_count = 形参)。
        slots: Vec<Slot>,
        /// 单赋值直线 local 的 SSA 值(赋值时物化)。
        ssa: HashMap<u32, EVal>,
        /// 输出结构体聚合临时的字段值(赋值时物化)。
        agg: HashMap<u32, Vec<EVal>>,
        /// 标量形参 → push constant 成员序(声明序;RXS-0302 L1 分配律)。
        scalar_members: Vec<(u32, PrimTy, u32)>,
        pc_var: Option<u32>,
        /// MIR block 下标 → label id(预分配,前向引用)。
        block_label: HashMap<usize, u32>,
        // 预扫描产物。
        assign_count: Vec<u32>,
        proj_write: Vec<bool>,
        def_block: Vec<Option<usize>>,
        use_block: Vec<Option<usize>>,
        multi_block_use: Vec<bool>,
        /// 数据流定型产物(`vec2`/`vec4` 为 typeck 容忍位〔非真实类型,承 RXS-0174
        /// 名约定〕,自 io 字段派生的 MIR local 常为 `Ty::Err`——形状自赋值链定点
        /// 迭代推导,与第一遍 BodyLowerer 的数据流定型同律)。
        inferred: Vec<Option<EKind>>,
        // u64/i64/bool 类型与常量缓存(第一遍 Builder 无 64 位/bool 面,本遍自持)。
        type_u64: Option<u32>,
        type_i64: Option<u32>,
        type_bool: Option<u32>,
        const_cache: HashMap<(u32, u64), u32>,
        pub(super) uses_int64: bool,
        pub(super) uses_int64_atomics: bool,
        // 码流:Function 变量 / entry 前导 / 各 block 结构化码。
        pub(super) func_vars: Vec<u32>,
        pub(super) preamble: Vec<u32>,
        pub(super) code: Vec<u32>,
    }

    impl<'a> ExtLowerer<'a> {
        fn new(body: &'a Body, res: &'a Resolutions, io_vars: &[IoVar]) -> Self {
            let input_vars: Vec<IoVar> = io_vars
                .iter()
                .copied()
                .filter(|v| v.dir == IoDir::In)
                .collect();
            let output_vars: Vec<IoVar> = io_vars
                .iter()
                .copied()
                .filter(|v| v.dir == IoDir::Out)
                .collect();
            let n = body.locals.len();
            ExtLowerer {
                body,
                res,
                output_written: vec![false; output_vars.len()],
                input_vars,
                output_vars,
                slots: vec![Slot::Zst; n],
                ssa: HashMap::new(),
                agg: HashMap::new(),
                scalar_members: Vec::new(),
                pc_var: None,
                block_label: HashMap::new(),
                assign_count: vec![0; n],
                proj_write: vec![false; n],
                def_block: vec![None; n],
                use_block: vec![None; n],
                multi_block_use: vec![false; n],
                inferred: vec![None; n],
                type_u64: None,
                type_i64: None,
                type_bool: None,
                const_cache: HashMap::new(),
                uses_int64: false,
                uses_int64_atomics: false,
                func_vars: Vec::new(),
                preamble: Vec::new(),
                code: Vec::new(),
            }
        }

        // ── 类型 / 常量 ──────────────────────────────────────────────

        /// 归一化标量 → SPIR-V 类型 id(f32/i32/u32 复用第一遍 Builder 缓存;
        /// u64/i64 本遍自持懒发并置 `Int64`;f64 恒拒 = RXS-0301 L3 负面清单轴)。
        fn prim_type(&mut self, b: &mut Builder, p: PrimTy) -> Result<u32, DxilError> {
            let Some(np) = norm_prim(p) else {
                return Err(unmappable(
                    "ext-negative-f64",
                    format!(
                        "类型 {p:?} 在图形 body 恒拒(RXS-0301 L3 负面清单:f64——\
                         cast 表与 compute `cast_opcode` 同表,f64 目标恒拒承 RXS-0203 L1;\
                         FS 复刻判定为精确 f32 字面权威,禁升 f64 改写,RXS-0303 L2)"
                    ),
                ));
            };
            match np {
                PrimTy::F32 | PrimTy::I32 | PrimTy::U32 => b.scalar_type(np),
                PrimTy::U64 => {
                    self.uses_int64 = true;
                    if let Some(id) = self.type_u64 {
                        return Ok(id);
                    }
                    let id = b.alloc_id();
                    Builder::emit(&mut b.types, OP_TYPE_INT, &[id, 64, 0]);
                    self.type_u64 = Some(id);
                    Ok(id)
                }
                PrimTy::I64 => {
                    self.uses_int64 = true;
                    if let Some(id) = self.type_i64 {
                        return Ok(id);
                    }
                    let id = b.alloc_id();
                    Builder::emit(&mut b.types, OP_TYPE_INT, &[id, 64, 1]);
                    self.type_i64 = Some(id);
                    Ok(id)
                }
                _ => unreachable!("norm_prim 已归一到建模集"),
            }
        }

        fn kind_type(&mut self, b: &mut Builder, k: EKind) -> Result<u32, DxilError> {
            match k {
                EKind::Scalar(p) => self.prim_type(b, p),
                EKind::Vector(p, n) => match p {
                    PrimTy::F32 | PrimTy::I32 | PrimTy::U32 => b.vector_type(p, n),
                    other => Err(unmappable(
                        "ext-vector-elem",
                        format!("向量分量类型 {other:?} 不在图形扩展白名单(f32/i32/u32)"),
                    )),
                },
            }
        }

        fn t_bool(&mut self, b: &mut Builder) -> u32 {
            if let Some(id) = self.type_bool {
                return id;
            }
            let id = b.alloc_id();
            Builder::emit(&mut b.types, OP_TYPE_BOOL, &[id]);
            self.type_bool = Some(id);
            id
        }

        /// 常量(按 SPIR-V 类型 id + 位型缓存去重;64 位双字小端)。
        fn const_bits(&mut self, b: &mut Builder, ty_id: u32, bits: u64, wide: bool) -> u32 {
            if let Some(&id) = self.const_cache.get(&(ty_id, bits)) {
                return id;
            }
            let id = b.alloc_id();
            if wide {
                Builder::emit(
                    &mut b.types,
                    OP_CONSTANT,
                    &[ty_id, id, bits as u32, (bits >> 32) as u32],
                );
            } else {
                Builder::emit(&mut b.types, OP_CONSTANT, &[ty_id, id, bits as u32]);
            }
            self.const_cache.insert((ty_id, bits), id);
            id
        }

        fn const_u32(&mut self, b: &mut Builder, v: u32) -> Result<u32, DxilError> {
            let ty = b.scalar_type(PrimTy::U32)?;
            Ok(self.const_bits(b, ty, u64::from(v), false))
        }

        // ── 形参分类(RXS-0302 L1:与 compute 同一分配律) ─────────────

        fn classify_params(&mut self, b: &mut Builder) -> Result<(), DxilError> {
            self.slots[0] = Slot::Ret;
            let mut next_binding = 0u32;
            let mut next_member = 0u32;
            let mut io_param: Option<u32> = None;
            for i in 1..=self.body.arg_count {
                let ty = &self.body.locals[i].ty;
                // ZST(unit/ThreadCtx)不物化。
                let zst = match ty {
                    Ty::Tuple(v) => v.is_empty(),
                    Ty::Adt(d, _) => self.res.lang_items.is_thread_ctx(*d),
                    _ => false,
                };
                if zst {
                    self.slots[i] = Slot::Zst;
                    continue;
                }
                if let Ty::Adt(d, args) = ty {
                    // View/ViewMut → SSBO(元素 = args[1];RXS-0302 L1)。
                    if self.res.lang_items.view_mutable(*d).is_some() {
                        let elem = Self::buffer_elem(args.get(1))?;
                        let var = self.emit_ssbo(b, elem, next_binding)?;
                        next_binding += 1;
                        self.slots[i] = Slot::Buffer { var, elem };
                        continue;
                    }
                    // AtomicView → SSBO(元素 = args[1];`Atomic<T>` 标量原子形参
                    // 不在 RXS-0302 首期面,fail-closed)。
                    if let Some(is_view) = self.res.lang_items.atomic_kind(*d) {
                        if !is_view {
                            return Err(unmappable(
                                "ext-param",
                                "图形入口 `Atomic<T>` 标量原子形参不在 RXS-0302 首期资源面\
                                 (仅 View/ViewMut/AtomicView 与标量,P-12 不预开面)",
                            ));
                        }
                        let elem = Self::buffer_elem(args.get(1))?;
                        let var = self.emit_ssbo(b, elem, next_binding)?;
                        next_binding += 1;
                        self.slots[i] = Slot::Buffer { var, elem };
                        continue;
                    }
                    // 纹理/采样器句柄:资源变量已由 `emit_resource` 发射(签名面);
                    // body 内引用仅采样方法可达,本遍恒拒(维持第一遍采样子集)。
                    if self.res.lang_items.is_texture2d(*d)
                        || self.res.lang_items.is_sampler(*d)
                        || self.res.lang_items.is_sampler_cmp(*d)
                        || self.res.lang_items.is_texture_rw2d(*d)
                    {
                        self.slots[i] = Slot::Resource;
                        continue;
                    }
                    // 其余 Adt = 命名 I/O 结构体形参(io_sig 的字段事实源;至多一个,
                    // 多 IO 结构体的字段-元素映射不可自 MIR 恢复 → fail-closed)。
                    if io_param.is_some() {
                        return Err(unmappable(
                            "ext-param",
                            "图形入口多于一个 I/O 结构体形参在扩展面外(字段→Location \
                             映射不可自 MIR 恢复,fail-closed)",
                        ));
                    }
                    io_param = Some(i as u32);
                    self.slots[i] = Slot::IoStruct;
                    continue;
                }
                if let Ty::Prim(p) = ty {
                    // 标量形参 → push constant 成员(声明序;RXS-0302 L1)。恒建
                    // Function 变量(entry 自 push constant 拷入,body 统一按内存式
                    // local 处理,镜像 compute「scalar 形参也建 Function local」)。
                    let np = norm_prim(*p).ok_or_else(|| {
                        unmappable(
                            "ext-negative-f64",
                            "f64 标量形参在图形 body 恒拒(RXS-0301 L3 负面清单)",
                        )
                    })?;
                    self.scalar_members.push((i as u32, np, next_member));
                    next_member += 1;
                    let ty_id = self.prim_type(b, np)?;
                    let ptr = b.pointer_type(STORAGE_FUNCTION, ty_id);
                    let var = b.alloc_id();
                    Builder::emit(
                        &mut self.func_vars,
                        OP_VARIABLE,
                        &[ptr, var, STORAGE_FUNCTION],
                    );
                    self.slots[i] = Slot::Var {
                        var,
                        kind: EKind::Scalar(np),
                    };
                    continue;
                }
                return Err(unmappable(
                    "ext-param",
                    format!(
                        "图形入口形参类型 {ty:?} 不在 RXS-0302 资源面\
                         (View/ViewMut/AtomicView/标量/I/O 结构体)"
                    ),
                ));
            }
            self.emit_push_constants(b)?;
            Ok(())
        }

        fn buffer_elem(arg: Option<&Ty>) -> Result<PrimTy, DxilError> {
            match arg {
                Some(Ty::Prim(p)) => norm_prim(*p).ok_or_else(|| {
                    unmappable(
                        "ext-negative-f64",
                        "f64 缓冲元素在图形 body 恒拒(RXS-0301 L3 负面清单)",
                    )
                }),
                other => Err(unmappable(
                    "ext-buffer-elem",
                    format!("存储缓冲元素类型 {other:?} 不在标量白名单"),
                )),
            }
        }

        /// SSBO 描述符(SPIR-V 1.0 形态:`OpTypeRuntimeArray`+`ArrayStride` →
        /// `OpTypeStruct`+`BufferBlock`+member Offset 0 → `Uniform` 存储类变量 +
        /// `DescriptorSet 0`/`Binding n`;与 compute 路 `emit_buffer_descriptors`
        /// 1.0 臂同形,RXS-0302 L1「set0-flat 与 render_exec Bindings 字面对齐」)。
        fn emit_ssbo(
            &mut self,
            b: &mut Builder,
            elem: PrimTy,
            binding: u32,
        ) -> Result<u32, DxilError> {
            let elem_ty = self.prim_type(b, elem)?;
            let stride: u32 = if matches!(elem, PrimTy::U64 | PrimTy::I64) {
                8
            } else {
                4
            };
            let rarr = b.alloc_id();
            Builder::emit(&mut b.types, OP_TYPE_RUNTIME_ARRAY, &[rarr, elem_ty]);
            Builder::emit(
                &mut b.decorations,
                OP_DECORATE,
                &[rarr, DECORATION_ARRAY_STRIDE, stride],
            );
            let st = b.alloc_id();
            Builder::emit(&mut b.types, OP_TYPE_STRUCT, &[st, rarr]);
            Builder::emit(
                &mut b.decorations,
                OP_MEMBER_DECORATE,
                &[st, 0, DECORATION_OFFSET, 0],
            );
            Builder::emit(
                &mut b.decorations,
                OP_DECORATE,
                &[st, DECORATION_BUFFER_BLOCK],
            );
            let ptr = b.pointer_type(STORAGE_UNIFORM, st);
            let var = b.alloc_id();
            Builder::emit(&mut b.variables, OP_VARIABLE, &[ptr, var, STORAGE_UNIFORM]);
            Builder::emit(
                &mut b.decorations,
                OP_DECORATE,
                &[var, DECORATION_DESCRIPTOR_SET, 0],
            );
            Builder::emit(
                &mut b.decorations,
                OP_DECORATE,
                &[var, DECORATION_BINDING, binding],
            );
            Ok(var)
        }

        /// 标量形参 → 单 push constant 块(member Offset 4 字节对齐顺排;u64 8 字节
        /// 对齐,与 compute `emit_push_constants`/`prim_layout` 同律,RXS-0302 L1)。
        fn emit_push_constants(&mut self, b: &mut Builder) -> Result<(), DxilError> {
            if self.scalar_members.is_empty() {
                return Ok(());
            }
            let mut member_tys = Vec::new();
            for (_, p, _) in self.scalar_members.clone() {
                member_tys.push(self.prim_type(b, p)?);
            }
            let st = b.alloc_id();
            let mut operands = vec![st];
            operands.extend_from_slice(&member_tys);
            Builder::emit(&mut b.types, OP_TYPE_STRUCT, &operands);
            let mut offset = 0u32;
            for (idx, (_, p, _)) in self.scalar_members.clone().into_iter().enumerate() {
                let (align, size) = if matches!(p, PrimTy::U64 | PrimTy::I64) {
                    (8u32, 8u32)
                } else {
                    (4u32, 4u32)
                };
                offset = offset.div_ceil(align) * align;
                Builder::emit(
                    &mut b.decorations,
                    OP_MEMBER_DECORATE,
                    &[st, idx as u32, DECORATION_OFFSET, offset],
                );
                offset += size;
            }
            Builder::emit(&mut b.decorations, OP_DECORATE, &[st, DECORATION_BLOCK]);
            let ptr = b.pointer_type(STORAGE_PUSH_CONSTANT, st);
            let var = b.alloc_id();
            Builder::emit(
                &mut b.variables,
                OP_VARIABLE,
                &[ptr, var, STORAGE_PUSH_CONSTANT],
            );
            self.pc_var = Some(var);
            Ok(())
        }

        // ── 预扫描(内存式 local 判据:多次赋值或跨分支活跃,RXS-0301 IR) ──

        fn note_use(&mut self, local: u32, bi: usize) {
            let l = local as usize;
            match self.use_block[l] {
                None => self.use_block[l] = Some(bi),
                Some(prev) if prev != bi => self.multi_block_use[l] = true,
                _ => {}
            }
        }

        fn note_operand(&mut self, o: &Operand, bi: usize) {
            if let Operand::Copy(p) | Operand::Move(p) = o {
                self.note_use(p.local.0, bi);
                for pe in &p.proj {
                    if let ProjElem::Index(l) = pe {
                        self.note_use(l.0, bi);
                    }
                }
            }
        }

        fn note_assign(&mut self, place: &Place, bi: usize) {
            let l = place.local.0 as usize;
            if place.proj.is_empty() {
                self.assign_count[l] += 1;
                self.def_block[l] = Some(bi);
            } else {
                self.proj_write[l] = true;
                for pe in &place.proj {
                    if let ProjElem::Index(idx) = pe {
                        self.note_use(idx.0, bi);
                    }
                }
            }
        }

        fn prescan(&mut self) {
            for (bi, bb) in self.body.blocks.iter().enumerate() {
                for stmt in &bb.stmts {
                    let StatementKind::Assign(place, rv) = &stmt.kind;
                    self.note_assign(place, bi);
                    for o in crate::move_check::rvalue_operands(rv) {
                        self.note_operand(o, bi);
                    }
                }
                match &bb.terminator.kind {
                    TerminatorKind::SwitchBool { discr, .. } => self.note_operand(discr, bi),
                    TerminatorKind::Call { args, dest, .. } => {
                        for a in args {
                            self.note_operand(a, bi);
                        }
                        self.note_assign(dest, bi);
                    }
                    _ => {}
                }
            }
        }

        // ── 数据流定型(定点迭代;`vec2/vec4` 容忍位派生的 Err 局部形状推导) ──

        /// 操作数形状(仅静态可判者;不发射任何指令)。
        fn try_kind_of_operand(&self, o: &Operand) -> Option<EKind> {
            match o {
                Operand::Const(Const::Int(_, p)) | Operand::Const(Const::Float(_, p)) => {
                    Some(EKind::Scalar(norm_prim(*p)?))
                }
                Operand::Const(_) => None,
                Operand::Copy(place) | Operand::Move(place) => {
                    let l = place.local.0 as usize;
                    let base = if l >= 1 && l <= self.body.arg_count {
                        match &self.slots[l] {
                            Slot::IoStruct => {
                                let field = match place.proj.first() {
                                    Some(ProjElem::Field(f)) => *f as usize,
                                    _ => return None,
                                };
                                let kind = ekind_of_io(self.input_vars.get(field)?.ty);
                                return match place.proj.get(1) {
                                    None => Some(kind),
                                    Some(ProjElem::Field(_)) => match kind {
                                        EKind::Vector(p, _) => Some(EKind::Scalar(p)),
                                        EKind::Scalar(_) => None,
                                    },
                                    Some(_) => None,
                                };
                            }
                            Slot::Buffer { elem, .. } => {
                                return matches!(place.proj.as_slice(), [ProjElem::Index(_)])
                                    .then_some(EKind::Scalar(*elem));
                            }
                            Slot::Var { kind, .. } => Some(*kind),
                            _ => None,
                        }
                    } else {
                        self.inferred[l].or_else(|| ekind_of(&self.body.locals[l].ty))
                    }?;
                    match place.proj.as_slice() {
                        [] => Some(base),
                        [ProjElem::Field(_)] => match base {
                            EKind::Vector(p, _) => Some(EKind::Scalar(p)),
                            EKind::Scalar(_) => None,
                        },
                        _ => None,
                    }
                }
            }
        }

        fn try_kind_of_rvalue(&self, rv: &Rvalue) -> Option<EKind> {
            match rv {
                Rvalue::Use(o) => self.try_kind_of_operand(o),
                Rvalue::BinaryOp(op, a, b) => match op {
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        // 比较 → bool(内存 u32 表示)。
                        Some(EKind::Scalar(PrimTy::U32))
                    }
                    _ => self
                        .try_kind_of_operand(a)
                        .or_else(|| self.try_kind_of_operand(b)),
                },
                Rvalue::Cast(_, Ty::Prim(p)) => Some(EKind::Scalar(norm_prim(*p)?)),
                Rvalue::Cast(_, _) => None,
                Rvalue::Aggregate(ty, ops) => {
                    if let Some(k) = ekind_of(ty) {
                        return Some(k);
                    }
                    // vec2/vec4 容忍位元组:分量形状推导(须同型标量,lowering 复核)。
                    if !(2..=4).contains(&ops.len()) {
                        return None;
                    }
                    let first = ops.iter().find_map(|o| self.try_kind_of_operand(o))?;
                    match first {
                        EKind::Scalar(p) => Some(EKind::Vector(p, ops.len() as u8)),
                        EKind::Vector(..) => None,
                    }
                }
                Rvalue::Atomic { target_local, .. } => {
                    match self.slots.get(target_local.0 as usize) {
                        Some(Slot::Buffer { elem, .. }) => Some(EKind::Scalar(*elem)),
                        _ => None,
                    }
                }
                _ => None,
            }
        }

        /// 定点迭代:自赋值链推导各 local 形状(有界:每轮至少定型一个,轮数 ≤
        /// locals 数;推导不到的 local 在 `alloc_locals` fail-closed)。
        fn infer_kinds(&mut self) {
            loop {
                let mut changed = false;
                for bb in &self.body.blocks {
                    for stmt in &bb.stmts {
                        let StatementKind::Assign(place, rv) = &stmt.kind;
                        let l = place.local.0 as usize;
                        if place.proj.is_empty()
                            && l > self.body.arg_count
                            && self.inferred[l].is_none()
                            && let Some(k) = self.try_kind_of_rvalue(rv)
                        {
                            self.inferred[l] = Some(k);
                            changed = true;
                        }
                    }
                    if let TerminatorKind::Call { target, dest, .. } = &bb.terminator.kind {
                        let l = dest.local.0 as usize;
                        if dest.proj.is_empty()
                            && l > self.body.arg_count
                            && self.inferred[l].is_none()
                            && matches!(target, CallTarget::Libdevice { .. })
                        {
                            // GLSL.std.450 内建首批面恒 f32 结果。
                            self.inferred[l] = Some(EKind::Scalar(PrimTy::F32));
                            changed = true;
                        }
                    }
                }
                if !changed {
                    break;
                }
            }
        }

        /// 槽位判定 + 内存式 local 的 Function `OpVariable` 发射 + block label 预分配。
        fn alloc_locals(&mut self, b: &mut Builder) -> Result<(), DxilError> {
            for bi in 0..self.body.blocks.len() {
                let id = b.alloc_id();
                self.block_label.insert(bi, id);
            }
            // 非形参 local(含 MIR 临时):按预扫描判据分派槽位。
            for i in (self.body.arg_count + 1)..self.body.locals.len() {
                let ty = &self.body.locals[i].ty;
                if matches!(ty, Ty::Tuple(v) if v.is_empty()) {
                    self.slots[i] = Slot::Zst;
                    continue;
                }
                // 全程未被触及的死 local:不物化(保守中性)。
                if self.assign_count[i] == 0 && self.use_block[i].is_none() && !self.proj_write[i] {
                    self.slots[i] = Slot::Zst;
                    continue;
                }
                // 形状 = 真实类型 或 数据流推导(vec2/vec4 容忍位派生局部)。
                match ekind_of(ty).or(self.inferred[i]) {
                    Some(kind) => {
                        let single_assign = self.assign_count[i] <= 1 && !self.proj_write[i];
                        let straight_line = !self.multi_block_use[i]
                            && match (self.def_block[i], self.use_block[i]) {
                                (_, None) => true,
                                (Some(d), Some(u)) => d == u,
                                (None, Some(_)) => false,
                            };
                        if single_assign && straight_line {
                            self.slots[i] = Slot::Ssa;
                        } else {
                            let ty_id = self.kind_type(b, kind)?;
                            let ptr = b.pointer_type(STORAGE_FUNCTION, ty_id);
                            let var = b.alloc_id();
                            Builder::emit(
                                &mut self.func_vars,
                                OP_VARIABLE,
                                &[ptr, var, STORAGE_FUNCTION],
                            );
                            self.slots[i] = Slot::Var { var, kind };
                        }
                    }
                    None => {
                        // 非标量/向量 local:仅放行「输出结构体聚合临时」(单赋值
                        // 直线;输出装配面,RXS-0301 graphics_output_assembly)。
                        if ty == self.body.ret_ty()
                            && self.assign_count[i] <= 1
                            && !self.proj_write[i]
                            && !self.multi_block_use[i]
                        {
                            self.slots[i] = Slot::Agg;
                        } else {
                            return Err(unmappable(
                                "ext-local",
                                format!(
                                    "local _{i} 类型 {ty:?} 不在图形扩展白名单\
                                     (标量/2..=4 同型元组向量/输出结构体聚合临时;\
                                     形状不可自赋值链定型)"
                                ),
                            ));
                        }
                    }
                }
            }
            Ok(())
        }

        /// entry 前导:标量形参自 push constant 块拷入 Function local,跳 bb0。
        fn emit_preamble(&mut self, b: &mut Builder) -> Result<(), DxilError> {
            for (local, p, member) in self.scalar_members.clone() {
                let pc = self.pc_var.expect("有 scalar 形参则 push-constant 块已建");
                let ty_id = self.prim_type(b, p)?;
                let ptr_pc = b.pointer_type(STORAGE_PUSH_CONSTANT, ty_id);
                let midx = self.const_u32(b, member)?;
                let acc = b.alloc_id();
                Builder::emit(
                    &mut self.preamble,
                    OP_ACCESS_CHAIN,
                    &[ptr_pc, acc, pc, midx],
                );
                let val = b.alloc_id();
                Builder::emit(&mut self.preamble, OP_LOAD, &[ty_id, val, acc]);
                let Slot::Var { var, .. } = self.slots[local as usize] else {
                    unreachable!("标量形参已建 Function 变量");
                };
                Builder::emit(&mut self.preamble, OP_STORE, &[var, val]);
            }
            let bb0 = self.block_label[&0];
            Builder::emit(&mut self.preamble, OP_BRANCH, &[bb0]);
            Ok(())
        }

        // ── 主循环:逐 block 结构化降级 ─────────────────────────────

        fn lower_blocks(&mut self, b: &mut Builder) -> Result<(), DxilError> {
            for bi in 0..self.body.blocks.len() {
                let label = self.block_label[&bi];
                Builder::emit(&mut self.code, OP_LABEL, &[label]);
                let bb = &self.body.blocks[bi];
                for stmt in &bb.stmts {
                    let StatementKind::Assign(place, rv) = &stmt.kind;
                    self.lower_assign(b, place, rv)?;
                }
                self.lower_terminator(b, bi)?;
            }
            Ok(())
        }

        fn lower_terminator(&mut self, b: &mut Builder, bi: usize) -> Result<(), DxilError> {
            let bb = &self.body.blocks[bi];
            match &bb.terminator.kind {
                TerminatorKind::Goto(t) => {
                    let lbl = self.block_label[&(t.0 as usize)];
                    Builder::emit(&mut self.code, OP_BRANCH, &[lbl]);
                }
                TerminatorKind::Return => {
                    Builder::emit(&mut self.code, OP_RETURN, &[]);
                }
                TerminatorKind::Unreachable => {
                    Builder::emit(&mut self.code, OP_UNREACHABLE, &[]);
                }
                TerminatorKind::Drop { next, .. } => {
                    let lbl = self.block_label[&(next.0 as usize)];
                    Builder::emit(&mut self.code, OP_BRANCH, &[lbl]);
                }
                TerminatorKind::Call {
                    target,
                    args,
                    dest,
                    next,
                } => {
                    self.lower_call(b, target, args, dest)?;
                    let lbl = self.block_label[&(next.0 as usize)];
                    Builder::emit(&mut self.code, OP_BRANCH, &[lbl]);
                }
                TerminatorKind::SwitchBool { discr, then, else_ } => {
                    // 结构化 if(RXS-0301 graphics_control_flow_and_calls):
                    // discr(u32 0/1)→ INotEqual 0 → OpSelectionMerge + 条件分支;
                    // merge = 前向可达交汇(与 compute 同一算法;回边已在预扫描恒拒)。
                    let dv = self.operand_value(b, discr, None)?;
                    let bool_ty = self.t_bool(b);
                    let zero = self.const_u32(b, 0)?;
                    let cond = b.alloc_id();
                    Builder::emit(&mut self.code, OP_INOTEQUAL, &[bool_ty, cond, dv.id, zero]);
                    let then_i = then.0 as usize;
                    let else_i = else_.0 as usize;
                    let merge = structured_merge(self.body, then_i, else_i).ok_or_else(|| {
                        unmappable(
                            "ext-control-flow",
                            "图形扩展白名单仅支持结构化 if(分支须收敛于唯一 merge 块;\
                             提前 return 不在面内)",
                        )
                    })?;
                    let merge_lbl = self.block_label[&merge];
                    let then_lbl = self.block_label[&then_i];
                    let else_lbl = self.block_label[&else_i];
                    Builder::emit(
                        &mut self.code,
                        OP_SELECTION_MERGE,
                        &[merge_lbl, SELECTION_CONTROL_NONE],
                    );
                    Builder::emit(
                        &mut self.code,
                        OP_BRANCH_CONDITIONAL,
                        &[cond, then_lbl, else_lbl],
                    );
                }
            }
            Ok(())
        }

        /// Call 终结子:GLSL.std.450 内建(首批 `round`→RoundEven,与 compute
        /// `glsl_ext_op` 同表)放行;用户 device fn 调用 = RXS-0301 L3 负面清单恒拒。
        fn lower_call(
            &mut self,
            b: &mut Builder,
            target: &CallTarget,
            args: &[Operand],
            dest: &Place,
        ) -> Result<(), DxilError> {
            match target {
                CallTarget::Libdevice { symbol } => {
                    let Some((glsl_op, arity)) = glsl_ext_op(symbol) else {
                        return Err(unmappable(
                            "ext-call",
                            format!("数学 intrinsic `{symbol}` 未在 GLSL.std.450 映射表内"),
                        ));
                    };
                    if args.len() != arity {
                        return Err(unmappable(
                            "ext-call",
                            format!(
                                "数学 intrinsic `{symbol}` 期望 {arity} 实参,得 {}",
                                args.len()
                            ),
                        ));
                    }
                    let set = b.glsl_ext_inst();
                    let float_ty = b.scalar_type(PrimTy::F32)?;
                    let result = b.alloc_id();
                    let mut operands = vec![float_ty, result, set, glsl_op];
                    for a in args {
                        let v = self.operand_value(b, a, Some(EKind::Scalar(PrimTy::F32)))?;
                        operands.push(v.id);
                    }
                    Builder::emit(&mut self.code, OP_EXT_INST, &operands);
                    self.store_value(
                        dest,
                        EVal {
                            id: result,
                            kind: EKind::Scalar(PrimTy::F32),
                        },
                    )
                }
                CallTarget::Fn { .. } => Err(unmappable(
                    "ext-negative-devfn-call",
                    "用户自定义 device fn 调用在图形 body 恒拒(RXS-0301 L3 负面清单,\
                     拒绝面 grow-only)",
                )),
                other => Err(unmappable(
                    "ext-call",
                    format!("调用目标 {other:?} 不在图形扩展白名单(GLSL.std.450 内建之外)"),
                )),
            }
        }

        // ── 赋值 / 输出装配 ─────────────────────────────────────────

        fn lower_assign(
            &mut self,
            b: &mut Builder,
            place: &Place,
            rv: &Rvalue,
        ) -> Result<(), DxilError> {
            // 返回槽:输出装配(单字段写 / 聚合分解;RXS-0301 graphics_output_assembly)。
            if place.local == LocalIdx(0) {
                if let Some(index) = single_field_projection(place)? {
                    let expected = self
                        .output_vars
                        .get(index)
                        .map(|v| ekind_of_io(v.ty))
                        .ok_or_else(|| {
                            unmappable("ext-output", format!("输出 I/O 字段 {index} 越界"))
                        })?;
                    let v = self.rvalue_value(b, rv, Some(expected))?;
                    return self.store_output(index, v);
                }
                return match self.rvalue_any(b, rv, None)? {
                    EOperand::Agg(fields) => self.store_output_fields(&fields),
                    EOperand::Value(v) if self.output_vars.len() == 1 => self.store_output(0, v),
                    EOperand::Unit if self.output_vars.is_empty() => Ok(()),
                    other => Err(unmappable(
                        "ext-output",
                        format!("返回值形态 {other:?} 与声明的 Output I/O 不匹配"),
                    )),
                };
            }
            let slot = self.slots[place.local.0 as usize].clone();
            match slot {
                Slot::Var { var, kind } => {
                    if place.proj.is_empty() {
                        let v = self.rvalue_value(b, rv, Some(kind))?;
                        if v.kind != kind {
                            return Err(unmappable(
                                "ext-store-type",
                                format!(
                                    "local _{} 声明形状 {kind:?} 与赋值 {:?} 不符",
                                    place.local.0, v.kind
                                ),
                            ));
                        }
                        Builder::emit(&mut self.code, OP_STORE, &[var, v.id]);
                        return Ok(());
                    }
                    // 向量分量写(单层 Field;内存式 local 的 OpAccessChain 分量指针)。
                    if let (Some(k), EKind::Vector(p, n)) = (single_field_projection(place)?, kind)
                    {
                        if (k as u8) >= n {
                            return Err(unmappable(
                                "ext-place",
                                format!("向量分量下标 {k} 越界(分量数 {n})"),
                            ));
                        }
                        let v = self.rvalue_value(b, rv, Some(EKind::Scalar(p)))?;
                        let comp_ty = self.prim_type(b, p)?;
                        let ptr_ty = b.pointer_type(STORAGE_FUNCTION, comp_ty);
                        let kidx = self.const_u32(b, k as u32)?;
                        let acc = b.alloc_id();
                        Builder::emit(&mut self.code, OP_ACCESS_CHAIN, &[ptr_ty, acc, var, kidx]);
                        Builder::emit(&mut self.code, OP_STORE, &[acc, v.id]);
                        return Ok(());
                    }
                    Err(unmappable(
                        "ext-place",
                        format!("写入投影 place `{place:?}` 不在图形扩展白名单"),
                    ))
                }
                Slot::Ssa => {
                    if !place.proj.is_empty() {
                        return Err(unmappable(
                            "ext-place",
                            "SSA local 不支持投影写(内部判据不一致)",
                        ));
                    }
                    let v = self.rvalue_value(b, rv, None)?;
                    self.ssa.insert(place.local.0, v);
                    Ok(())
                }
                Slot::Agg => {
                    if !place.proj.is_empty() {
                        return Err(unmappable("ext-place", "聚合临时不支持投影写"));
                    }
                    match self.rvalue_any(b, rv, None)? {
                        EOperand::Agg(fields) => {
                            self.agg.insert(place.local.0, fields);
                            Ok(())
                        }
                        other => Err(unmappable(
                            "ext-aggregate",
                            format!("聚合临时期望输出结构体字段序列,实得 {other:?}"),
                        )),
                    }
                }
                Slot::Buffer { .. } => Err(unmappable(
                    "ext-ssbo-write",
                    "SSBO 写不在 RXS-0301 放行面(graphics_buffer_indexing 仅放行动态索引**读**;\
                     可变副作用面 = RXS-0302 L2 u64 原子)",
                )),
                Slot::Zst => {
                    // unit 赋值:求值副作用后丢弃(常态为 `Use(Const(Unit))` no-op)。
                    let _ = self.rvalue_any(b, rv, None)?;
                    Ok(())
                }
                Slot::Ret | Slot::IoStruct | Slot::Resource => Err(unmappable(
                    "ext-place",
                    format!("写入 place `{place:?}`(输入/资源形参)不合法"),
                )),
            }
        }

        fn store_output(&mut self, index: usize, value: EVal) -> Result<(), DxilError> {
            let out =
                self.output_vars.get(index).copied().ok_or_else(|| {
                    unmappable("ext-output", format!("输出 I/O 字段 {index} 越界"))
                })?;
            let expected = ekind_of_io(out.ty);
            if expected != value.kind {
                return Err(unmappable(
                    "ext-output-type",
                    format!(
                        "输出字段 {index} 类型 {:?} 与值类型 {:?} 不符",
                        expected, value.kind
                    ),
                ));
            }
            Builder::emit(&mut self.code, OP_STORE, &[out.var_id, value.id]);
            if let Some(w) = self.output_written.get_mut(index) {
                *w = true;
            }
            Ok(())
        }

        fn store_output_fields(&mut self, fields: &[EVal]) -> Result<(), DxilError> {
            if fields.len() != self.output_vars.len() {
                return Err(unmappable(
                    "ext-output",
                    format!(
                        "输出聚合字段数 {} 与 Output I/O 元素数 {} 不一致",
                        fields.len(),
                        self.output_vars.len()
                    ),
                ));
            }
            for (idx, v) in fields.iter().copied().enumerate() {
                self.store_output(idx, v)?;
            }
            Ok(())
        }

        /// 值写入任意目标 place(Call 终结子 dest 复用赋值面)。
        fn store_value(&mut self, place: &Place, value: EVal) -> Result<(), DxilError> {
            if place.local == LocalIdx(0) {
                if let Some(index) = single_field_projection(place)? {
                    return self.store_output(index, value);
                }
                if self.output_vars.len() == 1 {
                    return self.store_output(0, value);
                }
                return Err(unmappable("ext-output", "多字段输出须聚合返回"));
            }
            match self.slots[place.local.0 as usize].clone() {
                Slot::Var { var, .. } if place.proj.is_empty() => {
                    Builder::emit(&mut self.code, OP_STORE, &[var, value.id]);
                    Ok(())
                }
                Slot::Ssa if place.proj.is_empty() => {
                    self.ssa.insert(place.local.0, value);
                    Ok(())
                }
                _ => Err(unmappable(
                    "ext-place",
                    format!("调用结果写入 place `{place:?}` 不在扩展面"),
                )),
            }
        }

        // ── rvalue / operand 求值 ───────────────────────────────────

        fn rvalue_value(
            &mut self,
            b: &mut Builder,
            rv: &Rvalue,
            expected: Option<EKind>,
        ) -> Result<EVal, DxilError> {
            match self.rvalue_any(b, rv, expected)? {
                EOperand::Value(v) => Ok(v),
                other => Err(unmappable(
                    "ext-rvalue",
                    format!("期望标量/向量值,实得 {other:?}"),
                )),
            }
        }

        fn rvalue_any(
            &mut self,
            b: &mut Builder,
            rv: &Rvalue,
            expected: Option<EKind>,
        ) -> Result<EOperand, DxilError> {
            match rv {
                Rvalue::Use(o) => self.lower_operand(b, o, expected),
                Rvalue::BinaryOp(op, lhs, rhs) => {
                    Ok(EOperand::Value(self.lower_binop(b, *op, lhs, rhs)?))
                }
                Rvalue::Cast(o, target) => Ok(EOperand::Value(self.lower_cast(b, o, target)?)),
                Rvalue::Aggregate(ty, ops) => self.lower_aggregate(b, ty, ops),
                Rvalue::Atomic {
                    op,
                    target_local,
                    index,
                    value,
                    compare,
                    scope,
                } => Ok(EOperand::Value(self.lower_atomic(
                    b,
                    *op,
                    *target_local,
                    index.as_ref(),
                    value,
                    compare.as_ref(),
                    *scope,
                )?)),
                Rvalue::ResourceSample { .. } => Err(unmappable(
                    "ext-sample",
                    "纹理采样在图形扩展遍不放行(维持第一遍采样最小子集;RXS-0301 白名单外)",
                )),
                Rvalue::RayQueryInitialize { .. } | Rvalue::RayQueryMethod { .. } => {
                    Err(unmappable(
                        "ext-negative-rayquery",
                        "RayQuery 在图形 body 恒拒(RXS-0301 L3 负面清单;RXS-0297~0300 \
                         为 compute 面)",
                    ))
                }
                other => Err(unmappable(
                    "ext-rvalue",
                    format!("rvalue `{other:?}` 不在 RXS-0301 图形扩展白名单"),
                )),
            }
        }

        fn lower_binop(
            &mut self,
            b: &mut Builder,
            op: BinOp,
            lhs: &Operand,
            rhs: &Operand,
        ) -> Result<EVal, DxilError> {
            let va = self.operand_value(b, lhs, None)?;
            let vc = self.operand_value(b, rhs, Some(va.kind))?;
            if va.kind != vc.kind {
                return Err(unmappable(
                    "ext-binop-type",
                    format!("二元操作左右类型不一致: {:?} vs {:?}", va.kind, vc.kind),
                ));
            }
            let EKind::Scalar(prim) = va.kind else {
                return Err(unmappable(
                    "ext-binop",
                    "向量算术不在图形扩展白名单(标量运算面)",
                ));
            };
            let is_float = matches!(prim, PrimTy::F32);
            let (opcode, is_bool) = binop_opcode(op, is_float, is_signed(prim), self.body.span)
                .map_err(|e| unmappable("ext-binop", e.detail))?;
            if is_bool {
                // 比较 → OpTypeBool → OpSelect u32(0/1)(bool 内存以 u32 表示,
                // 与 compute 路镜像;归一化后 kind = Scalar(U32))。
                let bool_ty = self.t_bool(b);
                let cmp = b.alloc_id();
                Builder::emit(&mut self.code, opcode, &[bool_ty, cmp, va.id, vc.id]);
                let u32_ty = b.scalar_type(PrimTy::U32)?;
                let one = self.const_u32(b, 1)?;
                let zero = self.const_u32(b, 0)?;
                let sel = b.alloc_id();
                Builder::emit(&mut self.code, OP_SELECT, &[u32_ty, sel, cmp, one, zero]);
                Ok(EVal {
                    id: sel,
                    kind: EKind::Scalar(PrimTy::U32),
                })
            } else {
                let ty_id = self.prim_type(b, prim)?;
                let id = b.alloc_id();
                Builder::emit(&mut self.code, opcode, &[ty_id, id, va.id, vc.id]);
                Ok(EVal { id, kind: va.kind })
            }
        }

        fn lower_cast(
            &mut self,
            b: &mut Builder,
            o: &Operand,
            target: &Ty,
        ) -> Result<EVal, DxilError> {
            let v = self.operand_value(b, o, None)?;
            let EKind::Scalar(src) = v.kind else {
                return Err(unmappable("ext-cast", "向量 Cast 不在图形扩展白名单"));
            };
            let Ty::Prim(dst_raw) = target else {
                return Err(unmappable(
                    "ext-cast",
                    format!("Cast 目标 {target:?} 非标量(白名单外)"),
                ));
            };
            // f64 目标在 prim_type 恒拒(RXS-0301 L3 负面清单轴)。
            let dst_ty_id = self.prim_type(b, *dst_raw)?;
            let dst = norm_prim(*dst_raw).expect("prim_type 已裁决 f64");
            let src_ty_id = self.prim_type(b, src)?;
            if src_ty_id == dst_ty_id {
                // 同 SPIR-V 类型(如 usize→u32)→ identity,零转换指令。
                return Ok(EVal {
                    id: v.id,
                    kind: EKind::Scalar(dst),
                });
            }
            let opcode = cast_opcode(src, dst).map_err(|e| unmappable("ext-cast", e.detail))?;
            let id = b.alloc_id();
            Builder::emit(&mut self.code, opcode, &[dst_ty_id, id, v.id]);
            Ok(EVal {
                id,
                kind: EKind::Scalar(dst),
            })
        }

        fn lower_aggregate(
            &mut self,
            b: &mut Builder,
            ty: &Ty,
            ops: &[Operand],
        ) -> Result<EOperand, DxilError> {
            // 标量 → 向量聚合构造(RXS-0301 graphics_output_assembly)。元组元素在
            // typeck 常为容忍位(vec2/vec4 名约定),分量类型自**求值后的分量 kind**
            // 定型(同型标量强制,与第一遍数据流定型同律)。
            if let Ty::Tuple(el) = ty
                && (2..=4).contains(&el.len())
            {
                if ops.len() != el.len() {
                    return Err(unmappable(
                        "ext-aggregate",
                        format!("向量构造分量数 {} 与元组宽度 {} 不符", ops.len(), el.len()),
                    ));
                }
                let mut vals = Vec::with_capacity(ops.len());
                let mut elem_prim: Option<PrimTy> = None;
                for op in ops {
                    let v = self.operand_value(b, op, elem_prim.map(EKind::Scalar))?;
                    let EKind::Scalar(p) = v.kind else {
                        return Err(unmappable("ext-aggregate", "向量构造分量必须是标量"));
                    };
                    match elem_prim {
                        None => elem_prim = Some(p),
                        Some(e) if e != p => {
                            return Err(unmappable(
                                "ext-aggregate",
                                format!("向量构造分量类型不一致: {e:?} vs {p:?}"),
                            ));
                        }
                        _ => {}
                    }
                    vals.push(v);
                }
                let p = elem_prim.expect("2..=4 分量非空");
                let kind = EKind::Vector(p, ops.len() as u8);
                let ty_id = self.kind_type(b, kind)?;
                let result = b.alloc_id();
                let mut operands = vec![ty_id, result];
                operands.extend(vals.iter().map(|v| v.id));
                Builder::emit(&mut self.code, OP_COMPOSITE_CONSTRUCT, &operands);
                return Ok(EOperand::Value(EVal { id: result, kind }));
            }
            // 输出结构体聚合(字段序 = 声明序 = Output I/O 元素序)。
            if ty == self.body.ret_ty() {
                if ops.len() != self.output_vars.len() {
                    return Err(unmappable(
                        "ext-aggregate",
                        format!(
                            "输出聚合字段数 {} 与 Output I/O 元素数 {} 不一致",
                            ops.len(),
                            self.output_vars.len()
                        ),
                    ));
                }
                let mut fields = Vec::with_capacity(ops.len());
                for (idx, op) in ops.iter().enumerate() {
                    let expected = ekind_of_io(self.output_vars[idx].ty);
                    fields.push(self.operand_value(b, op, Some(expected))?);
                }
                return Ok(EOperand::Agg(fields));
            }
            Err(unmappable(
                "ext-aggregate",
                format!("聚合类型 {ty:?} 不在图形扩展白名单(向量/输出结构体)"),
            ))
        }

        /// u64 SSBO 原子(RXS-0302 L2):`AtomicView<_,u64,_>.fetch_max(idx, val,
        /// Scope::Gpu)` → `OpAtomicUMax`(Device scope,Relaxed semantics——常量与
        /// compute 路发射段**逐字同值**);scope 仅 `Scope::Gpu`,`Scope::Block`(CTA)
        /// = RXS-0301 L3 负面清单恒拒,`Scope::System` 首期不放行(P-12)。
        #[allow(clippy::too_many_arguments)]
        fn lower_atomic(
            &mut self,
            b: &mut Builder,
            op: AtomicOp,
            target_local: LocalIdx,
            index: Option<&Operand>,
            value: &Operand,
            compare: Option<&Operand>,
            scope: Option<u8>,
        ) -> Result<EVal, DxilError> {
            match scope {
                Some(1) => {}
                Some(0) => {
                    return Err(unmappable(
                        "ext-negative-cta-atomic",
                        "`Scope::Block`(CTA 级)原子在图形 body 恒拒(RXS-0301 L3 负面清单;\
                         fragment 无 workgroup 语义,RXS-0302 L2 仅放行 `Scope::Gpu`)",
                    ));
                }
                Some(_) => {
                    return Err(unmappable(
                        "ext-atomic-scope",
                        "`Scope::System` 原子首期不放行(RXS-0302 L2,P-12 不为假想需求预开面)",
                    ));
                }
                None => {
                    return Err(unmappable(
                        "ext-atomic-scope",
                        "原子 scope 实参不可静态判定(fail-closed;RXS-0302 L2 要求 `Scope::Gpu`)",
                    ));
                }
            }
            if op != AtomicOp::FetchMax || compare.is_some() {
                return Err(unmappable(
                    "ext-atomic-op",
                    format!("原子算子 {op:?} 不在 RXS-0302 首期放行面(仅 u64 `fetch_max`)"),
                ));
            }
            let Slot::Buffer { var, elem } = self.slots[target_local.0 as usize].clone() else {
                return Err(unmappable(
                    "ext-atomic-target",
                    "原子目标须为签名 `AtomicView<global,..>` SSBO 形参(RXS-0302 L1)",
                ));
            };
            if elem != PrimTy::U64 {
                return Err(unmappable(
                    "ext-atomic-elem",
                    format!("原子元素 {elem:?} 不在 RXS-0302 首期放行面(仅 u64)"),
                ));
            }
            self.uses_int64_atomics = true;
            let elem_ty = self.prim_type(b, elem)?;
            let ptr_ty = b.pointer_type(STORAGE_UNIFORM, elem_ty);
            let idx = match index {
                Some(o) => self.operand_value(b, o, None)?.id,
                None => self.const_u32(b, 0)?,
            };
            let member0 = self.const_u32(b, 0)?;
            let ptr = b.alloc_id();
            Builder::emit(
                &mut self.code,
                OP_ACCESS_CHAIN,
                &[ptr_ty, ptr, var, member0, idx],
            );
            let v = self.operand_value(b, value, Some(EKind::Scalar(PrimTy::U64)))?;
            // scope/semantics 常量与 compute 路发射段逐字同值(SCOPE_DEVICE=1,
            // MEM_SEM_RELAXED=0;RXS-0302 L2)。
            let scope_c = self.const_u32(b, SCOPE_DEVICE)?;
            let sem_c = self.const_u32(b, MEM_SEM_RELAXED)?;
            let result = b.alloc_id();
            Builder::emit(
                &mut self.code,
                OP_ATOMIC_UMAX,
                &[elem_ty, result, ptr, scope_c, sem_c, v.id],
            );
            Ok(EVal {
                id: result,
                kind: EKind::Scalar(PrimTy::U64),
            })
        }

        fn operand_value(
            &mut self,
            b: &mut Builder,
            o: &Operand,
            expected: Option<EKind>,
        ) -> Result<EVal, DxilError> {
            match self.lower_operand(b, o, expected)? {
                EOperand::Value(v) => Ok(v),
                other => Err(unmappable(
                    "ext-operand",
                    format!("期望标量/向量值,实得 {other:?}"),
                )),
            }
        }

        fn lower_operand(
            &mut self,
            b: &mut Builder,
            o: &Operand,
            expected: Option<EKind>,
        ) -> Result<EOperand, DxilError> {
            match o {
                Operand::Const(Const::Unit) => Ok(EOperand::Unit),
                Operand::Const(c) => Ok(EOperand::Value(self.lower_const(b, c, expected)?)),
                Operand::Copy(place) | Operand::Move(place) => self.lower_place_read(b, place),
            }
        }

        fn lower_const(
            &mut self,
            b: &mut Builder,
            c: &Const,
            expected: Option<EKind>,
        ) -> Result<EVal, DxilError> {
            let (prim, bits) = match c {
                Const::Int(v, p) => {
                    let np = norm_prim(*p).ok_or_else(|| {
                        unmappable("ext-negative-f64", "64 位外整型/f64 常量不在建模集")
                    })?;
                    let bits =
                        match np {
                            PrimTy::I32 => u64::from(i32::try_from(*v).map_err(|_| {
                                unmappable("ext-constant", format!("i32 常量 {v} 越界"))
                            })? as u32),
                            PrimTy::U32 => u64::from(u32::try_from(*v).map_err(|_| {
                                unmappable("ext-constant", format!("u32 常量 {v} 越界"))
                            })?),
                            // 64 位常量:i128 → 位型(负数二补码截断)。
                            _ => *v as u64,
                        };
                    (np, bits)
                }
                Const::Float(v, PrimTy::F32) => (PrimTy::F32, u64::from((*v as f32).to_bits())),
                // 无后缀浮点字面在前端 Ty::Err 容忍位下缺省定型 F64(非用户显式 f64
                // 语义):期望侧为 f32 时按 f32 字面收编。RXS-0301 L3 f64 负面清单
                // 针对 f64 **类型/运算**(`as f64` cast 目标、f64 局部/IO)恒拒不受
                // 此影响(reject 语料 vk_hw_raster_f64_reject 的违规轴是 cast 表)。
                Const::Float(v, PrimTy::F64) if expected == Some(EKind::Scalar(PrimTy::F32)) => {
                    (PrimTy::F32, u64::from((*v as f32).to_bits()))
                }
                Const::Float(_, other) => {
                    return Err(unmappable(
                        "ext-negative-f64",
                        format!("浮点常量类型 {other:?} 恒拒(RXS-0301 L3 负面清单:f64)"),
                    ));
                }
                other => {
                    return Err(unmappable(
                        "ext-constant",
                        format!("常量 {other:?} 不在图形扩展白名单(f32/整型标量)"),
                    ));
                }
            };
            // 常量类型以**期望侧**收编(整型字面量在 MIR 常已定型;期望缺省取字面型)。
            let prim = match expected {
                Some(EKind::Scalar(e)) if e != prim => {
                    // 位宽同类间的字面收编(如 u32 字面喂 usize 槽);跨 32/64 位或
                    // 跨符号性不静默转换,维持字面型交由类型检查拒。
                    if self.prim_type(b, e)? == self.prim_type(b, prim)? {
                        e
                    } else {
                        prim
                    }
                }
                _ => prim,
            };
            let ty_id = self.prim_type(b, prim)?;
            let wide = matches!(prim, PrimTy::U64 | PrimTy::I64);
            let id = self.const_bits(b, ty_id, bits, wide);
            Ok(EVal {
                id,
                kind: EKind::Scalar(prim),
            })
        }

        fn lower_place_read(
            &mut self,
            b: &mut Builder,
            place: &Place,
        ) -> Result<EOperand, DxilError> {
            let slot = self.slots[place.local.0 as usize].clone();
            match slot {
                Slot::IoStruct => self.read_io(b, place),
                Slot::Buffer { var, elem } => {
                    // SSBO 动态索引读(RXS-0301 graphics_buffer_indexing)。
                    let [ProjElem::Index(idx_local)] = place.proj.as_slice() else {
                        return Err(unmappable(
                            "ext-place",
                            format!("buffer place `{place:?}` 仅支持单层动态索引读 `buf[i]`"),
                        ));
                    };
                    let idx = self.local_scalar(b, idx_local.0)?;
                    let elem_ty = self.prim_type(b, elem)?;
                    let ptr_ty = b.pointer_type(STORAGE_UNIFORM, elem_ty);
                    let member0 = self.const_u32(b, 0)?;
                    let acc = b.alloc_id();
                    Builder::emit(
                        &mut self.code,
                        OP_ACCESS_CHAIN,
                        &[ptr_ty, acc, var, member0, idx.id],
                    );
                    let id = b.alloc_id();
                    Builder::emit(&mut self.code, OP_LOAD, &[elem_ty, id, acc]);
                    Ok(EOperand::Value(EVal {
                        id,
                        kind: EKind::Scalar(elem),
                    }))
                }
                Slot::Var { var, kind } => {
                    if place.proj.is_empty() {
                        let ty_id = self.kind_type(b, kind)?;
                        let id = b.alloc_id();
                        Builder::emit(&mut self.code, OP_LOAD, &[ty_id, id, var]);
                        return Ok(EOperand::Value(EVal { id, kind }));
                    }
                    if let (Some(k), EKind::Vector(p, n)) = (single_field_projection(place)?, kind)
                    {
                        if (k as u8) >= n {
                            return Err(unmappable(
                                "ext-place",
                                format!("向量分量下标 {k} 越界(分量数 {n})"),
                            ));
                        }
                        let comp_ty = self.prim_type(b, p)?;
                        let ptr_ty = b.pointer_type(STORAGE_FUNCTION, comp_ty);
                        let kidx = self.const_u32(b, k as u32)?;
                        let acc = b.alloc_id();
                        Builder::emit(&mut self.code, OP_ACCESS_CHAIN, &[ptr_ty, acc, var, kidx]);
                        let id = b.alloc_id();
                        Builder::emit(&mut self.code, OP_LOAD, &[comp_ty, id, acc]);
                        return Ok(EOperand::Value(EVal {
                            id,
                            kind: EKind::Scalar(p),
                        }));
                    }
                    Err(unmappable(
                        "ext-place",
                        format!("place `{place:?}` 投影不在图形扩展白名单"),
                    ))
                }
                Slot::Ssa => {
                    let v = self.ssa.get(&place.local.0).copied().ok_or_else(|| {
                        unmappable(
                            "ext-place",
                            format!("local _{} 尚未物化(SSA 定值先于使用)", place.local.0),
                        )
                    })?;
                    if place.proj.is_empty() {
                        return Ok(EOperand::Value(v));
                    }
                    if let (Some(k), EKind::Vector(p, n)) =
                        (single_field_projection(place)?, v.kind)
                    {
                        if (k as u8) >= n {
                            return Err(unmappable(
                                "ext-place",
                                format!("向量分量下标 {k} 越界(分量数 {n})"),
                            ));
                        }
                        let comp_ty = self.prim_type(b, p)?;
                        let id = b.alloc_id();
                        Builder::emit(
                            &mut self.code,
                            OP_COMPOSITE_EXTRACT,
                            &[comp_ty, id, v.id, k as u32],
                        );
                        return Ok(EOperand::Value(EVal {
                            id,
                            kind: EKind::Scalar(p),
                        }));
                    }
                    Err(unmappable(
                        "ext-place",
                        format!("place `{place:?}` 投影不在图形扩展白名单"),
                    ))
                }
                Slot::Agg => {
                    let fields = self.agg.get(&place.local.0).cloned().ok_or_else(|| {
                        unmappable("ext-place", format!("聚合临时 _{} 尚未物化", place.local.0))
                    })?;
                    if place.proj.is_empty() {
                        return Ok(EOperand::Agg(fields));
                    }
                    if let Some(k) = single_field_projection(place)? {
                        return fields
                            .get(k)
                            .copied()
                            .map(EOperand::Value)
                            .ok_or_else(|| unmappable("ext-place", format!("聚合字段 {k} 越界")));
                    }
                    Err(unmappable("ext-place", "聚合临时仅支持单层字段投影"))
                }
                Slot::Zst => Ok(EOperand::Unit),
                Slot::Ret | Slot::Resource => Err(unmappable(
                    "ext-place",
                    format!("读取 place `{place:?}`(返回槽/资源句柄)不在扩展面"),
                )),
            }
        }

        /// IO 结构体形参投影读(RXS-0301 graphics_vector_component_projection):
        /// `inp.f` = 单层 Field → 整元素 Load;`inp.f.k` = 两层 Field → Load +
        /// `OpCompositeExtract k`。
        fn read_io(&mut self, b: &mut Builder, place: &Place) -> Result<EOperand, DxilError> {
            let (field, comp) = match place.proj.as_slice() {
                [ProjElem::Field(f)] => (*f as usize, None),
                [ProjElem::Field(f), ProjElem::Field(k)] => (*f as usize, Some(*k)),
                [] => {
                    return Err(unmappable(
                        "ext-place",
                        "I/O 结构体整体拷贝不在图形扩展白名单(按字段投影消费)",
                    ));
                }
                _ => {
                    return Err(unmappable(
                        "ext-place",
                        format!("I/O 投影 `{place:?}` 超出两层 Field 白名单"),
                    ));
                }
            };
            let var = self
                .input_vars
                .get(field)
                .copied()
                .ok_or_else(|| unmappable("ext-io", format!("输入 I/O 字段 {field} 越界")))?;
            let kind = ekind_of_io(var.ty);
            let ty_id = self.kind_type(b, kind)?;
            let loaded = b.alloc_id();
            Builder::emit(&mut self.code, OP_LOAD, &[ty_id, loaded, var.var_id]);
            let Some(k) = comp else {
                return Ok(EOperand::Value(EVal { id: loaded, kind }));
            };
            let EKind::Vector(p, n) = kind else {
                return Err(unmappable(
                    "ext-io",
                    format!("输入字段 {field} 非向量,不可再投影分量 {k}"),
                ));
            };
            if (k as u8) >= n {
                return Err(unmappable(
                    "ext-io",
                    format!("向量分量下标 {k} 越界(分量数 {n})"),
                ));
            }
            let comp_ty = self.prim_type(b, p)?;
            let id = b.alloc_id();
            Builder::emit(
                &mut self.code,
                OP_COMPOSITE_EXTRACT,
                &[comp_ty, id, loaded, k],
            );
            Ok(EOperand::Value(EVal {
                id,
                kind: EKind::Scalar(p),
            }))
        }

        /// 标量 local 取值(buffer 索引等场景;Var → Load,Ssa → 表值)。
        fn local_scalar(&mut self, b: &mut Builder, local: u32) -> Result<EVal, DxilError> {
            match self.slots[local as usize].clone() {
                Slot::Var {
                    var,
                    kind: kind @ EKind::Scalar(_),
                } => {
                    let ty_id = self.kind_type(b, kind)?;
                    let id = b.alloc_id();
                    Builder::emit(&mut self.code, OP_LOAD, &[ty_id, id, var]);
                    Ok(EVal { id, kind })
                }
                Slot::Ssa => self
                    .ssa
                    .get(&local)
                    .copied()
                    .ok_or_else(|| unmappable("ext-place", format!("local _{local} 尚未物化"))),
                other => Err(unmappable(
                    "ext-place",
                    format!("索引 local _{local}(槽位 {other:?})须为标量值"),
                )),
            }
        }
    }
}

// ───────────────────────── 测试(gate `dxil-backend`) ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FnColor, UnOp};
    use crate::hir::DefId;
    use crate::mir::{BasicBlock, Local, Statement, Terminator};
    use crate::span::{Edition, SourceId, Span};
    use crate::ty::Ty;

    /// 便捷构造一个 [`IoSigElem`]。
    fn elem(name: &str, kind: IoSigKind, ty: MirIoType, dir: IoDir) -> IoSigElem {
        IoSigElem {
            field_name: name.to_owned(),
            kind,
            ty,
            dir,
        }
    }

    /// 一组典型 vertex I/O:builtin position(out) + 若干 location varying + 顶点
    /// 属性输入 + builtin vertex_index(in)。
    fn vertex_set() -> Vec<IoSigElem> {
        vec![
            elem(
                "position",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            elem(
                "color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            elem(
                "uv",
                IoSigKind::Interpolate("flat".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::Out,
            ),
            elem(
                "in_pos",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 3),
                IoDir::In,
            ),
            elem(
                "vertex_index",
                IoSigKind::Builtin("vertex_index".to_owned()),
                MirIoType::Scalar(PrimTy::I32),
                IoDir::In,
            ),
        ]
    }

    /// 一组典型 fragment I/O:location 输入(含 flat 插值)+ builtin FragCoord(in)
    /// + location 输出 + builtin frag_depth(out)。
    fn fragment_set() -> Vec<IoSigElem> {
        vec![
            elem(
                "in_color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            elem(
                "in_uv",
                IoSigKind::Interpolate("flat".to_owned()),
                MirIoType::Vector(PrimTy::F32, 2),
                IoDir::In,
            ),
            elem(
                "frag_coord",
                IoSigKind::Builtin("position".to_owned()),
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::In,
            ),
            elem(
                "out_color",
                IoSigKind::Varying,
                MirIoType::Vector(PrimTy::F32, 4),
                IoDir::Out,
            ),
            elem(
                "out_depth",
                IoSigKind::Builtin("frag_depth".to_owned()),
                MirIoType::Scalar(PrimTy::F32),
                IoDir::Out,
            ),
        ]
    }

    /// 解析模块为 (opcode, operands) 指令序列(跳过 5 字 header)。
    fn instructions(module: &[u32]) -> Vec<(u16, Vec<u32>)> {
        let mut out = Vec::new();
        let mut i = 5;
        while i < module.len() {
            let word = module[i];
            let wc = (word >> 16) as usize;
            let opcode = (word & 0xFFFF) as u16;
            if wc == 0 || i + wc > module.len() {
                break;
            }
            out.push((opcode, module[i + 1..i + wc].to_vec()));
            i += wc;
        }
        out
    }

    fn dummy_span() -> Span {
        Span::new(SourceId(0), 0, 0, Edition::Rx0)
    }

    fn local(ty: Ty) -> Local {
        Local {
            ty,
            name: None,
            span: dummy_span(),
            shared: false,
            array_len: None,
        }
    }

    fn output_adt() -> Ty {
        Ty::Adt(DefId(7100), Vec::new())
    }

    fn input_adt() -> Ty {
        Ty::Adt(DefId(7101), Vec::new())
    }

    fn assign(local: LocalIdx, rv: Rvalue) -> Statement {
        Statement {
            kind: StatementKind::Assign(Place::local(local), rv),
            span: dummy_span(),
        }
    }

    fn field(local: LocalIdx, index: u32) -> Place {
        let mut place = Place::local(local);
        place.proj.push(ProjElem::Field(index));
        place
    }

    fn body_with(
        stage: ShaderStage,
        io_sig: Vec<IoSigElem>,
        locals: Vec<Local>,
        arg_count: usize,
        stmts: Vec<Statement>,
    ) -> Body {
        Body {
            def: DefId(0),
            symbol: "main".to_owned(),
            color: FnColor::Kernel,
            generic_args: Vec::new(),
            locals,
            arg_count,
            blocks: vec![BasicBlock {
                stmts,
                terminator: Terminator {
                    kind: TerminatorKind::Return,
                    span: dummy_span(),
                },
            }],
            span: dummy_span(),
            stage: Some(stage),
            io_sig,
            resources: Vec::new(),
            mesh_meta: None,
            accel_params: Vec::new(),
            compute_numthreads: None,
        }
    }

    fn variable_ids(instrs: &[(u16, Vec<u32>)], storage: u32) -> Vec<u32> {
        instrs
            .iter()
            .filter(|(op, ops)| *op == OP_VARIABLE && ops.get(2) == Some(&storage))
            .map(|(_, ops)| ops[1])
            .collect()
    }

    // ── 结构性单测(不依赖 spirv-val,恒跑) ──

    #[test]
    fn header_shape_is_correct() {
        let m = emit_spirv(ShaderStage::Vertex, &vertex_set(), &[]).expect("vertex emit ok");
        assert!(m.len() >= 5, "module 至少含 header 5 字");
        assert_eq!(m[0], SPIRV_MAGIC, "word0 = magic");
        assert_eq!(m[1], SPIRV_VERSION_1_0, "word1 = version 1.0");
        assert_eq!(m[2], SPIRV_GENERATOR, "word2 = generator");
        assert!(m[3] > 1, "word3 = bound (> 1)");
        assert_eq!(m[4], SPIRV_SCHEMA, "word4 = schema 0");
        // bound 为合理小整数(id 数 < 总字数);精确 bound=max_id+1 由构造保证。
        assert!(m[3] >= 6, "bound 至少覆盖 void/fn/main/label 等基础 id");
        assert!((m[3] as usize) <= m.len(), "bound(id 数)不应超过模块总字数");
    }

    #[test]
    fn vertex_module_has_entrypoint_and_decorations() {
        let m = emit_spirv(ShaderStage::Vertex, &vertex_set(), &[]).expect("vertex emit ok");
        let instrs = instructions(&m);

        // 含 capability / memory model / entry point。
        assert!(
            instrs
                .iter()
                .any(|(op, ops)| *op == OP_CAPABILITY && ops == &[CAP_SHADER])
        );
        assert!(instrs.iter().any(|(op, _)| *op == OP_ENTRY_POINT));
        let (_, ep_ops) = instrs.iter().find(|(op, _)| *op == OP_ENTRY_POINT).unwrap();
        assert_eq!(ep_ops[0], EXEC_MODEL_VERTEX, "vertex execution model");

        // 含 BuiltIn 装饰(position/vertex_index)、Location 装饰(varying)、
        // UserSemantic 保名装饰。
        assert!(
            instrs
                .iter()
                .any(|(op, ops)| *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_BUILTIN)),
            "应含 BuiltIn 装饰"
        );
        assert!(
            instrs
                .iter()
                .any(|(op, ops)| *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_LOCATION)),
            "应含 Location 装饰"
        );
        assert!(
            instrs.iter().any(
                |(op, ops)| *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_USER_SEMANTIC)
            ),
            "应含 UserSemantic 保名装饰"
        );
        // 用到 UserSemantic 时必 emit 扩展指令。
        assert!(
            instrs.iter().any(|(op, _)| *op == OP_EXTENSION),
            "应含 OpExtension"
        );

        // 平凡 passthrough main:含 OpFunction/OpReturn/OpFunctionEnd。
        assert!(instrs.iter().any(|(op, _)| *op == OP_FUNCTION));
        assert!(instrs.iter().any(|(op, _)| *op == OP_RETURN));
        assert!(instrs.iter().any(|(op, _)| *op == OP_FUNCTION_END));
    }

    #[test]
    fn fragment_module_has_origin_upper_left() {
        let m = emit_spirv(ShaderStage::Fragment, &fragment_set(), &[]).expect("fragment emit ok");
        let instrs = instructions(&m);
        let (_, ep_ops) = instrs.iter().find(|(op, _)| *op == OP_ENTRY_POINT).unwrap();
        assert_eq!(ep_ops[0], EXEC_MODEL_FRAGMENT, "fragment execution model");
        assert!(
            instrs.iter().any(|(op, ops)| *op == OP_EXECUTION_MODE
                && ops.get(1) == Some(&EXEC_MODE_ORIGIN_UPPER_LEFT)),
            "fragment 须含 OriginUpperLeft execution mode"
        );
    }

    #[test]
    fn vertex_has_no_execution_mode() {
        let m = emit_spirv(ShaderStage::Vertex, &vertex_set(), &[]).expect("vertex emit ok");
        let instrs = instructions(&m);
        assert!(
            !instrs.iter().any(|(op, _)| *op == OP_EXECUTION_MODE),
            "vertex 不应 emit OriginUpperLeft execution mode"
        );
    }

    /// RXS-0171:输出 I/O 聚合返回值机械分解为逐 Output 元素 OpStore。
    //@ spec: RXS-0171
    #[test]
    fn body_output_aggregate_return_splits_to_store() {
        let out_ty = output_adt();
        let temp = LocalIdx(1);
        let body = body_with(
            ShaderStage::Fragment,
            vec![elem(
                "out_luma",
                IoSigKind::Varying,
                MirIoType::Scalar(PrimTy::F32),
                IoDir::Out,
            )],
            vec![local(out_ty.clone()), local(out_ty.clone())],
            0,
            vec![
                assign(
                    temp,
                    Rvalue::Aggregate(
                        out_ty.clone(),
                        vec![Operand::Const(Const::Float(0.5, PrimTy::F32))],
                    ),
                ),
                assign(LocalIdx(0), Rvalue::Use(Operand::Move(Place::local(temp)))),
            ],
        );
        let m = emit_spirv_body(ShaderStage::Fragment, &body).expect("body lowering ok");
        let instrs = instructions(&m);
        assert!(instrs.iter().any(|(op, _)| *op == OP_CONSTANT));
        assert!(instrs.iter().any(|(op, _)| *op == OP_STORE));
    }

    /// RXS-0171:参数结构体字段声明序绑定 In 元素,返回结构体字段声明序绑定 Out 元素。
    //@ spec: RXS-0171
    #[test]
    fn body_field_order_binding_drives_load_and_store_order() {
        let out_ty = output_adt();
        let body = body_with(
            ShaderStage::Fragment,
            vec![
                elem(
                    "a",
                    IoSigKind::Varying,
                    MirIoType::Scalar(PrimTy::F32),
                    IoDir::In,
                ),
                elem(
                    "b",
                    IoSigKind::Varying,
                    MirIoType::Scalar(PrimTy::F32),
                    IoDir::In,
                ),
                elem(
                    "x",
                    IoSigKind::Varying,
                    MirIoType::Scalar(PrimTy::F32),
                    IoDir::Out,
                ),
                elem(
                    "y",
                    IoSigKind::Varying,
                    MirIoType::Scalar(PrimTy::F32),
                    IoDir::Out,
                ),
            ],
            vec![local(out_ty.clone()), local(input_adt())],
            1,
            vec![assign(
                LocalIdx(0),
                Rvalue::Aggregate(
                    out_ty,
                    vec![
                        Operand::Copy(field(LocalIdx(1), 1)),
                        Operand::Copy(field(LocalIdx(1), 0)),
                    ],
                ),
            )],
        );
        let m = emit_spirv_body(ShaderStage::Fragment, &body).expect("body lowering ok");
        let instrs = instructions(&m);
        let inputs = variable_ids(&instrs, STORAGE_INPUT);
        let outputs = variable_ids(&instrs, STORAGE_OUTPUT);
        let loads: Vec<u32> = instrs
            .iter()
            .filter(|(op, _)| *op == OP_LOAD)
            .map(|(_, ops)| ops[2])
            .collect();
        let stores: Vec<u32> = instrs
            .iter()
            .filter(|(op, _)| *op == OP_STORE)
            .map(|(_, ops)| ops[0])
            .collect();
        assert_eq!(
            loads,
            vec![inputs[1], inputs[0]],
            "Field(1), Field(0) 绑定 In 序"
        );
        assert_eq!(stores, outputs, "输出聚合按 Out 声明序 store");
    }

    /// RXS-0171:输入 place load + f32 常量 + 标量二元算术 + 输出 store。
    //@ spec: RXS-0171
    #[test]
    fn body_binary_arithmetic_lowers_to_spirv_ops() {
        let out_ty = output_adt();
        let sum = LocalIdx(2);
        let body = body_with(
            ShaderStage::Fragment,
            vec![
                elem(
                    "in_luma",
                    IoSigKind::Varying,
                    MirIoType::Scalar(PrimTy::F32),
                    IoDir::In,
                ),
                elem(
                    "out_luma",
                    IoSigKind::Varying,
                    MirIoType::Scalar(PrimTy::F32),
                    IoDir::Out,
                ),
            ],
            vec![
                local(out_ty.clone()),
                local(input_adt()),
                local(Ty::Prim(PrimTy::F32)),
            ],
            1,
            vec![
                assign(
                    sum,
                    Rvalue::BinaryOp(
                        BinOp::Add,
                        Operand::Copy(field(LocalIdx(1), 0)),
                        Operand::Const(Const::Float(1.0, PrimTy::F32)),
                    ),
                ),
                assign(
                    LocalIdx(0),
                    Rvalue::Aggregate(out_ty, vec![Operand::Copy(Place::local(sum))]),
                ),
            ],
        );
        let m = emit_spirv_body(ShaderStage::Fragment, &body).expect("body lowering ok");
        let instrs = instructions(&m);
        assert!(instrs.iter().any(|(op, _)| *op == OP_LOAD));
        assert!(instrs.iter().any(|(op, _)| *op == OP_CONSTANT));
        assert!(instrs.iter().any(|(op, _)| *op == OP_FADD));
        assert!(instrs.iter().any(|(op, _)| *op == OP_STORE));
    }

    /// RXS-0171 strict-only:白名单外 rvalue 不可映射(上层映射 RX6013)。
    //@ spec: RXS-0171
    #[test]
    fn body_unsupported_rvalue_is_unmappable() {
        let out_ty = output_adt();
        let body = body_with(
            ShaderStage::Fragment,
            vec![elem(
                "out_luma",
                IoSigKind::Varying,
                MirIoType::Scalar(PrimTy::F32),
                IoDir::Out,
            )],
            vec![local(out_ty)],
            0,
            vec![assign(
                LocalIdx(0),
                Rvalue::UnaryOp(UnOp::Neg, Operand::Const(Const::Float(1.0, PrimTy::F32))),
            )],
        );
        let r = emit_spirv_body(ShaderStage::Fragment, &body);
        assert!(
            matches!(r, Err(DxilError::Unmappable { .. })),
            "unsupported rvalue 必须 strict-only 拒绝, 实得 {r:?}"
        );
    }

    // ── strict-only:不可映射构造必 Err,绝不 Ok ──

    #[test]
    fn unmappable_scalar_type_is_rejected() {
        // f64 不在已建模标量子集(f32/i32/u32)。
        let io = vec![elem(
            "weird",
            IoSigKind::Varying,
            MirIoType::Scalar(PrimTy::F64),
            IoDir::Out,
        )];
        let r = emit_spirv(ShaderStage::Vertex, &io, &[]);
        assert!(
            matches!(r, Err(DxilError::Unmappable { .. })),
            "f64 应不可映射, got {r:?}"
        );
    }

    #[test]
    fn unmodeled_builtin_is_rejected() {
        let io = vec![elem(
            "foobar",
            IoSigKind::Builtin("foobar".to_owned()),
            MirIoType::Vector(PrimTy::F32, 4),
            IoDir::Out,
        )];
        let r = emit_spirv(ShaderStage::Vertex, &io, &[]);
        assert!(
            matches!(r, Err(DxilError::Unmappable { .. })),
            "未建模 builtin 应不可映射, got {r:?}"
        );
    }

    #[test]
    fn builtin_type_mismatch_is_rejected() {
        // position 须 vec4<f32>;给 vec2 应不可映射。
        let io = vec![elem(
            "position",
            IoSigKind::Builtin("position".to_owned()),
            MirIoType::Vector(PrimTy::F32, 2),
            IoDir::Out,
        )];
        let r = emit_spirv(ShaderStage::Vertex, &io, &[]);
        assert!(
            matches!(r, Err(DxilError::Unmappable { .. })),
            "builtin 类型不符应不可映射, got {r:?}"
        );
    }

    #[test]
    fn non_graphics_stage_is_rejected() {
        let r = emit_spirv(ShaderStage::Compute, &[], &[]);
        assert!(
            matches!(r, Err(DxilError::Unmappable { .. })),
            "compute 阶段不在编码器范围, got {r:?}"
        );
    }

    #[test]
    fn out_of_range_vector_width_is_rejected() {
        let io = vec![elem(
            "big",
            IoSigKind::Varying,
            MirIoType::Vector(PrimTy::F32, 5),
            IoDir::Out,
        )];
        let r = emit_spirv(ShaderStage::Vertex, &io, &[]);
        assert!(
            matches!(r, Err(DxilError::Unmappable { .. })),
            "向量宽度越界应不可映射, got {r:?}"
        );
    }

    // ── Property 1(编码器合规性):产物喂本机 spirv-val,无 error;不可用则 SKIP ──

    enum ValResult {
        Skip,
        Pass,
        Fail(String),
    }

    fn run_spirv_val(words: &[u32], tag: &str) -> ValResult {
        let Some(tool) = crate::toolchain::locate_spirv_val() else {
            return ValResult::Skip;
        };
        let mut bytes = Vec::with_capacity(words.len() * 4);
        for w in words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        let path =
            std::env::temp_dir().join(format!("rurix_spv_{}_{}.spv", std::process::id(), tag));
        if std::fs::write(&path, &bytes).is_err() {
            return ValResult::Skip;
        }
        let output = std::process::Command::new(&tool).arg(&path).output();
        let _ = std::fs::remove_file(&path);
        match output {
            // spawn 失败(工具不存在/不可执行)→ SKIP(对齐 RXS-0073 干验证纪律)。
            Err(_) => ValResult::Skip,
            Ok(o) if o.status.success() => ValResult::Pass,
            Ok(o) => ValResult::Fail(format!(
                "spirv-val 拒绝 {tag}: stdout={} stderr={}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            )),
        }
    }

    #[test]
    fn property1_encoder_products_pass_spirv_val() {
        let cases: Vec<(&str, ShaderStage, Vec<IoSigElem>)> = vec![
            ("vertex_full", ShaderStage::Vertex, vertex_set()),
            ("fragment_full", ShaderStage::Fragment, fragment_set()),
            (
                "vertex_min",
                ShaderStage::Vertex,
                vec![elem(
                    "position",
                    IoSigKind::Builtin("position".to_owned()),
                    MirIoType::Vector(PrimTy::F32, 4),
                    IoDir::Out,
                )],
            ),
            (
                "fragment_min",
                ShaderStage::Fragment,
                vec![elem(
                    "out_color",
                    IoSigKind::Varying,
                    MirIoType::Vector(PrimTy::F32, 4),
                    IoDir::Out,
                )],
            ),
            (
                "vertex_idx_inputs",
                ShaderStage::Vertex,
                vec![
                    elem(
                        "vertex_index",
                        IoSigKind::Builtin("vertex_index".to_owned()),
                        MirIoType::Scalar(PrimTy::U32),
                        IoDir::In,
                    ),
                    elem(
                        "instance_index",
                        IoSigKind::Builtin("instance_index".to_owned()),
                        MirIoType::Scalar(PrimTy::I32),
                        IoDir::In,
                    ),
                    elem(
                        "position",
                        IoSigKind::Builtin("position".to_owned()),
                        MirIoType::Vector(PrimTy::F32, 4),
                        IoDir::Out,
                    ),
                ],
            ),
        ];

        let mut skipped = false;
        for (tag, stage, io) in &cases {
            let spv =
                emit_spirv(*stage, io, &[]).unwrap_or_else(|e| panic!("emit {tag} failed: {e}"));
            match run_spirv_val(&spv, tag) {
                ValResult::Skip => {
                    skipped = true;
                }
                ValResult::Pass => {
                    eprintln!("[OK] spirv-val 通过: {tag}");
                }
                ValResult::Fail(msg) => panic!("{msg}"),
            }
        }
        if skipped {
            eprintln!("[SKIP] spirv-val 不可用(真实红绿在带 SPIRV-Tools 的 dev/owner 环境)");
        }
    }

    /// 资源句柄绑定 emit(RXS-0163;PR-E2b 闭合 assumed-1):`Texture2D<F>` + `Sampler`
    /// → opaque 资源类型(`OpTypeImage`/`OpTypeSampler`)+ `DescriptorSet`/`Binding`
    /// 装饰(host 推导给定 set/binding,声明序确定性);并经本机 spirv-val(可用则)。
    #[test]
    fn resource_bindings_emit_decorations_and_pass_val() {
        use crate::mir::ResourceCount;

        let resources = vec![
            ResourceBinding {
                name: "tex".to_owned(),
                res: MirResourceType::Texture2D(PrimTy::F32),
                count: ResourceCount::One,
            },
            ResourceBinding {
                name: "samp".to_owned(),
                res: MirResourceType::Sampler,
                count: ResourceCount::One,
            },
        ];
        // 含一个 builtin 输出以构成合法 fragment(out_color varying)。
        let io = vec![elem(
            "out_color",
            IoSigKind::Varying,
            MirIoType::Vector(PrimTy::F32, 4),
            IoDir::Out,
        )];
        let m = emit_spirv(ShaderStage::Fragment, &io, &resources).expect("资源 emit 应 Ok");
        let instrs = instructions(&m);

        // OpTypeImage + OpTypeSampler 各一。
        assert!(
            instrs.iter().any(|(op, _)| *op == OP_TYPE_IMAGE),
            "Texture2D 应 emit OpTypeImage"
        );
        assert!(
            instrs.iter().any(|(op, _)| *op == OP_TYPE_SAMPLER),
            "Sampler 应 emit OpTypeSampler"
        );
        // DescriptorSet(恒 0)+ Binding(0,1)装饰。
        let sets: Vec<u32> = instrs
            .iter()
            .filter(|(op, ops)| {
                *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_DESCRIPTOR_SET)
            })
            .map(|(_, ops)| ops[2])
            .collect();
        let bindings: Vec<u32> = instrs
            .iter()
            .filter(|(op, ops)| *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_BINDING))
            .map(|(_, ops)| ops[2])
            .collect();
        assert_eq!(sets, vec![0, 0], "首期单 set");
        // tex(SRV 轴)与 samp(Sampler 轴)各为不同种类轴 → per-class binding 各从 0
        // (RXS-0164;与 RTS0 register t0/s0 同口径,RFC-0007 对齐,sampler 不再落 s1)。
        assert_eq!(bindings, vec![0, 0], "Binding 按种类轴 per-class 从 0");

        // 资源 UniformConstant 变量不入 OpEntryPoint interface(SPIR-V 1.0)。
        let (_, ep_ops) = instrs.iter().find(|(op, _)| *op == OP_ENTRY_POINT).unwrap();
        // interface 段在 model + main + "main"(变长字串)之后;仅断言计数不含资源:
        // 接口只列 Input/Output(out_color 一个 location 输出)。这里以变量总数 vs
        // 接口长度的间接关系不易精确,转而断言 spirv-val 接受(下)即足。
        let _ = ep_ops;

        match run_spirv_val(&m, "fragment_resources") {
            ValResult::Skip => {
                eprintln!("[SKIP] spirv-val 不可用(资源绑定真实红绿在带 SPIRV-Tools 环境)")
            }
            ValResult::Pass => eprintln!("[OK] spirv-val 通过: fragment_resources"),
            ValResult::Fail(msg) => panic!("{msg}"),
        }
    }

    /// RXS-0230(E-3;RFC-0013 §4.B7):Vulkan 原生路(provenance=false)绑定装饰切
    /// **Vk-native set-per-class**(SRV→set1 / Sampler→set3),与 `run_graphics_offscreen_v2`
    /// 的 `plan_descriptor_sets` 分配律对齐;binding 号与 B 链同一事实源(per-class 从 0)。
    /// DXIL 路(provenance=true,`resource_bindings_emit_decorations_and_pass_val`)维持 set0
    /// 装饰字节不动——两测并列证「按目标选择两套 set 策略、单一 binding-号事实源」。
    #[test]
    fn vulkan_resource_bindings_use_set_per_class() {
        use crate::mir::ResourceCount;

        let resources = vec![
            ResourceBinding {
                name: "tex".to_owned(),
                res: MirResourceType::Texture2D(PrimTy::F32),
                count: ResourceCount::One,
            },
            ResourceBinding {
                name: "samp".to_owned(),
                res: MirResourceType::Sampler,
                count: ResourceCount::One,
            },
        ];
        let io = vec![elem(
            "out_color",
            IoSigKind::Varying,
            MirIoType::Vector(PrimTy::F32, 4),
            IoDir::Out,
        )];
        // provenance=false = Vulkan 原生路(emit_spirv_body_vulkan / --target vulkan 同路)。
        let m = emit_spirv_inner(ShaderStage::Fragment, &io, &resources, None, false)
            .expect("Vulkan 资源 emit 应 Ok");
        let instrs = instructions(&m);
        let sets: Vec<u32> = instrs
            .iter()
            .filter(|(op, ops)| {
                *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_DESCRIPTOR_SET)
            })
            .map(|(_, ops)| ops[2])
            .collect();
        let bindings: Vec<u32> = instrs
            .iter()
            .filter(|(op, ops)| *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_BINDING))
            .map(|(_, ops)| ops[2])
            .collect();
        // SRV 轴 → set1 / Sampler 轴 → set3(class_to_vk_set;plan_descriptor_sets 镜像)。
        assert_eq!(
            sets,
            vec![1, 3],
            "Vulkan 原生路 set = 类别轴(SRV=1/Sampler=3)"
        );
        // binding 号与 B 链同一事实源:per-class 各从 0(单一 binding-号事实源)。
        assert_eq!(bindings, vec![0, 0], "binding 与 B 链同源(per-class 从 0)");

        match run_spirv_val(&m, "vulkan_set_per_class") {
            ValResult::Skip => {
                eprintln!("[SKIP] spirv-val 不可用(Vk-native 绑定真实红绿在带 SPIRV-Tools 环境)")
            }
            ValResult::Pass => eprintln!("[OK] spirv-val 通过: vulkan_set_per_class"),
            ValResult::Fail(msg) => panic!("{msg}"),
        }
    }

    /// G3.4(RXS-0234;RFC-0013 §4.C3):无界 SRV 纹理表 `[Texture2D<F>]` 合法化——
    /// emit `OpTypeRuntimeArray` + `RuntimeDescriptorArray`/`ShaderNonUniform`
    /// capability + `SPV_EXT_descriptor_indexing` 扩展(自 Unmappable 翻转)。
    //@ spec: RXS-0234
    #[test]
    fn unbounded_srv_texture_emits_runtime_array() {
        use crate::mir::ResourceCount;
        let resources = vec![ResourceBinding {
            name: "table".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::Unbounded,
        }];
        let m = emit_spirv(ShaderStage::Fragment, &[], &resources)
            .expect("无界 SRV 纹理表应合法 emit(RXS-0234)");
        let instrs = instructions(&m);
        assert!(
            instrs.iter().any(|(op, _)| *op == OP_TYPE_RUNTIME_ARRAY),
            "应 emit OpTypeRuntimeArray(无界表元素)"
        );
        let caps: Vec<u32> = instrs
            .iter()
            .filter(|(op, _)| *op == OP_CAPABILITY)
            .map(|(_, ops)| ops[0])
            .collect();
        assert!(
            caps.contains(&CAP_RUNTIME_DESCRIPTOR_ARRAY),
            "应 emit RuntimeDescriptorArray capability"
        );
        assert!(
            caps.contains(&CAP_SHADER_NON_UNIFORM),
            "应 emit ShaderNonUniform capability"
        );
        // SPV_EXT_descriptor_indexing 扩展 emit(OpExtension 首操作数 = 扩展名字符串)。
        assert!(
            instrs.iter().any(|(op, _)| *op == OP_EXTENSION),
            "应 emit OpExtension SPV_EXT_descriptor_indexing"
        );

        match run_spirv_val(&m, "bindless_runtime_array") {
            ValResult::Skip => {
                eprintln!("[SKIP] spirv-val 不可用(bindless 无界表真实红绿在带 SPIRV-Tools 环境)")
            }
            ValResult::Pass => eprintln!("[OK] spirv-val 通过: bindless_runtime_array"),
            ValResult::Fail(msg) => panic!("{msg}"),
        }
    }

    /// reject(维持):无界**非-SRV-纹理**表(无界 Sampler)→ Unmappable/RX6013(§8,不新码)。
    #[test]
    fn unbounded_non_texture_still_unmappable() {
        use crate::mir::ResourceCount;
        let resources = vec![ResourceBinding {
            name: "samps".to_owned(),
            res: MirResourceType::Sampler,
            count: ResourceCount::Unbounded,
        }];
        let r = emit_spirv(ShaderStage::Fragment, &[], &resources);
        assert!(
            matches!(r, Err(DxilError::Unmappable { .. })),
            "无界非纹理表应维持 Unmappable(§8),实得 {r:?}"
        );
    }

    // ── Scheme B（codegen provenance gate，RXS-0210；仅 vulkan-backend 起门，
    //    dxil-backend 单独启用 test 数不受影响 → 保 404 字节不变基准）──

    /// 便捷构造一个「含具名 Out varying」的最小 fragment body（具名 → 触 UserSemantic
    /// provenance 路径；DXIL 保名 vs Vulkan 去名的差异全在此）。
    #[cfg(feature = "vulkan-backend")]
    fn provenance_probe_body() -> Body {
        let out_ty = output_adt();
        body_with(
            ShaderStage::Fragment,
            vec![elem(
                "out_luma",
                IoSigKind::Varying,
                MirIoType::Scalar(PrimTy::F32),
                IoDir::Out,
            )],
            vec![local(out_ty.clone()), local(out_ty)],
            0,
            vec![
                assign(
                    LocalIdx(1),
                    Rvalue::Aggregate(
                        output_adt(),
                        vec![Operand::Const(Const::Float(0.5, PrimTy::F32))],
                    ),
                ),
                assign(
                    LocalIdx(0),
                    Rvalue::Use(Operand::Move(Place::local(LocalIdx(1)))),
                ),
            ],
        )
    }

    /// RXS-0210：Vulkan 原生路（`emit_spirv_body_vulkan`，provenance=false）**不 emit**
    /// UserSemantic 装饰、**不 emit** `OpExtension SPV_GOOGLE_hlsl_functionality1`
    /// —— 即修 VUID-...-08742 的方案 B（去装饰而非产非法 SPIR-V）。
    //@ spec: RXS-0210
    #[cfg(feature = "vulkan-backend")]
    #[test]
    fn vulkan_variant_omits_user_semantic_and_extension() {
        let body = provenance_probe_body();
        // 第一遍成功路(res 不消费,空 Resolutions 即可)。
        let m = emit_spirv_body_vulkan(
            ShaderStage::Fragment,
            &body,
            &crate::resolve::Resolutions::default(),
        )
        .expect("Vulkan 变体 body lowering 应 Ok");
        let instrs = instructions(&m);
        assert!(
            !instrs.iter().any(
                |(op, ops)| *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_USER_SEMANTIC)
            ),
            "Vulkan 原生路不应 emit UserSemantic 装饰"
        );
        assert!(
            !instrs.iter().any(|(op, _)| *op == OP_EXTENSION),
            "Vulkan 原生路不应 emit OpExtension（SPV_GOOGLE 靠 used_user_semantic 自然为 false）"
        );
        // Location 装饰仍在（Vulkan 按 Location 消费，去的只是 provenance）。
        assert!(
            instrs
                .iter()
                .any(|(op, ops)| *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_LOCATION)),
            "Vulkan 原生路仍应保留 Location 装饰"
        );
    }

    /// RXS-0210：DXIL 路（`emit_spirv_body`，provenance=true）**保留** UserSemantic +
    /// `OpExtension SPV_GOOGLE`（保名字节不变，B 路 HLSL 转译边界消费）—— 证方案 B 是
    /// target-conditional 去装饰，DXIL 路零回归。
    //@ spec: RXS-0210
    #[cfg(feature = "vulkan-backend")]
    #[test]
    fn dxil_variant_keeps_user_semantic_and_extension() {
        let body = provenance_probe_body();
        let m =
            emit_spirv_body(ShaderStage::Fragment, &body).expect("DXIL 变体 body lowering 应 Ok");
        let instrs = instructions(&m);
        assert!(
            instrs.iter().any(
                |(op, ops)| *op == OP_DECORATE && ops.get(1) == Some(&DECORATION_USER_SEMANTIC)
            ),
            "DXIL 路应保留 UserSemantic provenance 装饰"
        );
        assert!(
            instrs.iter().any(|(op, _)| *op == OP_EXTENSION),
            "DXIL 路应保留 OpExtension SPV_GOOGLE_hlsl_functionality1"
        );
    }

    // ── 采样方法族 opcode 全家(RXS-0223/0226~0229;结构性单测,不依赖 spirv-val 恒跑) ──

    /// 构造一个 fragment body:输入 I/O 字段(coord/val/grad 等)+ 具名资源句柄形参 +
    /// 单条 `assign(temp, rv)` 语句(rv = 采样方法族 rvalue,texture_local=2 / sampler_local=3)。
    fn emit_resource_op_body(
        in_fields: Vec<(&str, MirIoType)>,
        resources: Vec<ResourceBinding>,
        rv: Rvalue,
    ) -> Result<Vec<u32>, DxilError> {
        let io_sig: Vec<IoSigElem> = in_fields
            .iter()
            .map(|(n, ty)| elem(n, IoSigKind::Varying, *ty, IoDir::In))
            .collect();
        // locals: 0=ret / 1=input struct arg / 2.. = 具名资源句柄 / 末=temp result。
        let mut locals = vec![local(output_adt()), local(input_adt())];
        for res in &resources {
            locals.push(Local {
                ty: output_adt(),
                name: Some(res.name.clone()),
                span: dummy_span(),
                shared: false,
                array_len: None,
            });
        }
        let temp = LocalIdx(locals.len() as u32);
        locals.push(local(output_adt()));
        let arg_count = 1 + resources.len();
        let body = Body {
            def: DefId(0),
            symbol: "main".to_owned(),
            color: FnColor::Kernel,
            generic_args: Vec::new(),
            locals,
            arg_count,
            blocks: vec![BasicBlock {
                stmts: vec![assign(temp, rv)],
                terminator: Terminator {
                    kind: TerminatorKind::Return,
                    span: dummy_span(),
                },
            }],
            span: dummy_span(),
            stage: Some(ShaderStage::Fragment),
            io_sig,
            resources,
            mesh_meta: None,
            accel_params: Vec::new(),
            compute_numthreads: None,
        };
        emit_spirv_body(ShaderStage::Fragment, &body)
    }

    fn rb(name: &str, res: MirResourceType) -> ResourceBinding {
        ResourceBinding {
            name: name.to_owned(),
            res,
            count: crate::mir::ResourceCount::One,
        }
    }

    /// coord = 读输入 I/O 字段 idx(identity 位置标识形态,RXS-0229)。
    fn coord_field(idx: u32) -> Operand {
        Operand::Copy(field(LocalIdx(1), idx))
    }

    fn has_op(m: &[u32], op: u16) -> bool {
        instructions(m).iter().any(|(o, _)| *o == op)
    }

    /// RXS-0226/0227:`sample`(隐式 LOD)→ `OpImageSampleImplicitLod`(无 Lod operand)。
    #[test]
    fn sample_lowers_to_implicit_lod() {
        //@ spec: RXS-0223
        //@ spec: RXS-0226
        //@ spec: RXS-0227
        let m = emit_resource_op_body(
            vec![("uv", MirIoType::Vector(PrimTy::F32, 2))],
            vec![
                rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
                rb("samp", MirResourceType::Sampler),
            ],
            Rvalue::ResourceSample {
                texture_local: LocalIdx(2),
                sampler_local: Some(LocalIdx(3)),
                table_index: None,
                method: ResourceMethod::Sample,
                coord: coord_field(0),
                extra: Vec::new(),
            },
        )
        .expect("sample 隐式 LOD 应 Ok");
        assert!(
            has_op(&m, OP_IMAGE_SAMPLE_IMPLICIT_LOD),
            "sample → OpImageSampleImplicitLod"
        );
        assert!(
            !has_op(&m, OP_IMAGE_SAMPLE_EXPLICIT_LOD),
            "sample 不应 emit ExplicitLod"
        );
    }

    /// RXS-0223:既有 `.sample()` 迁移 = `sample_lod` 空 extra → `OpImageSampleExplicitLod`(Lod 0)。
    #[test]
    fn sample_lod_empty_extra_lowers_to_explicit_lod0() {
        //@ spec: RXS-0223
        let m = emit_resource_op_body(
            vec![("uv", MirIoType::Vector(PrimTy::F32, 2))],
            vec![
                rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
                rb("samp", MirResourceType::Sampler),
            ],
            Rvalue::ResourceSample {
                texture_local: LocalIdx(2),
                sampler_local: Some(LocalIdx(3)),
                table_index: None,
                method: ResourceMethod::SampleLod,
                coord: coord_field(0),
                extra: Vec::new(),
            },
        )
        .expect("sample_lod 空 extra 应 Ok");
        assert!(
            has_op(&m, OP_IMAGE_SAMPLE_EXPLICIT_LOD),
            "sample_lod → OpImageSampleExplicitLod"
        );
    }

    /// RXS-0226:`sample_grad` → `OpImageSampleExplicitLod`(Grad ddx/ddy)。
    #[test]
    fn sample_grad_lowers_to_explicit_grad() {
        //@ spec: RXS-0226
        let m = emit_resource_op_body(
            vec![
                ("uv", MirIoType::Vector(PrimTy::F32, 2)),
                ("ddx", MirIoType::Vector(PrimTy::F32, 2)),
                ("ddy", MirIoType::Vector(PrimTy::F32, 2)),
            ],
            vec![
                rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
                rb("samp", MirResourceType::Sampler),
            ],
            Rvalue::ResourceSample {
                texture_local: LocalIdx(2),
                sampler_local: Some(LocalIdx(3)),
                table_index: None,
                method: ResourceMethod::SampleGrad,
                coord: coord_field(0),
                extra: vec![coord_field(1), coord_field(2)],
            },
        )
        .expect("sample_grad 应 Ok");
        let grad = instructions(&m)
            .into_iter()
            .find(|(op, _)| *op == OP_IMAGE_SAMPLE_EXPLICIT_LOD)
            .expect("应 emit ExplicitLod");
        assert!(
            grad.1.contains(&IMAGE_OPERANDS_GRAD),
            "ImageOperands 应含 Grad bit"
        );
    }

    /// RXS-0228:`load` → `OpImageFetch` + 越界钳制序列(ImageQuery cap + QuerySizeLod + UMin)。
    #[test]
    fn texel_fetch_lowers_with_clamp_sequence() {
        //@ spec: RXS-0228
        let m = emit_resource_op_body(
            vec![("px", MirIoType::Vector(PrimTy::U32, 2))],
            vec![rb("tex", MirResourceType::Texture2D(PrimTy::F32))],
            Rvalue::ResourceSample {
                texture_local: LocalIdx(2),
                sampler_local: None,
                table_index: None,
                method: ResourceMethod::Load,
                coord: coord_field(0),
                extra: Vec::new(),
            },
        )
        .expect("load texel fetch 应 Ok");
        assert!(has_op(&m, OP_IMAGE_FETCH), "load → OpImageFetch");
        assert!(
            has_op(&m, OP_IMAGE_QUERY_SIZE_LOD),
            "越界钳制应 emit OpImageQuerySizeLod"
        );
        assert!(
            has_op(&m, OP_EXT_INST),
            "越界钳制应 emit OpExtInst(GLSL.std.450 UMin)"
        );
        assert!(
            has_op(&m, OP_EXT_INST_IMPORT),
            "应 emit OpExtInstImport GLSL.std.450"
        );
        // ImageQuery capability 声明(RXS-0228)。
        let caps: Vec<u32> = instructions(&m)
            .iter()
            .filter(|(op, _)| *op == OP_CAPABILITY)
            .map(|(_, ops)| ops[0])
            .collect();
        assert!(
            caps.contains(&CAP_IMAGE_QUERY),
            "应声明 ImageQuery 能力,实得 {caps:?}"
        );
    }

    /// RXS-0229:`TextureRw2D.store`(identity 坐标)→ `OpImageWrite`(唯一写者)。
    #[test]
    fn storage_store_identity_lowers_to_image_write() {
        //@ spec: RXS-0229
        let m = emit_resource_op_body(
            vec![
                ("px", MirIoType::Vector(PrimTy::U32, 2)),
                ("val", MirIoType::Vector(PrimTy::F32, 4)),
            ],
            vec![rb("img", MirResourceType::TextureRw2D(PrimTy::F32))],
            Rvalue::ResourceSample {
                texture_local: LocalIdx(2),
                sampler_local: None,
                table_index: None,
                method: ResourceMethod::Store,
                coord: coord_field(0),
                extra: vec![coord_field(1)],
            },
        )
        .expect("identity store 应 Ok");
        assert!(has_op(&m, OP_IMAGE_WRITE), "store → OpImageWrite");
        // storage image 类型带显式 format(Rgba32f)。
        let img_ty = instructions(&m)
            .into_iter()
            .find(|(op, _)| *op == OP_TYPE_IMAGE)
            .expect("应 emit OpTypeImage");
        assert_eq!(
            img_ty.1.last(),
            Some(&IMAGE_FORMAT_RGBA32F),
            "storage image 应带显式 format Rgba32f"
        );
    }

    /// RXS-0229:非 identity(常量)坐标 store → strict-only 拒(唯一写者纪律,可 golden)。
    #[test]
    fn storage_store_nonidentity_rejects() {
        //@ spec: RXS-0229
        let r = emit_resource_op_body(
            vec![("val", MirIoType::Vector(PrimTy::F32, 4))],
            vec![rb("img", MirResourceType::TextureRw2D(PrimTy::F32))],
            Rvalue::ResourceSample {
                texture_local: LocalIdx(2),
                sampler_local: None,
                table_index: None,
                method: ResourceMethod::Store,
                // 常量坐标 = 所有 invocation 写同一 texel = 多写者 → 非 identity。
                coord: Operand::Const(Const::Int(0, PrimTy::U32)),
                extra: vec![coord_field(0)],
            },
        );
        assert!(
            matches!(r, Err(DxilError::SampleUnsupported { .. })),
            "非 identity 坐标 store 应 strict-only 拒(唯一写者纪律 RXS-0229),实得 {r:?}"
        );
    }

    /// RXS-0226:`gather` → `OpImageGather`;`sample_cmp` → `OpImageSampleDrefExplicitLod`;
    /// `TextureRw2D.load` → `OpImageRead`(probe-gated 子模式亦落 opcode 结构)。
    #[test]
    fn gather_cmp_storageload_lower_to_family_opcodes() {
        //@ spec: RXS-0226
        let gather = emit_resource_op_body(
            vec![("uv", MirIoType::Vector(PrimTy::F32, 2))],
            vec![
                rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
                rb("samp", MirResourceType::Sampler),
            ],
            Rvalue::ResourceSample {
                texture_local: LocalIdx(2),
                sampler_local: Some(LocalIdx(3)),
                table_index: None,
                method: ResourceMethod::Gather,
                coord: coord_field(0),
                extra: vec![Operand::Const(Const::Int(1, PrimTy::U32))],
            },
        )
        .expect("gather 应 Ok");
        assert!(has_op(&gather, OP_IMAGE_GATHER), "gather → OpImageGather");

        let cmp = emit_resource_op_body(
            vec![
                ("uv", MirIoType::Vector(PrimTy::F32, 2)),
                ("dref", MirIoType::Scalar(PrimTy::F32)),
            ],
            vec![
                rb("tex", MirResourceType::Texture2D(PrimTy::F32)),
                rb("scmp", MirResourceType::SamplerCmp),
            ],
            Rvalue::ResourceSample {
                texture_local: LocalIdx(2),
                sampler_local: Some(LocalIdx(3)),
                table_index: None,
                method: ResourceMethod::SampleCmp,
                coord: coord_field(0),
                extra: vec![coord_field(1)],
            },
        )
        .expect("sample_cmp 应 Ok");
        assert!(
            has_op(&cmp, OP_IMAGE_SAMPLE_DREF_EXPLICIT_LOD),
            "sample_cmp → DrefExplicitLod"
        );

        let rwload = emit_resource_op_body(
            vec![("px", MirIoType::Vector(PrimTy::U32, 2))],
            vec![rb("img", MirResourceType::TextureRw2D(PrimTy::F32))],
            Rvalue::ResourceSample {
                texture_local: LocalIdx(2),
                sampler_local: None,
                table_index: None,
                method: ResourceMethod::StorageLoad,
                coord: coord_field(0),
                extra: Vec::new(),
            },
        )
        .expect("TextureRw2D.load 应 Ok");
        assert!(
            has_op(&rwload, OP_IMAGE_READ),
            "TextureRw2D.load → OpImageRead"
        );
    }
}
