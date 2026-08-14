//! 皮肤 Burley 屏单 pass(G9.5 M115;RFC-0025 §4.F + §4.L;spec/
//! display_pipeline.md RXS-0373 L1~L5 逐条对齐)。
//!
//! //@ spec: RXS-0373
//!
//! 本模块承载 M115 皮肤专项着色语义面:
//!
//! - **Burley 屏单 pass**(L1):normalized diffusion 屏空单 pass separable
//!   SSS,**颜色/深度双 kernel**([`burley_color_kernel`] / [`depth_kernel`])
//!   一次卷积求值([`eval_skin_sss`]);与 host 参考实现双跑位级一致 + golden
//!   冻结带对照(device 对拍面归后续波,同 M112 host 确定性口径)。
//! - **扩散 profile 资产化**(L2):[`BurleyProfile`](crate::material::side_table::BurleyProfile)
//!   RGB 三通道 falloff 为 per-material 资产,经 RFC-0025 §4.L 资产化侧表
//!   扩展通道按材质槽 ID 索引接入;**扩散 profile 参数 → 扩散半径响应
//!   golden**(falloff 增大 ⇒ 扩散半径单调增大机核)。
//! - **pre-integrated LUT 回退档**(L3):曲率 × NdotL LUT
//!   ([`build_preintegrated_lut`])在低端 profile 启用;两档画质差(max/mean
//!   abs)纳入 golden 对照(measured 冻结)。
//! - **profile 全零衰减 RED 臂**(L4):profile 全零衰减注入必须**退化为纯
//!   漫反射**([`eval_pure_diffuse`] 逐位相等机核);未退化即 profile 未生效,
//!   `Err(ProfileNotWired)`(RED);非零 profile 输出与纯漫反射无差异同样
//!   判 profile 未生效 RED。
//! - **触 MaterialClosure 32B 经 RFC-0025 §4.L 修订行**(L5):扩展参数只经
//!   侧表通道;32B 布局 digest 逐位相等 + `reserved`/flags 未分配位段零消费
//!   机核([`crate::material::side_table`]);**缺省侧表 ≡ 既有输出逐位不变**
//!   ([`eval_skin_entry`] 无侧表/缺省侧表双路径 digest 恒等)。
//!
//! 纪律:host 纯 safe 确定性;零新 FFI;无 device 依赖;`RURIX_REQUIRE_REAL=1`
//! 下以 host 确定性为准。

use rurix_pkg::sha256;

use crate::material::side_table::{BurleyProfile, LobeExtension, MaterialSideTable, SideTableError};

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// SSS 卷积核半径抽头数(canonical;separable 单 pass 一维参照)。
pub const SSS_KERNEL_TAPS: usize = 16;
/// canonical 皮肤斑块采样数(1D 信号参照)。
pub const SKIN_PATCH_SAMPLES: usize = 64;
/// pre-integrated LUT 维度(曲率 × NdotL 各 32)。
pub const LUT_DIM: usize = 32;

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed)
// ---------------------------------------------------------------------------

/// 皮肤着色失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum SkinError {
    /// profile 未生效(全零衰减未退化纯漫反射 / 非零 profile 无扩散差异,RED 锚)。
    ProfileNotWired { why: &'static str },
    /// 输入含非有限值。
    NonFiniteValue { stage: &'static str },
    /// 非 canonical 构造。
    NotCanonical(&'static str),
    /// 侧表面失败传导。
    SideTable(SideTableError),
}

impl std::fmt::Display for SkinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkinError::ProfileNotWired { why } => write!(f, "扩散 profile 未生效: {why}(RED)"),
            SkinError::NonFiniteValue { stage } => write!(f, "{stage} 含非有限值"),
            SkinError::NotCanonical(why) => write!(f, "not canonical: {why}"),
            SkinError::SideTable(e) => write!(f, "侧表: {e}"),
        }
    }
}

impl std::error::Error for SkinError {}

impl From<SideTableError> for SkinError {
    fn from(e: SideTableError) -> Self {
        SkinError::SideTable(e)
    }
}

pub type Result<T> = std::result::Result<T, SkinError>;

// ---------------------------------------------------------------------------
// Burley normalized diffusion 双 kernel(L1)
// ---------------------------------------------------------------------------

