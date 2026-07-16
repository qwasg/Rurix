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
    ProjElem, ResourceBinding, Rvalue, StatementKind, TerminatorKind,
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
/// `OpSampledImage`(组合 image + sampler 为采样图像,RXS-0175;RFC-0007)。
const OP_SAMPLED_IMAGE: u16 = 86;
/// `OpImageSampleExplicitLod`(显式 LOD 采样,首期 LOD 0 规避隐式导数,RFC-0007 §4.6)。
const OP_IMAGE_SAMPLE_EXPLICIT_LOD: u16 = 88;
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
/// `ImageOperands` `Lod` bit(0x2;显式 LOD 采样,RXS-0175)。
const IMAGE_OPERANDS_LOD: u32 = 0x2;

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
#[derive(Clone, Debug)]
struct ResourceVarInfo {
    /// 源码形参名(保名依据;BodyLowerer 按 MIR local 名匹配解析此变量)。
    name: String,
    /// SPIR-V 全局变量 id(`UniformConstant` 存储类)。
    var_id: u32,
    /// 资源 SPIR-V 类型 id(image / sampler)。
    type_id: u32,
    /// 是否为纹理图像(true=Texture2D,false=Sampler)。
    is_image: bool,
    /// 纹理分量类型(image 用;sampler 占位)。
    sampled_prim: PrimTy,
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
        }
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
        let (res_type, is_image, sampled_prim) = match res.res {
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
                (id, true, prim)
            }
            MirResourceType::Sampler => {
                let id = self.alloc_id();
                Self::emit(&mut self.types, OP_TYPE_SAMPLER, &[id]);
                (id, false, PrimTy::F32)
            }
            other => {
                return Err(DxilError::unmappable(
                    "resource-type",
                    format!(
                        "资源 `{}` 类型 {other:?} 不在 B 路编码器资源最小子集\
                         (Texture2D<F>/Sampler)内(CBV/structured buffer SPIR-V 降级为后续扩展)",
                        res.name
                    ),
                ));
            }
        };

        let ptr = self.pointer_type(STORAGE_UNIFORM_CONSTANT, res_type);
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

        // 登记资源变量(RXS-0175;采样 body lowering 按名匹配 MIR local 解析)。
        self.resource_vars.push(ResourceVarInfo {
            name: res.name.clone(),
            var_id: var,
            type_id: res_type,
            is_image,
            sampled_prim,
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
                coord,
            } => Ok(LocalValue::Value(self.lower_resource_sample(
                b,
                texture_local.0,
                sampler_local.0,
                coord,
            )?)),
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

    /// 纹理采样 lowering(RXS-0175;RFC-0007 §4.5):`OpLoad` 纹理/采样器 +
    /// `OpSampledImage` + `OpImageSampleExplicitLod`(显式 LOD 0,规避隐式导数)→
    /// `vec4<F>`。首期收敛子集外(coord 非 `vec2<f32>` / 非 `Texture2D<f32>` /
    /// sampler 实参非 `Sampler`)→ [`DxilError::SampleUnsupported`](RX6023)。
    fn lower_resource_sample(
        &mut self,
        b: &mut Builder,
        texture_local: u32,
        sampler_local: u32,
        coord: &Operand,
    ) -> Result<SpirvValue, DxilError> {
        // coord 须为 vec2<f32>(归一化 UV;首期子集,RXS-0175)。
        let coord_val = self.lower_operand_value(b, coord, None)?;
        if coord_val.ty != MirIoType::Vector(PrimTy::F32, 2) {
            return Err(DxilError::sample_unsupported(format!(
                "采样坐标类型 {:?} 非 vec2<f32>(首期收敛子集)",
                coord_val.ty
            )));
        }

        let tex = self.resource_for_local(texture_local)?;
        let samp = self.resource_for_local(sampler_local)?;
        if !tex.is_image {
            return Err(DxilError::sample_unsupported(format!(
                "采样 receiver `{}` 非 Texture2D 纹理句柄",
                tex.name
            )));
        }
        if samp.is_image {
            return Err(DxilError::sample_unsupported(format!(
                "采样 sampler 实参 `{}` 非 Sampler 采样器句柄",
                samp.name
            )));
        }
        if tex.sampled_prim != PrimTy::F32 {
            return Err(DxilError::sample_unsupported(format!(
                "首期仅支持 Texture2D<f32>(实得分量类型 {:?})",
                tex.sampled_prim
            )));
        }

        // OpLoad 纹理 / 采样器对象(UniformConstant opaque 资源,SPIR-V 合法)。
        let img_id = b.alloc_id();
        Builder::emit(&mut self.ops, OP_LOAD, &[tex.type_id, img_id, tex.var_id]);
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

        // 显式 LOD 0 常量(规避隐式导数,RFC-0007 §4.6)。
        let f32_ty = b.scalar_type(PrimTy::F32)?;
        let lod0 = b.alloc_id();
        Builder::emit(&mut b.types, OP_CONSTANT, &[f32_ty, lod0, 0.0f32.to_bits()]);

        // OpImageSampleExplicitLod:结果 vec4<f32>,ImageOperands = Lod。
        let result_mir = MirIoType::Vector(PrimTy::F32, 4);
        let result_ty = b.value_type(result_mir)?;
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
                lod0,
            ],
        );
        Ok(SpirvValue {
            id: result_id,
            ty: result_mir,
        })
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

/// Vulkan 原生消费入口(RXS-0210):与 [`emit_spirv_body`] 同降级,但 **不 emit**
/// provenance 装饰(`UserSemantic` → `SPV_GOOGLE_hlsl_functionality1`)。保名仅 B 路
/// SPIRV-Cross→HLSL→dxc 边界需要(Vulkan 原生按 `Location`/`BuiltIn` 消费,永不需要);
/// 去装饰后 `.spv` 对所有 Vulkan ICD(NVIDIA/AMD/Android/lavapipe)零扩展依赖直喂
/// `vkCreateShaderModule`(免 device 扩展 `VK_GOOGLE_hlsl_functionality1`,VUID-...-08742)。
/// DXIL 路(`emit_spirv_body`,provenance=true)保名字节不变、零回归。
pub fn emit_spirv_body_vulkan(stage: ShaderStage, body: &Body) -> Result<Vec<u32>, DxilError> {
    emit_spirv_inner(
        stage,
        &body.io_sig,
        &body.resources,
        Some(body),
        /*provenance=*/ false,
    )
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
    let spirv_bindings =
        binding_layout::infer_spirv_bindings(resources).map_err(map_binding_err)?;
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

    // 2) capability。
    Builder::emit(&mut module, OP_CAPABILITY, &[CAP_SHADER]);

    // 3) extension(仅当用到 UserSemantic 保名)。
    if b.used_user_semantic {
        let mut operands = Vec::new();
        Builder::push_string(&mut operands, EXT_HLSL_FUNCTIONALITY1);
        Builder::emit(&mut module, OP_EXTENSION, &operands);
    }

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

    /// strict-only:bindless / unbounded 资源 → 透传 [`DxilError::Unmappable`]
    /// (RD-018 defer,不发明 descriptor heap 编码)。
    #[test]
    fn unbounded_resource_is_unmappable() {
        use crate::mir::ResourceCount;
        let resources = vec![ResourceBinding {
            name: "heap".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::Unbounded,
        }];
        let r = emit_spirv(ShaderStage::Fragment, &[], &resources);
        assert!(
            matches!(r, Err(DxilError::Unmappable { .. })),
            "unbounded 资源应不可映射(RD-018),实得 {r:?}"
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
        let m = emit_spirv_body_vulkan(ShaderStage::Fragment, &body)
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
}
