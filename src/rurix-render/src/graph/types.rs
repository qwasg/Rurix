//! 冻结契约类型(G5_PLAN §2;RFC-0016 跨章一致性约定)。
//!
//! 本文件是渲染器各子系统之间的**唯一共享契约**:图声明面、EB 三轴屏障、transient
//! 生命周期、簇记录/VisBuffer 位格式、材质闭合、流送页请求、异步车道 fence 对。
//! G5 波次内**不得漂移**——任何字段/位布局变更须回到主线裁决,子系统实现只允许
//! 新增自有类型,不允许改写本文件既有定义(镜像 RXS 条款 0-byte 纪律)。

// ---------------------------------------------------------------------------
// 句柄
// ---------------------------------------------------------------------------

/// 图内资源句柄(录制期描述符序号;与生命周期绑定,越期使用由图编译器确定性拒)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(pub u32);

/// 图内 pass 句柄(应用指定线性序的序号,Frostbite 式;执行序可由车道/重排置换,
/// 依赖语义以声明序为准)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PassId(pub u32);

// ---------------------------------------------------------------------------
// 队列车道(报告5 §2.4:异步候选三条件纪律——时长 ≥0.5ms / 无图形管线依赖 /
// 消费者距生产者足够远;验收以时间戳对比,无效则回退)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum QueueClass {
    #[default]
    Graphics,
    AsyncCompute,
}

// ---------------------------------------------------------------------------
// 资源描述(录制期描述符;transient 物理分配延迟到执行前,imported 资源图只管理
// 状态转换不管理内存——报告5 §2.3 两条约束)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba16Float,
    R11G11B10Float,
    Rg16Float,
    R32Uint,
    Rg32Uint,
    R32Float,
    Depth32Float,
}

