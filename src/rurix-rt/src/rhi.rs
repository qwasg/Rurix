#![forbid(unsafe_code)]
//! RXS-0256 ~ RXS-0260:UC-05 最小 RHI 纯 host 图合法性核验与自动资源状态推导
//! (EI1.3 Part B,RFC-0014 §4.B)。
//!
//! **定位**:声明式宿主库面(compute-pass render graph 核心)的 host 侧本体——图合法性
//! 装配核验(I3 依赖环 / I4 未声明访问 / I5 写写冲突)+ 自动资源状态推导(纯函数,同图
//! → 逐字节相同 hazard 计划)。本模块 **always-on、零 unsafe、零后端调用、无 GPU 依赖**
//! (`#![forbid(unsafe_code)]` 编译期封口);推导为纯函数,可 golden 锚。
//!
//! **与 G3.5 `graph.rs` 的关系**:平行的不同面——`graph.rs` 是 G3 图形面 render graph
//! (color/depth attachment + 双后端 barrier 映射);本模块是 UC-05 库面 compute-pass RHI
//! (UAV 读写 hazard 推导)。`graph.rs` 仅**设计参照**(状态推导 / 依赖建序思路),非代码复用
//! (RFC-0014 §7-2)。[`AccessKind`] 复用 `graph.rs` `ShaderRead`/`UavReadWrite` 的读/写语义
//! 但收敛为 compute 二元封闭枚举。
//!
//! **错误口径**:承诺面之外的构造走装配期(`submit()` 装配期)确定性 strict 拒——**库层状态
//! 值零新 RX 码**(I3/I5 镜像 G3.5 RX6029 口径,I4 镜像 RX6030 口径);运行期后端失败走确定性
//! 诊断 + 终止 + poisoned 传播(RXS-0193/0194)。图/RHI 本身零新语言机制(薄映射 std::gpu,
//! affine/brand/typestate 复用既有裁决)。
//!
//! **I4 未声明访问**:reflected 集由**编译器 typeck/构建期喂入**([`PassSpec::with_reflection`],
//! 镜像 `graph.rs::with_reflection`);`.rx` 无运行期反射(RD-026),故声明-反射相等核验计入
//! 语言/编译器面(仍零新 RX 码)。`None` = 纯 host 推导不要求反射(golden / 单测场景)。

use std::collections::BTreeSet;

// ── AccessKind 封闭枚举(compute UAV 面,单一事实源)──────────────────────────────────

/// 访问声明的封闭枚举——compute-pass 面「读 / 写」二元(RFC-0014 §4.B2)。C ABI 下发用稳定
/// u32 tag(`rxrt_rhi_declare` 的 `access` 参数;含义冻结,只追加)。
///
/// 收敛自 `graph.rs` [`crate::graph::AccessKind`] 的读/写语义:[`AccessKind::Read`] ≙ `ShaderRead`
/// (消费读,要求先前 pass 写过);[`AccessKind::Write`] ≙ `UavReadWrite` 的写侧(建立/更新内容)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AccessKind {
    /// `pass.reads(&res)`:shader/UAV 资源读(消费读——要求同资源已被先前 pass 写)。
    Read,
    /// `pass.writes(&res)`:UAV 资源写(建立/更新资源内容)。
    Write,
}

impl AccessKind {
    /// 该访问是否为「写」语义(建立/更新资源内容;推导的 read-before-write 判据用)。
    #[must_use]
    pub fn is_write(self) -> bool {
        matches!(self, AccessKind::Write)
    }

    /// 该访问是否为「消费读」语义(要求同资源已被先前 pass 写过;依赖环 I3 判据)。
    #[must_use]
    pub fn is_consuming_read(self) -> bool {
        matches!(self, AccessKind::Read)
    }

    /// C ABI / cabi 下发用的稳定 u32 tag(`rxrt_rhi_declare` 参数;含义冻结,只追加)。
    #[must_use]
    pub fn as_u32(self) -> u32 {
        match self {
            AccessKind::Read => 0,
            AccessKind::Write => 1,
        }
    }

    /// u32 tag → AccessKind(cabi 上行;未知 tag → `None`)。
    #[must_use]
    pub fn from_u32(v: u32) -> Option<AccessKind> {
        Some(match v {
            0 => AccessKind::Read,
            1 => AccessKind::Write,
            _ => return None,
        })
    }
}

// ── 资源与 pass 建面 ───────────────────────────────────────────────────────────────

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

/// pass 声明(访问集 + 可选 kernel 反射面)。
#[derive(Debug, Clone, Default)]
pub struct PassSpec {
    /// pass 诊断名。
    pub name: String,
    /// 访问声明集(声明序 = 提交序)。
    pub accesses: Vec<Access>,
    /// 可选 kernel 绑定反射面(编译器 typeck/构建期喂入;镜像 `graph.rs::with_reflection`):
    /// 存在时与声明集**双向精确相等**核验(I4);`None` = 纯 host 推导不要求反射。相等域 =
    /// compute 资源面(资源 id 集)。
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

    /// `reads(&res)`:shader/UAV 资源读。
    #[must_use]
    pub fn reads(self, res: ResourceId) -> PassSpec {
        self.with(res, AccessKind::Read)
    }

    /// `writes(&res)`:UAV 资源写。
    #[must_use]
    pub fn writes(self, res: ResourceId) -> PassSpec {
        self.with(res, AccessKind::Write)
    }

