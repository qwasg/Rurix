//! 程序化物理天空(Rayleigh + Mie + 臭氧单散射;G40 体积云展示面底座)。
//!
//! 本模块是 [`super::clouds`] 的照明与背景事实源:体积云的太阳色、天顶/地平
//! 环境光探针、以及云层之外的天空背景辐亮度全部由此单点产出。**零外部资产、
//! 零外部 crate、host 纯 safe 确定性**——同参数双跑位级一致。
//!
//! ## 大气模型
//!
//! 行星大气球壳(地表半径 [`EARTH_RADIUS_M`]、大气层顶 [`ATMOSPHERE_TOP_M`])
//! 内沿视线做单散射数值积分,三类介质各自独立:
//!
//! - **Rayleigh(分子散射)**:密度 `exp(-h / 8km)`,散射系数波长强相关
//!   (蓝端约为红端 5.7 倍)——天空之所以是蓝的、日落之所以是红的。
//! - **Mie(气溶胶散射)**:密度 `exp(-h / 1.2km)`,散射系数近乎灰色 + 强前向
//!   相位(g = 0.8)——太阳周围的白色光晕与地平线附近的霾。
//! - **臭氧(纯吸收)**:25km 处峰值的帐篷函数分布,不散射只吸收——黄昏天顶的
//!   青蓝色调来源(缺了它日落天空会偏土黄)。
//!
//! 系数取值沿 Hillaire 2020《A Scalable and Production Ready Sky and
//! Atmosphere Rendering Technique》的标准标定面(与 Bruneton 2008 同源)。
//!
//! ## 坐标约定
//!
//! **Y-up 右手系**:`+Y` = 天顶,`+X` = 东,`+Z` = 北。方位角自北起顺时针
//! (北 0° / 东 90° / 南 180° / 西 270°),与气象/天文惯例一致。此约定与
//! HPVolumeCloud 参考实现(HDRP `positionWS.y` 为高度)以及 glTF 同构。
//!
//! ## 预设标定来源
//!
//! 四个命名预设的太阳高度角与色温取自 Poly Haven CC0「Pure Sky」实拍天空
//! (纯天空穹顶、无地景遮挡),资产 slug 与出处逐条登记在 [`SKY_PRESETS`] 各
//! 常量的文档注释。**只取标定数值,不入库任何二进制资产。**

// ---------------------------------------------------------------------------
// 冻结常量面(大气几何与介质系数)
// ---------------------------------------------------------------------------

/// 地表半径(m;Bruneton/Hillaire 标准值)。
pub const EARTH_RADIUS_M: f32 = 6_360_000.0;
/// 大气层顶半径(m;地表以上 60km)。
pub const ATMOSPHERE_TOP_M: f32 = 6_420_000.0;

/// Rayleigh 散射系数(m⁻¹,RGB;海平面)。蓝端 ≈ 红端 5.7 倍 = 天空蓝的成因。
const RAYLEIGH_SCATTERING: [f32; 3] = [5.802e-6, 13.558e-6, 33.100e-6];
/// Rayleigh 密度标高(m)。
const RAYLEIGH_SCALE_HEIGHT: f32 = 8_000.0;

/// Mie 散射系数(m⁻¹;灰色)。
const MIE_SCATTERING: f32 = 3.996e-6;
/// Mie 消光系数(m⁻¹;> 散射系数,差值即气溶胶吸收)。
const MIE_EXTINCTION: f32 = 4.440e-6;
/// Mie 密度标高(m)。
const MIE_SCALE_HEIGHT: f32 = 1_200.0;
/// Mie 相位函数偏心率(前向散射;太阳光晕)。
const MIE_G: f32 = 0.8;

/// 臭氧吸收系数(m⁻¹,RGB;帐篷分布峰值处)。只吸收不散射。
const OZONE_ABSORPTION: [f32; 3] = [0.650e-6, 1.881e-6, 0.085e-6];
/// 臭氧帐篷函数中心高度(m)。
const OZONE_CENTER_M: f32 = 25_000.0;
/// 臭氧帐篷函数半宽(m)。
const OZONE_WIDTH_M: f32 = 15_000.0;

/// 视线积分步数(单散射数值积分;确定性冻结面)。
const VIEW_STEPS: u32 = 32;
/// 太阳透射率积分步数(次级光路)。
const SUN_STEPS: u32 = 8;

/// 太阳角半径(rad;约 0.267°)。
const SUN_ANGULAR_RADIUS: f32 = 0.004_654;

// ---------------------------------------------------------------------------
// 预设面(Poly Haven CC0 Pure Sky 标定)
// ---------------------------------------------------------------------------

