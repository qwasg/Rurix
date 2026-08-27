// Assisted-by: Kimi-K3（G13.4 UE 对拍波）
//! G13.4 M-c UE 超分对拍 Rurix 臂 render harness（spec/visual_comparison.md
//! RXS-0405 L1/L2/L3；门 `g13.p0.m_c.ue_upscale_parity`；G13_CONTRACT §4.2 M-c 行）。
//!
//! ## 职责闭集
//!
//! 1. **契约严格解析 + canonical digest 第二实现**（RXS-0405 L1 字段闭集
//!    fail-closed + L2 `G13USP-1\0` 前缀 + RXS-0384 L3 同构标签/键序/宽度规则；
//!    digest 域 = 字段闭集除 provenance）——`--contract-digest` 打印
//!    `sha256:…`（三方互证面：host python 参照
//!    `milestones/g13/harness/ue_python/g13_parity_contract.py` 实测
//!    sha256:137483a1696481971fc0da03fad1a188ef6f048243e4616953060014f1d0872f，
//!    本实现必须逐字全等）；`--render` 实算 digest ≠ 冻结注册值即拒出图
//!    （**契约 digest 不等仍出报告即 RED** 的 harness 承载面，M163 同族门序）。
//! 2. **真场景帧序渲染**（RXS-0405 L3）：M133 清单双场景闭集（cornell-box /
//!    bistro-interior）glTF 真几何 + 契约逐字段相机/光照/exposure/material_policy
//!    ——bin-local 最小 glTF 装载（rurix-render 不依赖 rurix-asset，循环依赖禁
//!    区；几何 accessors + 节点树世界变换烘焙 + 逐三角材质扁平化：bistro
//!    `texture_mean_albedo` 策略 = DDS BC1/BC3 真实解码均值线性域 ×
//!    baseColorFactor × (1−metallic)，非 BC1/BC3 容器 fail-closed 显式拒绝不
//!    静默；cornell `white_tex_to_white` 策略 = 默认 factor 白承载，G12.4 同
//!    律登记面非代码分支）。主射线车道 = host `rt::bvh::TriBvh`（真求交，
//!    深度 = ZO NDC round-trip 与 `temporal::common::compute_camera_mv` 同一
//!    约定面）；着色 = 双面 Lambert 直接光（契约 quad 面光 4×4 分层确定性采样
//!    + 点光 delta + emissive 材质主命中，逐灯 BVH 阴影射线；无 GI/天光——
//!    契约 sun/sky = 0.0 显式登记，直接光口径 receipt 显式登记不冒充 GI 帧）。
//!    逐帧 Halton jitter（契约 seed 派生窗口，RXS-0357 L2 固定 seed 位级确定性
//!    继承；`--calibration-seed` 用 calibration_seed），内部分辨率 =
//!    floor(输出 × tier%) 双向 floor 同一口径；MV = 相机分量（jittered
//!    view-proj 经 `compute_camera_mv` 公共底座，禁私写重投影）。
//! 3. **逐后端 upscale + 帧库落盘**：`UpscaleBackend` 冻结面（RFC-0016 §4.0-3，
//!    trait 签名面 0-byte）——`tsr_device` = M-b bin-local `TsrDeviceBackend`
//!    同模式（.rx kernel SPV 经 `rurix_rt::vk::run_compute` 双腿 dispatch）；
//!    `dlss_sr` / `fsr_3_1_5` = M-a bin-local adapter 同模式（只消费
//!    `rurix-rt::vendor_upscale` safe 公共面；vendor FFI unsafe 全留 rurix-rt
//!    既有面，本 bin forbid(unsafe_code)）。逐帧 HDR EXR（RXS-0385 rurix
//!    strict 元数据闭集，G10.5/G12.4 同 `image-io` 写出面）+ converged.exr
//!    （末帧 = 时域收敛帧）+ render_receipt.json（契约 digest / 逐帧
//!    {name,bytes,digest} / frame_ms / converged_digest / env 面）。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/vendor DLL/场景资产 → `SKIP DEV_ENV_DEGRADE`（退 0，
//! 非 fake pass；`RURIX_REQUIRE_REAL=1` 下缺真实面即 FAIL 退 1，禁 mock 充真
//! 跑——M-a/M-b 同语义）。契约解析违例/digest 不等/双跑位级漂移 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g13_4_ue_upscale_parity_render --contract-digest [--contract <contract.json>]
//! g13_4_ue_upscale_parity_render --selftest-digest [--contract <contract.json>]
//! g13_4_ue_upscale_parity_render --render --scene <cornell-box|bistro-interior> \
//!     --tier <50|67|100> --backend <tsr_device|dlss_sr|fsr_3_1_5> [--frames 32] \
//!     [--calibration-seed] [--contract <contract.json>] [--gltf <scene.gltf>] \
//!     [--spv-resample <a.spv> --spv-resolve <b.spv>] [--out-root <dir>] \
//!     [--expect-digest <sha256:…>]
//! ```

#![forbid(unsafe_code)]

use image_io::exr::{
    ChromaticitiesOrigin, ExrBitDepth, ExrChannelLayout, ExrDerivation, ExrDomain, ExrImage,
    ExrMetadata, ExrSourceEnd, ExrTransfer, encode_exr,
};
use rurix_render::rt::bvh::{Ray, TriBvh, Vec3};
use rurix_render::temporal::common::{
    Mat4, compute_camera_mv, halton, look_at_rh, perspective_rh_zo,
};
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::tsr::TsrParams;
use rurix_render::temporal::upscale::{UpscaleBackend, UpscaleInputs};
use rurix_rt::vendor_upscale::{
    DlssVkSession, FsrDx12Session, VendorFrameInput, VendorSessionReport, fsr_sdk_dir,
    streamline_sdk_dir,
};
use rurix_rt::vk;
use std::path::{Path, PathBuf};

const TAG: &str = "[g13_4_upscale_parity]";
/// RXS-0405 L2 冻结版本前缀（47 31 33 55 53 50 2D 31 00）。
const VERSION_PREFIX: &[u8] = b"G13USP-1\0";
const SCHEMA_ID: &str = "rurix.g13.ue_upscale_parity_contract.v1";
// M-d（RXS-0406）契约面：digest 三方臂②分派（解析/编码与 upscale 契约同律，
// 前缀 G13LGP-1\0 + 根/policy 字段类型表分派；渲染腿仅 upscale 契约）。
const LUMEN_SCHEMA_ID: &str = "rurix.g13.ue_lumen_gi_parity_contract.v1";
const LUMEN_PREFIX: &[u8] = b"G13LGP-1\0";
/// 三方互证冻结注册值（host python 参照实测；--render 默认比对锚）。
const FROZEN_CONTRACT_DIGEST: &str =
    "sha256:137483a1696481971fc0da03fad1a188ef6f048243e4616953060014f1d0872f";
/// --selftest-digest 内置最小合成对象 digest 锚（python 独立实现产，见 leg 注释）。
const SELFTEST_TINY_DIGEST: &str =
    "sha256:4b091627caebafdcbd85fd877c6fa969430337af1fa23a774e6d25b432616c62";
const UNIT_NORM_TOL: f64 = 9.094947017729282e-13; // 2^-40（RXS-0384 L2 谓词常量）
const DEFAULT_CONTRACT: &str = "milestones/g13/g13_ue_upscale_parity_contract.json";
const DEFAULT_OUT_ROOT: &str = "K:/rurix-ext/g13-frames/rurix_upscale";
const DEFAULT_SPV_RESAMPLE: &str = ".tmp/g13_gates/m_b/g13_tsr_resample.spv";
const DEFAULT_SPV_RESOLVE: &str = ".tmp/g13_gates/m_b/g13_tsr_resolve.spv";
/// jitter 派生窗口模数（素数；base = seed % 65521，receipt 协议字面登记）。
const JITTER_WINDOW_MOD: u64 = 65521;
/// quad 面光分层采样边数（4×4 = 16 确定性样本）。
const QUAD_SAMPLES: u32 = 4;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn sha256_hex(data: &[u8]) -> String {
    rurix_pkg::sha256::hex_digest(data)
}

fn require_real() -> bool {
    std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1")
}

/// DEV_ENV 三态裁决：RURIX_REQUIRE_REAL=1 → FAIL 退 1；否则 SKIP 退 0（非 fake pass）。
fn dev_env_or_fail(what: &str, err: &str) -> ! {
    if require_real() {
        fail(&format!(
            "{what} 不可用（RURIX_REQUIRE_REAL=1，禁 mock 充真跑）: {err}"
        ));
    }
    println!(
        "{{\"schema\":\"rurix.g13.upscale_parity.skip.v1\",\"state\":\"skipped_dev_env\",\"what\":{},\"reason\":{}}}",
        jstr(what),
        jstr(err)
    );
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// 最小 JSON 解析（bin-local 独立实现；int/float 字面区分保留——python json 类型
// 谓词同构面：u32/u64 字段拒 float 字面。重复键拒；控制字符/坏转义拒；深度限 64）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num { raw: String, v: f64, integral: bool },
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num { v, .. } => Some(*v),
            _ => None,
        }
    }
    fn as_u64(&self) -> Option<u64> {
        match self {
            Json::Num {
                raw,
                integral: true,
                ..
            } => raw.parse::<u64>().ok(),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }
    fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
    fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(p) => Some(p),
            _ => None,
        }
    }
}

struct JParser<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

