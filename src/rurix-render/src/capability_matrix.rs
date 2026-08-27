//! `capability_matrix` — G31+ 波 C Task C3 设备兼容矩阵与能力降级链系统化
//! (G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #50 兑现载体;host/safe 纯函数,渲染器面
//! 运行时降级裁决的**规范事实源**)。
//!
//! ## 事实流
//!
//! 设备探测归 rurix-rt `vk::probe_device_capability` + `bin/vk_capability_report`
//! (vendor/device id、RT/RayQuery、mesh shader、descriptor 面上限、显存 budget、
//! DLSS/FSR/TSR 三后端可用性 → `rurix.g31.capability_report.v1`);本模块消费其
//! host 侧镜像 [`CapabilityFacts`],对生产车道特性请求 [`FeatureRequest`] 按**降级
//! 映射闭集**逐链裁决 → [`ChainDecision`] 登记表(禁崩溃/禁静默错图:能力缺时
//! 确定性降级 + reason 携带缺失件字面,裁决集 digest 可重现进 evidence)。
//!
//! ## 降级映射闭集(六链,冻结序;每链 fail-closed)
//!
//! | 链 | 梯 | 能力需求(诚实来源) |
//! |---|---|---|
//! | `upscale` | `dlss_sr` → `fsr_3_1_5` → `tsr_device` | DLSS = vendor session 真建事实(NGX 动态加载 fail-closed,G13 M-a);FSR = D3D12 臂 session 真建事实;TSR 自研恒可用(需求 = Vulkan compute) |
//! | `hzb` | `on` → `off` | `rayQuery` + `accelerationStructure`(g31_window_present --hzb on 车道:逐 mesh 节点 BLAS 分解 + kernels/g31_hzb_{primary,shade}.rx 相机射线走 TLAS);off = 高成本如实(无遮挡剔除,全量场景渲染) |
//! | `restir` | `restir_high` → `megalights_low` | 显存 ≥ [`RESTIR_MIN_VRAM_BYTES`](reservoir 双带 + offset 表 + 空间重用快照;声明阈值);低档 = MegaLights 式均匀选灯(g31_restir_wiring --restir off 语义,RIS M=1 代数恒等) |
//! | `gi` | `on` → `off` | `rayQuery` + `accelerationStructure`(kernels/g16_gi_multibounce.rx / g14_3_direct_gi.rx:`AccelStruct` 形参 + `ray_query_initialize`) |
//! | `framegen` | `x2`/`x3` → `off` | 显存 ≥ [`FG_MIN_VRAM_BYTES`](prev/cur/mv/out RGBAf32 四缓冲 @1080p ≈ 100MB + 余量,声明阈值;kernels/g26_framegen.rx 纯图像空间 compute) |
//! | `texture_sampling` | `textures` → `constant_material` | `rayQuery` + `accelerationStructure` + `maxPerStageDescriptorStorageBuffers` ≥ [`TEXTURE_MIN_STORAGE_BUFFERS`](kernels/g31_texture_gi.rx:AccelStruct + 基座 7 + B4 五件 SSBO 侧表);常量材质 = textures off 车道现状语义 |
//!
//! 声明阈值 = 链定义自带策略常量(**如实标注 declared,非 measured**);能力布尔
//! 与上限全部为探测真值。AMD/Intel 真卡格 = 获得硬件后按同面补测(锚
//! G-MB1-6,milestones/g31/g31_compatibility_matrix.json DEV_ENV_DEGRADE 登记)。

use rurix_pkg::sha256;

/// ReSTIR 高档显存下界(声明阈值 1 GiB;reservoir 双带 + offset 表 + 空间重用
/// 8×8 网格快照的生产分辨率包络——低档 MegaLights 无 reservoir 表,无此门槛)。
pub const RESTIR_MIN_VRAM_BYTES: u64 = 1 << 30;
/// FG 显存下界(声明阈值 512 MiB;prev/cur/mv/out 四 RGBAf32 @1080p ≈ 100MB
/// + 历史/对齐余量——off 无生成帧缓冲面)。
pub const FG_MIN_VRAM_BYTES: u64 = 512 << 20;
/// 纹理采样链 SSBO 下界(g31_texture_gi 车道资源面事实:基座 7 绑定 + B4 追加
/// 24..=28 五件 = 12;见 g31_window_present.rs B4 注释面)。
pub const TEXTURE_MIN_STORAGE_BUFFERS: u32 = 12;

// ═══════════════════════ 请求/事实镜像(闭集) ═══════════════════════

/// 超分后端闭集(与 g14_3 车道 `--backend` 三臂同一字面)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UpscaleBackend {
    /// DLSS SR(Streamline 2.10.3 Vulkan interop 臂)。
    DlssSr,
    /// FSR 3.1.5(FidelityFX SDK 2.0.0 D3D12 臂)。
    Fsr315,
    /// 自研 TSR device(kernels/g13_tsr_* 经 vk::run_compute)。
    TsrDevice,
}

impl UpscaleBackend {
    /// 车道字面(`--backend` 闭集)。
    pub fn name(self) -> &'static str {
        match self {
            UpscaleBackend::DlssSr => "dlss_sr",
            UpscaleBackend::Fsr315 => "fsr_3_1_5",
            UpscaleBackend::TsrDevice => "tsr_device",
        }
    }
}

/// FG 档闭集(g31_window_present `--fg off|x2|x3` 同一字面)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FrameGenTier {
    /// 关(无双缓冲生成帧面)。
    Off,
    /// ×2 生成。
    X2,
    /// ×3 生成。
    X3,
}

impl FrameGenTier {
    /// 车道字面(`--fg` 闭集)。
    pub fn name(self) -> &'static str {
        match self {
            FrameGenTier::Off => "off",
            FrameGenTier::X2 => "x2",
            FrameGenTier::X3 => "x3",
        }
    }
}

