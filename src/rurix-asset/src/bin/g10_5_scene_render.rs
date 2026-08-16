//! G10.5a 双端帧链路 Rurix 侧场景渲染 harness（RFC-0026 §4.6 双端确定性契约消费面；
//! spec/visual_comparison.md RXS-0384 canonical digest 单源布局 / RXS-0386 度量域契约；
//! spec/imageio.md RXS-0385 EXR 容器）。
//!
//! ## 职责闭集（G10.5 波 A 段——链路建设，零画质修复纪律）
//!
//! 1. **契约解析 + digest（RXS-0384 字节布局独立第三实现）**：严格闭集解析
//!    （四节全字段必填、schema 外字段 fail-closed、unit-norm 2^-40 谓词、
//!    NaN/Inf 拒绝），canonical preimage + SHA-256 与双端 Python 实现对拍
//!    （M130 `--phase g10.5` 门机核消费 `--contract-digest` 输出）。
//! 2. **真渲染出帧**：glTF 场景（rurix-asset gltf 模块提取几何 + 节点树变换
//!    组合 + 材质 baseColorFactor）→ rurix-render GI 管线（`render_gbuffer_pinhole`
//!    同口径针孔主射线 + `RayTracedRadiance` 直光+阴影射线 + 屏幕探针单反弹
//!    GI，seed = 契约 time.random_seed）→ scene-linear HDR EXR 捕获（image-io，
//!    元数据闭集 + capture_params_digest = 契约 digest）。
//! 3. **应用层探针投影（RXS-0390）**：契约参数 → f32 view_proj（与渲染同一路径）
//!    投影冻结标志物世界坐标 → 像素坐标 JSON，供双端 pixel_delta ≤ 1e-3 px 断言。
//! 4. **LDR 派生（RXS-0386 L2 派生链）**：HDR 帧 × 曝光尺度 → aces13 view
//!    transform（rurix-render display 插件真身）→ IEC 61966-2-1 sRGB 编码 →
//!    LDR 派生 EXR（派生链元数据互证齐备）。
//!
//! ## 诚实边界（差距清单候选，G10 零修复纪律——只登记不修复）
//!
//! - 材质子集 = 逐图元 baseColorFactor（Lambert）；baseColorTexture / 法线贴图 /
//!   metallic-roughness PBR 全项不采样（G10.3 已登记 DDS 纹理解码归后续波次）；
//! - 几何法线（winding 朝向、双面着色翻转），平滑法线不消费；
//! - 灯种子集 = 契约 sun + sky 常量天光；点/面光源与 glTF emissive 不表达；
//! - GI = 屏幕探针单反弹（host 参考管线），非 Lumen 等效宣称；
//! - JSON 整数解析经 i64（u64 顶格 seed 被 fail-closed 拒绝——本波契约 seed=42）。
//!
//! ## 用法
//!
//! ```text
//! g10_5_scene_render --contract-digest <params.json>
//! g10_5_scene_render --render --gltf <scene.gltf> --contract <params.json> \
//!     --out-dir <dir> --scene-id <id> [--exposure-scale <f64>]
//! g10_5_scene_render --project-landmarks --contract <params.json> --landmarks <landmarks.json>
//! g10_5_scene_render --derive-ldr --hdr <frame.exr> --source-end <rurix|ue5> \
//!     --out <ldr.exr> --exposure-scale <f64>
//! ```
//!
//! Assisted-by: Kimi-K3（G10.5a 波）

#![forbid(unsafe_code)]

use image_io::exr::{
    ChromaticitiesOrigin, DecodedExr, ExrBitDepth, ExrChannelLayout, ExrDerivation, ExrDomain,
    ExrImage, ExrMetadata, ExrSourceEnd, ExrTransfer, ExrViewTransform, decode_exr, encode_exr,
};
use rurix_asset::gltf::json::{self, JsonValue};
use rurix_asset::gltf::validate;
use rurix_render::display::aces13::Aces13;
use rurix_render::display::view_transform::ViewTransform;
use rurix_render::gi::pipeline::{GiParams, render_gi};
use rurix_render::gi::probe::GiCamera;
use rurix_render::gi::tracer::{GiMeshInstance, GiScene, RayTracedRadiance};
use rurix_render::rt::bvh::{Ray, Transform3x4, Vec3};
use rurix_render::rt::ref_tracer::RAY_EPS;
use rurix_render::temporal::image::ImageF32;
use std::path::{Path, PathBuf};

const TAG: &str = "G10_5_RENDER";

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn sha256_hex(data: &[u8]) -> String {
    rurix_pkg::sha256::hex_digest(data)
}

// ─────────────────────────── 契约解析（RXS-0384 独立第三实现） ───────────────────────────

const UNIT_NORM_TOL: f64 = 9.094947017729282e-13; // 2^-40（RXS-0384 L2 谓词常量）

#[derive(Debug, Clone)]
struct Contract {
    cam_position: [f64; 3],
    cam_quat: [f64; 4], // w,x,y,z
    fov_y_deg: f64,
    near: f64,
    far: f64,
    res_w: u32,
    res_h: u32,
    sun_direction: [f64; 3], // 传播方向（UE 惯例）
    sun_intensity_lux: f64,
    sun_color: [f64; 3],
    sky_intensity: f64,
    sky_cubemap_id: Option<String>,
    ev100: f64,
    fixed_dt_s: f64,
    warmup_frames: u32,
    capture_frame_index: u32,
    random_seed: u64,
    jitter_index_base: u32,
    jitter_scale: f64,
}

fn cerr(msg: impl Into<String>) -> String {
    format!("契约解析失败（fail-closed）: {}", msg.into())
}

fn as_f64(name: &str, v: &JsonValue) -> Result<f64, String> {
    match v {
        JsonValue::F64(f) => {
            if !f.is_finite() {
                return Err(cerr(format!("{name}: NaN/Inf forbidden")));
            }
            Ok(*f)
        }
        JsonValue::I64(i) => Ok(*i as f64),
        _ => Err(cerr(format!("{name}: expected f64"))),
    }
}

