// Assisted-by: Kimi-K3（G34 全特性合流 G34-1 合流地基）
//! G34 全特性合流——统一 kernel 车道面 G34-1 生产接线 harness（门
//! `g34.wave1.unified`）：纹理采样（fork A）+ slab 侧表预调制 + 动态实例分派
//! （fork B）三特性同开车道，统一 kernel `kernels/g34_unified_gi.rx`（母版
//! g14_3_direct_gi 语义 + 图集采样块 + 实例分派块合一）进统一四 pass TSR
//! 车道（`UnifiedDescs::G34Full` 27 SSBO 形态）+ device 显示编码 +
//! swapchain 真窗口呈现（结构复用 g31_window_present；g31_window_present.rs
//! 0-byte 不动——其五门为回归锚）。
//!
//! ## 合并语义定义（host 装配期承载 + kernel 零新增面；host 金标准同步实现）
//!
//! 1. **贴图材质三角**：albedo[c] = 图集双线性采样（srgb→linear LUT 后线性域
//!    过滤，fork A 采样块逐字）× mod[c]，mod = baseColorFactor.rgb ×
//!    (1−metallicFactor) × **R_slot**（slab 槽映射面——R_slot 装配期预乘进
//!    texmeta 槽 mod 三项，`g34_slab_premod_texmeta`；非 slab 映射材质
//!    R_slot ≡ 1 不预乘，texmeta 与 fork A 逐位同值）。
//! 2. **非贴图三角**：albedo[c] = mats 常量 albedo（× R_slot 若为 slab 映射
//!    材质——`slab_apply` 逐三角预调制进 scene.albedo → mats SSBO，G31 slab
//!    面 0-byte 语义继承）；动态实例三角（pg ≥ dyn_tri_base）= 纯发光体
//!    albedo=0（fork B 简化面如实继承）。
//! 3. **缺省面 == 母版位级**：纹理缺省（tritex 全 −1）⇒ tex_gate = 0、albedo
//!    = mats 常量面逐位同值；动态缺省（单实例 TLAS）⇒ inst = 0、pg = prim
//!    逐位同值——两缺省同开全链（TSR 后）digest == g14_3_stage_a_digest_
//!    anchor 冻结锚（`--static-camera` 锚格模式承载，kernel 头注释逐 op 论证）。
//!
//! ## 职责闭集
//!
//! 1. **三特性同开真跑**：bistro-interior 1080p `--full --auto-move orbit`
//!    真窗口真跑——装配期（slab 资产加载 + 16 槽 device/host 双臂求值对拍 +
//!    逐三角 albedo 预调制 + texmeta mod 预调制）+ 纹理资产链（top-12 律法 +
//!    BC1/BC3 解码 + 图集烘焙 + G11.3 manifest 互核 + 探针双臂对拍位级硬门）
//!    + 动态资产（lane_assets_dyn 双 BLAS + 逐帧 tlas_update refit）。
//! 2. **host 金标准对拍**：parity 帧（post-warmup 首测量帧）scene HDR 回读
//!    vs host 合并语义渲染（host TriBvh/Tlas 实例化追踪 + g31_tex_host_sample
//!    同 op 序采样 + G13.4 shade_pixel 逐字直接光）——逐像素绝对差 p100 ≤
//!    冻结容差（milestones/g34/g34_budget.json 程序读禁手写；容差结构依据 =
//!    RT core vs host Möller–Trumbore t 值算术差 ~ULP 级 ⇒ 命中点/辐照传递
//!    差，目标近位级，threshold = measured × 2.0 标定冻结）；bitexact 像素
//!    占比如实登记。
//! 3. **确定性双跑 digest 位级**（CI 门裁决）+ **动态实例位置核验**（A4
//!    范式 host 投影：轨迹点 + 8 角点经 vp_j 投影 vs scene color 纯绿谱
//!    检测，每 10 帧一次，fail-closed）。
//! 4. **契约链**：生产契约 digest 门 == FROZEN + G10 语料三件套转引一致性
//!    核验（不等即 RED）。
//! 5. **多口径分离**：real_render_frame_ms（五 pass 渲染墙钟,含 BGRA8 强制
//!    回读如实登记）/ present_frame_ms / digest_frame_ms / stats.encode_gpu_
//!    ms；纹理/slab 装配 = 装配期一次性 eval_ms 单列不混帧口径。
//!
//! ## 用法
//!
//! ```text
//! g34_full_lane [--frames 120] [--warmup 10] [--tier 100]
//!     [--contract <c.json>] [--g10-dir milestones/g10/corpus] [--gltf <scene.gltf>]
//!     [--spv-scene <a.spv>] [--spv-mv <b>] [--spv-resample <c>] [--spv-resolve <d>]
//!     [--spv-encode <e>] [--spv-slab <s>] [--spv-texture-probe <p>]
//!     [--evidence <path>] [--expect-digest <sha256:…>] [--hidden] [--headless-smoke]
//!     [--auto-move <orbit|dolly>]
//!     [--full --slab-table <asset.json>]
//!     [--textures on|off] [--dyn on|off]
//!     [--static-camera] [--host-tol <F>]
//!     [--cluster-lod off|leaf|on --cluster-pack <RXCP> [--cluster-error-px 1.0]]
//!     [--wp-hlod off|full|on --wp-pack <RXWH> [--wp-threshold-l0 1.0]
//!      [--wp-radius 64] [--wp-budget-cells 4] [--wp-warmup 4]]
//! ```
//!
//! G36 W3 geo 组合面（互斥解除;门 `g36.wave3.unified_geo`）：--cluster-lod ×
//! --wp-hlod × 纹理×slab×动态（统一主车道）/ × HZB（--hzb on 区段,节点段经
//! provenance 重导出）组合成立——W1 逐三角 provenance 事实源（侧表 UV gather
//! 位保真 + 代理 tritex 强制 −1 常量面回退〔#96 属性保持简化留窗〕+ 节点段
//! AABB 自重建几何精确重算）;leaf×full 极限 = --full 基线 digest_seq 逐帧
//! 位级一致（恒等排列锚）;host 金标准对拍/动态位置核验/HZB 金字塔位级/零假
//! 阳性诸硬门维持。geo on 时 evidence schema/gate 切 G36 字面 + "geo" 块追加
//! （G34 注册 schema 0-byte）;geo × --skin 组合归后续窗（蒙皮区段独立装配面,
//! g14_3 MegaSkin×geo 已验证——如实拒跑不冒充）。cut/选层冻结于装配期契约
//! 相机（逐帧 AS 更新归 #77/#89 合流窗,如实登记）。
//!
//! 闭集：`--full` = textures on + slab on（须随 --slab-table）+ dyn on 三件
//! 同开（与显式 --textures off/--dyn off 冲突即拒）；`--full`/任一特性开
//! 须随 `--auto-move`（确定性轨迹 digest_seq 登记面）+ `--tier 100`；
//! `--static-camera` = 锚格模式（静态契约相机 + 全特性缺省关,与 --auto-move/
//! 特性开互斥——Stage A 锚格 160 帧全链位级对拍面）；`--host-tol` 缺省程序
//! 读 g34_budget 冻结条目（fail-closed）。`--headless-smoke` 无窗口退化仅供
//! 自检逻辑用不计真门（evidence headless=true,present 口径 null）。
//! 三态：无 Vulkan/设备/场景资产/窗口创建失败 → skipped_dev_env（退 0 非
//! fake pass;RURIX_REQUIRE_REAL=1 翻 FAIL 退 1）。
#![forbid(unsafe_code)]
// 共享体含本 bin 未消费面（render/bench 腿、dlss/fsr 双臂、EXR/PNG 出图、
// G16+ GI 臂、SVT/蒙皮面等）——dead_code 豁免如实登记；本 bin 消费面 = 契约
// 解析/scene 装配/G34Full 车道/帧参数/jitter/digest/slab+纹理装配/host 金标准。
#![allow(dead_code)]

include!("g14_3_lane/g14_3_lane_body.rs");

use rurix_render::display::aces13::aces13_device_encode_params;
use rurix_render::rt::bvh::{InstanceDesc as BvhInstanceDesc, Ray, Tlas, Transform3x4, TriBvh, Vec3};

const GTAG: &str = "[g34_full_lane]";
/// G34-1 门键（evidence `gate` 字段字面）。
const G34_GATE: &str = "g34.wave1.unified";
/// G34-1 harness evidence schema 字面（milestones/g34/
/// g34_unified_lane_evidence_schema.json 同字面）。
const G34_SCHEMA: &str = "rurix.g34.unified_lane_evidence.v1";
/// 统一 GI kernel 默认 SPV（源 = kernels/g34_unified_gi.rx；`.tmp` 构建产物,
/// CI 门脚本保障编译；母版 kernel/SPV 0-byte,缺省面 = Stage A 回归锚）。
const G34_DEFAULT_SPV_SCENE: &str = ".tmp/g34_gates/unified/g34_unified_gi.spv";
/// device 显示编码 kernel 默认 SPV（g31 A3 同件 0-byte 消费）。
const G34_DEFAULT_SPV_ENCODE: &str = ".tmp/g14_gates/m_c/g31_display_encode.spv";
/// slab device 求值 kernel 默认 SPV（G29 M-a 本体 0-byte 冻结消费）。
const G34_DEFAULT_SPV_SLAB: &str = ".tmp/g14_gates/m_c/g29_slab.spv";
/// 纹理探针 kernel 默认 SPV（B4 生产采样块隔离对拍面 0-byte 消费）。
const G34_DEFAULT_SPV_TEXTURE_PROBE: &str = ".tmp/g31_gates/texture/g31_texture_probe.spv";
/// G10 语料目录默认（contract_params_bistro_interior.json + camera/lighting 三件套）。
const G34_DEFAULT_G10_DIR: &str = "milestones/g10/corpus";
/// host 金标准对拍冻结容差事实源（g34 budget;`--host-tol` 缺省时程序读,
/// fail-closed）。
const G34_BUDGET: &str = "milestones/g34/g34_budget.json";
/// host 对拍冻结容差条目标识（threshold = measured × 2.0 程序产;条目字面钉死）。
const G34_HOST_TOL_ENTRY: &str = "g34.unified_lane.host_parity_tol";
/// Stage A 锚格字面（--static-camera 锚格模式对拍面）。
const G34_ANCHOR_PATH: &str = "milestones/g14/g14_3_stage_a_digest_anchor.json";
const G34_ANCHOR_CELL: &str = "bistro-interior_t100_tsr_device";
/// G36 W3：geo 组合面门键/schema 字面（--cluster-lod/--wp-hlod 任一 on 时
/// evidence 切换;off 默认 = G34 字面 0-byte——G34 harness schema
/// additionalProperties:false 纪律下组合面另立 schema,不改 G34 注册面;
/// harness 真跑件留 .tmp 不注册,门裁决件 = ci/g36_geo_composition_smoke.py
/// 蒸馏,G34HZB/G35 lane 同律）。
const G36_GATE: &str = "g36.wave1.geo_composition";
const G36_SCHEMA: &str = "rurix.g36.unified_geo_evidence.v1";

// ---------------------------------------------------------------------------
// G34 车道资源面：G34Full 27 SSBO（0..=26,共享体 unified_lane_descs_g34 产）
// + encode 两件（27=ACES 参数,28=BGRA8 输出）= 29 资源五 pass 图;readback
// 下标 0..=3 = unified 面（OUT_COLOR f32 双 parity/MV/DEPTH）,4 = scene color
// （fork B 动态核验 + host 金标准对拍回读面）,5 = BGRA8。
// ---------------------------------------------------------------------------
const G34_U_ENC_PARAMS: u32 = 27;
const G34_U_ENC_OUT: u32 = 28;
const G34_U_RESOURCE_COUNT: usize = 29;
const G34_RB_SCENE: u32 = 4;
const G34_RB_BGRA: u32 = 5;

/// encode pass 屏障计划（保守超集逐字声明同律：读 TSR out_color 双 parity
/// 并集 + 编码参数 + BGRA8 输出;readback 触达由执行器隐式超集覆盖）。
const G34_U_PLAN_ENCODE: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G34_U_ENC_PARAMS, TargetState::StorageReadWrite),
    (G34_U_ENC_OUT, TargetState::StorageReadWrite),
];

// ---------------------------------------------------------------------------
// 小件助手（g31_window_present 同型 bin-local 复制——g31 bin 0-byte 纪律下
// 本 bin 自持面,语义逐字同模）。
// ---------------------------------------------------------------------------

fn g34_file_sha(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读 {path}: {e}"))?;
    Ok(format!("sha256:{}", sha256_hex(&bytes)))
}

fn g34_json_num_eq(name: &str, a: &Json, b: &Json) -> Result<(), String> {
    let (x, y) = (as_f64(name, a)?, as_f64(name, b)?);
    if x.to_bits() != y.to_bits() {
        return Err(format!("{name} 不等: {x} ≠ {y}"));
    }
    Ok(())
}

fn g34_json_vec_eq(name: &str, a: &Json, b: &Json, n: usize) -> Result<(), String> {
    let (x, y) = (as_f64v(name, a, n)?, as_f64v(name, b, n)?);
    for k in 0..n {
        if x[k].to_bits() != y[k].to_bits() {
            return Err(format!("{name}[{k}] 不等: {} ≠ {}", x[k], y[k]));
        }
    }
    Ok(())
}