impl<'a> JParser<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.i < self.b.len() && self.b[self.i] == c {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("JSON: 期待 '{}' @{}", c as char, self.i))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        if self.depth >= 64 {
            return Err("JSON: 嵌套深度越 64".into());
        }
        self.ws();
        let Some(&c) = self.b.get(self.i) else {
            return Err("JSON: 意外结尾".into());
        };
        match c {
            b'{' => {
                self.i += 1;
                self.depth += 1;
                let mut pairs: Vec<(String, Json)> = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Obj(pairs));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    if pairs.iter().any(|(ek, _)| ek == &k) {
                        return Err(format!("JSON: 重复键 {k}"));
                    }
                    self.expect(b':')?;
                    let v = self.value()?;
                    pairs.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err("JSON: 对象缺 ,/}".into()),
                    }
                }
                self.depth -= 1;
                Ok(Json::Obj(pairs))
            }
            b'[' => {
                self.i += 1;
                self.depth += 1;
                let mut items = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err("JSON: 数组缺 ,/]".into()),
                    }
                }
                self.depth -= 1;
                Ok(Json::Arr(items))
            }
            b'"' => Ok(Json::Str(self.string()?)),
            b't' => self.lit("true", Json::Bool(true)),
            b'f' => self.lit("false", Json::Bool(false)),
            b'n' => self.lit("null", Json::Null),
            _ => self.number(),
        }
    }

    fn lit(&mut self, s: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            Ok(v)
        } else {
            Err(format!("JSON: 字面 {s} 不符 @{}", self.i))
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.b.get(self.i) != Some(&b'"') {
            return Err(format!("JSON: 期待字符串 @{}", self.i));
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let Some(&c) = self.b.get(self.i) else {
                return Err("JSON: 字符串未闭合".into());
            };
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else {
                        return Err("JSON: 转义未闭合".into());
                    };
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                // 高代理：须紧跟 \uDC00..DFFF 低代理。
                                if self.b.get(self.i) == Some(&b'\\')
                                    && self.b.get(self.i + 1) == Some(&b'u')
                                {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&lo) {
                                        return Err("JSON: 低代理越域".into());
                                    }
                                    0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                } else {
                                    return Err("JSON: 孤高代理".into());
                                }
                            } else if (0xDC00..0xE000).contains(&hi) {
                                return Err("JSON: 孤低代理".into());
                            } else {
                                hi
                            };
                            let ch = char::from_u32(cp).ok_or("JSON: \\u 码点越域")?;
                            out.push(ch);
                        }
                        _ => return Err("JSON: 非法转义".into()),
                    }
                }
                0x00..=0x1F => return Err("JSON: 未转义控制字符".into()),
                _ => {
                    // 原始 UTF-8 字节直透（输入经 read_to_string 保证合法 UTF-8）。
                    let s =
                        std::str::from_utf8(&self.b[self.i - 1..]).map_err(|_| "JSON: UTF-8")?;
                    let ch = s.chars().next().ok_or("JSON: 字符串截断")?;
                    out.push(ch);
                    self.i += ch.len_utf8() - 1;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.i + 4 > self.b.len() {
            return Err("JSON: \\u 截断".into());
        }
        let s = std::str::from_utf8(&self.b[self.i..self.i + 4]).map_err(|_| "JSON: \\u 非 hex")?;
        let v = u32::from_str_radix(s, 16).map_err(|_| "JSON: \\u 非 hex")?;
        self.i += 4;
        Ok(v)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let mut saw_digit = false;
        while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
            self.i += 1;
            saw_digit = true;
        }
        if !saw_digit {
            return Err(format!("JSON: 非法数字 @{start}"));
        }
        let mut integral = true;
        if self.b.get(self.i) == Some(&b'.') {
            integral = false;
            self.i += 1;
            if !self.b.get(self.i).is_some_and(|c| c.is_ascii_digit()) {
                return Err("JSON: 小数点缺位".into());
            }
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        if matches!(self.b.get(self.i), Some(b'e') | Some(b'E')) {
            integral = false;
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            if !self.b.get(self.i).is_some_and(|c| c.is_ascii_digit()) {
                return Err("JSON: 指数缺位".into());
            }
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        let raw = std::str::from_utf8(&self.b[start..self.i])
            .map_err(|_| "JSON: 数字 UTF-8")?
            .to_owned();
        let v: f64 = raw.parse().map_err(|_| format!("JSON: 数字解析 {raw}"))?;
        if !v.is_finite() {
            return Err(format!("JSON: 数字 {raw} 非有限"));
        }
        Ok(Json::Num { raw, v, integral })
    }
}

fn json_parse(text: &str) -> Result<Json, String> {
    let mut p = JParser {
        b: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err("JSON: 尾部余留字节".into());
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// 契约解析（RXS-0405 L1 字段闭集 fail-closed + 约束谓词；python 参照同型逐条）
// ---------------------------------------------------------------------------

fn cerr(msg: impl Into<String>) -> String {
    format!("契约解析: {}", msg.into())
}

fn as_f64(name: &str, v: &Json) -> Result<f64, String> {
    let x = v
        .as_f64()
        .ok_or_else(|| cerr(format!("{name}: expected f64")))?;
    if !x.is_finite() {
        return Err(cerr(format!("{name}: NaN/Inf forbidden")));
    }
    Ok(x)
}

fn as_u32(name: &str, v: &Json) -> Result<u32, String> {
    let x = v
        .as_u64()
        .ok_or_else(|| cerr(format!("{name}: expected u32")))?;
    if x > u32::MAX as u64 {
        return Err(cerr(format!("{name}: u32 越域 {x}")));
    }
    Ok(x as u32)
}

fn as_u64(name: &str, v: &Json) -> Result<u64, String> {
    v.as_u64()
        .ok_or_else(|| cerr(format!("{name}: expected u64")))
}

fn as_str<'a>(name: &str, v: &'a Json) -> Result<&'a str, String> {
    let s = v
        .as_str()
        .ok_or_else(|| cerr(format!("{name}: expected str")))?;
    if s.is_empty() {
        return Err(cerr(format!("{name}: empty str")));
    }
    Ok(s)
}

fn as_bool(name: &str, v: &Json) -> Result<bool, String> {
    v.as_bool()
        .ok_or_else(|| cerr(format!("{name}: expected bool")))
}

fn as_f64v(name: &str, v: &Json, n: usize) -> Result<Vec<f64>, String> {
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

fn closed<'a>(name: &str, v: &'a Json, keys: &[&str]) -> Result<&'a Json, String> {
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

struct Contract {
    raw: Json,
    digest: String,
}

fn parse_scene(idx: usize, s: &Json) -> Result<(), String> {
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
    as_str(
        "m133_manifest_digest",
        s.get("m133_manifest_digest").unwrap(),
    )?;
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
    let q = as_f64v(
        "camera.orientation_quat",
        cam.get("orientation_quat").unwrap(),
        4,
    )?;
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
    as_f64(
        "lighting.sun_intensity_lux",
        lig.get("sun_intensity_lux").unwrap(),
    )?;
    as_f64("lighting.sky_intensity", lig.get("sky_intensity").unwrap())?;
    for (i, q) in lig
        .get("quad_lights")
        .unwrap()
        .as_array()
        .ok_or_else(|| cerr("quad_lights 非数组"))?
        .iter()
        .enumerate()
    {
        closed(
            &format!("quad_lights[{i}]"),
            q,
            &["p00", "e1", "e2", "le_linear_rgb"],
        )?;
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
            &[
                "material_name",
                "material_index",
                "le_linear_rgb",
                "area_m2",
            ],
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
        "texture_mean_albedo",
        mp.get("texture_mean_albedo").unwrap(),
    )?;
    as_bool("white_tex_to_white", mp.get("white_tex_to_white").unwrap())?;
    Ok(())
}

fn parse_contract(text: &str) -> Result<Contract, String> {
    let doc = json_parse(text).map_err(|e| cerr(format!("JSON: {e}")))?;
    closed(
        "root",
        &doc,
        &[
            "schema",
            "contract_id",
            "version",
            "tier_sequence",
            "frame_count",
            "seed",
            "calibration_seed",
            "noise_probe_tier",
            "rurix_backends",
            "ue_dlss_quality_map",
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
    let tiers_v = doc.get("tier_sequence").unwrap();
    let tiers_a = tiers_v
        .as_array()
        .ok_or_else(|| cerr("tier_sequence 非数组"))?;
    if tiers_a.is_empty() {
        return Err(cerr("tier_sequence 空"));
    }
    let mut tiers: Vec<u32> = Vec::new();
    for (i, x) in tiers_a.iter().enumerate() {
        tiers.push(as_u32(&format!("tier_sequence[{i}]"), x)?);
    }
    for w in tiers.windows(2) {
        if w[0] >= w[1] {
            return Err(cerr("tier_sequence 非严格递增"));
        }
    }
    if tiers != [50, 67, 100] {
        return Err(cerr(format!("tier_sequence 越闭集: {tiers:?}")));
    }
    as_u32("frame_count", doc.get("frame_count").unwrap())?;
    let seed = as_u64("seed", doc.get("seed").unwrap())?;
    let cal = as_u64("calibration_seed", doc.get("calibration_seed").unwrap())?;
    if cal == seed {
        return Err(cerr("calibration_seed == seed"));
    }
    let probe = as_u32("noise_probe_tier", doc.get("noise_probe_tier").unwrap())?;
    if !tiers.contains(&probe) {
        return Err(cerr("noise_probe_tier 越 tier_sequence"));
    }
    let backends_v = doc.get("rurix_backends").unwrap();
    let backends_a = backends_v
        .as_array()
        .ok_or_else(|| cerr("rurix_backends 非数组"))?;
    if backends_a.is_empty() {
        return Err(cerr("rurix_backends 空"));
    }
    let mut backends: Vec<&str> = Vec::new();
    for (i, x) in backends_a.iter().enumerate() {
        backends.push(as_str(&format!("rurix_backends[{i}]"), x)?);
    }
    let mut bs = backends.clone();
    bs.sort();
    if bs != ["dlss_sr", "fsr_3_1_5", "tsr_device"] {
        return Err(cerr(format!("rurix_backends 越闭集: {backends:?}")));
    }
    let qm = doc.get("ue_dlss_quality_map").unwrap();
    closed("ue_dlss_quality_map", qm, &["50", "67", "100"])?;
    for (k, want) in [("50", "Performance"), ("67", "Quality"), ("100", "DLAA")] {
        let got = as_str(&format!("ue_dlss_quality_map[{k}]"), qm.get(k).unwrap())?;
        if got != want {
            return Err(cerr(format!("ue_dlss_quality_map[{k}] 离冻结映射: {got}")));
        }
    }
    let pol = doc.get("rendering_policy").unwrap();
    closed(
        "rendering_policy",
        pol,
        &["tonemap", "denoiser", "ue_temporal_upscaler", "jitter"],
    )?;
    if as_str("tonemap", pol.get("tonemap").unwrap())? != "off"
        || as_str("denoiser", pol.get("denoiser").unwrap())? != "off"
    {
        return Err(cerr("rendering_policy tonemap/denoiser 须 const off"));
    }
    if as_str(
        "ue_temporal_upscaler",
        pol.get("ue_temporal_upscaler").unwrap(),
    )? != "dlss_plugin"
    {
        return Err(cerr(
            "rendering_policy.ue_temporal_upscaler 须 const dlss_plugin",
        ));
    }
    if as_str("jitter", pol.get("jitter").unwrap())? != "halton_static" {
        return Err(cerr("rendering_policy.jitter 须 const halton_static"));
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
    ids.sort();
    if ids != ["bistro-interior", "cornell-box"] {
        return Err(cerr(format!("场景闭集不全等: {ids:?}")));
    }
    if doc.get("provenance").unwrap().as_object().is_none() {
        return Err(cerr("provenance 须为 obj（不入 digest）"));
    }
    let digest = format!("sha256:{}", sha256_hex(&canonical_preimage(&doc)?));
    Ok(Contract { raw: doc, digest })
}

/// M-d（RXS-0406）Lumen GI 对照契约 fail-closed 解析（digest 三方臂②分派面；
/// 字段闭集/const 谓词/场景闭集与 host python g13_parity_contract.py 逐字同律）。
fn parse_lumen_contract(text: &str) -> Result<String, String> {
    let doc = json_parse(text).map_err(|e| cerr(format!("JSON: {e}")))?;
    closed(
        "root",
        &doc,
        &[
            "schema",
            "contract_id",
            "version",
            "seed",
            "calibration_seed",
            "rendering_policy",
            "rurix_gi_surface",
            "scenes",
            "provenance",
        ],
    )?;
    if as_str("schema", doc.get("schema").unwrap())? != LUMEN_SCHEMA_ID {
        return Err(cerr("schema 字面不符（lumen）"));
    }
    as_str("contract_id", doc.get("contract_id").unwrap())?;
    as_u32("version", doc.get("version").unwrap())?;
    let seed = as_u64("seed", doc.get("seed").unwrap())?;
    let cal = as_u64("calibration_seed", doc.get("calibration_seed").unwrap())?;
    if cal == seed {
        return Err(cerr("calibration_seed == seed"));
    }
    let pol = doc.get("rendering_policy").unwrap();
    closed(
        "rendering_policy",
        pol,
        &[
            "ue_gi_method",
            "ue_reflection_method",
            "tonemap",
            "denoiser",
            "indirect_derivation",
        ],
    )?;
    if as_str("ue_gi_method", pol.get("ue_gi_method").unwrap())? != "lumen"
        || as_str(
            "ue_reflection_method",
            pol.get("ue_reflection_method").unwrap(),
        )? != "lumen"
    {
        return Err(cerr(
            "rendering_policy ue_gi_method/ue_reflection_method 须 const lumen",
        ));
    }
    if as_str("tonemap", pol.get("tonemap").unwrap())? != "off"
        || as_str("denoiser", pol.get("denoiser").unwrap())? != "off"
    {
        return Err(cerr("rendering_policy tonemap/denoiser 须 const off"));
    }
    if as_str(
        "indirect_derivation",
        pol.get("indirect_derivation").unwrap(),
    )? != "gi_on_minus_gi_off"
    {
        return Err(cerr(
            "rendering_policy.indirect_derivation 须 const gi_on_minus_gi_off",
        ));
    }
    let surf_v = doc.get("rurix_gi_surface").unwrap();
    let surf_a = surf_v
        .as_array()
        .ok_or_else(|| cerr("rurix_gi_surface 非数组"))?;
    if surf_a.is_empty() {
        return Err(cerr("rurix_gi_surface 空"));
    }
    let mut surf: Vec<&str> = Vec::new();
    for (i, x) in surf_a.iter().enumerate() {
        surf.push(as_str(&format!("rurix_gi_surface[{i}]"), x)?);
    }
    surf.sort();
    if surf
        != [
            "multibounce_chain",
            "screen_probe_near_field",
            "world_cache_far_field",
        ]
    {
        return Err(cerr(format!("rurix_gi_surface 越三锚闭集: {surf:?}")));
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
    ids.sort();
    if ids != ["bistro-interior", "cornell-box"] {
        return Err(cerr(format!("场景闭集不全等: {ids:?}")));
    }
    if doc.get("provenance").unwrap().as_object().is_none() {
        return Err(cerr("provenance 须为 obj（不入 digest）"));
    }
    Ok(format!("sha256:{}", sha256_hex(&canonical_preimage(&doc)?)))
}

// ---------------------------------------------------------------------------
// canonical preimage（RXS-0405 L2：标签/键序/宽度 RXS-0384 L3 同构；与
// milestones/g13/harness/ue_python/g13_parity_contract.py 逐字同表）
// ---------------------------------------------------------------------------

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

/// 字段类型表（digest 域；键序 = code point 升序通用律，本表钉类型；
/// 与 python 参照 UPSCALE_*_TYPES 逐字同表）。
fn root_type(k: &str) -> &'static str {
    match k {
        "calibration_seed" => "u64",
        "contract_id" => "str",
        "frame_count" => "u32",
        "noise_probe_tier" => "u32",
        "rendering_policy" => "obj_upscale_policy",
        "rurix_backends" => "arr_str",
        "schema" => "str",
        "scenes" => "arr_scene",
        "seed" => "u64",
        "tier_sequence" => "arr_u32",
        "ue_dlss_quality_map" => "obj_strmap",
        "version" => "u32",
        _ => "",
    }
}

fn policy_type(k: &str) -> &'static str {
    match k {
        "denoiser" | "jitter" | "tonemap" | "ue_temporal_upscaler" => "str",
        _ => "",
    }
}

fn lumen_root_type(k: &str) -> &'static str {
    match k {
        "calibration_seed" => "u64",
        "contract_id" => "str",
        "rendering_policy" => "obj_lumen_policy",
        "rurix_gi_surface" => "arr_str",
        "schema" => "str",
        "scenes" => "arr_scene",
        "seed" => "u64",
        "version" => "u32",
        _ => "",
    }
}

fn lumen_policy_type(k: &str) -> &'static str {
    match k {
        "denoiser"
        | "indirect_derivation"
        | "tonemap"
        | "ue_gi_method"
        | "ue_reflection_method" => "str",
        _ => "",
    }
}