fn as_u(name: &str, v: &JsonValue, bits: u32) -> Result<u64, String> {
    match v {
        JsonValue::I64(i) if *i >= 0 && (*i as u128) < (1u128 << bits) => Ok(*i as u64),
        _ => Err(cerr(format!("{name}: expected u{bits}（i64 域内非负）"))),
    }
}

fn as_f64_arr(name: &str, v: &JsonValue, n: usize) -> Result<Vec<f64>, String> {
    let arr = v
        .as_array()
        .ok_or_else(|| cerr(format!("{name}: expected array")))?;
    if arr.len() != n {
        return Err(cerr(format!("{name}: expected f64[{n}]")));
    }
    arr.iter()
        .enumerate()
        .map(|(i, x)| as_f64(&format!("{name}[{i}]"), x))
        .collect()
}

fn obj_closed<'a>(
    name: &str,
    v: &'a JsonValue,
    keys: &[&str],
) -> Result<&'a [(String, JsonValue)], String> {
    let obj = v
        .as_object()
        .ok_or_else(|| cerr(format!("{name}: expected object")))?;
    for (k, _) in obj {
        if !keys.contains(&k.as_str()) {
            return Err(cerr(format!("{name}: schema 外字段 {k:?}")));
        }
    }
    for k in keys {
        if v.get(k).is_none() {
            return Err(cerr(format!("{name}: 缺字段 {k:?}")));
        }
    }
    Ok(obj)
}

fn parse_contract(text: &str) -> Result<Contract, String> {
    let root = json::parse_str(text).map_err(|e| cerr(format!("JSON: {e}")))?;
    obj_closed("root", &root, &["camera", "lighting", "time", "post"])?;

    let cam = root.get("camera").unwrap();
    obj_closed(
        "camera",
        cam,
        &[
            "position",
            "orientation_quat",
            "fov_y_deg",
            "near",
            "far",
            "resolution",
        ],
    )?;
    let pos = as_f64_arr("camera.position", cam.get("position").unwrap(), 3)?;
    let quat = as_f64_arr(
        "camera.orientation_quat",
        cam.get("orientation_quat").unwrap(),
        4,
    )?;
    let res = cam.get("resolution").unwrap();
    obj_closed("camera.resolution", res, &["w", "h"])?;

    let lighting = root.get("lighting").unwrap();
    obj_closed("lighting", lighting, &["sun", "sky", "exposure"])?;
    let sun = lighting.get("sun").unwrap();
    obj_closed(
        "lighting.sun",
        sun,
        &["direction", "intensity_lux", "color_linear_rgb"],
    )?;
    let sky = lighting.get("sky").unwrap();
    obj_closed("lighting.sky", sky, &["intensity", "cubemap_id"])?;
    let cubemap_id = match sky.get("cubemap_id").unwrap() {
        JsonValue::Null => None,
        JsonValue::String(s) => Some(s.clone()),
        _ => return Err(cerr("lighting.sky.cubemap_id: expected string|null")),
    };
    let exposure = lighting.get("exposure").unwrap();
    obj_closed("lighting.exposure", exposure, &["mode", "ev100"])?;
    match exposure.get("mode").unwrap() {
        JsonValue::String(s) if s == "manual" => {}
        _ => return Err(cerr("lighting.exposure.mode: v1 闭集仅 \"manual\"")),
    }

    let time = root.get("time").unwrap();
    obj_closed(
        "time",
        time,
        &[
            "fixed_dt_s",
            "warmup_frames",
            "capture_frame_index",
            "random_seed",
            "jitter",
        ],
    )?;
    let jitter = time.get("jitter").unwrap();
    obj_closed("time.jitter", jitter, &["sequence", "index_base", "scale"])?;
    match jitter.get("sequence").unwrap() {
        JsonValue::String(s) if s == "halton_2_3" => {}
        _ => return Err(cerr("time.jitter.sequence: v1 闭集仅 \"halton_2_3\"")),
    }

    let post = root.get("post").unwrap();
    obj_closed(
        "post",
        post,
        &["view_transform", "bloom", "vignette", "motion_blur", "dof"],
    )?;
    match post.get("view_transform").unwrap() {
        JsonValue::String(s) if s == "aces13" => {}
        _ => return Err(cerr("post.view_transform: v1 闭集仅 \"aces13\"")),
    }
    for k in ["bloom", "vignette", "motion_blur", "dof"] {
        match post.get(k).unwrap() {
            JsonValue::Bool(false) => {}
            _ => return Err(cerr(format!("post.{k}: v1 闭集仅 false"))),
        }
    }

    let c = Contract {
        cam_position: [pos[0], pos[1], pos[2]],
        cam_quat: [quat[0], quat[1], quat[2], quat[3]],
        fov_y_deg: as_f64("camera.fov_y_deg", cam.get("fov_y_deg").unwrap())?,
        near: as_f64("camera.near", cam.get("near").unwrap())?,
        far: as_f64("camera.far", cam.get("far").unwrap())?,
        res_w: as_u("camera.resolution.w", res.get("w").unwrap(), 32)? as u32,
        res_h: as_u("camera.resolution.h", res.get("h").unwrap(), 32)? as u32,
        sun_direction: {
            let d = as_f64_arr("lighting.sun.direction", sun.get("direction").unwrap(), 3)?;
            [d[0], d[1], d[2]]
        },
        sun_intensity_lux: as_f64(
            "lighting.sun.intensity_lux",
            sun.get("intensity_lux").unwrap(),
        )?,
        sun_color: {
            let rgb = as_f64_arr(
                "lighting.sun.color_linear_rgb",
                sun.get("color_linear_rgb").unwrap(),
                3,
            )?;
            [rgb[0], rgb[1], rgb[2]]
        },
        sky_intensity: as_f64("lighting.sky.intensity", sky.get("intensity").unwrap())?,
        sky_cubemap_id: cubemap_id,
        ev100: as_f64("lighting.exposure.ev100", exposure.get("ev100").unwrap())?,
        fixed_dt_s: as_f64("time.fixed_dt_s", time.get("fixed_dt_s").unwrap())?,
        warmup_frames: as_u("time.warmup_frames", time.get("warmup_frames").unwrap(), 32)? as u32,
        capture_frame_index: as_u(
            "time.capture_frame_index",
            time.get("capture_frame_index").unwrap(),
            32,
        )? as u32,
        random_seed: as_u("time.random_seed", time.get("random_seed").unwrap(), 64)?,
        jitter_index_base: as_u(
            "time.jitter.index_base",
            jitter.get("index_base").unwrap(),
            32,
        )? as u32,
        jitter_scale: as_f64("time.jitter.scale", jitter.get("scale").unwrap())?,
    };
    let q2: f64 = c.cam_quat.iter().map(|x| x * x).sum();
    if (q2 - 1.0).abs() > UNIT_NORM_TOL {
        return Err(cerr(
            "camera.orientation_quat: unit-norm 违例（|q²−1| > 2^-40）",
        ));
    }
    let d2: f64 = c.sun_direction.iter().map(|x| x * x).sum();
    if (d2 - 1.0).abs() > UNIT_NORM_TOL {
        return Err(cerr(
            "lighting.sun.direction: unit-norm 违例（|d²−1| > 2^-40）",
        ));
    }
    Ok(c)
}

