//! GPU 蒙皮 host Kerbl 参照实现与距离分级更新率(G9.3 M92;
//! spec/virtual_geometry.md RXS-0353;RFC-0022 §4.2;D-1/D-3)。
//!
//! - **host 参照蒙皮求值器**(L1):cluster 感知 LBS——消费 skin_hdr/bone_idx
//!   段同构输入(每簇 `max_influences`/骨骼索引集/`bound_inflation`,ABI 见
//!   rurix-geom-pages `logical_v2`)+ 骨骼 palette,逐顶点输出蒙皮位置;累加
//!   序 = 权重行序(冻结),仅 `+`/`×`(IEEE 确定)。**定点化输入域**
//!   ([`SKIN_FIXED_POINT_SCALE`]/[`quantize_fixed`]):1/256 栅格上全部中间量
//!   精确可表 ⇒ host/未来 device 腿逐顶点对拍**容差 0**;浮点输入域容差待
//!   device 腿 measured 后经条款面冻结(P-09),本模块不手写。
//! - **保守包围体**(L2,Kerbl et al. 2021):位移 `|M_b·p − p|` 是 p 的凸函数
//!   ⇒ 骨 b 在簇静止 AABB 上的最大位移必在 8 角点取得;顶点蒙皮位移上界 =
//!   Σ w_i·δ_i ≤ max δ_i(权重和 ≤ 1 经 [`SkinError::WeightsNotNormalized`]
//!   fail-closed 保证)。保守 AABB = 静止 AABB 各轴外扩 `max δ_i +
//!   bound_inflation`;[`verify_bound_containment`] 逐顶点核验包含不变式,
//!   破坏 = fail-closed typed `Err`(剔除错杀即一致性校验失败,不设 UB)。
//! - **距离分级更新率**(L3,规范闭集):[`UpdateTier`] = {全速, 1/2, 1/3, 1/4}
//!   (10m 内全速);闭集外档位装配期 typed `Err` 拒绝;切换 = 距离纯函数,
//!   同输入确定;降级帧顶点缓冲**逐位不变**(skin cache 槽位零写)、保守包围体
//!   按未更新帧数 × 单帧位移上界放大;恢复全速当帧即更新、无跳变越界。
//! - **AS 更新计数**(L4/L5):更新帧经 `BlasCache::refit`(Deformable 策略)
//!   显式记账入 `AsStats`;静态帧(姿态 bit-equal 且档位不变)零 skin 写、
//!   零 refit、零构建——单测锚定。
//!
//! device 蒙皮 compute kernel 与 skin cache 显存布局归 CI 门代理统一接线
//! (`.rx` kernel 本波不落;host 参照即其对拍金标准,逐顶点容差 0 域见上)。

use rurix_pkg::sha256;

use crate::rt::as_manager::{BlasCache, BlasId, RefitOutcome};

// ---------------------------------------------------------------------------
// 定点化输入域(RXS-0353 L1:容差 0 对拍基准)
// ---------------------------------------------------------------------------

/// 定点栅格(1/256):|值| ≤ 8 时 1/256 栅格上的积与和(≤4 影响骨)全部落在
/// f32 24 位尾数精确域内 ⇒ 蒙皮求值逐位精确,host/device 对拍容差 0。
pub const SKIN_FIXED_POINT_SCALE: f32 = 256.0;

/// 量化到 1/256 定点栅格(最近舍入;输入域构造器)。
pub fn quantize_fixed(x: f32) -> f32 {
    (x * SKIN_FIXED_POINT_SCALE).round() / SKIN_FIXED_POINT_SCALE
}

/// 判定值是否落在定点栅格上(逐位;对拍输入域校验)。
pub fn is_fixed_point(x: f32) -> bool {
    x.is_finite() && quantize_fixed(x).to_bits() == x.to_bits()
}

// ---------------------------------------------------------------------------
// 骨骼 palette 与蒙皮输入
// ---------------------------------------------------------------------------

/// 行主 3×4 仿射(骨骼矩阵;与 `InstanceRecord::transform` 同约定)。
pub type BoneTransform = [[f32; 4]; 3];

/// 骨骼 palette(全局骨骼矩阵表;簇经 `bone_indices` 引用)。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkinPalette {
    /// 骨骼矩阵(行主 3×4)。
    pub bones: Vec<BoneTransform>,
}

impl SkinPalette {
    /// 姿态 digest(全部矩阵元 f32 位模式 sha256;静态帧判定的 bit-equal 面)。
    pub fn digest(&self) -> [u8; 32] {
        let mut bytes = Vec::with_capacity(self.bones.len() * 48 + 4);
        bytes.extend_from_slice(&(self.bones.len() as u32).to_le_bytes());
        for b in &self.bones {
            for row in b {
                for &x in row {
                    bytes.extend_from_slice(&x.to_bits().to_le_bytes());
                }
            }
        }
        sha256::digest(&bytes)
    }
}

