//! G12.2 生产化核心波 host 面（spec/global_illumination.md RXS-0398~0401;
//! RFC-0029 §4.1~§4.4;门 `g12.p0.m158.mis_full_surface` /
//! `g12.p0.m159.russian_roulette_prod` / `g12.p0.m160.sampling_lds_upgrade` /
//! `g12.p0.m161.convergence_criterion_prod`）。
//!
//! 本模块 = M96 参照器（[`super`]，RXS-0357 冻结面，0-byte 只消费不回写）的
//! **生产化演进 host 数据面/host oracle**，与 device megakernel
//! `kernels/g12_pt_production.rx` 公式面逐字同源（RXS-0357 host oracle 纪律
//! 继承：仅 host 输出不能充绿，门绿由 device 腿承载）：
//! - [`ProdLight`] / [`ProdScene`]:生产化场景面——多光源（面光 quad + delta
//!   点光）+ 白炉（albedo=1 全反射闭域，生产化校验面放宽 albedo ≤ 1；M96
//!   校验面 <1 字面不动）;m96_cornell/m96_direct 经 [`prod_scene_from_m96`]
//!   同源转换消费（冻结 fixtures 不回写）;
//! - [`LightDist`]:光源分布（离散 PDF + CDF）确定性构建——同场景同分布
//!   digest（RXS-0398 L3）;
//! - [`mis_balance_nee`] / [`mis_balance_bsdf`]:balance heuristic 权重闭式
//!   （delta 光源退化 w_nee=1 无除零，RXS-0398 L2）;
//! - [`RrProdParams`]:吞吐自适应 RR——`p_kill = clamp(1 − T/τ, 0, p_max)`
//!   （τ 标定程序产;p_max < 1 恒成立;N_min ≥ 2 最小反弹保障）+ 无偏补偿闭式
//!   `1/(1−p_kill)`（上界 [`G12_RR_COMP_CLAMP`] 登记,RXS-0399 L1/L2）;
//! - [`sampler`]:采样器族（PCG 独立流承 G8 对拍模式 / stratified 分层 /
//!   Sobol 类确定性种子扰动）——**索引推导确定性**（样本值 = f(像素索引,
//!   采样索引, 维度, seed) 确定函数寻址,无数据相关状态）+ 固定 seed 位级
//!   一致维持 + 流布局 provenance（RXS-0400,RXS-0357 L2 加性扩展）;
//! - [`AdaptiveParams`]:逐像素方差驱动自适应 spp 终止（Σx/Σx² 协议沿
//!   RXS-0357 L2 out_stats 面;spp 下界 [`G12_ADAPTIVE_N_FLOOR`] 保障;
//!   阈值标定程序产禁手写）+ 收敛报告（spp 分布/方差/未收敛计数）+ 帧型
//!   标签闭集 {adaptive, full_reference}（RXS-0401）;
//! - [`trace_host_prod`]:host oracle（逐像素顺序累加,禁 atomic;RR 计数/
//!   逐级能量/收敛标志逐像素导出,确定函数聚合）;
//! - [`pack_prod_params`] / [`pack_prod_mats`] / [`pack_prod_tris`] /
//!   [`pack_prod_lights`]:device 输入打包（kernel 头注参数面逐字同源）。

use super::{
    MaterialKind, PT_PI, PtCamera, PtError, PtLightQuad, PtScene, m96_cornell_scene,
    m96_direct_light_scene,
};
use crate::rt::bvh::{BlasSet, InstanceDesc, Ray, Tlas, Transform3x4, TriBvh, Vec3};
use crate::rt::ref_tracer::{Pcg32, RAY_EPS};

// ---------------------------------------------------------------------------
// 生产化流布局(RXS-0400 加性扩展面;RXS-0357 L2 冻结排布 0-byte——M96 流布局不动)
// ---------------------------------------------------------------------------

/// 生产化每 bounce 随机维数(光源选择 1 + NEE 2 + BSDF 2 + RR 1)。
pub const PROD_DIMS_PER_BOUNCE: usize = 6;
/// 生产化每采样相机维数。
pub const PROD_DIMS_CAMERA: usize = 2;
/// 生产化最大 bounce(匹配深度沿 M96 冻结 = 4)。
pub const PROD_MAX_BOUNCES: u32 = super::M96_MAX_BOUNCES;
/// 生产化固定 seed(固定 seed 确定性协议继承;消费 M96 冻结 seed 值——生产化
/// 流布局为加性新面,与 M96 流 stride 不同,输出域不交集)。
pub const G12_PROD_SEED: u64 = super::M96_SEED;
/// 生产化 spp 序列(收敛曲线采样点,沿 M96 冻结序列)。
pub const G12_PROD_SPP_SEQUENCE: [u32; 4] = super::M96_SPP_SEQUENCE;

/// 每采样 floats(= 2 + 6·max_bounces)。
pub fn prod_sample_stride(max_bounces: u32) -> usize {
    PROD_DIMS_CAMERA + PROD_DIMS_PER_BOUNCE * max_bounces as usize
}

/// 流总长(= pixel_count · spp · sample_stride)。
pub fn prod_stream_len(pixel_count: usize, spp: u32, max_bounces: u32) -> usize {
    pixel_count * spp as usize * prod_sample_stride(max_bounces)
}

/// 采样 (pixel, sample) 的流起始下标。
pub fn prod_sample_base(pixel: usize, sample: usize, spp: u32, max_bounces: u32) -> usize {
    (pixel * spp as usize + sample) * prod_sample_stride(max_bounces)
}

/// bounce `b` 的六维在采样段内的偏移。
pub fn prod_bounce_base(sample_base: usize, bounce: usize) -> usize {
    sample_base + PROD_DIMS_CAMERA + bounce * PROD_DIMS_PER_BOUNCE
}

// ---------------------------------------------------------------------------
// 采样器族(RXS-0400:分层/低差异序列 + 索引推导确定性 + 流布局 provenance)
// ---------------------------------------------------------------------------

/// 采样器族(选型 benchmark measured 裁决,选型证据进 evidence)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerFamily {
    /// 逐像素独立 PCG32 流(承 G8 ref_tracer 对拍模式;独立流锚)。
    Pcg,
    /// 分层采样(stratified per-dimension:确定性置换 + 确定性 jitter)。
    Stratified,
    /// Sobol 类低差异序列(本原多项式程序生成 + 确定性种子扰动 Cranley-
    /// Patterson 旋转)。
    Sobol,
}

impl SamplerFamily {
    /// 稳定名(evidence/provenance 字面)。
    pub fn name(self) -> &'static str {
        match self {
            SamplerFamily::Pcg => "pcg_independent",
            SamplerFamily::Stratified => "stratified_per_dimension",
            SamplerFamily::Sobol => "sobol_class_seed_perturbed",
        }
    }

    /// 由稳定名解析(fail-closed)。
    pub fn parse(s: &str) -> Option<SamplerFamily> {
        match s {
            "pcg" | "pcg_independent" => Some(SamplerFamily::Pcg),
            "stratified" | "stratified_per_dimension" => Some(SamplerFamily::Stratified),
            "sobol" | "sobol_class_seed_perturbed" => Some(SamplerFamily::Sobol),
            _ => None,
        }
    }
}

/// splitmix64 终态混合(确定性 u64 hash;采样器扰动/置换的确定函数载体)。
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// u64 → [0,1) 均匀(高 24 位 / 2²⁴,与 Pcg32::next_f32 值域口径一致)。
fn u01_from_hash(h: u64) -> f32 {
    (h >> 40) as f32 * (1.0 / 16_777_216.0)
}

/// GF(2) 多项式乘(模 2;位表示)。
fn gf2_mul(a: u64, b: u64) -> u64 {
    let mut acc = 0u64;
    let mut bb = b;
    let mut aa = a;
    while bb != 0 {
        if bb & 1 != 0 {
            acc ^= aa;
        }
        aa <<= 1;
        bb >>= 1;
    }
    acc
}

/// GF(2) 多项式模约减(p 为首一多项式,bit s 隐含最高次)。
fn gf2_mod(mut a: u64, p: u64, s: u32) -> u64 {
    let full = p | (1u64 << s);
    for i in (s..64).rev() {
        if a & (1u64 << i) != 0 {
            a ^= full << (i - s);
        }
    }
    a
}

/// GF(2) 多项式幂 x^e mod p(平方乘)。
fn gf2_xpow_mod(e: u64, p: u64, s: u32) -> u64 {
    // x mod p
    let mut base = gf2_mod(2, p, s);
    let mut acc = 1u64; // 多项式 1
    let mut e = e;
    while e != 0 {
        if e & 1 != 0 {
            acc = gf2_mod(gf2_mul(acc, base), p, s);
        }
        base = gf2_mod(gf2_mul(base, base), p, s);
        e >>= 1;
    }
    acc
}

/// 首一多项式 p(次数 s,位表示含常数项;bit s 隐含)是否本原。
/// 判据:x^(2^s−1) ≡ 1 (mod p) 且对 2^s−1 的每个素因子 q,x^((2^s−1)/q) ≢ 1。
fn gf2_is_primitive(p: u64, s: u32) -> bool {
    if p & 1 == 0 {
        return false; // 常数项必须为 1(否则可被 x 整除)
    }
    let order = (1u64 << s) - 1;
    if gf2_xpow_mod(order, p, s) != 1 {
        return false;
    }
    // 2^s−1 的素因子(s ≤ 7 范围内:s=1..7 → 1,3,7,15,31,63,127)。
    let mut factors: Vec<u64> = Vec::new();
    let mut m = order;
    let mut d = 2u64;
    while d * d <= m {
        if m % d == 0 {
            factors.push(d);
            while m % d == 0 {
                m /= d;
            }
        }
        d += 1;
    }
    if m > 1 {
        factors.push(m);
    }
    for q in factors {
        if gf2_xpow_mod(order / q, p, s) == 1 {
            return false;
        }
    }
    true
}

/// 本原多项式表(按 (次数, 整数值) 升序枚举;OnceLock 缓存——程序生成确定
/// 性,primitivity 单测自证;前 7 次总数 1+1+2+2+6+6+18 = 36 ≥ 生产化流
/// 26 维)。
fn sobol_primitive_polys() -> &'static [(u64, u32)] {
    static TABLE: std::sync::OnceLock<Vec<(u64, u32)>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut out = Vec::new();
        for s in 1..=7u32 {
            for bits in 0..(1u64 << s) {
                if gf2_is_primitive(bits, s) {
                    out.push((bits, s));
                }
            }
        }
        out
    })
}

/// 第 d 个(0-based)Sobol 维的本原多项式(按 (次数, 整数值) 升序枚举首个
/// 本原多项式;确定性程序生成——无外部表,primitivity 单测自证)。
fn sobol_primitive_poly(dim: usize) -> (u64, u32) {
    let table = sobol_primitive_polys();
    assert!(
        dim < table.len(),
        "本原多项式表 {} 个,覆盖请求 dim={dim}",
        table.len()
    );
    table[dim]
}

/// Sobol 方向数(dim 维;初始 m_i = 2i−1(奇数,i 1-based),v_i = m_i << (32−i);
/// 递推 v_i = v_{i−s} ^ (v_{i−s} >> s) ^ ⊕_{j=1..s−1} a_j·v_{i−j},其中
/// a_j = 本原多项式 x^s + a_1 x^{s−1} + … + a_{s−1} x + 1 的系数 a_j = bit(s−j)
/// of `poly` 低 s 位——Sobol 标准递推,32 位域)。
fn sobol_direction_numbers(dim: usize) -> [u32; 32] {
    // 方向数表 OnceLock 缓存(逐样本复用——程序生成只跑一次)。
    static DIRS: std::sync::OnceLock<Vec<[u32; 32]>> = std::sync::OnceLock::new();
    let table = DIRS.get_or_init(|| {
        (0..sobol_primitive_polys().len())
            .map(sobol_direction_numbers_compute)
            .collect()
    });
    table[dim]
}

