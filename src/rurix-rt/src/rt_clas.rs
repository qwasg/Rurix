//! `rt_clas` — G9.3 M94 CLAS×RT 合流 host 面(RXS-0351;RFC-0022 §4.3;门
//! `g9.p0.m94.clas_rt_convergence`)。
//!
//! always-on、零 unsafe、零后端调用(沿 `dgc.rs`/`descriptor_table.rs` 先例):
//!
//! - **`ClasBlasKey` = 可见簇集合内容 digest**(RXS-0351 L6):`ClasAssembler` 是
//!   当帧拼装产物(`AssembledBlas`)的**单所有者**——产物无 pub 构造器,只能经
//!   `assemble_frame` 产出;`ClasAsStats` 计数面(AS 构建/模板构建/实例化/静态帧)
//!   支撑「静态帧零 AS 构建,构建计数非零即 RED」(L4)的机器核验。
//! - **装配期一致性核验**(L3 RED 臂锚):[`verify_visible_blas_consistency`]
//!   逐簇比对可见集与 BLAS 内容——错开一簇即确定性 `Err`(RED)。
//! - **双腿调用面**(L1/L2/L7):同一 `AssembledBlas` 经 [`RtLeg`] 选择驱动
//!   CLAS 主腿(device 面归 `vk` U56 lane)或传统 triangles BLAS 回退腿
//!   (per-簇分组 = 「按对象分组」,L2);[`select_leg`] 在**装配期**裁决——
//!   manifest 显式 variant,主腿 capability 缺失 → 装载 fail-closed
//!   (确定性 `Err`),**禁止运行期静默换腿**。
//! - **host 金标准**:Möller–Trumbore 双面最近命中参照(与 `rurix-render`
//!   `rt::bvh` 同构语义的本地最小实现;主腿 not-supported 时逐命中一致判据
//!   以「回退腿 vs host 金标准」替代,容差 0)。
//!
//! DMM 禁止线(L8):本模块无任何 micromap 概念面。

use std::collections::BTreeMap;
use std::fmt;

// ---------------------------------------------------------------------------
// 可见簇集(RFC-0022 §4.4 元素:cluster stable id + LOD level + 蒙皮版本 + 变换 id)
// ---------------------------------------------------------------------------

/// 可见簇集元素(紧凑载荷;RXS-0350 的 selection cut 输出元素形态)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisibleClusterEntry {
    /// cluster stable id。
    pub cluster_id: u64,
    /// LOD level。
    pub lod_level: u8,
    /// 蒙皮版本(静态几何恒 0)。
    pub skin_version: u32,
    /// 变换 id。
    pub transform_id: u32,
}

// ---------------------------------------------------------------------------
// 簇几何(CLAS 烘焙输入的消费形态:三角形簇 = 位置 + 索引;簇级 AABB 逐位 min/max)
// ---------------------------------------------------------------------------

/// 簇几何(世界空间;`indices` 引用 `positions`,图元序 = 索引序)。
///
/// CLAS 构建要求显式索引缓冲(VUID-VkClusterAccelerationStructureBuildTriangleClusterInfoNV-
/// indexBuffer-parameter:地址必须有效),故本类型携带索引;回退腿传统 BLAS 经
/// [`Self::triangle_soup`] 展开为 9 f32/三角形(与 `vk::RayQuerySceneDesc` 同口径:
/// 顶点序即 `primitiveIndex` 序)。
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterGeometry {
    /// 顶点位置(3 f32/顶点,世界空间)。
    pub positions: Vec<[f32; 3]>,
    /// 三角形索引(3 u32/三角形)。
    pub indices: Vec<[u32; 3]>,
}

impl ClusterGeometry {
    /// 构造并校核(索引越界 / 空图元 → 确定性 `Err`,fail-closed)。
    pub fn new(positions: Vec<[f32; 3]>, indices: Vec<[u32; 3]>) -> Result<Self, ClasError> {
        if indices.is_empty() {
            return Err(ClasError::EmptyCluster);
        }
        let n = positions.len() as u32;
        if indices.iter().flatten().any(|&i| i >= n) {
            return Err(ClasError::IndexOutOfRange);
        }
        if positions.iter().flatten().any(|v| !v.is_finite()) {
            return Err(ClasError::NonFiniteVertex);
        }
        Ok(Self { positions, indices })
    }

    /// 簇级 AABB(逐位 min/max,RXS-0345 §3.4;与 geom-build `clas_bake_input_of` 同口径:
    /// 空簇产零盒——本类型拒绝空簇,防御分支仅为同构)。
    pub fn aabb(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for v in &self.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(v[k]);
                hi[k] = hi[k].max(v[k]);
            }
        }
        if self.positions.is_empty() {
            lo = [0.0; 3];
            hi = [0.0; 3];
        }
        (lo, hi)
    }

    /// 内容 digest(FNV-1a 64:位置逐位 + 索引;与 `BlasKey::from_mesh` 同源纪律,
    /// `+0.0`/`-0.0` 位模式不同视为不同内容)。
    pub fn content_digest(&self) -> u64 {
        let mut h = FNV_OFFSET;
        for p in &self.positions {
            for &c in p {
                fnv_mix(&mut h, u64::from(c.to_bits()));
            }
        }
        for t in &self.indices {
            for &vi in t {
                fnv_mix(&mut h, u64::from(vi));
            }
        }
        h
    }

    /// 拓扑 digest(模板分组键:图元/顶点计数 + 索引序列,**不含位置**——同拓扑
    /// 不同位置的静态重复几何共享 Cluster Template,RXS-0351 L1)。
    pub fn topology_digest(&self) -> u64 {
        let mut h = FNV_OFFSET;
        fnv_mix(&mut h, self.indices.len() as u64);
        fnv_mix(&mut h, self.positions.len() as u64);
        for t in &self.indices {
            for &vi in t {
                fnv_mix(&mut h, u64::from(vi));
            }
        }
        h
    }

    /// 回退腿消费面:展开为 9 f32/三角形(顶点序即 `primitiveIndex` 序)。
    pub fn triangle_soup(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.indices.len() * 9);
        for t in &self.indices {
            for &vi in t {
                out.extend_from_slice(&self.positions[vi as usize]);
            }
        }
        out
    }
}

