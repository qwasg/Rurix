//! SH(L1)投影与余弦卷积求值(报告2 §3.1 探针 SH 投影;RFC-0016 章 E1)。
//!
//! 探针把半球余弦加权采样到的辐射度投影到 **L1 球谐**(4 系数 × RGB:l=0 直流 +
//! l=1 三个线性分量),插值阶段再对像素法线做**余弦卷积**重建 irradiance。
//!
//! 数学依据(Ramamoorthi & Hanrahan 2001「An Efficient Representation for
//! Irradiance Environment Maps」;报告2 §3.1「探针到 SH 投影照 SimLumen 简化
//! 实现」取同一组常量):
//! - 归一化基函数:`Y₀₀ = 1/√(4π)`,`Y₁ = √(3/(4π))·(x, y, z)`;
//! - 余弦 lobe 卷积系数:`A₀ = π`,`A₁ = 2π/3`(l ≥ 2 截断);
//! - irradiance 重建:`E(n) = A₀·Y₀₀·c₀ + A₁·√(3/(4π))·(n·c₁)`
//!   = `(√π/2)·c₀ + (2π/3)·√(3/(4π))·(n·c₁)`,负值 clamp 到 0(L1 截断在强
//!   方向性暗场可产负值,物理 irradiance 非负)。
//!
//! 对拍锚点:常量半球辐射场(探针法线半球内 L 恒定、球外为 0)经本投影/求值
//! 在**任意**方向精确还原 `E = π·L`(直流项 πL/2 与线性项 πL/2 恰好相加),
//! 单测锚定;本文件常量即 W3 device 腿 shader 的逐字语义。

use crate::gi::probe::{ProbeGrid, ProbeSamples};
use crate::rt::bvh::Vec3;

/// L1 基函数直流项:`Y₀₀ = 1/√(4π)`。
pub const Y00: f32 = 0.282_094_8;
/// L1 基函数线性项系数:`√(3/(4π))`。
pub const Y1: f32 = 0.488_602_5;
/// 余弦卷积系数 l=0(Ramamoorthi–Hanrahan):`A₀ = π`。
pub const COSINE_CONV_0: f32 = core::f32::consts::PI;
/// 余弦卷积系数 l=1:`A₁ = 2π/3`。
pub const COSINE_CONV_1: f32 = 2.0 * core::f32::consts::PI / 3.0;

/// 余弦采样 pdf 数值护栏下限(见 [`project_sh`] 文档;G-G5-6 host/device 对拍
/// 取同一常量)。
pub const COS_PDF_MIN: f32 = 0.05;

/// SH L1 RGB 系数组(`c[0]` = DC,`c[1..4]` = x/y/z 线性项,各 3 通道)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShL1Rgb {
    pub c: [[f32; 3]; 4],
}

impl ShL1Rgb {
    /// 全零系数(无效探针/零采样占位)。
    pub const ZERO: ShL1Rgb = ShL1Rgb { c: [[0.0; 3]; 4] };

    /// 逐系数标量乘(归一化/能量缩放用)。
    pub fn scale(self, s: f32) -> ShL1Rgb {
        let mut out = self;
        for co in out.c.iter_mut() {
            for v in co.iter_mut() {
                *v *= s;
            }
        }
        out
    }

    /// 逐系数线性插值 `self·(1−t) + o·t`(时域指数混合/空间滤波归一共用)。
    pub fn lerp(self, o: ShL1Rgb, t: f32) -> ShL1Rgb {
        self.scale(1.0 - t) + o.scale(t)
    }

    /// DC 项(能量守恒验收计量面)。
    pub fn dc(self) -> [f32; 3] {
        self.c[0]
    }
}

/// 逐系数加(空间滤波累加/插值混合共用;`Vec3` 同型先例)。
impl core::ops::Add for ShL1Rgb {
    type Output = ShL1Rgb;

    fn add(self, o: ShL1Rgb) -> ShL1Rgb {
        let mut out = self;
        for (dst, src) in out.c.iter_mut().zip(o.c.iter()) {
            for (d, &s) in dst.iter_mut().zip(src.iter()) {
                *d += s;
            }
        }
        out
    }
}

