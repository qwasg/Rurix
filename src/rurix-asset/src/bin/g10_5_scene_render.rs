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
//! ## 诚实边界（G10.8b 锁定差距清单消费面 + G11.3 修复面）
//!
//! G11.3 资产与场景面修复波已落地（**全部旗标默认关——默认面与 G10.5 逐字节
//! 一致**，M141 benchmark digest 锚与 M139 R5 探针 parity 零降级；G11.3 复跑
//! 帧由旗标显式开启驱动）：
//!
//! - `--material-pbr`（R1 修复）：baseColorTexture（DDS 容器 BC1/BC3/BC5
//!   实测枚举，bcdec 真实解码 + sRGB→线性 IEC 分段）× baseColorFactor ×
//!   (1−metallic) 漫反射 + 太阳 GGX 高光（roughness/metallicFactor 消费，
//!   无 MR 纹理——bistro 70 材质实测）+ 法线贴图（BC5 XY 重建 Z，逐三角形
//!   UV 梯度切线架）；非 DDS 容器（cornell checker.png）显式登记
//!   `unconsumed_container`（禁静默丢弃）；GI 单反弹逐实例 albedo =
//!   漫反射均值（纹理均值 × factor × (1−metallic)，rurix-render 0-byte）。
//! - `--smooth-normals`（R2 修复）：顶点平滑法线重心插值 + 逆矩阵转置世界化
//!   + 双面翻转消费（朝向入射光线来向，tracer.rs 同口径）；默认 = winding
//!   几何法线（G10.5 口径）。
//! - `--u64-seed`（R5 修复）：契约解析走 `json::parse_str_u64` u64 全域入口，
//!   `time.random_seed` u64 顶格合法消费；默认面维持 i64 域 fail-closed
//!   （G10 M139 探针字节级 parity 面）。
//! - U3 修复（**无条件生效**，零像素影响）：glTF animations 包内通道计数显式
//!   探测 + 显式静态契约剥离声明（`--render` 输出 JSON `animations` 闭集块 +
//!   stderr 留痕），禁静默丢弃；相机位姿 = 静态节点契约（0-byte）。
//!
//! G11.4 光照与 GI 修复波已落地（**全部旗标默认关——默认面与 G10.5/G11.3
//! 逐字节一致**，M141 benchmark digest 锚与 parity 面零降级；G11.4 复跑帧由
//! 旗标显式开启驱动）：
//!
//! - `--light-seed-set <corpus/lighting_*.json>`（R3 修复，RXS-0394）：点光源
//!   集（E = color×I/d²·ndl·albedo/π·vis 阴影射线）+ glTF emissive 表面
//!   （主射线直出 + GI 双级能量贡献）消费——光源参数唯一事实源 = 契约光照
//!   JSON（corpus 派生链 M133 只追加修订程序转入），glTF 字段直读绕过即
//!   RED；逐盏 provenance 进 `lights` 闭集块；cornell 契约 sun+sky 灯面
//!   0-byte（legacy 文件 = 空集显式登记）。
//! - `--gi-multibounce`（R4 修复，RXS-0395/0396）：世界辐射缓存世界级承接
//!   （空间哈希 + 对数族距离自适应辐射 LOD〔LEVELS=4，s0=diag×2^-8，
//!   d_ref=diag×2^-4〕+ 双哈希步长线性探测 + 级间回落链）+ 多反弹
//!   （WC_BOUNCE_ITERS=3 级迭代在线构建，第二次及以上反弹入射辐射度经
//!   世界缓存查询）+ 屏幕探针失效像素级回落 → 天光末级兜底显式登记 +
//!   远场探针集能量回归计数——全部计数进 `world_cache` 闭集块。
//! - `--world-cache-fixture`（M154 锚②面）：M96 cornell fixture 对拍——
//!   本路径多反弹 host 渲染 vs M96 host oracle（trace_host 匹配深度 full
//!   档 spp=64）rel_dev measured 产冻结带（P-09，M99 同程序纪律）。
//! - `--gi-off`（G13.4 M-d 加性旗标，RXS-0406 L1 indirect_derivation 面）：
//!   GI 贡献零面——合成式自然退化为 direct + emissive（gi_on_minus_gi_off
//!   双端同构派生的 GI 关臂语义）；跳过 GI/世界缓存构建；与 --gi-multibounce
//!   互斥 fail-closed；默认关 = 既有路径逐字节 parity 0-byte。
//!
//! ## 用法
//!
//! ```text
//! g10_5_scene_render --contract-digest <params.json> [--u64-seed]
//! g10_5_scene_render --render --gltf <scene.gltf> --contract <params.json> \
//!     --out-dir <dir> --scene-id <id> [--exposure-scale <f64>] \
//!     [--material-pbr] [--smooth-normals] [--u64-seed]
//! g10_5_scene_render --project-landmarks --contract <params.json> --landmarks <landmarks.json>
//! g10_5_scene_render --derive-ldr --hdr <frame.exr> --source-end <rurix|ue5> \
//!     --out <ldr.exr> --exposure-scale <f64>
//! g10_5_scene_render --benchmark --gltf <scene.gltf> --contract <params.json> \
//!     --scene-id <id> [--warmup <n=10>] [--frames <n=150>]
//! ```
//!
//! `--benchmark`（G10.5b M141 帧率基线采样面，加性子模式，既有四面 0-byte）：
//! 场景装载 + GiScene 构建一次后置 warmup ≥10 帧（丢弃）+ N 帧计时渲染，
//! 逐帧墙钟毫秒（`Instant`，host CPU 管线口径）+ 逐帧内容 digest 集合
//! （确定性计数面）+ 首帧 digest（== A/B 库帧 digest 机核锚，release
//! profile 实测逐位复现 c2000ebf…/8519cc67…）单行 JSON 输出；只测量不
//! 定档（G10 零帧率通过线）。
//!
//! Assisted-by: Kimi-K3（G10.5a 波；G11.3 波修复面）

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
use rurix_render::gi::probe::{
    GiCamera, cosine_sample_hemisphere, place_probes, probe_seed,
};
use rurix_render::gi::tracer::{GiMeshInstance, GiScene, RadianceTracer, RayTracedRadiance};
use rurix_render::rt::bvh::{Ray, Transform3x4, Vec3};
use rurix_render::rt::ref_tracer::{Pcg32, RAY_EPS};
use rurix_render::temporal::image::ImageF32;
use std::path::{Path, PathBuf};

mod world_cache;
use world_cache::{
    WC_BOUNCE_ITERS, WC_BUILD_CELL, WC_BUILD_RAYS, WC_LEVELS, WorldCache,
};

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

