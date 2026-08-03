//! G7.4 W3c:`gi_probe` / `rtao` / `hard_shadow` 三效果核**共用同一真实 TLAS** 的
//! device 执行与 host oracle 对拍(RD-038「屏幕探针 GI」/「RTAO 硬阴影」;
//! RFC-0018 章 D;验收门 G-G7-6;CI 步骤 94)。
//!
//! ## 执行面
//! 三 kernel 经 `rurix_rt::vk::run_ray_query_effects` 在**一次** `VkAsManager`
//! 建面(3 BLAS × 3 实例,冻结场景)+ 一条 command buffer + 单次提交中依次 dispatch;
//! 每个 dispatch 的 set 0 / binding 0 写入**同一个** TLAS 句柄,identity 经
//! `RayQueryEffectsOutput::dispatch_tlas` 回传供机验(G-G7-6「三个内核共用真实
//! TLAS」的机器判据)。
//!
//! ## 场景冻结面(`milestones/g7/G7_SCENE_FREEZE.md`)
//! - 几何 = `scene::build_scene()` 的 3 网格逐实例世界空间三角形(plane/sphere/cube),
//!   与 host `Uc06Scene::{tlas, blases}` **同一份**数据;
//! - 实例 = 3,transform 恒 identity(三角形已世界空间),mask = 0xFE/0xFF/0xFF
//!   (冻结值;ray mask 恒 0xFF → 三实例全可见,与三个 host oracle 的 0xFF 口径一致);
//! - 实例 flags = `TRIANGLE_FACING_CULL_DISABLE`(host `rt::bvh` 三角形相交双面,
//!   device 须同口径);
//! - 光照常量 `SUN_DIR`/`SUN_COLOR`/`SKY_COLOR` 与相机 `CAMERA` 取冻结值。
//!
//! ## 探针集(确定性,零随机)
//! 自冻结相机以 [`PROBE_W`]×[`PROBE_H`] 针孔光线网格取样:
//! - `gi_probe` 消费**全部**光线(命中/未命中两臂皆覆盖 → miss 轴在 device 真跑);
//! - `rtao`/`hard_shadow` 消费其中**命中**光线压实后的 GBuffer(位置 + 世界法线),
//!   保证输入全有效 —— oracle 的无效像素臂(NaN/±inf 位置、零长法线/光方向)归
//!   host 单测覆盖,本波不在 device 表达(**诚实边界**,evidence 明记)。
//!
//! ## host oracle 纪律(RFC-0018 §D2)
//! oracle 仅作对拍参照,**不参与成功路径**:device 判据完全由 device readback 与
//! oracle 的差值构成,无任何 host 结果回填 device。oracle 数值语义 0-byte
//! (`rt::ref_tracer::cosine_sample_hemisphere` 仅作**可见性**加性升 `pub`,函数体
//! 与运算序逐字不变)。

use rurix_render::gi::tracer::{GiScene, RadianceTracer, RayTracedRadiance};
use rurix_render::material::closure::unpack;
use rurix_render::rt::bvh::{Ray, Vec3};
use rurix_render::rt::ref_tracer::{
    Pcg32, RAY_EPS, cosine_sample_hemisphere, hard_shadow_reference, rtao_reference,
};
use rurix_rt::render_exec::{self, KernelWave};
use rurix_rt::vk::{
    RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQueryRedProbe,
    RayQuerySceneDesc, entry_point_name, run_ray_query_effects_probed,
};

use crate::scene::{CAMERA, SKY_COLOR, SUN_COLOR, SUN_DIR, Uc06Scene};

const GI_PROBE_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gi_probe.spv"));
const RTAO_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rtao.spv"));
const HARD_SHADOW_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hard_shadow.spv"));

/// 探针光线网格宽(冻结;aspect 16:9 与冻结 1080p 一致)。
pub const PROBE_W: u32 = 64;
/// 探针光线网格高(冻结)。
pub const PROBE_H: u32 = 36;
/// RTAO 每像素采样数(冻结)。
pub const RTAO_SPP: u32 = 8;
/// RTAO 遮蔽半径(冻结;场景尺度 ~3 单位,取 2.0 使地面点能被球/立方体遮蔽 —— 遮蔽
/// 显著而非饱和,避免「几乎全无遮蔽」的弱判据)。
pub const RTAO_RADIUS: f32 = 2.0;
/// RTAO / 采样种子 = uc06 冻结默认种子(`RenderConfig::default().seed`)。
pub const W3_SEED: u64 = 0x5255_5258_5543_0006;
/// host `+∞` 的有限替身(冻结场景直径 ≪ 本值,几何等价;Vulkan RayTmax 取有限值)。
pub const T_MAX_FINITE: f32 = 1.0e30;

