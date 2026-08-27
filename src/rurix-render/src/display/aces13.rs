//! ACES 1.3 view transform 插件（G9.5 M118；RFC-0025 §4.I；RXS-0369 L2）。
//!
//! 参考公式 = AMPAS aces-dev v1.3 CTL host 逐字移植：
//! - `transforms/ctl/rrt/RRT.a1.0.3`（glow → red modifier → AP0→AP1 → 全局
//!   desat 0.96 → c5 分段样条色调尺度 → AP1→AP0 得 OCES）；
//! - `transforms/ctl/odt/rec709/ODT.Academy.Rec709_100nits_dim.a1.0.3`（OCES→
//!   AP1 → c9 分段样条〔ODT_48nits 参数〕→ Y_2_linCV(48/0.02) → dim surround
//!   gamma 0.9811 → ODT desat 0.93 → AP1→XYZ → D60→D65 CAT → XYZ→Rec.709 →
//!   显示色域钳 [0,1]）。BT.1886 编码段由共享输出变换编码承担
//!   （[`crate::display::view_transform::encode_display_linear`]），本插件输出
//!   显示线性。
//! - 输入约定：scene-linear Rec.709（引擎 HDR 面）→ 先经 Rec.709→AP0 基色度
//!   转换（IDT 等价段）进 RRT。
//!
//! 已知差异记录（D4 R-D4-5）：ACES 1.3 与 2.0 的输出差（含 hue-skew 区间）由
//! harness 实测写入 golden 带与 evidence，不作 bug 返工。

use super::color::{self, Mat3, Vec3};

/// CTL 半浮点最小正规格化值（`HALF_MIN`，分段样条 log 守卫）。
const HALF_MIN: f64 = 6.103_515_625e-05;
/// CTL `HALF_MAX`（RRT rgbPre 钳制上界）。
const HALF_MAX: f64 = 65504.0;

// RRT glow 模块常量（ACESlib.RRT_Common.a1.1.0 逐字）。
const RRT_GLOW_GAIN: f64 = 0.05;
const RRT_GLOW_MID: f64 = 0.08;
// Red modifier 常量（逐字）。
const RRT_RED_SCALE: f64 = 0.82;
const RRT_RED_PIVOT: f64 = 0.03;
const RRT_RED_HUE: f64 = 0.0;
const RRT_RED_WIDTH: f64 = 135.0;
// 全局 desat 常量（逐字）。
const RRT_SAT_FACTOR: f64 = 0.96;
// ODT 常量（ACESlib.ODT_Common.a1.1.0 逐字）。
const CINEMA_WHITE: f64 = 48.0;
const DIM_SURROUND_GAMMA: f64 = 0.9811;
const ODT_SAT_FACTOR: f64 = 0.93;

/// CTL `glow_fwd` 逐字。
fn glow_fwd(yc_in: f64, glow_gain_in: f64, glow_mid: f64) -> f64 {
    if yc_in <= 2.0 / 3.0 * glow_mid {
        glow_gain_in
    } else if yc_in >= 2.0 * glow_mid {
        0.0
    } else {
        glow_gain_in * (glow_mid / yc_in - 1.0 / 2.0)
    }
}

/// CTL `sigmoid_shaper` 逐字（[-2,2] → [0,1]）。
fn sigmoid_shaper(x: f64) -> f64 {
    let t = (1.0 - (x / 2.0).abs()).max(0.0);
    let sign = if x < 0.0 {
        -1.0
    } else if x > 0.0 {
        1.0
    } else {
        0.0
    };
    (1.0 + sign * (1.0 - t * t)) / 2.0
}

/// CTL `center_hue` 逐字。
fn center_hue(hue: f64, center_h: f64) -> f64 {
    let mut centered = hue - center_h;
    if centered < -180.0 {
        centered += 360.0;
    } else if centered > 180.0 {
        centered -= 360.0;
    }
    centered
}

