#![forbid(unsafe_code)]
//! RXS-0236 ~ RXS-0241:render graph 纯 host 自动资源状态推导(RFC-0013 §4.D)。
//!
//! **定位**:声明式宿主库面的 host 侧本体——图合法性装配核验(D2)+ 自动资源状态
//! 推导(D3,纯函数)+ 双后端 barrier 映射同源表(D3/D5)。本模块 **always-on、零 unsafe、
//! 零后端调用、无 GPU 依赖**(`#![forbid(unsafe_code)]` 编译期封口);推导为纯函数,同图 →
//! 逐字节相同计划(golden 可锚)。执行器(`vk.rs run_graph` / uc04 D3D12 shim)只**逐字重放**
//! 本模块产出的计划,**禁止后端侧二次推导或语义重映射**(P-11 单一事实源)。
//!
//! 🔒 **pass 边界 happens-before 语义本体(D4,RXS-0239)**:单 queue、声明序 = 提交序 =
//! pass 粒度完成序;每个 pass 边界是全序同步点;RAW/WAW/WAR 三类跨 pass 冲突全被该全序
//! 裁定,可见性仅在 pass 粒度给出。**本面无 UB 节**:承诺面之外的构造走装配期 6xxx strict 拒
//! (RX6029/RX6030),运行期后端失败走确定性诊断 + 终止 + poisoned 传播(RXS-0193/0194)。
//!
//! **D6 互证金标准(oracle 独立性)**:本模块 **禁止 import `uc04-demo` barrier.rs 任何推导
//! 逻辑**;推导与 uc04 手动 `plan_barriers`(RXS-0169)对 deferred 三 pass 图的集合相等由
//! `uc04-demo` 侧 D6 单测双向断言(两独立实现互证)。
//!
//! **首期不可表达面(§4.0-3)**:bindless 表声明、storage image(`TextureRw2D`)资源、
//! mesh/RT pass kind 均不在 [`AccessKind`] 封闭枚举内;凡含此三者的 pass 首期不可经 graph
//! 表达(显式登记 RD-034+,非静默)。storage image barrier 首期走 RXS-0169 手动路。

use std::collections::BTreeSet;

// ── Vulkan 数值常量(与 vk.rs 单一事实源逐值一致;执行器逐字重放,禁二次映射)──────────

/// `VkImageLayout` 数值(与 `vk.rs` 常量逐值一致)。
pub mod vk_layout {
    #![allow(missing_docs)]
    pub const UNDEFINED: u32 = 0;
    pub const GENERAL: u32 = 1;
    pub const COLOR_ATTACHMENT_OPTIMAL: u32 = 2;
    pub const DEPTH_STENCIL_ATTACHMENT_OPTIMAL: u32 = 3;
    pub const SHADER_READ_ONLY_OPTIMAL: u32 = 5;
    pub const TRANSFER_SRC_OPTIMAL: u32 = 6;
    pub const TRANSFER_DST_OPTIMAL: u32 = 7;
    pub const PRESENT_SRC_KHR: u32 = 1_000_001_002;
}

/// `VkPipelineStageFlagBits` 数值(与 `vk.rs` 常量逐值一致)。
pub mod vk_stage {
    #![allow(missing_docs)]
    pub const TOP_OF_PIPE: u32 = 0x1;
    pub const VERTEX_SHADER: u32 = 0x8;
    pub const FRAGMENT_SHADER: u32 = 0x80;
    pub const EARLY_FRAGMENT_TESTS: u32 = 0x100;
    pub const LATE_FRAGMENT_TESTS: u32 = 0x200;
    pub const COLOR_ATTACHMENT_OUTPUT: u32 = 0x400;
    pub const COMPUTE_SHADER: u32 = 0x800;
    pub const TRANSFER: u32 = 0x1000;
    pub const BOTTOM_OF_PIPE: u32 = 0x2000;
    pub const ALL_COMMANDS: u32 = 0x1_0000;
}

/// `VkAccessFlagBits` 数值(与 `vk.rs` 常量逐值一致)。
pub mod vk_access {
    #![allow(missing_docs)]
    pub const SHADER_READ: u32 = 0x20;
    pub const SHADER_WRITE: u32 = 0x40;
    pub const COLOR_ATTACHMENT_WRITE: u32 = 0x100;
    pub const DEPTH_STENCIL_ATTACHMENT_WRITE: u32 = 0x400;
    pub const TRANSFER_READ: u32 = 0x800;
    pub const TRANSFER_WRITE: u32 = 0x1000;
    pub const MEMORY_READ: u32 = 0x8000;
}

// ── AccessKind 封闭枚举(单一事实源,D3/D5)────────────────────────────────────────────