fn camera_type(k: &str) -> &'static str {
    match k {
        "far" | "fov_y_deg" | "near" => "f64",
        "orientation_quat" | "position" => "arr_f64",
        "resolution" => "obj_res",
        _ => "",
    }
}

fn res_type(k: &str) -> &'static str {
    match k {
        "h" | "w" => "u32",
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

fn lighting_type(k: &str) -> &'static str {
    match k {
        "emissive_materials" => "arr_emissive",
        "point_lights" => "arr_point",
        "quad_lights" => "arr_quad",
        "sky_intensity" | "sun_intensity_lux" => "f64",
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

fn scene_type(k: &str) -> &'static str {
    match k {
        "camera" => "obj_camera",
        "exposure" => "obj_exposure",
        "gltf_product_digest" | "m133_manifest_digest" | "scene_id" => "str",
        "lighting" => "obj_lighting",
        "material_policy" => "obj_matpol",
        _ => "",
    }
}

fn enc_typed(buf: &mut Vec<u8>, ty: &str, v: &Json) -> Result<(), String> {
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
        "arr_str" => {
            buf.push(0x09);
            for x in v.as_array().unwrap() {
                enc_str(buf, x.as_str().unwrap());
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
        "obj_upscale_policy" => enc_obj(buf, v, policy_type)?,
        "obj_lumen_policy" => enc_obj(buf, v, lumen_policy_type)?,
        "obj_strmap" => enc_obj(buf, v, |_| "str")?,
        "obj_camera" => enc_obj(buf, v, camera_type)?,
        "obj_res" => enc_obj(buf, v, res_type)?,
        "obj_exposure" => enc_obj(buf, v, exposure_type)?,
        "obj_lighting" => enc_obj(buf, v, lighting_type)?,
        "obj_matpol" => enc_obj(buf, v, matpol_type)?,
        _ => return Err(cerr(format!("未知类型标签 {ty}"))),
    }
    Ok(())
}

fn enc_obj(
    buf: &mut Vec<u8>,
    obj: &Json,
    types: impl Fn(&str) -> &'static str,
) -> Result<(), String> {
    buf.push(0x07);
    let pairs = obj.as_object().unwrap();
    let mut entries: Vec<(&String, &Json)> = pairs.iter().map(|(k, v)| (k, v)).collect();
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

fn canonical_preimage(doc: &Json) -> Result<Vec<u8>, String> {
    let schema = doc
        .get("schema")
        .and_then(|s| s.as_str())
        .ok_or_else(|| cerr("schema 字段缺失"))?;
    let lumen = match schema {
        SCHEMA_ID => false,
        LUMEN_SCHEMA_ID => true,
        _ => return Err(cerr(format!("未知 schema 字面: {schema}"))),
    };
    let mut buf = if lumen {
        LUMEN_PREFIX.to_vec()
    } else {
        VERSION_PREFIX.to_vec()
    };
    let pairs = doc.as_object().unwrap();
    // digest 域 = 根字段闭集除 provenance。
    let body = Json::Obj(
        pairs
            .iter()
            .filter(|(k, _)| k.as_str() != "provenance")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    enc_obj(&mut buf, &body, |k| {
        if lumen {
            lumen_root_type(k)
        } else {
            root_type(k)
        }
    })?;
    Ok(buf)
}

// ---------------------------------------------------------------------------
// 最小 glTF 装载（几何 accessors + 节点树世界变换 + 材质表；bin-local——
// rurix-render 不依赖 rurix-asset（循环依赖禁区），语义沿 G12.4
// load_prod_scene 同律：扁平单引用面、逐三角材质扁平化、契约灯面）
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

fn node_local_m4(node: &Json) -> Result<M4, String> {
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

struct Gltf {
    root: Json,
    buffers: Vec<Vec<u8>>,
}

fn load_gltf(path: &Path) -> Result<(Gltf, String), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("glTF 读取失败: {e}"))?;
    let gltf_sha256 = sha256_hex(text.as_bytes());
    let root = json_parse(&text).map_err(|e| format!("glTF JSON: {e}"))?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
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
    Ok((Gltf { root, buffers }, gltf_sha256))
}

impl Gltf {
    fn accessor(&self, idx: usize) -> Result<&Json, String> {
        self.root
            .get("accessors")
            .and_then(|v| v.as_array())
            .and_then(|a| a.get(idx))
            .ok_or_else(|| format!("accessor {idx} 缺"))
    }

    fn accessor_bytes(&self, idx: usize) -> Result<(usize, usize, usize, usize, &[u8]), String> {
        let a = self.accessor(idx)?;
        let count = a
            .get("count")
            .and_then(|v| v.as_u64())
            .ok_or("accessor 缺 count")? as usize;
        let ctype = a
            .get("componentType")
            .and_then(|v| v.as_u64())
            .ok_or("accessor 缺 componentType")? as usize;
        let ty = a
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("accessor 缺 type")?;
        let comps = match ty {
            "SCALAR" => 1usize,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            _ => return Err(format!("accessor type {ty} 不消费")),
        };
        let bv_idx = a
            .get("bufferView")
            .and_then(|v| v.as_u64())
            .ok_or("accessor 缺 bufferView（稀疏不消费）")? as usize;
        let bv = self
            .root
            .get("bufferViews")
            .and_then(|v| v.as_array())
            .and_then(|a| a.get(bv_idx))
            .ok_or("bufferView 缺")?;
        let buf_idx = bv
            .get("buffer")
            .and_then(|v| v.as_u64())
            .ok_or("bufferView 缺 buffer")? as usize;
        let stride = bv
            .get("byteStride")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let comp_size = match ctype {
            5120 | 5121 => 1usize,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            _ => return Err(format!("componentType {ctype} 不消费")),
        };
        let elem_size = comp_size * comps;
        let stride = stride.unwrap_or(elem_size);
        let off = bv.get("byteOffset").and_then(|v| v.as_u64()).unwrap_or(0) as usize
            + a.get("byteOffset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let data = self
            .buffers
            .get(buf_idx)
            .ok_or_else(|| format!("buffer {buf_idx} 缺"))?;
        if off + (count.saturating_sub(1)) * stride + elem_size > data.len() && count > 0 {
            return Err("accessor 越 buffer 界".into());
        }
        Ok((count, ctype, comps, stride, &data[off..]))
    }

    fn positions(&self, idx: usize) -> Result<Vec<[f32; 3]>, String> {
        let (count, ctype, comps, stride, data) = self.accessor_bytes(idx)?;
        if ctype != 5126 || comps != 3 {
            return Err("POSITION 须 float VEC3".into());
        }
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let o = i * stride;
            let f = |k: usize| {
                f32::from_le_bytes([
                    data[o + k * 4],
                    data[o + k * 4 + 1],
                    data[o + k * 4 + 2],
                    data[o + k * 4 + 3],
                ])
            };
            out.push([f(0), f(1), f(2)]);
        }
        Ok(out)
    }

    fn indices(&self, idx: usize) -> Result<Vec<u32>, String> {
        let (count, ctype, comps, stride, data) = self.accessor_bytes(idx)?;
        if comps != 1 {
            return Err("indices 须 SCALAR".into());
        }
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let o = i * stride;
            let v = match ctype {
                5121 => data[o] as u32,
                5123 => u16::from_le_bytes([data[o], data[o + 1]]) as u32,
                5125 => u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]),
                _ => return Err(format!("indices componentType {ctype} 不消费")),
            };
            out.push(v);
        }
        Ok(out)
    }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// DDS BC1（DXT1）/BC3（DXT5）mip0 真实解码 → 逐 texel sRGB→线性均值
/// （texture_mean_albedo 策略消费面；BC5/ATI2 等其它格式 fail-closed——
/// 法线图非 baseColor 消费槽，策略面只钉 baseColor 均值）。
fn dds_mean_linear_rgb(bytes: &[u8]) -> Result<[f32; 3], String> {
    if bytes.len() < 128 || &bytes[0..4] != b"DDS " {
        return Err("DDS magic 不符".into());
    }
    let h = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
    let w = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
    let fourcc = &bytes[84..88];
    let block_bytes = match fourcc {
        b"DXT1" => 8,
        b"DXT5" => 16,
        other => {
            return Err(format!(
                "DDS fourCC {} 非 BC1/BC3（fail-closed 不静默）",
                String::from_utf8_lossy(other)
            ));
        }
    };
    if w == 0 || h == 0 {
        return Err("DDS 零尺寸".into());
    }
    let bw = w.div_ceil(4);
    let bh = h.div_ceil(4);
    if 128 + bw * bh * block_bytes > bytes.len() {
        return Err("DDS mip0 数据截断".into());
    }
    let mut acc = [0.0f64; 3];
    let mut npx = 0usize;
    for by in 0..bh {
        for bx in 0..bw {
            let bo = 128 + (by * bw + bx) * block_bytes;
            let cb = &bytes[bo + block_bytes - 8..bo + block_bytes]; // BC1/BC3 颜色块恒末 8B
            let c0 = u16::from_le_bytes([cb[0], cb[1]]);
            let c1 = u16::from_le_bytes([cb[2], cb[3]]);
            let lut = u32::from_le_bytes([cb[4], cb[5], cb[6], cb[7]]);
            let unpack = |c: u16| -> [u8; 3] {
                let r = ((c >> 11) & 31) as u8;
                let g = ((c >> 5) & 63) as u8;
                let b = (c & 31) as u8;
                [
                    (r << 3) | (r >> 2),
                    (g << 2) | (g >> 4),
                    (b << 3) | (b >> 2),
                ]
            };
            let p0 = unpack(c0);
            let p1 = unpack(c1);
            let mut pal = [[0u8; 3]; 4];
            pal[0] = p0;
            pal[1] = p1;
            if c0 > c1 {
                for k in 0..3 {
                    pal[2][k] = ((2 * p0[k] as u32 + p1[k] as u32) / 3) as u8;
                    pal[3][k] = ((p0[k] as u32 + 2 * p1[k] as u32) / 3) as u8;
                }
            } else {
                for k in 0..3 {
                    pal[2][k] = ((p0[k] as u32 + p1[k] as u32) / 2) as u8;
                    pal[3][k] = 0; // DXT1 1-bit alpha 透明槽 → RGB=0（确定性登记口径）
                }
            }
            for py in 0..4usize {
                for px in 0..4usize {
                    let (x, y) = (bx * 4 + px, by * 4 + py);
                    if x >= w || y >= h {
                        continue;
                    }
                    let idx = ((lut >> (2 * (py * 4 + px))) & 3) as usize;
                    let c = pal[idx];
                    for (k, ac) in acc.iter_mut().enumerate() {
                        *ac += srgb_to_linear(c[k] as f32 / 255.0) as f64;
                    }
                    npx += 1;
                }
            }
        }
    }
    if npx == 0 {
        return Err("DDS 零 texel".into());
    }
    Ok([
        (acc[0] / npx as f64) as f32,
        (acc[1] / npx as f64) as f32,
        (acc[2] / npx as f64) as f32,
    ])
}

// ---------------------------------------------------------------------------
// 场景装配（契约逐字段：几何汤 + 逐三角 albedo/emission + 灯面 + 相机）
// ---------------------------------------------------------------------------

struct QuadLight {
    p00: [f32; 3],
    e1: [f32; 3],
    e2: [f32; 3],
    le: [f32; 3],
}

struct PointLight {
    pos: [f32; 3],
    intensity: [f32; 3], // color × intensity_cd（G12.4 同口径：点强 I 即 cd 直给）
}

struct CameraSpec {
    eye: [f32; 3],
    forward: [f32; 3],
    up0: [f32; 3],
    fov_y_rad: f32,
    near: f32,
    far: f32,
}

struct SceneData {
    positions: Vec<[f32; 3]>,
    indices: Vec<[u32; 3]>,
    albedo: Vec<[f32; 3]>,
    emission: Vec<[f32; 3]>,
    quads: Vec<QuadLight>,
    points: Vec<PointLight>,
    camera: CameraSpec,
    ev100: f32,
    texture_mean_albedo: bool,
    tri_count: usize,
    emissive_tri_count: usize,
    gltf_sha256: String,
}

fn f64v3(v: &[f64]) -> [f32; 3] {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

/// 契约四元数（w,x,y,z 序）旋转向量（G12.4 同式逐字）。
fn quat_rot(quat: &[f64], v: [f64; 3]) -> [f64; 3] {
    let (w, x, y, z) = (quat[0], quat[1], quat[2], quat[3]);
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

fn contract_scene_row<'a>(contract: &'a Json, scene_id: &str) -> Result<&'a Json, String> {
    contract
        .get("scenes")
        .and_then(|v| v.as_array())
        .and_then(|a| {
            a.iter().find(|s| {
                s.get("scene_id")
                    .and_then(|v| v.as_str())
                    .map(|x| x == scene_id)
                    .unwrap_or(false)
            })
        })
        .ok_or_else(|| cerr(format!("契约缺场景行 {scene_id}")))
}

fn assemble_scene(contract: &Json, scene_id: &str, gltf_path: &Path) -> Result<SceneData, String> {
    let srow = contract_scene_row(contract, scene_id)?;
    let cam = srow.get("camera").unwrap();
    let lig = srow.get("lighting").unwrap();
    let pol = srow.get("material_policy").unwrap();
    let texture_mean = pol
        .get("texture_mean_albedo")
        .and_then(|v| v.as_bool())
        .unwrap();

    let (gltf, gltf_sha256) = load_gltf(gltf_path)?;
    let base = gltf_path.parent().unwrap_or_else(|| Path::new("."));

    // 材质表（baseColorFactor/metallic/baseColorTexture 源图索引）。
    struct MatRec {
        factor: [f32; 3],
        metallic: f32,
        base_img: Option<usize>,
    }
    let mut mats: Vec<MatRec> = Vec::new();
    for m in gltf
        .root
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
            .and_then(|ti| gltf.root.get("textures")?.as_array()?.get(ti as usize))
            .and_then(|tex| tex.get("source"))
            .and_then(|v| v.as_u64())
            .map(|x| x as usize);
        mats.push(MatRec {
            factor: alb,
            metallic: pbr
                .and_then(|p| p.get("metallicFactor"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32,
            base_img: img,
        });
    }

    // 纹理均值（texture_mean_albedo 策略：baseColor 引用图 DDS BC1/BC3 真实解码
    // → 逐 texel sRGB→线性均值；其它格式 fail-closed 显式拒绝不静默）。
    let mut tex_mean: Vec<Option<[f32; 3]>> = Vec::new();
    if let Some(imgs) = gltf.root.get("images").and_then(|v| v.as_array()) {
        let consumed: std::collections::BTreeSet<usize> =
            mats.iter().filter_map(|m| m.base_img).collect();
        for (ii, im) in imgs.iter().enumerate() {
            let mut mean = None;
            if texture_mean && consumed.contains(&ii) {
                let uri = im
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| cerr("image 缺 uri（内嵌不消费）"))?;
                let raw = std::fs::read(base.join(uri))
                    .map_err(|e| format!("纹理 {uri} 读取失败: {e}"))?;
                mean = Some(
                    dds_mean_linear_rgb(&raw)
                        .map_err(|e| format!("纹理 {uri} DDS 解码失败: {e}"))?,
                );
            }
            tex_mean.push(mean);
        }
    }

    // 节点树世界变换（扁平单引用面；嵌套按 compose 递推）。
    let nodes = gltf
        .root
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| cerr("glTF 缺 nodes"))?;
    let mut world: Vec<Option<M4>> = vec![None; nodes.len()];
    fn compose(idx: usize, nodes: &[Json], world: &mut [Option<M4>]) -> Result<M4, String> {
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

    // 契约 emissive 材质集（material_index → Le）。
    let mut emissive_map: std::collections::HashMap<u32, [f32; 3]> =
        std::collections::HashMap::new();
    for m in lig
        .get("emissive_materials")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let mi = m.get("material_index").and_then(|v| v.as_u64()).unwrap() as u32;
        let le = as_f64v("le", m.get("le_linear_rgb").unwrap(), 3)?;
        emissive_map.insert(mi, f64v3(&le));
    }

    // 三角形汤装配（世界变换烘焙进顶点）。
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();
    let mut albedo: Vec<[f32; 3]> = Vec::new();
    let mut emission: Vec<[f32; 3]> = Vec::new();
    let mut emissive_tris = 0usize;
    let meshes = gltf
        .root
        .get("meshes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| cerr("glTF 缺 meshes"))?;
    for (ni, n) in nodes.iter().enumerate() {
        let Some(mesh_idx) = n.get("mesh").and_then(|v| v.as_u64()) else {
            continue;
        };
        let mesh = meshes
            .get(mesh_idx as usize)
            .ok_or_else(|| cerr("node.mesh 越界"))?;
        let w = world[ni].ok_or_else(|| cerr("节点世界变换缺失"))?;
        for prim in mesh
            .get("primitives")
            .and_then(|v| v.as_array())
            .ok_or_else(|| cerr("meshes[].primitives 缺"))?
        {
            let mode = prim.get("mode").and_then(|v| v.as_u64()).unwrap_or(4);
            if mode != 4 {
                return Err(cerr("非三角形 primitive（mode≠4）fail-closed"));
            }
            let mat_idx = prim
                .get("material")
                .and_then(|v| v.as_u64())
                .map(|x| x as u32);
            let alb = match mat_idx.and_then(|mi| mats.get(mi as usize)) {
                Some(rec) => {
                    let k = 1.0 - rec.metallic;
                    let b = rec
                        .base_img
                        .and_then(|ii| tex_mean.get(ii))
                        .and_then(|m| *m)
                        .map(|tm| {
                            [
                                tm[0] * rec.factor[0],
                                tm[1] * rec.factor[1],
                                tm[2] * rec.factor[2],
                            ]
                        })
                        .unwrap_or(rec.factor);
                    [b[0] * k, b[1] * k, b[2] * k]
                }
                None => [1.0, 1.0, 1.0],
            };
            let emi = mat_idx
                .and_then(|mi| emissive_map.get(&mi).copied())
                .unwrap_or([0.0, 0.0, 0.0]);
            let pos_acc = prim
                .get("attributes")
                .and_then(|a| a.get("POSITION"))
                .and_then(|v| v.as_u64())
                .ok_or_else(|| cerr("primitive 缺 POSITION"))?;
            let pos = gltf.positions(pos_acc as usize)?;
            let idx_acc = prim
                .get("indices")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| cerr("primitive 缺 indices"))?;
            let idx = gltf.indices(idx_acc as usize)?;
            if idx.len() % 3 != 0 {
                return Err(cerr("indices 非 3 整除"));
            }
            for t3 in idx.chunks_exact(3) {
                let v0 = xform(&w, pos[t3[0] as usize]);
                let v1 = xform(&w, pos[t3[1] as usize]);
                let v2 = xform(&w, pos[t3[2] as usize]);
                let bidx = positions.len() as u32;
                positions.push(v0);
                positions.push(v1);
                positions.push(v2);
                indices.push([bidx, bidx + 1, bidx + 2]);
                albedo.push(alb);
                emission.push(emi);
                if emi != [0.0, 0.0, 0.0] {
                    emissive_tris += 1;
                }
            }
        }
    }
    if indices.is_empty() {
        return Err(format!("场景 {scene_id} 装配零三角"));
    }

    // 契约 quad 面光（照明面 + 发光三角几何逐字一致追加，G12.4 同律——
    // 主命中可见灯面；阴影射线以 t_max 缩短排除目标灯面自遮蔽）。
    let mut quads: Vec<QuadLight> = Vec::new();
    for q in lig
        .get("quad_lights")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let f3 =
            |k: &str| -> Result<[f32; 3], String> { Ok(f64v3(&as_f64v(k, q.get(k).unwrap(), 3)?)) };
        let (p00, e1, e2, le) = (f3("p00")?, f3("e1")?, f3("e2")?, f3("le_linear_rgb")?);
        quads.push(QuadLight { p00, e1, e2, le });
        let p10 = [p00[0] + e1[0], p00[1] + e1[1], p00[2] + e1[2]];
        let p01 = [p00[0] + e2[0], p00[1] + e2[1], p00[2] + e2[2]];
        let p11 = [
            p00[0] + e1[0] + e2[0],
            p00[1] + e1[1] + e2[1],
            p00[2] + e1[2] + e2[2],
        ];
        for (a, b, c) in [(p00, p10, p11), (p00, p11, p01)] {
            let bidx = positions.len() as u32;
            positions.push(a);
            positions.push(b);
            positions.push(c);
            indices.push([bidx, bidx + 1, bidx + 2]);
            albedo.push([0.5, 0.5, 0.5]);
            emission.push(le);
        }
    }
    // 契约点光（delta；I = color × intensity_cd）。
    let mut points: Vec<PointLight> = Vec::new();
    for p in lig
        .get("point_lights")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let pos = as_f64v("position", p.get("position").unwrap(), 3)?;
        let col = as_f64v("color", p.get("color_linear_rgb").unwrap(), 3)?;
        let inten = p.get("intensity_cd").and_then(|v| v.as_f64()).unwrap();
        points.push(PointLight {
            pos: f64v3(&pos),
            intensity: [
                (col[0] * inten) as f32,
                (col[1] * inten) as f32,
                (col[2] * inten) as f32,
            ],
        });
    }

    // 相机（契约四元数 → look_at 同口径：forward = q·(0,0,−1)、up = q·(0,1,0)；
    // right = forward × up0（UE 一致手性，G12.4 波裁决实证面））。
    let pos = as_f64v("camera.position", cam.get("position").unwrap(), 3)?;
    let quat = as_f64v(
        "camera.orientation_quat",
        cam.get("orientation_quat").unwrap(),
        4,
    )?;
    let fov = cam.get("fov_y_deg").and_then(|v| v.as_f64()).unwrap();
    let near = cam.get("near").and_then(|v| v.as_f64()).unwrap();
    let far = cam.get("far").and_then(|v| v.as_f64()).unwrap();
    let fwd = quat_rot(&quat, [0.0, 0.0, -1.0]);
    let up0 = quat_rot(&quat, [0.0, 1.0, 0.0]);
    let ev100 = srow
        .get("exposure")
        .and_then(|e| e.get("ev100"))
        .and_then(|v| v.as_f64())
        .unwrap();

    Ok(SceneData {
        tri_count: indices.len(),
        positions,
        indices,
        albedo,
        emission,
        quads,
        points,
        camera: CameraSpec {
            eye: f64v3(&pos),
            forward: f64v3(&fwd),
            up0: f64v3(&up0),
            fov_y_rad: (fov as f32).to_radians(),
            near: near as f32,
            far: far as f32,
        },
        ev100: ev100 as f32,
        texture_mean_albedo: texture_mean,
        emissive_tri_count: emissive_tris,
        gltf_sha256,
    })
}