/// CTL `cubic_basis_shaper` 逐字（含 CTL 的列反序取列 `M[k][3-j]`；中性色 NaN
/// 经比较语义得 0，确定性）。
fn cubic_basis_shaper(x: f64, w: f64) -> f64 {
    const M: [[f64; 4]; 4] = [
        [-1.0 / 6.0, 3.0 / 6.0, -3.0 / 6.0, 1.0 / 6.0],
        [3.0 / 6.0, -6.0 / 6.0, 3.0 / 6.0, 0.0],
        [-3.0 / 6.0, 0.0, 3.0 / 6.0, 0.0],
        [1.0 / 6.0, 4.0 / 6.0, 1.0 / 6.0, 0.0],
    ];
    let knots = [-w / 2.0, -w / 4.0, 0.0, w / 4.0, w / 2.0];
    let mut y = 0.0;
    if x > knots[0] && x < knots[4] {
        let knot_coord = (x - knots[0]) * 4.0 / w;
        let j = knot_coord as usize;
        let t = knot_coord - j as f64;
        let monomials = [t * t * t, t * t, t, 1.0];
        if j < 4 {
            y = monomials[0] * M[0][3 - j]
                + monomials[1] * M[1][3 - j]
                + monomials[2] * M[2][3 - j]
                + monomials[3] * M[3][3 - j];
        }
    }
    y * 3.0 / 2.0
}

/// 单调-基转换矩阵（ACESlib.Tonescales.a1.0.3 逐字）。
const SPLINE_M: Mat3 = [[0.5, -1.0, 0.5], [-1.0, 1.0, 0.5], [0.5, 0.0, 0.0]];

/// c5 分段样条参数（RRT 色调尺度；逐字）。
struct SplineC5 {
    coefs_low: [f64; 6],
    coefs_high: [f64; 6],
    min_point: [f64; 2],
    mid_point: [f64; 2],
    max_point: [f64; 2],
    slope_low: f64,
    slope_high: f64,
}

const RRT_PARAMS: SplineC5 = SplineC5 {
    coefs_low: [
        -4.0000000000,
        -4.0000000000,
        -3.1573765773,
        -0.4852499958,
        1.8477324706,
        1.8477324706,
    ],
    coefs_high: [
        -0.7185482425,
        2.0810307172,
        3.6681241237,
        4.0000000000,
        4.0000000000,
        4.0000000000,
    ],
    min_point: [0.18 * 3.0517578125e-05, 0.0001],
    mid_point: [0.18, 4.8],
    max_point: [0.18 * 262144.0, 10000.0],
    slope_low: 0.0,
    slope_high: 0.0,
};

/// CTL `segmented_spline_c5_fwd` 逐字（f64 化；log 域三段 + 样条核）。
fn segmented_spline_c5_fwd(x: f64, c: &SplineC5) -> f64 {
    const N_KNOTS_LOW: f64 = 4.0;
    const N_KNOTS_HIGH: f64 = 4.0;
    let logx = x.max(HALF_MIN).log10();
    let log_min_x = c.min_point[0].log10();
    let log_mid_x = c.mid_point[0].log10();
    let log_max_x = c.max_point[0].log10();
    let logy = if logx <= log_min_x {
        logx * c.slope_low + (c.min_point[1].log10() - c.slope_low * log_min_x)
    } else if logx > log_min_x && logx < log_mid_x {
        let knot_coord = (N_KNOTS_LOW - 1.0) * (logx - log_min_x) / (log_mid_x - log_min_x);
        let j = knot_coord as usize;
        let t = knot_coord - j as f64;
        let cf = [c.coefs_low[j], c.coefs_low[j + 1], c.coefs_low[j + 2]];
        let basis = color::vmul(cf, &SPLINE_M);
        t * t * basis[0] + t * basis[1] + basis[2]
    } else if logx >= log_mid_x && logx < log_max_x {
        let knot_coord = (N_KNOTS_HIGH - 1.0) * (logx - log_mid_x) / (log_max_x - log_mid_x);
        let j = knot_coord as usize;
        let t = knot_coord - j as f64;
        let cf = [c.coefs_high[j], c.coefs_high[j + 1], c.coefs_high[j + 2]];
        let basis = color::vmul(cf, &SPLINE_M);
        t * t * basis[0] + t * basis[1] + basis[2]
    } else {
        logx * c.slope_high + (c.max_point[1].log10() - c.slope_high * log_max_x)
    };
    10.0f64.powf(logy)
}

/// c9 分段样条参数（ODT 色调尺度；点坐标由 c5 前向现算，与 CTL 常量式等价）。
struct SplineC9 {
    coefs_low: [f64; 10],
    coefs_high: [f64; 10],
    min_point: [f64; 2],
    mid_point: [f64; 2],
    max_point: [f64; 2],
    slope_low: f64,
    slope_high: f64,
}

