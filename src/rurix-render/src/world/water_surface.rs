//! G41 水面渲染前端(门 `g41.water.surface`)。
//!
//! 复现 HPWater(<https://github.com/AshenOneArt/HPWater>,Unity HDRP 水体渲染
//! 系统,MPL-2.0)所刻画的**技术方案**。本模块与 `kernels/g41_water_*.rx` 为
//! **clean-room 重写**:只按公开文献与该项目 README 所述算法族重新推导实现,
//! 不含任何来自该仓库的源码文本(HLSL → `.rx` 的逐行翻译在 MPL-2.0 §1.10 下构成
//! "Modification",与本仓库 `MIT OR Apache-2.0` 授权面冲突;故取技术参照而非
//! 代码派生,先例 = G40 对 HPVolumeCloud 的处理)。技术出处逐条登记于
//! `rfcs/0050-water-surface-rendering.md` §2 Prior art。
//!
//! ## 与 [`super::water`](super::water) 的分工(0-byte 纪律)
//!
//! [`super::water`] = M113 / RXS-0366 **语义面**(Tessendorf 大洋谱资产 + host
//! DFT 参照 + 双管线几何路径互斥机核 + 浮力接口预留),被
//! `milestones/g9/g9_m113_water_band.json` 冻结带锚定,**本模块 0-byte 不触碰**。
//! 本模块 = G41 **渲染面**:浅水波方程 device 化 + 水面着色/折射/体积散射的
//! host 金标准。二者只在"浅水波方程"这一数学事实上同源,不共享类型与状态。
//!
//! ## 五层结构(逐层对应一个 device kernel)
//!
//! 1. **波方程**([`WaveSim`] ↔ `g41_water_wave.rx`):`∂²u/∂t² = c²∇²u`
//!    中心差分 + 诺伊曼障碍边界(反射)+ 边缘海绵层吸收 + 高斯波源注入;
//!    三缓冲 ping-pong。
//! 2. **场景**([`LagoonScene`] ↔ `g41_water_scene.rx`):解析泻湖(高度场海床
//!    碗盆 + 球形礁石),光线步进产 **真视深** + scene-linear HDR 场景色。
//!    自持场景是刻意的——生产 Mega 车道的 `U_SCENE_DEPTH` 为 clip.x/y quirk 域
//!    (非真深度),而屏幕空间折射的前提正是真深度。
//! 3. **模糊链**(`g41_water_blur.rx`):3 级 2× box 降采样。执行面纹理为单 mip
//!    (无硬件 mip 链),故 HPWater 的 "散射密度 → mipLevel" 改为显式降采样链 +
//!    [`blur_level`] 选级。
//! 4. **水面**([`WaterParams`] ↔ `g41_water_surface.rx`):水体 GBuffer(法线/
//!    粗糙度/泡沫/σa/σs)+ 指数步进屏幕空间折射 + Fresnel/GGX + 天空反射 +
//!    Beer-Lambert 体积散射 + 三波长色散 + 解析焦散。
//! 5. **编码**(`g41_water_encode.rx`):曝光 → ACES filmic → sRGB → BGRA8。
//!
//! ## 纪律
//!
//! host 纯 safe 确定性(全库 `forbid(unsafe_code)`);全部 f32;无 RNG 消费
//! (抖动取 IGN 解析式);本模块每个公式面函数都是对应 kernel 内联段的**逐字
//! 同源事实源**,`g41_water_probe` 以此对拍。

use super::sky::Sky;

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// 波方程网格边长(2 的幂 ⇒ device 侧环绕索引可用位掩码)。
pub const WAVE_DIM: usize = 256;
/// 烘焙 2D 值噪声边长(2 的幂;海床细节 + 水面法线细节共用)。
pub const NOISE2D_DIM: usize = 256;
/// 礁石(解析球)最大数量。
pub const MAX_ROCKS: usize = 8;
/// 每颗礁石的参数字数(cx, cy, cz, r, albedo_r, albedo_g, albedo_b, rough)。
pub const ROCK_STRIDE: usize = 8;
/// 水面着色参数面字数([`pack_water_params`] 产出长度)。
pub const WATER_PARAM_COUNT: usize = 96 + MAX_ROCKS * ROCK_STRIDE;
/// 空气 → 水折射率比(η = 1 / 1.33)。
pub const AIR_TO_WATER_ETA: f32 = 1.0 / 1.33;
/// 水面 Schlick F0(n=1.33 的垂直入射反射率 ≈ 0.02)。
pub const WATER_F0: f32 = 0.02;
/// 指数步进因子基值(HPWater `EXP_FACTOR` 同量级)。
pub const EXP_FACTOR: f32 = 12.0;
/// 折射光追自适应指数步进参考距离(米)。
pub const REFRACTION_REFERENCE_DISTANCE: f32 = 20.0;
/// 体积散射单次散射采样数。
pub const WATER_SAMPLE_COUNT: usize = 6;
/// 模糊链级数(2× box)。
pub const BLUR_LEVELS: usize = 3;
/// 波方程 CFL 稳定上界(2D:c·dt/dx < 1/√2)。
pub const CFL_LIMIT_2D: f32 = std::f32::consts::FRAC_1_SQRT_2;
/// 单帧最多注入的波源数(device 定长槽位)。
pub const WAVE_MAX_DROPS: usize = 4;
/// 波方程参数面字数([`pack_wave_params`] 产出长度)。
pub const WAVE_PARAM_COUNT: usize = 8 + WAVE_MAX_DROPS * 4;

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed)
// ---------------------------------------------------------------------------

/// 水面渲染前端失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum WaterSurfaceError {
    /// 网格边长非 2 的幂或过小。
    GridNotPowerOfTwo { got: usize },
    /// 波方程参数越 CFL 稳定界(`c·dt` 须 < 1/√2)。
    CflViolation { c_dt: f32, limit: f32 },
    /// 波源注入越界。
    DropOutOfBounds { x: usize, y: usize, dim: usize },
    /// 非有限输入。
    NotFinite(&'static str),
    /// 礁石数超上界。
    TooManyRocks { got: usize, max: usize },
    /// 波源脚本解析失败。
    BadDropScript { why: &'static str },
}

impl std::fmt::Display for WaterSurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaterSurfaceError::GridNotPowerOfTwo { got } => {
                write!(f, "波网格边长 {got} 非 2 的幂(或 < 8)")
            }
            WaterSurfaceError::CflViolation { c_dt, limit } => {
                write!(f, "波方程越 CFL 稳定界: c·dt = {c_dt} ≥ {limit}")
            }
            WaterSurfaceError::DropOutOfBounds { x, y, dim } => {
                write!(f, "波源越界: ({x}, {y}) 超 {dim}×{dim}")
            }
            WaterSurfaceError::NotFinite(what) => write!(f, "非有限输入: {what}"),
            WaterSurfaceError::TooManyRocks { got, max } => {
                write!(f, "礁石数 {got} 超上界 {max}")
            }
            WaterSurfaceError::BadDropScript { why } => write!(f, "波源脚本非法: {why}"),
        }
    }
}

impl std::error::Error for WaterSurfaceError {}

/// 本模块结果别名。
pub type Result<T> = std::result::Result<T, WaterSurfaceError>;

// ---------------------------------------------------------------------------
// 层 1:波方程(host 金标准 ↔ g41_water_wave.rx)
// ---------------------------------------------------------------------------

/// 波方程仿真参数(host 与 device 逐字同源)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveParams {
    /// 波速 c(格/步)。
    pub speed: f32,
    /// 阻尼系数 ∈ [0, 1)(每步能量保留 = 1 − damping)。
    pub damping: f32,
    /// 时间步长 Δt。
    pub dt: f32,
    /// 海绵层厚度占边长比例 ∈ (0, 0.5](边缘吸收,防反射回弹)。
    pub sponge_frac: f32,
}

impl Default for WaveParams {
    fn default() -> Self {
        Self {
            speed: 0.7,
            damping: 0.004,
            dt: 1.0,
            sponge_frac: 0.10,
        }
    }
}

impl WaveParams {
    /// CFL 稳定性校验(2D 中心差分:`c·dt < 1/√2`)。
    pub fn validate(&self) -> Result<()> {
        if !self.speed.is_finite() || !self.damping.is_finite() || !self.dt.is_finite() {
            return Err(WaterSurfaceError::NotFinite("wave params"));
        }
        // 上面已断言三者有限,故此处用 `>=` 不会遇到 NaN 的偏序陷阱。
        let c_dt = self.speed * self.dt;
        if c_dt >= CFL_LIMIT_2D {
            return Err(WaterSurfaceError::CflViolation {
                c_dt,
                limit: CFL_LIMIT_2D,
            });
        }
        Ok(())
    }
}

