//! 体积云前端(G40;M112 契约「云与雾共用同一 Froxel 基础设施、**两个前端**」
//! 的云侧兑现)。
//!
//! 本模块是 host 金标准面——`kernels/g40_volumetric_cloud.rx` device 腿与本文
//! 公式逐字同源,对拍容差由 harness 标定。**host 纯 safe 确定性、零外部 crate、
//! 零贴图资产**(3D 噪声全部程序化现算),同参数双跑位级一致。
//!
//! ## 方案来源
//!
//! 复现 HanPi Volume Cloud(<https://github.com/AshenOneArt/HPVolumeCloud>,
//! MIT + 署名要求;其自身派生自 Unity HDRP 体积云)。五层结构:
//!
//! 1. **密度模型**(Schneider/Nubis 范式,SIGGRAPH 2015 Horizon Zero Dawn):
//!    球壳 slab + 2D weather map 覆盖度 + 3D Perlin-Worley 基础塑形 +
//!    Worley 高频侵蚀 + 三型高度廓形(Cu 积云 / Tcu 浓积云 / Cb 积雨云)。
//! 2. **主射线自适应步进**:空域走大步跳过,命中云体切小步精积分。
//! 3. **锥形光步进**:几何级数步宽(比率 [`CONE_RATIO`]),反解首步宽度使 N 步
//!    正好覆盖光程——加步数真正提精度而非只延伸覆盖。
//! 4. **双瓣 HG 相位 + Hillaire 2020 三倍频多重散射**:attenuation /
//!    contribution / eccentricity 三参数逐倍频独立衰减。
//! 5. **phi_fwd 各向同性漫射场**(参考项目的招牌贡献,见下)。
//!
//! ## phi_fwd:各向同性多重散射漫射场
//!
//! 单次散射(HG)方向性极强,厚云内部随光学深度指数变暗、云底近乎死黑;而真实
//! 高反照率水云(ω₀ ≈ 0.999)在 τ > 1 后亮度**趋于饱和**,云底与侧面有可见漫射
//! 发光。phi_fwd 专门建模这一「多重散射后各向同性漫射场」,作为与 Hillaire 乘性
//! MS **正交的加性独立项**注入。
//!
//! 物理链路:辐射传输方程 → 扩散近似(τ ≫ 1,g_eff → 0)→ Helmholtz 型 +
//! 格林函数 `G(x,x') = exp(−κr)/(4πr)` → 有效源项 → 沿太阳方向 1D 离散:
//!
//! ```text
//! φ(x) ≈ Σ_j  T_abs,j · (σ_s,j·Δs_j) · σ_tr,j · C_iso,j
//!             · (C_top,j · C_bottom,j) · exp(−∫κ ds) · (1/r_j)
//! ```
//!
//! | 因子 | 含义 |
//! |---|---|
//! | `T_abs` | 低吸收无逃逸存活率 `exp(−(1−ω₀)·OD)`——不是单次直射透射率 |
//! | `σ_s·Δs` | 局部散射沉积(有密度才有源) |
//! | `σ_tr` | `1/D` 的介质尺度近似(常数 3 吸收进整体归一化) |
//! | `C_iso` | 各向同性建立权重 `1 − exp(−τ_iso)`——边界首步仍归方向性散射 |
//! | `C_top` | 上层/侧向边界受光置信度(2D 高度代理差分法线的 wrap 光照) |
//! | `C_bottom` | 底部入射置信度(太阳不从云下注入,底面软化恢复) |
//! | `exp(−∫κ ds)` | 扩散体积衰减 |
//! | `1/r_j` | 点源几何扩展 |
//!
//! 因 ω₀ 写死、g_eff = 0 ⇒ `σ_tr = σ_t`、`κ = σ_t·√(3(1−ω₀))`,故
//! `∫κ ds = √(3(1−ω₀))·OD`——每步只需本步 OD 乘以 [`PHIFWD_KAPPA_OD_SCALE`]
//! 这个编译期常量,无需逐步开方。
//!
//! ## 坐标约定
//!
//! 与 [`super::sky`] 同:**Y-up 右手系**,`+Y` 天顶 / `+X` 东 / `+Z` 北;世界
//! 原点在地表观察者处,球心在 `(0, −EARTH_RADIUS_M, 0)`。

use super::atmosphere::{AtmosphereError, FroxelVolume, WeatherMap};
use super::sky::{EARTH_RADIUS_M, Sky};

// ---------------------------------------------------------------------------
// 冻结常量面(与 g40_volumetric_cloud.rx 逐字同源)
// ---------------------------------------------------------------------------

/// 多重散射倍频数(Hillaire 2020;参考实现 `NUM_MULTI_SCATTERING_OCTAVES`)。
pub const MS_OCTAVES: usize = 3;

/// phi_fwd 单散射反照率(水云;参考实现 `HP_PHIFWD_OMEGA0` 写死值)。
pub const PHIFWD_OMEGA0: f32 = 0.999;

/// phi_fwd 的 `κ/σ_t` 比值 `√(3(1−ω₀))` ≈ 0.0548。
///
/// 由 `g_eff = 0 ⇒ σ_tr = σ_t` 与 `σ_a = (1−ω₀)σ_t` 代入
/// `κ = √(3 σ_a σ_tr)` 得 `κ = σ_t·√(3(1−ω₀))`,于是路径积分
/// `∫κ ds = √(3(1−ω₀))·OD` 只需一次常量乘,无需逐步开方。
pub const PHIFWD_KAPPA_OD_SCALE: f32 = 0.054_772_255;

/// 锥形光步几何级数相邻步比率(参考实现 `CONE_RATIO`)。
pub const CONE_RATIO: f32 = 2.0;
/// 锥形光步首步宽度下限(m;防步数过多时首步退化)。
pub const CONE_MIN_STEP: f32 = 5.0;
/// 锥形光步总覆盖距离上限(m;超出后密度≈0 对自阴影无贡献)。
pub const CONE_MAX_DISTANCE: f32 = 6_000.0;

/// 主步进「云层厚度」近端步长 cap 比例(slab 厚度的 1/16)。
const STEP_NEAR_CAP_RATIO: f32 = 0.0625;
/// 主步进「云层厚度」远端步长 cap 比例(slab 厚度的 1/2)。
const STEP_FAR_CAP_RATIO: f32 = 0.5;
/// 主步进绝对视距步长比(`absDist / 8`;解决近云跨步)。
const STEP_VIEW_DIV: f32 = 8.0;
/// 主步进迭代保险上限倍率(相对 `primary_steps`)。
const STEP_ITER_SAFETY: u32 = 4;

/// phi_fwd 默认强度(本实现量纲下的标定值;见
/// [`CloudParams::phi_fwd_intensity`] 关于与参考实现建议区间差两个数量级的
/// 说明)。经 `--phi-intensity` 扫参标定:≲5 观感与关臂无异,20 起云底漫射
/// 发光可辨,60 上下饱和,取 30 为默认。
pub const DEFAULT_PHI_FWD_INTENSITY: f32 = 30.0;

/// 密度探测阈值(低于此值视为空域,走大步)。
const DENSITY_PROBE_EPS: f32 = 1e-4;
/// 视线透射率提前终止阈值(低于此值云已不透明,停止步进)。
const TRANSMITTANCE_CUTOFF: f32 = 0.003;

// ---------------------------------------------------------------------------
// 云型与参数面
// ---------------------------------------------------------------------------

/// 云型(高度廓形闭集;参考实现的 Cu / Tcu / Cb 三档)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudType {
    /// 淡积云:扁平、贴云底、覆盖度低。
    Cumulus,
    /// 浓积云:塔状、纵向发展。
    TowerCumulus,
    /// 积雨云:占满整个 slab、密度最高。
    Cumulonimbus,
}

impl CloudType {
    /// 归一化高度 `h ∈ [0,1]` 处的廓形权重(0 = 无云,1 = 满密度)。
    ///
    /// 三型共用「底部 smoothstep 起 + 顶部 smoothstep 落」的形状,差别在起落
    /// 位置:积云集中在 slab 下半,积雨云占满全高。
    pub fn profile(self, h: f32) -> f32 {
        let (b0, b1, t0, t1) = match self {
            CloudType::Cumulus => (0.00, 0.12, 0.28, 0.55),
            CloudType::TowerCumulus => (0.00, 0.10, 0.55, 0.85),
            CloudType::Cumulonimbus => (0.00, 0.06, 0.82, 1.00),
        };
        smoothstep(b0, b1, h) * (1.0 - smoothstep(t0, t1, h))
    }

    /// 该型的密度倍率(参考实现 `_HP_DensityMultiplier{Cu,Tcu,Cb}`)。
    pub fn density_multiplier(self) -> f32 {
        match self {
            CloudType::Cumulus => 1.0,
            CloudType::TowerCumulus => 1.35,
            CloudType::Cumulonimbus => 1.8,
        }
    }

    /// 由 weather map 的 type 通道(`[0,1]`)选型。
    pub fn from_channel(t: f32) -> Self {
        if t < 0.4 {
            CloudType::Cumulus
        } else if t < 0.75 {
            CloudType::TowerCumulus
        } else {
            CloudType::Cumulonimbus
        }
    }
}