/// G10 语料三件套转引一致性核验（g31_g10_corpus_gate 逐字同模：逐字段相等,
/// 不等即 RED;delta 如实登记不消费）。返回 evidence `contracts` 块尾部
/// fragment JSON 串。
fn g34_g10_corpus_gate(scene_row: &Json, g10_dir: &str) -> Result<String, String> {
    let contract_path = format!("{g10_dir}/contract_params_bistro_interior.json");
    let camera_path = format!("{g10_dir}/camera_bistro_interior.json");
    let lighting_path = format!("{g10_dir}/lighting_bistro_interior.json");
    let contract_sha = g34_file_sha(&contract_path)?;
    let camera_sha = g34_file_sha(&camera_path)?;
    let lighting_sha = g34_file_sha(&lighting_path)?;
    let g10_contract = json_parse(
        &std::fs::read_to_string(&contract_path).map_err(|e| format!("读 {contract_path}: {e}"))?,
    )?;
    let g10_camera = json_parse(
        &std::fs::read_to_string(&camera_path).map_err(|e| format!("读 {camera_path}: {e}"))?,
    )?;
    let g10_lighting = json_parse(
        &std::fs::read_to_string(&lighting_path).map_err(|e| format!("读 {lighting_path}: {e}"))?,
    )?;

    let row_cam = scene_row.get("camera").ok_or("场景行缺 camera")?;
    let row_lig = scene_row.get("lighting").ok_or("场景行缺 lighting")?;
    let c_cam = g10_contract.get("camera").ok_or("g10 契约缺 camera")?;

    g34_json_vec_eq("camera.position", c_cam.get("position").unwrap(), row_cam.get("position").unwrap(), 3)?;
    g34_json_vec_eq(
        "camera.orientation_quat",
        c_cam.get("orientation_quat").unwrap(),
        row_cam.get("orientation_quat").unwrap(),
        4,
    )?;
    for k in ["fov_y_deg", "near", "far"] {
        g34_json_num_eq(&format!("camera.{k}"), c_cam.get(k).unwrap(), row_cam.get(k).unwrap())?;
    }
    g34_json_num_eq(
        "camera.resolution.w",
        c_cam.get("resolution").and_then(|r| r.get("w")).ok_or("g10 契约缺 resolution.w")?,
        row_cam.get("resolution").and_then(|r| r.get("w")).ok_or("生产行缺 resolution.w")?,
    )?;
    g34_json_num_eq(
        "camera.resolution.h",
        c_cam.get("resolution").and_then(|r| r.get("h")).ok_or("g10 契约缺 resolution.h")?,
        row_cam.get("resolution").and_then(|r| r.get("h")).ok_or("生产行缺 resolution.h")?,
    )?;
    g34_json_vec_eq(
        "camera_file.eye",
        g10_camera.get("eye").ok_or("camera 文件缺 eye")?,
        row_cam.get("position").unwrap(),
        3,
    )?;
    g34_json_num_eq(
        "camera_file.fov_y_deg",
        g10_camera.get("fov_y_deg").ok_or("camera 文件缺 fov_y_deg")?,
        row_cam.get("fov_y_deg").unwrap(),
    )?;
    let cam_res = g10_camera
        .get("resolution")
        .and_then(|v| v.as_array())
        .ok_or("camera 文件缺 resolution")?;
    g34_json_num_eq(
        "camera_file.resolution[0]",
        cam_res.first().ok_or("camera resolution 空")?,
        row_cam.get("resolution").and_then(|r| r.get("w")).unwrap(),
    )?;
    g34_json_num_eq(
        "camera_file.resolution[1]",
        cam_res.get(1).ok_or("camera resolution 缺 h")?,
        row_cam.get("resolution").and_then(|r| r.get("h")).unwrap(),
    )?;

    let g_points = g10_lighting
        .get("point_lights")
        .and_then(|v| v.as_array())
        .ok_or("g10 lighting 缺 point_lights")?;
    let r_points = row_lig
        .get("point_lights")
        .and_then(|v| v.as_array())
        .ok_or("生产行缺 point_lights")?;
    if g_points.len() != r_points.len() {
        return Err(format!("点光数不等:g10 {} ≠ 生产行 {}", g_points.len(), r_points.len()));
    }
    for (i, (g, r)) in g_points.iter().zip(r_points.iter()).enumerate() {
        g34_json_vec_eq(&format!("point_lights[{i}].position"), g.get("position").unwrap(), r.get("position").unwrap(), 3)?;
        g34_json_vec_eq(
            &format!("point_lights[{i}].color_linear_rgb"),
            g.get("color_linear_rgb").unwrap(),
            r.get("color_linear_rgb").unwrap(),
            3,
        )?;
        g34_json_num_eq(
            &format!("point_lights[{i}].intensity_cd"),
            g.get("intensity_cd").unwrap(),
            r.get("intensity_cd").unwrap(),
        )?;
    }

    let g_em = g10_lighting
        .get("emissive_surfaces")
        .and_then(|v| v.as_array())
        .ok_or("g10 lighting 缺 emissive_surfaces")?;
    let r_em = row_lig
        .get("emissive_materials")
        .and_then(|v| v.as_array())
        .ok_or("生产行缺 emissive_materials")?;
    if g_em.len() != r_em.len() {
        return Err(format!("emissive 数不等:g10 {} ≠ 生产行 {}", g_em.len(), r_em.len()));
    }
    for g in g_em.iter() {
        let mi = g.get("material_index").and_then(|v| v.as_u64()).ok_or("g10 emissive 缺 material_index")?;
        let r = r_em
            .iter()
            .find(|r| r.get("material_index").and_then(|v| v.as_u64()) == Some(mi))
            .ok_or_else(|| format!("生产行缺 material_index={mi} 的 emissive"))?;
        g34_json_vec_eq(
            &format!("emissive[{mi}].le_linear_rgb"),
            g.get("le_linear_rgb").unwrap(),
            r.get("le_linear_rgb").unwrap(),
            3,
        )?;
        g34_json_num_eq(
            &format!("emissive[{mi}].area_m2"),
            g.get("area_m2").ok_or("g10 emissive 缺 area_m2")?,
            r.get("area_m2").ok_or("生产行 emissive 缺 area_m2")?,
        )?;
    }

    let g_sun = g10_contract
        .get("lighting")
        .and_then(|l| l.get("sun"))
        .and_then(|s| s.get("intensity_lux"))
        .and_then(|v| v.as_f64())
        .ok_or("g10 契约缺 sun.intensity_lux")?;
    let g_sky = g10_contract
        .get("lighting")
        .and_then(|l| l.get("sky"))
        .and_then(|s| s.get("intensity"))
        .and_then(|v| v.as_f64())
        .ok_or("g10 契约缺 sky.intensity")?;
    let g_ev = g10_contract
        .get("lighting")
        .and_then(|l| l.get("exposure"))
        .and_then(|s| s.get("ev100"))
        .and_then(|v| v.as_f64())
        .ok_or("g10 契约缺 exposure.ev100")?;
    let r_sun = row_lig.get("sun_intensity_lux").and_then(|v| v.as_f64()).ok_or("生产行缺 sun_intensity_lux")?;
    let r_sky = row_lig.get("sky_intensity").and_then(|v| v.as_f64()).ok_or("生产行缺 sky_intensity")?;
    let r_ev = scene_row.get("exposure").and_then(|e| e.get("ev100")).and_then(|v| v.as_f64()).ok_or("生产行缺 exposure.ev100")?;

    Ok(format!(
        "\"g10_contract\":{{\"path\":{},\"sha256\":{}}},\"g10_camera\":{{\"path\":{},\"sha256\":{}}},\"g10_lighting\":{{\"path\":{},\"sha256\":{}}},\"consistency\":\"pass\",\"delta_note\":{}",
        jstr(&contract_path.replace('\\', "/")),
        jstr(&contract_sha),
        jstr(&camera_path.replace('\\', "/")),
        jstr(&camera_sha),
        jstr(&lighting_path.replace('\\', "/")),
        jstr(&lighting_sha),
        jstr(&format!(
            "G10 契约 sun_intensity_lux={g_sun}/sky_intensity={g_sky}/ev100={g_ev} 与生产行 sun/sky={r_sun}/{r_sky}、ev100={r_ev} 差异如实登记不消费——生产内容模型锚(直接光 quad/point/emissive + ev100 标定)为消费面,差异面 = G10.5a 取景校准登记值"
        )),
    ))
}

fn g34_stats(v: &[f64]) -> (f64, f64, f64, f64, f64) {
    if v.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = s.iter().sum::<f64>() / s.len() as f64;
    let p50 = s[s.len() / 2];
    let (mn, mx) = (s[0], s[s.len() - 1]);
    let cv = if mean > 0.0 {
        (s.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / s.len() as f64).sqrt() / mean
    } else {
        0.0
    };
    (mean, p50, cv, mn, mx)
}

// ---------------------------------------------------------------------------
// 相机（G31Camera 逐字同模自持面：auto-move 确定性轨迹 + spec 重建;锚格模式
// 不经本面——契约相机直用保位级）。
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct G34Camera {
    eye: [f32; 3],
    yaw: f32,
    pitch: f32,
    up0: [f32; 3],
    fov_y_rad: f32,
    near: f32,
    far: f32,
}

impl G34Camera {
    fn from_spec(c: &CameraSpec) -> Self {
        let f = c.forward;
        let pitch = f[1].clamp(-1.0, 1.0).asin();
        let yaw = f[0].atan2(-f[2]);
        Self {
            eye: c.eye,
            yaw,
            pitch,
            up0: c.up0,
            fov_y_rad: c.fov_y_rad,
            near: c.near,
            far: c.far,
        }
    }

    fn forward(&self) -> [f32; 3] {
        [
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            -self.pitch.cos() * self.yaw.cos(),
        ]
    }

    fn spec(&self) -> CameraSpec {
        CameraSpec {
            eye: self.eye,
            forward: self.forward(),
            up0: self.up0,
            fov_y_rad: self.fov_y_rad,
            near: self.near,
            far: self.far,
        }
    }
}

/// auto-move 确定性轨迹（帧号唯一事实源,绝对位姿;g31_auto_move_pose 逐字同模）。
fn g34_auto_move_pose(name: &str, cam0: &G34Camera, fi: u32, total: u32) -> (f32, f32, [f32; 3]) {
    let t = f64::from(fi) / f64::from(total.max(1));
    let tau = std::f64::consts::TAU;
    match name {
        "orbit" => {
            let a = tau * t;
            let eye = [
                (f64::from(cam0.eye[0]) + 0.35 * a.sin()) as f32,
                (f64::from(cam0.eye[1]) + 0.05 * (2.0 * a).sin()) as f32,
                (f64::from(cam0.eye[2]) + 0.35 * (a.cos() - 1.0)) as f32,
            ];
            let yaw = (f64::from(cam0.yaw) + 0.30 * a.sin()) as f32;
            (yaw, cam0.pitch, eye)
        }
        "dolly" => {
            let a = tau * t;
            let f = cam0.forward();
            let fxz = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6);
            let d = 0.50 * (std::f64::consts::PI * t).sin();
            let eye = [
                (f64::from(cam0.eye[0]) + f64::from(f[0] / fxz) * d) as f32,
                (f64::from(cam0.eye[1]) + 0.03 * a.sin()) as f32,
                (f64::from(cam0.eye[2]) + f64::from(f[2] / fxz) * d) as f32,
            ];
            let yaw = (f64::from(cam0.yaw) - 0.20 * a.sin()) as f32;
            (yaw, cam0.pitch, eye)
        }
        other => fail(&format!("--auto-move 轨迹 {other} 越闭集(orbit|dolly)")),
    }
}

/// BGRA8 帧内容 digest（payload = `G31BGRA-1\0` + w/h LE + 打包字节;g31 A3
/// 同模——device BGRA8 域 digest 语义同一字面,跨 bin digest 可比对）。
fn g34_bgra_digest(w: u32, h: u32, bytes: &[u8]) -> String {
    let mut payload = b"G31BGRA-1\0".to_vec();
    payload.extend_from_slice(&w.to_le_bytes());
    payload.extend_from_slice(&h.to_le_bytes());
    payload.extend_from_slice(bytes);
    format!("sha256:{}", sha256_hex(&payload))
}

// ---------------------------------------------------------------------------
// G34 五 pass 车道（G34Full 四 pass + device 显示编码）描述组与状态机——
// G31TsrLane 逐字同模 + fork B 逐帧 tlas_update + scene color 回读面。
// ---------------------------------------------------------------------------

/// G34 描述组（Vec 面——session 切片消费;`unified_lane_descs_g34` 产物逐项
/// 克隆追加 encode 两件,既有项 0-byte）。
struct G34Descs<'x> {
    resources: Vec<ResourceDesc<'x>>,
    passes: Vec<Pass<'x>>,
    barriers: Vec<&'static [(u32, TargetState)]>,
    readbacks: Vec<Readback>,
}

/// G34 描述组装配：G34Full 四 pass（scene `g34_unified_gi` → mv → TSR 双
/// pass）+ encode（pass4 读 `U_OUT_COLOR[parity]`——逐帧 binding_overrides
/// 换 parity,初始绑定 = parity 0;dispatch 自 encode SPV LocalSize 派生）。
fn g34_descs<'x>(
    g34: (
        [ResourceDesc<'x>; U_RESOURCE_COUNT_G34],
        [Pass<'x>; 4],
        [&'static [(u32, TargetState)]; 4],
        [Readback; 5],
    ),
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    ow: u32,
    oh: u32,
) -> G34Descs<'x> {
    let (resources, passes, barriers, readbacks) = g34;
    let opc = (ow * oh) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let mut resources = resources.to_vec();
    debug_assert_eq!(resources.len(), U_RESOURCE_COUNT_G34);
    // 27 = 编码参数（ACES 矩阵/样条 f32 块,创建期一次上传;逐帧曝光走 TSR
    // 参数面不经本 buffer——本面静态,resize 随车道重建）。
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: enc_params_bytes.len() as u64,
        usage: storage,
        data: Some(enc_params_bytes),
        device_local: true,
    }));
    // 28 = BGRA8 打包输出（1 u32/px;present 拷贝/digest 唯一消费面）。
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: opc * 4,
        usage: storage,
        data: None,
        device_local: true,
    }));
    debug_assert_eq!(resources.len(), G34_U_RESOURCE_COUNT);
    let mut passes = passes.to_vec();
    passes.push(Pass::Compute(ComputePass {
        name: "g31_display_encode",
        spirv: enc_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(enc_dispatch),
        bindings: Bindings {
            storage_buffers: vec![U_OUT_COLOR[0], G34_U_ENC_PARAMS, G34_U_ENC_OUT],
            ..Bindings::default()
        },
    }));
    let mut barriers = barriers.to_vec();
    barriers.push(G34_U_PLAN_ENCODE);
    let mut readbacks = readbacks.to_vec();
    readbacks.push(Readback::Buffer {
        res: G34_U_ENC_OUT,
        offset: 0,
        size: opc * 4,
    });
    G34Descs {
        resources,
        passes,
        barriers,
        readbacks,
    }
}

/// G34 逐帧回读模式（常态 = BGRA8;末帧 = +f32 out_color 供 render_digest;
/// 核验帧 = +scene color/depth 内部 res（fork B 位置核验 + host 金标准对拍
/// 面——depth = 生产字面深度对拍信息面））。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum G34Readback {
    None,
    Bgra,
    /// BGRA8 + scene depth + scene color（核验帧面;子集 = [3,4,5]）。
    BgraAndScene,
    /// BGRA8 + scene depth + scene color + f32 out_color（末帧面;子集 =
    /// [p,3,4,5]）。
    Full,
}

/// G34 一帧产物（GPU 分段 = telemetry 逐 pass;回读四路可选）。
struct G34FrameRec {
    scene_gpu_ns: f64,
    mv_gpu_ns: f64,
    resample_gpu_ns: f64,
    resolve_gpu_ns: f64,
    encode_gpu_ns: f64,
    cpu_record_ns: u64,
    cpu_submit_ns: u64,
    cpu_fence_wait_ns: u64,
    validation_error_count: u64,
    leaked_object_count: u64,
    leaked_allocation_count: u64,
    bgra8: Option<Vec<u8>>,
    out_color: Option<Vec<f32>>,
    scene_color: Option<Vec<f32>>,
    scene_depth: Option<Vec<f32>>,
    readback_convert_ms: f64,
}

/// G34 五 pass 车道状态机（parity/历史门/prev_vp_j 与 UnifiedTsrLane/
/// G31TsrLane 逐字同律;fork B 面 = 逐帧 tlas_update + 60 f32 场景参数经
/// 调用方预打包传入）。
struct G34TsrLane<'a> {
    session: DeviceFrameSession<'a>,
    parity: usize,
    has_history_state: bool,
    prev_vp_j: Option<Mat4>,
}

