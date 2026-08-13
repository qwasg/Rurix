//! ACES 2.0 view transform 插件（G9.5 M118；RFC-0025 §4.I；RXS-0369 L2）。
//!
//! 参考公式 = AMPAS aces-core `Lib.Academy.OutputTransform.a2.v1` +
//! `Lib.Academy.Tonescale.a2.v1` host 逐字移植，preset =
//! aces-output `d65/rec709/Output.Academy.Rec709-D65_100nit_in_Rec709-D65_BT1886`
//! （limiting = Rec.709/D65，peakLuminance = 100，scale_white = false，eotf = 4
//! 〔BT.1886 gamma 2.4〕——EOTF 段由共享输出编码承担，本插件止于显示线性）。
//!
//! 链：scene-linear Rec.709 → AP0 → AP1 域钳 [0, forward_limit] → Hellwig2022
//! JMh → tonescale（MM + flare）+ chroma compress（reach M 表）→ gamut
//! compress（cusp/upper-hull gamma 表）→ limiting Rec.709 显示线性。
//!
//! 表生成（360 色相 × reach M / gamut cusp / upper-hull gamma）在插件构造时
//! 现算（确定性二分/搜索，与 CTL 同算法同容差）；CTL 已知怪癖逐字保留并注记：
//! - `build_limiting_cusp_corners_tables` 的 `min_index = 1`（字面 1，非 i）；
//! - `find_display_cusp_for_hue` 的 previous 恒 {0,0}（更新行在 CTL 中被注释）；
//! - `build_hue_table` 的 `last_idx` 未初始化读（CTL 解释器零初始化口径 = 0；
//!   首迭代 nominal_idx ≥ 1 时行为相同）；
//! - `lookup_hue_interval` 以 `totalTableSize`(362) 取 uniform 位（字面逐字）。

use super::color::{self, Mat3, Vec3};

// ---- 常量(Lib.Academy.OutputTransform.a2.v1 逐字) ----
const REF_LUMINANCE: f64 = 100.0;
const L_A: f64 = 100.0;
const Y_B: f64 = 20.0;
const SURROUND: [f64; 3] = [0.9, 0.59, 0.9]; // Dim surround
const J_SCALE: f64 = 100.0;
const CAM_NL_Y_REFERENCE: f64 = 100.0;
const CAM_NL_OFFSET: f64 = 0.2713 * CAM_NL_Y_REFERENCE;
const CAM_NL_SCALE: f64 = 4.0 * CAM_NL_Y_REFERENCE;
/// Hellwig2022 model gamma(surround[1]·(1.48+√(Y_b/ref));sqrt 非 const fn,
/// 函数化)。
fn model_gamma() -> f64 {
    SURROUND[1] * (1.48 + (Y_B / REF_LUMINANCE).sqrt())
}
// Chroma compression
const CHROMA_COMPRESS: f64 = 2.4;
const CHROMA_COMPRESS_FACT: f64 = 3.3;
const CHROMA_EXPAND: f64 = 1.3;
const CHROMA_EXPAND_FACT: f64 = 0.69;
const CHROMA_EXPAND_THR: f64 = 0.5;
// Gamut compression
const SMOOTH_CUSPS: f64 = 0.12;
const SMOOTH_M: f64 = 0.27;
const CUSP_MID_BLEND: f64 = 1.3;
const FOCUS_GAIN_BLEND: f64 = 0.3;
const FOCUS_DISTANCE: f64 = 1.35;
const FOCUS_DISTANCE_SCALING: f64 = 1.75;
const COMPRESSION_THRESHOLD: f64 = 0.75;
// Table generation
const TABLE_SIZE: usize = 360;
const TOTAL_TABLE_SIZE: usize = TABLE_SIZE + 2;
const BASE_INDEX: usize = 1;
const HUE_LIMIT: f64 = 360.0;
const CUSP_CORNER_COUNT: usize = 6;
const TOTAL_CORNER_COUNT: usize = CUSP_CORNER_COUNT + 2;
const MAX_SORTED_CORNERS: usize = 2 * CUSP_CORNER_COUNT;
const REACH_CUSP_TOLERANCE: f64 = 1e-3;
const DISPLAY_CUSP_TOLERANCE: f64 = 1e-7;
const GAMMA_MINIMUM: f64 = 0.0;
const GAMMA_MAXIMUM: f64 = 5.0;
const GAMMA_SEARCH_STEP: f64 = 0.4;
const GAMMA_ACCURACY: f64 = 1e-5;
const TEST_COUNT: usize = 5;
const TEST_POSITIONS: [f64; TEST_COUNT] = [0.01, 0.1, 0.5, 0.8, 0.99];

/// CAM16 基色度(CTL `CAM16_PRI` 逐字)。
const CAM16_PRI: color::Primaries = color::Primaries {
    red: [0.8336, 0.1735],
    green: [2.3854, -1.4659],
    blue: [0.087, -0.125],
    white: [0.333, 0.333],
};

// ---------------------------------------------------------------------------
// Tonescale(Lib.Academy.Tonescale.a2.v1 逐字)
// ---------------------------------------------------------------------------

/// CTL `TSParams` 逐字字段集(n/u_2/inverse_limit 为逆变换字段,本波只消费前向
/// 链;字段保留以维持逐字结构,禁删)。
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct TsParams {
    n: f64,
    n_r: f64,
    g: f64,
    t_1: f64,
    c_t: f64,
    s_2: f64,
    u_2: f64,
    m_2: f64,
    forward_limit: f64,
    inverse_limit: f64,
    log_peak: f64,
}

/// CTL `init_TSParams` 逐字。
fn init_ts_params(peak_luminance: f64) -> TsParams {
    let n = peak_luminance;
    let n_r = 100.0;
    let g = 1.15;
    let c = 0.18;
    let c_d = 10.013;
    let w_g = 0.14;
    let t_1 = 0.04;
    let r_hit_min = 128.0;
    let r_hit_max = 896.0;
    let r_hit = r_hit_min + (r_hit_max - r_hit_min) * ((n / n_r).ln() / (10000.0f64 / 100.0).ln());
    let m_0 = n / n_r;
    let m_1 = 0.5 * (m_0 + (m_0 * (m_0 + 4.0 * t_1)).sqrt());
    let u = ((r_hit / m_1) / ((r_hit / m_1) + 1.0)).powf(g);
    let m = m_1 / u;
    let w_i = (n / 100.0).ln() / 2.0f64.ln();
    let c_t = c_d / n_r * (1.0 + w_i * w_g);
    let g_ip = 0.5 * (c_t + (c_t * (c_t + 4.0 * t_1)).sqrt());
    let g_ipp2 = -(m_1 * (g_ip / m).powf(1.0 / g)) / ((g_ip / m).powf(1.0 / g) - 1.0);
    let w_2 = c / g_ipp2;
    let s_2 = w_2 * m_1;
    let u_2 = ((r_hit / m_1) / ((r_hit / m_1) + w_2)).powf(g);
    let m_2 = m_1 / u_2;
    TsParams {
        n,
        n_r,
        g,
        t_1,
        c_t,
        s_2,
        u_2,
        m_2,
        forward_limit: 8.0 * r_hit,
        inverse_limit: n / (u_2 * n_r),
        log_peak: (n / n_r).log10(),
    }
}

/// CTL `tonescale_fwd` 逐字(MM 色调尺度 + flare)。
fn tonescale_fwd(x: f64, p: &TsParams) -> f64 {
    let f = p.m_2 * (x.max(0.0) / (x + p.s_2)).powf(p.g);
    let h = (f * f / (f + p.t_1)).max(0.0);
    h * p.n_r
}