/// 天空预设(太阳位置 + 大气浊度 + 曝光标定)。
///
/// `turbidity` 以 1.0 为标准洁净大气,越大气溶胶越多(霾/雾感越强);实现上
/// 直接缩放 Mie 密度。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyPreset {
    /// 预设名(CLI `--preset` 取值)。
    pub name: &'static str,
    /// 太阳高度角(度;0 = 地平线,90 = 天顶,负值 = 地平线下)。
    pub sun_elevation_deg: f32,
    /// 太阳方位角(度;自北起顺时针)。
    pub sun_azimuth_deg: f32,
    /// 大气浊度(Mie 密度倍率;1.0 = 洁净)。
    pub turbidity: f32,
    /// 太阳辐照度倍率(相对标准太阳常数的标定系数)。
    pub sun_intensity: f32,
    /// 地面反照率(参与地平线以下的漫射回弹)。
    pub ground_albedo: f32,
    /// 展示用曝光(EV100;由参考资产 `evs_cap` 换算)。
    pub ev100: f32,
    /// 标定来源的 Poly Haven 资产 slug(CC0;只取数值不入库资产)。
    pub reference_slug: &'static str,
}

/// 正午高日档。
///
/// 标定源 `kloofendal_48d_partly_cloudy_puresky`(Poly Haven CC0,作者
/// Greg Zaal)。高度角取资产名编码的 48°;白平衡 5400K、`evs_cap` 21。
pub const PRESET_NOON: SkyPreset = SkyPreset {
    name: "noon",
    sun_elevation_deg: 48.0,
    sun_azimuth_deg: 200.0,
    turbidity: 1.0,
    sun_intensity: 1.0,
    ground_albedo: 0.15,
    ev100: 14.0,
    reference_slug: "kloofendal_48d_partly_cloudy_puresky",
};

/// 晴空白昼档(四档中唯一时间戳可核验的一档)。
///
/// 标定源 `kloofendal_43d_clear_puresky`(Poly Haven CC0,作者 Greg Zaal)。
/// 资产名编码 43°,由其 `coords` (-26.1234, 27.8836) 与 `date_taken`
/// (2023-05-17T10:46Z) 按 NOAA 太阳位置算法反算得 elev 43.470° / azim
/// 346.578°——与资产名逐位吻合,故本档方位角取实算值。白平衡 5313K、
/// `evs_cap` 22。
pub const PRESET_CLEAR: SkyPreset = SkyPreset {
    name: "clear",
    sun_elevation_deg: 43.470,
    sun_azimuth_deg: 346.578,
    turbidity: 0.85,
    sun_intensity: 1.0,
    ground_albedo: 0.12,
    ev100: 14.5,
    reference_slug: "kloofendal_43d_clear_puresky",
};

/// 黄金时刻档(低角度暖光,云侧受光与银边最出效果)。
///
/// 标定源 `evening_road_01_puresky`(Poly Haven CC0,作者 Sergej Majboroda
/// 摄 / Jarod Guest 天空编辑)。该资产 `coords` 缺失、`date_taken` 不可核验,
/// 故高度角按其 `attributes.time_of_day = sunset` 与描述「low golden sun,
/// medium contrast」取 8°。白平衡 5400K、`evs_cap` 12。
pub const PRESET_GOLDEN: SkyPreset = SkyPreset {
    name: "golden",
    sun_elevation_deg: 8.0,
    sun_azimuth_deg: 275.0,
    turbidity: 1.6,
    sun_intensity: 1.0,
    ground_albedo: 0.10,
    ev100: 12.0,
    reference_slug: "evening_road_01_puresky",
};

/// 日落档(太阳贴地平线;Rayleigh 长光程 + 臭氧共同给出暖橙地平 + 青紫天顶)。
///
/// 标定源 `belfast_sunset_puresky`(Poly Haven CC0,作者 Dimitrios Savva 摄 /
/// Greg Zaal 处理 / Jarod Guest 天空编辑)。其 `date_taken` 反算得 elev
/// −28.8°(深夜)与 `attributes.time_of_day = sunset` 矛盾,判定时间戳不可信,
/// 故高度角按描述「warm golden horizon, soft violet-blue light」取 2°。
/// `evs_cap` 12。
pub const PRESET_SUNSET: SkyPreset = SkyPreset {
    name: "sunset",
    sun_elevation_deg: 2.0,
    sun_azimuth_deg: 288.0,
    turbidity: 2.2,
    sun_intensity: 1.0,
    ground_albedo: 0.08,
    ev100: 10.5,
    reference_slug: "belfast_sunset_puresky",
};

/// 全部命名预设(CLI 闭集;顺序 = 太阳高度角降序)。
pub const SKY_PRESETS: [SkyPreset; 4] = [PRESET_NOON, PRESET_CLEAR, PRESET_GOLDEN, PRESET_SUNSET];

/// 按名查预设(闭集外返回 `None`,由调用方 fail-closed)。
pub fn preset_by_name(name: &str) -> Option<SkyPreset> {
    SKY_PRESETS.into_iter().find(|p| p.name == name)
}