/// 单次波源注入(高斯脉冲)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveDrop {
    /// 注入帧号。
    pub frame: u32,
    /// 归一化位置 u ∈ [0, 1]。
    pub u: f32,
    /// 归一化位置 v ∈ [0, 1]。
    pub v: f32,
    /// 强度(峰值抬升量)。
    pub intensity: f32,
    /// 半径(格)。
    pub radius: f32,
}

/// 浅水波方程仿真(高度场三缓冲 ping-pong)。
///
/// 离散化(HPWater `HPWaterWaveEquation.compute` 同一物理模型,独立推导):
///
/// ```text
/// u(t+Δt) = u(t) + [u(t) − u(t−Δt)]·retention + c²Δt²·∇²u
/// ∇²u ≈ u_l + u_r + u_u + u_d − 4u        (dx = 1 格)
/// ```
///
/// 边界:障碍格(海床高于水面)与网格外一律取**诺伊曼条件**(邻居 = 自身 ⇒
/// 法向导数 0 ⇒ 全反射);边缘海绵层按到边距离平滑降低能量保留率,吸收出射波。
#[derive(Debug, Clone, PartialEq)]
pub struct WaveSim {
    dim: usize,
    params: WaveParams,
    /// 当前时刻 u(t)。
    cur: Vec<f32>,
    /// 上一时刻 u(t−Δt)。
    prev: Vec<f32>,
    /// 障碍掩码(true = 该格为陆地/礁石,水波在此反射)。
    obstacle: Vec<bool>,
    /// 已步进帧数。
    frame: u32,
}

impl WaveSim {
    /// 建造(`dim` 须为 ≥ 8 的 2 的幂)。
    pub fn new(dim: usize, params: WaveParams) -> Result<Self> {
        if dim < 8 || !dim.is_power_of_two() {
            return Err(WaterSurfaceError::GridNotPowerOfTwo { got: dim });
        }
        params.validate()?;
        Ok(Self {
            dim,
            params,
            cur: vec![0.0; dim * dim],
            prev: vec![0.0; dim * dim],
            obstacle: vec![false; dim * dim],
            frame: 0,
        })
    }

    /// 网格边长。
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// 已步进帧数。
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// 当前高度场(行主序 `[y][x]`)。
    pub fn height(&self) -> &[f32] {
        &self.cur
    }

    /// 上一时刻高度场。
    pub fn prev_height(&self) -> &[f32] {
        &self.prev
    }

    /// 障碍掩码。
    pub fn obstacle(&self) -> &[bool] {
        &self.obstacle
    }

    /// 由场景海床填充障碍掩码:海床高于水面 ⇒ 陆地 ⇒ 障碍格。
    pub fn fill_obstacles_from_scene(&mut self, scene: &LagoonScene) {
        for y in 0..self.dim {
            for x in 0..self.dim {
                let (wx, wz) = self.grid_to_world(scene, x, y);
                self.obstacle[y * self.dim + x] = scene.floor_height(wx, wz) >= scene.water_level;
            }
        }
    }

    /// 波网格坐标 → 世界 XZ(网格覆盖 [`LagoonScene::wave_extent_m`] 见方区域)。
    pub fn grid_to_world(&self, scene: &LagoonScene, x: usize, y: usize) -> (f32, f32) {
        let n = self.dim as f32;
        let u = (x as f32 + 0.5) / n;
        let v = (y as f32 + 0.5) / n;
        let e = scene.wave_extent_m;
        (
            scene.center_x + (u - 0.5) * e,
            scene.center_z + (v - 0.5) * e,
        )
    }

    /// 高斯波源注入(越界即 typed `Err`,RED 锚)。
    pub fn poke(&mut self, u: f32, v: f32, intensity: f32, radius: f32) -> Result<()> {
        if !u.is_finite() || !v.is_finite() || !intensity.is_finite() || !radius.is_finite() {
            return Err(WaterSurfaceError::NotFinite("drop"));
        }
        let n = self.dim as f32;
        let cx = u * n;
        let cy = v * n;
        let xi = cx.floor();
        let yi = cy.floor();
        if !(0.0..n).contains(&xi) || !(0.0..n).contains(&yi) {
            return Err(WaterSurfaceError::DropOutOfBounds {
                x: xi.max(0.0) as usize,
                y: yi.max(0.0) as usize,
                dim: self.dim,
            });
        }
        let r = radius.max(0.5);
        let sigma = r * 0.4;
        let lo_x = (cx - r).floor().max(0.0) as usize;
        let hi_x = ((cx + r).ceil() as usize).min(self.dim - 1);
        let lo_y = (cy - r).floor().max(0.0) as usize;
        let hi_y = ((cy + r).ceil() as usize).min(self.dim - 1);
        for y in lo_y..=hi_y {
            for x in lo_x..=hi_x {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let d2 = dx * dx + dy * dy;
                if d2 <= r * r {
                    // 写成 `−d2 / (2σ²)` 而非 `−d2 · (1/(2σ²))`:与
                    // `g41_water_wave.rx` 的除法形式逐字同序,消除一处可避免的
                    // 舍入差(倒数预乘与直接相除在 f32 下不等价)。
                    let g = (0.0 - d2 / (2.0 * sigma * sigma)).exp();
                    let i = y * self.dim + x;
                    // 障碍格不接受注入(与 device 端算术门同义)。
                    if !self.obstacle[i] {
                        self.cur[i] += g * intensity;
                    }
                }
            }
        }
        Ok(())
    }

    /// 海绵层能量保留率(到边越近保留越低 ⇒ 吸收出射波,防边界回弹)。
    ///
    /// 与 device 内联段逐字同源。
    pub fn retention_at(&self, x: usize, y: usize) -> f32 {
        sponge_retention(x, y, self.dim, self.params.damping, self.params.sponge_frac)
    }

    /// 单步波方程步进。
    pub fn step(&mut self) {
        let dim = self.dim;
        let c2dt2 = self.params.speed * self.params.speed * self.params.dt * self.params.dt;
        let mut next = vec![0.0f32; dim * dim];
        for y in 0..dim {
            for x in 0..dim {
                let i = y * dim + x;
                if self.obstacle[i] {
                    next[i] = 0.0;
                    continue;
                }
                let u_c = self.cur[i];
                // 诺伊曼边界:越界或障碍邻居取自身值 ⇒ 该方向导数为 0 ⇒ 反射。
                let s = |xx: isize, yy: isize| -> f32 {
                    if xx < 0 || yy < 0 || xx >= dim as isize || yy >= dim as isize {
                        return u_c;
                    }
                    let j = yy as usize * dim + xx as usize;
                    if self.obstacle[j] { u_c } else { self.cur[j] }
                };
                let xi = x as isize;
                let yi = y as isize;
                let lap = s(xi - 1, yi) + s(xi + 1, yi) + s(xi, yi - 1) + s(xi, yi + 1) - 4.0 * u_c;
                let retention = self.retention_at(x, y);
                let inertia = (u_c - self.prev[i]) * retention;
                next[i] = u_c + inertia + c2dt2 * lap;
            }
        }
        self.prev = std::mem::replace(&mut self.cur, next);
        self.frame = self.frame.wrapping_add(1);
    }

    /// 步进并按脚本注入本帧波源。
    ///
    /// 注入次序 = **先步进、后注入**(波源加到 `u(t+Δt)` 上),与 device 单 pass
    /// 形态逐字等价:kernel 内无法回写只读的 `u(t)`(其它线程正在读),故一律
    /// 在算出 `next` 之后叠加脉冲。host 若改为"先注入后步进"会让脉冲提前一步
    /// 参与 Laplacian,与 device 产生系统性偏差。
    pub fn step_with_drops(&mut self, drops: &[WaveDrop]) -> Result<()> {
        let f = self.frame;
        self.step();
        for d in drops.iter().filter(|d| d.frame == f) {
            self.poke(d.u, d.v, d.intensity, d.radius)?;
        }
        Ok(())
    }

    /// 本帧应注入的波源(device 参数面打包用;与 [`Self::step_with_drops`] 同一
    /// 帧号判据)。
    pub fn drops_for_frame(drops: &[WaveDrop], frame: u32) -> Vec<WaveDrop> {
        drops.iter().copied().filter(|d| d.frame == frame).collect()
    }

    /// 高度场梯度(中心差分;水面法线来源)。
    pub fn gradient(&self, x: usize, y: usize) -> [f32; 2] {
        let d = self.dim;
        let ix = |v: isize| -> usize { v.clamp(0, d as isize - 1) as usize };
        let xi = x as isize;
        let yi = y as isize;
        let gx = (self.cur[y * d + ix(xi + 1)] - self.cur[y * d + ix(xi - 1)]) * 0.5;
        let gz = (self.cur[ix(yi + 1) * d + x] - self.cur[ix(yi - 1) * d + x]) * 0.5;
        [gx, gz]
    }

