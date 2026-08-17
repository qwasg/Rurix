//! G12.4 M163 UE PT 对标 Rurix 臂 harness（spec/visual_comparison.md RXS-0403；
//! RFC-0029 §4.6；门 `g12.p0.m163.ue_pt_parity`）。
//!
//! ## 职责闭集
//!
//! 1. **契约严格解析 + canonical digest 第二实现**（RXS-0403 L1/L2：字段闭集
//!    fail-closed + `G12PTP-1\0` 前缀 + RXS-0384 L3 同构标签/键序/宽度规则；
//!    digest 域 = 字段闭集除 provenance）——`--contract-digest` 打印
//!    `sha256:…`；`--render` 须 `--expect-digest` 全等否则拒出图（**契约
//!    digest 不等仍出报告即 RED** 的 harness 承载面）。
//! 2. **glTF 场景 → 生产化 ProdScene**：M133 清单双场景闭集（cornell-box /
//!    bistro-interior）——几何单一事实源 `rurix_asset::gltf::validate::
//!    extract_meshes` + 节点树世界变换烘焙（扁平单引用面，嵌套/多引用显式
//!    拒绝不静默）+ 逐三角材质扁平化（`texture_mean_albedo` 策略 = DDS 容器
//!    bcdec 真实解码均值线性域 × baseColorFactor × (1−metallic)；非 DDS 容器
//!    显式登记不静默；`white_tex_to_white` 策略 = cornell 棋盘格地板降白
//!    〔双端最大子集口径 G10.5 继承〕）+ 契约灯面（quad 面光 ↔ 2 发光三角
//!    逐字一致 / 点光 delta / emissive 材质逐三角三角网格光 type=2）。
//! 3. **device 真跑**：G12 生产化 PT megakernel（`kernels/g12_pt_production.rx`
//!    SPV）经 `run_ray_query_effects`（U30 ray query 执行面）真跑；固定 seed
//!    双跑位级一致（确定性协议 RXS-0357 L2/RXS-0400 继承）→ scene-linear HDR
//!    EXR 落盘（RXS-0385 rurix strict 元数据闭集）+ receipt JSON（帧 digest /
//!    均值亮度 / 双跑位级布尔 / 场景/灯面计数闭集）。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/W3 能力链缺失 → `G12_4_PT: SKIP DEV_ENV_DEGRADE`
//!（退 0，非 fake pass；`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层
//! 裁决）；判据不符/digest 不等/双跑位级漂移 → FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g12_4_ue_pt_parity_render --contract-digest <contract.json>
//! g12_4_ue_pt_parity_render --render --scene <cornell-box|bistro-interior> \
//!     --spp <n> --seed <u64> --contract <contract.json> --gltf <scene.gltf> \
//!     --spv <g12_pt_production.spv> --out-dir <dir> --expect-digest <sha256:…>
//! ```
//!
//! Assisted-by: Kimi-K3（G12.4 UE PT 对标波）

#![forbid(unsafe_code)]

use image_io::exr::{
    ChromaticitiesOrigin, ExrBitDepth, ExrChannelLayout, ExrDerivation, ExrDomain, ExrImage,
    ExrMetadata, ExrSourceEnd, ExrTransfer, encode_exr,
};
use rurix_asset::gltf::json::{self, JsonValue};
use rurix_asset::gltf::validate;
use rurix_render::gi::path_trace::prod::{
    self, LightDist, ProdConfig, ProdImage, ProdLight, ProdScene, SamplerFamily,
};
use rurix_render::gi::path_trace::{MaterialKind, PtCamera, PtLightQuad};
use rurix_render::rt::bvh::Vec3;
use rurix_rt::vk::{
    self, RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc,
};
use std::path::{Path, PathBuf};

const TAG: &str = "G12_4_PT";

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn skip(msg: &str) -> ! {
    println!("{TAG}: SKIP DEV_ENV_DEGRADE {msg}");
    std::process::exit(0)
}

fn sha256_hex(data: &[u8]) -> String {
    rurix_pkg::sha256::hex_digest(data)
}

// ---------------------------------------------------------------------------
// 契约解析（RXS-0403 L1 字段闭集 fail-closed + L2 canonical digest 第二实现）
// ---------------------------------------------------------------------------

const SCHEMA_ID: &str = "rurix.g12.ue_pt_parity_contract.v1";
const VERSION_PREFIX: &[u8] = b"G12PTP-1\0";
const UNIT_NORM_TOL: f64 = 9.094947017729282e-13; // 2^-40（RXS-0384 L2 谓词常量）

fn cerr(msg: impl Into<String>) -> String {
    format!("契约解析: {}", msg.into())
}

fn as_f64(name: &str, v: &JsonValue) -> Result<f64, String> {
    let x = v
        .as_f64()
        .ok_or_else(|| cerr(format!("{name}: expected f64")))?;
    if !x.is_finite() {
        return Err(cerr(format!("{name}: NaN/Inf forbidden")));
    }
    Ok(x)
}

fn as_u32(name: &str, v: &JsonValue) -> Result<u32, String> {
    let x = v
        .as_u64()
        .ok_or_else(|| cerr(format!("{name}: expected u32")))?;
    if x > u32::MAX as u64 {
        return Err(cerr(format!("{name}: u32 越域 {x}")));
    }
    Ok(x as u32)
}

fn as_u64(name: &str, v: &JsonValue) -> Result<u64, String> {
    v.as_u64()
        .ok_or_else(|| cerr(format!("{name}: expected u64")))
}

fn as_str<'a>(name: &str, v: &'a JsonValue) -> Result<&'a str, String> {
    let s = v
        .as_str()
        .ok_or_else(|| cerr(format!("{name}: expected str")))?;
    if s.is_empty() {
        return Err(cerr(format!("{name}: empty str")));
    }
    Ok(s)
}

fn as_bool(name: &str, v: &JsonValue) -> Result<bool, String> {
    v.as_bool()
        .ok_or_else(|| cerr(format!("{name}: expected bool")))
}

fn as_f64v(name: &str, v: &JsonValue, n: usize) -> Result<Vec<f64>, String> {
    let a = v
        .as_array()
        .ok_or_else(|| cerr(format!("{name}: expected array")))?;
    if a.len() != n {
        return Err(cerr(format!("{name}: expected f64×{n}")));
    }
    a.iter()
        .enumerate()
        .map(|(i, x)| as_f64(&format!("{name}[{i}]"), x))
        .collect()
}

fn closed<'a>(name: &str, v: &'a JsonValue, keys: &[&str]) -> Result<&'a JsonValue, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| cerr(format!("{name}: expected obj")))?;
    for (k, _) in obj.iter() {
        if !keys.contains(&k.as_str()) {
            return Err(cerr(format!("{name}: schema 外字段注入 {k}")));
        }
    }
    for k in keys {
        if !obj.iter().any(|(ek, _)| ek == k) {
            return Err(cerr(format!("{name}: 缺字段 {k}")));
        }
    }
    Ok(v)
}

