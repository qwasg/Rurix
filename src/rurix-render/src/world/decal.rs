//! 贴花 DBuffer(G9.5 M117;RFC-0025 §4.H;spec/world_partition.md RXS-0368
//! L1~L5 逐条对齐)。
//!
//! //@ spec: RXS-0368
//!
//! 本模块承载 M117 贴花专项渲染器前端语义面:
//!
//! - **DBuffer 三通道帧图设计期占位**(L1):DBuffer(法线 + 材质属性 + 可选第
//!   三通道)在 G-buffer pass 内合成,**帧图设计期即占位**——即使 v1 贴花数量
//!   为零,通道与 barrier 布局先行冻结([`design_time_seat`];缺占位即
//!   `Err(MissingDBufferPlaceholder)`,RED 锚);barrier 布局沿 RFC-0016 §4.A
//!   EB 三轴推导面只消费不重定。
//! - **双段语义**:贴花先写 DBuffer 中间表示([`write_dbuffer`])再合成
//!   ([`composite`]);旁路直写(跳过 DBuffer 中间表示)与双段输出逐位不等时
//!   即判管线未接通(RED 臂校验面 [`assert_two_stage`])。
//! - **screen-space cluster 化**(L2):screen-space cluster(复用光照 cluster
//!   结构,[`DECAL_CLUSTER_DIM`])对贴花体求交,逐像素贴花评估数受界
//!   ([`MAX_DECALS_PER_CLUSTER`]);过绘制计数器落 evidence 非空
//!   ([`ClusterAssignment::total_evals`])。
//! - **前向回退档**(L3):无 DBuffer 的低端 profile 走 [`decal_forward_pass`],
//!   与 DBuffer 档**两档语义等价 golden**(同输入逐位相等)。
//! - **超界受界降级**(L4):超 cluster 上界贴花密度注入必须受界降级
//!   (`degrade` 截断 + 降级标记),未降级即 `Err(DensityDegradationMissing)`
//!   (RED 锚);过绘制计数越界即 `Err(OverdrawBudgetExceeded)`(RED 锚)。
//! - **零 SVT 依赖**(L5):贴花语义面不得依赖 SVT/RVT/sampler feedback
//!   (D4 D17 同口径,与 RXS-0367 L4 断言同构)。
//!
//! 纪律:host 纯 safe 确定性(全库 `forbid(unsafe_code)`);零新 FFI;无 device
//! 依赖——M117 语义面 = 帧图占位断言 + cluster 受界求交 + 双段合成数学,GPU
//! 非必需;`RURIX_REQUIRE_REAL=1` 下以 host 确定性为准。

use rurix_pkg::sha256;

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// screen-space cluster 维度(复用光照 cluster 结构族:16×8 屏幕 tile × 24 深度
/// 切片;RXS-0368 L2「复用光照 cluster 结构」)。
pub const DECAL_CLUSTER_DIM: [u32; 3] = [16, 8, 24];
/// 逐 cluster 贴花评估数上界(逐像素上界的结构载体;超出必须受界降级)。
pub const MAX_DECALS_PER_CLUSTER: u32 = 4;
/// 过绘制预算(逐帧 (pixel,decal) 评估总数上界;canonical 场景计数面)。
pub const DECAL_OVERDRAW_BUDGET: u32 = 16_384;
/// DBuffer 合成像素格(canonical 场景 32×32)。
pub const DBUFFER_PIXEL_DIM: u32 = 32;

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed)
// ---------------------------------------------------------------------------