// ---------------------------------------------------------------------------
// 帧渲染（主射线 TriBvh 真求交 + 双面 Lambert 直接光 + ZO NDC 深度；
// 行带 scoped 线程——逐像素纯函数，输出与线程数无关位级一致）
// ---------------------------------------------------------------------------

const INV_PI: f32 = 1.0 / std::f32::consts::PI;

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm3(a: [f32; 3]) -> [f32; 3] {
    let l = dot3(a, a).sqrt();
    if l > 0.0 {
        [a[0] / l, a[1] / l, a[2] / l]
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// 未抖 view-proj（相机基向量 = G12.4 双端一致口径）。
fn build_vp(cam: &CameraSpec, w: u32, h: u32) -> Mat4 {
    let center = [
        cam.eye[0] + cam.forward[0],
        cam.eye[1] + cam.forward[1],
        cam.eye[2] + cam.forward[2],
    ];
    let view = look_at_rh(cam.eye, center, cam.up0);
    let proj = perspective_rh_zo(cam.fov_y_rad, w as f32 / h as f32, cam.near, cam.far);
    proj.mul(&view)
}

/// jitter 等价投影：内容 ≡ 采样于 (x+0.5+jx, y+0.5+jy) 的未抖渲染（M-a/M-b
/// render_input 同一采样口径）；S: ndc.x' = ndc.x − 2jx/w、ndc.y' = ndc.y + 2jy/h。
/// 仅改 row0/row1（clip.z/w 不变 → 深度值两口径位级同值）。
fn jittered_vp(vp: &Mat4, j: [f32; 2], w: u32, h: u32) -> Mat4 {
    let mut m = vp.m;
    let sx = 2.0 * j[0] / w as f32;
    let sy = 2.0 * j[1] / h as f32;
    for c in 0..4 {
        m[0][c] -= sx * m[3][c];
        m[1][c] += sy * m[3][c];
    }
    Mat4 { m }
}

/// 像素采样点（含 jitter）反投影（compute_camera_mv 同一 NDC 约定面）。
fn unproject(inv_vp: &Mat4, sx: f32, sy: f32, w: u32, h: u32, z: f32) -> [f32; 3] {
    let u = sx / w as f32;
    let v = sy / h as f32;
    let w4 = inv_vp.transform_vec4([2.0 * u - 1.0, 1.0 - 2.0 * v, z, 1.0]);
    [w4[0] / w4[3], w4[1] / w4[3], w4[2] / w4[3]]
}

/// 阴影可见性：origin 沿 wl 偏移 eps，目标距离缩短 2eps（目标灯面三角在
/// t_max 外——排除灯面自遮蔽；遮挡体在 (0, d−2eps) 开区间照常命中）。
fn visible(bvh: &TriBvh, origin: [f32; 3], dir: [f32; 3], dist: f32, eps: f32) -> bool {
    let t_max = dist - 2.0 * eps;
    if t_max <= 0.0 {
        return true;
    }
    !bvh.any_hit(
        &Ray {
            origin: Vec3::from_array(origin),
            dir: Vec3::from_array(dir),
        },
        t_max,
    )
}

fn shade_pixel(
    scene: &SceneData,
    bvh: &TriBvh,
    p: [f32; 3],
    n_in: [f32; 3],
    tri: u32,
    eps: f32,
) -> [f32; 3] {
    let albedo = scene.albedo[tri as usize];
    let mut lo = scene.emission[tri as usize];
    let n = n_in;
    let mut direct = [0.0f32; 3];
    for q in &scene.quads {
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
        let sample_area = area / (QUAD_SAMPLES * QUAD_SAMPLES) as f32;
        let mut acc = [0.0f32; 3];
        for sy in 0..QUAD_SAMPLES {
            for sx in 0..QUAD_SAMPLES {
                let u = (sx as f32 + 0.5) / QUAD_SAMPLES as f32;
                let v = (sy as f32 + 0.5) / QUAD_SAMPLES as f32;
                let lp = [
                    q.p00[0] + u * q.e1[0] + v * q.e2[0],
                    q.p00[1] + u * q.e1[1] + v * q.e2[1],
                    q.p00[2] + u * q.e1[2] + v * q.e2[2],
                ];
                let l = sub3(lp, p);
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
                let origin = [p[0] + wl[0] * eps, p[1] + wl[1] * eps, p[2] + wl[2] * eps];
                if !visible(bvh, origin, wl, d, eps) {
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
    for lgt in &scene.points {
        let l = sub3(lgt.pos, p);
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
        let origin = [p[0] + wl[0] * eps, p[1] + wl[1] * eps, p[2] + wl[2] * eps];
        if !visible(bvh, origin, wl, d, eps) {
            continue;
        }
        let g = cos_s / d2;
        for (k, a) in direct.iter_mut().enumerate() {
            *a += lgt.intensity[k] * g;
        }
    }
    for (k, o) in lo.iter_mut().enumerate() {
        *o += albedo[k] * INV_PI * direct[k];
    }
    lo
}

/// 渲染一帧：color（3ch scene-linear HDR）+ depth（1ch ZO NDC）。
fn render_frame(
    scene: &SceneData,
    bvh: &TriBvh,
    iw: u32,
    ih: u32,
    jitter: [f32; 2],
    eps: f32,
) -> (ImageF32, ImageF32) {
    let vp = build_vp(&scene.camera, iw, ih);
    let inv_vp = vp.inverse().expect("view-proj 必须可逆");
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
            let (scene_r, bvh_r, vp_r, inv_vp_r) = (scene, bvh, &vp, &inv_vp);
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
                        let sx = x as f32 + 0.5 + jitter[0];
                        let sy = y as f32 + 0.5 + jitter[1];
                        let near = unproject(inv_vp_r, sx, sy, iw, ih, 0.0);
                        let far = unproject(inv_vp_r, sx, sy, iw, ih, 1.0);
                        let dir = norm3(sub3(far, near));
                        let ray = Ray {
                            origin: Vec3::from_array(near),
                            dir: Vec3::from_array(dir),
                        };
                        let (rgb, z) = match bvh_r.intersect(&ray) {
                            Some(hit) => {
                                let p = [
                                    near[0] + dir[0] * hit.t,
                                    near[1] + dir[1] * hit.t,
                                    near[2] + dir[2] * hit.t,
                                ];
                                let clip = vp_r.transform_vec4([p[0], p[1], p[2], 1.0]);
                                let z = if clip[3].abs() > 1e-8 {
                                    clip[2] / clip[3]
                                } else {
                                    1.0
                                };
                                // 双面：法线翻转朝向相机（G12.4 two-sided 同律）。
                                let mut n = hit.normal;
                                if dot3(n, dir) > 0.0 {
                                    n = [-n[0], -n[1], -n[2]];
                                }
                                (shade_pixel(scene_r, bvh_r, p, n, hit.tri, eps), z)
                            }
                            None => ([0.0, 0.0, 0.0], 1.0),
                        };
                        c_row[x as usize * 3] = rgb[0];
                        c_row[x as usize * 3 + 1] = rgb[1];
                        c_row[x as usize * 3 + 2] = rgb[2];
                        d_row[x as usize] = z;
                    }
                }
            }));
        }
        for hnd in handles {
            hnd.join().expect("渲染线程 panic");
        }
    });
    (
        ImageF32 {
            w: iw,
            h: ih,
            c: 3,
            data: color,
        },
        ImageF32 {
            w: iw,
            h: ih,
            c: 1,
            data: depth,
        },
    )
}

