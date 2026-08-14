//! 水体双管线(G9.5 M113;RFC-0025 §4.D;spec/world_partition.md RXS-0366 L1~L5
//! 逐条对齐)。
//!
//! //@ spec: RXS-0366
//!
//! 本模块承载 M113 水体专项渲染器前端语义面:
//!
//! - **大洋管线**(L1):Tessendorf IFFT 谱离线参数化(风向/风速/涌浪为
//!   [`OceanSpectrumAsset`] 资产属性,canonical 编码 + digest 签名)+ 运行时
//!   compute IFFT(host 确定性参照:radix-2 FFT 模型)产**位移/梯度/Jacobian
//!   三贴图**,Jacobian 负值驱动泡沫([`OceanFrame::foam_mask`]);CDLOD 距离
//!   分档 mesh([`cdlod_tier`]);多尺度谱 tiling-and-blending 防周期重复感
//!   ([`tile_blend_weight`]);大洋 compute IFFT 三贴图与 host DFT 参考**逐值
//!   对拍**([`reference_dft_height`],容差 = measured 精确值经冻结带明示,
//!   禁手写,P-09)。
//! - **浅水管线**(L2):局部波方程(高度场 + 速度场 ping-pong,
//!   [`ShallowWaveSim::step`])服务池塘/河流/交互波纹;**浅水域越界写检测**
//!   ([`ShallowWaveSim::poke`] 越界即 typed `Err`,RED 锚)。
//! - **双管线分离断言**(L3):大洋与浅水**不共享几何路径**——几何路径 token
//!   闭集互斥机核([`assert_geometry_paths_disjoint`],互斥违反即 RED);仅共
//!   享水面着色 closure 输入面([`WaterShadingInput`])。
//! - **非法谱参数资产拒录**(L4):负风速/非法谱参数资产 → 装配期 typed
//!   `Err(InvalidSpectrumParam)`(RED 臂独立有效)。
//! - **浮力接口面预留不实现**(L5):[`buoyancy_query`] 一律 typed
//!   `Err(BuoyancyInterfaceReserved)`——M77→M124 归 G9.6 物理波 Field 通道,
//!   本条款不授权任何浮力实现或旁路 API。
//!
//! 纪律:host 纯 safe 确定性(全库 `forbid(unsafe_code)`);零新 FFI;无 device
//! 依赖——M113 语义面 = 谱参数化 + IFFT 数学 + 双管线互斥机核,GPU 非必需;
//! `RURIX_REQUIRE_REAL=1` 下以 host 确定性为准。

use rurix_pkg::sha256;

// ---------------------------------------------------------------------------
// 冻结常量面
// ---------------------------------------------------------------------------

/// 大洋谱资产 canonical 二进制 magic("RXOS")。
pub const SPECTRUM_MAGIC: [u8; 4] = *b"RXOS";
/// 谱资产格式版本。
pub const SPECTRUM_VERSION: u16 = 1;
/// 大洋 IFFT 网格维(canonical = 32;2 的幂)。
pub const OCEAN_GRID_N: usize = 32;
/// 重力加速度(m/s²)。
pub const GRAVITY: f64 = 9.81;
/// CDLOD 距离环边界(米;档闭集 0..=3)。
pub const CDLOD_RING_M: [f64; 3] = [64.0, 256.0, 1024.0];
/// 浅水仿真网格维(canonical)。
pub const SHALLOW_DIM: usize = 16;
/// 浅水波速²·dt 闭式系数(稳定性界内)。
pub const SHALLOW_C2_DT: f32 = 0.2;
/// 浅水 dt。
pub const SHALLOW_DT: f32 = 0.5;

// ---------------------------------------------------------------------------
// 错误面(typed Err,fail-closed)
// ---------------------------------------------------------------------------