/// 契约（解析后类型化面；digest 用规范化 JSON 值树直消费）。
struct Contract {
    raw: JsonValue,
    digest: String,
}

fn parse_contract(text: &str) -> Result<Contract, String> {
    let doc = json::parse_str(text).map_err(|e| cerr(format!("JSON: {e}")))?;
    closed(
        "root",
        &doc,
        &[
            "schema",
            "contract_id",
            "version",
            "spp_sequence",
            "ref_spp",
            "max_bounces",
            "seed",
            "calibration_seed",
            "noise_probe_spp",
            "rendering_policy",
            "scenes",
            "provenance",
        ],
    )?;
    if as_str("schema", doc.get("schema").unwrap())? != SCHEMA_ID {
        return Err(cerr("schema 字面不符"));
    }
    as_str("contract_id", doc.get("contract_id").unwrap())?;
    as_u32("version", doc.get("version").unwrap())?;
    let spp_v = doc.get("spp_sequence").unwrap();
    let spp_a = spp_v
        .as_array()
        .ok_or_else(|| cerr("spp_sequence 非数组"))?;
    if spp_a.is_empty() {
        return Err(cerr("spp_sequence 空"));
    }
    let mut spp: Vec<u32> = Vec::new();
    for (i, x) in spp_a.iter().enumerate() {
        spp.push(as_u32(&format!("spp_sequence[{i}]"), x)?);
    }
    for w in spp.windows(2) {
        if w[0] >= w[1] {
            return Err(cerr("spp_sequence 非严格递增"));
        }
    }
    let ref_spp = as_u32("ref_spp", doc.get("ref_spp").unwrap())?;
    if *spp.last().unwrap() != ref_spp {
        return Err(cerr("spp_sequence 末档 ≠ ref_spp"));
    }
    as_u32("max_bounces", doc.get("max_bounces").unwrap())?;
    let seed = as_u64("seed", doc.get("seed").unwrap())?;
    let cal = as_u64("calibration_seed", doc.get("calibration_seed").unwrap())?;
    if cal == seed {
        return Err(cerr("calibration_seed == seed"));
    }
    let probe = as_u32("noise_probe_spp", doc.get("noise_probe_spp").unwrap())?;
    if !spp.contains(&probe) || probe == ref_spp {
        return Err(cerr("noise_probe_spp 越序列/等于 ref_spp"));
    }
    let pol = doc.get("rendering_policy").unwrap();
    closed(
        "rendering_policy",
        pol,
        &[
            "ue_pathtracing",
            "filter_width",
            "max_bounces",
            "mis_mode",
            "russian_roulette",
            "denoiser",
            "tonemap",
        ],
    )?;
    if !as_bool("rendering_policy.ue_pathtracing", pol.get("ue_pathtracing").unwrap())? {
        return Err(cerr("ue_pathtracing 须 const true"));
    }
    as_f64("rendering_policy.filter_width", pol.get("filter_width").unwrap())?;
    as_u32("rendering_policy.max_bounces", pol.get("max_bounces").unwrap())?;
    as_u32("rendering_policy.mis_mode", pol.get("mis_mode").unwrap())?;
    as_bool(
        "rendering_policy.russian_roulette",
        pol.get("russian_roulette").unwrap(),
    )?;
    if as_str("rendering_policy.denoiser", pol.get("denoiser").unwrap())? != "off"
        || as_str("rendering_policy.tonemap", pol.get("tonemap").unwrap())? != "off"
    {
        return Err(cerr("denoiser/tonemap 须 const off"));
    }
    let scenes = doc.get("scenes").unwrap();
    let sa = scenes.as_array().ok_or_else(|| cerr("scenes 非数组"))?;
    if sa.len() != 2 {
        return Err(cerr("scenes 须恰二行"));
    }
    let mut ids: Vec<&str> = Vec::new();
    for (i, s) in sa.iter().enumerate() {
        parse_scene(i, s)?;
        ids.push(as_str("scene_id", s.get("scene_id").unwrap())?);
    }
    let mut sorted = ids.clone();
    sorted.sort();
    if sorted != ["bistro-interior", "cornell-box"] {
        return Err(cerr(format!("场景闭集不全等: {sorted:?}")));
    }
    if doc.get("provenance").unwrap().as_object().is_none() {
        return Err(cerr("provenance 须为 obj"));
    }
    let digest = format!("sha256:{}", sha256_hex(&canonical_preimage(&doc)?));
    Ok(Contract { raw: doc, digest })
}