/// 场景包围盒派生阴影 eps（尺度自适应；双面直接光自交规避）。
fn scene_eps(positions: &[[f32; 3]]) -> f32 {
    let mut mn = [f32::INFINITY; 3];
    let mut mx = [f32::NEG_INFINITY; 3];
    for p in positions {
        for k in 0..3 {
            mn[k] = mn[k].min(p[k]);
            mx[k] = mx[k].max(p[k]);
        }
    }
    let extent = (mx[0] - mn[0]).max(mx[1] - mn[1]).max(mx[2] - mn[2]);
    (extent * 1e-4).clamp(1e-3, 0.5)
}

// ---------------------------------------------------------------------------
// UpscaleBackend 接入面（冻结 trait 0-byte；三后端 bin-local adapter——
// tsr_device = M-b TsrDeviceBackend 同模式；dlss_sr/fsr_3_1_5 = M-a adapter 同模式）
// ---------------------------------------------------------------------------

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn load_spv(path: &str) -> Vec<u32> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("读 {path}: {e}")));
    if bytes.len() % 4 != 0 {
        fail("SPIR-V 字节数非 4 对齐");
    }
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// TSR kernel 参数面打包（与 .rx 双腿参数面逐字同源；M-b pack_tsr_params 同式，
/// RED 注入槽恒 0——本 harness 不设 RED 注入面）。
#[allow(clippy::too_many_arguments)]
fn pack_tsr_params(
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
    jitter: [f32; 2],
    exposure: f32,
    has_history: bool,
    has_reactive: bool,
) -> Vec<f32> {
    let p = TsrParams::default();
    let mut v = vec![
        (ow * oh) as f32,
        ow as f32,
        oh as f32,
        iw as f32,
        ih as f32,
        jitter[0],
        jitter[1],
        exposure,
        if has_history { 1.0 } else { 0.0 },
        p.base_alpha,
        p.min_alpha,
        2.0 / (p.flicker_window_frames as f32 + 1.0),
        p.flicker_tighten,
        p.flicker_deadzone_abs,
        p.flicker_deadzone_rel,
        p.depth_rel_tol,
        if has_reactive { 1.0 } else { 0.0 },
        0.0, // red_bias（M-b RED 槽；本面恒 0）
        0.0, // red_passthrough（同上）
    ];
    v.resize(32, 0.0);
    v
}