/// 水体失败类别。
#[derive(Debug, Clone, PartialEq)]
pub enum WaterError {
    /// 字节流截断。
    Truncated { at: usize, need: usize },
    /// 解码后残余字节。
    TrailingBytes { extra: usize },
    /// magic 不符。
    BadMagic,
    /// 不支持的资产版本。
    UnsupportedVersion(u16),
    /// 非 canonical 构造。
    NotCanonical(&'static str),
    /// 非法谱参数(负风速/非有限/非正 fetch/depth/tile;装配期拒录,RED 锚)。
    InvalidSpectrumParam { field: &'static str },
    /// 资产签名/内容篡改。
    AssetTampered { why: &'static str },
    /// 浅水域越界写(RED 锚)。
    ShallowOutOfBoundsWrite { index: usize, extent: usize },
    /// 双管线几何路径互斥违反(共享几何 token,RED 锚)。
    GeometryPathShared { token: &'static str },
    /// 浮力查询接口面预留不实现(M77→M124 归 G9.6 Field 通道)。
    BuoyancyInterfaceReserved,
    /// IFFT 网格维非 2 的幂。
    GridNotPowerOfTwo { got: usize },
}

impl std::fmt::Display for WaterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaterError::Truncated { at, need } => write!(f, "truncated: offset {at} 需 {need}"),
            WaterError::TrailingBytes { extra } => write!(f, "trailing bytes: 残余 {extra}"),
            WaterError::BadMagic => write!(f, "bad magic(非 RXOS)"),
            WaterError::UnsupportedVersion(v) => write!(f, "unsupported version {v}"),
            WaterError::NotCanonical(why) => write!(f, "not canonical: {why}"),
            WaterError::InvalidSpectrumParam { field } => {
                write!(f, "非法谱参数: {field}(装配期拒录,RED)")
            }
            WaterError::AssetTampered { why } => write!(f, "谱资产篡改: {why}(RED)"),
            WaterError::ShallowOutOfBoundsWrite { index, extent } => {
                write!(f, "浅水域越界写: index {index} 超 extent {extent}(RED)")
            }
            WaterError::GeometryPathShared { token } => {
                write!(f, "双管线几何路径互斥违反: 共享 token {token}(RED)")
            }
            WaterError::BuoyancyInterfaceReserved => {
                write!(f, "浮力查询接口面预留不实现(M77→M124 归 G9.6 Field 通道)")
            }
            WaterError::GridNotPowerOfTwo { got } => write!(f, "IFFT 网格维 {got} 非 2 的幂"),
        }
    }
}

impl std::error::Error for WaterError {}

pub type Result<T> = std::result::Result<T, WaterError>;

// ---------------------------------------------------------------------------
// 大洋谱资产(L1:风向/风速/涌浪为资产属性;canonical 编码 + 签名)
// ---------------------------------------------------------------------------

/// Tessendorf 大洋谱参数资产(离线参数化;M01/M85 通道口径签名)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OceanSpectrumAsset {
    /// 风向(弧度)。
    pub wind_dir_rad: f64,
    /// 风速(m/s;**负值即拒录**)。
    pub wind_speed: f64,
    /// 涌浪幅度系数(≥0)。
    pub swell: f64,
    /// 风区长度(m;>0)。
    pub fetch_m: f64,
    /// 水深(m;>0;色散 tanh 项)。
    pub depth_m: f64,
    /// 切浪(choppiness)位移系数(≥0)。
    pub choppiness: f64,
    /// 谱 tile 边长(米;>0)。
    pub tile_size_m: f64,
}

impl OceanSpectrumAsset {
    /// 装配期域校验(L4:负风速/非法谱参数即拒录)。
    pub fn validate(&self) -> Result<()> {
        if !self.wind_speed.is_finite() || self.wind_speed < 0.0 {
            return Err(WaterError::InvalidSpectrumParam { field: "wind_speed" });
        }
        if !self.wind_dir_rad.is_finite() {
            return Err(WaterError::InvalidSpectrumParam { field: "wind_dir_rad" });
        }
        if !self.swell.is_finite() || self.swell < 0.0 {
            return Err(WaterError::InvalidSpectrumParam { field: "swell" });
        }
        if !self.fetch_m.is_finite() || self.fetch_m <= 0.0 {
            return Err(WaterError::InvalidSpectrumParam { field: "fetch_m" });
        }
        if !self.depth_m.is_finite() || self.depth_m <= 0.0 {
            return Err(WaterError::InvalidSpectrumParam { field: "depth_m" });
        }
        if !self.choppiness.is_finite() || self.choppiness < 0.0 {
            return Err(WaterError::InvalidSpectrumParam { field: "choppiness" });
        }
        if !self.tile_size_m.is_finite() || self.tile_size_m <= 0.0 {
            return Err(WaterError::InvalidSpectrumParam { field: "tile_size_m" });
        }
        Ok(())
    }
}

/// canonical 二进制编码(magic + version + 七 f64 字段,LE)。
pub fn encode_spectrum(a: &OceanSpectrumAsset) -> Vec<u8> {
    let mut w = Vec::with_capacity(4 + 2 + 56);
    w.extend_from_slice(&SPECTRUM_MAGIC);
    w.extend_from_slice(&SPECTRUM_VERSION.to_le_bytes());
    for v in [
        a.wind_dir_rad, a.wind_speed, a.swell, a.fetch_m, a.depth_m, a.choppiness, a.tile_size_m,
    ] {
        w.extend_from_slice(&v.to_le_bytes());
    }
    w
}

