//! G9.4 M99 屏幕级 SPG 自适应细分 + Radiance Cache 双级 host 面
//! (spec/global_illumination.md RXS-0360;RFC-0022 §4.8;门
//! `g9.p1.m99.spg_radiance_cache`)。
//!
//! 本模块 = M99 的 **host 数据面/判据面/缓存面**:
//! - **SPG 自适应细分**(RXS-0360 L1;G8 既有 1/16 均匀 probe + 3×3 滤波底座
//!   上**增量、不重定底座**):屏幕空间 probe 基线 **16 px/probe**
//!   ([`M99_BASE_CELL`]),按**深度/法线不连续性 + radiance 方差**双判据闭集
//!   ([`SubdivideCause`])驱动自适应细分(16→8→4 px,上限 [`M99_MAX_SUBDIV`]
//!   级);判据阈值**先 measured 后冻结**(禁手写掩盖,P-09——冻结批实测值与
//!   provenance 见 `milestones/g9/g9_m99_spg_rc_band.json` 与各常量文档)。
//! - **3×3 probe 空间滤波**(≈48×48 屏幕有效滤波):作用于最细级 probe
//!   radiance 图,权重律与 G8 底座 [`crate::gi::filter`] **同一公式面**
//!   (深度 `1/(1+t²)`、法线 `max(dot,0)^8`、边界截断归一),只换负载
//!   (radiance 图替 SH)——增量不重定。
//! - **Radiance Cache 双级**(RXS-0360 L2):**屏幕空间级** = 4 px tile 级
//!   radiance 缓存(复用 probe 历史;命中/失效/插入计数面 [`RcCounters`]);
//!   **世界空间 clipmap 级** = 未 measured 举证(RD-040 条件分项)——
//!   [`check_world_clipmap_trigger`] fail-closed 判 not-triggered 显式登记,
//!   [`world_clipmap_lookup`] 恒 typed Err,**不充绿**(不得以屏幕级绿色冒充
//!   世界级已触发)。
//! - **product importance sampling**(RXS-0360 L2):第一反弹采样 =
//!   BRDF×入射光 product IS(NEE×MIS 估计子,与 M96 megakernel shade①②③
//!   逐字同式);**关 product IS ⇒ 均匀半球采样替代 ⇒ 方差回归必须可检测**
//!   (负例 RED 臂独立有效;[`M99_PRODUCT_IS_VAR_RATIO_MIN`] measured 冻结)。
//! - **temporal 公共底座消费**(D2-Q14):probe 历史/时域累积一律经
//!   [`crate::temporal::common`] 公共底座(`reproject_sample` /
//!   `validate_history_with_mv`),**禁私写重投影**——私写 variant
//!   ([`HistoryPath::PrivateReprojectInjected`])即 RED(审计 fail-closed)。
//! - **golden 对拍**(RXS-0360 L4):屏幕级输出(直接光 + albedo×缓存 radiance,
//!   1 次间接弹射 ⇒ 匹配深度 [`M99_MATCHED_DEPTH`])对 M96 golden,
//!   [`M99SpgRcBand`] measured 后冻结(milestones/g9/g9_m99_spg_rc_band.json;
//!   带 = measured × [`M99_BAND_MARGIN`],禁手写 P-09);M96 同深度 digest 与
//!   M97 冻结带 `m96_cornell` 条目逐字相等(D2-Q7 门序消费锚)。
//!
//! ## 确定性协议(承 RXS-0357 L2 同律)
//! - 场景 = M96 冻结 fixture [`path_trace::m96_cornell_scene`];GBuffer =
//!   [`fallback_chain::gbuffer_prepass`] 同一产线(输入是输入不是结果)。
//! - probe 追踪 RNG = PCG32 单一流按索引寻址([`m99_rng`]:逐 probe 逐采样
//!   4 维 [bsdf_r1, bsdf_r2, nee_u, nee_v];流为输入非结果,G7.4 先例);
//!   逐 probe 独立顺序累加,禁 atomic 顺序敏感累加。
//! - 全部 f32;device kernel `kernels/g9_m99_spg_probe.rx` 分支判定一律
//!   min/max 算术门 + 短 selection 臂(M96 已机验白名单形),host 公式面与
//!   kernel 逐字同源。

use crate::gi::fallback_chain::{self, GBuffer, cosine_dir, l2_closest_hit, scene_tri};
use crate::gi::path_trace::{self, PtScene};
use crate::rt::ref_tracer::RAY_EPS;
use crate::temporal::common;
use crate::temporal::image::ImageF32;

// ---------------------------------------------------------------------------
// 冻结常量(细分判据阈值/档位参数;实现确定、非 stable,RFC-0022 §10 口径;
// 阈值先 measured 后冻结,禁手写掩盖 P-09)
// ---------------------------------------------------------------------------

/// M99 确定性协议冻结 seed(独立于 M96/M97/M98 流,避免跨里程碑流耦合)。
pub const M99_SEED: u64 = 0x5E99_A1C4_77D2_3B05;
/// SPG 屏幕空间 probe 基线块边长(16 px/probe,RFC-0022 §4.8 逐字)。
pub const M99_BASE_CELL: u32 = 16;
/// 自适应细分上限级数(16→8→4 px;level ∈ {0,1,2} 闭集)。
pub const M99_MAX_SUBDIV: u32 = 2;
/// 深度不连续性判据:cell 内**相邻像素对**(右/下邻)视 z 相对跳变
/// |z₁−z₂|/max(z₁,z₂) 的最大值超阈 ⇒ 细分(跳变度量对平滑透视梯度鲁棒,
/// 只对几何断裂/轮廓边触发;冻结批实测:平滑墙面 cell 跳变 ≤ 0.03,盒棱/
/// 轮廓 cell ≥ 0.3;阈值取其间,measured 后冻结)。
pub const M99_DEPTH_REL_DISCONT: f32 = 0.10;
/// 法线不连续性判据:cell 内逐像素法线与 cell 平均法线最小点积低于阈 ⇒ 细分
/// (冻结批实测:墙面 cell min dot = 1.0,盒棱 cell ≤ 0.62)。
pub const M99_NORMAL_DOT_MIN: f32 = 0.90;
/// radiance 方差判据:cell 内直接光亮度方差(无偏方差)超阈 ⇒ 细分
/// (冻结批实测:跨阴影边界 cell 方差 ≥ 0.045,平坦区 ≤ 0.004)。
pub const M99_VAR_MIN: f32 = 0.01;
/// 每 probe 采样数(逐 probe Σlum/Σlum² 方差面;M96 同律)。
pub const M99_PROBE_SPP: u32 = 8;
/// 历史混合系数(指数混合 out = prev·(1−α)+cur·α;沿 gi::temporal 默认)。
pub const M99_HISTORY_ALPHA: f32 = 0.1;
/// 匹配深度(probe 1 次间接弹射 + 主直接光 ⇒ M96 max_bounces=2 档 golden)。
pub const M99_MATCHED_DEPTH: u32 = 2;
/// M96 golden 参照 spp(与 M97/M98 门序锚同档)。
pub const M99_M96_GOLDEN_SPP: u32 = 64;
/// 容差带倍率(band = measured × margin;禁手写,P-09;沿 M96/M97/M98 口径)。
pub const M99_BAND_MARGIN: f64 = 2.0;
/// 关 product IS RED 臂检出阈:均匀半球采样替代的逐 probe 平均方差 /
/// product IS 方差 必须 ≥ 本阈(冻结批 host 参照实测 = 3.47e2,阈值 =
/// 实测留 ~3.5× margin 冻结;实测值进 band json `product_is_variance_ratio`
/// 字段,禁手写掩盖 P-09)。
pub const M99_PRODUCT_IS_VAR_RATIO_MIN: f64 = 100.0;
/// 关自适应 RED 臂检出阈(收敛特征偏离度量):细分触发 cell(level>0)内
/// 逐像素相对误差均值(对 M96 golden)——关自适应(基线 16 px)输出 /
/// 自适应 golden 输出 的比必须 ≥ 本阈(冻结批 host 参照实测 = 1.42,阈值 =
/// 实测留 ~18% margin 冻结;实测值进 band json `adaptive_deviation_ratio`
/// 字段,禁手写掩盖 P-09)。
pub const M99_ADAPTIVE_DEVIATION_RATIO_MIN: f64 = 1.2;
/// 大数门乘子(与 M96 kernel `big` 位级同值)。
const BIG: f32 = 1e30;
/// 正下界(与 M96 kernel `tiny` 位级同值)。
const TINY: f32 = 0.000001;

// ---------------------------------------------------------------------------
// 错误面(fail-closed typed Err;本模块一切失败为类型化拒绝,严禁 UB)
// ---------------------------------------------------------------------------

/// M99 host 面错误(细分/追踪/缓存/审计/容差带全部 fail-closed)。
#[derive(Debug, Clone, PartialEq)]
pub enum SpgError {
    /// 配置/输入非法(尺寸不符/阈值非有限/流长不符等)。
    InvalidConfig(String),
    /// 私写重投影检出:probe 历史/时域累积未经 temporal 公共底座
    /// (D2-Q14;私写 variant 即 RED,负例臂承载)。
    PrivateReprojection(String),
    /// 世界级 clipmap 级被请求服务但未 measured 举证(RD-040 条件分项:
    /// 登记 not-triggered,禁静默当绿)。
    WorldClipmapNotTriggered(String),
    /// 深度容差带错误(解析/缺条目/digest 不符/越带)。
    DepthBand(String),
}

impl std::fmt::Display for SpgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpgError::InvalidConfig(m) => write!(f, "配置非法: {m}"),
            SpgError::PrivateReprojection(m) => {
                write!(f, "私写重投影检出(D2-Q14 私写 variant 即 RED): {m}")
            }
            SpgError::WorldClipmapNotTriggered(m) => {
                write!(f, "世界级 clipmap 未举证(not-triggered): {m}")
            }
            SpgError::DepthBand(m) => write!(f, "深度容差带: {m}"),
        }
    }
}

impl std::error::Error for SpgError {}