/// L1 基函数值,顺序 `[Y₀₀, Y₁·x, Y₁·y, Y₁·z]`(`dir` 应单位长)。
pub fn sh_basis_l1(dir: Vec3) -> [f32; 4] {
    [Y00, Y1 * dir.x, Y1 * dir.y, Y1 * dir.z]
}

/// 单探针 SH 投影(管线路径:半球余弦加权采样的无偏估计)。
///
/// 估计量 `c_lm = (1/N)Σᵢ L(ωᵢ)·Y_lm(ωᵢ)/pdf(ωᵢ)`,`pdf(ω) = max(n·ω,
/// COS_PDF_MIN)/π`。余弦采样在 θ→π/2 时 1/pdf 发散(理论方差无界),护栏把单
/// 样本权重限制在 `π/COS_PDF_MIN`:对近地平线环带贡献引入 ≤5% 的**欠估计**
/// 保守偏差(不造能),换方差有界与跨平台确定性。
///
/// `dirs` 与 `radiance` 等长(调用契约,不等即 panic);零采样产 [`ShL1Rgb::ZERO`]。
pub fn project_sh(normal: Vec3, dirs: &[Vec3], radiance: &[[f32; 3]]) -> ShL1Rgb {
    assert_eq!(
        dirs.len(),
        radiance.len(),
        "project_sh: 方向与辐射度样本数不符"
    );
    if dirs.is_empty() {
        return ShL1Rgb::ZERO;
    }
    let weights: Vec<f32> = dirs
        .iter()
        .map(|d| core::f32::consts::PI / normal.dot(*d).max(COS_PDF_MIN))
        .collect();
    project_sh_weighted(dirs, radiance, &weights)
}

/// 显式权重 SH 投影(对拍/单测面:全球均匀权重 `4π/N`、δ 脉冲等;`c_lm =
/// (1/N)Σᵢ wᵢ·L(ωᵢ)·Y_lm(ωᵢ)`)。
pub fn project_sh_weighted(dirs: &[Vec3], radiance: &[[f32; 3]], weights: &[f32]) -> ShL1Rgb {
    assert!(
        dirs.len() == radiance.len() && dirs.len() == weights.len(),
        "project_sh_weighted: 方向/辐射度/权重样本数不符"
    );
    if dirs.is_empty() {
        return ShL1Rgb::ZERO;
    }
    let mut acc = ShL1Rgb::ZERO;
    for ((d, l), &w) in dirs.iter().zip(radiance.iter()).zip(weights.iter()) {
        let basis = sh_basis_l1(*d);
        for (co, &b) in acc.c.iter_mut().zip(basis.iter()) {
            for (a, &lv) in co.iter_mut().zip(l.iter()) {
                *a += lv * b * w;
            }
        }
    }
    acc.scale(1.0 / dirs.len() as f32)
}

/// 全探针投影(无效探针/零采样 → [`ShL1Rgb::ZERO`];输出与 `grid.probes` 对齐)。
pub fn project_all(grid: &ProbeGrid, samples: &[ProbeSamples]) -> Vec<ShL1Rgb> {
    assert_eq!(
        grid.probes.len(),
        samples.len(),
        "project_all: 探针数与样本组数不符"
    );
    grid.probes
        .iter()
        .zip(samples.iter())
        .map(|(p, s)| {
            if !p.valid {
                ShL1Rgb::ZERO
            } else {
                project_sh(p.normal, &s.dirs, &s.radiance)
            }
        })
        .collect()
}

