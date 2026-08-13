//! G9.4 M98 四级追踪降级链 host 面(spec/global_illumination.md RXS-0359;
//! RFC-0022 §4.7;门 `g9.p0.m98.tracing_fallback_chain`)。
//!
//! 本模块 = 降级链的 **host 数据面/选档器/计数面/审计面**:
//! - **四级链**(RXS-0359 L1 冻结,RFC-0022 §4.7 表):L1 Screen Trace(屏幕空间
//!   高度场 ray march,~50 m 内、屏幕内;本模块 host 参照 + device kernel
//!   `kernels/g9_m98_screen_trace.rx` 逐字同源)→ L2 SWRT(软件 ray tracing:
//!   host 解析场景暴力求值腿,`rt::bvh` BVH 金标准对拍;~200 m)→ L3 HWRT
//!   (Vulkan RayQuery 对 TLAS;命中着色两档 = 简单兜底求值〔默认〕/ hit
//!   lighting 完整材质求值〔NEE+Lambert+阴影〕;device kernel
//!   `kernels/g9_m98_hwrt.rx`)→ L4 Far Field(HLOD 接口未就绪〔G9.5 M111〕:
//!   [`check_l4_trigger`] fail-closed 判 not-triggered 并显式登记,**不充绿**;
//!   [`l4_serve`] 恒 typed Err,禁静默当绿)。
//! - **选档契约**(RXS-0359 L2):逐像素按命中距离与覆盖优先级选档——L1 命中且
//!   t ≤ [`M98_L1_RANGE`] → L1;否则 L2 命中且 t ≤ [`M98_L2_RANGE`] → L2;否则
//!   L3(命中 → 按档着色,未命中 → 天空常量);全不可用 → 显式 Unserved 终端
//!   天空(flag=0,非静默——终端状态入 flags 与转移日志)。
//! - **逐档计数面**(RXS-0359 L2):每级独立开关([`ChainSwitches`])+ 独立
//!   evidence 计数面(命中率/射线量/耗时),逐帧导出;耗时 = 确定性代理计数
//!   (L1 march 步数 / L2 三角测试数 / L3 有效射线查询发行量 1+hit)+ host 壁钟
//!   ns 信息项(代理口径显式登记,禁手写)。
//! - **禁静默回退**(RXS-0359 L4):每次级别转移必须产生显式
//!   [`TransitionRecord`](含原因 Miss/OutOfRange/ForcedOff);装配后
//!   [`audit`] 独立重算期望转移集合并逐条比对——无记录的级别变化 =
//!   fail-closed [`FbError::SilentFallback`](静默回退 variant 必红,对接
//!   conformance/gi/reject/tracing_fallback_silent_demotion.rx 负例臂①)。
//! - **逐级强关回归可检测**(RXS-0359 L3):产物 digest = sha256(rgb‖flags)
//!   (flags 携带实际服务级别——级别转移必然改变产物 digest,结构性保证强关
//!   回归可检测;强关后输出仍同 golden 即 RED,负例臂②)。
//! - **golden 对拍**(RXS-0359 L6):各档按匹配深度(full chain 与逐档 solo 均
//!   以 1 次间接弹射 ⇒ 匹配深度 [`M98_MATCHED_DEPTH`])对 M96 golden,
//!   [`M98DepthBand`] measured 后冻结(milestones/g9/g9_m98_depth_band.json;
//!   带 = measured × [`M98_BAND_MARGIN`],禁手写 P-09);M96 同深度 digest 与
//!   M97 冻结带 `m96_cornell` 条目逐字相等(D2-Q7 门序消费锚)。
//!
//! ## 确定性协议(承 RXS-0357 L2 同律)
//! - 场景 = M96 冻结 fixture [`path_trace::m96_cornell_scene`];二次射线方向
//!   流 = PCG32 单一流按索引寻址([`m98_rng`],流为输入非结果,G7.4 先例)。
//! - 主光线 = 像素中心(无 jitter);GBuffer(深度=视 z、法线、albedo、直接
//!   光)为 host 预传递**输入**(与 RNG 流同纪律,host oracle 产输入、device
//!   消费);逐像素独立顺序累加,禁 atomic 顺序敏感累加。
//! - 全部 f32;device kernel 分支判定一律 min/max 算术门 + 短 selection 臂
//!   (M96 已机验白名单形),host 公式面与 kernel 逐字同源。

use crate::gi::path_trace::{self, PtScene};
use crate::rt::ref_tracer::RAY_EPS;

// ---------------------------------------------------------------------------
// 冻结常量(选档阈值/档位参数;实现确定、非 stable,RFC-0022 §10 口径)
// ---------------------------------------------------------------------------

/// M98 确定性协议冻结 seed(独立于 M96/M97 流,避免跨里程碑流耦合)。
pub const M98_SEED: u64 = 0x5A98_3C11_F0E7_D249;
/// L1 Screen Trace 覆盖上界(场景单位;~50 m 语义按 M96 cornell 冻结 fixture
/// 尺度缩放——单位盒内二次射线命中距离域,阈值先 measured 后冻结)。
pub const M98_L1_RANGE: f32 = 0.5;
/// L2 SWRT 覆盖上界(场景单位;~200 m 语义同尺度缩放)。
pub const M98_L2_RANGE: f32 = 1.0;
/// L1 高度场 march 固定采样步数(屏幕线段均匀采样;自像素经同像素门跳过)。
pub const M98_L1_MAX_STEPS: u32 = 32;
/// L1 深度穿越判定偏置(视 z 域;防自交/伪命中)。
pub const M98_L1_DEPTH_BIAS: f32 = 0.01;
/// 未命中/终端天空常量辐射度(线性 RGB;沿 M97 ambient 口径)。
pub const M98_SKY: [f32; 3] = [0.02, 0.02, 0.02];
/// 视距上界:命中 t 超出 ⇒ 本应升级 L4 Far Field;L4 接口未就绪 ⇒
/// fail-closed [`FbError::L4InterfaceNotReady`](禁静默当绿)。冻结 fixture
/// 内命中 t ≪ 本值 ⇒ L4 候选恒 0,登记 not-triggered。
pub const M98_VIEW_DIST: f32 = 1000.0;
/// 匹配深度(次光线 1 次间接弹射 ⇒ M96 max_bounces=2 档 golden)。
pub const M98_MATCHED_DEPTH: u32 = 2;
/// M96 golden 参照 spp(与 M97 门序锚同档)。
pub const M98_M96_GOLDEN_SPP: u32 = 64;
/// 容差带倍率(band = measured × margin;禁手写,P-09;沿 M96/M97 口径)。
pub const M98_BAND_MARGIN: f64 = 2.0;
/// 大数门乘子(与 M96 kernel `big` 位级同值)。
const BIG: f32 = 1e30;
/// 正下界(与 M96 kernel `tiny` 位级同值)。
const TINY: f32 = 0.000001;

// ---------------------------------------------------------------------------
// 错误面(fail-closed typed Err;本模块一切失败为类型化拒绝,严禁 UB)
// ---------------------------------------------------------------------------

/// M98 host 面错误(选档/装配/审计/容差带全部 fail-closed)。
#[derive(Debug, Clone, PartialEq)]
pub enum FbError {
    /// 配置/输入非法(长度不符/阈值非有限/像素数为零等)。
    InvalidConfig(String),
    /// 静默回退检出:实际服务级别发生转移但转移日志缺失/不符
    /// (RXS-0359 L4「无计数降级即 RED」;负例臂①承载)。
    SilentFallback(String),
    /// L4 Far Field 被请求服务但 HLOD 接口未就绪(RXS-0359 L5:
    /// 只登记 SKIP=not-triggered,禁静默当绿)。
    L4InterfaceNotReady(String),
    /// 深度容差带错误(解析/缺条目/digest 不符/越带)。
    DepthBand(String),
}

impl std::fmt::Display for FbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FbError::InvalidConfig(m) => write!(f, "配置非法: {m}"),
            FbError::SilentFallback(m) => write!(f, "静默回退检出(无计数降级即 RED): {m}"),
            FbError::L4InterfaceNotReady(m) => write!(f, "L4 Far Field 接口未就绪(not-triggered): {m}"),
            FbError::DepthBand(m) => write!(f, "深度容差带: {m}"),
        }
    }
}

impl std::error::Error for FbError {}

// ---------------------------------------------------------------------------
// 级别与开关(RXS-0359 L1/L2)
// ---------------------------------------------------------------------------

/// 追踪级别(四级链冻结;index = flags/evidence 编码)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceLevel {
    /// L1 Screen Trace(屏幕空间高度场 ray march;~50 m 内、屏幕内)。
    L1ScreenTrace,
    /// L2 SWRT(软件 ray tracing;~200 m)。
    L2Swrt,
    /// L3 HWRT(Vulkan RayQuery 对 TLAS;含 hit lighting 档)。
    L3Hwrt,
    /// L4 Far Field(远场代理辐射度;HLOD 接口未就绪 ⇒ not-triggered)。
    L4FarField,
}

impl TraceLevel {
    /// 可选档链序(L1→L2→L3;L4 不在选档器内——接口未就绪,见
    /// [`check_l4_trigger`])。
    pub const SELECTABLE: [TraceLevel; 3] = [
        TraceLevel::L1ScreenTrace,
        TraceLevel::L2Swrt,
        TraceLevel::L3Hwrt,
    ];
    /// 计数面全四级(L4 行恒零 + not-triggered 登记)。
    pub const ALL: [TraceLevel; 4] = [
        TraceLevel::L1ScreenTrace,
        TraceLevel::L2Swrt,
        TraceLevel::L3Hwrt,
        TraceLevel::L4FarField,
    ];

    /// flags/evidence 编码(1..=4;0 = Unserved 终端)。
    pub fn flag(self) -> f32 {
        match self {
            TraceLevel::L1ScreenTrace => 1.0,
            TraceLevel::L2Swrt => 2.0,
            TraceLevel::L3Hwrt => 3.0,
            TraceLevel::L4FarField => 4.0,
        }
    }

    /// 计数面数组下标(0..=3)。
    pub fn slot(self) -> usize {
        match self {
            TraceLevel::L1ScreenTrace => 0,
            TraceLevel::L2Swrt => 1,
            TraceLevel::L3Hwrt => 2,
            TraceLevel::L4FarField => 3,
        }
    }

    /// evidence 名(稳定字面)。
    pub fn name(self) -> &'static str {
        match self {
            TraceLevel::L1ScreenTrace => "l1_screen_trace",
            TraceLevel::L2Swrt => "l2_swrt",
            TraceLevel::L3Hwrt => "l3_hwrt",
            TraceLevel::L4FarField => "l4_far_field",
        }
    }
}

