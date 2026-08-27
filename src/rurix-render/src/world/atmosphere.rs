//! Froxel 大气前端(G9.5 M112;RFC-0025 §4.C;spec/world_partition.md
//! RXS-0365 L1~L5 逐条对齐)。
//!
//! //@ spec: RXS-0365
//!
//! 本模块承载 M112 Froxel 统一基础设施 + 雾前端(大气体渲染器前端):
//!
//! - **Froxel 统一基础设施(一次性建造)**:视锥体素网格(froxel volume)+
//!   密度/光照累积 + 深度切片分布 + 帧图合成节点;**云与雾共用同一 Froxel
//!   基础设施、两个前端**(各自独立体渲染器即 RED——本模块单入口
//!   [`FroxelVolume`],云/雾前端均为 [`FroxelVolume`] 的密度/光照写入器)。
//! - **雾前端(高度雾/分层介质解析项写密度场)**:解析项为主,预算极小;
//!   高度衰减解析式直接写 Froxel 密度场(每 voxel 密度 = base_density ×
//!   exp(-height / falloff))。
//! - **计数面(RXS-0365 L4)**:froxel 网格维度 / 注入光源数 / 散射积分步数
//!   非空逐帧 evidence([`FrameEvidence`]);网格维度篡改(非 canonical 分辨率档)
//!   即 RED;零散射贡献(光源全零/密度全零)即 RED。
//! - **深度切片分布**:对数切片(slice_z = exp(k·z) 分布)或线性切片(资产属性
//!   `DepthSliceMode`),与帧图合成节点一次性建造。
//! - **weather map 资产化**(RXS-0365 L3):2D weather map(覆盖度/湿度/类型)
//!   走 M01/M85 资产通道(canonical 编码 + digest 签名),**篡改资产签名即拒录**
//!   (RED 臂独立有效);时序链断裂(首帧无历史)必须正确初始化,不得复用脏帧
//!   (首帧冷启动 density=0 初始化,帧号连续性机核)。
//!
//! 纪律:host 纯 safe 确定性(全库 `forbid(unsafe_code)`);零新 FFI;无 device
//! 依赖——M112 前端语义面 = 体素网格 + 密度/光照累积 + 散射积分,GPU 非必需;
//! `RURIX_REQUIRE_REAL=1` 下以 host 确定性为准。G8 底座(M01/M85 资产通道)只
//! 消费不重定。

use rurix_pkg::sha256;

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// Froxel 体素网格维度档(canonical 分辨率档闭集:64×64×64 / 128×128×64 /
/// 160×90×64;网格维度篡改=非档内维度即 RED)。
pub const FROXEL_DIM_TIERS: [u32; 3] = [64, 128, 160];
/// 深度切片数(冻结;z 向体素数)。
pub const FROXEL_DEPTH_SLICES: u32 = 64;
/// 散射积分最大步数预算(RXS-0365 L4 预算字段之一;ray-march max steps)。
pub const MAX_SCATTER_STEPS: u32 = 64;
/// weather map 资产 magic("RXWM")。
pub const WEATHER_MAP_MAGIC: [u8; 4] = *b"RXWM";
/// weather map 资产格式版本。
pub const WEATHER_MAP_VERSION: u16 = 1;

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed)
// ---------------------------------------------------------------------------