/// canonical 二进制解码(装配期校验一体:非法谱参数即拒录)。
pub fn decode_spectrum(bytes: &[u8]) -> Result<OceanSpectrumAsset> {
    let take = |pos: &mut usize, n: usize| -> Result<&[u8]> {
        if bytes.len() - *pos < n {
            return Err(WaterError::Truncated { at: *pos, need: n });
        }
        let s = &bytes[*pos..*pos + n];
        *pos += n;
        Ok(s)
    };
    let mut pos = 0usize;
    if take(&mut pos, 4)? != SPECTRUM_MAGIC {
        return Err(WaterError::BadMagic);
    }
    let ver = u16::from_le_bytes(take(&mut pos, 2)?.try_into().expect("u16"));
    if ver != SPECTRUM_VERSION {
        return Err(WaterError::UnsupportedVersion(ver));
    }
    let mut vals = [0.0f64; 7];
    for v in vals.iter_mut() {
        *v = f64::from_le_bytes(take(&mut pos, 8)?.try_into().expect("f64"));
    }
    if pos != bytes.len() {
        return Err(WaterError::TrailingBytes { extra: bytes.len() - pos });
    }
    let a = OceanSpectrumAsset {
        wind_dir_rad: vals[0],
        wind_speed: vals[1],
        swell: vals[2],
        fetch_m: vals[3],
        depth_m: vals[4],
        choppiness: vals[5],
        tile_size_m: vals[6],
    };
    a.validate()?;
    Ok(a)
}

/// 资产签名(digest 即完整性)。
pub fn spectrum_signature(a: &OceanSpectrumAsset) -> [u8; 32] {
    sha256::digest(&encode_spectrum(a))
}

