//! Substrate 类双层 slab 材质闭合 host 参考臂（RFC-0039；G22.2 M-a；
//! RD-041「Substrate 类 slab 分层材质」分项的语义参考面）。
//!
//! ## 模型（方向-半球反照率层级；host 纯解析参考）
//!
//! 双层 slab：coating（反照率 `r_c`，透过率 `t_c = 1 − r_c`，无吸收损耗档）
//! 覆盖 base（反照率 `a_b`）。层间无穷次弹跳的解析闭式：
//!
//! ```text
//! R_total = r_c + t_c² · a_b / (1 − r_c · a_b)
//! ```
//!
//! （几何级数：base 反射经 coating 内面再反射回 base 的每一程乘 `r_c·a_b`。）
//!
//! ## 能量守恒硬不变量（M-a 程序产判据）
//!
//! - **白炉**：`a_b = 1` 且 coating 无损 ⇒ `R_total = 1`（容差 1e-6 解析级）；
//! - **上界**：任意 `r_c, a_b ∈ [0,1]` ⇒ `R_total ≤ 1`（能量不增生）；
//! - **单调**：`R_total` 对 `a_b` 严格单调不减；
//! - **闭式 ↔ 数值级数对拍**：解析闭式与 N 次弹跳数值和收敛一致；
//! - 纯 f32/f64 确定性（双跑位级）。
//!
//! 与 [`crate::material::closure`] 单层闭合面的关系：0-byte 不接线——本模块为
//! 独立加性语义参考；侧表/PSO 集成归后续波（RFC-0039 out-of-scope 登记）。

/// 双层 slab 参数（无损 coating 档：吸收档 = 后续波扩展位，本参考面显式 0）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlabStack {
    /// coating 方向-半球反照率 ∈ [0,1]。
    pub coating_reflectance: f32,
    /// base 方向-半球反照率 ∈ [0,1]。
    pub base_albedo: f32,
}

impl SlabStack {
    pub fn new(coating_reflectance: f32, base_albedo: f32) -> Self {
        assert!((0.0..=1.0).contains(&coating_reflectance), "r_c 域 [0,1]");
        assert!((0.0..=1.0).contains(&base_albedo), "a_b 域 [0,1]");
        Self {
            coating_reflectance,
            base_albedo,
        }
    }

    /// 解析闭式总反照率（无穷弹跳几何级数）。
    pub fn total_reflectance(&self) -> f64 {
        let rc = f64::from(self.coating_reflectance);
        let tc = 1.0 - rc;
        let ab = f64::from(self.base_albedo);
        let denom = 1.0 - rc * ab;
        if denom <= 0.0 {
            // rc=ab=1 极限：全反射
            return 1.0;
        }
        rc + tc * tc * ab / denom
    }

    /// N 次弹跳数值级数（闭式对拍金标准；截断和，不含尾和）。
    pub fn total_reflectance_series(&self, bounces: u32) -> f64 {
        let rc = f64::from(self.coating_reflectance);
        let tc = 1.0 - rc;
        let ab = f64::from(self.base_albedo);
        let mut acc = rc;
        let mut path = tc * ab * tc; // 首程：透过→base 反→透出
        let mut k = 0;
        while k < bounces {
            acc += path;
            path *= rc * ab; // 每加一程：coating 内面反 + base 再反
            k += 1;
        }
        acc
    }

    /// N 次截断后的解析尾和（几何级数余项；q = r_c·a_b ≥ 1 时首程为零 ⇒ 尾和 0）。
    pub fn series_tail(&self, bounces: u32) -> f64 {
        let rc = f64::from(self.coating_reflectance);
        let tc = 1.0 - rc;
        let ab = f64::from(self.base_albedo);
        let q = rc * ab;
        if q >= 1.0 {
            return 0.0; // 仅 rc=ab=1 可达；此时 tc=0 ⇒ 级数各程恒 0
        }
        let first = tc * tc * ab;
        first * q.powi(bounces as i32) / (1.0 - q)
    }

