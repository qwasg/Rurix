//! `reflection` — M31 shader reflection v1 与 interface hash(G8.2 硬门
//! `g8.p0.m31.reflection_hash`;RFC-0019 §4.4;spec/rendering_platform.md
//! RXS-0304~0307)。
//!
//! 本模块把编译单元的着色入口接口事实(reflection v1 字段闭集)落成
//! **canonical bytes** 与域分离 SHA-256 digest:
//!
//! - `interface_hash = SHA-256("rurix.shader-interface.v1\0" || canonical_interface_bytes)`
//!   (RFC-0019 §4.4 逐字;接口事实,**不含**函数体);
//! - `source_digest = SHA-256("rurix.shader-source.v1\0" || …)`(含函数体源文本段,
//!   承担「同接口不同内容」的编译期区分证据;后端 artifact digest 归属
//!   RXS-0290/0291 artifacts v2 发射面,不在本模块重复定义);
//! - `pipeline_key` = DDC/PSO/RT pipeline key 组成项的编译期见证(RXS-0306)。
//!
//! 数据源纪律(单一事实源):
//!
//! - I/O 意图签名 / 资源句柄提取 = [`crate::iface_extract`](自 `mir_build::dxil_io`
//!   机械搬迁,与 device MIR 附着同一提取律);
//! - vertex/fragment 绑定 = [`crate::binding_layout::infer_spirv_bindings_vk_native`]
//!   (Vk-native 分配律,RXS-0230 E-3,与 `dxil_spirv` Vulkan 原生路同一函数);
//! - compute 族(含 mesh,RXS-0275)形参分类镜像 `vulkan_codegen::classify_param`
//!   的输入面闭集(`ThreadCtx`/buffer/storage image/`AccelStruct`/标量);
//! - `AccelStruct` 判定 = [`crate::shader_stages::is_accel_struct`](RXS-0245/0297)。
//!
//! 诚实边界(RXS-0304 冻结):RT 阶段函数 v1 不可枚举(枚举接线归 M50);泛型着色
//! 函数不产 entry;同名 entry 跨 `mod` → 推导确定性失败(fail-closed);判定以
//! AST 类型头名匹配为准(承 RXS-0156/0245 先例)。canonical bytes 不含绝对路径、
//! 文件名、mtime、进程 ID、随机 seed、backend handle、driver query 值
//! (RXS-0305 禁用面)。

use crate::ast::{self, FnColor, ShaderStage};
use crate::hir::PrimTy;
use crate::iface_extract as iface;
use crate::mir::{IoDir, IoSigKind, MirIoType, MirResourceType, ResourceCount};
use crate::span::{SourceId, Span};
use rurix_pkg::sha256;

/// reflection 产物 schema 标识(RXS-0304 文档级闭集字段 `schema`)。
pub const SCHEMA_ID: &str = "rurix.shader-reflection.v1";
/// reflection schema 版本(字段闭集演进须升版)。
pub const SCHEMA_VERSION: u32 = 1;
/// canonical 反射目标(RXS-0304 目标纪律:Vk-native 分配律,生产运行时通道)。
pub const TARGET: &str = "vulkan";
/// canonical 后端形态。
pub const BACKEND: &str = "spirv";
/// MVP 期唯一 edition(`crate::span::Edition::Rx0`)。
pub const EDITION_RX0: &str = "Rx0";
/// 编译器标识。
pub const COMPILER: &str = "rurixc";

const IFACE_DOMAIN: &[u8] = b"rurix.shader-interface.v1\0";
const SOURCE_DOMAIN: &[u8] = b"rurix.shader-source.v1\0";
const UNIT_DOMAIN: &[u8] = b"rurix.shader-unit.v1\0";
const KEY_DOMAIN: &[u8] = b"rurix.pipeline-key.v1\0";
/// 「未选择 profile」的规范 digest 定义域(M32 未实现的确定性空编码,RXS-0304)。
const PROFILE_NONE_DOMAIN: &[u8] = b"rurix.profile-none.v1\0";
/// 空 permutation 域的规范 digest 定义域(M29 未实现的确定性空编码,RXS-0304)。
const PERM_EMPTY_DOMAIN: &[u8] = b"rurix.permutation-domain-empty.v1\0";

/// `location` 的空值编码(RXS-0305:`0xFFFF_FFFF` 哨兵)。
const LOCATION_NONE: u32 = u32::MAX;

/// 反射推导失败(strict-only,fail-closed;RXS-0304/0307)。
///
/// 错误码复用既有类别(零新码):`Unmappable`(绑定布局不可映射,如无界非 SRV
/// 纹理表)→ RX6013 `codegen.dxil_unmappable`(与 Vk-native 图形编码路同一裁决);
/// `Unsupported`(形参/构造超出 canonical target 已建模闭集)→ RX6026
/// `codegen.vulkan_unsupported`(与 `vulkan_codegen::classify_param` 同一口径)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReflectError {
    /// 绑定推导不可映射(承接 `binding_layout::BindingInferError::Unmappable`)。
    Unmappable {
        /// 诊断上下文(资源名/类别事实;不含布局猜测值)。
        detail: String,
    },
    /// 超出已建模闭集(compute 形参分类失败 / mesh_meta 缺失 / 同名 entry 冲突 /
    /// 跨文件 entry 源切片不可达)。
    Unsupported {
        /// 诊断上下文。
        detail: String,
    },
    /// permutation 域声明违例 / `--permutation-select` KEY 不在合法集(M29,
    /// RXS-0308/0310)→ RX3019 `shader.permutation_domain_invalid`(禁最接近回退)。
    Permutation {
        /// 诊断上下文。
        detail: String,
    },
    /// permutation 求解预算超限(M29,RXS-0310)→ RX7023
    /// `toolchain.permutation_budget_exceeded`。
    PermutationBudget {
        /// 诊断上下文。
        detail: String,
    },
}

impl ReflectError {
    /// 映射到诊断码(前两变体复用既有类别;M29 两变体 = RXS-0308/0310 配套新码)。
    pub fn error_code(&self) -> u16 {
        match self {
            ReflectError::Unmappable { .. } => 6013,
            ReflectError::Unsupported { .. } => 6026,
            ReflectError::Permutation { .. } => 3019,
            ReflectError::PermutationBudget { .. } => 7023,
        }
    }

    /// 诊断上下文文本。
    pub fn detail(&self) -> &str {
        match self {
            ReflectError::Unmappable { detail }
            | ReflectError::Unsupported { detail }
            | ReflectError::Permutation { detail }
            | ReflectError::PermutationBudget { detail } => detail,
        }
    }
}

impl std::fmt::Display for ReflectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReflectError::Unmappable { detail } => {
                write!(f, "reflection binding unmappable: {detail}")
            }
            ReflectError::Unsupported { detail } => write!(f, "reflection unsupported: {detail}"),
            ReflectError::Permutation { detail } => {
                write!(f, "permutation domain invalid: {detail}")
            }
            ReflectError::PermutationBudget { detail } => {
                write!(f, "permutation budget exceeded: {detail}")
            }
        }
    }
}

impl std::error::Error for ReflectError {}

/// stage I/O 元素(RXS-0304 `io` 字段;声明序即字段序)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IoField {
    /// 源字段名。
    pub name: String,
    /// 方向:`"in"` / `"out"`。
    pub dir: &'static str,
    /// 种类:`"builtin"` / `"interpolate"` / `"varying"`。
    pub kind: &'static str,
    /// builtin 名或插值限定名;`"varying"` 时为空串。
    pub annotation: String,
    /// 已建模类型渲染(标量 prim 名 / `vecN<T>`;RXS-0304 类型渲染闭集)。
    pub ty: String,
    /// 非 builtin 元素按方向各自自 0 递增分配;builtin 元素为 `None`。
    pub location: Option<u32>,
}

/// 资源绑定元素(RXS-0304 `resources` 字段;声明序)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResourceEntry {
    /// 源码形参名(保名依据)。
    pub name: String,
    /// 资源类:`"cbv"` / `"srv"` / `"uav"` / `"sampler"` / `"accel"`。
    pub class: &'static str,
    /// descriptor set(分配律见 RXS-0304 绑定推导律)。
    pub set: u32,
    /// binding 号。
    pub binding: u32,
    /// 基数:`1` 或有界 `n`;无界 SRV 纹理表 = `0` 哨兵(RXS-0305)。
    pub count: u32,
    /// 访问:`"read_only"` / `"read_write"` / `"sample"` / `"sample_cmp"` / `"accel"`。
    pub access: &'static str,
    /// 纹理分量 / buffer 元素 prim 名;无元素类型的资源为空串。
    pub format: String,
    /// 可见性位掩码(= 声明它的 entry 的阶段单 bit)。
    pub visibility: u32,
}

/// push-constant 块成员(compute 族标量形参;布局律与 `vulkan_codegen` 同一律)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PushConstMember {
    /// 源码形参名。
    pub name: String,
    /// 标量 prim 名。
    pub ty: &'static str,
    /// 成员序号(声明序自 0)。
    pub member: u32,
    /// 字节偏移(对齐累计)。
    pub offset: u32,
    /// 字节大小。
    pub size: u32,
}

/// entry 的 push-constant 块(无标量形参 = 空表 + `size_bytes 0`)。
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct PushConstants {
    /// 成员表(声明序)。
    pub members: Vec<PushConstMember>,
    /// 块总字节数(末成员 `offset + size`)。
    pub size_bytes: u32,
}

/// mesh 入口的源衍生执行模式(`#[numthreads]`/`#[outputs]`,RXS-0243);其余阶段恒
/// `None` 字段(空编码)。
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct ExecutionModes {
    /// `#[numthreads(x, y, z)]`(声明序)。
    pub numthreads: Option<(u32, u32, u32)>,
    /// `#[outputs(max_vertices = N)]`。
    pub max_vertices: Option<u32>,
    /// `#[outputs(max_primitives = M)]`。
    pub max_primitives: Option<u32>,
}

