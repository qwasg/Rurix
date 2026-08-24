//! ReSTIR DI 高档 reservoir host 参考臂（RFC-0038；G21.2 M-a；M100-high
//! 重判条件「高档 reservoir 证据齐备」的证据产出面）。
//!
//! ## 与 M100 低档面的关系（0-byte 纪律）
//!
//! [`crate::gi::multi_light`] 的低档 MegaLights 固定随机选灯 = 生产默认档
//! （兜底维持字面 0-byte）；其 `check_restir_trigger`/`restir_serve`
//! fail-closed 登记面本模块**不接线不改写**。本模块为独立加性算法面：
//! 流式加权蓄水池采样（WRS）实现 RIS（重采样重要性采样）估计子 +
//! 时域 reservoir 合并，产出「等验证射线预算下方差显著低于均匀选灯」的
//! measured 证据——M100-high 重判的程序输入。
//!
//! ## 估计子（Bitterli et al. 2020 §3）
//!
//! 候选集 M 个灯按源分布 p(x)=1/L 采样；目标函数 p̂(x) = 无阴影贡献标量；
//! WRS 以 w_i = p̂(x_i)/p(x_i) 保留一个样本 y，无偏权
//! `W_y = (1/p̂(y)) · (Σw_i / M)`；估计 = f(y)·W_y（f = p̂，本参考臂无遮挡
//! 几何 ⇒ 无偏性可对解析全灯和逐字验证）。时域合并 = reservoir merge
//! （历史 reservoir 以其 M 计数入池，M-cap 截断防无界置信漂移）。
//!
//! 纯 f32/f64 host 确定性（PCG32 固定 seed 流按索引寻址，双跑位级一致）。

/// PCG32（最小实现；流为输入非结果，G7.4 先例；与 M100_SEED 独立避免跨模块流耦合）。
#[derive(Debug, Clone, Copy)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut s = Self {
            state: 0,
            inc: (stream << 1) | 1,
        };
        s.next_u32();
        s.state = s.state.wrapping_add(seed);
        s.next_u32();
        s
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// [0,1) f32（24 位精度）。
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

/// 点灯（位置 + 强度标量；参考臂用标量辐射度足够承载方差对照语义）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    pub pos: [f32; 3],
    pub intensity: f32,
}

/// 着色点（位置 + 法线）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadePoint {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

/// 无阴影贡献目标函数 p̂（Lambert · 距离衰减；恒 ≥0）。
pub fn target_phat(sp: &ShadePoint, light: &PointLight) -> f32 {
    let d = [
        light.pos[0] - sp.pos[0],
        light.pos[1] - sp.pos[1],
        light.pos[2] - sp.pos[2],
    ];
    let dist2 = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).max(1e-6);
    let dist = dist2.sqrt();
    let ndotl = ((sp.normal[0] * d[0] + sp.normal[1] * d[1] + sp.normal[2] * d[2]) / dist).max(0.0);
    light.intensity * ndotl / dist2
}

/// 解析参考：全灯精确和（参考臂无遮挡几何 ⇒ 无偏性金标准）。
pub fn exact_direct(sp: &ShadePoint, lights: &[PointLight]) -> f64 {
    lights.iter().map(|l| f64::from(target_phat(sp, l))).sum()
}

/// RIS reservoir（单样本槽 + 权重和 + 候选计数）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reservoir {
    /// 保留样本的灯索引（usize::MAX = 空）。
    pub y: usize,
    /// 保留样本的 p̂ 值。
    pub phat_y: f32,
    /// Σ w_i。
    pub w_sum: f64,
    /// 已见候选计数（时域合并置信度载体）。
    pub m: u32,
}

impl Reservoir {
    pub fn empty() -> Self {
        Self {
            y: usize::MAX,
            phat_y: 0.0,
            w_sum: 0.0,
            m: 0,
        }
    }

    /// 流式更新（WRS）：以 w/w_sum 概率替换保留样本。
    pub fn update(&mut self, cand: usize, phat: f32, w: f64, rng: &mut Pcg32) {
        self.w_sum += w;
        self.m += 1;
        if self.w_sum > 0.0 && f64::from(rng.next_f32()) < w / self.w_sum {
            self.y = cand;
            self.phat_y = phat;
        }
    }