/// 自研 TSR device 后端（M-b bin-local adapter 同模式逐字：逐帧 vk::run_compute
/// 双 dispatch 驱动 .rx 双腿，历史状态 host 侧双缓冲轮换）。
struct TsrDeviceBackend {
    spv_resample: Vec<u32>,
    spv_resolve: Vec<u32>,
    entry_resample: String,
    entry_resolve: String,
    output_size: Option<(u32, u32)>,
    hist_color: Vec<f32>,
    hist_depth: Vec<f32>,
    prev_luma: Vec<f32>,
    prev_sign: Vec<f32>,
    prev_score: Vec<f32>,
}

impl TsrDeviceBackend {
    fn create(spv_resample: Vec<u32>, spv_resolve: Vec<u32>) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let entry_resample =
            vk::entry_point_name(&spv_resample).ok_or("resample SPV 无 OpEntryPoint")?;
        let entry_resolve =
            vk::entry_point_name(&spv_resolve).ok_or("resolve SPV 无 OpEntryPoint")?;
        Ok(Self {
            spv_resample,
            spv_resolve,
            entry_resample,
            entry_resolve,
            output_size: None,
            hist_color: Vec::new(),
            hist_depth: Vec::new(),
            prev_luma: Vec::new(),
            prev_sign: Vec::new(),
            prev_score: Vec::new(),
        })
    }

    fn clear_state(&mut self) {
        self.output_size = None;
        self.hist_color = Vec::new();
        self.hist_depth = Vec::new();
        self.prev_luma = Vec::new();
        self.prev_sign = Vec::new();
        self.prev_score = Vec::new();
    }
}

impl UpscaleBackend for TsrDeviceBackend {
    fn name(&self) -> &str {
        "tsr_device"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        if self.output_size != Some((ow, oh)) {
            self.clear_state();
            self.output_size = Some((ow, oh));
        }
        let pc = (ow * oh) as usize;
        let has_history = !inputs.reset && !self.hist_color.is_empty();
        let params = pack_tsr_params(
            iw,
            ih,
            ow,
            oh,
            inputs.jitter,
            inputs.exposure,
            has_history,
            inputs.reactive.is_some(),
        );
        let zero_in = vec![0.0f32; (iw * ih) as usize];
        let reactive = inputs.reactive.map(|r| &r.data[..]).unwrap_or(&zero_in);
        // ── 腿 ①：重采样（当前帧 → 输出网格显示域 + 亮度 + 深度最近邻）──
        let mut bufs = vec![
            bytes_f32(&inputs.color.data),
            bytes_f32(&inputs.depth.data),
            bytes_f32(&params),
            vec![0u8; pc * 12],
            vec![0u8; pc * 4],
            vec![0u8; pc * 4],
        ];
        vk::run_compute(
            &self.spv_resample,
            &self.entry_resample,
            &mut bufs,
            &[],
            [ow * oh, 1, 1],
        )
        .unwrap_or_else(|e| panic!("TSR resample dispatch 失败: {e}"));
        let cur_rgb = read_f32(&bufs[3]);
        let cur_luma = read_f32(&bufs[4]);
        let depth_hi = read_f32(&bufs[5]);
        // ── 腿 ②：resolve（闪烁 EMA + 重投影 + 验证 + AABB + 混合）──
        let hist_color = if has_history {
            self.hist_color.clone()
        } else {
            cur_rgb.clone()
        };
        let hist_depth = if has_history {
            self.hist_depth.clone()
        } else {
            depth_hi.clone()
        };
        let prev_luma = if has_history {
            self.prev_luma.clone()
        } else {
            cur_luma.clone()
        };
        let prev_sign = if has_history {
            self.prev_sign.clone()
        } else {
            vec![0.0f32; pc]
        };
        let prev_score = if has_history {
            self.prev_score.clone()
        } else {
            vec![0.0f32; pc]
        };
        let mut bufs2 = vec![
            bytes_f32(&cur_rgb),
            bytes_f32(&cur_luma),
            bytes_f32(&depth_hi),
            bytes_f32(&inputs.mv.data),
            bytes_f32(reactive),
            bytes_f32(&hist_color),
            bytes_f32(&hist_depth),
            bytes_f32(&prev_luma),
            bytes_f32(&prev_sign),
            bytes_f32(&prev_score),
            bytes_f32(&params),
            vec![0u8; pc * 12],
            vec![0u8; pc * 4],
            vec![0u8; pc * 4],
        ];
        vk::run_compute(
            &self.spv_resolve,
            &self.entry_resolve,
            &mut bufs2,
            &[],
            [ow * oh, 1, 1],
        )
        .unwrap_or_else(|e| panic!("TSR resolve dispatch 失败: {e}"));
        let out_color = read_f32(&bufs2[11]);
        let out_sign = read_f32(&bufs2[12]);
        let out_score = read_f32(&bufs2[13]);
        // 双缓冲：本帧输出即下帧历史。
        self.hist_color = out_color.clone();
        self.hist_depth = depth_hi;
        self.prev_luma = cur_luma;
        self.prev_sign = out_sign;
        self.prev_score = out_score;
        ImageF32 {
            w: ow,
            h: oh,
            c: 3,
            data: out_color,
        }
    }

    fn reset_history(&mut self) {
        self.clear_state();
    }
}

/// DLSS SR（Streamline Vulkan interop）→ UpscaleBackend 适配（M-a 同模式，
/// 尺寸参数化——session 创建时钉 (in,out)，逐帧断言一致）。
struct DlssBackend {
    session: DlssVkSession,
    in_size: (u32, u32),
    out_size: (u32, u32),
    pending_reset: bool,
}

impl DlssBackend {
    fn create(in_size: (u32, u32), out_size: (u32, u32)) -> Result<Self, String> {
        let dir = streamline_sdk_dir().map_err(|e| e.to_string())?;
        let session =
            DlssVkSession::create(&dir, in_size, out_size, false).map_err(|e| e.to_string())?;
        Ok(Self {
            session,
            in_size,
            out_size,
            pending_reset: true,
        })
    }
}

impl UpscaleBackend for DlssBackend {
    fn name(&self) -> &str {
        "dlss_sr"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        assert_eq!(
            (iw, ih),
            self.in_size,
            "DLSS adapter 输入分辨率与 session 不符"
        );
        assert_eq!(
            (ow, oh),
            self.out_size,
            "DLSS adapter 输出分辨率与 session 不符"
        );
        let vi = VendorFrameInput {
            color: &inputs.color.data,
            depth: &inputs.depth.data,
            mv: &inputs.mv.data,
            reactive: inputs.reactive.map(|r| &r.data[..]),
            exposure: inputs.exposure,
            jitter: inputs.jitter,
            frame_index: inputs.frame_index,
            reset: inputs.reset || self.pending_reset,
        };
        self.pending_reset = false;
        let data = self
            .session
            .upscale(&vi)
            .unwrap_or_else(|e| panic!("DLSS upscale 失败: {e}"));
        ImageF32 {
            w: ow,
            h: oh,
            c: 3,
            data,
        }
    }

    fn reset_history(&mut self) {
        self.pending_reset = true;
    }
}

/// FSR 3.1.5（FFX SDK DX12）→ UpscaleBackend 适配（M-a 同模式，尺寸参数化）。
struct FsrBackend {
    session: FsrDx12Session,
    in_size: (u32, u32),
    out_size: (u32, u32),
    pending_reset: bool,
}

impl FsrBackend {
    fn create(in_size: (u32, u32), out_size: (u32, u32)) -> Result<Self, String> {
        let dir = fsr_sdk_dir().map_err(|e| e.to_string())?;
        let validation = std::env::var("RURIX_VK_VALIDATION").ok().as_deref() == Some("1");
        let session = FsrDx12Session::create(&dir, in_size, out_size, validation)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            session,
            in_size,
            out_size,
            pending_reset: true,
        })
    }
}