/// 单个 entry 的 reflection v1 记录(RXS-0304 字段闭集)与其派生 digest。
#[derive(Clone, Debug)]
pub struct EntryReflection {
    /// entry identity(源级名称路径,`::` 连接;不含文件名/路径/mangle 符号)。
    pub name: String,
    /// 阶段名闭集:`"vertex"` / `"fragment"` / `"compute"` / `"mesh"`。
    pub stage: &'static str,
    /// `ShaderStage` 枚举声明序 tag(RXS-0290 单一事实源)。
    pub stage_tag: u32,
    /// 阶段可见性位掩码(`1 << stage_tag`)。
    pub stage_visibility: u32,
    /// stage I/O 表(声明序)。
    pub io: Vec<IoField>,
    /// 资源绑定表(声明序)。
    pub resources: Vec<ResourceEntry>,
    /// push-constant 块。
    pub push_constants: PushConstants,
    /// mesh 执行模式(其余阶段空编码)。
    pub execution_modes: ExecutionModes,
    /// entry 有效 requirement 集的排序 ID 表(M32 真值化,RXS-0304 v1.2 /
    /// RXS-0311;无任何 requirement 或未计算恒空表)。
    pub required_capabilities: Vec<String>,
    /// 「未选择 profile」规范 digest 或 `--profile` 真值化 digest(M32,RXS-0312)。
    pub selected_profile_digest: [u8; 32],
    /// 空 permutation 域规范 digest(M29 空编码)。
    pub permutation_domain_digest: [u8; 32],
    /// 本 variant key(M29 空编码 = 空串)。
    pub variant_key: String,
    /// canonical 接口字节(RXS-0305;不含函数体)。
    pub canonical: Vec<u8>,
    /// `SHA-256("rurix.shader-interface.v1\0" || canonical)`(RFC-0019 §4.4 逐字)。
    pub interface_hash: [u8; 32],
    /// 含函数体源文本段的内容 digest(RXS-0306 分离规则)。
    pub source_digest: [u8; 32],
    /// 下游 DDC/PSO/RT pipeline key 组成见证(RXS-0306)。
    pub pipeline_key: [u8; 32],
    /// **RXS-0347 尾随可选字段(G9.2 M103,RFC-0023 §4.3)**:「资源 → 全局 descriptor
    /// 索引」映射记录(资源名按**声明序**,与 `resources` 一一对应;值为全局索引)。
    /// `None` = 缺省——canonical 序列化字节 ≡ 字段不存在(0-drift,既有产物 0-byte;
    /// **不得以「空编码为 count 0」冒充**);`Some` = 真值化(经
    /// [`ReflectionDescPlan::global_index_table`] 供给,rurix-rt `descriptor_table`
    /// 分配律单一事实源)。**与 set/binding 对并存不删**(保 M31/M85 digest 链)。
    pub global_descriptor_indices: Option<Vec<u32>>,
}

/// 编译单元级 reflection v1 文档。
#[derive(Clone, Debug)]
pub struct ReflectionDoc {
    /// rurixc 包版本(workspace 版本字串;与路径/时间戳无关)。
    pub compiler_version: String,
    /// entry 记录(规范键 `(name, stage_tag)` 排序)。
    pub entries: Vec<EntryReflection>,
    /// 文档级 canonical bytes(版本前缀起始;RXS-0305)。
    pub canonical: Vec<u8>,
    /// 单元 digest:`SHA-256("rurix.shader-unit.v1\0" || doc_canonical)`。
    pub unit_digest: [u8; 32],
}

// ═══════════════════════ canonical 序列化写入器(RXS-0305) ═══════════════════════

/// canonical bytes 写入器:u32 一律小端;字符串 = u32 长度前缀 + UTF-8 字节;
/// 列表 = u32 计数 + 元素顺序排列。
struct CanonW {
    buf: Vec<u8>,
}

impl CanonW {
    fn new() -> Self {
        CanonW { buf: Vec::new() }
    }
    fn u32v(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn strv(&mut self, s: &str) {
        self.u32v(u32::try_from(s.len()).unwrap_or(u32::MAX));
        self.buf.extend_from_slice(s.as_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }
    fn opt_u32(&mut self, v: Option<u32>) {
        self.u32v(v.unwrap_or(LOCATION_NONE));
    }
}

/// prim → 规范名(RXS-0304 类型渲染闭集)。
fn prim_name(p: PrimTy) -> &'static str {
    match p {
        PrimTy::I8 => "i8",
        PrimTy::I16 => "i16",
        PrimTy::I32 => "i32",
        PrimTy::I64 => "i64",
        PrimTy::U8 => "u8",
        PrimTy::U16 => "u16",
        PrimTy::U32 => "u32",
        PrimTy::U64 => "u64",
        PrimTy::Usize => "usize",
        PrimTy::F32 => "f32",
        PrimTy::F64 => "f64",
        PrimTy::Bool => "bool",
        PrimTy::Char => "char",
        PrimTy::Str => "str",
    }
}

/// 已建模 I/O 类型渲染(标量 prim 名 / `vecN<T>`)。
fn io_type_name(ty: MirIoType) -> String {
    match ty {
        MirIoType::Scalar(p) => prim_name(p).to_owned(),
        MirIoType::Vector(p, n) => format!("vec{}<{}>", n, prim_name(p)),
    }
}

/// 阶段名(RXS-0304 阶段名闭集;v1 枚举面仅前四个,余者列出仅供完备匹配)。
fn stage_name(stage: ShaderStage) -> &'static str {
    match stage {
        ShaderStage::Vertex => "vertex",
        ShaderStage::Fragment => "fragment",
        ShaderStage::Compute => "compute",
        ShaderStage::Mesh => "mesh",
        ShaderStage::Task => "task",
        ShaderStage::RayGen => "raygen",
        ShaderStage::ClosestHit => "closesthit",
        ShaderStage::AnyHit => "anyhit",
        ShaderStage::Miss => "miss",
        ShaderStage::Intersection => "intersection",
        ShaderStage::Callable => "callable",
    }
}

/// push-constant 标量布局律(与 `vulkan_codegen::prim_layout` 同一律:
/// `i64/u64` → `(align=8,size=8)`,其余标量 → `(4,4)`)。
fn push_layout(p: PrimTy) -> (u32, u32) {
    if matches!(p, PrimTy::I64 | PrimTy::U64) {
        (8, 8)
    } else {
        (4, 4)
    }
}

fn align_up(value: u32, align: u32) -> u32 {
    value.div_ceil(align) * align
}

// ═══════════════════════ entry 枚举与提取(RXS-0304) ═══════════════════════

/// v1 可枚举阶段判定(诚实边界:RT 阶段与 task 不可枚举,归 M50/条件臂)。
fn enumerable_stage(stage: Option<ShaderStage>, color: FnColor) -> Option<ShaderStage> {
    match stage {
        None if color == FnColor::Kernel => Some(ShaderStage::Compute),
        Some(
            s @ (ShaderStage::Vertex
            | ShaderStage::Fragment
            | ShaderStage::Compute
            | ShaderStage::Mesh),
        ) => Some(s),
        _ => None,
    }
}

/// 递归枚举 entry(含嵌套 `mod`,路径以 `::` 连接);泛型着色函数不产 entry
/// (与 `mir_build` device 根收集口径一致)。保持 AST 声明序返回(文档级排序在
/// canonical 组装时按规范键完成)。M29 起同车携带 item 属性表(permutation 域
/// 声明面,RXS-0308)。
fn collect_entries<'a>(
    items: &'a [ast::Item],
    prefix: &str,
    out: &mut Vec<(String, &'a ast::FnItem, &'a [ast::Attr], Span)>,
) {
    for it in items {
        match &it.kind {
            ast::ItemKind::Fn(f) => {
                if !f.generics.params.is_empty() {
                    continue;
                }
                if enumerable_stage(f.stage, f.color).is_some() {
                    out.push((
                        format!("{prefix}{}", f.name.name),
                        f,
                        it.attrs.as_slice(),
                        it.span,
                    ));
                }
            }
            ast::ItemKind::Mod(m) => {
                collect_entries(&m.items, &format!("{prefix}{}::", m.name.name), out);
            }
            _ => {}
        }
    }
}

/// I/O 意图签名 → reflection `io` 表(location 分配律:非 builtin 按方向各自
/// 自 0 递增,builtin 不占 location——与 `dxil_spirv::emit_io_elem` 同一律)。
fn io_fields(elems: Vec<crate::mir::IoSigElem>) -> Vec<IoField> {
    let mut next_in = 0u32;
    let mut next_out = 0u32;
    elems
        .into_iter()
        .map(|e| {
            let (kind, annotation, is_builtin) = match &e.kind {
                IoSigKind::Builtin(n) => ("builtin", n.clone(), true),
                IoSigKind::Interpolate(m) => ("interpolate", m.clone(), false),
                IoSigKind::Varying => ("varying", String::new(), false),
            };
            let location = if is_builtin {
                None
            } else {
                let slot = match e.dir {
                    IoDir::In => &mut next_in,
                    IoDir::Out => &mut next_out,
                };
                let n = *slot;
                *slot += 1;
                Some(n)
            };
            IoField {
                name: e.field_name,
                dir: match e.dir {
                    IoDir::In => "in",
                    IoDir::Out => "out",
                },
                kind,
                annotation,
                ty: io_type_name(e.ty),
                location,
            }
        })
        .collect()
}

/// 资源类型 → (class, access, format)(RXS-0304 闭集映射)。
fn resource_faces(res: &MirResourceType) -> (&'static str, &'static str, String) {
    match res {
        MirResourceType::Texture2D(p) => ("srv", "read_only", prim_name(*p).to_owned()),
        MirResourceType::TextureRw2D(p) => ("uav", "read_write", prim_name(*p).to_owned()),
        MirResourceType::Sampler => ("sampler", "sample", String::new()),
        MirResourceType::SamplerCmp => ("sampler", "sample_cmp", String::new()),
        MirResourceType::ConstantBuffer => ("cbv", "read_only", String::new()),
        MirResourceType::StructuredBuffer { read_only } => {
            if *read_only {
                ("srv", "read_only", String::new())
            } else {
                ("uav", "read_write", String::new())
            }
        }
    }
}