/// FNV-1a 64 offset basis(与 `BlasKey` 同源)。
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64 prime。
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv_mix(h: &mut u64, v: u64) {
    *h = (*h ^ v).wrapping_mul(FNV_PRIME);
}

// ---------------------------------------------------------------------------
// CLAS 烘焙输入 32B 页内 ABI 镜像(RXS-0345 L4;logical_v2 CLAS_RECORD_SIZE=32)
// ---------------------------------------------------------------------------

/// CLAS 烘焙输入记录(页内 ABI 镜像:`triangle_offset:u32`、`triangle_count:u32`
/// 与簇级 AABB 6×f32,小端;**32B 定长**,与 `rurix_geom_pages::logical_v2`
/// 的 CLAS 段逐字段同构——跨 crate 一致性由单测锚定)。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClasBakeRecord {
    /// 页内三角形偏移(图元计)。
    pub triangle_offset: u32,
    /// 簇三角形数。
    pub triangle_count: u32,
    /// 簇级 AABB min。
    pub aabb_min: [f32; 3],
    /// 簇级 AABB max。
    pub aabb_max: [f32; 3],
}

impl ClasBakeRecord {
    /// 自簇几何推导(AABB 逐位 min/max,与 [`ClusterGeometry::aabb`] 同口径)。
    pub fn of_cluster(triangle_offset: u32, cluster: &ClusterGeometry) -> Self {
        let (aabb_min, aabb_max) = cluster.aabb();
        Self {
            triangle_offset,
            triangle_count: cluster.indices.len() as u32,
            aabb_min,
            aabb_max,
        }
    }

    /// 小端编码(32B;与 logical_v2 CLAS 段字段序逐字一致:offset/count/aabb)。
    pub fn encode_le(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[0..4].copy_from_slice(&self.triangle_offset.to_le_bytes());
        out[4..8].copy_from_slice(&self.triangle_count.to_le_bytes());
        for (k, &v) in self.aabb_min.iter().enumerate() {
            out[8 + k * 4..12 + k * 4].copy_from_slice(&v.to_le_bytes());
        }
        for (k, &v) in self.aabb_max.iter().enumerate() {
            out[20 + k * 4..24 + k * 4].copy_from_slice(&v.to_le_bytes());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// ClasBlasKey 与错误面
// ---------------------------------------------------------------------------

/// `ClasBlasKey` = 可见簇集合**内容** digest(RXS-0351 L6):可见集元素字段 +
/// 逐簇几何内容 digest 依序混合(FNV-1a 64;与 `BlasKey` 同源纪律)。同键 =
/// 同内容 = 静态帧(零 AS 构建);任一字段/任一簇内容漂移 → 异键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClasBlasKey(pub u64);

impl ClasBlasKey {
    /// 自可见集 + 逐簇内容 digest(与 `visible` 等长平行)计算。
    ///
    /// # Errors
    /// 长度失配产 `ClasError::DigestLengthMismatch`(调用契约违例,fail-closed)。
    pub fn of_visible_set(
        visible: &[VisibleClusterEntry],
        content_digests: &[u64],
    ) -> Result<Self, ClasError> {
        if visible.len() != content_digests.len() {
            return Err(ClasError::DigestLengthMismatch {
                visible: visible.len(),
                digests: content_digests.len(),
            });
        }
        let mut h = FNV_OFFSET;
        fnv_mix(&mut h, visible.len() as u64);
        for (e, &d) in visible.iter().zip(content_digests) {
            fnv_mix(&mut h, e.cluster_id);
            fnv_mix(&mut h, u64::from(e.lod_level));
            fnv_mix(&mut h, u64::from(e.skin_version));
            fnv_mix(&mut h, u64::from(e.transform_id));
            fnv_mix(&mut h, d);
        }
        Ok(ClasBlasKey(h))
    }
}

/// M94 错误面(确定性拒绝;调用契约/装配契约违例)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClasError {
    /// 可见簇集为空(不建空 AS)。
    EmptyVisibleSet,
    /// 可见集引用源中不存在的簇。
    UnknownCluster(u64),
    /// 簇零图元(不建空 CLAS/BLAS)。
    EmptyCluster,
    /// 簇索引越界位置表。
    IndexOutOfRange,
    /// 顶点含 NaN/inf。
    NonFiniteVertex,
    /// `ClasBlasKey::of_visible_set` 长度失配。
    DigestLengthMismatch {
        /// 可见集长度。
        visible: usize,
        /// digest 长度。
        digests: usize,
    },
    /// 装配期核验失败:可见集与 BLAS 内容错开(RED 臂;首个分叉簇位置与双方面)。
    VisibleBlasMismatch {
        /// 首个分叉位置(可见集序)。
        at: usize,
        /// 可见集侧簇 id(None = 可见集更短)。
        visible: Option<u64>,
        /// BLAS 侧簇 id(None = BLAS 更短)。
        blas: Option<u64>,
    },
    /// manifest 选 CLAS 主腿而 device capability 不满足(装载 fail-closed,
    /// RXS-0351 L7;禁止静默换腿)。
    ClasMainLegNotSupported,
}

impl fmt::Display for ClasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClasError::EmptyVisibleSet => write!(f, "可见簇集为空(fail-closed,不建空 AS)"),
            ClasError::UnknownCluster(id) => write!(f, "可见集引用未知簇 {id}"),
            ClasError::EmptyCluster => write!(f, "簇零图元(fail-closed,不建空 CLAS/BLAS)"),
            ClasError::IndexOutOfRange => write!(f, "簇索引越界位置表"),
            ClasError::NonFiniteVertex => write!(f, "簇顶点含 NaN/inf"),
            ClasError::DigestLengthMismatch { visible, digests } => {
                write!(f, "digest 长度失配:visible={visible} digests={digests}")
            }
            ClasError::VisibleBlasMismatch { at, visible, blas } => write!(
                f,
                "可见集/BLAS 内容错开(首个分叉位置 {at}:visible={visible:?} blas={blas:?};错开一簇即 RED,RXS-0351 L3)"
            ),
            ClasError::ClasMainLegNotSupported => write!(
                f,
                "manifest 选 CLAS 主腿而 device capability 不满足(装载 fail-closed,禁静默换腿,RXS-0351 L7)"
            ),
        }
    }
}