/// 单簇蒙皮输入(skin_hdr/bone_idx/clas_aabb 段 ABI 的运行时同构面;
/// `weights` 逐顶点 (骨骼 id, 权重) 行,骨骼 id 必须 ∈ `bone_indices`)。
#[derive(Debug, Clone)]
pub struct ClusterSkinInput<'a> {
    /// 蒙皮元数据:最大影响骨数(skin_hdr.max_influences;> 0)。
    pub max_influences: u32,
    /// 蒙皮元数据:骨骼索引集(bone_idx 段,确定性升序)。
    pub bone_indices: &'a [u32],
    /// 蒙皮元数据:包围体膨胀系数(skin_hdr.bound_inflation;≥ 0)。
    pub bound_inflation: f32,
    /// 簇级静止 AABB min(clas_aabb 段)。
    pub rest_aabb_min: [f32; 3],
    /// 簇级静止 AABB max。
    pub rest_aabb_max: [f32; 3],
    /// 簇局部静止顶点(v1 顶点段视图)。
    pub vertices: &'a [[f32; 3]],
    /// 逐顶点权重行(与 `vertices` 等长;行内 ≤ `max_influences` 对)。
    pub weights: &'a [Vec<(u32, f32)>],
}

/// 蒙皮求值/包围体 fail-closed typed 错误(无 UB)。
#[derive(Debug, Clone, PartialEq)]
pub enum SkinError {
    /// 输入非法(NaN/inf 或负膨胀系数)。
    NonFiniteInput(&'static str),
    /// 顶点权重和 > 1(保守界 `Σ w_i·δ_i ≤ max δ_i` 的前提破坏)。
    WeightsNotNormalized {
        /// 顶点下标(簇局部)。
        vertex: u32,
        /// 权重和。
        sum: f32,
    },
    /// 单顶点影响骨数超簇 `max_influences`(skin_hdr 字段面破坏)。
    InfluenceExceedsMax {
        /// 顶点下标(簇局部)。
        vertex: u32,
    },
    /// 骨骼 id 不在簇骨骼索引集内(bone_idx 段破坏)。
    BoneOutsideClusterSet {
        /// 顶点下标(簇局部)。
        vertex: u32,
        /// 骨骼 id。
        bone: u32,
    },
    /// 骨骼 id 越出 palette。
    BoneOutOfPalette {
        /// 骨骼 id。
        bone: u32,
    },
    /// 保守包围体不含全部蒙皮后顶点(包含不变式破坏;剔除错杀面)。
    BoundContainmentViolation {
        /// 首个违例顶点(簇局部)。
        vertex: u32,
    },
    /// 蒙皮簇 BLAS 策略非 Deformable(refit 路径拒绝;AS 更新必须可计数)。
    AsPolicyRejected,
}

impl std::fmt::Display for SkinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkinError::NonFiniteInput(d) => write!(f, "蒙皮输入非法:{d}"),
            SkinError::WeightsNotNormalized { vertex, sum } => {
                write!(f, "顶点 {vertex} 权重和 {sum} > 1(保守界前提破坏)")
            }
            SkinError::InfluenceExceedsMax { vertex } => {
                write!(f, "顶点 {vertex} 影响骨数超簇 max_influences")
            }
            SkinError::BoneOutsideClusterSet { vertex, bone } => {
                write!(f, "顶点 {vertex} 骨骼 {bone} 不在簇骨骼索引集内")
            }
            SkinError::BoneOutOfPalette { bone } => write!(f, "骨骼 {bone} 越出 palette"),
            SkinError::BoundContainmentViolation { vertex } => {
                write!(f, "保守包围体不含蒙皮顶点 {vertex}(包含不变式破坏)")
            }
            SkinError::AsPolicyRejected => write!(f, "蒙皮簇 BLAS 策略非 Deformable"),
        }
    }
}

impl std::error::Error for SkinError {}