// ─────────────────── canonical preimage（RXS-0384 L3 字节布局逐字） ───────────────────

const TAG_F64: u8 = 0x01;
const TAG_U32: u8 = 0x02;
const TAG_U64: u8 = 0x03;
const TAG_STR: u8 = 0x04;
const TAG_BOOL: u8 = 0x05;
const TAG_NULL: u8 = 0x06;
const TAG_OBJ_BEGIN: u8 = 0x07;
const TAG_OBJ_END: u8 = 0x08;
const TAG_ARR_BEGIN: u8 = 0x09;
const TAG_ARR_END: u8 = 0x0a;

fn enc_key(buf: &mut Vec<u8>, k: &str) {
    buf.extend_from_slice(&(k.len() as u32).to_le_bytes());
    buf.extend_from_slice(k.as_bytes());
}
fn enc_f64(buf: &mut Vec<u8>, v: f64) {
    buf.push(TAG_F64);
    buf.extend_from_slice(&v.to_le_bytes());
}
fn enc_arr3(buf: &mut Vec<u8>, v: &[f64; 3]) {
    buf.push(TAG_ARR_BEGIN);
    for x in v {
        enc_f64(buf, *x);
    }
    buf.push(TAG_ARR_END);
}

/// canonical preimage（键序 = Unicode code point 序，逐节显式展开——与
/// ci 双 Python 实现同字面；布局漂移即 digest 不等，M130 门检出）。
fn canonical_preimage(c: &Contract) -> Vec<u8> {
    let mut buf = b"G10DCP-1\x00".to_vec();
    buf.push(TAG_OBJ_BEGIN);
    // camera（far < fov_y_deg < near < orientation_quat < position < resolution）
    enc_key(&mut buf, "camera");
    buf.push(TAG_OBJ_BEGIN);
    enc_key(&mut buf, "far");
    enc_f64(&mut buf, c.far);
    enc_key(&mut buf, "fov_y_deg");
    enc_f64(&mut buf, c.fov_y_deg);
    enc_key(&mut buf, "near");
    enc_f64(&mut buf, c.near);
    enc_key(&mut buf, "orientation_quat");
    buf.push(TAG_ARR_BEGIN);
    for x in c.cam_quat {
        enc_f64(&mut buf, x);
    }
    buf.push(TAG_ARR_END);
    enc_key(&mut buf, "position");
    enc_arr3(&mut buf, &c.cam_position);
    enc_key(&mut buf, "resolution");
    buf.push(TAG_OBJ_BEGIN);
    enc_key(&mut buf, "h");
    buf.push(TAG_U32);
    buf.extend_from_slice(&c.res_h.to_le_bytes());
    enc_key(&mut buf, "w");
    buf.push(TAG_U32);
    buf.extend_from_slice(&c.res_w.to_le_bytes());
    buf.push(TAG_OBJ_END);
    buf.push(TAG_OBJ_END);
    // lighting（exposure < sky < sun）
    enc_key(&mut buf, "lighting");
    buf.push(TAG_OBJ_BEGIN);
    enc_key(&mut buf, "exposure");
    buf.push(TAG_OBJ_BEGIN);
    enc_key(&mut buf, "ev100");
    enc_f64(&mut buf, c.ev100);
    enc_key(&mut buf, "mode");
    buf.push(TAG_STR);
    enc_key(&mut buf, "manual");
    buf.push(TAG_OBJ_END);
    enc_key(&mut buf, "sky");
    buf.push(TAG_OBJ_BEGIN);
    enc_key(&mut buf, "cubemap_id");
    match &c.sky_cubemap_id {
        None => buf.push(TAG_NULL),
        Some(s) => {
            buf.push(TAG_STR);
            enc_key(&mut buf, s);
        }
    }
    enc_key(&mut buf, "intensity");
    enc_f64(&mut buf, c.sky_intensity);
    buf.push(TAG_OBJ_END);
    enc_key(&mut buf, "sun");
    buf.push(TAG_OBJ_BEGIN);
    enc_key(&mut buf, "color_linear_rgb");
    enc_arr3(&mut buf, &c.sun_color);
    enc_key(&mut buf, "direction");
    enc_arr3(&mut buf, &c.sun_direction);
    enc_key(&mut buf, "intensity_lux");
    enc_f64(&mut buf, c.sun_intensity_lux);
    buf.push(TAG_OBJ_END);
    buf.push(TAG_OBJ_END);
    // post（bloom < dof < motion_blur < view_transform < vignette）
    enc_key(&mut buf, "post");
    buf.push(TAG_OBJ_BEGIN);
    for k in ["bloom", "dof", "motion_blur"] {
        enc_key(&mut buf, k);
        buf.push(TAG_BOOL);
        buf.push(0x00);
    }
    enc_key(&mut buf, "view_transform");
    buf.push(TAG_STR);
    enc_key(&mut buf, "aces13");
    enc_key(&mut buf, "vignette");
    buf.push(TAG_BOOL);
    buf.push(0x00);
    buf.push(TAG_OBJ_END);
    // time（capture_frame_index < fixed_dt_s < jitter < random_seed < warmup_frames）
    enc_key(&mut buf, "time");
    buf.push(TAG_OBJ_BEGIN);
    enc_key(&mut buf, "capture_frame_index");
    buf.push(TAG_U32);
    buf.extend_from_slice(&c.capture_frame_index.to_le_bytes());
    enc_key(&mut buf, "fixed_dt_s");
    enc_f64(&mut buf, c.fixed_dt_s);
    enc_key(&mut buf, "jitter");
    buf.push(TAG_OBJ_BEGIN);
    enc_key(&mut buf, "index_base");
    buf.push(TAG_U32);
    buf.extend_from_slice(&c.jitter_index_base.to_le_bytes());
    enc_key(&mut buf, "scale");
    enc_f64(&mut buf, c.jitter_scale);
    enc_key(&mut buf, "sequence");
    buf.push(TAG_STR);
    enc_key(&mut buf, "halton_2_3");
    buf.push(TAG_OBJ_END);
    enc_key(&mut buf, "random_seed");
    buf.push(TAG_U64);
    buf.extend_from_slice(&c.random_seed.to_le_bytes());
    enc_key(&mut buf, "warmup_frames");
    buf.push(TAG_U32);
    buf.extend_from_slice(&c.warmup_frames.to_le_bytes());
    buf.push(TAG_OBJ_END);
    buf.push(TAG_OBJ_END);
    buf
}