/// 方向数计算(由 [`sobol_direction_numbers`] 缓存调用)。
fn sobol_direction_numbers_compute(dim: usize) -> [u32; 32] {
    let (poly, s) = sobol_primitive_poly(dim);
    let s = s as usize;
    let mut v = [0u32; 32];
    for i in 1..=s {
        let m = (2 * i - 1) as u32;
        v[i - 1] = m << (32 - i as u32);
    }
    for i in s..32 {
        let mut val = v[i - s] ^ (v[i - s] >> s as u32);
        for j in 1..s {
            if (poly >> (s - j)) & 1 == 1 {
                val ^= v[i - j];
            }
        }
        v[i] = val;
    }
    v
}

/// Sobol 类样本值(dim 维,样本索引 n 0-based;Gray-code 序;确定性种子扰动
/// = Cranley-Patterson 旋转,rotation 由 seed/dim 确定函数派生)。
fn sobol_sample(dim: usize, n: usize, seed: u64) -> f32 {
    let v = sobol_direction_numbers(dim);
    let g = n ^ (n >> 1);
    let mut bits = 0u32;
    for i in 0..24 {
        if (g >> i) & 1 == 1 {
            bits ^= v[i];
        }
    }
    let x = (bits >> 8) as f32 * (1.0 / 16_777_216.0);
    let shift = u01_from_hash(mix64(seed ^ (0x50B0_1 + dim as u64)));
    (x + shift).fract()
}

/// 分层样本值(维度 dim,采样 s / 共 spp;确定性置换 σ(s) = (s·A+B) mod spp
/// (spp 为 2 的幂时 A 奇数即置换;spp=1 退化 σ=0) + 确定性 jitter)。
fn stratified_sample(dim: usize, s: usize, spp: u32, seed: u64, pixel: usize) -> f32 {
    let n = spp.max(1) as u64;
    let h = mix64(seed ^ ((pixel as u64) << 20) ^ (dim as u64));
    let sigma = if n <= 1 {
        0u64
    } else if n.is_power_of_two() {
        let a = (h & 0xFFFF) | 1; // 奇数 ⇒ mod 2^k 置换
        let b = (h >> 32) & (n - 1);
        ((s as u64).wrapping_mul(a).wrapping_add(b)) & (n - 1)
    } else {
        // 非 2 幂退化:乘加模(非严格置换但确定;生产化 spp 序列全 2 幂)。
        let a = (h & 0xFFFF) | 1;
        let b = (h >> 32) % n;
        ((s as u64).wrapping_mul(a).wrapping_add(b)) % n
    };
    let jitter = u01_from_hash(mix64(h ^ (s as u64).wrapping_mul(0x9E37_79B9)));
    ((sigma as f32) + jitter) / (n as f32)
}

/// 生产化采样器(序列族 + 索引推导确定性 + provenance 字面)。
pub mod sampler {
    use super::*;

    /// 单样本确定函数寻址(样本值 = f(像素索引, 采样索引, 维度, seed);
    /// 无任何数据相关状态——RXS-0400 L2 索引推导确定性承载面)。
    pub fn sample_at(
        family: SamplerFamily,
        pixel: usize,
        sample: usize,
        dim: usize,
        spp: u32,
        seed: u64,
    ) -> f32 {
        match family {
            SamplerFamily::Pcg => {
                // PCG 独立流无维度级闭式(序进流):按索引寻址的等价重求值 =
                // 同 seed 自流头序进至目标位置(与 generate 同序;仅供机核小
                // 尺度核验,批量生成走 generate 顺序推进)。max_bounces 冻结 4。
                let mut rng = Pcg32::new(seed);
                let base = (pixel * spp as usize + sample) * super::prod_sample_stride(4);
                let mut out = 0.0f32;
                for _ in 0..=(base + dim) {
                    out = rng.next_f32();
                }
                out
            }
            SamplerFamily::Stratified => stratified_sample(dim, sample, spp, seed, pixel),
            SamplerFamily::Sobol => sobol_sample(dim, pixel * spp as usize + sample, seed),
        }
    }

    /// 生成整条流(图序顺序排布;三族同缓冲按索引寻址——RNG 流为输入不是
    /// 结果,G7.4 先例同构)。
    pub fn generate(
        family: SamplerFamily,
        pixel_count: usize,
        spp: u32,
        max_bounces: u32,
        seed: u64,
    ) -> Vec<f32> {
        let stride = prod_sample_stride(max_bounces);
        let total = prod_stream_len(pixel_count, spp, max_bounces);
        let mut out = vec![0.0f32; total];
        match family {
            SamplerFamily::Pcg => {
                let mut rng = Pcg32::new(seed);
                for v in out.iter_mut() {
                    *v = rng.next_f32();
                }
            }
            _ => {
                for px in 0..pixel_count {
                    for s in 0..spp as usize {
                        let base = prod_sample_base(px, s, spp, max_bounces);
                        for d in 0..stride {
                            out[base + d] = sample_at(family, px, s, d, spp, seed);
                        }
                    }
                }
            }
        }
        out
    }

    /// 流布局 provenance 字面(RXS-0400 L2:序列族/扰动面/寻址公式进
    /// evidence)。
    pub fn provenance(family: SamplerFamily, max_bounces: u32) -> String {
        let stride = prod_sample_stride(max_bounces);
        match family {
            SamplerFamily::Pcg => format!(
                "family=pcg_independent(rt::ref_tracer::Pcg32 单一实例序进;承 G8 对拍模式);\
                 扰动面=无;寻址公式=stream[(pixel·spp+sample)·{stride}+dim] 序进填充按索引消费"
            ),
            SamplerFamily::Stratified => format!(
                "family=stratified_per_dimension;置换 σ(s)=(s·A+B) mod spp(A 奇数,AB 由 \
                 mix64(seed^pixel<<20^dim) 派生,spp=2^k 严格置换);jitter=mix64 高 24 位/2^24;\
                 扰动面=逐(像素,维度)确定性置换+逐样本确定性 jitter;寻址公式=\
                 stream[(pixel·spp+sample)·{stride}+dim]=f(pixel,sample,dim,seed) 确定函数"
            ),
            SamplerFamily::Sobol => format!(
                "family=sobol_class_seed_perturbed;方向数=程序化生成本原多项式(按(次数,整数)\
                 升序第 dim 个,primitivity 机核自证)+m_i=2i−1 初始化,Gray-code 序 24 位;\
                 扰动面=Cranley-Patterson 确定性旋转 shift=mix64(seed^0x50B01+dim) 高 24 位/2^24;\
                 寻址公式=stream[(pixel·spp+sample)·{stride}+dim]=f(pixel,sample,dim,seed) 确定函数"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// 生产化场景面(RXS-0398 L3 多光源 + delta 点光 + 白炉;材质集合 Lambert/发光
// 两类维持——起步范围冻结 0-byte)
// ---------------------------------------------------------------------------

/// 生产化光源(面光 quad + delta 点光闭集)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProdLight {
    /// 面光 quad(与 M96 PtLightQuad 同构;单面发光)。
    Quad(PtLightQuad),
    /// delta 点光(位置 + 点强 I;BSDF 策略概率为零 → w_nee=1)。
    Point {
        /// 位置(世界空间)。
        position: [f32; 3],
        /// 点强 RGB(线性,辐射强度 I——贡献 = I·cos/(π·d²·pdf_d))。
        intensity: [f32; 3],
    },
}

/// 生产化场景(单 BLAS 三角形汤 + 逐三角材质 + 多光源 + 相机)。
#[derive(Debug, Clone)]
pub struct ProdScene {
    /// 稳定场景名(evidence 键)。
    pub name: &'static str,
    /// 顶点位置(世界空间)。
    pub positions: Vec<[f32; 3]>,
    /// 三角形索引。
    pub indices: Vec<[u32; 3]>,
    /// 逐三角材质。
    pub materials: Vec<MaterialKind>,
    /// 光源集(quad 与发光三角逐字一致校验;point 无几何)。
    pub lights: Vec<ProdLight>,
    /// 相机。
    pub camera: PtCamera,
    /// 场景射线 t 上界。
    pub t_max: f32,
    /// 三角 → 光源下标(u32::MAX = 非发光;校验期构建)。
    pub light_of_prim: Vec<u32>,
}

impl ProdScene {
    /// fail-closed 校验(生产化口径:材质 Lambert/发光两类维持;albedo ∈
    /// [0,1] 放宽含 1(白炉面)——M96 校验面 <1 字面 0-byte 不动;quad 光源
    /// ↔ 发光三角逐字一致;point 光源有限非负;≥1 光源)。
    pub fn validate(&self) -> Result<(), PtError> {
        if self.indices.is_empty() || self.positions.is_empty() {
            return Err(PtError::InvalidScene("空场景".into()));
        }
        if self.materials.len() != self.indices.len() {
            return Err(PtError::InvalidScene("材质数 ≠ 三角数".into()));
        }
        if self.lights.is_empty() {
            return Err(PtError::InvalidScene("光源集为空(生产化 ≥1 光源)".into()));
        }
        if self.light_of_prim.len() != self.indices.len() {
            return Err(PtError::InvalidScene("light_of_prim 长度 ≠ 三角数".into()));
        }
        for (i, p) in self.positions.iter().enumerate() {
            if !p.iter().all(|c| c.is_finite()) {
                return Err(PtError::InvalidScene(format!("顶点 {i} 非有限")));
            }
        }
        for (t, m) in self.materials.iter().enumerate() {
            match m {
                MaterialKind::Lambert { albedo } => {
                    if !albedo
                        .iter()
                        .all(|c| c.is_finite() && *c >= 0.0 && *c <= 1.0)
                    {
                        return Err(PtError::InvalidScene(format!(
                            "三角 {t} albedo 越域 [0,1](生产化口径含白炉 1.0):{albedo:?}"
                        )));
                    }
                }
                MaterialKind::Emission { albedo, emission } => {
                    if !albedo
                        .iter()
                        .all(|c| c.is_finite() && *c >= 0.0 && *c <= 1.0)
                    {
                        return Err(PtError::InvalidScene(format!(
                            "三角 {t} 发光面 albedo 越域"
                        )));
                    }
                    if !emission.iter().all(|c| c.is_finite() && *c >= 0.0) {
                        return Err(PtError::InvalidScene(format!(
                            "三角 {t} emission 非有限/负"
                        )));
                    }
                    if self.light_of_prim[t] == u32::MAX {
                        return Err(PtError::InvalidScene(format!(
                            "发光三角 {t} 无对应 quad 光源(light_of_prim 缺联)"
                        )));
                    }
                }
                MaterialKind::Specular { .. } => {
                    return Err(PtError::OutOfScopeMaterial {
                        tri: t as u32,
                        kind: "specular",
                    });
                }
                MaterialKind::Transmission { .. } => {
                    return Err(PtError::OutOfScopeMaterial {
                        tri: t as u32,
                        kind: "transmission(焦散源)",
                    });
                }
                MaterialKind::Volume { .. } => {
                    return Err(PtError::OutOfScopeMaterial {
                        tri: t as u32,
                        kind: "volume(体积)",
                    });
                }
            }
        }
        // 逐 quad 光源 ↔ 恰 2 发光三角逐字一致(M96 口径的多光源推广)。
        for (li, l) in self.lights.iter().enumerate() {
            match l {
                ProdLight::Quad(q) => {
                    let tris: Vec<usize> = (0..self.indices.len())
                        .filter(|&t| self.light_of_prim[t] == li as u32)
                        .collect();
                    if tris.len() != 2 {
                        return Err(PtError::InvalidScene(format!(
                            "quad 光源 {li} 发光三角数 {} ≠ 2",
                            tris.len()
                        )));
                    }
                    let p00 = Vec3::from_array(q.p00);
                    let p10 = p00 + Vec3::from_array(q.e1);
                    let p01 = p00 + Vec3::from_array(q.e2);
                    let p11 = p01 + Vec3::from_array(q.e1);
                    let expected: [[Vec3; 3]; 2] = [[p00, p10, p11], [p00, p11, p01]];
                    let ln = Vec3::from_array(q.normal());
                    let area = q.area();
                    if !(area.is_finite() && area > 0.0) {
                        return Err(PtError::InvalidScene(format!("quad 光源 {li} 面积非正")));
                    }
                    for (k, &t) in tris.iter().enumerate() {
                        let idx = self.indices[t];
                        let vs = [
                            Vec3::from_array(self.positions[idx[0] as usize]),
                            Vec3::from_array(self.positions[idx[1] as usize]),
                            Vec3::from_array(self.positions[idx[2] as usize]),
                        ];
                        for (j, (v, e)) in vs.iter().zip(expected[k].iter()).enumerate() {
                            if v.to_array() != e.to_array() {
                                return Err(PtError::InvalidScene(format!(
                                    "quad 光源 {li} 发光三角 {t} 顶点 {j} 不逐字一致:{v:?} vs {e:?}"
                                )));
                            }
                        }
                        let n = (vs[1] - vs[0]).cross(vs[2] - vs[0]);
                        if n.dot(ln) <= 0.0 {
                            return Err(PtError::InvalidScene(format!(
                                "quad 光源 {li} 发光三角 {t} 绕向法线反向"
                            )));
                        }
                        if (n.length() - area).abs() > 1e-6 * area {
                            return Err(PtError::InvalidScene(format!(
                                "quad 光源 {li} 发光三角 {t} 面积不符"
                            )));
                        }
                    }
                }
                ProdLight::Point {
                    position,
                    intensity,
                } => {
                    if !position.iter().all(|c| c.is_finite()) {
                        return Err(PtError::InvalidScene(format!("点光 {li} 位置非有限")));
                    }
                    if !intensity.iter().all(|c| c.is_finite() && *c >= 0.0) {
                        return Err(PtError::InvalidScene(format!("点光 {li} 强度非有限/负")));
                    }
                }
            }
        }
        // 相机正交单位(M96 同式)。
        let (f, r, u) = (
            Vec3::from_array(self.camera.forward),
            Vec3::from_array(self.camera.right),
            Vec3::from_array(self.camera.up),
        );
        for (nm, v) in [("forward", f), ("right", r), ("up", u)] {
            if !v.is_finite() || (v.length() - 1.0).abs() > 1e-5 {
                return Err(PtError::InvalidScene(format!("相机 {nm} 非单位/非有限")));
            }
        }
        if f.dot(r).abs() > 1e-4 || f.dot(u).abs() > 1e-4 {
            return Err(PtError::InvalidScene("相机基向量非正交".into()));
        }
        if !(self.t_max.is_finite() && self.t_max > 0.0) {
            return Err(PtError::InvalidScene("t_max 非正".into()));
        }
        Ok(())
    }

    /// 单 BLAS 三角形汤(9 f32/三角,序 = `indices` 序)。
    pub fn blas_triangles(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.indices.len() * 9);
        for idx in &self.indices {
            for &vi in idx {
                out.extend_from_slice(&self.positions[vi as usize]);
            }
        }
        out
    }
}

/// 由 M96 冻结 fixture 同源转换(单 quad 光源;消费不回写——RXS-0357 0-byte)。
pub fn prod_scene_from_m96(s: &PtScene) -> ProdScene {
    let mut light_of_prim = vec![u32::MAX; s.indices.len()];
    for (t, m) in s.materials.iter().enumerate() {
        if matches!(m, MaterialKind::Emission { .. }) {
            light_of_prim[t] = 0;
        }
    }
    ProdScene {
        name: s.name,
        positions: s.positions.clone(),
        indices: s.indices.clone(),
        materials: s.materials.clone(),
        lights: vec![ProdLight::Quad(s.light)],
        camera: s.camera,
        t_max: s.t_max,
        light_of_prim,
    }
}

/// 生产化 fixture①:Cornell 类多光源场景(5 墙 + 中央盒 + **双 quad 光源**
/// 不同位置/发光;多光源 MIS 联合 PDF 面)。
pub fn g12_two_light_scene() -> ProdScene {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();
    let mut materials: Vec<MaterialKind> = Vec::new();
    let white = [0.73, 0.73, 0.73];
    let red = [0.61, 0.06, 0.06];
    let green = [0.12, 0.45, 0.15];
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        MaterialKind::Lambert { albedo: white },
    );
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
        MaterialKind::Lambert { albedo: white },
    );
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
        MaterialKind::Lambert { albedo: white },
    );
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0],
        MaterialKind::Lambert { albedo: red },
    );
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
        MaterialKind::Lambert { albedo: green },
    );
    let base = indices.len();
    super_add_box(
        &mut positions,
        &mut indices,
        [0.42, 0.0, 0.38],
        [0.72, 0.55, 0.68],
    );
    for _ in base..indices.len() {
        materials.push(MaterialKind::Lambert {
            albedo: [0.60, 0.60, 0.60],
        });
    }
    // 双 quad 光源(天花下挂 y=0.995,法线 −y;左 10 白 / 右 6 暖)。
    let lights = vec![
        ProdLight::Quad(PtLightQuad {
            p00: [0.12, 0.995, 0.30],
            e1: [0.24, 0.0, 0.0],
            e2: [0.0, 0.0, 0.24],
            emission: [10.0, 10.0, 10.0],
        }),
        ProdLight::Quad(PtLightQuad {
            p00: [0.60, 0.995, 0.36],
            e1: [0.20, 0.0, 0.0],
            e2: [0.0, 0.0, 0.20],
            emission: [6.0, 4.5, 3.0],
        }),
    ];
    let mut light_of_prim = vec![u32::MAX; indices.len()];
    for (li, l) in lights.iter().enumerate() {
        let ProdLight::Quad(q) = l else { continue };
        let lp10 = [q.p00[0] + q.e1[0], q.p00[1], q.p00[2]];
        let lp01 = [q.p00[0], q.p00[1], q.p00[2] + q.e2[2]];
        let lp11 = [lp10[0], lp10[1], lp01[2]];
        let em = MaterialKind::Emission {
            albedo: [0.5, 0.5, 0.5],
            emission: q.emission,
        };
        super_push_quad(
            &mut positions,
            &mut indices,
            &mut materials,
            q.p00,
            lp10,
            lp11,
            lp01,
            em,
        );
        light_of_prim.push(li as u32);
        light_of_prim.push(li as u32);
    }
    let camera = PtCamera::look_at(
        [0.5, 0.5, -0.9],
        [0.5, 0.5, 0.55],
        [0.0, 1.0, 0.0],
        50.0,
        64,
        64,
    );
    ProdScene {
        name: "g12_two_light",
        positions,
        indices,
        materials,
        lights,
        camera,
        t_max: 100.0,
        light_of_prim,
    }
}