// ---------------------------------------------------------------------------
// 天空求值器
// ---------------------------------------------------------------------------

/// 程序化天空求值器(由 [`SkyPreset`] 建造;所有查询纯函数、无内部可变状态)。
#[derive(Debug, Clone)]
pub struct Sky {
    preset: SkyPreset,
    /// 指向太阳的单位向量(Y-up)。
    sun_dir: [f32; 3],
    /// 太阳盘辐亮度(已过大气衰减;RGB)。
    sun_color: [f32; 3],
    /// 天顶方向环境光探针(上半球平均;供云顶漫射)。
    ambient_top: [f32; 3],
    /// 地平线以下环境光探针(地面回弹;供云底漫射)。
    ambient_bottom: [f32; 3],
}

impl Sky {
    /// 由预设建造(建造期一次性预算太阳色与两个环境光探针)。
    pub fn new(preset: SkyPreset) -> Self {
        let sun_dir = direction_from_angles(preset.sun_elevation_deg, preset.sun_azimuth_deg);
        let mut sky = Self {
            preset,
            sun_dir,
            sun_color: [0.0; 3],
            ambient_top: [0.0; 3],
            ambient_bottom: [0.0; 3],
        };
        sky.sun_color = sky.compute_sun_color();
        let (top, bottom) = sky.compute_ambient_probe();
        sky.ambient_top = top;
        sky.ambient_bottom = bottom;
        sky
    }

    /// 建造本求值器的预设。
    pub fn preset(&self) -> SkyPreset {
        self.preset
    }

    /// 指向太阳的单位向量(Y-up;`+Y` 天顶 / `+X` 东 / `+Z` 北)。
    pub fn sun_direction(&self) -> [f32; 3] {
        self.sun_dir
    }

    /// 太阳盘辐亮度(RGB;已含沿太阳光路的大气透射衰减)。
    ///
    /// 这是体积云直接光照的输入——低太阳高度角时长光程把蓝端吃掉,自然得到
    /// 暖橙色的云受光面。
    pub fn sun_color(&self) -> [f32; 3] {
        self.sun_color
    }

    /// 环境光探针 `(天顶项, 地平以下项)`。
    ///
    /// 对应 HPVolumeCloud 参考实现里 `SampleSH9(probe, ±Y)` 的
    /// `ambientTermTop` / `ambientTermBottom` 两项:云顶接上半球天光,云底接
    /// 地面回弹光。
    pub fn ambient_probe(&self) -> ([f32; 3], [f32; 3]) {
        (self.ambient_top, self.ambient_bottom)
    }

    /// 视线方向的天空背景辐亮度(RGB;不含太阳盘本体)。
    ///
    /// `dir` 须为单位向量。地平线以下方向返回地面反照率调制后的暗色。
    pub fn radiance(&self, dir: [f32; 3]) -> [f32; 3] {
        self.scatter(dir, false)
    }

    /// 含太阳盘的天空辐亮度(视线正对太阳时叠加日面本体;供背景出图)。
    pub fn radiance_with_sun(&self, dir: [f32; 3]) -> [f32; 3] {
        let mut c = self.scatter(dir, false);
        let cos_sun = dot(dir, self.sun_dir);
        // 日面软边(角半径内满强度,边缘 smoothstep 过渡一个角半径)。
        let cos_edge = SUN_ANGULAR_RADIUS.cos();
        let cos_outer = (SUN_ANGULAR_RADIUS * 2.0).cos();
        if cos_sun > cos_outer {
            let t = ((cos_sun - cos_outer) / (cos_edge - cos_outer)).clamp(0.0, 1.0);
            let disk = t * t * (3.0 - 2.0 * t);
            // 日面辐亮度 = 太阳色 / 立体角(角半径 → sr),给出物理量级的过曝核心。
            let solid_angle = std::f32::consts::PI * SUN_ANGULAR_RADIUS * SUN_ANGULAR_RADIUS;
            for (out, sun) in c.iter_mut().zip(self.sun_color) {
                *out += sun / solid_angle * disk;
            }
        }
        c
    }

    // -- 内部:单散射积分 ---------------------------------------------------