/// 访问声明的封闭枚举——本面「不支持即不可表达」(D2)。D3/D5 双后端映射表的单一事实源。
///
/// 首期资源面**不含** bindless 表、storage image(`TextureRw2D`)资源、mesh/RT pass kind
/// (§4.0-3,RD-034+ 登记)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AccessKind {
    /// `writes_rt(t)`:color attachment 写(`RENDER_TARGET` / `COLOR_ATTACHMENT_OPTIMAL`)。
    ColorAttachmentWrite,
    /// `writes_depth(t)`:depth attachment 写(`DEPTH_WRITE` / `DEPTH_STENCIL_ATTACHMENT_OPTIMAL`)。
    DepthAttachmentWrite,
    /// `reads(t)`:shader 资源读(`PIXEL_SHADER_RESOURCE` / `SHADER_READ_ONLY_OPTIMAL`)。
    ShaderRead,
    /// `reads_writes_uav(b)`:UAV 读写合并(唯一合法读写合并;`UNORDERED_ACCESS` / `GENERAL`)。
    UavReadWrite,
    /// `readback(t, dst)` 源:copy 源(`COPY_SOURCE` / `TRANSFER_SRC_OPTIMAL`)。
    CopySrcReadback,
    /// `readback(t, dst)` 目的 buffer:copy 目的(`COPY_DEST` / buffer 无 layout)。
    CopyDstReadback,
    /// present 终端胶水(D5c):backbuffer 交回 present 会话(`PRESENT` / `PRESENT_SRC_KHR`)。
    PresentHandoff,
}

impl AccessKind {
    /// 该访问是否为「写」语义(建立/更新资源内容;推导的 read-before-write 判据用)。
    #[must_use]
    pub fn is_write(self) -> bool {
        matches!(
            self,
            AccessKind::ColorAttachmentWrite
                | AccessKind::DepthAttachmentWrite
                | AccessKind::UavReadWrite
                | AccessKind::CopyDstReadback
        )
    }

    /// 该访问是否为「消费读」语义(要求同资源已被先前 pass 写过)。
    #[must_use]
    pub fn is_consuming_read(self) -> bool {
        matches!(
            self,
            AccessKind::ShaderRead | AccessKind::CopySrcReadback | AccessKind::PresentHandoff
        )
    }

    /// AccessKind → D3D12 资源状态(单一事实源,D3 映射锚点)。
    #[must_use]
    pub fn d3d12_state(self) -> D3d12State {
        match self {
            AccessKind::ColorAttachmentWrite => D3d12State::RenderTarget,
            AccessKind::DepthAttachmentWrite => D3d12State::DepthWrite,
            AccessKind::ShaderRead => D3d12State::PixelShaderResource,
            AccessKind::UavReadWrite => D3d12State::UnorderedAccess,
            AccessKind::CopySrcReadback => D3d12State::CopySource,
            AccessKind::CopyDstReadback => D3d12State::CopyDest,
            AccessKind::PresentHandoff => D3d12State::Present,
        }
    }

    /// AccessKind → `VkImageLayout` 数值(单一事实源,D3 映射锚点;buffer 侧无 layout 见 [`BarrierForm`])。
    #[must_use]
    pub fn vk_layout(self) -> u32 {
        match self {
            AccessKind::ColorAttachmentWrite => vk_layout::COLOR_ATTACHMENT_OPTIMAL,
            AccessKind::DepthAttachmentWrite => vk_layout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            AccessKind::ShaderRead => vk_layout::SHADER_READ_ONLY_OPTIMAL,
            AccessKind::UavReadWrite => vk_layout::GENERAL,
            AccessKind::CopySrcReadback => vk_layout::TRANSFER_SRC_OPTIMAL,
            AccessKind::CopyDstReadback => vk_layout::TRANSFER_DST_OPTIMAL,
            AccessKind::PresentHandoff => vk_layout::PRESENT_SRC_KHR,
        }
    }

    /// AccessKind → 保守 `VkPipelineStageFlags`(D4 最保守 sound 掩码:覆盖生产/消费全阶段)。
    #[must_use]
    pub fn vk_stage(self) -> u32 {
        match self {
            AccessKind::ColorAttachmentWrite => vk_stage::COLOR_ATTACHMENT_OUTPUT,
            AccessKind::DepthAttachmentWrite => {
                vk_stage::EARLY_FRAGMENT_TESTS | vk_stage::LATE_FRAGMENT_TESTS
            }
            AccessKind::ShaderRead => vk_stage::FRAGMENT_SHADER,
            AccessKind::UavReadWrite => vk_stage::FRAGMENT_SHADER | vk_stage::COMPUTE_SHADER,
            AccessKind::CopySrcReadback | AccessKind::CopyDstReadback => vk_stage::TRANSFER,
            AccessKind::PresentHandoff => vk_stage::BOTTOM_OF_PIPE,
        }
    }

    /// AccessKind → `VkAccessFlags`(D4 保守访问掩码)。
    #[must_use]
    pub fn vk_access(self) -> u32 {
        match self {
            AccessKind::ColorAttachmentWrite => vk_access::COLOR_ATTACHMENT_WRITE,
            AccessKind::DepthAttachmentWrite => vk_access::DEPTH_STENCIL_ATTACHMENT_WRITE,
            AccessKind::ShaderRead => vk_access::SHADER_READ,
            AccessKind::UavReadWrite => vk_access::SHADER_READ | vk_access::SHADER_WRITE,
            AccessKind::CopySrcReadback => vk_access::TRANSFER_READ,
            AccessKind::CopyDstReadback => vk_access::TRANSFER_WRITE,
            AccessKind::PresentHandoff => vk_access::MEMORY_READ,
        }
    }