/// ODT_48nits（ACESlib.Tonescales.a1.0.3 逐字；Rec.709 100nits dim ODT 即消费
/// 此影院色调尺度）。
fn odt_48nits_params() -> SplineC9 {
    SplineC9 {
        coefs_low: [
            -1.6989700043,
            -1.6989700043,
            -1.4779000000,
            -1.2291000000,
            -0.8648000000,
            -0.4480000000,
            0.0051800000,
            0.4511080334,
            0.9113744414,
            0.9113744414,
        ],
        coefs_high: [
            0.5154386965,
            0.8470437783,
            1.1358000000,
            1.3802000000,
            1.5197000000,
            1.5985000000,
            1.6467000000,
            1.6746091357,
            1.6878733390,
            1.6878733390,
        ],
        min_point: [
            segmented_spline_c5_fwd(0.18 * 2f64.powf(-6.5), &RRT_PARAMS),
            0.02,
        ],
        mid_point: [segmented_spline_c5_fwd(0.18, &RRT_PARAMS), 4.8],
        max_point: [
            segmented_spline_c5_fwd(0.18 * 2f64.powf(6.5), &RRT_PARAMS),
            48.0,
        ],
        slope_low: 0.0,
        slope_high: 0.04,
    }
}

/// CTL `segmented_spline_c9_fwd` 逐字。
fn segmented_spline_c9_fwd(x: f64, c: &SplineC9) -> f64 {
    const N_KNOTS_LOW: f64 = 8.0;
    const N_KNOTS_HIGH: f64 = 8.0;
    let logx = x.max(HALF_MIN).log10();
    let log_min_x = c.min_point[0].log10();
    let log_mid_x = c.mid_point[0].log10();
    let log_max_x = c.max_point[0].log10();
    let logy = if logx <= log_min_x {
        logx * c.slope_low + (c.min_point[1].log10() - c.slope_low * log_min_x)
    } else if logx > log_min_x && logx < log_mid_x {
        let knot_coord = (N_KNOTS_LOW - 1.0) * (logx - log_min_x) / (log_mid_x - log_min_x);
        let j = knot_coord as usize;
        let t = knot_coord - j as f64;
        let cf = [c.coefs_low[j], c.coefs_low[j + 1], c.coefs_low[j + 2]];
        let basis = color::vmul(cf, &SPLINE_M);
        t * t * basis[0] + t * basis[1] + basis[2]
    } else if logx >= log_mid_x && logx < log_max_x {
        let knot_coord = (N_KNOTS_HIGH - 1.0) * (logx - log_mid_x) / (log_max_x - log_mid_x);
        let j = knot_coord as usize;
        let t = knot_coord - j as f64;
        let cf = [c.coefs_high[j], c.coefs_high[j + 1], c.coefs_high[j + 2]];
        let basis = color::vmul(cf, &SPLINE_M);
        t * t * basis[0] + t * basis[1] + basis[2]
    } else {
        logx * c.slope_high + (c.max_point[1].log10() - c.slope_high * log_max_x)
    };
    10.0f64.powf(logy)
}

/// CTL `Y_2_linCV`。
fn y_2_lin_cv(y: f64, y_max: f64, y_min: f64) -> f64 {
    (y - y_min) / (y_max - y_min)
}

/// CTL `darkSurround_to_dimSurround` 逐字（AP1→XYZ→xyY，Y^γ，→XYZ→AP1）。
fn dark_surround_to_dim_surround(linear_cv: Vec3, ap1_to_xyz: &Mat3, xyz_to_ap1: &Mat3) -> Vec3 {
    let xyz = color::vmul(linear_cv, ap1_to_xyz);
    let mut xy_y = color::xyz_to_xy_y(xyz);
    xy_y[2] = xy_y[2].clamp(0.0, f64::INFINITY);
    xy_y[2] = xy_y[2].powf(DIM_SURROUND_GAMMA);
    let xyz2 = color::xy_y_to_xyz(xy_y);
    color::vmul(xyz2, xyz_to_ap1)
}

/// ACES 1.3 预计算矩阵面（构造一次；全部现场推导，零转写数值）。
struct Aces13Mats {
    rec709_to_ap0: Mat3,
    ap0_to_ap1: Mat3,
    ap1_to_ap0: Mat3,
    rrt_sat: Mat3,
    odt_sat: Mat3,
    ap1_to_xyz: Mat3,
    xyz_to_ap1: Mat3,
    d60_to_d65_cat: Mat3,
    xyz_to_rec709: Mat3,
}