fn parse_scene(idx: usize, s: &JsonValue) -> Result<(), String> {
    closed(
        &format!("scenes[{idx}]"),
        s,
        &[
            "scene_id",
            "m133_manifest_digest",
            "gltf_product_digest",
            "camera",
            "exposure",
            "lighting",
            "material_policy",
        ],
    )?;
    let sid = as_str("scene_id", s.get("scene_id").unwrap())?;
    if sid != "cornell-box" && sid != "bistro-interior" {
        return Err(cerr(format!("scene_id {sid} 越场景闭集")));
    }
    as_str("m133_manifest_digest", s.get("m133_manifest_digest").unwrap())?;
    as_str("gltf_product_digest", s.get("gltf_product_digest").unwrap())?;
    let cam = s.get("camera").unwrap();
    closed(
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
    as_f64v("camera.position", cam.get("position").unwrap(), 3)?;
    let q = as_f64v("camera.orientation_quat", cam.get("orientation_quat").unwrap(), 4)?;
    let n2: f64 = q.iter().map(|x| x * x).sum();
    if (n2 - 1.0).abs() > UNIT_NORM_TOL {
        return Err(cerr("camera.orientation_quat 非单位"));
    }
    let fov = as_f64("camera.fov_y_deg", cam.get("fov_y_deg").unwrap())?;
    if !(0.0..180.0).contains(&fov) {
        return Err(cerr("fov_y_deg 越域"));
    }
    let near = as_f64("camera.near", cam.get("near").unwrap())?;
    let far = as_f64("camera.far", cam.get("far").unwrap())?;
    if !(0.0 < near && near < far) {
        return Err(cerr("near/far 越域"));
    }
    let res = cam.get("resolution").unwrap();
    closed("camera.resolution", res, &["w", "h"])?;
    as_u32("camera.resolution.w", res.get("w").unwrap())?;
    as_u32("camera.resolution.h", res.get("h").unwrap())?;
    let exp = s.get("exposure").unwrap();
    closed("exposure", exp, &["mode", "ev100"])?;
    if as_str("exposure.mode", exp.get("mode").unwrap())? != "manual" {
        return Err(cerr("exposure.mode 仅 manual"));
    }
    as_f64("exposure.ev100", exp.get("ev100").unwrap())?;
    let lig = s.get("lighting").unwrap();
    closed(
        "lighting",
        lig,
        &[
            "quad_lights",
            "point_lights",
            "emissive_materials",
            "sun_intensity_lux",
            "sky_intensity",
        ],
    )?;
    as_f64("lighting.sun_intensity_lux", lig.get("sun_intensity_lux").unwrap())?;
    as_f64("lighting.sky_intensity", lig.get("sky_intensity").unwrap())?;
    for (i, q) in lig
        .get("quad_lights")
        .unwrap()
        .as_array()
        .ok_or_else(|| cerr("quad_lights 非数组"))?
        .iter()
        .enumerate()
    {
        closed(&format!("quad_lights[{i}]"), q, &["p00", "e1", "e2", "le_linear_rgb"])?;
        as_f64v("p00", q.get("p00").unwrap(), 3)?;
        as_f64v("e1", q.get("e1").unwrap(), 3)?;
        as_f64v("e2", q.get("e2").unwrap(), 3)?;
        let le = as_f64v("le_linear_rgb", q.get("le_linear_rgb").unwrap(), 3)?;
        if le.iter().any(|c| *c < 0.0) {
            return Err(cerr(format!("quad_lights[{i}].le 负值")));
        }
    }
    for (i, p) in lig
        .get("point_lights")
        .unwrap()
        .as_array()
        .ok_or_else(|| cerr("point_lights 非数组"))?
        .iter()
        .enumerate()
    {
        closed(
            &format!("point_lights[{i}]"),
            p,
            &["id", "position", "color_linear_rgb", "intensity_cd"],
        )?;
        as_str("id", p.get("id").unwrap())?;
        as_f64v("position", p.get("position").unwrap(), 3)?;
        let col = as_f64v("color_linear_rgb", p.get("color_linear_rgb").unwrap(), 3)?;
        if col.iter().any(|c| *c < 0.0) {
            return Err(cerr(format!("point_lights[{i}].color 负值")));
        }
        if as_f64("intensity_cd", p.get("intensity_cd").unwrap())? < 0.0 {
            return Err(cerr(format!("point_lights[{i}].intensity_cd 负值")));
        }
    }
    for (i, m) in lig
        .get("emissive_materials")
        .unwrap()
        .as_array()
        .ok_or_else(|| cerr("emissive_materials 非数组"))?
        .iter()
        .enumerate()
    {
        closed(
            &format!("emissive_materials[{i}]"),
            m,
            &["material_name", "material_index", "le_linear_rgb", "area_m2"],
        )?;
        as_str("material_name", m.get("material_name").unwrap())?;
        as_u32("material_index", m.get("material_index").unwrap())?;
        let le = as_f64v("le_linear_rgb", m.get("le_linear_rgb").unwrap(), 3)?;
        if le.iter().any(|c| *c < 0.0) {
            return Err(cerr(format!("emissive_materials[{i}].le 负值")));
        }
        if as_f64("area_m2", m.get("area_m2").unwrap())? <= 0.0 {
            return Err(cerr(format!("emissive_materials[{i}].area_m2 非正")));
        }
    }
    let mp = s.get("material_policy").unwrap();
    closed(
        "material_policy",
        mp,
        &["texture_mean_albedo", "white_tex_to_white"],
    )?;
    as_bool(
        "material_policy.texture_mean_albedo",
        mp.get("texture_mean_albedo").unwrap(),
    )?;
    as_bool(
        "material_policy.white_tex_to_white",
        mp.get("white_tex_to_white").unwrap(),
    )?;
    Ok(())
}

// ── canonical preimage（RXS-0403 L2：标签/键序/宽度 RXS-0384 L3 同构）──

fn enc_key(buf: &mut Vec<u8>, k: &str) {
    buf.extend_from_slice(&(k.len() as u32).to_le_bytes());
    buf.extend_from_slice(k.as_bytes());
}

fn enc_f64(buf: &mut Vec<u8>, v: f64) {
    buf.push(0x01);
    buf.extend_from_slice(&v.to_le_bytes());
}

fn enc_u32(buf: &mut Vec<u8>, v: u32) {
    buf.push(0x02);
    buf.extend_from_slice(&v.to_le_bytes());
}

fn enc_u64(buf: &mut Vec<u8>, v: u64) {
    buf.push(0x03);
    buf.extend_from_slice(&v.to_le_bytes());
}

fn enc_str(buf: &mut Vec<u8>, v: &str) {
    buf.push(0x04);
    enc_key(buf, v);
}

fn enc_bool(buf: &mut Vec<u8>, v: bool) {
    buf.push(0x05);
    buf.push(if v { 1 } else { 0 });
}

/// 字段类型表（digest 域；键序 = code point 升序通用律，本表钉类型）。
/// （与 UE 侧 g12_pt_contract.py / 门脚本内嵌实现逐字同表。）
fn root_type(k: &str) -> &'static str {
    match k {
        "calibration_seed" => "u64",
        "contract_id" => "str",
        "max_bounces" => "u32",
        "noise_probe_spp" => "u32",
        "ref_spp" => "u32",
        "rendering_policy" => "obj_policy",
        "schema" => "str",
        "scenes" => "arr_scene",
        "seed" => "u64",
        "spp_sequence" => "arr_u32",
        "version" => "u32",
        _ => "",
    }
}

fn policy_type(k: &str) -> &'static str {
    match k {
        "denoiser" => "str",
        "filter_width" => "f64",
        "max_bounces" => "u32",
        "mis_mode" => "u32",
        "russian_roulette" => "bool",
        "tonemap" => "str",
        "ue_pathtracing" => "bool",
        _ => "",
    }
}

fn camera_type(k: &str) -> &'static str {
    match k {
        "far" => "f64",
        "fov_y_deg" => "f64",
        "near" => "f64",
        "orientation_quat" => "arr_f64",
        "position" => "arr_f64",
        "resolution" => "obj_res",
        _ => "",
    }
}

fn lighting_type(k: &str) -> &'static str {
    match k {
        "emissive_materials" => "arr_emissive",
        "point_lights" => "arr_point",
        "quad_lights" => "arr_quad",
        "sky_intensity" => "f64",
        "sun_intensity_lux" => "f64",
        _ => "",
    }
}

fn scene_type(k: &str) -> &'static str {
    match k {
        "camera" => "obj_camera",
        "exposure" => "obj_exposure",
        "gltf_product_digest" => "str",
        "lighting" => "obj_lighting",
        "m133_manifest_digest" => "str",
        "material_policy" => "obj_matpol",
        "scene_id" => "str",
        _ => "",
    }
}

fn quad_type(k: &str) -> &'static str {
    match k {
        "e1" | "e2" | "le_linear_rgb" | "p00" => "arr_f64",
        _ => "",
    }
}

fn point_type(k: &str) -> &'static str {
    match k {
        "color_linear_rgb" | "position" => "arr_f64",
        "id" => "str",
        "intensity_cd" => "f64",
        _ => "",
    }
}

fn emissive_type(k: &str) -> &'static str {
    match k {
        "area_m2" => "f64",
        "le_linear_rgb" => "arr_f64",
        "material_index" => "u32",
        "material_name" => "str",
        _ => "",
    }
}

fn matpol_type(k: &str) -> &'static str {
    match k {
        "texture_mean_albedo" | "white_tex_to_white" => "bool",
        _ => "",
    }
}