    /// 沿视线的 Rayleigh + Mie 单散射积分(相机置于地表以上 1m 的观察者)。
    fn scatter(&self, dir: [f32; 3], _unused: bool) -> [f32; 3] {
        let origin = [0.0f32, EARTH_RADIUS_M + 1.0, 0.0];
        // 视线与大气层顶求交;若先撞地面则积分到地面。
        let Some(t_top) = ray_sphere_far(origin, dir, ATMOSPHERE_TOP_M) else {
            return [0.0; 3];
        };
        let t_end = match ray_sphere_near(origin, dir, EARTH_RADIUS_M) {
            Some(t_ground) if t_ground > 0.0 => t_ground.min(t_top),
            _ => t_top,
        };
        if t_end <= 0.0 {
            return [0.0; 3];
        }

        let cos_theta = dot(dir, self.sun_dir);
        let phase_r = rayleigh_phase(cos_theta);
        let phase_m = mie_phase(cos_theta, MIE_G);

        let dt = t_end / VIEW_STEPS as f32;
        // 逐段透射率累乘(RGB;Rayleigh + Mie + 臭氧三项消光)。
        let mut transmittance = [1.0f32; 3];
        let mut radiance = [0.0f32; 3];

        for i in 0..VIEW_STEPS {
            let t = (i as f32 + 0.5) * dt;
            let p = add(origin, mul(dir, t));
            let h = length(p) - EARTH_RADIUS_M;
            let (dens_r, dens_m, dens_o) = self.densities(h);

            // 本段消光光学厚度。
            let mut seg_ext = [0.0f32; 3];
            for c in 0..3 {
                seg_ext[c] = (RAYLEIGH_SCATTERING[c] * dens_r
                    + MIE_EXTINCTION * dens_m
                    + OZONE_ABSORPTION[c] * dens_o)
                    * dt;
            }

            // 太阳方向透射率(次级光路;被地球遮挡则为零)。
            let sun_t = self.sun_transmittance(p);

            // 本段入散射 = (Rayleigh 散射 × Rayleigh 相位 + Mie 散射 × Mie 相位)
            //             × 太阳透射 × 视线累计透射 × 段长。
            for c in 0..3 {
                let scat =
                    RAYLEIGH_SCATTERING[c] * dens_r * phase_r + MIE_SCATTERING * dens_m * phase_m;
                // 段内解析积分(避免大 σ 时的阶梯误差):∫T dt = T·(1−e^(−σΔ))/σ。
                let seg_t = (-seg_ext[c]).exp();
                let denom = if seg_ext[c] > 1e-8 { seg_ext[c] } else { 1e-8 };
                let integ = (1.0 - seg_t) / denom * dt;
                radiance[c] += transmittance[c] * scat * sun_t[c] * integ;
                transmittance[c] *= seg_t;
            }
        }

        // 地面回弹(视线撞地时叠加漫射地面项)。
        if dir[1] < 0.0 {
            let a = self.preset.ground_albedo;
            let sun_up = self.sun_dir[1].max(0.0);
            for c in 0..3 {
                radiance[c] +=
                    transmittance[c] * a * sun_up * self.sun_color[c] / std::f32::consts::PI;
            }
        }

        for r in &mut radiance {
            *r *= self.preset.sun_intensity;
        }
        radiance
    }

    /// 三类介质在高度 `h`(m,地表以上)的归一化密度 `(rayleigh, mie, ozone)`。
    fn densities(&self, h: f32) -> (f32, f32, f32) {
        let h = h.max(0.0);
        let dens_r = (-h / RAYLEIGH_SCALE_HEIGHT).exp();
        let dens_m = (-h / MIE_SCALE_HEIGHT).exp() * self.preset.turbidity;
        // 臭氧帐篷函数:峰值 1.0 在 OZONE_CENTER_M,线性降到半宽外的 0。
        let dens_o = (1.0 - (h - OZONE_CENTER_M).abs() / OZONE_WIDTH_M).clamp(0.0, 1.0);
        (dens_r, dens_m, dens_o)
    }

    /// 自点 `p` 沿太阳方向到大气层顶的透射率(RGB;被地球遮挡返回零)。
    fn sun_transmittance(&self, p: [f32; 3]) -> [f32; 3] {
        // 光路撞地球 ⇒ 该点处于地影中。
        if ray_sphere_near(p, self.sun_dir, EARTH_RADIUS_M).is_some_and(|t| t > 0.0) {
            return [0.0; 3];
        }
        let Some(t_top) = ray_sphere_far(p, self.sun_dir, ATMOSPHERE_TOP_M) else {
            return [0.0; 3];
        };
        let dt = t_top / SUN_STEPS as f32;
        let mut od = [0.0f32; 3];
        for i in 0..SUN_STEPS {
            let t = (i as f32 + 0.5) * dt;
            let q = add(p, mul(self.sun_dir, t));
            let h = length(q) - EARTH_RADIUS_M;
            let (dens_r, dens_m, dens_o) = self.densities(h);
            for (c, od_c) in od.iter_mut().enumerate() {
                *od_c += (RAYLEIGH_SCATTERING[c] * dens_r
                    + MIE_EXTINCTION * dens_m
                    + OZONE_ABSORPTION[c] * dens_o)
                    * dt;
            }
        }
        [(-od[0]).exp(), (-od[1]).exp(), (-od[2]).exp()]
    }