/// **measured 后冻结**的对拍容差(G-G7-6 / RFC-0018 §D3:索引与 hit/miss 类零容差;
/// 浮点类先 measured 后冻结,阈值只来自真实 GPU 输出)。
///
/// 各阈值 = 本机(RTX 4070 Ti / driver 620.02 / SDK 1.3.296)实测最大差值向上取到
/// 相邻量级安全余量;实测值随 evidence 落盘(`measured_*` 字段),阈值即本常量组。
pub mod tol {
    /// `committed_t`(几何交点距离)。measured = 1.43e-6(2026-08-03,2304 光线 /
    /// 1706 命中);冻结 1e-5(≈7× 余量)。差源 = GPU 硬件三角形求交与 host
    /// Möller–Trumbore 的 f32 舍入路径不同,非语义差。
    pub const T: f32 = 1e-5;
    /// `committed_barycentric` 分量。measured = 1.26e-5;冻结 1e-4(≈8× 余量,
    /// 上取相邻量级)。重心坐标为求交内部量,精度弱于 t 属预期。
    pub const BARY: f32 = 1e-4;
    /// GI 命中点辐射度(RGB 逐分量)。measured = 1.19e-7;冻结 1e-5。
    /// 该量同时是**阴影可见性第二条 ray query** 与 host `any_hit` 全量一致的证据
    /// (若二者有一处分歧,该差值会跃到 O(1))。
    pub const RADIANCE: f32 = 1e-5;
    /// RTAO AO(离散值 k/spp)。measured = 0.0(**逐位一致**:遮蔽计数整数级相同);
    /// 冻结 1e-6(保留最小余量,不放宽到肉眼级)。
    pub const AO: f32 = 1e-6;
    /// 硬阴影可见性(0/1 二值)。measured = 0.0;冻结 **0.0**(零容差)。
    pub const VISIBILITY: f32 = 0.0;
}

/// 三核对拍结果(逐核对拍量 / measured 差值 / 判定;`--w3-effects` JSON 与
/// 步骤 94 evidence 的字段源)。
#[derive(Debug, Clone)]
pub struct W3MatchResults {
    /// 设备名(capability snapshot 同源)。
    pub device_name: String,
    /// TLAS 句柄 identity(单一真实 TLAS)。
    pub tlas_identity: u64,
    /// 逐 dispatch 绑定的 TLAS 句柄(须全等于 `tlas_identity`)。
    pub dispatch_tlas: Vec<u64>,
    /// 三 dispatch 是否共用同一 TLAS(机验判据)。
    pub shared_tlas: bool,
    /// BLAS 数 / 实例数 / 三角形总数(冻结场景规模)。
    pub blas_count: u32,
    pub instance_count: u32,
    pub triangle_count: u32,
    /// 探针光线数 / 命中光线数(= RTAO/硬阴影像素数)。
    pub probe_rays: u32,
    pub gbuffer_pixels: u32,
    /// ── 几何语义对拍(gi_probe 的 committed 五查询)──
    pub geom_hit_mismatches: u32,
    pub geom_instance_mismatches: u32,
    pub geom_primitive_mismatches: u32,
    pub geom_geometry_nonzero: u32,
    pub measured_t_max_abs: f32,
    pub measured_bary_max_abs: f32,
    /// ── 效果输出对拍 ──
    pub measured_radiance_max_abs: f32,
    pub measured_ao_max_abs: f32,
    pub measured_visibility_max_abs: f32,
    /// 效果统计(非判据,场景表达力留痕:AO 均值、**被遮蔽像素数**、阴影覆盖率、
    /// GI 非零比 —— 用于证明三核输出非退化常量,判据不空转)。
    pub ao_mean_device: f32,
    pub ao_occluded_pixels: u32,
    pub shadowed_ratio_device: f32,
    pub radiance_nonzero_ratio_device: f32,
    /// RTAO 采样方向 provenance(host 同源输入,非 host 回填结果)。
    pub rtao_dirs_provenance: &'static str,
    /// 逐核 PASS。
    pub gi_probe_pass: bool,
    pub rtao_pass: bool,
    pub hard_shadow_pass: bool,
}

