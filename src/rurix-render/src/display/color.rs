//! 色彩数学公共底座（G9.5 M118；RFC-0025 §4.I；spec/display_pipeline.md RXS-0369）。
//!
//! 逐字移植 AMPAS CTL 公共数学面（行向量×矩阵约定，与 CTL `mult_f3_f33` /
//! `mult_f33_f33` 一致：`vmul(v,m)[i] = Σ_j v[j]·m[j][i]`）：
//! - 基色度 → XYZ 矩阵（CTL `RGBtoXYZ_f33`/`XYZtoRGB_f33`，v1.3
//!   `ACESlib.Utilities_Color.ctl` / v2.0 `Lib.Academy.Utilities.ctl` 同式）；
//! - Bradford 色适应（CTL `calculate_cat_matrix`）；
//! - 饱和度调节矩阵（CTL `calc_sat_adjust_matrix`，含末尾 transpose 后的等效形）；
//! - moncurve（v1.3 `ACESlib.Utilities_Color.ctl` / v2.0 `Lib.Academy.DisplayEncoding.ctl`）、
//!   BT.1886、ST 2084（PQ）传递函数；
//! - `ACESlib.Utilities_Color.ctl` 的 `rgb_2_hue` / `rgb_2_yc` / `rgb_2_saturation`。
//!
//! 全部 f64：golden = host 参考公式逐字实现 + measured 冻结（同机双跑位级一致后
//! 冻结 digest）；f64 中间值消除 v1.3/v2.0 矩阵推导的转写误差面（矩阵一律由
//! 已发布基色度现场推导，不转写预计算数值）。本文件严禁 UB：非法构造 typed Err
//! 或确定性饱和，不设未定义行为。

/// 3×3 矩阵（CTL 数组序：`m[row][col]`，行向量左乘）。
pub type Mat3 = [[f64; 3]; 3];
/// RGB/XYZ 三元组。
pub type Vec3 = [f64; 3];

/// 基色度（CIE 1931 xy；CTL `Chromaticities`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Primaries {
    pub red: [f64; 2],
    pub green: [f64; 2],
    pub blue: [f64; 2],
    pub white: [f64; 2],
}

/// ACES2065-1 AP0（SMPTE ST2065-1；ACESlib.Utilities_Color.a1.1.0 逐字）。
pub const AP0: Primaries = Primaries {
    red: [0.73470, 0.26530],
    green: [0.00000, 1.00000],
    blue: [0.00010, -0.07700],
    white: [0.32168, 0.33767],
};
/// ACEScg/渲染空间 AP1（v1.3 与 v2.0 两库同值逐字）。
pub const AP1: Primaries = Primaries {
    red: [0.713, 0.293],
    green: [0.165, 0.830],
    blue: [0.128, 0.044],
    white: [0.32168, 0.33767],
};
/// Rec.709 / sRGB 基色度（D65）。
pub const REC709: Primaries = Primaries {
    red: [0.64000, 0.33000],
    green: [0.30000, 0.60000],
    blue: [0.15000, 0.06000],
    white: [0.31270, 0.32900],
};
/// Rec.2020 基色度（D65；PQ 显示路径目标色域）。
pub const REC2020: Primaries = Primaries {
    red: [0.70800, 0.29200],
    green: [0.17000, 0.79700],
    blue: [0.13100, 0.04600],
    white: [0.31270, 0.32900],
};

/// CTL `mult_f3_f33`：行向量×矩阵，`out[i] = Σ_j v[j]·m[j][i]`。
pub fn vmul(v: Vec3, m: &Mat3) -> Vec3 {
    [
        v[0] * m[0][0] + v[1] * m[1][0] + v[2] * m[2][0],
        v[0] * m[0][1] + v[1] * m[1][1] + v[2] * m[2][1],
        v[0] * m[0][2] + v[1] * m[1][2] + v[2] * m[2][2],
    ]
}

/// CTL `mult_f33_f33`：`c[i][j] = Σ_k a[i][k]·b[k][j]`（`vmul(v, mmul(a,b)) ==
/// vmul(vmul(v,a),b)`）。
pub fn mmul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut c = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            c[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    c
}

/// CTL `mult_f_f3`：标量×向量。
pub fn svmul(s: f64, v: Vec3) -> Vec3 {
    [s * v[0], s * v[1], s * v[2]]
}