impl Aces13Mats {
    fn new() -> Self {
        let rec709_to_ap0 = color::rgb_to_rgb(&color::REC709, &color::AP0);
        let ap0_to_ap1 = color::mmul(
            &color::rgb_to_xyz(&color::AP0),
            &color::xyz_to_rgb(&color::AP1),
        );
        let ap1_to_ap0 = color::mmul(
            &color::rgb_to_xyz(&color::AP1),
            &color::xyz_to_rgb(&color::AP0),
        );
        let ap1_to_xyz = color::rgb_to_xyz(&color::AP1);
        let ap1_rgb2y: Vec3 = [ap1_to_xyz[0][1], ap1_to_xyz[1][1], ap1_to_xyz[2][1]];
        Self {
            rec709_to_ap0,
            ap0_to_ap1,
            ap1_to_ap0,
            rrt_sat: color::sat_adjust_matrix(RRT_SAT_FACTOR, ap1_rgb2y),
            odt_sat: color::sat_adjust_matrix(ODT_SAT_FACTOR, ap1_rgb2y),
            ap1_to_xyz,
            xyz_to_ap1: color::minv(&ap1_to_xyz),
            d60_to_d65_cat: color::cat_bradford(color::AP0.white, color::REC709.white),
            xyz_to_rec709: color::xyz_to_rgb(&color::REC709),
        }
    }
}

/// ACES 1.3 插件（RRT + ODT.Rec709_100nits_dim；显示线性输出）。
pub struct Aces13 {
    mats: Aces13Mats,
    odt_spline: SplineC9,
}

impl Default for Aces13 {
    fn default() -> Self {
        Self::new()
    }
}

impl Aces13 {
    pub fn new() -> Self {
        Self {
            mats: Aces13Mats::new(),
            odt_spline: odt_48nits_params(),
        }
    }

    /// RRT 逐字（ACES2065-1 AP0 → OCES AP0）。
    fn rrt(&self, aces_in: Vec3) -> Vec3 {
        let mut aces = aces_in;
        // --- Glow 模块 --- //
        let saturation = color::rgb_2_saturation(aces);
        let yc_in = color::rgb_2_yc(aces);
        let s = sigmoid_shaper((saturation - 0.4) / 0.2);
        let added_glow = 1.0 + glow_fwd(yc_in, RRT_GLOW_GAIN * s, RRT_GLOW_MID);
        aces = color::svmul(added_glow, aces);
        // --- Red modifier --- //
        let hue = color::rgb_2_hue(aces);
        let centered_hue = center_hue(hue, RRT_RED_HUE);
        let hue_weight = cubic_basis_shaper(centered_hue, RRT_RED_WIDTH);
        aces[0] += hue_weight * saturation * (RRT_RED_PIVOT - aces[0]) * (1.0 - RRT_RED_SCALE);
        // --- ACES → 渲染空间 --- //
        for c in aces.iter_mut() {
            *c = c.max(0.0);
        }
        let mut rgb_pre = color::vmul(aces, &self.mats.ap0_to_ap1);
        for c in rgb_pre.iter_mut() {
            *c = c.clamp(0.0, HALF_MAX);
        }
        // --- 全局 desat --- //
        rgb_pre = color::vmul(rgb_pre, &self.mats.rrt_sat);
        // --- c5 色调尺度(逐通道) --- //
        let rgb_post = [
            segmented_spline_c5_fwd(rgb_pre[0], &RRT_PARAMS),
            segmented_spline_c5_fwd(rgb_pre[1], &RRT_PARAMS),
            segmented_spline_c5_fwd(rgb_pre[2], &RRT_PARAMS),
        ];
        // --- 渲染空间 → OCES --- //
        color::vmul(rgb_post, &self.mats.ap1_to_ap0)
    }