    /// C ABI / cabi 下发用的稳定 u32 tag(`rxrt_graph_declare` 参数;含义冻结,只追加)。
    #[must_use]
    pub fn as_u32(self) -> u32 {
        match self {
            AccessKind::ColorAttachmentWrite => 0,
            AccessKind::DepthAttachmentWrite => 1,
            AccessKind::ShaderRead => 2,
            AccessKind::UavReadWrite => 3,
            AccessKind::CopySrcReadback => 4,
            AccessKind::CopyDstReadback => 5,
            AccessKind::PresentHandoff => 6,
        }
    }

    /// u32 tag → AccessKind(cabi 上行;未知 tag → `None`)。
    #[must_use]
    pub fn from_u32(v: u32) -> Option<AccessKind> {
        Some(match v {
            0 => AccessKind::ColorAttachmentWrite,
            1 => AccessKind::DepthAttachmentWrite,
            2 => AccessKind::ShaderRead,
            3 => AccessKind::UavReadWrite,
            4 => AccessKind::CopySrcReadback,
            5 => AccessKind::CopyDstReadback,
            6 => AccessKind::PresentHandoff,
            _ => return None,
        })
    }
}

/// D3D12 资源状态(graph.rs 独立枚举,**不 import** uc04 `barrier.rs`;oracle 独立性,D6)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum D3d12State {
    /// `D3D12_RESOURCE_STATE_COMMON`。
    Common,
    /// `D3D12_RESOURCE_STATE_RENDER_TARGET`。
    RenderTarget,
    /// `D3D12_RESOURCE_STATE_DEPTH_WRITE`。
    DepthWrite,
    /// `D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE`。
    PixelShaderResource,
    /// `D3D12_RESOURCE_STATE_UNORDERED_ACCESS`。
    UnorderedAccess,
    /// `D3D12_RESOURCE_STATE_COPY_SOURCE`。
    CopySource,
    /// `D3D12_RESOURCE_STATE_COPY_DEST`。
    CopyDest,
    /// `D3D12_RESOURCE_STATE_PRESENT`(== `COMMON` 数值,语义 present handoff)。
    Present,
}

// ── 资源与 pass 建面 ───────────────────────────────────────────────────────────────

/// 资源类别(状态机诚实性,D3:不把 buffer/UAV 硬套 image 迁移模型)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceClass {
    /// color/depth attachment image:有 layout,barrier 形态 [`BarrierForm::Transition`]。
    Image,
    /// buffer:无 Vulkan layout,barrier 形态 [`BarrierForm::BufferSync`] / [`BarrierForm::UavSync`]。
    Buffer,
}

/// 图内资源标识(资源表下标)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(pub u32);

/// 单条访问声明(资源 + 访问种类)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Access {
    /// 被访问资源。
    pub resource: ResourceId,
    /// 访问种类(封闭枚举)。
    pub kind: AccessKind,
}

/// pass 声明(访问集 + 可选管线绑定反射面)。
#[derive(Debug, Clone, Default)]
pub struct PassSpec {
    /// pass 诊断名。
    pub name: String,
    /// 访问声明集(声明序 = 提交序)。
    pub accesses: Vec<Access>,
    /// 可选管线绑定反射面(RXS-0163~0166 单一事实源):存在时与声明集**双向精确相等**核验;
    /// `None` = 纯 host 推导不要求反射(D6 互证 / golden 场景)。相等域 = 首期封闭枚举资源面。
    pub reflection: Option<Vec<ResourceId>>,
}

impl PassSpec {
    /// 新建具名 pass。
    #[must_use]
    pub fn new(name: &str) -> PassSpec {
        PassSpec {
            name: name.to_owned(),
            accesses: Vec::new(),
            reflection: None,
        }
    }

    /// 追加一条访问声明(内部辅助)。
    #[must_use]
    fn with(mut self, resource: ResourceId, kind: AccessKind) -> PassSpec {
        self.accesses.push(Access { resource, kind });
        self
    }

    /// `writes_rt(t)`:color attachment 写。
    #[must_use]
    pub fn writes_rt(self, t: ResourceId) -> PassSpec {
        self.with(t, AccessKind::ColorAttachmentWrite)
    }

    /// `writes_depth(t)`:depth attachment 写。
    #[must_use]
    pub fn writes_depth(self, t: ResourceId) -> PassSpec {
        self.with(t, AccessKind::DepthAttachmentWrite)
    }

    /// `reads(t)`:shader 资源读。
    #[must_use]
    pub fn reads(self, t: ResourceId) -> PassSpec {
        self.with(t, AccessKind::ShaderRead)
    }

    /// `reads_writes_uav(b)`:UAV 读写合并(唯一合法读写合并)。
    #[must_use]
    pub fn reads_writes_uav(self, b: ResourceId) -> PassSpec {
        self.with(b, AccessKind::UavReadWrite)
    }

    /// present 终端胶水:backbuffer → PRESENT。
    #[must_use]
    pub fn present_handoff(self, t: ResourceId) -> PassSpec {
        self.with(t, AccessKind::PresentHandoff)
    }

    /// 附加管线绑定反射面(声明-反射双向相等核验开启)。
    #[must_use]
    pub fn with_reflection(mut self, resources: Vec<ResourceId>) -> PassSpec {
        self.reflection = Some(resources);
        self
    }
}