fn param_digest(c: &Contract) -> String {
    sha256_hex(&canonical_preimage(c))
}

// ─────────────────────────── 相机（契约 → f32 view_proj，渲染/探针同一路径） ───────────────────────────

/// 主动旋转 v' = q·v·q*（f64；契约四元数 w,x,y,z）。
fn quat_rotate(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let [w, x, y, z] = q;
    let uv = [
        y * v[2] - z * v[1],
        z * v[0] - x * v[2],
        x * v[1] - y * v[0],
    ];
    let uuv = [
        y * uv[2] - z * uv[1],
        z * uv[0] - x * uv[2],
        x * uv[1] - y * uv[0],
    ];
    [
        v[0] + 2.0 * (w * uv[0] + uuv[0]),
        v[1] + 2.0 * (w * uv[1] + uuv[1]),
        v[2] + 2.0 * (w * uv[2] + uuv[2]),
    ]
}

fn f32v(v: [f64; 3]) -> [f32; 3] {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

/// 契约相机 → GiCamera（view = look_at_rh 等价基底：f = q·(0,0,−1)、up = q·(0,1,0)）。
fn contract_camera(c: &Contract) -> GiCamera {
    let fwd = quat_rotate(c.cam_quat, [0.0, 0.0, -1.0]);
    let up = quat_rotate(c.cam_quat, [0.0, 1.0, 0.0]);
    let eye = f32v(c.cam_position);
    let f = f32v(fwd);
    let center = [eye[0] + f[0], eye[1] + f[1], eye[2] + f[2]];
    let aspect = c.res_w as f64 / c.res_h as f64;
    let proj = rurix_render::temporal::common::perspective_rh_zo(
        (c.fov_y_deg.to_radians()) as f32,
        aspect as f32,
        c.near as f32,
        c.far as f32,
    );
    let view = rurix_render::temporal::common::look_at_rh(eye, center, f32v(up));
    GiCamera::new(proj.mul(&view))
}

// ─────────────────────────── glTF 场景装载（几何 + 节点树 + 材质子集） ───────────────────────────

fn json_f64(v: &JsonValue) -> Option<f64> {
    match v {
        JsonValue::F64(f) => Some(*f),
        JsonValue::I64(i) => Some(*i as f64),
        _ => None,
    }
}

fn json_f32_arr(v: &JsonValue, n: usize) -> Option<Vec<f32>> {
    let arr = v.as_array()?;
    if arr.len() != n {
        return None;
    }
    arr.iter()
        .map(json_f64)
        .map(|x| x.map(|f| f as f32))
        .collect()
}

/// 4×4 行主（f64 组合，落定转 f32 Transform3x4）。
type M4 = [[f64; 4]; 4];

fn m4_identity() -> M4 {
    let mut m = [[0.0f64; 4]; 4];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

fn m4_mul(a: &M4, b: &M4) -> M4 {
    let mut out = [[0.0f64; 4]; 4];
    for (r, orow) in out.iter_mut().enumerate() {
        for (cc, o) in orow.iter_mut().enumerate() {
            *o = (0..4).map(|k| a[r][k] * b[k][cc]).sum();
        }
    }
    out
}

fn quat_to_m4(q: &[f64; 4]) -> M4 {
    // glTF 四元数 = (x,y,z,w) 序——调用方先换算。
    let [x, y, z, w] = [q[0], q[1], q[2], q[3]];
    let mut m = m4_identity();
    m[0][0] = 1.0 - 2.0 * (y * y + z * z);
    m[0][1] = 2.0 * (x * y - z * w);
    m[0][2] = 2.0 * (x * z + y * w);
    m[1][0] = 2.0 * (x * y + z * w);
    m[1][1] = 1.0 - 2.0 * (x * x + z * z);
    m[1][2] = 2.0 * (y * z - x * w);
    m[2][0] = 2.0 * (x * z - y * w);
    m[2][1] = 2.0 * (y * z + x * w);
    m[2][2] = 1.0 - 2.0 * (x * x + y * y);
    m
}

fn node_local_m4(node: &JsonValue) -> Result<M4, String> {
    if let Some(mv) = node.get("matrix") {
        let a = mv.as_array().ok_or_else(|| cerr("node.matrix 非数组"))?;
        if a.len() != 16 {
            return Err(cerr("node.matrix 长度 ≠ 16"));
        }
        // glTF matrix = 列主序。
        let mut m = m4_identity();
        for r in 0..4 {
            for cc in 0..4 {
                m[r][cc] = json_f64(&a[cc * 4 + r]).ok_or_else(|| cerr("node.matrix 非数值"))?;
            }
        }
        return Ok(m);
    }
    let t = node
        .get("translation")
        .and_then(|v| json_f64_arr3(v))
        .unwrap_or([0.0, 0.0, 0.0]);
    let q = node
        .get("rotation")
        .and_then(|v| json_f64_arr4(v))
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let s = node
        .get("scale")
        .and_then(|v| json_f64_arr3(v))
        .unwrap_or([1.0, 1.0, 1.0]);
    let mut m = quat_to_m4(&q);
    for r in 0..3 {
        for cc in 0..3 {
            m[r][cc] *= s[cc];
        }
        m[r][3] = t[r];
    }
    Ok(m)
}

fn json_f64_arr3(v: &JsonValue) -> Option<[f64; 3]> {
    let a = v.as_array()?;
    if a.len() != 3 {
        return None;
    }
    Some([json_f64(&a[0])?, json_f64(&a[1])?, json_f64(&a[2])?])
}

fn json_f64_arr4(v: &JsonValue) -> Option<[f64; 4]> {
    let a = v.as_array()?;
    if a.len() != 4 {
        return None;
    }
    Some([
        json_f64(&a[0])?,
        json_f64(&a[1])?,
        json_f64(&a[2])?,
        json_f64(&a[3])?,
    ])
}

struct SceneLoad {
    instances: Vec<GiMeshInstance>,
    primitive_count: usize,
    triangle_count: usize,
    material_count: usize,
}

fn load_gltf_scene(path: &Path) -> Result<SceneLoad, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("glTF 读取失败: {e}"))?;
    let root = json::parse_str(&text).map_err(|e| format!("glTF JSON: {e}"))?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    // buffers（外部 URI 闭集；GLB 本 harness 不消费——G10.5 语料均为 .gltf+bin）
    let mut buffers: Vec<Vec<u8>> = Vec::new();
    for b in root
        .get("buffers")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let uri = b
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| cerr("buffer 缺 uri（GLB/内嵌不消费）"))?;
        let data =
            std::fs::read(base.join(uri)).map_err(|e| format!("buffer {uri} 读取失败: {e}"))?;
        buffers.push(data);
    }
    // 几何提取（单一事实源 = rurix-asset gltf::validate::extract_meshes）
    let meshes =
        validate::extract_meshes(&root, &buffers).map_err(|e| format!("extract_meshes: {e}"))?;

    // 材质子集：baseColorFactor rgb（纹理/法线/mr 不采样——诚实边界见头注）
    let mut mat_albedo: Vec<[f32; 3]> = Vec::new();
    if let Some(mats) = root.get("materials").and_then(|v| v.as_array()) {
        for m in mats {
            let alb = m
                .get("pbrMetallicRoughness")
                .and_then(|p| p.get("baseColorFactor"))
                .and_then(|v| json_f32_arr(v, 4))
                .map(|v| [v[0], v[1], v[2]])
                .unwrap_or([1.0, 1.0, 1.0]);
            mat_albedo.push(alb);
        }
    }

    // 图元 → 材质索引（(mesh_id, primitive_id) → material；primitive_id 与
    // extract_meshes 的全局递增计数器同口径——逐图元递增、含非 TRIANGLES 跳过项）
    let mut prim_material: std::collections::HashMap<(u32, u32), Option<u32>> =
        std::collections::HashMap::new();
    let mut prim_global: u32 = 0;
    if let Some(ms) = root.get("meshes").and_then(|v| v.as_array()) {
        for (mi, m) in ms.iter().enumerate() {
            if let Some(prims) = m.get("primitives").and_then(|v| v.as_array()) {
                for p in prims.iter() {
                    let mat = p.get("material").and_then(|v| v.as_u32());
                    prim_material.insert((mi as u32, prim_global), mat);
                    prim_global += 1;
                }
            }
        }
    }

    // 节点树世界变换（scene 0 根向下组合）
    let nodes = root
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| cerr("glTF 缺 nodes"))?;
    let mut world: Vec<Option<M4>> = vec![None; nodes.len()];
    fn compose(idx: usize, nodes: &[JsonValue], world: &mut [Option<M4>]) -> Result<M4, String> {
        if let Some(m) = world[idx] {
            return Ok(m);
        }
        let local = node_local_m4(&nodes[idx])?;
        // 找父（O(n²) 可接受：节点千级）
        let mut parent = None;
        for (i, n) in nodes.iter().enumerate() {
            if let Some(ch) = n.get("children").and_then(|v| v.as_array()) {
                if ch.iter().any(|c| c.as_u32() == Some(idx as u32)) {
                    parent = Some(i);
                    break;
                }
            }
        }
        let w = match parent {
            Some(p) => m4_mul(&compose(p, nodes, world)?, &local),
            None => local,
        };
        world[idx] = Some(w);
        Ok(w)
    }
    for i in 0..nodes.len() {
        compose(i, nodes, &mut world)?;
    }

    let mut instances = Vec::new();
    let mut tri_total = 0usize;
    for m in &meshes {
        // 该 mesh 被哪些节点引用（一网格多实例按节点各出一份实例）
        for (ni, n) in nodes.iter().enumerate() {
            let Some(mesh_idx) = n.get("mesh").and_then(|v| v.as_u32()) else {
                continue;
            };
            if mesh_idx != m.mesh_id {
                continue;
            }
            let w = world[ni].ok_or_else(|| cerr("节点世界变换缺失"))?;
            let t = Transform3x4::from_rows([
                w[0][0] as f32,
                w[0][1] as f32,
                w[0][2] as f32,
                w[0][3] as f32,
                w[1][0] as f32,
                w[1][1] as f32,
                w[1][2] as f32,
                w[1][3] as f32,
                w[2][0] as f32,
                w[2][1] as f32,
                w[2][2] as f32,
                w[2][3] as f32,
            ]);
            let albedo = prim_material
                .get(&(m.mesh_id, m.primitive_id))
                .copied()
                .flatten()
                .and_then(|mi| mat_albedo.get(mi as usize).copied())
                .unwrap_or([1.0, 1.0, 1.0]);
            let mut indices = Vec::with_capacity(m.indices.len() / 3);
            for t3 in m.indices.chunks_exact(3) {
                indices.push([t3[0], t3[1], t3[2]]);
            }
            tri_total += indices.len();
            instances.push(GiMeshInstance {
                positions: m.positions.clone(),
                indices,
                transform: t,
                albedo,
            });
        }
    }
    Ok(SceneLoad {
        primitive_count: instances.len(),
        triangle_count: tri_total,
        material_count: mat_albedo.len(),
        instances,
    })
}

