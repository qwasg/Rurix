//! 探针空间 3×3 滤波(报告2 §3.2 P1「探针空间 3×3 滤波,等效大核屏幕滤波」;
//! RFC-0016 章 E1)。
//!
//! 探针网格 3×3 邻域的**归一化相似性加权平均**:权重 = 深度相似性 × 法线
//! 相似性 × 有效性:
//! - 深度相似性:相对深度差 `t = |d_q − d_r| / (tol·max(d_q, d_r, ε))`,
//!   `w_d = 1/(1 + t²)`——深度断裂处权重压止,不跨界扩散(薄几何/遮挡边界
//!   泄漏缓解,报告2 §2/§6);
//! - 法线相似性:`max(n_q·n_r, 0)^8`;
//! - 无效探针权重恒 0;无效中心探针输出保持零 SH。
//!
//! 能量语义:归一化滤波**常数场不变**(不造能/不丢能;内部探针脉冲总能量守
//! 恒,单测锚定);边界截断归一(不复制边界,避免假能量聚集)。

use crate::gi::probe::ProbeGrid;
use crate::gi::sh::ShL1Rgb;

/// 3×3 滤波参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterParams {
    /// 深度相对容差(相对深度差为该值时权重降到 1/2)。
    pub depth_rel_tol: f32,
    /// 法线相似性指数(越大越锐利)。
    pub normal_exp: i32,
}

impl Default for FilterParams {
    fn default() -> Self {
        FilterParams {
            depth_rel_tol: 0.1,
            normal_exp: 8,
        }
    }
}