// ---------------------------------------------------------------------------
// CAM(Hellwig2022 JMh;Lib.Academy.OutputTransform 逐字)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct JmhParams {
    rgb_to_cam16_c: Mat3,
    cam16_c_to_rgb: Mat3,
    cone_response_to_aab: Mat3,
    aab_to_cone_response: Mat3,
    f_l_n: f64,
    cz: f64,
    inv_cz: f64,
    a_w_j: f64,
    inv_a_w_j: f64,
}

/// CTL `_post_adaptation_cone_response_compression_fwd`(无符号标量核)。
fn pacrc_fwd_scalar(rc: f64) -> f64 {
    let f_l_y = rc.powf(0.42);
    f_l_y / (CAM_NL_OFFSET + f_l_y)
}

/// CTL `_post_adaptation_cone_response_compression_inv`。
fn pacrc_inv_scalar(ra: f64) -> f64 {
    let ra_lim = ra.min(0.99);
    let f_l_y = (CAM_NL_OFFSET * ra_lim) / (1.0 - ra_lim);
    f_l_y.powf(1.0 / 0.42)
}

fn pacrc_fwd(v: f64) -> f64 {
    pacrc_fwd_scalar(v.abs()).copysign(v)
}

fn pacrc_inv(v: f64) -> f64 {
    pacrc_inv_scalar(v.abs()).copysign(v)
}

fn achromatic_n_to_j(a: f64, cz: f64) -> f64 {
    J_SCALE * a.powf(cz)
}

fn j_to_achromatic_n(j: f64, inv_cz: f64) -> f64 {
    (j * (1.0 / J_SCALE)).powf(inv_cz)
}

fn a_to_y(a: f64, p: &JmhParams) -> f64 {
    let ra = p.a_w_j * a;
    pacrc_inv_scalar(ra) / p.f_l_n
}

fn j_to_y(j: f64, p: &JmhParams) -> f64 {
    a_to_y(j_to_achromatic_n(j.abs(), p.inv_cz), p)
}

fn y_to_j(y: f64, p: &JmhParams) -> f64 {
    let ra = pacrc_fwd_scalar(y.abs() * p.f_l_n);
    let j = achromatic_n_to_j(ra * p.inv_a_w_j, p.cz);
    j.copysign(y)
}

fn rgb_to_aab(rgb: Vec3, p: &JmhParams) -> Vec3 {
    let rgb_m = color::vmul(rgb, &p.rgb_to_cam16_c);
    let rgb_a = [pacrc_fwd(rgb_m[0]), pacrc_fwd(rgb_m[1]), pacrc_fwd(rgb_m[2])];
    color::vmul(rgb_a, &p.cone_response_to_aab)
}

fn aab_to_jmh(aab: Vec3, p: &JmhParams) -> Vec3 {
    if aab[0] <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let j = achromatic_n_to_j(aab[0], p.cz);
    let m = (aab[1] * aab[1] + aab[2] * aab[2]).sqrt();
    let h = wrap_to_360(aab[2].atan2(aab[1]).to_degrees());
    [j, m, h]
}

fn rgb_to_jmh(rgb: Vec3, p: &JmhParams) -> Vec3 {
    aab_to_jmh(rgb_to_aab(rgb, p), p)
}

fn jmh_to_aab(jmh: Vec3, p: &JmhParams) -> Vec3 {
    let h_rad = jmh[2].to_radians();
    let a = j_to_achromatic_n(jmh[0], p.inv_cz);
    [a, jmh[1] * h_rad.cos(), jmh[1] * h_rad.sin()]
}

fn aab_to_rgb(aab: Vec3, p: &JmhParams) -> Vec3 {
    let rgb_a = color::vmul(aab, &p.aab_to_cone_response);
    let rgb_m = [pacrc_inv(rgb_a[0]), pacrc_inv(rgb_a[1]), pacrc_inv(rgb_a[2])];
    color::vmul(rgb_m, &p.cam16_c_to_rgb)
}

fn jmh_to_rgb(jmh: Vec3, p: &JmhParams) -> Vec3 {
    aab_to_rgb(jmh_to_aab(jmh, p), p)
}

/// CTL `init_JMhParams` 逐字。
fn init_jmh_params(prims: &color::Primaries) -> JmhParams {
    let matrix_16 = color::xyz_to_rgb(&CAM16_PRI);
    let base_cone_response_to_aab: Mat3 = [
        [2.0, 1.0, 1.0 / 9.0],
        [1.0, -12.0 / 11.0, 1.0 / 9.0],
        [1.0 / 20.0, 1.0 / 11.0, -2.0 / 9.0],
    ];
    let rgb_to_xyz = color::rgb_to_xyz(prims);
    let xyz_w = color::vmul([REF_LUMINANCE; 3], &rgb_to_xyz);
    let y_w = xyz_w[1];
    let rgb_w = color::vmul(xyz_w, &matrix_16);
    let k = 1.0 / (5.0 * L_A + 1.0);
    let k4 = k * k * k * k;
    let f_l = 0.2 * k4 * (5.0 * L_A) + 0.1 * (1.0 - k4).powi(2) * (5.0 * L_A).powf(1.0 / 3.0);
    let f_l_n = f_l / REF_LUMINANCE;
    let cz = model_gamma();
    let d_rgb = [
        f_l_n * y_w / rgb_w[0],
        f_l_n * y_w / rgb_w[1],
        f_l_n * y_w / rgb_w[2],
    ];
    let rgb_wc = [d_rgb[0] * rgb_w[0], d_rgb[1] * rgb_w[1], d_rgb[2] * rgb_w[2]];
    let rgb_aw = [
        pacrc_fwd(rgb_wc[0]),
        pacrc_fwd(rgb_wc[1]),
        pacrc_fwd(rgb_wc[2]),
    ];
    let mut cone_response_to_aab = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            cone_response_to_aab[i][j] = CAM_NL_SCALE * base_cone_response_to_aab[i][j];
        }
    }
    let a_w = cone_response_to_aab[0][0] * rgb_aw[0]
        + cone_response_to_aab[1][0] * rgb_aw[1]
        + cone_response_to_aab[2][0] * rgb_aw[2];
    let a_w_j = pacrc_fwd_scalar(f_l);
    let matrix_rgb_to_cam16 = color::mmul(&rgb_to_xyz, &matrix_16);
    // scale_matrix_diagonal(I, D_RGB) = diag(D_RGB);mmul(M, diag(v)) 按列缩放。
    let mut rgb_to_cam16_c = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            rgb_to_cam16_c[i][j] = matrix_rgb_to_cam16[i][j] * REF_LUMINANCE * d_rgb[j];
        }
    }
    let mut cone_to_aab_final = [[0.0f64; 3]; 3];
    for i in 0..3 {
        cone_to_aab_final[i][0] = cone_response_to_aab[i][0] / a_w;
        cone_to_aab_final[i][1] = cone_response_to_aab[i][1] * 43.0 * SURROUND[2];
        cone_to_aab_final[i][2] = cone_response_to_aab[i][2] * 43.0 * SURROUND[2];
    }
    JmhParams {
        rgb_to_cam16_c,
        cam16_c_to_rgb: color::minv(&rgb_to_cam16_c),
        cone_response_to_aab: cone_to_aab_final,
        aab_to_cone_response: color::minv(&cone_to_aab_final),
        f_l_n,
        cz,
        inv_cz: 1.0 / cz,
        a_w_j,
        inv_a_w_j: 1.0 / a_w_j,
    }
}

// ---------------------------------------------------------------------------
// 表查找工具(逐字)
// ---------------------------------------------------------------------------

