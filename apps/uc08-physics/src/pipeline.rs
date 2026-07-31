//! 管线状态与逐帧执行(G6.3 uc08;RFC-0017 §4.B 合流 + RFC-0016 §1 管线图)——
//! host 全管线驱动,执行序 = 图声明线性序(照 uc06 阶段序,新增 `physics` /
//! `bridge_sync` / `mv` 阶段埋点):
//! 流送 tick(驻留沿 → 批插 body;剧本卸载 → 凭 RemovalReceipt 放页)→ 物理步
//! `world.step(dt_fixed)`(计时)→ `PhysicsBridge::sync_frame`(每帧新 SyncBudget;
//! 只写 active 动态/运动体,帧末内部 flush_dirty)→ TLAS 增量标脏 +
//! `rebuild_if_dirty`(刚体 BLAS 一律 `DynamicPolicy::Static` 零 refit)→ 两级剔除
//! → VisBuffer SW 光栅 → classify/resolve → GBuffer → MV(静态相机分量恒零 +
//! 动态体重投影覆写:`object = cur⁻¹·world → prev_world = prev·object`,
//! mv = prev_uv − cur_uv,与 temporal/common.rs 约定一致)→ VSM → GI →
//! RTAO+硬阴影(时域滤波)→ 单层材质延迟着色 → TAA(吃该 MV)→ TSR。
//!
//! 变换单向 physics → GpuScene(§4.B 冻结:渲染不回写物理)。

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use rurix_geom_build::Mat4 as GeomMat4;
use rurix_geom_build::cull_ref::{CullView, cull_clusters};
use rurix_physics::{
    BodyId, BodyKind, PhysicsBridge, PhysicsWorld, StreamingBridge, SyncBudget, WorldDesc,
    compose_transform_3x4,
};
use rurix_render::geometry::cull::{CullCamera, compact_draw_args};
use rurix_render::geometry::gpu_scene::GpuScene;
use rurix_render::geometry::material_pass::{MATERIAL_INVALID, classify, resolve};
use rurix_render::geometry::visbuffer::{RasterScene, VisBufferCpu, raster_clusters};
use rurix_render::gi::pipeline::{GiParams, render_gi};
use rurix_render::gi::probe::GiCamera;
use rurix_render::gi::temporal::GiHistory;
use rurix_render::gi::tracer::{GiMeshInstance, GiScene};
use rurix_render::graph::types::StreamingBudget;
use rurix_render::rt::as_manager::{BlasCache, BlasId, DynamicPolicy, TlasBuilder, TlasInstance};
use rurix_render::rt::bvh::Tlas;
use rurix_render::rt::denoise::{TemporalFilterParams, temporal_filter_effect};
use rurix_render::rt::effects::{EffectInputs, EffectStats, hard_shadow_pass, rtao_pass};
use rurix_render::streaming::{PagedResource, StreamingEngine};
use rurix_render::temporal::common::{
    Mat4, compute_camera_mv, jitter_sequence, look_at_rh, perspective_rh_zo, reproject_sample,
};
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::taa::{ClampMode, TaaParams, taa_resolve};
use rurix_render::temporal::tsr::{TsrParams, TsrUpscaler};
use rurix_render::temporal::upscale::{UpscaleBackend, UpscaleInputs};

use crate::scene::{CAMERA, FAR_ID, SKY_COLOR, SUN_COLOR, SUN_DIR, Uc08Scene, to_transform3x4};

/// 渲染配置(out 分辨率 + 内部分辨率 = out/2,TSR 2×;与 uc06 同律)。
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub out_w: u32,
    pub out_h: u32,
    pub frames: u32,
    pub seed: u64,
}

impl Default for RenderConfig {
    fn default() -> Self {
        RenderConfig {
            out_w: 128,
            out_h: 72,
            frames: 96,
            seed: 0x5255_5258_5543_0008, // "RURXUC"+8:固定默认种子
        }
    }
}

impl RenderConfig {
    pub fn internal_w(&self) -> u32 {
        (self.out_w / 2).max(2)
    }
    pub fn internal_h(&self) -> u32 {
        (self.out_h / 2).max(2)
    }
}

/// 流送剧本帧号(随帧数缩放:默认 96 帧 → K=10 / M=29 ≈ 任务书 K≈10,M≈K+20;
/// 短跑(如 device 烟跑 4 帧)同比例压缩,保证驻留沿/卸载沿在窗内可观测)。
pub fn script_k(frames: u32) -> u32 {
    (frames / 4).clamp(1, 10)
}

/// 剧本化卸载帧(≤ frames-1;≥ K+1 当帧数允许)。
pub fn script_m(frames: u32) -> u32 {
    let k = script_k(frames);
    let m = k + (frames / 5).clamp(1, 20);
    m.min(frames.saturating_sub(1))
        .max(k.min(frames.saturating_sub(1)))
}

/// 相机矩阵集(view/proj/view_proj/inv)。
pub struct CameraMats {
    pub view_proj: Mat4,
    pub inv_view_proj: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
    pub eye: [f32; 3],
    pub cull: CullCamera,
}

pub fn camera_matrices(w: u32, h: u32) -> CameraMats {
    let aspect = w as f32 / h as f32;
    let view = look_at_rh(CAMERA.eye, CAMERA.center, CAMERA.up);
    let proj = perspective_rh_zo(CAMERA.fov_y, aspect, CAMERA.z_near, CAMERA.z_far);
    let view_proj = proj.mul(&view);
    let inv_view_proj = view_proj.inverse().expect("相机矩阵必须可逆");
    CameraMats {
        view_proj,
        inv_view_proj,
        view,
        proj,
        eye: CAMERA.eye,
        cull: CullCamera {
            view_proj: view_proj.m,
            cam_pos: CAMERA.eye,
            screen_height_px: h as f32,
            error_threshold_px: 1.0,
        },
    }
}

/// 单网格页式资源(uc06 同款;远场景资源 root 页表为空 = 初始不停驻,
/// 页请求驱动驻留——流送剧本的「初始不请求」语义)。
struct MeshPages {
    id: u32,
    bytes: Vec<u8>,
    roots: Vec<u32>,
}

impl PagedResource for MeshPages {
    fn resource_id(&self) -> u32 {
        self.id
    }
    fn page_count(&self) -> u32 {
        1
    }
    fn root_pages(&self) -> &[u32] {
        &self.roots
    }
    fn read_page(&self, page: u32) -> Vec<u8> {
        assert_eq!(page, 0);
        self.bytes.clone()
    }
}

/// 应用侧页所有权凭证模型(RFC-0017 §4.B4:页卸载 → 先卸 body 凭 RemovalReceipt
/// 再放页)。`release` 按值消耗 receipt —— **编译期**保证「无 receipt 不可放页」
/// (RemovalReceipt 移动语义、不可 Clone);运行期计数进 JSON 断言。
#[derive(Debug, Default)]
pub struct PageCache {
    /// 放页次数。
    pub releases: u32,
    /// 消耗的 receipt 数(与 releases 恒等;凭证语义的运行时镜像)。
    pub receipts_consumed: u32,
}