/// 探针空间 3×3 滤波(输出与 `grid.probes` 等长对齐;`shs` 不等长即 panic)。
pub fn filter_probes_3x3(grid: &ProbeGrid, shs: &[ShL1Rgb], params: &FilterParams) -> Vec<ShL1Rgb> {
    assert_eq!(
        grid.probes.len(),
        shs.len(),
        "filter_probes_3x3: 探针数与 SH 组数不符"
    );
    let (gw, gh) = (grid.w, grid.h);
    let mut out = vec![ShL1Rgb::ZERO; grid.probes.len()];
    for j in 0..gh {
        for i in 0..gw {
            let q = &grid.probes[(j * gw + i) as usize];
            if !q.valid {
                continue;
            }
            let mut acc = ShL1Rgb::ZERO;
            let mut wsum = 0.0f32;
            for dj in -1i32..=1 {
                for di in -1i32..=1 {
                    let (ri, rj) = (i as i32 + di, j as i32 + dj);
                    if ri < 0 || rj < 0 || ri >= gw as i32 || rj >= gh as i32 {
                        continue; // 边界截断:跳过屏外邻元,归一化只计在屏邻元
                    }
                    let idx = (rj as u32 * gw + ri as u32) as usize;
                    let r = &grid.probes[idx];
                    if !r.valid {
                        continue;
                    }
                    let t = (q.depth - r.depth).abs()
                        / (params.depth_rel_tol * q.depth.max(r.depth).max(0.05));
                    let wd = 1.0 / (1.0 + t * t);
                    let wn = q.normal.dot(r.normal).max(0.0).powi(params.normal_exp);
                    let w = wd * wn;
                    acc = acc + shs[idx].scale(w);
                    wsum += w;
                }
            }
            if wsum > 1e-8 {
                out[(j * gw + i) as usize] = acc.scale(1.0 / wsum);
            }
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
    use crate::gi::probe::Probe;
    use crate::rt::bvh::Vec3;

    /// 手工构造 16×16 均匀探针网格(全有效、法线 +y;深度按列分半可配)。
    fn uniform_grid(depth_of: impl Fn(u32) -> f32) -> ProbeGrid {
        let (gw, gh) = (16u32, 16u32);
        let depth_of = &depth_of;
        let probes = (0..gh)
            .flat_map(move |j| {
                (0..gw).map(move |i| Probe {
                    anchor: [i * 4 + 2, j * 4 + 2],
                    pos: Vec3::new(i as f32, 0.0, j as f32),
                    normal: Vec3::new(0.0, 1.0, 0.0),
                    depth: depth_of(i),
                    valid: true,
                })
            })
            .collect();
        ProbeGrid {
            w: gw,
            h: gh,
            cell: 4,
            screen: [64, 64],
            probes,
        }
    }

    /// 全场 DC 通道和(能量守恒计量)。
    fn total_dc(shs: &[ShL1Rgb]) -> [f32; 3] {
        let mut acc = [0.0f32; 3];
        for s in shs {
            for (a, &v) in acc.iter_mut().zip(s.c[0].iter()) {
                *a += v;
            }
        }
        acc
    }

    #[test]
    fn filter_impulse_energy_conserved() {
        // 均匀场景(深度/法线全同 ⇒ 全部权重 1,退化为归一化盒式滤波):
        // 内部脉冲探针 (8,8) 的总能量滤波前后守恒(理论:脉冲对 9 个邻元各贡献
        // 1/9,9×(1/9) = 1,精确守恒;边界截断只影响边缘脉冲,本例取内部)。
        let grid = uniform_grid(|_| 0.5);
        let mut shs = vec![ShL1Rgb::ZERO; grid.probes.len()];
        let impulse = [10.0, 5.0, 2.0];
        shs[(8 * 16 + 8) as usize].c[0] = impulse;
        let before = total_dc(&shs);
        let out = filter_probes_3x3(&grid, &shs, &FilterParams::default());
        let after = total_dc(&out);
        for (ch, (&a, &b)) in after.iter().zip(before.iter()).enumerate() {
            let rel = (a - b).abs() / b;
            assert!(rel < 0.01, "ch{ch} 能量守恒 ±1%:前 {b} 后 {a}");
        }
        // 扩散形态锚定:脉冲 3×3 邻域每探针恰得 impulse/9。
        for dj in -1i32..=1 {
            for di in -1i32..=1 {
                let idx = ((8 + dj) as u32 * 16 + (8 + di) as u32) as usize;
                for (ch, (&got, &imp)) in out[idx].c[0].iter().zip(impulse.iter()).enumerate() {
                    assert!(
                        (got - imp / 9.0).abs() < 1e-5,
                        "({di},{dj}) ch{ch}: {got} vs {}",
                        imp / 9.0
                    );
                }
            }
        }
        // 邻域外探针保持 0。
        assert_eq!(out[(8 * 16 + 10) as usize], ShL1Rgb::ZERO);
    }

    #[test]
    fn filter_depth_break_no_cross_diffusion() {
        // 深度断裂:左半(x<8)深度 0.5,右半 0.9(相对差 0.4/(0.1·0.9) ≈ 4.4
        // ⇒ 跨界权重 ≈ 0.048)。脉冲放在断裂左缘 (7,8):右侧探针不得吸到脉冲。
        let grid = uniform_grid(|i| if i < 8 { 0.5 } else { 0.9 });
        let mut shs = vec![ShL1Rgb::ZERO; grid.probes.len()];
        shs[(8 * 16 + 7) as usize].c[0] = [10.0, 10.0, 10.0];
        let out = filter_probes_3x3(&grid, &shs, &FilterParams::default());
        // 右侧(x≥8)所有探针:扩散量 < 脉冲的 1%。
        // 锚定:q=(8,8) 邻域含 3 个左缘探针(权重 0.0482)与 6 个同侧探针(1)
        // ⇒ 吸入 = 0.0482/6.145 ≈ 0.78% < 1%。
        for j in 0..16u32 {
            for i in 8..16u32 {
                let got = out[(j * 16 + i) as usize].c[0][0];
                assert!(
                    got < 0.01 * 10.0,
                    "({i},{j}) 深度断裂右侧不得吸到脉冲: {got}"
                );
            }
        }
        // 左侧正常扩散:同侧邻元 (6,8) 得 impulse/9。
        let left = out[(8 * 16 + 6) as usize].c[0][0];
        assert!(
            (left - 10.0 / 9.0).abs() < 1e-4,
            "同侧邻元应正常扩散: {left} vs {}",
            10.0 / 9.0
        );
        // 脉冲所在探针 (7,8) 自身保留量:1/(6 + 3·0.0482) ≈ 0.1628 × 10。
        let center = out[(8 * 16 + 7) as usize].c[0][0];
        let expect = 10.0 / (6.0 + 3.0 / (1.0 + (0.4f32 / 0.09).powi(2)));
        assert!(
            (center - expect).abs() < 1e-3,
            "脉冲探针自身保留量锚定: {center} vs {expect}"
        );
    }
}