impl std::error::Error for ClasError {}

// ---------------------------------------------------------------------------
// 拼装产物(单所有者:仅 ClasAssembler 可构造)
// ---------------------------------------------------------------------------

/// 逐簇主腿构建 op(manifest 显式 variant 的产物形态;L1 当帧拼装的计划面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterOp {
    /// `BUILD_TRIANGLE_CLUSTER_NV`(直建)。
    DirectBuild,
    /// `INSTANTIATE_TRIANGLE_CLUSTER_NV`(自 `templates` 下标实例化;
    /// 静态重复几何共享 Cluster Template 底层 AS,L1)。
    Instantiate {
        /// `AssembledBlas::templates` 下标。
        template: u32,
    },
}

/// Cluster Template 计划(同拓扑簇组共享;模板序 = 主腿 `Instantiate{template}`
/// 的下标空间)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplatePlan {
    /// 拓扑源簇(可见集序下标;模板经其位置/索引构建)。
    pub source_slot: u32,
    /// 组内成员(可见集序下标,含源簇自身;全部经实例化产出 CLAS)。
    pub members: Vec<u32>,
}

/// 当帧拼装产物(`ClasBlasKey` 键控;**单所有者** = [`ClasAssembler`],字段全私有,
/// 无 pub 构造器——可见集/BLAS 一致性只能经装配路径建立)。
#[derive(Debug)]
pub struct AssembledBlas {
    key: ClasBlasKey,
    visible: Vec<VisibleClusterEntry>,
    clusters: Vec<ClusterGeometry>,
    ops: Vec<ClusterOp>,
    templates: Vec<TemplatePlan>,
    serial: u64,
}

impl AssembledBlas {
    /// 拼装键(= 可见簇集合内容 digest)。
    pub fn key(&self) -> ClasBlasKey {
        self.key
    }

    /// 装配时可见集快照(provenance;帧末一致性核验的对照面)。
    pub fn visible(&self) -> &[VisibleClusterEntry] {
        &self.visible
    }

    /// 逐簇几何(可见集序)。
    pub fn clusters(&self) -> &[ClusterGeometry] {
        &self.clusters
    }

    /// 逐簇主腿构建 op(可见集序;与 `clusters` 等长平行)。
    pub fn ops(&self) -> &[ClusterOp] {
        &self.ops
    }

    /// Cluster Template 计划(模板序 = 主腿 `Instantiate{template}` 的下标空间)。
    pub fn templates(&self) -> &[TemplatePlan] {
        &self.templates
    }

    /// 装配代次(单调;观测/调试用)。
    pub fn serial(&self) -> u64 {
        self.serial
    }

    /// 回退腿消费面:逐簇(= 逐 BLAS,per-对象分组,L2)9 f32/三角形展开,
    /// 实例槽位序 = 可见集序(恒等变换)。
    pub fn fallback_blas_triangles(&self) -> Vec<Vec<f32>> {
        self.clusters.iter().map(ClusterGeometry::triangle_soup).collect()
    }

    /// 拼装 digest(L5 golden 面:键 + 逐簇 op + 模板计划的确定性 digest;
    /// 物理字节/device address 非 stable 不入——L9)。
    pub fn assembly_digest(&self) -> u64 {
        let mut h = FNV_OFFSET;
        fnv_mix(&mut h, self.key.0);
        fnv_mix(&mut h, self.ops.len() as u64);
        for op in &self.ops {
            match op {
                ClusterOp::DirectBuild => fnv_mix(&mut h, 0),
                ClusterOp::Instantiate { template } => {
                    fnv_mix(&mut h, 1);
                    fnv_mix(&mut h, u64::from(*template));
                }
            }
        }
        fnv_mix(&mut h, self.templates.len() as u64);
        for t in &self.templates {
            fnv_mix(&mut h, u64::from(t.source_slot));
            fnv_mix(&mut h, t.members.len() as u64);
            for &m in &t.members {
                fnv_mix(&mut h, u64::from(m));
            }
        }
        h
    }
}

// ---------------------------------------------------------------------------
// 统计面(L4/L6:静态帧零 AS 构建的机器核验锚)
// ---------------------------------------------------------------------------

/// AS 构建计数面(evidence 埋点;单调递增,快照语义)。
///
/// `blas_builds`/`clas_builds`/`template_builds` 在**内容变化帧**分别 += 簇数/
/// += (direct + instantiate) 数/+= 模板数;**静态帧(键不变)三者增量恒 0**——
/// 非零即 RED(RXS-0351 L4)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClasAsStats {
    /// 回退腿传统 BLAS 构建数(每簇一 BLAS)。
    pub blas_builds: u64,
    /// 主腿 CLAS 构建数(direct + instantiate)。
    pub clas_builds: u64,
    /// Cluster Template 构建数。
    pub template_builds: u64,
    /// 内容变化帧(重建)计数。
    pub assemblies: u64,
    /// 静态帧(键不变零构建)计数。
    pub static_frames: u64,
    /// 显式驱逐计数(G8 显式策略同口径)。
    pub evictions: u64,
}