impl PageCache {
    /// 放页(编译期凭证:receipt 按值消耗,无 receipt 无法调用本函数)。
    pub fn release(&mut self, receipt: rurix_physics::RemovalReceipt) {
        let _page = receipt.page();
        let _bodies = receipt.removed_bodies().len();
        self.releases += 1;
        self.receipts_consumed += 1;
        drop(receipt);
    }
}

/// 跨帧状态(历史/页表/物理世界/双桥/AS 管理/流送引擎/PSO/TSR backend)。
pub struct PipelineState {
    pub compiled: rurix_render::graph::CompiledGraph,
    #[allow(dead_code)]
    pub frame_res: crate::graph_setup::FrameResources,
    pub cam: CameraMats,
    pub prev_view_proj: Mat4,
    pub streaming: StreamingEngine,
    pub pso: crate::shading::PsoSet,
    pub taa_history: Option<ImageF32>,
    pub depth_history: Option<ImageF32>,
    pub gi_history: Option<GiHistory>,
    pub tsr: TsrUpscaler,
    pub normals_prev: Option<ImageF32>,
    pub ao_history: Option<ImageF32>,
    pub shadow_history: Option<ImageF32>,
    pub frame_diff_series: Vec<f64>,
    pub jitter: Vec<[f32; 2]>,
    // --- G6.3 合流状态 ---
    /// 物理世界(Jolt 后端;dt_fixed = 1/60 位级不变)。
    pub world: PhysicsWorld,
    /// 物理 → GpuScene 同步桥(body ↔ 实例注册面 + 每帧同步)。
    pub bridge: PhysicsBridge,
    /// 页 ↔ body 流送桥(驻留批插/卸载 receipt)。
    pub streaming_bridge: StreamingBridge,
    /// GpuScene 实例表(桥的唯一写目标;渲染侧唯一事实来源)。
    pub gpu_scene: GpuScene,
    /// 页所有权凭证(无 receipt 不可放页)。
    pub page_cache: PageCache,
    /// BLAS 缓存(刚体一律 Static;零 refit 断言源)。
    pub blas_cache: BlasCache,
    /// TLAS 增量构建器(标脏 + rebuild_if_dirty)。
    pub tlas_builder: TlasBuilder,
    /// 当帧 TLAS(RT/GI 效果消费;脏帧由 builder 重建替换)。
    pub tlas: Tlas,
    /// 实例 → TLAS 槽位映射(TlasBuilder 自有槽位,demo 侧显式维护)。
    pub instance_slots: HashMap<u32, u32>,
    /// 在册动态体(body, 实例;睡眠沿判定 + transform_landed 断言遍历面;
    /// 静态体零脏写不参与变换断言,卸载沿同步摘除)。
    pub tracked_dynamic: Vec<(BodyId, u32)>,
    /// 停靠实例集(未驻留/已卸载;VSM 世界三角形排除面)。
    pub parked: HashSet<u32>,
    /// 远场景页已批插(insert 沿一次性)。
    pub far_inserted: bool,
    /// 远场景页已放页(remove 沿一次性)。
    pub far_released: bool,
    // --- evidence 计数(数字进 JSON,不进硬门) ---
    pub physics_steps: u64,
    pub physics_total_ms: f64,
    pub bodies_seen_last: u32,
    pub bodies_written_last: u32,
    pub writes_truncated_total: u64,
    pub sleep_frame: Option<u32>,
    pub insert_frame: Option<u32>,
    pub remove_frame: Option<u32>,
    pub receipt_bodies: u32,
    pub transform_landed_max_err: f32,
    pub early_mv_max: f32,
    pub post_sleep_mv_max: f32,
}

impl PipelineState {
    pub fn new(scene: &Uc08Scene, cfg: &RenderConfig) -> Self {
        let iw = cfg.internal_w();
        let ih = cfg.internal_h();
        let pool_est = 4096u64 * 128 * 128 * 4;
        let (compiled, frame_res) =
            crate::graph_setup::build_frame_graph(iw, ih, cfg.out_w, cfg.out_h, pool_est);
        let cam = camera_matrices(iw, ih);
        // 流送:近场景 6 资源 root 页钉住常驻;远场景资源 root 空 = 初始不停驻。
        let mut streaming = StreamingEngine::new(8);
        for (mid, m) in scene.meshes.iter().enumerate() {
            let mut bytes = Vec::new();
            for c in &m.dag.records {
                bytes.extend_from_slice(&bytemuck_cluster(c));
            }
            let roots = if mid as u32 == FAR_ID {
                Vec::new()
            } else {
                vec![0]
            };
            streaming.register_resource(Box::new(MeshPages {
                id: mid as u32,
                bytes,
                roots,
            }));
        }
        let pso = crate::shading::make_pso_set(&scene.materials);
        // GpuScene(实例序 = 网格序;远场景停靠位姿)。
        let gpu_scene = crate::scene::build_gpu_scene(scene);
        // 物理世界 + 初始体(地面 + 5 近立方体;远场景体走流送批插)。
        let mut world = PhysicsWorld::new(WorldDesc::default())
            .expect("Jolt 后端已编译(default feature jolt),WorldDesc::default 合法");
        let bodies = world
            .add_bodies_batch(&scene.body_descs)
            .expect("初始体描述合法、池有余量(场景构建契约)");
        // 同步桥注册面(初始体全量;远场景体在驻留沿注册)。
        let mut bridge = PhysicsBridge::new();
        let mut tracked_dynamic = Vec::new();
        for (i, &b) in bodies.iter().enumerate() {
            let iid = scene.body_instances[i];
            bridge.register(b, iid, scene.body_kinds[i]);
            if scene.body_kinds[i] == BodyKind::Dynamic {
                tracked_dynamic.push((b, iid));
            }
        }
        // AS 分级:BLAS 对象空间 Static(零 refit);TLAS 实例初始变换
        // (远场景停靠 → Tlas::build 语义下正常参与,视锥外零命中)。
        let mut blas_cache = BlasCache::new();
        let mut tlas_builder = TlasBuilder::new();
        let mut instance_slots = HashMap::new();
        for (mid, m) in scene.meshes.iter().enumerate() {
            let (pos, idx) = crate::scene::blas_inputs(m);
            let blas: BlasId = blas_cache.get_or_build(&pos, &idx, DynamicPolicy::Static);
            // 阴影光线掩码(uc06 同款:地面 = 0xFE 允许被排除防自遮挡伪影,
            // 立方体 = 0xFF 恒可见——遮挡者不被掩码排除)。
            let mask = if mid == 0 { 0xFE } else { 0xFF };
            let slot = tlas_builder.add_instance(
                TlasInstance::new(blas, to_transform3x4(&scene.initial_transforms[mid]))
                    .with_mask(mask),
            );
            instance_slots.insert(mid as u32, slot);
        }
        let tlas = tlas_builder
            .rebuild_if_dirty(&mut blas_cache)
            .expect("首建必脏(空构建器)");
        let mut parked = HashSet::new();
        parked.insert(FAR_ID);
        PipelineState {
            compiled,
            frame_res,
            prev_view_proj: cam.view_proj,
            cam,
            streaming,
            pso,
            taa_history: None,
            depth_history: None,
            gi_history: None,
            tsr: TsrUpscaler::new(TsrParams::default()),
            normals_prev: None,
            ao_history: None,
            shadow_history: None,
            frame_diff_series: Vec::new(),
            jitter: jitter_sequence(cfg.frames.max(16)),
            world,
            bridge,
            streaming_bridge: StreamingBridge::new(),
            gpu_scene,
            page_cache: PageCache::default(),
            blas_cache,
            tlas_builder,
            tlas,
            instance_slots,
            tracked_dynamic,
            parked,
            far_inserted: false,
            far_released: false,
            physics_steps: 0,
            physics_total_ms: 0.0,
            bodies_seen_last: 0,
            bodies_written_last: 0,
            writes_truncated_total: 0,
            sleep_frame: None,
            insert_frame: None,
            remove_frame: None,
            receipt_bodies: 0,
            transform_landed_max_err: 0.0,
            early_mv_max: 0.0,
            post_sleep_mv_max: 0.0,
        }
    }
}