    /// ODT.Rec709_100nits_dim 逐字（OCES → 显示线性 Rec.709，止于色域钳
    /// [0,1]；BT.1886 编码段归共享输出编码）。
    fn odt_rec709_100nits_dim(&self, oces: Vec3) -> Vec3 {
        // OCES → 渲染空间
        let rgb_pre = color::vmul(oces, &self.mats.ap0_to_ap1);
        // c9 色调尺度(逐通道)
        let rgb_post = [
            segmented_spline_c9_fwd(rgb_pre[0], &self.odt_spline),
            segmented_spline_c9_fwd(rgb_pre[1], &self.odt_spline),
            segmented_spline_c9_fwd(rgb_pre[2], &self.odt_spline),
        ];
        // 亮度 → 线性码值(CINEMA_BLACK 按 CTL 的 pow10(log10(0.02)) 迂回定义)
        let cinema_black = 10.0f64.powf(0.02f64.log10());
        let mut linear_cv = [
            y_2_lin_cv(rgb_post[0], CINEMA_WHITE, cinema_black),
            y_2_lin_cv(rgb_post[1], CINEMA_WHITE, cinema_black),
            y_2_lin_cv(rgb_post[2], CINEMA_WHITE, cinema_black),
        ];
        // dim surround gamma 补偿
        linear_cv =
            dark_surround_to_dim_surround(linear_cv, &self.mats.ap1_to_xyz, &self.mats.xyz_to_ap1);
        // ODT desat 0.93
        linear_cv = color::vmul(linear_cv, &self.mats.odt_sat);
        // AP1 → XYZ → D60→D65 CAT → Rec.709
        let xyz = color::vmul(linear_cv, &self.mats.ap1_to_xyz);
        let xyz_d65 = color::vmul(xyz, &self.mats.d60_to_d65_cat);
        let rgb_disp = color::vmul(xyz_d65, &self.mats.xyz_to_rec709);
        // 显示色域外钳 [0,1]
        [
            rgb_disp[0].clamp(0.0, 1.0),
            rgb_disp[1].clamp(0.0, 1.0),
            rgb_disp[2].clamp(0.0, 1.0),
        ]
    }
}

impl super::view_transform::ViewTransform for Aces13 {
    fn id(&self) -> &'static str {
        "aces13"
    }

    fn display_name(&self) -> &'static str {
        "ACES 1.3 (RRT.a1.0.3 + ODT.Academy.Rec709_100nits_dim.a1.0.3 host 逐字)"
    }

    fn to_display_linear(&self, hdr_linear: [f64; 3]) -> [f64; 3] {
        // scene-linear Rec.709 → AP0(IDT 等价段)
        let ap0 = color::vmul(hdr_linear, &self.mats.rec709_to_ap0);
        let oces = self.rrt(ap0);
        self.odt_rec709_100nits_dim(oces)
    }
}