/// 降级链 ID 闭集(六链,冻结序 = [`ChainId::ALL`])。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChainId {
    /// 超分臂 DLSS→FSR→TSR。
    Upscale,
    /// HZB 遮挡剔除 on→off。
    Hzb,
    /// ReSTIR 高档→MegaLights 低档。
    Restir,
    /// GI on→off。
    Gi,
    /// 帧生成 x2/x3→off。
    FrameGen,
    /// 纹理采样→常量材质。
    TextureSampling,
}

impl ChainId {
    /// 冻结字符串字面(注册表/登记面同字面)。
    pub fn name(self) -> &'static str {
        match self {
            ChainId::Upscale => "upscale",
            ChainId::Hzb => "hzb",
            ChainId::Restir => "restir",
            ChainId::Gi => "gi",
            ChainId::FrameGen => "framegen",
            ChainId::TextureSampling => "texture_sampling",
        }
    }

    /// 闭集全表(冻结序;裁决输出序 = 本表序,确定性)。
    pub const ALL: [ChainId; 6] = [
        ChainId::Upscale,
        ChainId::Hzb,
        ChainId::Restir,
        ChainId::Gi,
        ChainId::FrameGen,
        ChainId::TextureSampling,
    ];
}

/// 设备能力事实镜像(`rurix.g31.capability_report.v1` 的 host 侧消费面;
/// 字段闭集 = 六链判据实际消费项,全为探测真值,非 stable)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CapabilityFacts {
    /// `vendorID`(0x10DE=NVIDIA / 0x1002=AMD / 0x8086=Intel;登记面)。
    pub vendor_id: u32,
    /// `rayQuery` feature bit。
    pub ray_query: bool,
    /// `accelerationStructure` feature bit。
    pub acceleration_structure: bool,
    /// `maxPerStageDescriptorStorageBuffers` limits 真值。
    pub max_per_stage_descriptor_storage_buffers: u32,
    /// 有效显存(bytes)= `heapBudget`(VK_EXT_memory_budget 在位)否则
    /// DEVICE_LOCAL heap 求和——探测面已按此序折算,bin/单测同律。
    pub effective_vram_bytes: u64,
    /// DLSS 可用(vendor session 真建事实;NGX 动态加载 fail-closed)。
    pub dlss_available: bool,
    /// FSR 可用(D3D12 臂 session 真建事实)。
    pub fsr_available: bool,
    // TSR 不自字段:自研恒可用(需求 = Vulkan compute,设备面非空即有报告)。
}

/// 生产车道特性请求(g31_window_present / g14_3_pipeline_perf CLI 闭集镜像)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FeatureRequest {
    /// `--backend` 请求臂。
    pub upscale: UpscaleBackend,
    /// `--hzb on`。
    pub hzb_on: bool,
    /// `--restir on`(高档 reservoir;false = MegaLights 低档默认面)。
    pub restir_high: bool,
    /// `--gi on`。
    pub gi_on: bool,
    /// `--fg` 档。
    pub framegen: FrameGenTier,
    /// `--textures on`。
    pub textures_on: bool,
}

impl FeatureRequest {
    /// 全量最大请求(兼容矩阵逐格评估面:六链全要最高档)。
    pub fn full() -> Self {
        FeatureRequest {
            upscale: UpscaleBackend::DlssSr,
            hzb_on: true,
            restir_high: true,
            gi_on: true,
            framegen: FrameGenTier::X3,
            textures_on: true,
        }
    }
}

// ═══════════════════════ 降级裁决(fail-closed) ═══════════════════════

/// 单链裁决记录(登记面;`requested`/`selected` 字面值 ∈ 该链梯闭集)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChainDecision {
    /// 链 ID。
    pub chain: ChainId,
    /// 请求档字面。
    pub requested: String,
    /// 实际选中档字面(降级后)。
    pub selected: String,
    /// 是否发生降级(selected ≠ requested)。
    pub degraded: bool,
    /// 判据说明(降级时携带缺失件字面;未降级 = 需求满足登记)。
    pub reason: String,
}

/// RT 需求对(HZB/GI/纹理三链同源):rayQuery + accelerationStructure,
/// 返回缺失件字面表(空 = 满足)。
fn rt_missing(facts: &CapabilityFacts) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !facts.ray_query {
        missing.push("rt.ray_query(VK_KHR_ray_query feature 缺失)");
    }
    if !facts.acceleration_structure {
        missing.push("acceleration_structure(VK_KHR_acceleration_structure feature 缺失)");
    }
    missing
}

/// 六链降级裁决(fail-closed;输出序 = [`ChainId::ALL`] 冻结序,确定性)。
/// 每链:请求档能力需求满足 → 选请求档(reason = 需求满足);缺失 → 沿该链梯
/// 确定性降档(reason 携带缺失件字面,**登记不静默**);梯底(TSR/off/
/// megalights_low/constant_material)恒可选中——by construction 无崩溃路径、
/// 无静默错图路径。
pub fn resolve_chains(facts: &CapabilityFacts, request: &FeatureRequest) -> Vec<ChainDecision> {
    let mut out = Vec::with_capacity(ChainId::ALL.len());
    for chain in ChainId::ALL {
        out.push(resolve_one(facts, request, chain));
    }
    out
}

