//! 毛发 Marschner 三瓣与几何三档(G9.5 M114;RFC-0025 §4.E + §4.L;spec/
//! display_pipeline.md RXS-0372 L1~L5 逐条对齐)。
//!
//! //@ spec: RXS-0372
//!
//! 本模块承载 M114 毛发专项着色语义面:
//!
//! - **Marschner R/TT/TRT 三瓣**(L1):纵向/方位角分离参数化为资产属性
//!   ([`MarschnerParams`](crate::material::side_table::MarschnerParams)——每缕
//!   基调色、高光偏移、medulla 配置,经 RFC-0025 §4.L 资产化侧表扩展通道按材
//!   质槽 ID 索引接入);与参考实现**逐瓣对拍 golden**([`marschner_lobes`]
//!   三瓣 digest 分记);**瓣能量守恒**(归一化形状 × 闭集权重 Σw = 1 ⇒ 逐样本
//!   总瓣能 ≤ 1 机核,measured 冻结)。
//! - **单瓣置零 RED 臂**(L2):单瓣系数置零([`ZeroLobe`])的 RED 渲染独立有
//!   效——缺 TT 瓣必须可见差异,无差异即管线未接通
//!   ([`assert_lobe_wired`],`Err(LobeNotWired)`,RED)。
//! - **几何三档**(L3):近 strand / 中 card / 远 mesh([`HairTier`]);档间切
//!   换距离([`TierSwitchTable`])与 strand→card 股替换映射
//!   ([`bake_strand_replacement`],股聚类 + card 图集)由离线烘焙产出,**烘焙
//!   确定性 golden**(双构建逐位一致);card/mesh 档走默认半透明路径
//!   ([`TranslucencyPath::DefaultTranslucent`])。
//! - **strand 档强制精确 OIT——分项 not-triggered 登记**(L4):strand 档排序
//!   不可行,必须 linked-list 精确档([`request_strand_translucency`]——strand
//!   档请求排序 fallback/默认半透明路径即 `Err(StrandTierRequiresExactOit)`,
//!   排序依赖缺失 RED 锚);strand 档依赖 M120 精确档 benchmark 裁决数据,**数
//!   据可得性不足——strand 档分项登记 not-triggered 不充绿**
//!   ([`register_strand_tier`]:显式 [`StrandTierStatus::NotTriggered`] 结构,
//!   `counts_as_green = false`,承接锚「M120 精确档 benchmark 裁决数据落地后
//!   重判,兜底 G9.7 穷举」,M120 测量带数据可得性如实记录)。
//! - **触 32B 经 §4.L 修订行**(L5):Marschner 参数集经侧表通道接入单层
//!   closure 求值;32B 布局 0-byte(机核在
//!   [`crate::material::side_table`],M115 同通道)。
//!
//! 纪律:host 纯 safe 确定性;零新 FFI;无 device 依赖;`RURIX_REQUIRE_REAL=1`
//! 下以 host 确定性为准。

use rurix_pkg::sha256;

use crate::material::side_table::{
    LobeExtension, MarschnerParams, MaterialSideTable, SideTableError,
};
use crate::oit::selection::{ExactTierScope, exact_tier_scope};

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// 三瓣权重闭集(R/TT/TRT;Σ = 1,瓣能量守恒载体)。
pub const LOBE_WEIGHTS: [f32; 3] = [0.35, 0.45, 0.20];
/// 方位角 TT 瓣中心(π,透射)与宽度。
pub const AZ_TT_CENTER: f32 = std::f32::consts::PI;
/// 方位角 R 瓣宽度。
pub const AZ_R_WIDTH: f32 = 0.6;
/// 方位角 TT 瓣宽度。
pub const AZ_TT_WIDTH: f32 = 0.5;
/// 方位角 TRT 瓣宽度(双内反射再聚焦,峰在 0)。
pub const AZ_TRT_WIDTH: f32 = 0.8;
/// canonical 切换距离(strand 上界/ card 上界,米)。
pub const CANON_STRAND_MAX_M: f32 = 10.0;
pub const CANON_CARD_MAX_M: f32 = 60.0;
/// 股替换聚类尺寸(每股簇 strand 数;烘焙属性)。
pub const STRAND_CLUSTER_SIZE: u32 = 8;
/// strand 档承接锚(G9_ACCEPTANCE_MAP §3 M114 行登记字面)。
pub const STRAND_TIER_ANCHOR: &str = "M120 精确档 benchmark 裁决数据落地后重判,兜底 G9.7 穷举";

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed)
// ---------------------------------------------------------------------------