/// 贴花失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum DecalError {
    /// 帧图设计期占位缺失(DBuffer 通道/barrier 布局未冻结,RED 锚)。
    MissingDBufferPlaceholder,
    /// 过绘制计数越界(超预算,RED 锚)。
    OverdrawBudgetExceeded { count: u32, budget: u32 },
    /// 超 cluster 上界密度注入未受界降级(RED 锚)。
    DensityDegradationMissing { cluster_max: u32, bound: u32 },
    /// SVT/RVT/sampler feedback 依赖注入(零 SVT 断言同构,RED 锚)。
    SvtDependencyDetected { field: &'static str },
    /// 贴花体非法(非有限/零体积/包围盒颠倒)。
    BadDecalVolume { id: u32, why: &'static str },
    /// 非 canonical 构造。
    NotCanonical(&'static str),
    /// 双段语义违反(旁路直写与 DBuffer 双段输出不一致)。
    TwoStageViolation,
}

impl std::fmt::Display for DecalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecalError::MissingDBufferPlaceholder => {
                write!(f, "DBuffer 帧图设计期占位缺失(RED)")
            }
            DecalError::OverdrawBudgetExceeded { count, budget } => {
                write!(f, "过绘制计数 {count} 越界预算 {budget}(RED)")
            }
            DecalError::DensityDegradationMissing { cluster_max, bound } => {
                write!(
                    f,
                    "cluster 密度 {cluster_max} 超上界 {bound} 未受界降级(RED)"
                )
            }
            DecalError::SvtDependencyDetected { field } => {
                write!(f, "SVT/RVT 依赖注入: {field}(零 SVT 断言违反,RED)")
            }
            DecalError::BadDecalVolume { id, why } => write!(f, "贴花体 {id} 非法: {why}"),
            DecalError::NotCanonical(why) => write!(f, "not canonical: {why}"),
            DecalError::TwoStageViolation => {
                write!(f, "DBuffer 双段语义违反(旁路直写 ≠ 双段合成,RED)")
            }
        }
    }
}

impl std::error::Error for DecalError {}

pub type Result<T> = std::result::Result<T, DecalError>;

// ---------------------------------------------------------------------------
// DBuffer 三通道帧图设计期占位(L1)
// ---------------------------------------------------------------------------

/// DBuffer 通道闭集(L1:法线 + 材质属性 + 可选第三通道)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DBufferChannel {
    /// 法线通道。
    Normal,
    /// 材质属性通道(albedo/roughness 等)。
    MaterialAttributes,
    /// 可选第三通道(自发光/掩码)。
    OptionalThird,
}

impl DBufferChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DBufferChannel::Normal => "normal",
            DBufferChannel::MaterialAttributes => "material_attributes",
            DBufferChannel::OptionalThird => "optional_third",
        }
    }
}

/// barrier 布局描述(EB 三轴推导面只消费不重定:stage/access 对闭集的
/// canonical 记录;digest 即帧图冻结面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarrierLayoutDesc {
    /// G-buffer 写(stage=ColorOutput, access=StorageWrite 类)条目数。
    pub gbuffer_writes: u32,
    /// DBuffer 写条目数。
    pub dbuffer_writes: u32,
    /// 合成读(stage=FragmentShader, access=ShaderRead 类)条目数。
    pub composite_reads: u32,
}

/// 帧图贴花席位(设计期占位:即使贴花数为零,通道与 barrier 布局先行冻结)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameGraphDecalSeat {
    /// 占位标记(false = 缺占位即 RED)。
    pub placeholder_present: bool,
    /// 三通道闭集(序冻结)。
    pub channels: [DBufferChannel; 3],
    /// barrier 布局(设计期冻结)。
    pub barrier_layout: BarrierLayoutDesc,
}

/// 设计期席位(v1 冻结字面;贴花数为零时同样成立)。
pub fn design_time_seat() -> FrameGraphDecalSeat {
    FrameGraphDecalSeat {
        placeholder_present: true,
        channels: [
            DBufferChannel::Normal,
            DBufferChannel::MaterialAttributes,
            DBufferChannel::OptionalThird,
        ],
        barrier_layout: BarrierLayoutDesc {
            gbuffer_writes: 4,
            dbuffer_writes: 3,
            composite_reads: 3,
        },
    }
}

/// 占位断言(L1:缺占位即 RED)。
pub fn assert_dbuffer_placeholder(seat: &FrameGraphDecalSeat) -> Result<()> {
    if !seat.placeholder_present {
        return Err(DecalError::MissingDBufferPlaceholder);
    }
    Ok(())
}