    /// 高度场 Laplacian(解析焦散强度来源:∇²h 即折射会聚/发散度)。
    pub fn laplacian(&self, x: usize, y: usize) -> f32 {
        let d = self.dim;
        let ix = |v: isize| -> usize { v.clamp(0, d as isize - 1) as usize };
        let xi = x as isize;
        let yi = y as isize;
        let c = self.cur[y * d + x];
        self.cur[y * d + ix(xi - 1)]
            + self.cur[y * d + ix(xi + 1)]
            + self.cur[ix(yi - 1) * d + x]
            + self.cur[ix(yi + 1) * d + x]
            - 4.0 * c
    }

    /// 高度场总能量(|u| 求和;传播/衰减断言用)。
    pub fn energy(&self) -> f32 {
        self.cur.iter().map(|v| v.abs()).sum()
    }
}

/// 海绵层能量保留率(host / device 逐字同源的纯函数)。
///
/// 到边距离 < 海绵层厚度时按 smoothstep 抬升吸收量;网格内部恒为 `1 − damping`。
pub fn sponge_retention(x: usize, y: usize, dim: usize, damping: f32, sponge_frac: f32) -> f32 {
    let n = dim as f32;
    let thickness = (n * sponge_frac).max(1.0);
    let dx = (x as f32).min(n - 1.0 - x as f32);
    let dy = (y as f32).min(n - 1.0 - y as f32);
    let ex = 1.0 - smoothstep(0.0, thickness, dx);
    let ey = 1.0 - smoothstep(0.0, thickness, dy);
    // 两轴吸收合成(角落吸收最强),再平方软化过渡。
    let absorb = 1.0 - (1.0 - ex) * (1.0 - ey);
    let keep = (1.0 - absorb * absorb).clamp(0.0, 1.0);
    (1.0 - damping) * keep
}

// ---------------------------------------------------------------------------
// 层 2:场景(host 金标准 ↔ g41_water_scene.rx)
// ---------------------------------------------------------------------------

/// 解析礁石(球)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rock {
    /// 球心(世界)。
    pub center: [f32; 3],
    /// 半径(米)。
    pub radius: f32,
    /// 反照率。
    pub albedo: [f32; 3],
    /// 粗糙度。
    pub roughness: f32,
}

/// 解析泻湖场景(高度场海床碗盆 + 球形礁石 + 水平面)。
///
/// 设计意图:海床自中心的 `max_depth_m` 深处向外抬升,越过 `shore_radius_m`
/// 后高于水面形成岸线。于是同一帧内同时呈现**深水(强吸收/散射)→ 浅水
/// (清澈、焦散可见)→ 干岸**的完整梯度——这是水体渲染最有信息量的构图。
#[derive(Debug, Clone, PartialEq)]
pub struct LagoonScene {
    /// 泻湖中心 X。
    pub center_x: f32,
    /// 泻湖中心 Z。
    pub center_z: f32,
    /// 水面高度(世界 Y)。
    pub water_level: f32,
    /// 碗盆中心最大水深(米,正值)。
    pub max_depth_m: f32,
    /// 岸线半径(米;此处海床恰达水面)。
    pub shore_radius_m: f32,
    /// 岸外抬升高度(米)。
    pub beach_rise_m: f32,
    /// 海床沙丘细节振幅(米)。
    pub dune_amplitude_m: f32,
    /// 海床沙丘细节空间频率(1/米)。
    pub dune_frequency: f32,
    /// 沙地反照率。
    pub sand_albedo: [f32; 3],
    /// 岸上干沙/草地反照率。
    pub shore_albedo: [f32; 3],
    /// 波网格覆盖的世界见方边长(米)。
    pub wave_extent_m: f32,
    /// 礁石表。
    pub rocks: Vec<Rock>,
}

impl Default for LagoonScene {
    fn default() -> Self {
        Self {
            center_x: 0.0,
            center_z: 0.0,
            water_level: 0.0,
            max_depth_m: 6.0,
            shore_radius_m: 34.0,
            beach_rise_m: 3.2,
            dune_amplitude_m: 0.22,
            dune_frequency: 0.09,
            // 湿沙(水下)比干沙暗:实测反照率约 0.2~0.35 vs 干沙 0.4~0.6。
            sand_albedo: [0.40, 0.35, 0.26],
            shore_albedo: [0.58, 0.53, 0.42],
            wave_extent_m: 96.0,
            rocks: vec![
                Rock {
                    center: [-14.0, -1.6, 9.0],
                    radius: 3.1,
                    albedo: [0.30, 0.29, 0.28],
                    roughness: 0.82,
                },
                Rock {
                    center: [11.0, -2.4, -13.0],
                    radius: 4.0,
                    albedo: [0.26, 0.25, 0.24],
                    roughness: 0.86,
                },
                Rock {
                    center: [24.0, -0.4, 6.0],
                    radius: 2.4,
                    albedo: [0.33, 0.31, 0.28],
                    roughness: 0.80,
                },
                Rock {
                    center: [-3.0, -4.3, -22.0],
                    radius: 3.6,
                    albedo: [0.24, 0.24, 0.26],
                    roughness: 0.88,
                },
                Rock {
                    center: [-27.0, 0.6, -6.0],
                    radius: 2.9,
                    albedo: [0.35, 0.33, 0.30],
                    roughness: 0.78,
                },
                Rock {
                    center: [6.0, -3.9, 20.0],
                    radius: 2.2,
                    albedo: [0.28, 0.27, 0.26],
                    roughness: 0.84,
                },
            ],
        }
    }
}

impl LagoonScene {
    /// 域校验。
    pub fn validate(&self) -> Result<()> {
        if self.rocks.len() > MAX_ROCKS {
            return Err(WaterSurfaceError::TooManyRocks {
                got: self.rocks.len(),
                max: MAX_ROCKS,
            });
        }
        for v in [
            self.max_depth_m,
            self.shore_radius_m,
            self.beach_rise_m,
            self.wave_extent_m,
        ] {
            if !v.is_finite() || v <= 0.0 {
                return Err(WaterSurfaceError::NotFinite("scene geometry"));
            }
        }
        Ok(())
    }

    /// 海床高度场 `y = floor(x, z)`(host / device 逐字同源)。
    ///
    /// 碗盆项取 `cos` 抬升廓形:中心 `−max_depth`,`shore_radius` 处恰为
    /// `water_level`,其后线性抬升到岸。沙丘项为两组正交正弦的低幅叠加
    /// (确定性、无噪声表依赖,device 端同式内联)。
    pub fn floor_height(&self, x: f32, z: f32) -> f32 {
        let dx = x - self.center_x;
        let dz = z - self.center_z;
        let r = (dx * dx + dz * dz).sqrt();
        let rn = (r / self.shore_radius_m).min(1.0);
        // 碗盆:rn=0 → −max_depth;rn=1 → 0(恰达水面)。半余弦廓形,岸线处切向平滑。
        let bowl = -self.max_depth_m * 0.5 * (1.0 + (std::f32::consts::PI * rn).cos());
        // 岸外抬升(rn ≥ 1 之后按到岸距离线性升起)。
        let beyond = (r - self.shore_radius_m).max(0.0);
        let rise = self.beach_rise_m * (beyond / self.shore_radius_m.max(1.0)).min(1.0);
        let f = self.dune_frequency;
        let dune = self.dune_amplitude_m
            * ((f * x).sin() * (f * 0.87 * z).cos() + 0.45 * (f * 2.3 * x + f * 1.7 * z).sin());
        self.water_level + bowl + rise + dune
    }

    /// 海床反照率(沙 → 岸的高度混合;host / device 同源)。
    pub fn floor_albedo(&self, x: f32, z: f32) -> [f32; 3] {
        let h = self.floor_height(x, z);
        // 水面上方 0.6 m 内由湿沙过渡到干岸。
        let t = smoothstep(self.water_level - 0.15, self.water_level + 0.6, h);
        let mut out = [0.0f32; 3];
        for (o, (sand, shore)) in out
            .iter_mut()
            .zip(self.sand_albedo.iter().zip(self.shore_albedo.iter()))
        {
            *o = sand + (shore - sand) * t;
        }
        out
    }

    /// 某点的水深(水面 − 海床;≤ 0 表示该处为干地)。
    pub fn water_depth(&self, x: f32, z: f32) -> f32 {
        self.water_level - self.floor_height(x, z)
    }

