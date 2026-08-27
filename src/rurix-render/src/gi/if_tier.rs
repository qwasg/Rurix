//! G9.4 M101 IF 体素网格 + 档位阶梯 host 面(spec/global_illumination.md
//! RXS-0362;RFC-0022 §4.8 D2-Q5/D2-Q10;门 `g9.p1.m101.if_tier_ladder`)。
//!
//! 本模块 = M101 的 **host 数据面/档位面/预算面**:
//! - **八面体编码单一源**([`oct`]):probe 方向 ↔ 八面体 UV 编解码(**线性
//!   域**——编码的是方向/辐射度采样,辐射度值不做任何 gamma 变换;
//!   [`EncodeDomain::SrgbInjected`] 为 RED 臂注入 variant,编码域错误漏检
//!   即判 FAIL);编解码往返误差界单测锚定([`OCT_ROUNDTRIP_BOUND_IRR`] /
//!   [`OCT_ROUNDTRIP_BOUND_VIS`],measured 冻结)。
//! - **IF 体素网格**([`IfVoxelGrid`]):最小 grid(4×4×4 覆盖 cornell 单位
//!   盒)+ 逐 probe 辐射度八面体图(irradiance 8×8 + **visibility 16×16——
//!   防漏光优先于提 irradiance 分辨率**,RXS-0362 L1 逐字)+ 每帧轮换更新
//!   摊销([`IfVoxelGrid::rotate_update`],确定性游标);DDGI Resampling 演进
//!   项非首版(不做)。
//! - **档位阶梯 L0~L3 闭集**([`IfTier`] / [`tier_def`]):L0 屏幕空间 probe
//!   (SPG 完整形态,复用 [`crate::gi::spg_rc`] 网格)/ L1 clipmap 体积
//!   probe(DDGI 基线)/ L2 空间哈希缓存 / L3 per-pixel 参考档;**四档共享
//!   probe 着色与八面体编码内核、只换空间索引**——共享内核同一函数实例断言
//!   ([`assert_shared_kernel_instance`],各档复制实现即 RED);档间 golden
//!   对拍可归因到索引结构而非实现差异。
//! - **每档 AS 更新预算硬契约**(D2-Q10):每档定义强制含 **AS 更新预算行**
//!   ([`AsBudgetRow`]),档位切换判据消费 `as_manager` 既有 [`AsStats`] 计数
//!   面;**超 AS 更新预算必须强制降档**——逐级降档且每步显式
//!   [`DemotionRecord`],**禁静默降档**([`audit_demotions`] 独立重算逐条
//!   比对,无记录降档即 fail-closed);档位切换对同输入确定(双运行逐位
//!   一致);切换阈值先 measured 后冻结(SPG/IF 调参阈值为实现确定、非
//!   stable,RFC-0022 §10)。
//! - **golden 对拍**(RXS-0362 L6):各档按匹配深度(1 次间接弹射 ⇒
//!   [`M101_MATCHED_DEPTH`] = 2)对 M96 golden,容差带 measured 后冻结
//!   (milestones/g9/g9_m101_if_tier_band.json;带 = measured ×
//!   [`M101_BAND_MARGIN`],禁手写 P-09);门序消费锚 = M96 cornell 同深度
//!   digest 与 M97 冻结带条目逐字相等(D2-Q7)。
//!
//! ## 确定性协议(承 RXS-0357 L2 同律)
//! - 场景 = M96 冻结 fixture [`path_trace::m96_cornell_scene`];GBuffer =
//!   [`fallback_chain::gbuffer_prepass`] 同一产线;RNG = PCG32 单一流按索引
//!   寻址([`m101_rng`];流为输入非结果);逐实体(probe/像素)独立顺序累加,
//!   禁 atomic;L2 哈希缓存容器 = BTreeMap(迭代序确定)。
//! - 全部 f32;device kernel `kernels/g9_m101_probe_oct.rx` 与 host 镜像
//!   [`oct_probe_trace_host`] 逐字同源。

use crate::gi::fallback_chain::{self, GBuffer, l2_closest_hit, scene_tri};
use crate::gi::path_trace::{self, MaterialKind, PtScene};
use crate::gi::spg_rc;
use crate::rt::as_manager::AsStats;
use crate::rt::ref_tracer::RAY_EPS;

// ---------------------------------------------------------------------------
// 冻结常量(档位参数/预算行/检出阈;实现确定、非 stable,RFC-0022 §10 口径;
// 阈值先 measured 后冻结,禁手写掩盖 P-09)
// ---------------------------------------------------------------------------

/// M101 确定性协议冻结 seed(独立于 M96~M100 流,避免跨里程碑流耦合)。
pub const M101_SEED: u64 = 0x5E10_1F7E_A11D_0007;
/// IF 体素网格维度(cornell 单位盒 [0,1]³ 覆盖;最小 grid)。
pub const M101_GRID_DIMS: [u32; 3] = [4, 4, 4];
/// 体素边长(场景单位)。
pub const M101_GRID_CELL: f32 = 0.25;
/// 八面体 irradiance 图分辨率(8×8,DDGI 基线逐字)。
pub const M101_IRR_RES: u32 = 8;
/// 八面体 visibility 图分辨率(16×16——防漏光优先于提 irradiance 分辨率,
/// RXS-0362 L1 逐字)。
pub const M101_VIS_RES: u32 = 16;
/// visibility 遮蔽距离归一化界(probe oct 图存 `min(t, range)/range` 归一
/// 化遮蔽距离,miss = 1;采样侧以 probe→点距离对比判遮蔽——防漏光语义面)。
pub const M101_VIS_RANGE: f32 = 0.5;
/// 遮蔽判定余量(采样侧:存储距离 + 本余量 ≥ probe→点距离 ⇒ 可见;实现
/// 确定、非 stable,RFC-0022 §10)。
pub const M101_VIS_EPS: f32 = 0.02;
/// L1 每帧轮换更新摊销预算(probe 数/帧;64 probe ⇒ 4 帧全轮换)。
pub const M101_L1_UPDATE_BUDGET: u32 = 16;
/// 档位评估每实体采样数(probe/像素第一反弹估计)。
pub const M101_TIER_SPP: u32 = 4;
/// 匹配深度(1 次间接弹射 ⇒ M96 max_bounces=2 档 golden)。
pub const M101_MATCHED_DEPTH: u32 = 2;
/// M96 golden 参照 spp(与 M97~M99 门序锚同档)。
pub const M101_M96_GOLDEN_SPP: u32 = 64;
/// 容差带倍率(band = measured × margin;禁手写,P-09;沿 M96~M99 口径)。
pub const M101_BAND_MARGIN: f64 = 2.0;
/// SRGB 编码注入 RED 臂检出阈:SRGB 域编码的 L1 出图对线性域 golden 的
/// rel_dev 必须 ≥ 本阈(冻结批实测 = 5.0e-1,阈值 = 实测 ×0.5 margin 冻结;
/// 实测值进 band json `srgb_encode_rel_dev` 字段,禁手写掩盖 P-09)。
pub const M101_SRGB_REL_DEV_MIN: f64 = 0.25;
/// 正下界(与 M96 kernel `tiny` 位级同值)。
const TINY: f32 = 0.000001;

// ---------------------------------------------------------------------------
// 错误面(fail-closed typed Err;本模块一切失败为类型化拒绝,严禁 UB)
// ---------------------------------------------------------------------------

/// M101 host 面错误(编码/网格/选档/审计/容差带全部 fail-closed)。
#[derive(Debug, Clone, PartialEq)]
pub enum IfError {
    /// 配置/输入非法(尺寸不符/索引越界/流长不符等)。
    InvalidConfig(String),
    /// SRGB 编码域注入检出(八面体编码必须线性域;编码域错误漏检即 FAIL
    /// 的负例臂承载)。
    SrgbEncodingInjected(String),
    /// 静默降档检出:实际服务档位发生降档但降档记录缺失/不符(超 AS 预算
    /// 未显式降档即 RED;负例臂承载)。
    SilentDemotion(String),
    /// 深度容差带错误(解析/缺条目/digest 不符/越带)。
    DepthBand(String),
}

impl std::fmt::Display for IfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IfError::InvalidConfig(m) => write!(f, "配置非法: {m}"),
            IfError::SrgbEncodingInjected(m) => {
                write!(f, "SRGB 编码域注入检出(八面体编码须线性域): {m}")
            }
            IfError::SilentDemotion(m) => write!(f, "静默降档检出(超预算未显式降档即 RED): {m}"),
            IfError::DepthBand(m) => write!(f, "深度容差带: {m}"),
        }
    }
}