// ─────────────────────────── 渲染主路径 ───────────────────────────

/// 帧像素内容 digest（"G10EXRD-1" 布局，与 g10_m134_frame_capture / ci g10_exr_lib 同字面）。
fn frame_content_digest(width: u32, height: u32, channels: u8, pixels: &[f32]) -> String {
    let mut payload = b"G10EXRD-1\0".to_vec();
    payload.extend_from_slice(&width.to_le_bytes());
    payload.extend_from_slice(&height.to_le_bytes());
    payload.push(channels);
    for v in pixels {
        payload.extend_from_slice(&v.to_le_bytes());
    }
    format!("sha256:{}", sha256_hex(&payload))
}

fn hdr_metadata(width: u32, height: u32, digest: &str) -> ExrMetadata {
    let _ = (width, height);
    ExrMetadata {
        schema_version: "1".to_owned(),
        domain: ExrDomain::SceneLinearHdr,
        transfer: ExrTransfer::Linear,
        bit_depth: ExrBitDepth::Float32,
        source_end: ExrSourceEnd::Rurix,
        view_transform: None,
        capture_params_digest: format!("sha256:{digest}"),
        derivation: ExrDerivation::Capture,
        source_frame_digest: None,
        chromaticities_origin: Some(ChromaticitiesOrigin::Writer),
    }
}