/// 簇记录字节化(uc06 同款 64B 冻结契约逐字段 LE 手写,零 unsafe 纪律)。
fn bytemuck_cluster(c: &rurix_render::graph::types::ClusterRecord) -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut put = |off: usize, bytes: &[u8]| {
        out[off..off + bytes.len()].copy_from_slice(bytes);
    };
    for (i, v) in c.center.iter().enumerate() {
        put(i * 4, &v.to_le_bytes());
    }
    put(12, &c.radius.to_le_bytes());
    for (i, v) in c.cone_axis.iter().enumerate() {
        put(16 + i * 4, &v.to_le_bytes());
    }
    put(28, &c.cone_cutoff.to_le_bytes());
    put(32, &c.error.to_le_bytes());
    put(36, &c.parent_error.to_le_bytes());
    put(40, &c.vertex_offset.to_le_bytes());
    put(44, &c.triangle_offset.to_le_bytes());
    put(48, &c.vertex_count.to_le_bytes());
    put(52, &c.triangle_count.to_le_bytes());
    put(56, &c.page_id.to_le_bytes());
    put(60, &c.reserved.to_le_bytes());
    out
}

/// 当帧 GI 场景(实例变换取 GpuScene 当前值;对象空间几何 + 逐实例 albedo)。
pub fn gi_scene_now(scene: &Uc08Scene, gpu: &GpuScene) -> GiScene {
    let instances: Vec<GiMeshInstance> = scene
        .meshes
        .iter()
        .enumerate()
        .map(|(i, m)| GiMeshInstance {
            positions: m.object_triangles.iter().flatten().copied().collect(),
            indices: (0..m.object_triangles.len() as u32)
                .map(|t| [3 * t, 3 * t + 1, 3 * t + 2])
                .collect(),
            transform: to_transform3x4(&gpu.instances()[i].transform),
            albedo: rurix_render::material::closure::unpack(&scene.materials.closures()[i]).albedo,
        })
        .collect();
    GiScene::build(&instances, SUN_DIR, SUN_COLOR, SKY_COLOR)
}