impl std::error::Error for IfError {}

// ---------------------------------------------------------------------------
// 八面体编码(单一源;线性域;L0~L3 共享)
// ---------------------------------------------------------------------------

/// 八面体编解码内核(单一函数实例面——四档索引结构共用本模块函数;
/// host 与 device kernel `g9_m101_probe_oct.rx` 逐字同源)。
pub mod oct {
    /// sign(0) 取 +1(折叠边界零值防 NaN/方向翻转;与 material::closure
    /// 法线八面体同一约定,本模块为 GI probe 面的单一源)。
    fn sign01(v: f32) -> f32 {
        if v >= 0.0 { 1.0 } else { -1.0 }
    }

    /// 单位方向 → 八面体 UV([-1,1]²;标准 oct 映射:z<0 负半球折叠到菱形
    /// 边界;零向量收敛为 (0,0) → 解码单位 +Z)。
    pub fn encode_unit(dir: [f32; 3]) -> [f32; 2] {
        let l1 = (dir[0].abs() + dir[1].abs() + dir[2].abs()).max(f32::MIN_POSITIVE);
        let mut u = dir[0] / l1;
        let mut v = dir[1] / l1;
        if dir[2] < 0.0 {
            let ou = u;
            u = (1.0 - v.abs()) * sign01(ou);
            v = (1.0 - ou.abs()) * sign01(v);
        }
        [u, v]
    }

    /// 八面体 UV → 单位方向(归一化;UV 截断到 [-1,1];折叠以折叠前
    /// UV 求 z,与 [`encode_unit`] 互逆)。
    pub fn decode_unit(uv: [f32; 2]) -> [f32; 3] {
        let u = uv[0].clamp(-1.0, 1.0);
        let v = uv[1].clamp(-1.0, 1.0);
        let z = 1.0 - u.abs() - v.abs();
        let (mut x, mut y) = (u, v);
        if z < 0.0 {
            let ox = x;
            x = (1.0 - y.abs()) * sign01(ox);
            y = (1.0 - ox.abs()) * sign01(y);
        }
        let len = (x * x + y * y + z * z).sqrt();
        if len < f32::MIN_POSITIVE {
            return [0.0, 0.0, 1.0];
        }
        [x / len, y / len, z / len]
    }

    /// 量化编码(res×res texel 下标;texel 中心对齐)。
    pub fn encode_quant(dir: [f32; 3], res: u32) -> (u32, u32) {
        let [u, v] = encode_unit(dir);
        let q = |t: f32| -> u32 {
            (((t * 0.5 + 0.5) * res as f32).floor() as i64).clamp(0, res as i64 - 1) as u32
        };
        (q(u), q(v))
    }

    /// texel 下标 → 中心方向解码。
    pub fn decode_quant(ix: u32, iy: u32, res: u32) -> [f32; 3] {
        let u = ((ix as f32 + 0.5) / res as f32) * 2.0 - 1.0;
        let v = ((iy as f32 + 0.5) / res as f32) * 2.0 - 1.0;
        decode_unit([u, v])
    }

    /// 双线性采样(3ch radiance 图;UV 域 texel 中心对齐)。
    pub fn sample_bilinear3(map: &[[f32; 3]], res: u32, dir: [f32; 3]) -> [f32; 3] {
        let [u, v] = encode_unit(dir);
        let fx = (u * 0.5 + 0.5) * res as f32 - 0.5;
        let fy = (v * 0.5 + 0.5) * res as f32 - 0.5;
        let x0 = fx.floor().max(0.0).min(res as f32 - 1.0) as u32;
        let y0 = fy.floor().max(0.0).min(res as f32 - 1.0) as u32;
        let x1 = (x0 + 1).min(res - 1);
        let y1 = (y0 + 1).min(res - 1);
        let tx = fx.max(0.0).min(res as f32 - 1.0) - x0 as f32;
        let ty = fy.max(0.0).min(res as f32 - 1.0) - y0 as f32;
        let mut out = [0.0f32; 3];
        for c in 0..3 {
            let a = map[(y0 * res + x0) as usize][c];
            let b = map[(y0 * res + x1) as usize][c];
            let c0 = map[(y1 * res + x0) as usize][c];
            let d = map[(y1 * res + x1) as usize][c];
            out[c] = a * (1.0 - tx) * (1.0 - ty)
                + b * tx * (1.0 - ty)
                + c0 * (1.0 - tx) * ty
                + d * tx * ty;
        }
        out
    }

    /// 双线性采样(1ch visibility 图)。
    pub fn sample_bilinear1(map: &[f32], res: u32, dir: [f32; 3]) -> f32 {
        let [u, v] = encode_unit(dir);
        let fx = (u * 0.5 + 0.5) * res as f32 - 0.5;
        let fy = (v * 0.5 + 0.5) * res as f32 - 0.5;
        let x0 = fx.floor().max(0.0).min(res as f32 - 1.0) as u32;
        let y0 = fy.floor().max(0.0).min(res as f32 - 1.0) as u32;
        let x1 = (x0 + 1).min(res - 1);
        let y1 = (y0 + 1).min(res - 1);
        let tx = fx.max(0.0).min(res as f32 - 1.0) - x0 as f32;
        let ty = fy.max(0.0).min(res as f32 - 1.0) - y0 as f32;
        let a = map[(y0 * res + x0) as usize];
        let b = map[(y0 * res + x1) as usize];
        let c = map[(y1 * res + x0) as usize];
        let d = map[(y1 * res + x1) as usize];
        a * (1.0 - tx) * (1.0 - ty) + b * tx * (1.0 - ty) + c * (1.0 - tx) * ty + d * tx * ty
    }
}

/// irradiance 图(8×8)量化编解码往返角误差界(rad;冻结批实测最大值
/// 4.99e-1〔fibonacci 球面 4096 方向全量〕×2 margin 冻结,禁手写 P-09;
/// 单测以确定性方向集全量核验)。
pub const OCT_ROUNDTRIP_BOUND_IRR: f32 = 1.0;
/// visibility 图(16×16)量化编解码往返角误差界(rad;冻结批实测最大值
/// 2.36e-1 ×2 margin 冻结,同律)。
pub const OCT_ROUNDTRIP_BOUND_VIS: f32 = 0.48;

/// 编码域(闭集):线性(正例)/ SRGB 注入(RED 臂 variant)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeDomain {
    /// 线性域(冻结面;辐射度采样直存)。
    Linear,
    /// SRGB 编码域注入(γ≈1/2.2 后存储;编码域错误负例臂)。
    SrgbInjected,
}

/// 编码域应用(radiance 值域;八面体编码必须线性域——注入路径仅 RED 臂)。
pub fn apply_domain(rgb: [f32; 3], domain: EncodeDomain) -> [f32; 3] {
    match domain {
        EncodeDomain::Linear => rgb,
        EncodeDomain::SrgbInjected => [
            rgb[0].max(0.0).powf(1.0 / 2.2),
            rgb[1].max(0.0).powf(1.0 / 2.2),
            rgb[2].max(0.0).powf(1.0 / 2.2),
        ],
    }
}

// ---------------------------------------------------------------------------
// RNG 流(PCG32 单一流按索引寻址;流为输入非结果)
// ---------------------------------------------------------------------------

/// M101 流布局(冻结):逐实体(probe/像素/桶)逐采样 4 维
/// [bsdf_r1, bsdf_r2, nee_u, nee_v](与 M99 同形,独立流)。
pub mod m101_rng {
    use crate::rt::ref_tracer::Pcg32;

    /// 每采样随机维数。
    pub const DIMS_PER_SAMPLE: usize = 4;

    /// 流总长(= entity_count · spp · 4)。
    pub fn stream_len(entity_count: usize, spp: u32) -> usize {
        entity_count * spp as usize * DIMS_PER_SAMPLE
    }

    /// 采样 (entity, sample) 的流起始下标。
    pub fn sample_base(entity: usize, sample: usize, spp: u32) -> usize {
        (entity * spp as usize + sample) * DIMS_PER_SAMPLE
    }