    /// 附加 kernel 绑定反射面(声明-反射双向相等核验开启,I4)。
    #[must_use]
    pub fn with_reflection(mut self, resources: Vec<ResourceId>) -> PassSpec {
        self.reflection = Some(resources);
        self
    }
}

// ── hazard 计划(推导产物)────────────────────────────────────────────────────────────

/// 跨 pass 危险类别(compute 单 queue 声明全序;推导本体)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hazard {
    /// 读依赖先前写(RAW:read-after-write;须在读前对生产者写完成建同步序)。
    ReadAfterWrite,
    /// 写覆盖先前写(WAW:write-after-write)。
    WriteAfterWrite,
    /// 写覆盖先前读(WAR:write-after-read)。
    WriteAfterRead,
}

/// 一条确定性同步(推导产物;执行器逐字重放,禁二次推导)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedSync {
    /// 目标资源。
    pub resource: ResourceId,
    /// 该同步录制于第 `at_pass` 个 pass 的边界之前(执行器编排锚点)。
    pub at_pass: usize,
    /// 危险类别(RAW/WAW/WAR)。
    pub hazard: Hazard,
}

// ── 图错误(装配期 strict;库层状态值零新码)────────────────────────────────────────

/// 图装配期错误(装配期确定性核验,strict-only;RFC-0014 §4.B)。**库层状态值,零新 RX 码**
/// (I3/I5 镜像 G3.5 RX6029 口径,I4 镜像 RX6030 口径;非语言诊断码)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RhiError {
    /// 图结构违例族(依赖环 I3 / 写写冲突 I5 / 生命周期误用)。
    Structure {
        /// 诊断详情。
        detail: String,
    },
    /// 声明-反射失配族(漏声明 / 声明未用,I4;相等域 = compute 资源面)。
    ReflectionMismatch {
        /// 诊断详情。
        detail: String,
    },
}

impl RhiError {
    /// 库层错误类别标签(cabi 诊断行前缀;非语言 RX 码,零新码纪律)。
    #[must_use]
    pub fn category(&self) -> &'static str {
        match self {
            RhiError::Structure { .. } => "structure",
            RhiError::ReflectionMismatch { .. } => "reflection",
        }
    }
}

impl std::fmt::Display for RhiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RhiError::Structure { detail } => write!(f, "rhi graph structure violation: {detail}"),
            RhiError::ReflectionMismatch { detail } => {
                write!(f, "rhi declaration/reflection mismatch: {detail}")
            }
        }
    }
}

impl std::error::Error for RhiError {}

/// 图装配 `Result`(装配期错误 = 库层状态值)。
pub type Result<T> = std::result::Result<T, RhiError>;

// ── 图形 RHI 面(G4.2,RXS-0270~0273)──────────────────────────────────────────────────

/// 图形资源种类(RXS-0271;cabi `rxrt_rhi_resource` 类枚举 0~5 的 host 侧映射)。
///
/// `color_target`/`depth_target`/`texture2d` 为**资源状态访问**类(参 barrier 推导);
/// `sampler`/`texture_table` 为**无状态访问**类(RXS-0273:只核绑定完备性,不参 barrier 状态)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxResourceKind {
    /// `color_target(w,h)`:RGBA8 color attachment(transient image)。
    Color,
    /// `depth_target(w,h)`:D32F depth attachment(transient image)。
    Depth,
    /// `texture2d(w,h)`:RGBA8 可采样纹理(SRV image)。
    Texture,
    /// `sampler(desc)`:采样器状态对象(无资源状态)。
    Sampler,
    /// `texture_table()`:无界纹理注册表(bindless,无资源状态;RXS-0276)。
    Table,
}

impl GfxResourceKind {
    /// 是否为资源状态访问类(参 barrier 推导;RXS-0273 barrier 相等域)。
    #[must_use]
    pub fn has_state(self) -> bool {
        matches!(
            self,
            GfxResourceKind::Color | GfxResourceKind::Depth | GfxResourceKind::Texture
        )
    }
}

/// 图形 pass 阶段(RXS-0270):raster(vertex+fragment)或 mesh(mesh+fragment)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxPassStage {
    /// `raster_pass(vs, fs)`:vertex + fragment 着色对。
    Raster,
    /// `mesh_pass(ms, fs)`:mesh + fragment 着色对(RXS-0243 入口契约)。
    Mesh,
}

/// 图形 pass 记录(RXS-0270/0272;桥接 `graph.rs::PassSpec` 的 RHI 侧建面)。
///
/// 资源状态访问声明(`writes_rt`/`writes_depth`/`reads`/`reads_writes_uav`/`present`)经
/// [`crate::graph::AccessKind`] 单源表达(封闭枚举镜像 RXS-0236);sampler/table 绑定声明
/// 另记 [`GfxPassRecord::bindings`](无资源状态,RXS-0273)。seal 时桥接 `graph.rs::Graph`。
#[derive(Debug, Clone)]
pub struct GfxPassRecord {
    /// pass 诊断名。
    pub name: String,
    /// pass 阶段(raster / mesh)。
    pub stage: GfxPassStage,
    /// 资源状态访问声明(bridge to `graph.rs::AccessKind` single source;RXS-0272)。
    pub accesses: Vec<crate::graph::Access>,
    /// 采样器/table 绑定声明(无资源状态;RXS-0273 绑定完备性核验)。
    pub bindings: Vec<ResourceId>,
    /// 可选反射面(RXS-0273 声明↔反射双向相等;含 sampler/table 但标「无状态访问」类)。
    pub reflection: Option<Vec<ResourceId>>,
    /// present 终端声明(RXS-0272:每图 ≤1,且必须为声明序末 pass)。
    pub present: Option<ResourceId>,
}