impl TextureFormat {
    /// 每像素字节数(transient 池 size 估算用)。
    pub fn bytes_per_pixel(self) -> u64 {
        match self {
            TextureFormat::Rgba8Unorm
            | TextureFormat::R11G11B10Float
            | TextureFormat::R32Uint
            | TextureFormat::R32Float
            | TextureFormat::Depth32Float => 4,
            TextureFormat::Rgba16Float | TextureFormat::Rg32Uint => 8,
            TextureFormat::Rg16Float => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Buffer {
        size: u64,
    },
    Texture2d {
        width: u32,
        height: u32,
        format: TextureFormat,
        mip_levels: u32,
    },
}

impl ResourceKind {
    /// 保守字节尺寸(池分桶用;纹理按 mip 链几何级数上界 4/3 保守取整)。
    pub fn byte_size(&self) -> u64 {
        match *self {
            ResourceKind::Buffer { size } => size,
            ResourceKind::Texture2d {
                width,
                height,
                format,
                mip_levels,
            } => {
                let base = u64::from(width) * u64::from(height) * format.bytes_per_pixel();
                if mip_levels > 1 {
                    base + base / 3
                } else {
                    base
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceDesc {
    pub name: String,
    pub kind: ResourceKind,
    /// true = 外部资源 import(跨帧历史/流送产物/backbuffer):图只推导状态转换,
    /// 不入 transient 池、不参与别名(报告5 §2.3 约束一)。
    pub imported: bool,
}

// ---------------------------------------------------------------------------
// 访问声明(封闭枚举,镜像 spec/render_graph.md RXS-0236 口径;漏声明访问 =
// 编译期确定性拒)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessKind {
    /// 采样/SRV 只读。
    ShaderRead,
    /// UAV/storage 写(含读写)。
    ShaderWrite,
    /// color attachment 写。
    ColorTarget,
    /// depth attachment 写。
    DepthTarget,
    /// depth 只读采样(阴影投影/SSR 类)。
    DepthRead,
    /// 间接参数缓冲消费(draw/dispatch indirect)。
    IndirectArgs,
    CopySrc,
    CopyDst,
    /// present handoff(终端资源)。
    Present,
}

impl AccessKind {
    pub fn is_write(self) -> bool {
        matches!(
            self,
            AccessKind::ShaderWrite
                | AccessKind::ColorTarget
                | AccessKind::DepthTarget
                | AccessKind::CopyDst
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResAccess {
    pub res: ResourceId,
    pub access: AccessKind,
}

// ---------------------------------------------------------------------------
// pass 声明(声明与 execute 闭包分离——报告5 §3 render/pass 改造要求;execute
// 载体由 graph.rs 定义,本契约只冻结声明面)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PassDesc {
    pub name: String,
    pub queue: QueueClass,
    pub reads: Vec<ResAccess>,
    pub writes: Vec<ResAccess>,
}

// ---------------------------------------------------------------------------
// EB 三轴屏障(报告5 §2.2:D3D12 Enhanced Barriers sync/access/layout 三轴分离
// 作为后端中立内部规范形式;stage 采用 AnKi 简化集,access 简化为读/写两侧组合,
// 把 25+ stage/30+ access 组合爆炸挡在引擎内部)
// ---------------------------------------------------------------------------

/// AnKi 简化 stage 集(PC/主机上真正不同的 stage 基本只有 graphics 与 compute,
/// 传输队列正交另计)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyncStage {
    /// 无等待(首用/丢弃语义侧)。
    None,
    Graphics,
    Compute,
    Copy,
    /// 保守全阻塞(调试阀门;正常推导不应产出)。
    All,
}

/// 简化 access 集(fake flush 只读链优化用 `NONE`;报告5 §2.2 Granite 细节)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessMask {
    None,
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageLayout {
    /// 未定义(别名 handoff 入局丢弃旧数据;buffer 恒为 Undefined 占位)。
    Undefined,
    General,
    ColorAttachment,
    DepthAttachment,
    ShaderReadOnly,
    TransferSrc,
    TransferDst,
    Present,
}

/// 后端中立屏障规范形式(Vulkan 后端映射 sync2 / D3D12 后端映射 Enhanced
/// Barriers;golden 锚定单源)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Barrier {
    pub res: ResourceId,
    pub sync_before: SyncStage,
    pub sync_after: SyncStage,
    pub access_before: AccessMask,
    pub access_after: AccessMask,
    pub layout_before: ImageLayout,
    pub layout_after: ImageLayout,
}

// ---------------------------------------------------------------------------
// 异步车道 fence 对(报告5 §2.4:RDG 语义——异步段在图形管线最后生产者处 signal、
// 首个图形消费者处 wait;timeline 值单调)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FencePair {
    /// 该 pass 执行完毕后 signal。
    pub signal_after: PassId,
    /// 该 pass 执行前 wait。
    pub wait_before: PassId,
    /// timeline semaphore 值。
    pub value: u64,
}

// ---------------------------------------------------------------------------
// transient 生命周期与池槽位(报告5 §2.3;别名 handoff 以 layout_before=Undefined
// 入局;峰值审计 high-water 进 CI)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifeInterval {
    pub first_use: PassId,
    pub last_use: PassId,
}

impl LifeInterval {
    /// 区间相交判定(相交则不可别名共享)。
    pub fn overlaps(&self, other: &LifeInterval) -> bool {
        self.first_use <= other.last_use && other.first_use <= self.last_use
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoolSlot {
    /// 分桶(对齐/用途类别:GBuffer 类 / compute UAV 类 / AS scratch 类)。
    pub bucket: u32,
    /// 桶内槽序号(同槽不同 ResourceId = 别名共享)。
    pub slot: u32,
    pub size: u64,
}

// ---------------------------------------------------------------------------
// 几何:簇记录与 VisBuffer 位格式(报告1;GPU 可见定长布局,离线构建与运行时
// 剔除/光栅/resolve 共用,序列化预留页表字段)
// ---------------------------------------------------------------------------

/// 每簇三角形上限(工业共识 ~128;tri 索引占 VisBuffer 7 位)。
pub const MAX_TRIS_PER_CLUSTER: u32 = 128;
/// 每簇顶点上限(meshopt 惯例 64;顶点索引 u8 局部化)。
pub const MAX_VERTS_PER_CLUSTER: u32 = 64;

/// 簇记录(64B 定长,`repr(C)`,GPU buffer 元素;字段序冻结)。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClusterRecord {
    /// 包围球中心(对象空间)。
    pub center: [f32; 3],
    /// 包围球半径。
    pub radius: f32,
    /// 背面锥轴(单位向量)。
    pub cone_axis: [f32; 3],
    /// 背面锥剔除阈值(dot(view, axis) >= cutoff 则整簇背面剔除;meshopt 口径)。
    pub cone_cutoff: f32,
    /// 本簇简化误差(对象空间;LOD cut 判据:误差投影 ≤1px 取本层)。
    pub error: f32,
    /// 父组简化误差(DAG 单调:parent_error >= error;裂缝保护)。
    pub parent_error: f32,
    /// 顶点数据段偏移(页内局部;单层构建时为全局偏移)。
    pub vertex_offset: u32,
    /// 三角形索引段偏移。
    pub triangle_offset: u32,
    pub vertex_count: u32,
    /// ≤ MAX_TRIS_PER_CLUSTER。
    pub triangle_count: u32,
    /// 所属流送页 id(P0 单页常驻 = 0;预留页表字段,报告1 P4)。
    pub page_id: u32,
    pub reserved: u32,
}

/// VisBuffer 位格式:u64 = depth(30) | cluster(27) | tri(7),depth 在高位使
/// atomicMax 即深度测试(报告1 Nanite 口径)。
pub const VISBUFFER_DEPTH_BITS: u32 = 30;
pub const VISBUFFER_CLUSTER_BITS: u32 = 27;
pub const VISBUFFER_TRI_BITS: u32 = 7;

pub fn visbuffer_pack(depth30: u32, cluster: u32, tri: u32) -> u64 {
    debug_assert!(depth30 < (1 << VISBUFFER_DEPTH_BITS));
    debug_assert!(cluster < (1 << VISBUFFER_CLUSTER_BITS));
    debug_assert!(tri < (1 << VISBUFFER_TRI_BITS));
    (u64::from(depth30) << (VISBUFFER_CLUSTER_BITS + VISBUFFER_TRI_BITS))
        | (u64::from(cluster) << VISBUFFER_TRI_BITS)
        | u64::from(tri)
}

pub fn visbuffer_unpack(v: u64) -> (u32, u32, u32) {
    let tri = (v & ((1 << VISBUFFER_TRI_BITS) - 1)) as u32;
    let cluster = ((v >> VISBUFFER_TRI_BITS) & ((1 << VISBUFFER_CLUSTER_BITS) - 1)) as u32;
    let depth = (v >> (VISBUFFER_CLUSTER_BITS + VISBUFFER_TRI_BITS)) as u32;
    (depth, cluster, tri)
}

// ---------------------------------------------------------------------------
// 材质:单层 principled 闭合(报告6;32B 定长 GBuffer 载体,Blendable 式)
// ---------------------------------------------------------------------------

/// 单层材质闭合(32B 定长,`repr(C)`;打包口径冻结,pack/unpack 由 material 模块
/// 提供并单测往返)。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialClosure {
    /// albedo RGB + 不透明 A(RGBA8)。
    pub albedo_rgba8: u32,
    /// F0 RGB(RGBA8;A 保留)。
    pub f0_rgba8: u32,
    /// roughness(8) | metalness(8) | ao(8) | flags(8)。
    pub rough_metal_ao_flags: u32,
    /// 法线八面体编码(16+16)。
    pub normal_oct16: u32,
    /// 自发光 RGBE 共享指数编码。
    pub emissive_rgbe: u32,
    /// 材质 id(classify/resolve 分类键)。
    pub material_id: u32,
    pub reserved: [u32; 2],
}

// ---------------------------------------------------------------------------
// 流送:页请求与三预算(报告6;128KB 页,反馈驱动,几何/纹理同栈)
// ---------------------------------------------------------------------------

/// 流送页大小(128KB,报告6/报告1 共识)。
pub const STREAM_PAGE_SIZE: u32 = 128 * 1024;

/// GPU 反馈页请求(`repr(C)` 定长,GPU 回读缓冲元素)。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageRequest {
    /// 流送资源注册号(几何页/纹理页/未来 SVT 同栈)。
    pub resource: u32,
    /// 资源内页序号。
    pub page_index: u32,
    /// 优先级(屏幕误差/mip 距离驱动;大者优先)。
    pub priority: u32,
    /// 请求帧号(时效裁剪)。
    pub frame: u32,
}

/// 每帧三预算(io/transcode/upload 各自独立计量;超预算延后不丢弃)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamingBudget {
    pub io_bytes: u64,
    pub transcode_bytes: u64,
    pub upload_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_layout_sizes() {
        assert_eq!(core::mem::size_of::<ClusterRecord>(), 64);
        assert_eq!(core::mem::size_of::<MaterialClosure>(), 32);
        assert_eq!(core::mem::size_of::<PageRequest>(), 16);
    }

    #[test]
    fn visbuffer_roundtrip() {
        let v = visbuffer_pack((1 << 30) - 1, (1 << 27) - 1, 127);
        assert_eq!(v, u64::MAX); // 30+27+7 = 64 位满段全 1
        assert_eq!(visbuffer_unpack(v), ((1 << 30) - 1, (1 << 27) - 1, 127));
        let v2 = visbuffer_pack(12345, 678, 90);
        assert_eq!(visbuffer_unpack(v2), (12345, 678, 90));
        // depth 高位序:更大 depth 打包值更大(atomicMax 即深度测试)。
        assert!(visbuffer_pack(2, 0, 0) > visbuffer_pack(1, (1 << 27) - 1, 127));
    }

    #[test]
    fn life_interval_overlap() {
        let a = LifeInterval {
            first_use: PassId(0),
            last_use: PassId(2),
        };
        let b = LifeInterval {
            first_use: PassId(3),
            last_use: PassId(5),
        };
        let c = LifeInterval {
            first_use: PassId(2),
            last_use: PassId(3),
        };
        assert!(!a.overlaps(&b));
        assert!(a.overlaps(&c));
        assert!(b.overlaps(&c));
    }
}