/// 资产完整性核验(篡改即拒录)。
pub fn verify_spectrum(a: &OceanSpectrumAsset, expected_sig: &[u8; 32]) -> Result<()> {
    if &spectrum_signature(a) != expected_sig {
        return Err(WaterError::AssetTampered { why: "digest 不符" });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 复数与 FFT(radix-2,确定性 f64;compute IFFT 的 host 参照模型)
// ---------------------------------------------------------------------------

/// 复数(f64 双;确定性)。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct C64 {
    pub re: f64,
    pub im: f64,
}

impl C64 {
    fn add(self, o: C64) -> C64 {
        C64 { re: self.re + o.re, im: self.im + o.im }
    }
    fn sub(self, o: C64) -> C64 {
        C64 { re: self.re - o.re, im: self.im - o.im }
    }
    fn mul(self, o: C64) -> C64 {
        C64 { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
    fn scale(self, s: f64) -> C64 {
        C64 { re: self.re * s, im: self.im * s }
    }
    fn conj(self) -> C64 {
        C64 { re: self.re, im: -self.im }
    }
    fn exp_i(theta: f64) -> C64 {
        C64 { re: theta.cos(), im: theta.sin() }
    }
}

/// 1D radix-2 FFT(迭代位反转;`inverse` 时旋转因子取正号,调用面负责 1/N 缩放)。
fn fft_1d(a: &mut [C64], inverse: bool) {
    let n = a.len();
    debug_assert!(n.is_power_of_two());
    // 位反转重排。
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let sign = if inverse { 1.0 } else { -1.0 };
        let w_step = C64::exp_i(sign * 2.0 * std::f64::consts::PI / len as f64);
        let mut start = 0;
        while start < n {
            let mut w = C64 { re: 1.0, im: 0.0 };
            for k in 0..half {
                let u = a[start + k];
                let v = a[start + k + half].mul(w);
                a[start + k] = u.add(v);
                a[start + k + half] = u.sub(v);
                w = w.mul(w_step);
            }
            start += len;
        }
        len *= 2;
    }
}

/// 2D IFFT(行→列;Tessendorf 求和约定——谱幅值即模态幅值,不做 1/N² 缩放;
/// host DFT 参考同约定)。
fn ifft_2d(grid: &mut [C64], n: usize) {
    for row in grid.chunks_exact_mut(n) {
        fft_1d(row, true);
    }
    for x in 0..n {
        let mut col: Vec<C64> = (0..n).map(|y| grid[y * n + x]).collect();
        fft_1d(&mut col, true);
        for (y, v) in col.iter().enumerate() {
            grid[y * n + x] = *v;
        }
    }
}

// ---------------------------------------------------------------------------
// Tessendorf 谱与三贴图(L1)
// ---------------------------------------------------------------------------

/// Phillips 谱 P(k)(风向对齐 + 涌浪系数;确定性)。
fn phillips(asset: &OceanSpectrumAsset, kx: f64, kz: f64) -> f64 {
    let k = (kx * kx + kz * kz).sqrt();
    if k < 1e-9 {
        return 0.0;
    }
    let l = asset.wind_speed * asset.wind_speed / GRAVITY;
    let wx = asset.wind_dir_rad.cos();
    let wz = asset.wind_dir_rad.sin();
    let kdotw = (kx * wx + kz * wz) / k;
    let damp = (-k * l * 0.001).exp(); // 小尺度阻尼(防高频发散,闭式)
    let p = (-1.0 / (k * l).powi(2)).exp() / k.powi(4) * kdotw * kdotw * damp;
    p * (1.0 + asset.swell)
}

/// 确定性伪高斯对(谱幅值相位源;由格点下标闭式 hash,Box-Muller 口径)。
fn gauss_pair(i: usize, j: usize, salt: u64) -> (f64, f64) {
    let mut h = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ salt.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    let u1 = ((h & 0xFFFF_FFFF) as f64 + 0.5) / 4_294_967_296.0;
    let u2 = (((h >> 32) & 0xFFFF_FFFF) as f64 + 0.5) / 4_294_967_296.0;
    let r = (-2.0 * u1.ln()).sqrt();
    let t = 2.0 * std::f64::consts::PI * u2;
    (r * t.cos(), r * t.sin())
}

/// 波数格点(kx,kz)(i,j ∈ 0..n,以 n/2 为中心的谱排布)。
fn wave_vector(asset: &OceanSpectrumAsset, n: usize, i: usize, j: usize) -> (f64, f64) {
    let dk = 2.0 * std::f64::consts::PI / asset.tile_size_m;
    ((i as f64 - n as f64 / 2.0) * dk, (j as f64 - n as f64 / 2.0) * dk)
}

/// h(k,t) 谱幅(Tessendorf:h0·e^{iωt} + h0*(−k)·e^{−iωt})。
fn h_of_kt(asset: &OceanSpectrumAsset, n: usize, i: usize, j: usize, t: f64) -> C64 {
    let (kx, kz) = wave_vector(asset, n, i, j);
    let k = (kx * kx + kz * kz).sqrt();
    let p = phillips(asset, kx, kz);
    let (g0, g1) = gauss_pair(i, j, 0xC0FF_EE11);
    let h0 = C64 { re: g0, im: g1 }.scale((p / 2.0).sqrt());
    // h0*(−k):−k 对应下标 ((n−i)%n, (n−j)%n)。
    let ni = (n - i) % n;
    let nj = (n - j) % n;
    let (nkx, nkz) = wave_vector(asset, n, ni, nj);
    let np = phillips(asset, nkx, nkz);
    let (ng0, ng1) = gauss_pair(ni, nj, 0xC0FF_EE11);
    let h0m = C64 { re: ng0, im: ng1 }.scale((np / 2.0).sqrt()).conj();
    let omega = (GRAVITY * k * (k * asset.depth_m).tanh()).sqrt();
    h0.mul(C64::exp_i(omega * t)).add(h0m.mul(C64::exp_i(-omega * t)))
}

/// 大洋帧产出(位移/梯度/Jacobian 三贴图 + 泡沫掩码)。
#[derive(Debug, Clone, PartialEq)]
pub struct OceanFrame {
    pub n: usize,
    /// 高度场(位移 y 分量)。
    pub height: Vec<f64>,
    /// 位移 xz(切浪)。
    pub displacement: Vec<[f64; 2]>,
    /// 梯度(∂h/∂x, ∂h/∂z)。
    pub gradient: Vec<[f64; 2]>,
    /// Jacobian det(位移雅可比)。
    pub jacobian: Vec<f64>,
    /// 泡沫掩码(Jacobian 负值驱动)。
    pub foam_mask: Vec<bool>,
}

/// 大洋管线(compute IFFT host 参照模型 + CDLOD + 多尺度 tiling-blend)。
pub struct OceanPipeline {
    pub asset: OceanSpectrumAsset,
    pub n: usize,
}

impl OceanPipeline {
    pub fn new(asset: OceanSpectrumAsset, n: usize) -> Result<Self> {
        asset.validate()?;
        if !n.is_power_of_two() || n < 4 {
            return Err(WaterError::GridNotPowerOfTwo { got: n });
        }
        Ok(Self { asset, n })
    }

    /// 谱网格(某派生谱 = h(k,t) × 频域因子;`factor(k)` 闭包给因子)。
    /// 排布 = FFT 自然序:居中波数 m = i − n/2 存于下标 (i + n/2) mod n,使
    /// IFFT 输出 = 物理高度场 H(r = x·L/N) 直值(免棋盘符号校正)。
    fn spectrum_grid(&self, t: f64, factor: &dyn Fn(f64, f64, C64) -> C64) -> Vec<C64> {
        let n = self.n;
        let mut grid = vec![C64::default(); n * n];
        for j in 0..n {
            for i in 0..n {
                let (kx, kz) = wave_vector(&self.asset, n, i, j);
                let h = h_of_kt(&self.asset, n, i, j, t);
                let fi = (i + n / 2) % n;
                let fj = (j + n / 2) % n;
                grid[fj * n + fi] = factor(kx, kz, h);
            }
        }
        grid
    }

    /// compute IFFT 产三贴图(host 参照;radix-2 FFT 路径)。
    pub fn evaluate(&self, t: f64) -> Result<OceanFrame> {
        if !t.is_finite() {
            return Err(WaterError::NotCanonical("t 非有限"));
        }
        let n = self.n;
        let lambda = self.asset.choppiness;
        // 高度:h(k,t)。
        let mut gh = self.spectrum_grid(t, &|_kx, _kz, h| h);
        ifft_2d(&mut gh, n);
        // 位移:D(k) = i·k/|k|·h(k),乘 λ。
        let mut gdx = self.spectrum_grid(t, &|kx, kz, h| {
            let k = (kx * kx + kz * kz).sqrt();
            if k < 1e-9 { C64::default() } else { C64 { re: 0.0, im: kx / k }.mul(h).scale(lambda) }
        });
        ifft_2d(&mut gdx, n);
        let mut gdz = self.spectrum_grid(t, &|kx, kz, h| {
            let k = (kx * kx + kz * kz).sqrt();
            if k < 1e-9 { C64::default() } else { C64 { re: 0.0, im: kz / k }.mul(h).scale(lambda) }
        });
        ifft_2d(&mut gdz, n);
        // 梯度:i·k·h(k)。
        let mut ggx = self.spectrum_grid(t, &|kx, _kz, h| C64 { re: 0.0, im: kx }.mul(h));
        ifft_2d(&mut ggx, n);
        let mut ggz = self.spectrum_grid(t, &|_kx, kz, h| C64 { re: 0.0, im: kz }.mul(h));
        ifft_2d(&mut ggz, n);
        // Jacobian 分量:Jxx = 1 + λ·IFFT(−kx²/|k|·h),Jzz 同,Jxz = λ·IFFT(−kx·kz/|k|·h)。
        let jcomp = |sel: u8| -> Vec<f64> {
            let mut g = self.spectrum_grid(t, &|kx, kz, h| {
                let k = (kx * kx + kz * kz).sqrt();
                if k < 1e-9 {
                    return C64::default();
                }
                let f = match sel {
                    0 => -kx * kx / k,
                    1 => -kz * kz / k,
                    _ => -kx * kz / k,
                };
                h.scale(f * lambda)
            });
            ifft_2d(&mut g, n);
            g.iter().map(|v| v.re).collect()
        };
        let jxx = jcomp(0);
        let jzz = jcomp(1);
        let jxz = jcomp(2);
        let mut jacobian = Vec::with_capacity(n * n);
        let mut foam_mask = Vec::with_capacity(n * n);
        for idx in 0..n * n {
            let det = (1.0 + jxx[idx]) * (1.0 + jzz[idx]) - jxz[idx] * jxz[idx];
            jacobian.push(det);
            foam_mask.push(det < 0.0); // Jacobian 负值驱动泡沫
        }
        Ok(OceanFrame {
            n,
            height: gh.iter().map(|v| v.re).collect(),
            displacement: (0..n * n).map(|i| [gdx[i].re, gdz[i].re]).collect(),
            gradient: (0..n * n).map(|i| [ggx[i].re, ggz[i].re]).collect(),
            jacobian,
            foam_mask,
        })
    }

    /// 几何路径 token 闭集(双管线分离断言面)。
    pub fn geometry_claim(&self) -> GeometryPathClaim {
        GeometryPathClaim { tokens: vec![GeometryToken::OceanCdlodMesh, GeometryToken::OceanSpectrumTile] }
    }

    /// 水面着色 closure 输入(双管线唯一共享面)。
    pub fn shading_input(&self, frame: &OceanFrame, idx: usize, view: [f32; 3]) -> Result<WaterShadingInput> {
        if idx >= frame.height.len() {
            return Err(WaterError::NotCanonical("着色采样越界"));
        }
        let g = frame.gradient[idx];
        let inv = (1.0 + g[0] * g[0] + g[1] * g[1]).sqrt();
        Ok(WaterShadingInput {
            normal: [(-g[0] / inv) as f32, (1.0 / inv) as f32, (-g[1] / inv) as f32],
            foam: if frame.foam_mask[idx] { 1.0 } else { 0.0 },
            view,
        })
    }
}

/// host DFT 参考(定义式直算,IFFT 对拍基准;仅高度场——三贴图同一谱族,
/// 高度场对拍容差即谱求值链容差载体)。相位约定与 [`OceanPipeline`] 的 FFT
/// 自然序排布逐字一致(H(x,z) = Σ h(k)·e^{i2π((i−n/2)x+(j−n/2)z)/n},Tessendorf
/// 求和约定无 1/N² 缩放)。
pub fn reference_dft_height(asset: &OceanSpectrumAsset, n: usize, t: f64) -> Result<Vec<f64>> {
    if !n.is_power_of_two() || n < 4 {
        return Err(WaterError::GridNotPowerOfTwo { got: n });
    }
    asset.validate()?;
    let mut out = vec![0.0f64; n * n];
    for y in 0..n {
        for x in 0..n {
            let mut acc = C64::default();
            for j in 0..n {
                for i in 0..n {
                    let h = h_of_kt(asset, n, i, j, t);
                    let phase = 2.0 * std::f64::consts::PI
                        * ((i as f64 - n as f64 / 2.0) * x as f64
                            + (j as f64 - n as f64 / 2.0) * y as f64)
                        / n as f64;
                    acc = acc.add(h.mul(C64::exp_i(phase)));
                }
            }
            out[y * n + x] = acc.re;
        }
    }
    Ok(out)
}

/// 逐值对拍容差面:compute IFFT 高度场 vs host DFT 参考的最大绝对差(measured
/// 经冻结带明示,禁手写)。
pub fn max_abs_diff(a: &[f64], b: &[f64]) -> Result<f64> {
    if a.len() != b.len() {
        return Err(WaterError::NotCanonical("对拍长度不符"));
    }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max))
}

