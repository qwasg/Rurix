//! G42 湿街地面着色前端(门 `g42.wet.ground`)——host 金标准面。
//!
//! 本模块与 `kernels/g42_direct_gi_wet.rx` **公式面逐字同源**:下列每个函数都是
//! 对应 kernel 内联段的唯一事实源,同一运算序、同一左结合、同一夹紧形。host 纯
//! safe 确定性(全库 `forbid(unsafe_code)`)、全 f32、零外部 crate、零资产。
//!
//! ## fork 纪律(母版 0-byte)
//!
//! `g42_direct_gi_wet.rx` 是冻结母版 `kernels/g14_3_direct_gi.rx` 的**加性
//! fork**:母版 `.rx` 保持 0-byte 不动,湿街块以「乘性中性可退化」的形态叠在
//! 母版的 albedo / roughness / 高光三处。本模块同理 —— 不触
//! `src/rurix-render/src/world/mod.rs` 之外的任何文件。
//!
//! ## 参数槽位(`params[49..56)`,7 字)
//!
//! 母版 `.rx` 头注虽登记 `[42..48)=reserved(恒 0)`,但**该段不可用**:同文件的
//! 扩面链(`pack_frame_params_nrm` / `_ggx` / `_lamp` / `_tex` / `_gi2` /
//! `pack_frame_params_dyn`)已把 `[42]`(sky_intensity / dyn_tri_base)、
//! `[43]`(smooth_nrm)、`[44..48)`(半球环境光)、`[48]`(GGX)分给
//! `g18_smooth_nrm.rx` 等**其它** kernel 的臂;其中 `[42]` 由 env
//! `RURIX_G18_SKY_INTENSITY` 驱动,不受 CLI 闭集管辖 —— 撞位风险真实。
//!
//! 故湿街段改取 `[49..56)`。该段在 g35 车道基线路(`pack_frame_params` ⇒
//! smooth_nrm=false / ggx=false / gi2=false)恒 0:`[49]`(lamp_contrib)与
//! `[51..55)`(GI2)仅质量车道写、`[50]`(tex_kpix)基线路显式写 `0.0`、`[55]`
//! 无人写,**且全段无 env 驱动写入**。`PARAMS_LEN = 56` 已覆盖本段,故
//! `g14_3_lane/g14_3_lane_body.rs` 的 `pack_frame_params` **0-byte 不触**:
//! 湿街 7 字由 [`WetParams::pack`] 产出,bin 侧在 pack 之后逐槽覆写,且覆写前
//! 断言原值恰为 `0.0`(撞位 fail-closed 自证)。
//!
//! ## 中性性质(0-语义漂移的最强证据)
//!
//! 取 `dark = 1.0, puddle_amount = 0.0, spec = 0.0`([`WetParams::is_neutral`])
//! 时,fork 逐位复现母版:
//!
//! ```text
//! dark_f   = 1.0 + (1.0 − 1.0)·(1 − wet) = 1.0   (精确;0.0·有限 = 0.0)
//! pud      = sstep((nv − 1.0)·PUD_SOFT)·wet = 0.0(精确;nv ≤ 1.0 恒成立)
//! pud_dark = 1.0 − PUD_DARK_K·0.0 = 1.0          (精确)
//! refl_w   = fresnel·wet·0.0·(…) = 0.0           (精确)
//! ```
//!
//! ⇒ `albedo·1.0·1.0` 与母版位级恒等,高光/反射加性项恒 0。此性质由
//! `wet_albedo_neutral_is_bit_identical` / `puddle_mask_zero_when_amount_zero` /
//! `is_neutral_exact_predicate` 三条单测以 `to_bits()` 锚定,是门的 0-语义漂移
//! 主证据。
//!
//! ## 如实登记:host↔device 位级面的边界
//!
//! [`hash2`] 内含 `sin`。超越函数在 host(libm)与 device(SPIR-V
//! `GLSL.std.450 Sin`)上**不保证同一舍入**,故本模块**不主张**积水掩码的
//! host↔device 位级相等 —— 与 G41 [`super::water_surface`] 对 `exp` 的登记同律。
//! 主张的是:① 公式面逐字同源(运算序/夹紧形可逐行核对);② device 端**同参数
//! 双跑位级一致**(per-run 确定性,由双跑 bit-equal 门事实覆盖);③ 中性臂
//! 位级恒等 —— 该臂 `nv` 只经比较后被 sstep 夹到 0,`sin` 的舍入差被夹紧吃掉,
//! 不进入输出。
//!
//! ## 坐标约定
//!
//! 与 [`super::sky`] 同:Y-up 右手系。`hx/hy/hz` = 命中点世界坐标,`ny` = 命中点
//! 世界法线 Y 分量。

// ---------------------------------------------------------------------------
// 冻结常量面(host / device 逐字同值)
// ---------------------------------------------------------------------------