/// CTL `invert_f33`（伴随/行列式；与 CTL 标准库同式）。
pub fn minv(m: &Mat3) -> Mat3 {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let inv_det = 1.0 / det;
    let mut r = [[0.0f64; 3]; 3];
    r[0][0] = (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det;
    r[0][1] = (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det;
    r[0][2] = (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det;
    r[1][0] = (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det;
    r[1][1] = (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det;
    r[1][2] = (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det;
    r[2][0] = (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det;
    r[2][1] = (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det;
    r[2][2] = (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det;
    r
}

/// CTL `xyY_2_XYZ`。
pub fn xy_y_to_xyz(xy_y: Vec3) -> Vec3 {
    [
        xy_y[0] * xy_y[2] / xy_y[1].max(1e-10),
        xy_y[2],
        (1.0 - xy_y[0] - xy_y[1]) * xy_y[2] / xy_y[1].max(1e-10),
    ]
}

/// CTL `XYZ_2_xyY`。
pub fn xyz_to_xy_y(xyz: Vec3) -> Vec3 {
    let mut divisor = xyz[0] + xyz[1] + xyz[2];
    if divisor == 0.0 {
        divisor = 1e-10;
    }
    [xyz[0] / divisor, xyz[1] / divisor, xyz[1]]
}

/// CTL `RGBtoXYZ_f33(C, 1.0)` 逐字（由基色度现场推导，不转写预计算数值）。
pub fn rgb_to_xyz(c: &Primaries) -> Mat3 {
    let y = 1.0f64;
    let x = c.white[0] * y / c.white[1];
    let z = (1.0 - c.white[0] - c.white[1]) * y / c.white[1];
    let d = c.red[0] * (c.blue[1] - c.green[1])
        + c.blue[0] * (c.green[1] - c.red[1])
        + c.green[0] * (c.red[1] - c.blue[1]);
    let sr = (x * (c.blue[1] - c.green[1])
        - c.green[0] * (y * (c.blue[1] - 1.0) + c.blue[1] * (x + z))
        + c.blue[0] * (y * (c.green[1] - 1.0) + c.green[1] * (x + z)))
        / d;
    let sg = (x * (c.red[1] - c.blue[1])
        + c.red[0] * (y * (c.blue[1] - 1.0) + c.blue[1] * (x + z))
        - c.blue[0] * (y * (c.red[1] - 1.0) + c.red[1] * (x + z)))
        / d;
    let sb = (x * (c.green[1] - c.red[1])
        - c.red[0] * (y * (c.green[1] - 1.0) + c.green[1] * (x + z))
        + c.green[0] * (y * (c.red[1] - 1.0) + c.red[1] * (x + z)))
        / d;
    [
        [
            sr * c.red[0],
            sr * c.red[1],
            sr * (1.0 - c.red[0] - c.red[1]),
        ],
        [
            sg * c.green[0],
            sg * c.green[1],
            sg * (1.0 - c.green[0] - c.green[1]),
        ],
        [
            sb * c.blue[0],
            sb * c.blue[1],
            sb * (1.0 - c.blue[0] - c.blue[1]),
        ],
    ]
}

/// CTL `XYZtoRGB_f33(C, 1.0)`。
pub fn xyz_to_rgb(c: &Primaries) -> Mat3 {
    minv(&rgb_to_xyz(c))
}

/// CTL `calculate_rgb_to_rgb_matrix`（源基色度 → 目标基色度，Bradford 白点适应）。
pub fn rgb_to_rgb(src: &Primaries, dst: &Primaries) -> Mat3 {
    let src2xyz = rgb_to_xyz(src);
    let cat = cat_bradford(src.white, dst.white);
    let xyz2dst = xyz_to_rgb(dst);
    mmul(&src2xyz, &mmul(&cat, &xyz2dst))
}

/// Bradford 锥响应矩阵（CTL `CONE_RESP_MAT_BRADFORD` 逐字）。
pub const CONE_RESP_BRADFORD: Mat3 = [
    [0.89510, -0.75020, 0.03890],
    [0.26640, 1.71350, -0.06850],
    [-0.16140, 0.03670, 1.02960],
];

/// CTL `calculate_cat_matrix`（默认 Bradford）逐字。
pub fn cat_bradford(src_white: [f64; 2], dst_white: [f64; 2]) -> Mat3 {
    let src_xyz = xy_y_to_xyz([src_white[0], src_white[1], 1.0]);
    let dst_xyz = xy_y_to_xyz([dst_white[0], dst_white[1], 1.0]);
    let src_cone = vmul(src_xyz, &CONE_RESP_BRADFORD);
    let dst_cone = vmul(dst_xyz, &CONE_RESP_BRADFORD);
    let vk: Mat3 = [
        [dst_cone[0] / src_cone[0], 0.0, 0.0],
        [0.0, dst_cone[1] / src_cone[1], 0.0],
        [0.0, 0.0, dst_cone[2] / src_cone[2]],
    ];
    mmul(&CONE_RESP_BRADFORD, &mmul(&vk, &minv(&CONE_RESP_BRADFORD)))
}

/// CTL `calc_sat_adjust_matrix`（含其末尾 transpose 后的等效形：`m[i][j] =
/// (1-sat)·y[i] + sat·δij`，`vmul` 约定下 `out[j] = (1-sat)·(v·y) + sat·v[j]`）。
pub fn sat_adjust_matrix(sat: f64, rgb2y: Vec3) -> Mat3 {
    let mut m = [[0.0f64; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (1.0 - sat) * rgb2y[i] + if i == j { sat } else { 0.0 };
        }
    }
    m
}

/// CTL `moncurve_f`（前向监视器曲线；gamma=2.4/offs=0.055 即 sRGB EOTF 解码）。
pub fn moncurve_fwd(x: f64, gamma: f64, offs: f64) -> f64 {
    let fs = ((gamma - 1.0) / offs) * (offs * gamma / ((gamma - 1.0) * (1.0 + offs))).powf(gamma);
    let xb = offs / (gamma - 1.0);
    if x >= xb {
        ((x + offs) / (1.0 + offs)).powf(gamma)
    } else {
        x * fs
    }
}

/// CTL `moncurve_r` / v2.0 `moncurve_inv`（逆向 = 线性→信号，sRGB 编码）。
pub fn moncurve_inv(y: f64, gamma: f64, offs: f64) -> f64 {
    let yb = (offs * gamma / ((gamma - 1.0) * (1.0 + offs))).powf(gamma);
    let rs = ((gamma - 1.0) / offs).powf(gamma - 1.0) * ((1.0 + offs) / gamma).powf(gamma);
    if y >= yb {
        (1.0 + offs) * y.powf(1.0 / gamma) - offs
    } else {
        y * rs
    }
}

/// CTL `bt1886_f`（EOTF：信号→亮度）。
pub fn bt1886_fwd(v: f64, gamma: f64, lw: f64, lb: f64) -> f64 {
    let a = (lw.powf(1.0 / gamma) - lb.powf(1.0 / gamma)).powf(gamma);
    let b = lb.powf(1.0 / gamma) / (lw.powf(1.0 / gamma) - lb.powf(1.0 / gamma));
    a * (v + b).max(0.0).powf(gamma)
}

/// CTL `bt1886_r` / v2.0 `bt1886_inv`（逆 EOTF：亮度→信号）。
pub fn bt1886_inv(l: f64, gamma: f64, lw: f64, lb: f64) -> f64 {
    let a = (lw.powf(1.0 / gamma) - lb.powf(1.0 / gamma)).powf(gamma);
    let b = lb.powf(1.0 / gamma) / (lw.powf(1.0 / gamma) - lb.powf(1.0 / gamma));
    (l / a).max(0.0).powf(1.0 / gamma) - b
}

// SMPTE ST 2084-2014 常量（v2.0 Lib.Academy.DisplayEncoding.ctl 逐字）。
const PQ_M1: f64 = 0.1593017578125;
const PQ_M2: f64 = 78.84375;
const PQ_C1: f64 = 0.8359375;
const PQ_C2: f64 = 18.8515625;
const PQ_C3: f64 = 18.6875;
const PQ_C: f64 = 10000.0;

/// CTL `Y_to_ST2084`（线性 cd/m² → PQ 码值 [0,1]，全范围）。
pub fn y_to_st2084(c: f64) -> f64 {
    let l = c / PQ_C;
    let lm = l.powf(PQ_M1);
    let n = (PQ_C1 + PQ_C2 * lm) / (1.0 + PQ_C3 * lm);
    n.powf(PQ_M2)
}

/// CTL `ST2084_to_Y`（PQ 码值 → 线性 cd/m²）。
pub fn st2084_to_y(n: f64) -> f64 {
    let np = n.powf(1.0 / PQ_M2);
    let mut l = np - PQ_C1;
    if l < 0.0 {
        l = 0.0;
    }
    l /= PQ_C2 - PQ_C3 * np;
    l = l.powf(1.0 / PQ_M1);
    l * PQ_C
}

/// CTL `rgb_2_hue`（几何色相角，度；中性色 NaN 由调用侧守卫）。
pub fn rgb_2_hue(rgb: Vec3) -> f64 {
    if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
        return f64::NAN;
    }
    let mut hue = (180.0 / std::f64::consts::PI)
        * (3.0f64.sqrt() * (rgb[1] - rgb[2])).atan2(2.0 * rgb[0] - rgb[1] - rgb[2]);
    if hue < 0.0 {
        hue += 360.0;
    }
    hue
}

/// CTL `rgb_2_yc`（亮度代理 YC，ycRadiusWeight=1.75 默认）。
pub fn rgb_2_yc(rgb: Vec3) -> f64 {
    const YC_RADIUS_WEIGHT: f64 = 1.75;
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    let chroma = (b * (b - g) + g * (g - r) + r * (r - b)).sqrt();
    (b + g + r + YC_RADIUS_WEIGHT * chroma) / 3.0
}

/// CTL `rgb_2_saturation`（Transform_Common.a1.0.3 逐字；TINY=1e-10）。
pub fn rgb_2_saturation(rgb: Vec3) -> f64 {
    const TINY: f64 = 1e-10;
    let maxc = rgb[0].max(rgb[1]).max(rgb[2]);
    let minc = rgb[0].min(rgb[1]).min(rgb[2]);
    (maxc.max(TINY) - minc.max(TINY)) / maxc.max(1e-2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    //@ spec: RXS-0369
    #[test]
    fn matrix_inverse_roundtrip() {
        let m = rgb_to_xyz(&AP0);
        let inv = minv(&m);
        let id = mmul(&m, &inv);
        for i in 0..3 {
            for j in 0..3 {
                let expect = if i == j { 1.0 } else { 0.0 };
                assert!(approx(id[i][j], expect, 1e-12), "id[{i}][{j}]={}", id[i][j]);
            }
        }
    }

    //@ spec: RXS-0369
    #[test]
    fn ap0_ap1_published_values() {
        // 与 ACES 公布值互证(AP0↔AP1 转写防漂移锚)。注意约定:公布矩阵为列向
        // 量约定(XYZ = M·RGB);本库为 CTL 行向量约定(out = v·M)⇒ 期望 = 转置。
        let m = mmul(&rgb_to_xyz(&AP0), &xyz_to_rgb(&AP1));
        let expect_published: Mat3 = [
            [1.4514393161, -0.2365107469, -0.2149285693],
            [-0.0765537734, 1.1762296998, -0.0996759264],
            [0.0083161484, -0.0060324498, 0.9977163014],
        ];
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    approx(m[i][j], expect_published[j][i], 1e-9),
                    "ap0→ap1[{i}][{j}]={} expect {}",
                    m[i][j],
                    expect_published[j][i]
                );
            }
        }
        // AP1_RGB2Y 公布值互证(行向量约定下 = rgb_to_xyz(AP1)[i][1])。
        let ap1_xyz = rgb_to_xyz(&AP1);
        assert!(approx(ap1_xyz[0][1], 0.2722287168, 1e-9));
        assert!(approx(ap1_xyz[1][1], 0.6740817658, 1e-9));
        assert!(approx(ap1_xyz[2][1], 0.0536895174, 1e-9));
    }

    //@ spec: RXS-0369
    #[test]
    fn transfer_functions_roundtrip() {
        for v in [0.0, 0.003, 0.18, 0.5, 1.0, 4.0] {
            let s = bt1886_inv(v, 2.4, 1.0, 0.0);
            assert!(approx(bt1886_fwd(s, 2.4, 1.0, 0.0), v, 1e-12));
            let pq = y_to_st2084(v * 100.0);
            assert!(approx(st2084_to_y(pq), v * 100.0, 1e-9));
        }
        // moncurve 分段衔接点 C0 连续(gamma=2.4/offs=0.055;xb = 0.055/1.4)。
        let xb = 0.055 / 1.4;
        let lo = moncurve_fwd(xb * (1.0 - 1e-12), 2.4, 0.055);
        let hi = moncurve_fwd(xb * (1.0 + 1e-12), 2.4, 0.055);
        assert!(approx(lo, hi, 1e-9), "moncurve 衔接: {lo} vs {hi}");
        // moncurve 逆往返。
        for v in [0.0, 0.002, 0.18, 0.5, 1.0] {
            let s = moncurve_inv(v, 2.4, 0.055);
            assert!(approx(moncurve_fwd(s, 2.4, 0.055), v, 1e-12));
        }
    }

    //@ spec: RXS-0369
    #[test]
    fn cat_d60_d65_published() {
        // D60(ACES 白)→D65 Bradford CAT 公布值互证(列向量约定公布值转置后比较)。
        let cat = cat_bradford(AP0.white, REC709.white);
        let expect_published: Mat3 = [
            [0.987224, -0.00611327, 0.0159533],
            [-0.00759836, 1.00186, 0.00533002],
            [0.00307257, -0.00509595, 1.08168],
        ];
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    approx(cat[i][j], expect_published[j][i], 1e-5),
                    "cat[{i}][{j}]={} expect {}",
                    cat[i][j],
                    expect_published[j][i]
                );
            }
        }
    }
}