/// 冻结相机针孔光线网格(确定性;右手系,与 `look_at_rh` 同基构造)。
fn probe_rays() -> Vec<([f32; 3], [f32; 3])> {
    let eye = Vec3::from_array(CAMERA.eye);
    let fwd = (Vec3::from_array(CAMERA.center) - eye).normalize();
    let right = fwd.cross(Vec3::from_array(CAMERA.up)).normalize();
    let up = right.cross(fwd);
    let tan_half = (CAMERA.fov_y * 0.5).tan();
    let aspect = PROBE_W as f32 / PROBE_H as f32;
    let mut out = Vec::with_capacity((PROBE_W * PROBE_H) as usize);
    for y in 0..PROBE_H {
        for x in 0..PROBE_W {
            let u = (2.0 * (x as f32 + 0.5) / PROBE_W as f32 - 1.0) * aspect * tan_half;
            let v = (1.0 - 2.0 * (y as f32 + 0.5) / PROBE_H as f32) * tan_half;
            let dir = (fwd + right * u + up * v).normalize();
            out.push((eye.to_array(), dir.to_array()));
        }
    }
    out
}

/// 冻结场景的 GI 场景视图:**复用 `scene.tlas`/`scene.blases` 本体**(不另建几何),
/// 仅补逐实例 albedo 与光照常量 —— 与三核 device TLAS 同一份几何的 host 侧对偶。
fn gi_scene_view(scene: &Uc06Scene) -> GiScene {
    GiScene {
        blases: scene.blases.clone(),
        tlas: scene.tlas.clone(),
        albedos: scene
            .materials
            .closures()
            .iter()
            .map(|c| unpack(c).albedo)
            .collect(),
        sun_dir: Vec3::from_array(SUN_DIR).normalize(),
        sun_color: SUN_COLOR,
        sky_color: SKY_COLOR,
    }
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

fn read_u32(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn max_abs(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

/// W3 能力链门禁(缺一确定性拒绝;无 loader → `None` = dev-env degrade)。
fn w3_gate() -> Option<render_exec::DeviceCaps> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("[uc06 W3] SKIP: vulkan loader 不可用(dev-env degrade)");
        return None;
    }
    let caps = render_exec::probe_device_caps().ok()?;
    match render_exec::require_wave(&caps, KernelWave::W3) {
        Ok(()) => Some(caps),
        Err(e) => {
            eprintln!("[uc06 W3] SKIP: W3 能力链缺失({e})");
            None
        }
    }
}

fn spv_words(bytes: &'static [u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// 三核共用同一真实 TLAS 的 device 执行 + host oracle 对拍。
///
/// `tamper_dx` = **RED 轴**注入:非 0 时把 device 侧 BLAS 顶点与三角形表整体沿 x
/// 平移该量(host oracle 与 host TLAS **不动**)→ 几何数据流被篡改,对拍必红。
/// 生产调用恒 `0.0`。
///
/// `probe` = 执行面 RED 注入(过期 TLAS / 错误 barrier),生产调用恒
/// [`RayQueryRedProbe::None`]。
///
/// `None` = dev-env degrade(无 loader / W3 能力链缺失),不充绿。
pub fn run_w3_matches(
    scene: &Uc06Scene,
    tamper_dx: f32,
    probe: RayQueryRedProbe,
) -> Option<Result<W3MatchResults, String>> {
    let caps = w3_gate()?;

    // ── 冻结场景 → 逐实例世界空间三角形(host/device 同一份数据)──
    let per_instance: Vec<Vec<f32>> = scene
        .meshes
        .iter()
        .map(|m| {
            m.world_triangles
                .iter()
                .flat_map(|tri| tri.iter().flat_map(|v| v.iter().copied()))
                .collect()
        })
        .collect();
    // device 侧几何(RED 注入时整体平移 x;生产恒等于 host 侧)。
    let device_tris: Vec<Vec<f32>> = per_instance
        .iter()
        .map(|v| {
            v.iter()
                .enumerate()
                .map(|(k, &c)| if k % 3 == 0 { c + tamper_dx } else { c })
                .collect()
        })
        .collect();
    let blas_refs: Vec<&[f32]> = device_tris.iter().map(|v| v.as_slice()).collect();
    // 实例 mask 取冻结场景值(plane 0xFE / sphere,cube 0xFF);ray mask 恒 0xFF。
    let masks: [u8; 3] = [0xFE, 0xFF, 0xFF];
    let instances: Vec<RayQueryInstanceDesc> = (0..scene.meshes.len())
        .map(|i| RayQueryInstanceDesc {
            blas: i as u32,
            custom_index: i as u32,
            mask: masks[i.min(2)],
        })
        .collect();
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    // 全局三角形表(按实例序拼接)+ 逐实例基址(gi_probe 法线重建输入)。
    let mut tri_base: Vec<u32> = Vec::with_capacity(device_tris.len());
    let mut tris_flat: Vec<f32> = Vec::new();
    for v in &device_tris {
        tri_base.push((tris_flat.len() / 9) as u32);
        tris_flat.extend_from_slice(v);
    }
    let triangle_count = (tris_flat.len() / 9) as u32;

    // ── host oracle 侧:GI 场景视图(复用 scene.tlas/blases 本体)──
    let gi = gi_scene_view(scene);
    let tracer = RayTracedRadiance::new(gi.clone());
    let albedo_flat: Vec<f32> = gi.albedos.iter().flat_map(|a| a.iter().copied()).collect();

    // ── 探针光线 + host 几何/辐射度参照 ──
    let rays = probe_rays();
    let mut rays_flat: Vec<f32> = Vec::with_capacity(rays.len() * 6);
    let mut host_hit: Vec<bool> = Vec::with_capacity(rays.len());
    let mut host_t: Vec<f32> = Vec::with_capacity(rays.len());
    let mut host_bary: Vec<f32> = Vec::with_capacity(rays.len() * 2);
    let mut host_inst: Vec<u32> = Vec::with_capacity(rays.len());
    let mut host_prim: Vec<u32> = Vec::with_capacity(rays.len());
    let mut host_radiance: Vec<f32> = Vec::with_capacity(rays.len() * 3);
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    for (origin, dir) in &rays {
        rays_flat.extend_from_slice(origin);
        rays_flat.extend_from_slice(dir);
        let o = Vec3::from_array(*origin);
        let d = Vec3::from_array(*dir);
        let hit = scene
            .tlas
            .intersect(&scene.blases, &Ray { origin: o, dir: d });
        match hit {
            Some(h) => {
                host_hit.push(true);
                host_t.push(h.t);
                host_bary.extend_from_slice(&h.bary);
                host_inst.push(h.instance);
                host_prim.push(h.tri);
                // GBuffer(命中光线压实;位置/世界法线全有效)。
                positions.push((o + d * h.t).to_array());
                normals.push(h.normal);
            }
            None => {
                host_hit.push(false);
                host_t.push(-1.0);
                host_bary.extend_from_slice(&[0.0, 0.0]);
                host_inst.push(u32::MAX);
                host_prim.push(u32::MAX);
            }
        }
        host_radiance.extend_from_slice(&tracer.trace(o, d));
    }
    let ray_count = rays.len() as u32;
    let pixel_count = positions.len() as u32;
    if pixel_count == 0 {
        return Some(Err("冻结相机探针网格零命中(场景/相机冻结面异常)".into()));
    }

    // ── RTAO 采样方向:与 oracle **同一次生成**(同一 Pcg32 实例、同 seed、同消费序、
    //    同 cosine_sample_hemisphere 函数实例);host 同源输入,非 host 回填结果 ──
    let mut rng = Pcg32::new(W3_SEED);
    let mut dirs_flat: Vec<f32> = Vec::with_capacity(positions.len() * RTAO_SPP as usize * 3);
    for n in &normals {
        let nn = Vec3::from_array(*n).normalize();
        for _ in 0..RTAO_SPP {
            let r1 = rng.next_f32();
            let r2 = rng.next_f32();
            dirs_flat.extend_from_slice(&cosine_sample_hemisphere(nn, r1, r2).to_array());
        }
    }
    // oracle(独立 Pcg32,同 seed 同序 → 与上方逐条方向逐位一致)。
    let host_ao = rtao_reference(
        &positions,
        &normals,
        &scene.tlas,
        &scene.blases,
        RTAO_SPP,
        RTAO_RADIUS,
        W3_SEED,
    );
    // 硬阴影:光方向 = 指向光源 = −SUN_DIR(SUN_DIR 为光线传播方向)。
    let light_dir = [-SUN_DIR[0], -SUN_DIR[1], -SUN_DIR[2]];
    let host_vis = hard_shadow_reference(&positions, light_dir, &scene.tlas, &scene.blases);

    // ── device 侧输入/输出 buffer 与 push constants ──
    let pos_flat: Vec<f32> = positions.iter().flat_map(|p| p.iter().copied()).collect();
    let nrm_flat: Vec<f32> = normals.iter().flat_map(|p| p.iter().copied()).collect();
    let rays_b = bytes_f32(&rays_flat);
    let tris_b = bytes_f32(&tris_flat);
    let tri_base_b = bytes_u32(&tri_base);
    let albedo_b = bytes_f32(&albedo_flat);
    let pos_b = bytes_f32(&pos_flat);
    let nrm_b = bytes_f32(&nrm_flat);
    let dirs_b = bytes_f32(&dirs_flat);

    let sun_n = Vec3::from_array(SUN_DIR).normalize().to_array();
    let inv_pi = 1.0f32 / std::f32::consts::PI;
    let mut gi_pc = bytes_u32(&[ray_count]);
    gi_pc.extend_from_slice(&bytes_f32(&[
        sun_n[0],
        sun_n[1],
        sun_n[2],
        SUN_COLOR[0],
        SUN_COLOR[1],
        SUN_COLOR[2],
        SKY_COLOR[0],
        SKY_COLOR[1],
        SKY_COLOR[2],
        inv_pi,
        RAY_EPS,
        T_MAX_FINITE,
    ]));
    let mut rtao_pc = bytes_u32(&[pixel_count, RTAO_SPP]);
    rtao_pc.extend_from_slice(&bytes_f32(&[RTAO_RADIUS, RAY_EPS]));
    let mut hs_pc = bytes_u32(&[pixel_count]);
    hs_pc.extend_from_slice(&bytes_f32(&[
        light_dir[0],
        light_dir[1],
        light_dir[2],
        RAY_EPS,
        T_MAX_FINITE,
    ]));

    let gi_spv = spv_words(GI_PROBE_SPV);
    let rtao_spv = spv_words(RTAO_SPV);
    let hs_spv = spv_words(HARD_SHADOW_SPV);
    let gi_entry = entry_point_name(&gi_spv).ok_or("gi_probe.spv 无 OpEntryPoint");
    let rtao_entry = entry_point_name(&rtao_spv).ok_or("rtao.spv 无 OpEntryPoint");
    let hs_entry = entry_point_name(&hs_spv).ok_or("hard_shadow.spv 无 OpEntryPoint");
    let (gi_entry, rtao_entry, hs_entry) = match (gi_entry, rtao_entry, hs_entry) {
        (Ok(a), Ok(b), Ok(c)) => (a, b, c),
        _ => {
            return Some(Err(
                "W3 kernel SPIR-V 缺 OpEntryPoint(build.rs 降级?)".into()
            ));
        }
    };

    let gi_bufs = [
        RayQueryBufferDesc::Input(&rays_b),
        RayQueryBufferDesc::Input(&tris_b),
        RayQueryBufferDesc::Input(&tri_base_b),
        RayQueryBufferDesc::Input(&albedo_b),
        RayQueryBufferDesc::Output(ray_count as usize * 3 * 4),
        RayQueryBufferDesc::Output(ray_count as usize * 4 * 4),
        RayQueryBufferDesc::Output(ray_count as usize * 3 * 4),
    ];
    let rtao_bufs = [
        RayQueryBufferDesc::Input(&pos_b),
        RayQueryBufferDesc::Input(&nrm_b),
        RayQueryBufferDesc::Input(&dirs_b),
        RayQueryBufferDesc::Output(pixel_count as usize * 4),
    ];
    let hs_bufs = [
        RayQueryBufferDesc::Input(&pos_b),
        RayQueryBufferDesc::Output(pixel_count as usize * 4),
    ];
    let dispatches = [
        RayQueryDispatchDesc {
            name: "gi_probe",
            spv: &gi_spv,
            entry: &gi_entry,
            buffers: &gi_bufs,
            push_constants: &gi_pc,
            groups: [ray_count, 1, 1],
        },
        RayQueryDispatchDesc {
            name: "rtao",
            spv: &rtao_spv,
            entry: &rtao_entry,
            buffers: &rtao_bufs,
            push_constants: &rtao_pc,
            groups: [pixel_count, 1, 1],
        },
        RayQueryDispatchDesc {
            name: "hard_shadow",
            spv: &hs_spv,
            entry: &hs_entry,
            buffers: &hs_bufs,
            push_constants: &hs_pc,
            groups: [pixel_count, 1, 1],
        },
    ];

    let out = match run_ray_query_effects_probed(&scene_desc, &dispatches, probe) {
        Ok(o) => o,
        Err(e) => return Some(Err(e)),
    };

    // ── 回读与对拍 ──
    let dev_radiance = read_f32(&out.readbacks[0][0]);
    let dev_geom = read_f32(&out.readbacks[0][1]);
    let dev_idx = read_u32(&out.readbacks[0][2]);
    let dev_ao = read_f32(&out.readbacks[1][0]);
    let dev_vis = read_f32(&out.readbacks[2][0]);

    let mut geom_hit_mismatches = 0u32;
    let mut geom_instance_mismatches = 0u32;
    let mut geom_primitive_mismatches = 0u32;
    let mut geom_geometry_nonzero = 0u32;
    let mut measured_t_max_abs = 0.0f32;
    let mut measured_bary_max_abs = 0.0f32;
    for i in 0..ray_count as usize {
        let dev_is_hit = dev_geom[i * 4] != 0.0;
        if dev_is_hit != host_hit[i] {
            geom_hit_mismatches += 1;
            continue;
        }
        if !dev_is_hit {
            continue;
        }
        if dev_idx[i * 3] != host_inst[i] {
            geom_instance_mismatches += 1;
        }
        if dev_idx[i * 3 + 1] != host_prim[i] {
            geom_primitive_mismatches += 1;
        }
        // 单几何 BLAS ⇒ geometryIndex 恒 0(零容差)。
        if dev_idx[i * 3 + 2] != 0 {
            geom_geometry_nonzero += 1;
        }
        measured_t_max_abs = measured_t_max_abs.max((dev_geom[i * 4 + 1] - host_t[i]).abs());
        measured_bary_max_abs = measured_bary_max_abs
            .max((dev_geom[i * 4 + 2] - host_bary[i * 2]).abs())
            .max((dev_geom[i * 4 + 3] - host_bary[i * 2 + 1]).abs());
    }
    let measured_radiance_max_abs = max_abs(&dev_radiance, &host_radiance);
    let measured_ao_max_abs = max_abs(&dev_ao, &host_ao);
    let measured_visibility_max_abs = max_abs(&dev_vis, &host_vis);

    let ao_mean_device = dev_ao.iter().sum::<f32>() / dev_ao.len() as f32;
    let ao_occluded_pixels = dev_ao.iter().filter(|&&v| v < 1.0).count() as u32;
    let shadowed_ratio_device =
        dev_vis.iter().filter(|&&v| v == 0.0).count() as f32 / dev_vis.len() as f32;
    let radiance_nonzero_ratio_device =
        dev_radiance.iter().filter(|&&v| v != 0.0).count() as f32 / dev_radiance.len() as f32;

    let shared_tlas = out.tlas_identity != 0
        && out.dispatch_tlas.len() == 3
        && out.dispatch_tlas.iter().all(|&h| h == out.tlas_identity);

    let geom_ok = geom_hit_mismatches == 0
        && geom_instance_mismatches == 0
        && geom_primitive_mismatches == 0
        && geom_geometry_nonzero == 0
        && measured_t_max_abs <= tol::T
        && measured_bary_max_abs <= tol::BARY;
    let gi_probe_pass = shared_tlas && geom_ok && measured_radiance_max_abs <= tol::RADIANCE;
    let rtao_pass = shared_tlas && measured_ao_max_abs <= tol::AO;
    let hard_shadow_pass = shared_tlas && measured_visibility_max_abs <= tol::VISIBILITY;

    Some(Ok(W3MatchResults {
        device_name: caps.device_name.clone(),
        tlas_identity: out.tlas_identity,
        dispatch_tlas: out.dispatch_tlas,
        shared_tlas,
        blas_count: device_tris.len() as u32,
        instance_count: instances.len() as u32,
        triangle_count,
        probe_rays: ray_count,
        gbuffer_pixels: pixel_count,
        geom_hit_mismatches,
        geom_instance_mismatches,
        geom_primitive_mismatches,
        geom_geometry_nonzero,
        measured_t_max_abs,
        measured_bary_max_abs,
        measured_radiance_max_abs,
        measured_ao_max_abs,
        measured_visibility_max_abs,
        ao_mean_device,
        ao_occluded_pixels,
        shadowed_ratio_device,
        radiance_nonzero_ratio_device,
        rtao_dirs_provenance: "host-same-source-input(Pcg32 seed=W3_SEED + rt::ref_tracer::cosine_sample_hemisphere;\
             与 rtao_reference 同消费序;device 真做遍历与遮蔽判定)",
        gi_probe_pass,
        rtao_pass,
        hard_shadow_pass,
    }))
}

impl W3MatchResults {
    /// 三核全绿(共用同一 TLAS + 几何语义 + 效果输出全部在冻结容差内)。
    pub fn all_pass(&self) -> bool {
        self.shared_tlas && self.gi_probe_pass && self.rtao_pass && self.hard_shadow_pass
    }

    /// 单行 JSON(`--w3-effects` 输出;步骤 94 evidence 字段源)。
    pub fn json(&self) -> String {
        let tlas: Vec<String> = self.dispatch_tlas.iter().map(|h| h.to_string()).collect();
        format!(
            "{{\"subject\":\"uc06_w3_effects\",\"device_name\":\"{}\",\
             \"tlas_identity\":\"{}\",\"dispatch_tlas\":[{}],\"shared_tlas\":{},\
             \"blas_count\":{},\"instance_count\":{},\"triangle_count\":{},\
             \"probe_rays\":{},\"gbuffer_pixels\":{},\
             \"geom_hit_mismatches\":{},\"geom_instance_mismatches\":{},\
             \"geom_primitive_mismatches\":{},\"geom_geometry_nonzero\":{},\
             \"measured_t_max_abs\":{:.9e},\"measured_bary_max_abs\":{:.9e},\
             \"measured_radiance_max_abs\":{:.9e},\"measured_ao_max_abs\":{:.9e},\
             \"measured_visibility_max_abs\":{:.9e},\
             \"tol_t\":{:.9e},\"tol_bary\":{:.9e},\"tol_radiance\":{:.9e},\
             \"tol_ao\":{:.9e},\"tol_visibility\":{:.9e},\
             \"ao_mean_device\":{:.6},\"ao_occluded_pixels\":{},\
             \"shadowed_ratio_device\":{:.6},\
             \"radiance_nonzero_ratio_device\":{:.6},\
             \"rtao_dirs_provenance\":\"{}\",\
             \"gi_probe_pass\":{},\"rtao_pass\":{},\"hard_shadow_pass\":{},\"all_pass\":{}}}",
            self.device_name,
            self.tlas_identity,
            tlas.iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(","),
            self.shared_tlas,
            self.blas_count,
            self.instance_count,
            self.triangle_count,
            self.probe_rays,
            self.gbuffer_pixels,
            self.geom_hit_mismatches,
            self.geom_instance_mismatches,
            self.geom_primitive_mismatches,
            self.geom_geometry_nonzero,
            self.measured_t_max_abs,
            self.measured_bary_max_abs,
            self.measured_radiance_max_abs,
            self.measured_ao_max_abs,
            self.measured_visibility_max_abs,
            tol::T,
            tol::BARY,
            tol::RADIANCE,
            tol::AO,
            tol::VISIBILITY,
            self.ao_mean_device,
            self.ao_occluded_pixels,
            self.shadowed_ratio_device,
            self.radiance_nonzero_ratio_device,
            self.rtao_dirs_provenance,
            self.gi_probe_pass,
            self.rtao_pass,
            self.hard_shadow_pass,
            self.all_pass(),
        )
    }
}

/// 生产路径:三核共用同一真实 TLAS,零注入。
pub fn run_w3_effects(scene: &Uc06Scene) -> Option<Result<W3MatchResults, String>> {
    run_w3_matches(scene, 0.0, RayQueryRedProbe::None)
}

/// RED 轴 ①(**数据流反证**):篡改 device 侧场景顶点(host oracle 与 host TLAS 不动)
/// → 对拍必红。返回 `Some(true)` = RED 生效(对拍确实失败)。
pub fn red_tamper_geometry(scene: &Uc06Scene) -> Option<bool> {
    // 0.05 单位 ≫ 冻结容差,且 ≪ 场景尺度(仍在同一实例包围盒量级,不退化为全 miss)。
    match run_w3_matches(scene, 0.05, RayQueryRedProbe::None)? {
        Ok(r) => Some(!r.all_pass()),
        // 执行层报错亦算 RED 生效(篡改不可能被静默接受)。
        Err(_) => Some(true),
    }
}

/// RED 轴 ②(**注入式**):过期 TLAS → fail-closed 确定性 `Err`。
pub fn red_stale_tlas(scene: &Uc06Scene) -> Option<bool> {
    match run_w3_matches(scene, 0.0, RayQueryRedProbe::StaleTlas)? {
        Ok(_) => Some(false),
        Err(e) => Some(e.contains("过期") || e.contains("已销毁")),
    }
}

/// RED 轴 ③(**注入式**):错误 barrier → validation ERROR → fail-closed `Err`
/// (需 `RURIX_VK_VALIDATION=1`;未置则返回 `None` = 本轴未跑)。
pub fn red_wrong_barrier(scene: &Uc06Scene) -> Option<bool> {
    if std::env::var("RURIX_VK_VALIDATION").as_deref() != Ok("1") {
        return None;
    }
    match run_w3_matches(scene, 0.0, RayQueryRedProbe::WrongBarrier)? {
        Ok(_) => Some(false),
        Err(_) => Some(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 三核共用同一真实 TLAS device 真跑 + host oracle 对拍全绿(G-G7-6)。
    #[test]
    fn device_w3_effects_match_host_oracles() {
        let scene = crate::scene::build_scene();
        let Some(res) = run_w3_effects(&scene) else {
            return; // dev-env degrade(无 loader / W3 能力链缺失)
        };
        let r = res.expect("W3 三核 device 执行");
        assert!(
            r.shared_tlas,
            "三 dispatch 须共用同一 TLAS: identity={} per-dispatch={:?}",
            r.tlas_identity, r.dispatch_tlas
        );
        assert_eq!(r.geom_hit_mismatches, 0, "hit/miss 须零容差一致");
        assert_eq!(r.geom_instance_mismatches, 0, "instance index 须零容差一致");
        assert_eq!(
            r.geom_primitive_mismatches, 0,
            "primitive index 须零容差一致"
        );
        assert_eq!(
            r.geom_geometry_nonzero, 0,
            "单几何 BLAS 的 geometryIndex 恒 0"
        );
        assert!(
            r.measured_t_max_abs <= tol::T,
            "committed_t 差 {} > {}",
            r.measured_t_max_abs,
            tol::T
        );
        assert!(
            r.measured_bary_max_abs <= tol::BARY,
            "barycentric 差 {} > {}",
            r.measured_bary_max_abs,
            tol::BARY
        );
        assert!(
            r.measured_radiance_max_abs <= tol::RADIANCE,
            "GI 辐射度差 {} > {}",
            r.measured_radiance_max_abs,
            tol::RADIANCE
        );
        assert!(
            r.measured_ao_max_abs <= tol::AO,
            "RTAO AO 差 {} > {}",
            r.measured_ao_max_abs,
            tol::AO
        );
        assert!(
            r.measured_visibility_max_abs <= tol::VISIBILITY,
            "硬阴影可见性差 {} > {}",
            r.measured_visibility_max_abs,
            tol::VISIBILITY
        );
        assert!(r.all_pass());
    }

    /// RED:篡改 device 侧几何 → 对拍必红(数据流反证)。
    #[test]
    fn device_w3_tampered_geometry_is_red() {
        let scene = crate::scene::build_scene();
        let Some(red) = red_tamper_geometry(&scene) else {
            return;
        };
        assert!(red, "篡改 device 顶点后对拍仍通过 = 数据流未真实生效");
    }

    /// RED:过期 TLAS → fail-closed 确定性 Err。
    #[test]
    fn device_w3_stale_tlas_is_fail_closed() {
        let scene = crate::scene::build_scene();
        let Some(red) = red_stale_tlas(&scene) else {
            return;
        };
        assert!(red, "过期 TLAS 未被 fail-closed 拒绝");
    }
}