/// 资源描述(类别 + 创建意图初态 + 诊断名)。
#[derive(Debug, Clone)]
struct ResourceDesc {
    class: ResourceClass,
    /// 创建意图初态:attachment 创建即处写态(首写不发 barrier);buffer 创建即 `COMMON`。
    initial: D3d12State,
    name: String,
}

// ── barrier 计划(推导产物)──────────────────────────────────────────────────────────

/// barrier 形态(D3:三种,不把 buffer/UAV 硬套 image 迁移模型)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierForm {
    /// image/attachment 状态/layout 迁移(D3D12 states / Vulkan layout)。
    Transition,
    /// buffer 同步(无 layout;仅 stage+access)。
    BufferSync,
    /// 同资源相邻 UAV 写-写/写-读(D3D12 UAV barrier / Vulkan memory barrier)。
    UavSync,
}

/// 一条确定性 barrier(推导产物;执行器逐字重放,禁二次推导)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedBarrier {
    /// 目标资源。
    pub resource: ResourceId,
    /// 资源诊断名(D6 互证映射 / diag 用,非物理布局)。
    pub resource_name: String,
    /// barrier 形态。
    pub form: BarrierForm,
    /// D3D12 前态(逐字重放至 shim 数值透传)。
    pub d3d12_before: D3d12State,
    /// D3D12 后态。
    pub d3d12_after: D3d12State,
    /// Vulkan 前 layout(`BufferSync`/`UavSync` = `UNDEFINED`,无 layout)。
    pub vk_old_layout: u32,
    /// Vulkan 后 layout。
    pub vk_new_layout: u32,
    /// Vulkan 源 stage 掩码。
    pub vk_src_stage: u32,
    /// Vulkan 目的 stage 掩码。
    pub vk_dst_stage: u32,
    /// Vulkan 源 access 掩码。
    pub vk_src_access: u32,
    /// Vulkan 目的 access 掩码。
    pub vk_dst_access: u32,
    /// 该 barrier 录制于第 `at_pass` 个 pass 的边界之前(执行器编排锚点)。
    pub at_pass: usize,
}

// ── 图错误(装配期 strict,RX6029/RX6030)────────────────────────────────────────────

/// 图装配期错误(装配期确定性核验,strict-only;RFC-0013 §4.D2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// **RX6029** 图结构违例族(环/读未写/写写冲突/生命周期误用,§4.D2)。
    Structure {
        /// 诊断详情。
        detail: String,
    },
    /// **RX6030** 声明-反射失配族(漏声明/声明未用,§4.D2;相等域 = 首期封闭枚举资源面)。
    ReflectionMismatch {
        /// 诊断详情。
        detail: String,
    },
}

impl GraphError {
    /// 关联 RX 码(装配期 6xxx;error_codes.json 单一事实源)。
    #[must_use]
    pub fn rx_code(&self) -> &'static str {
        match self {
            GraphError::Structure { .. } => "RX6029",
            GraphError::ReflectionMismatch { .. } => "RX6030",
        }
    }
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::Structure { detail } => write!(f, "graph structure violation: {detail}"),
            GraphError::ReflectionMismatch { detail } => {
                write!(f, "graph declaration/reflection mismatch: {detail}")
            }
        }
    }
}

impl std::error::Error for GraphError {}

/// 图装配 `Result`(装配期错误 = 编译诊断段位 6xxx,承 RFC-0006 §9 Q-Err 先例)。
pub type Result<T> = std::result::Result<T, GraphError>;

// ── Graph 本体 ─────────────────────────────────────────────────────────────────────

/// render graph host 本体:资源表 + 声明序 pass 序列 + 装配核验 + 状态推导。
///
/// 生命周期:建面(`color_target`/`depth_target`/... + `add_pass`/`readback`)→ `seal()`(装配
/// 核验,一次性)→ `derive_barriers()`(纯函数推导)。`execute()` = seal + derive + 生命周期封口
/// (二次 execute → RX6029)。零 GPU 依赖:执行归 `vk.rs run_graph` / uc04 D3D12 shim。
#[derive(Debug, Clone, Default)]
pub struct Graph {
    resources: Vec<ResourceDesc>,
    passes: Vec<PassSpec>,
    sealed: bool,
    executed: bool,
}

impl Graph {
    /// 新建空图。
    #[must_use]
    pub fn new() -> Graph {
        Graph::default()
    }

    fn add_resource(
        &mut self,
        class: ResourceClass,
        initial: D3d12State,
        name: &str,
    ) -> ResourceId {
        let id = ResourceId(u32::try_from(self.resources.len()).unwrap_or(u32::MAX));
        self.resources.push(ResourceDesc {
            class,
            initial,
            name: name.to_owned(),
        });
        id
    }

    /// 分配 color target(image,创建即 `RENDER_TARGET`)。
    pub fn color_target(&mut self, name: &str) -> ResourceId {
        self.add_resource(ResourceClass::Image, D3d12State::RenderTarget, name)
    }

    /// 分配 depth target(image,创建即 `DEPTH_WRITE`)。
    pub fn depth_target(&mut self, name: &str) -> ResourceId {
        self.add_resource(ResourceClass::Image, D3d12State::DepthWrite, name)
    }