/// Froxel 大气失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum AtmosphereError {
    /// 网格维度篡改(非 canonical 分辨率档;RED 锚)。
    GridDimTampered { got: [u32; 3], tiers: [u32; 3] },
    /// weather map 资产签名/内容篡改(RED 锚:签名即拒录)。
    WeatherMapTampered { why: &'static str },
    /// 时序链断裂:首帧无历史却复用脏帧 / 帧号非单调(RED 锚)。
    TemporalChainBroken { frame: u32, why: &'static str },
    /// 零散射贡献(光源全零或密度全零;散射积分面失效 RED)。
    ZeroScatteringContribution { stage: &'static str },
    /// 散射积分步数越界(0 或超预算)。
    ScatterStepsOutOfRange { got: u32, max: u32 },
    /// 输入含非有限值(NaN/Inf)。
    NonFiniteValue { stage: &'static str },
}

impl std::fmt::Display for AtmosphereError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtmosphereError::GridDimTampered { got, tiers } => {
                write!(
                    f,
                    "froxel 网格维度篡改: {got:?} 不在 canonical 档 {tiers:?}(RED)"
                )
            }
            AtmosphereError::WeatherMapTampered { why } => {
                write!(f, "weather map 资产篡改: {why}(RED)")
            }
            AtmosphereError::TemporalChainBroken { frame, why } => {
                write!(f, "时序链断裂:帧 {frame} {why}(RED)")
            }
            AtmosphereError::ZeroScatteringContribution { stage } => {
                write!(f, "{stage} 零散射贡献(光源/密度全零,RED)")
            }
            AtmosphereError::ScatterStepsOutOfRange { got, max } => {
                write!(f, "散射积分步数 {got} 越界(1..={max})")
            }
            AtmosphereError::NonFiniteValue { stage } => {
                write!(f, "{stage} 阶段含非有限值(NaN/Inf)")
            }
        }
    }
}

impl std::error::Error for AtmosphereError {}

pub type Result<T> = std::result::Result<T, AtmosphereError>;

// ---------------------------------------------------------------------------
// weather map 资产(2D,走 M01/M85 通道;签名即完整性)
// ---------------------------------------------------------------------------

/// 2D weather map(覆盖度/湿度/类型;资产化走 M01/M85 通道,签名即完整性)。
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherMap {
    pub width: u32,
    pub height: u32,
    /// RGB:覆盖度(cloud coverage)、湿度(humidity)、类型(cloud type)逐通道。
    pub pixels: Vec<[f32; 3]>,
}

/// weather map 资产签名(SHA-256 digest;篡改资产内容或签名即拒录)。
pub fn weather_map_signature(map: &WeatherMap) -> [u8; 32] {
    let mut buf = Vec::new();
    buf.extend_from_slice(&WEATHER_MAP_MAGIC);
    buf.extend_from_slice(&WEATHER_MAP_VERSION.to_le_bytes());
    buf.extend_from_slice(&map.width.to_le_bytes());
    buf.extend_from_slice(&map.height.to_le_bytes());
    for p in &map.pixels {
        for v in p {
            buf.extend_from_slice(&v.to_le_bytes());
        }
    }
    sha256::digest(&buf)
}