/// Burley 颜色 kernel(normalized diffusion 径向剖面 R(r) = s·(e^{−sr} +
/// e^{−sr/3})/(8πr),s = 1/falloff;对称抽头 r = |t − TAPS/2|,归一化 Σw = 1
/// ——separable 单 pass 一维参照;**falloff = 0 ⇒ Dirac δ(中心抽头 = 1,其余
/// 0),退化纯漫反射**)。
pub fn burley_color_kernel(falloff: f32) -> Result<[f32; SSS_KERNEL_TAPS]> {
    if !falloff.is_finite() || falloff < 0.0 {
        return Err(SkinError::NonFiniteValue { stage: "burley kernel" });
    }
    let mut k = [0.0f32; SSS_KERNEL_TAPS];
    let half = SSS_KERNEL_TAPS / 2;
    if falloff == 0.0 {
        k[half] = 1.0;
        return Ok(k);
    }
    let s = 1.0 / falloff;
    let mut sum = 0.0f32;
    for (t, w) in k.iter_mut().enumerate() {
        let r = (t as f32 - half as f32).abs(); // 抽头半径(样本单位)
        let v = if r == 0.0 {
            // r→0 极限以 r=1/2 中值代替(确定性收敛)。
            s * ((-s * 0.5).exp() + (-s * 0.5 / 3.0).exp()) / (8.0 * std::f32::consts::PI * 0.5)
        } else {
            s * ((-s * r).exp() + (-s * r / 3.0).exp()) / (8.0 * std::f32::consts::PI * r)
        };
        *w = v;
        sum += v;
    }
    for w in k.iter_mut() {
        *w /= sum;
    }
    Ok(k)
}

/// 深度 kernel(双 kernel 之二:深度差门控扩散权重,e^{−|Δdepth|·sharpness};
/// sharpness 冻结 2.0)。
pub fn depth_kernel(delta: f32) -> f32 {
    (-delta.abs() * 2.0).exp()
}

/// Burley 屏单 pass 求值(颜色 kernel 卷积 × 深度 kernel 门控;RGB 逐通道
/// falloff)。`signal` = 漫反射着色输入(纯漫反射基线同源)。
pub fn eval_skin_sss(signal: &[[f32; 3]], depth: &[f32], profile: &BurleyProfile) -> Result<Vec<[f32; 3]>> {
    if signal.len() != depth.len() {
        return Err(SkinError::NotCanonical("signal/depth 长度不符"));
    }
    if signal.iter().any(|p| !p.iter().all(|v| v.is_finite()))
        || depth.iter().any(|d| !d.is_finite())
    {
        return Err(SkinError::NonFiniteValue { stage: "sss input" });
    }
    let kernels = [
        burley_color_kernel(profile.falloff_rgb[0])?,
        burley_color_kernel(profile.falloff_rgb[1])?,
        burley_color_kernel(profile.falloff_rgb[2])?,
    ];
    let n = signal.len();
    let mut out = vec![[0.0f32; 3]; n];
    for (i, o) in out.iter_mut().enumerate() {
        let mut acc = [0.0f32; 3];
        let mut wsum = [0.0f32; 3];
        let half = SSS_KERNEL_TAPS / 2;
        for (c, kernel) in kernels.iter().enumerate() {
            for (t, &kw) in kernel.iter().enumerate() {
                let j = i as isize + t as isize - half as isize;
                if j < 0 || j >= n as isize {
                    continue;
                }
                let j = j as usize;
                let w = kw * depth_kernel(depth[i] - depth[j]);
                acc[c] += signal[j][c] * w;
                wsum[c] += w;
            }
        }
        // 逐通道保能量归一(无邻近样本时回落自身)。
        for c in 0..3 {
            o[c] = if wsum[c] > 0.0 { acc[c] / wsum[c] } else { signal[i][c] };
        }
    }
    Ok(out)
}

/// 纯漫反射参考(L4 退化判据基准 = 无扩散恒等面)。
pub fn eval_pure_diffuse(signal: &[[f32; 3]]) -> Vec<[f32; 3]> {
    signal.to_vec()
}

/// 全零衰减退化机核(L4):profile 全零 ⇒ SSS 输出必须与纯漫反射**逐位相等**;
/// 不等即 profile 未生效 RED。返回逐位相等布尔(harness/单测判据面)。
pub fn zero_falloff_degrades_to_diffuse(out: &[[f32; 3]], diffuse: &[[f32; 3]]) -> bool {
    out.len() == diffuse.len()
        && out.iter().zip(diffuse.iter()).all(|(a, b)| {
            (0..3).all(|c| a[c].to_bits() == b[c].to_bits())
        })
}