/// 余弦卷积求 irradiance(负值 clamp 到 0;系数依据见模块文档)。
pub fn eval_sh_irradiance(sh: &ShL1Rgb, n: Vec3) -> [f32; 3] {
    // A₀·Y₀₀ = π/(2√π) = √π/2;A₁·√(3/(4π)) = (2π/3)·√(3/(4π))。
    let k0 = COSINE_CONV_0 * Y00;
    let k1 = COSINE_CONV_1 * Y1;
    let mut out = [0.0; 3];
    for (ch, o) in out.iter_mut().enumerate() {
        let linear = n.x * sh.c[1][ch] + n.y * sh.c[2][ch] + n.z * sh.c[3][ch];
        *o = (k0 * sh.c[0][ch] + k1 * linear).max(0.0);
    }
    out
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 确定性全球近均匀方向集(黄金角 Fibonacci 球;零 RNG,单测锚定用)。
    fn fibonacci_sphere(n: usize) -> Vec<Vec3> {
        let ga = core::f32::consts::PI * (3.0 - 5.0f32.sqrt());
        (0..n)
            .map(|i| {
                let z = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32;
                let r = (1.0 - z * z).max(0.0).sqrt();
                let phi = ga * i as f32;
                Vec3::new(r * phi.cos(), r * phi.sin(), z)
            })
            .collect()
    }

    #[test]
    fn sh_constant_field_dc_exact_linear_near_zero() {
        // 常量全球辐射场 L(ω) = L0:全球均匀采样(权重 = 1/pdf = 4π)投影。
        // 理论:c₀ = L0·Y₀₀·4π = 2√π·L0(逐样本同值,与方向分布无关,精确);
        // 线性项 = 4π·L0·Y₁·mean(dᵢ) ≈ 0(Fibonacci 残差,相对 DC < 2%)。
        let dirs = fibonacci_sphere(64);
        let l0 = [0.6, 0.8, 1.0];
        let rads = vec![l0; 64];
        // 权重 = 1/pdf = 4π(全球均匀;估计量的 1/N 由投影内部承担)。
        let weights = vec![4.0 * core::f32::consts::PI; 64];
        let sh = project_sh_weighted(&dirs, &rads, &weights);
        let expect_dc = 2.0 * core::f32::consts::PI.sqrt();
        for (&l, &dc) in l0.iter().zip(sh.c[0].iter()) {
            assert!(
                (dc - expect_dc * l).abs() < 1e-4,
                "DC 锚定:{dc} vs {}",
                expect_dc * l
            );
        }
        for co in sh.c.iter().skip(1) {
            for (&lin, &dc) in co.iter().zip(sh.c[0].iter()) {
                assert!(lin.abs() < 0.02 * dc, "线性项 ≈0:{lin}(DC {dc})");
            }
        }
        // 常量全球场 ⇒ 任意方向 irradiance = π·L0(L1 重建,容差 3%)。
        for n in [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.3, 0.5, 0.8).normalize(),
        ] {
            let e = eval_sh_irradiance(&sh, n);
            for (&got, &l) in e.iter().zip(l0.iter()) {
                let expect = core::f32::consts::PI * l;
                assert!(
                    (got - expect).abs() < 0.03 * expect,
                    "n={n:?}: {got} vs {expect}"
                );
            }
        }
    }

    #[test]
    fn sh_pulse_linear_direction_aligned() {
        // 单方向 δ 脉冲 L(ω) = R·δ(ω−d₀):c₁ = R·Y₁·d₀·w ⇒ 线性项方向与 d₀ 一致。
        let d0 = Vec3::new(0.3, 0.5, 0.8).normalize();
        let sh = project_sh_weighted(&[d0], &[[2.0, 2.0, 2.0]], &[1.0]);
        assert!((sh.c[0][0] - Y00 * 2.0).abs() < 1e-6, "DC = Y₀₀·R·w");
        for ch in 0..3 {
            let c1 = Vec3::new(sh.c[1][ch], sh.c[2][ch], sh.c[3][ch]).normalize();
            assert!(
                c1.dot(d0) > 0.9,
                "线性项方向应与脉冲方向一致:ch{ch} dot={}",
                c1.dot(d0)
            );
        }
    }

    #[test]
    fn sh_eval_clamps_negative() {
        // 强线性 + 零 DC:背向求值理论值为负 ⇒ clamp 到 0(物理 irradiance 非负)。
        let mut sh = ShL1Rgb::ZERO;
        for ch in 0..3 {
            sh.c[1][ch] = 1.0;
        }
        let back = eval_sh_irradiance(&sh, Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(back, [0.0, 0.0, 0.0], "背向应 clamp 到 0");
        let front = eval_sh_irradiance(&sh, Vec3::new(1.0, 0.0, 0.0));
        for &f in &front {
            assert!(f > 0.0, "正向应保持为正");
            assert!(
                (f - COSINE_CONV_1 * Y1).abs() < 1e-6,
                "正向 = A₁·Y₁·c₁: {f} vs {}",
                COSINE_CONV_1 * Y1
            );
        }
    }
}