struct RenderOut {
    pixels: Vec<f32>,
    covered: usize,
}

/// 主渲染：针孔主射线（render_gbuffer_pinhole 同口径）+ 直光/阴影（tracer 同公式）
/// + 屏幕探针单反弹 GI（seed = 契约 random_seed）。返回 scene-linear HDR RGB。
fn render_frame(scene: &GiScene, camera: &GiCamera, c: &Contract) -> RenderOut {
    let (w, h) = (c.res_w, c.res_h);
    // 契约：sun.direction = 传播方向；GiScene.sun_dir = 指向光源（= −direction）。
    let sun_toward = Vec3::from_array(normalize3([
        -c.sun_direction[0] as f32,
        -c.sun_direction[1] as f32,
        -c.sun_direction[2] as f32,
    ]));
    let sun_color = [
        (c.sun_color[0] * c.sun_intensity_lux) as f32,
        (c.sun_color[1] * c.sun_intensity_lux) as f32,
        (c.sun_color[2] * c.sun_intensity_lux) as f32,
    ];

    let mut depth = ImageF32::new(w, h, 1);
    let mut normals = ImageF32::new(w, h, 3);
    let mut albedo_px = vec![[0.0f32; 3]; (w * h) as usize];
    let mut direct = vec![[0.0f32; 3]; (w * h) as usize];
    let unproject = |nx: f32, ny: f32, z: f32| -> Option<Vec3> {
        let v4 = camera.inv_view_proj.transform_vec4([nx, ny, z, 1.0]);
        if !v4[3].is_finite() || v4[3].abs() < 1e-8 {
            return None;
        }
        Some(Vec3::new(v4[0] / v4[3], v4[1] / v4[3], v4[2] / v4[3]))
    };
    let mut covered = 0usize;
    for y in 0..h {
        for x in 0..w {
            let u = (x as f32 + 0.5) / w as f32;
            let v = (y as f32 + 0.5) / h as f32;
            let (nx, ny) = (2.0 * u - 1.0, 1.0 - 2.0 * v);
            let (Some(p0), Some(p1)) = (unproject(nx, ny, 0.0), unproject(nx, ny, 1.0)) else {
                depth.set(x, y, 0, 1.0);
                continue;
            };
            let dir = (p1 - p0).normalize();
            let idx = (y * w + x) as usize;
            let Some(hit) = scene
                .tlas
                .intersect(&scene.blases, &Ray { origin: p0, dir })
            else {
                depth.set(x, y, 0, 1.0);
                continue;
            };
            let p = p0 + dir * hit.t;
            let clip = camera.view_proj.transform_vec4([p.x, p.y, p.z, 1.0]);
            if clip[3] <= 1e-8 {
                depth.set(x, y, 0, 1.0);
                continue;
            }
            depth.set(x, y, 0, clip[2] / clip[3]);
            // 双面着色：法线翻转朝向入射光线来向（tracer.rs 同口径）。
            let mut n = Vec3::from_array(hit.normal);
            if n.dot(dir) > 0.0 {
                n = -n;
            }
            let n = n.normalize();
            normals.set_pixel3(x, y, n.to_array());
            let albedo = scene.albedos[hit.instance as usize];
            albedo_px[idx] = albedo;
            covered += 1;
            // 直光（gi/tracer.rs RadianceTracer::trace 命中分支同公式：
            // sun_color·ndl·albedo/π·太阳可见性；阴影射线原点沿法线偏移 RAY_EPS）。
            let ndl = n.dot(sun_toward).max(0.0);
            if ndl > 0.0 && c.sun_intensity_lux > 0.0 {
                let shadow = Ray {
                    origin: p + n * RAY_EPS,
                    dir: sun_toward,
                };
                if !scene.tlas.any_hit(&scene.blases, &shadow, f32::INFINITY) {
                    let inv_pi = 1.0 / core::f32::consts::PI;
                    for ch in 0..3 {
                        direct[idx][ch] = sun_color[ch] * ndl * albedo[ch] * inv_pi;
                    }
                }
            }
        }
    }

    // GI 单反弹（host 参考管线；seed = 契约 random_seed；temporal off 单帧口径）。
    let tracer = RayTracedRadiance::new(scene.clone());
    let gi_params = GiParams {
        seed: c.random_seed,
        temporal: false,
        ..GiParams::default()
    };
    let gi = render_gi(&depth, &normals, camera, &tracer, None, None, &gi_params);

    let inv_pi = 1.0 / core::f32::consts::PI;
    let mut pixels = vec![0.0f32; (w * h * 3) as usize];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if depth.get(x, y, 0) >= 1.0 {
                continue; // 主射线未命中 = 黑（UE 侧无天空网格同口径）
            }
            let gi_e = gi.irradiance.pixel3(x, y);
            for ch in 0..3 {
                pixels[idx * 3 + ch] = direct[idx][ch] + albedo_px[idx][ch] * inv_pi * gi_e[ch];
            }
        }
    }
    RenderOut { pixels, covered }
}