fn resolve_one(facts: &CapabilityFacts, request: &FeatureRequest, chain: ChainId) -> ChainDecision {
    match chain {
        ChainId::Upscale => {
            let requested = request.upscale;
            // 梯:DlssSr → Fsr315 → TsrDevice(逐级 fail-closed,梯底恒可用)。
            let (selected, reason) = match requested {
                UpscaleBackend::DlssSr => {
                    if facts.dlss_available {
                        (UpscaleBackend::DlssSr, "dlss_available 实测真建(session 全链在位)".to_owned())
                    } else if facts.fsr_available {
                        (UpscaleBackend::Fsr315, "dlss_available=false(NGX 动态加载 fail-closed)→ FSR 实测真建在位".to_owned())
                    } else {
                        (UpscaleBackend::TsrDevice, "dlss_available=false 且 fsr_available=false → TSR 自研恒可用(梯底)".to_owned())
                    }
                }
                UpscaleBackend::Fsr315 => {
                    if facts.fsr_available {
                        (UpscaleBackend::Fsr315, "fsr_available 实测真建(D3D12 臂全链在位)".to_owned())
                    } else {
                        (UpscaleBackend::TsrDevice, "fsr_available=false → TSR 自研恒可用(梯底)".to_owned())
                    }
                }
                UpscaleBackend::TsrDevice => {
                    (UpscaleBackend::TsrDevice, "TSR 自研恒可用(需求 = Vulkan compute)".to_owned())
                }
            };
            ChainDecision {
                chain,
                requested: requested.name().to_owned(),
                selected: selected.name().to_owned(),
                degraded: selected != requested,
                reason,
            }
        }
        ChainId::Hzb => {
            let requested = request.hzb_on;
            let missing = rt_missing(facts);
            let (selected, reason) = if !requested {
                ("off", "请求即 off(无降级)".to_owned())
            } else if missing.is_empty() {
                ("on", "rt.ray_query + acceleration_structure 双 feature 实测在位(BLAS 分解 + TLAS 相机射线面满足)".to_owned())
            } else {
                ("off", format!("HZB 车道需求缺失:{} → off(高成本如实:无遮挡剔除,全量场景渲染成本照实承担,禁静默错图)", missing.join(" + ")))
            };
            ChainDecision {
                chain,
                requested: if requested { "on" } else { "off" }.to_owned(),
                selected: selected.to_owned(),
                degraded: requested && selected == "off",
                reason,
            }
        }
        ChainId::Restir => {
            let requested = request.restir_high;
            let (selected, reason) = if !requested {
                ("megalights_low", "请求即 MegaLights 低档(无降级)".to_owned())
            } else if facts.effective_vram_bytes >= RESTIR_MIN_VRAM_BYTES {
                ("restir_high", format!("显存 {} ≥ 声明阈值 {}(reservoir 双带 + offset 表 + 空间重用快照包络满足)", facts.effective_vram_bytes, RESTIR_MIN_VRAM_BYTES))
            } else {
                ("megalights_low", format!("显存 {} < 声明阈值 {}(reservoir 表面不足)→ MegaLights 低档(均匀选灯 RIS M=1 语义,如实登记)", facts.effective_vram_bytes, RESTIR_MIN_VRAM_BYTES))
            };
            ChainDecision {
                chain,
                requested: if requested { "restir_high" } else { "megalights_low" }.to_owned(),
                selected: selected.to_owned(),
                degraded: requested && selected == "megalights_low",
                reason,
            }
        }
        ChainId::Gi => {
            let requested = request.gi_on;
            let missing = rt_missing(facts);
            let (selected, reason) = if !requested {
                ("off", "请求即 off(无降级)".to_owned())
            } else if missing.is_empty() {
                ("on", "rt.ray_query + acceleration_structure 双 feature 实测在位(GI kernel AccelStruct 形参面满足)".to_owned())
            } else {
                ("off", format!("GI kernel 需求缺失:{} → off(直接光现状语义,如实登记)", missing.join(" + ")))
            };
            ChainDecision {
                chain,
                requested: if requested { "on" } else { "off" }.to_owned(),
                selected: selected.to_owned(),
                degraded: requested && selected == "off",
                reason,
            }
        }
        ChainId::FrameGen => {
            let requested = request.framegen;
            let (selected, reason) = if requested == FrameGenTier::Off {
                (FrameGenTier::Off, "请求即 off(无降级)".to_owned())
            } else if facts.effective_vram_bytes >= FG_MIN_VRAM_BYTES {
                (requested, format!("显存 {} ≥ 声明阈值 {}(prev/cur/mv/out 四缓冲包络满足;纯图像空间 compute kernel)", facts.effective_vram_bytes, FG_MIN_VRAM_BYTES))
            } else {
                (FrameGenTier::Off, format!("显存 {} < 声明阈值 {}(生成帧缓冲面不足)→ off(双口径登记面维持,presented=real,如实登记)", facts.effective_vram_bytes, FG_MIN_VRAM_BYTES))
            };
            ChainDecision {
                chain,
                requested: requested.name().to_owned(),
                selected: selected.name().to_owned(),
                degraded: selected != requested,
                reason,
            }
        }
        ChainId::TextureSampling => {
            let requested = request.textures_on;
            let mut missing = rt_missing(facts);
            if facts.max_per_stage_descriptor_storage_buffers < TEXTURE_MIN_STORAGE_BUFFERS {
                missing.push("maxPerStageDescriptorStorageBuffers 不足 12(基座 7 + B4 五件 SSBO 侧表)");
            }
            let (selected, reason) = if !requested {
                ("constant_material", "请求即常量材质(textures off 现状语义,无降级)".to_owned())
            } else if missing.is_empty() {
                ("textures", format!("rt 双 feature + SSBO 面 {} ≥ 12 实测满足(逐三角贴图采样车道面满足)", facts.max_per_stage_descriptor_storage_buffers))
            } else {
                ("constant_material", format!("纹理采样车道需求缺失:{} → 常量材质(textures off 车道现状语义,如实登记)", missing.join(" + ")))
            };
            ChainDecision {
                chain,
                requested: if requested { "textures" } else { "constant_material" }.to_owned(),
                selected: selected.to_owned(),
                degraded: requested && selected == "constant_material",
                reason,
            }
        }
    }
}

// ═══════════════════════ 裁决集 canonical + digest(可重现) ═══════════════════════