/// 法线朝上门下限(`ny`;低于此值一律判为立面/朝下面,不参与湿街)。
pub const UP_LO: f32 = 0.55;
/// 朝上门软区倒数(软区宽 0.35 ⇒ `ny ∈ [0.55, 0.90]` 平滑起门)。
pub const UP_INV: f32 = 1.0 / 0.35;
/// 积水阈软区倒数(软区宽 0.18 ⇒ 积水边缘不出硬边)。
pub const PUD_SOFT: f32 = 1.0 / 0.18;
/// 积水降糙系数(积水处 roughness 再乘 `1 − 0.6·pud`)。
pub const PUD_ROUGH_K: f32 = 0.6;
/// 积水额外压暗系数(积水处 albedo 再乘 `1 − 0.35·pud`)。
pub const PUD_DARK_K: f32 = 0.35;
/// 湿面镜面基准权(无积水的湿沥青仍有可观镜面份额)。
pub const PUD_REFL_LO: f32 = 0.35;
/// 积水镜面增益(满积水时反射权 = `LO + HI` = 1.0)。
pub const PUD_REFL_HI: f32 = 0.65;
/// 积水噪声格距倒数(= 1 / 0.8 m ⇒ 积水斑块特征尺度约 0.8 米)。
pub const LATTICE: f32 = 1.25;
/// [`hash2`] 相位系数 A(与 B 构成无公度相位对,消除格点共振)。
///
/// 字面量恰好是 `FRAC_1_PI` 的四位前缀,故 `clippy::approx_constant`(correctness
/// 组,deny-by-default)会误判。此处**不可**换成 `std::f32::consts::FRAC_1_PI`:
/// 该常量与 `.rx` 侧写死的 `0.3183` 不同值,一换就破坏公式面逐字同源。
#[allow(clippy::approx_constant)]
pub const HASH_A: f32 = 0.3183;
/// [`hash2`] 相位系数 B。
pub const HASH_B: f32 = 0.7213;
/// [`hash2`] 幅值系数(取 2 的幂,`fract` 前的整数位丢弃可控)。
pub const HASH_M: f32 = 4096.0;
/// 湿介质 Fresnel F0(水 / 湿沥青;n ≈ 1.33 的垂直入射反射率量级)。
pub const F0: f32 = 0.03;
/// 湿街参数段在帧参数面中的起始下标(`params[49..56)`;母版 `[42..48)` 预留段
/// 已被扩面链与 env 占用,见模块头注「参数槽位」)。
pub const WET_PARAM_BASE: usize = 49;
/// 湿街参数段字数([`WetParams::pack`] 产出长度)。
pub const WET_PARAM_COUNT: usize = 7;

/// 地面世界 Y 中位缺省值(BistroExterior `exterior_scene_facts.json`
/// `ground.y_median`)。
pub const GROUND_Y_DEFAULT: f32 = 0.371;
/// 地面带半宽缺省值(由 `ground.y_p10` = 0.228 / `ground.y_p90` = 0.54 推)。
pub const GROUND_HALF_DEFAULT: f32 = 0.156;
/// 反照率乘子缺省值。
pub const DARK_DEFAULT: f32 = 0.55;
/// 湿面 roughness 缺省值。
pub const ROUGH_DEFAULT: f32 = 0.12;
/// 高光/镜面总强度缺省值。
pub const SPEC_DEFAULT: f32 = 1.0;
/// 积水覆盖比缺省值。
pub const PUDDLE_AMOUNT_DEFAULT: f32 = 0.45;

// ---------------------------------------------------------------------------
// 自由函数面(每个 = kernel 内联段的逐字同源事实源)
// ---------------------------------------------------------------------------

/// 小数部分 `x − floor(x)`(负数侧取正周期:`fract(−2.5) = 0.5`)。
pub fn fract(x: f32) -> f32 {
    x - x.floor()
}

/// smoothstep 的单参数形 `3t² − 2t³`,入参先夹到 `[0, 1]`。
///
/// 写成 `min`/`max` 而非 `clamp`:`.rx` 执行面无 `clamp` 内建,分支判定一律走
/// min/max 算术门(母版确定性协议同律),此处逐字同形。
pub fn sstep(t: f32) -> f32 {
    let c = t.min(1.0).max(0.0);
    c * c * (3.0 - 2.0 * c)
}

/// 二维格点哈希 ∈ `[0, 1)`(积水噪声的值源)。
///
/// **登记**:含 `sin`,host↔device 位级相等**不主张**(见模块头注)。
pub fn hash2(a: f32, b: f32) -> f32 {
    fract((a * HASH_A + b * HASH_B).sin() * HASH_M)
}

// ---------------------------------------------------------------------------
// 参数面
// ---------------------------------------------------------------------------