fn wrap_to_360(hue: f64) -> f64 {
    let mut y = hue % 360.0;
    if y < 0.0 {
        y += 360.0;
    }
    y
}

fn hue_position_in_uniform_table(hue: f64, table_size: usize) -> usize {
    (wrap_to_360(hue) / HUE_LIMIT * table_size as f64) as usize
}

fn base_hue_for_position(i_lo: usize, table_size: usize) -> f64 {
    i_lo as f64 * HUE_LIMIT / table_size as f64
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

fn lerp3(a: Vec3, b: Vec3, t: f64) -> Vec3 {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

fn midpoint_i(a: usize, b: usize) -> usize {
    (a + b) / 2
}

/// CTL `reach_M_from_table` 逐字(t = h - base,未归一)。
fn reach_m_from_table(h: f64, table: &[f64; TOTAL_TABLE_SIZE]) -> f64 {
    let base = hue_position_in_uniform_table(h, TABLE_SIZE);
    let t = h - base as f64;
    let i_lo = base + BASE_INDEX;
    lerp(table[i_lo], table[i_lo + 1], t)
}

fn reinhard_remap(scale: f64, nd: f64) -> f64 {
    scale * nd / (1.0 + nd)
}

/// CTL `toe`(前向)逐字。
fn toe(x: f64, limit: f64, k1_in: f64, k2_in: f64) -> f64 {
    if x > limit {
        return x;
    }
    let k2 = k2_in.max(0.001);
    let k1 = (k1_in * k1_in + k2 * k2).sqrt();
    let k3 = (limit + k1) / (limit + k2);
    let minus_b = k3 * x - k1;
    let minus_c = k2 * k3 * x;
    0.5 * (minus_b + (minus_b * minus_b + 4.0 * minus_c).sqrt())
}

/// CTL `chroma_compress_norm` 逐字。
fn chroma_compress_norm(h: f64, chroma_compress_scale: f64) -> f64 {
    let hr = h.to_radians();
    let a = hr.cos();
    let b = hr.sin();
    let cos_hr2 = a * a - b * b;
    let sin_hr2 = 2.0 * a * b;
    let cos_hr3 = 4.0 * a * a * a - 3.0 * a;
    let sin_hr3 = 3.0 * b - 4.0 * b * b * b;
    let m = 11.34072 * a + 16.46899 * cos_hr2 + 7.88380 * cos_hr3 + 14.66441 * b
        - 6.37224 * sin_hr2
        + 9.19364 * sin_hr3
        + 77.12896;
    m * chroma_compress_scale
}

// ---------------------------------------------------------------------------
// ODT 参数与表(逐字)
// ---------------------------------------------------------------------------

struct OdtParams {
    peak_luminance: f64,
    input_params: JmhParams,
    reach_params: JmhParams,
    limit_params: JmhParams,
    ts: TsParams,
    limit_j_max: f64,
    model_gamma_inv: f64,
    table_reach_m: [f64; TOTAL_TABLE_SIZE],
    sat: f64,
    sat_thr: f64,
    compr: f64,
    chroma_compress_scale: f64,
    mid_j: f64,
    focus_dist: f64,
    lower_hull_gamma_inv: f64,
    table_hues: [f64; TOTAL_TABLE_SIZE],
    table_gamut_cusps: [[f64; 3]; TOTAL_TABLE_SIZE],
    table_upper_hull_gamma: [f64; TOTAL_TABLE_SIZE],
    hue_linearity_search_range: [i64; 2],
}

/// CTL `clamp_AP0_to_AP1` 逐字。
fn clamp_ap0_to_ap1(aces: Vec3, lo: f64, hi: f64) -> Vec3 {
    let ap0_to_ap1 = color::mmul(&color::rgb_to_xyz(&color::AP0), &color::xyz_to_rgb(&color::AP1));
    let ap1_to_ap0 = color::mmul(&color::rgb_to_xyz(&color::AP1), &color::xyz_to_rgb(&color::AP0));
    let ap1 = color::vmul(aces, &ap0_to_ap1);
    let clamped = [
        ap1[0].clamp(lo, hi),
        ap1[1].clamp(lo, hi),
        ap1[2].clamp(lo, hi),
    ];
    color::vmul(clamped, &ap1_to_ap0)
}

/// CTL `chroma_compress_fwd` 逐字(invert=false 路径)。
fn chroma_compress_fwd(jmh: Vec3, tonemapped_j: f64, p: &OdtParams) -> Vec3 {
    let [j, m, h] = jmh;
    let mut m_compr = m;
    if m != 0.0 {
        let n_j = tonemapped_j / p.limit_j_max;
        let sn_j = (1.0 - n_j).max(0.0);
        let m_norm = chroma_compress_norm(h, p.chroma_compress_scale);
        let limit = n_j.powf(p.model_gamma_inv) * reach_m_from_table(h, &p.table_reach_m) / m_norm;
        let toe_limit = limit - 0.001;
        let toe_snj_sat = sn_j * p.sat;
        let toe_sqrt_nj_sat_thr = (n_j * n_j + p.sat_thr).sqrt();
        let toe_nj_compr = n_j * p.compr;
        m_compr = m * (tonemapped_j / j).powf(p.model_gamma_inv);
        m_compr /= m_norm;
        m_compr = limit - toe(limit - m_compr, toe_limit, toe_snj_sat, toe_sqrt_nj_sat_thr);
        m_compr = toe(m_compr, limit, toe_nj_compr, sn_j);
        m_compr *= m_norm;
    }
    [tonemapped_j, m_compr, h]
}

/// CTL `tonemap_and_compress_fwd` 逐字。
fn tonemap_and_compress_fwd(jmh: Vec3, p: &OdtParams) -> Vec3 {
    let linear = j_to_y(jmh[0], &p.input_params) / REF_LUMINANCE;
    let tonemapped_y = tonescale_fwd(linear, &p.ts);
    let j_ts = y_to_j(tonemapped_y, &p.input_params);
    chroma_compress_fwd(jmh, j_ts, p)
}

fn compute_compression_vector_slope(
    intersect_j: f64,
    focus_j: f64,
    limit_j_max: f64,
    slope_gain: f64,
) -> f64 {
    let direction_scalar = if intersect_j < focus_j {
        intersect_j
    } else {
        limit_j_max - intersect_j
    };
    direction_scalar * (intersect_j - focus_j) / (focus_j * slope_gain)
}

/// CTL `solve_J_intersect` 逐字。
fn solve_j_intersect(j: f64, m: f64, focus_j: f64, max_j: f64, slope_gain: f64) -> f64 {
    let m_scaled = m / slope_gain;
    let a = m_scaled / focus_j;
    if j < focus_j {
        let b = 1.0 - m_scaled;
        let c = -j;
        let det = b * b - 4.0 * a * c;
        let root = det.sqrt();
        -2.0 * c / (b + root)
    } else {
        let b = -(1.0 + m_scaled + max_j * a);
        let c = max_j * m_scaled + j;
        let det = b * b - 4.0 * a * c;
        let root = det.sqrt();
        -2.0 * c / (b - root)
    }
}

/// CTL `smin_scaled` 逐字。
fn smin_scaled(a: f64, b: f64, scale_reference: f64) -> f64 {
    let s_scaled = SMOOTH_CUSPS * scale_reference;
    let h = (s_scaled - (a - b).abs()).max(0.0) / s_scaled;
    a.min(b) - h * h * h * s_scaled * (1.0 / 6.0)
}

/// CTL `estimate_line_and_boundary_intersection_M` 逐字。
fn estimate_line_and_boundary_intersection_m(
    j_axis_intersect: f64,
    slope: f64,
    inv_gamma: f64,
    j_max: f64,
    m_max: f64,
    j_intersection_reference: f64,
) -> f64 {
    let normalised_j = j_axis_intersect / j_intersection_reference;
    let shifted_intersection = j_intersection_reference * normalised_j.powf(inv_gamma);
    shifted_intersection * m_max / (j_max - slope * m_max)
}

/// CTL `find_gamut_boundary_intersection` 逐字。
fn find_gamut_boundary_intersection(
    jm_cusp: [f64; 2],
    j_max: f64,
    gamma_top_inv: f64,
    gamma_bottom_inv: f64,
    j_intersect_source: f64,
    slope: f64,
    j_intersect_cusp: f64,
) -> f64 {
    let m_boundary_lower = estimate_line_and_boundary_intersection_m(
        j_intersect_source,
        slope,
        gamma_bottom_inv,
        jm_cusp[0],
        jm_cusp[1],
        j_intersect_cusp,
    );
    let f_j_intersect_cusp = j_max - j_intersect_cusp;
    let f_j_intersect_source = j_max - j_intersect_source;
    let f_jm_cusp_j = j_max - jm_cusp[0];
    let m_boundary_upper = estimate_line_and_boundary_intersection_m(
        f_j_intersect_source,
        -slope,
        gamma_top_inv,
        f_jm_cusp_j,
        jm_cusp[1],
        f_j_intersect_cusp,
    );
    smin_scaled(m_boundary_lower, m_boundary_upper, jm_cusp[1])
}

/// CTL `get_focus_gain` 逐字。
fn get_focus_gain(j: f64, analytical_threshold: f64, limit_j_max: f64, focus_dist: f64) -> f64 {
    let mut gain = limit_j_max * focus_dist;
    if j > analytical_threshold {
        let mut gain_adjustment =
            ((limit_j_max - analytical_threshold) / (0.0001f64).max(limit_j_max - j)).log10();
        gain_adjustment = gain_adjustment * gain_adjustment + 1.0;
        gain *= gain_adjustment;
    }
    gain
}

/// CTL `remap_M`(前向)逐字。
fn remap_m(m: f64, gamut_boundary_m: f64, reach_boundary_m: f64) -> f64 {
    let boundary_ratio = gamut_boundary_m / reach_boundary_m;
    let proportion = boundary_ratio.max(COMPRESSION_THRESHOLD);
    let threshold = proportion * gamut_boundary_m;
    if m <= threshold || proportion >= 1.0 {
        return m;
    }
    let m_offset = m - threshold;
    let gamut_offset = gamut_boundary_m - threshold;
    let reach_offset = reach_boundary_m - threshold;
    let scale = reach_offset / ((reach_offset / gamut_offset) - 1.0);
    let nd = m_offset / scale;
    threshold + reinhard_remap(scale, nd)
}

#[derive(Clone, Copy)]
struct HueDependentGamutParams {
    jm_cusp: [f64; 2],
    gamma_bottom_inv: f64,
    gamma_top_inv: f64,
    focus_j: f64,
    analytical_threshold: f64,
}

/// CTL `compress_gamut`(invert=false)逐字。
fn compress_gamut(jmh: Vec3, jx: f64, p: &OdtParams, hdp: &HueDependentGamutParams) -> Vec3 {
    let [j, m, h] = jmh;
    let slope_gain = get_focus_gain(jx, hdp.analytical_threshold, p.limit_j_max, p.focus_dist);
    let j_intersect_source = solve_j_intersect(j, m, hdp.focus_j, p.limit_j_max, slope_gain);
    let gamut_slope = compute_compression_vector_slope(
        j_intersect_source,
        hdp.focus_j,
        p.limit_j_max,
        slope_gain,
    );
    let j_intersect_cusp = solve_j_intersect(
        hdp.jm_cusp[0],
        hdp.jm_cusp[1],
        hdp.focus_j,
        p.limit_j_max,
        slope_gain,
    );
    let gamut_boundary_m = find_gamut_boundary_intersection(
        hdp.jm_cusp,
        p.limit_j_max,
        hdp.gamma_top_inv,
        hdp.gamma_bottom_inv,
        j_intersect_source,
        gamut_slope,
        j_intersect_cusp,
    );
    if gamut_boundary_m <= 0.0 {
        return [j, 0.0, h];
    }
    let reach_max_m = reach_m_from_table(h, &p.table_reach_m);
    let reach_boundary_m = estimate_line_and_boundary_intersection_m(
        j_intersect_source,
        gamut_slope,
        p.model_gamma_inv,
        p.limit_j_max,
        reach_max_m,
        p.limit_j_max,
    );
    let remapped_m = remap_m(m, gamut_boundary_m, reach_boundary_m);
    [j_intersect_source + remapped_m * gamut_slope, remapped_m, h]
}

/// CTL `cusp_from_table` 逐字(整数二分)。
fn cusp_from_table(h: f64, table: &[[f64; 3]; TOTAL_TABLE_SIZE]) -> [f64; 2] {
    let mut low_i = 0usize;
    let mut high_i = BASE_INDEX + TABLE_SIZE;
    let mut i = hue_position_in_uniform_table(h, TABLE_SIZE) + BASE_INDEX;
    while low_i + 1 < high_i {
        if h > table[i][2] {
            low_i = i;
        } else {
            high_i = i;
        }
        i = midpoint_i(low_i, high_i);
    }
    let lo = table[high_i - 1];
    let hi = table[high_i];
    let t = (h - lo[2]) / (hi[2] - lo[2]);
    [lerp(lo[0], hi[0], t), lerp(lo[1], hi[1], t)]
}

/// CTL `lookup_hue_interval` 逐字(注意以 totalTableSize 取 uniform 位)。
fn lookup_hue_interval(h: f64, hue_table: &[f64; TOTAL_TABLE_SIZE], range: [i64; 2]) -> usize {
    let i = BASE_INDEX + hue_position_in_uniform_table(h, TOTAL_TABLE_SIZE);
    let mut i_lo = BASE_INDEX.max((i as i64 + range[0]) as usize);
    let mut i_hi = (BASE_INDEX + TABLE_SIZE).min((i as i64 + range[1]) as usize);
    let mut i = i_lo;
    while i_lo + 1 < i_hi {
        if h > hue_table[i] {
            i_lo = i;
        } else {
            i_hi = i;
        }
        i = midpoint_i(i_lo, i_hi);
    }
    i_hi.max(1)
}

/// CTL `interpolation_weight` 逐字(未归一 = h - h_lo)。
fn interpolation_weight(h: f64, h_lo: f64) -> f64 {
    h - h_lo
}

fn compute_focus_j(cusp_j: f64, mid_j: f64, limit_j_max: f64) -> f64 {
    lerp(cusp_j, mid_j, (CUSP_MID_BLEND - cusp_j / limit_j_max).min(1.0))
}

/// CTL `init_HueDependentGamutParams` 逐字。
fn init_hue_dependent_gamut_params(hue: f64, p: &OdtParams) -> HueDependentGamutParams {
    let gamma_bottom_inv = p.lower_hull_gamma_inv;
    let i_hi = lookup_hue_interval(hue, &p.table_hues, p.hue_linearity_search_range);
    let t = interpolation_weight(hue, p.table_hues[i_hi - 1]);
    let jm_cusp = cusp_from_table(hue, &p.table_gamut_cusps);
    let gamma_top_inv = lerp(
        p.table_upper_hull_gamma[i_hi - 1],
        p.table_upper_hull_gamma[i_hi],
        t,
    );
    let focus_j = compute_focus_j(jm_cusp[0], p.mid_j, p.limit_j_max);
    let analytical_threshold = lerp(jm_cusp[0], p.limit_j_max, FOCUS_GAIN_BLEND);
    HueDependentGamutParams {
        jm_cusp,
        gamma_bottom_inv,
        gamma_top_inv,
        focus_j,
        analytical_threshold,
    }
}

/// CTL `gamut_compress_fwd` 逐字。
fn gamut_compress_fwd(jmh: Vec3, p: &OdtParams) -> Vec3 {
    let [j, m, h] = jmh;
    if j <= 0.0 {
        return [0.0, 0.0, h];
    }
    if m < 0.0 || j > p.limit_j_max {
        return [j, 0.0, h];
    }
    let hdp = init_hue_dependent_gamut_params(h, p);
    compress_gamut(jmh, j, p, &hdp)
}

// ---------------------------------------------------------------------------
// 表生成(逐字)
// ---------------------------------------------------------------------------

fn any_below_zero(rgb: Vec3) -> bool {
    rgb[0] < 0.0 || rgb[1] < 0.0 || rgb[2] < 0.0
}

/// CTL `generate_unit_cube_cusp_corners` 逐字(R,Y,G,C,B,M 序)。
fn generate_unit_cube_cusp_corners(corner: usize) -> Vec3 {
    let mut result = [0.0f64; 3];
    if (corner + 1) % CUSP_CORNER_COUNT < 3 {
        result[0] = 1.0;
    }
    if (corner + 5) % CUSP_CORNER_COUNT < 3 {
        result[1] = 1.0;
    }
    if (corner + 3) % CUSP_CORNER_COUNT < 3 {
        result[2] = 1.0;
    }
    result
}

/// CTL `build_limiting_cusp_corners_tables` 逐字(**含 CTL 怪癖
/// `min_index = 1`(字面 1,非 i)**)。
fn build_limiting_cusp_corners_tables(
    params: &JmhParams,
    peak_luminance: f64,
) -> ([[f64; 3]; TOTAL_CORNER_COUNT], [[f64; 3]; TOTAL_CORNER_COUNT]) {
    let mut rgb_corners = [[0.0f64; 3]; TOTAL_CORNER_COUNT];
    let mut jmh_corners = [[0.0f64; 3]; TOTAL_CORNER_COUNT];
    let mut temp_rgb = [[0.0f64; 3]; CUSP_CORNER_COUNT];
    let mut temp_jmh = [[0.0f64; 3]; CUSP_CORNER_COUNT];
    let mut min_index = 0usize;
    for i in 0..CUSP_CORNER_COUNT {
        temp_rgb[i] = color::svmul(
            peak_luminance / REF_LUMINANCE,
            generate_unit_cube_cusp_corners(i),
        );
        temp_jmh[i] = rgb_to_jmh(temp_rgb[i], params);
        if temp_jmh[i][2] < temp_jmh[min_index][2] {
            min_index = 1; // CTL 逐字:字面 1(非 i)
        }
    }
    for i in 0..CUSP_CORNER_COUNT {
        rgb_corners[i + 1] = temp_rgb[(i + min_index) % CUSP_CORNER_COUNT];
        jmh_corners[i + 1] = temp_jmh[(i + min_index) % CUSP_CORNER_COUNT];
    }
    rgb_corners[0] = rgb_corners[CUSP_CORNER_COUNT];
    rgb_corners[CUSP_CORNER_COUNT + 1] = rgb_corners[1];
    jmh_corners[0] = jmh_corners[CUSP_CORNER_COUNT];
    jmh_corners[CUSP_CORNER_COUNT + 1] = jmh_corners[1];
    jmh_corners[0][2] -= HUE_LIMIT;
    jmh_corners[CUSP_CORNER_COUNT + 1][2] += HUE_LIMIT;
    (rgb_corners, jmh_corners)
}

/// CTL `find_reach_corners_table` 逐字。
fn find_reach_corners_table(params_reach: &JmhParams, p: &OdtParams) -> [[f64; 3]; TOTAL_CORNER_COUNT] {
    let mut temp_jmh = [[0.0f64; 3]; CUSP_CORNER_COUNT];
    let mut jmh_corners = [[0.0f64; 3]; TOTAL_CORNER_COUNT];
    let limit_a = j_to_achromatic_n(p.limit_j_max, params_reach.inv_cz);
    let mut min_index = 0usize;
    for i in 0..CUSP_CORNER_COUNT {
        let rgb_vector = generate_unit_cube_cusp_corners(i);
        let mut lower = 0.0f64;
        let mut upper = p.ts.forward_limit;
        while (upper - lower) > REACH_CUSP_TOLERANCE {
            let test = (lower + upper) / 2.0;
            let test_corner = color::svmul(test, rgb_vector);
            let a = rgb_to_aab(test_corner, params_reach)[0];
            if a < limit_a {
                lower = test;
            } else {
                upper = test;
            }
        }
        temp_jmh[i] = rgb_to_jmh(color::svmul(upper, rgb_vector), params_reach);
        if temp_jmh[i][2] < temp_jmh[min_index][2] {
            min_index = i;
        }
    }
    for i in 0..CUSP_CORNER_COUNT {
        jmh_corners[i + 1] = temp_jmh[(i + min_index) % CUSP_CORNER_COUNT];
    }
    jmh_corners[0] = jmh_corners[CUSP_CORNER_COUNT];
    jmh_corners[CUSP_CORNER_COUNT + 1] = jmh_corners[1];
    jmh_corners[0][2] -= HUE_LIMIT;
    jmh_corners[CUSP_CORNER_COUNT + 1][2] += HUE_LIMIT;
    jmh_corners
}

/// CTL `extract_sorted_cube_hues` 逐字;返回(表, 实际写入数)。两表 12 色相
/// 全异时写满 12 槽;冲突时余槽保持 0.0(CTL 未初始化槽的确定性替代,构建期
/// 由 [`OdtParams::build`] 断言满 12)。
fn extract_sorted_cube_hues(
    reach_jmh: &[[f64; 3]; TOTAL_CORNER_COUNT],
    limit_jmh: &[[f64; 3]; TOTAL_CORNER_COUNT],
) -> ([f64; MAX_SORTED_CORNERS], usize) {
    let mut sorted_hues = [0.0f64; MAX_SORTED_CORNERS];
    let mut idx = 0usize;
    let mut reach_idx = 1usize;
    let mut limit_idx = 1usize;
    while reach_idx < CUSP_CORNER_COUNT + 1 || limit_idx < CUSP_CORNER_COUNT + 1 {
        // 越界侧按 INFINITY 处理(merge 耗尽语义;界内读恒 ≤ CUSP_CORNER_COUNT)。
        let reach_hue = if reach_idx < CUSP_CORNER_COUNT + 1 {
            reach_jmh[reach_idx][2]
        } else {
            f64::INFINITY
        };
        let limit_hue = if limit_idx < CUSP_CORNER_COUNT + 1 {
            limit_jmh[limit_idx][2]
        } else {
            f64::INFINITY
        };
        if reach_hue == limit_hue {
            sorted_hues[idx] = reach_hue;
            reach_idx += 1;
            limit_idx += 1;
        } else if reach_hue < limit_hue {
            sorted_hues[idx] = reach_hue;
            reach_idx += 1;
        } else {
            sorted_hues[idx] = limit_hue;
            limit_idx += 1;
        }
        idx += 1;
    }
    (sorted_hues, idx)
}

/// CTL `round` 逐字(四舍五入向整)。
fn ctl_round(x: f64) -> f64 {
    if x < 0.0 { (x - 0.5).trunc() } else { (x + 0.5).trunc() }
}

/// CTL `build_hue_sample_interval` 逐字。
fn build_hue_sample_interval(
    samples: usize,
    lower: f64,
    upper: f64,
    hue_table: &mut [f64; TOTAL_TABLE_SIZE],
    base: usize,
) {
    let delta = (upper - lower) / samples as f64;
    for (i, slot) in hue_table[base..base + samples].iter_mut().enumerate() {
        *slot = lower + i as f64 * delta;
    }
}

/// CTL `build_hue_table` 逐字(`last_idx` 未初始化读按 CTL 解释器零初始化口径
/// = 0;首迭代 nominal_idx ≥ 1 时行为相同)。
fn build_hue_table(sorted_hues: &[f64; MAX_SORTED_CORNERS]) -> [f64; TOTAL_TABLE_SIZE] {
    let mut hue_table = [0.0f64; TOTAL_TABLE_SIZE];
    let ideal_spacing = TABLE_SIZE as f64 / HUE_LIMIT;
    let mut samples_count = [0i64; 2 * CUSP_CORNER_COUNT + 2];
    let mut last_idx: i64 = 0;
    let mut min_index: i64 = if sorted_hues[0] == 0.0 { 0 } else { 1 };
    for hue_idx in 0..MAX_SORTED_CORNERS {
        let nominal_idx_f = ctl_round(sorted_hues[hue_idx] * ideal_spacing);
        let mut nominal_idx = (nominal_idx_f as i64)
            .max(min_index)
            .min(TABLE_SIZE as i64 - 1);
        if last_idx == nominal_idx {
            if hue_idx > 1 && samples_count[hue_idx - 2] != samples_count[hue_idx - 1] - 1 {
                samples_count[hue_idx - 1] -= 1;
            } else {
                nominal_idx += 1;
            }
        }
        samples_count[hue_idx] = nominal_idx.min(TABLE_SIZE as i64 - 1);
        min_index = nominal_idx;
        last_idx = min_index;
    }
    let mut total_samples = 0usize;
    let mut i = 0usize;
    build_hue_sample_interval(
        samples_count[i] as usize,
        0.0,
        sorted_hues[i],
        &mut hue_table,
        total_samples + 1,
    );
    total_samples += samples_count[i] as usize;
    for i2 in (i + 1)..MAX_SORTED_CORNERS {
        let samples = (samples_count[i2] - samples_count[i2 - 1]) as usize;
        build_hue_sample_interval(
            samples,
            sorted_hues[i2 - 1],
            sorted_hues[i2],
            &mut hue_table,
            total_samples + 1,
        );
        total_samples += samples;
        i = i2;
    }
    build_hue_sample_interval(
        TABLE_SIZE - total_samples,
        sorted_hues[i],
        HUE_LIMIT,
        &mut hue_table,
        total_samples + 1,
    );
    hue_table[0] = hue_table[BASE_INDEX + TABLE_SIZE - 1] - HUE_LIMIT;
    hue_table[BASE_INDEX + TABLE_SIZE] = hue_table[BASE_INDEX] + HUE_LIMIT;
    hue_table
}

/// CTL `find_display_cusp_for_hue` 逐字(previous 恒 {0,0}:更新行在 CTL 中被
/// 注释,故无暖启动)。
fn find_display_cusp_for_hue(
    hue: f64,
    rgb_corners: &[[f64; 3]; TOTAL_CORNER_COUNT],
    jmh_corners: &[[f64; 3]; TOTAL_CORNER_COUNT],
    params: &JmhParams,
) -> [f64; 2] {
    let mut upper_corner = 1usize;
    for (i, corner) in jmh_corners.iter().enumerate().skip(1).take(TOTAL_CORNER_COUNT - 1) {
        if corner[2] > hue {
            upper_corner = i;
            break;
        }
    }
    let lower_corner = upper_corner - 1;
    if jmh_corners[lower_corner][2] == hue {
        return [jmh_corners[lower_corner][0], jmh_corners[lower_corner][1]];
    }
    let cusp_lower = rgb_corners[lower_corner];
    let cusp_upper = rgb_corners[upper_corner];
    let mut lower_t = 0.0f64;
    let mut upper_t = 1.0f64;
    while (upper_t - lower_t) > DISPLAY_CUSP_TOLERANCE {
        let sample_t = (lower_t + upper_t) / 2.0;
        let sample = lerp3(cusp_lower, cusp_upper, sample_t);
        let jmh = rgb_to_jmh(sample, params);
        if jmh[2] < jmh_corners[lower_corner][2] {
            upper_t = sample_t;
        } else if jmh[2] >= jmh_corners[upper_corner][2] {
            lower_t = sample_t;
        } else if jmh[2] > hue {
            upper_t = sample_t;
        } else {
            lower_t = sample_t;
        }
    }
    let sample_t = (lower_t + upper_t) / 2.0;
    let sample = lerp3(cusp_lower, cusp_upper, sample_t);
    let jmh = rgb_to_jmh(sample, params);
    [jmh[0], jmh[1]]
}

/// CTL `build_cusp_table` 逐字(含 smooth_m × smooth_cusps 膨胀)。
fn build_cusp_table(
    hue_table: &[f64; TOTAL_TABLE_SIZE],
    rgb_corners: &[[f64; 3]; TOTAL_CORNER_COUNT],
    jmh_corners: &[[f64; 3]; TOTAL_CORNER_COUNT],
    params: &JmhParams,
) -> [[f64; 3]; TOTAL_TABLE_SIZE] {
    let mut output_table = [[0.0f64; 3]; TOTAL_TABLE_SIZE];
    for i in BASE_INDEX..TOTAL_TABLE_SIZE {
        let hue = hue_table[i];
        let jm = find_display_cusp_for_hue(hue, rgb_corners, jmh_corners, params);
        output_table[i][0] = jm[0];
        output_table[i][1] = jm[1] * (1.0 + SMOOTH_M * SMOOTH_CUSPS);
        output_table[i][2] = hue;
    }
    output_table[0][0] = output_table[TABLE_SIZE][0];
    output_table[0][1] = output_table[TABLE_SIZE][1];
    output_table[0][2] = hue_table[0];
    output_table[BASE_INDEX + TABLE_SIZE][0] = output_table[BASE_INDEX][0];
    output_table[BASE_INDEX + TABLE_SIZE][1] = output_table[BASE_INDEX][1];
    output_table[BASE_INDEX + TABLE_SIZE][2] = hue_table[BASE_INDEX + TABLE_SIZE];
    output_table
}

/// CTL `make_uniform_hue_gamut_table` 逐字。
fn make_uniform_hue_gamut_table(
    reach_params: &JmhParams,
    limit_params: &JmhParams,
    p: &OdtParams,
) -> ([[f64; 3]; TOTAL_TABLE_SIZE], [f64; TOTAL_TABLE_SIZE]) {
    let reach_jmh_corners = find_reach_corners_table(reach_params, p);
    let (limiting_rgb_corners, limiting_jmh_corners) =
        build_limiting_cusp_corners_tables(limit_params, p.peak_luminance);
    let (sorted_hues, count) = extract_sorted_cube_hues(&reach_jmh_corners, &limiting_jmh_corners);
    assert_eq!(
        count, MAX_SORTED_CORNERS,
        "reach/limit 角点色相冲撞(count={count} < {MAX_SORTED_CORNERS}),CTL 未定义面——停手核查"
    );
    let hue_table = build_hue_table(&sorted_hues);
    let cusp_table = build_cusp_table(&hue_table, &limiting_rgb_corners, &limiting_jmh_corners, limit_params);
    (cusp_table, hue_table)
}

/// CTL `make_reach_m_table` 逐字。
fn make_reach_m_table(params: &JmhParams, limit_j_max: f64) -> [f64; TOTAL_TABLE_SIZE] {
    let mut reach_table = [0.0f64; TOTAL_TABLE_SIZE];
    for i in 0..TABLE_SIZE {
        let hue = base_hue_for_position(i, TABLE_SIZE);
        let search_range = 50.0f64;
        let search_maximum = 1300.0f64;
        let mut low = 0.0f64;
        let mut high = low + search_range;
        let mut outside = false;
        while !outside && high < search_maximum {
            let search_jmh = [limit_j_max, high, hue];
            let new_limit_rgb = jmh_to_rgb(search_jmh, params);
            outside = any_below_zero(new_limit_rgb);
            if !outside {
                low = high;
                high += search_range;
            }
        }
        while high - low > 1e-2 {
            let sample_m = (high + low) / 2.0;
            let search_jmh = [limit_j_max, sample_m, hue];
            let new_limit_rgb = jmh_to_rgb(search_jmh, params);
            if any_below_zero(new_limit_rgb) {
                high = sample_m;
            } else {
                low = sample_m;
            }
        }
        reach_table[i + BASE_INDEX] = high;
    }
    reach_table[0] = reach_table[TABLE_SIZE];
    reach_table[BASE_INDEX + TABLE_SIZE] = reach_table[BASE_INDEX];
    reach_table
}

fn outside_hull(rgb: Vec3, max_rgb_test_val: f64) -> bool {
    rgb[0] > max_rgb_test_val || rgb[1] > max_rgb_test_val || rgb[2] > max_rgb_test_val
}

struct GammaTestData {
    test_jmh: [[f64; 3]; TEST_COUNT],
    j_intersect_source: [f64; TEST_COUNT],
    slopes: [f64; TEST_COUNT],
    j_intersect_cusp: [f64; TEST_COUNT],
}

/// CTL `generate_gamma_test_data` 逐字。
fn generate_gamma_test_data(
    jm_cusp: [f64; 2],
    hue: f64,
    limit_j_max: f64,
    mid_j: f64,
    focus_dist: f64,
) -> GammaTestData {
    let analytical_threshold = lerp(jm_cusp[0], limit_j_max, FOCUS_GAIN_BLEND);
    let focus_j = compute_focus_j(jm_cusp[0], mid_j, limit_j_max);
    let mut data = GammaTestData {
        test_jmh: [[0.0; 3]; TEST_COUNT],
        j_intersect_source: [0.0; TEST_COUNT],
        slopes: [0.0; TEST_COUNT],
        j_intersect_cusp: [0.0; TEST_COUNT],
    };
    for (test_index, pos) in TEST_POSITIONS.iter().enumerate() {
        let test_j = lerp(jm_cusp[0], limit_j_max, *pos);
        let slope_gain = get_focus_gain(test_j, analytical_threshold, limit_j_max, focus_dist);
        let j_intersect = solve_j_intersect(test_j, jm_cusp[1], focus_j, limit_j_max, slope_gain);
        let slope =
            compute_compression_vector_slope(j_intersect, focus_j, limit_j_max, slope_gain);
        let j_cusp = solve_j_intersect(jm_cusp[0], jm_cusp[1], focus_j, limit_j_max, slope_gain);
        data.test_jmh[test_index] = [test_j, jm_cusp[1], hue];
        data.j_intersect_source[test_index] = j_intersect;
        data.slopes[test_index] = slope;
        data.j_intersect_cusp[test_index] = j_cusp;
    }
    data
}

/// CTL `evaluate_gamma_fit` 逐字。
fn evaluate_gamma_fit(
    jm_cusp: [f64; 2],
    data: &GammaTestData,
    top_gamma_inv: f64,
    peak_luminance: f64,
    limit_j_max: f64,
    lower_hull_gamma_inv: f64,
    limit_params: &JmhParams,
) -> bool {
    let luminance_limit = peak_luminance / REF_LUMINANCE;
    for test_index in 0..TEST_COUNT {
        let approx_limit_m = find_gamut_boundary_intersection(
            jm_cusp,
            limit_j_max,
            top_gamma_inv,
            lower_hull_gamma_inv,
            data.j_intersect_source[test_index],
            data.slopes[test_index],
            data.j_intersect_cusp[test_index],
        );
        let approx_limit_j = data.j_intersect_source[test_index] + data.slopes[test_index] * approx_limit_m;
        let approximate_jmh = [approx_limit_j, approx_limit_m, data.test_jmh[test_index][2]];
        let new_limit_rgb = jmh_to_rgb(approximate_jmh, limit_params);
        if !outside_hull(new_limit_rgb, luminance_limit) {
            return false;
        }
    }
    true
}

/// CTL `make_upper_hull_gamma_table` 逐字。
fn make_upper_hull_gamma_table(
    gamut_cusp_table: &[[f64; 3]; TOTAL_TABLE_SIZE],
    p: &OdtParams,
) -> [f64; TOTAL_TABLE_SIZE] {
    let mut upper_hull_gamma = [0.0f64; TOTAL_TABLE_SIZE];
    for i in BASE_INDEX..BASE_INDEX + TABLE_SIZE {
        let hue = gamut_cusp_table[i][2];
        let jm_cusp = [gamut_cusp_table[i][0], gamut_cusp_table[i][1]];
        let data = generate_gamma_test_data(jm_cusp, hue, p.limit_j_max, p.mid_j, p.focus_dist);
        let mut low = GAMMA_MINIMUM;
        let mut high = low + GAMMA_SEARCH_STEP;
        let mut outside = false;
        while !outside && high < GAMMA_MAXIMUM {
            let gamma_found = evaluate_gamma_fit(
                jm_cusp,
                &data,
                1.0 / high,
                p.peak_luminance,
                p.limit_j_max,
                p.lower_hull_gamma_inv,
                &p.limit_params,
            );
            if !gamma_found {
                low = high;
                high += GAMMA_SEARCH_STEP;
            } else {
                outside = true;
            }
        }
        while (high - low) > GAMMA_ACCURACY {
            let test_gamma = (high + low) / 2.0;
            let gamma_found = evaluate_gamma_fit(
                jm_cusp,
                &data,
                1.0 / test_gamma,
                p.peak_luminance,
                p.limit_j_max,
                p.lower_hull_gamma_inv,
                &p.limit_params,
            );
            if gamma_found {
                high = test_gamma;
            } else {
                low = test_gamma;
            }
        }
        upper_hull_gamma[i] = 1.0 / high;
    }
    upper_hull_gamma[0] = upper_hull_gamma[TABLE_SIZE];
    upper_hull_gamma[TABLE_SIZE + BASE_INDEX] = upper_hull_gamma[BASE_INDEX];
    upper_hull_gamma
}

/// CTL `determine_hue_linearity_search_range` 逐字(以 totalTableSize 取位)。
fn determine_hue_linearity_search_range(hue_table: &[f64; TOTAL_TABLE_SIZE]) -> [i64; 2] {
    let lower_padding = 0i64;
    let upper_padding = 1i64;
    let mut range = [lower_padding, upper_padding];
    for (i, &hue) in hue_table
        .iter()
        .enumerate()
        .take(BASE_INDEX + TABLE_SIZE)
        .skip(BASE_INDEX)
    {
        let pos = hue_position_in_uniform_table(hue, TOTAL_TABLE_SIZE);
        let delta = i as i64 - pos as i64;
        range[0] = range[0].min(delta + lower_padding);
        range[1] = range[1].max(delta + upper_padding);
    }
    range
}

impl OdtParams {
    /// CTL `init_ODTParams` 逐字(limiting = Rec.709/D65,peak = 100 preset)。
    fn build(peak_luminance: f64, limiting: &color::Primaries) -> Self {
        let input_params = init_jmh_params(&color::AP0);
        let reach_params = init_jmh_params(&color::AP1); // REACH_PRI = AP1
        let limit_params = init_jmh_params(limiting);
        let ts = init_ts_params(peak_luminance);
        let limit_j_max = y_to_j(peak_luminance, &input_params);
        let model_gamma_inv = 1.0 / model_gamma();
        let mut p = OdtParams {
            peak_luminance,
            input_params,
            reach_params,
            limit_params,
            ts,
            limit_j_max,
            model_gamma_inv,
            table_reach_m: [0.0; TOTAL_TABLE_SIZE],
            sat: 0.0,
            sat_thr: 0.0,
            compr: 0.0,
            chroma_compress_scale: 0.0,
            mid_j: 0.0,
            focus_dist: 0.0,
            lower_hull_gamma_inv: 0.0,
            table_hues: [0.0; TOTAL_TABLE_SIZE],
            table_gamut_cusps: [[0.0; 3]; TOTAL_TABLE_SIZE],
            table_upper_hull_gamma: [0.0; TOTAL_TABLE_SIZE],
            hue_linearity_search_range: [0, 1],
        };
        p.table_reach_m = make_reach_m_table(&p.reach_params, p.limit_j_max);
        p.sat = (CHROMA_EXPAND - (CHROMA_EXPAND * CHROMA_EXPAND_FACT) * p.ts.log_peak).max(0.2);
        p.sat_thr = CHROMA_EXPAND_THR / peak_luminance;
        p.compr = CHROMA_COMPRESS + (CHROMA_COMPRESS * CHROMA_COMPRESS_FACT) * p.ts.log_peak;
        p.chroma_compress_scale = (0.03379 * peak_luminance).powf(0.30596) - 0.45135;
        p.mid_j = y_to_j(p.ts.c_t * REF_LUMINANCE, &p.input_params);
        p.focus_dist = FOCUS_DISTANCE + FOCUS_DISTANCE * FOCUS_DISTANCE_SCALING * p.ts.log_peak;
        let lower_hull_gamma = 1.14 + 0.07 * p.ts.log_peak;
        p.lower_hull_gamma_inv = 1.0 / lower_hull_gamma;
        let (cusp_table, hue_table) =
            make_uniform_hue_gamut_table(&p.reach_params, &p.limit_params, &p);
        p.table_gamut_cusps = cusp_table;
        p.table_hues = hue_table;
        p.table_upper_hull_gamma = make_upper_hull_gamma_table(&p.table_gamut_cusps, &p);
        p.hue_linearity_search_range = determine_hue_linearity_search_range(&p.table_hues);
        p
    }
}

/// CTL `outputTransform_fwd` 逐字(AP0 → limiting 基色度显示线性)。
fn output_transform_fwd(aces: Vec3, p: &OdtParams) -> Vec3 {
    let ap0_clamped = clamp_ap0_to_ap1(aces, 0.0, p.ts.forward_limit);
    let jmh = rgb_to_jmh(ap0_clamped, &p.input_params);
    let tonemapped = tonemap_and_compress_fwd(jmh, p);
    let compressed = gamut_compress_fwd(tonemapped, p);
    jmh_to_rgb(compressed, &p.limit_params)
}

/// ACES 2.0 插件(Rec709-D65 100nit BT1886 preset;显示线性输出)。
pub struct Aces20 {
    params: OdtParams,
    rec709_to_ap0: Mat3,
}

impl Default for Aces20 {
    fn default() -> Self {
        Self::new()
    }
}

impl Aces20 {
    /// preset:peakLuminance=100,limiting=Rec.709/D65,scale_white=false。
    pub fn new() -> Self {
        Self {
            params: OdtParams::build(100.0, &color::REC709),
            rec709_to_ap0: color::rgb_to_rgb(&color::REC709, &color::AP0),
        }
    }
}

impl super::view_transform::ViewTransform for Aces20 {
    fn id(&self) -> &'static str {
        "aces20"
    }

    fn display_name(&self) -> &'static str {
        "ACES 2.0 (aces-core Lib.Academy.OutputTransform.a2.v1 host 逐字;preset Rec709-D65_100nit_BT1886)"
    }

    fn to_display_linear(&self, hdr_linear: [f64; 3]) -> [f64; 3] {
        // scene-linear Rec.709 → AP0(与 ACES 1.3 同一输入约定)
        let ap0 = color::vmul(hdr_linear, &self.rec709_to_ap0);
        let rgb = output_transform_fwd(ap0, &self.params);
        // preset:clamp [0, peakLuminance/ref_luminance];limiting==encoding(Rec.709)
        // ⇒ MATRIX_limit_to_display ≈ I;BT.1886 编码归共享输出编码。
        let hi = self.params.peak_luminance / REF_LUMINANCE;
        [
            rgb[0].clamp(0.0, hi),
            rgb[1].clamp(0.0, hi),
            rgb[2].clamp(0.0, hi),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::view_transform::ViewTransform;
    use std::sync::OnceLock;

    fn plugin() -> &'static Aces20 {
        static P: OnceLock<Aces20> = OnceLock::new();
        P.get_or_init(Aces20::new)
    }

    //@ spec: RXS-0369
    #[test]
    fn aces20_known_output_landmarks() {
        let p = plugin();
        // 黑 → 黑(0 或近 0)。
        let b = p.to_display_linear([0.0, 0.0, 0.0]);
        assert!(b.iter().all(|v| v.abs() < 1e-6), "黑: {b:?}");
        // 中性 0.18 灰:ACES 2 设计目标 ≈ 10 nits 邻域(c_d=10.013 ⇒ 显示线性
        // ≈0.1),通道间中性(无 hue 偏)。
        let g = p.to_display_linear([0.18, 0.18, 0.18]);
        assert!(
            (g[0] - 0.10013).abs() < 0.02,
            "0.18 灰 → {g:?}(期望 ≈0.100,ACES2 锚点 10.013 nits)"
        );
        assert!(
            (g[0] - g[1]).abs() < 1e-9 && (g[1] - g[2]).abs() < 1e-9,
            "中性保持: {g:?}"
        );
        // 超白 → ≤1(显示线性参考电平)。
        let w = p.to_display_linear([64.0, 64.0, 64.0]);
        assert!(w.iter().all(|v| *v <= 1.0 && *v > 0.9), "超白: {w:?}");
        // 确定性双调用逐位一致。
        assert_eq!(p.to_display_linear([1.0, 0.5, 0.25]), p.to_display_linear([1.0, 0.5, 0.25]));
        // 单调性。
        let a = p.to_display_linear([0.09, 0.09, 0.09]);
        let m = p.to_display_linear([0.36, 0.36, 0.36]);
        assert!(a[0] < g[0] && g[0] < m[0]);
    }

    //@ spec: RXS-0369
    #[test]
    fn aces20_gamut_compress_bounded() {
        let p = plugin();
        // 超色域输入(Rec.709 无法表达的 AP0 纯色)必须被 gamut compress 收进
        // [0,1];gamut 外不炸(NaN/Inf fail-closed 断言)。
        let case = [4.0, 0.0, 0.0];
        let out = p.to_display_linear(case);
        assert!(
            out.iter().all(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0),
            "gamut compress 有界: {out:?}"
        );
    }
}