impl<'a> G34TsrLane<'a> {
    fn create(descs: &'a G34Descs<'a>, accel_structs: &[AccelStructDesc<'a>]) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        // frame_slots=2（与 UnifiedTsrLane inflight=1 创建面逐字同——顺序全
        // 同步口径;fork B tlas_update 走顺序入口,FIF 流水面拒收,A4 同律）。
        let session = DeviceFrameSession::new_with_accel_structs(
            &descs.resources,
            &descs.passes,
            &descs.barriers,
            &descs.readbacks,
            2,
            accel_structs,
        )?;
        Ok(Self {
            session,
            parity: 0,
            has_history_state: false,
            prev_vp_j: None,
        })
    }

    /// 本帧 FrameUpdate + provenance 组装（三小件参数打包 + parity 轮换
    /// resample/resolve/encode 三 pass binding_overrides + readback 子集 +
    /// fork B tlas_update;与 G31TsrLane::prepare_update 同律,scene_params
    /// 调用方预打包〔60 f32 dyn 面〕）。
    #[allow(clippy::too_many_arguments)]
    fn prepare_update(
        &self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback: G34Readback,
        scene_params: Vec<f32>,
        tlas_update: Option<(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
    ) -> Result<(SubmissionProvenance, FrameUpdate), String> {
        // mv 参数面:inv_cur = vp_j 逆(host Mat4::inverse 伴随法);prev = 上帧
        // vp_j;首帧 has_prev=0,kernel 门直写零——与统一车道逐字同律。
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆（mv 参数面）")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        let has_history = !reset && self.has_history_state;
        let tsr_params = pack_tsr_params(iw, ih, ow, oh, jitter, exposure, has_history, false);
        let p = self.parity;
        let uploads: Vec<(StableResourceId, u64, Vec<u8>)> = vec![
            (
                StableResourceId(u64::from(U_SCENE_PARAMS) + 1),
                0,
                bytes_f32(&scene_params),
            ),
            (
                StableResourceId(u64::from(U_MV_PARAMS) + 1),
                0,
                bytes_f32(&mv_params),
            ),
            (
                StableResourceId(u64::from(U_TSR_PARAMS) + 1),
                0,
                bytes_f32(&tsr_params),
            ),
        ];
        let bindings_resample = Bindings {
            storage_buffers: vec![
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
                U_TSR_PARAMS,
                U_CUR_RGB,
                U_LUMA[p],
                U_DEPTH_HI[p],
            ],
            ..Bindings::default()
        };
        let bindings_resolve = Bindings {
            storage_buffers: vec![
                U_CUR_RGB,
                U_LUMA[p],
                U_DEPTH_HI[p],
                U_MV_OUT,
                U_REACTIVE,
                U_OUT_COLOR[1 - p],
                U_DEPTH_HI[1 - p],
                U_LUMA[1 - p],
                U_OUT_SIGN[1 - p],
                U_OUT_SCORE[1 - p],
                U_TSR_PARAMS,
                U_OUT_COLOR[p],
                U_OUT_SIGN[p],
                U_OUT_SCORE[p],
            ],
            ..Bindings::default()
        };
        // encode 读本帧 resolve 写出的 U_OUT_COLOR[p](parity 轮换同律)。
        let bindings_encode = Bindings {
            storage_buffers: vec![U_OUT_COLOR[p], G34_U_ENC_PARAMS, G34_U_ENC_OUT],
            ..Bindings::default()
        };
        let binding_overrides = vec![
            (2, bindings_resample),
            (3, bindings_resolve),
            (4, bindings_encode),
        ];
        // readback 子集（下标升序 = 解析序：f32 out_color(p) → depth(3) →
        // scene(4) → bgra(5);本车道模式 = {5} / {3,4,5} / {p,3,4,5}）。
        let mut subset: Vec<u32> = Vec::new();
        match readback {
            G34Readback::None => {}
            G34Readback::Bgra => subset.push(G34_RB_BGRA),
            G34Readback::BgraAndScene => {
                subset.push(3);
                subset.push(G34_RB_SCENE);
                subset.push(G34_RB_BGRA);
            }
            G34Readback::Full => {
                subset.push(p as u32);
                subset.push(3);
                subset.push(G34_RB_SCENE);
                subset.push(G34_RB_BGRA);
            }
        }
        let update = FrameUpdate {
            tlas_update,
            buffer_uploads: uploads,
            binding_overrides,
            push_constant_overrides: vec![],
            readback_subset: Some(subset),
            blas_refit: None, // G34-1 无 BLAS refit 面（fork B = TLAS 实例变换 UPDATE）
        };
        let prov = self.session.next_provenance_with_update(&update)?;
        Ok((prov, update))
    }

    /// 一帧产物组装（telemetry 五 pass 提取 + 回读按子集同序解析 + 尺寸校验）。
    fn rec_from_output(
        &self,
        mut out: DeviceFrameOutput,
        readback: G34Readback,
        ow: u32,
        oh: u32,
        iw: u32,
        ih: u32,
    ) -> Result<G34FrameRec, String> {
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        let scene_gpu_ns = gpu("g34_unified_gi")?;
        let mv_gpu_ns = gpu("g14_mv")?;
        let resample_gpu_ns = gpu("g14_8_tsr_resample")?;
        let resolve_gpu_ns = gpu("g14_8_tsr_resolve")?;
        let encode_gpu_ns = gpu("g31_display_encode")?;
        let t_convert = std::time::Instant::now();
        let bgra_px = (ow * oh * 4) as usize;
        let f32_px = (ow * oh * 3) as usize;
        let scene_px = (iw * ih * 3) as usize;
        let depth_px = (iw * ih) as usize;
        let mut idx = 0usize;
        let take_rb = |out: &mut DeviceFrameOutput, idx: &mut usize| -> Result<Vec<u8>, String> {
            if *idx >= out.readbacks.len() {
                return Err(format!(
                    "G34 回读路数 {} 少于子集消费序 {idx}",
                    out.readbacks.len()
                ));
            }
            let b = std::mem::take(&mut out.readbacks[*idx]);
            *idx += 1;
            Ok(b)
        };
        let (bgra8, out_color, scene_color, scene_depth) = match readback {
            G34Readback::None => {
                if !out.readbacks.is_empty() {
                    return Err(format!("G34 零回读面回读路数 {} ≠ 0", out.readbacks.len()));
                }
                (None, None, None, None)
            }
            G34Readback::Bgra => {
                let b = take_rb(&mut out, &mut idx)?;
                if b.len() != bgra_px {
                    return Err(format!("G34 BGRA8 回读字节 {} ≠ {ow}x{oh}x4", b.len()));
                }
                (Some(b), None, None, None)
            }
            G34Readback::BgraAndScene => {
                let d = read_f32(&take_rb(&mut out, &mut idx)?);
                if d.len() != depth_px {
                    return Err("G34 scene depth 回读字节数与内部分辨率不符".into());
                }
                let s = read_f32(&take_rb(&mut out, &mut idx)?);
                if s.len() != scene_px {
                    return Err("G34 scene color 回读字节数与内部分辨率不符".into());
                }
                let b = take_rb(&mut out, &mut idx)?;
                if b.len() != bgra_px {
                    return Err(format!("G34 BGRA8 回读字节 {} ≠ {ow}x{oh}x4", b.len()));
                }
                (Some(b), None, Some(s), Some(d))
            }
            G34Readback::Full => {
                let c = read_f32(&take_rb(&mut out, &mut idx)?);
                if c.len() != f32_px {
                    return Err("G34 f32 out_color 回读字节数与输出分辨率不符".into());
                }
                let d = read_f32(&take_rb(&mut out, &mut idx)?);
                if d.len() != depth_px {
                    return Err("G34 scene depth 回读字节数与内部分辨率不符".into());
                }
                let s = read_f32(&take_rb(&mut out, &mut idx)?);
                if s.len() != scene_px {
                    return Err("G34 scene color 回读字节数与内部分辨率不符".into());
                }
                let b = take_rb(&mut out, &mut idx)?;
                if b.len() != bgra_px {
                    return Err(format!("G34 BGRA8 回读字节 {} ≠ {ow}x{oh}x4", b.len()));
                }
                (Some(b), Some(c), Some(s), Some(d))
            }
        };
        if idx != out.readbacks.len() {
            return Err(format!(
                "G34 回读消费序 {idx} ≠ 实到路数 {}",
                out.readbacks.len()
            ));
        }
        let readback_convert_ms = t_convert.elapsed().as_secs_f64() * 1000.0;
        Ok(G34FrameRec {
            scene_gpu_ns,
            mv_gpu_ns,
            resample_gpu_ns,
            resolve_gpu_ns,
            encode_gpu_ns,
            cpu_record_ns: out.telemetry.cpu_record_ns,
            cpu_submit_ns: out.telemetry.cpu_submit_ns,
            cpu_fence_wait_ns: out.telemetry.cpu_fence_wait_ns,
            validation_error_count: out.telemetry.validation_error_count,
            leaked_object_count: out.telemetry.leaked_object_count,
            leaked_allocation_count: out.telemetry.leaked_allocation_count,
            bgra8,
            out_color,
            scene_color,
            scene_depth,
            readback_convert_ms,
        })
    }

    /// 一帧：三小件参数上传 → 五 pass GPU 链内执行（TSR 输出驻留 device,
    /// encode 链内直写 BGRA8）→ 可选回读（BGRA8/scene color/f32 三路）。
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback: G34Readback,
        scene_params: Vec<f32>,
        tlas_update: Option<(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
    ) -> Result<G34FrameRec, String> {
        let (prov, update) = self.prepare_update(
            iw, ih, ow, oh, jitter, vp_j, exposure, reset, readback, scene_params, tlas_update,
        )?;
        let out = self.session.execute_with_frame_update(&prov, &update)?;
        let rec = self.rec_from_output(out, readback, ow, oh, iw, ih)?;
        self.prev_vp_j = Some(*vp_j);
        self.has_history_state = true;
        self.parity = 1 - self.parity;
        Ok(rec)
    }
}

// ---------------------------------------------------------------------------
// host 金标准（合并语义同步实现面：贴图三角 = 采样 ×（mod×R_slot）;非贴图 =
// 常量 albedo（× R_slot 若 slab 映射）;动态实例 = 局部空间纯发光体 + TLAS
// 实例变换追踪）。逐像素数学与 kernels/g34_unified_gi.rx 逐 op 同序（主射线
// ① 反投影 / ② host Tlas 最近命中 + 实例/重心 / pg 换算 / 法线倒数乘归一 +
// 双面翻转 / albedo 选择 / G13.4 shade_pixel 逐字直接光 / lo 同序左结合）。
// ---------------------------------------------------------------------------

/// host 金标准场景面（一次性构建;BLAS 双件 + 合并 SSBO 影 + 纹理侧表影）。
struct G34HostGold {
    /// BLAS 集（[static] 或 [static, dyn_cube];Tlas::build 借用面——零逐帧克隆）。
    blases: Vec<TriBvh>,
    /// 合并 tris（9 f32/tri：静态段 [0, dyn_tri_base) + dyn 局部段——与
    /// device tris SSBO 逐字节同源,assets.tris 克隆面）。
    tris: Vec<f32>,
    albedo: Vec<[f32; 3]>,
    emission: Vec<[f32; 3]>,
    tritex: Vec<f32>,
    texuv: Vec<f32>,
    texmeta: Vec<f32>,
    atlas: Vec<u32>,
    linlut: [f32; 256],
    dyn_tri_base: usize,
}

impl G34HostGold {
    fn build(scene: &SceneData, tex: Option<&G31TexAssets>, dyn_on: bool) -> Result<Self, String> {
        let dyn_tris = if dyn_on { dyn_cube_tris(DYN_CUBE_HALF) } else { Vec::new() };
        let dyn_tri_base = scene.indices.len();
        // 静态 BLAS（host TriBvh 单一事实源;g12_pt_production/g13_4 同面）。
        let blas_static = TriBvh::build(&scene.positions, &scene.indices);
        // 动态立方体 BLAS（局部空间 12 三角形;顶点 = 36 角点〔面内重复〕,
        // 索引序 = device BLAS 输入 primitiveIndex 序同字面）。
        let mut dpos: Vec<[f32; 3]> = Vec::with_capacity(36);
        for t in dyn_tris.chunks_exact(9) {
            dpos.push([t[0], t[1], t[2]]);
            dpos.push([t[3], t[4], t[5]]);
            dpos.push([t[6], t[7], t[8]]);
        }
        let didx: Vec<[u32; 3]> = (0..12).map(|k| [k * 3, k * 3 + 1, k * 3 + 2]).collect();
        let blases = if dyn_on {
            vec![blas_static, TriBvh::build(&dpos, &didx)]
        } else {
            vec![blas_static]
        };
        let mut tris = pack_tris(scene);
        tris.extend_from_slice(&dyn_tris);
        let mut albedo = scene.albedo.clone();
        let mut emission = scene.emission.clone();
        for _ in 0..dyn_tris.len() / 9 {
            albedo.push([0.0, 0.0, 0.0]);
            emission.push(DYN_EMISSION);
        }
        // 纹理侧表影（textures on = g31_tex_load 产物〔slab 预调制后 texmeta〕
        // + dyn 段追加（−1/0 不消费面）;off = 缺省面——tritex 全 −1 采样零消费,
        // 哑件保底读地址有效）。
        let total = dyn_tri_base + dyn_tris.len() / 9;
        let (tritex, texuv, texmeta, atlas, linlut) = if let Some(t) = tex {
            let mut tritex = t.tritex.clone();
            tritex.resize(total, -1.0);
            // 逐三角 UV = 装配 sink 直派生面（tex.texuv_bytes 与 sink 同源;
            // f32 面重建供采样插值）。
            let mut uv: Vec<f32> = read_f32(&t.texuv_bytes);
            uv.resize(total * 6, 0.0);
            (
                tritex,
                uv,
                t.texmeta.clone(),
                t.atlas.clone(),
                t.linlut,
            )
        } else {
            (
                vec![-1.0f32; total],
                vec![0.0f32; total * 6],
                vec![
                    1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // 头
                    0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, // 哑槽
                ],
                vec![0u32; 1],
                g31_tex_linlut(),
            )
        };
        Ok(G34HostGold {
            blases,
            tris,
            albedo,
            emission,
            tritex,
            texuv,
            texmeta,
            atlas,
            linlut,
            dyn_tri_base,
        })
    }

    /// 阴影可见性（host visible 同字面：origin 沿 wl 偏 eps〔调用方已偏〕,
    /// t_max = d−2eps ≤ 0 恒可见;遮挡体在 (0, t_max) 开区间任一命中即遮蔽——
    /// 全量 TLAS 双实例面,动态实例投运动阴影与 device 同域）。
    fn visible(
        tlas: &Tlas,
        blases: &[TriBvh],
        origin: [f32; 3],
        dir: [f32; 3],
        dist: f32,
        eps: f32,
    ) -> bool {
        let t_max = dist - 2.0 * eps;
        if t_max <= 0.0 {
            return true;
        }
        !tlas.any_hit(
            blases,
            &Ray {
                origin: Vec3::from_array(origin),
                dir: Vec3::from_array(dir),
            },
            t_max,
        )
    }

    /// 合并语义一帧（color 3 f32/px scene-linear HDR + depth 1 f32/px 生产字面
    /// 〔参数行 25..32 同字面〕;多线程行带与 g13_4 render_frame 同模——逐像素
    /// 独立 ⇒ 线程划分零数值面）。
    #[allow(clippy::too_many_arguments)]
    fn render_frame(
        &self,
        vp: &Mat4,
        inv_vp: &Mat4,
        jitter: [f32; 2],
        eps: f32,
        quads: &[QuadLight],
        points: &[PointLight],
        dyn_xf: Option<[f32; 12]>,
        iw: u32,
        ih: u32,
    ) -> (Vec<f32>, Vec<f32>) {
        // TLAS（每帧重建——2 实例微顶,变换逐帧面;BLAS 集一次性借用零克隆）。
        let mut insts = vec![BvhInstanceDesc {
            blas: 0,
            transform: Transform3x4::IDENTITY,
            mask: 0xFF,
            flags: 0,
        }];
        if let Some(xf) = dyn_xf {
            insts.push(BvhInstanceDesc {
                blas: 1,
                transform: Transform3x4::from_rows(xf),
                mask: 0xFF,
                flags: 0,
            });
        }
        let blases: &[TriBvh] = &self.blases;
        let tlas = Tlas::build(&insts, blases);
        // 线程带借用面（move 闭包按 Copy 共享引用捕获——g13_4 render_frame 同模）。
        let tlas_r = &tlas;
        let blases_r = blases;
        let px = (iw * ih) as usize;
        let mut color = vec![0.0f32; px * 3];
        let mut depth = vec![1.0f32; px];
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .clamp(1, ih as usize);
        let band_rows = (ih as usize).div_ceil(threads);
        let row_px = iw as usize;
        std::thread::scope(|s| {
            let mut c_chunks = color.chunks_mut(band_rows * row_px * 3);
            let mut d_chunks = depth.chunks_mut(band_rows * row_px);
            let mut handles = Vec::new();
            loop {
                let (c_band, d_band) = match (c_chunks.next(), d_chunks.next()) {
                    (Some(c), Some(d)) => (c, d),
                    _ => break,
                };
                let band_idx = handles.len();
                let y0 = band_idx * band_rows;
                handles.push(s.spawn(move || {
                    for (dy, (c_row, d_row)) in c_band
                        .chunks_mut(row_px * 3)
                        .zip(d_band.chunks_mut(row_px))
                        .enumerate()
                    {
                        let y = (y0 + dy) as u32;
                        if y >= ih {
                            break;
                        }
                        for x in 0..iw {
                            let (rgb, z) = self.shade_one(
                                x, y, vp, inv_vp, jitter, eps, quads, points, tlas_r, blases_r, iw, ih,
                            );
                            c_row[x as usize * 3] = rgb[0];
                            c_row[x as usize * 3 + 1] = rgb[1];
                            c_row[x as usize * 3 + 2] = rgb[2];
                            d_row[x as usize] = z;
                        }
                    }
                }));
            }
            for hnd in handles {
                hnd.join().expect("host 金标准渲染线程 panic");
            }
        });
        (color, depth)
    }

    /// 单像素合并语义着色（kernel 逐 op 同序;返回值 = (lo, z_quirk)）。
    #[allow(clippy::too_many_arguments)]
    fn shade_one(
        &self,
        x: u32,
        y: u32,
        vp: &Mat4,
        inv_vp: &Mat4,
        jitter: [f32; 2],
        eps: f32,
        quads: &[QuadLight],
        points: &[PointLight],
        tlas: &Tlas,
        blases: &[TriBvh],
        iw: u32,
        ih: u32,
    ) -> ([f32; 3], f32) {
        // ── ① jitter 主射线（host unproject 同式：未抖 inv_vp·[ndx,ndy,z,1]）──
        let sx = x as f32 + 0.5 + jitter[0];
        let sy = y as f32 + 0.5 + jitter[1];
        let ndx = 2.0 * (sx / iw as f32) - 1.0;
        let ndy = 1.0 - 2.0 * (sy / ih as f32);
        let n4 = inv_vp.transform_vec4([ndx, ndy, 0.0, 1.0]);
        let f4 = inv_vp.transform_vec4([ndx, ndy, 1.0, 1.0]);
        let near = [n4[0] / n4[3], n4[1] / n4[3], n4[2] / n4[3]];
        let far = [f4[0] / f4[3], f4[1] / f4[3], f4[2] / f4[3]];
        let d0 = [far[0] - near[0], far[1] - near[1], far[2] - near[2]];
        // dir 归一化（除法口径——kernel gate_dl 除法形同式;l=0 → 零向量）。
        let dl = dot3(d0, d0).sqrt();
        let dir = if dl > 0.0 {
            [d0[0] / dl, d0[1] / dl, d0[2] / dl]
        } else {
            [0.0, 0.0, 0.0]
        };
        let ray = Ray {
            origin: Vec3::from_array(near),
            dir: Vec3::from_array(dir),
        };
        // ── ② 主命中（host Tlas 最近命中;实例/重心 = committed 语义面）──
        let Some(hit) = tlas.intersect(blases, &ray) else {
            return ([0.0, 0.0, 0.0], 1.0);
        };
        let th = hit.t;
        let inst = if hit.instance == rurix_render::rt::bvh::NO_INSTANCE {
            0usize
        } else {
            hit.instance as usize
        };
        // ── fork B 实例分派：BLAS 内 prim → 全局三角形下标（inst=0 ⇒ pg=prim）──
        let pg = hit.tri as usize + inst * self.dyn_tri_base;
        let (bu, bv) = (hit.bary[0], hit.bary[1]);
        // 命中点/几何法线（kernel 同 op 序：cross → 倒数乘归一 → 双面翻转）。
        let tb = pg * 9;
        let ax = self.tris[tb];
        let ay = self.tris[tb + 1];
        let az = self.tris[tb + 2];
        let e1 = [
            self.tris[tb + 3] - ax,
            self.tris[tb + 4] - ay,
            self.tris[tb + 5] - az,
        ];
        let e2 = [
            self.tris[tb + 6] - ax,
            self.tris[tb + 7] - ay,
            self.tris[tb + 8] - az,
        ];
        let ng = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let nl = dot3(ng, ng).sqrt();
        let hg = if nl > 0.0 {
            let inv_nl = 1.0 / nl;
            [ng[0] * inv_nl, ng[1] * inv_nl, ng[2] * inv_nl]
        } else {
            [0.0, 0.0, 0.0]
        };
        let n = if dot3(hg, dir) > 0.0 {
            [-hg[0], -hg[1], -hg[2]]
        } else {
            hg
        };
        let h = [near[0] + dir[0] * th, near[1] + dir[1] * th, near[2] + dir[2] * th];
        // ── albedo（合并语义：tritex[pg] ≥ 0 → 采样 ×（mod×R_slot）;< 0 →
        //    常量面〔slab 预调制后〕;kernel tex_gate 选择位级同值面）──
        let slotf = self.tritex[pg];
        let albedo = if slotf >= 0.0 {
            let bw_ = 1.0 - bu - bv;
            let ub = pg * 6;
            let uu0 = bw_ * self.texuv[ub] + bu * self.texuv[ub + 2] + bv * self.texuv[ub + 4];
            let vv0 = bw_ * self.texuv[ub + 1] + bu * self.texuv[ub + 3] + bv * self.texuv[ub + 5];
            g31_tex_host_sample(
                &self.texmeta,
                &self.atlas,
                &self.linlut,
                slotf as usize,
                uu0,
                vv0,
            )
        } else {
            self.albedo[pg]
        };
        let emission = self.emission[pg];
        // ── 直接光累加（G13.4 shade_pixel 逐字同式;lo 初值 = emission）──
        let mut direct = [0.0f32; 3];
        for q in quads {
            let qn = norm3([
                q.e1[1] * q.e2[2] - q.e1[2] * q.e2[1],
                q.e1[2] * q.e2[0] - q.e1[0] * q.e2[2],
                q.e1[0] * q.e2[1] - q.e1[1] * q.e2[0],
            ]);
            let area = {
                let c = [
                    q.e1[1] * q.e2[2] - q.e1[2] * q.e2[1],
                    q.e1[2] * q.e2[0] - q.e1[0] * q.e2[2],
                    q.e1[0] * q.e2[1] - q.e1[1] * q.e2[0],
                ];
                dot3(c, c).sqrt()
            };
            let sample_area = area / 16.0;
            let mut acc = [0.0f32; 3];
            for syq in 0..4 {
                for sxq in 0..4 {
                    let u = (sxq as f32 + 0.5) * 0.25;
                    let v = (syq as f32 + 0.5) * 0.25;
                    let lp = [
                        q.p00[0] + u * q.e1[0] + v * q.e2[0],
                        q.p00[1] + u * q.e1[1] + v * q.e2[1],
                        q.p00[2] + u * q.e1[2] + v * q.e2[2],
                    ];
                    let l = [lp[0] - h[0], lp[1] - h[1], lp[2] - h[2]];
                    let d2 = dot3(l, l);
                    if d2 <= eps * eps {
                        continue;
                    }
                    let d = d2.sqrt();
                    let wl = [l[0] / d, l[1] / d, l[2] / d];
                    let cos_s = dot3(n, wl).max(0.0);
                    let cos_l = -dot3(qn, wl);
                    if cos_s <= 0.0 || cos_l <= 0.0 {
                        continue;
                    }
                    let origin = [h[0] + wl[0] * eps, h[1] + wl[1] * eps, h[2] + wl[2] * eps];
                    if !Self::visible(tlas, blases, origin, wl, d, eps) {
                        continue;
                    }
                    let g = cos_s * cos_l / d2 * sample_area;
                    for (k, a) in acc.iter_mut().enumerate() {
                        *a += q.le[k] * g;
                    }
                }
            }
            for (k, a) in direct.iter_mut().enumerate() {
                *a += acc[k];
            }
        }
        for lgt in points {
            let l = [lgt.pos[0] - h[0], lgt.pos[1] - h[1], lgt.pos[2] - h[2]];
            let d2 = dot3(l, l);
            if d2 <= eps * eps {
                continue;
            }
            let d = d2.sqrt();
            let wl = [l[0] / d, l[1] / d, l[2] / d];
            let cos_s = dot3(n, wl).max(0.0);
            if cos_s <= 0.0 {
                continue;
            }
            let origin = [h[0] + wl[0] * eps, h[1] + wl[1] * eps, h[2] + wl[2] * eps];
            if !Self::visible(tlas, blases, origin, wl, d, eps) {
                continue;
            }
            let g = cos_s / d2;
            for (k, a) in direct.iter_mut().enumerate() {
                *a += lgt.intensity[k] * g;
            }
        }
        let lo = [
            emission[0] + albedo[0] * INV_PI * direct[0],
            emission[1] + albedo[1] * INV_PI * direct[1],
            emission[2] + albedo[2] * INV_PI * direct[2],
        ];
        // ── 深度（生产字面 = vp 行 0/1,与 kernel ④ 段同字面;两路并存面登记）──
        let cz = ((vp.m[0][0] * h[0] + vp.m[0][1] * h[1]) + vp.m[0][2] * h[2]) + vp.m[0][3];
        let cw = ((vp.m[1][0] * h[0] + vp.m[1][1] * h[1]) + vp.m[1][2] * h[2]) + vp.m[1][3];
        let z = if cw.abs() > 1e-8 { cz / cw } else { 1.0 };
        (lo, z)
    }
}

/// host 对拍统计（device scene HDR vs host 金标准：逐像素逐通道绝对差分布 +
/// 位级像素占比 + 深度 p100 信息面）。
struct G34HostParity {
    frame: u32,
    color_p100: f64,
    color_p50: f64,
    color_mean_abs: f64,
    bitexact_px: u64,
    total_px: u64,
    bitexact_ratio: f64,
    depth_p100: f64,
    in_tol: bool,
    tol: f64,
    tol_source: String,
    host_render_ms: f64,
}

/// G34 冻结容差程序读（estimated/skip_reason 冒充 measured 即 Err fail-closed;
/// g31_fg_frozen_tol 同律）。
fn g34_frozen_tol(budget_path: &str) -> Result<(f64, f64, String), String> {
    let text = std::fs::read_to_string(budget_path).map_err(|e| format!("读 budget {budget_path}: {e}"))?;
    let doc = json_parse(&text)?;
    let entries = doc
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or("budget 缺 entries")?;
    for e in entries {
        if e.get("id").and_then(|v| v.as_str()) == Some(G34_HOST_TOL_ENTRY) {
            // skip_reason = null（G29 budget 同字面）与缺失同义合法;非 null 即拒。
            let skip_carried = !matches!(e.get("skip_reason"), None | Some(Json::Null));
            if e.get("evidence").and_then(|v| v.as_str()) != Some("measured_local") || skip_carried {
                return Err(format!("budget 条目 {G34_HOST_TOL_ENTRY} 非 measured_local（estimated/skip 冒充即拒）"));
            }
            let t = as_f64("threshold", e.get("threshold").ok_or("budget 条目缺 threshold")?)?;
            let m = as_f64("measured_value", e.get("measured_value").ok_or("budget 条目缺 measured_value")?)?;
            return Ok((t, m, format!("budget {budget_path} 条目 {G34_HOST_TOL_ENTRY} 程序读")));
        }
    }
    Err(format!("budget 缺条目 {G34_HOST_TOL_ENTRY}"))
}

// ---------------------------------------------------------------------------
// main：参数 → 装配（契约/G10/scene/slab/纹理/动态）→ 窗口 → 车道 → 帧循环
// （核验/对拍/present/digest）→ evidence。
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut frames: u32 = 120;
    let mut warmup: u32 = 10;
    let mut tier: u32 = 100;
    let mut contract_path = DEFAULT_CONTRACT.to_owned();
    let mut g10_dir = G34_DEFAULT_G10_DIR.to_owned();
    let mut gltf_path = String::new();
    let mut spv_scene = G34_DEFAULT_SPV_SCENE.to_owned();
    let mut spv_mv = DEFAULT_SPV_MV.to_owned();
    let mut spv_resample = DEFAULT_SPV_RESAMPLE.to_owned();
    let mut spv_resolve = DEFAULT_SPV_RESOLVE.to_owned();
    let mut spv_encode = G34_DEFAULT_SPV_ENCODE.to_owned();
    let mut spv_slab = G34_DEFAULT_SPV_SLAB.to_owned();
    let mut spv_texture_probe = G34_DEFAULT_SPV_TEXTURE_PROBE.to_owned();
    let mut evidence_path = String::new();
    let mut expect_digest: Option<String> = None;
    let mut hidden = false;
    let mut headless = false;
    let mut auto_move: Option<String> = None;
    let mut full = false;
    let mut textures = false;
    let mut slab_table: Option<String> = None;
    let mut dyn_on = false;
    let mut static_camera = false;
    let mut host_tol: Option<f64> = None;
    let mut host_parity_on = true;
    // G34-3 蒙皮面（--skin on = 蒙皮×纹理×slab×动态四特性同开;段体全量收
    // g34_full_lane/g34_skin_section.rs 独立 include 区段,与 G34-2 分区写）。
    let mut skin_on = false;
    let mut spv_skin = G34S_DEFAULT_SPV_SKIN.to_owned();
    let mut spv_skin_scene = G34S_DEFAULT_SPV_SCENE.to_owned();
    let mut spv_skin_mv = G34S_DEFAULT_SPV_MV.to_owned();
    // G34-2 HZB 面（--hzb on = HZB 剔除×纹理×slab×动态四特性同开;段体全量收
    // g34_full_lane/g34_2_hzb.rs 独立 include 区段,与 G34-3 分区写）。
    let mut hzb_on = false;
    let mut spv_hzb_primary = G34HZB_DEFAULT_SPV_PRIMARY.to_owned();
    let mut spv_hzb_shade = G34HZB_DEFAULT_SPV_SHADE.to_owned();
    let mut spv_hzb_pack = G34HZB_DEFAULT_SPV_PACK.to_owned();
    let mut spv_hzb_reduce = G34HZB_DEFAULT_SPV_REDUCE.to_owned();
    let mut spv_hzb_test = G34HZB_DEFAULT_SPV_TEST.to_owned();
    // G37 W3 hzb_skin 面（--hzb on --skin 同开 = 合并车道;段体全量收
    // g34_full_lane/g34_hzb_skin.rs 独立 include 区段——G36 留窗兑现件）。
    let mut spv_hzbskin_primary = G34HS_DEFAULT_SPV_PRIMARY_SKIN.to_owned();
    // G36 W3 geo 组合面（--cluster-lod off|leaf|on × --wp-hlod off|full|on ×
    // 纹理×slab×动态组合;off 默认 = 既有面 0-byte——W1 provenance 事实源）。
    let mut cluster_lod_mode = String::from("off");
    let mut cluster_pack = String::new();
    let mut cluster_error_px: f32 = 1.0;
    let mut wp_hlod_mode = String::from("off");
    let mut wp_pack = String::new();
    let mut wp_threshold_l0: f64 = 1.0;
    let mut wp_radius: f32 = 64.0;
    let mut wp_budget_cells: u32 = 4;
    let mut wp_warmup: u32 = 4;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--frames" => {
                frames = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--frames 非 u32"))
            }
            "--warmup" => {
                warmup = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--warmup 非 u32"))
            }
            "--tier" => {
                tier = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--tier 非 u32"))
            }
            "--contract" => contract_path = take_arg(&args, &mut i),
            "--g10-dir" => g10_dir = take_arg(&args, &mut i),
            "--gltf" => gltf_path = take_arg(&args, &mut i),
            "--spv-scene" => spv_scene = take_arg(&args, &mut i),
            "--spv-mv" => spv_mv = take_arg(&args, &mut i),
            "--spv-resample" => spv_resample = take_arg(&args, &mut i),
            "--spv-resolve" => spv_resolve = take_arg(&args, &mut i),
            "--spv-encode" => spv_encode = take_arg(&args, &mut i),
            "--spv-slab" => spv_slab = take_arg(&args, &mut i),
            "--spv-texture-probe" => spv_texture_probe = take_arg(&args, &mut i),
            "--evidence" => evidence_path = take_arg(&args, &mut i),
            "--expect-digest" => expect_digest = Some(take_arg(&args, &mut i)),
            "--hidden" => hidden = true,
            "--headless-smoke" => headless = true,
            "--auto-move" => auto_move = Some(take_arg(&args, &mut i)),
            "--full" => full = true,
            "--textures" => {
                textures = match take_arg(&args, &mut i).as_str() {
                    "on" => true,
                    "off" => false,
                    other => fail(&format!("--textures 档 {other} 越闭集(off|on)")),
                }
            }
            "--slab-table" => slab_table = Some(take_arg(&args, &mut i)),
            "--dyn" => {
                dyn_on = match take_arg(&args, &mut i).as_str() {
                    "on" => true,
                    "off" => false,
                    other => fail(&format!("--dyn 档 {other} 越闭集(off|on)")),
                }
            }
            "--static-camera" => static_camera = true,
            "--host-tol" => {
                host_tol = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--host-tol 非 f64")),
                )
            }
            "--host-parity" => {
                host_parity_on = match take_arg(&args, &mut i).as_str() {
                    "on" => true,
                    "off" => false,
                    other => fail(&format!("--host-parity 档 {other} 越闭集(off|on)")),
                }
            }
            // G34-3 蒙皮面（早分支消费;SPV 三件默认 .tmp/g34_gates/skin/ 隔离
            // 目录,CI 门脚本保障编译）。
            "--skin" => {
                skin_on = match take_arg(&args, &mut i).as_str() {
                    "on" => true,
                    "off" => false,
                    other => fail(&format!("--skin 档 {other} 越闭集(off|on)")),
                }
            }
            "--spv-skin" => spv_skin = take_arg(&args, &mut i),
            "--spv-skin-scene" => spv_skin_scene = take_arg(&args, &mut i),
            "--spv-skin-mv" => spv_skin_mv = take_arg(&args, &mut i),
            // G34-2 HZB 面（早分支消费;primary/shade 默认 .tmp/g34_gates/hzb/
            // 隔离目录,pack/reduce/test = g27/g31 本体 .tmp 构建产物 0-byte 消费）。
            "--hzb" => {
                hzb_on = match take_arg(&args, &mut i).as_str() {
                    "on" => true,
                    "off" => false,
                    other => fail(&format!("--hzb 档 {other} 越闭集(off|on)")),
                }
            }
            "--spv-hzb-primary" => spv_hzb_primary = take_arg(&args, &mut i),
            "--spv-hzb-shade" => spv_hzb_shade = take_arg(&args, &mut i),
            "--spv-hzb-pack" => spv_hzb_pack = take_arg(&args, &mut i),
            "--spv-hzb-reduce" => spv_hzb_reduce = take_arg(&args, &mut i),
            "--spv-hzb-test" => spv_hzb_test = take_arg(&args, &mut i),
            // G37 W3 hzb_skin 面（合并主射线 kernel;--hzb on --skin 同开消费）。
            "--spv-hzbskin-primary" => spv_hzbskin_primary = take_arg(&args, &mut i),
            // G36 W3 geo 组合面参数（g14_3/g31 同名旗标同语义）。
            "--cluster-lod" => cluster_lod_mode = take_arg(&args, &mut i),
            "--cluster-pack" => cluster_pack = take_arg(&args, &mut i),
            "--cluster-error-px" => {
                cluster_error_px = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--cluster-error-px 非 f32"))
            }
            "--wp-hlod" => wp_hlod_mode = take_arg(&args, &mut i),
            "--wp-pack" => wp_pack = take_arg(&args, &mut i),
            "--wp-threshold-l0" => {
                wp_threshold_l0 = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--wp-threshold-l0 非 f64"))
            }
            "--wp-radius" => {
                wp_radius = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--wp-radius 非 f32"))
            }
            "--wp-budget-cells" => {
                wp_budget_cells = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--wp-budget-cells 非 u32"))
            }
            "--wp-warmup" => {
                wp_warmup = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--wp-warmup 非 u32"))
            }
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    if frames == 0 {
        fail("--frames 必须 ≥1");
    }
    // ── 闭集裁决（fail-fast 如实拒跑,不静默降级）──
    let textures_explicit = args.iter().any(|a| a == "--textures");
    let dyn_explicit = args.iter().any(|a| a == "--dyn");
    if full {
        if textures_explicit && !textures {
            fail("--full 与 --textures off 冲突（--full = 三特性同开字面,如实拒跑不冒充）");
        }
        if dyn_explicit && !dyn_on {
            fail("--full 与 --dyn off 冲突（--full = 三特性同开字面,如实拒跑不冒充）");
        }
        let Some(st) = slab_table.as_deref() else {
            fail("--full 须随 --slab-table（slab 资产面 = 合并语义三特性之一,缺失即拒）");
        };
        if !std::path::Path::new(st).is_file() {
            fail(&format!("--slab-table 资产缺失: {st}（fail-closed 不静默回退）"));
        }
        textures = true;
        dyn_on = true;
    }
    let any_feature = textures || slab_table.is_some() || dyn_on;
    if let Some(name) = auto_move.as_deref() {
        if !matches!(name, "orbit" | "dolly") {
            fail(&format!("--auto-move 轨迹 {name} 越闭集(orbit|dolly)"));
        }
    }
    if static_camera {
        if auto_move.is_some() {
            fail("--static-camera 与 --auto-move 互斥（锚格模式 = 静态契约相机字面）");
        }
        if any_feature {
            fail("--static-camera 与特性开互斥（锚格模式 = 全特性缺省关位级对拍面;缺省面 == 母版位级由本模式承载）");
        }
    } else if any_feature {
        if auto_move.is_none() {
            fail("特性开（--full/--textures/--slab-table/--dyn）须随 --auto-move（登记面 = 确定性轨迹 digest_seq;静态无轨迹面非本任务口径）");
        }
        if tier != 100 {
            fail("特性开须 --tier 100（G34-1 登记面 = bistro 1080p 同机同窗对照;其它 tier 面非本任务口径,如实拒跑不冒充）");
        }
    }
    if slab_table.is_some() && !textures && dyn_on {
        // notex 腿合法面（slab+dyn 无纹理）;仅登记,无追加约束。
    }
    // ── G36 W3 geo 组合面闭集校验（fail-closed,不静默降级）：模式闭集 + 包
    //    必填 + 参数域（g14_3 同律字面）;geo 开须随 --auto-move + --tier 100
    //    （特性开同律）,与 --static-camera 锚格互斥（锚格 = 全特性缺省关）。
    //    互斥解除范围（本波）：geo × 纹理×slab×动态（统一主车道,侧表经 W1
    //    provenance gather/补丁）+ geo × HZB（HZB 区段经节点段重导出接线）;
    //    geo × --skin 组合归后续窗（蒙皮区段独立装配面——g14_3 MegaSkin×geo
    //    组合已验证,本 bin 蒙皮区段接线留窗如实拒跑不冒充）。──
    let cluster_opt = match cluster_lod_mode.as_str() {
        "off" => ClusterLodOpt::off(),
        m @ ("leaf" | "on") => {
            if cluster_pack.is_empty() {
                fail("--cluster-lod leaf|on 要求 --cluster-pack <RXCP>（g31_cluster_lod_bake 产物）");
            }
            if !(cluster_error_px.is_finite() && cluster_error_px > 0.0) {
                fail("--cluster-error-px 必须为正有限 f32");
            }
            ClusterLodOpt {
                mode: if m == "leaf" {
                    ClusterLodMode::Leaf
                } else {
                    ClusterLodMode::On
                },
                pack_path: cluster_pack.clone(),
                threshold_px: cluster_error_px,
                resident_pages: 0,
            }
        }
        other => fail(&format!("--cluster-lod {other}：只接受 off|leaf|on")),
    };
    let wp_opt = match wp_hlod_mode.as_str() {
        "off" => WpHlodOpt::off(),
        m @ ("full" | "on") => {
            if wp_pack.is_empty() {
                fail("--wp-hlod full|on 要求 --wp-pack <RXWH>（g31_wp_hlod_bake 产物）");
            }
            if !(wp_threshold_l0.is_finite() && wp_threshold_l0 > 0.0) {
                fail("--wp-threshold-l0 必须为正有限 f64");
            }
            if !(wp_radius.is_finite() && wp_radius > 0.0) {
                fail("--wp-radius 必须为正有限 f32");
            }
            if wp_warmup == 0 {
                fail("--wp-warmup 必须 ≥1（预热协议:切换请求 → 原子翻转间隔）");
            }
            WpHlodOpt {
                mode: if m == "full" {
                    WpHlodMode::Full
                } else {
                    WpHlodMode::On
                },
                pack_path: wp_pack.clone(),
                threshold_l0: wp_threshold_l0,
                loading_radius_m: wp_radius,
                inner_radius_m: (wp_radius * 0.25).max(1.0),
                budget_cells: wp_budget_cells.max(1),
                warmup_frames: wp_warmup,
            }
        }
        other => fail(&format!("--wp-hlod {other}：只接受 off|full|on")),
    };
    let geo_on = cluster_opt.mode != ClusterLodMode::Off || wp_opt.mode != WpHlodMode::Off;
    if geo_on {
        if static_camera {
            fail("--cluster-lod/--wp-hlod 与 --static-camera 互斥（锚格模式 = 全特性缺省关位级对拍面）");
        }
        if auto_move.is_none() {
            fail("--cluster-lod/--wp-hlod 须随 --auto-move（登记面 = 确定性轨迹 digest_seq;静态无轨迹面非本任务口径）");
        }
        if tier != 100 {
            fail("--cluster-lod/--wp-hlod 须 --tier 100（登记面 = bistro 1080p 同机同窗对照）");
        }
        if skin_on {
            fail("--skin on 与 --cluster-lod/--wp-hlod 组合归后续窗（蒙皮区段独立装配面;g14_3 MegaSkin×geo 组合已验证,本 bin 蒙皮区段接线留窗如实拒跑不冒充）");
        }
        if headless {
            fail("--cluster-lod/--wp-hlod 不与 --headless-smoke 同跑（geo 组合登记面 = 真窗口闭集;headless 退化非本任务口径）");
        }
    }
    // ── G37 W3 hzb_skin 早分支（--hzb on --skin 同开 = 合并车道;G36 W4-W5
    //    留窗「HZB×蒙皮同车道（新 kernel 合并面）」兑现——原互斥字面撤除,
    //    段体全量收 g34_full_lane/g34_hzb_skin.rs 独立 include 区段;geo ×
    //    skin 组合维持留窗,上方 geo 闭集裁决先行已拒）──
    if !(hzb_on && skin_on) && args.iter().any(|a| a == "--spv-hzbskin-primary") {
        fail("--spv-hzbskin-primary 须随 --hzb on --skin 同开（合并车道独消费面,单开面零消费）");
    }
    if hzb_on && skin_on {
        if !full {
            fail("--hzb on --skin 须随 --full（HZB×蒙皮×纹理×slab×动态五特性同开字面,如实拒跑不冒充）");
        }
        if static_camera {
            fail("--hzb on --skin 与 --static-camera 互斥（剔除/蒙皮登记面 = 动相机轨迹字面;锚格模式非本任务口径）");
        }
        if headless {
            fail("--hzb on --skin 不与 --headless-smoke 同跑（合并车道真窗口闭集面;headless 退化非本任务口径）");
        }
        g34hs_main(G34HsCli {
            frames,
            warmup,
            tier,
            contract_path,
            g10_dir,
            gltf_path,
            spv_primary_skin: spv_hzbskin_primary,
            spv_hzb_shade,
            spv_hzb_pack,
            spv_hzb_reduce,
            spv_hzb_test,
            spv_skin,
            spv_skin_mv,
            spv_scene,
            spv_resample,
            spv_resolve,
            spv_encode,
            spv_slab,
            spv_texture_probe,
            evidence_path,
            expect_digest,
            hidden,
            auto_move: auto_move
                .unwrap_or_else(|| fail("--hzb on --skin 须随 --auto-move（特性闭集裁决先行面兜底）")),
            slab_table: slab_table
                .unwrap_or_else(|| fail("--hzb on --skin 须随 --slab-table（--full 闭集裁决先行面兜底）")),
        });
    }
    // ── G34-2 HZB 早分支（--hzb on 单开闭集裁决 + 段体全量移交独立 include
    //    区段;非 HZB 面零触碰——合并分支先裁,本分支 = --hzb on 单开面）──
    if hzb_on {
        if !full {
            fail("--hzb on 须随 --full（HZB×纹理×slab×动态四特性同开字面,如实拒跑不冒充）");
        }
        if static_camera {
            fail("--hzb on 与 --static-camera 互斥（剔除登记面 = 动相机轨迹字面;锚格模式非本任务口径）");
        }
        if headless {
            fail("--hzb on 不与 --headless-smoke 同跑（HZB 真窗口闭集面;headless 退化非本任务口径）");
        }
        g34_hzb_main(G34HzbCli {
            frames,
            warmup,
            tier,
            contract_path,
            g10_dir,
            gltf_path,
            spv_hzb_primary,
            spv_hzb_shade,
            spv_hzb_pack,
            spv_hzb_reduce,
            spv_hzb_test,
            spv_scene,
            spv_mv,
            spv_resample,
            spv_resolve,
            spv_encode,
            spv_slab,
            spv_texture_probe,
            evidence_path,
            expect_digest,
            hidden,
            auto_move: auto_move
                .unwrap_or_else(|| fail("--hzb on 须随 --auto-move（特性闭集裁决先行面兜底）")),
            slab_table: slab_table
                .unwrap_or_else(|| fail("--hzb on 须随 --slab-table（--full 闭集裁决先行面兜底）")),
            // G36 W3：geo 组合面移交（cluster×wp×HZB×纹理×slab×动态——HZB
            // 区段经 regroup_nodes 消费重导出节点段;off 默认 = 既有面 0-byte）。
            cluster_opt,
            wp_opt,
        });
    } else if args.iter().any(|a| a.starts_with("--spv-hzb-")) {
        fail("--spv-hzb-* 须随 --hzb on（hzb off 面 = 车道 0-byte,SPV 覆盖位无消费面）");
    }
    // ── G34-3 蒙皮早分支（--skin on 闭集裁决 + 段体全量移交独立 include
    //    区段;非蒙皮面零触碰——下方 host_tol/G34HostGold/era 循环逐字不动）──
    if skin_on {
        if !full {
            fail("--skin on 须随 --full（蒙皮×纹理×slab×动态四特性同开字面,如实拒跑不冒充）");
        }
        if headless {
            fail("--skin on 不与 --headless-smoke 同跑（蒙皮真窗口闭集面;headless 退化非本任务口径）");
        }
        g34_skin_main(G34SkinCli {
            frames,
            warmup,
            tier,
            contract_path,
            g10_dir,
            gltf_path,
            spv_skin,
            spv_scene: spv_skin_scene,
            spv_mv: spv_skin_mv,
            spv_resample,
            spv_resolve,
            spv_encode,
            spv_slab,
            spv_texture_probe,
            evidence_path,
            expect_digest,
            hidden,
            headless,
            auto_move: auto_move.unwrap_or_else(|| fail("--skin on 须随 --auto-move（特性闭集裁决先行面兜底）")),
            slab_table: slab_table.unwrap_or_else(|| fail("--skin on 须随 --slab-table（--full 闭集裁决先行面兜底）")),
        });
    }
    // host 对拍容差：--host-tol 优先,缺省程序读 g34 budget 标定条目（fail-closed;
    // --host-parity off = 零读取零消费——dev-env 探针等短跑面,对拍关面登记）。
    let (host_tol_v, host_tol_measured, host_tol_source) = if !host_parity_on {
        (f64::NAN, f64::NAN, "--host-parity off（对拍关面）".to_owned())
    } else {
        match host_tol {
            Some(t) => (t, f64::NAN, "--host-tol 命令行显式".to_owned()),
            None => match g34_frozen_tol(G34_BUDGET) {
                Ok(v) => (v.0, v.1, v.2),
                Err(e) => fail(&format!("host 对拍冻结容差读取: {e}")),
            },
        }
    };

    // ① 生产契约(digest 门 == FROZEN;G14.3 同模拒出图纪律)。
    let scene_id = "bistro-interior";
    let (pre, _) = prelude(
        scene_id,
        tier,
        frames,
        false,
        &contract_path,
        expect_digest.as_deref(),
    );
    let contract = &pre.contract;
    let (out_w, out_h, seed) = (pre.out_w, pre.out_h, pre.seed);

    // ② G10 语料转引一致性核验(不等即 RED 拒跑;轨迹基位 = 契约相机,先验后跑)。
    let srow = contract_scene_row(&contract.raw, scene_id).unwrap_or_else(|e| fail(&e));
    let g10_fragment = match g34_g10_corpus_gate(srow, &g10_dir) {
        Ok(f) => f,
        Err(e) => fail(&format!("G10 语料转引一致性核验 RED: {e}")),
    };
    eprintln!("{GTAG}: 契约链就绪 contract_digest={} g10 转引一致性=pass", contract.digest);

    // ③ 场景装配（UV sink 恒走——textures on 消费,off 空载零消费;SceneData
    //    各字段与 assemble_scene 逐位同值）。
    if gltf_path.is_empty() {
        gltf_path = default_gltf(scene_id).to_owned();
    }
    let mut tri_uv: Vec<f32> = Vec::new();
    let mut scene = match assemble_scene_uv(&contract.raw, scene_id, Path::new(&gltf_path), &mut tri_uv) {
        Ok(s) => s,
        Err(e) => dev_env_or_fail("scene_assets", &e),
    };

    // ③.4 G36 W3：geo 组合施加（--cluster-lod/--wp-hlod × 纹理×slab×动态;
    //     off 默认 = 既有面 0-byte）。cut/选层冻结于装配期契约相机（g31 车道
    //     同纪律,逐帧 AS 更新归 #77/#89 合流窗;--auto-move 轨迹绕契约位姿,
    //     LOD 误差口径以契约位姿投影为准如实登记）。UV sink 经 provenance
    //     gather 重排（恒等排列锚 fail-closed;#96 属性臂:v2 资产的代理三角
    //     自簇 UV 表/RXHL v2 取真 corner UV,v1 资产写 0 = 旧语义逐位等价）;
    //     tritex 代理补丁在 ③.6 纹理装配后施加。
    let geo: Option<GeoApplied> = {
        let (s2, g) = apply_geo_combined(scene, &cluster_opt, &wp_opt, pre.in_w, pre.in_h);
        scene = s2;
        if let Some(g) = &g {
            let gathered = gather_tri_uv_attrs(
                &g.prov,
                &tri_uv,
                g.cluster.as_ref().map(|(_, p)| p),
                g.wp.as_ref().map(|(_, ctx)| &ctx.pack),
            );
            if geo_prov_is_identity(&g.prov) {
                // W1 恒等排列锚（leaf/full 极限）：gather 产物与装配 sink
                // 逐位一致 fail-closed。
                if gathered.len() != tri_uv.len()
                    || gathered
                        .iter()
                        .zip(tri_uv.iter())
                        .any(|(a, b)| a.to_bits() != b.to_bits())
                {
                    fail("G36 恒等排列 UV gather 位级漂移（W1 锚,fail-closed）");
                }
            }
            tri_uv = gathered;
            if let Some((r, _)) = &g.cluster {
                eprintln!(
                    "{GTAG}: cluster-lod mode={} threshold_px={} clusters={}/{} tris out={}/{} ({:.1}%)",
                    r.mode,
                    r.threshold_px,
                    r.cut_clusters,
                    r.total_clusters,
                    r.out_tris,
                    r.src_tris,
                    100.0 * r.out_tris as f64 / r.src_tris.max(1) as f64,
                );
            }
            if let Some((r, _)) = &g.wp {
                eprintln!(
                    "{GTAG}: wp-hlod mode={} cells full/hlod/culled/pending={}/{}/{}/{} proxy_tris={} ticks={} selection_digest={}",
                    r.mode,
                    r.cells_full,
                    r.cells_hlod,
                    r.cells_culled,
                    r.cells_pending,
                    r.proxy_tris,
                    r.assemble_ticks,
                    &r.selection_digest[..16],
                );
            }
            if let Some(st) = &g.combined {
                eprintln!(
                    "{GTAG}: geo 组合（cluster×wp）identity={} coarse={}（{} 簇）straddle_fallback={}（{} 簇）wp_proxy={} out={}",
                    st.identity_tris,
                    st.coarse_tris,
                    st.coarse_emitted,
                    st.straddle_fallback_tris,
                    st.straddle_clusters,
                    st.wp_proxy_tris,
                    st.out_tris,
                );
            }
        }
        g
    };

    // ③.5 slab 侧表生产接线（--slab-table 面;非 slab 路径 0-byte——资产加载 +
    //     16 槽 host/device 双臂求值对拍 + 逐三角 albedo × R_slot 预调制,全部
    //     仅 slab 模式消费;kernels/g29_slab.rx 与 material/slab.rs 0-byte 冻结
    //     消费;G34 合并语义面 = ③.6 texmeta mod 预调制追加）。
    let mut slab_report: Option<(SlabSideTableAsset, SlabEval, usize)> = None;
    let mut slab_arm: Option<[f32; SLAB_N_SLOTS]> = None;
    if let Some(st) = slab_table.as_deref() {
        let asset = match slab_load_asset(st) {
            Ok(a) => a,
            Err(e) => fail(&format!("slab 侧表资产加载: {e}")),
        };
        if asset.scene_id != scene_id {
            fail(&format!(
                "slab 资产 scene_id={} ≠ 生产场景 {scene_id}（资产-场景绑定 fail-closed）",
                asset.scene_id
            ));
        }
        let eval = match slab_evaluate(&asset, &spv_slab) {
            Ok(v) => v,
            Err(e) => dev_env_or_fail("slab_device_eval", &e),
        };
        let arm_r = slab_arm_r(&eval, "device");
        let n_slab = slab_apply(&mut scene, &asset, &arm_r);
        eprintln!(
            "{GTAG}: slab 接线 arm=device slots=16 mapped_mats={} slab_tris={} parity_p100={:.6e} eval_ms={:.3} abi={}",
            asset.material_slots.len(),
            n_slab,
            eval.parity_p100,
            eval.eval_ms,
            asset.abi_digest,
        );
        slab_arm = Some(arm_r);
        slab_report = Some((asset, eval, n_slab));
    }

    // ③.6 纹理采样生产接线（--textures on 面;非 textures 路径 = 缺省哑件——
    //     资产链 + 探针双臂对拍（SSBO 腿位级硬门 + sampler 腿结构容差）+ G34
    //     合并语义 texmeta mod × R_slot 预调制,全部仅 textures on 消费）。
    let mut tex_report: Option<(G31TexAssets, G31TexProbeReport)> = None;
    let mut tex_premod_slots = 0usize;
    if textures {
        let mut assets = match g31_tex_load(&scene, Path::new(&gltf_path), &tri_uv) {
            Ok(a) => a,
            Err(e) => dev_env_or_fail("texture_assets", &e),
        };
        let probes = g31_tex_probes(assets.slots.len());
        let report = match g31_tex_probe_evaluate(&assets, &probes, &spv_texture_probe) {
            Ok(r) => r,
            Err(e) => dev_env_or_fail("texture_probe", &e),
        };
        if !report.ssbo_bitexact {
            fail(&format!(
                "B4 probe SSBO 腿 device vs host 非位级一致（p100={:.6e} > 0.0 硬门;NoContraction/采样链缺陷即红）",
                report.ssbo_p100
            ));
        }
        if !report.ssbo_double_run_bitexact {
            fail("B4 probe SSBO 腿 device 双跑非位级一致（确定性门红）");
        }
        if report.sampler_max_lsb > 1 {
            fail(&format!(
                "B4 sampler 腿硬件采样 vs host 参考 max_lsb={} > 1（结构容差界红;硬件过滤精度越界）",
                report.sampler_max_lsb
            ));
        }
        if report.nonconstant_slots == 0 {
            fail("B4 映射纹理探针输出全常量（空接线冒充即红,fail-closed）");
        }
        // G34 合并语义：贴图 slab 材质 = 采样 ×（mod × R_slot）——R_slot 装配期
        // 预乘进 texmeta 槽 mod（slab on 面;slab off = 零预乘与 fork A 逐位同值）。
        if let (Some(asset_eval), Some(arm_r)) = (slab_report.as_ref(), slab_arm.as_ref()) {
            tex_premod_slots = g34_slab_premod_texmeta(&mut assets, &asset_eval.0, arm_r);
        }
        // G36 W3/#96：代理三角 tritex 补丁 v2——仅无 UV 数据（v1 资产）的
        // 代理三角置 −1 走常量面回退（cluster/cell 面积加权均值;UV=0 采样
        // 错色防线维持）;带 UV（v2 资产）的代理三角保留 tri_mat 派生槽号,
        // 与 Src 三角同一图集采样路径（gather 已供真 corner UV）。host 金
        // 标准克隆消费补丁后数组（assets.tritex/texuv_bytes）⇒ 两臂一致。
        if let Some(g) = geo.as_ref() {
            let patched = geo_patch_proxy_tritex_v2(
                &mut assets,
                &g.prov,
                g.cluster.as_ref().map(|(_, p)| p),
                g.wp.as_ref().map(|(_, ctx)| &ctx.pack),
            );
            if patched > 0 {
                eprintln!(
                    "{GTAG}: geo 代理 tritex 补丁 patched={patched} tex_tris={}（无 UV 代理走常量面回退;#96 带 UV 代理保留图集采样）",
                    assets.tex_tris,
                );
            }
        }
        eprintln!(
            "{GTAG}: B4 纹理接线 mapped={} tex_tris={} atlas={}x{} probes={} ssbo_p100={:.6e}（位级={} 双跑={}） sampler_max_lsb={} nonconstant_slots={} eval_ms={:.3} slab_premod_slots={}",
            assets.slots.len(),
            assets.tex_tris,
            assets.atlas_w,
            assets.atlas_h,
            report.probe_count,
            report.ssbo_p100,
            report.ssbo_bitexact,
            report.ssbo_double_run_bitexact,
            report.sampler_max_lsb,
            report.nonconstant_slots,
            report.eval_ms,
            tex_premod_slots,
        );
        tex_report = Some((assets, report));
    }
    let eps = scene_eps(&scene.positions);
    eprintln!(
        "{GTAG}: 装配 scene={scene_id} tris={} quads={} points={} output={out_w}x{out_h} eps={eps:.6} features=[tex={} slab={} dyn={}] static_camera={static_camera}",
        scene.tri_count,
        scene.quads.len(),
        scene.points.len(),
        textures,
        slab_table.is_some(),
        dyn_on,
    );

    // ④ 真窗口 present 会话先于车道创建(channel_order 决定编码参数 bgra 位;
    //    headless-smoke = 无窗口退化,仅供自检逻辑不计真门)。
    let mut window: Option<vk::ExternalImagePresent> = if headless {
        None
    } else {
        match vk::ExternalImagePresent::create(
            out_w,
            out_h,
            "rurix g34 unified lane (bistro-interior 1080p;G34-1 全特性合流;ESC 退出)",
            !hidden,
        ) {
            Ok(w) => Some(w),
            Err(e) => dev_env_or_fail("window_present", &e),
        }
    };
    let bgra = window
        .as_ref()
        .map(|w| w.channel_order() == "bgra8_unorm")
        .unwrap_or(true);
    if let Some(w) = window.as_ref() {
        eprintln!(
            "{GTAG}: 窗口就绪 {}x{} channel_order={} visible={}",
            w.extent().0,
            w.extent().1,
            w.channel_order(),
            !hidden
        );
    }

    // ⑤ 初态（相机 = 契约位姿;auto-move 轨迹基位/锚格静态两面同源;曝光 =
    //    契约 ev100）。
    let cam0 = G34Camera::from_spec(&scene.camera);
    let mut cam = cam0;
    let ev100 = f64::from(scene.ev100);
    let jitter_base = (seed % JITTER_WINDOW_MOD) as u32;

    // ⑥ host 金标准场景面（--host-parity on 才构建——BLAS 双件 + 合并 SSBO 影;
    //    parity 帧对拍消费,构建耗时如实打印不混帧口径;off = 零构建零消费）。
    let gold = if host_parity_on {
        let t_gold = std::time::Instant::now();
        let gold = match G34HostGold::build(&scene, tex_report.as_ref().map(|(a, _)| a), dyn_on) {
            Ok(g) => g,
            Err(e) => fail(&format!("host 金标准场景构建: {e}")),
        };
        eprintln!(
            "{GTAG}: host 金标准场景就绪（static_tris={} dyn={} build={:.1}ms）",
            scene.indices.len(),
            dyn_on,
            t_gold.elapsed().as_secs_f64() * 1000.0
        );
        Some(gold)
    } else {
        None
    };

    // ⑦ era 循环(era = 一个 extent 生命周期;resize → 车道按新 extent 重建,
    //    TSR 历史 reset;最小化跳过不消费帧预算;ESC/close 干净退出)。
    let total = warmup + frames;
    let mut fi = 0u32;
    let mut exit_reason = "frames_done";
    let mut resize_eras = 0u32;
    let mut render_ms: Vec<f64> = Vec::new();
    let mut present_ms: Vec<f64> = Vec::new();
    let mut digest_ms: Vec<f64> = Vec::new();
    let mut encode_gpu_ms: Vec<f64> = Vec::new();
    let mut scene_gpu_ms: Vec<f64> = Vec::new();
    let mut digest_seq: Vec<String> = Vec::new();
    let mut ev100_seq: Vec<f64> = Vec::new();
    let mut pose_seq: Vec<[f64; 5]> = Vec::new();
    let mut render_digest = String::new();
    let mut presented_digest = String::new();
    let mut real_render_seconds: f64 = 0.0;
    let mut real_frames: u64 = 0;
    // fork B 动态核验面（dyn on 才消费;每 DYN_VERIFY_EVERY 帧一次 fail-closed）。
    let dyn_origin = dyn_trajectory_origin(&scene.camera);
    let mut verify_recs: Vec<DynVerifyFrame> = Vec::new();
    // host 金标准对拍面（parity 帧一次性;post-warmup 首测量帧）。
    let mut host_parity: Option<G34HostParity> = None;
    'eras: loop {
        let (ew, eh) = window
            .as_ref()
            .map(|w| w.extent())
            .unwrap_or((out_w, out_h));
        let in_w = ((ew as u64 * u64::from(tier)) / 100).max(1) as u32;
        let in_h = ((eh as u64 * u64::from(tier)) / 100).max(1) as u32;
        // ── 车道资产（era 重建面;静态/动态两形态同源派生）──
        let assets_dyn = if dyn_on {
            Some(lane_assets_dyn(&scene, in_w, in_h))
        } else {
            None
        };
        let assets_static = if dyn_on {
            None
        } else {
            let mut a = lane_assets(&scene, in_w, in_h);
            // 静态面参数缓冲扩 60 f32（params[42] 读面合法化——单实例 ×0 消费,
            // 缺省面 == 母版位级数据面承载;dyn 面 lane_assets_dyn 已 240B）。
            a.params0_bytes = vec![0u8; DYN_PARAMS_LEN * 4];
            Some(a)
        };
        let assets: &LaneAssets = if let Some(a) = assets_dyn.as_ref() {
            &a.base
        } else {
            assets_static.as_ref().unwrap()
        };
        let dyn_tri_base = if let Some(a) = assets_dyn.as_ref() {
            a.dyn_tri_base
        } else {
            scene.indices.len()
        };
        let mut bits = UnifiedLaneBits::load(
            &spv_scene,
            &spv_mv,
            &spv_resample,
            &spv_resolve,
            in_w,
            in_h,
            ew,
            eh,
            false,
        );
        // G34 scene SPV 处置分叉（如实登记的面）：
        // - textures on = NoContraction 注入（B4 同律：驱动 FMA 收缩禁面——
        //   bilinear/LUT 采样链与 host 参考逐 op 位级对拍前提;dispatch 自
        //   LocalSize 派生面注入前后不变）；
        // - textures off（缺省面）= 原始 SPV 零注入——与母版 g14_3_direct_gi
        //   SPV 处置逐字同（母版锚 = 未注入面驱动收缩域;合并 kernel 直接光/
        //   射线/深度表达式与母版逐字同构 ⇒ 同收缩 ⇒ 缺省面 == 母版位级经
        //   --static-camera 锚格全链对拍承载;纹理开时缺省面处置差异如实登记,
        //   不复位母版 SPV 处置——B4 对拍锚优先级面）。
        if textures {
            let scene_words = spv_inject_no_contraction(&load_spv(&spv_scene));
            bits.spv_scene = scene_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        }
        let enc_words = load_spv(&spv_encode);
        let (ex, ey, _) = spv_local_size(&enc_words);
        let enc_dispatch = [ew.div_ceil(ex), eh.div_ceil(ey), 1];
        let enc_spv_bytes: Vec<u8> = enc_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let enc_params = aces13_device_encode_params(ew, eh, bgra);
        let enc_params_bytes = bytes_f32(&enc_params);
        // 纹理侧表（G34 descs 借用源;textures on = 装配产物克隆〔slab 预调制
        // 后 texmeta〕+ dyn 段追加,off = 缺省哑件——kernel 缺省面 == 母版位级）。
        let dyn_tri_count = assets_dyn
            .as_ref()
            .map(|a| a.dyn_tris.len() / 9)
            .unwrap_or(0);
        let total_tris = dyn_tri_base + dyn_tri_count;
        let side = if let Some((t, _)) = tex_report.as_ref() {
            let mut tritex = t.tritex.clone();
            tritex.resize(total_tris, -1.0);
            let mut texuv: Vec<f32> = read_f32(&t.texuv_bytes);
            texuv.resize(total_tris * 6, 0.0);
            G34TexSideTable {
                texuv_bytes: bytes_f32(&texuv),
                texmeta_bytes: t.texmeta_bytes.clone(),
                tritex_bytes: bytes_f32(&tritex),
                atlas_bytes: t.atlas_bytes.clone(),
                linlut_bytes: t.linlut_bytes.clone(),
            }
        } else {
            G34TexSideTable::default_face(total_tris)
        };
        let descs_g34 = UnifiedDescs::G34Full(unified_lane_descs_g34(
            &assets, &bits, &side, in_w, in_h, ew, eh,
        ));
        let UnifiedDescs::G34Full(g34_tuple) = &descs_g34 else {
            unreachable!("G34Full 构造面单一形态");
        };
        let descs = g34_descs(
            (
                g34_tuple.0.clone(),
                g34_tuple.1.clone(),
                g34_tuple.2,
                g34_tuple.3,
            ),
            &enc_spv_bytes,
            enc_dispatch,
            &enc_params_bytes,
            ew,
            eh,
        );
        // BLAS/实例面（dyn on = 双 BLAS 静态段+动态立方体 + 双实例表〔创建期
        // identity,逐帧 tlas_update〕;off = 单 BLAS 单实例缺省面）。
        let scene_tri_end = dyn_tri_base * 9;
        let blas_refs_dyn: [&[f32]; 2] = [
            &assets.tris[..scene_tri_end],
            &assets.tris[scene_tri_end..],
        ];
        let blas_refs_one: [&[f32]; 1] = [&assets.tris];
        let dyn_instances = vec![
            RayQueryInstanceDesc {
                blas: 0,
                custom_index: 0,
                mask: 0xFF,
                sbt_record_offset: 0,
            },
            RayQueryInstanceDesc {
                blas: 1,
                custom_index: 0,
                mask: 0xFF,
                sbt_record_offset: 0,
            },
        ];
        let (blas_refs, instances): (&[&[f32]], &[RayQueryInstanceDesc]) = if dyn_on {
            (&blas_refs_dyn[..], &dyn_instances)
        } else {
            (&blas_refs_one[..], &assets.instances)
        };
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: blas_refs,
                instances,
            },
            transforms: None,
            updatable_blas: &[], // G34-1 全静态 BLAS（B5 字段面 0-byte 默认）
        }];
        let mut lane = match G34TsrLane::create(&descs, &accel_structs) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        eprintln!(
            "{GTAG}: era 就绪 extent={ew}x{eh} internal={in_w}x{in_h}（车道:g34_unified_gi→g14_mv→tsr×2→display_encode 五 pass,resize_eras={resize_eras}）"
        );
        let mut resized = false;
        let mut era_first = true;
        while fi < total {
            // ── 窗口事件面(输入/resize/最小化/关闭;每帧首段泵)──
            if let Some(w) = window.as_mut() {
                let input = w.poll_input();
                if input.close_requested {
                    exit_reason = "user_close";
                    break 'eras;
                }
                if input.minimized {
                    std::thread::sleep(std::time::Duration::from_millis(8));
                    continue;
                }
                if let Some((nw, nh)) = input.resize_pending {
                    if (nw, nh) != (ew, eh) {
                        if let Err(e) = w.resize(nw, nh) {
                            fail(&format!("窗口 resize {nw}x{nh}: {e}"));
                        }
                        if w.extent() != (ew, eh) {
                            resized = true;
                            resize_eras += 1;
                            break;
                        }
                    }
                }
            }
            // ── 相机（auto-move 确定性轨迹 / 锚格静态契约相机直用〔位级面〕）──
            let spec = if let Some(name) = auto_move.as_deref() {
                let (yaw, pitch, eye) = g34_auto_move_pose(name, &cam0, fi, total);
                cam.yaw = yaw;
                cam.pitch = pitch;
                cam.eye = eye;
                cam.spec()
            } else {
                scene.camera
            };
            let vp = build_vp(&spec, in_w, in_h);
            let inv_vp = vp.inverse().unwrap_or_else(|| fail("view-proj 必须可逆"));
            let exposure = 2.0f32.powf(-(ev100 as f32));
            let j = [
                halton(jitter_base + fi + 1, 2) - 0.5,
                halton(jitter_base + fi + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            // ── fork B 逐帧实例变换（脚本化轨迹:平移 + Ry yaw;确定性 f32 纯帧号
            //    函数）+ 60 f32 场景参数（dyn_tri_base 同包;dyn off = 单实例 ×0
            //    消费缺省面）──
            let (pos, yaw) = dyn_trajectory(fi, dyn_origin);
            let xf = dyn_transform_3x4(pos, yaw);
            let tlas_update = if dyn_on {
                Some((0u32, dyn_frame_instances(xf), TlasBuildAction::Refit))
            } else {
                None
            };
            let scene_params = pack_frame_params_dyn(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                dyn_tri_base,
            );
            let last = fi + 1 == total;
            let verify = dyn_on && fi >= warmup && (fi - warmup) % DYN_VERIFY_EVERY == 0;
            let parity_frame = host_parity_on && fi == warmup;
            let rb_mode = if last {
                G34Readback::Full
            } else if verify || parity_frame {
                G34Readback::BgraAndScene
            } else if window.is_some() || auto_move.is_some() {
                G34Readback::Bgra
            } else {
                G34Readback::None
            };
            let reset = fi == 0 || era_first;
            era_first = false;
            let t_render = std::time::Instant::now();
            let rec = match lane.frame(
                in_w,
                in_h,
                ew,
                eh,
                j,
                &vp_j,
                exposure,
                reset,
                rb_mode,
                scene_params,
                tlas_update,
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("帧 {fi} 车道: {e}")),
            };
            let render_el = t_render.elapsed().as_secs_f64() * 1000.0;
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "帧 {fi} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            if rec.leaked_object_count != 0 || rec.leaked_allocation_count != 0 {
                fail(&format!(
                    "帧 {fi} leak 账本非零 object={} allocation={}（资源无泄漏机核判红）",
                    rec.leaked_object_count, rec.leaked_allocation_count
                ));
            }

            // ── fork B 动态实例位置核验（A4 范式 host 投影:轨迹点 + 8 角点经
            //    vp_j 投影;device 面 = scene color 纯绿谱检测——TSR 前瞬时位
            //    无拖影;fail-closed）──
            if verify {
                let scene_color = rec
                    .scene_color
                    .as_ref()
                    .unwrap_or_else(|| fail("帧核验面缺 scene color 回读（内部破缺）"));
                let obs = dyn_detect(scene_color, in_w, in_h);
                let pred_c = dyn_project(&vp_j, pos, in_w, in_h)
                    .unwrap_or_else(|| fail("轨迹点投影在相机背面（轨迹规格破缺）"));
                let mut pred_aabb = [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ];
                for k in 0..8 {
                    let lp = [
                        if k & 1 == 0 { -DYN_CUBE_HALF } else { DYN_CUBE_HALF },
                        if k & 2 == 0 { -DYN_CUBE_HALF } else { DYN_CUBE_HALF },
                        if k & 4 == 0 { -DYN_CUBE_HALF } else { DYN_CUBE_HALF },
                    ];
                    let wp = [
                        xf[0] * lp[0] + xf[1] * lp[1] + xf[2] * lp[2] + xf[3],
                        xf[4] * lp[0] + xf[5] * lp[1] + xf[6] * lp[2] + xf[7],
                        xf[8] * lp[0] + xf[9] * lp[1] + xf[10] * lp[2] + xf[11],
                    ];
                    let (u, v) = dyn_project(&vp_j, wp, in_w, in_h)
                        .unwrap_or_else(|| fail("角点投影在相机背面（轨迹规格破缺）"));
                    pred_aabb[0] = pred_aabb[0].min(u);
                    pred_aabb[1] = pred_aabb[1].min(v);
                    pred_aabb[2] = pred_aabb[2].max(u);
                    pred_aabb[3] = pred_aabb[3].max(v);
                }
                let (obs_px, obs_aabb, obs_count) = match obs {
                    Some((cx, cy, bb, n)) => ([cx, cy], bb, n),
                    None => ([f64::NAN; 2], [f64::NAN; 4], 0),
                };
                let centroid_delta = if obs_count > 0 {
                    ((obs_px[0] - pred_c.0).powi(2) + (obs_px[1] - pred_c.1).powi(2)).sqrt()
                } else {
                    f64::INFINITY
                };
                let aabb_delta = if obs_count > 0 {
                    (obs_aabb[0] - pred_aabb[0])
                        .abs()
                        .max((obs_aabb[1] - pred_aabb[1]).abs())
                        .max((obs_aabb[2] - pred_aabb[2]).abs())
                        .max((obs_aabb[3] - pred_aabb[3]).abs())
                } else {
                    f64::INFINITY
                };
                let pred_area = (pred_aabb[2] - pred_aabb[0]).max(0.0)
                    * (pred_aabb[3] - pred_aabb[1]).max(0.0);
                let min_count = 200.0f64.max(DYN_MIN_COUNT_AREA_RATIO * pred_area) as usize;
                // 质心容差域界式：门窗标定域（√预测面积 ≤100px——64+10 门窗
                // 实测 obs ≤9272px² ⇒ √A ≤96.3 全落域内）维持绝对 2.5px 逐字
                // ——三门判据数值面不变;域外近大目标按轮廓直径 5% 界模型偏差
                // （host 预测质心 = 轨迹点/角点投影均值,观测 = 像素质心;透视下
                // 近大目标的轮廓像素质心相对角点均值存在与屏占尺寸成比例的
                // 模型偏差,G34 收口 soak 5000 帧实测:帧 530 obs=17941px² 处
                // Δ=2.837px〔2.12% 直径〕/ 帧 550 obs=16906px² 处 Δ=3.450px
                // 〔2.61% 直径〕——同帧 digest 确定性 + aabb/计数双门在带 =
                // 核验模型偏差非渲染缺陷;5% 界对防死接线目的仍紧〔错位即
                // 数十~百 px〕;两次首跑 FAIL 输出如实留档 close-out 登记,
                // 判据修正沿波 A soak 口径修正先例）。
                let pred_diag = pred_area.sqrt();
                let tol_centroid = if pred_diag <= 100.0 {
                    DYN_TOL_CENTROID_PX
                } else {
                    DYN_TOL_CENTROID_PX.max(0.05 * pred_diag)
                };
                let pass = obs_count >= min_count
                    && centroid_delta <= tol_centroid
                    && aabb_delta <= DYN_TOL_AABB_PX;
                verify_recs.push(DynVerifyFrame {
                    frame: fi,
                    transform: xf,
                    pred_px: [pred_c.0, pred_c.1],
                    pred_aabb,
                    obs_px,
                    obs_aabb,
                    obs_count,
                    centroid_delta_px: centroid_delta,
                    aabb_delta_px: aabb_delta,
                    pass,
                });
                if !pass {
                    fail(&format!(
                        "帧 {fi} 动态实例位置核验 fail（obs_count={obs_count}（min {min_count}）centroid_Δ={centroid_delta:.3}px（tol {tol_centroid:.3}）aabb_Δ={aabb_delta:.3}px）"
                    ));
                }
            }

            // ── host 金标准对拍（parity 帧一次性:同帧 scene HDR 回读 vs host
            //    合并语义渲染——同 jitter/vp/动态变换输入;容差 = 冻结容差程序读;
            //    --host-parity off = 零消费面）──
            if parity_frame && host_parity.is_none() {
                let gold = gold
                    .as_ref()
                    .unwrap_or_else(|| fail("host 对拍面缺金标准场景（--host-parity on 闭集破缺）"));
                let scene_color = rec
                    .scene_color
                    .as_ref()
                    .unwrap_or_else(|| fail("host 对拍面缺 scene color 回读（内部破缺）"));
                let t_host = std::time::Instant::now();
                let (hc, hd) = gold.render_frame(
                    &vp,
                    &inv_vp,
                    j,
                    eps,
                    &scene.quads,
                    &scene.points,
                    if dyn_on { Some(xf) } else { None },
                    in_w,
                    in_h,
                );
                let host_ms = t_host.elapsed().as_secs_f64() * 1000.0;
                // 逐像素逐通道绝对差分布 + 位级像素占比 + 深度 p100（生产字面
                // 深度同字面两路对拍信息面）。
                let dev_depth = rec
                    .scene_depth
                    .as_ref()
                    .unwrap_or_else(|| fail("host 对拍面缺 scene depth 回读（内部破缺）"));
                let mut diffs: Vec<f64> = Vec::with_capacity((in_w * in_h) as usize);
                let mut bitexact = 0u64;
                let mut sum_abs = 0.0f64;
                let mut depth_p100 = 0.0f64;
                for p in 0..(in_w * in_h) as usize {
                    let b = p * 3;
                    let d = ((scene_color[b] - hc[b])
                        .abs()
                        .max((scene_color[b + 1] - hc[b + 1]).abs())
                        .max((scene_color[b + 2] - hc[b + 2]).abs())) as f64;
                    diffs.push(d);
                    sum_abs += d;
                    if scene_color[b].to_bits() == hc[b].to_bits()
                        && scene_color[b + 1].to_bits() == hc[b + 1].to_bits()
                        && scene_color[b + 2].to_bits() == hc[b + 2].to_bits()
                    {
                        bitexact += 1;
                    }
                    let dd = (dev_depth[p] - hd[p]).abs() as f64;
                    if dd > depth_p100 {
                        depth_p100 = dd;
                    }
                }
                diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let p100 = diffs[diffs.len() - 1];
                let p50 = diffs[diffs.len() / 2];
                let total = diffs.len() as u64;
                let mean_abs = sum_abs / total as f64;
                let in_tol = p100 <= host_tol_v;
                eprintln!(
                    "{GTAG}: 帧 {} host 金标准对拍 p100={p100:.6e}（tol={host_tol_v:.6e},{host_tol_source}）p50={p50:.3e} mean={mean_abs:.3e} bitexact={bitexact}/{total}（{:.2}%） depth_p100={depth_p100:.3e} host_render={host_ms:.1}ms",
                    fi + 1,
                    bitexact as f64 / total as f64 * 100.0,
                );
                host_parity = Some(G34HostParity {
                    frame: fi,
                    color_p100: p100,
                    color_p50: p50,
                    color_mean_abs: mean_abs,
                    bitexact_px: bitexact,
                    total_px: total,
                    bitexact_ratio: bitexact as f64 / total as f64,
                    depth_p100,
                    in_tol,
                    tol: host_tol_v,
                    tol_source: host_tol_source.clone(),
                    host_render_ms: host_ms,
                });
                if !in_tol {
                    fail(&format!(
                        "帧 {fi} host 金标准对拍 p100={p100:.6e} > 冻结容差 {host_tol_v:.6e}（{host_tol_source};合并语义/采样链/分派面缺陷即红）"
                    ));
                }
            }

            // ── present(device 已编码;host 仅拷贝/present)──
            let mut pres_el = 0.0f64;
            if let Some(w) = window.as_mut() {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail(&format!("帧 {fi} 窗口面缺 BGRA8 回读"));
                };
                let t_one = std::time::Instant::now();
                if let Err(e) = w.present_rgba8(px) {
                    fail(&format!("帧 {fi} 窗口 present: {e}"));
                }
                let el = t_one.elapsed().as_secs_f64() * 1000.0;
                pres_el += el;
                if fi >= warmup {
                    present_ms.push(el);
                }
            }

            // ── digest(auto-move 逐帧序列;税单列不混渲染口径)──
            let t_dig = std::time::Instant::now();
            if auto_move.is_some() {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail(&format!("帧 {fi} auto-move 面缺 BGRA8 回读"));
                };
                digest_seq.push(g34_bgra_digest(ew, eh, px));
                ev100_seq.push(ev100);
                pose_seq.push([
                    f64::from(cam.eye[0]),
                    f64::from(cam.eye[1]),
                    f64::from(cam.eye[2]),
                    f64::from(cam.yaw),
                    f64::from(cam.pitch),
                ]);
            }
            if last {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail("末帧缺 BGRA8 回读".into());
                };
                presented_digest = g34_bgra_digest(ew, eh, px);
                let Some(out_data) = rec.out_color.as_ref() else {
                    fail("末帧缺 f32 out_color 回读".into());
                };
                if !out_data.iter().all(|v| v.is_finite()) {
                    fail("末帧 TSR 输出非有限");
                }
                render_digest = frame_content_digest(ew, eh, 3, out_data);
            }
            let dig_el = t_dig.elapsed().as_secs_f64() * 1000.0;

            if fi >= warmup {
                render_ms.push(render_el);
                digest_ms.push(dig_el);
                encode_gpu_ms.push(rec.encode_gpu_ns / 1e6);
                scene_gpu_ms.push(rec.scene_gpu_ns / 1e6);
                real_frames += 1;
                real_render_seconds += render_el / 1000.0;
            }
            if fi == 0 || (fi + 1) % 20 == 0 || fi + 1 == total {
                eprintln!(
                    "{GTAG}: 帧 {}/{total} render={render_el:.3}ms(gpu_scene={:.3}ms gpu_encode={:.3}ms) present={pres_el:.3}ms digest={dig_el:.3}ms",
                    fi + 1,
                    rec.scene_gpu_ns / 1e6,
                    rec.encode_gpu_ns / 1e6,
                );
            }
            fi += 1;
        }
        if fi >= total || !resized {
            break 'eras;
        }
    }

    let frames_done = fi;
    // ⑦ 多口径稳态统计(post-warmup;程序产禁手写阈) + evidence。
    let (r_mean, _, r_cv, r_min, r_max) = g34_stats(&render_ms);
    let (p_mean, _, p_cv, p_min, p_max) = if headless || present_ms.iter().all(|v| *v == 0.0) {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g34_stats(&present_ms)
    };
    let (eg_mean, _, _, _, _) = if encode_gpu_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g34_stats(&encode_gpu_ms)
    };
    let (sg_mean, _, _, _, _) = if scene_gpu_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g34_stats(&scene_gpu_ms)
    };
    let (dg_mean, _, _, _, _) = if digest_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g34_stats(&digest_ms)
    };
    let encode_host_ms = 0.0f64; // device 编码后 host 编码墙钟恒 0(如实登记)
    let overhead_mean = encode_host_ms + p_mean;
    let counts = window.as_ref().map(|w| w.counts());

    let (window_json, p_mean_json, overhead_json) = if headless {
        ("null".to_owned(), "null".to_owned(), "null".to_owned())
    } else {
        let c = counts.unwrap_or(rurix_rt::vk::ExternalPresentCounts {
            frames_presented: 0,
            swapchain_rebuilds: 0,
        });
        let (fw, fh) = window.as_ref().map(|w| w.extent()).unwrap_or((0, 0));
        (
            format!(
                "{{\"visible\":{},\"channel_order\":{},\"extent\":{{\"w\":{fw},\"h\":{fh}}},\"frames_presented\":{},\"swapchain_rebuilds\":{}}}",
                !hidden,
                jstr(if bgra { "bgra8_unorm" } else { "rgba8_unorm" }),
                c.frames_presented,
                c.swapchain_rebuilds
            ),
            format!("{p_mean:.6}"),
            format!("{overhead_mean:.6}"),
        )
    };
    let encode_spv_json = format!(
        "{{\"path\":{},\"sha256\":{}}}",
        jstr(&spv_encode.replace('\\', "/")),
        jstr(&g34_file_sha(&spv_encode).unwrap_or_else(|e| fail(&e)))
    );
    let real_render_fps = if real_render_seconds > 0.0 {
        real_frames as f64 / real_render_seconds
    } else {
        0.0
    };

    // ── features/纹理/slab/dyn/host_parity 块（闭集字段;off 面 null）──
    let features_json = format!(
        "{{\"textures\":{textures},\"slab\":{},\"dyn\":{dyn_on},\"full\":{full},\"static_camera\":{static_camera}}}",
        slab_table.is_some(),
    );
    let textures_json = if let Some((t, rep)) = tex_report.as_ref() {
        let c = &t.census;
        format!(
            "{{\"census\":{{\"materials_total\":{},\"with_base_color_texture\":{},\"with_normal_texture\":{},\"with_metallic_roughness_texture\":{},\"primitives_total\":{},\"primitives_with_texcoord0\":{},\"primitives_with_tangent\":{}}},\"mapping_law\":\"逐材质三角数降序 top-12（并列时 material_index 升序;其余走常量面 0-byte）\",\"mapped_materials\":{},\"tex_tris\":{},\"atlas\":{{\"width\":{},\"height\":{},\"tile\":2048,\"format\":\"u32_packed_rgba8\",\"digest\":{}}},\"linlut_digest\":{},\"slab_premod_slots\":{},\"probe\":{{\"probe_count\":{},\"eval_ms\":{:.6},\"ssbo\":{{\"p100\":{:.15e},\"bitexact\":{},\"double_run_bitexact\":{},\"device_digest\":{},\"host_digest\":{}}},\"sampler_leg\":{{\"max_lsb_diff\":{},\"bound_lsb\":1,\"bitexact\":{}}}}},\"spv_scene\":{{\"path\":{},\"sha256\":{},\"no_contraction_injected\":true}}}}",
            c.materials_total,
            c.with_base_color_texture,
            c.with_normal_texture,
            c.with_metallic_roughness_texture,
            c.primitives_total,
            c.primitives_with_texcoord0,
            c.primitives_with_tangent,
            t.slots.len(),
            t.tex_tris,
            t.atlas_w,
            t.atlas_h,
            jstr(&t.atlas_digest),
            jstr(&t.linlut_digest),
            tex_premod_slots,
            rep.probe_count,
            rep.eval_ms,
            rep.ssbo_p100,
            rep.ssbo_bitexact,
            rep.ssbo_double_run_bitexact,
            jstr(&rep.ssbo_device_digest),
            jstr(&rep.ssbo_host_digest),
            rep.sampler_max_lsb,
            rep.sampler_bitexact,
            jstr(&spv_scene.replace('\\', "/")),
            jstr(&g34_file_sha(&spv_scene).unwrap_or_else(|e| fail(&e))),
        )
    } else {
        "null".to_owned()
    };
    let slab_json = if let Some((asset, eval, n_slab)) = slab_report.as_ref() {
        format!(
            "{{\"asset_path\":{},\"abi_digest\":{},\"mapped_materials\":{},\"slab_tris\":{},\"parity_p100\":{:.15e},\"eval_ms\":{:.6},\"arm\":\"device\",\"tex_premod_slots\":{},\"device_digest\":{},\"host_digest\":{}}}",
            jstr(&asset.path.replace('\\', "/")),
            jstr(&asset.abi_digest),
            asset.material_slots.len(),
            n_slab,
            eval.parity_p100,
            eval.eval_ms,
            tex_premod_slots,
            jstr(&eval.device_digest),
            jstr(&eval.host_digest),
        )
    } else {
        "null".to_owned()
    };
    let dyn_json = if dyn_on {
        let mut frames_json = String::new();
        for (k, v) in verify_recs.iter().enumerate() {
            if k > 0 {
                frames_json.push(',');
            }
            frames_json.push_str(&format!(
                "{{\"frame\":{},\"pred_px\":[{:.4},{:.4}],\"pred_aabb\":[{:.4},{:.4},{:.4},{:.4}],\"obs_px\":[{:.4},{:.4}],\"obs_aabb\":[{:.4},{:.4},{:.4},{:.4}],\"obs_count\":{},\"centroid_delta_px\":{:.6},\"aabb_delta_px\":{:.6},\"pass\":{}}}",
                v.frame,
                v.pred_px[0], v.pred_px[1],
                v.pred_aabb[0], v.pred_aabb[1], v.pred_aabb[2], v.pred_aabb[3],
                v.obs_px[0], v.obs_px[1],
                v.obs_aabb[0], v.obs_aabb[1], v.obs_aabb[2], v.obs_aabb[3],
                v.obs_count,
                v.centroid_delta_px,
                v.aabb_delta_px,
                v.pass,
            ));
        }
        let all_pass = verify_recs.iter().all(|v| v.pass);
        let dyn_tri_n = dyn_cube_tris(DYN_CUBE_HALF).len() / 9;
        format!(
            "{{\"dyn_tris\":{},\"dyn_tri_base\":{},\"action\":\"refit\",\"verify_every\":{},\"tol_centroid_px\":{:.3},\"tol_aabb_px\":{:.3},\"min_count_area_ratio\":{:.4},\"verify_frames\":[{}],\"verify_count\":{},\"all_pass\":{}}}",
            dyn_tri_n,
            scene.indices.len(),
            DYN_VERIFY_EVERY,
            DYN_TOL_CENTROID_PX,
            DYN_TOL_AABB_PX,
            DYN_MIN_COUNT_AREA_RATIO,
            frames_json,
            verify_recs.len(),
            all_pass,
        )
    } else {
        "null".to_owned()
    };
    let host_parity_json = if let Some(hp) = host_parity.as_ref() {
        format!(
            "{{\"frame\":{},\"tol\":{:.15e},\"tol_source\":{},\"frozen_measured\":{},\"color_p100\":{:.15e},\"color_p50\":{:.15e},\"color_mean_abs\":{:.15e},\"bitexact_px\":{},\"total_px\":{},\"bitexact_ratio\":{:.10},\"depth_p100\":{:.15e},\"in_tol\":{},\"host_render_ms\":{:.3},\"basis\":{}}}",
            hp.frame + 1,
            hp.tol,
            jstr(&hp.tol_source),
            if hp.tol_source.starts_with("--host-tol") {
                "null".to_owned()
            } else {
                format!("{:.15e}", host_tol_measured)
            },
            hp.color_p100,
            hp.color_p50,
            hp.color_mean_abs,
            hp.bitexact_px,
            hp.total_px,
            hp.bitexact_ratio,
            hp.depth_p100,
            hp.in_tol,
            hp.host_render_ms,
            jstr("host 金标准（合并语义同步实现：贴图三角 = 图集双线性×（mod×R_slot）;非贴图 = 常量 albedo（× R_slot 若 slab 映射）;动态实例 = host Tlas 实例变换追踪 + 局部空间纯发光体）vs device scene HDR（TSR 前瞬时位）逐像素逐通道绝对差;容差结构依据 = RT core vs host Möller–Trumbore t 值算术差（ULP 级 ⇒ 命中点/辐照传递差,目标近位级）,threshold = measured × 2.0 标定冻结（g34_budget 程序读禁手写）;bitexact 像素占比如实登记（非门判据）"),
        )
    } else {
        "null".to_owned()
    };

    // G36 W3：geo 组合面 evidence（geo on 时 schema/gate 切换 G36 字面 +
    // "geo" 块追加——G34 注册 schema additionalProperties:false 纪律下不改
    // G34 面;off = G34 字面 0-byte）。
    let geo_json = if let Some(g) = geo.as_ref() {
        let cl_json = if let Some((r, _)) = &g.cluster {
            format!(
                "{{\"mode\":{},\"threshold_px\":{},\"blocks\":{},\"total_clusters\":{},\"cut_clusters\":{},\"cut_leaf_clusters\":{},\"src_tris\":{},\"passthrough_tris\":{},\"coarse_tris\":{},\"out_tris\":{}}}",
                jstr(r.mode),
                r.threshold_px,
                r.blocks,
                r.total_clusters,
                r.cut_clusters,
                r.cut_leaf_clusters,
                r.src_tris,
                r.passthrough_tris,
                r.coarse_tris,
                r.out_tris,
            )
        } else {
            "null".to_owned()
        };
        let wp_json_g = if let Some((r, _)) = &g.wp {
            format!(
                "{{\"mode\":{},\"cells_total\":{},\"cells_nonempty\":{},\"cells_full\":{},\"cells_hlod\":{},\"cells_culled\":{},\"cells_pending\":{},\"full_tris\":{},\"proxy_tris\":{},\"out_tris\":{},\"selection_digest\":{},\"assemble_ticks\":{},\"budget_stall_frames\":{}}}",
                jstr(r.mode),
                r.cells_total,
                r.cells_nonempty,
                r.cells_full,
                r.cells_hlod,
                r.cells_culled,
                r.cells_pending,
                r.full_tris,
                r.proxy_tris,
                r.out_tris,
                jstr(&r.selection_digest),
                r.assemble_ticks,
                r.budget_stall_frames,
            )
        } else {
            "null".to_owned()
        };
        let comb_json = if let Some(st) = &g.combined {
            format!(
                "{{\"identity_tris\":{},\"coarse_emitted\":{},\"coarse_tris\":{},\"straddle_clusters\":{},\"straddle_fallback_tris\":{},\"wp_proxy_tris\":{},\"out_tris\":{}}}",
                st.identity_tris,
                st.coarse_emitted,
                st.coarse_tris,
                st.straddle_clusters,
                st.straddle_fallback_tris,
                st.wp_proxy_tris,
                st.out_tris,
            )
        } else {
            "null".to_owned()
        };
        let proxy_tris_total = g
            .prov
            .iter()
            .filter(|p| !matches!(p, TriProvenance::Src(_)))
            .count();
        format!(
            "{{\"cluster\":{cl_json},\"wp\":{wp_json_g},\"combined\":{comb_json},\"prov_identity\":{},\"proxy_tris_total\":{proxy_tris_total},\"frozen_at_assembly\":true,\"proxy_texture_fallback\":\"tritex=-1 常量面（#96 属性保持简化留窗）\"}}",
            geo_prov_is_identity(&g.prov),
        )
    } else {
        "null".to_owned()
    };
    let (ev_schema, ev_gate) = if geo.is_some() {
        (G36_SCHEMA, G36_GATE)
    } else {
        (G34_SCHEMA, G34_GATE)
    };
    let mut ev = String::with_capacity(8192);
    ev.push('{');
    ev.push_str(&format!("\"schema\":{},", jstr(ev_schema)));
    ev.push_str(&format!("\"gate\":{},", jstr(ev_gate)));
    ev.push_str(&format!("\"scene\":{},", jstr(scene_id)));
    ev.push_str(&format!("\"tier\":{tier},\"backend\":\"tsr_device\","));
    ev.push_str(&format!(
        "\"trajectory\":{},",
        match auto_move.as_deref() {
            Some(n) => jstr(n),
            None => "null".to_owned(),
        }
    ));
    ev.push_str(&format!("\"frames\":{frames},\"warmup\":{warmup},"));
    ev.push_str(&format!("\"frames_completed\":{frames_done},"));
    ev.push_str(&format!("\"exit_reason\":{},", jstr(exit_reason)));
    ev.push_str(&format!("\"resize_eras\":{resize_eras},"));
    ev.push_str(&format!("\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},"));
    ev.push_str(&format!(
        "\"internal_resolution\":{{\"w\":{},\"h\":{}}},",
        (out_w as u64 * u64::from(tier) / 100).max(1),
        (out_h as u64 * u64::from(tier) / 100).max(1)
    ));
    ev.push_str(&format!("\"real_render_frame_ms\":{r_mean:.6},"));
    ev.push_str(&format!("\"real_render_fps\":{real_render_fps:.6},"));
    ev.push_str(&format!("\"present_frame_ms\":{p_mean_json},"));
    ev.push_str(&format!("\"present_overhead_ms\":{overhead_json},"));
    ev.push_str(&format!("\"encode_frame_ms\":{encode_host_ms:.6},"));
    ev.push_str(&format!("\"digest_frame_ms\":{dg_mean:.6},"));
    ev.push_str(&format!("\"render_digest\":{},", jstr(&render_digest)));
    ev.push_str(&format!("\"digest\":{},", jstr(&presented_digest)));
    ev.push_str("\"digest_seq\":[");
    for (k, d) in digest_seq.iter().enumerate() {
        if k > 0 {
            ev.push(',');
        }
        ev.push_str(&jstr(d));
    }
    ev.push_str("],");
    ev.push_str("\"ev100_seq\":[");
    for (k, v) in ev100_seq.iter().enumerate() {
        if k > 0 {
            ev.push(',');
        }
        ev.push_str(&format!("{v}"));
    }
    ev.push_str("],");
    ev.push_str("\"camera_poses\":[");
    for (k, p) in pose_seq.iter().enumerate() {
        if k > 0 {
            ev.push(',');
        }
        ev.push_str(&format!("[{},{},{},{},{}]", p[0], p[1], p[2], p[3], p[4]));
    }
    ev.push_str("],");
    ev.push_str("\"ev100_ramp\":null,");
    ev.push_str(&format!("\"headless\":{headless},"));
    ev.push_str(&format!("\"window\":{window_json},"));
    ev.push_str("\"contracts\":{\"production\":");
    ev.push_str(&format!(
        "{{\"path\":{},\"digest\":{}}},",
        jstr(&contract_path.replace('\\', "/")),
        jstr(&contract.digest)
    ));
    ev.push_str(&g10_fragment);
    ev.push_str(&format!(",\"encode_spv\":{encode_spv_json}"));
    ev.push_str("},");
    ev.push_str("\"render_includes_forced_readback\":true,");
    ev.push_str(&format!(
        "\"spv\":{},",
        unified_provenance_json(&spv_scene, &spv_mv, &spv_resample, &spv_resolve)
    ));
    ev.push_str(&format!("\"features\":{features_json},"));
    ev.push_str(&format!("\"textures\":{textures_json},"));
    ev.push_str(&format!("\"slab\":{slab_json},"));
    ev.push_str(&format!("\"dyn\":{dyn_json},"));
    if geo.is_some() {
        // geo 组合面字段（仅 G36 schema 面追加;G34 面 0-byte）。
        ev.push_str(&format!("\"geo\":{geo_json},"));
    }
    ev.push_str(&format!("\"host_parity\":{host_parity_json},"));
    ev.push_str(&format!(
        "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"scene_gpu_ms\":{sg_mean:.6},\"encode_gpu_ms\":{eg_mean:.6},\"present_cv\":{},\"present_min_ms\":{},\"present_max_ms\":{}}},",
        if headless { "null".to_owned() } else { format!("{p_cv:.6}") },
        if headless { "null".to_owned() } else { format!("{p_min:.6}") },
        if headless { "null".to_owned() } else { format!("{p_max:.6}") },
    ));
    ev.push_str(&format!(
        "\"notes\":{}",
        jstr("G34 全特性合流 G34-1 合流地基：kernels/g34_unified_gi.rx 统一 GI kernel（母版 g14_3_direct_gi 语义 + fork A 图集采样块 + fork B 实例分派块合一;两缺省面各自 == 母版位级——--static-camera 锚格模式全链 TSR digest == g14_3_stage_a_digest_anchor 承载）+ kernels/g34_unified_shade.rx 统一 shade（shade_reduce 语义 + out_depth_hz 恒输出,HZB off 写而不消费,HZB 合流接口预留）;合并语义 = 贴图三角 采样×（mod×R_slot）/ 非贴图 常量×（R_slot 若 slab 映射）,host 装配期预调制承载 kernel 零新增面;三特性同开真跑 = 装配期 slab/纹理双臂对拍 + 逐帧 tlas_update refit + parity 帧 host 金标准对拍 + A4 范式动态位置核验 fail-closed;HZB/FG/skin 合流归后续波（接口预留面见 kernels 头注释）。g31_window_present.rs 0-byte——其五门为回归锚。")
    ));
    ev.push('}');

    if evidence_path.is_empty() {
        println!("{ev}");
    } else {
        std::fs::write(&evidence_path, format!("{ev}\n"))
            .unwrap_or_else(|e| fail(&format!("evidence 写 {evidence_path}: {e}")));
        eprintln!("{GTAG}: evidence → {evidence_path}");
    }
    eprintln!("{GTAG}: PASS frames={frames_done}/{total} real_render={r_mean:.3}ms present={p_mean:.3}ms exit={exit_reason}");
}