/// 湿街着色参数(CLI `--wet-*` 闭集面;fail-closed 校验见
/// [`WetParams::validate`])。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WetParams {
    /// 湿街总门(false ⇒ [`wet_gate`] 恒 0 ⇒ fork 与母版位级恒等)。
    pub on: bool,
    /// 反照率乘子,闭集 `(0.2, 1.0]`,缺省 [`DARK_DEFAULT`] = 0.55。
    ///
    /// 1.0 = 不压暗(中性臂之一)。下界开区间:0.2 以下的湿面在夜景里塌成死黑,
    /// 无可用画面,故 fail-closed 拒录而非静默夹紧。
    pub dark: f32,
    /// 湿面 roughness,闭集 `(0.02, 0.6]`,缺省 [`ROUGH_DEFAULT`] = 0.12。
    ///
    /// 下界开区间:< 0.02 时 GGX 的 `alpha²` 落到 `tiny` 夹紧域,峰值被截平,
    /// 参数失去单调意义。
    pub rough: f32,
    /// 高光/镜面总强度,闭集 `[0.0, 4.0]`,缺省 [`SPEC_DEFAULT`] = 1.0。
    ///
    /// 0.0 = 关高光与反射(中性臂之一)。
    pub spec: f32,
    /// 积水覆盖比,闭集 `[0.0, 1.0]`,缺省 [`PUDDLE_AMOUNT_DEFAULT`] = 0.45。
    ///
    /// 0.0 = 无积水([`puddle_mask`] 位级恒 0;中性臂之一)。
    pub puddle_amount: f32,
    /// 地面世界 Y 中位,缺省 [`GROUND_Y_DEFAULT`] = 0.371。
    pub ground_y: f32,
    /// 地面带半宽,缺省 [`GROUND_HALF_DEFAULT`] = 0.156;须 `> 0`。
    pub ground_half: f32,
}

impl Default for WetParams {
    fn default() -> Self {
        Self {
            on: false,
            dark: DARK_DEFAULT,
            rough: ROUGH_DEFAULT,
            spec: SPEC_DEFAULT,
            puddle_amount: PUDDLE_AMOUNT_DEFAULT,
            ground_y: GROUND_Y_DEFAULT,
            ground_half: GROUND_HALF_DEFAULT,
        }
    }
}

impl WetParams {
    /// 闭集域校验(fail-closed:越界一律拒录,不静默夹紧)。
    ///
    /// 先断言全字段有限,再做偏序比较 —— 否则 NaN 会从 `!(lo < x && x <= hi)`
    /// 的偏序陷阱里漏过去。
    pub fn validate(&self) -> Result<(), String> {
        if !self.dark.is_finite()
            || !self.rough.is_finite()
            || !self.spec.is_finite()
            || !self.puddle_amount.is_finite()
            || !self.ground_y.is_finite()
            || !self.ground_half.is_finite()
        {
            return Err("湿街参数须全部有限(非 NaN/Inf)".to_string());
        }
        if self.dark <= 0.2 || self.dark > 1.0 {
            return Err("--wet-dark 须 ∈ (0.2, 1.0](缺省 0.55)".to_string());
        }
        if self.rough <= 0.02 || self.rough > 0.6 {
            return Err("--wet-rough 须 ∈ (0.02, 0.6](缺省 0.12)".to_string());
        }
        if !(0.0..=4.0).contains(&self.spec) {
            return Err("--wet-spec 须 ∈ [0.0, 4.0](缺省 1.0)".to_string());
        }
        if !(0.0..=1.0).contains(&self.puddle_amount) {
            return Err("--wet-puddle 须 ∈ [0.0, 1.0](缺省 0.45;0.0 = 无积水)".to_string());
        }
        if self.ground_half <= 0.0 {
            return Err("--wet-ground-half 须 > 0(缺省 0.156)".to_string());
        }
        Ok(())
    }

    /// 打包湿街参数段(长度恒 [`WET_PARAM_COUNT`];落位
    /// `params[WET_PARAM_BASE + i]`)。
    ///
    /// 槽位序与 `kernels/g42_direct_gi_wet.rx` 头注逐字同源:
    ///
    /// ```text
    /// [49]=on(0/1) [50]=dark [51]=rough [52]=spec
    /// [53]=puddle_amount [54]=ground_y [55]=ground_half
    /// ```
    pub fn pack(&self) -> [f32; WET_PARAM_COUNT] {
        let on = if self.on { 1.0 } else { 0.0 };
        [
            on,
            self.dark,
            self.rough,
            self.spec,
            self.puddle_amount,
            self.ground_y,
            self.ground_half,
        ]
    }

    /// 是否为**中性臂** —— 三项乘性中性同时成立时,fork 逐位复现冻结母版。
    ///
    /// 注意 `on` 不参与判据:`on = true` 且三项中性,仍须位级复现母版;这正是
    /// 「fork 接线本身零漂移」的断言点(比 `on = false` 的平凡臂强)。
    pub fn is_neutral(&self) -> bool {
        self.dark == 1.0 && self.puddle_amount == 0.0 && self.spec == 0.0
    }
}

// ---------------------------------------------------------------------------
// 公式面(逐字同源;禁移项、禁改结合序、禁换夹紧形)
// ---------------------------------------------------------------------------

/// 湿街门 ∈ `[0, 1]` = 总门 × 地面带门 × 法线朝上门。
///
/// 地面带:`|hy − ground_y|` 归一到半宽后取 `sstep(1 − dyb)` —— 带心 1、带缘
/// 精确 0(`1 − dyb ≤ 0` 被 [`sstep`] 夹死),带外无拖尾。
/// 朝上门:立面与朝下面(`ny ≤ UP_LO`)一律不湿 —— 雨水不挂墙。
pub fn wet_gate(hy: f32, ny: f32, p: &WetParams) -> f32 {
    let on = if p.on { 1.0 } else { 0.0 };
    let dyb = (hy - p.ground_y).abs() / p.ground_half;
    let band = sstep(1.0 - dyb);
    let up = sstep((ny - UP_LO) * UP_INV);
    on * band * up
}