/// 基数编码(`One` → 1;`Bounded(n)` → n;`Unbounded` → 0 哨兵)。
fn count_code(count: ResourceCount) -> u32 {
    match count {
        ResourceCount::One => 1,
        ResourceCount::Bounded(n) => n,
        ResourceCount::Unbounded => 0,
    }
}

/// compute 族形参分类(镜像 `vulkan_codegen::classify_param` 的输入面闭集;
/// AST 头名判定承 RXS-0156/0245 先例)。
enum ComputeParam {
    /// `ThreadCtx`(ZST 执行上下文形参;不占 binding / push-constant / 任何 ABI 字段)。
    ThreadCtx,
    /// `AccelStruct`(compute 签名,RXS-0297)→ accel 资源。
    Accel,
    /// `View`/`ViewMut`/`Atomic`/`AtomicView` → buffer 资源。
    Buffer {
        /// 资源类与访问面。
        class: &'static str,
        /// 访问面。
        access: &'static str,
        /// 元素 prim 名。
        elem: &'static str,
    },
    /// `TextureRw2D<F>` → storage image 资源。
    Image {
        /// 分量 prim 名。
        elem: &'static str,
    },
    /// 标量形参 → push-constant 成员。
    Scalar(PrimTy),
}

/// 取路径末段第 `n` 个类型实参的 prim 名(`View<global, f32>` 的 `f32` = 下标 1)。
fn nth_type_arg_prim(ty: &ast::Ty, n: usize) -> Option<PrimTy> {
    let ast::TyKind::Path(p) = &ty.kind else {
        return None;
    };
    let seg = p.segments.last()?;
    let args = seg.args.as_ref()?;
    args.args
        .iter()
        .filter_map(|a| match a {
            ast::GenericArg::Type(t) => Some(t),
            _ => None,
        })
        .nth(n)
        .and_then(|t| iface::ty_head_name(t))
        .and_then(PrimTy::from_name)
}

/// compute 族单形参分类(AST 层;失败 = `Unsupported`,与 classify_param 同口径)。
fn classify_compute_param(ty: &ast::Ty) -> Result<ComputeParam, ReflectError> {
    let ty = iface::unwrap_ty(ty);
    // `AccelStruct` 判定 = shader_stages 单一事实源(RXS-0245/0297)。
    if crate::shader_stages::is_accel_struct(ty) {
        return Ok(ComputeParam::Accel);
    }
    let Some(head) = iface::ty_head_name(ty) else {
        return Err(ReflectError::Unsupported {
            detail: "compute 族入口形参类型不可建模(非路径类型;RXS-0304 闭集)".to_owned(),
        });
    };
    let buffer = |class, access, idx: usize, head: &str| {
        nth_type_arg_prim(ty, idx)
            .map(|elem| ComputeParam::Buffer { class, access, elem: prim_name(elem) })
            .ok_or_else(|| ReflectError::Unsupported {
                detail: format!(
                    "`{head}` 形参元素类型不可建模(元素须为标量 prim;与 vulkan compute 分类同一口径)"
                ),
            })
    };
    match head {
        "ThreadCtx" => Ok(ComputeParam::ThreadCtx),
        "View" => buffer("srv", "read_only", 1, head),
        "ViewMut" | "AtomicView" => buffer("uav", "read_write", 1, head),
        "Atomic" => buffer("uav", "read_write", 0, head),
        "TextureRw2D" => {
            // 镜像 classify_param:分量缺省 f32,仅 f32/i32/u32 合法。
            let elem = nth_type_arg_prim(ty, 0).unwrap_or(PrimTy::F32);
            if !matches!(elem, PrimTy::F32 | PrimTy::I32 | PrimTy::U32) {
                return Err(ReflectError::Unsupported {
                    detail: "TextureRw2D storage image 仅支持 f32/i32/u32 分量(与 vulkan compute 分类同一口径)"
                        .to_owned(),
                });
            }
            Ok(ComputeParam::Image {
                elem: prim_name(elem),
            })
        }
        _ => match PrimTy::from_name(head) {
            Some(p) => Ok(ComputeParam::Scalar(p)),
            None => Err(ReflectError::Unsupported {
                detail: format!(
                    "compute 族入口形参 `{head}` 超出已建模闭集(View/ViewMut/Atomic/AtomicView、TextureRw2D、AccelStruct、标量、ThreadCtx;RXS-0304)"
                ),
            }),
        },
    }
}

/// push-constant 成员追加(声明序;对齐累计布局)。
fn push_member(pc: &mut PushConstants, name: String, prim: PrimTy) {
    let (align, size) = push_layout(prim);
    let offset = align_up(pc.size_bytes, align);
    let member = pc.members.len() as u32;
    pc.members.push(PushConstMember {
        name,
        ty: prim_name(prim),
        member,
        offset,
        size,
    });
    pc.size_bytes = offset + size;
}

/// 图形阶段(vertex/fragment)资源 + push-constant 提取。
fn graphics_resources(
    file: &ast::SourceFile,
    bare: &str,
    stage: ShaderStage,
    visibility: u32,
) -> Result<Vec<ResourceEntry>, ReflectError> {
    let res = iface::resources_for(file, bare, stage);
    // 绑定推导 = binding_layout 单一事实源(Vk-native 分配律,RXS-0230 E-3)。
    let bindings = crate::binding_layout::infer_spirv_bindings_vk_native(&res).map_err(|e| {
        ReflectError::Unmappable {
            detail: format!("资源绑定推导不可映射:{e}"),
        }
    })?;
    Ok(res
        .iter()
        .zip(bindings.iter())
        .map(|(r, b)| {
            let (class, access, format) = resource_faces(&r.res);
            ResourceEntry {
                name: r.name.clone(),
                class,
                set: b.set,
                binding: b.binding,
                count: count_code(r.count),
                access,
                format,
                visibility,
            }
        })
        .collect())
}

/// `--descriptor-table-plan` 的 reflection 输入面(RXS-0347;M103)。**尾随可选**:
/// 缺省(`global_index_table = None`)时序列化字节 ≡ 字段不存在(0-drift,既有
/// 产物 0-byte)。
#[derive(Clone, Default)]
pub struct ReflectionDescPlan {
    /// 全局 descriptor 索引映射(「资源名 → 全局索引」;rurix-rt
    /// `descriptor_table::GlobalDescriptorTable::mapping_snapshot` 的产物,或测试
    /// 直供)。None = 缺省(字段不发射,序列化 0-drift)。
    pub global_index_table: Option<Vec<(String, u32)>>,
    /// 索引空间预算(capability profile 事实,RXS-0347 §4);索引 ≥ 预算 = 装配期
    /// 确定性拒绝(fail-closed)。None = 不做预算门(缺省路径 0-drift)。
    pub index_budget: Option<u32>,
}


#[derive(Clone, Copy, Default)]
pub struct ReflectionPermPlan<'a> {
    /// `--permutation-select=KEY`(字符串形态;选中后 `variant_key = KEY`、
    /// 非空域 `permutation_domain_digest` 真值化)。
    pub select: Option<&'a str>,
    /// `--permutation-budget=N`(CLI 覆盖 attr 声明值;select 求解路径生效)。
    pub budget_override: Option<u32>,
}

/// `--profile` / capability 选择律的 reflection 输入面(M32,RXS-0304 v1.2 /
/// RXS-0311 / RXS-0312)。默认(无 profile + 无 capability 上下文)下空路径与
/// M31/M29 基线产物**逐字节 0 漂移**。
#[derive(Clone, Copy, Default)]
pub struct ReflectionCapPlan<'a> {
    /// 编译单元调用图并集 capability 事实(RXS-0311);`required_capabilities`
    /// 真值化源。None = 未计算(恒空表)。
    pub unit_caps: Option<&'a crate::capability_check::UnitCapabilities>,
    /// `--profile` 的规范 digest(RXS-0312);None = 恒既有常量
    /// `SHA-256("rurix.profile-none.v1\0")`(0 漂移)。
    pub profile_digest: Option<[u8; 32]>,
    /// 构建期选择律结果(RXS-0312);fallback 选中的逻辑 entry 记录取 fallback
    /// 实体(主 variant 不发射——其记录不进文档;逻辑名→实体映射在
    /// capabilities 报告中可查)。
    pub selection: Option<&'a crate::capability_check::SelectionOutcome>,
}

/// entry 的接口契约事实(RXS-0312 v1.3 fallback 兼容判定面;与 [`build_entry`]
/// 同一提取律——单一事实源,reflection 产物与本判定不各说各话)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct InterfaceFacts {
    /// 阶段名闭集(`stage_name`)。
    pub(crate) stage: &'static str,
    /// stage I/O 表(声明序)。
    pub(crate) io: Vec<IoField>,
    /// 资源绑定表(声明序)。
    pub(crate) resources: Vec<ResourceEntry>,
    /// push-constant 块。
    pub(crate) push_constants: PushConstants,
    /// mesh 执行模式(其余阶段空编码)。
    pub(crate) execution_modes: ExecutionModes,
}