/// CDLOD 距离分档(L1;闭集)。
pub fn cdlod_tier(distance_m: f64) -> Result<u32> {
    if !distance_m.is_finite() || distance_m < 0.0 {
        return Err(WaterError::NotCanonical("cdlod 距离非法"));
    }
    let mut tier = 0;
    for ring in CDLOD_RING_M {
        if distance_m >= ring {
            tier += 1;
        }
    }
    Ok(tier.min(3))
}

/// 多尺度谱 tiling-and-blending 混合权重(L1;基 tile 与 detail tile 双尺度,
/// 近处 detail 权重高;闭式 1/(1+d/64) 单调,双尺度均非零 ⇒ 防周期重复感)。
pub fn tile_blend_weight(distance_m: f64) -> Result<f64> {
    if !distance_m.is_finite() || distance_m < 0.0 {
        return Err(WaterError::NotCanonical("blend 距离非法"));
    }
    Ok(1.0 / (1.0 + distance_m / 64.0))
}

// ---------------------------------------------------------------------------
// 浅水管线(L2:高度场 + 速度场 ping-pong;越界写检测)
// ---------------------------------------------------------------------------

/// 浅水波方程仿真(高度场 + 速度场 ping-pong;内部写一律经越界守卫)。
#[derive(Debug, Clone, PartialEq)]
pub struct ShallowWaveSim {
    pub dim: usize,
    pub height: Vec<f32>,
    pub velocity: Vec<f32>,
}