    /// 分配 UAV buffer(buffer,创建即 `COMMON`)。
    pub fn uav_buffer(&mut self, name: &str) -> ResourceId {
        self.add_resource(ResourceClass::Buffer, D3d12State::Common, name)
    }

    /// 分配 readback 目的 buffer(buffer,创建即 `COMMON`)。
    pub fn readback_buffer(&mut self, name: &str) -> ResourceId {
        self.add_resource(ResourceClass::Buffer, D3d12State::Common, name)
    }

    /// 追加一个 pass(声明序 = 提交序)。seal 后追加 → RX6029(生命周期)。
    ///
    /// # Errors
    /// seal 后追加 pass → [`GraphError::Structure`](RX6029)。
    pub fn add_pass(&mut self, pass: PassSpec) -> Result<()> {
        if self.sealed {
            return Err(GraphError::Structure {
                detail: format!("seal 后追加 pass `{}`(生命周期误用)", pass.name),
            });
        }
        self.passes.push(pass);
        Ok(())
    }

    /// 追加 readback pass(源 `CopySrcReadback` + 目的 buffer `CopyDstReadback`;§3.4 顶层胶水)。
    ///
    /// # Errors
    /// seal 后追加 → [`GraphError::Structure`](RX6029)。
    pub fn readback(&mut self, src: ResourceId, dst: ResourceId) -> Result<()> {
        self.add_pass(
            PassSpec::new("readback")
                .with(src, AccessKind::CopySrcReadback)
                .with(dst, AccessKind::CopyDstReadback),
        )
    }

    fn resource_name(&self, id: ResourceId) -> String {
        self.resources
            .get(id.0 as usize)
            .map_or_else(|| format!("res#{}", id.0), |r| r.name.clone())
    }

    /// 装配期确定性核验(strict-only,一次性)。违例 → RX6029/RX6030。
    ///
    /// 核验:① 空图 / 二次 seal(生命周期)② per-pass 同资源多次声明(写写/读写冲突)
    /// ③ 读未写(环 = use-before-write 可达形态)④ 声明-反射双向精确相等(有反射时)。
    ///
    /// # Errors
    /// 见 [`GraphError`]:图结构违例 → RX6029;声明-反射失配 → RX6030。
    pub fn seal(&mut self) -> Result<()> {
        if self.sealed {
            return Err(GraphError::Structure {
                detail: "重复 seal(生命周期误用)".to_owned(),
            });
        }
        if self.passes.is_empty() {
            return Err(GraphError::Structure {
                detail: "空图 seal/execute(生命周期误用)".to_owned(),
            });
        }

        // 已被写过的资源集(声明全序推进;读未写判据)。
        let mut written: BTreeSet<u32> = BTreeSet::new();

        for pass in &self.passes {
            // ② per-pass 同资源多次声明 = 写写冲突 / 读写冲突(reads_writes_uav 为唯一合法读写合并,
            //    以单条 UavReadWrite 表达 → 每资源每 pass 至多一条声明)。
            let mut seen: BTreeSet<u32> = BTreeSet::new();
            for a in &pass.accesses {
                if !seen.insert(a.resource.0) {
                    return Err(GraphError::Structure {
                        detail: format!(
                            "pass `{}` 对资源 `{}` 多次声明访问(写写/读写冲突;读写合并须用 reads_writes_uav)",
                            pass.name,
                            self.resource_name(a.resource)
                        ),
                    });
                }
            }

            // ③ 读未写(环/读未写可达形态):消费读须有先前 pass 的写。
            for a in &pass.accesses {
                if a.kind.is_consuming_read() && !written.contains(&a.resource.0) {
                    return Err(GraphError::Structure {
                        detail: format!(
                            "pass `{}` 读资源 `{}` 但无先前 pass 写入(读未写 / use-before-write 可达环形态)",
                            pass.name,
                            self.resource_name(a.resource)
                        ),
                    });
                }
            }

            // ④ 声明-反射双向精确相等(有反射时;相等域 = 首期封闭枚举资源面)。
            if let Some(refl) = &pass.reflection {
                let declared: BTreeSet<u32> = pass.accesses.iter().map(|a| a.resource.0).collect();
                let reflected: BTreeSet<u32> = refl.iter().map(|r| r.0).collect();
                if declared != reflected {
                    let missing: Vec<u32> = reflected.difference(&declared).copied().collect();
                    let unused: Vec<u32> = declared.difference(&reflected).copied().collect();
                    return Err(GraphError::ReflectionMismatch {
                        detail: format!(
                            "pass `{}` 声明-反射失配:漏声明(反射有声明无)={missing:?} / 声明未用(声明有反射无)={unused:?}",
                            pass.name
                        ),
                    });
                }
            }

            // 本 pass 的写更新 written 集(供后续 pass 的读未写判据)。
            for a in &pass.accesses {
                if a.kind.is_write() {
                    written.insert(a.resource.0);
                }
            }
        }

        self.sealed = true;
        Ok(())
    }