impl GfxPassRecord {
    /// 新建具名图形 pass。
    #[must_use]
    pub fn new(name: &str, stage: GfxPassStage) -> GfxPassRecord {
        GfxPassRecord {
            name: name.to_owned(),
            stage,
            accesses: Vec::new(),
            bindings: Vec::new(),
            reflection: None,
            present: None,
        }
    }

    /// `writes_rt(&res)`:color attachment 写(RXS-0272)。
    #[must_use]
    pub fn writes_rt(mut self, res: ResourceId) -> GfxPassRecord {
        self.accesses.push(crate::graph::Access {
            resource: crate::graph::ResourceId(res.0),
            kind: crate::graph::AccessKind::ColorAttachmentWrite,
        });
        self
    }

    /// `writes_depth(&res)`:depth attachment 写(RXS-0272)。
    #[must_use]
    pub fn writes_depth(mut self, res: ResourceId) -> GfxPassRecord {
        self.accesses.push(crate::graph::Access {
            resource: crate::graph::ResourceId(res.0),
            kind: crate::graph::AccessKind::DepthAttachmentWrite,
        });
        self
    }

    /// `reads(&res)`:shader 资源读(含采样纹理;RXS-0272)。
    #[must_use]
    pub fn reads(mut self, res: ResourceId) -> GfxPassRecord {
        self.accesses.push(crate::graph::Access {
            resource: crate::graph::ResourceId(res.0),
            kind: crate::graph::AccessKind::ShaderRead,
        });
        self
    }

    /// `reads_writes_uav(&res)`:UAV 读写合并(RXS-0272)。
    #[must_use]
    pub fn reads_writes_uav(mut self, res: ResourceId) -> GfxPassRecord {
        self.accesses.push(crate::graph::Access {
            resource: crate::graph::ResourceId(res.0),
            kind: crate::graph::AccessKind::UavReadWrite,
        });
        self
    }

    /// `binds_sampler(&res)`:采样器/table 绑定声明(非资源状态访问;RXS-0272/0273)。
    #[must_use]
    pub fn binds_sampler(mut self, res: ResourceId) -> GfxPassRecord {
        self.bindings.push(res);
        self
    }

    /// `present(&res)`:呈现终端 handoff(RXS-0272:每图 ≤1,且必须为声明序末 pass)。
    #[must_use]
    pub fn present(mut self, res: ResourceId) -> GfxPassRecord {
        self.present = Some(res);
        self
    }

    /// 附加反射面(RXS-0273 声明↔反射双向相等核验开启;含 sampler/table)。
    #[must_use]
    pub fn with_reflection(mut self, resources: Vec<ResourceId>) -> GfxPassRecord {
        self.reflection = Some(resources);
        self
    }
}

// ── RhiGraph 本体 ──────────────────────────────────────────────────────────────────

/// UC-05 RHI host 本体:资源表 + 声明序 pass 序列 + 装配核验 + hazard 推导。
///
/// 生命周期:建面(`resource` + `add_pass`)→ `seal()`(装配核验,一次性)→ `derive_syncs()`
/// (纯函数推导)。`execute()` = seal + derive + 生命周期封口(二次 execute → Structure)。
/// 零 GPU 依赖:执行(真实 barrier/dispatch)归 engine_host 侧,本模块只产计划。
///
/// **G4.2 图形 RHI 扩面(RXS-0270~0273)**:图形资源(`color_target`/`depth_target`/
/// `texture2d`/`sampler`/`texture_table`)与图形 pass(`raster_pass`/`mesh_pass`)经
/// `seal()` 桥接 `graph.rs::Graph`(同 crate 直接构造,`AccessKind` 单源复用,无 cabi
/// marshalling)→ `derive_barriers()` 产 `PlannedBarrier`。compute-only 图维持
/// `derive_syncs()` CUDA 既有路 **0-byte**(无图形 pass/resource 时 gfx 桥接为 no-op)。
#[derive(Debug, Clone, Default)]
pub struct RhiGraph {
    /// 资源诊断名表(下标 = [`ResourceId`])。
    resources: Vec<String>,
    /// 声明序 pass 序列(声明序 = 提交序)。
    passes: Vec<PassSpec>,
    sealed: bool,
    executed: bool,
    /// 资源种类表(与 `resources` 平行;G4.2 图形资源面 RXS-0271)。
    /// 默认(compute UAV buffer)为 `None`;图形资源为 `Some(kind)`。
    resource_kinds: Vec<Option<GfxResourceKind>>,
    /// 图形 pass 序列(G4.2,RXS-0270;与 compute `passes` 并列,声明序 = 提交序)。
    gfx_passes: Vec<GfxPassRecord>,
    /// 图形 barrier 推导产物(`seal()` 桥接 `graph.rs::derive_barriers` 产;RXS-0272)。
    /// compute-only 图(无 gfx pass)为空——`derive_syncs` 0-byte 不受影响。
    gfx_barriers: Vec<crate::graph::PlannedBarrier>,
}