/// 体积云参数面。
///
/// 字段名后的括号标注参考实现 `VolumetricClouds.hlsl` 里的对应 uniform,便于
/// 逐条对照移植正确性。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudParams {
    // ── 云层几何 ──────────────────────────────────────────────────────────
    /// 云层底高度(m,地表以上;`_LowestCloudAltitude`)。
    pub lowest_altitude_m: f32,
    /// 云层顶高度(m,地表以上;`_HighestCloudAltitude`)。
    pub highest_altitude_m: f32,
    /// 主射线最大步进距离(m;`_MaxRayMarchingDistance`)。
    pub max_ray_distance_m: f32,

    // ── 步进预算 ──────────────────────────────────────────────────────────
    /// 主射线基准步数(`_NumPrimarySteps`)。
    pub primary_steps: u32,
    /// 光步进步数(`_NumLightSteps`)。
    pub light_steps: u32,

    // ── 密度塑形 ──────────────────────────────────────────────────────────
    /// 基础噪声世界尺度(m;越大云团越大)。
    pub noise_scale_m: f32,
    /// 侵蚀噪声世界尺度(m;`_HP_DetailNoiseScale`)。
    pub detail_scale_m: f32,
    /// 侵蚀强度(`_HP_DetailStrength*`)。
    pub detail_strength: f32,
    /// 密度阈值(低于此值削零;`_HP_DensityThreshold`)。
    pub density_threshold: f32,
    /// 密度总倍率(`_HP_DensityMultiplier`)。
    pub density_multiplier: f32,
    /// 覆盖度总倍率(weather map r 通道的强度调制)。
    pub coverage_multiplier: f32,
    /// 云底软化高度(归一化;`_HP_BottomSmoothHeight`)。
    pub bottom_smooth_height: f32,
    /// 消光系数基准(m⁻¹;`density × sigma_t` = σ_t)。
    pub sigma_t: f32,
    /// weather map 覆盖的世界尺寸(m;`_HP_WeatherMapWorldSize`)。
    pub weather_world_size_m: f32,
    /// 风场偏移(m;基础噪声随时间平移)。
    pub wind_offset_m: [f32; 2],

    // ── 光照 ──────────────────────────────────────────────────────────────
    /// 前向散射偏心率(`_HP_ForwardEccentricity`;银边效果)。
    pub forward_eccentricity: f32,
    /// 后向散射偏心率(`_HP_BackwardEccentricity`;背光晕)。
    pub backward_eccentricity: f32,
    /// 多重散射消光衰减率(逐倍频;`_HP_MS_Attenuation`)。
    pub ms_attenuation: f32,
    /// 多重散射能量权重(逐倍频;`_HP_MS_Contribution`)。
    pub ms_contribution: f32,
    /// 多重散射相位偏心率衰减率(逐倍频;`_HP_MS_Eccentricity`)。
    pub ms_eccentricity: f32,
    /// 云顶环境光倍增(`_HP_AmbientTopMultiplier`)。
    pub ambient_top_multiplier: f32,
    /// 云底环境光倍增(`_HP_AmbientBottomMultiplier`)。
    pub ambient_bottom_multiplier: f32,
    /// 向上透射率 AO 强度(`_HP_AOUpwardScale`)。
    pub ao_upward_scale: f32,
    /// powder effect 强度(`_PowderEffectIntensity`)。
    pub powder_intensity: f32,
    /// 低密度散射源 OD 参考尺度(`_HP_ScatterSourceODScale`)。
    pub scatter_source_od_scale: f32,
    /// 低密度散射源曲线指数(`_HP_ScatterSourceCurvePow`)。
    pub scatter_source_curve_pow: f32,

    // ── phi_fwd 漫射场 ────────────────────────────────────────────────────
    /// 总强度(0 = 关闭本项;`_HP_HpHpPhiFwd_Intensity`)。
    ///
    /// **量纲注意**:参考实现建议 0.1~2.0,本实现的可用区间高两个数量级
    /// (默认 [`DEFAULT_PHI_FWD_INTENSITY`])。原因是 φ 的量纲由
    /// `σ_s·Δs · σ_tr · (1/r)` 决定,而三者都随 σ_t 的标定与光步的米制尺度走
    /// ——本实现 `sigma_t` 取 0.075 m⁻¹、锥形光步首步约 20m、`1/r` 以米计,
    /// 得到的 φ 比参考实现的标定小约 100×。这是**归一化常数的差异,不是公式
    /// 差异**(格林函数各因子逐条同源);参考实现自己也把扩散系数里的常数 3
    /// 「吸收到整体归一化强度里」,同一处置。
    pub phi_fwd_intensity: f32,
    /// 深度衰减指数(`_HP_HpHpPhiFwd_DepthPow`)。
    pub phi_fwd_depth_pow: f32,
    /// 深度曲线垂直偏移(`_HP_HpHpPhiFwd_DepthBias`)。
    pub phi_fwd_depth_bias: f32,
    /// 边界受光置信度强度(0 = 禁用;`_HP_HpHpPhiFwd_BoundaryConfidence`)。
    pub phi_fwd_boundary_confidence: f32,
    /// 各向同性 MS 建立速度(`_HP_HpHpPhiFwd_MSBuildScale`)。
    pub phi_fwd_ms_build_scale: f32,
    /// 最终软饱和压缩强度(0 = 线性不压缩;`_HP_HpHpPhiFwd_Compress`)。
    pub phi_fwd_compress: f32,
}

impl Default for CloudParams {
    /// 参考实现推荐值面(积云 slab 1.5km~4km,64 主步 / 6 光步)。
    fn default() -> Self {
        Self {
            lowest_altitude_m: 1_500.0,
            highest_altitude_m: 4_000.0,
            max_ray_distance_m: 50_000.0,
            primary_steps: 64,
            light_steps: 6,
            noise_scale_m: 3_200.0,
            detail_scale_m: 380.0,
            detail_strength: 0.42,
            density_threshold: 0.06,
            density_multiplier: 1.35,
            coverage_multiplier: 1.0,
            bottom_smooth_height: 0.18,
            sigma_t: 0.075,
            weather_world_size_m: 40_000.0,
            wind_offset_m: [0.0, 0.0],
            forward_eccentricity: 0.85,
            backward_eccentricity: 0.30,
            ms_attenuation: 0.5,
            ms_contribution: 0.5,
            ms_eccentricity: 0.5,
            ambient_top_multiplier: 1.5,
            ambient_bottom_multiplier: 0.65,
            ao_upward_scale: 1.0,
            powder_intensity: 0.6,
            scatter_source_od_scale: 0.06,
            scatter_source_curve_pow: 1.0,
            phi_fwd_intensity: DEFAULT_PHI_FWD_INTENSITY,
            phi_fwd_depth_pow: 1.0,
            phi_fwd_depth_bias: 0.05,
            phi_fwd_boundary_confidence: 1.0,
            phi_fwd_ms_build_scale: 1.0,
            phi_fwd_compress: 0.6,
        }
    }
}

impl CloudParams {
    /// slab 厚度(m)。
    pub fn slab_thickness_m(&self) -> f32 {
        (self.highest_altitude_m - self.lowest_altitude_m).max(1.0)
    }

    /// 关闭 phi_fwd 的同参数副本(对照臂;只动 intensity 单字段)。
    pub fn without_phi_fwd(&self) -> Self {
        Self {
            phi_fwd_intensity: 0.0,
            ..*self
        }
    }
}

// ---------------------------------------------------------------------------
// 逐点云属性
// ---------------------------------------------------------------------------

/// 单点云属性(参考实现 `struct CloudProperties` 同构)。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CloudProperties {
    /// 归一化密度 `[0,1]`。
    pub density: f32,
    /// 环境遮蔽(几何代理)。
    pub ambient_occlusion: f32,
    /// slab 归一化真实海拔高度 `[0,1]`(与廓形无关,单调反映竖直距离)。
    pub height: f32,
    /// 云体 profile 局部高度(0 = 云底,1 = 云顶;供 AO / phi_fwd 使用)。
    pub local_height: f32,
    /// 消光系数(m⁻¹;`density × sigma_t`)。
    pub sigma_t: f32,
}

/// 视线积分结果(参考实现 `struct VolumetricRayResult` 同构)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumetricRayResult {
    /// 累计入散射(RGB,scene-linear)。
    pub in_scattering: [f32; 3],
    /// 视线剩余透射率(1 = 全透,0 = 全遮)。
    pub transmittance: f32,
    /// 加权平均命中距离(m;供时域重投影,无命中时为 `f32::INFINITY`)。
    pub mean_distance: f32,
    /// 是否为无效射线(未与 slab 相交)。
    pub invalid_ray: bool,
}