    /// 太阳盘辐照度(地表观察者所见;透射衰减后)。
    fn compute_sun_color(&self) -> [f32; 3] {
        // 地平线以下 ⇒ 无直射。
        if self.sun_dir[1] <= 0.0 {
            return [0.0; 3];
        }
        let origin = [0.0f32, EARTH_RADIUS_M + 1.0, 0.0];
        let t = self.sun_transmittance(origin);
        // 大气层外太阳辐照度(W/m²,已按 CIE 三刺激粗分到 RGB 并归一化到 ~1.0
        // 量级——绝对标定由 ev100 承担,此处只保留相对光谱形状)。
        const SOLAR_IRRADIANCE: [f32; 3] = [1.0, 0.98, 0.92];
        [
            SOLAR_IRRADIANCE[0] * t[0] * self.preset.sun_intensity,
            SOLAR_IRRADIANCE[1] * t[1] * self.preset.sun_intensity,
            SOLAR_IRRADIANCE[2] * t[2] * self.preset.sun_intensity,
        ]
    }

    /// 环境光探针 `(天顶项, 云底项)`。
    ///
    /// - **天顶项** = 上半球余弦加权平均天空辐亮度(云顶朝上所见)。
    /// - **云底项** = 地面出射辐亮度。云底朝下看到的是**地面回弹光**而非天空:
    ///   地面接收的下行辐照度 `E = π·L̄_sky + L_sun·max(sun_y, 0)`,朗伯地面
    ///   出射辐亮度 `L_ground = albedo·E/π`。反照率 < 1 保证其不强于天光。
    fn compute_ambient_probe(&self) -> ([f32; 3], [f32; 3]) {
        // 8×16 等面积上半球网格(确定性,无随机);余弦加权平均。
        const N_THETA: u32 = 8;
        const N_PHI: u32 = 16;
        let mut top = [0.0f32; 3];
        let mut w_top = 0.0f32;
        for i in 0..N_THETA {
            // 等面积采样:cos(theta) 均匀分布于 [-1, 1];只取上半球。
            let cos_t = 1.0 - 2.0 * (i as f32 + 0.5) / N_THETA as f32;
            if cos_t <= 0.0 {
                continue;
            }
            let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
            for j in 0..N_PHI {
                let phi = std::f32::consts::TAU * (j as f32 + 0.5) / N_PHI as f32;
                let dir = [sin_t * phi.cos(), cos_t, sin_t * phi.sin()];
                let r = self.scatter(dir, false);
                for c in 0..3 {
                    top[c] += r[c] * cos_t;
                }
                w_top += cos_t;
            }
        }
        if w_top > 0.0 {
            for c in top.iter_mut() {
                *c /= w_top;
            }
        }

        // 地面朗伯出射:albedo × (天光平均 + 直射辐照/π)。
        let a = self.preset.ground_albedo;
        let sun_up = self.sun_dir[1].max(0.0);
        let mut bottom = [0.0f32; 3];
        for c in 0..3 {
            bottom[c] = a * (top[c] + self.sun_color[c] * sun_up / std::f32::consts::PI);
        }
        (top, bottom)
    }
}

// ---------------------------------------------------------------------------
// sky-view LUT(device 腿事实源)
// ---------------------------------------------------------------------------

/// sky-view LUT 参数化的 `cos(view, sun)` 轴分辨率。
pub const SKY_LUT_W: usize = 128;
/// sky-view LUT 参数化的 `dir.y` 轴分辨率。
pub const SKY_LUT_H: usize = 128;