    /// 礁石参数打平(不足 [`MAX_ROCKS`] 的槽位半径填 0 ⇒ device 端算术门恒不命中)。
    pub fn pack_rocks(&self) -> Vec<f32> {
        let mut out = vec![0.0f32; MAX_ROCKS * ROCK_STRIDE];
        for (i, rk) in self.rocks.iter().take(MAX_ROCKS).enumerate() {
            let b = i * ROCK_STRIDE;
            out[b] = rk.center[0];
            out[b + 1] = rk.center[1];
            out[b + 2] = rk.center[2];
            out[b + 3] = rk.radius;
            out[b + 4] = rk.albedo[0];
            out[b + 5] = rk.albedo[1];
            out[b + 6] = rk.albedo[2];
            out[b + 7] = rk.roughness;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// 层 4:水体参数与着色公式(host 金标准 ↔ g41_water_surface.rx)
// ---------------------------------------------------------------------------

/// 水体外观与算法参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterParams {
    /// 吸收系数 σa(1/米;RGB。水对长波吸收强 ⇒ R 分量最大 ⇒ 深处偏青蓝)。
    pub absorption: [f32; 3],
    /// 散射系数 σs(1/米;RGB)。
    pub scattering: [f32; 3],
    /// 水面基础粗糙度。
    pub roughness: f32,
    /// 法线细节强度(波梯度之外的高频扰动)。
    pub normal_detail: f32,
    /// 折射法线增益(HPWater `_RefractionStrength`:只放大水平分量)。
    pub refraction_strength: f32,
    /// 折射光追最大穿越距离(米)。
    pub max_refraction_cross_m: f32,
    /// 体积散射最大穿越距离(米)。
    pub max_cross_m: f32,
    /// 折射光追采样数。
    pub refract_steps: u32,
    /// 折射厚度阈值(米;防自遮挡)。
    pub refract_thickness_m: f32,
    /// Henyey-Greenstein 相位各向异性 g。
    pub phase_g: f32,
    /// 三波长色散强度。
    pub dispersion: f32,
    /// 散射密度 → 模糊级的权重。
    pub scatter_blur_density: f32,
    /// 泡沫阈值(波高超过此值起泡)。
    pub foam_threshold: f32,
    /// 泡沫强度。
    pub foam_intensity: f32,
    /// 岸线泡沫宽度(米;水深小于此值起浪花)。
    pub shore_foam_depth_m: f32,
    /// 焦散强度(0 = 关)。
    pub caustic_intensity: f32,
    /// 焦散会聚系数(∇²h → 面积压缩的比例常数)。
    pub caustic_convergence: f32,
    /// 波高 → 世界位移的米制缩放。
    pub wave_amplitude_m: f32,
    /// 环境反射强度。
    pub reflection_strength: f32,
}

impl Default for WaterParams {
    fn default() -> Self {
        Self {
            // 清澈热带海水量级:红端吸收 ~0.35/m,蓝端 ~0.02/m。
            absorption: [0.42, 0.09, 0.045],
            scattering: [0.045, 0.11, 0.15],
            roughness: 0.055,
            normal_detail: 0.55,
            refraction_strength: 2.4,
            max_refraction_cross_m: 18.0,
            max_cross_m: 24.0,
            refract_steps: 10,
            refract_thickness_m: 0.45,
            phase_g: 0.72,
            dispersion: 0.09,
            scatter_blur_density: 0.5,
            foam_threshold: 0.80,
            foam_intensity: 0.55,
            shore_foam_depth_m: 0.22,
            caustic_intensity: 1.35,
            caustic_convergence: 2.2,
            wave_amplitude_m: 0.30,
            reflection_strength: 1.0,
        }
    }
}

/// 相机(device 参数打包用;右手 Y-up,基向量须正交单位)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterCamera {
    /// 相机世界位置。
    pub origin: [f32; 3],
    /// 前向单位向量。
    pub forward: [f32; 3],
    /// 右向单位向量。
    pub right: [f32; 3],
    /// 上向单位向量。
    pub up: [f32; 3],
    /// 垂直视场半角正切。
    pub tan_half_fov: f32,
    /// 宽高比。
    pub aspect: f32,
}

// ── 公式面(每个函数 = 对应 kernel 内联段的逐字同源事实源)─────────────────

/// smoothstep(host / device 同式)。
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-8)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Schlick 菲涅尔(标量 F0)。
pub fn fresnel_schlick(cos_theta: f32, f0: f32) -> f32 {
    let m = (1.0 - cos_theta).clamp(0.0, 1.0);
    let m2 = m * m;
    f0 + (1.0 - f0) * (m2 * m2 * m)
}

/// Henyey-Greenstein 相位函数(**含 1/4π 归一化**,∫p·dω = 1)。
///
/// 归一化是必须的:本模块的 [`rayleigh_phase`] 取的是标准归一化式
/// `3(1+cos²θ)/16π`,若 HG 侧漏掉 1/4π,两项就不同量纲,合成相位会整体偏大
/// 约 4π ≈ 12.6 倍——实测表现为水体内散射过亮、深水反而比浅水更亮的"发光
/// 牛奶池"(首版教训,如实登记)。
pub fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    (1.0 - g2) / (4.0 * std::f32::consts::PI * denom.abs().max(1e-6).powf(1.5))
}

/// 瑞利相位。
pub fn rayleigh_phase(cos_theta: f32) -> f32 {
    (1.0 + cos_theta * cos_theta) * (3.0 / (16.0 * std::f32::consts::PI))
}

/// 水体散射相位:瑞利 5% + 米氏(HG)95%,按波长加权。
///
/// 瑞利项的 `1/λ⁴` 波长依赖是水体"深处发蓝"的成因之一(另一成因是红端吸收)。
pub fn scatter_phase(cos_theta: f32, g: f32) -> [f32; 3] {
    // β_R ∝ 1/λ⁴(λ = 680 / 550 / 440 nm),归一到绿通道为 1。
    const BETA_R: [f32; 3] = [0.418, 1.0, 2.452];
    let r = rayleigh_phase(cos_theta);
    let m = henyey_greenstein(cos_theta, g);
    let mut out = [0.0f32; 3];
    for i in 0..3 {
        out[i] = BETA_R[i] * r * 0.05 + m * 0.95;
    }
    out
}

/// 单段介质的散射光与透射率(Beer-Lambert + 单次散射反照率)。
///
/// ```text
/// σt = σa + σs
/// T  = exp(−σt·d)
/// L  = L₀·(1 − T)·(σs/σt)·phase
/// ```
///
/// 返回 `(散射光, 透射率)`。能量守恒断言:`T + 吸收份额 + 散射份额 = 1`。
pub fn scattered_light(
    incoming: [f32; 3],
    sigma_a: [f32; 3],
    sigma_s: [f32; 3],
    distance: f32,
    phase: [f32; 3],
) -> ([f32; 3], [f32; 3]) {
    let mut scattered = [0.0f32; 3];
    let mut transmittance = [0.0f32; 3];
    for i in 0..3 {
        let sigma_t = sigma_a[i] + sigma_s[i];
        let t = (-sigma_t * distance.max(0.0)).exp();
        transmittance[i] = t;
        let albedo = if sigma_t > 1e-8 {
            sigma_s[i] / sigma_t
        } else {
            0.0
        };
        scattered[i] = incoming[i] * (1.0 - t) * albedo * phase[i];
    }
    (scattered, transmittance)
}

/// 自适应指数步进因子(距离远于参考距离时步进更"指数",近处趋近线性)。
pub fn adaptive_exp_factor(distance: f32, reference: f32) -> f32 {
    let ratio = distance.max(0.01) / reference.max(1e-6);
    (ratio * ratio).clamp(1.01, 32.0)
}

/// 指数步进归一化位置 `d(t) = (F^t − 1)/(F − 1)`,`t ∈ [0,1]`。
///
/// 性质(单测锚定):`d(0) = 0`、`d(1) = 1`、在 `F > 1` 时严格单调递增且凸——
/// 近处采样密、远处稀,正是折射/体积步进需要的分布。
pub fn exp_step_position(t: f32, exp_factor: f32) -> f32 {
    let f = exp_factor.max(1.000_001);
    (f.powf(t) - 1.0) / (f - 1.0)
}

/// Interleaved Gradient Noise(逐像素逐帧抖动;解析式、无 RNG 状态)。
pub fn interleaved_gradient_noise(px: f32, py: f32, frame: u32) -> f32 {
    let x = px + (frame as f32) * 5.588_238;
    let y = py + (frame as f32) * 5.588_238;
    let m = 0.067_110_56 * x + 0.005_837_15 * y;
    (52.982_918 * (m - m.floor())).fract()
}