/// 单帧阶段耗时报告。
#[derive(Debug, Clone, Default)]
pub struct FrameReport {
    pub stages: Vec<(&'static str, f64)>,
    pub final_hdr: Option<ImageF32>,
    pub final_tsr: Option<ImageF32>,
    pub shadow_lit_ratio: f32,
    pub hdr_mean: f64,
    pub hdr_std: f64,
}

/// 单帧执行(物理 → 同步 → 渲染;frame 0 初始化历史)。
pub fn run_frame(
    scene: &Uc08Scene,
    st: &mut PipelineState,
    cfg: &RenderConfig,
    frame: u32,
) -> FrameReport {
    let iw = cfg.internal_w();
    let ih = cfg.internal_h();
    let mut rep = FrameReport::default();
    let k = script_k(cfg.frames);
    let m_unload = script_m(cfg.frames);
    // app 侧 GpuScene 写(驻留上线/卸载停靠;非物理驱动,随本帧 flush 一并上报)。
    let mut app_dirty: Vec<u32> = Vec::new();

    // ① 流送:tick(三预算分段扣账)→ 驻留沿批插 body → 剧本卸载凭 receipt 放页。
    let t = Instant::now();
    let mut fb = rurix_render::streaming::FeedbackBuilder::new(frame);
    for (mid, _) in scene.meshes.iter().enumerate() {
        // 近场景常驻(root 钉住;请求 = 零成本触帧);远场景仅剧本窗口内请求。
        if mid as u32 == FAR_ID && !(k..m_unload).contains(&frame) {
            continue;
        }
        fb.add(
            mid as u32,
            0,
            rurix_render::streaming::FEEDBACK_BASE_GEOMETRY_LOD,
            1,
        );
    }
    let reqs = fb.build();
    st.streaming.submit_requests(&reqs);
    let budget = StreamingBudget {
        io_bytes: 1 << 20,
        transcode_bytes: 1 << 20,
        upload_bytes: 1 << 20,
    };
    let _tick = st.streaming.tick(frame, &budget);

    // 驻留沿(§4.B2:页驻留 → body 批插):远场景页本帧驻留且尚未批插。
    if !st.far_inserted && st.streaming.is_resident(FAR_ID, 0) {
        let key = rurix_physics::PageKey {
            resource: FAR_ID,
            page: 0,
        };
        let bodies = st
            .streaming_bridge
            .insert_page(&mut st.world, key, std::slice::from_ref(&scene.far_desc))
            .expect("远场景页批插必须成功(描述合法、池有余量)");
        for &b in &bodies {
            st.bridge.register(b, FAR_ID, BodyKind::Dynamic);
            st.tracked_dynamic.push((b, FAR_ID));
        }
        // 渲染实例上线:停靠位姿 → 出生位姿(app 侧写,非物理驱动)。
        assert!(st.gpu_scene.update_transform(FAR_ID, scene.far_spawn));
        app_dirty.push(FAR_ID);
        st.parked.remove(&FAR_ID);
        st.far_inserted = true;
        st.insert_frame = Some(frame);
    }

    // 剧本化卸载(§4.B4:先卸 body 凭 RemovalReceipt 再放页)。
    if frame == m_unload && st.far_inserted && !st.far_released {
        let key = rurix_physics::PageKey {
            resource: FAR_ID,
            page: 0,
        };
        let receipt = st
            .streaming_bridge
            .remove_page(&mut st.world, key)
            .expect("远场景页卸载必须成功(页在 watch 表)");
        st.receipt_bodies = receipt.removed_bodies().len() as u32;
        for &b in receipt.removed_bodies() {
            st.bridge.unregister(b);
            st.tracked_dynamic.retain(|&(tb, _)| tb != b);
        }
        // 编译期凭证:release 按值消耗 receipt(无 receipt 不可放页)。
        st.page_cache.release(receipt);
        debug_assert!(
            st.streaming_bridge
                .bodies_of(key)
                .is_none_or(|bs| bs.is_empty()),
            "放页后页 → body 映射必须为空(§4.B4 运行时镜像)"
        );
        // 渲染实例下线:回停靠位姿(GpuScene 无实例移除面;视锥外零像素)。
        assert!(
            st.gpu_scene
                .update_transform(FAR_ID, crate::scene::PARKED_3X4)
        );
        app_dirty.push(FAR_ID);
        st.parked.insert(FAR_ID);
        st.far_released = true;
        st.remove_frame = Some(frame);
    }
    rep.stages
        .push(("streaming", t.elapsed().as_secs_f64() * 1000.0));

    // ② 物理步(固定步位级 = WorldDesc.dt_fixed;耗时 measured 进 evidence 不进硬门)。
    let t = Instant::now();
    let dt = st.world.desc().dt_fixed;
    let _stats = st
        .world
        .step(dt)
        .expect("固定步长位级一致(宿主每拍同 dt),step 不得失败");
    let phys_ms = t.elapsed().as_secs_f64() * 1000.0;
    st.physics_steps += 1;
    st.physics_total_ms += phys_ms;
    rep.stages.push(("physics", phys_ms));

    // 睡眠沿:全部在册动态体 is_active = false 的首帧(远场景体卸载后不在册)。
    if st.sleep_frame.is_none()
        && !st.tracked_dynamic.is_empty()
        && st
            .tracked_dynamic
            .iter()
            .all(|&(b, _)| st.world.is_active(b) == Ok(false))
    {
        st.sleep_frame = Some(frame);
    }

    // ③ 同步桥 + AS 分级(sync_frame 只写 active 动态/运动体,静态/睡眠零脏写;
    // 帧末内部 flush_dirty;每帧新 SyncBudget = §4.A6 重置语义)。
    let t = Instant::now();
    let mut sync_budget = SyncBudget::new(1024, 4096, 256);
    let report = st
        .bridge
        .sync_frame(&st.world, &mut st.gpu_scene, &mut sync_budget);
    st.bodies_seen_last = report.bodies_seen;
    st.bodies_written_last = report.bodies_written;
    st.writes_truncated_total = st
        .writes_truncated_total
        .saturating_add(u64::from(report.writes_truncated));

    // transform_landed:逐在册动态体 GpuScene == compose(body_transform)
    // (§4.B2 单向写正确性;静态体零脏写不参与——其渲染位姿由场景构建对齐)。
    // 睡眠体同样满足等式,但**入睡边界**存在 Jolt 行为级微调:入睡拍体脱离
    // active_transforms(bridge 停写),当拍 body_transform 与末次 active 快照
    // 有数值级小差(实测 ≤ 5.6e-5,亚毫米)——属零脏写语义的合理包络,
    // 容差取 1e-4(近零容差;active 期逐位为 0)。
    let mut frame_err = 0.0f32;
    for &(b, iid) in &st.tracked_dynamic {
        let Ok(pt) = st.world.body_transform(b) else {
            continue;
        };
        let expect = compose_transform_3x4(&pt);
        let got = st.gpu_scene.instances()[iid as usize].transform;
        for r in 0..3 {
            for c in 0..4 {
                frame_err = frame_err.max((expect[r][c] - got[r][c]).abs());
            }
        }
    }
    st.transform_landed_max_err = st.transform_landed_max_err.max(frame_err);

    // TLAS 增量(物理脏实例 ∪ app 上线/下线实例 → 标脏 → rebuild_if_dirty;
    // 刚体 BLAS Static 零 refit,运动全部走实例变换,§4.B AS 分级裁决)。
    for &iid in report.dirty_instances.iter().chain(app_dirty.iter()) {
        let cur = st.gpu_scene.instances()[iid as usize].transform;
        if let Some(&slot) = st.instance_slots.get(&iid) {
            st.tlas_builder
                .update_transform(slot, to_transform3x4(&cur))
                .expect("TLAS 槽位在册(实例→槽映射自维护)");
        }
    }
    if let Some(tlas) = st.tlas_builder.rebuild_if_dirty(&mut st.blas_cache) {
        st.tlas = tlas;
    }
    rep.stages
        .push(("bridge_sync", t.elapsed().as_secs_f64() * 1000.0));

    // ④ 两级剔除(CPU 参照剔除器 + GPU-driven 语义对拍;GpuScene 实例变换已同步)。
    let t = Instant::now();
    let view = CullView::new(
        GeomMat4(st.cam.view.m),
        GeomMat4(st.cam.proj.m),
        st.cam.eye,
        ih as f32,
    );
    let (visible_set, _cull_stats) = cull_clusters(&scene.clusters, &view);
    let visible: Vec<rurix_render::geometry::cull::VisibleCluster> = scene
        .meshes
        .iter()
        .enumerate()
        .flat_map(|(iid, m)| {
            let lo = m.cluster_offset;
            let set = &visible_set;
            (0..m.dag.records.len() as u32).filter_map(move |c| {
                let gid = lo + c;
                set.contains(&gid)
                    .then_some(rurix_render::geometry::cull::VisibleCluster {
                        instance: iid as u32,
                        cluster: gid,
                    })
            })
        })
        .collect();
    let draw_args = compact_draw_args(
        &visible,
        st.gpu_scene.instances(),
        &scene.clusters,
        &st.cam.cull,
        32.0,
    );
    rep.stages
        .push(("cull", t.elapsed().as_secs_f64() * 1000.0));

    // ⑤ VisBuffer 光栅(SW 参考路;实例变换 = 当帧物理位姿)。
    let t = Instant::now();
    let mut vis = VisBufferCpu::new(iw, ih);
    let rs = RasterScene {
        instances: st.gpu_scene.instances(),
        clusters: &scene.clusters,
        vertices: &scene.vertices,
        indices: &scene.indices,
        view_proj: st.cam.view_proj.m,
    };
    let all_visible: Vec<rurix_render::geometry::cull::VisibleCluster> = draw_args
        .hw_clusters
        .iter()
        .chain(draw_args.sw_clusters.iter())
        .cloned()
        .collect();
    raster_clusters(&mut vis, &all_visible, &rs);
    rep.stages
        .push(("visbuffer", t.elapsed().as_secs_f64() * 1000.0));

    // ⑥ 材质 classify/resolve。**c2m 按可见列表序**:VisBuffer 的 cluster27 =
    // 「传入光栅的可见簇列表位置」(visbuffer.rs 帧内可见簇列表下标裁决),非全局
    // 簇 id——classify/resolve 以其直接索引 c2m,故 c2m 必须与 all_visible 同序
    // (VisibleCluster.instance = 网格 id = 材质 id,零查表)。mat_ids = 逐像素
    // 材质 = 实例 id,MV 阶段的实例覆盖图与之同源,零额外光栅。
    let t = Instant::now();
    let c2m: Vec<u16> = all_visible.iter().map(|vc| vc.instance as u16).collect();
    let class = classify(&vis, &c2m, 8);
    let mat_ids = resolve(&vis, &c2m);
    let _ = class;
    rep.stages
        .push(("mat_resolve", t.elapsed().as_secs_f64() * 1000.0));

    // ⑦ GBuffer(针孔;gi_scene 取当帧实例变换,深度 NDC z ∈ [0,1] + 世界法线)。
    let t = Instant::now();
    let gi_cam = GiCamera::new(st.cam.view_proj);
    let gi_scene = gi_scene_now(scene, &st.gpu_scene);
    let (depth, normals) =
        rurix_render::gi::pipeline::render_gbuffer_pinhole(&gi_scene, &gi_cam, iw, ih);
    rep.stages
        .push(("gbuffer", t.elapsed().as_secs_f64() * 1000.0));

    // ⑧ MV(静态相机 → 相机分量恒零;motion_hints 逐动态实例覆写物体重投影 MV,
    // 缺席 = 零 MV。object = inverse(cur_3x4)·world(刚体转置公式),prev_world =
    // prev_3x4·object,mv = project(prev_vp, prev_world) − project(cur_vp, world),
    // 与 temporal/common.rs `mv = prev_uv − cur_uv` 约定一致)。
    let t = Instant::now();
    let mut mv = compute_camera_mv(&depth, &st.cam.view_proj, &st.prev_view_proj);
    for hint in st.bridge.motion_hints() {
        let inv_cur = crate::scene::invert_rigid_3x4(&hint.cur_transform);
        for y in 0..ih {
            for x in 0..iw {
                let idx = (y * iw + x) as usize;
                if mat_ids[idx] == MATERIAL_INVALID || mat_ids[idx] as u32 != hint.instance {
                    continue;
                }
                let d = depth.get(x, y, 0);
                if d >= 1.0 {
                    continue;
                }
                let world = unproject(&st.cam.inv_view_proj, x, y, d, iw, ih);
                let object = rurix_render::geometry::gpu_scene::transform_point(&inv_cur, world);
                let prev_world = rurix_render::geometry::gpu_scene::transform_point(
                    &hint.prev_transform,
                    object,
                );
                if let (Some((pu, pv)), Some((cu, cv))) = (
                    project_uv(&st.prev_view_proj, prev_world),
                    project_uv(&st.cam.view_proj, world),
                ) {
                    mv.set(x, y, 0, pu - cu);
                    mv.set(x, y, 1, pv - cv);
                }
            }
        }
    }
    let mv_max = max_mv_magnitude(&mv);
    // 下落早期窗口(帧数缩放:96 帧 → 1..=12;4 帧 → 1..=2)。
    let early_end = (cfg.frames / 2).clamp(2, 12);
    if (1..=early_end).contains(&frame) {
        st.early_mv_max = st.early_mv_max.max(mv_max);
    }
    // 睡眠后帧(含睡眠沿当帧):motion_hints 已空 → 全图 MV ≈ 相机 MV = 0。
    if let Some(sf) = st.sleep_frame
        && frame >= sf
    {
        st.post_sleep_mv_max = st.post_sleep_mv_max.max(mv_max);
    }
    rep.stages.push(("mv", t.elapsed().as_secs_f64() * 1000.0));

    // ⑨ VSM(mark → alloc → raster → sample;当帧世界三角形,停靠实例排除)。
    let t = Instant::now();
    let world_tris = crate::scene::world_tris_now(scene, &st.gpu_scene, &st.parked);
    let mut vsm = crate::shading::make_vsm(&world_tris);
    vsm.page_mark(&depth, &st.cam.inv_view_proj);
    vsm.page_alloc();
    vsm.shadow_depth_raster(&world_tris);
    rep.stages.push(("vsm", t.elapsed().as_secs_f64() * 1000.0));

    // ⑩ GI(时域累积经公共底座;历史双缓冲;MV 输入含物体分量)。
    let t = Instant::now();
    let gi_params = GiParams {
        seed: cfg.seed,
        ..Default::default()
    };
    let tracer = rurix_render::gi::tracer::RayTracedRadiance::new(gi_scene);
    let gi_out = render_gi(
        &depth,
        &normals,
        &gi_cam,
        &tracer,
        st.gi_history.as_ref(),
        Some(&mv),
        &gi_params,
    );
    st.gi_history = Some(gi_out.history.clone());
    rep.stages.push(("gi", t.elapsed().as_secs_f64() * 1000.0));

    // ⑪ RTAO + 硬阴影 + 时域滤波(TLAS = 当帧增量重建,BLAS = 对象空间 Static)。
    let t = Instant::now();
    let mut stats = EffectStats::default();
    let eff = EffectInputs::new(&depth, &normals, st.cam.view_proj, &st.tlas, &st.blas_cache);
    let ao_raw = rtao_pass(&eff, 2, 0.5, frame, cfg.seed, &mut stats);
    let shadow_raw = hard_shadow_pass(&eff, SUN_DIR, &mut stats);
    let _ = stats;
    let prev_depth = st.depth_history.clone().unwrap_or_else(|| depth.clone());
    let prev_normals = st.normals_prev.clone().unwrap_or_else(|| normals.clone());
    let filter_params = TemporalFilterParams::default();
    // 历史双缓冲:帧 0 以当前帧为历史(零阶启动),帧 ≥1 真历史。
    let ao_filtered = temporal_filter_effect(
        &ao_raw,
        st.ao_history.as_ref().unwrap_or(&ao_raw),
        &mv,
        &depth,
        &prev_depth,
        &normals,
        &prev_normals,
        &filter_params,
    );
    let shadow_filtered = temporal_filter_effect(
        &shadow_raw,
        st.shadow_history.as_ref().unwrap_or(&shadow_raw),
        &mv,
        &depth,
        &prev_depth,
        &normals,
        &prev_normals,
        &filter_params,
    );
    st.ao_history = Some(ao_filtered.clone());
    st.shadow_history = Some(shadow_filtered.clone());
    rep.stages
        .push(("rt_effects", t.elapsed().as_secs_f64() * 1000.0));

    // ⑫ VSM 逐像素采样 + 与硬阴影取保守 min(uc06 同款双路阴影证据)。
    let t = Instant::now();
    let mut shadow_map = ImageF32::new(iw, ih, 1);
    for y in 0..ih {
        for x in 0..iw {
            let d = depth.get(x, y, 0);
            if d >= 1.0 {
                shadow_map.set(x, y, 0, 1.0);
                continue;
            }
            let world = unproject(&st.cam.inv_view_proj, x, y, d, iw, ih);
            shadow_map.set(x, y, 0, vsm.sample_shadow(world));
        }
    }
    for y in 0..ih {
        for x in 0..iw {
            let s = shadow_map.get(x, y, 0).min(shadow_filtered.get(x, y, 0));
            shadow_map.set(x, y, 0, s);
        }
    }
    rep.stages
        .push(("shadow_project", t.elapsed().as_secs_f64() * 1000.0));

    // ⑬ 材质求值(单层闭合延迟着色)。
    let t = Instant::now();
    let hdr = crate::shading::shade_frame(
        &mat_ids,
        MATERIAL_INVALID,
        iw,
        ih,
        &scene.materials,
        &normals,
        &shadow_map,
        &gi_out.irradiance,
        &ao_filtered,
        SKY_COLOR,
    );
    rep.stages
        .push(("deferred_shade", t.elapsed().as_secs_f64() * 1000.0));

    // ⑭ TAA(历史经公共底座;MV = 相机分量 + 动态体分量合成图)。
    let t = Instant::now();
    let taa_params = TaaParams {
        clamp_mode: ClampMode::Aabb,
        ..Default::default()
    };
    let taa_out = match &st.taa_history {
        Some(hist) => {
            let (hist_reproj, validity) = reproject_sample(hist, &mv);
            taa_resolve(&hdr, &hist_reproj, &mv, &validity, &taa_params)
        }
        None => hdr.clone(),
    };
    st.taa_history = Some(taa_out.clone());
    st.depth_history = Some(depth.clone());
    st.normals_prev = Some(normals.clone());
    rep.stages.push(("taa", t.elapsed().as_secs_f64() * 1000.0));

    // ⑮ TSR 超分(输出 out;输入/输出分辨率解耦)。
    let t = Instant::now();
    let inputs = UpscaleInputs {
        color: &taa_out,
        depth: &depth,
        mv: &mv,
        reactive: None,
        exposure: 1.0,
        jitter: st.jitter[(frame as usize) % st.jitter.len()],
        output_size: (cfg.out_w, cfg.out_h),
        frame_index: frame,
        reset: frame == 0,
    };
    let tsr_out = st.tsr.upscale(&inputs);
    rep.stages.push(("tsr", t.elapsed().as_secs_f64() * 1000.0));

    // 帧指标(亮度统计 + 帧间差;静态相机下后段收敛 = 时域底座证据)。
    let (mean, std) = image_stats(&hdr);
    rep.hdr_mean = mean;
    rep.hdr_std = std;
    rep.shadow_lit_ratio = shadow_lit_ratio(&shadow_map);
    rep.final_hdr = Some(hdr);
    rep.final_tsr = Some(tsr_out);
    st.frame_diff_series.push(mean);
    // prev_view_proj 滚动(静态相机 = 同矩阵;时域口径诚实留口)。
    st.prev_view_proj = st.cam.view_proj;
    rep
}

/// 汇总(host 断言面;JSON/exit 判定源)。
pub struct Uc08Summary {
    pub mode: &'static str,
    pub frames: u32,
    pub width: u32,
    pub height: u32,
    pub internal_width: u32,
    pub internal_height: u32,
    pub stages: Vec<(String, f64)>,
    pub asserts: Vec<(String, bool)>,
    pub pso_warnings: u64,
    pub graph_pass_count: usize,
    pub graph_barrier_count: usize,
    pub graph_fence_count: usize,
    pub graph_alias_peak: u64,
    pub graph_no_alias_peak: u64,
    pub final_mean: f64,
    pub final_std: f64,
    pub shadow_lit_ratio: f32,
    pub frame_means: Vec<f64>,
    pub streaming_pop_in: u64,
    pub physics_steps: u64,
    pub physics_total_ms: f64,
    pub bodies_seen_last: u32,
    pub bodies_written_last: u32,
    pub writes_truncated_total: u64,
    pub sleep_frame: Option<u32>,
    pub insert_frame: Option<u32>,
    pub remove_frame: Option<u32>,
    pub receipt_bodies: u32,
    pub releases: u32,
    pub transform_landed_max_err: f32,
    pub early_mv_max: f32,
    pub post_sleep_mv_max: f32,
    pub tlas_rebuilds: u64,
    pub blas_builds: u64,
    pub blas_refits: u64,
    pub tracked_count: usize,
    pub watched_count: usize,
}

/// device 腿结果(device.rs 真跑产出;字段 = JSON 冻结面)。
pub struct DeviceLeg {
    pub device_name: String,
    pub steps_before_a: u32,
    pub steps_before_b: u32,
    pub pixels_a: u32,
    pub pixels_b: u32,
    pub changed_pixels: u32,
    pub device_pixels_nontrivial: bool,
    pub device_motion_pixels_changed: bool,
}

pub fn assemble_summary(
    st: &PipelineState,
    frames: &[FrameReport],
    device_requested: bool,
) -> Result<Uc08Summary, String> {
    let last = frames.last().ok_or("frames 为空")?;
    let (mean, std) = (last.hdr_mean, last.hdr_std);
    let n_frames = frames.len() as u32;

    let mut asserts: Vec<(String, bool)> = Vec::new();
    // ① 物理步耗时 measured:stages 含 physics 且累计 > 0(P-09:数字进
    // evidence 不进硬门——断言只锁「埋点存在且非零」,不锁耗时阈值)。
    let has_physics_stage = last.stages.iter().any(|(n, _)| *n == "physics");
    asserts.push((
        "physics_step_measured".into(),
        has_physics_stage && st.physics_total_ms > 0.0,
    ));
    // ② 变换落地:全部在册动态体 GpuScene == compose(body_transform),逐元素
    // 近零容差 1e-4(active 期逐位为 0;入睡边界微调包络,见 run_frame 注)。
    asserts.push((
        "transform_landed".into(),
        st.transform_landed_max_err <= 1e-4,
    ));
    // ③ 下落早期 MV:动态体覆盖区 MV 幅值 > 阈(物体 MV 唯一 MV 源)。
    asserts.push(("mv_dynamic_present_early".into(), st.early_mv_max > 5e-4));
    // ④ 睡眠后零 MV:全部在册动态体睡眠后帧全图 MV ≈ 相机 MV = 0
    // (短跑未观测到睡眠时 vacuous;长跑由 ⑬ 兜底睡眠到达)。
    asserts.push(("mv_zero_after_sleep".into(), st.post_sleep_mv_max <= 1e-4));
    // ⑤ 流送驻留沿:批插发生(页驻留 → body 批插)。
    asserts.push(("streaming_insert_seen".into(), st.insert_frame.is_some()));
    // ⑥ 卸载 receipt:remove_page 拿到 receipt 且携带 ≥1 body。
    asserts.push((
        "streaming_remove_receipt_seen".into(),
        st.remove_frame.is_some() && st.receipt_bodies >= 1,
    ));
    // ⑦ 凭 receipt 放页:放页次数 == 消耗 receipt 数 == 1(编译期凭证 +
    // 运行期计数镜像;先 receipt 后放页的次序由移动语义强制)。
    asserts.push((
        "release_after_receipt_only".into(),
        st.page_cache.releases == 1 && st.page_cache.releases == st.page_cache.receipts_consumed,
    ));
    // ⑧ TLAS 重建 ≥1(动态体标脏驱动增量重建真发生)。
    asserts.push((
        "tlas_rebuilds_ge1".into(),
        st.blas_cache.stats().tlas_rebuilds >= 1,
    ));
    // ⑨ 刚体 BLAS Static 零 refit(运动全部走 TLAS 实例变换)。
    asserts.push((
        "blas_static_zero_refit".into(),
        st.blas_cache.stats().refits == 0,
    ));
    // ⑩ 最终帧非平凡:亮度方差非零(非全黑全白)。
    asserts.push(("final_image_nontrivial".into(), std > 1e-4 && mean > 1e-3));
    // ⑪ 材质求值影内外差:最终帧亮度分布非均匀(有明有暗)。
    asserts.push(("shading_has_contrast".into(), std > 0.01));
    // ⑫ 时域收敛:帧均值序列后段 |Δ| ≤ 前段(动态期变化大、睡眠后趋零;
    // 短跑 <8 帧 vacuous,与 uc06 n<4 同款口径)。
    let s = &st.frame_diff_series;
    let conv = if s.len() >= 8 {
        let early: f64 = s[1..=5]
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .sum::<f64>()
            / 4.0;
        let n = s.len();
        let late: f64 = s[n - 5..]
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .sum::<f64>()
            / 4.0;
        late <= early + 1e-6
    } else {
        true
    };
    asserts.push(("temporal_converges".into(), conv));
    // ⑬ 长跑睡眠到达:frames ≥ 90 时必须观测到全部在册动态体睡眠
    // (短跑 vacuous;默认 96 帧实质约束——睡眠零 MV 的场景前提)。
    asserts.push((
        "sleep_reached_when_long_run".into(),
        n_frames < 90 || st.sleep_frame.is_some(),
    ));
    // ⑭ PSO 运行时编译告警归零(uc06 G-G5-7 同款)。
    asserts.push(("pso_zero_warnings".into(), st.pso.cache.warnings() == 0));
    // ⑮ 图结构:fence 非空 + 别名峰值 < 无别名(uc06 同款)。
    let pool = st.compiled.pool();
    asserts.push((
        "graph_fences_nonempty".into(),
        !st.compiled.fences().is_empty(),
    ));
    asserts.push((
        "graph_alias_saves".into(),
        pool.high_water() < pool.no_alias_peak(),
    ));

    let stages: Vec<(String, f64)> = last
        .stages
        .iter()
        .map(|(n, ms)| ((*n).to_owned(), *ms))
        .collect();

    Ok(Uc08Summary {
        mode: if device_requested { "device" } else { "host" },
        frames: n_frames,
        width: 0, // 由调用方按 cfg 回填
        height: 0,
        internal_width: 0,
        internal_height: 0,
        stages,
        asserts,
        pso_warnings: st.pso.cache.warnings(),
        graph_pass_count: st.compiled.passes().len(),
        graph_barrier_count: st.compiled.barriers().iter().map(|(_, b)| b.len()).sum(),
        graph_fence_count: st.compiled.fences().len(),
        graph_alias_peak: pool.high_water(),
        graph_no_alias_peak: pool.no_alias_peak(),
        final_mean: mean,
        final_std: std,
        shadow_lit_ratio: last.shadow_lit_ratio,
        frame_means: st.frame_diff_series.clone(),
        streaming_pop_in: st.streaming.pop_in_count(),
        physics_steps: st.physics_steps,
        physics_total_ms: st.physics_total_ms,
        bodies_seen_last: st.bodies_seen_last,
        bodies_written_last: st.bodies_written_last,
        writes_truncated_total: st.writes_truncated_total,
        sleep_frame: st.sleep_frame,
        insert_frame: st.insert_frame,
        remove_frame: st.remove_frame,
        receipt_bodies: st.receipt_bodies,
        releases: st.page_cache.releases,
        transform_landed_max_err: st.transform_landed_max_err,
        early_mv_max: st.early_mv_max,
        post_sleep_mv_max: st.post_sleep_mv_max,
        tlas_rebuilds: st.blas_cache.stats().tlas_rebuilds,
        blas_builds: st.blas_cache.stats().blas_builds,
        blas_refits: st.blas_cache.stats().refits,
        tracked_count: st.bridge.tracked_count(),
        watched_count: st.streaming_bridge.watched_count(),
    })
}

impl Uc08Summary {
    pub fn one_line(&self) -> String {
        format!(
            "mode={} frames={} mean={:.4} std={:.4} physics_ms={:.2} sleep={:?} tlas_rebuilds={} blas_refits={}",
            self.mode,
            self.frames,
            self.final_mean,
            self.final_std,
            self.physics_total_ms,
            self.sleep_frame,
            self.tlas_rebuilds,
            self.blas_refits
        )
    }