    /// 生成整条流(单 [`Pcg32`] 实例,实体序 × 采样序顺序产出)。
    pub fn generate_stream(entity_count: usize, spp: u32, seed: u64) -> Vec<f32> {
        let mut rng = Pcg32::new(seed);
        let mut out = Vec::with_capacity(stream_len(entity_count, spp));
        for _ in 0..stream_len(entity_count, spp) {
            out.push(rng.next_f32());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// 共享 probe 着色内核(单一函数实例;四档共用,只换空间索引——D2-Q5)
// ---------------------------------------------------------------------------

/// 三角形几何法线(依 winding,归一化;M96 host 同式)。
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

/// 共享 probe 着色内核(**单一函数实例**;L0~L3 四档 dispatch 全部指向本
/// 函数——各档复制实现即 RED,[`assert_shared_kernel_instance`] 机器锚)。
///
/// 语义 = 第一反弹出射辐射度估计(NEE×MIS + BSDF 命中发光面×MIS,M96
/// megakernel shade①②③ 单反弹特例逐字同式;与 M99 probe 公式面同族)。
/// `dims` = 4 维流样本 [bsdf_r1, bsdf_r2, nee_u, nee_v]。
pub fn shade_probe_shared(
    scene: &PtScene,
    p: [f32; 3],
    n: [f32; 3],
    albedo: [f32; 3],
    dims: [f32; 4],
) -> [f32; 3] {
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
    // ── NEE(M96 shade② 逐字同式)──
    let q = fallback_chain::light_sample(scene, dims[2], dims[3]);
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
        let (blocked, _t) = l2_closest_hit(scene, so, wi, t_sh);
        let vis = if blocked.is_some() { 0.0 } else { 1.0 };
        li[0] += albedo[0] * le[0] * nee_core * w_l * vis;
        li[1] += albedo[1] * le[1] * nee_core * w_l * vis;
        li[2] += albedo[2] * le[2] * nee_core * w_l * vis;
    }
    // ── BSDF 余弦采样命中发光面(M96 shade①③ 逐字同式)──
    let nd = fallback_chain::cosine_dir(n, dims[0], dims[1]);
    let (hit, _t) = l2_closest_hit(scene, so, nd, scene.t_max);
    if let Some((t, tri)) = hit
        && let MaterialKind::Emission { emission, .. } = &scene.materials[tri as usize]
    {
        let (a, b, c) = scene_tri(scene, tri as usize);
        let ng = tri_normal(a, b, c);
        let cos_emit = -(ng[0] * nd[0] + ng[1] * nd[1] + ng[2] * nd[2]);
        if emission.iter().any(|&x| x > 0.0) && cos_emit > 0.0 {
            let cos_nd = (n[0] * nd[0] + n[1] * nd[1] + n[2] * nd[2]).max(0.0);
            let pdf_b = path_trace::cosine_hemisphere_pdf(cos_nd);
            let w_b = path_trace::mis_weight_bsdf(t, area, cos_emit, pdf_b);
            li[0] += albedo[0] * emission[0] * w_b;
            li[1] += albedo[1] * emission[1] * w_b;
            li[2] += albedo[2] * emission[2] * w_b;
        }
    }
    li
}

/// 共享 probe 着色内核函数指针形(dispatch 表面;四档同一实例)。
pub type ShadeProbeFn = fn(&PtScene, [f32; 3], [f32; 3], [f32; 3], [f32; 4]) -> [f32; 3];

/// 档位着色 dispatch 表项(四档共享内核的机器锚面)。
pub struct TierShadeDispatch {
    /// 档位。
    pub tier: IfTier,
    /// probe 着色内核函数指针(四档必须同一实例)。
    pub shade: ShadeProbeFn,
}

/// 四档 dispatch 表(全部指向 [`shade_probe_shared`] 同一实例)。
pub const TIER_DISPATCH: [TierShadeDispatch; 4] = [
    TierShadeDispatch {
        tier: IfTier::L0ScreenProbe,
        shade: shade_probe_shared,
    },
    TierShadeDispatch {
        tier: IfTier::L1ClipmapVolume,
        shade: shade_probe_shared,
    },
    TierShadeDispatch {
        tier: IfTier::L2SpatialHash,
        shade: shade_probe_shared,
    },
    TierShadeDispatch {
        tier: IfTier::L3PerPixel,
        shade: shade_probe_shared,
    },
];

/// 共享内核同一函数实例断言(RXS-0362 L2:各档复制实现即 RED——函数指针
/// 两两相等才过;`fn_addr_eq` 为稳定面)。
pub fn assert_shared_kernel_instance() -> bool {
    TIER_DISPATCH
        .windows(2)
        .all(|w| std::ptr::fn_addr_eq(w[0].shade, w[1].shade))
}

// ---------------------------------------------------------------------------
// 档位阶梯 L0~L3 闭集 + 每档 AS 更新预算行(D2-Q10)
// ---------------------------------------------------------------------------

/// IF 档位(阶梯闭集;index = dispatch/budget 编码)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IfTier {
    /// L0 屏幕空间 probe(SPG 完整形态)。
    L0ScreenProbe,
    /// L1 clipmap 体积 probe(DDGI 基线)。
    L1ClipmapVolume,
    /// L2 空间哈希缓存(SHaRC 式)。
    L2SpatialHash,
    /// L3 per-pixel(全分辨率逐像素追踪;参考档/截图档)。
    L3PerPixel,
}

impl IfTier {
    /// 阶梯升序(L0→L3)。
    pub const ALL: [IfTier; 4] = [
        IfTier::L0ScreenProbe,
        IfTier::L1ClipmapVolume,
        IfTier::L2SpatialHash,
        IfTier::L3PerPixel,
    ];

    /// evidence 名(稳定字面)。
    pub fn name(self) -> &'static str {
        match self {
            IfTier::L0ScreenProbe => "l0_screen_probe",
            IfTier::L1ClipmapVolume => "l1_clipmap_volume",
            IfTier::L2SpatialHash => "l2_spatial_hash",
            IfTier::L3PerPixel => "l3_per_pixel",
        }
    }

    /// 降一档(L0 为终端 ⇒ None)。
    pub fn demote(self) -> Option<IfTier> {
        match self {
            IfTier::L3PerPixel => Some(IfTier::L2SpatialHash),
            IfTier::L2SpatialHash => Some(IfTier::L1ClipmapVolume),
            IfTier::L1ClipmapVolume => Some(IfTier::L0ScreenProbe),
            IfTier::L0ScreenProbe => None,
        }
    }
}

/// 每档 AS 更新预算行(硬契约;D2-Q10——消费 `as_manager::AsStats` 计数面。
/// 语义:该档可承受的每帧 AS 更新上限;高档逐像素追踪须新鲜 AS,预算行
/// 最紧;低档 probe 结构容忍滞更,预算行最宽——行值随档位单调)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsBudgetRow {
    /// 每帧 BLAS 全量构建上限。
    pub max_blas_builds: u64,
    /// 每帧 BLAS refit 上限。
    pub max_refits: u64,
    /// 每帧 TLAS 重建上限。
    pub max_tlas_rebuilds: u64,
}

/// 档位定义行(闭集;每档强制含 AS 更新预算行 + 索引结构字面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierDef {
    /// 档位。
    pub tier: IfTier,
    /// 空间索引结构字面(档间 golden 对拍归因面)。
    pub index_kind: &'static str,
    /// AS 更新预算行。
    pub as_budget: AsBudgetRow,
}

/// 档位定义(阶梯闭集;预算行冻结——实现确定、非 stable,RFC-0022 §10)。
pub fn tier_def(tier: IfTier) -> TierDef {
    match tier {
        IfTier::L0ScreenProbe => TierDef {
            tier,
            index_kind: "screen_probe_grid(spg_rc 自适应网格)",
            as_budget: AsBudgetRow {
                max_blas_builds: 8,
                max_refits: 64,
                max_tlas_rebuilds: 8,
            },
        },
        IfTier::L1ClipmapVolume => TierDef {
            tier,
            index_kind: "clipmap_volume_probe(4×4×4 体素 + oct irr8/vis16)",
            as_budget: AsBudgetRow {
                max_blas_builds: 4,
                max_refits: 32,
                max_tlas_rebuilds: 4,
            },
        },
        IfTier::L2SpatialHash => TierDef {
            tier,
            index_kind: "spatial_hash_cache(位置量化+法线卦限键)",
            as_budget: AsBudgetRow {
                max_blas_builds: 2,
                max_refits: 16,
                max_tlas_rebuilds: 2,
            },
        },
        IfTier::L3PerPixel => TierDef {
            tier,
            index_kind: "per_pixel(全分辨率逐像素)",
            as_budget: AsBudgetRow {
                max_blas_builds: 1,
                max_refits: 8,
                max_tlas_rebuilds: 1,
            },
        },
    }
}

/// 预算核验:观测 AS 更新是否超出档位预算行(逐项;任一超即超预算)。
pub fn exceeds_budget(observed: &AsStats, budget: &AsBudgetRow) -> bool {
    observed.blas_builds > budget.max_blas_builds
        || observed.refits > budget.max_refits
        || observed.tlas_rebuilds > budget.max_tlas_rebuilds
}