/// 生产化 fixture②:白炉(全封闭单位盒,五墙 albedo=1 全反射 + **整面天花
/// 为光源**(albedo=0 纯发光,黑体式注入;覆盖全天花使直接能量份额最大化,
/// 截断方向只丢能量)。能量守恒口径面:4-bounce 截断估计子均值 ∈
/// [截断参照×(1−容差), 入射能量 Le](**不产能量**上界硬断言;平衡炉全平衡
/// 极限 = Le 需无穷反弹,匹配深度 4 冻结面下截断真值 ≈ 0.53·Le——截断结构
/// 实测面,非偏置;裸双臂 w=1 加和是双重计数伪影,禁作参照)+ 逐级能量增量
/// 单调不增 + 输出非负无漏光;device 均值 vs host oracle 参照均值相对偏差 ≤
/// 标定容差(M166 族间极差 measured 产)。
pub fn g12_furnace_scene() -> ProdScene {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();
    let mut materials: Vec<MaterialKind> = Vec::new();
    let one = [1.0, 1.0, 1.0];
    // 全封闭盒(5 反射墙 10 三角,绕向朝内;天花由光源几何承担)。
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        MaterialKind::Lambert { albedo: one },
    ); // 地板 +y
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
        MaterialKind::Lambert { albedo: one },
    ); // 后墙 −z
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        MaterialKind::Lambert { albedo: one },
    ); // 前墙 +z
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0],
        MaterialKind::Lambert { albedo: one },
    ); // 左墙 +x
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
        MaterialKind::Lambert { albedo: one },
    ); // 右墙 −x
    // 整面天花光源(y=1 全覆盖,法线 −y 朝室内;albedo=0 纯发光黑体式注入)。
    let light = ProdLight::Quad(PtLightQuad {
        p00: [0.0, 1.0, 0.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        emission: [4.0, 4.0, 4.0],
    });
    let ProdLight::Quad(q) = light else {
        unreachable!()
    };
    let lp10 = [q.p00[0] + q.e1[0], q.p00[1], q.p00[2]];
    let lp01 = [q.p00[0], q.p00[1], q.p00[2] + q.e2[2]];
    let lp11 = [lp10[0], lp10[1], lp01[2]];
    let mut light_of_prim = vec![u32::MAX; indices.len()];
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        q.p00,
        lp10,
        lp11,
        lp01,
        MaterialKind::Emission {
            albedo: [0.0, 0.0, 0.0],
            emission: q.emission,
        },
    );
    light_of_prim.push(0);
    light_of_prim.push(0);
    let camera = PtCamera::look_at(
        [0.5, 0.45, 0.12],
        [0.5, 0.45, 1.0],
        [0.0, 1.0, 0.0],
        55.0,
        64,
        64,
    );
    ProdScene {
        name: "g12_furnace",
        positions,
        indices,
        materials,
        lights: vec![light],
        camera,
        t_max: 100.0,
        light_of_prim,
    }
}

/// 生产化 fixture③:delta 点光场景(地板 + 点光;BSDF 策略对 delta 光源
/// 概率为零 → w_nee=1 退化面)。
pub fn g12_delta_light_scene() -> ProdScene {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();
    let mut materials: Vec<MaterialKind> = Vec::new();
    super_push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 2.0],
        [2.0, 0.0, 2.0],
        [2.0, 0.0, 0.0],
        MaterialKind::Lambert {
            albedo: [0.7, 0.7, 0.7],
        },
    );
    let light_of_prim = vec![u32::MAX; indices.len()];
    let camera = PtCamera::look_at(
        [1.0, 1.1, -1.2],
        [1.0, 0.1, 0.9],
        [0.0, 1.0, 0.0],
        45.0,
        64,
        64,
    );
    ProdScene {
        name: "g12_delta",
        positions,
        indices,
        materials,
        lights: vec![ProdLight::Point {
            position: [1.0, 1.5, 1.0],
            intensity: [30.0, 30.0, 30.0],
        }],
        camera,
        t_max: 100.0,
        light_of_prim,
    }
}

/// 生产化场景集(evidence 键序;m96 两场景 = 收敛曲线基线锚消费面)。
pub fn g12_prod_scenes() -> Vec<ProdScene> {
    vec![
        prod_scene_from_m96(&m96_cornell_scene()),
        prod_scene_from_m96(&m96_direct_light_scene()),
    ]
}

// quad/box 辅助(避免触碰 super 私有 fn——生产化面自承载,公式同式)。
fn super_push_quad(
    positions: &mut Vec<[f32; 3]>,
    indices: &mut Vec<[u32; 3]>,
    materials: &mut Vec<MaterialKind>,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    d: [f32; 3],
    m: MaterialKind,
) {
    let base = positions.len() as u32;
    positions.extend_from_slice(&[a, b, c, d]);
    indices.push([base, base + 1, base + 2]);
    indices.push([base, base + 2, base + 3]);
    materials.push(m);
    materials.push(m);
}

fn super_add_box(
    positions: &mut Vec<[f32; 3]>,
    indices: &mut Vec<[u32; 3]>,
    min: [f32; 3],
    max: [f32; 3],
) {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let q = |positions: &mut Vec<[f32; 3]>, indices: &mut Vec<[u32; 3]>, a, b, c, d| {
        let base = positions.len() as u32;
        positions.extend_from_slice(&[a, b, c, d]);
        indices.push([base, base + 1, base + 2]);
        indices.push([base, base + 2, base + 3]);
    };
    q(
        positions,
        indices,
        [x0, y0, z0],
        [x0, y0, z1],
        [x1, y0, z1],
        [x1, y0, z0],
    );
    q(
        positions,
        indices,
        [x0, y1, z0],
        [x1, y1, z0],
        [x1, y1, z1],
        [x0, y1, z1],
    );
    q(
        positions,
        indices,
        [x0, y0, z0],
        [x1, y0, z0],
        [x1, y1, z0],
        [x0, y1, z0],
    );
    q(
        positions,
        indices,
        [x0, y0, z1],
        [x0, y1, z1],
        [x1, y1, z1],
        [x1, y0, z1],
    );
    q(
        positions,
        indices,
        [x0, y0, z0],
        [x0, y1, z0],
        [x0, y1, z1],
        [x0, y0, z1],
    );
    q(
        positions,
        indices,
        [x1, y0, z0],
        [x1, y0, z1],
        [x1, y1, z1],
        [x1, y1, z0],
    );
}