    pub fn all_asserts_pass(&self, device: Option<&DeviceLeg>) -> bool {
        let host_ok = self.asserts.iter().all(|(_, ok)| *ok);
        let dev_ok = match device {
            Some(d) => d.device_pixels_nontrivial && d.device_motion_pixels_changed,
            None => true,
        };
        host_ok && dev_ok
    }
}

fn opt_u32(v: Option<u32>) -> String {
    v.map_or_else(|| "null".into(), |x| x.to_string())
}

/// 单行 JSON(smoke 消费;字段集冻结)。
pub fn summary_json(s: &Uc08Summary, device: Option<&DeviceLeg>, device_requested: bool) -> String {
    let asserts: Vec<String> = s
        .asserts
        .iter()
        .map(|(k, v)| format!("\"{k}\":{v}"))
        .collect();
    let stages: Vec<String> = s
        .stages
        .iter()
        .map(|(n, ms)| format!("{{\"name\":\"{n}\",\"cpu_ms\":{ms:.4}}}"))
        .collect();
    let frame_means: Vec<String> = s.frame_means.iter().map(|v| format!("{v:.6}")).collect();
    let dev_field = match device {
        Some(d) => format!(
            "{{\"device_name\":\"{}\",\"steps_before_a\":{},\"steps_before_b\":{},\"pixels_a\":{},\"pixels_b\":{},\"changed_pixels\":{},\"device_pixels_nontrivial\":{},\"device_motion_pixels_changed\":{}}}",
            d.device_name,
            d.steps_before_a,
            d.steps_before_b,
            d.pixels_a,
            d.pixels_b,
            d.changed_pixels,
            d.device_pixels_nontrivial,
            d.device_motion_pixels_changed
        ),
        None => "null".into(),
    };
    format!(
        "{{\"subject\":\"uc08_physics\",\"mode\":\"{}\",\"frames\":{},\"width\":{},\"height\":{},\"internal_width\":{},\"internal_height\":{},\"stages\":[{}],\"asserts\":{{{}}},\"physics\":{{\"steps\":{},\"total_step_ms\":{:.4},\"bodies_seen_last\":{},\"bodies_written_last\":{},\"writes_truncated_total\":{},\"sleep_frame\":{},\"transform_landed_max_err\":{:.8}}},\"streaming\":{{\"insert_frame\":{},\"remove_frame\":{},\"receipt_bodies\":{},\"releases\":{},\"pop_in\":{},\"watched_count\":{}}},\"mv\":{{\"early_max\":{:.8},\"post_sleep_max\":{:.8}}},\"as\":{{\"tlas_rebuilds\":{},\"blas_builds\":{},\"blas_refits\":{}}},\"tracked_count\":{},\"pso_runtime_compile_warnings\":{},\"graph\":{{\"pass_count\":{},\"barrier_count\":{},\"fence_count\":{},\"alias_peak\":{},\"no_alias_peak\":{}}},\"final\":{{\"mean\":{:.6},\"std\":{:.6},\"shadow_lit_ratio\":{:.4}}},\"frame_means\":[{}],\"device\":{},\"device_requested\":{},\"exit_ok\":{}}}",
        s.mode,
        s.frames,
        s.width,
        s.height,
        s.internal_width,
        s.internal_height,
        stages.join(","),
        asserts.join(","),
        s.physics_steps,
        s.physics_total_ms,
        s.bodies_seen_last,
        s.bodies_written_last,
        s.writes_truncated_total,
        opt_u32(s.sleep_frame),
        s.transform_landed_max_err,
        opt_u32(s.insert_frame),
        opt_u32(s.remove_frame),
        s.receipt_bodies,
        s.releases,
        s.streaming_pop_in,
        s.watched_count,
        s.early_mv_max,
        s.post_sleep_mv_max,
        s.tlas_rebuilds,
        s.blas_builds,
        s.blas_refits,
        s.tracked_count,
        s.pso_warnings,
        s.graph_pass_count,
        s.graph_barrier_count,
        s.graph_fence_count,
        s.graph_alias_peak,
        s.graph_no_alias_peak,
        s.final_mean,
        s.final_std,
        s.shadow_lit_ratio,
        frame_means.join(","),
        dev_field,
        device_requested,
        s.all_asserts_pass(device)
    )
}

/// 世界点 → uv(针孔投影;与 compute_camera_mv 的 NDC 约定一致:
/// u = (ndc.x+1)/2,v = (1−ndc.y)/2;w ≤ 0 相机背后 → None)。
fn project_uv(vp: &Mat4, world: [f32; 3]) -> Option<(f32, f32)> {
    let c = vp.transform_vec4([world[0], world[1], world[2], 1.0]);
    if c[3] <= 1e-8 {
        return None;
    }
    Some((0.5 * (c[0] / c[3] + 1.0), 0.5 * (1.0 - c[1] / c[3])))
}

/// 全图 MV 幅值上限(√(du²+dv²) 逐像素最大)。
fn max_mv_magnitude(mv: &ImageF32) -> f32 {
    let mut m = 0.0f32;
    for y in 0..mv.h {
        for x in 0..mv.w {
            let du = mv.get(x, y, 0);
            let dv = mv.get(x, y, 1);
            m = m.max(du * du + dv * dv);
        }
    }
    m.sqrt()
}

fn unproject(inv: &Mat4, x: u32, y: u32, z: f32, w: u32, h: u32) -> [f32; 3] {
    let u = (x as f32 + 0.5) / w as f32;
    let v = (y as f32 + 0.5) / h as f32;
    let ndc = [2.0 * u - 1.0, 1.0 - 2.0 * v, z, 1.0];
    let p = inv.transform_vec4(ndc);
    if p[3].abs() < 1e-8 {
        return [0.0; 3];
    }
    [p[0] / p[3], p[1] / p[3], p[2] / p[3]]
}

fn image_stats(img: &ImageF32) -> (f64, f64) {
    let n = (img.w * img.h) as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }
    let mut mean = 0.0f64;
    for y in 0..img.h {
        for x in 0..img.w {
            mean += f64::from(img.get(x, y, 0) + img.get(x, y, 1) + img.get(x, y, 2)) / 3.0;
        }
    }
    mean /= n;
    let mut var = 0.0f64;
    for y in 0..img.h {
        for x in 0..img.w {
            let l = f64::from(img.get(x, y, 0) + img.get(x, y, 1) + img.get(x, y, 2)) / 3.0;
            var += (l - mean) * (l - mean);
        }
    }
    (mean, (var / n).sqrt())
}