/// 毛发着色失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum HairError {
    /// 单瓣置零无可见差异(管线未接通,RED 锚)。
    LobeNotWired { lobe: &'static str },
    /// strand 档请求非精确 OIT 路径(排序依赖缺失,RED 锚)。
    StrandTierRequiresExactOit { requested: &'static str },
    /// 输入含非有限值。
    NonFiniteValue { stage: &'static str },
    /// 非 canonical 构造。
    NotCanonical(&'static str),
    /// 侧表面失败传导。
    SideTable(SideTableError),
}

impl std::fmt::Display for HairError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HairError::LobeNotWired { lobe } => write!(f, "单瓣置零无差异({lobe} 瓣未接通,RED)"),
            HairError::StrandTierRequiresExactOit { requested } => {
                write!(
                    f,
                    "strand 档必须 linked-list 精确 OIT,请求 {requested}(排序依赖缺失,RED)"
                )
            }
            HairError::NonFiniteValue { stage } => write!(f, "{stage} 含非有限值"),
            HairError::NotCanonical(why) => write!(f, "not canonical: {why}"),
            HairError::SideTable(e) => write!(f, "侧表: {e}"),
        }
    }
}

impl std::error::Error for HairError {}

impl From<SideTableError> for HairError {
    fn from(e: SideTableError) -> Self {
        HairError::SideTable(e)
    }
}

pub type Result<T> = std::result::Result<T, HairError>;

// ---------------------------------------------------------------------------
// Marschner 三瓣(L1:纵向/方位角分离;归一化形状 × 闭集权重)
// ---------------------------------------------------------------------------

/// 三瓣取值(逐瓣 golden 载体;着色值 = r·白 + (tt+trt)·基调色)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HairLobes {
    pub r: f32,
    pub tt: f32,
    pub trt: f32,
}

/// 归一化高斯(exp(−x²/(2β²)),峰值 1;纵向/方位角形状同源)。
fn gauss01(beta: f32, x: f32) -> f32 {
    (-x * x / (2.0 * beta * beta)).exp()
}

/// Marschner 三瓣求值(纵向 M_p = gauss01(width_p, θ_h − shift_p),θ_h =
/// (θ_i + θ_r)/2;方位角 N_R = gauss01(AZ_R_WIDTH, φ)(峰 0)/ N_TT =
/// gauss01(AZ_TT_WIDTH, φ − π) × medulla 衰减 / N_TRT = gauss01(AZ_TRT_WIDTH, φ);
/// 瓣值 = 权重 × M_p × N_p,Σ权重 = 1 ⇒ 逐样本总瓣能 ≤ 1)。
pub fn marschner_lobes(
    params: &MarschnerParams,
    theta_i: f32,
    theta_r: f32,
    phi: f32,
) -> Result<HairLobes> {
    params.validate()?;
    if ![theta_i, theta_r, phi].iter().all(|v| v.is_finite()) {
        return Err(HairError::NonFiniteValue {
            stage: "marschner angles",
        });
    }
    let theta_h = (theta_i + theta_r) * 0.5;
    let m_r = gauss01(params.width_r, theta_h - params.shift_r);
    let m_tt = gauss01(params.width_tt, theta_h - params.shift_tt);
    let m_trt = gauss01(params.width_trt, theta_h - params.shift_trt);
    let n_r = gauss01(AZ_R_WIDTH, phi);
    let n_tt = gauss01(AZ_TT_WIDTH, phi - AZ_TT_CENTER) / (1.0 + params.medulla);
    let n_trt = gauss01(AZ_TRT_WIDTH, phi);
    Ok(HairLobes {
        r: LOBE_WEIGHTS[0] * m_r * n_r,
        tt: LOBE_WEIGHTS[1] * m_tt * n_tt,
        trt: LOBE_WEIGHTS[2] * m_trt * n_trt,
    })
}

