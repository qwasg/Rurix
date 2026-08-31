//! Rust 级多 pass 图形执行器(RFC-0016 章 B 主通道;G5 门 G-G5-4 前置;U32)。
//!
//! 消费方为引擎渲染器库(`rurix-render` 图编译器/uc06):接受资源描述(buffer/texture2d)+
//! pass 序列(`Raster`/`Compute` 混合)+ **pass 间屏障计划**(调用方图编译器产出的简化
//! 「每 pass 前 (资源, 目标状态) 转换表」)+ readback 请求,创建 device、逐 pass 录制
//! (内建 pipeline cache:相同 SPIR-V+格式组合复用 `VkPipeline`)、屏障经
//! `vkCmdPipelineBarrier2KHR`(`VK_KHR_synchronization2`)逐字回放、执行、readback 返字节。
//! 既有 `vk.rs` 全部入口(`run_compute`/`run_graphics_offscreen*`/`run_mesh_offscreen`/
//! `run_graph_offscreen`/`run_rhi_graphics_offscreen` 等)**0-byte 语义保留**;本模块为
//! 独立组装的新执行脊柱,沿 U26/U27/U30/U31 审计模式登记 **U32**。
//!
//! ## descriptor 固定约定(文档级契约)
//!
//! 每 pass 至多一个 descriptor set(**set 0**),binding 号按固定区间分配(与
//! rurixc `infer_spirv_bindings_vk_native` 的「类内声明序」同律,shader 侧按此约定取号):
//!
//! - binding `[0..A)`:A 个 **acceleration structure**(`Bindings::accel_structs` 声明序;
//!   值 = session AS 表下标;G7.6 Wave B,仅 [`DeviceFrameSession`] 携带 AS 表时可用)。
//!   AS 在前的理由:rurixc 按形参声明序分配 SPIR-V binding,W3 ray query 三核
//!   (gi_probe/rtao/hard_shadow)的 `tlas: AccelStruct` 均为**首形参**(binding 0),
//!   其后才是 SSBO/image 形参——故 AS 段占据 binding 最前区间与其声明序一致;
//! - binding `[A..A+N)`:N 个 **storage buffer**(`Bindings::storage_buffers` 声明序);
//! - binding `[A+N..A+N+M)`:M 个 **sampled image**(`Bindings::sampled_images` 声明序;
//!   `COMBINED_IMAGE_SAMPLER`,sampler 为执行器内建唯一**线性 sampler**:min/mag LINEAR、
//!   mipmap NEAREST、address CLAMP_TO_EDGE(阴影/GI 采样需要)、lod [0,1]、无各项异性);
//! - binding `[A+N+M..A+N+M+K)`:K 个 **storage image**(`Bindings::storage_images` 声明序,
//!   layout `GENERAL`);
//! - binding `[A+N+M+K]`:可选 1 个 **uniform buffer**(`Bindings::uniform`,offset+size)。
//!
//! **A=0 时(无 AS 绑定的既有 pass)binding 编号与 G5 起五段布局逐位不变**(0-byte 兼容)。
//!
//! push constants:单块、offset 0、≤128B,stage flags 恒 `VERTEX|FRAGMENT|COMPUTE`。
//! stage flags(descriptor 与 push range)恒 `VERTEX|FRAGMENT|COMPUTE`(保守超集,单约定)。
//!
//! ## G7.6 Wave B:AS 绑定面与 FrameUpdate 每帧重录
//!
//! - [`DeviceFrameSession::new_with_accel_structs`] 携带 AS 表([`AccelStructDesc`];
//!   复用 vk.rs **同一单所有者** `VkAsManager` 建 BLAS/TLAS,禁第二套 BVH),设备创建
//!   追加 `VK_KHR_ray_query`/`VK_KHR_acceleration_structure`/`VK_KHR_buffer_device_address`/
//!   `VK_KHR_deferred_host_operations` 四扩展 + feature 链(缺一 → 确定性 `Err`,不降级);
//!   初始 build 录进创建期专用一次性 cmd(单 fence 有界等待,非 `vkQueueWaitIdle`);
//!   AS 全量分配登记 session allocation ledger,AS 表项得 `StableResourceId`
//!   (= `resource_count + as_index + 1`),ray query pass 经 `accel_structs` 绑定声明
//!   对 TLAS 的 Read provenance。
//! - [`DeviceFrameSession::execute_with_frame_update`] 消费数据驱动 [`FrameUpdate`]:
//!   TLAS refit/rebuild(记 Write 并 bump generation)、buffer 上传(host-visible+coherent
//!   故 submit 前 memcpy;本执行器 buffer 恒 host-visible,staging 路不可达)、binding 覆盖
//!   (history ping-pong;布局键须与声明逐位一致)、push constant 覆盖(长度须等)、
//!   readback 子集;provenance 覆核仍在 `vkQueueSubmit` 前 fail-closed。
//!
//! ## 屏障计划语义
//!
//! - `barriers[i]` = 第 i 个 pass 录制**前**逐条回放的 (资源下标, [`TargetState`]) 转换表,
//!   由调用方图编译器提供;执行器**逐字回放、不重排**。
//! - 资源状态跟踪初值:buffer 带初始数据 → `HOST/HOST_WRITE`;buffer 无数据 → `NONE`;
//!   image → `UNDEFINED`(带初始数据的 image 经 staging 上传后 = `TRANSFER_DST`)。
//! - **隐式补全规则**(确定性、文档化):plan 回放后,pass 实际使用要求的态与跟踪态不一致
//!   时自动补一条转换(attachment→对应 attachment 态;sampled→`ShaderRead`;storage
//!   image→`StorageImageReadWrite`;storage buffer→保守 `StorageReadWrite` 读写超集;
//!   uniform→`UniformRead`;vertex buffer→`VertexInput`;indirect→`IndirectRead`)。完整
//!   plan 与空 plan 在同一单 queue 全序执行下产出**相同**命令序列语义——plan 的价值在
//!   调用方显式见证与跨 pass 复用控制,正确性不由 plan 完备性支撑。
//! - readback 为**终端胶水**(沿 v1/v2 先例):image readback 自动迁至 `TRANSFER_SRC` 后
//!   `vkCmdCopyImageToBuffer`;buffer 经 host-visible+coherent 内存 `vkQueueWaitIdle` 后
//!   直接 map(免 flush,同 `run_compute` 先例)。
//!
//! ## 错误口径(P-01 fail-closed,镜像 RXS-0193)
//!
//! Vulkan loader 不可用 / 无物理设备 / 无 graphics queue / `VK_KHR_synchronization2`
//! 缺失 / 资源或 plan 非法 / pipeline 创建失败 → **确定性 `Err`(不 panic、无静默
//! fallback、不 fake pass)**。`probe_device_caps` 暴露 `VK_KHR_shader_atomic_int64`
//! (`shaderBufferInt64Atomics`)探测面,作为 VisBuffer SW u64 内核的 W2 波次 fail-closed
//! 门禁;扩展存在时 device 启用该 feature。依 RFC-0016 §4.0-2,执行面不作静默降级,
//! 降级只允许 CI/demo 编排层显式选择并写入 evidence。
//!
//! # SAFETY(U32,Rust 级多 pass 图形执行 FFI 边界;沿 U26/U27/U31 审计模式)
//! 对上全 safe(无 `unsafe` 签名)。内部 `probe_caps_inner`/`execute_frame_inner` 全程手写
//! Vulkan FFI:`vulkan-1.dll`/`libvulkan.so` 经 vk.rs `load_vulkan_loader`(U26 同一 loader,
//! pub(crate) 复用)动态装载,缺失 → `Err` 非 panic;每个 `#[repr(C)]` VkStruct 与 Vulkan
//! spec 逐字节对齐(布局锚定单测 `ffi_layout_anchors` + `VK_LAYER_KHRONOS_validation`
//! `RURIX_VK_VALIDATION=1` 真跑 messenger fail-closed 双证);句柄(instance/device/buffer/
//! image/view/sampler/shaderModule/descriptorSetLayout/pipelineLayout/pipeline/renderPass/
//! framebuffer/descriptorPool/commandPool/staging+readback buffer)经 `Cleanup` 登记表单点
//! **逆序销毁**,早退路径同样走完销毁序,无泄漏/双释放;host-visible+coherent 内存免 flush;
//! 单 graphics queue 一次提交 + `vkQueueWaitIdle` 后回读(无数据竞争);messenger 契约同
//! U27(`p_user_data` 指向栈上 `AtomicBool`,生命周期严格短于该栈变量,ERROR 级校验翻
//! `Err`)。gate feature `vulkan` 默认关闭,CUDA 路零回归。

#![allow(non_snake_case, non_upper_case_globals, unsafe_op_in_unsafe_fn)]

use core::ffi::{CStr, c_char, c_void};
use std::collections::HashMap;

use crate::vk::{
    FnGetInstanceProcAddr, PfnVoid, VkAsFns, VkAsManager, WriteDescriptorSetAccelStructure,
    cast_fn, load_vulkan_loader,
};

// G7.6 Wave B:AS 绑定面/每帧重录公共类型复用 vk.rs 单一事实源(禁第二份定义)。
pub use crate::vk::{
    RayQueryInstanceDesc, RayQuerySceneDesc, RayQueryTransformedInstanceDesc, TlasBuildAction,
};

// ─────────────────────────── 公共 API:格式与资源 ───────────────────────────

/// texture2d 像素格式(G5 常见集;Vulkan format 枚举值经 SDK 头核对)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TexFormat {
    /// `VK_FORMAT_R8G8B8A8_UNORM`(37)。
    Rgba8Unorm,
    /// `VK_FORMAT_R16G16B16A16_SFLOAT`(97)。
    Rgba16Float,
    /// `VK_FORMAT_R32_UINT`(98;VisBuffer 32 位降级路 / 计数器)。
    R32Uint,
    /// `VK_FORMAT_R32G32_UINT`(101;VisBuffer 64 位 atomicMax u64 载体)。
    Rg32Uint,
    /// `VK_FORMAT_R32_SFLOAT`(100;G14.10b vendor 输入驻留:depth 单通道 f32
    /// 直通格式——D32 不可作 storage image 写,R32F 可 compute 直写)。
    R32Float,
    /// `VK_FORMAT_R32G32_SFLOAT`(103;G14.10b:motion vector RG f32 直通,
    /// 与 vendor(DLSS)mv 输入格式同一;storage 写需
    /// `shaderStorageImageExtendedFormats`)。
    Rg32Float,
    /// `VK_FORMAT_R32G32B32A32_SFLOAT`(109;G14.10b:color RGBA f32 直通候选
    /// ——SL color tag 格式容忍度探明面)。
    Rgba32Float,
    /// `VK_FORMAT_D32_SFLOAT`(126)。
    Depth32Float,
}

impl TexFormat {
    /// Vulkan `VkFormat` 枚举值(SDK 1.3.296 `vulkan_core.h` 核对)。
    #[must_use]
    pub fn vk_format(self) -> u32 {
        match self {
            TexFormat::Rgba8Unorm => 37,
            TexFormat::Rgba16Float => 97,
            TexFormat::R32Uint => 98,
            TexFormat::R32Float => 100,
            TexFormat::Rg32Uint => 101,
            TexFormat::Rg32Float => 103,
            TexFormat::Rgba32Float => 109,
            TexFormat::Depth32Float => 126,
        }
    }

    /// 每纹素字节数(readback/上传尺寸推导)。
    #[must_use]
    pub fn bytes_per_texel(self) -> usize {
        match self {
            TexFormat::Rgba8Unorm
            | TexFormat::R32Uint
            | TexFormat::R32Float
            | TexFormat::Depth32Float => 4,
            TexFormat::Rgba16Float | TexFormat::Rg32Uint | TexFormat::Rg32Float => 8,
            TexFormat::Rgba32Float => 16,
        }
    }

    /// 是否 depth 格式(决定 aspect 与 attachment 类别)。
    #[must_use]
    pub fn is_depth(self) -> bool {
        matches!(self, TexFormat::Depth32Float)
    }

    /// image aspect 掩码(`IMAGE_ASPECT_COLOR`=0x1 / `DEPTH`=0x2)。
    #[must_use]
    pub fn aspect_mask(self) -> u32 {
        if self.is_depth() {
            0x2
        } else {
            IMAGE_ASPECT_COLOR
        }
    }
}

/// buffer 用途位(建面 `VkBufferUsageFlags` 来源;按位或)。
#[derive(Debug, Clone, Copy, Default)]
pub struct BufferUsage {
    /// storage shader 读写(SSBO 绑定)。
    pub storage: bool,
    /// uniform 绑定。
    pub uniform: bool,
    /// vertex buffer 绑定。
    pub vertex: bool,
    /// indirect draw/dispatch 参数载体。
    pub indirect: bool,
}

/// texture2d 用途位(建面 `VkImageUsageFlags` 来源;按位或)。
#[derive(Debug, Clone, Copy, Default)]
pub struct TextureUsage {
    /// sampled image 绑定(线性 sampler 采样 / fetch)。
    pub sampled: bool,
    /// storage image 绑定(shader 直写,GENERAL)。
    pub storage: bool,
    /// color attachment。
    pub color: bool,
    /// depth attachment(须 `TexFormat::Depth32Float`)。
    pub depth: bool,
}

/// buffer 资源描述。内存驻留按 `device_local` 分路(G14.10d,RFC-0030 §4.3):
/// `false` = host-visible+coherent(既有态;免 flush,初始数据创建期 map 上传,
/// `FrameUpdate::buffer_uploads` 逐帧覆盖写的小参数 buffer 用之);`true` =
/// DEVICE_LOCAL 独占 VRAM(GPU 全速;不可 map——初始数据经 one-shot staging copy
/// 上传,被 readback 的输出经帧尾 copy 到 session 级 cached staging 回读;内存型
/// 只改驻留位置不改数据内容,copy 字节精确 → 位级零漂移)。
#[derive(Debug, Clone)]
pub struct BufferDesc<'a> {
    /// 字节数(≥4;0 长 buffer 无意义且 VUID 拒)。
    pub size: u64,
    /// 用途位。
    pub usage: BufferUsage,
    /// 可选初始数据(长度 ≤ `size`;host 路创建期 map 上传,device-local 路
    /// staging copy 上传)。
    pub data: Option<&'a [u8]>,
    /// `true` = DEVICE_LOCAL 驻留(GPU 独占 VRAM;上传目标 buffer 禁用——
    /// `FrameUpdate::buffer_uploads` 目标须 host-visible,校验期 fail-closed)。
    pub device_local: bool,
}

/// texture2d 资源描述(device-local optimal tiling;初始数据经 staging 上传)。
#[derive(Debug, Clone)]
pub struct TextureDesc<'a> {
    /// 宽(≥1)。
    pub width: u32,
    /// 高(≥1)。
    pub height: u32,
    /// 像素格式。
    pub format: TexFormat,
    /// 用途位。
    pub usage: TextureUsage,
    /// 可选初始数据(逐纹素紧凑字节,长度须 = `width*height*bytes_per_texel`)。
    pub data: Option<&'a [u8]>,
}

/// 帧资源描述(buffer 或 texture2d;下标即资源号,pass/屏障/readback 均按下标引用)。
#[derive(Debug, Clone)]
pub enum ResourceDesc<'a> {
    /// buffer 资源。
    Buffer(BufferDesc<'a>),
    /// texture2d 资源。
    Texture(TextureDesc<'a>),
}

// ─────────────────────────── 公共 API:屏障计划 ───────────────────────────

/// 资源目标状态(简化枚举;执行器经 `state_fields` 单源映射到
/// (layout, stage2, access2)——`VK_KHR_synchronization2` 64 位掩码)。
///
/// 类别纪律(校验期确定性拒):image 态 = `ColorAttachmentWrite`/`DepthAttachmentWrite`/
/// `StorageImageReadWrite`;buffer 态 = `StorageWrite`/`StorageReadWrite`/`UniformRead`/
/// `VertexInput`/`IndirectRead`;`ShaderRead`/`TransferSrc`/`TransferDst` 两类通用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetState {
    /// image:color attachment 写(`COLOR_ATTACHMENT_OPTIMAL`)。
    ColorAttachmentWrite,
    /// image:depth attachment 写(`DEPTH_STENCIL_ATTACHMENT_OPTIMAL`)。
    DepthAttachmentWrite,
    /// image:shader 采样读(`SHADER_READ_ONLY_OPTIMAL`);buffer:storage 读。
    ShaderRead,
    /// image:storage 读写(`GENERAL`)。
    StorageImageReadWrite,
    /// 传输源(image `TRANSFER_SRC_OPTIMAL`;buffer `TRANSFER_READ`)。
    TransferSrc,
    /// 传输目的(image `TRANSFER_DST_OPTIMAL`;buffer `TRANSFER_WRITE`)。
    TransferDst,
    /// buffer:storage 写。
    StorageWrite,
    /// buffer:storage 读写(绑定 storage buffer 的保守默认超集)。
    StorageReadWrite,
    /// buffer:uniform 读。
    UniformRead,
    /// buffer:vertex attribute 读。
    VertexInput,
    /// buffer:indirect 参数读。
    IndirectRead,
}

// ─────────────────────────── 公共 API:pass 描述 ───────────────────────────

/// raster pass 顶点数据来源。
#[derive(Debug, Clone)]
pub enum VertexData<'a> {
    /// 内联交错顶点字节(执行器建临时 host-visible vertex buffer 上传;G5 静态几何/全屏
    /// 三角形路径)。`attrs` = `(location, format, offset)`(单 binding 0,镜像 vk.rs
    /// 既有顶点面);`stride` = 每顶点字节(≥1)。
    Inline {
        /// 交错顶点字节(长度须为 `stride` 整倍)。
        data: &'a [u8],
        /// 每顶点字节步长。
        stride: u32,
        /// `(location, format, offset)` 顶点属性。
        attrs: &'a [(u32, u32, u32)],
    },
    /// 设备 buffer 资源作 vertex buffer(`usage.vertex` 须真;GPU 生成几何路径)。
    Resource {
        /// 资源下标(buffer 类)。
        res: u32,
        /// 首顶点字节偏移。
        offset: u64,
        /// 每顶点字节步长。
        stride: u32,
        /// `(location, format, offset)` 顶点属性。
        attrs: &'a [(u32, u32, u32)],
    },
    /// vertex-pull(无 VB;shader 经 `gl_VertexIndex`/storage buffer 自取)。
    Pull,
}

/// raster pass draw 参数。
#[derive(Debug, Clone, Copy)]
pub enum DrawSpec {
    /// `vkCmdDraw(vertex_count, instance_count, first_vertex, first_instance)`。
    Direct {
        /// 顶点数。
        vertex_count: u32,
        /// 实例数(≥1)。
        instance_count: u32,
        /// 首顶点下标。
        first_vertex: u32,
        /// 首实例下标。
        first_instance: u32,
    },
    /// `vkCmdDrawIndirect(buffer, offset, 1, 16)`(`VkDrawIndirectCommand` 单条;
    /// `usage.indirect` 须真)。
    Indirect {
        /// 资源下标(buffer 类)。
        res: u32,
        /// 命令字节偏移。
        offset: u64,
    },
}

/// compute pass dispatch 参数。
#[derive(Debug, Clone, Copy)]
pub enum DispatchSpec {
    /// `vkCmdDispatch(x, y, z)`(各维 ≥1)。
    Direct([u32; 3]),
    /// `vkCmdDispatchIndirect(buffer, offset)`(`usage.indirect` 须真)。
    Indirect {
        /// 资源下标(buffer 类)。
        res: u32,
        /// 命令字节偏移。
        offset: u64,
    },
}

/// color attachment 引用(attachment 序 = 声明序 = render pass attachment 下标)。
#[derive(Debug, Clone, Copy)]
pub struct ColorAttachmentRef {
    /// 资源下标(texture 类,`usage.color` 须真,非 depth 格式)。
    pub res: u32,
    /// `Some(rgba)` → loadOp CLEAR(清屏色);`None` → LOAD(保留既有内容)。
    pub clear: Option<[f32; 4]>,
}

/// depth attachment 引用(≤1 个)。
#[derive(Debug, Clone, Copy)]
pub struct DepthAttachmentRef {
    /// 资源下标(texture 类,`usage.depth` 须真,`TexFormat::Depth32Float`)。
    pub res: u32,
    /// `Some(d)` → loadOp CLEAR(清深值,通常 1.0);`None` → LOAD。
    pub clear: Option<f32>,
}

/// uniform buffer 绑定引用(set0 固定约定末槽)。
#[derive(Debug, Clone, Copy)]
pub struct UniformRef {
    /// 资源下标(buffer 类,`usage.uniform` 须真)。
    pub res: u32,
    /// 绑定起始字节偏移。
    pub offset: u64,
    /// 绑定字节范围(>0;`offset+size` ≤ buffer `size`)。
    pub size: u64,
}

/// pass 绑定集(set0 固定约定各区段来源;全空 = 无 descriptor 绑定)。
#[derive(Debug, Clone, Default)]
pub struct Bindings {
    /// acceleration structure 绑定列(**session AS 表下标** → binding `[0..A)`;ray query
    /// compute pass 的 TLAS 只读绑定;仅 [`DeviceFrameSession::new_with_accel_structs`]
    /// 携带 AS 表的 session 合法,`execute_frame`/空 AS 表 session 须为空——校验期
    /// 确定性拒)。A=0 时其余四段 binding 编号与 G5 布局逐位不变。
    pub accel_structs: Vec<u32>,
    /// storage buffer 资源下标列(声明序 → binding `[A..A+N)`;compute/raster 均可读写,
    /// 屏障按保守读写超集计)。
    pub storage_buffers: Vec<u32>,
    /// sampled image 资源下标列(声明序 → binding `[A+N..A+N+M)`;内建线性 sampler)。
    pub sampled_images: Vec<u32>,
    /// storage image 资源下标列(声明序 → binding `[A+N+M..A+N+M+K)`;`GENERAL`)。
    pub storage_images: Vec<u32>,
    /// 可选 uniform 绑定(→ binding `[A+N+M+K]`)。
    pub uniform: Option<UniformRef>,
    /// push constants 字节(≤128;offset 0 单块;空 = 不推)。
    pub push_constants: Vec<u8>,
}

/// 持久 session 加速结构表项(G7.6 Wave B;BLAS 三角形组 + TLAS 实例描述,
/// 复用 vk.rs [`RayQuerySceneDesc`] 既有类型——经同一单所有者 `VkAsManager` 建面,
/// 禁第二套 BVH/AS 所有者)。
pub struct AccelStructDesc<'a> {
    /// 场景描述(逐 BLAS 世界空间三角形 + TLAS 实例;口径同 vk.rs RayQuery 面)。
    pub scene: RayQuerySceneDesc<'a>,
    /// 可选逐实例显式行主 3×4 transform(`None` = 全 identity;`Some` 时长度须
    /// = `scene.instances.len()`)。
    pub transforms: Option<&'a [[f32; 12]]>,
    /// G31+ 波 B Task B5:顶点可更新 BLAS 下标表(逐帧 `blas_refit` 的创建期
    /// 打标面;透传 `VkAsManager::create_scene_ex`——表内 BLAS 带
    /// ALLOW_UPDATE + scratch 双尺寸上界 + vbuf TRANSFER_DST)。`&[]` = 全
    /// 静态(既有面 0-byte,flags=0 基线不变)。
    pub updatable_blas: &'a [u32],
}

/// G31+ 波 B Task B5:逐帧 BLAS 顶点 refit 描述(蒙皮/WPO 形变通路;
/// [`FrameUpdate::blas_refit`] 载荷)。
///
/// 语义:本帧在 pass `after_pass` 录完后——① `vkCmdCopyBuffer` 把
/// `src`(session buffer 资源,蒙皮 compute pass 的蒙皮后顶点输出,布局须与
/// 目标 BLAS 顶点缓冲逐字节同形:9 f32/三角形汤)经 `src_offset` 起
/// `byte_len` 字节桥接进 AS `as_index` 的 `blas_index` 号 BLAS 顶点缓冲;
/// ② 原地 `UPDATE` 模式 BLAS build(单所有者 `VkAsManager::
/// record_blas_refit`;目标须创建期 `create_scene_ex` updatable 打标,否则
/// 确定性 `Err`);③ consume barrier(AS_WRITE→AS_READ,COMPUTE_SHADER
/// dst——`record_tlas_update` 同律)。全链 GPU 内零 host 回读;顶点数/拓扑
/// 不变(refit 合法域),实例变换/TLAS 引用不动。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlasRefitUpdate {
    /// session AS 表下标(与 `tlas_update` 同域;同帧两者同现时须同值)。
    pub as_index: u32,
    /// 目标 BLAS 下标(`RayQuerySceneDesc::blas_triangles` 序)。
    pub blas_index: u32,
    /// 蒙皮后顶点源(session buffer 资源;本帧已被前序 pass 写入)。
    pub src: StableResourceId,
    /// 源字节偏移(4 对齐;拷贝区段须落在 src buffer 界内)。
    pub src_offset: u64,
    /// 拷贝字节数(= BLAS 顶点缓冲字节数;非 0,4 对齐)。
    pub byte_len: u64,
    /// 在该 pass 录完后插入桥接 + UPDATE build(蒙皮 compute pass 下标)。
    pub after_pass: u32,
}

/// G38 T3 加性:BLAS refit 桥扩展(多 region 脏区段 copy + 桥接 GPU 计时;
/// [`DeviceFrameSession::execute_with_frame_update_bridge_ext`] 载荷)。
/// 既有入口不感知本结构——`None`/缺省 = 既有单 region 全量桥逐字不变。
///
/// 语义:
/// - `copy_regions = None`:桥接 copy 为既有单 region
///   `[src_offset, +byte_len) → [0, +byte_len)`(命令流逐字节不变)。
/// - `copy_regions = Some(rs)`:一次 `vkCmdCopyBuffer` 带 region 数组,每段
///   `(off, len)` 拷 `src[src_offset+off ..] → vbuf[off ..]`(src 与 vbuf 同
///   布局的脏差集;须升序不重叠、4 对齐、落在 `[0, byte_len)` 内——
///   fail-closed 校验)。**空列表合法** = 本帧无脏字节,跳过桥接 copy 三步
///   (屏障对 + copy),UPDATE build 照录(vbuf 已与 src 同步,digest 与全量
///   copy 位级等价)。
/// - `collect_gpu_timing = true`:桥接段首/copy 后/build 后各写一个 GPU
///   timestamp(query pool 追加区,**不动既有逐 pass 时戳口径**),结果经
///   [`DeviceFrameTelemetry::blas_bridge_copy_gpu_ms`]/
///   [`DeviceFrameTelemetry::blas_bridge_build_gpu_ms`] 返回;query 读取失败
///   fail-soft `None`(不冒充数值)。`blas_refit` 为 `None` 时本旗标静默无效
///   (无桥可计时,两字段恒 `None`)。
#[derive(Debug, Clone, Default)]
pub struct BlasRefitBridgeExt {
    /// 脏区段列表((offset,len) 相对 refit 窗;`None` = 既有单 region 全量)。
    pub copy_regions: Option<Vec<(u64, u64)>>,
    /// 采集桥接段 GPU 计时(copy/build 两段分解)。
    pub collect_gpu_timing: bool,
}

/// 每帧重录更新描述(G7.6 Wave B;**数据驱动、不用闭包**,provenance 可机验;
/// [`DeviceFrameSession::execute_with_frame_update`] 消费)。
///
/// 字段语义:
/// - `tlas_update`:`Some((as 表下标, 实例 transforms, 动作))` → 本帧先 host 写
///   instance buffer,再在重录 cmd 首段录 `BUILD`(Rebuild)/`UPDATE`(Refit)+
///   consume barrier;AS provenance 记 Write 并 bump generation(本帧 ray query pass
///   的 Read producer 指向新 generation)。
/// - `buffer_uploads`:`(StableResourceId, offset, bytes)` 逐段 submit 前 host memcpy
///   (本执行器 buffer 恒 host-visible+coherent,`create_device_buffer` 单内存路径,
///   故无 staging 分支);上传记 Write 并 bump 该资源 generation。仅 buffer 资源合法。
/// - `binding_overrides`:`(pass 下标, Bindings)` 本帧该 pass 的绑定覆盖(history
///   ping-pong);**set0 布局键(AS/storage/sampled/storage-image 计数 + uniform 有无)
///   与 push constants 长度须与声明逐位一致**(pipeline layout 创建期固化),不一致 →
///   确定性 `Err`;覆盖 pass 的 descriptor set 在 submit 前重写(前帧 fence 已等待,
///   set 不在途)。
/// - `push_constant_overrides`:`(pass 下标, bytes)` 在(可能覆盖后的)绑定之上再覆盖
///   push constants;长度须等于声明长度。
/// - `readback_subset`:`Some(下标列)` = 本帧仅执行/readback 这些 session readback
///   请求(`DeviceFrameOutput::readbacks[k]` 对应 `readbacks[subset[k]]`);
///   `None` = 本帧不 readback(输出 readbacks 为空)。
#[derive(Debug, Clone, Default)]
pub struct FrameUpdate {
    /// 见上;`as 表下标` 越界 / 实例数不符 → 确定性 `Err`。
    pub tlas_update: Option<(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
    /// 见上;非 buffer 资源 / 越界区段 / 空段 → 确定性 `Err`。
    pub buffer_uploads: Vec<(StableResourceId, u64, Vec<u8>)>,
    /// 见上;布局键或 push 长度漂移 / pass 下标越界 / 重复下标 → 确定性 `Err`。
    pub binding_overrides: Vec<(u32, Bindings)>,
    /// 见上;长度不等 / pass 下标越界 / 重复下标 → 确定性 `Err`。
    pub push_constant_overrides: Vec<(u32, Vec<u8>)>,
    /// 见上;下标越界 / 重复 → 确定性 `Err`。
    pub readback_subset: Option<Vec<u32>>,
    /// G31+ 波 B Task B5:逐帧 BLAS 顶点 refit(蒙皮通路;语义见
    /// [`BlasRefitUpdate`])。`None` = 无 BLAS 更新(既有面 0-byte);
    /// FIF 流水面不支持(BLAS 顶点缓冲为共享写面,在飞帧 ray query 读取中
    /// 不可改写——`tlas_update` 同律拒)。
    pub blas_refit: Option<BlasRefitUpdate>,
}

/// raster pass(vertex+fragment 着色对;`OpEntryPoint` 名恒 `"main"`,沿 vk.rs 先例)。
#[derive(Debug, Clone)]
pub struct RasterPass<'a> {
    /// pass 诊断名(错误消息定位)。
    pub name: &'a str,
    /// vertex SPIR-V 字节(4 字节对齐,magic `0x07230203`)。
    pub vs_spirv: &'a [u8],
    /// fragment SPIR-V 字节。
    pub fs_spirv: &'a [u8],
    /// 顶点数据来源(Inline / Resource / Pull)。
    pub vertex: VertexData<'a>,
    /// draw 参数(Direct / Indirect)。
    pub draw: DrawSpec,
    /// color attachment 列(attachment 序 = render pass 下标序;全 attachments 须同尺寸)。
    pub colors: Vec<ColorAttachmentRef>,
    /// 可选 depth attachment(depth test+write 开,compare `LESS_OR_EQUAL`)。
    pub depth: Option<DepthAttachmentRef>,
    /// 视口 `(w, h)`;`None` = attachment 尺寸。scissor 恒全幅。
    pub viewport: Option<(u32, u32)>,
    /// 绑定集(set0 固定约定)。
    pub bindings: Bindings,
    /// 保守光栅化(G7.5b,RXS-0303 L2):`Some` = pipeline 光栅状态 `pNext` 链
    /// `VkPipelineRasterizationConservativeStateCreateInfoEXT{ OVERESTIMATE }`;
    /// `None` = 既有默认光栅行为(0-byte)。设备无 `VK_EXT_conservative_rasterization`
    /// 而 pass 要求 `Some` → 确定性 `Err`(RXS-0303 L3 fail-closed,不静默降级)。
    pub conservative: Option<ConservativeRasterDesc>,
}

/// 保守光栅 pass 描述(RXS-0303 L2;模式钉死 OVERESTIMATE,不暴露 UNDERESTIMATE/
/// DISABLED——覆盖超集是本面的唯一语义)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConservativeRasterDesc {
    /// `extraPrimitiveOverestimationSize`(附加过估尺寸,像素;常态 0.0,取值须在
    /// [`ConservativeRasterProps::max_extra_primitive_overestimation_size`] 内且按
    /// granularity 对齐——驱动侧钳制,本执行器原样透传)。
    pub extra_overestimation: f32,
}

/// compute pass。
#[derive(Debug, Clone)]
pub struct ComputePass<'a> {
    /// pass 诊断名。
    pub name: &'a str,
    /// compute SPIR-V 字节。
    pub spirv: &'a [u8],
    /// 入口名;`None` = 自 `OpEntryPoint` 解析(复用 vk.rs `entry_point_name`)。
    pub entry: Option<&'a str>,
    /// dispatch 参数(Direct xyz / Indirect)。
    pub dispatch: DispatchSpec,
    /// 绑定集(set0 固定约定)。
    pub bindings: Bindings,
}

/// 帧 pass(raster/compute 混合,声明序 = 录制序 = 单 queue 执行序)。
#[derive(Debug, Clone)]
pub enum Pass<'a> {
    /// raster(vertex+fragment)。
    Raster(RasterPass<'a>),
    /// compute。
    Compute(ComputePass<'a>),
}

// ─────────────────────────── 公共 API:readback / 能力 / 执行 ───────────────────────────

/// readback 请求(返回字节与请求列一一对应)。
#[derive(Debug, Clone, Copy)]
pub enum Readback {
    /// buffer 区段(host-visible+coherent,`vkQueueWaitIdle` 后直接 map 拷出)。
    Buffer {
        /// 资源下标(buffer 类)。
        res: u32,
        /// 起始字节偏移。
        offset: u64,
        /// 字节数(>0;`offset+size` ≤ buffer `size`)。
        size: u64,
    },
    /// texture 全幅逐纹素紧凑字节(自动迁 `TRANSFER_SRC` 后
    /// `vkCmdCopyImageToBuffer`;`width*height*bytes_per_texel`)。
    Texture {
        /// 资源下标(texture 类)。
        res: u32,
    },
}

/// 设备能力面(`probe_device_caps` 产物;VisBuffer 能力分级与 GI/阴影 feature 裁决输入)。
#[derive(Debug, Clone)]
pub struct DeviceCaps {
    /// 物理设备名(`VkPhysicalDeviceProperties::deviceName`)。
    pub device_name: String,
    /// `VK_KHR_synchronization2` 扩展 + `synchronization2` feature 均可用
    /// (`execute_frame` 硬依赖;缺失 → 确定性 `Err`)。
    pub synchronization2: bool,
    /// `VK_KHR_shader_atomic_int64` 扩展 + `shaderBufferInt64Atomics` feature 均可用
    /// (VisBuffer SW 光栅 atomicMax u64 路径分级;`Rg32Uint` storage image / u64 SSBO)。
    pub shader_buffer_int64_atomics: bool,
    /// 核心 `shaderInt64` feature；u64 SSBO 标量读取/位运算所需。
    pub shader_int64: bool,
    /// 核心 `fragmentStoresAndAtomics` feature(G7.5b,RXS-0303:fragment stage 写
    /// SSBO/原子的 Vulkan 前提〔VUID-RuntimeSpirv-NonWritable-06340〕;HW 光栅
    /// VisBuffer 腿 FS `OpAtomicUMax` 依赖,探测到即启用)。
    pub fragment_stores_and_atomics: bool,
    /// `VK_KHR_ray_query` 扩展 + `rayQuery` feature。
    pub ray_query: bool,
    /// `VK_KHR_acceleration_structure` 扩展 + `accelerationStructure` feature。
    pub acceleration_structure: bool,
    /// `VK_KHR_buffer_device_address` 扩展或 Vulkan 1.2 core + `bufferDeviceAddress` feature。
    pub buffer_device_address: bool,
    /// `VK_EXT_descriptor_indexing` 扩展 + `runtimeDescriptorArray` feature。
    pub descriptor_indexing: bool,
    /// `VK_KHR_deferred_host_operations` 扩展存在。
    pub deferred_host_operations: bool,
    /// `VK_EXT_memory_budget` 驱动 heap budget/usage 查询面。
    pub memory_budget: bool,
    /// `VK_EXT_conservative_rasterization` 在位时的属性快照(G7.5b,RXS-0303 IR1;
    /// `vkGetPhysicalDeviceProperties2` 链 `VkPhysicalDeviceConservativeRasterizationPropertiesEXT`
    /// 运行时实采,非公开数据库口径);`None` = 扩展不在位(HW 光栅腿 fail-closed 依据)。
    pub conservative_raster: Option<ConservativeRasterProps>,
    /// `VkPhysicalDeviceLimits::timestampPeriod`（ns/tick，驱动实值）。
    pub timestamp_period_ns: f32,
    /// `maxPushConstantsSize`(Vulkan 保底 128;本执行器约定 ≤128)。
    pub max_push_constants_size: u32,
}

/// `VkPhysicalDeviceConservativeRasterizationPropertiesEXT` 快照四项(RXS-0303 IR1
/// 逐字:`primitive_overestimation_size / max_extra / granularity /
/// degenerate_triangles_rasterized`,全量进证据;RFC-0018 §E2 本机探测事实的运行时重采源)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConservativeRasterProps {
    /// `primitiveOverestimationSize`(驱动固有过估尺寸,像素;本机 RTX 4070 Ti 实测
    /// 0.00195312 = 1/512 px——过估仅使 HW fragment 集更大,超集论证不受影响)。
    pub primitive_overestimation_size: f32,
    /// `maxExtraPrimitiveOverestimationSize`([`ConservativeRasterDesc::extra_overestimation`]
    /// 的合法上界)。
    pub max_extra_primitive_overestimation_size: f32,
    /// `extraPrimitiveOverestimationSizeGranularity`(附加过估的取值粒度)。
    pub extra_primitive_overestimation_size_granularity: f32,
    /// `degenerateTrianglesRasterized`(量化后零面积 sliver 三角形是否仍光栅化;
    /// `true` 时细长三角形超集性质额外加固,RFC-0018 §8.1 风险 #3 不触发)。
    pub degenerate_triangles_rasterized: bool,
}

/// 渲染效果内核设备化波次；高波次能力要求为累积集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelWave {
    /// 基线内核：仅依赖 synchronization2 执行脊柱。
    W1,
    /// u64 原子内核：W1 + shader buffer int64 atomics。
    W2,
    /// ray-query 内核：W2 + ray query 五件能力链。
    W3,
}

/// 波次能力门禁错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderExecError {
    /// 设备缺失目标波次所需能力；列表按声明顺序稳定输出。
    MissingCapabilities {
        /// 被拒绝的目标波次。
        wave: KernelWave,
        /// 缺失能力的稳定内部名。
        missing: Vec<&'static str>,
    },
}

impl std::fmt::Display for RenderExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderExecError::MissingCapabilities { wave, missing } => {
                write!(f, "内核波次 {wave:?} 缺失设备能力: {}", missing.join(", "))
            }
        }
    }
}

impl std::error::Error for RenderExecError {}

/// W1 基线能力声明。
pub const W1_REQUIRED_CAPABILITIES: &[&str] = &["synchronization2"];
/// W2 累积能力声明。
pub const W2_REQUIRED_CAPABILITIES: &[&str] = &["synchronization2", "shader_buffer_int64_atomics"];
/// W3 累积能力声明(ray query 五件链位于 W2 之后)。
pub const W3_REQUIRED_CAPABILITIES: &[&str] = &[
    "synchronization2",
    "shader_buffer_int64_atomics",
    "ray_query",
    "acceleration_structure",
    "buffer_device_address",
    "descriptor_indexing",
    "deferred_host_operations",
];

/// 返回波次所需能力名的声明序切片。
#[must_use]
pub const fn required_capabilities(wave: KernelWave) -> &'static [&'static str] {
    match wave {
        KernelWave::W1 => W1_REQUIRED_CAPABILITIES,
        KernelWave::W2 => W2_REQUIRED_CAPABILITIES,
        KernelWave::W3 => W3_REQUIRED_CAPABILITIES,
    }
}

fn has_capability(caps: &DeviceCaps, capability: &str) -> bool {
    match capability {
        "synchronization2" => caps.synchronization2,
        "shader_buffer_int64_atomics" => caps.shader_buffer_int64_atomics,
        "ray_query" => caps.ray_query,
        "acceleration_structure" => caps.acceleration_structure,
        "buffer_device_address" => caps.buffer_device_address,
        "descriptor_indexing" => caps.descriptor_indexing,
        "deferred_host_operations" => caps.deferred_host_operations,
        _ => false,
    }
}

/// 对目标波次执行 fail-closed 能力门禁。
pub fn require_wave(caps: &DeviceCaps, wave: KernelWave) -> Result<(), RenderExecError> {
    let missing = required_capabilities(wave)
        .iter()
        .copied()
        .filter(|name| !has_capability(caps, name))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(RenderExecError::MissingCapabilities { wave, missing })
    }
}

/// 内核名到设备化波次的声明式路由项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelWaveRoute {
    /// 稳定内核名。
    pub kernel: &'static str,
    /// 内核所需波次。
    pub wave: KernelWave,
}

/// RD-038 首批效果内核的波次路由表。
pub const KERNEL_WAVE_ROUTES: &[KernelWaveRoute] = &[
    KernelWaveRoute {
        kernel: "cull",
        wave: KernelWave::W1,
    },
    KernelWaveRoute {
        kernel: "classify_resolve",
        wave: KernelWave::W1,
    },
    KernelWaveRoute {
        kernel: "vsm_page_mark",
        wave: KernelWave::W1,
    },
    KernelWaveRoute {
        kernel: "taa",
        wave: KernelWave::W1,
    },
    KernelWaveRoute {
        kernel: "visbuffer_sw_u64",
        wave: KernelWave::W2,
    },
    KernelWaveRoute {
        kernel: "gi_probe",
        wave: KernelWave::W3,
    },
    KernelWaveRoute {
        kernel: "rtao",
        wave: KernelWave::W3,
    },
    KernelWaveRoute {
        kernel: "hard_shadow",
        wave: KernelWave::W3,
    },
];

/// 按稳定内核名查询设备化波次。
#[must_use]
pub fn kernel_wave(kernel: &str) -> Option<KernelWave> {
    KERNEL_WAVE_ROUTES
        .iter()
        .find(|route| route.kernel == kernel)
        .map(|route| route.wave)
}

/// 物理设备能力探测(仅建 instance,不建 device;轻量,可帧前/初始化期调用)。
///
/// 缺 Vulkan loader / 无物理设备 → 确定性 `Err`(镜像 RXS-0193 口径,不 fake)。
pub fn probe_device_caps() -> Result<DeviceCaps, String> {
    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;
    // SAFETY: 见模块头 U32 契约;句柄线性配对 create/destroy,末尾逆序销毁。
    unsafe { probe_caps_inner(gipa) }
}

/// 执行一帧:创建 device → 资源建面 → 逐 pass 录制(pipeline cache 内建)→ 屏障
/// `vkCmdPipelineBarrier2KHR` 逐字回放 + 隐式补全 → 单 queue 一次提交 → readback 返字节
/// (与 `readbacks` 一一对应)。
///
/// - `resources`:帧资源表(下标 = 资源号)。
/// - `passes`:pass 序列(≥1,声明序 = 执行序)。
/// - `barriers`:`barriers.len()` 须 = `passes.len()`;`barriers[i]` = 第 i 个 pass 录制前
///   逐字回放的 (资源下标, [`TargetState`]) 转换表(语义见模块头「屏障计划语义」)。
/// - `readbacks`:readback 请求列(终端胶水,见模块头)。
///
/// `VK_KHR_synchronization2` 不可用 → 确定性 `Err`(不降级到旧 barrier,口径钉死)。
pub fn execute_frame(
    resources: &[ResourceDesc],
    passes: &[Pass],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
) -> Result<Vec<Vec<u8>>, String> {
    // 纯 host 预校验(P-01 fail-closed,在任何句柄创建前)。
    validate_frame(resources, passes, barriers, readbacks)?;
    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;
    // SAFETY: 见模块头 U32 契约;句柄经 Cleanup 表单点逆序销毁,早退路径同走销毁序。
    unsafe { execute_frame_inner(gipa, resources, passes, barriers, readbacks) }
}

/// 跨帧稳定资源编号。编号由 [`DeviceFrameSession`] 创建时按资源表声明序分配，
/// session 生命周期内不复用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StableResourceId(pub u64);

/// 跨帧稳定 Vulkan 分配编号。编号对应一次真实 `vkAllocateMemory`，session 生命周期内
/// 不随 frame slot 或提交次数变化。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StableAllocationId(pub u64);

/// 一次真实 Vulkan allocation 的账本项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationLedgerEntry {
    /// 稳定 allocation ID。
    pub allocation_id: StableAllocationId,
    /// 主资源 ID；内部 staging/readback 分配为 `None`。
    pub resource_id: Option<StableResourceId>,
    /// 驱动 `VkMemoryRequirements::size` 裁定的实际分配字节。
    pub bytes: u64,
    /// `VkMemoryType::heapIndex`。
    pub heap_index: u32,
}

/// pass 对资源的真实用途（由绑定/attachment/draw 描述推导，不按 pass 名猜 hazard）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAccessKind {
    /// 只读消费。
    Read,
    /// 只写生产。
    Write,
    /// 保守读写（storage buffer/image）。
    ReadWrite,
}

/// 消费者所引用的生产 allocation/generation。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProducerVersion {
    /// 生产者实际 allocation。
    pub allocation_id: StableAllocationId,
    /// 该 allocation 的内容代次。
    pub generation: u64,
}

/// 一条 pass/resource 运行期 provenance。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeResourceProvenance {
    /// 稳定资源 ID。
    pub resource_id: StableResourceId,
    /// 本 pass 使用的真实 allocation。
    pub allocation_id: StableAllocationId,
    /// 读/写用途。
    pub access: RuntimeAccessKind,
    /// 读取时必须指向实际最近生产者；纯写为 `None`。
    pub producer: Option<ProducerVersion>,
    /// 本 pass 写后产生的 generation；纯读为 `None`。
    pub produced_generation: Option<u64>,
}

/// 单 pass 运行期 provenance。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePassProvenance {
    /// 声明序稳定 pass ID（从 1 起）。
    pub pass_id: u64,
    /// 诊断名，仅用于报告；hazard/验证不读取名字。
    pub name: String,
    /// 由资源绑定事实推导的逐资源 provenance。
    pub resources: Vec<RuntimeResourceProvenance>,
}

/// 一次提交的完整 provenance；可克隆后篡改供 RED 注入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionProvenance {
    /// session 内帧序号（从 1 起）。
    pub frame_generation: u64,
    /// pass 声明序 provenance。
    pub passes: Vec<RuntimePassProvenance>,
}

/// 提交前 fail-closed 校验：消费者必须引用执行器按资源/绑定事实推导出的实际
/// producer allocation/generation。供生产提交与 RED 注入共用。
pub fn validate_submission_provenance(
    expected: &SubmissionProvenance,
    supplied: &SubmissionProvenance,
) -> Result<(), String> {
    if supplied.frame_generation != expected.frame_generation {
        return Err(format!(
            "provenance frame generation {} != actual {}",
            supplied.frame_generation, expected.frame_generation
        ));
    }
    if supplied.passes.len() != expected.passes.len() {
        return Err(format!(
            "provenance pass count {} != actual {}",
            supplied.passes.len(),
            expected.passes.len()
        ));
    }
    for (pi, (want, got)) in expected.passes.iter().zip(&supplied.passes).enumerate() {
        if want.pass_id != got.pass_id || want.resources != got.resources {
            return Err(format!(
                "provenance pass[{pi}] 与 actual allocation/generation 不一致(fail-closed)"
            ));
        }
    }
    Ok(())
}

/// 每 pass 的真实 GPU timestamp 结果。
#[derive(Debug, Clone)]
pub struct PassGpuTiming {
    /// 稳定 pass ID。
    pub pass_id: u64,
    /// 诊断名。
    pub name: String,
    /// 驱动 timestamp tick 差乘 `timestampPeriod` 后的纳秒值。
    pub gpu_ns: f64,
}

/// `VK_EXT_memory_budget` 单 heap 快照。
#[derive(Debug, Clone)]
pub struct HeapBudgetTelemetry {
    /// heap 下标。
    pub heap_index: u32,
    /// 驱动预算字节。
    pub budget_bytes: u64,
    /// 驱动报告的 heap usage 字节。
    pub driver_usage_bytes: u64,
    /// 本 session allocation ledger 落在该 heap 的字节和。
    pub ledger_bytes: u64,
}

/// 一帧 mandatory real telemetry。字段均为实测值；session 创建时若 timestamp 或
/// `VK_EXT_memory_budget` 不可用则 fail-closed，不以 `null`/unavailable 冒充成功。
#[derive(Debug, Clone)]
pub struct DeviceFrameTelemetry {
    /// 设备 `timestampPeriod`（ns/tick）。
    pub timestamp_period_ns: f32,
    /// 逐 pass GPU 时间。
    pub passes: Vec<PassGpuTiming>,
    /// CPU 提交前校验/录制准备时间。
    pub cpu_record_ns: u64,
    /// `vkQueueSubmit` 调用耗时。
    pub cpu_submit_ns: u64,
    /// 有界 `vkWaitForFences` 耗时。
    pub cpu_fence_wait_ns: u64,
    /// 逐 heap memory-budget 实测。
    pub heaps: Vec<HeapBudgetTelemetry>,
    /// 全量真实 allocation ledger。
    pub allocations: Vec<AllocationLedgerEntry>,
    /// validation ERROR 累计数（session messenger 实数）。
    pub validation_error_count: u64,
    /// 本提交是否返回 `VK_ERROR_DEVICE_LOST`。
    pub device_lost: bool,
    /// 有界 fence 等待是否超时，提示 TDR/卡死嫌疑。
    pub tdr_suspected: bool,
    /// 当前仍由 session 正常持有的 Vulkan object 数（非 leak）。
    pub outstanding_object_count: u64,
    /// 当前仍由 session 正常持有的 allocation 数（非 leak）。
    pub outstanding_allocation_count: u64,
    /// 所有权账本外 object 数；成功帧必须为 0。
    pub leaked_object_count: u64,
    /// 所有权账本外 allocation 数；成功帧必须为 0。
    pub leaked_allocation_count: u64,
    /// G38 T3 加性:BLAS refit 桥接 copy 段 GPU 毫秒(桥首→copy 后;含桥内
    /// 屏障对)。仅 [`BlasRefitBridgeExt::collect_gpu_timing`] 开启且本帧带
    /// `blas_refit` 时 `Some`;query 读取失败 fail-soft `None`(不冒充数值)。
    pub blas_bridge_copy_gpu_ms: Option<f64>,
    /// G38 T3 加性:BLAS refit UPDATE build 段 GPU 毫秒(copy 后→consume
    /// barrier 后)。`Some`/`None` 判据同 `blas_bridge_copy_gpu_ms`。
    pub blas_bridge_build_gpu_ms: Option<f64>,
}

/// 持久 session 的一帧结果。
#[derive(Debug, Clone)]
pub struct DeviceFrameOutput {
    /// 与创建时 readback 请求一一对应。
    pub readbacks: Vec<Vec<u8>>,
    /// 本提交的实际 provenance。
    pub provenance: SubmissionProvenance,
    /// mandatory real telemetry。
    pub telemetry: DeviceFrameTelemetry,
}

/// G14plus vendor 域(RFC-0030)session Texture 资源原生 Vulkan 句柄簿记
/// (加性只读导出面;见 [`DeviceFrameSession::texture_native_handles`])。
/// 句柄归 session 所有——调用方不得销毁/越 session 生命周期持有;本 session
/// 与 vendor(DLSS)session 各持**独立** VkInstance/VkDevice,句柄跨 device
/// 不可直接消费(需 VK_KHR_external_memory 导出/导入改造,RFC-0030 裁决面)。
#[derive(Debug, Clone, Copy)]
pub struct NativeTextureHandles {
    /// VkImage(non-dispatchable u64)。
    pub image: u64,
    /// VkDeviceMemory。
    pub memory: u64,
    /// VkImageView。
    pub view: u64,
    pub width: u32,
    pub height: u32,
    /// VkFormat 数值([`TexFormat::vk_format`] 同源)。
    pub vk_format: u32,
}

/// G14plus vendor 域(RFC-0030)session Vulkan 顶层句柄簿记(加性只读导出面;
/// 见 [`DeviceFrameSession::native_vk_raw`])。dispatchable 句柄以地址值承载
/// (usize;不暴露裸指针类型,消费侧仅作同一性/拓扑对拍,不得解引用)。
#[derive(Debug, Clone, Copy)]
pub struct NativeVkHandles {
    /// VkInstance 地址值。
    pub instance: usize,
    /// VkPhysicalDevice 地址值。
    pub physical_device: usize,
    /// VkDevice 地址值。
    pub device: usize,
}

/// G14.10b(RFC-0030 §4.3)exportable 纹理的 Win32 导出簿记:导入方
/// (vendor/DLSS session 或任意同 LUID 物理设备上的 VkDevice)据此重建
/// **参数一致**的 external image 并导入同一块 device memory。
///
/// 所有权与生命周期:`handle` 为 NT handle,归产出它的 [`DeviceFrameSession`]
/// 所有(Drop 单点 `CloseHandle`)——消费侧**不得关闭**;句柄仅在 session
/// 存活期有效(导入完成后导入方持有自己的引用,session 先亡不影响已导入内存,
/// 但不得再用本簿记导入)。跨界内容有效性:session 每帧 cmd 末已对 exportable
/// image 录 `VK_QUEUE_FAMILY_EXTERNAL` release barrier(layout 恒 GENERAL),
/// 导入方消费前须录对应 acquire barrier(EXTERNAL→本家族,GENERAL→GENERAL),
/// 且须在该帧 fence 完成(`collect`/顺序 execute 返回)之后才提交消费。
#[derive(Debug, Clone, Copy)]
pub struct ExportedTextureWin32 {
    /// NT handle 地址值(`VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_BIT`)。
    pub handle: usize,
    pub width: u32,
    pub height: u32,
    /// VkFormat 数值(导入侧 image 须同格式)。
    pub vk_format: u32,
    /// 导出 image 的 `VkImageUsageFlags`(导入侧 image 须同 usage)。
    pub usage_flags: u32,
    /// 导出 allocation 字节数(导入侧 `allocationSize` 须一致)。
    pub allocation_size: u64,
    /// 导出 allocation 的 memory type index(同 physical device 上类型序一致,
    /// 导入侧直接复用;LUID 对拍先行)。
    pub memory_type_index: u32,
}

/// G14.10f exportable **buffer** 的 Win32 导出簿记(所有权/生命周期/跨界
/// 有效性契约与 [`ExportedTextureWin32`] 同律;buffer 线性布局跨 device 无
/// 歧义——vendor 输入驻留正道)。导入方以同 `allocation_size`/
/// `memory_type_index` import + dedicated(buffer) 绑定自建 VkBuffer(usage
/// 可异构,如导入侧 TRANSFER_SRC)。
#[derive(Debug, Clone, Copy)]
pub struct ExportedBufferWin32 {
    /// NT handle 地址值(`VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_BIT`)。
    pub handle: usize,
    /// buffer 声明字节数(导入侧 VkBuffer size 须一致)。
    pub size: u64,
    /// 导出 allocation 字节数(导入侧 `allocationSize` 须一致)。
    pub allocation_size: u64,
    /// 导出 allocation 的 memory type index(同 physical device 类型序一致)。
    pub memory_type_index: u32,
}

/// FIF 流水帧票据(G14plus RFC-0030 §4.3 L2;
/// [`DeviceFrameSession::submit_with_frame_update`] 产、
/// [`DeviceFrameSession::collect`] 消费,须交还产出它的同一 session)。
/// 线性令牌:不可克隆;弃置未 collect 的票据将使对应 frame slot 持续占用
/// (后续对该 slot 的 submit 与全部顺序 execute 入口确定性 `Err`——
/// fail-closed,不悬垂 fence)。
pub struct FrameTicket {
    inner: PersistentFrameTicket,
    provenance: SubmissionProvenance,
}

/// 持久 Vulkan device-frame session。
///
/// instance/device/queue/resources/descriptors/pipelines/command buffer/readback/history 均在
/// `new` 后保持到 Drop；每帧仅做 provenance 校验、fence slot 提交/有界等待、timestamp /
/// memory-budget/readback 采集。正常帧循环不调用 `vkQueueWaitIdle`；Drop 最终 teardown
/// 才允许排空 queue。
///
/// G7.6 Wave B:[`Self::new_with_accel_structs`] 追加 session AS 表(ray query compute
/// pass 经 `Bindings::accel_structs` 绑定);[`Self::execute_with_frame_update`] 支持
/// 数据驱动每帧重录(TLAS update / buffer 上传 / binding 覆盖 / push 覆盖 / readback
/// 子集);`execute`/`execute_with_provenance` 重放路径行为 0-byte。
pub struct DeviceFrameSession<'a> {
    resources: &'a [ResourceDesc<'a>],
    passes: &'a [Pass<'a>],
    barriers: &'a [&'a [(u32, TargetState)]],
    readbacks: &'a [Readback],
    native: NativePersistentFrame,
    /// 逐资源 generation(扩展表:`resources` 后接 AS 表槽位,初值全 0 = 初始内容代)。
    resource_generations: Vec<u64>,
    frame_generation: u64,
}

impl<'a> DeviceFrameSession<'a> {
    /// 创建固定 frame graph 的持久 session。`frame_slots` 须 ≥2；slot 各有独立 fence，
    /// 资源和 pipeline 跨 slot 共用且 ID 稳定。
    pub fn new(
        resources: &'a [ResourceDesc<'a>],
        passes: &'a [Pass<'a>],
        barriers: &'a [&'a [(u32, TargetState)]],
        readbacks: &'a [Readback],
        frame_slots: usize,
    ) -> Result<Self, String> {
        Self::new_with_accel_structs(resources, passes, barriers, readbacks, frame_slots, &[])
    }

    /// [`Self::new`] + session AS 表(G7.6 Wave B)。`accel_structs` 逐表项经 vk.rs
    /// **同一单所有者** `VkAsManager` 在同一 device 建 BLAS/TLAS(初始 build 创建期
    /// 一次性提交,单 fence 有界等待);AS 全量分配入 allocation ledger,AS 表项
    /// `StableResourceId` = `resources.len() + as_index + 1`。设备 ray query 四扩展 +
    /// feature 链缺一 → 确定性 `Err`(fail-closed,不隐式降级)。`accel_structs` 为空
    /// 时与 [`Self::new`] 逐字节同行为。
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_accel_structs(
        resources: &'a [ResourceDesc<'a>],
        passes: &'a [Pass<'a>],
        barriers: &'a [&'a [(u32, TargetState)]],
        readbacks: &'a [Readback],
        frame_slots: usize,
        accel_structs: &[AccelStructDesc<'a>],
    ) -> Result<Self, String> {
        Self::new_with_exportable_textures(
            resources,
            passes,
            barriers,
            readbacks,
            frame_slots,
            accel_structs,
            &[],
        )
    }

    /// G14.10b(RFC-0030 §4.3)[`Self::new_with_accel_structs`] + exportable
    /// 纹理集:`exportable_textures` 列出的 `resources` 下标(须为 Texture 资源,
    /// 不重复)以 external memory chain 建面——`VkExternalMemoryImageCreateInfo`
    /// (OPAQUE_WIN32)+ `VkExportMemoryAllocateInfo` + **dedicated allocation**
    /// (NVIDIA 上 Win32 导出实务强制,`VkMemoryDedicatedAllocateInfo` 必挂);
    /// device 启用 `VK_KHR_external_memory` + `VK_KHR_external_memory_win32`,
    /// 缺扩展确定性 `Err`(fail-closed,不降级)。集内资源的绑定/屏障/readback/
    /// provenance 面与普通 Texture 完全一致(既有资源路径 0-byte,新分支仅改
    /// 分配链);每帧 cmd 末自动追加:layout→GENERAL 收敛 +
    /// `VK_QUEUE_FAMILY_EXTERNAL` release barrier(跨 device 内容有效性;导入方
    /// acquire 契约见 [`ExportedTextureWin32`])。设备支持时一并启用
    /// `shaderStorageImageExtendedFormats`(RG32F 等扩展 storage 格式 compute
    /// 直写面)。`exportable_textures` 为空时与
    /// [`Self::new_with_accel_structs`] 逐字节同行为。
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_exportable_textures(
        resources: &'a [ResourceDesc<'a>],
        passes: &'a [Pass<'a>],
        barriers: &'a [&'a [(u32, TargetState)]],
        readbacks: &'a [Readback],
        frame_slots: usize,
        accel_structs: &[AccelStructDesc<'a>],
        exportable_textures: &[u32],
    ) -> Result<Self, String> {
        Self::new_with_external_textures(
            resources,
            passes,
            barriers,
            readbacks,
            frame_slots,
            accel_structs,
            exportable_textures,
            &[],
        )
    }

    /// G14.11(RFC-0030 §4.3)[`Self::new_with_exportable_textures`] +
    /// **D3D12 反向导入**纹理集:`imported_d3d12_textures` 各 `(资源下标, NT
    /// handle 地址值)`(handle = D3D12 `CreateSharedHandle` 产出,
    /// `D3D12_HEAP_FLAG_SHARED` committed 资源)以
    /// `VK_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE_BIT` external chain 建
    /// image 并 `VkImportMemoryWin32HandleInfoKHR` + dedicated 导入同一块显存
    /// (memoryTypeIndex 取 image requirements ∩ handle properties 的
    /// DEVICE_LOCAL 首匹配)。集内资源的绑定/屏障/provenance 面与 exportable
    /// 集同律:layout 状态机每帧 UNDEFINED 重初始化(内容不保留——本 session
    /// 恒为生产者),帧末自动追加 GENERAL 收敛 + `VK_QUEUE_FAMILY_EXTERNAL`
    /// release barrier(D3D12 侧消费窗内容有效性;跨 API 同步契约 = 本 session
    /// 该帧 fence 完成后 D3D12 才提交消费)。handle 归 D3D12 侧所有,本 session
    /// 不关闭。LUID 对拍(D3D12 adapter vs 本 session physical device)由调用
    /// 方先行,不匹配时导入确定性失败(fail-closed)。两集不得重叠;
    /// `imported_d3d12_textures` 为空时与
    /// [`Self::new_with_exportable_textures`] 逐字节同行为。
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_imported_d3d12_textures(
        resources: &'a [ResourceDesc<'a>],
        passes: &'a [Pass<'a>],
        barriers: &'a [&'a [(u32, TargetState)]],
        readbacks: &'a [Readback],
        frame_slots: usize,
        accel_structs: &[AccelStructDesc<'a>],
        exportable_textures: &[u32],
        imported_d3d12_textures: &[(u32, usize)],
    ) -> Result<Self, String> {
        Self::new_with_external_textures(
            resources,
            passes,
            barriers,
            readbacks,
            frame_slots,
            accel_structs,
            exportable_textures,
            imported_d3d12_textures,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_external_textures(
        resources: &'a [ResourceDesc<'a>],
        passes: &'a [Pass<'a>],
        barriers: &'a [&'a [(u32, TargetState)]],
        readbacks: &'a [Readback],
        frame_slots: usize,
        accel_structs: &[AccelStructDesc<'a>],
        exportable_textures: &[u32],
        imported_d3d12_textures: &[(u32, usize)],
    ) -> Result<Self, String> {
        validate_frame_with_as(
            resources,
            passes,
            barriers,
            readbacks,
            accel_structs.len() as u32,
        )?;
        // exportable 集校验(fail-closed;越界/重复/非法类均确定性拒)。G14.10f
        // 语义拓宽:Texture **或** Buffer(Buffer 须 data=None + device_local
        // ——external dedicated 分配与创建期 staging 上传互斥,且导出面必须
        // DEVICE_LOCAL;跨 device OPTIMAL tiling image 布局解释不保证一致
        // (NVIDIA 实测乱序),buffer 线性布局无歧义——vendor 输入驻留正道)。
        for (k, &res) in exportable_textures.iter().enumerate() {
            let Some(desc) = resources.get(res as usize) else {
                return Err(format!("exportable_textures[{k}]: 资源下标 {res} 越界"));
            };
            match desc {
                ResourceDesc::Texture(_) => {}
                ResourceDesc::Buffer(b) => {
                    if b.data.is_some() {
                        return Err(format!(
                            "exportable_textures[{k}]: buffer 资源 {res} 带初始数据(export 面须 data=None)"
                        ));
                    }
                    if !b.device_local {
                        return Err(format!(
                            "exportable_textures[{k}]: buffer 资源 {res} 非 device_local(export 面强制)"
                        ));
                    }
                }
            }
            if exportable_textures[..k].contains(&res) {
                return Err(format!("exportable_textures[{k}]: 资源 {res} 重复声明"));
            }
        }
        // G14.11 imported 集校验(同律;附 handle 非零 + 与 exportable 不重叠)。
        // 语义与 exportable 集同步拓宽:Texture **或** Buffer(Buffer 须
        // data=None + device_local——D3D12_RESOURCE 导入 dedicated 分配与创建
        // 期 staging 上传互斥;texture 直共享跨 API tiling 解释不一致弃案,
        // buffer 线性布局无歧义——FSR 驻留正道)。
        for (k, &(res, handle)) in imported_d3d12_textures.iter().enumerate() {
            let Some(desc) = resources.get(res as usize) else {
                return Err(format!("imported_d3d12_textures[{k}]: 资源下标 {res} 越界"));
            };
            match desc {
                ResourceDesc::Texture(t) => {
                    if t.data.is_some() {
                        return Err(format!(
                            "imported_d3d12_textures[{k}]: 资源 {res} 声明初始数据(导入面内容归 GPU 链,不受理 staging 初值)"
                        ));
                    }
                }
                ResourceDesc::Buffer(b) => {
                    if b.data.is_some() {
                        return Err(format!(
                            "imported_d3d12_textures[{k}]: buffer 资源 {res} 带初始数据(导入面须 data=None)"
                        ));
                    }
                    if !b.device_local {
                        return Err(format!(
                            "imported_d3d12_textures[{k}]: buffer 资源 {res} 非 device_local(导入面强制)"
                        ));
                    }
                }
            }
            if handle == 0 {
                return Err(format!(
                    "imported_d3d12_textures[{k}]: 资源 {res} handle 为空"
                ));
            }
            if imported_d3d12_textures[..k].iter().any(|&(r, _)| r == res) {
                return Err(format!("imported_d3d12_textures[{k}]: 资源 {res} 重复声明"));
            }
            if exportable_textures.contains(&res) {
                return Err(format!(
                    "imported_d3d12_textures[{k}]: 资源 {res} 同时声明 exportable(两集互斥)"
                ));
            }
        }
        #[cfg(not(windows))]
        if !exportable_textures.is_empty() || !imported_d3d12_textures.is_empty() {
            return Err("exportable/imported 纹理仅支持 Windows(win32 handle 面)".into());
        }
        if frame_slots < 2 {
            return Err("persistent frame session 须至少 2 个 fence frame slots".into());
        }
        let gipa =
            load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;
        // SAFETY: U32 持久扩注；所有 native 句柄移入 NativePersistentFrame，Drop 单点逆序
        // teardown。validation user_data 为 Box<AtomicU64>，地址在 session 生命周期内稳定。
        let native = unsafe {
            create_persistent_frame(
                gipa,
                resources,
                passes,
                barriers,
                readbacks,
                frame_slots,
                accel_structs,
                exportable_textures,
                imported_d3d12_textures,
            )?
        };
        let as_count = native.as_count();
        Ok(Self {
            resources,
            passes,
            barriers,
            readbacks,
            native,
            resource_generations: vec![0; resources.len() + as_count],
            frame_generation: 0,
        })
    }

    /// session AS 表项数(0 = 无 AS 面)。
    fn as_count(&self) -> usize {
        self.native.as_count()
    }

    /// 各 pass 声明绑定集克隆(重放路径 provenance 的 effective bindings;
    /// `Bindings` 全 Vec 小表,每帧一次浅克隆)。
    fn declared_bindings(&self) -> Vec<Bindings> {
        self.passes
            .iter()
            .map(|p| pass_bindings(p).clone())
            .collect()
    }

    /// 下一帧的实际 provenance（未提交）；RED 可克隆并篡改后交给
    /// [`Self::execute_with_provenance`] 验证。
    #[must_use]
    pub fn next_provenance(&self) -> SubmissionProvenance {
        build_runtime_provenance_ext(
            self.passes,
            &self.declared_bindings(),
            &self.native.resource_allocations,
            &self.resource_generations,
            self.frame_generation + 1,
            self.resources.len() as u32,
        )
    }

    /// 使用执行器生成的 actual provenance 提交一帧。
    pub fn execute(&mut self) -> Result<DeviceFrameOutput, String> {
        let provenance = self.next_provenance();
        self.execute_with_provenance(&provenance)
    }

    /// 使用调用方提供的 provenance 提交；任何 stale/wrong allocation 或 generation 在
    /// `vkQueueSubmit` 前确定性拒绝（RED 注入入口）。
    pub fn execute_with_provenance(
        &mut self,
        supplied: &SubmissionProvenance,
    ) -> Result<DeviceFrameOutput, String> {
        let record_started = std::time::Instant::now();
        let expected = self.next_provenance();
        validate_submission_provenance(&expected, supplied)?;
        let validate_ns = elapsed_ns(record_started);
        // SAFETY: native session 独占 &mut self；fence 保证 command buffer/readback 不在途；
        // execute_persistent_frame 正常帧只 wait/reset fence，不 queue-wait-idle。
        let (readbacks, mut telemetry) = unsafe {
            execute_persistent_frame(
                &mut self.native,
                self.resources,
                self.passes,
                self.barriers,
                self.readbacks,
                None,
                None,
            )?
        };
        telemetry.cpu_record_ns += validate_ns;
        self.commit_provenance(&expected);
        Ok(DeviceFrameOutput {
            readbacks,
            provenance: expected,
            telemetry,
        })
    }

    /// FrameUpdate 派生态(effective bindings + pre-bump generations;两 provenance
    /// 入口共用同一事实源)。G34-2:`tlas_b` = 第二 TLAS 更新(双 TLAS 车道
    /// 表 1;校验/异槽纪律见 [`validate_tlas_update_b`])——`None` = 既有面
    /// 0-byte。G37 W3 hzb_skin:`blas_b` = 第二 BLAS refit(表 1 manager 的
    /// 蒙皮顶点 refit;校验见 [`validate_blas_refit_b`])——`None` = 既有面
    /// 0-byte。
    fn frame_update_state(
        &self,
        update: &FrameUpdate,
        tlas_b: Option<&(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
        blas_b: Option<&BlasRefitUpdate>,
    ) -> Result<(Vec<Bindings>, Vec<u64>), String> {
        let as_count = self.as_count() as u32;
        validate_frame_update(
            self.resources,
            self.passes,
            self.readbacks,
            as_count,
            update,
        )?;
        validate_tlas_update_b(as_count, update, tlas_b)?;
        validate_blas_refit_b(self.resources, self.passes, tlas_b, blas_b)?;
        // effective bindings:声明 → binding_overrides → push_constant_overrides。
        let mut effective = self.declared_bindings();
        for &(pi, ref bindings) in &update.binding_overrides {
            effective[pi as usize] = bindings.clone();
        }
        for &(pi, ref bytes) in &update.push_constant_overrides {
            effective[pi as usize].push_constants.clone_from(bytes);
        }
        // 内容写(TLAS update / buffer 上传 / BLAS refit)先 bump generation,
        // provenance 消费侧 producer 指向新代。
        let mut generations = self.resource_generations.clone();
        if let Some((as_index, _, _)) = &update.tlas_update {
            let slot = self.resources.len() + *as_index as usize;
            generations[slot] = generations[slot].saturating_add(1);
        }
        // G31+ 波 B Task B5:BLAS refit 改写 AS 内容,与 TLAS update 同槽同律
        // 记账;同帧 tlas+blas 同槽同现时仅本臂 bump(tlas 臂未命中时),
        // 同槽双写归并为一个内容代(代序语义 = 「本帧内容代 +1」)。
        if let Some(b) = &update.blas_refit {
            let tlas_same_slot =
                matches!(&update.tlas_update, Some((ai, _, _)) if *ai == b.as_index);
            if !tlas_same_slot {
                let slot = self.resources.len() + b.as_index as usize;
                generations[slot] = generations[slot].saturating_add(1);
            }
        }
        // G34-2:第二 TLAS 更新同律记账(校验面已保证与 tlas_update/blas_refit
        // 异槽——各槽内容写各自 bump 一代,代序语义 = 「本帧内容代 +1」)。
        // G37 W3 hzb_skin:blas_b 校验面已保证与 tlas_b 同槽同现 ⇒ 同槽双写
        // 归并为一个内容代(blas_refit×tlas_update 同律),本臂零追加 bump。
        if let Some((as_index_b, _, _)) = tlas_b {
            let slot = self.resources.len() + *as_index_b as usize;
            generations[slot] = generations[slot].saturating_add(1);
        }
        for (resource_id, _, _) in &update.buffer_uploads {
            let slot = (resource_id.0 - 1) as usize;
            generations[slot] = generations[slot].saturating_add(1);
        }
        Ok((effective, generations))
    }

    /// 下一帧在 [`FrameUpdate`] 作用下的实际 provenance(未提交;含 TLAS/上传
    /// generation pre-bump 与 override 后 effective bindings)。RED 可克隆并篡改后
    /// 交给 [`Self::execute_with_frame_update`] 验证。update 非法(同
    /// `validate_frame_update` 判据)→ 确定性 `Err`。
    pub fn next_provenance_with_update(
        &self,
        update: &FrameUpdate,
    ) -> Result<SubmissionProvenance, String> {
        self.next_provenance_with_update_dual_tlas(update, None)
    }

    /// G34-2(HZB 接统一车道)加性:双 TLAS 更新 provenance 预推入口——
    /// `tlas_update_b` = 同帧第二 TLAS 实例更新(双 TLAS 车道表 1;校验/异槽
    /// 纪律见 [`validate_tlas_update_b`])。`None` 与
    /// [`Self::next_provenance_with_update`] 逐字同路径(既有面 0-byte)。
    pub fn next_provenance_with_update_dual_tlas(
        &self,
        update: &FrameUpdate,
        tlas_update_b: Option<&(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
    ) -> Result<SubmissionProvenance, String> {
        self.next_provenance_with_update_dual_tlas_ex(update, tlas_update_b, None)
    }

    /// G37 W3 hzb_skin 加性:双 TLAS + 第二 BLAS refit provenance 预推入口——
    /// `blas_refit_b` = 表 1 manager 的同帧蒙皮顶点 refit(校验/同槽纪律见
    /// [`validate_blas_refit_b`])。`None` 与
    /// [`Self::next_provenance_with_update_dual_tlas`] 逐字同路径(既有面 0-byte)。
    pub fn next_provenance_with_update_dual_tlas_ex(
        &self,
        update: &FrameUpdate,
        tlas_update_b: Option<&(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
        blas_refit_b: Option<&BlasRefitUpdate>,
    ) -> Result<SubmissionProvenance, String> {
        let (effective, generations) =
            self.frame_update_state(update, tlas_update_b, blas_refit_b)?;
        Ok(build_runtime_provenance_ext(
            self.passes,
            &effective,
            &self.native.resource_allocations,
            &generations,
            self.frame_generation + 1,
            self.resources.len() as u32,
        ))
    }

    /// 使用调用方提供的 provenance + 数据驱动 [`FrameUpdate`] 提交一帧(G7.6 Wave B
    /// 每帧重录面)。校验序(全部在 `vkQueueSubmit` 前 fail-closed):
    /// ① `validate_frame_update`(下标/区段/布局键/push 长度/readback 子集合法性);
    /// ② override 后 effective bindings 重推 expected provenance(TLAS update 与
    /// buffer 上传先 bump 各自 generation);③ `validate_submission_provenance`
    /// (override 引用的 resource/generation 一并覆核)。
    /// 有重录内容时 `vkResetCommandBuffer` + 重录(上传段 → TLAS update+consume
    /// barrier → 固定 pass 链应用 overrides → timestamp 标记 → readback 子集);
    /// `cpu_record_ns` 如实计量校验 + host 写 + 重录耗时。
    pub fn execute_with_frame_update(
        &mut self,
        supplied: &SubmissionProvenance,
        update: &FrameUpdate,
    ) -> Result<DeviceFrameOutput, String> {
        self.execute_with_frame_update_dual_tlas(supplied, update, None)
    }

    /// G34-2(HZB 接统一车道)加性:双 TLAS 更新提交入口——`tlas_update_b` =
    /// 同帧第二 TLAS 实例更新(双 TLAS 车道表 1;动态实例场景下初剔表与全量
    /// 表各自逐帧 refit 的消费面)。校验序/录制序与
    /// [`Self::execute_with_frame_update`] 同源同律(第二更新先于 pass 链录
    /// update + 与主更新共单条 consume barrier);`None` 与既有入口逐字同
    /// 路径(既有面 0-byte——命令流/provenance/遥测逐字节不变)。FIF 流水面
    /// 不开放(`tlas_update` 同约束——共享 host 写面在飞帧读取中不可改写)。
    pub fn execute_with_frame_update_dual_tlas(
        &mut self,
        supplied: &SubmissionProvenance,
        update: &FrameUpdate,
        tlas_update_b: Option<(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
    ) -> Result<DeviceFrameOutput, String> {
        self.execute_with_frame_update_dual_tlas_ex(supplied, update, tlas_update_b, None)
    }

    /// G37 W3 hzb_skin 加性:双 TLAS + 第二 BLAS refit 提交入口——
    /// `blas_refit_b` = 表 1 manager 的同帧蒙皮顶点 refit(双 TLAS×蒙皮合并
    /// 车道消费面:主射线表 0 与阴影射线表 1 各持 BLAS 副本,蒙皮角色两副本
    /// 须同帧 refit;录制面 = `blas_refit` 同律桥接——pass `after_pass` 录完
    /// 后 copy + UPDATE build + consume barrier,目标 = 表 1 manager)。
    /// 校验(fail-closed 确定性 Err):须与 `tlas_update_b` 同现同槽(单帧 AS
    /// 操作归并同一 manager——`blas_refit`×`tlas_update` 同律);目标 BLAS 须
    /// 创建期 updatable 打标。`None` 与
    /// [`Self::execute_with_frame_update_dual_tlas`] 逐字同路径(既有面
    /// 0-byte——命令流/provenance/遥测逐字节不变)。FIF 流水面不开放
    /// (`blas_refit` 同约束)。
    pub fn execute_with_frame_update_dual_tlas_ex(
        &mut self,
        supplied: &SubmissionProvenance,
        update: &FrameUpdate,
        tlas_update_b: Option<(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
        blas_refit_b: Option<BlasRefitUpdate>,
    ) -> Result<DeviceFrameOutput, String> {
        self.execute_with_frame_update_inner(supplied, update, tlas_update_b, blas_refit_b, None)
    }

    /// G38 T3 加性:BLAS refit 桥扩展提交入口——`bridge_ext` = 多 region 脏
    /// 区段 copy + 桥接 GPU 计时(语义见 [`BlasRefitBridgeExt`])。`None` 与
    /// [`Self::execute_with_frame_update`] 逐字同路径(既有面 0-byte——命令
    /// 流/provenance/遥测逐字节不变);`copy_regions` 须与 `update.blas_refit`
    /// 同现(fail-closed 校验见 [`validate_bridge_ext`])。provenance 面不感知
    /// regions(copy 子集为执行细节——src 与 vbuf 同布局下任意合法区段集的
    /// vbuf 终态字节相同,AS 内容代记账仍由 `blas_refit` 承载)。
    pub fn execute_with_frame_update_bridge_ext(
        &mut self,
        supplied: &SubmissionProvenance,
        update: &FrameUpdate,
        bridge_ext: Option<&BlasRefitBridgeExt>,
    ) -> Result<DeviceFrameOutput, String> {
        self.execute_with_frame_update_inner(supplied, update, None, None, bridge_ext)
    }

    /// 提交入口共用主体(`execute_with_frame_update_dual_tlas_ex` 原主体整体
    /// 迁入;`bridge_ext = None` 时与迁入前逐字等价——既有三入口 0-byte)。
    fn execute_with_frame_update_inner(
        &mut self,
        supplied: &SubmissionProvenance,
        update: &FrameUpdate,
        tlas_update_b: Option<(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
        blas_refit_b: Option<BlasRefitUpdate>,
        bridge_ext: Option<&BlasRefitBridgeExt>,
    ) -> Result<DeviceFrameOutput, String> {
        let record_started = std::time::Instant::now();
        // G38 T3:桥扩展 fail-closed 预校验(任何 GPU 调用前;None 恒过)。
        validate_bridge_ext(update, bridge_ext)?;
        let (effective, generations) =
            self.frame_update_state(update, tlas_update_b.as_ref(), blas_refit_b.as_ref())?;
        let expected = build_runtime_provenance_ext(
            self.passes,
            &effective,
            &self.native.resource_allocations,
            &generations,
            self.frame_generation + 1,
            self.resources.len() as u32,
        );
        validate_submission_provenance(&expected, supplied)?;
        let validate_ns = elapsed_ns(record_started);

        let mut descriptor_overrides: Vec<u32> = Vec::new();
        for &(pi, _) in &update.binding_overrides {
            descriptor_overrides.push(pi);
        }
        let uploads: Vec<(u32, u64, &[u8])> = update
            .buffer_uploads
            .iter()
            .map(|(resource_id, offset, bytes)| {
                ((resource_id.0 - 1) as u32, *offset, bytes.as_slice())
            })
            .collect();
        let tlas = update
            .tlas_update
            .as_ref()
            .map(|(as_index, instances, action)| (*as_index, instances.as_slice(), *action));
        // G34-2:第二 TLAS 更新同形解析(校验面已保证异槽)。
        let tlas_b = tlas_update_b
            .as_ref()
            .map(|(as_index, instances, action)| (*as_index, instances.as_slice(), *action));
        // G31+ 波 B Task B5:blas_refit 解析为(native 侧消费形)——src
        // StableResourceId → 资源下标(校验已在 frame_update_state 完成)。
        let blas = update.blas_refit.map(|b| {
            (
                b.as_index,
                b.blas_index,
                (b.src.0 - 1) as u32,
                b.src_offset,
                b.byte_len,
                b.after_pass,
            )
        });
        // G37 W3 hzb_skin:第二 BLAS refit 同形解析(校验面已保证与 tlas_b 同槽)。
        let blas_b = blas_refit_b.map(|b| {
            (
                b.as_index,
                b.blas_index,
                (b.src.0 - 1) as u32,
                b.src_offset,
                b.byte_len,
                b.after_pass,
            )
        });
        let (effective_readbacks, effective_rb_sources) = match &update.readback_subset {
            Some(indices) => (
                indices
                    .iter()
                    .map(|&i| self.readbacks[i as usize])
                    .collect::<Vec<Readback>>(),
                indices.iter().map(|&i| i as usize).collect::<Vec<usize>>(),
            ),
            None => (Vec::new(), Vec::new()),
        };
        // G14.12:readback 子集「同形」时命令体逐字节不变 → 跳过重录(见
        // NativeDeviceFrame::recorded_rb_sources)。TLAS update / binding override /
        // push override 任一存在则命令体确实变化,照旧重录。
        let rb_shape_stale = self.native.frame.recorded_rb_sources.as_deref()
            != Some(effective_rb_sources.as_slice());
        let needs_rerecord = tlas.is_some()
            || tlas_b.is_some()
            || blas.is_some()
            || blas_b.is_some()
            || !descriptor_overrides.is_empty()
            || !update.push_constant_overrides.is_empty()
            || rb_shape_stale;
        let prepared = PreparedFrameUpdate {
            uploads: &uploads,
            tlas,
            tlas_b,
            blas,
            blas_b,
            descriptor_overrides: &descriptor_overrides,
            effective_bindings: &effective,
            effective_readbacks: &effective_readbacks,
            effective_rb_sources: &effective_rb_sources,
            needs_rerecord,
        };
        // SAFETY: native session 独占 &mut self;prepared 全部引用本帧栈上数据,随调用
        // 结束失效;fence 纪律同 execute_with_provenance(正常帧不 queue-wait-idle)。
        let (readbacks, mut telemetry) = unsafe {
            execute_persistent_frame(
                &mut self.native,
                self.resources,
                self.passes,
                self.barriers,
                self.readbacks,
                Some(&prepared),
                bridge_ext,
            )?
        };
        telemetry.cpu_record_ns += validate_ns;
        // 提交成功才落账:先写回 pre-bump 后 generations,再按 pass 产出覆盖。
        self.resource_generations = generations;
        self.commit_provenance(&expected);
        Ok(DeviceFrameOutput {
            readbacks,
            provenance: expected,
            telemetry,
        })
    }

    /// FIF 流水提交半程(G14plus RFC-0030 §4.3 L2,G14.10 消费面):校验序与
    /// [`Self::execute_with_frame_update`] 同源(① `validate_frame_update`
    /// ② effective 态重推 expected provenance ③ `validate_submission_provenance`),
    /// 但 submit 后**不等完成 fence**,返回 [`FrameTicket`] 交
    /// [`Self::collect`] 收集——CPU 侧 submit/collect 解耦即 FIF 流水收益。
    /// GPU 帧间由 per-slot cmd 首条全局守卫 barrier 全序化,逐帧图像与顺序
    /// execute(FIF=1)**位级一致**(per-slot cmd/timestamp 区间/上传 staging/
    /// 回读 staging 隔离;同 slot 复用前 fence 等待;见
    /// `submit_pipelined_frame` 确定性论证)。
    ///
    /// G31(波 A Task A2):`binding_overrides` 经 **per-slot descriptor
    /// override set** 入流水(共享 session set 在飞帧使用中不可重写;slot set
    /// 的上次使用 = 本 slot 上一帧,submit 期 fence 已等待,host 重写无在途
    /// 竞争;写经创建期同一 `write_pass_descriptor_set` 事实源,内容与顺序路
    /// 共享 set 重写产物逐位同 ⇒ 位级零漂移;首个消费 = 生产管线 TSR parity
    /// 轮换——G14.3 头注释登记的未消费面①兑现)。
    ///
    /// 流水约束(fail-closed 确定性 `Err`,不静默降级):
    /// - `tlas_update`——TLAS instance buffer 为共享 host 写面,在飞帧读取中
    ///   不可改写(需 TLAS 更新走顺序入口);
    /// - 同 slot 票据未 collect 再 submit(FIF 深度 = `frame_slots` 已满);
    /// - 任何票据未 collect 时调用顺序 `execute*` 入口。
    ///
    /// 提交成功即落账 generation/frame 计数(流水语义:下帧 provenance 须见
    /// 本帧内容代;collect 失败属会话级失败,不回滚)。
    pub fn submit_with_frame_update(
        &mut self,
        supplied: &SubmissionProvenance,
        update: &FrameUpdate,
    ) -> Result<FrameTicket, String> {
        let record_started = std::time::Instant::now();
        // FIF 路无双 TLAS/双 BLAS 更新面(tlas_update_b/blas_b 不开放——G34-2
        // 双 TLAS 与 G37 hzb_skin 双 BLAS 车道走顺序入口;None = 既有面 0-byte。
        // 第三参随 hzb_skin 窗 frame_update_state 签名迁移机械适配,行为 0 变)。
        let (effective, generations) = self.frame_update_state(update, None, None)?;
        let expected = build_runtime_provenance_ext(
            self.passes,
            &effective,
            &self.native.resource_allocations,
            &generations,
            self.frame_generation + 1,
            self.resources.len() as u32,
        );
        validate_submission_provenance(&expected, supplied)?;
        let validate_ns = elapsed_ns(record_started);
        if update.tlas_update.is_some() {
            return Err(
                "FIF 流水不支持 tlas_update(TLAS instance buffer 为共享 host 写面,在飞帧\
                 读取中不可改写;需 TLAS 更新请走顺序 execute_with_frame_update)"
                    .into(),
            );
        }
        // G31+ 波 B Task B5:BLAS 顶点缓冲同为由在飞帧 ray query 读取的共享
        // 写面,refit 走顺序入口(tlas_update 同律,fail-closed 不静默降级)。
        if update.blas_refit.is_some() {
            return Err(
                "FIF 流水不支持 blas_refit(BLAS 顶点缓冲为共享写面,在飞帧 ray query\
                 读取中不可改写;需 BLAS 更新请走顺序 execute_with_frame_update)"
                    .into(),
            );
        }
        let uploads: Vec<(u32, u64, &[u8])> = update
            .buffer_uploads
            .iter()
            .map(|(resource_id, offset, bytes)| {
                ((resource_id.0 - 1) as u32, *offset, bytes.as_slice())
            })
            .collect();
        let (effective_readbacks, effective_rb_sources) = match &update.readback_subset {
            Some(indices) => (
                indices
                    .iter()
                    .map(|&i| self.readbacks[i as usize])
                    .collect::<Vec<Readback>>(),
                indices.iter().map(|&i| i as usize).collect::<Vec<usize>>(),
            ),
            None => (Vec::new(), Vec::new()),
        };
        let mut descriptor_overrides: Vec<u32> = Vec::new();
        for &(pi, _) in &update.binding_overrides {
            descriptor_overrides.push(pi);
        }
        let prepared = PreparedFrameUpdate {
            uploads: &uploads,
            tlas: None,
            // FIF 路 tlas_update_b 已在上方 fail-closed 拒(恒 None)。
            tlas_b: None,
            // FIF 路 blas_refit 已在上方 fail-closed 拒(恒 None)。
            blas: None,
            // FIF 路无 blas_refit_b 入口(dual_tlas_ex 顺序独有;恒 None)。
            blas_b: None,
            descriptor_overrides: &descriptor_overrides,
            effective_bindings: &effective,
            effective_readbacks: &effective_readbacks,
            effective_rb_sources: &effective_rb_sources,
            // 流水路恒重录本 slot cmd(slot query 区间与 staged 段皆帧相关)。
            needs_rerecord: true,
        };
        // SAFETY: native session 独占 &mut self;prepared 全部引用本帧栈上数据,
        // 随调用结束失效;slot fence 纪律见 submit_pipelined_frame 契约。
        let mut inner = unsafe {
            submit_pipelined_frame(
                &mut self.native,
                self.resources,
                self.passes,
                self.barriers,
                self.readbacks,
                &prepared,
            )?
        };
        inner.record_ns += validate_ns;
        self.resource_generations = generations;
        self.commit_provenance(&expected);
        Ok(FrameTicket {
            inner,
            provenance: expected,
        })
    }

    /// FIF 流水收集半程:等待票据帧 fence(有界)→ timestamp(slot 区间)→
    /// per-slot staging 回读 → telemetry,释放 slot 占用。任意顺序 collect 均
    /// 正确(各票据等各自 fence);典型消费为 FIFO(submit N+1 后 collect N,
    /// FIF=2)。`cpu_fence_wait_ns` 如实包含 submit 与 collect 之间的应用侧
    /// 时间(诚实计量,不掩饰)。
    pub fn collect(&mut self, ticket: FrameTicket) -> Result<DeviceFrameOutput, String> {
        let FrameTicket { inner, provenance } = ticket;
        // SAFETY: native session 独占 &mut self;票据由本 session submit 产出,
        // slot/fence/staging 均存活;collect_persistent_frame 只等 fence 不 reset。
        // FIF 路无 blas_refit(fail-closed 已拒)⇒ 桥接计时恒 None。
        let (readbacks, telemetry) =
            unsafe { collect_persistent_frame(&mut self.native, self.passes, inner, None)? };
        Ok(DeviceFrameOutput {
            readbacks,
            provenance,
            telemetry,
        })
    }

    /// 提交成功后的 generation/frame 计数落账(两入口共用)。
    fn commit_provenance(&mut self, expected: &SubmissionProvenance) {
        self.frame_generation += 1;
        for pass in &expected.passes {
            for r in &pass.resources {
                if let Some(generation) = r.produced_generation {
                    self.resource_generations[(r.resource_id.0 - 1) as usize] = generation;
                }
            }
        }
    }

    /// G14plus vendor 域(RFC-0030)加性只读 accessor:`resource_index`(创建
    /// 时 `resources` 下标)对应 Texture 资源的原生 Vulkan 句柄簿记。越界或该
    /// 下标为 Buffer 资源 → `None`。执行语义 0-byte(纯簿记读,不触碰任何
    /// 录制/提交/生命周期面);句柄所有权约束见 [`NativeTextureHandles`]。
    pub fn texture_native_handles(&self, resource_index: usize) -> Option<NativeTextureHandles> {
        let img = self.native.frame.rt.get(resource_index)?.image()?;
        Some(NativeTextureHandles {
            image: img.image,
            memory: img.mem,
            view: img.view,
            width: img.width,
            height: img.height,
            vk_format: img.format.vk_format(),
        })
    }

    /// G14plus vendor 域(RFC-0030)加性只读 accessor:session 的 Vulkan 顶层
    /// 句柄地址簿记(instance/physical device/device)。消费侧仅作同一性/
    /// 拓扑对拍(如与 vendor session 的 device 独立性核对),不得解引用。
    pub fn native_vk_raw(&self) -> NativeVkHandles {
        NativeVkHandles {
            instance: self.native.instance as usize,
            physical_device: self.native.pd as usize,
            device: self.native.device as usize,
        }
    }

    /// G14.10b 加性只读 accessor:session 物理设备 LUID
    /// (`VkPhysicalDeviceIDProperties::deviceLUID`,创建期一次性实采)。
    /// `None` = 驱动报 `deviceLUIDValid=false`。消费侧与 vendor(DLSS)session
    /// 的 LUID 对拍——**同 adapter 才可共享 external memory**,不同即 fail-closed。
    pub fn physical_device_luid(&self) -> Option<[u8; 8]> {
        self.native.device_luid
    }

    /// G14.10b:exportable 纹理的 Win32 NT handle 导出(仅
    /// [`Self::new_with_exportable_textures`] 声明过的下标合法;其余 → `Err`)。
    /// 首次调用经 `vkGetMemoryWin32HandleKHR` 真导出并缓存,重复调用返回同一
    /// 句柄(免句柄泄漏);句柄归 session 所有,Drop 单点 `CloseHandle`——
    /// 调用方**不得关闭**。导入参数契约见 [`ExportedTextureWin32`]。
    pub fn export_texture_win32_handle(
        &mut self,
        resource_index: usize,
    ) -> Result<ExportedTextureWin32, String> {
        self.native.export_texture_win32_handle(resource_index)
    }

    /// G14.10f:exportable **buffer** 的 Win32 NT handle 导出(缓存/所有权
    /// 纪律与 [`Self::export_texture_win32_handle`] 同律;导入参数契约见
    /// [`ExportedBufferWin32`])。
    pub fn export_buffer_win32_handle(
        &mut self,
        resource_index: usize,
    ) -> Result<ExportedBufferWin32, String> {
        self.native.export_buffer_win32_handle(resource_index)
    }

    /// G31+ 波 C Task C7 加性只读 accessor:本 session 录制面 debug label
    /// 标注是否活跃（VK_EXT_debug_utils 启用且双符号解析成功;执行语义
    /// 0-byte 纯簿记读）。`true` = 各 pass cmd 已录
    /// `vkCmdBeginDebugUtilsLabelEXT`/`vkCmdEndDebugUtilsLabelEXT` 配对
    /// （pass 名;Nsight/RenderDoc 逐 pass 可辨识）,`false` = absent 零开销跳过。
    pub fn debug_labels_active(&self) -> bool {
        self.native.frame.dev.labels_active()
    }
}

// ─────────────────────────── 纯函数层(host 可测,零 unsafe) ───────────────────────────

/// FNV-1a 64 位散列(pipeline cache 键原料;确定性,host 可测)。
pub(crate) fn fnv1a_u64(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// raster pipeline cache 键(相同 SPIR-V+格式组合+顶点布局复用 `VkPipeline`;
/// render pass 兼容性只取决于格式/采样数,与 loadOp 无关,故 clear 变体不入键)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RasterPipelineKey {
    vs_hash: u64,
    fs_hash: u64,
    color_formats: Vec<u32>,
    /// 0 = 无 depth。
    depth_format: u32,
    vertex_stride: u32,
    attrs: Vec<(u32, u32, u32)>,
    has_vb: bool,
    /// 保守光栅状态入键(G7.5b,RXS-0303 L2):`None` = 无 pNext 链(既有键值 0-byte);
    /// `Some(bits)` = OVERESTIMATE + `extra_overestimation` 的 f32 位型(f32 无 Eq/Hash,
    /// 以 `to_bits` 承载——同值同键、位型不同即不同 pipeline)。
    conservative: Option<u32>,
}

/// compute pipeline cache 键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ComputePipelineKey {
    spv_hash: u64,
    entry: Vec<u8>,
}

/// set0 descriptor 布局键:(accel struct A, storage N, sampled M, storage image K, uniform U)。
/// A=0 时与 G5 起四元组同值域(0-byte 兼容)。
type SetLayoutKey = (u32, u32, u32, u32, bool);

/// pipeline layout 键(set0 布局 + push constant 块尺寸)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PipelineLayoutKey {
    set: SetLayoutKey,
    pc_size: u32,
}

/// render pass 键(格式 + 逐 attachment loadOp;loadOp 影响 render pass 本体不入
/// pipeline 键)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RenderPassKey {
    color_formats: Vec<u32>,
    /// 0 = 无 depth。
    depth_format: u32,
    color_clears: Vec<bool>,
    depth_clear: bool,
}

// synchronization2 stage/access(64 位;SDK 1.3.296 `vulkan_core.h` 核对)。
const STAGE2_NONE: u64 = 0;
const STAGE2_DRAW_INDIRECT: u64 = 0x2;
const STAGE2_VERTEX_INPUT: u64 = 0x4;
const STAGE2_VERTEX_SHADER: u64 = 0x8;
const STAGE2_FRAGMENT_SHADER: u64 = 0x80;
const STAGE2_EARLY_FRAGMENT_TESTS: u64 = 0x100;
const STAGE2_LATE_FRAGMENT_TESTS: u64 = 0x200;
const STAGE2_COLOR_ATTACHMENT_OUTPUT: u64 = 0x400;
const STAGE2_COMPUTE_SHADER: u64 = 0x800;
const STAGE2_TRANSFER: u64 = 0x1000;
const STAGE2_HOST: u64 = 0x4000;

const ACCESS2_INDIRECT_COMMAND_READ: u64 = 0x1;
const ACCESS2_VERTEX_ATTRIBUTE_READ: u64 = 0x4;
const ACCESS2_UNIFORM_READ: u64 = 0x8;
const ACCESS2_SHADER_READ: u64 = 0x20;
const ACCESS2_SHADER_WRITE: u64 = 0x40;
const ACCESS2_COLOR_ATTACHMENT_READ: u64 = 0x80;
const ACCESS2_COLOR_ATTACHMENT_WRITE: u64 = 0x100;
const ACCESS2_DEPTH_STENCIL_ATTACHMENT_READ: u64 = 0x200;
const ACCESS2_DEPTH_STENCIL_ATTACHMENT_WRITE: u64 = 0x400;
const ACCESS2_TRANSFER_READ: u64 = 0x800;
const ACCESS2_TRANSFER_WRITE: u64 = 0x1000;
const ACCESS2_HOST_WRITE: u64 = 0x4000;
// 全域 access2(G14plus FIF 流水帧间守卫 memory barrier 用;SDK 1.3.296)。
const ACCESS2_MEMORY_READ: u64 = 0x8000;
const ACCESS2_MEMORY_WRITE: u64 = 0x1_0000;
// G31+ 波 B Task B5:BLAS refit 桥接的 sync2 stage/access(SDK 1.3.296
// `VK_PIPELINE_STAGE_2_ACCELERATION_STRUCTURE_BUILD_BIT_KHR` /
// `VK_ACCESS_2_ACCELERATION_STRUCTURE_READ_BIT_KHR`;与 vk.rs legacy 32 位
// 同名常量 0x0200_0000/0x0020_0000 同值同域)。
const STAGE2_ACCEL_STRUCTURE_BUILD: u64 = 0x0200_0000;
const ACCESS2_ACCEL_STRUCTURE_READ: u64 = 0x0020_0000;

/// 全部着色阶段 stage2(VS|FS|CS;descriptor/push 约定的 stage 超集同律)。
const STAGE2_ALL_SHADERS: u64 =
    STAGE2_VERTEX_SHADER | STAGE2_FRAGMENT_SHADER | STAGE2_COMPUTE_SHADER;

// image layout(Vulkan 1.0 枚举)。
const LAYOUT_UNDEFINED: u32 = 0;
const LAYOUT_GENERAL: u32 = 1;
const LAYOUT_COLOR_ATTACHMENT_OPTIMAL: u32 = 2;
const LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL: u32 = 3;
const LAYOUT_SHADER_READ_ONLY_OPTIMAL: u32 = 5;
const LAYOUT_TRANSFER_SRC_OPTIMAL: u32 = 6;
const LAYOUT_TRANSFER_DST_OPTIMAL: u32 = 7;

/// [`TargetState`] → (image layout, stage2, access2) 单源映射(host 可测纯函数;
/// buffer 忽略 layout 槽)。跟踪态三元组 = 本映射产物,屏障前后场直接取。
pub(crate) fn state_fields(s: TargetState) -> (u32, u64, u64) {
    match s {
        TargetState::ColorAttachmentWrite => (
            LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
            STAGE2_COLOR_ATTACHMENT_OUTPUT,
            ACCESS2_COLOR_ATTACHMENT_WRITE | ACCESS2_COLOR_ATTACHMENT_READ,
        ),
        TargetState::DepthAttachmentWrite => (
            LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            STAGE2_EARLY_FRAGMENT_TESTS | STAGE2_LATE_FRAGMENT_TESTS,
            ACCESS2_DEPTH_STENCIL_ATTACHMENT_WRITE | ACCESS2_DEPTH_STENCIL_ATTACHMENT_READ,
        ),
        TargetState::ShaderRead => (
            LAYOUT_SHADER_READ_ONLY_OPTIMAL,
            STAGE2_ALL_SHADERS,
            ACCESS2_SHADER_READ,
        ),
        TargetState::StorageImageReadWrite => (
            LAYOUT_GENERAL,
            STAGE2_ALL_SHADERS,
            ACCESS2_SHADER_READ | ACCESS2_SHADER_WRITE,
        ),
        TargetState::TransferSrc => (
            LAYOUT_TRANSFER_SRC_OPTIMAL,
            STAGE2_TRANSFER,
            ACCESS2_TRANSFER_READ,
        ),
        TargetState::TransferDst => (
            LAYOUT_TRANSFER_DST_OPTIMAL,
            STAGE2_TRANSFER,
            ACCESS2_TRANSFER_WRITE,
        ),
        TargetState::StorageWrite => (LAYOUT_UNDEFINED, STAGE2_ALL_SHADERS, ACCESS2_SHADER_WRITE),
        TargetState::StorageReadWrite => (
            LAYOUT_UNDEFINED,
            STAGE2_ALL_SHADERS,
            ACCESS2_SHADER_READ | ACCESS2_SHADER_WRITE,
        ),
        TargetState::UniformRead => (LAYOUT_UNDEFINED, STAGE2_ALL_SHADERS, ACCESS2_UNIFORM_READ),
        TargetState::VertexInput => (
            LAYOUT_UNDEFINED,
            STAGE2_VERTEX_INPUT,
            ACCESS2_VERTEX_ATTRIBUTE_READ,
        ),
        TargetState::IndirectRead => (
            LAYOUT_UNDEFINED,
            STAGE2_DRAW_INDIRECT,
            ACCESS2_INDIRECT_COMMAND_READ,
        ),
    }
}

/// 跟踪态三元组(layout, stage2, access2);buffer 的 layout 槽恒 0。
type TrackedState = (u32, u64, u64);

/// 屏障表构造(host 可测纯函数):跟踪态 `from` → 目标 `to` 的转换项。
/// 已在目标态(三元组全等)→ `None`(幂等去重);否则 `Some((old_layout, new_layout,
/// src_stage, src_access, dst_stage, dst_access))`,执行器逐字灌入 barrier2 结构。
pub(crate) fn barrier_fields(
    from: TrackedState,
    to: TargetState,
) -> Option<(u32, u32, u64, u64, u64, u64)> {
    let (new_layout, dst_stage, dst_access) = state_fields(to);
    if from == (new_layout, dst_stage, dst_access) {
        None
    } else {
        Some((from.0, new_layout, from.1, from.2, dst_stage, dst_access))
    }
}

/// image 类目标态(类别纪律,校验用)。
fn is_image_state(s: TargetState) -> bool {
    matches!(
        s,
        TargetState::ColorAttachmentWrite
            | TargetState::DepthAttachmentWrite
            | TargetState::ShaderRead
            | TargetState::StorageImageReadWrite
            | TargetState::TransferSrc
            | TargetState::TransferDst
    )
}

/// buffer 类目标态(类别纪律,校验用)。
fn is_buffer_state(s: TargetState) -> bool {
    matches!(
        s,
        TargetState::ShaderRead
            | TargetState::TransferSrc
            | TargetState::TransferDst
            | TargetState::StorageWrite
            | TargetState::StorageReadWrite
            | TargetState::UniformRead
            | TargetState::VertexInput
            | TargetState::IndirectRead
    )
}

/// set0 descriptor 布局规划(host 可测纯函数;模块头「descriptor 固定约定」的
/// 可执行形态):返回 (binding, descriptor_type) 列,binding 号按区间分配。
/// A=0 委托等价形(既有四段布局逐位不变,`set0_layout_convention` 锚定)。
#[allow(dead_code)] // lib 侧消费已迁 plan_set0_layout_with_as;本形为测试锚 + 文档契约面。
pub(crate) fn plan_set0_layout(
    n_storage: u32,
    n_sampled: u32,
    n_storage_img: u32,
    has_uniform: bool,
) -> Vec<(u32, u32)> {
    plan_set0_layout_with_as(0, n_storage, n_sampled, n_storage_img, has_uniform)
}

/// [`plan_set0_layout`] 的 AS 扩面形(G7.6 Wave B):binding `[0..A)` =
/// accel struct(`DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR`),其后四段顺延
/// (AS 在前 = rurixc 形参声明序,`AccelStruct` 首形参拿 binding 0)。
pub(crate) fn plan_set0_layout_with_as(
    n_as: u32,
    n_storage: u32,
    n_sampled: u32,
    n_storage_img: u32,
    has_uniform: bool,
) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for a in 0..n_as {
        out.push((a, DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR));
    }
    for i in 0..n_storage {
        out.push((n_as + i, DESCRIPTOR_TYPE_STORAGE_BUFFER));
    }
    for j in 0..n_sampled {
        out.push((n_as + n_storage + j, DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER));
    }
    for k in 0..n_storage_img {
        out.push((
            n_as + n_storage + n_sampled + k,
            DESCRIPTOR_TYPE_STORAGE_IMAGE,
        ));
    }
    if has_uniform {
        out.push((
            n_as + n_storage + n_sampled + n_storage_img,
            DESCRIPTOR_TYPE_UNIFORM_BUFFER,
        ));
    }
    out
}

/// SPIR-V 字节 → u32 字流(4 字节对齐 + 最小头长 + magic 校验;host 可测)。
fn spirv_to_words(spv: &[u8], what: &str) -> Result<Vec<u32>, String> {
    if spv.len() < 20 || !spv.len().is_multiple_of(4) {
        return Err(format!(
            "{what}: SPIR-V 字节长度 {} 非法(须 ≥20 且 4 字节整倍)",
            spv.len()
        ));
    }
    let words: Vec<u32> = spv
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    if words[0] != 0x0723_0203 {
        return Err(format!("{what}: SPIR-V magic 须 0x07230203"));
    }
    Ok(words)
}

/// pass push constants 上限(Vulkan 保底 128;本执行器约定硬上限)。
const MAX_PUSH_CONSTANTS: usize = 128;

/// 帧级 host 预校验(P-01 fail-closed,在任何句柄创建前;host 可测纯函数)。
/// `execute_frame` 入口形:无 AS 表(as_count=0,`accel_structs` 非空即拒)。
fn validate_frame(
    resources: &[ResourceDesc],
    passes: &[Pass],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
) -> Result<(), String> {
    validate_frame_with_as(resources, passes, barriers, readbacks, 0)
}

/// [`validate_frame`] 的 AS 表扩面形(G7.6 Wave B):`as_count` = session AS 表项数,
/// `Bindings::accel_structs` 各下标须 `< as_count`(空表 → 非空绑定确定性拒)。
fn validate_frame_with_as(
    resources: &[ResourceDesc],
    passes: &[Pass],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
    as_count: u32,
) -> Result<(), String> {
    if passes.is_empty() {
        return Err("passes 为空(≥1 pass)".to_owned());
    }
    if barriers.len() != passes.len() {
        return Err(format!(
            "barriers 列数 {} ≠ passes 列数 {}(须一一对应)",
            barriers.len(),
            passes.len()
        ));
    }
    // 资源个体合法性。
    for (i, r) in resources.iter().enumerate() {
        match r {
            ResourceDesc::Buffer(b) => {
                if b.size < 4 {
                    return Err(format!("resources[{i}]: buffer size {} < 4", b.size));
                }
                if !b.usage.storage && !b.usage.uniform && !b.usage.vertex && !b.usage.indirect {
                    return Err(format!("resources[{i}]: buffer 须至少一个用途位"));
                }
                if let Some(d) = b.data
                    && d.len() as u64 > b.size
                {
                    return Err(format!(
                        "resources[{i}]: buffer 初始数据 {}B 超出 size {}B",
                        d.len(),
                        b.size
                    ));
                }
            }
            ResourceDesc::Texture(t) => {
                if t.width == 0 || t.height == 0 {
                    return Err(format!(
                        "resources[{i}]: 纹理尺寸须 ≥1({}x{})",
                        t.width, t.height
                    ));
                }
                if !t.usage.sampled && !t.usage.storage && !t.usage.color && !t.usage.depth {
                    return Err(format!("resources[{i}]: 纹理须至少一个用途位"));
                }
                if t.format.is_depth() {
                    if !t.usage.depth || t.usage.color || t.usage.storage {
                        return Err(format!(
                            "resources[{i}]: depth 格式仅允许 depth(+sampled)用途"
                        ));
                    }
                } else if t.usage.depth {
                    return Err(format!("resources[{i}]: 非 depth 格式不得声明 depth 用途"));
                }
                if let Some(d) = t.data {
                    let want = t.width as usize * t.height as usize * t.format.bytes_per_texel();
                    if d.len() != want {
                        return Err(format!(
                            "resources[{i}]: 纹理初始数据 {}B ≠ {want}B(w*h*bpp)",
                            d.len()
                        ));
                    }
                }
            }
        }
    }
    let is_texture = |idx: u32| -> bool {
        matches!(resources.get(idx as usize), Some(ResourceDesc::Texture(_)))
    };
    // 屏障表合法性(资源号越界 / 类别失配)。
    for (pi, table) in barriers.iter().enumerate() {
        for &(res, state) in *table {
            let Some(r) = resources.get(res as usize) else {
                return Err(format!(
                    "barriers[{pi}]: 资源号 {res} 越界(resources len={})",
                    resources.len()
                ));
            };
            match r {
                ResourceDesc::Buffer(_) if !is_buffer_state(state) => {
                    return Err(format!(
                        "barriers[{pi}]: 资源 {res} 为 buffer,目标态 {state:?} 为 image 类"
                    ));
                }
                ResourceDesc::Texture(_) if !is_image_state(state) => {
                    return Err(format!(
                        "barriers[{pi}]: 资源 {res} 为 texture,目标态 {state:?} 为 buffer 类"
                    ));
                }
                _ => {}
            }
        }
    }
    // 绑定集逐项合法性(公共子例程)。
    let validate_bindings = |name: &str, b: &Bindings| -> Result<(), String> {
        validate_bindings_with_as(name, b, resources, as_count)
    };
    // pass 个体合法性。
    for p in passes {
        match p {
            Pass::Raster(rp) => {
                spirv_to_words(rp.vs_spirv, &format!("pass `{}` vs", rp.name))?;
                spirv_to_words(rp.fs_spirv, &format!("pass `{}` fs", rp.name))?;
                if rp.colors.is_empty() && rp.depth.is_none() {
                    return Err(format!("pass `{}`: 须 ≥1 个 attachment", rp.name));
                }
                if rp.colors.len() > 8 {
                    return Err(format!(
                        "pass `{}`: color attachment 数 {} > 8",
                        rp.name,
                        rp.colors.len()
                    ));
                }
                let mut dims: Vec<(u32, u32)> = Vec::new();
                for ca in &rp.colors {
                    let Some(ResourceDesc::Texture(t)) = resources.get(ca.res as usize) else {
                        return Err(format!(
                            "pass `{}`: color attachment {} 非 texture",
                            rp.name, ca.res
                        ));
                    };
                    if !t.usage.color || t.format.is_depth() {
                        return Err(format!(
                            "pass `{}`: color attachment {} 须 color 用途 + 非 depth 格式",
                            rp.name, ca.res
                        ));
                    }
                    dims.push((t.width, t.height));
                }
                if let Some(d) = rp.depth {
                    let Some(ResourceDesc::Texture(t)) = resources.get(d.res as usize) else {
                        return Err(format!(
                            "pass `{}`: depth attachment {} 非 texture",
                            rp.name, d.res
                        ));
                    };
                    if !t.usage.depth || !t.format.is_depth() {
                        return Err(format!(
                            "pass `{}`: depth attachment {} 须 depth 用途 + depth 格式",
                            rp.name, d.res
                        ));
                    }
                    dims.push((t.width, t.height));
                }
                if dims.windows(2).any(|w| w[0] != w[1]) {
                    return Err(format!("pass `{}`: attachments 尺寸不一致", rp.name));
                }
                if let Some((vw, vh)) = rp.viewport
                    && (vw == 0 || vh == 0)
                {
                    return Err(format!("pass `{}`: viewport 尺寸须 ≥1", rp.name));
                }
                match &rp.vertex {
                    VertexData::Inline {
                        data,
                        stride,
                        attrs,
                    } => {
                        if *stride == 0 {
                            return Err(format!("pass `{}`: vertex stride 须 ≥1", rp.name));
                        }
                        if attrs.is_empty() {
                            return Err(format!("pass `{}`: inline VB 须 ≥1 顶点属性", rp.name));
                        }
                        if !data.len().is_multiple_of(*stride as usize) {
                            return Err(format!(
                                "pass `{}`: 顶点字节 {} 非 stride {} 整倍",
                                rp.name,
                                data.len(),
                                stride
                            ));
                        }
                    }
                    VertexData::Resource {
                        res, stride, attrs, ..
                    } => {
                        match resources.get(*res as usize) {
                            Some(ResourceDesc::Buffer(d)) if d.usage.vertex => {}
                            Some(ResourceDesc::Buffer(_)) => {
                                return Err(format!(
                                    "pass `{}`: vertex buffer {res} 未声明 vertex 用途",
                                    rp.name
                                ));
                            }
                            _ => {
                                return Err(format!(
                                    "pass `{}`: vertex buffer 资源号 {res} 非 buffer",
                                    rp.name
                                ));
                            }
                        }
                        if *stride == 0 || attrs.is_empty() {
                            return Err(format!(
                                "pass `{}`: vertex buffer 须 stride ≥1 且 ≥1 属性",
                                rp.name
                            ));
                        }
                    }
                    VertexData::Pull => {}
                }
                match rp.draw {
                    DrawSpec::Direct {
                        vertex_count,
                        instance_count,
                        ..
                    } => {
                        if vertex_count == 0 || instance_count == 0 {
                            return Err(format!("pass `{}`: draw 顶点数/实例数须 ≥1", rp.name));
                        }
                    }
                    DrawSpec::Indirect { res, .. } => match resources.get(res as usize) {
                        Some(ResourceDesc::Buffer(d)) if d.usage.indirect => {}
                        Some(ResourceDesc::Buffer(_)) => {
                            return Err(format!(
                                "pass `{}`: indirect buffer {res} 未声明 indirect 用途",
                                rp.name
                            ));
                        }
                        _ => {
                            return Err(format!(
                                "pass `{}`: indirect 资源号 {res} 非 buffer",
                                rp.name
                            ));
                        }
                    },
                }
                validate_bindings(rp.name, &rp.bindings)?;
            }
            Pass::Compute(cp) => {
                spirv_to_words(cp.spirv, &format!("pass `{}` cs", cp.name))?;
                if let Some(e) = cp.entry
                    && (e.is_empty() || e.contains('\0'))
                {
                    return Err(format!("pass `{}`: entry 名非法", cp.name));
                }
                match cp.dispatch {
                    DispatchSpec::Direct(g) => {
                        if g[0] == 0 || g[1] == 0 || g[2] == 0 {
                            return Err(format!("pass `{}`: dispatch 各维须 ≥1", cp.name));
                        }
                    }
                    DispatchSpec::Indirect { res, .. } => match resources.get(res as usize) {
                        Some(ResourceDesc::Buffer(d)) if d.usage.indirect => {}
                        Some(ResourceDesc::Buffer(_)) => {
                            return Err(format!(
                                "pass `{}`: indirect buffer {res} 未声明 indirect 用途",
                                cp.name
                            ));
                        }
                        _ => {
                            return Err(format!(
                                "pass `{}`: indirect 资源号 {res} 非 buffer",
                                cp.name
                            ));
                        }
                    },
                }
                validate_bindings(cp.name, &cp.bindings)?;
            }
        }
    }
    // readback 合法性。
    for (i, rb) in readbacks.iter().enumerate() {
        match *rb {
            Readback::Buffer { res, offset, size } => {
                let Some(ResourceDesc::Buffer(d)) = resources.get(res as usize) else {
                    return Err(format!("readbacks[{i}]: 资源号 {res} 非 buffer"));
                };
                if size == 0 || offset + size > d.size {
                    return Err(format!(
                        "readbacks[{i}]: 区段 [{offset}, {offset}+{size}) 越出 buffer size {}",
                        d.size
                    ));
                }
            }
            Readback::Texture { res } => {
                if !is_texture(res) {
                    return Err(format!("readbacks[{i}]: 资源号 {res} 非 texture"));
                }
            }
        }
    }
    Ok(())
}

/// 绑定集逐项合法性(`validate_frame_with_as` 提取的公共子例程;`accel_structs`
/// 各下标须 `< as_count`,`execute_frame`/空 AS 表 session 经 as_count=0 拒非空)。
fn validate_bindings_with_as(
    name: &str,
    b: &Bindings,
    resources: &[ResourceDesc],
    as_count: u32,
) -> Result<(), String> {
    for &as_index in &b.accel_structs {
        if as_index >= as_count {
            return Err(format!(
                "pass `{name}`: accel struct 下标 {as_index} 越界(session AS 表 {as_count} 项)"
            ));
        }
    }
    for &res in &b.storage_buffers {
        match resources.get(res as usize) {
            Some(ResourceDesc::Buffer(d)) if d.usage.storage => {}
            Some(ResourceDesc::Buffer(_)) => {
                return Err(format!(
                    "pass `{name}`: storage buffer {res} 未声明 storage 用途"
                ));
            }
            _ => {
                return Err(format!(
                    "pass `{name}`: storage buffer 资源号 {res} 非 buffer"
                ));
            }
        }
    }
    for &res in &b.sampled_images {
        match resources.get(res as usize) {
            Some(ResourceDesc::Texture(d)) if d.usage.sampled => {}
            Some(ResourceDesc::Texture(_)) => {
                return Err(format!(
                    "pass `{name}`: sampled image {res} 未声明 sampled 用途"
                ));
            }
            _ => {
                return Err(format!(
                    "pass `{name}`: sampled image 资源号 {res} 非 texture"
                ));
            }
        }
    }
    for &res in &b.storage_images {
        match resources.get(res as usize) {
            Some(ResourceDesc::Texture(d)) if d.usage.storage => {}
            Some(ResourceDesc::Texture(_)) => {
                return Err(format!(
                    "pass `{name}`: storage image {res} 未声明 storage 用途"
                ));
            }
            _ => {
                return Err(format!(
                    "pass `{name}`: storage image 资源号 {res} 非 texture"
                ));
            }
        }
    }
    if let Some(u) = b.uniform {
        match resources.get(u.res as usize) {
            Some(ResourceDesc::Buffer(d)) if d.usage.uniform => {
                if u.size == 0 || u.offset + u.size > d.size {
                    return Err(format!(
                        "pass `{name}`: uniform 区段 [{}, {}+{}) 越出 buffer size {}",
                        u.offset, u.offset, u.size, d.size
                    ));
                }
            }
            Some(ResourceDesc::Buffer(_)) => {
                return Err(format!(
                    "pass `{name}`: uniform {} 未声明 uniform 用途",
                    u.res
                ));
            }
            _ => return Err(format!("pass `{name}`: uniform 资源号 {} 非 buffer", u.res)),
        }
    }
    if b.push_constants.len() > MAX_PUSH_CONSTANTS {
        return Err(format!(
            "pass `{name}`: push constants {}B > {MAX_PUSH_CONSTANTS}B",
            b.push_constants.len()
        ));
    }
    Ok(())
}

/// 保守光栅 fail-closed 门(G7.5b,RXS-0303 L3;P-01):任一 raster pass 要求
/// `conservative=Some` 而设备无 `VK_EXT_conservative_rasterization`(caps 探测
/// `conservative_raster=None`)→ 确定性 `Err`,**在任何 pipeline 创建之前**——
/// 不静默降级、不静默跳过(降级臂须由编排层显式选择并写入 evidence,RFC-0016 §4.0-2)。
/// 纯 host 函数(caps 为入参),host 可测。
fn validate_conservative_raster(passes: &[Pass], caps: &DeviceCaps) -> Result<(), String> {
    if caps.conservative_raster.is_some() {
        return Ok(());
    }
    for (i, p) in passes.iter().enumerate() {
        if let Pass::Raster(rp) = p
            && rp.conservative.is_some()
        {
            return Err(format!(
                "passes[{i}] `{}`: 要求保守光栅化(conservative=Some)但设备 `{}` 无 \
                 VK_EXT_conservative_rasterization(RXS-0303 L3 fail-closed,不静默降级)",
                rp.name, caps.device_name
            ));
        }
    }
    Ok(())
}

/// pass 声明绑定集(单事实源;provenance/需求态/录制/override 基准共用)。
fn pass_bindings<'a>(p: &'a Pass<'a>) -> &'a Bindings {
    match p {
        Pass::Raster(rp) => &rp.bindings,
        Pass::Compute(cp) => &cp.bindings,
    }
}

/// pass 诊断名(provenance/校验报告用;hazard 推导不读名)。
fn pass_name<'a>(p: &'a Pass<'a>) -> &'a str {
    match p {
        Pass::Raster(rp) => rp.name,
        Pass::Compute(cp) => cp.name,
    }
}

/// G34-2(HZB 接统一车道)加性:双 TLAS 入口第二更新的校验面(与
/// [`validate_frame_update`] 同伴调用;`None` 恒过 = 既有面 0-byte)。
/// 判据(fail-closed 确定性 Err):AS 下标在界 + 与 `update.tlas_update`/
/// `update.blas_refit` **异槽**(单帧单槽单写纪律;同槽双写语义不在本面
/// 开放——双 TLAS 车道双表各自一槽)。
fn validate_tlas_update_b(
    as_count: u32,
    update: &FrameUpdate,
    tlas_b: Option<&(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
) -> Result<(), String> {
    let Some((as_index_b, _, _)) = tlas_b else {
        return Ok(());
    };
    if *as_index_b >= as_count {
        return Err(format!(
            "tlas_update_b: AS 下标 {as_index_b} 越界(session AS 表 {as_count} 项)"
        ));
    }
    if matches!(&update.tlas_update, Some((ai, _, _)) if *ai == *as_index_b) {
        return Err(format!(
            "tlas_update_b: as_index {as_index_b} 与 tlas_update 同槽(单帧单槽单写纪律)"
        ));
    }
    if matches!(&update.blas_refit, Some(b) if b.as_index == *as_index_b) {
        return Err(format!(
            "tlas_update_b: as_index {as_index_b} 与 blas_refit 同槽(单帧单槽单写纪律)"
        ));
    }
    Ok(())
}

/// G37 W3 hzb_skin 加性:第二 BLAS refit 校验面(与 [`validate_tlas_update_b`]
/// 同伴调用;`None` 恒过 = 既有面 0-byte)。判据(fail-closed 确定性 Err):
/// 须与 `tlas_update_b` 同现且同槽(单帧 AS 操作归并同一 manager——
/// `blas_refit`×`tlas_update` 同槽纪律的表 1 镜像);src 资源 buffer 区段/
/// byte_len 4 对齐/after_pass 在界(`validate_frame_update` blas 臂同律)。
fn validate_blas_refit_b(
    resources: &[ResourceDesc],
    passes: &[Pass],
    tlas_b: Option<&(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
    blas_b: Option<&BlasRefitUpdate>,
) -> Result<(), String> {
    let Some(b) = blas_b else {
        return Ok(());
    };
    let Some((as_index_b, _, _)) = tlas_b else {
        return Err("blas_refit_b: 须与 tlas_update_b 同现(表 1 manager 单帧操作归并面)".into());
    };
    if b.as_index != *as_index_b {
        return Err(format!(
            "blas_refit_b: as_index {} 与 tlas_update_b 的 {as_index_b} 不同槽(单帧 AS 操作归并同一 manager)",
            b.as_index
        ));
    }
    let index = b.src.0;
    if index == 0 || index > resources.len() as u64 {
        return Err(format!(
            "blas_refit_b: src StableResourceId({index}) 非资源表项(1..={})",
            resources.len()
        ));
    }
    let Some(ResourceDesc::Buffer(desc)) = resources.get((index - 1) as usize) else {
        return Err(format!("blas_refit_b: src StableResourceId({index}) 非 buffer"));
    };
    if b.byte_len == 0 || !b.byte_len.is_multiple_of(4) {
        return Err(format!(
            "blas_refit_b: byte_len {} 非 4 对齐正数(f32 顶点流)",
            b.byte_len
        ));
    }
    if b.src_offset + b.byte_len > desc.size {
        return Err(format!(
            "blas_refit_b: 区段 [{}, +{}) 越出 src buffer size {}",
            b.src_offset, b.byte_len, desc.size
        ));
    }
    if b.after_pass as usize >= passes.len() {
        return Err(format!(
            "blas_refit_b: after_pass {} 越界({} pass)",
            b.after_pass,
            passes.len()
        ));
    }
    Ok(())
}

/// G38 T3 加性:BLAS refit 桥扩展校验面(fail-closed 确定性 Err;`None` 恒过
/// = 既有面 0-byte)。判据:`copy_regions` 须与 `update.blas_refit` 同现;每段
/// (off,len) 4 对齐正段、升序不重叠、落在 refit 窗 `[0, byte_len)` 内(空列表
/// 合法 = 本帧无脏字节,桥接 copy 跳过)。`collect_gpu_timing` 无校验面
/// (观测旗标;无桥时静默无效)。host 可测纯函数。
fn validate_bridge_ext(
    update: &FrameUpdate,
    ext: Option<&BlasRefitBridgeExt>,
) -> Result<(), String> {
    let Some(e) = ext else {
        return Ok(());
    };
    let Some(regions) = &e.copy_regions else {
        return Ok(());
    };
    let Some(b) = &update.blas_refit else {
        return Err(
            "BlasRefitBridgeExt.copy_regions 须与 FrameUpdate.blas_refit 同现(无桥可分段)"
                .into(),
        );
    };
    let mut prev_end = 0u64;
    for (i, &(off, len)) in regions.iter().enumerate() {
        if len == 0 || !off.is_multiple_of(4) || !len.is_multiple_of(4) {
            return Err(format!(
                "BlasRefitBridgeExt.copy_regions[{i}] (off={off}, len={len}) 非 4 对齐正段(f32 顶点流)"
            ));
        }
        if off < prev_end {
            return Err(format!(
                "BlasRefitBridgeExt.copy_regions[{i}] off={off} 与前段末端 {prev_end} 重叠/乱序(须升序不重叠)"
            ));
        }
        let end = off
            .checked_add(len)
            .ok_or_else(|| format!("BlasRefitBridgeExt.copy_regions[{i}] off+len 溢出 u64"))?;
        if end > b.byte_len {
            return Err(format!(
                "BlasRefitBridgeExt.copy_regions[{i}] 末端 {end} 越出 refit 窗 byte_len {}",
                b.byte_len
            ));
        }
        prev_end = end;
    }
    Ok(())
}

/// [`FrameUpdate`] host 预校验(G7.6 Wave B;`execute_with_frame_update` 在任何
/// GPU 调用前 fail-closed;host 可测纯函数)。
fn validate_frame_update(
    resources: &[ResourceDesc],
    passes: &[Pass],
    readbacks: &[Readback],
    as_count: u32,
    update: &FrameUpdate,
) -> Result<(), String> {
    if let Some((as_index, _, _)) = &update.tlas_update
        && *as_index >= as_count
    {
        return Err(format!(
            "FrameUpdate.tlas_update: AS 下标 {as_index} 越界(session AS 表 {as_count} 项)"
        ));
    }
    // G31+ 波 B Task B5:blas_refit 校验(AS/blas 下标、src 资源 buffer 区段、
    // after_pass 越界、与 tlas_update 同槽一致——全部 fail-closed 确定性 Err)。
    if let Some(b) = &update.blas_refit {
        if b.as_index >= as_count {
            return Err(format!(
                "FrameUpdate.blas_refit: AS 下标 {} 越界(session AS 表 {as_count} 项)",
                b.as_index
            ));
        }
        if let Some((ai, _, _)) = &update.tlas_update
            && *ai != b.as_index
        {
            return Err(format!(
                "FrameUpdate.blas_refit: as_index {} 与 tlas_update 的 {ai} 不同槽(单帧 AS 操作归并同一 manager)",
                b.as_index
            ));
        }
        let index = b.src.0;
        if index == 0 || index > resources.len() as u64 {
            return Err(format!(
                "FrameUpdate.blas_refit: src StableResourceId({index}) 非资源表项(1..={})",
                resources.len()
            ));
        }
        let Some(ResourceDesc::Buffer(desc)) = resources.get((index - 1) as usize) else {
            return Err(format!(
                "FrameUpdate.blas_refit: src StableResourceId({index}) 非 buffer"
            ));
        };
        if b.byte_len == 0 || !b.byte_len.is_multiple_of(4) {
            return Err(format!(
                "FrameUpdate.blas_refit: byte_len {} 非 4 对齐正数(f32 顶点流)",
                b.byte_len
            ));
        }
        if b.src_offset + b.byte_len > desc.size {
            return Err(format!(
                "FrameUpdate.blas_refit: 区段 [{}, +{}) 越出 src buffer size {}",
                b.src_offset, b.byte_len, desc.size
            ));
        }
        if b.after_pass as usize >= passes.len() {
            return Err(format!(
                "FrameUpdate.blas_refit: after_pass {} 越界({} pass)",
                b.after_pass,
                passes.len()
            ));
        }
    }
    for (resource_id, offset, bytes) in &update.buffer_uploads {
        let index = resource_id.0;
        if index == 0 || index > resources.len() as u64 {
            return Err(format!(
                "FrameUpdate.buffer_uploads: StableResourceId({index}) 非资源表项(1..={})",
                resources.len()
            ));
        }
        let Some(ResourceDesc::Buffer(desc)) = resources.get((index - 1) as usize) else {
            return Err(format!(
                "FrameUpdate.buffer_uploads: StableResourceId({index}) 非 buffer"
            ));
        };
        if desc.device_local {
            return Err(format!(
                "FrameUpdate.buffer_uploads: StableResourceId({index}) 为 DEVICE_LOCAL 驻留\
                 (不可 map;上传目标须 host-visible——device_local=false,G14.10d fail-closed)"
            ));
        }
        if bytes.is_empty() {
            return Err("FrameUpdate.buffer_uploads: 空上传段(无意义,fail-closed)".into());
        }
        if offset + bytes.len() as u64 > desc.size {
            return Err(format!(
                "FrameUpdate.buffer_uploads: 区段 [{offset}, {offset}+{}) 越出 buffer size {}",
                bytes.len(),
                desc.size
            ));
        }
    }
    let mut overridden_passes: Vec<u32> = Vec::new();
    for (pass_index, bindings) in &update.binding_overrides {
        let Some(pass) = passes.get(*pass_index as usize) else {
            return Err(format!(
                "FrameUpdate.binding_overrides: pass 下标 {pass_index} 越界({} pass)",
                passes.len()
            ));
        };
        if overridden_passes.contains(pass_index) {
            return Err(format!(
                "FrameUpdate.binding_overrides: pass {pass_index} 重复覆盖(歧义,fail-closed)"
            ));
        }
        overridden_passes.push(*pass_index);
        validate_bindings_with_as(pass_name(pass), bindings, resources, as_count)?;
        let declared = pass_bindings(pass);
        let layout_key = |b: &Bindings| {
            (
                b.accel_structs.len(),
                b.storage_buffers.len(),
                b.sampled_images.len(),
                b.storage_images.len(),
                b.uniform.is_some(),
            )
        };
        if layout_key(bindings) != layout_key(declared) {
            return Err(format!(
                "FrameUpdate.binding_overrides: pass {pass_index} set0 布局键漂移(pipeline \
                 layout 创建期固化,禁异构覆盖)"
            ));
        }
        if bindings.push_constants.len() != declared.push_constants.len() {
            return Err(format!(
                "FrameUpdate.binding_overrides: pass {pass_index} push constants 长度 {} != 声明 {}",
                bindings.push_constants.len(),
                declared.push_constants.len()
            ));
        }
    }
    let mut pc_overridden: Vec<u32> = Vec::new();
    for (pass_index, bytes) in &update.push_constant_overrides {
        let Some(pass) = passes.get(*pass_index as usize) else {
            return Err(format!(
                "FrameUpdate.push_constant_overrides: pass 下标 {pass_index} 越界({} pass)",
                passes.len()
            ));
        };
        if pc_overridden.contains(pass_index) {
            return Err(format!(
                "FrameUpdate.push_constant_overrides: pass {pass_index} 重复覆盖(歧义,fail-closed)"
            ));
        }
        pc_overridden.push(*pass_index);
        let declared_len = pass_bindings(pass).push_constants.len();
        if bytes.len() != declared_len {
            return Err(format!(
                "FrameUpdate.push_constant_overrides: pass {pass_index} 长度 {} != 声明 {declared_len}",
                bytes.len()
            ));
        }
    }
    if let Some(subset) = &update.readback_subset {
        for (k, &index) in subset.iter().enumerate() {
            if index as usize >= readbacks.len() {
                return Err(format!(
                    "FrameUpdate.readback_subset[{k}]: 下标 {index} 越界({} readback)",
                    readbacks.len()
                ));
            }
            if subset[..k].contains(&index) {
                return Err(format!(
                    "FrameUpdate.readback_subset: 下标 {index} 重复(输出对齐歧义,fail-closed)"
                ));
            }
        }
    }
    Ok(())
}

/// buffer 用途位 → `VkBufferUsageFlags`(建面;传输位恒附加——初始上传/readback 需要)。
fn buffer_usage_flags(u: BufferUsage) -> u32 {
    let mut f = 0x1 | 0x2; // TRANSFER_SRC | TRANSFER_DST
    if u.uniform {
        f |= 0x10;
    }
    if u.storage {
        f |= 0x20;
    }
    if u.vertex {
        f |= 0x80;
    }
    if u.indirect {
        f |= 0x100;
    }
    f
}

/// texture 用途位 → `VkImageUsageFlags`(建面;传输位恒附加——上传/readback 需要)。
fn texture_usage_flags(u: TextureUsage) -> u32 {
    let mut f = 0x1 | 0x2; // TRANSFER_SRC | TRANSFER_DST
    if u.sampled {
        f |= 0x4;
    }
    if u.storage {
        f |= 0x8;
    }
    if u.color {
        f |= 0x10;
    }
    if u.depth {
        f |= 0x20;
    }
    f
}

/// pass 资源需求态(隐式补全事实源;plan 回放后按本表补齐未到态的转换)。
/// 返回 (资源下标, 目标态) 列;同一资源重复需求去重(首见为准)。
/// effective bindings 形(创建录制 = 声明绑定;G7.6 Wave B 重录路 = override 后;
/// accel structs 为 AS 句柄非资源表项,不产资源转换)。
fn pass_requirements_with(p: &Pass, b: &Bindings) -> Vec<(u32, TargetState)> {
    let mut out: Vec<(u32, TargetState)> = Vec::new();
    let mut push = |res: u32, s: TargetState| {
        if !out.iter().any(|&(r, _)| r == res) {
            out.push((res, s));
        }
    };
    fn bindings_requirements(b: &Bindings, push: &mut impl FnMut(u32, TargetState)) {
        for &res in &b.storage_buffers {
            push(res, TargetState::StorageReadWrite);
        }
        for &res in &b.sampled_images {
            push(res, TargetState::ShaderRead);
        }
        for &res in &b.storage_images {
            push(res, TargetState::StorageImageReadWrite);
        }
        if let Some(u) = b.uniform {
            push(u.res, TargetState::UniformRead);
        }
    }
    match p {
        Pass::Raster(rp) => {
            for ca in &rp.colors {
                push(ca.res, TargetState::ColorAttachmentWrite);
            }
            if let Some(d) = rp.depth {
                push(d.res, TargetState::DepthAttachmentWrite);
            }
            match &rp.vertex {
                VertexData::Resource { res, .. } => push(*res, TargetState::VertexInput),
                VertexData::Inline { .. } | VertexData::Pull => {}
            }
            if let DrawSpec::Indirect { res, .. } = rp.draw {
                push(res, TargetState::IndirectRead);
            }
            bindings_requirements(b, &mut push);
        }
        Pass::Compute(cp) => {
            if let DispatchSpec::Indirect { res, .. } = cp.dispatch {
                push(res, TargetState::IndirectRead);
            }
            bindings_requirements(b, &mut push);
        }
    }
    out
}

fn elapsed_ns(start: std::time::Instant) -> u64 {
    start.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
}

/// pass 的 read/write 分类事实源；只读结构字段，不读取诊断名。
/// effective bindings + AS 偏移植形(创建/provenance = 声明绑定;G7.6 Wave B 重录
/// = override 后):`accel_structs` 各表项产 (`as_offset + as_index`, Read)——ray
/// query pass 对 TLAS 的只读消费 provenance(AS 表项 StableResourceId =
/// resource_count + as_index + 1)。
fn pass_runtime_accesses_with(
    p: &Pass<'_>,
    bindings: &Bindings,
    as_offset: u32,
) -> Vec<(u32, RuntimeAccessKind)> {
    fn push(out: &mut Vec<(u32, RuntimeAccessKind)>, res: u32, access: RuntimeAccessKind) {
        if let Some((_, old)) = out.iter_mut().find(|(r, _)| *r == res) {
            *old = match (*old, access) {
                (RuntimeAccessKind::Read, RuntimeAccessKind::Read) => RuntimeAccessKind::Read,
                (RuntimeAccessKind::Write, RuntimeAccessKind::Write) => RuntimeAccessKind::Write,
                _ => RuntimeAccessKind::ReadWrite,
            };
        } else {
            out.push((res, access));
        }
    }
    fn bindings_accesses(out: &mut Vec<(u32, RuntimeAccessKind)>, b: &Bindings, as_offset: u32) {
        for &as_index in &b.accel_structs {
            push(out, as_offset + as_index, RuntimeAccessKind::Read);
        }
        for &res in &b.storage_buffers {
            push(out, res, RuntimeAccessKind::ReadWrite);
        }
        for &res in &b.sampled_images {
            push(out, res, RuntimeAccessKind::Read);
        }
        for &res in &b.storage_images {
            push(out, res, RuntimeAccessKind::ReadWrite);
        }
        if let Some(u) = b.uniform {
            push(out, u.res, RuntimeAccessKind::Read);
        }
    }
    let mut out: Vec<(u32, RuntimeAccessKind)> = Vec::new();
    match p {
        Pass::Raster(rp) => {
            for c in &rp.colors {
                push(&mut out, c.res, RuntimeAccessKind::Write);
            }
            if let Some(d) = rp.depth {
                push(&mut out, d.res, RuntimeAccessKind::Write);
            }
            if let VertexData::Resource { res, .. } = rp.vertex {
                push(&mut out, res, RuntimeAccessKind::Read);
            }
            if let DrawSpec::Indirect { res, .. } = rp.draw {
                push(&mut out, res, RuntimeAccessKind::Read);
            }
            bindings_accesses(&mut out, bindings, as_offset);
        }
        Pass::Compute(cp) => {
            if let DispatchSpec::Indirect { res, .. } = cp.dispatch {
                push(&mut out, res, RuntimeAccessKind::Read);
            }
            bindings_accesses(&mut out, bindings, as_offset);
        }
    }
    out
}

/// 声明绑定 + 无 AS 表形(既有测试锚;委托 [`build_runtime_provenance_ext`])。
#[allow(dead_code)] // lib 侧消费已迁 ext;本形保留为 host 测试锚。
fn build_runtime_provenance(
    resources: &[ResourceDesc<'_>],
    passes: &[Pass<'_>],
    allocations: &[StableAllocationId],
    generations: &[u64],
    frame_generation: u64,
) -> SubmissionProvenance {
    let declared: Vec<Bindings> = passes.iter().map(|p| pass_bindings(p).clone()).collect();
    build_runtime_provenance_ext(
        passes,
        &declared,
        allocations,
        generations,
        frame_generation,
        resources.len() as u32,
    )
}

/// G7.6 Wave B 扩面形:`effective_bindings` = 各 pass 本帧实际绑定(声明或 override);
/// `allocations`/`generations` 为**扩展表**(resources 后接 AS 表槽位,长度 =
/// resource_count + as_count);`as_offset` = resources 数(AS 表项 provenance
/// resource 下标 = as_offset + as_index)。
fn build_runtime_provenance_ext(
    passes: &[Pass<'_>],
    effective_bindings: &[Bindings],
    allocations: &[StableAllocationId],
    generations: &[u64],
    frame_generation: u64,
    as_offset: u32,
) -> SubmissionProvenance {
    debug_assert_eq!(passes.len(), effective_bindings.len());
    debug_assert_eq!(allocations.len(), generations.len());
    let mut current = generations.to_vec();
    let mut out = Vec::with_capacity(passes.len());
    for (pi, pass) in passes.iter().enumerate() {
        let name = pass_name(pass);
        let mut used = Vec::new();
        for (res, access) in pass_runtime_accesses_with(pass, &effective_bindings[pi], as_offset) {
            let idx = res as usize;
            let allocation_id = allocations[idx];
            let reads = matches!(
                access,
                RuntimeAccessKind::Read | RuntimeAccessKind::ReadWrite
            );
            let writes = matches!(
                access,
                RuntimeAccessKind::Write | RuntimeAccessKind::ReadWrite
            );
            let producer = reads.then_some(ProducerVersion {
                allocation_id,
                generation: current[idx],
            });
            let produced_generation = if writes {
                current[idx] = current[idx].saturating_add(1);
                Some(current[idx])
            } else {
                None
            };
            used.push(RuntimeResourceProvenance {
                resource_id: StableResourceId(idx as u64 + 1),
                allocation_id,
                access,
                producer,
                produced_generation,
            });
        }
        out.push(RuntimePassProvenance {
            pass_id: pi as u64 + 1,
            name: name.to_owned(),
            resources: used,
        });
    }
    SubmissionProvenance {
        frame_generation,
        passes: out,
    }
}

// ─────────────────────────── FFI 声明(布局复制 vk.rs 同源纪律,锚定单测) ───────────────────────────

type VkInstance = *mut c_void;
type VkPhysicalDevice = *mut c_void;
type VkDevice = *mut c_void;
type VkQueue = *mut c_void;
type VkCommandBuffer = *mut c_void;
type VkBuffer = u64;
type VkDeviceMemory = u64;
type VkImage = u64;
type VkImageView = u64;
type VkShaderModule = u64;
type VkDescriptorSetLayout = u64;
type VkPipelineLayout = u64;
type VkPipeline = u64;
type VkRenderPass = u64;
type VkFramebuffer = u64;
type VkDescriptorPool = u64;
type VkDescriptorSet = u64;
type VkSampler = u64;
type VkCommandPool = u64;
type VkFence = u64;
type VkQueryPool = u64;
type VkDebugUtilsMessengerEXT = u64;
type VkResult = i32;
type VkFlags = u32;
type VkDeviceSize = u64;

const VK_SUCCESS: VkResult = 0;
const VK_NULL_HANDLE: u64 = 0;
const WHOLE_SIZE: u64 = u64::MAX;

// sType(Vulkan 1.0/1.1 core + KHR;SDK 头核对)。
const ST_APPLICATION_INFO: u32 = 0;
const ST_INSTANCE_CREATE_INFO: u32 = 1;
const ST_DEVICE_QUEUE_CREATE_INFO: u32 = 2;
const ST_DEVICE_CREATE_INFO: u32 = 3;
const ST_SUBMIT_INFO: u32 = 4;
const ST_MEMORY_ALLOCATE_INFO: u32 = 5;
const ST_BUFFER_CREATE_INFO: u32 = 12;
const ST_IMAGE_CREATE_INFO: u32 = 14;
const ST_IMAGE_VIEW_CREATE_INFO: u32 = 15;
const ST_SHADER_MODULE_CREATE_INFO: u32 = 16;
const ST_PIPELINE_SHADER_STAGE_CREATE_INFO: u32 = 18;
const ST_PIPELINE_VERTEX_INPUT_STATE_CI: u32 = 19;
const ST_PIPELINE_INPUT_ASSEMBLY_STATE_CI: u32 = 20;
const ST_PIPELINE_VIEWPORT_STATE_CI: u32 = 22;
const ST_PIPELINE_RASTERIZATION_STATE_CI: u32 = 23;
const ST_PIPELINE_MULTISAMPLE_STATE_CI: u32 = 24;
const ST_PIPELINE_DEPTH_STENCIL_STATE_CI: u32 = 25;
const ST_PIPELINE_COLOR_BLEND_STATE_CI: u32 = 26;
const ST_GRAPHICS_PIPELINE_CREATE_INFO: u32 = 28;
const ST_COMPUTE_PIPELINE_CREATE_INFO: u32 = 29;
const ST_PIPELINE_LAYOUT_CREATE_INFO: u32 = 30;
const ST_DESCRIPTOR_SET_LAYOUT_CREATE_INFO: u32 = 32;
const ST_DESCRIPTOR_POOL_CREATE_INFO: u32 = 33;
const ST_DESCRIPTOR_SET_ALLOCATE_INFO: u32 = 34;
const ST_WRITE_DESCRIPTOR_SET: u32 = 35;
const ST_FRAMEBUFFER_CREATE_INFO: u32 = 37;
const ST_RENDER_PASS_CREATE_INFO: u32 = 38;
const ST_COMMAND_POOL_CREATE_INFO: u32 = 39;
const ST_COMMAND_BUFFER_ALLOCATE_INFO: u32 = 40;
const ST_COMMAND_BUFFER_BEGIN_INFO: u32 = 42;
const ST_RENDER_PASS_BEGIN_INFO: u32 = 43;
const ST_QUERY_POOL_CREATE_INFO: u32 = 11;
const ST_FENCE_CREATE_INFO: u32 = 8;
const ST_PHYSICAL_DEVICE_MEMORY_PROPERTIES_2: u32 = 1_000_059_006;
const ST_PHYSICAL_DEVICE_MEMORY_BUDGET_PROPERTIES_EXT: u32 = 1_000_237_000;
const ST_SAMPLER_CREATE_INFO: u32 = 31;
const ST_PHYSICAL_DEVICE_FEATURES_2: u32 = 1_000_059_000;
/// `VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2`(Vulkan 1.1 core;保守光栅属性
/// 链查询的链头,G7.5b RXS-0303 IR1)。
const ST_PHYSICAL_DEVICE_PROPERTIES_2: u32 = 1_000_059_001;
/// `VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_CONSERVATIVE_RASTERIZATION_PROPERTIES_EXT`
/// (`VK_EXT_conservative_rasterization` 扩展 #102 → 1000101000;本机 SDK 1.3.296
/// `vulkan_core.h` 核对,非凭记忆)。
const ST_PHYSICAL_DEVICE_CONSERVATIVE_RASTERIZATION_PROPERTIES_EXT: u32 = 1_000_101_000;
/// `VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_CONSERVATIVE_STATE_CREATE_INFO_EXT`
/// (同扩展 → 1000101001;pipeline 光栅状态 pNext 链节点,RXS-0303 L2)。
const ST_PIPELINE_RASTERIZATION_CONSERVATIVE_STATE_CI_EXT: u32 = 1_000_101_001;
/// `VK_CONSERVATIVE_RASTERIZATION_MODE_OVERESTIMATE_EXT`(=1;RXS-0303 L2 钉死
/// OVERESTIMATE——覆盖超集方向是唯一需要硬件担保的性质,RFC-0018 §E2)。
const CONSERVATIVE_RASTERIZATION_MODE_OVERESTIMATE: u32 = 1;
const ST_PHYSICAL_DEVICE_SHADER_ATOMIC_INT64_FEATURES: u32 = 1_000_180_000;
const ST_PHYSICAL_DEVICE_SYNCHRONIZATION_2_FEATURES: u32 = 1_000_314_007;
const ST_PHYSICAL_DEVICE_DESCRIPTOR_INDEXING_FEATURES: u32 = 1_000_161_001;
const ST_PHYSICAL_DEVICE_ACCELERATION_STRUCTURE_FEATURES_KHR: u32 = 1_000_150_013;

// ── G14.10b external memory 导出面(RFC-0030 §4.3;vendor 输入驻留)──
// sType 值经 SDK 1.3.296 `vulkan_core.h` 核对(扩展号:VK_KHR_external_memory_
// capabilities #72 → 1000071xxx;VK_KHR_external_memory #73 → 1000072xxx;
// VK_KHR_external_memory_win32 #74 → 1000073xxx;VK_KHR_dedicated_allocation
// #128 → 1000127xxx;均 Vulkan 1.1 core 收编,win32 仍为扩展)。
/// `VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES`(1.1 core;deviceLUID 载体)。
const ST_PHYSICAL_DEVICE_ID_PROPERTIES: u32 = 1_000_071_004;
/// `VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_BUFFER_CREATE_INFO`(G14.10f buffer
/// 共享面)。
const ST_EXTERNAL_MEMORY_BUFFER_CREATE_INFO: u32 = 1_000_072_000;
/// `VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO`。
const ST_EXTERNAL_MEMORY_IMAGE_CREATE_INFO: u32 = 1_000_072_001;
/// `VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO`。
const ST_EXPORT_MEMORY_ALLOCATE_INFO: u32 = 1_000_072_002;
/// `VK_STRUCTURE_TYPE_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR`(导入侧;G14.11
/// 起产品面消费——D3D12_RESOURCE 反向导入;单测跨 device 闭环同用)。
const ST_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR: u32 = 1_000_073_000;
/// `VK_STRUCTURE_TYPE_MEMORY_WIN32_HANDLE_PROPERTIES_KHR`(G14.11:导入 handle
/// 的 memoryTypeBits 查询出参;`vkGetMemoryWin32HandlePropertiesKHR`)。
const ST_MEMORY_WIN32_HANDLE_PROPERTIES_KHR: u32 = 1_000_073_002;
/// `VK_STRUCTURE_TYPE_MEMORY_GET_WIN32_HANDLE_INFO_KHR`。
const ST_MEMORY_GET_WIN32_HANDLE_INFO_KHR: u32 = 1_000_073_003;
/// `VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO`(1.1 core;NVIDIA 上
/// Win32 导出/导入 image 常强制 dedicated,必挂)。
const ST_MEMORY_DEDICATED_ALLOCATE_INFO: u32 = 1_000_127_001;
/// `VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_BIT`(= 0x2,NT handle;位 0x1
/// 是 OPAQUE_FD——初版误用 0x1 被 validation 层 VUID-00990/00656/00664 三连抓,
/// SDK 1.3.296 核对后钉死 0x2)。同 LUID 物理设备的两个 VkDevice 间共享;
/// D3D12 OpenSharedHandle **不可**消费此类句柄,FSR D3D12 臂需反向
/// D3D12→Vulkan 导入路线,见 G14.10b 登记。
const EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32: u32 = 0x2;
/// `VK_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE_BIT`(= 0x20;G14.11 反向
/// 共享:D3D12 `CreateSharedHandle` 产出的 resource NT handle 经
/// `VkImportMemoryWin32HandleInfoKHR` 导入——G14.10b 已证 OPAQUE_WIN32 正向
/// 不可被 D3D12 消费,本位是唯一可行方向;SDK 1.3.296 `vulkan_core.h` 核对)。
const EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE: u32 = 0x20;
/// `VK_QUEUE_FAMILY_EXTERNAL`(= ~1u32;跨 device release/acquire 家族哨兵)。
const QUEUE_FAMILY_EXTERNAL: u32 = !1u32;
const ST_PHYSICAL_DEVICE_BUFFER_DEVICE_ADDRESS_FEATURES: u32 = 1_000_257_000;
const ST_PHYSICAL_DEVICE_RAY_QUERY_FEATURES_KHR: u32 = 1_000_348_013;
const ST_MEMORY_BARRIER_2: u32 = 1_000_314_000;
const ST_BUFFER_MEMORY_BARRIER_2: u32 = 1_000_314_001;
const ST_IMAGE_MEMORY_BARRIER_2: u32 = 1_000_314_002;
const ST_DEPENDENCY_INFO: u32 = 1_000_314_003;
const ST_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT: u32 = 1_000_128_004;
/// G31+ 波 C Task C7:`VkDebugUtilsLabelEXT` sType（pass 标注面）。
const ST_DEBUG_UTILS_LABEL_EXT: u32 = 1_000_128_002;

const QUEUE_GRAPHICS_BIT: u32 = 0x1;
const MEM_DEVICE_LOCAL: u32 = 0x1;
const MEM_HOST_VISIBLE: u32 = 0x2;
const MEM_HOST_COHERENT: u32 = 0x4;
/// `VK_MEMORY_PROPERTY_HOST_CACHED_BIT`(G14plus RFC-0030 §4.3 L1:readback
/// 用途缓冲优选;vendor_upscale.rs DLSS readback 同型先例)。
const MEM_HOST_CACHED: u32 = 0x8;
const SHARING_MODE_EXCLUSIVE: u32 = 0;
/// instance apiVersion = Vulkan 1.3(`VK_KHR_synchronization2` 于 1.3 收编 core;
/// 1.1 实例下本机 loader 的 feature 链查询不填 sync2/atomic int64——实测怪癖,
/// 镜像 vk.rs bindless 用 1.2 实例的先例,本执行器硬依赖 sync2 故直接 1.3)。
const API_VERSION_1_3: u32 = (1 << 22) | (3 << 12);
const API_VERSION_1_2: u32 = (1 << 22) | (2 << 12);
const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;

const IMAGE_TYPE_2D: u32 = 1;
const IMAGE_VIEW_TYPE_2D: u32 = 1;
const IMAGE_TILING_OPTIMAL: u32 = 0;
const IMAGE_ASPECT_COLOR: u32 = 0x1;

const DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER: u32 = 1;
const DESCRIPTOR_TYPE_STORAGE_IMAGE: u32 = 3;
const DESCRIPTOR_TYPE_UNIFORM_BUFFER: u32 = 6;
const DESCRIPTOR_TYPE_STORAGE_BUFFER: u32 = 7;
/// `VK_DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR`(SDK 1.3.296 `vulkan_core.h`;
/// 与 vk.rs 同源同值,禁第二套逻辑定义——结构体经 `WriteDescriptorSetAccelStructure`
/// pub(crate) 复用)。
const DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR: u32 = 1_000_150_000;
/// `VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET_ACCELERATION_STRUCTURE_KHR`(同vk.rs 源)。
const ST_WRITE_DESCRIPTOR_SET_ACCELERATION_STRUCTURE_KHR: u32 = 1_000_150_007;
/// `VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT`(经典 32 位 stage 掩码;AS consume
/// barrier 的 dst stage,与 vk.rs `PIPELINE_STAGE_COMPUTE_SHADER` 同源同值)。
const PIPELINE_STAGE_COMPUTE_SHADER: u32 = 0x800;

const SHADER_STAGE_VERTEX: u32 = 0x1;
const SHADER_STAGE_FRAGMENT: u32 = 0x10;
const SHADER_STAGE_COMPUTE: u32 = 0x20;
const SHADER_STAGE_RFX: u32 = SHADER_STAGE_VERTEX | SHADER_STAGE_FRAGMENT | SHADER_STAGE_COMPUTE;

const PIPELINE_BIND_POINT_GRAPHICS: u32 = 0;
const PIPELINE_BIND_POINT_COMPUTE: u32 = 1;
const PRIMITIVE_TOPOLOGY_TRIANGLE_LIST: u32 = 3;
const SUBPASS_CONTENTS_INLINE: u32 = 0;
const SAMPLE_COUNT_1: u32 = 0x1;
const POLYGON_MODE_FILL: u32 = 0;
const CULL_MODE_NONE: u32 = 0;
const FRONT_FACE_COUNTER_CLOCKWISE: u32 = 0;
const VERTEX_INPUT_RATE_VERTEX: u32 = 0;
const COMPONENT_SWIZZLE_IDENTITY: u32 = 0;
const COLOR_COMPONENT_RGBA: u32 = 0xF;
const COMPARE_OP_LESS_OR_EQUAL: u32 = 3;

const ATTACHMENT_LOAD_OP_LOAD: u32 = 0;
const ATTACHMENT_LOAD_OP_CLEAR: u32 = 1;
const ATTACHMENT_STORE_OP_STORE: u32 = 0;

const CMD_BUFFER_LEVEL_PRIMARY: u32 = 0;
const CMD_BUFFER_USAGE_ONE_TIME_SUBMIT: u32 = 0x1;
/// `VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT`(持久 session 每帧重录的
/// `vkResetCommandBuffer` VUID-00046 要求;ephemeral 路恒 flags=0 不变)。
const CMD_POOL_CREATE_RESET_COMMAND_BUFFER_BIT: u32 = 0x2;
const FENCE_CREATE_SIGNALED: u32 = 0x1;
const QUERY_TYPE_TIMESTAMP: u32 = 2;
const QUERY_RESULT_64_BIT: u32 = 0x1;
const QUERY_RESULT_WAIT_BIT: u32 = 0x2;
const VK_TIMEOUT: VkResult = 2;
const VK_ERROR_DEVICE_LOST: VkResult = -4;
const STAGE2_ALL_COMMANDS: u64 = 0x0001_0000;

const FILTER_LINEAR: u32 = 1;
const SAMPLER_MIPMAP_MODE_NEAREST: u32 = 0;
const SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE: u32 = 2;

const DEBUG_UTILS_SEVERITY_ERROR: u32 = 0x0000_1000;
const DEBUG_UTILS_TYPE_GENERAL: u32 = 0x1;
const DEBUG_UTILS_TYPE_VALIDATION: u32 = 0x2;
const DEBUG_UTILS_TYPE_PERFORMANCE: u32 = 0x4;

// ── #[repr(C)] 结构(布局与 Vulkan spec 逐字节对齐;ffi_layout_anchors 锚定) ──

#[repr(C)]
struct ApplicationInfo {
    s_type: u32,
    p_next: *const c_void,
    p_application_name: *const c_char,
    application_version: u32,
    p_engine_name: *const c_char,
    engine_version: u32,
    api_version: u32,
}

#[repr(C)]
struct InstanceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    p_application_info: *const ApplicationInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const c_char,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const c_char,
}

#[repr(C)]
struct VkExtent3D {
    width: u32,
    height: u32,
    depth: u32,
}

#[repr(C)]
struct QueueFamilyProperties {
    queue_flags: VkFlags,
    queue_count: u32,
    timestamp_valid_bits: u32,
    min_image_transfer_granularity: VkExtent3D,
}

#[repr(C)]
struct DeviceQueueCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    queue_family_index: u32,
    queue_count: u32,
    p_queue_priorities: *const f32,
}

#[repr(C)]
struct DeviceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    queue_create_info_count: u32,
    p_queue_create_infos: *const DeviceQueueCreateInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const c_char,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const c_char,
    p_enabled_features: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryType {
    property_flags: VkFlags,
    heap_index: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryHeap {
    size: VkDeviceSize,
    flags: VkFlags,
}

#[repr(C)]
struct PhysicalDeviceMemoryProperties {
    memory_type_count: u32,
    memory_types: [MemoryType; 32],
    memory_heap_count: u32,
    memory_heaps: [MemoryHeap; 16],
}

#[repr(C)]
struct BufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    size: VkDeviceSize,
    usage: VkFlags,
    sharing_mode: u32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
}

#[repr(C)]
struct MemoryRequirements {
    size: VkDeviceSize,
    alignment: VkDeviceSize,
    memory_type_bits: u32,
}

#[repr(C)]
struct MemoryAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    allocation_size: VkDeviceSize,
    memory_type_index: u32,
}

#[repr(C)]
struct ImageCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    image_type: u32,
    format: u32,
    extent: VkExtent3D,
    mip_levels: u32,
    array_layers: u32,
    samples: VkFlags,
    tiling: u32,
    usage: VkFlags,
    sharing_mode: u32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
    initial_layout: u32,
}

#[repr(C)]
struct VkComponentMapping {
    r: u32,
    g: u32,
    b: u32,
    a: u32,
}

#[repr(C)]
struct VkImageSubresourceRange {
    aspect_mask: VkFlags,
    base_mip_level: u32,
    level_count: u32,
    base_array_layer: u32,
    layer_count: u32,
}

#[repr(C)]
struct VkImageSubresourceLayers {
    aspect_mask: VkFlags,
    mip_level: u32,
    base_array_layer: u32,
    layer_count: u32,
}

#[repr(C)]
struct ImageViewCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    image: VkImage,
    view_type: u32,
    format: u32,
    components: VkComponentMapping,
    subresource_range: VkImageSubresourceRange,
}

#[repr(C)]
struct VkOffset2D {
    x: i32,
    y: i32,
}

#[repr(C)]
struct VkOffset3D {
    x: i32,
    y: i32,
    z: i32,
}

#[repr(C)]
struct VkRect2D {
    offset: VkOffset2D,
    extent: VkExtent2D,
}

#[repr(C)]
struct VkExtent2D {
    width: u32,
    height: u32,
}

#[repr(C)]
struct VkViewport {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    min_depth: f32,
    max_depth: f32,
}

#[repr(C)]
struct ShaderModuleCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    code_size: usize,
    p_code: *const u32,
}

#[repr(C)]
struct DescriptorSetLayoutBinding {
    binding: u32,
    descriptor_type: u32,
    descriptor_count: u32,
    stage_flags: VkFlags,
    p_immutable_samplers: *const c_void,
}

#[repr(C)]
struct DescriptorSetLayoutCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    binding_count: u32,
    p_bindings: *const DescriptorSetLayoutBinding,
}

#[repr(C)]
struct PushConstantRange {
    stage_flags: VkFlags,
    offset: u32,
    size: u32,
}

#[repr(C)]
struct PipelineLayoutCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    set_layout_count: u32,
    p_set_layouts: *const VkDescriptorSetLayout,
    push_constant_range_count: u32,
    p_push_constant_ranges: *const PushConstantRange,
}

#[repr(C)]
struct PipelineShaderStageCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    stage: VkFlags,
    module: VkShaderModule,
    p_name: *const c_char,
    p_specialization_info: *const c_void,
}

#[repr(C)]
struct ComputePipelineCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    stage: PipelineShaderStageCreateInfo,
    layout: VkPipelineLayout,
    base_pipeline_handle: VkPipeline,
    base_pipeline_index: i32,
}

#[repr(C)]
struct DescriptorPoolSize {
    descriptor_type: u32,
    descriptor_count: u32,
}

#[repr(C)]
struct DescriptorPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    max_sets: u32,
    pool_size_count: u32,
    p_pool_sizes: *const DescriptorPoolSize,
}

#[repr(C)]
struct DescriptorSetAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    descriptor_pool: VkDescriptorPool,
    descriptor_set_count: u32,
    p_set_layouts: *const VkDescriptorSetLayout,
}

#[repr(C)]
struct DescriptorBufferInfo {
    buffer: VkBuffer,
    offset: VkDeviceSize,
    range: VkDeviceSize,
}

#[repr(C)]
struct DescriptorImageInfo {
    sampler: VkSampler,
    image_view: VkImageView,
    image_layout: u32,
}

#[repr(C)]
struct WriteDescriptorSet {
    s_type: u32,
    p_next: *const c_void,
    dst_set: VkDescriptorSet,
    dst_binding: u32,
    dst_array_element: u32,
    descriptor_count: u32,
    descriptor_type: u32,
    p_image_info: *const DescriptorImageInfo,
    p_buffer_info: *const DescriptorBufferInfo,
    p_texel_buffer_view: *const c_void,
}

#[repr(C)]
struct CommandPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    queue_family_index: u32,
}

#[repr(C)]
struct CommandBufferAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    command_pool: VkCommandPool,
    level: u32,
    command_buffer_count: u32,
}

#[repr(C)]
struct CommandBufferBeginInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    p_inheritance_info: *const c_void,
}

#[repr(C)]
struct FenceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
}

#[repr(C)]
struct QueryPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    query_type: u32,
    query_count: u32,
    pipeline_statistics: VkFlags,
}

#[repr(C)]
struct PhysicalDeviceMemoryProperties2 {
    s_type: u32,
    p_next: *mut c_void,
    memory_properties: PhysicalDeviceMemoryProperties,
}

#[repr(C)]
struct PhysicalDeviceMemoryBudgetPropertiesExt {
    s_type: u32,
    p_next: *mut c_void,
    heap_budget: [VkDeviceSize; 16],
    heap_usage: [VkDeviceSize; 16],
}

#[repr(C)]
struct SubmitInfo {
    s_type: u32,
    p_next: *const c_void,
    wait_semaphore_count: u32,
    p_wait_semaphores: *const u64,
    p_wait_dst_stage_mask: *const VkFlags,
    command_buffer_count: u32,
    p_command_buffers: *const VkCommandBuffer,
    signal_semaphore_count: u32,
    p_signal_semaphores: *const u64,
}

#[repr(C)]
struct AttachmentDescription {
    flags: VkFlags,
    format: u32,
    samples: VkFlags,
    load_op: u32,
    store_op: u32,
    stencil_load_op: u32,
    stencil_store_op: u32,
    initial_layout: u32,
    final_layout: u32,
}

#[repr(C)]
struct AttachmentReference {
    attachment: u32,
    layout: u32,
}

#[repr(C)]
struct SubpassDescription {
    flags: VkFlags,
    pipeline_bind_point: u32,
    input_attachment_count: u32,
    p_input_attachments: *const AttachmentReference,
    color_attachment_count: u32,
    p_color_attachments: *const AttachmentReference,
    p_resolve_attachments: *const AttachmentReference,
    p_depth_stencil_attachment: *const AttachmentReference,
    preserve_attachment_count: u32,
    p_preserve_attachments: *const u32,
}

#[repr(C)]
struct RenderPassCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    attachment_count: u32,
    p_attachments: *const AttachmentDescription,
    subpass_count: u32,
    p_subpasses: *const SubpassDescription,
    dependency_count: u32,
    p_dependencies: *const c_void,
}

#[repr(C)]
struct FramebufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    render_pass: VkRenderPass,
    attachment_count: u32,
    p_attachments: *const VkImageView,
    width: u32,
    height: u32,
    layers: u32,
}

#[repr(C)]
struct VkVertexInputBindingDescription {
    binding: u32,
    stride: u32,
    input_rate: u32,
}

#[repr(C)]
struct VkVertexInputAttributeDescription {
    location: u32,
    binding: u32,
    format: u32,
    offset: u32,
}

#[repr(C)]
struct PipelineVertexInputStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    vertex_binding_description_count: u32,
    p_vertex_binding_descriptions: *const VkVertexInputBindingDescription,
    vertex_attribute_description_count: u32,
    p_vertex_attribute_descriptions: *const VkVertexInputAttributeDescription,
}

#[repr(C)]
struct PipelineInputAssemblyStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    topology: u32,
    primitive_restart_enable: u32,
}

#[repr(C)]
struct PipelineViewportStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    viewport_count: u32,
    p_viewports: *const VkViewport,
    scissor_count: u32,
    p_scissors: *const VkRect2D,
}

#[repr(C)]
struct PipelineRasterizationStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    depth_clamp_enable: u32,
    rasterizer_discard_enable: u32,
    polygon_mode: u32,
    cull_mode: VkFlags,
    front_face: u32,
    depth_bias_enable: u32,
    depth_bias_constant_factor: f32,
    depth_bias_clamp: f32,
    depth_bias_slope_factor: f32,
    line_width: f32,
}

#[repr(C)]
struct PipelineMultisampleStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    rasterization_samples: VkFlags,
    sample_shading_enable: u32,
    min_sample_shading: f32,
    p_sample_mask: *const u32,
    alpha_to_coverage_enable: u32,
    alpha_to_one_enable: u32,
}

#[repr(C)]
struct PipelineColorBlendAttachmentState {
    blend_enable: u32,
    src_color_blend_factor: u32,
    dst_color_blend_factor: u32,
    color_blend_op: u32,
    src_alpha_blend_factor: u32,
    dst_alpha_blend_factor: u32,
    alpha_blend_op: u32,
    color_write_mask: VkFlags,
}

#[repr(C)]
struct PipelineColorBlendStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    logic_op_enable: u32,
    logic_op: u32,
    attachment_count: u32,
    p_attachments: *const PipelineColorBlendAttachmentState,
    blend_constants: [f32; 4],
}

#[repr(C)]
struct PipelineDepthStencilStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    depth_test_enable: u32,
    depth_write_enable: u32,
    depth_compare_op: u32,
    depth_bounds_test_enable: u32,
    stencil_test_enable: u32,
    front: [u32; 7],
    back: [u32; 7],
    min_depth_bounds: f32,
    max_depth_bounds: f32,
}

#[repr(C)]
struct GraphicsPipelineCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    stage_count: u32,
    p_stages: *const PipelineShaderStageCreateInfo,
    p_vertex_input_state: *const PipelineVertexInputStateCreateInfo,
    p_input_assembly_state: *const PipelineInputAssemblyStateCreateInfo,
    p_tessellation_state: *const c_void,
    p_viewport_state: *const PipelineViewportStateCreateInfo,
    p_rasterization_state: *const PipelineRasterizationStateCreateInfo,
    p_multisample_state: *const PipelineMultisampleStateCreateInfo,
    p_depth_stencil_state: *const PipelineDepthStencilStateCreateInfo,
    p_color_blend_state: *const PipelineColorBlendStateCreateInfo,
    p_dynamic_state: *const c_void,
    layout: VkPipelineLayout,
    render_pass: VkRenderPass,
    subpass: u32,
    base_pipeline_handle: VkPipeline,
    base_pipeline_index: i32,
}

/// `VkClearValue` union(color float32[4] / depthStencil {f32,u32} 同 16 字节槽)。
#[repr(C)]
struct ClearValue {
    color: [f32; 4],
}

#[repr(C)]
struct RenderPassBeginInfo {
    s_type: u32,
    p_next: *const c_void,
    render_pass: VkRenderPass,
    framebuffer: VkFramebuffer,
    render_area: VkRect2D,
    clear_value_count: u32,
    p_clear_values: *const ClearValue,
}

#[repr(C)]
struct VkBufferImageCopy {
    buffer_offset: VkDeviceSize,
    buffer_row_length: u32,
    buffer_image_height: u32,
    image_subresource: VkImageSubresourceLayers,
    image_offset: VkOffset3D,
    image_extent: VkExtent3D,
}

#[repr(C)]
struct SamplerCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    mag_filter: u32,
    min_filter: u32,
    mipmap_mode: u32,
    address_mode_u: u32,
    address_mode_v: u32,
    address_mode_w: u32,
    mip_lod_bias: f32,
    anisotropy_enable: u32,
    max_anisotropy: f32,
    compare_enable: u32,
    compare_op: u32,
    min_lod: f32,
    max_lod: f32,
    border_color: u32,
    unnormalized_coordinates: u32,
}

// ── synchronization2 + feature 探测结构(SDK 头核对 sType) ──

/// `VkMemoryBarrier2`(全局 memory barrier;G14plus FIF 流水帧间守卫 /
/// staged copy 冲刷专用——pass 图内资源级转换仍走 buffer/image barrier2,
/// 不混用)。
#[repr(C)]
struct MemoryBarrier2 {
    s_type: u32,
    p_next: *const c_void,
    src_stage_mask: u64,
    src_access_mask: u64,
    dst_stage_mask: u64,
    dst_access_mask: u64,
}

/// `VkBufferCopy`(staged 上传 / staged buffer readback 的 buffer→buffer 区段)。
#[repr(C)]
struct VkBufferCopy {
    src_offset: u64,
    dst_offset: u64,
    size: u64,
}

#[repr(C)]
struct BufferMemoryBarrier2 {
    s_type: u32,
    p_next: *const c_void,
    src_stage_mask: u64,
    src_access_mask: u64,
    dst_stage_mask: u64,
    dst_access_mask: u64,
    src_queue_family_index: u32,
    dst_queue_family_index: u32,
    buffer: VkBuffer,
    offset: u64,
    size: u64,
}

#[repr(C)]
struct ImageMemoryBarrier2 {
    s_type: u32,
    p_next: *const c_void,
    src_stage_mask: u64,
    src_access_mask: u64,
    dst_stage_mask: u64,
    dst_access_mask: u64,
    old_layout: u32,
    new_layout: u32,
    src_queue_family_index: u32,
    dst_queue_family_index: u32,
    image: VkImage,
    subresource_range: VkImageSubresourceRange,
}

#[repr(C)]
struct DependencyInfo {
    s_type: u32,
    p_next: *const c_void,
    dependency_flags: u32,
    memory_barrier_count: u32,
    p_memory_barriers: *const c_void,
    buffer_memory_barrier_count: u32,
    p_buffer_memory_barriers: *const BufferMemoryBarrier2,
    image_memory_barrier_count: u32,
    p_image_memory_barriers: *const ImageMemoryBarrier2,
}

#[repr(C)]
struct PhysicalDeviceSynchronization2Features {
    s_type: u32,
    p_next: *mut c_void,
    synchronization2: u32,
}

#[repr(C)]
struct PhysicalDeviceShaderAtomicInt64Features {
    s_type: u32,
    p_next: *mut c_void,
    shader_buffer_int64_atomics: u32,
    shader_shared_int64_atomics: u32,
}

#[repr(C)]
struct PhysicalDeviceRayQueryFeatures {
    s_type: u32,
    p_next: *mut c_void,
    ray_query: u32,
}

#[repr(C)]
struct PhysicalDeviceAccelerationStructureFeatures {
    s_type: u32,
    p_next: *mut c_void,
    acceleration_structure: u32,
    acceleration_structure_capture_replay: u32,
    acceleration_structure_indirect_build: u32,
    acceleration_structure_host_commands: u32,
    descriptor_binding_acceleration_structure_update_after_bind: u32,
}

#[repr(C)]
struct PhysicalDeviceBufferDeviceAddressFeatures {
    s_type: u32,
    p_next: *mut c_void,
    buffer_device_address: u32,
    buffer_device_address_capture_replay: u32,
    buffer_device_address_multi_device: u32,
}

/// `VkPhysicalDeviceDescriptorIndexingFeatures` 的 20 个 `VkBool32` 按 spec 字段序建模；
/// `runtimeDescriptorArray` 为末槽(索引 19)。
#[repr(C)]
struct PhysicalDeviceDescriptorIndexingFeatures {
    s_type: u32,
    p_next: *mut c_void,
    bits: [u32; 20],
}

/// `VkPhysicalDeviceFeatures2`(features 本体 55 个 VkBool32 定长槽,仅作 pNext 链头,
/// 不读字段——与 vk.rs bindless 先例同律)。
#[repr(C)]
struct PhysicalDeviceFeatures2 {
    s_type: u32,
    p_next: *mut c_void,
    features: [u32; 55],
}

/// `VkPhysicalDeviceProperties` 承载 blob(真实结构严格超集;vk.rs bindless 2048 字节
/// blob 先例同律)。读取偏移见 `read_physical_caps` 注释(driverVersion 字段与 limits
/// align 8 补齐均在位,勿再按「apiVersion 后紧跟 vendorID」推算)。
#[repr(C, align(8))]
struct PropertiesBlob {
    bytes: [u8; 2048],
}

/// `VkPhysicalDeviceProperties2`(sType@0 / pNext@8 / properties@16;properties 槽以
/// [`PropertiesBlob`] 超集承载,仅作 pNext 链头,本体字段不在此读——与 vk.rs
/// `PhysicalDeviceProperties2Rt` 同律)。G7.5b 保守光栅属性链查询(RXS-0303 IR1)。
#[repr(C)]
struct PhysicalDeviceProperties2Chain {
    s_type: u32,
    p_next: *mut c_void,
    properties: PropertiesBlob,
}

/// `VkPhysicalDeviceConservativeRasterizationPropertiesEXT`(SDK 1.3.296 `vulkan_core.h`
/// 字段序逐一对齐:sType@0 / pNext@8 / 3×f32@16..28 / 6×VkBool32@28..52,size 56 align 8;
/// 布局锚定见 `ffi_layout_anchors`)。G7.5b RXS-0303 IR1 全字段读回,四项进 [`DeviceCaps`]。
#[repr(C)]
struct PhysicalDeviceConservativeRasterizationProperties {
    s_type: u32,
    p_next: *mut c_void,
    primitive_overestimation_size: f32,
    max_extra_primitive_overestimation_size: f32,
    extra_primitive_overestimation_size_granularity: f32,
    primitive_underestimation: u32,
    conservative_point_and_line_rasterization: u32,
    degenerate_triangles_rasterized: u32,
    degenerate_lines_rasterized: u32,
    fully_covered_fragment_shader_input_variable: u32,
    conservative_rasterization_post_depth_coverage: u32,
}

/// `VkPipelineRasterizationConservativeStateCreateInfoEXT`(sType@0 / pNext@8 /
/// flags@16 / mode@20 / extraPrimitiveOverestimationSize@24,size 32 align 8;
/// 布局锚定见 `ffi_layout_anchors`)。RXS-0303 L2:raster pass `conservative=Some`
/// 时挂入 [`PipelineRasterizationStateCreateInfo::p_next`],mode 恒 OVERESTIMATE。
#[repr(C)]
struct PipelineRasterizationConservativeStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    conservative_rasterization_mode: u32,
    extra_primitive_overestimation_size: f32,
}

/// `VkExtensionProperties`(char[256] + u32)。
#[repr(C)]
#[derive(Clone, Copy)]
struct ExtensionProperties {
    extension_name: [c_char; 256],
    spec_version: u32,
}

// ── G14.10b external memory 导出面结构(SDK 1.3.296 `vulkan_core.h` 字段序
// 逐一核对;布局锚定见 `ffi_layout_anchors`)──

/// `VkPhysicalDeviceIDProperties`(1.1 core;sType@0 / pNext@8 / deviceUUID[16]@16 /
/// driverUUID[16]@32 / deviceLUID[8]@48 / deviceNodeMask@56 / deviceLUIDValid@60,
/// size 64 align 8)。G14.10b 同 physical device 前置断言的 LUID 事实源。
#[repr(C)]
struct PhysicalDeviceIDProperties {
    s_type: u32,
    p_next: *mut c_void,
    device_uuid: [u8; 16],
    driver_uuid: [u8; 16],
    device_luid: [u8; 8],
    device_node_mask: u32,
    device_luid_valid: u32,
}

/// `VkExternalMemoryImageCreateInfo`(sType@0 / pNext@8 / handleTypes@16,
/// size 24 align 8)。挂 [`ImageCreateInfo::p_next`] 声明该 image 内存可导出。
#[repr(C)]
struct ExternalMemoryImageCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    handle_types: u32,
}

/// `VkExternalMemoryBufferCreateInfo`(布局同 image 版;sType = 1000072000)。
/// 挂 [`BufferCreateInfo::p_next`] 声明该 buffer 内存可导出(G14.10f buffer
/// 共享面:跨 device OPTIMAL image 布局解释不一致的正解——buffer 线性无歧义)。
#[repr(C)]
struct ExternalMemoryBufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    handle_types: u32,
}

/// `VkExportMemoryAllocateInfo`(sType@0 / pNext@8 / handleTypes@16,size 24)。
/// 挂 [`MemoryAllocateInfo::p_next`] 声明该分配可经 win32 handle 导出。
#[repr(C)]
struct ExportMemoryAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    handle_types: u32,
}

/// `VkMemoryDedicatedAllocateInfo`(sType@0 / pNext@8 / image@16 / buffer@24,
/// size 32)。NVIDIA 上 OPAQUE_WIN32 image 导出/导入实务上强制 dedicated。
#[repr(C)]
struct MemoryDedicatedAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    image: VkImage,
    buffer: VkBuffer,
}

/// `VkMemoryGetWin32HandleInfoKHR`(sType@0 / pNext@8 / memory@16 / handleType@24,
/// size 32)。`vkGetMemoryWin32HandleKHR` 入参。
#[repr(C)]
struct MemoryGetWin32HandleInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    memory: VkDeviceMemory,
    handle_type: u32,
}

/// `VkImportMemoryWin32HandleInfoKHR`(sType@0 / pNext@8 / handleType@16 /
/// handle@24 / name@32,size 40)。导入侧(G14.11 D3D12_RESOURCE 反向导入
/// 产品面 + 单测跨 device 闭环;DLSS 侧 vendor_upscale.rs 持同族独立定义
/// ——两模块 FFI 自足纪律)。
#[repr(C)]
struct ImportMemoryWin32HandleInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    handle_type: u32,
    handle: *mut c_void,
    name: *const u16,
}

/// `VkMemoryWin32HandlePropertiesKHR`(sType@0 / pNext@8 / memoryTypeBits@16,
/// size 24)。G14.11:导入 D3D12 resource handle 的兼容内存类型查询出参。
#[repr(C)]
struct MemoryWin32HandlePropertiesKHR {
    s_type: u32,
    p_next: *mut c_void,
    memory_type_bits: u32,
}

type FnGetMemoryWin32HandleKHR = unsafe extern "system" fn(
    VkDevice,
    *const MemoryGetWin32HandleInfoKHR,
    *mut *mut c_void,
) -> VkResult;

/// `vkGetMemoryWin32HandlePropertiesKHR`(G14.11;OPAQUE 类 handle 不可查,
/// D3D12_RESOURCE 可查——VUID-00666)。
type FnGetMemoryWin32HandlePropertiesKHR = unsafe extern "system" fn(
    VkDevice,
    u32,
    *mut c_void,
    *mut MemoryWin32HandlePropertiesKHR,
) -> VkResult;

// kernel32 `CloseHandle`(导出 NT handle 归 session 所有,Drop 单点关闭——
// 调用方不得 CloseHandle,防双关)。
#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CloseHandle(handle: *mut c_void) -> i32;
}

// ── debug messenger(U27 同律 fail-closed) ──

type PfnDebugUtilsMessengerCallback = unsafe extern "system" fn(
    u32,
    u32,
    *const DebugUtilsMessengerCallbackDataEXT,
    *mut c_void,
) -> u32;

#[repr(C)]
struct DebugUtilsMessengerCreateInfoEXT {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    message_severity: u32,
    message_type: u32,
    pfn_user_callback: PfnDebugUtilsMessengerCallback,
    p_user_data: *mut c_void,
}

#[repr(C)]
struct DebugUtilsMessengerCallbackDataEXT {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    p_message_id_name: *const c_char,
    message_id_number: i32,
    p_message: *const c_char,
    queue_label_count: u32,
    p_queue_labels: *const c_void,
    cmd_buf_label_count: u32,
    p_cmd_buf_labels: *const c_void,
    object_count: u32,
    p_objects: *const c_void,
}

/// 进程内 validation ERROR 累计数(本模块**全部** messenger 回调共同累加)。
///
/// `execute_frame` 的 fail-closed 判据用的是栈上 `AtomicBool`(一帧内有无 ERROR),
/// 但调用方(门/证据 JSON)需要**真实计数**而不是写死的 0 —— 故此处再记一份进程级
/// 计数,经 [`validation_error_total`] 暴露。
static VALIDATION_ERROR_TOTAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 本进程是否**确实**装上了 validation layer + debug messenger。
///
/// 无此位,`validation_error_total() == 0` 无法区分「真跑零 ERROR」与「layer 没装
/// 上,压根没人报」—— 后者是假绿。门须同时要求本位为真。
static VALIDATION_MESSENGER_INSTALLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// 进程内 validation ERROR 累计数(messenger 回调实数;未装 messenger 恒 0)。
#[must_use]
pub fn validation_error_total() -> u64 {
    VALIDATION_ERROR_TOTAL.load(std::sync::atomic::Ordering::Relaxed)
}

/// 本进程是否成功创建过 debug messenger(= ERROR 计数可信)。
#[must_use]
pub fn validation_messenger_installed() -> bool {
    VALIDATION_MESSENGER_INSTALLED.load(std::sync::atomic::Ordering::Relaxed)
}

/// ERROR 级校验消息 → 置栈上 `AtomicBool` + stderr(U27 回调同律;返回 VK_FALSE 不中断)。
unsafe extern "system" fn debug_messenger_cb(
    severity: u32,
    _types: u32,
    data: *const DebugUtilsMessengerCallbackDataEXT,
    user_data: *mut c_void,
) -> u32 {
    if severity & DEBUG_UTILS_SEVERITY_ERROR != 0 {
        VALIDATION_ERROR_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if !user_data.is_null() {
            // SAFETY: user_data 指向 execute_frame_inner 栈上 AtomicBool;messenger 生命周期
            // 严格短于该栈变量(末尾 instance destroy 前销毁)。原子写经共享引用合法。
            let flag = &*(user_data as *const std::sync::atomic::AtomicBool);
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if !data.is_null() {
            // SAFETY: 回调契约保证 data 在回调期间有效;p_message 为有效 NUL 结尾 C 串。
            let d = &*data;
            if !d.p_message.is_null() {
                let msg = CStr::from_ptr(d.p_message).to_string_lossy();
                eprintln!("[vk-validation] {msg}");
            }
        }
    }
    0
}

/// 持久 session ERROR 计数回调；`user_data` 指向 Box 内稳定 `AtomicU64`。
unsafe extern "system" fn persistent_debug_messenger_cb(
    severity: u32,
    _types: u32,
    data: *const DebugUtilsMessengerCallbackDataEXT,
    user_data: *mut c_void,
) -> u32 {
    if severity & DEBUG_UTILS_SEVERITY_ERROR != 0 {
        VALIDATION_ERROR_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if !user_data.is_null() {
            // SAFETY: create_persistent_frame 在 messenger 前分配 Box<AtomicU64>，Box 内容地址
            // 跨 NativePersistentFrame 移动稳定，Drop 在销毁 messenger 后才释放 Box。
            let count = &*(user_data as *const std::sync::atomic::AtomicU64);
            count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if !data.is_null() {
            let d = &*data;
            if !d.p_message.is_null() {
                let msg = CStr::from_ptr(d.p_message).to_string_lossy();
                eprintln!("[vk-validation] {msg}");
            }
        }
    }
    0
}

// ── 函数指针类型 ──

type FnCreateInstance = unsafe extern "system" fn(
    *const InstanceCreateInfo,
    *const c_void,
    *mut VkInstance,
) -> VkResult;
type FnDestroyInstance = unsafe extern "system" fn(VkInstance, *const c_void);
type FnEnumeratePhysicalDevices =
    unsafe extern "system" fn(VkInstance, *mut u32, *mut VkPhysicalDevice) -> VkResult;
type FnGetPhysicalDeviceQueueFamilyProperties =
    unsafe extern "system" fn(VkPhysicalDevice, *mut u32, *mut QueueFamilyProperties);
type FnGetPhysicalDeviceMemoryProperties =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDeviceMemoryProperties);
type FnGetPhysicalDeviceMemoryProperties2 =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDeviceMemoryProperties2);
type FnGetPhysicalDeviceProperties =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PropertiesBlob);
type FnGetPhysicalDeviceProperties2 =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDeviceProperties2Chain);
type FnGetPhysicalDeviceFeatures2 =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDeviceFeatures2);
type FnEnumerateDeviceExtensionProperties = unsafe extern "system" fn(
    VkPhysicalDevice,
    *const c_char,
    *mut u32,
    *mut ExtensionProperties,
) -> VkResult;
/// G31+ 波 C Task C7:instance 扩展枚举(VK_EXT_debug_utils 在位判定;loader 层)。
type FnEnumerateInstanceExtensionProperties =
    unsafe extern "system" fn(*const c_char, *mut u32, *mut ExtensionProperties) -> VkResult;
type FnCreateDevice = unsafe extern "system" fn(
    VkPhysicalDevice,
    *const DeviceCreateInfo,
    *const c_void,
    *mut VkDevice,
) -> VkResult;
type FnDestroyDevice = unsafe extern "system" fn(VkDevice, *const c_void);
type FnGetDeviceProcAddr = unsafe extern "system" fn(VkDevice, *const c_char) -> Option<PfnVoid>;
type FnGetDeviceQueue = unsafe extern "system" fn(VkDevice, u32, u32, *mut VkQueue);
type FnCreateBuffer = unsafe extern "system" fn(
    VkDevice,
    *const BufferCreateInfo,
    *const c_void,
    *mut VkBuffer,
) -> VkResult;
type FnDestroyBuffer = unsafe extern "system" fn(VkDevice, VkBuffer, *const c_void);
type FnGetBufferMemoryRequirements =
    unsafe extern "system" fn(VkDevice, VkBuffer, *mut MemoryRequirements);
type FnAllocateMemory = unsafe extern "system" fn(
    VkDevice,
    *const MemoryAllocateInfo,
    *const c_void,
    *mut VkDeviceMemory,
) -> VkResult;
type FnFreeMemory = unsafe extern "system" fn(VkDevice, VkDeviceMemory, *const c_void);
type FnBindBufferMemory =
    unsafe extern "system" fn(VkDevice, VkBuffer, VkDeviceMemory, VkDeviceSize) -> VkResult;
type FnMapMemory = unsafe extern "system" fn(
    VkDevice,
    VkDeviceMemory,
    VkDeviceSize,
    VkDeviceSize,
    VkFlags,
    *mut *mut c_void,
) -> VkResult;
type FnUnmapMemory = unsafe extern "system" fn(VkDevice, VkDeviceMemory);
type FnCreateImage = unsafe extern "system" fn(
    VkDevice,
    *const ImageCreateInfo,
    *const c_void,
    *mut VkImage,
) -> VkResult;
type FnDestroyImage = unsafe extern "system" fn(VkDevice, VkImage, *const c_void);
type FnGetImageMemoryRequirements =
    unsafe extern "system" fn(VkDevice, VkImage, *mut MemoryRequirements);
type FnBindImageMemory =
    unsafe extern "system" fn(VkDevice, VkImage, VkDeviceMemory, VkDeviceSize) -> VkResult;
type FnCreateImageView = unsafe extern "system" fn(
    VkDevice,
    *const ImageViewCreateInfo,
    *const c_void,
    *mut VkImageView,
) -> VkResult;
type FnDestroyImageView = unsafe extern "system" fn(VkDevice, VkImageView, *const c_void);
type FnCreateShaderModule = unsafe extern "system" fn(
    VkDevice,
    *const ShaderModuleCreateInfo,
    *const c_void,
    *mut VkShaderModule,
) -> VkResult;
type FnDestroyShaderModule = unsafe extern "system" fn(VkDevice, VkShaderModule, *const c_void);
type FnCreateDescriptorSetLayout = unsafe extern "system" fn(
    VkDevice,
    *const DescriptorSetLayoutCreateInfo,
    *const c_void,
    *mut VkDescriptorSetLayout,
) -> VkResult;
type FnDestroyDescriptorSetLayout =
    unsafe extern "system" fn(VkDevice, VkDescriptorSetLayout, *const c_void);
type FnCreatePipelineLayout = unsafe extern "system" fn(
    VkDevice,
    *const PipelineLayoutCreateInfo,
    *const c_void,
    *mut VkPipelineLayout,
) -> VkResult;
type FnDestroyPipelineLayout = unsafe extern "system" fn(VkDevice, VkPipelineLayout, *const c_void);
type FnCreateComputePipelines = unsafe extern "system" fn(
    VkDevice,
    u64,
    u32,
    *const ComputePipelineCreateInfo,
    *const c_void,
    *mut VkPipeline,
) -> VkResult;
type FnCreateGraphicsPipelines = unsafe extern "system" fn(
    VkDevice,
    u64,
    u32,
    *const GraphicsPipelineCreateInfo,
    *const c_void,
    *mut VkPipeline,
) -> VkResult;
type FnDestroyPipeline = unsafe extern "system" fn(VkDevice, VkPipeline, *const c_void);
type FnCreateRenderPass = unsafe extern "system" fn(
    VkDevice,
    *const RenderPassCreateInfo,
    *const c_void,
    *mut VkRenderPass,
) -> VkResult;
type FnDestroyRenderPass = unsafe extern "system" fn(VkDevice, VkRenderPass, *const c_void);
type FnCreateFramebuffer = unsafe extern "system" fn(
    VkDevice,
    *const FramebufferCreateInfo,
    *const c_void,
    *mut VkFramebuffer,
) -> VkResult;
type FnDestroyFramebuffer = unsafe extern "system" fn(VkDevice, VkFramebuffer, *const c_void);
type FnCreateDescriptorPool = unsafe extern "system" fn(
    VkDevice,
    *const DescriptorPoolCreateInfo,
    *const c_void,
    *mut VkDescriptorPool,
) -> VkResult;
type FnDestroyDescriptorPool = unsafe extern "system" fn(VkDevice, VkDescriptorPool, *const c_void);
type FnAllocateDescriptorSets = unsafe extern "system" fn(
    VkDevice,
    *const DescriptorSetAllocateInfo,
    *mut VkDescriptorSet,
) -> VkResult;
type FnUpdateDescriptorSets =
    unsafe extern "system" fn(VkDevice, u32, *const WriteDescriptorSet, u32, *const c_void);
type FnCreateSampler = unsafe extern "system" fn(
    VkDevice,
    *const SamplerCreateInfo,
    *const c_void,
    *mut VkSampler,
) -> VkResult;
type FnDestroySampler = unsafe extern "system" fn(VkDevice, VkSampler, *const c_void);
type FnCreateCommandPool = unsafe extern "system" fn(
    VkDevice,
    *const CommandPoolCreateInfo,
    *const c_void,
    *mut VkCommandPool,
) -> VkResult;
type FnDestroyCommandPool = unsafe extern "system" fn(VkDevice, VkCommandPool, *const c_void);
type FnAllocateCommandBuffers = unsafe extern "system" fn(
    VkDevice,
    *const CommandBufferAllocateInfo,
    *mut VkCommandBuffer,
) -> VkResult;
type FnBeginCommandBuffer =
    unsafe extern "system" fn(VkCommandBuffer, *const CommandBufferBeginInfo) -> VkResult;
type FnEndCommandBuffer = unsafe extern "system" fn(VkCommandBuffer) -> VkResult;
type FnResetCommandBuffer = unsafe extern "system" fn(VkCommandBuffer, VkFlags) -> VkResult;
type FnFreeCommandBuffers =
    unsafe extern "system" fn(VkDevice, VkCommandPool, u32, *const VkCommandBuffer);
type FnCmdBindPipeline = unsafe extern "system" fn(VkCommandBuffer, u32, VkPipeline);
type FnCmdBindDescriptorSets = unsafe extern "system" fn(
    VkCommandBuffer,
    u32,
    VkPipelineLayout,
    u32,
    u32,
    *const VkDescriptorSet,
    u32,
    *const u32,
);
type FnCmdPushConstants =
    unsafe extern "system" fn(VkCommandBuffer, VkPipelineLayout, VkFlags, u32, u32, *const c_void);
type FnCmdDispatch = unsafe extern "system" fn(VkCommandBuffer, u32, u32, u32);
type FnCmdDispatchIndirect = unsafe extern "system" fn(VkCommandBuffer, VkBuffer, VkDeviceSize);
type FnCmdBeginRenderPass =
    unsafe extern "system" fn(VkCommandBuffer, *const RenderPassBeginInfo, u32);
type FnCmdEndRenderPass = unsafe extern "system" fn(VkCommandBuffer);
type FnCmdBindVertexBuffers =
    unsafe extern "system" fn(VkCommandBuffer, u32, u32, *const VkBuffer, *const VkDeviceSize);
type FnCmdDraw = unsafe extern "system" fn(VkCommandBuffer, u32, u32, u32, u32);
type FnCmdDrawIndirect =
    unsafe extern "system" fn(VkCommandBuffer, VkBuffer, VkDeviceSize, u32, u32);
type FnCmdPipelineBarrier2 = unsafe extern "system" fn(VkCommandBuffer, *const DependencyInfo);
type FnCmdCopyImageToBuffer = unsafe extern "system" fn(
    VkCommandBuffer,
    VkImage,
    u32,
    VkBuffer,
    u32,
    *const VkBufferImageCopy,
);
type FnCmdCopyBufferToImage = unsafe extern "system" fn(
    VkCommandBuffer,
    VkBuffer,
    VkImage,
    u32,
    u32,
    *const VkBufferImageCopy,
);
type FnCmdCopyBuffer =
    unsafe extern "system" fn(VkCommandBuffer, VkBuffer, VkBuffer, u32, *const VkBufferCopy);
type FnQueueSubmit =
    unsafe extern "system" fn(VkQueue, u32, *const SubmitInfo, VkFence) -> VkResult;
type FnQueueWaitIdle = unsafe extern "system" fn(VkQueue) -> VkResult;
type FnCreateFence = unsafe extern "system" fn(
    VkDevice,
    *const FenceCreateInfo,
    *const c_void,
    *mut VkFence,
) -> VkResult;
type FnDestroyFence = unsafe extern "system" fn(VkDevice, VkFence, *const c_void);
type FnWaitForFences =
    unsafe extern "system" fn(VkDevice, u32, *const VkFence, u32, u64) -> VkResult;
type FnResetFences = unsafe extern "system" fn(VkDevice, u32, *const VkFence) -> VkResult;
type FnCreateQueryPool = unsafe extern "system" fn(
    VkDevice,
    *const QueryPoolCreateInfo,
    *const c_void,
    *mut VkQueryPool,
) -> VkResult;
type FnDestroyQueryPool = unsafe extern "system" fn(VkDevice, VkQueryPool, *const c_void);
type FnCmdResetQueryPool = unsafe extern "system" fn(VkCommandBuffer, VkQueryPool, u32, u32);
type FnCmdWriteTimestamp2 = unsafe extern "system" fn(VkCommandBuffer, u64, VkQueryPool, u32);
type FnGetQueryPoolResults = unsafe extern "system" fn(
    VkDevice,
    VkQueryPool,
    u32,
    u32,
    usize,
    *mut c_void,
    VkDeviceSize,
    VkFlags,
) -> VkResult;
type FnCreateDebugUtilsMessengerEXT = unsafe extern "system" fn(
    VkInstance,
    *const DebugUtilsMessengerCreateInfoEXT,
    *const c_void,
    *mut VkDebugUtilsMessengerEXT,
) -> VkResult;
type FnDestroyDebugUtilsMessengerEXT =
    unsafe extern "system" fn(VkInstance, VkDebugUtilsMessengerEXT, *const c_void);

// ── G31+ 波 C Task C7:debug label 标注面(VK_EXT_debug_utils 在位才装载; ──
// ── Nsight/RenderDoc 逐 pass 可辨识;符号缺失 = 录制零开销跳过 fail-silent) ──

type FnCmdBeginDebugUtilsLabelEXT =
    unsafe extern "system" fn(VkCommandBuffer, *const DebugUtilsLabelEXT);
type FnCmdEndDebugUtilsLabelEXT = unsafe extern "system" fn(VkCommandBuffer);

/// `VkDebugUtilsLabelEXT`(sType@0 / pNext@8 / pLabelName@16 / color[4]@24,
/// size 40 align 8;SDK 1.3.296 `vulkan_core.h` 字段序)。
#[repr(C)]
struct DebugUtilsLabelEXT {
    s_type: u32,
    p_next: *const c_void,
    p_label_name: *const c_char,
    color: [f32; 4],
}

// ─────────────────────────── device 执行体 ───────────────────────────

/// instance 创建(validation 层 + debug ext 按 `RURIX_VK_VALIDATION=1` 装载;U27 同律)。
/// G31+ 波 C Task C7:VK_EXT_debug_utils 枚举在位即一并启用(validation 关也在位——
/// 逐 pass debug label 标注面,Nsight/RenderDoc 消费;枚举符号缺失/扩展 absent →
/// fail-silent 不启用,录制侧零开销跳过,不崩)。返回 (instance, validation 开?,
/// debug_utils 扩展已启用?)。
unsafe fn create_instance(
    gipa: FnGetInstanceProcAddr,
    app_name: &CStr,
) -> Result<(VkInstance, bool, bool), String> {
    let vk_create_instance: FnCreateInstance =
        cast_fn(gipa(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
            .ok_or("缺 vkCreateInstance")?;
    let validation = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    let layer_name = c"VK_LAYER_KHRONOS_validation";
    let layers: [*const c_char; 1] = [layer_name.as_ptr()];
    let debug_ext = c"VK_EXT_debug_utils";
    // G31+ 波 C Task C7:instance 扩展枚举(loader 层枚举,validation 关也可判定;
    // 枚举符号缺失/调用失败 = 保守不启用,fail-silent)。
    let mut debug_utils_listed = false;
    if let Some(vk_enum_iext) = cast_fn::<FnEnumerateInstanceExtensionProperties>(gipa(
        std::ptr::null_mut(),
        c"vkEnumerateInstanceExtensionProperties".as_ptr(),
    )) {
        let mut ext_count = 0u32;
        if vk_enum_iext(std::ptr::null(), &mut ext_count, std::ptr::null_mut()) == VK_SUCCESS {
            let mut exts_list = vec![
                ExtensionProperties {
                    extension_name: [0; 256],
                    spec_version: 0,
                };
                ext_count as usize
            ];
            if vk_enum_iext(std::ptr::null(), &mut ext_count, exts_list.as_mut_ptr()) == VK_SUCCESS
            {
                debug_utils_listed = exts_list.iter().any(|e| {
                    // SAFETY: loader 写入的 extensionName 为 256 字节定长槽内 NUL 结尾 C 串。
                    let bytes = unsafe { CStr::from_ptr(e.extension_name.as_ptr()) }.to_bytes();
                    bytes == debug_ext.to_bytes()
                });
            }
        }
    }
    let enable_debug_ext = validation || debug_utils_listed;
    let exts: [*const c_char; 1] = [debug_ext.as_ptr()];
    let app = ApplicationInfo {
        s_type: ST_APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: app_name.as_ptr(),
        application_version: 0,
        p_engine_name: c"rurix".as_ptr(),
        engine_version: 0,
        api_version: API_VERSION_1_3,
    };
    let ici = InstanceCreateInfo {
        s_type: ST_INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_application_info: &app,
        enabled_layer_count: u32::from(validation),
        pp_enabled_layer_names: if validation {
            layers.as_ptr()
        } else {
            std::ptr::null()
        },
        enabled_extension_count: u32::from(enable_debug_ext),
        pp_enabled_extension_names: if enable_debug_ext {
            exts.as_ptr()
        } else {
            std::ptr::null()
        },
    };
    let mut instance: VkInstance = std::ptr::null_mut();
    let r = vk_create_instance(&ici, std::ptr::null(), &mut instance);
    if r != VK_SUCCESS {
        return Err(format!("vkCreateInstance 失败: {r}"));
    }
    Ok((instance, validation, enable_debug_ext))
}

/// 枚举个物理设备,取首个(执行器首期单设备策略,与 vk.rs 全部入口同律)。
unsafe fn pick_physical_device(
    gipa: FnGetInstanceProcAddr,
    instance: VkInstance,
) -> Result<VkPhysicalDevice, String> {
    let vk_enum_pd: FnEnumeratePhysicalDevices =
        cast_fn(gipa(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
            .ok_or("缺 vkEnumeratePhysicalDevices")?;
    let mut count = 0u32;
    vk_enum_pd(instance, &mut count, std::ptr::null_mut());
    if count == 0 {
        return Err("无 Vulkan 物理设备".into());
    }
    let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
    vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
    Ok(pds[0])
}

/// 物理设备能力读取(extension 集 + features2 链 + properties blob;probe/execute 共用)。
unsafe fn read_physical_caps(
    gipa: FnGetInstanceProcAddr,
    instance: VkInstance,
    pd: VkPhysicalDevice,
) -> Result<DeviceCaps, String> {
    let vk_enum_ext: FnEnumerateDeviceExtensionProperties = cast_fn(gipa(
        instance,
        c"vkEnumerateDeviceExtensionProperties".as_ptr(),
    ))
    .ok_or("缺 vkEnumerateDeviceExtensionProperties")?;
    let vk_get_props: FnGetPhysicalDeviceProperties =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceProperties".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceProperties")?;
    let vk_get_features2: FnGetPhysicalDeviceFeatures2 =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceFeatures2".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceFeatures2(须 Vulkan 1.1)")?;

    // 扩展名集(NUL 结尾定长字符数组 → &str 前缀)。
    let mut ext_count = 0u32;
    vk_enum_ext(pd, std::ptr::null(), &mut ext_count, std::ptr::null_mut());
    let mut exts = vec![
        ExtensionProperties {
            extension_name: [0; 256],
            spec_version: 0,
        };
        ext_count as usize
    ];
    vk_enum_ext(pd, std::ptr::null(), &mut ext_count, exts.as_mut_ptr());
    let has_ext = |name: &CStr| -> bool {
        exts.iter().any(|e| {
            // SAFETY: 驱动写入的 extensionName 为 256 字节定长槽内 NUL 结尾 C 串。
            let bytes = unsafe { CStr::from_ptr(e.extension_name.as_ptr()) }.to_bytes();
            bytes == name.to_bytes()
        })
    };
    let sync2_ext = has_ext(c"VK_KHR_synchronization2");
    let int64_ext = has_ext(c"VK_KHR_shader_atomic_int64");
    let ray_query_ext = has_ext(c"VK_KHR_ray_query");
    let acceleration_structure_ext = has_ext(c"VK_KHR_acceleration_structure");
    let buffer_device_address_ext = has_ext(c"VK_KHR_buffer_device_address");
    let descriptor_indexing_ext = has_ext(c"VK_EXT_descriptor_indexing");
    let deferred_host_operations_ext = has_ext(c"VK_KHR_deferred_host_operations");
    let memory_budget_ext = has_ext(c"VK_EXT_memory_budget");
    let conservative_raster_ext = has_ext(c"VK_EXT_conservative_rasterization");

    // features2 链(sync2 + atomic int64 + W3 四节 feature;不存在扩展的节读回 0,
    // 无副作用)。deferred-host-operations 仅看扩展存在性,无 feature 结构。
    let mut descriptor_indexing_feat = PhysicalDeviceDescriptorIndexingFeatures {
        s_type: ST_PHYSICAL_DEVICE_DESCRIPTOR_INDEXING_FEATURES,
        p_next: std::ptr::null_mut(),
        bits: [0; 20],
    };
    let mut buffer_device_address_feat = PhysicalDeviceBufferDeviceAddressFeatures {
        s_type: ST_PHYSICAL_DEVICE_BUFFER_DEVICE_ADDRESS_FEATURES,
        p_next: (&mut descriptor_indexing_feat as *mut PhysicalDeviceDescriptorIndexingFeatures)
            .cast::<c_void>(),
        buffer_device_address: 0,
        buffer_device_address_capture_replay: 0,
        buffer_device_address_multi_device: 0,
    };
    let mut acceleration_structure_feat = PhysicalDeviceAccelerationStructureFeatures {
        s_type: ST_PHYSICAL_DEVICE_ACCELERATION_STRUCTURE_FEATURES_KHR,
        p_next: (&mut buffer_device_address_feat as *mut PhysicalDeviceBufferDeviceAddressFeatures)
            .cast::<c_void>(),
        acceleration_structure: 0,
        acceleration_structure_capture_replay: 0,
        acceleration_structure_indirect_build: 0,
        acceleration_structure_host_commands: 0,
        descriptor_binding_acceleration_structure_update_after_bind: 0,
    };
    let mut ray_query_feat = PhysicalDeviceRayQueryFeatures {
        s_type: ST_PHYSICAL_DEVICE_RAY_QUERY_FEATURES_KHR,
        p_next: (&mut acceleration_structure_feat
            as *mut PhysicalDeviceAccelerationStructureFeatures)
            .cast::<c_void>(),
        ray_query: 0,
    };
    let mut int64_feat = PhysicalDeviceShaderAtomicInt64Features {
        s_type: ST_PHYSICAL_DEVICE_SHADER_ATOMIC_INT64_FEATURES,
        p_next: (&mut ray_query_feat as *mut PhysicalDeviceRayQueryFeatures).cast::<c_void>(),
        shader_buffer_int64_atomics: 0,
        shader_shared_int64_atomics: 0,
    };
    let mut sync2_feat = PhysicalDeviceSynchronization2Features {
        s_type: ST_PHYSICAL_DEVICE_SYNCHRONIZATION_2_FEATURES,
        p_next: (&mut int64_feat as *mut PhysicalDeviceShaderAtomicInt64Features).cast::<c_void>(),
        synchronization2: 0,
    };
    let mut feat2 = PhysicalDeviceFeatures2 {
        s_type: ST_PHYSICAL_DEVICE_FEATURES_2,
        p_next: (&mut sync2_feat as *mut PhysicalDeviceSynchronization2Features).cast::<c_void>(),
        features: [0; 55],
    };
    vk_get_features2(pd, &mut feat2);

    // properties blob(plain vkGetPhysicalDeviceProperties,vk.rs bindless 先例)。
    // 字段偏移(SDK 1.3.296 vulkan_core.h 核对):apiVersion@0 / driverVersion@4 /
    // vendorID@8 / deviceID@12 / deviceType@16 / deviceName[256]@20 /
    // pipelineCacheUUID[16]@276 / limits@296(VkDeviceSize 成员致 align 8,自 292 补齐)/
    // limits.maxPushConstantsSize @296+32 = @328。
    let mut blob = PropertiesBlob { bytes: [0; 2048] };
    vk_get_props(pd, &mut blob);
    let api_version =
        u32::from_le_bytes([blob.bytes[0], blob.bytes[1], blob.bytes[2], blob.bytes[3]]);
    let name_bytes = &blob.bytes[20..276];
    let end = name_bytes.iter().position(|&b| b == 0).unwrap_or(256);
    let device_name = String::from_utf8_lossy(&name_bytes[..end]).into_owned();
    let max_push_constants_size = u32::from_le_bytes([
        blob.bytes[328],
        blob.bytes[329],
        blob.bytes[330],
        blob.bytes[331],
    ]);
    // VkPhysicalDeviceProperties.limits@296 + VkPhysicalDeviceLimits.timestampPeriod@424
    // (SDK 1.3.296 `vulkan_core.h` 逐字段 ABI 计算；布局锚单测固定总偏移 720)。
    let timestamp_period_ns = f32::from_le_bytes([
        blob.bytes[720],
        blob.bytes[721],
        blob.bytes[722],
        blob.bytes[723],
    ]);

    // 保守光栅属性链(G7.5b,RXS-0303 IR1):扩展在位时经 `vkGetPhysicalDeviceProperties2`
    // (Vulkan 1.1 core,与 features2 同门槛)链式读回全字段;扩展不在位 → `None`
    // (不发起链查询,零副作用)。
    let conservative_raster = if conservative_raster_ext {
        let vk_get_props2: FnGetPhysicalDeviceProperties2 =
            cast_fn(gipa(instance, c"vkGetPhysicalDeviceProperties2".as_ptr()))
                .ok_or("缺 vkGetPhysicalDeviceProperties2(须 Vulkan 1.1)")?;
        let mut cons_props = PhysicalDeviceConservativeRasterizationProperties {
            s_type: ST_PHYSICAL_DEVICE_CONSERVATIVE_RASTERIZATION_PROPERTIES_EXT,
            p_next: std::ptr::null_mut(),
            primitive_overestimation_size: 0.0,
            max_extra_primitive_overestimation_size: 0.0,
            extra_primitive_overestimation_size_granularity: 0.0,
            primitive_underestimation: 0,
            conservative_point_and_line_rasterization: 0,
            degenerate_triangles_rasterized: 0,
            degenerate_lines_rasterized: 0,
            fully_covered_fragment_shader_input_variable: 0,
            conservative_rasterization_post_depth_coverage: 0,
        };
        let mut props2 = PhysicalDeviceProperties2Chain {
            s_type: ST_PHYSICAL_DEVICE_PROPERTIES_2,
            p_next: (&mut cons_props as *mut PhysicalDeviceConservativeRasterizationProperties)
                .cast::<c_void>(),
            properties: PropertiesBlob { bytes: [0; 2048] },
        };
        vk_get_props2(pd, &mut props2);
        Some(ConservativeRasterProps {
            primitive_overestimation_size: cons_props.primitive_overestimation_size,
            max_extra_primitive_overestimation_size: cons_props
                .max_extra_primitive_overestimation_size,
            extra_primitive_overestimation_size_granularity: cons_props
                .extra_primitive_overestimation_size_granularity,
            degenerate_triangles_rasterized: cons_props.degenerate_triangles_rasterized != 0,
        })
    } else {
        None
    };

    Ok(DeviceCaps {
        device_name,
        synchronization2: sync2_ext && sync2_feat.synchronization2 != 0,
        shader_buffer_int64_atomics: int64_ext && int64_feat.shader_buffer_int64_atomics != 0,
        shader_int64: feat2.features[40] != 0,
        fragment_stores_and_atomics: feat2.features[26] != 0,
        ray_query: ray_query_ext && ray_query_feat.ray_query != 0,
        acceleration_structure: acceleration_structure_ext
            && acceleration_structure_feat.acceleration_structure != 0,
        buffer_device_address: (buffer_device_address_ext || api_version >= API_VERSION_1_2)
            && buffer_device_address_feat.buffer_device_address != 0,
        descriptor_indexing: descriptor_indexing_ext && descriptor_indexing_feat.bits[19] != 0,
        deferred_host_operations: deferred_host_operations_ext,
        memory_budget: memory_budget_ext,
        conservative_raster,
        timestamp_period_ns,
        max_push_constants_size,
    })
}

/// [`probe_device_caps`] 内部(instance 级探测,不建 device)。
unsafe fn probe_caps_inner(gipa: FnGetInstanceProcAddr) -> Result<DeviceCaps, String> {
    let (instance, _validation, _debug_utils) = create_instance(gipa, c"rurix-render-exec-probe")?;
    let vk_destroy_instance: FnDestroyInstance =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
    let out = (|| {
        let pd = pick_physical_device(gipa, instance)?;
        read_physical_caps(gipa, instance, pd)
    })();
    vk_destroy_instance(instance, std::ptr::null());
    out
}

/// device 级函数表(execute_frame_inner 单点装配;`dp!` 宏逐符号 null 校验)。
#[derive(Clone, Copy)]
struct Dev {
    get_device_queue: FnGetDeviceQueue,
    create_buffer: FnCreateBuffer,
    destroy_buffer: FnDestroyBuffer,
    buf_mem_req: FnGetBufferMemoryRequirements,
    alloc_mem: FnAllocateMemory,
    free_mem: FnFreeMemory,
    bind_buf: FnBindBufferMemory,
    map_mem: FnMapMemory,
    unmap_mem: FnUnmapMemory,
    create_image: FnCreateImage,
    destroy_image: FnDestroyImage,
    img_mem_req: FnGetImageMemoryRequirements,
    bind_img: FnBindImageMemory,
    create_image_view: FnCreateImageView,
    destroy_image_view: FnDestroyImageView,
    create_shader: FnCreateShaderModule,
    destroy_shader: FnDestroyShaderModule,
    create_dsl: FnCreateDescriptorSetLayout,
    destroy_dsl: FnDestroyDescriptorSetLayout,
    create_pl: FnCreatePipelineLayout,
    destroy_pl: FnDestroyPipelineLayout,
    create_cp: FnCreateComputePipelines,
    create_gp: FnCreateGraphicsPipelines,
    destroy_pipe: FnDestroyPipeline,
    create_rp: FnCreateRenderPass,
    destroy_rp: FnDestroyRenderPass,
    create_fb: FnCreateFramebuffer,
    destroy_fb: FnDestroyFramebuffer,
    create_dp: FnCreateDescriptorPool,
    destroy_dp: FnDestroyDescriptorPool,
    alloc_ds: FnAllocateDescriptorSets,
    update_ds: FnUpdateDescriptorSets,
    create_sampler: FnCreateSampler,
    destroy_sampler: FnDestroySampler,
    create_cmdpool: FnCreateCommandPool,
    destroy_cmdpool: FnDestroyCommandPool,
    alloc_cmd: FnAllocateCommandBuffers,
    begin_cmd: FnBeginCommandBuffer,
    end_cmd: FnEndCommandBuffer,
    reset_cmd: FnResetCommandBuffer,
    free_cmd: FnFreeCommandBuffers,
    cmd_bind_pipe: FnCmdBindPipeline,
    cmd_bind_ds: FnCmdBindDescriptorSets,
    cmd_push: FnCmdPushConstants,
    cmd_dispatch: FnCmdDispatch,
    cmd_dispatch_indirect: FnCmdDispatchIndirect,
    cmd_begin_rp: FnCmdBeginRenderPass,
    cmd_end_rp: FnCmdEndRenderPass,
    cmd_bind_vb: FnCmdBindVertexBuffers,
    cmd_draw: FnCmdDraw,
    cmd_draw_indirect: FnCmdDrawIndirect,
    cmd_barrier2: FnCmdPipelineBarrier2,
    cmd_copy_img2buf: FnCmdCopyImageToBuffer,
    cmd_copy_buf2img: FnCmdCopyBufferToImage,
    cmd_copy_buf: FnCmdCopyBuffer,
    queue_submit: FnQueueSubmit,
    queue_wait: FnQueueWaitIdle,
    create_fence: FnCreateFence,
    destroy_fence: FnDestroyFence,
    wait_fences: FnWaitForFences,
    reset_fences: FnResetFences,
    create_query_pool: FnCreateQueryPool,
    destroy_query_pool: FnDestroyQueryPool,
    cmd_reset_query_pool: FnCmdResetQueryPool,
    cmd_write_timestamp2: FnCmdWriteTimestamp2,
    get_query_pool_results: FnGetQueryPoolResults,
    /// G31+ 波 C Task C7:debug label 符号对(instance 级解析;VK_EXT_debug_utils
    /// 启用且双符号可解析才 Some,否则录制侧零开销跳过——fail-silent 不崩)。
    cmd_begin_label: Option<FnCmdBeginDebugUtilsLabelEXT>,
    cmd_end_label: Option<FnCmdEndDebugUtilsLabelEXT>,
}

impl Dev {
    /// device 级符号单点装配(逐符号 null 校验,缺失 → 确定性 Err)。
    ///
    /// # Safety
    /// `gdpa`/`device` 为有效 device 与 proc 查询函数;符号名 ⇔ 类型签名逐一对应。
    unsafe fn load(gdpa: FnGetDeviceProcAddr, device: VkDevice) -> Result<Dev, String> {
        macro_rules! dp {
            ($name:literal, $ty:ty) => {
                cast_fn::<$ty>(gdpa(device, $name.as_ptr()))
                    .ok_or_else(|| format!("缺 device 符号 {:?}", $name))?
            };
        }
        Ok(Dev {
            get_device_queue: dp!(c"vkGetDeviceQueue", FnGetDeviceQueue),
            create_buffer: dp!(c"vkCreateBuffer", FnCreateBuffer),
            destroy_buffer: dp!(c"vkDestroyBuffer", FnDestroyBuffer),
            buf_mem_req: dp!(
                c"vkGetBufferMemoryRequirements",
                FnGetBufferMemoryRequirements
            ),
            alloc_mem: dp!(c"vkAllocateMemory", FnAllocateMemory),
            free_mem: dp!(c"vkFreeMemory", FnFreeMemory),
            bind_buf: dp!(c"vkBindBufferMemory", FnBindBufferMemory),
            map_mem: dp!(c"vkMapMemory", FnMapMemory),
            unmap_mem: dp!(c"vkUnmapMemory", FnUnmapMemory),
            create_image: dp!(c"vkCreateImage", FnCreateImage),
            destroy_image: dp!(c"vkDestroyImage", FnDestroyImage),
            img_mem_req: dp!(
                c"vkGetImageMemoryRequirements",
                FnGetImageMemoryRequirements
            ),
            bind_img: dp!(c"vkBindImageMemory", FnBindImageMemory),
            create_image_view: dp!(c"vkCreateImageView", FnCreateImageView),
            destroy_image_view: dp!(c"vkDestroyImageView", FnDestroyImageView),
            create_shader: dp!(c"vkCreateShaderModule", FnCreateShaderModule),
            destroy_shader: dp!(c"vkDestroyShaderModule", FnDestroyShaderModule),
            create_dsl: dp!(c"vkCreateDescriptorSetLayout", FnCreateDescriptorSetLayout),
            destroy_dsl: dp!(
                c"vkDestroyDescriptorSetLayout",
                FnDestroyDescriptorSetLayout
            ),
            create_pl: dp!(c"vkCreatePipelineLayout", FnCreatePipelineLayout),
            destroy_pl: dp!(c"vkDestroyPipelineLayout", FnDestroyPipelineLayout),
            create_cp: dp!(c"vkCreateComputePipelines", FnCreateComputePipelines),
            create_gp: dp!(c"vkCreateGraphicsPipelines", FnCreateGraphicsPipelines),
            destroy_pipe: dp!(c"vkDestroyPipeline", FnDestroyPipeline),
            create_rp: dp!(c"vkCreateRenderPass", FnCreateRenderPass),
            destroy_rp: dp!(c"vkDestroyRenderPass", FnDestroyRenderPass),
            create_fb: dp!(c"vkCreateFramebuffer", FnCreateFramebuffer),
            destroy_fb: dp!(c"vkDestroyFramebuffer", FnDestroyFramebuffer),
            create_dp: dp!(c"vkCreateDescriptorPool", FnCreateDescriptorPool),
            destroy_dp: dp!(c"vkDestroyDescriptorPool", FnDestroyDescriptorPool),
            alloc_ds: dp!(c"vkAllocateDescriptorSets", FnAllocateDescriptorSets),
            update_ds: dp!(c"vkUpdateDescriptorSets", FnUpdateDescriptorSets),
            create_sampler: dp!(c"vkCreateSampler", FnCreateSampler),
            destroy_sampler: dp!(c"vkDestroySampler", FnDestroySampler),
            create_cmdpool: dp!(c"vkCreateCommandPool", FnCreateCommandPool),
            destroy_cmdpool: dp!(c"vkDestroyCommandPool", FnDestroyCommandPool),
            alloc_cmd: dp!(c"vkAllocateCommandBuffers", FnAllocateCommandBuffers),
            begin_cmd: dp!(c"vkBeginCommandBuffer", FnBeginCommandBuffer),
            end_cmd: dp!(c"vkEndCommandBuffer", FnEndCommandBuffer),
            reset_cmd: dp!(c"vkResetCommandBuffer", FnResetCommandBuffer),
            free_cmd: dp!(c"vkFreeCommandBuffers", FnFreeCommandBuffers),
            cmd_bind_pipe: dp!(c"vkCmdBindPipeline", FnCmdBindPipeline),
            cmd_bind_ds: dp!(c"vkCmdBindDescriptorSets", FnCmdBindDescriptorSets),
            cmd_push: dp!(c"vkCmdPushConstants", FnCmdPushConstants),
            cmd_dispatch: dp!(c"vkCmdDispatch", FnCmdDispatch),
            cmd_dispatch_indirect: dp!(c"vkCmdDispatchIndirect", FnCmdDispatchIndirect),
            cmd_begin_rp: dp!(c"vkCmdBeginRenderPass", FnCmdBeginRenderPass),
            cmd_end_rp: dp!(c"vkCmdEndRenderPass", FnCmdEndRenderPass),
            cmd_bind_vb: dp!(c"vkCmdBindVertexBuffers", FnCmdBindVertexBuffers),
            cmd_draw: dp!(c"vkCmdDraw", FnCmdDraw),
            cmd_draw_indirect: dp!(c"vkCmdDrawIndirect", FnCmdDrawIndirect),
            cmd_barrier2: dp!(c"vkCmdPipelineBarrier2KHR", FnCmdPipelineBarrier2),
            cmd_copy_img2buf: dp!(c"vkCmdCopyImageToBuffer", FnCmdCopyImageToBuffer),
            cmd_copy_buf2img: dp!(c"vkCmdCopyBufferToImage", FnCmdCopyBufferToImage),
            cmd_copy_buf: dp!(c"vkCmdCopyBuffer", FnCmdCopyBuffer),
            queue_submit: dp!(c"vkQueueSubmit", FnQueueSubmit),
            queue_wait: dp!(c"vkQueueWaitIdle", FnQueueWaitIdle),
            create_fence: dp!(c"vkCreateFence", FnCreateFence),
            destroy_fence: dp!(c"vkDestroyFence", FnDestroyFence),
            wait_fences: dp!(c"vkWaitForFences", FnWaitForFences),
            reset_fences: dp!(c"vkResetFences", FnResetFences),
            create_query_pool: dp!(c"vkCreateQueryPool", FnCreateQueryPool),
            destroy_query_pool: dp!(c"vkDestroyQueryPool", FnDestroyQueryPool),
            cmd_reset_query_pool: dp!(c"vkCmdResetQueryPool", FnCmdResetQueryPool),
            cmd_write_timestamp2: dp!(c"vkCmdWriteTimestamp2", FnCmdWriteTimestamp2),
            get_query_pool_results: dp!(c"vkGetQueryPoolResults", FnGetQueryPoolResults),
            cmd_begin_label: None,
            cmd_end_label: None,
        })
    }

    /// G31+ 波 C Task C7:debug label 符号对装载(instance 级 `vkGetInstanceProcAddr`
    /// 解析——debug utils 命令为 instance 级,device proc 查询不保证可解析)。扩展未
    /// 启用/任一符号缺失 → 双 None 不报错（fail-silent;录制侧 `None` 零开销跳过）。
    ///
    /// # Safety
    /// `gipa`/`instance` 为有效 instance 与 proc 查询函数;符号名 ⇔ 类型签名逐一对应。
    unsafe fn load_debug_labels(
        &mut self,
        gipa: FnGetInstanceProcAddr,
        instance: VkInstance,
        enabled: bool,
    ) {
        if !enabled {
            return;
        }
        let beg = cast_fn::<FnCmdBeginDebugUtilsLabelEXT>(gipa(
            instance,
            c"vkCmdBeginDebugUtilsLabelEXT".as_ptr(),
        ));
        let end = cast_fn::<FnCmdEndDebugUtilsLabelEXT>(gipa(
            instance,
            c"vkCmdEndDebugUtilsLabelEXT".as_ptr(),
        ));
        if let (Some(b), Some(e)) = (beg, end) {
            self.cmd_begin_label = Some(b);
            self.cmd_end_label = Some(e);
        }
    }

    /// G31+ 波 C Task C7:label 录制面是否活跃（双符号均在位）。
    fn labels_active(&self) -> bool {
        self.cmd_begin_label.is_some() && self.cmd_end_label.is_some()
    }
}

struct TrackedAllocation {
    memory: VkDeviceMemory,
    entry: AllocationLedgerEntry,
}

/// 句柄销毁登记表(单点逆序销毁;早退路径同走——U32 泄漏/双释放纪律)。
#[derive(Default)]
struct Cleanup {
    cmdpool: VkCommandPool,
    query_pool: VkQueryPool,
    pool: VkDescriptorPool,
    sampler: VkSampler,
    views: Vec<VkImageView>,
    images: Vec<(VkImage, VkDeviceMemory)>,
    buffers: Vec<(VkBuffer, VkDeviceMemory)>,
    framebuffers: Vec<VkFramebuffer>,
    render_passes: Vec<VkRenderPass>,
    pipelines: Vec<VkPipeline>,
    pipe_layouts: Vec<VkPipelineLayout>,
    dsls: Vec<VkDescriptorSetLayout>,
    shader_modules: Vec<VkShaderModule>,
    allocations: Vec<TrackedAllocation>,
    next_allocation_id: u64,
}

impl Cleanup {
    /// 固定逆序销毁(cmdpool→pool→sampler→view→image→buffer→fb→rp→pipe→layout→dsl→
    /// shader);device 存活期调用,调用前已 `vkQueueWaitIdle`(无在途使用)。
    ///
    /// # Safety
    /// 全部句柄由本帧 device 创建且仅登记一次;登记表不跨帧复用。
    unsafe fn destroy_all(&self, dev: &Dev, device: VkDevice) {
        if self.cmdpool != VK_NULL_HANDLE {
            (dev.destroy_cmdpool)(device, self.cmdpool, std::ptr::null());
        }
        if self.query_pool != VK_NULL_HANDLE {
            (dev.destroy_query_pool)(device, self.query_pool, std::ptr::null());
        }
        if self.pool != VK_NULL_HANDLE {
            (dev.destroy_dp)(device, self.pool, std::ptr::null());
        }
        if self.sampler != VK_NULL_HANDLE {
            (dev.destroy_sampler)(device, self.sampler, std::ptr::null());
        }
        for &v in &self.views {
            (dev.destroy_image_view)(device, v, std::ptr::null());
        }
        for &(img, mem) in &self.images {
            (dev.destroy_image)(device, img, std::ptr::null());
            (dev.free_mem)(device, mem, std::ptr::null());
        }
        for &(buf, mem) in &self.buffers {
            (dev.destroy_buffer)(device, buf, std::ptr::null());
            (dev.free_mem)(device, mem, std::ptr::null());
        }
        for &fb in &self.framebuffers {
            (dev.destroy_fb)(device, fb, std::ptr::null());
        }
        for &rp in &self.render_passes {
            (dev.destroy_rp)(device, rp, std::ptr::null());
        }
        for &p in &self.pipelines {
            (dev.destroy_pipe)(device, p, std::ptr::null());
        }
        for &pl in &self.pipe_layouts {
            (dev.destroy_pl)(device, pl, std::ptr::null());
        }
        for &d in &self.dsls {
            (dev.destroy_dsl)(device, d, std::ptr::null());
        }
        for &m in &self.shader_modules {
            (dev.destroy_shader)(device, m, std::ptr::null());
        }
    }

    fn register_allocation(
        &mut self,
        memory: VkDeviceMemory,
        bytes: u64,
        heap_index: u32,
        resource_id: Option<StableResourceId>,
    ) -> StableAllocationId {
        self.next_allocation_id = self.next_allocation_id.saturating_add(1);
        let allocation_id = StableAllocationId(self.next_allocation_id);
        self.allocations.push(TrackedAllocation {
            memory,
            entry: AllocationLedgerEntry {
                allocation_id,
                resource_id,
                bytes,
                heap_index,
            },
        });
        allocation_id
    }

    fn object_count(&self) -> u64 {
        u64::from(self.cmdpool != VK_NULL_HANDLE)
            + u64::from(self.query_pool != VK_NULL_HANDLE)
            + u64::from(self.pool != VK_NULL_HANDLE)
            + u64::from(self.sampler != VK_NULL_HANDLE)
            + self.views.len() as u64
            + self.images.len() as u64
            + self.buffers.len() as u64
            + self.framebuffers.len() as u64
            + self.render_passes.len() as u64
            + self.pipelines.len() as u64
            + self.pipe_layouts.len() as u64
            + self.dsls.len() as u64
            + self.shader_modules.len() as u64
    }
}

/// 选内存类型(type_bits 允许集合内首个含全部 required 标志者;vk.rs 同名先例同律)。
fn pick_mem_type(
    memprops: &PhysicalDeviceMemoryProperties,
    type_bits: u32,
    required: VkFlags,
) -> Option<u32> {
    (0..memprops.memory_type_count).find(|&i| {
        let mt = memprops.memory_types[i as usize];
        type_bits & (1 << i) != 0 && mt.property_flags & required == required
    })
}

/// 本模块 `PhysicalDeviceMemoryProperties` → vk.rs 同名类型(两侧均 `#[repr(C)]`
/// 且字段逐一对应,布局各经本侧 `ffi_layout_anchors` 520 字节锚定;**逐字段显式
/// 拷贝**——不 transmute,供 vk.rs `VkAsManager` 建面消费其本侧类型)。
fn memprops_to_vk(
    src: &PhysicalDeviceMemoryProperties,
) -> crate::vk::PhysicalDeviceMemoryProperties {
    crate::vk::PhysicalDeviceMemoryProperties {
        memory_type_count: src.memory_type_count,
        memory_types: src.memory_types.map(|mt| crate::vk::MemoryType {
            property_flags: mt.property_flags,
            heap_index: mt.heap_index,
        }),
        memory_heap_count: src.memory_heap_count,
        memory_heaps: src.memory_heaps.map(|mh| crate::vk::MemoryHeap {
            size: mh.size,
            flags: mh.flags,
        }),
    }
}

/// 运行期资源(buffer/image 句柄对;image 挂 view 与可选 staging)。
struct RtBuffer {
    buffer: VkBuffer,
    mem: VkDeviceMemory,
}

struct RtImage {
    image: VkImage,
    mem: VkDeviceMemory,
    view: VkImageView,
    width: u32,
    height: u32,
    format: TexFormat,
    /// 建面最终 `VkImageUsageFlags`(`texture_usage_flags` 产;G14.10b 导出簿记
    /// 消费——导入侧 image 须同 usage)。
    usage_flags: u32,
    /// 初始数据 staging(host-visible;命令流首段 copy 后于帧末销毁)。
    staging: Option<VkBuffer>,
}

enum RtRes {
    Buf(RtBuffer),
    Img(RtImage),
}

impl RtRes {
    fn image(&self) -> Option<&RtImage> {
        match self {
            RtRes::Img(i) => Some(i),
            RtRes::Buf(_) => None,
        }
    }
}

/// buffer 内存分路(G14.10d 三路判定,RFC-0030 §4.3;内存型只改驻留位置不改
/// 数据内容——位级零漂移机核)。
#[derive(Clone, Copy, PartialEq, Eq)]
enum BufferMemClass {
    /// DEVICE_LOCAL 独占 VRAM(GPU 全速 ~500GB/s;不可 map——初始数据/回读均经
    /// staging copy)。session SSBO 的默认新态:TSR 访存归因 = 恒 HOST_VISIBLE
    /// 把 SSBO 放 PCIe 侧 ~25GB/s,bistro t50 每帧跨 PCIe 净流量 135MB → 物理
    /// 下限 7-8ms;DEVICE_LOCAL 同流量 ~0.3ms。
    DeviceLocal,
    /// host-visible+coherent 首匹配(NVIDIA 上 = uncached/WC;host 写向最优)。
    /// 既有态保留:被 `FrameUpdate::buffer_uploads` 逐帧覆盖写的小参数 buffer
    /// 与上传 staging 用之。
    HostWc,
    /// `HOST_VISIBLE|HOST_COHERENT|HOST_CACHED` 优选,缺型回退 HostWc(G14plus
    /// §4.3 L1,vendor_upscale.rs DLSS readback 同型先例):readback staging
    /// (GPU copy 写、CPU map 读)专用——WC 逐元素 host 读 = PCIe 往返延迟
    /// (bistro t100 回读损失 ~71ms),cached 块读/逐元素读均缓存命中口径。
    /// 两条路径均含 HOST_COHERENT → map 后免 vkInvalidateMappedMemoryRanges
    /// 的既有纪律不变(防御性注释)。
    HostCachedPreferred,
}

/// 建 buffer + 按 `mem_class` 选型内存 + 绑定 + 可选初始数据上传(host 路 map
/// 写;DEVICE_LOCAL 路不可 map,`data` 须 `None`——初始数据由调用方 staging copy
/// 承载,传入即内部纪律 Err)。登记 cleanup,返回 (buffer, mem) 句柄对。
///
/// # Safety
/// dev/device 有效;desc 已经 validate;memprops 为本物理设备内存属性。
#[allow(clippy::too_many_arguments)]
unsafe fn create_device_buffer(
    dev: &Dev,
    device: VkDevice,
    memprops: &PhysicalDeviceMemoryProperties,
    size: u64,
    usage: VkFlags,
    data: Option<&[u8]>,
    resource_id: Option<StableResourceId>,
    cleanup: &mut Cleanup,
    mem_class: BufferMemClass,
) -> Result<(VkBuffer, VkDeviceMemory), String> {
    if mem_class == BufferMemClass::DeviceLocal && data.is_some() {
        return Err("内部纪律:DEVICE_LOCAL buffer 初始数据须走 staging copy(不可 map)".into());
    }
    let bci = BufferCreateInfo {
        s_type: ST_BUFFER_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        size,
        usage,
        sharing_mode: SHARING_MODE_EXCLUSIVE,
        queue_family_index_count: 0,
        p_queue_family_indices: std::ptr::null(),
    };
    let mut buffer: VkBuffer = VK_NULL_HANDLE;
    if (dev.create_buffer)(device, &bci, std::ptr::null(), &mut buffer) != VK_SUCCESS {
        return Err(format!("vkCreateBuffer 失败(size={size})"));
    }
    let mut req = std::mem::zeroed::<MemoryRequirements>();
    (dev.buf_mem_req)(device, buffer, &mut req);
    // 三路选型(同一 pick_mem_type 事实源):DeviceLocal 首匹配 DEVICE_LOCAL
    // (image 建面同律,缺型 fail-closed);HostCachedPreferred 优选 cached、
    // 缺型回退 HV+HC;HostWc 首匹配 HV+HC(既有行为 0-byte)。
    let mt = match mem_class {
        BufferMemClass::DeviceLocal => {
            let Some(mt) = pick_mem_type(memprops, req.memory_type_bits, MEM_DEVICE_LOCAL) else {
                (dev.destroy_buffer)(device, buffer, std::ptr::null());
                return Err("无 device-local 内存类型(buffer)".into());
            };
            mt
        }
        BufferMemClass::HostWc | BufferMemClass::HostCachedPreferred => {
            let cached_mt = if mem_class == BufferMemClass::HostCachedPreferred {
                pick_mem_type(
                    memprops,
                    req.memory_type_bits,
                    MEM_HOST_VISIBLE | MEM_HOST_COHERENT | MEM_HOST_CACHED,
                )
            } else {
                None
            };
            let Some(mt) = cached_mt.or_else(|| {
                pick_mem_type(
                    memprops,
                    req.memory_type_bits,
                    MEM_HOST_VISIBLE | MEM_HOST_COHERENT,
                )
            }) else {
                (dev.destroy_buffer)(device, buffer, std::ptr::null());
                return Err("无 host-visible+coherent 内存类型".into());
            };
            mt
        }
    };
    let mai = MemoryAllocateInfo {
        s_type: ST_MEMORY_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        allocation_size: req.size,
        memory_type_index: mt,
    };
    let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
    if (dev.alloc_mem)(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
        (dev.destroy_buffer)(device, buffer, std::ptr::null());
        return Err("vkAllocateMemory 失败(buffer)".into());
    }
    (dev.bind_buf)(device, buffer, mem, 0);
    if let Some(d) = data
        && !d.is_empty()
    {
        let mut ptr: *mut c_void = std::ptr::null_mut();
        if (dev.map_mem)(device, mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
            (dev.destroy_buffer)(device, buffer, std::ptr::null());
            (dev.free_mem)(device, mem, std::ptr::null());
            return Err("vkMapMemory 失败(buffer 上传)".into());
        }
        std::ptr::copy_nonoverlapping(d.as_ptr(), ptr.cast::<u8>(), d.len());
        (dev.unmap_mem)(device, mem);
    }
    cleanup.buffers.push((buffer, mem));
    cleanup.register_allocation(
        mem,
        req.size,
        memprops.memory_types[mt as usize].heap_index,
        resource_id,
    );
    Ok((buffer, mem))
}

/// per-pass 装配结果(创建期一次性;persistent session 持有供每帧重录复用,
/// pipeline/descriptor set/render pass/framebuffer 跨帧不变)。
struct PassSetup {
    set: Option<VkDescriptorSet>,
    /// set0 layout(G31 FIF 流水 per-slot override set 分配的 layout 事实源;
    /// override 布局键同构校验 ⇒ 与 set 同 layout)。
    dsl: VkDescriptorSetLayout,
    pl: VkPipelineLayout,
    pc_size: u32,
    pipe: VkPipeline,
    // raster 专属。
    rp: VkRenderPass,
    fb: VkFramebuffer,
    extent: (u32, u32),
    clears: Vec<ClearValue>,
}

/// pass descriptor set 写入(创建装配与 FrameUpdate binding override 重写共用**同一
/// 事实源**,禁第二份写路径)。binding 序 = accel struct `[0..A)` → storage buffer
/// `[A..A+N)` → sampled image → storage image → uniform(模块头固定约定);AS 写经
/// pNext `WriteDescriptorSetAccelStructure`(vk.rs pub(crate) 复用,禁第二份定义)。
/// 指针回填模式:info/pNext 向量先收集、定长后统一取址(地址稳定)。
///
/// # Safety
/// dev/device 有效;set 已由调用方分配且不在途(创建期新分配 / 重写前帧 fence 已等待);
/// `b` 已经 validate(下标/用途合法),`as_handles` 与 session AS 表一一对应。
#[allow(clippy::too_many_arguments)]
unsafe fn write_pass_descriptor_set(
    dev: &Dev,
    device: VkDevice,
    set: VkDescriptorSet,
    b: &Bindings,
    rt: &[RtRes],
    sampler: VkSampler,
    as_handles: &[u64],
    pass_diag: usize,
) -> Result<(), String> {
    let n_as = b.accel_structs.len() as u32;
    let n_sb = b.storage_buffers.len() as u32;
    let n_si = b.sampled_images.len() as u32;
    let n_simg = b.storage_images.len() as u32;
    let mut writes: Vec<WriteDescriptorSet> = Vec::new();
    let mut buf_infos: Vec<DescriptorBufferInfo> = Vec::new();
    let mut img_infos: Vec<DescriptorImageInfo> = Vec::new();
    let mut as_writes: Vec<WriteDescriptorSetAccelStructure> = Vec::new();
    for (i, &as_index) in b.accel_structs.iter().enumerate() {
        let Some(&tlas) = as_handles.get(as_index as usize) else {
            return Err(format!(
                "pass {pass_diag}: accel struct 下标 {as_index} 越界(session AS 表 {} 项)",
                as_handles.len()
            ));
        };
        if tlas == VK_NULL_HANDLE {
            return Err(format!(
                "pass {pass_diag}: accel struct {as_index} TLAS 句柄已销毁/过期\
                 (fail-closed,禁消费悬垂加速结构)"
            ));
        }
        as_writes.push(WriteDescriptorSetAccelStructure {
            s_type: ST_WRITE_DESCRIPTOR_SET_ACCELERATION_STRUCTURE_KHR,
            p_next: std::ptr::null(),
            acceleration_structure_count: 1,
            p_acceleration_structures: &as_handles[as_index as usize],
        });
        writes.push(WriteDescriptorSet {
            s_type: ST_WRITE_DESCRIPTOR_SET,
            p_next: std::ptr::null(), // 下文回填(as_writes 定长后取址)
            dst_set: set,
            dst_binding: i as u32,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR,
            p_image_info: std::ptr::null(),
            p_buffer_info: std::ptr::null(),
            p_texel_buffer_view: std::ptr::null(),
        });
    }
    for (i, &res) in b.storage_buffers.iter().enumerate() {
        let RtRes::Buf(rb) = &rt[res as usize] else {
            return Err(format!("pass {pass_diag}: storage buffer {res} 非 buffer"));
        };
        buf_infos.push(DescriptorBufferInfo {
            buffer: rb.buffer,
            offset: 0,
            range: WHOLE_SIZE,
        });
        writes.push(WriteDescriptorSet {
            s_type: ST_WRITE_DESCRIPTOR_SET,
            p_next: std::ptr::null(),
            dst_set: set,
            dst_binding: n_as + i as u32,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: DESCRIPTOR_TYPE_STORAGE_BUFFER,
            p_image_info: std::ptr::null(),
            p_buffer_info: std::ptr::null(), // 下文回填(buf_infos 定长后取址)
            p_texel_buffer_view: std::ptr::null(),
        });
    }
    for (j, &res) in b.sampled_images.iter().enumerate() {
        let Some(ri) = rt[res as usize].image() else {
            return Err(format!("pass {pass_diag}: sampled image {res} 非 texture"));
        };
        img_infos.push(DescriptorImageInfo {
            sampler,
            image_view: ri.view,
            image_layout: LAYOUT_SHADER_READ_ONLY_OPTIMAL,
        });
        writes.push(WriteDescriptorSet {
            s_type: ST_WRITE_DESCRIPTOR_SET,
            p_next: std::ptr::null(),
            dst_set: set,
            dst_binding: n_as + n_sb + j as u32,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
            p_image_info: std::ptr::null(), // 下文回填
            p_buffer_info: std::ptr::null(),
            p_texel_buffer_view: std::ptr::null(),
        });
    }
    for (k, &res) in b.storage_images.iter().enumerate() {
        let Some(ri) = rt[res as usize].image() else {
            return Err(format!("pass {pass_diag}: storage image {res} 非 texture"));
        };
        img_infos.push(DescriptorImageInfo {
            sampler: VK_NULL_HANDLE,
            image_view: ri.view,
            image_layout: LAYOUT_GENERAL,
        });
        writes.push(WriteDescriptorSet {
            s_type: ST_WRITE_DESCRIPTOR_SET,
            p_next: std::ptr::null(),
            dst_set: set,
            dst_binding: n_as + n_sb + n_si + k as u32,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: DESCRIPTOR_TYPE_STORAGE_IMAGE,
            p_image_info: std::ptr::null(), // 下文回填
            p_buffer_info: std::ptr::null(),
            p_texel_buffer_view: std::ptr::null(),
        });
    }
    if let Some(u) = b.uniform {
        let RtRes::Buf(rb) = &rt[u.res as usize] else {
            return Err(format!("pass {pass_diag}: uniform {} 非 buffer", u.res));
        };
        buf_infos.push(DescriptorBufferInfo {
            buffer: rb.buffer,
            offset: u.offset,
            range: u.size,
        });
        writes.push(WriteDescriptorSet {
            s_type: ST_WRITE_DESCRIPTOR_SET,
            p_next: std::ptr::null(),
            dst_set: set,
            dst_binding: n_as + n_sb + n_si + n_simg,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: DESCRIPTOR_TYPE_UNIFORM_BUFFER,
            p_image_info: std::ptr::null(),
            p_buffer_info: std::ptr::null(), // 下文回填
            p_texel_buffer_view: std::ptr::null(),
        });
    }
    // 回填指针(info/pNext 向量定长,地址稳定;写入序 = AS 列 → storage 列 → uniform)。
    let mut ai = 0usize;
    let mut bi = 0usize;
    let mut ii = 0usize;
    for w in &mut writes {
        match w.descriptor_type {
            DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR => {
                w.p_next =
                    (&as_writes[ai] as *const WriteDescriptorSetAccelStructure).cast::<c_void>();
                ai += 1;
            }
            DESCRIPTOR_TYPE_STORAGE_BUFFER | DESCRIPTOR_TYPE_UNIFORM_BUFFER => {
                w.p_buffer_info = &buf_infos[bi];
                bi += 1;
            }
            _ => {
                w.p_image_info = &img_infos[ii];
                ii += 1;
            }
        }
    }
    (dev.update_ds)(
        device,
        writes.len() as u32,
        writes.as_ptr(),
        0,
        std::ptr::null(),
    );
    Ok(())
}

/// 帧命令体录制入参(创建录制与 FrameUpdate 重录共用同一录制事实源;禁第二套
/// 屏障/录制逻辑)。`effective_bindings` = 各 pass 本帧实际绑定(创建期 = 声明;
/// 重录期 = override 后);`record_upload_segment` 恒 true 于创建期(初始数据上传),
/// 重录期恒 false(初始数据已在 image/buffer 内,不重复回灌)。
struct FrameBodyParams<'a> {
    dev: &'a Dev,
    device: VkDevice,
    memprops: &'a PhysicalDeviceMemoryProperties,
    cmd: VkCommandBuffer,
    resources: &'a [ResourceDesc<'a>],
    rt: &'a [RtRes],
    passes: &'a [Pass<'a>],
    barriers: &'a [&'a [(u32, TargetState)]],
    effective_bindings: &'a [Bindings],
    setups: &'a [PassSetup],
    query_pool: VkQueryPool,
    /// timestamp query 首下标(G14plus FIF:slot k 用区间
    /// `[k*passes*2, (k+1)*passes*2)`;顺序路恒 0——既有行为 0-byte)。
    query_base: u32,
    inline_vbs: &'a [Option<VkBuffer>],
    readbacks: &'a [Readback],
    record_upload_segment: bool,
    /// G14.10b exportable 纹理下标集(帧末 layout→GENERAL 收敛 + EXTERNAL
    /// release barrier;空 = 无导出面,命令流 0-byte)。
    exportable: &'a [u32],
    /// release barrier 的 src queue family(session 单 graphics queue)。
    queue_family_index: u32,
    /// G31 FIF 流水:逐 pass descriptor 绑定覆盖(`Some` 且 `[pi]` 非空 → 绑
    /// 本 slot override set 而非 session 共享 set;顺序路恒 `None`——既有
    /// 命令流 0-byte)。
    slot_set_overrides: Option<&'a [Option<VkDescriptorSet>]>,
}

/// 帧命令体的 AS 操作包(G31+ 波 A Task A4 TLAS 实例更新 + G31+ 波 B Task B5
/// BLAS 顶点 refit;单一 `&mut` manager 承载同帧全部 AS 操作——两操作同帧
/// 同 manager 合法,免双借冲突)。
struct AsFrameOps<'a> {
    /// 目标 AS manager(session AS 表项;与 cmd 同属一个 device)。
    mgr: &'a mut VkAsManager,
    /// AS device 符号表。
    fns: &'a VkAsFns,
    /// TLAS 实例变换更新动作(帧首录;`None` = 无)。
    tlas_action: Option<TlasBuildAction>,
    /// BLAS 顶点 refit(在 pass `after_pass` 录完后插入;`None` = 无)。
    blas_refit: Option<BlasRefitRecord>,
    /// G34-2 加性:第二 TLAS 更新的 manager + 动作(双 TLAS 车道表 1;校验面
    /// 已保证与 `mgr` 异槽——`split_at_mut` 双借承载;`None` = 无,既有面
    /// 0-byte)。录制序 = `tlas_action` → 本件 → 单条 consume barrier(双
    /// update 同域一次序化,与单 update 面命令流逐字节同前缀)。
    tlas_b: Option<(&'a mut VkAsManager, TlasBuildAction)>,
    /// G37 W3 hzb_skin 加性:第二 BLAS refit(表 1 manager——`tlas_b` 的
    /// manager 承载;校验面已保证 `tlas_b` 同现同槽;`None` = 无,既有面
    /// 0-byte)。录制面 = `blas_refit` 同律桥接(pass `after_pass` 后)。
    blas_refit_b: Option<BlasRefitRecord>,
}

/// BLAS refit 录制载荷(公网 [`BlasRefitUpdate`] 的 native 解析形;
/// `src_res` 为 session 资源下标,copy 源 buffer 在录制点经 rt 表解析)。
#[derive(Debug, Clone, Copy)]
struct BlasRefitRecord {
    /// 目标 BLAS 下标。
    blas_index: u32,
    /// 蒙皮后顶点源(session 资源下标;buffer)。
    src_res: u32,
    /// 源字节偏移。
    src_offset: u64,
    /// 拷贝字节数(= BLAS 顶点缓冲字节数)。
    byte_len: u64,
    /// 在该 pass 录完后插入桥接 + UPDATE build。
    after_pass: u32,
}

/// G38 T3 加性:BLAS refit 桥录制扩展(公网 [`BlasRefitBridgeExt`] 的 native
/// 解析形;只作用于**主** refit 臂——`blas_refit_b`〔hzb_skin 表 1〕不开放,
/// 既有命令流 0-byte)。独立小件而非 [`BlasRefitRecord`]/[`AsFrameOps`] 加
/// 字段:两结构被 render_exec_g37_fif_dyn.rs 以字面量构造(T2 冻结面),
/// 加字段即打崩其编译——加性纪律以新类型承载。
#[derive(Debug, Clone, Copy)]
struct BridgeRecordExt<'a> {
    /// 脏区段(off,len)列表(相对 refit 窗,已过 `validate_bridge_ext`;
    /// `None` = 既有单 region 全量;`Some(&[])` = 跳过桥接 copy 三步)。
    regions: Option<&'a [(u64, u64)]>,
    /// 桥接时戳 query 追加区首下标(`Some` = 录 3 点:桥首/copy 后/build 后)。
    query_base: Option<u32>,
}

/// 帧命令体录制:[可选上传段] → [可选 TLAS update + consume barrier] → 逐 pass
/// (plan 逐字回放 → 隐式补全 → pass 本体,timestamp 包裹;pass 后可插 G31+
/// 波 B BLAS refit 桥) → readback 段。
///
/// `cleanup` = `Some` 时为创建录制(按需建持久 readback buffer 并登记 ledger);
/// `None` 时 `rb_buffers` 须已与 `readbacks` 逐位对齐(重录不新建分配)。
/// `as_ops` = `Some` 时:含 TLAS 动作则于 pass 链前录 `record_tlas_update` +
/// `record_consume_barrier(COMPUTE_SHADER)`;含 BLAS refit 则于 pass
/// `after_pass` 录完后插入桥接 copy + UPDATE build + consume barrier
/// (G31+ 波 B Task B5;两操作可同帧同 manager)。
///
/// # Safety
/// `p.cmd` 处于录制态(begin 后);`p.rt`/`p.setups`/`p.inline_vbs` 与资源/pass 表
/// 一一对应且存活;`as_ops` 的 manager 与 cmd 同属一个 device,TLAS 已建且
/// descriptor 在 submit 前不重写;状态跟踪按创建期同一初值规则确定性重建。
unsafe fn record_frame_body(
    p: &FrameBodyParams<'_>,
    rb_buffers: &mut Vec<Option<(VkBuffer, VkDeviceMemory)>>,
    cleanup: Option<&mut Cleanup>,
    as_ops: Option<AsFrameOps<'_>>,
) -> Result<(), String> {
    // G38 T3:既有签名恒转发 _ex(bridge = None 命令流逐字节不变;fif_dyn/
    // 创建期/FIF 流水调用点零改写)。
    // SAFETY: 契约与本函数逐字同(见上方 Safety 注)。
    unsafe { record_frame_body_ex(p, rb_buffers, cleanup, as_ops, None) }
}

/// G38 T3 加性:[`record_frame_body`] 的桥扩展体(`bridge` = BLAS refit 桥的
/// 多 region 脏区段 + 桥接时戳 query 追加区;`None` = 与既有主体逐字等价)。
///
/// # Safety
/// 同 [`record_frame_body`];`bridge.query_base` 为 `Some` 时 query pool 须含
/// 追加区 `[base, base+BRIDGE_QUERY_COUNT)`(创建期恒分配)。
unsafe fn record_frame_body_ex(
    p: &FrameBodyParams<'_>,
    rb_buffers: &mut Vec<Option<(VkBuffer, VkDeviceMemory)>>,
    mut cleanup: Option<&mut Cleanup>,
    mut as_ops: Option<AsFrameOps<'_>>,
    bridge: Option<BridgeRecordExt<'_>>,
) -> Result<(), String> {
    let dev = p.dev;
    let cmd = p.cmd;
    let rt = p.rt;
    let query_pool = p.query_pool;
    // 状态跟踪初值(host 路 buffer 数据→HOST_WRITE;G14.10d DEVICE_LOCAL buffer
    // 数据经创建期 one-shot copy submit + fence 有界等待——写已完成且可用,帧内
    // 无待序 host 写,初值同无数据 = NONE;image→UNDEFINED;带 staging 的 image
    // 在命令流首段迁 TRANSFER_DST 后 = TRANSFER_DST 态)。创建录制与重录同一
    // 确定性初值规则(仅由 ResourceDesc 决定,不依赖运行态)。
    let mut tracked: Vec<TrackedState> = p
        .resources
        .iter()
        .map(|r| match r {
            ResourceDesc::Buffer(b) => {
                if b.data.is_some() && !b.device_local {
                    (0, STAGE2_HOST, ACCESS2_HOST_WRITE)
                } else {
                    (0, STAGE2_NONE, 0)
                }
            }
            ResourceDesc::Texture(_) => (LAYOUT_UNDEFINED, STAGE2_NONE, 0),
        })
        .collect();
    // G14.12：跨界(exportable/D3D12 imported)image 的 layout 跨帧常驻 GENERAL
    // ——帧末收敛态 = 帧首初值,免掉每帧一次 UNDEFINED→GENERAL 全表面压缩元数据
    // 重初始化(1920×1080 三标实测 ≈0.3ms/帧)。初值仍只由资源种别 + 静态
    // release 集决定(不依赖运行态),创建录制与重录同一规则;建面期
    // initial_layout=UNDEFINED 由创建段 one-shot 迁移一次性补齐(见
    // create_persistent_frame 的 layout_init_imgs 段)。跨帧写-写序由上一帧帧末
    // EXTERNAL release(ALL_COMMANDS/MEMORY_WRITE 全域可用性)+ host 侧 fence
    // 等待共同保证,故本 pass 无需再补 layout 转换。
    // G14.12 **撤销**「跨界 image layout 跨帧常驻 GENERAL」优化(原 G14.11 ②,
    // 值 ~0.165ms/帧 @1920×1080 三标)——**确定性优先裁决**:锚重收割门实测
    // cornell-box/t50/dlss_sr(输入 256×256)双跑 digest 不等,5 跑 5 值;单变量
    // bisect 实证归因于本优化(恢复每帧 `UNDEFINED→GENERAL` 即回到双跑位级
    // 一致)。机理:每帧那次 UNDEFINED 转换会把 image 的压缩元数据/未写 padding
    // 重初始化为确定态;常驻 GENERAL 后这些从未被 pack 写到的位面保留跨运行
    // 不定的陈值,经 DLSS 时域累积放大为全图微幅抖动(实测 58.6% 像素有差但
    // 均值差仅 1.5e-3、总亮度几乎不变、边界 8px 带完全一致 = 数值抖动而非
    // 结构性垃圾)。RFC-0030 §4.8「漂移即弃」同律处置。
    // 后续可攻面(G15+ 承接):建面期一次性 `vkCmdClearColorImage` 把整幅(含
    // padding)定义为确定态,再恢复常驻 GENERAL——需 FFI 新增且须逐格复验,
    // 本期不追(余量已足:去本优化后 bistro t100 dlss 仍 ratio ≥1.05)。
    let _ = &p.exportable;
    // inline VB 跟踪(独立于 resources;上传后 = HOST_WRITE)。
    let mut inline_vb_tracked: Vec<TrackedState> = p
        .inline_vbs
        .iter()
        .map(|b| {
            if b.is_some() {
                (0, STAGE2_HOST, ACCESS2_HOST_WRITE)
            } else {
                (0, STAGE2_NONE, 0)
            }
        })
        .collect();

    // barrier2 批量录制助手:收集本批 image/buffer 转换后一次 DependencyInfo。
    macro_rules! flush_barriers {
        ($img_barriers:expr, $buf_barriers:expr) => {
            if !$img_barriers.is_empty() || !$buf_barriers.is_empty() {
                let di = DependencyInfo {
                    s_type: ST_DEPENDENCY_INFO,
                    p_next: std::ptr::null(),
                    dependency_flags: 0,
                    memory_barrier_count: 0,
                    p_memory_barriers: std::ptr::null(),
                    buffer_memory_barrier_count: $buf_barriers.len() as u32,
                    p_buffer_memory_barriers: if $buf_barriers.is_empty() {
                        std::ptr::null()
                    } else {
                        $buf_barriers.as_ptr()
                    },
                    image_memory_barrier_count: $img_barriers.len() as u32,
                    p_image_memory_barriers: if $img_barriers.is_empty() {
                        std::ptr::null()
                    } else {
                        $img_barriers.as_ptr()
                    },
                };
                (dev.cmd_barrier2)(cmd, &di);
            }
            $img_barriers.clear();
            $buf_barriers.clear();
        };
    }
    let mut img_barriers: Vec<ImageMemoryBarrier2> = Vec::new();
    let mut buf_barriers: Vec<BufferMemoryBarrier2> = Vec::new();
    // 资源转换收集(plan 回放与隐式补全共用;buffer/image 按类分流)。
    macro_rules! transit {
        ($res:expr, $to:expr) => {
            if let Some((old_layout, new_layout, src_stage, src_access, dst_stage, dst_access)) =
                barrier_fields(tracked[$res as usize], $to)
            {
                match &rt[$res as usize] {
                    RtRes::Img(ri) => img_barriers.push(ImageMemoryBarrier2 {
                        s_type: ST_IMAGE_MEMORY_BARRIER_2,
                        p_next: std::ptr::null(),
                        src_stage_mask: src_stage,
                        src_access_mask: src_access,
                        dst_stage_mask: dst_stage,
                        dst_access_mask: dst_access,
                        old_layout,
                        new_layout,
                        src_queue_family_index: QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                        image: ri.image,
                        subresource_range: VkImageSubresourceRange {
                            aspect_mask: ri.format.aspect_mask(),
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                    }),
                    RtRes::Buf(rb) => buf_barriers.push(BufferMemoryBarrier2 {
                        s_type: ST_BUFFER_MEMORY_BARRIER_2,
                        p_next: std::ptr::null(),
                        src_stage_mask: src_stage,
                        src_access_mask: src_access,
                        dst_stage_mask: dst_stage,
                        dst_access_mask: dst_access,
                        src_queue_family_index: QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                        buffer: rb.buffer,
                        offset: 0,
                        size: WHOLE_SIZE,
                    }),
                }
                tracked[$res as usize] = state_fields($to);
            }
        };
    }

    // ── 上传段(仅创建录制):image 初始数据 UNDEFINED→TRANSFER_DST + copy ──
    if p.record_upload_segment {
        for (i, r) in rt.iter().enumerate() {
            if let RtRes::Img(ri) = r
                && let Some(staging) = ri.staging
            {
                transit!(i as u32, TargetState::TransferDst);
                flush_barriers!(img_barriers, buf_barriers);
                let region = VkBufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource: VkImageSubresourceLayers {
                        aspect_mask: ri.format.aspect_mask(),
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
                    image_extent: VkExtent3D {
                        width: ri.width,
                        height: ri.height,
                        depth: 1,
                    },
                };
                (dev.cmd_copy_buf2img)(
                    cmd,
                    staging,
                    ri.image,
                    LAYOUT_TRANSFER_DST_OPTIMAL,
                    1,
                    &region,
                );
            }
        }
        // inline VB 上传(host 直写已于建面完成;首用前 HOST_WRITE→VERTEX_ATTRIBUTE_READ
        // 由下方逐 pass 隐式补全落到跟踪表 inline_vb_tracked)。
    }

    // ── TLAS update + consume barrier(仅重录带 update;创建期初始 build 走一次性
    // 创建 cmd,持久 cmd 不重复 build;G34-2 双 TLAS 面 = 主 update → 第二 update
    // → 单条 consume barrier,主 update 单件面命令流与既有逐字节同)──
    if let Some(ops) = as_ops.as_mut() {
        let mut any_update = false;
        if let Some(action) = ops.tlas_action {
            ops.mgr.record_tlas_update(ops.fns, cmd, action);
            any_update = true;
        }
        if let Some((mgr_b, action_b)) = ops.tlas_b.as_mut() {
            mgr_b.record_tlas_update(ops.fns, cmd, *action_b);
            any_update = true;
        }
        if any_update {
            ops.mgr
                .record_consume_barrier(ops.fns, cmd, PIPELINE_STAGE_COMPUTE_SHADER);
        }
    }

    // ── 逐 pass 录制:plan 逐字回放 → 隐式补全 → pass 本体 ──
    // G31+ 波 C Task C7:pass 标注面(debug utils label;符号在位才消费,CStr
    // 预建于循环外——录制期一次性,absent = 零开销跳过不分配;label 包裹
    // timestamp 区间 + pass 本体,Nsight/RenderDoc 逐 pass 可辨识)。
    let label_names: Vec<std::ffi::CString> = if dev.labels_active() {
        p.passes
            .iter()
            .map(|pass| {
                std::ffi::CString::new(pass_name(pass))
                    .unwrap_or_else(|_| std::ffi::CString::new("rurix_pass").expect("字面量无 NUL"))
            })
            .collect()
    } else {
        Vec::new()
    };
    for (pi, pass) in p.passes.iter().enumerate() {
        // ① plan 逐字回放(不重排;调用方图编译器产物)。
        for &(res, state) in p.barriers[pi] {
            transit!(res, state);
        }
        // ② 隐式补全(pass 需求态与跟踪态不一致者补一条;确定性固定规则,模块头契约;
        // effective bindings 决定本帧实际资源需求)。
        for (res, state) in pass_requirements_with(pass, &p.effective_bindings[pi]) {
            transit!(res, state);
        }
        // inline VB 首用转换(独立跟踪表;HOST_WRITE→VERTEX_ATTRIBUTE_READ)。
        if let Pass::Raster(rp) = pass
            && matches!(rp.vertex, VertexData::Inline { .. })
            && p.inline_vbs[pi].is_some()
        {
            let from = inline_vb_tracked[pi];
            let to = state_fields(TargetState::VertexInput);
            if from != to {
                buf_barriers.push(BufferMemoryBarrier2 {
                    s_type: ST_BUFFER_MEMORY_BARRIER_2,
                    p_next: std::ptr::null(),
                    src_stage_mask: from.1,
                    src_access_mask: from.2,
                    dst_stage_mask: to.1,
                    dst_access_mask: to.2,
                    src_queue_family_index: QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                    buffer: p.inline_vbs[pi].expect("inline vb 存在(上判)"),
                    offset: 0,
                    size: WHOLE_SIZE,
                });
                inline_vb_tracked[pi] = to;
            }
        }
        flush_barriers!(img_barriers, buf_barriers);

        // ③ pass 本体;timestamp 覆盖该 pass 的实际 GPU 命令区间。
        // G31+ 波 C Task C7:debug label begin(pass 名;label_names 预建——
        // labels_active ⇔ label_names.len() == passes.len() 同源)。
        if let (Some(beg), Some(_)) = (dev.cmd_begin_label, dev.cmd_end_label) {
            let label = DebugUtilsLabelEXT {
                s_type: ST_DEBUG_UTILS_LABEL_EXT,
                p_next: std::ptr::null(),
                p_label_name: label_names[pi].as_ptr(),
                color: [0.0; 4],
            };
            // SAFETY: cmd 处于录制态;label 指针调用期内有效(驱动同步拷贝
            // label 串);VK_EXT_debug_utils 已启用(装载门 fail-silent)。
            beg(cmd, &label);
        }
        (dev.cmd_write_timestamp2)(
            cmd,
            STAGE2_ALL_COMMANDS,
            query_pool,
            p.query_base + (pi as u32) * 2,
        );
        let setup = &p.setups[pi];
        let push_constants = &p.effective_bindings[pi].push_constants;
        match pass {
            Pass::Raster(rp) => {
                let rpbi = RenderPassBeginInfo {
                    s_type: ST_RENDER_PASS_BEGIN_INFO,
                    p_next: std::ptr::null(),
                    render_pass: setup.rp,
                    framebuffer: setup.fb,
                    render_area: VkRect2D {
                        offset: VkOffset2D { x: 0, y: 0 },
                        extent: VkExtent2D {
                            width: setup.extent.0,
                            height: setup.extent.1,
                        },
                    },
                    clear_value_count: setup.clears.len() as u32,
                    p_clear_values: if setup.clears.is_empty() {
                        std::ptr::null()
                    } else {
                        setup.clears.as_ptr()
                    },
                };
                (dev.cmd_begin_rp)(cmd, &rpbi, SUBPASS_CONTENTS_INLINE);
                (dev.cmd_bind_pipe)(cmd, PIPELINE_BIND_POINT_GRAPHICS, setup.pipe);
                // G31:流水路逐 pass 优先绑本 slot override set(内容经创建期
                // 同一 write_pass_descriptor_set 事实源写入,与共享 set 重写
                // 产物逐位同 ⇒ 位级零漂移);顺序路 slot_set_overrides=None 恒
                // 落回 setup.set(0-byte)。
                let set = p
                    .slot_set_overrides
                    .and_then(|o| o.get(pi).copied().flatten())
                    .or(setup.set);
                if let Some(set) = set {
                    (dev.cmd_bind_ds)(
                        cmd,
                        PIPELINE_BIND_POINT_GRAPHICS,
                        setup.pl,
                        0,
                        1,
                        &set,
                        0,
                        std::ptr::null(),
                    );
                }
                if !push_constants.is_empty() {
                    (dev.cmd_push)(
                        cmd,
                        setup.pl,
                        SHADER_STAGE_RFX,
                        0,
                        setup.pc_size,
                        push_constants.as_ptr().cast::<c_void>(),
                    );
                }
                match &rp.vertex {
                    VertexData::Inline { .. } => {
                        let vb = p.inline_vbs[pi].expect("inline vb 已建");
                        let offsets: VkDeviceSize = 0;
                        (dev.cmd_bind_vb)(cmd, 0, 1, &vb, &offsets);
                    }
                    VertexData::Resource { res, offset, .. } => {
                        let RtRes::Buf(rb) = &rt[*res as usize] else {
                            return Err(format!("pass {pi}: vertex buffer {res} 非 buffer"));
                        };
                        let off: VkDeviceSize = *offset;
                        (dev.cmd_bind_vb)(cmd, 0, 1, &rb.buffer, &off);
                    }
                    VertexData::Pull => {}
                }
                match rp.draw {
                    DrawSpec::Direct {
                        vertex_count,
                        instance_count,
                        first_vertex,
                        first_instance,
                    } => {
                        (dev.cmd_draw)(
                            cmd,
                            vertex_count,
                            instance_count,
                            first_vertex,
                            first_instance,
                        );
                    }
                    DrawSpec::Indirect { res, offset } => {
                        let RtRes::Buf(rb) = &rt[res as usize] else {
                            return Err(format!("pass {pi}: indirect {res} 非 buffer"));
                        };
                        (dev.cmd_draw_indirect)(cmd, rb.buffer, offset, 1, 16);
                    }
                }
                (dev.cmd_end_rp)(cmd);
            }
            Pass::Compute(cp) => {
                (dev.cmd_bind_pipe)(cmd, PIPELINE_BIND_POINT_COMPUTE, setup.pipe);
                // G31:流水路逐 pass 优先绑本 slot override set(内容经创建期
                // 同一 write_pass_descriptor_set 事实源写入,与共享 set 重写
                // 产物逐位同 ⇒ 位级零漂移);顺序路 slot_set_overrides=None 恒
                // 落回 setup.set(0-byte)。
                let set = p
                    .slot_set_overrides
                    .and_then(|o| o.get(pi).copied().flatten())
                    .or(setup.set);
                if let Some(set) = set {
                    (dev.cmd_bind_ds)(
                        cmd,
                        PIPELINE_BIND_POINT_COMPUTE,
                        setup.pl,
                        0,
                        1,
                        &set,
                        0,
                        std::ptr::null(),
                    );
                }
                if !push_constants.is_empty() {
                    (dev.cmd_push)(
                        cmd,
                        setup.pl,
                        SHADER_STAGE_RFX,
                        0,
                        setup.pc_size,
                        push_constants.as_ptr().cast::<c_void>(),
                    );
                }
                match cp.dispatch {
                    DispatchSpec::Direct(g) => {
                        (dev.cmd_dispatch)(cmd, g[0], g[1], g[2]);
                    }
                    DispatchSpec::Indirect { res, offset } => {
                        let RtRes::Buf(rb) = &rt[res as usize] else {
                            return Err(format!("pass {pi}: indirect {res} 非 buffer"));
                        };
                        (dev.cmd_dispatch_indirect)(cmd, rb.buffer, offset);
                    }
                }
            }
        }
        (dev.cmd_write_timestamp2)(
            cmd,
            STAGE2_ALL_COMMANDS,
            query_pool,
            p.query_base + (pi as u32) * 2 + 1,
        );
        // G31+ 波 C Task C7:debug label end(与 begin 同 cmd 同录制态配对)。
        if let Some(end) = dev.cmd_end_label {
            // SAFETY: cmd 处于录制态;与上方 begin label 配对(begin 未录则
            // 本分支同条件不进入——双符号同在判定同源)。
            end(cmd);
        }
        // ── G31+ 波 B Task B5:BLAS 顶点 refit 桥(蒙皮通路)——本 pass
        // (蒙皮 compute)录完后:① 蒙皮输出 SSBO → TransferSrc(tracked 一致);
        // ② vbuf ACCEL_READ→TRANSFER_WRITE;③ vkCmdCopyBuffer 桥接;④ vbuf
        // TRANSFER_WRITE→ACCEL_READ;⑤ UPDATE 模式原地 build;⑥ consume
        // barrier(AS_WRITE→AS_READ,COMPUTE_SHADER——后续 ray query pass 读
        // 新 BLAS 内容;record_tlas_update 同律)。timestamp 区间不包本桥
        // (逐 pass GPU 分段口径不变;refit GPU 段计入帧墙钟,如实计量)。
        if let Some(ops) = as_ops.as_mut()
            && let Some(br) = ops.blas_refit
            && br.after_pass == pi as u32
        {
            let RtRes::Buf(src_rb) = &rt[br.src_res as usize] else {
                return Err(format!(
                    "BLAS refit: src 资源 {} 非 buffer(校验漏网)",
                    br.src_res
                ));
            };
            let vbuf = ops.mgr.blas_vertex_buffer(br.blas_index)?;
            // G38 T3:桥接时戳(query 追加区,先 reset 再写——同 cmd 内合法;
            // 逐 pass 时戳口径上方原样不动)。三点:桥首/copy 后/build 后。
            let bridge_q = bridge.as_ref().and_then(|b| b.query_base);
            if let Some(qb) = bridge_q {
                (dev.cmd_reset_query_pool)(cmd, query_pool, qb, BRIDGE_QUERY_COUNT);
                (dev.cmd_write_timestamp2)(cmd, STAGE2_ALL_COMMANDS, query_pool, qb);
            }
            // G38 T3:脏区段列表(None = 既有单 region 全量,命令流逐字节
            // 不变;Some 空 = 本帧无脏字节,跳过桥接 copy 三步——vbuf 已与
            // src 同步,UPDATE build 照录,终态与全量 copy 位级等价)。
            let regions = bridge.as_ref().and_then(|b| b.regions);
            if regions.is_none_or(|r| !r.is_empty()) {
                transit!(br.src_res, TargetState::TransferSrc);
                flush_barriers!(img_barriers, buf_barriers);
                // vbuf 前态 = ACCEL build 读(初始 build/上一帧 refit;创建期
                // one-shot submit + fence 已完成)——逐帧稳态自洽。
                let pre_copy = BufferMemoryBarrier2 {
                    s_type: ST_BUFFER_MEMORY_BARRIER_2,
                    p_next: std::ptr::null(),
                    src_stage_mask: STAGE2_ACCEL_STRUCTURE_BUILD,
                    src_access_mask: ACCESS2_ACCEL_STRUCTURE_READ,
                    dst_stage_mask: STAGE2_TRANSFER,
                    dst_access_mask: ACCESS2_TRANSFER_WRITE,
                    src_queue_family_index: QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                    buffer: vbuf,
                    offset: 0,
                    size: br.byte_len,
                };
                let di = DependencyInfo {
                    s_type: ST_DEPENDENCY_INFO,
                    p_next: std::ptr::null(),
                    dependency_flags: 0,
                    memory_barrier_count: 0,
                    p_memory_barriers: std::ptr::null(),
                    buffer_memory_barrier_count: 1,
                    p_buffer_memory_barriers: &pre_copy,
                    image_memory_barrier_count: 0,
                    p_image_memory_barriers: std::ptr::null(),
                };
                (dev.cmd_barrier2)(cmd, &di);
                match regions {
                    // 既有单 region 全量桥(逐字保序)。
                    None => {
                        let region = VkBufferCopy {
                            src_offset: br.src_offset,
                            dst_offset: 0,
                            size: br.byte_len,
                        };
                        (dev.cmd_copy_buf)(cmd, src_rb.buffer, vbuf, 1, &region);
                    }
                    // G38 T3:一次 vkCmdCopyBuffer 携脏区段数组(src 与 vbuf
                    // 同布局:src_offset+off → off;区段合法性已过
                    // validate_bridge_ext fail-closed)。
                    Some(rs) => {
                        let vk_regions: Vec<VkBufferCopy> = rs
                            .iter()
                            .map(|&(off, len)| VkBufferCopy {
                                src_offset: br.src_offset + off,
                                dst_offset: off,
                                size: len,
                            })
                            .collect();
                        (dev.cmd_copy_buf)(
                            cmd,
                            src_rb.buffer,
                            vbuf,
                            vk_regions.len() as u32,
                            vk_regions.as_ptr(),
                        );
                    }
                }
                let post_copy = BufferMemoryBarrier2 {
                    src_stage_mask: STAGE2_TRANSFER,
                    src_access_mask: ACCESS2_TRANSFER_WRITE,
                    dst_stage_mask: STAGE2_ACCEL_STRUCTURE_BUILD,
                    dst_access_mask: ACCESS2_ACCEL_STRUCTURE_READ,
                    ..pre_copy
                };
                let di = DependencyInfo {
                    buffer_memory_barrier_count: 1,
                    p_buffer_memory_barriers: &post_copy,
                    ..di
                };
                (dev.cmd_barrier2)(cmd, &di);
            }
            if let Some(qb) = bridge_q {
                (dev.cmd_write_timestamp2)(cmd, STAGE2_ALL_COMMANDS, query_pool, qb + 1);
            }
            ops.mgr.record_blas_refit(ops.fns, cmd, br.blas_index)?;
            ops.mgr
                .record_consume_barrier(ops.fns, cmd, PIPELINE_STAGE_COMPUTE_SHADER);
            if let Some(qb) = bridge_q {
                (dev.cmd_write_timestamp2)(cmd, STAGE2_ALL_COMMANDS, query_pool, qb + 2);
            }
        }
        // ── G37 W3 hzb_skin:第二 BLAS refit 桥(表 1 manager——双 TLAS×蒙皮
        // 合并车道:主射线表 0 与阴影射线表 1 各持 BLAS 副本,蒙皮角色两副本
        // 同帧 refit;桥接六步与上方 blas_refit 逐字同律,目标 = tlas_b 的
        // manager;src 同资源同帧双桥时第二次 transit 为幂等无操作)──
        if let Some(ops) = as_ops.as_mut()
            && let Some(br) = ops.blas_refit_b
            && br.after_pass == pi as u32
        {
            let Some((mgr_b, _)) = ops.tlas_b.as_mut() else {
                return Err("BLAS refit_b: 无表 1 manager(校验漏网)".into());
            };
            let RtRes::Buf(src_rb) = &rt[br.src_res as usize] else {
                return Err(format!(
                    "BLAS refit_b: src 资源 {} 非 buffer(校验漏网)",
                    br.src_res
                ));
            };
            let vbuf = mgr_b.blas_vertex_buffer(br.blas_index)?;
            transit!(br.src_res, TargetState::TransferSrc);
            flush_barriers!(img_barriers, buf_barriers);
            let pre_copy = BufferMemoryBarrier2 {
                s_type: ST_BUFFER_MEMORY_BARRIER_2,
                p_next: std::ptr::null(),
                src_stage_mask: STAGE2_ACCEL_STRUCTURE_BUILD,
                src_access_mask: ACCESS2_ACCEL_STRUCTURE_READ,
                dst_stage_mask: STAGE2_TRANSFER,
                dst_access_mask: ACCESS2_TRANSFER_WRITE,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                buffer: vbuf,
                offset: 0,
                size: br.byte_len,
            };
            let di = DependencyInfo {
                s_type: ST_DEPENDENCY_INFO,
                p_next: std::ptr::null(),
                dependency_flags: 0,
                memory_barrier_count: 0,
                p_memory_barriers: std::ptr::null(),
                buffer_memory_barrier_count: 1,
                p_buffer_memory_barriers: &pre_copy,
                image_memory_barrier_count: 0,
                p_image_memory_barriers: std::ptr::null(),
            };
            (dev.cmd_barrier2)(cmd, &di);
            let region = VkBufferCopy {
                src_offset: br.src_offset,
                dst_offset: 0,
                size: br.byte_len,
            };
            (dev.cmd_copy_buf)(cmd, src_rb.buffer, vbuf, 1, &region);
            let post_copy = BufferMemoryBarrier2 {
                src_stage_mask: STAGE2_TRANSFER,
                src_access_mask: ACCESS2_TRANSFER_WRITE,
                dst_stage_mask: STAGE2_ACCEL_STRUCTURE_BUILD,
                dst_access_mask: ACCESS2_ACCEL_STRUCTURE_READ,
                ..pre_copy
            };
            let di = DependencyInfo {
                buffer_memory_barrier_count: 1,
                p_buffer_memory_barriers: &post_copy,
                ..di
            };
            (dev.cmd_barrier2)(cmd, &di);
            mgr_b.record_blas_refit(ops.fns, cmd, br.blas_index)?;
            mgr_b.record_consume_barrier(ops.fns, cmd, PIPELINE_STAGE_COMPUTE_SHADER);
        }
    }

    // ── readback 段:image 迁 TRANSFER_SRC + copy 到 readback buffer;host-visible
    // buffer 免录制(collect/execute 直接 map 资源本体);DEVICE_LOCAL buffer
    // (G14.10d)= 帧尾 copy 到 session 级 cached readback staging(不可 map,
    // staging 即回读事实源;copy 字节精确 → 位级零漂移)──
    // 创建录制(cleanup=Some)顺带建持久 readback staging(cached 优选 TRANSFER_DST)
    // 并登记 ledger;重录路 rb_buffers 已逐位对齐,不新建分配——readback_subset
    // 动态子集下 staging 布局按 session readback 表项固定预分配,copy 只录
    // subset 内的(rb_buffers 有效项即录制判据,创建/重录同一确定性规则)。
    for (i, rb) in p.readbacks.iter().enumerate() {
        match *rb {
            Readback::Texture { res } => {
                let Some(ri) = rt[res as usize].image() else {
                    return Err("readback: texture 资源非 image".to_owned());
                };
                if let Some(c) = cleanup.as_deref_mut() {
                    let sz = (ri.width as u64)
                        * (ri.height as u64)
                        * (ri.format.bytes_per_texel() as u64);
                    let (rbuf, rmem) = create_device_buffer(
                        dev,
                        p.device,
                        p.memprops,
                        sz.max(4),
                        0x2, // TRANSFER_DST
                        None,
                        None,
                        c,
                        // readback 专用(GPU copy 写、CPU map 读)→ cached 优选。
                        BufferMemClass::HostCachedPreferred,
                    )?;
                    rb_buffers.push(Some((rbuf, rmem)));
                }
                transit!(res, TargetState::TransferSrc);
            }
            Readback::Buffer { res, size, .. } => {
                if let Some(c) = cleanup.as_deref_mut() {
                    let staged = matches!(
                        p.resources.get(res as usize),
                        Some(ResourceDesc::Buffer(b)) if b.device_local
                    );
                    if staged {
                        let (rbuf, rmem) = create_device_buffer(
                            dev,
                            p.device,
                            p.memprops,
                            size.max(4),
                            0x2, // TRANSFER_DST
                            None,
                            None,
                            c,
                            // readback staging(GPU copy 写、CPU map 读)→ cached 优选。
                            BufferMemClass::HostCachedPreferred,
                        )?;
                        rb_buffers.push(Some((rbuf, rmem)));
                    } else {
                        rb_buffers.push(None);
                    }
                }
                // staging 有效项 = DEVICE_LOCAL 源:pass 链写完后迁 TRANSFER_READ
                // (SHADER_WRITE→TRANSFER barrier),copy 于 flush 后录制。
                if rb_buffers[i].is_some() {
                    transit!(res, TargetState::TransferSrc);
                }
            }
        }
    }
    flush_barriers!(img_barriers, buf_barriers);
    for (i, rb) in p.readbacks.iter().enumerate() {
        if let (Readback::Buffer { res, offset, size }, Some((buf, _))) = (rb, rb_buffers[i]) {
            let RtRes::Buf(src) = &rt[*res as usize] else {
                return Err(format!("readbacks[{i}]: 资源号 {res} 非 buffer"));
            };
            let region = VkBufferCopy {
                src_offset: *offset,
                dst_offset: 0,
                size: *size,
            };
            (dev.cmd_copy_buf)(cmd, src.buffer, buf, 1, &region);
        }
        if let (Readback::Texture { res }, Some((buf, _))) = (rb, rb_buffers[i]) {
            let Some(ri) = rt[*res as usize].image() else {
                return Err(format!("readbacks[{i}]: 资源号 {res} 非 texture"));
            };
            let region = VkBufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: VkImageSubresourceLayers {
                    aspect_mask: ri.format.aspect_mask(),
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
                image_extent: VkExtent3D {
                    width: ri.width,
                    height: ri.height,
                    depth: 1,
                },
            };
            (dev.cmd_copy_img2buf)(cmd, ri.image, LAYOUT_TRANSFER_SRC_OPTIMAL, buf, 1, &region);
        }
    }

    // ── G14.10b exportable 帧末段(集空 = 0 命令,既有命令流 0-byte)──
    // ① layout→GENERAL 收敛(跨界 layout 协定恒 GENERAL:release/acquire 两侧
    //   old/newLayout 规范要求一致,固定 GENERAL 免跨模块状态协商;跟踪态已
    //   GENERAL 时 barrier_fields 返 None 自然跳过);② EXTERNAL release
    //   (qfi→VK_QUEUE_FAMILY_EXTERNAL,GENERAL→GENERAL 零转换,src=ALL_COMMANDS/
    //   MEMORY_WRITE 全域可用性;release 侧 dst scope 规范上被忽略,取合法保守值)。
    // 下一帧本 session 重写该 image 时按既有状态机从 UNDEFINED 重初始化(内容
    // 不保留语义),规范允许跳过 re-acquire——与「每帧 UNDEFINED 初值」既有
    // 纪律天然一致,无需帧首 acquire。
    for &res in p.exportable {
        // G14.10f exportable buffer:EXTERNAL release(buffer 无 layout 面,
        // 仅所有权/可用性;导入方消费前录对应 acquire——契约同 image)。
        if let RtRes::Buf(rb) = &rt[res as usize] {
            buf_barriers.push(BufferMemoryBarrier2 {
                s_type: ST_BUFFER_MEMORY_BARRIER_2,
                p_next: std::ptr::null(),
                src_stage_mask: STAGE2_ALL_COMMANDS,
                src_access_mask: ACCESS2_MEMORY_WRITE,
                dst_stage_mask: STAGE2_ALL_COMMANDS,
                dst_access_mask: 0,
                src_queue_family_index: p.queue_family_index,
                dst_queue_family_index: QUEUE_FAMILY_EXTERNAL,
                buffer: rb.buffer,
                offset: 0,
                size: !0u64, // VK_WHOLE_SIZE
            });
            continue;
        }
        transit!(res, TargetState::StorageImageReadWrite);
        let RtRes::Img(ri) = &rt[res as usize] else {
            return Err(format!("exportable {res} 非 image/buffer(校验漏网)"));
        };
        img_barriers.push(ImageMemoryBarrier2 {
            s_type: ST_IMAGE_MEMORY_BARRIER_2,
            p_next: std::ptr::null(),
            src_stage_mask: STAGE2_ALL_COMMANDS,
            src_access_mask: ACCESS2_MEMORY_WRITE,
            dst_stage_mask: STAGE2_ALL_COMMANDS,
            dst_access_mask: 0,
            old_layout: LAYOUT_GENERAL,
            new_layout: LAYOUT_GENERAL,
            src_queue_family_index: p.queue_family_index,
            dst_queue_family_index: QUEUE_FAMILY_EXTERNAL,
            image: ri.image,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: ri.format.aspect_mask(),
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        });
    }
    flush_barriers!(img_barriers, buf_barriers);
    Ok(())
}

/// [`DeviceFrameSession::execute_with_frame_update`] 的 native 侧已校验更新包
/// (全部引用调用方栈上数据,生命周期限于单次提交;`effective_readbacks[k]` 对应
/// session readbacks 下标 `effective_rb_sources[k]`)。
struct PreparedFrameUpdate<'a> {
    /// buffer 上传段((资源下标, offset, 字节);submit 前 host memcpy)。
    uploads: &'a [(u32, u64, &'a [u8])],
    /// TLAS update((AS 表下标, 实例 transforms, 动作))。
    tlas: Option<(u32, &'a [RayQueryTransformedInstanceDesc], TlasBuildAction)>,
    /// G34-2 加性:第二 TLAS update(双 TLAS 车道表 1;校验面已保证与
    /// `tlas`/`blas` 异槽;`None` = 既有面 0-byte)。
    tlas_b: Option<(u32, &'a [RayQueryTransformedInstanceDesc], TlasBuildAction)>,
    /// G31+ 波 B Task B5:BLAS 顶点 refit((AS 表下标, blas 下标, src 资源
    /// 下标, src_offset, byte_len, after_pass);录制面在 pass 循环
    /// after_pass 后插入桥接 copy + UPDATE build + consume barrier)。
    blas: Option<(u32, u32, u32, u64, u64, u32)>,
    /// G37 W3 hzb_skin 加性:第二 BLAS refit(同形;目标 = `tlas_b` 槽
    /// manager,校验面已保证同现同槽;`None` = 既有面 0-byte)。
    blas_b: Option<(u32, u32, u32, u64, u64, u32)>,
    /// 需在 submit 前重写 descriptor set 的 pass 下标(binding_overrides 命中)。
    descriptor_overrides: &'a [u32],
    /// 各 pass 本帧 effective bindings(override 已应用)。
    effective_bindings: &'a [Bindings],
    /// 本帧实际 readback 请求(readback_subset 解析后;空 = 不 readback)。
    effective_readbacks: &'a [Readback],
    /// `effective_readbacks[k]` 对应 session readbacks 下标(rb_buffers 对齐用)。
    effective_rb_sources: &'a [usize],
    /// 本帧是否重录命令体(false = 原样重放,仅可能有 host 上传)。
    needs_rerecord: bool,
}

/// 持久 session 的 AS 面(G7.6 Wave B;vk.rs **同一单所有者** `VkAsManager` 逐表项
/// 持有,禁第二套 BVH;`VkAsFns` device 符号表一次加载)。
struct PersistentAsState {
    fns: VkAsFns,
    managers: Vec<VkAsManager>,
}

struct NativeDeviceFrame {
    dev: Dev,
    queue: VkQueue,
    cleanup: Cleanup,
    rt: Vec<RtRes>,
    rb_buffers: Vec<Option<(VkBuffer, VkDeviceMemory)>>,
    cmd: VkCommandBuffer,
    resource_allocations: Vec<StableAllocationId>,
    /// per-pass 装配(FrameUpdate 重录复用;pipeline/descriptor set 跨帧不变)。
    setups: Vec<PassSetup>,
    /// inline VB 句柄(重录路 vertex 绑定复用)。
    inline_vbs: Vec<Option<VkBuffer>>,
    /// G14.10b exportable 纹理簿记(资源下标, allocation 字节, memory type index;
    /// 空 = 无导出面,全旧行为)。record_frame_body 帧末 release barrier 与
    /// win32 导出 accessor 共用事实源。
    exportable_meta: Vec<(u32, u64, u32)>,
    /// G14.10f exportable buffer 声明尺寸簿记(资源下标, 声明字节;导出
    /// accessor 的 [`ExportedBufferWin32::size`] 事实源——allocation 有对齐,
    /// 声明尺寸须单列)。
    exportable_buf_sizes: Vec<(u32, u64)>,
    /// G14.11 D3D12 导入纹理下标(空 = 无导入面;帧末 EXTERNAL release 集 =
    /// exportable ∪ imported——D3D12 消费窗内容有效性同律)。
    imported_indices: Vec<u32>,
    /// 建面 queue family(release barrier 的 src 家族;既有单 graphics queue 策略)。
    queue_family_index: u32,
    /// G14.12:`cmd` 当前所载命令体的 readback 源集形(`None` = 创建期录制,含
    /// 初始上传段,不可原样重放跳过重录)。`readback_subset` 逐帧同形(驻留车道
    /// 恒 `Some([])`)且无 TLAS/binding/push override 时,命令体逐字节不变——
    /// 据此跳过 `vkResetCommandBuffer`+重录,省下每帧 CPU 录制税(1080p 三 pass
    /// 驻留车道实测 ≈0.23ms/帧)。host 上传段在 cmd 外(重录路
    /// `record_upload_segment=false`),跳过重录不影响逐帧参数写入。
    recorded_rb_sources: Option<Vec<usize>>,
}

/// FIF 流水 per-slot 资源(G14plus RFC-0030 §4.3 L2;懒建于首次
/// [`DeviceFrameSession::submit_with_frame_update`]——纯顺序 session 零对象/
/// 零分配增量,既有 telemetry 计数 0-byte)。所有缓冲经 `create_device_buffer`
/// 建面 → cleanup 登记(对象/ledger 计数与销毁单点纪律不变);cmd 出自同一
/// cmdpool(RESET flag 已开,随 pool 销毁)。
///
/// per-slot 化裁决(按实际代码判断,注释说明):
/// - **cmd + timestamp query 区间**:slot 独立(单条 cmd 在 FIF=2 下重录竞争
///   在飞帧;query 区间 `[slot*passes*2, ..)` 同池分段,免第二池对象)。
/// - **上传缓冲(params 类)**:per-slot host staging + cmd 首段 GPU copy 至目标
///   SSBO——帧 N 在飞时帧 N+1 host 只写自己的 staging 槽,GPU copy 按队列序
///   落在帧 N 之后(帧间守卫 barrier),共享 SSBO 无 host/GPU 竞争,绑定不变。
/// - **readback staging**:per-slot(Buffer readback 原为直接 map 共享 SSBO——
///   FIF 下在飞帧改写同一 SSBO 即竞争,故帧尾 GPU copy 至 slot staging 后 map;
///   Texture readback 的 rb buffer 同理 per-slot 化)。
/// - **descriptor set**:SSBO 常驻绑定不变面无需 per-slot(session set 创建期
///   一次写入,staged 上传/回读均为 copy,不改绑定);G31 起 `binding_overrides`
///   经 per-slot override set 入流水(`override_sets` 懒分配,见
///   [`ensure_pipelined_override_set`]——内容与顺序路共享 set 重写产物逐位同)。
struct PipelinedSlot {
    cmd: VkCommandBuffer,
    /// G31:per-slot descriptor override set(逐 pass 懒分配;binding_overrides
    /// 入流水的承载——共享 session set 在飞帧使用中不可重写,本 slot set 的上次
    /// GPU 使用 = 本 slot 上一帧,submit 期 fence 已等待,host 重写无在途竞争)。
    override_sets: Vec<Option<VkDescriptorSet>>,
    /// 上传 staging(host 写 → cmd 首段 copy;(buffer, memory, 容量) grow-only,
    /// 扩容仅发生于 slot fence 已等待后——旧分配无在途使用,cleanup 登记同步换新)。
    upload_staging: Option<(VkBuffer, VkDeviceMemory, u64)>,
    /// 逐 session readback 的 per-slot staging(与 session readbacks 等长;
    /// Buffer = SSBO 区段 copy 目的,Texture = image copy 目的;尺寸 = 声明尺寸,
    /// cached 优选——G14plus §4.3 L1 同型)。
    rb_staging: Vec<(VkBuffer, VkDeviceMemory)>,
}

struct NativePersistentFrame {
    instance: VkInstance,
    destroy_instance: FnDestroyInstance,
    messenger: VkDebugUtilsMessengerEXT,
    destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT>,
    validation_errors: Box<std::sync::atomic::AtomicU64>,
    pd: VkPhysicalDevice,
    get_mem2: FnGetPhysicalDeviceMemoryProperties2,
    device: VkDevice,
    destroy_device: FnDestroyDevice,
    frame: NativeDeviceFrame,
    fences: Vec<VkFence>,
    next_slot: usize,
    timestamp_period_ns: f32,
    /// G38 T3:BLAS refit 桥接时戳 query 追加区首下标(= 创建期逐 pass 区总数
    /// `passes*2*slots`;追加区恒 [`BRIDGE_QUERY_COUNT`] 个,只在顺序路桥接
    /// 计时开启帧内 reset+写——既有逐 pass/FIF slot 区间口径不动)。
    bridge_query_base: u32,
    /// 物理设备内存属性(创建期一次性查询;FrameUpdate 重录的 record_frame_body 入参,
    /// 避免每帧重查)。
    memprops: PhysicalDeviceMemoryProperties,
    /// 扩展表:`resources` 各 allocation 后接 AS 表项代表 allocation(TLAS storage)。
    resource_allocations: Vec<StableAllocationId>,
    /// session AS 面(G7.6 Wave B;`None` = 无 AS 表,全旧行为)。
    as_state: Option<PersistentAsState>,
    /// FIF 流水 per-slot 面(G14plus L2;下标 = frame slot,懒建)。
    pipelined_slots: Vec<Option<PipelinedSlot>>,
    /// 已 submit 未 collect 的 slot(票据在外)。fail-closed 纪律:同 slot 重
    /// submit → Err(fence 已 reset 会悬垂 collect);顺序 execute 入口在任何
    /// 票据未清时 → Err(共享 query 区间/fence 轮转不可交错)。
    slot_busy: Vec<bool>,
    /// G31:FIF 流水 per-slot override descriptor set 专用池(懒建于首个带
    /// binding_overrides 的流水 submit;VK_NULL_HANDLE = 未建——纯顺序 session
    /// 零对象增量纪律同 PipelinedSlot;Drop 单点销毁,set 随池释放不单独 free)。
    pipelined_pool: VkDescriptorPool,
    /// G14.10b:物理设备 LUID(创建期 `VkPhysicalDeviceIDProperties` 实采;
    /// `None` = 驱动报 LUID 无效)。
    device_luid: Option<[u8; 8]>,
    /// G14.10b:`vkGetMemoryWin32HandleKHR`(仅 exportable session 解析;
    /// `None` = 无导出面)。
    get_memory_win32: Option<FnGetMemoryWin32HandleKHR>,
    /// G14.10b:已导出 NT handle 缓存((资源下标, handle 地址);Drop 单点
    /// CloseHandle,重复导出请求返缓存——免句柄泄漏)。
    exported_handles: Vec<(u32, usize)>,
}

impl NativePersistentFrame {
    /// session AS 表项数。
    fn as_count(&self) -> usize {
        self.as_state.as_ref().map_or(0, |s| s.managers.len())
    }

    /// G14.10b:exportable 纹理 Win32 NT handle 导出本体(缓存;契约见
    /// [`DeviceFrameSession::export_texture_win32_handle`])。
    fn export_texture_win32_handle(
        &mut self,
        resource_index: usize,
    ) -> Result<ExportedTextureWin32, String> {
        let Some(&(res, alloc_size, mem_type)) = self
            .frame
            .exportable_meta
            .iter()
            .find(|&&(r, _, _)| r as usize == resource_index)
        else {
            return Err(format!(
                "资源 {resource_index} 未声明为 exportable(new_with_exportable_textures)"
            ));
        };
        let img = self
            .frame
            .rt
            .get(resource_index)
            .and_then(RtRes::image)
            .ok_or_else(|| format!("资源 {resource_index} 非 image(exportable 簿记不一致)"))?;
        // 描述面(尺寸/格式/usage 与建面同一事实源——RtImage 建面时定格)。
        let mut out = ExportedTextureWin32 {
            handle: 0,
            width: img.width,
            height: img.height,
            vk_format: img.format.vk_format(),
            usage_flags: img.usage_flags,
            allocation_size: alloc_size,
            memory_type_index: mem_type,
        };
        let memory = img.mem;
        if let Some(&(_, h)) = self.exported_handles.iter().find(|&&(r, _)| r == res) {
            out.handle = h;
            return Ok(out);
        }
        let get_fn = self
            .get_memory_win32
            .ok_or("vkGetMemoryWin32HandleKHR 未解析(非 exportable session)")?;
        let info = MemoryGetWin32HandleInfoKHR {
            s_type: ST_MEMORY_GET_WIN32_HANDLE_INFO_KHR,
            p_next: std::ptr::null(),
            memory,
            handle_type: EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
        };
        let mut handle: *mut c_void = std::ptr::null_mut();
        // SAFETY: device 存活(self 持有);info 栈上存活;memory 为本 session
        // exportable 分配(建面挂 VkExportMemoryAllocateInfo);出参栈上有效写。
        let r = unsafe { get_fn(self.device, &info, &mut handle) };
        if r != VK_SUCCESS || handle.is_null() {
            return Err(format!("vkGetMemoryWin32HandleKHR 失败: {r}"));
        }
        self.exported_handles.push((res, handle as usize));
        out.handle = handle as usize;
        Ok(out)
    }

    /// G14.10f:exportable buffer Win32 NT handle 导出本体(缓存;契约见
    /// [`DeviceFrameSession::export_buffer_win32_handle`])。
    fn export_buffer_win32_handle(
        &mut self,
        resource_index: usize,
    ) -> Result<ExportedBufferWin32, String> {
        let Some(&(res, alloc_size, mem_type)) = self
            .frame
            .exportable_meta
            .iter()
            .find(|&&(r, _, _)| r as usize == resource_index)
        else {
            return Err(format!(
                "资源 {resource_index} 未声明为 exportable(new_with_exportable_textures)"
            ));
        };
        let RtRes::Buf(rb) = &self.frame.rt[resource_index] else {
            return Err(format!(
                "资源 {resource_index} 非 buffer(exportable 簿记不一致)"
            ));
        };
        let size = self
            .frame
            .exportable_buf_sizes
            .iter()
            .find(|&&(r, _)| r as usize == resource_index)
            .map(|&(_, s)| s)
            .ok_or_else(|| format!("资源 {resource_index} 无 buffer 尺寸簿记"))?;
        let memory = rb.mem;
        let mut out = ExportedBufferWin32 {
            handle: 0,
            size,
            allocation_size: alloc_size,
            memory_type_index: mem_type,
        };
        if let Some(&(_, h)) = self.exported_handles.iter().find(|&&(r, _)| r == res) {
            out.handle = h;
            return Ok(out);
        }
        let get_fn = self
            .get_memory_win32
            .ok_or("vkGetMemoryWin32HandleKHR 未解析(非 exportable session)")?;
        let info = MemoryGetWin32HandleInfoKHR {
            s_type: ST_MEMORY_GET_WIN32_HANDLE_INFO_KHR,
            p_next: std::ptr::null(),
            memory,
            handle_type: EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
        };
        let mut handle: *mut c_void = std::ptr::null_mut();
        // SAFETY: 同 texture 导出本体(export chain 分配已在建面保证)。
        let r = unsafe { get_fn(self.device, &info, &mut handle) };
        if r != VK_SUCCESS || handle.is_null() {
            return Err(format!("vkGetMemoryWin32HandleKHR 失败: {r}"));
        }
        self.exported_handles.push((res, handle as usize));
        out.handle = handle as usize;
        Ok(out)
    }
}

/// [`execute_frame`] 内部(模块头 U32 契约本体)。结构:
/// instance/device/queue → 能力核对(sync2 硬依赖)→ 资源建面 → descriptor/pipeline 缓存
/// → 命令录制(上传段 → 逐 pass〔plan 回放 + 隐式补全 + pass 本体〕→ readback 段)→
/// 提交+等待 → map 回读 → messenger fail-closed → Cleanup 逆序销毁。
unsafe fn execute_frame_inner(
    gipa: FnGetInstanceProcAddr,
    resources: &[ResourceDesc],
    passes: &[Pass],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
) -> Result<Vec<Vec<u8>>, String> {
    let (instance, validation, debug_utils) = create_instance(gipa, c"rurix-render-exec")?;
    let vk_destroy_instance: FnDestroyInstance =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;

    // messenger(validation 开时;ERROR 级 → 栈上 AtomicBool → 末尾翻 Err,U27 同律)。
    let validation_error = std::sync::atomic::AtomicBool::new(false);
    let mut messenger: VkDebugUtilsMessengerEXT = VK_NULL_HANDLE;
    let mut destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT> = None;
    if validation {
        destroy_messenger = cast_fn(gipa(instance, c"vkDestroyDebugUtilsMessengerEXT".as_ptr()));
        if let Some(create_messenger) = cast_fn::<FnCreateDebugUtilsMessengerEXT>(gipa(
            instance,
            c"vkCreateDebugUtilsMessengerEXT".as_ptr(),
        )) {
            let mci = DebugUtilsMessengerCreateInfoEXT {
                s_type: ST_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
                p_next: std::ptr::null(),
                flags: 0,
                message_severity: DEBUG_UTILS_SEVERITY_ERROR,
                message_type: DEBUG_UTILS_TYPE_GENERAL
                    | DEBUG_UTILS_TYPE_VALIDATION
                    | DEBUG_UTILS_TYPE_PERFORMANCE,
                pfn_user_callback: debug_messenger_cb,
                p_user_data: (&validation_error as *const std::sync::atomic::AtomicBool)
                    .cast_mut()
                    .cast::<c_void>(),
            };
            let _ = create_messenger(instance, &mci, std::ptr::null(), &mut messenger);
            if messenger != VK_NULL_HANDLE {
                // ERROR 计数自此可信(见 VALIDATION_MESSENGER_INSTALLED 注释)。
                VALIDATION_MESSENGER_INSTALLED.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
    // messenger 拆除宏(U27 同律:每个 early-return 前与末尾正常路径均拆除,先于 instance)。
    macro_rules! destroy_msgr {
        () => {
            if let Some(dm) = destroy_messenger {
                if messenger != VK_NULL_HANDLE {
                    dm(instance, messenger, std::ptr::null());
                }
            }
        };
    }

    let out = (|| {
        let pd = pick_physical_device(gipa, instance)?;
        let caps = read_physical_caps(gipa, instance, pd)?;
        if !caps.synchronization2 {
            return Err(format!(
                "VK_KHR_synchronization2 不可用(device `{}`;render_exec 屏障经 \
                 vkCmdPipelineBarrier2KHR,硬依赖,RXS-0193 口径不降级)",
                caps.device_name
            ));
        }
        // RXS-0303 L3:conservative pass × 无扩展 → 确定性 Err(任何 pipeline 创建前)。
        validate_conservative_raster(passes, &caps)?;
        let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
            instance,
            c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
        ))
        .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
        let vk_get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
            instance,
            c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
        ))
        .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
        let vk_create_device: FnCreateDevice =
            cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
        let vk_get_device_proc: FnGetDeviceProcAddr =
            cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr()))
                .ok_or("缺 vkGetDeviceProcAddr")?;

        // graphics queue family。
        let mut qf_count = 0u32;
        vk_get_qf(pd, &mut qf_count, std::ptr::null_mut());
        let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
            .map(|_| QueueFamilyProperties {
                queue_flags: 0,
                queue_count: 0,
                timestamp_valid_bits: 0,
                min_image_transfer_granularity: VkExtent3D {
                    width: 0,
                    height: 0,
                    depth: 0,
                },
            })
            .collect();
        vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
        let qfi = qfs
            .iter()
            .position(|q| q.queue_flags & QUEUE_GRAPHICS_BIT != 0)
            .ok_or("无 graphics queue family")? as u32;

        // device 创建:sync2(硬)+ atomic int64(机会性)扩展与 feature 链。
        let mut exts: Vec<*const c_char> = vec![c"VK_KHR_synchronization2".as_ptr()];
        let mut int64_feat = PhysicalDeviceShaderAtomicInt64Features {
            s_type: ST_PHYSICAL_DEVICE_SHADER_ATOMIC_INT64_FEATURES,
            p_next: std::ptr::null_mut(),
            shader_buffer_int64_atomics: u32::from(caps.shader_buffer_int64_atomics),
            shader_shared_int64_atomics: 0,
        };
        let mut sync2_feat = PhysicalDeviceSynchronization2Features {
            s_type: ST_PHYSICAL_DEVICE_SYNCHRONIZATION_2_FEATURES,
            p_next: std::ptr::null_mut(),
            synchronization2: 1,
        };
        if caps.shader_buffer_int64_atomics {
            exts.push(c"VK_KHR_shader_atomic_int64".as_ptr());
            sync2_feat.p_next =
                (&mut int64_feat as *mut PhysicalDeviceShaderAtomicInt64Features).cast::<c_void>();
        }
        // 保守光栅(G7.5b,RXS-0303 IR1):探测到即启用该设备扩展(无 feature 结构体,
        // 仅扩展名;不在位时不 push——conservative pass 已在上方 fail-closed 拒)。
        if caps.conservative_raster.is_some() {
            exts.push(c"VK_EXT_conservative_rasterization".as_ptr());
        }
        let prio = [1.0f32];
        let dqci = DeviceQueueCreateInfo {
            s_type: ST_DEVICE_QUEUE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            queue_family_index: qfi,
            queue_count: 1,
            p_queue_priorities: prio.as_ptr(),
        };
        let mut core_features = [0u32; 55];
        core_features[40] = u32::from(caps.shader_int64);
        // fragmentStoresAndAtomics(下标 26,G7.5b):FS 写 SSBO/原子的 core feature,
        // 探测到即启用(HW 光栅 VisBuffer 腿依赖;VUID-RuntimeSpirv-NonWritable-06340)。
        core_features[26] = u32::from(caps.fragment_stores_and_atomics);
        let dci = DeviceCreateInfo {
            s_type: ST_DEVICE_CREATE_INFO,
            p_next: (&sync2_feat as *const PhysicalDeviceSynchronization2Features).cast::<c_void>(),
            flags: 0,
            queue_create_info_count: 1,
            p_queue_create_infos: &dqci,
            enabled_layer_count: 0,
            pp_enabled_layer_names: std::ptr::null(),
            enabled_extension_count: exts.len() as u32,
            pp_enabled_extension_names: exts.as_ptr(),
            p_enabled_features: core_features.as_ptr().cast::<c_void>(),
        };
        let mut device: VkDevice = std::ptr::null_mut();
        if vk_create_device(pd, &dci, std::ptr::null(), &mut device) != VK_SUCCESS {
            return Err("vkCreateDevice 失败".to_owned());
        }

        let result = execute_on_device(
            gipa,
            instance,
            debug_utils,
            vk_get_device_proc,
            device,
            pd,
            vk_get_mem,
            qfi,
            resources,
            passes,
            barriers,
            readbacks,
            None,
            true,
            &[],
            1,
            &[],
            &[],
        );

        let dev_destroy: Option<FnDestroyDevice> =
            cast_fn(vk_get_device_proc(device, c"vkDestroyDevice".as_ptr()));
        if let Some(dd) = dev_destroy {
            dd(device, std::ptr::null());
        }
        result
    })();

    destroy_msgr!();
    vk_destroy_instance(instance, std::ptr::null());
    let out = out?;
    if validation_error.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(
            "VK_LAYER_KHRONOS_validation ERROR 级消息(fail-closed;stderr 见详情)".to_owned(),
        );
    }
    Ok(out)
}

/// device 存活期主体(资源建面 → 缓存 → 录制 → 提交 → 回读;Cleanup 兜底销毁)。
/// `as_handles` = session AS 表各 TLAS 句柄(G7.6 Wave B;空 = 无 AS 绑定面,
/// `Bindings::accel_structs` 非空在校验期已拒,运行期越界同样确定性 `Err`)。
#[allow(clippy::too_many_arguments)]
unsafe fn execute_on_device(
    // G31+ 波 C Task C7:instance 三元(instance/gipa/debug_utils 启用位)——
    // debug label 符号装载事实源(instance 级 vkGetInstanceProcAddr 解析;
    // 未启用 = 双 None 录制零开销跳过,fail-silent)。
    gipa: FnGetInstanceProcAddr,
    instance: VkInstance,
    debug_utils: bool,
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    qfi: u32,
    resources: &[ResourceDesc],
    passes: &[Pass],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
    mut capture: Option<&mut Option<NativeDeviceFrame>>,
    submit_now: bool,
    as_handles: &[u64],
    // timestamp query pool 容量倍数(G14plus FIF:persistent 路 = frame_slots,
    // slot k 用区间 [k*passes*2, (k+1)*passes*2);ephemeral 路恒 1——池对象数与
    // 既有用法 [0, passes*2) 均 0-byte,仅容量加性扩大)。
    query_slots: usize,
    // G14.10b exportable 纹理下标集(已校验;空 = 无导出面,建面/录制全旧行为。
    // device 须已启用 VK_KHR_external_memory(+_win32)——create_persistent_frame
    // 侧 fail-closed 保证;ephemeral 路恒空)。
    exportable: &[u32],
    // G14.11 D3D12 反向导入纹理表(已校验 (资源下标, NT handle);空 = 无导入面,
    // 建面/录制全旧行为;device 扩展保证同上;ephemeral 路恒空)。
    imported_d3d12: &[(u32, usize)],
) -> Result<Vec<Vec<u8>>, String> {
    let mut dev = Dev::load(gdpa, device)?;
    // G31+ 波 C Task C7:debug label 符号对装载(fail-silent;absent = 录制零开销)。
    dev.load_debug_labels(gipa, instance, debug_utils);
    // G14.11:导入 handle 兼容内存类型查询 fn(仅导入面解析;缺符号 = 驱动异常
    // fail-closed——扩展已启用)。
    let get_win32_props: Option<FnGetMemoryWin32HandlePropertiesKHR> = if imported_d3d12.is_empty()
    {
        None
    } else {
        match cast_fn::<FnGetMemoryWin32HandlePropertiesKHR>(gdpa(
            device,
            c"vkGetMemoryWin32HandlePropertiesKHR".as_ptr(),
        )) {
            Some(f) => Some(f),
            None => {
                return Err("缺 vkGetMemoryWin32HandlePropertiesKHR(扩展已启用仍不可解析)".into());
            }
        }
    };
    let mut queue: VkQueue = std::ptr::null_mut();
    (dev.get_device_queue)(device, qfi, 0, &mut queue);
    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    vk_get_mem(pd, &mut memprops);

    let mut cleanup = Cleanup::default();
    let mut retained = false;
    // 主体闭包:任何早退都落到下面统一等待+销毁(无泄漏路径)。
    let result = (|| {
        // ── 资源建面(buffer 上传即写;image staging 就绪,命令流首段 copy)──
        let mut rt: Vec<RtRes> = Vec::with_capacity(resources.len());
        // G14.10b exportable 簿记((资源下标, allocation 字节, memory type);
        // 建面处一次定格,导出 accessor/release barrier 共用。
        let mut exportable_meta: Vec<(u32, u64, u32)> = Vec::new();
        let mut exportable_buf_sizes: Vec<(u32, u64)> = Vec::new();
        // G14.10d DEVICE_LOCAL 初始数据一次性上传表:(staging buffer, 目标 buffer,
        // 字节数)——资源建面期收集,cmdpool 就绪后单次 one-shot copy submit + 有界
        // 等待,staging 随即销毁(稳态零驻留)。
        let mut init_copies: Vec<(VkBuffer, VkDeviceMemory, VkBuffer, u64)> = Vec::new();
        for (i, r) in resources.iter().enumerate() {
            match r {
                ResourceDesc::Buffer(b) => {
                    // G14.11:imported(D3D12_RESOURCE)buffer 独立分支(创建期
                    // 已校验 data=None + device_local + handle 非零):external
                    // chain 建 buffer → handle properties ∩ requirements 选
                    // DEVICE_LOCAL → import + dedicated(buffer) 分配绑定。D3D12
                    // 侧 SHARED committed BUFFER 线性字节,跨 API 布局无歧义
                    // (texture 直共享 tiling 弃案的 fallback 正道)。
                    if let Some(&(_, h)) = imported_d3d12.iter().find(|&&(r, _)| r as usize == i) {
                        let ext_buf_info = ExternalMemoryBufferCreateInfo {
                            s_type: ST_EXTERNAL_MEMORY_BUFFER_CREATE_INFO,
                            p_next: std::ptr::null(),
                            handle_types: EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE,
                        };
                        let bci = BufferCreateInfo {
                            s_type: ST_BUFFER_CREATE_INFO,
                            p_next: (&ext_buf_info as *const ExternalMemoryBufferCreateInfo).cast(),
                            flags: 0,
                            size: b.size,
                            usage: buffer_usage_flags(b.usage),
                            sharing_mode: SHARING_MODE_EXCLUSIVE,
                            queue_family_index_count: 0,
                            p_queue_family_indices: std::ptr::null(),
                        };
                        let mut buf: VkBuffer = VK_NULL_HANDLE;
                        if (dev.create_buffer)(device, &bci, std::ptr::null(), &mut buf)
                            != VK_SUCCESS
                        {
                            return Err(format!(
                                "resources[{i}]: vkCreateBuffer(imported d3d12) 失败"
                            ));
                        }
                        let mut req = std::mem::zeroed::<MemoryRequirements>();
                        (dev.buf_mem_req)(device, buf, &mut req);
                        let mut props = MemoryWin32HandlePropertiesKHR {
                            s_type: ST_MEMORY_WIN32_HANDLE_PROPERTIES_KHR,
                            p_next: std::ptr::null_mut(),
                            memory_type_bits: 0,
                        };
                        let get_props = get_win32_props
                            .expect("imported 非空时 get_win32_props 已解析(前置 fail-closed)");
                        let r = get_props(
                            device,
                            EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE,
                            h as *mut c_void,
                            &mut props,
                        );
                        if r != VK_SUCCESS {
                            (dev.destroy_buffer)(device, buf, std::ptr::null());
                            return Err(format!(
                                "resources[{i}]: vkGetMemoryWin32HandlePropertiesKHR(buffer) 失败: {r}(D3D12 handle 无效或跨 adapter——LUID 对拍先行)"
                            ));
                        }
                        let type_bits = req.memory_type_bits & props.memory_type_bits;
                        let Some(mt) = pick_mem_type(&memprops, type_bits, MEM_DEVICE_LOCAL) else {
                            (dev.destroy_buffer)(device, buf, std::ptr::null());
                            return Err(format!(
                                "resources[{i}]: 无 device-local 内存类型(imported buffer)"
                            ));
                        };
                        let dedicated = MemoryDedicatedAllocateInfo {
                            s_type: ST_MEMORY_DEDICATED_ALLOCATE_INFO,
                            p_next: std::ptr::null(),
                            image: VK_NULL_HANDLE,
                            buffer: buf,
                        };
                        let import_info = ImportMemoryWin32HandleInfoKHR {
                            s_type: ST_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR,
                            p_next: (&dedicated as *const MemoryDedicatedAllocateInfo).cast(),
                            handle_type: EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE,
                            handle: h as *mut c_void,
                            name: std::ptr::null(),
                        };
                        let mai = MemoryAllocateInfo {
                            s_type: ST_MEMORY_ALLOCATE_INFO,
                            p_next: (&import_info as *const ImportMemoryWin32HandleInfoKHR).cast(),
                            allocation_size: req.size,
                            memory_type_index: mt,
                        };
                        let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
                        if (dev.alloc_mem)(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
                            (dev.destroy_buffer)(device, buf, std::ptr::null());
                            return Err(format!(
                                "resources[{i}]: vkAllocateMemory(import d3d12 buffer) 失败"
                            ));
                        }
                        (dev.bind_buf)(device, buf, mem, 0);
                        cleanup.buffers.push((buf, mem));
                        cleanup.register_allocation(
                            mem,
                            req.size,
                            memprops.memory_types[mt as usize].heap_index,
                            Some(StableResourceId(i as u64 + 1)),
                        );
                        rt.push(RtRes::Buf(RtBuffer { buffer: buf, mem }));
                        continue;
                    }
                    // G14.10f:exportable buffer 走 external memory chain 独立
                    // 分支(创建期已校验 data=None + device_local;export +
                    // dedicated(buffer) 分配,DEVICE_LOCAL 首匹配)。
                    if exportable.contains(&(i as u32)) {
                        let ext_buf_info = ExternalMemoryBufferCreateInfo {
                            s_type: ST_EXTERNAL_MEMORY_BUFFER_CREATE_INFO,
                            p_next: std::ptr::null(),
                            handle_types: EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
                        };
                        let bci = BufferCreateInfo {
                            s_type: ST_BUFFER_CREATE_INFO,
                            p_next: (&ext_buf_info as *const ExternalMemoryBufferCreateInfo).cast(),
                            flags: 0,
                            size: b.size,
                            usage: buffer_usage_flags(b.usage),
                            sharing_mode: SHARING_MODE_EXCLUSIVE,
                            queue_family_index_count: 0,
                            p_queue_family_indices: std::ptr::null(),
                        };
                        let mut buf: VkBuffer = VK_NULL_HANDLE;
                        if (dev.create_buffer)(device, &bci, std::ptr::null(), &mut buf)
                            != VK_SUCCESS
                        {
                            return Err(format!("resources[{i}]: vkCreateBuffer(exportable) 失败"));
                        }
                        let mut req = std::mem::zeroed::<MemoryRequirements>();
                        (dev.buf_mem_req)(device, buf, &mut req);
                        let Some(mt) =
                            pick_mem_type(&memprops, req.memory_type_bits, MEM_DEVICE_LOCAL)
                        else {
                            (dev.destroy_buffer)(device, buf, std::ptr::null());
                            return Err(format!(
                                "resources[{i}]: 无 device-local 内存类型(exportable buffer)"
                            ));
                        };
                        let dedicated = MemoryDedicatedAllocateInfo {
                            s_type: ST_MEMORY_DEDICATED_ALLOCATE_INFO,
                            p_next: std::ptr::null(),
                            image: VK_NULL_HANDLE,
                            buffer: buf,
                        };
                        let export_info = ExportMemoryAllocateInfo {
                            s_type: ST_EXPORT_MEMORY_ALLOCATE_INFO,
                            p_next: (&dedicated as *const MemoryDedicatedAllocateInfo).cast(),
                            handle_types: EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
                        };
                        let mai = MemoryAllocateInfo {
                            s_type: ST_MEMORY_ALLOCATE_INFO,
                            p_next: (&export_info as *const ExportMemoryAllocateInfo).cast(),
                            allocation_size: req.size,
                            memory_type_index: mt,
                        };
                        let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
                        if (dev.alloc_mem)(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
                            (dev.destroy_buffer)(device, buf, std::ptr::null());
                            return Err(format!(
                                "resources[{i}]: vkAllocateMemory(exportable buffer) 失败"
                            ));
                        }
                        (dev.bind_buf)(device, buf, mem, 0);
                        cleanup.buffers.push((buf, mem));
                        cleanup.register_allocation(
                            mem,
                            req.size,
                            memprops.memory_types[mt as usize].heap_index,
                            Some(StableResourceId(i as u64 + 1)),
                        );
                        exportable_meta.push((i as u32, req.size, mt));
                        exportable_buf_sizes.push((i as u32, b.size));
                        rt.push(RtRes::Buf(RtBuffer { buffer: buf, mem }));
                        continue;
                    }
                    // G14.10d 三路判定(RFC-0030 §4.3;§4.3 L1 波的实测归因先例:
                    // SSBO 本体切 HOST_CACHED 使 GPU 直写吃 snoop 惩罚 3.5×——
                    // cached 仅用于 staging 类,SSBO 本体 host 路恒 WC):
                    // ① `device_local=true` → DEVICE_LOCAL 独占 VRAM(GPU 全速;
                    //   初始数据经 one-shot staging copy,回读经帧尾 copy 到
                    //   cached staging——见 record_frame_body readback 段);
                    // ② `device_local=false` → 既有首匹配 HV+HC(WC;
                    //   FrameUpdate.buffer_uploads 逐帧覆盖写的小参数 buffer,
                    //   host map 写向最优,HEAD 行为 0-byte)。
                    let mem_class = if b.device_local {
                        BufferMemClass::DeviceLocal
                    } else {
                        BufferMemClass::HostWc
                    };
                    let staged_init = b.device_local && b.data.is_some_and(|d| !d.is_empty());
                    let (buf, mem) = create_device_buffer(
                        &dev,
                        device,
                        &memprops,
                        b.size,
                        buffer_usage_flags(b.usage),
                        if staged_init { None } else { b.data },
                        Some(StableResourceId(i as u64 + 1)),
                        &mut cleanup,
                        mem_class,
                    )?;
                    if staged_init {
                        let d = b.data.unwrap_or(&[]);
                        let (sbuf, smem) = create_device_buffer(
                            &dev,
                            device,
                            &memprops,
                            d.len().max(4) as u64,
                            0x1, // TRANSFER_SRC(copy 源)
                            Some(d),
                            None,
                            &mut cleanup,
                            // host 写向 one-shot staging:WC 型即最优。
                            BufferMemClass::HostWc,
                        )?;
                        init_copies.push((sbuf, smem, buf, d.len() as u64));
                    }
                    rt.push(RtRes::Buf(RtBuffer { buffer: buf, mem }));
                }
                ResourceDesc::Texture(t) => {
                    // G14.10b:exportable 纹理走 external memory chain 新分支
                    // (集外资源 p_next 恒 null——既有路径 0-byte);
                    // G14.11:imported(D3D12_RESOURCE)同链异位——external
                    // chain handleTypes 与分配链 import/export 按集切换。
                    let is_exportable = exportable.contains(&(i as u32));
                    let imported = imported_d3d12
                        .iter()
                        .find(|&&(r, _)| r as usize == i)
                        .map(|&(_, h)| h);
                    let ext_img_info = ExternalMemoryImageCreateInfo {
                        s_type: ST_EXTERNAL_MEMORY_IMAGE_CREATE_INFO,
                        p_next: std::ptr::null(),
                        handle_types: if imported.is_some() {
                            EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE
                        } else {
                            EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32
                        },
                    };
                    let usage_flags = texture_usage_flags(t.usage);
                    let ici = ImageCreateInfo {
                        s_type: ST_IMAGE_CREATE_INFO,
                        p_next: if is_exportable || imported.is_some() {
                            (&ext_img_info as *const ExternalMemoryImageCreateInfo).cast()
                        } else {
                            std::ptr::null()
                        },
                        flags: 0,
                        image_type: IMAGE_TYPE_2D,
                        format: t.format.vk_format(),
                        extent: VkExtent3D {
                            width: t.width,
                            height: t.height,
                            depth: 1,
                        },
                        mip_levels: 1,
                        array_layers: 1,
                        samples: SAMPLE_COUNT_1,
                        tiling: IMAGE_TILING_OPTIMAL,
                        usage: usage_flags,
                        sharing_mode: SHARING_MODE_EXCLUSIVE,
                        queue_family_index_count: 0,
                        p_queue_family_indices: std::ptr::null(),
                        initial_layout: LAYOUT_UNDEFINED,
                    };
                    let mut image: VkImage = VK_NULL_HANDLE;
                    if (dev.create_image)(device, &ici, std::ptr::null(), &mut image) != VK_SUCCESS
                    {
                        return Err(format!("resources[{i}]: vkCreateImage 失败"));
                    }
                    let mut req = std::mem::zeroed::<MemoryRequirements>();
                    (dev.img_mem_req)(device, image, &mut req);
                    // G14.11:导入面内存类型位 = image requirements ∩ handle
                    // properties(vkGetMemoryWin32HandlePropertiesKHR;
                    // D3D12_RESOURCE 可查——OPAQUE 类不可,VUID-00666)。
                    let mut type_bits = req.memory_type_bits;
                    if let (Some(h), Some(get_props)) = (imported, get_win32_props) {
                        let mut props = MemoryWin32HandlePropertiesKHR {
                            s_type: ST_MEMORY_WIN32_HANDLE_PROPERTIES_KHR,
                            p_next: std::ptr::null_mut(),
                            memory_type_bits: 0,
                        };
                        let r = get_props(
                            device,
                            EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE,
                            h as *mut c_void,
                            &mut props,
                        );
                        if r != VK_SUCCESS {
                            (dev.destroy_image)(device, image, std::ptr::null());
                            return Err(format!(
                                "resources[{i}]: vkGetMemoryWin32HandlePropertiesKHR 失败: {r}(D3D12 handle 无效或跨 adapter——LUID 对拍先行)"
                            ));
                        }
                        type_bits &= props.memory_type_bits;
                    }
                    let Some(mt) = pick_mem_type(&memprops, type_bits, MEM_DEVICE_LOCAL) else {
                        (dev.destroy_image)(device, image, std::ptr::null());
                        return Err(format!("resources[{i}]: 无 device-local 内存类型"));
                    };
                    // 分配链:exportable = export(OPAQUE_WIN32)→ dedicated;
                    // imported = import(D3D12_RESOURCE handle)→ dedicated。
                    // NVIDIA 上 Win32 导出/导入 image 实务强制 dedicated,必挂。
                    let dedicated = MemoryDedicatedAllocateInfo {
                        s_type: ST_MEMORY_DEDICATED_ALLOCATE_INFO,
                        p_next: std::ptr::null(),
                        image,
                        buffer: VK_NULL_HANDLE,
                    };
                    let export_info = ExportMemoryAllocateInfo {
                        s_type: ST_EXPORT_MEMORY_ALLOCATE_INFO,
                        p_next: (&dedicated as *const MemoryDedicatedAllocateInfo).cast(),
                        handle_types: EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
                    };
                    let import_info = ImportMemoryWin32HandleInfoKHR {
                        s_type: ST_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR,
                        p_next: (&dedicated as *const MemoryDedicatedAllocateInfo).cast(),
                        handle_type: EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE,
                        handle: imported.unwrap_or(0) as *mut c_void,
                        name: std::ptr::null(),
                    };
                    let mai = MemoryAllocateInfo {
                        s_type: ST_MEMORY_ALLOCATE_INFO,
                        p_next: if imported.is_some() {
                            (&import_info as *const ImportMemoryWin32HandleInfoKHR).cast()
                        } else if is_exportable {
                            (&export_info as *const ExportMemoryAllocateInfo).cast()
                        } else {
                            std::ptr::null()
                        },
                        allocation_size: req.size,
                        memory_type_index: mt,
                    };
                    let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
                    if (dev.alloc_mem)(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
                        (dev.destroy_image)(device, image, std::ptr::null());
                        return Err(format!(
                            "resources[{i}]: vkAllocateMemory 失败(image{})",
                            if imported.is_some() {
                                ",D3D12_RESOURCE 导入——handle/LUID/参数对齐检查"
                            } else {
                                ""
                            }
                        ));
                    }
                    (dev.bind_img)(device, image, mem, 0);
                    cleanup.images.push((image, mem));
                    cleanup.register_allocation(
                        mem,
                        req.size,
                        memprops.memory_types[mt as usize].heap_index,
                        Some(StableResourceId(i as u64 + 1)),
                    );
                    if is_exportable {
                        exportable_meta.push((i as u32, req.size, mt));
                    }
                    let ivci = ImageViewCreateInfo {
                        s_type: ST_IMAGE_VIEW_CREATE_INFO,
                        p_next: std::ptr::null(),
                        flags: 0,
                        image,
                        view_type: IMAGE_VIEW_TYPE_2D,
                        format: t.format.vk_format(),
                        components: VkComponentMapping {
                            r: COMPONENT_SWIZZLE_IDENTITY,
                            g: COMPONENT_SWIZZLE_IDENTITY,
                            b: COMPONENT_SWIZZLE_IDENTITY,
                            a: COMPONENT_SWIZZLE_IDENTITY,
                        },
                        subresource_range: VkImageSubresourceRange {
                            aspect_mask: t.format.aspect_mask(),
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                    };
                    let mut view: VkImageView = VK_NULL_HANDLE;
                    if (dev.create_image_view)(device, &ivci, std::ptr::null(), &mut view)
                        != VK_SUCCESS
                    {
                        return Err(format!("resources[{i}]: vkCreateImageView 失败"));
                    }
                    cleanup.views.push(view);
                    // 初始数据 staging(命令流首段 copy;句柄登记 cleanup 随帧销毁;
                    // usage TRANSFER_SRC——VUID-vkCmdCopyBufferToImage-srcBuffer-00176)。
                    let staging = if let Some(d) = t.data {
                        let sz = (d.len().max(4)) as u64;
                        let (sbuf, _smem) = create_device_buffer(
                            &dev,
                            device,
                            &memprops,
                            sz,
                            0x1, // TRANSFER_SRC
                            Some(d),
                            None,
                            &mut cleanup,
                            BufferMemClass::HostWc, // host 写向 staging:WC 型最优
                        )?;
                        Some(sbuf)
                    } else {
                        None
                    };
                    rt.push(RtRes::Img(RtImage {
                        image,
                        mem,
                        view,
                        width: t.width,
                        height: t.height,
                        format: t.format,
                        usage_flags,
                        staging,
                    }));
                }
            }
        }

        // inline 顶点 buffer(随 pass 声明建临时资源;命令流外 host 直写)。
        let mut inline_vbs: Vec<Option<VkBuffer>> = Vec::with_capacity(passes.len());
        for p in passes {
            match p {
                Pass::Raster(rp) => match &rp.vertex {
                    VertexData::Inline { data, .. } => {
                        let (vbuf, _vmem) = create_device_buffer(
                            &dev,
                            device,
                            &memprops,
                            (data.len().max(4)) as u64,
                            0x80, // VERTEX_BUFFER(host 直写上传,无 transfer 命令)
                            Some(data),
                            None,
                            &mut cleanup,
                            BufferMemClass::HostWc, // host 写向:WC 型最优
                        )?;
                        inline_vbs.push(Some(vbuf));
                    }
                    _ => inline_vbs.push(None),
                },
                Pass::Compute(_) => inline_vbs.push(None),
            }
        }

        // 状态跟踪初值由 record_frame_body 内部按同一规则重建(创建录制与
        // FrameUpdate 重录共用同一事实源,禁第二份)。

        // ── 内建线性 sampler(任一 pass 有 sampled 绑定时建)──
        let need_sampler = passes.iter().any(|p| match p {
            Pass::Raster(rp) => !rp.bindings.sampled_images.is_empty(),
            Pass::Compute(cp) => !cp.bindings.sampled_images.is_empty(),
        });
        let sampler = if need_sampler {
            let sci = SamplerCreateInfo {
                s_type: ST_SAMPLER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                mag_filter: FILTER_LINEAR,
                min_filter: FILTER_LINEAR,
                mipmap_mode: SAMPLER_MIPMAP_MODE_NEAREST,
                address_mode_u: SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                address_mode_v: SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                address_mode_w: SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                mip_lod_bias: 0.0,
                anisotropy_enable: 0,
                max_anisotropy: 1.0,
                compare_enable: 0,
                compare_op: 0,
                min_lod: 0.0,
                max_lod: 1.0,
                border_color: 0,
                unnormalized_coordinates: 0,
            };
            let mut s: VkSampler = VK_NULL_HANDLE;
            if (dev.create_sampler)(device, &sci, std::ptr::null(), &mut s) != VK_SUCCESS {
                return Err("vkCreateSampler 失败".to_owned());
            }
            cleanup.sampler = s;
            s
        } else {
            VK_NULL_HANDLE
        };

        // ── 缓存:dsl / pipeline layout / render pass / shader module / pipeline ──
        let mut dsl_cache: HashMap<SetLayoutKey, VkDescriptorSetLayout> = HashMap::new();
        let mut pl_cache: HashMap<PipelineLayoutKey, VkPipelineLayout> = HashMap::new();
        let mut rp_cache: HashMap<RenderPassKey, VkRenderPass> = HashMap::new();
        let mut shader_cache: HashMap<u64, VkShaderModule> = HashMap::new();
        let mut gfx_pipe_cache: HashMap<RasterPipelineKey, VkPipeline> = HashMap::new();
        let mut cmp_pipe_cache: HashMap<ComputePipelineKey, VkPipeline> = HashMap::new();

        let shader_module = |spv: &[u8],
                             dev: &Dev,
                             cache: &mut HashMap<u64, VkShaderModule>,
                             cleanup: &mut Cleanup|
         -> Result<VkShaderModule, String> {
            let words = spirv_to_words(spv, "shader module")?;
            let h = fnv1a_u64(spv);
            if let Some(&m) = cache.get(&h) {
                return Ok(m);
            }
            let smci = ShaderModuleCreateInfo {
                s_type: ST_SHADER_MODULE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                code_size: spv.len(),
                p_code: words.as_ptr(),
            };
            let mut m: VkShaderModule = VK_NULL_HANDLE;
            if (dev.create_shader)(device, &smci, std::ptr::null(), &mut m) != VK_SUCCESS {
                return Err("vkCreateShaderModule 失败".to_owned());
            }
            cache.insert(h, m);
            cleanup.shader_modules.push(m);
            Ok(m)
        };

        // dsl 获取/创建(set0 固定约定;空布局同样合法,统一缓存)。
        let get_dsl = |key: SetLayoutKey,
                       dev: &Dev,
                       cache: &mut HashMap<SetLayoutKey, VkDescriptorSetLayout>,
                       cleanup: &mut Cleanup|
         -> Result<VkDescriptorSetLayout, String> {
            if let Some(&d) = cache.get(&key) {
                return Ok(d);
            }
            let plan = plan_set0_layout_with_as(key.0, key.1, key.2, key.3, key.4);
            let bindings: Vec<DescriptorSetLayoutBinding> = plan
                .iter()
                .map(|&(binding, ty)| DescriptorSetLayoutBinding {
                    binding,
                    descriptor_type: ty,
                    descriptor_count: 1,
                    stage_flags: SHADER_STAGE_RFX,
                    p_immutable_samplers: std::ptr::null(),
                })
                .collect();
            let dci = DescriptorSetLayoutCreateInfo {
                s_type: ST_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                binding_count: bindings.len() as u32,
                p_bindings: if bindings.is_empty() {
                    std::ptr::null()
                } else {
                    bindings.as_ptr()
                },
            };
            let mut d: VkDescriptorSetLayout = VK_NULL_HANDLE;
            if (dev.create_dsl)(device, &dci, std::ptr::null(), &mut d) != VK_SUCCESS {
                return Err("vkCreateDescriptorSetLayout 失败".to_owned());
            }
            cache.insert(key, d);
            cleanup.dsls.push(d);
            Ok(d)
        };

        // pipeline layout 获取/创建(set0 + 单 push range(pc_size>0 时))。
        let get_pl = |key: PipelineLayoutKey,
                      dev: &Dev,
                      dsl_cache: &mut HashMap<SetLayoutKey, VkDescriptorSetLayout>,
                      pl_cache: &mut HashMap<PipelineLayoutKey, VkPipelineLayout>,
                      cleanup: &mut Cleanup|
         -> Result<VkPipelineLayout, String> {
            if let Some(&p) = pl_cache.get(&key) {
                return Ok(p);
            }
            let dsl = get_dsl(key.set, dev, dsl_cache, cleanup)?;
            let pc_range = PushConstantRange {
                stage_flags: SHADER_STAGE_RFX,
                offset: 0,
                size: key.pc_size,
            };
            let plci = PipelineLayoutCreateInfo {
                s_type: ST_PIPELINE_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                set_layout_count: 1,
                p_set_layouts: &dsl,
                push_constant_range_count: u32::from(key.pc_size > 0),
                p_push_constant_ranges: if key.pc_size > 0 {
                    &pc_range
                } else {
                    std::ptr::null()
                },
            };
            let mut p: VkPipelineLayout = VK_NULL_HANDLE;
            if (dev.create_pl)(device, &plci, std::ptr::null(), &mut p) != VK_SUCCESS {
                return Err("vkCreatePipelineLayout 失败".to_owned());
            }
            pl_cache.insert(key, p);
            cleanup.pipe_layouts.push(p);
            Ok(p)
        };

        // render pass 获取/创建(initial=final=attachment 态,迁移全在 render pass 外
        // 经 barrier2;subpass 无依赖——单 queue 全序 + 显式 barrier 封口)。
        let get_rp = |key: &RenderPassKey,
                      dev: &Dev,
                      rp_cache: &mut HashMap<RenderPassKey, VkRenderPass>,
                      cleanup: &mut Cleanup|
         -> Result<VkRenderPass, String> {
            if let Some(&r) = rp_cache.get(key) {
                return Ok(r);
            }
            let mut attachments: Vec<AttachmentDescription> = Vec::new();
            let mut color_refs: Vec<AttachmentReference> = Vec::new();
            for (i, &fmt) in key.color_formats.iter().enumerate() {
                let clear = key.color_clears.get(i).copied().unwrap_or(false);
                attachments.push(AttachmentDescription {
                    flags: 0,
                    format: fmt,
                    samples: SAMPLE_COUNT_1,
                    load_op: if clear {
                        ATTACHMENT_LOAD_OP_CLEAR
                    } else {
                        ATTACHMENT_LOAD_OP_LOAD
                    },
                    store_op: ATTACHMENT_STORE_OP_STORE,
                    stencil_load_op: 0,
                    stencil_store_op: 0,
                    initial_layout: LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                    final_layout: LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                });
                color_refs.push(AttachmentReference {
                    attachment: i as u32,
                    layout: LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                });
            }
            let depth_ref;
            let p_depth: *const AttachmentReference = if key.depth_format != 0 {
                attachments.push(AttachmentDescription {
                    flags: 0,
                    format: key.depth_format,
                    samples: SAMPLE_COUNT_1,
                    load_op: if key.depth_clear {
                        ATTACHMENT_LOAD_OP_CLEAR
                    } else {
                        ATTACHMENT_LOAD_OP_LOAD
                    },
                    store_op: ATTACHMENT_STORE_OP_STORE,
                    stencil_load_op: 0,
                    stencil_store_op: 0,
                    initial_layout: LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    final_layout: LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                });
                depth_ref = AttachmentReference {
                    attachment: (attachments.len() - 1) as u32,
                    layout: LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                };
                &depth_ref
            } else {
                std::ptr::null()
            };
            let subpass = SubpassDescription {
                flags: 0,
                pipeline_bind_point: PIPELINE_BIND_POINT_GRAPHICS,
                input_attachment_count: 0,
                p_input_attachments: std::ptr::null(),
                color_attachment_count: color_refs.len() as u32,
                p_color_attachments: if color_refs.is_empty() {
                    std::ptr::null()
                } else {
                    color_refs.as_ptr()
                },
                p_resolve_attachments: std::ptr::null(),
                p_depth_stencil_attachment: p_depth,
                preserve_attachment_count: 0,
                p_preserve_attachments: std::ptr::null(),
            };
            let rpci = RenderPassCreateInfo {
                s_type: ST_RENDER_PASS_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                attachment_count: attachments.len() as u32,
                p_attachments: attachments.as_ptr(),
                subpass_count: 1,
                p_subpasses: &subpass,
                dependency_count: 0,
                p_dependencies: std::ptr::null(),
            };
            let mut r: VkRenderPass = VK_NULL_HANDLE;
            if (dev.create_rp)(device, &rpci, std::ptr::null(), &mut r) != VK_SUCCESS {
                return Err("vkCreateRenderPass 失败".to_owned());
            }
            rp_cache.insert(key.clone(), r);
            cleanup.render_passes.push(r);
            Ok(r)
        };

        // ── descriptor pool(容量 = 全 pass 绑定合计)──
        let mut total_as = 0u32;
        let mut total_sb = 0u32;
        let mut total_si = 0u32;
        let mut total_simg = 0u32;
        let mut total_ub = 0u32;
        for p in passes {
            let b = match p {
                Pass::Raster(rp) => &rp.bindings,
                Pass::Compute(cp) => &cp.bindings,
            };
            total_as += b.accel_structs.len() as u32;
            total_sb += b.storage_buffers.len() as u32;
            total_si += b.sampled_images.len() as u32;
            total_simg += b.storage_images.len() as u32;
            total_ub += u32::from(b.uniform.is_some());
        }
        let mut pool_sizes: Vec<DescriptorPoolSize> = Vec::new();
        if total_as > 0 {
            pool_sizes.push(DescriptorPoolSize {
                descriptor_type: DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR,
                descriptor_count: total_as,
            });
        }
        if total_sb > 0 {
            pool_sizes.push(DescriptorPoolSize {
                descriptor_type: DESCRIPTOR_TYPE_STORAGE_BUFFER,
                descriptor_count: total_sb,
            });
        }
        if total_si > 0 {
            pool_sizes.push(DescriptorPoolSize {
                descriptor_type: DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                descriptor_count: total_si,
            });
        }
        if total_simg > 0 {
            pool_sizes.push(DescriptorPoolSize {
                descriptor_type: DESCRIPTOR_TYPE_STORAGE_IMAGE,
                descriptor_count: total_simg,
            });
        }
        if total_ub > 0 {
            pool_sizes.push(DescriptorPoolSize {
                descriptor_type: DESCRIPTOR_TYPE_UNIFORM_BUFFER,
                descriptor_count: total_ub,
            });
        }
        let need_pool = !pool_sizes.is_empty();
        let pool = if need_pool {
            let dpci = DescriptorPoolCreateInfo {
                s_type: ST_DESCRIPTOR_POOL_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                max_sets: passes.len() as u32,
                pool_size_count: pool_sizes.len() as u32,
                p_pool_sizes: pool_sizes.as_ptr(),
            };
            let mut p: VkDescriptorPool = VK_NULL_HANDLE;
            if (dev.create_dp)(device, &dpci, std::ptr::null(), &mut p) != VK_SUCCESS {
                return Err("vkCreateDescriptorPool 失败".to_owned());
            }
            cleanup.pool = p;
            p
        } else {
            VK_NULL_HANDLE
        };

        // ── per-pass 装配(ds 分配/写入 + pipeline + framebuffer),录制前一次性完成 ──
        let mut setups: Vec<PassSetup> = Vec::with_capacity(passes.len());
        for (pi, p) in passes.iter().enumerate() {
            let b = match p {
                Pass::Raster(rp) => &rp.bindings,
                Pass::Compute(cp) => &cp.bindings,
            };
            let set_key: SetLayoutKey = (
                b.accel_structs.len() as u32,
                b.storage_buffers.len() as u32,
                b.sampled_images.len() as u32,
                b.storage_images.len() as u32,
                b.uniform.is_some(),
            );
            let pc_size = b.push_constants.len() as u32;
            let pl_key = PipelineLayoutKey {
                set: set_key,
                pc_size,
            };
            let pl = get_pl(pl_key, &dev, &mut dsl_cache, &mut pl_cache, &mut cleanup)?;
            let dsl = *dsl_cache
                .get(&set_key)
                .ok_or_else(|| format!("pass {pi}: set0 layout 未入缓存"))?;

            // descriptor set 分配 + 写入(全空绑定跳过分配,录制时不绑)。
            let has_any_binding =
                set_key.0 > 0 || set_key.1 > 0 || set_key.2 > 0 || set_key.3 > 0 || set_key.4;
            let set = if has_any_binding {
                let dsai = DescriptorSetAllocateInfo {
                    s_type: ST_DESCRIPTOR_SET_ALLOCATE_INFO,
                    p_next: std::ptr::null(),
                    descriptor_pool: pool,
                    descriptor_set_count: 1,
                    p_set_layouts: &dsl,
                };
                let mut s: VkDescriptorSet = VK_NULL_HANDLE;
                if (dev.alloc_ds)(device, &dsai, &mut s) != VK_SUCCESS {
                    return Err(format!("pass {pi}: vkAllocateDescriptorSets 失败"));
                }
                write_pass_descriptor_set(&dev, device, s, b, &rt, sampler, as_handles, pi)?;
                Some(s)
            } else {
                None
            };

            // pipeline(+raster 的 rp/fb/clear)。
            match p {
                Pass::Raster(rp) => {
                    let vs_mod = shader_module(rp.vs_spirv, &dev, &mut shader_cache, &mut cleanup)?;
                    let fs_mod = shader_module(rp.fs_spirv, &dev, &mut shader_cache, &mut cleanup)?;
                    let extent = {
                        let Some(first) = rp
                            .colors
                            .first()
                            .map(|c| c.res)
                            .or_else(|| rp.depth.map(|d| d.res))
                        else {
                            return Err(format!("pass {pi}: 无 attachment(校验漏网)"));
                        };
                        let Some(ri) = rt[first as usize].image() else {
                            return Err(format!("pass {pi}: attachment 非 texture"));
                        };
                        (ri.width, ri.height)
                    };
                    let rp_key = RenderPassKey {
                        color_formats: rp
                            .colors
                            .iter()
                            .map(|c| {
                                rt[c.res as usize]
                                    .image()
                                    .map(|i| i.format.vk_format())
                                    .unwrap_or(0)
                            })
                            .collect(),
                        depth_format: rp
                            .depth
                            .map(|d| {
                                rt[d.res as usize]
                                    .image()
                                    .map(|i| i.format.vk_format())
                                    .unwrap_or(0)
                            })
                            .unwrap_or(0),
                        color_clears: rp.colors.iter().map(|c| c.clear.is_some()).collect(),
                        depth_clear: rp.depth.map(|d| d.clear.is_some()).unwrap_or(false),
                    };
                    let render_pass = get_rp(&rp_key, &dev, &mut rp_cache, &mut cleanup)?;

                    // framebuffer(attachments = color 列 + depth 尾)。
                    let mut views: Vec<VkImageView> = Vec::new();
                    for c in &rp.colors {
                        let Some(ri) = rt[c.res as usize].image() else {
                            return Err(format!(
                                "pass {pi}: color attachment {} 非 texture",
                                c.res
                            ));
                        };
                        views.push(ri.view);
                    }
                    if let Some(d) = rp.depth {
                        let Some(ri) = rt[d.res as usize].image() else {
                            return Err(format!(
                                "pass {pi}: depth attachment {} 非 texture",
                                d.res
                            ));
                        };
                        views.push(ri.view);
                    }
                    let fbci = FramebufferCreateInfo {
                        s_type: ST_FRAMEBUFFER_CREATE_INFO,
                        p_next: std::ptr::null(),
                        flags: 0,
                        render_pass,
                        attachment_count: views.len() as u32,
                        p_attachments: views.as_ptr(),
                        width: extent.0,
                        height: extent.1,
                        layers: 1,
                    };
                    let mut fb: VkFramebuffer = VK_NULL_HANDLE;
                    if (dev.create_fb)(device, &fbci, std::ptr::null(), &mut fb) != VK_SUCCESS {
                        return Err(format!("pass {pi}: vkCreateFramebuffer 失败"));
                    }
                    cleanup.framebuffers.push(fb);

                    // clear 值列(color 列 + depth 尾;depth 用 color[0] 槽)。
                    let mut clears: Vec<ClearValue> = rp
                        .colors
                        .iter()
                        .map(|c| ClearValue {
                            color: c.clear.unwrap_or([0.0, 0.0, 0.0, 0.0]),
                        })
                        .collect();
                    if let Some(d) = rp.depth {
                        clears.push(ClearValue {
                            color: [d.clear.unwrap_or(0.0), 0.0, 0.0, 0.0],
                        });
                    }

                    // graphics pipeline(cache 键:SPIR-V+格式+顶点布局;render pass 兼容
                    // 只取决于格式,与 loadOp 无关)。
                    let (vstride, vattrs, has_vb): (u32, Vec<(u32, u32, u32)>, bool) =
                        match &rp.vertex {
                            VertexData::Inline { stride, attrs, .. }
                            | VertexData::Resource { stride, attrs, .. } => {
                                (*stride, attrs.to_vec(), true)
                            }
                            VertexData::Pull => (0, Vec::new(), false),
                        };
                    let pipe_key = RasterPipelineKey {
                        vs_hash: fnv1a_u64(rp.vs_spirv),
                        fs_hash: fnv1a_u64(rp.fs_spirv),
                        color_formats: rp_key.color_formats.clone(),
                        depth_format: rp_key.depth_format,
                        vertex_stride: vstride,
                        attrs: vattrs.clone(),
                        has_vb,
                        conservative: rp.conservative.map(|c| c.extra_overestimation.to_bits()),
                    };
                    let pipe = if let Some(&cached) = gfx_pipe_cache.get(&pipe_key) {
                        cached
                    } else {
                        let stages = [
                            PipelineShaderStageCreateInfo {
                                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                                p_next: std::ptr::null(),
                                flags: 0,
                                stage: SHADER_STAGE_VERTEX,
                                module: vs_mod,
                                p_name: c"main".as_ptr(),
                                p_specialization_info: std::ptr::null(),
                            },
                            PipelineShaderStageCreateInfo {
                                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                                p_next: std::ptr::null(),
                                flags: 0,
                                stage: SHADER_STAGE_FRAGMENT,
                                module: fs_mod,
                                p_name: c"main".as_ptr(),
                                p_specialization_info: std::ptr::null(),
                            },
                        ];
                        let vbind = VkVertexInputBindingDescription {
                            binding: 0,
                            stride: vstride,
                            input_rate: VERTEX_INPUT_RATE_VERTEX,
                        };
                        let vattr_descs: Vec<VkVertexInputAttributeDescription> = vattrs
                            .iter()
                            .map(
                                |&(location, format, offset)| VkVertexInputAttributeDescription {
                                    location,
                                    binding: 0,
                                    format,
                                    offset,
                                },
                            )
                            .collect();
                        let vinput = PipelineVertexInputStateCreateInfo {
                            s_type: ST_PIPELINE_VERTEX_INPUT_STATE_CI,
                            p_next: std::ptr::null(),
                            flags: 0,
                            vertex_binding_description_count: u32::from(has_vb),
                            p_vertex_binding_descriptions: if has_vb {
                                &vbind
                            } else {
                                std::ptr::null()
                            },
                            vertex_attribute_description_count: vattr_descs.len() as u32,
                            p_vertex_attribute_descriptions: if vattr_descs.is_empty() {
                                std::ptr::null()
                            } else {
                                vattr_descs.as_ptr()
                            },
                        };
                        let ia = PipelineInputAssemblyStateCreateInfo {
                            s_type: ST_PIPELINE_INPUT_ASSEMBLY_STATE_CI,
                            p_next: std::ptr::null(),
                            flags: 0,
                            topology: PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
                            primitive_restart_enable: 0,
                        };
                        // 固定管线视口/scissor(非 dynamic state):视口取 rp.viewport 或
                        // attachment 尺寸,scissor 恒全幅。
                        let vp = VkViewport {
                            x: 0.0,
                            y: 0.0,
                            width: rp.viewport.map(|v| v.0 as f32).unwrap_or(extent.0 as f32),
                            height: rp.viewport.map(|v| v.1 as f32).unwrap_or(extent.1 as f32),
                            min_depth: 0.0,
                            max_depth: 1.0,
                        };
                        let scissor = VkRect2D {
                            offset: VkOffset2D { x: 0, y: 0 },
                            extent: VkExtent2D {
                                width: extent.0,
                                height: extent.1,
                            },
                        };
                        let viewport = PipelineViewportStateCreateInfo {
                            s_type: ST_PIPELINE_VIEWPORT_STATE_CI,
                            p_next: std::ptr::null(),
                            flags: 0,
                            viewport_count: 1,
                            p_viewports: &vp,
                            scissor_count: 1,
                            p_scissors: &scissor,
                        };
                        // 保守光栅 pNext 链节点(G7.5b,RXS-0303 L2:mode 恒 OVERESTIMATE)。
                        // `conservative=None` 时不建节点、p_next 维持 null(既有行为 0-byte)。
                        // 节点为本作用域栈上值,存活跨越下方 vkCreateGraphicsPipelines 调用。
                        let conservative_ci = rp.conservative.map(|c| {
                            PipelineRasterizationConservativeStateCreateInfo {
                                s_type: ST_PIPELINE_RASTERIZATION_CONSERVATIVE_STATE_CI_EXT,
                                p_next: std::ptr::null(),
                                flags: 0,
                                conservative_rasterization_mode:
                                    CONSERVATIVE_RASTERIZATION_MODE_OVERESTIMATE,
                                extra_primitive_overestimation_size: c.extra_overestimation,
                            }
                        });
                        let raster = PipelineRasterizationStateCreateInfo {
                            s_type: ST_PIPELINE_RASTERIZATION_STATE_CI,
                            p_next: conservative_ci.as_ref().map_or(std::ptr::null(), |c| {
                                (c as *const PipelineRasterizationConservativeStateCreateInfo)
                                    .cast::<c_void>()
                            }),
                            flags: 0,
                            depth_clamp_enable: 0,
                            rasterizer_discard_enable: 0,
                            polygon_mode: POLYGON_MODE_FILL,
                            cull_mode: CULL_MODE_NONE,
                            front_face: FRONT_FACE_COUNTER_CLOCKWISE,
                            depth_bias_enable: 0,
                            depth_bias_constant_factor: 0.0,
                            depth_bias_clamp: 0.0,
                            depth_bias_slope_factor: 0.0,
                            line_width: 1.0,
                        };
                        let msaa = PipelineMultisampleStateCreateInfo {
                            s_type: ST_PIPELINE_MULTISAMPLE_STATE_CI,
                            p_next: std::ptr::null(),
                            flags: 0,
                            rasterization_samples: SAMPLE_COUNT_1,
                            sample_shading_enable: 0,
                            min_sample_shading: 0.0,
                            p_sample_mask: std::ptr::null(),
                            alpha_to_coverage_enable: 0,
                            alpha_to_one_enable: 0,
                        };
                        let blend_atts: Vec<PipelineColorBlendAttachmentState> = rp
                            .colors
                            .iter()
                            .map(|_| PipelineColorBlendAttachmentState {
                                blend_enable: 0,
                                src_color_blend_factor: 0,
                                dst_color_blend_factor: 0,
                                color_blend_op: 0,
                                src_alpha_blend_factor: 0,
                                dst_alpha_blend_factor: 0,
                                alpha_blend_op: 0,
                                color_write_mask: COLOR_COMPONENT_RGBA,
                            })
                            .collect();
                        let blend = PipelineColorBlendStateCreateInfo {
                            s_type: ST_PIPELINE_COLOR_BLEND_STATE_CI,
                            p_next: std::ptr::null(),
                            flags: 0,
                            logic_op_enable: 0,
                            logic_op: 0,
                            attachment_count: blend_atts.len() as u32,
                            p_attachments: if blend_atts.is_empty() {
                                std::ptr::null()
                            } else {
                                blend_atts.as_ptr()
                            },
                            blend_constants: [0.0; 4],
                        };
                        let depth_state = PipelineDepthStencilStateCreateInfo {
                            s_type: ST_PIPELINE_DEPTH_STENCIL_STATE_CI,
                            p_next: std::ptr::null(),
                            flags: 0,
                            depth_test_enable: u32::from(rp.depth.is_some()),
                            depth_write_enable: u32::from(rp.depth.is_some()),
                            depth_compare_op: COMPARE_OP_LESS_OR_EQUAL,
                            depth_bounds_test_enable: 0,
                            stencil_test_enable: 0,
                            front: [0; 7],
                            back: [0; 7],
                            min_depth_bounds: 0.0,
                            max_depth_bounds: 1.0,
                        };
                        let gpci = GraphicsPipelineCreateInfo {
                            s_type: ST_GRAPHICS_PIPELINE_CREATE_INFO,
                            p_next: std::ptr::null(),
                            flags: 0,
                            stage_count: 2,
                            p_stages: stages.as_ptr(),
                            p_vertex_input_state: &vinput,
                            p_input_assembly_state: &ia,
                            p_tessellation_state: std::ptr::null(),
                            p_viewport_state: &viewport,
                            p_rasterization_state: &raster,
                            p_multisample_state: &msaa,
                            p_depth_stencil_state: &depth_state,
                            p_color_blend_state: &blend,
                            p_dynamic_state: std::ptr::null(),
                            layout: pl,
                            render_pass,
                            subpass: 0,
                            base_pipeline_handle: VK_NULL_HANDLE,
                            base_pipeline_index: -1,
                        };
                        let mut gp: VkPipeline = VK_NULL_HANDLE;
                        if (dev.create_gp)(
                            device,
                            VK_NULL_HANDLE,
                            1,
                            &gpci,
                            std::ptr::null(),
                            &mut gp,
                        ) != VK_SUCCESS
                        {
                            return Err(format!("pass {pi}: vkCreateGraphicsPipelines 失败"));
                        }
                        gfx_pipe_cache.insert(pipe_key, gp);
                        cleanup.pipelines.push(gp);
                        gp
                    };
                    setups.push(PassSetup {
                        set,
                        dsl,
                        pl,
                        pc_size,
                        pipe,
                        rp: render_pass,
                        fb,
                        extent,
                        clears,
                    });
                }
                Pass::Compute(cp) => {
                    let module = shader_module(cp.spirv, &dev, &mut shader_cache, &mut cleanup)?;
                    let words = spirv_to_words(cp.spirv, &format!("pass {pi} cs"))?;
                    let entry = match cp.entry {
                        Some(e) => e.to_owned(),
                        None => crate::vk::entry_point_name(&words)
                            .ok_or_else(|| format!("pass {pi}: SPIR-V 无 OpEntryPoint"))?,
                    };
                    let entry_c = std::ffi::CString::new(entry.clone())
                        .map_err(|_| format!("pass {pi}: entry 名含内嵌 NUL"))?;
                    let pipe_key = ComputePipelineKey {
                        spv_hash: fnv1a_u64(cp.spirv),
                        entry: entry.into_bytes(),
                    };
                    let pipe = if let Some(&cached) = cmp_pipe_cache.get(&pipe_key) {
                        cached
                    } else {
                        let stage = PipelineShaderStageCreateInfo {
                            s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                            p_next: std::ptr::null(),
                            flags: 0,
                            stage: SHADER_STAGE_COMPUTE,
                            module,
                            p_name: entry_c.as_ptr(),
                            p_specialization_info: std::ptr::null(),
                        };
                        let cpci = ComputePipelineCreateInfo {
                            s_type: ST_COMPUTE_PIPELINE_CREATE_INFO,
                            p_next: std::ptr::null(),
                            flags: 0,
                            stage,
                            layout: pl,
                            base_pipeline_handle: VK_NULL_HANDLE,
                            base_pipeline_index: -1,
                        };
                        let mut gp: VkPipeline = VK_NULL_HANDLE;
                        if (dev.create_cp)(
                            device,
                            VK_NULL_HANDLE,
                            1,
                            &cpci,
                            std::ptr::null(),
                            &mut gp,
                        ) != VK_SUCCESS
                        {
                            return Err(format!("pass {pi}: vkCreateComputePipelines 失败"));
                        }
                        cmp_pipe_cache.insert(pipe_key, gp);
                        cleanup.pipelines.push(gp);
                        gp
                    };
                    setups.push(PassSetup {
                        set,
                        dsl,
                        pl,
                        pc_size,
                        pipe,
                        rp: VK_NULL_HANDLE,
                        fb: VK_NULL_HANDLE,
                        extent: (0, 0),
                        clears: Vec::new(),
                    });
                }
            }
        }

        // ── timestamp query pool + 命令池 + 主命令缓冲 ──
        // G38 T3:池尾追加 BRIDGE_QUERY_COUNT 个桥接时戳 query(既有逐 pass/
        // FIF slot 区间下标不动;追加区不开启桥接计时时从不 reset/写/读)。
        let qpci = QueryPoolCreateInfo {
            s_type: ST_QUERY_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            query_type: QUERY_TYPE_TIMESTAMP,
            query_count: (passes.len() as u32) * 2 * (query_slots.max(1) as u32)
                + BRIDGE_QUERY_COUNT,
            pipeline_statistics: 0,
        };
        let mut query_pool = VK_NULL_HANDLE;
        if (dev.create_query_pool)(device, &qpci, std::ptr::null(), &mut query_pool) != VK_SUCCESS {
            return Err("vkCreateQueryPool(timestamp) 失败".to_owned());
        }
        cleanup.query_pool = query_pool;
        let cpci = CommandPoolCreateInfo {
            s_type: ST_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            // persistent capture 路允许逐 cmd reset(FrameUpdate 每帧重录
            // `vkResetCommandBuffer` VUID-00046);ephemeral 路恒 flags=0(0-byte)。
            flags: if submit_now {
                0
            } else {
                CMD_POOL_CREATE_RESET_COMMAND_BUFFER_BIT
            },
            queue_family_index: qfi,
        };
        let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;
        if (dev.create_cmdpool)(device, &cpci, std::ptr::null(), &mut cmdpool) != VK_SUCCESS {
            return Err("vkCreateCommandPool 失败".to_owned());
        }
        cleanup.cmdpool = cmdpool;

        // ── G14.10d DEVICE_LOCAL 初始数据 one-shot 上传:专用一次性 cmd + 单 fence
        // 有界等待(AS 初始 build 同型纪律;创建期一次,正常帧循环仍禁
        // vkQueueWaitIdle)。fence 等待后 GPU copy 已完成且写已可用(fence signal
        // 含全域 availability),后续帧提交经 host 序 happens-after——免跨提交
        // barrier。staging 随即销毁 + cleanup/ledger 同步摘除(稳态零驻留)──
        // G14.12：exportable/imported 跨界 image 的 layout 常驻 GENERAL——建面期
        // 一次性 UNDEFINED→GENERAL 迁移(与初始数据上传共用同一 one-shot cmd +
        // fence 有界等待),此后帧内初值/收敛态恒 GENERAL,免掉每帧一次全表面
        // 压缩元数据重初始化(实测 1920×1080 三标 ≈0.3ms/帧)。
        let layout_init_imgs: Vec<(VkImage, u32)> = exportable
            .iter()
            .copied()
            .chain(imported_d3d12.iter().map(|&(r, _)| r))
            .filter_map(|res| match &rt[res as usize] {
                RtRes::Img(ri) => Some((ri.image, ri.format.aspect_mask())),
                RtRes::Buf(_) => None,
            })
            .collect();
        if !init_copies.is_empty() || !layout_init_imgs.is_empty() {
            const INIT_UPLOAD_WAIT_NS: u64 = 5_000_000_000;
            let cbai = CommandBufferAllocateInfo {
                s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                command_pool: cmdpool,
                level: CMD_BUFFER_LEVEL_PRIMARY,
                command_buffer_count: 1,
            };
            let mut up_cmd: VkCommandBuffer = std::ptr::null_mut();
            if (dev.alloc_cmd)(device, &cbai, &mut up_cmd) != VK_SUCCESS {
                return Err("初始数据上传: vkAllocateCommandBuffers 失败".into());
            }
            let fci = FenceCreateInfo {
                s_type: ST_FENCE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
            };
            let mut fence = VK_NULL_HANDLE;
            if (dev.create_fence)(device, &fci, std::ptr::null(), &mut fence) != VK_SUCCESS {
                (dev.free_cmd)(device, cmdpool, 1, &up_cmd);
                return Err("初始数据上传: vkCreateFence 失败".into());
            }
            let one_shot: Result<(), String> = (|| {
                let cbi = CommandBufferBeginInfo {
                    s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
                    p_next: std::ptr::null(),
                    flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
                    p_inheritance_info: std::ptr::null(),
                };
                if (dev.begin_cmd)(up_cmd, &cbi) != VK_SUCCESS {
                    return Err("初始数据上传: vkBeginCommandBuffer 失败".into());
                }
                for &(sbuf, _, dst, len) in &init_copies {
                    let region = VkBufferCopy {
                        src_offset: 0,
                        dst_offset: 0,
                        size: len,
                    };
                    (dev.cmd_copy_buf)(up_cmd, sbuf, dst, 1, &region);
                }
                if !layout_init_imgs.is_empty() {
                    let bs: Vec<ImageMemoryBarrier2> = layout_init_imgs
                        .iter()
                        .map(|&(image, aspect_mask)| ImageMemoryBarrier2 {
                            s_type: ST_IMAGE_MEMORY_BARRIER_2,
                            p_next: std::ptr::null(),
                            src_stage_mask: STAGE2_ALL_COMMANDS,
                            src_access_mask: 0,
                            dst_stage_mask: STAGE2_ALL_COMMANDS,
                            dst_access_mask: ACCESS2_MEMORY_WRITE,
                            old_layout: LAYOUT_UNDEFINED,
                            new_layout: LAYOUT_GENERAL,
                            src_queue_family_index: QUEUE_FAMILY_IGNORED,
                            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                            image,
                            subresource_range: VkImageSubresourceRange {
                                aspect_mask,
                                base_mip_level: 0,
                                level_count: 1,
                                base_array_layer: 0,
                                layer_count: 1,
                            },
                        })
                        .collect();
                    let di = DependencyInfo {
                        s_type: ST_DEPENDENCY_INFO,
                        p_next: std::ptr::null(),
                        dependency_flags: 0,
                        memory_barrier_count: 0,
                        p_memory_barriers: std::ptr::null(),
                        buffer_memory_barrier_count: 0,
                        p_buffer_memory_barriers: std::ptr::null(),
                        image_memory_barrier_count: bs.len() as u32,
                        p_image_memory_barriers: bs.as_ptr(),
                    };
                    (dev.cmd_barrier2)(up_cmd, &di);
                }
                if (dev.end_cmd)(up_cmd) != VK_SUCCESS {
                    return Err("初始数据上传: vkEndCommandBuffer 失败".into());
                }
                let si = SubmitInfo {
                    s_type: ST_SUBMIT_INFO,
                    p_next: std::ptr::null(),
                    wait_semaphore_count: 0,
                    p_wait_semaphores: std::ptr::null(),
                    p_wait_dst_stage_mask: std::ptr::null(),
                    command_buffer_count: 1,
                    p_command_buffers: &up_cmd,
                    signal_semaphore_count: 0,
                    p_signal_semaphores: std::ptr::null(),
                };
                if (dev.queue_submit)(queue, 1, &si, fence) != VK_SUCCESS {
                    return Err("初始数据上传: vkQueueSubmit 失败".into());
                }
                let done = (dev.wait_fences)(device, 1, &fence, 1, INIT_UPLOAD_WAIT_NS);
                if done == VK_TIMEOUT {
                    return Err(format!(
                        "初始数据上传 fence 有界等待超时({INIT_UPLOAD_WAIT_NS}ns;TDR-suspected)"
                    ));
                }
                if done != VK_SUCCESS {
                    return Err(queue_result_error("初始数据上传 vkWaitForFences", done));
                }
                Ok(())
            })();
            (dev.destroy_fence)(device, fence, std::ptr::null());
            (dev.free_cmd)(device, cmdpool, 1, &up_cmd);
            one_shot?;
            // staging 销毁 + cleanup/ledger 摘除(ensure_upload_staging 同型纪律:
            // 不摘则 Drop 期双重销毁/ledger 假账)。
            for &(sbuf, smem, _, _) in &init_copies {
                (dev.destroy_buffer)(device, sbuf, std::ptr::null());
                (dev.free_mem)(device, smem, std::ptr::null());
                cleanup.buffers.retain(|&(b, _)| b != sbuf);
                cleanup.allocations.retain(|a| a.memory != smem);
            }
            init_copies.clear();
        }

        let cbai = CommandBufferAllocateInfo {
            s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: cmdpool,
            level: CMD_BUFFER_LEVEL_PRIMARY,
            command_buffer_count: 1,
        };
        let mut cmd: VkCommandBuffer = std::ptr::null_mut();
        if (dev.alloc_cmd)(device, &cbai, &mut cmd) != VK_SUCCESS {
            return Err("vkAllocateCommandBuffers 失败".to_owned());
        }
        let cbi = CommandBufferBeginInfo {
            s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: if submit_now {
                CMD_BUFFER_USAGE_ONE_TIME_SUBMIT
            } else {
                0
            },
            p_inheritance_info: std::ptr::null(),
        };
        if (dev.begin_cmd)(cmd, &cbi) != VK_SUCCESS {
            return Err("vkBeginCommandBuffer 失败".to_owned());
        }
        (dev.cmd_reset_query_pool)(cmd, query_pool, 0, (passes.len() as u32) * 2);

        // ── 录制帧命令体(上传段 → 逐 pass → readback 段;record_frame_body 单一
        // 事实源,FrameUpdate 重录路共用;创建期 effective bindings = 声明绑定)──
        // G14.11:帧末 EXTERNAL release 集 = exportable ∪ imported(D3D12 消费
        // 窗同律;重录路从 NativeDeviceFrame 两簿记同式合成)。
        let imported_indices: Vec<u32> = imported_d3d12.iter().map(|&(r, _)| r).collect();
        let release_set: Vec<u32> = exportable
            .iter()
            .copied()
            .chain(imported_indices.iter().copied())
            .collect();
        let declared_bindings: Vec<Bindings> =
            passes.iter().map(|p| pass_bindings(p).clone()).collect();
        let mut rb_buffers: Vec<Option<(VkBuffer, VkDeviceMemory)>> =
            Vec::with_capacity(readbacks.len());
        record_frame_body(
            &FrameBodyParams {
                dev: &dev,
                device,
                memprops: &memprops,
                cmd,
                resources,
                rt: &rt,
                passes,
                barriers,
                effective_bindings: &declared_bindings,
                setups: &setups,
                query_pool,
                query_base: 0,
                inline_vbs: &inline_vbs,
                readbacks,
                record_upload_segment: true,
                exportable: &release_set,
                queue_family_index: qfi,
                slot_set_overrides: None,
            },
            &mut rb_buffers,
            Some(&mut cleanup),
            None,
        )?;

        // ── 提交 + 等待 + map 回读 ──
        if (dev.end_cmd)(cmd) != VK_SUCCESS {
            return Err("vkEndCommandBuffer 失败".to_owned());
        }
        if !submit_now {
            let mut resource_allocations = Vec::with_capacity(rt.len());
            for (i, resource) in rt.iter().enumerate() {
                let memory = match resource {
                    RtRes::Buf(b) => b.mem,
                    RtRes::Img(image) => image.mem,
                };
                let allocation_id = cleanup
                    .allocations
                    .iter()
                    .find(|a| a.memory == memory)
                    .map(|a| a.entry.allocation_id)
                    .ok_or_else(|| format!("resources[{i}] allocation 未入 ledger"))?;
                resource_allocations.push(allocation_id);
            }
            let slot = capture.as_deref_mut().ok_or("persistent capture 槽缺失")?;
            *slot = Some(NativeDeviceFrame {
                dev,
                queue,
                cleanup: std::mem::take(&mut cleanup),
                rt,
                rb_buffers,
                cmd,
                resource_allocations,
                setups,
                inline_vbs,
                exportable_meta,
                exportable_buf_sizes,
                imported_indices,
                queue_family_index: qfi,
                // 创建期录制(含初始上传段)：首次 FrameUpdate 必重录后才可跳过。
                recorded_rb_sources: None,
            });
            retained = true;
            return Ok(Vec::new());
        }
        let si = SubmitInfo {
            s_type: ST_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &cmd,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        if (dev.queue_submit)(queue, 1, &si, 0) != VK_SUCCESS {
            return Err("vkQueueSubmit 失败".to_owned());
        }
        if (dev.queue_wait)(queue) != VK_SUCCESS {
            return Err("vkQueueWaitIdle 失败".to_owned());
        }
        let mut out: Vec<Vec<u8>> = Vec::with_capacity(readbacks.len());
        for (i, rb) in readbacks.iter().enumerate() {
            match *rb {
                Readback::Buffer { res, offset, size } => {
                    let RtRes::Buf(rbuf) = &rt[res as usize] else {
                        return Err(format!("readbacks[{i}]: 资源号 {res} 非 buffer"));
                    };
                    // G14.10d:DEVICE_LOCAL 源不可 map——读帧尾 copy 产物 cached
                    // staging(rb_buffers 有效项即判据,offset 已在 copy 侧消化);
                    // host 路直接 map 资源本体(免 flush;queueWaitIdle 后无在途写)。
                    let (mem, map_offset) = match rb_buffers[i] {
                        Some((_, rmem)) => (rmem, 0),
                        None => (rbuf.mem, offset),
                    };
                    let mut ptr: *mut c_void = std::ptr::null_mut();
                    if (dev.map_mem)(device, mem, map_offset, size, 0, &mut ptr) != VK_SUCCESS {
                        return Err(format!("readbacks[{i}]: vkMapMemory 失败"));
                    }
                    let mut v = vec![0u8; size as usize];
                    std::ptr::copy_nonoverlapping(ptr.cast::<u8>(), v.as_mut_ptr(), size as usize);
                    (dev.unmap_mem)(device, mem);
                    out.push(v);
                }
                Readback::Texture { res } => {
                    let Some(ri) = rt[res as usize].image() else {
                        return Err(format!("readbacks[{i}]: 资源号 {res} 非 texture"));
                    };
                    let sz = ((ri.width as u64)
                        * (ri.height as u64)
                        * (ri.format.bytes_per_texel() as u64))
                        .max(4);
                    let Some((_rbuf, rmem)) = rb_buffers[i] else {
                        return Err(format!("readbacks[{i}]: texture readback buffer 未建"));
                    };
                    let mut ptr: *mut c_void = std::ptr::null_mut();
                    if (dev.map_mem)(device, rmem, 0, sz, 0, &mut ptr) != VK_SUCCESS {
                        return Err(format!("readbacks[{i}]: vkMapMemory 失败(texture)"));
                    }
                    let mut v = vec![0u8; sz as usize];
                    std::ptr::copy_nonoverlapping(ptr.cast::<u8>(), v.as_mut_ptr(), sz as usize);
                    (dev.unmap_mem)(device, rmem);
                    out.push(v);
                }
            }
        }
        Ok(out)
    })();

    // 统一收尾:ephemeral 路 queue 空闲后 Cleanup 逆序销毁；persistent capture 将
    // 所有权移交 session，正常帧循环不走 queueWaitIdle。
    if !retained {
        let _ = (dev.queue_wait)(queue);
        cleanup.destroy_all(&dev, device);
    }
    result
}

#[allow(clippy::too_many_arguments)]
unsafe fn create_persistent_frame(
    gipa: FnGetInstanceProcAddr,
    resources: &[ResourceDesc<'_>],
    passes: &[Pass<'_>],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
    frame_slots: usize,
    accel_structs: &[AccelStructDesc<'_>],
    exportable: &[u32],
    imported_d3d12: &[(u32, usize)],
) -> Result<NativePersistentFrame, String> {
    let (instance, validation, debug_utils) = create_instance(gipa, c"rurix-persistent-frame")?;
    // soak 取证关闭 validation layer(开销/误报);健康靠 fence/telemetry/device-lost。
    // `RURIX_SOAK=1` 显式放行 REQUIRE_REAL 而无 validation(设计案 §5)。
    let soak = std::env::var("RURIX_SOAK").as_deref() == Ok("1");
    if std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1") && !validation && !soak {
        let destroy_instance: FnDestroyInstance =
            cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
        destroy_instance(instance, std::ptr::null());
        return Err(
            "RURIX_REQUIRE_REAL=1 要求 RURIX_VK_VALIDATION=1（ERROR count 不可 unavailable）"
                .into(),
        );
    }
    let destroy_instance: FnDestroyInstance =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
    let validation_errors = Box::new(std::sync::atomic::AtomicU64::new(0));
    let mut messenger = VK_NULL_HANDLE;
    let mut destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT> = None;
    if validation {
        destroy_messenger = cast_fn(gipa(instance, c"vkDestroyDebugUtilsMessengerEXT".as_ptr()));
        if let Some(create_messenger) = cast_fn::<FnCreateDebugUtilsMessengerEXT>(gipa(
            instance,
            c"vkCreateDebugUtilsMessengerEXT".as_ptr(),
        )) {
            let mci = DebugUtilsMessengerCreateInfoEXT {
                s_type: ST_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
                p_next: std::ptr::null(),
                flags: 0,
                message_severity: DEBUG_UTILS_SEVERITY_ERROR,
                message_type: DEBUG_UTILS_TYPE_GENERAL
                    | DEBUG_UTILS_TYPE_VALIDATION
                    | DEBUG_UTILS_TYPE_PERFORMANCE,
                pfn_user_callback: persistent_debug_messenger_cb,
                p_user_data: (&*validation_errors as *const std::sync::atomic::AtomicU64)
                    .cast_mut()
                    .cast::<c_void>(),
            };
            let r = create_messenger(instance, &mci, std::ptr::null(), &mut messenger);
            if r != VK_SUCCESS {
                destroy_instance(instance, std::ptr::null());
                return Err(format!("vkCreateDebugUtilsMessengerEXT 失败: {r}"));
            }
            VALIDATION_MESSENGER_INSTALLED.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
    let destroy_msgr = |messenger: VkDebugUtilsMessengerEXT| {
        if messenger != VK_NULL_HANDLE
            && let Some(destroy) = destroy_messenger
        {
            destroy(instance, messenger, std::ptr::null());
        }
    };

    let result = (|| {
        let pd = pick_physical_device(gipa, instance)?;
        let caps = read_physical_caps(gipa, instance, pd)?;
        if !caps.synchronization2 {
            return Err(format!(
                "VK_KHR_synchronization2 不可用(device `{}`)",
                caps.device_name
            ));
        }
        if !caps.memory_budget {
            return Err("VK_EXT_memory_budget 不可用(mandatory real telemetry fail-closed)".into());
        }
        if !caps.timestamp_period_ns.is_finite() || caps.timestamp_period_ns <= 0.0 {
            return Err(format!(
                "timestampPeriod={} 非有效驱动实值(mandatory real telemetry fail-closed)",
                caps.timestamp_period_ns
            ));
        }
        // RXS-0303 L3:conservative pass × 无扩展 → 确定性 Err(任何 pipeline 创建前)。
        validate_conservative_raster(passes, &caps)?;

        // ── G14.10b exportable / G14.11 imported 能力面(两集皆空 = 全跳过,
        // 0-byte)──
        // ① VK_KHR_external_memory_win32 设备扩展探测(external_memory 本体
        //   1.1 core 免探;导出/导入同一扩展)+ shaderStorageImageExtendedFormats
        //   feature 探测;
        // ② 物理设备 LUID 实采(1.1 core vkGetPhysicalDeviceProperties2 链
        //   VkPhysicalDeviceIDProperties;LUID 无效 → None,消费侧对拍自决)。
        let external_any = !exportable.is_empty() || !imported_d3d12.is_empty();
        let mut storage_ext_formats = false;
        if external_any {
            let vk_enum_ext: FnEnumerateDeviceExtensionProperties = cast_fn(gipa(
                instance,
                c"vkEnumerateDeviceExtensionProperties".as_ptr(),
            ))
            .ok_or("缺 vkEnumerateDeviceExtensionProperties")?;
            let mut ext_count = 0u32;
            vk_enum_ext(pd, std::ptr::null(), &mut ext_count, std::ptr::null_mut());
            let mut exts_list = vec![
                ExtensionProperties {
                    extension_name: [0; 256],
                    spec_version: 0,
                };
                ext_count as usize
            ];
            vk_enum_ext(pd, std::ptr::null(), &mut ext_count, exts_list.as_mut_ptr());
            let has_win32_ext = exts_list.iter().any(|e| {
                // SAFETY: 驱动写入的 extensionName 为 256 字节定长槽内 NUL 结尾 C 串。
                let bytes = unsafe { CStr::from_ptr(e.extension_name.as_ptr()) }.to_bytes();
                bytes == c"VK_KHR_external_memory_win32".to_bytes()
            });
            if !has_win32_ext {
                return Err(format!(
                    "VK_KHR_external_memory_win32 不可用(device `{}`;exportable/\
                     imported 纹理面硬依赖,fail-closed 不降级)",
                    caps.device_name
                ));
            }
            let vk_get_features2: FnGetPhysicalDeviceFeatures2 =
                cast_fn(gipa(instance, c"vkGetPhysicalDeviceFeatures2".as_ptr()))
                    .ok_or("缺 vkGetPhysicalDeviceFeatures2(须 Vulkan 1.1)")?;
            let mut feat2 = PhysicalDeviceFeatures2 {
                s_type: ST_PHYSICAL_DEVICE_FEATURES_2,
                p_next: std::ptr::null_mut(),
                features: [0; 55],
            };
            vk_get_features2(pd, &mut feat2);
            // VkPhysicalDeviceFeatures 字段序第 29 位 = shaderStorageImageExtendedFormats
            // (SDK 1.3.296 `vulkan_core.h` 字段序核对;RG32F 等扩展 storage 格式门)。
            storage_ext_formats = feat2.features[29] != 0;
        }
        let device_luid: Option<[u8; 8]> = {
            let vk_get_props2: Option<FnGetPhysicalDeviceProperties2> =
                cast_fn(gipa(instance, c"vkGetPhysicalDeviceProperties2".as_ptr()));
            vk_get_props2.and_then(|get2| {
                let mut id_props = PhysicalDeviceIDProperties {
                    s_type: ST_PHYSICAL_DEVICE_ID_PROPERTIES,
                    p_next: std::ptr::null_mut(),
                    device_uuid: [0; 16],
                    driver_uuid: [0; 16],
                    device_luid: [0; 8],
                    device_node_mask: 0,
                    device_luid_valid: 0,
                };
                let mut props2 = PhysicalDeviceProperties2Chain {
                    s_type: ST_PHYSICAL_DEVICE_PROPERTIES_2,
                    p_next: (&mut id_props as *mut PhysicalDeviceIDProperties).cast::<c_void>(),
                    properties: PropertiesBlob { bytes: [0; 2048] },
                };
                get2(pd, &mut props2);
                (id_props.device_luid_valid != 0).then_some(id_props.device_luid)
            })
        };

        let get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
            instance,
            c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
        ))
        .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
        let get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
            instance,
            c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
        ))
        .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
        let get_mem2: FnGetPhysicalDeviceMemoryProperties2 = cast_fn(gipa(
            instance,
            c"vkGetPhysicalDeviceMemoryProperties2".as_ptr(),
        ))
        .ok_or("缺 vkGetPhysicalDeviceMemoryProperties2")?;
        let create_device: FnCreateDevice =
            cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
        let gdpa: FnGetDeviceProcAddr = cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr()))
            .ok_or("缺 vkGetDeviceProcAddr")?;

        let mut qf_count = 0;
        get_qf(pd, &mut qf_count, std::ptr::null_mut());
        let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
            .map(|_| QueueFamilyProperties {
                queue_flags: 0,
                queue_count: 0,
                timestamp_valid_bits: 0,
                min_image_transfer_granularity: VkExtent3D {
                    width: 0,
                    height: 0,
                    depth: 0,
                },
            })
            .collect();
        get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
        let qfi = qfs
            .iter()
            .position(|q| q.queue_flags & QUEUE_GRAPHICS_BIT != 0)
            .ok_or("无 graphics queue family")? as u32;
        if qfs[qfi as usize].timestamp_valid_bits == 0 {
            return Err(
                "graphics queue timestampValidBits=0(mandatory GPU timestamp 不可用)".into(),
            );
        }

        // ray query 能力链 fail-closed(G7.6 Wave B D1.3;session 声明 AS 时缺一
        // 确定性 Err,不隐式降级)。
        let as_count = accel_structs.len();
        if as_count > 0 {
            let mut missing: Vec<&str> = Vec::new();
            if !caps.ray_query {
                missing.push("ray_query");
            }
            if !caps.acceleration_structure {
                missing.push("acceleration_structure");
            }
            if !caps.buffer_device_address {
                missing.push("buffer_device_address");
            }
            if !caps.deferred_host_operations {
                missing.push("deferred_host_operations");
            }
            if !missing.is_empty() {
                return Err(format!(
                    "session AS 表需要 ray query 能力链,device `{}` 缺: {}(fail-closed,不降级)",
                    caps.device_name,
                    missing.join(", ")
                ));
            }
        }

        let mut int64_feat = PhysicalDeviceShaderAtomicInt64Features {
            s_type: ST_PHYSICAL_DEVICE_SHADER_ATOMIC_INT64_FEATURES,
            p_next: std::ptr::null_mut(),
            shader_buffer_int64_atomics: u32::from(caps.shader_buffer_int64_atomics),
            shader_shared_int64_atomics: 0,
        };
        let mut sync2_feat = PhysicalDeviceSynchronization2Features {
            s_type: ST_PHYSICAL_DEVICE_SYNCHRONIZATION_2_FEATURES,
            p_next: std::ptr::null_mut(),
            synchronization2: 1,
        };
        let mut as_feat = PhysicalDeviceAccelerationStructureFeatures {
            s_type: ST_PHYSICAL_DEVICE_ACCELERATION_STRUCTURE_FEATURES_KHR,
            p_next: std::ptr::null_mut(),
            acceleration_structure: u32::from(as_count > 0),
            acceleration_structure_capture_replay: 0,
            acceleration_structure_indirect_build: 0,
            acceleration_structure_host_commands: 0,
            descriptor_binding_acceleration_structure_update_after_bind: 0,
        };
        let mut rq_feat = PhysicalDeviceRayQueryFeatures {
            s_type: ST_PHYSICAL_DEVICE_RAY_QUERY_FEATURES_KHR,
            p_next: std::ptr::null_mut(),
            ray_query: u32::from(as_count > 0),
        };
        let mut bda_feat = PhysicalDeviceBufferDeviceAddressFeatures {
            s_type: ST_PHYSICAL_DEVICE_BUFFER_DEVICE_ADDRESS_FEATURES,
            p_next: std::ptr::null_mut(),
            buffer_device_address: u32::from(as_count > 0),
            buffer_device_address_capture_replay: 0,
            buffer_device_address_multi_device: 0,
        };
        let mut exts = vec![
            c"VK_KHR_synchronization2".as_ptr(),
            c"VK_EXT_memory_budget".as_ptr(),
        ];
        if caps.shader_buffer_int64_atomics {
            exts.push(c"VK_KHR_shader_atomic_int64".as_ptr());
            sync2_feat.p_next =
                (&mut int64_feat as *mut PhysicalDeviceShaderAtomicInt64Features).cast();
        }
        // 保守光栅(G7.5b,RXS-0303 IR1):探测到即启用(无 feature 结构体,仅扩展名)。
        if caps.conservative_raster.is_some() {
            exts.push(c"VK_EXT_conservative_rasterization".as_ptr());
        }
        if as_count > 0 {
            // ray query 四扩展 + feature 链(as → rq → bda → sync2 → int64;
            // 写值先于取址,沿 vk.rs run_rq_inner「重挂 pNext 链」先例)。
            exts.push(c"VK_KHR_ray_query".as_ptr());
            exts.push(c"VK_KHR_acceleration_structure".as_ptr());
            exts.push(c"VK_KHR_buffer_device_address".as_ptr());
            exts.push(c"VK_KHR_deferred_host_operations".as_ptr());
            bda_feat.p_next =
                (&mut sync2_feat as *mut PhysicalDeviceSynchronization2Features).cast();
            rq_feat.p_next =
                (&mut bda_feat as *mut PhysicalDeviceBufferDeviceAddressFeatures).cast();
            as_feat.p_next = (&mut rq_feat as *mut PhysicalDeviceRayQueryFeatures).cast();
        }
        // G14.10b/G14.11:external 面两扩展(win32 在位已核;external_memory
        // 1.1 core 收编但显式列出——两名皆注册扩展名,驱动恒接受;导出/导入
        // 同一对扩展;两集皆空不列,0-byte)。
        if external_any {
            exts.push(c"VK_KHR_external_memory".as_ptr());
            exts.push(c"VK_KHR_external_memory_win32".as_ptr());
        }
        let priority = [1.0f32];
        let dqci = DeviceQueueCreateInfo {
            s_type: ST_DEVICE_QUEUE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            queue_family_index: qfi,
            queue_count: 1,
            p_queue_priorities: priority.as_ptr(),
        };
        // session 内存属性快照(NativePersistentFrame.memprops;重录入参)。
        let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
        get_mem(pd, &mut memprops);
        let mut core_features = [0u32; 55];
        core_features[40] = u32::from(caps.shader_int64);
        // fragmentStoresAndAtomics 机会性启用(同 execute_frame 路;字段序注同上)。
        core_features[26] = u32::from(caps.fragment_stores_and_atomics);
        // G14.10b:shaderStorageImageExtendedFormats(仅 exportable session 且
        // 设备支持时启用——RG32F storage 直写面;集空恒 0,既有 feature 位 0-byte)。
        core_features[29] = u32::from(storage_ext_formats);
        let dci = DeviceCreateInfo {
            s_type: ST_DEVICE_CREATE_INFO,
            p_next: if as_count > 0 {
                (&as_feat as *const PhysicalDeviceAccelerationStructureFeatures).cast()
            } else {
                (&sync2_feat as *const PhysicalDeviceSynchronization2Features).cast()
            },
            flags: 0,
            queue_create_info_count: 1,
            p_queue_create_infos: &dqci,
            enabled_layer_count: 0,
            pp_enabled_layer_names: std::ptr::null(),
            enabled_extension_count: exts.len() as u32,
            pp_enabled_extension_names: exts.as_ptr(),
            p_enabled_features: core_features.as_ptr().cast(),
        };
        let mut device: VkDevice = std::ptr::null_mut();
        let create_result = create_device(pd, &dci, std::ptr::null(), &mut device);
        if create_result != VK_SUCCESS {
            return Err(format!("vkCreateDevice(persistent) 失败: {create_result}"));
        }
        let Some(destroy_device) =
            cast_fn::<FnDestroyDevice>(gdpa(device, c"vkDestroyDevice".as_ptr()))
        else {
            return Err("缺 vkDestroyDevice".into());
        };

        // ── session AS 表建面(G7.6 Wave B;vk.rs **同一单所有者** `VkAsManager`,
        // 不建第二套 BVH;初始 build 在下文创建期一次性提交)──
        let mut as_state: Option<PersistentAsState> = None;
        let mut as_handles: Vec<u64> = Vec::with_capacity(as_count);
        if as_count > 0 {
            let fns = match VkAsFns::load(gdpa, device) {
                Ok(f) => f,
                Err(error) => {
                    destroy_device(device, std::ptr::null());
                    return Err(error);
                }
            };
            let vk_memprops = memprops_to_vk(&memprops);
            let mut managers: Vec<VkAsManager> = Vec::with_capacity(as_count);
            for (ai, desc) in accel_structs.iter().enumerate() {
                // G31+ 波 B Task B5:updatable_blas 打标透传(create_scene_ex;
                // 空表与既有 create_scene 面逐字同——静态锚零漂移)。
                match VkAsManager::create_scene_ex(
                    &fns,
                    device,
                    &vk_memprops,
                    &desc.scene,
                    desc.transforms,
                    desc.updatable_blas,
                ) {
                    Ok(mgr) => managers.push(mgr),
                    Err(error) => {
                        for mut mgr in managers.drain(..) {
                            mgr.destroy(&fns, device);
                        }
                        destroy_device(device, std::ptr::null());
                        return Err(format!("accel_structs[{ai}]: {error}"));
                    }
                }
            }
            as_handles = managers.iter().map(|mgr| mgr.tlas()).collect();
            as_state = Some(PersistentAsState { fns, managers });
        }
        // AS 面错误收尾宏:managers 先毁(经 as fns),再毁 device。
        macro_rules! bail_as {
            ($error:expr) => {
                if let Some(state) = as_state.as_mut() {
                    for mgr in &mut state.managers {
                        mgr.destroy(&state.fns, device);
                    }
                }
                destroy_device(device, std::ptr::null());
                return Err($error);
            };
        }

        // G14.10b:vkGetMemoryWin32HandleKHR 解析(仅 exportable session;扩展
        // 已启用,缺符号 = 驱动异常,fail-closed;bail_as! 走 AS 面收尾序)。
        let get_memory_win32: Option<FnGetMemoryWin32HandleKHR> = if exportable.is_empty() {
            None
        } else {
            match cast_fn::<FnGetMemoryWin32HandleKHR>(gdpa(
                device,
                c"vkGetMemoryWin32HandleKHR".as_ptr(),
            )) {
                Some(f) => Some(f),
                None => {
                    bail_as!("缺 vkGetMemoryWin32HandleKHR(扩展已启用仍不可解析)".into());
                }
            }
        };

        let mut captured = None;
        let prepare = execute_on_device(
            gipa,
            instance,
            debug_utils,
            gdpa,
            device,
            pd,
            get_mem,
            qfi,
            resources,
            passes,
            barriers,
            readbacks,
            Some(&mut captured),
            false,
            &as_handles,
            frame_slots,
            exportable,
            imported_d3d12,
        );
        if let Err(error) = prepare {
            bail_as!(error);
        }
        let Some(mut frame) = captured else {
            bail_as!("persistent native frame capture 未产状态".into());
        };

        // ── AS 初始 build:创建期专用一次性 cmd(同一 cmdpool;**非持久 cmd**——持久
        // cmd 每帧重放/重录,不重复 build)+ consume barrier(COMPUTE_SHADER),单
        // fence 有界等待(创建期纪律,正常帧循环仍禁 vkQueueWaitIdle)──
        if let Some(state) = as_state.as_mut() {
            const AS_BUILD_WAIT_NS: u64 = 5_000_000_000;
            let build_result: Result<(), String> = (|| {
                let cbai = CommandBufferAllocateInfo {
                    s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
                    p_next: std::ptr::null(),
                    command_pool: frame.cleanup.cmdpool,
                    level: CMD_BUFFER_LEVEL_PRIMARY,
                    command_buffer_count: 1,
                };
                let mut build_cmd: VkCommandBuffer = std::ptr::null_mut();
                if (frame.dev.alloc_cmd)(device, &cbai, &mut build_cmd) != VK_SUCCESS {
                    return Err("AS 初始 build: vkAllocateCommandBuffers 失败".into());
                }
                let fci = FenceCreateInfo {
                    s_type: ST_FENCE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                };
                let mut fence = VK_NULL_HANDLE;
                if (frame.dev.create_fence)(device, &fci, std::ptr::null(), &mut fence)
                    != VK_SUCCESS
                {
                    (frame.dev.free_cmd)(device, frame.cleanup.cmdpool, 1, &build_cmd);
                    return Err("AS 初始 build: vkCreateFence 失败".into());
                }
                let cbi = CommandBufferBeginInfo {
                    s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
                    p_next: std::ptr::null(),
                    flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
                    p_inheritance_info: std::ptr::null(),
                };
                if (frame.dev.begin_cmd)(build_cmd, &cbi) != VK_SUCCESS {
                    (frame.dev.destroy_fence)(device, fence, std::ptr::null());
                    (frame.dev.free_cmd)(device, frame.cleanup.cmdpool, 1, &build_cmd);
                    return Err("AS 初始 build: vkBeginCommandBuffer 失败".into());
                }
                for mgr in &mut state.managers {
                    mgr.record_build(&state.fns, build_cmd);
                    mgr.record_consume_barrier(
                        &state.fns,
                        build_cmd,
                        PIPELINE_STAGE_COMPUTE_SHADER,
                    );
                }
                if (frame.dev.end_cmd)(build_cmd) != VK_SUCCESS {
                    (frame.dev.destroy_fence)(device, fence, std::ptr::null());
                    (frame.dev.free_cmd)(device, frame.cleanup.cmdpool, 1, &build_cmd);
                    return Err("AS 初始 build: vkEndCommandBuffer 失败".into());
                }
                let si = SubmitInfo {
                    s_type: ST_SUBMIT_INFO,
                    p_next: std::ptr::null(),
                    wait_semaphore_count: 0,
                    p_wait_semaphores: std::ptr::null(),
                    p_wait_dst_stage_mask: std::ptr::null(),
                    command_buffer_count: 1,
                    p_command_buffers: &build_cmd,
                    signal_semaphore_count: 0,
                    p_signal_semaphores: std::ptr::null(),
                };
                if (frame.dev.queue_submit)(frame.queue, 1, &si, fence) != VK_SUCCESS {
                    (frame.dev.destroy_fence)(device, fence, std::ptr::null());
                    (frame.dev.free_cmd)(device, frame.cleanup.cmdpool, 1, &build_cmd);
                    return Err("AS 初始 build: vkQueueSubmit 失败".into());
                }
                let done = (frame.dev.wait_fences)(device, 1, &fence, 1, AS_BUILD_WAIT_NS);
                (frame.dev.destroy_fence)(device, fence, std::ptr::null());
                (frame.dev.free_cmd)(device, frame.cleanup.cmdpool, 1, &build_cmd);
                if done == VK_TIMEOUT {
                    return Err(format!(
                        "AS 初始 build fence 有界等待超时({AS_BUILD_WAIT_NS}ns;TDR-suspected)"
                    ));
                }
                if done != VK_SUCCESS {
                    return Err(queue_result_error("AS 初始 build vkWaitForFences", done));
                }
                Ok(())
            })();
            if let Err(error) = build_result {
                bail_as!(error);
            }
        }

        // ── AS 分配登记 session allocation ledger(resource_id = AS 表项
        // StableResourceId = resources.len() + ai + 1;provenance 代表 allocation =
        // TLAS storage,接扩展表尾)──
        let mut resource_allocations = frame.resource_allocations.clone();
        if let Some(state) = as_state.as_ref() {
            for (ai, mgr) in state.managers.iter().enumerate() {
                let as_resource = StableResourceId(resources.len() as u64 + ai as u64 + 1);
                let tlas_index = mgr.tlas_storage_allocation_index();
                let mut representative = None;
                for (i, &(mem, bytes, heap)) in mgr.allocation_handles().iter().enumerate() {
                    let allocation_id =
                        frame
                            .cleanup
                            .register_allocation(mem, bytes, heap, Some(as_resource));
                    if i == tlas_index {
                        representative = Some(allocation_id);
                    }
                }
                let Some(representative) = representative else {
                    bail_as!("AS TLAS storage 分配未入 handles 表".into());
                };
                resource_allocations.push(representative);
            }
        }

        let fci = FenceCreateInfo {
            s_type: ST_FENCE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: FENCE_CREATE_SIGNALED,
        };
        let mut fences = Vec::with_capacity(frame_slots);
        for slot in 0..frame_slots {
            let mut fence = VK_NULL_HANDLE;
            let r = (frame.dev.create_fence)(device, &fci, std::ptr::null(), &mut fence);
            if r != VK_SUCCESS {
                for old in fences.drain(..) {
                    (frame.dev.destroy_fence)(device, old, std::ptr::null());
                }
                frame.cleanup.destroy_all(&frame.dev, device);
                bail_as!(format!("vkCreateFence(frame slot {slot}) 失败: {r}"));
            }
            fences.push(fence);
        }
        Ok(NativePersistentFrame {
            instance,
            destroy_instance,
            messenger,
            destroy_messenger,
            validation_errors,
            pd,
            get_mem2,
            device,
            destroy_device,
            frame,
            fences,
            next_slot: 0,
            timestamp_period_ns: caps.timestamp_period_ns,
            // G38 T3:桥接时戳追加区首下标(创建期 pool 逐 pass 区总数同式;
            // persistent 路建面 query_slots 字面 = frame_slots,见上方调用)。
            bridge_query_base: (passes.len() as u32) * 2 * (frame_slots.max(1) as u32),
            memprops,
            resource_allocations,
            as_state,
            pipelined_slots: (0..frame_slots).map(|_| None).collect(),
            slot_busy: vec![false; frame_slots],
            pipelined_pool: VK_NULL_HANDLE,
            device_luid,
            get_memory_win32,
            exported_handles: Vec::new(),
        })
    })();

    match result {
        Ok(native) => Ok(native),
        Err(error) => {
            destroy_msgr(messenger);
            destroy_instance(instance, std::ptr::null());
            Err(error)
        }
    }
}

/// 持久帧 slot fence 有界等待共用超时(submit 的 slot-reuse 等待与 collect 的
/// 完成等待同一口径;拆分前为 `execute_persistent_frame` 内局部常量,值不变)。
const PERSISTENT_WAIT_TIMEOUT_NS: u64 = 5_000_000_000;

/// G38 T3:BLAS refit 桥接时戳 query 数(桥首 / copy 后 / build 后三点;
/// query pool 尾部追加区,不动既有逐 pass 时戳口径)。
const BRIDGE_QUERY_COUNT: u32 = 3;

/// 持久帧内部票据(G14plus RFC-0030 §4.3 L2:submit/collect 拆分承载体)。
/// [`submit_persistent_frame`] / [`submit_pipelined_frame`] 产,
/// [`collect_persistent_frame`] 消费。
struct PersistentFrameTicket {
    /// 本帧占用的 fence frame slot。
    slot: usize,
    /// 计时起点(slot-reuse 等待前采样;collect 完成等待后
    /// `cpu_fence_wait_ns = elapsed(wait_started)`——顺序路与拆分前逐字同口径;
    /// 流水路该值如实包含 submit 与 collect 之间的应用侧时间,诚实计量不掩饰)。
    wait_started: std::time::Instant,
    /// 帧前段(校验 + host 写 + 重录)耗时。
    record_ns: u64,
    /// vkQueueSubmit 耗时。
    cpu_submit_ns: u64,
    /// submit 起点的 validation ERROR 计数快照(collect 侧差分 fail-closed;
    /// 流水路差分可能含相邻在飞帧的消息——归因粗化但任何 ERROR 仍必致某帧 Err)。
    validation_before: u64,
    /// 回读计划((readback, session readbacks 下标);submit 期由 update 解析,
    /// collect 免持 update 引用)。
    rb_plan: Vec<(Readback, usize)>,
    /// true = FIF 流水路(collect 从 per-slot staging 读、timestamp 取 slot 区间、
    /// 完成后清 slot_busy);false = 顺序路(与拆分前行为逐字等价)。
    pipelined: bool,
}

/// 持久帧提交:fence slot 轮转(有界等待 + reset)→ [可选 FrameUpdate 帧前段:
/// host 上传 / TLAS transforms 写 / descriptor 重写 / cmd 重录] → submit → 完成
/// 有界等待 → timestamp / readback / heap budget / ledger telemetry。
/// `update = None` 为 Wave A 原样重放路径(行为 0-byte,不重录、不重写 descriptor)。
///
/// G14plus §4.3 L2:主体拆为 [`submit_persistent_frame`](到 vkQueueSubmit 为止)
/// 与 [`collect_persistent_frame`](当帧 fence 等待 + timestamp + 回读 + telemetry),
/// 本函数改为顺序调用两者——命令序/等待序/回读序与拆分前**逐字等价**
/// (位级零漂移的关键设计;FIF 流水消费走
/// [`DeviceFrameSession::submit_with_frame_update`]/[`DeviceFrameSession::collect`])。
#[allow(clippy::too_many_arguments)]
unsafe fn execute_persistent_frame(
    native: &mut NativePersistentFrame,
    resources: &[ResourceDesc<'_>],
    passes: &[Pass<'_>],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
    update: Option<&PreparedFrameUpdate<'_>>,
    bridge_ext: Option<&BlasRefitBridgeExt>,
) -> Result<(Vec<Vec<u8>>, DeviceFrameTelemetry), String> {
    // G38 T3:桥接计时读取判据 = 计时开启 ∧ 本帧带 blas_refit(录制侧写时戳
    // 的条件同源——写/读同帧一致,免 WAIT_BIT 悬垂)。
    let bridge_query = match (bridge_ext, update) {
        (Some(e), Some(up)) if e.collect_gpu_timing && up.blas.is_some() => {
            Some(native.bridge_query_base)
        }
        _ => None,
    };
    let ticket = submit_persistent_frame(
        native, resources, passes, barriers, readbacks, update, bridge_ext,
    )?;
    collect_persistent_frame(native, passes, ticket, bridge_query)
}

/// 持久帧提交半程(顺序路;G14plus §4.3 L2 拆分体):slot-reuse 有界等待 + reset
/// → FrameUpdate 帧前段 → vkQueueSubmit,返回票据。命令与调用序 = 拆分前
/// `execute_persistent_frame` 前半段逐字保序。
#[allow(clippy::too_many_arguments)]
unsafe fn submit_persistent_frame(
    native: &mut NativePersistentFrame,
    resources: &[ResourceDesc<'_>],
    passes: &[Pass<'_>],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
    update: Option<&PreparedFrameUpdate<'_>>,
    bridge_ext: Option<&BlasRefitBridgeExt>,
) -> Result<PersistentFrameTicket, String> {
    const WAIT_TIMEOUT_NS: u64 = PERSISTENT_WAIT_TIMEOUT_NS;
    // 顺序入口与流水票据不可交错(fail-closed):流水在飞帧占用 fence/query 区间/
    // 共享 SSBO,顺序路的单 cmd + [0,2P) query 区间会与之竞争。
    if native.slot_busy.iter().any(|&busy| busy) {
        return Err(
            "存在未 collect 的 FIF 流水票据:顺序 execute 入口须待全部票据 collect 后使用\
             (fail-closed,不静默交错)"
                .into(),
        );
    }
    let slot = native.next_slot;
    native.next_slot = (native.next_slot + 1) % native.fences.len();
    let fence = native.fences[slot];
    let validation_before = native
        .validation_errors
        .load(std::sync::atomic::Ordering::Relaxed);

    let wait_started = std::time::Instant::now();
    // C4 注入臂(默认关):第 n 次有界等待返回值覆写 VK_TIMEOUT 演习 TDR 处置面。
    let prior = g31_fault_fence_timeout((native.frame.dev.wait_fences)(
        native.device,
        1,
        &fence,
        1,
        WAIT_TIMEOUT_NS,
    ));
    if prior == VK_TIMEOUT {
        return Err(format!(
            "frame slot {slot} fence reuse bounded-wait 超时({WAIT_TIMEOUT_NS}ns;TDR-suspected)"
        ));
    }
    if prior != VK_SUCCESS {
        return Err(queue_result_error("vkWaitForFences(slot reuse)", prior));
    }
    let reset = (native.frame.dev.reset_fences)(native.device, 1, &fence);
    if reset != VK_SUCCESS {
        return Err(queue_result_error("vkResetFences", reset));
    }

    // ── FrameUpdate 帧前段(全在 submit 前;slot fence 已等待 + 上一帧完成等待已
    // 于上次调用返回前发生 → 无在途 GPU 工作,host 写 / descriptor 重写 / cmd 重置
    // 均无竞争;`cpu_record_ns` 如实计量本段)──
    let mut record_ns = 0u64;
    if let Some(up) = update {
        let record_started = std::time::Instant::now();
        // buffer 上传:目标 buffer host-visible+coherent(create_device_buffer 两条
        // 内存路径——首匹配 HV+HC 与 readback cached 优选——均含 HOST_COHERENT,
        // 故无 staging 分支、免 flush/invalidate——最简诚实路径),
        // submit 前 memcpy;Vulkan 保证 submit 后提交的工作可见 host 写。
        for &(res, offset, bytes) in up.uploads {
            let RtRes::Buf(rb) = &native.frame.rt[res as usize] else {
                return Err(format!("FrameUpdate: 上传目标资源 {res} 非 buffer"));
            };
            // SAFETY: rb.mem 为 host-visible+coherent 分配(建面纪律,免 flush);
            // offset+bytes.len() ≤ buffer size 已经 validate_frame_update;前帧 GPU
            // 工作已完成(上判 fence),无在途读。
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let map = (native.frame.dev.map_mem)(
                native.device,
                rb.mem,
                offset,
                bytes.len() as u64,
                0,
                &mut ptr,
            );
            if map != VK_SUCCESS || ptr.is_null() {
                return Err(format!(
                    "FrameUpdate: buffer 上传 vkMapMemory 失败(res {res}): {map}"
                ));
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), bytes.len());
            (native.frame.dev.unmap_mem)(native.device, rb.mem);
        }
        // TLAS 实例 transforms host 写(instance buffer 同 host-visible+coherent;
        // write_transforms 内做实例数/NaN fail-closed)。
        if let Some((as_index, instances, _)) = &up.tlas {
            let Some(state) = native.as_state.as_mut() else {
                return Err("FrameUpdate: tlas_update 指向无 AS 面的 session(校验漏网)".into());
            };
            let mgr = &mut state.managers[*as_index as usize];
            mgr.write_transforms(&state.fns, native.device, instances)?;
        }
        // G34-2:第二 TLAS 实例 transforms host 写(同 host-visible+coherent 面;
        // 校验面已保证与 tlas/blas 异槽,借用序先写后录不叠加)。
        if let Some((as_index_b, instances_b, _)) = &up.tlas_b {
            let Some(state) = native.as_state.as_mut() else {
                return Err("FrameUpdate: tlas_update_b 指向无 AS 面的 session(校验漏网)".into());
            };
            let mgr = &mut state.managers[*as_index_b as usize];
            mgr.write_transforms(&state.fns, native.device, instances_b)?;
        }
        // binding override pass descriptor set 重写(set 不在途;重写经创建期同一
        // write_pass_descriptor_set 事实源,布局键已由 host 校验逐位一致)。
        if !up.descriptor_overrides.is_empty() {
            let as_handles: Vec<u64> = native
                .as_state
                .as_ref()
                .map_or_else(Vec::new, |s| s.managers.iter().map(|m| m.tlas()).collect());
            for &pi in up.descriptor_overrides {
                let setup = &native.frame.setups[pi as usize];
                let Some(set) = setup.set else {
                    return Err(format!(
                        "FrameUpdate: pass {pi} 无 descriptor set(校验漏网)"
                    ));
                };
                write_pass_descriptor_set(
                    &native.frame.dev,
                    native.device,
                    set,
                    &up.effective_bindings[pi as usize],
                    &native.frame.rt,
                    native.frame.cleanup.sampler,
                    &as_handles,
                    pi as usize,
                )?;
            }
        }
        // 重录命令体(上传段不重录——初始数据已在资源内;TLAS update + consume
        // barrier → 固定 pass 链应用 overrides → timestamp → readback 子集)。
        if up.needs_rerecord {
            let cmd = native.frame.cmd;
            if (native.frame.dev.reset_cmd)(cmd, 0) != VK_SUCCESS {
                return Err("FrameUpdate: vkResetCommandBuffer 失败".into());
            }
            let cbi = CommandBufferBeginInfo {
                s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                p_inheritance_info: std::ptr::null(),
            };
            if (native.frame.dev.begin_cmd)(cmd, &cbi) != VK_SUCCESS {
                return Err("FrameUpdate: vkBeginCommandBuffer 失败".into());
            }
            (native.frame.dev.cmd_reset_query_pool)(
                cmd,
                native.frame.cleanup.query_pool,
                0,
                (passes.len() as u32) * 2,
            );
            // G31+ 波 A Task A4 TLAS update / 波 B Task B5 BLAS refit 归并同一
            // manager 借出(校验面已保证同帧同槽;单 &mut 承载全部 AS 操作)。
            // G34-2:第二 TLAS 更新(tlas_b)与主槽异槽(校验面保证)——
            // `split_at_mut` 双借承载;仅第二更新在案时归并主位(单更新语义
            // 与主位同律,既有面 0-byte)。
            let as_ops = match (&up.tlas, &up.blas, &up.tlas_b) {
                (None, None, None) => None,
                (tlas, blas, tlas_b) => {
                    let as_index = tlas
                        .map(|(i, _, _)| i)
                        .or_else(|| blas.map(|b| b.0))
                        .or_else(|| tlas_b.map(|(i, _, _)| i));
                    let Some(as_index) = as_index else {
                        return Err("AS 操作包空(内部不一致)".into());
                    };
                    let state = native
                        .as_state
                        .as_mut()
                        .ok_or("FrameUpdate: AS 操作无 AS 面(校验漏网)")?;
                    let b_slot = tlas_b.map(|(i, _, a)| (i, a));
                    let to_record = |b: (u32, u32, u32, u64, u64, u32)| {
                        let (_, blas_index, src_res, src_offset, byte_len, after_pass) = b;
                        BlasRefitRecord {
                            blas_index,
                            src_res,
                            src_offset,
                            byte_len,
                            after_pass,
                        }
                    };
                    let (tlas_action, blas_refit) =
                        (tlas.map(|(_, _, a)| a), blas.map(to_record));
                    // G37 W3 hzb_skin:第二 refit(校验面已保证与 tlas_b 同现
                    // 同槽——b 槽归并主位时同折主 refit 位,双借面恒自洽)。
                    let blas_refit_b = up.blas_b.map(to_record);
                    let (tlas_action, blas_refit, blas_refit_b, b_owned) = match b_slot {
                        Some((bi, ba)) if bi != as_index => {
                            (tlas_action, blas_refit, blas_refit_b, Some((bi, ba)))
                        }
                        // 仅第二更新在案(主位空):归并主位(校验面已拒同帧
                        // 同槽双写 ⇒ 到本臂 tlas_action/blas_refit 恒 None,
                        // blas_refit_b 随槽折入主 refit 位)。
                        Some((_, ba)) => (Some(ba), blas_refit_b, None, None),
                        None => (tlas_action, blas_refit, None, None),
                    };
                    if let Some((bi, ba)) = b_owned {
                        let (mgr, mgr_b): (&mut VkAsManager, &mut VkAsManager) =
                            if as_index < bi {
                                let (lo, hi) = state.managers.split_at_mut(bi as usize);
                                (&mut lo[as_index as usize], &mut hi[0])
                            } else {
                                let (lo, hi) = state.managers.split_at_mut(as_index as usize);
                                (&mut hi[0], &mut lo[bi as usize])
                            };
                        Some(AsFrameOps {
                            mgr,
                            fns: &state.fns,
                            tlas_action,
                            blas_refit,
                            tlas_b: Some((mgr_b, ba)),
                            blas_refit_b,
                        })
                    } else {
                        Some(AsFrameOps {
                            mgr: &mut state.managers[as_index as usize],
                            fns: &state.fns,
                            tlas_action,
                            blas_refit,
                            tlas_b: None,
                            blas_refit_b,
                        })
                    }
                }
            };
            let mut effective_rb: Vec<Option<(VkBuffer, VkDeviceMemory)>> = up
                .effective_rb_sources
                .iter()
                .map(|&source| native.frame.rb_buffers[source])
                .collect();
            // G14.11:release 集 = exportable ∪ imported(创建期录制同式)。
            let exportable_indices: Vec<u32> = native
                .frame
                .exportable_meta
                .iter()
                .map(|&(r, _, _)| r)
                .chain(native.frame.imported_indices.iter().copied())
                .collect();
            // G38 T3:桥扩展录制载荷(blas_refit 在案才有意义;None = 既有
            // 单 region 全量桥 + 无桥接时戳——命令流逐字节不变)。
            let bridge_query_base = native.bridge_query_base;
            let bridge_rec = match (bridge_ext, &up.blas) {
                (Some(e), Some(_)) => Some(BridgeRecordExt {
                    regions: e.copy_regions.as_deref(),
                    query_base: e.collect_gpu_timing.then_some(bridge_query_base),
                }),
                _ => None,
            };
            record_frame_body_ex(
                &FrameBodyParams {
                    dev: &native.frame.dev,
                    device: native.device,
                    memprops: &native.memprops,
                    cmd,
                    resources,
                    rt: &native.frame.rt,
                    passes,
                    barriers,
                    effective_bindings: up.effective_bindings,
                    setups: &native.frame.setups,
                    query_pool: native.frame.cleanup.query_pool,
                    query_base: 0,
                    inline_vbs: &native.frame.inline_vbs,
                    readbacks: up.effective_readbacks,
                    record_upload_segment: false,
                    exportable: &exportable_indices,
                    queue_family_index: native.frame.queue_family_index,
                    slot_set_overrides: None,
                },
                &mut effective_rb,
                None,
                as_ops,
                bridge_rec,
            )?;
            if (native.frame.dev.end_cmd)(cmd) != VK_SUCCESS {
                return Err("FrameUpdate: vkEndCommandBuffer 失败".into());
            }
            // 命令体所载 readback 源集形登记(下一帧同形即可原样重放)。
            native.frame.recorded_rb_sources = Some(up.effective_rb_sources.to_vec());
        }
        record_ns = elapsed_ns(record_started);
    }

    let si = SubmitInfo {
        s_type: ST_SUBMIT_INFO,
        p_next: std::ptr::null(),
        wait_semaphore_count: 0,
        p_wait_semaphores: std::ptr::null(),
        p_wait_dst_stage_mask: std::ptr::null(),
        command_buffer_count: 1,
        p_command_buffers: &native.frame.cmd,
        signal_semaphore_count: 0,
        p_signal_semaphores: std::ptr::null(),
    };
    let submit_started = std::time::Instant::now();
    let submit = (native.frame.dev.queue_submit)(native.frame.queue, 1, &si, fence);
    let cpu_submit_ns = elapsed_ns(submit_started);
    if submit != VK_SUCCESS {
        return Err(queue_result_error(
            "vkQueueSubmit(persistent frame)",
            submit,
        ));
    }

    // readback 计划(拆分前于完成等待后计算;仅依赖 update/readbacks,前移至
    // submit 期语义不变——collect 免持 update 引用):重放路径 = 全量 session
    // readbacks(下标恒等);update 路径 = readback_subset 解析结果。
    let rb_plan: Vec<(Readback, usize)> = match &update {
        Some(up) => up
            .effective_readbacks
            .iter()
            .copied()
            .zip(up.effective_rb_sources.iter().copied())
            .collect(),
        None => readbacks
            .iter()
            .copied()
            .enumerate()
            .map(|(i, r)| (r, i))
            .collect(),
    };
    Ok(PersistentFrameTicket {
        slot,
        wait_started,
        record_ns,
        cpu_submit_ns,
        validation_before,
        rb_plan,
        pipelined: false,
    })
}

/// 持久帧收集半程(G14plus §4.3 L2 拆分体):当帧 fence 有界等待 → timestamp
/// 查询 → 按票据回读计划 map 拷出 → heap budget / ledger / validation telemetry。
/// 顺序票据(`pipelined = false`)与拆分前 `execute_persistent_frame` 后半段
/// 逐字等价;流水票据从 per-slot staging 读、timestamp 取 slot 区间、完成后
/// 释放 slot 占用。
unsafe fn collect_persistent_frame(
    native: &mut NativePersistentFrame,
    passes: &[Pass<'_>],
    ticket: PersistentFrameTicket,
    bridge_query: Option<u32>,
) -> Result<(Vec<Vec<u8>>, DeviceFrameTelemetry), String> {
    const WAIT_TIMEOUT_NS: u64 = PERSISTENT_WAIT_TIMEOUT_NS;
    let PersistentFrameTicket {
        slot,
        wait_started,
        record_ns,
        cpu_submit_ns,
        validation_before,
        rb_plan,
        pipelined,
    } = ticket;
    if pipelined && !native.slot_busy[slot] {
        return Err(format!(
            "collect: frame slot {slot} 无在飞流水票据(重复 collect 或票据伪造;fail-closed)"
        ));
    }
    let fence = native.fences[slot];
    // C4 注入臂(默认关):第 n 次有界等待返回值覆写 VK_TIMEOUT 演习 TDR 处置面。
    let done = g31_fault_fence_timeout((native.frame.dev.wait_fences)(
        native.device,
        1,
        &fence,
        1,
        WAIT_TIMEOUT_NS,
    ));
    let cpu_fence_wait_ns = elapsed_ns(wait_started);
    if done == VK_TIMEOUT {
        return Err(format!(
            "frame slot {slot} fence completion bounded-wait 超时({WAIT_TIMEOUT_NS}ns;TDR-suspected)"
        ));
    }
    if done != VK_SUCCESS {
        return Err(queue_result_error(
            "vkWaitForFences(frame completion)",
            done,
        ));
    }

    // timestamp:顺序路恒 [0, passes*2)(拆分前逐字);流水路取本 slot 区间。
    let query_base = if pipelined {
        (slot * passes.len() * 2) as u32
    } else {
        0
    };
    let mut ticks = vec![0u64; passes.len() * 2];
    let query_result = (native.frame.dev.get_query_pool_results)(
        native.device,
        native.frame.cleanup.query_pool,
        query_base,
        ticks.len() as u32,
        ticks.len() * std::mem::size_of::<u64>(),
        ticks.as_mut_ptr().cast(),
        std::mem::size_of::<u64>() as u64,
        QUERY_RESULT_64_BIT | QUERY_RESULT_WAIT_BIT,
    );
    if query_result != VK_SUCCESS {
        return Err(queue_result_error(
            "vkGetQueryPoolResults(timestamp)",
            query_result,
        ));
    }
    let gpu_passes = passes
        .iter()
        .enumerate()
        .map(|(i, pass)| {
            let name = match pass {
                Pass::Raster(p) => p.name,
                Pass::Compute(p) => p.name,
            };
            PassGpuTiming {
                pass_id: i as u64 + 1,
                name: name.to_owned(),
                gpu_ns: ticks[i * 2 + 1].wrapping_sub(ticks[i * 2]) as f64
                    * f64::from(native.timestamp_period_ns),
            }
        })
        .collect();

    // G38 T3:桥接时戳追加区读取(仅本帧确实录写过才读——写/读判据在
    // execute_persistent_frame 同源;逐 pass 口径上方原样不动)。读取失败
    // fail-soft None(不冒充数值、不拒帧——观测面不设行为门)。
    let (blas_bridge_copy_gpu_ms, blas_bridge_build_gpu_ms) = match bridge_query {
        Some(qb) => {
            let mut bt = [0u64; BRIDGE_QUERY_COUNT as usize];
            let br = (native.frame.dev.get_query_pool_results)(
                native.device,
                native.frame.cleanup.query_pool,
                qb,
                BRIDGE_QUERY_COUNT,
                bt.len() * std::mem::size_of::<u64>(),
                bt.as_mut_ptr().cast(),
                std::mem::size_of::<u64>() as u64,
                QUERY_RESULT_64_BIT | QUERY_RESULT_WAIT_BIT,
            );
            if br == VK_SUCCESS {
                let period = f64::from(native.timestamp_period_ns);
                (
                    Some(bt[1].wrapping_sub(bt[0]) as f64 * period / 1e6),
                    Some(bt[2].wrapping_sub(bt[1]) as f64 * period / 1e6),
                )
            } else {
                (None, None)
            }
        }
        None => (None, None),
    };

    // 回读:顺序路 = 直接 map 资源/共享 rb buffer(拆分前逐字;fence 已等待 +
    // 分配恒 HOST_COHERENT → 免 vkInvalidateMappedMemoryRanges,cached 优选型
    // 同为 coherent,防御性注明);流水路 = map 本 slot staging(帧尾 GPU copy
    // 产物;共享 SSBO 可能已被后续在飞帧改写,故不可直接 map——per-slot 隔离
    // 即 FIF 位级确定性的承载)。
    let mut out = Vec::with_capacity(rb_plan.len());
    for (i, (readback, source)) in rb_plan.iter().enumerate() {
        let (memory, offset, size) = match *readback {
            Readback::Buffer { res, offset, size } => {
                let RtRes::Buf(buffer) = &native.frame.rt[res as usize] else {
                    return Err(format!("readbacks[{i}] buffer 资源类型漂移"));
                };
                if pipelined {
                    let Some(slot_state) = native.pipelined_slots[slot].as_ref() else {
                        return Err(format!("readbacks[{i}] 流水 slot 面缺失"));
                    };
                    (slot_state.rb_staging[*source].1, 0, size)
                } else if let Some((_, staging_mem)) = native.frame.rb_buffers[*source] {
                    // G14.10d:DEVICE_LOCAL 源不可 map——读帧尾 copy 产物 session 级
                    // cached staging(offset 已在 copy 侧消化)。
                    (staging_mem, 0, size)
                } else {
                    (buffer.mem, offset, size)
                }
            }
            Readback::Texture { res } => {
                let RtRes::Img(image) = &native.frame.rt[res as usize] else {
                    return Err(format!("readbacks[{i}] texture 资源类型漂移"));
                };
                let size = image.width as u64
                    * image.height as u64
                    * image.format.bytes_per_texel() as u64;
                if pipelined {
                    let Some(slot_state) = native.pipelined_slots[slot].as_ref() else {
                        return Err(format!("readbacks[{i}] 流水 slot 面缺失"));
                    };
                    (slot_state.rb_staging[*source].1, 0, size)
                } else {
                    let Some((_, memory)) = native.frame.rb_buffers[*source] else {
                        return Err(format!("readbacks[{i}] 持久 readback allocation 缺失"));
                    };
                    (memory, 0, size)
                }
            }
        };
        let map_size = size.max(4);
        let mut ptr = std::ptr::null_mut();
        let map = (native.frame.dev.map_mem)(native.device, memory, offset, map_size, 0, &mut ptr);
        if map != VK_SUCCESS || ptr.is_null() {
            return Err(format!("readbacks[{i}] vkMapMemory 失败: {map}"));
        }
        let mut bytes = vec![0u8; size as usize];
        std::ptr::copy_nonoverlapping(ptr.cast::<u8>(), bytes.as_mut_ptr(), bytes.len());
        (native.frame.dev.unmap_mem)(native.device, memory);
        out.push(bytes);
    }

    let mut budget = PhysicalDeviceMemoryBudgetPropertiesExt {
        s_type: ST_PHYSICAL_DEVICE_MEMORY_BUDGET_PROPERTIES_EXT,
        p_next: std::ptr::null_mut(),
        heap_budget: [0; 16],
        heap_usage: [0; 16],
    };
    let mut memory = PhysicalDeviceMemoryProperties2 {
        s_type: ST_PHYSICAL_DEVICE_MEMORY_PROPERTIES_2,
        p_next: (&mut budget as *mut PhysicalDeviceMemoryBudgetPropertiesExt).cast(),
        memory_properties: std::mem::zeroed(),
    };
    (native.get_mem2)(native.pd, &mut memory);
    let allocations: Vec<AllocationLedgerEntry> = native
        .frame
        .cleanup
        .allocations
        .iter()
        .map(|allocation| allocation.entry.clone())
        .collect();
    // C4 注入臂(默认关 = None 实值直通):budget 上报钳到注入上限演习 OOM 处置面。
    let budget_cap = g31_fault_budget_cap();
    let heaps = (0..memory.memory_properties.memory_heap_count as usize)
        .map(|heap_index| HeapBudgetTelemetry {
            heap_index: heap_index as u32,
            budget_bytes: match budget_cap {
                Some(cap) => budget.heap_budget[heap_index].min(cap),
                None => budget.heap_budget[heap_index],
            },
            driver_usage_bytes: budget.heap_usage[heap_index],
            ledger_bytes: allocations
                .iter()
                .filter(|entry| entry.heap_index as usize == heap_index)
                .map(|entry| entry.bytes)
                .sum(),
        })
        .collect::<Vec<_>>();
    if heaps.is_empty() || heaps.iter().any(|heap| heap.budget_bytes == 0) {
        return Err("VK_EXT_memory_budget 返回空/非法 heap 实值(fail-closed)".into());
    }
    if let Some(heap) = heaps
        .iter()
        .find(|heap| heap.driver_usage_bytes > heap.budget_bytes)
    {
        // C4 budget 违约处置面(注入臂下专项确定性 OOM 消息;默认关 = 既有
        // 非法实值消息逐字不变——驱动实报 usage>budget 本属非法遥测)。
        if budget_cap.is_some() {
            return Err(format!(
                "显存 budget 违约(OOM-suspected,fail-closed):heap {} driver_usage {} > \
                 budget {}(注入 budget 上限;分配请求超 budget 确定性失败,不降级不挂死)",
                heap.heap_index, heap.driver_usage_bytes, heap.budget_bytes
            ));
        }
        return Err("VK_EXT_memory_budget 返回空/非法 heap 实值(fail-closed)".into());
    }
    let validation_error_count = native
        .validation_errors
        .load(std::sync::atomic::Ordering::Relaxed);
    if validation_error_count > validation_before {
        return Err(format!(
            "VK_LAYER_KHRONOS_validation 新增 {} 条 ERROR(fail-closed)",
            validation_error_count - validation_before
        ));
    }
    let as_objects: u64 = native.as_state.as_ref().map_or(0, |state| {
        state.managers.iter().map(VkAsManager::object_count).sum()
    });
    let outstanding_object_count = native.frame.cleanup.object_count()
        + as_objects
        + native.fences.len() as u64
        + 3
        + u64::from(native.pipelined_pool != VK_NULL_HANDLE);
    let outstanding_allocation_count = allocations.len() as u64;
    // 流水票据成功收集 → 释放 slot 占用(fence 保持 signaled,下次 submit 时
    // wait+reset;Err 早退不清——TDR/device-lost/validation 级失败为会话性,
    // fail-closed 保持占用防悬垂 fence 竞态)。
    if pipelined {
        native.slot_busy[slot] = false;
    }
    Ok((
        out,
        DeviceFrameTelemetry {
            timestamp_period_ns: native.timestamp_period_ns,
            passes: gpu_passes,
            cpu_record_ns: record_ns,
            cpu_submit_ns,
            cpu_fence_wait_ns,
            heaps,
            allocations,
            validation_error_count,
            device_lost: false,
            tdr_suspected: false,
            outstanding_object_count,
            outstanding_allocation_count,
            leaked_object_count: 0,
            leaked_allocation_count: 0,
            blas_bridge_copy_gpu_ms,
            blas_bridge_build_gpu_ms,
        },
    ))
}

/// 全局 memory barrier2 录制(FIF 流水专用:帧间守卫 / staged copy 冲刷;
/// pass 图内资源级转换仍走 `record_frame_body` 单一事实源,不混用)。
///
/// # Safety
/// `cmd` 处于录制态;dev/device 同源。
unsafe fn cmd_global_barrier2(
    dev: &Dev,
    cmd: VkCommandBuffer,
    src_stage: u64,
    src_access: u64,
    dst_stage: u64,
    dst_access: u64,
) {
    let mb = MemoryBarrier2 {
        s_type: ST_MEMORY_BARRIER_2,
        p_next: std::ptr::null(),
        src_stage_mask: src_stage,
        src_access_mask: src_access,
        dst_stage_mask: dst_stage,
        dst_access_mask: dst_access,
    };
    let di = DependencyInfo {
        s_type: ST_DEPENDENCY_INFO,
        p_next: std::ptr::null(),
        dependency_flags: 0,
        memory_barrier_count: 1,
        p_memory_barriers: (&mb as *const MemoryBarrier2).cast::<c_void>(),
        buffer_memory_barrier_count: 0,
        p_buffer_memory_barriers: std::ptr::null(),
        image_memory_barrier_count: 0,
        p_image_memory_barriers: std::ptr::null(),
    };
    (dev.cmd_barrier2)(cmd, &di);
}

/// FIF 流水 slot 面懒建(G14plus §4.3 L2):per-slot cmd(同一 cmdpool——
/// persistent 路 RESET flag 已开,随 pool 销毁)+ 逐 session readback 的
/// per-slot staging(尺寸 = 声明尺寸;cached 优选——CPU 读向,§4.3 L1 同型)。
/// 缓冲经 `create_device_buffer` → cleanup 登记(对象/ledger/销毁单点纪律不变)。
///
/// # Safety
/// native 有效;slot < frame_slots;调用点在 slot fence 等待之后(无在途使用)。
unsafe fn ensure_pipelined_slot(
    native: &mut NativePersistentFrame,
    resources: &[ResourceDesc<'_>],
    readbacks: &[Readback],
    slot: usize,
) -> Result<(), String> {
    if native.pipelined_slots[slot].is_some() {
        return Ok(());
    }
    let cbai = CommandBufferAllocateInfo {
        s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        command_pool: native.frame.cleanup.cmdpool,
        level: CMD_BUFFER_LEVEL_PRIMARY,
        command_buffer_count: 1,
    };
    let mut cmd: VkCommandBuffer = std::ptr::null_mut();
    if (native.frame.dev.alloc_cmd)(native.device, &cbai, &mut cmd) != VK_SUCCESS {
        return Err(format!("FIF slot {slot}: vkAllocateCommandBuffers 失败"));
    }
    let mut rb_staging = Vec::with_capacity(readbacks.len());
    for (k, rb) in readbacks.iter().enumerate() {
        let size = match *rb {
            Readback::Buffer { size, .. } => size,
            Readback::Texture { res } => match &resources[res as usize] {
                ResourceDesc::Texture(t) => {
                    t.width as u64 * t.height as u64 * t.format.bytes_per_texel() as u64
                }
                ResourceDesc::Buffer(_) => {
                    return Err(format!(
                        "FIF slot {slot}: readbacks[{k}] texture 资源类型漂移"
                    ));
                }
            },
        };
        let (buf, mem) = create_device_buffer(
            &native.frame.dev,
            native.device,
            &native.memprops,
            size.max(4),
            0x2, // TRANSFER_DST(copy 目的)
            None,
            None,
            &mut native.frame.cleanup,
            // readback staging(GPU copy 写、CPU map 读)→ cached 优选。
            BufferMemClass::HostCachedPreferred,
        )?;
        rb_staging.push((buf, mem));
    }
    native.pipelined_slots[slot] = Some(PipelinedSlot {
        cmd,
        override_sets: Vec::new(),
        upload_staging: None,
        rb_staging,
    });
    Ok(())
}

/// FIF slot 上传 staging 保障(grow-only;扩容仅发生于 slot fence 已等待后——
/// 旧分配无在途使用;cleanup buffers/ledger 同步换新,销毁单点纪律不变——
/// 稳态上传尺寸固定(params 192B 类)即零 churn)。
///
/// # Safety
/// slot 面已建([`ensure_pipelined_slot`]);slot fence 已等待。
unsafe fn ensure_upload_staging(
    native: &mut NativePersistentFrame,
    slot: usize,
    needed: u64,
) -> Result<(VkBuffer, VkDeviceMemory), String> {
    let existing = native.pipelined_slots[slot]
        .as_ref()
        .and_then(|s| s.upload_staging);
    if let Some((buf, mem, capacity)) = existing {
        if capacity >= needed {
            return Ok((buf, mem));
        }
        // 扩容:旧缓冲销毁 + cleanup 登记同步摘除(否则 Drop 期双重销毁/ledger 假账)。
        (native.frame.dev.destroy_buffer)(native.device, buf, std::ptr::null());
        (native.frame.dev.free_mem)(native.device, mem, std::ptr::null());
        native.frame.cleanup.buffers.retain(|&(b, _)| b != buf);
        native.frame.cleanup.allocations.retain(|a| a.memory != mem);
    }
    let capacity = needed.max(256);
    let (buf, mem) = create_device_buffer(
        &native.frame.dev,
        native.device,
        &native.memprops,
        capacity,
        0x1, // TRANSFER_SRC(copy 源)
        None,
        None,
        &mut native.frame.cleanup,
        BufferMemClass::HostWc, // host 写向 staging:WC 型最优
    )?;
    let Some(slot_state) = native.pipelined_slots[slot].as_mut() else {
        return Err(format!("FIF slot {slot}: slot 面缺失(建面序漂移)"));
    };
    slot_state.upload_staging = Some((buf, mem, capacity));
    Ok((buf, mem))
}

/// G31(波 A Task A2)FIF 流水 per-slot descriptor override set 懒建:池懒
/// 建于首个带 binding_overrides 的流水 submit(尺寸 = 全 pass 声明组合 ×
/// frame_slots——override 布局键同构校验 ⇒ 池需求即声明组合上界;纯顺序
/// session 零对象增量纪律同 PipelinedSlot);set 按 (slot, pass) 懒分配自
/// `setups[pi].dsl`(布局事实源),不单独 free,随池销毁(创建期主池
/// flags=0 同律)。
///
/// # Safety
/// native 有效;slot < frame_slots;调用点在 slot fence 等待之后(本 slot set
/// 无在途 GPU 使用)。
unsafe fn ensure_pipelined_override_set(
    native: &mut NativePersistentFrame,
    passes: &[Pass<'_>],
    slot: usize,
    pass_index: usize,
) -> Result<VkDescriptorSet, String> {
    let frame_slots = native.fences.len() as u32;
    if native.pipelined_pool == VK_NULL_HANDLE {
        let mut total_as = 0u32;
        let mut total_sb = 0u32;
        let mut total_si = 0u32;
        let mut total_simg = 0u32;
        let mut total_ub = 0u32;
        let mut bound_passes = 0u32;
        for p in passes {
            let b = pass_bindings(p);
            let n_as = b.accel_structs.len() as u32;
            let n_sb = b.storage_buffers.len() as u32;
            let n_si = b.sampled_images.len() as u32;
            let n_simg = b.storage_images.len() as u32;
            let n_ub = u32::from(b.uniform.is_some());
            if n_as + n_sb + n_si + n_simg + n_ub > 0 {
                bound_passes += 1;
            }
            total_as += n_as;
            total_sb += n_sb;
            total_si += n_si;
            total_simg += n_simg;
            total_ub += n_ub;
        }
        if bound_passes == 0 {
            return Err("FIF: binding_overrides 指向无绑定 pass 集(校验漏网)".into());
        }
        let mut pool_sizes: Vec<DescriptorPoolSize> = Vec::new();
        for (dtype, total) in [
            (DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR, total_as),
            (DESCRIPTOR_TYPE_STORAGE_BUFFER, total_sb),
            (DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER, total_si),
            (DESCRIPTOR_TYPE_STORAGE_IMAGE, total_simg),
            (DESCRIPTOR_TYPE_UNIFORM_BUFFER, total_ub),
        ] {
            if total > 0 {
                pool_sizes.push(DescriptorPoolSize {
                    descriptor_type: dtype,
                    descriptor_count: total * frame_slots,
                });
            }
        }
        let dpci = DescriptorPoolCreateInfo {
            s_type: ST_DESCRIPTOR_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            max_sets: bound_passes * frame_slots,
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
        };
        let mut pool: VkDescriptorPool = VK_NULL_HANDLE;
        if (native.frame.dev.create_dp)(native.device, &dpci, std::ptr::null(), &mut pool)
            != VK_SUCCESS
        {
            return Err("FIF: vkCreateDescriptorPool(per-slot override) 失败".into());
        }
        native.pipelined_pool = pool;
    }
    let dsl = native.frame.setups[pass_index].dsl;
    {
        let slot_state = native.pipelined_slots[slot]
            .as_mut()
            .ok_or_else(|| format!("FIF slot {slot}: slot 面缺失(建面序漂移)"))?;
        if slot_state.override_sets.len() < passes.len() {
            slot_state.override_sets.resize(passes.len(), None);
        }
        if let Some(set) = slot_state.override_sets[pass_index] {
            return Ok(set);
        }
    }
    let dsai = DescriptorSetAllocateInfo {
        s_type: ST_DESCRIPTOR_SET_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        descriptor_pool: native.pipelined_pool,
        descriptor_set_count: 1,
        p_set_layouts: &dsl,
    };
    let mut set: VkDescriptorSet = VK_NULL_HANDLE;
    if (native.frame.dev.alloc_ds)(native.device, &dsai, &mut set) != VK_SUCCESS {
        return Err(format!(
            "FIF: pass {pass_index} per-slot override set vkAllocateDescriptorSets 失败"
        ));
    }
    let slot_state = native.pipelined_slots[slot]
        .as_mut()
        .ok_or_else(|| format!("FIF slot {slot}: slot 面缺失(建面序漂移)"))?;
    slot_state.override_sets[pass_index] = Some(set);
    Ok(set)
}

/// FIF 流水帧提交(G14plus RFC-0030 §4.3 L2):slot 占用检查 → slot-reuse 有界
/// 等待 + reset → per-slot 面懒建 → 上传写入本 slot staging → per-slot cmd 全量
/// 重录(帧间守卫 barrier → staged 上传 copies → 冲刷 barrier →
/// `record_frame_body` **同一录制事实源**(slot timestamp 区间)→ 帧尾 staged
/// buffer readback copies)→ submit(**不等完成 fence**——collect 侧等),返回
/// 流水票据。
///
/// 确定性论证(FIF=2 逐帧图像与 FIF=1 位级一致,§4.3 L2 D4):
/// - **GPU 帧间全序**:cmd 首条全局守卫 barrier(ALL_COMMANDS/MEMORY_WRITE →
///   ALL_COMMANDS/MEMORY_READ|WRITE)使帧 N+1 全部 GPU 访问序于帧 N 之后——
///   共享 SSBO 的跨帧数据依赖与顺序执行逐位同构(流水收益 = CPU 侧 submit 与
///   fence 等待解耦,非 GPU 帧重叠;GPU 帧间重叠留后续演进,登记)。
/// - **host 面隔离**:上传只写本 slot staging(同 slot 复用前 fence 等待保证无
///   在途消费);回读只读本 slot staging(帧尾 copy 产物,后续在飞帧不触碰)。
/// - staged 段均为 copy(内容逐位透传),不改任何 pass 命令/绑定/数据内容。
///
/// 拒绝面(公共入口 fail-closed,此处防御性复核):TLAS update(共享 instance
/// buffer host 写面)不入流水;descriptor 重写 G31 起经 per-slot override set
/// 支持(见 [`ensure_pipelined_override_set`] 安全性论证)。
///
/// # Safety
/// U32 契约同 [`submit_persistent_frame`];`prepared` 引用调用方栈上数据,
/// 生命周期限于本次调用。
unsafe fn submit_pipelined_frame(
    native: &mut NativePersistentFrame,
    resources: &[ResourceDesc<'_>],
    passes: &[Pass<'_>],
    barriers: &[&[(u32, TargetState)]],
    readbacks: &[Readback],
    prepared: &PreparedFrameUpdate<'_>,
) -> Result<PersistentFrameTicket, String> {
    const WAIT_TIMEOUT_NS: u64 = PERSISTENT_WAIT_TIMEOUT_NS;
    if prepared.tlas.is_some() {
        return Err("FIF 流水不支持 tlas_update(公共入口已拒;防御性复核)".into());
    }
    let slot = native.next_slot;
    if native.slot_busy[slot] {
        return Err(format!(
            "frame slot {slot} 票据未 collect(FIF 深度已满:先 collect 最早票据再 submit;\
             fail-closed 防 fence reset 悬垂)"
        ));
    }
    native.next_slot = (native.next_slot + 1) % native.fences.len();
    let fence = native.fences[slot];
    let validation_before = native
        .validation_errors
        .load(std::sync::atomic::Ordering::Relaxed);

    let wait_started = std::time::Instant::now();
    // C4 注入臂(默认关):第 n 次有界等待返回值覆写 VK_TIMEOUT 演习 TDR 处置面。
    let prior = g31_fault_fence_timeout((native.frame.dev.wait_fences)(
        native.device,
        1,
        &fence,
        1,
        WAIT_TIMEOUT_NS,
    ));
    if prior == VK_TIMEOUT {
        return Err(format!(
            "frame slot {slot} fence reuse bounded-wait 超时({WAIT_TIMEOUT_NS}ns;TDR-suspected)"
        ));
    }
    if prior != VK_SUCCESS {
        return Err(queue_result_error("vkWaitForFences(slot reuse)", prior));
    }
    let reset = (native.frame.dev.reset_fences)(native.device, 1, &fence);
    if reset != VK_SUCCESS {
        return Err(queue_result_error("vkResetFences", reset));
    }

    let record_started = std::time::Instant::now();
    ensure_pipelined_slot(native, resources, readbacks, slot)?;

    // ── G31:binding override → per-slot descriptor set 重写(共享 session set
    // 在飞帧使用中不可重写;本 slot set 的上次 GPU 使用 = 本 slot 上一帧,
    // slot fence 已等待,host 重写无在途竞争;写经创建期同一
    // write_pass_descriptor_set 事实源,内容与顺序路共享 set 重写产物逐位同)──
    let mut slot_set_overrides: Vec<Option<VkDescriptorSet>> = vec![None; passes.len()];
    if !prepared.descriptor_overrides.is_empty() {
        let as_handles: Vec<u64> = native
            .as_state
            .as_ref()
            .map_or_else(Vec::new, |s| s.managers.iter().map(|m| m.tlas()).collect());
        for &pi in prepared.descriptor_overrides {
            let set = ensure_pipelined_override_set(native, passes, slot, pi as usize)?;
            write_pass_descriptor_set(
                &native.frame.dev,
                native.device,
                set,
                &prepared.effective_bindings[pi as usize],
                &native.frame.rt,
                native.frame.cleanup.sampler,
                &as_handles,
                pi as usize,
            )?;
            slot_set_overrides[pi as usize] = Some(set);
        }
    }

    // ── 上传 → 本 slot staging(host 写;GPU copy 录于 cmd 首段)──
    let total_upload: u64 = prepared
        .uploads
        .iter()
        .map(|&(_, _, bytes)| bytes.len() as u64)
        .sum();
    // (staging 内偏移, 目标资源, 目标偏移, 字节数)。
    let mut staged_copies: Vec<(u64, u32, u64, u64)> = Vec::with_capacity(prepared.uploads.len());
    let mut upload_src: VkBuffer = VK_NULL_HANDLE;
    if total_upload > 0 {
        let (sbuf, smem) = ensure_upload_staging(native, slot, total_upload)?;
        upload_src = sbuf;
        let mut ptr: *mut c_void = std::ptr::null_mut();
        let map = (native.frame.dev.map_mem)(native.device, smem, 0, total_upload, 0, &mut ptr);
        if map != VK_SUCCESS || ptr.is_null() {
            return Err(format!(
                "FIF slot {slot}: 上传 staging vkMapMemory 失败: {map}"
            ));
        }
        let mut staging_offset = 0u64;
        for &(res, dst_offset, bytes) in prepared.uploads {
            if !matches!(&native.frame.rt[res as usize], RtRes::Buf(_)) {
                (native.frame.dev.unmap_mem)(native.device, smem);
                return Err(format!("FIF: 上传目标资源 {res} 非 buffer(校验漏网)"));
            }
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                ptr.cast::<u8>().add(staging_offset as usize),
                bytes.len(),
            );
            staged_copies.push((staging_offset, res, dst_offset, bytes.len() as u64));
            staging_offset += bytes.len() as u64;
        }
        (native.frame.dev.unmap_mem)(native.device, smem);
    }

    // ── per-slot cmd 全量重录(流水路恒重录:slot query 区间与 staged 段皆帧相关)──
    let query_base = (slot * passes.len() * 2) as u32;
    let dev = &native.frame.dev;
    let Some(slot_state) = native.pipelined_slots[slot].as_ref() else {
        return Err(format!("FIF slot {slot}: slot 面缺失(建面序漂移)"));
    };
    let slot_cmd = slot_state.cmd;
    if (dev.reset_cmd)(slot_cmd, 0) != VK_SUCCESS {
        return Err("FIF: vkResetCommandBuffer 失败".into());
    }
    let cbi = CommandBufferBeginInfo {
        s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_inheritance_info: std::ptr::null(),
    };
    if (dev.begin_cmd)(slot_cmd, &cbi) != VK_SUCCESS {
        return Err("FIF: vkBeginCommandBuffer 失败".into());
    }
    (dev.cmd_reset_query_pool)(
        slot_cmd,
        native.frame.cleanup.query_pool,
        query_base,
        (passes.len() as u32) * 2,
    );
    // 帧间守卫(见函数头确定性论证)。
    cmd_global_barrier2(
        dev,
        slot_cmd,
        STAGE2_ALL_COMMANDS,
        ACCESS2_MEMORY_WRITE,
        STAGE2_ALL_COMMANDS,
        ACCESS2_MEMORY_READ | ACCESS2_MEMORY_WRITE,
    );
    if !staged_copies.is_empty() {
        for &(src_offset, res, dst_offset, size) in &staged_copies {
            let RtRes::Buf(rb) = &native.frame.rt[res as usize] else {
                return Err(format!("FIF: 上传目标资源 {res} 非 buffer(上判已拒)"));
            };
            let region = VkBufferCopy {
                src_offset,
                dst_offset,
                size,
            };
            (dev.cmd_copy_buf)(slot_cmd, upload_src, rb.buffer, 1, &region);
        }
        // staged 上传冲刷:pass 图 tracked 初值屏障 src=HOST 不覆盖 TRANSFER 写,
        // 此处显式冲刷后 record_frame_body 命令流与顺序路逐字同形。
        cmd_global_barrier2(
            dev,
            slot_cmd,
            STAGE2_TRANSFER,
            ACCESS2_TRANSFER_WRITE,
            STAGE2_ALL_COMMANDS,
            ACCESS2_MEMORY_READ | ACCESS2_MEMORY_WRITE,
        );
    }
    // texture readback copy 目的 = 本 slot staging;buffer readback 由流水路
    // 自己的帧尾 staged copy 承载(per-slot 隔离)——恒 None 占位,即便源为
    // DEVICE_LOCAL(G14.10d session 级 staging 只服务顺序路;record_frame_body
    // 对 None 项免录制,不会双写)。
    let mut effective_rb: Vec<Option<(VkBuffer, VkDeviceMemory)>> = prepared
        .effective_readbacks
        .iter()
        .zip(prepared.effective_rb_sources.iter())
        .map(|(rb, &source)| match rb {
            Readback::Texture { .. } => Some(slot_state.rb_staging[source]),
            Readback::Buffer { .. } => None,
        })
        .collect();
    // G14.11:release 集 = exportable ∪ imported(创建期录制同式)。
    let exportable_indices: Vec<u32> = native
        .frame
        .exportable_meta
        .iter()
        .map(|&(r, _, _)| r)
        .chain(native.frame.imported_indices.iter().copied())
        .collect();
    record_frame_body(
        &FrameBodyParams {
            dev,
            device: native.device,
            memprops: &native.memprops,
            cmd: slot_cmd,
            resources,
            rt: &native.frame.rt,
            passes,
            barriers,
            effective_bindings: prepared.effective_bindings,
            setups: &native.frame.setups,
            query_pool: native.frame.cleanup.query_pool,
            query_base,
            inline_vbs: &native.frame.inline_vbs,
            readbacks: prepared.effective_readbacks,
            record_upload_segment: false,
            exportable: &exportable_indices,
            queue_family_index: native.frame.queue_family_index,
            slot_set_overrides: Some(&slot_set_overrides),
        },
        &mut effective_rb,
        None,
        None,
    )?;
    // ── 帧尾 staged buffer readback copies(pass 链写完 → TRANSFER copy 至本
    // slot staging;后续在飞帧改写共享 SSBO 不再影响本帧回读内容)──
    let has_buffer_rb = prepared
        .effective_readbacks
        .iter()
        .any(|rb| matches!(rb, Readback::Buffer { .. }));
    if has_buffer_rb {
        cmd_global_barrier2(
            dev,
            slot_cmd,
            STAGE2_ALL_COMMANDS,
            ACCESS2_MEMORY_WRITE,
            STAGE2_TRANSFER,
            ACCESS2_TRANSFER_READ,
        );
        for (rb, &source) in prepared
            .effective_readbacks
            .iter()
            .zip(prepared.effective_rb_sources.iter())
        {
            if let Readback::Buffer { res, offset, size } = *rb {
                let RtRes::Buf(src) = &native.frame.rt[res as usize] else {
                    return Err(format!("FIF: readback 资源 {res} 非 buffer(类型漂移)"));
                };
                let region = VkBufferCopy {
                    src_offset: offset,
                    dst_offset: 0,
                    size,
                };
                (dev.cmd_copy_buf)(
                    slot_cmd,
                    src.buffer,
                    slot_state.rb_staging[source].0,
                    1,
                    &region,
                );
            }
        }
    }
    if (dev.end_cmd)(slot_cmd) != VK_SUCCESS {
        return Err("FIF: vkEndCommandBuffer 失败".into());
    }
    let record_ns = elapsed_ns(record_started);

    let si = SubmitInfo {
        s_type: ST_SUBMIT_INFO,
        p_next: std::ptr::null(),
        wait_semaphore_count: 0,
        p_wait_semaphores: std::ptr::null(),
        p_wait_dst_stage_mask: std::ptr::null(),
        command_buffer_count: 1,
        p_command_buffers: &slot_cmd,
        signal_semaphore_count: 0,
        p_signal_semaphores: std::ptr::null(),
    };
    let submit_started = std::time::Instant::now();
    let submit = (native.frame.dev.queue_submit)(native.frame.queue, 1, &si, fence);
    let cpu_submit_ns = elapsed_ns(submit_started);
    if submit != VK_SUCCESS {
        return Err(queue_result_error(
            "vkQueueSubmit(FIF pipelined frame)",
            submit,
        ));
    }
    native.slot_busy[slot] = true;

    let rb_plan: Vec<(Readback, usize)> = prepared
        .effective_readbacks
        .iter()
        .copied()
        .zip(prepared.effective_rb_sources.iter().copied())
        .collect();
    Ok(PersistentFrameTicket {
        slot,
        wait_started,
        record_ns,
        cpu_submit_ns,
        validation_before,
        rb_plan,
        pipelined: true,
    })
}

fn queue_result_error(op: &str, result: VkResult) -> String {
    if result == VK_ERROR_DEVICE_LOST {
        format!("{op} 失败: VK_ERROR_DEVICE_LOST({result})(device loss,fail-closed)")
    } else {
        format!("{op} 失败: {result}")
    }
}

// ── G31+ 波 C Task C4 故障注入面（env 门控默认关,零行为变更;OnceLock 一次解析,
// 热路径仅一次原子读）──
/// `RURIX_G31_FAULT_FENCE_TIMEOUT=<n>`(n ≥ 1):第 n 次(1-based)持久帧 fence
/// 有界等待的**返回值**被覆写为 `VK_TIMEOUT`(真实等待已先行完成——fence/GPU
/// 态不受污染;仅超时处置面 = "TDR-suspected" 确定性 Err 被演习,长帧/卡死
/// 模拟不挂死进程、不真触系统 TDR)。未设置/解析失败 → 原样直通(默认关)。
fn g31_fault_fence_timeout(real: VkResult) -> VkResult {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TARGET: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let target = TARGET.get_or_init(|| {
        std::env::var("RURIX_G31_FAULT_FENCE_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&n| n >= 1)
    });
    match target {
        Some(n) if SEQ.fetch_add(1, Ordering::Relaxed) + 1 == *n => VK_TIMEOUT,
        _ => real,
    }
}

/// `RURIX_G31_FAULT_BUDGET_BYTES=<n>`(n ≥ 1):telemetry 采集的 heap budget
/// 上报值被钳到 n(usage 实值不动)→ 显存分配请求超 budget 的违约处置面
/// (确定性 OOM Err,fail-closed)演习;未设置/解析失败 → `None` 实值直通(默认关)。
fn g31_fault_budget_cap() -> Option<u64> {
    static CAP: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    *CAP.get_or_init(|| {
        std::env::var("RURIX_G31_FAULT_BUDGET_BYTES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&n| n >= 1)
    })
}

impl Drop for NativePersistentFrame {
    fn drop(&mut self) {
        // SAFETY: NativePersistentFrame 单所有者；最终 teardown 允许 queueWaitIdle。所有 fence /
        // frame object / AS manager(经 as fns,单所有者逆序)/ device / messenger / instance
        // 均按创建逆序且只销毁一次;AS 分配在 ledger 仅记账,不由 Cleanup 释放(无双重释放)。
        // G14.10b:导出 NT handle 先于 memory 释放单点 CloseHandle(handle 引用
        // 由 OS 计数,导入方已 import 的内存不受影响;本 session 不再持有)。
        unsafe {
            let _ = (self.frame.dev.queue_wait)(self.frame.queue);
            #[cfg(windows)]
            for (_, h) in self.exported_handles.drain(..) {
                let _ = CloseHandle(h as *mut c_void);
            }
            for fence in self.fences.drain(..) {
                (self.frame.dev.destroy_fence)(self.device, fence, std::ptr::null());
            }
            // G31:per-slot override descriptor 池(set 随池释放,不单独 free)。
            if self.pipelined_pool != VK_NULL_HANDLE {
                (self.frame.dev.destroy_dp)(self.device, self.pipelined_pool, std::ptr::null());
            }
            self.frame.cleanup.destroy_all(&self.frame.dev, self.device);
            if let Some(state) = self.as_state.as_mut() {
                for mgr in &mut state.managers {
                    mgr.destroy(&state.fns, self.device);
                }
            }
            (self.destroy_device)(self.device, std::ptr::null());
            if self.messenger != VK_NULL_HANDLE
                && let Some(destroy) = self.destroy_messenger
            {
                destroy(self.instance, self.messenger, std::ptr::null());
            }
            (self.destroy_instance)(self.instance, std::ptr::null());
        }
    }
}

// ─────────────────────────── 测试 ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ① host 侧(无 GPU 恒跑):pipeline cache 键 / 屏障表构造 / 校验 / 布局锚 ──

    #[test]
    fn ffi_layout_anchors() {
        // #[repr(C)] 结构尺寸/对齐锚定(与 Vulkan spec 逐字节对齐的运行期断言;
        // vk.rs mesh_rt_ffi_layout_anchors 先例同律)。x64 指针 8B。
        assert_eq!(size_of::<ApplicationInfo>(), 48);
        assert_eq!(size_of::<InstanceCreateInfo>(), 64);
        assert_eq!(size_of::<BufferCreateInfo>(), 56);
        assert_eq!(size_of::<ImageCreateInfo>(), 88);
        assert_eq!(size_of::<ImageViewCreateInfo>(), 80);
        assert_eq!(size_of::<ShaderModuleCreateInfo>(), 40);
        assert_eq!(size_of::<DescriptorSetLayoutBinding>(), 24);
        assert_eq!(size_of::<PushConstantRange>(), 12);
        assert_eq!(size_of::<PipelineLayoutCreateInfo>(), 48);
        assert_eq!(size_of::<PipelineShaderStageCreateInfo>(), 48);
        assert_eq!(size_of::<ComputePipelineCreateInfo>(), 96);
        assert_eq!(size_of::<DescriptorPoolSize>(), 8);
        assert_eq!(size_of::<DescriptorPoolCreateInfo>(), 40);
        assert_eq!(size_of::<DescriptorSetAllocateInfo>(), 40);
        assert_eq!(size_of::<DescriptorBufferInfo>(), 24);
        assert_eq!(size_of::<DescriptorImageInfo>(), 24);
        assert_eq!(size_of::<WriteDescriptorSet>(), 64);
        assert_eq!(size_of::<AttachmentDescription>(), 36);
        assert_eq!(size_of::<AttachmentReference>(), 8);
        assert_eq!(size_of::<SubpassDescription>(), 72);
        assert_eq!(size_of::<RenderPassCreateInfo>(), 64);
        assert_eq!(size_of::<FramebufferCreateInfo>(), 64);
        assert_eq!(size_of::<VkVertexInputBindingDescription>(), 12);
        assert_eq!(size_of::<VkVertexInputAttributeDescription>(), 16);
        assert_eq!(size_of::<PipelineVertexInputStateCreateInfo>(), 48);
        assert_eq!(size_of::<PipelineRasterizationStateCreateInfo>(), 64);
        assert_eq!(size_of::<PipelineMultisampleStateCreateInfo>(), 48);
        assert_eq!(size_of::<PipelineColorBlendAttachmentState>(), 32);
        assert_eq!(size_of::<PipelineColorBlendStateCreateInfo>(), 56);
        assert_eq!(size_of::<PipelineDepthStencilStateCreateInfo>(), 104);
        assert_eq!(size_of::<GraphicsPipelineCreateInfo>(), 144);
        assert_eq!(size_of::<ClearValue>(), 16);
        assert_eq!(size_of::<RenderPassBeginInfo>(), 64);
        assert_eq!(size_of::<VkBufferImageCopy>(), 56);
        assert_eq!(size_of::<SamplerCreateInfo>(), 80);
        assert_eq!(size_of::<BufferMemoryBarrier2>(), 80);
        assert_eq!(size_of::<ImageMemoryBarrier2>(), 96);
        assert_eq!(size_of::<MemoryBarrier2>(), 48);
        assert_eq!(size_of::<VkBufferCopy>(), 24);
        assert_eq!(size_of::<DependencyInfo>(), 64);
        assert_eq!(size_of::<PhysicalDeviceSynchronization2Features>(), 24);
        assert_eq!(size_of::<PhysicalDeviceShaderAtomicInt64Features>(), 24);
        assert_eq!(size_of::<PhysicalDeviceRayQueryFeatures>(), 24);
        assert_eq!(size_of::<PhysicalDeviceAccelerationStructureFeatures>(), 40);
        assert_eq!(size_of::<PhysicalDeviceBufferDeviceAddressFeatures>(), 32);
        assert_eq!(size_of::<PhysicalDeviceDescriptorIndexingFeatures>(), 96);
        assert_eq!(size_of::<PhysicalDeviceFeatures2>(), 240);
        assert_eq!(size_of::<ExtensionProperties>(), 260);
        assert_eq!(align_of::<PropertiesBlob>(), 8);
        assert_eq!(size_of::<DebugUtilsMessengerCreateInfoEXT>(), 48);
        assert_eq!(size_of::<DebugUtilsMessengerCallbackDataEXT>(), 96);
        assert_eq!(size_of::<MemoryRequirements>(), 24);
        assert_eq!(size_of::<MemoryAllocateInfo>(), 32);
        assert_eq!(size_of::<PhysicalDeviceMemoryProperties>(), 520);
        assert_eq!(size_of::<QueueFamilyProperties>(), 24);
        assert_eq!(size_of::<SubmitInfo>(), 72);
        assert_eq!(size_of::<CommandPoolCreateInfo>(), 24);
        assert_eq!(size_of::<FenceCreateInfo>(), 24);
        assert_eq!(size_of::<QueryPoolCreateInfo>(), 32);
        assert_eq!(size_of::<PhysicalDeviceMemoryProperties2>(), 536);
        assert_eq!(size_of::<PhysicalDeviceMemoryBudgetPropertiesExt>(), 272);
        assert_eq!(size_of::<CommandBufferAllocateInfo>(), 32);
        assert_eq!(size_of::<CommandBufferBeginInfo>(), 32);
        assert_eq!(size_of::<VkViewport>(), 24);
        assert_eq!(size_of::<VkRect2D>(), 16);
        assert_eq!(size_of::<DeviceQueueCreateInfo>(), 40);
        assert_eq!(size_of::<DeviceCreateInfo>(), 72);
        assert_eq!(size_of::<PipelineInputAssemblyStateCreateInfo>(), 32);
        assert_eq!(size_of::<PipelineViewportStateCreateInfo>(), 48);
        assert_eq!(size_of::<VkExtent3D>(), 12);
        assert_eq!(size_of::<VkImageSubresourceRange>(), 20);
        assert_eq!(size_of::<VkImageSubresourceLayers>(), 16);
        // G7.6 Wave B:AS descriptor pNext 结构(vk.rs pub(crate) 复用,禁第二份定义)。
        assert_eq!(size_of::<WriteDescriptorSetAccelStructure>(), 32);
        // G7.5b 保守光栅(RXS-0303):sType@0 / pNext@8 / 3×f32@16 / 6×VkBool32@28,
        // 尾补齐 → 56;pipeline pNext 节点 sType@0 / pNext@8 / flags@16 / mode@20 /
        // extra@24 → 32(SDK 1.3.296 `vulkan_core.h` 逐字段核对)。
        assert_eq!(
            size_of::<PhysicalDeviceConservativeRasterizationProperties>(),
            56
        );
        assert_eq!(
            size_of::<PipelineRasterizationConservativeStateCreateInfo>(),
            32
        );
        assert_eq!(align_of::<PhysicalDeviceProperties2Chain>(), 8);
        // G14.10b external memory 导出面(SDK 1.3.296 `vulkan_core.h` 逐字段核对):
        // IDProperties sType@0/pNext@8/UUID×2@16..48/LUID@48/nodeMask@56/valid@60 → 64;
        // 三个 24/32 小节点与 win32 get/import 结构。
        assert_eq!(size_of::<PhysicalDeviceIDProperties>(), 64);
        assert_eq!(size_of::<ExternalMemoryImageCreateInfo>(), 24);
        assert_eq!(size_of::<ExportMemoryAllocateInfo>(), 24);
        assert_eq!(size_of::<MemoryDedicatedAllocateInfo>(), 32);
        assert_eq!(size_of::<MemoryGetWin32HandleInfoKHR>(), 32);
        assert_eq!(size_of::<ImportMemoryWin32HandleInfoKHR>(), 40);
    }

    #[test]
    fn provenance_rejects_wrong_producer_allocation_before_submit() {
        let data = [1u8, 2, 3, 4];
        let resources = [
            ResourceDesc::Buffer(BufferDesc {
                size: 4,
                usage: BufferUsage {
                    storage: true,
                    ..BufferUsage::default()
                },
                data: Some(&data),
                device_local: false,
            }),
            ResourceDesc::Buffer(BufferDesc {
                size: 4,
                usage: BufferUsage {
                    storage: true,
                    ..BufferUsage::default()
                },
                data: None,
                device_local: false,
            }),
        ];
        let pass = Pass::Compute(ComputePass {
            name: "name-is-diagnostic-only",
            spirv: &[],
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([1, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![0, 1],
                ..Bindings::default()
            },
        });
        let expected = build_runtime_provenance(
            &resources,
            &[pass],
            &[StableAllocationId(11), StableAllocationId(29)],
            &[3, 7],
            8,
        );
        validate_submission_provenance(&expected, &expected).expect("actual provenance 应通过");
        let mut wrong = expected.clone();
        wrong.passes[0].resources[0]
            .producer
            .as_mut()
            .expect("storage readwrite 有 producer")
            .allocation_id = StableAllocationId(999);
        assert!(
            validate_submission_provenance(&expected, &wrong)
                .unwrap_err()
                .contains("allocation/generation")
        );
    }

    fn test_caps() -> DeviceCaps {
        DeviceCaps {
            device_name: "host-mock".to_owned(),
            synchronization2: false,
            shader_buffer_int64_atomics: false,
            shader_int64: false,
            fragment_stores_and_atomics: false,
            ray_query: false,
            acceleration_structure: false,
            buffer_device_address: false,
            descriptor_indexing: false,
            deferred_host_operations: false,
            memory_budget: false,
            conservative_raster: None,
            timestamp_period_ns: 1.0,
            max_push_constants_size: 128,
        }
    }

    #[test]
    fn wave_gate_is_cumulative_and_fail_closed() {
        let mut caps = test_caps();
        assert_eq!(
            require_wave(&caps, KernelWave::W1).unwrap_err().to_string(),
            "内核波次 W1 缺失设备能力: synchronization2"
        );

        caps.synchronization2 = true;
        require_wave(&caps, KernelWave::W1).expect("sync2 基线应通过 W1");
        assert_eq!(
            require_wave(&caps, KernelWave::W2).unwrap_err().to_string(),
            "内核波次 W2 缺失设备能力: shader_buffer_int64_atomics"
        );

        caps.shader_buffer_int64_atomics = true;
        require_wave(&caps, KernelWave::W2).expect("sync2 + atomic int64 应通过 W2");
        assert_eq!(
            require_wave(&caps, KernelWave::W3).unwrap_err().to_string(),
            "内核波次 W3 缺失设备能力: ray_query, acceleration_structure, \
             buffer_device_address, descriptor_indexing, deferred_host_operations"
        );

        caps.ray_query = true;
        caps.acceleration_structure = true;
        caps.buffer_device_address = true;
        caps.descriptor_indexing = true;
        caps.deferred_host_operations = true;
        require_wave(&caps, KernelWave::W3).expect("完整累积能力应通过 W3");
    }

    #[test]
    fn kernel_wave_routes_are_complete_and_unique() {
        let expected = [
            ("cull", KernelWave::W1),
            ("classify_resolve", KernelWave::W1),
            ("vsm_page_mark", KernelWave::W1),
            ("taa", KernelWave::W1),
            ("visbuffer_sw_u64", KernelWave::W2),
            ("gi_probe", KernelWave::W3),
            ("rtao", KernelWave::W3),
            ("hard_shadow", KernelWave::W3),
        ];
        assert_eq!(KERNEL_WAVE_ROUTES.len(), expected.len());
        for (kernel, wave) in expected {
            assert_eq!(kernel_wave(kernel), Some(wave), "内核 `{kernel}` 波次");
            assert_eq!(
                KERNEL_WAVE_ROUTES
                    .iter()
                    .filter(|route| route.kernel == kernel)
                    .count(),
                1,
                "内核 `{kernel}` 路由须唯一"
            );
        }
        assert_eq!(kernel_wave("unknown"), None);
    }

    #[test]
    fn fnv_hash_deterministic_and_distinct() {
        let a = b"rurix-render-exec";
        assert_eq!(fnv1a_u64(a), fnv1a_u64(a));
        assert_ne!(fnv1a_u64(a), fnv1a_u64(b"rurix-render-exec!"));
        assert_ne!(fnv1a_u64(b""), fnv1a_u64(b"\0"));
    }

    #[test]
    fn pipeline_cache_keys_equality() {
        let k1 = RasterPipelineKey {
            vs_hash: 1,
            fs_hash: 2,
            color_formats: vec![37],
            depth_format: 0,
            vertex_stride: 32,
            attrs: vec![(0, 109, 0), (1, 109, 16)],
            has_vb: true,
            conservative: None,
        };
        let mut k2 = k1.clone();
        assert_eq!(k1, k2);
        k2.fs_hash = 3;
        assert_ne!(k1, k2);
        let mut k3 = k1.clone();
        k3.attrs[1].2 = 20;
        assert_ne!(k1, k3);
        let mut k4 = k1.clone();
        k4.depth_format = 126;
        assert_ne!(k1, k4);
        // 顶点布局变化(stride/attrs/has_vb)必须换键(G5 多材质 PSO 变体之根)。
        let mut k5 = k1.clone();
        k5.has_vb = false;
        k5.vertex_stride = 0;
        k5.attrs.clear();
        assert_ne!(k1, k5);
        // 保守光栅状态入键(G7.5b,RXS-0303 L2):on/off 与 extra 位型均换键。
        let mut k6 = k1.clone();
        k6.conservative = Some(0.0f32.to_bits());
        assert_ne!(k1, k6);
        let mut k7 = k6.clone();
        k7.conservative = Some(0.25f32.to_bits());
        assert_ne!(k6, k7);
        let c1 = ComputePipelineKey {
            spv_hash: 7,
            entry: b"main".to_vec(),
        };
        let c2 = ComputePipelineKey {
            spv_hash: 7,
            entry: b"main2".to_vec(),
        };
        assert_ne!(c1, c2);
    }

    /// G7.5b(RXS-0303 L3):conservative pass × 无扩展 caps → 确定性 Err;
    /// caps 在位 / pass 不要求 → Ok(纯 host 红绿)。
    #[test]
    fn conservative_raster_gate_is_fail_closed() {
        let raster = |conservative: Option<ConservativeRasterDesc>| {
            Pass::Raster(RasterPass {
                name: "hw",
                vs_spirv: &[],
                fs_spirv: &[],
                vertex: VertexData::Pull,
                draw: DrawSpec::Direct {
                    vertex_count: 3,
                    instance_count: 1,
                    first_vertex: 0,
                    first_instance: 0,
                },
                colors: vec![],
                depth: None,
                viewport: None,
                bindings: Bindings::default(),
                conservative,
            })
        };
        let caps_none = test_caps();
        let mut caps_some = test_caps();
        caps_some.conservative_raster = Some(ConservativeRasterProps {
            primitive_overestimation_size: 0.001_953_12,
            max_extra_primitive_overestimation_size: 0.75,
            extra_primitive_overestimation_size_granularity: 0.25,
            degenerate_triangles_rasterized: true,
        });
        let want = raster(Some(ConservativeRasterDesc {
            extra_overestimation: 0.0,
        }));
        let plain = raster(None);
        // RED:要求 conservative 而 caps 无扩展 → 确定性 Err(消息含 fail-closed 归因)。
        let err = validate_conservative_raster(std::slice::from_ref(&want), &caps_none)
            .expect_err("caps 无保守光栅时 conservative pass 必须确定性拒");
        assert!(
            err.contains("VK_EXT_conservative_rasterization") && err.contains("fail-closed"),
            "错误消息须携归因: {err}"
        );
        // GREEN:caps 在位 → 放行;pass 不要求 → 与 caps 无关恒放行(0-byte 行为)。
        validate_conservative_raster(std::slice::from_ref(&want), &caps_some)
            .expect("caps 在位应放行");
        validate_conservative_raster(std::slice::from_ref(&plain), &caps_none)
            .expect("conservative=None 不受 caps 影响");
    }

    #[test]
    fn set0_layout_convention() {
        // 固定约定:binding [0..N) storage → [N..N+M) sampled → [N+M..N+M+K) storage image
        // → [N+M+K] uniform(模块头文档级契约的可执行形态)。
        let plan = plan_set0_layout(2, 1, 1, true);
        assert_eq!(
            plan,
            vec![
                (0, DESCRIPTOR_TYPE_STORAGE_BUFFER),
                (1, DESCRIPTOR_TYPE_STORAGE_BUFFER),
                (2, DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER),
                (3, DESCRIPTOR_TYPE_STORAGE_IMAGE),
                (4, DESCRIPTOR_TYPE_UNIFORM_BUFFER),
            ]
        );
        assert!(plan_set0_layout(0, 0, 0, false).is_empty());
        assert_eq!(
            plan_set0_layout(0, 2, 0, false),
            vec![
                (0, DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER),
                (1, DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER),
            ]
        );
    }

    #[test]
    fn set0_layout_with_as_convention() {
        // G7.6 Wave B:AS 段占 binding [0..A)(rurixc `AccelStruct` 首形参 = binding 0),
        // 其余四段顺延;A=0 与既有五段布局逐位一致(0-byte 兼容锚)。
        let plan = plan_set0_layout_with_as(1, 2, 1, 1, true);
        assert_eq!(
            plan,
            vec![
                (0, DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR),
                (1, DESCRIPTOR_TYPE_STORAGE_BUFFER),
                (2, DESCRIPTOR_TYPE_STORAGE_BUFFER),
                (3, DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER),
                (4, DESCRIPTOR_TYPE_STORAGE_IMAGE),
                (5, DESCRIPTOR_TYPE_UNIFORM_BUFFER),
            ]
        );
        // 多 AS(同 TLAS 复绑之外的合法形态):两个 AS 表项占 [0..2)。
        assert_eq!(
            plan_set0_layout_with_as(2, 1, 0, 0, false),
            vec![
                (0, DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR),
                (1, DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR),
                (2, DESCRIPTOR_TYPE_STORAGE_BUFFER),
            ]
        );
        // A=0 逐位等于旧布局(随机几组形状对拍)。
        for (n, m, k, u) in [(0u32, 0u32, 0u32, false), (2, 1, 1, true), (3, 0, 2, false)] {
            assert_eq!(
                plan_set0_layout_with_as(0, n, m, k, u),
                plan_set0_layout(n, m, k, u),
                "A=0 须逐位等于旧布局({n},{m},{k},{u})"
            );
        }
        // AS descriptor 类型枚举锚(SDK 1.3.296 vulkan_core.h;与 vk.rs 同源同值)。
        assert_eq!(DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR, 1_000_150_000);
        assert_eq!(
            ST_WRITE_DESCRIPTOR_SET_ACCELERATION_STRUCTURE_KHR,
            1_000_150_007
        );
    }

    #[test]
    fn provenance_as_read_and_red_rejection() {
        // AS 表项 provenance:StableResourceId = resource_count + as_index + 1;ray query
        // pass 对 TLAS 为 Read,producer 指向 AS 代表 allocation(TLAS storage)。
        let pass = Pass::Compute(ComputePass {
            name: "rq",
            spirv: &[],
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([1, 1, 1]),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![0],
                ..Bindings::default()
            },
        });
        let declared: Vec<Bindings> = std::iter::once(pass_bindings(&pass).clone()).collect();
        // 扩展表:1 资源 + 1 AS 槽;AS 代表 allocation = StableAllocationId(77)。
        let allocations = [StableAllocationId(11), StableAllocationId(77)];
        let generations = [3u64, 5u64];
        let expected = build_runtime_provenance_ext(
            std::slice::from_ref(&pass),
            &declared,
            &allocations,
            &generations,
            8,
            1,
        );
        let as_entry = expected.passes[0]
            .resources
            .iter()
            .find(|r| r.resource_id == StableResourceId(2))
            .expect("AS provenance 项(resource 1 + AS 槽 = id 2)");
        assert_eq!(as_entry.access, RuntimeAccessKind::Read);
        assert_eq!(as_entry.allocation_id, StableAllocationId(77));
        assert_eq!(
            as_entry.producer.expect("Read 有 producer"),
            ProducerVersion {
                allocation_id: StableAllocationId(77),
                generation: 5,
            }
        );
        assert_eq!(as_entry.produced_generation, None, "纯读不产 generation");
        validate_submission_provenance(&expected, &expected).expect("actual provenance 应通过");
        // RED:AS 资源 stale generation → submit 前确定性拒。
        let mut stale = expected.clone();
        stale.passes[0]
            .resources
            .iter_mut()
            .find(|r| r.resource_id == StableResourceId(2))
            .expect("AS 项")
            .producer
            .as_mut()
            .expect("producer")
            .generation = 4;
        assert!(
            validate_submission_provenance(&expected, &stale)
                .unwrap_err()
                .contains("fail-closed")
        );
        // RED:AS 资源错 allocation → submit 前确定性拒。
        let mut wrong = expected.clone();
        wrong.passes[0]
            .resources
            .iter_mut()
            .find(|r| r.resource_id == StableResourceId(2))
            .expect("AS 项")
            .producer
            .as_mut()
            .expect("producer")
            .allocation_id = StableAllocationId(999);
        assert!(
            validate_submission_provenance(&expected, &wrong)
                .unwrap_err()
                .contains("allocation/generation")
        );
    }

    /// 供 FrameUpdate 校验测试的最小帧:2 storage buffer + 1 compute pass
    /// (storage [0] + push 4B)+ 1 buffer readback。
    fn frame_update_test_frame() -> TestFrame {
        let spv = sample_compute_spv_words();
        let spv_bytes: Vec<u8> = spv.iter().flat_map(|w| w.to_le_bytes()).collect();
        let leaked: &'static [u8] = Box::leak(spv_bytes.into_boxed_slice());
        let mkbuf = || {
            ResourceDesc::Buffer(BufferDesc {
                size: 32,
                usage: BufferUsage {
                    storage: true,
                    ..BufferUsage::default()
                },
                data: None,
                device_local: false,
            })
        };
        let resources = vec![mkbuf(), mkbuf()];
        let passes = vec![Pass::Compute(ComputePass {
            name: "c0",
            spirv: leaked,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([8, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![0],
                push_constants: 100u32.to_le_bytes().to_vec(),
                ..Bindings::default()
            },
        })];
        let barriers = vec![vec![(0u32, TargetState::StorageWrite)]];
        let readbacks = vec![Readback::Buffer {
            res: 0,
            offset: 0,
            size: 32,
        }];
        (resources, passes, barriers, readbacks)
    }

    #[test]
    fn validate_frame_update_rules() {
        let (r, p, _b, rb) = frame_update_test_frame();
        // 空 update 合法;合法组合(tlas_update 下标 0 / 上传 / 同构 binding 覆盖 /
        // push 覆盖 / readback 子集)合法。
        validate_frame_update(&r, &p, &rb, 1, &FrameUpdate::default()).expect("空 update 合法");
        let ok = FrameUpdate {
            tlas_update: Some((0, Vec::new(), TlasBuildAction::Refit)),
            buffer_uploads: vec![(StableResourceId(1), 4, vec![0xAB; 8])],
            binding_overrides: vec![(
                0,
                Bindings {
                    storage_buffers: vec![1],
                    push_constants: 100u32.to_le_bytes().to_vec(),
                    ..Bindings::default()
                },
            )],
            push_constant_overrides: vec![(0, 200u32.to_le_bytes().to_vec())],
            readback_subset: Some(vec![0]),
            blas_refit: None,
        };
        validate_frame_update(&r, &p, &rb, 1, &ok).expect("合法组合 update");
        // tlas_update 下标越界 / 无 AS 表。
        let bad = FrameUpdate {
            tlas_update: Some((1, Vec::new(), TlasBuildAction::Refit)),
            ..FrameUpdate::default()
        };
        assert!(
            validate_frame_update(&r, &p, &rb, 1, &bad)
                .unwrap_err()
                .contains("越界")
        );
        assert!(validate_frame_update(&r, &p, &rb, 0, &bad).is_err());
        // 上传:id 0 / 越界 id / 空段 / 越界区段。
        for (id, off, bytes) in [
            (0u64, 0u64, vec![1u8]),
            (3, 0, vec![1u8]),
            (1, 0, Vec::new()),
            (1, 30, vec![1u8, 2, 3]),
        ] {
            let bad = FrameUpdate {
                buffer_uploads: vec![(StableResourceId(id), off, bytes)],
                ..FrameUpdate::default()
            };
            assert!(
                validate_frame_update(&r, &p, &rb, 1, &bad).is_err(),
                "上传非法须拒(id={id}, off={off})"
            );
        }
        // binding 覆盖:布局键漂移(计数变)/ push 长度漂移 / pass 越界 / 重复覆盖。
        let drift = FrameUpdate {
            binding_overrides: vec![(0, Bindings::default())],
            ..FrameUpdate::default()
        };
        assert!(
            validate_frame_update(&r, &p, &rb, 1, &drift)
                .unwrap_err()
                .contains("布局键")
        );
        let pc_drift = FrameUpdate {
            binding_overrides: vec![(
                0,
                Bindings {
                    storage_buffers: vec![0],
                    push_constants: vec![0; 8],
                    ..Bindings::default()
                },
            )],
            ..FrameUpdate::default()
        };
        assert!(
            validate_frame_update(&r, &p, &rb, 1, &pc_drift)
                .unwrap_err()
                .contains("push constants")
        );
        let oob = FrameUpdate {
            binding_overrides: vec![(1, Bindings::default())],
            ..FrameUpdate::default()
        };
        assert!(validate_frame_update(&r, &p, &rb, 1, &oob).is_err());
        let dup = FrameUpdate {
            binding_overrides: vec![
                (
                    0,
                    Bindings {
                        storage_buffers: vec![1],
                        push_constants: 100u32.to_le_bytes().to_vec(),
                        ..Bindings::default()
                    },
                ),
                (
                    0,
                    Bindings {
                        storage_buffers: vec![1],
                        push_constants: 100u32.to_le_bytes().to_vec(),
                        ..Bindings::default()
                    },
                ),
            ],
            ..FrameUpdate::default()
        };
        assert!(
            validate_frame_update(&r, &p, &rb, 1, &dup)
                .unwrap_err()
                .contains("重复")
        );
        // binding 覆盖带越界 AS 下标。
        let as_oob = FrameUpdate {
            binding_overrides: vec![(
                0,
                Bindings {
                    accel_structs: vec![1],
                    ..Bindings::default()
                },
            )],
            ..FrameUpdate::default()
        };
        assert!(validate_frame_update(&r, &p, &rb, 1, &as_oob).is_err());
        // push 覆盖:长度不等 / pass 越界。
        let pc_bad = FrameUpdate {
            push_constant_overrides: vec![(0, vec![0; 8])],
            ..FrameUpdate::default()
        };
        assert!(
            validate_frame_update(&r, &p, &rb, 1, &pc_bad)
                .unwrap_err()
                .contains("长度")
        );
        let pc_oob = FrameUpdate {
            push_constant_overrides: vec![(9, vec![0; 4])],
            ..FrameUpdate::default()
        };
        assert!(validate_frame_update(&r, &p, &rb, 1, &pc_oob).is_err());
        // readback 子集:越界 / 重复。
        let rb_oob = FrameUpdate {
            readback_subset: Some(vec![1]),
            ..FrameUpdate::default()
        };
        assert!(validate_frame_update(&r, &p, &rb, 1, &rb_oob).is_err());
        let rb_dup = FrameUpdate {
            readback_subset: Some(vec![0, 0]),
            ..FrameUpdate::default()
        };
        assert!(
            validate_frame_update(&r, &p, &rb, 1, &rb_dup)
                .unwrap_err()
                .contains("重复")
        );
    }

    #[test]
    fn validate_rejects_accel_structs_without_as_table() {
        // `execute_frame` / 空 AS 表 session(as_count=0)带 accel_structs 绑定 →
        // 确定性拒;as_count=1 时同形状合法。
        let (r, p, b, rb) = frame_update_test_frame();
        let brefs: Vec<&[(u32, TargetState)]> = b.iter().map(Vec::as_slice).collect();
        let mut p2 = p.clone();
        if let Pass::Compute(cp) = &mut p2[0] {
            cp.bindings.accel_structs = vec![0];
        }
        assert!(
            validate_frame(&r, &p2, &brefs, &rb)
                .unwrap_err()
                .contains("accel struct")
        );
        validate_frame_with_as(&r, &p2, &brefs, &rb, 1).expect("as_count=1 合法");
        // 越界 AS 下标(as_count=1 引 1)亦拒。
        let mut p3 = p.clone();
        if let Pass::Compute(cp) = &mut p3[0] {
            cp.bindings.accel_structs = vec![1];
        }
        assert!(validate_frame_with_as(&r, &p3, &brefs, &rb, 1).is_err());
    }

    #[test]
    fn state_fields_mapping_anchors() {
        // 单源映射锚定(SDK 头核对值):layout/stage2/access2。
        assert_eq!(
            state_fields(TargetState::ColorAttachmentWrite),
            (2, 0x400, 0x180)
        );
        assert_eq!(
            state_fields(TargetState::DepthAttachmentWrite),
            (3, 0x300, 0x600)
        );
        assert_eq!(state_fields(TargetState::ShaderRead), (5, 0x888, 0x20));
        assert_eq!(
            state_fields(TargetState::StorageImageReadWrite),
            (1, 0x888, 0x60)
        );
        assert_eq!(state_fields(TargetState::TransferSrc), (6, 0x1000, 0x800));
        assert_eq!(state_fields(TargetState::TransferDst), (7, 0x1000, 0x1000));
        assert_eq!(state_fields(TargetState::StorageWrite), (0, 0x888, 0x40));
        assert_eq!(state_fields(TargetState::UniformRead), (0, 0x888, 0x8));
        assert_eq!(state_fields(TargetState::VertexInput), (0, 0x4, 0x4));
        assert_eq!(state_fields(TargetState::IndirectRead), (0, 0x2, 0x1));
    }

    #[test]
    fn barrier_table_construction() {
        // 屏障表构造(隐式补全的事实源):UNDEFINED 初态 → 目标产转换项;已在目标态 → None。
        let init: TrackedState = (LAYOUT_UNDEFINED, STAGE2_NONE, 0);
        let f = barrier_fields(init, TargetState::ColorAttachmentWrite).expect("初态须产转换");
        assert_eq!(f, (0, 2, 0, 0, 0x400, 0x180));
        // 转换后跟踪态 = state_fields(目标),再转同态 → None(幂等去重)。
        let now = state_fields(TargetState::ColorAttachmentWrite);
        assert!(barrier_fields(now, TargetState::ColorAttachmentWrite).is_none());
        // attachment → shader read(RT→SRV 经典转换):src = CAO/CA_W|R,dst = 全 shader/SHADER_R。
        let f2 = barrier_fields(now, TargetState::ShaderRead).expect("CA→SR 须产转换");
        assert_eq!(f2.0, 2, "old layout = COLOR_ATTACHMENT_OPTIMAL");
        assert_eq!(f2.1, 5, "new layout = SHADER_READ_ONLY_OPTIMAL");
        assert_eq!(f2.2, 0x400);
        assert_eq!(f2.3, 0x180);
        assert_eq!(f2.4, 0x888);
        assert_eq!(f2.5, 0x20);
        // host 上传态 → transfer dst(上传段首转换)。
        let f3 = barrier_fields(init, TargetState::TransferDst).expect("上传转换");
        assert_eq!(f3, (0, 7, 0, 0, 0x1000, 0x1000));
        // host 写 buffer → storage 读写(compute 首用)。
        let host_buf: TrackedState = (0, STAGE2_HOST, ACCESS2_HOST_WRITE);
        let f4 = barrier_fields(host_buf, TargetState::StorageReadWrite).expect("host→ssbo");
        assert_eq!(f4.2, STAGE2_HOST);
        assert_eq!(f4.3, ACCESS2_HOST_WRITE);
        assert_eq!(f4.4, 0x888);
        assert_eq!(f4.5, 0x60);
    }

    /// 测试帧四元组类型(资源 / pass / 屏障计划 / readback;clippy type_complexity 因式分解)。
    type TestFrame = (
        Vec<ResourceDesc<'static>>,
        Vec<Pass<'static>>,
        Vec<Vec<(u32, TargetState)>>,
        Vec<Readback>,
    );

    /// 最小合法帧(三角形 raster → 纹理 readback)供校验 accept 用例。
    fn minimal_valid_frame() -> TestFrame {
        let spv = sample_compute_spv_words();
        let spv_bytes: Vec<u8> = spv.iter().flat_map(|w| w.to_le_bytes()).collect();
        let leaked: &'static [u8] = Box::leak(spv_bytes.into_boxed_slice());
        let resources = vec![
            ResourceDesc::Texture(TextureDesc {
                width: 64,
                height: 64,
                format: TexFormat::Rgba8Unorm,
                usage: TextureUsage {
                    sampled: false,
                    storage: false,
                    color: true,
                    depth: false,
                },
                data: None,
            }),
            ResourceDesc::Buffer(BufferDesc {
                size: 32,
                usage: BufferUsage {
                    storage: true,
                    uniform: false,
                    vertex: false,
                    indirect: false,
                },
                data: None,
                device_local: false,
            }),
        ];
        let passes = vec![Pass::Compute(ComputePass {
            name: "c0",
            spirv: leaked,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([8, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![1],
                ..Bindings::default()
            },
        })];
        let barriers = vec![vec![(1u32, TargetState::StorageWrite)]];
        let readbacks = vec![Readback::Buffer {
            res: 1,
            offset: 0,
            size: 32,
        }];
        (resources, passes, barriers, readbacks)
    }

    #[test]
    fn validate_accepts_minimal_frame() {
        let (r, p, b, rb) = minimal_valid_frame();
        let brefs: Vec<&[(u32, TargetState)]> = b.iter().map(Vec::as_slice).collect();
        validate_frame(&r, &p, &brefs, &rb).expect("最小合法帧应过校验");
    }

    #[test]
    fn validate_rejects_bad_frames() {
        let (r, p, b, rb) = minimal_valid_frame();
        let brefs: Vec<&[(u32, TargetState)]> = b.iter().map(Vec::as_slice).collect();
        // barriers 列数不符。
        assert!(
            validate_frame(&r, &p, &[], &rb)
                .unwrap_err()
                .contains("barriers")
        );
        // push constants 超 128B。
        let mut p2 = p.clone();
        if let Pass::Compute(cp) = &mut p2[0] {
            cp.bindings.push_constants = vec![0u8; 129];
        }
        assert!(
            validate_frame(&r, &p2, &brefs, &rb)
                .unwrap_err()
                .contains("push constants")
        );
        // SPIR-V magic 非法。
        let mut p3 = p.clone();
        if let Pass::Compute(cp) = &mut p3[0] {
            cp.spirv = &[0u8; 20];
        }
        assert!(
            validate_frame(&r, &p3, &brefs, &rb)
                .unwrap_err()
                .contains("magic")
        );
        // SPIR-V 长度非 4 整倍。
        let mut p4 = p.clone();
        if let Pass::Compute(cp) = &mut p4[0] {
            cp.spirv = &cp.spirv[..cp.spirv.len() - 1];
        }
        assert!(validate_frame(&r, &p4, &brefs, &rb).is_err());
        // 屏障资源号越界。
        let b_bad = [vec![(99u32, TargetState::StorageWrite)]];
        let b_bad_refs: Vec<&[(u32, TargetState)]> = b_bad.iter().map(Vec::as_slice).collect();
        assert!(
            validate_frame(&r, &p, &b_bad_refs, &rb)
                .unwrap_err()
                .contains("越界")
        );
        // 屏障类别失配(image 态落 buffer)。
        let b_bad2 = [vec![(1u32, TargetState::ColorAttachmentWrite)]];
        let b_bad2_refs: Vec<&[(u32, TargetState)]> = b_bad2.iter().map(Vec::as_slice).collect();
        assert!(
            validate_frame(&r, &p, &b_bad2_refs, &rb)
                .unwrap_err()
                .contains("image 类")
        );
        // readback 越界。
        let rb_bad = vec![Readback::Buffer {
            res: 1,
            offset: 16,
            size: 32,
        }];
        assert!(validate_frame(&r, &p, &brefs, &rb_bad).is_err());
        // sampled 绑到非 sampled 纹理。
        let mut p5 = p.clone();
        if let Pass::Compute(cp) = &mut p5[0] {
            cp.bindings.sampled_images = vec![0];
        }
        assert!(
            validate_frame(&r, &p5, &brefs, &rb)
                .unwrap_err()
                .contains("sampled")
        );
        // 空 passes。
        assert!(
            validate_frame(&r, &[], &[], &[])
                .unwrap_err()
                .contains("passes 为空")
        );
    }

    #[test]
    fn validate_depth_and_attachment_rules() {
        // depth 格式作 color attachment → 拒;depth 用途配非 depth 格式 → 拒。
        let spv = sample_compute_spv_words();
        let spv_bytes: Vec<u8> = spv.iter().flat_map(|w| w.to_le_bytes()).collect();
        let leaked: &'static [u8] = Box::leak(spv_bytes.into_boxed_slice());
        let bad_depth_color = vec![ResourceDesc::Texture(TextureDesc {
            width: 8,
            height: 8,
            format: TexFormat::Depth32Float,
            usage: TextureUsage {
                sampled: false,
                storage: false,
                color: true,
                depth: true,
            },
            data: None,
        })];
        let p = vec![Pass::Compute(ComputePass {
            name: "c",
            spirv: leaked,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([1, 1, 1]),
            bindings: Bindings::default(),
        })];
        let b: [Vec<(u32, TargetState)>; 1] = [Vec::new()];
        let brefs: Vec<&[(u32, TargetState)]> = b.iter().map(Vec::as_slice).collect();
        assert!(
            validate_frame(&bad_depth_color, &p, &brefs, &[])
                .unwrap_err()
                .contains("depth 格式")
        );
        let bad_usage = vec![ResourceDesc::Texture(TextureDesc {
            width: 8,
            height: 8,
            format: TexFormat::Rgba8Unorm,
            usage: TextureUsage {
                sampled: false,
                storage: true,
                color: false,
                depth: true,
            },
            data: None,
        })];
        assert!(
            validate_frame(&bad_usage, &p, &brefs, &[])
                .unwrap_err()
                .contains("depth 用途")
        );
        // 纹理初始数据长度不符。
        let bad_data = vec![ResourceDesc::Texture(TextureDesc {
            width: 8,
            height: 8,
            format: TexFormat::Rgba8Unorm,
            usage: TextureUsage {
                sampled: true,
                storage: false,
                color: false,
                depth: false,
            },
            data: Some(&[0u8; 100]),
        })];
        assert!(validate_frame(&bad_data, &p, &brefs, &[]).is_err());
    }

    #[test]
    fn tex_format_constants_anchor() {
        // Vulkan format 枚举值锚定(SDK 1.3.296 vulkan_core.h 核对)。
        assert_eq!(TexFormat::Rgba8Unorm.vk_format(), 37);
        assert_eq!(TexFormat::Rgba16Float.vk_format(), 97);
        assert_eq!(TexFormat::R32Uint.vk_format(), 98);
        assert_eq!(TexFormat::Rg32Uint.vk_format(), 101);
        assert_eq!(TexFormat::Depth32Float.vk_format(), 126);
        assert_eq!(TexFormat::Rgba8Unorm.bytes_per_texel(), 4);
        assert_eq!(TexFormat::Rgba16Float.bytes_per_texel(), 8);
        assert_eq!(TexFormat::Rg32Uint.bytes_per_texel(), 8);
        assert!(TexFormat::Depth32Float.is_depth());
        assert!(!TexFormat::Rgba8Unorm.is_depth());
        assert_eq!(TexFormat::Rgba8Unorm.aspect_mask(), IMAGE_ASPECT_COLOR);
        assert_eq!(TexFormat::Depth32Float.aspect_mask(), 0x2);
    }

    // ── 手编最小 SPIR-V 见证模块(vk.rs mesh_witness_fs_spv 先例同律;RFC-0016
    //    §9 Q-A R-6 测试见证极小模块例外:程序化生成,非手写二进制入仓) ──

    fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
        v.push(op | ((ops.len() as u32 + 1) << 16));
        v.extend_from_slice(ops);
    }

    /// 最小 compute 见证:`buf[gid.x] = gid.x + pc.add`(u32 runtime 数组 SSBO binding0 +
    /// push constant 块 {u32 add})。约 30 指令,SPIR-V 1.3(StorageBuffer 类免
    /// SPV_KHR_storage_buffer_storage_class 扩展声明),GLCompute LocalSize(1,1,1)。
    fn sample_compute_spv_words() -> Vec<u32> {
        let mut v = vec![0x0723_0203, 0x0001_0300, 0, 30, 0];
        inst(&mut v, 17, &[1]); // OpCapability Shader
        inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
        inst(&mut v, 15, &[5, 20, 0x6E69_616D, 0, 6]); // OpEntryPoint GLCompute %20 "main" %6
        inst(&mut v, 16, &[20, 17, 1, 1, 1]); // OpExecutionMode %20 LocalSize 1 1 1
        inst(&mut v, 71, &[6, 11, 28]); // OpDecorate %6 BuiltIn GlobalInvocationId
        inst(&mut v, 71, &[10, 34, 0]); // OpDecorate %10 DescriptorSet 0
        inst(&mut v, 71, &[10, 33, 0]); // OpDecorate %10 Binding 0
        inst(&mut v, 71, &[8, 2]); // OpDecorate %8 Block
        inst(&mut v, 72, &[8, 0, 35, 0]); // OpMemberDecorate %8 0 Offset 0
        inst(&mut v, 71, &[7, 6, 4]); // OpDecorate %7 ArrayStride 4
        inst(&mut v, 71, &[13, 2]); // OpDecorate %13 Block(pc)
        inst(&mut v, 72, &[13, 0, 35, 0]); // OpMemberDecorate %13 0 Offset 0
        inst(&mut v, 19, &[1]); // %1 = OpTypeVoid
        inst(&mut v, 33, &[2, 1]); // %2 = OpTypeFunction %1
        inst(&mut v, 21, &[3, 32, 0]); // %3 = OpTypeInt 32 0(u32)
        inst(&mut v, 23, &[4, 3, 3]); // %4 = OpTypeVector %3 3
        inst(&mut v, 32, &[5, 1, 4]); // %5 = OpTypePointer Input %4
        inst(&mut v, 59, &[5, 6, 1]); // %6 = OpVariable %5 Input(gid)
        inst(&mut v, 29, &[7, 3]); // %7 = OpTypeRuntimeArray %3
        inst(&mut v, 30, &[8, 7]); // %8 = OpTypeStruct %7
        inst(&mut v, 32, &[9, 12, 8]); // %9 = OpTypePointer StorageBuffer %8
        inst(&mut v, 59, &[9, 10, 12]); // %10 = OpVariable %9 StorageBuffer(buf)
        inst(&mut v, 32, &[11, 12, 3]); // %11 = OpTypePointer StorageBuffer %3
        inst(&mut v, 43, &[3, 12, 0]); // %12 = OpConstant %3 0
        inst(&mut v, 30, &[13, 3]); // %13 = OpTypeStruct %3(pc 块)
        inst(&mut v, 32, &[14, 9, 13]); // %14 = OpTypePointer PushConstant %13
        inst(&mut v, 59, &[14, 15, 9]); // %15 = OpVariable %14 PushConstant
        inst(&mut v, 32, &[16, 9, 3]); // %16 = OpTypePointer PushConstant %3
        inst(&mut v, 54, &[1, 20, 0, 2]); // %20 = OpFunction %1 None %2
        inst(&mut v, 248, &[21]); // %21 = OpLabel
        inst(&mut v, 61, &[4, 22, 6]); // %22 = OpLoad %4 %6
        inst(&mut v, 81, &[3, 23, 22, 0]); // %23 = OpCompositeExtract %3 %22 0
        inst(&mut v, 65, &[16, 24, 15, 12]); // %24 = OpAccessChain %16 %15 %12
        inst(&mut v, 61, &[3, 25, 24]); // %25 = OpLoad %3 %24(pc.add)
        inst(&mut v, 128, &[3, 26, 23, 25]); // %26 = OpIAdd %3 %23 %25
        inst(&mut v, 65, &[11, 27, 10, 12, 23]); // %27 = OpAccessChain %11 %10 %12 %23
        inst(&mut v, 62, &[27, 26]); // OpStore %27 %26
        inst(&mut v, 253, &[]); // OpReturn
        inst(&mut v, 56, &[]); // OpFunctionEnd
        v
    }

    /// 最小 compute 纹理取址见证:`out[0..4] = texelFetch(tex, (32,32), 0)` 四分量
    /// (f32 runtime 数组 SSBO binding0 + sampled image binding1,2D/f32/sampled=1)。
    /// SPIR-V 1.3(同 compute 见证的 StorageBuffer 类口径)。
    fn sample_fetch_spv_words() -> Vec<u32> {
        let mut v = vec![0x0723_0203, 0x0001_0300, 0, 40, 0];
        inst(&mut v, 17, &[1]); // OpCapability Shader
        inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
        inst(&mut v, 15, &[5, 30, 0x6E69_616D, 0]); // OpEntryPoint GLCompute %30 "main"
        inst(&mut v, 16, &[30, 17, 1, 1, 1]); // OpExecutionMode %30 LocalSize 1 1 1
        inst(&mut v, 71, &[10, 34, 0]); // OpDecorate %10 DescriptorSet 0
        inst(&mut v, 71, &[10, 33, 0]); // OpDecorate %10 Binding 0
        inst(&mut v, 71, &[16, 34, 0]); // OpDecorate %16 DescriptorSet 0
        inst(&mut v, 71, &[16, 33, 1]); // OpDecorate %16 Binding 1
        inst(&mut v, 71, &[8, 2]); // OpDecorate %8 Block
        inst(&mut v, 72, &[8, 0, 35, 0]); // OpMemberDecorate %8 0 Offset 0
        inst(&mut v, 71, &[7, 6, 4]); // OpDecorate %7 ArrayStride 4
        inst(&mut v, 19, &[1]); // %1 = OpTypeVoid
        inst(&mut v, 33, &[2, 1]); // %2 = OpTypeFunction %1
        inst(&mut v, 21, &[3, 32, 0]); // %3 = OpTypeInt 32 0(u32)
        inst(&mut v, 22, &[4, 32]); // %4 = OpTypeFloat 32
        inst(&mut v, 23, &[5, 4, 4]); // %5 = OpTypeVector %4 4
        inst(&mut v, 23, &[6, 3, 2]); // %6 = OpTypeVector %3 2
        inst(&mut v, 29, &[7, 4]); // %7 = OpTypeRuntimeArray %4
        inst(&mut v, 30, &[8, 7]); // %8 = OpTypeStruct %7
        inst(&mut v, 32, &[9, 12, 8]); // %9 = OpTypePointer StorageBuffer %8
        inst(&mut v, 59, &[9, 10, 12]); // %10 = OpVariable %9 StorageBuffer(out)
        inst(&mut v, 32, &[11, 12, 4]); // %11 = OpTypePointer StorageBuffer %4
        // %12 = OpTypeImage %4 2D(depth0,arrayed0,ms0,sampled1,Unknown)
        inst(&mut v, 25, &[12, 4, 1, 0, 0, 0, 1, 0]);
        inst(&mut v, 32, &[13, 0, 12]); // %13 = OpTypePointer UniformConstant %12
        inst(&mut v, 59, &[13, 16, 0]); // %16 = OpVariable %13 UniformConstant(tex)
        inst(&mut v, 43, &[3, 17, 32]); // %17 = OpConstant %3 32
        inst(&mut v, 43, &[3, 18, 0]); // %18 = OpConstant %3 0
        inst(&mut v, 44, &[6, 19, 17, 17]); // %19 = OpConstantComposite %6 (32,32)
        inst(&mut v, 43, &[3, 20, 1]); // %20 = OpConstant %3 1
        inst(&mut v, 43, &[3, 21, 2]); // %21 = OpConstant %3 2
        inst(&mut v, 43, &[3, 22, 3]); // %22 = OpConstant %3 3
        inst(&mut v, 54, &[1, 30, 0, 2]); // %30 = OpFunction %1 None %2
        inst(&mut v, 248, &[31]); // %31 = OpLabel
        inst(&mut v, 61, &[12, 32, 16]); // %32 = OpLoad %12 %16
        // %33 = OpImageFetch %5 %32 %19 Lod %18(image operands 掩码 Lod=0x2;0x1=Bias)
        inst(&mut v, 95, &[5, 33, 32, 19, 2, 18]);
        for (i, c) in [18u32, 20, 21, 22].iter().enumerate() {
            // %34+i = OpCompositeExtract %4 %33 i;经 OpAccessChain 存 out[i]。
            inst(&mut v, 81, &[4, 34 + i as u32, 33, i as u32]);
            inst(&mut v, 65, &[11, 24 + i as u32, 10, 18, *c]);
            inst(&mut v, 62, &[24 + i as u32, 34 + i as u32]);
        }
        inst(&mut v, 253, &[]); // OpReturn
        inst(&mut v, 56, &[]); // OpFunctionEnd
        v
    }

    /// rurixc 编译的真实 ray query compute 内核(G7.2 W3a 语料
    /// `conformance/rayquery/accept/ray_query_hit_miss.rx` 的 SPIR-V 1.4 产物,
    /// RXS-0297~0300 布局:set 0 / binding 0 = `AccelStruct`、binding 1 = SSBO,
    /// entry `rx_ray_query_hit_miss_8`;`vk_ray_query` harness 在本机 RTX 4070 Ti
    /// 真跑 hit/miss 双绿的同一份字节,逐字嵌入使测试自包含、不依赖 target/ 工件)。
    /// 语义:ray (0,0,0)+t·(0,0,1) 遍历 TLAS,has_committed → out[0] = t,否则 −1.0。
    const RAY_QUERY_HIT_MISS_SPV_WORDS: &[u32] = &[
        0x07230203, 0x00010400, 0x00000000, 0x0000004A, 0x00000000, 0x00020011, 0x00000001,
        0x00020011, 0x00001178, 0x0006000A, 0x5F565053, 0x5F52484B, 0x5F796172, 0x72657571,
        0x00000079, 0x0003000E, 0x00000000, 0x00000001, 0x000C000F, 0x00000005, 0x00000001,
        0x725F7872, 0x715F7961, 0x79726575, 0x7469685F, 0x73696D5F, 0x00385F73, 0x00000022,
        0x00000004, 0x00000009, 0x00060010, 0x00000001, 0x00000011, 0x00000001, 0x00000001,
        0x00000001, 0x00040047, 0x00000004, 0x00000022, 0x00000000, 0x00040047, 0x00000004,
        0x00000021, 0x00000000, 0x00040047, 0x00000006, 0x00000006, 0x00000004, 0x00050048,
        0x00000007, 0x00000000, 0x00000023, 0x00000000, 0x00030047, 0x00000007, 0x00000002,
        0x00040047, 0x00000009, 0x00000022, 0x00000000, 0x00040047, 0x00000009, 0x00000021,
        0x00000001, 0x00040047, 0x00000022, 0x0000000B, 0x0000001C, 0x000214DD, 0x00000002,
        0x00040020, 0x00000003, 0x00000000, 0x00000002, 0x0004003B, 0x00000003, 0x00000004,
        0x00000000, 0x00030016, 0x00000005, 0x00000020, 0x0003001D, 0x00000006, 0x00000005,
        0x0003001E, 0x00000007, 0x00000006, 0x00040020, 0x00000008, 0x0000000C, 0x00000007,
        0x0004003B, 0x00000008, 0x00000009, 0x0000000C, 0x00040015, 0x00000013, 0x00000020,
        0x00000000, 0x00040020, 0x00000014, 0x00000007, 0x00000013, 0x00040017, 0x00000017,
        0x00000005, 0x00000003, 0x00040020, 0x00000018, 0x00000007, 0x00000017, 0x00040020,
        0x0000001D, 0x00000007, 0x00000005, 0x00040017, 0x00000020, 0x00000013, 0x00000003,
        0x00040020, 0x00000021, 0x00000001, 0x00000020, 0x0004003B, 0x00000021, 0x00000022,
        0x00000001, 0x0004002B, 0x00000005, 0x00000027, 0x00000000, 0x0004002B, 0x00000005,
        0x00000029, 0x3F800000, 0x00021178, 0x0000002A, 0x00040020, 0x0000002B, 0x00000007,
        0x0000002A, 0x0004002B, 0x00000005, 0x00000030, 0x42C80000, 0x0004002B, 0x00000013,
        0x00000031, 0x00000001, 0x0004002B, 0x00000013, 0x00000032, 0x000000FF, 0x00020014,
        0x00000033, 0x0004002B, 0x00000013, 0x00000035, 0x00000000, 0x00040020, 0x00000040,
        0x0000000C, 0x00000005, 0x00020013, 0x00000047, 0x00030021, 0x00000048, 0x00000047,
        0x00050036, 0x00000047, 0x00000001, 0x00000000, 0x00000048, 0x000200F8, 0x00000049,
        0x0004003B, 0x00000014, 0x00000015, 0x00000007, 0x0004003B, 0x00000014, 0x00000016,
        0x00000007, 0x0004003B, 0x00000018, 0x00000019, 0x00000007, 0x0004003B, 0x00000018,
        0x0000001A, 0x00000007, 0x0004003B, 0x00000014, 0x0000001B, 0x00000007, 0x0004003B,
        0x00000014, 0x0000001C, 0x00000007, 0x0004003B, 0x0000001D, 0x0000001E, 0x00000007,
        0x0004003B, 0x0000001D, 0x0000001F, 0x00000007, 0x0004003B, 0x0000002B, 0x0000002C,
        0x00000007, 0x000200F9, 0x0000000A, 0x000200F8, 0x0000000A, 0x0004003D, 0x00000020,
        0x00000023, 0x00000022, 0x00050051, 0x00000013, 0x00000024, 0x00000023, 0x00000000,
        0x0003003E, 0x00000016, 0x00000024, 0x000200F9, 0x0000000B, 0x000200F8, 0x0000000B,
        0x0004003D, 0x00000013, 0x00000025, 0x00000016, 0x0003003E, 0x00000015, 0x00000025,
        0x00060050, 0x00000017, 0x00000026, 0x00000027, 0x00000027, 0x00000027, 0x0003003E,
        0x00000019, 0x00000026, 0x00060050, 0x00000017, 0x00000028, 0x00000027, 0x00000027,
        0x00000029, 0x0003003E, 0x0000001A, 0x00000028, 0x0004003D, 0x00000002, 0x0000002D,
        0x00000004, 0x0004003D, 0x00000017, 0x0000002E, 0x00000019, 0x0004003D, 0x00000017,
        0x0000002F, 0x0000001A, 0x00091179, 0x0000002C, 0x0000002D, 0x00000031, 0x00000032,
        0x0000002E, 0x00000027, 0x0000002F, 0x00000030, 0x000200F9, 0x0000000C, 0x000200F8,
        0x0000000C, 0x0004117D, 0x00000033, 0x00000034, 0x0000002C, 0x000600A9, 0x00000013,
        0x00000036, 0x00000034, 0x00000031, 0x00000035, 0x0003003E, 0x0000001B, 0x00000036,
        0x0004003D, 0x00000013, 0x00000037, 0x0000001B, 0x000500AB, 0x00000033, 0x00000038,
        0x00000037, 0x00000035, 0x000400F6, 0x0000000E, 0x00000012, 0x00000000, 0x000400FA,
        0x00000038, 0x0000000D, 0x0000000E, 0x000200F8, 0x0000000D, 0x000200F9, 0x00000012,
        0x000200F8, 0x00000012, 0x000200F9, 0x0000000C, 0x000200F8, 0x0000000E, 0x0005117F,
        0x00000013, 0x0000003A, 0x0000002C, 0x00000031, 0x000500AB, 0x00000033, 0x00000039,
        0x0000003A, 0x00000035, 0x000600A9, 0x00000013, 0x0000003B, 0x00000039, 0x00000031,
        0x00000035, 0x0003003E, 0x0000001C, 0x0000003B, 0x0004003D, 0x00000013, 0x0000003C,
        0x0000001C, 0x000500AB, 0x00000033, 0x0000003D, 0x0000003C, 0x00000035, 0x000300F7,
        0x00000011, 0x00000000, 0x000400FA, 0x0000003D, 0x0000000F, 0x00000010, 0x000200F8,
        0x0000000F, 0x00051782, 0x00000005, 0x0000003E, 0x0000002C, 0x00000031, 0x0003003E,
        0x0000001E, 0x0000003E, 0x0004003D, 0x00000005, 0x0000003F, 0x0000001E, 0x0004003D,
        0x00000013, 0x00000041, 0x00000015, 0x00060041, 0x00000040, 0x00000042, 0x00000009,
        0x00000035, 0x00000041, 0x0003003E, 0x00000042, 0x0000003F, 0x000200F9, 0x00000011,
        0x000200F8, 0x00000010, 0x00050083, 0x00000005, 0x00000043, 0x00000027, 0x00000029,
        0x0003003E, 0x0000001F, 0x00000043, 0x0004003D, 0x00000005, 0x00000044, 0x0000001F,
        0x0004003D, 0x00000013, 0x00000045, 0x00000015, 0x00060041, 0x00000040, 0x00000046,
        0x00000009, 0x00000035, 0x00000045, 0x0003003E, 0x00000046, 0x00000044, 0x000200F9,
        0x00000011, 0x000200F8, 0x00000011, 0x000100FD, 0x00010038,
    ];

    fn spv_bytes(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn hand_assembled_spv_wellformed() {
        // 手编见证模块的结构自检:entry 名可解析、magic 过校验、无截断指令。
        let w = sample_compute_spv_words();
        assert_eq!(w[0], 0x0723_0203);
        assert_eq!(crate::vk::entry_point_name(&w).as_deref(), Some("main"));
        spirv_to_words(&spv_bytes(&w), "compute 见证").expect("compute 见证过校验");
        let wf = sample_fetch_spv_words();
        assert_eq!(crate::vk::entry_point_name(&wf).as_deref(), Some("main"));
        spirv_to_words(&spv_bytes(&wf), "fetch 见证").expect("fetch 见证过校验");
        // 指令流扫描:wordCount 不越界(截断即红)。
        for words in [&w, &wf] {
            let mut i = 5usize;
            while i < words.len() {
                let wc = (words[i] >> 16) as usize;
                assert!(wc > 0 && i + wc <= words.len(), "指令截断 @word {i}");
                i += wc;
            }
            assert_eq!(i, words.len(), "指令流须恰好终了");
        }
    }

    // ── ②③④ device 侧(feature vulkan + 运行时探测,不可用 → SKIP 非 fake) ──

    /// 居中三角形顶点字节(pos vec4 @0 + color vec4 @16,stride 32;vk_triangle 先例同律)。
    fn triangle_vertices() -> Vec<u8> {
        let mut v = Vec::with_capacity(3 * 32);
        let mut push = |vals: [f32; 4]| {
            for f in vals {
                v.extend_from_slice(&f.to_le_bytes());
            }
        };
        push([0.0, 0.7, 0.0, 1.0]); // v0 pos(上)
        push([1.0, 0.0, 0.0, 1.0]); // v0 color R
        push([-0.7, -0.7, 0.0, 1.0]); // v1 pos(左下)
        push([0.0, 1.0, 0.0, 1.0]); // v1 color G
        push([0.7, -0.7, 0.0, 1.0]); // v2 pos(右下)
        push([0.0, 0.0, 1.0, 1.0]); // v2 color B
        v
    }

    const FORMAT_R32G32B32A32_SFLOAT: u32 = 109;
    const TRI_ATTRS: [(u32, u32, u32); 2] = [
        (0, FORMAT_R32G32B32A32_SFLOAT, 0),
        (1, FORMAT_R32G32B32A32_SFLOAT, 16),
    ];

    /// device 可用性门(vulkan loader + build.rs demo 着色器均须在;否则 SKIP 三态)。
    fn device_gate() -> Option<(&'static [u8], &'static [u8])> {
        if !crate::vk::vulkan_available() {
            eprintln!("[render_exec] SKIP: vulkan loader 不可用(dev-env degrade)");
            return None;
        }
        let (vs, fs, _saxpy) = crate::vk::demo_shaders_spv();
        if vs.is_empty() || fs.is_empty() {
            eprintln!("[render_exec] SKIP: build.rs 未产 tri_vs/tri_fs.spv(codegen 降级)");
            return None;
        }
        Some((vs, fs))
    }

    #[test]
    fn device_caps_probe() {
        if !crate::vk::vulkan_available() {
            eprintln!("[render_exec] SKIP: vulkan loader 不可用(caps probe)");
            return;
        }
        let caps = probe_device_caps().expect("caps 探测应成功(loader 在)");
        assert!(!caps.device_name.is_empty(), "设备名非空");
        assert!(caps.max_push_constants_size >= 128, "Vulkan 保底 128B push");
        eprintln!(
            "[render_exec] caps: device=`{}` sync2={} atomic_int64={} max_pc={}",
            caps.device_name,
            caps.synchronization2,
            caps.shader_buffer_int64_atomics,
            caps.max_push_constants_size
        );
    }

    #[test]
    fn device_ray_query_chain_caps_probe() {
        if !crate::vk::vulkan_available() {
            eprintln!("[render_exec] SKIP: vulkan loader 不可用(ray-query caps probe)");
            return;
        }
        let caps = probe_device_caps().expect("ray-query 能力链探测应成功(loader 在)");
        let chain = [
            ("ray_query", caps.ray_query),
            ("acceleration_structure", caps.acceleration_structure),
            ("buffer_device_address", caps.buffer_device_address),
            ("descriptor_indexing", caps.descriptor_indexing),
            ("deferred_host_operations", caps.deferred_host_operations),
        ];
        assert_eq!(chain.len(), 5, "W3 ray-query 能力链字段须完整");
        assert!(chain.iter().all(|(name, _)| !name.is_empty()));
        eprintln!(
            "[render_exec] ray-query caps: device=`{}` {}",
            caps.device_name,
            chain
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    /// ② 三角形真 draw:64x64 Rgba8 target,清黑,居中三角形,readback 断言
    /// 角 = 清色 / 中心覆盖非清色 / 覆盖计数 > 0(vk_triangle 判据同律)。
    #[test]
    fn device_triangle_draw_readback() {
        let Some((vs, fs)) = device_gate() else {
            return;
        };
        let verts = triangle_vertices();
        let resources = vec![ResourceDesc::Texture(TextureDesc {
            width: 64,
            height: 64,
            format: TexFormat::Rgba8Unorm,
            usage: TextureUsage {
                sampled: false,
                storage: false,
                color: true,
                depth: false,
            },
            data: None,
        })];
        let passes = vec![Pass::Raster(RasterPass {
            name: "tri",
            vs_spirv: vs,
            fs_spirv: fs,
            vertex: VertexData::Inline {
                data: &verts,
                stride: 32,
                attrs: &TRI_ATTRS,
            },
            draw: DrawSpec::Direct {
                vertex_count: 3,
                instance_count: 1,
                first_vertex: 0,
                first_instance: 0,
            },
            colors: vec![ColorAttachmentRef {
                res: 0,
                clear: Some([0.0, 0.0, 0.0, 1.0]),
            }],
            depth: None,
            viewport: None,
            bindings: Bindings::default(),
            conservative: None,
        })];
        let plan: Vec<Vec<(u32, TargetState)>> = vec![vec![(0, TargetState::ColorAttachmentWrite)]];
        let brefs: Vec<&[(u32, TargetState)]> = plan.iter().map(Vec::as_slice).collect();
        let readbacks = vec![Readback::Texture { res: 0 }];
        let out =
            execute_frame(&resources, &passes, &brefs, &readbacks).expect("三角形帧应执行成功");
        assert_eq!(out.len(), 1);
        let px = &out[0];
        assert_eq!(px.len(), 64 * 64 * 4, "紧凑 RGBA8 回读");
        let at = |x: u32, y: u32| -> (u8, u8, u8, u8) {
            let o = ((y * 64 + x) * 4) as usize;
            (px[o], px[o + 1], px[o + 2], px[o + 3])
        };
        let is_bg = |p: (u8, u8, u8, u8)| p.0 == 0 && p.1 == 0 && p.2 == 0;
        let corner = at(0, 63);
        assert!(
            is_bg(corner) && corner.3 == 255,
            "角须清色黑(A=255),实得 {corner:?}"
        );
        let center = at(32, 32);
        assert!(!is_bg(center), "中心须被三角形覆盖,实得 {center:?}");
        let covered = (0..64)
            .flat_map(|y| (0..64).map(move |x| at(x, y)))
            .filter(|&p| !is_bg(p))
            .count();
        assert!(covered > 0, "覆盖计数须 >0");
        eprintln!("[render_exec] ② 三角形真 draw: covered={covered} center={center:?}");
    }

    /// ③ compute pass 写 buffer readback 断言:`buf[i] = i + pc.add`(dispatch [8,1,1])。
    #[test]
    fn device_compute_write_buffer() {
        if !crate::vk::vulkan_available() {
            eprintln!("[render_exec] SKIP: vulkan loader 不可用(compute 写 buffer)");
            return;
        }
        let spv = spv_bytes(&sample_compute_spv_words());
        let resources = vec![ResourceDesc::Buffer(BufferDesc {
            size: 32,
            usage: BufferUsage {
                storage: true,
                uniform: false,
                vertex: false,
                indirect: false,
            },
            data: Some(&[0u8; 32]),
            device_local: false,
        })];
        let mut pc = Vec::new();
        pc.extend_from_slice(&100u32.to_le_bytes());
        let passes = vec![Pass::Compute(ComputePass {
            name: "c0",
            spirv: &spv,
            entry: None, // 自 OpEntryPoint 解析("main")
            dispatch: DispatchSpec::Direct([8, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![0],
                push_constants: pc,
                ..Bindings::default()
            },
        })];
        let plan: Vec<Vec<(u32, TargetState)>> = vec![vec![(0, TargetState::StorageWrite)]];
        let brefs: Vec<&[(u32, TargetState)]> = plan.iter().map(Vec::as_slice).collect();
        let readbacks = vec![Readback::Buffer {
            res: 0,
            offset: 0,
            size: 32,
        }];
        let out =
            execute_frame(&resources, &passes, &brefs, &readbacks).expect("compute 帧应执行成功");
        let words: Vec<u32> = out[0]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        for (i, &w) in words.iter().enumerate() {
            assert_eq!(w, i as u32 + 100, "buf[{i}] = i+100");
        }
        eprintln!("[render_exec] ③ compute 写 buffer: {words:?}");
    }

    /// ③b 持久 session 连续两帧：同 allocation/pipeline/command 保持、fence slot 提交，
    /// mandatory timestamp/memory-budget/ledger/provenance 均为实值。
    #[test]
    fn device_persistent_session_two_frames_real_telemetry() {
        if !crate::vk::vulkan_available() {
            eprintln!("[render_exec] SKIP: vulkan loader 不可用(persistent session)");
            return;
        }
        let spv = spv_bytes(&sample_compute_spv_words());
        let initial = [0u8; 32];
        let resources = [ResourceDesc::Buffer(BufferDesc {
            size: 32,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: Some(&initial),
            device_local: false,
        })];
        let passes = [Pass::Compute(ComputePass {
            name: "persistent-c0",
            spirv: &spv,
            entry: None,
            dispatch: DispatchSpec::Direct([8, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![0],
                push_constants: 100u32.to_le_bytes().to_vec(),
                ..Bindings::default()
            },
        })];
        let plan = [[(0, TargetState::StorageWrite)]];
        let barriers: [&[(u32, TargetState)]; 1] = [&plan[0]];
        let readbacks = [Readback::Buffer {
            res: 0,
            offset: 0,
            size: 32,
        }];
        let mut session = DeviceFrameSession::new(&resources, &passes, &barriers, &readbacks, 2)
            .expect("persistent session 应创建成功");
        let first = session.execute().expect("persistent frame 1");
        let second = session.execute().expect("persistent frame 2");
        assert_eq!(first.readbacks, second.readbacks);
        assert_eq!(first.telemetry.allocations, second.telemetry.allocations);
        assert_eq!(first.provenance.frame_generation, 1);
        assert_eq!(second.provenance.frame_generation, 2);
        assert_eq!(first.telemetry.passes.len(), 1);
        assert!(first.telemetry.timestamp_period_ns > 0.0);
        assert!(!first.telemetry.heaps.is_empty());
        assert_eq!(first.telemetry.leaked_object_count, 0);
        assert_eq!(first.telemetry.leaked_allocation_count, 0);
    }

    /// ③c G7.6 Wave B:AS session 两帧——ray query compute pass(TLAS binding 0)+
    /// buffer compute pass 串联;帧 1 重放(hit t=1.0),帧 2 FrameUpdate(TLAS refit
    /// x+10 平移 → miss −1.0,数据流证 refit 真实生效;buffer 上传 + push 覆盖 +
    /// readback 子集),再帧 3 RED(stale AS generation submit 前拒)+ 净帧复跑。
    #[test]
    fn device_ray_query_session_frame_update() {
        if !crate::vk::vulkan_available() {
            eprintln!("[render_exec] SKIP: vulkan loader 不可用(ray query session)");
            return;
        }
        let caps = probe_device_caps().expect("caps 探测应成功(loader 在)");
        let chain_ok = caps.ray_query
            && caps.acceleration_structure
            && caps.buffer_device_address
            && caps.deferred_host_operations;
        if !chain_ok {
            eprintln!(
                "[render_exec] SKIP: ray query 四件链不全(rq={} as={} bda={} dho={})",
                caps.ray_query,
                caps.acceleration_structure,
                caps.buffer_device_address,
                caps.deferred_host_operations
            );
            return;
        }
        let rq_spv = spv_bytes(RAY_QUERY_HIT_MISS_SPV_WORDS);
        assert_eq!(
            crate::vk::entry_point_name(RAY_QUERY_HIT_MISS_SPV_WORDS).as_deref(),
            Some("rx_ray_query_hit_miss_8")
        );
        let c_spv = spv_bytes(&sample_compute_spv_words());
        // hit 场景:三角形位于 z=1 平面,xy 投影覆盖原点(vk_ray_query harness TRI_HIT 同形)。
        let tri: [f32; 9] = [0.0, 0.6, 1.0, -0.6, -0.6, 1.0, 0.6, -0.6, 1.0];
        let tris: [&[f32]; 1] = [&tri[..]];
        let instances = [RayQueryInstanceDesc {
            blas: 0,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
        }];
        let zero4 = [0u8; 4];
        let zero32 = [0u8; 32];
        let resources = [
            ResourceDesc::Buffer(BufferDesc {
                size: 4,
                usage: BufferUsage {
                    storage: true,
                    ..BufferUsage::default()
                },
                data: Some(&zero4),
                device_local: false,
            }),
            ResourceDesc::Buffer(BufferDesc {
                size: 32,
                usage: BufferUsage {
                    storage: true,
                    ..BufferUsage::default()
                },
                data: Some(&zero32),
                device_local: false,
            }),
        ];
        let passes = [
            Pass::Compute(ComputePass {
                name: "rq",
                spirv: &rq_spv,
                entry: None,
                dispatch: DispatchSpec::Direct([1, 1, 1]),
                bindings: Bindings {
                    accel_structs: vec![0],
                    storage_buffers: vec![0],
                    ..Bindings::default()
                },
            }),
            Pass::Compute(ComputePass {
                name: "c0",
                spirv: &c_spv,
                entry: None,
                dispatch: DispatchSpec::Direct([8, 1, 1]),
                bindings: Bindings {
                    storage_buffers: vec![1],
                    push_constants: 100u32.to_le_bytes().to_vec(),
                    ..Bindings::default()
                },
            }),
        ];
        let plan = [
            [(0u32, TargetState::StorageReadWrite)],
            [(1u32, TargetState::StorageWrite)],
        ];
        let barriers: [&[(u32, TargetState)]; 2] = [&plan[0], &plan[1]];
        let readbacks = [
            Readback::Buffer {
                res: 0,
                offset: 0,
                size: 4,
            },
            Readback::Buffer {
                res: 1,
                offset: 0,
                size: 32,
            },
        ];
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &tris,
                instances: &instances,
            },
            transforms: None,
            // G31+ 波 B Task B5 字段面:本 fixture 无顶点可更新 BLAS(0-byte)。
            updatable_blas: &[],
        }];
        let mut session = DeviceFrameSession::new_with_accel_structs(
            &resources,
            &passes,
            &barriers,
            &readbacks,
            2,
            &accel_structs,
        )
        .expect("AS session 应创建成功(ray query 链在)");

        // ── 帧 1:重放路(无 update;hit → out[0] = t = 1.0)──
        let f1 = session.execute().expect("frame 1");
        let out1 = f32::from_le_bytes(f1.readbacks[0][..4].try_into().expect("4B"));
        assert_eq!(
            out1, 1.0,
            "帧 1 ray query hit: out[0] = t = 1.0,实得 {out1}"
        );
        let w1: Vec<u32> = f1.readbacks[1]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        for (i, &w) in w1.iter().enumerate() {
            assert_eq!(w, i as u32 + 100, "帧 1 buf[{i}] = i+100");
        }
        assert_eq!(f1.provenance.frame_generation, 1);
        assert_eq!(f1.telemetry.passes.len(), 2);
        assert!(
            f1.telemetry.passes.iter().all(|p| p.gpu_ns > 0.0),
            "帧 1 两 pass gpu_ns 均 > 0(非退化): {:?}",
            f1.telemetry.passes
        );
        assert_eq!(f1.telemetry.validation_error_count, 0);
        assert_eq!(f1.telemetry.leaked_object_count, 0);
        assert_eq!(f1.telemetry.leaked_allocation_count, 0);
        // AS provenance:rq pass 的 AS 项 = StableResourceId(3)(2 资源后接 AS 槽),
        // Read / producer gen 0(初始 build 代);AS 分配入 ledger 且 bytes > 0。
        let as1 = f1.provenance.passes[0]
            .resources
            .iter()
            .find(|r| r.resource_id == StableResourceId(3))
            .expect("帧 1 AS provenance 项");
        assert_eq!(as1.access, RuntimeAccessKind::Read);
        assert_eq!(
            as1.producer.expect("Read 有 producer").generation,
            0,
            "帧 1 AS producer generation = 0(初始 build)"
        );
        assert!(
            f1.telemetry
                .allocations
                .iter()
                .any(|a| a.resource_id == Some(StableResourceId(3)) && a.bytes > 0),
            "AS 分配须入 ledger(resource_id=AS 表项,bytes>0)"
        );

        // ── 帧 2:FrameUpdate(TLAS refit x+10 → miss;buffer 上传;push 覆盖;
        // readback 子集)──
        let moved = RayQueryTransformedInstanceDesc {
            blas: 0,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
            // 行主 3×4:平移 +10 x(同三角形移出射线 → 遍历穷尽 → 哨兵 −1.0)。
            transform: [
                1.0, 0.0, 0.0, 10.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0,
            ],
        };
        let update = FrameUpdate {
            tlas_update: Some((0, vec![moved], TlasBuildAction::Refit)),
            buffer_uploads: vec![(StableResourceId(1), 0, vec![0xAB; 4])],
            binding_overrides: vec![],
            push_constant_overrides: vec![(1, 200u32.to_le_bytes().to_vec())],
            readback_subset: Some(vec![0, 1]),
            blas_refit: None,
        };
        let supplied2 = session
            .next_provenance_with_update(&update)
            .expect("帧 2 update expected provenance");
        let f2 = session
            .execute_with_frame_update(&supplied2, &update)
            .expect("frame 2 with TLAS refit");
        let out2 = f32::from_le_bytes(f2.readbacks[0][..4].try_into().expect("4B"));
        assert_eq!(
            out2, -1.0,
            "帧 2 refit 后 ray query miss: out[0] = 哨兵 -1.0(refit 真实生效),实得 {out2}"
        );
        let w2: Vec<u32> = f2.readbacks[1]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        for (i, &w) in w2.iter().enumerate() {
            assert_eq!(w, i as u32 + 200, "帧 2 push override: buf[{i}] = i+200");
        }
        assert_eq!(f2.provenance.frame_generation, 2);
        // AS generation 递增:refit 记 Write 并 bump(0 → 1),Read producer 指向新代。
        let as2 = f2.provenance.passes[0]
            .resources
            .iter()
            .find(|r| r.resource_id == StableResourceId(3))
            .expect("帧 2 AS provenance 项");
        assert_eq!(
            as2.producer.expect("Read 有 producer").generation,
            1,
            "帧 2 AS producer generation = 1(refit bump 递增)"
        );
        // buffer 上传亦 bump:res0 上传后 gen 2(帧 1 产出 1,上传 bump 至 2)。
        let res0 = f2.provenance.passes[0]
            .resources
            .iter()
            .find(|r| r.resource_id == StableResourceId(1))
            .expect("帧 2 res0 provenance 项");
        assert_eq!(
            res0.producer.expect("ReadWrite 有 producer").generation,
            2,
            "帧 2 res0 producer generation = 2(上传 bump)"
        );
        assert!(
            f2.telemetry.passes.iter().all(|p| p.gpu_ns > 0.0),
            "帧 2 两 pass gpu_ns 均 > 0(非退化): {:?}",
            f2.telemetry.passes
        );
        assert!(f2.telemetry.cpu_record_ns > 0, "重录耗时应如实计量(>0)");
        assert_eq!(f2.telemetry.validation_error_count, 0);
        assert_eq!(f2.telemetry.leaked_object_count, 0);
        assert_eq!(f2.telemetry.leaked_allocation_count, 0);

        // ── 帧 3 RED:stale AS generation(篡改 producer 为 refit 前的 0)→
        // submit 前确定性拒;拒后 session 未污染,净帧复跑成功 ──
        let mut red = session.next_provenance();
        let as_red = red.passes[0]
            .resources
            .iter_mut()
            .find(|r| r.resource_id == StableResourceId(3))
            .expect("帧 3 AS provenance 项");
        as_red.producer.as_mut().expect("producer").generation = 0;
        let err = session
            .execute_with_provenance(&red)
            .expect_err("stale AS generation 须在 submit 前拒");
        assert!(err.contains("fail-closed"), "RED 消息含 fail-closed: {err}");
        let f3 = session
            .execute()
            .expect("帧 3 净帧复跑(RED 未污染 session)");
        assert_eq!(f3.provenance.frame_generation, 3);
        assert_eq!(f3.telemetry.validation_error_count, 0);
        eprintln!(
            "[render_exec] ③c AS session: f1 out={out1} gen0, f2 refit out={out2} AS gen 0→1, \
             f3 RED stale-gen 拒后净帧成功;passes gpu_ns f2={:?}",
            f2.telemetry
                .passes
                .iter()
                .map(|p| p.gpu_ns)
                .collect::<Vec<_>>()
        );
    }

    /// ④ raster+compute 混合两 pass:raster 写纹理 → compute 取中心纹素写 buffer。
    #[test]
    fn device_raster_then_compute_fetch() {
        let Some((vs, fs)) = device_gate() else {
            return;
        };
        let verts = triangle_vertices();
        let fetch_spv = spv_bytes(&sample_fetch_spv_words());
        let resources = vec![
            ResourceDesc::Texture(TextureDesc {
                width: 64,
                height: 64,
                format: TexFormat::Rgba8Unorm,
                usage: TextureUsage {
                    sampled: true,
                    storage: false,
                    color: true,
                    depth: false,
                },
                data: None,
            }),
            ResourceDesc::Buffer(BufferDesc {
                size: 16,
                usage: BufferUsage {
                    storage: true,
                    uniform: false,
                    vertex: false,
                    indirect: false,
                },
                data: None,
                device_local: false,
            }),
        ];
        let passes = vec![
            Pass::Raster(RasterPass {
                name: "tri",
                vs_spirv: vs,
                fs_spirv: fs,
                vertex: VertexData::Inline {
                    data: &verts,
                    stride: 32,
                    attrs: &TRI_ATTRS,
                },
                draw: DrawSpec::Direct {
                    vertex_count: 3,
                    instance_count: 1,
                    first_vertex: 0,
                    first_instance: 0,
                },
                colors: vec![ColorAttachmentRef {
                    res: 0,
                    clear: Some([0.0, 0.0, 0.0, 1.0]),
                }],
                depth: None,
                viewport: None,
                bindings: Bindings::default(),
                conservative: None,
            }),
            Pass::Compute(ComputePass {
                name: "fetch",
                spirv: &fetch_spv,
                entry: None,
                dispatch: DispatchSpec::Direct([1, 1, 1]),
                bindings: Bindings {
                    storage_buffers: vec![1],
                    sampled_images: vec![0],
                    ..Bindings::default()
                },
            }),
        ];
        // 显式 plan 见证(raster 后 RT→SRV + buffer 备写);隐式补全亦覆盖,两者幂等。
        let plan: Vec<Vec<(u32, TargetState)>> = vec![
            vec![(0, TargetState::ColorAttachmentWrite)],
            vec![(0, TargetState::ShaderRead), (1, TargetState::StorageWrite)],
        ];
        let brefs: Vec<&[(u32, TargetState)]> = plan.iter().map(Vec::as_slice).collect();
        let readbacks = vec![Readback::Buffer {
            res: 1,
            offset: 0,
            size: 16,
        }];
        let out = execute_frame(&resources, &passes, &brefs, &readbacks).expect("混合帧应执行成功");
        let floats: Vec<f32> = out[0]
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(floats.len(), 4);
        let (r, g, b, a) = (floats[0], floats[1], floats[2], floats[3]);
        assert!(
            r + g + b > 0.0,
            "中心纹素须非清色黑(三角形覆盖),实得 ({r},{g},{b},{a})"
        );
        assert!((a - 1.0).abs() < 1e-3, "alpha 须 1.0,实得 {a}");
        eprintln!("[render_exec] ④ 混合两 pass: texel(32,32)=({r:.3},{g:.3},{b:.3},{a:.3})");
    }

    // ── G14.10b external memory 导出面 ──

    /// exportable 集校验 fail-closed(host 恒跑):越界 / buffer 非法形态
    /// (G14.10f 语义拓宽后 Buffer 合法,但须 data=None ∧ device_local)/ 重复。
    #[test]
    fn g14_10b_exportable_validation_fail_closed() {
        let spv = spv_bytes(&sample_compute_spv_words());
        let resources = vec![
            ResourceDesc::Buffer(BufferDesc {
                size: 32,
                usage: BufferUsage {
                    storage: true,
                    ..BufferUsage::default()
                },
                data: Some(&[0u8; 32]),
                device_local: false,
            }),
            ResourceDesc::Texture(TextureDesc {
                width: 4,
                height: 4,
                format: TexFormat::Rgba8Unorm,
                usage: TextureUsage {
                    sampled: true,
                    storage: false,
                    color: false,
                    depth: false,
                },
                data: None,
            }),
        ];
        let mut pc = Vec::new();
        pc.extend_from_slice(&0u32.to_le_bytes());
        let passes = vec![Pass::Compute(ComputePass {
            name: "c0",
            spirv: &spv,
            entry: None,
            dispatch: DispatchSpec::Direct([8, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![0],
                push_constants: pc,
                ..Bindings::default()
            },
        })];
        let plan: Vec<Vec<(u32, TargetState)>> = vec![vec![(0, TargetState::StorageWrite)]];
        let brefs: Vec<&[(u32, TargetState)]> = plan.iter().map(Vec::as_slice).collect();
        for (exportable, needle) in [
            (vec![9u32], "越界"),
            // G14.10f:资源 0 = buffer 带初始数据 + 非 device_local,两条
            // buffer 形态门任一命中即拒(external dedicated 分配与创建期
            // staging 上传互斥;导出面强制 DEVICE_LOCAL)。
            (vec![0u32], "带初始数据"),
            (vec![1u32, 1u32], "重复"),
        ] {
            let r = DeviceFrameSession::new_with_exportable_textures(
                &resources,
                &passes,
                &brefs,
                &[],
                2,
                &[],
                &exportable,
            );
            let err = r.err().unwrap_or_else(|| panic!("{needle} 臂须拒"));
            assert!(err.contains(needle), "{needle} 臂错误字面不符: {err}");
        }
        // G14.10f 加性臂:合法 buffer 形态(data=None + device_local)须**通过
        // 校验段**——不得被形态门误拒(后续失败只可能来自设备面,非校验面)。
        let ok_resources = vec![
            ResourceDesc::Buffer(BufferDesc {
                size: 32,
                usage: BufferUsage {
                    storage: true,
                    ..BufferUsage::default()
                },
                data: None,
                device_local: true,
            }),
            resources[1].clone(),
        ];
        let r = DeviceFrameSession::new_with_exportable_textures(
            &ok_resources,
            &passes,
            &brefs,
            &[],
            2,
            &[],
            &[0u32],
        );
        if let Err(e) = &r {
            assert!(
                !e.contains("exportable_textures[0]"),
                "合法 buffer 形态被校验段误拒: {e}"
            );
        }
    }

    /// GPU 真跑:exportable image 创建/导出/第二 device 导入/读回闭环。
    /// session A(初始数据上传 + 帧末 EXTERNAL release)→ 导出 NT handle →
    /// 手写导入方(独立 instance/device,acquire + copy 到 host buffer)→
    /// 与源图案及 session 侧 readback **位级比对**。LUID 同 adapter 前置断言。
    ///
    /// 已知 validation 假阳性(RURIX_VK_VALIDATION=1 时):导入方 layer 报
    /// `InvalidImageLayout ... expects GENERAL, current layout is UNDEFINED`——
    /// 跨 instance 的 external memory layout 无法被单 instance layout tracker
    /// 跟踪;acquire 侧 oldLayout=GENERAL 与导出侧 release 配对是规范 7.7.4 的
    /// 正确做法(oldLayout=UNDEFINED 会许可驱动丢内容,不可用)。内容面由本
    /// 测试位级比对判定,不受该假阳性影响。
    #[test]
    #[cfg(windows)]
    fn g14_10b_exportable_cross_device_roundtrip() {
        if !crate::vk::vulkan_available() {
            eprintln!("[render_exec] SKIP: vulkan loader 不可用(g14.10b 闭环)");
            return;
        }
        const W: u32 = 64;
        const H: u32 = 64;
        // 确定性测试图案(RGBA8 逐字节;含 0/255 边界值)。
        let pattern: Vec<u8> = (0..(W * H * 4) as usize)
            .map(|i| ((i * 31 + 7) % 256) as u8)
            .collect();
        let spv = spv_bytes(&sample_compute_spv_words());
        let resources = vec![
            ResourceDesc::Texture(TextureDesc {
                width: W,
                height: H,
                format: TexFormat::Rgba8Unorm,
                usage: TextureUsage {
                    sampled: true,
                    storage: false,
                    color: false,
                    depth: false,
                },
                data: Some(&pattern),
            }),
            ResourceDesc::Buffer(BufferDesc {
                size: 32,
                usage: BufferUsage {
                    storage: true,
                    ..BufferUsage::default()
                },
                data: Some(&[0u8; 32]),
                device_local: false,
            }),
        ];
        let mut pc = Vec::new();
        pc.extend_from_slice(&5u32.to_le_bytes());
        let passes = vec![Pass::Compute(ComputePass {
            name: "c0",
            spirv: &spv,
            entry: None,
            dispatch: DispatchSpec::Direct([8, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![1],
                push_constants: pc,
                ..Bindings::default()
            },
        })];
        let plan: Vec<Vec<(u32, TargetState)>> = vec![vec![(1, TargetState::StorageWrite)]];
        let brefs: Vec<&[(u32, TargetState)]> = plan.iter().map(Vec::as_slice).collect();
        let readbacks = vec![Readback::Texture { res: 0 }];
        let mut session = match DeviceFrameSession::new_with_exportable_textures(
            &resources,
            &passes,
            &brefs,
            &readbacks,
            2,
            &[],
            &[0],
        ) {
            Ok(s) => s,
            Err(e) => {
                // 扩展缺位环境如实 SKIP(RTX 4070 Ti 在位环境走全链)。
                eprintln!("[render_exec] SKIP: exportable session 不可用({e})");
                return;
            }
        };
        let Some(luid) = session.physical_device_luid() else {
            eprintln!("[render_exec] SKIP: 驱动报 deviceLUIDValid=false");
            return;
        };
        // 帧 1:上传段写入图案 → 帧末 release;session 侧 readback 对照。
        let out = session.execute().expect("exportable 帧应执行成功");
        assert_eq!(
            out.readbacks[0], pattern,
            "session 侧 readback 须与源图案位级一致"
        );
        let exported = session
            .export_texture_win32_handle(0)
            .expect("导出 win32 handle");
        assert!(exported.handle != 0, "NT handle 须非零");
        assert_eq!((exported.width, exported.height), (W, H));
        assert_eq!(exported.vk_format, TexFormat::Rgba8Unorm.vk_format());
        // 重复导出返缓存同句柄(免泄漏)。
        let again = session
            .export_texture_win32_handle(0)
            .expect("重复导出走缓存");
        assert_eq!(again.handle, exported.handle, "重复导出须返同一缓存句柄");
        // 非 exportable 下标 fail-closed。
        assert!(session.export_texture_win32_handle(1).is_err());

        // 导入方(独立 instance/device;同 physical device LUID 前置断言)。
        // SAFETY: 全部句柄本函数内创建、末尾逆序销毁;handle 归 session 所有不关闭;
        // acquire barrier 与 session 帧末 release 配对(GENERAL→GENERAL)。
        let imported =
            unsafe { import_and_readback_rgba8(&exported, luid, W, H) }.expect("导入方闭环");
        assert_eq!(
            imported, pattern,
            "跨 device 导入读回须与源图案位级一致(external memory 闭环)"
        );
        eprintln!(
            "[render_exec] g14.10b 闭环 PASS: {}×{} RGBA8 {}B 位级一致(handle={:#x}, alloc={}B, memType={})",
            W,
            H,
            pattern.len(),
            exported.handle,
            exported.allocation_size,
            exported.memory_type_index
        );
    }

    /// 手写导入方:第二 VkInstance/VkDevice 导入 OPAQUE_WIN32 handle →
    /// acquire(EXTERNAL→qf)→ GENERAL→TRANSFER_SRC → copy 到 host buffer → map 读出。
    ///
    /// # Safety
    /// `exported.handle` 为有效 NT handle 且导出 session 存活;调用方保证导出帧
    /// fence 已完成(execute 已返回)。
    #[cfg(windows)]
    unsafe fn import_and_readback_rgba8(
        exported: &ExportedTextureWin32,
        expect_luid: [u8; 8],
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, String> {
        let gipa = load_vulkan_loader().ok_or("vulkan loader 不可用")?;
        let (instance, _validation, _debug_utils) = create_instance(gipa, c"rurix-g14-10b-import")?;
        let destroy_instance: FnDestroyInstance =
            cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
        let result = (|| {
            let pd = pick_physical_device(gipa, instance)?;
            // LUID 对拍(同 adapter 才可共享;不同即 fail-closed)。
            let get2: FnGetPhysicalDeviceProperties2 =
                cast_fn(gipa(instance, c"vkGetPhysicalDeviceProperties2".as_ptr()))
                    .ok_or("缺 vkGetPhysicalDeviceProperties2")?;
            let mut id_props = PhysicalDeviceIDProperties {
                s_type: ST_PHYSICAL_DEVICE_ID_PROPERTIES,
                p_next: std::ptr::null_mut(),
                device_uuid: [0; 16],
                driver_uuid: [0; 16],
                device_luid: [0; 8],
                device_node_mask: 0,
                device_luid_valid: 0,
            };
            let mut props2 = PhysicalDeviceProperties2Chain {
                s_type: ST_PHYSICAL_DEVICE_PROPERTIES_2,
                p_next: (&mut id_props as *mut PhysicalDeviceIDProperties).cast::<c_void>(),
                properties: PropertiesBlob { bytes: [0; 2048] },
            };
            get2(pd, &mut props2);
            if id_props.device_luid_valid == 0 || id_props.device_luid != expect_luid {
                return Err(format!(
                    "导入方物理设备 LUID 不匹配(valid={}, luid={:?} vs {:?})——不同 adapter \
                     不可共享,fail-closed",
                    id_props.device_luid_valid, id_props.device_luid, expect_luid
                ));
            }
            let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
                instance,
                c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
            ))
            .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
            let vk_get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
                instance,
                c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
            ))
            .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
            let vk_create_device: FnCreateDevice =
                cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
            let gdpa: FnGetDeviceProcAddr =
                cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr()))
                    .ok_or("缺 vkGetDeviceProcAddr")?;
            let mut qf_count = 0u32;
            vk_get_qf(pd, &mut qf_count, std::ptr::null_mut());
            let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
                .map(|_| QueueFamilyProperties {
                    queue_flags: 0,
                    queue_count: 0,
                    timestamp_valid_bits: 0,
                    min_image_transfer_granularity: VkExtent3D {
                        width: 0,
                        height: 0,
                        depth: 0,
                    },
                })
                .collect();
            vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
            let qfi = qfs
                .iter()
                .position(|q| q.queue_flags & QUEUE_GRAPHICS_BIT != 0)
                .ok_or("无 graphics queue family")? as u32;
            let mut sync2_feat = PhysicalDeviceSynchronization2Features {
                s_type: ST_PHYSICAL_DEVICE_SYNCHRONIZATION_2_FEATURES,
                p_next: std::ptr::null_mut(),
                synchronization2: 1,
            };
            let exts = [
                c"VK_KHR_synchronization2".as_ptr(),
                c"VK_KHR_external_memory".as_ptr(),
                c"VK_KHR_external_memory_win32".as_ptr(),
            ];
            let priority = [1.0f32];
            let dqci = DeviceQueueCreateInfo {
                s_type: ST_DEVICE_QUEUE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                queue_family_index: qfi,
                queue_count: 1,
                p_queue_priorities: priority.as_ptr(),
            };
            let dci = DeviceCreateInfo {
                s_type: ST_DEVICE_CREATE_INFO,
                p_next: (&mut sync2_feat as *mut PhysicalDeviceSynchronization2Features)
                    .cast::<c_void>()
                    .cast_const(),
                flags: 0,
                queue_create_info_count: 1,
                p_queue_create_infos: &dqci,
                enabled_layer_count: 0,
                pp_enabled_layer_names: std::ptr::null(),
                enabled_extension_count: exts.len() as u32,
                pp_enabled_extension_names: exts.as_ptr(),
                p_enabled_features: std::ptr::null(),
            };
            let mut device: VkDevice = std::ptr::null_mut();
            let r = vk_create_device(pd, &dci, std::ptr::null(), &mut device);
            if r != VK_SUCCESS {
                return Err(format!("vkCreateDevice(import) 失败: {r}"));
            }
            let vk_destroy_device: FnDestroyDevice =
                cast_fn(gdpa(device, c"vkDestroyDevice".as_ptr())).ok_or("缺 vkDestroyDevice")?;
            let dev = Dev::load(gdpa, device)?;
            let import_result = (|| {
                let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
                vk_get_mem(pd, &mut memprops);
                // 导入 image:与导出侧参数一致(format/extent/tiling/usage)+ external chain。
                let ext_info = ExternalMemoryImageCreateInfo {
                    s_type: ST_EXTERNAL_MEMORY_IMAGE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    handle_types: EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
                };
                let ici = ImageCreateInfo {
                    s_type: ST_IMAGE_CREATE_INFO,
                    p_next: (&ext_info as *const ExternalMemoryImageCreateInfo).cast(),
                    flags: 0,
                    image_type: IMAGE_TYPE_2D,
                    format: exported.vk_format,
                    extent: VkExtent3D {
                        width: w,
                        height: h,
                        depth: 1,
                    },
                    mip_levels: 1,
                    array_layers: 1,
                    samples: SAMPLE_COUNT_1,
                    tiling: IMAGE_TILING_OPTIMAL,
                    usage: exported.usage_flags,
                    sharing_mode: SHARING_MODE_EXCLUSIVE,
                    queue_family_index_count: 0,
                    p_queue_family_indices: std::ptr::null(),
                    initial_layout: LAYOUT_UNDEFINED,
                };
                let mut image: VkImage = VK_NULL_HANDLE;
                if (dev.create_image)(device, &ici, std::ptr::null(), &mut image) != VK_SUCCESS {
                    return Err("vkCreateImage(import) 失败".to_owned());
                }
                let cleanup_image = |img: VkImage, mem: VkDeviceMemory| {
                    if img != VK_NULL_HANDLE {
                        (dev.destroy_image)(device, img, std::ptr::null());
                    }
                    if mem != VK_NULL_HANDLE {
                        (dev.free_mem)(device, mem, std::ptr::null());
                    }
                };
                // 导入分配:import(handle)→ dedicated(image);allocationSize /
                // memoryTypeIndex 采导出侧簿记(同 LUID 物理设备类型序一致)。
                let dedicated = MemoryDedicatedAllocateInfo {
                    s_type: ST_MEMORY_DEDICATED_ALLOCATE_INFO,
                    p_next: std::ptr::null(),
                    image,
                    buffer: VK_NULL_HANDLE,
                };
                let import_info = ImportMemoryWin32HandleInfoKHR {
                    s_type: ST_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR,
                    p_next: (&dedicated as *const MemoryDedicatedAllocateInfo).cast(),
                    handle_type: EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
                    handle: exported.handle as *mut c_void,
                    name: std::ptr::null(),
                };
                let mai = MemoryAllocateInfo {
                    s_type: ST_MEMORY_ALLOCATE_INFO,
                    p_next: (&import_info as *const ImportMemoryWin32HandleInfoKHR).cast(),
                    allocation_size: exported.allocation_size,
                    memory_type_index: exported.memory_type_index,
                };
                let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
                if (dev.alloc_mem)(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
                    cleanup_image(image, VK_NULL_HANDLE);
                    return Err("vkAllocateMemory(import win32) 失败".to_owned());
                }
                if (dev.bind_img)(device, image, mem, 0) != VK_SUCCESS {
                    cleanup_image(image, mem);
                    return Err("vkBindImageMemory(import) 失败".to_owned());
                }
                // host 可见 readback buffer。
                let byte_len = (w as u64) * (h as u64) * 4;
                let bci = BufferCreateInfo {
                    s_type: ST_BUFFER_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                    size: byte_len,
                    usage: 0x2, // TRANSFER_DST
                    sharing_mode: SHARING_MODE_EXCLUSIVE,
                    queue_family_index_count: 0,
                    p_queue_family_indices: std::ptr::null(),
                };
                let mut rbuf: VkBuffer = VK_NULL_HANDLE;
                if (dev.create_buffer)(device, &bci, std::ptr::null(), &mut rbuf) != VK_SUCCESS {
                    cleanup_image(image, mem);
                    return Err("vkCreateBuffer(import readback) 失败".to_owned());
                }
                let mut breq = std::mem::zeroed::<MemoryRequirements>();
                (dev.buf_mem_req)(device, rbuf, &mut breq);
                let Some(bmt) = pick_mem_type(
                    &memprops,
                    breq.memory_type_bits,
                    MEM_HOST_VISIBLE | MEM_HOST_COHERENT,
                ) else {
                    (dev.destroy_buffer)(device, rbuf, std::ptr::null());
                    cleanup_image(image, mem);
                    return Err("无 host-visible+coherent 内存类型".to_owned());
                };
                let bmai = MemoryAllocateInfo {
                    s_type: ST_MEMORY_ALLOCATE_INFO,
                    p_next: std::ptr::null(),
                    allocation_size: breq.size,
                    memory_type_index: bmt,
                };
                let mut bmem: VkDeviceMemory = VK_NULL_HANDLE;
                if (dev.alloc_mem)(device, &bmai, std::ptr::null(), &mut bmem) != VK_SUCCESS {
                    (dev.destroy_buffer)(device, rbuf, std::ptr::null());
                    cleanup_image(image, mem);
                    return Err("vkAllocateMemory(import readback) 失败".to_owned());
                }
                (dev.bind_buf)(device, rbuf, bmem, 0);
                let cleanup_all = |cmdpool: VkCommandPool| {
                    if cmdpool != VK_NULL_HANDLE {
                        (dev.destroy_cmdpool)(device, cmdpool, std::ptr::null());
                    }
                    (dev.destroy_buffer)(device, rbuf, std::ptr::null());
                    (dev.free_mem)(device, bmem, std::ptr::null());
                    cleanup_image(image, mem);
                };
                // cmd:acquire(EXTERNAL→qfi,GENERAL→GENERAL 与导出侧 release 配对)
                // → GENERAL→TRANSFER_SRC → copy → submit → wait。
                let cpci = CommandPoolCreateInfo {
                    s_type: ST_COMMAND_POOL_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                    queue_family_index: qfi,
                };
                let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;
                if (dev.create_cmdpool)(device, &cpci, std::ptr::null(), &mut cmdpool) != VK_SUCCESS
                {
                    cleanup_all(VK_NULL_HANDLE);
                    return Err("vkCreateCommandPool(import) 失败".to_owned());
                }
                let cbai = CommandBufferAllocateInfo {
                    s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
                    p_next: std::ptr::null(),
                    command_pool: cmdpool,
                    level: CMD_BUFFER_LEVEL_PRIMARY,
                    command_buffer_count: 1,
                };
                let mut cmd: VkCommandBuffer = std::ptr::null_mut();
                if (dev.alloc_cmd)(device, &cbai, &mut cmd) != VK_SUCCESS {
                    cleanup_all(cmdpool);
                    return Err("vkAllocateCommandBuffers(import) 失败".to_owned());
                }
                let cbi = CommandBufferBeginInfo {
                    s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
                    p_next: std::ptr::null(),
                    flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
                    p_inheritance_info: std::ptr::null(),
                };
                if (dev.begin_cmd)(cmd, &cbi) != VK_SUCCESS {
                    cleanup_all(cmdpool);
                    return Err("vkBeginCommandBuffer(import) 失败".to_owned());
                }
                let subrange = || VkImageSubresourceRange {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                };
                let barriers2 = [
                    // acquire:EXTERNAL→qfi(GENERAL→GENERAL 零转换,与 release 配对)。
                    ImageMemoryBarrier2 {
                        s_type: ST_IMAGE_MEMORY_BARRIER_2,
                        p_next: std::ptr::null(),
                        src_stage_mask: STAGE2_ALL_COMMANDS,
                        src_access_mask: 0,
                        dst_stage_mask: STAGE2_TRANSFER,
                        dst_access_mask: ACCESS2_TRANSFER_READ,
                        old_layout: LAYOUT_GENERAL,
                        new_layout: LAYOUT_GENERAL,
                        src_queue_family_index: QUEUE_FAMILY_EXTERNAL,
                        dst_queue_family_index: qfi,
                        image,
                        subresource_range: subrange(),
                    },
                    // 本 device 内 GENERAL→TRANSFER_SRC。
                    ImageMemoryBarrier2 {
                        s_type: ST_IMAGE_MEMORY_BARRIER_2,
                        p_next: std::ptr::null(),
                        src_stage_mask: STAGE2_TRANSFER,
                        src_access_mask: 0,
                        dst_stage_mask: STAGE2_TRANSFER,
                        dst_access_mask: ACCESS2_TRANSFER_READ,
                        old_layout: LAYOUT_GENERAL,
                        new_layout: LAYOUT_TRANSFER_SRC_OPTIMAL,
                        src_queue_family_index: QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                        image,
                        subresource_range: subrange(),
                    },
                ];
                for b in &barriers2 {
                    let di = DependencyInfo {
                        s_type: ST_DEPENDENCY_INFO,
                        p_next: std::ptr::null(),
                        dependency_flags: 0,
                        memory_barrier_count: 0,
                        p_memory_barriers: std::ptr::null(),
                        buffer_memory_barrier_count: 0,
                        p_buffer_memory_barriers: std::ptr::null(),
                        image_memory_barrier_count: 1,
                        p_image_memory_barriers: b,
                    };
                    (dev.cmd_barrier2)(cmd, &di);
                }
                let region = VkBufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource: VkImageSubresourceLayers {
                        aspect_mask: IMAGE_ASPECT_COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
                    image_extent: VkExtent3D {
                        width: w,
                        height: h,
                        depth: 1,
                    },
                };
                (dev.cmd_copy_img2buf)(cmd, image, LAYOUT_TRANSFER_SRC_OPTIMAL, rbuf, 1, &region);
                if (dev.end_cmd)(cmd) != VK_SUCCESS {
                    cleanup_all(cmdpool);
                    return Err("vkEndCommandBuffer(import) 失败".to_owned());
                }
                let mut queue: VkQueue = std::ptr::null_mut();
                (dev.get_device_queue)(device, qfi, 0, &mut queue);
                let si = SubmitInfo {
                    s_type: ST_SUBMIT_INFO,
                    p_next: std::ptr::null(),
                    wait_semaphore_count: 0,
                    p_wait_semaphores: std::ptr::null(),
                    p_wait_dst_stage_mask: std::ptr::null(),
                    command_buffer_count: 1,
                    p_command_buffers: &cmd,
                    signal_semaphore_count: 0,
                    p_signal_semaphores: std::ptr::null(),
                };
                if (dev.queue_submit)(queue, 1, &si, 0) != VK_SUCCESS {
                    cleanup_all(cmdpool);
                    return Err("vkQueueSubmit(import) 失败".to_owned());
                }
                if (dev.queue_wait)(queue) != VK_SUCCESS {
                    cleanup_all(cmdpool);
                    return Err("vkQueueWaitIdle(import) 失败".to_owned());
                }
                let mut ptr: *mut c_void = std::ptr::null_mut();
                if (dev.map_mem)(device, bmem, 0, byte_len, 0, &mut ptr) != VK_SUCCESS
                    || ptr.is_null()
                {
                    cleanup_all(cmdpool);
                    return Err("vkMapMemory(import readback) 失败".to_owned());
                }
                let bytes =
                    std::slice::from_raw_parts(ptr.cast::<u8>(), byte_len as usize).to_vec();
                (dev.unmap_mem)(device, bmem);
                cleanup_all(cmdpool);
                Ok(bytes)
            })();
            let _ = (dev.queue_wait)({
                let mut q: VkQueue = std::ptr::null_mut();
                (dev.get_device_queue)(device, qfi, 0, &mut q);
                q
            });
            vk_destroy_device(device, std::ptr::null());
            import_result
        })();
        destroy_instance(instance, std::ptr::null());
        result
    }
}

// ── G37 W3 #90:FIF×动态共存判档加性面(每槽 AS 副本 opt-in 提交入口;既有
//    入口/行为 0-byte——vk_g37_async_lanes body-include 先例同律,语义头注见
//    该文件;RFC-0030 §4.3 L2 修订行草案的实现底稿)──
include!("render_exec_g37_fif_dyn.rs");