impl RhiGraph {
    /// 新建空图。
    #[must_use]
    pub fn new() -> RhiGraph {
        RhiGraph::default()
    }

    /// 分配一个 compute 资源(UAV buffer 面),返回稳定单调 [`ResourceId`]。
    pub fn resource(&mut self, name: &str) -> ResourceId {
        let id = ResourceId(u32::try_from(self.resources.len()).unwrap_or(u32::MAX));
        self.resources.push(name.to_owned());
        self.resource_kinds.push(None);
        id
    }

    /// 分配一个图形资源(RXS-0271),返回稳定单调 [`ResourceId`]。
    fn gfx_resource(&mut self, name: &str, kind: GfxResourceKind) -> ResourceId {
        let id = ResourceId(u32::try_from(self.resources.len()).unwrap_or(u32::MAX));
        self.resources.push(name.to_owned());
        self.resource_kinds.push(Some(kind));
        id
    }

    /// `color_target(w, h)`:RGBA8 color attachment(transient image;RXS-0271)。
    pub fn color_target(&mut self, name: &str, _w: u32, _h: u32) -> ResourceId {
        self.gfx_resource(name, GfxResourceKind::Color)
    }

    /// `depth_target(w, h)`:D32F depth attachment(transient image;RXS-0271)。
    pub fn depth_target(&mut self, name: &str, _w: u32, _h: u32) -> ResourceId {
        self.gfx_resource(name, GfxResourceKind::Depth)
    }

    /// `texture2d(w, h)`:RGBA8 可采样纹理(SRV image;RXS-0271)。
    pub fn texture2d(&mut self, name: &str, _w: u32, _h: u32) -> ResourceId {
        self.gfx_resource(name, GfxResourceKind::Texture)
    }

    /// `sampler(desc)`:采样器状态对象(无资源状态;RXS-0271)。
    pub fn sampler(&mut self, name: &str) -> ResourceId {
        self.gfx_resource(name, GfxResourceKind::Sampler)
    }

    /// `texture_table()`:无界纹理注册表(bindless,无资源状态;RXS-0271/0276)。
    pub fn texture_table(&mut self, name: &str) -> ResourceId {
        self.gfx_resource(name, GfxResourceKind::Table)
    }

    fn resource_name(&self, id: ResourceId) -> String {
        self.resources
            .get(id.0 as usize)
            .cloned()
            .unwrap_or_else(|| format!("res#{}", id.0))
    }

    /// 追加一个 pass(声明序 = 提交序)。seal 后追加 → Structure(生命周期误用)。
    ///
    /// # Errors
    /// seal 后追加 pass → [`RhiError::Structure`]。
    pub fn add_pass(&mut self, pass: PassSpec) -> Result<()> {
        if self.sealed {
            return Err(RhiError::Structure {
                detail: format!("seal 后追加 pass `{}`(生命周期误用)", pass.name),
            });
        }
        self.passes.push(pass);
        Ok(())
    }

    /// 追加一个图形 pass(G4.2,RXS-0270;声明序 = 提交序)。seal 后追加 → Structure。
    ///
    /// # Errors
    /// seal 后追加 pass → [`RhiError::Structure`]。
    pub fn add_gfx_pass(&mut self, pass: GfxPassRecord) -> Result<()> {
        if self.sealed {
            return Err(RhiError::Structure {
                detail: format!("seal 后追加 gfx pass `{}`(生命周期误用)", pass.name),
            });
        }
        self.gfx_passes.push(pass);
        Ok(())
    }