/// 提取单 entry 的接口契约事实(与 `build_entry` 同一提取律;调用方须先按
/// [`enumerable_stage`] 口径确认可枚举)。
pub(crate) fn extract_interface_facts(
    file: &ast::SourceFile,
    src: &str,
    f: &ast::FnItem,
) -> Result<InterfaceFacts, ReflectError> {
    let stage = enumerable_stage(f.stage, f.color).expect("枚举口径已过滤");
    let bare = f.name.name.as_str();
    let stage_tag = crate::codegen::stage_tag(stage);
    let visibility = 1u32 << stage_tag;

    let mut io: Vec<IoField> = Vec::new();
    let mut resources: Vec<ResourceEntry> = Vec::new();
    let mut pc = PushConstants::default();
    let mut exec = ExecutionModes::default();

    match stage {
        ShaderStage::Vertex | ShaderStage::Fragment => {
            io = io_fields(iface::io_sig_for(file, bare, stage));
            resources = graphics_resources(file, bare, stage, visibility)?;
            // 图形阶段标量形参 → push-constant 块(布局律同一;图形侧资源面依据
            // RXS-0302)。资源句柄与命名 I/O 结构体形参不属于本闭集,不进入。
            for p in &f.params {
                if let ast::ParamKind::Typed { pat, ty } = &p.kind {
                    let ty = iface::unwrap_ty(ty);
                    if let Some(head) = iface::ty_head_name(ty)
                        && let Some(prim) = PrimTy::from_name(head)
                    {
                        push_member(
                            &mut pc,
                            iface::pat_binding_name(pat).unwrap_or_default(),
                            prim,
                        );
                    }
                }
            }
        }
        ShaderStage::Compute | ShaderStage::Mesh => {
            // compute 族(含 mesh,RXS-0275 镜像 lower_compute 形参分类):set=0,
            // binding 按资源形参声明序全局递增;标量形参进 push-constant 块。
            let mut next_binding = 0u32;
            for p in &f.params {
                let ast::ParamKind::Typed { pat, ty } = &p.kind else {
                    continue;
                };
                let name = iface::pat_binding_name(pat).unwrap_or_default();
                match classify_compute_param(ty)? {
                    ComputeParam::ThreadCtx => {}
                    ComputeParam::Accel => {
                        resources.push(ResourceEntry {
                            name,
                            class: "accel",
                            set: 0,
                            binding: next_binding,
                            count: 1,
                            access: "accel",
                            format: String::new(),
                            visibility,
                        });
                        next_binding += 1;
                    }
                    ComputeParam::Buffer {
                        class,
                        access,
                        elem,
                    } => {
                        resources.push(ResourceEntry {
                            name,
                            class,
                            set: 0,
                            binding: next_binding,
                            count: 1,
                            access,
                            format: elem.to_owned(),
                            visibility,
                        });
                        next_binding += 1;
                    }
                    ComputeParam::Image { elem } => {
                        resources.push(ResourceEntry {
                            name,
                            class: "uav",
                            set: 0,
                            binding: next_binding,
                            count: 1,
                            access: "read_write",
                            format: elem.to_owned(),
                            visibility,
                        });
                        next_binding += 1;
                    }
                    ComputeParam::Scalar(prim) => push_member(&mut pc, name, prim),
                }
            }
            if stage == ShaderStage::Mesh {
                // mesh 入口携带 io_sig(输出结构体面)+ 执行模式(RXS-0243/0275)。
                io = io_fields(iface::io_sig_for(file, bare, stage));
                let meta = iface::mesh_meta_for(file, src, bare).ok_or_else(|| {
                    ReflectError::Unsupported {
                        detail: format!(
                            "mesh 入口 `{bare}` 缺 mesh_meta(#[numthreads]/#[outputs];RXS-0243 预校验后保守缺失)"
                        ),
                    }
                })?;
                exec = ExecutionModes {
                    numthreads: Some(meta.numthreads),
                    max_vertices: Some(meta.max_vertices),
                    max_primitives: Some(meta.max_primitives),
                };
            }
        }
        _ => unreachable!("v1 枚举口径仅 vertex/fragment/compute/mesh"),
    }

    Ok(InterfaceFacts {
        stage: stage_name(stage),
        io,
        resources,
        push_constants: pc,
        execution_modes: exec,
    })
}

/// 构建单条 entry 记录。
#[allow(clippy::too_many_arguments)]
fn build_entry(
    file: &ast::SourceFile,
    src: &str,
    main_file: SourceId,
    name_path: &str,
    f: &ast::FnItem,
    attrs: &[ast::Attr],
    span: Span,
    perm_plan: &ReflectionPermPlan<'_>,
    cap_plan: &ReflectionCapPlan<'_>,
    desc_plan: &ReflectionDescPlan,
) -> Result<EntryReflection, ReflectError> {
    let stage = enumerable_stage(f.stage, f.color).expect("枚举口径已过滤");
    let stage_tag = crate::codegen::stage_tag(stage);
    let visibility = 1u32 << stage_tag;

    // 接口事实 = 单一提取律(RXS-0312 v1.3 兼容判定同面)。
    let InterfaceFacts {
        io,
        resources,
        push_constants: pc,
        execution_modes: exec,
        ..
    } = extract_interface_facts(file, src, f)?;

    // ── canonical 接口字节(RXS-0305;字段序即本闭集声明序)──
    let mut w = CanonW::new();
    w.u32v(SCHEMA_VERSION);
    w.strv(name_path);
    w.strv(stage_name(stage));
    w.u32v(stage_tag);
    w.u32v(visibility);
    w.u32v(io.len() as u32);
    for e in &io {
        w.strv(&e.name);
        w.strv(e.dir);
        w.strv(e.kind);
        w.strv(&e.annotation);
        w.strv(&e.ty);
        w.opt_u32(e.location);
    }
    w.u32v(resources.len() as u32);
    for r in &resources {
        w.strv(&r.name);
        w.strv(r.class);
        w.u32v(r.set);
        w.u32v(r.binding);
        w.u32v(r.count);
        w.strv(r.access);
        w.strv(&r.format);
        w.u32v(r.visibility);
    }
    w.u32v(pc.members.len() as u32);
    for m in &pc.members {
        w.strv(&m.name);
        w.strv(m.ty);
        w.u32v(m.member);
        w.u32v(m.offset);
        w.u32v(m.size);
    }
    w.u32v(pc.size_bytes);
    // execution_modes:0(非 mesh)或 5 个 u32(numthreads 三元 + max_vertices +
    // max_primitives)。
    match (exec.numthreads, exec.max_vertices, exec.max_primitives) {
        (Some(nt), Some(mv), Some(mp)) => {
            w.u32v(5);
            w.u32v(nt.0);
            w.u32v(nt.1);
            w.u32v(nt.2);
            w.u32v(mv);
            w.u32v(mp);
        }
        _ => w.u32v(0),
    }
    // M50 保留位的确定性空编码(7 个 RT/库字段恒计数 0)。
    for _ in 0..7 {
        w.u32v(0);
    }
    // M32 真值化(RXS-0304 v1.2 / RXS-0311):`required_capabilities` = entry 有效
    // requirement 集的排序 ID 表(字节序);无 requirement 或未计算恒空表——
    // 计数 0 编码与既有空编码逐字节一致(0 漂移)。
    let required_capabilities: Vec<String> = cap_plan
        .unit_caps
        .and_then(|u| u.entries.iter().find(|e| e.name == name_path))
        .map(|e| e.effective.iter().map(|c| c.name().to_owned()).collect())
        .unwrap_or_default();
    w.u32v(required_capabilities.len() as u32);
    for c in &required_capabilities {
        w.strv(c);
    }
    // M32 真值化(RXS-0304 v1.2 / RXS-0312):`--profile` 给定时 =
    // 该 profile 规范 digest;未给定恒既有常量(0 漂移)。
    let profile_digest = cap_plan
        .profile_digest
        .unwrap_or_else(|| sha256::digest(PROFILE_NONE_DOMAIN));
    // M29 真值化(RXS-0304 v1.1 / RXS-0309/0310):entry 声明了非空 permutation
    // 域 → 真 domain digest;`--permutation-select` 选中 → `variant_key = KEY`
    // (KEY ∉ 合法集 = RX3019 类确定性错误,禁最接近回退)。无 `#[permutation]`
    // 标注(空域)→ 既有常量 + 空串,与 M31 产物逐字节 0 漂移。
    let (perm_digest, variant_key) = match crate::permutation::extract_domain(attrs, src) {        Ok(Some(domain)) => {
            let digest = domain.digest();
            let variant = match perm_plan.select {
                Some(key) => domain
                    .validate_select_key(key, perm_plan.budget_override)
                    .map_err(|e| match e {
                        crate::permutation::SelectError::InvalidKey { detail } => {
                            ReflectError::Permutation { detail }
                        }
                        crate::permutation::SelectError::Budget(b) => {
                            ReflectError::PermutationBudget {
                                detail: format!(
                                    "entry `{name_path}` permutation 预算超限:enumerated {} > budget {}(RXS-0310)",
                                    b.enumerated, b.budget
                                ),
                            }
                        }
                    })?,
                None => String::new(),
            };
            (digest, variant)
        }
        Ok(None) => (sha256::digest(PERM_EMPTY_DOMAIN), String::new()),
        Err(inv) => {
            return Err(ReflectError::Permutation {
                detail: format!("entry `{name_path}`: {}", inv.detail),
            });
        }
    };
    w.bytes(&profile_digest);
    w.bytes(&perm_digest);
    w.strv(&variant_key);
    // RXS-0347 尾随可选字段(0-drift;缺省 = 字段不存在,既有字节 0-byte)——
    // 真值化时按资源声明序逐资源编码「全局 descriptor 索引」(u32 LE);**不得以
    // count 0 冒充 0-byte**(本段在 `Some` 时才写,与既有字节流严格加性尾随)。
    let gdi: Option<Vec<u32>> = match desc_plan.global_index_table.as_ref() {
        Some(map) => {
            let mut v = Vec::with_capacity(resources.len());
            for r in &resources {
                let idx = map
                    .iter()
                    .find(|(n, _)| n == &r.name)
                    .map(|(_, i)| *i)
                    .ok_or_else(|| ReflectError::Unsupported {
                        detail: format!(
                            "资源 `{}` 缺全局 descriptor 索引映射(悬空;fail-closed,RXS-0347 §3)",
                            r.name
                        ),
                    })?;
                if let Some(budget) = desc_plan.index_budget
                    && idx >= budget
                {
                    return Err(ReflectError::Unsupported {
                        detail: format!(
                            "资源 `{}` 全局索引 {idx} ≥ 索引空间预算 {budget}(capability profile 事实;装配期确定性拒绝,RXS-0347 §4)",
                            r.name
                        ),
                    });
                }
                w.u32v(idx);
                v.push(idx);
            }
            Some(v)
        }
        None => None,
    };
    let canonical = w.buf;

    let mut h = sha256::Sha256::new();
    h.update(IFACE_DOMAIN);
    h.update(&canonical);
    let interface_hash = h.finalize();

    // source_digest:接口字节 + entry 源文本段(含函数体;跨文件 out-of-line mod
    // 的 entry 在 v1 不可切片 → fail-closed)。
    if span.file != main_file {
        return Err(ReflectError::Unsupported {
            detail: format!(
                "entry `{name_path}` 位于 out-of-line 模块(v1 源切片限主文件;RXS-0196 装配面归后续)"
            ),
        });
    }
    let slice = src
        .get(span.lo.0 as usize..span.hi.0 as usize)
        .ok_or_else(|| ReflectError::Unsupported {
            detail: format!("entry `{name_path}` 源切片越界(实现 bug,fail-closed)"),
        })?;
    let mut sw = CanonW::new();
    sw.bytes(&canonical);
    sw.strv(slice);
    let mut h = sha256::Sha256::new();
    h.update(SOURCE_DOMAIN);
    h.update(&sw.buf);
    let source_digest = h.finalize();

    // pipeline key 组成见证(RXS-0306;字段序 = spec 冻结序)。
    let compiler_version = env!("CARGO_PKG_VERSION").to_owned();
    let mut kw = CanonW::new();
    kw.strv(name_path);
    kw.bytes(&interface_hash);
    kw.bytes(&source_digest);
    kw.bytes(&profile_digest);
    kw.bytes(&perm_digest);
    kw.strv(&variant_key);
    kw.strv(COMPILER);
    kw.strv(&compiler_version);
    kw.strv(EDITION_RX0);
    kw.strv(TARGET);
    kw.strv(BACKEND);
    let mut h = sha256::Sha256::new();
    h.update(KEY_DOMAIN);
    h.update(&kw.buf);
    let pipeline_key = h.finalize();

    Ok(EntryReflection {
        name: name_path.to_owned(),
        stage: stage_name(stage),
        stage_tag,
        stage_visibility: visibility,
        io,
        resources,
        push_constants: pc,
        execution_modes: exec,
        required_capabilities,
        selected_profile_digest: profile_digest,
        permutation_domain_digest: perm_digest,
        variant_key,
        canonical,
        interface_hash,
        source_digest,
        pipeline_key,
        global_descriptor_indices: gdi,
    })
}