    /// 合并另一 reservoir（时域重用；other 以其 m 计数入池）。
    pub fn merge(&mut self, other: &Reservoir, rng: &mut Pcg32, m_cap: u32) {
        if other.y == usize::MAX {
            return;
        }
        let m_other = other.m.min(m_cap);
        // other 的等效权 = phat_y · W_other · m_other
        let w_other = if other.phat_y > 0.0 && other.m > 0 {
            other.w_sum * f64::from(m_other) / f64::from(other.m)
        } else {
            0.0
        };
        let m_before = self.m;
        self.w_sum += w_other;
        if self.w_sum > 0.0 && f64::from(rng.next_f32()) < w_other / self.w_sum {
            self.y = other.y;
            self.phat_y = other.phat_y;
        }
        self.m = m_before + m_other;
    }

    /// 无偏估计权 W_y = (1/p̂(y)) · (w_sum / m)。
    pub fn unbiased_weight(&self) -> f64 {
        if self.y == usize::MAX || self.phat_y <= 0.0 || self.m == 0 {
            return 0.0;
        }
        self.w_sum / (f64::from(self.phat_y) * f64::from(self.m))
    }
}

/// RIS 单帧估计：M 个均匀候选 → WRS 保留 1 → 估计 f(y)·W_y。
pub fn estimate_ris(
    sp: &ShadePoint,
    lights: &[PointLight],
    m_candidates: u32,
    rng: &mut Pcg32,
) -> (f64, Reservoir) {
    let n = lights.len();
    let mut r = Reservoir::empty();
    for _ in 0..m_candidates {
        let cand = (rng.next_u32() as usize) % n;
        let phat = target_phat(sp, &lights[cand]);
        // 源分布 p = 1/L ⇒ w = p̂ · L
        let w = f64::from(phat) * n as f64;
        r.update(cand, phat, w, rng);
    }
    let est = f64::from(r.phat_y) * r.unbiased_weight();
    (est, r)
}

/// 低档同型对照：均匀选 1 灯 MC 估计（选灯概率 1/L 折贡献 ×L）。
pub fn estimate_uniform(sp: &ShadePoint, lights: &[PointLight], rng: &mut Pcg32) -> f64 {
    let n = lights.len();
    let cand = (rng.next_u32() as usize) % n;
    f64::from(target_phat(sp, &lights[cand])) * n as f64
}

/// 方差对照结果（M100-high 重判程序输入）。
#[derive(Debug, Clone, PartialEq)]
pub struct VarianceReport {
    pub n_trials: u32,
    pub reference: f64,
    pub uniform_mean: f64,
    pub uniform_var: f64,
    pub ris_mean: f64,
    pub ris_var: f64,
    pub ris_temporal_mean: f64,
    pub ris_temporal_var: f64,
    /// var(uniform)/var(ris)（>1 = RIS 收益）。
    pub variance_reduction: f64,
    /// var(ris)/var(ris_temporal)（>1 = 时域合并进一步收益）。
    pub temporal_reduction: f64,
}

/// 确定性多灯夹具（L 灯环形分布 + 强度梯度）。
pub fn fixture_lights(n: u32) -> Vec<PointLight> {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            PointLight {
                pos: [2.5 * a.cos(), 1.5 + 0.8 * (a * 3.0).sin(), 2.5 * a.sin()],
                intensity: 0.4 + 3.6 * ((i * 7 + 3) % n) as f32 / n as f32,
            }
        })
        .collect()
}