// ---------------------------------------------------------------------------
// SPG 自适应细分(RXS-0360 L1:判据闭集 + 16 px/probe 基线)
// ---------------------------------------------------------------------------

/// 细分判据闭集(触发原因;三判据按冻结序首触发者记录)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubdivideCause {
    /// 深度不连续性(视 z 相对极差超 [`M99_DEPTH_REL_DISCONT`])。
    DepthDiscontinuity,
    /// 法线不连续性(最小点积低于 [`M99_NORMAL_DOT_MIN`])。
    NormalDiscontinuity,
    /// radiance 方差(直接光亮度 cell 方差超 [`M99_VAR_MIN`])。
    RadianceVariance,
}

impl SubdivideCause {
    /// evidence 名(稳定字面)。
    pub fn name(self) -> &'static str {
        match self {
            SubdivideCause::DepthDiscontinuity => "depth_discontinuity",
            SubdivideCause::NormalDiscontinuity => "normal_discontinuity",
            SubdivideCause::RadianceVariance => "radiance_variance",
        }
    }
}

/// 单枚屏幕 probe(叶 cell 锚定;世界位置/法线/albedo 自 GBuffer 锚点取)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpgProbe {
    /// 锚定像素坐标。
    pub anchor: [u32; 2],
    /// 叶 cell 矩形 [x0, y0, w, h](像素;level 0 = 16×16)。
    pub cell: [u32; 4],
    /// 细分级(0 = 基线 16 px;上限 [`M99_MAX_SUBDIV`])。
    pub level: u32,
    /// 世界位置(锚点主命中点)。
    pub pos: [f32; 3],
    /// 世界法线(朝相机翻转)。
    pub normal: [f32; 3],
    /// 线性 albedo。
    pub albedo: [f32; 3],
    /// 有效旗标(cell 内无主命中像素 ⇒ false,追踪/滤波/缓存全跳过)。
    pub valid: bool,
}

/// SPG 网格(自适应细分产物;像素 → probe 查找图为确定性闭式)。
#[derive(Debug, Clone, PartialEq)]
pub struct SpgGrid {
    /// 图宽(像素)。
    pub width: u32,
    /// 图高(像素)。
    pub height: u32,
    /// probe 数组(细分递归行序)。
    pub probes: Vec<SpgProbe>,
    /// 像素 → probe 下标(u32::MAX = 无 probe〔该像素主未命中且 cell 内无
    /// 有效锚〕)。
    pub pixel_probe: Vec<u32>,
    /// 逐基线 cell(16 px)最终细分级(0/1/2;自适应关 ⇒ 全 0)——产物
    /// digest 结构域(关自适应必然改变 level_map ⇒ digest 分叉,结构性保证
    /// 回归可检测)。
    pub level_map: Vec<u32>,
    /// 逐判据触发计数(细分闭集计数面,evidence 导出)。
    pub cause_counts: [u64; 3],
}

/// cell 统计(细分判据输入;自主命中像素计,全 f32 确定性)。
#[derive(Debug, Clone, Copy, Default)]
struct CellStats {
    /// 有效像素数。
    count: u32,
    /// 相邻像素对视 z 相对跳变最大值(不连续性度量)。
    z_jump_max: f32,
    /// 平均法线点积最小值(对 cell 平均法线)。
    n_dot_min: f32,
    /// 直接光亮度无偏方差。
    lum_var: f32,
}

/// cell 统计求值(扫描 cell 内像素;主未命中像素不入统)。深度不连续性 =
/// cell 内相邻像素对(右/下邻,双方均主命中)相对跳变最大值——平滑透视
/// 梯度不触发,几何断裂/轮廓边触发。
fn cell_stats(gb: &GBuffer, x0: u32, y0: u32, w: u32, h: u32) -> CellStats {
    let mut n = 0u32;
    let mut z_jump_max = 0.0f32;
    let mut mean_n = [0.0f64; 3];
    let mut sum = 0.0f64;
    let mut sumsq = 0.0f64;
    let (xe, ye) = ((x0 + w).min(gb.width), (y0 + h).min(gb.height));
    for y in y0..ye {
        for x in x0..xe {
            let i = (y * gb.width + x) as usize;
            if !gb.primary_hit[i] {
                continue;
            }
            n += 1;
            let z = gb.depth[i];
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx >= xe || ny >= ye {
                    continue;
                }
                let j = (ny * gb.width + nx) as usize;
                if !gb.primary_hit[j] {
                    continue;
                }
                let z2 = gb.depth[j];
                let jump = (z - z2).abs() / z.max(z2).max(TINY);
                z_jump_max = z_jump_max.max(jump);
            }
            for (c, m) in mean_n.iter_mut().enumerate() {
                *m += f64::from(gb.nrm[i * 3 + c]);
            }
            let lum = (f64::from(gb.direct[i * 3])
                + f64::from(gb.direct[i * 3 + 1])
                + f64::from(gb.direct[i * 3 + 2]))
                / 3.0;
            sum += lum;
            sumsq += lum * lum;
        }
    }
    if n == 0 {
        return CellStats::default();
    }
    let inv = 1.0 / f64::from(n);
    let mean = [mean_n[0] * inv, mean_n[1] * inv, mean_n[2] * inv];
    let ml = (mean[0] * mean[0] + mean[1] * mean[1] + mean[2] * mean[2])
        .sqrt()
        .max(1e-12);
    let mut n_dot_min = f32::INFINITY;
    for y in y0..ye {
        for x in x0..xe {
            let i = (y * gb.width + x) as usize;
            if !gb.primary_hit[i] {
                continue;
            }
            let d = (gb.nrm[i * 3] as f64 * mean[0]
                + gb.nrm[i * 3 + 1] as f64 * mean[1]
                + gb.nrm[i * 3 + 2] as f64 * mean[2])
                / ml;
            n_dot_min = n_dot_min.min(d as f32);
        }
    }
    let lum_mean = sum * inv;
    // 无偏方差 = E[x²] − E[x]²(n≥2 时按 n/(n−1) 无偏修正;确定性闭式)。
    let mut lum_var = (sumsq * inv - lum_mean * lum_mean).max(0.0);
    if n >= 2 {
        lum_var *= f64::from(n) / f64::from(n - 1);
    }
    CellStats {
        count: n,
        z_jump_max,
        n_dot_min,
        lum_var: lum_var as f32,
    }
}

/// 细分判据三元(纯函数;判据闭集 [深度, 法线, 方差] 独立评估——一 cell
/// 可同时触发多判据,计数面分判据独立计数;全 false ⇒ 不细分)。
fn subdivide_triggers(st: &CellStats) -> [bool; 3] {
    if st.count < 2 {
        return [false; 3]; // 有效像素不足,判据不可得 ⇒ 不细分(显式)
    }
    [
        st.z_jump_max > M99_DEPTH_REL_DISCONT,
        st.n_dot_min < M99_NORMAL_DOT_MIN,
        st.lum_var > M99_VAR_MIN,
    ]
}

/// 叶 cell 建 probe(锚 = cell 中心像素;无效则行序扫描首个主命中像素;
/// 全无 ⇒ valid=false 占位)。
fn make_probe(gb: &GBuffer, cell: [u32; 4], level: u32) -> SpgProbe {
    let [x0, y0, w, h] = cell;
    let cx = (x0 + w / 2).min(gb.width - 1);
    let cy = (y0 + h / 2).min(gb.height - 1);
    let mut anchor = [cx, cy];
    if !gb.primary_hit[(cy * gb.width + cx) as usize] {
        let mut found = false;
        'scan: for y in y0..(y0 + h).min(gb.height) {
            for x in x0..(x0 + w).min(gb.width) {
                if gb.primary_hit[(y * gb.width + x) as usize] {
                    anchor = [x, y];
                    found = true;
                    break 'scan;
                }
            }
        }
        if !found {
            return SpgProbe {
                anchor,
                cell,
                level,
                pos: [0.0; 3],
                normal: [0.0; 3],
                albedo: [0.0; 3],
                valid: false,
            };
        }
    }
    let i = (anchor[1] * gb.width + anchor[0]) as usize;
    let n = [gb.nrm[i * 3], gb.nrm[i * 3 + 1], gb.nrm[i * 3 + 2]];
    // 主命中点 = 二次射线原点回退 n·RAY_EPS(与 GBuffer 预传递同一产线)。
    let p = [
        gb.sec_o[i * 3] - n[0] * RAY_EPS,
        gb.sec_o[i * 3 + 1] - n[1] * RAY_EPS,
        gb.sec_o[i * 3 + 2] - n[2] * RAY_EPS,
    ];
    SpgProbe {
        anchor,
        cell,
        level,
        pos: p,
        normal: n,
        albedo: [gb.alb[i * 3], gb.alb[i * 3 + 1], gb.alb[i * 3 + 2]],
        valid: true,
    }
}