/// 构建编译单元的 reflection v1 文档(RXS-0304~0306;纯函数,同输入恒同输出)。
///
/// `src` 为主文件源文本(entry 源切片用),`main_file` 为其 SourceId。推导失败
/// (绑定不可映射 / 超出闭集 / 同名 entry 冲突 / permutation 域违例或 select
/// KEY 非法)→ `ReflectError`,fail-closed,不产部分产物。`perm_plan` = M29
/// `--permutation-select`/`--permutation-budget` 输入面(默认 → 空域路径 0 漂移)。
/// `cap_plan` = M32 `--profile`/capability 选择律输入面(RXS-0304 v1.2;默认 →
/// `required_capabilities` 恒空表、`selected_profile_digest` 恒既有常量、无
/// 主 variant 抑制,空路径与 M31/M29 基线逐字节 0 漂移)。
pub fn build_reflection(
    file: &ast::SourceFile,
    src: &str,
    main_file: SourceId,
    perm_plan: &ReflectionPermPlan<'_>,
    cap_plan: &ReflectionCapPlan<'_>,
) -> Result<ReflectionDoc, ReflectError> {
    build_reflection_with_desc(file, src, main_file, perm_plan, cap_plan, &ReflectionDescPlan::default())
}

/// RXS-0347(M103)加性入口:携 `ReflectionDescPlan`(「资源 → 全局 descriptor 索引」
/// 尾随可选字段供给面)。缺省 plan ≡ [`build_reflection`](序列化 0-drift,既有产物
/// 0-byte);真值化 plan → 每 entry canonical 尾随逐资源全局索引 + JSON 加性字段。
pub fn build_reflection_with_desc(
    file: &ast::SourceFile,
    src: &str,
    main_file: SourceId,
    perm_plan: &ReflectionPermPlan<'_>,
    cap_plan: &ReflectionCapPlan<'_>,
    desc_plan: &ReflectionDescPlan,
) -> Result<ReflectionDoc, ReflectError> {
    let mut raw: Vec<(String, &ast::FnItem, &[ast::Attr], Span)> = Vec::new();
    collect_entries(&file.items, "", &mut raw);
    // 同名 entry(裸名)跨 mod 冲突 → fail-closed(提取层按裸名首个匹配,
    // 不猜测归属;RXS-0304 诚实边界)。
    for (i, (path_a, _, _, _)) in raw.iter().enumerate() {
        for (path_b, _, _, _) in &raw[i + 1..] {
            let bare_a = path_a.rsplit("::").next().unwrap_or(path_a);
            let bare_b = path_b.rsplit("::").next().unwrap_or(path_b);
            if bare_a == bare_b {
                return Err(ReflectError::Unsupported {
                    detail: format!(
                        "同名 entry `{bare_a}` 出现于 `{path_a}` 与 `{path_b}`(v1 不裁决跨 mod 同名归属)"
                    ),
                });
            }
        }
    }

    let mut entries = Vec::with_capacity(raw.len());
    let mut select_applied = false;
    for (path, f, attrs, span) in &raw {
        let e = build_entry(
            file, src, main_file, path, f, attrs, *span, perm_plan, cap_plan, desc_plan,
        )?;
        if !e.variant_key.is_empty() {
            select_applied = true;
        }
        entries.push(e);
    }
    // M32(RXS-0312):fallback 选中的逻辑 entry 主 variant 不发射——其记录不
    // 进文档(名字段 = fallback entry 的 entry identity 的记录由 fallback
    // entry 自身承担;逻辑名→实体映射在 capabilities 报告中可查)。
    if let Some(sel) = cap_plan.selection {
        entries.retain(|e| !sel.suppressed_names.contains(&e.name));
    }
    // `--permutation-select` 给出但单元内无任何非空 permutation 域可应用 →
    // 确定性错误(RX3019 类;静默忽略属「最接近」式回退的变体,禁)。
    if perm_plan.select.is_some() && !select_applied {
        return Err(ReflectError::Permutation {
            detail: "`--permutation-select` 给出,但编译单元内无任何 entry 声明 permutation 域(无可选对象,RXS-0310)".to_owned(),
        });
    }
    // 规范键排序(字节序字典序;与源文件声明序无关,RXS-0305)。
    entries.sort_by(|a, b| (a.name.as_str(), a.stage_tag).cmp(&(b.name.as_str(), b.stage_tag)));

    // 文档级 canonical bytes(版本前缀起始)。
    let compiler_version = env!("CARGO_PKG_VERSION").to_owned();
    let mut w = CanonW::new();
    w.bytes(b"rurix.reflection.v1\0");
    w.u32v(SCHEMA_VERSION);
    w.strv(COMPILER);
    w.strv(&compiler_version);
    w.strv(EDITION_RX0);
    w.strv(TARGET);
    w.strv(BACKEND);
    w.u32v(entries.len() as u32);
    for e in &entries {
        w.strv(&e.name);
        w.u32v(e.stage_tag);
        w.u32v(e.canonical.len() as u32);
        w.bytes(&e.canonical);
    }
    let canonical = w.buf;
    let mut h = sha256::Sha256::new();
    h.update(UNIT_DOMAIN);
    h.update(&canonical);
    let unit_digest = h.finalize();

    Ok(ReflectionDoc {
        compiler_version,
        entries,
        canonical,
        unit_digest,
    })
}

// ═══════════════════════ 装配期核验(RXS-0307) ═══════════════════════

/// 装配期接口核验失败(typed `Err`,fail-closed;by construction 不含任何
/// 修复/再反射路径,RXS-0307)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InterfaceMismatch {
    /// 相异字段名(只携带字段名级事实,不携带布局猜测值)。
    pub field: &'static str,
}

impl std::fmt::Display for InterfaceMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "interface mismatch on `{}` (fail-closed, RXS-0307)",
            self.field
        )
    }
}

impl std::error::Error for InterfaceMismatch {}

/// 装配期核验原语(RXS-0307):先核 `schema` 常量与 `schema_version`,再比对
/// `interface_hash`;任一不符 → `Err(InterfaceMismatch{ field })`;全部一致 → `Ok(())`。
///
/// 本函数不做、也不提供重反射或 host layout 猜测修复(RFC-0019 §4.4 逐字)。
pub fn verify_interface_pair(
    expected_schema: &str,
    expected_version: u32,
    expected_hash: &[u8; 32],
    actual_schema: &str,
    actual_version: u32,
    actual_hash: &[u8; 32],
) -> Result<(), InterfaceMismatch> {
    if expected_schema != actual_schema {
        return Err(InterfaceMismatch { field: "schema" });
    }
    if expected_version != actual_version {
        return Err(InterfaceMismatch {
            field: "schema_version",
        });
    }
    if expected_hash != actual_hash {
        return Err(InterfaceMismatch {
            field: "interface_hash",
        });
    }
    Ok(())
}

// ═══════════════════════ JSON 产物(确定性 canonical JSON) ═══════════════════════

/// JSON 串转义(接口名为标识符,防御性转义引号/反斜杠/控制字符)。
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn hex_of(d: &[u8; 32]) -> String {
    sha256::hex(d)
}

fn hex_bytes(b: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push(HEX[(x >> 4) as usize] as char);
        s.push(HEX[(x & 0xF) as usize] as char);
    }
    s
}