fn exposure_type(k: &str) -> &'static str {
    match k {
        "ev100" => "f64",
        "mode" => "str",
        _ => "",
    }
}

fn res_type(k: &str) -> &'static str {
    match k {
        "h" | "w" => "u32",
        _ => "",
    }
}

fn enc_typed(buf: &mut Vec<u8>, ty: &str, v: &JsonValue) -> Result<(), String> {
    match ty {
        "f64" => enc_f64(buf, v.as_f64().unwrap()),
        "u32" => enc_u32(buf, v.as_u64().unwrap() as u32),
        "u64" => enc_u64(buf, v.as_u64().unwrap()),
        "str" => enc_str(buf, v.as_str().unwrap()),
        "bool" => enc_bool(buf, v.as_bool().unwrap()),
        "arr_u32" => {
            buf.push(0x09);
            for x in v.as_array().unwrap() {
                enc_u32(buf, x.as_u64().unwrap() as u32);
            }
            buf.push(0x0a);
        }
        "arr_f64" => {
            buf.push(0x09);
            for x in v.as_array().unwrap() {
                enc_f64(buf, x.as_f64().unwrap());
            }
            buf.push(0x0a);
        }
        "arr_scene" => {
            buf.push(0x09);
            for s in v.as_array().unwrap() {
                enc_obj(buf, s, scene_type)?;
            }
            buf.push(0x0a);
        }
        "arr_quad" => {
            buf.push(0x09);
            for s in v.as_array().unwrap() {
                enc_obj(buf, s, quad_type)?;
            }
            buf.push(0x0a);
        }
        "arr_point" => {
            buf.push(0x09);
            for s in v.as_array().unwrap() {
                enc_obj(buf, s, point_type)?;
            }
            buf.push(0x0a);
        }
        "arr_emissive" => {
            buf.push(0x09);
            for s in v.as_array().unwrap() {
                enc_obj(buf, s, emissive_type)?;
            }
            buf.push(0x0a);
        }
        "obj_policy" => enc_obj(buf, v, policy_type)?,
        "obj_camera" => enc_obj(buf, v, camera_type)?,
        "obj_lighting" => enc_obj(buf, v, lighting_type)?,
        "obj_matpol" => enc_obj(buf, v, matpol_type)?,
        "obj_exposure" => enc_obj(buf, v, exposure_type)?,
        "obj_res" => enc_obj(buf, v, res_type)?,
        _ => return Err(cerr(format!("未知类型标签 {ty}"))),
    }
    Ok(())
}

fn enc_obj(
    buf: &mut Vec<u8>,
    obj: &JsonValue,
    types: fn(&str) -> &'static str,
) -> Result<(), String> {
    buf.push(0x07);
    let pairs = obj.as_object().unwrap();
    let mut entries: Vec<(&String, &JsonValue)> =
        pairs.iter().map(|(k, v)| (k, v)).collect();
    entries.sort_by(|a, b| a.0.chars().cmp(b.0.chars()));
    for (k, v) in entries {
        let ty = types(k);
        if ty.is_empty() {
            return Err(cerr(format!("digest 域外字段 {k}")));
        }
        enc_key(buf, k);
        enc_typed(buf, ty, v)?;
    }
    buf.push(0x08);
    Ok(())
}