// ---------------------------------------------------------------------------
// 拼装管理器(单所有者)
// ---------------------------------------------------------------------------

/// 簇几何源(可见集 id → 几何;页流送消费方实现)。
pub trait ClusterSource {
    /// 取簇几何;未知 id 产 `None`(装配 fail-closed)。
    fn cluster(&self, id: u64) -> Option<&ClusterGeometry>;
}

/// 当帧拼装管理器(`ClasBlasKey` 单所有者;RXS-0351 L1/L4/L6)。
///
/// 语义:**键不变(静态帧)→ 零 AS 构建**,返回缓存产物(降级簇 CLAS/BLAS 引用
/// 不变,L4);键变 → 全量重拼(逐簇 op 计划 + 模板分组),计数面如实记账。
#[derive(Debug)]
pub struct ClasAssembler<S: ClusterSource> {
    src: S,
    current: Option<AssembledBlas>,
    stats: ClasAsStats,
    serial: u64,
}

impl<S: ClusterSource> ClasAssembler<S> {
    /// 空管理器(包裹几何源)。
    pub fn new(src: S) -> Self {
        Self {
            src,
            current: None,
            stats: ClasAsStats::default(),
            serial: 0,
        }
    }

    /// 当帧拼装:解析可见集 → 内容 digest 键 → 键同则零构建透传,键异则
    /// 重拼(逐簇 op + 同拓扑模板分组)。
    ///
    /// 模板分组:拓扑 digest 相同且簇数 ≥ 2 的组共享一个 Cluster Template
    /// (源簇 = 组内首簇;组内全部成员经实例化产出 CLAS);孤拓扑簇直建。
    ///
    /// # Errors
    /// 空可见集 / 未知簇 / 键长度失配 → 确定性 `Err`(fail-closed)。
    pub fn assemble_frame(
        &mut self,
        visible: &[VisibleClusterEntry],
    ) -> Result<&AssembledBlas, ClasError> {
        if visible.is_empty() {
            return Err(ClasError::EmptyVisibleSet);
        }
        let mut clusters: Vec<&ClusterGeometry> = Vec::with_capacity(visible.len());
        for e in visible {
            clusters.push(
                self.src
                    .cluster(e.cluster_id)
                    .ok_or(ClasError::UnknownCluster(e.cluster_id))?,
            );
        }
        let digests: Vec<u64> = clusters.iter().map(|c| c.content_digest()).collect();
        let key = ClasBlasKey::of_visible_set(visible, &digests)?;
        if let Some(cur) = &self.current
            && cur.key == key
        {
            // 静态帧:降级簇 CLAS/BLAS 引用不变,零 AS 构建(L4)。
            self.stats.static_frames += 1;
            return Ok(self.current.as_ref().expect("current 已判定 Some"));
        }

        // ── 重拼:同拓扑分组(可见集序确定性;BTreeMap 键序 = digest 序)──
        let mut by_topology: BTreeMap<u64, Vec<u32>> = BTreeMap::new();
        for (slot, c) in clusters.iter().enumerate() {
            by_topology
                .entry(c.topology_digest())
                .or_default()
                .push(slot as u32);
        }
        let mut templates: Vec<TemplatePlan> = Vec::new();
        let mut ops: Vec<ClusterOp> = vec![ClusterOp::DirectBuild; visible.len()];
        for members in by_topology.values() {
            if members.len() >= 2 {
                let tidx = templates.len() as u32;
                templates.push(TemplatePlan {
                    source_slot: members[0],
                    members: members.clone(),
                });
                for &m in members {
                    ops[m as usize] = ClusterOp::Instantiate { template: tidx };
                }
            }
        }
        let owned: Vec<ClusterGeometry> = clusters.into_iter().cloned().collect();
        self.serial += 1;
        let product = AssembledBlas {
            key,
            visible: visible.to_vec(),
            clusters: owned,
            ops,
            templates,
            serial: self.serial,
        };
        // 计数面(L6):回退腿 BLAS = 每簇一建;主腿 CLAS = 全簇(direct+instantiate);
        // 模板 = 组数。静态帧不入此径,故此处必有增量。
        self.stats.blas_builds += visible.len() as u64;
        self.stats.clas_builds += visible.len() as u64;
        self.stats.template_builds += product.templates.len() as u64;
        self.stats.assemblies += 1;
        self.current = Some(product);
        Ok(self.current.as_ref().expect("current 刚置 Some"))
    }

    /// 计数面快照(evidence 埋点)。
    pub fn stats(&self) -> ClasAsStats {
        self.stats
    }

    /// 当前产物(未装配 = None)。
    pub fn current(&self) -> Option<&AssembledBlas> {
        self.current.as_ref()
    }

    /// 显式驱逐(G8 显式策略同口径;下次装配强制重拼)。
    pub fn evict(&mut self) -> bool {
        if self.current.take().is_some() {
            self.stats.evictions += 1;
            true
        } else {
            false
        }
    }
}