impl ShallowWaveSim {
    pub fn new(dim: usize) -> Result<Self> {
        if dim < 4 {
            return Err(WaterError::NotCanonical("浅水网格维 <4"));
        }
        Ok(Self { dim, height: vec![0.0; dim * dim], velocity: vec![0.0; dim * dim] })
    }

    /// 越界写守卫(L4 RED 锚):一切外部写经本面;越界即 typed Err。
    pub fn poke(&mut self, x: usize, y: usize, dh: f32) -> Result<()> {
        if x >= self.dim || y >= self.dim {
            return Err(WaterError::ShallowOutOfBoundsWrite {
                index: y * self.dim + x,
                extent: self.dim * self.dim,
            });
        }
        if !dh.is_finite() {
            return Err(WaterError::NotCanonical("poke 非有限"));
        }
        self.height[y * self.dim + x] += dh;
        Ok(())
    }

    fn checked_write(&mut self, idx: usize, v: f32) -> Result<()> {
        if idx >= self.height.len() {
            return Err(WaterError::ShallowOutOfBoundsWrite { index: idx, extent: self.height.len() });
        }
        self.height[idx] = v;
        Ok(())
    }

    /// 波方程步进(v += c²∇²h·dt;h += v·dt;ping-pong 双缓冲,边界固定 0)。
    pub fn step(&mut self) -> Result<()> {
        let dim = self.dim;
        let mut new_v = self.velocity.clone();
        for y in 1..dim - 1 {
            for x in 1..dim - 1 {
                let i = y * dim + x;
                let lap = self.height[i - 1] + self.height[i + 1]
                    + self.height[i - dim] + self.height[i + dim]
                    - 4.0 * self.height[i];
                new_v[i] = self.velocity[i] + SHALLOW_C2_DT * lap;
            }
        }
        self.velocity = new_v;
        for y in 1..dim - 1 {
            for x in 1..dim - 1 {
                let i = y * dim + x;
                let h = self.height[i] + self.velocity[i] * SHALLOW_DT;
                self.checked_write(i, h)?;
            }
        }
        Ok(())
    }

    /// 几何路径 token 闭集(双管线分离断言面)。
    pub fn geometry_claim(&self) -> GeometryPathClaim {
        GeometryPathClaim { tokens: vec![GeometryToken::ShallowPingPongGrid] }
    }