/// Unserved 终端 flag(全可选档不可用/未服务;显式终端态,非静默)。
pub const FLAG_UNSERVED: f32 = 0.0;

/// L3 命中着色两档(RXS-0359 L1:简单兜底求值〔默认〕/ hit lighting 完整
/// 材质求值〔高档〕)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L3ShadeMode {
    /// 简单兜底求值:点光源近似(光源中心)+ 阴影 + 天空 ambient 项。
    Simple,
    /// hit lighting 完整材质求值:NEE(流采样光源 quad 点)+ Lambert + 阴影。
    HitLighting,
}

impl L3ShadeMode {
    /// 参数编码(0/1;kernel params[1])。
    pub fn as_f32(self) -> f32 {
        match self {
            L3ShadeMode::Simple => 0.0,
            L3ShadeMode::HitLighting => 1.0,
        }
    }

    /// evidence 名(稳定字面)。
    pub fn name(self) -> &'static str {
        match self {
            L3ShadeMode::Simple => "simple",
            L3ShadeMode::HitLighting => "hit_lighting",
        }
    }
}

/// 逐级独立开关(每级可强制关闭;L4 不在此——经 [`check_l4_trigger`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainSwitches {
    /// L1 启用。
    pub l1: bool,
    /// L2 启用。
    pub l2: bool,
    /// L3 启用。
    pub l3: bool,
}

impl ChainSwitches {
    /// 全开(生产默认)。
    pub const ALL_ON: ChainSwitches = ChainSwitches {
        l1: true,
        l2: true,
        l3: true,
    };

    /// 某可选档是否启用(L4 恒 false——接口未就绪)。
    pub fn enabled(&self, level: TraceLevel) -> bool {
        match level {
            TraceLevel::L1ScreenTrace => self.l1,
            TraceLevel::L2Swrt => self.l2,
            TraceLevel::L3Hwrt => self.l3,
            TraceLevel::L4FarField => false,
        }
    }
}

// ---------------------------------------------------------------------------
// 腿样本与转移记录(选档器输入/日志面)
// ---------------------------------------------------------------------------

/// 单腿单像素求值结果(设备/host 腿统一回读形)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegSample {
    /// 几何命中。
    pub hit: bool,
    /// 命中距离(世界 t;未命中 = 0)。
    pub t: f32,
    /// 命中点辐射度(该腿着色公式;未命中 = 天空常量)。
    pub rgb: [f32; 3],
    /// 确定性耗时代理计数(L1 = march 步数;L2 = 三角测试数;L3 = 有效射线
    /// 查询发行量〔主查询 1 + 命中时阴影查询 1〕;口径显式登记)。
    pub work: u32,
}

/// 转移原因(选档契约字面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionCause {
    /// 该级几何未命中(L1 含射线离屏)。
    Miss,
    /// 命中但超出该级覆盖距离上界。
    OutOfRange,
    /// 该级被强制关闭(逐级强关 RED 臂锚)。
    ForcedOff,
}

impl TransitionCause {
    /// evidence 名(稳定字面)。
    pub fn name(self) -> &'static str {
        match self {
            TransitionCause::Miss => "miss",
            TransitionCause::OutOfRange => "out_of_range",
            TransitionCause::ForcedOff => "forced_off",
        }
    }
}

/// 级别转移记录(禁静默回退的显式日志;RXS-0359 L4)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionRecord {
    /// 像素下标。
    pub pixel: u32,
    /// 转移出级。
    pub from: TraceLevel,
    /// 转移入级。
    pub to: TraceLevel,
    /// 原因。
    pub cause: TransitionCause,
}

// ---------------------------------------------------------------------------
// 逐档计数面(命中率/射线量/耗时,逐帧导出;RXS-0359 L2)
// ---------------------------------------------------------------------------

/// 单级单帧计数面(全部非空导出;零值显式)。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LevelCounters {
    /// 尝试射线量(该级腿批量执行的链内像素数;级关 = 0)。
    pub rays_attempted: u64,
    /// 几何命中射线量(不限覆盖距离)。
    pub rays_hit: u64,
    /// 实际服务像素数(选档结果)。
    pub pixels_served: u64,
    /// 确定性耗时代理计数合计(口径见 [`LegSample::work`])。
    pub work_count: u64,
    /// host 壁钟 ns(信息项,非确定性判据)。
    pub wall_ns: u64,
}

impl LevelCounters {
    /// 命中率(hit/attempted;attempted=0 ⇒ 0.0 显式)。
    pub fn hit_rate(&self) -> f64 {
        if self.rays_attempted == 0 {
            0.0
        } else {
            self.rays_hit as f64 / self.rays_attempted as f64
        }
    }
}

/// 一帧装配产物(rgb + 逐像素实际服务级别 flags + 四级计数 + 转移日志)。
#[derive(Debug, Clone, PartialEq)]
pub struct ChainFrame {
    /// 图宽。
    pub width: u32,
    /// 图高。
    pub height: u32,
    /// 合成辐射度 RGB(3 f32/px;直接光 + albedo × 选档腿辐射度)。
    pub rgb: Vec<f32>,
    /// 逐像素实际服务级别(1..=4;0 = Unserved 终端天空)。
    pub flags: Vec<f32>,
    /// 四级计数面(按 [`TraceLevel::slot`])。
    pub counters: [LevelCounters; 4],
    /// 级别转移日志(像素序 × 链序;禁静默回退的显式记录)。
    pub transitions: Vec<TransitionRecord>,
}

impl ChainFrame {
    /// 产物 digest = sha256(rgb 字节 ‖ flags 字节)(flags 携带实际服务级别——
    /// 级别转移必然改变产物 digest,结构性保证逐级强关回归可检测,RXS-0359 L3)。
    pub fn product_digest(&self) -> [u8; 32] {
        let mut pre = Vec::with_capacity(self.rgb.len() * 4 + self.flags.len() * 4);
        for v in &self.rgb {
            pre.extend_from_slice(&v.to_le_bytes());
        }
        for v in &self.flags {
            pre.extend_from_slice(&v.to_le_bytes());
        }
        rurix_pkg::sha256::digest(&pre)
    }

    /// 使用日志 digest = sha256(flags 字节 ‖ 逐转移记录编码)(实际使用级别
    /// 显式记录的 provenance 锚;RXS-0359 L4)。
    pub fn usage_log_digest(&self) -> [u8; 32] {
        let mut pre = Vec::new();
        for v in &self.flags {
            pre.extend_from_slice(&v.to_le_bytes());
        }
        for r in &self.transitions {
            pre.extend_from_slice(&r.pixel.to_le_bytes());
            pre.push(r.from.flag() as u8);
            pre.push(r.to.flag() as u8);
            pre.push(match r.cause {
                TransitionCause::Miss => 0,
                TransitionCause::OutOfRange => 1,
                TransitionCause::ForcedOff => 2,
            });
        }
        rurix_pkg::sha256::digest(&pre)
    }
}

// ---------------------------------------------------------------------------
// RNG 流(PCG32 单一流按索引寻址;流为输入非结果)
// ---------------------------------------------------------------------------

/// M98 流布局(冻结):逐像素 4 维 [bsdf_r1, bsdf_r2, nee_u, nee_v]——
/// 前二维产二次射线余弦加权方向,后二维供 L3 hit lighting NEE 光源采样。
pub mod m98_rng {
    use crate::rt::ref_tracer::Pcg32;

    /// 每像素随机维数。
    pub const DIMS_PER_PIXEL: usize = 4;

    /// 流总长(= pixel_count · 4)。
    pub fn stream_len(pixel_count: usize) -> usize {
        pixel_count * DIMS_PER_PIXEL
    }

    /// 像素流起始下标。
    pub fn pixel_base(pixel: usize) -> usize {
        pixel * DIMS_PER_PIXEL
    }