/// profile 生效机核(L4 对偶):非零 profile 输出必须与纯漫反射有可见差异
/// (无差异 = profile 未生效 RED)。返回「存在差异」布尔。
pub fn profile_has_visible_effect(out: &[[f32; 3]], diffuse: &[[f32; 3]]) -> bool {
    !zero_falloff_degrades_to_diffuse(out, diffuse)
}

/// 扩散半径响应(L2 golden 面):kernel 加权平均半径(falloff 增大 ⇒ 单调增大)。
pub fn diffusion_radius(profile: &BurleyProfile) -> Result<f32> {
    let half = SSS_KERNEL_TAPS / 2;
    let mut acc = 0.0f32;
    for c in 0..3 {
        let k = burley_color_kernel(profile.falloff_rgb[c])?;
        for (t, w) in k.iter().enumerate() {
            acc += (t as f32 - half as f32).abs() * w;
        }
    }
    Ok(acc / 3.0)
}

// ---------------------------------------------------------------------------
// pre-integrated LUT 回退档(L3)
// ---------------------------------------------------------------------------

/// pre-integrated LUT(曲率 × NdotL;低端 profile 回退档;确定性闭式:
/// lut[c][n] = 漫反射半球积分近似 shade = clamp(n·l 经曲率展宽))。
pub fn build_preintegrated_lut() -> Vec<[f32; LUT_DIM]> {
    let mut lut = vec![[0.0f32; LUT_DIM]; LUT_DIM];
    for (ci, row) in lut.iter_mut().enumerate() {
        let curvature = ci as f32 / (LUT_DIM - 1) as f32;
        for (ni, cell) in row.iter_mut().enumerate() {
            let ndotl = ni as f32 / (LUT_DIM - 1) as f32;
            // 曲率展宽:NdotL 经 wrap 角 θ = atan(curvature·π)。
            let wrap = curvature;
            let wrapped = ((ndotl + wrap) / (1.0 + wrap)).clamp(0.0, 1.0);
            *cell = wrapped;
        }
    }
    lut
}

/// LUT 回退档求值(逐样本:diffuse × lut[curvature, ndotl])。
pub fn eval_lut_fallback(
    signal: &[[f32; 3]],
    lut: &[[f32; LUT_DIM]],
    curvature_idx: &[usize],
    ndotl_idx: &[usize],
) -> Result<Vec<[f32; 3]>> {
    if signal.len() != curvature_idx.len() || signal.len() != ndotl_idx.len() {
        return Err(SkinError::NotCanonical("lut 索引长度不符"));
    }
    let mut out = Vec::with_capacity(signal.len());
    for (i, p) in signal.iter().enumerate() {
        let ci = curvature_idx[i].min(LUT_DIM - 1);
        let ni = ndotl_idx[i].min(LUT_DIM - 1);
        let s = lut[ci][ni];
        out.push([p[0] * s, p[1] * s, p[2] * s]);
    }
    Ok(out)
}

/// 两档画质差(L3 golden 对照面):max/mean abs。
pub fn tier_quality_diff(a: &[[f32; 3]], b: &[[f32; 3]]) -> Result<(f32, f32)> {
    if a.len() != b.len() {
        return Err(SkinError::NotCanonical("画质差长度不符"));
    }
    let mut max_d = 0.0f32;
    let mut sum = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        for c in 0..3 {
            let d = (x[c] - y[c]).abs();
            max_d = max_d.max(d);
            sum += d;
        }
    }
    Ok((max_d, sum / (a.len() * 3) as f32))
}

// ---------------------------------------------------------------------------
// §4.L 侧表接入面(L5)
// ---------------------------------------------------------------------------

/// 皮肤求值入口(侧表通道):`table = None` ⇒ 无侧表基线(纯漫反射——既有材
/// 质输出面);`Some(空/缺省侧表)` ⇒ 缺省路径,**必须逐位等于基线**(修订行
/// 零漂移);槽命中 Burley 扩展 ⇒ 扩散求值。
pub fn eval_skin_entry(
    signal: &[[f32; 3]],
    depth: &[f32],
    slot: u32,
    table: Option<&MaterialSideTable>,
) -> Result<Vec<[f32; 3]>> {
    let ext = match table {
        None => None,
        Some(t) => t.lookup(slot),
    };
    match ext {
        Some(LobeExtension::Burley(p)) => eval_skin_sss(signal, depth, p),
        Some(LobeExtension::Marschner(_)) => {
            Err(SkinError::SideTable(SideTableError::NotCanonical("皮肤槽误挂 Marschner 扩展")))
        }
        None => Ok(eval_pure_diffuse(signal)),
    }
}