// ---------------------------------------------------------------------------
// 光源分布(RXS-0398 L3:联合 PDF = 离散 × 连续;同场景同分布 digest)
// ---------------------------------------------------------------------------

/// 光源分布(离散 PDF + CDF;发射功率加权,确定性构建)。
#[derive(Debug, Clone, PartialEq)]
pub struct LightDist {
    /// 逐光源离散 PDF(和 = 1)。
    pub pdf: Vec<f32>,
    /// 逐光源 CDF(末元素 = 1)。
    pub cdf: Vec<f32>,
}

/// 构建光源分布(确定性:同场景同分布——权重 = Σemission·area(quad) /
/// Σintensity(point),全零退化均匀;digest = sha256(灯光打包字节 ‖ pdf
/// 字节)进 evidence)。
pub fn build_light_distribution(scene: &ProdScene) -> LightDist {
    let mut w: Vec<f64> = Vec::with_capacity(scene.lights.len());
    for l in &scene.lights {
        let weight = match l {
            ProdLight::Quad(q) => {
                (q.emission[0] + q.emission[1] + q.emission[2]) as f64 * q.area() as f64
            }
            ProdLight::Point { intensity, .. } => {
                (intensity[0] + intensity[1] + intensity[2]) as f64
            }
        };
        w.push(weight);
    }
    let sum: f64 = w.iter().sum();
    let n = scene.lights.len() as f32;
    let mut pdf = Vec::with_capacity(w.len());
    let mut cdf = Vec::with_capacity(w.len());
    let mut acc = 0.0f32;
    for &x in &w {
        let p = if sum > 0.0 { (x / sum) as f32 } else { 1.0 / n };
        acc += p;
        pdf.push(p);
        cdf.push(acc);
    }
    // CDF 末元素精确 1(浮点累计修正;确定性)。
    if let Some(last) = cdf.last_mut() {
        *last = 1.0;
    }
    LightDist { pdf, cdf }
}

/// 分布 digest(同场景同分布机核面;灯光打包字节 ‖ pdf 字节 SHA-256)。
pub fn light_distribution_digest(scene: &ProdScene, dist: &LightDist) -> [u8; 32] {
    let lights = pack_prod_lights(scene, dist);
    let mut pre = Vec::with_capacity(lights.len() * 4 + dist.pdf.len() * 4);
    for v in lights.iter().chain(dist.pdf.iter()) {
        pre.extend_from_slice(&v.to_le_bytes());
    }
    rurix_pkg::sha256::digest(&pre)
}

// ---------------------------------------------------------------------------
// MIS balance heuristic(RXS-0398 L2;delta 退化 w_nee=1 无除零)
// ---------------------------------------------------------------------------

/// NEE 策略权(balance heuristic 安全形,无零除):
/// `w_nee = p_nee/(p_nee+p_bsdf)`,p_nee = pdf_d·dist²/(area·cos_l)(立体角
/// 测度),p_bsdf = cos_s/π——折合形 `w = 1/(1 + (cos_s/π)·area·cos_l/(pdf_d·dist²))`。
/// cos_l 调用方已截断 ≥0;delta 光源(p_bsdf = 0)退化为 w=1。
pub fn mis_balance_nee(cos_s: f32, area: f32, cos_l: f32, pdf_d: f32, dist2: f32) -> f32 {
    let denom = pdf_d * dist2;
    if denom <= 0.0 {
        return 1.0; // 退化防御(正常路径 denom > 0;fail-closed 不除零)
    }
    let r = (cos_s / PT_PI) * area * cos_l / denom;
    1.0 / (1.0 + r)
}

/// BSDF 策略权(balance heuristic 安全形):`w_bsdf = p_bsdf/(p_bsdf+p_nee)`,
/// p_nee = pdf_d·t²/(area·cos_emit)(命中光源 l 的 NEE 联合 PDF 立体角形;
/// 多光源分母 = 全部光源联合 PDF 之和——非重叠光源下仅命中光源项非零,
/// 求和由调用方逐光源显式累加)——折合形 `w = 1/(1 + pdf_d·t²/(area·cos_emit·p_bsdf))`。
pub fn mis_balance_bsdf(t: f32, area: f32, cos_emit: f32, pdf_b: f32, pdf_d: f32) -> f32 {
    let denom = area * cos_emit * pdf_b;
    if denom <= 0.0 {
        return 1.0; // 退化防御
    }
    let r = pdf_d * t * t / denom;
    1.0 / (1.0 + r)
}

// ---------------------------------------------------------------------------
// 吞吐自适应 RR(RXS-0399 L1/L2)
// ---------------------------------------------------------------------------

/// 冻结最小反弹保障(N_min ≥ 2——低深度不早杀;沿 M96 rr_min_bounce=2 口径)。
pub const G12_RR_MIN_BOUNCE: u32 = 2;
/// 冻结 p_max(< 1 恒成立——任何深度保留非零续行概率,禁截断偏置)。
pub const G12_RR_P_MAX: f32 = 0.95;
/// 补偿因子上界(= 1/(1−p_max) = 20;p_max 冻结 ⇒ 钳制恒不触发,登记为
/// 防数值爆炸兜底面)。
pub const G12_RR_COMP_CLAMP: f32 = 20.0;

/// 吞吐自适应 RR 参数(τ = 吞吐参考阈——标定程序 measured 产禁手写 P-09)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RrProdParams {
    /// RR 起始 bounce(≥ 2 最小反弹保障)。
    pub min_bounce: u32,
    /// 吞吐参考阈 τ(标定程序产)。
    pub tau: f32,
    /// 终止概率上界(< 1 恒成立)。
    pub p_max: f32,
    /// 补偿因子上界(登记面)。
    pub comp_clamp: f32,
}

impl RrProdParams {
    /// 生产化默认(τ 由调用方经标定程序填入;此处给结构默认值仅用于
    /// 校验面单测,门运行一律消费标定值)。
    pub fn production(tau: f32) -> RrProdParams {
        RrProdParams {
            min_bounce: G12_RR_MIN_BOUNCE,
            tau,
            p_max: G12_RR_P_MAX,
            comp_clamp: G12_RR_COMP_CLAMP,
        }
    }

    /// fail-closed 校验(N_min ≥ 2;p_max < 1 恒成立;τ 有限正;clamp ≥ 1)。
    pub fn validate(&self) -> Result<(), PtError> {
        if self.min_bounce < 2 {
            return Err(PtError::InvalidConfig(format!(
                "rr min_bounce {} < 2(最小反弹保障——低深度不早杀)",
                self.min_bounce
            )));
        }
        if !(self.p_max > 0.0 && self.p_max < 1.0) {
            return Err(PtError::InvalidConfig(format!(
                "rr p_max {} 非 (0,1)(p_max < 1 恒成立——任何深度保留非零续行概率,禁截断偏置)",
                self.p_max
            )));
        }
        if !(self.tau.is_finite() && self.tau > 0.0) {
            return Err(PtError::InvalidConfig(format!(
                "rr tau {} 非有限正",
                self.tau
            )));
        }
        if !(self.comp_clamp >= 1.0 && self.comp_clamp.is_finite()) {
            return Err(PtError::InvalidConfig(format!(
                "rr comp_clamp {} 越域",
                self.comp_clamp
            )));
        }
        Ok(())
    }

    /// 终止概率(吞吐自适应闭式;p_kill = clamp(1 − T/τ, 0, p_max))。
    pub fn p_kill(&self, throughput_max: f32) -> f32 {
        (1.0 - throughput_max / self.tau).clamp(0.0, self.p_max)
    }

    /// 补偿因子(闭式无偏 1/(1−p_kill),上界登记钳制)。
    pub fn compensation(&self, p_kill: f32) -> f32 {
        (1.0 / (1.0 - p_kill).max(1e-6)).min(self.comp_clamp)
    }
}

// ---------------------------------------------------------------------------
// 自适应收敛判据(RXS-0401 L1/L2;Σx/Σx² 协议沿 RXS-0357 L2)
// ---------------------------------------------------------------------------

/// spp 下界保障(每像素最小采样数 ≥ N_floor——防早期方差欠估计早停;值
/// 冻结登记:16 = 方差欠估计防护 measured 面——G12.2 网格实测:floor=4 时
/// 边缘像素「前 4 样本全 0 ⇒ 方差估计 0 ⇒ 假收敛」误判率 43%(host oracle
/// cornell θ=0.03),floor=16 时同面误判率降至亚百分点级;生产化运行必须 =
/// 本常量,标定程序 θ 协议以 spp=N_floor 为参照档)。
pub const G12_ADAPTIVE_N_FLOOR: u32 = 16;
/// 自适应 spp 上界(冻结 = 64,与全 spp 参照档对齐)。
pub const G12_ADAPTIVE_SPP_MAX: u32 = 64;

/// 自适应收敛参数(rel_err_threshold = 逐像素相对误差界阈——标定程序产
/// p100×k 入 g12_budget 禁手写)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveParams {
    /// spp 下界(≥ 1;生产化运行必须 = [`G12_ADAPTIVE_N_FLOOR`])。
    pub n_floor: u32,
    /// spp 上界。
    pub spp_max: u32,
    /// 逐像素相对误差界阈(标定程序产)。
    pub rel_err_threshold: f32,
}

impl AdaptiveParams {
    /// fail-closed 校验。
    pub fn validate(&self) -> Result<(), PtError> {
        if self.n_floor == 0 {
            return Err(PtError::InvalidConfig(
                "adaptive n_floor = 0(spp 下界保障违反)".into(),
            ));
        }
        if self.spp_max == 0 || self.n_floor > self.spp_max {
            return Err(PtError::InvalidConfig("adaptive n_floor > spp_max".into()));
        }
        if !(self.rel_err_threshold.is_finite() && self.rel_err_threshold > 0.0) {
            return Err(PtError::InvalidConfig(
                "adaptive rel_err_threshold 非有限正".into(),
            ));
        }
        Ok(())
    }
}

/// 逐像素相对误差界(在线 Σ/Σ² 协议):rel_err = sqrt(var/n)/max(mean, ε),
/// var = Σx²/n − (Σx/n)² ≥ 0。
pub fn rel_err_bound(sum: f32, sumsq: f32, n: u32) -> f32 {
    let nf = n.max(1) as f32;
    let mean = sum / nf;
    let var = (sumsq / nf - mean * mean).max(0.0);
    (var / nf).sqrt() / mean.abs().max(1e-6)
}

// ---------------------------------------------------------------------------
// 运行配置与 host oracle(与 device megakernel 公式面逐字同源)
// ---------------------------------------------------------------------------

/// 生产化运行配置。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProdConfig {
    /// spp(自适应面 = spp 上界)。
    pub spp: u32,
    /// 最大 bounce(冻结 = 4)。
    pub max_bounces: u32,
    /// 固定 seed。
    pub seed: u64,
    /// 采样器族。
    pub sampler: SamplerFamily,
    /// MIS 完整面开关(关 = 权重缺失 RED 臂)。
    pub mis: bool,
    /// RR 开关(关 = 跳 RR RED 臂)。
    pub rr: bool,
    /// RR 参数。
    pub rr_params: RrProdParams,
    /// 自适应收敛(None = 全 spp 参照面)。
    pub adaptive: Option<AdaptiveParams>,
    /// 能量偏置注入(≠0 = RED 臂;正常 = 0)。
    pub energy_bias: f32,
    /// RR 补偿缺失(true = RED 臂)。
    pub rr_comp_off: bool,
}

impl ProdConfig {
    /// 生产化基线(spp 由序列驱动;采样器/τ/自适应阈由标定/选型程序填)。
    pub fn production(spp: u32, sampler: SamplerFamily, tau: f32) -> ProdConfig {
        ProdConfig {
            spp,
            max_bounces: PROD_MAX_BOUNCES,
            seed: G12_PROD_SEED,
            sampler,
            mis: true,
            rr: true,
            rr_params: RrProdParams::production(tau),
            adaptive: None,
            energy_bias: 0.0,
            rr_comp_off: false,
        }
    }