    /// 层参数线性插值（Substrate 类 slab 参数域连续性载体）。
    pub fn lerp(a: &SlabStack, b: &SlabStack, t: f32) -> SlabStack {
        SlabStack {
            coating_reflectance: a.coating_reflectance * (1.0 - t) + b.coating_reflectance * t,
            base_albedo: a.base_albedo * (1.0 - t) + b.base_albedo * t,
        }
    }
}

/// 白炉能量审计报告（M-a 消费面）。
#[derive(Debug, Clone, PartialEq)]
pub struct FurnaceReport {
    pub samples: u32,
    pub max_total: f64,
    pub white_furnace_dev: f64,
    pub monotonic_violations: u32,
    pub series_closed_form_max_dev: f64,
}

/// 确定性参数网格白炉审计。
pub fn furnace_audit(grid: u32, bounces: u32) -> FurnaceReport {
    let mut max_total = 0.0f64;
    let mut mono_violations = 0u32;
    let mut series_dev = 0.0f64;
    for i in 0..=grid {
        let rc = i as f32 / grid as f32;
        let mut prev = -1.0f64;
        for j in 0..=grid {
            let ab = j as f32 / grid as f32;
            let s = SlabStack::new(rc, ab);
            let total = s.total_reflectance();
            max_total = max_total.max(total);
            if total + 1e-12 < prev {
                mono_violations += 1;
            }
            prev = total;
            // 恒等式对拍：closed == series(N) + 解析尾和（数学精确，仅剩浮点误差）
            let num = s.total_reflectance_series(bounces) + s.series_tail(bounces);
            series_dev = series_dev.max((total - num).abs());
        }
    }
    let white = SlabStack::new(0.3, 1.0).total_reflectance();
    FurnaceReport {
        samples: (grid + 1) * (grid + 1),
        max_total,
        white_furnace_dev: (white - 1.0).abs(),
        monotonic_violations: mono_violations,
        series_closed_form_max_dev: series_dev,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_furnace_identity() {
        for rc in [0.0f32, 0.1, 0.3, 0.7, 0.95] {
            let s = SlabStack::new(rc, 1.0);
            let t = s.total_reflectance();
            assert!((t - 1.0).abs() < 1e-9, "白炉 r_c={rc}: total={t} 必须 =1");
        }
    }

    #[test]
    fn energy_never_exceeds_unity() {
        let rep = furnace_audit(64, 64);
        assert!(rep.max_total <= 1.0 + 1e-9, "能量增生：max_total={}", rep.max_total);
        assert_eq!(rep.monotonic_violations, 0, "对 a_b 单调性违例");
    }

    #[test]
    fn series_plus_tail_identity_matches_closed_form() {
        let rep = furnace_audit(32, 96);
        assert!(
            rep.series_closed_form_max_dev < 1e-9,
            "闭式 vs 级数+尾和恒等式偏差 {}（数学精确，容差 1e-9 浮点级）",
            rep.series_closed_form_max_dev
        );
        // 内域（远离 q→1 角点）截断和自身也须收敛到闭式
        let s = SlabStack::new(0.4, 0.6);
        let dev = (s.total_reflectance() - s.total_reflectance_series(64)).abs();
        assert!(dev < 1e-9, "内域 64 弹跳截断和未收敛：dev={dev}");
    }

    #[test]
    fn lossy_base_loses_energy() {
        let s = SlabStack::new(0.2, 0.5);
        let t = s.total_reflectance();
        assert!(t < 1.0 && t > 0.2, "有损 base 总反照率应 ∈ (r_c, 1)：{t}");
    }

    #[test]
    fn lerp_continuity_and_determinism() {
        let a = SlabStack::new(0.1, 0.2);
        let b = SlabStack::new(0.8, 0.9);
        let mut prev = SlabStack::lerp(&a, &b, 0.0).total_reflectance();
        for k in 1..=16 {
            let t = k as f32 / 16.0;
            let cur = SlabStack::lerp(&a, &b, t).total_reflectance();
            assert!((cur - prev).abs() < 0.12, "参数 lerp 反照率跳变 {prev}→{cur}");
            prev = cur;
        }
        let r1 = furnace_audit(24, 48);
        let r2 = furnace_audit(24, 48);
        assert_eq!(r1, r2, "白炉审计双跑必须位级一致");
    }
}