/// 积水掩码 ∈ `[0, 1]` = 双线性值噪声过 `1 − puddle_amount` 软阈,再乘湿街门。
///
/// 噪声取 [`LATTICE`] 格距的 2D 值噪声(格点 [`hash2`] + [`sstep`] 缓入插值),
/// 不依赖任何烘焙表 —— device 侧同式现算,无额外 SSBO。
///
/// `puddle_amount = 0.0` 时阈值退化为 `nv − 1.0 ≤ 0`,[`sstep`] 夹死 ⇒ **位级
/// 恒 0**(中性臂;`nv` 为格点值的凸组合,f32 舍入下仍恒 `≤ 1.0`)。
pub fn puddle_mask(hx: f32, hz: f32, wet: f32, p: &WetParams) -> f32 {
    let pu = hx * LATTICE;
    let pv = hz * LATTICE;
    let iu = pu.floor();
    let iv = pv.floor();
    let su = sstep(pu - iu);
    let sv = sstep(pv - iv);
    let n00 = hash2(iu, iv);
    let n10 = hash2(iu + 1.0, iv);
    let n01 = hash2(iu, iv + 1.0);
    let n11 = hash2(iu + 1.0, iv + 1.0);
    let a0 = n00 + (n10 - n00) * su;
    let a1 = n01 + (n11 - n01) * su;
    let nv = a0 + (a1 - a0) * sv;
    sstep((nv - (1.0 - p.puddle_amount)) * PUD_SOFT) * wet
}

/// 湿面反照率:按湿街门在 `1.0` 与 `dark` 之间插值,积水处再额外压暗。
///
/// `dark_f` 写成 `dark + (1 − dark)·(1 − wet)`(= `lerp(1, dark, wet)` 的展开
/// 形)而非 `1 + (dark − 1)·wet`:前者在 `dark = 1.0` 时第二项含因子 `0.0`,
/// 乘性归零精确,`dark_f` 恒为 `1.0` 位级 —— 这是中性性质的算术依据。
pub fn wet_albedo(albedo: [f32; 3], wet: f32, pud: f32, p: &WetParams) -> [f32; 3] {
    let dark_f = p.dark + (1.0 - p.dark) * (1.0 - wet);
    let pud_dark = 1.0 - PUD_DARK_K * pud;
    [
        albedo[0] * dark_f * pud_dark,
        albedo[1] * dark_f * pud_dark,
        albedo[2] * dark_f * pud_dark,
    ]
}

/// 湿面粗糙度链,返回 `(rough, alpha, alpha2)`。
///
/// 与 [`wet_albedo`] 同律:`rough0` 取展开形,`wet = 0` 时精确回到 `1.0`
/// (母版干面口径),`wet = 1 且 pud = 0` 时精确回到 `p.rough`。
/// `alpha = rough²`、`alpha2 = alpha²` 与母版 D6 GGX 块的取值口径同字面。
pub fn wet_alpha2(wet: f32, pud: f32, p: &WetParams) -> (f32, f32, f32) {
    let rough0 = p.rough + (1.0 - p.rough) * (1.0 - wet);
    let rough = rough0 * (1.0 - PUD_ROUGH_K * pud);
    let alpha = rough * rough;
    let alpha2 = alpha * alpha;
    (rough, alpha, alpha2)
}

/// GGX 高光项(D·G·F / 4·cos_s·cos_v)。
///
/// **逐字转写**自 `kernels/g31_realism.rx` L920-934 的 D6 GGX 块:同一运算序、
/// 同一 `(dd·dd).max(tiny)` 分母保底、同一 `4·cos_s·cos_v + 0.001` 常数偏置、
/// 同一 `om²·om²·om` 五次幂展开。禁改结合序 —— 母版冻结链的位级对拍锚在此。
///
/// - `cos_s` = 法线·入射,`cos_v` = 法线·出射,`cos_h` = 法线·半角,
///   `cos_wh` = 出射·半角(均已 `max(0)`);
/// - `inv_pi` / `tiny` 由 host 位级传入(母版 `params[41]` / `tiny = 1e-6` 同源)。
#[allow(clippy::too_many_arguments)]
pub fn ggx_spec(
    cos_s: f32,
    cos_v: f32,
    cos_h: f32,
    cos_wh: f32,
    alpha: f32,
    alpha2: f32,
    inv_pi: f32,
    tiny: f32,
) -> f32 {
    let dd = cos_h * cos_h * (alpha2 - 1.0) + 1.0;
    let d_ggx = alpha2 * inv_pi / (dd * dd).max(tiny);
    let k_g = alpha * 0.5;
    let g1_s = cos_s / (cos_s * (1.0 - k_g) + k_g);
    let g1_v = cos_v / (cos_v * (1.0 - k_g) + k_g);
    let g_sm = g1_s * g1_v;
    let om = 1.0 - cos_wh;
    let om2 = om * om;
    let sch5 = om2 * om2 * om;
    let den = 4.0 * cos_s * cos_v + 0.001;
    let dgf = d_ggx * g_sm / den;
    dgf * (F0 + (1.0 - F0) * sch5)
}