/// 方差对照实验（等验证预算：每 trial 各估计子恰 1 个保留样本）。
pub fn variance_experiment(
    lights: &[PointLight],
    m_candidates: u32,
    n_trials: u32,
    temporal_frames: u32,
    m_cap: u32,
    seed: u64,
) -> VarianceReport {
    let sp = ShadePoint {
        pos: [0.0, 0.0, 0.0],
        normal: [0.0, 1.0, 0.0],
    };
    let reference = exact_direct(&sp, lights);
    let stats = |vals: &[f64]| -> (f64, f64) {
        let mean = vals.iter().sum::<f64>() / vals.len() as f64;
        let var = vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (vals.len() - 1) as f64;
        (mean, var)
    };
    let mut uni = Vec::with_capacity(n_trials as usize);
    let mut ris = Vec::with_capacity(n_trials as usize);
    let mut ris_t = Vec::with_capacity(n_trials as usize);
    for t in 0..n_trials {
        let mut rng_u = Pcg32::new(seed, u64::from(t) * 4);
        let mut rng_r = Pcg32::new(seed, u64::from(t) * 4 + 1);
        let mut rng_t = Pcg32::new(seed, u64::from(t) * 4 + 2);
        uni.push(estimate_uniform(&sp, lights, &mut rng_u));
        ris.push(estimate_ris(&sp, lights, m_candidates, &mut rng_r).0);
        // 时域：temporal_frames 帧 reservoir 链式合并后估计
        let mut hist = Reservoir::empty();
        for _ in 0..temporal_frames {
            let (_, cur) = estimate_ris(&sp, lights, m_candidates, &mut rng_t);
            let mut merged = cur;
            merged.merge(&hist, &mut rng_t, m_cap);
            hist = merged;
        }
        ris_t.push(f64::from(hist.phat_y) * hist.unbiased_weight());
    }
    let (u_mean, u_var) = stats(&uni);
    let (r_mean, r_var) = stats(&ris);
    let (t_mean, t_var) = stats(&ris_t);
    VarianceReport {
        n_trials,
        reference,
        uniform_mean: u_mean,
        uniform_var: u_var,
        ris_mean: r_mean,
        ris_var: r_var,
        ris_temporal_mean: t_mean,
        ris_temporal_var: t_var,
        variance_reduction: u_var / r_var.max(1e-30),
        temporal_reduction: r_var / t_var.max(1e-30),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbiasedness_vs_exact_reference() {
        let lights = fixture_lights(64);
        let rep = variance_experiment(&lights, 16, 20_000, 4, 60, 0x0521_0A11);
        // 3σ 无偏检验（σ_mean = sqrt(var/n)）
        for (name, mean, var) in [
            ("uniform", rep.uniform_mean, rep.uniform_var),
            ("ris", rep.ris_mean, rep.ris_var),
            ("ris_temporal", rep.ris_temporal_mean, rep.ris_temporal_var),
        ] {
            let sigma_mean = (var / f64::from(rep.n_trials)).sqrt();
            let dev = (mean - rep.reference).abs();
            assert!(
                dev < 3.0 * sigma_mean + 1e-9,
                "{name} 估计子偏置：mean={mean:.6} ref={:.6} dev={dev:.6} > 3σ={:.6}",
                rep.reference,
                3.0 * sigma_mean
            );
        }
    }

    #[test]
    fn ris_variance_below_uniform() {
        let lights = fixture_lights(64);
        let rep = variance_experiment(&lights, 16, 8_000, 4, 60, 0x0521_0B22);
        assert!(
            rep.variance_reduction > 2.0,
            "RIS 方差收益不足：var(uniform)/var(ris)={:.3}（要求 >2 显著收益）",
            rep.variance_reduction
        );
    }

    #[test]
    fn temporal_merge_reduces_variance_further() {
        let lights = fixture_lights(64);
        let rep = variance_experiment(&lights, 16, 8_000, 8, 60, 0x0521_0C33);
        assert!(
            rep.temporal_reduction > 1.2,
            "时域合并收益不足：var(ris)/var(temporal)={:.3}（要求 >1.2）",
            rep.temporal_reduction
        );
    }

    #[test]
    fn reservoir_mcap_bounds_confidence() {
        let mut r = Reservoir::empty();
        let mut rng = Pcg32::new(7, 7);
        r.update(0, 1.0, 1.0, &mut rng);
        let mut other = Reservoir::empty();
        other.update(1, 1.0, 1.0, &mut rng);
        other.m = 1000;
        other.w_sum = 1000.0;
        let mut merged = r;
        merged.merge(&other, &mut rng, 20);
        assert_eq!(merged.m, 1 + 20, "m_cap 必须截断历史置信计数");
    }

    #[test]
    fn double_run_bitexact() {
        let lights = fixture_lights(32);
        let a = variance_experiment(&lights, 8, 2_000, 4, 60, 0x0521_0D44);
        let b = variance_experiment(&lights, 8, 2_000, 4, 60, 0x0521_0D44);
        assert_eq!(a, b, "固定 seed 双跑必须位级一致");
    }
}