/// 单瓣闭集(置零 RED 臂用)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroLobe {
    R,
    Tt,
    Trt,
}

impl ZeroLobe {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZeroLobe::R => "R",
            ZeroLobe::Tt => "TT",
            ZeroLobe::Trt => "TRT",
        }
    }
}

/// 单瓣置零求值(L2 RED 面;置零瓣输出恒 0)。
pub fn marschner_lobes_zeroed(
    params: &MarschnerParams,
    theta_i: f32,
    theta_r: f32,
    phi: f32,
    zero: ZeroLobe,
) -> Result<HairLobes> {
    let mut l = marschner_lobes(params, theta_i, theta_r, phi)?;
    match zero {
        ZeroLobe::R => l.r = 0.0,
        ZeroLobe::Tt => l.tt = 0.0,
        ZeroLobe::Trt => l.trt = 0.0,
    }
    Ok(l)
}

/// 单瓣接线机核(L2):完整三瓣与置零变体的扫描 digest 必须不同——相同即管线
/// 未接通(`Err(LobeNotWired)`,RED)。
pub fn assert_lobe_wired(
    full_digest: &[u8; 32],
    zeroed_digest: &[u8; 32],
    lobe: ZeroLobe,
) -> Result<()> {
    if full_digest == zeroed_digest {
        return Err(HairError::LobeNotWired {
            lobe: lobe.as_str(),
        });
    }
    Ok(())
}

/// canonical 角度扫描(θ_r ∈ −0.6..0.6 步 0.05,φ ∈ 0..π 步 π/36;θ_i 固定 0.3)。
pub fn canonical_sweep(params: &MarschnerParams) -> Vec<HairLobes> {
    let mut out = Vec::new();
    let mut ti = -0.6f32;
    while ti <= 0.6f32 {
        let mut k = 0u32;
        while k <= 36 {
            let phi = k as f32 * std::f32::consts::PI / 36.0;
            out.push(marschner_lobes(params, 0.3, ti, phi).expect("sweep"));
            k += 1;
        }
        ti += 0.05;
    }
    out
}

/// 扫描逐瓣 digest(逐瓣对拍 golden 事实源;返回 (r, tt, trt) 三 digest)。
pub fn lobe_digests(sweep: &[HairLobes]) -> ([u8; 32], [u8; 32], [u8; 32]) {
    let mut rb = Vec::with_capacity(sweep.len() * 4);
    let mut tb = Vec::with_capacity(sweep.len() * 4);
    let mut trb = Vec::with_capacity(sweep.len() * 4);
    for l in sweep {
        rb.extend_from_slice(&l.r.to_le_bytes());
        tb.extend_from_slice(&l.tt.to_le_bytes());
        trb.extend_from_slice(&l.trt.to_le_bytes());
    }
    (
        sha256::digest(&rb),
        sha256::digest(&tb),
        sha256::digest(&trb),
    )
}

/// 瓣能量守恒机核(L1):逐样本总瓣能 ≤ 1(归一化形状 × Σ权重=1 ⇒ 上界 1);
/// 返回扫描最大总瓣能(measured 冻结面)。
pub fn max_total_lobe_energy(sweep: &[HairLobes]) -> f32 {
    sweep
        .iter()
        .map(|l| l.r + l.tt + l.trt)
        .fold(0.0f32, f32::max)
}

// ---------------------------------------------------------------------------
// 几何三档(L3)+ strand→card 股替换烘焙
// ---------------------------------------------------------------------------