    /// 装配期确定性核验(strict-only,一次性)。违例 → [`RhiError`]。
    ///
    /// 核验:① 空图 / 二次 seal(生命周期)② per-pass 同资源多次声明(写写 / 读写冲突,I5)
    /// ③ 读未写(依赖环 = use-before-write 可达形态,I3)④ 声明-反射双向精确相等(有反射时,I4)
    /// ⑤ gfx present 唯一且末位(RXS-0272)⑥ gfx pass per-pass 同资源多次声明(I5)
    /// ⑦ gfx 声明↔反射双向相等(资源状态访问域;RXS-0273)⑧ gfx 桥接 graph.rs derive_barriers。
    ///
    /// # Errors
    /// 图结构违例(I3/I5/生命周期)→ [`RhiError::Structure`];声明-反射失配(I4)→
    /// [`RhiError::ReflectionMismatch`]。
    pub fn seal(&mut self) -> Result<()> {
        if self.sealed {
            return Err(RhiError::Structure {
                detail: "重复 seal(生命周期误用)".to_owned(),
            });
        }
        if self.passes.is_empty() && self.gfx_passes.is_empty() {
            return Err(RhiError::Structure {
                detail: "空图 seal/submit(生命周期误用)".to_owned(),
            });
        }

        // 已被写过的资源集(声明全序推进;读未写判据 I3)。
        let mut written: BTreeSet<u32> = BTreeSet::new();

        for pass in &self.passes {
            // ② per-pass 同资源多次声明 = 写写 / 读写冲突(I5;compute 面每资源每 pass 至多一条声明)。
            let mut seen: BTreeSet<u32> = BTreeSet::new();
            for a in &pass.accesses {
                if !seen.insert(a.resource.0) {
                    return Err(RhiError::Structure {
                        detail: format!(
                            "pass `{}` 对资源 `{}` 多次声明访问(写写 / 读写冲突,I5)",
                            pass.name,
                            self.resource_name(a.resource)
                        ),
                    });
                }
            }

            // ③ 读未写(依赖环 / use-before-write 可达形态,I3):消费读须有先前 pass 的写。
            for a in &pass.accesses {
                if a.kind.is_consuming_read() && !written.contains(&a.resource.0) {
                    return Err(RhiError::Structure {
                        detail: format!(
                            "pass `{}` 读资源 `{}` 但无先前 pass 写入(依赖环 / use-before-write,I3)",
                            pass.name,
                            self.resource_name(a.resource)
                        ),
                    });
                }
            }

            // ④ 声明-反射双向精确相等(有反射时,I4;相等域 = compute 资源面)。
            if let Some(refl) = &pass.reflection {
                let declared: BTreeSet<u32> = pass.accesses.iter().map(|a| a.resource.0).collect();
                let reflected: BTreeSet<u32> = refl.iter().map(|r| r.0).collect();
                if declared != reflected {
                    let missing: Vec<u32> = reflected.difference(&declared).copied().collect();
                    let unused: Vec<u32> = declared.difference(&reflected).copied().collect();
                    return Err(RhiError::ReflectionMismatch {
                        detail: format!(
                            "pass `{}` 声明-反射失配(I4):漏声明(反射有声明无)={missing:?} / \
                             声明未用(声明有反射无)={unused:?}",
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

        // ── G4.2 图形 pass 装配核验 + 桥接 graph.rs(RXS-0270~0273)──────────────────
        // compute-only 图(无 gfx_passes)跳过本段,gfx_barriers 维持空 → derive_syncs 0-byte。
        if !self.gfx_passes.is_empty() {
            self.seal_gfx(&mut written)?;
        }

        self.sealed = true;
        Ok(())
    }

    /// 图形 pass 装配核验 + 桥接 `graph.rs::Graph`(G4.2,RXS-0272/0273)。
    ///
    /// 核验:⑤ present 唯一且末位 ⑥ per-pass 同资源多次声明(I5)⑦ 声明↔反射双向相等
    /// (资源状态访问域 + sampler/table 绑定完备性)⑧ 桥接 `graph.rs::derive_barriers`。
    /// `written` 继承 compute pass 已写集(混合图 compute→gfx 跨阶段依赖)。
    fn seal_gfx(&mut self, written: &mut BTreeSet<u32>) -> Result<()> {
        use crate::graph;

        // ⑤ present 唯一性(RXS-0272:每图 ≤1)。
        let present_count = self
            .gfx_passes
            .iter()
            .filter(|p| p.present.is_some())
            .count();
        if present_count > 1 {
            return Err(RhiError::Structure {
                detail: format!("图含 {present_count} 个 present 终端(每图 ≤1,RXS-0272)"),
            });
        }
        // present 末位:若存在,必须在最后一个 gfx pass。
        if present_count == 1
            && self
                .gfx_passes
                .last()
                .map(|p| p.present.is_none())
                .unwrap_or(true)
        {
            return Err(RhiError::Structure {
                detail: "present 终端不在声明序末位(RXS-0272)".to_owned(),
            });
        }

        for pass in &self.gfx_passes {
            // ⑥ per-pass 同资源多次声明(I5;资源状态访问域)。
            let mut seen: BTreeSet<u32> = BTreeSet::new();
            for a in &pass.accesses {
                if !seen.insert(a.resource.0) {
                    return Err(RhiError::Structure {
                        detail: format!(
                            "gfx pass `{}` 对资源 `{}` 多次声明访问(写写 / 读写冲突,I5)",
                            pass.name,
                            self.resource_name(ResourceId(a.resource.0))
                        ),
                    });
                }
            }

            // ③ gfx 读未写(I3;含跨 compute→gfx 的 written 集继承)。
            for a in &pass.accesses {
                if a.kind.is_consuming_read() && !written.contains(&a.resource.0) {
                    return Err(RhiError::Structure {
                        detail: format!(
                            "gfx pass `{}` 读资源 `{}` 但无先前 pass 写入(依赖环 / use-before-write,I3)",
                            pass.name,
                            self.resource_name(ResourceId(a.resource.0))
                        ),
                    });
                }
            }

            // ⑦ 声明↔反射双向精确相等(RXS-0273;资源状态访问域 = color/depth/texture/uav,
            //    sampler/table 计入反射但标「无状态访问」类——barrier 相等域只核状态访问资源,
            //    sampler/table 另核绑定完备性)。
            if let Some(refl) = &pass.reflection {
                let declared_state: BTreeSet<u32> =
                    pass.accesses.iter().map(|a| a.resource.0).collect();
                let reflected_state: BTreeSet<u32> = refl
                    .iter()
                    .filter(|r| {
                        self.resource_kinds
                            .get(r.0 as usize)
                            .copied()
                            .flatten()
                            .is_none_or(|k| k.has_state())
                    })
                    .map(|r| r.0)
                    .collect();
                if declared_state != reflected_state {
                    let missing: Vec<u32> = reflected_state
                        .difference(&declared_state)
                        .copied()
                        .collect();
                    let unused: Vec<u32> = declared_state
                        .difference(&reflected_state)
                        .copied()
                        .collect();
                    return Err(RhiError::ReflectionMismatch {
                        detail: format!(
                            "gfx pass `{}` 声明-反射失配(RXS-0273):漏声明(反射有声明无)={missing:?} / \
                             声明未用(声明有反射无)={unused:?}",
                            pass.name
                        ),
                    });
                }
            }

            // 本 gfx pass 的写更新 written 集。
            for a in &pass.accesses {
                if a.kind.is_write() {
                    written.insert(a.resource.0);
                }
            }
        }

        // ⑧ 桥接 graph.rs::Graph(同 crate 直接构造,AccessKind 单源复用,无 cabi marshalling)。
        let mut gfx_graph = graph::Graph::new();
        // 资源映射:RHI ResourceId → graph.rs ResourceId(按 RHI 资源序同步重建)。
        let mut res_map: Vec<graph::ResourceId> = Vec::with_capacity(self.resources.len());
        for (i, kind) in self.resource_kinds.iter().enumerate() {
            let name = self
                .resources
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("res#{i}"));
            let gid = match kind {
                Some(GfxResourceKind::Color) => gfx_graph.color_target(&name),
                Some(GfxResourceKind::Depth) => gfx_graph.depth_target(&name),
                Some(GfxResourceKind::Texture) => gfx_graph.sampled_texture(&name),
                // sampler / table / None(compute UAV)→ uav_buffer(buffer,COMMON;无 layout)。
                // compute UAV 在混合图中 reads→ShaderRead / writes→UavReadWrite(RXS-0272 映射)。
                _ => gfx_graph.uav_buffer(&name),
            };
            res_map.push(gid);
        }

        // gfx pass → graph.rs::PassSpec(声明序 = 提交序;AccessKind 单源直传)。
        for gfx_pass in &self.gfx_passes {
            let mut ps = graph::PassSpec::new(&gfx_pass.name);
            for a in &gfx_pass.accesses {
                let rhi_rid = ResourceId(a.resource.0);
                let gid = res_map
                    .get(rhi_rid.0 as usize)
                    .copied()
                    .unwrap_or(graph::ResourceId(a.resource.0));
                ps = ps.with(gid, a.kind);
            }
            // present 终端 handoff → PresentHandoff 声明(RXS-0272)。
            if let Some(present_rid) = gfx_pass.present {
                let gid = res_map
                    .get(present_rid.0 as usize)
                    .copied()
                    .unwrap_or(graph::ResourceId(present_rid.0));
                ps = ps.present_handoff(gid);
            }
            gfx_graph.add_pass(ps).map_err(|e| RhiError::Structure {
                detail: format!("gfx 桥接 graph.rs: {e}"),
            })?;
        }

        // graph.rs 装配核验 + derive_barriers(纯函数;同图 → 逐字节相同计划,golden 可锚)。
        gfx_graph.seal().map_err(|e| RhiError::Structure {
            detail: format!("gfx 桥接 graph.rs seal: {e}"),
        })?;
        self.gfx_barriers = gfx_graph.derive_barriers();
        Ok(())
    }

    /// 自动资源 hazard 推导(**纯函数**)。输入 = 已 seal 图;输出 = 确定性同步计划:逐资源沿
    /// 声明全序推进,与先前访问形成 RAW/WAW/WAR 危险即在该 pass 边界产出一条同步。同图 →
    /// 逐字节相同计划(golden 可锚)。
    ///
    /// 调用前须 `seal()`(未 seal → 返回空计划;`submit()`/`execute()` 走完整生命周期)。
    #[must_use]
    pub fn derive_syncs(&self) -> Vec<PlannedSync> {
        if !self.sealed {
            return Vec::new();
        }
        // 逐资源上一访问种类 + 所在 pass(跨 pass hazard 判据)。
        let mut last: Vec<Option<(AccessKind, usize)>> = vec![None; self.resources.len()];
        let mut plan = Vec::new();

        for (pass_idx, pass) in self.passes.iter().enumerate() {
            for a in &pass.accesses {
                let ridx = a.resource.0 as usize;
                if ridx >= last.len() {
                    continue;
                }
                if let Some((prev_kind, prev_pass)) = last[ridx]
                    && prev_pass < pass_idx
                {
                    let hazard = match (prev_kind, a.kind) {
                        (AccessKind::Write, AccessKind::Read) => Some(Hazard::ReadAfterWrite),
                        (AccessKind::Write, AccessKind::Write) => Some(Hazard::WriteAfterWrite),
                        (AccessKind::Read, AccessKind::Write) => Some(Hazard::WriteAfterRead),
                        // read-after-read:无 hazard(无写者,不需同步)。
                        (AccessKind::Read, AccessKind::Read) => None,
                    };
                    if let Some(hazard) = hazard {
                        plan.push(PlannedSync {
                            resource: a.resource,
                            at_pass: pass_idx,
                            hazard,
                        });
                    }
                }
                last[ridx] = Some((a.kind, pass_idx));
            }
        }
        plan
    }

    /// 完整装配生命周期:seal(如未 seal)+ 推导 + 生命周期封口(二次 execute → Structure)。
    /// 返回确定性 hazard 计划,供执行器逐字重放。`rxrt_rhi_submit` 转发本函数(1-submit)。
    ///
    /// # Errors
    /// 图结构违例(I3/I5)→ [`RhiError::Structure`];声明-反射失配(I4)→
    /// [`RhiError::ReflectionMismatch`];二次 execute → [`RhiError::Structure`]。
    pub fn execute(&mut self) -> Result<Vec<PlannedSync>> {
        if self.executed {
            return Err(RhiError::Structure {
                detail: "重复 submit/execute(生命周期误用)".to_owned(),
            });
        }
        if !self.sealed {
            self.seal()?;
        }
        let plan = self.derive_syncs();
        self.executed = true;
        Ok(plan)
    }

    /// 图形 barrier 推导产物(G4.2,RXS-0272;`seal()` 桥接 `graph.rs::derive_barriers` 产)。
    /// 供 Vulkan 执行器逐字重放(禁二次推导,P-11)。compute-only 图(无 gfx pass)为空。
    /// 调用前须 `seal()`(未 seal → 返回空切片)。
    #[must_use]
    pub fn derive_gfx_barriers(&self) -> &[crate::graph::PlannedBarrier] {
        if !self.sealed {
            return &[];
        }
        &self.gfx_barriers
    }

    /// 图形 pass 数(G4.2,RXS-0270;执行器录制用)。
    #[must_use]
    pub fn gfx_pass_count(&self) -> usize {
        self.gfx_passes.len()
    }

    /// 是否含图形 pass(G4.2,RXS-0272:含图形 pass 的图仅经 Vulkan 后端执行)。
    #[must_use]
    pub fn has_graphics(&self) -> bool {
        !self.gfx_passes.is_empty()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个合法三 pass compute 图:produce(写 a)→ transform(读 a 写 b)→ consume(读 b 写 c)。
    fn linear_graph() -> RhiGraph {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let b = g.resource("b");
        let c = g.resource("c");
        g.add_pass(PassSpec::new("produce").writes(a)).unwrap();
        g.add_pass(PassSpec::new("transform").reads(a).writes(b))
            .unwrap();
        g.add_pass(PassSpec::new("consume").reads(b).writes(c))
            .unwrap();
        g
    }

    /// accept:合法线性图 submit 通过,hazard 推导产出恰 2 条 RAW(a@transform / b@consume)。
    //@ spec: RXS-0258
    #[test]
    fn accepts_linear_graph_derives_raw_syncs() {
        let mut g = linear_graph();
        let plan = g.execute().expect("合法图应 submit 通过");
        assert_eq!(plan.len(), 2, "线性图应恰 2 条 RAW 同步");
        assert!(plan.iter().all(|s| s.hazard == Hazard::ReadAfterWrite));
        // a 在 transform(pass 1)读 → RAW @ pass 1;b 在 consume(pass 2)读 → RAW @ pass 2。
        assert!(plan.iter().any(|s| s.at_pass == 1));
        assert!(plan.iter().any(|s| s.at_pass == 2));
    }

    /// 推导纯函数确定性:同图两次推导逐字节相同(golden 可锚,RXS-0258)。
    //@ spec: RXS-0258
    #[test]
    fn derivation_is_deterministic() {
        let g1 = {
            let mut g = linear_graph();
            g.seal().unwrap();
            g
        };
        let g2 = {
            let mut g = linear_graph();
            g.seal().unwrap();
            g
        };
        assert_eq!(g1.derive_syncs(), g2.derive_syncs());
        assert_eq!(g1.derive_syncs(), g1.derive_syncs());
    }

    /// 推导覆盖 WAW / WAR:写后写 + 读后写各产一条同步(RXS-0258)。
    //@ spec: RXS-0258
    #[test]
    fn derives_waw_and_war_syncs() {
        // WAW:pass0 写 a,pass1 再写 a。
        let mut gw = RhiGraph::new();
        let a = gw.resource("a");
        gw.add_pass(PassSpec::new("w0").writes(a)).unwrap();
        gw.add_pass(PassSpec::new("w1").writes(a)).unwrap();
        let plan = gw.execute().unwrap();
        assert!(plan.iter().any(|s| s.hazard == Hazard::WriteAfterWrite));

        // WAR:pass0 写 b(建内容),pass1 读 b,pass2 写 b。
        let mut gr = RhiGraph::new();
        let b = gr.resource("b");
        gr.add_pass(PassSpec::new("seed").writes(b)).unwrap();
        gr.add_pass(PassSpec::new("r").reads(b)).unwrap();
        gr.add_pass(PassSpec::new("w").writes(b)).unwrap();
        let plan = gr.execute().unwrap();
        assert!(plan.iter().any(|s| s.hazard == Hazard::WriteAfterRead));
    }

    // ── I3 依赖环 / I5 写写冲突 / 生命周期(Structure)+ I4 反射失配(ReflectionMismatch)── //

    /// reject(I3):读未写(依赖环 / use-before-write 可达形态)→ Structure(库层状态值)。
    //@ spec: RXS-0258
    #[test]
    fn rejects_read_before_write_i3() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let out = g.resource("out");
        // transform 读 a,但无先前 pass 写 a。
        g.add_pass(PassSpec::new("transform").reads(a).writes(out))
            .unwrap();
        match g.seal() {
            Err(e @ RhiError::Structure { .. }) => assert_eq!(e.category(), "structure"),
            other => panic!("读未写应 Structure(I3),实得 {other:?}"),
        }
    }

    /// reject(I5):同 pass 对同资源写写冲突(重复 writes)→ Structure(库层状态值)。
    //@ spec: RXS-0258
    #[test]
    fn rejects_write_write_conflict_i5() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        g.add_pass(PassSpec::new("bad").writes(a).writes(a))
            .unwrap();
        assert!(matches!(g.seal(), Err(RhiError::Structure { .. })));
    }

    /// reject(I5):同 pass 既 reads 又 writes 同资源(读写冲突)→ Structure(库层状态值)。
    //@ spec: RXS-0258
    #[test]
    fn rejects_read_write_same_pass_i5() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        g.add_pass(PassSpec::new("seed").writes(a)).unwrap();
        // 后续 pass 同资源既读又写 = feedback 读写冲突。
        g.add_pass(PassSpec::new("bad").reads(a).writes(a)).unwrap();
        assert!(matches!(g.seal(), Err(RhiError::Structure { .. })));
    }

    /// reject:空图 / 生命周期误用(二次 seal / seal 后追加 / 二次 submit)→ Structure。
    //@ spec: RXS-0258
    #[test]
    fn rejects_lifecycle_misuse() {
        // 空图。
        let mut empty = RhiGraph::new();
        assert!(matches!(empty.seal(), Err(RhiError::Structure { .. })));

        // seal 后追加 pass。
        let mut g = linear_graph();
        g.seal().unwrap();
        assert!(matches!(
            g.add_pass(PassSpec::new("extra")),
            Err(RhiError::Structure { .. })
        ));
        // 二次 seal。
        assert!(matches!(g.seal(), Err(RhiError::Structure { .. })));

        // 二次 submit/execute。
        let mut g2 = linear_graph();
        g2.execute().unwrap();
        assert!(matches!(g2.execute(), Err(RhiError::Structure { .. })));
    }

    /// reject(I4):声明-反射双向失配(漏声明 / 声明未用)→ ReflectionMismatch(库层状态值)。
    //@ spec: RXS-0257
    #[test]
    fn rejects_reflection_mismatch_i4() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let b = g.resource("b");
        // 声明只写 a;kernel 反射面含 a+b(漏声明 b)→ ReflectionMismatch。
        g.add_pass(PassSpec::new("k").writes(a).with_reflection(vec![a, b]))
            .unwrap();
        match g.seal() {
            Err(e @ RhiError::ReflectionMismatch { .. }) => assert_eq!(e.category(), "reflection"),
            other => panic!("声明-反射失配应 ReflectionMismatch(I4),实得 {other:?}"),
        }
    }

    /// accept(I4):声明-反射精确相等 → 通过(顺序无关,集合相等)。
    //@ spec: RXS-0257
    #[test]
    fn accepts_reflection_exact_match_i4() {
        let mut g = RhiGraph::new();
        let a = g.resource("a");
        let b = g.resource("b");
        g.add_pass(
            PassSpec::new("seed")
                .writes(a)
                .writes(b)
                .with_reflection(vec![b, a]),
        )
        .unwrap();
        g.add_pass(PassSpec::new("use").reads(a).reads(b)).unwrap();
        assert!(g.seal().is_ok());
    }

    /// transient 资源图内生命周期容量记账(RXS-0262,I10 峰值观测源)。**EI1.3 兑现面 = host 侧
    /// 容量记账**:`resource()` 单调分配、`resource_count()` 精确追踪图内 transient 资源数(声明区间
    /// = 首写→末读)。**诚实收窄**:RXS-0262 Legality 的 const 泛型定长数组编译期越界拒随后续期落地
    /// (现 host 记账为 Vec 承载,实际峰值 evidence 归 device 执行期计数 EI1.4);此测证已实现的 host
    /// 侧容量记账本体(I10 报告项的静态源)。
    //@ spec: RXS-0262
    #[test]
    fn transient_resource_capacity_accounting() {
        let mut g = RhiGraph::new();
        assert_eq!(g.resource_count(), 0, "空图 0 资源");
        let a = g.resource("a");
        let b = g.resource("b");
        let c = g.resource("c");
        assert_eq!(g.resource_count(), 3, "三 transient 资源单调记账");
        // 资源 id 稳定单调(下标即 id,声明序 = 分配序)。
        assert_eq!((a.0, b.0, c.0), (0, 1, 2), "资源 id 稳定单调(声明序)");
        // 图内声明区间容量(峰值 <= 声明容量,I10 静态源;实际执行期峰值归 device measured)。
        g.add_pass(PassSpec::new("p0").writes(a)).unwrap();
        g.add_pass(PassSpec::new("p1").reads(a).writes(b)).unwrap();
        g.add_pass(PassSpec::new("p2").reads(b).writes(c)).unwrap();
        assert_eq!(g.resource_count(), 3, "pass 声明不改 transient 资源容量");
        assert!(g.execute().is_ok(), "合法图装配核验通过");
    }

    /// AccessKind u32 tag round-trip(cabi 下发;read=0 / write=1)。
    //@ spec: RXS-0257
    #[test]
    fn access_kind_tag_round_trip() {
        assert_eq!(AccessKind::Read.as_u32(), 0);
        assert_eq!(AccessKind::Write.as_u32(), 1);
        for k in [AccessKind::Read, AccessKind::Write] {
            assert_eq!(AccessKind::from_u32(k.as_u32()), Some(k));
        }
        assert_eq!(AccessKind::from_u32(2), None);
        assert!(AccessKind::Write.is_write());
        assert!(AccessKind::Read.is_consuming_read());
    }
}