/// entry 记录 → 确定性 JSON 片段(键序固定 = RXS-0304 闭集声明序;整数不浮点)。
fn entry_json(e: &EntryReflection, ind: &str) -> String {
    let mut s = String::new();
    let i2 = format!("{ind}  ");
    let i3 = format!("{ind}    ");
    s.push_str(&format!("{ind}{{\n"));
    s.push_str(&format!("{i2}\"name\": \"{}\",\n", json_escape(&e.name)));
    s.push_str(&format!("{i2}\"stage\": \"{}\",\n", e.stage));
    s.push_str(&format!("{i2}\"stage_tag\": {},\n", e.stage_tag));
    s.push_str(&format!(
        "{i2}\"stage_visibility\": {},\n",
        e.stage_visibility
    ));
    // io
    if e.io.is_empty() {
        s.push_str(&format!("{i2}\"io\": [],\n"));
    } else {
        s.push_str(&format!("{i2}\"io\": [\n"));
        for (k, f) in e.io.iter().enumerate() {
            let loc = f.location.map_or("null".to_owned(), |n| n.to_string());
            s.push_str(&format!(
                "{i3}{{\"name\": \"{}\", \"dir\": \"{}\", \"kind\": \"{}\", \"annotation\": \"{}\", \"type\": \"{}\", \"location\": {}}}{}\n",
                json_escape(&f.name), f.dir, f.kind, json_escape(&f.annotation), json_escape(&f.ty), loc,
                if k + 1 == e.io.len() { "" } else { "," },
            ));
        }
        s.push_str(&format!("{i2}],\n"));
    }
    // resources
    if e.resources.is_empty() {
        s.push_str(&format!("{i2}\"resources\": [],\n"));
    } else {
        s.push_str(&format!("{i2}\"resources\": [\n"));
        for (k, r) in e.resources.iter().enumerate() {
            s.push_str(&format!(
                "{i3}{{\"name\": \"{}\", \"class\": \"{}\", \"set\": {}, \"binding\": {}, \"count\": {}, \"access\": \"{}\", \"format\": \"{}\", \"visibility\": {}}}{}\n",
                json_escape(&r.name), r.class, r.set, r.binding, r.count, r.access, json_escape(&r.format), r.visibility,
                if k + 1 == e.resources.len() { "" } else { "," },
            ));
        }
        s.push_str(&format!("{i2}],\n"));
    }
    // push_constants
    if e.push_constants.members.is_empty() {
        s.push_str(&format!(
            "{i2}\"push_constants\": {{\"members\": [], \"size_bytes\": 0}},\n"
        ));
    } else {
        s.push_str(&format!("{i2}\"push_constants\": {{\"members\": [\n"));
        for (k, m) in e.push_constants.members.iter().enumerate() {
            s.push_str(&format!(
                "{i3}{{\"name\": \"{}\", \"type\": \"{}\", \"member\": {}, \"offset\": {}, \"size\": {}}}{}\n",
                json_escape(&m.name), m.ty, m.member, m.offset, m.size,
                if k + 1 == e.push_constants.members.len() { "" } else { "," },
            ));
        }
        s.push_str(&format!(
            "{i2}], \"size_bytes\": {}}},\n",
            e.push_constants.size_bytes
        ));
    }
    // execution_modes(非 mesh = null 字段空编码)
    let (nt, mv, mp) = match &e.execution_modes {
        ExecutionModes {
            numthreads: Some((x, y, z)),
            max_vertices: Some(v),
            max_primitives: Some(p),
        } => (
            format!("[{}, {}, {}]", x, y, z),
            v.to_string(),
            p.to_string(),
        ),
        _ => ("null".to_owned(), "null".to_owned(), "null".to_owned()),
    };
    s.push_str(&format!(
        "{i2}\"execution_modes\": {{\"numthreads\": {}, \"max_vertices\": {}, \"max_primitives\": {}}},\n",
        nt, mv, mp
    ));
    // M50 保留位(恒空表)。
    s.push_str(&format!("{i2}\"rt_payloads\": [],\n"));
    s.push_str(&format!("{i2}\"rt_hit_attributes\": [],\n"));
    s.push_str(&format!("{i2}\"rt_callable_data\": [],\n"));
    s.push_str(&format!("{i2}\"rt_task_payloads\": [],\n"));
    s.push_str(&format!("{i2}\"shader_records\": [],\n"));
    s.push_str(&format!("{i2}\"rt_group_membership\": [],\n"));
    s.push_str(&format!("{i2}\"library_exports\": [],\n"));
    // M32 真值化(RXS-0304 v1.2):有效 requirement 集排序 ID 表(空 = 恒空表)。
    if e.required_capabilities.is_empty() {
        s.push_str(&format!("{i2}\"required_capabilities\": [],\n"));
    } else {
        let caps = e
            .required_capabilities
            .iter()
            .map(|c| format!("\"{}\"", json_escape(c)))
            .collect::<Vec<_>>()
            .join(", ");
        s.push_str(&format!("{i2}\"required_capabilities\": [{caps}],\n"));
    }
    s.push_str(&format!(
        "{i2}\"selected_profile_digest\": \"{}\",\n",
        hex_of(&e.selected_profile_digest)
    ));
    s.push_str(&format!(
        "{i2}\"permutation_domain_digest\": \"{}\",\n",
        hex_of(&e.permutation_domain_digest)
    ));
    s.push_str(&format!(
        "{i2}\"variant_key\": \"{}\",\n",
        json_escape(&e.variant_key)
    ));
    // RXS-0347 尾随可选字段(JSON 加性;缺省 = 字段不发射,0-drift)。
    if let Some(gdi) = &e.global_descriptor_indices {
        let list = gdi
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        s.push_str(&format!(
            "{i2}\"global_descriptor_indices\": [{list}],\n"
        ));
    }
    // digest 面。
    s.push_str(&format!(
        "{i2}\"interface_hash\": \"{}\",\n",
        hex_of(&e.interface_hash)
    ));
    s.push_str(&format!(
        "{i2}\"source_digest\": \"{}\",\n",
        hex_of(&e.source_digest)
    ));
    s.push_str(&format!(
        "{i2}\"pipeline_key\": \"{}\",\n",
        hex_of(&e.pipeline_key)
    ));
    s.push_str(&format!(
        "{i2}\"canonical_hex\": \"{}\"\n",
        hex_bytes(&e.canonical)
    ));
    s.push_str(&format!("{ind}}}"));
    s
}

/// reflection 文档 → 确定性 JSON 产物(键序固定、UTF-8、LF 行尾;不含路径/
/// 文件名/时间戳,RXS-0305 禁用面)。
pub fn to_json(doc: &ReflectionDoc) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"schema\": \"{}\",\n", SCHEMA_ID));
    s.push_str(&format!("  \"schema_version\": {},\n", SCHEMA_VERSION));
    s.push_str(&format!("  \"compiler\": \"{}\",\n", COMPILER));
    s.push_str(&format!(
        "  \"compiler_version\": \"{}\",\n",
        json_escape(&doc.compiler_version)
    ));
    s.push_str(&format!("  \"edition\": \"{}\",\n", EDITION_RX0));
    s.push_str(&format!("  \"target\": \"{}\",\n", TARGET));
    s.push_str(&format!("  \"backend\": \"{}\",\n", BACKEND));
    s.push_str(&format!(
        "  \"unit_digest\": \"{}\",\n",
        hex_of(&doc.unit_digest)
    ));
    if doc.entries.is_empty() {
        s.push_str("  \"entries\": []\n");
    } else {
        s.push_str("  \"entries\": [\n");
        for (k, e) in doc.entries.iter().enumerate() {
            s.push_str(&entry_json(e, "    "));
            s.push_str(if k + 1 == doc.entries.len() {
                "\n"
            } else {
                ",\n"
            });
        }
        s.push_str("  ]\n");
    }
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::source_map::SourceMap;
    use crate::span::Edition;

    fn parse_src(src: &str) -> (ast::SourceFile, SourceId) {
        let diag = DiagCtxt::new();
        let mut sm = SourceMap::new();
        let id = sm.add_file("test.rx".to_owned(), src, Edition::Rx0);
        let toks = crate::lexer::lex(src, id, Edition::Rx0, &diag);
        let file = crate::parser::parse(src, toks, id, Edition::Rx0, &diag);
        assert!(!diag.has_errors(), "测试源须解析干净");
        (file, id)
    }

    fn reflect(src: &str) -> ReflectionDoc {
        let (file, id) = parse_src(src);
        build_reflection(
            &file,
            src,
            id,
            &ReflectionPermPlan::default(),
            &ReflectionCapPlan::default(),
        )
        .expect("reflection 推导须成功")
    }

    /// 覆盖 compute 族(View/ViewMut/Atomic/AccelStruct/标量/ThreadCtx)与
    /// vertex/fragment 图形对(io 结构体 + Texture2D/Sampler)的基准源。
    const BASE: &str = r#"
struct VsOut {
    #[builtin(position)] pos: f32,
    #[interpolate(perspective)] uv: f32,
    #[interpolate(flat)] mat_id: u32,
}

vertex fn vs_main(inp: VsOut, tex: Texture2D<f32>, samp: Sampler) -> VsOut {
    VsOut { pos: 0.0, uv: 0.0, mat_id: 0 }
}

fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler) -> VsOut {
    inp
}

kernel fn kmain(
    t: ThreadCtx<1>,
    tlas: AccelStruct,
    buf: ViewMut<global, f32>,
    n: u32,
) {
    let i = t.global_id();
    if i < n {
        buf[i] = 1.0;
    }
}