    /// 自动资源状态推导(D3,**纯函数**)。输入 = 已 seal 图;输出 = 确定性 barrier 计划:
    /// 逐资源状态机沿声明全序推进,下一使用点所需状态 ≠ 当前状态即在该 pass 边界产出一条转换;
    /// 同资源相邻 UAV 读写(状态不变)产出 [`BarrierForm::UavSync`]。同图 → 逐字节相同计划。
    ///
    /// 调用前须 `seal()`(未 seal → 返回空计划;`execute()` 走完整生命周期)。
    #[must_use]
    pub fn derive_barriers(&self) -> Vec<PlannedBarrier> {
        if !self.sealed {
            return Vec::new();
        }
        // 逐资源当前状态 + 上一访问种类(UAV hazard 判据)。
        let mut cur: Vec<D3d12State> = self.resources.iter().map(|r| r.initial).collect();
        let mut last_kind: Vec<Option<AccessKind>> = vec![None; self.resources.len()];
        let mut plan = Vec::new();

        for (pass_idx, pass) in self.passes.iter().enumerate() {
            for a in &pass.accesses {
                let ridx = a.resource.0 as usize;
                let Some(desc) = self.resources.get(ridx) else {
                    continue;
                };
                let required = a.kind.d3d12_state();
                let current = cur[ridx];
                if required != current {
                    // 状态迁移:image → Transition;buffer → BufferSync。
                    let form = match desc.class {
                        ResourceClass::Image => BarrierForm::Transition,
                        ResourceClass::Buffer => BarrierForm::BufferSync,
                    };
                    let (old_layout, new_layout) = match desc.class {
                        ResourceClass::Image => (state_vk_layout(current), a.kind.vk_layout()),
                        // buffer 无 layout。
                        ResourceClass::Buffer => (vk_layout::UNDEFINED, vk_layout::UNDEFINED),
                    };
                    plan.push(PlannedBarrier {
                        resource: a.resource,
                        resource_name: desc.name.clone(),
                        form,
                        d3d12_before: current,
                        d3d12_after: required,
                        vk_old_layout: old_layout,
                        vk_new_layout: new_layout,
                        vk_src_stage: last_kind[ridx]
                            .map_or(vk_stage::TOP_OF_PIPE, AccessKind::vk_stage),
                        vk_dst_stage: a.kind.vk_stage(),
                        vk_src_access: last_kind[ridx].map_or(0, AccessKind::vk_access),
                        vk_dst_access: a.kind.vk_access(),
                        at_pass: pass_idx,
                    });
                    cur[ridx] = required;
                } else if a.kind == AccessKind::UavReadWrite
                    && last_kind[ridx] == Some(AccessKind::UavReadWrite)
                {
                    // 同资源相邻 UAV 读写(状态不变):写-写/写-读 hazard → UavSync(D3D12 UAV barrier /
                    // Vulkan memory barrier)。
                    plan.push(PlannedBarrier {
                        resource: a.resource,
                        resource_name: desc.name.clone(),
                        form: BarrierForm::UavSync,
                        d3d12_before: current,
                        d3d12_after: required,
                        vk_old_layout: vk_layout::UNDEFINED,
                        vk_new_layout: vk_layout::UNDEFINED,
                        vk_src_stage: AccessKind::UavReadWrite.vk_stage(),
                        vk_dst_stage: AccessKind::UavReadWrite.vk_stage(),
                        vk_src_access: AccessKind::UavReadWrite.vk_access(),
                        vk_dst_access: AccessKind::UavReadWrite.vk_access(),
                        at_pass: pass_idx,
                    });
                }
                last_kind[ridx] = Some(a.kind);
            }
        }
        plan
    }

    /// 完整装配生命周期:seal(如未 seal)+ 推导 + 生命周期封口(二次 execute → RX6029)。
    /// 返回确定性 barrier 计划,供执行器逐字重放。
    ///
    /// # Errors
    /// 图结构违例 → RX6029;声明-反射失配 → RX6030;二次 execute → RX6029。
    pub fn execute(&mut self) -> Result<Vec<PlannedBarrier>> {
        if self.executed {
            return Err(GraphError::Structure {
                detail: "重复 execute(生命周期误用)".to_owned(),
            });
        }
        if !self.sealed {
            self.seal()?;
        }
        let plan = self.derive_barriers();
        self.executed = true;
        Ok(plan)
    }

    /// pass 数(执行器录制用)。
    #[must_use]
    pub fn pass_count(&self) -> usize {
        self.passes.len()
    }

    /// 资源数。
    #[must_use]
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }
}

