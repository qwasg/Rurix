//! G7.5:RD-038 余项的 device 腿 —— **VSM 页内深度光栅 + 阴影采样**(字面分项
//! 「VSM 深度」)与 **TSR 空间超分核**(字面分项「TAA-TSR」的 TSR 腿),对 host
//! oracle 逐值对拍(验收门 G-G7-7;CI 步骤 95 device 段)。
//!
//! ## 与既有波次的关系
//! - W1 `vsm_page_mark.rx` 只把「虚拟页 ID → 位图标记」搬上 device;**页内深度**
//!   与**采样**此前全在 host(`RD038_LITERAL_MATRIX.md` §1 行 5「部分」)。本模块
//!   补齐这两条腿。
//! - W1 `taa.rx` 已 device 化时域 resolve;TSR 此前**纯 host**(§1 行 8「部分」)。
//!   本模块补 TSR 的空间超分核。
//!
//! ## host oracle 纪律(沿 RFC-0018 §D2 / G7.4 口径)
//! oracle **不参与成功路径**:判据完全由 device readback 与 host 参照的差值构成,
//! 无任何 host 结果回填 device。`rurix-render` 数值语义 **0-byte**(本波未改
//! `shadow::vsm` / `temporal::tsr` 任何一行)。
//!
//! ## 输入 provenance(与 W1/W2/W3c 同纪律,evidence 字段化)
//! - VSM:灯空间三角形 = host `LightBasis::to_light` 预变换(**场景装配面**);
//!   逐页 `(origin, page_world, z_range)` = host 页表/窗口状态快照(**配置面**);
//!   device 真做「逐纹素 × 逐三角形」的边函数覆盖、重心深度与 min 归约。
//! - VSM 采样:灯基/相机/`base_radius`/`depth_bias`/逐级窗口 = 配置面;device 真做
//!   距离、选级、回退环、页表寻址与解包、纹素定位、深度比较。
//! - TSR:输入色图取自**冻结场景 GBuffer**(深度 + 世界法线合成,含真实轮廓硬边,
//!   正是 Catmull-Rom 负瓣振铃的压力面),jitter 取冻结 Halton 序列首项;device 真做
//!   16 tap 加权、邻域包络与抗振铃钳制。

use rurix_render::shadow::clipmap::{ClipmapConfig, LightBasis};
use rurix_render::shadow::page_table::PageTableEntry;
use rurix_render::shadow::vsm::{Vsm, VsmConfig};
use rurix_render::temporal::common::jitter_sequence;
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::tsr::TsrUpscaler;
use rurix_render::temporal::upscale::UpscaleInputs;
use rurix_rt::render_exec::{
    self, Bindings, BufferDesc, BufferUsage, ComputePass, DispatchSpec, KernelWave, Pass, Readback,
    ResourceDesc,
};

use crate::scene::{CAMERA, Uc06Scene, VSM_LIGHT_DIR};

const VSM_DEPTH_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vsm_depth_raster.spv"));
const VSM_SAMPLE_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vsm_sample.spv"));
const TSR_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsr_resample.spv"));

/// 页边长 / 页纹素数(`shadow::clipmap` 冻结常量的本地镜像,仅作索引算术)。
const PAGE_DIM: usize = 128;
const PAGE_TEXELS: usize = PAGE_DIM * PAGE_DIM;
/// 单级页表槽位数(128×128 项)。
const PAGE_TABLE_SLOTS: usize = PAGE_DIM * PAGE_DIM;

/// VSM device 腿的**测试用** clipmap 配置(沿 `match_w1_vsm_page_mark` 先例:device
/// 对拍用专置配置,不占用 `shading::make_vsm` 的 demo 配置面)。
///
/// `base_radius = 4.0` ⇒ `page_world(0) = 2·4/128 = 0.0625`;冻结场景灯平面跨度
/// 约 3.5 世界单位 ⇒ 覆盖页规模可控,且采样点距相机 ∈ [2, 5] 使 `select_level`
/// 在 0/1 两级间真实取值(回退环非空转)。
const VSM_LEVELS: u8 = 4;
const VSM_BASE_RADIUS: f32 = 4.0;
const VSM_POOL_PAGES: u16 = 128;
const VSM_DEPTH_BIAS: f32 = 1e-3;
/// page_mark 驱动深度图边长(标记页数 ≤ 像素数;GPU 侧 gather 规模 = 页数 × 16384)。
const VSM_MARK_DIM: u32 = 8;