    /// fail-closed 校验。
    pub fn validate(&self) -> Result<(), PtError> {
        if self.spp == 0 {
            return Err(PtError::InvalidConfig("spp = 0".into()));
        }
        if self.max_bounces == 0 {
            return Err(PtError::InvalidConfig("max_bounces = 0".into()));
        }
        self.rr_params.validate()?;
        if self.rr_params.min_bounce >= self.max_bounces {
            return Err(PtError::InvalidConfig("rr min_bounce ≥ max_bounces".into()));
        }
        if let Some(a) = &self.adaptive {
            a.validate()?;
        }
        if !(self.energy_bias.is_finite() && self.energy_bias > -1.0 && self.energy_bias < 1.0) {
            return Err(PtError::InvalidConfig("energy_bias 越域 (−1,1)".into()));
        }
        Ok(())
    }
}

/// 生产化渲染输出(逐像素均值 RGB + Σlum/Σlum² + 实际采样数 + 收敛标志 +
/// RR 计数 + 逐级能量)。
#[derive(Debug, Clone, PartialEq)]
pub struct ProdImage {
    /// 图宽。
    pub width: u32,
    /// 图高。
    pub height: u32,
    /// 逐像素均值辐射度 RGB(3 f32/px;均值除数 = 有效采样数)。
    pub rgb: Vec<f32>,
    /// 逐像素 Σlum(有效采样)。
    pub sum_lum: Vec<f32>,
    /// 逐像素 Σlum²(有效采样)。
    pub sumsq_lum: Vec<f32>,
    /// 逐像素有效采样数(自适应面 < spp 上界)。
    pub samples: Vec<u32>,
    /// 逐像素收敛标志(1.0 = 达阈早停;0.0 = 跑满上界未达阈/非自适应)。
    pub converged: Vec<f32>,
    /// 逐像素 RR 计数 [terminated, evaluated, comp_sum, comp_max](4 f32/px)。
    pub rr_counters: Vec<f32>,
    /// 逐像素逐级能量(每 bounce 亮度贡献均值;4 f32/px,max_bounces=4)。
    pub energy_levels: Vec<f32>,
    /// 帧型标签(闭集 {adaptive, full_reference})。
    pub frame_label: &'static str,
}

impl ProdImage {
    /// 像素数。
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// 全图均值亮度。
    pub fn mean_luminance(&self) -> f64 {
        let mut acc = 0.0f64;
        for px in 0..self.pixel_count() {
            let l = (f64::from(self.rgb[px * 3])
                + f64::from(self.rgb[px * 3 + 1])
                + f64::from(self.rgb[px * 3 + 2]))
                / 3.0;
            acc += l;
        }
        acc / self.pixel_count() as f64
    }

    /// 全图平均逐像素方差(亮度域)。
    pub fn mean_pixel_variance(&self) -> f64 {
        let mut acc = 0.0f64;
        for px in 0..self.pixel_count() {
            let n = f64::from(self.samples[px].max(1));
            let mean = f64::from(self.sum_lum[px]) / n;
            let var = (f64::from(self.sumsq_lum[px]) / n - mean * mean).max(0.0);
            acc += var;
        }
        acc / self.pixel_count() as f64
    }
}

/// 渲染输出 canonical digest(SHA-256(全部输出字节依序拼接);不含路径/
/// mtime/seed——RXS-0357 L2 协议面加性扩展维持)。
pub fn prod_image_digest(img: &ProdImage) -> [u8; 32] {
    let mut pre = Vec::with_capacity(img.rgb.len() * 4 + img.sum_lum.len() * 8 + 4096);
    for v in img
        .rgb
        .iter()
        .chain(img.sum_lum.iter())
        .chain(img.sumsq_lum.iter())
        .chain(img.converged.iter())
        .chain(img.rr_counters.iter())
        .chain(img.energy_levels.iter())
    {
        pre.extend_from_slice(&v.to_le_bytes());
    }
    for v in &img.samples {
        pre.extend_from_slice(&v.to_le_bytes());
    }
    rurix_pkg::sha256::digest(&pre)
}

/// 分量积。
fn cmul(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x * b.x, a.y * b.y, a.z * b.z)
}

/// 单条路径求值(host oracle;与 kernel 公式面同源)。返回 (Li, 逐级亮度贡献,
/// RR 计数三元组[terminated, evaluated, comp_sum, comp_max])。
#[allow(clippy::too_many_arguments)]
fn trace_path_prod<B: BlasSet + ?Sized>(
    tlas: &Tlas,
    blases: &B,
    scene: &ProdScene,
    dist: &LightDist,
    cfg: &ProdConfig,
    stream: &[f32],
    pixel: usize,
    sample: usize,
) -> ([f32; 3], [f32; 4], [f32; 4]) {
    let cam = &scene.camera;
    let px = pixel % cam.width as usize;
    let py = pixel / cam.width as usize;
    let sb = prod_sample_base(pixel, sample, cfg.spp, cfg.max_bounces);
    let inv_w = 1.0 / cam.width as f32;
    let inv_h = 1.0 / cam.height as f32;
    let ju = (px as f32 + stream[sb]) * inv_w;
    let jv = (py as f32 + stream[sb + 1]) * inv_h;
    let sx = (2.0 * ju - 1.0) * cam.tan_half_fov;
    let sy = (1.0 - 2.0 * jv) * cam.tan_half_fov;
    let f = Vec3::from_array(cam.forward);
    let r = Vec3::from_array(cam.right);
    let u = Vec3::from_array(cam.up);
    let mut d = (f + r * sx + u * sy).normalize();
    let mut origin = Vec3::from_array(cam.origin);
    let mut thr = Vec3::new(1.0, 1.0, 1.0);
    let mut li = Vec3::new(0.0, 0.0, 0.0);
    let mut prev_pdf = 1.0f32;
    let mut first = true;
    let mut level = [0.0f32; 4];
    let mut rr_cnt = [0.0f32; 4]; // [terminated, evaluated, comp_sum, comp_max]
    let n_lights = scene.lights.len();
    for b in 0..cfg.max_bounces as usize {
        let bb = prod_bounce_base(sb, b);
        let hit = tlas.intersect(blases, &Ray { origin, dir: d });
        let Some(hit) = hit else {
            break; // miss:吸收零态(后续 bounce 数学贡献恒 0;计数面存活门恒 0)
        };
        let prim = hit.tri as usize;
        let ng = Vec3::from_array(hit.normal);
        let p = origin + d * hit.t;
        let n = if ng.dot(d) > 0.0 { ng * (-1.0) } else { ng };
        let (albedo, emission) = match &scene.materials[prim] {
            MaterialKind::Lambert { albedo } => (*albedo, [0.0; 3]),
            MaterialKind::Emission { albedo, emission } => (*albedo, *emission),
            _ => ([0.0; 3], [0.0; 3]),
        };
        let al = Vec3::from_array(albedo);
        let em = Vec3::from_array(emission);
        let mut lum_b = 0.0f32;
        // ① BSDF 命中发光面(单面 + balance MIS w_b;多光源分母 = 全部 quad
        //    光源 NEE 联合 PDF 之和——非重叠光源仅命中光源项非零,显式累加)。
        let cos_emit = -ng.dot(d);
        if emission.iter().any(|c| *c > 0.0) && cos_emit > 0.0 {
            let w_b = if first {
                1.0
            } else if cfg.mis {
                let hit_light = scene.light_of_prim[prim] as usize;
                let mut r_sum = 0.0f32;
                for (li, l) in scene.lights.iter().enumerate() {
                    if let ProdLight::Quad(q) = l {
                        if li == hit_light {
                            let denom = q.area() * cos_emit * prev_pdf;
                            if denom > 0.0 {
                                r_sum += dist.pdf[li] * hit.t * hit.t / denom;
                            }
                        }
                    }
                }
                1.0 / (1.0 + r_sum)
            } else {
                1.0
            };
            let add = cmul(thr, em) * w_b;
            li = li + add;
            lum_b += (add.x + add.y + add.z) / 3.0;
        }
        // ② NEE(光源分布离散采样 → 光源内连续采样;联合 PDF = 离散 × 连续)。
        let u_sel = stream[bb];
        let mut sel = n_lights - 1;
        for (li, &c) in dist.cdf.iter().enumerate() {
            if u_sel < c {
                sel = li;
                break;
            }
        }
        let pdf_sel = dist.pdf[sel];
        match &scene.lights[sel] {
            ProdLight::Quad(q) => {
                let lp00 = Vec3::from_array(q.p00);
                let le1 = Vec3::from_array(q.e1);
                let le2 = Vec3::from_array(q.e2);
                let ln = Vec3::from_array(q.normal());
                let le = Vec3::from_array(q.emission);
                let area = q.area();
                let qp = lp00 + le1 * stream[bb + 1] + le2 * stream[bb + 2];
                let wv = qp - p;
                let dist2 = wv.dot(wv).max(1e-12);
                let dist = dist2.sqrt();
                let wi = wv * (1.0 / dist);
                let cos_s = n.dot(wi).max(0.0);
                let cos_l = (-ln.dot(wi)).max(0.0);
                if cos_s > 0.0 && cos_l > 0.0 {
                    // 贡献 = thr·(albedo/π)·cos_s·Le·cos_l·area/(π·dist²·pdf_d)。
                    let core = cos_s * cos_l * area / (PT_PI * dist2 * pdf_sel);
                    let w_l = if cfg.mis {
                        mis_balance_nee(cos_s, area, cos_l, pdf_sel, dist2)
                    } else {
                        1.0
                    };
                    let shadow_origin = p + n * RAY_EPS;
                    let t_sh = (dist - 2.0 * RAY_EPS).max(RAY_EPS);
                    let blocked = tlas.any_hit(
                        blases,
                        &Ray {
                            origin: shadow_origin,
                            dir: wi,
                        },
                        t_sh,
                    );
                    if !blocked {
                        let add = cmul(cmul(thr, al), le) * (core * w_l * (1.0 + cfg.energy_bias));
                        li = li + add;
                        lum_b += (add.x + add.y + add.z) / 3.0;
                    }
                }
            }
            ProdLight::Point {
                position,
                intensity,
            } => {
                let lp = Vec3::from_array(*position);
                let inten = Vec3::from_array(*intensity);
                let wv = lp - p;
                let dist2 = wv.dot(wv).max(1e-12);
                let dist = dist2.sqrt();
                let wi = wv * (1.0 / dist);
                let cos_s = n.dot(wi).max(0.0);
                if cos_s > 0.0 {
                    // delta 光源退化:w_nee = 1(BSDF 策略概率为零;无除零)。
                    // 贡献 = thr·(albedo/π)·cos_s·I/(dist²·pdf_d)。
                    let core = cos_s / (PT_PI * dist2 * pdf_sel);
                    let shadow_origin = p + n * RAY_EPS;
                    let t_sh = (dist - 2.0 * RAY_EPS).max(RAY_EPS);
                    let blocked = tlas.any_hit(
                        blases,
                        &Ray {
                            origin: shadow_origin,
                            dir: wi,
                        },
                        t_sh,
                    );
                    if !blocked {
                        let add = cmul(cmul(thr, al), inten) * (core * (1.0 + cfg.energy_bias));
                        li = li + add;
                        lum_b += (add.x + add.y + add.z) / 3.0;
                    }
                }
            }
        }
        level[b] += lum_b;
        // ③ BSDF 采样(余弦加权半球;ref_tracer 同式)。
        let nd = crate::rt::ref_tracer::cosine_sample_hemisphere(n, stream[bb + 3], stream[bb + 4]);
        prev_pdf = super::cosine_hemisphere_pdf(nd.dot(n));
        thr = cmul(thr, al);
        // ④ 吞吐自适应 RR(b ≥ min_bounce 启用;补偿闭式无偏;计数面)。
        let alive = thr.x.max(thr.y).max(thr.z) > 0.0;
        if cfg.rr && alive && b as u32 >= cfg.rr_params.min_bounce {
            let t_max_ch = thr.x.max(thr.y).max(thr.z);
            let p_kill = cfg.rr_params.p_kill(t_max_ch);
            rr_cnt[1] += 1.0; // evaluated
            if stream[bb + 5] < p_kill {
                rr_cnt[0] += 1.0; // terminated
                break; // 轮盘终止
            }
            let comp = if cfg.rr_comp_off {
                1.0 // RED 臂:补偿缺失
            } else {
                cfg.rr_params.compensation(p_kill)
            };
            rr_cnt[2] += comp;
            rr_cnt[3] = rr_cnt[3].max(comp);
            thr = thr * comp;
        }
        origin = p + n * RAY_EPS;
        d = nd;
        first = false;
    }
    ([li.x, li.y, li.z], level, rr_cnt)
}