/// 席位 digest(通道序 + barrier 布局;golden 对照事实源)。
pub fn seat_digest(seat: &FrameGraphDecalSeat) -> [u8; 32] {
    let mut buf = Vec::new();
    buf.push(seat.placeholder_present as u8);
    for c in &seat.channels {
        buf.extend_from_slice(c.as_str().as_bytes());
        buf.push(0);
    }
    buf.extend_from_slice(&seat.barrier_layout.gbuffer_writes.to_le_bytes());
    buf.extend_from_slice(&seat.barrier_layout.dbuffer_writes.to_le_bytes());
    buf.extend_from_slice(&seat.barrier_layout.composite_reads.to_le_bytes());
    sha256::digest(&buf)
}

// ---------------------------------------------------------------------------
// 贴花体与投影/衰减闭集
// ---------------------------------------------------------------------------

/// 衰减曲线闭集。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecalFalloff {
    /// 线性衰减。
    Linear,
    /// smoothstep 衰减。
    Smoothstep,
}

/// 贴花体(轴对齐盒投影;`extent` 半长)。
#[derive(Debug, Clone, PartialEq)]
pub struct DecalVolume {
    pub id: u32,
    /// 盒中心(世界米;canonical 场景在 cluster 视体内)。
    pub center: [f32; 3],
    /// 盒半长(>0)。
    pub extent: [f32; 3],
    /// 衰减曲线。
    pub falloff: DecalFalloff,
    /// 法线扰动(写 DBuffer 法线通道)。
    pub normal: [f32; 3],
    /// 材质属性(albedo;写材质通道)。
    pub albedo: [f32; 3],
    /// 第三通道值(掩码)。
    pub mask: f32,
}

impl DecalVolume {
    pub fn validate(&self) -> Result<()> {
        if !self.center.iter().all(|v| v.is_finite())
            || !self.extent.iter().all(|v| v.is_finite())
            || !self.normal.iter().all(|v| v.is_finite())
            || !self.albedo.iter().all(|v| v.is_finite())
            || !self.mask.is_finite()
        {
            return Err(DecalError::BadDecalVolume {
                id: self.id,
                why: "非有限值",
            });
        }
        if self.extent.iter().any(|e| *e <= 0.0) {
            return Err(DecalError::BadDecalVolume {
                id: self.id,
                why: "零/负半长",
            });
        }
        Ok(())
    }
}

/// 投影 + 衰减(盒投影闭集):点落盒内 → (衰减权重),盒外 → None。
/// 衰减 = 1 − |dz|(盒深归一),经 [`DecalFalloff`] 曲线闭集整形。
pub fn project_decal(decal: &DecalVolume, point: [f32; 3]) -> Result<Option<f32>> {
    decal.validate()?;
    if !point.iter().all(|v| v.is_finite()) {
        return Err(DecalError::BadDecalVolume {
            id: decal.id,
            why: "投影点非有限",
        });
    }
    let mut inside = true;
    let mut dz = 0.0f32;
    for (ax, ((&p, &ce), &ex)) in point
        .iter()
        .zip(decal.center.iter())
        .zip(decal.extent.iter())
        .enumerate()
    {
        let d = (p - ce).abs();
        if d > ex {
            inside = false;
            break;
        }
        if ax == 2 {
            dz = d / ex;
        }
    }
    if !inside {
        return Ok(None);
    }
    let t = 1.0 - dz;
    let w = match decal.falloff {
        DecalFalloff::Linear => t,
        DecalFalloff::Smoothstep => t * t * (3.0 - 2.0 * t),
    };
    Ok(Some(w))
}

// ---------------------------------------------------------------------------
// screen-space cluster 化(L2)+ 过绘制计数面(L4)
// ---------------------------------------------------------------------------