impl UpscaleBackend for FsrBackend {
    fn name(&self) -> &str {
        "fsr_3_1_5"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        assert_eq!(
            (iw, ih),
            self.in_size,
            "FSR adapter 输入分辨率与 session 不符"
        );
        assert_eq!(
            (ow, oh),
            self.out_size,
            "FSR adapter 输出分辨率与 session 不符"
        );
        let vi = VendorFrameInput {
            color: &inputs.color.data,
            depth: &inputs.depth.data,
            mv: &inputs.mv.data,
            reactive: inputs.reactive.map(|r| &r.data[..]),
            exposure: inputs.exposure,
            jitter: inputs.jitter,
            frame_index: inputs.frame_index,
            reset: inputs.reset || self.pending_reset,
        };
        self.pending_reset = false;
        let data = self
            .session
            .upscale(&vi)
            .unwrap_or_else(|e| panic!("FSR upscale 失败: {e}"));
        ImageF32 {
            w: ow,
            h: oh,
            c: 3,
            data,
        }
    }

    fn reset_history(&mut self) {
        self.pending_reset = true;
    }
}

enum Backend {
    Tsr(TsrDeviceBackend),
    Dlss(DlssBackend),
    Fsr(FsrBackend),
}

impl UpscaleBackend for Backend {
    fn name(&self) -> &str {
        match self {
            Backend::Tsr(b) => b.name(),
            Backend::Dlss(b) => b.name(),
            Backend::Fsr(b) => b.name(),
        }
    }
    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        match self {
            Backend::Tsr(b) => b.upscale(inputs),
            Backend::Dlss(b) => b.upscale(inputs),
            Backend::Fsr(b) => b.upscale(inputs),
        }
    }
    fn reset_history(&mut self) {
        match self {
            Backend::Tsr(b) => b.reset_history(),
            Backend::Dlss(b) => b.reset_history(),
            Backend::Fsr(b) => b.reset_history(),
        }
    }
}

// ---------------------------------------------------------------------------
// EXR 落盘（RXS-0385 rurix strict 元数据闭集；G10.5/G12.4 同 image-io 写出面）
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

fn write_exr(path: &Path, w: u32, h: u32, rgb: &[f32], digest: &str) -> Result<u64, String> {
    let img = ExrImage::new(
        w,
        h,
        ExrChannelLayout::Rgb,
        rgb.to_vec(),
        hdr_metadata(digest),
    )
    .map_err(|e| format!("EXR 构造: {e}"))?;
    let bytes = encode_exr(&img).map_err(|e| format!("EXR 编码: {e}"))?;
    std::fs::write(path, &bytes).map_err(|e| format!("EXR 落盘: {e}"))?;
    Ok(bytes.len() as u64)
}

// ---------------------------------------------------------------------------
// JSON 出报（手写，零新依赖）
// ---------------------------------------------------------------------------

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn jstr(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn take_arg(args: &[String], i: &mut usize) -> String {
    *i += 1;
    args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
}

// ---------------------------------------------------------------------------
// legs
// ---------------------------------------------------------------------------

fn contract_leg(path: &str) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| fail(&format!("契约读取: {e}")));
    // schema 分派（M-c upscale / M-d lumen 双契约面；fail-closed）。
    let schema = json_parse(&text).ok().and_then(|d| {
        d.get("schema")
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned())
    });
    if schema.as_deref() == Some(LUMEN_SCHEMA_ID) {
        match parse_lumen_contract(&text) {
            Ok(d) => println!("{d}"),
            Err(e) => fail(&e),
        }
        return;
    }
    match parse_contract(&text) {
        Ok(c) => println!("{}", c.digest),
        Err(e) => fail(&e),
    }
}

/// digest 自检：①内置最小合成对象（G13USP-1\0 + obj{arr:[u32 1,2], f:f64 1.5,
/// s:"ok"}，键 code point 升序）经本 bin 同一 enc 面编码 → sha256 须 ==
/// SELFTEST_TINY_DIGEST（python 独立实现锚，跨实现互证）；②实契约 digest 须 ==
/// 冻结注册值（契约文件在位时）。
fn selftest_leg(path: &str) {
    let mut buf = VERSION_PREFIX.to_vec();
    buf.push(0x07);
    enc_key(&mut buf, "arr");
    buf.push(0x09);
    enc_u32(&mut buf, 1);
    enc_u32(&mut buf, 2);
    buf.push(0x0a);
    enc_key(&mut buf, "f");
    enc_f64(&mut buf, 1.5);
    enc_key(&mut buf, "s");
    enc_str(&mut buf, "ok");
    buf.push(0x08);
    let tiny = format!("sha256:{}", sha256_hex(&buf));
    let tiny_ok = tiny == SELFTEST_TINY_DIGEST;
    let mut contract_ok = false;
    let mut contract_digest = String::new();
    if let Ok(text) = std::fs::read_to_string(path) {
        if let Ok(c) = parse_contract(&text) {
            contract_digest = c.digest.clone();
            contract_ok = c.digest == FROZEN_CONTRACT_DIGEST;
        }
    }
    let state = if tiny_ok && contract_ok {
        "pass"
    } else {
        "fail"
    };
    println!(
        "{{\"schema\":\"rurix.g13.upscale_parity.selftest.v1\",\"state\":{},\"tiny_digest\":{},\"tiny_expected\":{},\"tiny_ok\":{},\"contract_digest\":{},\"contract_frozen\":{},\"contract_ok\":{}}}",
        jstr(state),
        jstr(&tiny),
        jstr(SELFTEST_TINY_DIGEST),
        tiny_ok,
        jstr(&contract_digest),
        jstr(FROZEN_CONTRACT_DIGEST),
        contract_ok,
    );
    if state == "fail" {
        std::process::exit(1);
    }
}

fn default_gltf(scene_id: &str) -> &'static str {
    match scene_id {
        "cornell-box" => "K:/rurix_g10_cache/cornell-box-generated/v1/cornell_box.gltf",
        "bistro-interior" => {
            "K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf"
        }
        _ => fail("未知场景"),
    }
}