impl Default for VolumetricRayResult {
    fn default() -> Self {
        Self {
            in_scattering: [0.0; 3],
            transmittance: 1.0,
            mean_distance: f32::INFINITY,
            invalid_ray: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 云体求值器
// ---------------------------------------------------------------------------

/// 体积云求值器(密度场 + 光照 + 视线积分的单一入口)。
///
/// 建造期绑定天空(照明事实源)与 weather map(覆盖度事实源),之后所有查询为
/// 纯函数——逐像素独立、可并行、位级确定。
pub struct CloudRenderer<'a> {
    params: CloudParams,
    sky: &'a Sky,
    weather: &'a WeatherMap,
    /// 烘焙噪声体(host/device 共享事实源)。
    noise: &'a NoiseVolumes,
    /// 预算的太阳方向(Y-up 单位向量)。
    sun_dir: [f32; 3],
    /// 预算的太阳色。
    sun_color: [f32; 3],
    /// 预算的环境光探针(已乘 top/bottom 倍率)。
    ambient_top: [f32; 3],
    ambient_bottom: [f32; 3],
    /// 球心(世界系;`(0, −R, 0)`)。
    planet_center: [f32; 3],
}

impl<'a> CloudRenderer<'a> {
    /// 建造(预算照明常量;不做任何 IO)。
    ///
    /// `noise` 由 [`NoiseVolumes::bake`] 一次性产出并可跨帧/跨预设复用——烘焙
    /// 是本模块最重的一次性开销,禁逐帧重烘。
    pub fn new(
        params: CloudParams,
        sky: &'a Sky,
        weather: &'a WeatherMap,
        noise: &'a NoiseVolumes,
    ) -> Self {
        let (top, bottom) = sky.ambient_probe();
        Self {
            params,
            sky,
            weather,
            noise,
            sun_dir: sky.sun_direction(),
            sun_color: sky.sun_color(),
            ambient_top: mul3(top, params.ambient_top_multiplier),
            ambient_bottom: mul3(bottom, params.ambient_bottom_multiplier),
            planet_center: [0.0, -EARTH_RADIUS_M, 0.0],
        }
    }

    /// 参数面(只读)。
    pub fn params(&self) -> CloudParams {
        self.params
    }

    /// 绑定的天空(只读)。
    pub fn sky(&self) -> &Sky {
        self.sky
    }

    // -- weather map 采样 --------------------------------------------------

    /// 世界 XZ → weather map UV(超出覆盖范围时环绕平铺)。
    fn weather_uv(&self, x: f32, z: f32) -> (f32, f32) {
        let s = self.params.weather_world_size_m.max(1.0);
        (fract(x / s + 0.5), fract(z / s + 0.5))
    }

    /// weather map 双线性采样 → `(coverage, humidity, type)`。
    fn sample_weather(&self, x: f32, z: f32) -> [f32; 3] {
        let (u, v) = self.weather_uv(x, z);
        let w = self.weather.width.max(1);
        let h = self.weather.height.max(1);
        let fx = u * w as f32 - 0.5;
        let fy = v * h as f32 - 0.5;
        let x0 = fx.floor();
        let y0 = fy.floor();
        let tx = fx - x0;
        let ty = fy - y0;
        let wrap = |i: f32, n: u32| -> usize {
            let n_i = n as i64;
            (((i as i64 % n_i) + n_i) % n_i) as usize
        };
        let ix0 = wrap(x0, w);
        let ix1 = wrap(x0 + 1.0, w);
        let iy0 = wrap(y0, h);
        let iy1 = wrap(y0 + 1.0, h);
        let px = |ix: usize, iy: usize| -> [f32; 3] { self.weather.pixels[iy * w as usize + ix] };
        let p00 = px(ix0, iy0);
        let p10 = px(ix1, iy0);
        let p01 = px(ix0, iy1);
        let p11 = px(ix1, iy1);
        let mut out = [0.0f32; 3];
        for c in 0..3 {
            let a = lerp(p00[c], p10[c], tx);
            let b = lerp(p01[c], p11[c], tx);
            out[c] = lerp(a, b, ty);
        }
        out
    }

    // -- 密度模型 ----------------------------------------------------------

    /// 世界点的云属性求值(参考实现 `EvaluateCloudProperties`)。
    ///
    /// `simple` = true 时跳过高频侵蚀采样,供主步进的快速密度探测使用。
    pub fn evaluate_cloud_properties(&self, p: [f32; 3], simple: bool) -> CloudProperties {
        let mut out = CloudProperties {
            ambient_occlusion: 1.0,
            ..CloudProperties::default()
        };

        // slab 归一化高度(球壳:到球心距离减去地球半径)。
        let altitude = dist3(p, self.planet_center) - EARTH_RADIUS_M;
        let pp = &self.params;
        if altitude < pp.lowest_altitude_m || altitude > pp.highest_altitude_m {
            return out;
        }
        let h = ((altitude - pp.lowest_altitude_m) / pp.slab_thickness_m()).clamp(0.0, 1.0);
        out.height = h;

        // weather map:覆盖度 / 湿度 / 云型。
        let w = self.sample_weather(p[0], p[2]);
        let coverage = (w[0] * pp.coverage_multiplier).clamp(0.0, 1.0);
        if coverage <= 0.0 {
            return out;
        }
        let cloud_type = CloudType::from_channel(w[2]);
        let profile = cloud_type.profile(h);
        if profile <= 0.0 {
            return out;
        }
        // 局部高度:相对本型廓形的归一化位置(0 = 本型云底,1 = 本型云顶)。
        out.local_height = h;

        // 基础塑形:Perlin-Worley 烘焙体三线性平铺取样。
        let wind = [p[0] + pp.wind_offset_m[0], p[1], p[2] + pp.wind_offset_m[1]];
        let base_uvw = mul3(wind, 1.0 / pp.noise_scale_m.max(1.0));
        let base = self.noise.sample_base(base_uvw);

        // 覆盖度重映射:coverage 越高,base 的下限被抬得越高(云越满)。
        let cov_shaped = coverage * profile;
        let mut density = remap(base, 1.0 - cov_shaped, 1.0, 0.0, 1.0);
        if density <= 0.0 {
            return out;
        }

        // 高频侵蚀(simple 模式跳过):Worley FBM 烘焙体从边缘啃掉密度。
        if !simple && pp.detail_strength > 0.0 {
            let det_uvw = mul3(wind, 1.0 / pp.detail_scale_m.max(1.0));
            let erosion = self.noise.sample_detail(det_uvw);
            // 侵蚀在云顶更强(billowy → wispy 过渡),用 h 调制。
            let e_strength = pp.detail_strength * lerp(0.7, 1.0, h);
            density = remap(density, erosion * e_strength, 1.0, 0.0, 1.0);
            if density <= 0.0 {
                return out;
            }
        }

        // 云底软化(避免 slab 底面出现硬切平面)。
        let bottom_soft = smoothstep(0.0, pp.bottom_smooth_height.max(1e-3), h);
        density *= bottom_soft;

        // 阈值 + 倍率 + 云型倍率。
        density = ((density - pp.density_threshold) / (1.0 - pp.density_threshold).max(1e-3))
            .clamp(0.0, 1.0);
        density *= pp.density_multiplier * cloud_type.density_multiplier();

        out.density = density.clamp(0.0, 1.0);
        out.sigma_t = pp.sigma_t;
        // 环境遮蔽几何代理:云内越深越暗。
        out.ambient_occlusion = 1.0 - 0.6 * out.density * (1.0 - h);
        out
    }

    // -- phi_fwd 边界置信度 ------------------------------------------------

    /// 2D 云顶高度代理(参考实现 `HP_EvaluateHpHpPhiFwdTopHeightProxy`)。
    ///
    /// 只用于判断「最近相关边界是否面向太阳」,不参与密度本身的精确评估。
    fn phi_fwd_top_height_proxy(&self, x: f32, z: f32) -> f32 {
        let w = self.sample_weather(x, z);
        (w[0] * self.params.coverage_multiplier).clamp(0.0, 1.0)
    }

    /// 上层/侧向边界受光置信度 `C_top`(参考实现
    /// `HP_EvaluateHpHpPhiFwdBoundaryLight`)。
    ///
    /// 由 2D 高度代理的 XZ 差分重建边界法线,与太阳方向做 SSS 风格 wrap 光照:
    /// `wrap(n) = saturate((n + w) / (1 + w))`。背光/逃逸边界的扩散源不可信,
    /// 由此被抑制——这是「云底不死黑但背光侧也不被错误打亮」的关键。
    fn phi_fwd_boundary_light(&self, p: [f32; 3]) -> f32 {
        let step = (self.params.weather_world_size_m * 0.001).clamp(25.0, 200.0);
        let h_l = self.phi_fwd_top_height_proxy(p[0] - step, p[2]);
        let h_r = self.phi_fwd_top_height_proxy(p[0] + step, p[2]);
        let h_d = self.phi_fwd_top_height_proxy(p[0], p[2] - step);
        let h_u = self.phi_fwd_top_height_proxy(p[0], p[2] + step);

        let slab = self.params.slab_thickness_m();
        let dh_dx = (h_r - h_l) * slab / (2.0 * step).max(1.0);
        let dh_dz = (h_u - h_d) * slab / (2.0 * step).max(1.0);
        let n = normalize3([-dh_dx, 1.0, -dh_dz]);

        let n_dot_l = dot3(n, self.sun_dir);
        const WRAP: f32 = 0.5;
        let wrapped = ((n_dot_l + WRAP) / (1.0 + WRAP)).clamp(0.0, 1.0);
        lerp(
            1.0,
            wrapped,
            self.params.phi_fwd_boundary_confidence.clamp(0.0, 1.0),
        )
    }

    // -- 光步进 + phi_fwd --------------------------------------------------

    /// 太阳光照求值(参考实现 `EvaluateSunLuminance`)。
    ///
    /// 返回 `(方向性亮度, 光路光学深度, phi_fwd 漫射场标量)` 三元组,分别对应
    /// 参考实现的返回值与两个 `out` 参数。
    pub fn evaluate_sun_luminance(
        &self,
        p: [f32; 3],
        local_height: f32,
        phase: [f32; MS_OCTAVES],
    ) -> ([f32; 3], f32, f32) {
        let pp = &self.params;
        // 光路到 slab 顶的距离;被地球遮挡(太阳在地平线下)则无直射。
        let Some(total_light_dist) = self.light_ray_distance(p) else {
            return ([0.0; 3], 0.0, 0.0);
        };

        // ── 锥形采样:固定比率 r,反解首步宽度 w0 使 N 步几何级数正好覆盖
        //    coverDist —— 步数越多 w0 越小、近处越密,而总覆盖不变。
        let cover_dist = total_light_dist.clamp(0.0, CONE_MAX_DISTANCE);
        let num_steps = pp.light_steps.max(1);
        let r = CONE_RATIO;
        let denom = (r.powi(num_steps as i32) - 1.0).max(1e-4);
        let mut cur_width = (cover_dist * (r - 1.0) / denom).max(CONE_MIN_STEP);

        let mut extinction_sum = 0.0f32;
        // phi_fwd:观测点 → 当前步入口的 ∫κ ds 累积。
        let mut kappa_od_sum = 0.0f32;
        let mut t_cum = 1.0f32;
        let mut cum_dist = 0.0f32;
        let mut phi_fwd = 0.0f32;

        // 源可信度(边界项 B_eff = C_top · C_bottom;逐观测点一次,步内共用)。
        let src_confidence = if pp.phi_fwd_intensity > 0.0 {
            let bottom_h = local_height + pp.phi_fwd_depth_bias;
            let column = self.phi_fwd_top_height_proxy(p[0], p[2]);
            let soft_h = (pp.bottom_smooth_height * lerp(1.0, 4.0, column)).max(1e-3);
            let c_bottom = if pp.phi_fwd_depth_pow > 0.0 {
                1.0 - (-bottom_h.max(0.0) / soft_h * pp.phi_fwd_depth_pow).exp()
            } else {
                1.0
            };
            self.phi_fwd_boundary_light(p) * c_bottom
        } else {
            0.0
        };

        for _ in 0..num_steps {
            let step_width = cur_width.min(cover_dist - cum_dist);
            if step_width <= 0.0 {
                break;
            }
            // 步中心距观测点,作 r_j 与 κ 积分终点(中点法则)。
            let dist = cum_dist + step_width * 0.5;
            let sample = add3(p, mul3(self.sun_dir, dist));
            let props = self.evaluate_cloud_properties(sample, false);

            // 局部 σ:每步从 density·sigma_t 拆出;ω₀ 写死。
            let sigma_t = props.density * props.sigma_t;
            let local_od = sigma_t * step_width;

            if pp.phi_fwd_intensity > 0.0 {
                let sigma_s = sigma_t * PHIFWD_OMEGA0;
                // Q:局部散射沉积 ∝ σ_s·Δs。
                let q_src = sigma_s * step_width;
                // 1/D ∝ σ_tr;g_eff = 0 时 σ_tr ≈ σ_t,常数 3 吸收到强度。
                let inv_d = sigma_t;
                // κ·Δs = OD·√(3(1−ω₀));积到步中心(入口累积 + 本步半步)。
                let kappa_step = local_od * PHIFWD_KAPPA_OD_SCALE;
                let per_src_exp = (-(kappa_od_sum + kappa_step * 0.5)).exp();
                // C_iso:各向同性 MS 需经一定光学深度才建立,边界首步仍归方向性。
                let ms_build = 1.0
                    - (-(extinction_sum + local_od * 0.5) * pp.phi_fwd_ms_build_scale.max(0.0))
                        .exp();
                // 几何扩展 1/r_j。
                let inv_r = 1.0 / dist.max(step_width * 0.5);
                phi_fwd += t_cum * q_src * inv_d * src_confidence * ms_build * per_src_exp * inv_r;
                kappa_od_sum += kappa_step;
                // T_abs:无逃逸漫射源只按 σ_a = (1−ω₀)σ_t 极慢衰减。
                t_cum *= (-local_od * (1.0 - PHIFWD_OMEGA0)).exp();
            }

            extinction_sum += local_od;
            cum_dist += step_width;
            cur_width *= CONE_RATIO;
        }

        // ── Hillaire 2020 三参数多重散射:attenuation 缩光学深度,
        //    contribution 缩能量权重,eccentricity 已折进 phase 入参。
        let mut luminance = [0.0f32; 3];
        for (o, ph) in phase.iter().enumerate() {
            let att = pp.ms_attenuation.powi(o as i32);
            let con = pp.ms_contribution.powi(o as i32);
            let trans = (-extinction_sum * att).exp();
            for (lum, sun) in luminance.iter_mut().zip(self.sun_color) {
                *lum += trans * sun * ph * con;
            }
        }

        (luminance, extinction_sum, phi_fwd)
    }

    /// 自点 `p` 沿太阳方向到 slab 顶的距离;太阳被地球遮挡返回 `None`。
    fn light_ray_distance(&self, p: [f32; 3]) -> Option<f32> {
        let o = sub3(p, self.planet_center);
        // 光路撞地球 ⇒ 太阳被遮挡。
        if ray_sphere_near(o, self.sun_dir, EARTH_RADIUS_M).is_some_and(|t| t > 0.0) {
            return None;
        }
        ray_sphere_far(
            o,
            self.sun_dir,
            EARTH_RADIUS_M + self.params.highest_altitude_m,
        )
    }

    // -- 逐步合成 ----------------------------------------------------------

    /// 单步云体合成(参考实现 `EvaluateCloud`)。
    fn evaluate_cloud(
        &self,
        props: CloudProperties,
        p: [f32; 3],
        step_size: f32,
        cos_angle: f32,
        phase: [f32; MS_OCTAVES],
        result: &mut VolumetricRayResult,
    ) {
        let pp = &self.params;
        let extinction = props.density * props.sigma_t;
        let transmittance = (-extinction * step_size).exp();

        let (total_luminance, light_od, phi_fwd) =
            self.evaluate_sun_luminance(p, props.local_height, phase);

        // ── 方向性/环境散射源积分 ────────────────────────────────────────
        // 稀薄边缘的单次散射源由本步散射 OD 决定,而非缩短 extinction 步长近似;
        // 视线透射率仍用完整 extinction 保持云体不透明度。
        let scatter_od = extinction * PHIFWD_OMEGA0 * step_size;
        let mut src = 1.0 - (-scatter_od / pp.scatter_source_od_scale.max(1e-3)).exp();
        src = src
            .clamp(0.0, 1.0)
            .powf(pp.scatter_source_curve_pow.max(0.01));

        // powder effect:背光方向的云边暗化(方向性散射本身不施加,保留银边)。
        let powder = powder_effect(props.density, cos_angle, pp.powder_intensity);

        let mut integ = [0.0f32; 3];
        for c in 0..3 {
            integ[c] = total_luminance[c] * src;
        }

        // ── phi_fwd 加性注入(与 Hillaire 乘性 MS 正交)────────────────────
        // 底部/边界置信度已在 evaluate_sun_luminance 的源项中应用;此处只把
        // 积分结果按完整 transmittance 注入视线散射。
        if pp.phi_fwd_intensity > 0.0 {
            let scalar = phi_fwd * pp.phi_fwd_intensity;
            let mapped = if pp.phi_fwd_compress > 0.0 {
                (1.0 - (-scalar * pp.phi_fwd_compress).exp()) / pp.phi_fwd_compress
            } else {
                scalar
            };
            for (acc, sun) in integ.iter_mut().zip(self.sun_color) {
                let l = mapped * sun;
                *acc += l - l * transmittance;
            }
        }

        // ── 环境光散射 ────────────────────────────────────────────────────
        // upwardAO:用太阳光路真实 OD × sin(仰角) 折算竖直方向遮蔽透射率。
        let sin_elev = self.sun_dir[1].max(0.05);
        let upward_ao = (-light_od * sin_elev * pp.ao_upward_scale.max(0.0)).exp();
        for ((acc, top), bottom) in integ
            .iter_mut()
            .zip(self.ambient_top)
            .zip(self.ambient_bottom)
        {
            let ambient = top * upward_ao + bottom * (1.0 - props.height);
            *acc += ambient * src * powder;
        }

        for (out, add) in result.in_scattering.iter_mut().zip(integ) {
            *out += add * result.transmittance;
        }
        result.transmittance *= transmittance;
    }

    // -- 主射线步进 --------------------------------------------------------

    /// 视线体积积分(参考实现 `HPTraceVolumetricRay`)。
    ///
    /// `origin` 为世界系相机位置,`dir` 为单位视线方向,`max_dist` 为不透明几何
    /// 深度(无几何时传 `f32::INFINITY`)。
    pub fn trace(&self, origin: [f32; 3], dir: [f32; 3], max_dist: f32) -> VolumetricRayResult {
        let mut result = VolumetricRayResult::default();
        let pp = &self.params;

        let Some((t_start, t_len)) = self.slab_intersection(origin, dir) else {
            return result;
        };
        if max_dist < t_start {
            return result;
        }
        let total = t_len.min(max_dist - t_start).min(pp.max_ray_distance_m);
        if total <= 0.0 {
            return result;
        }
        result.invalid_ray = false;

        let cos_angle = dot3(dir, self.sun_dir);
        let phase = self.phase_function(cos_angle);

        let slab = pp.slab_thickness_m();
        let near_cap = slab * STEP_NEAR_CAP_RATIO;
        let far_cap = slab * STEP_FAR_CAP_RATIO;
        // 基准步长由**未裁剪**的 slab 穿越长度导出,而非裁剪后的 total——否则
        // 不透明几何深度一变,步长跟着变,同一段云会因积分精度不同而改变遮挡量
        // (深度裁剪本应只截断、不改采样密度)。
        let step_base = t_len.min(pp.max_ray_distance_m) / pp.primary_steps.max(1) as f32;

        let mut dist = 0.0f32;
        let mut mean_accum = 0.0f32;
        let mut mean_weight = 0.0f32;
        let max_iter = pp.primary_steps.max(1) * STEP_ITER_SAFETY;

        for _ in 0..max_iter {
            if dist >= total || result.transmittance < TRANSMITTANCE_CUTOFF {
                break;
            }
            let abs_dist = t_start + dist;
            // 步长 cap 由两项取 min:绝对视距比 与 云层厚度渐变。
            let view_cap = abs_dist / STEP_VIEW_DIV;
            let dist_norm = (abs_dist / pp.max_ray_distance_m).clamp(0.0, 1.0);
            let thick_cap = lerp(near_cap, far_cap, dist_norm * dist_norm);
            let step_large = step_base.min(view_cap).min(thick_cap).max(1.0);

            let p = add3(origin, mul3(dir, abs_dist));
            // 快速密度探测(跳过侵蚀采样)。
            let probe = self.evaluate_cloud_properties(p, true);
            if probe.density <= DENSITY_PROBE_EPS {
                dist += step_large;
                continue;
            }
            // 命中云体:补做完整采样并以细步积分。
            let step_small = step_large * 0.25;
            let props = self.evaluate_cloud_properties(p, false);
            if props.density > DENSITY_PROBE_EPS {
                let before = result.transmittance;
                self.evaluate_cloud(props, p, step_small, cos_angle, phase, &mut result);
                let absorbed = before - result.transmittance;
                if absorbed > 0.0 {
                    mean_accum += abs_dist * absorbed;
                    mean_weight += absorbed;
                }
            }
            dist += step_small;
        }

        if mean_weight > 0.0 {
            result.mean_distance = mean_accum / mean_weight;
        }
        result
    }

    /// 双瓣 HG 相位函数逐倍频(前向 + 后向;偏心率按倍频衰减)。
    fn phase_function(&self, cos_angle: f32) -> [f32; MS_OCTAVES] {
        let pp = &self.params;
        let mut out = [0.0f32; MS_OCTAVES];
        for (o, slot) in out.iter_mut().enumerate() {
            let ecc = pp.ms_eccentricity.powi(o as i32);
            *slot = henyey_greenstein(cos_angle, pp.forward_eccentricity * ecc)
                + henyey_greenstein(cos_angle, -pp.backward_eccentricity * ecc);
        }
        out
    }

    /// 视线与云层球壳 slab 的相交区间 `(起点距离, 区间长度)`。
    fn slab_intersection(&self, origin: [f32; 3], dir: [f32; 3]) -> Option<(f32, f32)> {
        let o = sub3(origin, self.planet_center);
        let r_lo = EARTH_RADIUS_M + self.params.lowest_altitude_m;
        let r_hi = EARTH_RADIUS_M + self.params.highest_altitude_m;
        let altitude = length3(o) - EARTH_RADIUS_M;

        // 观察者在云层下方(常态):进入 = 撞内壳近点,离开 = 撞外壳远点。
        if altitude < self.params.lowest_altitude_m {
            // 视线朝下或撞地球 ⇒ 看不到云。
            if ray_sphere_near(o, dir, EARTH_RADIUS_M).is_some_and(|t| t > 0.0) {
                return None;
            }
            let enter = ray_sphere_far(o, dir, r_lo)?;
            let exit = ray_sphere_far(o, dir, r_hi)?;
            if exit <= enter {
                return None;
            }
            return Some((enter, exit - enter));
        }
        // 观察者在云层内:自身出发,离开 = 内壳近点或外壳远点中较近者。
        if altitude <= self.params.highest_altitude_m {
            let exit_hi = ray_sphere_far(o, dir, r_hi).unwrap_or(0.0);
            let exit = match ray_sphere_near(o, dir, r_lo) {
                Some(t) if t > 0.0 => t.min(exit_hi),
                _ => exit_hi,
            };
            if exit <= 0.0 {
                return None;
            }
            return Some((0.0, exit));
        }
        // 观察者在云层上方:进入 = 外壳近点,离开 = 内壳近点(或外壳远点)。
        let enter = ray_sphere_near(o, dir, r_hi).filter(|t| *t > 0.0)?;
        let exit = match ray_sphere_near(o, dir, r_lo) {
            Some(t) if t > enter => t,
            _ => ray_sphere_far(o, dir, r_hi)?,
        };
        if exit <= enter {
            return None;
        }
        Some((enter, exit - enter))
    }

    /// 视线最终颜色:云体积分结果与天空背景按透射率合成。
    pub fn shade(&self, origin: [f32; 3], dir: [f32; 3], max_dist: f32) -> [f32; 3] {
        let ray = self.trace(origin, dir, max_dist);
        let bg = self.sky.radiance_with_sun(dir);
        let mut out = [0.0f32; 3];
        for c in 0..3 {
            out[c] = ray.in_scattering[c] + bg[c] * ray.transmittance;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// device 腿参数打包(与 g40_volumetric_cloud.rx 参数面逐字同源)
// ---------------------------------------------------------------------------

/// device 参数面 f32 数量(kernel 头部 `params` 表逐字对应)。
pub const CLOUD_PARAM_COUNT: usize = 68;

/// 相机(device 参数打包用;右手 Y-up,基向量须正交单位)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudCamera {
    pub origin: [f32; 3],
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    /// 垂直视场半角的正切。
    pub tan_half_fov: f32,
    /// 宽高比(width / height)。
    pub aspect: f32,
}

/// 打包 device 参数面(`CLOUD_PARAM_COUNT` f32)。
///
/// 与 `kernels/g40_volumetric_cloud.rx` 头部注释的下标表**逐字同源**——改一处
/// 必须改两处,顺序即 ABI。
pub fn pack_cloud_params(
    params: &CloudParams,
    sky: &Sky,
    cam: &CloudCamera,
    width: u32,
    height: u32,
    weather: &WeatherMap,
    noise: &NoiseVolumes,
    sky_lut_w: usize,
    sky_lut_h: usize,
) -> Vec<f32> {
    let sun = sky.sun_direction();
    let sun_c = sky.sun_color();
    let (amb_t, amb_b) = sky.ambient_probe();
    let amb_t = mul3(amb_t, params.ambient_top_multiplier);
    let amb_b = mul3(amb_b, params.ambient_bottom_multiplier);
    let mut p = Vec::with_capacity(CLOUD_PARAM_COUNT);
    p.push(width as f32); // 0
    p.push(height as f32); // 1
    p.extend_from_slice(&cam.origin); // 2..5
    p.extend_from_slice(&cam.forward); // 5..8
    p.extend_from_slice(&cam.right); // 8..11
    p.extend_from_slice(&cam.up); // 11..14
    p.push(cam.tan_half_fov); // 14
    p.push(cam.aspect); // 15
    p.extend_from_slice(&sun); // 16..19
    p.extend_from_slice(&sun_c); // 19..22
    p.extend_from_slice(&amb_t); // 22..25
    p.extend_from_slice(&amb_b); // 25..28
    p.push(params.lowest_altitude_m); // 28
    p.push(params.highest_altitude_m); // 29
    p.push(params.max_ray_distance_m); // 30
    p.push(params.primary_steps as f32); // 31
    p.push(params.light_steps as f32); // 32
    p.push(params.noise_scale_m); // 33
    p.push(params.detail_scale_m); // 34
    p.push(params.detail_strength); // 35
    p.push(params.density_threshold); // 36
    p.push(params.density_multiplier); // 37
    p.push(params.coverage_multiplier); // 38
    p.push(params.bottom_smooth_height); // 39
    p.push(params.sigma_t); // 40
    p.push(params.weather_world_size_m); // 41
    p.push(params.wind_offset_m[0]); // 42
    p.push(params.wind_offset_m[1]); // 43
    p.push(params.forward_eccentricity); // 44
    p.push(params.backward_eccentricity); // 45
    p.push(params.ms_attenuation); // 46
    p.push(params.ms_contribution); // 47
    p.push(params.ms_eccentricity); // 48
    p.push(params.ao_upward_scale); // 49
    p.push(params.powder_intensity); // 50
    p.push(params.scatter_source_od_scale); // 51
    p.push(params.scatter_source_curve_pow); // 52
    p.push(params.phi_fwd_intensity); // 53
    p.push(params.phi_fwd_depth_pow); // 54
    p.push(params.phi_fwd_depth_bias); // 55
    p.push(params.phi_fwd_boundary_confidence); // 56
    p.push(params.phi_fwd_ms_build_scale); // 57
    p.push(params.phi_fwd_compress); // 58
    p.push(weather.width as f32); // 59
    p.push(weather.height as f32); // 60
    p.push(noise.base_dim as f32); // 61
    p.push(noise.detail_dim as f32); // 62
    p.push(sky_lut_w as f32); // 63
    p.push(sky_lut_h as f32); // 64
    p.push(EARTH_RADIUS_M); // 65
    p.push((params.primary_steps.max(1) * STEP_ITER_SAFETY) as f32); // 66 max_iter
    // 67:锥形采样的 r^N(r = 2);host 预算免 kernel 内做整数幂。
    p.push((2.0f32).powi(params.light_steps.max(1) as i32));
    debug_assert_eq!(p.len(), CLOUD_PARAM_COUNT);
    p
}

/// weather map 打平为 device SSBO 字节序(3 f32/texel,行主序)。
pub fn pack_weather(weather: &WeatherMap) -> Vec<f32> {
    let mut out = Vec::with_capacity(weather.pixels.len() * 3);
    for p in &weather.pixels {
        out.extend_from_slice(p);
    }
    out
}

// ---------------------------------------------------------------------------
// Froxel 云前端(兑现 M112「两个前端」契约)
// ---------------------------------------------------------------------------

/// 云前端:与 [`super::atmosphere::FogFrontend`] 同签名写同一
/// [`FroxelVolume`] 密度场——**云与雾共用同一 Froxel 基础设施、两个前端**
/// (RXS-0365 L1 断言;各自独立体渲染器即 RED)。
///
/// 与雾前端的解析高度衰减不同,云前端按 froxel 世界位置采样体积云密度场。
pub struct CloudFrontend<'a> {
    /// 密度求值器(与视线积分共用同一密度模型,禁第二套密度)。
    pub renderer: &'a CloudRenderer<'a>,
    /// froxel 网格覆盖的世界范围(XZ 边长 m)。
    pub world_extent_m: f32,
    /// froxel 网格 z 向切片间距(m)。
    pub slice_spacing_m: f32,
    /// froxel 网格原点(世界系)。
    pub origin_m: [f32; 3],
}

impl CloudFrontend<'_> {
    /// 写 Froxel 密度场(与 `FogFrontend::write_density` 同签名同错误面)。
    pub fn write_density(&self, volume: &mut FroxelVolume) -> Result<(), AtmosphereError> {
        let p = self.renderer.params();
        if !p.density_multiplier.is_finite() || p.density_multiplier < 0.0 {
            return Err(AtmosphereError::NonFiniteValue {
                stage: "cloud_density_multiplier",
            });
        }
        if !self.world_extent_m.is_finite() || self.world_extent_m <= 0.0 {
            return Err(AtmosphereError::NonFiniteValue {
                stage: "cloud_world_extent",
            });
        }
        if !self.slice_spacing_m.is_finite() || self.slice_spacing_m <= 0.0 {
            return Err(AtmosphereError::NonFiniteValue {
                stage: "cloud_slice_spacing",
            });
        }
        let dim = volume.dim;
        let slices = volume.depth_slices;
        let sx = self.world_extent_m / dim[0].max(1) as f32;
        let sz = self.world_extent_m / dim[1].max(1) as f32;
        for z in 0..slices {
            let world_y = self.origin_m[1] + z as f32 * self.slice_spacing_m;
            for y in 0..dim[1] {
                let world_z = self.origin_m[2] + (y as f32 - dim[1] as f32 * 0.5) * sz;
                for x in 0..dim[0] {
                    let world_x = self.origin_m[0] + (x as f32 - dim[0] as f32 * 0.5) * sx;
                    let props = self
                        .renderer
                        .evaluate_cloud_properties([world_x, world_y, world_z], true);
                    let idx = (z * dim[1] * dim[0] + y * dim[0] + x) as usize;
                    volume.density[idx] = props.density * props.sigma_t;
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 相位函数与 powder(与 .rx 逐字同源)
// ---------------------------------------------------------------------------

/// Henyey-Greenstein 相位函数(4π 上归一化)。
pub fn henyey_greenstein(cos_angle: f32, g: f32) -> f32 {
    let g2 = g * g;
    let den = (1.0 + g2 - 2.0 * g * cos_angle).max(0.0);
    (1.0 / (4.0 * std::f32::consts::PI)) * (1.0 - g2) / den.powf(1.5).max(1e-6)
}

/// powder effect(背光方向的云边暗化;参考实现 `PowderEffect`)。
pub fn powder_effect(density: f32, cos_angle: f32, intensity: f32) -> f32 {
    let p = (1.0 - (-density * 4.0).exp()) * 2.0;
    let p = p.clamp(0.0, 1.0);
    lerp(
        1.0,
        lerp(1.0, p, smoothstep_desc(0.5, -0.5, cos_angle)),
        intensity,
    )
}

/// 密度重映射(参考实现 `DensityRemap`;区间线性变换后钳制)。
pub fn remap(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    let t = (x - a) / (b - a).max(1e-6);
    (t * (d - c) + c).clamp(c.min(d), c.max(d))
}

// ---------------------------------------------------------------------------
// 程序化 3D 噪声(确定性 hash;零贴图资产)
// ---------------------------------------------------------------------------

/// 整数格点 hash → `[0,1)` 伪随机标量(确定性,跨平台位级一致)。
fn hash31(ix: i32, iy: i32, iz: i32) -> f32 {
    // Wang hash 变体;全 u32 wrapping,无浮点参与 ⇒ 位级可复现。
    let mut h = (ix as u32).wrapping_mul(0x8da6_b343)
        ^ (iy as u32).wrapping_mul(0xd816_3841_u32)
        ^ (iz as u32).wrapping_mul(0xcb1a_b31f);
    h = h.wrapping_add(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    (h >> 8) as f32 / 16_777_216.0
}

/// 三维 value noise(三线性插值 + 五次平滑核)。
fn value_noise(p: [f32; 3]) -> f32 {
    let fx = p[0].floor();
    let fy = p[1].floor();
    let fz = p[2].floor();
    let ix = fx as i32;
    let iy = fy as i32;
    let iz = fz as i32;
    let tx = quintic(p[0] - fx);
    let ty = quintic(p[1] - fy);
    let tz = quintic(p[2] - fz);
    let c = |dx: i32, dy: i32, dz: i32| hash31(ix + dx, iy + dy, iz + dz);
    let x00 = lerp(c(0, 0, 0), c(1, 0, 0), tx);
    let x10 = lerp(c(0, 1, 0), c(1, 1, 0), tx);
    let x01 = lerp(c(0, 0, 1), c(1, 0, 1), tx);
    let x11 = lerp(c(0, 1, 1), c(1, 1, 1), tx);
    lerp(lerp(x00, x10, ty), lerp(x01, x11, ty), tz)
}

/// 三维 Worley(cellular)noise:返回到最近特征点的距离,归一化到 `[0,1]`。
fn worley(p: [f32; 3]) -> f32 {
    let fx = p[0].floor();
    let fy = p[1].floor();
    let fz = p[2].floor();
    let ix = fx as i32;
    let iy = fy as i32;
    let iz = fz as i32;
    let mut min_d2 = f32::INFINITY;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cx = ix + dx;
                let cy = iy + dy;
                let cz = iz + dz;
                // 每格一个特征点,三次独立 hash 给出格内偏移。
                let ox = hash31(cx, cy, cz);
                let oy = hash31(cx.wrapping_add(7717), cy, cz);
                let oz = hash31(cx, cy.wrapping_add(31337), cz);
                let px = cx as f32 + ox - p[0];
                let py = cy as f32 + oy - p[1];
                let pz = cz as f32 + oz - p[2];
                let d2 = px * px + py * py + pz * pz;
                if d2 < min_d2 {
                    min_d2 = d2;
                }
            }
        }
    }
    // 单格内最大可能距离约 sqrt(3);归一化并钳制。
    (min_d2.sqrt() / 1.732_050_8).clamp(0.0, 1.0)
}

/// Perlin-Worley 基础塑形噪声(FBM value noise 与反相 Worley 的 remap 组合)。
///
/// Schneider 范式:低频 FBM 给出团块轮廓,反相 Worley 给出蓬松的「云絮」内核,
/// 二者 remap 组合得到既有大形又有内部结构的基础密度。
///
/// **只在烘焙 [`NoiseVolumes`] 时调用**——逐样点现算需约 105 次 hash,实时路径
/// 一律走烘焙体的三线性取样。
fn perlin_worley(p: [f32; 3]) -> f32 {
    // 三倍频 FBM。
    let fbm =
        value_noise(p) * 0.5 + value_noise(mul3(p, 2.03)) * 0.3 + value_noise(mul3(p, 4.01)) * 0.2;
    // 反相 Worley(1 − 距离:格心处为 1)。
    let w = 1.0 - worley(mul3(p, 1.7));
    remap(fbm, w - 1.0, 1.0, 0.0, 1.0)
}

/// Worley FBM 侵蚀噪声(三倍频;高频细节)。烘焙期专用,同 [`perlin_worley`]。
fn worley_fbm(p: [f32; 3]) -> f32 {
    let w0 = 1.0 - worley(p);
    let w1 = 1.0 - worley(mul3(p, 2.11));
    let w2 = 1.0 - worley(mul3(p, 4.07));
    (w0 * 0.625 + w1 * 0.25 + w2 * 0.125).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// 烘焙噪声体(host/device 共享事实源)
// ---------------------------------------------------------------------------

/// 基础塑形噪声体边长(128³ = 2M f32 = 8MB;Schneider 范式的低频体)。
pub const NOISE_BASE_DIM: usize = 128;
/// 侵蚀噪声体边长(32³ = 32K f32 = 128KB;高频细节体)。
pub const NOISE_DETAIL_DIM: usize = 32;

/// 预烘焙 3D 噪声体(**周期性平铺**:烘焙域恰为一个世界周期,取样时对边长取模)。
///
/// 逐样点现算 Perlin-Worley 需约 350 次 hash,实时路径不可承受;烘焙一次后取样
/// 只需 8 次读 + 三线性插值。host 金标准与 `g40_volumetric_cloud.rx` device 腿
/// **共享同一份烘焙数据**(host 建造后原字节上传 SSBO)——噪声不再是两边各自
/// 现算的近似,而是同一事实源,对拍容差里不含噪声项。
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseVolumes {
    /// 基础塑形体(`base_dim³`,行主序 `[z][y][x]`)。
    pub base: Vec<f32>,
    /// 基础体边长。
    pub base_dim: usize,
    /// 侵蚀细节体(`detail_dim³`,同序)。
    pub detail: Vec<f32>,
    /// 侵蚀体边长。
    pub detail_dim: usize,
}

/// 基础体烘焙周期(噪声函数定义域跨度;整数保证跨边界无缝平铺)。
const NOISE_BASE_PERIOD: f32 = 8.0;
/// 侵蚀体烘焙周期。
const NOISE_DETAIL_PERIOD: f32 = 4.0;

impl NoiseVolumes {
    /// 生产档烘焙(128³ 基础 + 32³ 侵蚀;确定性,同调用双跑位级相等)。
    ///
    /// 这是本模块最重的一次性开销(约 2.2 亿次 hash),**禁逐帧重烘**——建造一次
    /// 后跨帧、跨预设、跨 host/device 复用。
    pub fn bake() -> Self {
        Self::bake_dims(NOISE_BASE_DIM, NOISE_DETAIL_DIM)
    }

    /// 指定边长烘焙(测试/低配档用;边长须 ≥ 2)。
    pub fn bake_dims(base_dim: usize, detail_dim: usize) -> Self {
        let base_dim = base_dim.max(2);
        let detail_dim = detail_dim.max(2);
        let mut base = vec![0.0f32; base_dim * base_dim * base_dim];
        for z in 0..base_dim {
            for y in 0..base_dim {
                for x in 0..base_dim {
                    let s = NOISE_BASE_PERIOD / base_dim as f32;
                    let p = [x as f32 * s, y as f32 * s, z as f32 * s];
                    base[(z * base_dim + y) * base_dim + x] = perlin_worley(p);
                }
            }
        }
        let mut detail = vec![0.0f32; detail_dim * detail_dim * detail_dim];
        for z in 0..detail_dim {
            for y in 0..detail_dim {
                for x in 0..detail_dim {
                    let s = NOISE_DETAIL_PERIOD / detail_dim as f32;
                    let p = [x as f32 * s, y as f32 * s, z as f32 * s];
                    detail[(z * detail_dim + y) * detail_dim + x] = worley_fbm(p);
                }
            }
        }
        Self {
            base,
            base_dim,
            detail,
            detail_dim,
        }
    }

    /// 基础体三线性平铺取样(`uvw` 单位为「周期数」,自动 wrap)。
    pub fn sample_base(&self, uvw: [f32; 3]) -> f32 {
        sample_tiled(&self.base, self.base_dim, uvw)
    }

    /// 侵蚀体三线性平铺取样。
    pub fn sample_detail(&self, uvw: [f32; 3]) -> f32 {
        sample_tiled(&self.detail, self.detail_dim, uvw)
    }
}

/// 立方体积的三线性环绕取样(与 `.rx` 侧逐字同式;`uvw` 为周期归一化坐标)。
fn sample_tiled(vol: &[f32], dim: usize, uvw: [f32; 3]) -> f32 {
    let n = dim as f32;
    let fx = uvw[0] * n;
    let fy = uvw[1] * n;
    let fz = uvw[2] * n;
    let x0 = fx.floor();
    let y0 = fy.floor();
    let z0 = fz.floor();
    let tx = fx - x0;
    let ty = fy - y0;
    let tz = fz - z0;
    // 环绕索引(负坐标亦正确;i64 取模后修正符号)。
    let w = |v: f32| -> usize {
        let d = dim as i64;
        (((v as i64 % d) + d) % d) as usize
    };
    let ix0 = w(x0);
    let ix1 = w(x0 + 1.0);
    let iy0 = w(y0);
    let iy1 = w(y0 + 1.0);
    let iz0 = w(z0);
    let iz1 = w(z0 + 1.0);
    let at = |x: usize, y: usize, z: usize| vol[(z * dim + y) * dim + x];
    let c00 = lerp(at(ix0, iy0, iz0), at(ix1, iy0, iz0), tx);
    let c10 = lerp(at(ix0, iy1, iz0), at(ix1, iy1, iz0), tx);
    let c01 = lerp(at(ix0, iy0, iz1), at(ix1, iy0, iz1), tx);
    let c11 = lerp(at(ix0, iy1, iz1), at(ix1, iy1, iz1), tx);
    lerp(lerp(c00, c10, ty), lerp(c01, c11, ty), tz)
}

// ---------------------------------------------------------------------------
// 标量/向量小工具
// ---------------------------------------------------------------------------

fn quintic(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn fract(x: f32) -> f32 {
    x - x.floor()
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 降序 smoothstep(`e0 > e1` 的 HLSL 语义;`x ≤ e1` 时为 1)。
fn smoothstep_desc(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mul3(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn length3(a: [f32; 3]) -> f32 {
    dot3(a, a).sqrt()
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    length3(sub3(a, b))
}

fn normalize3(a: [f32; 3]) -> [f32; 3] {
    let l = length3(a);
    if l > 1e-12 {
        mul3(a, 1.0 / l)
    } else {
        [0.0, 1.0, 0.0]
    }
}

fn ray_sphere_far(origin: [f32; 3], dir: [f32; 3], r: f32) -> Option<f32> {
    let b = dot3(origin, dir);
    let c = dot3(origin, origin) - r * r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b + disc.sqrt();
    if t < 0.0 { None } else { Some(t) }
}

fn ray_sphere_near(origin: [f32; 3], dir: [f32; 3], r: f32) -> Option<f32> {
    let b = dot3(origin, dir);
    let c = dot3(origin, origin) - r * r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t < 0.0 { None } else { Some(t) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::sky::{PRESET_CLEAR, PRESET_GOLDEN, direction_from_angles};

    /// 测试档烘焙噪声体(小边长;全测试共享一次烘焙,生产档 128³ 太慢不入单测)。
    fn test_noise() -> &'static NoiseVolumes {
        static NOISE: std::sync::OnceLock<NoiseVolumes> = std::sync::OnceLock::new();
        NOISE.get_or_init(|| NoiseVolumes::bake_dims(32, 16))
    }

    /// 全覆盖 weather map(coverage=cov,湿度 0.5,指定云型通道)。
    fn uniform_weather(cov: f32, type_channel: f32) -> WeatherMap {
        WeatherMap {
            width: 4,
            height: 4,
            pixels: vec![[cov, 0.5, type_channel]; 16],
        }
    }

    /// 观察者:地表 500m,看向太阳所在半天(能吃到 slab)。
    fn observer() -> [f32; 3] {
        [0.0, 500.0, 0.0]
    }

    /// 云型廓形:slab 上下边界处恒零,三型峰值位置递增可区分。
    #[test]
    fn cloud_type_profiles_distinct() {
        for t in [
            CloudType::Cumulus,
            CloudType::TowerCumulus,
            CloudType::Cumulonimbus,
        ] {
            assert_eq!(t.profile(0.0), 0.0, "{t:?} 云底廓形为零");
            assert!(t.profile(1.001) <= 1e-6, "{t:?} slab 顶外廓形为零");
            assert!(t.density_multiplier() > 0.0);
        }
        // 纵向发展高度递增:积云最扁(只占 slab 下部),积雨云占满全高。
        // 判据取「廓形仍 ≥ 0.5 的最高处」——三型的塑形差异在云顶而非峰值位置
        // (廓形中段是平台,峰值不唯一)。
        let top_extent = |t: CloudType| {
            (0..=1000)
                .rev()
                .map(|i| i as f32 / 1000.0)
                .find(|&h| t.profile(h) >= 0.5)
                .unwrap_or(0.0)
        };
        let cu = top_extent(CloudType::Cumulus);
        let tcu = top_extent(CloudType::TowerCumulus);
        let cb = top_extent(CloudType::Cumulonimbus);
        assert!(cu < tcu && tcu < cb, "云顶高度应递增: {cu} < {tcu} < {cb}");
        assert!(cu < 0.5, "积云应集中在 slab 下半: {cu}");
        assert!(cb > 0.85, "积雨云应几乎占满 slab: {cb}");
        // 通道选型闭集。
        assert_eq!(CloudType::from_channel(0.0), CloudType::Cumulus);
        assert_eq!(CloudType::from_channel(0.5), CloudType::TowerCumulus);
        assert_eq!(CloudType::from_channel(1.0), CloudType::Cumulonimbus);
    }

    /// 密度场:slab 上下界外恒零;覆盖度提升 ⇒ 平均密度单调不降。
    #[test]
    fn density_bounded_by_slab_and_monotone_in_coverage() {
        let sky = Sky::new(PRESET_CLEAR);
        let p = CloudParams::default();

        // slab 外恒零(下方 / 上方各取若干高度)。
        let w = uniform_weather(1.0, 0.5);
        let r = CloudRenderer::new(p, &sky, &w, test_noise());
        for y in [0.0f32, 500.0, 1_400.0, 4_100.0, 8_000.0] {
            let props = r.evaluate_cloud_properties([120.0, y, -340.0], false);
            assert_eq!(props.density, 0.0, "高度 {y} 在 slab 外应无密度");
        }

        // 覆盖度单调性(slab 中部网格平均密度)。
        let mid_y = (p.lowest_altitude_m + p.highest_altitude_m) * 0.5;
        let mean_density = |cov: f32| -> f32 {
            let w = uniform_weather(cov, 0.5);
            let r = CloudRenderer::new(p, &sky, &w, test_noise());
            let mut sum = 0.0;
            let mut n = 0;
            for gx in 0..12 {
                for gz in 0..12 {
                    let x = (gx as f32 - 6.0) * 900.0;
                    let z = (gz as f32 - 6.0) * 900.0;
                    sum += r.evaluate_cloud_properties([x, mid_y, z], false).density;
                    n += 1;
                }
            }
            sum / n as f32
        };
        let mut prev = -1.0f32;
        for cov in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let d = mean_density(cov);
            assert!(d.is_finite() && d >= 0.0, "coverage {cov} 密度有限非负");
            assert!(
                d >= prev - 1e-6,
                "coverage {cov} 平均密度 {d} 应不低于上一档 {prev}"
            );
            prev = d;
        }
        assert!(prev > 0.0, "满覆盖应有非零密度");
        assert_eq!(mean_density(0.0), 0.0, "零覆盖应无密度");
    }

    /// HG 相位函数在 4π 上归一化(球面数值积分,容差 1%)。
    #[test]
    fn henyey_greenstein_normalized() {
        for g in [0.0f32, 0.3, 0.6, 0.85, -0.3] {
            const N: u32 = 4096;
            let mut sum = 0.0f64;
            for i in 0..N {
                let c = -1.0 + 2.0 * (i as f32 + 0.5) / N as f32;
                sum += f64::from(henyey_greenstein(c, g))
                    * 2.0
                    * std::f64::consts::PI
                    * (2.0 / f64::from(N));
            }
            assert!((sum - 1.0).abs() < 0.01, "g={g} 归一化: {sum}");
        }
    }

    /// `remap` / `powder_effect` 的边界与值域。
    #[test]
    fn remap_and_powder_bounded() {
        assert!((remap(0.5, 0.0, 1.0, 0.0, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(remap(-1.0, 0.0, 1.0, 0.0, 1.0), 0.0, "低于下界钳到 c");
        assert_eq!(remap(2.0, 0.0, 1.0, 0.0, 1.0), 1.0, "高于上界钳到 d");
        // 退化区间不产生 NaN/Inf。
        assert!(remap(0.5, 1.0, 1.0, 0.0, 1.0).is_finite());
        for d in [0.0f32, 0.2, 1.0] {
            for c in [-1.0f32, 0.0, 1.0] {
                let p = powder_effect(d, c, 0.6);
                assert!(
                    p.is_finite() && (0.0..=1.0).contains(&p),
                    "powder({d},{c})={p}"
                );
            }
        }
        // 强度 0 ⇒ 恒等 1(关臂零影响)。
        assert_eq!(powder_effect(0.5, -1.0, 0.0), 1.0);
    }

    /// 噪声:值域受界、确定性、非常数(否则密度场退化为平板)。
    #[test]
    fn noise_bounded_deterministic_and_varying() {
        let mut min_v = f32::INFINITY;
        let mut max_v = f32::NEG_INFINITY;
        for i in 0..500 {
            let p = [i as f32 * 0.37, i as f32 * 0.11 - 3.0, i as f32 * 0.73];
            for v in [value_noise(p), worley(p), perlin_worley(p), worley_fbm(p)] {
                assert!(
                    v.is_finite() && (0.0..=1.0).contains(&v),
                    "噪声 {v} 越界 @ {p:?}"
                );
            }
            let pw = perlin_worley(p);
            min_v = min_v.min(pw);
            max_v = max_v.max(pw);
            // 确定性:同输入双跑位级。
            assert_eq!(perlin_worley(p).to_bits(), perlin_worley(p).to_bits());
        }
        assert!(
            max_v - min_v > 0.2,
            "perlin_worley 动态范围过窄: {min_v}..{max_v}"
        );
    }

    /// 烘焙噪声体:值域受界、跨周期边界无缝平铺、双跑位级、非常数。
    #[test]
    fn noise_volumes_tile_seamlessly() {
        let n = test_noise();
        assert_eq!(n.base.len(), n.base_dim.pow(3));
        assert_eq!(n.detail.len(), n.detail_dim.pow(3));
        assert!(
            n.base
                .iter()
                .chain(n.detail.iter())
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
            "烘焙值域受界"
        );

        // 无缝平铺:坐标平移整数个周期,取样值相等(含负坐标)。
        //
        // 这里只能要求**容差相等**而非位级:平移后 `fx − floor(fx)` 的尾数不同
        // (如 0.3×32 = 9.6 与 1.3×32 = 41.6,取整后小数位分别是 0.5999999 与
        // 0.5999985),插值权重差 1 ULP。命中的纹素完全相同,平铺本身无缝。
        for i in 0..24 {
            let p = [i as f32 * 0.137, i as f32 * 0.061 + 0.3, i as f32 * 0.211];
            for shift in [1.0f32, -1.0, 3.0, -5.0] {
                let q = [p[0] + shift, p[1] - shift, p[2] + shift * 2.0];
                assert!(
                    (n.sample_base(p) - n.sample_base(q)).abs() < 1e-5,
                    "基础体平移 {shift} 周期后应等值 @ {p:?}"
                );
                assert!(
                    (n.sample_detail(p) - n.sample_detail(q)).abs() < 1e-5,
                    "侵蚀体平移 {shift} 周期后应等值 @ {p:?}"
                );
            }
        }

        // 同坐标位级确定(host/device 对拍的前提 —— 两侧同一 IEEE 运算序)。
        for i in 0..32 {
            let p = [
                i as f32 * 0.091 - 1.5,
                i as f32 * 0.033,
                i as f32 * 0.157 + 2.0,
            ];
            assert_eq!(n.sample_base(p).to_bits(), n.sample_base(p).to_bits());
            assert_eq!(n.sample_detail(p).to_bits(), n.sample_detail(p).to_bits());
        }

        // 动态范围:烘焙体不得退化为常数(否则密度场是平板)。
        let samples: Vec<f32> = (0..200)
            .map(|i| n.sample_base([i as f32 * 0.023, i as f32 * 0.041, i as f32 * 0.017]))
            .collect();
        let lo = samples.iter().copied().fold(f32::INFINITY, f32::min);
        let hi = samples.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(hi - lo > 0.2, "基础体动态范围过窄: {lo}..{hi}");

        // 双跑位级(烘焙确定性 —— device 上传同一份字节的前提)。
        let again = NoiseVolumes::bake_dims(n.base_dim, n.detail_dim);
        assert_eq!(&again, n, "同边长双跑烘焙应位级相等");
    }

    /// phi_fwd 关臂 = `intensity 0`:与 `without_phi_fwd()` 构造的臂逐位相等,
    /// 且开臂必须严格更亮(加性项不得为零贡献 —— 防「接了等于没接」)。
    #[test]
    fn phi_fwd_off_arm_bit_exact_and_on_arm_brighter() {
        let sky = Sky::new(PRESET_GOLDEN);
        let w = uniform_weather(0.85, 0.6);
        let on = CloudParams::default();
        let off = on.without_phi_fwd();
        // 关臂只动 intensity 单字段。
        assert_eq!(off.phi_fwd_intensity, 0.0);
        assert_eq!(off.density_multiplier, on.density_multiplier);
        assert_eq!(off.sigma_t, on.sigma_t);

        let r_on = CloudRenderer::new(on, &sky, &w, test_noise());
        let r_off = CloudRenderer::new(off, &sky, &w, test_noise());
        let r_off2 = CloudRenderer::new(
            CloudParams {
                phi_fwd_intensity: 0.0,
                ..on
            },
            &sky,
            &w,
            test_noise(),
        );

        let origin = observer();
        let mut brighter = 0u32;
        let mut compared = 0u32;
        for i in 0..40 {
            let dir = direction_from_angles(4.0 + i as f32 * 0.9, 250.0 + i as f32 * 2.5);
            let a = r_off.trace(origin, dir, f32::INFINITY);
            let b = r_off2.trace(origin, dir, f32::INFINITY);
            // 两条关臂路径逐位相等(关臂语义唯一)。
            assert_eq!(
                a.in_scattering.map(f32::to_bits),
                b.in_scattering.map(f32::to_bits),
                "关臂双路径应位级相等"
            );
            let c = r_on.trace(origin, dir, f32::INFINITY);
            assert!(c.in_scattering.iter().all(|v| v.is_finite() && *v >= 0.0));
            let s_off: f32 = a.in_scattering.iter().sum();
            let s_on: f32 = c.in_scattering.iter().sum();
            if s_off > 1e-6 {
                compared += 1;
                assert!(s_on >= s_off - 1e-6, "phi_fwd 为加性项,不得使画面变暗");
                // 5% 阈值锁定 DEFAULT_PHI_FWD_INTENSITY 的标定:强度掉回参考实现
                // 的 0.1~2.0 区间时本判据即红(那个量纲下 phi_fwd 观感等同关臂)。
                if s_on > s_off * 1.05 {
                    brighter += 1;
                }
            }
        }
        assert!(compared > 0, "测试方向集须命中云体");
        assert!(
            brighter * 2 >= compared,
            "phi_fwd 开臂应在多数命中方向显著提亮(>5%): {brighter}/{compared}"
        );
    }

    /// phi_fwd 的物理特征:随光学深度增大,漫射场趋于**饱和**而非指数衰减。
    ///
    /// 取云内一列样点,沿太阳反方向逐步深入(光学深度递增),比较 phi_fwd 与
    /// 纯 Beer-Lambert 单散射 `exp(−OD)` 的衰减速度——扩散场必须衰减得更慢。
    #[test]
    fn phi_fwd_saturates_with_optical_depth() {
        let sky = Sky::new(PRESET_CLEAR);
        let w = uniform_weather(1.0, 0.9); // 积雨云,厚
        let p = CloudParams {
            density_multiplier: 2.5,
            sigma_t: 0.15,
            light_steps: 8,
            ..CloudParams::default()
        };
        let r = CloudRenderer::new(p, &sky, &w, test_noise());
        let phase = r.phase_function(0.0);

        // 自云顶沿 −sun 方向深入,记录 (光学深度, phi_fwd)。
        let mut samples: Vec<(f32, f32)> = Vec::new();
        let top = [0.0f32, p.highest_altitude_m - 50.0, 0.0];
        for k in 0..8 {
            let depth = k as f32 * 260.0;
            let pos = sub3(top, mul3(r.sun_dir, -depth));
            let pos = [
                pos[0],
                (top[1] - depth).max(p.lowest_altitude_m + 20.0),
                pos[2],
            ];
            let props = r.evaluate_cloud_properties(pos, false);
            if props.density <= 0.0 {
                continue;
            }
            let (_, od, phi) = r.evaluate_sun_luminance(pos, props.local_height, phase);
            assert!(phi.is_finite() && phi >= 0.0, "phi_fwd 有限非负");
            if od > 0.05 {
                samples.push((od, phi));
            }
        }
        assert!(
            samples.len() >= 3,
            "样点不足以判定衰减趋势: {}",
            samples.len()
        );

        // 判据:phi_fwd 相对最浅样点的比值,始终高于同 OD 下的 Beer-Lambert 比值。
        let (od0, phi0) = samples[0];
        for &(od, phi) in &samples[1..] {
            if od <= od0 {
                continue;
            }
            let beer = (-(od - od0)).exp();
            let diffusion = phi / phi0.max(1e-12);
            assert!(
                diffusion > beer,
                "OD {od0}→{od}: 漫射场比值 {diffusion} 应显著慢于 Beer-Lambert {beer}"
            );
        }
    }

    /// phi_fwd 边界受光置信度:背光边界的源被抑制,受光边界不被抑制。
    #[test]
    fn phi_fwd_boundary_confidence_suppresses_backlit() {
        let sky = Sky::new(PRESET_GOLDEN);
        // 覆盖度沿 X 有梯度 ⇒ 高度代理差分产生非平凡法线。
        let weather = WeatherMap {
            width: 8,
            height: 8,
            pixels: (0..64)
                .map(|i| {
                    let x = (i % 8) as f32 / 7.0;
                    [x, 0.5, 0.6]
                })
                .collect(),
        };
        let on = CloudParams::default();
        let off = CloudParams {
            phi_fwd_boundary_confidence: 0.0,
            ..on
        };
        let r_on = CloudRenderer::new(on, &sky, &weather, test_noise());
        let r_off = CloudRenderer::new(off, &sky, &weather, test_noise());

        let mid_y = (on.lowest_altitude_m + on.highest_altitude_m) * 0.5;
        let mut any_suppressed = false;
        for gx in 0..16 {
            let pos = [(gx as f32 - 8.0) * 2_000.0, mid_y, 0.0];
            let c_on = r_on.phi_fwd_boundary_light(pos);
            let c_off = r_off.phi_fwd_boundary_light(pos);
            assert!((0.0..=1.0).contains(&c_on), "置信度受界: {c_on}");
            assert_eq!(c_off, 1.0, "置信度强度 0 ⇒ 恒 1(禁用语义)");
            assert!(c_on <= c_off + 1e-6, "开启置信度只会抑制不会放大");
            if c_on < 0.999 {
                any_suppressed = true;
            }
        }
        assert!(any_suppressed, "梯度覆盖场应至少在部分位置产生背光抑制");
    }

    /// 视线积分:透射率受界单调、无命中时保持全透、命中后必然衰减。
    #[test]
    fn trace_transmittance_bounded() {
        let sky = Sky::new(PRESET_CLEAR);
        let w = uniform_weather(0.9, 0.7);
        let r = CloudRenderer::new(CloudParams::default(), &sky, &w, test_noise());
        let origin = observer();

        // 朝下 ⇒ 不与 slab 相交 ⇒ 无效射线、全透。
        let down = r.trace(origin, [0.0, -1.0, 0.0], f32::INFINITY);
        assert!(down.invalid_ray, "朝下射线不应命中云层");
        assert_eq!(down.transmittance, 1.0);
        assert_eq!(down.in_scattering, [0.0; 3]);

        // 天顶 ⇒ 必然穿过 slab。
        let up = r.trace(origin, [0.0, 1.0, 0.0], f32::INFINITY);
        assert!(!up.invalid_ray, "天顶射线应命中 slab");
        assert!((0.0..=1.0).contains(&up.transmittance), "透射率受界");
        assert!(up.in_scattering.iter().all(|v| v.is_finite() && *v >= 0.0));

        // 全方向有限性 + 命中时 mean_distance 落在合理量级。
        for i in 0..30 {
            let dir = direction_from_angles(2.0 + i as f32 * 2.8, i as f32 * 12.0);
            let res = r.trace(origin, dir, f32::INFINITY);
            assert!(
                (0.0..=1.0).contains(&res.transmittance),
                "透射率 {}",
                res.transmittance
            );
            assert!(res.in_scattering.iter().all(|v| v.is_finite()));
            if res.transmittance < 0.999 {
                assert!(
                    res.mean_distance.is_finite() && res.mean_distance > 0.0,
                    "命中射线须有有限平均距离"
                );
            }
        }
    }

    /// 不透明几何深度裁剪:`max_dist` 缩短 ⇒ 云的遮挡只会减少不会增加。
    #[test]
    fn opaque_depth_clips_clouds() {
        let sky = Sky::new(PRESET_CLEAR);
        let w = uniform_weather(0.95, 0.8);
        let r = CloudRenderer::new(CloudParams::default(), &sky, &w, test_noise());
        let origin = observer();
        let dir = direction_from_angles(35.0, 340.0);
        let full = r.trace(origin, dir, f32::INFINITY);
        let mut prev_t = full.transmittance;
        for d in [40_000.0f32, 20_000.0, 8_000.0, 3_000.0, 500.0] {
            let clipped = r.trace(origin, dir, d);
            assert!(
                clipped.transmittance >= prev_t - 1e-5,
                "深度 {d} 裁剪后透射率 {} 不应低于更远档 {prev_t}",
                clipped.transmittance
            );
            prev_t = clipped.transmittance;
        }
        assert_eq!(prev_t, 1.0, "极近深度应完全裁掉云层");
    }

    /// 合成:云透射率为 1 时画面 == 纯天空背景(合成式无偏移)。
    #[test]
    fn shade_matches_sky_when_fully_transmissive() {
        let sky = Sky::new(PRESET_CLEAR);
        // 零覆盖 ⇒ 无云。
        let w = uniform_weather(0.0, 0.5);
        let r = CloudRenderer::new(CloudParams::default(), &sky, &w, test_noise());
        let origin = observer();
        for i in 0..12 {
            let dir = direction_from_angles(10.0 + i as f32 * 6.0, i as f32 * 30.0);
            let shaded = r.shade(origin, dir, f32::INFINITY);
            let bg = sky.radiance_with_sun(dir);
            assert_eq!(
                shaded.map(f32::to_bits),
                bg.map(f32::to_bits),
                "无云时合成结果应与天空背景位级相等"
            );
        }
    }

    /// 确定性:同参数双跑逐位相等(展示图 digest 可复现的前提)。
    #[test]
    fn deterministic_double_run() {
        let sky = Sky::new(PRESET_GOLDEN);
        let w = uniform_weather(0.8, 0.55);
        let p = CloudParams::default();
        let origin = observer();
        let a = CloudRenderer::new(p, &sky, &w, test_noise());
        let b = CloudRenderer::new(p, &sky, &w, test_noise());
        for i in 0..48 {
            let dir = direction_from_angles(3.0 + i as f32 * 1.7, 240.0 + i as f32 * 3.3);
            let ra = a.shade(origin, dir, f32::INFINITY);
            let rb = b.shade(origin, dir, f32::INFINITY);
            assert_eq!(
                ra.map(f32::to_bits),
                rb.map(f32::to_bits),
                "方向 {i} 双跑位级"
            );
        }
    }

    /// M112 契约兑现:云前端写同一 [`FroxelVolume`],与雾前端同签名同错误面;
    /// 密度来自同一密度模型(禁第二套密度)。
    #[test]
    fn cloud_frontend_writes_shared_froxel_volume() {
        use crate::world::atmosphere::{FROXEL_DEPTH_SLICES, FogFrontend};

        let sky = Sky::new(PRESET_CLEAR);
        let w = uniform_weather(1.0, 0.8);
        let params = CloudParams::default();
        let renderer = CloudRenderer::new(params, &sky, &w, test_noise());
        let frontend = CloudFrontend {
            renderer: &renderer,
            world_extent_m: 20_000.0,
            slice_spacing_m: params.slab_thickness_m()
                / f32::from(u16::try_from(FROXEL_DEPTH_SLICES).unwrap()),
            origin_m: [0.0, params.lowest_altitude_m, 0.0],
        };

        let mut vol = FroxelVolume::new([64, 64, 64], FROXEL_DEPTH_SLICES).expect("canonical grid");
        assert!(vol.density.iter().all(|&d| d == 0.0), "建造期零初始化");
        frontend.write_density(&mut vol).expect("云前端写密度场");

        let nonzero = vol.density.iter().filter(|&&d| d > 0.0).count();
        assert!(nonzero > 0, "满覆盖云前端应写出非零密度");
        assert!(
            vol.density.iter().all(|d| d.is_finite() && *d >= 0.0),
            "密度场全有限非负"
        );

        // 与雾前端共用同一 volume 单入口:雾写完云再写,不产生结构冲突。
        let mut shared = FroxelVolume::new([64, 64, 64], FROXEL_DEPTH_SLICES).unwrap();
        let fog = FogFrontend {
            base_density: 0.05,
            falloff_m: 64.0,
            height_offset_m: 0.0,
            slice_spacing_m: 2.0,
            log_slices: true,
        };
        fog.write_density(&mut shared).expect("雾前端");
        frontend
            .write_density(&mut shared)
            .expect("云前端复写同一 volume");
        assert_eq!(shared.voxel_count(), vol.voxel_count(), "共用同一网格形态");

        // 逐 voxel 与密度模型直查一致(单一事实源核验)。
        let idx = 20 * 64 * 64 + 32 * 64 + 32;
        let sx = frontend.world_extent_m / 64.0;
        let sz = frontend.world_extent_m / 64.0;
        let expect_pos = [
            frontend.origin_m[0] + (32.0 - 32.0) * sx,
            frontend.origin_m[1] + 20.0 * frontend.slice_spacing_m,
            frontend.origin_m[2] + (32.0 - 32.0) * sz,
        ];
        let direct = renderer.evaluate_cloud_properties(expect_pos, true);
        assert_eq!(
            vol.density[idx].to_bits(),
            (direct.density * direct.sigma_t).to_bits(),
            "froxel 密度须与密度模型直查位级相等"
        );
    }

    /// 云前端非法参数 fail-closed(与雾前端同错误面)。
    #[test]
    fn cloud_frontend_rejects_invalid_params() {
        use crate::world::atmosphere::FROXEL_DEPTH_SLICES;

        let sky = Sky::new(PRESET_CLEAR);
        let w = uniform_weather(1.0, 0.5);
        let renderer = CloudRenderer::new(CloudParams::default(), &sky, &w, test_noise());
        let mut vol = FroxelVolume::new([64, 64, 64], FROXEL_DEPTH_SLICES).unwrap();

        for (extent, spacing) in [
            (0.0f32, 40.0f32),
            (-1.0, 40.0),
            (20_000.0, 0.0),
            (20_000.0, -3.0),
        ] {
            let f = CloudFrontend {
                renderer: &renderer,
                world_extent_m: extent,
                slice_spacing_m: spacing,
                origin_m: [0.0, 1_500.0, 0.0],
            };
            assert!(
                matches!(
                    f.write_density(&mut vol),
                    Err(AtmosphereError::NonFiniteValue { .. })
                ),
                "extent={extent} spacing={spacing} 应 fail-closed"
            );
        }
    }
}
