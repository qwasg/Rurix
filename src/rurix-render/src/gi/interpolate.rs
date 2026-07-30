//! 全屏逐像素平面加权插值(报告2 §2「平面感知插值,薄几何泄漏缓解」;
//! RFC-0016 章 E1)。
//!
//! 每像素取 2×2 邻域探针,权重 = **双线性 × 平面权重 × 法线权重 × 有效性**,
//! 归一化后混合探针 SH,再对像素法线做余弦卷积求 irradiance:
//! - 平面权重:像素世界位置到探针世界平面 `(pos_q, n_q)` 的有符号距离
//!   `d = (p − pos_q)·n_q`,`w_p = 1/(1 + (d/plane_scale)²)`——薄几何/台阶
//!   深度不连续处,异侧探针平面距离大 ⇒ 权重压止,缓解泄漏(报告2 §2;屏幕
//!   探针类方法的固有泄漏缓解不根除,报告2 §6 如实标注);
//! - 法线权重:`max(n·n_q, 0)⁴`——背向探针排斥,垂直面强衰减;
//! - 无效探针(无几何像素)权重恒 0;四探针权重和 ≈ 0(全无效/全排斥)⇒ 该
//!   像素输出 0(天空像素无间接漫反射)。

use crate::gi::probe::GiCamera;
use crate::gi::probe::{ProbeGrid, back_project};
use crate::gi::sh::{ShL1Rgb, eval_sh_irradiance};
use crate::rt::bvh::Vec3;
use crate::temporal::image::ImageF32;

/// 默认平面权重尺度(世界单位;像素到探针平面距离超过该值即显著衰减。
/// 按场景尺度调;报告2 §2 平面感知项)。
pub const DEFAULT_PLANE_SCALE: f32 = 0.25;

/// 单探针插值权重(归一化后;`index` 为 `grid.probes` 下标)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProbeWeight {
    /// 探针下标(行主序)。
    pub index: usize,
    /// 归一化权重(全零权重组 = 四探针皆不可用)。
    pub weight: f32,
}

/// 2×2 邻域探针权重(顺序 `(i0,j0) (i1,j0) (i0,j1) (i1,j1)`;归一化)。
///
/// 权重和语义:同平面同法线的平坦场景 ⇒ 退化为双线性,和恒 = 1(单测锚定);
/// 全零 ⇒ 调用方应输出 0。
pub fn gather_probe_weights(
    grid: &ProbeGrid,
    pos: Vec3,
    normal: Vec3,
    px: u32,
    py: u32,
    plane_scale: f32,
) -> [ProbeWeight; 4] {
    let cell = grid.cell as f32;
    // 探针 i 锚点像素 = i·cell + cell/2:像素 px 的连续网格坐标。
    let gx = (px as f32 - cell * 0.5) / cell;
    let gy = (py as f32 - cell * 0.5) / cell;
    let i0 = gx.floor() as i32;
    let j0 = gy.floor() as i32;
    let fx = gx - i0 as f32;
    let fy = gy - j0 as f32;
    let clamp_i = |i: i32| i.clamp(0, grid.w as i32 - 1) as u32;
    let clamp_j = |j: i32| j.clamp(0, grid.h as i32 - 1) as u32;
    let coords = [
        (clamp_i(i0), clamp_j(j0), (1.0 - fx) * (1.0 - fy)),
        (clamp_i(i0 + 1), clamp_j(j0), fx * (1.0 - fy)),
        (clamp_i(i0), clamp_j(j0 + 1), (1.0 - fx) * fy),
        (clamp_i(i0 + 1), clamp_j(j0 + 1), fx * fy),
    ];
    let mut out = [ProbeWeight {
        index: 0,
        weight: 0.0,
    }; 4];
    let mut sum = 0.0f32;
    for (dst, &(i, j, wb)) in out.iter_mut().zip(coords.iter()) {
        let idx = (j * grid.w + i) as usize;
        let q = &grid.probes[idx];
        let mut w = if q.valid { wb } else { 0.0 };
        if w > 0.0 {
            // 平面权重:像素世界位置到探针世界平面的有符号距离。
            let d = (pos - q.pos).dot(q.normal);
            let t = d / plane_scale;
            w *= 1.0 / (1.0 + t * t);
            // 法线权重:背向排斥,垂直强衰减。
            w *= normal.dot(q.normal).max(0.0).powi(4);
        }
        *dst = ProbeWeight {
            index: idx,
            weight: w,
        };
        sum += w;
    }
    if sum > 1e-8 {
        for dst in out.iter_mut() {
            dst.weight /= sum;
        }
    }
    out
}