fn main() {}
"#;

    fn entry<'a>(doc: &'a ReflectionDoc, name: &str) -> &'a EntryReflection {
        doc.entries
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("entry {name} 应在 reflection 中"))
    }

    /// 双次构建:canonical 字节与 digest 逐字节相等(M31 判据腿①)。
    //@ spec: RXS-0305, RXS-0306
    #[test]
    fn double_reflection_is_byte_identical() {
        let a = reflect(BASE);
        let b = reflect(BASE);
        assert_eq!(a.canonical, b.canonical);
        assert_eq!(a.unit_digest, b.unit_digest);
        assert_eq!(a.entries.len(), 3);
        for (ea, eb) in a.entries.iter().zip(b.entries.iter()) {
            assert_eq!(ea.canonical, eb.canonical);
            assert_eq!(ea.interface_hash, eb.interface_hash);
            assert_eq!(ea.pipeline_key, eb.pipeline_key);
        }
        // 规范键排序(与声明序无关):compute(kmain) < fragment(fs_main)? 按名:
        // fs_main < kmain < vs_main。
        let names: Vec<&str> = a.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["fs_main", "kmain", "vs_main"]);
    }

    /// entry 声明次序置换 + 文件改名等价(路径不入产物)→ canonical 字节与
    /// hash 不变(M31 判据腿②:声明序/无语义路径扰动)。
    //@ spec: RXS-0304, RXS-0305
    #[test]
    fn declaration_order_permutation_is_invariant() {
        // 手工重排:把 fs_main 整段挪到 vs_main 之前(entry 声明序置换)。
        let fs_block = "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler) -> VsOut {\n    inp\n}\n\n";
        assert!(BASE.contains(fs_block));
        let reordered = BASE.replace(fs_block, "");
        let reordered =
            reordered.replace("vertex fn vs_main", &format!("{fs_block}vertex fn vs_main"));
        assert_ne!(reordered, BASE); // 防御:置换须真实生效
        let a = reflect(BASE);
        let b = reflect(&reordered);
        assert_eq!(a.canonical, b.canonical, "entry 声明序置换后文档字节须不变");
        assert_eq!(a.unit_digest, b.unit_digest);
        for name in ["vs_main", "fs_main", "kmain"] {
            assert_eq!(
                entry(&a, name).interface_hash,
                entry(&b, name).interface_hash
            );
        }
    }

    /// 仅改函数体(字面量)→ interface_hash 不变、source_digest 必变
    /// (M31 判据腿③;RFC-0019 §4.4 分离规则)。
    //@ spec: RXS-0306
    #[test]
    fn body_only_change_keeps_interface_hash() {
        let edited = BASE.replace("buf[i] = 1.0;", "buf[i] = 2.0;");
        assert_ne!(edited, BASE);
        let a = reflect(BASE);
        let b = reflect(&edited);
        let (ka, kb) = (entry(&a, "kmain"), entry(&b, "kmain"));
        assert_eq!(
            ka.interface_hash, kb.interface_hash,
            "仅函数体改动接口 hash 不变"
        );
        assert_ne!(
            ka.source_digest, kb.source_digest,
            "函数体改动 source digest 必变"
        );
        assert_ne!(
            ka.pipeline_key, kb.pipeline_key,
            "pipeline key 含 source digest,必变"
        );
        // 其余 entry 完全不动。
        assert_eq!(
            entry(&a, "vs_main").source_digest,
            entry(&b, "vs_main").source_digest
        );
    }

    /// ABI 扰动四轴(各自独立断言,不合并):binding / resource kind /
    /// stage visibility / value type 任一改变 → interface_hash 必变(M31 判据腿④)。
    //@ spec: RXS-0304, RXS-0306
    #[test]
    fn abi_binding_change_flips_hash() {
        // 交换 fs_main 两个资源形参的声明序 → binding 分配交换(t 轴 0↔1 不变?
        // 注意:tex_b(Texture2D)与 samp(Sampler)不同类轴,交换后各类轴内序号
        // 不变——故本轴改用「新增一个同类资源顶位」令原资源 binding 位移)。
        let edited = BASE.replace(
            "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler)",
            "fragment fn fs_main(inp: VsOut, tex_a: Texture2D<f32>, tex_b: Texture2D<f32>, samp: Sampler)",
        );
        assert_ne!(edited, BASE);
        let a = reflect(BASE);
        let b = reflect(&edited);
        let (fa, fb) = (entry(&a, "fs_main"), entry(&b, "fs_main"));
        // 原 tex_b 在 B 中 binding 自 0 → 1(t 轴顶入 tex_a)。
        assert_eq!(fa.resources[0].name, "tex_b");
        assert_eq!(fa.resources[0].binding, 0);
        assert_eq!(fb.resources[1].name, "tex_b");
        assert_eq!(fb.resources[1].binding, 1);
        assert_ne!(
            fa.interface_hash, fb.interface_hash,
            "binding 改变 hash 必变"
        );
    }

    //@ spec: RXS-0304, RXS-0306
    #[test]
    fn abi_resource_kind_change_flips_hash() {
        // Texture2D → TextureRw2D:SRV → UAV(class 变)。
        let edited = BASE.replace(
            "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler)",
            "fragment fn fs_main(inp: VsOut, tex_b: TextureRw2D<f32>, samp: Sampler)",
        );
        assert_ne!(edited, BASE);
        let a = reflect(BASE);
        let b = reflect(&edited);
        let (fa, fb) = (entry(&a, "fs_main"), entry(&b, "fs_main"));
        assert_eq!(fa.resources[0].class, "srv");
        assert_eq!(fb.resources[0].class, "uav");
        assert_ne!(fa.interface_hash, fb.interface_hash);
    }

    //@ spec: RXS-0304, RXS-0306
    #[test]
    fn abi_stage_visibility_change_flips_hash() {
        // samp 自 fs_main 挪到 vs_main:资源的 stage 可见性 fragment → vertex。
        let edited = BASE
            .replace(
                "vertex fn vs_main(inp: VsOut, tex: Texture2D<f32>, samp: Sampler)",
                "vertex fn vs_main(inp: VsOut, tex: Texture2D<f32>)",
            )
            .replace(
                "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler)",
                "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>)",
            );
        // 上一步只是删除——可见性变化的净形态 = vs 失 samp(该 entry hash 变)。
        assert_ne!(edited, BASE);
        let a = reflect(BASE);
        let b = reflect(&edited);
        assert_ne!(
            entry(&a, "vs_main").interface_hash,
            entry(&b, "vs_main").interface_hash,
            "资源离开某阶段 = stage visibility 变化,hash 必变"
        );
        // 资源 visibility 位:fragment(1<<1)= 2。
        assert_eq!(entry(&a, "fs_main").resources[1].visibility, 2);
    }

    //@ spec: RXS-0304, RXS-0306
    #[test]
    fn abi_value_type_change_flips_hash() {
        // io 字段 value type:mat_id u32 → i32。
        let edited = BASE.replace("mat_id: u32", "mat_id: i32");
        assert_ne!(edited, BASE);
        let a = reflect(BASE);
        let b = reflect(&edited);
        assert_eq!(entry(&a, "vs_main").io[2].ty, "u32");
        assert_eq!(entry(&b, "vs_main").io[2].ty, "i32");
        assert_ne!(
            entry(&a, "vs_main").interface_hash,
            entry(&b, "vs_main").interface_hash
        );
        // compute 标量形参 value type:n u32 → u64(push-constant 布局 4→8 字节)。
        let edited2 = BASE.replace("n: u32,", "n: u64,");
        let c = reflect(&edited2);
        let kc = entry(&c, "kmain");
        assert_eq!(kc.push_constants.members[0].ty, "u64");
        assert_eq!(kc.push_constants.members[0].size, 8);
        assert_ne!(entry(&a, "kmain").interface_hash, kc.interface_hash);
    }

    /// compute 族布局与绑定:kmain 的 AccelStruct/ViewMut 占 binding 0/1,
    /// 标量 n 落 push-constant member 0;ThreadCtx 不占任何 ABI 字段。
    //@ spec: RXS-0304
    #[test]
    fn compute_bindings_and_push_constants_layout() {
        let doc = reflect(BASE);
        let k = entry(&doc, "kmain");
        assert_eq!(k.resources.len(), 2, "ThreadCtx 不占资源位");
        assert_eq!(k.resources[0].name, "tlas");
        assert_eq!(k.resources[0].class, "accel");
        assert_eq!(k.resources[0].set, 0);
        assert_eq!(k.resources[0].binding, 0);
        assert_eq!(k.resources[1].name, "buf");
        assert_eq!(k.resources[1].class, "uav");
        assert_eq!(k.resources[1].binding, 1);
        assert_eq!(k.resources[1].format, "f32");
        assert_eq!(k.push_constants.members.len(), 1);
        assert_eq!(k.push_constants.members[0].name, "n");
        assert_eq!(k.push_constants.members[0].offset, 0);
        assert_eq!(k.push_constants.size_bytes, 4);
        assert_eq!(k.stage_visibility, 1 << 2); // compute = tag 2
    }

    /// graphics 绑定 = Vk-native 分配律(vk-native:set=类别轴,binding=类内序)。
    //@ spec: RXS-0304
    #[test]
    fn graphics_bindings_vk_native() {
        let doc = reflect(BASE);
        let fs = entry(&doc, "fs_main");
        assert_eq!(fs.resources[0].name, "tex_b");
        assert_eq!((fs.resources[0].set, fs.resources[0].binding), (1, 0)); // SRV 轴
        assert_eq!(fs.resources[1].name, "samp");
        assert_eq!((fs.resources[1].set, fs.resources[1].binding), (3, 0)); // Sampler 轴
        // io:builtin position 不占 location;uv(in/out 各自序)…
        let vs = entry(&doc, "vs_main");
        assert_eq!(vs.io[0].kind, "builtin");
        assert_eq!(vs.io[0].location, None);
        assert_eq!(vs.io[1].name, "uv");
        assert_eq!(vs.io[1].location, Some(0));
        assert_eq!(vs.io[2].location, Some(1));
        // fs_main 的 in 方向:pos(builtin,无 location)/uv loc0/mat_id loc1。
        assert_eq!(fs.io[0].dir, "in");
        assert_eq!(fs.io[1].location, Some(0));
    }

    /// 空编码稳定(M29/M32/M50 未实现字段的确定性编码):profile/permutation
    /// digest 为域分隔常量,RT 字段位恒零计数。
    //@ spec: RXS-0304
    #[test]
    fn empty_encodings_are_stable_constants() {
        let a = reflect(BASE);
        let b = reflect("fn main() {}\n");
        assert_eq!(b.entries.len(), 0, "无 entry 单元产空表(确定性)");
        let e = entry(&a, "kmain");
        assert_eq!(
            e.selected_profile_digest,
            sha256::digest(b"rurix.profile-none.v1\0")
        );
        assert_eq!(
            e.permutation_domain_digest,
            sha256::digest(b"rurix.permutation-domain-empty.v1\0")
        );
        assert_eq!(e.variant_key, "");
    }

    /// 同名 entry 跨 mod → 推导确定性失败(fail-closed,不猜测归属)。
    //@ spec: RXS-0304
    #[test]
    fn duplicate_bare_entry_name_fails_closed() {
        let src = "mod a { vertex fn dup() {} }\nmod b { vertex fn dup() {} }\n";
        let (file, id) = parse_src(src);
        let err = build_reflection(
            &file,
            src,
            id,
            &ReflectionPermPlan::default(),
            &ReflectionCapPlan::default(),
        )
        .expect_err("同名 entry 须 fail-closed");
        assert_eq!(err.error_code(), 6026);
    }

    /// 无界非 SRV 纹理表(无界 Sampler 数组)→ 绑定推导不可映射(RX6013 类别)。
    //@ spec: RXS-0304
    #[test]
    fn unbounded_sampler_table_is_unmappable() {
        let src = "vertex fn v(samps: [Sampler]) {}\n";
        let (file, id) = parse_src(src);
        let err = build_reflection(
            &file,
            src,
            id,
            &ReflectionPermPlan::default(),
            &ReflectionCapPlan::default(),
        )
        .expect_err("无界 Sampler 表须 fail-closed");
        assert_eq!(err.error_code(), 6013);
    }

    /// compute 形参超闭集(命名结构体形参)→ Unsupported(RX6026 类别)。
    //@ spec: RXS-0304
    #[test]
    fn compute_param_outside_closed_set_fails() {
        let src = "struct S { x: f32 }\nkernel fn k(t: ThreadCtx<1>, s: S) {}\n";
        let (file, id) = parse_src(src);
        let err = build_reflection(
            &file,
            src,
            id,
            &ReflectionPermPlan::default(),
            &ReflectionCapPlan::default(),
        )
        .expect_err("超闭集形参须 fail-closed");
        assert_eq!(err.error_code(), 6026);
    }

    /// 装配期核验(RXS-0307):schema/version/hash 三级;不符即 typed Err,
    /// by construction 无修复路径。
    //@ spec: RXS-0307
    #[test]
    fn verify_interface_pair_fail_closed() {
        let a = reflect(BASE);
        let h = entry(&a, "kmain").interface_hash;
        assert!(
            verify_interface_pair(SCHEMA_ID, SCHEMA_VERSION, &h, SCHEMA_ID, SCHEMA_VERSION, &h)
                .is_ok()
        );
        assert_eq!(
            verify_interface_pair("other", SCHEMA_VERSION, &h, SCHEMA_ID, SCHEMA_VERSION, &h)
                .unwrap_err()
                .field,
            "schema"
        );
        assert_eq!(
            verify_interface_pair(SCHEMA_ID, 99, &h, SCHEMA_ID, SCHEMA_VERSION, &h)
                .unwrap_err()
                .field,
            "schema_version"
        );
        let other = reflect(&BASE.replace("n: u32,", "n: u64,"));
        let h2 = entry(&other, "kmain").interface_hash;
        assert_eq!(
            verify_interface_pair(
                SCHEMA_ID,
                SCHEMA_VERSION,
                &h,
                SCHEMA_ID,
                SCHEMA_VERSION,
                &h2
            )
            .unwrap_err()
            .field,
            "interface_hash"
        );
    }

    /// JSON 产物面:确定性(两次逐字节相等)+ 键齐 + 不含路径/时间戳
    /// (RXS-0305 禁用面);canonical_hex 与 canonical 一致。
    //@ spec: RXS-0305
    #[test]
    fn json_artifact_is_deterministic_and_pure() {
        let a = reflect(BASE);
        let b = reflect(BASE);
        let (ja, jb) = (to_json(&a), to_json(&b));
        assert_eq!(ja, jb, "JSON 产物两次生成须逐字节相等");
        assert!(ja.ends_with("}\n"), "LF 尾换行");
        assert!(!ja.contains('\r'), "禁 CRLF");
        assert!(!ja.contains("test.rx"), "文件名/路径不得入产物");
        for e in &a.entries {
            assert!(ja.contains(&format!(
                "\"interface_hash\": \"{}\"",
                hex_of(&e.interface_hash)
            )));
            assert!(ja.contains(&hex_bytes(&e.canonical)));
        }
    }

    // ── RXS-0347(G9.2 M103,RFC-0023 §4.3)尾随可选字段 0-drift / 真值化 / RED ──

    fn reflect_desc(src: &str, plan: &ReflectionDescPlan) -> ReflectionDoc {
        let (file, id) = parse_src(src);
        build_reflection_with_desc(
            &file,
            src,
            id,
            &ReflectionPermPlan::default(),
            &ReflectionCapPlan::default(),
            plan,
        )
        .expect("reflection 推导须成功")
    }

    /// **0-drift 恒跑(RXS-0347 §1)**:缺省 plan(`global_index_table = None`)产物
    /// 与缺省 `build_reflection` 逐字节相等(canonical/interface_hash/JSON 三面);
    /// 「不得以 count 0 冒充 0-byte」。
    //@ spec: RXS-0347
    #[test]
    fn gdi_absent_is_byte_identical_zero_drift() {
        let base = reflect(BASE);
        let via_desc = reflect_desc(BASE, &ReflectionDescPlan::default());
        assert_eq!(base.canonical, via_desc.canonical, "文档 canonical 0-drift");
        assert_eq!(base.unit_digest, via_desc.unit_digest);
        for (ea, eb) in base.entries.iter().zip(via_desc.entries.iter()) {
            assert_eq!(ea.canonical, eb.canonical, "entry canonical 0-drift");
            assert_eq!(ea.interface_hash, eb.interface_hash);
            assert!(eb.global_descriptor_indices.is_none());
        }
        assert_eq!(to_json(&base), to_json(&via_desc), "JSON 0-drift");
        assert!(!to_json(&via_desc).contains("global_descriptor_indices"));
    }

    /// **真值化(RXS-0347 §2/§3)**:plan 供给「资源 → 全局索引」→ 每 entry canonical
    /// 尾随逐资源索引(声明序)+ JSON 加性字段;同输入同映射逐字节等值;与
    /// set/binding 并存不删(既有字段不动)。
    //@ spec: RXS-0347
    #[test]
    fn gdi_truth_table_trailing_additive() {
        // BASE 的 fs_main 资源 = tex_b(srv)/ samp(sampler);vs_main = tex/samp;
        // kmain = tlas(accel)/ buf(uav)。分配序 = 资源声明序(确定性)。
        let plan = ReflectionDescPlan {
            global_index_table: Some(vec![
                ("tex".to_owned(), 0),
                ("samp".to_owned(), 1),
                ("tex_b".to_owned(), 2),
                ("tlas".to_owned(), 3),
                ("buf".to_owned(), 4),
            ]),
            index_budget: Some(65536),
        };
        let a = reflect_desc(BASE, &plan);
        let b = reflect_desc(BASE, &plan);
        assert_eq!(a.canonical, b.canonical, "同输入同映射逐字节等值");
        let fs = a.entries.iter().find(|e| e.name == "fs_main").unwrap();
        let gdi = fs.global_descriptor_indices.as_ref().unwrap();
        // fs_main 资源声明序 = tex_b, samp → [2, 1]。
        assert_eq!(gdi, &vec![2u32, 1], "尾随索引 = 声明序映射值");
        // set/binding 并存不删(既有字段不变)。
        assert!(fs.resources.iter().any(|r| r.name == "tex_b" && r.class == "srv"));
        let json = to_json(&a);
        assert!(json.contains("\"global_descriptor_indices\": [2, 1]"));
        // 缺省基线 canonical 是真值化的前缀(尾随加性,不改既有字节)。
        let base = reflect(BASE);
        let base_fs = base.entries.iter().find(|e| e.name == "fs_main").unwrap();
        assert!(
            fs.canonical.starts_with(&base_fs.canonical[..]),
            "真值化 canonical = 既有字节流 + 尾随索引(0-drift 加性)"
        );
    }

    /// **RED:悬空/越界 fail-closed(RXS-0347 §3/§4)**:资源缺映射(悬空)→
    /// 确定性拒绝;索引 ≥ 预算 → 装配期确定性拒绝。
    //@ spec: RXS-0347
    #[test]
    fn gdi_dangling_and_budget_fail_closed() {
        // 悬空:plan 缺 `buf` 的映射。
        let dangling = ReflectionDescPlan {
            global_index_table: Some(vec![
                ("tex".to_owned(), 0),
                ("samp".to_owned(), 1),
                ("tex_b".to_owned(), 2),
                ("tlas".to_owned(), 3),
            ]),
            index_budget: None,
        };
        let (file, id) = parse_src(BASE);
        let r = build_reflection_with_desc(
            &file,
            BASE,
            id,
            &ReflectionPermPlan::default(),
            &ReflectionCapPlan::default(),
            &dangling,
        );
        assert!(r.is_err(), "悬空索引应 fail-closed");
        // 越界:索引 ≥ 预算。
        let over = ReflectionDescPlan {
            global_index_table: Some(vec![
                ("tex".to_owned(), 0),
                ("samp".to_owned(), 1),
                ("tex_b".to_owned(), 2),
                ("tlas".to_owned(), 3),
                ("buf".to_owned(), 70000),
            ]),
            index_budget: Some(65536),
        };
        let (file2, id2) = parse_src(BASE);
        let r2 = build_reflection_with_desc(
            &file2,
            BASE,
            id2,
            &ReflectionPermPlan::default(),
            &ReflectionCapPlan::default(),
            &over,
        );
        assert!(r2.is_err(), "超预算索引应 fail-closed");
    }
}