/// SPG 网格构建(自适应细分;`adaptive=false` ⇒ 全基线 16 px 均匀——
/// 关自适应 RED 臂注入 variant)。判据阈值冻结常量;同输入确定(纯函数)。
pub fn build_spg_grid(gb: &GBuffer, adaptive: bool) -> SpgGrid {
    let pixel_count = (gb.width * gb.height) as usize;
    let mut probes: Vec<SpgProbe> = Vec::new();
    let mut pixel_probe = vec![u32::MAX; pixel_count];
    let base_w = gb.width.div_ceil(M99_BASE_CELL);
    let base_h = gb.height.div_ceil(M99_BASE_CELL);
    let mut level_map = vec![0u32; (base_w * base_h) as usize];
    let mut cause_counts = [0u64; 3];
    // 递归细分(闭式:level 上限 + 判据闭集;递归深度 ≤ M99_MAX_SUBDIV)。
    fn subdivide(
        gb: &GBuffer,
        adaptive: bool,
        cell: [u32; 4],
        level: u32,
        probes: &mut Vec<SpgProbe>,
        pixel_probe: &mut [u32],
        cause_counts: &mut [u64; 3],
    ) -> u32 {
        let st = cell_stats(gb, cell[0], cell[1], cell[2], cell[3]);
        let triggers = if adaptive && level < M99_MAX_SUBDIV {
            subdivide_triggers(&st)
        } else {
            [false; 3]
        };
        if !triggers.iter().any(|&t| t) {
            // 叶:建 probe + 填像素查找图。
            let id = probes.len() as u32;
            let probe = make_probe(gb, cell, level);
            probes.push(probe);
            let [x0, y0, w, h] = cell;
            for y in y0..(y0 + h).min(gb.height) {
                for x in x0..(x0 + w).min(gb.width) {
                    pixel_probe[(y * gb.width + x) as usize] = id;
                }
            }
            return level;
        }
        for (k, &t) in triggers.iter().enumerate() {
            cause_counts[k] += u64::from(t);
        }
        // 2×2 四分(行序;边缘截断)。
        let [x0, y0, w, h] = cell;
        let hw = w.div_ceil(2);
        let hh = h.div_ceil(2);
        let mut max_level = level;
        for (ox, oy) in [(0u32, 0u32), (hw, 0), (0, hh), (hw, hh)] {
            let sx = x0 + ox;
            let sy = y0 + oy;
            if sx >= gb.width || sy >= gb.height || (ox > 0 && ox >= w) || (oy > 0 && oy >= h) {
                continue;
            }
            let sw = (w - ox).min(hw).min(gb.width - sx);
            let sh = (h - oy).min(hh).min(gb.height - sy);
            let l = subdivide(
                gb,
                adaptive,
                [sx, sy, sw, sh],
                level + 1,
                probes,
                pixel_probe,
                cause_counts,
            );
            max_level = max_level.max(l);
        }
        max_level
    }
    for by in 0..base_h {
        for bx in 0..base_w {
            let x0 = bx * M99_BASE_CELL;
            let y0 = by * M99_BASE_CELL;
            let w = M99_BASE_CELL.min(gb.width - x0);
            let h = M99_BASE_CELL.min(gb.height - y0);
            let l = subdivide(
                gb,
                adaptive,
                [x0, y0, w, h],
                0,
                &mut probes,
                &mut pixel_probe,
                &mut cause_counts,
            );
            level_map[(by * base_w + bx) as usize] = l;
        }
    }
    SpgGrid {
        width: gb.width,
        height: gb.height,
        probes,
        pixel_probe,
        level_map,
        cause_counts,
    }
}

// ---------------------------------------------------------------------------
// probe 追踪 RNG 流(PCG32 单一流按索引寻址;流为输入非结果)
// ---------------------------------------------------------------------------

/// M99 流布局(冻结):逐 probe 逐采样 4 维 [bsdf_r1, bsdf_r2, nee_u, nee_v]。
pub mod m99_rng {
    use crate::rt::ref_tracer::Pcg32;

    /// 每采样随机维数。
    pub const DIMS_PER_SAMPLE: usize = 4;

    /// 流总长(= probe_count · spp · 4)。
    pub fn stream_len(probe_count: usize, spp: u32) -> usize {
        probe_count * spp as usize * DIMS_PER_SAMPLE
    }

    /// 采样 (probe, sample) 的流起始下标。
    pub fn sample_base(probe: usize, sample: usize, spp: u32) -> usize {
        (probe * spp as usize + sample) * DIMS_PER_SAMPLE
    }