/// 烘焙 sky-view LUT(`SKY_LUT_W × SKY_LUT_H × 3` f32,行主序)。
///
/// 参数化取 `u = (dot(dir, sun) + 1)/2`、`v = (dir.y + 1)/2`。水平均匀大气下这
/// 两个量唯一决定天空辐亮度(绕「天顶-太阳」平面镜像对称),因此这是**精确**
/// 参数化而非近似;且 device 侧只需两次点积,不需要 `atan2`
/// (`g40_volumetric_cloud.rx` 的数学内建表里没有 atan2)。
///
/// **不含日面**:日面角半径仅 0.27°,在 LUT 分辨率下会被抹成一大片,故由 kernel
/// 按 `cos(dir, sun)` 解析叠加。
pub fn bake_sky_view_lut(sky: &Sky) -> Vec<f32> {
    let sun = sky.sun_direction();
    // 太阳方向的水平分量(sun 在天顶时退化,此时任意方位等价)。
    let sh = (sun[0] * sun[0] + sun[2] * sun[2]).sqrt();
    let mut lut = vec![0.0f32; SKY_LUT_W * SKY_LUT_H * 3];
    for j in 0..SKY_LUT_H {
        let dir_y = ((j as f32 + 0.5) / SKY_LUT_H as f32) * 2.0 - 1.0;
        let horiz = (1.0 - dir_y * dir_y).max(0.0).sqrt();
        for i in 0..SKY_LUT_W {
            let cos_gamma = ((i as f32 + 0.5) / SKY_LUT_W as f32) * 2.0 - 1.0;
            // 由 (dir.y, cos_gamma) 反解一条满足约束的视线方向。
            let dir = if sh < 1e-4 {
                // 太阳在天顶:cos_gamma 只由 dir.y 决定,水平方位任取。
                [horiz, dir_y, 0.0]
            } else {
                // 水平分量对「水平太阳方向」的投影须为 k。
                let k = cos_gamma - sun[1] * dir_y;
                let cos_phi = if horiz > 1e-6 {
                    (k / (horiz * sh)).clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                let sin_phi = (1.0 - cos_phi * cos_phi).max(0.0).sqrt();
                // 水平正交基:e1 = 归一化水平太阳方向,e2 ⊥ e1。
                let e1 = [sun[0] / sh, sun[2] / sh];
                let e2 = [-e1[1], e1[0]];
                [
                    (e1[0] * cos_phi + e2[0] * sin_phi) * horiz,
                    dir_y,
                    (e1[1] * cos_phi + e2[1] * sin_phi) * horiz,
                ]
            };
            let r = sky.radiance(normalize(dir));
            let o = (j * SKY_LUT_W + i) * 3;
            lut[o] = r[0];
            lut[o + 1] = r[1];
            lut[o + 2] = r[2];
        }
    }
    lut
}

// ---------------------------------------------------------------------------
// 小工具(host 面 f32 向量;不引入外部数学库)
// ---------------------------------------------------------------------------

fn normalize(a: [f32; 3]) -> [f32; 3] {
    let l = length(a);
    if l > 1e-12 {
        [a[0] / l, a[1] / l, a[2] / l]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// 高度角/方位角(度)→ 单位方向向量(Y-up;方位角自北起顺时针)。
pub fn direction_from_angles(elevation_deg: f32, azimuth_deg: f32) -> [f32; 3] {
    let el = elevation_deg.to_radians();
    let az = azimuth_deg.to_radians();
    let horiz = el.cos();
    [horiz * az.sin(), el.sin(), horiz * az.cos()]
}

/// Rayleigh 相位函数 `3/(16π)·(1 + cos²θ)`(4π 上归一化)。
pub fn rayleigh_phase(cos_theta: f32) -> f32 {
    3.0 / (16.0 * std::f32::consts::PI) * (1.0 + cos_theta * cos_theta)
}

/// Cornette-Shanks Mie 相位函数(4π 上归一化;`g` 为偏心率)。
pub fn mie_phase(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let num = 3.0 * (1.0 - g2) * (1.0 + cos_theta * cos_theta);
    let den = 8.0
        * std::f32::consts::PI
        * (2.0 + g2)
        * (1.0 + g2 - 2.0 * g * cos_theta).max(1e-6).powf(1.5);
    num / den
}

/// 射线与以原点为心、半径 `r` 的球的**较远**正交点参数;无交返回 `None`。
fn ray_sphere_far(origin: [f32; 3], dir: [f32; 3], r: f32) -> Option<f32> {
    let b = dot(origin, dir);
    let c = dot(origin, origin) - r * r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b + disc.sqrt();
    if t < 0.0 { None } else { Some(t) }
}

/// 射线与球的**较近**正交点参数(> 0 才有效);无交或全在背后返回 `None`。
fn ray_sphere_near(origin: [f32; 3], dir: [f32; 3], r: f32) -> Option<f32> {
    let b = dot(origin, dir);
    let c = dot(origin, origin) - r * r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t < 0.0 { None } else { Some(t) }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn mul(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn length(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 预设闭集:四档齐备、名字可查、闭集外 fail-closed。
    #[test]
    fn preset_closed_set() {
        assert_eq!(SKY_PRESETS.len(), 4);
        for p in SKY_PRESETS {
            assert_eq!(preset_by_name(p.name), Some(p), "预设 {} 可按名查", p.name);
            assert!(!p.reference_slug.is_empty(), "预设须登记标定来源");
        }
        assert_eq!(preset_by_name("nonexistent"), None);
        // 顺序 = 太阳高度角降序。
        for w in SKY_PRESETS.windows(2) {
            assert!(w[0].sun_elevation_deg > w[1].sun_elevation_deg);
        }
    }

    /// 方向换算:天顶/东/北三个基准角度逐分量核对。
    #[test]
    fn direction_from_angles_basis() {
        let up = direction_from_angles(90.0, 0.0);
        assert!((up[1] - 1.0).abs() < 1e-5, "90° 高度角 = +Y 天顶");
        let east = direction_from_angles(0.0, 90.0);
        assert!((east[0] - 1.0).abs() < 1e-5, "方位角 90° = +X 东");
        let north = direction_from_angles(0.0, 0.0);
        assert!((north[2] - 1.0).abs() < 1e-5, "方位角 0° = +Z 北");
        // 全为单位向量。
        for (el, az) in [(43.47, 346.578), (8.0, 275.0), (2.0, 288.0), (-10.0, 45.0)] {
            let d = direction_from_angles(el, az);
            assert!((length(d) - 1.0).abs() < 1e-5);
        }
    }

    /// 相位函数在 4π 立体角上归一化(球面数值积分,容差 1%)。
    #[test]
    fn phase_functions_normalized() {
        const N: u32 = 2048;
        let mut sum_r = 0.0f64;
        let mut sum_m = 0.0f64;
        // cos θ 在 [-1,1] 均匀 ⇒ dΩ = 2π d(cosθ)。
        for i in 0..N {
            let c = -1.0 + 2.0 * (i as f32 + 0.5) / N as f32;
            let dw = 2.0 * std::f64::consts::PI * (2.0 / N as f64);
            sum_r += f64::from(rayleigh_phase(c)) * dw;
            sum_m += f64::from(mie_phase(c, MIE_G)) * dw;
        }
        assert!((sum_r - 1.0).abs() < 0.01, "Rayleigh 相位归一化: {sum_r}");
        assert!((sum_m - 1.0).abs() < 0.01, "Mie 相位归一化: {sum_m}");
    }

    /// 天空辐亮度全有限、非负,且天顶偏蓝(蓝通道 > 红通道)。
    #[test]
    fn zenith_is_blue_and_finite() {
        let sky = Sky::new(PRESET_CLEAR);
        let zenith = sky.radiance([0.0, 1.0, 0.0]);
        assert!(
            zenith.iter().all(|v| v.is_finite() && *v >= 0.0),
            "天顶有限非负"
        );
        assert!(
            zenith[2] > zenith[0] * 1.5,
            "天顶蓝端显著强于红端: {zenith:?}"
        );
        // 全方向有限。
        for (el, az) in [
            (90.0, 0.0),
            (45.0, 90.0),
            (5.0, 180.0),
            (0.0, 270.0),
            (-30.0, 0.0),
        ] {
            let r = sky.radiance(direction_from_angles(el, az));
            assert!(
                r.iter().all(|v| v.is_finite() && *v >= 0.0),
                "{el}/{az} 有限非负"
            );
        }
    }

    /// 太阳高度角下降 ⇒ 直射光路变长 ⇒ 太阳色红移(蓝红比单调下降)。
    #[test]
    fn low_sun_reddens() {
        let mut prev_ratio = f32::INFINITY;
        for elev in [60.0f32, 40.0, 20.0, 10.0, 4.0, 1.0] {
            let p = SkyPreset {
                name: "t",
                sun_elevation_deg: elev,
                sun_azimuth_deg: 180.0,
                turbidity: 1.0,
                sun_intensity: 1.0,
                ground_albedo: 0.1,
                ev100: 13.0,
                reference_slug: "test",
            };
            let sun = Sky::new(p).sun_color();
            assert!(
                sun.iter().all(|v| v.is_finite() && *v > 0.0),
                "elev {elev} 太阳色非零"
            );
            let ratio = sun[2] / sun[0];
            assert!(
                ratio < prev_ratio,
                "elev {elev} 蓝红比 {ratio} 应低于上一档 {prev_ratio}"
            );
            prev_ratio = ratio;
        }
        // 日落档蓝端被显著吃掉。
        assert!(prev_ratio < 0.5, "1° 太阳蓝红比应显著小于 1: {prev_ratio}");
    }

    /// 地平线以下的太阳 ⇒ 无直射光(fail-closed 到零,不产生负值或 NaN)。
    #[test]
    fn sun_below_horizon_is_dark() {
        let p = SkyPreset {
            name: "night",
            sun_elevation_deg: -10.0,
            sun_azimuth_deg: 90.0,
            turbidity: 1.0,
            sun_intensity: 1.0,
            ground_albedo: 0.1,
            ev100: 8.0,
            reference_slug: "test",
        };
        let sky = Sky::new(p);
        assert_eq!(sky.sun_color(), [0.0, 0.0, 0.0]);
        // 天空本身仍应有限(散射项可为 0,但不得 NaN)。
        let z = sky.radiance([0.0, 1.0, 0.0]);
        assert!(z.iter().all(|v| v.is_finite() && *v >= 0.0));
    }

    /// 环境光探针:天顶项非零有限;云底项(地面朗伯回弹)满足能量守恒——
    /// 出射辐射度 `π·L_ground` 不得超过入射辐照度 `π·L̄_sky + L_sun·sinθ`。
    ///
    /// 注意**不能**断言「云底项 ≤ 天顶项」:正午被直射的地面确实比天空平均
    /// 辐亮度更亮(直射辐照 ≫ 天光辐照),那是物理事实而非缺陷。
    #[test]
    fn ambient_probe_energy_conserving() {
        for preset in SKY_PRESETS {
            let sky = Sky::new(preset);
            let (top, bottom) = sky.ambient_probe();
            let sun_up = sky.sun_direction()[1].max(0.0);
            assert!(
                top.iter().all(|v| v.is_finite() && *v > 0.0),
                "{} 天顶探针非零: {top:?}",
                preset.name
            );
            for c in 0..3 {
                assert!(
                    bottom[c].is_finite() && bottom[c] >= 0.0,
                    "{} 云底探针有限非负: {bottom:?}",
                    preset.name
                );
                // 入射辐照度(天光半球项 + 直射水平分量)。
                let incident = std::f32::consts::PI * top[c] + sky.sun_color()[c] * sun_up;
                let exitant = std::f32::consts::PI * bottom[c];
                assert!(
                    exitant <= incident + 1e-6,
                    "{} 通道 {c} 地面出射 {exitant} 超过入射 {incident}(能量不守恒)",
                    preset.name
                );
            }
        }
    }

    /// 云底回弹光对地面反照率线性(反照率 = 反射比,0 反照率 ⇒ 无回弹)。
    #[test]
    fn ambient_bottom_scales_with_albedo() {
        let base = SkyPreset {
            name: "t",
            sun_elevation_deg: 40.0,
            sun_azimuth_deg: 180.0,
            turbidity: 1.0,
            sun_intensity: 1.0,
            ground_albedo: 0.0,
            ev100: 13.0,
            reference_slug: "test",
        };
        let zero = Sky::new(base).ambient_probe().1;
        assert_eq!(zero, [0.0, 0.0, 0.0], "零反照率 ⇒ 无地面回弹");

        let half = Sky::new(SkyPreset {
            ground_albedo: 0.2,
            ..base
        })
        .ambient_probe()
        .1;
        let full = Sky::new(SkyPreset {
            ground_albedo: 0.4,
            ..base
        })
        .ambient_probe()
        .1;
        for c in 0..3 {
            assert!(half[c] > 0.0, "非零反照率 ⇒ 有回弹");
            let ratio = full[c] / half[c];
            assert!(
                (ratio - 2.0).abs() < 1e-3,
                "反照率翻倍 ⇒ 回弹翻倍(通道 {c} 实测 {ratio})"
            );
        }
    }

    /// 太阳落到地平线以下 ⇒ 直射项消失,云底回弹只剩天光反射(显著变暗)。
    #[test]
    fn ambient_bottom_dims_when_sun_sets() {
        let day = SkyPreset {
            name: "t",
            sun_elevation_deg: 40.0,
            sun_azimuth_deg: 180.0,
            turbidity: 1.0,
            sun_intensity: 1.0,
            ground_albedo: 0.2,
            ev100: 13.0,
            reference_slug: "test",
        };
        let night = SkyPreset {
            sun_elevation_deg: -6.0,
            ..day
        };
        let d: f32 = Sky::new(day).ambient_probe().1.iter().sum();
        let n: f32 = Sky::new(night).ambient_probe().1.iter().sum();
        assert!(n < d * 0.2, "日落后地面回弹应显著变暗: {n} vs {d}");
        assert!(n >= 0.0 && n.is_finite());
    }

    /// 确定性:同预设双跑逐位相等(展示图 digest 可复现的前提)。
    #[test]
    fn deterministic_double_run() {
        let dirs: Vec<[f32; 3]> = (0..64)
            .map(|i| direction_from_angles(i as f32 * 1.4 - 20.0, i as f32 * 5.6))
            .collect();
        for preset in SKY_PRESETS {
            let a = Sky::new(preset);
            let b = Sky::new(preset);
            assert_eq!(
                a.sun_color().map(f32::to_bits),
                b.sun_color().map(f32::to_bits)
            );
            assert_eq!(
                a.ambient_probe().0.map(f32::to_bits),
                b.ambient_probe().0.map(f32::to_bits)
            );
            for d in &dirs {
                assert_eq!(
                    a.radiance(*d).map(f32::to_bits),
                    b.radiance(*d).map(f32::to_bits),
                    "{} 方向 {d:?} 双跑位级",
                    preset.name
                );
            }
        }
    }

    /// 日面叠加:正对太阳的辐亮度远高于偏离一度的背景。
    #[test]
    fn sun_disk_is_bright() {
        let sky = Sky::new(PRESET_CLEAR);
        let on_sun = sky.radiance_with_sun(sky.sun_direction());
        let off_sun = sky.radiance_with_sun(direction_from_angles(
            PRESET_CLEAR.sun_elevation_deg + 5.0,
            PRESET_CLEAR.sun_azimuth_deg,
        ));
        let s: f32 = on_sun.iter().sum();
        let o: f32 = off_sun.iter().sum();
        assert!(s > o * 100.0, "日面应远亮于 5° 外背景: {s} vs {o}");
        assert!(on_sun.iter().all(|v| v.is_finite()));
    }
}