/// u64 全域整数读取（G11.3 R5 修复面，`--u64-seed` 消费）：I64 非负 / U64 全值。
fn as_u64_full(name: &str, v: &JsonValue) -> Result<u64, String> {
    match v {
        JsonValue::I64(i) if *i >= 0 => Ok(*i as u64),
        JsonValue::U64(u) => Ok(*u),
        _ => Err(cerr(format!("{name}: expected u64（u64 全域非负）"))),
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

fn parse_contract(text: &str, u64_seed: bool) -> Result<Contract, String> {
    // G11.3 R5 修复面：`--u64-seed` 时走 u64 全域入口（u64 顶格 seed 合法消费）；
    // 默认面维持 json::parse_str i64 域 fail-closed（G10 M139 探针 parity 0-byte）。
    let root = if u64_seed {
        json::parse_str_u64(text).map_err(|e| cerr(format!("JSON: {e}")))?
    } else {
        json::parse_str(text).map_err(|e| cerr(format!("JSON: {e}")))?
    };
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
        random_seed: if u64_seed {
            as_u64_full("time.random_seed", time.get("random_seed").unwrap())?
        } else {
            as_u("time.random_seed", time.get("random_seed").unwrap(), 64)?
        },
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
        .and_then(json_f64_arr3)
        .unwrap_or([0.0, 0.0, 0.0]);
    let q = node
        .get("rotation")
        .and_then(json_f64_arr4)
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let s = node
        .get("scale")
        .and_then(json_f64_arr3)
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

// ───────────────── G11.3 资产面修复（R1 材质子集 / R2 平滑法线 / U3 动画显式剥离） ─────────────────

/// 材质记录（glTF PBR 子集消费面；bistro 70 材质实测无 metallicRoughnessTexture——
/// metallic/roughness 为 factor 标量；emissive 不表达〔R3 行 G11.4 承接面登记〕）。
#[derive(Debug, Clone)]
struct MaterialRec {
    base_color_factor: [f32; 3],
    base_color_img: Option<usize>,
    normal_img: Option<usize>,
    metallic: f32,
    roughness: f32,
    // G11.4 R3 修复面（--light-seed-set 消费；解析无条件，像素面默认 0-byte）
    emissive_factor: [f32; 3],
    emissive_img: Option<usize>,
    // G11.5b 诊断面（解析无条件登记，着色面默认不消费 = 0-byte parity）：
    // 材质名（诊断直方图锚）+ glTF alphaMode 原始字面 + baseColorFactor 第 4 通道。
    name: String,
    alpha_mode: String,
    base_color_alpha: f32,
}

/// 纹理消费记录（解码后 RGBA8 行主序；baseColor 经 sRGB→线性 IEC 分段于采样时换算）。
#[derive(Debug)]
struct TextureRec {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
    format_tag: String,
    /// 漫反射均值（线性域；GI 逐实例 albedo 代理面，rurix-render 0-byte）。
    mean_linear: [f32; 3],
}

/// 纹理消费状态（禁静默丢弃——非 DDS 容器显式登记）。
#[derive(Debug)]
enum TextureSlot {
    Consumed(TextureRec),
    DeclaredUnconsumed { uri: String, reason: String },
}

/// 逐实例着色辅助面（与 instances 同序平行）。
#[derive(Clone)]
struct InstShade {
    mesh_pos: usize,
    material: Option<usize>,
    inv_transform: Transform3x4,
}

struct SceneLoad {
    instances: Vec<GiMeshInstance>,
    primitive_count: usize,
    triangle_count: usize,
    material_count: usize,
    // G11.3 资产面（旗标消费；默认空载零开销）
    shade: Vec<InstShade>,
    mesh_normals: Vec<Option<Vec<[f32; 3]>>>,
    mesh_uvs: Vec<Option<Vec<[f32; 2]>>>,
    materials: Vec<MaterialRec>,
    textures: Vec<TextureSlot>,
    animation_count: usize,
    animation_channels: usize,
    textured_materials: usize,
    normal_mapped_materials: usize,
    // G11.4 面：场景包围盒对角线（世界空间实测；世界缓存 s0/d_ref 标定源）
    scene_diag: f32,
}

/// glTF accessor 读取（本 harness 局部面——validate::extract_meshes 单一事实源
/// 0-byte；属性面读取与几何面同 buffer 闭集）。仅消费 float32 形态（bistro/
/// cornell 语料实测 FLOAT）；其余 componentType 显式 None（登记非静默）。
fn read_accessor_f32(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_idx: u32,
    comps: usize,
) -> Option<Vec<f32>> {
    let accessors = root.get("accessors")?.as_array()?;
    let acc = accessors.get(accessor_idx as usize)?;
    if acc.get("componentType")?.as_u64()? != 5126 {
        return None;
    }
    let count = acc.get("count")?.as_u64()? as usize;
    let bvi = acc.get("bufferView")?.as_u64()? as usize;
    let bv = root.get("bufferViews")?.as_array()?.get(bvi)?;
    let buf_idx = bv.get("buffer")?.as_u64()? as usize;
    let buf = buffers.get(buf_idx)?;
    let stride = bv
        .get("byteStride")
        .and_then(|v| v.as_u64())
        .map(|s| s as usize)
        .unwrap_or(comps * 4);
    let base_off = bv.get("byteOffset").and_then(|v| v.as_u64()).unwrap_or(0) as usize
        + acc.get("byteOffset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let mut out = Vec::with_capacity(count * comps);
    for i in 0..count {
        let off = base_off + i * stride;
        for c in 0..comps {
            let b = buf.get(off + c * 4..off + c * 4 + 4)?;
            out.push(f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        }
    }
    Some(out)
}

fn read_accessor_vec3(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_idx: u32,
) -> Option<Vec<[f32; 3]>> {
    let flat = read_accessor_f32(root, buffers, accessor_idx, 3)?;
    Some(flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

fn read_accessor_vec2(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_idx: u32,
) -> Option<Vec<[f32; 2]>> {
    let flat = read_accessor_f32(root, buffers, accessor_idx, 2)?;
    Some(flat.chunks_exact(2).map(|c| [c[0], c[1]]).collect())
}

/// IEC 61966-2-1 sRGB → 线性（f32；采样时逐 texel 换算，确定性）。
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn load_gltf_scene(path: &Path, material_pbr: bool) -> Result<SceneLoad, String> {
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

    // U3 修复面（无条件生效，零像素影响）：animations 包内通道计数显式探测——
    // 显式静态契约剥离（相机 = 静态节点位姿契约，动画通道不驱动渲染），禁静默丢弃。
    let mut animation_count = 0usize;
    let mut animation_channels = 0usize;
    if let Some(anims) = root.get("animations").and_then(|v| v.as_array()) {
        animation_count = anims.len();
        for a in anims {
            animation_channels += a
                .get("channels")
                .and_then(|v| v.as_array())
                .map(|c| c.len())
                .unwrap_or(0);
        }
    }
    if animation_count > 0 {
        eprintln!(
            "[{TAG}] 动画显式剥离（U3 修复面）: animations={} channels={} policy=strip_static_contract（相机位姿契约 0-byte）",
            animation_count, animation_channels
        );
    }

    // 材质子集：baseColorFactor rgb +（--material-pbr 下）baseColorTexture /
    // normalTexture / metallicFactor / roughnessFactor 消费（G11.3 R1 修复面）。
    let mut mat_albedo: Vec<[f32; 3]> = Vec::new();
    let mut materials: Vec<MaterialRec> = Vec::new();
    if let Some(mats) = root.get("materials").and_then(|v| v.as_array()) {
        for m in mats {
            let pbr = m.get("pbrMetallicRoughness");
            let alb = pbr
                .and_then(|p| p.get("baseColorFactor"))
                .and_then(|v| json_f32_arr(v, 4))
                .map(|v| [v[0], v[1], v[2]])
                .unwrap_or([1.0, 1.0, 1.0]);
            mat_albedo.push(alb);
            let tex_img = |key: &str| -> Option<usize> {
                let t = pbr.and_then(|p| p.get(key))?;
                let ti = t.get("index")?.as_u64()? as usize;
                let tex = root.get("textures")?.as_array()?.get(ti)?;
                Some(tex.get("source")?.as_u64()? as usize)
            };
            let normal_img = m
                .get("normalTexture")
                .and_then(|t| t.get("index"))
                .and_then(|v| v.as_u64())
                .and_then(|ti| root.get("textures")?.as_array()?.get(ti as usize))
                .and_then(|tex| tex.get("source"))
                .and_then(|v| v.as_u64())
                .map(|s| s as usize);
            // G11.4 R3 面：emissiveFactor / emissiveTexture 解析（无条件，
            // 默认零像素影响；--light-seed-set 下经契约光照 JSON 登记面消费）。
            let emissive_factor = m
                .get("emissiveFactor")
                .and_then(|v| json_f32_arr(v, 3))
                .map(|v| [v[0], v[1], v[2]])
                .unwrap_or([0.0, 0.0, 0.0]);
            let emissive_img = m
                .get("emissiveTexture")
                .and_then(|t| t.get("index"))
                .and_then(|v| v.as_u64())
                .and_then(|ti| root.get("textures")?.as_array()?.get(ti as usize))
                .and_then(|tex| tex.get("source"))
                .and_then(|v| v.as_u64())
                .map(|s| s as usize);
            materials.push(MaterialRec {
                base_color_factor: alb,
                base_color_img: tex_img("baseColorTexture"),
                normal_img,
                metallic: pbr
                    .and_then(|p| p.get("metallicFactor"))
                    .and_then(|v| json_f64(v))
                    .unwrap_or(1.0) as f32,
                roughness: pbr
                    .and_then(|p| p.get("roughnessFactor"))
                    .and_then(|v| json_f64(v))
                    .unwrap_or(1.0) as f32,
                emissive_factor,
                emissive_img,
                name: m
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                alpha_mode: m
                    .get("alphaMode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("OPAQUE")
                    .to_owned(),
                base_color_alpha: pbr
                    .and_then(|p| p.get("baseColorFactor"))
                    .and_then(|v| json_f32_arr(v, 4))
                    .map(|v| v[3])
                    .unwrap_or(1.0),
            });
        }
    }

    // 纹理解码（--material-pbr 消费面；DDS 容器经 bcdec 真实解码——G10-N7 承接锚）。
    // 逐材质引用去重解码；非 DDS 容器（cornell checker.png）显式登记不静默。
    let mut textures: Vec<TextureSlot> = Vec::new();
    let image_uris: Vec<Option<String>> = root
        .get("images")
        .and_then(|v| v.as_array())
        .map(|imgs| {
            imgs.iter()
                .map(|im| im.get("uri").and_then(|u| u.as_str()).map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();
    if material_pbr {
        for (ii, uri) in image_uris.iter().enumerate() {
            let Some(uri) = uri else {
                textures.push(TextureSlot::DeclaredUnconsumed {
                    uri: format!("images[{ii}]"),
                    reason: "无 uri（内嵌 bufferView 形态不在消费闭集）".to_owned(),
                });
                continue;
            };
            if !uri.to_ascii_lowercase().ends_with(".dds") {
                textures.push(TextureSlot::DeclaredUnconsumed {
                    uri: uri.clone(),
                    reason: "非 DDS 容器（host 参考管线纹理消费闭集 = DDS/BCn；显式登记不静默）".to_owned(),
                });
                continue;
            }
            let raw = std::fs::read(base.join(uri))
                .map_err(|e| format!("纹理 {uri} 读取失败: {e}"))?;
            let img = rurix_asset::bcdec::decode_dds(&raw)
                .map_err(|e| format!("纹理 {uri} DDS 解码失败: {e}"))?;
            // 漫反射均值（线性域逐 texel srgb→线性；baseColor 用途的 GI 代理面）
            let mut acc = [0.0f64; 3];
            let npx = (img.width as usize) * (img.height as usize);
            for px in img.rgba8.chunks_exact(4) {
                for ch in 0..3 {
                    acc[ch] += srgb_to_linear(px[ch] as f32 / 255.0) as f64;
                }
            }
            let mean_linear = [
                (acc[0] / npx as f64) as f32,
                (acc[1] / npx as f64) as f32,
                (acc[2] / npx as f64) as f32,
            ];
            textures.push(TextureSlot::Consumed(TextureRec {
                width: img.width,
                height: img.height,
                rgba8: img.rgba8,
                format_tag: img.format.as_str().to_owned(),
                mean_linear,
            }));
        }
    }
    let textured_materials = materials.iter().filter(|m| m.base_color_img.is_some()).count();
    let normal_mapped_materials = materials.iter().filter(|m| m.normal_img.is_some()).count();

    // 图元 → 材质索引 + 属性面（(mesh_id, primitive_id) → material / NORMAL /
    // TEXCOORD_0；primitive_id 与 extract_meshes 的全局递增计数器同口径——
    // 逐图元递增、含非 TRIANGLES 跳过项）
    let mut prim_material: std::collections::HashMap<(u32, u32), Option<u32>> =
        std::collections::HashMap::new();
    let mut prim_normals: std::collections::HashMap<(u32, u32), Option<Vec<[f32; 3]>>> =
        std::collections::HashMap::new();
    let mut prim_uvs: std::collections::HashMap<(u32, u32), Option<Vec<[f32; 2]>>> =
        std::collections::HashMap::new();
    let mut prim_global: u32 = 0;
    if let Some(ms) = root.get("meshes").and_then(|v| v.as_array()) {
        for (mi, m) in ms.iter().enumerate() {
            if let Some(prims) = m.get("primitives").and_then(|v| v.as_array()) {
                for p in prims.iter() {
                    let mat = p.get("material").and_then(|v| v.as_u32());
                    prim_material.insert((mi as u32, prim_global), mat);
                    let attrs = p.get("attributes");
                    let normals = attrs
                        .and_then(|a| a.get("NORMAL"))
                        .and_then(|v| v.as_u32())
                        .and_then(|ai| read_accessor_vec3(&root, &buffers, ai));
                    let uvs = attrs
                        .and_then(|a| a.get("TEXCOORD_0"))
                        .and_then(|v| v.as_u32())
                        .and_then(|ai| read_accessor_vec2(&root, &buffers, ai));
                    prim_normals.insert((mi as u32, prim_global), normals);
                    prim_uvs.insert((mi as u32, prim_global), uvs);
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
            if let Some(ch) = n.get("children").and_then(|v| v.as_array())
                && ch.iter().any(|c| c.as_u32() == Some(idx as u32))
            {
                parent = Some(i);
                break;
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
    let mut shade: Vec<InstShade> = Vec::new();
    let mut mesh_normals: Vec<Option<Vec<[f32; 3]>>> = Vec::new();
    let mut mesh_uvs: Vec<Option<Vec<[f32; 2]>>> = Vec::new();
    let mut tri_total = 0usize;
    let mut bbox_min = [f32::INFINITY; 3];
    let mut bbox_max = [f32::NEG_INFINITY; 3];
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
            let mat_idx = prim_material
                .get(&(m.mesh_id, m.primitive_id))
                .copied()
                .flatten();
            // GI 逐实例 albedo（rurix-render 0-byte 代理面）：默认 = baseColorFactor
            // （G10.5 口径）；--material-pbr = 漫反射均值（纹理均值线性域 × factor
            // × (1−metallic)——能量量级代理，逐像素纹理细节由主射线着色面承载）。
            let albedo = mat_idx
                .and_then(|mi| mat_albedo.get(mi as usize).copied())
                .unwrap_or([1.0, 1.0, 1.0]);
            let albedo = if material_pbr {
                match mat_idx.and_then(|mi| materials.get(mi as usize)) {
                    Some(rec) => {
                        let k = 1.0 - rec.metallic;
                        // 纹理已消费 = 纹理均值 × factor × k;未消费/无纹理 = factor × k
                        //（factor 不重复乘）。
                        let base = rec
                            .base_color_img
                            .and_then(|ii| textures.get(ii))
                            .and_then(|s| match s {
                                TextureSlot::Consumed(t) => Some(t.mean_linear),
                                TextureSlot::DeclaredUnconsumed { .. } => None,
                            })
                            .map(|tm| {
                                [
                                    tm[0] * rec.base_color_factor[0],
                                    tm[1] * rec.base_color_factor[1],
                                    tm[2] * rec.base_color_factor[2],
                                ]
                            })
                            .unwrap_or(rec.base_color_factor);
                        [base[0] * k, base[1] * k, base[2] * k]
                    }
                    None => albedo,
                }
            } else {
                albedo
            };
            let mut indices = Vec::with_capacity(m.indices.len() / 3);
            for t3 in m.indices.chunks_exact(3) {
                indices.push([t3[0], t3[1], t3[2]]);
            }
            tri_total += indices.len();
            mesh_normals.push(
                prim_normals
                    .get(&(m.mesh_id, m.primitive_id))
                    .cloned()
                    .flatten(),
            );
            mesh_uvs.push(
                prim_uvs
                    .get(&(m.mesh_id, m.primitive_id))
                    .cloned()
                    .flatten(),
            );
            shade.push(InstShade {
                mesh_pos: mesh_normals.len() - 1,
                material: mat_idx.map(|x| x as usize),
                inv_transform: t.inverse().unwrap_or(Transform3x4::IDENTITY),
            });
            instances.push(GiMeshInstance {
                positions: m.positions.clone(),
                indices,
                transform: t,
                albedo,
            });
            // G11.4：世界空间包围盒（世界缓存标定源；顶点级变换累计）。
            for v in &m.positions {
                let wp = t.apply_point(Vec3::from_array(*v)).to_array();
                for ch in 0..3 {
                    bbox_min[ch] = bbox_min[ch].min(wp[ch]);
                    bbox_max[ch] = bbox_max[ch].max(wp[ch]);
                }
            }
        }
    }
    let scene_diag = if bbox_min[0].is_finite() && bbox_max[0].is_finite() {
        ((bbox_max[0] - bbox_min[0]).powi(2)
            + (bbox_max[1] - bbox_min[1]).powi(2)
            + (bbox_max[2] - bbox_min[2]).powi(2))
        .sqrt()
    } else {
        0.0
    };
    Ok(SceneLoad {
        primitive_count: instances.len(),
        triangle_count: tri_total,
        material_count: mat_albedo.len(),
        instances,
        shade,
        mesh_normals,
        mesh_uvs,
        materials,
        textures,
        animation_count,
        animation_channels,
        textured_materials,
        normal_mapped_materials,
        scene_diag,
    })
}

// ─────────────────── G11.4 R3 灯种子集（RXS-0394 契约光照面单通道消费） ───────────────────

/// 点光源记录（契约光照 JSON 派生面；逐盏 provenance）。
#[derive(Debug, Clone)]
struct PointLightRec {
    id: String,
    position: [f32; 3],
    color: [f32; 3],
    intensity_cd: f32,
    /// 发光轴向（朗伯余弦瓣；单位长——灯具单面发光口径，全向化即 RED 面）。
    emit_dir: [f32; 3],
    /// 关联灯具几何表面积（m²；近场钳制盘等效半径源——d²_eff = max(d², A/π)）。
    area_m2: f32,
    /// 代理覆盖的材质索引（NEE 覆盖面；None = 不覆盖任何 emissive 材质）。
    covers_material_index: Option<usize>,
    derived_from: String,
}

/// emissive 表面登记（契约光照 JSON 面；material_index 对齐 glTF 材质序）。
#[derive(Debug, Clone)]
struct EmissiveRec {
    material_index: usize,
    material_name: String,
    le: [f32; 3],
}

/// 灯种子集（契约光照参数面唯一事实源消费；legacy 文件 = 空集显式登记）。
#[derive(Debug)]
struct LightSeedSet {
    point_lights: Vec<PointLightRec>,
    emissive: Vec<EmissiveRec>,
    area_lights_declared_absent: bool,
    source_digest: String,
    legacy_face: bool,
}

impl LightSeedSet {
    fn emissive_le(&self, material_index: usize) -> Option<[f32; 3]> {
        self.emissive
            .iter()
            .find(|e| e.material_index == material_index)
            .map(|e| e.le)
    }

    /// NEE 覆盖面（点光代理关联材质）：GI 面 Le 整零排除（防 NEE/缓存双重
    /// 计数）；未覆盖 emissive 面 GI 正常进入。
    fn is_nee_covered(&self, material_index: usize) -> bool {
        self.point_lights
            .iter()
            .any(|p| p.covers_material_index == Some(material_index))
    }
}

/// 契约光照 JSON 闭集解析（fail-closed；schema 外字段拒绝）。
/// 既有键集 = schema/scene_id/lights/note；G11.4 修订面键 = point_lights /
/// emissive_surfaces / derived（M133 只追加修订程序产物；缺失 = legacy 空集
/// 显式登记，不静默冒充）。
fn parse_light_seed_set(path: &Path) -> Result<LightSeedSet, String> {
    let raw = std::fs::read(path).map_err(|e| format!("契约光照 JSON 读取失败: {e}"))?;
    let digest = format!("sha256:{}", sha256_hex(&raw));
    let text = String::from_utf8(raw).map_err(|e| format!("契约光照 JSON 非 UTF-8: {e}"))?;
    let root = json::parse_str(&text).map_err(|e| format!("契约光照 JSON: {e}"))?;
    let obj = root
        .as_object()
        .ok_or_else(|| "契约光照 JSON 根非 object".to_owned())?;
    const KEYS: &[&str] = &[
        "schema",
        "scene_id",
        "lights",
        "note",
        "point_lights",
        "emissive_surfaces",
        "derived",
    ];
    for (k, _) in obj {
        if !KEYS.contains(&k.as_str()) {
            return Err(format!("契约光照 JSON schema 外字段 {k:?}（fail-closed）"));
        }
    }
    let mut point_lights = Vec::new();
    if let Some(pl) = root.get("point_lights") {
        let arr = pl
            .as_array()
            .ok_or_else(|| "point_lights 非数组".to_owned())?;
        for (i, it) in arr.iter().enumerate() {
            let o = it
                .as_object()
                .ok_or_else(|| format!("point_lights[{i}] 非 object"))?;
            const LKEYS: &[&str] = &[
                "id",
                "node_name",
                "position",
                "color_linear_rgb",
                "intensity_cd",
                "emit_direction",
                "area_m2",
                "covers_material_index",
                "derived_from",
            ];
            for (k, _) in o {
                if !LKEYS.contains(&k.as_str()) {
                    return Err(format!("point_lights[{i}] schema 外字段 {k:?}"));
                }
            }
            for k in [
                "id",
                "position",
                "color_linear_rgb",
                "intensity_cd",
                "emit_direction",
                "area_m2",
                "derived_from",
            ] {
                if it.get(k).is_none() {
                    return Err(format!("point_lights[{i}] 缺字段 {k}"));
                }
            }
            let pos = as_f64_arr(&format!("point_lights[{i}].position"), it.get("position").unwrap(), 3)?;
            let color = as_f64_arr(
                &format!("point_lights[{i}].color_linear_rgb"),
                it.get("color_linear_rgb").unwrap(),
                3,
            )?;
            let intensity = as_f64(
                &format!("point_lights[{i}].intensity_cd"),
                it.get("intensity_cd").unwrap(),
            )?;
            if !(intensity > 0.0) {
                return Err(format!("point_lights[{i}].intensity_cd 非正（{intensity}）"));
            }
            let emit = as_f64_arr(
                &format!("point_lights[{i}].emit_direction"),
                it.get("emit_direction").unwrap(),
                3,
            )?;
            let en2: f64 = emit.iter().map(|x| x * x).sum();
            if (en2 - 1.0).abs() > 9.094947017729282e-13 {
                return Err(format!("point_lights[{i}].emit_direction 非单位长（|d|²={en2}）"));
            }
            let area = as_f64(
                &format!("point_lights[{i}].area_m2"),
                it.get("area_m2").unwrap(),
            )?;
            if !(area > 0.0) {
                return Err(format!("point_lights[{i}].area_m2 非正（{area}）"));
            }
            let covers = match it.get("covers_material_index") {
                None | Some(JsonValue::Null) => None,
                Some(v) => Some(as_u(
                    &format!("point_lights[{i}].covers_material_index"),
                    v,
                    32,
                )? as usize),
            };
            point_lights.push(PointLightRec {
                id: it.get("id").unwrap().as_str().unwrap_or("").to_owned(),
                position: [pos[0] as f32, pos[1] as f32, pos[2] as f32],
                color: [color[0] as f32, color[1] as f32, color[2] as f32],
                intensity_cd: intensity as f32,
                emit_dir: [emit[0] as f32, emit[1] as f32, emit[2] as f32],
                area_m2: area as f32,
                covers_material_index: covers,
                derived_from: it
                    .get("derived_from")
                    .unwrap()
                    .as_str()
                    .unwrap_or("")
                    .to_owned(),
            });
        }
    }
    let mut emissive = Vec::new();
    if let Some(em) = root.get("emissive_surfaces") {
        let arr = em
            .as_array()
            .ok_or_else(|| "emissive_surfaces 非数组".to_owned())?;
        for (i, it) in arr.iter().enumerate() {
            let o = it
                .as_object()
                .ok_or_else(|| format!("emissive_surfaces[{i}] 非 object"))?;
            const EKEYS: &[&str] = &[
                "material_index",
                "material_name",
                "le_linear_rgb",
                "area_m2",
                "texture_ref",
            ];
            for (k, _) in o {
                if !EKEYS.contains(&k.as_str()) {
                    return Err(format!("emissive_surfaces[{i}] schema 外字段 {k:?}"));
                }
            }
            for k in ["material_index", "material_name", "le_linear_rgb"] {
                if it.get(k).is_none() {
                    return Err(format!("emissive_surfaces[{i}] 缺字段 {k}"));
                }
            }
            let mi = as_u(
                &format!("emissive_surfaces[{i}].material_index"),
                it.get("material_index").unwrap(),
                32,
            )? as usize;
            let le = as_f64_arr(
                &format!("emissive_surfaces[{i}].le_linear_rgb"),
                it.get("le_linear_rgb").unwrap(),
                3,
            )?;
            emissive.push(EmissiveRec {
                material_index: mi,
                material_name: it.get("material_name").unwrap().as_str().unwrap_or("").to_owned(),
                le: [le[0] as f32, le[1] as f32, le[2] as f32],
            });
        }
    }
    // 面光源集：bistro 包内无 area/spot 灯节点（缺类显式登记，不冒充空集）。
    let area_lights_declared_absent = true;
    let legacy_face = root.get("point_lights").is_none() && root.get("emissive_surfaces").is_none();
    Ok(LightSeedSet {
        point_lights,
        emissive,
        area_lights_declared_absent,
        source_digest: digest,
        legacy_face,
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
    // G11.4 旗标面计数/标定登记（None = 旗标关，默认面 0-byte parity）
    g114: Option<G114Stats>,
}

/// G11.4 渲染登记面（lights/world_cache 闭集块数据源；evidence 消费）。
#[derive(Debug, Default)]
struct G114Stats {
    // R3 灯种子集面（RXS-0394）
    point_lights_consumed: usize,
    emissive_materials_consumed: usize,
    emissive_instances: usize,
    // R4 世界缓存面（RXS-0395/0396）
    scene_diag: f32,
    s0: f32,
    d_ref: f32,
    levels: u32,
    bounce_iters: u32,
    build_cell: u32,
    build_rays: u32,
    deposits: [u64; 4],
    dropped: [u64; 4],
    queries: [u64; 4],
    hits: [u64; 4],
    coarse_fallback: [u64; 4],
    cache_miss: u64,
    energy_per_iter: Vec<[f64; 4]>,
    fallback_px: u64,
    last_resort_px: u64,
    farfield_probe_count: usize,
    farfield_energy_mean: f64,
    // 诊断面（帧均值分项登记）
    gi_mean: f64,
    direct_mean: f64,
    emissive_mean: f64,
    // G11.5b 天光直接 IBL 面（RXS-0397；--sky-ibl 旗标登记面）
    sky_ibl: bool,
    sky_ibl_direct_mean: f64,
}

/// 双线性纹理采样（REPEAT 环绕；glTF UV 原点 = 图像左上，DDS 行主序同向直映射）。
/// baseColor 用途：逐 texel sRGB→线性 IEC 分段换算后双线性（f32 确定性）。
fn sample_basecolor_linear(tex: &TextureRec, u: f32, v: f32) -> [f32; 3] {
    let x = u * tex.width as f32 - 0.5;
    let y = v * tex.height as f32 - 0.5;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let w = tex.width as i32;
    let h = tex.height as i32;
    let mut acc = [0.0f32; 3];
    for (dx, dy, wt) in [
        (0i32, 0i32, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let xx = (x0 + dx).rem_euclid(w) as usize;
        let yy = (y0 + dy).rem_euclid(h) as usize;
        let i = (yy * tex.width as usize + xx) * 4;
        for ch in 0..3 {
            let lin = srgb_to_linear(tex.rgba8[i + ch] as f32 / 255.0);
            acc[ch] += lin * wt;
        }
    }
    acc
}

/// 法线贴图采样（BC5 XY → [-1,1]，Z 重建；返回切线空间法线）。
fn sample_normal_ts(tex: &TextureRec, u: f32, v: f32) -> [f32; 3] {
    let x = u * tex.width as f32 - 0.5;
    let y = v * tex.height as f32 - 0.5;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let w = tex.width as i32;
    let h = tex.height as i32;
    let mut acc = [0.0f32; 2];
    for (dx, dy, wt) in [
        (0i32, 0i32, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let xx = (x0 + dx).rem_euclid(w) as usize;
        let yy = (y0 + dy).rem_euclid(h) as usize;
        let i = (yy * tex.width as usize + xx) * 4;
        acc[0] += (tex.rgba8[i] as f32 / 255.0 * 2.0 - 1.0) * wt;
        acc[1] += (tex.rgba8[i + 1] as f32 / 255.0 * 2.0 - 1.0) * wt;
    }
    let z2 = (1.0 - acc[0] * acc[0] - acc[1] * acc[1]).max(0.0);
    [acc[0], acc[1], z2.sqrt()]
}

/// 太阳 GGX 高光（UE Default Lit 同族闭式：D=GGX(a=roughness²)，
/// Vis=Schlick-Smith(k=a/2)，F=Schlick；确定性解析式，host 参考管线直射面）。
fn ggx_specular(n: Vec3, v_dir: Vec3, l: Vec3, roughness: f32, f0: [f32; 3]) -> [f32; 3] {
    let h = (v_dir + l).normalize();
    let ndh = n.dot(h).max(0.0);
    let ndv = n.dot(v_dir).max(1e-6);
    let ndl = n.dot(l).max(0.0);
    let vdh = v_dir.dot(h).max(0.0);
    if ndl <= 0.0 || ndh <= 0.0 {
        return [0.0; 3];
    }
    let a = roughness * roughness;
    let a2 = a * a;
    let d_denom = ndh * ndh * (a2 - 1.0) + 1.0;
    let d = a2 / (core::f32::consts::PI * d_denom * d_denom);
    let k = a * 0.5;
    let g1 = |x: f32| x / (x * (1.0 - k) + k);
    let vis = g1(ndl) * g1(ndv) / (4.0 * ndl * ndv);
    let fres = (1.0 - vdh).powi(5);
    let mut out = [0.0f32; 3];
    for ch in 0..3 {
        let f = f0[ch] + (1.0 - f0[ch]) * fres;
        out[ch] = d * vis * f;
    }
    out
}

// ─────────────────── G11.4 双级 GI 面（RXS-0395/0396 消费面） ───────────────────

/// 点光源集直接光（RXS-0394 L3 辐射链：E = color × I₀·cosθ_emit/d²（朗伯
/// 余弦瓣——灯具单面发光口径），L = E·ndl·albedo/π·vis；阴影射线原点沿法线
/// 偏移 RAY_EPS、遮蔽上界 = 灯距——验证射线零跳过，RXS-0361 L2 口径继承）。
/// 返回漫反射出射辐射度（无点光 = 零）。
fn point_lights_direct(
    scene: &GiScene,
    p: Vec3,
    n: Vec3,
    albedo: [f32; 3],
    lights: &LightSeedSet,
) -> [f32; 3] {
    let inv_pi = 1.0 / core::f32::consts::PI;
    let origin = p + n * RAY_EPS;
    let mut out = [0.0f32; 3];
    for pl in &lights.point_lights {
        let lp = Vec3::from_array(pl.position);
        let to_l = lp - p;
        let d2 = to_l.dot(to_l);
        if d2 <= 1e-12 {
            continue;
        }
        let d = d2.sqrt();
        let dir_l = to_l * (1.0 / d);
        let nl = n.dot(dir_l).max(0.0);
        if nl <= 0.0 {
            continue;
        }
        // 朗伯余弦瓣（发光轴向单侧；背向零——单面发光灯具口径）。
        let emit = Vec3::from_array(pl.emit_dir);
        let cos_emit = emit.dot(Vec3::new(-dir_l.x, -dir_l.y, -dir_l.z));
        if cos_emit <= 0.0 {
            continue;
        }
        let shadow = Ray { origin, dir: dir_l };
        if scene.tlas.any_hit(&scene.blases, &shadow, d * (1.0 - 1e-4)) {
            continue;
        }
        // 近场钳制（RXS-0394 L3 登记）：d²_eff = max(d², A/π)——盘等效半径
        // 截断点代理的 1/d² 奇异性（接触面辐照度界 = Le·π，物理界非拟合）。
        let d2_eff = d2.max(pl.area_m2 / core::f32::consts::PI);
        let e = pl.intensity_cd * cos_emit / d2_eff;
        for ch in 0..3 {
            out[ch] += pl.color[ch] * e * nl * albedo[ch] * inv_pi;
        }
    }
    out
}

/// 命中点直接光辐射度（G11.4 扩展面：太阳 + 点光源集 + emissive 表面；
/// tracer.rs 单太阳面同公式扩展——阴影射线原点沿法线偏移 RAY_EPS 同口径）。
/// `emissive_inst` = 逐实例 Le（契约光照 JSON 登记面；非 emissive 实例 = 0）。
#[allow(clippy::too_many_arguments)]
fn direct_light_at(
    scene: &GiScene,
    p: Vec3,
    n: Vec3,
    instance: usize,
    sun_toward: Vec3,
    sun_color: [f32; 3],
    lights: Option<&LightSeedSet>,
    emissive_inst: &[[f32; 3]],
) -> [f32; 3] {
    let albedo = scene.albedos[instance];
    let inv_pi = 1.0 / core::f32::consts::PI;
    let mut out = emissive_inst[instance];
    let ndl = n.dot(sun_toward).max(0.0);
    if ndl > 0.0 && sun_color[0] > 0.0 {
        let shadow = Ray {
            origin: p + n * RAY_EPS,
            dir: sun_toward,
        };
        if !scene.tlas.any_hit(&scene.blases, &shadow, f32::INFINITY) {
            for ch in 0..3 {
                out[ch] += sun_color[ch] * ndl * albedo[ch] * inv_pi;
            }
        }
    }
    if let Some(ls) = lights {
        let pl = point_lights_direct(scene, p, n, albedo, ls);
        for ch in 0..3 {
            out[ch] += pl[ch];
        }
    }
    out
}

/// 双级 GI 追踪器（RXS-0395 L1：近场屏幕探针 + 远场世界缓存兜底）——
/// 命中 ⇒ **路径终止式缓存查询**：命中返回缓存总辐射度（直接+多反弹间接
/// 已在沉积面单计数）；未命中格 ⇒ live 直接+天空项（只丢间接 = 只丢能量
/// 方向）+ miss 计数登记；射线未命中 = 天空常量色（tracer.rs 同口径）。
struct WcTracer<'a> {
    scene: &'a GiScene,
    sun_toward: Vec3,
    sun_color: [f32; 3],
    sky_color: [f32; 3],
    lights: Option<&'a LightSeedSet>,
    emissive_inst: &'a [[f32; 3]],
    cache: &'a WorldCache,
    /// G11.5b（RXS-0397）：--sky-ibl 开时间接估计子 miss 射线整零（天光首反弹
    /// = 主射线直接项单计数，防 GI/直接双重计数）；关 = 天空常量（0-byte 旧口径）。
    sky_ibl: bool,
}

impl RadianceTracer for WcTracer<'_> {
    fn trace(&self, origin: Vec3, dir: Vec3) -> [f32; 3] {
        let Some(hit) = self
            .scene
            .tlas
            .intersect(&self.scene.blases, &Ray { origin, dir })
        else {
            return if self.sky_ibl { [0.0; 3] } else { self.sky_color };
        };
        let p = origin + dir * hit.t;
        let mut n = Vec3::from_array(hit.normal);
        if n.dot(dir) > 0.0 {
            n = -n;
        }
        let n = n.normalize();
        if let Some(lq) = self.cache.query_radiance(p, n) {
            return lq;
        }
        shade_for_cache(
            self.scene,
            p,
            n,
            hit.instance as usize,
            self.sun_toward,
            self.sun_color,
            self.lights,
            self.emissive_inst,
            None,
            self.sky_ibl,
        )
    }
}

/// 命中点辐射度（构建/渲染共用统一面）：直接（太阳+点光+emissive 基项——
/// `emissive_gi` = GI 面 emissive（NEE 代理已覆盖的灯具 Le 整零排除，防
/// NEE/缓存双重计数；未代理 emissive 面正常进入））+ 上级缓存查询间接项
/// （albedo×L_parent，路径终止式）。天空能量入口 = 探针点沉积的收集均值
/// （miss 射线 = 天空常量，无偏逃逸率估计）。
#[allow(clippy::too_many_arguments)]
fn shade_for_cache(
    scene: &GiScene,
    p: Vec3,
    n: Vec3,
    instance: usize,
    sun_toward: Vec3,
    sun_color: [f32; 3],
    lights: Option<&LightSeedSet>,
    emissive_inst: &[[f32; 3]],
    parent: Option<&WorldCache>,
    sky_ibl: bool,
) -> [f32; 3] {
    let mut l = direct_light_at(
        scene,
        p,
        n,
        instance,
        sun_toward,
        sun_color,
        lights,
        emissive_inst,
    );
    let albedo = scene.albedos[instance];
    // G11.5b（RXS-0397）：--sky-ibl 直接天光漫反射 IBL（全向口径 + 下半球黑
    // 半球混合 (1+n·up)/2）——沉积/缓存面直接项单计数（GI miss 射线同期整零）。
    if sky_ibl {
        let hemi = 0.5 * (1.0 + n.y).max(0.0);
        for ch in 0..3 {
            l[ch] += albedo[ch] * scene.sky_color[ch] * hemi;
        }
    }
    if let Some(par) = parent {
        if let Some(lq) = par.query_radiance(p, n) {
            for ch in 0..3 {
                l[ch] += albedo[ch] * lq[ch];
            }
        }
    }
    l
}

/// 世界缓存单级构建（SHaRC 同构在线建格 + 双沉积）：
/// - **命中点沉积**：构建探针射线命中点 q 沉积 `L(q)` = 统一面辐射度（直接+
///   天空项+上级间接）——缓存覆盖跟随射线到达面（含相机不可达远场）；
/// - **探针点沉积**：探针 p 处沉积 `L(p)` = 直接 + albedo(p)×mean(L_seen)
///   （p 的收集场均值，miss 射线 = 天空常量）——远场能量经探针收集链接进
///   p 的格（长程传播 = 射线终止于远方亮面并读取该处缓存）。
/// 两类沉积均为总辐射度量纲（邻域权重均值混合，零能量放大）；探针 = 屏幕
/// 粗格（WC_BUILD_CELL）有效探针 × WC_BUILD_RAYS 余弦半球光线；种子 = 契约
/// random_seed 经 probe_seed 派生 + 迭代盐去相关；`inst_px` = 逐像素实例
/// 回取面（探针锚点像素 → 实例 albedo/emissive）。
#[allow(clippy::too_many_arguments)]
fn build_world_cache_level(
    scene: &GiScene,
    probes: &[(Vec3, Vec3, u32)],
    parent: Option<&WorldCache>,
    params: world_cache::WcParams,
    sun_toward: Vec3,
    sun_color: [f32; 3],
    sky_color: [f32; 3],
    lights: Option<&LightSeedSet>,
    emissive_inst: &[[f32; 3]],
    seed: u64,
    iteration: u32,
    sky_ibl: bool,
) -> WorldCache {
    let mut wc = WorldCache::new(params);
    for (idx, &(ppos, pn, inst)) in probes.iter().enumerate() {
        let inst = inst as usize;
        let mut rng = Pcg32::new(
            probe_seed(seed, idx as u32)
                .wrapping_add(0xB1CE_u64.wrapping_mul(u64::from(iteration) + 1)),
        );
        let origin = ppos + pn * RAY_EPS;
        let mut acc = [0.0f64; 3];
        for _ in 0..WC_BUILD_RAYS {
            let dir = cosine_sample_hemisphere(pn, rng.next_f32(), rng.next_f32());
            let l_seen = match scene
                .tlas
                .intersect(&scene.blases, &Ray { origin, dir })
            {
                Some(hit) => {
                    let q = origin + dir * hit.t;
                    let mut qn = Vec3::from_array(hit.normal);
                    if qn.dot(dir) > 0.0 {
                        qn = -qn;
                    }
                    let qn = qn.normalize();
                    let lv = shade_for_cache(
                        scene,
                        q,
                        qn,
                        hit.instance as usize,
                        sun_toward,
                        sun_color,
                        lights,
                        emissive_inst,
                        parent,
                        sky_ibl,
                    );
                    wc.deposit(q, qn, lv);
                    lv
                }
                // G11.5b（RXS-0397）：--sky-ibl 开时 miss 射线整零（天光首反弹
                // = 直接项单计数，防 GI/直接双重计数）；关 = 天空常量旧口径 0-byte。
                None => {
                    if sky_ibl {
                        [0.0; 3]
                    } else {
                        sky_color
                    }
                }
            };
            for ch in 0..3 {
                acc[ch] += f64::from(l_seen[ch]);
            }
        }
        // 探针点沉积：L(p) = 直接（太阳+点光+emissive）+ albedo × 收集均值
        //（miss 射线 = 天空常量 ⇒ 天空经无偏逃逸率进入；E=π×mean 恒等式消 π）。
        let mut dp = direct_light_at(
            scene,
            ppos,
            pn,
            inst,
            sun_toward,
            sun_color,
            lights,
            emissive_inst,
        );
        let albedo_p = scene.albedos[inst];
        // G11.5b（RXS-0397）：探针点直接项 += 天光漫反射 IBL（半球混合口径），
        // 天空二反弹及以上经缓存链接正常进入。
        if sky_ibl {
            let hemi = 0.5 * (1.0 + pn.y).max(0.0);
            for ch in 0..3 {
                dp[ch] += albedo_p[ch] * sky_color[ch] * hemi;
            }
        }
        for ch in 0..3 {
            dp[ch] += albedo_p[ch] * (acc[ch] / f64::from(WC_BUILD_RAYS)) as f32;
        }
        wc.deposit(ppos, pn, dp);
    }
    wc
}

/// 主渲染：针孔主射线（render_gbuffer_pinhole 同口径）+ 直光/阴影（tracer 同公式）
/// + 屏幕探针单反弹 GI（seed = 契约 random_seed）。返回 scene-linear HDR RGB。
///
/// G11.3 旗标面（默认关 = G10.5 逐字节口径）：`pbr` = baseColorTexture×factor×
/// (1−metallic) 漫反射 + 太阳 GGX + 法线贴图切线空间扰动（R1）；`smooth` = 顶点
/// 平滑法线重心插值 + 双面翻转（R2）。
///
/// G11.4 旗标面（默认关 = 0-byte parity）：`lights` = R3 灯种子集（契约光照
/// JSON 单通道消费——点光源直接光 + emissive 表面主射线直出，RXS-0394）；
/// `multibounce` = R4 多反弹 + 世界辐射缓存世界级承接（双级 GI：近场屏幕探针
/// + 远场世界缓存兜底 + 像素级失效回落 → 天光末级兜底显式登记，RXS-0395/0396）；
/// `gi_off` = G13.4 M-d 加性旗标（GI 贡献零面，gi_on_minus_gi_off 派生 GI 关臂，
/// RXS-0406 L1；默认关 = 既有路径逐字节 parity）。
#[allow(clippy::too_many_arguments)]
fn render_frame(
    scene: &GiScene,
    camera: &GiCamera,
    c: &Contract,
    load: &SceneLoad,
    pbr: bool,
    smooth: bool,
    lights: Option<&LightSeedSet>,
    multibounce: bool,
    sky_ibl: bool,
    gi_off: bool,
) -> RenderOut {
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
    // G11.4 旗标面：emissive 逐实例 Le（契约光照 JSON 登记消费）+ 主射线
    // emissive 直出缓冲 + 登记计数。旗标关 = 全零缓冲零消费（0-byte parity）。
    let mut g114 = if lights.is_some() || multibounce || sky_ibl {
        Some(G114Stats::default())
    } else {
        None
    };
    if let Some(st) = g114.as_mut() {
        st.sky_ibl = sky_ibl;
    }
    let mut emissive_inst = vec![[0.0f32; 3]; load.instances.len()];
    let mut emissive_gi = vec![[0.0f32; 3]; load.instances.len()];
    let mut emissive_px = vec![[0.0f32; 3]; (w * h) as usize];
    // G11.4：逐像素实例回取面（世界缓存构建探针锚点 → 实例 albedo/emissive）。
    let mut inst_px = vec![u32::MAX; (w * h) as usize];
    if let (Some(ls), Some(st)) = (lights, g114.as_mut()) {
        st.point_lights_consumed = ls.point_lights.len();
        st.emissive_materials_consumed = ls.emissive.len();
        for (ii, sh) in load.shade.iter().enumerate() {
            if let Some(mi) = sh.material {
                if let Some(le) = ls.emissive_le(mi) {
                    emissive_inst[ii] = le;
                    st.emissive_instances += 1;
                    // GI 面排除面：NEE 代理已覆盖的灯具（Le 经点光 NEE 进入，
                    // GI 面整零防双重计数）；未代理 emissive 面正常进 GI。
                    if !ls.is_nee_covered(mi) {
                        emissive_gi[ii] = le;
                    }
                }
            }
        }
    }
    let unproject = |nx: f32, ny: f32, z: f32| -> Option<Vec3> {
        let v4 = camera.inv_view_proj.transform_vec4([nx, ny, z, 1.0]);
        if !v4[3].is_finite() || v4[3].abs() < 1e-8 {
            return None;
        }
        Some(Vec3::new(v4[0] / v4[3], v4[1] / v4[3], v4[2] / v4[3]))
    };
    let mut covered = 0usize;
    // G11.5b（RXS-0397）：--sky-ibl 直接天光项帧均值累加器（登记面）。
    let mut sky_acc = 0.0f64;
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

            // ── 命中着色面（G11.3）：实例/图元/重心坐标回取属性 ──
            let shade = &load.shade[hit.instance as usize];
            let inst = &load.instances[hit.instance as usize];
            let tri = inst.indices[hit.tri as usize];
            let (bu, bv) = (hit.bary[0], hit.bary[1]);
            let bw = 1.0 - bu - bv;

            // 法线：默认 = winding 几何法线（G10.5 口径）；--smooth-normals =
            // 顶点平滑法线重心插值 + 逆矩阵转置世界化（bvh.rs 世界化同式）。
            let mut n = Vec3::from_array(hit.normal);
            if smooth && let Some(Some(vn)) = load.mesh_normals.get(shade.mesh_pos) {
                let nl = Vec3::new(
                    vn[tri[0] as usize][0] * bw
                        + vn[tri[1] as usize][0] * bu
                        + vn[tri[2] as usize][0] * bv,
                    vn[tri[0] as usize][1] * bw
                        + vn[tri[1] as usize][1] * bu
                        + vn[tri[2] as usize][1] * bv,
                    vn[tri[0] as usize][2] * bw
                        + vn[tri[1] as usize][2] * bu
                        + vn[tri[2] as usize][2] * bv,
                );
                let nw = shade.inv_transform.transpose_apply(nl);
                if nw.dot(nw) > 1e-12 {
                    n = nw.normalize();
                }
            }
            // 双面着色：法线翻转朝向入射光线来向（tracer.rs 同口径）。
            if n.dot(dir) > 0.0 {
                n = -n;
            }
            let mut n = n.normalize();

            // 命中点 UV（--material-pbr / 法线贴图消费面）。
            let hit_uv: Option<[f32; 2]> = if pbr {
                load.mesh_uvs
                    .get(shade.mesh_pos)
                    .and_then(|u| u.as_ref())
                    .map(|uvs| {
                        [
                            uvs[tri[0] as usize][0] * bw
                                + uvs[tri[1] as usize][0] * bu
                                + uvs[tri[2] as usize][0] * bv,
                            uvs[tri[0] as usize][1] * bw
                                + uvs[tri[1] as usize][1] * bu
                                + uvs[tri[2] as usize][1] * bv,
                        ]
                    })
            } else {
                None
            };

            // 法线贴图切线空间扰动（glTF 切线缺省面 = 逐三角形 UV 梯度切线架；
            // UV 退化〔r 非有限〕显式回落几何架不静默）。
            if pbr && let (Some(uv), Some(mi)) = (hit_uv, shade.material) {
                let nmap_tex = load
                    .materials
                    .get(mi)
                    .and_then(|m| m.normal_img)
                    .and_then(|ii| load.textures.get(ii))
                    .and_then(|s| match s {
                        TextureSlot::Consumed(t) => Some(t),
                        TextureSlot::DeclaredUnconsumed { .. } => None,
                    });
                if let Some(nmap) = nmap_tex {
                    let a = inst.transform.apply_point(Vec3::from_array(
                        inst.positions[tri[0] as usize],
                    ));
                    let b = inst.transform.apply_point(Vec3::from_array(
                        inst.positions[tri[1] as usize],
                    ));
                    let cc = inst.transform.apply_point(Vec3::from_array(
                        inst.positions[tri[2] as usize],
                    ));
                    let uvs = load.mesh_uvs[shade.mesh_pos].as_ref().unwrap();
                    let (uv0, uv1, uv2) = (
                        uvs[tri[0] as usize],
                        uvs[tri[1] as usize],
                        uvs[tri[2] as usize],
                    );
                    let duv1 = [uv1[0] - uv0[0], uv1[1] - uv0[1]];
                    let duv2 = [uv2[0] - uv0[0], uv2[1] - uv0[1]];
                    let det = duv1[0] * duv2[1] - duv1[1] * duv2[0];
                    if det.abs() > 1e-12 {
                        let r = 1.0 / det;
                        let t = Vec3::new(
                            (b - a).x * duv2[1] * r - (cc - a).x * duv1[1] * r,
                            (b - a).y * duv2[1] * r - (cc - a).y * duv1[1] * r,
                            (b - a).z * duv2[1] * r - (cc - a).z * duv1[1] * r,
                        );
                        let t = t.normalize();
                        // Gram-Schmidt 正交化 + 右手架（glTF 缺省切线口径）。
                        let t = (t - n * n.dot(t)).normalize();
                        let bdir = n.cross(t);
                        let ts = sample_normal_ts(nmap, uv[0], uv[1]);
                        n = (t * ts[0] + bdir * ts[1] + n * ts[2]).normalize();
                    }
                }
            }
            normals.set_pixel3(x, y, n.to_array());

            // 反照率/高光面：默认 = 逐实例 baseColorFactor（G10.5 口径）；
            // --material-pbr = baseColorTexture × factor，漫反射 ×(1−metallic)，
            // F0 = mix(0.04, baseColor, metallic)，GGX(roughness) 太阳高光。
            let mut albedo = scene.albedos[hit.instance as usize];
            let mut f0 = [0.04f32; 3];
            let mut roughness = 1.0f32;
            if pbr && let Some(mi) = shade.material {
                if let Some(rec) = load.materials.get(mi) {
                    let base = rec
                        .base_color_img
                        .and_then(|ii| load.textures.get(ii))
                        .and_then(|s| match s {
                            TextureSlot::Consumed(t) => Some(t),
                            TextureSlot::DeclaredUnconsumed { .. } => None,
                        })
                        .and_then(|t| hit_uv.map(|uv| sample_basecolor_linear(t, uv[0], uv[1])))
                        .map(|tb| {
                            [
                                tb[0] * rec.base_color_factor[0],
                                tb[1] * rec.base_color_factor[1],
                                tb[2] * rec.base_color_factor[2],
                            ]
                        })
                        .unwrap_or(rec.base_color_factor);
                    let kdiff = 1.0 - rec.metallic;
                    albedo = [base[0] * kdiff, base[1] * kdiff, base[2] * kdiff];
                    for ch in 0..3 {
                        f0[ch] = 0.04 + (base[ch] - 0.04) * rec.metallic;
                    }
                    roughness = rec.roughness;
                }
            }
            albedo_px[idx] = albedo;
            inst_px[idx] = hit.instance;
            covered += 1;
            // 直光（gi/tracer.rs RadianceTracer::trace 命中分支同公式：
            // sun_color·ndl·albedo/π·太阳可见性；阴影射线原点沿法线偏移 RAY_EPS；
            // --material-pbr 增太阳 GGX 高光项——确定性解析式）。
            let ndl = n.dot(sun_toward).max(0.0);
            if ndl > 0.0 && c.sun_intensity_lux > 0.0 {
                let shadow = Ray {
                    origin: p + n * RAY_EPS,
                    dir: sun_toward,
                };
                if !scene.tlas.any_hit(&scene.blases, &shadow, f32::INFINITY) {
                    let inv_pi = 1.0 / core::f32::consts::PI;
                    if pbr {
                        let view_dir = Vec3::new(-dir.x, -dir.y, -dir.z);
                        let spec = ggx_specular(n, view_dir, sun_toward, roughness, f0);
                        for ch in 0..3 {
                            direct[idx][ch] =
                                sun_color[ch] * ndl * (albedo[ch] * inv_pi + spec[ch]);
                        }
                    } else {
                        for ch in 0..3 {
                            direct[idx][ch] = sun_color[ch] * ndl * albedo[ch] * inv_pi;
                        }
                    }
                }
            }

            // ── G11.4 R3 面（旗标面）：emissive 主射线直出 + 点光源直接光 ──
            if let Some(ls) = lights {
                let le_mean = emissive_inst[hit.instance as usize];
                let mut le_px = le_mean;
                // pbr 面：emissiveTexture 逐像素采样 × emissiveFactor（与
                // baseColor 同双线性/sRGB→线性口径；无纹理/未消费 = 契约登记均值）。
                if pbr
                    && le_mean == [0.0; 3]
                    && let (Some(uv), Some(mi)) = (hit_uv, shade.material)
                    && let Some(rec) = load.materials.get(mi)
                    && rec.emissive_factor != [0.0; 3]
                {
                    let em_tex = rec
                        .emissive_img
                        .and_then(|ii| load.textures.get(ii))
                        .and_then(|s| match s {
                            TextureSlot::Consumed(t) => Some(t),
                            TextureSlot::DeclaredUnconsumed { .. } => None,
                        });
                    if let Some(t) = em_tex {
                        let tb = sample_basecolor_linear(t, uv[0], uv[1]);
                        le_px = [
                            tb[0] * rec.emissive_factor[0],
                            tb[1] * rec.emissive_factor[1],
                            tb[2] * rec.emissive_factor[2],
                        ];
                    }
                }
                emissive_px[idx] = le_px;
                let pl = point_lights_direct(scene, p, n, albedo, ls);
                for ch in 0..3 {
                    direct[idx][ch] += pl[ch];
                }
            }
            // ── G11.5b 天光直接漫反射 IBL 面（旗标 --sky-ibl；RXS-0397）：契约
            // 天光 = 全向常量辐射度 L_sky（UE SkyLight 指定 cubemap 口径对齐），
            // E = π·L·(1+n·up)/2（下半球黑半球混合）⇒ Lo = albedo·L·(1+n·up)/2；
            // 解析式确定性、零采样面；GI 侧 miss 射线同期整零（双重计数排除）。
            if sky_ibl {
                let hemi = 0.5 * (1.0 + n.y).max(0.0);
                for ch in 0..3 {
                    direct[idx][ch] += albedo[ch] * scene.sky_color[ch] * hemi;
                }
                sky_acc += f64::from(
                    scene.sky_color[0] * 0.2126 + scene.sky_color[1] * 0.7152
                        + scene.sky_color[2] * 0.0722,
                ) * f64::from(
                    albedo[0] * 0.2126 + albedo[1] * 0.7152 + albedo[2] * 0.0722,
                ) * f64::from(hemi);
            }
        }
    }

    // GI 面：默认 = 屏幕探针单反弹（G10.5 口径 0-byte）；--gi-multibounce =
    // 双级 GI（近场屏幕探针 + 远场世界缓存，WC_BOUNCE_ITERS 级迭代在线构建）；
    // --gi-off（G13.4 M-d 加性旗标）= GI 贡献零面（gi_on_minus_gi_off 双端同构
    // 派生的 GI 关臂语义；跳过 GI/世界缓存构建；默认关 = 既有逐字节 parity）。
    let gi_params = GiParams {
        seed: c.random_seed,
        temporal: false,
        ..GiParams::default()
    };
    let mut caches: Vec<WorldCache> = Vec::new();
    let gi = if gi_off {
        None
    } else if multibounce {
        let cam_pos = [
            c.cam_position[0] as f32,
            c.cam_position[1] as f32,
            c.cam_position[2] as f32,
        ];
        let params = WorldCache::params_from_scene(load.scene_diag, cam_pos);
        if let Some(st) = g114.as_mut() {
            st.scene_diag = load.scene_diag;
            st.s0 = params.s0;
            st.d_ref = params.d_ref;
            st.levels = WC_LEVELS;
            st.bounce_iters = WC_BOUNCE_ITERS;
            st.build_cell = WC_BUILD_CELL;
            st.build_rays = WC_BUILD_RAYS;
        }
        // 构建探针集 = 屏幕粗格有效探针（锚点像素实例回取）。
        let build_grid = place_probes(&depth, &normals, camera, WC_BUILD_CELL);
        let probe_list: Vec<(Vec3, Vec3, u32)> = build_grid
            .probes
            .iter()
            .filter(|p| p.valid)
            .map(|p| {
                let pi_idx = (p.anchor[1] * w + p.anchor[0]) as usize;
                (p.pos, p.normal, inst_px[pi_idx])
            })
            .filter(|&(_, _, inst)| inst != u32::MAX)
            .collect();
        for it in 0..WC_BOUNCE_ITERS {
            let wc = build_world_cache_level(
                scene,
                &probe_list,
                caches.last(),
                params,
                sun_toward,
                sun_color,
                scene.sky_color,
                lights,
                &emissive_gi,
                c.random_seed,
                it,
                sky_ibl,
            );
            if let Some(st) = g114.as_mut() {
                st.energy_per_iter.push(wc.stats.energy);
                for lv in 0..WC_LEVELS as usize {
                    st.deposits[lv] += wc.stats.deposits[lv];
                    st.dropped[lv] += wc.stats.dropped[lv];
                }
            }
            caches.push(wc);
        }
        let wc_tracer = WcTracer {
            scene,
            sun_toward,
            sun_color,
            sky_color: scene.sky_color,
            lights,
            emissive_inst: &emissive_gi,
            cache: caches.last().expect("multibounce 面至少一级缓存"),
            sky_ibl,
        };
        Some(render_gi(&depth, &normals, camera, &wc_tracer, None, None, &gi_params))
    } else {
        // GI 单反弹（host 参考管线；seed = 契约 random_seed；temporal off 单帧口径）。
        let tracer = RayTracedRadiance::new(scene.clone());
        Some(render_gi(&depth, &normals, camera, &tracer, None, None, &gi_params))
    };
    if let (Some(st), Some(wc)) = (g114.as_mut(), caches.last()) {
        for lv in 0..WC_LEVELS as usize {
            st.queries[lv] = wc.stats.queries[lv].get();
            st.hits[lv] = wc.stats.hits[lv].get();
            st.coarse_fallback[lv] = wc.stats.coarse_fallback[lv].get();
        }
        st.cache_miss = wc.stats.miss.get();
    }

    let inv_pi = 1.0 / core::f32::consts::PI;
    let mut pixels = vec![0.0f32; (w * h * 3) as usize];
    // G11.4 诊断面：GI 场/直接/emissive 分项帧均值（evidence 登记，零像素影响）。
    let mut sum_gi = 0.0f64;
    let mut sum_direct = 0.0f64;
    let mut sum_emissive = 0.0f64;
    let mut sum_covered = 0u64;
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if depth.get(x, y, 0) >= 1.0 {
                continue; // 主射线未命中 = 黑（UE 侧无天空网格同口径）
            }
            let mut gi_e = gi.as_ref().map(|g| g.irradiance.pixel3(x, y)).unwrap_or([0.0; 3]);
            // G11.4 像素级失效回落（RXS-0395 L1/RXS-0396 L4）：屏幕探针辐照度
            // 全零（查询失效面）→ 世界缓存直接查询 → 天光/常量环境末级兜底
            //（π×sky_color 与探针全 miss 估计子同口径）——逐级计数显式登记，
            // 禁静默返回零辐射。
            if multibounce && gi_e == [0.0; 3] {
                let u = (x as f32 + 0.5) / w as f32;
                let v = (y as f32 + 0.5) / h as f32;
                let (nx, ny) = (2.0 * u - 1.0, 1.0 - 2.0 * v);
                let n = Vec3::from_array(normals.pixel3(x, y));
                let pos = unproject(nx, ny, depth.get(x, y, 0));
                if let (Some(p), Some(wc), Some(st)) =
                    (pos, caches.last(), g114.as_mut())
                {
                    st.fallback_px += 1;
                    if n.length() > 0.0 {
                        if let Some(lq) = wc.query_radiance(p, n.normalize()) {
                            // 像素回落面 = 辐照度：E = π×L（恒等式口径）。
                            let pi = core::f32::consts::PI;
                            gi_e = [pi * lq[0], pi * lq[1], pi * lq[2]];
                        } else {
                            st.last_resort_px += 1;
                            // G11.5b（RXS-0397 修订行口径）：--sky-ibl 开时天光
                            // 末级兜底由主射线直接项承接（天光已单计数），GI 零值
                            // = 有效零间接，不再重复注入 π×sky；last_resort_px
                            // 计数显式登记维持。关 = π×sky 旧口径 0-byte。
                            if !sky_ibl {
                                let pi = core::f32::consts::PI;
                                gi_e = [
                                    pi * scene.sky_color[0],
                                    pi * scene.sky_color[1],
                                    pi * scene.sky_color[2],
                                ];
                            }
                        }
                    }
                }
            }
            sum_gi += f64::from(gi_e[0] * 0.2126 + gi_e[1] * 0.7152 + gi_e[2] * 0.0722);
            sum_direct +=
                f64::from(direct[idx][0] * 0.2126 + direct[idx][1] * 0.7152 + direct[idx][2] * 0.0722);
            sum_emissive += f64::from(
                emissive_px[idx][0] * 0.2126 + emissive_px[idx][1] * 0.7152 + emissive_px[idx][2] * 0.0722,
            );
            sum_covered += 1;
            for ch in 0..3 {
                pixels[idx * 3 + ch] = direct[idx][ch]
                    + albedo_px[idx][ch] * inv_pi * gi_e[ch]
                    + emissive_px[idx][ch];
            }
        }
    }
    if let Some(st) = g114.as_mut() {
        let nc = sum_covered.max(1) as f64;
        st.gi_mean = sum_gi / nc;
        st.direct_mean = sum_direct / nc;
        st.emissive_mean = sum_emissive / nc;
        st.sky_ibl_direct_mean = sky_acc / nc;
    }
    // G11.4 远场探针集（RXS-0396 L5 锚①场景标定面）：场景三角形质心确定性
    // 步进采样（≤64），分类 = 相机不可达（画幅外/背面/被遮蔽）⇒ 远场探针；
    // 能量回归 = 末级缓存逐点辐照度查询亮度均值（None = 0 计入）。
    if let (Some(st), Some(wc)) = (g114.as_mut(), caches.last()) {
        let mut ff_points: Vec<(Vec3, Vec3)> = Vec::new();
        let tri_total = load.triangle_count.max(1);
        let stride = (tri_total / 256).max(1);
        let cam_pos = Vec3::from_array([
            c.cam_position[0] as f32,
            c.cam_position[1] as f32,
            c.cam_position[2] as f32,
        ]);
        let mut g = 0usize;
        'outer: for inst in &load.instances {
            for tri in &inst.indices {
                g += 1;
                if g % stride != 0 {
                    continue;
                }
                let a = inst.transform.apply_point(Vec3::from_array(inst.positions[tri[0] as usize]));
                let b = inst.transform.apply_point(Vec3::from_array(inst.positions[tri[1] as usize]));
                let cc = inst.transform.apply_point(Vec3::from_array(inst.positions[tri[2] as usize]));
                let centroid = (a + b + cc) * (1.0 / 3.0);
                let gn = (b - a).cross(cc - a);
                if gn.length() <= 1e-12 {
                    continue;
                }
                let gn = gn.normalize();
                // 相机不可达分类：画幅外/背面 或 遮蔽射线命中更近几何。
                let clip = camera
                    .view_proj
                    .transform_vec4([centroid.x, centroid.y, centroid.z, 1.0]);
                let to_c = centroid - cam_pos;
                let dist = to_c.length();
                let facing = gn.dot(to_c * (1.0 / dist.max(1e-12))) < 0.0;
                let in_frame = clip[3] > 1e-8
                    && (clip[0] / clip[3]).abs() <= 1.0
                    && (clip[1] / clip[3]).abs() <= 1.0
                    && clip[2] / clip[3] >= 0.0
                    && clip[2] / clip[3] <= 1.0;
                let occluded = if in_frame && facing {
                    let dir = to_c * (1.0 / dist.max(1e-12));
                    scene.tlas.any_hit(
                        &scene.blases,
                        &Ray {
                            origin: cam_pos,
                            dir,
                        },
                        dist * (1.0 - 1e-3),
                    )
                } else {
                    false
                };
                if !(in_frame && facing) || occluded {
                    ff_points.push((centroid, gn));
                    if ff_points.len() >= 64 {
                        break 'outer;
                    }
                }
            }
        }
        st.farfield_probe_count = ff_points.len();
        if !ff_points.is_empty() {
            let mut acc = 0.0f64;
            for (p, n) in &ff_points {
                let lq = wc.query_radiance(*p, *n).unwrap_or([0.0; 3]);
                // 能量回归面 = 辐照度 E = π×L 亮度（f64 累加）。
                acc += core::f64::consts::PI
                    * f64::from(lq[0] * 0.2126 + lq[1] * 0.7152 + lq[2] * 0.0722);
            }
            st.farfield_energy_mean = acc / ff_points.len() as f64;
        }
    }
    RenderOut {
        pixels,
        covered,
        g114,
    }
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

    let u64_seed = args.iter().any(|a| a == "--u64-seed");

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
        match parse_contract(&text, u64_seed) {
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
        let c = parse_contract(&text, u64_seed).unwrap_or_else(|e| fail(&e));
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

    // ───────────────── G11.5b 诊断面（只读诊断模式，零渲染/派生语义影响） ─────────────────
    // --diag-aces13-sweep：aces13 view transform + 共享 sRGB 编码器的实际输入→输出
    // 曲线采样（与 --derive-ldr 同一代码路径单源消费；诊断取证面，禁假设）。
    if args.iter().any(|a| a == "--diag-aces13-sweep") {
        let aces = Aces13::new();
        let mut parts: Vec<String> = Vec::new();
        let mut push = |inp: [f64; 3], tag: &str, parts: &mut Vec<String>| {
            let disp = aces.to_display_linear(inp);
            let enc = [
                srgb_encode(disp[0].clamp(0.0, 1.0)),
                srgb_encode(disp[1].clamp(0.0, 1.0)),
                srgb_encode(disp[2].clamp(0.0, 1.0)),
            ];
            parts.push(format!(
                "{{\"tag\":\"{tag}\",\"in\":[{},{},{}],\"display_linear\":[{},{},{}],\"srgb\":[{},{},{}]}}",
                inp[0], inp[1], inp[2], disp[0], disp[1], disp[2], enc[0], enc[1], enc[2]
            ));
        };
        for (k, l) in [
            0.0f64, 1e-6, 1e-5, 3e-5, 1e-4, 3e-4, 1e-3, 3e-3, 1e-2, 3e-2, 0.09, 0.18, 0.36,
            1.0, 3.0, 10.0, 30.0, 100.0,
        ]
        .iter()
        .enumerate()
        {
            push([*l, *l, *l], &format!("neutral_{k}_{l}"), &mut parts);
        }
        push([1.0, 0.0, 0.0], "red", &mut parts);
        push([0.0, 1.0, 0.0], "green", &mut parts);
        push([0.0, 0.0, 1.0], "blue", &mut parts);
        push([0.5, 0.25, 0.1], "chroma_mixed", &mut parts);
        push([0.02, 0.02, 0.02], "shadow_neutral", &mut parts);
        println!(
            "{{\"view_transform\":\"aces13\",\"chain\":\"hdr*exposure->aces13->clamp01->srgb_encode\",\"samples\":[{}]}}",
            parts.join(",")
        );
        std::process::exit(0);
    }

    // --diag-ldr-stages：HDR→LDR 派生链逐段中间帧落盘（stage1 曝光后 scene-linear /
    // stage2 view transform 后显示线性 / stage3 sRGB 后 = --derive-ldr 产物逐位一致面），
    // 供逐段亮度统计定位发散段（G11.5b LDR 残差分解诊断面）。
    if args.iter().any(|a| a == "--diag-ldr-stages") {
        let mut hdr_path = None;
        let mut out_prefix = None;
        let mut source_end = None;
        let mut exposure_scale = None;
        let mut params_digest = None;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--hdr" => hdr_path = Some(take_arg(&args, &mut i)),
                "--out-prefix" => out_prefix = Some(take_arg(&args, &mut i)),
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
        let hdr =
            decode_exr(&bytes, end).unwrap_or_else(|e| fail(&format!("HDR 帧解码失败: {e}")));
        let aces = Aces13::new();
        let mut s1: Vec<f32> = Vec::with_capacity(hdr.pixels.len());
        let mut s2: Vec<f32> = Vec::with_capacity(hdr.pixels.len());
        let mut s3: Vec<f32> = Vec::with_capacity(hdr.pixels.len());
        for px in hdr.pixels.chunks_exact(3) {
            let lin = [
                f64::from(px[0]) * scale,
                f64::from(px[1]) * scale,
                f64::from(px[2]) * scale,
            ];
            for ch in lin {
                s1.push(ch as f32);
            }
            let disp = aces.to_display_linear(lin);
            for ch in disp {
                let cl = ch.clamp(0.0, 1.0);
                s2.push(cl as f32);
                s3.push(srgb_encode(cl) as f32);
            }
        }
        let prefix = out_prefix.unwrap_or_else(|| fail("缺 --out-prefix"));
        let write_stage = |path: &str, pixels: Vec<f32>, md: ExrMetadata| {
            let img = ExrImage::new(hdr.width, hdr.height, ExrChannelLayout::Rgb, pixels, md)
                .unwrap_or_else(|e| fail(&format!("阶段帧构造失败: {e}")));
            let bytes = encode_exr(&img).unwrap_or_else(|e| fail(&format!("阶段帧编码失败: {e}")));
            std::fs::write(path, &bytes).unwrap_or_else(|e| fail(&format!("阶段帧落盘失败: {e}")));
        };
        let base_md = || ExrMetadata {
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
        };
        write_stage(&format!("{prefix}_stage1_post_exposure.exr"), s1, base_md());
        let mut md2 = base_md();
        md2.view_transform = Some(ExrViewTransform::Aces13);
        write_stage(&format!("{prefix}_stage2_view_linear.exr"), s2, md2);
        let src_digest = frame_content_digest(hdr.width, hdr.height, 3, &hdr.pixels);
        let md3 = ExrMetadata {
            schema_version: "1".to_owned(),
            domain: ExrDomain::DisplayReferredLdr,
            transfer: ExrTransfer::Srgb,
            bit_depth: ExrBitDepth::Float32,
            source_end: ExrSourceEnd::Rurix,
            view_transform: Some(ExrViewTransform::Aces13),
            capture_params_digest: format!("sha256:{digest}"),
            derivation: ExrDerivation::DerivedHostSrgbEncoderV1,
            source_frame_digest: Some(src_digest.clone()),
            chromaticities_origin: Some(ChromaticitiesOrigin::Writer),
        };
        write_stage(&format!("{prefix}_stage3_srgb.exr"), s3, md3);
        eprintln!("[{TAG}] LDR 逐段诊断帧落盘（源帧 content digest {src_digest}）");
        std::process::exit(0);
    }

    // --diag-sky-vis：天空/太阳可见性审计（逐跨距像素主射线命中点 K 条余弦半球
    // 射线逃逸率 + 遮挡者材质直方图 + 太阳验证射线遮挡归属；Pcg32/契约 seed 确定性；
    // 只读诊断，零像素输出影响）——G11.5b LDR 残差空间分布定位面。
    if args.iter().any(|a| a == "--diag-sky-vis") {
        let mut gltf_path = None;
        let mut contract_path = None;
        let mut scene_id = None;
        let mut stride = 8usize;
        let mut rays_k = 32usize;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--gltf" => gltf_path = Some(take_arg(&args, &mut i)),
                "--contract" => contract_path = Some(take_arg(&args, &mut i)),
                "--scene-id" => scene_id = Some(take_arg(&args, &mut i)),
                "--stride" => {
                    stride = take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--stride 非 usize"))
                }
                "--rays" => {
                    rays_k = take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--rays 非 usize"))
                }
                _ => {}
            }
            i += 1;
        }
        let text = std::fs::read_to_string(contract_path.unwrap())
            .unwrap_or_else(|e| fail(&format!("契约参数读取失败: {e}")));
        let c = parse_contract(&text, u64_seed).unwrap_or_else(|e| fail(&e));
        let scene_id = scene_id.unwrap_or_else(|| fail("缺 --scene-id"));
        let load = load_gltf_scene(Path::new(&gltf_path.unwrap()), false)
            .unwrap_or_else(|e| fail(&format!("场景装载失败: {e}")));
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
        let sun_toward_v = Vec3::from_array(normalize3([
            -c.sun_direction[0] as f32,
            -c.sun_direction[1] as f32,
            -c.sun_direction[2] as f32,
        ]));
        let (w, h) = (c.res_w, c.res_h);
        let unproject = |nx: f32, ny: f32, z: f32| -> Option<Vec3> {
            let v4 = camera.inv_view_proj.transform_vec4([nx, ny, z, 1.0]);
            if !v4[3].is_finite() || v4[3].abs() < 1e-8 {
                return None;
            }
            Some(Vec3::new(v4[0] / v4[3], v4[1] / v4[3], v4[2] / v4[3]))
        };
        let glass_of: Vec<bool> = load
            .materials
            .iter()
            .map(|m| m.name.to_lowercase().contains("glass"))
            .collect();
        const GW: usize = 32;
        const GH: usize = 18;
        let mut grid_cnt = vec![0u64; GW * GH];
        let mut grid_vis = vec![0f64; GW * GH];
        let mut grid_glass = vec![0f64; GW * GH];
        let mut vis_samples: Vec<f32> = Vec::new();
        let mut blocker: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
        let mut sun_blocker: std::collections::BTreeMap<String, u64> =
            std::collections::BTreeMap::new();
        let (mut covered, mut sky_px) = (0u64, 0u64);
        let (mut total_rays, mut glass_rays) = (0u64, 0u64);
        let (mut sun_vis, mut sun_block) = (0u64, 0u64);
        for y in (0..h).step_by(stride) {
            for x in (0..w).step_by(stride) {
                let u = (x as f32 + 0.5) / w as f32;
                let v = (y as f32 + 0.5) / h as f32;
                let (nx, ny) = (2.0 * u - 1.0, 1.0 - 2.0 * v);
                let (Some(p0), Some(p1)) = (unproject(nx, ny, 0.0), unproject(nx, ny, 1.0))
                else {
                    continue;
                };
                let dir = (p1 - p0).normalize();
                let Some(hit) = scene
                    .tlas
                    .intersect(&scene.blases, &Ray { origin: p0, dir })
                else {
                    sky_px += 1;
                    continue;
                };
                covered += 1;
                let p = p0 + dir * hit.t;
                let mut n = Vec3::from_array(hit.normal);
                if n.dot(dir) > 0.0 {
                    n = -n;
                }
                let n = n.normalize();
                let origin = p + n * RAY_EPS;
                let pidx = (y * w + x) as u32;
                let mut rng = Pcg32::new(probe_seed(c.random_seed, pidx));
                let mut miss = 0u64;
                let mut glass_hit = 0u64;
                for _ in 0..rays_k {
                    let d = cosine_sample_hemisphere(n, rng.next_f32(), rng.next_f32());
                    total_rays += 1;
                    match scene.tlas.intersect(&scene.blases, &Ray { origin, dir: d }) {
                        None => miss += 1,
                        Some(h2) => {
                            let mi = load.shade[h2.instance as usize].material;
                            let name = mi
                                .and_then(|mm| load.materials.get(mm))
                                .map(|m| m.name.as_str())
                                .unwrap_or("<none>")
                                .to_owned();
                            *blocker.entry(name).or_insert(0) += 1;
                            if mi.map(|mm| glass_of.get(mm).copied().unwrap_or(false))
                                .unwrap_or(false)
                            {
                                glass_hit += 1;
                                glass_rays += 1;
                            }
                        }
                    }
                }
                if c.sun_intensity_lux > 0.0 && n.dot(sun_toward_v) > 0.0 {
                    match scene.tlas.intersect(
                        &scene.blases,
                        &Ray {
                            origin,
                            dir: sun_toward_v,
                        },
                    ) {
                        None => sun_vis += 1,
                        Some(h2) => {
                            sun_block += 1;
                            let name = load.shade[h2.instance as usize]
                                .material
                                .and_then(|mm| load.materials.get(mm))
                                .map(|m| m.name.as_str())
                                .unwrap_or("<none>")
                                .to_owned();
                            *sun_blocker.entry(name).or_insert(0) += 1;
                        }
                    }
                }
                let vf = miss as f32 / rays_k as f32;
                vis_samples.push(vf);
                let gx = ((x as usize) * GW / (w as usize)).min(GW - 1);
                let gy = ((y as usize) * GH / (h as usize)).min(GH - 1);
                let cell = gy * GW + gx;
                grid_cnt[cell] += 1;
                grid_vis[cell] += f64::from(vf);
                grid_glass[cell] += glass_hit as f64 / rays_k as f64;
            }
        }
        vis_samples.sort_by(|a, b| a.total_cmp(b));
        let n_s = vis_samples.len().max(1);
        let q = |qnt: f64| vis_samples[(n_s as f64 * qnt) as usize].min(vis_samples[n_s - 1]);
        let mean_vis = vis_samples.iter().map(|v| f64::from(*v)).sum::<f64>() / n_s as f64;
        let below_001 = vis_samples.iter().filter(|v| **v < 0.01).count();
        let mut blocker_vec: Vec<(String, u64)> = blocker.into_iter().collect();
        blocker_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let blocker_json = blocker_vec
            .iter()
            .take(12)
            .map(|(nm, ct)| format!("{{\"material\":\"{nm}\",\"rays\":{ct}}}"))
            .collect::<Vec<_>>()
            .join(",");
        let mut sun_vec: Vec<(String, u64)> = sun_blocker.into_iter().collect();
        sun_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let sun_json = sun_vec
            .iter()
            .take(12)
            .map(|(nm, ct)| format!("{{\"material\":\"{nm}\",\"points\":{ct}}}"))
            .collect::<Vec<_>>()
            .join(",");
        let glass_names = load
            .materials
            .iter()
            .filter(|m| m.name.to_lowercase().contains("glass"))
            .map(|m| {
                format!(
                    "{{\"name\":\"{}\",\"alpha_mode\":\"{}\",\"base_color_alpha\":{},\"metallic\":{},\"roughness\":{}}}",
                    m.name, m.alpha_mode, m.base_color_alpha, m.metallic, m.roughness
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let grid_vis_json = (0..GW * GH)
            .map(|c| {
                if grid_cnt[c] > 0 {
                    format!("{}", grid_vis[c] / grid_cnt[c] as f64)
                } else {
                    "null".to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        let grid_glass_json = (0..GW * GH)
            .map(|c| {
                if grid_cnt[c] > 0 {
                    format!("{}", grid_glass[c] / grid_cnt[c] as f64)
                } else {
                    "null".to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{{\"scene_id\":\"{scene_id}\",\"stride\":{stride},\"rays_per_point\":{rays_k},\"covered_points\":{covered},\"sky_pixels\":{sky_px},\"sky_intensity\":{sky_i},\"sky_visibility\":{{\"mean\":{mean_vis},\"p10\":{},\"median\":{},\"p90\":{},\"frac_below_0.01\":{}}},\"glass_blocked_ray_share\":{},\"sun\":{{\"visible_points\":{sun_vis},\"blocked_points\":{sun_block}}},\"hemisphere_blockers\":[{blocker_json}],\"sun_blockers\":[{sun_json}],\"glass_materials\":[{glass_names}],\"grid\":{{\"w\":{GW},\"h\":{GH},\"sky_vis_mean\":[{grid_vis_json}],\"glass_block_frac\":[{grid_glass_json}]}}}}",
            q(0.10),
            q(0.50),
            q(0.90),
            below_001 as f64 / n_s as f64,
            glass_rays as f64 / total_rays.max(1) as f64,
        );
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--render") {
        let mut gltf_path = None;
        let mut contract_path = None;
        let mut out_dir = None;
        let mut scene_id = None;
        let mut exposure_scale = 1.0f64;
        let material_pbr = args.iter().any(|a| a == "--material-pbr");
        let smooth_normals = args.iter().any(|a| a == "--smooth-normals");
        // G11.4 旗标面（默认关 = G10.5/G11.3 逐字节 parity）
        let gi_multibounce = args.iter().any(|a| a == "--gi-multibounce");
        // G11.5b 旗标面（默认关 = 逐字节 parity）：天光直接漫反射 IBL（RXS-0397）
        let sky_ibl = args.iter().any(|a| a == "--sky-ibl");
        // G13.4 M-d 旗标面（默认关 = 逐字节 parity）：GI 关臂（gi_on_minus_gi_off
        // 双端同构派生面；与 --gi-multibounce 互斥 fail-closed）
        let gi_off = args.iter().any(|a| a == "--gi-off");
        if gi_off && gi_multibounce {
            fail("--gi-off 与 --gi-multibounce 互斥");
        }
        let mut light_seed_path: Option<String> = None;
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
                "--light-seed-set" => light_seed_path = Some(take_arg(&args, &mut i)),
                _ => {}
            }
            i += 1;
        }
        let text = std::fs::read_to_string(contract_path.unwrap())
            .unwrap_or_else(|e| fail(&format!("契约参数读取失败: {e}")));
        let c = parse_contract(&text, u64_seed).unwrap_or_else(|e| fail(&e));
        let digest = param_digest(&c);
        let scene_id = scene_id.unwrap_or_else(|| fail("缺 --scene-id"));
        let load = load_gltf_scene(Path::new(&gltf_path.unwrap()), material_pbr)
            .unwrap_or_else(|e| fail(&format!("场景装载失败: {e}")));
        let tex_consumed = load
            .textures
            .iter()
            .filter(|s| matches!(s, TextureSlot::Consumed(_)))
            .count();
        let tex_declared = load
            .textures
            .iter()
            .filter(|s| matches!(s, TextureSlot::DeclaredUnconsumed { .. }))
            .count();
        eprintln!(
            "[{TAG}] 场景装载: prims={} tris={} mats={} textured_mats={} normal_mats={} tex_consumed={} tex_declared_unconsumed={}（material_pbr={material_pbr} smooth_normals={smooth_normals}）",
            load.primitive_count,
            load.triangle_count,
            load.material_count,
            load.textured_materials,
            load.normal_mapped_materials,
            tex_consumed,
            tex_declared,
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
        // G11.4 R3 面：契约光照 JSON（唯一事实源单通道）闭集解析。
        let light_seed = light_seed_path
            .as_deref()
            .map(|p| parse_light_seed_set(Path::new(p)).unwrap_or_else(|e| fail(&e)));
        if let Some(ls) = &light_seed {
            eprintln!(
                "[{TAG}] 灯种子集（RXS-0394 契约面）: point_lights={} emissive_materials={} legacy_face={} source={}",
                ls.point_lights.len(),
                ls.emissive.len(),
                ls.legacy_face,
                ls.source_digest
            );
        }
        let t0 = std::time::Instant::now();
        let frame = render_frame(
            &scene,
            &camera,
            &c,
            &load,
            material_pbr,
            smooth_normals,
            light_seed.as_ref(),
            gi_multibounce,
            sky_ibl,
            gi_off,
        );
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
        // U3 显式剥离声明（无条件生效，零像素影响）+ G11.3 资产面消费登记（旗标面）。
        let declared_uris: Vec<String> = load
            .textures
            .iter()
            .filter_map(|s| match s {
                TextureSlot::DeclaredUnconsumed { uri, reason } => {
                    Some(format!("{{\"uri\":\"{uri}\",\"reason\":\"{reason}\"}}"))
                }
                TextureSlot::Consumed(_) => None,
            })
            .collect();
        let mut tex_formats: Vec<&str> = load
            .textures
            .iter()
            .filter_map(|s| match s {
                TextureSlot::Consumed(t) => Some(t.format_tag.as_str()),
                TextureSlot::DeclaredUnconsumed { .. } => None,
            })
            .collect();
        tex_formats.sort_unstable();
        tex_formats.dedup();
        let tex_formats = tex_formats
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(",");
        // G11.4 登记块（闭集；旗标关 = enabled:false 显式登记不冒充）。
        let lights_json = match &light_seed {
            Some(ls) => {
                let pls = ls
                    .point_lights
                    .iter()
                    .map(|p| {
                        format!(
                            "{{\"id\":\"{}\",\"position\":[{},{},{}],\"color_linear_rgb\":[{},{},{}],\"intensity_cd\":{},\"emit_direction\":[{},{},{}],\"area_m2\":{},\"derived_from\":\"{}\"}}",
                            p.id, p.position[0], p.position[1], p.position[2],
                            p.color[0], p.color[1], p.color[2], p.intensity_cd,
                            p.emit_dir[0], p.emit_dir[1], p.emit_dir[2], p.area_m2, p.derived_from
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let st = frame.g114.as_ref();
                format!(
                    "{{\"enabled\":true,\"source_digest\":\"{}\",\"legacy_face\":{},\"point_lights_consumed\":{},\"emissive_materials_consumed\":{},\"emissive_instances\":{},\"area_lights_declared_absent\":{},\"point_lights\":[{}]}}",
                    ls.source_digest,
                    ls.legacy_face,
                    ls.point_lights.len(),
                    ls.emissive.len(),
                    st.map(|s| s.emissive_instances).unwrap_or(0),
                    ls.area_lights_declared_absent,
                    pls
                )
            }
            None => "{\"enabled\":false}".to_owned(),
        };
        let wc_json = match frame.g114.as_ref().filter(|s| s.levels > 0) {
            Some(st) => {
                let arr = |v: &[u64; 4]| {
                    v.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                };
                let energy = st
                    .energy_per_iter
                    .iter()
                    .map(|e| format!("[{},{},{},{}]", e[0], e[1], e[2], e[3]))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{{\"enabled\":true,\"levels\":{},\"scene_diag\":{},\"s0\":{},\"d_ref\":{},\"bounce_iters\":{},\"build_cell\":{},\"build_rays\":{},\"deposits\":[{}],\"dropped\":[{}],\"queries\":[{}],\"hits\":[{}],\"coarse_fallback\":[{}],\"cache_miss\":{},\"energy_per_iter\":[{}],\"fallback_px\":{},\"last_resort_px\":{},\"farfield_probe_count\":{},\"farfield_energy_mean\":{},\"gi_mean\":{},\"direct_mean\":{},\"emissive_mean\":{}}}",
                    st.levels, st.scene_diag, st.s0, st.d_ref, st.bounce_iters,
                    st.build_cell, st.build_rays,
                    arr(&st.deposits), arr(&st.dropped), arr(&st.queries), arr(&st.hits),
                    arr(&st.coarse_fallback), st.cache_miss, energy,
                    st.fallback_px, st.last_resort_px, st.farfield_probe_count,
                    st.farfield_energy_mean, st.gi_mean, st.direct_mean, st.emissive_mean
                )
            }
            None => "{\"enabled\":false}".to_owned(),
        };
        // G11.5b 登记块（闭集；旗标关 = enabled:false 显式登记不冒充）。
        let sky_json = match frame.g114.as_ref() {
            Some(st) if st.sky_ibl => format!(
                "{{\"enabled\":true,\"mode\":\"diffuse_omni_ibl_lower_hemi_black\",\"direct_sky_mean\":{}}}",
                st.sky_ibl_direct_mean
            ),
            _ => "{\"enabled\":false}".to_owned(),
        };
        println!(
            "{{\"scene_id\":\"{scene_id}\",\"param_digest\":\"sha256:{digest}\",\"frame\":\"{}\",\"frame_content_digest\":\"{content}\",\"covered_px\":{},\"triangles\":{},\"animations\":{{\"package_count\":{},\"channels\":{},\"consumed_channels\":0,\"policy\":\"strip_static_contract\"}},\"lights\":{lights_json},\"world_cache\":{wc_json},\"sky_ibl\":{sky_json},\"materials\":{{\"count\":{},\"textured\":{},\"normal_mapped\":{},\"textures_consumed\":{},\"texture_formats\":[{}],\"textures_declared_unconsumed\":[{}],\"material_pbr\":{material_pbr},\"smooth_normals\":{smooth_normals},\"gi_multibounce\":{gi_multibounce}}}}}",
            frame_path.display(),
            frame.covered,
            load.triangle_count,
            load.animation_count,
            load.animation_channels,
            load.material_count,
            load.textured_materials,
            load.normal_mapped_materials,
            tex_consumed,
            tex_formats,
            declared_uris.join(","),
        );
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--benchmark") {
        let mut gltf_path = None;
        let mut contract_path = None;
        let mut scene_id = None;
        let mut warmup = 10usize;
        let mut frames = 150usize;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--gltf" => gltf_path = Some(take_arg(&args, &mut i)),
                "--contract" => contract_path = Some(take_arg(&args, &mut i)),
                "--scene-id" => scene_id = Some(take_arg(&args, &mut i)),
                "--warmup" => {
                    warmup = take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--warmup 非 usize"))
                }
                "--frames" => {
                    frames = take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--frames 非 usize"))
                }
                _ => {}
            }
            i += 1;
        }
        if frames == 0 {
            fail("--frames 须 ≥1");
        }
        let text = std::fs::read_to_string(contract_path.unwrap())
            .unwrap_or_else(|e| fail(&format!("契约参数读取失败: {e}")));
        let c = parse_contract(&text, u64_seed).unwrap_or_else(|e| fail(&e));
        let digest = param_digest(&c);
        let scene_id = scene_id.unwrap_or_else(|| fail("缺 --scene-id"));
        let load = load_gltf_scene(Path::new(&gltf_path.unwrap()), false)
            .unwrap_or_else(|e| fail(&format!("场景装载失败: {e}")));
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
        let mut warmup_ms = Vec::with_capacity(warmup);
        for _ in 0..warmup {
            let t = std::time::Instant::now();
            let _ = render_frame(&scene, &camera, &c, &load, false, false, None, false, false, false);
            warmup_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        let mut frame_ms = Vec::with_capacity(frames);
        let mut digest_set = std::collections::BTreeSet::new();
        let mut first_digest = String::new();
        let mut covered = 0usize;
        for k in 0..frames {
            let t = std::time::Instant::now();
            let fr = render_frame(&scene, &camera, &c, &load, false, false, None, false, false, false);
            frame_ms.push(t.elapsed().as_secs_f64() * 1000.0);
            let d = frame_content_digest(c.res_w, c.res_h, 3, &fr.pixels);
            if k == 0 {
                first_digest = d.clone();
                covered = fr.covered;
            }
            digest_set.insert(d);
        }
        let fmt_ms = |v: &[f64]| {
            v.iter()
                .map(|x| format!("{x:?}"))
                .collect::<Vec<_>>()
                .join(",")
        };
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        println!(
            "{{\"scene_id\":\"{scene_id}\",\"param_digest\":\"sha256:{digest}\",\"profile\":\"{profile}\",\"warmup_count\":{warmup},\"timed_count\":{frames},\"warmup_ms\":[{}],\"frame_ms\":[{}],\"first_frame_digest\":\"{first_digest}\",\"distinct_frame_digests\":{},\"covered_px\":{covered},\"triangles\":{}}}",
            fmt_ms(&warmup_ms),
            fmt_ms(&frame_ms),
            digest_set.len(),
            load.triangle_count
        );
        std::process::exit(0);
    }

    // ─────────────────── G11.4 M154 fixture 对拍面（RXS-0396 L5 锚②） ───────────────────
    // M96 cornell fixture（path_trace::m96_cornell_scene 0-byte 消费）上跑世界缓存
    // 多反弹 host 路径 vs M96 host oracle（trace_host，匹配深度 full 档 max_bounces=4，
    // spp=64）——rel_dev measured 产冻结带（P-09；M99 同程序纪律 band=measured×2.0）。
    if args.iter().any(|a| a == "--world-cache-fixture") {
        use rurix_render::gi::path_trace::{self, MaterialKind, PtConfig};

        let fixture = path_trace::m96_cornell_scene();
        let (fw, fh) = (fixture.camera.width, fixture.camera.height);
        // 逐类材质拆实例（白/红/绿/灰盒/发光灯面——GiScene 逐实例 albedo 口径）。
        let mut groups: std::collections::BTreeMap<String, (Vec<[f32; 3]>, Vec<[u32; 3]>, [f32; 3], [f32; 3])> =
            std::collections::BTreeMap::new();
        for (t, tri) in fixture.indices.iter().enumerate() {
            let (key, albedo, emission) = match &fixture.materials[t] {
                MaterialKind::Lambert { albedo } => (format!("l{albedo:?}"), *albedo, [0.0; 3]),
                MaterialKind::Emission { albedo, emission } => {
                    ("emission".to_owned(), *albedo, *emission)
                }
                other => fail(&format!("fixture 材质越域: {other:?}")),
            };
            let g = groups.entry(key).or_default();
            let base = g.0.len() as u32;
            for &vi in tri {
                g.0.push(fixture.positions[vi as usize]);
            }
            g.1.push([base, base + 1, base + 2]);
            g.2 = albedo;
            g.3 = emission;
        }
        let mut instances = Vec::new();
        let mut emissive_inst: Vec<[f32; 3]> = Vec::new();
        let mut emissive_gi: Vec<[f32; 3]> = Vec::new();
        for (_k, (pos, idx, albedo, emission)) in groups {
            emissive_inst.push(emission);
            // fixture 灯面 = 点代理 NEE 覆盖 ⇒ GI 面 Le 整零（防双重计数，
            // 与主路径 is_nee_covered 同语义）。
            emissive_gi.push([0.0; 3]);
            instances.push(GiMeshInstance {
                positions: pos,
                indices: idx,
                transform: Transform3x4::IDENTITY,
                albedo,
            });
        }
        let scene = GiScene::build(&instances, [0.0, 0.0, 0.0], [0.0; 3], [0.0; 3]);
        // 场景标定：fixture 包围盒对角线实测。
        let mut bmin = [f32::INFINITY; 3];
        let mut bmax = [f32::NEG_INFINITY; 3];
        for v in &fixture.positions {
            for ch in 0..3 {
                bmin[ch] = bmin[ch].min(v[ch]);
                bmax[ch] = bmax[ch].max(v[ch]);
            }
        }
        let scene_diag = ((bmax[0] - bmin[0]).powi(2)
            + (bmax[1] - bmin[1]).powi(2)
            + (bmax[2] - bmin[2]).powi(2))
        .sqrt();
        let cam_pos = fixture.camera.origin;
        // 点光代理（RXS-0394 L3 派生链同源：I = Le × A 朗伯轴向点强，位置 = 灯
        // quad 中心；fixture 灯光谱中性 ⇒ color = [1,1,1]）。
        let lq = &fixture.light;
        let lc = [
            lq.p00[0] + lq.e1[0] * 0.5 + lq.e2[0] * 0.5,
            lq.p00[1] + lq.e1[1] * 0.5 + lq.e2[1] * 0.5,
            lq.p00[2] + lq.e1[2] * 0.5 + lq.e2[2] * 0.5,
        ];
        let proxy = LightSeedSet {
            point_lights: vec![PointLightRec {
                id: "fixture_light_quad_proxy".to_owned(),
                position: lc,
                color: [1.0, 1.0, 1.0],
                intensity_cd: lq.emission[0] * lq.area(),
                emit_dir: lq.normal(),
                area_m2: lq.area(),
                covers_material_index: Some(usize::MAX), // 灯面实例 GI 面整零（emissive_gi 已排除）
                derived_from: "m96_cornell light quad (Le×A 派生链同源)".to_owned(),
            }],
            emissive: Vec::new(),
            area_lights_declared_absent: true,
            source_digest: "fixture:internal".to_owned(),
            legacy_face: false,
        };
        // 主射线（PtCamera 约定逐字同式，像素中心无 jitter）+ 直接光 + 逐像素
        // 余弦收集（16 光线，seed 链派生）× 世界缓存多反弹（3 级迭代）。
        let mut depth = ImageF32::new(fw, fh, 1);
        let mut normals = ImageF32::new(fw, fh, 3);
        let mut hit_pos: Vec<Vec3> = vec![Vec3::ZERO; (fw * fh) as usize];
        let mut hit_inst = vec![usize::MAX; (fw * fh) as usize];
        let cam = &fixture.camera;
        let cf = Vec3::from_array(cam.forward);
        let cr = Vec3::from_array(cam.right);
        let cu = Vec3::from_array(cam.up);
        let co = Vec3::from_array(cam.origin);
        for py in 0..fh {
            for px in 0..fw {
                let ju = (px as f32 + 0.5) / fw as f32;
                let jv = (py as f32 + 0.5) / fh as f32;
                let sx = (2.0 * ju - 1.0) * cam.tan_half_fov;
                let sy = (1.0 - 2.0 * jv) * cam.tan_half_fov;
                let dir = (cf + cr * sx + cu * sy).normalize();
                let idx = (py * fw + px) as usize;
                if let Some(hit) = scene.tlas.intersect(&scene.blases, &Ray { origin: co, dir }) {
                    let p = co + dir * hit.t;
                    let mut n = Vec3::from_array(hit.normal);
                    if n.dot(dir) > 0.0 {
                        n = -n;
                    }
                    hit_pos[idx] = p;
                    hit_inst[idx] = hit.instance as usize;
                    depth.set(px, py, 0, 0.5); // fixture 面：深度仅占位（缓构建探针不走相机反投影）
                    normals.set_pixel3(px, py, n.normalize().to_array());
                }
            }
        }
        // 世界缓存构建（探针 = 覆盖像素步进采样，与主流水面同一构建函数）。
        let params = WorldCache::params_from_scene(scene_diag, cam_pos);
        let probe_list: Vec<(Vec3, Vec3, u32)> = (0..(fw * fh) as usize)
            .step_by(2)
            .filter(|&idx| hit_inst[idx] != usize::MAX)
            .map(|idx| {
                (
                    hit_pos[idx],
                    Vec3::from_array(normals.pixel3(idx as u32 % fw, idx as u32 / fw)),
                    hit_inst[idx] as u32,
                )
            })
            .collect();
        let mut caches: Vec<WorldCache> = Vec::new();
        for it in 0..WC_BOUNCE_ITERS {
            let wc = build_world_cache_level(
                &scene,
                &probe_list,
                caches.last(),
                params,
                Vec3::ZERO,
                [0.0; 3],
                [0.0; 3],
                Some(&proxy),
                &emissive_gi,
                path_trace::M96_SEED,
                it,
                false,
            );
            caches.push(wc);
        }
        let wc = caches.last().expect("fixture 缓存");
        // 逐像素收集（64 spp 余弦半球，E = π/N Σ L——irradiance_bruteforce 同
        // 估计子；spp 与 M96 host oracle 参照档同值对齐）。
        const GATHER_SPP: u32 = 64;
        let mut pixels = vec![0.0f32; (fw * fh * 3) as usize];
        for py in 0..fh {
            for px in 0..fw {
                let idx = (py * fw + px) as usize;
                if hit_inst[idx] == usize::MAX {
                    continue;
                }
                let p = hit_pos[idx];
                let n = Vec3::from_array(normals.pixel3(px, py));
                let inst = hit_inst[idx];
                let direct = direct_light_at(
                    &scene,
                    p,
                    n,
                    inst,
                    Vec3::ZERO,
                    [0.0; 3],
                    Some(&proxy),
                    &emissive_inst,
                );
                let mut rng = Pcg32::new(probe_seed(path_trace::M96_SEED ^ 0x6A17, idx as u32));
                let origin = p + n * RAY_EPS;
                let mut acc = [0.0f64; 3];
                for _ in 0..GATHER_SPP {
                    let dir = cosine_sample_hemisphere(n, rng.next_f32(), rng.next_f32());
                    let Some(hit) = scene.tlas.intersect(&scene.blases, &Ray { origin, dir })
                    else {
                        continue;
                    };
                    let hp = origin + dir * hit.t;
                    let mut hn = Vec3::from_array(hit.normal);
                    if hn.dot(dir) > 0.0 {
                        hn = -hn;
                    }
                    let hn = hn.normalize();
                    // 路径终止式缓存查询（命中 = 缓存总辐射度；未命中 = live 直接
                    // 面——与主流水 WcTracer 同一语义）。
                    let l = match wc.query_radiance(hp, hn) {
                        Some(lq) => lq,
                        None => shade_for_cache(
                            &scene,
                            hp,
                            hn,
                            hit.instance as usize,
                            Vec3::ZERO,
                            [0.0; 3],
                            Some(&proxy),
                            &emissive_gi,
                            None,
                            false,
                        ),
                    };
                    for ch in 0..3 {
                        acc[ch] += f64::from(l[ch]);
                    }
                }
                let scale = core::f64::consts::PI / f64::from(GATHER_SPP);
                let albedo = scene.albedos[inst];
                for ch in 0..3 {
                    pixels[idx * 3 + ch] =
                        direct[ch] + albedo[ch] * (acc[ch] * scale) as f32 / core::f32::consts::PI;
                }
            }
        }
        // 参考：M96 host oracle（匹配深度 full 档 max_bounces=4，spp=64）。
        let cfg = PtConfig::reference(64);
        let stream = path_trace::rng::generate_stream(
            (fw * fh) as usize,
            cfg.spp,
            cfg.max_bounces,
            path_trace::M96_SEED,
        );
        let reference = path_trace::trace_host(&fixture, &cfg, &stream)
            .unwrap_or_else(|e| fail(&format!("M96 host oracle 失败: {e:?}")));
        // 直接光分离诊断（--fixture-direct-diag）：参照 max_bounces=1（纯 NEE
        // 直接光）vs 本路径直接光（点代理 NEE + emissive）——分离直接/间接链
        // 残差归属（调试登记面，不进 golden）。
        if args.iter().any(|a| a == "--fixture-direct-diag") {
            let cfg1 = PtConfig {
                spp: 64,
                max_bounces: 1,
                rr_min_bounce: 0,
                seed: path_trace::M96_SEED,
                switches: path_trace::PtSwitches::REFERENCE,
            };
            let stream1 = path_trace::rng::generate_stream(
                (fw * fh) as usize,
                cfg1.spp,
                cfg1.max_bounces,
                path_trace::M96_SEED,
            );
            let ref1 = path_trace::trace_host(&fixture, &cfg1, &stream1)
                .unwrap_or_else(|e| fail(&format!("M96 host oracle(d1) 失败: {e:?}")));
            let mut my_direct = vec![0.0f32; (fw * fh * 3) as usize];
            for idx in 0..(fw * fh) as usize {
                if hit_inst[idx] == usize::MAX {
                    continue;
                }
                let d = direct_light_at(
                    &scene,
                    hit_pos[idx],
                    Vec3::from_array(normals.pixel3(idx as u32 % fw, idx as u32 / fw)),
                    hit_inst[idx],
                    Vec3::ZERO,
                    [0.0; 3],
                    Some(&proxy),
                    &emissive_inst,
                );
                my_direct[idx * 3] = d[0];
                my_direct[idx * 3 + 1] = d[1];
                my_direct[idx * 3 + 2] = d[2];
            }
            let mut dnum = 0.0f64;
            let mut dden = 0.0f64;
            let mut dcnt = 0usize;
            for idx in 0..(fw * fh) as usize {
                if hit_inst[idx] == usize::MAX {
                    continue;
                }
                let a = my_direct[idx * 3] * 0.2126 + my_direct[idx * 3 + 1] * 0.7152
                    + my_direct[idx * 3 + 2] * 0.0722;
                let b = ref1.rgb[idx * 3] * 0.2126 + ref1.rgb[idx * 3 + 1] * 0.7152
                    + ref1.rgb[idx * 3 + 2] * 0.0722;
                dnum += f64::from((a - b).abs());
                dden += f64::from(b);
                dcnt += 1;
            }
            eprintln!(
                "[fixture-direct-diag] mean|a−b|={:.6} mean_b={:.6} rel={:.4}",
                dnum / dcnt.max(1) as f64,
                dden / dcnt.max(1) as f64,
                dnum / dden.max(1e-12)
            );
        }
        // rel_dev = 覆盖像素亮度 |a−b|/max(b, floor) 均值（floor = 1% 参考均值亮度）。
        let floor_lum = {
            let mut s = 0.0f64;
            let mut n = 0usize;
            for v in reference.rgb.chunks_exact(3) {
                let l = f64::from(v[0] * 0.2126 + v[1] * 0.7152 + v[2] * 0.0722);
                if l > 0.0 {
                    s += l;
                    n += 1;
                }
            }
            (s / n.max(1) as f64) * 0.01
        };
        let mut rel_acc = 0.0f64;
        let mut rel_cnt = 0usize;
        for idx in 0..(fw * fh) as usize {
            if hit_inst[idx] == usize::MAX {
                continue;
            }
            let a = pixels[idx * 3] * 0.2126 + pixels[idx * 3 + 1] * 0.7152 + pixels[idx * 3 + 2] * 0.0722;
            let b = reference.rgb[idx * 3] * 0.2126
                + reference.rgb[idx * 3 + 1] * 0.7152
                + reference.rgb[idx * 3 + 2] * 0.0722;
            rel_acc += f64::from((a - b).abs()) / f64::from(b).max(floor_lum);
            rel_cnt += 1;
        }
        let rel_dev = rel_acc / rel_cnt.max(1) as f64;
        let product = frame_content_digest(fw, fh, 3, &pixels);
        let m96_digest = frame_content_digest(fw, fh, 3, &reference.rgb);
        // 调试面（--fixture-dump <dir>：双方像素 f32le 落盘，误差结构分析用）。
        if let Some(di) = args.iter().position(|a| a == "--fixture-dump") {
            let dir = args.get(di + 1).map(|s| s.as_str()).unwrap_or(".");
            std::fs::create_dir_all(dir).ok();
            let dump = |name: &str, data: &[f32]| {
                let b: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
                std::fs::write(format!("{dir}/{name}"), b).ok();
            };
            dump("mine_f32le.raw", &pixels);
            dump("ref_f32le.raw", &reference.rgb);
        }
        // 远场探针集（fixture 面：背面/遮蔽分类——中央盒背面朝远场）。
        let mut ff: Vec<(Vec3, Vec3)> = Vec::new();
        for inst in &instances {
            for tri in &inst.indices {
                let a = Vec3::from_array(inst.positions[tri[0] as usize]);
                let b = Vec3::from_array(inst.positions[tri[1] as usize]);
                let cc = Vec3::from_array(inst.positions[tri[2] as usize]);
                let centroid = (a + b + cc) * (1.0 / 3.0);
                let gn = (b - a).cross(cc - a);
                if gn.length() <= 1e-12 {
                    continue;
                }
                let gn = gn.normalize();
                let to_c = centroid - co;
                let dist = to_c.length();
                let facing = gn.dot(to_c * (1.0 / dist.max(1e-12))) < 0.0;
                let occluded = scene.tlas.any_hit(
                    &scene.blases,
                    &Ray {
                        origin: co,
                        dir: to_c * (1.0 / dist.max(1e-12)),
                    },
                    dist * (1.0 - 1e-3),
                );
                if !facing || occluded {
                    ff.push((centroid, gn));
                }
            }
        }
        let mut ff_energy = 0.0f64;
        for (p, n) in &ff {
            let lq = wc.query_radiance(*p, *n).unwrap_or([0.0; 3]);
            ff_energy += core::f64::consts::PI
                * f64::from(lq[0] * 0.2126 + lq[1] * 0.7152 + lq[2] * 0.0722);
        }
        let ff_energy_mean = ff_energy / ff.len().max(1) as f64;
        let arr = |v: &[u64; 4]| {
            v.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        let energy = caches
            .iter()
            .map(|c| {
                format!(
                    "[{},{},{},{}]",
                    c.stats.energy[0], c.stats.energy[1], c.stats.energy[2], c.stats.energy[3]
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{{\"fixture\":\"m96_cornell\",\"matched_depth\":\"full(4)\",\"reference_spp\":64,\"gather_spp\":{GATHER_SPP},\"product_digest\":\"{product}\",\"m96_host_digest\":\"{m96_digest}\",\"rel_dev\":{rel_dev},\"covered_px\":{rel_cnt},\"point_light_proxy\":{{\"position\":[{},{},{}],\"intensity_cd\":{}}},\"farfield_probe_count\":{},\"farfield_energy_mean\":{},\"world_cache\":{{\"levels\":{},\"scene_diag\":{},\"s0\":{},\"d_ref\":{},\"bounce_iters\":{},\"deposits\":[{}],\"queries\":[{}],\"hits\":[{}],\"cache_miss\":{},\"energy_per_iter\":[{}]}}}}",
            lc[0], lc[1], lc[2], lq.emission[0] * lq.area(),
            ff.len(),
            ff_energy_mean,
            WC_LEVELS, scene_diag, params.s0, params.d_ref, WC_BOUNCE_ITERS,
            arr(&wc.stats.deposits),
            arr(&[
                wc.stats.queries[0].get(),
                wc.stats.queries[1].get(),
                wc.stats.queries[2].get(),
                wc.stats.queries[3].get(),
            ]),
            arr(&[
                wc.stats.hits[0].get(),
                wc.stats.hits[1].get(),
                wc.stats.hits[2].get(),
                wc.stats.hits[3].get(),
            ]),
            wc.stats.miss.get(),
            energy
        );
        std::process::exit(0);
    }

    fail(
        "未知模式（--contract-digest / --render / --project-landmarks / --derive-ldr / --benchmark / --world-cache-fixture）",
    );
}