/// 散射密度 → 模糊级(HPWater `CalculateHPWaterMipLevel` 同一构造)。
///
/// 执行面无硬件 mip 链,故本级号用于在 3 级 box 降采样链之间做线性混合。
pub fn blur_level(depth_m: f32, scaling: f32, scatter_density: f32, max_level: f32) -> f32 {
    let effective = scaling + scatter_density * 10.0;
    (1.0 + depth_m.max(0.0) * effective)
        .log2()
        .clamp(0.0, max_level)
}

/// GGX 法线分布 × Smith 可见性的联合项(直接光高光)。
pub fn ggx_specular(n_dot_h: f32, n_dot_l: f32, n_dot_v: f32, roughness: f32) -> f32 {
    let a = (roughness * roughness).max(1e-4);
    let a2 = a * a;
    let d_denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    let d = a2 / (std::f32::consts::PI * (d_denom * d_denom).max(1e-8));
    let k = a * 0.5;
    let g_v = n_dot_v / (n_dot_v * (1.0 - k) + k).max(1e-6);
    let g_l = n_dot_l / (n_dot_l * (1.0 - k) + k).max(1e-6);
    d * g_v * g_l / (4.0 * n_dot_v * n_dot_l).max(1e-6)
}

/// 解析焦散强度。
///
/// 水面局部曲率把折射光束会聚/发散:面积压缩比 ≈ `1 + D·k·∇²h`(D 为水深,
/// `∇²h` 为水面高度场 Laplacian),强度与之成反比。较之 HPWater 的光子步进 +
/// `InterlockedAdd` 累积,本式是同一物理量的解析闭式估计——无原子、无额外
/// pass、逐像素确定性,代价是不含多次折射与全反射焦散。
pub fn caustic_gain(laplacian_h: f32, depth_m: f32, convergence: f32, intensity: f32) -> f32 {
    let compression = 1.0 + depth_m.max(0.0) * convergence * laplacian_h;
    let gain = 1.0 / compression.max(0.12);
    // intensity = 0 时恒等于 1(关臂 ⇒ 乘性中性)。
    1.0 + (gain - 1.0) * intensity
}

/// 折射方向(HPWater 的"只保留法线扰动分量"构造)。
///
/// 直接用 `refract(V, N, η)` 会让整个水面产生一个全局位移(平静水面也整体偏折),
/// 与"平静水面应当看到未变形的水下场景"的期望不符。故减去平面法线的折射基准、
/// 再加回原视线方向:得到**只由波纹扰动引起**的折射偏移。
pub fn refraction_direction(view_dir: [f32; 3], normal: [f32; 3], eta: f32, gain: f32) -> [f32; 3] {
    let n_gain = normalize3([normal[0] * gain, normal[1], normal[2] * gain]);
    let bent = refract3(view_dir, n_gain, eta);
    let flat = refract3(view_dir, [0.0, 1.0, 0.0], eta);
    normalize3([
        bent[0] - flat[0] + view_dir[0],
        bent[1] - flat[1] + view_dir[1],
        bent[2] - flat[2] + view_dir[2],
    ])
}

/// 折射向量(GLSL `refract` 语义;全反射时返回零向量)。
pub fn refract3(incident: [f32; 3], normal: [f32; 3], eta: f32) -> [f32; 3] {
    let d = dot3(normal, incident);
    let k = 1.0 - eta * eta * (1.0 - d * d);
    if k < 0.0 {
        return [0.0; 3];
    }
    let s = eta * d + k.sqrt();
    [
        eta * incident[0] - s * normal[0],
        eta * incident[1] - s * normal[1],
        eta * incident[2] - s * normal[2],
    ]
}