/// 几何三档闭集。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HairTier {
    /// 近:strand(逐缕;强制精确 OIT)。
    Strand,
    /// 中:card(各向异性法线/切线贴图;默认半透明路径)。
    Card,
    /// 远:mesh(默认半透明路径)。
    Mesh,
}

impl HairTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            HairTier::Strand => "strand",
            HairTier::Card => "card",
            HairTier::Mesh => "mesh",
        }
    }
}

/// 档间切换距离表(烘焙属性闭集)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TierSwitchTable {
    pub strand_max_m: f32,
    pub card_max_m: f32,
}

impl TierSwitchTable {
    pub fn validate(&self) -> Result<()> {
        if !self.strand_max_m.is_finite()
            || !self.card_max_m.is_finite()
            || self.strand_max_m <= 0.0
            || self.card_max_m <= self.strand_max_m
        {
            return Err(HairError::NotCanonical("切换距离表非法"));
        }
        Ok(())
    }
}

/// canonical 切换距离表。
pub fn canonical_switch_table() -> TierSwitchTable {
    TierSwitchTable {
        strand_max_m: CANON_STRAND_MAX_M,
        card_max_m: CANON_CARD_MAX_M,
    }
}

/// 距离选档(闭集;确定性)。
pub fn tier_for_distance(table: &TierSwitchTable, distance_m: f32) -> Result<HairTier> {
    table.validate()?;
    if !distance_m.is_finite() || distance_m < 0.0 {
        return Err(HairError::NonFiniteValue {
            stage: "tier distance",
        });
    }
    Ok(if distance_m < table.strand_max_m {
        HairTier::Strand
    } else if distance_m < table.card_max_m {
        HairTier::Card
    } else {
        HairTier::Mesh
    })
}

/// 半透明路径闭集(L3/L4:card/mesh 档走默认半透明路径;strand 档必须精确
/// linked-list)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslucencyPath {
    /// 默认半透明路径(TAA 合成;card/mesh 档)。
    DefaultTranslucent,
    /// 排序 fallback(depth-sorted alpha;strand 档禁用——排序依赖缺失 RED 锚)。
    SortedFallback,
    /// 精确 linked-list(仅毛发 strand 作用域)。
    ExactLinkedList,
}

impl TranslucencyPath {
    pub fn as_str(&self) -> &'static str {
        match self {
            TranslucencyPath::DefaultTranslucent => "default_translucent",
            TranslucencyPath::SortedFallback => "sorted_fallback",
            TranslucencyPath::ExactLinkedList => "exact_linked_list",
        }
    }
}

/// 档位 → 半透明路径(L3:card/mesh ⇒ 默认半透明;strand ⇒ 精确 linked-list)。
pub fn tier_translucency_path(tier: HairTier) -> TranslucencyPath {
    match tier {
        HairTier::Card | HairTier::Mesh => TranslucencyPath::DefaultTranslucent,
        HairTier::Strand => TranslucencyPath::ExactLinkedList,
    }
}

/// strand 档半透明路径请求机核(L4 RED 锚):strand 档请求排序 fallback 或默认
/// 半透明路径即 `Err(StrandTierRequiresExactOit)`(排序依赖缺失);精确
/// linked-list 且作用域经 [`exact_tier_scope`] 核验(仅毛发 strand)⇒ Ok。
pub fn request_strand_translucency(path: TranslucencyPath) -> Result<()> {
    match path {
        TranslucencyPath::ExactLinkedList => {
            exact_tier_scope(ExactTierScope::HairStrandOnly)
                .map_err(|_| HairError::NotCanonical("精确档作用域核验失败"))?;
            Ok(())
        }
        other => Err(HairError::StrandTierRequiresExactOit {
            requested: other.as_str(),
        }),
    }
}

/// strand→card 股替换条目(离线烘焙产物)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplacementEntry {
    pub strand_begin: u32,
    pub strand_end: u32,
    pub card_id: u32,
    pub atlas_tile: u32,
}