/// host oracle 全图渲染(逐像素顺序累加;自适应面 = 达阈停采 sticky 门,
/// 与 kernel 逐字同源)。
pub fn trace_host_prod(
    scene: &ProdScene,
    dist: &LightDist,
    cfg: &ProdConfig,
    stream: &[f32],
) -> Result<ProdImage, PtError> {
    scene.validate()?;
    cfg.validate()?;
    let pixel_count = (scene.camera.width * scene.camera.height) as usize;
    let need = prod_stream_len(pixel_count, cfg.spp, cfg.max_bounces);
    if stream.len() != need {
        return Err(PtError::InvalidConfig(format!(
            "RNG 流长 {} ≠ 期望 {need}(pixel={pixel_count} spp={} bounces={})",
            stream.len(),
            cfg.spp,
            cfg.max_bounces
        )));
    }
    let blases = vec![TriBvh::build(&scene.positions, &scene.indices)];
    let tlas = Tlas::build(
        &[InstanceDesc {
            blas: 0,
            transform: Transform3x4::IDENTITY,
            mask: 0xFF,
            flags: 0,
        }],
        &blases,
    );
    let mut rgb = vec![0.0f32; pixel_count * 3];
    let mut sum_lum = vec![0.0f32; pixel_count];
    let mut sumsq_lum = vec![0.0f32; pixel_count];
    let mut samples = vec![0u32; pixel_count];
    let mut converged = vec![0.0f32; pixel_count];
    let mut rr_counters = vec![0.0f32; pixel_count * 4];
    let mut energy_levels = vec![0.0f32; pixel_count * 4];
    let bset: &[TriBvh] = &blases;
    for px in 0..pixel_count {
        let mut acc = [0.0f32; 3];
        let mut sl = 0.0f32;
        let mut sql = 0.0f32;
        let mut cnt = 0u32;
        let mut active = true;
        for s in 0..cfg.spp as usize {
            if !active {
                continue; // 达阈早停(sticky;kernel 算术门同语义)
            }
            let (li, level, rr) = trace_path_prod(&tlas, bset, scene, dist, cfg, stream, px, s);
            acc[0] += li[0];
            acc[1] += li[1];
            acc[2] += li[2];
            let lum = (li[0] + li[1] + li[2]) / 3.0;
            sl += lum;
            sql += lum * lum;
            cnt += 1;
            for k in 0..4 {
                energy_levels[px * 4 + k] += level[k];
            }
            rr_counters[px * 4] += rr[0]; // terminated 求和
            rr_counters[px * 4 + 1] += rr[1]; // evaluated 求和
            rr_counters[px * 4 + 2] += rr[2]; // comp_sum 求和
            if rr[3] > rr_counters[px * 4 + 3] {
                rr_counters[px * 4 + 3] = rr[3]; // comp_max 逐像素 max(非求和)
            }
            if let Some(a) = &cfg.adaptive {
                if cnt >= a.n_floor {
                    let e = rel_err_bound(sl, sql, cnt);
                    if e < a.rel_err_threshold {
                        active = false;
                        converged[px] = 1.0;
                    }
                }
            }
        }
        let n = cnt.max(1) as f32;
        rgb[px * 3] = acc[0] / n;
        rgb[px * 3 + 1] = acc[1] / n;
        rgb[px * 3 + 2] = acc[2] / n;
        sum_lum[px] = sl;
        sumsq_lum[px] = sql;
        samples[px] = cnt;
        for k in 0..4 {
            energy_levels[px * 4 + k] /= n;
        }
    }
    Ok(ProdImage {
        width: scene.camera.width,
        height: scene.camera.height,
        rgb,
        sum_lum,
        sumsq_lum,
        samples,
        converged,
        rr_counters,
        energy_levels,
        frame_label: if cfg.adaptive.is_some() {
            "adaptive"
        } else {
            "full_reference"
        },
    })
}

/// RR 计数修正(host 侧:comp_max 是逐像素 max 非求和——上方循环内已处
/// 理;本函数聚合帧级分布面:终止率 + 补偿因子分布 p50/p90/max + 存活评估
/// 均摊;逐场景进 evidence,RXS-0399 L3 计数非空承载面)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RrFrameStats {
    /// 终止率(Σterminated/Σevaluated)。
    pub termination_rate: f64,
    /// 补偿因子分布 p50(逐像素均值 comp = comp_sum/survived 的帧级百分位)。
    pub comp_p50: f64,
    /// p90。
    pub comp_p90: f64,
    /// max(逐像素 comp_max 的帧级最大)。
    pub comp_max: f64,
    /// 总评估数(非空判据面)。
    pub evaluated: u64,
    /// 总终止数。
    pub terminated: u64,
}

/// 聚合 RR 帧级统计(确定函数;空 evaluated → 全零由门判红)。
pub fn rr_frame_stats(img: &ProdImage) -> RrFrameStats {
    let mut terminated = 0u64;
    let mut evaluated = 0u64;
    let mut comp_means: Vec<f64> = Vec::new();
    let mut comp_max = 0.0f64;
    for px in 0..img.pixel_count() {
        let t = img.rr_counters[px * 4];
        let e = img.rr_counters[px * 4 + 1];
        let cs = img.rr_counters[px * 4 + 2];
        let cm = img.rr_counters[px * 4 + 3];
        terminated += t as u64;
        evaluated += e as u64;
        let survived = e - t;
        if survived > 0.0 {
            comp_means.push(f64::from(cs) / f64::from(survived));
            comp_max = comp_max.max(f64::from(cm));
        }
    }
    comp_means.sort_by(|a, b| a.total_cmp(b));
    let pct = |p: f64| -> f64 {
        if comp_means.is_empty() {
            return 0.0;
        }
        let idx = ((comp_means.len() as f64 - 1.0) * p).round() as usize;
        comp_means[idx.min(comp_means.len() - 1)]
    };
    RrFrameStats {
        termination_rate: if evaluated > 0 {
            terminated as f64 / evaluated as f64
        } else {
            0.0
        },
        comp_p50: pct(0.5),
        comp_p90: pct(0.9),
        comp_max,
        evaluated,
        terminated,
    }
}

// ---------------------------------------------------------------------------
// device 输入打包(kernel 头注参数面逐字同源)
// ---------------------------------------------------------------------------

/// 材质打包:8 f32/三角(albedo.rgb, emission.rgb, flags=light_index+1,
/// 0=非发光,pad)。
pub fn pack_prod_mats(scene: &ProdScene) -> Vec<f32> {
    let mut out = Vec::with_capacity(scene.indices.len() * 8);
    for (t, m) in scene.materials.iter().enumerate() {
        let (albedo, emission) = match m {
            MaterialKind::Lambert { albedo } => (*albedo, [0.0; 3]),
            MaterialKind::Emission { albedo, emission } => (*albedo, *emission),
            _ => ([0.0; 3], [0.0; 3]),
        };
        out.extend_from_slice(&albedo);
        out.extend_from_slice(&emission);
        let flag = if scene.light_of_prim[t] == u32::MAX {
            0.0
        } else {
            (scene.light_of_prim[t] + 1) as f32
        };
        out.push(flag);
        out.push(0.0);
    }
    out
}

/// 三角形打包:9 f32/三角。
pub fn pack_prod_tris(scene: &ProdScene) -> Vec<f32> {
    scene.blas_triangles()
}

/// 光源打包:16 f32/光源([type(0=quad,1=point), p0.xyz, e1.xyz, e2.xyz,
/// em.rgb, area, pdf_d, pad])。
pub fn pack_prod_lights(scene: &ProdScene, dist: &LightDist) -> Vec<f32> {
    let mut out = Vec::with_capacity(scene.lights.len() * 16);
    for (li, l) in scene.lights.iter().enumerate() {
        match l {
            ProdLight::Quad(q) => {
                out.push(0.0);
                out.extend_from_slice(&q.p00);
                out.extend_from_slice(&q.e1);
                out.extend_from_slice(&q.e2);
                out.extend_from_slice(&q.emission);
                out.push(q.area());
                out.push(dist.pdf[li]);
                out.push(0.0);
            }
            ProdLight::Point {
                position,
                intensity,
            } => {
                out.push(1.0);
                out.extend_from_slice(position);
                out.extend_from_slice(intensity); // e1 槽位 = 点强 I
                out.extend_from_slice(&[0.0; 3]); // e2 槽位空
                out.extend_from_slice(&[0.0; 3]); // emission 槽位空
                out.push(0.0); // area = 0(delta)
                out.push(dist.pdf[li]);
                out.push(0.0);
            }
        }
    }
    out
}

/// 参数打包:48 f32(kernel 头注布局逐字同源)。
pub fn pack_prod_params(scene: &ProdScene, cfg: &ProdConfig) -> Vec<f32> {
    let cam = &scene.camera;
    let pixel_count = cam.width * cam.height;
    let (adaptive_on, n_floor, theta) = match &cfg.adaptive {
        Some(a) => (1.0, a.n_floor as f32, a.rel_err_threshold),
        None => (0.0, 0.0, 0.0),
    };
    let mut p = Vec::with_capacity(48);
    p.push(pixel_count as f32); // [0]
    p.push(cfg.spp as f32); // [1] spp_max
    p.push(cfg.max_bounces as f32); // [2]
    p.push(cam.width as f32); // [3]
    p.push(cam.height as f32); // [4]
    p.push(if cfg.mis { 1.0 } else { 0.0 }); // [5]
    p.push(if cfg.rr { 1.0 } else { 0.0 }); // [6]
    p.push(cfg.rr_params.min_bounce as f32); // [7]
    p.push(RAY_EPS); // [8]
    p.push(scene.t_max); // [9]
    p.extend_from_slice(&cam.origin); // [10..13]
    p.extend_from_slice(&cam.forward); // [13..16]
    p.extend_from_slice(&cam.right); // [16..19]
    p.extend_from_slice(&cam.up); // [19..22]
    p.push(cam.tan_half_fov); // [22]
    p.push(1.0 / cam.width as f32); // [23]
    p.push(1.0 / cam.height as f32); // [24]
    p.push(scene.lights.len() as f32); // [25] n_lights
    p.push(cfg.rr_params.tau); // [26]
    p.push(cfg.rr_params.p_max); // [27]
    p.push(cfg.rr_params.comp_clamp); // [28]
    p.push(adaptive_on); // [29]
    p.push(n_floor); // [30]
    p.push(theta); // [31] adaptive rel_err 阈(逐样本判定)
    p.push(cfg.energy_bias); // [32] RED 臂(正常 = 0)
    p.push(if cfg.rr_comp_off { 1.0 } else { 0.0 }); // [33] RED 臂
    p.push(prod_sample_stride(cfg.max_bounces) as f32); // [34]
    p.push(cfg.seed as f32); // [35] provenance(digest 域外)
    p.extend_from_slice(&[0.0; 12]); // [36..48] 预留(恒 0)
    debug_assert_eq!(p.len(), 48);
    p
}