/// 图像 digest(golden 对照事实源)。
pub fn image_digest(img: &[[f32; 3]]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(img.len() * 12);
    for p in img {
        for c in p {
            buf.extend_from_slice(&c.to_le_bytes());
        }
    }
    sha256::digest(&buf)
}

// ---------------------------------------------------------------------------
// canonical 场景(golden 事实源)
// ---------------------------------------------------------------------------

/// canonical 皮肤斑块(64 样本漫反射信号 = Lambert 渐变 + 中央高光脉冲;深度
/// = 缓坡 + 阶跃,驱动深度 kernel 门控)。
pub fn canonical_skin_patch() -> (Vec<[f32; 3]>, Vec<f32>) {
    let mut signal = Vec::with_capacity(SKIN_PATCH_SAMPLES);
    let mut depth = Vec::with_capacity(SKIN_PATCH_SAMPLES);
    for i in 0..SKIN_PATCH_SAMPLES {
        let t = i as f32 / (SKIN_PATCH_SAMPLES - 1) as f32;
        let lamber = (t * std::f32::consts::PI).sin() * 0.7 + 0.3;
        let impulse = if i == SKIN_PATCH_SAMPLES / 2 { 0.8 } else { 0.0 };
        signal.push([
            lamber * 0.9 + impulse,
            lamber * 0.7 + impulse * 0.9,
            lamber * 0.55 + impulse * 0.7,
        ]);
        depth.push(t * 0.6 + if i >= SKIN_PATCH_SAMPLES / 2 { 0.05 } else { 0.0 });
    }
    (signal, depth)
}

/// canonical 扩散 profile(皮肤典型 RGB falloff 0.8/0.5/0.25)。
pub fn canonical_skin_profile() -> BurleyProfile {
    BurleyProfile { falloff_rgb: [0.8, 0.5, 0.25] }
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::side_table::assert_default_table_invariant;

    //@ spec: RXS-0373
    #[test]
    fn zero_falloff_degrades_to_pure_diffuse() {
        let (signal, depth) = canonical_skin_patch();
        let zero = BurleyProfile { falloff_rgb: [0.0, 0.0, 0.0] };
        let out = eval_skin_sss(&signal, &depth, &zero).unwrap();
        let diffuse = eval_pure_diffuse(&signal);
        assert!(zero_falloff_degrades_to_diffuse(&out, &diffuse));
        // 非零 profile ⇒ 必须有可见差异(profile 生效)。
        let out2 = eval_skin_sss(&signal, &depth, &canonical_skin_profile()).unwrap();
        assert!(profile_has_visible_effect(&out2, &diffuse));
    }

    //@ spec: RXS-0373
    #[test]
    fn diffusion_radius_monotonic_in_falloff() {
        let r1 = diffusion_radius(&BurleyProfile { falloff_rgb: [0.2, 0.2, 0.2] }).unwrap();
        let r2 = diffusion_radius(&BurleyProfile { falloff_rgb: [0.6, 0.6, 0.6] }).unwrap();
        let r3 = diffusion_radius(&canonical_skin_profile()).unwrap();
        assert!(r2 > r1, "扩散半径未随 falloff 增大: {r1} -> {r2}");
        assert!(r3 > 0.0);
    }

    //@ spec: RXS-0373
    #[test]
    fn default_side_table_bit_equal_baseline() {
        let (signal, depth) = canonical_skin_patch();
        let baseline = eval_skin_entry(&signal, &depth, 0, None).unwrap();
        let default_tbl = MaterialSideTable::new();
        let with_default = eval_skin_entry(&signal, &depth, 0, Some(&default_tbl)).unwrap();
        assert_default_table_invariant(&image_digest(&baseline), &image_digest(&with_default)).unwrap();
    }

    //@ spec: RXS-0373
    #[test]
    fn lut_fallback_and_tier_diff() {
        let (signal, depth) = canonical_skin_patch();
        let sss = eval_skin_sss(&signal, &depth, &canonical_skin_profile()).unwrap();
        let lut = build_preintegrated_lut();
        let idx = vec![8usize; SKIN_PATCH_SAMPLES];
        let ndx: Vec<usize> = (0..SKIN_PATCH_SAMPLES)
            .map(|i| i * (LUT_DIM - 1) / SKIN_PATCH_SAMPLES)
            .collect();
        let lut_out = eval_lut_fallback(&signal, &lut, &idx, &ndx).unwrap();
        let (max_d, mean_d) = tier_quality_diff(&sss, &lut_out).unwrap();
        assert!(max_d > 0.0 && mean_d > 0.0); // 两档画质差非空(golden 对照面)
    }
}