/// strand→card 股替换映射(股聚类 + card 图集;烘焙确定性 = 双构建逐位一致)。
#[derive(Debug, Clone, PartialEq)]
pub struct StrandReplacementMap {
    pub entries: Vec<ReplacementEntry>,
}

/// 股替换烘焙器(离线;确定性:连续 strand 按 [`STRAND_CLUSTER_SIZE`] 聚类,
/// card 与图集瓦片 1:1 序分配)。
pub fn bake_strand_replacement(strand_count: u32) -> Result<StrandReplacementMap> {
    if strand_count == 0 {
        return Err(HairError::NotCanonical("零 strand"));
    }
    let mut entries = Vec::new();
    let mut begin = 0u32;
    let mut card = 0u32;
    while begin < strand_count {
        let end = (begin + STRAND_CLUSTER_SIZE).min(strand_count);
        entries.push(ReplacementEntry {
            strand_begin: begin,
            strand_end: end,
            card_id: card,
            atlas_tile: card,
        });
        begin = end;
        card += 1;
    }
    Ok(StrandReplacementMap { entries })
}

/// 替换映射 digest(烘焙确定性 golden 事实源)。
pub fn replacement_digest(map: &StrandReplacementMap) -> [u8; 32] {
    let mut buf = Vec::with_capacity(map.entries.len() * 16);
    for e in &map.entries {
        buf.extend_from_slice(&e.strand_begin.to_le_bytes());
        buf.extend_from_slice(&e.strand_end.to_le_bytes());
        buf.extend_from_slice(&e.card_id.to_le_bytes());
        buf.extend_from_slice(&e.atlas_tile.to_le_bytes());
    }
    sha256::digest(&buf)
}

// ---------------------------------------------------------------------------
// strand 档 not-triggered 登记(L4;消费 M120 测量冻结带)
// ---------------------------------------------------------------------------

/// strand 档分项状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrandTierStatus {
    /// not-triggered(数据可得性不足;不充绿)。
    NotTriggered,
}

/// M120 测量带数据可得性(如实记录面)。
#[derive(Debug, Clone, PartialEq)]
pub struct M120DataAvailability {
    /// 测量冻结带文件存在。
    pub measurements_present: bool,
    /// 测量冻结带 digest(文件字节 SHA-256;缺失为零)。
    pub measurements_digest: [u8; 32],
    /// linked-list 精确档测量记录数(`"linked_list_` 键出现次数)。
    pub linked_list_record_count: u32,
    /// 帧时腿为 host-only 参考值(M120 冻结带 host 字段声明)。
    pub host_only_reference: bool,
    /// 可得性裁决(如实记录)。
    pub verdict: &'static str,
}

/// strand 档分项登记(显式结构;`counts_as_green` 恒 false——条件未触发只表示
/// 决策已记录,不是成功)。
#[derive(Debug, Clone, PartialEq)]
pub struct StrandTierRegistration {
    pub status: StrandTierStatus,
    /// 承接锚(MAP §3 M114 行登记字面)。
    pub anchor: &'static str,
    /// 恒 false(不充绿)。
    pub counts_as_green: bool,
    /// M120 数据可得性。
    pub m120: M120DataAvailability,
}

/// strand 档分项登记(L4):消费 M120 测量冻结带文本(可选),如实记录数据可
/// 得性;**无论测量带是否在场,strand 档分项维持 not-triggered 不充绿**——
/// M120 本波仅产测量数据不定档(RXS-0371 L5),精确档 benchmark **裁决**数据
/// 未落地,承接锚 = [`STRAND_TIER_ANCHOR`]。
pub fn register_strand_tier(measurements_text: Option<&str>) -> StrandTierRegistration {
    let m120 = match measurements_text {
        None => M120DataAvailability {
            measurements_present: false,
            measurements_digest: [0u8; 32],
            linked_list_record_count: 0,
            host_only_reference: false,
            verdict: "M120 测量冻结带缺失——数据可得性不足",
        },
        Some(text) => {
            let count = text.matches("\"linked_list_").count() as u32;
            M120DataAvailability {
                measurements_present: true,
                measurements_digest: sha256::digest(text.as_bytes()),
                linked_list_record_count: count,
                host_only_reference: text.contains("host-only"),
                verdict: "M120 测量带已落地(host 确定性参照,linked-list 记录在案)但精确档裁决数据未落地(M120 仅测量不定档)——strand 档分项维持 not-triggered 不充绿,待承接锚重判",
            }
        }
    };
    StrandTierRegistration {
        status: StrandTierStatus::NotTriggered,
        anchor: STRAND_TIER_ANCHOR,
        counts_as_green: false,
        m120,
    }
}