/// 三维点积。
pub fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// 三维归一化(零向量安全:退化为 +Y)。
pub fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = dot3(v, v).sqrt();
    if l < 1e-6 {
        [0.0, 1.0, 0.0]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

// ---------------------------------------------------------------------------
// 烘焙:2D 值噪声(海床细节 + 水面法线细节;host 烘焙 → device 双线性查表)
// ---------------------------------------------------------------------------

/// 烘焙环绕 2D 值噪声(三倍频;边长 = [`NOISE2D_DIM`],行主序)。
///
/// 与 G40 同律:device 端不现算 hash 噪声,由 host 一次烘焙上传,使
/// host/device 对拍容差里不含噪声项。
pub fn bake_noise2d(seed: u64) -> Vec<f32> {
    let n = NOISE2D_DIM;
    let hash = |x: i64, y: i64, s: u64| -> f32 {
        let mut h = (x as u64)
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add((y as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f))
            ^ s;
        h ^= h >> 29;
        h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        h ^= h >> 32;
        (h >> 40) as f32 / 16_777_216.0
    };
    let vnoise = |u: f32, v: f32, period: i64, s: u64| -> f32 {
        let fx = u * period as f32;
        let fy = v * period as f32;
        let x0 = fx.floor();
        let y0 = fy.floor();
        let tx = smoothstep(0.0, 1.0, fx - x0);
        let ty = smoothstep(0.0, 1.0, fy - y0);
        let wrap = |i: f32| -> i64 { ((i as i64 % period) + period) % period };
        let (ix0, iy0) = (wrap(x0), wrap(y0));
        let (ix1, iy1) = (wrap(x0 + 1.0), wrap(y0 + 1.0));
        let c00 = hash(ix0, iy0, s);
        let c10 = hash(ix1, iy0, s);
        let c01 = hash(ix0, iy1, s);
        let c11 = hash(ix1, iy1, s);
        let a = c00 + (c10 - c00) * tx;
        let b = c01 + (c11 - c01) * tx;
        a + (b - a) * ty
    };
    let mut out = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let u = x as f32 / n as f32;
            let v = y as f32 / n as f32;
            let f = vnoise(u, v, 8, seed) * 0.55
                + vnoise(u, v, 16, seed ^ 0x11) * 0.30
                + vnoise(u, v, 32, seed ^ 0x22) * 0.15;
            out.push(f);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 参数打包(device 参数面唯一事实源)
// ---------------------------------------------------------------------------

/// 打包水面着色参数面(长度恒 [`WATER_PARAM_COUNT`])。
///
/// 布局与 `kernels/g41_water_scene.rx` / `g41_water_surface.rx` 头注逐字同源:
///
/// ```text
/// [0]=width [1]=height
/// [2..5]=cam_origin [5..8]=cam_forward [8..11]=cam_right [11..14]=cam_up
/// [14]=tan_half_fov [15]=aspect
/// [16..19]=sun_dir [19..22]=sun_color [22..25]=ambient_top [25..28]=ambient_bottom
/// [28]=water_level [29]=center_x [30]=center_z [31]=max_depth
/// [32]=shore_radius [33]=beach_rise [34]=dune_amp [35]=dune_freq
/// [36..39]=sand_albedo [39..42]=shore_albedo
/// [42]=wave_extent [43]=wave_dim [44]=wave_amplitude
/// [45..48]=absorption [48..51]=scattering
/// [51]=roughness [52]=normal_detail [53]=refraction_strength
/// [54]=max_refraction_cross [55]=max_cross [56]=refract_steps
/// [57]=refract_thickness [58]=phase_g [59]=dispersion [60]=scatter_blur_density
/// [61]=foam_threshold [62]=foam_intensity [63]=shore_foam_depth
/// [64]=caustic_intensity [65]=caustic_convergence [66]=reflection_strength
/// [67]=eta [68]=f0 [69]=exp_factor [70]=refraction_reference
/// [71]=frame_index [72]=noise2d_dim [73]=sky_lut_w [74]=sky_lut_h
/// [75]=scene_march_steps [76]=scene_max_dist [77]=blur_levels
/// [78]=water_sample_count [79]=exposure
/// [80]=rock_count [81]=time_s [82]=caustic_enabled [83]=refract_enabled
/// [84]=dispersion_enabled [85]=foam_enabled [86]=volume_enabled
/// [87]=reflect_enabled [88]=debug_view [89]=water_enabled(主开关)
/// [90..96]=reserved(恒 0)
/// [96..96+MAX_ROCKS*8]=rocks(cx,cy,cz,r,ar,ag,ab,rough)
/// ```
#[allow(clippy::too_many_arguments)]
pub fn pack_water_params(
    params: &WaterParams,
    scene: &LagoonScene,
    sky: &Sky,
    cam: &WaterCamera,
    width: u32,
    height: u32,
    frame_index: u32,
    time_s: f32,
    exposure: f32,
    arms: WaterArms,
    scene_march_steps: u32,
    scene_max_dist: f32,
    sky_lut_w: usize,
    sky_lut_h: usize,
    debug_view: u32,
) -> Vec<f32> {
    let mut p = vec![0.0f32; WATER_PARAM_COUNT];
    let sun_dir = sky.sun_direction();
    let sun_color = sky.sun_color();
    let (amb_top, amb_bottom) = sky.ambient_probe();

    p[0] = width as f32;
    p[1] = height as f32;
    p[2..5].copy_from_slice(&cam.origin);
    p[5..8].copy_from_slice(&cam.forward);
    p[8..11].copy_from_slice(&cam.right);
    p[11..14].copy_from_slice(&cam.up);
    p[14] = cam.tan_half_fov;
    p[15] = cam.aspect;
    p[16..19].copy_from_slice(&sun_dir);
    p[19..22].copy_from_slice(&sun_color);
    p[22..25].copy_from_slice(&amb_top);
    p[25..28].copy_from_slice(&amb_bottom);
    p[28] = scene.water_level;
    p[29] = scene.center_x;
    p[30] = scene.center_z;
    p[31] = scene.max_depth_m;
    p[32] = scene.shore_radius_m;
    p[33] = scene.beach_rise_m;
    p[34] = scene.dune_amplitude_m;
    p[35] = scene.dune_frequency;
    p[36..39].copy_from_slice(&scene.sand_albedo);
    p[39..42].copy_from_slice(&scene.shore_albedo);
    p[42] = scene.wave_extent_m;
    p[43] = WAVE_DIM as f32;
    p[44] = params.wave_amplitude_m;
    p[45..48].copy_from_slice(&params.absorption);
    p[48..51].copy_from_slice(&params.scattering);
    p[51] = params.roughness;
    p[52] = params.normal_detail;
    p[53] = params.refraction_strength;
    p[54] = params.max_refraction_cross_m;
    p[55] = params.max_cross_m;
    p[56] = params.refract_steps as f32;
    p[57] = params.refract_thickness_m;
    p[58] = params.phase_g;
    p[59] = params.dispersion;
    p[60] = params.scatter_blur_density;
    p[61] = params.foam_threshold;
    p[62] = params.foam_intensity;
    p[63] = params.shore_foam_depth_m;
    p[64] = params.caustic_intensity;
    p[65] = params.caustic_convergence;
    p[66] = params.reflection_strength;
    p[67] = AIR_TO_WATER_ETA;
    p[68] = WATER_F0;
    p[69] = EXP_FACTOR;
    p[70] = REFRACTION_REFERENCE_DISTANCE;
    p[71] = frame_index as f32;
    p[72] = NOISE2D_DIM as f32;
    p[73] = sky_lut_w as f32;
    p[74] = sky_lut_h as f32;
    p[75] = scene_march_steps as f32;
    p[76] = scene_max_dist;
    p[77] = BLUR_LEVELS as f32;
    p[78] = WATER_SAMPLE_COUNT as f32;
    p[79] = exposure;
    p[80] = scene.rocks.len().min(MAX_ROCKS) as f32;
    p[81] = time_s;
    p[82] = f32::from(u8::from(arms.caustics));
    p[83] = f32::from(u8::from(arms.refraction));
    p[84] = f32::from(u8::from(arms.dispersion));
    p[85] = f32::from(u8::from(arms.foam));
    p[86] = f32::from(u8::from(arms.volume));
    p[87] = f32::from(u8::from(arms.reflection));
    p[88] = debug_view as f32;
    // 主开关:任一特性开 ⇒ 水面参与合成;全关 ⇒ 水面 pass 直通场景色
    // (`--water off` 的语义 = 完全没有水,而非"有水但特性全关")。
    p[89] = f32::from(u8::from(arms.any()));
    // [90..96) 预留恒 0。
    p[96..].copy_from_slice(&scene.pack_rocks());
    p
}

/// 打包波方程参数面(长度恒 [`WAVE_PARAM_COUNT`];`kernels/g41_water_wave.rx`
/// 头注逐字同源)。
///
/// ```text
/// [0]=dim [1]=c2dt2 [2]=damping [3]=sponge_frac [4]=drop_count
/// [8+i*4 .. +4] = 第 i 滴 (u, v, intensity, radius),i ∈ 0..WAVE_MAX_DROPS
/// ```
///
/// 超过 [`WAVE_MAX_DROPS`] 的同帧波源按脚本序截断(如实截断,不静默合并)。
pub fn pack_wave_params(dim: usize, params: &WaveParams, drops: &[WaveDrop]) -> Vec<f32> {
    let mut p = vec![0.0f32; WAVE_PARAM_COUNT];
    p[0] = dim as f32;
    p[1] = params.speed * params.speed * params.dt * params.dt;
    p[2] = params.damping;
    p[3] = params.sponge_frac;
    let n = drops.len().min(WAVE_MAX_DROPS);
    p[4] = n as f32;
    for (i, d) in drops.iter().take(n).enumerate() {
        let b = 8 + i * 4;
        p[b] = d.u;
        p[b + 1] = d.v;
        p[b + 2] = d.intensity;
        p[b + 3] = d.radius;
    }
    p
}

/// 由场景产障碍掩码字节面(1.0 = 陆地/礁石,0.0 = 水;device 上传用)。
pub fn bake_obstacle_field(dim: usize, scene: &LagoonScene) -> Vec<f32> {
    let mut out = vec![0.0f32; dim * dim];
    let n = dim as f32;
    for y in 0..dim {
        for x in 0..dim {
            let u = (x as f32 + 0.5) / n;
            let v = (y as f32 + 0.5) / n;
            let wx = scene.center_x + (u - 0.5) * scene.wave_extent_m;
            let wz = scene.center_z + (v - 0.5) * scene.wave_extent_m;
            out[y * dim + x] = if scene.floor_height(wx, wz) >= scene.water_level {
                1.0
            } else {
                0.0
            };
        }
    }
    out
}

/// 水面特性开关闭集(每一项默认开;`off` 臂用于 A/B 归因与单特性出图)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterArms {
    /// 屏幕空间折射光追。
    pub refraction: bool,
    /// Beer-Lambert 体积吸收/散射。
    pub volume: bool,
    /// 解析焦散。
    pub caustics: bool,
    /// 三波长色散。
    pub dispersion: bool,
    /// 泡沫(波峰 + 岸线)。
    pub foam: bool,
    /// 环境反射。
    pub reflection: bool,
}

impl Default for WaterArms {
    fn default() -> Self {
        Self {
            refraction: true,
            volume: true,
            caustics: true,
            dispersion: true,
            foam: true,
            reflection: true,
        }
    }
}

impl WaterArms {
    /// 是否至少有一项开启(水面主开关;全关 ⇒ 水面 pass 直通场景色)。
    pub fn any(&self) -> bool {
        self.refraction
            || self.volume
            || self.caustics
            || self.dispersion
            || self.foam
            || self.reflection
    }

    /// 全关(纯几何基线;A/B 归因的 off 锚)。
    pub fn all_off() -> Self {
        Self {
            refraction: false,
            volume: false,
            caustics: false,
            dispersion: false,
            foam: false,
            reflection: false,
        }
    }
}

/// 解析波源脚本 `"帧:u,v,强度[,半径];…"`(确定性、可复现的交互替身)。
pub fn parse_drop_script(s: &str) -> Result<Vec<WaveDrop>> {
    let mut out = Vec::new();
    for item in s.split(';').map(str::trim).filter(|t| !t.is_empty()) {
        let (frame_s, rest) = item
            .split_once(':')
            .ok_or(WaterSurfaceError::BadDropScript {
                why: "缺 `帧:` 前缀",
            })?;
        let frame: u32 = frame_s
            .trim()
            .parse()
            .map_err(|_| WaterSurfaceError::BadDropScript {
                why: "帧号非法"
            })?;
        let f: Vec<f32> = rest
            .split(',')
            .map(|t| t.trim().parse::<f32>())
            .collect::<std::result::Result<_, _>>()
            .map_err(|_| WaterSurfaceError::BadDropScript {
                why: "分量非法"
            })?;
        if f.len() < 3 || f.len() > 4 {
            return Err(WaterSurfaceError::BadDropScript {
                why: "分量数须为 3(u,v,I)或 4(u,v,I,r)",
            });
        }
        if !(0.0..=1.0).contains(&f[0]) || !(0.0..=1.0).contains(&f[1]) {
            return Err(WaterSurfaceError::BadDropScript {
                why: "u/v 须 ∈ [0,1]",
            });
        }
        out.push(WaveDrop {
            frame,
            u: f[0],
            v: f[1],
            intensity: f[2],
            radius: if f.len() == 4 { f[3] } else { 6.0 },
        });
    }
    Ok(out)
}

/// canonical 波源脚本(golden 场景:三滴不同时刻不同位置的雨点)。
pub fn canonical_drops() -> Vec<WaveDrop> {
    vec![
        WaveDrop {
            frame: 0,
            u: 0.50,
            v: 0.50,
            intensity: 0.55,
            radius: 7.0,
        },
        WaveDrop {
            frame: 24,
            u: 0.34,
            v: 0.62,
            intensity: 0.40,
            radius: 5.0,
        },
        WaveDrop {
            frame: 52,
            u: 0.66,
            v: 0.38,
            intensity: 0.48,
            radius: 6.0,
        },
    ]
}

/// 波场 digest(双跑位级一致断言用)。
pub fn wave_digest(sim: &WaveSim) -> [u8; 32] {
    let mut buf = Vec::with_capacity(sim.cur.len() * 8);
    for v in &sim.cur {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    for v in &sim.prev {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    rurix_pkg::sha256::digest(&buf)
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn wave_grid_domain_and_cfl_red() {
        // 非 2 的幂 ⇒ 拒录。
        assert!(matches!(
            WaveSim::new(100, WaveParams::default()),
            Err(WaterSurfaceError::GridNotPowerOfTwo { got: 100 })
        ));
        // 越 CFL 界 ⇒ 拒录(c·dt = 1.0 ≥ 0.7071)。
        let bad = WaveParams {
            speed: 1.0,
            dt: 1.0,
            ..WaveParams::default()
        };
        assert!(matches!(
            WaveSim::new(64, bad),
            Err(WaterSurfaceError::CflViolation { .. })
        ));
        // 界内合法。
        assert!(WaveSim::new(64, WaveParams::default()).is_ok());
    }

    #[test]
    fn wave_pulse_propagates_and_is_bounded() {
        let mut sim = WaveSim::new(64, WaveParams::default()).unwrap();
        sim.poke(0.5, 0.5, 1.0, 5.0).unwrap();
        let e0 = sim.energy();
        assert!(e0 > 0.0, "注入后能量须为正");
        // 中心脉冲应向外传播:若干步后远离中心处出现非零位移。
        for _ in 0..14 {
            sim.step();
        }
        let d = sim.dim();
        let far = sim.height()[(d / 2) * d + (d / 2 + 11)];
        assert!(far.abs() > 1e-6, "脉冲未传播到远处: {far}");
        // 有界:阻尼 + 海绵层下不得发散。
        assert!(
            sim.height().iter().all(|v| v.is_finite() && v.abs() < 50.0),
            "波场发散"
        );
    }

    #[test]
    fn wave_sponge_absorbs_at_border() {
        // 边缘保留率必须低于中心(海绵层生效)。
        let dim = 64;
        let center = sponge_retention(dim / 2, dim / 2, dim, 0.004, 0.10);
        let border = sponge_retention(0, dim / 2, dim, 0.004, 0.10);
        assert!(
            border < center,
            "海绵层未吸收: border={border} center={center}"
        );
        assert!(approx(center, 0.996, 1e-5), "内部保留率须为 1−damping");
    }

    #[test]
    fn wave_obstacle_is_reflective_and_zeroed() {
        let mut sim = WaveSim::new(32, WaveParams::default()).unwrap();
        // 右半场设为障碍。
        for y in 0..32 {
            for x in 16..32 {
                sim.obstacle[y * 32 + x] = true;
            }
        }
        sim.poke(0.25, 0.5, 1.0, 3.0).unwrap();
        for _ in 0..10 {
            sim.step();
        }
        // 障碍区高度恒 0。
        for y in 0..32 {
            for x in 16..32 {
                assert_eq!(sim.height()[y * 32 + x], 0.0, "障碍格须恒 0");
            }
        }
    }

    #[test]
    fn wave_double_run_bit_equal() {
        let build = || {
            let mut s = WaveSim::new(64, WaveParams::default()).unwrap();
            let drops = canonical_drops();
            for _ in 0..60 {
                s.step_with_drops(&drops).unwrap();
            }
            s
        };
        assert_eq!(
            wave_digest(&build()),
            wave_digest(&build()),
            "双跑须位级一致"
        );
    }

    #[test]
    fn wave_drop_out_of_bounds_red() {
        let mut sim = WaveSim::new(32, WaveParams::default()).unwrap();
        assert!(sim.poke(0.5, 0.5, 1.0, 3.0).is_ok());
        assert!(matches!(
            sim.poke(1.5, 0.5, 1.0, 3.0),
            Err(WaterSurfaceError::DropOutOfBounds { .. })
        ));
        assert!(matches!(
            sim.poke(0.5, -0.2, 1.0, 3.0),
            Err(WaterSurfaceError::DropOutOfBounds { .. })
        ));
    }

    #[test]
    fn beer_lambert_energy_conservation() {
        // 白炉:透射 + 吸收份额 + 散射份额 ≡ 1(逐通道)。
        let sa = [0.42, 0.09, 0.045];
        let ss = [0.020, 0.045, 0.060];
        for d in [0.0f32, 0.5, 3.0, 12.0, 40.0] {
            let (scat, trans) = scattered_light([1.0; 3], sa, ss, d, [1.0; 3]);
            for i in 0..3 {
                let sigma_t = sa[i] + ss[i];
                let absorbed = (1.0 - trans[i]) * (sa[i] / sigma_t);
                let total = trans[i] + absorbed + scat[i];
                assert!(approx(total, 1.0, 1e-5), "能量不守恒 ch{i} d={d}: {total}");
            }
        }
    }

    #[test]
    fn beer_lambert_depth_makes_water_blue() {
        // 红端吸收远强于蓝端 ⇒ 深处透射率红 << 蓝(水显青蓝的物理成因)。
        let p = WaterParams::default();
        let (_, t) = scattered_light([1.0; 3], p.absorption, p.scattering, 8.0, [1.0; 3]);
        assert!(t[0] < t[1] && t[1] < t[2], "透射率须 R<G<B,得到 {t:?}");
        assert!(t[0] < 0.05, "8m 深红端应几乎全吸收: {}", t[0]);
    }

    #[test]
    fn fresnel_bounds_and_grazing() {
        let f0 = WATER_F0;
        assert!(approx(fresnel_schlick(1.0, f0), f0, 1e-6), "垂直入射 = F0");
        assert!(approx(fresnel_schlick(0.0, f0), 1.0, 1e-6), "掠射 = 1");
        for i in 0..=20 {
            let c = i as f32 / 20.0;
            let f = fresnel_schlick(c, f0);
            assert!((f0 - 1e-6..=1.0 + 1e-6).contains(&f), "F 越界: {f}");
        }
        // 单调递减(cos 越大越接近垂直,反射越弱)。
        assert!(fresnel_schlick(0.2, f0) > fresnel_schlick(0.9, f0));
    }

    #[test]
    fn exp_step_endpoints_and_monotone() {
        let f = EXP_FACTOR;
        assert!(approx(exp_step_position(0.0, f), 0.0, 1e-6));
        assert!(approx(exp_step_position(1.0, f), 1.0, 1e-6));
        let mut prev = -1.0;
        for i in 0..=32 {
            let t = i as f32 / 32.0;
            let d = exp_step_position(t, f);
            assert!(d > prev, "非单调 t={t}");
            // 凸性:近处步长 < 平均步长 ⇒ d(t) < t。
            if i > 0 && i < 32 {
                assert!(d < t, "指数步进须近密远疏: d({t}) = {d}");
            }
            prev = d;
        }
    }

    #[test]
    fn adaptive_exp_factor_clamped() {
        assert!(approx(adaptive_exp_factor(20.0, 20.0), 1.01, 1e-6));
        assert!(adaptive_exp_factor(200.0, 20.0) <= 32.0);
        assert!(adaptive_exp_factor(0.0, 20.0) >= 1.01);
    }

    #[test]
    fn refraction_flat_surface_is_undistorted() {
        // 平静水面(法线 = +Y)⇒ 折射方向 ≡ 视线方向(只保留扰动分量的构造)。
        let v = normalize3([0.3, -0.8, 0.5]);
        let r = refraction_direction(v, [0.0, 1.0, 0.0], AIR_TO_WATER_ETA, 1.0);
        for i in 0..3 {
            assert!(approx(r[i], v[i], 1e-5), "平面折射须无位移: {r:?} vs {v:?}");
        }
        // 扰动法线 ⇒ 产生偏移。
        let n = normalize3([0.12, 1.0, -0.08]);
        let r2 = refraction_direction(v, n, AIR_TO_WATER_ETA, 1.0);
        assert!(dot3(r2, v) < 0.9999, "扰动法线须产生折射偏移");
    }

    #[test]
    fn caustic_gain_neutral_when_off() {
        // intensity = 0 ⇒ 乘性中性,恒 1(关臂零漂移)。
        for lap in [-0.5f32, 0.0, 0.5] {
            assert!(approx(caustic_gain(lap, 4.0, 2.2, 0.0), 1.0, 1e-6));
        }
        // 会聚(∇²h < 0)⇒ 增益 > 1;发散 ⇒ < 1。
        assert!(caustic_gain(-0.1, 4.0, 2.2, 1.0) > 1.0);
        assert!(caustic_gain(0.1, 4.0, 2.2, 1.0) < 1.0);
    }

    #[test]
    fn scene_depth_gradient_shallow_to_deep() {
        let s = LagoonScene::default();
        s.validate().unwrap();
        // 中心最深。
        let d_center = s.water_depth(s.center_x, s.center_z);
        assert!(
            approx(d_center, s.max_depth_m, 1e-4),
            "中心水深须为 max_depth: {d_center}"
        );
        // 岸线半径处恰达水面(沙丘项之外)。
        let d_shore = s.water_depth(s.center_x + s.shore_radius_m, s.center_z);
        assert!(
            d_shore.abs() < s.dune_amplitude_m * 2.0,
            "岸线水深应 ≈ 0: {d_shore}"
        );
        // 岸外为干地。
        let d_out = s.water_depth(s.center_x + s.shore_radius_m * 1.6, s.center_z);
        assert!(d_out < 0.0, "岸外须为干地: {d_out}");
        // 单调:自中心向外水深递减(取沙丘平均后的粗粒度断言)。
        let mut last = f32::INFINITY;
        for i in 0..=8 {
            let r = s.shore_radius_m * (i as f32 / 8.0);
            let d = s.water_depth(s.center_x + r, s.center_z);
            assert!(d < last + s.dune_amplitude_m * 2.5, "水深非递减 r={r}");
            last = d;
        }
    }

    #[test]
    fn scene_albedo_blends_at_waterline() {
        let s = LagoonScene::default();
        let wet = s.floor_albedo(s.center_x, s.center_z);
        let dry = s.floor_albedo(s.center_x + s.shore_radius_m * 1.8, s.center_z);
        assert!(wet != dry, "水下与岸上反照率须不同");
        // 水下 = 沙色。
        for i in 0..3 {
            assert!(approx(wet[i], s.sand_albedo[i], 1e-4));
        }
    }

    #[test]
    fn scene_rock_pack_layout_and_padding() {
        let s = LagoonScene::default();
        let packed = s.pack_rocks();
        assert_eq!(packed.len(), MAX_ROCKS * ROCK_STRIDE);
        assert_eq!(packed[3], s.rocks[0].radius);
        // 尾部空槽半径恒 0 ⇒ device 算术门恒不命中。
        for i in s.rocks.len()..MAX_ROCKS {
            assert_eq!(packed[i * ROCK_STRIDE + 3], 0.0);
        }
        // 超上界拒录。
        let mut too_many = s.clone();
        too_many.rocks = vec![s.rocks[0]; MAX_ROCKS + 1];
        assert!(matches!(
            too_many.validate(),
            Err(WaterSurfaceError::TooManyRocks { .. })
        ));
    }

    #[test]
    fn obstacle_mask_matches_scene_land() {
        let scene = LagoonScene::default();
        let mut sim = WaveSim::new(64, WaveParams::default()).unwrap();
        sim.fill_obstacles_from_scene(&scene);
        let land = sim.obstacle().iter().filter(|o| **o).count();
        assert!(land > 0, "泻湖外圈须有陆地障碍格");
        assert!(land < 64 * 64, "不可全为陆地");
        // 中心必为水。
        assert!(!sim.obstacle()[32 * 64 + 32], "泻湖中心须为水");
    }

    #[test]
    fn param_pack_layout_is_frozen() {
        let sky = Sky::new(super::super::sky::PRESET_GOLDEN);
        let scene = LagoonScene::default();
        let wp = WaterParams::default();
        let cam = WaterCamera {
            origin: [0.0, 4.0, 40.0],
            forward: [0.0, 0.0, -1.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            tan_half_fov: 0.5,
            aspect: 16.0 / 9.0,
        };
        let p = pack_water_params(
            &wp,
            &scene,
            &sky,
            &cam,
            1920,
            1080,
            7,
            0.117,
            8.0,
            WaterArms::default(),
            96,
            220.0,
            128,
            128,
            0,
        );
        assert_eq!(p.len(), WATER_PARAM_COUNT);
        assert_eq!(p[0], 1920.0);
        assert_eq!(p[1], 1080.0);
        assert_eq!(p[28], scene.water_level);
        assert_eq!(p[43], WAVE_DIM as f32);
        assert_eq!(p[67], AIR_TO_WATER_ETA);
        assert_eq!(p[71], 7.0);
        assert_eq!(p[80], scene.rocks.len() as f32);
        assert_eq!(p[88], 0.0, "debug_view 缺省关");
        assert_eq!(p[89], 1.0, "默认臂 ⇒ 水面主开关开");
        // 预留段恒 0。
        for v in &p[90..96] {
            assert_eq!(*v, 0.0);
        }
        // 全关臂 ⇒ 六个开关位全 0。
        let off = pack_water_params(
            &wp,
            &scene,
            &sky,
            &cam,
            1920,
            1080,
            7,
            0.117,
            8.0,
            WaterArms::all_off(),
            96,
            220.0,
            128,
            128,
            0,
        );
        for i in 82..88 {
            assert_eq!(off[i], 0.0, "关臂位 {i} 须为 0");
        }
        assert_eq!(off[89], 0.0, "全关臂 ⇒ 水面主开关关(pass 直通场景色)");
        assert!(p.iter().all(|v| v.is_finite()), "参数面须全有限");
    }

    #[test]
    fn drop_script_roundtrip_and_red() {
        let d = parse_drop_script("0:0.5,0.5,1.6; 24:0.34,0.62,1.1,5.0").unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].frame, 0);
        assert!(approx(d[1].radius, 5.0, 1e-6));
        // 默认半径。
        assert!(approx(d[0].radius, 6.0, 1e-6));
        // RED 臂。
        assert!(parse_drop_script("0.5,0.5,1.0").is_err(), "缺帧前缀须拒");
        assert!(parse_drop_script("0:1.5,0.5,1.0").is_err(), "u 越界须拒");
        assert!(parse_drop_script("0:0.5,0.5").is_err(), "分量不足须拒");
        assert!(parse_drop_script("x:0.5,0.5,1.0").is_err(), "帧号非法须拒");
    }

    #[test]
    fn blur_level_monotone_and_clamped() {
        let max = (BLUR_LEVELS - 1) as f32;
        let mut prev = -1.0;
        for d in [0.0f32, 0.5, 2.0, 8.0, 30.0, 100.0] {
            let l = blur_level(d, 0.2, 0.3, max);
            assert!((0.0..=max).contains(&l), "级号越界: {l}");
            assert!(l >= prev, "级号须随深度单调不减");
            prev = l;
        }
        assert_eq!(blur_level(0.0, 0.2, 0.3, max), 0.0, "零深度须为 0 级");
    }

    #[test]
    fn scatter_phase_is_positive_and_wavelength_ordered() {
        for c in [-1.0f32, -0.4, 0.0, 0.5, 1.0] {
            let p = scatter_phase(c, 0.72);
            assert!(p.iter().all(|v| v.is_finite() && *v > 0.0), "相位须正有限");
        }
        // 背向散射(cos ≈ −1)时瑞利项占比抬升 ⇒ 蓝通道相对更强。
        let back = scatter_phase(-1.0, 0.72);
        assert!(back[2] > back[0], "背向散射蓝端须强于红端");
    }

    #[test]
    fn noise2d_bake_is_deterministic_and_bounded() {
        let a = bake_noise2d(0x51ee_d001);
        let b = bake_noise2d(0x51ee_d001);
        assert_eq!(a, b, "同种子须逐位一致");
        assert_eq!(a.len(), NOISE2D_DIM * NOISE2D_DIM);
        assert!(a.iter().all(|v| (0.0..=1.0).contains(v)), "噪声须 ∈ [0,1]");
        let c = bake_noise2d(0x51ee_d002);
        assert_ne!(a, c, "异种子须不同");
    }

    #[test]
    fn ggx_specular_is_finite_and_nonnegative() {
        for r in [0.02f32, 0.05, 0.3, 0.9] {
            for nh in [0.1f32, 0.5, 0.999] {
                let s = ggx_specular(nh, 0.6, 0.7, r);
                assert!(s.is_finite() && s >= 0.0, "GGX 非法: r={r} nh={nh} → {s}");
            }
        }
    }

    #[test]
    fn ign_is_unit_interval() {
        for f in 0..8u32 {
            for i in 0..64 {
                let v = interleaved_gradient_noise(i as f32, (i * 7 % 33) as f32, f);
                assert!((0.0..1.0).contains(&v), "IGN 越界: {v}");
            }
        }
    }
}