/// 输入校核(fail-closed 前置;求值/包围体共用)。
fn validate_input(input: &ClusterSkinInput<'_>, palette: &SkinPalette) -> Result<(), SkinError> {
    if !input.bound_inflation.is_finite() || input.bound_inflation < 0.0 {
        return Err(SkinError::NonFiniteInput("bound_inflation 非有限或负"));
    }
    if input.vertices.len() != input.weights.len() {
        return Err(SkinError::NonFiniteInput("顶点/权重表长不齐"));
    }
    for (vi, (v, row)) in input.vertices.iter().zip(input.weights.iter()).enumerate() {
        if !v.iter().all(|x| x.is_finite()) {
            return Err(SkinError::NonFiniteInput("顶点分量非有限"));
        }
        if row.len() > input.max_influences as usize {
            return Err(SkinError::InfluenceExceedsMax { vertex: vi as u32 });
        }
        let mut sum = 0.0f32;
        for &(b, w) in row {
            if !w.is_finite() || w < 0.0 {
                return Err(SkinError::NonFiniteInput("权重非有限或负"));
            }
            if !input.bone_indices.contains(&b) {
                return Err(SkinError::BoneOutsideClusterSet {
                    vertex: vi as u32,
                    bone: b,
                });
            }
            if b as usize >= palette.bones.len() {
                return Err(SkinError::BoneOutOfPalette { bone: b });
            }
            sum += w;
        }
        if sum > 1.0 {
            return Err(SkinError::WeightsNotNormalized {
                vertex: vi as u32,
                sum,
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// LBS 蒙皮求值器(host 参照;逐顶点对拍基线)
// ---------------------------------------------------------------------------

/// 单顶点 LBS:`p' = Σ_k w_k·(M_{b_k}·p)`,累加序 = 权重行序(冻结);
/// 仅 `+`/`×`(IEEE 逐位确定)。调用前必经 [`validate_input`](由
/// [`skin_cluster`] 统一执行)。
pub fn skin_vertex(
    pos: [f32; 3],
    weights: &[(u32, f32)],
    palette: &SkinPalette,
) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for &(b, w) in weights {
        let m = &palette.bones[b as usize];
        for (i, o) in out.iter_mut().enumerate() {
            *o += w * (m[i][0] * pos[0] + m[i][1] * pos[1] + m[i][2] * pos[2] + m[i][3]);
        }
    }
    out
}

/// 簇蒙皮(逐顶点;输出序 = 输入顶点序)。校核失败 = typed `Err`(fail-closed)。
pub fn skin_cluster(
    input: &ClusterSkinInput<'_>,
    palette: &SkinPalette,
) -> Result<Vec<[f32; 3]>, SkinError> {
    validate_input(input, palette)?;
    Ok(input
        .vertices
        .iter()
        .zip(input.weights.iter())
        .map(|(&v, row)| skin_vertex(v, row, palette))
        .collect())
}

// ---------------------------------------------------------------------------
// 保守包围体(RXS-0353 L2;Kerbl et al. 2021)
// ---------------------------------------------------------------------------

/// 保守蒙皮 AABB = 静止 AABB 各轴外扩 `max_bone δ_b + bound_inflation`;
/// `δ_b` = 骨 b 仿射位移在静止 AABB 8 角点的最大值(位移是凸函数 ⇒ 角点
/// 最大即全盒上界;顶点蒙皮位移 ≤ Σ w_i·δ_i ≤ max δ_i,权重和 ≤ 1 已校核)。
pub fn conservative_skinned_aabb(
    input: &ClusterSkinInput<'_>,
    palette: &SkinPalette,
) -> Result<([f32; 3], [f32; 3]), SkinError> {
    validate_input(input, palette)?;
    let (lo, hi) = (input.rest_aabb_min, input.rest_aabb_max);
    if !lo.iter().all(|x| x.is_finite()) || !hi.iter().all(|x| x.is_finite()) {
        return Err(SkinError::NonFiniteInput("rest AABB 非有限"));
    }
    let mut delta = 0.0f32;
    for &b in input.bone_indices {
        let m = &palette.bones[b as usize];
        for &x in &[lo[0], hi[0]] {
            for &y in &[lo[1], hi[1]] {
                for &z in &[lo[2], hi[2]] {
                    let c = [x, y, z];
                    let mut d2 = 0.0f32;
                    for (i, mc) in m.iter().enumerate() {
                        let t = mc[0] * c[0] + mc[1] * c[1] + mc[2] * c[2] + mc[3] - c[i];
                        d2 += t * t;
                    }
                    delta = delta.max(d2.sqrt());
                }
            }
        }
    }
    let grow = delta + input.bound_inflation;
    let mut out_lo = lo;
    let mut out_hi = hi;
    for k in 0..3 {
        out_lo[k] -= grow;
        out_hi[k] += grow;
    }
    Ok((out_lo, out_hi))
}

/// 包含不变式核验(fail-closed):包围体必须含**全部**蒙皮后顶点(含边界,
/// 闭区间比较)。违例 = `BoundContainmentViolation`(剔除错杀即一致性失败)。
pub fn verify_bound_containment(
    bound: &([f32; 3], [f32; 3]),
    skinned: &[[f32; 3]],
) -> Result<(), SkinError> {
    for (i, v) in skinned.iter().enumerate() {
        let inside = v
            .iter()
            .enumerate()
            .all(|(k, &x)| bound.0[k] <= x && x <= bound.1[k]);
        if !inside {
            return Err(SkinError::BoundContainmentViolation { vertex: i as u32 });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 距离分级更新率(RXS-0353 L3;规范闭集)
// ---------------------------------------------------------------------------

/// 全速半径(米;10m 内全速,RXS-0353 L3 冻结面)。
pub const FULL_SPEED_RADIUS_M: f32 = 10.0;
/// 1/2 档半径上界(实现冻结常量;确定性切换表)。
pub const HALF_RATE_RADIUS_M: f32 = 20.0;
/// 1/3 档半径上界(实现冻结常量;≥ 此距离进 1/4 档)。
pub const THIRD_RATE_RADIUS_M: f32 = 40.0;

/// 更新率档位(规范闭集 = {全速, 1/2, 1/3, 1/4};闭集外不可声明)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UpdateTier {
    /// 全速(每帧更新)。
    Full,
    /// 1/2 更新率(每 2 帧)。
    Half,
    /// 1/3 更新率(每 3 帧)。
    Third,
    /// 1/4 更新率(每 4 帧)。
    Quarter,
}

/// 档位闭集外声明错误(装配期确定性拒绝)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierError {
    /// 被拒的周期声明。
    pub period: u32,
}

impl std::fmt::Display for TierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "更新率周期 {} 不在规范闭集 {{1, 2, 3, 4}} 内", self.period)
    }
}

impl std::error::Error for TierError {}

impl UpdateTier {
    /// 规范闭集(升序;evidence 直方图桶序 = 本序)。
    pub const ALL: [UpdateTier; 4] = [
        UpdateTier::Full,
        UpdateTier::Half,
        UpdateTier::Third,
        UpdateTier::Quarter,
    ];

    /// 更新周期(帧)。
    pub fn period(self) -> u32 {
        match self {
            UpdateTier::Full => 1,
            UpdateTier::Half => 2,
            UpdateTier::Third => 3,
            UpdateTier::Quarter => 4,
        }
    }

    /// 闭集构造(装配期;周期 ∉ {1,2,3,4} = typed `Err` 拒绝)。
    pub fn from_period(period: u32) -> Result<Self, TierError> {
        Ok(match period {
            1 => UpdateTier::Full,
            2 => UpdateTier::Half,
            3 => UpdateTier::Third,
            4 => UpdateTier::Quarter,
            other => return Err(TierError { period: other }),
        })
    }

    /// 距离 → 档位(确定性纯函数;同输入逐位一致。NaN 保守落最远档)。
    pub fn for_distance(dist_m: f32) -> Self {
        if dist_m < FULL_SPEED_RADIUS_M {
            UpdateTier::Full
        } else if dist_m < HALF_RATE_RADIUS_M {
            UpdateTier::Half
        } else if dist_m < THIRD_RATE_RADIUS_M {
            UpdateTier::Third
        } else {
            UpdateTier::Quarter
        }
    }

    /// 本帧是否更新(帧号取模周期;确定性调度面)。
    pub fn updates_on(self, frame_serial: u64) -> bool {
        frame_serial.is_multiple_of(u64::from(self.period()))
    }

    /// 直方图桶下标(evidence `anim_update_tier_histogram` 埋点)。
    pub fn histogram_slot(self) -> usize {
        match self {
            UpdateTier::Full => 0,
            UpdateTier::Half => 1,
            UpdateTier::Third => 2,
            UpdateTier::Quarter => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// skin cache 与帧驱动(host 参照;device 布局归 CI 门代理接线)
// ---------------------------------------------------------------------------

/// skin cache 单簇槽位(布局/失效律 host 参照面:更新帧重写 + 版本 +1;
/// 降级/静态帧**逐位不变**)。
#[derive(Debug, Clone, PartialEq)]
pub struct SkinCacheSlot {
    /// 蒙皮后顶点(簇局部,与静止顶点等长)。
    pub positions: Vec<[f32; 3]>,
    /// 保守包围体(更新帧精确;降级帧按未更新帧数放大)。
    pub bound: ([f32; 3], [f32; 3]),
    /// 蒙皮版本(VisibleClusterSet `skin_version` 载荷源)。
    pub version: u32,
    /// 连续未更新帧数(包围体放大依据;更新帧清零)。
    pub stale_frames: u32,
}

/// skin cache(槽位序 = 蒙皮簇登记序,确定性)。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkinCache {
    /// 槽位表。
    pub slots: Vec<SkinCacheSlot>,
}

/// 蒙皮帧驱动统计(evidence 埋点源;单调递增快照)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkinningStats {
    /// 蒙皮求值 + skin cache 更新次数。
    pub skinned_updates: u64,
    /// 降级帧跳过次数(顶点缓冲逐位不变)。
    pub stale_skips: u64,
    /// 静态帧跳过次数(姿态 bit-equal 且档位不变;零 AS 构建)。
    pub static_skips: u64,
    /// 档位直方图(桶序 = [`UpdateTier::ALL`];`anim_update_tier_histogram`)。
    pub tier_histogram: [u64; 4],
}

/// 单簇帧输入(驱动一次 `drive_frame` 的面)。
#[derive(Debug, Clone, Copy)]
pub struct SkinnedClusterFrame<'a> {
    /// 蒙皮输入(顶点/权重/元数据)。
    pub input: &'a ClusterSkinInput<'a>,
    /// 相机距离(米;档位切换源)。
    pub distance_m: f32,
    /// 蒙皮簇 BLAS 句柄(Deformable 策略;更新帧 refit 计数)。
    pub blas: BlasId,
}

/// 蒙皮帧驱动器(持有 skin cache 与静态判定态;`AsStats` 计数消费面)。
#[derive(Debug, Default)]
pub struct SkinningDriver {
    /// skin cache(槽位序 = 登记序)。
    pub cache: SkinCache,
    /// 统计计数面。
    pub stats: SkinningStats,
    /// 逐簇**已应用**姿态 digest(静态帧判定锚:与当帧姿态 bit-equal 才静态;
    /// 降级未应用的姿态不构成「已应用」,待更新点必须补应用)。
    applied_pose: Vec<Option<[u8; 32]>>,
    last_tiers: Vec<UpdateTier>,
}

impl SkinningDriver {
    /// 新驱动(槽位数 = 蒙皮簇数)。
    pub fn new(cluster_count: usize) -> Self {
        Self {
            cache: SkinCache {
                slots: vec![
                    SkinCacheSlot {
                        positions: Vec::new(),
                        bound: ([0.0; 3], [0.0; 3]),
                        version: 0,
                        stale_frames: 0,
                    };
                    cluster_count
                ],
            },
            stats: SkinningStats::default(),
            applied_pose: vec![None; cluster_count],
            last_tiers: vec![UpdateTier::Full; cluster_count],
        }
    }

    /// 各簇当前蒙皮版本(VisibleClusterSet `skin_version` 直喂面)。
    pub fn versions(&self) -> Vec<u32> {
        self.cache.slots.iter().map(|s| s.version).collect()
    }

    /// 驱动一帧(RXS-0353 L3/L4/L5):
    /// - 档位 = 距离纯函数(直方图记账);
    /// - 静态帧(姿态 bit-equal 且档位不变):零 skin 写、零 refit、零构建;
    /// - 更新帧:蒙皮 → 包围体包含核验(fail-closed)→ skin cache 重写
    ///   (版本 +1)→ `BlasCache::refit`(AsStats 显式记账);
    /// - 降级帧:顶点缓冲逐位不变,包围体按 `max_frame_displacement` 逐轴放大
    ///   (最大未更新帧数语义:每降级帧 +1 个上界步长)。
    pub fn drive_frame(
        &mut self,
        frame_serial: u64,
        clusters: &[SkinnedClusterFrame<'_>],
        palette: &SkinPalette,
        max_frame_displacement: f32,
        blas: &mut BlasCache,
    ) -> Result<(), SkinError> {
        if !max_frame_displacement.is_finite() || max_frame_displacement < 0.0 {
            return Err(SkinError::NonFiniteInput("max_frame_displacement 非法"));
        }
        let pose_digest = palette.digest();
        for (i, cf) in clusters.iter().enumerate() {
            let tier = UpdateTier::for_distance(cf.distance_m);
            self.stats.tier_histogram[tier.histogram_slot()] += 1;
            // 静态帧 = 当帧姿态与**已应用**姿态 bit-equal 且档位不变(L5)。
            let static_frame =
                self.applied_pose[i] == Some(pose_digest) && self.last_tiers[i] == tier;
            if static_frame {
                // L5:静态帧零 AS 构建/零 refit/零顶点写。
                self.stats.static_skips += 1;
                continue;
            }
            self.last_tiers[i] = tier;
            if tier.updates_on(frame_serial) {
                let skinned = skin_cluster(cf.input, palette)?;
                let bound = conservative_skinned_aabb(cf.input, palette)?;
                verify_bound_containment(&bound, &skinned)?; // L2 fail-closed
                // L4:AS 更新经 AsStats 显式记账(Deformable refit 路径);
                // 失败即 typed Err,cache/版本不半应用(fail-closed 原子面)。
                match blas.refit(cf.blas, &skinned) {
                    Ok(RefitOutcome::Refitted(_)) => {}
                    _ => return Err(SkinError::AsPolicyRejected),
                }
                let slot = &mut self.cache.slots[i];
                slot.positions = skinned;
                slot.bound = bound;
                slot.version += 1;
                slot.stale_frames = 0;
                self.applied_pose[i] = Some(pose_digest);
                self.stats.skinned_updates += 1;
            } else {
                // 降级帧:顶点缓冲逐位不变;包围体按未更新帧数放大。
                self.stats.stale_skips += 1;
                let slot = &mut self.cache.slots[i];
                slot.stale_frames += 1;
                for k in 0..3 {
                    slot.bound.0[k] -= max_frame_displacement;
                    slot.bound.1[k] += max_frame_displacement;
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::as_manager::DynamicPolicy;

    const IDENTITY: BoneTransform = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];

    fn translate(x: f32, y: f32, z: f32) -> BoneTransform {
        [
            [1.0, 0.0, 0.0, x],
            [0.0, 1.0, 0.0, y],
            [0.0, 0.0, 1.0, z],
        ]
    }

    /// z 轴旋转 90°(定点精确:0/±1 元素)。
    fn rot_z_90() -> BoneTransform {
        [
            [0.0, -1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]
    }

    fn skin_input<'a>(
        vertices: &'a [[f32; 3]],
        weights: &'a [Vec<(u32, f32)>],
        bone_indices: &'a [u32],
        bound_inflation: f32,
        aabb: ([f32; 3], [f32; 3]),
    ) -> ClusterSkinInput<'a> {
        ClusterSkinInput {
            max_influences: 4,
            bone_indices,
            bound_inflation,
            rest_aabb_min: aabb.0,
            rest_aabb_max: aabb.1,
            vertices,
            weights,
        }
    }

    // ———— M92 RXS-0353 L1:蒙皮逐顶点 golden(定点域容差 0)————

    //@ spec: RXS-0353
    #[test]
    fn lbs_skinning_fixed_point_golden() {
        // 定点化输入域(1/256 栅格):全部中间量 f32 精确 ⇒ 逐顶点容差 0。
        let vertices: Vec<[f32; 3]> = [
            [1.0, 0.5, -0.25],
            [2.0, -1.0, 0.75],
            [0.0, 0.0, 0.0],
        ]
        .into_iter()
        .map(|v| v.map(quantize_fixed))
        .collect();
        let weights: Vec<Vec<(u32, f32)>> = vec![
            vec![(0, 0.5), (1, 0.5)],
            vec![(0, 0.75), (1, 0.25)],
            vec![(1, 1.0)],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(|(b, w)| (b, quantize_fixed(w))).collect())
        .collect();
        let palette = SkinPalette {
            bones: vec![translate(1.0, 2.0, 3.0), rot_z_90()],
        };
        let input = skin_input(
            &vertices,
            &weights,
            &[0, 1],
            0.0,
            ([-0.25, -1.0, -0.25], [2.0, 0.5, 0.75]),
        );
        let out = skin_cluster(&input, &palette).expect("合法输入");
        // golden(定点域整数手算,逐位锚定):
        // v0 = 0.5·(v0+(1,2,3)) + 0.5·rot(v0)
        //    = 0.5·(2,2.5,2.75) + 0.5·(-0.5,1,-0.25) = (0.75,1.75,1.25)
        // v1 = 0.75·(3,1,3.75) + 0.25·(1,2,0.75) = (2.5,1.25,3.0)
        // v2 = rot(0,0,0) = (0,0,0)
        let golden: [[f32; 3]; 3] = [[0.75, 1.75, 1.25], [2.5, 1.25, 3.0], [0.0, 0.0, 0.0]];
        for (o, g) in out.iter().zip(golden.iter()) {
            assert_eq!(
                o.map(f32::to_bits),
                g.map(f32::to_bits),
                "定点域逐顶点容差 0(位级全等)"
            );
        }
    }

    //@ spec: RXS-0353
    #[test]
    fn lbs_skinning_deterministic_double_run() {
        let vertices: Vec<[f32; 3]> = vec![[0.125, -2.5, 1.0], [3.25, 0.0, -0.5]];
        let weights: Vec<Vec<(u32, f32)>> =
            vec![vec![(0, 0.625), (1, 0.375)], vec![(1, 1.0)]];
        let palette = SkinPalette {
            bones: vec![rot_z_90(), translate(-1.0, 0.5, 2.0)],
        };
        let input = skin_input(
            &vertices,
            &weights,
            &[0, 1],
            0.0,
            ([-2.5, -2.5, -0.5], [3.25, 0.0, 1.0]),
        );
        let a = skin_cluster(&input, &palette).expect("run A");
        let b = skin_cluster(&input, &palette).expect("run B");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.map(f32::to_bits), y.map(f32::to_bits), "双跑逐位一致");
        }
    }

    //@ spec: RXS-0353
    #[test]
    fn lbs_skinning_input_validation_fail_closed() {
        let vertices: Vec<[f32; 3]> = vec![[0.0; 3]];
        // 权重和 > 1 ⇒ typed Err(保守界前提破坏)。
        let bad_sum = vec![vec![(0u32, 0.75f32), (1, 0.5)]];
        let palette = SkinPalette { bones: vec![IDENTITY, IDENTITY] };
        let input = skin_input(&vertices, &bad_sum, &[0, 1], 0.0, ([0.0; 3], [0.0; 3]));
        assert!(matches!(
            skin_cluster(&input, &palette),
            Err(SkinError::WeightsNotNormalized { vertex: 0, .. })
        ));
        // 骨骼不在簇索引集 ⇒ typed Err。
        let bad_bone = vec![vec![(7u32, 1.0f32)]];
        let input = skin_input(&vertices, &bad_bone, &[0, 1], 0.0, ([0.0; 3], [0.0; 3]));
        assert_eq!(
            skin_cluster(&input, &palette),
            Err(SkinError::BoneOutsideClusterSet { vertex: 0, bone: 7 })
        );
        // 骨骼越出 palette ⇒ typed Err。
        let oob = vec![vec![(1u32, 1.0f32)]];
        let input = skin_input(&vertices, &oob, &[1], 0.0, ([0.0; 3], [0.0; 3]));
        let one_bone = SkinPalette { bones: vec![IDENTITY] };
        assert_eq!(
            skin_cluster(&input, &one_bone),
            Err(SkinError::BoneOutOfPalette { bone: 1 })
        );
        // 负膨胀系数 ⇒ typed Err。
        let ok = vec![vec![(0u32, 1.0f32)]];
        let input = skin_input(&vertices, &ok, &[0], -0.5, ([0.0; 3], [0.0; 3]));
        assert!(matches!(
            skin_cluster(&input, &palette),
            Err(SkinError::NonFiniteInput(_))
        ));
    }

    // ———— M92 RXS-0353 L2:保守包围体包含不变式 ————

    //@ spec: RXS-0353
    #[test]
    fn conservative_bound_contains_all_skinned_adversarial() {
        // 对抗姿态序列:大平移 + 旋转 + 混合权重,任意姿态 100% 包含。
        let vertices: Vec<[f32; 3]> = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.25, 0.25, 0.5],
        ];
        let weights: Vec<Vec<(u32, f32)>> = vec![
            vec![(0, 1.0)],
            vec![(0, 0.5), (1, 0.5)],
            vec![(1, 1.0)],
            vec![(0, 0.25), (1, 0.75)],
        ];
        let aabb = ([0.0, 0.0, 0.0], [1.0, 1.0, 0.5]);
        // 姿态序列:含 90° 旋转(角点位移 √2·|c| 级)与 ±50 大平移的极端组合。
        let poses: Vec<SkinPalette> = vec![
            SkinPalette { bones: vec![IDENTITY, IDENTITY] },
            SkinPalette { bones: vec![rot_z_90(), translate(50.0, -50.0, 25.0)] },
            SkinPalette { bones: vec![translate(-100.0, 0.0, 0.0), rot_z_90()] },
        ];
        for (pi, palette) in poses.iter().enumerate() {
            let input = skin_input(&vertices, &weights, &[0, 1], 0.125, aabb);
            let skinned = skin_cluster(&input, palette).expect("合法输入");
            let bound = conservative_skinned_aabb(&input, palette).expect("包围体");
            verify_bound_containment(&bound, &skinned)
                .unwrap_or_else(|e| panic!("姿态 {pi} 包含不变式破坏:{e}"));
        }
    }

    //@ spec: RXS-0353
    #[test]
    fn conservative_bound_boundary_injection_contained() {
        // 边界注入:恒等姿态 + 零膨胀 ⇒ 包围体 = 静止 AABB,角点顶点恰落边界
        // (闭区间包含判定不得漏)。
        let vertices: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
        let weights: Vec<Vec<(u32, f32)>> = vec![vec![(0, 1.0)], vec![(0, 1.0)]];
        let palette = SkinPalette { bones: vec![IDENTITY] };
        let input = skin_input(&vertices, &weights, &[0], 0.0, ([0.0; 3], [1.0; 3]));
        let skinned = skin_cluster(&input, &palette).expect("合法输入");
        let bound = conservative_skinned_aabb(&input, &palette).expect("包围体");
        assert_eq!(bound, ([0.0; 3], [1.0; 3]));
        verify_bound_containment(&bound, &skinned).expect("边界顶点必须被包含(闭区间)");
    }

    //@ spec: RXS-0353
    #[test]
    fn conservative_bound_shrunk_variant_red() {
        // RED 臂:人为缩小包围体 variant 必须被包含核验检出(fail-closed)。
        let vertices: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let weights: Vec<Vec<(u32, f32)>> = vec![vec![(0, 1.0)], vec![(0, 1.0)]];
        let palette = SkinPalette { bones: vec![translate(4.0, 0.0, 0.0)] };
        let input = skin_input(&vertices, &weights, &[0], 0.0, ([0.0; 3], [1.0; 3]));
        let skinned = skin_cluster(&input, &palette).expect("合法输入");
        let bound = conservative_skinned_aabb(&input, &palette).expect("包围体");
        verify_bound_containment(&bound, &skinned).expect("未缩小包围体必须通过");
        // 人为缩小(各轴对称内缩 2.0)⇒ 蒙皮后顶点 x ∈ {4, 5} 越出上界 ⇒ RED。
        let shrunk = (
            [bound.0[0] + 2.0, bound.0[1] + 2.0, bound.0[2] + 2.0],
            [bound.1[0] - 2.0, bound.1[1] - 2.0, bound.1[2] - 2.0],
        );
        assert!(matches!(
            verify_bound_containment(&shrunk, &skinned),
            Err(SkinError::BoundContainmentViolation { .. })
        ));
    }

    // ———— M92 RXS-0353 L3:档位闭集与切换确定性 ————

    //@ spec: RXS-0353
    #[test]
    fn update_tier_closed_set_and_deterministic_switch() {
        // 规范闭集:{1, 2, 3, 4} 全可声明且往返一致。
        for (p, t) in [
            (1, UpdateTier::Full),
            (2, UpdateTier::Half),
            (3, UpdateTier::Third),
            (4, UpdateTier::Quarter),
        ] {
            assert_eq!(UpdateTier::from_period(p), Ok(t));
            assert_eq!(t.period(), p);
        }
        // 闭集外(0/5/6/u32::MAX)装配期确定性拒绝。
        for p in [0u32, 5, 6, u32::MAX] {
            assert_eq!(
                UpdateTier::from_period(p),
                Err(TierError { period: p }),
                "闭集外周期 {p} 必须拒绝"
            );
        }
        // 距离切换表(10m 内全速;边界确定性)。
        assert_eq!(UpdateTier::for_distance(0.0), UpdateTier::Full);
        assert_eq!(UpdateTier::for_distance(9.99), UpdateTier::Full);
        assert_eq!(UpdateTier::for_distance(10.0), UpdateTier::Half);
        assert_eq!(UpdateTier::for_distance(19.99), UpdateTier::Half);
        assert_eq!(UpdateTier::for_distance(20.0), UpdateTier::Third);
        assert_eq!(UpdateTier::for_distance(39.99), UpdateTier::Third);
        assert_eq!(UpdateTier::for_distance(40.0), UpdateTier::Quarter);
        assert_eq!(UpdateTier::for_distance(1000.0), UpdateTier::Quarter);
        // 同距离序列双跑逐位一致。
        let seq = [3.0f32, 12.0, 25.0, 80.0, 9.0, 41.0];
        let run = || seq.map(UpdateTier::for_distance);
        assert_eq!(run(), run());
        // 调度面:帧号取模周期。
        assert!(UpdateTier::Full.updates_on(7));
        assert!(UpdateTier::Half.updates_on(8) && !UpdateTier::Half.updates_on(7));
        assert!(UpdateTier::Third.updates_on(9) && !UpdateTier::Third.updates_on(8));
        assert!(UpdateTier::Quarter.updates_on(12) && !UpdateTier::Quarter.updates_on(11));
    }

    // ———— M92 RXS-0353 L4/L5:AS 更新计数与静态帧零构建 ————

    /// 单骨平移姿态场景的驱动 fixture(两簇同一网格内容 ⇒ 各自 BLAS)。
    #[allow(clippy::type_complexity)]
    fn drive_fixture() -> (
        Vec<[f32; 3]>,
        Vec<Vec<(u32, f32)>>,
        Vec<u32>,
        ([f32; 3], [f32; 3]),
    ) {
        let vertices: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let weights: Vec<Vec<(u32, f32)>> = vec![
            vec![(0, 1.0)],
            vec![(0, 1.0)],
            vec![(0, 1.0)],
        ];
        let bones = vec![0u32];
        let aabb = ([0.0, 0.0, 0.0], [1.0, 1.0, 0.0]);
        (vertices, weights, bones, aabb)
    }

    fn deformable_blas(cache: &mut BlasCache, positions: &[[f32; 3]]) -> BlasId {
        let indices: Vec<[u32; 3]> = vec![[0, 1, 2]];
        cache.get_or_build(
            positions,
            &indices,
            DynamicPolicy::Deformable {
                refit_budget_frames: 1,
            },
        )
    }

    //@ spec: RXS-0353
    #[test]
    fn skinned_as_update_counted_and_static_frame_zero_build() {
        let (vertices, weights, bones, aabb) = drive_fixture();
        let mut blas = BlasCache::new();
        let handle = deformable_blas(&mut blas, &vertices);
        let input = skin_input(&vertices, &weights, &bones, 0.0, aabb);
        let mut driver = SkinningDriver::new(1);
        let builds0 = blas.stats().blas_builds;
        let refits0 = blas.stats().refits;

        // 帧 0(近距全速):蒙皮 + refit ⇒ refits +1、builds +0;版本 0→1。
        let pose_a = SkinPalette { bones: vec![translate(1.0, 0.0, 0.0)] };
        driver
            .drive_frame(
                0,
                &[SkinnedClusterFrame { input: &input, distance_m: 5.0, blas: handle }],
                &pose_a,
                0.5,
                &mut blas,
            )
            .expect("帧 0 更新");
        assert_eq!(blas.stats().refits - refits0, 1, "更新帧 refit 计数非空可机核");
        assert_eq!(blas.stats().blas_builds - builds0, 0, "更新帧零全量构建");
        assert_eq!(driver.cache.slots[0].version, 1);
        let frame0_positions = driver.cache.slots[0].positions.clone();
        let frame0_bound = driver.cache.slots[0].bound;

        // 帧 1(同姿态 bit-equal、同档位):静态帧 ⇒ 零 refit、零构建、零顶点写。
        let r1 = blas.stats().refits;
        let b1 = blas.stats().blas_builds;
        driver
            .drive_frame(
                1,
                &[SkinnedClusterFrame { input: &input, distance_m: 5.0, blas: handle }],
                &pose_a,
                0.5,
                &mut blas,
            )
            .expect("静态帧");
        assert_eq!(blas.stats().refits - r1, 0, "静态帧零 refit");
        assert_eq!(blas.stats().blas_builds - b1, 0, "静态帧零 AS 构建(非零即 RED)");
        assert_eq!(driver.cache.slots[0].positions, frame0_positions);
        assert_eq!(driver.cache.slots[0].version, 1);
        assert_eq!(driver.stats.static_skips, 1);

        // 姿态变化 + 远距(1/4 档)帧 2:非静态但降级帧 ⇒ 顶点逐位不变、
        // 包围体按未更新帧数放大、零 refit。
        let pose_b = SkinPalette { bones: vec![translate(2.0, 0.0, 0.0)] };
        let r2 = blas.stats().refits;
        driver
            .drive_frame(
                2,
                &[SkinnedClusterFrame { input: &input, distance_m: 50.0, blas: handle }],
                &pose_b,
                0.5,
                &mut blas,
            )
            .expect("降级帧");
        assert_eq!(blas.stats().refits - r2, 0, "降级帧零 refit");
        assert_eq!(
            driver.cache.slots[0].positions, frame0_positions,
            "降级帧顶点缓冲逐位不变"
        );
        assert_eq!(driver.cache.slots[0].stale_frames, 1);
        assert!(
            driver.cache.slots[0].bound.1[0] > frame0_bound.1[0],
            "降级帧包围体按未更新帧数放大"
        );
        assert_eq!(driver.cache.slots[0].version, 1, "降级帧版本不增");

        // 帧 4(同姿态、4 % 4 == 0 更新点):恢复更新 ⇒ refits +1、版本 2、
        // 包围体含新姿态顶点(恢复无跳变越界)。
        let r4 = blas.stats().refits;
        driver
            .drive_frame(
                4,
                &[SkinnedClusterFrame { input: &input, distance_m: 50.0, blas: handle }],
                &pose_b,
                0.5,
                &mut blas,
            )
            .expect("档位更新点");
        assert_eq!(blas.stats().refits - r4, 1);
        assert_eq!(driver.cache.slots[0].version, 2);
        assert_eq!(driver.cache.slots[0].stale_frames, 0);
        let slot = &driver.cache.slots[0];
        verify_bound_containment(&slot.bound, &slot.positions).expect("恢复更新包含不变式");
        // pose_b = 平移 (2,0,0):蒙皮顶点 = 静止顶点 + (2,0,0)(逐位锚定)。
        let expect = [[2.0f32, 0.0, 0.0], [3.0, 0.0, 0.0], [2.0, 1.0, 0.0]];
        for (v, g) in slot.positions.iter().zip(expect.iter()) {
            assert_eq!(v.map(f32::to_bits), g.map(f32::to_bits));
        }
        // 直方图:Full×2 + Quarter×2。
        assert_eq!(driver.stats.tier_histogram, [2, 0, 0, 2]);
        assert_eq!(driver.stats.skinned_updates, 2);
        assert_eq!(driver.stats.stale_skips, 1);
    }

    //@ spec: RXS-0353
    #[test]
    fn skinned_cluster_static_policy_rejected_fail_closed() {
        let (vertices, weights, bones, aabb) = drive_fixture();
        let mut blas = BlasCache::new();
        let indices: Vec<[u32; 3]> = vec![[0, 1, 2]];
        let handle = blas.get_or_build(&vertices, &indices, DynamicPolicy::Static);
        let input = skin_input(&vertices, &weights, &bones, 0.0, aabb);
        let mut driver = SkinningDriver::new(1);
        let pose = SkinPalette { bones: vec![IDENTITY] };
        // Static 策略蒙皮簇:refit 被策略透传 ⇒ typed Err(AS 更新必须可计数)。
        assert_eq!(
            driver.drive_frame(
                0,
                &[SkinnedClusterFrame { input: &input, distance_m: 5.0, blas: handle }],
                &pose,
                0.5,
                &mut blas,
            ),
            Err(SkinError::AsPolicyRejected)
        );
    }
}