// ---------------------------------------------------------------------------
// §4.L 侧表接入面(L5)
// ---------------------------------------------------------------------------

/// 毛发求值入口(侧表通道):槽命中 Marschner 扩展 ⇒ 三瓣求值;未命中 ⇒
/// `Err(NotCanonical)`(毛发材质必须显式携带 Marschner 参数集——无静默默认
/// 瓣,防侧表缺省冒充毛发着色)。
pub fn hair_params_from_side_table(
    table: &MaterialSideTable,
    slot: u32,
) -> Result<MarschnerParams> {
    match table.lookup(slot) {
        Some(LobeExtension::Marschner(p)) => Ok(*p),
        Some(LobeExtension::Burley(_)) => Err(HairError::SideTable(SideTableError::NotCanonical(
            "毛发槽误挂 Burley 扩展",
        ))),
        None => Err(HairError::NotCanonical("毛发槽缺 Marschner 参数集")),
    }
}

// ---------------------------------------------------------------------------
// canonical 场景(golden 事实源)
// ---------------------------------------------------------------------------

/// canonical Marschner 参数集(棕发:基调色 + 三瓣偏移/宽度 + medulla 0.3)。
pub fn canonical_marschner() -> MarschnerParams {
    MarschnerParams {
        base_color: [0.35, 0.22, 0.12],
        shift_r: -0.10,
        shift_tt: -0.05,
        shift_trt: -0.15,
        width_r: 0.10,
        width_tt: 0.08,
        width_trt: 0.14,
        medulla: 0.3,
    }
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0372
    #[test]
    fn lobes_energy_conservation_and_per_lobe_golden() {
        let p = canonical_marschner();
        let sweep = canonical_sweep(&p);
        let max_e = max_total_lobe_energy(&sweep);
        assert!(max_e <= 1.0, "瓣能量守恒违反: max {max_e}");
        assert!(max_e > 0.0);
        let (r, tt, trt) = lobe_digests(&sweep);
        assert_ne!(r, tt);
        assert_ne!(tt, trt);
        // 双跑位级一致。
        assert_eq!(lobe_digests(&canonical_sweep(&p)), (r, tt, trt));
    }

    //@ spec: RXS-0372
    #[test]
    fn single_lobe_zeroed_visible_diff_red() {
        let p = canonical_marschner();
        let full = canonical_sweep(&p);
        let full_d = {
            let mut buf = Vec::new();
            for l in &full {
                buf.extend_from_slice(&l.r.to_le_bytes());
                buf.extend_from_slice(&l.tt.to_le_bytes());
                buf.extend_from_slice(&l.trt.to_le_bytes());
            }
            sha256::digest(&buf)
        };
        for zero in [ZeroLobe::R, ZeroLobe::Tt, ZeroLobe::Trt] {
            let z = canonical_sweep_zeroed(&p, zero);
            let zd = {
                let mut buf = Vec::new();
                for l in &z {
                    buf.extend_from_slice(&l.r.to_le_bytes());
                    buf.extend_from_slice(&l.tt.to_le_bytes());
                    buf.extend_from_slice(&l.trt.to_le_bytes());
                }
                sha256::digest(&buf)
            };
            assert_lobe_wired(&full_d, &zd, zero).unwrap();
        }
        // sabotage:置零无差异 ⇒ 管线未接通 RED。
        assert!(matches!(
            assert_lobe_wired(&full_d, &full_d, ZeroLobe::Tt),
            Err(HairError::LobeNotWired { lobe: "TT" })
        ));
    }

    /// 置零扫描(单测/harness 共用形态)。
    fn canonical_sweep_zeroed(p: &MarschnerParams, zero: ZeroLobe) -> Vec<HairLobes> {
        let mut out = Vec::new();
        let mut ti = -0.6f32;
        while ti <= 0.6f32 {
            let mut k = 0u32;
            while k <= 36 {
                let phi = k as f32 * std::f32::consts::PI / 36.0;
                out.push(marschner_lobes_zeroed(p, 0.3, ti, phi, zero).expect("z"));
                k += 1;
            }
            ti += 0.05;
        }
        out
    }

    //@ spec: RXS-0372
    #[test]
    fn geometry_tiers_and_strand_exact_oit_red() {
        let table = canonical_switch_table();
        assert_eq!(tier_for_distance(&table, 3.0).unwrap(), HairTier::Strand);
        assert_eq!(tier_for_distance(&table, 30.0).unwrap(), HairTier::Card);
        assert_eq!(tier_for_distance(&table, 300.0).unwrap(), HairTier::Mesh);
        // card/mesh ⇒ 默认半透明路径。
        assert_eq!(
            tier_translucency_path(HairTier::Card),
            TranslucencyPath::DefaultTranslucent
        );
        assert_eq!(
            tier_translucency_path(HairTier::Mesh),
            TranslucencyPath::DefaultTranslucent
        );
        // strand ⇒ 精确 linked-list;请求排序 fallback / 默认半透明 ⇒ RED。
        assert!(request_strand_translucency(TranslucencyPath::ExactLinkedList).is_ok());
        assert!(matches!(
            request_strand_translucency(TranslucencyPath::SortedFallback),
            Err(HairError::StrandTierRequiresExactOit { .. })
        ));
        assert!(matches!(
            request_strand_translucency(TranslucencyPath::DefaultTranslucent),
            Err(HairError::StrandTierRequiresExactOit { .. })
        ));
    }

    //@ spec: RXS-0372
    #[test]
    fn strand_replacement_bake_deterministic() {
        let a = bake_strand_replacement(64).unwrap();
        let b = bake_strand_replacement(64).unwrap();
        assert_eq!(a, b);
        assert_eq!(replacement_digest(&a), replacement_digest(&b));
        assert_eq!(a.entries.len(), 8); // 64/8
        assert!(bake_strand_replacement(0).is_err());
    }

    //@ spec: RXS-0372
    #[test]
    fn strand_tier_not_triggered_registration() {
        let reg = register_strand_tier(None);
        assert_eq!(reg.status, StrandTierStatus::NotTriggered);
        assert!(!reg.counts_as_green);
        assert!(!reg.m120.measurements_present);
        assert_eq!(reg.anchor, STRAND_TIER_ANCHOR);
        // 测量带在场 ⇒ 如实记录可得性,仍 not-triggered。
        let text =
            "{\"host\": {\"device\": \"host-only(x)\"}, \"linked_list_4_image_digest\": \"aa\"}";
        let reg2 = register_strand_tier(Some(text));
        assert_eq!(reg2.status, StrandTierStatus::NotTriggered);
        assert!(!reg2.counts_as_green);
        assert!(reg2.m120.measurements_present);
        assert!(reg2.m120.linked_list_record_count >= 1);
        assert!(reg2.m120.host_only_reference);
    }

    //@ spec: RXS-0372
    #[test]
    fn side_table_consumption() {
        let mut t = MaterialSideTable::new();
        t.insert(0, LobeExtension::Marschner(canonical_marschner()), 1)
            .unwrap();
        let p = hair_params_from_side_table(&t, 0).unwrap();
        assert_eq!(p, canonical_marschner());
        // 缺参数集 ⇒ 拒(无静默默认瓣)。
        assert!(hair_params_from_side_table(&MaterialSideTable::new(), 0).is_err());
    }
}