fn shadow_lit_ratio(shadow: &ImageF32) -> f32 {
    let mut lit = 0u32;
    let mut total = 0u32;
    for y in 0..shadow.h {
        for x in 0..shadow.w {
            total += 1;
            if shadow.get(x, y, 0) > 0.5 {
                lit += 1;
            }
        }
    }
    if total == 0 {
        0.0
    } else {
        lit as f32 / total as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_frames_scale_with_run_length() {
        // 默认 96 帧:K=10,M=29(≈ 任务书 K≈10,M≈K+20);短跑 4 帧:K=1,M=2。
        assert_eq!((script_k(96), script_m(96)), (10, 29));
        assert_eq!((script_k(4), script_m(4)), (1, 2));
        assert!(script_m(96) < 96 && script_k(96) < script_m(96));
        assert!(script_m(4) < 4 && script_k(4) < script_m(4));
    }

    #[test]
    fn full_pipeline_runs_and_converges() {
        let scene = crate::scene::build_scene();
        let cfg = RenderConfig {
            out_w: 64,
            out_h: 64,
            frames: 8,
            ..Default::default()
        };
        let mut st = PipelineState::new(&scene, &cfg);
        let mut reports = Vec::new();
        for f in 0..cfg.frames {
            reports.push(run_frame(&scene, &mut st, &cfg, f));
        }
        let last = reports.last().unwrap();
        assert!(last.hdr_mean > 0.0 && last.hdr_std > 0.0, "最终帧非平凡");
        assert!(st.pso.cache.warnings() == 0);
        assert!(!st.compiled.fences().is_empty());
        assert!(st.compiled.pool().high_water() < st.compiled.pool().no_alias_peak());
        // 物理面:8 步全计时、TLAS 重建发生、BLAS 零 refit、变换落地。
        assert_eq!(st.physics_steps, 8);
        assert!(st.physics_total_ms > 0.0);
        assert!(st.blas_cache.stats().tlas_rebuilds >= 1);
        assert_eq!(st.blas_cache.stats().refits, 0);
        assert!(st.transform_landed_max_err <= 1e-4);
        // 流送剧本(8 帧:K=2,M=3):批插/卸载/receipt 全沿可见。
        assert!(st.insert_frame.is_some());
        assert!(st.remove_frame.is_some() && st.receipt_bodies >= 1);
        assert_eq!(st.page_cache.releases, 1);
        assert_eq!(st.page_cache.releases, st.page_cache.receipts_consumed);
        // 下落早期 MV 非零(动态立方体覆盖区)。
        assert!(st.early_mv_max > 5e-4, "early_mv_max={}", st.early_mv_max);
    }

    #[test]
    fn summary_json_is_single_line_and_parseable() {
        let scene = crate::scene::build_scene();
        let cfg = RenderConfig {
            out_w: 32,
            out_h: 32,
            frames: 4,
            ..Default::default()
        };
        let mut st = PipelineState::new(&scene, &cfg);
        let mut reports = Vec::new();
        for f in 0..cfg.frames {
            reports.push(run_frame(&scene, &mut st, &cfg, f));
        }
        let mut s = assemble_summary(&st, &reports, false).unwrap();
        s.width = cfg.out_w;
        s.height = cfg.out_h;
        s.internal_width = cfg.internal_w();
        s.internal_height = cfg.internal_h();
        let json = summary_json(&s, None, false);
        assert!(!json.contains('\n'), "单行 JSON");
        assert!(json.starts_with('{') && json.ends_with('}'));
        assert!(json.contains("\"subject\":\"uc08_physics\""));
        assert!(json.contains("\"pso_runtime_compile_warnings\":0"));
    }
}