/// 降档原因(闭集)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemotionCause {
    /// 超 AS 更新预算(D2-Q10)。
    AsBudgetExceeded,
}

/// 降档记录(禁静默降档的显式日志;每级一步一条)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemotionRecord {
    /// 降出档。
    pub from: IfTier,
    /// 降入档。
    pub to: IfTier,
    /// 原因(闭集)。
    pub cause: DemotionCause,
    /// 观测 AS 更新(消费面快照)。
    pub observed: AsStats,
    /// 被超预算行。
    pub budget: AsBudgetRow,
}

/// 选档器(纯函数;对同输入确定):自 `wanted` 起,观测超当前档预算行 ⇒
/// 强制逐级降档,每步一条 [`DemotionRecord`];`log=false` = 静默降档注入
/// variant(负例臂;[`audit_demotions`] 必 fail-closed)。L0 为终端
/// (超预算亦无档可降——终端显式,记录止步于 L0)。
pub fn select_tier(wanted: IfTier, observed: &AsStats, log: bool) -> (IfTier, Vec<DemotionRecord>) {
    let mut cur = wanted;
    let mut records = Vec::new();
    while exceeds_budget(observed, &tier_def(cur).as_budget) {
        let Some(next) = cur.demote() else { break };
        if log {
            records.push(DemotionRecord {
                from: cur,
                to: next,
                cause: DemotionCause::AsBudgetExceeded,
                observed: *observed,
                budget: tier_def(cur).as_budget,
            });
        }
        cur = next;
    }
    (cur, records)
}