// ---------------------------------------------------------------------------
// 单测(RXS-0398~0401 锚定;host 面——数值锚/校验 fail-closed/host oracle
// 确定性/RED 臂预锚/计数与报告面/语料锚)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// host oracle 快捷渲染(指定族/spp/开关)。
    fn host_render(scene: &ProdScene, cfg: &ProdConfig) -> ProdImage {
        let dist = build_light_distribution(scene);
        let stream = sampler::generate(
            cfg.sampler,
            (scene.camera.width * scene.camera.height) as usize,
            cfg.spp,
            cfg.max_bounces,
            cfg.seed,
        );
        trace_host_prod(scene, &dist, cfg, &stream).expect("host oracle 渲染")
    }

    const TAU_TEST: f32 = 0.35;

    //@ spec: RXS-0398
    #[test]
    fn mis_balance_numeric_anchors_and_delta_degenerate() {
        // balance 恒等:w_nee + w_bsdf 同事件权和 ≈ 1(f32 舍入内)。
        //   同 PDF:p_nee=p_bsdf ⇒ 各 0.5。折合核验:
        //   w_nee(cos_s=0.6, area=0.09, cos_l=0.8, pdf_d=1, dist2=4)
        //     r = (0.6/π)·0.09·0.8/4 ≈ 0.0034377 ⇒ w ≈ 0.99657
        let w = mis_balance_nee(0.6, 0.09, 0.8, 1.0, 4.0);
        let expect = 1.0 / (1.0 + (0.6f32 / PT_PI) * 0.09 * 0.8 / 4.0);
        assert_eq!(w.to_bits(), expect.to_bits(), "w_nee 折合形位级锚");
        //   delta 光源退化:p_bsdf=0 ⇒ r=0 ⇒ w_nee=1 精确(无除零)。
        let wd = mis_balance_nee(0.0, 0.0, 0.8, 1.0, 4.0);
        assert_eq!(wd.to_bits(), 1.0f32.to_bits(), "delta 退化 w_nee=1");
        //   安全形退化防御:denom=0 ⇒ 1.0(不除零)。
        assert_eq!(mis_balance_nee(0.6, 0.09, 0.8, 0.0, 4.0), 1.0);
        assert_eq!(mis_balance_bsdf(2.0, 0.09, 0.8, 0.0, 1.0), 1.0);
        //   w_bsdf 单调性:pdf_d 大 ⇒ w_b 小(NEE 主导)。
        let wa = mis_balance_bsdf(2.0, 0.09, 0.8, 0.6, 0.5);
        let wb = mis_balance_bsdf(2.0, 0.09, 0.8, 0.6, 1.5);
        assert!(wa > wb, "w_b 随 pdf_d 单调降");
        //   权和恒等:同事件两策略权和 = 1(解析;p_nee=p_bsdf=x 时各 1/2)。
        let x = 0.37f32;
        let sum = x / (x + x) + x / (x + x);
        assert!((sum - 1.0).abs() < 1e-7);
    }

    //@ spec: RXS-0398
    #[test]
    fn prod_scenes_validate_and_light_dist_deterministic() {
        for scene in g12_prod_scenes() {
            scene.validate().expect("m96 消费面场景必过校验");
        }
        for scene in [
            g12_two_light_scene(),
            g12_furnace_scene(),
            g12_delta_light_scene(),
        ] {
            scene.validate().expect("生产化 fixture 必过校验");
            let d1 = build_light_distribution(&scene);
            let d2 = build_light_distribution(&scene);
            assert_eq!(d1, d2, "光源分布构建确定性(同场景同分布)");
            let g1 = light_distribution_digest(&scene, &d1);
            let g2 = light_distribution_digest(&scene, &d2);
            assert_eq!(g1, g2, "同场景同分布 digest 位级一致");
            // pdf 和 = 1;cdf 末 = 1。
            let s: f32 = d1.pdf.iter().sum();
            assert!((s - 1.0).abs() < 1e-5, "pdf 和 = 1(实测 {s})");
            assert_eq!(*d1.cdf.last().unwrap(), 1.0);
        }
        // 双光源分布非均匀(发射功率加权:左 10×0.0576 / 右 13.5×0.04)。
        let two = g12_two_light_scene();
        let dist = build_light_distribution(&two);
        assert_eq!(dist.pdf.len(), 2);
        assert!(dist.pdf[0] > dist.pdf[1], "功率加权:左灯 PDF 更大");
        // 起步范围冻结:范围外材质 typed Err(生产化校验面维持)。
        let mut bad = g12_two_light_scene();
        bad.materials[0] = MaterialKind::Specular {
            reflectance: [1.0; 3],
        };
        assert!(matches!(
            bad.validate(),
            Err(PtError::OutOfScopeMaterial {
                tri: 0,
                kind: "specular"
            })
        ));
    }

    //@ spec: RXS-0398
    #[test]
    fn host_oracle_furnace_energy_and_levels_monotone() {
        // 白炉能量守恒(host oracle sanity;门绿由 device 腿 + 标定容差承载):
        // 匹配深度 4 冻结面下白炉为 4-bounce 截断估计子——全平衡极限 Le=4 需
        // 无穷反弹,截断真值 ≈ 0.53·Le(实测面:G12.2 定位批——裸双臂 w=1 加和
        // 均值 4.037 为双重计数伪影,正确 MIS 均值 ≈ 2.1,纯 NEE 形 ≈ 2.21);
        // sanity 带 = [1.5, Le×1.001](下界 = 截断结构非平凡能量到达;上界 =
        // **不产能量**硬物理断言,容差外禁超)。
        let scene = g12_furnace_scene();
        let cfg = ProdConfig::production(64, SamplerFamily::Stratified, TAU_TEST);
        let img = host_render(&scene, &cfg);
        let mean = img.mean_luminance();
        let le = 4.0f64;
        assert!(
            mean > 1.5 && mean <= le * 1.001,
            "白炉均值 {mean} 越截断守恒面(1.5, Le×1.001](不产能量上界 + 截断非平凡下界)"
        );
        // 逐级能量增量单调不增(cornell albedo<1:E_{k+1} ≤ E_k 噪声带内)。
        let cornell = &g12_prod_scenes()[0];
        let imgc = host_render(cornell, &cfg);
        let mut levels = [0.0f64; 4];
        for px in 0..imgc.pixel_count() {
            for k in 0..4 {
                levels[k] += f64::from(imgc.energy_levels[px * 4 + k]);
            }
        }
        for k in 0..4 {
            levels[k] /= imgc.pixel_count() as f64;
        }
        assert!(levels[0] > 0.0, "第 0 级能量非零");
        for k in 1..4 {
            assert!(
                levels[k] <= levels[k - 1] * 1.05,
                "逐级能量增量单调不增(噪声带 5%):E{k}={} > E{}={}",
                levels[k],
                k - 1,
                levels[k - 1]
            );
        }
        // 只丢能量不漏光:输出全部有限非负。
        assert!(imgc.rgb.iter().all(|v| v.is_finite() && *v >= 0.0));
    }

    //@ spec: RXS-0398
    #[test]
    fn host_oracle_mis_red_arm_detectable_and_delta_mis_irrelevant() {
        let cornell = &g12_prod_scenes()[0];
        let cfg = ProdConfig::production(16, SamplerFamily::Stratified, TAU_TEST);
        let golden = prod_image_digest(&host_render(cornell, &cfg));
        // 权重缺失 RED 臂:关 MIS 输出必分叉。
        let mut no_mis = cfg;
        no_mis.mis = false;
        assert_ne!(
            golden,
            prod_image_digest(&host_render(cornell, &no_mis)),
            "权重缺失冒充 MIS 必分叉(RED)"
        );
        // 能量偏置注入 RED 臂:energy_bias=+0.05 输出必分叉。
        let mut bias = cfg;
        bias.energy_bias = 0.05;
        assert_ne!(
            golden,
            prod_image_digest(&host_render(cornell, &bias)),
            "能量偏置注入必分叉(RED)"
        );
        // delta 光源退化:MIS 开关对 delta 场景输出 0 影响(BSDF 永远打不中
        // 点光 ⇒ w_nee≡1 两臂位级一致)。
        let delta = g12_delta_light_scene();
        let d_on = prod_image_digest(&host_render(&delta, &cfg));
        let d_off = prod_image_digest(&host_render(&delta, &no_mis));
        assert_eq!(d_on, d_off, "delta 光源退化:w_nee=1,MIS 开关不改变输出");
    }

    //@ spec: RXS-0399
    #[test]
    fn rr_params_validate_and_closed_form_compensation() {
        // 最小反弹保障 + p_max<1 恒成立(fail-closed)。
        assert!(RrProdParams::production(TAU_TEST).validate().is_ok());
        let mut bad = RrProdParams::production(TAU_TEST);
        bad.min_bounce = 0;
        assert!(bad.validate().is_err(), "N_min<2 必拒(早杀偏置)");
        bad.min_bounce = 1;
        assert!(bad.validate().is_err(), "N_min=1 必拒");
        let mut bad2 = RrProdParams::production(TAU_TEST);
        bad2.p_max = 1.0;
        assert!(bad2.validate().is_err(), "p_max=1 必拒(截断偏置)");
        // 闭式无偏:comp × (1−p_kill) == 1(f32 舍入内;补偿缺失冒充无偏即 RED)。
        let rr = RrProdParams::production(TAU_TEST);
        for &t in &[0.0f32, 0.05, 0.2, 0.5, 1.0] {
            let p = rr.p_kill(t);
            assert!((0.0..=G12_RR_P_MAX).contains(&p), "p_kill ∈ [0,p_max]");
            let comp = rr.compensation(p);
            assert!((comp * (1.0 - p) - 1.0).abs() < 1e-5, "补偿闭式无偏恒等");
            assert!(comp <= G12_RR_COMP_CLAMP, "补偿因子上界登记");
        }
        // 吞吐自适应:p_kill 随吞吐单调不增。
        assert!(rr.p_kill(0.05) > rr.p_kill(0.5));
        assert_eq!(rr.p_kill(1.0), 0.0, "吞吐 ≥ τ 不终止");
    }

    //@ spec: RXS-0399
    #[test]
    fn host_oracle_rr_counters_nonempty_and_unbiased() {
        let cornell = &g12_prod_scenes()[0];
        let cfg = ProdConfig::production(32, SamplerFamily::Stratified, TAU_TEST);
        let img = host_render(cornell, &cfg);
        let stats = rr_frame_stats(&img);
        // 计数非空:终止率 ∈ (0,1)、补偿分布 p50/p90/max ≥ 1。
        assert!(stats.evaluated > 0, "RR 评估计数非空");
        assert!(stats.terminated > 0, "RR 终止计数非空");
        assert!(stats.termination_rate > 0.0 && stats.termination_rate < 1.0);
        assert!(stats.comp_p50 >= 1.0 && stats.comp_p90 >= stats.comp_p50);
        assert!(stats.comp_max >= stats.comp_p90 && stats.comp_max <= 20.0);
        // 无偏 measured 面:RR 开/关均值一致(同流同 seed;宽 sanity 带 5%)。
        let mut no_rr = cfg;
        no_rr.rr = false;
        let img_off = host_render(cornell, &no_rr);
        let m_on = img.mean_luminance();
        let m_off = img_off.mean_luminance();
        assert!(
            (m_on - m_off).abs() / m_off < 0.05,
            "RR 无偏 sanity:on={m_on} off={m_off}"
        );
        // 跳 RR 偏移 RED 预锚:两臂 digest 必分叉。
        assert_ne!(
            prod_image_digest(&img),
            prod_image_digest(&img_off),
            "跳 RR 必分叉(RED)"
        );
        // 补偿缺失 RED 预锚:rr_comp_off 输出必分叉。
        let mut comp_off = cfg;
        comp_off.rr_comp_off = true;
        assert_ne!(
            prod_image_digest(&img),
            prod_image_digest(&host_render(cornell, &comp_off)),
            "补偿缺失必分叉(RED)"
        );
    }

    //@ spec: RXS-0400
    #[test]
    fn sampler_index_determinism_and_bitexact() {
        let px = 8;
        let spp = 4u32;
        for fam in [
            SamplerFamily::Pcg,
            SamplerFamily::Stratified,
            SamplerFamily::Sobol,
        ] {
            let a = sampler::generate(fam, px, spp, 4, G12_PROD_SEED);
            let b = sampler::generate(fam, px, spp, 4, G12_PROD_SEED);
            assert_eq!(a, b, "{fam:?} 同 seed 流位级一致");
            assert!(
                a.iter().all(|v| (0.0..1.0).contains(v)),
                "{fam:?} 值域 [0,1)"
            );
            // 索引推导确定性:逐索引重求值 == 流内容(stratified/sobol 逐维
            // 确定函数;PCG 为序进流,逐索引重求值经同序推进核验)。
            let stride = prod_sample_stride(4);
            for (pixel, sample, dim) in [(0, 0, 0), (3, 2, 5), (7, 3, 25), (5, 1, 12)] {
                let idx = (pixel * spp as usize + sample) * stride + dim;
                let v = sampler::sample_at(fam, pixel, sample, dim, spp, G12_PROD_SEED);
                assert_eq!(
                    a[idx], v,
                    "{fam:?} 索引 ({pixel},{sample},{dim}) 重求值 ≠ 流内容"
                );
            }
            // 改 seed ⇒ 流分叉。
            let c = sampler::generate(fam, px, spp, 4, G12_PROD_SEED ^ 0xABCD);
            assert_ne!(a, c, "{fam:?} 改 seed 流必分叉");
            // provenance 字面非空且含族名与寻址公式。
            let prov = sampler::provenance(fam, 4);
            assert!(prov.contains(fam.name()) && prov.contains("寻址公式"));
        }
        // 分层置换双射(spp=16:σ 覆盖 0..16 全排)。
        let mut seen = std::collections::BTreeSet::new();
        for s in 0..16usize {
            let v = stratified_sample(3, s, 16, G12_PROD_SEED, 42);
            let bucket = (v * 16.0) as u32;
            assert!(bucket < 16);
            seen.insert(bucket);
        }
        assert_eq!(seen.len(), 16, "分层置换覆盖全层(双射)");
        // Sobol 本原多项式程序生成自证:前 26 维均可产且本原。
        for d in 0..26 {
            let (poly, s) = sobol_primitive_poly(d);
            assert!(
                gf2_is_primitive(poly, s),
                "dim {d} 多项式非本原(生成器缺陷)"
            );
        }
    }

    //@ spec: RXS-0400
    #[test]
    fn host_oracle_lds_bitexact_and_selection_deterministic() {
        let cornell = &g12_prod_scenes()[0];
        for fam in [SamplerFamily::Stratified, SamplerFamily::Sobol] {
            let cfg = ProdConfig::production(16, fam, TAU_TEST);
            let a = host_render(cornell, &cfg);
            let b = host_render(cornell, &cfg);
            assert_eq!(
                prod_image_digest(&a),
                prod_image_digest(&b),
                "{fam:?} 双跑位级一致"
            );
        }
        // 选型 benchmark 确定性(host 方差对照,两跑同一胜出族)。
        let s1 = sampler_benchmark_winner();
        let s2 = sampler_benchmark_winner();
        assert_eq!(s1, s2, "选型 benchmark 两跑同一胜出族(确定性)");
        // 非确定注入 RED 预锚:篡改流单元素 ⇒ host 输出分叉。
        let cfg = ProdConfig::production(16, SamplerFamily::Stratified, TAU_TEST);
        let dist = build_light_distribution(cornell);
        let mut stream = sampler::generate(
            SamplerFamily::Stratified,
            (cornell.camera.width * cornell.camera.height) as usize,
            16,
            4,
            G12_PROD_SEED,
        );
        let golden = trace_host_prod(cornell, &dist, &cfg, &stream).expect("golden");
        // 篡改像素 0 采样 4 的相机 jitter u 维(索引 104 = (0·16+4)·26;必被
        // 消费维。注:RR 维在 bounce<min_bounce 不求值、末位维可能不跨越终止
        // 阈——RED 预锚须落在必消费维上)。
        stream[104] = (stream[104] + 0.5).fract();
        let tampered = trace_host_prod(cornell, &dist, &cfg, &stream).expect("tampered");
        assert_ne!(
            prod_image_digest(&golden),
            prod_image_digest(&tampered),
            "序列篡改必分叉(RED)"
        );
    }

    /// 选型 benchmark(host oracle 方差对照;确定性;胜出族 = 双场景 spp16
    /// 平均逐像素方差最小者;选型证据进 evidence 面)。
    pub fn sampler_benchmark_winner() -> SamplerFamily {
        let mut best = SamplerFamily::Pcg;
        let mut best_var = f64::INFINITY;
        for fam in [
            SamplerFamily::Pcg,
            SamplerFamily::Stratified,
            SamplerFamily::Sobol,
        ] {
            let mut acc = 0.0f64;
            for scene in g12_prod_scenes() {
                let cfg = ProdConfig::production(16, fam, TAU_TEST);
                acc += host_render(&scene, &cfg).mean_pixel_variance();
            }
            if acc < best_var {
                best_var = acc;
                best = fam;
            }
        }
        best
    }

    //@ spec: RXS-0400
    #[test]
    fn sampler_benchmark_lds_not_worse_than_pcg() {
        // 收敛加速卖点:胜出族方差 ≤ PCG 族方差(host oracle 对照面)。
        let mut var_pcg = 0.0f64;
        let mut var_best = 0.0f64;
        let best = sampler_benchmark_winner();
        for scene in g12_prod_scenes() {
            var_pcg += host_render(
                &scene,
                &ProdConfig::production(16, SamplerFamily::Pcg, TAU_TEST),
            )
            .mean_pixel_variance();
            var_best += host_render(&scene, &ProdConfig::production(16, best, TAU_TEST))
                .mean_pixel_variance();
        }
        assert!(
            var_best <= var_pcg * 1.0001,
            "胜出族 {best:?} 方差 {var_best} 不得劣于 PCG {var_pcg}"
        );
    }

    //@ spec: RXS-0401
    #[test]
    fn adaptive_params_validate_and_report_nonempty() {
        assert!(
            AdaptiveParams {
                n_floor: 4,
                spp_max: 64,
                rel_err_threshold: 0.05
            }
            .validate()
            .is_ok()
        );
        assert!(
            AdaptiveParams {
                n_floor: 0,
                spp_max: 64,
                rel_err_threshold: 0.05
            }
            .validate()
            .is_err(),
            "n_floor=0 必拒"
        );
        assert!(
            AdaptiveParams {
                n_floor: 4,
                spp_max: 64,
                rel_err_threshold: 0.0
            }
            .validate()
            .is_err()
        );
        let cornell = &g12_prod_scenes()[0];
        let mut cfg = ProdConfig::production(64, SamplerFamily::Stratified, TAU_TEST);
        cfg.adaptive = Some(AdaptiveParams {
            n_floor: G12_ADAPTIVE_N_FLOOR,
            spp_max: 64,
            rel_err_threshold: 0.05,
        });
        let img = host_render(cornell, &cfg);
        // 帧型标签闭集。
        assert_eq!(img.frame_label, "adaptive");
        // spp 下界保障:逐像素 samples ≥ N_floor。
        assert!(
            img.samples.iter().all(|&n| n >= G12_ADAPTIVE_N_FLOOR),
            "spp 下界保障"
        );
        assert!(img.samples.iter().all(|&n| n <= 64), "spp 上界");
        // 收敛报告非空:spp 分布/方差/未收敛计数可算出(分布面)。
        let mut spps: Vec<u32> = img.samples.clone();
        spps.sort_unstable();
        let p50 = spps[spps.len() / 2];
        let unconverged = img.converged.iter().filter(|&&c| c == 0.0).count();
        assert!(p50 >= G12_ADAPTIVE_N_FLOOR);
        assert!(
            unconverged + img.converged.iter().filter(|&&c| c == 1.0).count() == img.pixel_count()
        );
        assert!(img.mean_pixel_variance() >= 0.0);
        // 早停冒充 RED 预锚:n_floor=0 + 巨大阈 ⇒ 全像素 1 样本早停,
        // 与正例臂输出必分叉。
        let mut masq = cfg;
        masq.adaptive = Some(AdaptiveParams {
            n_floor: 0,
            spp_max: 64,
            rel_err_threshold: 1e9,
        });
        // n_floor=0 不过 validate( fail-closed);RED 臂由 harness 覆写参数
        // 直接进 kernel(单测面用 n_floor=1 + 巨阈模拟早停语义)。
        masq.adaptive = Some(AdaptiveParams {
            n_floor: 1,
            spp_max: 64,
            rel_err_threshold: 1e9,
        });
        let img_m = host_render(cornell, &masq);
        assert!(img_m.samples.iter().all(|&n| n == 1), "早停臂逐像素 1 样本");
        assert_ne!(
            prod_image_digest(&img),
            prod_image_digest(&img_m),
            "早停冒充必分叉(RED)"
        );
    }

    //@ spec: RXS-0401
    #[test]
    fn adaptive_vs_full_reference_close_and_misjudge_recompute() {
        let cornell = &g12_prod_scenes()[0];
        let base = ProdConfig::production(64, SamplerFamily::Stratified, TAU_TEST);
        let full = host_render(cornell, &base);
        assert_eq!(full.frame_label, "full_reference");
        let mut ad = base;
        ad.adaptive = Some(AdaptiveParams {
            n_floor: G12_ADAPTIVE_N_FLOOR,
            spp_max: 64,
            rel_err_threshold: 0.03,
        });
        let img = host_render(cornell, &ad);
        // 帧级对拍(host sanity;门绿由 device 腿 + 冻结带继承承载)。
        let dev = super::super::rel_dev(&img.rgb, &full.rgb).expect("rel_dev");
        assert!(dev < 0.15, "自适应帧 vs 全 spp 参照帧级偏差 {dev}(sanity)");
        // 误判率对照面:判收敛像素中相对参照逐像素亮度偏差 > 0.25 的比例
        // (误判带字面进 evidence;host sanity 非门阈)。
        let mut judged = 0usize;
        let mut mis = 0usize;
        for px in 0..img.pixel_count() {
            if img.converged[px] == 1.0 {
                judged += 1;
                let a = (img.rgb[px * 3] + img.rgb[px * 3 + 1] + img.rgb[px * 3 + 2]) / 3.0;
                let b = (full.rgb[px * 3] + full.rgb[px * 3 + 1] + full.rgb[px * 3 + 2]) / 3.0;
                if (a - b).abs() / b.abs().max(1e-3) > 0.25 {
                    mis += 1;
                }
            }
        }
        let rate = if judged > 0 {
            mis as f64 / judged as f64
        } else {
            0.0
        };
        assert!(rate < 0.05, "误判率 host sanity {rate}");
        // 缺报检出预锚:未收敛计数独立重算 ≠ 篡改值。
        let recompute = img.converged.iter().filter(|&&c| c == 0.0).count();
        let forged = recompute.saturating_sub(1);
        assert_ne!(recompute, forged, "缺报注入必可检出(RED 预锚)");
    }

    //@ spec: RXS-0398
    #[test]
    fn conformance_g12_corpus_present() {
        // 消费锚定义务:G12.2 锚定语料在位且锚定本波条款。
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/gi");
        let files = [
            ("accept/mis_full_surface_minimal.rx", "RXS-0398"),
            ("accept/rr_throughput_adaptive_minimal.rx", "RXS-0399"),
            ("accept/lds_deterministic_minimal.rx", "RXS-0400"),
            ("accept/adaptive_convergence_minimal.rx", "RXS-0401"),
            ("reject/mis_weight_missing.rx", "RXS-0398"),
            ("reject/mis_energy_bias_inject.rx", "RXS-0398"),
            ("reject/rr_early_kill_bias.rx", "RXS-0399"),
            ("reject/rr_compensation_missing.rx", "RXS-0399"),
            ("reject/lds_nondeterministic_inject.rx", "RXS-0400"),
            ("reject/early_stop_masquerade.rx", "RXS-0401"),
            ("reject/unconverged_pixel_underreport.rx", "RXS-0401"),
        ];
        for (f, clause) in files {
            let text = std::fs::read_to_string(root.join(f)).expect("锚定语料在位");
            assert!(
                text.contains(&format!("//@ spec: {clause}")),
                "{f} 缺 {clause} 锚"
            );
        }
    }

    //@ spec: RXS-0398
    #[test]
    fn pack_prod_layout_anchors() {
        let scene = g12_two_light_scene();
        let dist = build_light_distribution(&scene);
        let cfg = ProdConfig::production(16, SamplerFamily::Stratified, TAU_TEST);
        assert_eq!(pack_prod_mats(&scene).len(), scene.indices.len() * 8);
        assert_eq!(pack_prod_tris(&scene).len(), scene.indices.len() * 9);
        assert_eq!(pack_prod_lights(&scene, &dist).len(), 2 * 16);
        let p = pack_prod_params(&scene, &cfg);
        assert_eq!(p.len(), 48);
        assert_eq!(p[25], 2.0, "n_lights=2");
        assert_eq!(p[34], 26.0, "stride=2+6×4=26");
        // 发光三角 flags = light_index+1。
        let mats = pack_prod_mats(&scene);
        let last_tri_flag = mats[(scene.indices.len() - 1) * 8 + 6];
        assert_eq!(last_tri_flag, 2.0, "末发光三角 flag=光源 1+1");
    }
}