// ─────────────────────────── LDR 派生（RXS-0386 L2） ───────────────────────────

/// IEC 61966-2-1 sRGB 编码（host 侧单源编码步骤；f64 域）。
fn srgb_encode(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn derive_ldr(
    hdr: &DecodedExr,
    exposure_scale: f64,
    params_digest: &str,
    out_path: &Path,
) -> Result<String, String> {
    let aces = Aces13::new();
    let mut out = Vec::with_capacity(hdr.pixels.len());
    for px in hdr.pixels.chunks_exact(3) {
        let lin = [
            f64::from(px[0]) * exposure_scale,
            f64::from(px[1]) * exposure_scale,
            f64::from(px[2]) * exposure_scale,
        ];
        let disp = aces.to_display_linear(lin);
        for ch in disp {
            out.push(srgb_encode(ch.clamp(0.0, 1.0)) as f32);
        }
    }
    let src_digest = frame_content_digest(hdr.width, hdr.height, 3, &hdr.pixels);
    let md = ExrMetadata {
        schema_version: "1".to_owned(),
        domain: ExrDomain::DisplayReferredLdr,
        transfer: ExrTransfer::Srgb,
        bit_depth: ExrBitDepth::Float32,
        source_end: ExrSourceEnd::Rurix,
        view_transform: Some(ExrViewTransform::Aces13),
        capture_params_digest: format!("sha256:{params_digest}"),
        derivation: ExrDerivation::DerivedHostSrgbEncoderV1,
        source_frame_digest: Some(src_digest.clone()),
        chromaticities_origin: Some(ChromaticitiesOrigin::Writer),
    };
    let img = ExrImage::new(hdr.width, hdr.height, ExrChannelLayout::Rgb, out, md)
        .map_err(|e| format!("LDR 帧构造失败: {e}"))?;
    let bytes = encode_exr(&img).map_err(|e| format!("LDR 帧编码失败: {e}"))?;
    std::fs::write(out_path, &bytes).map_err(|e| format!("LDR 帧落盘失败: {e}"))?;
    Ok(src_digest)
}

// ─────────────────────────── CLI ───────────────────────────

fn take_arg(args: &[String], i: &mut usize) -> String {
    *i += 1;
    args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        fail("缺模式参数（--contract-digest / --render / --project-landmarks / --derive-ldr）");
    }

    if args.iter().any(|a| a == "--contract-digest") {
        let mut path = None;
        let mut i = 0;
        while i < args.len() {
            if args[i] == "--contract-digest" {
                path = Some(take_arg(&args, &mut i));
            }
            i += 1;
        }
        let text = std::fs::read_to_string(path.unwrap())
            .unwrap_or_else(|e| fail(&format!("契约参数读取失败: {e}")));
        match parse_contract(&text) {
            Ok(c) => {
                println!("param_digest_rust = {}", param_digest(&c));
                std::process::exit(0);
            }
            Err(e) => fail(&e),
        }
    }

    if args.iter().any(|a| a == "--project-landmarks") {
        let mut contract_path = None;
        let mut landmarks_path = None;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--contract" => contract_path = Some(take_arg(&args, &mut i)),
                "--landmarks" => landmarks_path = Some(take_arg(&args, &mut i)),
                _ => {}
            }
            i += 1;
        }
        let text = std::fs::read_to_string(contract_path.unwrap())
            .unwrap_or_else(|e| fail(&format!("契约参数读取失败: {e}")));
        let c = parse_contract(&text).unwrap_or_else(|e| fail(&e));
        let lm_text = std::fs::read_to_string(landmarks_path.unwrap())
            .unwrap_or_else(|e| fail(&format!("标志物读取失败: {e}")));
        let lm_root =
            json::parse_str(&lm_text).unwrap_or_else(|e| fail(&format!("标志物 JSON: {e}")));
        let camera = contract_camera(&c);
        let (w, h) = (c.res_w as f32, c.res_h as f32);
        let mut out = String::from("{\"pixels\":[");
        let lms = lm_root
            .get("landmarks")
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| fail("landmarks 缺数组"));
        for (li, lm) in lms.iter().enumerate() {
            let p = json_f64_arr3(lm).unwrap_or_else(|| fail("landmark 非 [x,y,z]"));
            let clip =
                camera
                    .view_proj
                    .transform_vec4([p[0] as f32, p[1] as f32, p[2] as f32, 1.0]);
            if li > 0 {
                out.push(',');
            }
            if clip[3] <= 1e-8 {
                out.push_str("null");
                continue;
            }
            let ndc_x = clip[0] / clip[3];
            let ndc_y = clip[1] / clip[3];
            let px = (ndc_x + 1.0) * 0.5 * w;
            let py = (1.0 - ndc_y) * 0.5 * h;
            out.push_str(&format!("[{px},{py}]"));
        }
        out.push_str("]}");
        println!("{out}");
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--derive-ldr") {
        let mut hdr_path = None;
        let mut out_path = None;
        let mut source_end = None;
        let mut exposure_scale = None;
        let mut params_digest = None;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--hdr" => hdr_path = Some(take_arg(&args, &mut i)),
                "--out" => out_path = Some(take_arg(&args, &mut i)),
                "--source-end" => source_end = Some(take_arg(&args, &mut i)),
                "--exposure-scale" => exposure_scale = Some(take_arg(&args, &mut i)),
                "--params-digest" => params_digest = Some(take_arg(&args, &mut i)),
                _ => {}
            }
            i += 1;
        }
        let end = match source_end.as_deref() {
            Some("rurix") => ExrSourceEnd::Rurix,
            Some("ue5") => ExrSourceEnd::Ue5,
            other => fail(&format!("--source-end 闭集外: {other:?}")),
        };
        let scale: f64 = exposure_scale
            .unwrap_or_else(|| fail("缺 --exposure-scale"))
            .parse()
            .unwrap_or_else(|_| fail("--exposure-scale 非 f64"));
        let digest = params_digest.unwrap_or_else(|| fail("缺 --params-digest"));
        let bytes = std::fs::read(hdr_path.unwrap())
            .unwrap_or_else(|e| fail(&format!("HDR 帧读取失败: {e}")));
        let hdr = decode_exr(&bytes, end).unwrap_or_else(|e| fail(&format!("HDR 帧解码失败: {e}")));
        let src = derive_ldr(&hdr, scale, &digest, Path::new(&out_path.unwrap()))
            .unwrap_or_else(|e| fail(&e));
        eprintln!("[{TAG}] LDR 派生落盘（源帧 content digest {src}）");
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--render") {
        let mut gltf_path = None;
        let mut contract_path = None;
        let mut out_dir = None;
        let mut scene_id = None;
        let mut exposure_scale = 1.0f64;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--gltf" => gltf_path = Some(take_arg(&args, &mut i)),
                "--contract" => contract_path = Some(take_arg(&args, &mut i)),
                "--out-dir" => out_dir = Some(take_arg(&args, &mut i)),
                "--scene-id" => scene_id = Some(take_arg(&args, &mut i)),
                "--exposure-scale" => {
                    exposure_scale = take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--exposure-scale 非 f64"))
                }
                _ => {}
            }
            i += 1;
        }
        let text = std::fs::read_to_string(contract_path.unwrap())
            .unwrap_or_else(|e| fail(&format!("契约参数读取失败: {e}")));
        let c = parse_contract(&text).unwrap_or_else(|e| fail(&e));
        let digest = param_digest(&c);
        let scene_id = scene_id.unwrap_or_else(|| fail("缺 --scene-id"));
        let load = load_gltf_scene(Path::new(&gltf_path.unwrap()))
            .unwrap_or_else(|e| fail(&format!("场景装载失败: {e}")));
        eprintln!(
            "[{TAG}] 场景装载: prims={} tris={} mats={}（材质子集=baseColorFactor，纹理未采样——诚实边界）",
            load.primitive_count, load.triangle_count, load.material_count
        );
        let sun_toward = [
            -c.sun_direction[0],
            -c.sun_direction[1],
            -c.sun_direction[2],
        ];
        let sun_color = [
            (c.sun_color[0] * c.sun_intensity_lux) as f32,
            (c.sun_color[1] * c.sun_intensity_lux) as f32,
            (c.sun_color[2] * c.sun_intensity_lux) as f32,
        ];
        let sky_i = c.sky_intensity as f32;
        let scene = GiScene::build(
            &load.instances,
            f32v(sun_toward),
            sun_color,
            [sky_i, sky_i, sky_i],
        );
        let camera = contract_camera(&c);
        let t0 = std::time::Instant::now();
        let frame = render_frame(&scene, &camera, &c);
        let mut px = frame.pixels;
        if exposure_scale != 1.0 {
            for v in px.iter_mut() {
                *v = (*v as f64 * exposure_scale) as f32;
            }
        }
        let out_dir = PathBuf::from(out_dir.unwrap());
        if std::fs::create_dir_all(&out_dir).is_err() {
            fail("输出目录创建失败");
        }
        let frame_path = out_dir.join(format!("{scene_id}.exr"));
        let img = ExrImage::new(
            c.res_w,
            c.res_h,
            ExrChannelLayout::Rgb,
            px.clone(),
            hdr_metadata(c.res_w, c.res_h, &digest),
        )
        .unwrap_or_else(|e| fail(&format!("HDR 帧构造失败: {e}")));
        let bytes = encode_exr(&img).unwrap_or_else(|e| fail(&format!("HDR 帧编码失败: {e}")));
        if std::fs::write(&frame_path, &bytes).is_err() {
            fail("HDR 帧落盘失败");
        }
        let content = frame_content_digest(c.res_w, c.res_h, 3, &px);
        eprintln!(
            "[{TAG}] 渲染完成: {}x{} covered={}/{} render_s={:.1}",
            c.res_w,
            c.res_h,
            frame.covered,
            c.res_w * c.res_h,
            t0.elapsed().as_secs_f64()
        );
        println!(
            "{{\"scene_id\":\"{scene_id}\",\"param_digest\":\"sha256:{digest}\",\"frame\":\"{}\",\"frame_content_digest\":\"{content}\",\"covered_px\":{},\"triangles\":{}}}",
            frame_path.display(),
            frame.covered,
            load.triangle_count
        );
        std::process::exit(0);
    }

    fail("未知模式（--contract-digest / --render / --project-landmarks / --derive-ldr）");
}