/// 降档审计(fail-closed):独立重算期望降档链(同输入重跑选档器参考路径)
/// 并逐条比对——实际服务档位与期望不符、或降档发生而记录缺失/错位 =
/// 静默降档(超限未降档/静默降档即 RED)。
pub fn audit_demotions(
    wanted: IfTier,
    observed: &AsStats,
    served: IfTier,
    records: &[DemotionRecord],
) -> Result<(), IfError> {
    let (expect, expect_records) = select_tier(wanted, observed, true);
    if served != expect {
        return Err(IfError::SilentDemotion(format!(
            "实际服务档位 {} 与预算判定期望 {} 不符(超预算未降档/私自降档)",
            served.name(),
            expect.name()
        )));
    }
    if records != expect_records {
        return Err(IfError::SilentDemotion(format!(
            "降档记录 {} 条与期望 {} 条不符(静默降档检出)",
            records.len(),
            expect_records.len()
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// IF 体素网格 + probe 八面体图(L1 clipmap 体积 probe,DDGI 基线)
// ---------------------------------------------------------------------------

/// 单 probe 八面体图(irradiance 8×8 RGB + visibility 16×16;线性域)。
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeOctMaps {
    /// irradiance 图(8×8×3;入射辐射度沿 texel 方向)。
    pub irr: Vec<[f32; 3]>,
    /// visibility 图(16×16;0=遮蔽 1=无遮)。
    pub vis: Vec<f32>,
}

/// IF 体素网格(最小 grid;逐 probe oct 图 + 轮换更新游标)。
#[derive(Debug, Clone, PartialEq)]
pub struct IfVoxelGrid {
    /// 维度。
    pub dims: [u32; 3],
    /// 体素边长。
    pub cell: f32,
    /// 逐 probe oct 图(行主序 x + y·dx + z·dx·dy)。
    pub probes: Vec<ProbeOctMaps>,
    /// 轮换更新游标(每帧摊销确定性)。
    pub update_cursor: u32,
}

impl Default for IfVoxelGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl IfVoxelGrid {
    /// 空网格构建(probe 位置 = 体素中心;cornell [0,1]³ 覆盖)。
    pub fn new() -> IfVoxelGrid {
        let [dx, dy, dz] = M101_GRID_DIMS;
        let count = (dx * dy * dz) as usize;
        IfVoxelGrid {
            dims: M101_GRID_DIMS,
            cell: M101_GRID_CELL,
            probes: vec![
                ProbeOctMaps {
                    irr: vec![[0.0; 3]; (M101_IRR_RES * M101_IRR_RES) as usize],
                    vis: vec![0.0; (M101_VIS_RES * M101_VIS_RES) as usize],
                };
                count
            ],
            update_cursor: 0,
        }
    }

    /// probe 总数。
    pub fn probe_count(&self) -> usize {
        self.probes.len()
    }

    /// probe 世界位置(体素中心)。
    pub fn probe_pos(&self, idx: usize) -> [f32; 3] {
        let [dx, dy, _dz] = self.dims;
        let x = idx as u32 % dx;
        let y = (idx as u32 / dx) % dy;
        let z = idx as u32 / (dx * dy);
        [
            (x as f32 + 0.5) * self.cell,
            (y as f32 + 0.5) * self.cell,
            (z as f32 + 0.5) * self.cell,
        ]
    }

    /// 位置 → probe 下标(截断到网格内;界外 None)。
    pub fn lookup(&self, p: [f32; 3]) -> Option<usize> {
        let [dx, dy, dz] = self.dims;
        let mut c = [0u32; 3];
        for k in 0..3 {
            if p[k] < 0.0 || p[k] >= dx.max(dy).max(dz) as f32 * self.cell {
                return None;
            }
            c[k] = (p[k] / self.cell).floor() as u32;
            if c[k] >= [dx, dy, dz][k] {
                return None;
            }
        }
        Some((c[2] * dx * dy + c[1] * dx + c[0]) as usize)
    }

    /// 每帧轮换更新摊销(确定性游标;返回本帧应更新的 probe 下标段——
    /// 预算 [`M101_L1_UPDATE_BUDGET`] 个/帧,绕回取模;计数面进 evidence)。
    pub fn rotate_update(&mut self) -> Vec<u32> {
        let n = self.probes.len() as u32;
        let budget = M101_L1_UPDATE_BUDGET.min(n);
        let mut out = Vec::with_capacity(budget as usize);
        for _ in 0..budget {
            out.push(self.update_cursor);
            self.update_cursor = (self.update_cursor + 1) % n;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// probe 八面体图求值(host 镜像,与 kernel `g9_m101_probe_oct` 逐字同源;
// 门绿由 device 腿承载——仅 host 输出不能充绿)
// ---------------------------------------------------------------------------

/// 八面体图追踪模式(kernel params[2] 位级同值)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OctTraceMode {
    /// irradiance 图(8×8;命中点直接光 NEE 近似 × 阴影)。
    Irradiance,
    /// visibility 图(16×16;命中 t < [`M101_VIS_RANGE`] ⇒ 遮蔽)。
    Visibility,
}

impl OctTraceMode {
    /// kernel 参数位级编码。
    pub fn as_f32(self) -> f32 {
        match self {
            OctTraceMode::Irradiance => 0.0,
            OctTraceMode::Visibility => 1.0,
        }
    }

    /// 每 probe texel 数。
    pub fn texels(self) -> u32 {
        match self {
            OctTraceMode::Irradiance => M101_IRR_RES * M101_IRR_RES,
            OctTraceMode::Visibility => M101_VIS_RES * M101_VIS_RES,
        }
    }

    /// 图分辨率。
    pub fn res(self) -> u32 {
        match self {
            OctTraceMode::Irradiance => M101_IRR_RES,
            OctTraceMode::Visibility => M101_VIS_RES,
        }
    }
}

/// oct 图单 texel 求值(host 镜像;与 kernel 逐字同源):自 probe 沿 texel
/// 方向 ray query(暴力解析求值代理);Irradiance ⇒ 命中点直接光(点光源
/// 近似 × 阴影,M98 L1 档公式面)+ 未命中零;Visibility ⇒ 遮蔽位。
pub fn oct_texel_trace_host(
    scene: &PtScene,
    probe: [f32; 3],
    texel: u32,
    mode: OctTraceMode,
) -> ([f32; 3], f32) {
    let res = mode.res();
    let dir = oct::decode_quant(texel % res, texel / res, res);
    let (hit, _t) = l2_closest_hit(scene, probe, dir, scene.t_max);
    match mode {
        OctTraceMode::Visibility => {
            // 归一化遮蔽距离:命中 ⇒ min(t/range, 1);miss ⇒ 1(采样侧以
            // probe→点距离对比判遮蔽)。
            let v = match hit {
                Some((t, _)) => (t / M101_VIS_RANGE).min(1.0),
                None => 1.0,
            };
            ([0.0; 3], v)
        }
        OctTraceMode::Irradiance => {
            let Some((t, tri)) = hit else {
                return ([0.0; 3], 1.0);
            };
            let hp = [
                probe[0] + dir[0] * t,
                probe[1] + dir[1] * t,
                probe[2] + dir[2] * t,
            ];
            let (a, b, c) = scene_tri(scene, tri as usize);
            let ng = tri_normal(a, b, c);
            let n = if ng[0] * dir[0] + ng[1] * dir[1] + ng[2] * dir[2] > 0.0 {
                [-ng[0], -ng[1], -ng[2]]
            } else {
                ng
            };
            let albedo = match &scene.materials[tri as usize] {
                MaterialKind::Lambert { albedo } => *albedo,
                MaterialKind::Emission { albedo, .. } => *albedo,
                _ => [0.0; 3],
            };
            let rgb = fallback_chain::shade_point_unshadowed(albedo, hp, n, scene);
            // 自发光面直入(命中灯 quad ⇒ 入射辐射度 = 发光)。
            if let MaterialKind::Emission { emission, .. } = &scene.materials[tri as usize] {
                let cos_emit = -(ng[0] * dir[0] + ng[1] * dir[1] + ng[2] * dir[2]);
                if emission.iter().any(|&x| x > 0.0) && cos_emit > 0.0 {
                    return (*emission, 0.0);
                }
            }
            (rgb, 0.0)
        }
    }
}

/// 全网格 oct 图求值(host 镜像;`domain` = 编码域——Linear 正例 /
/// SrgbInjected RED 臂 variant,作用于 irradiance 图存储)。
pub fn oct_probe_trace_host(
    scene: &PtScene,
    grid: &IfVoxelGrid,
    domain: EncodeDomain,
) -> Vec<ProbeOctMaps> {
    let mut out = Vec::with_capacity(grid.probe_count());
    for pi in 0..grid.probe_count() {
        let pos = grid.probe_pos(pi);
        let mut irr = vec![[0.0f32; 3]; (M101_IRR_RES * M101_IRR_RES) as usize];
        let mut vis = vec![0.0f32; (M101_VIS_RES * M101_VIS_RES) as usize];
        for tx in 0..(M101_IRR_RES * M101_IRR_RES) {
            let (rgb, _v) = oct_texel_trace_host(scene, pos, tx, OctTraceMode::Irradiance);
            irr[tx as usize] = apply_domain(rgb, domain);
        }
        for tx in 0..(M101_VIS_RES * M101_VIS_RES) {
            let (_rgb, v) = oct_texel_trace_host(scene, pos, tx, OctTraceMode::Visibility);
            vis[tx as usize] = v;
        }
        out.push(ProbeOctMaps { irr, vis });
    }
    out
}

// ---------------------------------------------------------------------------
// 四档评估(共享着色内核,只换空间索引;rgb = direct + albedo × 档间接辐射度)
// ---------------------------------------------------------------------------

/// 档位帧产物(合成辐射度 + 服务档 + 计数面)。
#[derive(Debug, Clone, PartialEq)]
pub struct TierFrame {
    /// 服务档。
    pub tier: IfTier,
    /// 图宽。
    pub width: u32,
    /// 图高。
    pub height: u32,
    /// 合成辐射度 RGB(3 f32/px;主未命中 = 0,沿 GBuffer 口径)。
    pub rgb: Vec<f32>,
    /// L2 缓存计数(命中/失效;他档恒零显式)。
    pub cache_hits: u64,
    /// L2 缓存失效数。
    pub cache_misses: u64,
}

impl TierFrame {
    /// 产物 digest = sha256(rgb 字节 ‖ 服务档 id)(服务档入键——档位转移
    /// 必然改变产物 digest,结构性保证档间对拍可归因)。
    pub fn product_digest(&self) -> [u8; 32] {
        let mut pre = Vec::with_capacity(self.rgb.len() * 4 + 4);
        for v in &self.rgb {
            pre.extend_from_slice(&v.to_le_bytes());
        }
        pre.extend_from_slice(&(self.tier as u32).to_le_bytes());
        rurix_pkg::sha256::digest(&pre)
    }
}

/// 主命中像素世界信息(p = sec_o 回退 n·eps;与 GBuffer 预传递同一产线)。
fn primary_point(gb: &GBuffer, i: usize) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let n = [gb.nrm[i * 3], gb.nrm[i * 3 + 1], gb.nrm[i * 3 + 2]];
    let p = [
        gb.sec_o[i * 3] - n[0] * RAY_EPS,
        gb.sec_o[i * 3 + 1] - n[1] * RAY_EPS,
        gb.sec_o[i * 3 + 2] - n[2] * RAY_EPS,
    ];
    let a = [gb.alb[i * 3], gb.alb[i * 3 + 1], gb.alb[i * 3 + 2]];
    (p, n, a)
}

/// 合成像素(共享式:rgb = direct + albedo × indirect;主未命中 = 0;
/// FnMut 容纳 L2 缓存插入/计数)。
fn compose(gb: &GBuffer, mut indirect: impl FnMut(usize) -> [f32; 3]) -> Vec<f32> {
    let pixel_count = (gb.width * gb.height) as usize;
    let mut rgb = vec![0.0f32; pixel_count * 3];
    for i in 0..pixel_count {
        if !gb.primary_hit[i] {
            continue;
        }
        let ind = indirect(i);
        let a = [gb.alb[i * 3], gb.alb[i * 3 + 1], gb.alb[i * 3 + 2]];
        rgb[i * 3] = gb.direct[i * 3] + a[0] * ind[0];
        rgb[i * 3 + 1] = gb.direct[i * 3 + 1] + a[1] * ind[1];
        rgb[i * 3 + 2] = gb.direct[i * 3 + 2] + a[2] * ind[2];
    }
    rgb
}

/// L3 per-pixel 参考档(逐像素第一反弹;共享内核 [`shade_probe_shared`])。
pub fn eval_l3_per_pixel(scene: &PtScene, gb: &GBuffer) -> TierFrame {
    let pixel_count = (gb.width * gb.height) as usize;
    let stream = m101_rng::generate_stream(pixel_count, M101_TIER_SPP, M101_SEED);
    let rgb = compose(gb, |i| {
        let (p, n, a) = primary_point(gb, i);
        let mut acc = [0.0f32; 3];
        for s in 0..M101_TIER_SPP as usize {
            let sb = m101_rng::sample_base(i, s, M101_TIER_SPP);
            let li = shade_probe_shared(
                scene,
                p,
                n,
                a,
                [stream[sb], stream[sb + 1], stream[sb + 2], stream[sb + 3]],
            );
            acc[0] += li[0];
            acc[1] += li[1];
            acc[2] += li[2];
        }
        let inv = 1.0 / M101_TIER_SPP as f32;
        [acc[0] * inv, acc[1] * inv, acc[2] * inv]
    });
    TierFrame {
        tier: IfTier::L3PerPixel,
        width: gb.width,
        height: gb.height,
        rgb,
        cache_hits: 0,
        cache_misses: 0,
    }
}

/// L0 屏幕空间 probe 档(SPG 完整形态:复用 [`spg_rc`] 自适应网格 + 共享
/// 内核逐 probe 求值 + 3×3 滤波 tile 图查找;只换空间索引)。
pub fn eval_l0_screen_probe(scene: &PtScene, gb: &GBuffer) -> TierFrame {
    let grid = spg_rc::build_spg_grid(gb, true);
    let n_valid = spg_rc::valid_probe_count(&grid);
    let stream = m101_rng::generate_stream(n_valid, M101_TIER_SPP, M101_SEED);
    // 逐有效 probe 共享内核求值(流按下标序 = 有效序)。
    let mut vals = vec![[0.0f32; 3]; grid.probes.len()];
    let mut vi = 0usize;
    for (pi, probe) in grid.probes.iter().enumerate() {
        if !probe.valid {
            continue;
        }
        let mut acc = [0.0f32; 3];
        for s in 0..M101_TIER_SPP as usize {
            let sb = m101_rng::sample_base(vi, s, M101_TIER_SPP);
            let li = shade_probe_shared(
                scene,
                probe.pos,
                probe.normal,
                probe.albedo,
                [stream[sb], stream[sb + 1], stream[sb + 2], stream[sb + 3]],
            );
            acc[0] += li[0];
            acc[1] += li[1];
            acc[2] += li[2];
        }
        vi += 1;
        let inv = 1.0 / M101_TIER_SPP as f32;
        vals[pi] = [acc[0] * inv, acc[1] * inv, acc[2] * inv];
    }
    // tile 图 + 3×3 滤波(G8 底座权重律,spg_rc 同面)。
    let tw = gb.width.div_ceil(spg_rc::M99_FILTER_CELL);
    let th = gb.height.div_ceil(spg_rc::M99_FILTER_CELL);
    let mut rad = vec![[0.0f32; 3]; (tw * th) as usize];
    let mut dep = vec![0.0f32; (tw * th) as usize];
    let mut nrm = vec![[0.0f32; 3]; (tw * th) as usize];
    let mut valid = vec![false; (tw * th) as usize];
    for ty in 0..th {
        for tx in 0..tw {
            let t = (ty * tw + tx) as usize;
            let cx = (tx * spg_rc::M99_FILTER_CELL + spg_rc::M99_FILTER_CELL / 2).min(gb.width - 1);
            let cy =
                (ty * spg_rc::M99_FILTER_CELL + spg_rc::M99_FILTER_CELL / 2).min(gb.height - 1);
            let pi = grid.pixel_probe[(cy * gb.width + cx) as usize];
            if pi == u32::MAX || !grid.probes[pi as usize].valid {
                continue;
            }
            let probe = &grid.probes[pi as usize];
            rad[t] = vals[pi as usize];
            let ai = (probe.anchor[1] * gb.width + probe.anchor[0]) as usize;
            dep[t] = gb.depth[ai];
            nrm[t] = [gb.nrm[ai * 3], gb.nrm[ai * 3 + 1], gb.nrm[ai * 3 + 2]];
            valid[t] = true;
        }
    }
    let filtered = spg_rc::filter_radiance_3x3(tw, th, &rad, &dep, &nrm, &valid);
    let rgb = compose(gb, |i| {
        let t = ((i as u32 / gb.width / spg_rc::M99_FILTER_CELL) * tw
            + (i as u32 % gb.width) / spg_rc::M99_FILTER_CELL) as usize;
        filtered[t]
    });
    TierFrame {
        tier: IfTier::L0ScreenProbe,
        width: gb.width,
        height: gb.height,
        rgb,
        cache_hits: 0,
        cache_misses: 0,
    }
}

/// L1 clipmap 体积 probe 档(体素网格 + oct 图;`maps` = device/host oct 图
/// 输入——采样与编码域在装配侧;irradiance 沿法线双线性采样 × visibility
/// 沿 probe→点方向加权(防漏光优先))。
pub fn eval_l1_clipmap_volume(
    gb: &GBuffer,
    grid: &IfVoxelGrid,
    maps: &[ProbeOctMaps],
) -> Result<TierFrame, IfError> {
    if maps.len() != grid.probe_count() {
        return Err(IfError::InvalidConfig(format!(
            "oct 图数 {} ≠ probe 数 {}",
            maps.len(),
            grid.probe_count()
        )));
    }
    let rgb = compose(gb, |i| {
        let (p, n, _a) = primary_point(gb, i);
        let Some(pi) = grid.lookup(p) else {
            return [0.0; 3];
        };
        let c = grid.probe_pos(pi);
        let irr = oct::sample_bilinear3(&maps[pi].irr, M101_IRR_RES, n);
        let dv = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
        let dl = (dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2])
            .sqrt()
            .max(TINY);
        let dir = [dv[0] / dl, dv[1] / dl, dv[2] / dl];
        // 防漏光:存储遮蔽距离(双线性)+ 余量 ≥ probe→点距离 ⇒ 可见;
        // 遮蔽 probe 贡献压止(visibility 优先语义的消费面)。
        let dist_est = oct::sample_bilinear1(&maps[pi].vis, M101_VIS_RES, dir) * M101_VIS_RANGE;
        let vis = if dist_est + M101_VIS_EPS >= dl {
            1.0
        } else {
            0.0
        };
        [irr[0] * vis, irr[1] * vis, irr[2] * vis]
    });
    Ok(TierFrame {
        tier: IfTier::L1ClipmapVolume,
        width: gb.width,
        height: gb.height,
        rgb,
        cache_hits: 0,
        cache_misses: 0,
    })
}

/// L2 空间哈希缓存键(位置量化 + 法线卦限;确定性闭式)。
fn spatial_hash_key(p: [f32; 3], n: [f32; 3]) -> u64 {
    let q = |v: f32| -> u64 { ((v / M101_GRID_CELL).floor() as i64) as u64 & 0x3FFF };
    let octant = ((if n[0] >= 0.0 { 1u64 } else { 0 })
        | (if n[1] >= 0.0 { 2 } else { 0 })
        | (if n[2] >= 0.0 { 4 } else { 0 }))
        << 42;
    q(p[0]) | (q(p[1]) << 14) | (q(p[2]) << 28) | octant
}

/// L2 空间哈希缓存档(SHaRC 式按需分级:逐像素位置/法线键查询,失效即
/// 共享内核求值并插入;BTreeMap 迭代序确定;命中/失效计数面)。
pub fn eval_l2_spatial_hash(scene: &PtScene, gb: &GBuffer) -> TierFrame {
    let pixel_count = (gb.width * gb.height) as usize;
    let stream = m101_rng::generate_stream(pixel_count, M101_TIER_SPP, M101_SEED);
    let mut cache: std::collections::BTreeMap<u64, [f32; 3]> = Default::default();
    let mut hits = 0u64;
    let mut misses = 0u64;
    let rgb = compose(gb, |i| {
        let (p, n, a) = primary_point(gb, i);
        let key = spatial_hash_key(p, n);
        if let Some(&v) = cache.get(&key) {
            hits += 1;
            return v;
        }
        misses += 1;
        let mut acc = [0.0f32; 3];
        for s in 0..M101_TIER_SPP as usize {
            let sb = m101_rng::sample_base(i, s, M101_TIER_SPP);
            let li = shade_probe_shared(
                scene,
                p,
                n,
                a,
                [stream[sb], stream[sb + 1], stream[sb + 2], stream[sb + 3]],
            );
            acc[0] += li[0];
            acc[1] += li[1];
            acc[2] += li[2];
        }
        let inv = 1.0 / M101_TIER_SPP as f32;
        let v = [acc[0] * inv, acc[1] * inv, acc[2] * inv];
        cache.insert(key, v);
        v
    });
    TierFrame {
        tier: IfTier::L2SpatialHash,
        width: gb.width,
        height: gb.height,
        rgb,
        cache_hits: hits,
        cache_misses: misses,
    }
}

/// 档位评估分发(只换空间索引;L1 需 oct 图输入)。
pub fn eval_tier(
    tier: IfTier,
    scene: &PtScene,
    gb: &GBuffer,
    grid: &IfVoxelGrid,
    maps: &[ProbeOctMaps],
) -> Result<TierFrame, IfError> {
    match tier {
        IfTier::L0ScreenProbe => Ok(eval_l0_screen_probe(scene, gb)),
        IfTier::L1ClipmapVolume => eval_l1_clipmap_volume(gb, grid, maps),
        IfTier::L2SpatialHash => Ok(eval_l2_spatial_hash(scene, gb)),
        IfTier::L3PerPixel => Ok(eval_l3_per_pixel(scene, gb)),
    }
}

// ---------------------------------------------------------------------------
// 容差带(measured 后冻结,P-09;fail-closed)
// ---------------------------------------------------------------------------

/// M101 容差带单条目(逐档;匹配深度 [`M101_MATCHED_DEPTH`])。
#[derive(Debug, Clone, PartialEq)]
pub struct M101BandEntry {
    /// 档位名(l0_screen_probe / l1_clipmap_volume / l2_spatial_hash /
    /// l3_per_pixel)。
    pub tier: String,
    /// 冻结 golden:该档产物 digest(sha256(rgb‖tier))。
    pub product_digest: String,
    /// 冻结 golden:M96 同深度参照产物 digest。
    pub m96_digest: String,
    /// 冻结容差带(rel_dev 上界 = measured × [`M101_BAND_MARGIN`];禁手写)。
    pub band_rel_dev: f64,
    /// 冻结时实测 rel_dev(provenance)。
    pub measured_rel_dev: f64,
}

/// M101 容差带(`milestones/g9/g9_m101_if_tier_band.json` 的内存形)。
#[derive(Debug, Clone, PartialEq)]
pub struct M101Band {
    /// provenance:冻结时刻 UTC。
    pub frozen_at_utc: String,
    /// provenance:device 名。
    pub device_name: String,
    /// 冻结场景名(M96 冻结 fixture)。
    pub scene: String,
    /// M96 门序消费锚(D2-Q7 机器锚)。
    pub m96_anchor_digest: String,
    /// provenance:SRGB 编码注入 rel_dev 实测(检出阈 =
    /// [`M101_SRGB_REL_DEV_MIN`])。
    pub srgb_encode_rel_dev: f64,
    /// 逐档条目。
    pub entries: Vec<M101BandEntry>,
}

impl M101Band {
    /// 查条目(fail-closed:缺条目 = Err)。
    pub fn entry(&self, tier: &str) -> Result<&M101BandEntry, IfError> {
        self.entries
            .iter()
            .find(|e| e.tier == tier)
            .ok_or_else(|| IfError::DepthBand(format!("容差带缺条目 tier={tier}")))
    }

    /// 比对(fail-closed):双 digest 全等 且 rel_dev ≤ 带;违例逐条列名。
    pub fn check(
        &self,
        tier: &str,
        product_digest: &str,
        m96_digest: &str,
        rel_dev: f64,
    ) -> Result<(), IfError> {
        let e = self.entry(tier)?;
        if product_digest != e.product_digest {
            return Err(IfError::DepthBand(format!(
                "tier={tier} product_digest {product_digest} ≠ golden {}",
                e.product_digest
            )));
        }
        if m96_digest != e.m96_digest {
            return Err(IfError::DepthBand(format!(
                "tier={tier} m96_digest {m96_digest} ≠ golden {}",
                e.m96_digest
            )));
        }
        if rel_dev.is_nan() || rel_dev > e.band_rel_dev {
            return Err(IfError::DepthBand(format!(
                "tier={tier} rel_dev {rel_dev:.6e} 越带(上界 {:.6e})",
                e.band_rel_dev
            )));
        }
        Ok(())
    }

    /// 序列化(手工 JSON;字段序冻结,浮点 `{:e}` 确定性格式)。
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n  \"schema\": \"rurix.g9m101.if_tier_band.v1\",\n");
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
            "  \"freeze_rule\": \"band_rel_dev = measured_rel_dev * {:.1}(规则冻结于 gi::if_tier::M101_BAND_MARGIN;基值 = 冻结批实测,禁手写 P-09)\",\n",
            M101_BAND_MARGIN
        ));
        s.push_str(&format!(
            "  \"matched_depth\": \"{}\",\n",
            M101_MATCHED_DEPTH
        ));
        s.push_str(&format!(
            "  \"m96_golden_spp\": \"{}\",\n",
            M101_M96_GOLDEN_SPP
        ));
        s.push_str(&format!("  \"seed_chain\": \"{}\",\n", M101_SEED));
        s.push_str(&format!(
            "  \"srgb_encode_rel_dev\": \"{:e}\",\n",
            self.srgb_encode_rel_dev
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
    pub fn parse(text: &str) -> Result<M101Band, IfError> {
        let err = |m: &str| IfError::DepthBand(format!("容差带解析: {m}"));
        if !text.contains("\"schema\": \"rurix.g9m101.if_tier_band.v1\"") {
            return Err(err("schema 失配"));
        }
        let get_str = |key: &str| -> Result<String, IfError> {
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
            let field = |key: &str| -> Result<String, IfError> {
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
            if entries.iter().any(|e: &M101BandEntry| e.tier == tier) {
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
            entries.push(M101BandEntry {
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
        Ok(M101Band {
            frozen_at_utc: get_str("frozen_at_utc")?,
            device_name: get_str("device_name")?,
            scene: get_str("scene")?,
            m96_anchor_digest: get_str("m96_anchor_digest")?,
            srgb_encode_rel_dev: get_str("srgb_encode_rel_dev")?
                .parse()
                .map_err(|_| err("srgb_encode_rel_dev 非数值"))?,
            entries,
        })
    }
}

// ---------------------------------------------------------------------------
// device 输入打包(kernel `g9_m101_probe_oct.rx` 头注参数面逐字同源)
// ---------------------------------------------------------------------------

/// probe 位置打包(4 f32/probe:pos 3 + pad)。
pub fn pack_probe_positions(grid: &IfVoxelGrid) -> Vec<f32> {
    let mut out = Vec::with_capacity(grid.probe_count() * 4);
    for i in 0..grid.probe_count() {
        out.extend_from_slice(&grid.probe_pos(i));
        out.push(0.0);
    }
    out
}

/// kernel 参数打包(23 f32;与 `kernels/g9_m101_probe_oct.rx` 头注逐字同源)。
pub fn pack_oct_params(scene: &PtScene, probe_count: u32, mode: OctTraceMode) -> Vec<f32> {
    let l = &scene.light;
    let ln = l.normal();
    let mut p = Vec::with_capacity(23);
    p.push(probe_count as f32);
    p.push(mode.texels() as f32);
    p.push(mode.as_f32());
    p.push(mode.res() as f32);
    p.push(RAY_EPS);
    p.push(scene.t_max);
    p.push(M101_VIS_RANGE);
    p.extend_from_slice(&l.p00);
    p.extend_from_slice(&l.e1);
    p.extend_from_slice(&l.e2);
    p.push(l.area());
    p.extend_from_slice(&l.emission);
    p.extend_from_slice(&ln);
    debug_assert_eq!(p.len(), 23);
    p
}

// ---------------------------------------------------------------------------
// 单测(RXS-0362 锚定;八面体往返界 / 线性域 RED / 档位闭集预算行 / 强制
// 降档 + 禁静默 / 共享内核单实例 / 确定性 / 容差带 fail-closed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gi::fallback_chain as fb;
    use crate::gi::surface_cache;
    use crate::rt::as_manager::{BlasCache, DynamicPolicy};

    fn cornell() -> PtScene {
        let s = path_trace::m96_cornell_scene();
        s.validate().expect("cornell 冻结 fixture 装载");
        s
    }

    //@ spec: RXS-0362
    #[test]
    fn oct_roundtrip_error_within_frozen_bounds() {
        // 编解码往返误差界(确定性方向集全量核验;线性域):fibonacci 球面
        // 4096 方向 × 两分辨率,最大角误差 ≤ 冻结界(界 = 实测 ×2 margin)。
        let mut max_err_irr = 0.0f32;
        let mut max_err_vis = 0.0f32;
        let n = 4096u32;
        let golden = core::f32::consts::PI * (3.0 - (5.0f32).sqrt());
        for i in 0..n {
            let y = 1.0 - (i as f32 + 0.5) * 2.0 / n as f32;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let th = golden * i as f32;
            let d = [r * th.cos(), y, r * th.sin()];
            for (res, acc) in [
                (M101_IRR_RES, &mut max_err_irr),
                (M101_VIS_RES, &mut max_err_vis),
            ] {
                let (ix, iy) = oct::encode_quant(d, res);
                let back = oct::decode_quant(ix, iy, res);
                let dot = (d[0] * back[0] + d[1] * back[1] + d[2] * back[2]).clamp(-1.0, 1.0);
                *acc = acc.max(dot.acos());
            }
        }
        println!("[m101 oct] max err irr8={max_err_irr:.6e} vis16={max_err_vis:.6e} rad");
        assert!(
            max_err_irr <= OCT_ROUNDTRIP_BOUND_IRR,
            "irr8 往返界: {max_err_irr}"
        );
        assert!(
            max_err_vis <= OCT_ROUNDTRIP_BOUND_VIS,
            "vis16 往返界: {max_err_vis}"
        );
        // 零向量收敛 + 编码域基本锚(±Z 半球往返一致)。
        assert_eq!(oct::encode_unit([0.0, 0.0, 0.0]), [0.0, 0.0]);
        let back = oct::decode_unit(oct::encode_unit([0.3, -0.4, 0.5]));
        let n = [0.3f32, -0.4, 0.5];
        let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        let dot = (back[0] * n[0] + back[1] * n[1] + back[2] * n[2]) / l;
        assert!((dot - 1.0).abs() < 1e-5, "未量化往返一致");
    }

    //@ spec: RXS-0362
    #[test]
    fn srgb_encode_injection_red() {
        // 线性域硬契约:apply_domain 线性 = 恒等;SRGB 注入 ⇒ 中灰辐射度
        // 显著抬升(0.5^(1/2.2)≈0.73),编码域错误可检测(负例臂)。
        let lin = apply_domain([0.5, 0.25, 0.75], EncodeDomain::Linear);
        assert_eq!(lin, [0.5, 0.25, 0.75], "线性域恒等");
        let srgb = apply_domain([0.5, 0.25, 0.75], EncodeDomain::SrgbInjected);
        let dev = path_trace::rel_dev(&srgb, &lin).expect("rel_dev");
        assert!(dev > 0.2, "SRGB 注入偏离可检测: {dev}");
    }

    //@ spec: RXS-0362
    #[test]
    fn tier_ladder_closed_set_budget_rows_and_forced_demotion() {
        // 档位闭集:四档定义行全量 + 每档强制含 AS 预算行 + 预算行随档单调。
        for (i, t) in IfTier::ALL.iter().enumerate() {
            let def = tier_def(*t);
            assert_eq!(def.tier, *t);
            assert!(!def.index_kind.is_empty(), "档位定义强制含索引结构字面");
            let _ = (
                def.as_budget.max_blas_builds,
                def.as_budget.max_refits,
                def.as_budget.max_tlas_rebuilds,
            );
            if i > 0 {
                let prev = tier_def(IfTier::ALL[i - 1]);
                assert!(
                    def.as_budget.max_refits <= prev.as_budget.max_refits,
                    "预算行随档单调(高档更紧):{:?} vs {:?}",
                    def.as_budget,
                    prev.as_budget
                );
            }
        }
        // 真实 AsStats 消费面(as_manager 单源):cornell 建一 BLAS ⇒
        // builds=1 ≤ L3 预算 ⇒ 不降档,零记录。
        let scene = cornell();
        let mut cache = BlasCache::new();
        let _id = cache.get_or_build(&scene.positions, &scene.indices, DynamicPolicy::Static);
        let stats = cache.stats();
        assert_eq!(stats.blas_builds, 1);
        let (served, recs) = select_tier(IfTier::L3PerPixel, &stats, true);
        assert_eq!(served, IfTier::L3PerPixel);
        assert!(recs.is_empty());
        audit_demotions(IfTier::L3PerPixel, &stats, served, &recs).expect("无降档审计过");
        // 超预算 ⇒ 强制逐级降档且每步显式记录(refits=99:L3→L2→L1 止,
        // L1 预算 32 仍 < 99 ⇒ 再降 L0;L0 预算 64 < 99 ⇒ 终端止)。
        let hot = AsStats {
            blas_builds: 0,
            refits: 99,
            tlas_rebuilds: 0,
        };
        let (served, recs) = select_tier(IfTier::L3PerPixel, &hot, true);
        assert_eq!(served, IfTier::L0ScreenProbe, "终端降档到 L0");
        assert_eq!(recs.len(), 3, "每级一条记录(L3→L2→L1→L0)");
        assert!(
            recs.iter()
                .all(|r| r.cause == DemotionCause::AsBudgetExceeded)
        );
        assert_eq!(recs[0].from, IfTier::L3PerPixel);
        assert_eq!(recs[0].to, IfTier::L2SpatialHash);
        audit_demotions(IfTier::L3PerPixel, &hot, served, &recs).expect("降档审计过");
        // 双运行逐位一致(选档对同输入确定)。
        let again = select_tier(IfTier::L3PerPixel, &hot, true);
        assert_eq!(&(served, recs.clone()), &again);
        // 静默降档注入(抑记录)⇒ 审计必 fail-closed(超限未显式降档即 RED)。
        let (served_silent, recs_silent) = select_tier(IfTier::L3PerPixel, &hot, false);
        assert!(recs_silent.is_empty());
        let audited = audit_demotions(IfTier::L3PerPixel, &hot, served_silent, &recs_silent);
        assert!(
            matches!(audited, Err(IfError::SilentDemotion(_))),
            "静默降档注入必拒: {audited:?}"
        );
        // 超限未降档注入(服务档伪造为 L3)⇒ 审计必拒。
        let audited2 = audit_demotions(IfTier::L3PerPixel, &hot, IfTier::L3PerPixel, &recs);
        assert!(matches!(audited2, Err(IfError::SilentDemotion(_))));
    }

    //@ spec: RXS-0362
    #[test]
    fn shared_kernel_single_instance_and_tier_determinism() {
        // 共享内核同一函数实例断言(D2-Q5;各档复制实现即 RED)。
        assert!(
            assert_shared_kernel_instance(),
            "四档共享 probe 着色内核同一实例"
        );
        // 档间 golden 归因面:四档评估双跑 digest 各自相等(确定性)。
        let scene = cornell();
        let gb = fb::gbuffer_prepass(&scene);
        let grid = IfVoxelGrid::new();
        let maps = oct_probe_trace_host(&scene, &grid, EncodeDomain::Linear);
        for tier in IfTier::ALL {
            let a = eval_tier(tier, &scene, &gb, &grid, &maps).expect("评估");
            let b = eval_tier(tier, &scene, &gb, &grid, &maps).expect("双跑");
            assert_eq!(a, b, "{} 双跑位级一致", tier.name());
        }
        // L2 缓存计数面:命中+失效非空(同键复用存在)。
        let l2 = eval_l2_spatial_hash(&scene, &gb);
        assert!(
            l2.cache_misses > 0 && l2.cache_hits > 0,
            "L2 缓存命中/失效非空"
        );
    }

    //@ spec: RXS-0362
    #[test]
    fn voxel_grid_rotation_amortization_and_lookup() {
        // 体素网格:4×4×4 = 64 probe;轮换更新摊销确定性(每帧 16,4 帧全
        // 轮换,游标绕行);lookup 界内命中/界外 None。
        let mut grid = IfVoxelGrid::new();
        assert_eq!(grid.probe_count(), 64);
        let mut seen: Vec<u32> = Vec::new();
        for _ in 0..4 {
            seen.extend(grid.rotate_update());
        }
        seen.sort_unstable();
        assert_eq!(seen, (0..64).collect::<Vec<u32>>(), "4 帧全轮换无重无漏");
        assert!(grid.lookup([0.5, 0.5, 0.5]).is_some());
        assert!(grid.lookup([1.5, 0.5, 0.5]).is_none());
        assert!(grid.lookup([-0.1, 0.5, 0.5]).is_none());
        // oct 图尺寸:irradiance 8×8 + visibility 16×16(防漏光优先字面)。
        let scene = cornell();
        let maps = oct_probe_trace_host(&scene, &grid, EncodeDomain::Linear);
        assert_eq!(maps[0].irr.len(), 64);
        assert_eq!(maps[0].vis.len(), 256);
        // visibility 分辨率 > irradiance 分辨率(规范字面)。
        assert!(M101_VIS_RES > M101_IRR_RES);
        // oct 图双跑位级一致。
        let maps2 = oct_probe_trace_host(&scene, &grid, EncodeDomain::Linear);
        assert_eq!(maps, maps2);
        // SRGB 注入 ⇒ oct 图与线性域逐 texel 偏离(编码域 RED 面)。
        let maps_srgb = oct_probe_trace_host(&scene, &grid, EncodeDomain::SrgbInjected);
        assert_ne!(maps, maps_srgb);
    }

    //@ spec: RXS-0362
    #[test]
    fn band_roundtrip_and_fail_closed() {
        let band = M101Band {
            frozen_at_utc: "2026-08-13T00:00:00Z".into(),
            device_name: "testdev".into(),
            scene: "m96_cornell".into(),
            m96_anchor_digest: "a".repeat(64),
            srgb_encode_rel_dev: 0.05,
            entries: vec![M101BandEntry {
                tier: "l3_per_pixel".into(),
                product_digest: "b".repeat(64),
                m96_digest: "a".repeat(64),
                band_rel_dev: 1.0,
                measured_rel_dev: 0.5,
            }],
        };
        let text = band.to_json();
        let parsed = M101Band::parse(&text).expect("解析");
        assert_eq!(parsed, band, "序列化往返全等");
        parsed
            .check("l3_per_pixel", &"b".repeat(64), &"a".repeat(64), 1.0)
            .expect("带内");
        assert!(
            parsed
                .check("l3_per_pixel", &"c".repeat(64), &"a".repeat(64), 0.5)
                .is_err()
        );
        assert!(
            parsed
                .check("l3_per_pixel", &"b".repeat(64), &"a".repeat(64), 1.01)
                .is_err()
        );
        assert!(
            parsed
                .check("nope", &"b".repeat(64), &"a".repeat(64), 0.1)
                .is_err()
        );
        assert!(M101Band::parse("{\"schema\": \"nope\"}").is_err());
    }

    //@ spec: RXS-0362
    #[test]
    fn m96_anchor_consumed_from_m97_band() {
        // 门序消费锚(D2-Q7):M97 冻结带 m96_cornell 深度 2 条目可读。
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../milestones/g9/g9_m97_depth_band.json"
        );
        let text = std::fs::read_to_string(path).expect("M97 冻结带存在");
        let band = surface_cache::DepthBand::parse(&text).expect("M97 带解析");
        let e = band.entry(M101_MATCHED_DEPTH).expect("深度 2 条目");
        assert_eq!(e.m96_digest.len(), 64);
        assert_eq!(M101_MATCHED_DEPTH, 2);
    }
}