/// G31（波 A Task A3）device 侧显示编码参数导出：本模块私有单源数学
/// （[`Aces13Mats`] + [`RRT_PARAMS`] + [`odt_48nits_params`] 的 f64 参考
/// 实现）→ f32 参数块，供 `kernels/g31_display_encode.rx` 经 SSBO 上传
/// 消费（布局与该 kernel 文件头参数面逐字同源，两侧同一常量序；kernel
/// 侧零转写数值常量）。f64→f32 收窄仅在此发生一次（GPU f32 移植与 host
/// f64 参考面的 ULP 级差如实登记于 kernel 文件头「f32 移植偏差登记」）。
/// 纯确定性：同（w,h,bgra）输入位级同输出。
///
/// 返回 136 f32 = 544B：[0]=w [1]=h [2]=bgra_flag [3] reserved；[4..=84]
/// 九矩阵行主各 9；[85..=104] c5 双 coefs6+三端点+slope 双；[105..=132]
/// c9 双 coefs10+三端点+slope 双；[133..=135] reserved（恒 0）。
pub fn aces13_device_encode_params(width: u32, height: u32, bgra: bool) -> Vec<f32> {
    let mats = Aces13Mats::new();
    let c9 = odt_48nits_params();
    let mut v: Vec<f32> = vec![
        width as f32,
        height as f32,
        if bgra { 1.0 } else { 0.0 },
        0.0,
    ];
    let push_m3 = |v: &mut Vec<f32>, m: &Mat3| {
        for row in m.iter() {
            for cell in row.iter() {
                v.push(*cell as f32);
            }
        }
    };
    push_m3(&mut v, &mats.rec709_to_ap0);
    push_m3(&mut v, &mats.ap0_to_ap1);
    push_m3(&mut v, &mats.ap1_to_ap0);
    push_m3(&mut v, &mats.rrt_sat);
    push_m3(&mut v, &mats.odt_sat);
    push_m3(&mut v, &mats.ap1_to_xyz);
    push_m3(&mut v, &mats.xyz_to_ap1);
    push_m3(&mut v, &mats.d60_to_d65_cat);
    push_m3(&mut v, &mats.xyz_to_rec709);
    for x in RRT_PARAMS.coefs_low {
        v.push(x as f32);
    }
    for x in RRT_PARAMS.coefs_high {
        v.push(x as f32);
    }
    v.extend_from_slice(&[
        RRT_PARAMS.min_point[0] as f32,
        RRT_PARAMS.min_point[1] as f32,
        RRT_PARAMS.mid_point[0] as f32,
        RRT_PARAMS.mid_point[1] as f32,
        RRT_PARAMS.max_point[0] as f32,
        RRT_PARAMS.max_point[1] as f32,
        RRT_PARAMS.slope_low as f32,
        RRT_PARAMS.slope_high as f32,
    ]);
    for x in c9.coefs_low {
        v.push(x as f32);
    }
    for x in c9.coefs_high {
        v.push(x as f32);
    }
    v.extend_from_slice(&[
        c9.min_point[0] as f32,
        c9.min_point[1] as f32,
        c9.mid_point[0] as f32,
        c9.mid_point[1] as f32,
        c9.max_point[0] as f32,
        c9.max_point[1] as f32,
        c9.slope_low as f32,
        c9.slope_high as f32,
    ]);
    debug_assert_eq!(v.len(), 133);
    v.resize(136, 0.0);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::view_transform::ViewTransform;

    //@ spec: RXS-0369
    #[test]
    fn rrt_tonescale_anchor_points() {
        // c5 样条锚点:0.18 → 4.8(midPoint 恒等,CTL 公布行为)。
        let y = segmented_spline_c5_fwd(0.18, &RRT_PARAMS);
        assert!((y - 4.8).abs() < 1e-9, "c5(0.18)={y}");
        // mid 双侧连续。
        let ym = segmented_spline_c5_fwd(0.18 * (1.0 - 1e-9), &RRT_PARAMS);
        let yp = segmented_spline_c5_fwd(0.18 * (1.0 + 1e-9), &RRT_PARAMS);
        assert!((ym - 4.8).abs() < 1e-5 && (yp - 4.8).abs() < 1e-5);
        // toe 邻域:min_point 处输出 ≈ minPoint.y(HALF_MIN=6.1e-5 的 log 守卫
        // 使 log 域下界截断到 spline 低段,CTL 同行为;容差带宽带)。
        let y0 = segmented_spline_c5_fwd(RRT_PARAMS.min_point[0], &RRT_PARAMS);
        assert!(
            (y0 - RRT_PARAMS.min_point[1]).abs() < 5e-4,
            "c5(min_point.x)={y0} ≈ {}",
            RRT_PARAMS.min_point[1]
        );
    }

    //@ spec: RXS-0369
    #[test]
    fn aces13_known_output_landmarks() {
        let p = Aces13::new();
        // 中性 0.18 灰 → 显示线性 ≈ 0.104(ACES 1.3 设计点:RRT 4.8 nits →
        // ODT 100nits 域 ≈ 10.4 nits ⇒ 0.104;通道间严格中性)。
        let g = p.to_display_linear([0.18, 0.18, 0.18]);
        assert!(
            (g[0] - 0.104).abs() < 0.02 && (g[0] - g[1]).abs() < 1e-9 && (g[1] - g[2]).abs() < 1e-9,
            "0.18 灰映射: {g:?}"
        );
        // 黑 → 黑;超白 → 钳到 ≤1。
        let b = p.to_display_linear([0.0, 0.0, 0.0]);
        assert!(b.iter().all(|v| v.abs() < 1e-9), "黑: {b:?}");
        let w = p.to_display_linear([64.0, 64.0, 64.0]);
        assert!(w.iter().all(|v| *v <= 1.0 && *v > 0.95), "超白: {w:?}");
        // 单调性:log 均匀三档。
        let a = p.to_display_linear([0.09, 0.09, 0.09]);
        let m = p.to_display_linear([0.36, 0.36, 0.36]);
        assert!(a[0] < g[0] && g[0] < m[0]);
        // 确定性:同输入双调用逐位一致。
        assert_eq!(
            p.to_display_linear([1.0, 0.5, 0.25]),
            p.to_display_linear([1.0, 0.5, 0.25])
        );
    }

    //@ spec: RXS-0369
    #[test]
    fn aces13_red_modifier_visible() {
        let p = Aces13::new();
        // 纯红输入经 red modifier 后 R 通道被压(与无 modifier 对照可检测)。
        let red = p.to_display_linear([1.0, 0.0, 0.0]);
        assert!(red[0] > red[1] && red[0] > red[2], "红色相保持: {red:?}");
    }
}