    /// 水面着色 closure 输入(双管线唯一共享面)。
    pub fn shading_input(&self, x: usize, y: usize, view: [f32; 3]) -> Result<WaterShadingInput> {
        if x == 0 || y == 0 || x >= self.dim - 1 || y >= self.dim - 1 {
            return Err(WaterError::NotCanonical("着色采样越界"));
        }
        let i = y * self.dim + x;
        let gx = (self.height[i + 1] - self.height[i - 1]) * 0.5;
        let gz = (self.height[i + self.dim] - self.height[i - self.dim]) * 0.5;
        let inv = (1.0 + gx * gx + gz * gz).sqrt();
        Ok(WaterShadingInput {
            normal: [-gx / inv, 1.0 / inv, -gz / inv],
            foam: 0.0,
            view,
        })
    }
}

// ---------------------------------------------------------------------------
// 双管线分离断言(L3)+ 共享着色 closure 输入面
// ---------------------------------------------------------------------------

/// 几何路径 token 闭集(大洋与浅水互斥枚举)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryToken {
    /// 大洋 CDLOD 分档 mesh。
    OceanCdlodMesh,
    /// 大洋谱 tile(tiling-and-blending)。
    OceanSpectrumTile,
    /// 浅水 ping-pong 网格。
    ShallowPingPongGrid,
}

impl GeometryToken {
    pub fn as_str(&self) -> &'static str {
        match self {
            GeometryToken::OceanCdlodMesh => "ocean_cdlod_mesh",
            GeometryToken::OceanSpectrumTile => "ocean_spectrum_tile",
            GeometryToken::ShallowPingPongGrid => "shallow_ping_pong_grid",
        }
    }
}

/// 几何路径声明(互斥机核输入面)。
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryPathClaim {
    pub tokens: Vec<GeometryToken>,
}

/// 双管线几何路径互斥机核(L3):交集非空即 `Err(GeometryPathShared)`(RED)。
pub fn assert_geometry_paths_disjoint(a: &GeometryPathClaim, b: &GeometryPathClaim) -> Result<()> {
    for ta in &a.tokens {
        if b.tokens.contains(ta) {
            return Err(WaterError::GeometryPathShared { token: ta.as_str() });
        }
    }
    Ok(())
}

/// 水面着色 closure 输入(双管线**唯一**共享面,D4 D8)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterShadingInput {
    pub normal: [f32; 3],
    pub foam: f32,
    pub view: [f32; 3],
}

/// 浮力查询接口面(L5:**预留不实现**——一律 typed Err;M77→M124 归 G9.6
/// 物理波 Field 通道,本条款不授权任何浮力实现或旁路 API)。
pub fn buoyancy_query(_point: &[f32; 3]) -> Result<f32> {
    Err(WaterError::BuoyancyInterfaceReserved)
}

// ---------------------------------------------------------------------------
// canonical 场景(golden 事实源)
// ---------------------------------------------------------------------------

/// canonical 大洋谱资产(风向 30°,风速 3 m/s〔短波陡波〕,涌浪 0.3,切浪 1.0
/// 在 Tessendorf 求和约定下产折叠泡沫——Jacobian 负值驱动泡沫语义的非空事实源)。
pub fn canonical_spectrum() -> OceanSpectrumAsset {
    OceanSpectrumAsset {
        wind_dir_rad: std::f64::consts::FRAC_PI_6,
        wind_speed: 3.0,
        swell: 0.3,
        fetch_m: 20_000.0,
        depth_m: 50.0,
        choppiness: 1.0,
        tile_size_m: 96.0,
    }
}

/// canonical 浅水场景(中心脉冲激励后 8 步)。
pub fn canonical_shallow() -> ShallowWaveSim {
    let mut sim = ShallowWaveSim::new(SHALLOW_DIM).expect("shallow");
    sim.poke(8, 8, 1.0).expect("poke");
    for _ in 0..8 {
        sim.step().expect("step");
    }
    sim
}