    /// 生成整条流(单 [`Pcg32`] 实例,probe 序 × 采样序顺序产出)。
    pub fn generate_stream(probe_count: usize, spp: u32, seed: u64) -> Vec<f32> {
        let mut rng = Pcg32::new(seed);
        let mut out = Vec::with_capacity(stream_len(probe_count, spp));
        for _ in 0..stream_len(probe_count, spp) {
            out.push(rng.next_f32());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// probe 第一反弹追踪(host 参照,与 kernel `g9_m99_spg_probe` 逐字同源;
// product IS = BRDF×入射光 product importance sampling,RXS-0360 L2)
// ---------------------------------------------------------------------------

/// 三角形几何法线(依 winding,归一化;与 M98 host 同式)。
fn tri_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let nx = e1[1] * e2[2] - e1[2] * e2[1];
    let ny = e1[2] * e2[0] - e1[0] * e2[2];
    let nz = e1[0] * e2[1] - e1[1] * e2[0];
    let l = (nx * nx + ny * ny + nz * nz).sqrt();
    if l > 0.0 {
        [nx / l, ny / l, nz / l]
    } else {
        [0.0; 3]
    }
}

/// 阴影可见性(暴力全扫,确定性测试计数;命中 ⇒ 0.0 否则 1.0)。
fn shadow_vis(scene: &PtScene, o: [f32; 3], d: [f32; 3], t_max: f32) -> f32 {
    let (hit, _tests) = l2_closest_hit(scene, o, d, t_max);
    if hit.is_some() { 0.0 } else { 1.0 }
}

/// 三角形发光辐射度(非发光 ⇒ 零)。
fn tri_emission(scene: &PtScene, tri: u32) -> [f32; 3] {
    match &scene.materials[tri as usize] {
        path_trace::MaterialKind::Emission { emission, .. } => *emission,
        _ => [0.0; 3],
    }
}

/// 均匀半球方向(product IS 关闭替代的采样律;切线框架与
/// [`cosine_dir`] 同一构造,局部方向 z = r2〔cosθ 均匀〕)。
#[allow(clippy::manual_clamp)] // 算术门即公式面(.min/.max 序与 kernel 逐字同源;clamp 的 NaN 传播语义不同,禁改写)
pub fn uniform_dir(n: [f32; 3], r1: f32, r2: f32) -> [f32; 3] {
    let pi = path_trace::PT_PI;
    let phi = 2.0 * pi * r1;
    let rr = (1.0 - r2 * r2).max(0.0).sqrt();
    let lx = rr * phi.cos();
    let ly = rr * phi.sin();
    let lz = r2;
    let up_sel = ((0.999 - n[1].abs()) * BIG).min(1.0).max(0.0);
    let upx = 1.0 - up_sel;
    let upy = up_sel;
    let t1x = upy * n[2];
    let t1y = -upx * n[2];
    let t1z = upx * n[1] - upy * n[0];
    let t1l = 1.0 / (t1x * t1x + t1y * t1y + t1z * t1z).sqrt();
    let tx = t1x * t1l;
    let ty = t1y * t1l;
    let tz = t1z * t1l;
    let bx = n[1] * tz - n[2] * ty;
    let by = n[2] * tx - n[0] * tz;
    let bz = n[0] * ty - n[1] * tx;
    let ndx = tx * lx + bx * ly + n[0] * lz;
    let ndy = ty * lx + by * ly + n[1] * lz;
    let ndz = tz * lx + bz * ly + n[2] * lz;
    let inv = 1.0 / (ndx * ndx + ndy * ndy + ndz * ndz).sqrt();
    [ndx * inv, ndy * inv, ndz * inv]
}

/// 单 probe 单采样第一反弹估计(product IS 开 = NEE×MIS + BSDF 命中发光面
/// ×MIS——与 M96 megakernel shade①②③ 逐字同式的单反弹特例;关 = 均匀半球
/// 采样仅 BSDF 命中发光面贡献,pdf = 1/(2π) ⇒ throughput = 2·albedo·cos)。
fn trace_probe_sample(
    scene: &PtScene,
    probe: &SpgProbe,
    stream: &[f32],
    probe_idx: usize,
    sample: usize,
    spp: u32,
    product_is: bool,
) -> [f32; 3] {
    let sb = m99_rng::sample_base(probe_idx, sample, spp);
    let r1 = stream[sb];
    let r2 = stream[sb + 1];
    let u = stream[sb + 2];
    let v = stream[sb + 3];
    let p = probe.pos;
    let n = probe.normal;
    let al = probe.albedo;
    let l = &scene.light;
    let ln = l.normal();
    let area = l.area();
    let le = l.emission;
    let so = [
        p[0] + n[0] * RAY_EPS,
        p[1] + n[1] * RAY_EPS,
        p[2] + n[2] * RAY_EPS,
    ];
    let mut li = [0.0f32; 3];
    if product_is {
        // ── NEE(M96 shade② 逐字同式)──
        let q = fallback_chain::light_sample(scene, u, v);
        let wv = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
        let dist2 = (wv[0] * wv[0] + wv[1] * wv[1] + wv[2] * wv[2]).max(TINY * TINY);
        let dist = dist2.sqrt();
        let wi = [wv[0] / dist, wv[1] / dist, wv[2] / dist];
        let cos_s = (n[0] * wi[0] + n[1] * wi[1] + n[2] * wi[2]).max(0.0);
        let cos_l = (-(ln[0] * wi[0] + ln[1] * wi[1] + ln[2] * wi[2])).max(0.0);
        if cos_s > 0.0 && cos_l > 0.0 {
            let nee_core = cos_s * cos_l * area / (path_trace::PT_PI * dist2);
            let w_l = path_trace::mis_weight_light(cos_s / path_trace::PT_PI, area, cos_l, dist2);
            let t_sh = (dist - 2.0 * RAY_EPS).max(RAY_EPS);
            let vis = shadow_vis(scene, so, wi, t_sh);
            li[0] += al[0] * le[0] * nee_core * w_l * vis;
            li[1] += al[1] * le[1] * nee_core * w_l * vis;
            li[2] += al[2] * le[2] * nee_core * w_l * vis;
        }
        // ── BSDF 余弦采样命中发光面(M96 shade①③ 逐字同式)──
        let nd = cosine_dir(n, r1, r2);
        let (hit, _t) = l2_closest_hit(scene, so, nd, scene.t_max);
        if let Some((t, tri)) = hit {
            let em = tri_emission(scene, tri);
            let (a, b, c) = scene_tri(scene, tri as usize);
            let ng = tri_normal(a, b, c);
            let cos_emit = -(ng[0] * nd[0] + ng[1] * nd[1] + ng[2] * nd[2]);
            if em.iter().any(|&x| x > 0.0) && cos_emit > 0.0 {
                let cos_nd = (n[0] * nd[0] + n[1] * nd[1] + n[2] * nd[2]).max(0.0);
                let pdf_b = path_trace::cosine_hemisphere_pdf(cos_nd);
                let w_b = path_trace::mis_weight_bsdf(t, area, cos_emit, pdf_b);
                li[0] += al[0] * em[0] * w_b;
                li[1] += al[1] * em[1] * w_b;
                li[2] += al[2] * em[2] * w_b;
            }
        }
    } else {
        // ── 均匀半球采样(product IS 关闭注入 variant;RED 臂承载)──
        let nd = uniform_dir(n, r1, r2);
        let (hit, _t) = l2_closest_hit(scene, so, nd, scene.t_max);
        if let Some((_t, tri)) = hit {
            let em = tri_emission(scene, tri);
            let (a, b, c) = scene_tri(scene, tri as usize);
            let ng = tri_normal(a, b, c);
            let cos_emit = -(ng[0] * nd[0] + ng[1] * nd[1] + ng[2] * nd[2]);
            if em.iter().any(|&x| x > 0.0) && cos_emit > 0.0 {
                let cos_nd = (n[0] * nd[0] + n[1] * nd[1] + n[2] * nd[2]).max(0.0);
                // (albedo/π)·Le·cos / (1/2π) = 2·albedo·Le·cos_nd。
                li[0] += al[0] * em[0] * 2.0 * cos_nd;
                li[1] += al[1] * em[1] * 2.0 * cos_nd;
                li[2] += al[2] * em[2] * 2.0 * cos_nd;
            }
        }
    }
    li
}

/// probe 追踪输出(逐 probe 均值 RGB + Σlum/Σlum² 方差面;M96 同律)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProbeTraceOut {
    /// 均值辐射度 RGB(线性)。
    pub rgb: [f32; 3],
    /// 亮度累加 Σlum。
    pub sum_lum: f32,
    /// 亮度平方累加 Σlum²(方差 = Σlum²/spp − (Σlum/spp)²)。
    pub sumsq_lum: f32,
}

impl ProbeTraceOut {
    /// 逐 probe 亮度方差(E[x²]−E[x]²;spp 由调用方给)。
    pub fn variance(&self, spp: u32) -> f64 {
        let inv = 1.0 / f64::from(spp);
        let e = f64::from(self.sum_lum) * inv;
        let e2 = f64::from(self.sumsq_lum) * inv;
        (e2 - e * e).max(0.0)
    }
}

/// probe 批量追踪(host 参照;逐 probe 顺序累加,确定性)。`product_is=false`
/// = 关 product IS 注入 variant(方差回归 RED 臂)。
pub fn trace_probes_host(
    scene: &PtScene,
    grid: &SpgGrid,
    stream: &[f32],
    spp: u32,
    product_is: bool,
) -> Result<Vec<ProbeTraceOut>, SpgError> {
    let valid_count = grid.probes.iter().filter(|p| p.valid).count();
    if stream.len() != m99_rng::stream_len(valid_count, spp) {
        return Err(SpgError::InvalidConfig(format!(
            "RNG 流长 {} ≠ 期望 {}(valid_probes={valid_count} spp={spp})",
            stream.len(),
            m99_rng::stream_len(valid_count, spp)
        )));
    }
    let mut out = Vec::with_capacity(grid.probes.len());
    let mut vi = 0usize;
    for probe in &grid.probes {
        if !probe.valid {
            out.push(ProbeTraceOut {
                rgb: [0.0; 3],
                sum_lum: 0.0,
                sumsq_lum: 0.0,
            });
            continue;
        }
        let mut acc = [0.0f32; 3];
        let mut sum_lum = 0.0f32;
        let mut sumsq_lum = 0.0f32;
        for s in 0..spp as usize {
            let li = trace_probe_sample(scene, probe, stream, vi, s, spp, product_is);
            acc[0] += li[0];
            acc[1] += li[1];
            acc[2] += li[2];
            let lum = (li[0] + li[1] + li[2]) / 3.0;
            sum_lum += lum;
            sumsq_lum += lum * lum;
        }
        vi += 1;
        let inv = 1.0 / spp as f32;
        out.push(ProbeTraceOut {
            rgb: [acc[0] * inv, acc[1] * inv, acc[2] * inv],
            sum_lum,
            sumsq_lum,
        });
    }
    Ok(out)
}

/// 有效 probe 数(流长/打包共用)。
pub fn valid_probe_count(grid: &SpgGrid) -> usize {
    grid.probes.iter().filter(|p| p.valid).count()
}

// ---------------------------------------------------------------------------
// 3×3 probe 空间滤波(最细级 4 px tile radiance 图;权重律与 G8 底座
// gi::filter 同一公式面——深度 1/(1+t²)、法线 max(dot,0)^8、边界截断归一;
// 负载 = radiance 图替 SH,增量不重定底座)
// ---------------------------------------------------------------------------

/// 滤波 tile 图边长(最细级 cell = BASE_CELL >> MAX_SUBDIV = 4 px)。
pub const M99_FILTER_CELL: u32 = M99_BASE_CELL >> M99_MAX_SUBDIV;

/// 滤波参数(与 G8 底座 [`crate::gi::filter::FilterParams::default`] 同值)。
pub const M99_FILTER_DEPTH_REL_TOL: f32 = 0.1;
/// 法线相似性指数(同 G8 底座)。
pub const M99_FILTER_NORMAL_EXP: i32 = 8;

/// tile 图束(probe 追踪结果 → 最细级 tile 图的打包形;滤波/历史验证输入)。
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeTileMaps {
    /// tile radiance(逐 tile 取其 cover probe 的均值 radiance)。
    pub rad: Vec<[f32; 3]>,
    /// tile 深度(cover probe 锚点视 z)。
    pub dep: Vec<f32>,
    /// tile 法线(cover probe 锚点)。
    pub nrm: Vec<[f32; 3]>,
    /// tile 有效旗标。
    pub valid: Vec<bool>,
}

/// probe 追踪结果 → 最细级 tile radiance 图(逐 tile 取其 cover probe 的
/// 均值 radiance)+ tile 深度/法线图(中心像素采样,滤波/历史验证输入)。
pub fn probe_tile_maps(gb: &GBuffer, grid: &SpgGrid, traced: &[ProbeTraceOut]) -> ProbeTileMaps {
    let tw = gb.width.div_ceil(M99_FILTER_CELL);
    let th = gb.height.div_ceil(M99_FILTER_CELL);
    let mut rad = vec![[0.0f32; 3]; (tw * th) as usize];
    let mut dep = vec![0.0f32; (tw * th) as usize];
    let mut nrm = vec![[0.0f32; 3]; (tw * th) as usize];
    let mut valid = vec![false; (tw * th) as usize];
    for ty in 0..th {
        for tx in 0..tw {
            let t = (ty * tw + tx) as usize;
            // tile 中心像素 → probe 查找(像素查找图同一产线)。
            let cx = (tx * M99_FILTER_CELL + M99_FILTER_CELL / 2).min(gb.width - 1);
            let cy = (ty * M99_FILTER_CELL + M99_FILTER_CELL / 2).min(gb.height - 1);
            let pi = grid.pixel_probe[(cy * gb.width + cx) as usize];
            if pi == u32::MAX {
                continue;
            }
            let probe = grid.probes[pi as usize];
            if !probe.valid {
                continue;
            }
            rad[t] = traced[pi as usize].rgb;
            // 深度/法线 = probe 锚点(与 probe radiance 同一锚,滤波判据一致)。
            let ai = (probe.anchor[1] * gb.width + probe.anchor[0]) as usize;
            dep[t] = gb.depth[ai];
            nrm[t] = [gb.nrm[ai * 3], gb.nrm[ai * 3 + 1], gb.nrm[ai * 3 + 2]];
            valid[t] = true;
        }
    }
    ProbeTileMaps {
        rad,
        dep,
        nrm,
        valid,
    }
}

/// tile radiance 图 3×3 深度/法线相似性滤波(权重律与 G8 底座同式;
/// 无效 tile 权重恒 0,无效中心输出保持零——同 G8 语义)。
pub fn filter_radiance_3x3(
    tw: u32,
    th: u32,
    rad: &[[f32; 3]],
    dep: &[f32],
    nrm: &[[f32; 3]],
    valid: &[bool],
) -> Vec<[f32; 3]> {
    let mut out = vec![[0.0f32; 3]; (tw * th) as usize];
    for j in 0..th {
        for i in 0..tw {
            let q = (j * tw + i) as usize;
            if !valid[q] {
                continue;
            }
            let mut acc = [0.0f32; 3];
            let mut wsum = 0.0f32;
            for dj in -1i32..=1 {
                for di in -1i32..=1 {
                    let (ri, rj) = (i as i32 + di, j as i32 + dj);
                    if ri < 0 || rj < 0 || ri >= tw as i32 || rj >= th as i32 {
                        continue; // 边界截断归一(同 G8 底座)
                    }
                    let r = (rj as u32 * tw + ri as u32) as usize;
                    if !valid[r] {
                        continue;
                    }
                    let t = (dep[q] - dep[r]).abs()
                        / (M99_FILTER_DEPTH_REL_TOL * dep[q].max(dep[r]).max(0.05));
                    let wd = 1.0 / (1.0 + t * t);
                    let dot =
                        (nrm[q][0] * nrm[r][0] + nrm[q][1] * nrm[r][1] + nrm[q][2] * nrm[r][2])
                            .max(0.0);
                    let wn = dot.powi(M99_FILTER_NORMAL_EXP);
                    let w = wd * wn;
                    acc[0] += rad[r][0] * w;
                    acc[1] += rad[r][1] * w;
                    acc[2] += rad[r][2] * w;
                    wsum += w;
                }
            }
            if wsum > 1e-8 {
                out[q] = [acc[0] / wsum, acc[1] / wsum, acc[2] / wsum];
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Radiance Cache 双级(RXS-0360 L2/L3):屏幕 tile 级(temporal 公共底座
// 历史复用)+ 世界级 clipmap(not-triggered 登记)
// ---------------------------------------------------------------------------

/// 双级计数面(逐帧导出;世界级恒零显式——not-triggered 不充绿)。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RcCounters {
    /// 屏幕级历史命中 tile 数(validity=1)。
    pub screen_hits: u64,
    /// 屏幕级历史失效 tile 数(validity=0 ⇒ 重算)。
    pub screen_misses: u64,
    /// 屏幕级插入数(失效重算后写回历史)。
    pub screen_inserts: u64,
    /// 世界级查询数(恒 0;世界级未举证,查询即 typed Err)。
    pub world_lookups: u64,
}

/// 历史路径(闭集):temporal 公共底座 / 私写重投影注入 variant(RED 臂)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryPath {
    /// temporal 公共底座(`reproject_sample` + `validate_history_with_mv`;
    /// D2-Q14 正例)。
    TemporalBase,
    /// 私写重投影注入(绕过历史验证直接拷贝;D2-Q14 私写 variant 即 RED)。
    PrivateReprojectInjected,
}

/// 一帧缓存产物(混合后 tile radiance 图 + 计数 + 路径记录)。
#[derive(Debug, Clone)]
pub struct CacheFrame {
    /// 混合后 tile radiance(出图/装配消费)。
    pub map: Vec<[f32; 3]>,
    /// 本帧计数面。
    pub counters: RcCounters,
    /// 历史路径(审计输入)。
    pub path: HistoryPath,
    /// 历史是否经公共底座验证(私写 variant = false;审计判据)。
    pub history_validated: bool,
    /// 历史有效性 mask(1ch 0/1;temporal 底座产物;无历史/私写 = None)。
    pub validity: Option<ImageF32>,
}

/// tile 图打包为 ImageF32(3ch;temporal 底座接口形)。
pub fn tiles_to_image(tw: u32, th: u32, tiles: &[[f32; 3]]) -> ImageF32 {
    ImageF32::from_fn(tw, th, 3, |x, y, c| {
        tiles[(y * tw + x) as usize][c as usize]
    })
}

/// 1ch/3ch 辅助打包(深度/法线 tile 图 → ImageF32;历史验证输入)。
pub fn tiles_depth_image(tw: u32, th: u32, dep: &[f32]) -> ImageF32 {
    ImageF32::from_fn(tw, th, 1, |x, y, _| dep[(y * tw + x) as usize])
}

/// 法线 tile 图打包。
pub fn tiles_nrm_image(tw: u32, th: u32, nrm: &[[f32; 3]]) -> ImageF32 {
    ImageF32::from_fn(tw, th, 3, |x, y, c| nrm[(y * tw + x) as usize][c as usize])
}

/// 屏幕级缓存帧(历史复用经 temporal 公共底座;`prev` = 上一帧混合后
/// radiance/深度/法线 tile 图 + MV 场〔2ch,tile 分辨率;静态相机 = 零场,
/// 由 `temporal::common::compute_camera_mv` 同语义无动画派生〕)。
///
/// 语义:无历史 ⇒ 全 miss + 全插入(直出当前帧);有历史且
/// [`HistoryPath::TemporalBase`] ⇒ 公共底座重投影 + 深度/法线双判据验证,
/// valid ⇒ 指数混合 `prev·(1−α)+cur·α`(hit),invalid ⇒ 当前帧直出
/// (miss + insert);[`HistoryPath::PrivateReprojectInjected`] ⇒ 绕过验证
/// 直接拷贝重投影历史(私写重投影注入,审计必 fail-closed)。
#[allow(clippy::too_many_arguments)]
pub fn screen_cache_frame(
    tw: u32,
    th: u32,
    cur: &[[f32; 3]],
    cur_dep: &ImageF32,
    cur_nrm: &ImageF32,
    prev: Option<(&[[f32; 3]], &ImageF32, &ImageF32)>,
    mv: &ImageF32,
    path: HistoryPath,
) -> Result<CacheFrame, SpgError> {
    let n = (tw * th) as usize;
    if cur.len() != n || mv.c != 2 || mv.w != tw || mv.h != th {
        return Err(SpgError::InvalidConfig(format!(
            "缓存帧形状不符(tw={tw} th={th} cur={} mv={}x{}x{})",
            cur.len(),
            mv.w,
            mv.h,
            mv.c
        )));
    }
    let Some((prev_rad, prev_dep, prev_nrm)) = prev else {
        return Ok(CacheFrame {
            map: cur.to_vec(),
            counters: RcCounters {
                screen_hits: 0,
                screen_misses: n as u64,
                screen_inserts: n as u64,
                world_lookups: 0,
            },
            path,
            history_validated: false,
            validity: None,
        });
    };
    if prev_rad.len() != n {
        return Err(SpgError::InvalidConfig("历史 tile 数不符".into()));
    }
    let prev_img = tiles_to_image(tw, th, prev_rad);
    let mut counters = RcCounters::default();
    let mut out = vec![[0.0f32; 3]; n];
    match path {
        HistoryPath::TemporalBase => {
            // ── temporal 公共底座(禁私写重投影,D2-Q14)──
            let (prev_reproj, inside) = common::reproject_sample(&prev_img, mv);
            let validity = common::validate_history_with_mv(
                cur_dep,
                prev_dep,
                cur_nrm,
                prev_nrm,
                mv,
                M99_FILTER_DEPTH_REL_TOL,
                crate::gi::temporal::TemporalParams::default().normal_dot_min,
            );
            for t in 0..n {
                let (x, y) = (t as u32 % tw, t as u32 / tw);
                let v = validity.get(x, y, 0) * inside.get(x, y, 0);
                if v >= 0.5 {
                    counters.screen_hits += 1;
                    let a = M99_HISTORY_ALPHA;
                    for ((c, o), &cv) in out[t].iter_mut().enumerate().zip(cur[t].iter()) {
                        *o = prev_reproj.get(x, y, c as u32) * (1.0 - a) + cv * a;
                    }
                } else {
                    counters.screen_misses += 1;
                    counters.screen_inserts += 1;
                    out[t] = cur[t];
                }
            }
            Ok(CacheFrame {
                map: out,
                counters,
                path,
                history_validated: true,
                validity: Some(validity),
            })
        }
        HistoryPath::PrivateReprojectInjected => {
            // ── 私写重投影注入(绕过验证;RED 臂承载——审计必拒)──
            let (prev_reproj, inside) = common::reproject_sample(&prev_img, mv);
            for t in 0..n {
                let (x, y) = (t as u32 % tw, t as u32 / tw);
                if inside.get(x, y, 0) >= 0.5 {
                    counters.screen_hits += 1;
                    for (c, o) in out[t].iter_mut().enumerate() {
                        *o = prev_reproj.get(x, y, c as u32);
                    }
                } else {
                    counters.screen_misses += 1;
                    counters.screen_inserts += 1;
                    out[t] = cur[t];
                }
            }
            Ok(CacheFrame {
                map: out,
                counters,
                path,
                history_validated: false,
                validity: None,
            })
        }
    }
}

/// 历史路径审计(fail-closed):任何「有历史且未经公共底座验证」的帧 =
/// 私写重投影(D2-Q14;私写 variant 即 RED,负例臂承载)。
pub fn audit_history_paths(frames: &[CacheFrame], had_history: &[bool]) -> Result<(), SpgError> {
    if frames.len() != had_history.len() {
        return Err(SpgError::InvalidConfig("审计输入长度不符".into()));
    }
    for (i, (f, &had)) in frames.iter().zip(had_history.iter()).enumerate() {
        if had && (!f.history_validated || f.path != HistoryPath::TemporalBase) {
            return Err(SpgError::PrivateReprojection(format!(
                "帧 {i} 有历史但 history_validated={} path={:?}(未经 temporal 公共底座)",
                f.history_validated, f.path
            )));
        }
    }
    Ok(())
}

/// 世界级 clipmap 触发核验结果(显式结构;RXS-0360 L3)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldClipmapTrigger {
    /// 世界级须 RD-040 measured 触发举证,未举证只做屏幕级 ⇒ 登记
    /// not-triggered(条件未触发只表示决策已记录,不是成功)。
    NotTriggered {
        /// 未触发原因(evidence 字面)。
        reason: &'static str,
    },
}

/// 世界级 clipmap 触发核验器(fail-closed):RD-040 measured 触发举证未满足
/// (G9_CANDIDATE_DECISIONS v1.3 校准注)⇒ 恒
/// [`WorldClipmapTrigger::NotTriggered`];任何世界级查询经
/// [`world_clipmap_lookup`] typed Err 显式拒绝。
pub fn check_world_clipmap_trigger() -> WorldClipmapTrigger {
    WorldClipmapTrigger::NotTriggered {
        reason: "世界级 clipmap 证据不足(RD-040 measured 触发举证未满足):未举证只做屏幕级,登记 not-triggered 不充绿",
    }
}

/// 世界级 clipmap 查询接口(冻结面;未举证 ⇒ 恒 fail-closed
/// [`SpgError::WorldClipmapNotTriggered`],禁静默当绿)。
pub fn world_clipmap_lookup() -> Result<[f32; 3], SpgError> {
    Err(SpgError::WorldClipmapNotTriggered(
        "世界级 clipmap 查询被请求,但 RD-040 measured 触发举证未满足——登记 not-triggered,拒绝静默服务".into(),
    ))
}

// ---------------------------------------------------------------------------
// 装配(直接光 + albedo × 缓存 radiance;产物 digest 结构域含 level_map)
// ---------------------------------------------------------------------------

/// M99 一帧产物(合成辐射度 + 细分结构域 + 缓存计数)。
#[derive(Debug, Clone, PartialEq)]
pub struct SpgRcFrame {
    /// 图宽。
    pub width: u32,
    /// 图高。
    pub height: u32,
    /// 合成辐射度 RGB(3 f32/px;主未命中 = 直接光 0 + 天空零项——沿
    /// GBuffer 预传递口径,主未命中像素 rgb=0)。
    pub rgb: Vec<f32>,
    /// 逐基线 cell 细分级(结构域;关自适应 ⇒ 全 0 ⇒ digest 必分叉)。
    pub level_map: Vec<u32>,
    /// probe 总数/有效数(计数面)。
    pub probe_count: u32,
    /// 有效 probe 数。
    pub valid_probe_count: u32,
    /// 缓存计数(末帧)。
    pub counters: RcCounters,
}

impl SpgRcFrame {
    /// 产物 digest = sha256(rgb 字节 ‖ level_map 字节)(level_map 携带细分
    /// 结构——关自适应/判据漂移必然改变产物 digest,结构性保证回归可检测)。
    pub fn product_digest(&self) -> [u8; 32] {
        let mut pre = Vec::with_capacity(self.rgb.len() * 4 + self.level_map.len() * 4);
        for v in &self.rgb {
            pre.extend_from_slice(&v.to_le_bytes());
        }
        for v in &self.level_map {
            pre.extend_from_slice(&v.to_le_bytes());
        }
        rurix_pkg::sha256::digest(&pre)
    }
}

/// 装配一帧(纯函数;逐像素 rgb = direct + albedo × tile radiance〔tile =
/// 4 px 最细级〕;主未命中像素直出 0)。
pub fn assemble(
    gb: &GBuffer,
    tile_rad: &[[f32; 3]],
    counters: RcCounters,
    grid: &SpgGrid,
) -> Result<SpgRcFrame, SpgError> {
    let tw = gb.width.div_ceil(M99_FILTER_CELL);
    let th = gb.height.div_ceil(M99_FILTER_CELL);
    if tile_rad.len() != (tw * th) as usize {
        return Err(SpgError::InvalidConfig(format!(
            "tile radiance 数 {} ≠ {}x{}",
            tile_rad.len(),
            tw,
            th
        )));
    }
    let pixel_count = (gb.width * gb.height) as usize;
    let mut rgb = vec![0.0f32; pixel_count * 3];
    for py in 0..gb.height {
        for px in 0..gb.width {
            let i = (py * gb.width + px) as usize;
            if !gb.primary_hit[i] {
                continue;
            }
            let t = ((py / M99_FILTER_CELL) * tw + (px / M99_FILTER_CELL)) as usize;
            let a = [gb.alb[i * 3], gb.alb[i * 3 + 1], gb.alb[i * 3 + 2]];
            rgb[i * 3] = gb.direct[i * 3] + a[0] * tile_rad[t][0];
            rgb[i * 3 + 1] = gb.direct[i * 3 + 1] + a[1] * tile_rad[t][1];
            rgb[i * 3 + 2] = gb.direct[i * 3 + 2] + a[2] * tile_rad[t][2];
        }
    }
    Ok(SpgRcFrame {
        width: gb.width,
        height: gb.height,
        rgb,
        level_map: grid.level_map.clone(),
        probe_count: grid.probes.len() as u32,
        valid_probe_count: valid_probe_count(grid) as u32,
        counters,
    })
}

// ---------------------------------------------------------------------------
// 按匹配深度对 M96 golden 的容差带(measured 后冻结,P-09;fail-closed)
// ---------------------------------------------------------------------------

/// M99 深度带单条目(一档 = spg_adaptive golden / spg_uniform 关自适应 /
/// product_is_off 关引导;匹配深度 [`M99_MATCHED_DEPTH`])。
#[derive(Debug, Clone, PartialEq)]
pub struct M99BandEntry {
    /// 档位名(spg_adaptive / spg_uniform / product_is_off)。
    pub tier: String,
    /// 冻结 golden:该档产物 digest(sha256(rgb‖level_map))。
    pub product_digest: String,
    /// 冻结 golden:M96 同深度参照产物 digest。
    pub m96_digest: String,
    /// 冻结容差带(rel_dev 上界 = measured × [`M99_BAND_MARGIN`];禁手写)。
    pub band_rel_dev: f64,
    /// 冻结时实测 rel_dev(该档合成图 vs M96 同深度;provenance)。
    pub measured_rel_dev: f64,
}

/// M99 容差带(`milestones/g9/g9_m99_spg_rc_band.json` 的内存形)。
#[derive(Debug, Clone, PartialEq)]
pub struct M99SpgRcBand {
    /// provenance:冻结时刻 UTC。
    pub frozen_at_utc: String,
    /// provenance:device 名。
    pub device_name: String,
    /// 冻结场景名(M96 冻结 fixture)。
    pub scene: String,
    /// M96 门序消费锚:本带 m96_digest 与 M97 冻结带 `m96_cornell` 同深度
    /// 条目逐字相等(D2-Q7 门序消费面的机器锚)。
    pub m96_anchor_digest: String,
    /// provenance:关 product IS 方差比实测(off/on;检出阈 =
    /// [`M99_PRODUCT_IS_VAR_RATIO_MIN`] = 实测留 margin 冻结)。
    pub product_is_variance_ratio: f64,
    /// provenance:关自适应收敛特征偏离比实测(触发 cell 误差比 off/on;
    /// 检出阈 = [`M99_ADAPTIVE_DEVIATION_RATIO_MIN`])。
    pub adaptive_deviation_ratio: f64,
    /// 逐档条目。
    pub entries: Vec<M99BandEntry>,
}

impl M99SpgRcBand {
    /// 查条目(fail-closed:缺条目 = Err)。
    pub fn entry(&self, tier: &str) -> Result<&M99BandEntry, SpgError> {
        self.entries
            .iter()
            .find(|e| e.tier == tier)
            .ok_or_else(|| SpgError::DepthBand(format!("容差带缺条目 tier={tier}")))
    }

    /// 比对(fail-closed):双 digest 全等 且 rel_dev ≤ 带;违例逐条列名。
    pub fn check(
        &self,
        tier: &str,
        product_digest: &str,
        m96_digest: &str,
        rel_dev: f64,
    ) -> Result<(), SpgError> {
        let e = self.entry(tier)?;
        if product_digest != e.product_digest {
            return Err(SpgError::DepthBand(format!(
                "tier={tier} product_digest {product_digest} ≠ golden {}",
                e.product_digest
            )));
        }
        if m96_digest != e.m96_digest {
            return Err(SpgError::DepthBand(format!(
                "tier={tier} m96_digest {m96_digest} ≠ golden {}",
                e.m96_digest
            )));
        }
        if rel_dev.is_nan() || rel_dev > e.band_rel_dev {
            return Err(SpgError::DepthBand(format!(
                "tier={tier} rel_dev {rel_dev:.6e} 越带(上界 {:.6e})",
                e.band_rel_dev
            )));
        }
        Ok(())
    }

    /// 序列化(手工 JSON;字段序冻结,浮点 `{:e}` 确定性格式)。
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n  \"schema\": \"rurix.g9m99.spg_rc_band.v1\",\n");
        s.push_str(&format!(
            "  \"frozen_at_utc\": \"{}\",\n",
            self.frozen_at_utc
        ));
        s.push_str(&format!("  \"device_name\": \"{}\",\n", self.device_name));
        s.push_str(&format!("  \"scene\": \"{}\",\n", self.scene));
        s.push_str(&format!(
            "  \"m96_anchor_digest\": \"{}\",\n",
            self.m96_anchor_digest
        ));
        s.push_str(&format!(
            "  \"freeze_rule\": \"band_rel_dev = measured_rel_dev * {:.1}(规则冻结于 gi::spg_rc::M99_BAND_MARGIN;基值 = 冻结批实测,禁手写 P-09)\",\n",
            M99_BAND_MARGIN
        ));
        s.push_str(&format!(
            "  \"matched_depth\": \"{}\",\n",
            M99_MATCHED_DEPTH
        ));
        s.push_str(&format!(
            "  \"m96_golden_spp\": \"{}\",\n",
            M99_M96_GOLDEN_SPP
        ));
        s.push_str(&format!("  \"seed_chain\": \"{}\",\n", M99_SEED));
        s.push_str(&format!(
            "  \"product_is_variance_ratio\": \"{:e}\",\n",
            self.product_is_variance_ratio
        ));
        s.push_str(&format!(
            "  \"adaptive_deviation_ratio\": \"{:e}\",\n",
            self.adaptive_deviation_ratio
        ));
        s.push_str("  \"entries\": [\n");
        for (i, e) in self.entries.iter().enumerate() {
            s.push_str(&format!(
                "    {{\"tier\": \"{}\", \"product_digest\": \"{}\", \"m96_digest\": \"{}\", \"band_rel_dev\": \"{:e}\", \"measured_rel_dev\": \"{:e}\"}}{}\n",
                e.tier,
                e.product_digest,
                e.m96_digest,
                e.band_rel_dev,
                e.measured_rel_dev,
                if i + 1 == self.entries.len() { "" } else { "," }
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }

    /// 解析(fail-closed:schema 不符/键缺失/数值非法/条目重复一律 Err)。
    pub fn parse(text: &str) -> Result<M99SpgRcBand, SpgError> {
        let err = |m: &str| SpgError::DepthBand(format!("容差带解析: {m}"));
        if !text.contains("\"schema\": \"rurix.g9m99.spg_rc_band.v1\"") {
            return Err(err("schema 失配"));
        }
        let get_str = |key: &str| -> Result<String, SpgError> {
            let needle = format!("\"{key}\": \"");
            let start = text
                .find(&needle)
                .ok_or_else(|| err(&format!("缺键 {key}")))?
                + needle.len();
            let end = text[start..]
                .find('"')
                .ok_or_else(|| err(&format!("键 {key} 值未闭合")))?
                + start;
            Ok(text[start..end].to_string())
        };
        let mut entries = Vec::new();
        let entries_sec = text
            .split("\"entries\": [")
            .nth(1)
            .ok_or_else(|| err("缺 entries 段"))?;
        for chunk in entries_sec.split('{').skip(1) {
            let body = chunk.split('}').next().ok_or_else(|| err("条目未闭合"))?;
            let field = |key: &str| -> Result<String, SpgError> {
                let needle = format!("\"{key}\": \"");
                let start = body
                    .find(&needle)
                    .ok_or_else(|| err(&format!("条目缺键 {key}")))?
                    + needle.len();
                let end = body[start..]
                    .find('"')
                    .ok_or_else(|| err("条目键 {key} 值未闭合"))?
                    + start;
                Ok(body[start..end].to_string())
            };
            let tier = field("tier")?;
            if entries.iter().any(|e: &M99BandEntry| e.tier == tier) {
                return Err(err("条目 tier 重复"));
            }
            let product_digest = field("product_digest")?;
            let m96_digest = field("m96_digest")?;
            for (nm, d) in [
                ("product_digest", &product_digest),
                ("m96_digest", &m96_digest),
            ] {
                if d.len() != 64 || !d.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(err(&format!("{nm} 非 64 位 hex")));
                }
            }
            let band_rel_dev: f64 = field("band_rel_dev")?
                .parse()
                .map_err(|_| err("band_rel_dev 非数值"))?;
            let measured_rel_dev: f64 = field("measured_rel_dev")?
                .parse()
                .map_err(|_| err("measured_rel_dev 非数值"))?;
            if band_rel_dev <= 0.0 || !band_rel_dev.is_finite() {
                return Err(err("band_rel_dev 非正有限数"));
            }
            entries.push(M99BandEntry {
                tier,
                product_digest,
                m96_digest,
                band_rel_dev,
                measured_rel_dev,
            });
        }
        if entries.is_empty() {
            return Err(err("entries 为空"));
        }
        Ok(M99SpgRcBand {
            frozen_at_utc: get_str("frozen_at_utc")?,
            device_name: get_str("device_name")?,
            scene: get_str("scene")?,
            m96_anchor_digest: get_str("m96_anchor_digest")?,
            product_is_variance_ratio: get_str("product_is_variance_ratio")?
                .parse()
                .map_err(|_| err("product_is_variance_ratio 非数值"))?,
            adaptive_deviation_ratio: get_str("adaptive_deviation_ratio")?
                .parse()
                .map_err(|_| err("adaptive_deviation_ratio 非数值"))?,
            entries,
        })
    }
}

// ---------------------------------------------------------------------------
// device 输入打包(kernel `g9_m99_spg_probe.rx` 头注参数面逐字同源)
// ---------------------------------------------------------------------------

/// probe 打包:10 f32/probe(pos 3, nrm 3, albedo 3, valid)——仅有效 probe
/// 入包(流/下标按有效序,host/kernel 同一序)。
pub fn pack_probes(grid: &SpgGrid) -> Vec<f32> {
    let mut out = Vec::new();
    for p in grid.probes.iter().filter(|p| p.valid) {
        out.extend_from_slice(&p.pos);
        out.extend_from_slice(&p.normal);
        out.extend_from_slice(&p.albedo);
        out.push(1.0);
    }
    out
}

/// kernel 参数打包(21 f32;与 `kernels/g9_m99_spg_probe.rx` 头注逐字同源)。
pub fn pack_probe_params(
    scene: &PtScene,
    probe_count: u32,
    spp: u32,
    product_is: bool,
) -> Vec<f32> {
    let l = &scene.light;
    let ln = l.normal();
    let mut p = Vec::with_capacity(21);
    p.push(probe_count as f32);
    p.push(spp as f32);
    p.push(if product_is { 1.0 } else { 0.0 });
    p.push(RAY_EPS);
    p.push(scene.t_max);
    p.extend_from_slice(&l.p00);
    p.extend_from_slice(&l.e1);
    p.extend_from_slice(&l.e2);
    p.push(l.area());
    p.extend_from_slice(&l.emission);
    p.extend_from_slice(&ln);
    debug_assert_eq!(p.len(), 21);
    p
}

// ---------------------------------------------------------------------------
// 单测(RXS-0360 锚定;细分判据闭集 / 滤波权重律 / product IS 方差 RED /
// temporal 底座审计 / 世界级 not-triggered / 容差带 fail-closed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gi::fallback_chain as fb;
    use crate::gi::surface_cache;

    fn cornell() -> PtScene {
        let s = path_trace::m96_cornell_scene();
        s.validate().expect("cornell 冻结 fixture 装载");
        s
    }

    fn gb_of(scene: &PtScene) -> GBuffer {
        fb::gbuffer_prepass(scene)
    }

    //@ spec: RXS-0360
    #[test]
    fn subdivide_cause_closed_set_and_names() {
        // 判据闭集三字面冻结(evidence 键)。
        assert_eq!(
            SubdivideCause::DepthDiscontinuity.name(),
            "depth_discontinuity"
        );
        assert_eq!(
            SubdivideCause::NormalDiscontinuity.name(),
            "normal_discontinuity"
        );
        assert_eq!(SubdivideCause::RadianceVariance.name(), "radiance_variance");
        // 纯函数判据三元:深度/法线/方差独立评估(一 cell 可同触发多判据)。
        let st = CellStats {
            count: 4,
            z_jump_max: 0.5,
            n_dot_min: 0.5,
            lum_var: 1.0,
        };
        assert_eq!(subdivide_triggers(&st), [true, true, true], "三判据同触发");
        let st = CellStats {
            z_jump_max: 0.01,
            ..st
        };
        assert_eq!(subdivide_triggers(&st), [false, true, true]);
        let st = CellStats {
            n_dot_min: 1.0,
            ..st
        };
        assert_eq!(subdivide_triggers(&st), [false, false, true]);
        // 全不触发 ⇒ 不细分;有效像素不足 ⇒ 不细分(显式)。
        let st = CellStats { lum_var: 0.0, ..st };
        assert_eq!(subdivide_triggers(&st), [false; 3]);
        assert_eq!(subdivide_triggers(&CellStats::default()), [false; 3]);
    }

    //@ spec: RXS-0360
    #[test]
    fn grid_baseline_16px_and_adaptive_increment() {
        let scene = cornell();
        let gb = gb_of(&scene);
        // 关自适应 ⇒ 基线 16 px/probe 均匀:64×64 ⇒ 4×4 = 16 probes,level 全 0。
        let uni = build_spg_grid(&gb, false);
        assert_eq!(uni.probes.len(), 16);
        assert!(uni.level_map.iter().all(|&l| l == 0));
        assert!(uni.cause_counts.iter().all(|&c| c == 0));
        assert!(
            uni.probes
                .iter()
                .all(|p| p.level == 0 && p.cell[2] == M99_BASE_CELL)
        );
        // 开自适应 ⇒ 增量细分:cornell 盒棱/阴影边界 cell 必触发(level>0),
        // probe 数 > 16,判据闭集计数非空;双跑位级一致。
        let ad = build_spg_grid(&gb, true);
        let ad2 = build_spg_grid(&gb, true);
        assert_eq!(ad, ad2, "细分同输入确定(双跑位级一致)");
        assert!(ad.probes.len() > uni.probes.len(), "自适应 probe 数 > 基线");
        assert!(ad.level_map.iter().any(|&l| l > 0), "存在细分 cell");
        assert!(ad.cause_counts.iter().sum::<u64>() > 0, "判据闭集计数非空");
        assert!(ad.probes.iter().all(|p| p.level <= M99_MAX_SUBDIV));
        // 像素查找图闭合:主命中像素必有有效 probe 覆盖。
        for (i, &hit) in gb.primary_hit.iter().enumerate() {
            if hit {
                let pi = ad.pixel_probe[i];
                assert!(
                    pi != u32::MAX && ad.probes[pi as usize].valid,
                    "像素 {i} 无有效 probe"
                );
            }
        }
    }

    //@ spec: RXS-0360
    #[test]
    fn probe_trace_product_is_variance_red_arm() {
        // product IS 开 ⇒ NEE×MIS 低方差;关 ⇒ 均匀半球高方差——方差回归
        // 必须可检测(负例 RED 臂独立有效,RXS-0360 L2)。
        let scene = cornell();
        let gb = gb_of(&scene);
        let grid = build_spg_grid(&gb, true);
        let n = valid_probe_count(&grid);
        let stream = m99_rng::generate_stream(n, M99_PROBE_SPP, M99_SEED);
        let on = trace_probes_host(&scene, &grid, &stream, M99_PROBE_SPP, true).expect("追踪");
        let off = trace_probes_host(&scene, &grid, &stream, M99_PROBE_SPP, false).expect("追踪");
        let var = |outs: &[ProbeTraceOut]| -> f64 {
            outs.iter()
                .zip(grid.probes.iter())
                .filter(|(_, p)| p.valid)
                .map(|(o, _)| o.variance(M99_PROBE_SPP))
                .sum::<f64>()
                / n as f64
        };
        let ratio = var(&off) / var(&on).max(1e-30);
        assert!(
            ratio >= M99_PRODUCT_IS_VAR_RATIO_MIN,
            "关 product IS 方差回归可检测:ratio={ratio:.3} ≥ {M99_PRODUCT_IS_VAR_RATIO_MIN}"
        );
        // sabotage 探针:开 vs 开 方差比 = 1 ⇒ 不得误检(能红证明的对偶)。
        let ratio_same = var(&on) / var(&on).max(1e-30);
        assert!(
            ratio_same < M99_PRODUCT_IS_VAR_RATIO_MIN,
            "on/on 比 = 1 不误检"
        );
        // 确定性:同流双跑位级一致。
        let on2 = trace_probes_host(&scene, &grid, &stream, M99_PROBE_SPP, true).expect("追踪");
        assert_eq!(on, on2, "同输入双跑位级一致");
    }

    //@ spec: RXS-0360
    #[test]
    fn filter_radiance_weight_law_matches_g8_base() {
        // 权重律锚定(与 G8 底座 gi::filter 同一公式面):均匀场滤波 = 恒等
        // (常数场不变);深度断裂处权重 = 1/(1+t²) 压止(不跨界扩散)。
        let (tw, th) = (4u32, 4u32);
        let rad = vec![[1.0f32, 0.5, 0.25]; 16];
        let dep = vec![0.5f32; 16];
        let nrm = vec![[0.0f32, 1.0, 0.0]; 16];
        let valid = vec![true; 16];
        let out = filter_radiance_3x3(tw, th, &rad, &dep, &nrm, &valid);
        for (t, o) in out.iter().enumerate() {
            assert_eq!(*o, [1.0, 0.5, 0.25], "tile {t} 常数场不变");
        }
        // 深度断裂:中心 (1,1) dep=0.5,右上 (2,1) dep=5.0 单点脉冲 ⇒
        // 脉冲对中心权重 = 1/(1+t²),t = 4.5/(0.1×5.0) = 9 ⇒ w = 1/82。
        let mut dep2 = dep.clone();
        dep2[6] = 5.0;
        let mut rad2 = vec![[0.0f32; 3]; 16];
        rad2[6] = [82.0, 0.0, 0.0];
        let out2 = filter_radiance_3x3(tw, th, &rad2, &dep2, &nrm, &valid);
        // 中心 (1,1) 邻域:8 个权重 1 + 脉冲权重 1/82 ⇒ 吸入 = (82/82)/(8+1/82)=1/8.0122。
        let got = out2[5][0];
        let expect = 1.0 / (8.0 + 1.0 / 82.0);
        assert!(
            (got - expect).abs() < 1e-4,
            "深度断裂权重锚定: {got} vs {expect}"
        );
        // 无效中心输出保持零(G8 同语义)。
        let mut valid3 = valid.clone();
        valid3[5] = false;
        let out3 = filter_radiance_3x3(tw, th, &rad, &dep, &nrm, &valid3);
        assert_eq!(out3[5], [0.0; 3]);
    }

    //@ spec: RXS-0360
    #[test]
    fn screen_cache_temporal_base_and_private_red() {
        // 屏幕级缓存:首帧全 miss+insert;次帧静态相机(MV 零场)⇒ 公共底座
        // 验证全 valid ⇒ 全 hit;私写重投影注入 ⇒ 审计 fail-closed(D2-Q14)。
        let (tw, th) = (4u32, 2u32);
        let cur = vec![[1.0f32, 1.0, 1.0]; 8];
        let dep0 = tiles_depth_image(tw, th, &vec![0.5f32; 8]);
        let nrm0 = tiles_nrm_image(tw, th, &vec![[0.0f32, 1.0, 0.0]; 8]);
        let mv = ImageF32::new(tw, th, 2); // 零 MV 场(静态相机)
        // 首帧:无历史 ⇒ 全 miss+insert,无验证语义(history_validated=false 显式)。
        let f0 = screen_cache_frame(
            tw,
            th,
            &cur,
            &dep0,
            &nrm0,
            None,
            &mv,
            HistoryPath::TemporalBase,
        )
        .expect("首帧");
        assert_eq!(f0.counters.screen_misses, 8);
        assert_eq!(f0.counters.screen_inserts, 8);
        assert_eq!(f0.counters.screen_hits, 0);
        // 次帧:历史 = 首帧(经底座验证)。
        let f1 = screen_cache_frame(
            tw,
            th,
            &cur,
            &dep0,
            &nrm0,
            Some((&f0.map, &dep0, &nrm0)),
            &mv,
            HistoryPath::TemporalBase,
        )
        .expect("次帧");
        assert_eq!(f1.counters.screen_hits, 8, "静态相机全 hit");
        assert!(f1.history_validated && f1.validity.is_some());
        // 混合律锚定:out = prev·(1−α)+cur·α = 1.0(等值场)。
        assert!(f1.map.iter().all(|p| (*p)[0] == 1.0));
        // 审计:正例过。
        audit_history_paths(&[f0.clone(), f1.clone()], &[false, true]).expect("正例审计过");
        // 私写注入:同输入但绕过验证 ⇒ 审计必 fail-closed(私写 variant 即 RED)。
        let f1_bad = screen_cache_frame(
            tw,
            th,
            &cur,
            &dep0,
            &nrm0,
            Some((&f0.map, &dep0, &nrm0)),
            &mv,
            HistoryPath::PrivateReprojectInjected,
        )
        .expect("注入帧产出");
        assert!(!f1_bad.history_validated);
        let audited = audit_history_paths(&[f0, f1_bad], &[false, true]);
        assert!(
            matches!(audited, Err(SpgError::PrivateReprojection(_))),
            "私写重投影注入必拒: {audited:?}"
        );
        // 形状非法 fail-closed。
        assert!(
            screen_cache_frame(
                tw,
                th,
                &cur[..4],
                &dep0,
                &nrm0,
                None,
                &mv,
                HistoryPath::TemporalBase
            )
            .is_err()
        );
    }

    //@ spec: RXS-0360
    #[test]
    fn world_clipmap_not_triggered_registration() {
        // 世界级 clipmap:未 measured 举证 ⇒ 登记 not-triggered(显式结构),
        // 查询即 typed Err(禁静默当绿,RXS-0360 L3)。
        let WorldClipmapTrigger::NotTriggered { reason } = check_world_clipmap_trigger();
        assert!(reason.contains("RD-040"), "登记原因含 RD-040 字面");
        let looked = world_clipmap_lookup();
        assert!(
            matches!(looked, Err(SpgError::WorldClipmapNotTriggered(_))),
            "世界级查询必 typed Err"
        );
    }

    //@ spec: RXS-0360
    #[test]
    fn assemble_and_digest_structural_divergence() {
        // 装配语义:rgb = direct + albedo × tile radiance;产物 digest 结构域
        // 含 level_map ⇒ 关自适应必然 digest 分叉(回归可检测的结构性保证)。
        let scene = cornell();
        let gb = gb_of(&scene);
        let grid = build_spg_grid(&gb, true);
        let tw = gb.width.div_ceil(M99_FILTER_CELL);
        let th = gb.height.div_ceil(M99_FILTER_CELL);
        let tiles = vec![[0.25f32, 0.5, 0.75]; (tw * th) as usize];
        let frame = assemble(&gb, &tiles, RcCounters::default(), &grid).expect("装配");
        for (i, &hit) in gb.primary_hit.iter().enumerate() {
            if hit {
                let t = ((i as u32 / gb.width / M99_FILTER_CELL) * tw
                    + (i as u32 % gb.width) / M99_FILTER_CELL) as usize;
                let expect = gb.direct[i * 3] + gb.alb[i * 3] * 0.25;
                assert!((frame.rgb[i * 3] - expect).abs() < 1e-6, "装配式锚 px={i}");
                let _ = t;
            } else {
                assert_eq!(frame.rgb[i * 3..i * 3 + 3], [0.0; 3]);
            }
        }
        // 关自适应 ⇒ level_map 全 0 ⇒ digest 分叉(结构域)。
        let uni = build_spg_grid(&gb, false);
        let frame_uni = assemble(&gb, &tiles, RcCounters::default(), &uni).expect("装配");
        assert_ne!(frame.product_digest(), frame_uni.product_digest());
        // 同输入双跑 digest 相等。
        let frame2 = assemble(&gb, &tiles, RcCounters::default(), &grid).expect("装配");
        assert_eq!(frame.product_digest(), frame2.product_digest());
        // 形状非法 fail-closed。
        assert!(assemble(&gb, &tiles[..4], RcCounters::default(), &grid).is_err());
    }

    //@ spec: RXS-0360
    #[test]
    fn band_roundtrip_and_fail_closed() {
        // 容差带:序列化/解析往返全等;篡改 digest/越带/缺条目必 fail-closed。
        let band = M99SpgRcBand {
            frozen_at_utc: "2026-08-13T00:00:00Z".into(),
            device_name: "testdev".into(),
            scene: "m96_cornell".into(),
            m96_anchor_digest: "a".repeat(64),
            product_is_variance_ratio: 3.25,
            adaptive_deviation_ratio: 1.125,
            entries: vec![M99BandEntry {
                tier: "spg_adaptive".into(),
                product_digest: "b".repeat(64),
                m96_digest: "a".repeat(64),
                band_rel_dev: 1.5,
                measured_rel_dev: 0.75,
            }],
        };
        let text = band.to_json();
        let parsed = M99SpgRcBand::parse(&text).expect("解析");
        assert_eq!(parsed, band, "序列化往返全等");
        // 正例:check 过。
        parsed
            .check("spg_adaptive", &"b".repeat(64), &"a".repeat(64), 0.75)
            .expect("带内");
        parsed
            .check("spg_adaptive", &"b".repeat(64), &"a".repeat(64), 1.5)
            .expect("带上界含");
        // 篡改 digest 必拒;越带必拒;缺条目必拒;NaN 必拒。
        assert!(
            parsed
                .check("spg_adaptive", &"c".repeat(64), &"a".repeat(64), 0.75)
                .is_err()
        );
        assert!(
            parsed
                .check("spg_adaptive", &"b".repeat(64), &"a".repeat(64), 1.51)
                .is_err()
        );
        assert!(
            parsed
                .check("nope", &"b".repeat(64), &"a".repeat(64), 0.1)
                .is_err()
        );
        assert!(
            parsed
                .check("spg_adaptive", &"b".repeat(64), &"a".repeat(64), f64::NAN)
                .is_err()
        );
        // 坏 schema/坏 hex 必拒。
        assert!(M99SpgRcBand::parse("{\"schema\": \"nope\"}").is_err());
        let bad = text.replace(&"b".repeat(64), &"z".repeat(64));
        assert!(M99SpgRcBand::parse(&bad).is_err());
    }

    //@ spec: RXS-0360
    #[test]
    fn m96_anchor_consumed_from_m97_band() {
        // 门序消费锚(D2-Q7):M97 冻结带 m96_cornell 深度 2 条目 digest 可读,
        // 与 M99 门序锚口径一致(本条 host 侧锚;harness 侧再对 M96 实跑 digest)。
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../milestones/g9/g9_m97_depth_band.json"
        );
        let text = std::fs::read_to_string(path).expect("M97 冻结带存在(G9.4 波内已就位)");
        let band = surface_cache::DepthBand::parse(&text).expect("M97 带解析");
        let e = band.entry(M99_MATCHED_DEPTH).expect("深度 2 条目");
        assert_eq!(e.m96_digest.len(), 64, "M96 digest 64 位 hex");
        assert_eq!(M99_MATCHED_DEPTH, 2);
    }
}