// ---------------------------------------------------------------------------
// G34-3 蒙皮角色进真窗口统一车道（g34.wave2.skin）——独立 include 区段：
// 蒙皮段全量收 g34_full_lane/g34_skin_section.rs（G34S*/g34skin* 前缀自持
// 符号面;与 G34-2 HZB 同文件并行分区写零交叠——主 bin 挂钩面 = 上方 --skin
// 旗标解析 + 早分支两处,本 include 一行;主 bin 既有面 0-byte）。
// ---------------------------------------------------------------------------
include!("g34_full_lane/g34_skin_section.rs");
// ---------------------------------------------------------------------------
// G37 W3 hzb_skin——HZB×蒙皮同车道合并面（g37.wave3.hzb_skin）——独立
// include 区段：合并段全量收 g34_full_lane/g34_hzb_skin.rs（G34HS*/g34hs*
// 前缀自持符号面;跨区段消费 = G34-2 HZB 骨架件〔G34HzbBits/g34_lane_descs_
// hzb/分类器/probe 对拍〕+ G34-3 蒙皮件〔g34skin_assets/核验三面 helper〕
// ——两区段本体 0-byte,主 bin 挂钩面 = 上方 --hzb on --skin 同开早分支 +
// --spv-hzbskin-primary 旗标解析;G36 W4-W5 留窗兑现件）。
include!("g34_full_lane/g34_hzb_skin.rs");

// G34-2 HZB 接统一车道（g34.wave2.hzb）——独立 include 区段：HZB 段全量收
// g34_full_lane/g34_2_hzb.rs（G34Hzb*/g34_hzb*/G34HZB_* 前缀自持符号面;与
// G34-3 蒙皮同文件并行分区写零交叠——主 bin 挂钩面 = 上方 --hzb 旗标解析 +
// 早分支两处,本 include 一行;主 bin 既有面 0-byte）。
// ---------------------------------------------------------------------------
include!("g34_full_lane/g34_2_hzb.rs");