/// 全屏逐像素插值:输出间接漫反射 irradiance(3ch;无效像素 = 0)。
///
/// `shs` 必须与 `grid.probes` 等长对齐(调用契约,不等即 panic)。
pub fn interpolate(
    grid: &ProbeGrid,
    shs: &[ShL1Rgb],
    depth: &ImageF32,
    normals: &ImageF32,
    camera: &GiCamera,
    plane_scale: f32,
) -> ImageF32 {
    assert_eq!(
        grid.probes.len(),
        shs.len(),
        "interpolate: 探针数与 SH 组数不符"
    );
    assert_eq!(depth.c, 1, "interpolate: 深度图必须单通道");
    assert!(
        normals.c == 3 && normals.w == depth.w && normals.h == depth.h,
        "interpolate: 法线图形状与深度图不符"
    );
    let (w, h) = (depth.w, depth.h);
    let mut out = ImageF32::new(w, h, 3);
    for y in 0..h {
        for x in 0..w {
            let d = depth.get(x, y, 0);
            let n = Vec3::from_array(normals.pixel3(x, y));
            if !d.is_finite() || d >= 1.0 || !n.is_finite() || n.length() == 0.0 {
                continue; // 无几何像素:输出 0(天空像素无间接漫反射)
            }
            let n = n.normalize();
            let Some(pos) = back_project(camera, x, y, w, h, d) else {
                continue;
            };
            let weights = gather_probe_weights(grid, pos, n, x, y, plane_scale);
            let mut blend = ShL1Rgb::ZERO;
            let mut wsum = 0.0f32;
            for pw in weights {
                blend = blend + shs[pw.index].scale(pw.weight);
                wsum += pw.weight;
            }
            if wsum <= 1e-8 {
                continue; // 四探针皆不可用:输出 0
            }
            out.set_pixel3(x, y, eval_sh_irradiance(&blend, n));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gi::probe::place_probes;
    use crate::temporal::common::{look_at_rh, perspective_rh_zo};

    /// 测试相机:原点看向 −z(视图 = 恒等),fov 90°。
    fn test_camera() -> GiCamera {
        let proj = perspective_rh_zo(core::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let view = look_at_rh([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]);
        GiCamera::new(proj.mul(&view))
    }

    /// 由视空间 z(负值)手算 NDC 深度(ZO;m22/m23 与 perspective_rh_zo 同式)。
    fn ndc_of_view_z(z: f64) -> f32 {
        let (n, f) = (0.1f64, 100.0f64);
        let m22 = f / (n - f);
        let m23 = n * f / (n - f);
        ((m22 * z + m23) / (-z)) as f32
    }

    #[test]
    fn interpolate_flat_scene_weights_sum_one() {
        let cam = test_camera();
        let depth = ImageF32::from_fn(32, 32, 1, |_, _, _| ndc_of_view_z(-2.0));
        let normals = ImageF32::from_fn(32, 32, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        let grid = place_probes(&depth, &normals, &cam, 4);
        assert_eq!((grid.w, grid.h), (8, 8));
        // 常量 SH 场(含非零线性项):平坦场景四探针权重和 = 1,插值结果恒等于
        // 常量 SH 求值(权重与求值锚定)。
        let mut csh = ShL1Rgb::ZERO;
        csh.c[0] = [1.0, 0.8, 0.6];
        csh.c[1] = [0.1, 0.05, -0.02];
        csh.c[3] = [0.2, 0.1, 0.05];
        let shs = vec![csh; grid.probes.len()];
        let out = interpolate(&grid, &shs, &depth, &normals, &cam, DEFAULT_PLANE_SCALE);
        let expect = eval_sh_irradiance(&csh, Vec3::new(0.0, 0.0, 1.0));
        for y in 0..32 {
            for x in 0..32 {
                let pos = back_project(&cam, x, y, 32, 32, ndc_of_view_z(-2.0)).expect("有效");
                let ws = gather_probe_weights(
                    &grid,
                    pos,
                    Vec3::new(0.0, 0.0, 1.0),
                    x,
                    y,
                    DEFAULT_PLANE_SCALE,
                );
                let wsum: f32 = ws.iter().map(|w| w.weight).sum();
                assert!(
                    (wsum - 1.0).abs() < 1e-6,
                    "({x},{y}) 平坦场景四探针权重和应 = 1,实得 {wsum}"
                );
                let got = out.pixel3(x, y);
                for ch in 0..3 {
                    assert!(
                        (got[ch] - expect[ch]).abs() < 1e-5,
                        "({x},{y}) ch{ch}: {} vs 常量场 {expect:?}",
                        got[ch]
                    );
                }
            }
        }
    }

    #[test]
    fn interpolate_step_depth_leak_guard() {
        let cam = test_camera();
        // 台阶场景:左半 z = −1(前景),右半 z = −5(背景),法线同 +z
        // (平面平行 — 只有平面权重能区分,正是薄几何泄漏考点)。
        let (d_near, d_far) = (ndc_of_view_z(-1.0), ndc_of_view_z(-5.0));
        let depth = ImageF32::from_fn(64, 64, 1, |x, _, _| if x < 32 { d_near } else { d_far });
        let normals = ImageF32::from_fn(64, 64, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        let grid = place_probes(&depth, &normals, &cam, 4);
        assert_eq!(grid.valid_count(), 256);
        // 前景边界像素 x=31:连续网格坐标 gx = (31−2)/4 = 7.25 ⇒ 邻域含前景
        // 探针 7 与背景探针 8。平面权重:d_far = |−1 −(−5)| = 4 ≫ plane_scale。
        let pos = back_project(&cam, 31, 32, 64, 64, d_near).expect("有效");
        let ws = gather_probe_weights(
            &grid,
            pos,
            Vec3::new(0.0, 0.0, 1.0),
            31,
            32,
            DEFAULT_PLANE_SCALE,
        );
        let (mut w_fg, mut w_bg) = (0.0f32, 0.0f32);
        for w in ws {
            let q = &grid.probes[w.index];
            if (q.pos.z + 1.0).abs() < 0.05 {
                w_fg += w.weight;
            } else if (q.pos.z + 5.0).abs() < 0.1 {
                w_bg += w.weight;
            } else {
                panic!("邻域探针不在两台平面上: {:?}", q.pos);
            }
        }
        assert!(w_fg > 0.9, "前景探针应主导:w_fg={w_fg}");
        assert!(
            w_bg / w_fg < 0.05,
            "前景像素不得吸后景探针:w_bg/w_fg = {}",
            w_bg / w_fg
        );
        // 端对端:前景探针 SH = A(dc 1),背景探针 SH = B(dc 0) ⇒ 边界像素
        // irradiance 应 ≈ 前景值(不泄漏)。
        let mut a = ShL1Rgb::ZERO;
        a.c[0] = [1.0, 1.0, 1.0];
        let shs: Vec<ShL1Rgb> = grid
            .probes
            .iter()
            .map(|q| {
                if (q.pos.z + 1.0).abs() < 0.05 {
                    a
                } else {
                    ShL1Rgb::ZERO
                }
            })
            .collect();
        let out = interpolate(&grid, &shs, &depth, &normals, &cam, DEFAULT_PLANE_SCALE);
        let expect_fg = eval_sh_irradiance(&a, Vec3::new(0.0, 0.0, 1.0))[0];
        let got = out.get(31, 32, 0);
        assert!(
            got > 0.95 * expect_fg,
            "边界前景像素应贴近前景值: {got} vs {expect_fg}"
        );
        // 镜像:背景边界像素 x=32 应贴近背景值(0)。
        let got_bg = out.get(32, 32, 0);
        assert!(
            got_bg < 0.05 * expect_fg,
            "边界背景像素应贴近背景值: {got_bg} vs {expect_fg}"
        );
    }
}