/// D3D12 状态 → 该状态对应的 Vulkan image layout(推导内部:前态 layout 复算)。
fn state_vk_layout(s: D3d12State) -> u32 {
    match s {
        D3d12State::Common => vk_layout::UNDEFINED,
        D3d12State::RenderTarget => vk_layout::COLOR_ATTACHMENT_OPTIMAL,
        D3d12State::DepthWrite => vk_layout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        D3d12State::PixelShaderResource => vk_layout::SHADER_READ_ONLY_OPTIMAL,
        D3d12State::UnorderedAccess => vk_layout::GENERAL,
        D3d12State::CopySource => vk_layout::TRANSFER_SRC_OPTIMAL,
        D3d12State::CopyDest => vk_layout::TRANSFER_DST_OPTIMAL,
        D3d12State::Present => vk_layout::PRESENT_SRC_KHR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造 uc04 deferred 三 pass 图(§3.4 结构;depth 建模为 color target 以镜像 uc04
    /// 冻结 oracle 的统一 `RenderTarget` 处置——`barrier.rs` `ResourceState` 无 depth 变体,
    /// depth 的 `DEPTH_WRITE` 路由由独立单测覆盖,见 `derives_depth_write_transition`)。
    fn deferred_graph() -> Graph {
        let mut g = Graph::new();
        let albedo = g.color_target("gbuf:Albedo");
        let normal = g.color_target("gbuf:Normal");
        let depth = g.color_target("gbuf:Depth");
        let lit = g.color_target("lighting_out");
        let readback = g.readback_buffer("readback");
        g.add_pass(
            PassSpec::new("geometry")
                .writes_rt(albedo)
                .writes_rt(normal)
                .writes_rt(depth),
        )
        .unwrap();
        g.add_pass(
            PassSpec::new("lighting")
                .reads(albedo)
                .reads(normal)
                .reads(depth)
                .writes_rt(lit),
        )
        .unwrap();
        g.readback(lit, readback).unwrap();
        g
    }

    /// 推导计划 golden:deferred 三 pass 图产出恰 5 条 barrier,逐条锚定(RXS-0238)。
    //@ spec: RXS-0238
    #[test]
    fn derives_deferred_golden_plan() {
        let mut g = deferred_graph();
        let plan = g.execute().expect("合法图应 execute 通过");
        assert_eq!(plan.len(), 5, "deferred 图应恰 5 条 barrier");

        // 3 条 G-buffer RT→PSR + lighting_out RT→CopySource + readback Common→CopyDest。
        let rt_psr = plan
            .iter()
            .filter(|b| {
                b.d3d12_before == D3d12State::RenderTarget
                    && b.d3d12_after == D3d12State::PixelShaderResource
            })
            .count();
        assert_eq!(rt_psr, 3, "3 条 G-buffer RT→PSR");
        assert!(plan.iter().any(|b| b.resource_name == "lighting_out"
            && b.d3d12_before == D3d12State::RenderTarget
            && b.d3d12_after == D3d12State::CopySource));
        let rb = plan
            .iter()
            .find(|b| b.resource_name == "readback")
            .expect("readback barrier");
        assert_eq!(rb.d3d12_before, D3d12State::Common);
        assert_eq!(rb.d3d12_after, D3d12State::CopyDest);
        assert_eq!(
            rb.form,
            BarrierForm::BufferSync,
            "buffer → BufferSync(无 layout)"
        );
    }

    /// 推导纯函数确定性:同图两次推导逐字节相同(golden 可锚,RXS-0238)。
    //@ spec: RXS-0238
    #[test]
    fn derivation_is_deterministic() {
        let g1 = {
            let mut g = deferred_graph();
            g.seal().unwrap();
            g
        };
        let g2 = {
            let mut g = deferred_graph();
            g.seal().unwrap();
            g
        };
        assert_eq!(g1.derive_barriers(), g2.derive_barriers());
        // 同一图两次推导亦相同。
        assert_eq!(g1.derive_barriers(), g1.derive_barriers());
    }

    /// 双后端映射同源:每 AccessKind 的 D3D12 / Vulkan 映射为单一事实源(RXS-0240)。
    //@ spec: RXS-0240
    #[test]
    fn access_kind_mapping_single_source() {
        assert_eq!(
            AccessKind::ColorAttachmentWrite.d3d12_state(),
            D3d12State::RenderTarget
        );
        assert_eq!(
            AccessKind::ColorAttachmentWrite.vk_layout(),
            vk_layout::COLOR_ATTACHMENT_OPTIMAL
        );
        assert_eq!(
            AccessKind::ShaderRead.vk_layout(),
            vk_layout::SHADER_READ_ONLY_OPTIMAL
        );
        assert_eq!(
            AccessKind::CopySrcReadback.d3d12_state(),
            D3d12State::CopySource
        );
        assert_eq!(
            AccessKind::PresentHandoff.vk_layout(),
            vk_layout::PRESENT_SRC_KHR
        );
        // u32 tag round-trip(cabi 下发)。
        for k in [
            AccessKind::ColorAttachmentWrite,
            AccessKind::DepthAttachmentWrite,
            AccessKind::ShaderRead,
            AccessKind::UavReadWrite,
            AccessKind::CopySrcReadback,
            AccessKind::CopyDstReadback,
            AccessKind::PresentHandoff,
        ] {
            assert_eq!(AccessKind::from_u32(k.as_u32()), Some(k));
        }
        assert_eq!(AccessKind::from_u32(99), None);
    }

    /// depth 独立路由:`writes_depth` → `DEPTH_WRITE`,读转换 `DEPTH_WRITE→PSR`(RXS-0238)。
    //@ spec: RXS-0238
    #[test]
    fn derives_depth_write_transition() {
        let mut g = Graph::new();
        let depth = g.depth_target("depth");
        let lit = g.color_target("lit");
        g.add_pass(PassSpec::new("geo").writes_depth(depth))
            .unwrap();
        g.add_pass(PassSpec::new("light").reads(depth).writes_rt(lit))
            .unwrap();
        let plan = g.execute().unwrap();
        let d = plan
            .iter()
            .find(|b| b.resource_name == "depth")
            .expect("depth barrier");
        assert_eq!(d.d3d12_before, D3d12State::DepthWrite);
        assert_eq!(d.d3d12_after, D3d12State::PixelShaderResource);
        assert_eq!(d.vk_old_layout, vk_layout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    }

    /// UAV 相邻读写 hazard → UavSync barrier(状态不变仍发,RXS-0238)。
    //@ spec: RXS-0238
    #[test]
    fn derives_uav_sync_between_adjacent_uav_passes() {
        let mut g = Graph::new();
        let buf = g.uav_buffer("uav");
        let out = g.color_target("out");
        g.add_pass(PassSpec::new("passA").reads_writes_uav(buf))
            .unwrap();
        g.add_pass(PassSpec::new("passB").reads_writes_uav(buf).writes_rt(out))
            .unwrap();
        let plan = g.execute().unwrap();
        // passA:Common→UnorderedAccess(BufferSync);passB:同态 UAV hazard(UavSync)。
        assert!(plan.iter().any(|b| b.form == BarrierForm::BufferSync
            && b.d3d12_before == D3d12State::Common
            && b.d3d12_after == D3d12State::UnorderedAccess));
        assert!(
            plan.iter().any(|b| b.form == BarrierForm::UavSync),
            "相邻 UAV pass 须发 UavSync"
        );
    }

    // ── RX6029 图结构违例族 ×4 + RX6030 声明-反射失配(RXS-0237)────────────────────

    /// reject:读未写(环/use-before-write 可达形态)→ RX6029(RXS-0237)。
    //@ spec: RXS-0237
    #[test]
    fn rejects_read_before_write() {
        let mut g = Graph::new();
        let a = g.color_target("a");
        let out = g.color_target("out");
        // lighting 读 a,但无先前 pass 写 a。
        g.add_pass(PassSpec::new("light").reads(a).writes_rt(out))
            .unwrap();
        match g.seal() {
            Err(e @ GraphError::Structure { .. }) => assert_eq!(e.rx_code(), "RX6029"),
            other => panic!("读未写应 RX6029,实得 {other:?}"),
        }
    }

    /// reject:同 pass 对同资源写写冲突(重复 writes_rt)→ RX6029(RXS-0237)。
    //@ spec: RXS-0237
    #[test]
    fn rejects_write_write_conflict() {
        let mut g = Graph::new();
        let a = g.color_target("a");
        g.add_pass(PassSpec::new("geo").writes_rt(a).writes_rt(a))
            .unwrap();
        assert!(matches!(g.seal(), Err(GraphError::Structure { .. })));
    }

    /// reject:同 pass 既 reads 又 writes_rt 同资源(feedback 读写冲突)→ RX6029(RXS-0237)。
    //@ spec: RXS-0237
    #[test]
    fn rejects_read_write_same_pass() {
        let mut g = Graph::new();
        let a = g.color_target("a");
        g.add_pass(PassSpec::new("geo").writes_rt(a)).unwrap();
        // 后续 pass 同资源既读又写 = feedback → RX6029(reads_writes_uav 才是唯一合法读写合并)。
        g.add_pass(PassSpec::new("bad").reads(a).writes_rt(a))
            .unwrap();
        assert!(matches!(g.seal(), Err(GraphError::Structure { .. })));
    }

    /// reject:空图 / 生命周期误用(二次 seal / seal 后追加 / 二次 execute)→ RX6029(RXS-0237)。
    //@ spec: RXS-0237
    #[test]
    fn rejects_lifecycle_misuse() {
        // 空图。
        let mut empty = Graph::new();
        assert!(matches!(empty.seal(), Err(GraphError::Structure { .. })));

        // seal 后追加 pass。
        let mut g = deferred_graph();
        g.seal().unwrap();
        let extra = PassSpec::new("extra");
        assert!(matches!(
            g.add_pass(extra),
            Err(GraphError::Structure { .. })
        ));
        // 二次 seal。
        assert!(matches!(g.seal(), Err(GraphError::Structure { .. })));

        // 二次 execute。
        let mut g2 = deferred_graph();
        g2.execute().unwrap();
        assert!(matches!(g2.execute(), Err(GraphError::Structure { .. })));
    }

    /// reject:声明-反射双向失配(漏声明 / 声明未用)→ RX6030(RXS-0237)。
    //@ spec: RXS-0237
    #[test]
    fn rejects_reflection_mismatch() {
        let mut g = Graph::new();
        let a = g.color_target("a");
        let b = g.color_target("b");
        // 声明只写 a;反射面含 a+b(漏声明 b)→ RX6030。
        g.add_pass(
            PassSpec::new("geo")
                .writes_rt(a)
                .with_reflection(vec![a, b]),
        )
        .unwrap();
        match g.seal() {
            Err(e @ GraphError::ReflectionMismatch { .. }) => assert_eq!(e.rx_code(), "RX6030"),
            other => panic!("声明-反射失配应 RX6030,实得 {other:?}"),
        }
    }

    /// accept:声明-反射精确相等 → 通过(RXS-0237)。
    //@ spec: RXS-0237
    #[test]
    fn accepts_reflection_exact_match() {
        let mut g = Graph::new();
        let a = g.color_target("a");
        let b = g.color_target("b");
        g.add_pass(
            PassSpec::new("geo")
                .writes_rt(a)
                .writes_rt(b)
                .with_reflection(vec![b, a]),
        )
        .unwrap();
        assert!(g.seal().is_ok());
    }
}