/// 裁决集 canonical 文本(键序 = ChainId::ALL 冻结序;字段定界显式,
/// 无路径/时间戳——双次生成逐字节相等,RXS-0305 同律)。
pub fn decisions_canonical(decisions: &[ChainDecision]) -> String {
    let mut s = String::new();
    s.push_str("rurix.g31.chain-decisions.v1\n");
    for d in decisions {
        s.push_str(&format!(
            "{}|{}|{}|{}|{}\n",
            d.chain.name(),
            d.requested,
            d.selected,
            u8::from(d.degraded),
            d.reason
        ));
    }
    s
}

/// 裁决集 digest(SHA-256(canonical 文本) hex;复用 rurix-pkg 手写实现,
/// RXS-0306 同源——输出仍合法的可重现机器证明面)。
pub fn decisions_digest(decisions: &[ChainDecision]) -> String {
    sha256::hex_digest(decisions_canonical(decisions).as_bytes())
}

// ═══════════════ G35 第七链:粒子(加性登记;六链冻结闭集 0-byte) ═══════════════
//
// RFC-0049 §4.13(D-409 评审 F12 disposition)+ G35_CONTRACT G-G35-10
// `capability_chain_registered` 兑现面:particles 主链 gpu_particles → off +
// 碰撞臂 ray_query → depth_buffer → off,fail-closed 禁静默换臂。
// 六链 [`ChainId::ALL`]/[`resolve_chains`]/[`FeatureRequest`] 为冻结闭集,
// 本链以独立类型加性并列(既有消费方 0-byte),消费方 = 粒子车道装配期。

/// 粒子车道单 pass SSBO 需求下界(诚实来源 = kernels/g35_particle_compact.rx
/// 21 件绑定:params + flags + scan_out + A 九流 + B 九流;G35-2 门在案)。
pub const PARTICLES_MIN_STORAGE_BUFFERS: u32 = 21;

/// 粒子碰撞臂档位闭集(g35_collision_device `--collision` 同字面;
/// RFC-0049 §4.13 三档梯)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParticleCollisionTier {
    /// 同帧 TLAS ray query 精确碰撞(G35-5 门 g35.wave5.collision 在案)。
    RayQuery,
    /// 深度缓冲对照臂(屏幕空间局限如实登记,教育对照档)。
    DepthBuffer,
    /// 关(粒子无碰撞,纯力场积分)。
    Off,
}

impl ParticleCollisionTier {
    /// 车道字面(`--collision` 闭集)。
    pub fn name(self) -> &'static str {
        match self {
            ParticleCollisionTier::RayQuery => "ray_query",
            ParticleCollisionTier::DepthBuffer => "depth_buffer",
            ParticleCollisionTier::Off => "off",
        }
    }
}

/// 粒子链请求(g35_particle_lane `--particles` + g35_collision_device
/// `--collision` CLI 闭集镜像)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParticlesRequest {
    /// `--particles on`。
    pub particles_on: bool,
    /// 碰撞臂请求档。
    pub collision: ParticleCollisionTier,
}

impl ParticlesRequest {
    /// 全量最大请求(gpu_particles + ray_query 碰撞)。
    pub fn full() -> Self {
        ParticlesRequest {
            particles_on: true,
            collision: ParticleCollisionTier::RayQuery,
        }
    }
}

/// 粒子链裁决记录(主链 + 碰撞臂双梯合一登记;字面值 ∈ 各梯闭集)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParticlesChainDecision {
    /// 主链请求档字面(`gpu_particles`/`off`)。
    pub requested: String,
    /// 主链选中档字面。
    pub selected: String,
    /// 碰撞臂请求档字面。
    pub collision_requested: String,
    /// 碰撞臂选中档字面(主链 off ⇒ 恒 off)。
    pub collision_selected: String,
    /// 任一梯发生降级即 true(登记不静默)。
    pub degraded: bool,
    /// 判据说明(降级携带缺失件字面)。
    pub reason: String,
}

/// 粒子链降级裁决(fail-closed;梯底 off 恒可选中——无崩溃/无静默错图路径,
/// 六链 [`resolve_one`] 同律)。主链需求 = Vulkan compute(设备面非空即有
/// 报告,TSR 同律)+ `maxPerStageDescriptorStorageBuffers` ≥
/// [`PARTICLES_MIN_STORAGE_BUFFERS`];碰撞臂 ray_query 档需求 = rt 双
/// feature([`rt_missing`] 同源),缺失 → depth_buffer(**显式降级登记,
/// 禁静默换臂**);depth_buffer/off 档零额外需求。
pub fn resolve_particles_chain(
    facts: &CapabilityFacts,
    request: &ParticlesRequest,
) -> ParticlesChainDecision {
    let requested = if request.particles_on { "gpu_particles" } else { "off" };
    let ssbo_ok = facts.max_per_stage_descriptor_storage_buffers >= PARTICLES_MIN_STORAGE_BUFFERS;
    let (selected, mut reason) = if !request.particles_on {
        ("off", "请求即 off(无降级)".to_owned())
    } else if ssbo_ok {
        (
            "gpu_particles",
            format!(
                "SSBO 面 {} ≥ {}(g35_particle_compact 21 件绑定包络满足;compute 基线 = 设备面非空)",
                facts.max_per_stage_descriptor_storage_buffers, PARTICLES_MIN_STORAGE_BUFFERS
            ),
        )
    } else {
        (
            "off",
            format!(
                "粒子车道需求缺失:maxPerStageDescriptorStorageBuffers {} < {}(compact 21 件绑定面)→ off(无粒子面如实,禁静默错图)",
                facts.max_per_stage_descriptor_storage_buffers, PARTICLES_MIN_STORAGE_BUFFERS
            ),
        )
    };
    let (collision_selected, collision_reason) = if selected == "off" {
        ("off", "主链 off ⇒ 碰撞臂恒 off".to_owned())
    } else {
        match request.collision {
            ParticleCollisionTier::RayQuery => {
                let missing = rt_missing(facts);
                if missing.is_empty() {
                    (
                        "ray_query",
                        "rt 双 feature 实测在位(同帧 TLAS 碰撞面满足)".to_owned(),
                    )
                } else {
                    (
                        "depth_buffer",
                        format!(
                            "碰撞 ray_query 档需求缺失:{} → depth_buffer(屏幕空间局限如实登记,显式降级禁静默换臂)",
                            missing.join(" + ")
                        ),
                    )
                }
            }
            ParticleCollisionTier::DepthBuffer => {
                ("depth_buffer", "请求即 depth_buffer(无降级)".to_owned())
            }
            ParticleCollisionTier::Off => ("off", "请求即 off(无降级)".to_owned()),
        }
    };
    reason.push_str(";碰撞臂:");
    reason.push_str(&collision_reason);
    let collision_requested = request.collision.name();
    ParticlesChainDecision {
        requested: requested.to_owned(),
        selected: selected.to_owned(),
        collision_requested: collision_requested.to_owned(),
        collision_selected: collision_selected.to_owned(),
        degraded: (request.particles_on && selected == "off")
            || (selected == "gpu_particles" && collision_selected != collision_requested),
        reason,
    }
}