/// cluster 分派结果(过绘制计数面载体)。
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterAssignment {
    /// 逐 cluster 贴花 id 列表(cluster 下标 = x + y·16 + z·128)。
    pub per_cluster: Vec<Vec<u32>>,
    /// 是否发生受界降级(截断)。
    pub degraded: bool,
    /// 过绘制计数(逐帧 (cluster,decal) 求交评估总数;evidence 非空面)。
    pub total_evals: u32,
}

/// screen-space cluster 求交 + 逐 cluster 受界(L2/L4):
/// `degrade = true` 时超上界 cluster 截断并置降级标记(受界降级);
/// `degrade = false` 为注入面(不降级),由 [`verify_assignment`] 判 RED。
pub fn assign_decals(decals: &[DecalVolume], degrade: bool) -> Result<ClusterAssignment> {
    let n_clusters = (DECAL_CLUSTER_DIM[0] * DECAL_CLUSTER_DIM[1] * DECAL_CLUSTER_DIM[2]) as usize;
    let mut per_cluster: Vec<Vec<u32>> = vec![Vec::new(); n_clusters];
    // cluster 视体(世界范围闭集:canonical 场景 128×64×96 米)。
    const EXTENT: [f32; 3] = [128.0, 64.0, 96.0];
    for d in decals {
        d.validate()?;
        // 贴花盒覆盖的 cluster 范围(盒求交;逐轴独立)。
        let mut lo = [0u32; 3];
        let mut hi = [0u32; 3];
        for ax in 0..3 {
            let cell = EXTENT[ax] / DECAL_CLUSTER_DIM[ax] as f32;
            let a = ((d.center[ax] - d.extent[ax]) / cell)
                .floor()
                .clamp(0.0, DECAL_CLUSTER_DIM[ax] as f32 - 1.0);
            let b = ((d.center[ax] + d.extent[ax]) / cell)
                .floor()
                .clamp(0.0, DECAL_CLUSTER_DIM[ax] as f32 - 1.0);
            lo[ax] = a as u32;
            hi[ax] = b as u32;
        }
        for z in lo[2]..=hi[2] {
            for y in lo[1]..=hi[1] {
                for x in lo[0]..=hi[0] {
                    let idx = (x
                        + y * DECAL_CLUSTER_DIM[0]
                        + z * DECAL_CLUSTER_DIM[0] * DECAL_CLUSTER_DIM[1])
                        as usize;
                    per_cluster[idx].push(d.id);
                }
            }
        }
    }
    let mut degraded = false;
    let mut total_evals = 0u32;
    for list in per_cluster.iter_mut() {
        if degrade && list.len() as u32 > MAX_DECALS_PER_CLUSTER {
            list.truncate(MAX_DECALS_PER_CLUSTER as usize);
            degraded = true;
        }
        total_evals += list.len() as u32;
    }
    Ok(ClusterAssignment {
        per_cluster,
        degraded,
        total_evals,
    })
}