/// TSR 对拍分辨率:内部 → 输出 **2×**(冻结 `RenderConfig::internal = out/2` 契约)。
const TSR_IN_W: u32 = 64;
const TSR_IN_H: u32 = 36;
const TSR_OUT_W: u32 = TSR_IN_W * 2;
const TSR_OUT_H: u32 = TSR_IN_H * 2;

/// 冻结容差(**measured 后冻结**,沿 G7.4 口径:阈值只来自真实 GPU 输出,
/// 不为过门放宽;`measured_* <= tol_*` 成对机验)。
///
/// ## 为什么深度/TSR 不是逐位相等(如实登记,非容差放宽借口)
/// device 与 host 的**表达式与求值序逐字一致**,残差唯一来源 = **浮点收缩**:
/// SPIR-V 侧未加 `NoContraction` 装饰,驱动可把 `a*b − c*d`(边函数)与
/// `acc + w*p`(tap 累加)融合为 FMA,而 Rust host 不自动收缩。实测量级
/// (深度 3.58e-7 / TSR 1.49e-8)正是 f32 单位舍入(ULP(1.0) = 1.19e-7)的
/// 数倍,与该解释相符。**VisBuffer(W2)之所以能逐位相等**是因为其判据落在
/// 量化后的 30 位整数域,收缩差被量化吸收;VSM 深度与 TSR 输出是 f32 域直比,
/// 故按本波纪律 measured → 冻结,不改 host oracle 数值语义。
pub mod tol {
    /// VSM 页内深度(f32 域;measured 3.576278687e-7 @ RTX 4070 Ti/driver 620.02)。
    pub const VSM_DEPTH: f32 = 1e-6;
    /// VSM 采样输出为 0/1 **二值**:任何不一致都是级/页/纹素定位分歧而非舍入,
    /// 故**零容差**(measured 0.0,764 采样零 mismatch)。
    pub const VSM_SAMPLE: f32 = 0.0;
    /// TSR 16 tap 加权 + 抗振铃钳制(measured 1.490116119e-8)。
    pub const TSR: f32 = 1e-7;
}

/// G7.5 余项对拍结果(`--g75-residuals` JSON 与步骤 95 evidence 的字段源)。
#[derive(Debug, Clone)]
pub struct G75MatchResults {
    pub device_name: String,
    // ── VSM 页内深度光栅 ──
    /// 参与对拍的物理页数(host「脏且驻留」枚举序)。
    pub vsm_pages: u32,
    /// 对拍纹素数 = 页数 × 128×128。
    pub vsm_depth_texels: u32,
    /// 参与光栅的三角形数(冻结场景 `world_tris`)。
    pub vsm_triangles: u32,
    /// **逐位相等**纹素数(应 == `vsm_depth_texels`)。
    pub vsm_depth_bitexact_texels: u32,
    /// 被真实覆盖(深度 < 1.0 远平面)的纹素数——证明判据不空转。
    pub vsm_depth_covered_texels: u32,
    pub measured_vsm_depth_max_abs: f32,
    // ── VSM 阴影采样 ──
    pub vsm_samples: u32,
    /// device/host 采样值不一致数(0/1 二值,应为 0)。
    pub vsm_sample_mismatches: u32,
    /// device 侧判为遮蔽的采样比例——证明采样非退化全 lit。
    pub vsm_shadowed_ratio_device: f32,
    pub measured_vsm_sample_max_abs: f32,
    // ── TSR 空间超分 ──
    pub tsr_in_w: u32,
    pub tsr_in_h: u32,
    pub tsr_out_w: u32,
    pub tsr_out_h: u32,
    /// 对拍标量数 = out_w × out_h × 3。
    pub tsr_channels: u32,
    pub tsr_bitexact_channels: u32,
    /// 抗振铃钳制**真实生效**的通道数(host 侧统计:加权和越出邻域包络)——
    /// 证明对拍覆盖了 Catmull-Rom 负瓣分支,不是只跑了平凡加权。
    pub tsr_clamped_channels: u32,
    pub measured_tsr_max_abs: f32,
    // ── 判定 ──
    pub vsm_depth_pass: bool,
    pub vsm_sample_pass: bool,
    pub tsr_pass: bool,
    /// 输入 provenance(诚实边界字段化)。
    pub input_provenance: &'static str,
}