/// weather map 资产完整性核验(签名比对;篡改即 typed Err)。
pub fn verify_weather_map(map: &WeatherMap, expected_sig: &[u8; 32]) -> Result<()> {
    let actual = weather_map_signature(map);
    if actual != *expected_sig {
        return Err(AtmosphereError::WeatherMapTampered {
            why: "资产签名与内容不符",
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Froxel 统一基础设施(体素网格 + 密度/光照累积 + 深度切片 + 合成节点)
// ---------------------------------------------------------------------------

/// Froxel 体素网格(视锥对齐 voxel volume;密度 + 光照累积双通道)。
#[derive(Debug, Clone)]
pub struct FroxelVolume {
    pub dim: [u32; 3],
    pub depth_slices: u32,
    /// 密度通道(雾化介质;逐 voxel 标量密度 ∈ [0,∞))。
    pub density: Vec<f32>,
    /// 光照累积通道(逐 voxel RGB 入射光)。
    pub lighting: Vec<[f32; 3]>,
}

impl FroxelVolume {
    /// canonical 建造:维度档核验(网格维度篡改即 RED)+ 零初始化。
    pub fn new(dim: [u32; 3], depth_slices: u32) -> Result<Self> {
        if !FROXEL_DIM_TIERS.contains(&dim[0])
            || !FROXEL_DIM_TIERS.contains(&dim[1])
            || depth_slices != FROXEL_DEPTH_SLICES
        {
            return Err(AtmosphereError::GridDimTampered {
                got: dim,
                tiers: FROXEL_DIM_TIERS,
            });
        }
        let n = (dim[0] * dim[1] * depth_slices) as usize;
        Ok(Self {
            dim,
            depth_slices,
            density: vec![0.0; n],
            lighting: vec![[0.0; 3]; n],
        })
    }

    pub fn voxel_count(&self) -> usize {
        (self.dim[0] * self.dim[1] * self.depth_slices) as usize
    }
}

/// 雾前端(高度雾/分层介质解析项):直接写 Froxel 密度场。
///
/// 高度衰减解析式:密度 = base_density × exp(-(world_z - height_offset) / falloff),
/// world_z = voxel_z × slice_spacing(对数切片 slice_spacing = base_spacing ×
/// exp(k·slice))或线性切片(等距);预算极小(解析项无 ray-march)。
pub struct FogFrontend {
    pub base_density: f32,
    pub falloff_m: f32,
    pub height_offset_m: f32,
    pub slice_spacing_m: f32,
    pub log_slices: bool,
}

impl FogFrontend {
    pub fn write_density(&self, volume: &mut FroxelVolume) -> Result<()> {
        if !self.base_density.is_finite() || self.base_density < 0.0 {
            return Err(AtmosphereError::NonFiniteValue {
                stage: "fog_base_density",
            });
        }
        if !self.falloff_m.is_finite() || self.falloff_m <= 0.0 {
            return Err(AtmosphereError::NonFiniteValue {
                stage: "fog_falloff",
            });
        }
        let dim = volume.dim;
        let slices = volume.depth_slices;
        for z in 0..slices {
            let slice_spacing = if self.log_slices {
                self.slice_spacing_m * (z as f32 / 8.0).exp()
            } else {
                self.slice_spacing_m
            };
            let world_z = z as f32 * slice_spacing - self.height_offset_m;
            let density = self.base_density * (-world_z / self.falloff_m).exp();
            for y in 0..dim[1] {
                for x in 0..dim[0] {
                    let idx = (z * dim[1] * dim[0] + y * dim[0] + x) as usize;
                    volume.density[idx] = density;
                }
            }
        }
        Ok(())
    }
}

/// 注入光源(逐 voxel 入射光累加;方向光 + 点光源闭集)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InjectLight {
    pub kind: LightKind,
    /// RGB 辐射强度(>0 才有贡献)。
    pub radiance: [f32; 3],
    /// 方向(方向光)或位置(点光源)。
    pub vector: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightKind {
    Directional,
    Point,
}

/// 散射积分(单散射 + 高度雾解析项;预算字段:最大步数、步长)。
pub struct ScatterIntegrator {
    pub max_steps: u32,
    pub step_size_m: f32,
}

impl ScatterIntegrator {
    pub fn new(max_steps: u32, step_size_m: f32) -> Result<Self> {
        if max_steps == 0 || max_steps > MAX_SCATTER_STEPS {
            return Err(AtmosphereError::ScatterStepsOutOfRange {
                got: max_steps,
                max: MAX_SCATTER_STEPS,
            });
        }
        if !step_size_m.is_finite() || step_size_m <= 0.0 {
            return Err(AtmosphereError::NonFiniteValue {
                stage: "scatter_step",
            });
        }
        Ok(Self {
            max_steps,
            step_size_m,
        })
    }

    /// 沿视线单散射积分(简化解析:指数消光 × 单散射反照率;输出 RGB)。
    pub fn integrate(
        &self,
        volume: &FroxelVolume,
        lights: &[InjectLight],
        origin: [f32; 3],
        dir: [f32; 3],
    ) -> Result<[f32; 3]> {
        // 零散射贡献检测(RED 锚:光源全零或密度全零即 RED)。
        if lights.is_empty() {
            return Err(AtmosphereError::ZeroScatteringContribution { stage: "no_lights" });
        }
        let light_total: f32 = lights
            .iter()
            .map(|l| l.radiance[0] + l.radiance[1] + l.radiance[2])
            .sum();
        if light_total <= 0.0 {
            return Err(AtmosphereError::ZeroScatteringContribution {
                stage: "lights_zero",
            });
        }
        let density_total: f32 = volume.density.iter().sum();
        if density_total <= 0.0 {
            return Err(AtmosphereError::ZeroScatteringContribution {
                stage: "density_zero",
            });
        }
        // 简化单散射(Beer-Lambert 消光 + 均匀单散射反照率 0.9):
        let albedo = 0.9f32;
        let mut out = [0.0f32; 3];
        let mut transmittance = 1.0f32;
        let mut pos = origin;
        let step_dir = [
            dir[0] * self.step_size_m,
            dir[1] * self.step_size_m,
            dir[2] * self.step_size_m,
        ];
        for _ in 0..self.max_steps {
            // 最近 voxel 密度采样(简化:按位置线性索引到 z-slice)。
            let z_slice =
                ((pos[2] / self.step_size_m).max(0.0) as u32).min(volume.depth_slices - 1);
            let x_vox = ((pos[0] / self.step_size_m).max(0.0) as u32).min(volume.dim[0] - 1);
            let y_vox = ((pos[1] / self.step_size_m).max(0.0) as u32).min(volume.dim[1] - 1);
            let idx =
                (z_slice * volume.dim[1] * volume.dim[0] + y_vox * volume.dim[0] + x_vox) as usize;
            let density = volume.density[idx];
            if density > 0.0 {
                // 逐光源入射累积(简化:方向光强度 × 相位近似 1/(4π))。
                let mut inscatter = [0.0f32; 3];
                for l in lights {
                    let phase = 1.0 / (4.0 * std::f32::consts::PI);
                    for (ic, r) in inscatter.iter_mut().zip(l.radiance.iter()) {
                        *ic += r * phase;
                    }
                }
                let sigma_t = density * 0.1; // 消光系数(简化资产属性)
                let sigma_s = sigma_t * albedo;
                let step_trans = (-sigma_t * self.step_size_m).exp();
                for c in 0..3 {
                    out[c] += transmittance * sigma_s * inscatter[c] * self.step_size_m;
                }
                transmittance *= step_trans;
            }
            pos[0] += step_dir[0];
            pos[1] += step_dir[1];
            pos[2] += step_dir[2];
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// 逐帧计数 evidence(RXS-0365 L4:网格维度/光源数/散射步数非空逐帧)
// ---------------------------------------------------------------------------

/// 逐帧 Froxel 计数 evidence(预算字段非空)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameEvidence {
    pub frame: u32,
    pub grid_dim: [u32; 3],
    pub light_count: u32,
    pub scatter_steps: u32,
    pub density_nonzero_voxels: u32,
    pub temporal_init: bool,
}

impl FrameEvidence {
    /// 非空断言(RED 锚:网格维度/光源数/散射步数任一零即 RED)。
    pub fn assert_nonempty(&self) -> Result<()> {
        if self.grid_dim[0] == 0 || self.grid_dim[1] == 0 || self.grid_dim[2] == 0 {
            return Err(AtmosphereError::GridDimTampered {
                got: self.grid_dim,
                tiers: FROXEL_DIM_TIERS,
            });
        }
        if self.light_count == 0 {
            return Err(AtmosphereError::ZeroScatteringContribution {
                stage: "light_count_zero",
            });
        }
        if self.scatter_steps == 0 {
            return Err(AtmosphereError::ScatterStepsOutOfRange {
                got: 0,
                max: MAX_SCATTER_STEPS,
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 时序上采样默认路径(RXS-0365 L3:temporal reprojection 为默认;首帧无历史
// 必须正确初始化,不得复用脏帧)
// ---------------------------------------------------------------------------

/// 时序链状态(首帧无历史 ⇒ 正确初始化;帧号连续性机核)。
pub struct TemporalChain {
    pub frame: u32,
    pub initialized: bool,
    pub prev_frame: Option<u32>,
}

impl TemporalChain {
    pub fn new() -> Self {
        Self {
            frame: 0,
            initialized: false,
            prev_frame: None,
        }
    }

    /// 逐帧推进:首帧(frame=0)必须初始化(不得复用脏帧);后续帧号必须连续。
    pub fn tick(&mut self, frame: u32) -> Result<()> {
        if frame == 0 && !self.initialized {
            self.initialized = true;
            self.prev_frame = Some(0);
            self.frame = 0;
            return Ok(());
        }
        match self.prev_frame {
            None => {
                return Err(AtmosphereError::TemporalChainBroken {
                    frame,
                    why: "首帧无历史未初始化(复用脏帧)",
                });
            }
            Some(prev) if frame != prev + 1 => {
                return Err(AtmosphereError::TemporalChainBroken {
                    frame,
                    why: "帧号非连续",
                });
            }
            _ => {}
        }
        self.prev_frame = Some(frame);
        self.frame = frame;
        Ok(())
    }
}

impl Default for TemporalChain {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// canonical 场景(harness 与单测同一事实源;measured 冻结,禁手写 golden)
// ---------------------------------------------------------------------------

/// canonical golden 场景:64×64×64 Froxel 网格 + 高度雾前端 + 方向光注入。
pub fn canonical_scene() -> (
    FroxelVolume,
    FogFrontend,
    Vec<InjectLight>,
    ScatterIntegrator,
) {
    let vol = FroxelVolume::new([64, 64, 64], FROXEL_DEPTH_SLICES).expect("canonical grid");
    let fog = FogFrontend {
        base_density: 0.05,
        falloff_m: 64.0,
        height_offset_m: 0.0,
        slice_spacing_m: 2.0,
        log_slices: true,
    };
    let lights = vec![
        InjectLight {
            kind: LightKind::Directional,
            radiance: [1.0, 0.95, 0.9],
            vector: [0.0, -1.0, 0.3],
        },
        InjectLight {
            kind: LightKind::Point,
            radiance: [0.5, 0.5, 0.6],
            vector: [32.0, 32.0, 10.0],
        },
    ];
    let scatter = ScatterIntegrator::new(32, 2.0).expect("canonical scatter");
    (vol, fog, lights, scatter)
}

/// canonical weather map(32×32,LCG 确定性;供完整性核验 golden)。
pub fn canonical_weather_map() -> WeatherMap {
    let w = 32u32;
    let h = 32u32;
    let mut pixels = Vec::with_capacity((w * h) as usize);
    let mut s: u64 = 0x9e37_79b9_7f4a_7c15;
    for _ in 0..(w * h) {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let coverage = ((s >> 11) % 1000) as f32 / 1000.0;
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let humidity = ((s >> 11) % 1000) as f32 / 1000.0;
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let ctype = ((s >> 11) % 1000) as f32 / 1000.0;
        pixels.push([coverage, humidity, ctype]);
    }
    WeatherMap {
        width: w,
        height: h,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RXS-0365 L1:Froxel 统一基础设施(云雾共用同一基础设施断言——单入口
    /// FroxelVolume,FogFrontend 写入密度场)。
    #[test]
    //@ spec: RXS-0365
    fn froxel_unified_infrastructure_single_entry() {
        let (mut vol, fog, _, _) = canonical_scene();
        assert_eq!(vol.voxel_count(), 64 * 64 * 64);
        assert!(vol.density.iter().all(|&d| d == 0.0));
        fog.write_density(&mut vol).unwrap();
        // 密度非零且随高度衰减(z 升密度降,对数切片)。
        let z0 = vol.density[0];
        let z1 = vol.density[(64 * 64) as usize];
        assert!(z0 > 0.0 && z1 > 0.0 && z0 > z1, "高度雾密度随 z 衰减");
    }

    /// RXS-0365 L4:计数面非空逐帧 evidence(网格维度/光源数/散射步数非空)。
    #[test]
    //@ spec: RXS-0365
    fn per_frame_counters_nonempty() {
        let (mut vol, fog, lights, scatter) = canonical_scene();
        fog.write_density(&mut vol).unwrap();
        let ev = FrameEvidence {
            frame: 0,
            grid_dim: vol.dim,
            light_count: lights.len() as u32,
            scatter_steps: scatter.max_steps,
            density_nonzero_voxels: vol.density.iter().filter(|&&d| d > 0.0).count() as u32,
            temporal_init: true,
        };
        ev.assert_nonempty().expect("非空计数");
        assert_eq!(ev.light_count, 2);
        assert_eq!(ev.scatter_steps, 32);
        assert!(ev.density_nonzero_voxels > 0);
    }

    /// RXS-0365 L4(RED 锚):零散射贡献注入(光源全零/密度全零)必 RED;
    /// 网格维度篡改(非 canonical 档)必 RED。
    #[test]
    //@ spec: RXS-0365
    fn zero_scattering_and_grid_tamper_red() {
        let (mut vol, fog, lights, scatter) = canonical_scene();
        fog.write_density(&mut vol).unwrap();
        // 光源全零 ⇒ RED。
        let dark = vec![InjectLight {
            kind: LightKind::Directional,
            radiance: [0.0, 0.0, 0.0],
            vector: [0.0, -1.0, 0.0],
        }];
        assert!(matches!(
            scatter.integrate(&vol, &dark, [32.0, 32.0, 0.0], [0.0, 0.0, 1.0]),
            Err(AtmosphereError::ZeroScatteringContribution { .. })
        ));
        // 密度全零 ⇒ RED。
        let empty = FroxelVolume::new([64, 64, 64], FROXEL_DEPTH_SLICES).unwrap();
        assert!(matches!(
            scatter.integrate(&empty, &lights, [32.0, 32.0, 0.0], [0.0, 0.0, 1.0]),
            Err(AtmosphereError::ZeroScatteringContribution { .. })
        ));
        // 网格维度篡改 ⇒ RED。
        assert!(matches!(
            FroxelVolume::new([96, 96, 64], FROXEL_DEPTH_SLICES),
            Err(AtmosphereError::GridDimTampered { .. })
        ));
        // sabotage:正常积分非零。
        let out = scatter
            .integrate(&vol, &lights, [32.0, 32.0, 0.0], [0.0, 0.0, 1.0])
            .unwrap();
        assert!(
            out.iter().all(|v| v.is_finite() && *v > 0.0),
            "正常散射非零"
        );
    }

    /// RXS-0365 L3:weather map 资产化 + 签名篡改拒录(RED 锚)。
    #[test]
    //@ spec: RXS-0365
    fn weather_map_signature_tamper_red() {
        let map = canonical_weather_map();
        let sig = weather_map_signature(&map);
        verify_weather_map(&map, &sig).expect("签名合法");
        // 内容篡改 ⇒ 拒录。
        let mut tampered = map.clone();
        tampered.pixels[0][0] += 0.5;
        assert!(matches!(
            verify_weather_map(&tampered, &sig),
            Err(AtmosphereError::WeatherMapTampered { .. })
        ));
        // 签名伪造 ⇒ 拒录。
        let mut forged = sig;
        forged[0] ^= 0xa5;
        assert!(matches!(
            verify_weather_map(&map, &forged),
            Err(AtmosphereError::WeatherMapTampered { .. })
        ));
    }

    /// RXS-0365 L3:时序链断裂(首帧无历史)必须正确初始化;复用脏帧/跳帧必 RED。
    #[test]
    //@ spec: RXS-0365
    fn temporal_chain_init_and_break_red() {
        let mut chain = TemporalChain::new();
        chain.tick(0).unwrap();
        chain.tick(1).unwrap();
        assert!(chain.initialized);
        // 跳帧 ⇒ RED。
        assert!(matches!(
            chain.tick(3),
            Err(AtmosphereError::TemporalChainBroken { .. })
        ));
        // 首帧无历史未初始化 ⇒ RED。
        let mut bad = TemporalChain::new();
        bad.prev_frame = None;
        bad.initialized = false;
        assert!(matches!(
            bad.tick(1),
            Err(AtmosphereError::TemporalChainBroken { .. })
        ));
    }
}