/// 装配期一致性核验(L3 RED 臂锚):可见集与 BLAS 内容必须**逐簇精确一致**
/// (同序同长);错开一簇(首个分叉位置)即确定性 `Err`。
///
/// # Errors
/// 任一位置簇 id 漂移 / 长度失配 → `ClasError::VisibleBlasMismatch`。
pub fn verify_visible_blas_consistency(
    visible: &[VisibleClusterEntry],
    assembled: &AssembledBlas,
) -> Result<(), ClasError> {
    let blas = assembled.visible();
    let n = visible.len().max(blas.len());
    for i in 0..n {
        let v = visible.get(i);
        let b = blas.get(i);
        if v != b {
            return Err(ClasError::VisibleBlasMismatch {
                at: i,
                visible: v.map(|e| e.cluster_id),
                blas: b.map(|e| e.cluster_id),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 双腿选择(RXS-0351 L7:构建/装配期换腿纪律)
// ---------------------------------------------------------------------------

/// RT 合流腿(manifest 显式 variant;选择发生在构建/装配期,L7)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtLeg {
    /// CLAS 主腿(NV cluster AS;当帧 multi-indirect device 拼装 + 模板实例化)。
    ClasMain,
    /// 传统 triangles BLAS 回退腿(per-簇分组 = 按对象,L2;正确性基线)。
    TraditionalBlasFallback,
}

/// 主腿 device 支撑能力(由 `vk::probe_cluster_acceleration_structure` 快照蒸馏)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClasLegSupport {
    /// 扩展 + feature 均在位。
    Supported,
    /// 任一缺失(DEV_ENV_DEGRADE 诚实登记)。
    NotSupported,
}

/// 装配期换腿裁决:manifest 选主腿而能力缺失 → 装载 fail-closed
/// (**禁止运行期发现不支持后静默换腿**,L7 逐字)。
///
/// # Errors
/// `(ClasMain, NotSupported)` 产 `ClasError::ClasMainLegNotSupported`。
pub fn select_leg(manifest: RtLeg, support: ClasLegSupport) -> Result<RtLeg, ClasError> {
    match (manifest, support) {
        (RtLeg::ClasMain, ClasLegSupport::Supported) => Ok(RtLeg::ClasMain),
        (RtLeg::ClasMain, ClasLegSupport::NotSupported) => Err(ClasError::ClasMainLegNotSupported),
        (RtLeg::TraditionalBlasFallback, _) => Ok(RtLeg::TraditionalBlasFallback),
    }
}

// ---------------------------------------------------------------------------
// host 金标准(Möller–Trumbore 双面最近命中;逐命中一致对拍的参照面)
// ---------------------------------------------------------------------------

/// 光线(世界空间;`t` 有效区间闭 `[t_min, t_max]`)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// 原点。
    pub origin: [f32; 3],
    /// 方向(不要求归一)。
    pub dir: [f32; 3],
    /// 最近接受距离。
    pub t_min: f32,
    /// 最远接受距离。
    pub t_max: f32,
}

/// 逐命中记录(对拍元组;`t_bits` = 距离位模式,**容差 0** 比对)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitRecord {
    /// 有 committed 命中。
    pub committed: bool,
    /// 最近命中距离位模式(miss = 0)。
    pub t_bits: u32,
    /// 簇槽位(可见集序;device 面 = 回退腿实例槽位 / 主腿 CLAS geometry index)。
    pub cluster_slot: u32,
    /// 簇内图元下标(`primitiveIndex` 同口径)。
    pub primitive: u32,
}

/// miss 哨兵记录(committed=false,余字段 0)。
pub const MISS_RECORD: HitRecord = HitRecord {
    committed: false,
    t_bits: 0,
    cluster_slot: 0,
    primitive: 0,
};

/// 单三角形相交(Möller–Trumbore,**双面**,精确比较零 epsilon——命中边界
/// 与 device any-hit 同口径;命中须 `t ∈ [t_min, t_max]` 闭区间)。
/// 命中产 `(t, ())`,未命中/平行/区间外产 `None`。
fn tri_intersect(
    ray: &Ray,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
) -> Option<f32> {
    let sub = |x: [f32; 3], y: [f32; 3]| [x[0] - y[0], x[1] - y[1], x[2] - y[2]];
    let cross = |x: [f32; 3], y: [f32; 3]| {
        [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ]
    };
    let dot = |x: [f32; 3], y: [f32; 3]| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let p = cross(ray.dir, e2);
    let det = dot(e1, p);
    if det == 0.0 {
        return None; // 平行/退化:双面语义下不命中(与 HW 无 cull 但平行不交同口径)。
    }
    let inv = 1.0 / det;
    let tvec = sub(ray.origin, a);
    let u = dot(tvec, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(tvec, e1);
    let v = dot(ray.dir, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(e2, q) * inv;
    if t < ray.t_min || t > ray.t_max {
        return None;
    }
    Some(t)
}

/// host 金标准:逐簇(可见集序)逐图元精确求交,**最近命中**胜出;
/// 等距并列取先序簇/先序图元(确定性;fixture 设计无并列)。
pub fn host_trace_clusters(clusters: &[ClusterGeometry], ray: &Ray) -> HitRecord {
    let mut best: Option<(f32, u32, u32)> = None;
    for (slot, cluster) in clusters.iter().enumerate() {
        for (prim, t) in cluster.indices.iter().enumerate() {
            let a = cluster.positions[t[0] as usize];
            let b = cluster.positions[t[1] as usize];
            let c = cluster.positions[t[2] as usize];
            if let Some(t_hit) = tri_intersect(ray, a, b, c)
                && best.is_none_or(|(bt, _, _)| t_hit < bt)
            {
                best = Some((t_hit, slot as u32, prim as u32));
            }
        }
    }
    match best {
        Some((t, slot, prim)) => HitRecord {
            committed: true,
            t_bits: t.to_bits(),
            cluster_slot: slot,
            primitive: prim,
        },
        None => MISS_RECORD,
    }
}

/// 命中流 digest(FNV-1a 64;逐记录 committed/t_bits/cluster_slot/primitive
/// 依序混合——G7 RayQuery 对拍体例的 digest golden 面,容差 0)。
pub fn hit_stream_digest(hits: &[HitRecord]) -> u64 {
    let mut h = FNV_OFFSET;
    fnv_mix(&mut h, hits.len() as u64);
    for hit in hits {
        fnv_mix(&mut h, u64::from(hit.committed as u8));
        fnv_mix(&mut h, u64::from(hit.t_bits));
        fnv_mix(&mut h, u64::from(hit.cluster_slot));
        fnv_mix(&mut h, u64::from(hit.primitive));
    }
    h
}

// ---------------------------------------------------------------------------
// M94 确定性 fixture(程序化簇集场景;精确可表示坐标保证容差 0 可判)
// ---------------------------------------------------------------------------

/// M94 拼装 digest golden(`AssembledBlas::assembly_digest` 对 fixture 的冻结值;
/// 首跑实测 bless,此后漂移即红——L5「拼装 digest 等于 golden」的 host 锚)。
pub const M94_ASSEMBLY_DIGEST_GOLDEN: u64 = 0x0bc2_49b0_4298_11fd;
/// M94 命中流 digest golden(`hit_stream_digest` 对 fixture 期望表的冻结值;
/// 首跑实测 bless——L2 逐命中一致容差 0 的 digest 面)。
pub const M94_HIT_STREAM_DIGEST_GOLDEN: u64 = 0x1761_c961_3893_a90d;

/// M94 fixture(簇集 + 可见集 + 确定性光线集;host/device/对拍共用同一事实源)。
#[derive(Debug)]
pub struct M94Fixture {
    /// 可见集(簇 id/LOD;蒙皮版本恒 0,变换 id 恒 0 = 恒等)。
    pub visible: Vec<VisibleClusterEntry>,
    /// 逐簇几何(与 `visible` 等长平行,可见集序 = 槽位序)。
    pub clusters: Vec<ClusterGeometry>,
    /// 确定性光线集。
    pub rays: Vec<Ray>,
}

impl ClusterSource for M94Fixture {
    fn cluster(&self, id: u64) -> Option<&ClusterGeometry> {
        self.visible
            .iter()
            .position(|e| e.cluster_id == id)
            .map(|i| &self.clusters[i])
    }
}

/// 2×2 四边形簇(z 平面;4 顶点 2 三角形 `[0,1,2]/[0,2,3]`)。
fn quad_cluster(x0: f32, y0: f32, z: f32) -> ClusterGeometry {
    ClusterGeometry::new(
        vec![
            [x0, y0, z],
            [x0 + 2.0, y0, z],
            [x0 + 2.0, y0 + 2.0, z],
            [x0, y0 + 2.0, z],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    )
    .expect("fixture quad 合法")
}

/// 单三角形簇(z 平面;3 顶点 1 三角形——与四边形**异拓扑**,走直建臂)。
fn tri_cluster(x0: f32, y0: f32, z: f32) -> ClusterGeometry {
    ClusterGeometry::new(
        vec![[x0, y0, z], [x0 + 2.0, y0, z], [x0 + 1.0, y0 + 2.0, z]],
        vec![[0, 1, 2]],
    )
    .expect("fixture tri 合法")
}

/// M94 确定性场景:6 簇(5 四边形同拓扑 → Cluster Template 实例化臂;1 单三角形
/// 异拓扑 → 直建臂),全坐标小整数/半整数(f32 精确可表示);9 条轴向光线
/// (origin z=10,dir (0,0,-1)),命中距离 = `10 - z平面` 整数精确。
///
/// 槽位布局(可见集序 = 槽位序):
/// - slot0 id=100 lod1:quad(0,0,z=2) / slot1 id=101 lod1:quad(3,0,z=4)
/// - slot2 id=200 lod0:quad(0,3,z=6) / slot3 id=201 lod0:quad(3,3,z=3)
/// - slot4 id=300 lod0:tri(6,0,z=5)  / slot5 id=301 lod1:quad(1,1,z=8)
///
/// slot5 与 slot0 在 xy 域 [1,2]² 重叠(z=8 vs z=2)——跨簇最近命中轴:
/// 重叠域光线须命中 slot5(t=2)而非 slot0(t=8)。
pub fn m94_fixture() -> M94Fixture {
    let visible = vec![
        VisibleClusterEntry { cluster_id: 100, lod_level: 1, skin_version: 0, transform_id: 0 },
        VisibleClusterEntry { cluster_id: 101, lod_level: 1, skin_version: 0, transform_id: 0 },
        VisibleClusterEntry { cluster_id: 200, lod_level: 0, skin_version: 0, transform_id: 0 },
        VisibleClusterEntry { cluster_id: 201, lod_level: 0, skin_version: 0, transform_id: 0 },
        VisibleClusterEntry { cluster_id: 300, lod_level: 0, skin_version: 0, transform_id: 0 },
        VisibleClusterEntry { cluster_id: 301, lod_level: 1, skin_version: 0, transform_id: 0 },
    ];
    let clusters = vec![
        quad_cluster(0.0, 0.0, 2.0),
        quad_cluster(3.0, 0.0, 4.0),
        quad_cluster(0.0, 3.0, 6.0),
        quad_cluster(3.0, 3.0, 3.0),
        tri_cluster(6.0, 0.0, 5.0),
        quad_cluster(1.0, 1.0, 8.0),
    ];
    let ray = |x: f32, y: f32| Ray {
        origin: [x, y, 10.0],
        dir: [0.0, 0.0, -1.0],
        t_min: 0.0,
        t_max: 100.0,
    };
    let rays = vec![
        ray(0.5, 1.0),   // slot0 tri1 t=8
        ray(4.0, 0.5),   // slot1 tri0 t=6
        ray(0.5, 4.0),   // slot2 tri1 t=4
        ray(4.5, 4.0),   // slot3 tri0 t=7
        ray(7.0, 1.0),   // slot4 tri0 t=5
        ray(10.0, 10.0), // miss
        ray(1.25, 1.5),  // 重叠域:slot5 tri1 t=2(压 slot0 t=8)
        ray(2.5, 2.9),   // slot5 tri1 t=2
        ray(2.5, 1.25),  // slot5 tri0 t=2
    ];
    M94Fixture {
        visible,
        clusters,
        rays,
    }
}

/// M94 fixture 的 host 金标准期望表(逐光线;硬编码独立手算值,
/// host 参照实现的**对拍锚**——参照错了这里先红)。
pub fn m94_fixture_expected() -> Vec<HitRecord> {
    let hit = |t: f32, slot: u32, prim: u32| HitRecord {
        committed: true,
        t_bits: t.to_bits(),
        cluster_slot: slot,
        primitive: prim,
    };
    vec![
        hit(8.0, 0, 1),
        hit(6.0, 1, 0),
        hit(4.0, 2, 1),
        hit(7.0, 3, 0),
        hit(5.0, 4, 0),
        MISS_RECORD,
        hit(2.0, 5, 1),
        hit(2.0, 5, 1),
        hit(2.0, 5, 0),
    ]
}

impl M94Fixture {
    /// 可见集切片(测试/harness 便利面)。
    pub fn visible_slice(&self) -> &[VisibleClusterEntry] {
        &self.visible
    }
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> M94Fixture {
        m94_fixture()
    }

    /// 键管理:同内容同键(确定性);任一簇内容/字段漂移 → 异键。
    //@ spec: RXS-0351
    #[test]
    fn clas_blas_key_content_digest_semantics() {
        let f = fixture();
        let digests: Vec<u64> = f.clusters.iter().map(|c| c.content_digest()).collect();
        let k1 = ClasBlasKey::of_visible_set(&f.visible, &digests).expect("合法");
        let k2 = ClasBlasKey::of_visible_set(&f.visible, &digests).expect("合法");
        assert_eq!(k1, k2, "同内容同键");
        // 单簇几何漂移 → 异键。
        let mut d2 = digests.clone();
        d2[3] ^= 1;
        assert_ne!(k1, ClasBlasKey::of_visible_set(&f.visible, &d2).expect("合法"));
        // 可见集字段漂移(LOD)→ 异键。
        let mut v2 = f.visible.clone();
        v2[0].lod_level = 0;
        assert_ne!(k1, ClasBlasKey::of_visible_set(&v2, &digests).expect("合法"));
        // 长度失配 → fail-closed。
        assert_eq!(
            ClasBlasKey::of_visible_set(&f.visible, &digests[..5]),
            Err(ClasError::DigestLengthMismatch {
                visible: 6,
                digests: 5
            })
        );
    }

    /// 静态帧零 AS 构建(L4):同集复拼装 → 计数面零增量;键变 → 重建记账。
    //@ spec: RXS-0351
    #[test]
    fn static_frame_zero_as_build() {
        let f = fixture();
        let vis: Vec<VisibleClusterEntry> = f.visible_slice().to_vec();
        let mut asm = ClasAssembler::new(f);
        let key1 = asm.assemble_frame(&vis).expect("装配合法").key();
        let s1 = asm.stats();
        assert_eq!(s1.blas_builds, 6, "首帧:每簇一 BLAS");
        assert_eq!(s1.clas_builds, 6);
        assert_eq!(s1.template_builds, 1, "5 四边形同拓扑共享 1 模板");
        assert_eq!(s1.assemblies, 1);
        assert_eq!(s1.static_frames, 0);
        // 静态帧(同可见集):零构建。
        let key2 = asm.assemble_frame(&vis).expect("合法").key();
        assert_eq!(key1, key2);
        let s2 = asm.stats();
        assert_eq!(s2.blas_builds, s1.blas_builds, "静态帧 blas 构建零增量");
        assert_eq!(s2.clas_builds, s1.clas_builds);
        assert_eq!(s2.template_builds, s1.template_builds);
        assert_eq!(s2.assemblies, 1);
        assert_eq!(s2.static_frames, 1);
        // 内容变化(撤一簇):重建记账。
        asm.assemble_frame(&vis[..5]).expect("合法");
        let s3 = asm.stats();
        assert_eq!(s3.blas_builds, 6 + 5);
        assert_eq!(s3.assemblies, 2);
        // 显式驱逐 → 同集亦重建。
        assert!(asm.evict());
        asm.assemble_frame(&vis).expect("合法");
        assert_eq!(asm.stats().blas_builds, 6 + 5 + 6);
        assert_eq!(asm.stats().evictions, 1);
        // 空集 / 未知簇 → fail-closed。
        assert_eq!(
            asm.assemble_frame(&[]).unwrap_err(),
            ClasError::EmptyVisibleSet
        );
        let mut bogus = vis.clone();
        bogus[0].cluster_id = 999;
        assert_eq!(
            asm.assemble_frame(&bogus).unwrap_err(),
            ClasError::UnknownCluster(999)
        );
    }

    /// 错开一簇 RED(L3):可见集与 BLAS 内容逐簇一致 → Ok;任一漂移 → RED。
    //@ spec: RXS-0351
    #[test]
    fn visible_blas_mismatch_red() {
        let f = fixture();
        let vis: Vec<VisibleClusterEntry> = f.visible_slice().to_vec();
        let mut asm = ClasAssembler::new(f);
        let ok = asm.assemble_frame(&vis).expect("合法");
        assert!(verify_visible_blas_consistency(&vis, ok).is_ok());
        // 错开一簇(slot2 换入不在装配集的 id 777)→ 首个分叉位置 RED。
        let mut drifted = vis.clone();
        drifted[2].cluster_id = 777;
        assert_eq!(
            verify_visible_blas_consistency(&drifted, ok),
            Err(ClasError::VisibleBlasMismatch {
                at: 2,
                visible: Some(777),
                blas: Some(200),
            })
        );
        // 截短(少一簇)→ RED。
        assert!(matches!(
            verify_visible_blas_consistency(&vis[..5], ok),
            Err(ClasError::VisibleBlasMismatch { at: 5, .. })
        ));
    }

    /// 拼装计划:模板分组确定性 + 拼装 digest golden(L5 面)。
    //@ spec: RXS-0351
    #[test]
    fn assembly_plan_template_grouping_and_digest_golden() {
        let f = fixture();
        let vis: Vec<VisibleClusterEntry> = f.visible_slice().to_vec();
        let mut asm = ClasAssembler::new(f);
        let a = asm.assemble_frame(&vis).expect("合法");
        // 5 四边形同拓扑 → 模板 0(源 slot0,成员 {0,1,2,3,5});slot4 异拓扑直建。
        assert_eq!(
            a.templates(),
            &[TemplatePlan {
                source_slot: 0,
                members: vec![0, 1, 2, 3, 5]
            }]
        );
        for (slot, op) in a.ops().iter().enumerate() {
            let want = if slot == 4 {
                ClusterOp::DirectBuild
            } else {
                ClusterOp::Instantiate { template: 0 }
            };
            assert_eq!(*op, want, "slot{slot}");
        }
        // 拼装 digest 确定性 golden(双跑同值 + 硬编码锚)。
        assert_eq!(a.assembly_digest(), a.assembly_digest());
        assert_eq!(a.assembly_digest(), M94_ASSEMBLY_DIGEST_GOLDEN);
    }

    /// 换腿纪律(L7):主腿能力缺失 → fail-closed;回退腿恒可装载。
    //@ spec: RXS-0351
    #[test]
    fn select_leg_fail_closed_truth_table() {
        assert_eq!(
            select_leg(RtLeg::ClasMain, ClasLegSupport::Supported),
            Ok(RtLeg::ClasMain)
        );
        assert_eq!(
            select_leg(RtLeg::ClasMain, ClasLegSupport::NotSupported),
            Err(ClasError::ClasMainLegNotSupported)
        );
        assert_eq!(
            select_leg(RtLeg::TraditionalBlasFallback, ClasLegSupport::NotSupported),
            Ok(RtLeg::TraditionalBlasFallback)
        );
        assert_eq!(
            select_leg(RtLeg::TraditionalBlasFallback, ClasLegSupport::Supported),
            Ok(RtLeg::TraditionalBlasFallback)
        );
    }

    /// host 金标准自校验:fixture 逐光线期望表(硬编码手算锚)。
    //@ spec: RXS-0351
    #[test]
    fn host_golden_fixture_expected_table() {
        let f = fixture();
        let expected = m94_fixture_expected();
        assert_eq!(f.rays.len(), expected.len());
        for (i, ray) in f.rays.iter().enumerate() {
            let got = host_trace_clusters(&f.clusters, ray);
            assert_eq!(got, expected[i], "ray{i} 期望表分叉");
        }
        // 命中流 digest golden(容差 0 面;硬编码锚)。
        let digest = hit_stream_digest(&expected);
        assert_eq!(digest, M94_HIT_STREAM_DIGEST_GOLDEN);
    }

    /// 烘焙输入 ABI:32B 定长 + 字段序编码 + AABB 逐位 min/max 同口径。
    //@ spec: RXS-0351
    #[test]
    fn bake_record_abi_matches_logical_v2() {
        // 跨 crate ABI 锚:logical_v2 CLAS_RECORD_SIZE == 本记录尺寸。
        assert_eq!(
            size_of::<ClasBakeRecord>(),
            rurix_geom_pages::logical_v2::CLAS_RECORD_SIZE,
            "CLAS 烘焙记录 32B ABI 漂移"
        );
        let quad = quad_cluster(1.0, 1.0, 8.0);
        let rec = ClasBakeRecord::of_cluster(7, &quad);
        assert_eq!(rec.triangle_offset, 7);
        assert_eq!(rec.triangle_count, 2);
        assert_eq!(rec.aabb_min, [1.0, 1.0, 8.0]);
        assert_eq!(rec.aabb_max, [3.0, 3.0, 8.0]);
        // 编码字段序:offset(u32 LE) | count(u32 LE) | aabb 6×f32 LE。
        let enc = rec.encode_le();
        assert_eq!(&enc[0..4], &7u32.to_le_bytes());
        assert_eq!(&enc[4..8], &2u32.to_le_bytes());
        assert_eq!(&enc[8..12], &1.0f32.to_le_bytes());
        assert_eq!(&enc[28..32], &8.0f32.to_le_bytes());
    }

    /// 簇几何校核:空簇/索引越界/NaN → fail-closed;soup 展开序 = 图元序。
    //@ spec: RXS-0351
    #[test]
    fn cluster_geometry_validation_and_soup_order() {
        assert_eq!(
            ClusterGeometry::new(vec![[0.0; 3]], vec![]),
            Err(ClasError::EmptyCluster)
        );
        assert_eq!(
            ClusterGeometry::new(vec![[0.0; 3]], vec![[0, 1, 2]]),
            Err(ClasError::IndexOutOfRange)
        );
        assert_eq!(
            ClusterGeometry::new(vec![[f32::NAN; 3], [0.0; 3], [0.0; 3]], vec![[0, 1, 2]]),
            Err(ClasError::NonFiniteVertex)
        );
        let tri = tri_cluster(6.0, 0.0, 5.0);
        let soup = tri.triangle_soup();
        assert_eq!(soup.len(), 9);
        assert_eq!(&soup[0..3], &[6.0, 0.0, 5.0]);
        assert_eq!(&soup[6..9], &[7.0, 2.0, 5.0]);
    }
}