impl G75MatchResults {
    pub fn all_pass(&self) -> bool {
        self.vsm_depth_pass && self.vsm_sample_pass && self.tsr_pass
    }

    /// 单行 JSON(`--g75-residuals` 输出;步骤 95 evidence 字段源)。
    pub fn json(&self) -> String {
        format!(
            "{{\"subject\":\"uc06_g75_residuals\",\"device_name\":\"{}\",\
             \"vsm_pages\":{},\"vsm_depth_texels\":{},\"vsm_triangles\":{},\
             \"vsm_depth_bitexact_texels\":{},\"vsm_depth_covered_texels\":{},\
             \"measured_vsm_depth_max_abs\":{:.9e},\"tol_vsm_depth\":{:.9e},\
             \"vsm_samples\":{},\"vsm_sample_mismatches\":{},\
             \"vsm_shadowed_ratio_device\":{:.6},\
             \"measured_vsm_sample_max_abs\":{:.9e},\"tol_vsm_sample\":{:.9e},\
             \"tsr_in_w\":{},\"tsr_in_h\":{},\"tsr_out_w\":{},\"tsr_out_h\":{},\
             \"tsr_channels\":{},\"tsr_bitexact_channels\":{},\"tsr_clamped_channels\":{},\
             \"measured_tsr_max_abs\":{:.9e},\"tol_tsr\":{:.9e},\
             \"input_provenance\":\"{}\",\
             \"vsm_depth_pass\":{},\"vsm_sample_pass\":{},\"tsr_pass\":{},\"all_pass\":{}}}",
            self.device_name,
            self.vsm_pages,
            self.vsm_depth_texels,
            self.vsm_triangles,
            self.vsm_depth_bitexact_texels,
            self.vsm_depth_covered_texels,
            self.measured_vsm_depth_max_abs,
            tol::VSM_DEPTH,
            self.vsm_samples,
            self.vsm_sample_mismatches,
            self.vsm_shadowed_ratio_device,
            self.measured_vsm_sample_max_abs,
            tol::VSM_SAMPLE,
            self.tsr_in_w,
            self.tsr_in_h,
            self.tsr_out_w,
            self.tsr_out_h,
            self.tsr_channels,
            self.tsr_bitexact_channels,
            self.tsr_clamped_channels,
            self.measured_tsr_max_abs,
            tol::TSR,
            self.input_provenance,
            self.vsm_depth_pass,
            self.vsm_sample_pass,
            self.tsr_pass,
            self.all_pass(),
        )
    }
}

// ── 通用小工具(镜像 device_kernels.rs / device_w3.rs 同名件)────────────────────

fn storage<'a>(size: usize, data: Option<&'a [u8]>) -> ResourceDesc<'a> {
    ResourceDesc::Buffer(BufferDesc {
        size: size as u64,
        usage: BufferUsage {
            storage: true,
            ..Default::default()
        },
        data,
    })
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_u32(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn max_abs(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

fn bitexact_count(a: &[f32], b: &[f32]) -> u32 {
    a.iter()
        .zip(b)
        .filter(|(x, y)| x.to_bits() == y.to_bits())
        .count() as u32
}

/// W1 能力链门禁(本波三核只用 f32/u32 SSBO,W1 即足;无 loader → `None`)。
fn g75_gate() -> Option<render_exec::DeviceCaps> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("[uc06 G7.5] SKIP: vulkan loader 不可用(dev-env degrade)");
        return None;
    }
    let caps = render_exec::probe_device_caps().ok()?;
    match render_exec::require_wave(&caps, KernelWave::W1) {
        Ok(()) => Some(caps),
        Err(e) => {
            eprintln!("[uc06 G7.5] SKIP: W1 能力链缺失({e})");
            None
        }
    }
}