/// 粒子链裁决 canonical 文本(六链 [`decisions_canonical`] 同律;双次生成
/// 逐字节相等)。
pub fn particles_decision_canonical(d: &ParticlesChainDecision) -> String {
    format!(
        "rurix.g35.particles-chain-decision.v1\nparticles|{}|{}|{}|{}|{}|{}\n",
        d.requested,
        d.selected,
        d.collision_requested,
        d.collision_selected,
        u8::from(d.degraded),
        d.reason
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NVIDIA Ada 实测镜像(2026-08-25 vk_capability_report 真跑事实:
    /// RTX 4070 Ti vendor 0x10DE;十 feature 全真;SSBO 上限 1048576;
    /// heapBudget 11771314176;DLSS/FSR 双臂真建 Ok)。
    fn ada_facts() -> CapabilityFacts {
        CapabilityFacts {
            vendor_id: 0x10DE,
            ray_query: true,
            acceleration_structure: true,
            max_per_stage_descriptor_storage_buffers: 1_048_576,
            effective_vram_bytes: 11_771_314_176,
            dlss_available: true,
            fsr_available: true,
        }
    }

    /// AMD 类镜像(无 NGX 宿主 → dlss_available=false;FSR D3D12 臂厂商中立
    /// 真建;RT 面按 RDNA 实测假设在位;16 GiB)。
    fn amd_like_facts() -> CapabilityFacts {
        CapabilityFacts {
            vendor_id: 0x1002,
            ray_query: true,
            acceleration_structure: true,
            max_per_stage_descriptor_storage_buffers: 1_048_576,
            effective_vram_bytes: 16 << 30,
            dlss_available: false,
            fsr_available: true,
        }
    }

    /// Intel 类镜像(无 NGX/无 FSR 实测(D3D12 臂假设未验);RT 面缺失;
    /// SSBO 面上限低位;8 GiB)。
    fn intel_like_facts() -> CapabilityFacts {
        CapabilityFacts {
            vendor_id: 0x8086,
            ray_query: false,
            acceleration_structure: false,
            max_per_stage_descriptor_storage_buffers: 8,
            effective_vram_bytes: 8 << 30,
            dlss_available: false,
            fsr_available: false,
        }
    }

    fn decision<'a>(ds: &'a [ChainDecision], chain: ChainId) -> &'a ChainDecision {
        ds.iter().find(|d| d.chain == chain).expect("六链全产")
    }

    /// GREEN:NVIDIA Ada 实测镜像 + 全量最大请求 → 六链全选请求档(零降级),
    /// digest 双跑可重现。
    #[test]
    fn ada_full_request_zero_degradation() {
        let ds = resolve_chains(&ada_facts(), &FeatureRequest::full());
        assert_eq!(ds.len(), 6, "六链全产");
        for d in &ds {
            assert!(!d.degraded, "{:?} 不应降级: {}", d.chain, d.reason);
            assert_eq!(d.requested, d.selected, "{:?} 选档 = 请求档", d.chain);
        }
        assert_eq!(decision(&ds, ChainId::Upscale).selected, "dlss_sr");
        assert_eq!(decision(&ds, ChainId::Hzb).selected, "on");
        assert_eq!(decision(&ds, ChainId::Restir).selected, "restir_high");
        assert_eq!(decision(&ds, ChainId::Gi).selected, "on");
        assert_eq!(decision(&ds, ChainId::FrameGen).selected, "x3");
        assert_eq!(decision(&ds, ChainId::TextureSampling).selected, "textures");
        // digest 可重现(双跑位级一致)。
        assert_eq!(decisions_digest(&ds), decisions_digest(&ds));
        // 输出序 = ChainId::ALL 冻结序(确定性)。
        for (d, c) in ds.iter().zip(ChainId::ALL) {
            assert_eq!(d.chain, c);
        }
    }

    /// 超分链梯:DLSS 缺 → FSR;双缺 → TSR(梯底);FSR 请求缺 → TSR;
    /// TSR 请求恒选中(自研恒可用)。每跳 reason 携带缺失件字面。
    #[test]
    fn upscale_ladder_fail_closed() {
        let mut facts = ada_facts();
        // ① DLSS 缺 → FSR(degraded,reason 携带 dlss_available=false)。
        facts.dlss_available = false;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        let d = decision(&ds, ChainId::Upscale);
        assert!(d.degraded && d.selected == "fsr_3_1_5" && d.requested == "dlss_sr");
        assert!(d.reason.contains("dlss_available=false"), "reason 携带缺失件: {}", d.reason);
        // ② 双缺 → TSR(梯底恒可选中,by construction 无崩溃)。
        facts.fsr_available = false;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        let d = decision(&ds, ChainId::Upscale);
        assert!(d.degraded && d.selected == "tsr_device");
        assert!(d.reason.contains("fsr_available=false"));
        // ③ FSR 请求 + FSR 缺 → TSR。
        let req = FeatureRequest { upscale: UpscaleBackend::Fsr315, ..FeatureRequest::full() };
        let ds = resolve_chains(&facts, &req);
        let d = decision(&ds, ChainId::Upscale);
        assert!(d.degraded && d.selected == "tsr_device" && d.requested == "fsr_3_1_5");
        // ④ TSR 请求:双缺下仍选中(自研恒可用,零降级)。
        let req = FeatureRequest { upscale: UpscaleBackend::TsrDevice, ..FeatureRequest::full() };
        let ds = resolve_chains(&facts, &req);
        let d = decision(&ds, ChainId::Upscale);
        assert!(!d.degraded && d.selected == "tsr_device");
        // ⑤ FSR 在位时 FSR 请求零降级。
        facts.fsr_available = true;
        let req = FeatureRequest { upscale: UpscaleBackend::Fsr315, ..FeatureRequest::full() };
        let ds = resolve_chains(&facts, &req);
        assert!(!decision(&ds, ChainId::Upscale).degraded);
    }

    /// HZB 链:ray_query 缺 → off(reason 携带 rt.ray_query);双缺 → reason
    /// 双件全列;请求 off → 无降级(选 off 非降级语义)。
    #[test]
    fn hzb_chain_fail_closed() {
        let mut facts = ada_facts();
        facts.ray_query = false;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        let d = decision(&ds, ChainId::Hzb);
        assert!(d.degraded && d.selected == "off" && d.requested == "on");
        assert!(d.reason.contains("rt.ray_query"), "reason 携带缺失件: {}", d.reason);
        assert!(d.reason.contains("高成本如实"), "off 语义如实标注: {}", d.reason);
        facts.acceleration_structure = false;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        let d = decision(&ds, ChainId::Hzb);
        assert!(d.reason.contains("rt.ray_query") && d.reason.contains("acceleration_structure"));
        // 请求 off:selected=off 但非降级(请求=选中)。
        let req = FeatureRequest { hzb_on: false, ..FeatureRequest::full() };
        let ds = resolve_chains(&facts, &req);
        let d = decision(&ds, ChainId::Hzb);
        assert!(!d.degraded && d.selected == "off");
    }

    /// ReSTIR 链:显存 < 1 GiB → megalights_low;恰阈值 → 高档维持(≥ 判据);
    /// 请求低档 → 无降级。
    #[test]
    fn restir_chain_vram_threshold() {
        let mut facts = ada_facts();
        facts.effective_vram_bytes = RESTIR_MIN_VRAM_BYTES - 1;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        let d = decision(&ds, ChainId::Restir);
        assert!(d.degraded && d.selected == "megalights_low" && d.requested == "restir_high");
        assert!(d.reason.contains("声明阈值"), "阈值如实标注 declared: {}", d.reason);
        // 恰阈值(≥ 判据)→ 高档维持。
        facts.effective_vram_bytes = RESTIR_MIN_VRAM_BYTES;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        assert!(!decision(&ds, ChainId::Restir).degraded);
        // 请求低档 → 无降级。
        let req = FeatureRequest { restir_high: false, ..FeatureRequest::full() };
        facts.effective_vram_bytes = 1;
        let ds = resolve_chains(&facts, &req);
        let d = decision(&ds, ChainId::Restir);
        assert!(!d.degraded && d.selected == "megalights_low");
    }

    /// GI 链:ray_query 缺 → off(reason 携带缺失件);请求 off → 无降级。
    #[test]
    fn gi_chain_fail_closed() {
        let mut facts = ada_facts();
        facts.ray_query = false;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        let d = decision(&ds, ChainId::Gi);
        assert!(d.degraded && d.selected == "off" && d.requested == "on");
        assert!(d.reason.contains("rt.ray_query"));
        let req = FeatureRequest { gi_on: false, ..FeatureRequest::full() };
        let ds = resolve_chains(&facts, &req);
        assert!(!decision(&ds, ChainId::Gi).degraded);
    }

    /// FG 链:显存 < 512 MiB → off(x2/x3 同律);恰阈值维持;off 请求无降级。
    #[test]
    fn framegen_chain_fail_closed() {
        let mut facts = ada_facts();
        facts.effective_vram_bytes = FG_MIN_VRAM_BYTES - 1;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        let d = decision(&ds, ChainId::FrameGen);
        assert!(d.degraded && d.selected == "off" && d.requested == "x3");
        assert!(d.reason.contains("presented=real"), "双口径登记面如实: {}", d.reason);
        // x2 同律。
        let req = FeatureRequest { framegen: FrameGenTier::X2, ..FeatureRequest::full() };
        let ds = resolve_chains(&facts, &req);
        assert!(decision(&ds, ChainId::FrameGen).degraded);
        // 恰阈值维持。
        facts.effective_vram_bytes = FG_MIN_VRAM_BYTES;
        let ds = resolve_chains(&facts, &req);
        let d = decision(&ds, ChainId::FrameGen);
        assert!(!d.degraded && d.selected == "x2");
        // off 请求:低显存下仍无降级。
        facts.effective_vram_bytes = 1;
        let req = FeatureRequest { framegen: FrameGenTier::Off, ..FeatureRequest::full() };
        let ds = resolve_chains(&facts, &req);
        assert!(!decision(&ds, ChainId::FrameGen).degraded);
    }

    /// 纹理链:ray_query 缺 → 常量材质;SSBO 上限 11 → 常量材质(12 = 下界,
    /// 基座 7 + B4 五件事实);恰 12 维持;请求常量材质无降级。
    #[test]
    fn texture_chain_fail_closed() {
        let mut facts = ada_facts();
        facts.ray_query = false;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        let d = decision(&ds, ChainId::TextureSampling);
        assert!(d.degraded && d.selected == "constant_material" && d.requested == "textures");
        facts = ada_facts();
        facts.max_per_stage_descriptor_storage_buffers = TEXTURE_MIN_STORAGE_BUFFERS - 1;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        let d = decision(&ds, ChainId::TextureSampling);
        assert!(d.degraded && d.selected == "constant_material");
        assert!(d.reason.contains("SSBO"), "缺失件携带 SSBO 面: {}", d.reason);
        facts.max_per_stage_descriptor_storage_buffers = TEXTURE_MIN_STORAGE_BUFFERS;
        let ds = resolve_chains(&facts, &FeatureRequest::full());
        assert!(!decision(&ds, ChainId::TextureSampling).degraded);
        let req = FeatureRequest { textures_on: false, ..FeatureRequest::full() };
        facts.max_per_stage_descriptor_storage_buffers = 0;
        let ds = resolve_chains(&facts, &req);
        assert!(!decision(&ds, ChainId::TextureSampling).degraded);
    }

    /// AMD 类镜像:upscale → FSR(DLSS 缺,厂商中立臂真建);其余链零降级
    /// (RT 面在位 + 显存足)——跨厂商降级面确定性。
    #[test]
    fn amd_like_degrades_only_upscale() {
        let ds = resolve_chains(&amd_like_facts(), &FeatureRequest::full());
        let d = decision(&ds, ChainId::Upscale);
        assert!(d.degraded && d.selected == "fsr_3_1_5");
        for chain in [ChainId::Hzb, ChainId::Restir, ChainId::Gi, ChainId::FrameGen, ChainId::TextureSampling] {
            assert!(!decision(&ds, chain).degraded, "{chain:?} 不应降级");
        }
    }

    /// Intel 类镜像:upscale → TSR(梯底);HZB/GI → off;纹理 → 常量材质
    /// (RT 缺 + SSBO 不足);ReSTIR/FG 维持(显存足)——多链同降确定性。
    #[test]
    fn intel_like_multi_chain_degradation() {
        let ds = resolve_chains(&intel_like_facts(), &FeatureRequest::full());
        assert_eq!(decision(&ds, ChainId::Upscale).selected, "tsr_device");
        assert_eq!(decision(&ds, ChainId::Hzb).selected, "off");
        assert_eq!(decision(&ds, ChainId::Gi).selected, "off");
        assert_eq!(decision(&ds, ChainId::TextureSampling).selected, "constant_material");
        assert_eq!(decision(&ds, ChainId::Restir).selected, "restir_high");
        assert_eq!(decision(&ds, ChainId::FrameGen).selected, "x3");
        let degraded_count = ds.iter().filter(|d| d.degraded).count();
        assert_eq!(degraded_count, 4, "恰四链降级(upscale/hzb/gi/texture)");
    }

    /// 输出合法性闭集机核:遍历事实组合面,selected 恒 ∈ 该链梯闭集
    /// (禁静默错图的机器证明——选中档永远合法)。
    #[test]
    fn selected_always_in_closed_ladder() {
        let ladders: [(ChainId, &[&str]); 6] = [
            (ChainId::Upscale, &["dlss_sr", "fsr_3_1_5", "tsr_device"]),
            (ChainId::Hzb, &["on", "off"]),
            (ChainId::Restir, &["restir_high", "megalights_low"]),
            (ChainId::Gi, &["on", "off"]),
            (ChainId::FrameGen, &["off", "x2", "x3"]),
            (ChainId::TextureSampling, &["textures", "constant_material"]),
        ];
        for rq in [false, true] {
            for as_ in [false, true] {
                for dlss in [false, true] {
                    for fsr in [false, true] {
                        for vram in [0u64, FG_MIN_VRAM_BYTES, RESTIR_MIN_VRAM_BYTES, 12 << 30] {
                            for ssbo in [0u32, TEXTURE_MIN_STORAGE_BUFFERS, 1_048_576] {
                                let facts = CapabilityFacts {
                                    vendor_id: 0x10DE,
                                    ray_query: rq,
                                    acceleration_structure: as_,
                                    max_per_stage_descriptor_storage_buffers: ssbo,
                                    effective_vram_bytes: vram,
                                    dlss_available: dlss,
                                    fsr_available: fsr,
                                };
                                let ds = resolve_chains(&facts, &FeatureRequest::full());
                                for (chain, ladder) in &ladders {
                                    let d = decision(&ds, *chain);
                                    assert!(
                                        ladder.contains(&d.selected.as_str()),
                                        "{chain:?} selected={} 越梯闭集(facts={facts:?})",
                                        d.selected
                                    );
                                    // degraded ⇔ selected ≠ requested(恒等)。
                                    assert_eq!(d.degraded, d.selected != d.requested);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// digest 敏感性:任一事实面变化 → 裁决集 digest 必变(防静默换档);
    /// 同事实双跑 digest 位级一致(可重现)。
    #[test]
    fn digest_reproducible_and_tamper_sensitive() {
        let base = decisions_digest(&resolve_chains(&ada_facts(), &FeatureRequest::full()));
        assert_eq!(base.len(), 64, "sha256 hex 64 字符");
        let mut facts = ada_facts();
        facts.dlss_available = false;
        let alt = decisions_digest(&resolve_chains(&facts, &FeatureRequest::full()));
        assert_ne!(base, alt, "DLSS 缺失 → 裁决变 → digest 必变");
        facts = ada_facts();
        facts.ray_query = false;
        let alt2 = decisions_digest(&resolve_chains(&facts, &FeatureRequest::full()));
        assert_ne!(base, alt2, "ray_query 缺失 → digest 必变");
        // 请求面变化(非降级)亦变:hzb 请求 off。
        let req = FeatureRequest { hzb_on: false, ..FeatureRequest::full() };
        let alt3 = decisions_digest(&resolve_chains(&ada_facts(), &req));
        assert_ne!(base, alt3, "请求面变化 → digest 必变(请求字面进 canonical)");
    }

    /// 注册表锚:milestones/g31/g31_compatibility_matrix.json 的六链字面与
    /// 本模块 CHAINS 闭集同源(chain id + 梯档字面 + 声明阈值数字三向互核,
    /// 防注册表漂移;0-byte 不回写,只读消费)。
    #[test]
    fn registry_json_chains_anchored() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../milestones/g31/g31_compatibility_matrix.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("注册表须可读 {}: {e}", path.display()));
        for chain in ChainId::ALL {
            assert!(
                text.contains(&format!("\"chain\": \"{}\"", chain.name())),
                "注册表缺链 {} ",
                chain.name()
            );
        }
        for lit in [
            "dlss_sr", "fsr_3_1_5", "tsr_device", "restir_high", "megalights_low",
            "constant_material", "textures",
        ] {
            assert!(text.contains(lit), "注册表缺梯档字面 {lit}");
        }
        // 声明阈值数字锚(1 GiB / 512 MiB / 12 SSBO)。
        assert!(text.contains(&(1u64 << 30).to_string()), "RESTIR_MIN_VRAM_BYTES 字面锚");
        assert!(text.contains(&(512u64 << 20).to_string()), "FG_MIN_VRAM_BYTES 字面锚");
        // AMD/Intel 格 DEV_ENV_DEGRADE 如实登记锚(G-MB1-6 获得硬件后补测)。
        assert!(text.contains("dev_env_degrade"), "AMD/Intel 格降级登记");
        assert!(text.contains("G-MB1-6"), "G-MB1-6 锚");
    }
}

#[cfg(test)]
mod g35_particles_chain_tests {
    use super::*;

    /// 全能力事实(Ada 实测镜像同值;六链 tests 夹具字面独立复制,
    /// 不触碰冻结 tests 模块)。
    fn full_facts() -> CapabilityFacts {
        CapabilityFacts {
            vendor_id: 0x10DE,
            ray_query: true,
            acceleration_structure: true,
            max_per_stage_descriptor_storage_buffers: 1_048_576,
            effective_vram_bytes: 11_771_314_176,
            dlss_available: true,
            fsr_available: true,
        }
    }

    /// GREEN:全能力 + 全量请求 → gpu_particles + ray_query 零降级。
    #[test]
    fn full_caps_zero_degradation() {
        let d = resolve_particles_chain(&full_facts(), &ParticlesRequest::full());
        assert_eq!(d.selected, "gpu_particles");
        assert_eq!(d.collision_selected, "ray_query");
        assert!(!d.degraded, "全能力不应降级: {}", d.reason);
        // canonical 双跑位级一致(确定性)。
        assert_eq!(
            particles_decision_canonical(&d),
            particles_decision_canonical(&d)
        );
    }

    /// RED→显式降级:SSBO 上限 20 < 21 → 主链 off,reason 携带缺失件字面。
    #[test]
    fn ssbo_shortfall_degrades_to_off_with_reason() {
        let mut facts = full_facts();
        facts.max_per_stage_descriptor_storage_buffers = 20;
        let d = resolve_particles_chain(&facts, &ParticlesRequest::full());
        assert_eq!(d.selected, "off");
        assert!(d.degraded);
        assert!(
            d.reason.contains("maxPerStageDescriptorStorageBuffers 20 < 21"),
            "reason 须携带缺失件字面: {}",
            d.reason
        );
        assert_eq!(d.collision_selected, "off", "主链 off ⇒ 碰撞臂恒 off");
    }

    /// RED→显式降级:rt 双 feature 缺失 → 碰撞臂 ray_query 降 depth_buffer
    /// (禁静默换臂:degraded=true + reason 携带缺失件)。
    #[test]
    fn rt_missing_degrades_collision_to_depth_buffer() {
        let mut facts = full_facts();
        facts.ray_query = false;
        facts.acceleration_structure = false;
        let d = resolve_particles_chain(&facts, &ParticlesRequest::full());
        assert_eq!(d.selected, "gpu_particles", "主链不依赖 rt 面");
        assert_eq!(d.collision_selected, "depth_buffer");
        assert!(d.degraded, "换臂必须显式登记降级");
        assert!(d.reason.contains("ray_query"), "reason 须携带缺失件: {}", d.reason);
    }

    /// 请求即 off/depth_buffer → 零降级(请求档恒可满足面)。
    #[test]
    fn off_and_depth_requests_never_degrade() {
        let d_off = resolve_particles_chain(
            &full_facts(),
            &ParticlesRequest {
                particles_on: false,
                collision: ParticleCollisionTier::Off,
            },
        );
        assert_eq!(d_off.selected, "off");
        assert!(!d_off.degraded);
        let d_depth = resolve_particles_chain(
            &full_facts(),
            &ParticlesRequest {
                particles_on: true,
                collision: ParticleCollisionTier::DepthBuffer,
            },
        );
        assert_eq!(d_depth.collision_selected, "depth_buffer");
        assert!(!d_depth.degraded);
    }

    /// 六链冻结闭集 0-byte 互核:ALL 仍为六链且不含 particles 字面
    /// (第七链 = 加性并列独立面)。
    #[test]
    fn six_chain_closed_set_untouched() {
        assert_eq!(ChainId::ALL.len(), 6, "六链冻结闭集不扩");
        assert!(
            ChainId::ALL.iter().all(|c| c.name() != "particles"),
            "particles 不进六链闭集"
        );
    }
}