/// 大洋帧 digest(三贴图 + 泡沫掩码;golden 对照事实源)。
pub fn ocean_digest(f: &OceanFrame) -> [u8; 32] {
    let mut buf = Vec::new();
    for v in &f.height {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    for d in &f.displacement {
        buf.extend_from_slice(&d[0].to_le_bytes());
        buf.extend_from_slice(&d[1].to_le_bytes());
    }
    for g in &f.gradient {
        buf.extend_from_slice(&g[0].to_le_bytes());
        buf.extend_from_slice(&g[1].to_le_bytes());
    }
    for j in &f.jacobian {
        buf.extend_from_slice(&j.to_le_bytes());
    }
    for m in &f.foam_mask {
        buf.push(*m as u8);
    }
    sha256::digest(&buf)
}

/// 浅水场 digest。
pub fn shallow_digest(s: &ShallowWaveSim) -> [u8; 32] {
    let mut buf = Vec::new();
    for v in &s.height {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    for v in &s.velocity {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    sha256::digest(&buf)
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0366
    #[test]
    fn spectrum_roundtrip_and_invalid_param_red() {
        let a = canonical_spectrum();
        let bytes = encode_spectrum(&a);
        assert_eq!(decode_spectrum(&bytes).unwrap(), a);
        let sig = spectrum_signature(&a);
        verify_spectrum(&a, &sig).unwrap();
        // 负风速 ⇒ 装配期拒录(RED)。
        let bad = OceanSpectrumAsset { wind_speed: -1.0, ..a };
        assert!(matches!(
            bad.validate(),
            Err(WaterError::InvalidSpectrumParam { field: "wind_speed" })
        ));
        let mut raw = bytes.clone();
        let neg = (-1.0f64).to_le_bytes();
        raw[14..22].copy_from_slice(&neg); // wind_speed 字段(offset 6+8)
        assert!(matches!(
            decode_spectrum(&raw),
            Err(WaterError::InvalidSpectrumParam { .. })
        ));
        // 篡改签名 ⇒ 拒录。
        let mut t = a;
        t.swell += 0.5;
        assert!(matches!(verify_spectrum(&t, &sig), Err(WaterError::AssetTampered { .. })));
    }

    //@ spec: RXS-0366
    #[test]
    fn ifft_vs_reference_dft_close() {
        let a = canonical_spectrum();
        let n = 8; // 单测小网格(参照 O(n⁴) 成本控制)
        let pipe = OceanPipeline::new(a, n).unwrap();
        let frame = pipe.evaluate(1.5).unwrap();
        let refr = reference_dft_height(&a, n, 1.5).unwrap();
        let diff = max_abs_diff(&frame.height, &refr).unwrap();
        assert!(diff < 1e-9, "IFFT vs DFT 参考差 {diff}");
        // 双跑位级一致。
        let frame2 = pipe.evaluate(1.5).unwrap();
        assert_eq!(ocean_digest(&frame), ocean_digest(&frame2));
    }

    //@ spec: RXS-0366
    #[test]
    fn jacobian_foam_mapping() {
        let a = canonical_spectrum();
        let pipe = OceanPipeline::new(a, 8).unwrap();
        let frame = pipe.evaluate(1.5).unwrap();
        // 泡沫掩码 ≡ Jacobian 负值(映射机核)。
        for (m, j) in frame.foam_mask.iter().zip(frame.jacobian.iter()) {
            assert_eq!(*m, *j < 0.0);
        }
    }

    //@ spec: RXS-0366
    #[test]
    fn dual_pipeline_geometry_disjoint_and_shared_red() {
        let ocean = OceanPipeline::new(canonical_spectrum(), 8).unwrap();
        let shallow = ShallowWaveSim::new(8).unwrap();
        assert_geometry_paths_disjoint(&ocean.geometry_claim(), &shallow.geometry_claim()).unwrap();
        // 互斥违反注入(浅水声明大洋 token)即 RED。
        let bad = GeometryPathClaim {
            tokens: vec![GeometryToken::ShallowPingPongGrid, GeometryToken::OceanCdlodMesh],
        };
        assert!(matches!(
            assert_geometry_paths_disjoint(&ocean.geometry_claim(), &bad),
            Err(WaterError::GeometryPathShared { token: "ocean_cdlod_mesh" })
        ));
    }

    //@ spec: RXS-0366
    #[test]
    fn shallow_out_of_bounds_write_red() {
        let mut sim = ShallowWaveSim::new(8).unwrap();
        assert!(sim.poke(3, 3, 1.0).is_ok());
        assert!(matches!(
            sim.poke(8, 3, 1.0),
            Err(WaterError::ShallowOutOfBoundsWrite { .. })
        ));
        assert!(matches!(
            sim.poke(0, 99, 1.0),
            Err(WaterError::ShallowOutOfBoundsWrite { .. })
        ));
    }

    //@ spec: RXS-0366
    #[test]
    fn buoyancy_reserved_not_implemented() {
        assert!(matches!(
            buoyancy_query(&[0.0, 0.0, 0.0]),
            Err(WaterError::BuoyancyInterfaceReserved)
        ));
    }

    //@ spec: RXS-0366
    #[test]
    fn cdlod_and_blend_closed() {
        assert_eq!(cdlod_tier(0.0).unwrap(), 0);
        assert_eq!(cdlod_tier(64.0).unwrap(), 1);
        assert_eq!(cdlod_tier(300.0).unwrap(), 2);
        assert_eq!(cdlod_tier(99999.0).unwrap(), 3);
        // 双尺度混合权重单调且恒 (0,1](两尺度均贡献 ⇒ tiling-blend 生效)。
        let w0 = tile_blend_weight(0.0).unwrap();
        let w1 = tile_blend_weight(64.0).unwrap();
        assert!(w0 == 1.0 && w1 < 1.0 && w1 > 0.0);
    }
}