fn backend_provenance_json(
    backend: &Backend,
    spv_a: &str,
    spv_b: &str,
    vendor_report: Option<&VendorSessionReport>,
) -> String {
    match (backend, vendor_report) {
        (Backend::Tsr(_), _) => {
            let da = std::fs::read(spv_a)
                .map(|b| sha256_hex(&b))
                .unwrap_or_else(|_| "unreadable".into());
            let db = std::fs::read(spv_b)
                .map(|b| sha256_hex(&b))
                .unwrap_or_else(|_| "unreadable".into());
            format!(
                "{{\"kind\":\"tsr_device\",\"lane\":\"vk::run_compute 双腿 dispatch（M-b .rx kernel 面）\",\"spv_resample\":{},\"spv_resample_sha256\":{},\"spv_resolve\":{},\"spv_resolve_sha256\":{}}}",
                jstr(spv_a),
                jstr(&format!("sha256:{da}")),
                jstr(spv_b),
                jstr(&format!("sha256:{db}")),
            )
        }
        (_, Some(r)) => {
            let dlls: Vec<String> = r
                .dlls
                .iter()
                .map(|d| format!("[{},{},{}]", jstr(&d.name), jstr(&d.sha256), d.bytes))
                .collect();
            format!(
                "{{\"kind\":{},\"gpu\":{},\"engine_version\":{},\"dlls\":[{}]}}",
                jstr(&r.backend),
                jstr(&r.gpu_name),
                jstr(&r.engine_version),
                dlls.join(",")
            )
        }
        _ => "{\"kind\":\"unknown\"}".into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_leg(
    scene_id: &str,
    tier: u32,
    backend_name: &str,
    frames: u32,
    calibration: bool,
    contract_path: &str,
    gltf_path: &str,
    spv_resample: &str,
    spv_resolve: &str,
    out_root: &str,
    expect_digest: Option<&str>,
) {
    // ① 契约解析 + digest 门序（不等仍出报告即 RED 的承载面）。
    let text =
        std::fs::read_to_string(contract_path).unwrap_or_else(|e| fail(&format!("契约读取: {e}")));
    let contract = parse_contract(&text).unwrap_or_else(|e| fail(&e));
    let expect = expect_digest.unwrap_or(FROZEN_CONTRACT_DIGEST);
    if contract.digest != expect {
        fail(&format!(
            "契约 digest 不等仍出报告即 RED：harness 实算 {} ≠ 期望 {}——拒出图",
            contract.digest, expect
        ));
    }
    // ② 场景行/档位闭集。
    let srow = contract_scene_row(&contract.raw, scene_id).unwrap_or_else(|e| fail(&e));
    let res = srow
        .get("camera")
        .and_then(|c| c.get("resolution"))
        .unwrap();
    let out_w = res.get("w").and_then(|v| v.as_u64()).unwrap() as u32;
    let out_h = res.get("h").and_then(|v| v.as_u64()).unwrap() as u32;
    let tiers: Vec<u64> = contract
        .raw
        .get("tier_sequence")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    if !tiers.contains(&(tier as u64)) {
        fail(&format!("tier {tier} 越契约 tier_sequence 闭集"));
    }
    // 内部分辨率 = floor(输出 × tier%)（双向 floor 同一取整口径，契约 camera_source
    // 注释公式面；provenance 枚举值 1287×723 与 floor 得 1286×723 的分歧如实留痕
    // 汇报——provenance 非 digest 域，公式面 floor 为规范读法）。
    let in_w = ((out_w as u64 * tier as u64) / 100) as u32;
    let in_h = ((out_h as u64 * tier as u64) / 100) as u32;
    if in_w == 0 || in_h == 0 {
        fail("内部分辨率塌零");
    }
    let seed_main = contract.raw.get("seed").and_then(|v| v.as_u64()).unwrap();
    let seed_cal = contract
        .raw
        .get("calibration_seed")
        .and_then(|v| v.as_u64())
        .unwrap();
    let seed = if calibration { seed_cal } else { seed_main };
    let contract_frames = contract
        .raw
        .get("frame_count")
        .and_then(|v| v.as_u64())
        .unwrap() as u32;
    let frames = if frames == 0 { contract_frames } else { frames };
    if frames == 0 {
        fail("--frames 必须 ≥1");
    }

    // ③ 场景装配（DEV_ENV 三态：资产缺失 = dev_env degrade）。
    let scene = match assemble_scene(&contract.raw, scene_id, Path::new(gltf_path)) {
        Ok(s) => s,
        Err(e) => dev_env_or_fail("scene_assets", &e),
    };
    eprintln!(
        "{TAG}: 装配 scene={scene_id} tris={} emissive_tris={} quads={} points={} tex_mean={} internal={in_w}x{in_h} output={out_w}x{out_h}",
        scene.tri_count,
        scene.emissive_tri_count,
        scene.quads.len(),
        scene.points.len(),
        scene.texture_mean_albedo,
    );
    let eps = scene_eps(&scene.positions);
    let bvh = TriBvh::build(&scene.positions, &scene.indices);
    eprintln!(
        "{TAG}: BVH 建成（tris={} eps={eps:.6}）",
        bvh.triangle_count()
    );

    // ④ backend 创建（DEV_ENV 三态：GPU/vendor DLL 缺失 = dev_env degrade）。
    let mut backend = match backend_name {
        "tsr_device" => {
            let spv_a = load_spv(spv_resample);
            let spv_b = load_spv(spv_resolve);
            match TsrDeviceBackend::create(spv_a, spv_b) {
                Ok(b) => Backend::Tsr(b),
                Err(e) => dev_env_or_fail("tsr_device", &e),
            }
        }
        "dlss_sr" => match DlssBackend::create((in_w, in_h), (out_w, out_h)) {
            Ok(b) => Backend::Dlss(b),
            Err(e) => dev_env_or_fail("dlss_sr", &e),
        },
        "fsr_3_1_5" => match FsrBackend::create((in_w, in_h), (out_w, out_h)) {
            Ok(b) => Backend::Fsr(b),
            Err(e) => dev_env_or_fail("fsr_3_1_5", &e),
        },
        other => fail(&format!(
            "未知 backend: {other}（tsr_device|dlss_sr|fsr_3_1_5）"
        )),
    };
    eprintln!("{TAG}: backend {} 就绪", backend.name());

    // ⑤ 帧序：Halton jitter（seed 派生窗口；RXS-0357 L2 固定 seed 位级确定性继承）。
    let jitter_base = (seed % JITTER_WINDOW_MOD) as u32;
    let exposure = 2.0f32.powf(-scene.ev100);
    let out_dir = PathBuf::from(out_root)
        .join(scene_id)
        .join(format!("tier{tier}"))
        .join(backend.name());
    let frames_dir = out_dir.join("frames");
    std::fs::create_dir_all(&frames_dir).unwrap_or_else(|e| fail(&format!("输出目录: {e}")));
    let mut frames_json: Vec<String> = Vec::new();
    let mut frame_ms: Vec<f64> = Vec::new();
    let mut upscale_ms: Vec<f64> = Vec::new();
    let mut scene_ms: Vec<f64> = Vec::new();
    let mut converged: Option<ImageF32> = None;
    let mut converged_digest = String::new();
    let mut prev_vp: Option<Mat4> = None;
    for i in 0..frames {
        let t_frame = std::time::Instant::now();
        let t_scene = std::time::Instant::now();
        let j = [
            halton(jitter_base + i + 1, 2) - 0.5,
            halton(jitter_base + i + 1, 3) - 0.5,
        ];
        let (color, depth) = render_frame(&scene, &bvh, in_w, in_h, j, eps);
        let vp = build_vp(&scene.camera, in_w, in_h);
        let vp_j = jittered_vp(&vp, j, in_w, in_h);
        let mv = match prev_vp {
            Some(prev) => compute_camera_mv(&depth, &vp_j, &prev),
            None => ImageF32::new(in_w, in_h, 2),
        };
        prev_vp = Some(vp_j);
        let scene_el = t_scene.elapsed().as_secs_f64() * 1000.0;
        let t_up = std::time::Instant::now();
        let inputs = UpscaleInputs {
            color: &color,
            depth: &depth,
            mv: &mv,
            reactive: None,
            exposure,
            jitter: j,
            output_size: (out_w, out_h),
            frame_index: i,
            reset: i == 0,
        };
        let out = backend.upscale(&inputs);
        let up_el = t_up.elapsed().as_secs_f64() * 1000.0;
        if !out.data.iter().all(|v| v.is_finite()) {
            fail(&format!("帧 {i} upscale 输出非有限"));
        }
        let name = format!("frame_{i:04}.exr");
        let path = frames_dir.join(&name);
        let bytes = write_exr(&path, out_w, out_h, &out.data, &contract.digest)
            .unwrap_or_else(|e| fail(&e));
        let digest = frame_content_digest(out.w, out.h, 3, &out.data);
        frames_json.push(format!(
            "{{\"name\":{},\"bytes\":{},\"digest\":{}}}",
            jstr(&format!("frames/{name}")),
            bytes,
            jstr(&digest)
        ));
        converged_digest = digest;
        converged = Some(out);
        let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
        scene_ms.push(scene_el);
        upscale_ms.push(up_el);
        frame_ms.push(frame_el);
        if i == 0 || (i + 1) % 8 == 0 || i + 1 == frames {
            eprintln!(
                "{TAG}: 帧 {}/{frames} scene={scene_el:.1}ms upscale={up_el:.1}ms",
                i + 1
            );
        }
    }
    let converged = converged.expect("至少一帧");
    let converged_bytes = write_exr(
        &out_dir.join("converged.exr"),
        out_w,
        out_h,
        &converged.data,
        &contract.digest,
    )
    .unwrap_or_else(|e| fail(&e));

    // ⑥ receipt。
    let vendor_report = match &backend {
        Backend::Dlss(b) => Some(b.session.report()),
        Backend::Fsr(b) => Some(b.session.report()),
        Backend::Tsr(_) => None,
    };
    let provenance =
        backend_provenance_json(&backend, spv_resample, spv_resolve, vendor_report.as_ref());
    let join_ms = |v: &[f64]| {
        v.iter()
            .map(|x| format!("{x:.6}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let require_real_str = std::env::var("RURIX_REQUIRE_REAL").unwrap_or_else(|_| "0".into());
    let validation_str = std::env::var("RURIX_VK_VALIDATION").unwrap_or_else(|_| "0".into());
    let receipt = format!(
        "{{\n  \"schema\": \"rurix.g13.ue_upscale_parity_rurix_receipt.v1\",\n  \"contract\": {},\n  \"contract_digest_rurix\": {},\n  \"scene_id\": {},\n  \"tier\": {},\n  \"backend\": {},\n  \"seed_role\": {},\n  \"seed\": {},\n  \"jitter_protocol\": {},\n  \"frame_count\": {},\n  \"output_size\": [{}, {}],\n  \"internal_size\": [{}, {}],\n  \"internal_rounding\": \"floor(out*tier/100) 双向 floor 同一口径\",\n  \"exposure\": {},\n  \"lighting_model\": \"direct_only_lambert_twosided + emissive_primary（无 GI/天光——契约 sun/sky=0.0 显式登记；不冒充 GI 帧）\",\n  \"texture_mean_albedo\": {},\n  \"tri_count\": {},\n  \"emissive_tri_count\": {},\n  \"gltf_path\": {},\n  \"gltf_sha256\": {},\n  \"frames\": [{}],\n  \"frame_ms\": [{}],\n  \"upscale_ms\": [{}],\n  \"scene_render_ms\": [{}],\n  \"timer\": \"host Instant 墙钟；frame_ms = 逐帧全链路（场景帧渲染+MV+upscale），scene_render_ms/upscale_ms = 分项\",\n  \"converged_frame\": \"converged.exr\",\n  \"converged_bytes\": {},\n  \"converged_digest\": {},\n  \"digest_payload\": \"G10EXRD-1\\\\0 + w:u32LE + h:u32LE + c:u8 + f32LE pixels（G12.4 frame_content_digest 同构）\",\n  \"backend_provenance\": {},\n  \"env\": {{\"RURIX_REQUIRE_REAL\": {}, \"RURIX_VK_VALIDATION\": {}}}\n}}\n",
        jstr(&contract_path.replace('\\', "/")),
        jstr(&contract.digest),
        jstr(scene_id),
        tier,
        jstr(backend.name()),
        jstr(if calibration { "calibration" } else { "main" }),
        seed,
        jstr(&format!(
            "halton(2,3) centered [-0.5,0.5) 输入像素单位；窗口 base = seed % {JITTER_WINDOW_MOD}；jitter_i = [halton(base+i+1,2)-0.5, halton(base+i+1,3)-0.5]（RXS-0357 L2/RXS-0400 固定 seed 位级确定性继承）"
        )),
        frames,
        out_w,
        out_h,
        in_w,
        in_h,
        exposure,
        scene.texture_mean_albedo,
        scene.tri_count,
        scene.emissive_tri_count,
        jstr(&gltf_path.replace('\\', "/")),
        jstr(&format!("sha256:{}", scene.gltf_sha256)),
        frames_json.join(","),
        join_ms(&frame_ms),
        join_ms(&upscale_ms),
        join_ms(&scene_ms),
        converged_bytes,
        jstr(&converged_digest),
        provenance,
        jstr(&require_real_str),
        jstr(&validation_str),
    );
    let receipt_path = out_dir.join("render_receipt.json");
    std::fs::write(&receipt_path, &receipt).unwrap_or_else(|e| fail(&format!("receipt 落盘: {e}")));
    println!(
        "{TAG}: PASS scene={scene_id} tier={tier} backend={} frames={frames} converged={} out={}",
        backend.name(),
        converged_digest,
        out_dir.display()
    );
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        fail("缺子模式（--contract-digest / --selftest-digest / --render）");
    }
    match args[1].as_str() {
        "--contract-digest" => {
            // 路径面：--contract <p> 或位置参数（G12.4 同形）；默认契约路径。
            let mut path = DEFAULT_CONTRACT.to_owned();
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--contract" => path = take_arg(&args, &mut i),
                    other if !other.starts_with("--") => path = other.to_owned(),
                    other => fail(&format!("未知参数 {other}")),
                }
                i += 1;
            }
            contract_leg(&path);
        }
        "--selftest-digest" => {
            let mut path = DEFAULT_CONTRACT.to_owned();
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--contract" => path = take_arg(&args, &mut i),
                    other => fail(&format!("未知参数 {other}")),
                }
                i += 1;
            }
            selftest_leg(&path);
        }
        "--render" => {
            let mut scene_id = String::new();
            let mut tier: u32 = 0;
            let mut backend = String::new();
            let mut frames: u32 = 0;
            let mut calibration = false;
            let mut contract_path = DEFAULT_CONTRACT.to_owned();
            let mut gltf_path = String::new();
            let mut spv_resample = DEFAULT_SPV_RESAMPLE.to_owned();
            let mut spv_resolve = DEFAULT_SPV_RESOLVE.to_owned();
            let mut out_root = DEFAULT_OUT_ROOT.to_owned();
            let mut expect_digest: Option<String> = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--scene" => scene_id = take_arg(&args, &mut i),
                    "--tier" => {
                        tier = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--tier 非 u32"))
                    }
                    "--backend" => backend = take_arg(&args, &mut i),
                    "--frames" => {
                        frames = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--frames 非 u32"))
                    }
                    "--calibration-seed" => calibration = true,
                    "--contract" => contract_path = take_arg(&args, &mut i),
                    "--gltf" => gltf_path = take_arg(&args, &mut i),
                    "--spv-resample" => spv_resample = take_arg(&args, &mut i),
                    "--spv-resolve" => spv_resolve = take_arg(&args, &mut i),
                    "--out-root" => out_root = take_arg(&args, &mut i),
                    "--expect-digest" => expect_digest = Some(take_arg(&args, &mut i)),
                    other => fail(&format!("未知参数 {other}")),
                }
                i += 1;
            }
            if scene_id.is_empty() || tier == 0 || backend.is_empty() {
                fail("--render 参数闭集缺行（scene/tier/backend）");
            }
            if gltf_path.is_empty() {
                gltf_path = default_gltf(&scene_id).to_owned();
            }
            render_leg(
                &scene_id,
                tier,
                &backend,
                frames,
                calibration,
                &contract_path,
                &gltf_path,
                &spv_resample,
                &spv_resolve,
                &out_root,
                expect_digest.as_deref(),
            );
        }
        other => fail(&format!("未知子模式 {other}")),
    }
}