    /// 生成整条流(单 [`Pcg32`] 实例,图序顺序产出;承 G8 对拍模式)。
    pub fn generate_stream(pixel_count: usize, seed: u64) -> Vec<f32> {
        let mut rng = Pcg32::new(seed);
        let mut out = Vec::with_capacity(stream_len(pixel_count));
        for _ in 0..stream_len(pixel_count) {
            out.push(rng.next_f32());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// 共享数值核(host/kernel 逐字同源;改公式 ⇒ 双端同步)
// ---------------------------------------------------------------------------

/// 余弦加权半球方向(与 M96 kernel shade③ 逐字同式:t = normalize(cross(up, n)),
/// up 选择 = (0.999 − |n.y|) 门;返回单位向量,保证 n·d ≥ 0)。
#[allow(clippy::manual_clamp)] // 算术门即公式面(.min/.max 序与 kernel 逐字同源;clamp 的 NaN 传播语义不同,禁改写)
pub fn cosine_dir(n: [f32; 3], r1: f32, r2: f32) -> [f32; 3] {
    let pi = path_trace::PT_PI;
    let phi = 2.0 * pi * r1;
    let rr2 = r2.sqrt();
    let lx = rr2 * phi.cos();
    let ly = rr2 * phi.sin();
    let lz = (1.0 - r2).max(0.0).sqrt();
    let up_sel = ((0.999 - n[1].abs()) * BIG).min(1.0).max(0.0);
    let upx = 1.0 - up_sel;
    let upy = up_sel;
    let t1x = upy * n[2];
    let t1y = -upx * n[2];
    let t1z = upx * n[1] - upy * n[0];
    let t1l = 1.0 / (t1x * t1x + t1y * t1y + t1z * t1z).sqrt();
    let tx = t1x * t1l;
    let ty = t1y * t1l;
    let tz = t1z * t1l;
    let bx = n[1] * tz - n[2] * ty;
    let by = n[2] * tx - n[0] * tz;
    let bz = n[0] * ty - n[1] * tx;
    let ndx = tx * lx + bx * ly + n[0] * lz;
    let ndy = ty * lx + by * ly + n[1] * lz;
    let ndz = tz * lx + bz * ly + n[2] * lz;
    let inv = 1.0 / (ndx * ndx + ndy * ndy + ndz * ndz).sqrt();
    [ndx * inv, ndy * inv, ndz * inv]
}

/// 点光源近似核心(全档共享):core = cos_s·cos_l·area/(π·dist²),
/// cos_s = max(n·wi, 0)(n 已朝入射侧翻转)、cos_l = max(−ln·wi, 0)(光源单面)。
/// 返回 irradiance 核心标量(乘以 albedo×emission 即出射辐射度)。
pub fn point_light_core(p: [f32; 3], n: [f32; 3], q: [f32; 3], scene: &PtScene) -> f32 {
    let l = &scene.light;
    let ln = l.normal();
    let wvx = q[0] - p[0];
    let wvy = q[1] - p[1];
    let wvz = q[2] - p[2];
    let dist2 = wvx * wvx + wvy * wvy + wvz * wvz;
    let dist = dist2.sqrt().max(TINY);
    let wix = wvx / dist;
    let wiy = wvy / dist;
    let wiz = wvz / dist;
    let cos_s = (n[0] * wix + n[1] * wiy + n[2] * wiz).max(0.0);
    let cos_l = (-(ln[0] * wix + ln[1] * wiy + ln[2] * wiz)).max(0.0);
    cos_s * cos_l * l.area() / (path_trace::PT_PI * dist2)
}

/// 光源中心(简单兜底求值的点近似锚)。
pub fn light_center(scene: &PtScene) -> [f32; 3] {
    let l = &scene.light;
    [
        l.p00[0] + 0.5 * l.e1[0] + 0.5 * l.e2[0],
        l.p00[1] + 0.5 * l.e1[1] + 0.5 * l.e2[1],
        l.p00[2] + 0.5 * l.e1[2] + 0.5 * l.e2[2],
    ]
}

/// 光源 quad 确定性采样点(hit lighting 档;q = p00 + u·e1 + v·e2,流采样)。
pub fn light_sample(scene: &PtScene, u: f32, v: f32) -> [f32; 3] {
    let l = &scene.light;
    [
        l.p00[0] + u * l.e1[0] + v * l.e2[0],
        l.p00[1] + u * l.e1[1] + v * l.e2[1],
        l.p00[2] + u * l.e1[2] + v * l.e2[2],
    ]
}

/// 点光源近似着色(未阴影;L1 档):rgb = albedo × emission × core(q=中心)。
pub fn shade_point_unshadowed(albedo: [f32; 3], p: [f32; 3], n: [f32; 3], scene: &PtScene) -> [f32; 3] {
    let core = point_light_core(p, n, light_center(scene), scene);
    let em = scene.light.emission;
    [
        albedo[0] * em[0] * core,
        albedo[1] * em[1] * core,
        albedo[2] * em[2] * core,
    ]
}

// ---------------------------------------------------------------------------
// L2 SWRT host 腿(解析场景暴力求值;rt::bvh 金标准对拍;确定性测试计数)
// ---------------------------------------------------------------------------

/// 单三角 Möller–Trumbore(与 `rt::bvh::intersect_triangle` 逐字同式;
/// 双面命中,t ∈ (0, t_max))。
fn tri_hit(
    o: [f32; 3],
    d: [f32; 3],
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    t_max: f32,
) -> Option<f32> {
    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let p = [
        d[1] * e2[2] - d[2] * e2[1],
        d[2] * e2[0] - d[0] * e2[2],
        d[0] * e2[1] - d[1] * e2[0],
    ];
    let det = e1[0] * p[0] + e1[1] * p[1] + e1[2] * p[2];
    if det.abs() < 1e-8 {
        return None;
    }
    let inv_det = 1.0 / det;
    let s = [o[0] - a[0], o[1] - a[1], o[2] - a[2]];
    let u = (s[0] * p[0] + s[1] * p[1] + s[2] * p[2]) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = [
        s[1] * e1[2] - s[2] * e1[1],
        s[2] * e1[0] - s[0] * e1[2],
        s[0] * e1[1] - s[1] * e1[0],
    ];
    let v = (d[0] * q[0] + d[1] * q[1] + d[2] * q[2]) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = (e2[0] * q[0] + e2[1] * q[1] + e2[2] * q[2]) * inv_det;
    if t > 0.0 && t < t_max { Some(t) } else { None }
}

/// 三角形几何法线(依 winding,归一化;与 `rt::bvh::face_normal` 同式)。
fn tri_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let nx = e1[1] * e2[2] - e1[2] * e2[1];
    let ny = e1[2] * e2[0] - e1[0] * e2[2];
    let nz = e1[0] * e2[1] - e1[1] * e2[0];
    let l = (nx * nx + ny * ny + nz * nz).sqrt();
    if l > 0.0 {
        [nx / l, ny / l, nz / l]
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// 场景三角 `i` 的 (v0, v1, v2)(`positions`/`indices` 序 = device
/// primitiveIndex 序,M96 打包同律)。
pub fn scene_tri(scene: &PtScene, i: usize) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let idx = scene.indices[i];
    (
        scene.positions[idx[0] as usize],
        scene.positions[idx[1] as usize],
        scene.positions[idx[2] as usize],
    )
}

/// L2 暴力最近命中(host 解析场景求值;返回 (最近 (t, 三角号), 三角测试数))。
/// 全量扫描无早退 ⇒ 测试数 = 三角数,确定性计数面。
pub fn l2_closest_hit(scene: &PtScene, o: [f32; 3], d: [f32; 3], t_max: f32) -> (Option<(f32, u32)>, u64) {
    let mut best: Option<(f32, u32)> = None;
    let mut tests = 0u64;
    for i in 0..scene.indices.len() {
        tests += 1;
        let (a, b, c) = scene_tri(scene, i);
        if let Some(t) = tri_hit(o, d, a, b, c, t_max) {
            let better = match best {
                Some((bt, _)) => t < bt,
                None => true,
            };
            if better {
                best = Some((t, i as u32));
            }
        }
    }
    (best, tests)
}

/// L2 阴影可见性(早退 any-hit;返回 (可见 1/0, 三角测试数〔确定性〕))。
fn l2_shadow_vis(scene: &PtScene, o: [f32; 3], d: [f32; 3], t_max: f32) -> (f32, u64) {
    let mut tests = 0u64;
    for i in 0..scene.indices.len() {
        tests += 1;
        let (a, b, c) = scene_tri(scene, i);
        if tri_hit(o, d, a, b, c, t_max).is_some() {
            return (0.0, tests);
        }
    }
    (1.0, tests)
}

/// 命中三角形 albedo(Emission 材质取其 albedo 槽;范围外材质 validate 先行拒)。
fn tri_albedo(scene: &PtScene, tri: u32) -> [f32; 3] {
    match scene.materials[tri as usize] {
        path_trace::MaterialKind::Lambert { albedo } => albedo,
        path_trace::MaterialKind::Emission { albedo, .. } => albedo,
        _ => [0.0; 3],
    }
}

/// 双面着色法线(朝入射光线来向翻转;与 M96 kernel `flip` 门同式)。
#[allow(clippy::manual_clamp)] // flip 门即公式面(.min/.max 序与 kernel 逐字同源)
fn flip_normal(n: [f32; 3], d: [f32; 3]) -> [f32; 3] {
    let facing = n[0] * d[0] + n[1] * d[1] + n[2] * d[2];
    let flip = 1.0 - 2.0 * (facing * BIG).min(1.0).max(0.0);
    [n[0] * flip, n[1] * flip, n[2] * flip]
}

/// L2 SWRT 腿求值(批量;host;命中着色 = 点光源近似 × host 阴影可见性,
/// 未命中 = 天空常量)。`origins`/`dirs` 3 f32/px。
pub fn l2_leg_host(scene: &PtScene, origins: &[f32], dirs: &[f32]) -> Vec<LegSample> {
    let pixel_count = origins.len() / 3;
    let mut out = Vec::with_capacity(pixel_count);
    for i in 0..pixel_count {
        let o = [origins[i * 3], origins[i * 3 + 1], origins[i * 3 + 2]];
        let d = [dirs[i * 3], dirs[i * 3 + 1], dirs[i * 3 + 2]];
        let (best, mut tests) = l2_closest_hit(scene, o, d, scene.t_max);
        let Some((t, tri)) = best else {
            out.push(LegSample {
                hit: false,
                t: 0.0,
                rgb: M98_SKY,
                work: tests as u32,
            });
            continue;
        };
        let (a, b, c) = scene_tri(scene, tri as usize);
        let n = flip_normal(tri_normal(a, b, c), d);
        let p = [o[0] + d[0] * t, o[1] + d[1] * t, o[2] + d[2] * t];
        let q = light_center(scene);
        let core = point_light_core(p, n, q, scene);
        // 阴影光线(原点沿着色法线偏移,t_sh = dist − 2ε,M96 同式)。
        let wv = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
        let dist = (wv[0] * wv[0] + wv[1] * wv[1] + wv[2] * wv[2]).sqrt().max(TINY);
        let wi = [wv[0] / dist, wv[1] / dist, wv[2] / dist];
        let t_sh = (dist - 2.0 * RAY_EPS).max(RAY_EPS);
        let so = [p[0] + n[0] * RAY_EPS, p[1] + n[1] * RAY_EPS, p[2] + n[2] * RAY_EPS];
        let (vis, sh_tests) = l2_shadow_vis(scene, so, wi, t_sh);
        tests += sh_tests;
        let albedo = tri_albedo(scene, tri);
        let em = scene.light.emission;
        out.push(LegSample {
            hit: true,
            t,
            rgb: [
                albedo[0] * em[0] * core * vis,
                albedo[1] * em[1] * core * vis,
                albedo[2] * em[2] * core * vis,
            ],
            work: tests as u32,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// GBuffer 预传递(host 输入产线;与 RNG 流同纪律——输入是输入不是结果)
// ---------------------------------------------------------------------------

/// GBuffer(主光线像素中心无 jitter;深度 = 视 z;法线朝相机翻转;
/// 二次射线原点/方向预计算供全腿统一消费)。
#[derive(Debug, Clone, PartialEq)]
pub struct GBuffer {
    /// 图宽。
    pub width: u32,
    /// 图高。
    pub height: u32,
    /// 视 z(1 f32/px;主未命中 = 1e30 哨兵)。
    pub depth: Vec<f32>,
    /// 法线(3 f32/px,朝相机翻转)。
    pub nrm: Vec<f32>,
    /// albedo(3 f32/px)。
    pub alb: Vec<f32>,
    /// 二次射线原点(3 f32/px;主命中点 + n·RAY_EPS)。
    pub sec_o: Vec<f32>,
    /// 二次射线方向(3 f32/px;余弦加权,流采样)。
    pub sec_d: Vec<f32>,
    /// 主命中直接光(3 f32/px;点光源近似 × host 阴影;未命中 = 0)。
    pub direct: Vec<f32>,
    /// 主命中掩码。
    pub primary_hit: Vec<bool>,
    /// RNG 流(4 f32/px;[bsdf_r1, bsdf_r2, nee_u, nee_v])。
    pub stream: Vec<f32>,
}

/// GBuffer 预传递(主光线 brute-force 求值 = L2 同一解析场景面;确定性双跑
/// 位级一致)。主未命中像素:深度哨兵、direct=0、二次射线原点/方向为保底
/// 值(选档器不消费——链内像素 = 主命中像素)。
pub fn gbuffer_prepass(scene: &PtScene) -> GBuffer {
    let cam = &scene.camera;
    let width = cam.width;
    let height = cam.height;
    let pixel_count = (width * height) as usize;
    let stream = m98_rng::generate_stream(pixel_count, M98_SEED);
    let mut depth = vec![0.0f32; pixel_count];
    let mut nrm = vec![0.0f32; pixel_count * 3];
    let mut alb = vec![0.0f32; pixel_count * 3];
    let mut sec_o = vec![0.0f32; pixel_count * 3];
    let mut sec_d = vec![0.0f32; pixel_count * 3];
    let mut direct = vec![0.0f32; pixel_count * 3];
    let mut primary_hit = vec![false; pixel_count];
    for py in 0..height {
        for px in 0..width {
            let i = (py * width + px) as usize;
            // 主光线(像素中心;与 kernel ray gen 同式,jitter=0.5)。
            let ju = (px as f32 + 0.5) / width as f32;
            let jv = (py as f32 + 0.5) / height as f32;
            let sx = (2.0 * ju - 1.0) * cam.tan_half_fov;
            let sy = (1.0 - 2.0 * jv) * cam.tan_half_fov;
            let dx = cam.forward[0] + cam.right[0] * sx + cam.up[0] * sy;
            let dy = cam.forward[1] + cam.right[1] * sx + cam.up[1] * sy;
            let dz = cam.forward[2] + cam.right[2] * sx + cam.up[2] * sy;
            let inv = 1.0 / (dx * dx + dy * dy + dz * dz).sqrt();
            let d = [dx * inv, dy * inv, dz * inv];
            let (best, _tests) = l2_closest_hit(scene, cam.origin, d, scene.t_max);
            let Some((t, tri)) = best else {
                depth[i] = BIG;
                sec_o[i * 3] = 0.0;
                sec_o[i * 3 + 1] = -100.0;
                sec_o[i * 3 + 2] = 0.0;
                sec_d[i * 3 + 1] = -1.0;
                continue;
            };
            primary_hit[i] = true;
            let (a, b, c) = scene_tri(scene, tri as usize);
            let n = flip_normal(tri_normal(a, b, c), d);
            let p = [
                cam.origin[0] + d[0] * t,
                cam.origin[1] + d[1] * t,
                cam.origin[2] + d[2] * t,
            ];
            let v = [p[0] - cam.origin[0], p[1] - cam.origin[1], p[2] - cam.origin[2]];
            depth[i] = v[0] * cam.forward[0] + v[1] * cam.forward[1] + v[2] * cam.forward[2];
            nrm[i * 3..i * 3 + 3].copy_from_slice(&n);
            let albedo = tri_albedo(scene, tri);
            alb[i * 3..i * 3 + 3].copy_from_slice(&albedo);
            let o = [p[0] + n[0] * RAY_EPS, p[1] + n[1] * RAY_EPS, p[2] + n[2] * RAY_EPS];
            sec_o[i * 3..i * 3 + 3].copy_from_slice(&o);
            let base = m98_rng::pixel_base(i);
            let sd = cosine_dir(n, stream[base], stream[base + 1]);
            sec_d[i * 3..i * 3 + 3].copy_from_slice(&sd);
            // 主直接光(点光源近似 × host 阴影;与 L2 腿着色核同式)。
            let q = light_center(scene);
            let core = point_light_core(p, n, q, scene);
            let wv = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
            let dist = (wv[0] * wv[0] + wv[1] * wv[1] + wv[2] * wv[2]).sqrt().max(TINY);
            let wi = [wv[0] / dist, wv[1] / dist, wv[2] / dist];
            let t_sh = (dist - 2.0 * RAY_EPS).max(RAY_EPS);
            let (vis, _) = l2_shadow_vis(scene, o, wi, t_sh);
            let em = scene.light.emission;
            direct[i * 3] = albedo[0] * em[0] * core * vis;
            direct[i * 3 + 1] = albedo[1] * em[1] * core * vis;
            direct[i * 3 + 2] = albedo[2] * em[2] * core * vis;
        }
    }
    GBuffer {
        width,
        height,
        depth,
        nrm,
        alb,
        sec_o,
        sec_d,
        direct,
        primary_hit,
        stream,
    }
}

// ---------------------------------------------------------------------------
// L1 Screen Trace host 参照(与 kernel `g9_m98_screen_trace` 逐字同源)
// ---------------------------------------------------------------------------

/// L1 屏幕空间高度场 ray march(host 参照;kernel 逐字同源公式面)。
/// 屏幕线段均匀 [`M98_L1_MAX_STEPS`] 采样;同像素门跳过自像素;穿越判定 =
/// 视 z 超过缓冲 + [`M98_L1_DEPTH_BIAS`];命中即锁存(首命中)。返回腿样本
/// (命中着色 = 点光源近似未阴影;work = 命中前执行步数)。
#[allow(clippy::manual_clamp)] // march 算术门即公式面(.min/.max 序与 kernel 逐字同源;clamp 的 NaN 传播语义不同,禁改写)
pub fn l1_march_host(scene: &PtScene, gb: &GBuffer, pixel: usize) -> LegSample {
    let cam = &scene.camera;
    let w = gb.width as f32;
    let h = gb.height as f32;
    let o = [gb.sec_o[pixel * 3], gb.sec_o[pixel * 3 + 1], gb.sec_o[pixel * 3 + 2]];
    let d = [gb.sec_d[pixel * 3], gb.sec_d[pixel * 3 + 1], gb.sec_d[pixel * 3 + 2]];
    // 世界 → 屏幕投影(与 kernel 同式:z = v·f;sx = (v·r)/(z·tan))。
    let project = |p: [f32; 3]| -> (f32, f32, f32) {
        let v = [p[0] - cam.origin[0], p[1] - cam.origin[1], p[2] - cam.origin[2]];
        let z = v[0] * cam.forward[0] + v[1] * cam.forward[1] + v[2] * cam.forward[2];
        let x = v[0] * cam.right[0] + v[1] * cam.right[1] + v[2] * cam.right[2];
        let y = v[0] * cam.up[0] + v[1] * cam.up[1] + v[2] * cam.up[2];
        let sx = x / (z * cam.tan_half_fov);
        let sy = y / (z * cam.tan_half_fov);
        ((sx + 1.0) * 0.5 * w, (1.0 - sy) * 0.5 * h, z)
    };
    let (x0, y0, z0) = project(o);
    let e = [
        o[0] + d[0] * M98_L1_RANGE,
        o[1] + d[1] * M98_L1_RANGE,
        o[2] + d[2] * M98_L1_RANGE,
    ];
    let (x1, y1, z1) = project(e);
    // 端点在相机后 ⇒ 离屏,march 全门关闭(算术门,kernel 同式)。
    let valid = ((z0 - TINY) * BIG).min(1.0).max(0.0) * ((z1 - TINY) * BIG).min(1.0).max(0.0);
    let px0 = x0.max(0.0).min(w - 1.0);
    let py0 = y0.max(0.0).min(h - 1.0);
    let mut hit = 0.0f32;
    let mut hit_t = 0.0f32;
    let mut hit_idx = 0.0f32;
    let mut steps = 0.0f32;
    let inv_steps = 1.0 / M98_L1_MAX_STEPS as f32;
    for k in 1..=M98_L1_MAX_STEPS {
        let s = k as f32 * inv_steps;
        let xf = x0 + (x1 - x0) * s;
        let yf = y0 + (y1 - y0) * s;
        let zr = z0 + (z1 - z0) * s;
        let inb = ((xf) * BIG).min(1.0).max(0.0)
            * ((w - xf) * BIG).min(1.0).max(0.0)
            * ((yf) * BIG).min(1.0).max(0.0)
            * ((h - yf) * BIG).min(1.0).max(0.0);
        let xc = xf.max(0.0).min(w - 1.0);
        let yc = yf.max(0.0).min(h - 1.0);
        let idx = (yc as usize) * gb.width as usize + (xc as usize);
        let zb = gb.depth[idx];
        let same = ((0.5 - (xc - px0).abs()) * BIG).min(1.0).max(0.0)
            * ((0.5 - (yc - py0).abs()) * BIG).min(1.0).max(0.0);
        let cond = valid
            * inb
            * (1.0 - same)
            * ((zr - (zb + M98_L1_DEPTH_BIAS)) * BIG).min(1.0).max(0.0)
            * (1.0 - hit);
        hit_t += cond * (s * M98_L1_RANGE);
        hit_idx += cond * (idx as f32);
        steps += 1.0 - hit;
        hit += cond;
    }
    if hit < 0.5 {
        return LegSample {
            hit: false,
            t: 0.0,
            rgb: M98_SKY,
            work: steps as u32,
        };
    }
    let hi = hit_idx as usize;
    let n = flip_normal(
        [gb.nrm[hi * 3], gb.nrm[hi * 3 + 1], gb.nrm[hi * 3 + 2]],
        d,
    );
    let albedo = [gb.alb[hi * 3], gb.alb[hi * 3 + 1], gb.alb[hi * 3 + 2]];
    let p = [o[0] + d[0] * hit_t, o[1] + d[1] * hit_t, o[2] + d[2] * hit_t];
    LegSample {
        hit: true,
        t: hit_t,
        rgb: shade_point_unshadowed(albedo, p, n, scene),
        work: steps as u32,
    }
}

/// L1 腿批量(host 参照;仅供单测/harness 对拍腿)。
pub fn l1_leg_host(scene: &PtScene, gb: &GBuffer) -> Vec<LegSample> {
    (0..(gb.width * gb.height) as usize)
        .map(|i| l1_march_host(scene, gb, i))
        .collect()
}

// ---------------------------------------------------------------------------
// L3 HWRT host 镜像(公式锚;门绿由 device 腿承载——仅 host 输出不能充绿)
// ---------------------------------------------------------------------------

/// L3 腿 host 镜像(与 kernel `g9_m98_hwrt` 逐字同源;命中着色两档:
/// Simple = 点光源近似 × 阴影 + 天空 ambient;HitLighting = NEE 流采样
/// × 阴影;未命中 = 天空常量)。work 用 L2 同一解析求值测试数代理
/// (host 镜像无 ray query proceed 计数;device 腿计数为准)。
pub fn l3_leg_host(scene: &PtScene, gb: &GBuffer, mode: L3ShadeMode) -> Vec<LegSample> {
    let pixel_count = (gb.width * gb.height) as usize;
    let mut out = Vec::with_capacity(pixel_count);
    for i in 0..pixel_count {
        let o = [gb.sec_o[i * 3], gb.sec_o[i * 3 + 1], gb.sec_o[i * 3 + 2]];
        let d = [gb.sec_d[i * 3], gb.sec_d[i * 3 + 1], gb.sec_d[i * 3 + 2]];
        let (best, tests) = l2_closest_hit(scene, o, d, scene.t_max);
        let Some((t, tri)) = best else {
            out.push(LegSample {
                hit: false,
                t: 0.0,
                rgb: M98_SKY,
                work: tests as u32,
            });
            continue;
        };
        let (a, b, c) = scene_tri(scene, tri as usize);
        let n = flip_normal(tri_normal(a, b, c), d);
        let p = [o[0] + d[0] * t, o[1] + d[1] * t, o[2] + d[2] * t];
        let base = m98_rng::pixel_base(i);
        let q = match mode {
            L3ShadeMode::Simple => light_center(scene),
            L3ShadeMode::HitLighting => {
                light_sample(scene, gb.stream[base + 2], gb.stream[base + 3])
            }
        };
        let core = point_light_core(p, n, q, scene);
        let wv = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
        let dist = (wv[0] * wv[0] + wv[1] * wv[1] + wv[2] * wv[2]).sqrt().max(TINY);
        let wi = [wv[0] / dist, wv[1] / dist, wv[2] / dist];
        let t_sh = (dist - 2.0 * RAY_EPS).max(RAY_EPS);
        let so = [p[0] + n[0] * RAY_EPS, p[1] + n[1] * RAY_EPS, p[2] + n[2] * RAY_EPS];
        let (vis, sh_tests) = l2_shadow_vis(scene, so, wi, t_sh);
        let albedo = tri_albedo(scene, tri);
        let em = scene.light.emission;
        let mut rgb = [
            albedo[0] * em[0] * core * vis,
            albedo[1] * em[1] * core * vis,
            albedo[2] * em[2] * core * vis,
        ];
        if mode == L3ShadeMode::Simple {
            rgb[0] += M98_SKY[0] * albedo[0];
            rgb[1] += M98_SKY[1] * albedo[1];
            rgb[2] += M98_SKY[2] * albedo[2];
        }
        out.push(LegSample {
            hit: true,
            t,
            rgb,
            work: (tests + sh_tests) as u32,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// L4 Far Field(HLOD 接口未就绪 ⇒ SKIP=not-triggered 登记,禁静默当绿)
// ---------------------------------------------------------------------------

/// L4 触发条件核验结果(显式结构;RXS-0359 L5)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum L4TriggerState {
    /// HLOD 接口未就绪(G9.5 M111 资产/接口面未落地)⇒ 登记
    /// SKIP=not-triggered(条件未触发只表示决策已记录,不是成功)。
    NotTriggered {
        /// 未就绪原因(evidence 字面)。
        reason: &'static str,
    },
}

/// L4 触发条件核验器(fail-closed):HLOD 接口类型在本仓不存在(G9.5 M111)
/// ⇒ 恒 [`L4TriggerState::NotTriggered`];任何 L4 服务请求经 [`l4_serve`]
/// typed Err 显式拒绝。
pub fn check_l4_trigger() -> L4TriggerState {
    L4TriggerState::NotTriggered {
        reason: "HLOD 接口未就绪(G9.5 M111 资产/接口面未落地):L4 Far Field 登记 SKIP=not-triggered,不充绿",
    }
}

/// L4 消费接口(冻结面;RXS-0359 L1「本条款只冻结消费接口」):接口未就绪
/// ⇒ 恒 fail-closed [`FbError::L4InterfaceNotReady`],禁静默当绿。
pub fn l4_serve() -> Result<LegSample, FbError> {
    Err(FbError::L4InterfaceNotReady(
        "L4 Far Field 服务被请求,但 HLOD 接口未就绪(G9.5 M111)——登记 not-triggered,拒绝静默服务".into(),
    ))
}

// ---------------------------------------------------------------------------
// 选档器 + 装配 + 禁静默回退审计(RXS-0359 L2/L3/L4)
// ---------------------------------------------------------------------------

/// 单像素选档(纯函数;返回 (服务级别, 该像素转移记录))。
/// 链序 L1→L2→L3;L3 启用即终端服务(命中着色/未命中天空);全不可用 ⇒
/// None(Unserved 终端)。L4 永不在结果中(接口未就绪)。
fn select_pixel(
    switches: &ChainSwitches,
    l1: &LegSample,
    l2: &LegSample,
    l3: &LegSample,
    pixel: u32,
) -> Result<(Option<TraceLevel>, Vec<TransitionRecord>), FbError> {
    let mut records = Vec::new();
    for (idx, level) in TraceLevel::SELECTABLE.iter().enumerate() {
        let next = if idx + 1 < TraceLevel::SELECTABLE.len() {
            TraceLevel::SELECTABLE[idx + 1]
        } else {
            TraceLevel::L4FarField
        };
        if !switches.enabled(*level) {
            records.push(TransitionRecord {
                pixel,
                from: *level,
                to: next,
                cause: TransitionCause::ForcedOff,
            });
            continue;
        }
        let s = match level {
            TraceLevel::L1ScreenTrace => l1,
            TraceLevel::L2Swrt => l2,
            _ => l3,
        };
        let in_range = match level {
            TraceLevel::L1ScreenTrace => s.t <= M98_L1_RANGE,
            TraceLevel::L2Swrt => s.t <= M98_L2_RANGE,
            _ => true,
        };
        if s.hit && in_range {
            if *level == TraceLevel::L3Hwrt && s.t > M98_VIEW_DIST {
                // 视距外 ⇒ 本应升级 L4;接口未就绪 ⇒ fail-closed(禁静默当绿)。
                return Err(FbError::L4InterfaceNotReady(format!(
                    "像素 {pixel} L3 命中 t={} 超视距 {M98_VIEW_DIST},应升级 L4 但接口未就绪",
                    s.t
                )));
            }
            return Ok((Some(*level), records));
        }
        if *level == TraceLevel::L3Hwrt {
            // L3 终端:未命中 = 天空服务(不转移)。
            return Ok((Some(TraceLevel::L3Hwrt), records));
        }
        records.push(TransitionRecord {
            pixel,
            from: *level,
            to: next,
            cause: if s.hit {
                TransitionCause::OutOfRange
            } else {
                TransitionCause::Miss
            },
        });
    }
    // L1/L2/L3 全关 ⇒ Unserved 终端(显式;最后一条记录 to=L3 的 ForcedOff
    // 之后无链内下级,终端天空由装配层写入)。
    Ok((None, records))
}

/// 腿样本取值(按级别)。
fn leg_of<'a>(
    level: TraceLevel,
    l1: &'a [LegSample],
    l2: &'a [LegSample],
    l3: &'a [LegSample],
    i: usize,
) -> &'a LegSample {
    match level {
        TraceLevel::L1ScreenTrace => &l1[i],
        TraceLevel::L2Swrt => &l2[i],
        _ => &l3[i],
    }
}

/// 装配一帧(选档 + 合成 + 计数 + 转移日志 + 审计)。`log_transitions=false`
/// = 静默回退注入 variant(负例臂①;审计必 fail-closed Err)。
///
/// 合成语义:主未命中像素 ⇒ 天空常量直出(flag=0,不入链);链内像素 ⇒
/// rgb = 主直接光 + 主 albedo × 选档腿辐射度(Unserved ⇒ 腿辐射度 = 天空)。
pub fn assemble(
    gb: &GBuffer,
    switches: ChainSwitches,
    l1: &[LegSample],
    l2: &[LegSample],
    l3: &[LegSample],
    log_transitions: bool,
) -> Result<ChainFrame, FbError> {
    let pixel_count = (gb.width * gb.height) as usize;
    for (name, leg) in [("l1", l1), ("l2", l2), ("l3", l3)] {
        if leg.len() != pixel_count {
            return Err(FbError::InvalidConfig(format!(
                "{name} 腿样本数 {} ≠ 像素数 {pixel_count}",
                leg.len()
            )));
        }
    }
    let mut rgb = vec![0.0f32; pixel_count * 3];
    let mut flags = vec![FLAG_UNSERVED; pixel_count];
    let mut counters = [LevelCounters::default(); 4];
    let mut transitions: Vec<TransitionRecord> = Vec::new();
    for i in 0..pixel_count {
        if !gb.primary_hit[i] {
            rgb[i * 3..i * 3 + 3].copy_from_slice(&M98_SKY);
            continue;
        }
        let (served, records) = select_pixel(&switches, &l1[i], &l2[i], &l3[i], i as u32)?;
        if log_transitions {
            transitions.extend_from_slice(&records);
        }
        let leg_rgb = match served {
            Some(level) => {
                flags[i] = level.flag();
                leg_of(level, l1, l2, l3, i).rgb
            }
            None => M98_SKY,
        };
        let a = [gb.alb[i * 3], gb.alb[i * 3 + 1], gb.alb[i * 3 + 2]];
        rgb[i * 3] = gb.direct[i * 3] + a[0] * leg_rgb[0];
        rgb[i * 3 + 1] = gb.direct[i * 3 + 1] + a[1] * leg_rgb[1];
        rgb[i * 3 + 2] = gb.direct[i * 3 + 2] + a[2] * leg_rgb[2];
    }
    // 计数面:启用级 ⇒ attempted = 链内像素数(批量执行);hit/work 全量计;
    // served 按 flags。级关 ⇒ 全零显式。
    let chain_pixels = gb.primary_hit.iter().filter(|&&b| b).count() as u64;
    for level in TraceLevel::SELECTABLE {
        let slot = level.slot();
        if switches.enabled(level) {
            counters[slot].rays_attempted = chain_pixels;
            for i in 0..pixel_count {
                if !gb.primary_hit[i] {
                    continue;
                }
                let s = leg_of(level, l1, l2, l3, i);
                if s.hit {
                    counters[slot].rays_hit += 1;
                }
                counters[slot].work_count += u64::from(s.work);
            }
            counters[slot].pixels_served =
                flags.iter().filter(|&&f| f == level.flag()).count() as u64;
        }
    }
    let frame = ChainFrame {
        width: gb.width,
        height: gb.height,
        rgb,
        flags,
        counters,
        transitions,
    };
    audit(&frame, gb, &switches, l1, l2, l3)?;
    Ok(frame)
}

/// 禁静默回退审计(fail-closed;RXS-0359 L4):独立重算期望转移集合(逐像素
/// 重走选档契约)与帧转移日志逐条比对——任何实际发生但未记录的级别变化 =
/// [`FbError::SilentFallback`];计数面 served 与 flags 复核一致。
pub fn audit(
    frame: &ChainFrame,
    gb: &GBuffer,
    switches: &ChainSwitches,
    l1: &[LegSample],
    l2: &[LegSample],
    l3: &[LegSample],
) -> Result<(), FbError> {
    let pixel_count = (gb.width * gb.height) as usize;
    let mut expected: Vec<TransitionRecord> = Vec::new();
    for i in 0..pixel_count {
        if !gb.primary_hit[i] {
            if frame.flags[i] != FLAG_UNSERVED {
                return Err(FbError::SilentFallback(format!(
                    "像素 {i} 主未命中但 flags={} ≠ Unserved(0)",
                    frame.flags[i]
                )));
            }
            continue;
        }
        let (served, records) = select_pixel(switches, &l1[i], &l2[i], &l3[i], i as u32)?;
        let flag = served.map_or(FLAG_UNSERVED, |l| l.flag());
        if frame.flags[i] != flag {
            return Err(FbError::SilentFallback(format!(
                "像素 {i} flags={} ≠ 选档重算 {flag}(实际服务级别与记录不符)",
                frame.flags[i]
            )));
        }
        expected.extend_from_slice(&records);
    }
    if frame.transitions != expected {
        return Err(FbError::SilentFallback(format!(
            "转移日志 {} 条 ≠ 独立重算 {} 条(或内容不符)——无记录降级即 RED",
            frame.transitions.len(),
            expected.len()
        )));
    }
    for level in TraceLevel::SELECTABLE {
        let served = frame
            .flags
            .iter()
            .filter(|&&f| f == level.flag())
            .count() as u64;
        if switches.enabled(level) && frame.counters[level.slot()].pixels_served != served {
            return Err(FbError::SilentFallback(format!(
                "{} 计数面 pixels_served={} ≠ flags 重算 {served}",
                level.name(),
                frame.counters[level.slot()].pixels_served
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 按匹配深度对 M96 golden 的容差带(measured 后冻结,P-09;fail-closed)
// ---------------------------------------------------------------------------

/// 深度带单条目(一档 = 逐档 solo / 全链;匹配深度 [`M98_MATCHED_DEPTH`])。
#[derive(Debug, Clone, PartialEq)]
pub struct M98BandEntry {
    /// 档位名(l1_solo / l2_solo / l3_simple_solo / l3_hit_lighting_solo /
    /// chain_simple / chain_hit_lighting)。
    pub tier: String,
    /// 冻结 golden:该档产物 digest(sha256(rgb‖flags))。
    pub chain_digest: String,
    /// 冻结 golden:M96 同深度参照产物 digest。
    pub m96_digest: String,
    /// 冻结容差带(rel_dev 上界 = measured × [`M98_BAND_MARGIN`];禁手写)。
    pub band_rel_dev: f64,
    /// 冻结时实测 rel_dev(该档合成图 vs M96 同深度;provenance)。
    pub measured_rel_dev: f64,
}

/// M98 深度容差带(`milestones/g9/g9_m98_depth_band.json` 的内存形)。
#[derive(Debug, Clone, PartialEq)]
pub struct M98DepthBand {
    /// provenance:冻结时刻 UTC。
    pub frozen_at_utc: String,
    /// provenance:device 名。
    pub device_name: String,
    /// 冻结场景名(M96 冻结 fixture)。
    pub scene: String,
    /// M96 门序消费锚:本带 m96_digest 与 M97 冻结带 `m96_cornell` 同深度
    /// 条目逐字相等(D2-Q7 门序消费面的机器锚)。
    pub m96_anchor_digest: String,
    /// 逐档条目。
    pub entries: Vec<M98BandEntry>,
}

impl M98DepthBand {
    /// 查条目(fail-closed:缺条目 = Err)。
    pub fn entry(&self, tier: &str) -> Result<&M98BandEntry, FbError> {
        self.entries
            .iter()
            .find(|e| e.tier == tier)
            .ok_or_else(|| FbError::DepthBand(format!("容差带缺条目 tier={tier}")))
    }

    /// 比对(fail-closed):双 digest 全等 且 rel_dev ≤ 带;违例逐条列名。
    pub fn check(
        &self,
        tier: &str,
        chain_digest: &str,
        m96_digest: &str,
        rel_dev: f64,
    ) -> Result<(), FbError> {
        let e = self.entry(tier)?;
        if chain_digest != e.chain_digest {
            return Err(FbError::DepthBand(format!(
                "tier={tier} chain_digest {chain_digest} ≠ golden {}",
                e.chain_digest
            )));
        }
        if m96_digest != e.m96_digest {
            return Err(FbError::DepthBand(format!(
                "tier={tier} m96_digest {m96_digest} ≠ golden {}",
                e.m96_digest
            )));
        }
        if rel_dev.is_nan() || rel_dev > e.band_rel_dev {
            return Err(FbError::DepthBand(format!(
                "tier={tier} rel_dev {rel_dev:.6e} 越带(上界 {:.6e})",
                e.band_rel_dev
            )));
        }
        Ok(())
    }

    /// 序列化(手工 JSON;字段序冻结,浮点 `{:e}` 确定性格式)。
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n  \"schema\": \"rurix.g9m98.depth_band.v1\",\n");
        s.push_str(&format!("  \"frozen_at_utc\": \"{}\",\n", self.frozen_at_utc));
        s.push_str(&format!("  \"device_name\": \"{}\",\n", self.device_name));
        s.push_str(&format!("  \"scene\": \"{}\",\n", self.scene));
        s.push_str(&format!(
            "  \"m96_anchor_digest\": \"{}\",\n",
            self.m96_anchor_digest
        ));
        s.push_str(&format!(
            "  \"freeze_rule\": \"band_rel_dev = measured_rel_dev * {:.1}(规则冻结于 gi::fallback_chain::M98_BAND_MARGIN;基值 = 冻结批实测,禁手写 P-09)\",\n",
            M98_BAND_MARGIN
        ));
        s.push_str(&format!("  \"matched_depth\": \"{}\",\n", M98_MATCHED_DEPTH));
        s.push_str(&format!("  \"m96_golden_spp\": \"{}\",\n", M98_M96_GOLDEN_SPP));
        s.push_str(&format!("  \"seed_chain\": \"{}\",\n", M98_SEED));
        s.push_str("  \"entries\": [\n");
        for (i, e) in self.entries.iter().enumerate() {
            s.push_str(&format!(
                "    {{\"tier\": \"{}\", \"chain_digest\": \"{}\", \"m96_digest\": \"{}\", \"band_rel_dev\": \"{:e}\", \"measured_rel_dev\": \"{:e}\"}}{}\n",
                e.tier,
                e.chain_digest,
                e.m96_digest,
                e.band_rel_dev,
                e.measured_rel_dev,
                if i + 1 == self.entries.len() { "" } else { "," }
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }

    /// 解析(fail-closed:schema 不符/键缺失/数值非法/条目重复一律 Err)。
    pub fn parse(text: &str) -> Result<M98DepthBand, FbError> {
        let err = |m: &str| FbError::DepthBand(format!("容差带解析: {m}"));
        if !text.contains("\"schema\": \"rurix.g9m98.depth_band.v1\"") {
            return Err(err("schema 失配"));
        }
        let get_str = |key: &str| -> Result<String, FbError> {
            let needle = format!("\"{key}\": \"");
            let start = text
                .find(&needle)
                .ok_or_else(|| err(&format!("缺键 {key}")))?
                + needle.len();
            let end = text[start..]
                .find('"')
                .ok_or_else(|| err(&format!("键 {key} 值未闭合")))?
                + start;
            Ok(text[start..end].to_string())
        };
        let mut entries = Vec::new();
        let entries_sec = text
            .split("\"entries\": [")
            .nth(1)
            .ok_or_else(|| err("缺 entries 段"))?;
        for chunk in entries_sec.split('{').skip(1) {
            let body = chunk.split('}').next().ok_or_else(|| err("条目未闭合"))?;
            let field = |key: &str| -> Result<String, FbError> {
                let needle = format!("\"{key}\": \"");
                let start = body
                    .find(&needle)
                    .ok_or_else(|| err(&format!("条目缺键 {key}")))?
                    + needle.len();
                let end = body[start..]
                    .find('"')
                    .ok_or_else(|| err("条目键 {key} 值未闭合"))?
                    + start;
                Ok(body[start..end].to_string())
            };
            let tier = field("tier")?;
            if entries.iter().any(|e: &M98BandEntry| e.tier == tier) {
                return Err(err("条目 tier 重复"));
            }
            let chain_digest = field("chain_digest")?;
            let m96_digest = field("m96_digest")?;
            for (nm, d) in [("chain_digest", &chain_digest), ("m96_digest", &m96_digest)] {
                if d.len() != 64 || !d.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(err(&format!("{nm} 非 64 位 hex")));
                }
            }
            let band_rel_dev: f64 = field("band_rel_dev")?
                .parse()
                .map_err(|_| err("band_rel_dev 非数值"))?;
            let measured_rel_dev: f64 = field("measured_rel_dev")?
                .parse()
                .map_err(|_| err("measured_rel_dev 非数值"))?;
            if band_rel_dev <= 0.0 || !band_rel_dev.is_finite() {
                return Err(err("band_rel_dev 非正有限数"));
            }
            entries.push(M98BandEntry {
                tier,
                chain_digest,
                m96_digest,
                band_rel_dev,
                measured_rel_dev,
            });
        }
        if entries.is_empty() {
            return Err(err("entries 为空"));
        }
        Ok(M98DepthBand {
            frozen_at_utc: get_str("frozen_at_utc")?,
            device_name: get_str("device_name")?,
            scene: get_str("scene")?,
            m96_anchor_digest: get_str("m96_anchor_digest")?,
            entries,
        })
    }
}

// ---------------------------------------------------------------------------
// device 输入打包(kernel 头注参数面逐字同源;f32 位级编码)
// ---------------------------------------------------------------------------

/// L1 kernel 参数打包(30 f32;与 `kernels/g9_m98_screen_trace.rx` 头注逐字同源)。
pub fn pack_l1_params(scene: &PtScene, gb: &GBuffer) -> Vec<f32> {
    let cam = &scene.camera;
    let c = light_center(scene);
    let ln = scene.light.normal();
    let pixel_count = gb.width * gb.height;
    let mut p = Vec::with_capacity(30);
    p.push(pixel_count as f32);
    p.push(gb.width as f32);
    p.push(gb.height as f32);
    p.push(M98_L1_MAX_STEPS as f32);
    p.push(1.0 / M98_L1_MAX_STEPS as f32);
    p.push(M98_L1_RANGE);
    p.push(M98_L1_DEPTH_BIAS);
    p.extend_from_slice(&cam.origin);
    p.extend_from_slice(&cam.forward);
    p.extend_from_slice(&cam.right);
    p.extend_from_slice(&cam.up);
    p.push(cam.tan_half_fov);
    p.extend_from_slice(&c);
    p.extend_from_slice(&scene.light.emission);
    p.push(scene.light.area());
    p.extend_from_slice(&ln);
    debug_assert_eq!(p.len(), 30);
    p
}

/// L3 kernel 参数打包(23 f32;与 `kernels/g9_m98_hwrt.rx` 头注逐字同源)。
pub fn pack_l3_params(scene: &PtScene, mode: L3ShadeMode) -> Vec<f32> {
    let cam = &scene.camera;
    let l = &scene.light;
    let ln = l.normal();
    let pixel_count = cam.width * cam.height;
    let mut p = Vec::with_capacity(23);
    p.push(pixel_count as f32);
    p.push(mode.as_f32());
    p.push(RAY_EPS);
    p.push(scene.t_max);
    p.extend_from_slice(&l.p00);
    p.extend_from_slice(&l.e1);
    p.extend_from_slice(&l.e2);
    p.push(l.area());
    p.extend_from_slice(&l.emission);
    p.extend_from_slice(&ln);
    p.extend_from_slice(&M98_SKY);
    debug_assert_eq!(p.len(), 23);
    p
}

// ---------------------------------------------------------------------------
// 单测(RXS-0359 锚定;host 面——选档契约 / 转移日志 / 静默回退审计 / L4
// not-triggered / L2 金标准对拍 / 数值锚 / 容差带 fail-closed / 强关可检测)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::bvh::{Ray, TriBvh, Vec3};
    use crate::rt::ref_tracer::Pcg32;

    fn cornell() -> PtScene {
        let s = path_trace::m96_cornell_scene();
        s.validate().expect("cornell 冻结 fixture 装载");
        s
    }

    fn legs(gb: &GBuffer) -> (Vec<LegSample>, Vec<LegSample>, Vec<LegSample>) {
        let scene = cornell();
        (
            l1_leg_host(&scene, gb),
            l2_leg_host(&scene, &gb.sec_o, &gb.sec_d),
            l3_leg_host(&scene, gb, L3ShadeMode::Simple),
        )
    }

    //@ spec: RXS-0359
    #[test]
    fn level_order_flags_and_names() {
        // 四级链冻结序 L1→L2→L3→L4;flag 编码 1..=4;计数面 slot 0..=3。
        let order = [
            TraceLevel::L1ScreenTrace,
            TraceLevel::L2Swrt,
            TraceLevel::L3Hwrt,
            TraceLevel::L4FarField,
        ];
        for (i, l) in order.iter().enumerate() {
            assert_eq!(l.flag() as u32, (i + 1) as u32);
            assert_eq!(l.slot(), i);
        }
        assert_eq!(TraceLevel::SELECTABLE.len(), 3);
        assert_eq!(TraceLevel::ALL.len(), 4);
        assert_eq!(TraceLevel::L1ScreenTrace.name(), "l1_screen_trace");
        assert_eq!(TraceLevel::L4FarField.name(), "l4_far_field");
        // L3 着色两档字面冻结。
        assert_eq!(L3ShadeMode::Simple.as_f32(), 0.0);
        assert_eq!(L3ShadeMode::HitLighting.as_f32(), 1.0);
        assert_eq!(L3ShadeMode::HitLighting.name(), "hit_lighting");
    }

    //@ spec: RXS-0359
    #[test]
    fn switches_independent_per_level() {
        let all = ChainSwitches::ALL_ON;
        assert!(all.enabled(TraceLevel::L1ScreenTrace));
        // L4 不在开关面(接口未就绪 ⇒ 恒不可用)。
        assert!(!all.enabled(TraceLevel::L4FarField));
        let off = ChainSwitches {
            l1: false,
            l2: true,
            l3: true,
        };
        assert!(!off.enabled(TraceLevel::L1ScreenTrace));
        assert!(off.enabled(TraceLevel::L2Swrt));
    }

    //@ spec: RXS-0359
    #[test]
    fn selector_distance_and_coverage_priority() {
        // 合成腿:L1 命中 t=0.3(≤0.5)⇒ 服务 L1,零转移。
        let mk = |hit: bool, t: f32| LegSample {
            hit,
            t,
            rgb: [1.0, 0.5, 0.25],
            work: 7,
        };
        let (served, recs) =
            select_pixel(&ChainSwitches::ALL_ON, &mk(true, 0.3), &mk(true, 0.4), &mk(true, 9.0), 5)
                .expect("选档");
        assert_eq!(served, Some(TraceLevel::L1ScreenTrace));
        assert!(recs.is_empty(), "L1 直服务无转移");
        // L1 未命中 ⇒ 转移 L1→L2(Miss);L2 命中 t=0.8(≤1.0)⇒ 服务 L2。
        let (served, recs) =
            select_pixel(&ChainSwitches::ALL_ON, &mk(false, 0.0), &mk(true, 0.8), &mk(true, 9.0), 5)
                .expect("选档");
        assert_eq!(served, Some(TraceLevel::L2Swrt));
        assert_eq!(
            recs,
            vec![TransitionRecord {
                pixel: 5,
                from: TraceLevel::L1ScreenTrace,
                to: TraceLevel::L2Swrt,
                cause: TransitionCause::Miss,
            }]
        );
        // L1 miss + L2 命中 t=1.7(>1.0)⇒ OutOfRange 转移;L3 服务。
        let (served, recs) =
            select_pixel(&ChainSwitches::ALL_ON, &mk(false, 0.0), &mk(true, 1.7), &mk(true, 1.7), 6)
                .expect("选档");
        assert_eq!(served, Some(TraceLevel::L3Hwrt));
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[1].cause, TransitionCause::OutOfRange);
        assert_eq!(recs[1].to, TraceLevel::L3Hwrt);
        // L3 miss ⇒ L3 终端服务(天空),无 L4 转移(L4 not-triggered)。
        let (served, recs) =
            select_pixel(&ChainSwitches::ALL_ON, &mk(false, 0.0), &mk(false, 0.0), &mk(false, 0.0), 7)
                .expect("选档");
        assert_eq!(served, Some(TraceLevel::L3Hwrt));
        assert_eq!(recs.len(), 2);
    }

    //@ spec: RXS-0359
    #[test]
    fn force_off_records_forced_off_cause() {
        let mk = |hit: bool, t: f32| LegSample {
            hit,
            t,
            rgb: [0.0; 3],
            work: 0,
        };
        // L1 强关 ⇒ 转移 L1→L2(ForcedOff);L2 服务。
        let sw = ChainSwitches {
            l1: false,
            l2: true,
            l3: true,
        };
        let (served, recs) =
            select_pixel(&sw, &mk(true, 0.3), &mk(true, 0.4), &mk(true, 0.4), 0).expect("选档");
        assert_eq!(served, Some(TraceLevel::L2Swrt));
        assert_eq!(recs[0].cause, TransitionCause::ForcedOff);
        assert_eq!(recs[0].from, TraceLevel::L1ScreenTrace);
        // 全关 ⇒ Unserved 终端(显式,非静默)。
        let sw = ChainSwitches {
            l1: false,
            l2: false,
            l3: false,
        };
        let (served, recs) =
            select_pixel(&sw, &mk(true, 0.3), &mk(true, 0.4), &mk(true, 0.4), 0).expect("选档");
        assert_eq!(served, None);
        assert_eq!(recs.len(), 3, "三级 ForcedOff 全记录");
        assert!(recs.iter().all(|r| r.cause == TransitionCause::ForcedOff));
    }

    //@ spec: RXS-0359
    #[test]
    fn silent_demotion_injection_fails_audit() {
        let scene = cornell();
        let gb = gbuffer_prepass(&scene);
        let (l1, l2, l3) = legs(&gb);
        // 正例:生产路径(记录开)⇒ 装配 + 审计过。
        let frame = assemble(&gb, ChainSwitches::ALL_ON, &l1, &l2, &l3, true)
            .expect("正例装配");
        audit(&frame, &gb, &ChainSwitches::ALL_ON, &l1, &l2, &l3).expect("正例审计");
        assert!(!frame.transitions.is_empty(), "cornell 存在级别转移");
        // 负例臂①(静默回退注入):同一选档结果但转移日志被抑 ⇒ 审计必拒。
        let silent = assemble(&gb, ChainSwitches::ALL_ON, &l1, &l2, &l3, false);
        match silent {
            Err(FbError::SilentFallback(_)) => {}
            other => panic!("静默回退注入必须 fail-closed SilentFallback,实际 {other:?}"),
        }
        // 直接审计面:手工伪造缺日志帧 ⇒ 必拒。
        let mut forged = frame.clone();
        forged.transitions.clear();
        let e = audit(&forged, &gb, &ChainSwitches::ALL_ON, &l1, &l2, &l3)
            .expect_err("缺日志帧必拒");
        assert!(matches!(e, FbError::SilentFallback(_)));
    }

    //@ spec: RXS-0359
    #[test]
    fn l4_not_triggered_registration_fail_closed() {
        // 触发条件核验器:HLOD 接口未就绪 ⇒ NotTriggered(显式结构,非绿色)。
        let st = check_l4_trigger();
        let L4TriggerState::NotTriggered { reason } = st;
        assert!(reason.contains("HLOD"), "登记原因须含 HLOD 未就绪字面");
        // L4 服务请求 ⇒ fail-closed typed Err(禁静默当绿)。
        match l4_serve() {
            Err(FbError::L4InterfaceNotReady(_)) => {}
            other => panic!("L4 服务必须 typed Err,实际 {other:?}"),
        }
        // 选档器结构断言:L4 永不可选(开关面恒 false)。
        assert!(!ChainSwitches::ALL_ON.enabled(TraceLevel::L4FarField));
    }

    //@ spec: RXS-0359
    #[test]
    fn l2_bruteforce_matches_bvh_gold_standard() {
        // L2 host 暴力腿 vs rt::bvh BVH 金标准:逐命中一致( prim 精确相等,
        // t 位级容差 ≤1e-6——遍历序不同但同公式最近命中)。
        let scene = cornell();
        let bvh = TriBvh::build(&scene.positions, &scene.indices);
        let mut rng = Pcg32::new(M98_SEED);
        let mut checked = 0u32;
        for _ in 0..256 {
            let o = [
                rng.next_f32() * 1.5 - 0.25,
                rng.next_f32() * 1.5 - 0.25,
                rng.next_f32() * 1.5 - 0.25,
            ];
            let d = Vec3::new(
                rng.next_f32() - 0.5,
                rng.next_f32() - 0.5,
                rng.next_f32() - 0.5,
            )
            .normalize()
            .to_array();
            let ray = Ray {
                origin: Vec3::from_array(o),
                dir: Vec3::from_array(d),
            };
            let gold = bvh.intersect(&ray);
            let (mine, tests) = l2_closest_hit(&scene, o, d, scene.t_max);
            assert_eq!(tests, scene.indices.len() as u64, "全量扫描测试数确定");
            match (gold, mine) {
                (Some(g), Some((t, prim))) => {
                    assert_eq!(g.tri, prim, "命中三角号一致");
                    assert!((g.t - t).abs() <= 1e-6, "t 一致: {} vs {t}", g.t);
                }
                (None, None) => {}
                (g, m) => panic!("命中性分叉: gold={g:?} mine={m:?}"),
            }
            checked += 1;
        }
        assert!(checked > 0);
    }

    //@ spec: RXS-0359
    #[test]
    fn cosine_dir_and_stream_determinism() {
        // 流确定性 + 值域 + 改 seed 分叉。
        let a = m98_rng::generate_stream(8, M98_SEED);
        let b = m98_rng::generate_stream(8, M98_SEED);
        assert_eq!(a, b, "同 seed 流位级一致");
        assert!(a.iter().all(|v| (0.0..1.0).contains(v)));
        let c = m98_rng::generate_stream(8, M98_SEED + 1);
        assert_ne!(a, c);
        assert_eq!(m98_rng::stream_len(8), 32);
        assert_eq!(m98_rng::pixel_base(3), 12);
        // 余弦方向:单位长 + 上半球(n·d ≥ 0)+ 确定性。
        let n = [0.0, 1.0, 0.0];
        let d1 = cosine_dir(n, 0.31, 0.72);
        let d2 = cosine_dir(n, 0.31, 0.72);
        assert_eq!(d1, d2);
        let len = (d1[0] * d1[0] + d1[1] * d1[1] + d1[2] * d1[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-6);
        assert!(d1[1] >= 0.0, "上半球");
        // 退化法线(±y 邻近)不产 NaN。
        let d3 = cosine_dir([0.0, 0.9999, 0.0], 0.5, 0.5);
        assert!(d3.iter().all(|v| v.is_finite()));
    }

    //@ spec: RXS-0359
    #[test]
    fn point_light_core_numeric_anchor() {
        // 手算锚:命中点 (0.5, 0.0, 0.5),法线 +y;光源中心 (0.5,0.995,0.5),
        // dist=0.995 ⇒ dist2=0.990025;cos_s=1,cos_l=1;area=0.09。
        let scene = cornell();
        let core = point_light_core([0.5, 0.0, 0.5], [0.0, 1.0, 0.0], light_center(&scene), &scene);
        let expect = 0.09f32 / (path_trace::PT_PI * 0.990_025);
        assert!(
            (core - expect).abs() / expect < 1e-5,
            "core {core} vs 手算 {expect}"
        );
        // 背光面法线 ⇒ cos_s=0 ⇒ core=0。
        let back = point_light_core([0.5, 0.0, 0.5], [0.0, -1.0, 0.0], light_center(&scene), &scene);
        assert_eq!(back, 0.0);
        // 光源中心锚:(p00 + 0.5e1 + 0.5e2) = (0.5, 0.995, 0.5)。
        assert_eq!(light_center(&scene), [0.5, 0.995, 0.5]);
    }

    //@ spec: RXS-0359
    #[test]
    fn gbuffer_prepass_deterministic_and_sane() {
        let scene = cornell();
        let a = gbuffer_prepass(&scene);
        let b = gbuffer_prepass(&scene);
        assert_eq!(a, b, "GBuffer 预传递双跑位级一致");
        // cornell 开口盒:主命中像素存在且占多数;中心像素必命中。
        let hits = a.primary_hit.iter().filter(|&&x| x).count();
        assert!(hits > 3000, "主命中像素 {hits} 应占多数");
        let center = (32 * 64 + 32) as usize;
        assert!(a.primary_hit[center]);
        assert!(a.depth[center] > 0.0 && a.depth[center] < 10.0);
        // 未命中像素深度 = 哨兵;direct 非负有限。
        for i in 0..(64 * 64) as usize {
            if !a.primary_hit[i] {
                assert_eq!(a.depth[i], BIG);
            }
            assert!(a.direct[i * 3].is_finite() && a.direct[i * 3] >= 0.0);
        }
    }

    //@ spec: RXS-0359
    #[test]
    fn force_off_changes_product_digest_structural() {
        // 逐级强关回归可检测的结构性锚(host 全腿):golden vs 强关 L1/L2/L3
        // 的产物 digest 必分叉(flags 携带实际服务级别 ⇒ 转移必改 digest)。
        let scene = cornell();
        let gb = gbuffer_prepass(&scene);
        let (l1, l2, l3) = legs(&gb);
        let golden = assemble(&gb, ChainSwitches::ALL_ON, &l1, &l2, &l3, true)
            .expect("golden 装配");
        // golden 必须真实消费全部三级(否则强关臂空转 = 降级链失效)。
        for level in TraceLevel::SELECTABLE {
            assert!(
                golden.counters[level.slot()].pixels_served > 0,
                "{} 在 golden 中服务像素数必须 > 0",
                level.name()
            );
        }
        for (name, sw) in [
            ("l1", ChainSwitches { l1: false, l2: true, l3: true }),
            ("l2", ChainSwitches { l1: true, l2: false, l3: true }),
            ("l3", ChainSwitches { l1: true, l2: true, l3: false }),
        ] {
            let off = assemble(&gb, sw, &l1, &l2, &l3, true).expect("强关装配");
            assert_ne!(
                golden.product_digest(),
                off.product_digest(),
                "强关 {name} 后产物 digest 仍同 golden = RED(回归不可检测)"
            );
            assert!(
                off.transitions
                    .iter()
                    .any(|r| r.cause == TransitionCause::ForcedOff),
                "强关 {name} 转移日志须含 ForcedOff"
            );
        }
        // 双跑位级一致。
        let again = assemble(&gb, ChainSwitches::ALL_ON, &l1, &l2, &l3, true)
            .expect("双跑装配");
        assert_eq!(golden, again, "装配双跑位级一致");
        assert_eq!(golden.usage_log_digest(), again.usage_log_digest());
    }

    //@ spec: RXS-0359
    #[test]
    fn counters_faces_non_empty_per_frame() {
        let scene = cornell();
        let gb = gbuffer_prepass(&scene);
        let (l1, l2, l3) = legs(&gb);
        let frame = assemble(&gb, ChainSwitches::ALL_ON, &l1, &l2, &l3, true)
            .expect("装配");
        let chain_px = gb.primary_hit.iter().filter(|&&b| b).count() as u64;
        for level in TraceLevel::SELECTABLE {
            let c = frame.counters[level.slot()];
            assert_eq!(c.rays_attempted, chain_px, "{} attempted", level.name());
            assert!(c.rays_hit > 0, "{} hit 非空", level.name());
            assert!(c.work_count > 0, "{} 耗时计数非空", level.name());
            assert!(c.hit_rate() > 0.0 && c.hit_rate() <= 1.0);
        }
        // L4 行:零计数 + not-triggered 登记(不充绿)。
        assert_eq!(frame.counters[TraceLevel::L4FarField.slot()], LevelCounters::default());
        assert!(matches!(check_l4_trigger(), L4TriggerState::NotTriggered { .. }));
    }

    //@ spec: RXS-0359
    #[test]
    fn band_roundtrip_and_fail_closed() {
        let band = M98DepthBand {
            frozen_at_utc: "2026-08-12T00:00:00Z".into(),
            device_name: "test-device".into(),
            scene: "m96_cornell".into(),
            m96_anchor_digest: "ab".repeat(32),
            entries: vec![
                M98BandEntry {
                    tier: "chain_simple".into(),
                    chain_digest: "11".repeat(32),
                    m96_digest: "22".repeat(32),
                    band_rel_dev: 0.4,
                    measured_rel_dev: 0.2,
                },
                M98BandEntry {
                    tier: "chain_hit_lighting".into(),
                    chain_digest: "33".repeat(32),
                    m96_digest: "22".repeat(32),
                    band_rel_dev: 0.6,
                    measured_rel_dev: 0.3,
                },
            ],
        };
        let text = band.to_json();
        let back = M98DepthBand::parse(&text).expect("roundtrip");
        assert_eq!(band, back);
        // 正常比对过。
        band.check("chain_simple", &"11".repeat(32), &"22".repeat(32), 0.2)
            .expect("在带内");
        band.check("chain_simple", &"11".repeat(32), &"22".repeat(32), 0.4)
            .expect("带界");
        // digest 不符 ⇒ Err。
        assert!(band.check("chain_simple", &"44".repeat(32), &"22".repeat(32), 0.2).is_err());
        assert!(band.check("chain_simple", &"11".repeat(32), &"55".repeat(32), 0.2).is_err());
        // 越带 ⇒ Err;NaN ⇒ Err;缺条目 ⇒ Err。
        assert!(band.check("chain_simple", &"11".repeat(32), &"22".repeat(32), 0.41).is_err());
        assert!(band
            .check("chain_simple", &"11".repeat(32), &"22".repeat(32), f64::NAN)
            .is_err());
        assert!(band.check("nope", &"11".repeat(32), &"22".repeat(32), 0.1).is_err());
        // 解析 fail-closed:schema 失配/重复条目/空条目/非 hex。
        assert!(M98DepthBand::parse("{}").is_err());
        let dup = text.replace("chain_hit_lighting", "chain_simple");
        assert!(M98DepthBand::parse(&dup).is_err(), "重复 tier 必拒");
        let bad = text.replace(&"11".repeat(32), "zz");
        assert!(M98DepthBand::parse(&bad).is_err(), "非 hex digest 必拒");
    }

    //@ spec: RXS-0359
    #[test]
    fn leg_work_counters_deterministic() {
        // 耗时确定性代理:同输入双腿位级一致(步数/测试数/计数可复现)。
        let scene = cornell();
        let gb = gbuffer_prepass(&scene);
        let a = l1_leg_host(&scene, &gb);
        let b = l1_leg_host(&scene, &gb);
        assert_eq!(a, b, "L1 host 参照双跑位级一致");
        let c = l2_leg_host(&scene, &gb.sec_o, &gb.sec_d);
        let d = l2_leg_host(&scene, &gb.sec_o, &gb.sec_d);
        assert_eq!(c, d, "L2 host 腿双跑位级一致");
        assert!(a.iter().any(|s| s.work > 0));
        assert!(c.iter().all(|s| s.work > 0));
        // L3 两档镜像:hit lighting 与 simple 输出不同(档间可区分)。
        let s = l3_leg_host(&scene, &gb, L3ShadeMode::Simple);
        let hl = l3_leg_host(&scene, &gb, L3ShadeMode::HitLighting);
        assert!(s.iter().zip(hl.iter()).any(|(x, y)| x.rgb != y.rgb));
    }
}