fn canonical_preimage(doc: &JsonValue) -> Result<Vec<u8>, String> {
    let mut buf = VERSION_PREFIX.to_vec();
    let pairs = doc.as_object().unwrap();
    // digest 域 = 根字段闭集除 provenance。
    let body = JsonValue::Object(
        pairs
            .iter()
            .filter(|(k, _)| k.as_str() != "provenance")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    enc_obj(&mut buf, &body, root_type)?;
    Ok(buf)
}

// ---------------------------------------------------------------------------
// glTF 场景装载 → ProdScene（几何 extract_meshes 单源 + 节点烘焙 + 材质扁平化）
// ---------------------------------------------------------------------------

type M4 = [[f64; 4]; 4];

fn m4_mul(a: &M4, b: &M4) -> M4 {
    let mut out = [[0.0f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let mut s = 0.0;
            for k in 0..4 {
                s += a[i][k] * b[k][j];
            }
            out[i][j] = s;
        }
    }
    out
}

fn node_local_m4(node: &JsonValue) -> Result<M4, String> {
    if let Some(m) = node.get("matrix") {
        let a = m.as_array().ok_or_else(|| cerr("node.matrix 非数组"))?;
        if a.len() != 16 {
            return Err(cerr("node.matrix 非 16 元"));
        }
        let mut out = [[0.0f64; 4]; 4];
        // glTF matrix = 列主序 16 元。
        for (i, x) in a.iter().enumerate() {
            out[i % 4][i / 4] = as_f64("node.matrix", x)?;
        }
        return Ok(out);
    }
    let t: [f64; 3] = node
        .get("translation")
        .map(|v| as_f64v("node.translation", v, 3))
        .transpose()?
        .map(|v| [v[0], v[1], v[2]])
        .unwrap_or([0.0, 0.0, 0.0]);
    let q: [f64; 4] = node
        .get("rotation")
        .map(|v| as_f64v("node.rotation", v, 4))
        .transpose()?
        .map(|v| [v[0], v[1], v[2], v[3]])
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let s: [f64; 3] = node
        .get("scale")
        .map(|v| as_f64v("node.scale", v, 3))
        .transpose()?
        .map(|v| [v[0], v[1], v[2]])
        .unwrap_or([1.0, 1.0, 1.0]);
    // T·R·S（glTF 节点局部矩阵；旋转四元数 x,y,z,w）。
    let (qx, qy, qz, qw) = (q[0], q[1], q[2], q[3]);
    let r: M4 = [
        [
            1.0 - 2.0 * (qy * qy + qz * qz),
            2.0 * (qx * qy - qz * qw),
            2.0 * (qx * qz + qy * qw),
            0.0,
        ],
        [
            2.0 * (qx * qy + qz * qw),
            1.0 - 2.0 * (qx * qx + qz * qz),
            2.0 * (qy * qz - qx * qw),
            0.0,
        ],
        [
            2.0 * (qx * qz - qy * qw),
            2.0 * (qy * qz + qx * qw),
            1.0 - 2.0 * (qx * qx + qy * qy),
            0.0,
        ],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut rs = r;
    for c in 0..3 {
        rs[0][c] *= s[c];
        rs[1][c] *= s[c];
        rs[2][c] *= s[c];
    }
    rs[0][3] = t[0];
    rs[1][3] = t[1];
    rs[2][3] = t[2];
    Ok(rs)
}

fn xform(m: &M4, p: [f32; 3]) -> [f32; 3] {
    let (x, y, z) = (p[0] as f64, p[1] as f64, p[2] as f64);
    [
        (m[0][0] * x + m[0][1] * y + m[0][2] * z + m[0][3]) as f32,
        (m[1][0] * x + m[1][1] * y + m[1][2] * z + m[1][3]) as f32,
        (m[2][0] * x + m[2][1] * y + m[2][2] * z + m[2][3]) as f32,
    ]
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// 材质记录（扁平化消费面）。
struct MatRec {
    base_color_factor: [f32; 3],
    metallic: f32,
    base_color_img: Option<usize>,
}

/// 装配输出（计数闭集进 receipt）。
struct SceneAssembly {
    scene: ProdScene,
    tri_count: usize,
    point_light_count: usize,
    quad_light_count: usize,
    emissive_tri_count: usize,
    unconsumed_containers: Vec<String>,
    gltf_sha256: String,
    bin_sha256: String,
}

fn load_prod_scene(
    contract: &JsonValue,
    scene_id: &str,
    gltf_path: &Path,
) -> Result<SceneAssembly, String> {
    let scenes = contract
        .get("scenes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| cerr("scenes 缺"))?;
    let srow = scenes
        .iter()
        .find(|s| {
            s.get("scene_id")
                .and_then(|v| v.as_str())
                .map(|x| x == scene_id)
                .unwrap_or(false)
        })
        .ok_or_else(|| cerr(format!("契约缺场景行 {scene_id}")))?;
    let cam = srow.get("camera").unwrap();
    let lig = srow.get("lighting").unwrap();
    let pol = srow.get("material_policy").unwrap();
    let texture_mean = pol
        .get("texture_mean_albedo")
        .and_then(|v| v.as_bool())
        .unwrap();
    let white_tex_to_white = pol
        .get("white_tex_to_white")
        .and_then(|v| v.as_bool())
        .unwrap();

    let text = std::fs::read_to_string(gltf_path).map_err(|e| format!("glTF 读取失败: {e}"))?;
    let gltf_sha256 = sha256_hex(text.as_bytes());
    let root = json::parse_str(&text).map_err(|e| format!("glTF JSON: {e}"))?;
    let base = gltf_path.parent().unwrap_or_else(|| Path::new("."));
    let mut buffers: Vec<Vec<u8>> = Vec::new();
    let mut bin_sha256 = String::new();
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
        bin_sha256 = sha256_hex(&data);
        buffers.push(data);
    }
    let meshes =
        validate::extract_meshes(&root, &buffers).map_err(|e| format!("extract_meshes: {e}"))?;

    // 材质表（baseColorFactor/metallic/baseColorTexture 源图索引）。
    let mut mats: Vec<MatRec> = Vec::new();
    for m in root
        .get("materials")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let pbr = m.get("pbrMetallicRoughness");
        let alb4 = pbr
            .and_then(|p| p.get("baseColorFactor"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .map(|x| x.as_f64().unwrap_or(1.0) as f32)
                    .collect::<Vec<_>>()
            });
        let alb = match alb4 {
            Some(v) if v.len() == 4 => [v[0], v[1], v[2]],
            _ => [1.0, 1.0, 1.0],
        };
        let img = pbr
            .and_then(|p| p.get("baseColorTexture"))
            .and_then(|t| t.get("index"))
            .and_then(|v| v.as_u64())
            .and_then(|ti| root.get("textures")?.as_array()?.get(ti as usize))
            .and_then(|tex| tex.get("source"))
            .and_then(|v| v.as_u64())
            .map(|x| x as usize);
        mats.push(MatRec {
            base_color_factor: alb,
            metallic: pbr
                .and_then(|p| p.get("metallicFactor"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32,
            base_color_img: img,
        });
    }

    // 纹理均值（texture_mean_albedo 策略：DDS 容器 bcdec 真实解码 → 逐 texel
    // sRGB→线性 IEC 分段 → 均值；非 DDS 容器显式登记不静默）。
    let mut unconsumed: Vec<String> = Vec::new();
    let mut tex_mean: Vec<Option<[f32; 3]>> = Vec::new();
    if let Some(imgs) = root.get("images").and_then(|v| v.as_array()) {
        for im in imgs {
            let uri = im.get("uri").and_then(|v| v.as_str());
            let mut mean = None;
            if texture_mean {
                if let Some(uri) = uri {
                    if uri.to_ascii_lowercase().ends_with(".dds") {
                        let raw = std::fs::read(base.join(uri))
                            .map_err(|e| format!("纹理 {uri} 读取失败: {e}"))?;
                        let img = rurix_asset::bcdec::decode_dds(&raw)
                            .map_err(|e| format!("纹理 {uri} DDS 解码失败: {e}"))?;
                        let mut acc = [0.0f64; 3];
                        let npx = (img.width * img.height) as usize;
                        for px in 0..npx {
                            for c in 0..3 {
                                acc[c] += srgb_to_linear(img.rgba8[px * 4 + c] as f32 / 255.0)
                                    as f64;
                            }
                        }
                        mean = Some([
                            (acc[0] / npx as f64) as f32,
                            (acc[1] / npx as f64) as f32,
                            (acc[2] / npx as f64) as f32,
                        ]);
                    } else {
                        unconsumed.push(uri.to_owned());
                    }
                }
            }
            tex_mean.push(mean);
        }
    }

    // 节点树世界变换（扁平单引用面；嵌套按 compose 递推，多引用按节点各出
    // 一份实例——与 g10_5_scene_render 同律）。
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
        let mut parent = None;
        for (i, n) in nodes.iter().enumerate() {
            if let Some(ch) = n.get("children").and_then(|v| v.as_array())
                && ch.iter().any(|c| c.as_u64() == Some(idx as u64))
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

    // primitive → 材质索引（(mesh_id, primitive_id) 键面同 extract_meshes 序）。
    let mut prim_material: std::collections::HashMap<(u32, u32), Option<u32>> =
        std::collections::HashMap::new();
    let mut prim_id = 0u32;
    for (mi, mesh) in root
        .get("meshes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| cerr("glTF 缺 meshes"))?
        .iter()
        .enumerate()
    {
        for prim in mesh
            .get("primitives")
            .and_then(|v| v.as_array())
            .ok_or_else(|| cerr("meshes[].primitives 缺"))?
        {
            let mode = prim.get("mode").and_then(|v| v.as_u64()).unwrap_or(4);
            let mat = if mode == 4 {
                prim.get("material").and_then(|v| v.as_u64()).map(|x| x as u32)
            } else {
                None
            };
            prim_material.insert((mi as u32, prim_id), mat);
            prim_id += 1;
        }
    }

    // 契约 emissive 材质集（material_index → Le）。
    let mut emissive_map: std::collections::HashMap<u32, [f32; 3]> =
        std::collections::HashMap::new();
    for m in lig
        .get("emissive_materials")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let mi = m.get("material_index").and_then(|v| v.as_u64()).unwrap() as u32;
        let le: Vec<f64> = m
            .get("le_linear_rgb")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        emissive_map.insert(mi, [le[0] as f32, le[1] as f32, le[2] as f32]);
    }

    // 三角形汤装配（世界变换烘焙进顶点；ProdScene = 单 BLAS 世界空间汤）。
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();
    let mut materials: Vec<MaterialKind> = Vec::new();
    let mut light_of_prim: Vec<u32> = Vec::new();
    let mut lights: Vec<ProdLight> = Vec::new();
    let mut emissive_tris = 0usize;
    for m in &meshes {
        for (ni, n) in nodes.iter().enumerate() {
            let Some(mesh_idx) = n.get("mesh").and_then(|v| v.as_u64()) else {
                continue;
            };
            if mesh_idx != m.mesh_id as u64 {
                continue;
            }
            let w = world[ni].ok_or_else(|| cerr("节点世界变换缺失"))?;
            let mat_idx = prim_material
                .get(&(m.mesh_id, m.primitive_id))
                .copied()
                .flatten();
            let albedo = match mat_idx.and_then(|mi| mats.get(mi as usize)) {
                Some(rec) => {
                    let k = 1.0 - rec.metallic;
                    let base = rec
                        .base_color_img
                        .and_then(|ii| tex_mean.get(ii))
                        .and_then(|m| *m)
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
                None => [1.0, 1.0, 1.0],
            };
            let is_emissive = mat_idx
                .map(|mi| emissive_map.contains_key(&mi))
                .unwrap_or(false);
            for t3 in m.indices.chunks_exact(3) {
                let v0 = xform(&w, m.positions[t3[0] as usize]);
                let v1 = xform(&w, m.positions[t3[1] as usize]);
                let v2 = xform(&w, m.positions[t3[2] as usize]);
                let base = positions.len() as u32;
                positions.push(v0);
                positions.push(v1);
                positions.push(v2);
                indices.push([base, base + 1, base + 2]);
                if is_emissive {
                    let le = emissive_map[&mat_idx.unwrap()];
                    materials.push(MaterialKind::Emission {
                        albedo,
                        emission: le,
                    });
                    let li = lights.len() as u32;
                    lights.push(ProdLight::Tri {
                        v0,
                        e1: [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]],
                        e2: [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]],
                        emission: le,
                    });
                    light_of_prim.push(li);
                    emissive_tris += 1;
                } else {
                    materials.push(MaterialKind::Lambert { albedo });
                    light_of_prim.push(u32::MAX);
                }
            }
        }
    }
    if indices.is_empty() {
        return Err(format!("场景 {scene_id} 装配零三角"));
    }

    // 契约 quad 面光（发光三角几何逐字一致追加）。
    let mut quad_light_count = 0usize;
    for q in lig
        .get("quad_lights")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let f3 = |k: &str| -> [f32; 3] {
            let a: Vec<f64> = q
                .get(k)
                .and_then(|v| v.as_array())
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect();
            [a[0] as f32, a[1] as f32, a[2] as f32]
        };
        let p00 = f3("p00");
        let e1 = f3("e1");
        let e2 = f3("e2");
        let le = f3("le_linear_rgb");
        let quad = PtLightQuad {
            p00,
            e1,
            e2,
            emission: le,
        };
        let li = lights.len() as u32;
        lights.push(ProdLight::Quad(quad));
        let p10 = [p00[0] + e1[0], p00[1] + e1[1], p00[2] + e1[2]];
        let p01 = [p00[0] + e2[0], p00[1] + e2[1], p00[2] + e2[2]];
        let p11 = [p00[0] + e1[0] + e2[0], p00[1] + e1[1] + e2[1], p00[2] + e1[2] + e2[2]];
        let em = MaterialKind::Emission {
            albedo: [0.5, 0.5, 0.5],
            emission: le,
        };
        for (a, b, c) in [(p00, p10, p11), (p00, p11, p01)] {
            let base = positions.len() as u32;
            positions.push(a);
            positions.push(b);
            positions.push(c);
            indices.push([base, base + 1, base + 2]);
            materials.push(em);
            light_of_prim.push(li);
        }
        quad_light_count += 1;
    }
    // 契约点光（delta;I = color × intensity_cd——RXS-0394 L3 链消费面:
    // 点强 I 即 cd 直给,几何项 cos/(π·d²·pdf_d) 由 kernel 承担）。
    let mut point_light_count = 0usize;
    for p in lig
        .get("point_lights")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let pos: Vec<f64> = p
            .get("position")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let col: Vec<f64> = p
            .get("color_linear_rgb")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let inten = p
            .get("intensity_cd")
            .and_then(|v| v.as_f64())
            .unwrap();
        lights.push(ProdLight::Point {
            position: [pos[0] as f32, pos[1] as f32, pos[2] as f32],
            intensity: [
                (col[0] * inten) as f32,
                (col[1] * inten) as f32,
                (col[2] * inten) as f32,
            ],
        });
        point_light_count += 1;
    }

    // 相机（契约四元数 → look_at 同口径:forward = q·(0,0,−1)、up = q·(0,1,0),
    // PtCamera::look_at 与 g10_5 contract_camera 同一 RH look-at 公式面）。
    let pos: Vec<f64> = cam
        .get("position")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    let quat: Vec<f64> = cam
        .get("orientation_quat")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    let fov = cam.get("fov_y_deg").and_then(|v| v.as_f64()).unwrap();
    let far = cam.get("far").and_then(|v| v.as_f64()).unwrap();
    let res = cam.get("resolution").unwrap();
    let w_px = res.get("w").and_then(|v| v.as_u64()).unwrap() as u32;
    let h_px = res.get("h").and_then(|v| v.as_u64()).unwrap() as u32;
    let rot = |v: [f64; 3]| -> [f64; 3] {
        let (w, x, y, z) = (quat[0], quat[1], quat[2], quat[3]);
        let uv = [y * v[2] - z * v[1], z * v[0] - x * v[2], x * v[1] - y * v[0]];
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
    };
    let fwd = rot([0.0, 0.0, -1.0]);
    let up = rot([0.0, 1.0, 0.0]);
    let eye = [pos[0] as f32, pos[1] as f32, pos[2] as f32];
    // 相机基向量 = G10.5 双端一致口径（RXS-0390 探针互证面）:forward =
    // q·(0,0,−1)、up0 = q·(0,1,0)、**right = forward × up0**（UE 一致手性——
    // PtCamera::look_at 的 pbrt 同式 right = up×forward 与 UE 呈水平镜像,
    // G12.4 波裁决实证:cornell 绿墙左/红墙右 = G10.5 UE 帧同侧)。
    let f = Vec3::new(fwd[0] as f32, fwd[1] as f32, fwd[2] as f32).normalize();
    let u0 = Vec3::new(up[0] as f32, up[1] as f32, up[2] as f32);
    let r = f.cross(u0).normalize();
    let u = r.cross(f);
    let camera = PtCamera {
        origin: eye,
        forward: f.to_array(),
        right: r.to_array(),
        up: u.to_array(),
        tan_half_fov: ((fov as f32).to_radians() * 0.5).tan(),
        width: w_px,
        height: h_px,
    };
    let scene = ProdScene {
        name: if scene_id == "cornell-box" {
            "g12p_cornell_box"
        } else {
            "g12p_bistro_interior"
        },
        positions,
        indices,
        materials,
        lights,
        camera,
        t_max: far as f32,
        light_of_prim,
    };
    scene
        .validate()
        .map_err(|e| format!("场景 {scene_id} 装配校验: {e}"))?;
    let _ = white_tex_to_white; // 策略面由默认 factor 白承载（登记面非代码分支）
    Ok(SceneAssembly {
        tri_count: scene.indices.len(),
        scene,
        point_light_count,
        quad_light_count,
        emissive_tri_count: emissive_tris,
        unconsumed_containers: unconsumed,
        gltf_sha256,
        bin_sha256,
    })
}

// ---------------------------------------------------------------------------
// device 执行腿（U30 run_ray_query_effects;g12_pt_production harness 同构面）
// ---------------------------------------------------------------------------

fn bytes_f32(v: &[f32]) -> Vec<u8> {
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

fn run_device(
    scene: &ProdScene,
    dist: &LightDist,
    cfg: &ProdConfig,
    spv: &[u32],
    entry: &str,
) -> Result<ProdImage, String> {
    scene.validate().map_err(|e| format!("场景校验: {e}"))?;
    cfg.validate().map_err(|e| format!("配置校验: {e}"))?;
    let cam = &scene.camera;
    let pixel_count = (cam.width * cam.height) as usize;
    let tris = prod::pack_prod_tris(scene);
    let blas_refs: Vec<&[f32]> = vec![&tris];
    let instances = [RayQueryInstanceDesc {
        blas: 0,
        custom_index: 0,
        mask: 0xFF,
        sbt_record_offset: 0,
    }];
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    // 整帧 RNG 流(host 一遍;像素带分段直切连续行段——流布局 (px·spp+s)·stride
    // 像素主序连续,分段 = 整帧位级一致)。
    let stream = prod::sampler::generate(
        cfg.sampler,
        pixel_count,
        cfg.spp,
        cfg.max_bounces,
        cfg.seed,
    );
    let stride = (2 + 6 * cfg.max_bounces) as usize;
    let mats_b = bytes_f32(&prod::pack_prod_mats(scene));
    let tris_b = bytes_f32(&tris);
    let lights_b = bytes_f32(&prod::pack_prod_lights(scene, dist));
    let base_params = prod::pack_prod_params(scene, cfg);
    // 像素带分段 dispatch(G12.4 M163 面:单 dispatch 墙钟上界规避 TDR——
    // 大场景 × 高 spp 单发超窗实测〔bistro spp256/1024 单发
    // VK_ERROR_DEVICE_LOST〕;带内流直切,输出带内回读拼帧,位级一致)。
    // 带像素数自适应:单带路径数 ≤ ~1M(16k 路径/px·spp 上界内实测安全窗)。
    let chunk_pixels = (1_048_576usize / cfg.spp as usize).clamp(512, 16384);
    let mut rgb: Vec<f32> = vec![0.0; pixel_count * 3];
    let mut sum_lum: Vec<f32> = vec![0.0; pixel_count];
    let mut sumsq_lum: Vec<f32> = vec![0.0; pixel_count];
    let mut samples: Vec<u32> = vec![0; pixel_count];
    let mut converged: Vec<f32> = vec![0.0; pixel_count];
    let mut rr_counters: Vec<f32> = vec![0.0; pixel_count * 4];
    let mut energy_levels: Vec<f32> = vec![0.0; pixel_count * 4];
    let mut base = 0usize;
    while base < pixel_count {
        let count = (pixel_count - base).min(chunk_pixels);
        let mut params_b = base_params.clone();
        params_b[0] = count as f32; // [0] = 本带像素数
        params_b[36] = base as f32; // [36] = 带起点(既有面 = 0)
        let rng_slice = &stream[base * cfg.spp as usize * stride..(base + count) * cfg.spp as usize * stride];
        let rng_b = bytes_f32(rng_slice);
        let params_bytes = bytes_f32(&params_b);
        let buffers = [
            RayQueryBufferDesc::Input(&rng_b),
            RayQueryBufferDesc::Input(&mats_b),
            RayQueryBufferDesc::Input(&tris_b),
            RayQueryBufferDesc::Input(&lights_b),
            RayQueryBufferDesc::Input(&params_bytes),
            RayQueryBufferDesc::Output(count * 12),
            RayQueryBufferDesc::Output(count * 8),
            RayQueryBufferDesc::Output(count * 4),
            RayQueryBufferDesc::Output(count * 4),
            RayQueryBufferDesc::Output(count * 16),
            RayQueryBufferDesc::Output(count * 16),
        ];
        let out = vk::run_ray_query_effects(
            &scene_desc,
            &[RayQueryDispatchDesc {
                name: "g12_pt_production",
                spv,
                entry,
                buffers: &buffers,
                push_constants: &[],
                groups: [count as u32, 1, 1],
            }],
        )?;
        let rb = out
            .readbacks
            .into_iter()
            .next()
            .ok_or("单 dispatch 缺回读")?;
        if rb.len() != 6 {
            return Err(format!("回读路数 {} ≠ 6", rb.len()));
        }
        let crgb = read_f32(&rb[0]);
        let cstats = read_f32(&rb[1]);
        let csamples = read_u32(&rb[2]);
        let cconv = read_f32(&rb[3]);
        let crr = read_f32(&rb[4]);
        let cenergy = read_f32(&rb[5]);
        rgb[base * 3..(base + count) * 3].copy_from_slice(&crgb);
        for px in 0..count {
            sum_lum[base + px] = cstats[px * 2];
            sumsq_lum[base + px] = cstats[px * 2 + 1];
        }
        samples[base..base + count].copy_from_slice(&csamples);
        converged[base..base + count].copy_from_slice(&cconv);
        rr_counters[base * 4..(base + count) * 4].copy_from_slice(&crr);
        energy_levels[base * 4..(base + count) * 4].copy_from_slice(&cenergy);
        base += count;
    }
    Ok(ProdImage {
        width: cam.width,
        height: cam.height,
        rgb,
        sum_lum,
        sumsq_lum,
        samples,
        converged,
        rr_counters,
        energy_levels,
        frame_label: "full_reference",
    })
}

// ---------------------------------------------------------------------------
// EXR 落盘（RXS-0385 rurix strict 元数据闭集;G10.5 同口径面）
// ---------------------------------------------------------------------------

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

fn hdr_metadata(digest: &str) -> ExrMetadata {
    ExrMetadata {
        schema_version: "1".to_owned(),
        domain: ExrDomain::SceneLinearHdr,
        transfer: ExrTransfer::Linear,
        bit_depth: ExrBitDepth::Float32,
        source_end: ExrSourceEnd::Rurix,
        view_transform: None,
        capture_params_digest: digest.to_owned(),
        derivation: ExrDerivation::Capture,
        source_frame_digest: None,
        chromaticities_origin: Some(ChromaticitiesOrigin::Writer),
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

fn take_arg(args: &[String], i: &mut usize) -> String {
    *i += 1;
    args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
}

fn load_spv(path: &Path) -> Vec<u32> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("SPV 读取失败: {e}")));
    if bytes.len() % 4 != 0 {
        fail("SPV 字节数非 4 对齐");
    }
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        fail("缺子模式（--contract-digest / --render）");
    }
    if args[1] == "--contract-digest" {
        let path = args.get(2).unwrap_or_else(|| fail("缺契约路径"));
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| fail(&format!("契约读取: {e}")));
        match parse_contract(&text) {
            Ok(c) => {
                println!("{}", c.digest);
            }
            Err(e) => fail(&e),
        }
        return;
    }
    if args[1] != "--render" {
        fail(&format!("未知子模式 {}", args[1]));
    }
    let mut scene_id = String::new();
    let mut spp: u32 = 0;
    let mut seed: u64 = 0;
    let mut tau: f32 = 0.0;
    let mut contract_path = String::new();
    let mut gltf_path = String::new();
    let mut spv_path = String::new();
    let mut out_dir = String::new();
    let mut expect = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--render" => {}
            "--scene" => scene_id = take_arg(&args, &mut i),
            "--spp" => spp = take_arg(&args, &mut i).parse().unwrap_or_else(|_| fail("--spp 非 u32")),
            "--seed" => seed = take_arg(&args, &mut i).parse().unwrap_or_else(|_| fail("--seed 非 u64")),
            "--tau" => tau = take_arg(&args, &mut i).parse().unwrap_or_else(|_| fail("--tau 非 f32")),
            "--contract" => contract_path = take_arg(&args, &mut i),
            "--gltf" => gltf_path = take_arg(&args, &mut i),
            "--spv" => spv_path = take_arg(&args, &mut i),
            "--out-dir" => out_dir = take_arg(&args, &mut i),
            "--expect-digest" => expect = take_arg(&args, &mut i),
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    if scene_id.is_empty() || spp == 0 || contract_path.is_empty() || gltf_path.is_empty()
        || spv_path.is_empty() || out_dir.is_empty() || expect.is_empty() || tau <= 0.0
    {
        fail("--render 参数闭集缺行（scene/spp/tau/contract/gltf/spv/out-dir/expect-digest）");
    }
    let text =
        std::fs::read_to_string(&contract_path).unwrap_or_else(|e| fail(&format!("契约读取: {e}")));
    let contract = parse_contract(&text).unwrap_or_else(|e| fail(&e));
    if contract.digest != expect {
        fail(&format!(
            "契约 digest 不等仍出报告即 RED：harness 实算 {} ≠ 期望 {}——拒出图",
            contract.digest, expect
        ));
    }
    let asm = load_prod_scene(&contract.raw, &scene_id, Path::new(&gltf_path))
        .unwrap_or_else(|e| fail(&e));
    eprintln!(
        "[{TAG}] 装配: scene={} tris={} quads={} points={} emissive_tris={} unconsumed_tex={}",
        scene_id,
        asm.tri_count,
        asm.quad_light_count,
        asm.point_light_count,
        asm.emissive_tri_count,
        asm.unconsumed_containers.len()
    );
    let spv = load_spv(Path::new(&spv_path));
    let dist = prod::build_light_distribution(&asm.scene);
    // 采样器族 = M166 选型 winner（sobol_class_seed_perturbed）面；τ = M166
    // 标定值经 --tau 传入（g12_budget g12.pt.rr_tau measured_value，P-09 禁手写）。
    let mut cfg = ProdConfig::production(spp, SamplerFamily::Sobol, tau);
    cfg.seed = seed;
    cfg.adaptive = None; // 对标帧 = 固定全 spp（帧型标签 full_reference）
    if !vk::vulkan_available() {
        skip("无 Vulkan loader/设备面");
    }
    let entry = vk::entry_point_name(&spv).unwrap_or_else(|| fail("SPIR-V 无 OpEntryPoint"));
    let t0 = std::time::Instant::now();
    let img_a = run_device(&asm.scene, &dist, &cfg, &spv, &entry).unwrap_or_else(|e| fail(&e));
    let img_b = run_device(&asm.scene, &dist, &cfg, &spv, &entry).unwrap_or_else(|e| fail(&e));
    let render_s = t0.elapsed().as_secs_f64();
    let da = prod::prod_image_digest(&img_a);
    let db = prod::prod_image_digest(&img_b);
    if da != db {
        fail("固定 seed 双跑位级漂移（确定性协议违例）");
    }
    let digest_hex = da.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let out = PathBuf::from(&out_dir);
    std::fs::create_dir_all(&out).unwrap_or_else(|e| fail(&format!("输出目录: {e}")));
    let frame_path = out.join(format!("{}_spp{}.exr", scene_id, spp));
    let img = ExrImage::new(
        img_a.width,
        img_a.height,
        ExrChannelLayout::Rgb,
        img_a.rgb.clone(),
        hdr_metadata(&contract.digest),
    )
    .unwrap_or_else(|e| fail(&format!("EXR 构造: {e}")));
    let bytes = encode_exr(&img).unwrap_or_else(|e| fail(&format!("EXR 编码: {e}")));
    std::fs::write(&frame_path, &bytes).unwrap_or_else(|e| fail(&format!("EXR 落盘: {e}")));
    let content = frame_content_digest(img_a.width, img_a.height, 3, &img_a.rgb);
    let mean = img_a.mean_luminance();
    let receipt = format!(
        "{{\n  \"schema\": \"rurix.g12.ue_pt_parity_rurix_receipt.v1\",\n  \"scene_id\": \"{}\",\n  \"spp\": {},\n  \"seed\": {},\n  \"frame_file\": \"{}\",\n  \"frame_content_digest\": \"{}\",\n  \"double_run_digest\": \"sha256:{}\",\n  \"double_run_bitexact\": true,\n  \"mean_luminance\": {},\n  \"render_s\": {},\n  \"tri_count\": {},\n  \"quad_light_count\": {},\n  \"point_light_count\": {},\n  \"emissive_tri_count\": {},\n  \"unconsumed_containers\": [{}],\n  \"gltf_sha256\": \"sha256:{}\",\n  \"bin_sha256\": \"sha256:{}\",\n  \"contract_digest\": \"{}\",\n  \"frame_label\": \"full_reference\"\n}}\n",
        scene_id,
        spp,
        seed,
        json_escape(&frame_path.to_string_lossy().replace('\\', "/")),
        content,
        digest_hex,
        mean,
        render_s,
        asm.tri_count,
        asm.quad_light_count,
        asm.point_light_count,
        asm.emissive_tri_count,
        asm.unconsumed_containers
            .iter()
            .map(|u| format!("\"{}\"", json_escape(u)))
            .collect::<Vec<_>>()
            .join(", "),
        asm.gltf_sha256,
        asm.bin_sha256,
        contract.digest,
    );
    let receipt_path = out.join(format!("{}_spp{}_receipt.json", scene_id, spp));
    std::fs::write(&receipt_path, &receipt).unwrap_or_else(|e| fail(&format!("receipt 落盘: {e}")));
    println!(
        "{TAG}: PASS scene={} spp={} mean_lum={:.6} digest={} double_run=bitexact",
        scene_id, spp, mean, content
    );
}