/// 湿介质 Schlick 菲涅尔(标量,F0 取 [`F0`])。
///
/// 五次幂写成 `om2 · om2 · om` —— 与 [`ggx_spec`] 内联段同一展开形,禁用
/// `powi(5)`(执行面无该内建,且结合序不同会产生舍入差)。
pub fn fresnel_schlick(cos_v: f32) -> f32 {
    let om = 1.0 - cos_v;
    let om2 = om * om;
    F0 + (1.0 - F0) * (om2 * om2 * om)
}

/// 环境/天空反射权:菲涅尔 × 湿街门 × 总强度 × 积水增益。
///
/// 积水增益 `PUD_REFL_LO + PUD_REFL_HI·pud`:湿面(pud = 0)已有 0.35 的基准
/// 镜面份额,满积水(pud = 1)升到 1.0 —— 水膜与水洼的观感差就在这一段。
///
/// `spec = 0.0` 或 `wet = 0.0` 时精确为 `0.0`(中性臂;加性项完全不注入)。
pub fn refl_weight(wet: f32, pud: f32, cos_v: f32, p: &WetParams) -> f32 {
    fresnel_schlick(cos_v) * wet * p.spec * (PUD_REFL_LO + PUD_REFL_HI * pud)
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// ① `sstep` 双端夹紧、中点恰 0.5、64 点扫描单调不减。
    #[test]
    fn sstep_clamped_and_monotone() {
        assert_eq!(sstep(-1.0), 0.0, "左端须夹到 0");
        assert_eq!(sstep(2.0), 1.0, "右端须夹到 1");
        assert_eq!(sstep(0.5), 0.5, "中点须恰为 0.5");
        let mut prev = -1.0f32;
        for i in 0..64 {
            let t = -0.5 + i as f32 * (2.0 / 63.0);
            let v = sstep(t);
            assert!((0.0..=1.0).contains(&v), "sstep 越界: t={t} → {v}");
            assert!(v >= prev, "sstep 非单调不减: t={t} v={v} prev={prev}");
            prev = v;
        }
    }

    /// ② `fract` 恒 ∈ [0, 1)(含负数侧;取正周期)。
    #[test]
    fn fract_is_unit_interval() {
        for i in 0..128 {
            let x = -4.0 + i as f32 * 0.0625;
            let v = fract(x);
            assert!((0.0..1.0).contains(&v), "fract 越界: x={x} → {v}");
        }
        assert_eq!(fract(2.5), 0.5);
        assert_eq!(fract(-2.5), 0.5, "负数侧须取正周期");
        assert_eq!(fract(3.0), 0.0);
        assert_eq!(fract(-3.0), 0.0);
    }

    /// ③ `hash2` 同输入位级确定 + 32×32 格点扫描恒 ∈ [0, 1) + 不对称。
    #[test]
    fn hash2_deterministic_and_unit_interval() {
        for iu in 0..32 {
            for iv in 0..32 {
                let (a, b) = (iu as f32, iv as f32);
                let h = hash2(a, b);
                assert_eq!(
                    h.to_bits(),
                    hash2(a, b).to_bits(),
                    "hash2 非确定: ({a}, {b})"
                );
                assert!((0.0..1.0).contains(&h), "hash2 越界: ({a}, {b}) → {h}");
            }
        }
        assert!(hash2(3.0, 7.0) != hash2(7.0, 3.0), "hash2 须对 (a, b) 不对称");
    }

    /// ④ 总门关 ⇒ `wet_gate` 恒 0(fork 平凡零漂移臂)。
    #[test]
    fn wet_gate_off_is_zero() {
        let p = WetParams::default();
        assert!(!p.on, "缺省须为关臂");
        for hy in [-1.0f32, 0.0, GROUND_Y_DEFAULT, 0.5, 4.0] {
            for ny in [-1.0f32, 0.0, 0.55, 0.9, 1.0] {
                assert_eq!(wet_gate(hy, ny, &p), 0.0, "总门关须恒 0: hy={hy} ny={ny}");
            }
        }
    }

    /// ⑤ 地面带缘及带外 ⇒ `wet_gate` 精确 0(无拖尾)。
    #[test]
    fn wet_gate_zero_outside_band() {
        let p = WetParams {
            on: true,
            ..WetParams::default()
        };
        assert_eq!(wet_gate(p.ground_y + p.ground_half, 1.0, &p), 0.0, "上带缘");
        assert_eq!(wet_gate(p.ground_y - p.ground_half, 1.0, &p), 0.0, "下带缘");
        assert_eq!(wet_gate(p.ground_y + p.ground_half * 3.0, 1.0, &p), 0.0, "带外");
        assert_eq!(wet_gate(p.ground_y - p.ground_half * 3.0, 1.0, &p), 0.0, "带外");
        // 二进制精确参数面:排除 dyb 的 ±1 ulp 抖动,带缘等式是结构性的而非侥幸。
        let q = WetParams {
            on: true,
            ground_y: 2.0,
            ground_half: 0.5,
            ..WetParams::default()
        };
        assert_eq!(wet_gate(2.5, 1.0, &q), 0.0, "精确参数面上带缘");
        assert_eq!(wet_gate(1.5, 1.0, &q), 0.0, "精确参数面下带缘");
    }

    /// ⑥ 带心 + 正朝上法线 ⇒ `wet_gate` 精确 1.0。
    #[test]
    fn wet_gate_peaks_at_band_center() {
        let p = WetParams {
            on: true,
            ..WetParams::default()
        };
        assert_eq!(wet_gate(p.ground_y, 1.0, &p), 1.0, "带心朝上须满门");
    }

    /// ⑦ 朝下/立面法线(`ny ≤ UP_LO`)⇒ `wet_gate` 精确 0(雨水不挂墙)。
    #[test]
    fn wet_gate_zero_for_downward_normals() {
        let p = WetParams {
            on: true,
            ..WetParams::default()
        };
        for ny in [-1.0f32, -0.2, 0.0, 0.3, UP_LO] {
            assert_eq!(wet_gate(p.ground_y, ny, &p), 0.0, "非朝上须关门: ny={ny}");
        }
        assert!(wet_gate(p.ground_y, 0.95, &p) > 0.0, "朝上须开门");
    }

    /// ⑧ `wet_gate` 随 |hy − ground_y| 单调不增。
    #[test]
    fn wet_gate_monotone_in_band_distance() {
        let p = WetParams {
            on: true,
            ..WetParams::default()
        };
        let mut prev = f32::INFINITY;
        for i in 0..=32 {
            let d = i as f32 * (p.ground_half / 32.0);
            let v = wet_gate(p.ground_y + d, 1.0, &p);
            assert!(v <= prev, "带内须单调不增: d={d} v={v} prev={prev}");
            prev = v;
        }
        assert_eq!(prev, 0.0, "扫到带缘须归 0");
    }

    /// ⑨ `puddle_amount = 0` ⇒ `puddle_mask` **位级** 0(中性性质的底座)。
    #[test]
    fn puddle_mask_zero_when_amount_zero() {
        let p = WetParams {
            on: true,
            puddle_amount: 0.0,
            ..WetParams::default()
        };
        for i in 0..20 {
            for j in 0..20 {
                let hx = -5.0 + i as f32 * 0.5;
                let hz = -5.0 + j as f32 * 0.5;
                let m = puddle_mask(hx, hz, 1.0, &p);
                assert_eq!(
                    m.to_bits(),
                    0.0f32.to_bits(),
                    "无积水臂须位级 0: ({hx}, {hz}) → {m}"
                );
            }
        }
    }

    /// ⑩ 覆盖率随 `puddle_amount` 单调不减,且满覆盖严格多于零覆盖。
    #[test]
    fn puddle_coverage_grows_with_amount() {
        let mut fracs = Vec::new();
        for pa in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let p = WetParams {
                on: true,
                puddle_amount: pa,
                ..WetParams::default()
            };
            let mut hit = 0u32;
            for i in 0..64 {
                for j in 0..64 {
                    let hx = -8.0 + i as f32 * 0.25;
                    let hz = -8.0 + j as f32 * 0.25;
                    if puddle_mask(hx, hz, 1.0, &p) > 0.5 {
                        hit += 1;
                    }
                }
            }
            fracs.push(hit as f32 / 4096.0);
        }
        for w in fracs.windows(2) {
            assert!(w[1] >= w[0], "覆盖率须随 puddle_amount 单调不减: {fracs:?}");
        }
        assert_eq!(fracs[0], 0.0, "pa = 0 覆盖率须恰为 0");
        assert!(
            fracs[4] > fracs[0],
            "pa = 1 须严格多于 pa = 0: {fracs:?}"
        );
    }

    /// ⑪ `wet = 0` ⇒ `puddle_mask` 恒 0(干面无积水)。
    #[test]
    fn puddle_mask_zero_when_dry() {
        let p = WetParams {
            on: true,
            puddle_amount: 1.0,
            ..WetParams::default()
        };
        for i in 0..16 {
            let hx = -3.0 + i as f32 * 0.375;
            assert_eq!(puddle_mask(hx, 1.25, 0.0, &p), 0.0, "wet = 0 须关积水: hx={hx}");
        }
    }

    /// ⑫ `dark = 1, pud = 0` ⇒ `wet_albedo` **位级恒等**(中性性质主断言)。
    #[test]
    fn wet_albedo_neutral_is_bit_identical() {
        let p = WetParams {
            on: true,
            dark: 1.0,
            ..WetParams::default()
        };
        let a = [0.482_15f32, 0.031_25, 0.913_77];
        for i in 0..=16 {
            let wet = i as f32 / 16.0;
            let out = wet_albedo(a, wet, 0.0, &p);
            for c in 0..3 {
                assert_eq!(
                    out[c].to_bits(),
                    a[c].to_bits(),
                    "中性臂须位级恒等: wet={wet} ch{c} → {}",
                    out[c]
                );
            }
        }
    }

    /// ⑬ `dark < 1` 时 `wet_albedo` 随 wet 单调压暗,积水再额外压暗。
    #[test]
    fn wet_albedo_darkens_with_wet() {
        let p = WetParams {
            on: true,
            ..WetParams::default()
        };
        let a = [0.6f32, 0.6, 0.6];
        let mut prev = f32::INFINITY;
        for i in 0..=16 {
            let wet = i as f32 / 16.0;
            let v = wet_albedo(a, wet, 0.0, &p)[0];
            assert!(v <= prev, "须随 wet 单调不增: wet={wet} v={v} prev={prev}");
            prev = v;
        }
        assert!(prev < a[0], "全湿须暗于干面: {prev} vs {}", a[0]);
        let no_pud = wet_albedo(a, 1.0, 0.0, &p)[0];
        let full_pud = wet_albedo(a, 1.0, 1.0, &p)[0];
        assert!(full_pud < no_pud, "积水须额外压暗: {full_pud} vs {no_pud}");
    }

    /// ⑭ `wet_alpha2` 两端精确 + `alpha2 == rough⁴`。
    #[test]
    fn wet_alpha2_endpoints_and_power() {
        let p = WetParams {
            on: true,
            ..WetParams::default()
        };
        let (r0, a0, a20) = wet_alpha2(0.0, 0.0, &p);
        assert_eq!(r0, 1.0, "干面 roughness 须精确回到 1.0(母版口径)");
        assert_eq!(a0, 1.0);
        assert_eq!(a20, 1.0);
        let (r1, a1, a21) = wet_alpha2(1.0, 0.0, &p);
        assert_eq!(
            r1.to_bits(),
            p.rough.to_bits(),
            "全湿无积水须精确回到 p.rough"
        );
        assert_eq!(a1.to_bits(), (r1 * r1).to_bits(), "alpha 须为 rough²");
        assert_eq!(a21.to_bits(), (a1 * a1).to_bits(), "alpha2 须为 rough⁴");
        let (r2, _, _) = wet_alpha2(1.0, 1.0, &p);
        assert!(r2 < r1, "积水须进一步降糙: {r2} vs {r1}");
    }

    /// ⑮ `fresnel_schlick` 两端位级精确(垂直 = F0,掠射 = 1.0)且单调不增。
    #[test]
    fn fresnel_endpoints_bit_exact() {
        assert_eq!(
            fresnel_schlick(1.0).to_bits(),
            F0.to_bits(),
            "垂直入射须恰为 F0"
        );
        assert_eq!(
            fresnel_schlick(0.0).to_bits(),
            1.0f32.to_bits(),
            "掠射须恰为 1.0"
        );
        let mut prev = f32::INFINITY;
        for i in 0..=20 {
            let c = i as f32 / 20.0;
            let v = fresnel_schlick(c);
            assert!((F0..=1.0).contains(&v), "F 越界: cos={c} → {v}");
            assert!(v <= prev, "F 须随 cos 单调不增: cos={c}");
            prev = v;
        }
    }

    /// ⑯ `spec = 0` 或 `wet = 0` ⇒ `refl_weight` 精确 0(加性项不注入)。
    #[test]
    fn refl_weight_zero_arms() {
        let off = WetParams {
            on: true,
            spec: 0.0,
            ..WetParams::default()
        };
        let on = WetParams {
            on: true,
            ..WetParams::default()
        };
        for cos_v in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            assert_eq!(
                refl_weight(1.0, 1.0, cos_v, &off),
                0.0,
                "spec = 0 须关反射: cos={cos_v}"
            );
            assert_eq!(
                refl_weight(0.0, 1.0, cos_v, &on),
                0.0,
                "wet = 0 须关反射: cos={cos_v}"
            );
        }
        assert!(refl_weight(1.0, 1.0, 0.2, &on) > 0.0, "湿 + 积水 + 掠射须有反射权");
        assert!(
            refl_weight(1.0, 1.0, 0.2, &on) > refl_weight(1.0, 0.0, 0.2, &on),
            "积水须抬高反射权"
        );
    }

    /// ⑰ 缺省面合法 + 越界/NaN 表逐条拒录 + 闭集端点收放正确。
    #[test]
    fn validate_default_ok_and_out_of_range_red() {
        assert!(WetParams::default().validate().is_ok(), "缺省面须合法");
        let base = WetParams::default();
        let nan = f32::NAN;
        let bad = [
            WetParams { dark: 0.2, ..base },
            WetParams { dark: 1.01, ..base },
            WetParams { rough: 0.02, ..base },
            WetParams { rough: 0.61, ..base },
            WetParams { spec: -0.1, ..base },
            WetParams { spec: 4.01, ..base },
            WetParams {
                puddle_amount: -0.1,
                ..base
            },
            WetParams {
                puddle_amount: 1.01,
                ..base
            },
            WetParams {
                ground_half: 0.0,
                ..base
            },
            WetParams {
                ground_half: -0.156,
                ..base
            },
            WetParams { dark: nan, ..base },
            WetParams { rough: nan, ..base },
            WetParams { spec: nan, ..base },
            WetParams {
                puddle_amount: nan,
                ..base
            },
            WetParams {
                ground_y: nan,
                ..base
            },
            WetParams {
                ground_half: nan,
                ..base
            },
        ];
        for (i, p) in bad.iter().enumerate() {
            assert!(p.validate().is_err(), "越界面 #{i} 须拒录: {p:?}");
        }
        // 闭集端点:上界含、下界不含。
        assert!(WetParams { dark: 1.0, ..base }.validate().is_ok());
        assert!(WetParams { rough: 0.6, ..base }.validate().is_ok());
        assert!(WetParams { spec: 0.0, ..base }.validate().is_ok());
        assert!(WetParams { spec: 4.0, ..base }.validate().is_ok());
        assert!(
            WetParams {
                puddle_amount: 0.0,
                ..base
            }
            .validate()
            .is_ok()
        );
        assert!(
            WetParams {
                puddle_amount: 1.0,
                ..base
            }
            .validate()
            .is_ok()
        );
    }

    /// ⑱ `pack` 槽位序与长度冻结(`params[49..56)`)。
    #[test]
    fn pack_slot_order_is_frozen() {
        assert_eq!(WET_PARAM_COUNT, 7, "湿街段恒 7 字");
        assert_eq!(WET_PARAM_BASE, 49, "湿街段起始下标恒 49");
        assert_eq!(
            WET_PARAM_BASE + WET_PARAM_COUNT,
            56,
            "湿街段须恰好收在 PARAMS_LEN=56 边界内(不越界、不留缝)"
        );
        let off = WetParams::default();
        let a = off.pack();
        assert_eq!(a.len(), WET_PARAM_COUNT);
        assert_eq!(a[0], 0.0, "总门关 ⇒ params[49] = 0");
        let on = WetParams { on: true, ..off };
        let b = on.pack();
        assert_eq!(b[0], 1.0, "总门开 ⇒ params[49] = 1");
        assert_eq!(b[1].to_bits(), on.dark.to_bits(), "params[50] = dark");
        assert_eq!(b[2].to_bits(), on.rough.to_bits(), "params[51] = rough");
        assert_eq!(b[3].to_bits(), on.spec.to_bits(), "params[52] = spec");
        assert_eq!(
            b[4].to_bits(),
            on.puddle_amount.to_bits(),
            "params[53] = puddle_amount"
        );
        assert_eq!(b[5].to_bits(), on.ground_y.to_bits(), "params[54] = ground_y");
        assert_eq!(
            b[6].to_bits(),
            on.ground_half.to_bits(),
            "params[55] = ground_half"
        );
        assert!(b.iter().all(|v| v.is_finite()), "参数面须全有限");
    }

    /// ⑲ `is_neutral` 恰为三项判据,且中性面下全公式面乘性恒等。
    #[test]
    fn is_neutral_exact_predicate() {
        let n = WetParams {
            on: true,
            dark: 1.0,
            puddle_amount: 0.0,
            spec: 0.0,
            ..WetParams::default()
        };
        assert!(n.is_neutral(), "三项中性须判中性(与 on 无关)");
        assert!(!WetParams { dark: 0.999, ..n }.is_neutral(), "dark 偏离须非中性");
        assert!(
            !WetParams {
                puddle_amount: 0.001,
                ..n
            }
            .is_neutral(),
            "积水偏离须非中性"
        );
        assert!(!WetParams { spec: 0.001, ..n }.is_neutral(), "spec 偏离须非中性");
        assert!(!WetParams::default().is_neutral(), "缺省面非中性");
        // 中性面 ⇒ 反照率位级恒等、反射权恒 0(fork 位级复现母版)。
        let a = [0.3f32, 0.45, 0.7];
        for i in 0..=8 {
            let wet = i as f32 / 8.0;
            let out = wet_albedo(a, wet, 0.0, &n);
            for c in 0..3 {
                assert_eq!(out[c].to_bits(), a[c].to_bits(), "中性面 albedo ch{c}");
            }
            assert_eq!(refl_weight(wet, 0.0, 0.5, &n), 0.0, "中性面反射权须恒 0");
        }
    }

    /// ⑳ `ggx_spec`(母版 D6 转写块)恒有限非负,峰值落在半角对齐处。
    #[test]
    fn ggx_spec_finite_and_peaked() {
        let inv_pi = 1.0 / std::f32::consts::PI;
        let tiny = 1e-6f32;
        let p = WetParams {
            on: true,
            ..WetParams::default()
        };
        let (_, alpha, alpha2) = wet_alpha2(1.0, 0.0, &p);
        for cos_h in [0.1f32, 0.5, 0.9, 1.0] {
            for cos_wh in [0.0f32, 0.4, 1.0] {
                let v = ggx_spec(0.7, 0.6, cos_h, cos_wh, alpha, alpha2, inv_pi, tiny);
                assert!(
                    v.is_finite() && v >= 0.0,
                    "GGX 非法: cos_h={cos_h} cos_wh={cos_wh} → {v}"
                );
            }
        }
        let peak = ggx_spec(0.7, 0.6, 1.0, 0.4, alpha, alpha2, inv_pi, tiny);
        let side = ggx_spec(0.7, 0.6, 0.5, 0.4, alpha, alpha2, inv_pi, tiny);
        assert!(peak > side, "峰值须在半角对齐处: {peak} vs {side}");
        // 掠射(cos_wh = 0 ⇒ sch5 = 1)的 Fresnel 因子须强于垂直(sch5 = 0)。
        let grazing = ggx_spec(0.7, 0.6, 1.0, 0.0, alpha, alpha2, inv_pi, tiny);
        let normal = ggx_spec(0.7, 0.6, 1.0, 1.0, alpha, alpha2, inv_pi, tiny);
        assert!(grazing > normal, "掠射 Fresnel 须强于垂直: {grazing} vs {normal}");
    }
}