fn dispatch_compute(
    name: &'static str,
    spirv: &'static [u8],
    resources: &[ResourceDesc<'_>],
    storage_buffers: Vec<u32>,
    push_constants: Vec<u8>,
    threads: u32,
    readbacks: &[Readback],
) -> Result<Vec<Vec<u8>>, String> {
    let passes = [Pass::Compute(ComputePass {
        name,
        spirv,
        entry: None,
        dispatch: DispatchSpec::Direct([threads, 1, 1]),
        bindings: Bindings {
            storage_buffers,
            push_constants,
            ..Default::default()
        },
    })];
    let barriers: [&[(u32, render_exec::TargetState)]; 1] = [&[]];
    render_exec::execute_frame(resources, &passes, &barriers, readbacks)
}

// ── VSM ────────────────────────────────────────────────────────────────────────

/// 一页的 device 光栅参数(与 `vsm_depth_raster.rx` 的 `pages` buffer 布局逐字对应)。
struct PageParam {
    phys: u16,
    origin: [f32; 2],
    page_world: f32,
    z_range: [f32; 2],
}

/// 建 VSM 并推进到「已标记 + 已分配、待光栅」状态,返回 host 枚举序的脏且驻留页参数。
///
/// 枚举序**逐字复刻** `Vsm::shadow_depth_raster` 的「逐级 × 逐槽位下标」双层序,
/// 使 device 侧页序与 host 物理页内容一一对应。
fn vsm_pending_pages(scene: &Uc06Scene) -> (Vsm, Vec<PageParam>) {
    let basis = LightBasis::from_direction(VSM_LIGHT_DIR);
    let (mut zmin, mut zmax) = (f32::INFINITY, f32::NEG_INFINITY);
    for t in &scene.world_tris {
        for v in t.v {
            let z = basis.to_light(v)[2];
            zmin = zmin.min(z);
            zmax = zmax.max(z);
        }
    }
    let cfg = VsmConfig {
        clip: ClipmapConfig {
            levels: VSM_LEVELS,
            base_radius: VSM_BASE_RADIUS,
            depth_extent: ((zmax - zmin) * 0.55).max(1.0),
        },
        pool_pages: VSM_POOL_PAGES,
        depth_bias: VSM_DEPTH_BIAS,
    };
    let mut vsm = Vsm::new(cfg, VSM_LIGHT_DIR, CAMERA.eye);

    // 屏幕反馈标记:冻结相机 GBuffer 深度(NDC z,1.0 = 天空)驱动 page_mark。
    let mats = crate::pipeline::camera_matrices(VSM_MARK_DIM, VSM_MARK_DIM);
    let gi_cam = rurix_render::gi::probe::GiCamera::new(mats.view_proj);
    let (depth, _normals) =
        crate::shading::scene_gbuffer(scene, &gi_cam, VSM_MARK_DIM, VSM_MARK_DIM);
    vsm.page_mark(&depth, &mats.inv_view_proj);
    vsm.page_alloc();

    let views = vsm.views();
    let mut pages = Vec::new();
    for view in &views {
        let li = view.level;
        let pw = view.page_world;
        // 页表槽位数 = 128×128,行主序 —— 与 `shadow_depth_raster` 的 `for idx in 0..SLOTS` 同序。
        for idx in 0..PAGE_TABLE_SLOTS {
            let e = PageTableEntry::unpack(vsm.table(li).entries[idx]);
            if !(e.resident && e.dirty) {
                continue;
            }
            let (sx, sy) = ((idx % PAGE_DIM) as u8, (idx / PAGE_DIM) as u8);
            let wp = vsm.slot_world_page(li, sx, sy);
            pages.push(PageParam {
                phys: e.phys,
                origin: [wp[0] as f32 * pw, wp[1] as f32 * pw],
                page_world: pw,
                z_range: view.z_range,
            });
        }
    }
    (vsm, pages)
}

/// VSM 页内深度光栅 device 腿。`tamper_dz` ≠ 0 = **RED 轴**:device 侧灯空间三角形
/// 整体沿灯向平移该量(host oracle 不动)→ 深度对拍必红。
#[allow(clippy::type_complexity)]
fn run_vsm_depth(
    scene: &Uc06Scene,
    tamper_dz: f32,
) -> Result<(Vsm, Vec<PageParam>, Vec<f32>, Vec<f32>), String> {
    let (mut vsm, pages) = vsm_pending_pages(scene);
    if pages.is_empty() {
        return Err("VSM page_mark/alloc 后无脏且驻留页(判据会空转)".to_owned());
    }
    let basis = LightBasis::from_direction(VSM_LIGHT_DIR);
    let mut tris = Vec::with_capacity(scene.world_tris.len() * 9);
    for t in &scene.world_tris {
        for v in t.v {
            let l = basis.to_light(v);
            tris.extend_from_slice(&[l[0], l[1], l[2] + tamper_dz]);
        }
    }
    let mut page_params = Vec::with_capacity(pages.len() * 5);
    for p in &pages {
        page_params.extend_from_slice(&[
            p.origin[0],
            p.origin[1],
            p.page_world,
            p.z_range[0],
            p.z_range[1],
        ]);
    }
    let tri_count = scene.world_tris.len() as u32;
    let page_count = pages.len() as u32;
    let out_len = pages.len() * PAGE_TEXELS;
    let tris_b = bytes_f32(&tris);
    let pages_b = bytes_f32(&page_params);
    let resources = [
        storage(tris_b.len(), Some(&tris_b)),
        storage(pages_b.len(), Some(&pages_b)),
        storage(out_len * 4, None),
    ];
    let readbacks = [Readback::Buffer {
        res: 2,
        offset: 0,
        size: (out_len * 4) as u64,
    }];
    let out = dispatch_compute(
        "vsm_depth_raster",
        VSM_DEPTH_SPV,
        &resources,
        vec![0, 1, 2],
        bytes_u32(&[tri_count, page_count]),
        page_count * PAGE_TEXELS as u32,
        &readbacks,
    )?;
    let device = read_f32(&out[0]);

    // host oracle:同一 VSM 实例真实光栅,再按同一页序取物理页内容。
    vsm.shadow_depth_raster(&scene.world_tris);
    let mut host = Vec::with_capacity(out_len);
    for p in &pages {
        host.extend_from_slice(vsm.pool().page(p.phys));
    }
    Ok((vsm, pages, device, host))
}

/// 采样点集:冻结场景每个世界三角形的**重心**(确定性、零随机、覆盖地面/球/立方体,
/// 天然含被遮蔽与受光两臂)。
fn vsm_sample_points(scene: &Uc06Scene) -> Vec<[f32; 3]> {
    scene
        .world_tris
        .iter()
        .map(|t| {
            [
                (t.v[0][0] + t.v[1][0] + t.v[2][0]) / 3.0,
                (t.v[0][1] + t.v[1][1] + t.v[2][1]) / 3.0,
                (t.v[0][2] + t.v[1][2] + t.v[2][2]) / 3.0,
            ]
        })
        .collect()
}

/// VSM 阴影采样 device 腿(在 `run_vsm_depth` 之后调用:此时页表已清脏、池已填深度)。
fn run_vsm_sample(vsm: &Vsm, points: &[[f32; 3]]) -> Result<(Vec<f32>, Vec<f32>), String> {
    let basis = LightBasis::from_direction(VSM_LIGHT_DIR);
    let views = vsm.views();
    let mut lparams = Vec::with_capacity(views.len() * 5);
    for v in &views {
        lparams.extend_from_slice(&[
            v.page_world,
            v.window_min_pages[0] as f32,
            v.window_min_pages[1] as f32,
            v.z_range[0],
            v.z_range[1],
        ]);
    }
    let mut entries = Vec::with_capacity(views.len() * PAGE_TEXELS);
    for v in &views {
        entries.extend_from_slice(&vsm.table(v.level).entries);
    }
    let mut pool = Vec::with_capacity(VSM_POOL_PAGES as usize * PAGE_TEXELS);
    for p in 0..VSM_POOL_PAGES {
        pool.extend_from_slice(vsm.pool().page(p));
    }
    let samples: Vec<f32> = points.iter().flat_map(|p| *p).collect();

    let samples_b = bytes_f32(&samples);
    let lparams_b = bytes_f32(&lparams);
    let entries_b = bytes_u32(&entries);
    let pool_b = bytes_f32(&pool);
    let out_size = points.len() * 4;
    let resources = [
        storage(samples_b.len(), Some(&samples_b)),
        storage(lparams_b.len(), Some(&lparams_b)),
        storage(entries_b.len(), Some(&entries_b)),
        storage(pool_b.len(), Some(&pool_b)),
        storage(out_size, None),
    ];
    let readbacks = [Readback::Buffer {
        res: 4,
        offset: 0,
        size: out_size as u64,
    }];
    let mut push = bytes_u32(&[
        points.len() as u32,
        u32::from(VSM_LEVELS),
        u32::from(VSM_POOL_PAGES),
    ]);
    for v in [
        CAMERA.eye[0],
        CAMERA.eye[1],
        CAMERA.eye[2],
        basis.right[0],
        basis.right[1],
        basis.right[2],
        basis.up[0],
        basis.up[1],
        basis.up[2],
        basis.fwd[0],
        basis.fwd[1],
        basis.fwd[2],
        VSM_BASE_RADIUS,
        VSM_DEPTH_BIAS,
    ] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    let out = dispatch_compute(
        "vsm_sample",
        VSM_SAMPLE_SPV,
        &resources,
        vec![0, 1, 2, 3, 4],
        push,
        points.len() as u32,
        &readbacks,
    )?;
    let device = read_f32(&out[0]);
    let host: Vec<f32> = points.iter().map(|&p| vsm.sample_shadow(p)).collect();
    Ok((device, host))
}

// ── TSR ────────────────────────────────────────────────────────────────────────

/// TSR 输入色图:冻结场景 GBuffer(世界法线映射到 [0,1] × 深度前景权重)。
/// 天空(depth == 1.0)恒 0 ⇒ 轮廓处形成硬边,压满 Catmull-Rom 负瓣与抗振铃钳制。
fn tsr_input_color(scene: &Uc06Scene) -> (ImageF32, ImageF32) {
    let mats = crate::pipeline::camera_matrices(TSR_IN_W, TSR_IN_H);
    let gi_cam = rurix_render::gi::probe::GiCamera::new(mats.view_proj);
    let (depth, normals) = crate::shading::scene_gbuffer(scene, &gi_cam, TSR_IN_W, TSR_IN_H);
    let color = ImageF32::from_fn(TSR_IN_W, TSR_IN_H, 3, |x, y, ch| {
        let n = normals.get(x, y, ch) * 0.5 + 0.5;
        n * (1.0 - depth.get(x, y, 0))
    });
    (color, depth)
}

/// TSR 空间超分 device 腿。`tamper_jitter` ≠ 0 = **RED 轴**:device 侧 jitter 被
/// 注入偏移(host oracle 不动)→ 重采样相位错位,对拍必红。
fn run_tsr(color: &ImageF32, jitter: [f32; 2], tamper_jitter: f32) -> Result<Vec<f32>, String> {
    let color_b = bytes_f32(&color.data);
    let out_len = (TSR_OUT_W * TSR_OUT_H * 3) as usize;
    let resources = [
        storage(color_b.len(), Some(&color_b)),
        storage(out_len * 4, None),
    ];
    let readbacks = [Readback::Buffer {
        res: 1,
        offset: 0,
        size: (out_len * 4) as u64,
    }];
    let mut push = bytes_u32(&[TSR_IN_W, TSR_IN_H, TSR_OUT_W, TSR_OUT_H]);
    for v in [
        jitter[0] + tamper_jitter,
        jitter[1],
        1.0f32,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    let out = dispatch_compute(
        "tsr_resample",
        TSR_SPV,
        &resources,
        vec![0, 1],
        push,
        TSR_OUT_W * TSR_OUT_H,
        &readbacks,
    )?;
    Ok(read_f32(&out[0]))
}

/// host 侧统计:抗振铃钳制真实生效的通道数(加权和越出 4×4 邻域包络)。
/// 用于证明对拍覆盖了 Catmull-Rom 负瓣分支,判据不空转。
fn tsr_clamped_channels(color: &ImageF32, jitter: [f32; 2]) -> u32 {
    let (iw, ih) = (color.w as i32, color.h as i32);
    let (sx, sy) = (
        color.w as f32 / TSR_OUT_W as f32,
        color.h as f32 / TSR_OUT_H as f32,
    );
    let kernel_scale = |r: f32| if r > 1.0 { r * 0.75 } else { 1.0 };
    let rx = kernel_scale(TSR_OUT_W as f32 / color.w as f32);
    let ry = kernel_scale(TSR_OUT_H as f32 / color.h as f32);
    let cr = |t: f32| {
        let t = t.abs();
        if t <= 1.0 {
            1.5 * t * t * t - 2.5 * t * t + 1.0
        } else if t < 2.0 {
            -0.5 * t * t * t + 2.5 * t * t - 4.0 * t + 2.0
        } else {
            0.0
        }
    };
    let mut clamped = 0u32;
    for oy in 0..TSR_OUT_H {
        for ox in 0..TSR_OUT_W {
            let gx = (ox as f32 + 0.5) * sx - 0.5 - jitter[0];
            let gy = (oy as f32 + 0.5) * sy - 0.5 - jitter[1];
            let (bx, by) = (gx.floor() as i32, gy.floor() as i32);
            let mut acc = [0.0f32; 3];
            let mut wsum = 0.0f32;
            let mut mn = [f32::INFINITY; 3];
            let mut mx = [f32::NEG_INFINITY; 3];
            for dy in -1i32..=2 {
                for dx in -1i32..=2 {
                    let tx = (bx + dx).clamp(0, iw - 1) as u32;
                    let ty = (by + dy).clamp(0, ih - 1) as u32;
                    let w = cr((gx - (bx + dx) as f32) * rx) * cr((gy - (by + dy) as f32) * ry);
                    let p = color.pixel3(tx, ty);
                    for ch in 0..3 {
                        acc[ch] += w * p[ch];
                        mn[ch] = mn[ch].min(p[ch]);
                        mx[ch] = mx[ch].max(p[ch]);
                    }
                    wsum += w;
                }
            }
            for ch in 0..3 {
                let v = acc[ch] / wsum;
                if v < mn[ch] || v > mx[ch] {
                    clamped += 1;
                }
            }
        }
    }
    clamped
}

// ── 编排 ───────────────────────────────────────────────────────────────────────

fn run_matches(
    scene: &Uc06Scene,
    tamper_vsm_dz: f32,
    tamper_jitter: f32,
) -> Option<Result<G75MatchResults, String>> {
    let caps = g75_gate()?;
    Some(run_matches_inner(
        scene,
        &caps,
        tamper_vsm_dz,
        tamper_jitter,
    ))
}

fn run_matches_inner(
    scene: &Uc06Scene,
    caps: &render_exec::DeviceCaps,
    tamper_vsm_dz: f32,
    tamper_jitter: f32,
) -> Result<G75MatchResults, String> {
    // ① VSM 页内深度光栅。
    let (vsm, pages, vsm_dev, vsm_host) = run_vsm_depth(scene, tamper_vsm_dz)?;
    let vsm_depth_texels = vsm_dev.len() as u32;
    let measured_vsm_depth_max_abs = max_abs(&vsm_dev, &vsm_host);
    let vsm_depth_bitexact_texels = bitexact_count(&vsm_dev, &vsm_host);
    let vsm_depth_covered_texels = vsm_host.iter().filter(|&&d| d < 1.0).count() as u32;

    // ② VSM 阴影采样(消费 ① 后的真实深度池)。
    let points = vsm_sample_points(scene);
    let (smp_dev, smp_host) = run_vsm_sample(&vsm, &points)?;
    let measured_vsm_sample_max_abs = max_abs(&smp_dev, &smp_host);
    let vsm_sample_mismatches = smp_dev
        .iter()
        .zip(&smp_host)
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count() as u32;
    let shadowed = smp_dev.iter().filter(|&&v| v < 0.5).count() as f32;

    // ③ TSR 空间超分。`resample_current_frame` 只消费 color/jitter/exposure;
    // depth/mv 仅过 `UpscaleInputs::validated()` 的形状契约(depth 1ch = 真实 GBuffer
    // 深度,mv 2ch = 冻结相机静态 ⇒ 恒零,与 `G7_SCENE_FREEZE.md` §2「MV 恒零」一致)。
    let (color, depth) = tsr_input_color(scene);
    let mv = ImageF32::new(TSR_IN_W, TSR_IN_H, 2);
    let jitter = jitter_sequence(8)[0];
    let tsr_dev = run_tsr(&color, jitter, tamper_jitter)?;
    let inputs = UpscaleInputs {
        color: &color,
        depth: &depth,
        mv: &mv,
        reactive: None,
        exposure: 1.0,
        jitter,
        output_size: (TSR_OUT_W, TSR_OUT_H),
        frame_index: 0,
        reset: true,
    };
    let tsr_host = TsrUpscaler::resample_current_frame(&inputs);
    let measured_tsr_max_abs = max_abs(&tsr_dev, &tsr_host.data);
    let tsr_bitexact_channels = bitexact_count(&tsr_dev, &tsr_host.data);
    let tsr_clamped = tsr_clamped_channels(&color, jitter);

    Ok(G75MatchResults {
        device_name: caps.device_name.clone(),
        vsm_pages: pages.len() as u32,
        vsm_depth_texels,
        vsm_triangles: scene.world_tris.len() as u32,
        vsm_depth_bitexact_texels,
        vsm_depth_covered_texels,
        measured_vsm_depth_max_abs,
        vsm_samples: points.len() as u32,
        vsm_sample_mismatches,
        vsm_shadowed_ratio_device: shadowed / points.len() as f32,
        measured_vsm_sample_max_abs,
        tsr_in_w: TSR_IN_W,
        tsr_in_h: TSR_IN_H,
        tsr_out_w: TSR_OUT_W,
        tsr_out_h: TSR_OUT_H,
        tsr_channels: tsr_host.data.len() as u32,
        tsr_bitexact_channels,
        tsr_clamped_channels: tsr_clamped,
        measured_tsr_max_abs,
        // 判据 = measured ≤ 冻结容差 **且** 判据面非退化(覆盖纹素 / 抗振铃分支
        // 真实命中);`*_bitexact_*` 为留痕统计,不入判据(见 `tol` 模块的收缩说明)。
        vsm_depth_pass: measured_vsm_depth_max_abs <= tol::VSM_DEPTH
            && vsm_depth_covered_texels > 0,
        vsm_sample_pass: measured_vsm_sample_max_abs <= tol::VSM_SAMPLE
            && vsm_sample_mismatches == 0,
        tsr_pass: measured_tsr_max_abs <= tol::TSR && tsr_clamped > 0,
        input_provenance: "vsm:host-light-space-tris+page-state|tsr:frozen-scene-gbuffer+halton-jitter",
    })
}

/// 生产路径:零注入。
pub fn run_g75_residuals(scene: &Uc06Scene) -> Option<Result<G75MatchResults, String>> {
    run_matches(scene, 0.0, 0.0)
}

/// RED 轴 ①:篡改 device 侧灯空间三角形深度 → VSM 深度对拍**必红**(数据流反证)。
/// 返回 `Some(true)` = RED-OK。
pub fn red_tamper_vsm_depth(scene: &Uc06Scene) -> Option<bool> {
    match run_matches(scene, 1e-3, 0.0)? {
        Ok(r) => Some(!r.vsm_depth_pass),
        // 执行面报错也算「未通过对拍」,但须区分:执行失败不是数据流反证。
        Err(_) => Some(false),
    }
}

/// RED 轴 ②:篡改 device 侧 jitter → TSR 重采样相位错位,对拍**必红**。
pub fn red_tamper_tsr_jitter(scene: &Uc06Scene) -> Option<bool> {
    match run_matches(scene, 0.0, 0.25)? {
        Ok(r) => Some(!r.tsr_pass),
        Err(_) => Some(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_g75_vsm_and_tsr_match_host() {
        let scene = crate::scene::build_scene();
        let Some(res) = run_g75_residuals(&scene) else {
            return; // dev-env degrade
        };
        let r = res.expect("G7.5 余项 device 执行");
        assert!(r.all_pass(), "G7.5 余项对拍未全过: {}", r.json());
    }
}