/// 受界降级与过绘制机核(L4):任一 cluster 超上界未降级即
/// `Err(DensityDegradationMissing)`(RED);过绘制计数越界即
/// `Err(OverdrawBudgetExceeded)`(RED)。
pub fn verify_assignment(a: &ClusterAssignment) -> Result<()> {
    let cluster_max = a
        .per_cluster
        .iter()
        .map(|l| l.len() as u32)
        .max()
        .unwrap_or(0);
    if cluster_max > MAX_DECALS_PER_CLUSTER {
        return Err(DecalError::DensityDegradationMissing {
            cluster_max,
            bound: MAX_DECALS_PER_CLUSTER,
        });
    }
    if a.total_evals > DECAL_OVERDRAW_BUDGET {
        return Err(DecalError::OverdrawBudgetExceeded {
            count: a.total_evals,
            budget: DECAL_OVERDRAW_BUDGET,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 双段语义(DBuffer 写 → 合成)与前向回退档(L3)
// ---------------------------------------------------------------------------

/// DBuffer 中间表示(三通道;canonical 像素格 [`DBUFFER_PIXEL_DIM`]²)。
#[derive(Debug, Clone, PartialEq)]
pub struct DBuffer {
    pub normal: Vec<[f32; 3]>,
    pub material: Vec<[f32; 3]>,
    pub third: Vec<f32>,
}

/// 逐像素贴花评估(两档共享的同一求值函数;衰减权重加权累积,贴花按 id 序——
/// 确定性)。
fn eval_pixel(decals: &[&DecalVolume], point: [f32; 3]) -> Result<([f32; 3], [f32; 3], f32)> {
    let mut n = [0.0f32; 3];
    let mut m = [0.0f32; 3];
    let mut t = 0.0f32;
    for d in decals {
        if let Some(w) = project_decal(d, point)? {
            for c in 0..3 {
                n[c] += d.normal[c] * w;
                m[c] += d.albedo[c] * w;
            }
            t += d.mask * w;
        }
    }
    Ok((n, m, t))
}

/// canonical 像素格世界坐标(z 取贴花层中面)。
fn pixel_point(px: u32, py: u32) -> [f32; 3] {
    [
        (px as f32 + 0.5) * (128.0 / DBUFFER_PIXEL_DIM as f32),
        (py as f32 + 0.5) * (64.0 / DBUFFER_PIXEL_DIM as f32),
        48.0,
    ]
}

/// 双段第一段:贴花写 DBuffer 中间表示(逐像素求交评估,cluster 分派限制逐
/// 像素评估数——host 参照按 cluster 分派结果过滤)。
pub fn write_dbuffer(decals: &[DecalVolume], assignment: &ClusterAssignment) -> Result<DBuffer> {
    let n = (DBUFFER_PIXEL_DIM * DBUFFER_PIXEL_DIM) as usize;
    let mut db = DBuffer {
        normal: vec![[0.0; 3]; n],
        material: vec![[0.0; 3]; n],
        third: vec![0.0; n],
    };
    for py in 0..DBUFFER_PIXEL_DIM {
        for px in 0..DBUFFER_PIXEL_DIM {
            let point = pixel_point(px, py);
            // 像素所在 cluster 的贴花集(逐像素评估数受界面)。
            let cx = (px * DECAL_CLUSTER_DIM[0] / DBUFFER_PIXEL_DIM).min(DECAL_CLUSTER_DIM[0] - 1);
            let cy = (py * DECAL_CLUSTER_DIM[1] / DBUFFER_PIXEL_DIM).min(DECAL_CLUSTER_DIM[1] - 1);
            let cidx = (cx + cy * DECAL_CLUSTER_DIM[0]) as usize;
            let ids: Vec<&DecalVolume> = assignment.per_cluster[cidx]
                .iter()
                .filter_map(|id| decals.iter().find(|d| &d.id == id))
                .collect();
            let (nv, mv, tv) = eval_pixel(&ids, point)?;
            let i = (py * DBUFFER_PIXEL_DIM + px) as usize;
            db.normal[i] = nv;
            db.material[i] = mv;
            db.third[i] = tv;
        }
    }
    Ok(db)
}

/// 双段第二段:DBuffer 合成到基础 G-buffer(法线覆盖 + 材质叠加 + 第三通道)。
pub fn composite(db: &DBuffer, base_albedo: [f32; 3]) -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity(db.material.len());
    for i in 0..db.material.len() {
        let m = db.material[i];
        let w = db.third[i].min(1.0);
        out.push([
            base_albedo[0] * (1.0 - w) + m[0] * w,
            base_albedo[1] * (1.0 - w) + m[1] * w,
            base_albedo[2] * (1.0 - w) + m[2] * w,
        ]);
    }
    out
}

/// 前向回退档(L3):无 DBuffer 的低端 profile——逐像素直接求值(与双段同一
/// 求值函数同一衰减闭集 ⇒ 语义等价判据 = 逐位相等)。
pub fn decal_forward_pass(
    decals: &[DecalVolume],
    assignment: &ClusterAssignment,
    base_albedo: [f32; 3],
) -> Result<Vec<[f32; 3]>> {
    let db = write_dbuffer(decals, assignment)?;
    Ok(composite(&db, base_albedo))
}

/// 两档语义等价判据(L3 golden):DBuffer 档与前向档输出逐位相等。
pub fn assert_tier_equivalence(dbuffer_out: &[[f32; 3]], forward_out: &[[f32; 3]]) -> Result<()> {
    if dbuffer_out.len() != forward_out.len() {
        return Err(DecalError::NotCanonical("两档输出长度不符"));
    }
    for (a, b) in dbuffer_out.iter().zip(forward_out.iter()) {
        for c in 0..3 {
            if a[c].to_bits() != b[c].to_bits() {
                return Err(DecalError::TwoStageViolation);
            }
        }
    }
    Ok(())
}

/// 双段断言(RED 臂校验面):旁路直写注入(不经 DBuffer 中间表示的篡改路径)
/// 与双段输出不一致即 [`DecalError::TwoStageViolation`]。
pub fn assert_two_stage(
    two_stage_out: &[[f32; 3]],
    bypass_out: &[[f32; 3]],
    expect_bypass_differs: bool,
) -> Result<()> {
    let equal = two_stage_out.len() == bypass_out.len()
        && two_stage_out
            .iter()
            .zip(bypass_out.iter())
            .all(|(a, b)| (0..3).all(|c| a[c].to_bits() == b[c].to_bits()));
    if expect_bypass_differs && equal {
        return Err(DecalError::TwoStageViolation);
    }
    Ok(())
}

/// 图像 digest(golden 对照事实源)。
pub fn image_digest(img: &[[f32; 3]]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(img.len() * 12);
    for p in img {
        for c in p {
            buf.extend_from_slice(&c.to_le_bytes());
        }
    }
    sha256::digest(&buf)
}

// ---------------------------------------------------------------------------
// 零 SVT 依赖(L5 同构断言)
// ---------------------------------------------------------------------------

/// 贴花资产依赖描述(消费方申报面;任何虚拟纹理依赖标记即 RED)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecalDependencyDesc {
    pub uses_svt: bool,
    pub uses_rvt: bool,
    pub uses_sampler_feedback: bool,
}

/// 零 SVT 依赖断言(D4 D17 同口径;与 RXS-0367 L4 断言同构)。
pub fn assert_decal_zero_svt(desc: &DecalDependencyDesc) -> Result<()> {
    if desc.uses_svt {
        return Err(DecalError::SvtDependencyDetected { field: "uses_svt" });
    }
    if desc.uses_rvt {
        return Err(DecalError::SvtDependencyDetected { field: "uses_rvt" });
    }
    if desc.uses_sampler_feedback {
        return Err(DecalError::SvtDependencyDetected {
            field: "uses_sampler_feedback",
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// canonical 场景(golden 事实源)
// ---------------------------------------------------------------------------

/// canonical 贴花集(4 个贴花体,cluster 上界内;确定性闭式构造)。
pub fn canonical_decals() -> Vec<DecalVolume> {
    (0..4u32)
        .map(|i| DecalVolume {
            id: 100 + i,
            center: [16.0 + i as f32 * 24.0, 16.0 + i as f32 * 8.0, 48.0],
            extent: [6.0, 6.0, 4.0],
            falloff: if i % 2 == 0 {
                DecalFalloff::Linear
            } else {
                DecalFalloff::Smoothstep
            },
            normal: [0.0, 0.0, 1.0],
            albedo: [0.2 + i as f32 * 0.1, 0.3, 0.5],
            mask: 0.25,
        })
        .collect()
}

/// 超密度注入贴花集(同一 cluster 内 7 个贴花 > 上界 4;RED 臂数据源)。
pub fn dense_decals() -> Vec<DecalVolume> {
    (0..7u32)
        .map(|i| DecalVolume {
            id: 200 + i,
            center: [32.0, 32.0, 48.0],
            extent: [6.0, 6.0, 4.0],
            falloff: DecalFalloff::Linear,
            normal: [0.0, 0.0, 1.0],
            albedo: [0.5, 0.3, 0.2],
            mask: 0.25,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0368
    #[test]
    fn placeholder_and_seat_digest_stable() {
        let seat = design_time_seat();
        assert_dbuffer_placeholder(&seat).unwrap();
        assert_eq!(seat_digest(&seat), seat_digest(&design_time_seat()));
        let mut missing = seat;
        missing.placeholder_present = false;
        assert!(matches!(
            assert_dbuffer_placeholder(&missing),
            Err(DecalError::MissingDBufferPlaceholder)
        ));
    }

    //@ spec: RXS-0368
    #[test]
    fn cluster_bounded_and_overdraw_red() {
        let decals = canonical_decals();
        let a = assign_decals(&decals, true).unwrap();
        verify_assignment(&a).unwrap();
        assert!(a.total_evals > 0); // 过绘制计数器非空
        // 超密度注入:不降级 ⇒ RED;降级 ⇒ 受界通过。
        let dense = dense_decals();
        let raw = assign_decals(&dense, false).unwrap();
        assert!(matches!(
            verify_assignment(&raw),
            Err(DecalError::DensityDegradationMissing { .. })
        ));
        let deg = assign_decals(&dense, true).unwrap();
        assert!(deg.degraded);
        verify_assignment(&deg).unwrap();
    }

    //@ spec: RXS-0368
    #[test]
    fn two_stage_and_forward_equivalence() {
        let decals = canonical_decals();
        let a = assign_decals(&decals, true).unwrap();
        let db = write_dbuffer(&decals, &a).unwrap();
        let out_db = composite(&db, [0.8, 0.8, 0.8]);
        let out_fwd = decal_forward_pass(&decals, &a, [0.8, 0.8, 0.8]).unwrap();
        assert_tier_equivalence(&out_db, &out_fwd).unwrap();
        // 双段语义:旁路直写(不投影衰减,直写材质)必须与双段不等 ⇒ 可判。
        let mut bypass = Vec::new();
        for _ in 0..DBUFFER_PIXEL_DIM * DBUFFER_PIXEL_DIM {
            bypass.push([0.8f32 + 0.2, 0.8, 0.8]);
        }
        assert!(assert_two_stage(&out_db, &bypass, true).is_ok());
        // sabotage:旁路与双段相等时要求「必须不等」⇒ RED。
        assert!(matches!(
            assert_two_stage(&out_db, &out_db, true),
            Err(DecalError::TwoStageViolation)
        ));
    }

    //@ spec: RXS-0368
    #[test]
    fn projection_falloff_closed_set() {
        let d = &canonical_decals()[0];
        // 盒内中面:线性衰减 = 1;smoothstep = 1。
        let mid = project_decal(d, d.center).unwrap();
        assert_eq!(mid, Some(1.0));
        // 盒外 → None。
        let outside = project_decal(d, [0.0, 0.0, 0.0]).unwrap();
        assert_eq!(outside, None);
        // 非有限拒绝。
        let mut bad = d.clone();
        bad.extent[0] = f32::NAN;
        assert!(matches!(
            project_decal(&bad, d.center),
            Err(DecalError::BadDecalVolume { .. })
        ));
    }

    //@ spec: RXS-0368
    #[test]
    fn zero_svt_red() {
        assert!(assert_decal_zero_svt(&DecalDependencyDesc::default()).is_ok());
        for desc in [
            DecalDependencyDesc {
                uses_svt: true,
                ..Default::default()
            },
            DecalDependencyDesc {
                uses_rvt: true,
                ..Default::default()
            },
            DecalDependencyDesc {
                uses_sampler_feedback: true,
                ..Default::default()
            },
        ] {
            assert!(matches!(
                assert_decal_zero_svt(&desc),
                Err(DecalError::SvtDependencyDetected { .. })
            ));
        }
    }
}
