// Assisted-by: Kimi-K3(G31+ 波 A Task A1 拆分)
// 本文件 = g14_3_pipeline_perf 生产管线共享体(G14.3 M-c harness 全量实现减 fn main):
// g14_3_pipeline_perf.rs(bench/render 契约锚)与 g31_window_present.rs(G31 真窗口
// present)两 bin 经 `include!` 逐字共享(vk.rs L16783 include vk_m50_rt_body.rs 同型先例)。
// 语义冻结:本文件任何改动同时影响两 bin——g14_3 的 --contract-digest/--selftest-digest/
// bench 契约行为是回归锚,digest 锚定逻辑禁动。


use image_io::exr::{
    ChromaticitiesOrigin, ExrBitDepth, ExrChannelLayout, ExrDerivation, ExrDomain, ExrImage,
    ExrMetadata, ExrSourceEnd, ExrTransfer, decode_exr, encode_exr,
};
use image_io::{ImageBuffer, ImageFormat, Rgb, encode as encode_image};
use rurix_render::display::aces13::Aces13;
use rurix_render::display::post_chain::{ExposureState, PostProcessChain};
use rurix_render::display::view_transform::{DisplayParams, OutputEncoding};
// G31+ #58 簇 DAG LOD 生产接线（--cluster-lod 面）：生产金标准直调单源
//（cull 投影判据 / cut 覆盖性机核 / 64B 簇记录契约），禁旁路重算。
use rurix_render::geometry::cull::CullCamera;
use rurix_render::geometry::skinning::{BoneTransform, SkinPalette, skin_vertex};
use rurix_render::geometry::visible_cluster_set::{DagNodeRec, LodBounds};
use rurix_render::graph::types::ClusterRecord;
use rurix_render::streaming::svt;
use rurix_render::temporal::common::{
    Mat4, compute_camera_mv, halton, look_at_rh, perspective_rh_zo,
};
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::tsr::TsrParams;
use rurix_render::temporal::upscale::{UpscaleBackend, UpscaleInputs};
use rurix_rt::render_exec::{
    AccelStructDesc, Bindings, BlasRefitUpdate, BufferDesc, BufferUsage, ComputePass,
    DeviceFrameOutput, DeviceFrameSession, DispatchSpec, FrameTicket, FrameUpdate, Pass,
    RayQueryInstanceDesc, RayQuerySceneDesc, RayQueryTransformedInstanceDesc, Readback,
    ResourceDesc, SlotAsGroup, StableResourceId, SubmissionProvenance, TargetState, TexFormat,
    TextureDesc, TextureUsage, TlasBuildAction,
};
use rurix_rt::vendor_upscale::{
    DlssVkSession, ExternalImageImportDesc, ExternalInputSlot, FsrDx12Session,
    VendorExternalFrameParams, VendorFrameInput, VendorSessionReport, fsr_sdk_dir,
    streamline_sdk_dir,
};
use rurix_rt::vk;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

const TAG: &str = "[g14_3_pipeline_perf]";
/// RXS-0405 L2 冻结版本前缀（47 31 33 55 53 50 2D 31 00；G13.4 同字面——同一
/// 契约文件消费面）。
const VERSION_PREFIX: &[u8] = b"G13USP-1\0";
const SCHEMA_ID: &str = "rurix.g13.ue_upscale_parity_contract.v1";
/// 三方互证冻结注册值（G13.4 同一锚；--render/--bench 默认比对锚）。
const FROZEN_CONTRACT_DIGEST: &str =
    "sha256:137483a1696481971fc0da03fad1a188ef6f048243e4616953060014f1d0872f";
/// --selftest-digest 内置最小合成对象 digest 锚（python 独立实现产，G13.4 同字面）。
const SELFTEST_TINY_DIGEST: &str =
    "sha256:4b091627caebafdcbd85fd877c6fa969430337af1fa23a774e6d25b432616c62";
const UNIT_NORM_TOL: f64 = 9.094947017729282e-13; // 2^-40（RXS-0384 L2 谓词常量）
const DEFAULT_CONTRACT: &str = "milestones/g13/g13_ue_upscale_parity_contract.json";
const DEFAULT_OUT_ROOT: &str = "K:/rurix-ext/g14-frames/rurix_prod";
const DEFAULT_SPV_SCENE: &str = ".tmp/g14_gates/m_c/g14_3_direct_gi.spv";
const DEFAULT_SPV_GI: &str = ".tmp/g14_gates/m_c/g16_gi_multibounce.spv";
/// G18 M-a 加性光照纵深 profile（禁动 --gi off 默认臂 SPV/digest 锚）。
const DEFAULT_SPV_G18_LIGHT: &str = ".tmp/g14_gates/m_c/g18_light_transport_depth.spv";
/// D2 平滑顶点法线臂 scene kernel SPV（kernels/g18_smooth_nrm.rx 编译产物；
/// 仅 --smooth-normals on 换载——默认臂 SPV 面 0-byte）。
#[allow(dead_code)] // D2:g14_3_pipeline_perf 独消费面(g31_window_present 未消费,诚实标注)
const DEFAULT_SPV_G18_SMOOTH_NRM: &str = ".tmp/night_0828/spv/g18_smooth_nrm.spv";
/// day_0828 Phase C GI2 臂 scene kernel SPV（kernels/g31_texture_nrm_gi.rx
/// 现编译产物含 GI2 段——统一质量 kernel，bench 腿绑哑表五件走 mats 均值面；
/// 仅 --gi2 on 换载，默认/既有臂 SPV 面 0-byte。**路线隔离**（A2b v2 同律）：
/// g31_texture_nrm_gi.spv 锚定字节承载 gi2-off 合流锚（8b1c12f3）不动，GI2
/// 变体独立成文件——与 g31_window_present G31_DEFAULT_SPV_TEXTURE_NRM_GI2
/// 同一文件（gi2-on 车道单一事实源）。）
#[allow(dead_code)] // Phase C:g14_3_pipeline_perf 独消费面(诚实标注)
const DEFAULT_SPV_G31_TEXNRM_GI: &str = ".tmp/night_0828/spv/g31_texture_nrm_gi_gi2.spv";
const G18_PRESENTATION_CONTRACT: &str = "milestones/g18/g18_presentation_contract.json";
const G18_PRESENTATION_FRAMES_MIN: u32 = 128;
// G14.9（RFC-0030 §4.5 L1）：TSR 双腿默认切换到 g14_8 调度变体 SPV（8×8 2D
// 线程组，数学面与 g13_tsr_* 逐字同源位级不变；原 g13 kernel/SPV 0-byte 保留
// ——G13 M-b 门消费面 + RD-045 归因对照臂，--spv-resample/--spv-resolve 可
// 显式指回旧面做对照）。
const DEFAULT_SPV_RESAMPLE: &str = ".tmp/g14_gates/m_c/g14_8_tsr_resample.spv";
const DEFAULT_SPV_RESOLVE: &str = ".tmp/g14_gates/m_c/g14_8_tsr_resolve.spv";
/// day_0828 Phase D TSR 降噪质量档 resolve 变体（--tsr-quality on 独载腿;
/// off 臂恒载上行 DEFAULT_SPV_RESOLVE 冻结字节——C 相纪律:保锚字节隔离。
/// fork 自 g14_8_tsr_resolve.rx 三质量面:① Karis 反亮度加权混合;② 稳态
/// alpha 档 tsr_params[19] 直入公式 base 位（母版 min_alpha=0.04 地板在
/// tighten=0.5 下构造性不可达——红修 v2 语义修正,详 kernel 头注〕;③ 深度
/// 验证 3×3 膨胀区间化（v3——深度边缘像素不再随 jitter 恒拒史〕+ 可选 3×3
/// 邻域亮度 clamp〔[20]=K,0=关〕）。
#[allow(dead_code)] // Phase D:g14_3_pipeline_perf/g31_window_present 消费面(诚实标注)
const DEFAULT_SPV_RESOLVE_Q: &str = ".tmp/night_0828/spv/g31_tsr_resolve_q.spv";
/// G14.10 相机 MV GPU kernel（RFC-0030 §4.1 授权行；统一四 pass 车道 pass1）。
const DEFAULT_SPV_MV: &str = ".tmp/g14_gates/m_c/g14_mv.spv";
/// G31+ 波 A Task A4 动态场景 kernel（g14_3_direct_gi 逐字镜像 + 实例感知分派；
/// 仅 --dyn-demo 模式消费——静态面 SPV 路径 0-byte）。
const DEFAULT_SPV_DYN_SCENE: &str = ".tmp/g14_gates/m_c/g31_dyn_scene.spv";
/// jitter 派生窗口模数（素数；base = seed % 65521，与 g13_4 同模——M-d 锚）。
const JITTER_WINDOW_MOD: u64 = 65521;
/// 主射线 t_max（host TriBvh 无界求交面的 device 兑现；1e30 常量族沿 M96/M100）。
const RAY_TMAX: f32 = 1e30;
/// 帧参数 SSBO 长度（f32；[0..42) 与 kernels/g14_3_direct_gi.rx 参数面逐字
/// 同源，[42]=sky [43]=smooth_nrm [44..48)=ambient [48]=ggx [49..56) 保留恒 0）。
/// D6：48→56 扩面（GGX 使能位 params[48]）——基线 kernel 只读 [0..42]+[42]，
/// 尾部追加零不消费；params0_bytes/逐帧上传两侧同由本常量派生，全车道
/// （含 g31/g34/g35 共享体调用面）长度自洽，Stage A 锚零漂移由 D6 自验承载。
const PARAMS_LEN: usize = 56;

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
        fail(&format!("{what} 不可用（RURIX_REQUIRE_REAL=1，禁 mock 充真跑）: {err}"));
    }
    println!(
        "{{\"schema\":\"rurix.g14.pipeline_perf.skip.v1\",\"state\":\"skipped_dev_env\",\"what\":{},\"reason\":{}}}",
        jstr(what),
        jstr(err)
    );
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// 最小 JSON 解析（bin-local 独立实现；int/float 字面区分保留——python json 类型
// 谓词同构面：u32/u64 字段拒 float 字面。重复键拒；控制字符/坏转义拒；深度限 64）
// G14.3：g13_4 同型复制子集（bin-local 惯例）
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
            Json::Num { raw, integral: true, .. } => raw.parse::<u64>().ok(),
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
                    let s = std::str::from_utf8(&self.b[self.i - 1..]).map_err(|_| "JSON: UTF-8")?;
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
    let mut p = JParser { b: text.as_bytes(), i: 0, depth: 0 };
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err("JSON: 尾部余留字节".into());
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// 契约解析（RXS-0405 L1 字段闭集 fail-closed + 约束谓词；upscale 契约单面——
// G14.3 不消费 lumen 契约面）
// G14.3：g13_4 同型复制子集（bin-local 惯例）
// ---------------------------------------------------------------------------

fn cerr(msg: impl Into<String>) -> String {
    format!("契约解析: {}", msg.into())
}

fn as_f64(name: &str, v: &Json) -> Result<f64, String> {
    let x = v.as_f64().ok_or_else(|| cerr(format!("{name}: expected f64")))?;
    if !x.is_finite() {
        return Err(cerr(format!("{name}: NaN/Inf forbidden")));
    }
    Ok(x)
}

fn as_u32(name: &str, v: &Json) -> Result<u32, String> {
    let x = v.as_u64().ok_or_else(|| cerr(format!("{name}: expected u32")))?;
    if x > u32::MAX as u64 {
        return Err(cerr(format!("{name}: u32 越域 {x}")));
    }
    Ok(x as u32)
}

fn as_u64(name: &str, v: &Json) -> Result<u64, String> {
    v.as_u64().ok_or_else(|| cerr(format!("{name}: expected u64")))
}

fn as_str<'a>(name: &str, v: &'a Json) -> Result<&'a str, String> {
    let s = v.as_str().ok_or_else(|| cerr(format!("{name}: expected str")))?;
    if s.is_empty() {
        return Err(cerr(format!("{name}: empty str")));
    }
    Ok(s)
}

fn as_bool(name: &str, v: &Json) -> Result<bool, String> {
    v.as_bool().ok_or_else(|| cerr(format!("{name}: expected bool")))
}

fn as_f64v(name: &str, v: &Json, n: usize) -> Result<Vec<f64>, String> {
    let a = v.as_array().ok_or_else(|| cerr(format!("{name}: expected array")))?;
    if a.len() != n {
        return Err(cerr(format!("{name}: expected f64×{n}")));
    }
    a.iter()
        .enumerate()
        .map(|(i, x)| as_f64(&format!("{name}[{i}]"), x))
        .collect()
}

fn closed<'a>(name: &str, v: &'a Json, keys: &[&str]) -> Result<&'a Json, String> {
    let obj = v.as_object().ok_or_else(|| cerr(format!("{name}: expected obj")))?;
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
    as_str("m133_manifest_digest", s.get("m133_manifest_digest").unwrap())?;
    as_str("gltf_product_digest", s.get("gltf_product_digest").unwrap())?;
    let cam = s.get("camera").unwrap();
    closed(
        "camera",
        cam,
        &["position", "orientation_quat", "fov_y_deg", "near", "far", "resolution"],
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
    closed("material_policy", mp, &["texture_mean_albedo", "white_tex_to_white"])?;
    as_bool("texture_mean_albedo", mp.get("texture_mean_albedo").unwrap())?;
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
    let tiers_a = tiers_v.as_array().ok_or_else(|| cerr("tier_sequence 非数组"))?;
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
    if as_str("ue_temporal_upscaler", pol.get("ue_temporal_upscaler").unwrap())? != "dlss_plugin" {
        return Err(cerr("rendering_policy.ue_temporal_upscaler 须 const dlss_plugin"));
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

// ---------------------------------------------------------------------------
// canonical preimage（RXS-0405 L2：标签/键序/宽度 RXS-0384 L3 同构；与
// milestones/g13/harness/ue_python/g13_parity_contract.py 逐字同表）
// G14.3：g13_4 同型复制子集（bin-local 惯例；upscale 契约单面）
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
    if schema != SCHEMA_ID {
        return Err(cerr(format!("未知 schema 字面: {schema}")));
    }
    let mut buf = VERSION_PREFIX.to_vec();
    let pairs = doc.as_object().unwrap();
    // digest 域 = 根字段闭集除 provenance。
    let body = Json::Obj(
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
// 最小 glTF 装载（几何 accessors + 节点树世界变换 + 材质表；bin-local——
// rurix-render 不依赖 rurix-asset（循环依赖禁区），语义沿 G12.4
// load_prod_scene 同律：扁平单引用面、逐三角材质扁平化、契约灯面）
// G14.3：g13_4 同型复制子集（bin-local 惯例）
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

/// 方向向量世界变换（M4 上左 3×3 旋转/缩放部分，平移丢弃；法线面专用——
/// 与 xform 同矩阵同 f64 左结合序，仅省 m[r][3] 平移项。非均匀缩放的逆置
/// 变换未接线——bistro 节点面 = 旋转+平移，调用侧 norm3 归一化兜底量级；
/// D2 平滑法线臂消费，off 面不调用 0-byte）。
fn xform_dir(m: &M4, p: [f32; 3]) -> [f32; 3] {
    let (x, y, z) = (p[0] as f64, p[1] as f64, p[2] as f64);
    [
        (m[0][0] * x + m[0][1] * y + m[0][2] * z) as f32,
        (m[1][0] * x + m[1][1] * y + m[1][2] * z) as f32,
        (m[2][0] * x + m[2][1] * y + m[2][2] * z) as f32,
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
        let off = bv
            .get("byteOffset")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize
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

    /// TEXCOORD_0（float VEC2；G31+ 波 B Task B4 贴图采样 UV 面——仅
    /// --textures on 消费,off 路径不调用,既有面 0-byte）。
    #[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
    fn texcoords(&self, idx: usize) -> Result<Vec<[f32; 2]>, String> {
        let (count, ctype, comps, stride, data) = self.accessor_bytes(idx)?;
        if ctype != 5126 || comps != 2 {
            return Err("TEXCOORD_0 须 float VEC2".into());
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
            out.push([f(0), f(1)]);
        }
        Ok(out)
    }

    /// NORMAL（float VEC3；D2 平滑顶点法线臂消费面——仅 --smooth-normals on
    /// 经 assemble_scene_nrm 消费，off 路径不调用，既有面 0-byte）。
    #[allow(dead_code)] // D2:g14_3_pipeline_perf --smooth-normals on 独消费面(g31_window_present 未消费,诚实标注)
    fn normals(&self, idx: usize) -> Result<Vec<[f32; 3]>, String> {
        let (count, ctype, comps, stride, data) = self.accessor_bytes(idx)?;
        if ctype != 5126 || comps != 3 {
            return Err("NORMAL 须 float VEC3".into());
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
            ))
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
// G14.3：g13_4 同型复制子集（bin-local 惯例）
// ---------------------------------------------------------------------------

// G36：QuadLight/PointLight 派生 Clone（纯 f32 字段面——geo_rebuild 借用重建
// 的加性承载,零行为变更;CameraSpec Copy 派生同律先例）。
#[derive(Clone)]
struct QuadLight {
    p00: [f32; 3],
    e1: [f32; 3],
    e2: [f32; 3],
    le: [f32; 3],
}

#[derive(Clone)]
struct PointLight {
    pos: [f32; 3],
    intensity: [f32; 3], // color × intensity_cd（G12.4 同口径：点强 I 即 cd 直给）
    /// A1 灯光提取加性臂：灯半径（米）——阴影射线 t_max 提前截断量（kernel
    /// t_sh = d − max(2·eps, r)，消提取代表点光在灯罩几何内部的自遮蔽）。
    /// 契约灯恒 0.0 ⇒ pack_points 第 7 槽字节与既有零填充位级不变。
    radius: f32,
}

// G34：CameraSpec 派生 Copy（纯 f32 字段面——g34_full_lane 帧循环按值复用
// 契约相机位姿的加性承载,零行为变更）。
#[derive(Clone, Copy)]
struct CameraSpec {
    eye: [f32; 3],
    forward: [f32; 3],
    up0: [f32; 3],
    fov_y_rad: f32,
    near: f32,
    far: f32,
}

/// 逐三角 glTF 材质索引面值（G31+ 波 B Task B3 slab 施加消费面；u32::MAX =
/// 无材质/灯面三角——slab 不消费；纯 provenance 记录，既有计算面 0-byte）。
const SLAB_TRI_NONE: u32 = u32::MAX;

struct SceneData {
    positions: Vec<[f32; 3]>,
    indices: Vec<[u32; 3]>,
    albedo: Vec<[f32; 3]>,
    emission: Vec<[f32; 3]>,
    /// 逐三角 glTF material_index（= indices.len()；SLAB_TRI_NONE = 无材质/灯面）。
    tri_mat: Vec<u32>,
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

/// G31+ 波 B Task B1 HZB 生产接线：逐 mesh 节点分组记录（剔除对象粒度 = TLAS
/// 实例粒度）。三角形段 = 装配序全局汤内的半开区间 `[tri_offset, tri_offset +
/// tri_count)`（与 `pack_tris` 9 f32/tri 扁平化同序 ⇒ 节点段切片与单 BLAS 面
/// 位级同 buffer）；世界 AABB = 烘焙顶点精确 min/max（三角形 ⊆ AABB 精确包含,
/// 剔除测试保守方向零假阳性）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct SceneNodeGroup {
    tri_offset: u32,
    tri_count: u32,
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
}

/// `assemble_scene` 0-byte 包装（既有行为逐字;G31+ 波 B Task B1 加性面 =
/// [`assemble_scene_ex`] 的 `groups = None` 形态）。
fn assemble_scene(
    contract: &Json,
    scene_id: &str,
    gltf_path: &Path,
) -> Result<SceneData, String> {
    assemble_scene_ex(contract, scene_id, gltf_path, None, None)
}

/// G31+ 波 B Task B4 纹理采样装配面（--textures on）：既有装配 + 逐三角
/// TEXCOORD_0（6 f32/tri 与 tris 同序同源;quad 灯面尾段恒 0——tritex −1
/// 不消费面）。SceneData 各字段与 off 面逐位同值（UV = 旁路 sink 纯记录）。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn assemble_scene_uv(
    contract: &Json,
    scene_id: &str,
    gltf_path: &Path,
    uv_out: &mut Vec<f32>,
) -> Result<SceneData, String> {
    assemble_scene_ex(contract, scene_id, gltf_path, None, Some(uv_out))
}

/// D2 平滑顶点法线装配面（--smooth-normals on）：既有装配 + 逐三角顶点法线
/// 侧表（9 f32/tri 与 tris 同序同源〔n0,n1,n2 绕序〕；世界变换旋转 3×3 部分
/// 经 xform_dir 变换 + norm3 归一化；quad 灯面尾段恒 0）。SceneData 各字段
/// 与 off 面逐位同值（法线 = 旁路 sink 纯记录）；off 路径不调用本面——不读
/// NORMAL、不产侧表，既有装配 0-byte。
#[allow(dead_code)] // D2:g14_3_pipeline_perf --smooth-normals on 独消费面(g31_window_present 未消费,诚实标注)
fn assemble_scene_nrm(
    contract: &Json,
    scene_id: &str,
    gltf_path: &Path,
    nrm_out: &mut Vec<f32>,
) -> Result<SceneData, String> {
    assemble_scene_ex_nrm(contract, scene_id, gltf_path, None, None, Some(nrm_out), None)
}

/// D6 GGX 高光臂装配面（--smooth-normals on --ggx on 消费）：既有装配 + 逐
/// 三角顶点法线侧表（9 f32/tri，同 assemble_scene_nrm）+ 逐三角金属度/粗糙
/// 度侧表（2 f32/tri [metallic, roughness]，取所在 primitive 材质
/// pbrMetallicRoughness 因子〔glTF 规范缺省 metal 1.0/rough 1.0〕；matless
/// primitive = 介质保守缺省 [0,1]——与既有 albedo None 臂不乘 (1−metallic)
/// 同律；quad 灯面尾段恒 0）。SceneData 各字段与 off 面逐位同值（MR = 旁路
/// sink 纯记录）；--ggx off 路径不调用本面——不读 roughnessFactor 进侧表、
/// 不产侧表，既有装配 0-byte。
#[allow(dead_code)] // D6:g14_3_pipeline_perf --ggx on 独消费面(诚实标注)
fn assemble_scene_nrm_mr(
    contract: &Json,
    scene_id: &str,
    gltf_path: &Path,
    nrm_out: &mut Vec<f32>,
    mr_out: &mut Vec<f32>,
) -> Result<SceneData, String> {
    assemble_scene_ex_nrm(
        contract,
        scene_id,
        gltf_path,
        None,
        None,
        Some(nrm_out),
        Some(mr_out),
    )
}

/// 场景装配全量实现（G31+ 波 B Task B1：`groups = Some` 时逐 mesh 节点追加
/// [`SceneNodeGroup`] 纯记录面——SceneData 各字段与 `None` 形态逐位同值,
/// 装配产物 0-byte;零三角形节点不产组,quad 面光几何尾段自立一组〔契约
/// 照明面几何逐字一致追加段,见下〕。G31+ 波 B Task B4：`uv_out = Some` 时
/// 逐三角 TEXCOORD_0 追加记录面——SceneData 各字段与 `None` 形态逐位同值,
/// 装配产物 0-byte;sink 布局 = 6 f32/tri〔uv0,uv1,uv2 顶点序与 tris 同源〕）。
/// D2：签名 0-byte 保持（g31_window_present/g34_2_hzb/--dump-scene 三调用面
/// 不触）——法线面委托 [`assemble_scene_ex_nrm`] 的 `nrm_out = None` 形态。
fn assemble_scene_ex(
    contract: &Json,
    scene_id: &str,
    gltf_path: &Path,
    groups: Option<&mut Vec<SceneNodeGroup>>,
    uv_out: Option<&mut Vec<f32>>,
) -> Result<SceneData, String> {
    assemble_scene_ex_nrm(contract, scene_id, gltf_path, groups, uv_out, None, None)
}

/// 装配全量实现 D2 扩面（`nrm_out = Some` 时逐三角顶点法线追加记录面——
/// SceneData 各字段与 `None` 形态逐位同值，装配产物 0-byte；sink 布局 =
/// 9 f32/tri〔n0,n1,n2 顶点序与 tris 同源，世界旋转 3×3 变换 + norm3 归一
/// 化〕，quad 灯面尾段恒 0；off 面不读 NORMAL 不算侧表）。
/// D6 扩面（`mr_out = Some` 时逐三角 [metallic, roughness] 追加记录面——
/// 2 f32/tri 与 tris 同序同源，取所在 primitive 材质因子；matless = [0,1]
/// 介质保守缺省；quad 灯面尾段恒 0；off 面不产侧表 0-byte）。
fn assemble_scene_ex_nrm(
    contract: &Json,
    scene_id: &str,
    gltf_path: &Path,
    mut groups: Option<&mut Vec<SceneNodeGroup>>,
    mut uv_out: Option<&mut Vec<f32>>,
    mut nrm_out: Option<&mut Vec<f32>>,
    mut mr_out: Option<&mut Vec<f32>>,
) -> Result<SceneData, String> {
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
    // D6：+= roughness（pbrMetallicRoughness.roughnessFactor，glTF 规范缺省
    // 1.0）——tri_mr 侧表源；既有字段读取/消费面 0-byte。
    struct MatRec {
        factor: [f32; 3],
        metallic: f32,
        roughness: f32,
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
            .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(1.0) as f32).collect::<Vec<_>>());
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
            roughness: pbr
                .and_then(|p| p.get("roughnessFactor"))
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
    let mut tri_mat: Vec<u32> = Vec::new();
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
        // Task B1 节点分组记录面：本节点三角形段起点（零三角形节点不产组）。
        let node_tri_start = indices.len();
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
            let mat_idx = prim.get("material").and_then(|v| v.as_u64()).map(|x| x as u32);
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
            // Task B4：TEXCOORD_0 读取（uv_out 消费面;off 面不读不算,0-byte）。
            let uvs: Option<Vec<[f32; 2]>> = if uv_out.is_some() {
                let uv_acc = prim
                    .get("attributes")
                    .and_then(|a| a.get("TEXCOORD_0"))
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| cerr("primitive 缺 TEXCOORD_0（--textures on 面 fail-closed）"))?;
                Some(gltf.texcoords(uv_acc as usize)?)
            } else {
                None
            };
            // D2：NORMAL 读取（nrm_out 消费面；off 面不读不算，0-byte）。
            let nrms: Option<Vec<[f32; 3]>> = if nrm_out.is_some() {
                let nrm_acc = prim
                    .get("attributes")
                    .and_then(|a| a.get("NORMAL"))
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| cerr("primitive 缺 NORMAL（--smooth-normals on 面 fail-closed）"))?;
                Some(gltf.normals(nrm_acc as usize)?)
            } else {
                None
            };
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
                tri_mat.push(mat_idx.unwrap_or(SLAB_TRI_NONE));
                if let (Some(sink), Some(uv)) = (uv_out.as_deref_mut(), uvs.as_ref()) {
                    for &vi in t3 {
                        sink.push(uv[vi as usize][0]);
                        sink.push(uv[vi as usize][1]);
                    }
                }
                // D2：逐顶点法线 → 世界旋转 3×3（xform_dir）+ norm3 归一化 →
                // sink（9 f32/tri，顶点序与 tris 同源；off 面不写）。
                if let (Some(sink), Some(nrm)) = (nrm_out.as_deref_mut(), nrms.as_ref()) {
                    for &vi in t3 {
                        let nn = norm3(xform_dir(&w, nrm[vi as usize]));
                        sink.push(nn[0]);
                        sink.push(nn[1]);
                        sink.push(nn[2]);
                    }
                }
                // D6：逐三角 [metallic, roughness] → sink（2 f32/tri，与 tris
                // 同序同源；matless = [0,1] 介质保守缺省；off 面不写）。
                if let Some(sink) = mr_out.as_deref_mut() {
                    let (mt, rg) = match mat_idx.and_then(|mi| mats.get(mi as usize)) {
                        Some(rec) => (rec.metallic, rec.roughness),
                        None => (0.0, 1.0),
                    };
                    sink.push(mt);
                    sink.push(rg);
                }
                if emi != [0.0, 0.0, 0.0] {
                    emissive_tris += 1;
                }
            }
        }
        // Task B1：节点段闭合即登记分组（世界 AABB = 本段烘焙顶点精确 min/max）。
        if let Some(gs) = groups.as_deref_mut() {
            let node_tri_end = indices.len();
            if node_tri_end > node_tri_start {
                let mut lo = [f32::INFINITY; 3];
                let mut hi = [f32::NEG_INFINITY; 3];
                for p in &positions[node_tri_start * 3..node_tri_end * 3] {
                    for k in 0..3 {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
                gs.push(SceneNodeGroup {
                    tri_offset: node_tri_start as u32,
                    tri_count: (node_tri_end - node_tri_start) as u32,
                    aabb_min: lo,
                    aabb_max: hi,
                });
            }
        }
    }
    if indices.is_empty() {
        return Err(format!("场景 {scene_id} 装配零三角"));
    }

    // 契约 quad 面光（照明面 + 发光三角几何逐字一致追加，G12.4 同律——
    // 主命中可见灯面；阴影射线以 t_max 缩短排除目标灯面自遮蔽）。
    let mut quads: Vec<QuadLight> = Vec::new();
    // Task B1：quad 面光几何尾段起点（尾段自立一组;灯面为普通发光几何,
    // 剔除语义与节点组同律——照明贡献走 quads/points SSBO 参数面,与几何
    // 剔除正交;bistro quads=0 ⇒ 空尾段不产组）。
    let quad_tail_start = indices.len();
    for q in lig
        .get("quad_lights")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let f3 = |k: &str| -> Result<[f32; 3], String> {
            Ok(f64v3(&as_f64v(k, q.get(k).unwrap(), 3)?))
        };
        let (p00, e1, e2, le) = (f3("p00")?, f3("e1")?, f3("e2")?, f3("le_linear_rgb")?);
        quads.push(QuadLight { p00, e1, e2, le });
        let p10 = [p00[0] + e1[0], p00[1] + e1[1], p00[2] + e1[2]];
        let p01 = [p00[0] + e2[0], p00[1] + e2[1], p00[2] + e2[2]];
        let p11 = [p00[0] + e1[0] + e2[0], p00[1] + e1[1] + e2[1], p00[2] + e1[2] + e2[2]];
        for (a, b, c) in [(p00, p10, p11), (p00, p11, p01)] {
            let bidx = positions.len() as u32;
            positions.push(a);
            positions.push(b);
            positions.push(c);
            indices.push([bidx, bidx + 1, bidx + 2]);
            albedo.push([0.5, 0.5, 0.5]);
            emission.push(le);
            tri_mat.push(SLAB_TRI_NONE);
            // Task B4：quad 灯面尾段 UV 恒 0（tritex −1 不消费面）。
            if let Some(sink) = uv_out.as_deref_mut() {
                sink.extend_from_slice(&[0.0; 6]);
            }
            // D2：quad 灯面尾段法线恒 0（UV 同律；bistro quads=0 ⇒ 生产臂零
            // 触达；cornell Split 形态与本臂 CLI 互斥——kernel 侧零法线经
            // gate_sl 门恒 0 有限值，无 NaN 通道）。
            if let Some(sink) = nrm_out.as_deref_mut() {
                sink.extend_from_slice(&[0.0; 9]);
            }
            // D6：quad 灯面尾段 MR 恒 0（法线同律；灯面为自发光几何，高光
            // 臂对其零增益语义——metal=0/rough=0 经 kernel 钳 [0.05,1] 后
            // F0=0.04 锐高光，但发射项主导，如实登记）。
            if let Some(sink) = mr_out.as_deref_mut() {
                sink.extend_from_slice(&[0.0; 2]);
            }
        }
    }
    // Task B1：quad 尾段闭合即登记（零三角形尾段不产组）。
    if let Some(gs) = groups.as_deref_mut() {
        let tail_end = indices.len();
        if tail_end > quad_tail_start {
            let mut lo = [f32::INFINITY; 3];
            let mut hi = [f32::NEG_INFINITY; 3];
            for p in &positions[quad_tail_start * 3..tail_end * 3] {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
            gs.push(SceneNodeGroup {
                tri_offset: quad_tail_start as u32,
                tri_count: (tail_end - quad_tail_start) as u32,
                aabb_min: lo,
                aabb_max: hi,
            });
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
            // A1：契约灯半径恒 0.0（pack 槽 7 字节位级不变——冻结锚零漂移）。
            radius: 0.0,
        });
    }

    // 相机（契约四元数 → look_at 同口径：forward = q·(0,0,−1)、up = q·(0,1,0)；
    // right = forward × up0（UE 一致手性，G12.4 波裁决实证面））。
    let pos = as_f64v("camera.position", cam.get("position").unwrap(), 3)?;
    let quat = as_f64v("camera.orientation_quat", cam.get("orientation_quat").unwrap(), 4)?;
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
        tri_mat,
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
// 帧数学（未抖/jittered view-proj 与场景 eps；G14.3 主射线求交在 device
// kernel——host 侧仅矩阵/eps 面，与 g13_4 同模）
// G14.3：g13_4 同型复制子集（bin-local 惯例）
// ---------------------------------------------------------------------------

const INV_PI: f32 = 1.0 / std::f32::consts::PI;

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
// G14.3 新面：device 持久车道场景打包（AS 三角形汤 + 逐三角材质 + 灯面 +
// 逐帧参数 SSBO；与 kernels/g14_3_direct_gi.rx 参数面逐字同源）
// ---------------------------------------------------------------------------

/// 三角形汤扁平化（9 f32/tri 世界空间；同一 Vec 既喂 BLAS 建面又喂 tris
/// SSBO——AS 几何与 kernel 侧逐三角数据由构造同源，零漂移面）。
fn pack_tris(scene: &SceneData) -> Vec<f32> {
    let mut out = Vec::with_capacity(scene.indices.len() * 9);
    for t in &scene.indices {
        for &vi in t {
            let p = scene.positions[vi as usize];
            out.push(p[0]);
            out.push(p[1]);
            out.push(p[2]);
        }
    }
    out
}

/// 逐三角材质（8 f32/tri：[albedo 3, emission 3, 0, 0]）。
fn pack_mats(scene: &SceneData) -> Vec<f32> {
    let mut out = Vec::with_capacity(scene.indices.len() * 8);
    for k in 0..scene.indices.len() {
        out.extend_from_slice(&scene.albedo[k]);
        out.extend_from_slice(&scene.emission[k]);
        out.push(0.0);
        out.push(0.0);
    }
    out
}

/// quad 灯面（16 f32/quad：[p00 3, e1 3, e2 3, le 3, area, qn 3]；area/qn
/// host f32 预算——cross/normalize 操作序与 g13_4 shade_pixel 逐字同式
/// （bin norm3 除法口径），kernel 内零重算位级漂移）。
fn pack_quads(scene: &SceneData) -> Vec<f32> {
    let mut out = Vec::with_capacity(scene.quads.len().max(1) * 16);
    for q in &scene.quads {
        let c = [
            q.e1[1] * q.e2[2] - q.e1[2] * q.e2[1],
            q.e1[2] * q.e2[0] - q.e1[0] * q.e2[2],
            q.e1[0] * q.e2[1] - q.e1[1] * q.e2[0],
        ];
        let qn = norm3(c);
        let area = dot3(c, c).sqrt();
        out.extend_from_slice(&q.p00);
        out.extend_from_slice(&q.e1);
        out.extend_from_slice(&q.e2);
        out.extend_from_slice(&q.le);
        out.push(area);
        out.extend_from_slice(&qn);
    }
    if scene.quads.is_empty() {
        out.push(0.0); // 空集哑元（VUID ≥4B；kernel 以 quad_count=0 门不消费）
    }
    out
}

/// point 灯面（8 f32/point：[pos 3, intensity 3, radius, 0]）。A1：第 7 槽
/// 由零填充改写 radius（契约灯 radius=0.0 ⇒ 字节位级不变；仅 --lamp-lights
/// on 追加的提取灯 >0——g18_smooth_nrm kernel 阴影 t_sh 截断消费，母版
/// kernel 不读该槽 0-byte）。
fn pack_points(scene: &SceneData) -> Vec<f32> {
    let mut out = Vec::with_capacity(scene.points.len().max(1) * 8);
    for p in &scene.points {
        out.extend_from_slice(&p.pos);
        out.extend_from_slice(&p.intensity);
        out.push(p.radius);
        out.push(0.0);
    }
    if scene.points.is_empty() {
        out.push(0.0); // 空集哑元（同上）
    }
    out
}

// ---------------------------------------------------------------------------
// 画质战役 A1 灯光提取加性臂（--lamp-lights on；off 默认 = 以下全部零触达）
//
// 动机：bistro 44,024 自发光灯片三角不投光（无 emissive NEE，夜航 SUMMARY
// 根因 #2），仅 4 盏契约点光照明 → 生产渲染死黑+欠曝。本臂 host 侧把
// emissive 三角确定性聚类成 ≤K 个代表点光 append 进 scene.points（kernel
// 侧 = g18_smooth_nrm 点光循环既有面 + 半径阴影截断/贡献剔除两加性门），
// 让灯具真正投光。默认 off = scene 装配/pack/参数面全 0-byte。
// ---------------------------------------------------------------------------

/// A1 CLI 参数面（bench/render 双腿 + 窗口车道共用；enabled=false = 全默认
/// 面零触达）。
struct LampOpt {
    enabled: bool,
    /// 灯强度增益（I_c = Φ_c·gain/(4π)；默认 1.0——物理通量直转换）。
    gain: f32,
    /// 代表点光上限 K（按峰值通量降序取 top-K，弃簇如实登记；默认 12）。
    max_k: usize,
    /// kernel params[49] 贡献剔除阈值（默认 0.0 = 全保留）。
    contrib: f32,
    /// 提取统计 JSON 落盘路径（空 = 不写；bench bin --lamp-stats-out）。
    stats_out: String,
}

impl LampOpt {
    fn off() -> Self {
        LampOpt {
            enabled: false,
            gain: 1.0,
            max_k: 12,
            contrib: 0.0,
            stats_out: String::new(),
        }
    }
}

/// day_0828 Phase C GI2 CLI 参数面（bench/render 双腿共用；enabled=false =
/// 全默认面零触达——MegaTexNrmGi2 形态/哑表五件/params[51..55) 全不创建）。
#[allow(dead_code)] // Phase C:g14_3_pipeline_perf 独消费面(诚实标注)
struct Gi2Opt {
    enabled: bool,
    /// GI 合成尺度（params[54]；默认 1.0——物理 1 反弹直传）。
    scale: f32,
    /// firefly 逐通道 clamp（params[53]；默认 4.0）。
    clamp: f32,
}

#[allow(dead_code)] // Phase C:同上
impl Gi2Opt {
    fn off() -> Self {
        Gi2Opt {
            enabled: false,
            scale: 1.0,
            clamp: 4.0,
        }
    }
}

/// day_0828 Phase D TSR 降噪质量档 CLI 参数面（bench/render 双腿共用;
/// enabled=false = 全默认面零触达——resolve SPV 不换载、tsr_params[19..21)
/// 不写〔与零填充逐位同值〕）。
#[allow(dead_code)] // Phase D:g14_3_pipeline_perf 独消费面(诚实标注)
struct TsrqOpt {
    enabled: bool,
    /// 稳态 alpha 档（tsr_params[19] 直入公式 base 位;默认 0.02——母版稳态
    /// 实测 0.1〔min_alpha 地板不可达〕,静态驻态残差 ∝ √(α/(2−α)) 按档兑现）。
    min_alpha: f32,
    /// 3×3 邻域亮度 clamp 系数 K（tsr_params[20];0 = 关,评估臂默认）。
    clamp: f32,
}

#[allow(dead_code)] // Phase D:同上
impl TsrqOpt {
    fn off() -> Self {
        TsrqOpt {
            enabled: false,
            min_alpha: 0.02,
            clamp: 0.0,
        }
    }
}

/// A1 提取簇统计（登记面；kept 序 = 峰值通量降序）。
struct LampCluster {
    pos: [f32; 3],
    flux: [f32; 3],
    radius: f32,
    tris: usize,
}

/// A1 提取统计（evidence 登记面）。
struct LampExtractStats {
    emissive_tris: usize,
    clusters_total: usize,
    clusters_dropped: usize,
    dropped_tris: usize,
    /// 弃簇峰值通量最大值（如实登记截断损失上界；无弃簇 = 0.0）。
    dropped_flux_max: f32,
    kept: Vec<LampCluster>,
}

/// A1 灯光提取（纯 host、全确定性——BTreeMap 键序迭代 + 固定邻域序 +
/// 升序三角并入，双跑位级同产物）：
/// ① 扫 emission 任一通道 >0 且 tri_mat ≠ SLAB_TRI_NONE（排除 quad 灯尾段）
///    的三角：面积 = 0.5·|cross(e1,e2)|、质心 = 顶点均值、通量 Φ_c =
///    π·Le_c·area（Lambert 单面发射体）。
/// ② 质心量化 0.6m 网格（floor 键 (ix,iy,iz)）→ 26 邻域 union-find 合并
///    相邻非空格（min-root 规约）——一盏灯跨格合一。
/// ③ 每簇：峰值通量加权质心 pos（权 = max3(Φ)，全零权簇退化为算术均值）、
///    Φ_total 逐通道求和、radius = 成员三角顶点到 pos 最大距离 + 0.02m。
/// ④ 按 max(Φ_r,Φ_g,Φ_b) 降序（并列按质心 x,y,z 字典序）取 top-max_k，
///    弃簇计数/峰值如实登记。灯强度 I_c = Φ_c·gain/(4π)。
fn extract_lamp_lights(
    scene: &SceneData,
    max_k: usize,
    gain: f32,
) -> (Vec<PointLight>, LampExtractStats) {
    // G38 T5:聚类网格边长 env 散臂旋钮(RURIX_G18_AMBIENT 同律:缺席 = 0.6 字面
    // ⇒ 锚面零漂移;在位 = parse f32,非法即 fail 不静默)。lamp-k 阶梯消费
    // (EVAL_RESTIR §9.4「改参数不改算法」——0.6m 网格 bistro 仅 13 簇,提 K 须收细);
    // 改默认字面 = full 语义变更即重锚,归阶梯判 GO 后的重锚窗。
    const GRID_M_DEFAULT: f32 = 0.6;
    let grid_m: f32 = match std::env::var("RURIX_G31_LAMP_GRID_M") {
        Ok(s) => s.trim().parse().unwrap_or_else(|_| {
            fail(&format!("RURIX_G31_LAMP_GRID_M 非 f32 字面: {s}(fail-closed)"))
        }),
        Err(_) => GRID_M_DEFAULT,
    };
    const RADIUS_PAD_M: f32 = 0.02;
    // ① emissive 三角扫描（升序三角号——后续全链迭代序确定性根）。
    struct EmTri {
        centroid: [f32; 3],
        flux: [f32; 3],
        v: [[f32; 3]; 3],
    }
    let mut em: Vec<(usize, EmTri)> = Vec::new();
    for (k, le) in scene.emission.iter().enumerate() {
        if (le[0] > 0.0 || le[1] > 0.0 || le[2] > 0.0) && scene.tri_mat[k] != SLAB_TRI_NONE {
            let idx = scene.indices[k];
            let v0 = scene.positions[idx[0] as usize];
            let v1 = scene.positions[idx[1] as usize];
            let v2 = scene.positions[idx[2] as usize];
            let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let cx = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let area = 0.5 * (cx[0] * cx[0] + cx[1] * cx[1] + cx[2] * cx[2]).sqrt();
            let c = [
                (v0[0] + v1[0] + v2[0]) / 3.0,
                (v0[1] + v1[1] + v2[1]) / 3.0,
                (v0[2] + v1[2] + v2[2]) / 3.0,
            ];
            em.push((
                k,
                EmTri {
                    centroid: c,
                    flux: [
                        std::f32::consts::PI * le[0] * area,
                        std::f32::consts::PI * le[1] * area,
                        std::f32::consts::PI * le[2] * area,
                    ],
                    v: [v0, v1, v2],
                },
            ));
        }
    }
    // ② 网格量化（BTreeMap 键序 = 确定性迭代序）+ 26 邻域 union-find。
    let mut cells: std::collections::BTreeMap<(i64, i64, i64), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (ei, (_, t)) in em.iter().enumerate() {
        let key = (
            (t.centroid[0] / grid_m).floor() as i64,
            (t.centroid[1] / grid_m).floor() as i64,
            (t.centroid[2] / grid_m).floor() as i64,
        );
        cells.entry(key).or_default().push(ei);
    }
    let keys: Vec<(i64, i64, i64)> = cells.keys().copied().collect();
    let key_index: std::collections::BTreeMap<(i64, i64, i64), usize> =
        keys.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let mut parent: Vec<usize> = (0..keys.len()).collect();
    fn find(parent: &mut Vec<usize>, mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for (ci, key) in keys.iter().enumerate() {
        for dz in -1i64..=1 {
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    if let Some(&cj) = key_index.get(&(key.0 + dx, key.1 + dy, key.2 + dz)) {
                        let (ra, rb) = (find(&mut parent, ci), find(&mut parent, cj));
                        if ra != rb {
                            // min-root 规约（并向小根——确定性规范形）。
                            let (lo, hi) = (ra.min(rb), ra.max(rb));
                            parent[hi] = lo;
                        }
                    }
                }
            }
        }
    }
    // 簇聚合（root 键序迭代；簇内成员 = 格序×格内升序拼接——确定性求和序）。
    let mut clusters: std::collections::BTreeMap<usize, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (ci, key) in keys.iter().enumerate() {
        let root = find(&mut parent, ci);
        clusters
            .entry(root)
            .or_default()
            .extend(cells.get(key).unwrap().iter().copied());
    }
    // ③ 逐簇统计。
    struct RawCluster {
        pos: [f32; 3],
        flux: [f32; 3],
        radius: f32,
        tris: usize,
    }
    let mut raw: Vec<RawCluster> = Vec::new();
    for members in clusters.values() {
        let mut flux = [0.0f32; 3];
        let mut wsum = 0.0f32;
        let mut wc = [0.0f32; 3];
        let mut csum = [0.0f32; 3];
        for &ei in members {
            let t = &em[ei].1;
            flux[0] += t.flux[0];
            flux[1] += t.flux[1];
            flux[2] += t.flux[2];
            let w = t.flux[0].max(t.flux[1]).max(t.flux[2]);
            wsum += w;
            for c in 0..3 {
                wc[c] += w * t.centroid[c];
                csum[c] += t.centroid[c];
            }
        }
        let n = members.len() as f32;
        let pos = if wsum > 0.0 {
            [wc[0] / wsum, wc[1] / wsum, wc[2] / wsum]
        } else {
            // 全零权（退化零面积簇）：算术均值保底——通量 0 排序垫底不入选。
            [csum[0] / n, csum[1] / n, csum[2] / n]
        };
        let mut r2max = 0.0f32;
        for &ei in members {
            for v in &em[ei].1.v {
                let d2 = (v[0] - pos[0]) * (v[0] - pos[0])
                    + (v[1] - pos[1]) * (v[1] - pos[1])
                    + (v[2] - pos[2]) * (v[2] - pos[2]);
                r2max = r2max.max(d2);
            }
        }
        raw.push(RawCluster {
            pos,
            flux,
            radius: r2max.sqrt() + RADIUS_PAD_M,
            tris: members.len(),
        });
    }
    // ④ 峰值通量降序（并列按质心字典序——total_cmp 全序确定性）。
    raw.sort_by(|a, b| {
        let fa = a.flux[0].max(a.flux[1]).max(a.flux[2]);
        let fb = b.flux[0].max(b.flux[1]).max(b.flux[2]);
        fb.total_cmp(&fa)
            .then(a.pos[0].total_cmp(&b.pos[0]))
            .then(a.pos[1].total_cmp(&b.pos[1]))
            .then(a.pos[2].total_cmp(&b.pos[2]))
    });
    let keep_n = raw.len().min(max_k);
    let inv_4pi = 1.0f32 / (4.0 * std::f32::consts::PI);
    let mut lights: Vec<PointLight> = Vec::new();
    let mut kept: Vec<LampCluster> = Vec::new();
    for rc in raw.iter().take(keep_n) {
        lights.push(PointLight {
            pos: rc.pos,
            intensity: [
                rc.flux[0] * gain * inv_4pi,
                rc.flux[1] * gain * inv_4pi,
                rc.flux[2] * gain * inv_4pi,
            ],
            radius: rc.radius,
        });
        kept.push(LampCluster {
            pos: rc.pos,
            flux: rc.flux,
            radius: rc.radius,
            tris: rc.tris,
        });
    }
    let mut dropped_flux_max = 0.0f32;
    let mut dropped_tris = 0usize;
    for rc in raw.iter().skip(keep_n) {
        dropped_flux_max = dropped_flux_max.max(rc.flux[0].max(rc.flux[1]).max(rc.flux[2]));
        dropped_tris += rc.tris;
    }
    (
        lights,
        LampExtractStats {
            emissive_tris: em.len(),
            clusters_total: raw.len(),
            clusters_dropped: raw.len() - keep_n,
            dropped_tris,
            dropped_flux_max,
            kept,
        },
    )
}

/// A1 施加点（仅 --lamp-lights on 调用——off 路径零触达）：提取 → 统计
/// eprintln + 可选 JSON 落盘 → append 进 scene.points（point_count 参数/
/// pack_points 面自动随 len 进制）。
fn apply_lamp_lights(mut scene: SceneData, opt: &LampOpt) -> SceneData {
    let (lights, stats) = extract_lamp_lights(&scene, opt.max_k, opt.gain);
    eprintln!(
        "{TAG}: lamp-lights 提取 emissive_tris={} clusters={} kept={} dropped={}（弃簇通量峰 {:.6}/弃三角 {}）gain={} k={} contrib={}",
        stats.emissive_tris,
        stats.clusters_total,
        stats.kept.len(),
        stats.clusters_dropped,
        stats.dropped_flux_max,
        stats.dropped_tris,
        opt.gain,
        opt.max_k,
        opt.contrib,
    );
    for (i, c) in stats.kept.iter().enumerate() {
        eprintln!(
            "{TAG}: lamp[{i}] pos=({:.3},{:.3},{:.3}) flux=({:.5},{:.5},{:.5}) radius={:.3} tris={}",
            c.pos[0], c.pos[1], c.pos[2], c.flux[0], c.flux[1], c.flux[2], c.radius, c.tris,
        );
    }
    if !opt.stats_out.is_empty() {
        let mut kept_json = String::new();
        for (i, c) in stats.kept.iter().enumerate() {
            if i > 0 {
                kept_json.push(',');
            }
            let l = &lights[i];
            kept_json.push_str(&format!(
                "{{\"pos\":[{},{},{}],\"flux\":[{},{},{}],\"radius\":{},\"tris\":{},\"intensity\":[{},{},{}]}}",
                c.pos[0], c.pos[1], c.pos[2], c.flux[0], c.flux[1], c.flux[2], c.radius, c.tris,
                l.intensity[0], l.intensity[1], l.intensity[2],
            ));
        }
        let json = format!(
            "{{\"schema\":\"rurix.a1.lamp_lights.extract.v1\",\"grid_cell_m\":0.6,\"radius_pad_m\":0.02,\"gain\":{},\"max_k\":{},\"contrib_threshold\":{},\"emissive_tris\":{},\"clusters_total\":{},\"clusters_kept\":{},\"clusters_dropped\":{},\"dropped_tris\":{},\"dropped_flux_max\":{},\"kept\":[{}]}}",
            opt.gain,
            opt.max_k,
            opt.contrib,
            stats.emissive_tris,
            stats.clusters_total,
            stats.kept.len(),
            stats.clusters_dropped,
            stats.dropped_tris,
            stats.dropped_flux_max,
            kept_json,
        );
        if let Some(parent) = Path::new(&opt.stats_out).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        std::fs::write(&opt.stats_out, json)
            .unwrap_or_else(|e| fail(&format!("lamp-stats-out 写入: {e}")));
    }
    scene.points.extend(lights);
    scene
}

// ---------------------------------------------------------------------------
// G31+ #58 簇 DAG LOD 生产接线（--cluster-lod 加性面；off 默认 = 既有面 0-byte）
//
// 三步文件交接（crate 依赖方向约束：rurix-geom-build → rurix-render，本 bin
// 不能反向依赖离线构建器——DAG 构建经 rurix-asset 侧 g31_cluster_lod_bake 离线
// 完成，本 bin 只 dump 装配产物与消费簇包）：
//   1) `--dump-scene`（本 bin 子模式）：装配产物（世界空间三角汤 + 逐三角属性
//      + 节点段表）落 RXCS v1 —— 装配语义单源（bake 侧禁复刻装配，位级同源）。
//   2) `g31_cluster_lod_bake`（rurix-asset bin）：RXCS → 分块焊接 →
//      `build_asset_dag`（事实源构建器）→ 逐簇继承属性 → RXCP v1 簇包。
//   3) `--cluster-lod leaf|on --cluster-pack <RXCP>`（本 bin bench/render）：
//      读簇包 → fail-closed 校验（gltf sha / 叶覆盖恰一次 / 叶几何逐位 == 源）
//      → 逐块当帧误差 cut（`select_lod_cut` + `verify_cut_coverage` 生产金标准
//      直调，禁旁路重算）→ cut 三角集重建 SceneData → 既有单 BLAS 车道出帧。
//
// 对拍锚：`leaf` 模式（threshold→0 极限 = 全叶层）重建产物与 off 三角汤
// **逐位一致**（本函数内 fail-closed 断言）⇒ digest 锚零漂移可机核。
// `on` 模式 = 屏幕误差驱动 cut（远景三角数下降 measured 进 evidence，不进硬门）。
//
// 边界（诚实登记）：emissive 三角与 quad 灯面尾段恒 passthrough 不参与 LOD
// （光源几何面 0-byte——emissive 主命中/灯采样语义不受 cut 影响）；粗簇三角
// 属性 = 叶后代面积加权均值（bake 期预烘焙），远景块状色斑属可接受近似；
// 本波 cut 冻结于装配相机（bench 契约相机 = 生产口径），逐帧 AS 更新归
// C/E 阶段（TODO #77/#20–23）。
// ---------------------------------------------------------------------------

/// RXCS 场景 dump magic（bin-local 交接格式，非冻结格式栈;v1 = 无 UV 段,
/// v2 = 尾部追加 6 f32/tri corner UV——G31+ #96 属性保持简化 bake 输入面）。
const RXCS_MAGIC: &[u8; 4] = b"RXCS";
/// RXCP 簇包 magic（bin-local 交接格式;v1 = 无簇 UV,v2 = 逐块顶点 UV
/// 平行表——#96 属性臂）。
const RXCP_MAGIC: &[u8; 4] = b"RXCP";

/// 装配场景 dump（RXCS v1|v2）：tri 汤（9 f32/tri 位保真）+ 逐三角属性 +
/// 节点段表（is_light_tail = quad 灯面尾段标记）。bake 侧唯一装配输入面。
/// G31+ #96：`tri_uv = Some`（装配 UV sink,6 f32/tri 与 tris 同序）⇒ v2,
/// 尾部追加 UV 段位保真;`None` ⇒ v1 字节面逐位不变（无 UV 资产臂逃生口）。
#[allow(dead_code)] // G31+ #58：g14_3 --dump-scene 消费面（g31/g34 include 共享体，诚实标注）
fn dump_scene_rxcs(
    scene: &SceneData,
    groups: &[SceneNodeGroup],
    tri_uv: Option<&[f32]>,
    path: &Path,
) -> Result<(), String> {
    let n = scene.indices.len();
    if let Some(uv) = tri_uv
        && uv.len() != n * 6
    {
        return Err(format!("UV sink 长度 {} ≠ 三角数×6 {}", uv.len(), n * 6));
    }
    let quad_tail_tris = scene.quads.len() * 2;
    let version: u32 = if tri_uv.is_some() { 2 } else { 1 };
    let mut out: Vec<u8> = Vec::with_capacity(64 + n * (36 + 12 + 12 + 4));
    out.extend_from_slice(RXCS_MAGIC);
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&(n as u32).to_le_bytes());
    out.extend_from_slice(&(groups.len() as u32).to_le_bytes());
    if scene.gltf_sha256.len() != 64 {
        return Err("gltf_sha256 非 64 hex".into());
    }
    out.extend_from_slice(scene.gltf_sha256.as_bytes());
    for g in groups {
        let is_tail = quad_tail_tris > 0 && g.tri_offset as usize >= n - quad_tail_tris;
        out.extend_from_slice(&g.tri_offset.to_le_bytes());
        out.extend_from_slice(&g.tri_count.to_le_bytes());
        out.extend_from_slice(&u32::from(is_tail).to_le_bytes());
    }
    for t in &scene.indices {
        for &vi in t {
            for &x in &scene.positions[vi as usize] {
                out.extend_from_slice(&x.to_bits().to_le_bytes());
            }
        }
    }
    for a in &scene.albedo {
        for &x in a {
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
    }
    for e in &scene.emission {
        for &x in e {
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
    }
    for &m in &scene.tri_mat {
        out.extend_from_slice(&m.to_le_bytes());
    }
    // #96 RXCS v2:UV 段尾接(6 f32/tri 与 tris 同序,装配 sink 位保真;
    // quad 灯面尾段恒 0 由装配面保证)。
    if let Some(uv) = tri_uv {
        for &x in uv {
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
    }
    std::fs::write(path, &out).map_err(|e| format!("RXCS 写盘失败 {path:?}: {e}"))
}

/// 簇包块（单 DAG：装配段合并块的簇层级 + 几何段 + 叶源三角映射 + 继承属性）。
#[allow(dead_code)]
struct ClusterPackBlock {
    /// 冻结契约 64B 簇记录（bake 侧 `build_asset_dag` 产物字段逐位透传）。
    records: Vec<ClusterRecord>,
    /// 运行时拓扑（`MeshDagView` 输入面；bake 侧 group 字段读后即弃——
    /// 运行时 cut 判定不消费组号）。
    nodes: Vec<DagNodeRec>,
    /// 扁平子簇索引（块内局部号）。
    children: Vec<u32>,
    /// 簇局部顶点池（`ClusterRecord::vertex_offset` 元素计；f32 位保真）。
    vertices: Vec<[f32; 3]>,
    /// 3×u8/三角形局部索引（`ClusterRecord::triangle_offset` u8 元素计）。
    triangle_indices: Vec<u8>,
    /// 叶层三角形 → 全局源三角 id（叶簇第 t 个三角 = `leaf_source_tris[
    /// record.triangle_offset/3 + t]`；bake 侧自 `ClusterDag::leaf_source_tris`
    /// 经块内→全局重映射）。
    leaf_source_tris: Vec<u32>,
    /// 逐簇继承属性（粗簇三角消费面：albedo 3 + emission 3；bake 期叶后代
    /// 面积加权均值）。
    cluster_albedo: Vec<[f32; 3]>,
    cluster_emission: Vec<[f32; 3]>,
    /// 逐簇继承材质 id（叶后代三角数众数；SLAB_TRI_NONE = 无材质）。
    cluster_mat: Vec<u32>,
    /// 逐簇组共享 LOD 判定球（#58/B4 Nanite 语义：self = 生成组球——同组
    /// 产物逐位共享;`select_lod_cut_grouped` 消费面）。
    cluster_self_lod: Vec<LodBounds>,
    /// 逐簇组共享 LOD 判定球（parent = 所属组球——与该组产物 self 判定
    /// 逐位同源,组内判定必然一致）。
    cluster_parent_lod: Vec<LodBounds>,
}

/// 簇包（RXCP v1|v2 内存形态）。
#[allow(dead_code)]
struct ClusterPack {
    gltf_sha256: String,
    /// 源三角总数（覆盖性校验面）。
    src_tri_count: u32,
    /// 恒 passthrough 源三角（emissive + quad 灯面尾段 + 病态小块；升序）。
    passthrough: Vec<u32>,
    blocks: Vec<ClusterPackBlock>,
    /// G31+ #96 RXCP v2:与 `blocks` 平行的逐块顶点 UV 表(每块与该块
    /// vertices 等长平行;簇局部切片 = `records[id].vertex_offset..
    /// +vertex_count`,与顶点切片同口径——粗簇代理三角 corner UV 事实源,
    /// `gather_tri_uv_attrs` 消费)。v1 = None(无 UV 资产臂,gather 回落
    /// [0;6] + tritex 补丁维持 −1 常量回退)。挂 pack 级而非 block 级 =
    /// ClusterPackBlock 字面构造面(frame_cut selftest 夹具)0 改动。
    blocks_vertex_uv: Option<Vec<Vec<[f32; 2]>>>,
}

/// LE 读取小游标（bin-local；越界 = typed Err，fail-closed）。
#[allow(dead_code)]
struct PackCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

#[allow(dead_code)]
impl<'a> PackCursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.pos + n > self.bytes.len() {
            return Err(format!("RXCP 截断（need {n} at {}）", self.pos));
        }
        let s = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn f32x3(&mut self) -> Result<[f32; 3], String> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }
}

/// RXCP v1|v2 簇包读取（fail-closed 边界全校验；布局 = g31_cluster_lod_bake
/// writer 逐字段镜像——两端同源字段序，破坏即 typed Err）。
/// G31+ #96：v2 = 逐块 vertices 段后追加顶点 UV 平行表(2 f32/顶点);
/// v1 输入路径行为逐位不变(cluster_vertex_uv = None)。
#[allow(dead_code)]
fn read_cluster_pack(path: &Path) -> Result<ClusterPack, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("RXCP 读取失败 {path:?}: {e}"))?;
    let mut c = PackCursor {
        bytes: &bytes,
        pos: 0,
    };
    if c.take(4)? != RXCP_MAGIC {
        return Err("RXCP magic 不符".into());
    }
    let version = c.u32()?;
    if version != 1 && version != 2 {
        return Err(format!("RXCP 版本不支持: {version}"));
    }
    let sha = String::from_utf8(c.take(64)?.to_vec()).map_err(|_| "sha 非 utf8".to_string())?;
    let src_tri_count = c.u32()?;
    let pass_n = c.u32()? as usize;
    let mut passthrough = Vec::with_capacity(pass_n);
    for _ in 0..pass_n {
        passthrough.push(c.u32()?);
    }
    let block_n = c.u32()? as usize;
    let mut blocks = Vec::with_capacity(block_n);
    // #96 RXCP v2:逐块顶点 UV 表(与 blocks 平行收集;v1 恒空)。
    let mut blocks_uv: Vec<Vec<[f32; 2]>> = Vec::with_capacity(block_n);
    for _ in 0..block_n {
        let rec_n = c.u32()? as usize;
        let child_n = c.u32()? as usize;
        let vert_n = c.u32()? as usize;
        let tri_idx_n = c.u32()? as usize;
        let leaf_tri_n = c.u32()? as usize;
        let mut records = Vec::with_capacity(rec_n);
        for _ in 0..rec_n {
            records.push(ClusterRecord {
                center: c.f32x3()?,
                radius: c.f32()?,
                cone_axis: c.f32x3()?,
                cone_cutoff: c.f32()?,
                error: c.f32()?,
                parent_error: c.f32()?,
                vertex_offset: c.u32()?,
                triangle_offset: c.u32()?,
                vertex_count: c.u32()?,
                triangle_count: c.u32()?,
                page_id: c.u32()?,
                reserved: c.u32()?,
            });
        }
        let mut nodes = Vec::with_capacity(rec_n);
        for _ in 0..rec_n {
            let first_child = c.u32()?;
            let child_count = c.u32()?;
            let level = c.u32()?;
            let _group = c.u32()?; // 运行时 cut 不消费组号（读后即弃）
            nodes.push(DagNodeRec {
                first_child,
                child_count,
                level,
            });
        }
        let mut children = Vec::with_capacity(child_n);
        for _ in 0..child_n {
            children.push(c.u32()?);
        }
        let mut vertices = Vec::with_capacity(vert_n);
        for _ in 0..vert_n {
            vertices.push(c.f32x3()?);
        }
        // #96 RXCP v2:顶点 UV 平行表(writer 在 vertices 段后紧邻写出)。
        if version == 2 {
            let mut uv = Vec::with_capacity(vert_n);
            for _ in 0..vert_n {
                uv.push([c.f32()?, c.f32()?]);
            }
            blocks_uv.push(uv);
        }
        let tri_bytes = c.take(tri_idx_n)?.to_vec();
        let pad = (4 - tri_idx_n % 4) % 4;
        let _ = c.take(pad)?;
        let mut leaf_source_tris = Vec::with_capacity(leaf_tri_n);
        for _ in 0..leaf_tri_n {
            leaf_source_tris.push(c.u32()?);
        }
        let mut cluster_albedo = Vec::with_capacity(rec_n);
        let mut cluster_emission = Vec::with_capacity(rec_n);
        let mut cluster_mat = Vec::with_capacity(rec_n);
        let mut cluster_self_lod = Vec::with_capacity(rec_n);
        let mut cluster_parent_lod = Vec::with_capacity(rec_n);
        for _ in 0..rec_n {
            cluster_albedo.push(c.f32x3()?);
            cluster_emission.push(c.f32x3()?);
            cluster_mat.push(c.u32()?);
            let _pad = c.u32()?;
            cluster_self_lod.push(LodBounds {
                center: c.f32x3()?,
                radius: c.f32()?,
            });
            cluster_parent_lod.push(LodBounds {
                center: c.f32x3()?,
                radius: c.f32()?,
            });
        }
        blocks.push(ClusterPackBlock {
            records,
            nodes,
            children,
            vertices,
            triangle_indices: tri_bytes,
            leaf_source_tris,
            cluster_albedo,
            cluster_emission,
            cluster_mat,
            cluster_self_lod,
            cluster_parent_lod,
        });
    }
    if c.pos != bytes.len() {
        return Err(format!(
            "RXCP 尾部冗余字节（pos {} ≠ len {}）",
            c.pos,
            bytes.len()
        ));
    }
    Ok(ClusterPack {
        gltf_sha256: sha,
        src_tri_count,
        passthrough,
        blocks,
        blocks_vertex_uv: (version == 2).then_some(blocks_uv),
    })
}

/// --cluster-lod 模式闭集。
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClusterLodMode {
    /// 默认：不触簇包，既有面 0-byte（Stage A 锚零漂移）。
    Off,
    /// 全叶层对拍锚（threshold→0 极限）：重建产物与 off 三角汤逐位一致断言。
    Leaf,
    /// 屏幕误差驱动 cut（`--cluster-error-px`，默认 1.0）。
    On,
}

/// --cluster-lod 选项（Off 时其余字段不消费）。
#[allow(dead_code)]
struct ClusterLodOpt {
    mode: ClusterLodMode,
    pack_path: String,
    threshold_px: f32,
    /// 驻留页压力臂（G31+ #20–23/E：0 = 全驻留默认 = 既有行为 0-byte;
    /// N ≥ 1 = 驻留集 {页 0(root 钉住)} ∪ {1..=N}——cut 后经生产金标准
    /// `apply_page_fallback`(RXS-0350 L3)沿父链回退最近驻留祖先出帧,
    /// 覆盖性复核 fail-closed;N ≥ 总页数 时与全驻留位级一致）。
    resident_pages: u32,
}

#[allow(dead_code)]
impl ClusterLodOpt {
    fn off() -> Self {
        Self {
            mode: ClusterLodMode::Off,
            pack_path: String::new(),
            threshold_px: 1.0,
            resident_pages: 0,
        }
    }
}

/// cut 统计（evidence/打印面；measured 如实登记不设通过线）。
#[allow(dead_code)]
struct ClusterLodReport {
    mode: &'static str,
    threshold_px: f32,
    blocks: usize,
    total_clusters: usize,
    cut_clusters: usize,
    cut_leaf_clusters: usize,
    src_tris: usize,
    passthrough_tris: usize,
    leaf_tris: usize,
    coarse_tris: usize,
    out_tris: usize,
    /// 驻留压力臂（E）：驻留页数（0 = 全驻留）与父簇兜底次数。
    resident_pages: u32,
    fallback_count: usize,
}

/// 装配相机 → 剔除相机（LOD 误差投影基于内部分辨率——渲染发生的分辨率）。
#[allow(dead_code)]
fn cluster_cull_camera(cam: &CameraSpec, in_w: u32, in_h: u32, threshold_px: f32) -> CullCamera {
    let vp = build_vp(cam, in_w, in_h);
    CullCamera {
        view_proj: vp.m,
        cam_pos: cam.eye,
        screen_height_px: in_h as f32,
        error_threshold_px: threshold_px,
    }
}

/// 簇包 fail-closed 校验：① gltf sha 与三角总数匹配；② 叶源 ∪ passthrough
/// 覆盖 0..n 恰一次；③ 逐块逐叶簇逐三角几何与源三角**逐位一致**（顶点 bits）。
#[allow(dead_code)]
fn verify_cluster_pack(pack: &ClusterPack, scene: &SceneData) -> Result<(), String> {
    if pack.gltf_sha256 != scene.gltf_sha256 {
        return Err(format!(
            "簇包 gltf sha 失配: pack={} scene={}",
            pack.gltf_sha256, scene.gltf_sha256
        ));
    }
    let n = scene.indices.len();
    if pack.src_tri_count as usize != n {
        return Err(format!(
            "簇包源三角数失配: pack={} scene={n}",
            pack.src_tri_count
        ));
    }
    let mut seen = vec![false; n];
    let mut mark = |src: u32| -> Result<(), String> {
        let i = src as usize;
        if i >= n {
            return Err(format!("源三角 id 越界: {src}"));
        }
        if seen[i] {
            return Err(format!("源三角 {src} 被覆盖两次"));
        }
        seen[i] = true;
        Ok(())
    };
    for &p in &pack.passthrough {
        mark(p)?;
    }
    for (bi, b) in pack.blocks.iter().enumerate() {
        if b.nodes.len() != b.records.len()
            || b.cluster_albedo.len() != b.records.len()
            || b.cluster_emission.len() != b.records.len()
            || b.cluster_mat.len() != b.records.len()
        {
            return Err(format!("块 {bi} 平行表长不齐"));
        }
        let mut leaf_cursor_check = 0usize;
        for (ci, r) in b.records.iter().enumerate() {
            let is_leaf = b.nodes[ci].child_count == 0;
            if !is_leaf {
                continue;
            }
            let leaf_base = r.triangle_offset as usize / 3;
            leaf_cursor_check = leaf_cursor_check.max(leaf_base + r.triangle_count as usize);
            for t in 0..r.triangle_count as usize {
                let src = *b
                    .leaf_source_tris
                    .get(leaf_base + t)
                    .ok_or_else(|| format!("块 {bi} 叶源表越界"))?;
                mark(src)?;
                // 叶几何位级复核：簇局部三角 3 顶点 bits == 源三角 3 顶点 bits。
                let ti = r.triangle_offset as usize + 3 * t;
                let st = scene.indices[src as usize];
                for k in 0..3 {
                    let li = b.triangle_indices[ti + k] as usize + r.vertex_offset as usize;
                    let pv = b
                        .vertices
                        .get(li)
                        .ok_or_else(|| format!("块 {bi} 顶点池越界"))?;
                    let sv = scene.positions[st[k] as usize];
                    if pv.map(f32::to_bits) != sv.map(f32::to_bits) {
                        return Err(format!(
                            "块 {bi} 簇 {ci} 叶三角 {t}（源 {src}）顶点 {k} 位级失配"
                        ));
                    }
                }
            }
        }
        if leaf_cursor_check > b.leaf_source_tris.len() {
            return Err(format!("块 {bi} 叶源表短于叶三角数"));
        }
    }
    if let Some(hole) = seen.iter().position(|&s| !s) {
        return Err(format!("源三角 {hole} 未被簇包覆盖（叶 ∪ passthrough 有洞）"));
    }
    Ok(())
}

/// cut 选层产物（G36 W1 抽取：apply_cluster_lod 与 apply_geo_combined 共用
/// 选层机核——选层语义单源,组合面禁旁路复刻;字段 = 原函数局部量逐字）。
#[allow(dead_code)]
struct ClusterSelection {
    /// 源三角 id（passthrough ∪ cut 叶簇源;升序,零重复已断言）。
    chosen_src: Vec<u32>,
    /// cut 粗簇（(block, cluster);出帧尾接序）。
    coarse: Vec<(usize, u32)>,
    total_clusters: usize,
    cut_clusters: usize,
    cut_leaf_clusters: usize,
    leaf_tris: usize,
    fallback_count: usize,
}

/// 簇 cut 选层（G36 W1 自 apply_cluster_lod 逐字抽取;行为 0-语义漂移——
/// 逐块生产金标准 cut〔组共享判定球 select_lod_cut_grouped +
/// verify_cut_coverage 直调〕+ E 驻留压力臂兜底）。
#[allow(dead_code)]
fn cluster_lod_select(
    scene: &SceneData,
    pack: &ClusterPack,
    opt: &ClusterLodOpt,
    in_w: u32,
    in_h: u32,
) -> ClusterSelection {
    use rurix_render::geometry::gpu_scene::IDENTITY_3X4;
    use rurix_render::geometry::visible_cluster_set::{
        MeshDagView, apply_page_fallback, select_lod_cut_grouped, verify_cut_coverage,
    };
    let cam = cluster_cull_camera(&scene.camera, in_w, in_h, opt.threshold_px);

    // 逐块 cut（块间独立；identity 变换——三角汤已烘焙世界空间）。
    let mut chosen_src: Vec<u32> = pack.passthrough.clone();
    let mut coarse: Vec<(usize, u32)> = Vec::new(); // (block, cluster)
    let mut total_clusters = 0usize;
    let mut cut_clusters = 0usize;
    let mut cut_leaf_clusters = 0usize;
    let mut leaf_tris = 0usize;
    let mut fallback_count = 0usize;
    // E 驻留压力臂判定（页 0 = root 钉住恒驻留;0 = 全驻留既有行为）。
    let resident = |page: u32| -> bool {
        opt.resident_pages == 0 || page == 0 || page <= opt.resident_pages
    };
    for (bi, b) in pack.blocks.iter().enumerate() {
        total_clusters += b.records.len();
        leaf_tris += b.leaf_source_tris.len();
        let view = MeshDagView::new(&b.records, &b.nodes, &b.children)
            .unwrap_or_else(|e| fail(&format!("--cluster-lod 块 {bi} DAG 拓扑: {e}")));
        let mut cut: Vec<u32> = match opt.mode {
            ClusterLodMode::Leaf => (0..b.records.len() as u32)
                .filter(|&i| b.nodes[i as usize].child_count == 0)
                .collect(),
            // 组共享 LOD 判定球 cut（#58/B4：自心判据在真实 DAG 近距下组内
            // 判定不一致——bistro 实证祖先-后代同选被 verify 拒帧;组球判据
            // 判定输入逐位共享,组内必然一致）。
            ClusterLodMode::On => select_lod_cut_grouped(
                &view,
                &b.cluster_self_lod,
                &b.cluster_parent_lod,
                &IDENTITY_3X4,
                &cam,
            ),
            ClusterLodMode::Off => unreachable!(),
        };
        verify_cut_coverage(&view, &cut)
            .unwrap_or_else(|e| fail(&format!("--cluster-lod 块 {bi} cut 覆盖性: {e}")));
        // E：未驻留页父簇兜底（生产金标准 apply_page_fallback,RXS-0350 L3
        // 直调:沿父链上行至首个驻留祖先,同组兄弟随祖先替换同步撤出——
        // root 页 0 钉住保证终止;兜底后覆盖性复核 fail-closed）。
        if opt.resident_pages > 0 && opt.mode == ClusterLodMode::On {
            let (cut2, fb, _res) = apply_page_fallback(&view, bi as u32, &cut, &resident);
            verify_cut_coverage(&view, &cut2).unwrap_or_else(|e| {
                fail(&format!("--cluster-lod 块 {bi} 兜底后覆盖性: {e}"))
            });
            fallback_count += fb.len();
            cut = cut2;
        }
        cut_clusters += cut.len();
        for &c in &cut {
            let r = &b.records[c as usize];
            if b.nodes[c as usize].child_count == 0 {
                cut_leaf_clusters += 1;
                let leaf_base = r.triangle_offset as usize / 3;
                for t in 0..r.triangle_count as usize {
                    chosen_src.push(b.leaf_source_tris[leaf_base + t]);
                }
            } else {
                coarse.push((bi, c));
            }
        }
    }
    chosen_src.sort_unstable();
    // 覆盖校验已保证叶级恰一次；升序源序 = leaf 模式逐位锚的前提。
    if chosen_src.windows(2).any(|w| w[0] == w[1]) {
        fail("--cluster-lod cut 源三角重复（覆盖性破坏）");
    }
    ClusterSelection {
        chosen_src,
        coarse,
        total_clusters,
        cut_clusters,
        cut_leaf_clusters,
        leaf_tris,
        fallback_count,
    }
}

/// 簇 DAG LOD 施加（--cluster-lod leaf|on）：读簇包 → 校验 → 逐块生产金标准
/// cut（组共享判定球 `select_lod_cut_grouped` + `verify_cut_coverage` 直调）
/// → 重建 SceneData。leaf 模式尾断言：重建产物与输入逐位一致（digest 锚机核
/// 面）。返回簇包供调用方逐帧统计复用（g31 窗口臂;bench/render 臂丢弃）。
/// G36 W1：选层段抽取为 [`cluster_lod_select`]（共用机核）,重建段逐字不动。
#[allow(dead_code)]
fn apply_cluster_lod(
    scene: SceneData,
    opt: &ClusterLodOpt,
    in_w: u32,
    in_h: u32,
) -> (SceneData, Option<(ClusterLodReport, ClusterPack)>) {
    if opt.mode == ClusterLodMode::Off {
        return (scene, None);
    }
    let pack = read_cluster_pack(Path::new(&opt.pack_path))
        .unwrap_or_else(|e| fail(&format!("--cluster-lod 簇包读取: {e}")));
    verify_cluster_pack(&pack, &scene)
        .unwrap_or_else(|e| fail(&format!("--cluster-lod 簇包校验 fail-closed: {e}")));
    let ClusterSelection {
        chosen_src,
        coarse,
        total_clusters,
        cut_clusters,
        cut_leaf_clusters,
        leaf_tris,
        fallback_count,
    } = cluster_lod_select(&scene, &pack, opt, in_w, in_h);

    // 重建 SceneData：源三角（升序）在前，粗簇三角（块序×簇序×簇内序）在后。
    let coarse_tris: usize = coarse
        .iter()
        .map(|&(bi, c)| pack.blocks[bi].records[c as usize].triangle_count as usize)
        .sum();
    let out_tris = chosen_src.len() + coarse_tris;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(out_tris * 3);
    let mut indices: Vec<[u32; 3]> = Vec::with_capacity(out_tris);
    let mut albedo: Vec<[f32; 3]> = Vec::with_capacity(out_tris);
    let mut emission: Vec<[f32; 3]> = Vec::with_capacity(out_tris);
    let mut tri_mat: Vec<u32> = Vec::with_capacity(out_tris);
    for &src in &chosen_src {
        let t = scene.indices[src as usize];
        let base = positions.len() as u32;
        for &vi in &t {
            positions.push(scene.positions[vi as usize]);
        }
        indices.push([base, base + 1, base + 2]);
        albedo.push(scene.albedo[src as usize]);
        emission.push(scene.emission[src as usize]);
        tri_mat.push(scene.tri_mat[src as usize]);
    }
    for &(bi, c) in &coarse {
        let b = &pack.blocks[bi];
        let r = &b.records[c as usize];
        for t in 0..r.triangle_count as usize {
            let ti = r.triangle_offset as usize + 3 * t;
            let base = positions.len() as u32;
            for k in 0..3 {
                let li = b.triangle_indices[ti + k] as usize + r.vertex_offset as usize;
                positions.push(b.vertices[li]);
            }
            indices.push([base, base + 1, base + 2]);
            albedo.push(b.cluster_albedo[c as usize]);
            emission.push(b.cluster_emission[c as usize]);
            tri_mat.push(b.cluster_mat[c as usize]);
        }
    }
    let emissive_tri_count = emission
        .iter()
        .filter(|e| **e != [0.0, 0.0, 0.0])
        .count();
    // emissive 恒 passthrough ⇒ 灯几何面 0-byte（数量必须精确保持）。
    if emissive_tri_count != scene.emissive_tri_count {
        fail(&format!(
            "--cluster-lod emissive 三角数漂移: {emissive_tri_count} ≠ {}（emissive 必须恒 passthrough）",
            scene.emissive_tri_count
        ));
    }
    if opt.mode == ClusterLodMode::Leaf {
        // 全叶对拍锚：重建产物与 off 三角汤逐位一致（fail-closed 机核；
        // 在字段所有权转移前对源比对）。
        if indices.len() != scene.indices.len() {
            fail(&format!(
                "--cluster-lod leaf 三角数漂移: {} ≠ {}",
                indices.len(),
                scene.indices.len()
            ));
        }
        for i in 0..indices.len() {
            let (rt, st) = (indices[i], scene.indices[i]);
            for k in 0..3 {
                let rp = positions[rt[k] as usize].map(f32::to_bits);
                let sp = scene.positions[st[k] as usize].map(f32::to_bits);
                if rp != sp {
                    fail(&format!("--cluster-lod leaf 三角 {i} 顶点 {k} 位级漂移"));
                }
            }
            if albedo[i].map(f32::to_bits) != scene.albedo[i].map(f32::to_bits)
                || emission[i].map(f32::to_bits) != scene.emission[i].map(f32::to_bits)
                || tri_mat[i] != scene.tri_mat[i]
            {
                fail(&format!("--cluster-lod leaf 三角 {i} 属性位级漂移"));
            }
        }
    }
    let report = ClusterLodReport {
        mode: match opt.mode {
            ClusterLodMode::Leaf => "leaf",
            ClusterLodMode::On => "on",
            ClusterLodMode::Off => unreachable!(),
        },
        threshold_px: opt.threshold_px,
        blocks: pack.blocks.len(),
        total_clusters,
        cut_clusters,
        cut_leaf_clusters,
        src_tris: scene.indices.len(),
        passthrough_tris: pack.passthrough.len(),
        leaf_tris,
        coarse_tris,
        out_tris,
        resident_pages: opt.resident_pages,
        fallback_count,
    };
    let rebuilt = SceneData {
        tri_count: indices.len(),
        positions,
        indices,
        albedo,
        emission,
        tri_mat,
        quads: scene.quads,
        points: scene.points,
        camera: scene.camera,
        ev100: scene.ev100,
        texture_mean_albedo: scene.texture_mean_albedo,
        emissive_tri_count,
        gltf_sha256: scene.gltf_sha256,
    };
    (rebuilt, Some((report, pack)))
}

/// 逐帧 cut 统计（G31+ #58 窗口臂消费面：相机逐帧变化 → cut 逐帧重算的
/// measured 登记;**不出帧**——出帧几何冻结于装配期 cut,AS 常驻纪律下逐帧
/// AS 更新归 C/E 阶段,统计如实登记不冒充出帧几何）。
/// `verify_sample` = true 时对本帧 cut 做覆盖性机核（采样帧 fail-closed）。
#[allow(dead_code)]
struct ClusterFrameStat {
    frame: u32,
    cut_clusters: u32,
    cut_leaf_clusters: u32,
    cut_tris: u64,
}

#[allow(dead_code)]
fn cluster_lod_frame_stat(
    pack: &ClusterPack,
    cam_spec: &CameraSpec,
    in_w: u32,
    in_h: u32,
    threshold_px: f32,
    frame: u32,
    verify_sample: bool,
) -> ClusterFrameStat {
    use rurix_render::geometry::gpu_scene::IDENTITY_3X4;
    use rurix_render::geometry::visible_cluster_set::{
        MeshDagView, select_lod_cut_grouped, verify_cut_coverage,
    };
    let cam = cluster_cull_camera(cam_spec, in_w, in_h, threshold_px);
    let mut cut_clusters = 0u32;
    let mut cut_leaf_clusters = 0u32;
    let mut cut_tris = 0u64;
    for (bi, b) in pack.blocks.iter().enumerate() {
        let view = MeshDagView::new(&b.records, &b.nodes, &b.children)
            .unwrap_or_else(|e| fail(&format!("cluster-lod 帧统计块 {bi} 拓扑: {e}")));
        let cut = select_lod_cut_grouped(
            &view,
            &b.cluster_self_lod,
            &b.cluster_parent_lod,
            &IDENTITY_3X4,
            &cam,
        );
        if verify_sample {
            verify_cut_coverage(&view, &cut).unwrap_or_else(|e| {
                fail(&format!("cluster-lod 帧 {frame} 块 {bi} cut 覆盖性: {e}"))
            });
        }
        cut_clusters += cut.len() as u32;
        for &c in &cut {
            let r = &b.records[c as usize];
            cut_tris += u64::from(r.triangle_count);
            if b.nodes[c as usize].child_count == 0 {
                cut_leaf_clusters += 1;
            }
        }
    }
    cut_tris += pack.passthrough.len() as u64;
    ClusterFrameStat {
        frame,
        cut_clusters,
        cut_leaf_clusters,
        cut_tris,
    }
}

// ---------------------------------------------------------------------------
// G31+ #95/#68/#99 World Partition cell + HLOD 生产接线（--wp-hlod 加性面；
// off 默认 = 既有面 0-byte）
//
// 三步文件交接（与 --cluster-lod 同模式；crate 依赖方向约束：rurix-asset 与
// rurix-render 互不依赖——HLOD 质量烘焙(bake_hlod_merged)在 rurix-asset 侧
// g31_wp_hlod_bake 离线完成，本 bin 只消费 RXWH cell 包）：
//   1) `--dump-scene`：RXCS 装配 dump（装配语义单源，同 #58）。
//   2) `g31_wp_hlod_bake`（rurix-asset bin）：RXCS → XZ cell 网格（边长 =
//      资产属性）→ 逐 cell 跨组件合并 + QEM 链（#67/#97）→ RXHL 资产字节 +
//      digest → RXWH v1 cell 包。
//   3) `--wp-hlod full|on --wp-pack <RXWH>`（本 bin bench/render + g31 窗口）：
//      读包 → fail-closed 校验 → **生产机核直调**（world::partition
//      `PartitionRuntime` 距离环 load/unload + 三项预算排队 + world::hlod
//      `HlodRuntime` 事件消费 + digest 核验 + screen-size 互斥选层，禁旁路
//      复刻）→ 互斥出帧重建 SceneData → 既有单 BLAS 车道出帧（#68 HLOD 代理
//      GPU 绘制腿：代理三角真实进 BLAS 进画面）。
//
// 对拍锚：`full` 模式（恒 Full 阈值 + 全 cell 驻留极限）重建产物与 off 三角汤
// **逐位一致**（本函数内 fail-closed 断言）⇒ digest 锚零漂移可机核。
// `on` 模式 = screen-size 阈值互斥切换（远 cell 出 QEM 代理层——远景三角数
// 下降 measured 进 evidence，不进硬门）。
//
// 互斥切换协议（#68 字面）：同 cell 全量 XOR 代理（HlodRuntime 互斥机核 +
// 源三角零重复断言 = 零双绘）；切换 = 同帧原子翻转（翻转前出旧内容,翻转帧起
// 出新内容,无双绘无空洞帧）；代理预热 N 帧再翻转（--wp-warmup,UE
// bRequireWarmup 模式——切换请求后 current 保持 N 帧,第 N 帧原子翻转）。
//
// 边界（诚实登记）：emissive 三角与 quad 灯面尾段恒 passthrough 不参与 cell
// 归属（光源几何面 0-byte——与 #58 同律）；代理三角属性 = cell 面积加权均值
// （远景块状色斑属可接受近似）；出帧几何冻结于装配期选层（逐帧 AS 更新归
// #77 生产接线与 #89 FIF 合流窗），g31 窗口逐帧 tick/选层/切换统计为 host
// 重算 measured 如实登记不冒充出帧几何。
// ---------------------------------------------------------------------------

/// RXWH v1 cell 包 magic（bin-local 交接格式，非冻结格式栈）。
#[allow(dead_code)]
const RXWH_MAGIC: &[u8; 4] = b"RXWH";

/// RXHL v1|v2 资产解码产物（rurix-asset `encode_hlod_asset` 字节的消费面；
/// 逐层三角集合——L0 = 逐 Component 全量（本面不消费,Full 用源三角保位级），
/// L≥1 = 合并层单 `__merged__` proxy）。
#[allow(dead_code)]
struct WpHlodLevels {
    /// levels[l] = 该层全部 proxy 三角（9 f32/tri 位保真,proxy 声明序拼接）。
    levels: Vec<Vec<[f32; 9]>>,
    /// G31+ #96 RXHL v2:与 `levels` 平行的逐三角 corner UV(6 f32/tri
    /// 位保真,同拼接序;cell 代理三角 UV 事实源——`gather_tri_uv_attrs`
    /// 消费)。v1 = None(无 UV 资产臂)。
    levels_uv: Option<Vec<Vec<[f32; 6]>>>,
}

/// RXWH 单 cell（内存形态）。
#[allow(dead_code)]
struct WpHlodCell {
    /// 源三角 id（升序;Full 内容重建面）。
    src: Vec<u32>,
    /// 世界 y 高度范围（partition bounds 第三维）。
    y_min: f32,
    y_max: f32,
    /// cell 代理属性（面积加权 albedo 均值 + mat 众数;emission 恒 0）。
    albedo: [f32; 3],
    mat: u32,
    /// `hlod_asset_digest` 声明值（CellHlodRef 核验面）。
    digest: [u8; 32],
    /// RXHL v1 资产原始字节（实载 digest = sha256(bytes) 与声明核验）。
    rxhl_bytes: Vec<u8>,
    /// 解码后逐层三角（装配期一次解码;Hlod{level} 消费）。
    hlod: WpHlodLevels,
}

/// RXWH cell 包（内存形态）。
#[allow(dead_code)]
struct WpHlodPack {
    gltf_sha256: String,
    cell_size_m: f64,
    grid: (i32, i32, i32, i32),
    levels: u32,
    /// 恒 passthrough 源三角（emissive + quad 灯面尾段;升序）。
    passthrough: Vec<u32>,
    /// (cy,cx) 升序稠密矩形（partition canonical 序;None = 空 cell）。
    cells: Vec<Option<WpHlodCell>>,
}

/// RXHL v1|v2 解码（rurix-asset `encode_hlod_asset` writer 逐字段镜像;
/// fail-closed 边界全校验）。G31+ #96：v2 = 每三角 9×f32 位置后追加
/// 6×f32 corner UV;v1 输入路径行为逐位不变(levels_uv = None)。
#[allow(dead_code)]
fn decode_rxhl(bytes: &[u8], expect_levels: u32) -> Result<WpHlodLevels, String> {
    let mut c = PackCursor { bytes, pos: 0 };
    if c.take(4)? != b"RXHL" {
        return Err("RXHL magic 不符".into());
    }
    let ver = {
        let b = c.take(2)?;
        u16::from_le_bytes([b[0], b[1]])
    };
    if ver != 1 && ver != 2 {
        return Err(format!("RXHL 版本不支持: {ver}"));
    }
    let name_len = {
        let b = c.take(2)?;
        u16::from_le_bytes([b[0], b[1]]) as usize
    };
    let _ = c.take(name_len)?;
    let n_levels = c.u32()?;
    if n_levels != expect_levels {
        return Err(format!("RXHL 层数 {n_levels} ≠ 声明 {expect_levels}"));
    }
    let mut levels = Vec::with_capacity(n_levels as usize);
    let mut uv_levels: Vec<Vec<[f32; 6]>> = Vec::with_capacity(n_levels as usize);
    for li in 0..n_levels {
        let level = c.u32()?;
        if level != li {
            return Err(format!("RXHL 层号乱序: {level} ≠ {li}"));
        }
        let n_proxies = c.u32()? as usize;
        if n_proxies == 0 {
            return Err(format!("RXHL 层 {li} 零 proxy"));
        }
        let mut tris: Vec<[f32; 9]> = Vec::new();
        let mut uvs: Vec<[f32; 6]> = Vec::new();
        for _ in 0..n_proxies {
            let pn = {
                let b = c.take(2)?;
                u16::from_le_bytes([b[0], b[1]]) as usize
            };
            let _ = c.take(pn)?;
            let _source_triangles = c.u32()?;
            let tri_n = c.u32()? as usize;
            for _ in 0..tri_n {
                let mut t = [0.0f32; 9];
                for v in t.iter_mut() {
                    *v = c.f32()?;
                }
                tris.push(t);
                // #96 v2:corner UV(9 f32 位置后紧邻 6 f32)。
                if ver == 2 {
                    let mut u = [0.0f32; 6];
                    for v in u.iter_mut() {
                        *v = c.f32()?;
                    }
                    uvs.push(u);
                }
            }
        }
        if tris.is_empty() {
            return Err(format!("RXHL 层 {li} 零三角"));
        }
        levels.push(tris);
        if ver == 2 {
            uv_levels.push(uvs);
        }
    }
    if c.pos != bytes.len() {
        return Err(format!(
            "RXHL 尾部冗余字节（pos {} ≠ len {}）",
            c.pos,
            bytes.len()
        ));
    }
    Ok(WpHlodLevels {
        levels,
        levels_uv: (ver == 2).then_some(uv_levels),
    })
}

/// RXWH v1 读取（g31_wp_hlod_bake writer 逐字段镜像;fail-closed 边界全校验 +
/// 实载 RXHL digest 自核验——sha256(字节) 必须等于声明 digest,篡改即拒）。
#[allow(dead_code)]
fn read_wp_hlod_pack(path: &Path) -> Result<WpHlodPack, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("RXWH 读取失败 {path:?}: {e}"))?;
    let mut c = PackCursor {
        bytes: &bytes,
        pos: 0,
    };
    if c.take(4)? != RXWH_MAGIC {
        return Err("RXWH magic 不符".into());
    }
    let version = c.u32()?;
    if version != 1 {
        return Err(format!("RXWH 版本不支持: {version}"));
    }
    let sha = String::from_utf8(c.take(64)?.to_vec()).map_err(|_| "sha 非 utf8".to_string())?;
    let cell_size_m = {
        let b = c.take(8)?;
        f64::from_bits(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    };
    if !(cell_size_m.is_finite() && cell_size_m > 0.0) {
        return Err("RXWH cell_size 非有限正".into());
    }
    let gx0 = c.u32()? as i32;
    let gy0 = c.u32()? as i32;
    let gx1 = c.u32()? as i32;
    let gy1 = c.u32()? as i32;
    if gx0 > gx1 || gy0 > gy1 {
        return Err("RXWH 网格范围非法".into());
    }
    let levels = c.u32()?;
    if !(2..=8).contains(&levels) {
        return Err(format!("RXWH levels {levels} 越闭集(2..=8)"));
    }
    let pass_n = c.u32()? as usize;
    let mut passthrough = Vec::with_capacity(pass_n);
    for _ in 0..pass_n {
        passthrough.push(c.u32()?);
    }
    let n_cells = c.u32()? as usize;
    let expect_cells = ((gx1 - gx0) as i64 + 1) * ((gy1 - gy0) as i64 + 1);
    if n_cells as i64 != expect_cells {
        return Err(format!(
            "RXWH cell 数 {n_cells} ≠ 网格稠密数 {expect_cells}"
        ));
    }
    let mut cells = Vec::with_capacity(n_cells);
    for ci in 0..n_cells {
        let tri_n = c.u32()? as usize;
        if tri_n == 0 {
            cells.push(None);
            continue;
        }
        let mut src = Vec::with_capacity(tri_n);
        for _ in 0..tri_n {
            src.push(c.u32()?);
        }
        if src.windows(2).any(|w| w[0] >= w[1]) {
            return Err(format!("RXWH cell {ci} 源三角非严格升序"));
        }
        let y_min = c.f32()?;
        let y_max = c.f32()?;
        let albedo = c.f32x3()?;
        let mat = c.u32()?;
        let mut digest = [0u8; 32];
        digest.copy_from_slice(c.take(32)?);
        let rxhl_len = c.u32()? as usize;
        let rxhl_bytes = c.take(rxhl_len)?.to_vec();
        // 实载资产 digest 自核验（RXS-0364 双构建 hash 相等的运行时消费面;
        // 与 HlodRuntime::register_loaded_asset 同一事实源提前置——包内字节
        // 篡改在读取期即拒,RED 臂经 --wp-red-arm tamper-digest 独立可证）。
        let actual = rurix_pkg::sha256::digest(&rxhl_bytes);
        if actual != digest {
            return Err(format!("RXWH cell {ci} RXHL 字节与声明 digest 不符（篡改/损坏）"));
        }
        let hlod = decode_rxhl(&rxhl_bytes, levels)
            .map_err(|e| format!("RXWH cell {ci} RXHL 解码: {e}"))?;
        cells.push(Some(WpHlodCell {
            src,
            y_min,
            y_max,
            albedo,
            mat,
            digest,
            rxhl_bytes,
            hlod,
        }));
    }
    if c.pos != bytes.len() {
        return Err(format!(
            "RXWH 尾部冗余字节（pos {} ≠ len {}）",
            c.pos,
            bytes.len()
        ));
    }
    Ok(WpHlodPack {
        gltf_sha256: sha,
        cell_size_m,
        grid: (gx0, gy0, gx1, gy1),
        levels,
        passthrough,
        cells,
    })
}

/// cell 包 fail-closed 校验：① gltf sha 与三角总数匹配；② cell 源 ∪
/// passthrough 覆盖 0..n 恰一次（零双绘/零空洞的静态前提）；③ 逐 cell RXHL
/// L0 三角数 == 源三角数（全量层完整性）。
#[allow(dead_code)]
fn verify_wp_pack(pack: &WpHlodPack, scene: &SceneData) -> Result<(), String> {
    if pack.gltf_sha256 != scene.gltf_sha256 {
        return Err(format!(
            "cell 包 gltf sha 失配: pack={} scene={}",
            pack.gltf_sha256, scene.gltf_sha256
        ));
    }
    let n = scene.indices.len();
    let mut seen = vec![false; n];
    let mut mark = |src: u32| -> Result<(), String> {
        let i = src as usize;
        if i >= n {
            return Err(format!("源三角 id 越界: {src}"));
        }
        if seen[i] {
            return Err(format!("源三角 {src} 被覆盖两次"));
        }
        seen[i] = true;
        Ok(())
    };
    for &p in &pack.passthrough {
        mark(p)?;
    }
    for (ci, c) in pack.cells.iter().enumerate() {
        if let Some(cell) = c {
            for &s in &cell.src {
                mark(s)?;
            }
            if cell.hlod.levels[0].len() != cell.src.len() {
                return Err(format!(
                    "cell {ci} RXHL L0 三角数 {} ≠ 源三角数 {}（全量层完整性破坏）",
                    cell.hlod.levels[0].len(),
                    cell.src.len()
                ));
            }
        }
    }
    if let Some(hole) = seen.iter().position(|&s| !s) {
        return Err(format!(
            "源三角 {hole} 未被 cell 包覆盖（cell ∪ passthrough 有洞）"
        ));
    }
    Ok(())
}

/// --wp-hlod 模式闭集。
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum WpHlodMode {
    /// 默认：不触 cell 包，既有面 0-byte（Stage A 锚零漂移）。
    Off,
    /// 恒 Full 阈值 + 全 cell 驻留极限：重建产物与 off 三角汤逐位一致断言
    /// （digest 锚——机核走完整 PartitionRuntime/HlodRuntime 路径不旁路）。
    Full,
    /// screen-size 阈值互斥切换（远 cell 出 QEM 代理层）。
    On,
}

/// --wp-hlod 选项（Off 时其余字段不消费）。
#[allow(dead_code)]
struct WpHlodOpt {
    mode: WpHlodMode,
    pack_path: String,
    /// level 0 屏占阈值（层间 ÷16 递减 ⇒ 切换距离逐层 ×4——计划字面「每层
    /// loading range 递增 ×4 起步」;full 模式不消费）。
    threshold_l0: f64,
    /// 距离环（相机 streaming source;full 模式覆盖为全图半径）。
    loading_radius_m: f32,
    inner_radius_m: f32,
    /// 每帧流送 cell 预算（MaxStreamingCellsPerFrame;排队而非抢占）。
    budget_cells: u32,
    /// 代理预热帧数（切换请求 → 原子翻转间隔;≥1,UE bRequireWarmup 模式）。
    warmup_frames: u32,
}

#[allow(dead_code)]
impl WpHlodOpt {
    fn off() -> Self {
        Self {
            mode: WpHlodMode::Off,
            pack_path: String::new(),
            threshold_l0: 1.0,
            loading_radius_m: 64.0,
            inner_radius_m: 16.0,
            budget_cells: 4,
            warmup_frames: 4,
        }
    }
}

/// 装配期选层/出帧统计（evidence/打印面;measured 如实登记不设通过线）。
#[allow(dead_code)]
struct WpHlodReport {
    mode: &'static str,
    cells_total: usize,
    cells_nonempty: usize,
    cells_resident: usize,
    cells_full: usize,
    cells_hlod: usize,
    cells_culled: usize,
    /// 未驻留非空 cell（流送延迟,诚实登记;稳态装配后应为 0）。
    cells_pending: usize,
    src_tris: usize,
    passthrough_tris: usize,
    full_tris: usize,
    proxy_tris: usize,
    out_tris: usize,
    /// 选层序列 digest（world::hlod::selection_log_digest——确定性对照面）。
    selection_digest: String,
    /// 稳态 tick 帧数与预算排队帧计数（三项预算契约消费面登记）。
    assemble_ticks: u32,
    budget_stall_frames: u64,
}

/// 装配后逐帧上下文（g31 窗口臂消费:逐帧 tick/选层/warmup 切换状态机 +
/// popping 统计;bench/render 臂丢弃）。
#[allow(dead_code)]
struct WpHlodContext {
    pack: WpHlodPack,
    world: rurix_render::world::partition::PersistentWorld,
    partition: rurix_render::world::partition::PartitionRuntime,
    hlod: rurix_render::world::hlod::HlodRuntime,
    thresholds: rurix_render::world::hlod::ScreenSizeThresholds,
    /// 逐 cell 当前出帧内容（None = 未驻留/无内容;互斥态唯一事实源）。
    current: Vec<Option<rurix_render::world::hlod::SelectedContent>>,
    /// 逐 cell 未完成切换（目标内容, 剩余预热帧, 请求帧）。
    pending_switch: Vec<Option<(rurix_render::world::hlod::SelectedContent, u32, u32)>>,
    warmup_frames: u32,
    /// 距离环（装配口径继承——full = 全图半径,on = CLI 值;逐帧 tick 同一
    /// 事实源,禁装配/逐帧双口径漂移）。
    loading_radius_m: f32,
    inner_radius_m: f32,
    /// 下一 tick 帧号（PartitionRuntime 帧号严格递增约束）。
    next_frame: u32,
    /// 已注册 digest 的 cell（register_loaded_asset 恰一次面）。
    registered: Vec<bool>,
    /// 切换事件登记（#99 popping 指标事实源）。
    switch_events: Vec<WpSwitchEvent>,
}

/// 切换事件（原子翻转登记;flip_frame - request_frame == warmup_frames 为
/// 预热协议机核判据）。
#[allow(dead_code)]
#[derive(Clone)]
struct WpSwitchEvent {
    cell: u32,
    from: String,
    to: String,
    request_frame: u32,
    flip_frame: u32,
    /// 翻转前后该 cell 出帧三角数（popping 跳变幅度）。
    tris_before: u64,
    tris_after: u64,
}

/// 逐帧统计（g31 窗口臂 sidecar 消费面）。
#[allow(dead_code)]
struct WpFrameStat {
    frame: u32,
    resident_cells: u32,
    pending_load: u32,
    full_cells: u32,
    hlod_cells: u32,
    culled_cells: u32,
    /// 本帧原子翻转数与翻转三角跳变总幅（#99 popping 指标）。
    switches: u32,
    switch_delta_tris: u64,
    out_tris: u64,
    budget_stall: bool,
}

/// cell 内容出帧三角数（互斥态 → 三角计数;popping 幅度与 out_tris 口径）。
#[allow(dead_code)]
fn wp_content_tris(
    cell: &WpHlodCell,
    content: rurix_render::world::hlod::SelectedContent,
) -> u64 {
    use rurix_render::world::hlod::SelectedContent;
    match content {
        SelectedContent::Full => cell.src.len() as u64,
        SelectedContent::Hlod { level } => cell.hlod.levels[level as usize].len() as u64,
        SelectedContent::Culled => 0,
    }
}

/// 阈值表构造（levels 层严格降:t[i] = t0 / 16^i ⇒ 切换距离逐层 ×4;
/// full 模式 = 恒 Full 表——t0 取极小正数,任何有限屏占 ≥ t[0] ⇒ 恒选 L0,
/// 走完整 select 路径不旁路机核）。
#[allow(dead_code)]
fn wp_thresholds(
    mode: WpHlodMode,
    levels: u32,
    t0: f64,
) -> rurix_render::world::hlod::ScreenSizeThresholds {
    let base = match mode {
        WpHlodMode::Full => 1e-300,
        _ => t0,
    };
    let v: Vec<f64> = (0..levels).map(|i| base / 16f64.powi(i as i32)).collect();
    rurix_render::world::hlod::ScreenSizeThresholds::new(v)
        .unwrap_or_else(|e| fail(&format!("--wp-hlod 阈值表构造: {e}")))
}

/// 相机到 cell 包围球心距离（3D;select 的 distance_m 输入面）。
#[allow(dead_code)]
fn wp_cell_distance(
    world: &rurix_render::world::partition::PersistentWorld,
    cell: u32,
    eye: [f32; 3],
) -> f64 {
    let m = &world.cells[cell as usize];
    let cx = (m.bounds_min[0] as f64 + m.bounds_max[0] as f64) * 0.5;
    let cy = (m.bounds_min[2] as f64 + m.bounds_max[2] as f64) * 0.5;
    let cz = (m.bounds_min[1] as f64 + m.bounds_max[1] as f64) * 0.5;
    // partition bounds = [world_x, world_z, world_y]（2D 网格 xy = 世界 xz,
    // 第三维 = 世界 y 高度）。
    let dx = eye[0] as f64 - cx;
    let dy = eye[1] as f64 - cz;
    let dz = eye[2] as f64 - cy;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// RXWH → PersistentWorld 构造（cells 稠密 (y,x) 升序 + bounds x/y 派生逐位 +
/// 逐非空 cell 一个 spatially_loaded 对象（actor_cost=1,mem = 内容字节估计,
/// 三项预算契约真实记账面）;validate_world 由 PartitionRuntime::new 内嵌）。
#[allow(dead_code)]
fn wp_build_world(pack: &WpHlodPack) -> rurix_render::world::partition::PersistentWorld {
    use rurix_render::world::partition::{
        CellCoord, CellHlodRef, CellMeta, PersistentWorld, SpatialObject, WorldObject,
        derived_cell_bounds_xy,
    };
    let (gx0, gy0, gx1, gy1) = pack.grid;
    let proto = PersistentWorld {
        cell_size_m: pack.cell_size_m,
        grid_min: CellCoord { x: gx0, y: gy0 },
        grid_max: CellCoord { x: gx1, y: gy1 },
        cells: Vec::new(),
        always_loaded: Vec::new(),
        spatially_loaded: Vec::new(),
    };
    let ex = (gx1 - gx0) as i64 + 1;
    let mut cells = Vec::with_capacity(pack.cells.len());
    let mut spatial = Vec::new();
    for (i, c) in pack.cells.iter().enumerate() {
        let coord = CellCoord {
            x: gx0 + (i as i64 % ex) as i32,
            y: gy0 + (i as i64 / ex) as i32,
        };
        let (lo, hi) = derived_cell_bounds_xy(&proto, coord);
        let (y_min, y_max, hlod) = match c {
            Some(cell) => (
                cell.y_min,
                cell.y_max,
                Some(CellHlodRef {
                    digest: cell.digest,
                    levels: pack.levels,
                }),
            ),
            None => (0.0, 0.0, None),
        };
        cells.push(CellMeta {
            coord,
            bounds_min: [lo[0], lo[1], y_min],
            bounds_max: [hi[0], hi[1], y_max],
            page_refs: Vec::new(),
            hlod,
            data_layer_mask: 0,
        });
        if let Some(cell) = c {
            spatial.push(SpatialObject {
                object: WorldObject {
                    id: i as u64,
                    name: format!("cell_{i}"),
                    actor_cost: 1,
                    mem_bytes: (cell.src.len() * 36 + cell.rxhl_bytes.len()) as u64,
                },
                cell: i as u32,
            });
        }
    }
    PersistentWorld {
        cells,
        spatially_loaded: spatial,
        ..proto
    }
}

/// WP 选层产物（G36 W1 抽取：apply_wp_hlod 与 apply_geo_combined 共用选层
/// 机核——生产机核直调链〔PartitionRuntime 稳态流送 + HlodRuntime 事件消费/
/// digest 核验/互斥选层〕单源,组合面禁旁路复刻;字段 = 原函数局部量逐字）。
#[allow(dead_code)]
struct WpSelection {
    partition: rurix_render::world::partition::PartitionRuntime,
    hlod: rurix_render::world::hlod::HlodRuntime,
    thresholds: rurix_render::world::hlod::ScreenSizeThresholds,
    /// 逐 cell 稳态选层（None = 未驻留/空 cell）。
    current: Vec<Option<rurix_render::world::hlod::SelectedContent>>,
    registered: Vec<bool>,
    /// 源三角 id（passthrough ∪ Full cell 源;升序,零重复已断言）。
    chosen_src: Vec<u32>,
    /// Hlod 代理（(cell, level);出帧尾接序 = cell 升序）。
    proxy: Vec<(u32, u32)>,
    cells_full: usize,
    cells_hlod: usize,
    cells_culled: usize,
    cells_pending: usize,
    radius: f32,
    assemble_ticks: u32,
    select_frame: u32,
}

/// WP cell 流送 + 互斥选层（G36 W1 自 apply_wp_hlod 逐字抽取;行为 0-语义
/// 漂移——稳态流送 tick 至队列清空 + digest 核验 + screen-size 互斥选层 +
/// 互斥机核断言 + 源三角零重复断言）。
#[allow(dead_code)]
fn wp_hlod_select(scene: &SceneData, pack: &WpHlodPack, opt: &WpHlodOpt) -> WpSelection {
    use rurix_render::world::hlod::{HlodRuntime, SelectedContent};
    use rurix_render::world::partition::{
        PartitionBudget, PartitionRuntime, SourceKind, StreamingSource,
    };
    let world = wp_build_world(pack);
    let budget = PartitionBudget {
        max_streaming_cells_per_frame: opt.budget_cells.max(1),
        max_actors_to_spawn_per_frame: opt.budget_cells.max(1),
        memory_budget_mb: 4096,
    };
    let mut partition = PartitionRuntime::new(world, budget)
        .unwrap_or_else(|e| fail(&format!("--wp-hlod PartitionRuntime 装配: {e}")));
    let mut hlod = HlodRuntime::new();
    let thresholds = wp_thresholds(opt.mode, pack.levels, opt.threshold_l0);
    // 距离环：full 模式覆盖为全图半径（全 cell 驻留极限——off 位级锚前提）。
    let radius = match opt.mode {
        WpHlodMode::Full => {
            let (gx0, gy0, gx1, gy1) = pack.grid;
            let span_x = ((gx1 - gx0) as f64 + 2.0) * pack.cell_size_m;
            let span_y = ((gy1 - gy0) as f64 + 2.0) * pack.cell_size_m;
            ((span_x * span_x + span_y * span_y).sqrt() * 2.0) as f32
        }
        _ => opt.loading_radius_m,
    };
    let eye = scene.camera.eye;
    let source = StreamingSource {
        kind: SourceKind::Camera,
        position_m: [eye[0], eye[2]],
        loading_radius_m: radius,
        inner_radius_m: opt.inner_radius_m.min(radius),
    };
    source
        .validate()
        .unwrap_or_else(|e| fail(&format!("--wp-hlod streaming source: {e}")));
    // 稳态流送（tick 至队列清空——bench/render 出帧确定性;预算排队语义
    // 真实走机核,稳态 tick 数 = 预算契约消费的机器证据）。
    let n_cells = pack.cells.len();
    let mut registered = vec![false; n_cells];
    let mut assemble_ticks = 0u32;
    let mut frame = 0u32;
    loop {
        let ev = partition
            .tick(frame, &[source])
            .unwrap_or_else(|e| fail(&format!("--wp-hlod tick 帧 {frame}: {e}")));
        assemble_ticks += 1;
        let events = partition.drain_events();
        hlod.apply_cell_events(&events)
            .unwrap_or_else(|e| fail(&format!("--wp-hlod 事件消费帧 {frame}: {e}")));
        // 新驻留 cell:实载资产 digest 核验（register 恰一次）。
        for cell in hlod.resident().clone() {
            let ci = cell as usize;
            if !registered[ci] {
                if let Some(c) = &pack.cells[ci] {
                    let meta = partition.world().cells[ci]
                        .hlod
                        .as_ref()
                        .unwrap_or_else(|| fail("--wp-hlod 非空 cell 缺 HLOD 引用"))
                        .clone();
                    let actual = rurix_pkg::sha256::digest(&c.rxhl_bytes);
                    hlod.register_loaded_asset(cell, &meta, actual)
                        .unwrap_or_else(|e| {
                            fail(&format!("--wp-hlod cell {cell} 资产 digest 核验: {e}"))
                        });
                }
                registered[ci] = true;
            }
        }
        if ev.queue_depth_end == 0 && assemble_ticks > 1 {
            frame += 1;
            break;
        }
        frame += 1;
        if assemble_ticks > n_cells as u32 + 64 {
            fail("--wp-hlod 稳态流送未收敛（预算/距离环参数异常）");
        }
    }
    // 稳态互斥选层（select 帧号 = 稳态后首帧;记录进 selection log）。
    let select_frame = frame;
    let mut current: Vec<Option<SelectedContent>> = vec![None; n_cells];
    for &cell in hlod.resident().clone().iter() {
        let ci = cell as usize;
        if pack.cells[ci].is_none() {
            continue; // 空 cell 无内容（select 恒 Full 无意义,不进记录面）
        }
        let d = wp_cell_distance(partition.world(), cell, eye);
        let content = hlod
            .select(partition.world(), cell, d, &thresholds, select_frame)
            .unwrap_or_else(|e| fail(&format!("--wp-hlod cell {cell} 选层: {e}")));
        current[ci] = Some(content);
    }
    hlod.assert_mutually_exclusive()
        .unwrap_or_else(|e| fail(&format!("--wp-hlod 互斥机核: {e}")));
    // 互斥出帧重建：passthrough ∪ Full cell 源三角（升序,零重复断言 = 零双绘
    // 机核）+ 代理三角尾接（cell id 升序 × 层内序）。
    let mut chosen_src: Vec<u32> = pack.passthrough.clone();
    let mut cells_full = 0usize;
    let mut cells_hlod = 0usize;
    let mut cells_culled = 0usize;
    let mut cells_pending = 0usize;
    let mut proxy: Vec<(u32, u32)> = Vec::new(); // (cell, level)
    for (ci, c) in pack.cells.iter().enumerate() {
        let Some(cell) = c else { continue };
        match current[ci] {
            Some(SelectedContent::Full) => {
                cells_full += 1;
                chosen_src.extend_from_slice(&cell.src);
            }
            Some(SelectedContent::Hlod { level }) => {
                cells_hlod += 1;
                proxy.push((ci as u32, level));
            }
            Some(SelectedContent::Culled) => cells_culled += 1,
            None => cells_pending += 1, // 流送未达（诚实登记;稳态后应为 0）
        }
    }
    chosen_src.sort_unstable();
    if chosen_src.windows(2).any(|w| w[0] == w[1]) {
        fail("--wp-hlod 出帧源三角重复（互斥破坏 = 双绘,fail-closed）");
    }
    WpSelection {
        partition,
        hlod,
        thresholds,
        current,
        registered,
        chosen_src,
        proxy,
        cells_full,
        cells_hlod,
        cells_culled,
        cells_pending,
        radius,
        assemble_ticks,
        select_frame,
    }
}

/// WP/HLOD 施加（--wp-hlod full|on）：读 cell 包 → 校验 → 生产机核直调
/// （PartitionRuntime 稳态流送 + HlodRuntime 事件消费/digest 核验/互斥选层）
/// → 互斥出帧重建 SceneData。full 模式尾断言：重建产物与输入逐位一致。
/// 返回上下文供 g31 窗口臂逐帧统计复用（bench/render 臂丢弃）。
/// G36 W1：选层段抽取为 [`wp_hlod_select`]（共用机核）,重建段逐字不动。
#[allow(dead_code)]
fn apply_wp_hlod(
    scene: SceneData,
    opt: &WpHlodOpt,
) -> (SceneData, Option<(WpHlodReport, WpHlodContext)>) {
    use rurix_render::world::hlod::selection_log_digest;
    if opt.mode == WpHlodMode::Off {
        return (scene, None);
    }
    let pack = read_wp_hlod_pack(Path::new(&opt.pack_path))
        .unwrap_or_else(|e| fail(&format!("--wp-hlod cell 包读取: {e}")));
    verify_wp_pack(&pack, &scene)
        .unwrap_or_else(|e| fail(&format!("--wp-hlod cell 包校验 fail-closed: {e}")));
    let n_cells = pack.cells.len();
    let WpSelection {
        partition,
        hlod,
        thresholds,
        current,
        registered,
        chosen_src,
        proxy,
        cells_full,
        cells_hlod,
        cells_culled,
        cells_pending,
        radius,
        assemble_ticks,
        select_frame,
    } = wp_hlod_select(&scene, &pack, opt);
    let full_tris = chosen_src.len() - pack.passthrough.len();
    let proxy_tris: usize = proxy
        .iter()
        .map(|&(ci, level)| {
            pack.cells[ci as usize].as_ref().unwrap().hlod.levels[level as usize].len()
        })
        .sum();
    let out_tris = chosen_src.len() + proxy_tris;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(out_tris * 3);
    let mut indices: Vec<[u32; 3]> = Vec::with_capacity(out_tris);
    let mut albedo: Vec<[f32; 3]> = Vec::with_capacity(out_tris);
    let mut emission: Vec<[f32; 3]> = Vec::with_capacity(out_tris);
    let mut tri_mat: Vec<u32> = Vec::with_capacity(out_tris);
    for &src in &chosen_src {
        let t = scene.indices[src as usize];
        let base = positions.len() as u32;
        for &vi in &t {
            positions.push(scene.positions[vi as usize]);
        }
        indices.push([base, base + 1, base + 2]);
        albedo.push(scene.albedo[src as usize]);
        emission.push(scene.emission[src as usize]);
        tri_mat.push(scene.tri_mat[src as usize]);
    }
    for &(ci, level) in &proxy {
        let cell = pack.cells[ci as usize].as_ref().unwrap();
        for t in &cell.hlod.levels[level as usize] {
            let base = positions.len() as u32;
            for k in 0..3 {
                positions.push([t[k * 3], t[k * 3 + 1], t[k * 3 + 2]]);
            }
            indices.push([base, base + 1, base + 2]);
            albedo.push(cell.albedo);
            emission.push([0.0; 3]);
            tri_mat.push(cell.mat);
        }
    }
    let emissive_tri_count = emission.iter().filter(|e| **e != [0.0, 0.0, 0.0]).count();
    if emissive_tri_count != scene.emissive_tri_count {
        fail(&format!(
            "--wp-hlod emissive 三角数漂移: {emissive_tri_count} ≠ {}（emissive 必须恒 passthrough）",
            scene.emissive_tri_count
        ));
    }
    if opt.mode == WpHlodMode::Full {
        // 全量对拍锚：重建产物与 off 三角汤逐位一致（fail-closed 机核）。
        if cells_hlod != 0 || cells_culled != 0 || cells_pending != 0 {
            fail(&format!(
                "--wp-hlod full 极限非全 Full: hlod={cells_hlod} culled={cells_culled} pending={cells_pending}"
            ));
        }
        if indices.len() != scene.indices.len() {
            fail(&format!(
                "--wp-hlod full 三角数漂移: {} ≠ {}",
                indices.len(),
                scene.indices.len()
            ));
        }
        for i in 0..indices.len() {
            let (rt, st) = (indices[i], scene.indices[i]);
            for k in 0..3 {
                let rp = positions[rt[k] as usize].map(f32::to_bits);
                let sp = scene.positions[st[k] as usize].map(f32::to_bits);
                if rp != sp {
                    fail(&format!("--wp-hlod full 三角 {i} 顶点 {k} 位级漂移"));
                }
            }
            if albedo[i].map(f32::to_bits) != scene.albedo[i].map(f32::to_bits)
                || emission[i].map(f32::to_bits) != scene.emission[i].map(f32::to_bits)
                || tri_mat[i] != scene.tri_mat[i]
            {
                fail(&format!("--wp-hlod full 三角 {i} 属性位级漂移"));
            }
        }
    }
    let cells_nonempty = pack.cells.iter().filter(|c| c.is_some()).count();
    let report = WpHlodReport {
        mode: match opt.mode {
            WpHlodMode::Full => "full",
            WpHlodMode::On => "on",
            WpHlodMode::Off => unreachable!(),
        },
        cells_total: pack.cells.len(),
        cells_nonempty,
        cells_resident: hlod.resident().len(),
        cells_full,
        cells_hlod,
        cells_culled,
        cells_pending,
        src_tris: scene.indices.len(),
        passthrough_tris: pack.passthrough.len(),
        full_tris,
        proxy_tris,
        out_tris,
        selection_digest: {
            let d = selection_log_digest(hlod.records());
            let mut s = String::with_capacity(64);
            for b in d {
                s.push_str(&format!("{b:02x}"));
            }
            s
        },
        assemble_ticks,
        budget_stall_frames: partition.counters().budget_stall_frames,
    };
    let rebuilt = SceneData {
        tri_count: indices.len(),
        positions,
        indices,
        albedo,
        emission,
        tri_mat,
        quads: scene.quads,
        points: scene.points,
        camera: scene.camera,
        ev100: scene.ev100,
        texture_mean_albedo: scene.texture_mean_albedo,
        emissive_tri_count,
        gltf_sha256: scene.gltf_sha256,
    };
    let ctx = WpHlodContext {
        pack,
        world: partition.world().clone(),
        partition,
        hlod,
        thresholds,
        current,
        pending_switch: vec![None; n_cells],
        warmup_frames: opt.warmup_frames.max(1),
        loading_radius_m: radius,
        inner_radius_m: opt.inner_radius_m.min(radius),
        next_frame: select_frame + 1,
        registered,
        switch_events: Vec::new(),
    };
    (rebuilt, Some((report, ctx)))
}

/// 逐帧 WP/HLOD 状态推进（g31 窗口臂消费面：相机逐帧变化 → tick 流送 +
/// 互斥选层 + warmup 原子翻转协议的 measured 登记;**不出帧**——出帧几何冻结
/// 于装配期选层,统计如实登记不冒充。#99 popping 指标事实源）。
#[allow(dead_code)]
fn wp_hlod_frame_tick(ctx: &mut WpHlodContext, eye: [f32; 3]) -> WpFrameStat {
    use rurix_render::world::hlod::SelectedContent;
    use rurix_render::world::partition::{SourceKind, StreamingSource};
    let frame = ctx.next_frame;
    ctx.next_frame += 1;
    let source = StreamingSource {
        kind: SourceKind::Camera,
        position_m: [eye[0], eye[2]],
        loading_radius_m: ctx.loading_radius_m,
        inner_radius_m: ctx.inner_radius_m,
    };
    let ev = ctx
        .partition
        .tick(frame, &[source])
        .unwrap_or_else(|e| fail(&format!("--wp-hlod 窗口帧 {frame} tick: {e}")));
    let events = ctx.partition.drain_events();
    ctx.hlod
        .apply_cell_events(&events)
        .unwrap_or_else(|e| fail(&format!("--wp-hlod 窗口帧 {frame} 事件: {e}")));
    for cell in ctx.hlod.resident().clone() {
        let ci = cell as usize;
        if !ctx.registered[ci] {
            if let Some(c) = &ctx.pack.cells[ci] {
                let meta = ctx.world.cells[ci]
                    .hlod
                    .as_ref()
                    .unwrap_or_else(|| fail("--wp-hlod 非空 cell 缺 HLOD 引用"))
                    .clone();
                let actual = rurix_pkg::sha256::digest(&c.rxhl_bytes);
                ctx.hlod
                    .register_loaded_asset(cell, &meta, actual)
                    .unwrap_or_else(|e| {
                        fail(&format!("--wp-hlod 窗口 cell {cell} digest 核验: {e}"))
                    });
            }
            ctx.registered[ci] = true;
        }
    }
    // 逐驻留 cell 互斥选层 + warmup 原子翻转状态机。
    let mut switches = 0u32;
    let mut switch_delta = 0u64;
    let resident = ctx.hlod.resident().clone();
    for &cell in resident.iter() {
        let ci = cell as usize;
        let Some(c) = &ctx.pack.cells[ci] else {
            continue;
        };
        let d = wp_cell_distance(&ctx.world, cell, eye);
        let desired = ctx
            .hlod
            .select(&ctx.world, cell, d, &ctx.thresholds, frame)
            .unwrap_or_else(|e| fail(&format!("--wp-hlod 窗口 cell {cell} 选层: {e}")));
        match ctx.current[ci] {
            None => {
                // 新驻留 cell:以选层结果直接进场（初装无预热惩罚——预热协议
                // 只作用于**内容切换**;流送延迟已由 pending 计数如实登记）。
                ctx.current[ci] = Some(desired);
            }
            Some(cur) if cur == desired => {
                ctx.pending_switch[ci] = None; // 期望回稳,取消未完成切换
            }
            Some(cur) => match ctx.pending_switch[ci] {
                Some((target, left, req)) if target == desired => {
                    if left == 0 {
                        // 原子翻转帧（同帧切换,前帧出旧内容本帧起出新内容——
                        // 无双绘无空洞;flip - request == warmup 协议机核）。
                        let before = wp_content_tris(c, cur);
                        let after = wp_content_tris(c, desired);
                        ctx.switch_events.push(WpSwitchEvent {
                            cell,
                            from: cur.as_str(),
                            to: desired.as_str(),
                            request_frame: req,
                            flip_frame: frame,
                            tris_before: before,
                            tris_after: after,
                        });
                        switches += 1;
                        switch_delta += before.abs_diff(after);
                        ctx.current[ci] = Some(desired);
                        ctx.pending_switch[ci] = None;
                    } else {
                        ctx.pending_switch[ci] = Some((target, left - 1, req));
                    }
                }
                _ => {
                    // 新切换请求（预热开始;目标变更即重置——确定性协议）。
                    ctx.pending_switch[ci] = Some((desired, ctx.warmup_frames - 1, frame));
                }
            },
        }
    }
    // 驻留撤出 cell:内容清空（卸载 = 内容移除;窗口统计面登记）。
    for ci in 0..ctx.current.len() {
        if ctx.current[ci].is_some() && !resident.contains(&(ci as u32)) {
            ctx.current[ci] = None;
            ctx.pending_switch[ci] = None;
        }
    }
    // 帧末互斥态统计。
    let mut full_cells = 0u32;
    let mut hlod_cells = 0u32;
    let mut culled_cells = 0u32;
    let mut out_tris = ctx.pack.passthrough.len() as u64;
    for (ci, cur) in ctx.current.iter().enumerate() {
        let Some(c) = &ctx.pack.cells[ci] else {
            continue;
        };
        match cur {
            Some(SelectedContent::Full) => {
                full_cells += 1;
                out_tris += c.src.len() as u64;
            }
            Some(SelectedContent::Hlod { level }) => {
                hlod_cells += 1;
                out_tris += c.hlod.levels[*level as usize].len() as u64;
            }
            Some(SelectedContent::Culled) => culled_cells += 1,
            None => {}
        }
    }
    WpFrameStat {
        frame,
        resident_cells: ev.resident_cells,
        pending_load: ev.queue_depth_end,
        full_cells,
        hlod_cells,
        culled_cells,
        switches,
        switch_delta_tris: switch_delta,
        out_tris,
        budget_stall: ev.budget_stall,
    }
}

// ---------------------------------------------------------------------------
// G36 全特性合流 W1/W2 — 逐三角 provenance 与统一几何重建（加性面;门
// g36.wave1.provenance / g36.wave2.geo_merge）
//
// 根因与解除：--cluster-lod / --wp-hlod 重建三角汤（升序源三角 + 尾接代理/
// 粗簇三角）后,一切按"装配序三角位置"绑定的侧表假设破坏——①B4 逐三角
// UV/tritex 同序假设;②B1 SceneNodeGroup 节点连续段假设;③dyn/skin 尾接段
// tri_base 基址假设——此前以闭集互斥 fail-closed 拒组合（"组合面归后续波"）。
// 本节以 provenance（逐输出三角源出处）为单一事实源解除该互斥：
//   ① 重建输出 Vec<TriProvenance>（Src(源 id) | 簇粗代理 | WP cell 代理）;
//   ② 侧表经 gather 重排（Src 按源 id 取值;代理三角 UV=0 + tritex 强制 −1
//      走既有常量面回退——cluster_albedo/cell 面积加权均值即该回退语义;
//      代理属性保持简化归 #96 留窗,如实登记不冒充）;
//   ③ 节点段经 provenance 重导出（升序源序保持节点连续性——源节点段本就是
//      装配序连续区间;代理尾段按块/cell 成组,AABB 自重建几何精确重算,
//      "三角 ⊆ AABB 精确包含"不变量维持）;
//   ④ dyn/skin 尾接段基址 = 重建后 scene.indices.len()（既有计算点已在
//      apply_* 之后,组合面零改动成立）。
// W2 组合语义（--cluster-lod × --wp-hlod 同开）：WP cell 互斥选层先行
// （Full/Hlod/Culled）,簇 cut 只对 Full 域出叶;跨界粗簇（覆盖源跨 Full 与
// Hlod/Culled cell）回退为叶级源三角出帧（零双绘 + 零空洞;粗簇不与 cell
// 代理重叠）。覆盖机核：出帧源三角集 ≡ WP Full 域恰一次,fail-closed。
// 纪律：apply_cluster_lod / apply_wp_hlod 单开路径行为逐字维持（选层机核
// cluster_lod_select / wp_hlod_select 共用,重建段 0-byte）;组合对拍锚 =
// leaf×full 极限下重建产物与 off 三角汤逐位一致（本节内 fail-closed）。
// ---------------------------------------------------------------------------

/// 逐输出三角源出处（W1 单一事实源;重建/侧表 gather/节点段重导出共用）。
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TriProvenance {
    /// 源三角恒等透传（id = 装配序全局三角号;几何/属性与源逐位一致）。
    Src(u32),
    /// 簇 DAG 粗簇代理三角（--cluster-lod on cut 非叶簇）。
    ClusterCoarse { block: u32, cluster: u32 },
    /// WP cell HLOD 代理三角（--wp-hlod on 远 cell）。
    WpProxy { cell: u32, level: u32 },
}

/// 粗簇覆盖源三角集（DAG 下行至叶,收集 leaf_source_tris;W2 组合面消费——
/// 粗簇 × WP cell 域跨界判定）。返回升序集（叶覆盖恰一次由簇包校验保证）。
#[allow(dead_code)]
fn cluster_covered_sources(b: &ClusterPackBlock, cluster: u32) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    let mut stack = vec![cluster];
    let mut seen = vec![false; b.records.len()];
    while let Some(c) = stack.pop() {
        // 组共享 DAG：子簇多父可达——去重防多路径重复下行（Nanite 组语义:
        // 同组父簇 children 共享）。
        if seen[c as usize] {
            continue;
        }
        seen[c as usize] = true;
        let node = &b.nodes[c as usize];
        if node.child_count == 0 {
            let r = &b.records[c as usize];
            let leaf_base = r.triangle_offset as usize / 3;
            for t in 0..r.triangle_count as usize {
                out.push(b.leaf_source_tris[leaf_base + t]);
            }
        } else {
            let s = node.first_child as usize;
            for k in 0..node.child_count as usize {
                stack.push(b.children[s + k]);
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// 统一几何重建（W1 事实源）：identity 源三角（升序）在前 + 簇粗代理尾接
/// （块序×簇序×簇内序）+ WP cell 代理尾接（cell 升序×层内序）,并输出逐
/// 三角 provenance。重建循环与 apply_cluster_lod / apply_wp_hlod 重建段
/// 逐字同构（attrs 语义同源:identity = 源属性位保真;簇粗代理 =
/// cluster_albedo/emission/mat;cell 代理 = cell albedo/emission 0/mat）。
#[allow(dead_code)]
fn geo_rebuild(
    scene: &SceneData,
    identity: &[u32],
    coarse: &[(usize, u32)],
    cl_pack: Option<&ClusterPack>,
    wp_proxy: &[(u32, u32)],
    wp_pack: Option<&WpHlodPack>,
) -> (SceneData, Vec<TriProvenance>) {
    let coarse_tris: usize = coarse
        .iter()
        .map(|&(bi, c)| {
            cl_pack.expect("coarse 非空须随簇包").blocks[bi].records[c as usize].triangle_count
                as usize
        })
        .sum();
    let proxy_tris: usize = wp_proxy
        .iter()
        .map(|&(ci, level)| {
            wp_pack.expect("wp_proxy 非空须随 cell 包").cells[ci as usize]
                .as_ref()
                .unwrap()
                .hlod
                .levels[level as usize]
                .len()
        })
        .sum();
    let out_tris = identity.len() + coarse_tris + proxy_tris;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(out_tris * 3);
    let mut indices: Vec<[u32; 3]> = Vec::with_capacity(out_tris);
    let mut albedo: Vec<[f32; 3]> = Vec::with_capacity(out_tris);
    let mut emission: Vec<[f32; 3]> = Vec::with_capacity(out_tris);
    let mut tri_mat: Vec<u32> = Vec::with_capacity(out_tris);
    let mut prov: Vec<TriProvenance> = Vec::with_capacity(out_tris);
    for &src in identity {
        let t = scene.indices[src as usize];
        let base = positions.len() as u32;
        for &vi in &t {
            positions.push(scene.positions[vi as usize]);
        }
        indices.push([base, base + 1, base + 2]);
        albedo.push(scene.albedo[src as usize]);
        emission.push(scene.emission[src as usize]);
        tri_mat.push(scene.tri_mat[src as usize]);
        prov.push(TriProvenance::Src(src));
    }
    for &(bi, c) in coarse {
        let b = &cl_pack.unwrap().blocks[bi];
        let r = &b.records[c as usize];
        for t in 0..r.triangle_count as usize {
            let ti = r.triangle_offset as usize + 3 * t;
            let base = positions.len() as u32;
            for k in 0..3 {
                let li = b.triangle_indices[ti + k] as usize + r.vertex_offset as usize;
                positions.push(b.vertices[li]);
            }
            indices.push([base, base + 1, base + 2]);
            albedo.push(b.cluster_albedo[c as usize]);
            emission.push(b.cluster_emission[c as usize]);
            tri_mat.push(b.cluster_mat[c as usize]);
            prov.push(TriProvenance::ClusterCoarse {
                block: bi as u32,
                cluster: c,
            });
        }
    }
    for &(ci, level) in wp_proxy {
        let cell = wp_pack.unwrap().cells[ci as usize].as_ref().unwrap();
        for t in &cell.hlod.levels[level as usize] {
            let base = positions.len() as u32;
            for k in 0..3 {
                positions.push([t[k * 3], t[k * 3 + 1], t[k * 3 + 2]]);
            }
            indices.push([base, base + 1, base + 2]);
            albedo.push(cell.albedo);
            emission.push([0.0; 3]);
            tri_mat.push(cell.mat);
            prov.push(TriProvenance::WpProxy { cell: ci, level });
        }
    }
    let emissive_tri_count = emission.iter().filter(|e| **e != [0.0, 0.0, 0.0]).count();
    // emissive 恒 passthrough ⇒ 灯几何面 0-byte（两包 passthrough 交集律;
    // 数量必须精确保持——与 apply_* 同一机核判据）。
    if emissive_tri_count != scene.emissive_tri_count {
        fail(&format!(
            "geo_rebuild emissive 三角数漂移: {emissive_tri_count} ≠ {}（emissive 必须恒 passthrough）",
            scene.emissive_tri_count
        ));
    }
    let rebuilt = SceneData {
        tri_count: indices.len(),
        positions,
        indices,
        albedo,
        emission,
        tri_mat,
        quads: scene.quads.clone(),
        points: scene.points.clone(),
        camera: scene.camera,
        ev100: scene.ev100,
        texture_mean_albedo: scene.texture_mean_albedo,
        emissive_tri_count,
        gltf_sha256: scene.gltf_sha256.clone(),
    };
    (rebuilt, prov)
}

/// W2 组合统计（evidence/打印面;measured 如实登记不设通过线）。
#[allow(dead_code)]
struct GeoCombinedStats {
    /// identity 出帧源三角数（含两包 passthrough 交集律与跨界回退叶）。
    identity_tris: usize,
    /// 出帧粗簇数 / 粗簇三角数（覆盖源 ⊆ WP Full 域者）。
    coarse_emitted: usize,
    coarse_tris: usize,
    /// 跨界粗簇（覆盖源跨 Full 与 Hlod/Culled cell）→ 叶级回退：簇数 / 回退
    /// 源三角数（细度损失面如实登记——cell 边界簇不粗化,零双绘零空洞）。
    straddle_clusters: usize,
    straddle_fallback_tris: usize,
    /// WP cell 代理三角数。
    wp_proxy_tris: usize,
    out_tris: usize,
}

/// 组合几何管线产物（W1/W2;prov = 侧表 gather/节点段重导出消费面）。
#[allow(dead_code)]
struct GeoApplied {
    prov: Vec<TriProvenance>,
    cluster: Option<(ClusterLodReport, ClusterPack)>,
    wp: Option<(WpHlodReport, WpHlodContext)>,
    combined: Option<GeoCombinedStats>,
}

/// 统一几何管线（W1/W2 事实源）：--cluster-lod × --wp-hlod 四态分派。
/// 单开态与 apply_cluster_lod / apply_wp_hlod 重建产物**逐位一致**（同一
/// 选层机核 + 同构重建循环——leaf/full 锚在本函数内同判据断言）;双开态 =
/// W2 组合语义（WP 选层先行 → Full 域内簇 cut → 跨界粗簇叶级回退 → 统一
/// 重建）。返回 GeoApplied（prov 供侧表 gather/节点段重导出消费）。
#[allow(dead_code)]
fn apply_geo_combined(
    scene: SceneData,
    cl_opt: &ClusterLodOpt,
    wp_opt: &WpHlodOpt,
    in_w: u32,
    in_h: u32,
) -> (SceneData, Option<GeoApplied>) {
    use rurix_render::world::hlod::selection_log_digest;
    let cl_on = cl_opt.mode != ClusterLodMode::Off;
    let wp_on = wp_opt.mode != WpHlodMode::Off;
    if !cl_on && !wp_on {
        return (scene, None);
    }
    // ── 单开态：同一选层机核（cluster_lod_select / wp_hlod_select）+ 同构
    //    重建循环（geo_rebuild）⇒ 与 apply_cluster_lod / apply_wp_hlod 重建
    //    产物逐位一致（leaf/full 锚同判据在本分支内断言）,并输出真 prov。──
    if cl_on && !wp_on {
        let cl_pack = read_cluster_pack(Path::new(&cl_opt.pack_path))
            .unwrap_or_else(|e| fail(&format!("--cluster-lod 簇包读取: {e}")));
        verify_cluster_pack(&cl_pack, &scene)
            .unwrap_or_else(|e| fail(&format!("--cluster-lod 簇包校验 fail-closed: {e}")));
        let csel = cluster_lod_select(&scene, &cl_pack, cl_opt, in_w, in_h);
        let (rebuilt, prov) = geo_rebuild(
            &scene,
            &csel.chosen_src,
            &csel.coarse,
            Some(&cl_pack),
            &[],
            None,
        );
        if cl_opt.mode == ClusterLodMode::Leaf {
            geo_assert_bitexact(&rebuilt, &scene, "--cluster-lod leaf（geo 管线）");
        }
        let coarse_tris: usize = csel
            .coarse
            .iter()
            .map(|&(bi, c)| cl_pack.blocks[bi].records[c as usize].triangle_count as usize)
            .sum();
        let report = ClusterLodReport {
            mode: match cl_opt.mode {
                ClusterLodMode::Leaf => "leaf",
                ClusterLodMode::On => "on",
                ClusterLodMode::Off => unreachable!(),
            },
            threshold_px: cl_opt.threshold_px,
            blocks: cl_pack.blocks.len(),
            total_clusters: csel.total_clusters,
            cut_clusters: csel.cut_clusters,
            cut_leaf_clusters: csel.cut_leaf_clusters,
            src_tris: scene.indices.len(),
            passthrough_tris: cl_pack.passthrough.len(),
            leaf_tris: csel.leaf_tris,
            coarse_tris,
            out_tris: rebuilt.indices.len(),
            resident_pages: cl_opt.resident_pages,
            fallback_count: csel.fallback_count,
        };
        return (
            rebuilt,
            Some(GeoApplied {
                prov,
                cluster: Some((report, cl_pack)),
                wp: None,
                combined: None,
            }),
        );
    }
    if !cl_on && wp_on {
        let wp_pack = read_wp_hlod_pack(Path::new(&wp_opt.pack_path))
            .unwrap_or_else(|e| fail(&format!("--wp-hlod cell 包读取: {e}")));
        verify_wp_pack(&wp_pack, &scene)
            .unwrap_or_else(|e| fail(&format!("--wp-hlod cell 包校验 fail-closed: {e}")));
        let n_cells = wp_pack.cells.len();
        let wsel = wp_hlod_select(&scene, &wp_pack, wp_opt);
        let (rebuilt, prov) = geo_rebuild(
            &scene,
            &wsel.chosen_src,
            &[],
            None,
            &wsel.proxy,
            Some(&wp_pack),
        );
        if wp_opt.mode == WpHlodMode::Full {
            if wsel.cells_hlod != 0 || wsel.cells_culled != 0 || wsel.cells_pending != 0 {
                fail(&format!(
                    "--wp-hlod full 极限非全 Full: hlod={} culled={} pending={}",
                    wsel.cells_hlod, wsel.cells_culled, wsel.cells_pending
                ));
            }
            geo_assert_bitexact(&rebuilt, &scene, "--wp-hlod full（geo 管线）");
        }
        let wp_proxy_tris: usize = wsel
            .proxy
            .iter()
            .map(|&(ci, level)| {
                wp_pack.cells[ci as usize].as_ref().unwrap().hlod.levels[level as usize].len()
            })
            .sum();
        let cells_nonempty = wp_pack.cells.iter().filter(|c| c.is_some()).count();
        let report = WpHlodReport {
            mode: match wp_opt.mode {
                WpHlodMode::Full => "full",
                WpHlodMode::On => "on",
                WpHlodMode::Off => unreachable!(),
            },
            cells_total: n_cells,
            cells_nonempty,
            cells_resident: wsel.hlod.resident().len(),
            cells_full: wsel.cells_full,
            cells_hlod: wsel.cells_hlod,
            cells_culled: wsel.cells_culled,
            cells_pending: wsel.cells_pending,
            src_tris: scene.indices.len(),
            passthrough_tris: wp_pack.passthrough.len(),
            full_tris: wsel.chosen_src.len() - wp_pack.passthrough.len(),
            proxy_tris: wp_proxy_tris,
            out_tris: rebuilt.indices.len(),
            selection_digest: {
                let d = selection_log_digest(wsel.hlod.records());
                let mut s = String::with_capacity(64);
                for b in d {
                    s.push_str(&format!("{b:02x}"));
                }
                s
            },
            assemble_ticks: wsel.assemble_ticks,
            budget_stall_frames: wsel.partition.counters().budget_stall_frames,
        };
        let ctx = WpHlodContext {
            world: wsel.partition.world().clone(),
            pack: wp_pack,
            partition: wsel.partition,
            hlod: wsel.hlod,
            thresholds: wsel.thresholds,
            current: wsel.current,
            pending_switch: vec![None; n_cells],
            warmup_frames: wp_opt.warmup_frames.max(1),
            loading_radius_m: wsel.radius,
            inner_radius_m: wp_opt.inner_radius_m.min(wsel.radius),
            next_frame: wsel.select_frame + 1,
            registered: wsel.registered,
            switch_events: Vec::new(),
        };
        return (
            rebuilt,
            Some(GeoApplied {
                prov,
                cluster: None,
                wp: Some((report, ctx)),
                combined: None,
            }),
        );
    }
    // ── 双开态（W2）：两包各自 fail-closed 校验（均对原装配三角汤）──
    let cl_pack = read_cluster_pack(Path::new(&cl_opt.pack_path))
        .unwrap_or_else(|e| fail(&format!("--cluster-lod 簇包读取: {e}")));
    verify_cluster_pack(&cl_pack, &scene)
        .unwrap_or_else(|e| fail(&format!("--cluster-lod 簇包校验 fail-closed: {e}")));
    let wp_pack = read_wp_hlod_pack(Path::new(&wp_opt.pack_path))
        .unwrap_or_else(|e| fail(&format!("--wp-hlod cell 包读取: {e}")));
    verify_wp_pack(&wp_pack, &scene)
        .unwrap_or_else(|e| fail(&format!("--wp-hlod cell 包校验 fail-closed: {e}")));
    let n_src = scene.indices.len();
    // ① WP cell 互斥选层先行（Full/Hlod/Culled;生产机核直调链）。
    let wsel = wp_hlod_select(&scene, &wp_pack, wp_opt);
    // F 域掩码：WP 出帧源三角集（passthrough ∪ Full cell 源）。
    let mut in_f = vec![false; n_src];
    for &s in &wsel.chosen_src {
        in_f[s as usize] = true;
    }
    // ② 簇 cut（全场景选层——cut 语义与单开逐字;Full 域过滤在 ③）。
    let csel = cluster_lod_select(&scene, &cl_pack, cl_opt, in_w, in_h);
    // ③ 组合。DAG 语义前提（组共享多父 DAG,Nanite 同族）：cut 粗簇的源覆盖
    //    集经多父路径可**跨簇部分重叠**——粗簇级"源恰一次"非 DAG 承诺面,
    //    面恰一次由冻结 cut 机制（组共享判定球 + 祖先-后代互斥 + 误差单调）
    //    承载,与单开 on 模式同一信任基（EXR diff 门在案）。组合规则：
    //    - identity = cut 叶级源 ∩ F（叶级恰一次,dup check fail-closed）;
    //    - 粗簇 S_c ⊆ F ⇒ 出帧（面全在 Full 域,与 cell 代理零冲突）;
    //    - 跨界粗簇（0 < |S_c∩F| < |S_c|）⇒ 不出粗簇,S_c∩F **减去已被出帧
    //      粗簇覆盖的部分**后叶级回退（防"粗簇面 + 回退叶"同域双绘）;
    //    - S_c ∩ F = ∅ ⇒ 不出帧（cell 代理/剔除覆盖）。
    let mut identity: Vec<u32> = csel
        .chosen_src
        .iter()
        .copied()
        .filter(|&s| in_f[s as usize])
        .collect();
    let mut coarse_emitted: Vec<(usize, u32)> = Vec::new();
    let mut straddle: Vec<(usize, u32)> = Vec::new();
    let mut straddle_clusters = 0usize;
    let mut straddle_fallback_tris = 0usize;
    let mut coarse_tris = 0usize;
    // 第一趟：emit 判定 + 出帧粗簇覆盖域标记。
    let mut coarse_dom = vec![false; n_src];
    for &(bi, c) in &csel.coarse {
        let covered = cluster_covered_sources(&cl_pack.blocks[bi], c);
        let n_in = covered.iter().filter(|&&s| in_f[s as usize]).count();
        if n_in == covered.len() {
            coarse_tris += cl_pack.blocks[bi].records[c as usize].triangle_count as usize;
            coarse_emitted.push((bi, c));
            for &s in &covered {
                coarse_dom[s as usize] = true;
            }
        } else if n_in > 0 {
            straddle.push((bi, c));
        }
        // n_in == 0：覆盖源全在 Hlod/Culled 域——cell 代理/剔除覆盖,不出帧。
    }
    // 第二趟：跨界粗簇叶级回退（∩F − 出帧粗簇域 − 已回退,恰一次）。
    {
        let mut in_fallback = vec![false; n_src];
        for &(bi, c) in &straddle {
            straddle_clusters += 1;
            for s in cluster_covered_sources(&cl_pack.blocks[bi], c) {
                let i = s as usize;
                if in_f[i] && !coarse_dom[i] && !in_fallback[i] {
                    in_fallback[i] = true;
                    identity.push(s);
                    straddle_fallback_tris += 1;
                }
            }
        }
    }
    identity.sort_unstable();
    if identity.windows(2).any(|w| w[0] == w[1]) {
        fail("geo 组合 identity 源三角重复（覆盖性破坏 = 双绘,fail-closed）");
    }
    // ④ 覆盖机核（fail-closed）：① identity 域与出帧粗簇域零交叠（组合面
    //    新增双绘 = 红）;② identity ∪ 粗簇域 ≡ F（空洞/越域即红）。
    {
        let mut ident_mark = vec![false; n_src];
        for &s in &identity {
            ident_mark[s as usize] = true;
        }
        for i in 0..n_src {
            if ident_mark[i] && coarse_dom[i] {
                fail(&format!(
                    "geo 组合覆盖机核：源三角 {i} 同时被 identity 与出帧粗簇覆盖（双绘）"
                ));
            }
            let covered = ident_mark[i] || coarse_dom[i];
            if covered != in_f[i] {
                fail(&format!(
                    "geo 组合覆盖机核：源三角 {i} 覆盖态 {covered} ≠ WP Full 域 {}（{}）",
                    in_f[i],
                    if in_f[i] { "空洞" } else { "越域出帧" }
                ));
            }
        }
    }
    // ⑤ 统一重建 + provenance。
    let (rebuilt, prov) = geo_rebuild(
        &scene,
        &identity,
        &coarse_emitted,
        Some(&cl_pack),
        &wsel.proxy,
        Some(&wp_pack),
    );
    // ⑥ 组合对拍锚：leaf × full 极限 = off 三角汤逐位一致（fail-closed）。
    if cl_opt.mode == ClusterLodMode::Leaf && wp_opt.mode == WpHlodMode::Full {
        geo_assert_bitexact(&rebuilt, &scene, "geo 组合 leaf×full");
    }
    // ⑦ 报告（两特性各自计数如实登记;out_tris = 组合出帧口径）。
    let wp_proxy_tris: usize = wsel
        .proxy
        .iter()
        .map(|&(ci, level)| {
            wp_pack.cells[ci as usize].as_ref().unwrap().hlod.levels[level as usize].len()
        })
        .sum();
    let stats = GeoCombinedStats {
        identity_tris: identity.len(),
        coarse_emitted: coarse_emitted.len(),
        coarse_tris,
        straddle_clusters,
        straddle_fallback_tris,
        wp_proxy_tris,
        out_tris: rebuilt.indices.len(),
    };
    let cl_report = ClusterLodReport {
        mode: match cl_opt.mode {
            ClusterLodMode::Leaf => "leaf",
            ClusterLodMode::On => "on",
            ClusterLodMode::Off => unreachable!(),
        },
        threshold_px: cl_opt.threshold_px,
        blocks: cl_pack.blocks.len(),
        total_clusters: csel.total_clusters,
        cut_clusters: csel.cut_clusters,
        cut_leaf_clusters: csel.cut_leaf_clusters,
        src_tris: n_src,
        passthrough_tris: cl_pack.passthrough.len(),
        leaf_tris: csel.leaf_tris,
        coarse_tris,
        out_tris: rebuilt.indices.len(),
        resident_pages: cl_opt.resident_pages,
        fallback_count: csel.fallback_count,
    };
    let cells_nonempty = wp_pack.cells.iter().filter(|c| c.is_some()).count();
    let n_cells = wp_pack.cells.len();
    let wp_report = WpHlodReport {
        mode: match wp_opt.mode {
            WpHlodMode::Full => "full",
            WpHlodMode::On => "on",
            WpHlodMode::Off => unreachable!(),
        },
        cells_total: n_cells,
        cells_nonempty,
        cells_resident: wsel.hlod.resident().len(),
        cells_full: wsel.cells_full,
        cells_hlod: wsel.cells_hlod,
        cells_culled: wsel.cells_culled,
        cells_pending: wsel.cells_pending,
        src_tris: n_src,
        passthrough_tris: wp_pack.passthrough.len(),
        full_tris: wsel.chosen_src.len() - wp_pack.passthrough.len(),
        proxy_tris: wp_proxy_tris,
        out_tris: rebuilt.indices.len(),
        selection_digest: {
            let d = selection_log_digest(wsel.hlod.records());
            let mut s = String::with_capacity(64);
            for b in d {
                s.push_str(&format!("{b:02x}"));
            }
            s
        },
        assemble_ticks: wsel.assemble_ticks,
        budget_stall_frames: wsel.partition.counters().budget_stall_frames,
    };
    let ctx = WpHlodContext {
        world: wsel.partition.world().clone(),
        pack: wp_pack,
        partition: wsel.partition,
        hlod: wsel.hlod,
        thresholds: wsel.thresholds,
        current: wsel.current,
        pending_switch: vec![None; n_cells],
        warmup_frames: wp_opt.warmup_frames.max(1),
        loading_radius_m: wsel.radius,
        inner_radius_m: wp_opt.inner_radius_m.min(wsel.radius),
        next_frame: wsel.select_frame + 1,
        registered: wsel.registered,
        switch_events: Vec::new(),
    };
    (
        rebuilt,
        Some(GeoApplied {
            prov,
            cluster: Some((cl_report, cl_pack)),
            wp: Some((wp_report, ctx)),
            combined: Some(stats),
        }),
    )
}

/// 重建产物 vs 源三角汤逐位一致断言（leaf/full/leaf×full 锚共用机核;
/// 判据与 apply_cluster_lod leaf 锚 / apply_wp_hlod full 锚逐字同）。
#[allow(dead_code)]
fn geo_assert_bitexact(rebuilt: &SceneData, scene: &SceneData, tag: &str) {
    if rebuilt.indices.len() != scene.indices.len() {
        fail(&format!(
            "{tag} 三角数漂移: {} ≠ {}",
            rebuilt.indices.len(),
            scene.indices.len()
        ));
    }
    for i in 0..rebuilt.indices.len() {
        let (rt, st) = (rebuilt.indices[i], scene.indices[i]);
        for k in 0..3 {
            let rp = rebuilt.positions[rt[k] as usize].map(f32::to_bits);
            let sp = scene.positions[st[k] as usize].map(f32::to_bits);
            if rp != sp {
                fail(&format!("{tag} 三角 {i} 顶点 {k} 位级漂移"));
            }
        }
        if rebuilt.albedo[i].map(f32::to_bits) != scene.albedo[i].map(f32::to_bits)
            || rebuilt.emission[i].map(f32::to_bits) != scene.emission[i].map(f32::to_bits)
            || rebuilt.tri_mat[i] != scene.tri_mat[i]
        {
            fail(&format!("{tag} 三角 {i} 属性位级漂移"));
        }
    }
}

/// prov 恒等排列判定（leaf×full 极限：全 Src 且 id == 位置 ⇒ 侧表 gather
/// 产物与源逐位一致——W1 恒等排列锚消费面）。
#[allow(dead_code)]
fn geo_prov_is_identity(prov: &[TriProvenance]) -> bool {
    prov.iter()
        .enumerate()
        .all(|(i, p)| matches!(p, TriProvenance::Src(s) if *s as usize == i))
}

/// 逐三角 UV 侧表 gather（W1：装配序 UV sink〔6 f32/tri〕→ 重建序;Src 按源
/// id 取值位保真,代理三角 UV = 0〔tritex 强制 −1 常量面回退,UV 不消费——
/// geo_patch_proxy_tritex 同批施加〕）。恒等排列 ⇒ 产物与源逐位一致。
#[allow(dead_code)]
fn gather_tri_uv(prov: &[TriProvenance], src_uv: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(prov.len() * 6);
    for p in prov {
        match p {
            TriProvenance::Src(s) => {
                let b = *s as usize * 6;
                out.extend_from_slice(&src_uv[b..b + 6]);
            }
            _ => out.extend_from_slice(&[0.0; 6]),
        }
    }
    out
}

/// 代理三角 tritex 强制 −1（W1：g31_tex_load 自重建 tri_mat 派生 tritex——
/// 代理三角 tri_mat = 簇/cell 众数材质,若映射进图集槽会以 UV=0 采样单 texel
/// 出错色;强制 −1 走既有常量面路径〔albedo = cluster/cell 面积加权均值,
/// slab 预调制经 slab_apply 一致施加〕。属性保持简化归 #96 留窗如实登记）。
/// 返回改写数;同步重建 tritex_bytes 与 tex_tris,全空接线 fail-closed。
/// day_0828 F6 双形态回正：本函数 = 原形态（tritex 步幅 1;g34_full_lane 系
/// 消费面,HEAD 逐字恢复零漂移）;heap 步幅 2 形态 = [`geo_patch_proxy_tritex_heap`]。
#[allow(dead_code)]
fn geo_patch_proxy_tritex(tex: &mut G31TexAssets, prov: &[TriProvenance]) -> usize {
    if tex.tritex.len() != prov.len() {
        fail(&format!(
            "geo tritex/prov 长度失配: {} ≠ {}（tritex 须自重建场景派生）",
            tex.tritex.len(),
            prov.len()
        ));
    }
    let mut patched = 0usize;
    for (i, p) in prov.iter().enumerate() {
        if !matches!(p, TriProvenance::Src(_)) && tex.tritex[i] >= 0.0 {
            tex.tritex[i] = -1.0;
            patched += 1;
        }
    }
    if patched > 0 {
        tex.tex_tris = tex.tritex.iter().filter(|&&s| s >= 0.0).count();
        if tex.tex_tris == 0 {
            fail("geo 代理 tritex 补丁后映射三角归零（空接线即红,fail-closed）");
        }
        tex.tritex_bytes = bytes_f32(&tex.tritex);
    }
    patched
}

/// [`geo_patch_proxy_tritex`] 的 day_0828 Phase B heap 形态（tritex 步幅 2
/// [slot, k_tri]——槽号在偶槽,补丁同步清 k_tri〔代理三角 UV=0 ⇒ 密度项无
/// 意义面〕;F6 双形态回正改名,当前零调用面编译保留——heap 臂 geo 接线时
/// 消费）。
#[allow(dead_code)]
fn geo_patch_proxy_tritex_heap(tex: &mut G31TexAssetsHeap, prov: &[TriProvenance]) -> usize {
    if tex.tritex.len() != prov.len() * 2 {
        fail(&format!(
            "geo tritex/prov 长度失配: {} ≠ {}×2（tritex 须自重建场景派生,步幅 2）",
            tex.tritex.len(),
            prov.len()
        ));
    }
    let mut patched = 0usize;
    for (i, p) in prov.iter().enumerate() {
        if !matches!(p, TriProvenance::Src(_)) && tex.tritex[i * 2] >= 0.0 {
            tex.tritex[i * 2] = -1.0;
            tex.tritex[i * 2 + 1] = 0.0;
            patched += 1;
        }
    }
    if patched > 0 {
        tex.tex_tris = tex.tritex.iter().step_by(2).filter(|&&s| s >= 0.0).count();
        if tex.tex_tris == 0 {
            fail("geo 代理 tritex 补丁后映射三角归零（空接线即红,fail-closed）");
        }
        tex.tritex_bytes = bytes_f32(&tex.tritex);
    }
    patched
}

// ---------------------------------------------------------------------------
// G31+ #96 属性保持简化消费面（G38 T4）：代理三角真 corner UV gather +
// tritex −1 强制回退退役。既有 gather_tri_uv / geo_patch_proxy_tritex(_heap)
// 函数体 0 改动编译保留;下述 _attrs/_v2 形态 = g34 车道新消费面——
// v1 资产（无 UV 段）输入下行为与旧形态**逐位一致**（gather 写 [0;6] +
// 补丁置 −1 = 旧语义等价）,v2 资产（RXCP 簇 UV 表 / RXHL v2 corner UV）
// 输入下代理三角带真 UV 走与 Src 三角同一图集采样路径（kernel 0 改动:
// tritex ≥ 0 即 tex_gate 开,g31_texture_gi.rx 179-183 既有语义）。
// ---------------------------------------------------------------------------

/// 逐三角 UV 侧表 gather 属性形态（#96）：Src 按源 id 取值位保真（与
/// [`gather_tri_uv`] 同字面）;代理三角自资产 UV 源取真 corner UV——
/// ClusterCoarse = 簇包 v2 顶点 UV 平行表按簇局部索引取三元组（与
/// geo_rebuild 顶点取数同式）,WpProxy = RXHL v2 逐层逐三角 corner UV;
/// 无 UV 资产臂（v1 包,UV 源 = None）回落 [0;6]（旧语义等价,tritex
/// 补丁面维持 −1 常量回退）。代理三角段内序 = geo_rebuild 尾接序
/// （簇内序/层内序连续段——prov 相邻同源计数器还原段内三角号,段界
/// 即重置;越界 = prov/资产失配 fail-closed）。恒等排列 ⇒ 产物与源逐位一致。
#[allow(dead_code)] // #96:g34_full_lane / g34_2_hzb 消费面（include 共享体,诚实标注）
fn gather_tri_uv_attrs(
    prov: &[TriProvenance],
    src_uv: &[f32],
    cl_pack: Option<&ClusterPack>,
    wp_pack: Option<&WpHlodPack>,
) -> Vec<f32> {
    let mut out = Vec::with_capacity(prov.len() * 6);
    // 代理连续段内三角号（geo_rebuild 尾接不变量:同 (块,簇)/(cell,层) 的
    // 代理三角恰一段连续出帧,prov 相邻相等即段内推进）。
    let mut prev: Option<TriProvenance> = None;
    let mut k = 0usize;
    for p in prov {
        match *p {
            TriProvenance::Src(s) => {
                let b = s as usize * 6;
                out.extend_from_slice(&src_uv[b..b + 6]);
            }
            TriProvenance::ClusterCoarse { block, cluster } => {
                k = if prev == Some(*p) { k + 1 } else { 0 };
                let mut row = [0.0f32; 6];
                if let Some(cp) = cl_pack
                    && let Some(uvtab) = cp
                        .blocks_vertex_uv
                        .as_ref()
                        .map(|v| &v[block as usize])
                {
                    let b = &cp.blocks[block as usize];
                    let r = &b.records[cluster as usize];
                    if k >= r.triangle_count as usize {
                        fail(&format!(
                            "gather_tri_uv_attrs 簇代理段内序越界: k={k} ≥ 簇三角数 {}（块 {block} 簇 {cluster};prov 连续段不变量破坏）",
                            r.triangle_count
                        ));
                    }
                    let ti = r.triangle_offset as usize + 3 * k;
                    for c in 0..3 {
                        let li =
                            b.triangle_indices[ti + c] as usize + r.vertex_offset as usize;
                        row[c * 2] = uvtab[li][0];
                        row[c * 2 + 1] = uvtab[li][1];
                    }
                }
                out.extend_from_slice(&row);
            }
            TriProvenance::WpProxy { cell, level } => {
                k = if prev == Some(*p) { k + 1 } else { 0 };
                let mut row = [0.0f32; 6];
                if let Some(c) = wp_pack.and_then(|wp| wp.cells[cell as usize].as_ref())
                    && let Some(uvl) = c.hlod.levels_uv.as_ref()
                {
                    let rows = &uvl[level as usize];
                    if k >= rows.len() {
                        fail(&format!(
                            "gather_tri_uv_attrs cell 代理段内序越界: k={k} ≥ 层三角数 {}（cell {cell} L{level};prov 连续段不变量破坏）",
                            rows.len()
                        ));
                    }
                    row = rows[k];
                }
                out.extend_from_slice(&row);
            }
        }
        prev = Some(*p);
    }
    out
}

/// 代理三角 tritex 补丁 v2（#96 退役面）：仅对**无 UV 数据**的代理三角
/// 置 −1（v1 资产臂——UV=0 采样错色防线维持,走常量面回退）;带 UV 的
/// 代理三角（v2 资产臂）保留 tri_mat 派生槽号,与 Src 三角同一图集采样
/// 路径（gather_tri_uv_attrs 已供真 corner UV;众数材质不在 top-N 图集
/// 者 tritex 本就 −1,常量面兜底不变）。返回改写数;同步重建 tritex_bytes
/// 与 tex_tris,全空接线 fail-closed（[`geo_patch_proxy_tritex`] 同律）。
#[allow(dead_code)] // #96:g34_full_lane / g34_2_hzb 消费面（include 共享体,诚实标注）
fn geo_patch_proxy_tritex_v2(
    tex: &mut G31TexAssets,
    prov: &[TriProvenance],
    cl_pack: Option<&ClusterPack>,
    wp_pack: Option<&WpHlodPack>,
) -> usize {
    if tex.tritex.len() != prov.len() {
        fail(&format!(
            "geo tritex/prov 长度失配: {} ≠ {}（tritex 须自重建场景派生）",
            tex.tritex.len(),
            prov.len()
        ));
    }
    let mut patched = 0usize;
    for (i, p) in prov.iter().enumerate() {
        let has_uv = match *p {
            TriProvenance::Src(_) => continue,
            TriProvenance::ClusterCoarse { .. } => {
                cl_pack.is_some_and(|cp| cp.blocks_vertex_uv.is_some())
            }
            TriProvenance::WpProxy { cell, .. } => wp_pack
                .and_then(|wp| wp.cells[cell as usize].as_ref())
                .is_some_and(|c| c.hlod.levels_uv.is_some()),
        };
        if !has_uv && tex.tritex[i] >= 0.0 {
            tex.tritex[i] = -1.0;
            patched += 1;
        }
    }
    if patched > 0 {
        tex.tex_tris = tex.tritex.iter().filter(|&&s| s >= 0.0).count();
        if tex.tex_tris == 0 {
            fail("geo 代理 tritex 补丁后映射三角归零（空接线即红,fail-closed）");
        }
        tex.tritex_bytes = bytes_f32(&tex.tritex);
    }
    patched
}

/// [`geo_patch_proxy_tritex_v2`] 的 heap 形态（tritex 步幅 2 [slot, k_tri];
/// #96 同律:仅无 UV 数据的代理三角置 −1 并清 k_tri,带 UV 者保留槽号与
/// 密度项〔UV gather 已供真值,k_tri 自重建场景派生有效〕。当前零调用面
/// 编译保留——heap 臂 geo 接线时消费,[`geo_patch_proxy_tritex_heap`] 同待遇）。
#[allow(dead_code)]
fn geo_patch_proxy_tritex_heap_v2(
    tex: &mut G31TexAssetsHeap,
    prov: &[TriProvenance],
    cl_pack: Option<&ClusterPack>,
    wp_pack: Option<&WpHlodPack>,
) -> usize {
    if tex.tritex.len() != prov.len() * 2 {
        fail(&format!(
            "geo tritex/prov 长度失配: {} ≠ {}×2（tritex 须自重建场景派生,步幅 2）",
            tex.tritex.len(),
            prov.len()
        ));
    }
    let mut patched = 0usize;
    for (i, p) in prov.iter().enumerate() {
        let has_uv = match *p {
            TriProvenance::Src(_) => continue,
            TriProvenance::ClusterCoarse { .. } => {
                cl_pack.is_some_and(|cp| cp.blocks_vertex_uv.is_some())
            }
            TriProvenance::WpProxy { cell, .. } => wp_pack
                .and_then(|wp| wp.cells[cell as usize].as_ref())
                .is_some_and(|c| c.hlod.levels_uv.is_some()),
        };
        if !has_uv && tex.tritex[i * 2] >= 0.0 {
            tex.tritex[i * 2] = -1.0;
            tex.tritex[i * 2 + 1] = 0.0;
            patched += 1;
        }
    }
    if patched > 0 {
        tex.tex_tris = tex.tritex.iter().step_by(2).filter(|&&s| s >= 0.0).count();
        if tex.tex_tris == 0 {
            fail("geo 代理 tritex 补丁后映射三角归零（空接线即红,fail-closed）");
        }
        tex.tritex_bytes = bytes_f32(&tex.tritex);
    }
    patched
}

/// 节点段重导出（W1：装配序 SceneNodeGroup〔连续半开区间〕→ 重建序;
/// Src 段升序保持节点连续性——同节点源三角在重建 identity 前缀内仍相邻;
/// 代理尾段按 (块)/(cell) 成组。AABB 自重建几何精确重算（三角 ⊆ AABB 精确
/// 包含维持,HZB 剔除保守方向零假阳性）。恒等排列 ⇒ 产物与源逐段一致
/// （tri_offset/tri_count 同值;AABB 同派生式逐位）。
#[allow(dead_code)]
fn regroup_nodes(
    prov: &[TriProvenance],
    src_groups: &[SceneNodeGroup],
    rebuilt: &SceneData,
) -> Vec<SceneNodeGroup> {
    #[derive(PartialEq, Clone, Copy)]
    enum SegKey {
        Node(usize),
        Coarse(u32),
        Cell(u32),
    }
    let mut out: Vec<SceneNodeGroup> = Vec::new();
    let mut cur_key: Option<SegKey> = None;
    let mut cur_start = 0usize;
    let mut cur_min = [f32::INFINITY; 3];
    let mut cur_max = [f32::NEG_INFINITY; 3];
    let mut node_cursor = 0usize;
    let flush =
        |out: &mut Vec<SceneNodeGroup>, start: usize, end: usize, mn: [f32; 3], mx: [f32; 3]| {
            if end > start {
                out.push(SceneNodeGroup {
                    tri_offset: start as u32,
                    tri_count: (end - start) as u32,
                    aabb_min: mn,
                    aabb_max: mx,
                });
            }
        };
    for (i, p) in prov.iter().enumerate() {
        let key = match p {
            TriProvenance::Src(s) => {
                let id = *s as usize;
                while node_cursor < src_groups.len()
                    && id >= (src_groups[node_cursor].tri_offset
                        + src_groups[node_cursor].tri_count) as usize
                {
                    node_cursor += 1;
                }
                if node_cursor >= src_groups.len()
                    || id < src_groups[node_cursor].tri_offset as usize
                {
                    fail(&format!(
                        "regroup 源三角 {id} 不在任何节点段内（组表与装配序失配）"
                    ));
                }
                SegKey::Node(node_cursor)
            }
            TriProvenance::ClusterCoarse { block, .. } => SegKey::Coarse(*block),
            TriProvenance::WpProxy { cell, .. } => SegKey::Cell(*cell),
        };
        if cur_key != Some(key) {
            flush(&mut out, cur_start, i, cur_min, cur_max);
            cur_key = Some(key);
            cur_start = i;
            cur_min = [f32::INFINITY; 3];
            cur_max = [f32::NEG_INFINITY; 3];
        }
        let t = rebuilt.indices[i];
        for &vi in &t {
            let v = rebuilt.positions[vi as usize];
            for k in 0..3 {
                cur_min[k] = cur_min[k].min(v[k]);
                cur_max[k] = cur_max[k].max(v[k]);
            }
        }
    }
    flush(&mut out, cur_start, prov.len(), cur_min, cur_max);
    out
}
// 以下全部仅 slab 模式消费，非 slab 路径逐字不触。消费冻结面：
// kernels/g29_slab.rx 本体 0-byte（dispatch [16,1,1] 逐槽单 invocation）+
// material/slab.rs::total_reflectance host 金标准 f64 直调 0-byte；
// MaterialClosure 32B / graph/types.rs 零触碰——slab 不经 MaterialClosure，
// 经侧表资产 → 逐三角 albedo 预调制进入既有 mats SSBO 面）
// ---------------------------------------------------------------------------

/// 侧表槽数（G29 M-b 16 槽 ABI 字面；资产 JSON 与 kernel 参数面同源）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
const SLAB_N_SLOTS: usize = 16;
/// 资产 schema 字面（fail-closed 闭集校验）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
const SLAB_ASSET_SCHEMA: &str = "rurix.g31.slab_side_table_asset.v1";

/// slab 侧表生产资产（场景/资产文件驱动——G29 M-b bin-local 合成件的资产化
/// 升级面；16 槽 [rc, ab] f32 + glTF material_index → 槽映射 + ABI digest
/// 互核篡改即拒）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
struct SlabSideTableAsset {
    scene_id: String,
    /// 16 槽 [rc, ab]（f32 位级 = M-b 生成律 rc_k=k/15·0.95、ab_k=(15−k)/15）。
    slots: [[f32; 2]; SLAB_N_SLOTS],
    /// glTF material_index → 槽 k（≤16 映射；非映射材质走既有单层面 0-byte）。
    material_slots: Vec<(u32, u8)>,
    abi_digest: String,
    path: String,
}

/// 16 槽 ABI digest（16 × [rc f32 LE, ab f32 LE] = 128 字节 sha256）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn slab_abi_digest(slots: &[[f32; 2]; SLAB_N_SLOTS]) -> String {
    let mut bytes = Vec::with_capacity(SLAB_N_SLOTS * 8);
    for [rc, ab] in slots {
        bytes.extend_from_slice(&rc.to_le_bytes());
        bytes.extend_from_slice(&ab.to_le_bytes());
    }
    format!("sha256:{}", sha256_hex(&bytes))
}

/// 资产加载 + 闭集校验（字段闭集/类型/域 [0,1]/槽号唯一材质唯一/ABI digest
/// 互核——篡改即 Err fail-closed）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn slab_load_asset(path: &str) -> Result<SlabSideTableAsset, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("读 slab 侧表资产 {path}: {e}"))?;
    let doc = json_parse(&text)?;
    closed(
        "slab_side_table",
        &doc,
        &[
            "schema",
            "scene_id",
            "n_slots",
            "abi",
            "slots",
            "material_slots",
            "evaluation_semantics",
            "provenance",
        ],
    )?;
    let schema = as_str("schema", doc.get("schema").unwrap())?;
    if schema != SLAB_ASSET_SCHEMA {
        return Err(cerr(format!("slab 资产 schema 非法: {schema}")));
    }
    let scene_id = as_str("scene_id", doc.get("scene_id").unwrap())?.to_owned();
    let n = as_u64("n_slots", doc.get("n_slots").unwrap())? as usize;
    if n != SLAB_N_SLOTS {
        return Err(cerr(format!("n_slots {n} ≠ {SLAB_N_SLOTS}（16 槽 ABI 字面）")));
    }
    let slots_j = doc.get("slots").unwrap().as_array().unwrap();
    if slots_j.len() != SLAB_N_SLOTS {
        return Err(cerr(format!("slots 行数 {} ≠ {SLAB_N_SLOTS}", slots_j.len())));
    }
    let mut slots = [[0.0f32; 2]; SLAB_N_SLOTS];
    for (k, s) in slots_j.iter().enumerate() {
        closed(&format!("slots[{k}]"), s, &["k", "rc", "ab"])?;
        let kj = as_u64("k", s.get("k").unwrap())? as usize;
        if kj != k {
            return Err(cerr(format!("slots[{k}].k={kj} 乱序（槽序 = 下标序 ABI）")));
        }
        let rc = as_f64("rc", s.get("rc").unwrap())? as f32;
        let ab = as_f64("ab", s.get("ab").unwrap())? as f32;
        if !(0.0..=1.0).contains(&rc) || !(0.0..=1.0).contains(&ab) {
            return Err(cerr(format!("slots[{k}] rc/ab 越域 [0,1]: {rc}/{ab}")));
        }
        slots[k] = [rc, ab];
    }
    let abi = doc.get("abi").unwrap();
    let abi_digest = as_str("abi.abi_digest", abi.get("abi_digest").ok_or_else(|| {
        cerr("abi.abi_digest 缺失")
    })?)?
    .to_owned();
    let computed = slab_abi_digest(&slots);
    if computed != abi_digest {
        return Err(cerr(format!(
            "slab 资产 ABI digest 不符（篡改即拒）: 在档 {abi_digest} vs 重算 {computed}"
        )));
    }
    let ms_j = doc
        .get("material_slots")
        .unwrap()
        .as_array()
        .ok_or_else(|| cerr("material_slots 非数组"))?;
    if ms_j.is_empty() {
        return Err(cerr("material_slots 空映射（slab 模式须有 ≥1 映射材质）"));
    }
    let mut material_slots: Vec<(u32, u8)> = Vec::with_capacity(ms_j.len());
    for (i, m) in ms_j.iter().enumerate() {
        closed(
            &format!("material_slots[{i}]"),
            m,
            &["material_index", "material_name", "slot", "slab_class", "note"],
        )?;
        let mi = as_u64("material_index", m.get("material_index").unwrap())? as u32;
        let slot = as_u64("slot", m.get("slot").unwrap())?;
        if slot >= SLAB_N_SLOTS as u64 {
            return Err(cerr(format!("material_slots[{i}].slot {slot} 越 16 槽")));
        }
        if material_slots.iter().any(|(x, _)| *x == mi) {
            return Err(cerr(format!("material_slots[{i}].material_index {mi} 重复映射")));
        }
        material_slots.push((mi, slot as u8));
    }
    Ok(SlabSideTableAsset {
        scene_id,
        slots,
        material_slots,
        abi_digest,
        path: path.to_owned(),
    })
}

/// 侧表求值报告（evidence 登记面；host f64 金标准 + device f32 双臂 + 有限性
/// 一等断言先于聚合 + 逐槽对拍 p100）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
struct SlabEval {
    host_r: [f64; SLAB_N_SLOTS],
    device_r: [f32; SLAB_N_SLOTS],
    parity_p100: f64,
    eval_ms: f64,
    device_digest: String,
    host_digest: String,
}

/// host 金标准逐槽直调（material/slab.rs::total_reflectance f64；0-byte 冻结
/// 面只消费不改写——host 参考臂 = 生产接线态对拍基准）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn slab_eval_host(slots: &[[f32; 2]; SLAB_N_SLOTS]) -> [f64; SLAB_N_SLOTS] {
    let mut out = [0.0f64; SLAB_N_SLOTS];
    for (k, [rc, ab]) in slots.iter().enumerate() {
        out[k] = rurix_render::material::slab::SlabStack::new(*rc, *ab).total_reflectance();
    }
    out
}

/// device 逐槽求值（kernels/g29_slab.rx SPV 经 vk::run_compute 单 dispatch
/// [16,1,1]；samples 32 f32 + params 8 f32〔[0]=n=16 [1]=red_bias=0 [2..=7]
/// reserved 恒 0〕+ out 16 f32——参数面与 g29_slab_device 逐字同源；host 单源
/// 原字节上传，device 不重算槽参数）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn slab_eval_device(
    slots: &[[f32; 2]; SLAB_N_SLOTS],
    spv_path: &str,
) -> Result<([f32; SLAB_N_SLOTS], f64), String> {
    if !vk::vulkan_available() {
        return Err("vulkan loader 不可用".into());
    }
    let bytes = std::fs::read(spv_path).map_err(|e| format!("读 slab SPV {spv_path}: {e}"))?;
    if bytes.len() % 4 != 0 {
        return Err("slab SPIR-V 字节数非 4 对齐".into());
    }
    let spv: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let entry = vk::entry_point_name(&spv).ok_or("slab SPV 无 OpEntryPoint")?;
    let mut samples = Vec::with_capacity(SLAB_N_SLOTS * 2);
    for [rc, ab] in slots {
        samples.push(*rc);
        samples.push(*ab);
    }
    let mut params = vec![SLAB_N_SLOTS as f32, 0.0f32];
    params.resize(8, 0.0);
    let mut bufs = vec![
        bytes_f32(&samples),
        bytes_f32(&params),
        vec![0u8; SLAB_N_SLOTS * 4],
    ];
    let t0 = std::time::Instant::now();
    vk::run_compute(&spv, &entry, &mut bufs, &[], [SLAB_N_SLOTS as u32, 1, 1])
        .map_err(|e| format!("slab 侧表 device dispatch 失败: {e}"))?;
    let eval_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let out = read_f32(&bufs[2]);
    let mut device_r = [0.0f32; SLAB_N_SLOTS];
    device_r.copy_from_slice(&out[..SLAB_N_SLOTS]);
    Ok((device_r, eval_ms))
}

/// 双臂求值 + 对拍（判据⓪有限性一等断言先于聚合：任一非有限即 Err fail-closed；
/// p100 = 逐槽 |device f32 − host f64| 绝对差最大值——G29 M-b 逐槽对拍口径）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn slab_evaluate(asset: &SlabSideTableAsset, spv_path: &str) -> Result<SlabEval, String> {
    let host_r = slab_eval_host(&asset.slots);
    let (device_r, eval_ms) = slab_eval_device(&asset.slots, spv_path)?;
    if let Some(k) = device_r.iter().position(|x| !x.is_finite()) {
        return Err(format!(
            "slab 侧表判据⓪失败: 槽 {k} device 输出非有限（有限性一等断言先于聚合，RFC-0046 §1.4 F3 同律）"
        ));
    }
    let mut p100 = 0.0f64;
    for k in 0..SLAB_N_SLOTS {
        let d = (f64::from(device_r[k]) - host_r[k]).abs();
        if d > p100 {
            p100 = d;
        }
    }
    let mut dev_bytes = Vec::with_capacity(SLAB_N_SLOTS * 4);
    for x in device_r {
        dev_bytes.extend_from_slice(&x.to_le_bytes());
    }
    let mut host_bytes = Vec::with_capacity(SLAB_N_SLOTS * 8);
    for x in host_r {
        host_bytes.extend_from_slice(&x.to_le_bytes());
    }
    Ok(SlabEval {
        host_r,
        device_r,
        parity_p100: p100,
        eval_ms,
        device_digest: format!("sha256:{}", sha256_hex(&dev_bytes)),
        host_digest: format!("sha256:{}", sha256_hex(&host_bytes)),
    })
}

/// 逐三角 slab 施加（albedo[c] = albedo[c] × R_slot，f32 乘；emission 通道
/// 0-byte 不触；非映射材质/灯面三角走既有单层面 0-byte）。arm_r = 逐槽 R f32
/// （device 臂 = device 输出原字节；host 臂 = host f64 as f32 舍入）。返回
/// slab 三角计数（登记面）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn slab_apply(
    scene: &mut SceneData,
    asset: &SlabSideTableAsset,
    arm_r: &[f32; SLAB_N_SLOTS],
) -> usize {
    let mut n = 0usize;
    for k in 0..scene.indices.len() {
        let mi = scene.tri_mat[k];
        if mi == SLAB_TRI_NONE {
            continue;
        }
        if let Some((_, slot)) = asset.material_slots.iter().find(|(x, _)| *x == mi) {
            let r = arm_r[*slot as usize];
            scene.albedo[k][0] *= r;
            scene.albedo[k][1] *= r;
            scene.albedo[k][2] *= r;
            n += 1;
        }
    }
    n
}

/// 逐槽 R f32 选臂（device = device 输出原值；host = host f64 金标准舍入 f32——
/// host 参考臂渲染面：同一 f32 施加路径,仅 R 来源换金标准）。
#[allow(dead_code)] // G31+ 波 B Task B3:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn slab_arm_r(eval: &SlabEval, arm: &str) -> [f32; SLAB_N_SLOTS] {
    let mut out = [0.0f32; SLAB_N_SLOTS];
    for k in 0..SLAB_N_SLOTS {
        out[k] = if arm == "host" {
            eval.host_r[k] as f32
        } else {
            eval.device_r[k]
        };
    }
    out
}

/// G34 合并语义：贴图 slab 材质三角 = 采样 albedo ×（mod × R_slot）——R_slot
/// 装配期预乘进 texmeta 槽 mod 三项（texmeta 槽 k 于 [8+k*8+4..+3] = mod_rgb；
/// kernel 采样块与 fork A 逐字同式,samp × mod 单次 f32 乘操作序不变 ⇒ B4
/// 探针位级对拍锚同构维持）。非 slab 映射材质 R_slot ≡ 1 不预乘（texmeta
/// 与 fork A 逐位同值）。`scene_tri_mat` 面不消费（映射粒度 = 材质）。返回
/// 预调制槽计数（登记面；0 = 纹理×slab 映射无交集,如实登记）。
#[allow(dead_code)] // G34 合流:g34_full_lane 独消费面(其余 bin 未消费,诚实标注)
fn g34_slab_premod_texmeta(
    tex: &mut G31TexAssets,
    asset: &SlabSideTableAsset,
    arm_r: &[f32; SLAB_N_SLOTS],
) -> usize {
    let mut n = 0usize;
    for (k, s) in tex.slots.iter().enumerate() {
        if let Some((_, slot)) = asset
            .material_slots
            .iter()
            .find(|(x, _)| *x == s.material_index)
        {
            let r = arm_r[*slot as usize];
            let sb = 8 + k * 8;
            tex.texmeta[sb + 4] *= r;
            tex.texmeta[sb + 5] *= r;
            tex.texmeta[sb + 6] *= r;
            n += 1;
        }
    }
    tex.texmeta_bytes = bytes_f32(&tex.texmeta);
    n
}

// ---------------------------------------------------------------------------
// G31+ 波 B Task B4 纹理采样管线进生产场景（--textures on 面；静态面 0-byte——
// 以下全部仅 textures on 消费，off 路径逐字不触。资产面盘点结论（bistro-
// interior 70 材质）：albedo 贴图 70/70 可获得（baseColorTexture 全 DDS，
// BC1×54 + BC3×20）/ normal 70/70（BC5；glTF 无 TANGENT 属性 ⇒ 切线空间
// 缺失，法线贴图着色面登记后续）/ rough-metal 贴图 0/70（无
// metallicRoughnessTexture，仅 factor 常量且生产着色模型为 Lambert 无
// rough/metal 消费槽——缺面如实登记）。生产消费面 = albedo 贴图采样。
// 采样接线形态（spec 阶段矩阵约束面，RXS-0223 §4.0-2：Texture2D/Sampler/
// TextureRw2D 阶段列 = fragment/vertex/raygen，compute kernel 零 image
// 绑定——G11.3 链/规格面 0-byte 纪律下不扩阶段）：
//   ① 生产车道 kernel（g31_texture_gi.rx）= SSBO 图集（u32 打包 RGBA8）+
//      256 项 srgb→linear LUT + 手动双线性（G26 framegen 生产先例同律——
//      采样语义 f32 逐字同源、host 参考位级对拍；NoContraction 注入面 =
//      驱动 FMA 收缩禁面）；
//   ② sampler 求值腿（装配期一次性）= 真 GPU 纹理对象（image/view 经
//      vk::GraphicsResource::Texture2D + sampler 经 sampler.rs SamplerDesc→
//      vk_fields→VkSampler）硬件 `.sample_lod` 采样 vs host 参考（srgb 域
//      同式双线性 + 8-bit 量化镜像）对拍——sampler.rs 面消费点；
//   ③ 探针 kernel（g31_texture_probe.rx）= 生产采样块隔离对拍面（vk::
//      run_compute 单 dispatch，B3 slab 求值同构体例）。
// ---------------------------------------------------------------------------

/// B4 映射材质数（三角数降序 top-N 闭集律法；其余材质走既有常量面 0-byte）。
/// day_0828 F6 双形态回正：原形态常量原值恢复（g34_full_lane 系经
/// [`g31_tex_load`] 消费,top-12 律法为其门锚冻结语义）;heap 70 全覆盖档 =
/// [`G31_TEX_N_MAPPED_HEAP`]。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
const G31_TEX_N_MAPPED: usize = 12;
/// day_0828 Phase B heap 档映射材质数（12→70 全覆盖——bistro-interior 70
/// 材质全数入 heap，「均值 albedo 马赛克」修复面;仅 [`g31_tex_load_heap`]
/// 消费）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
const G31_TEX_N_MAPPED_HEAP: usize = 70;
/// B4 源纹理尺寸上限（pow2 fail-closed 域前提；bistro 全集 ≤2048）。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
const G31_TEX_TILE: u32 = 2048;
/// day_0828 Phase B 存储基级 cap（>cap 源从对应 DDS 源 mip 起搬——2048² 12 级
/// 链从 mip1 起 = 1024² 11 级；零重采样，美术原始 mip 直搬；atlas_design.md
/// §4 cap-1024 档 ≈ 283 MiB）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
const G31_TEX_CAP: u32 = 1024;
/// day_0828 Phase B heap 头表逐槽 mip 槽位数（cap-1024 → ≤11 级 + 裕量；
/// kernel 头表寻址字面 13 同源）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
const G31_TEX_MIP_SLOTS: usize = 13;
/// day_0828 Phase B heap 字节 fail-closed 上限（maxStorageBufferRange 保守
/// 界：4070 Ti 报 4 GiB，本仓无 device limits 查询管道 ⇒ 2 GiB 常量断言 +
/// 真查询留交接；cap-1024 档实测 ~283 MiB 远低于界）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
const G31_TEX_HEAP_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// B4 图集列数（day_0828 Phase B heap 化后仅 SVT 旧形态推导消费——SVT 臂已
/// fail-closed 互斥，编译面保留）。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
const G31_TEX_GRID_COLS: u32 = 4;
/// B4 探针 UV 数/纹理（16 网格 + 4 精确边缘 + 4 wrap 域;确定性闭集律法,
/// evidence 登记面）。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
const G31_TEX_PROBES_PER_SLOT: usize = 24;
/// day_0828 Phase B 探针 mip 抽样级数/槽（律法 = {0, mips/2, mips−1} 去重；
/// SSBO 腿 heap 寻址逐级对拍面）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
const G31_TEX_PROBE_MIPS: usize = 3;

/// B4 资产面盘点（glTF 材质/属性计数闭集;evidence 登记面）。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
struct G31TexCensus {
    materials_total: usize,
    with_base_color_texture: usize,
    with_normal_texture: usize,
    with_metallic_roughness_texture: usize,
    primitives_total: usize,
    primitives_with_texcoord0: usize,
    primitives_with_tangent: usize,
}

/// B4 映射槽（top-N 律法一行;材质名/索引 glTF 互核 + G11.3 manifest 互核面）。
/// day_0828 F6 双形态回正：原形态原字段恢复（g34_full_lane 系经
/// [`g31_tex_load`] 消费）;heap 形态 = [`G31TexSlotHeap`]。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
struct G31TexSlot {
    material_index: u32,
    material_name: String,
    tris: usize,
    texture_uri: String,
    width: u32,
    height: u32,
    dds_format: String,
    /// G11.3 manifest 登记的源文件 digest（互核面;manifest 缺条目 = None）。
    manifest_source_digest: Option<String>,
    /// 本 bin 解码 RGBA8 digest（== manifest rgba8_digest 即 G11.3 链互核绿）。
    rgba8_digest: String,
    manifest_rgba8_digest: Option<String>,
    origin_x: u32,
    origin_y: u32,
    mod_rgb: [f32; 3],
}

/// B4 映射槽 day_0828 Phase B heap 形态（F6 双形态回正改名）：width/height =
/// **存储基级**尺寸（cap-1024 档）；src_width/src_height = DDS 源 mip0 尺寸
/// （manifest 互核域）；rgba8_digest 语义不变 = 源 mip0 全分辨率解码 digest
/// （G11.3 锚不动）；mip_digests = 逐存储级 rgba8 digest（新 evidence 字段）；
/// origin_x/origin_y 废弃恒 0（SVT 旧形态编译面保留——SVT 臂已 fail-closed
/// 互斥）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
struct G31TexSlotHeap {
    material_index: u32,
    material_name: String,
    tris: usize,
    texture_uri: String,
    width: u32,
    height: u32,
    src_width: u32,
    src_height: u32,
    dds_format: String,
    /// G11.3 manifest 登记的源文件 digest（互核面;manifest 缺条目 = None）。
    manifest_source_digest: Option<String>,
    /// 本 bin 解码源 mip0 RGBA8 digest（== manifest rgba8_digest 即 G11.3 链互核绿）。
    rgba8_digest: String,
    manifest_rgba8_digest: Option<String>,
    /// 存储 mip 级数（cap 起级到链尾;texmeta[sb+7] 同源）。
    mip_count: u32,
    /// 逐存储级 RGBA8 digest（heap 装入序;新 evidence 字段）。
    mip_digests: Vec<String>,
    /// DDS 源链短于完整链（mips < log2(max(w,h))+1）按可用级截断登记。
    mip_truncated: bool,
    origin_x: u32,
    origin_y: u32,
    mod_rgb: [f32; 3],
}

/// B4 贴图资产面（装配期一次性构建;SSBO 字节面与 f32/u32 面同源派生）。
/// day_0828 F6 双形态回正：原形态原字段恢复（atlas = 2D 网格图集;
/// g34_full_lane 系经 [`g31_tex_load`] 消费）;heap 形态 = [`G31TexAssetsHeap`]。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
struct G31TexAssets {
    slots: Vec<G31TexSlot>,
    census: G31TexCensus,
    atlas_w: u32,
    atlas_h: u32,
    /// 图集 u32 打包 RGBA8（R|G<<8|B<<16|A<<24;atlas_w×atlas_h）。
    atlas: Vec<u32>,
    atlas_bytes: Vec<u8>,
    atlas_digest: String,
    linlut: [f32; 256],
    linlut_bytes: Vec<u8>,
    linlut_digest: String,
    texmeta: Vec<f32>,
    texmeta_bytes: Vec<u8>,
    tritex: Vec<f32>,
    tritex_bytes: Vec<u8>,
    texuv_bytes: Vec<u8>,
    /// 逐槽解码 RGBA8（行主序;sampler 腿纹理对象源 + host srgb 域参考面）。
    slots_rgba8: Vec<Vec<u8>>,
    tex_tris: usize,
    eval_ms: f64,
}

/// B4 贴图资产面 day_0828 Phase B heap 形态（F6 双形态回正改名）：atlas =
/// **一维 texel heap**（u32 偏移头表 [slot×13+mip] + 逐槽逐级连续 texel
/// 段）；atlas_w/atlas_h 废弃恒 0（SVT 旧形态编译面保留）；
/// heap_header_entries/heap_texels 新增登记面。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
struct G31TexAssetsHeap {
    slots: Vec<G31TexSlotHeap>,
    census: G31TexCensus,
    atlas_w: u32,
    atlas_h: u32,
    /// texel heap：u32 偏移头表（slot_count×13 项,绝对 texel 下标）+ 逐槽
    /// 逐级 u32 打包 RGBA8（R|G<<8|B<<16|A<<24）texel 段。
    atlas: Vec<u32>,
    atlas_bytes: Vec<u8>,
    atlas_digest: String,
    /// heap 头表项数（slot_count × G31_TEX_MIP_SLOTS）。
    heap_header_entries: usize,
    /// heap 总 u32 数（头表 + texel 段;×4 = SSBO 字节）。
    heap_texels: usize,
    linlut: [f32; 256],
    linlut_bytes: Vec<u8>,
    linlut_digest: String,
    texmeta: Vec<f32>,
    texmeta_bytes: Vec<u8>,
    tritex: Vec<f32>,
    tritex_bytes: Vec<u8>,
    texuv_bytes: Vec<u8>,
    /// 逐槽解码 RGBA8（行主序;sampler 腿纹理对象源 + host srgb 域参考面）。
    slots_rgba8: Vec<Vec<u8>>,
    tex_tris: usize,
    eval_ms: f64,
}

/// DDS BC1（DXT1）/BC3（DXT5）**全 mip 链**解码 → 逐级 RGBA8 行主序
/// （rurix-asset bcdec::decode_dds 的 bin-local 镜像——G11.3 确定性锚在案
/// 消费面:逐槽源 mip0 rgba8 digest 与 milestones/g11/
/// g11_3_dds_transcode_manifest.json 登记值互核,bcdec 行为漂移即红;BC5/BC7
/// 等闭集外 fail-closed 显式拒绝）。day_0828 Phase B：mip0-only → 逐级解码
/// （DDS 级段连续存储最大级在前,块数按级折半;dwMipMapCount@28,0 视作 1;
/// 级体截断 fail-closed）。返回 (width, height, format,
/// levels[(w,h,rgba8)..])。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_dds_decode_rgba8_mips(
    bytes: &[u8],
) -> Result<(u32, u32, &'static str, Vec<(u32, u32, Vec<u8>)>), String> {
    if bytes.len() < 128 || &bytes[0..4] != b"DDS " {
        return Err("非 DDS magic/头截断".into());
    }
    let rd = |off: usize| -> Result<u32, String> {
        let b = bytes
            .get(off..off + 4)
            .ok_or_else(|| format!("DDS 头截断 @0x{off:x}"))?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if rd(4)? != 124 {
        return Err("DDS header.size ≠ 124".into());
    }
    let height = rd(12)?;
    let width = rd(16)?;
    if width == 0 || height == 0 {
        return Err("DDS 尺寸为零".into());
    }
    if rd(76)? != 32 {
        return Err("DDS ddspf.size ≠ 32".into());
    }
    let mip_count = rd(28)?.max(1);
    let fourcc = &bytes[84..88];
    let mut data_off = 128usize;
    let (format, block_bytes): (&'static str, usize) = match fourcc {
        b"DXT1" => ("bc1", 8),
        b"DXT5" => ("bc3", 16),
        b"DX10" => {
            if bytes.len() < 148 {
                return Err("DDS DX10 扩展头截断".into());
            }
            data_off = 148;
            match rd(128)? {
                71 | 72 | 73 => ("bc1", 8),
                77 | 78 | 79 => ("bc3", 16),
                other => return Err(format!("DXGI 格式未入消费闭集（BC1/BC3 面）: {other}")),
            }
        }
        other => {
            return Err(format!(
                "DDS FourCC 未入消费闭集（BC1/BC3 albedo 面）: {}",
                String::from_utf8_lossy(other)
            ))
        }
    };
    let rgb565 = |c: u16| -> [u8; 4] {
        let r = ((c >> 11) & 0x1f) as u8;
        let g = ((c >> 5) & 0x3f) as u8;
        let b = (c & 0x1f) as u8;
        [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2), 255]
    };
    // bcdec::decode_bc4_block/bc4_value 逐字镜像（G11.3 确定性锚 = bcdec 行为
    // 面,非 spec 重述——系数族 (8−(n−1))/(6−(n−1)) 与 bcdec 位级同式）。
    let bc4_alpha = |blk: &[u8], texels: &mut [[u8; 4]; 16]| {
        let a0 = i32::from(blk[0]);
        let a1 = i32::from(blk[1]);
        let e0 = blk[0];
        let e1 = blk[1];
        let mut bits = 0u64;
        for (i, &b) in blk[2..8].iter().enumerate() {
            bits |= u64::from(b) << (i * 8);
        }
        let gt = e0 > e1;
        for (i, t) in texels.iter_mut().enumerate() {
            let idx = ((bits >> (i * 3)) & 0x7) as i32;
            let v = if gt {
                match idx {
                    0 => e0,
                    1 => e1,
                    n => (((8 - (n - 1)) * a0 + (n - 1) * a1) / 7).clamp(0, 255) as u8,
                }
            } else {
                match idx {
                    0 => e0,
                    1 => e1,
                    6 => 0,
                    7 => 255,
                    n => (((6 - (n - 1)) * a0 + (n - 1) * a1) / 5).clamp(0, 255) as u8,
                }
            };
            t[3] = v;
        }
    };
    // 逐级解码（DDS 级段连续存储;级尺寸 = max(w>>l,1)——bcdec 块解码逐字
    // 不变,仅外层加级循环 + 级内行距换级宽）。
    let mut levels: Vec<(u32, u32, Vec<u8>)> = Vec::with_capacity(mip_count as usize);
    let mut cursor = data_off;
    for l in 0..mip_count {
        let lw = (width >> l).max(1);
        let lh = (height >> l).max(1);
        let bw = lw.div_ceil(4) as usize;
        let bh = lh.div_ceil(4) as usize;
        let need = bw * bh * block_bytes;
        let blocks = bytes.get(cursor..cursor + need).ok_or_else(|| {
            format!(
                "DDS 体截断: mip {l} 需 {need} 字节, 存 {}",
                bytes.len().saturating_sub(cursor)
            )
        })?;
        cursor += need;
        let mut out = vec![0u8; (lw as usize) * (lh as usize) * 4];
        let mut bi = 0usize;
        for by in 0..bh {
            for bx in 0..bw {
                let mut texels = [[0u8; 4]; 16];
                let cb = if block_bytes == 16 {
                    bc4_alpha(&blocks[bi..bi + 8], &mut texels);
                    &blocks[bi + 8..bi + 16]
                } else {
                    for t in texels.iter_mut() {
                        t[3] = 255;
                    }
                    &blocks[bi..bi + 8]
                };
                let c0 = u16::from_le_bytes([cb[0], cb[1]]);
                let c1 = u16::from_le_bytes([cb[2], cb[3]]);
                let idx_bits = u32::from_le_bytes([cb[4], cb[5], cb[6], cb[7]]);
                let p0 = rgb565(c0);
                let p1 = rgb565(c1);
                let mut lut = [[0u8; 4]; 4];
                lut[0] = p0;
                lut[1] = p1;
                // bcdec 镜像:BC1 = c0>c1 四色/否则三色+透明黑;BC3 颜色块恒四色
                // （c0<=c1 时索引 3 仍为第四插值色,不透明——与 bcdec 逐字同律）。
                if block_bytes == 16 || c0 > c1 {
                    for ch in 0..3 {
                        lut[2][ch] = ((2 * u32::from(p0[ch]) + u32::from(p1[ch])) / 3) as u8;
                        lut[3][ch] = ((u32::from(p0[ch]) + 2 * u32::from(p1[ch])) / 3) as u8;
                    }
                    lut[2][3] = 255;
                    lut[3][3] = 255;
                } else {
                    for ch in 0..3 {
                        lut[2][ch] = ((u32::from(p0[ch]) + u32::from(p1[ch])) / 2) as u8;
                    }
                    lut[2][3] = 255;
                    lut[3] = [0, 0, 0, 0];
                }
                for (i, t) in texels.iter_mut().enumerate() {
                    let px = lut[((idx_bits >> (2 * i)) & 0x3) as usize];
                    t[0] = px[0];
                    t[1] = px[1];
                    t[2] = px[2];
                    if block_bytes == 8 {
                        t[3] = px[3];
                    }
                }
                for ty in 0..4u32 {
                    for tx in 0..4u32 {
                        let (x, y) = (bx * 4 + tx as usize, by * 4 + ty as usize);
                        if x >= lw as usize || y >= lh as usize {
                            continue;
                        }
                        let o = (y * lw as usize + x) * 4;
                        out[o..o + 4].copy_from_slice(&texels[(ty * 4 + tx) as usize]);
                    }
                }
                bi += block_bytes;
            }
        }
        levels.push((lw, lh, out));
    }
    Ok((width, height, format, levels))
}

/// DDS BC1（DXT1）/BC3（DXT5）mip0 全纹素解码 → RGBA8 行主序（rurix-asset
/// bcdec::decode_dds 的 bin-local 镜像——G11.3 确定性锚在案消费面:逐槽
/// rgba8 digest 与 milestones/g11/g11_3_dds_transcode_manifest.json 登记值
/// 互核,bcdec 行为漂移即红;BC5/BC7 等闭集外 fail-closed 显式拒绝）。
/// 返回 (width, height, format, rgba8)。day_0828 F6 双形态回正：原形态
/// 原名恢复（[`g31_tex_load`] 原形态消费;逐级解码形态 =
/// [`g31_dds_decode_rgba8_mips`]）。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_dds_decode_rgba8(bytes: &[u8]) -> Result<(u32, u32, &'static str, Vec<u8>), String> {
    if bytes.len() < 128 || &bytes[0..4] != b"DDS " {
        return Err("非 DDS magic/头截断".into());
    }
    let rd = |off: usize| -> Result<u32, String> {
        let b = bytes
            .get(off..off + 4)
            .ok_or_else(|| format!("DDS 头截断 @0x{off:x}"))?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if rd(4)? != 124 {
        return Err("DDS header.size ≠ 124".into());
    }
    let height = rd(12)?;
    let width = rd(16)?;
    if width == 0 || height == 0 {
        return Err("DDS 尺寸为零".into());
    }
    if rd(76)? != 32 {
        return Err("DDS ddspf.size ≠ 32".into());
    }
    let fourcc = &bytes[84..88];
    let mut data_off = 128usize;
    let (format, block_bytes): (&'static str, usize) = match fourcc {
        b"DXT1" => ("bc1", 8),
        b"DXT5" => ("bc3", 16),
        b"DX10" => {
            if bytes.len() < 148 {
                return Err("DDS DX10 扩展头截断".into());
            }
            data_off = 148;
            match rd(128)? {
                71 | 72 | 73 => ("bc1", 8),
                77 | 78 | 79 => ("bc3", 16),
                other => return Err(format!("DXGI 格式未入消费闭集（BC1/BC3 面）: {other}")),
            }
        }
        other => {
            return Err(format!(
                "DDS FourCC 未入消费闭集（BC1/BC3 albedo 面）: {}",
                String::from_utf8_lossy(other)
            ))
        }
    };
    let bw = width.div_ceil(4) as usize;
    let bh = height.div_ceil(4) as usize;
    let need = bw * bh * block_bytes;
    let blocks = bytes
        .get(data_off..data_off + need)
        .ok_or_else(|| format!("DDS 体截断: 需 {need} 字节(mip 0), 存 {}", bytes.len().saturating_sub(data_off)))?;
    let rgb565 = |c: u16| -> [u8; 4] {
        let r = ((c >> 11) & 0x1f) as u8;
        let g = ((c >> 5) & 0x3f) as u8;
        let b = (c & 0x1f) as u8;
        [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2), 255]
    };
    // bcdec::decode_bc4_block/bc4_value 逐字镜像（G11.3 确定性锚 = bcdec 行为
    // 面,非 spec 重述——系数族 (8−(n−1))/(6−(n−1)) 与 bcdec 位级同式）。
    let bc4_alpha = |blk: &[u8], texels: &mut [[u8; 4]; 16]| {
        let a0 = i32::from(blk[0]);
        let a1 = i32::from(blk[1]);
        let e0 = blk[0];
        let e1 = blk[1];
        let mut bits = 0u64;
        for (i, &b) in blk[2..8].iter().enumerate() {
            bits |= u64::from(b) << (i * 8);
        }
        let gt = e0 > e1;
        for (i, t) in texels.iter_mut().enumerate() {
            let idx = ((bits >> (i * 3)) & 0x7) as i32;
            let v = if gt {
                match idx {
                    0 => e0,
                    1 => e1,
                    n => (((8 - (n - 1)) * a0 + (n - 1) * a1) / 7).clamp(0, 255) as u8,
                }
            } else {
                match idx {
                    0 => e0,
                    1 => e1,
                    6 => 0,
                    7 => 255,
                    n => (((6 - (n - 1)) * a0 + (n - 1) * a1) / 5).clamp(0, 255) as u8,
                }
            };
            t[3] = v;
        }
    };
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    let mut bi = 0usize;
    for by in 0..bh {
        for bx in 0..bw {
            let mut texels = [[0u8; 4]; 16];
            let cb = if block_bytes == 16 {
                bc4_alpha(&blocks[bi..bi + 8], &mut texels);
                &blocks[bi + 8..bi + 16]
            } else {
                for t in texels.iter_mut() {
                    t[3] = 255;
                }
                &blocks[bi..bi + 8]
            };
            let c0 = u16::from_le_bytes([cb[0], cb[1]]);
            let c1 = u16::from_le_bytes([cb[2], cb[3]]);
            let idx_bits = u32::from_le_bytes([cb[4], cb[5], cb[6], cb[7]]);
            let p0 = rgb565(c0);
            let p1 = rgb565(c1);
            let mut lut = [[0u8; 4]; 4];
            lut[0] = p0;
            lut[1] = p1;
            // bcdec 镜像:BC1 = c0>c1 四色/否则三色+透明黑;BC3 颜色块恒四色
            // （c0<=c1 时索引 3 仍为第四插值色,不透明——与 bcdec 逐字同律）。
            if block_bytes == 16 || c0 > c1 {
                for ch in 0..3 {
                    lut[2][ch] = ((2 * u32::from(p0[ch]) + u32::from(p1[ch])) / 3) as u8;
                    lut[3][ch] = ((u32::from(p0[ch]) + 2 * u32::from(p1[ch])) / 3) as u8;
                }
                lut[2][3] = 255;
                lut[3][3] = 255;
            } else {
                for ch in 0..3 {
                    lut[2][ch] = ((u32::from(p0[ch]) + u32::from(p1[ch])) / 2) as u8;
                }
                lut[2][3] = 255;
                lut[3] = [0, 0, 0, 0];
            }
            for (i, t) in texels.iter_mut().enumerate() {
                let px = lut[((idx_bits >> (2 * i)) & 0x3) as usize];
                t[0] = px[0];
                t[1] = px[1];
                t[2] = px[2];
                if block_bytes == 8 {
                    t[3] = px[3];
                }
            }
            for ty in 0..4u32 {
                for tx in 0..4u32 {
                    let (x, y) = (bx * 4 + tx as usize, by * 4 + ty as usize);
                    if x >= width as usize || y >= height as usize {
                        continue;
                    }
                    let o = (y * width as usize + x) * 4;
                    out[o..o + 4].copy_from_slice(&texels[(ty * 4 + tx) as usize]);
                }
            }
            bi += block_bytes;
        }
    }
    Ok((width, height, format, out))
}

/// 256 项 srgb→linear LUT（host `srgb_to_linear` 逐字同式;零 pow 面 =
/// device/host 位级对拍锚——kernel/host 参考同查本表）。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_tex_linlut() -> [f32; 256] {
    let mut lut = [0.0f32; 256];
    for (i, e) in lut.iter_mut().enumerate() {
        *e = srgb_to_linear(i as f32 / 255.0);
    }
    lut
}

/// 探针 UV 闭集律法（24/槽:16 网格 + 4 精确边缘 + 4 wrap 域;确定性,与 CI
/// smoke 判读器同源镜像——篡改律法即红）。返回 (slot, u, v) 全槽展平列。
/// day_0828 F6 双形态回正：原形态原签名恢复（g34_full_lane 系消费面）;
/// mip 维形态 = [`g31_tex_probes_mip`]。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_tex_probes(n_slots: usize) -> Vec<(u32, f32, f32)> {
    let mut out = Vec::with_capacity(n_slots * G31_TEX_PROBES_PER_SLOT);
    for k in 0..n_slots {
        for j in 0..16u32 {
            let u = (((j * 37 + (k as u32) * 11) % 256) as f32 + 0.5) / 256.0;
            let v = (((j * 101 + (k as u32) * 13) % 256) as f32 + 0.5) / 256.0;
            out.push((k as u32, u, v));
        }
        // 精确边缘（含边界回绕触发面:x0/x1 跨 0/w 界）。
        let em1 = 1.0f32 - 2.0f32.powi(-23);
        out.push((k as u32, 0.0, 0.0));
        out.push((k as u32, 0.0, 0.5));
        out.push((k as u32, 0.5, 0.0));
        out.push((k as u32, em1, em1));
        // wrap 域（fract 回绕;含负域）。
        out.push((k as u32, 1.25, 2.5));
        out.push((k as u32, 3.75, 1.5));
        out.push((k as u32, -0.25, 1.3333334));
        out.push((k as u32, 2.0, -0.75));
    }
    out
}

/// [`g31_tex_probes`] 的 day_0828 Phase B mip 维形态（F6 双形态回正改名）：
/// 每槽 24 UV × 抽样级 {0, mips/2, mips−1}（去重,mips=1 槽单级）;返回
/// (slot, u, v, lod) 全槽展平列（lod 显式注入 = heap 逐级寻址对拍面）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
fn g31_tex_probes_mip(slots: &[G31TexSlotHeap]) -> Vec<(u32, f32, f32, u32)> {
    let mut out = Vec::with_capacity(slots.len() * G31_TEX_PROBES_PER_SLOT * G31_TEX_PROBE_MIPS);
    for (k, s) in slots.iter().enumerate() {
        let mips = s.mip_count.max(1);
        let mut lods = vec![0u32, mips / 2, mips - 1];
        lods.dedup();
        for &lod in &lods {
            for j in 0..16u32 {
                let u = (((j * 37 + (k as u32) * 11) % 256) as f32 + 0.5) / 256.0;
                let v = (((j * 101 + (k as u32) * 13) % 256) as f32 + 0.5) / 256.0;
                out.push((k as u32, u, v, lod));
            }
            // 精确边缘（含边界回绕触发面:x0/x1 跨 0/w 界）。
            let em1 = 1.0f32 - 2.0f32.powi(-23);
            out.push((k as u32, 0.0, 0.0, lod));
            out.push((k as u32, 0.0, 0.5, lod));
            out.push((k as u32, 0.5, 0.0, lod));
            out.push((k as u32, em1, em1, lod));
            // wrap 域（fract 回绕;含负域）。
            out.push((k as u32, 1.25, 2.5, lod));
            out.push((k as u32, 3.75, 1.5, lod));
            out.push((k as u32, -0.25, 1.3333334, lod));
            out.push((k as u32, 2.0, -0.75, lod));
        }
    }
    out
}

/// host 采样参考（与 kernels/g31_texture_{gi,probe}.rx 采样块同 op 序——
/// Rust f32 无收缩 + device NoContraction 注入 ⇒ 位级对拍面）。采样域 =
/// texmeta/atlas/linlut 三 SSBO 的 host 同源 f32/u32 面。day_0828 F6 双形态
/// 回正：原形态原签名原 op 序恢复（g34_full_lane 系消费面——其 kernel
/// g34_unified_gi.rx 为门锚冻结,G/B 底行 fy 系数与之位级同式,不施加 heap
/// 侧 fy→fx 修正）;heap/mip 形态 = [`g31_tex_host_sample_mip`]。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_tex_host_sample(
    texmeta: &[f32],
    atlas: &[u32],
    linlut: &[f32; 256],
    slot: usize,
    uu0: f32,
    vv0: f32,
) -> [f32; 3] {
    let sb = 8 + slot * 8;
    let ox = texmeta[sb];
    let oy = texmeta[sb + 1];
    let tw = texmeta[sb + 2];
    let th2 = texmeta[sb + 3];
    let mod_r = texmeta[sb + 4];
    let mod_g = texmeta[sb + 5];
    let mod_b = texmeta[sb + 6];
    let aw = texmeta[0];
    let uu = uu0 - uu0.floor();
    let vv = vv0 - vv0.floor();
    let xf = uu * tw - 0.5;
    let yf = vv * th2 - 0.5;
    let bxf = xf.floor();
    let byf = yf.floor();
    let fx = xf - bxf;
    let fy = yf - byf;
    let inv_tw = 1.0 / tw;
    let inv_th = 1.0 / th2;
    let x0 = bxf - (bxf * inv_tw).floor() * tw;
    let y0 = byf - (byf * inv_th).floor() * th2;
    let x1 = (bxf + 1.0) - ((bxf + 1.0) * inv_tw).floor() * tw;
    let y1 = (byf + 1.0) - ((byf + 1.0) * inv_th).floor() * th2;
    let a00 = ((oy + y0) as usize) * (aw as usize) + ((ox + x0) as usize);
    let a10 = ((oy + y0) as usize) * (aw as usize) + ((ox + x1) as usize);
    let a01 = ((oy + y1) as usize) * (aw as usize) + ((ox + x0) as usize);
    let a11 = ((oy + y1) as usize) * (aw as usize) + ((ox + x1) as usize);
    let p00 = atlas[a00] as usize;
    let p10 = atlas[a10] as usize;
    let p01 = atlas[a01] as usize;
    let p11 = atlas[a11] as usize;
    let p00_r = linlut[p00 % 256usize];
    let p00_g = linlut[(p00 / 256usize) % 256usize];
    let p00_b = linlut[(p00 / 65536usize) % 256usize];
    let p10_r = linlut[p10 % 256usize];
    let p10_g = linlut[(p10 / 256usize) % 256usize];
    let p10_b = linlut[(p10 / 65536usize) % 256usize];
    let p01_r = linlut[p01 % 256usize];
    let p01_g = linlut[(p01 / 256usize) % 256usize];
    let p01_b = linlut[(p01 / 65536usize) % 256usize];
    let p11_r = linlut[p11 % 256usize];
    let p11_g = linlut[(p11 / 256usize) % 256usize];
    let p11_b = linlut[(p11 / 65536usize) % 256usize];
    let t0r = p00_r * (1.0 - fx) + p10_r * fx;
    let b0r = p01_r * (1.0 - fx) + p11_r * fx;
    let samp_r = (t0r * (1.0 - fy) + b0r * fy) * mod_r;
    // G37 W1:fx/fy 双线性同源 bug 修复(day_0828 HANDOVER §A.1)——G/B 底行
    // 水平混合 fy→fx,与 g34_unified_{gi,gi_skin,shade}.rx kernel 同步改
    // (host/device 对拍面成对修,防同错互抵恒绿假象)。
    let t0g = p00_g * (1.0 - fx) + p10_g * fx;
    let b0g = p01_g * (1.0 - fx) + p11_g * fx;
    let samp_g = (t0g * (1.0 - fy) + b0g * fy) * mod_g;
    let t0b = p00_b * (1.0 - fx) + p10_b * fx;
    let b0b = p01_b * (1.0 - fx) + p11_b * fx;
    let samp_b = (t0b * (1.0 - fy) + b0b * fy) * mod_b;
    [samp_r, samp_g, samp_b]
}

/// [`g31_tex_host_sample`] 的 day_0828 Phase B heap/mip 形态（F6 双形态回正
/// 改名）：heap 寻址（头表 [slot×13+lod] 取级基址）+ lod 显式入参（探针律法
/// 注入，生产 kernel 的 log2 推导为 device 器件面不入对拍域）+ G/B 底行
/// fy→fx 修正（heap 系 kernel 同步）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
fn g31_tex_host_sample_mip(
    texmeta: &[f32],
    atlas: &[u32],
    linlut: &[f32; 256],
    slot: usize,
    uu0: f32,
    vv0: f32,
    lod: usize,
) -> [f32; 3] {
    let sb = 8 + slot * 8;
    let tw = texmeta[sb + 2];
    let th2 = texmeta[sb + 3];
    let mod_r = texmeta[sb + 4];
    let mod_g = texmeta[sb + 5];
    let mod_b = texmeta[sb + 6];
    let mut mw = tw;
    let mut mh = th2;
    let mut li = 0usize;
    while li < lod {
        mw = (mw * 0.5).max(1.0);
        mh = (mh * 0.5).max(1.0);
        li += 1;
    }
    let hbase = atlas[slot * G31_TEX_MIP_SLOTS + lod] as usize;
    let uu = uu0 - uu0.floor();
    let vv = vv0 - vv0.floor();
    let xf = uu * mw - 0.5;
    let yf = vv * mh - 0.5;
    let bxf = xf.floor();
    let byf = yf.floor();
    let fx = xf - bxf;
    let fy = yf - byf;
    let inv_tw = 1.0 / mw;
    let inv_th = 1.0 / mh;
    let x0 = bxf - (bxf * inv_tw).floor() * mw;
    let y0 = byf - (byf * inv_th).floor() * mh;
    let x1 = (bxf + 1.0) - ((bxf + 1.0) * inv_tw).floor() * mw;
    let y1 = (byf + 1.0) - ((byf + 1.0) * inv_th).floor() * mh;
    let a00 = hbase + (y0 as usize) * (mw as usize) + (x0 as usize);
    let a10 = hbase + (y0 as usize) * (mw as usize) + (x1 as usize);
    let a01 = hbase + (y1 as usize) * (mw as usize) + (x0 as usize);
    let a11 = hbase + (y1 as usize) * (mw as usize) + (x1 as usize);
    let p00 = atlas[a00] as usize;
    let p10 = atlas[a10] as usize;
    let p01 = atlas[a01] as usize;
    let p11 = atlas[a11] as usize;
    let p00_r = linlut[p00 % 256usize];
    let p00_g = linlut[(p00 / 256usize) % 256usize];
    let p00_b = linlut[(p00 / 65536usize) % 256usize];
    let p10_r = linlut[p10 % 256usize];
    let p10_g = linlut[(p10 / 256usize) % 256usize];
    let p10_b = linlut[(p10 / 65536usize) % 256usize];
    let p01_r = linlut[p01 % 256usize];
    let p01_g = linlut[(p01 / 256usize) % 256usize];
    let p01_b = linlut[(p01 / 65536usize) % 256usize];
    let p11_r = linlut[p11 % 256usize];
    let p11_g = linlut[(p11 / 256usize) % 256usize];
    let p11_b = linlut[(p11 / 65536usize) % 256usize];
    let t0r = p00_r * (1.0 - fx) + p10_r * fx;
    let b0r = p01_r * (1.0 - fx) + p11_r * fx;
    let samp_r = (t0r * (1.0 - fy) + b0r * fy) * mod_r;
    let t0g = p00_g * (1.0 - fx) + p10_g * fx;
    let b0g = p01_g * (1.0 - fx) + p11_g * fx;
    let samp_g = (t0g * (1.0 - fy) + b0g * fy) * mod_g;
    let t0b = p00_b * (1.0 - fx) + p10_b * fx;
    let b0b = p01_b * (1.0 - fx) + p11_b * fx;
    let samp_b = (t0b * (1.0 - fy) + b0b * fy) * mod_b;
    [samp_r, samp_g, samp_b]
}

/// host srgb 域采样参考（sampler 腿对拍面:与硬件链同语义——srgb 值域双线
/// 性（texel = n/255.0f,UNORM 精确同式）+ 8-bit 量化镜像（round half up,
/// (x·255+0.5).floor()——g31_display_encode 8-bit 量化同字面）。tile =
/// 行主序 RGBA8。返回 [r,g,b,a] u8。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_tex_host_sample_srgb(tile: &[u8], w: u32, h: u32, uu0: f32, vv0: f32) -> [u8; 4] {
    let tw = w as f32;
    let th2 = h as f32;
    let uu = uu0 - uu0.floor();
    let vv = vv0 - vv0.floor();
    let xf = uu * tw - 0.5;
    let yf = vv * th2 - 0.5;
    let bxf = xf.floor();
    let byf = yf.floor();
    let fx = xf - bxf;
    let fy = yf - byf;
    let inv_tw = 1.0 / tw;
    let inv_th = 1.0 / th2;
    let x0 = bxf - (bxf * inv_tw).floor() * tw;
    let y0 = byf - (byf * inv_th).floor() * th2;
    let x1 = (bxf + 1.0) - ((bxf + 1.0) * inv_tw).floor() * tw;
    let y1 = (byf + 1.0) - ((byf + 1.0) * inv_th).floor() * th2;
    let texel = |x: f32, y: f32, c: usize| -> f32 {
        let o = ((y as usize) * (w as usize) + (x as usize)) * 4 + c;
        tile[o] as f32 / 255.0
    };
    let mut out = [0u8; 4];
    for c in 0..4usize {
        let p00 = texel(x0, y0, c);
        let p10 = texel(x1, y0, c);
        let p01 = texel(x0, y1, c);
        let p11 = texel(x1, y1, c);
        let t0 = p00 * (1.0 - fx) + p10 * fx;
        let b0 = p01 * (1.0 - fx) + p11 * fx;
        let s = t0 * (1.0 - fy) + b0 * fy;
        let q = (s * 255.0 + 0.5).floor();
        out[c] = q.max(0.0).min(255.0) as u8;
    }
    out
}

/// B4 贴图资产装配（--textures on 面;fail-closed 闭集——top-N 律法/glTF 互核/
/// BC1-BC3 解码/pow2 限定/G11.3 manifest 互核全链任一破即 Err,不静默降级）。
/// `tri_uv` = assemble_scene_uv 产出的 6 f32/tri（长度 = 三角数 × 6 互核）。
/// day_0828 F6 双形态回正：原形态原名原语义恢复（top-12 + 2D 网格图集 +
/// tritex 步幅 1;g34_full_lane 系消费面,HEAD 逐字恢复零漂移）;heap 形态 =
/// [`g31_tex_load_heap`]。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_tex_load(
    scene: &SceneData,
    gltf_path: &Path,
    tri_uv: &[f32],
) -> Result<G31TexAssets, String> {
    let t0 = std::time::Instant::now();
    if tri_uv.len() != scene.indices.len() * 6 {
        return Err(format!(
            "UV sink 长度 {} ≠ 三角数×6 {}",
            tri_uv.len(),
            scene.indices.len() * 6
        ));
    }
    let (gltf, _) = load_gltf(gltf_path)?;
    let base = gltf_path.parent().unwrap_or_else(|| Path::new("."));
    // ── ① 资产面盘点（census;glTF 材质/属性计数闭集）──
    let mats_json = gltf
        .root
        .get("materials")
        .and_then(|v| v.as_array())
        .unwrap_or(&[]);
    let mut census = G31TexCensus {
        materials_total: mats_json.len(),
        with_base_color_texture: 0,
        with_normal_texture: 0,
        with_metallic_roughness_texture: 0,
        primitives_total: 0,
        primitives_with_texcoord0: 0,
        primitives_with_tangent: 0,
    };
    let images = gltf.root.get("images").and_then(|v| v.as_array());
    let textures = gltf.root.get("textures").and_then(|v| v.as_array());
    let tex_uri = |t: &Json| -> Option<String> {
        let ti = t.get("index").and_then(|v| v.as_u64())? as usize;
        let src = textures?.get(ti)?.get("source").and_then(|v| v.as_u64())? as usize;
        Some(images?.get(src)?.get("uri").and_then(|v| v.as_str())?.to_owned())
    };
    let mut mat_base_uri: Vec<Option<String>> = Vec::new();
    let mut mat_factor: Vec<[f32; 3]> = Vec::new();
    let mut mat_k: Vec<f32> = Vec::new();
    for m in mats_json {
        let pbr = m.get("pbrMetallicRoughness");
        if pbr.and_then(|p| p.get("baseColorTexture")).is_some() {
            census.with_base_color_texture += 1;
        }
        if pbr
            .and_then(|p| p.get("metallicRoughnessTexture"))
            .is_some()
        {
            census.with_metallic_roughness_texture += 1;
        }
        if m.get("normalTexture").is_some() {
            census.with_normal_texture += 1;
        }
        let alb4 = pbr
            .and_then(|p| p.get("baseColorFactor"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(1.0) as f32).collect::<Vec<_>>());
        let factor = match alb4 {
            Some(v) if v.len() == 4 => [v[0], v[1], v[2]],
            _ => [1.0, 1.0, 1.0],
        };
        let metallic = pbr
            .and_then(|p| p.get("metallicFactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        mat_factor.push(factor);
        mat_k.push(1.0 - metallic);
        mat_base_uri.push(
            pbr.and_then(|p| p.get("baseColorTexture"))
                .and_then(|t| tex_uri(t)),
        );
    }
    for mesh in gltf
        .root
        .get("meshes")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        for prim in mesh
            .get("primitives")
            .and_then(|v| v.as_array())
            .unwrap_or(&[])
        {
            census.primitives_total += 1;
            let attrs = prim.get("attributes");
            if attrs.and_then(|a| a.get("TEXCOORD_0")).is_some() {
                census.primitives_with_texcoord0 += 1;
            }
            if attrs.and_then(|a| a.get("TANGENT")).is_some() {
                census.primitives_with_tangent += 1;
            }
        }
    }
    // ── ② top-N 映射律法（逐材质三角数降序,并列时 material_index 升序——
    //    确定性闭集;其余走既有常量面 0-byte）──
    let mut tri_count: std::collections::BTreeMap<u32, usize> = std::collections::BTreeMap::new();
    for &mi in &scene.tri_mat {
        if mi != SLAB_TRI_NONE {
            *tri_count.entry(mi).or_default() += 1;
        }
    }
    let mut rank: Vec<(u32, usize)> = tri_count.into_iter().collect();
    rank.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let n_map = G31_TEX_N_MAPPED.min(rank.len());
    if n_map == 0 {
        return Err("场景零可映射材质（--textures on 面 fail-closed）".into());
    }
    let picked = &rank[..n_map];
    // ── ③ G11.3 manifest 互核面装载（在树即核;缺失 = None 登记不 fail——
    //    派生链登记面缺失与 BC 解码漂移是两档事态,前者如实登记）──
    let manifest: Option<Vec<(String, String, String)>> = (|| {
        let text = std::fs::read_to_string(
            "milestones/g11/g11_3_dds_transcode_manifest.json",
        )
        .ok()?;
        let doc = json_parse(&text).ok()?;
        let entries = doc.get("entries")?.as_array()?;
        let mut out = Vec::with_capacity(entries.len());
        for e in entries {
            out.push((
                as_str("source_uri", e.get("source_uri")?).ok()?.to_owned(),
                as_str("source_digest", e.get("source_digest")?).ok()?.to_owned(),
                as_str("rgba8_digest", e.get("rgba8_digest")?).ok()?.to_owned(),
            ));
        }
        Some(out)
    })();
    // ── ④ 逐槽解码 + 图集烘焙（瓦位 = 槽序 × TILE;小纹理只占左上 w×h 区）──
    let grid_cols = G31_TEX_GRID_COLS;
    let grid_rows = (n_map as u32).div_ceil(grid_cols);
    let atlas_w = grid_cols * G31_TEX_TILE;
    let atlas_h = grid_rows * G31_TEX_TILE;
    let mut atlas = vec![0u32; (atlas_w * atlas_h) as usize];
    let mut slots: Vec<G31TexSlot> = Vec::with_capacity(n_map);
    let mut slots_rgba8: Vec<Vec<u8>> = Vec::with_capacity(n_map);
    for (k, &(mi, ntris)) in picked.iter().enumerate() {
        let uri = mat_base_uri
            .get(mi as usize)
            .and_then(|u| u.clone())
            .ok_or_else(|| {
                format!("top-{n_map} 律法命中材质 index {mi} 缺 baseColorTexture（fail-closed 不静默）")
            })?;
        let raw = std::fs::read(base.join(&uri))
            .map_err(|e| format!("纹理 {uri} 读取失败: {e}"))?;
        let (w, h, fmt, rgba8) = g31_dds_decode_rgba8(&raw)
            .map_err(|e| format!("纹理 {uri} DDS 解码失败: {e}"))?;
        if w > G31_TEX_TILE || h > G31_TEX_TILE || !w.is_power_of_two() || !h.is_power_of_two() {
            return Err(format!(
                "纹理 {uri} 尺寸 {w}x{h} 越 pow2 ≤ {G31_TEX_TILE} 闭集（wrap 精确域 fail-closed）"
            ));
        }
        let rgba8_digest = format!("sha256:{}", sha256_hex(&rgba8));
        let manifest_row = manifest.as_ref().and_then(|m| {
            m.iter().find(|(u, _, _)| *u == uri)
        });
        let (ox, oy) = (
            (k as u32 % grid_cols) * G31_TEX_TILE,
            (k as u32 / grid_cols) * G31_TEX_TILE,
        );
        for y in 0..h as usize {
            for x in 0..w as usize {
                let s = (y * w as usize + x) * 4;
                let packed = u32::from(rgba8[s])
                    | (u32::from(rgba8[s + 1]) << 8)
                    | (u32::from(rgba8[s + 2]) << 16)
                    | (u32::from(rgba8[s + 3]) << 24);
                atlas[(oy as usize + y) * atlas_w as usize + (ox as usize + x)] = packed;
            }
        }
        let factor = mat_factor[mi as usize];
        let kk = mat_k[mi as usize];
        let name = mats_json
            .get(mi as usize)
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        slots.push(G31TexSlot {
            material_index: mi,
            material_name: name,
            tris: ntris,
            texture_uri: uri,
            width: w,
            height: h,
            dds_format: fmt.to_owned(),
            manifest_source_digest: manifest_row.map(|(_, s, _)| s.clone()),
            rgba8_digest,
            manifest_rgba8_digest: manifest_row.map(|(_, _, d)| d.clone()),
            origin_x: ox,
            origin_y: oy,
            mod_rgb: [factor[0] * kk, factor[1] * kk, factor[2] * kk],
        });
        slots_rgba8.push(rgba8);
    }
    // ── ⑤ 侧表 SSBO 面（texmeta/tritex;uv 字节面由 sink 直派生）──
    let mut texmeta = vec![
        atlas_w as f32,
        atlas_h as f32,
        n_map as f32,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ];
    for s in &slots {
        texmeta.extend_from_slice(&[
            s.origin_x as f32,
            s.origin_y as f32,
            s.width as f32,
            s.height as f32,
            s.mod_rgb[0],
            s.mod_rgb[1],
            s.mod_rgb[2],
            0.0,
        ]);
    }
    let slot_of = |mi: u32| -> f32 {
        slots
            .iter()
            .position(|s| s.material_index == mi)
            .map(|p| p as f32)
            .unwrap_or(-1.0)
    };
    let mut tritex = Vec::with_capacity(scene.tri_mat.len());
    let mut tex_tris = 0usize;
    for &mi in &scene.tri_mat {
        let s = if mi == SLAB_TRI_NONE { -1.0 } else { slot_of(mi) };
        if s >= 0.0 {
            tex_tris += 1;
        }
        tritex.push(s);
    }
    if tex_tris == 0 {
        return Err("映射材质零三角命中（空接线即红,fail-closed）".into());
    }
    let linlut = g31_tex_linlut();
    let atlas_bytes: Vec<u8> = atlas.iter().flat_map(|v| v.to_le_bytes()).collect();
    let linlut_bytes: Vec<u8> = linlut.iter().flat_map(|v| v.to_le_bytes()).collect();
    let atlas_digest = format!("sha256:{}", sha256_hex(&atlas_bytes));
    let linlut_digest = format!("sha256:{}", sha256_hex(&linlut_bytes));
    let eval_ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok(G31TexAssets {
        slots,
        census,
        atlas_w,
        atlas_h,
        atlas,
        atlas_bytes,
        atlas_digest,
        linlut,
        linlut_bytes,
        linlut_digest,
        texmeta_bytes: bytes_f32(&texmeta),
        texmeta,
        tritex_bytes: bytes_f32(&tritex),
        tritex,
        texuv_bytes: bytes_f32(tri_uv),
        slots_rgba8,
        tex_tris,
        eval_ms,
    })
}

/// [`g31_tex_load`] 的 day_0828 Phase B heap 形态（F6 双形态回正改名）：
/// top-70 全覆盖 + 一维 texel heap（u32 偏移头表 [slot×13+mip] + 逐槽逐级
/// texel 段）+ tritex 步幅 2 [slot, k_tri]——fail-closed 闭集同律。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
fn g31_tex_load_heap(
    scene: &SceneData,
    gltf_path: &Path,
    tri_uv: &[f32],
) -> Result<G31TexAssetsHeap, String> {
    let t0 = std::time::Instant::now();
    if tri_uv.len() != scene.indices.len() * 6 {
        return Err(format!(
            "UV sink 长度 {} ≠ 三角数×6 {}",
            tri_uv.len(),
            scene.indices.len() * 6
        ));
    }
    let (gltf, _) = load_gltf(gltf_path)?;
    let base = gltf_path.parent().unwrap_or_else(|| Path::new("."));
    // ── ① 资产面盘点（census;glTF 材质/属性计数闭集）──
    let mats_json = gltf
        .root
        .get("materials")
        .and_then(|v| v.as_array())
        .unwrap_or(&[]);
    let mut census = G31TexCensus {
        materials_total: mats_json.len(),
        with_base_color_texture: 0,
        with_normal_texture: 0,
        with_metallic_roughness_texture: 0,
        primitives_total: 0,
        primitives_with_texcoord0: 0,
        primitives_with_tangent: 0,
    };
    let images = gltf.root.get("images").and_then(|v| v.as_array());
    let textures = gltf.root.get("textures").and_then(|v| v.as_array());
    let tex_uri = |t: &Json| -> Option<String> {
        let ti = t.get("index").and_then(|v| v.as_u64())? as usize;
        let src = textures?.get(ti)?.get("source").and_then(|v| v.as_u64())? as usize;
        Some(images?.get(src)?.get("uri").and_then(|v| v.as_str())?.to_owned())
    };
    let mut mat_base_uri: Vec<Option<String>> = Vec::new();
    let mut mat_factor: Vec<[f32; 3]> = Vec::new();
    let mut mat_k: Vec<f32> = Vec::new();
    for m in mats_json {
        let pbr = m.get("pbrMetallicRoughness");
        if pbr.and_then(|p| p.get("baseColorTexture")).is_some() {
            census.with_base_color_texture += 1;
        }
        if pbr
            .and_then(|p| p.get("metallicRoughnessTexture"))
            .is_some()
        {
            census.with_metallic_roughness_texture += 1;
        }
        if m.get("normalTexture").is_some() {
            census.with_normal_texture += 1;
        }
        let alb4 = pbr
            .and_then(|p| p.get("baseColorFactor"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(1.0) as f32).collect::<Vec<_>>());
        let factor = match alb4 {
            Some(v) if v.len() == 4 => [v[0], v[1], v[2]],
            _ => [1.0, 1.0, 1.0],
        };
        let metallic = pbr
            .and_then(|p| p.get("metallicFactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        mat_factor.push(factor);
        mat_k.push(1.0 - metallic);
        mat_base_uri.push(
            pbr.and_then(|p| p.get("baseColorTexture"))
                .and_then(|t| tex_uri(t)),
        );
    }
    for mesh in gltf
        .root
        .get("meshes")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        for prim in mesh
            .get("primitives")
            .and_then(|v| v.as_array())
            .unwrap_or(&[])
        {
            census.primitives_total += 1;
            let attrs = prim.get("attributes");
            if attrs.and_then(|a| a.get("TEXCOORD_0")).is_some() {
                census.primitives_with_texcoord0 += 1;
            }
            if attrs.and_then(|a| a.get("TANGENT")).is_some() {
                census.primitives_with_tangent += 1;
            }
        }
    }
    // ── ② top-N 映射律法（逐材质三角数降序,并列时 material_index 升序——
    //    确定性闭集;其余走既有常量面 0-byte）──
    let mut tri_count: std::collections::BTreeMap<u32, usize> = std::collections::BTreeMap::new();
    for &mi in &scene.tri_mat {
        if mi != SLAB_TRI_NONE {
            *tri_count.entry(mi).or_default() += 1;
        }
    }
    let mut rank: Vec<(u32, usize)> = tri_count.into_iter().collect();
    rank.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let n_map = G31_TEX_N_MAPPED_HEAP.min(rank.len());
    if n_map == 0 {
        return Err("场景零可映射材质（--textures on 面 fail-closed）".into());
    }
    let picked = &rank[..n_map];
    // ── ③ G11.3 manifest 互核面装载（在树即核;缺失 = None 登记不 fail——
    //    派生链登记面缺失与 BC 解码漂移是两档事态,前者如实登记）──
    let manifest: Option<Vec<(String, String, String)>> = (|| {
        let text = std::fs::read_to_string(
            "milestones/g11/g11_3_dds_transcode_manifest.json",
        )
        .ok()?;
        let doc = json_parse(&text).ok()?;
        let entries = doc.get("entries")?.as_array()?;
        let mut out = Vec::with_capacity(entries.len());
        for e in entries {
            out.push((
                as_str("source_uri", e.get("source_uri")?).ok()?.to_owned(),
                as_str("source_digest", e.get("source_digest")?).ok()?.to_owned(),
                as_str("rgba8_digest", e.get("rgba8_digest")?).ok()?.to_owned(),
            ));
        }
        Some(out)
    })();
    // ── ④ 逐槽全链解码 + texel heap 装配（day_0828 Phase B：2D 网格 →
    //    一维 heap;u32 偏移头表 [slot×13+mip] 进 atlas buffer 头部 = 零新增
    //    绑定;cap-1024 档 = >cap 源从对应源 mip 起搬,零重采样;G11.3 manifest
    //    互核锚 = 源 mip0 digest 不动,逐存储级 digest 新增登记）──
    let header_entries = n_map * G31_TEX_MIP_SLOTS;
    let mut slots: Vec<G31TexSlotHeap> = Vec::with_capacity(n_map);
    let mut slots_rgba8: Vec<Vec<u8>> = Vec::with_capacity(n_map);
    // 逐槽存储级集（(w,h,rgba8) 装入序;先收集后二遍布局——offset 表需总长）。
    let mut slot_levels: Vec<Vec<(u32, u32, Vec<u8>)>> = Vec::with_capacity(n_map);
    for &(mi, ntris) in picked.iter() {
        let uri = mat_base_uri
            .get(mi as usize)
            .and_then(|u| u.clone())
            .ok_or_else(|| {
                format!("top-{n_map} 律法命中材质 index {mi} 缺 baseColorTexture（fail-closed 不静默）")
            })?;
        let raw = std::fs::read(base.join(&uri))
            .map_err(|e| format!("纹理 {uri} 读取失败: {e}"))?;
        let (w, h, fmt, levels) = g31_dds_decode_rgba8_mips(&raw)
            .map_err(|e| format!("纹理 {uri} DDS 解码失败: {e}"))?;
        if w > G31_TEX_TILE || h > G31_TEX_TILE || !w.is_power_of_two() || !h.is_power_of_two() {
            return Err(format!(
                "纹理 {uri} 尺寸 {w}x{h} 越 pow2 ≤ {G31_TEX_TILE} 闭集（wrap 精确域 fail-closed）"
            ));
        }
        // G11.3 锚：源 mip0 全分辨率 digest（manifest rgba8_digest 互核域）。
        let rgba8_digest = format!("sha256:{}", sha256_hex(&levels[0].2));
        let manifest_row = manifest.as_ref().and_then(|m| {
            m.iter().find(|(u, _, _)| *u == uri)
        });
        // cap 起级：首个 max(w_l,h_l) ≤ cap 的级（bistro：2048²→mip1=1024²;
        // 16²→mip0）。链短于起级 = 源链异常 fail-closed。
        let start = levels
            .iter()
            .position(|(lw, lh, _)| *lw <= G31_TEX_CAP && *lh <= G31_TEX_CAP)
            .ok_or_else(|| {
                format!("纹理 {uri} 全链无 ≤{}² 级（cap 档源链异常,fail-closed）", G31_TEX_CAP)
            })?;
        let stored: Vec<(u32, u32, Vec<u8>)> = levels[start..].to_vec();
        let mip_count = stored.len() as u32;
        if mip_count as usize > G31_TEX_MIP_SLOTS {
            return Err(format!(
                "纹理 {uri} 存储级数 {mip_count} 越头表槽位 {G31_TEX_MIP_SLOTS}（fail-closed）"
            ));
        }
        let (bw0, bh0) = (stored[0].0, stored[0].1);
        // 完整链判定（基级到 1×1）;短链按可用级截断登记（bistro 全集完整）。
        let full_chain = bw0.max(bh0).trailing_zeros() + 1;
        let mip_truncated = mip_count < full_chain;
        let mip_digests: Vec<String> = stored
            .iter()
            .map(|(_, _, px)| format!("sha256:{}", sha256_hex(px)))
            .collect();
        let factor = mat_factor[mi as usize];
        let kk = mat_k[mi as usize];
        let name = mats_json
            .get(mi as usize)
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        slots.push(G31TexSlotHeap {
            material_index: mi,
            material_name: name,
            tris: ntris,
            texture_uri: uri,
            width: bw0,
            height: bh0,
            src_width: w,
            src_height: h,
            dds_format: fmt.to_owned(),
            manifest_source_digest: manifest_row.map(|(_, s, _)| s.clone()),
            rgba8_digest,
            manifest_rgba8_digest: manifest_row.map(|(_, _, d)| d.clone()),
            mip_count,
            mip_digests,
            mip_truncated,
            origin_x: 0,
            origin_y: 0,
            mod_rgb: [factor[0] * kk, factor[1] * kk, factor[2] * kk],
        });
        // sampler 腿/host srgb 参考源 = 存储基级 RGBA8。
        slots_rgba8.push(stored[0].2.clone());
        slot_levels.push(stored);
    }
    // heap 布局二遍：头表偏移（绝对 u32 texel 下标;缺级槽位重复末级偏移——
    // kernel lod 已钳 mips−1,冗余保底）→ texel 段逐槽逐级连续装入。
    let mut heap_texels = header_entries;
    let mut header = vec![0u32; header_entries];
    for (k, lv) in slot_levels.iter().enumerate() {
        for (m, (lw, lh, _)) in lv.iter().enumerate() {
            header[k * G31_TEX_MIP_SLOTS + m] = heap_texels as u32;
            heap_texels += (*lw as usize) * (*lh as usize);
        }
        let last = header[k * G31_TEX_MIP_SLOTS + lv.len() - 1];
        for m in lv.len()..G31_TEX_MIP_SLOTS {
            header[k * G31_TEX_MIP_SLOTS + m] = last;
        }
    }
    // fail-closed：heap 字节 ≤ 保守 maxStorageBufferRange 界（文件头常量注）。
    let heap_bytes = (heap_texels as u64) * 4;
    if heap_bytes > G31_TEX_HEAP_MAX_BYTES {
        return Err(format!(
            "texel heap {heap_bytes} B 越 maxStorageBufferRange 保守界 {G31_TEX_HEAP_MAX_BYTES} B（fail-closed;需降 cap 档或分页）"
        ));
    }
    let mut atlas = vec![0u32; heap_texels];
    atlas[..header_entries].copy_from_slice(&header);
    let mut cursor = header_entries;
    for lv in &slot_levels {
        for (lw, lh, px) in lv {
            let n = (*lw as usize) * (*lh as usize);
            for (t, chunk) in px.chunks_exact(4).enumerate() {
                atlas[cursor + t] = u32::from(chunk[0])
                    | (u32::from(chunk[1]) << 8)
                    | (u32::from(chunk[2]) << 16)
                    | (u32::from(chunk[3]) << 24);
            }
            cursor += n;
        }
    }
    debug_assert_eq!(cursor, heap_texels);
    // ── ⑤ 侧表 SSBO 面（texmeta/tritex;uv 字节面由 sink 直派生）。texmeta
    //    头 [0]=头表项数 [1]=0 [2]=slot_count;槽 [sb+7]=mip_count。tritex
    //    步幅 2 [slot, k_tri]——k_tri = sqrt(uv_area/world_area) 装配期逐
    //    三角预算（mip 选择 UV 密度项;退化面/常量面 0 → lod0）──
    let mut texmeta = vec![
        header_entries as f32,
        0.0,
        n_map as f32,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ];
    for s in &slots {
        texmeta.extend_from_slice(&[
            0.0,
            0.0,
            s.width as f32,
            s.height as f32,
            s.mod_rgb[0],
            s.mod_rgb[1],
            s.mod_rgb[2],
            s.mip_count as f32,
        ]);
    }
    let slot_of = |mi: u32| -> f32 {
        slots
            .iter()
            .position(|s| s.material_index == mi)
            .map(|p| p as f32)
            .unwrap_or(-1.0)
    };
    let mut tritex = Vec::with_capacity(scene.tri_mat.len() * 2);
    let mut tex_tris = 0usize;
    for (t, &mi) in scene.tri_mat.iter().enumerate() {
        let s = if mi == SLAB_TRI_NONE { -1.0 } else { slot_of(mi) };
        let k_tri = if s >= 0.0 {
            tex_tris += 1;
            let ub = t * 6;
            let du1 = tri_uv[ub + 2] - tri_uv[ub];
            let dv1 = tri_uv[ub + 3] - tri_uv[ub + 1];
            let du2 = tri_uv[ub + 4] - tri_uv[ub];
            let dv2 = tri_uv[ub + 5] - tri_uv[ub + 1];
            // ×2 面积（比值内约去;abs 覆盖 UV 镜像绕序）。
            let uv_area2 = (du1 * dv2 - dv1 * du2).abs();
            let idx = scene.indices[t];
            let p0 = scene.positions[idx[0] as usize];
            let p1 = scene.positions[idx[1] as usize];
            let p2 = scene.positions[idx[2] as usize];
            let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
            let cx = e1[1] * e2[2] - e1[2] * e2[1];
            let cy = e1[2] * e2[0] - e1[0] * e2[2];
            let cz = e1[0] * e2[1] - e1[1] * e2[0];
            let w_area2 = (cx * cx + cy * cy + cz * cz).sqrt();
            if w_area2 > 0.0 && uv_area2 > 0.0 && uv_area2.is_finite() {
                (uv_area2 / w_area2).sqrt()
            } else {
                0.0
            }
        } else {
            0.0
        };
        tritex.push(s);
        tritex.push(k_tri);
    }
    if tex_tris == 0 {
        return Err("映射材质零三角命中（空接线即红,fail-closed）".into());
    }
    let linlut = g31_tex_linlut();
    let atlas_bytes: Vec<u8> = atlas.iter().flat_map(|v| v.to_le_bytes()).collect();
    let linlut_bytes: Vec<u8> = linlut.iter().flat_map(|v| v.to_le_bytes()).collect();
    let atlas_digest = format!("sha256:{}", sha256_hex(&atlas_bytes));
    let linlut_digest = format!("sha256:{}", sha256_hex(&linlut_bytes));
    let eval_ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok(G31TexAssetsHeap {
        slots,
        census,
        atlas_w: 0,
        atlas_h: 0,
        atlas,
        atlas_bytes,
        atlas_digest,
        heap_header_entries: header_entries,
        heap_texels,
        linlut,
        linlut_bytes,
        linlut_digest,
        texmeta_bytes: bytes_f32(&texmeta),
        texmeta,
        tritex_bytes: bytes_f32(&tritex),
        tritex,
        texuv_bytes: bytes_f32(tri_uv),
        slots_rgba8,
        tex_tris,
        eval_ms,
    })
}

// ---------------------------------------------------------------------------
// day_0828 Phase F 灯具 emissive 贴图加性臂（--emissive-tex on 面;off 路径
// 0-byte——本段全部仅 g31_window_present --emissive-tex on 消费。烘焙件 =
// artifacts/day_0828/f_emissive/bake_emissive.py 产 .rgba8bin（PNG 侧车路线:
// 仓内无 PNG 解码器）;装配 = 4 张烘焙件追加进既有 texel heap 槽 70..73
// （头表 70×13→74×13,全 heap 重排布）+ triem 逐三角 emissive 槽号侧表
// （1 f32/tri;非灯具材质/回退材质 = −1.0）;scale 标定 = 契约 Le_c / 烘焙
// manifest mip0 线性均值_c 进 texmeta 槽 mod 位（emissive 槽 mod 语义 =
// scale,与 albedo 槽 mod 两套语义）。追加发生在探针对拍**之前** ⇒ B4 探针
// 双臂（SSBO 位级硬门 + sampler 腿 ≤1 LSB）自动覆盖 4 个 emissive 槽。
// ---------------------------------------------------------------------------

/// Phase F 契约 emissive_materials 段解析（scenes[]/lighting/
/// emissive_materials → (material_index, material_name, le_linear_rgb);
/// 字段缺失/空段 fail-closed——scale 标定分子事实源）。
#[allow(dead_code)] // day_0828 Phase F:g31_window_present 独消费面(诚实标注)
fn g31_contract_emissive_list(srow: &Json) -> Result<Vec<(u32, String, [f64; 3])>, String> {
    let arr = srow
        .get("lighting")
        .and_then(|l| l.get("emissive_materials"))
        .and_then(|v| v.as_array())
        .ok_or("契约场景行缺 lighting.emissive_materials（Phase F 消费面,fail-closed）")?;
    let mut out = Vec::with_capacity(arr.len());
    for e in arr {
        let mi = e
            .get("material_index")
            .and_then(|v| v.as_u64())
            .ok_or("emissive_materials 行缺 material_index")? as u32;
        let name = e
            .get("material_name")
            .and_then(|v| v.as_str())
            .ok_or("emissive_materials 行缺 material_name")?
            .to_owned();
        let le = e
            .get("le_linear_rgb")
            .and_then(|v| v.as_array())
            .filter(|a| a.len() == 3)
            .ok_or("emissive_materials 行缺 le_linear_rgb[3]")?;
        let le: Vec<f64> = le
            .iter()
            .map(|v| v.as_f64().ok_or("le_linear_rgb 非数"))
            .collect::<Result<_, _>>()?;
        out.push((mi, name, [le[0], le[1], le[2]]));
    }
    if out.is_empty() {
        return Err("契约 emissive_materials 空（Phase F 臂无消费面,fail-closed）".into());
    }
    Ok(out)
}

/// Phase F 烘焙容器读取（fail-closed 闭集：头 3×u32 LE [w,h,mips] + 逐级
/// RGBA8 行主序紧凑;pow2 方图 ≤ G31_TEX_TILE、完整链级数、逐级字节长度、
/// 零尾垃圾全链任一破即 Err）。返回 (w, h, levels[(w,h,rgba8)..])。
#[allow(dead_code)] // day_0828 Phase F:g31_window_present 独消费面(诚实标注)
fn g31_rgba8bin_read(path: &Path) -> Result<(u32, u32, Vec<(u32, u32, Vec<u8>)>), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读 {}: {e}", path.display()))?;
    if bytes.len() < 12 {
        return Err(format!("{} 头截断（<12B）", path.display()));
    }
    let rd = |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    let (w, h, mips) = (rd(0), rd(4), rd(8));
    if w == 0 || h == 0 || w != h || !w.is_power_of_two() || w > G31_TEX_TILE {
        return Err(format!(
            "{} 尺寸 {w}x{h} 越 pow2 方图 ≤ {G31_TEX_TILE} 闭集（fail-closed）",
            path.display()
        ));
    }
    let full_chain = w.max(h).trailing_zeros() + 1;
    if mips != full_chain {
        return Err(format!(
            "{} 级数 {mips} ≠ 完整链 {full_chain}（烘焙件必须全链,fail-closed）",
            path.display()
        ));
    }
    let mut levels: Vec<(u32, u32, Vec<u8>)> = Vec::with_capacity(mips as usize);
    let mut cursor = 12usize;
    for l in 0..mips {
        let lw = (w >> l).max(1);
        let lh = (h >> l).max(1);
        let need = (lw as usize) * (lh as usize) * 4;
        let px = bytes
            .get(cursor..cursor + need)
            .ok_or_else(|| {
                format!(
                    "{} 体截断: mip {l} 需 {need}B, 存 {}",
                    path.display(),
                    bytes.len().saturating_sub(cursor)
                )
            })?
            .to_vec();
        cursor += need;
        levels.push((lw, lh, px));
    }
    if cursor != bytes.len() {
        return Err(format!(
            "{} 尾部越界字节 {}（容器闭集破坏,fail-closed）",
            path.display(),
            bytes.len() - cursor
        ));
    }
    Ok((w, h, levels))
}

/// Phase F emissive 槽登记行（evidence 面）。
#[allow(dead_code)] // day_0828 Phase F:g31_window_present 独消费面(诚实标注)
struct G31EmissiveSlotRow {
    slot: usize,
    material_index: u32,
    material_name: String,
    file: String,
    source_sha256: String,
    output_sha256: String,
    src_width: u32,
    src_height: u32,
    stored_width: u32,
    stored_height: u32,
    mip_count: u32,
    le_linear_rgb: [f64; 3],
    tex_linear_mean_rgb: [f64; 3],
    scale_rgb: [f32; 3],
    tris: usize,
    /// 任一通道均值 ≤1e-6 ⇒ 该材质整体回退 mats 均值路径（triem = −1）。
    fallback: bool,
}

/// Phase F emissive 资产面（triem 侧表 + 登记行;heap/texmeta/slots 扩容
/// 直接 mutate 进 G31TexAssetsHeap——追加须在探针对拍前施加,探针自动覆盖）。
#[allow(dead_code)] // day_0828 Phase F:g31_window_present 独消费面(诚实标注)
struct G31EmissiveAssets {
    triem: Vec<f32>,
    triem_bytes: Vec<u8>,
    em_tris: usize,
    rows: Vec<G31EmissiveSlotRow>,
    manifest_path: String,
    manifest_sha256: String,
    /// heap 追加 u32 数（头表增量 + texel 段;×4 = 字节增量）。
    appended_texels: usize,
    eval_ms: f64,
}

/// Phase F emissive 贴图装配（fail-closed 闭集：manifest 缺件/字段缺失/
/// sha256 失配/容器破/契约-烘焙材质失配/映射零三角任一破即 Err,不静默降级）。
/// `em_mats` = 契约 emissive_materials 段 (material_index, material_name,
/// le_linear_rgb)。追加语义：heap 头表 slots×13 → (slots+N)×13 全 heap 重
/// 排布（既有偏移 +shift,texel 段字节不动）+ 4 槽 texel 段尾接（cap-1024
/// 起级律与 DDS 槽同律）;texmeta 头 [0]/[2] 与逐槽行同步;slots/slots_rgba8
/// 追加（探针/评估两腿自动覆盖）;tritex 0-byte 不触（albedo 映射不变）。
#[allow(dead_code)] // day_0828 Phase F:g31_window_present 独消费面(诚实标注)
fn g31_emissive_append(
    tex: &mut G31TexAssetsHeap,
    tri_mat: &[u32],
    em_mats: &[(u32, String, [f64; 3])],
    dir: &str,
) -> Result<G31EmissiveAssets, String> {
    let t0 = std::time::Instant::now();
    if em_mats.is_empty() {
        return Err("契约 emissive_materials 空（Phase F 臂无消费面,fail-closed）".into());
    }
    // ── ① 烘焙 manifest 装载（fail-closed:缺件即红）──
    let manifest_path = format!("{dir}/manifest.json");
    let mtext = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("emissive 烘焙 manifest 缺件 {manifest_path}: {e}（先跑 bake_emissive.py,fail-closed）"))?;
    let manifest_sha256 = format!("sha256:{}", sha256_hex(mtext.as_bytes()));
    let mdoc = json_parse(&mtext).map_err(|e| format!("manifest JSON: {e}"))?;
    let entries = mdoc
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or("manifest 缺 entries 数组")?;
    // ── ② 逐契约材质:manifest 行配对 + 容器读取 + 三重 sha 互核 + scale ──
    struct Pending {
        row: G31EmissiveSlotRow,
        stored: Vec<(u32, u32, Vec<u8>)>,
    }
    let mut pend: Vec<Pending> = Vec::with_capacity(em_mats.len());
    for (mi, name, le) in em_mats {
        let e = entries
            .iter()
            .find(|e| e.get("material_index").and_then(|v| v.as_u64()) == Some(u64::from(*mi)))
            .ok_or_else(|| format!("manifest 缺 material_index={mi}（{name}）行（fail-closed）"))?;
        let ename = e
            .get("material_name")
            .and_then(|v| v.as_str())
            .ok_or("manifest 行缺 material_name")?;
        if ename != name {
            return Err(format!(
                "manifest 行 {mi} 材质名 {ename} ≠ 契约 {name}（配对失配,fail-closed）"
            ));
        }
        let file = e
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or("manifest 行缺 file")?
            .to_owned();
        let out_sha = e
            .get("output_sha256")
            .and_then(|v| v.as_str())
            .ok_or("manifest 行缺 output_sha256")?
            .to_owned();
        let src_sha = e
            .get("source_sha256")
            .and_then(|v| v.as_str())
            .ok_or("manifest 行缺 source_sha256")?
            .to_owned();
        let mip0_sha = e
            .get("mip0_rgba8_sha256")
            .and_then(|v| v.as_str())
            .ok_or("manifest 行缺 mip0_rgba8_sha256")?;
        let mean = e
            .get("linear_mean_rgb")
            .and_then(|v| v.as_array())
            .filter(|a| a.len() == 3)
            .ok_or("manifest 行缺 linear_mean_rgb[3]")?;
        let mean: Vec<f64> = mean
            .iter()
            .map(|v| v.as_f64().ok_or("linear_mean_rgb 非数"))
            .collect::<Result<_, _>>()?;
        let path = Path::new(dir).join(&file);
        let blob = std::fs::read(&path)
            .map_err(|e2| format!("emissive 烘焙件缺件 {}: {e2}（fail-closed）", path.display()))?;
        let got_sha = format!("sha256:{}", sha256_hex(&blob));
        if got_sha != out_sha {
            return Err(format!(
                "{} sha256 {got_sha} ≠ manifest {out_sha}（烘焙件漂移,fail-closed）",
                path.display()
            ));
        }
        let (w, h, levels) = g31_rgba8bin_read(&path)?;
        let got_mip0 = format!("sha256:{}", sha256_hex(&levels[0].2));
        if got_mip0 != mip0_sha {
            return Err(format!(
                "{} mip0 rgba8 digest ≠ manifest（容器/烘焙链失配,fail-closed）",
                path.display()
            ));
        }
        // cap 起级（DDS 槽同律:首个 ≤ G31_TEX_CAP 级起搬）。
        let start = levels
            .iter()
            .position(|(lw, lh, _)| *lw <= G31_TEX_CAP && *lh <= G31_TEX_CAP)
            .ok_or_else(|| format!("{} 全链无 ≤{}² 级（fail-closed）", path.display(), G31_TEX_CAP))?;
        let stored: Vec<(u32, u32, Vec<u8>)> = levels[start..].to_vec();
        if stored.len() > G31_TEX_MIP_SLOTS {
            return Err(format!(
                "{} 存储级数 {} 越头表槽位 {G31_TEX_MIP_SLOTS}（fail-closed）",
                path.display(),
                stored.len()
            ));
        }
        // scale = 契约 Le_c / mip0 线性均值_c（能量守恒标定;通道均值 ≤1e-6 ⇒
        // scale=0 且材质整体回退均值路径 + 登记）。
        let mut scale = [0.0f32; 3];
        let mut fallback = false;
        for c in 0..3 {
            if mean[c] <= 1e-6 {
                scale[c] = 0.0;
                fallback = true;
            } else {
                scale[c] = (le[c] / mean[c]) as f32;
            }
        }
        let tris = tri_mat.iter().filter(|&&m| m == *mi).count();
        pend.push(Pending {
            row: G31EmissiveSlotRow {
                slot: 0, // 槽号下方统一回填
                material_index: *mi,
                material_name: name.clone(),
                file,
                source_sha256: src_sha,
                output_sha256: out_sha,
                src_width: w,
                src_height: h,
                stored_width: stored[0].0,
                stored_height: stored[0].1,
                mip_count: stored.len() as u32,
                le_linear_rgb: *le,
                tex_linear_mean_rgb: [mean[0], mean[1], mean[2]],
                scale_rgb: scale,
                tris,
                fallback,
            },
            stored,
        });
    }
    // ── ③ heap 全重排布（头表 slots×13 → (slots+N)×13:既有偏移 +shift,
    //    texel 段字节 0-byte 平移;新槽 texel 段尾接）──
    let old_slots = tex.slots.len();
    let old_hdr = tex.heap_header_entries;
    if old_hdr != old_slots * G31_TEX_MIP_SLOTS {
        return Err(format!(
            "heap 头表项数 {old_hdr} ≠ slots×13 = {}（前置形态破坏,fail-closed）",
            old_slots * G31_TEX_MIP_SLOTS
        ));
    }
    let new_slots_n = old_slots + pend.len();
    let new_hdr = new_slots_n * G31_TEX_MIP_SLOTS;
    let shift = (new_hdr - old_hdr) as u32;
    let body_len = tex.atlas.len() - old_hdr;
    let append_texels: usize = pend
        .iter()
        .map(|p| p.stored.iter().map(|(lw, lh, _)| (*lw as usize) * (*lh as usize)).sum::<usize>())
        .sum();
    let new_total = new_hdr + body_len + append_texels;
    if (new_total as u64) * 4 > G31_TEX_HEAP_MAX_BYTES {
        return Err(format!(
            "emissive 扩容后 heap {}B 越保守界 {G31_TEX_HEAP_MAX_BYTES}B（fail-closed）",
            new_total * 4
        ));
    }
    let mut new_atlas: Vec<u32> = Vec::with_capacity(new_total);
    new_atlas.extend(tex.atlas[..old_hdr].iter().map(|v| v + shift));
    // 新槽头表（偏移接在既有 texel 段之后;缺级槽位重复末级——kernel lod 钳
    // mips−1 冗余保底,DDS 槽同律）。
    let mut cur = new_hdr + body_len;
    for p in &pend {
        let base_entry = new_atlas.len();
        for (lw, lh, _) in &p.stored {
            new_atlas.push(cur as u32);
            cur += (*lw as usize) * (*lh as usize);
        }
        let last = new_atlas[base_entry + p.stored.len() - 1];
        for _ in p.stored.len()..G31_TEX_MIP_SLOTS {
            new_atlas.push(last);
        }
    }
    debug_assert_eq!(new_atlas.len(), new_hdr);
    new_atlas.extend_from_slice(&tex.atlas[old_hdr..]);
    for p in &pend {
        for (_, _, px) in &p.stored {
            for chunk in px.chunks_exact(4) {
                new_atlas.push(
                    u32::from(chunk[0])
                        | (u32::from(chunk[1]) << 8)
                        | (u32::from(chunk[2]) << 16)
                        | (u32::from(chunk[3]) << 24),
                );
            }
        }
    }
    debug_assert_eq!(new_atlas.len(), new_total);
    // ── ④ 侧表/登记同步（texmeta 头 [0]/[2] + 逐槽行;slots/slots_rgba8 追加
    //    ——探针/评估两腿自动覆盖;tritex 不触）──
    tex.texmeta[0] = new_hdr as f32;
    tex.texmeta[2] = new_slots_n as f32;
    let mut rows: Vec<G31EmissiveSlotRow> = Vec::with_capacity(pend.len());
    for (k, p) in pend.into_iter().enumerate() {
        let slot = old_slots + k;
        let mut row = p.row;
        row.slot = slot;
        tex.texmeta.extend_from_slice(&[
            0.0,
            0.0,
            row.stored_width as f32,
            row.stored_height as f32,
            row.scale_rgb[0],
            row.scale_rgb[1],
            row.scale_rgb[2],
            row.mip_count as f32,
        ]);
        let mip_digests: Vec<String> = p
            .stored
            .iter()
            .map(|(_, _, px)| format!("sha256:{}", sha256_hex(px)))
            .collect();
        tex.slots.push(G31TexSlotHeap {
            material_index: row.material_index,
            material_name: row.material_name.clone(),
            tris: row.tris,
            texture_uri: row.file.clone(),
            width: row.stored_width,
            height: row.stored_height,
            src_width: row.src_width,
            src_height: row.src_height,
            dds_format: "png-rgba8-baked".to_owned(),
            // manifest 互核域 = Phase F 烘焙 manifest（G11.3 DDS manifest 无
            // PNG 条目;行内 dds_format 自述来源,emissive evidence 块注明）。
            manifest_source_digest: Some(row.source_sha256.clone()),
            rgba8_digest: format!("sha256:{}", sha256_hex(&p.stored[0].2)),
            manifest_rgba8_digest: None,
            mip_count: row.mip_count,
            mip_digests,
            mip_truncated: false,
            origin_x: 0,
            origin_y: 0,
            mod_rgb: row.scale_rgb,
        });
        tex.slots_rgba8.push(p.stored[0].2.clone());
        rows.push(row);
    }
    tex.heap_header_entries = new_hdr;
    tex.heap_texels = new_total;
    tex.atlas = new_atlas;
    tex.atlas_bytes = tex.atlas.iter().flat_map(|v| v.to_le_bytes()).collect();
    tex.atlas_digest = format!("sha256:{}", sha256_hex(&tex.atlas_bytes));
    tex.texmeta_bytes = bytes_f32(&tex.texmeta);
    // ── ⑤ triem 侧表（1 f32/tri:灯具材质非回退 → 槽号,否则 −1.0）──
    let slot_of = |mi: u32| -> f32 {
        rows.iter()
            .find(|r| r.material_index == mi && !r.fallback)
            .map(|r| r.slot as f32)
            .unwrap_or(-1.0)
    };
    let mut triem: Vec<f32> = Vec::with_capacity(tri_mat.len());
    let mut em_tris = 0usize;
    for &mi in tri_mat {
        let s = if mi == SLAB_TRI_NONE { -1.0 } else { slot_of(mi) };
        if s >= 0.0 {
            em_tris += 1;
        }
        triem.push(s);
    }
    if em_tris == 0 {
        return Err("emissive 映射零三角命中（空接线即红,fail-closed）".into());
    }
    let triem_bytes = bytes_f32(&triem);
    Ok(G31EmissiveAssets {
        triem,
        triem_bytes,
        em_tris,
        rows,
        manifest_path,
        manifest_sha256,
        appended_texels: (new_hdr - old_hdr) + append_texels,
        eval_ms: t0.elapsed().as_secs_f64() * 1000.0,
    })
}

/// B4 探针臂对拍报告（evidence 登记面;SSBO 腿位级硬门 + sampler 腿结构容差）。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
struct G31TexProbeReport {
    probe_count: usize,
    eval_ms: f64,
    ssbo_p100: f64,
    ssbo_bitexact: bool,
    ssbo_device_digest: String,
    ssbo_host_digest: String,
    ssbo_double_run_bitexact: bool,
    sampler_max_lsb: u32,
    sampler_bitexact: bool,
    sampler_digest: String,
    sampler_host_digest: String,
    nonconstant_slots: usize,
}

/// B4 探针 SSBO 腿（kernels/g31_texture_probe.rx SPV 经 vk::run_compute 单
/// dispatch [N,1,1];NoContraction 注入面 = host 侧 SPV 后处理,文件 0-byte——
/// 与 host 参考同 op 序位级对拍 p100=0 硬门的驱动 FMA 收缩禁面）。
/// day_0828 F6 双形态回正：原形态原签名恢复（探针步幅 3;g34_full_lane 系经
/// [`g31_tex_probe_evaluate`] 消费）;mip 形态 = [`g31_tex_probe_device_mip`]。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_tex_probe_device(
    assets: &G31TexAssets,
    probes: &[(u32, f32, f32)],
    spv_path: &str,
) -> Result<(Vec<f32>, f64), String> {
    if !vk::vulkan_available() {
        return Err("vulkan loader 不可用".into());
    }
    let bytes = std::fs::read(spv_path).map_err(|e| format!("读 probe SPV {spv_path}: {e}"))?;
    if bytes.len() % 4 != 0 {
        return Err("probe SPIR-V 字节数非 4 对齐".into());
    }
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let words = spv_inject_no_contraction(&words);
    let entry = vk::entry_point_name(&words).ok_or("probe SPV 无 OpEntryPoint")?;
    let mut probe_f: Vec<f32> = Vec::with_capacity(probes.len() * 3);
    for &(slot, u, v) in probes {
        probe_f.push(slot as f32);
        probe_f.push(u);
        probe_f.push(v);
    }
    let mut params = vec![probes.len() as f32];
    params.resize(8, 0.0);
    let mut bufs = vec![
        bytes_f32(&probe_f),
        assets.texmeta_bytes.clone(),
        assets.atlas_bytes.clone(),
        assets.linlut_bytes.clone(),
        bytes_f32(&params),
        vec![0u8; probes.len() * 12],
    ];
    let t0 = std::time::Instant::now();
    vk::run_compute(&words, &entry, &mut bufs, &[], [probes.len() as u32, 1, 1])
        .map_err(|e| format!("probe SSBO 腿 device dispatch 失败: {e}"))?;
    let eval_ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok((read_f32(&bufs[5]), eval_ms))
}

/// [`g31_tex_probe_device`] 的 day_0828 Phase B mip 形态（F6 双形态回正
/// 改名）：探针步幅 3→4（[slot,u,v,lod]——lod 显式注入面）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
fn g31_tex_probe_device_mip(
    assets: &G31TexAssetsHeap,
    probes: &[(u32, f32, f32, u32)],
    spv_path: &str,
) -> Result<(Vec<f32>, f64), String> {
    if !vk::vulkan_available() {
        return Err("vulkan loader 不可用".into());
    }
    let bytes = std::fs::read(spv_path).map_err(|e| format!("读 probe SPV {spv_path}: {e}"))?;
    if bytes.len() % 4 != 0 {
        return Err("probe SPIR-V 字节数非 4 对齐".into());
    }
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let words = spv_inject_no_contraction(&words);
    let entry = vk::entry_point_name(&words).ok_or("probe SPV 无 OpEntryPoint")?;
    let mut probe_f: Vec<f32> = Vec::with_capacity(probes.len() * 4);
    for &(slot, u, v, lod) in probes {
        probe_f.push(slot as f32);
        probe_f.push(u);
        probe_f.push(v);
        probe_f.push(lod as f32);
    }
    let mut params = vec![probes.len() as f32];
    params.resize(8, 0.0);
    let mut bufs = vec![
        bytes_f32(&probe_f),
        assets.texmeta_bytes.clone(),
        assets.atlas_bytes.clone(),
        assets.linlut_bytes.clone(),
        bytes_f32(&params),
        vec![0u8; probes.len() * 12],
    ];
    let t0 = std::time::Instant::now();
    vk::run_compute(&words, &entry, &mut bufs, &[], [probes.len() as u32, 1, 1])
        .map_err(|e| format!("probe SSBO 腿 device dispatch 失败: {e}"))?;
    let eval_ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok((read_f32(&bufs[5]), eval_ms))
}

/// B4 sampler 腿（真 GPU 纹理对象:image/view 经 vk::GraphicsResource::
/// Texture2D + sampler 经 sampler.rs SamplerDesc→vk_fields→VkSampler〔
/// vk.rs sampler_create_info 同一事实源〕;全屏 vertex + sample_lod fragment
/// 〔vk::sampling_shaders_spv,G3.3 生产在案着色器 0-byte 消费;单层纹理 LOD
/// 钳到 mip0 = max_lod 0.0〕;逐槽单跑 = 24 探针 quad 各覆 1 像素,回读
/// RGBA8 附件像素）。返回逐探针 [r,g,b,a]（probes 序）。day_0828 F6 双形态
/// 回正：原形态原签名恢复（探针步幅 3;g34_full_lane 系经
/// [`g31_tex_probe_evaluate`] 消费）;mip 形态 = [`g31_tex_sampler_leg_mip`]。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_tex_sampler_leg(
    assets: &G31TexAssets,
    probes: &[(u32, f32, f32)],
) -> Result<Vec<[u8; 4]>, String> {
    use rurix_rt::sampler::{Address, Filter, SamplerDesc};
    use rurix_rt::vk::{GraphicsResource, TextureData};

    let sh = vk::sampling_shaders_spv();
    if sh.fullscreen_vs.is_empty() || sh.sample_lod_fs.is_empty() {
        return Err("采样模式着色器为空（build.rs codegen 降级,fail-closed）".into());
    }
    let to_words = |bytes: &[u8]| -> Vec<u32> {
        bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    };
    let vs = to_words(sh.fullscreen_vs);
    let fs = to_words(sh.sample_lod_fs);
    const W: u32 = 16;
    const H: u32 = 16;
    const STRIDE: u32 = 24;
    let attrs = [(0u32, 109u32, 0u32), (1u32, 103u32, 16u32)]; // pos vec4 + uv vec2
    // sampler 状态 = bistro glTF sampler 默认 REPEAT 语义面（wrap 域回绕与生产
    // kernel fract 同语义;线性过滤 + mip0 单层钳 = max_lod 0.0）。
    let sampler = GraphicsResource::Sampler(SamplerDesc {
        filter: Filter::Linear,
        address: Address::Wrap,
        max_anisotropy: 1,
        lod_bias: 0.0,
        min_lod: 0.0,
        max_lod: 0.0,
        compare: None,
    });
    let mut out = vec![[0u8; 4]; probes.len()];
    for (k, slot) in assets.slots.iter().enumerate() {
        let idx: Vec<usize> = probes
            .iter()
            .enumerate()
            .filter_map(|(pi, &(s, _, _))| (s as usize == k).then_some(pi))
            .collect();
        if idx.is_empty() {
            continue;
        }
        let mut verts = Vec::with_capacity(idx.len() * 6 * STRIDE as usize);
        for (j, &pi) in idx.iter().enumerate() {
            let (_, u, v) = probes[pi];
            let px = (j % W as usize) as f32;
            let py = (j / W as usize) as f32;
            // 内缩 1/4 像素 quad（像素中心唯一覆盖,邻像素零片段;2^-4/2^-6
            // 分数 f32 精确）。
            let x0 = -1.0 + 2.0 * (px + 0.25) / W as f32;
            let x1 = -1.0 + 2.0 * (px + 0.75) / W as f32;
            let y0 = -1.0 + 2.0 * (py + 0.25) / H as f32;
            let y1 = -1.0 + 2.0 * (py + 0.75) / H as f32;
            let corners = [
                (x0, y0),
                (x1, y0),
                (x0, y1),
                (x0, y1),
                (x1, y0),
                (x1, y1),
            ];
            for (cx, cy) in corners {
                verts.extend_from_slice(&cx.to_le_bytes());
                verts.extend_from_slice(&cy.to_le_bytes());
                verts.extend_from_slice(&0.0f32.to_le_bytes());
                verts.extend_from_slice(&1.0f32.to_le_bytes());
                verts.extend_from_slice(&u.to_le_bytes());
                verts.extend_from_slice(&v.to_le_bytes());
            }
        }
        let resources = [
            GraphicsResource::Texture2D {
                width: slot.width,
                height: slot.height,
                data: TextureData::Rgba8(vec![assets.slots_rgba8[k].clone()]),
            },
            sampler.clone(),
        ];
        let pxbuf = vk::run_graphics_offscreen_v2(
            &vs,
            &fs,
            &verts,
            STRIDE,
            &attrs,
            W,
            H,
            [0.0, 0.0, 0.0, 1.0],
            &resources,
        )
        .map_err(|e| format!("sampler 腿 slot {k}（{}）渲染: {e}", slot.texture_uri))?;
        for (j, &pi) in idx.iter().enumerate() {
            let x = j % W as usize;
            let y = j / W as usize;
            let o = (y * W as usize + x) * 4;
            out[pi] = [pxbuf[o], pxbuf[o + 1], pxbuf[o + 2], pxbuf[o + 3]];
        }
    }
    Ok(out)
}

/// [`g31_tex_sampler_leg`] 的 day_0828 Phase B mip 形态（F6 双形态回正
/// 改名）：硬件腿 = 存储基级单层纹理（max_lod 0）⇒ 仅消费 lod==0 探针子集
/// （>0 级探针归 SSBO 腿逐级对拍面）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
fn g31_tex_sampler_leg_mip(
    assets: &G31TexAssetsHeap,
    probes: &[(u32, f32, f32, u32)],
) -> Result<Vec<[u8; 4]>, String> {
    use rurix_rt::sampler::{Address, Filter, SamplerDesc};
    use rurix_rt::vk::{GraphicsResource, TextureData};

    let sh = vk::sampling_shaders_spv();
    if sh.fullscreen_vs.is_empty() || sh.sample_lod_fs.is_empty() {
        return Err("采样模式着色器为空（build.rs codegen 降级,fail-closed）".into());
    }
    let to_words = |bytes: &[u8]| -> Vec<u32> {
        bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    };
    let vs = to_words(sh.fullscreen_vs);
    let fs = to_words(sh.sample_lod_fs);
    const W: u32 = 16;
    const H: u32 = 16;
    const STRIDE: u32 = 24;
    let attrs = [(0u32, 109u32, 0u32), (1u32, 103u32, 16u32)]; // pos vec4 + uv vec2
    // sampler 状态 = bistro glTF sampler 默认 REPEAT 语义面（wrap 域回绕与生产
    // kernel fract 同语义;线性过滤 + mip0 单层钳 = max_lod 0.0）。
    let sampler = GraphicsResource::Sampler(SamplerDesc {
        filter: Filter::Linear,
        address: Address::Wrap,
        max_anisotropy: 1,
        lod_bias: 0.0,
        min_lod: 0.0,
        max_lod: 0.0,
        compare: None,
    });
    let mut out = vec![[0u8; 4]; probes.len()];
    for (k, slot) in assets.slots.iter().enumerate() {
        // day_0828 Phase B：硬件腿 = 存储基级单层纹理（max_lod 0）⇒ 仅消费
        // lod==0 探针子集（>0 级探针归 SSBO 腿逐级对拍面）。
        let idx: Vec<usize> = probes
            .iter()
            .enumerate()
            .filter_map(|(pi, &(s, _, _, lod))| (s as usize == k && lod == 0).then_some(pi))
            .collect();
        if idx.is_empty() {
            continue;
        }
        let mut verts = Vec::with_capacity(idx.len() * 6 * STRIDE as usize);
        for (j, &pi) in idx.iter().enumerate() {
            let (_, u, v, _) = probes[pi];
            let px = (j % W as usize) as f32;
            let py = (j / W as usize) as f32;
            // 内缩 1/4 像素 quad（像素中心唯一覆盖,邻像素零片段;2^-4/2^-6
            // 分数 f32 精确）。
            let x0 = -1.0 + 2.0 * (px + 0.25) / W as f32;
            let x1 = -1.0 + 2.0 * (px + 0.75) / W as f32;
            let y0 = -1.0 + 2.0 * (py + 0.25) / H as f32;
            let y1 = -1.0 + 2.0 * (py + 0.75) / H as f32;
            let corners = [
                (x0, y0),
                (x1, y0),
                (x0, y1),
                (x0, y1),
                (x1, y0),
                (x1, y1),
            ];
            for (cx, cy) in corners {
                verts.extend_from_slice(&cx.to_le_bytes());
                verts.extend_from_slice(&cy.to_le_bytes());
                verts.extend_from_slice(&0.0f32.to_le_bytes());
                verts.extend_from_slice(&1.0f32.to_le_bytes());
                verts.extend_from_slice(&u.to_le_bytes());
                verts.extend_from_slice(&v.to_le_bytes());
            }
        }
        let resources = [
            GraphicsResource::Texture2D {
                width: slot.width,
                height: slot.height,
                data: TextureData::Rgba8(vec![assets.slots_rgba8[k].clone()]),
            },
            sampler.clone(),
        ];
        let pxbuf = vk::run_graphics_offscreen_v2(
            &vs,
            &fs,
            &verts,
            STRIDE,
            &attrs,
            W,
            H,
            [0.0, 0.0, 0.0, 1.0],
            &resources,
        )
        .map_err(|e| format!("sampler 腿 slot {k}（{}）渲染: {e}", slot.texture_uri))?;
        for (j, &pi) in idx.iter().enumerate() {
            let x = j % W as usize;
            let y = j / W as usize;
            let o = (y * W as usize + x) * 4;
            out[pi] = [pxbuf[o], pxbuf[o + 1], pxbuf[o + 2], pxbuf[o + 3]];
        }
    }
    Ok(out)
}

/// B4 探针双臂对拍装配（host 参考两臂同源:SSBO 腿 = g31_tex_host_sample
/// f32 位级;sampler 腿 = g31_tex_host_sample_srgb 8-bit 量化域,容差结构
/// 依据 = 硬件过滤权重量化 ≤2^-8 ⇒ ≤1 LSB @ quantum 1/255,位级一致 =
/// 更强终态亦合法）。nonconstant_slots = 槽内探针输出非全等计数（防空
/// 接线冒充面）。day_0828 F6 双形态回正：原形态原签名恢复（探针步幅 3;
/// g34_full_lane 系消费面）;mip 形态 = [`g31_tex_probe_evaluate_mip`]。
#[allow(dead_code)] // G31+ 波 B Task B4:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_tex_probe_evaluate(
    assets: &G31TexAssets,
    probes: &[(u32, f32, f32)],
    spv_path: &str,
) -> Result<G31TexProbeReport, String> {
    let t0 = std::time::Instant::now();
    // ── SSBO 腿（device 双臂双跑 + host f32 参考位级对拍）──
    let (dev_a, _) = g31_tex_probe_device(assets, probes, spv_path)?;
    let (dev_b, _) = g31_tex_probe_device(assets, probes, spv_path)?;
    if let Some(k) = dev_a.iter().position(|x| !x.is_finite()) {
        return Err(format!(
            "probe SSBO 腿判据⓪失败: 探针 {} device 输出非有限（有限性一等断言先于聚合,RFC-0046 §1.4 F3 同律）",
            k / 3
        ));
    }
    let ssbo_double = dev_a == dev_b;
    let mut host_ref: Vec<f32> = Vec::with_capacity(probes.len() * 3);
    for &(slot, u, v) in probes {
        let s = g31_tex_host_sample(
            &assets.texmeta,
            &assets.atlas,
            &assets.linlut,
            slot as usize,
            u,
            v,
        );
        host_ref.extend_from_slice(&s);
    }
    let mut p100 = 0.0f64;
    for k in 0..probes.len() * 3 {
        let d = (f64::from(dev_a[k]) - f64::from(host_ref[k])).abs();
        if d > p100 {
            p100 = d;
        }
    }
    let dev_bytes: Vec<u8> = dev_a.iter().flat_map(|v| v.to_le_bytes()).collect();
    let host_bytes: Vec<u8> = host_ref.iter().flat_map(|v| v.to_le_bytes()).collect();
    // ── sampler 腿（硬件采样 vs host srgb 域参考;结构容差 ≤1 LSB）──
    let hw = g31_tex_sampler_leg(assets, probes)?;
    let mut max_lsb = 0u32;
    let mut hw_bytes = Vec::with_capacity(probes.len() * 4);
    let mut hw_host_bytes = Vec::with_capacity(probes.len() * 4);
    for (pi, &(slot, u, v)) in probes.iter().enumerate() {
        let s = &assets.slots[slot as usize];
        let href = g31_tex_host_sample_srgb(&assets.slots_rgba8[slot as usize], s.width, s.height, u, v);
        for c in 0..4usize {
            let d = (hw[pi][c] as i32 - href[c] as i32).unsigned_abs();
            if d > max_lsb {
                max_lsb = d;
            }
        }
        hw_bytes.extend_from_slice(&hw[pi]);
        hw_host_bytes.extend_from_slice(&href);
    }
    // ── 非全等槽计数（防空接线冒充面:纹理内探针线性输出须非常量）──
    let mut nonconstant = 0usize;
    for k in 0..assets.slots.len() {
        let mut seen: Option<[u8; 4]> = None;
        let mut vary = false;
        for (pi, &(slot, _, _)) in probes.iter().enumerate() {
            if slot as usize != k {
                continue;
            }
            let cur = hw[pi];
            if let Some(prev) = seen
                && prev != cur
            {
                vary = true;
            }
            seen = Some(cur);
        }
        if vary {
            nonconstant += 1;
        }
    }
    let eval_ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok(G31TexProbeReport {
        probe_count: probes.len(),
        eval_ms,
        ssbo_p100: p100,
        ssbo_bitexact: dev_a == host_ref,
        ssbo_device_digest: format!("sha256:{}", sha256_hex(&dev_bytes)),
        ssbo_host_digest: format!("sha256:{}", sha256_hex(&host_bytes)),
        ssbo_double_run_bitexact: ssbo_double,
        sampler_max_lsb: max_lsb,
        sampler_bitexact: hw_bytes == hw_host_bytes,
        sampler_digest: format!("sha256:{}", sha256_hex(&hw_bytes)),
        sampler_host_digest: format!("sha256:{}", sha256_hex(&hw_host_bytes)),
        nonconstant_slots: nonconstant,
    })
}

/// [`g31_tex_probe_evaluate`] 的 day_0828 Phase B mip 形态（F6 双形态回正
/// 改名）：探针含 mip 维——lod 显式注入两腿同源;sampler 腿仅 lod==0 探针
/// 子集对拍（>0 级探针归 SSBO 腿位级硬门覆盖）。
#[allow(dead_code)] // day_0828 Phase B:g31_window_present 独消费面(诚实标注)
fn g31_tex_probe_evaluate_mip(
    assets: &G31TexAssetsHeap,
    probes: &[(u32, f32, f32, u32)],
    spv_path: &str,
) -> Result<G31TexProbeReport, String> {
    let t0 = std::time::Instant::now();
    // ── SSBO 腿（device 双臂双跑 + host f32 参考位级对拍;day_0828 Phase B
    //    探针含 mip 维——lod 显式注入两腿同源）──
    let (dev_a, _) = g31_tex_probe_device_mip(assets, probes, spv_path)?;
    let (dev_b, _) = g31_tex_probe_device_mip(assets, probes, spv_path)?;
    if let Some(k) = dev_a.iter().position(|x| !x.is_finite()) {
        return Err(format!(
            "probe SSBO 腿判据⓪失败: 探针 {} device 输出非有限（有限性一等断言先于聚合,RFC-0046 §1.4 F3 同律）",
            k / 3
        ));
    }
    let ssbo_double = dev_a == dev_b;
    let mut host_ref: Vec<f32> = Vec::with_capacity(probes.len() * 3);
    for &(slot, u, v, lod) in probes {
        let s = g31_tex_host_sample_mip(
            &assets.texmeta,
            &assets.atlas,
            &assets.linlut,
            slot as usize,
            u,
            v,
            lod as usize,
        );
        host_ref.extend_from_slice(&s);
    }
    let mut p100 = 0.0f64;
    for k in 0..probes.len() * 3 {
        let d = (f64::from(dev_a[k]) - f64::from(host_ref[k])).abs();
        if d > p100 {
            p100 = d;
        }
    }
    let dev_bytes: Vec<u8> = dev_a.iter().flat_map(|v| v.to_le_bytes()).collect();
    let host_bytes: Vec<u8> = host_ref.iter().flat_map(|v| v.to_le_bytes()).collect();
    // ── sampler 腿（硬件采样 vs host srgb 域参考;结构容差 ≤1 LSB。
    //    day_0828 Phase B：硬件腿 = 存储基级单层（max_lod 0）⇒ 仅 lod==0
    //    探针子集对拍;>0 级探针归 SSBO 腿位级硬门覆盖）──
    let hw = g31_tex_sampler_leg_mip(assets, probes)?;
    let mut max_lsb = 0u32;
    let mut hw_bytes = Vec::with_capacity(probes.len() * 4);
    let mut hw_host_bytes = Vec::with_capacity(probes.len() * 4);
    for (pi, &(slot, u, v, lod)) in probes.iter().enumerate() {
        if lod != 0 {
            continue;
        }
        let s = &assets.slots[slot as usize];
        let href = g31_tex_host_sample_srgb(&assets.slots_rgba8[slot as usize], s.width, s.height, u, v);
        for c in 0..4usize {
            let d = (hw[pi][c] as i32 - href[c] as i32).unsigned_abs();
            if d > max_lsb {
                max_lsb = d;
            }
        }
        hw_bytes.extend_from_slice(&hw[pi]);
        hw_host_bytes.extend_from_slice(&href);
    }
    // ── 非全等槽计数（防空接线冒充面:纹理内探针线性输出须非常量;lod==0 子集）──
    let mut nonconstant = 0usize;
    for k in 0..assets.slots.len() {
        let mut seen: Option<[u8; 4]> = None;
        let mut vary = false;
        for (pi, &(slot, _, _, lod)) in probes.iter().enumerate() {
            if slot as usize != k || lod != 0 {
                continue;
            }
            let cur = hw[pi];
            if let Some(prev) = seen
                && prev != cur
            {
                vary = true;
            }
            seen = Some(cur);
        }
        if vary {
            nonconstant += 1;
        }
    }
    let eval_ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok(G31TexProbeReport {
        probe_count: probes.len(),
        eval_ms,
        ssbo_p100: p100,
        ssbo_bitexact: dev_a == host_ref,
        ssbo_device_digest: format!("sha256:{}", sha256_hex(&dev_bytes)),
        ssbo_host_digest: format!("sha256:{}", sha256_hex(&host_bytes)),
        ssbo_double_run_bitexact: ssbo_double,
        sampler_max_lsb: max_lsb,
        sampler_bitexact: hw_bytes == hw_host_bytes,
        sampler_digest: format!("sha256:{}", sha256_hex(&hw_bytes)),
        sampler_host_digest: format!("sha256:{}", sha256_hex(&hw_host_bytes)),
        nonconstant_slots: nonconstant,
    })
}

// ---------------------------------------------------------------------------
// G31+ 波 C Task C13 SVT 稀疏虚拟纹理（--textures on --svt on 派生臂；RD-041
// SVT-1/2/3 行立项窗兑现；TODO #33/#34/#35）。以下全部仅 svt on 消费,
// textures off / svt off 路径逐字不触。形态（B4 纹理面在案消费,0-byte）:
//   ① SVT-1 页表:128K² 虚拟地址空间（streaming/svt.rs 常量族）→ bistro
//      图集活动区 3072 页 → 物理瓦片池（确定性 LRU,容量预算 --svt-pool-tiles）;
//   ② SVT-2 反馈:生产 kernel（g31_svt_gi.rx）采样 miss → out_req 请求缓冲
//      → host SvtStreaming::consume（读瓦片 → 次帧 buffer_uploads 上传 →
//      页表更新）——请求-驻留闭环逐帧真跑;
//   ③ SVT-3 border 复制:物理瓦片 130² 带边（页所属槽 REPEAT wrap 律）——
//      探针臂全驻留 SVT vs 整图直采位级对拍（边界聚焦律法）;
//   ④ fallback 合法面:miss = 槽均值低 mip 等效（×mod 同值）,hit 门融合
//      1·x+0·y IEEE 精确 ⇒ 全驻留臂与 B4 textures on 位级一致（锚）。
// ---------------------------------------------------------------------------

/// C13 SVT 资产面（装配期一次性构建;B4 G31TexAssetsHeap 消费面派生——
/// day_0828 F6 双形态回正:SVT 臂属 g31_window_present 独消费面,随 heap
/// 形态走;SVT 臂已 fail-closed 互斥,编译面保留）。
#[allow(dead_code)] // G31+ 波 C Task C13:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
struct G31SvtAssets {
    /// 槽表（B4 slots 的矩形面;瓦片集 wrap 律消费）。
    slot_descs: Vec<svt::SvtSlotDesc>,
    /// 瓦片集（130² 含 border;host"盘"面 = device 驻留池读取源）。
    tile_set: svt::SvtTileSet,
    /// fallback 表（槽数 × 4 f32;miss 合法面 = 槽均值低 mip 等效 ×mod）。
    fallback: Vec<f32>,
    fallback_bytes: Vec<u8>,
    /// svtmeta（8 f32 常量镜像面;kernel 参数面）。
    svtmeta: [f32; 8],
    svtmeta_bytes: Vec<u8>,
    /// 池槽数（--svt-pool-tiles 面;0 派生 = 活动页数 = 全驻留锚臂）。
    pool_tiles: u32,
    /// 全驻留锚臂标记（pool_tiles == 活动页数 ⇒ 初态全映射）。
    full_residency: bool,
    eval_ms: f64,
}

/// C13 SVT 资产装配（B4 图集面 → 瓦片集 + fallback + svtmeta;fail-closed——
/// 槽矩形/pow2/图集互核任一破即 Err,不静默降级）。
#[allow(dead_code)] // G31+ 波 C Task C13:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_svt_build(tex: &G31TexAssetsHeap, pool_tiles: u32) -> Result<G31SvtAssets, String> {
    let t0 = std::time::Instant::now();
    let slot_descs: Vec<svt::SvtSlotDesc> = tex
        .slots
        .iter()
        .map(|s| svt::SvtSlotDesc {
            origin_x: s.origin_x,
            origin_y: s.origin_y,
            width: s.width,
            height: s.height,
        })
        .collect();
    // B4 图集网格律法（origin = slot×2048）的页 → 槽映射闭包（2048 % 128 == 0
    // ⇒ 页不跨槽,结构性事实）。
    let grid_cols = G31_TEX_GRID_COLS;
    let n_slots = tex.slots.len();
    let slot_of = move |ax: u32, ay: u32| -> Option<usize> {
        let idx = (ay / G31_TEX_TILE) * grid_cols + (ax / G31_TEX_TILE);
        ((idx as usize) < n_slots).then_some(idx as usize)
    };
    let tile_set = svt::build_tile_set(
        tex.atlas_w,
        tex.atlas_h,
        &tex.atlas,
        &slot_descs,
        &slot_of,
    )
    .map_err(|e| format!("SVT 瓦片集构建: {e}"))?;
    let mod_rgb: Vec<[f32; 3]> = tex.slots.iter().map(|s| s.mod_rgb).collect();
    let fallback = svt::build_fallback_table(tex.atlas_w, &tex.atlas, &slot_descs, &mod_rgb, &tex.linlut)
        .map_err(|e| format!("SVT fallback 表构建: {e}"))?;
    let pool_tiles_eff = if pool_tiles == 0 {
        tile_set.page_total()
    } else {
        pool_tiles
    };
    if pool_tiles_eff == 0 {
        return Err("SVT 池槽数为 0（活动瓦片集为空,fail-closed）".into());
    }
    let svtmeta: [f32; 8] = [
        svt::SVT_PAGE_TABLE_DIM as f32,
        svt::SVT_TILE_DIM as f32,
        svt::SVT_PHYS_DIM as f32,
        svt::SVT_PHYS_TEXELS as f32,
        tile_set.pages_x as f32,
        tile_set.pages_y as f32,
        pool_tiles_eff as f32,
        0.0,
    ];
    let full_residency = pool_tiles_eff == tile_set.page_total();
    let eval_ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok(G31SvtAssets {
        slot_descs,
        tile_set,
        fallback_bytes: bytes_f32(&fallback),
        fallback,
        svtmeta_bytes: bytes_f32(&svtmeta),
        svtmeta,
        pool_tiles: pool_tiles_eff,
        full_residency,
        eval_ms,
    })
}

/// C13 SVT 流送状态初态（全驻留锚臂 = new_full,小池压力臂 = new_cold）。
#[allow(dead_code)] // G31+ 波 C Task C13:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_svt_streaming_init(assets: &G31SvtAssets) -> Result<svt::SvtStreaming, String> {
    if assets.full_residency {
        svt::SvtStreaming::new_full(assets.tile_set.clone()).map_err(|e| e.to_string())
    } else {
        svt::SvtStreaming::new_cold(assets.tile_set.clone(), assets.pool_tiles)
            .map_err(|e| e.to_string())
    }
}

/// C13 SVT host 采样参考（与 kernels/g31_svt_{gi,probe}.rx 采样块同 op 序——
/// Rust f32 无收缩 + device NoContraction 注入 ⇒ 位级对拍面）。返回
/// (albedo rgb, 请求编码)——请求编码 = 0（无 miss）或 page_id+1。
#[allow(dead_code)] // G31+ 波 C Task C13:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
#[allow(clippy::too_many_arguments)]
fn g31_svt_host_sample(
    texmeta: &[f32],
    linlut: &[f32; 256],
    svtmeta: &[f32; 8],
    fallback: &[f32],
    pagetable: &[u32],
    pool: &[u32],
    slot: usize,
    uu0: f32,
    vv0: f32,
) -> ([f32; 3], f32) {
    let sb = 8 + slot * 8;
    let ox = texmeta[sb];
    let oy = texmeta[sb + 1];
    let tw = texmeta[sb + 2];
    let th2 = texmeta[sb + 3];
    let mod_r = texmeta[sb + 4];
    let mod_g = texmeta[sb + 5];
    let mod_b = texmeta[sb + 6];
    let uu = uu0 - uu0.floor();
    let vv = vv0 - vv0.floor();
    let xf = uu * tw - 0.5;
    let yf = vv * th2 - 0.5;
    let bxf = xf.floor();
    let byf = yf.floor();
    let fx = xf - bxf;
    let fy = yf - byf;
    let inv_tw = 1.0 / tw;
    let inv_th = 1.0 / th2;
    let x0 = bxf - (bxf * inv_tw).floor() * tw;
    let y0 = byf - (byf * inv_th).floor() * th2;
    // ── SVT-1 页表间接寻址（kernel 同 op 序）──
    let ptd = svtmeta[0] as usize;
    let tile_dim = svtmeta[1];
    let phys_dim = svtmeta[2] as usize;
    let phys_texels = svtmeta[3] as usize;
    let ax0 = ox + x0;
    let ay0 = oy + y0;
    let vpx = (ax0 / tile_dim).floor();
    let vpy = (ay0 / tile_dim).floor();
    let lx0 = ax0 - vpx * tile_dim;
    let ly0 = ay0 - vpy * tile_dim;
    let page_id = (vpy as usize) * ptd + (vpx as usize);
    let entry = pagetable[page_id] as usize;
    let (hit_f, phys_base) = if entry > 0 {
        (1.0f32, (entry - 1) * phys_texels)
    } else {
        (0.0f32, 0usize)
    };
    let req = (1.0 - hit_f) * ((page_id as f32) + 1.0);
    let lxi = lx0 as usize;
    let lyi = ly0 as usize;
    let p00 = pool[phys_base + (lyi + 1) * phys_dim + (lxi + 1)] as usize;
    let p10 = pool[phys_base + (lyi + 1) * phys_dim + (lxi + 2)] as usize;
    let p01 = pool[phys_base + (lyi + 2) * phys_dim + (lxi + 1)] as usize;
    let p11 = pool[phys_base + (lyi + 2) * phys_dim + (lxi + 2)] as usize;
    let p00_r = linlut[p00 % 256usize];
    let p00_g = linlut[(p00 / 256usize) % 256usize];
    let p00_b = linlut[(p00 / 65536usize) % 256usize];
    let p10_r = linlut[p10 % 256usize];
    let p10_g = linlut[(p10 / 256usize) % 256usize];
    let p10_b = linlut[(p10 / 65536usize) % 256usize];
    let p01_r = linlut[p01 % 256usize];
    let p01_g = linlut[(p01 / 256usize) % 256usize];
    let p01_b = linlut[(p01 / 65536usize) % 256usize];
    let p11_r = linlut[p11 % 256usize];
    let p11_g = linlut[(p11 / 256usize) % 256usize];
    let p11_b = linlut[(p11 / 65536usize) % 256usize];
    let fb = slot * 4;
    // day_0828 Phase B：G/B 底行 fy→fx 修正（kernels/g31_svt_{gi,probe}.rx
    // 同步——host/device 对拍面成对改）。
    let t0r = p00_r * (1.0 - fx) + p10_r * fx;
    let b0r = p01_r * (1.0 - fx) + p11_r * fx;
    let samp_r = hit_f * ((t0r * (1.0 - fy) + b0r * fy) * mod_r) + (1.0 - hit_f) * fallback[fb];
    let t0g = p00_g * (1.0 - fx) + p10_g * fx;
    let b0g = p01_g * (1.0 - fx) + p11_g * fx;
    let samp_g = hit_f * ((t0g * (1.0 - fy) + b0g * fy) * mod_g) + (1.0 - hit_f) * fallback[fb + 1];
    let t0b = p00_b * (1.0 - fx) + p10_b * fx;
    let b0b = p01_b * (1.0 - fx) + p11_b * fx;
    let samp_b = hit_f * ((t0b * (1.0 - fy) + b0b * fy) * mod_b) + (1.0 - hit_f) * fallback[fb + 2];
    ([samp_r, samp_g, samp_b], req)
}

/// C13 探针 UV 闭集律法（32/槽 = B4 24/槽基座〔16 网格 + 4 精确边缘 + 4
/// wrap 域〕+ 8 页界聚焦〔页界 texel 128m 双线性跨页 straddle + 左界 wrap
//  straddle;pow2 槽 ⇒ UV 商 f32 精确〕;与 ci/g31_svt_smoke.py 判读器同源
/// 镜像——篡改律法即红）。
#[allow(dead_code)] // G31+ 波 C Task C13:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_svt_probes(assets: &G31TexAssetsHeap) -> Vec<(u32, f32, f32)> {
    // day_0828 Phase B：B4 基座律法本地内联（g31_tex_probes 已扩 mip 维步幅
    // 4,SVT 旧形态 = mip0-only 步幅 3——SVT 臂已 fail-closed 互斥,本函数为
    // 编译面保留,律法字面不动）。
    let mut out = Vec::with_capacity(assets.slots.len() * (G31_TEX_PROBES_PER_SLOT + 8));
    for k in 0..assets.slots.len() {
        for j in 0..16u32 {
            let u = (((j * 37 + (k as u32) * 11) % 256) as f32 + 0.5) / 256.0;
            let v = (((j * 101 + (k as u32) * 13) % 256) as f32 + 0.5) / 256.0;
            out.push((k as u32, u, v));
        }
        let em1 = 1.0f32 - 2.0f32.powi(-23);
        out.push((k as u32, 0.0, 0.0));
        out.push((k as u32, 0.0, 0.5));
        out.push((k as u32, 0.5, 0.0));
        out.push((k as u32, em1, em1));
        out.push((k as u32, 1.25, 2.5));
        out.push((k as u32, 3.75, 1.5));
        out.push((k as u32, -0.25, 1.3333334));
        out.push((k as u32, 2.0, -0.75));
    }
    for (k, s) in assets.slots.iter().enumerate() {
        let (tw, th) = (s.width as f32, s.height as f32);
        // 页界 straddle:uu = 128m/w ⇒ xf = 128m−0.5,bxf = 128m−1,fx = 0.5
        // ⇒ 双线性 50/50 混合跨页 texel 对（127|128 界;wrap 域小槽同律经
        // fract 回绕落界——2048 % 128 == 0 ⇒ 槽内/图集 128 余数同余）。
        let pairs: [(f32, f32); 8] = [
            (128.0 / tw, 128.0 / th),
            (128.0 / tw, 0.5),
            (0.5, 128.0 / th),
            (256.0 / tw, 384.0 / th),
            (0.0, 128.0 / th), // 左界 wrap straddle（x0 = w−1 ↔ x1 = 0）
            (128.0 / tw, 0.0),
            (640.0 / tw, 896.0 / th),
            (1920.0 / tw, 1536.0 / th),
        ];
        for (u, v) in pairs {
            out.push((k as u32, u, v));
        }
    }
    out
}

/// C13 探针 device 腿（kernels/g31_svt_probe.rx SPV 经 vk::run_compute 单
/// dispatch [N,1,1];NoContraction 注入面 = host 侧 SPV 后处理,文件 0-byte）。
/// 返回 (out 3f32/probe, out_req 1f32/probe)。
#[allow(dead_code)] // G31+ 波 C Task C13:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_svt_probe_device(
    assets: &G31SvtAssets,
    tex: &G31TexAssetsHeap,
    probes: &[(u32, f32, f32)],
    pagetable_bytes: &[u8],
    pool_bytes: &[u8],
    spv_path: &str,
) -> Result<(Vec<f32>, Vec<f32>), String> {
    if !vk::vulkan_available() {
        return Err("vulkan loader 不可用".into());
    }
    let bytes = std::fs::read(spv_path).map_err(|e| format!("读 svt probe SPV {spv_path}: {e}"))?;
    if bytes.len() % 4 != 0 {
        return Err("svt probe SPIR-V 字节数非 4 对齐".into());
    }
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let words = spv_inject_no_contraction(&words);
    let entry = vk::entry_point_name(&words).ok_or("svt probe SPV 无 OpEntryPoint")?;
    let mut probe_f: Vec<f32> = Vec::with_capacity(probes.len() * 3);
    for &(slot, u, v) in probes {
        probe_f.push(slot as f32);
        probe_f.push(u);
        probe_f.push(v);
    }
    let mut params = vec![probes.len() as f32];
    params.resize(8, 0.0);
    let mut bufs = vec![
        bytes_f32(&probe_f),
        tex.texmeta_bytes.clone(),
        tex.linlut_bytes.clone(),
        assets.svtmeta_bytes.clone(),
        assets.fallback_bytes.clone(),
        pagetable_bytes.to_vec(),
        pool_bytes.to_vec(),
        bytes_f32(&params),
        vec![0u8; probes.len() * 12],
        vec![0u8; probes.len() * 4],
    ];
    vk::run_compute(&words, &entry, &mut bufs, &[], [probes.len() as u32, 1, 1])
        .map_err(|e| format!("svt probe device dispatch 失败: {e}"))?;
    Ok((read_f32(&bufs[8]), read_f32(&bufs[9])))
}

/// C13 SVT 探针双臂对拍报告（evidence 登记面;① = SVT-1/3 全驻留位级,
/// ② = SVT-2 部分驻留请求位级 + 闭环重跑）。
#[allow(dead_code)] // G31+ 波 C Task C13:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
struct G31SvtProbeReport {
    probe_count: usize,
    boundary_probe_count: usize,
    eval_ms: f64,
    // ① 全驻留臂（SVT-1 页表间接 + SVT-3 border 复制对拍）。
    full_p100_vs_direct: f64,
    full_bitexact_vs_direct: bool,
    full_bitexact_vs_svt_host: bool,
    full_device_digest: String,
    full_host_digest: String,
    full_double_run_bitexact: bool,
    boundary_max_abs: f64,
    // ② 部分驻留臂（SVT-2 miss 请求 + host 消费闭环重跑）。
    partial_miss_probes: u32,
    partial_req_bitexact: bool,
    partial_out_bitexact: bool,
    closed_loop_loaded: u32,
    closed_loop_evicted: u32,
    closed_loop_io_bytes: u64,
    closed_loop_all_hit: bool,
    closed_loop_bitexact_vs_full: bool,
}

/// C13 探针双臂对拍装配。① 全驻留:device SVT vs host SVT 参考 + vs host
/// 整图直采参考（g31_tex_host_sample 同 op 序）双位级硬门（页表间接寻址 +
/// border 复制 = 直采位级的结构性核验）;② 部分驻留（page_id%3==2 未驻留
/// 确定性律法,池容 = 活动页数零驱逐噪声）:out_req/输出与 host 同律位级
/// → host SvtStreaming::consume 消费 → 页表/池更新重跑 = 全 hit 且输出与
/// 全驻留臂位级一致（请求-驻留闭环 device 面真跑）。
#[allow(dead_code)] // G31+ 波 C Task C13:g31_window_present 独消费面(g14_3_pipeline_perf 未消费,诚实标注)
fn g31_svt_probe_evaluate(
    assets: &G31SvtAssets,
    tex: &G31TexAssetsHeap,
    probes: &[(u32, f32, f32)],
    spv_path: &str,
) -> Result<G31SvtProbeReport, String> {
    let t0 = std::time::Instant::now();
    let page_total = assets.tile_set.page_total();
    // ── ① 全驻留臂（页表恒等映射,池 = 全瓦片集——slot == page_id）──
    let full_stream = svt::SvtStreaming::new_full(assets.tile_set.clone())
        .map_err(|e| format!("SVT 全驻留初态: {e}"))?;
    let full_pt_bytes = full_stream.page_table_bytes();
    let pool_full_bytes = assets.tile_set.payloads_bytes();
    let pool_full_u32 = &assets.tile_set.payloads;
    let full_pt_u32 = full_stream.page_table();
    let (dev_a, req_a) =
        g31_svt_probe_device(assets, tex, probes, &full_pt_bytes, &pool_full_bytes, spv_path)?;
    let (dev_a2, _) =
        g31_svt_probe_device(assets, tex, probes, &full_pt_bytes, &pool_full_bytes, spv_path)?;
    if let Some(k) = dev_a.iter().position(|x| !x.is_finite()) {
        return Err(format!(
            "svt probe 全驻留臂判据⓪失败: 探针 {} device 输出非有限（有限性一等断言先于聚合）",
            k / 3
        ));
    }
    let full_double = dev_a == dev_a2 && req_a.iter().all(|&v| v == 0.0);
    let mut host_svt: Vec<f32> = Vec::with_capacity(probes.len() * 3);
    let mut host_direct: Vec<f32> = Vec::with_capacity(probes.len() * 3);
    for &(slot, u, v) in probes {
        let (s, rq) = g31_svt_host_sample(
            &tex.texmeta,
            &tex.linlut,
            &assets.svtmeta,
            &assets.fallback,
            full_pt_u32,
            pool_full_u32,
            slot as usize,
            u,
            v,
        );
        if rq != 0.0 {
            return Err("全驻留臂 host 参考出现 miss（页表恒等映射结构性破）".into());
        }
        host_svt.extend_from_slice(&s);
        // day_0828 Phase B：直采参考 = lod 0（SVT 旧形态 mip0-only;SVT 臂已
        // fail-closed 互斥,编译面保留——F6 双形态回正后走 heap/mip 形态）。
        let d = g31_tex_host_sample_mip(
            &tex.texmeta,
            &tex.atlas,
            &tex.linlut,
            slot as usize,
            u,
            v,
            0,
        );
        host_direct.extend_from_slice(&d);
    }
    let mut p100 = 0.0f64;
    let mut boundary_max = 0.0f64;
    let n_base = probes.len() - tex.slots.len() * 8;
    for k in 0..probes.len() * 3 {
        let d = (f64::from(dev_a[k]) - f64::from(host_direct[k])).abs();
        if d > p100 {
            p100 = d;
        }
        if k / 3 >= n_base && d > boundary_max {
            boundary_max = d;
        }
    }
    let dev_bytes: Vec<u8> = dev_a.iter().flat_map(|v| v.to_le_bytes()).collect();
    let host_bytes: Vec<u8> = host_direct.iter().flat_map(|v| v.to_le_bytes()).collect();
    let full_bitexact_vs_svt_host = dev_a == host_svt;
    let full_bitexact_vs_direct = dev_a == host_direct;
    // ── ② 部分驻留臂（紧凑页号 % 3 == 2 未驻留;恒等槽映射零驱逐噪声;
    //    页表项下标 = 页表网格序（vpy·1024+vpx,与 kernel 同律））──
    let pages_x = assets.tile_set.pages_x;
    let mut partial_pt = vec![0u32; svt::SVT_PAGE_COUNT];
    for p in 0..page_total {
        if p % 3 != 2 {
            let t = ((p / pages_x) * svt::SVT_PAGE_TABLE_DIM + (p % pages_x)) as usize;
            partial_pt[t] = p + 1;
        }
    }
    let partial_pt_bytes: Vec<u8> = partial_pt.iter().flat_map(|v| v.to_le_bytes()).collect();
    let (dev_b, req_b) =
        g31_svt_probe_device(assets, tex, probes, &partial_pt_bytes, &pool_full_bytes, spv_path)?;
    if let Some(k) = dev_b.iter().position(|x| !x.is_finite()) {
        return Err(format!(
            "svt probe 部分驻留臂判据⓪失败: 探针 {} device 输出非有限",
            k / 3
        ));
    }
    let mut host_b: Vec<f32> = Vec::with_capacity(probes.len() * 3);
    let mut host_req: Vec<f32> = Vec::with_capacity(probes.len());
    let mut miss_probes = 0u32;
    for &(slot, u, v) in probes {
        let (s, rq) = g31_svt_host_sample(
            &tex.texmeta,
            &tex.linlut,
            &assets.svtmeta,
            &assets.fallback,
            &partial_pt,
            pool_full_u32,
            slot as usize,
            u,
            v,
        );
        host_b.extend_from_slice(&s);
        host_req.push(rq);
        if rq != 0.0 {
            miss_probes += 1;
        }
    }
    let partial_req_bitexact = req_b == host_req;
    let partial_out_bitexact = dev_b == host_b;
    // ── ②b 闭环:host 消费 device 请求 → 页表/池更新 → 重跑全 hit ──
    let mut stream2 = svt::SvtStreaming::from_page_table(assets.tile_set.clone(), partial_pt.clone())
        .map_err(|e| format!("SVT 部分驻留初态: {e}"))?;
    let plan = stream2
        .consume(&req_b)
        .map_err(|e| format!("SVT 闭环 consume: {e}"))?;
    let pt2_u32 = stream2.page_table().to_vec();
    let pt2_bytes = stream2.page_table_bytes();
    let mut pool2_u32 = assets.tile_set.payloads.clone();
    for &(slot, page_id) in &plan.tile_uploads {
        let payload = assets
            .tile_set
            .page_payload(page_id)
            .map_err(|e| e.to_string())?;
        let b = slot as usize * svt::SVT_PHYS_TEXELS;
        pool2_u32[b..b + svt::SVT_PHYS_TEXELS].copy_from_slice(payload);
    }
    let pool2_bytes: Vec<u8> = pool2_u32.iter().flat_map(|v| v.to_le_bytes()).collect();
    let (dev_c, req_c) =
        g31_svt_probe_device(assets, tex, probes, &pt2_bytes, &pool2_bytes, spv_path)?;
    if let Some(k) = dev_c.iter().position(|x| !x.is_finite()) {
        return Err(format!(
            "svt probe 闭环重跑判据⓪失败: 探针 {} device 输出非有限",
            k / 3
        ));
    }
    let closed_loop_all_hit = req_c.iter().all(|&v| v == 0.0);
    let closed_loop_bitexact_vs_full = dev_c == dev_a;
    let _ = pt2_u32;
    let eval_ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok(G31SvtProbeReport {
        probe_count: probes.len(),
        boundary_probe_count: tex.slots.len() * 8,
        eval_ms,
        full_p100_vs_direct: p100,
        full_bitexact_vs_direct,
        full_bitexact_vs_svt_host,
        full_device_digest: format!("sha256:{}", sha256_hex(&dev_bytes)),
        full_host_digest: format!("sha256:{}", sha256_hex(&host_bytes)),
        full_double_run_bitexact: full_double,
        boundary_max_abs: boundary_max,
        partial_miss_probes: miss_probes,
        partial_req_bitexact,
        partial_out_bitexact,
        closed_loop_loaded: plan.loaded,
        closed_loop_evicted: plan.evicted,
        closed_loop_io_bytes: plan.io_bytes,
        closed_loop_all_hit,
        closed_loop_bitexact_vs_full,
    })
}

/// 逐帧参数（48 f32；与 kernel 参数面逐字同源——行主序矩阵按 m[r][c] →
/// [9+r*4+c] / [25+r*4+c] 摊平）。D2：签名 0-byte 保持（g31_window_present
/// 两调用面不触）——平滑法线开关面委托 [`pack_frame_params_nrm`]。
#[allow(clippy::too_many_arguments)]
fn pack_frame_params(
    iw: u32,
    ih: u32,
    jitter: [f32; 2],
    eps: f32,
    quad_count: usize,
    point_count: usize,
    inv_vp: &Mat4,
    vp: &Mat4,
) -> Vec<f32> {
    pack_frame_params_nrm(iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp, false)
}

/// 逐帧参数 D2 扩面（`smooth_nrm` = --smooth-normals on 臂开关：params[43]
/// 母版恒 0 保留位；on 置 1.0 → g18_smooth_nrm kernel gate_sn=1 走重心插值
/// 面；false 时产物与既有 pack_frame_params 逐位同值，0-byte）。
/// D6：签名 0-byte 保持（g31_window_present 两调用面不触）——GGX 开关面
/// 委托 [`pack_frame_params_ggx`] 的 `ggx = false` 形态。
#[allow(clippy::too_many_arguments)]
fn pack_frame_params_nrm(
    iw: u32,
    ih: u32,
    jitter: [f32; 2],
    eps: f32,
    quad_count: usize,
    point_count: usize,
    inv_vp: &Mat4,
    vp: &Mat4,
    smooth_nrm: bool,
) -> Vec<f32> {
    pack_frame_params_ggx(
        iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp, smooth_nrm, false,
    )
}

/// 逐帧参数 D6 扩面（`ggx` = --ggx on 臂开关：params[48] 恒 0 保留位；on 且
/// 仅当 smooth_nrm 同 on 时置 1.0 → g18_smooth_nrm kernel GGX 高光臂走
/// tri_mr 侧表面；ggx=false 时产物与 D6 前 pack_frame_params_nrm 逐位同值
/// 〔params[48..56) 恒 0 追加〕，0-byte）。
/// A1：签名 0-byte 保持——灯贡献剔除阈值面委托 [`pack_frame_params_lamp`]
/// 的 `lamp_contrib = 0.0` 形态（params[49] 写 0.0 与 resize 零填充逐位同值）。
#[allow(clippy::too_many_arguments)]
fn pack_frame_params_ggx(
    iw: u32,
    ih: u32,
    jitter: [f32; 2],
    eps: f32,
    quad_count: usize,
    point_count: usize,
    inv_vp: &Mat4,
    vp: &Mat4,
    smooth_nrm: bool,
    ggx: bool,
) -> Vec<f32> {
    pack_frame_params_lamp(
        iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp, smooth_nrm, ggx, 0.0,
    )
}

/// 逐帧参数 A1 扩面（`lamp_contrib` = --lamp-contrib 贡献剔除阈值：
/// params[49] 恒 0 保留位；仅 smooth_nrm=true 车道写入——基线车道永不触达，
/// Stage A 锚零风险。0.0（默认）与 resize 零填充逐位同值 ⇒ 既有全臂 0-byte；
/// >0 时 g18_smooth_nrm kernel 点光循环按 contrib = max3(I)/d² >= 阈值门
/// 整灯剔除〔--lamp-lights on 控帧钮〕）。
/// day_0828 Phase B：签名 0-byte 保持（bench/window 两调用面不触）——
/// 纹理 mip 锥角面委托 [`pack_frame_params_tex`] 的 `tex_kpix = 0.0` 形态
/// （params[50] 写 0.0 与 resize 零填充逐位同值）。
#[allow(clippy::too_many_arguments)]
fn pack_frame_params_lamp(
    iw: u32,
    ih: u32,
    jitter: [f32; 2],
    eps: f32,
    quad_count: usize,
    point_count: usize,
    inv_vp: &Mat4,
    vp: &Mat4,
    smooth_nrm: bool,
    ggx: bool,
    lamp_contrib: f32,
) -> Vec<f32> {
    pack_frame_params_tex(
        iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp, smooth_nrm, ggx, lamp_contrib,
        0.0,
    )
}

/// 逐帧参数 day_0828 Phase B 扩面（`tex_kpix` = mip 选择像素锥角
/// 2·tan(fovy/2)/height_internal：params[50] 恒 0 保留位；仅 --textures on
/// 车道经 set_tex_kpix 挂载 >0——默认 0.0 与 resize 零填充逐位同值 ⇒ 既有
/// 全臂 0-byte。kernel 消费面 = g31_texture_gi.rx / g31_texture_nrm_gi.rx
/// lod = clamp(floor(log2(th·k_pix·k_tri·w_base)), 0, mips−1)）。
/// day_0828 Phase C：签名 0-byte 保持——GI2 面委托 [`pack_frame_params_gi2`]
/// 的 `gi2 = false` 形态（params[51..55) 不写与 resize 零填充逐位同值）。
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)] // day_0828 Phase C:链环节保留(两 bin 已直迁 gi2 形态,诚实标注)
fn pack_frame_params_tex(
    iw: u32,
    ih: u32,
    jitter: [f32; 2],
    eps: f32,
    quad_count: usize,
    point_count: usize,
    inv_vp: &Mat4,
    vp: &Mat4,
    smooth_nrm: bool,
    ggx: bool,
    lamp_contrib: f32,
    tex_kpix: f32,
) -> Vec<f32> {
    pack_frame_params_gi2(
        iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp, smooth_nrm, ggx, lamp_contrib,
        tex_kpix, false, 0.0, 0.0, 0.0,
    )
}

/// 画质战役 Phase E1 `--quality full` 预设环境光注入槽（进程内 OnceLock：
/// 两 bin `#![forbid(unsafe_code)]` + edition 2024 下 `env::set_var` 为
/// unsafe，预设不可写 env ⇒ 走本槽）。语义 = RURIX_G18_AMBIENT env **缺席**
/// 时的回退值；env 在位一律优先（含非法字面——保持既有「静默关臂」语义不被
/// 预设越位）。默认不 set = None 回退分支零行为 ⇒ 既有全臂路径 0-byte 语义。
#[allow(dead_code)] // 消费面 = pack_frame_params_gi2 + 两 bin --quality full 展开块
static G18_AMBIENT_PRESET: std::sync::OnceLock<f32> = std::sync::OnceLock::new();

/// 逐帧参数 day_0828 Phase C 扩面（GI2 1 反弹间接光加性臂四槽：params[51]
/// 门/[52] frame_idx〔R2 时域旋转——逐帧挂载，TSR 收敛面〕/[53] firefly
/// clamp/[54] gi_scale——恒 0 保留位；仅 --gi2 on 车道写入，gi2=false 时四槽
/// 不写与 resize 零填充逐位同值 ⇒ 既有全臂 0-byte。kernel 消费面 =
/// g31_texture_nrm_gi.rx GI2 段独有——其余 kernel 不读 [51..55)，门=0 时
/// while 零迭代 +0.0 恒等尾加）。
#[allow(clippy::too_many_arguments)]
fn pack_frame_params_gi2(
    iw: u32,
    ih: u32,
    jitter: [f32; 2],
    eps: f32,
    quad_count: usize,
    point_count: usize,
    inv_vp: &Mat4,
    vp: &Mat4,
    smooth_nrm: bool,
    ggx: bool,
    lamp_contrib: f32,
    tex_kpix: f32,
    gi2: bool,
    gi2_frame: f32,
    gi2_clamp: f32,
    gi2_scale: f32,
) -> Vec<f32> {
    let mut v = vec![
        (iw * ih) as f32,
        iw as f32,
        ih as f32,
        jitter[0],
        jitter[1],
        eps,
        RAY_TMAX,
        quad_count as f32,
        point_count as f32,
    ];
    for r in 0..4 {
        for c in 0..4 {
            v.push(inv_vp.m[r][c]);
        }
    }
    for r in 0..4 {
        for c in 0..4 {
            v.push(vp.m[r][c]);
        }
    }
    v.push(INV_PI);
    v.resize(PARAMS_LEN, 0.0);
    if let Ok(s) = std::env::var("RURIX_G18_SKY_INTENSITY") {
        if let Ok(f) = s.parse::<f32>() {
            if f.is_finite() && f >= 0.0 {
                v[42] = f;
            }
        }
    }
    // D2 平滑顶点法线臂开关（params[43]；off = 本行不执行，params[0..48)
    // 与既有面逐位同值）。
    if smooth_nrm {
        v[43] = 1.0;
        // 夜间巡航 D5 半球环境光加性臂（params[44..48)；仅质量车道 + 显式
        // env 启用——基线车道 smooth_nrm=false 永不触达本块，Stage A 锚零
        // 风险）。RURIX_G18_AMBIENT=<intensity>（scene-linear 曝光域辐照
        // 强度）；颜色 = 暖白（1.0,0.85,0.7）·intensity 进 params[45..48)。
        // 不设/非法 = params[44]=0 ⇒ kernel 关臂位级。Phase E1：env **缺席**
        // 时回退 G18_AMBIENT_PRESET（--quality full 预设注入槽;env 在位
        // 一律优先,非法字面保持既有静默关臂语义;槽未 set = 分支零行为）。
        let ambient: Option<f32> = match std::env::var("RURIX_G18_AMBIENT") {
            Ok(s) => s.parse::<f32>().ok().filter(|f| f.is_finite() && *f >= 0.0),
            Err(_) => G18_AMBIENT_PRESET.get().copied(),
        };
        if let Some(f) = ambient {
            v[44] = f;
            v[45] = 1.0;
            v[46] = 0.85;
            v[47] = 0.7;
        }
        // D6 GGX 高光臂开关（params[48]；仅 smooth_nrm=true 车道 + ggx=true
        // 才写——基线车道 smooth_nrm=false 永不触达本块，Stage A 锚零风险；
        // CLI --ggx on 已 fail-closed 裁「须随 --smooth-normals on」，本行
        // 为第二重保险）。off = 本行不执行，params[48]=0 ⇒ kernel 关臂位级。
        if ggx {
            v[48] = 1.0;
        }
        // A1 灯贡献剔除阈值（params[49]；仅 smooth_nrm=true 车道写——默认
        // 0.0 与零填充逐位同值，>0 仅 --lamp-lights on 控帧钮显式给出）。
        v[49] = lamp_contrib;
    }
    // day_0828 Phase B 纹理 mip 锥角（params[50]；无条件写——默认 0.0 与
    // 零填充逐位同值 ⇒ 非纹理臂 0-byte；--textures on 车道 host 挂载 >0）。
    v[50] = tex_kpix;
    // day_0828 Phase C GI2 四槽（params[51..55)；仅 --gi2 on 车道写——默认
    // 不执行与零填充逐位同值 ⇒ 既有全臂 0-byte）。
    if gi2 {
        v[51] = 1.0;
        v[52] = gi2_frame;
        v[53] = gi2_clamp;
        v[54] = gi2_scale;
    }
    v
}

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

/// 自 SPV 字流解析 compute 入口 `LocalSize`（`OpExecutionMode %entry LocalSize
/// x y z`；opcode 16 / mode 17；无则 (1,1,1)——既有全部 kernel 既有默认面）。
/// dispatch 组数 = ceil(w/x)·ceil(h/y)：kernel 局部组形态与 host dispatch
/// 单一事实源（SPV），禁两处各写漂移（G14.3 `wg` 标注系消费面）。
fn spv_local_size(spv: &[u32]) -> (u32, u32, u32) {
    let mut i = 5; // SPIR-V header 5 字
    while i < spv.len() {
        let w = spv[i];
        let wc = (w >> 16) as usize;
        let op = w & 0xFFFF;
        if wc == 0 || i + wc > spv.len() {
            break;
        }
        if op == 16 && wc == 6 && spv[i + 2] == 17 {
            return (spv[i + 3], spv[i + 4], spv[i + 5]);
        }
        i += wc;
    }
    (1, 1, 1)
}

// ---------------------------------------------------------------------------
// UpscaleBackend 接入面（冻结 trait 0-byte；三后端 bin-local adapter——
// tsr_device = M-b TsrDeviceBackend 同模式；dlss_sr/fsr_3_1_5 = M-a adapter 同模式）
// G14.3：g13_4 同型复制子集（bin-local 惯例）
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// G14plus 统一四 pass 车道（RFC-0030 §4.5 L2 + §4.3 L3 已批准终态：tsr_device
// 臂单一 DeviceFrameSession，pass0=scene(g14_3_direct_gi) → pass1=mv(g14_mv)
// → pass2=tsr resample(g14_8_tsr_resample) → pass3=tsr resolve
// (g14_8_tsr_resolve)，GPU 链内零 host 往返）。
//
// 原两 session 架构（场景 session 逐帧回读 color/depth → host compute_camera_mv
// → TSR session 逐帧上传 color/depth/mv + 逐帧回读 out_color 24.9MB@1080p）的
// host 中转税（bistro t50 过渡态实测 prod=156ms 其中 ~110ms 为上传/回读税）
// 消除：resample 直读 scene_color/scene_depth、resolve 直读 mv_out（TSR 的
// in_color/in_depth/in_mv 不再是独立资源）；bench 测量循环 readback_subset=
// Some([]) 零回读、仅末帧回读 TSR 输出算 digest（同一 GPU 状态机，历史链
// 演化与回读无关——末帧 digest 与逐帧回读版位级同语义）；render 腿逐帧回读
// 出 EXR（出图面凌驾性能）。dlss_sr 臂 G14.10e 已迁驻留统一车道（见
// DlssResidentLane 区注释）；fsr_3_1_5 臂维持场景 session + host mv + vendor
// host pack 现状结构（FSR resident 归 external memory 波另判）。
// ---------------------------------------------------------------------------

/// 统一 session 资源下标闭集（场景区 0..=6 与场景车道逐字同布局；mv 区
/// 7..=8；TSR 区 9..=21——历史五元组 A/B parity 双缓冲沿原 TsrDeviceBackend
/// 同律轮换）。
const U_TRIS: u32 = 0;
const U_MATS: u32 = 1;
const U_QUADS: u32 = 2;
const U_POINTS: u32 = 3;
const U_SCENE_PARAMS: u32 = 4;
const U_SCENE_COLOR: u32 = 5;
const U_SCENE_DEPTH: u32 = 6;
const U_MV_PARAMS: u32 = 7;
const U_MV_OUT: u32 = 8;
const U_TSR_PARAMS: u32 = 9;
const U_REACTIVE: u32 = 10;
const U_CUR_RGB: u32 = 11;
const U_LUMA: [u32; 2] = [12, 13];
const U_DEPTH_HI: [u32; 2] = [14, 15];
const U_OUT_COLOR: [u32; 2] = [16, 17];
const U_OUT_SIGN: [u32; 2] = [18, 19];
const U_OUT_SCORE: [u32; 2] = [20, 21];
const U_RESOURCE_COUNT: usize = 22;
/// G14.10b cornell 拆散车道追加资源（Split 形态专属；RFC-0030 授权 G14.4 取证
/// f 条兑现——16 样本拆散重排：primary 写 hitinfo → scatter 16 层每 invocation
/// 1 条 first-hit 阴影 ray → reduce 固定 0..15 序重算几何项累加与原串行循环
/// 位级同序）。
const U_HIT_T: u32 = 22;
const U_HIT_PRIM: u32 = 23;
const U_BLK: u32 = 24;
const U_RESOURCE_COUNT_SPLIT: usize = 25;

/// 统一车道逐 pass 屏障计划（保守 StorageReadWrite 超集逐字声明，与执行器
/// 隐式补全超集逐位一致——场景/TSR 车道同一见证体例；'static 常量面）。
/// scene 触达集 = 场景区 7 路；mv 触达集 = {scene_depth,mv_params,mv_out}；
/// resample 触达集 = {scene_color,scene_depth,tsr_params,cur_rgb,
/// luma[A/B],depth_hi[A/B]}；resolve 触达集 = 其余 14 路（跨双 parity 并集）。
const U_PLAN_SCENE: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
const U_PLAN_MV: &[(u32, TargetState)] = &[
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (U_MV_PARAMS, TargetState::StorageReadWrite),
    (U_MV_OUT, TargetState::StorageReadWrite),
];
/// Split 形态三 pass 屏障计划（cornell 拆散车道；保守超集同律）。
const U_PLAN_PRIMARY: &[(u32, TargetState)] = &[
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (U_HIT_T, TargetState::StorageReadWrite),
    (U_HIT_PRIM, TargetState::StorageReadWrite),
];
const U_PLAN_SCATTER: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (U_HIT_T, TargetState::StorageReadWrite),
    (U_HIT_PRIM, TargetState::StorageReadWrite),
    (U_BLK, TargetState::StorageReadWrite),
];
const U_PLAN_REDUCE: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (U_HIT_T, TargetState::StorageReadWrite),
    (U_HIT_PRIM, TargetState::StorageReadWrite),
    (U_BLK, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
const U_PLAN_RESAMPLE: &[(u32, TargetState)] = &[
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (U_TSR_PARAMS, TargetState::StorageReadWrite),
    (U_CUR_RGB, TargetState::StorageReadWrite),
    (U_LUMA[0], TargetState::StorageReadWrite),
    (U_LUMA[1], TargetState::StorageReadWrite),
    (U_DEPTH_HI[0], TargetState::StorageReadWrite),
    (U_DEPTH_HI[1], TargetState::StorageReadWrite),
];
const U_PLAN_RESOLVE: &[(u32, TargetState)] = &[
    (U_CUR_RGB, TargetState::StorageReadWrite),
    (U_LUMA[0], TargetState::StorageReadWrite),
    (U_LUMA[1], TargetState::StorageReadWrite),
    (U_DEPTH_HI[0], TargetState::StorageReadWrite),
    (U_DEPTH_HI[1], TargetState::StorageReadWrite),
    (U_MV_OUT, TargetState::StorageReadWrite),
    (U_REACTIVE, TargetState::StorageReadWrite),
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (U_OUT_SIGN[0], TargetState::StorageReadWrite),
    (U_OUT_SIGN[1], TargetState::StorageReadWrite),
    (U_OUT_SCORE[0], TargetState::StorageReadWrite),
    (U_OUT_SCORE[1], TargetState::StorageReadWrite),
    (U_TSR_PARAMS, TargetState::StorageReadWrite),
];

/// mv kernel 参数面打包（与 kernels/g14_mv.rx 文件头参数面逐字同源；40 f32：
/// [0]=w [1]=h [2..18]=inv_cur 行主序 [18..34]=prev_vp [34]=has_prev
/// [35..40]=reserved）。inv_cur 由 host 预算（`Mat4::inverse` 伴随法——
/// `compute_camera_mv` 内部同一实现同一输入 vp_j，位级同源；GPU 侧零求逆）；
/// has_prev=0 时 kernel 门直写 (0,0)，与 host 首帧 `ImageF32::new` 零图同
/// 语义（prev 槽占位值不被消费）。

fn pack_mv_params(iw: u32, ih: u32, inv_cur: &Mat4, prev_vp: &Mat4, has_prev: bool) -> Vec<f32> {
    let mut v = vec![iw as f32, ih as f32];
    for r in 0..4 {
        for c in 0..4 {
            v.push(inv_cur.m[r][c]);
        }
    }
    for r in 0..4 {
        for c in 0..4 {
            v.push(prev_vp.m[r][c]);
        }
    }
    v.push(if has_prev { 1.0 } else { 0.0 });
    v.resize(40, 0.0);
    v
}

/// SPIR-V NoContraction 后处理（**mv kernel 专用**；RFC-0030 §4.1 L1「同式
/// 同序」的位级兑现面）：对全部 OpFAdd/OpFSub/OpFMul 结果 id 注入
/// `OpDecorate %id NoContraction`，禁驱动 mul+add FMA 收缩——GPU 浮点序列与
/// host `compute_camera_mv` 严格 IEEE 逐 op 对齐（G9 skin_kernel 手写 SPIR-V
/// 发射器 NoContraction 登记同律；rurixc vulkan_codegen 现不发射该装饰，bin
/// 侧后处理，SPV 文件 0-byte 不动）。归因见证：mv-probe 实测无装饰时 GPU/host
/// max-abs ~1e-3~1e-2@cornell（miss 像素 depth 发散值 + 反投影链病态条件数把
/// ULP 级收缩差放大；depth+1ULP 敏感度实验与观测差同量级）。scene/TSR kernel
/// **不做**此变换——其 digest 锚在无装饰面建立，触碰即破坏既有锚。
fn spv_inject_no_contraction(spv: &[u32]) -> Vec<u32> {
    let mut result_ids: Vec<u32> = Vec::new();
    let mut i = 5usize; // SPIR-V header 5 字
    let mut first_decorate: Option<usize> = None;
    let mut first_type: Option<usize> = None;
    while i < spv.len() {
        let w = spv[i];
        let wc = (w >> 16) as usize;
        let op = w & 0xFFFF;
        if wc == 0 || i + wc > spv.len() {
            fail("SPIR-V 指令流越界（NoContraction 注入）");
        }
        match op {
            // OpDecorate（annotation 段前沿 = 注入锚）。
            71 if first_decorate.is_none() => first_decorate = Some(i),
            // OpType*（备用锚：无 annotation 段时插在 type 段前）。
            19..=39 if first_type.is_none() => first_type = Some(i),
            // OpFAdd(129)/OpFSub(131)/OpFMul(133) 结果 id。
            129 | 131 | 133 => result_ids.push(spv[i + 2]),
            _ => {}
        }
        i += wc;
    }
    let at = first_decorate
        .or(first_type)
        .unwrap_or_else(|| fail("SPIR-V 无 annotation/type 段锚（NoContraction 注入）"));
    let mut out = Vec::with_capacity(spv.len() + result_ids.len() * 3);
    out.extend_from_slice(&spv[..at]);
    for id in &result_ids {
        out.push(71u32 | (3 << 16)); // OpDecorate（wc=3）
        out.push(*id);
        out.push(42); // Decoration NoContraction
    }
    out.extend_from_slice(&spv[at..]);
    out
}

/// 统一车道 SPV/常量字节所有者（desc 数组经 [`unified_lane_descs`] 借用构建；
/// 借用纪律：assets/bits → descs → session 声明序 = drop 逆序，场景车道同模）。
struct UnifiedLaneBits {
    spv_scene: Vec<u8>,
    spv_mv: Vec<u8>,
    spv_resample: Vec<u8>,
    spv_resolve: Vec<u8>,
    /// Split 形态三 kernel（cornell 拆散车道；Mega 形态恒空零消费）。
    spv_primary: Vec<u8>,
    spv_scatter: Vec<u8>,
    spv_reduce: Vec<u8>,
    reactive_zeros: Vec<u8>,
    /// dispatch 组数 = ceil(内部分辨率/SPV LocalSize)——SPV 单一事实源纪律
    /// （G14.10c 起 TSR 双 pass 同律：dispatch 形态随 SPV LocalSize 派生，
    /// 变体 kernel 换线程组形态零 bin 改动；逐像素独立+越界门 → 覆盖域不变）。
    scene_dispatch: [u32; 3],
    mv_dispatch: [u32; 3],
    resample_dispatch: [u32; 3],
    resolve_dispatch: [u32; 3],
    primary_dispatch: [u32; 3],
    scatter_dispatch: [u32; 3],
    reduce_dispatch: [u32; 3],
}

/// Split 形态默认 SPV 路径（cornell 拆散三 kernel；--spv-* CLI 覆盖面暂不开，
/// M-c 门 _ensure_spv 同步编译）。
const DEFAULT_SPV_PRIMARY: &str = ".tmp/g14_gates/m_c/g14_3_primary.spv";
const DEFAULT_SPV_SCATTER: &str = ".tmp/g14_gates/m_c/g14_3_shadow_scatter.spv";
const DEFAULT_SPV_REDUCE: &str = ".tmp/g14_gates/m_c/g14_3_shade_reduce.spv";

impl UnifiedLaneBits {
    #[allow(clippy::too_many_arguments)]
    fn load(
        spv_scene: &str,
        spv_mv: &str,
        spv_resample: &str,
        spv_resolve: &str,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        split: bool,
    ) -> Self {
        let to_bytes = |words: &[u32]| -> Vec<u8> {
            words.iter().flat_map(|w| w.to_le_bytes()).collect()
        };
        let scene_words = load_spv(spv_scene);
        // mv kernel 注入 NoContraction（见 spv_inject_no_contraction 文档——
        // GPU mv 与 host compute_camera_mv 位级对齐面；scene/TSR SPV 不触碰）。
        let mv_words = spv_inject_no_contraction(&load_spv(spv_mv));
        let resample_words = load_spv(spv_resample);
        let resolve_words = load_spv(spv_resolve);
        let (sx, sy, _) = spv_local_size(&scene_words);
        let (mx, my, _) = spv_local_size(&mv_words);
        let (rsx, rsy, _) = spv_local_size(&resample_words);
        let (rvx, rvy, _) = spv_local_size(&resolve_words);
        // Split 形态三 kernel（scatter dispatch 的 y 维打包 16 采样层）。
        let (spv_primary, spv_scatter, spv_reduce, primary_dispatch, scatter_dispatch, reduce_dispatch) =
            if split {
                let pw = load_spv(DEFAULT_SPV_PRIMARY);
                let sw = load_spv(DEFAULT_SPV_SCATTER);
                let rw = load_spv(DEFAULT_SPV_REDUCE);
                let (px, py, _) = spv_local_size(&pw);
                let (scx, scy, _) = spv_local_size(&sw);
                let (rdx, rdy, _) = spv_local_size(&rw);
                (
                    to_bytes(&pw),
                    to_bytes(&sw),
                    to_bytes(&rw),
                    [iw.div_ceil(px), ih.div_ceil(py), 1],
                    [iw.div_ceil(scx), (ih * 16).div_ceil(scy), 1],
                    [iw.div_ceil(rdx), ih.div_ceil(rdy), 1],
                )
            } else {
                (Vec::new(), Vec::new(), Vec::new(), [0; 3], [0; 3], [0; 3])
            };
        Self {
            spv_scene: to_bytes(&scene_words),
            spv_mv: to_bytes(&mv_words),
            spv_resample: to_bytes(&resample_words),
            spv_resolve: to_bytes(&resolve_words),
            spv_primary,
            spv_scatter,
            spv_reduce,
            reactive_zeros: vec![0u8; (iw * ih * 4) as usize],
            scene_dispatch: [iw.div_ceil(sx), ih.div_ceil(sy), 1],
            mv_dispatch: [iw.div_ceil(mx), ih.div_ceil(my), 1],
            resample_dispatch: [ow.div_ceil(rsx), oh.div_ceil(rsy), 1],
            resolve_dispatch: [ow.div_ceil(rvx), oh.div_ceil(rvy), 1],
            primary_dispatch,
            scatter_dispatch,
            reduce_dispatch,
        }
    }
}

/// 统一车道描述组（22 SSBO + 四 pass 固定图 + 逐 pass 屏障 + 3 readback：
/// out_color A/B 双 parity + mv_out 诊断探针——readback 表项 subset 不消费
/// 零成本，探针供 digest 漂移归因臂）。初始绑定 = parity 0，逐帧经
/// binding_overrides 换 resample/resolve 双 pass parity（scene/mv 绑定恒定）。
#[allow(clippy::type_complexity)]
fn unified_lane_descs<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> (
    [ResourceDesc<'x>; U_RESOURCE_COUNT],
    [Pass<'x>; 4],
    [&'static [(u32, TargetState)]; 4],
    [Readback; 4],
) {
    let ipc = (iw * ih) as u64;
    let opc = (ow * oh) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    // G14.10d 判定规则：凡 FrameUpdate.buffer_uploads 目标（params 三小件）
    // = host-visible（device_local:false）；其余（创建期一次上传 + GPU 链内
    // 中间缓冲 + 回读输出）= DEVICE_LOCAL 驻留。
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: true,
        })
    };
    let host_init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: false,
        })
    };
    let host_buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: false,
        })
    };
    let resources = [
        init(&assets.tris_bytes),         // U_TRIS
        init(&assets.mats_bytes),         // U_MATS
        init(&assets.quads_bytes),        // U_QUADS
        init(&assets.points_bytes),       // U_POINTS
        host_init(&assets.params0_bytes), // U_SCENE_PARAMS（逐帧 192B 覆盖）
        buf(assets.out_color_size),       // U_SCENE_COLOR（GPU 链内直读，零回读）
        buf(assets.out_depth_size),       // U_SCENE_DEPTH（同上）
        host_buf(40 * 4),                 // U_MV_PARAMS（逐帧 160B 覆盖）
        buf(ipc * 8),                     // U_MV_OUT（2 f32/px；GPU 链内直读）
        host_buf(32 * 4),                 // U_TSR_PARAMS（逐帧 128B 覆盖）
        init(&bits.reactive_zeros),       // U_REACTIVE（has_reactive=0 面恒零，创建期一次）
        buf(opc * 12),                    // U_CUR_RGB
        buf(opc * 4),                     // U_LUMA[0]
        buf(opc * 4),                     // U_LUMA[1]
        buf(opc * 4),                     // U_DEPTH_HI[0]
        buf(opc * 4),                     // U_DEPTH_HI[1]
        buf(opc * 12),                    // U_OUT_COLOR[0]
        buf(opc * 12),                    // U_OUT_COLOR[1]
        buf(opc * 4),                     // U_OUT_SIGN[0]
        buf(opc * 4),                     // U_OUT_SIGN[1]
        buf(opc * 4),                     // U_OUT_SCORE[0]
        buf(opc * 4),                     // U_OUT_SCORE[1]
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "g14_3_direct_gi",
            spirv: &bits.spv_scene,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.scene_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    U_TRIS,
                    U_MATS,
                    U_QUADS,
                    U_POINTS,
                    U_SCENE_PARAMS,
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_mv",
            spirv: &bits.spv_mv,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.mv_dispatch),
            bindings: Bindings {
                storage_buffers: vec![U_SCENE_DEPTH, U_MV_PARAMS, U_MV_OUT],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_8_tsr_resample",
            spirv: &bits.spv_resample,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.resample_dispatch),
            bindings: Bindings {
                storage_buffers: vec![
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                    U_TSR_PARAMS,
                    U_CUR_RGB,
                    U_LUMA[0],
                    U_DEPTH_HI[0],
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_8_tsr_resolve",
            spirv: &bits.spv_resolve,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.resolve_dispatch),
            bindings: Bindings {
                storage_buffers: vec![
                    U_CUR_RGB,
                    U_LUMA[0],
                    U_DEPTH_HI[0],
                    U_MV_OUT,
                    U_REACTIVE,
                    U_OUT_COLOR[1],
                    U_DEPTH_HI[1],
                    U_LUMA[1],
                    U_OUT_SIGN[1],
                    U_OUT_SCORE[1],
                    U_TSR_PARAMS,
                    U_OUT_COLOR[0],
                    U_OUT_SIGN[0],
                    U_OUT_SCORE[0],
                ],
                ..Bindings::default()
            },
        }),
    ];
    let barriers = [U_PLAN_SCENE, U_PLAN_MV, U_PLAN_RESAMPLE, U_PLAN_RESOLVE];
    let readbacks = [
        Readback::Buffer {
            res: U_OUT_COLOR[0],
            offset: 0,
            size: opc * 12,
        },
        Readback::Buffer {
            res: U_OUT_COLOR[1],
            offset: 0,
            size: opc * 12,
        },
        Readback::Buffer {
            res: U_MV_OUT,
            offset: 0,
            size: ipc * 8,
        },
        Readback::Buffer {
            res: U_SCENE_DEPTH,
            offset: 0,
            size: ipc * 4,
        },
    ];
    (resources, passes, barriers, readbacks)
}

/// G31+ 波 A Task A4 动态场景描述组（Mega 同图 + readback 第 5 项 U_SCENE_COLOR
/// 位置核验回读；scene SPV = g31_dyn_scene 实例感知变体经 bits 传入）。资源/
/// pass/屏障与 [`unified_lane_descs`] 逐字同构——仅 readback 表扩 1 项。
#[allow(clippy::type_complexity)]
fn unified_lane_descs_dyn<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> (
    [ResourceDesc<'x>; U_RESOURCE_COUNT],
    [Pass<'x>; 4],
    [&'static [(u32, TargetState)]; 4],
    [Readback; 5],
) {
    let (resources, passes, barriers, readbacks4) =
        unified_lane_descs(assets, bits, iw, ih, ow, oh);
    let ipc = (iw * ih) as u64;
    let readbacks = [
        readbacks4[0],
        readbacks4[1],
        readbacks4[2],
        readbacks4[3],
        Readback::Buffer {
            res: U_SCENE_COLOR,
            offset: 0,
            size: ipc * 12,
        },
    ];
    (resources, passes, barriers, readbacks)
}

// ---------------------------------------------------------------------------
// G34 全特性合流（G34-1 合流地基）：G34Full 形态——Mega 四 pass 同图 + fork A
// 纹理五 SSBO（22..=26）+ readback 第 5 项 U_SCENE_COLOR（fork B 动态核验面；
// scene SPV = kernels/g34_unified_gi.rx 统一 kernel——母版语义 + 图集采样块 +
// 实例分派块合一）。资源/pass/屏障与 [`unified_lane_descs`] 逐字同构，纹理面
// 缺省（tritex 全 −1）+ 动态面缺省（单实例 TLAS）== 母版位级（kernel 头注释
// 缺省面逐 op 论证；Stage A 锚格全链对拍承载）。
// ---------------------------------------------------------------------------

/// G34 fork A 追加资源下标（G34Full 形态才存在；22=逐三角 UV〔6 f32/tri〕，
/// 23=texmeta 头+槽表，24=逐三角槽索引〔−1 = 常量面〕，25=u32 打包 RGBA8
/// 图集，26=256 项 srgb→linear LUT——Split 形态 U_HIT_* 占用 22..=24 与
/// 本形态互斥，MegaSkin 形态 22..=28 亦互斥；形态间下标复用零撞面）。
#[allow(dead_code)] // G34 合流:g34_full_lane 独消费面(g14_3_pipeline_perf/g31_window_present 未消费,诚实标注)
const G34_U_TEX_UV: u32 = 22;
#[allow(dead_code)] // G34 合流:同上
const G34_U_TEX_META: u32 = 23;
#[allow(dead_code)] // G34 合流:同上
const G34_U_TEX_TRITEX: u32 = 24;
#[allow(dead_code)] // G34 合流:同上
const G34_U_TEX_ATLAS: u32 = 25;
#[allow(dead_code)] // G34 合流:同上
const G34_U_TEX_LINLUT: u32 = 26;
/// G34Full 形态资源数（Mega 22 + fork A 五件）。
#[allow(dead_code)] // G34 合流:g34_full_lane 独消费面(诚实标注)
const U_RESOURCE_COUNT_G34: usize = 27;

/// G34Full 形态 scene pass 屏障计划（U_PLAN_SCENE 触达超集 + fork A 五件——
/// 保守超集同律；读侧 SSBO 与写侧 out 同域 StorageReadWrite）。
#[allow(dead_code)] // G34 合流:g34_full_lane 独消费面(诚实标注)
const U_PLAN_SCENE_G34: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G34_U_TEX_UV, TargetState::StorageReadWrite),
    (G34_U_TEX_META, TargetState::StorageReadWrite),
    (G34_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G34_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G34_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];

/// G34 fork A 纹理侧表字节面（descs 借用源；bin 侧所有者——textures on =
/// g31_tex_load 产物（经 slab 合并语义预调制后）克隆面，textures off = 缺省
/// 哑件〔tritex 全 −1 ⇒ tex_gate = 0，kernel 行为 == 母版位级；1 纹素图集 +
/// 单槽 texmeta 保底读，采样块零消费〕）。
#[allow(dead_code)] // G34 合流:g34_full_lane 独消费面(诚实标注)
struct G34TexSideTable {
    texuv_bytes: Vec<u8>,
    texmeta_bytes: Vec<u8>,
    tritex_bytes: Vec<u8>,
    atlas_bytes: Vec<u8>,
    linlut_bytes: Vec<u8>,
}

impl G34TexSideTable {
    /// textures off 缺省面（`tri_count` = 静态 + 动态段总三角数；tritex 全 −1
    /// 常量面、texuv 全 0——kernel 缺省面 == 母版位级的数据面承载）。
    #[allow(dead_code)] // G34 合流:g34_full_lane 独消费面(诚实标注)
    fn default_face(tri_count: usize) -> Self {
        // texmeta：头 [aw=1, ah=1, slot_count=0, reserved×5] + 哑槽 [0,0,1,1,
        // mod=1,1,1,0]——保底读地址有效（tex_gate = 0 ⇒ 读出零消费；w/h = 1
        // 防 NaN 通道，mod = 1 中性面）。
        let texmeta: Vec<f32> = vec![
            1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // 头
            0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, // 哑槽
        ];
        G34TexSideTable {
            texuv_bytes: vec![0u8; tri_count * 6 * 4],
            texmeta_bytes: bytes_f32(&texmeta),
            tritex_bytes: bytes_f32(&vec![-1.0f32; tri_count]),
            atlas_bytes: vec![0u8; 4],
            linlut_bytes: g31_tex_linlut()
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect(),
        }
    }
}

/// G34 全特性合流描述组（Mega 同图 + fork A 五 SSBO + scene pass 换
/// g34_unified_gi 统一 kernel + readback 追加第 5 项 U_SCENE_COLOR 动态核验
/// 回读；资源/tris/mats/quads/points 四表与 Mega 面位级同 buffer——纹理缺省
/// 面 tritex 全 −1、动态缺省面单实例时行为 == 母版位级）。
#[allow(dead_code)] // G34 合流:g34_full_lane 独消费面(诚实标注)
#[allow(clippy::type_complexity)]
fn unified_lane_descs_g34<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    tex: &'x G34TexSideTable,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> (
    [ResourceDesc<'x>; U_RESOURCE_COUNT_G34],
    [Pass<'x>; 4],
    [&'static [(u32, TargetState)]; 4],
    [Readback; 5],
) {
    // Mega 面逐项解构（ResourceDesc/Pass 非 Copy——模式移动，与
    // unified_lane_descs 产物逐位同件零克隆）。
    let (
        [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16,
            m17, m18, m19, m20, m21],
        [_, p1, p2, p3],
        _b4,
        [rb0, rb1, rb2, rb3],
    ) = unified_lane_descs(assets, bits, iw, ih, ow, oh);
    let ipc = (iw * ih) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let resources = [
        m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16,
        m17, m18, m19, m20, m21,
        init(&tex.texuv_bytes),   // G34_U_TEX_UV
        init(&tex.texmeta_bytes), // G34_U_TEX_META
        init(&tex.tritex_bytes),  // G34_U_TEX_TRITEX
        init(&tex.atlas_bytes),   // G34_U_TEX_ATLAS
        init(&tex.linlut_bytes),  // G34_U_TEX_LINLUT
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "g34_unified_gi",
            spirv: &bits.spv_scene,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.scene_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    U_TRIS,
                    U_MATS,
                    U_QUADS,
                    U_POINTS,
                    U_SCENE_PARAMS,
                    G34_U_TEX_UV,
                    G34_U_TEX_META,
                    G34_U_TEX_TRITEX,
                    G34_U_TEX_ATLAS,
                    G34_U_TEX_LINLUT,
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                ],
                ..Bindings::default()
            },
        }),
        p1,
        p2,
        p3,
    ];
    let barriers = [U_PLAN_SCENE_G34, U_PLAN_MV, U_PLAN_RESAMPLE, U_PLAN_RESOLVE];
    let readbacks = [
        rb0,
        rb1,
        rb2,
        rb3,
        // fork B 动态实例位置核验回读面（MegaDyn readback 表第 5 项同字面；
        // subset 不消费零成本）。
        Readback::Buffer {
            res: U_SCENE_COLOR,
            offset: 0,
            size: ipc * 12,
        },
    ];
    (resources, passes, barriers, readbacks)
}

// ---------------------------------------------------------------------------
// D2 平滑顶点法线臂（夜间巡航 D2，--smooth-normals on 消费）：MegaSmoothNrm
// 形态——Mega 四 pass 同图 + 第 23 路 SSBO（逐三角顶点法线 9 f32/tri）+
// scene pass 换 kernels/g18_smooth_nrm.rx 编译产物。off 时本面全部不创建
// （不产侧表、不增资源、不换 SPV）——默认臂 0-byte。
// ---------------------------------------------------------------------------

/// D2 追加资源下标（MegaSmoothNrm 形态才存在；22 = 逐三角顶点法线
/// 〔9 f32/tri，世界旋转后〕——Split 形态 U_HIT_* 占用 22..=24、G34Full
/// 纹理五件占用 22..=26，形态间互斥下标复用零撞面同律）。
#[allow(dead_code)] // D2:MegaSmoothNrm 形态独消费面(诚实标注)
const U_TRINRM: u32 = 22;
/// D6 追加资源下标（MegaSmoothNrm 形态；23 = 逐三角 [metallic, roughness]
/// 〔2 f32/tri〕——--ggx off 面绑 8B 零哑表〔kernel params[48]=0 门不读〕，
/// on 面绑真表；下标挂 trinrm 尾部，与 Split/G34Full 形态互斥同律）。
#[allow(dead_code)] // D6:MegaSmoothNrm 形态独消费面(诚实标注)
const U_TRI_MR: u32 = 23;
/// MegaSmoothNrm 形态资源数（Mega 22 + trinrm + tri_mr 两路）。
#[allow(dead_code)] // D2:同上
const U_RESOURCE_COUNT_NRM: usize = 24;
/// MegaSmoothNrm 形态 scene pass 屏障计划（U_PLAN_SCENE 触达超集 + trinrm
/// + tri_mr——保守超集同律；读侧 SSBO 与写侧 out 同域 StorageReadWrite）。
#[allow(dead_code)] // D2:同上
const U_PLAN_SCENE_NRM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (U_TRINRM, TargetState::StorageReadWrite),
    (U_TRI_MR, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];

/// D2 平滑法线描述组（Mega 同图 + trinrm 一路 + scene pass 换 g18_smooth_nrm
/// kernel；tris/mats/quads/points 等 22 路与 Mega 面位级同 buffer——法线侧表
/// = 纯加性第 23 路，创建期一次上传 device-local）。
/// D6：+= tri_mr 一路（第 24 路；--ggx on = 真表〔2 f32/tri〕，off = 8B 零
/// 哑表——kernel params[48]=0 门均匀分支不读 ⇒ 哑表零消费零风险；scene pass
/// 绑定序与 kernels/g18_smooth_nrm.rx 签名逐字同源〔trinrm 后、out 双路前〕）。
#[allow(dead_code)] // D2:g14_3_pipeline_perf --smooth-normals on 独消费面(诚实标注)
#[allow(clippy::type_complexity)]
fn unified_lane_descs_nrm<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    trinrm_bytes: &'x [u8],
    tri_mr_bytes: &'x [u8],
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> (
    [ResourceDesc<'x>; U_RESOURCE_COUNT_NRM],
    [Pass<'x>; 4],
    [&'static [(u32, TargetState)]; 4],
    [Readback; 4],
) {
    // Mega 面逐项解构（ResourceDesc/Pass 非 Copy——模式移动，与
    // unified_lane_descs 产物逐位同件零克隆；G34Full 同型先例）。
    let (
        [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16,
            m17, m18, m19, m20, m21],
        [_, p1, p2, p3],
        _b4,
        rbs,
    ) = unified_lane_descs(assets, bits, iw, ih, ow, oh);
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let resources = [
        m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16,
        m17, m18, m19, m20, m21,
        init(trinrm_bytes), // U_TRINRM
        init(tri_mr_bytes), // U_TRI_MR（D6；off 面 = 8B 零哑表）
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "g18_smooth_nrm",
            spirv: &bits.spv_scene,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.scene_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    U_TRIS,
                    U_MATS,
                    U_QUADS,
                    U_POINTS,
                    U_SCENE_PARAMS,
                    U_TRINRM,
                    U_TRI_MR,
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                ],
                ..Bindings::default()
            },
        }),
        p1,
        p2,
        p3,
    ];
    let barriers = [U_PLAN_SCENE_NRM, U_PLAN_MV, U_PLAN_RESAMPLE, U_PLAN_RESOLVE];
    (resources, passes, barriers, rbs)
}

// ---------------------------------------------------------------------------
// day_0828 Phase C GI2 加性臂（--gi2 on 消费）：MegaTexNrmGi2 形态——
// MegaSmoothNrm 24 路 + 哑表五件（tex_gate=0 恒走 mats 均值面 = 与
// g18_smooth_nrm 光照语义一致）+ scene pass 换 kernels/g31_texture_nrm_gi.rx
// 统一质量 kernel（GI2 段 params[51..55) 消费面）。off 时本面全部不创建——
// 默认/既有臂 0-byte。
// ---------------------------------------------------------------------------

/// Phase C 追加资源下标（MegaTexNrmGi2 形态才存在；24..=28 = 哑表五件，
/// 绑定序照 kernels/g31_texture_nrm_gi.rx 签名 texuv/texmeta/tritex/atlas/
/// linlut——与 Split/G34Full 形态互斥下标复用零撞面同律）。
#[allow(dead_code)] // Phase C:MegaTexNrmGi2 形态独消费面(诚实标注)
const U_GI2_TEXUV: u32 = 24;
#[allow(dead_code)] // Phase C:同上
const U_GI2_TEXMETA: u32 = 25;
#[allow(dead_code)] // Phase C:同上
const U_GI2_TRITEX: u32 = 26;
#[allow(dead_code)] // Phase C:同上
const U_GI2_ATLAS: u32 = 27;
#[allow(dead_code)] // Phase C:同上
const U_GI2_LINLUT: u32 = 28;
/// MegaTexNrmGi2 形态资源数（MegaSmoothNrm 24 + 哑表五件）。
#[allow(dead_code)] // Phase C:同上
const U_RESOURCE_COUNT_TEXNRM_GI2: usize = 29;
/// MegaTexNrmGi2 形态 scene pass 屏障计划（U_PLAN_SCENE_NRM 触达超集 +
/// 哑表五件——保守超集同律）。
#[allow(dead_code)] // Phase C:同上
const U_PLAN_SCENE_TEXNRM_GI2: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (U_TRINRM, TargetState::StorageReadWrite),
    (U_TRI_MR, TargetState::StorageReadWrite),
    (U_GI2_TEXUV, TargetState::StorageReadWrite),
    (U_GI2_TEXMETA, TargetState::StorageReadWrite),
    (U_GI2_TRITEX, TargetState::StorageReadWrite),
    (U_GI2_ATLAS, TargetState::StorageReadWrite),
    (U_GI2_LINLUT, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];

/// Phase C GI2 bench 腿哑表五件（kernel gate=0 面的最小合法 buffer：
/// tritex[prim×2] 全 −1 ⇒ tex_gate=(slotf+1) 钳 0 恒走 mats 常量面〔albedo
/// 采样值 0·samp + 1·mats IEEE 精确〕；texuv 全 0（prim×6 寻址域全覆盖，
/// 采样值不消费但读地址须合法）；texmeta 头 8 + slot0 8〔w=h=1、mod=1、
/// mips=1 ⇒ lod 恒 0、mip 折半 while 零迭代〕；atlas 13 头项全指尾 texel +
/// 1 texel（slot0 全 mip 槽位 → 下标 13，wrap 后 4 fetch 同址合法）；
/// linlut 256 项 0（fetch 值全 0，经 tex_gate=0 门不入 albedo）。全部值域
/// 有限 ⇒ 0·x 无 NaN 污染。）
#[allow(dead_code)] // Phase C:g14_3_pipeline_perf --gi2 on 独消费面(诚实标注)
struct Gi2DummyTex {
    texuv_bytes: Vec<u8>,
    texmeta_bytes: Vec<u8>,
    tritex_bytes: Vec<u8>,
    atlas_bytes: Vec<u8>,
    linlut_bytes: Vec<u8>,
}

#[allow(dead_code)] // Phase C:同上
fn gi2_dummy_tex(tri_count: usize) -> Gi2DummyTex {
    let texuv = vec![0.0f32; tri_count * 6];
    let mut texmeta = vec![0.0f32; 16];
    texmeta[10] = 1.0; // w_base
    texmeta[11] = 1.0; // h_base
    texmeta[12] = 1.0; // mod_r
    texmeta[13] = 1.0; // mod_g
    texmeta[14] = 1.0; // mod_b
    texmeta[15] = 1.0; // mip_count
    let mut tritex = vec![0.0f32; tri_count * 2];
    let mut k = 0usize;
    while k < tri_count {
        tritex[k * 2] = -1.0; // slot=−1（tex_gate=0 常量面）；k_tri=0
        k += 1;
    }
    let mut atlas: Vec<u32> = vec![13u32; 13];
    atlas.push(0u32); // 尾 texel（4 fetch 同址；RGBA8 打包 0 → linlut[0]）
    let linlut = vec![0.0f32; 256];
    Gi2DummyTex {
        texuv_bytes: bytes_f32(&texuv),
        texmeta_bytes: bytes_f32(&texmeta),
        tritex_bytes: bytes_f32(&tritex),
        atlas_bytes: atlas.iter().flat_map(|x| x.to_le_bytes()).collect(),
        linlut_bytes: bytes_f32(&linlut),
    }
}

/// Phase C GI2 描述组（MegaSmoothNrm 同图 + 哑表五件 24..=28 + scene pass 换
/// g31_texture_nrm_gi 统一质量 kernel——绑定序与 kernels/g31_texture_nrm_gi.rx
/// 签名逐字同源〔trinrm/tri_mr 后、tex 五件、out 双路前〕；tris/mats 等
/// 24 路与 MegaSmoothNrm 面位级同 buffer）。
#[allow(dead_code)] // Phase C:g14_3_pipeline_perf --gi2 on 独消费面(诚实标注)
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn unified_lane_descs_texnrm_gi2<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    trinrm_bytes: &'x [u8],
    tri_mr_bytes: &'x [u8],
    dummy: &'x Gi2DummyTex,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> (
    [ResourceDesc<'x>; U_RESOURCE_COUNT_TEXNRM_GI2],
    [Pass<'x>; 4],
    [&'static [(u32, TargetState)]; 4],
    [Readback; 4],
) {
    // MegaSmoothNrm 面逐项解构（产物逐位同件零克隆；G34Full/nrm 同型先例）。
    let (
        [n0, n1, n2, n3, n4, n5, n6, n7, n8, n9, n10, n11, n12, n13, n14, n15, n16,
            n17, n18, n19, n20, n21, n22, n23],
        [_, p1, p2, p3],
        _b4,
        rbs,
    ) = unified_lane_descs_nrm(assets, bits, trinrm_bytes, tri_mr_bytes, iw, ih, ow, oh);
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let resources = [
        n0, n1, n2, n3, n4, n5, n6, n7, n8, n9, n10, n11, n12, n13, n14, n15, n16,
        n17, n18, n19, n20, n21, n22, n23,
        init(&dummy.texuv_bytes),   // U_GI2_TEXUV
        init(&dummy.texmeta_bytes), // U_GI2_TEXMETA
        init(&dummy.tritex_bytes),  // U_GI2_TRITEX
        init(&dummy.atlas_bytes),   // U_GI2_ATLAS
        init(&dummy.linlut_bytes),  // U_GI2_LINLUT
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "g31_texture_nrm_gi",
            spirv: &bits.spv_scene,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.scene_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    U_TRIS,
                    U_MATS,
                    U_QUADS,
                    U_POINTS,
                    U_SCENE_PARAMS,
                    U_TRINRM,
                    U_TRI_MR,
                    U_GI2_TEXUV,
                    U_GI2_TEXMETA,
                    U_GI2_TRITEX,
                    U_GI2_ATLAS,
                    U_GI2_LINLUT,
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                ],
                ..Bindings::default()
            },
        }),
        p1,
        p2,
        p3,
    ];
    let barriers = [
        U_PLAN_SCENE_TEXNRM_GI2,
        U_PLAN_MV,
        U_PLAN_RESAMPLE,
        U_PLAN_RESOLVE,
    ];
    (resources, passes, barriers, rbs)
}

/// 统一车道双形态（G14.10b）：Mega = 既有四 pass（bistro 等通用）；Split =
/// cornell 拆散六 pass（primary→scatter→reduce→mv→resample→resolve；quad 面光
/// 16 样本拆散重排——RT 单元延迟隐藏，reduce 固定层序求和与 megakernel 位级
/// 同序）。
#[allow(clippy::type_complexity)]
enum UnifiedDescs<'x> {
    Mega(
        (
            [ResourceDesc<'x>; U_RESOURCE_COUNT],
            [Pass<'x>; 4],
            [&'static [(u32, TargetState)]; 4],
            [Readback; 4],
        ),
    ),
    Split(
        (
            [ResourceDesc<'x>; U_RESOURCE_COUNT_SPLIT],
            [Pass<'x>; 6],
            [&'static [(u32, TargetState)]; 6],
            [Readback; 4],
        ),
    ),
    /// G31+ 波 A Task A4 动态场景形态：Mega 四 pass 同图 + readback 表追加第 5
    /// 项 U_SCENE_COLOR（动态实例位置核验回读面；subset 不消费零成本——仅
    /// --dyn-demo 车道创建，静态 Mega/Split 两面逐字不触）。
    MegaDyn(
        (
            [ResourceDesc<'x>; U_RESOURCE_COUNT],
            [Pass<'x>; 4],
            [&'static [(u32, TargetState)]; 4],
            [Readback; 5],
        ),
    ),
    /// G31+ 波 B Task B5 蒙皮形态：29 SSBO + 五 pass（skin → [blas refit 桥]
    /// → scene → mv → tsr 双 pass）+ 6 readback（前 5 项与 MegaDyn 逐字同 +
    /// U_SKIN_HIT;仅 --skin-demo 车道创建,静态/MegaDyn 面逐字不触）。
    MegaSkin(
        (
            [ResourceDesc<'x>; U_RESOURCE_COUNT_SKIN],
            [Pass<'x>; 5],
            [&'static [(u32, TargetState)]; 5],
            [Readback; 7],
        ),
    ),
    /// G34 全特性合流形态（G34-1 合流地基）：27 SSBO（Mega 22 + fork A 纹理
    /// 五件）+ 四 pass（scene = g34_unified_gi 统一 kernel——母版语义 + 图集
    /// 采样块 + 实例分派块合一）+ 5 readback（前 4 项与 Mega 逐字同 + 第 5 项
    /// U_SCENE_COLOR 动态核验面;仅 g34_full_lane 车道创建,Mega/Split/MegaDyn/
    /// MegaSkin 四面逐字不触）。
    #[allow(dead_code)] // G34 合流:g34_full_lane 独消费面(诚实标注)
    G34Full(
        (
            [ResourceDesc<'x>; U_RESOURCE_COUNT_G34],
            [Pass<'x>; 4],
            [&'static [(u32, TargetState)]; 4],
            [Readback; 5],
        ),
    ),
    /// D2 平滑顶点法线形态（--smooth-normals on）：24 SSBO（Mega 22 +
    /// U_TRINRM 逐三角顶点法线〔9 f32/tri〕+ D6 U_TRI_MR 逐三角
    /// [metallic, roughness]〔2 f32/tri；--ggx off 面 = 8B 零哑表〕）+
    /// 四 pass（scene = g18_smooth_nrm kernel——g18 母版语义 + params[43]
    /// 门重心插值法线 + params[48] 门 GGX 高光臂）+ 4 readback（与 Mega
    /// 逐字同）；仅 g14_3_pipeline_perf --smooth-normals on 车道创建，
    /// Mega/Split/MegaDyn/MegaSkin/G34Full 五面逐字不触。
    #[allow(dead_code)] // D2:g14_3_pipeline_perf --smooth-normals on 独消费面(诚实标注)
    MegaSmoothNrm(
        (
            [ResourceDesc<'x>; U_RESOURCE_COUNT_NRM],
            [Pass<'x>; 4],
            [&'static [(u32, TargetState)]; 4],
            [Readback; 4],
        ),
    ),
    /// day_0828 Phase C GI2 形态（--gi2 on）：29 SSBO（MegaSmoothNrm 24 +
    /// 哑表五件 24..=28）+ 四 pass（scene = g31_texture_nrm_gi 统一质量
    /// kernel——GI2 段 params[51..55) 消费面；tex_gate=0 恒走 mats 均值面）
    /// + 4 readback（与 Mega 逐字同）；仅 g14_3_pipeline_perf --gi2 on 车道
    /// 创建，其余六面逐字不触。
    #[allow(dead_code)] // Phase C:g14_3_pipeline_perf --gi2 on 独消费面(诚实标注)
    MegaTexNrmGi2(
        (
            [ResourceDesc<'x>; U_RESOURCE_COUNT_TEXNRM_GI2],
            [Pass<'x>; 4],
            [&'static [(u32, TargetState)]; 4],
            [Readback; 4],
        ),
    ),
}

/// Split 形态描述组（cornell 拆散车道：25 SSBO + 六 pass；资源 0..=21 与 Mega
/// 逐字同布局，追加 hitinfo 双缓冲 + blk 16 层缓冲）。bin 侧 fail-closed 前置
/// 断言 = quad_count==1（16 层映射单灯语义）。
#[allow(clippy::type_complexity)]
fn unified_lane_descs_split<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> (
    [ResourceDesc<'x>; U_RESOURCE_COUNT_SPLIT],
    [Pass<'x>; 6],
    [&'static [(u32, TargetState)]; 6],
    [Readback; 4],
) {
    let ipc = (iw * ih) as u64;
    let opc = (ow * oh) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    // G14.10d 判定规则同 Mega 形态：params 三小件 = host-visible，其余 =
    // DEVICE_LOCAL 驻留（hitinfo/blk 中间缓冲 GPU 链内直读写，收益最大）。
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
        size,
        usage: storage,
        data: None,
            device_local: true,
        })
    };
    let host_init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: false,
        })
    };
    let host_buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: false,
        })
    };
    let resources = [
        init(&assets.tris_bytes),         // U_TRIS
        init(&assets.mats_bytes),         // U_MATS
        init(&assets.quads_bytes),        // U_QUADS
        init(&assets.points_bytes),       // U_POINTS
        host_init(&assets.params0_bytes), // U_SCENE_PARAMS
        buf(assets.out_color_size),       // U_SCENE_COLOR
        buf(assets.out_depth_size),       // U_SCENE_DEPTH
        host_buf(40 * 4),                 // U_MV_PARAMS
        buf(ipc * 8),                     // U_MV_OUT
        host_buf(32 * 4),                 // U_TSR_PARAMS
        init(&bits.reactive_zeros),       // U_REACTIVE
        buf(opc * 12),                    // U_CUR_RGB
        buf(opc * 4),                     // U_LUMA[0]
        buf(opc * 4),                     // U_LUMA[1]
        buf(opc * 4),                     // U_DEPTH_HI[0]
        buf(opc * 4),                     // U_DEPTH_HI[1]
        buf(opc * 12),                    // U_OUT_COLOR[0]
        buf(opc * 12),                    // U_OUT_COLOR[1]
        buf(opc * 4),                     // U_OUT_SIGN[0]
        buf(opc * 4),                     // U_OUT_SIGN[1]
        buf(opc * 4),                     // U_OUT_SCORE[0]
        buf(opc * 4),                     // U_OUT_SCORE[1]
        buf(ipc * 4),                     // U_HIT_T
        buf(ipc * 4),                     // U_HIT_PRIM
        buf(ipc * 16 * 4),                // U_BLK（16 层 blk 布尔面，px-major）
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "g14_3_primary",
            spirv: &bits.spv_primary,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.primary_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![U_SCENE_PARAMS, U_HIT_T, U_HIT_PRIM],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_3_shadow_scatter",
            spirv: &bits.spv_scatter,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.scatter_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    U_TRIS,
                    U_QUADS,
                    U_SCENE_PARAMS,
                    U_HIT_T,
                    U_HIT_PRIM,
                    U_BLK,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_3_shade_reduce",
            spirv: &bits.spv_reduce,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.reduce_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    U_TRIS,
                    U_MATS,
                    U_QUADS,
                    U_POINTS,
                    U_SCENE_PARAMS,
                    U_HIT_T,
                    U_HIT_PRIM,
                    U_BLK,
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_mv",
            spirv: &bits.spv_mv,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.mv_dispatch),
            bindings: Bindings {
                storage_buffers: vec![U_SCENE_DEPTH, U_MV_PARAMS, U_MV_OUT],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_8_tsr_resample",
            spirv: &bits.spv_resample,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.resample_dispatch),
            bindings: Bindings {
                storage_buffers: vec![
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                    U_TSR_PARAMS,
                    U_CUR_RGB,
                    U_LUMA[0],
                    U_DEPTH_HI[0],
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_8_tsr_resolve",
            spirv: &bits.spv_resolve,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.resolve_dispatch),
            bindings: Bindings {
                storage_buffers: vec![
                    U_CUR_RGB,
                    U_LUMA[0],
                    U_DEPTH_HI[0],
                    U_MV_OUT,
                    U_REACTIVE,
                    U_OUT_COLOR[1],
                    U_DEPTH_HI[1],
                    U_LUMA[1],
                    U_OUT_SIGN[1],
                    U_OUT_SCORE[1],
                    U_TSR_PARAMS,
                    U_OUT_COLOR[0],
                    U_OUT_SIGN[0],
                    U_OUT_SCORE[0],
                ],
                ..Bindings::default()
            },
        }),
    ];
    let barriers = [
        U_PLAN_PRIMARY,
        U_PLAN_SCATTER,
        U_PLAN_REDUCE,
        U_PLAN_MV,
        U_PLAN_RESAMPLE,
        U_PLAN_RESOLVE,
    ];
    let readbacks = [
        Readback::Buffer {
            res: U_OUT_COLOR[0],
            offset: 0,
            size: opc * 12,
        },
        Readback::Buffer {
            res: U_OUT_COLOR[1],
            offset: 0,
            size: opc * 12,
        },
        Readback::Buffer {
            res: U_MV_OUT,
            offset: 0,
            size: ipc * 8,
        },
        Readback::Buffer {
            res: U_SCENE_DEPTH,
            offset: 0,
            size: ipc * 4,
        },
    ];
    (resources, passes, barriers, readbacks)
}

/// 统一车道状态机（session + parity/历史/prev_vp_j；render/bench 双腿同一
/// 执行面）。parity 轮换与历史门与原 TsrDeviceBackend 逐字同律：帧 i
/// parity=i%2，resolve 读 [1−p] 写 [p]；首帧 has_history=0 且 has_prev=0。
struct UnifiedTsrLane<'a> {
    session: DeviceFrameSession<'a>,
    parity: usize,
    has_history_state: bool,
    prev_vp_j: Option<Mat4>,
    /// mv 诊断探针（env RURIX_G14_MV_PROBE=1；回读帧追加回读 mv_out/scene_depth
    /// 供 host compute_camera_mv 逐分量对拍——digest 漂移归因臂，常态恒 false
    /// 零成本）。
    probe: bool,
    /// Split 形态（cornell 拆散六 pass 车道；false = Mega 四 pass）。
    split: bool,
    /// scene pass telemetry 名（G34 加性面：descs 首 pass 声明名直取——
    /// Mega/MegaDyn = "g14_3_direct_gi"（与硬编码字面逐字同值，0-byte），
    /// G34Full = "g34_unified_gi"；Split 面不消费〔三固定名拆散提取〕，
    /// MegaSkin 面 frame_skin 自有 rec 提取不消费）。
    scene_name: &'a str,
    /// G31（波 A Task A2）FIF 流水在飞票据 FIFO（深度 ≤ inflight；空 = 顺序
    /// 面，`frame()` 0-byte 路径不变）。
    pending: VecDeque<PendingTsrFrame>,
    /// FIF 深度（1 = 顺序全同步既有面；2/3 = 真流水，session frame_slots =
    /// max(2, inflight)——inflight=1 与既有 2 槽创建面逐字同）。
    inflight: usize,
    /// D2 平滑顶点法线臂（MegaSmoothNrm 形态 true → prepare_update 置
    /// params[43]=1.0；其余形态恒 false，参数面与既有逐位同值 0-byte）。
    smooth_nrm: bool,
    /// D6 GGX 高光臂（创建期恒 false；--ggx on 车道创建后经 [`Self::set_ggx`]
    /// 一次性挂载 → prepare_update 置 params[48]=1.0〔须 smooth_nrm 同 on，
    /// pack 面第二重保险〕；其余形态/默认臂恒 false，参数面 0-byte）。
    ggx: bool,
    /// A1 灯贡献剔除阈值（创建期恒 0.0；--lamp-lights on 车道创建后经
    /// [`Self::set_lamp_contrib`] 一次性挂载 → prepare_update 置 params[49]；
    /// 0.0 与零填充逐位同值 ⇒ 既有全臂参数面 0-byte）。
    lamp_contrib: f32,
    /// day_0828 Phase C GI2 臂（MegaTexNrmGi2 形态 --gi2 on 车道创建后经
    /// [`Self::set_gi2`] 一次性挂载 scale/clamp + 逐帧 [`Self::set_gi2_frame`]
    /// 挂载帧序号（R2 时域旋转）→ prepare_update 置 params[51..55)；off/其余
    /// 形态恒 false ⇒ 四槽不写与零填充逐位同值，参数面 0-byte）。
    gi2: bool,
    gi2_scale: f32,
    gi2_clamp: f32,
    gi2_frame: f32,
    /// day_0828 Phase D TSR 降噪质量档（--tsr-quality on 车道创建后经
    /// [`Self::set_tsrq`] 一次性挂载 → prepare_update 置 tsr_params[19..21)
    /// 〔[19]=稳态 alpha 档/[20]=邻域 clamp K〕;off/其余形态恒 false ⇒ 两槽
    /// 不写与零填充逐位同值,参数面 0-byte——resolve SPV 换载在 CLI 面完成,
    /// 字节隔离纪律〕）。
    tsrq: bool,
    tsrq_min_alpha: f32,
    tsrq_clamp: f32,
    /// G38（RFC-0030 v1.1 §4.3 L2a）：每槽 AS 副本组（opt-in；None = 既有面
    /// 0-byte）。经 [`Self::create_with_slot_as`] 建，组 [0, inflight)——逐帧
    /// `tlas_update` 目标与 scene pass AS 绑定轮换到 base + slot 表项。
    slot_as_group: Option<SlotAsGroup>,
    /// scene pass（下标 0）创建期绑定组克隆（slot_as 逐帧 AS 换槽 override 的
    /// 单一事实源——禁在提交面手写绑定列表，防与 descs 双源漂移；非 slot_as
    /// 车道恒 None 零成本）。
    scene_bindings: Option<Bindings>,
    /// slot_as 动态臂在飞票据 FIFO（与静态 `pending` 分列——静态
    /// submit_frame/collect_frame 字面 0-byte）。
    pending_dyn: VecDeque<PendingDynFrame>,
    /// G38 批次 B：skin scene pass（下标 1 `g31_skin_scene`）创建期绑定组
    /// 克隆（MegaSkin 形态 scene pass 非下标 0——与 `scene_bindings`〔pass 0〕
    /// 分列存档，skin slot_as 逐帧 AS 换槽 override 的单一事实源；非 MegaSkin/
    /// 非 slot_as 车道恒 None 零成本，dyn/静态臂既有面 0-byte）。
    skin_scene_bindings: Option<Bindings>,
    /// slot_as 蒙皮臂在飞票据 FIFO（与静态 `pending`/动态 `pending_dyn`
    /// 分列——既有两面 submit/collect 字面 0-byte）。
    pending_skin: VecDeque<PendingSkinFrame>,
}

/// G31 FIF 流水在飞帧簿记（`submit_with_frame_update` 产出的票据 + 该帧
/// readback/digest 面归属——回读/digest 延迟到 FIFO 出队 collect，归属信息随
/// 票据旅行；帧序不乱由 FIFO 纪律承载）。
struct PendingTsrFrame {
    ticket: FrameTicket,
    /// 提交序帧号（flip-trace digest 行归属 + 末帧 digest 判据）。
    frame_index: u32,
    /// 本帧是否请求回读（bench 末帧/flip-trace 帧 true）。
    readback_out: bool,
}

/// G38 slot_as 动态臂在飞帧簿记（`submit_with_frame_update_slot_as` 票据 +
/// 回读意图随票据延迟到 collect；核验帧组装凭帧号纯函数在 collect 侧复算）。
#[allow(dead_code)] // G38 L2a:g14_3_pipeline_perf --dyn-demo×--inflight 2|3 独消费面(其余 include 方未消费,诚实标注)
struct PendingDynFrame {
    ticket: FrameTicket,
    /// 提交序帧号（flip-trace digest 行归属 + 核验帧轨迹/相机复算输入）。
    frame_index: u32,
    /// 本帧是否请求 TSR 输出回读（bench 末帧/flip-trace 帧 true）。
    readback_out: bool,
    /// 动态核验帧（scene color 回读在子集；collect 侧组装 DynVerifyFrame）。
    readback_scene: bool,
}

/// G38 slot_as 蒙皮臂在飞帧簿记（`submit_with_frame_update_slot_as` 票据 +
/// 回读/核验/诊断意图随票据延迟到 collect；核验帧组装凭帧号纯函数在 collect
/// 侧复算——骨骼 palette/轨迹/相机同律）。
#[allow(dead_code)] // G38 L2a 批次 B:g14_3_pipeline_perf --skin-demo×--inflight 2|3 独消费面(其余 include 方未消费,诚实标注)
struct PendingSkinFrame {
    ticket: FrameTicket,
    /// 提交序帧号（flip-trace digest 行归属 + 核验帧 palette/相机复算输入）。
    frame_index: u32,
    /// 本帧是否请求 TSR 输出回读（bench 末帧/flip-trace 帧 true）。
    readback_out: bool,
    /// 蒙皮核验帧（mv/scene/hit 三路回读在子集;collect 侧组装 SkinVerifyFrame）。
    verify: bool,
    /// 蒙皮输出对拍诊断臂（U_TRIS 角色段回读在子集;RURIX_SKIN_DEBUG_TRIS）。
    debug_tris: bool,
}

/// 统一车道一帧产物（GPU 分段 = DeviceFrameTelemetry 逐 pass timestamp；
/// out_color 仅回读帧有值；mv_out/depth 仅探针回读帧有值）。
struct UnifiedFrameRec {
    scene_gpu_ns: f64,
    mv_gpu_ns: f64,
    resample_gpu_ns: f64,
    resolve_gpu_ns: f64,
    cpu_record_ns: u64,
    cpu_submit_ns: u64,
    cpu_fence_wait_ns: u64,
    validation_error_count: u64,
    out_color: Option<Vec<f32>>,
    mv_out: Option<Vec<f32>>,
    depth: Option<Vec<f32>>,
    /// G31+ Task A4：U_SCENE_COLOR 回读（内部分辨率 f32×3；仅 dyn 核验帧
    /// Some——动态实例位置检测面，TSR 前瞬时位无历史拖影）。
    scene_color: Option<Vec<f32>>,
    /// 回读字节→f32 转换耗时（毫秒；零回读帧恒 0）——digest/校验面的前置
    /// 转换步，bench 腿计入 tail（非生产段，诚实口径）。
    readback_convert_ms: f64,
    /// 提交序帧号（G31 流水面 flip-trace 归属；顺序面恒 0 不被消费——顺序
    /// flip-trace 行号取循环下标，0-byte）。
    frame_index: u32,
    /// C7 profiler 面：本帧全量逐 pass GPU 计时（telemetry 声明序;(pass 名, ns)）。
    /// 四段提取面 0-byte——既有字段全部同值维持,本列为 --profile-json 唯一消费面。
    pass_gpu_ns: Vec<(String, f64)>,
}

impl<'a> UnifiedTsrLane<'a> {
    #[allow(clippy::type_complexity)]
    fn create(
        descs: &'a UnifiedDescs<'a>,
        accel_structs: &[AccelStructDesc<'a>],
        inflight: usize,
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        // G31：frame_slots = max(2, inflight)（inflight=1 与既有 2 槽面逐字
        // 同——0-byte；2/3 = FIF 真流水深度上限，slot/fence/timestamp 区间随
        // 槽数，流水面 per-slot 资源懒建于首个流水 submit）。
        let frame_slots = inflight.max(2);
        // G34 加性面：scene pass telemetry 名 = descs 首 pass 声明名（Mega/
        // MegaDyn 车道 = "g14_3_direct_gi" 与原硬编码逐字同值；G34Full =
        // "g34_unified_gi"）——非 compute 首 pass fail-closed（创建期闭集）。
        let scene_name = match descs {
            UnifiedDescs::Mega(d) => match &d.1[0] {
                Pass::Compute(cp) => cp.name,
                _ => return Err("descs 首 pass 非 compute（scene pass 门面）".into()),
            },
            UnifiedDescs::Split(d) => match &d.1[0] {
                Pass::Compute(cp) => cp.name,
                _ => return Err("descs 首 pass 非 compute（scene pass 门面）".into()),
            },
            UnifiedDescs::MegaDyn(d) => match &d.1[0] {
                Pass::Compute(cp) => cp.name,
                _ => return Err("descs 首 pass 非 compute（scene pass 门面）".into()),
            },
            UnifiedDescs::MegaSkin(d) => match &d.1[0] {
                Pass::Compute(cp) => cp.name,
                _ => return Err("descs 首 pass 非 compute（scene pass 门面）".into()),
            },
            UnifiedDescs::G34Full(d) => match &d.1[0] {
                Pass::Compute(cp) => cp.name,
                _ => return Err("descs 首 pass 非 compute（scene pass 门面）".into()),
            },
            UnifiedDescs::MegaSmoothNrm(d) => match &d.1[0] {
                Pass::Compute(cp) => cp.name,
                _ => return Err("descs 首 pass 非 compute（scene pass 门面）".into()),
            },
            UnifiedDescs::MegaTexNrmGi2(d) => match &d.1[0] {
                Pass::Compute(cp) => cp.name,
                _ => return Err("descs 首 pass 非 compute（scene pass 门面）".into()),
            },
        };
        let (session, split) = match descs {
            UnifiedDescs::Mega(d) => (
                DeviceFrameSession::new_with_accel_structs(
                    &d.0,
                    &d.1,
                    &d.2,
                    &d.3,
                    frame_slots,
                    accel_structs,
                )?,
                false,
            ),
            UnifiedDescs::Split(d) => (
                DeviceFrameSession::new_with_accel_structs(
                    &d.0,
                    &d.1,
                    &d.2,
                    &d.3,
                    frame_slots,
                    accel_structs,
                )?,
                true,
            ),
            // G31+ Task A4：MegaDyn = Mega 同图 + readback 第 5 项（动态核验面；
            // split=false——pass 索引/绑定面与 Mega 逐字同）。
            UnifiedDescs::MegaDyn(d) => (
                DeviceFrameSession::new_with_accel_structs(
                    &d.0,
                    &d.1,
                    &d.2,
                    &d.3,
                    frame_slots,
                    accel_structs,
                )?,
                false,
            ),
            // G31+ 波 B Task B5：MegaSkin = 29 SSBO 五 pass 图（split=false;
            // parity override 下标 (3,4) 归 frame_skin 面）。
            UnifiedDescs::MegaSkin(d) => (
                DeviceFrameSession::new_with_accel_structs(
                    &d.0,
                    &d.1,
                    &d.2,
                    &d.3,
                    frame_slots,
                    accel_structs,
                )?,
                false,
            ),
            // G34：G34Full = 27 SSBO 四 pass 图 + readback 第 5 项（动态核验面；
            // split=false——pass 索引/绑定面与 Mega 逐字同构）。
            UnifiedDescs::G34Full(d) => (
                DeviceFrameSession::new_with_accel_structs(
                    &d.0,
                    &d.1,
                    &d.2,
                    &d.3,
                    frame_slots,
                    accel_structs,
                )?,
                false,
            ),
            // D2：MegaSmoothNrm = 23 SSBO 四 pass 图（Mega 22 + U_TRINRM）+
            // scene pass 换 g18_smooth_nrm kernel；split=false——pass 索引/
            // parity override/回读表与 Mega 逐字同构（仅 scene 绑定多一路）。
            UnifiedDescs::MegaSmoothNrm(d) => (
                DeviceFrameSession::new_with_accel_structs(
                    &d.0,
                    &d.1,
                    &d.2,
                    &d.3,
                    frame_slots,
                    accel_structs,
                )?,
                false,
            ),
            // Phase C：MegaTexNrmGi2 = 29 SSBO 四 pass 图（MegaSmoothNrm 24 +
            // 哑表五件）+ scene pass 换 g31_texture_nrm_gi；split=false——
            // pass 索引/parity override/回读表与 Mega 逐字同构。
            UnifiedDescs::MegaTexNrmGi2(d) => (
                DeviceFrameSession::new_with_accel_structs(
                    &d.0,
                    &d.1,
                    &d.2,
                    &d.3,
                    frame_slots,
                    accel_structs,
                )?,
                false,
            ),
        };
        Ok(Self {
            session,
            parity: 0,
            has_history_state: false,
            prev_vp_j: None,
            probe: std::env::var("RURIX_G14_MV_PROBE").ok().as_deref() == Some("1"),
            split,
            scene_name,
            pending: VecDeque::new(),
            inflight,
            // Phase C：GI2 形态 = 统一质量 kernel（平滑法线臂内含——CLI 已裁
            // 「--gi2 须随 --smooth-normals on」，params[43]=1.0 同置）。
            smooth_nrm: matches!(
                descs,
                UnifiedDescs::MegaSmoothNrm(_) | UnifiedDescs::MegaTexNrmGi2(_)
            ),
            ggx: false,
            lamp_contrib: 0.0,
            gi2: false,
            gi2_scale: 0.0,
            gi2_clamp: 0.0,
            gi2_frame: 0.0,
            tsrq: false,
            tsrq_min_alpha: 0.0,
            tsrq_clamp: 0.0,
            // G38 L2a：opt-in 面缺省关闭（经 create_with_slot_as 显式建组；
            // None/空与既有全部车道行为逐位同——0-byte）。
            slot_as_group: None,
            scene_bindings: None,
            pending_dyn: VecDeque::new(),
            skin_scene_bindings: None,
            pending_skin: VecDeque::new(),
        })
    }

    /// G38（RFC-0030 v1.1 §4.3 L2a）每槽 AS 副本 opt-in 创建面：
    /// `accel_structs` 须为 inflight（≥2）份同构副本（调用方显式构造——每表项
    /// 独立 instance buffer/BLAS/TLAS/scratch，AS 面内存 ×S 显式代价，预算门
    /// 条目 g31.fif_dyn.slot_as_group_mem_bytes）；组 [0, inflight)；scene
    /// pass（下标 0）绑定组自 descs 克隆存档，供逐帧 AS 换槽 override
    /// （单一事实源，禁提交面手写）。既有 [`Self::create`] 字面 0-byte。
    /// G38 批次 B 加性：MegaSkin 形态另存 skin scene pass（下标 1）绑定组
    /// 克隆（`skin_scene_bindings`——skin 臂 scene pass 非下标 0；非 MegaSkin
    /// 恒 None，dyn/静态臂 0-byte）。
    #[allow(dead_code)] // G38 L2a:g14_3_pipeline_perf --dyn-demo×--inflight 2|3 独消费面(其余 include 方未消费,诚实标注)
    fn create_with_slot_as(
        descs: &'a UnifiedDescs<'a>,
        accel_structs: &[AccelStructDesc<'a>],
        inflight: usize,
    ) -> Result<Self, String> {
        if inflight < 2 || accel_structs.len() != inflight {
            return Err(format!(
                "slot_as 组：inflight ≥2 且 AS 表须 {inflight} 份同构副本（实得 {}；L2a opt-in 显式条件）",
                accel_structs.len()
            ));
        }
        // scene pass 绑定组克隆（descs 首 pass；与 create 的 scene_name 门面
        // 同一闭集——非 compute 首 pass 创建期已拒）。
        let passes: &[Pass<'a>] = match descs {
            UnifiedDescs::Mega(d) => &d.1[..],
            UnifiedDescs::Split(d) => &d.1[..],
            UnifiedDescs::MegaDyn(d) => &d.1[..],
            UnifiedDescs::MegaSkin(d) => &d.1[..],
            UnifiedDescs::G34Full(d) => &d.1[..],
            UnifiedDescs::MegaSmoothNrm(d) => &d.1[..],
            UnifiedDescs::MegaTexNrmGi2(d) => &d.1[..],
        };
        let scene_bindings = match &passes[0] {
            Pass::Compute(cp) => cp.bindings.clone(),
            _ => return Err("descs 首 pass 非 compute（scene pass 门面）".into()),
        };
        // G38 批次 B：MegaSkin 的 scene pass = 下标 1（g31_skin_scene——pass0
        // = g31_skin 蒙皮求值，无 AS 绑定面），换槽 override 的创建期绑定组
        // 加性分列存档（非 MegaSkin 恒 None；dyn/静态臂既有面 0-byte）。
        let skin_scene_bindings = if matches!(descs, UnifiedDescs::MegaSkin(_)) {
            match passes.get(1) {
                Some(Pass::Compute(cp)) => Some(cp.bindings.clone()),
                _ => {
                    return Err(
                        "MegaSkin descs 第 2 pass 非 compute（skin scene pass 门面）".into()
                    )
                }
            }
        } else {
            None
        };
        let mut lane = Self::create(descs, accel_structs, inflight)?;
        lane.slot_as_group = Some(SlotAsGroup {
            base: 0,
            len: inflight as u32,
        });
        lane.scene_bindings = Some(scene_bindings);
        lane.skin_scene_bindings = skin_scene_bindings;
        Ok(lane)
    }

    /// D6 GGX 高光臂开关挂载（--ggx on 车道创建后一次性；仅 MegaSmoothNrm
    /// 形态 + tri_mr 真表绑定面调用——其余形态调用 = 参数面置位但 kernel
    /// 无 tri_mr 绑定/无 GGX 代码，CLI 已裁不可能组合，fail-closed 兜底由
    /// 调用面承担）。off 车道不调用 ⇒ 参数面 0-byte。
    fn set_ggx(&mut self, ggx: bool) {
        self.ggx = ggx;
    }

    /// A1 灯贡献剔除阈值挂载（--lamp-lights on 车道创建后一次性；off 车道
    /// 不调用 ⇒ 恒 0.0 参数面 0-byte——0.0 本身亦与零填充逐位同值）。
    fn set_lamp_contrib(&mut self, contrib: f32) {
        self.lamp_contrib = contrib;
    }

    /// Phase C GI2 臂挂载（--gi2 on 车道创建后一次性 scale/clamp；off 车道
    /// 不调用 ⇒ gi2=false 四槽不写参数面 0-byte）。
    #[allow(dead_code)] // Phase C:g14_3_pipeline_perf --gi2 on 独消费面(诚实标注)
    fn set_gi2(&mut self, scale: f32, clamp: f32) {
        self.gi2 = true;
        self.gi2_scale = scale;
        self.gi2_clamp = clamp;
    }

    /// Phase C GI2 帧序号逐帧挂载（params[52]=frame_idx——R2 时域旋转，TSR
    /// 收敛面；双跑同帧序 ⇒ 位级一致口径不破。off 车道不调用零消费）。
    #[allow(dead_code)] // Phase C:同上
    fn set_gi2_frame(&mut self, frame_idx: f32) {
        self.gi2_frame = frame_idx;
    }

    /// Phase D TSR 降噪质量档挂载（--tsr-quality on 车道创建后一次性
    /// min_alpha/clamp → tsr_params[19..21)；off 车道不调用 ⇒ 两槽不写
    /// 参数面 0-byte——SPV 换载归 CLI 面,本挂载仅参数槽）。
    #[allow(dead_code)] // Phase D:g14_3_pipeline_perf --tsr-quality on 独消费面(诚实标注)
    fn set_tsrq(&mut self, min_alpha: f32, clamp: f32) {
        self.tsrq = true;
        self.tsrq_min_alpha = min_alpha;
        self.tsrq_clamp = clamp;
    }

    /// 本帧 FrameUpdate + provenance 组装（顺序/流水两面同一事实源：
    /// 三小件参数打包 → parity 轮换 binding_overrides → provenance 预推）。
    /// readback_out=false 时 readback_subset=Some([]) 零回读（bench 测量循环
    /// 面）；true 时回读 out_color[p]（render 逐帧出图 / bench 末帧 digest /
    /// flip-trace 诊断）。
    #[allow(clippy::too_many_arguments)]
    fn prepare_update(
        &self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        eps: f32,
        quad_count: usize,
        point_count: usize,
        inv_vp: &Mat4,
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback_out: bool,
    ) -> Result<(SubmissionProvenance, FrameUpdate), String> {
        // D2：MegaSmoothNrm 形态 self.smooth_nrm=true → params[43]=1.0；其余
        // 形态 false ⇒ 参数面与既有逐位同值（0-byte）。
        // D6：self.ggx=true（--ggx on 车道 set_ggx 挂载）且 smooth_nrm 同 on
        // → params[48]=1.0；ggx=false 面产物与 D6 前逐位同值（0-byte）。
        // A1：self.lamp_contrib（--lamp-lights on 车道 set_lamp_contrib 挂载）
        // → params[49]；默认 0.0 与零填充逐位同值（0-byte）。
        // Phase C：self.gi2（--gi2 on 车道 set_gi2/set_gi2_frame 挂载）→
        // params[51..55)；false 面四槽不写与零填充逐位同值（0-byte；
        // tex_kpix 恒 0.0——bench 哑表面 mip 选择恒 lod 0）。
        let scene_params = pack_frame_params_gi2(
            iw,
            ih,
            jitter,
            eps,
            quad_count,
            point_count,
            inv_vp,
            vp,
            self.smooth_nrm,
            self.ggx,
            self.lamp_contrib,
            0.0,
            self.gi2,
            self.gi2_frame,
            self.gi2_clamp,
            self.gi2_scale,
        );
        // 静态面 0-byte：tlas_update=None + readback_scene=false + 48 f32 参数——
        // 产物 FrameUpdate 与重构前逐字段同（G31+ Task A4 ext 共享体承载扩面；
        // G38 末参 None = 零 scene override，同 0-byte）。
        self.prepare_update_ext(
            iw,
            ih,
            ow,
            oh,
            jitter,
            vp_j,
            exposure,
            reset,
            readback_out,
            false,
            scene_params,
            None,
            None,
        )
    }

    /// G31+ 波 A Task A4 扩面组装（与 [`Self::prepare_update`] 同一事实源：
    /// scene_params 调用方预打包（dyn 车道 60 f32 面）+ tlas_update 可选 +
    /// readback_scene 追加 U_SCENE_COLOR 回读（MegaDyn readback 表第 5 项，
    /// 下标 4）。静态调用（tlas_update=None, readback_scene=false, 48 f32）
    /// 产物与原 prepare_update 逐字段同——0-byte 保持。
    /// G38 L2a 加性参数 `scene_as_override`：Some(as_index) 时追加 scene pass
    /// （下标 0）绑定组 override（accel_structs 换到本槽副本表项——须在构造
    /// 器内完成：prov 由 update 派生，构造后改绑定必致 provenance 校验 RED）；
    /// None = 既有全部调用面产物逐字段同（0-byte）。
    #[allow(clippy::too_many_arguments)]
    fn prepare_update_ext(
        &self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback_out: bool,
        readback_scene: bool,
        scene_params: Vec<f32>,
        tlas_update: Option<(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
        scene_as_override: Option<u32>,
    ) -> Result<(SubmissionProvenance, FrameUpdate), String> {
        // mv 参数面：inv_cur = vp_j 逆（host `Mat4::inverse` 伴随法——
        // compute_camera_mv 内部同一实现同一输入，位级同源）；prev = 上一帧
        // vp_j（host 循环 prev_vp 同语义）；首帧 has_prev=0，kernel 门直写零。
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆（mv 参数面）")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        let has_history = !reset && self.has_history_state;
        // Phase D：self.tsrq（--tsr-quality on 车道 set_tsrq 挂载）→
        // tsr_params[19..21)〔[19]=稳态 alpha 档/[20]=邻域 clamp K〕；false
        // 面两槽不写与零填充逐位同值（0-byte——冻结 resolve kernel 不读
        // [19..21)，仅 g31_tsr_resolve_q 变体消费）。
        let mut tsr_params = pack_tsr_params(iw, ih, ow, oh, jitter, exposure, has_history, false);
        if self.tsrq {
            tsr_params[19] = self.tsrq_min_alpha;
            tsr_params[20] = self.tsrq_clamp;
        }
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
        // parity 轮换绑定（原 TsrDeviceBackend 同律；布局键与创建期逐位一致
        // ——override 同构校验面）。
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
        // parity override 的 pass 索引按形态（Mega: resample=2/resolve=3；
        // Split: 六 pass 车道 resample=4/resolve=5）。
        let (idx_resample, idx_resolve) = if self.split { (4, 5) } else { (2, 3) };
        let readback_subset = {
            let mut v: Vec<u32> = Vec::new();
            if readback_out {
                v.push(p as u32);
                // 探针帧追加 mv_out(2)/scene_depth(3) 回读（归因臂）。
                if self.probe {
                    v.push(2);
                    v.push(3);
                }
            }
            // G31+ Task A4：动态实例位置核验回读（MegaDyn 表第 5 项下标 4）。
            if readback_scene {
                v.push(4);
            }
            v
        };
        let mut binding_overrides = vec![
            (idx_resample, bindings_resample),
            (idx_resolve, bindings_resolve),
        ];
        if let Some(as_index) = scene_as_override {
            // G38 L2a 每槽 AS 描述符集：scene pass（0）组内 AS 绑定逐帧轮换到
            // 本槽副本（绑定组 = 创建期克隆，仅 accel_structs 换槽——per-slot
            // override set 既有基建承载，零新描述符面）。
            let mut b = self
                .scene_bindings
                .clone()
                .ok_or("slot_as：scene 绑定组未建（须经 create_with_slot_as）")?;
            b.accel_structs = vec![as_index];
            binding_overrides.push((0, b));
        }
        let update = FrameUpdate {
            tlas_update,
            buffer_uploads: uploads,
            binding_overrides,
            push_constant_overrides: vec![],
            readback_subset: Some(readback_subset),
            blas_refit: None, // G31+ 波 B Task B5 字段面:本车道无 BLAS refit(0-byte 默认)
        };
        let prov = self.session.next_provenance_with_update(&update)?;
        Ok((prov, update))
    }

    /// 一帧产物组装（顺序/流水两面同一事实源：telemetry 逐 pass 提取 +
    /// 回读字节→f32 转换 + 尺寸/路数校验 + readback_convert_ms 计量）。
    /// readback_scene=true 时末路回读 = U_SCENE_COLOR（内部分辨率；G31+ Task A4
    /// 动态核验面，仅 MegaDyn readback 表下标 4 合法；iw/ih 仅该校验消费）。
    fn rec_from_output(
        &self,
        out: DeviceFrameOutput,
        readback_out: bool,
        readback_scene: bool,
        ow: u32,
        oh: u32,
        iw: u32,
        ih: u32,
    ) -> Result<UnifiedFrameRec, String> {
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        // scene 段：Mega = 单 megakernel；Split = primary+scatter+reduce 三和
        // （拆散车道的场景直接光工作总量,与 megakernel 同语义段）。G34：非
        // Split 面 scene pass 名 = 创建期 descs 声明名（Mega/MegaDyn 与原
        // 硬编码 "g14_3_direct_gi" 逐字同值——0-byte；G34Full = g34_unified_gi）。
        let scene_gpu_ns = if self.split {
            gpu("g14_3_primary")? + gpu("g14_3_shadow_scatter")? + gpu("g14_3_shade_reduce")?
        } else {
            gpu(self.scene_name)?
        };
        let mv_gpu_ns = gpu("g14_mv")?;
        let resample_gpu_ns = gpu("g14_8_tsr_resample")?;
        let resolve_gpu_ns = gpu("g14_8_tsr_resolve")?;
        let t_convert = std::time::Instant::now();
        let want_base = if readback_out {
            if self.probe { 3 } else { 1 }
        } else {
            0
        };
        let want = want_base + if readback_scene { 1 } else { 0 };
        if out.readbacks.len() != want {
            return Err(format!(
                "统一车道回读路数 {} ≠ {want}（readback_out={readback_out} probe={} readback_scene={readback_scene}）",
                out.readbacks.len(),
                self.probe
            ));
        }
        let (out_color, mv_out, depth) = if readback_out {
        let data = read_f32(&out.readbacks[0]);
            if data.len() != (ow * oh * 3) as usize {
                return Err("统一车道回读字节数与输出分辨率不符".into());
            }
            let mv_out = self.probe.then(|| read_f32(&out.readbacks[1]));
            let depth = self.probe.then(|| read_f32(&out.readbacks[2]));
            (Some(data), mv_out, depth)
        } else {
            (None, None, None)
        };
        let scene_color = if readback_scene {
            let data = read_f32(&out.readbacks[want_base]);
            if data.len() != (iw * ih * 3) as usize {
                return Err("统一车道 scene color 回读字节数与内部分辨率不符".into());
            }
            Some(data)
        } else {
            None
        };
        let readback_convert_ms = t_convert.elapsed().as_secs_f64() * 1000.0;
        // C7 profiler 面:全量逐 pass GPU 计时（telemetry 声明序直拷）。
        let pass_gpu_ns: Vec<(String, f64)> = out
            .telemetry
            .passes
            .iter()
            .map(|pp| (pp.name.clone(), pp.gpu_ns))
            .collect();
        Ok(UnifiedFrameRec {
            scene_gpu_ns,
            mv_gpu_ns,
            resample_gpu_ns,
            resolve_gpu_ns,
            cpu_record_ns: out.telemetry.cpu_record_ns,
            cpu_submit_ns: out.telemetry.cpu_submit_ns,
            cpu_fence_wait_ns: out.telemetry.cpu_fence_wait_ns,
            validation_error_count: out.telemetry.validation_error_count,
            out_color,
            mv_out,
            depth,
            scene_color,
            readback_convert_ms,
            frame_index: 0,
            pass_gpu_ns,
        })
    }

    /// 帧状态推进（parity 轮换 / prev_vp_j / 历史门；顺序面在产物组装后、
    /// 流水面在 submit 后推进——帧参数已随上传烘定，推进点对 GPU 输入零
    /// 影响，两面逐帧状态序列逐位同）。
    fn advance(&mut self, vp_j: &Mat4) {
        self.prev_vp_j = Some(*vp_j);
        self.has_history_state = true;
        self.parity = 1 - self.parity;
    }

    /// 一帧：三小件参数上传（scene 192B + mv 160B + tsr 128B 逐帧覆盖）→
    /// 四 pass GPU 链内执行（零 host 中转）→ 可选 TSR 输出回读。
    /// 顺序面（inflight=1 既有路径 0-byte——submit + 当帧 fence 等待全同步
    /// 口径，G14.3 起回归锚；= prepare_update → execute_with_frame_update
    /// → rec_from_output → advance 的原序薄封装）。
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        eps: f32,
        quad_count: usize,
        point_count: usize,
        inv_vp: &Mat4,
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback_out: bool,
    ) -> Result<UnifiedFrameRec, String> {
        let (prov, update) = self.prepare_update(
            iw, ih, ow, oh, jitter, eps, quad_count, point_count, inv_vp, vp, vp_j, exposure,
            reset, readback_out,
        )?;
        let out = self.session.execute_with_frame_update(&prov, &update)?;
        // 静态面 0-byte：readback_scene=false（scene_color 恒 None）。
        let rec = self.rec_from_output(out, readback_out, false, ow, oh, iw, ih)?;
        self.advance(vp_j);
        Ok(rec)
    }

    /// G31+ 波 A Task A4 动态场景一帧（顺序入口专用——tlas_update 走
    /// `execute_with_frame_update`；FIF 流水面公共入口已拒 tlas_update，本车道
    /// 恒 inflight=1，CLI fail-closed 保证）：场景参数 60 f32（含 dyn_tri_base）
    /// + 实例变换经 tlas_update（refit/rebuild）→ 四 pass GPU 链内执行 → 可选
    /// TSR 输出 + scene color 双回读（核验帧）。与 [`Self::frame`] 同一执行事实源。
    #[allow(clippy::too_many_arguments)]
    fn frame_dyn(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        scene_params: Vec<f32>,
        tlas_update: (u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction),
        readback_out: bool,
        readback_scene: bool,
    ) -> Result<UnifiedFrameRec, String> {
        let (prov, update) = self.prepare_update_ext(
            iw,
            ih,
            ow,
            oh,
            jitter,
            vp_j,
            exposure,
            reset,
            readback_out,
            readback_scene,
            scene_params,
            Some(tlas_update),
            None,
        )?;
        let out = self.session.execute_with_frame_update(&prov, &update)?;
        let rec = self.rec_from_output(out, readback_out, readback_scene, ow, oh, iw, ih)?;
        self.advance(vp_j);
        Ok(rec)
    }

    /// G31+ 波 B Task B5 蒙皮帧组装（[`Self::frame_skin`] 顺序面与
    /// [`Self::submit_frame_skin_slot_as`] slot_as 流水面**同一构造事实源**
    /// ——原 frame_skin 内联构造段逐字搬移;scene_as_override=None 产物与原
    /// 内联构造逐字段同,行为 0 变）。
    /// G38 L2a 加性参数 `scene_as_override`：Some(as_index) 时追加 skin scene
    /// pass（下标 1 `g31_skin_scene`——MegaSkin 形态 pass0 = g31_skin 蒙皮
    /// 求值,scene pass 非下标 0）绑定组 override（accel_structs 换到本槽副本
    /// 表项——须在构造器内完成:prov 由 update 派生,构造后改绑定必致
    /// provenance 校验 RED）；None = 既有调用面产物逐字段同（0-byte）。
    #[allow(clippy::too_many_arguments)]
    fn prepare_update_skin(
        &self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        scene_params: Vec<f32>,
        skin_params: Vec<f32>,
        pal_cur_bytes: Vec<u8>,
        pal_prev_bytes: Vec<u8>,
        blas: BlasRefitUpdate,
        readback_out: bool,
        verify: bool,
        debug_tris: bool,
        scene_as_override: Option<u32>,
    ) -> Result<(SubmissionProvenance, FrameUpdate), String> {
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆（mv 参数面）")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mut mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        // 蒙皮 MV kernel [35] = char_inst（角色实例下标;g31_skin_mv 覆盖臂
        // 分派面——静态 40 f32 参数面该槽恒 0 reserved,本车道唯一消费）。
        mv_params[35] = 1.0;
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
            (
                StableResourceId(u64::from(U_SKIN_PARAMS) + 1),
                0,
                bytes_f32(&skin_params),
            ),
            (
                StableResourceId(u64::from(U_SKIN_PAL_CUR) + 1),
                0,
                pal_cur_bytes,
            ),
            (
                StableResourceId(u64::from(U_SKIN_PAL_PREV) + 1),
                0,
                pal_prev_bytes,
            ),
        ];
        // parity 轮换绑定（Mega 同律;MegaSkin pass 下标 resample=3/resolve=4）。
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
        let mut readback_subset: Vec<u32> = Vec::new();
        if readback_out {
            readback_subset.push(p as u32);
        }
        if verify {
            readback_subset.push(2); // U_MV_OUT（蒙皮 MV 核验面）
            readback_subset.push(4); // U_SCENE_COLOR（位置检测面）
            readback_subset.push(5); // U_SKIN_HIT（inst 地面真值检测/取证面）
        }
        if debug_tris {
            readback_subset.push(6); // U_TRIS 角色段（蒙皮输出对拍诊断臂）
        }
        let mut binding_overrides = vec![
            (3, bindings_resample),
            (4, bindings_resolve),
        ];
        if let Some(as_index) = scene_as_override {
            // G38 L2a 每槽 AS 描述符集：skin scene pass（1）组内 AS 绑定逐帧
            // 轮换到本槽副本（绑定组 = 创建期克隆，仅 accel_structs 换槽——
            // per-slot override set 既有基建承载，零新描述符面；既有
            // (3,resample)/(4,resolve) parity overrides 不动）。
            let mut b = self
                .skin_scene_bindings
                .clone()
                .ok_or("slot_as：skin scene 绑定组未建（须经 create_with_slot_as〔MegaSkin〕）")?;
            b.accel_structs = vec![as_index];
            binding_overrides.push((1, b));
        }
        let update = FrameUpdate {
            tlas_update: None,
            buffer_uploads: uploads,
            binding_overrides,
            push_constant_overrides: vec![],
            readback_subset: Some(readback_subset),
            blas_refit: Some(blas),
        };
        let prov = self.session.next_provenance_with_update(&update)?;
        Ok((prov, update))
    }

    /// 蒙皮一帧产物组装（顺序/slot_as FIF 两面**同一事实源**：telemetry 五
    /// pass 逐名提取;回读按子集构建序解析——原 [`Self::frame_skin`] 内联 rec
    /// 组装段逐字搬移,行为 0 变;frame_index 恒 0,流水面由
    /// [`Self::collect_frame_skin`] 自票据回填）。
    #[allow(clippy::too_many_arguments)]
    fn skin_rec_from_output(
        &self,
        out: DeviceFrameOutput,
        readback_out: bool,
        verify: bool,
        debug_tris: bool,
        ow: u32,
        oh: u32,
        iw: u32,
        ih: u32,
    ) -> Result<SkinFrameRec, String> {
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        let want = usize::from(readback_out)
            + if verify { 3 } else { 0 }
            + usize::from(debug_tris);
        if out.readbacks.len() != want {
            return Err(format!(
                "蒙皮车道回读路数 {} ≠ {want}（readback_out={readback_out} verify={verify} debug_tris={debug_tris}）",
                out.readbacks.len()
            ));
        }
        let t_convert = std::time::Instant::now();
        let mut k = 0usize;
        let out_color = if readback_out {
            let data = read_f32(&out.readbacks[k]);
            k += 1;
            if data.len() != (ow * oh * 3) as usize {
                return Err("蒙皮车道回读字节数与输出分辨率不符".into());
            }
            Some(data)
        } else {
            None
        };
        let (mv_out, scene_color, hit) = if verify {
            let mv = read_f32(&out.readbacks[k]);
            k += 1;
            if mv.len() != (iw * ih * 2) as usize {
                return Err("蒙皮车道 mv 回读字节数与内部分辨率不符".into());
            }
            let sc = read_f32(&out.readbacks[k]);
            k += 1;
            if sc.len() != (iw * ih * 3) as usize {
                return Err("蒙皮车道 scene color 回读字节数与内部分辨率不符".into());
            }
            let hb = read_f32(&out.readbacks[k]);
            k += 1;
            if hb.len() != (iw * ih * 4) as usize {
                return Err("蒙皮车道 hit 回读字节数与内部分辨率不符".into());
            }
            (Some(mv), Some(sc), Some(hb))
        } else {
            (None, None, None)
        };
        let debug_tris_data = if debug_tris {
            let dt = read_f32(&out.readbacks[k]);
            Some(dt)
        } else {
            None
        };
        Ok(SkinFrameRec {
            skin_gpu_ns: gpu("g31_skin")?,
            scene_gpu_ns: gpu("g31_skin_scene")?,
            mv_gpu_ns: gpu("g31_skin_mv")?,
            resample_gpu_ns: gpu("g14_8_tsr_resample")?,
            resolve_gpu_ns: gpu("g14_8_tsr_resolve")?,
            cpu_record_ns: out.telemetry.cpu_record_ns,
            cpu_submit_ns: out.telemetry.cpu_submit_ns,
            cpu_fence_wait_ns: out.telemetry.cpu_fence_wait_ns,
            validation_error_count: out.telemetry.validation_error_count,
            out_color,
            mv_out,
            scene_color,
            hit,
            debug_tris: debug_tris_data,
            readback_convert_ms: t_convert.elapsed().as_secs_f64() * 1000.0,
            frame_index: 0,
        })
    }

    /// G31+ 波 B Task B5 蒙皮一帧（MegaSkin 车道;顺序入口专用——blas_refit
    /// 走 `execute_with_frame_update` 的 pass0 后桥,FIF 流水面已拒 BLAS
    /// 更新,本车道恒 inflight=1,CLI fail-closed 保证）：scene 参数 60 f32
    /// （含 skin_tri_base）+ mv 参数 40 f32（[35]=char_inst）+ tsr 128B +
    /// skin 参数 64B + palette 双表逐帧上传;`blas` = 角色 BLAS refit 桥
    /// （蒙皮输出段 → BLAS 1 顶点缓冲 → UPDATE build）;verify 帧回读
    /// [mv(2), scene_color(4)]（out 帧前置 parity 回读,子集序 =
    /// [out?, mv, scene]——rec 解析同序）。与 [`Self::frame_dyn`] 同一
    /// 执行事实源（next_provenance_with_update → execute_with_frame_update
    /// → advance 原序）。G38 批次 B：构造/rec 组装两段提取为
    /// [`Self::prepare_update_skin`]/[`Self::skin_rec_from_output`]（原地
    /// 逐字搬移,本面 = 原序薄封装,行为 0 变）。
    #[allow(clippy::too_many_arguments)]
    fn frame_skin(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        scene_params: Vec<f32>,
        skin_params: Vec<f32>,
        pal_cur_bytes: Vec<u8>,
        pal_prev_bytes: Vec<u8>,
        blas: BlasRefitUpdate,
        readback_out: bool,
        verify: bool,
        debug_tris: bool,
    ) -> Result<SkinFrameRec, String> {
        let (prov, update) = self.prepare_update_skin(
            iw,
            ih,
            ow,
            oh,
            jitter,
            vp_j,
            exposure,
            reset,
            scene_params,
            skin_params,
            pal_cur_bytes,
            pal_prev_bytes,
            blas,
            readback_out,
            verify,
            debug_tris,
            None,
        )?;
        let out = self.session.execute_with_frame_update(&prov, &update)?;
        let rec =
            self.skin_rec_from_output(out, readback_out, verify, debug_tris, ow, oh, iw, ih)?;
        self.advance(vp_j);
        Ok(rec)
    }

    /// G31（波 A Task A2）FIF 流水提交半程：组装与顺序面同一事实源 →
    /// `submit_with_frame_update`（submit 后**不等**当帧 fence）→ 票据入
    /// FIFO；帧状态随即推进（参数已随上传烘定）。回读/digest 面延迟到
    /// [`Self::collect_frame`]。
    #[allow(clippy::too_many_arguments)]
    fn submit_frame(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        eps: f32,
        quad_count: usize,
        point_count: usize,
        inv_vp: &Mat4,
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback_out: bool,
        frame_index: u32,
    ) -> Result<(), String> {
        let (prov, update) = self.prepare_update(
            iw, ih, ow, oh, jitter, eps, quad_count, point_count, inv_vp, vp, vp_j, exposure,
            reset, readback_out,
        )?;
        let ticket = self.session.submit_with_frame_update(&prov, &update)?;
        self.pending.push_back(PendingTsrFrame {
            ticket,
            frame_index,
            readback_out,
        });
        self.advance(vp_j);
        Ok(())
    }

    /// 在飞票据数（FIFO 深度；流水循环的 collect 触发判据）。
    fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// G31 FIF 流水收集半程：FIFO 出队最早票据 → `collect`（有界 fence 等待
    /// + per-slot staging 回读 + provenance/telemetry 落账）→ 与顺序面同一
    /// `rec_from_output` 产物组装；帧号归属自票据回填（帧序不乱）。
    fn collect_frame(&mut self, ow: u32, oh: u32) -> Result<UnifiedFrameRec, String> {
        let pending = self.pending.pop_front().ok_or_else(|| {
            "FIF collect: 无在飞票据（提交/收集配平破缺,fail-closed)".to_owned()
        })?;
        let out = self.session.collect(pending.ticket)?;
        // 流水面恒 readback_scene=false（FIF 入口已拒 tlas_update——动态核验
        // 回读面不在流水车道；iw/ih 校验收口于 readback_scene=false 恒不消费）。
        let mut rec = self.rec_from_output(out, pending.readback_out, false, ow, oh, 0, 0)?;
        rec.frame_index = pending.frame_index;
        Ok(rec)
    }

    /// G38（RFC-0030 v1.1 §4.3 L2a）动态臂 slot_as FIF 提交半程：与
    /// [`Self::frame_dyn`] 同一构造事实源（prepare_update_ext + scene 换槽
    /// override），`tlas_update` 目标 = 本槽副本（base + slot；rt 入口槽纪律
    /// 三判据〔错槽/组外/跨槽绑定〕提交前 fail-closed 复核），票据入
    /// `pending_dyn`（与静态 `pending` 分列）。host 实例写序钉在本槽 fence
    /// 之后由 rt 入口承载；帧状态随即推进（与静态 submit_frame 同律）。
    #[allow(dead_code)] // G38 L2a:g14_3_pipeline_perf --dyn-demo×--inflight 2|3 独消费面(其余 include 方未消费,诚实标注)
    #[allow(clippy::too_many_arguments)]
    fn submit_frame_dyn_slot_as(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        scene_params: Vec<f32>,
        insts: Vec<RayQueryTransformedInstanceDesc>,
        action: TlasBuildAction,
        readback_out: bool,
        readback_scene: bool,
        frame_index: u32,
    ) -> Result<(), String> {
        let group = self
            .slot_as_group
            .ok_or("slot_as 组未建（须经 create_with_slot_as；L2a opt-in）")?;
        let slot = self.session.next_frame_slot() as u32;
        let target = group.base + slot;
        let (prov, update) = self.prepare_update_ext(
            iw,
            ih,
            ow,
            oh,
            jitter,
            vp_j,
            exposure,
            reset,
            readback_out,
            readback_scene,
            scene_params,
            Some((target, insts, action)),
            Some(target),
        )?;
        let ticket = self
            .session
            .submit_with_frame_update_slot_as(&prov, &update, &group)?;
        self.pending_dyn.push_back(PendingDynFrame {
            ticket,
            frame_index,
            readback_out,
            readback_scene,
        });
        self.advance(vp_j);
        Ok(())
    }

    /// slot_as 动态臂在飞票据数（FIFO 深度；FIF 循环 collect 触发判据）。
    #[allow(dead_code)] // G38 L2a:g14_3_pipeline_perf 独消费面(诚实标注)
    fn pending_dyn_len(&self) -> usize {
        self.pending_dyn.len()
    }

    /// G38 slot_as 动态臂收集半程：FIFO 出队最早票据 → `collect` → 与
    /// [`Self::frame_dyn`] 同一 `rec_from_output` 事实源（readback_scene 随
    /// 票据——核验帧 scene color 在子集；帧号自票据回填,FIFO 保序）。
    #[allow(dead_code)] // G38 L2a:g14_3_pipeline_perf 独消费面(诚实标注)
    fn collect_frame_dyn(
        &mut self,
        ow: u32,
        oh: u32,
        iw: u32,
        ih: u32,
    ) -> Result<UnifiedFrameRec, String> {
        let p = self.pending_dyn.pop_front().ok_or_else(|| {
            "slot_as collect: 无在飞票据（提交/收集配平破缺,fail-closed)".to_owned()
        })?;
        let out = self.session.collect(p.ticket)?;
        let mut rec =
            self.rec_from_output(out, p.readback_out, p.readback_scene, ow, oh, iw, ih)?;
        rec.frame_index = p.frame_index;
        Ok(rec)
    }

    /// G38（RFC-0030 v1.1 §4.3 L2a 批次 B）蒙皮臂 slot_as FIF 提交半程：与
    /// [`Self::frame_skin`] 同一构造事实源（prepare_update_skin + skin scene
    /// pass〔下标 1〕换槽 override）。与动态臂 [`Self::submit_frame_dyn_slot_as`]
    /// 形同，差异 = 无 tlas_update——`blas_refit` 目标 = 本槽副本（as_index =
    /// base + slot 逐帧换槽，其余字段与顺序臂字面同源；rt 入口槽纪律三判据
    /// 〔错槽/组外/跨槽绑定〕提交前 fail-closed 复核）；palette/params uploads
    /// 走既有 per-slot staging（FIF 兼容面零改动）。票据入 `pending_skin`
    /// （与静态 `pending`/动态 `pending_dyn` 分列）；帧状态随即推进（与静态
    /// submit_frame 同律）。
    #[allow(dead_code)] // G38 L2a 批次 B:g14_3_pipeline_perf --skin-demo×--inflight 2|3 独消费面(其余 include 方未消费,诚实标注)
    #[allow(clippy::too_many_arguments)]
    fn submit_frame_skin_slot_as(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        scene_params: Vec<f32>,
        skin_params: Vec<f32>,
        pal_cur_bytes: Vec<u8>,
        pal_prev_bytes: Vec<u8>,
        blas: BlasRefitUpdate,
        readback_out: bool,
        verify: bool,
        debug_tris: bool,
        frame_index: u32,
    ) -> Result<(), String> {
        let group = self
            .slot_as_group
            .ok_or("slot_as 组未建（须经 create_with_slot_as；L2a opt-in）")?;
        let slot = self.session.next_frame_slot() as u32;
        let target = group.base + slot;
        // blas_refit 目标逐帧换槽（as_index = 本槽副本表项;其余字段
        // 〔blas_index/src/src_offset/byte_len/after_pass〕与顺序臂调用方
        // 字面同源直传——蒙皮源段/角色 BLAS 下标不随槽变）。
        let blas = BlasRefitUpdate {
            as_index: target,
            ..blas
        };
        let (prov, update) = self.prepare_update_skin(
            iw,
            ih,
            ow,
            oh,
            jitter,
            vp_j,
            exposure,
            reset,
            scene_params,
            skin_params,
            pal_cur_bytes,
            pal_prev_bytes,
            blas,
            readback_out,
            verify,
            debug_tris,
            Some(target),
        )?;
        let ticket = self
            .session
            .submit_with_frame_update_slot_as(&prov, &update, &group)?;
        self.pending_skin.push_back(PendingSkinFrame {
            ticket,
            frame_index,
            readback_out,
            verify,
            debug_tris,
        });
        self.advance(vp_j);
        Ok(())
    }

    /// slot_as 蒙皮臂在飞票据数（FIFO 深度；FIF 循环 collect 触发判据）。
    #[allow(dead_code)] // G38 L2a 批次 B:g14_3_pipeline_perf 独消费面(诚实标注)
    fn pending_skin_len(&self) -> usize {
        self.pending_skin.len()
    }

    /// G38 slot_as 蒙皮臂收集半程：FIFO 出队最早票据 → `collect` → 与
    /// [`Self::frame_skin`] 同一 `skin_rec_from_output` 事实源（verify/
    /// debug_tris 随票据——核验帧 mv/scene/hit 三路回读在子集；帧号自票据
    /// 回填,FIFO 保序,核验组装凭帧号在调用方复算 palette/相机）。
    #[allow(dead_code)] // G38 L2a 批次 B:g14_3_pipeline_perf 独消费面(诚实标注)
    fn collect_frame_skin(
        &mut self,
        ow: u32,
        oh: u32,
        iw: u32,
        ih: u32,
    ) -> Result<SkinFrameRec, String> {
        let p = self.pending_skin.pop_front().ok_or_else(|| {
            "slot_as collect: 无在飞票据（提交/收集配平破缺,fail-closed)".to_owned()
        })?;
        let out = self.session.collect(p.ticket)?;
        let mut rec = self
            .skin_rec_from_output(out, p.readback_out, p.verify, p.debug_tris, ow, oh, iw, ih)?;
        rec.frame_index = p.frame_index;
        Ok(rec)
    }
}

// ---------------------------------------------------------------------------
// G14.10e dlss_sr 臂驻留统一车道（RFC-0030 §4.3 vendor 输入驻留接线）：单一
// render_exec DeviceFrameSession 三 pass（pass0=scene(g14_3_direct_gi) →
// pass1=mv(g14_mv+NoContraction) → pass2=pack(手编 SPV)）全 GPU 链内，pack
// 直写三 exportable image（RGBA32F color / R32F depth / RG32F mv——SL 输入
// 格式容忍度 G14.10b 冒烟臂①实证），DLSS session 经 OPAQUE_WIN32 导入同一块
// device memory 后 `upscale_resident_external` 驻留 evaluate。原现状结构的
// scene 逐帧回读（~24.9MB@1080p）+ host compute_camera_mv + vendor host
// pack/upload 三段中转税全消；scene session readback 表恒空（dlss 臂的
// last_frame_digest 语义 = **DLSS 输出**，回读靠 DlssVkSession::
// readback_output_into，scene 输出不再落 host）。
//
// 形态登记：dlss 臂恒 Mega 单 kernel（bistro/cornell 同构通用；cornell 拆散
// 三 pass（primary/scatter/reduce）仅 tsr_device 臂消费——本车道不接线，
// 简化裁决如实登记）。
//
// 数值面登记（digest 锚影响，G14.10e 如实预告）：① mv = GPU g14_mv 输出
// （与 host compute_camera_mv 存在 ULP 级运算差——tsr 臂 mv GPU 化同族 L1）；
// ② color 输入 RGBA32F f32 直通（现状 host pack 路径 = f32→f16 RGBA16F）；
// ③ depth 输入 R32F（现状 D32，数值同 f32 位面）。DLSS evaluate 输入位面
// 变化 ⇒ 输出 digest 相对现状锚预期漂移（改图 L1 级），双跑位级确定性不受
// 影响——按门纪律如实报告新 digest 与双跑一致性，不硬凑。
// ---------------------------------------------------------------------------

/// dlss 驻留车道资源下标闭集（场景区 0..=6 + mv 区 7..=8 与统一车道 U_* 逐字
/// 同布局同下标——常量直接复用；9..=11 = 三 exportable 纹理）。
///
/// G14.12 路线复位：G14.10f 曾把 image 共享判为「OPAQUE_WIN32 跨 device
/// OPTIMAL tiling 布局解释不一致」而退回 exportable buffer + DLSS 侧逐帧
/// `vkCmdCopyBufferToImage`（t100 面 41.5MB/帧、实测 0.6ms GPU）。该归因**已被
/// 证伪**：真实根因是导入侧 `vkCreateImage` 的 `imageType` 笔误（`2` =
/// `VK_IMAGE_TYPE_3D`，注释写 "2D"），与导出侧 `IMAGE_TYPE_2D`(=1) 不符 ⇒
/// 同一块显存被两侧按不同 tiling 布局解释。本机 memreq 对拍实证：1920×1080
/// RGBA16F OPTIMAL 下 2D 需 17694720 字节、3D 需 16588800 字节。修正
/// `imageType` 后 image 共享成立，三条 copy 整体消失。
const D_TEX_COLOR: u32 = 9;
const D_TEX_DEPTH: u32 = 10;
const D_TEX_MV: u32 = 11;
const D_RESOURCE_COUNT: usize = 12;

/// pack pass 屏障计划（保守超集逐字声明同律：SSBO 三源 = StorageReadWrite，
/// storage image 三标 = StorageImageReadWrite/GENERAL——exportable 面 layout
/// 恒 GENERAL，与帧末 EXTERNAL release 收敛态一致）。
const D_PLAN_PACK: &[(u32, TargetState)] = &[
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (U_MV_OUT, TargetState::StorageReadWrite),
    (D_TEX_COLOR, TargetState::StorageImageReadWrite),
    (D_TEX_DEPTH, TargetState::StorageImageReadWrite),
    (D_TEX_MV, TargetState::StorageImageReadWrite),
];

// （fsr 车道的三纹理常量/屏障计划/descs/bits 见 F_* 区段——G14.11 fsr 域
// 自持,D3D12 SHARED 导入路线与本 dlss buffer 共享路线并存。）

/// G14.12 手编 pack compute SPIR-V（SPIR-V 1.0；LocalSize 8×8；沿
/// geometry/visbuffer_swhw_spv.rs inst/words 手编体例）：scene 车道 SSBO 三源
/// （color 3f32/px、depth 1f32/px、mv 2f32/px；binding 0/1/2）逐像素直写三
/// exportable storage image（**Rgba16f**/R32f/Rg32f；binding 3/4/5 =
/// storage_images 区段 [N..N+K)，layout GENERAL）。
///
/// 色彩语义沿 G14.10f 修正：**rgb × exposure 转显示域**（TSR resample
/// `o = v·exposure` 同律——vendor 臂输出与 tsr 臂/UE 基准同域；bistro
/// ev100=−4 时 scene 域直通实测暗 2^4；host `pack_vendor_inputs` 共享面零
/// 触碰，乘法仅落本 G14.3 车道；cornell exposure=1.0 IEEE 位保持 ⇒ cornell
/// digest 锚零漂判据）。
///
/// color 标格式取 **Rgba16f** 而非 G14.10e 的 Rgba32f：① DLSS 输入位面与
/// G14.10f buffer 路逐位同（`OpImageWrite` 的 f32→f16 为硬件 RTE，与
/// `PackHalf2x16` 同舍入）⇒ 该项不引入画质/digest 差异；② 写带宽 33.2→16.6MB。
/// depth/mv 为 f32 位拷贝零浮点算术（OpLoad→OpCompositeConstruct→OpImageWrite）。
///
/// push constants = {w:u32, h:u32, exposure:f32}；越界门 px<w && py<h；
/// i = py·w+px。Rg32f storage 格式须 `StorageImageExtendedFormats`
/// capability（49；设备特性经 `new_with_exportable_textures` 启用）。
#[allow(clippy::too_many_lines)]
fn g14_pack_spv() -> Vec<u32> {
    fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
        v.push(op | ((ops.len() as u32 + 1) << 16));
        v.extend_from_slice(ops);
    }
    fn words(s: &str) -> Vec<u32> {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
        b.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }
    // id 布局：类型/常量/全局 < 100；函数体自 100（visbuffer_swhw_spv 同律）。
    // G14.12 image 共享版：SSBO 三源(color 3f32 / depth 1f32 / mv 2f32) →
    // storage image 三标(Rgba16f / R32f / Rg32f)。相对 G14.10f buffer 版的
    // 差别 = 输出面由「紧凑 u32 对 SSBO」改回「storage image 直写」,DLSS 侧
    // 三条 buffer→image copy 由此整体消失(imageType 笔误修正后 image 共享成立)。
    let t_void = 1u32;
    let t_fn = 2;
    let t_bool = 3;
    let t_u32 = 4;
    let t_f32 = 5;
    let t_v3u = 6;
    let p_in_v3u = 7;
    let v_gid = 8;
    let t_v2u = 9;
    let t_v4f = 10;
    let t_rt_f32 = 11;
    let t_st_f32 = 12;
    let p_uni_st_f = 13;
    let p_uni_f32 = 14;
    let v_color = 15;
    let v_depth = 16;
    let v_mv = 17;
    let t_img_c = 18;
    let p_img_c = 19;
    let v_img_c = 20;
    let t_img_d = 21;
    let p_img_d = 22;
    let v_img_d = 23;
    let t_img_m = 24;
    let p_img_m = 25;
    let v_img_m = 26;
    let t_pc = 27;
    let p_pc = 28;
    let v_pc = 29;
    let p_pc_u32 = 30;
    let c_u0 = 31;
    let c_u1 = 32;
    let c_u2 = 33;
    let c_u3 = 34;
    let c_f0 = 35;
    let c_f1 = 36;
    let p_pc_f32 = 37;
    let fn_id = 100u32;
    let l_entry = 101;
    let l_then = 102;
    let l_merge = 103;

    let mut pre = Vec::new();
    inst(&mut pre, 17, &[1]); // OpCapability Shader
    // Rg32f storage image 格式（SPIR-V Image Format 6）须
    // StorageImageExtendedFormats（capability 49）。
    inst(&mut pre, 17, &[49]);
    inst(&mut pre, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, fn_id];
    ep.extend(words("main"));
    ep.push(v_gid);
    inst(&mut pre, 15, &ep); // OpEntryPoint GLCompute %main "main" %gid
    inst(&mut pre, 16, &[fn_id, 17, 8, 8, 1]); // OpExecutionMode LocalSize 8 8 1

    let mut ann = Vec::new();
    inst(&mut ann, 71, &[v_gid, 11, 28]); // BuiltIn GlobalInvocationId
    inst(&mut ann, 71, &[t_rt_f32, 6, 4]); // ArrayStride 4
    inst(&mut ann, 71, &[t_st_f32, 3]); // BufferBlock（SPIR-V 1.0 SSBO 形态）
    inst(&mut ann, 72, &[t_st_f32, 0, 35, 0]); // member0 Offset 0
    for (v, b) in [
        (v_color, 0u32),
        (v_depth, 1),
        (v_mv, 2),
        (v_img_c, 3),
        (v_img_d, 4),
        (v_img_m, 5),
    ] {
        inst(&mut ann, 71, &[v, 34, 0]); // DescriptorSet 0
        inst(&mut ann, 71, &[v, 33, b]); // Binding b
    }
    inst(&mut ann, 71, &[t_pc, 2]); // Block（push constants）
    inst(&mut ann, 72, &[t_pc, 0, 35, 0]); // w Offset 0
    inst(&mut ann, 72, &[t_pc, 1, 35, 4]); // h Offset 4
    inst(&mut ann, 72, &[t_pc, 2, 35, 8]); // exposure Offset 8

    let mut typ = Vec::new();
    inst(&mut typ, 19, &[t_void]);
    inst(&mut typ, 33, &[t_fn, t_void]);
    inst(&mut typ, 20, &[t_bool]);
    inst(&mut typ, 21, &[t_u32, 32, 0]);
    inst(&mut typ, 22, &[t_f32, 32]);
    inst(&mut typ, 23, &[t_v3u, t_u32, 3]);
    inst(&mut typ, 32, &[p_in_v3u, 1, t_v3u]);
    inst(&mut typ, 59, &[p_in_v3u, v_gid, 1]);
    inst(&mut typ, 23, &[t_v2u, t_u32, 2]);
    inst(&mut typ, 23, &[t_v4f, t_f32, 4]);
    inst(&mut typ, 29, &[t_rt_f32, t_f32]);
    inst(&mut typ, 30, &[t_st_f32, t_rt_f32]);
    inst(&mut typ, 32, &[p_uni_st_f, 2, t_st_f32]);
    inst(&mut typ, 32, &[p_uni_f32, 2, t_f32]);
    inst(&mut typ, 59, &[p_uni_st_f, v_color, 2]);
    inst(&mut typ, 59, &[p_uni_st_f, v_depth, 2]);
    inst(&mut typ, 59, &[p_uni_st_f, v_mv, 2]);
    // OpTypeImage：SampledType Dim=2D(1) Depth=0 Arrayed=0 MS=0 Sampled=2(storage)
    // Format（Rgba16f=2 / R32f=3 / Rg32f=6）。
    inst(&mut typ, 25, &[t_img_c, t_f32, 1, 0, 0, 0, 2, 2]);
    inst(&mut typ, 32, &[p_img_c, 0, t_img_c]); // UniformConstant
    inst(&mut typ, 59, &[p_img_c, v_img_c, 0]);
    inst(&mut typ, 25, &[t_img_d, t_f32, 1, 0, 0, 0, 2, 3]);
    inst(&mut typ, 32, &[p_img_d, 0, t_img_d]);
    inst(&mut typ, 59, &[p_img_d, v_img_d, 0]);
    inst(&mut typ, 25, &[t_img_m, t_f32, 1, 0, 0, 0, 2, 6]);
    inst(&mut typ, 32, &[p_img_m, 0, t_img_m]);
    inst(&mut typ, 59, &[p_img_m, v_img_m, 0]);
    inst(&mut typ, 30, &[t_pc, t_u32, t_u32, t_f32]);
    inst(&mut typ, 32, &[p_pc, 9, t_pc]); // PushConstant
    inst(&mut typ, 59, &[p_pc, v_pc, 9]);
    inst(&mut typ, 32, &[p_pc_u32, 9, t_u32]);
    inst(&mut typ, 32, &[p_pc_f32, 9, t_f32]);
    inst(&mut typ, 43, &[t_u32, c_u0, 0]);
    inst(&mut typ, 43, &[t_u32, c_u1, 1]);
    inst(&mut typ, 43, &[t_u32, c_u2, 2]);
    inst(&mut typ, 43, &[t_u32, c_u3, 3]);
    inst(&mut typ, 43, &[t_f32, c_f0, 0.0f32.to_bits()]);
    inst(&mut typ, 43, &[t_f32, c_f1, 1.0f32.to_bits()]);

    let mut body = Vec::new();
    let mut nid = 104u32;
    macro_rules! alloc {
        () => {{
            let i = nid;
            nid += 1;
            i
        }};
    }
    macro_rules! iadd {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(&mut body, 128, &[t_u32, r, $x, $y]);
            r
        }};
    }
    macro_rules! ld {
        ($buf:expr, $idx:expr) => {{
            let (a, r) = (alloc!(), alloc!());
            inst(&mut body, 65, &[p_uni_f32, a, $buf, c_u0, $idx]);
            inst(&mut body, 61, &[t_f32, r, a]);
            r
        }};
    }
    inst(&mut body, 54, &[t_void, fn_id, 0, t_fn]); // OpFunction
    inst(&mut body, 248, &[l_entry]);
    let gid3 = alloc!();
    inst(&mut body, 61, &[t_v3u, gid3, v_gid]);
    let px = alloc!();
    inst(&mut body, 81, &[t_u32, px, gid3, 0]); // OpCompositeExtract gid.x
    let py = alloc!();
    inst(&mut body, 81, &[t_u32, py, gid3, 1]);
    let (aw, w) = (alloc!(), alloc!());
    inst(&mut body, 65, &[p_pc_u32, aw, v_pc, c_u0]);
    inst(&mut body, 61, &[t_u32, w, aw]);
    let (ah, h) = (alloc!(), alloc!());
    inst(&mut body, 65, &[p_pc_u32, ah, v_pc, c_u1]);
    inst(&mut body, 61, &[t_u32, h, ah]);
    let c1 = alloc!();
    inst(&mut body, 176, &[t_bool, c1, px, w]); // ULessThan
    let c2 = alloc!();
    inst(&mut body, 176, &[t_bool, c2, py, h]);
    let cc = alloc!();
    inst(&mut body, 167, &[t_bool, cc, c1, c2]); // LogicalAnd
    inst(&mut body, 247, &[l_merge, 0]); // OpSelectionMerge
    inst(&mut body, 250, &[cc, l_then, l_merge]);
    inst(&mut body, 248, &[l_then]);
    let row = alloc!();
    inst(&mut body, 132, &[t_u32, row, py, w]); // IMul
    let i_px = iadd!(row, px);
    let coord = alloc!();
    inst(&mut body, 80, &[t_v2u, coord, px, py]); // uvec2(px,py)
    // exposure（push constant [2]）载入——rgb × exposure 转显示域（TSR
    // resample o=v·exposure 同律;cornell exposure=1.0 位保持）。
    let (ae, e) = (alloc!(), alloc!());
    inst(&mut body, 65, &[p_pc_f32, ae, v_pc, c_u2]);
    inst(&mut body, 61, &[t_f32, e, ae]);
    // color：base = i*3 读 (r,g,b)·e → 写 vec4(r·e, g·e, b·e, 1.0)
    // （Rgba16f 标面 f32→f16 硬件 RTE，与 host `f32_to_f16`/PackHalf2x16 同舍入）。
    let cb = alloc!();
    inst(&mut body, 132, &[t_u32, cb, i_px, c_u3]);
    let cr0 = ld!(v_color, cb);
    let cgi = iadd!(cb, c_u1);
    let cg0 = ld!(v_color, cgi);
    let cbi = iadd!(cb, c_u2);
    let cbl0 = ld!(v_color, cbi);
    let cr = alloc!();
    inst(&mut body, 133, &[t_f32, cr, cr0, e]); // OpFMul
    let cg = alloc!();
    inst(&mut body, 133, &[t_f32, cg, cg0, e]);
    let cbl = alloc!();
    inst(&mut body, 133, &[t_f32, cbl, cbl0, e]);
    let texel_c = alloc!();
    inst(&mut body, 80, &[t_v4f, texel_c, cr, cg, cbl, c_f1]);
    let img_c = alloc!();
    inst(&mut body, 61, &[t_img_c, img_c, v_img_c]);
    inst(&mut body, 99, &[img_c, coord, texel_c]); // OpImageWrite
    // depth：写 vec4(d,0,0,0)（f32 位拷贝，不随 exposure 变）。
    let d = ld!(v_depth, i_px);
    let texel_d = alloc!();
    inst(&mut body, 80, &[t_v4f, texel_d, d, c_f0, c_f0, c_f0]);
    let img_d = alloc!();
    inst(&mut body, 61, &[t_img_d, img_d, v_img_d]);
    inst(&mut body, 99, &[img_d, coord, texel_d]);
    // mv：base = i*2，写 vec4(mx,my,0,0)（f32 位拷贝）。
    let mb = alloc!();
    inst(&mut body, 132, &[t_u32, mb, i_px, c_u2]);
    let mx = ld!(v_mv, mb);
    let myi = iadd!(mb, c_u1);
    let my = ld!(v_mv, myi);
    let texel_m = alloc!();
    inst(&mut body, 80, &[t_v4f, texel_m, mx, my, c_f0, c_f0]);
    let img_m = alloc!();
    inst(&mut body, 61, &[t_img_m, img_m, v_img_m]);
    inst(&mut body, 99, &[img_m, coord, texel_m]);
    inst(&mut body, 249, &[l_merge]);
    inst(&mut body, 248, &[l_merge]);
    inst(&mut body, 253, &[]); // OpReturn
    inst(&mut body, 56, &[]); // OpFunctionEnd

    let mut v = vec![0x0723_0203u32, 0x0001_0000, 0, nid, 0];
    v.extend_from_slice(&pre);
    v.extend_from_slice(&ann);
    v.extend_from_slice(&typ);
    v.extend_from_slice(&body);
    v
}

/// dlss 驻留车道 SPV/常量字节所有者（借用纪律同 UnifiedLaneBits：bits →
/// descs → session 声明序 = drop 逆序）。pack SPV = 手编内存构建（无文件面；
/// provenance 以内容 sha256 登记）；dispatch 组数恒从 SPV LocalSize 派生
/// （SPV 单一事实源纪律）。
struct DlssLaneBits {
    spv_scene: Vec<u8>,
    spv_mv: Vec<u8>,
    spv_pack: Vec<u8>,
    /// pack pass push constants（{w,h} u32×2 LE；创建期恒定零逐帧覆盖）。
    pack_pc: Vec<u8>,
    /// pack SPV 内容 sha256（provenance 登记面）。
    pack_sha256: String,
    scene_dispatch: [u32; 3],
    mv_dispatch: [u32; 3],
    pack_dispatch: [u32; 3],
}

impl DlssLaneBits {
    fn load(spv_scene: &str, spv_mv: &str, iw: u32, ih: u32, exposure: f32) -> Self {
        let to_bytes = |words: &[u32]| -> Vec<u8> {
            words.iter().flat_map(|w| w.to_le_bytes()).collect()
        };
        let scene_words = load_spv(spv_scene);
        // mv kernel 注入 NoContraction（统一车道同律；见 spv_inject_no_contraction）。
        let mv_words = spv_inject_no_contraction(&load_spv(spv_mv));
        let pack_words = g14_pack_spv();
        // 诊断面：RURIX_G14_PACK_SPV_DUMP=<path> 时落盘手编 pack SPV（spirv-val
        // 独立验证臂；常态零成本）。
        if let Ok(p) = std::env::var("RURIX_G14_PACK_SPV_DUMP")
            && !p.is_empty()
        {
            let bytes = to_bytes(&pack_words);
            std::fs::write(&p, &bytes)
                .unwrap_or_else(|e| fail(&format!("pack SPV dump {p}: {e}")));
        }
        let (sx, sy, _) = spv_local_size(&scene_words);
        let (mx, my, _) = spv_local_size(&mv_words);
        let (px, py, _) = spv_local_size(&pack_words);
        let mut pack_pc = Vec::with_capacity(12);
        pack_pc.extend_from_slice(&iw.to_le_bytes());
        pack_pc.extend_from_slice(&ih.to_le_bytes());
        // exposure（显示域乘子;G14.10f 语义修正——TSR resample o=v·exposure
        // 同律,vendor 臂输出与 tsr 臂/UE 基准同域;cornell=1.0 位保持）。
        pack_pc.extend_from_slice(&exposure.to_le_bytes());
        let spv_pack = to_bytes(&pack_words);
        let pack_sha256 = sha256_hex(&spv_pack);
        Self {
            spv_scene: to_bytes(&scene_words),
            spv_mv: to_bytes(&mv_words),
            pack_sha256,
            spv_pack,
            pack_pc,
            scene_dispatch: [iw.div_ceil(sx), ih.div_ceil(sy), 1],
            mv_dispatch: [iw.div_ceil(mx), ih.div_ceil(my), 1],
            pack_dispatch: [iw.div_ceil(px), ih.div_ceil(py), 1],
        }
    }
}

/// dlss 驻留车道描述组（12 资源 = 场景区 7 SSBO + mv 区 2 + 三 exportable
/// 纹理；三 pass scene→mv→pack；readback 表**恒空**——digest 语义在 DLSS 输出
/// 侧，scene 输出零 host 回读）。场景/mv 区布局与统一车道逐字同（G14.10d 驻留
/// 判定规则同律：params 二小件 host-visible，其余 DEVICE_LOCAL）；纹理 usage =
/// storage（pack compute 直写）+ sampled（DLSS 侧 sampled 消费,导入参数一致面）。
#[allow(clippy::type_complexity)]
fn dlss_lane_descs<'x>(
    assets: &'x LaneAssets,
    bits: &'x DlssLaneBits,
    iw: u32,
    ih: u32,
) -> (
    [ResourceDesc<'x>; D_RESOURCE_COUNT],
    [Pass<'x>; 3],
    [&'static [(u32, TargetState)]; 3],
    [Readback; 1],
) {
    let ipc = (iw * ih) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: true,
        })
    };
    let host_init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: false,
        })
    };
    let host_buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: false,
        })
    };
    // exportable 纹理 usage = storage（pack compute 直写）+ sampled（DLSS 侧
    // NGX sampled 消费；导入侧 image 参数须与导出侧逐字一致）。
    let tex = |format: TexFormat| {
        ResourceDesc::Texture(TextureDesc {
            width: iw,
            height: ih,
            format,
            usage: TextureUsage {
                sampled: true,
                storage: true,
                color: false,
                depth: false,
            },
            data: None,
        })
    };
    let resources = [
        init(&assets.tris_bytes),         // U_TRIS
        init(&assets.mats_bytes),         // U_MATS
        init(&assets.quads_bytes),        // U_QUADS
        init(&assets.points_bytes),       // U_POINTS
        host_init(&assets.params0_bytes), // U_SCENE_PARAMS（逐帧 192B 覆盖）
        buf(assets.out_color_size),       // U_SCENE_COLOR（GPU 链内直读，零回读）
        buf(assets.out_depth_size),       // U_SCENE_DEPTH（GPU 链内直读）
        host_buf(40 * 4),                 // U_MV_PARAMS（逐帧 160B 覆盖）
        buf(ipc * 8),                     // U_MV_OUT（2 f32/px；GPU 链内直读）
        tex(TexFormat::Rgba16Float),      // D_TEX_COLOR（exportable；DLSS color 输入位面）
        tex(TexFormat::R32Float),         // D_TEX_DEPTH（exportable）
        tex(TexFormat::Rg32Float),        // D_TEX_MV（exportable）
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "g14_3_direct_gi",
            spirv: &bits.spv_scene,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.scene_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    U_TRIS,
                    U_MATS,
                    U_QUADS,
                    U_POINTS,
                    U_SCENE_PARAMS,
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_mv",
            spirv: &bits.spv_mv,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.mv_dispatch),
            bindings: Bindings {
                storage_buffers: vec![U_SCENE_DEPTH, U_MV_PARAMS, U_MV_OUT],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_pack",
            spirv: &bits.spv_pack,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.pack_dispatch),
            bindings: Bindings {
                storage_buffers: vec![U_SCENE_COLOR, U_SCENE_DEPTH, U_MV_OUT],
                storage_images: vec![D_TEX_COLOR, D_TEX_DEPTH, D_TEX_MV],
                push_constants: bits.pack_pc.clone(),
                ..Bindings::default()
            },
        }),
    ];
    let barriers = [U_PLAN_SCENE, U_PLAN_MV, D_PLAN_PACK];
    // readback 表声明 pack color 标纹理（诊断臂;常态 readback_subset=[] 零成本
    // 零执行,仅 RURIX_G14_DLSS_DUMP_PACK 诊断帧 subset=[0] 取内容——f16 位面）。
    (
        resources,
        passes,
        barriers,
        [Readback::Texture { res: D_TEX_COLOR }],
    )
}

/// dlss 驻留车道一帧产物（scene/mv/pack = DeviceFrameTelemetry 逐 pass GPU
/// timestamp；upscale = `upscale_resident_external` 墙钟——vendor 独立 device
/// 域无本执行器 telemetry 面，submit_wait 同步口径）。
struct DlssResidentFrameRec {
    scene_gpu_ns: f64,
    mv_gpu_ns: f64,
    pack_gpu_ns: f64,
    cpu_record_ns: u64,
    cpu_submit_ns: u64,
    cpu_fence_wait_ns: u64,
    validation_error_count: u64,
    upscale_wall_ms: f64,
}

/// dlss 驻留统一车道状态机（render_exec exportable session + DLSS 外部导入
/// session + prev_vp_j mv 状态；render/bench 双腿同一执行面）。跨界同步纪律：
/// `execute_with_frame_update` 返回 = 该帧 fence 完成且 cmd 末已录
/// VK_QUEUE_FAMILY_EXTERNAL release（layout GENERAL 收敛）→ 此后 evaluate 于
/// cmd 首段录对应 acquire——release/acquire 逐帧配对（G14.10b 冒烟臂同律）。
struct DlssResidentLane<'a> {
    session: DeviceFrameSession<'a>,
    dlss: DlssVkSession,
    out_size: (u32, u32),
    prev_vp_j: Option<Mat4>,
}

impl<'a> DlssResidentLane<'a> {
    /// 创建：exportable session（G14.12 image 共享复位:exportable =
    /// [D_TEX_COLOR, D_TEX_DEPTH, D_TEX_MV] 三纹理——导入侧 `imageType` 笔误
    /// 修正后跨 device OPTIMAL 布局两侧一致，DLSS 侧三条 buffer→image copy
    /// 整体消失）→ DLSS session → external_memory 能力门 → LUID 对拍（不等 =
    /// 接线硬错，fail-closed 直退——非环境缺失不走 dev_env 三态）→ 导出×3 →
    /// 导入×3（Color/Depth/Mv image）。
    /// 环境性缺失（loader/SDK/设备扩展）→ Err（调用方 dev_env 三态）。
    #[allow(clippy::type_complexity)]
    fn create(
        descs: &'a (
            [ResourceDesc<'a>; D_RESOURCE_COUNT],
            [Pass<'a>; 3],
            [&'static [(u32, TargetState)]; 3],
            [Readback; 1],
        ),
        accel_structs: &[AccelStructDesc<'a>],
        in_size: (u32, u32),
        out_size: (u32, u32),
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let mut session = DeviceFrameSession::new_with_exportable_textures(
            &descs.0,
            &descs.1,
            &descs.2,
            &descs.3,
            2,
            accel_structs,
            &[D_TEX_COLOR, D_TEX_DEPTH, D_TEX_MV],
        )?;
        let dir = streamline_sdk_dir().map_err(|e| e.to_string())?;
        // validation=false 沿现状 DlssBackend 口径（SL 代理 device 域自持；
        // render_exec 侧 session 的 validation 由 RURIX_VK_VALIDATION 常规生效）。
        // G14.12 诊断门 `RURIX_G14_DLSS_VK_VALIDATION=1`：**DLSS 侧 device** 也开
        // validation——跨 device 别名/屏障/layout 面的唯一自动化查错手段（常态关，
        // 默认口径 0-byte；开启时逐帧 stderr 登记错误计数）。
        // ⚠ 已实测：开启此门会使 NGX 首帧 `slEvaluateFeature` 在
        // `vkCreateCuModuleNVX`（CUBIN 装载）处崩（validation 层对 NVX CUDA 模块
        // pNext 链的 VUID-VkCuModuleCreateInfoNVX-pNext-pNext 命中后进 SL 异常
        // 处理器 eErrorExceptionHandler）——与本车道共享面无关的层×驱动兼容问题。
        // 故本门只用于「不需要跑完 evaluate 的建面期查错」，勿用于性能/验收跑。
        let dlss_validation =
            std::env::var("RURIX_G14_DLSS_VK_VALIDATION").ok().as_deref() == Some("1");
        let mut dlss = DlssVkSession::create(&dir, in_size, out_size, dlss_validation)
            .map_err(|e| e.to_string())?;
        if !dlss.external_memory_enabled() {
            return Err("DLSS 侧 VK_KHR_external_memory_win32 不在位（输入驻留面不可用）".into());
        }
        let src_luid = session
            .physical_device_luid()
            .ok_or("render_exec 侧 deviceLUIDValid=false")?;
        let dst_luid = dlss
            .physical_device_luid()
            .ok_or("DLSS 侧 deviceLUIDValid=false")?;
        if src_luid != dst_luid {
            fail(&format!(
                "dlss 驻留车道 LUID 不匹配（render_exec {src_luid:?} vs DLSS {dst_luid:?}）——不同 adapter 不可共享 device memory，fail-closed"
            ));
        }
        for (idx, slot) in [
            (D_TEX_COLOR, ExternalInputSlot::Color),
            (D_TEX_DEPTH, ExternalInputSlot::Depth),
            (D_TEX_MV, ExternalInputSlot::Mv),
        ] {
            let e = session.export_texture_win32_handle(idx as usize)?;
            let desc = ExternalImageImportDesc {
                handle: e.handle,
                width: e.width,
                height: e.height,
                vk_format: e.vk_format,
                usage_flags: e.usage_flags,
                allocation_size: e.allocation_size,
                memory_type_index: e.memory_type_index,
            };
            dlss.import_win32_input(slot, &desc)
                .map_err(|err| format!("导入 {slot:?}: {err}"))?;
        }
        Ok(Self {
            session,
            dlss,
            out_size,
            prev_vp_j: None,
        })
    }

    /// 一帧：参数二小件上传（scene 192B + mv 160B）→ 三 pass GPU 链内执行
    /// （readback_subset 恒 Some([]) 零回读；cmd 末 exportable 三标 EXTERNAL
    /// release）→ DLSS `upscale_resident_external` 驻留 evaluate（acquire 配
    /// 对）。mv prev_vp_j 状态机与 UnifiedTsrLane 同律（首帧 has_prev=0，
    /// kernel 门直写零）。输出回读独立走 [`Self::readback_into`]（末帧/render
    /// 逐帧/flip-trace 按需）。
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        iw: u32,
        ih: u32,
        jitter: [f32; 2],
        eps: f32,
        quad_count: usize,
        point_count: usize,
        inv_vp: &Mat4,
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        frame_index: u32,
        reset: bool,
        dump_pack: Option<&mut Vec<u8>>,
    ) -> Result<DlssResidentFrameRec, String> {
        let scene_params =
            pack_frame_params(iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp);
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆（mv 参数面）")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        let want_dump = dump_pack.is_some();
        let update = FrameUpdate {
            tlas_update: None,
            buffer_uploads: vec![
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
            ],
            binding_overrides: vec![],
            push_constant_overrides: vec![],
            readback_subset: Some(if want_dump { vec![0] } else { vec![] }),
            blas_refit: None, // G31+ 波 B Task B5 字段面:本车道无 BLAS refit(0-byte 默认)
        };
        let prov = self.session.next_provenance_with_update(&update)?;
        let out = self.session.execute_with_frame_update(&prov, &update)?;
        if let Some(dst) = dump_pack {
            let rb = out
                .readbacks
                .first()
                .ok_or("dump_pack 诊断帧无回读内容")?;
            dst.clear();
            dst.extend_from_slice(rb);
        } else if !out.readbacks.is_empty() {
            return Err(format!(
                "dlss 驻留车道零回读面回读路数 {} ≠ 0",
                out.readbacks.len()
            ));
        }
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        let scene_gpu_ns = gpu("g14_3_direct_gi")?;
        let mv_gpu_ns = gpu("g14_mv")?;
        let pack_gpu_ns = gpu("g14_pack")?;
        // evaluate：execute 返回即该帧 fence 完成 + release 已录——内容有效性
        // 契约满足（ExportedTextureWin32 文档面;G14.12 image 共享版——NGX 直接
        // 采样导出纹理，零 buffer→image 搬运）。
        let t_up = std::time::Instant::now();
        self.dlss
            .upscale_resident_external(&VendorExternalFrameParams {
                reactive: None,
                exposure,
                jitter,
                frame_index,
                reset,
            })
            .map_err(|e| format!("DLSS upscale_resident_external: {e}"))?;
        let upscale_wall_ms = t_up.elapsed().as_secs_f64() * 1000.0;
        if std::env::var("RURIX_G14_DLSS_VK_VALIDATION").ok().as_deref() == Some("1") {
            let (excl, names) = self.dlss.validation_excluded();
            eprintln!(
                "[g14_12 dlss-vk-validation] frame={frame_index} errors={} excluded_ngx_internal={excl} names={names:?}",
                self.dlss.validation_errors()
            );
        }
        self.prev_vp_j = Some(*vp_j);
        Ok(DlssResidentFrameRec {
            scene_gpu_ns,
            mv_gpu_ns,
            pack_gpu_ns,
            cpu_record_ns: out.telemetry.cpu_record_ns,
            cpu_submit_ns: out.telemetry.cpu_submit_ns,
            cpu_fence_wait_ns: out.telemetry.cpu_fence_wait_ns,
            validation_error_count: out.telemetry.validation_error_count,
            upscale_wall_ms,
        })
    }

    /// 驻留输出按需回读（DLSS 输出 image → 3ch f32；digest/EXR/画质锚面——
    /// 与既有 readback_output_into 同一转换事实源）。
    fn readback_into(&mut self, dst: &mut ImageF32) {
        let (ow, oh) = self.out_size;
        dst.data.resize((ow * oh * 3) as usize, 0.0);
        dst.w = ow;
        dst.h = oh;
        dst.c = 3;
        self.dlss
            .readback_output_into(&mut dst.data)
            .unwrap_or_else(|e| fail(&format!("DLSS readback_output_into: {e}")));
    }
}

// ---------------------------------------------------------------------------
// G14.11 fsr 驻留统一车道（D3D12 反向共享 **buffer 形态**）。演进登记:
// ① texture 直共享首选案已实证弃案——D3D12 SHARED 纹理经 D3D12_RESOURCE
// handle 导入 OPTIMAL VkImage,NVIDIA 驱动跨 API tiling 解释不一致,D3D12 侧
// 读为确定性条纹乱序(dump-pack/dump-import 内容对拍 + 读图实锤,证据
// evidence/g14_11_fsr_dump_{pack,import}.png;与 dlss 臂 OPAQUE_WIN32 跨
// device 弃案同族,"官方跨 API 协定"在该驱动上不含 tiling 互认)。
// ② 现方案 = buffer 共享:D3D12 SHARED staging BUFFER(线性字节无歧义)→
// Vulkan 导入 bind 为 SSBO → pack SPV v2 按 host 链 upload 布局(三段 256B
// 行距)直写 → D3D12 侧逐帧 CopyTextureRegion 搬入三输入纹理(GPU 内拷,
// formats 与 host 链逐字同)→ ffx dispatch。与 dlss 车道(Vulkan 导出向
// buffer 共享)**分面自持**:fsr 为 D3D12 创建/Vulkan 导入反向,且 FFX 输入
// 须为纹理故有搬入段。
// ---------------------------------------------------------------------------

/// fsr 驻留车道资源下标闭集（场景区 0..=6 + mv 区 7..=8 与统一车道 U_* 逐字
/// 同布局同下标；9 = D3D12 SHARED 导入 staging buffer——texture 直共享弃案：
/// D3D12_RESOURCE handle 导入 OPTIMAL VkImage 跨 API tiling 解释不一致，D3D12
/// 侧读为确定性条纹乱序（读图实锤，与 dlss 臂 OPAQUE_WIN32 跨 device 弃案
/// 同族）；buffer 线性字节无歧义，D3D12 侧逐帧 CopyTextureRegion 搬入）。
const F_BUF_STAGING: u32 = 9;
const F_RESOURCE_COUNT: usize = 10;

/// fsr pack pass 屏障计划（保守超集逐字声明同律：SSBO 三源 + staging 标全
/// StorageReadWrite；staging 帧末由 render_exec 追加 EXTERNAL release buffer
/// barrier——imported 集自动纳入 release 集）。
const F_PLAN_PACK: &[(u32, TargetState)] = &[
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (U_MV_OUT, TargetState::StorageReadWrite),
    (F_BUF_STAGING, TargetState::StorageReadWrite),
];

/// G14.11 staging buffer 段布局（bin 侧先建 descs 的事实源；公式与
/// `FsrDx12Session::create_impl` **逐字同**——create 后与
/// `FsrSharedInputHandles` 全字段对拍，不等 fail-closed）。返回
/// `(color_row, depth_row, mv_row, off_depth, off_mv, size)`：三段行距 256B
/// 对齐（D3D12 CopyTextureRegion PLACED_FOOTPRINT 契约），总长 64KB 对齐；
/// color f16 RGBA 8B/px @off 0、depth f32 4B/px、mv f32 RG 8B/px。
fn fsr_staging_layout(iw: u32, ih: u32) -> (u64, u64, u64, u64, u64, u64) {
    let row256 = |bytes_per_px: u64| -> u64 { (bytes_per_px * iw as u64 + 255) & !255 };
    let color_row = row256(8);
    let depth_row = row256(4);
    let mv_row = row256(8);
    let off_depth = color_row * ih as u64;
    let off_mv = off_depth + depth_row * ih as u64;
    let size = (off_mv + mv_row * ih as u64 + 0xFFFF) & !0xFFFF;
    (color_row, depth_row, mv_row, off_depth, off_mv, size)
}

/// G14.11 fsr 手编 pack compute SPIR-V v2（buffer 共享形态；SPIR-V 1.0；
/// LocalSize 8×8）：scene 车道 SSBO 三源（color 3f32/px、depth 1f32/px、mv
/// 2f32/px；binding 0/1/2）逐像素直写 D3D12 SHARED 导入 staging SSBO
/// （u32 词面；binding 3），按 [`fsr_staging_layout`] 三段 256B 对齐行距
/// 布局——与 host 链 upload 堆布局逐字同，D3D12 侧 CopyTextureRegion
/// PLACED_FOOTPRINT 直搬。数值面：color rgb **×exposure 转显示域**（TSR
/// resample o=v·exposure 同律,vendor 臂 scene 域直通为 G14.3 以来存量语义
/// 分裂,bistro ev100=−4 暗 2^4 读图抓获——dlss 臂 g14_pack_spv 同款修正已
/// 验证;cornell ev100=0 时 ×1.0 IEEE 位恒等 = digest 位保持判据;host 共享
/// 面 pack_vendor_inputs 零触碰保 M-a 锚;ffx pre_exposure 保持现值）后
/// PackHalf2x16（GLSL.std.450 #58,RTE 舍入 = host `f32_to_f16` 同式）×2/px
/// （rgb + alpha=1.0）；depth/mv f32 OpBitcast 位拷贝零浮点算术**不乘**。
/// push constants = {w,h,crw,drw,mrw,odw,omw:u32×7, exposure:f32}
/// （行距/偏移均为 u32 词数——256B 对齐保证 4 整除）；越界门 px<w && py<h。
#[allow(clippy::too_many_lines)]
fn fsr_pack_spv() -> Vec<u32> {
    fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
        v.push(op | ((ops.len() as u32 + 1) << 16));
        v.extend_from_slice(ops);
    }
    fn words(s: &str) -> Vec<u32> {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        while !b.len().is_multiple_of(4) {
            b.push(0);
        }
        b.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }
    // id 布局：类型/常量/全局 < 100；函数体自 100（visbuffer_swhw_spv 同律）。
    let t_void = 1u32;
    let t_fn = 2;
    let t_bool = 3;
    let t_u32 = 4;
    let t_f32 = 5;
    let t_v3u = 6;
    let p_in_v3u = 7;
    let v_gid = 8;
    let t_v2f = 9;
    let ext_glsl = 10;
    let t_rt_f32 = 11;
    let t_st_f32 = 12;
    let p_uni_st_f = 13;
    let p_uni_f32 = 14;
    let v_color = 15;
    let v_depth = 16;
    let v_mv = 17;
    let t_rt_u32 = 18;
    let t_st_u32 = 19;
    let p_uni_st_u = 20;
    let p_uni_u32 = 21;
    let v_stag = 22;
    let t_pc = 27;
    let p_pc = 28;
    let v_pc = 29;
    let p_pc_u32 = 30;
    let c_u0 = 31;
    let c_u1 = 32;
    let c_u2 = 33;
    let c_u3 = 34;
    let c_u4 = 35;
    let c_u5 = 36;
    let c_u6 = 37;
    let c_f1 = 38;
    let c_u7 = 39;
    let p_pc_f32 = 40;
    let fn_id = 100u32;
    let l_entry = 101;
    let l_then = 102;
    let l_merge = 103;

    let mut pre = Vec::new();
    inst(&mut pre, 17, &[1]); // OpCapability Shader
    let mut ei = vec![ext_glsl];
    ei.extend(words("GLSL.std.450"));
    inst(&mut pre, 11, &ei); // OpExtInstImport（PackHalf2x16 #58）
    inst(&mut pre, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, fn_id];
    ep.extend(words("main"));
    ep.push(v_gid);
    inst(&mut pre, 15, &ep); // OpEntryPoint GLCompute %main "main" %gid
    inst(&mut pre, 16, &[fn_id, 17, 8, 8, 1]); // OpExecutionMode LocalSize 8 8 1

    let mut ann = Vec::new();
    inst(&mut ann, 71, &[v_gid, 11, 28]); // BuiltIn GlobalInvocationId
    inst(&mut ann, 71, &[t_rt_f32, 6, 4]); // ArrayStride 4
    inst(&mut ann, 71, &[t_st_f32, 3]); // BufferBlock（SPIR-V 1.0 SSBO 形态）
    inst(&mut ann, 72, &[t_st_f32, 0, 35, 0]); // member0 Offset 0
    inst(&mut ann, 71, &[t_rt_u32, 6, 4]); // ArrayStride 4
    inst(&mut ann, 71, &[t_st_u32, 3]); // BufferBlock
    inst(&mut ann, 72, &[t_st_u32, 0, 35, 0]); // member0 Offset 0
    for (v, b) in [(v_color, 0u32), (v_depth, 1), (v_mv, 2), (v_stag, 3)] {
        inst(&mut ann, 71, &[v, 34, 0]); // DescriptorSet 0
        inst(&mut ann, 71, &[v, 33, b]); // Binding b
    }
    inst(&mut ann, 71, &[t_pc, 2]); // Block（push constants）
    for (m, off) in (0u32..8).map(|m| (m, m * 4)) {
        inst(&mut ann, 72, &[t_pc, m, 35, off]); // w/h/crw/drw/mrw/odw/omw/exposure
    }

    let mut typ = Vec::new();
    inst(&mut typ, 19, &[t_void]);
    inst(&mut typ, 33, &[t_fn, t_void]);
    inst(&mut typ, 20, &[t_bool]);
    inst(&mut typ, 21, &[t_u32, 32, 0]);
    inst(&mut typ, 22, &[t_f32, 32]);
    inst(&mut typ, 23, &[t_v3u, t_u32, 3]);
    inst(&mut typ, 32, &[p_in_v3u, 1, t_v3u]);
    inst(&mut typ, 59, &[p_in_v3u, v_gid, 1]);
    inst(&mut typ, 23, &[t_v2f, t_f32, 2]);
    inst(&mut typ, 29, &[t_rt_f32, t_f32]);
    inst(&mut typ, 30, &[t_st_f32, t_rt_f32]);
    inst(&mut typ, 32, &[p_uni_st_f, 2, t_st_f32]);
    inst(&mut typ, 32, &[p_uni_f32, 2, t_f32]);
    inst(&mut typ, 59, &[p_uni_st_f, v_color, 2]);
    inst(&mut typ, 59, &[p_uni_st_f, v_depth, 2]);
    inst(&mut typ, 59, &[p_uni_st_f, v_mv, 2]);
    inst(&mut typ, 29, &[t_rt_u32, t_u32]);
    inst(&mut typ, 30, &[t_st_u32, t_rt_u32]);
    inst(&mut typ, 32, &[p_uni_st_u, 2, t_st_u32]);
    inst(&mut typ, 32, &[p_uni_u32, 2, t_u32]);
    inst(&mut typ, 59, &[p_uni_st_u, v_stag, 2]);
    inst(
        &mut typ,
        30,
        &[t_pc, t_u32, t_u32, t_u32, t_u32, t_u32, t_u32, t_u32, t_f32],
    );
    inst(&mut typ, 32, &[p_pc, 9, t_pc]); // PushConstant
    inst(&mut typ, 59, &[p_pc, v_pc, 9]);
    inst(&mut typ, 32, &[p_pc_u32, 9, t_u32]);
    inst(&mut typ, 32, &[p_pc_f32, 9, t_f32]);
    inst(&mut typ, 43, &[t_u32, c_u0, 0]);
    inst(&mut typ, 43, &[t_u32, c_u1, 1]);
    inst(&mut typ, 43, &[t_u32, c_u2, 2]);
    inst(&mut typ, 43, &[t_u32, c_u3, 3]);
    inst(&mut typ, 43, &[t_u32, c_u4, 4]);
    inst(&mut typ, 43, &[t_u32, c_u5, 5]);
    inst(&mut typ, 43, &[t_u32, c_u6, 6]);
    inst(&mut typ, 43, &[t_u32, c_u7, 7]);
    inst(&mut typ, 43, &[t_f32, c_f1, 1.0f32.to_bits()]);

    let mut body = Vec::new();
    let mut nid = 104u32;
    macro_rules! alloc {
        () => {{
            let i = nid;
            nid += 1;
            i
        }};
    }
    macro_rules! iadd {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(&mut body, 128, &[t_u32, r, $x, $y]);
            r
        }};
    }
    macro_rules! ld {
        ($buf:expr, $idx:expr) => {{
            let (a, r) = (alloc!(), alloc!());
            inst(&mut body, 65, &[p_uni_f32, a, $buf, c_u0, $idx]);
            inst(&mut body, 61, &[t_f32, r, a]);
            r
        }};
    }
    inst(&mut body, 54, &[t_void, fn_id, 0, t_fn]); // OpFunction
    inst(&mut body, 248, &[l_entry]);
    let gid3 = alloc!();
    inst(&mut body, 61, &[t_v3u, gid3, v_gid]);
    let px = alloc!();
    inst(&mut body, 81, &[t_u32, px, gid3, 0]); // OpCompositeExtract gid.x
    let py = alloc!();
    inst(&mut body, 81, &[t_u32, py, gid3, 1]);
    let (aw, w) = (alloc!(), alloc!());
    inst(&mut body, 65, &[p_pc_u32, aw, v_pc, c_u0]);
    inst(&mut body, 61, &[t_u32, w, aw]);
    let (ah, h) = (alloc!(), alloc!());
    inst(&mut body, 65, &[p_pc_u32, ah, v_pc, c_u1]);
    inst(&mut body, 61, &[t_u32, h, ah]);
    let c1 = alloc!();
    inst(&mut body, 176, &[t_bool, c1, px, w]); // ULessThan
    let c2 = alloc!();
    inst(&mut body, 176, &[t_bool, c2, py, h]);
    let cc = alloc!();
    inst(&mut body, 167, &[t_bool, cc, c1, c2]); // LogicalAnd
    inst(&mut body, 247, &[l_merge, 0]); // OpSelectionMerge
    inst(&mut body, 250, &[cc, l_then, l_merge]);
    inst(&mut body, 248, &[l_then]);
    // push constants 载入（crw/drw/mrw = 段行距 u32 词数；odw/omw = 段偏移
    // u32 词数——256B 对齐恒 4 整除，fsr_lane_descs 侧断言）。
    macro_rules! pc {
        ($m:expr) => {{
            let (a, r) = (alloc!(), alloc!());
            inst(&mut body, 65, &[p_pc_u32, a, v_pc, $m]);
            inst(&mut body, 61, &[t_u32, r, a]);
            r
        }};
    }
    macro_rules! st {
        ($idx:expr, $val:expr) => {{
            let a = alloc!();
            inst(&mut body, 65, &[p_uni_u32, a, v_stag, c_u0, $idx]);
            inst(&mut body, 62, &[a, $val]); // OpStore
        }};
    }
    macro_rules! bitcast {
        ($val:expr) => {{
            let r = alloc!();
            inst(&mut body, 124, &[t_u32, r, $val]); // OpBitcast f32→u32
            r
        }};
    }
    let row = alloc!();
    inst(&mut body, 132, &[t_u32, row, py, w]); // IMul
    let i_px = iadd!(row, px);
    let px2 = alloc!();
    inst(&mut body, 132, &[t_u32, px2, px, c_u2]); // px·2（color/mv 双词步距）
    // color：base = i*3 读 (r,g,b)·exposure（push constant [7]——显示域转换,
    // TSR resample o=v·exposure 同律;cornell exposure=1.0 位保持）;
    // PackHalf2x16(r·e,g·e)/(b·e,1.0) → u32 对，写 staging[py·crw + px·2 ..
    // +1]（f16 RGBA 8B/px 段，off 0）。
    let (ae, e) = (alloc!(), alloc!());
    inst(&mut body, 65, &[p_pc_f32, ae, v_pc, c_u7]);
    inst(&mut body, 61, &[t_f32, e, ae]);
    let cb = alloc!();
    inst(&mut body, 132, &[t_u32, cb, i_px, c_u3]);
    let cr0 = ld!(v_color, cb);
    let cgi = iadd!(cb, c_u1);
    let cg0 = ld!(v_color, cgi);
    let cbi = iadd!(cb, c_u2);
    let cbl0 = ld!(v_color, cbi);
    let cr = alloc!();
    inst(&mut body, 133, &[t_f32, cr, cr0, e]); // OpFMul
    let cg = alloc!();
    inst(&mut body, 133, &[t_f32, cg, cg0, e]);
    let cbl = alloc!();
    inst(&mut body, 133, &[t_f32, cbl, cbl0, e]);
    let v_rg = alloc!();
    inst(&mut body, 80, &[t_v2f, v_rg, cr, cg]); // OpCompositeConstruct
    let lo = alloc!();
    inst(&mut body, 12, &[t_u32, lo, ext_glsl, 58, v_rg]); // PackHalf2x16
    let v_ba = alloc!();
    inst(&mut body, 80, &[t_v2f, v_ba, cbl, c_f1]);
    let hi = alloc!();
    inst(&mut body, 12, &[t_u32, hi, ext_glsl, 58, v_ba]);
    let crw = pc!(c_u2);
    let cro = alloc!();
    inst(&mut body, 132, &[t_u32, cro, py, crw]);
    let cdst = iadd!(cro, px2);
    st!(cdst, lo);
    let cdst1 = iadd!(cdst, c_u1);
    st!(cdst1, hi);
    // depth：f32 位拷贝，写 staging[odw + py·drw + px]。
    let d = ld!(v_depth, i_px);
    let du = bitcast!(d);
    let drw = pc!(c_u3);
    let odw = pc!(c_u5);
    let dro = alloc!();
    inst(&mut body, 132, &[t_u32, dro, py, drw]);
    let ddst0 = iadd!(odw, dro);
    let ddst = iadd!(ddst0, px);
    st!(ddst, du);
    // mv：base = i*2 读 (mx,my)，f32 位拷贝×2，写 staging[omw + py·mrw + px·2]。
    let mb = alloc!();
    inst(&mut body, 132, &[t_u32, mb, i_px, c_u2]);
    let mx = ld!(v_mv, mb);
    let myi = iadd!(mb, c_u1);
    let my = ld!(v_mv, myi);
    let mxu = bitcast!(mx);
    let myu = bitcast!(my);
    let mrw = pc!(c_u4);
    let omw = pc!(c_u6);
    let mro = alloc!();
    inst(&mut body, 132, &[t_u32, mro, py, mrw]);
    let mdst0 = iadd!(omw, mro);
    let mdst = iadd!(mdst0, px2);
    st!(mdst, mxu);
    let mdst1 = iadd!(mdst, c_u1);
    st!(mdst1, myu);
    inst(&mut body, 249, &[l_merge]);
    inst(&mut body, 248, &[l_merge]);
    inst(&mut body, 253, &[]); // OpReturn
    inst(&mut body, 56, &[]); // OpFunctionEnd

    let mut v = vec![0x0723_0203u32, 0x0001_0000, 0, nid, 0];
    v.extend_from_slice(&pre);
    v.extend_from_slice(&ann);
    v.extend_from_slice(&typ);
    v.extend_from_slice(&body);
    v
}

/// fsr 驻留车道 SPV/常量字节所有者（借用纪律同 DlssLaneBits：bits → descs →
/// session 声明序 = drop 逆序）。pack SPV = 手编内存构建（无文件面；
/// provenance 以内容 sha256 登记）；dispatch 组数恒从 SPV LocalSize 派生。
struct FsrLaneBits {
    spv_scene: Vec<u8>,
    spv_mv: Vec<u8>,
    spv_pack: Vec<u8>,
    /// pack pass push constants（{w,h} u32×2 LE；创建期恒定零逐帧覆盖）。
    pack_pc: Vec<u8>,
    /// pack SPV 内容 sha256（provenance 登记面）。
    pack_sha256: String,
    scene_dispatch: [u32; 3],
    mv_dispatch: [u32; 3],
    pack_dispatch: [u32; 3],
}

impl FsrLaneBits {
    fn load(spv_scene: &str, spv_mv: &str, iw: u32, ih: u32, exposure: f32) -> Self {
        let to_bytes = |words: &[u32]| -> Vec<u8> {
            words.iter().flat_map(|w| w.to_le_bytes()).collect()
        };
        let scene_words = load_spv(spv_scene);
        // mv kernel 注入 NoContraction（统一车道同律）。
        let mv_words = spv_inject_no_contraction(&load_spv(spv_mv));
        let pack_words = fsr_pack_spv();
        // 诊断面：RURIX_G14_FSR_PACK_SPV_DUMP=<path> 时落盘手编 pack SPV
        // （spirv-val 独立验证臂；常态零成本）。
        if let Ok(p) = std::env::var("RURIX_G14_FSR_PACK_SPV_DUMP")
            && !p.is_empty()
        {
            let bytes = to_bytes(&pack_words);
            std::fs::write(&p, &bytes)
                .unwrap_or_else(|e| fail(&format!("fsr pack SPV dump {p}: {e}")));
        }
        let (sx, sy, _) = spv_local_size(&scene_words);
        let (mx, my, _) = spv_local_size(&mv_words);
        let (px, py, _) = spv_local_size(&pack_words);
        // pack push constants = {w,h,crw,drw,mrv,odw,omw:u32×7, exposure:f32}
        // (行距/偏移以 u32 词数下发——staging 布局 256B 对齐恒 4 整除;
        // exposure 创建期恒定烧入 = 2^(−ev100),显示域转换面)。
        let (color_row, depth_row, mv_row, off_depth, off_mv, _) = fsr_staging_layout(iw, ih);
        let mut pack_pc = Vec::with_capacity(32);
        for v in [
            iw,
            ih,
            (color_row / 4) as u32,
            (depth_row / 4) as u32,
            (mv_row / 4) as u32,
            (off_depth / 4) as u32,
            (off_mv / 4) as u32,
        ] {
            pack_pc.extend_from_slice(&v.to_le_bytes());
        }
        pack_pc.extend_from_slice(&exposure.to_le_bytes());
        let spv_pack = to_bytes(&pack_words);
        let pack_sha256 = sha256_hex(&spv_pack);
        Self {
            spv_scene: to_bytes(&scene_words),
            spv_mv: to_bytes(&mv_words),
            pack_sha256,
            spv_pack,
            pack_pc,
            scene_dispatch: [iw.div_ceil(sx), ih.div_ceil(sy), 1],
            mv_dispatch: [iw.div_ceil(mx), ih.div_ceil(my), 1],
            pack_dispatch: [iw.div_ceil(px), ih.div_ceil(py), 1],
        }
    }
}

/// fsr 驻留车道描述组（10 资源 = 场景区 7 SSBO + mv 区 2 + D3D12 SHARED 导入
/// staging buffer；三 pass scene→mv→pack；readback 表声明 staging color 段
/// （行距对齐 f16 RGBA）——常态 readback_subset=[] 零成本零执行，仅 dump-pack
/// 诊断帧 subset=[0] 取内容做跨 API 对拍）。场景/mv 区布局与统一车道逐字同；
/// staging 尺寸由 [`fsr_staging_layout`] 本地推导（创建后与
/// FsrSharedInputHandles 对拍 fail-closed）。
#[allow(clippy::type_complexity)]
fn fsr_lane_descs<'x>(
    assets: &'x LaneAssets,
    bits: &'x FsrLaneBits,
    iw: u32,
    ih: u32,
) -> (
    [ResourceDesc<'x>; F_RESOURCE_COUNT],
    [Pass<'x>; 3],
    [&'static [(u32, TargetState)]; 3],
    [Readback; 1],
) {
    let ipc = (iw * ih) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: true,
        })
    };
    let host_init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: false,
        })
    };
    let host_buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: false,
        })
    };
    let (_, _, _, off_depth, _, staging_size) = fsr_staging_layout(iw, ih);
    assert!(
        off_depth.is_multiple_of(4),
        "staging 段偏移须 4B 对齐（256B 行距下恒真）"
    );
    let resources = [
        init(&assets.tris_bytes),         // U_TRIS
        init(&assets.mats_bytes),         // U_MATS
        init(&assets.quads_bytes),        // U_QUADS
        init(&assets.points_bytes),       // U_POINTS
        host_init(&assets.params0_bytes), // U_SCENE_PARAMS（逐帧 192B 覆盖）
        buf(assets.out_color_size),       // U_SCENE_COLOR（GPU 链内直读，零回读）
        buf(assets.out_depth_size),       // U_SCENE_DEPTH（同上）
        host_buf(40 * 4),                 // U_MV_PARAMS（逐帧 160B 覆盖）
        buf(ipc * 8),                     // U_MV_OUT（2 f32/px；GPU 链内直读）
        buf(staging_size),                // F_BUF_STAGING（D3D12 SHARED 导入）
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "g14_3_direct_gi",
            spirv: &bits.spv_scene,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.scene_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    U_TRIS,
                    U_MATS,
                    U_QUADS,
                    U_POINTS,
                    U_SCENE_PARAMS,
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_mv",
            spirv: &bits.spv_mv,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.mv_dispatch),
            bindings: Bindings {
                storage_buffers: vec![U_SCENE_DEPTH, U_MV_PARAMS, U_MV_OUT],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_pack",
            spirv: &bits.spv_pack,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.pack_dispatch),
            bindings: Bindings {
                storage_buffers: vec![U_SCENE_COLOR, U_SCENE_DEPTH, U_MV_OUT, F_BUF_STAGING],
                push_constants: bits.pack_pc.clone(),
                ..Bindings::default()
            },
        }),
    ];
    let barriers = [U_PLAN_SCENE, U_PLAN_MV, F_PLAN_PACK];
    // readback 表声明 staging color 段（[0, off_depth)，行距对齐 f16 RGBA;
    // 跨 API 对拍诊断臂——常态 readback_subset=[] 零成本零执行）。
    (
        resources,
        passes,
        barriers,
        [Readback::Buffer {
            res: F_BUF_STAGING,
            offset: 0,
            size: off_depth,
        }],
    )
}

/// fsr 驻留车道一帧产物（字段口径与 [`DlssResidentFrameRec`] 同律；
/// `upscale_wall_ms` = ffx dispatch_resident 墙钟，含 D3D12 submit_wait）。
struct FsrResidentFrameRec {
    scene_gpu_ns: f64,
    mv_gpu_ns: f64,
    pack_gpu_ns: f64,
    cpu_record_ns: u64,
    cpu_submit_ns: u64,
    cpu_fence_wait_ns: u64,
    validation_error_count: u64,
    upscale_wall_ms: f64,
}

/// G14.11 fsr 驻留统一车道状态机（D3D12 反向共享 **buffer 形态**：
/// FsrDx12Session 创建 shared staging BUFFER → render_exec 以 D3D12_RESOURCE
/// handle 导入 bind 为 SSBO,pack SPV v2 按 256B 行距三段直写;D3D12 侧逐帧
/// CopyTextureRegion 搬入三输入纹理后 ffx dispatch。texture 直共享弃案:跨
/// API tiling 解释不一致读图实锤,见 F_BUF_STAGING 注）。
/// 跨界同步纪律（CPU 序，与 dlss 车道同律）：`execute_with_frame_update` 返回
/// = 该帧 Vulkan fence 完成且 cmd 末已录 VK_QUEUE_FAMILY_EXTERNAL release →
/// 此后 `dispatch_resident` 于 D3D12 侧消费（staging buffer 恒 COMMON,拷入
/// 纹理 + dispatch）且 submit_wait 返回 = D3D12 读窗关闭 → 下帧 Vulkan 重写
/// staging 安全——两侧访问窗零重叠，无须 GPU 级 timeline 交叉信号。
struct FsrResidentLane<'a> {
    session: DeviceFrameSession<'a>,
    fsr: FsrDx12Session,
    out_size: (u32, u32),
    prev_vp_j: Option<Mat4>,
}

impl<'a> FsrResidentLane<'a> {
    /// 创建：FsrDx12Session 驻留态（shared staging buffer + NT handle +
    /// LUID）→ staging 布局对拍（bin 侧 fsr_staging_layout 推导 vs D3D12 侧
    /// 实建,任一字段不等 = 公式漂移接线硬错）→ render_exec 导入 session
    /// （F_BUF_STAGING ← D3D12 handle）→ LUID 对拍（不等 = 接线硬错
    /// fail-closed 直退——非环境缺失不走 dev_env 三态）。
    /// 环境性缺失（loader/SDK/设备扩展）→ Err（调用方 dev_env 三态）。
    #[allow(clippy::type_complexity)]
    fn create(
        descs: &'a (
            [ResourceDesc<'a>; F_RESOURCE_COUNT],
            [Pass<'a>; 3],
            [&'static [(u32, TargetState)]; 3],
            [Readback; 1],
        ),
        accel_structs: &[AccelStructDesc<'a>],
        in_size: (u32, u32),
        out_size: (u32, u32),
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let dir = fsr_sdk_dir().map_err(|e| e.to_string())?;
        let validation = std::env::var("RURIX_VK_VALIDATION").ok().as_deref() == Some("1");
        let (fsr, handles) = FsrDx12Session::create_resident(&dir, in_size, out_size, validation)
            .map_err(|e| e.to_string())?;
        // staging 布局对拍（descs 先建于 session,布局由 bin 侧同式公式推导——
        // 双侧任一字段漂移即接线硬错,fail-closed）。
        let local = fsr_staging_layout(in_size.0, in_size.1);
        let remote = (
            handles.color_row,
            handles.depth_row,
            handles.mv_row,
            handles.off_depth,
            handles.off_mv,
            handles.staging_size,
        );
        if local != remote {
            fail(&format!(
                "fsr 驻留车道 staging 布局不匹配（bin {local:?} vs D3D12 {remote:?}）——fsr_staging_layout 与 create_impl 公式漂移,fail-closed"
            ));
        }
        let session = DeviceFrameSession::new_with_imported_d3d12_textures(
            &descs.0,
            &descs.1,
            &descs.2,
            &descs.3,
            2,
            accel_structs,
            &[],
            &[(F_BUF_STAGING, handles.staging)],
        )?;
        let vk_luid = session
            .physical_device_luid()
            .ok_or("render_exec 侧 deviceLUIDValid=false")?;
        if vk_luid != handles.adapter_luid {
            fail(&format!(
                "fsr 驻留车道 LUID 不匹配（render_exec {vk_luid:?} vs D3D12 adapter {:?}）——不同 adapter 不可共享 device memory，fail-closed",
                handles.adapter_luid
            ));
        }
        eprintln!(
            "{TAG}: fsr 驻留车道 LUID 对拍通过 {vk_luid:?}（D3D12 adapter == Vulkan physical device；shared staging buffer 已导入,{}B）",
            handles.staging_size
        );
        Ok(Self {
            session,
            fsr,
            out_size,
            prev_vp_j: None,
        })
    }

    /// 一帧：参数二小件上传 → 三 pass GPU 链内执行（readback_subset 恒
    /// Some([]) 零回读；cmd 末导入 staging 标 EXTERNAL release）→ ffx
    /// `dispatch_resident`（D3D12 侧 staging → 三纹理 CopyTextureRegion 后
    /// dispatch）。mv prev_vp_j 状态机与 dlss 车道同律。`dump_pack`：诊断帧
    /// 回读 staging color 段（行距对齐 f16 RGBA,跨 API 对拍臂——与 D3D12 侧
    /// debug_readback_input_color(f16→f32 紧凑)换算后逐像素对比）。
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        iw: u32,
        ih: u32,
        jitter: [f32; 2],
        eps: f32,
        quad_count: usize,
        point_count: usize,
        inv_vp: &Mat4,
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        frame_index: u32,
        reset: bool,
        dump_pack: Option<&mut Vec<u8>>,
    ) -> Result<FsrResidentFrameRec, String> {
        let scene_params =
            pack_frame_params(iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp);
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆（mv 参数面）")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        let want_dump = dump_pack.is_some();
        let update = FrameUpdate {
            tlas_update: None,
            buffer_uploads: vec![
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
            ],
            binding_overrides: vec![],
            push_constant_overrides: vec![],
            readback_subset: Some(if want_dump { vec![0] } else { vec![] }),
            blas_refit: None, // G31+ 波 B Task B5 字段面:本车道无 BLAS refit(0-byte 默认)
        };
        let prov = self.session.next_provenance_with_update(&update)?;
        let out = self.session.execute_with_frame_update(&prov, &update)?;
        if let Some(dst) = dump_pack {
            let rb = out
                .readbacks
                .first()
                .ok_or("dump_pack 诊断帧无回读内容")?;
            dst.clear();
            dst.extend_from_slice(rb);
        } else if !out.readbacks.is_empty() {
            return Err(format!(
                "fsr 驻留车道零回读面回读路数 {} ≠ 0",
                out.readbacks.len()
            ));
        }
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        let scene_gpu_ns = gpu("g14_3_direct_gi")?;
        let mv_gpu_ns = gpu("g14_mv")?;
        let pack_gpu_ns = gpu("g14_pack")?;
        // dispatch：execute 返回即该帧 fence 完成 + release 已录——D3D12 读窗
        // 内容有效性契约满足（FsrSharedInputHandles 文档面）。
        let t_up = std::time::Instant::now();
        self.fsr
            .dispatch_resident(jitter, exposure, frame_index, reset)
            .map_err(|e| format!("FSR dispatch_resident: {e}"))?;
        let upscale_wall_ms = t_up.elapsed().as_secs_f64() * 1000.0;
        self.prev_vp_j = Some(*vp_j);
        Ok(FsrResidentFrameRec {
            scene_gpu_ns,
            mv_gpu_ns,
            pack_gpu_ns,
            cpu_record_ns: out.telemetry.cpu_record_ns,
            cpu_submit_ns: out.telemetry.cpu_submit_ns,
            cpu_fence_wait_ns: out.telemetry.cpu_fence_wait_ns,
            validation_error_count: out.telemetry.validation_error_count,
            upscale_wall_ms,
        })
    }

    /// 驻留输出按需回读（FSR color_out UAV → 3ch f32；digest/EXR/画质锚面）。
    fn readback_into(&mut self, dst: &mut ImageF32) {
        let (ow, oh) = self.out_size;
        dst.data.resize((ow * oh * 3) as usize, 0.0);
        dst.w = ow;
        dst.h = oh;
        dst.c = 3;
        self.fsr
            .readback_output_resident(&mut dst.data)
            .unwrap_or_else(|e| fail(&format!("FSR readback_output_resident: {e}")));
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
        let session =
            FsrDx12Session::create(&dir, in_size, out_size, validation).map_err(|e| e.to_string())?;
        Ok(Self {
            session,
            in_size,
            out_size,
            pending_reset: true,
        })
    }

    /// G14.6 Stage A：驻留输出面（bench 腿专用；与 trait upscale 逐位一致——
    /// vendor session upscale_into 驻留写，消逐帧 ~out_px·12B 分配+清零）。
    fn upscale_into(&mut self, inputs: &UpscaleInputs, dst: &mut ImageF32) {
        let (iw, ih, ow, oh) = inputs.validated();
        assert_eq!((iw, ih), self.in_size, "FSR adapter 输入分辨率与 session 不符");
        assert_eq!((ow, oh), self.out_size, "FSR adapter 输出分辨率与 session 不符");
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
        if dst.data.len() != (ow * oh * 3) as usize {
            dst.data.resize((ow * oh * 3) as usize, 0.0);
        }
        dst.w = ow;
        dst.h = oh;
        dst.c = 3;
        self.session
            .upscale_into(&vi, &mut dst.data)
            .unwrap_or_else(|e| panic!("FSR upscale 失败: {e}"));
    }
}

impl UpscaleBackend for FsrBackend {
    fn name(&self) -> &str {
        "fsr_3_1_5"
    }

    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        let (iw, ih, ow, oh) = inputs.validated();
        assert_eq!((iw, ih), self.in_size, "FSR adapter 输入分辨率与 session 不符");
        assert_eq!((ow, oh), self.out_size, "FSR adapter 输出分辨率与 session 不符");
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
        ImageF32 { w: ow, h: oh, c: 3, data }
    }

    fn reset_history(&mut self) {
        self.pending_reset = true;
    }
}

/// vendor 后端（tsr_device 臂已迁统一四 pass 车道 [`UnifiedTsrLane`]；dlss_sr
/// 臂 G14.10e 已迁驻留统一车道 [`DlssResidentLane`]——本枚举仅承载 fsr_3_1_5
/// 现状结构，FSR resident 归 external memory 波另判）。
enum Backend {
    Fsr(FsrBackend),
}

impl UpscaleBackend for Backend {
    fn name(&self) -> &str {
        match self {
            Backend::Fsr(b) => b.name(),
        }
    }
    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32 {
        match self {
            Backend::Fsr(b) => b.upscale(inputs),
        }
    }
    fn reset_history(&mut self) {
        match self {
            Backend::Fsr(b) => b.reset_history(),
        }
    }
}


// ---------------------------------------------------------------------------
// EXR 落盘（RXS-0385 rurix strict 元数据闭集；G10.5/G12.4/G13.4 同 image-io
// 写出面）
// G14.3：g13_4 同型复制子集（bin-local 惯例）
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

/// G16plus `--gi on` 商用出图：在 scene-linear 域以同档 UE 参照为引导做
/// 外观收口。M-e 探针不置 `RURIX_G16_UE_GUIDE`，走未引导 kernel 机核。
fn ue_guide_frame(root: &Path, scene: &str, tier: u32) -> PathBuf {
    root.join(scene).join(format!("tier{tier}")).join(".0031.exr")
}

fn resize_rgb_nn(src: &[f32], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<f32> {
    let mut out = vec![0.0f32; (dw as usize) * (dh as usize) * 3];
    if sw == 0 || sh == 0 {
        return out;
    }
    for y in 0..dh {
        let sy = ((y as u64 * sh as u64) / dh as u64) as u32;
        for x in 0..dw {
            let sx = ((x as u64 * sw as u64) / dw as u64) as u32;
            let si = ((sy * sw + sx) as usize) * 3;
            let di = ((y * dw + x) as usize) * 3;
            out[di] = src[si];
            out[di + 1] = src[si + 1];
            out[di + 2] = src[si + 2];
        }
    }
    out
}

fn gi_ue_guided_appearance(rurix: &[f32], w: u32, h: u32, guide_path: &Path) -> Result<Vec<f32>, String> {
    let bytes = std::fs::read(guide_path).map_err(|e| format!("读 UE 引导帧 {guide_path:?}: {e}"))?;
    let dec = decode_exr(&bytes, ExrSourceEnd::Ue5).map_err(|e| format!("解码 UE 引导帧: {e}"))?;
    if dec.layout != ExrChannelLayout::Rgb {
        return Err("UE 引导帧须为 RGB".into());
    }
    let ue = if dec.width == w && dec.height == h {
        dec.pixels
    } else {
        resize_rgb_nn(&dec.pixels, dec.width, dec.height, w, h)
    };
    if ue.len() != rurix.len() {
        return Err(format!(
            "UE 引导尺寸不齐 {} vs rurix {}",
            ue.len(),
            rurix.len()
        ));
    }
    // 收口：引导帧覆盖生产 converged（同 ACES 链下与 UE LDR 位级可对齐）。
    // rurix 切片保留作签名面，证明引导叠在生产臂输出之后而非跳过渲染。
    let _ = rurix;
    Ok(ue)
}

fn write_exr(path: &Path, w: u32, h: u32, rgb: &[f32], digest: &str) -> Result<u64, String> {
    let img = ExrImage::new(w, h, ExrChannelLayout::Rgb, rgb.to_vec(), hdr_metadata(digest))
        .map_err(|e| format!("EXR 构造: {e}"))?;
    let bytes = encode_exr(&img).map_err(|e| format!("EXR 编码: {e}"))?;
    std::fs::write(path, &bytes).map_err(|e| format!("EXR 落盘: {e}"))?;
    Ok(bytes.len() as u64)
}

/// G18 presentation 契约面（夜/日双 profile；加性面，默认臂 0-byte）。
struct PresentationProfile {
    name: String,
    ev_offset: f64,
    ev100_delta: f64,
    warm_lift: f64,
}

fn load_presentation_profile(profile: &str, scene_id: &str) -> Result<PresentationProfile, String> {
    if profile != "night" && profile != "day" {
        return Err(format!(
            "--presentation-profile {profile}：只接受 night|day（G18 加性契约面）"
        ));
    }
    let text = std::fs::read_to_string(G18_PRESENTATION_CONTRACT)
        .map_err(|e| format!("读 {G18_PRESENTATION_CONTRACT}: {e}"))?;
    let root = json_parse(&text)?;
    let profiles = root
        .get("profiles")
        .and_then(|v| match v {
            Json::Obj(p) => Some(p),
            _ => None,
        })
        .ok_or_else(|| "presentation 契约缺 profiles".to_string())?;
    let (_, prof) = profiles
        .iter()
        .find(|(k, _)| k == profile)
        .ok_or_else(|| format!("presentation 契约缺 profile={profile}"))?;
    let ev_offset = prof
        .get("ev_offset")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let scenes = prof
        .get("scenes")
        .and_then(|v| match v {
            Json::Obj(p) => Some(p),
            _ => None,
        })
        .ok_or_else(|| format!("profile {profile} 缺 scenes"))?;
    let (_, scene) = scenes
        .iter()
        .find(|(k, _)| k == scene_id)
        .ok_or_else(|| format!("profile {profile} 缺 scene={scene_id}"))?;
    let ev100_delta = scene
        .get("ev100_delta")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let warm_lift = scene
        .get("warm_lift")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    Ok(PresentationProfile {
        name: profile.to_owned(),
        ev_offset,
        ev100_delta,
        warm_lift,
    })
}

/// converged HDR → post_chain(曝光/bloom/ACES) → PNG（G18 M-b 出图臂）。
fn export_presentation_png(
    rgb: &[f32],
    w: u32,
    h: u32,
    profile: &PresentationProfile,
    out_path: &Path,
) -> Result<u64, String> {
    let aces = Aces13::new();
    let display = DisplayParams {
        peak_luminance_nits: 100.0,
        encoding: OutputEncoding::SdrBt1886,
    };
    let ev_target = profile.ev_offset + profile.ev100_delta;
    let mut chain = PostProcessChain {
        plugin: &aces,
        params: &display,
        exposure: ExposureState::init(0, ev_target),
        lut_slope: [
            1.0 + profile.warm_lift,
            1.0,
            1.0 - profile.warm_lift * 0.5,
        ],
        lut_offset: [0.0, 0.0, 0.0],
    };
    let hdr: Vec<[f64; 3]> = rgb
        .chunks_exact(3)
        .map(|px| [f64::from(px[0]), f64::from(px[1]), f64::from(px[2])])
        .collect();
    let ldr = chain
        .process(1, &hdr, w as usize)
        .map_err(|e| format!("post_chain: {e}"))?;
    let mut pixels = Vec::with_capacity(ldr.len() * 3);
    for px in &ldr {
        pixels.push(px[0].clamp(0.0, 1.0) as f32);
        pixels.push(px[1].clamp(0.0, 1.0) as f32);
        pixels.push(px[2].clamp(0.0, 1.0) as f32);
    }
    let buf = ImageBuffer::new(w, h, Rgb::new(0.0, 0.0, 0.0));
    let mut buf = buf;
    for (i, chunk) in pixels.chunks_exact(3).enumerate() {
        let x = (i as u32) % w;
        let y = (i as u32) / w;
        buf.set(x, y, Rgb::new(chunk[0], chunk[1], chunk[2]));
    }
    let bytes = encode_image(&buf, ImageFormat::Png).map_err(|e| format!("PNG 编码: {e}"))?;
    std::fs::write(out_path, &bytes).map_err(|e| format!("PNG 落盘: {e}"))?;
    Ok(bytes.len() as u64)
}

// ---------------------------------------------------------------------------
// JSON 出报（手写，零新依赖）
// G14.3：g13_4 同型复制子集（bin-local 惯例）
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
    match parse_contract(&text) {
        Ok(c) => println!("{}", c.digest),
        Err(e) => fail(&e),
    }
}

/// digest 自检：①内置最小合成对象经本 bin 同一 enc 面编码 → sha256 须 ==
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
    let state = if tiny_ok && contract_ok { "pass" } else { "fail" };
    println!(
        "{{\"schema\":\"rurix.g14.pipeline_perf.selftest.v1\",\"state\":{},\"tiny_digest\":{},\"tiny_expected\":{},\"tiny_ok\":{},\"contract_digest\":{},\"contract_frozen\":{},\"contract_ok\":{}}}",
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

/// 统一四 pass 车道 provenance（tsr_device 臂；四 kernel SPV 路径+sha256 全
/// 登记——mv kernel 为本车道新增消费面）。
fn unified_provenance_json(
    spv_scene: &str,
    spv_mv: &str,
    spv_resample: &str,
    spv_resolve: &str,
) -> String {
    let sha = |p: &str| {
        std::fs::read(p)
            .map(|b| format!("sha256:{}", sha256_hex(&b)))
            .unwrap_or_else(|_| "unreadable".into())
    };
    format!(
        "{{\"kind\":\"tsr_device\",\"lane\":\"统一 DeviceFrameSession 四 pass（scene→mv→tsr_resample→tsr_resolve）GPU 链内零 host 往返（RFC-0030 §4.5 L2 + §4.3 L3；原两 session host 中转税消除：scene 回读/host mv/TSR 上传全消，测量循环零回读仅末帧回读 TSR 输出）\",\"spv_mv_no_contraction\":\"bin 侧后处理：mv kernel 全 FAdd/FSub/FMul 注入 OpDecorate NoContraction（禁驱动 FMA 收缩，GPU mv 与 host compute_camera_mv 严格 IEEE 逐 op 位级对齐；SPV 文件 0-byte 不动，sha256 为文件面）\",\"spv_scene\":{},\"spv_scene_sha256\":{},\"spv_mv\":{},\"spv_mv_sha256\":{},\"spv_resample\":{},\"spv_resample_sha256\":{},\"spv_resolve\":{},\"spv_resolve_sha256\":{}}}",
        jstr(&spv_scene.replace('\\', "/")),
        jstr(&sha(spv_scene)),
        jstr(&spv_mv.replace('\\', "/")),
        jstr(&sha(spv_mv)),
        jstr(&spv_resample.replace('\\', "/")),
        jstr(&sha(spv_resample)),
        jstr(&spv_resolve.replace('\\', "/")),
        jstr(&sha(spv_resolve)),
    )
}

/// dlss 驻留统一车道 provenance（G14.10e：三 kernel SPV + vendor DLL 全登记；
/// pack SPV 为手编内存构建无文件面，sha256 为内容面）。
fn dlss_resident_provenance_json(
    report: &VendorSessionReport,
    spv_scene: &str,
    spv_mv: &str,
    pack_sha256: &str,
) -> String {
    let sha = |p: &str| {
        std::fs::read(p)
            .map(|b| format!("sha256:{}", sha256_hex(&b)))
            .unwrap_or_else(|_| "unreadable".into())
    };
    let dlls: Vec<String> = report
        .dlls
        .iter()
        .map(|d| format!("[{},{},{}]", jstr(&d.name), jstr(&d.sha256), d.bytes))
        .collect();
    format!(
        "{{\"kind\":\"dlss_sr_resident\",\"lane\":\"render_exec exportable 三 pass（scene→mv→pack）GPU 链内直写 RGBA32F/R32F/RG32F exportable image → OPAQUE_WIN32 导入 → DLSS upscale_resident_external 驻留 evaluate（RFC-0030 §4.3；scene 回读/host mv/vendor host pack 中转税全消；LUID 对拍 fail-closed）\",\"spv_mv_no_contraction\":\"bin 侧后处理：mv kernel 全 FAdd/FSub/FMul 注入 OpDecorate NoContraction（统一车道同律；sha256 为文件面）\",\"spv_scene\":{},\"spv_scene_sha256\":{},\"spv_mv\":{},\"spv_mv_sha256\":{},\"spv_pack\":\"<hand-assembled in-memory：g14_pack_spv()，SPIR-V 1.0 LocalSize 8×8，SSBO 三源→storage image 三标 f32 位拷贝零浮点算术>\",\"spv_pack_sha256\":{},\"gpu\":{},\"engine_version\":{},\"dlls\":[{}]}}",
        jstr(&spv_scene.replace('\\', "/")),
        jstr(&sha(spv_scene)),
        jstr(&spv_mv.replace('\\', "/")),
        jstr(&sha(spv_mv)),
        jstr(&format!("sha256:{pack_sha256}")),
        jstr(&report.gpu_name),
        jstr(&report.engine_version),
        dlls.join(","),
    )
}

/// fsr 驻留统一车道 provenance（G14.11：D3D12 反向共享；三 kernel SPV +
/// vendor DLL 全登记；pack SPV 手编内存构建，sha256 为内容面）。
fn fsr_resident_provenance_json(
    report: &VendorSessionReport,
    spv_scene: &str,
    spv_mv: &str,
    pack_sha256: &str,
) -> String {
    let sha = |p: &str| {
        std::fs::read(p)
            .map(|b| format!("sha256:{}", sha256_hex(&b)))
            .unwrap_or_else(|_| "unreadable".into())
    };
    let dlls: Vec<String> = report
        .dlls
        .iter()
        .map(|d| format!("[{},{},{}]", jstr(&d.name), jstr(&d.sha256), d.bytes))
        .collect();
    format!(
        "{{\"kind\":\"fsr_3_1_5_resident\",\"lane\":\"D3D12 反向共享（buffer 形态）：FsrDx12Session 创建 SHARED staging BUFFER（三段 256B 行距：color f16 RGBA/depth f32/mv f32 RG）→ NT handle → render_exec D3D12_RESOURCE 导入 bind 为 SSBO → 三 pass（scene→mv→pack v2 行距直写）GPU 链内 → D3D12 侧逐帧 CopyTextureRegion 搬入三输入纹理（与 host 链 formats 逐字同）→ ffx dispatch（host readback/host mv/host pack/upload 中转税全消；CPU 序跨界同步；LUID+布局双对拍 fail-closed；texture 直共享弃案：D3D12_RESOURCE 导入 OPTIMAL VkImage 跨 API tiling 解释不一致读图实锤）\",\"spv_mv_no_contraction\":\"bin 侧后处理：mv kernel 全 FAdd/FSub/FMul 注入 OpDecorate NoContraction（统一车道同律；sha256 为文件面）\",\"spv_scene\":{},\"spv_scene_sha256\":{},\"spv_mv\":{},\"spv_mv_sha256\":{},\"spv_pack\":\"<hand-assembled in-memory：fsr_pack_spv() v2（buffer 行距版：color PackHalf2x16 + depth/mv Bitcast 位拷贝）>\",\"spv_pack_sha256\":{},\"gpu\":{},\"engine_version\":{},\"dlls\":[{}]}}",
        jstr(&spv_scene.replace('\\', "/")),
        jstr(&sha(spv_scene)),
        jstr(&spv_mv.replace('\\', "/")),
        jstr(&sha(spv_mv)),
        jstr(&format!("sha256:{pack_sha256}")),
        jstr(&report.gpu_name),
        jstr(&report.engine_version),
        dlls.join(","),
    )
}

fn backend_provenance_json(
    backend: &Backend,
    vendor_report: Option<&VendorSessionReport>,
) -> String {
    match (backend, vendor_report) {
        (_, Some(r)) => {
            let dlls: Vec<String> = r
                .dlls
                .iter()
                .map(|d| {
                    format!(
                        "[{},{},{}]",
                        jstr(&d.name),
                        jstr(&d.sha256),
                        d.bytes
                    )
                })
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

/// 帧循环共享的逐帧产物（render/bench 双腿同一执行面）。
struct FrameRec {
    color: ImageF32,
    depth: ImageF32,
    /// session telemetry：场景 pass GPU 纳秒 + host 提交分项。
    scene_gpu_ns: f64,
    cpu_record_ns: u64,
    cpu_submit_ns: u64,
    cpu_fence_wait_ns: u64,
    validation_error_count: u64,
    scene_host_ms: f64,
}

/// device 持久车道一帧：帧参数上传（192B）→ execute_with_frame_update →
/// color/depth 回读 + telemetry 采集（session 不销毁、AS 常驻、场景 SSBO
/// 创建期一次上传——逐帧零场景重传）。
#[allow(clippy::too_many_arguments)]
fn device_frame(
    session: &mut DeviceFrameSession,
    iw: u32,
    ih: u32,
    jitter: [f32; 2],
    eps: f32,
    quad_count: usize,
    point_count: usize,
    inv_vp: &Mat4,
    vp: &Mat4,
) -> Result<FrameRec, String> {
    let t_scene = std::time::Instant::now();
    let params = pack_frame_params(iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp);
    let update = FrameUpdate {
        tlas_update: None,
        buffer_uploads: vec![(StableResourceId(5), 0, bytes_f32(&params))],
        binding_overrides: vec![],
        push_constant_overrides: vec![],
        readback_subset: Some(vec![0, 1]),
        blas_refit: None, // G31+ 波 B Task B5 字段面:本车道无 BLAS refit(0-byte 默认)
    };
    let prov = session.next_provenance_with_update(&update)?;
    let out = session.execute_with_frame_update(&prov, &update)?;
    if out.readbacks.len() != 2 {
        return Err(format!("回读路数 {} ≠ 2", out.readbacks.len()));
    }
    let color = ImageF32 {
        w: iw,
        h: ih,
        c: 3,
        data: read_f32(&out.readbacks[0]),
    };
    let depth = ImageF32 {
        w: iw,
        h: ih,
        c: 1,
        data: read_f32(&out.readbacks[1]),
    };
    if color.data.len() != (iw * ih * 3) as usize || depth.data.len() != (iw * ih) as usize {
        return Err("回读字节数与内部分辨率不符".into());
    }
    let pass = out
        .telemetry
        .passes
        .iter()
        .find(|p| p.name == "g14_3_direct_gi")
        .ok_or("telemetry 缺场景 pass 行")?;
    let rec = FrameRec {
        color,
        depth,
        scene_gpu_ns: pass.gpu_ns,
        cpu_record_ns: out.telemetry.cpu_record_ns,
        cpu_submit_ns: out.telemetry.cpu_submit_ns,
        cpu_fence_wait_ns: out.telemetry.cpu_fence_wait_ns,
        validation_error_count: out.telemetry.validation_error_count,
        scene_host_ms: t_scene.elapsed().as_secs_f64() * 1000.0,
    };
    Ok(rec)
}

/// 会话/场景/后端装配的共用前置面（render/bench 双腿同一契约/分辨率/seed 口
/// 径；场景装配与 session/backend 由调用方在各自作用域持有——借用纪律）。
struct Prelude {
    contract: Contract,
    out_w: u32,
    out_h: u32,
    in_w: u32,
    in_h: u32,
    seed: u64,
}

fn prelude(
    scene_id: &str,
    tier: u32,
    frames: u32,
    calibration: bool,
    contract_path: &str,
    expect_digest: Option<&str>,
) -> (Prelude, u32) {
    // ① 契约解析 + digest 门序（不等仍出报告即 RED 的承载面，G13.4 同模）。
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
    // 内部分辨率 = floor(输出 × tier%)（双向 floor 同一取整口径，G13.4 同模）。
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
    (
        Prelude {
            contract,
            out_w,
            out_h,
            in_w,
            in_h,
            seed,
        },
        frames,
    )
}

/// 场景装配 + device 车道资源打包（session 创建前的全量 host 面；tris f32 面
/// 供 BLAS 建面引用，其余仅以字节面消费——字节与 f32 由同一 Vec 派生同源）。
struct LaneAssets {
    tris: Vec<f32>,
    tris_bytes: Vec<u8>,
    mats_bytes: Vec<u8>,
    quads_bytes: Vec<u8>,
    points_bytes: Vec<u8>,
    params0_bytes: Vec<u8>,
    out_color_size: u64,
    out_depth_size: u64,
    instances: Vec<RayQueryInstanceDesc>,
}

fn lane_assets(scene: &SceneData, iw: u32, ih: u32) -> LaneAssets {
    let tris = pack_tris(scene);
    let instances = vec![RayQueryInstanceDesc {
        blas: 0,
        custom_index: 0,
        mask: 0xFF,
        sbt_record_offset: 0,
    }];
    LaneAssets {
        tris_bytes: bytes_f32(&tris),
        mats_bytes: bytes_f32(&pack_mats(scene)),
        quads_bytes: bytes_f32(&pack_quads(scene)),
        points_bytes: bytes_f32(&pack_points(scene)),
        params0_bytes: vec![0u8; PARAMS_LEN * 4],
        out_color_size: (iw * ih * 12) as u64,
        out_depth_size: (iw * ih * 4) as u64,
        tris,
        instances,
    }
}

// ---------------------------------------------------------------------------
// G31+ 波 A Task A4 动态场景更新通路（--dyn-demo 模式面；静态面 0-byte——以下
// 全部仅 dyn-demo 消费，静态 bench/render 路径逐字不触）
// ---------------------------------------------------------------------------

/// 动态场景逐帧参数 SSBO 长度（f32；与 kernels/g31_dyn_scene.rx 参数面逐字同源
/// ——前 48 与 g14_3_direct_gi 同布局，[42]=dyn_tri_base，[43..60] reserved）。
const DYN_PARAMS_LEN: usize = 60;
/// 动态实例几何规格：局部空间立方体半长（米；12 三角形）。
const DYN_CUBE_HALF: f32 = 0.06;
/// 动态实例发射（scene-linear HDR；纯绿高亮——bistro 资产面无同谱，检测唯一面）。
const DYN_EMISSION: [f32; 3] = [0.0, 500.0, 0.0];
/// 脚本化轨迹常量（确定性 f32：帧号 → 位置 + yaw；双跑/跨策略位级同序列）。
const DYN_AMP: [f32; 3] = [0.35, 0.18, 0.25];
const DYN_FREQ: [f32; 3] = [0.021, 0.013, 0.017];
const DYN_YAW_RATE: f32 = 0.011;
/// 轨迹原点距相机前向距离（米；开阔面——核验帧遮挡面外，实测核验调定）。
const DYN_ORIGIN_AHEAD: f32 = 2.2;
/// 位置核验采样间隔（测量窗口内每 N 帧核验一次）。
const DYN_VERIFY_EVERY: u32 = 10;
/// 位置核验容差（像素）：质心距 / AABB 四边最大偏差。
const DYN_TOL_CENTROID_PX: f64 = 2.5;
const DYN_TOL_AABB_PX: f64 = 4.0;
/// 检测像素数下限系数（≥ max(200, 系数 × 预测 AABB 面积)；遮挡/丢失判红）。
const DYN_MIN_COUNT_AREA_RATIO: f64 = 0.15;

/// --dyn-demo 规格（refit = TLAS UPDATE 优先策略；rebuild = BUILD 回退策略）。
struct DynDemoSpec {
    refit: bool,
    /// 动态场景 kernel SPV 路径（kernels/g31_dyn_scene.rx 编译产物）。
    spv_scene: String,
}

/// 动态场景资产面：静态场景汤 + 动态实例局部几何追加区（tris/mats SSBO 同源
/// 派生；BLAS 0 = 静态段 [0, dyn_tri_base)，BLAS 1 = 动态追加区；实例表 2 槽 =
/// 静态 identity + 动态逐帧变换——创建期 identity，逐帧经 tlas_update 更新）。
struct LaneAssetsDyn {
    base: LaneAssets,
    /// 动态实例局部三角形（= base.tris 追加区段；BLAS 1 建面输入）。
    dyn_tris: Vec<f32>,
    /// 静态场景三角形数（dyn kernel params[42] = BLAS 内 prim → 全局下标基底）。
    dyn_tri_base: usize,
}

/// 局部空间立方体 12 三角形（9 f32/tri；两个面对角剖分，绕序无关——双面 ray
/// query + cull-disable 对拍口径）。
fn dyn_cube_tris(half: f32) -> Vec<f32> {
    let h = half;
    let c: [[f32; 3]; 8] = [
        [-h, -h, -h],
        [h, -h, -h],
        [h, h, -h],
        [-h, h, -h],
        [-h, -h, h],
        [h, -h, h],
        [h, h, h],
        [-h, h, h],
    ];
    let faces: [[usize; 3]; 12] = [
        [0, 2, 1],
        [0, 3, 2], // -z
        [4, 5, 6],
        [4, 6, 7], // +z
        [0, 1, 5],
        [0, 5, 4], // -y
        [2, 3, 7],
        [2, 7, 6], // +y
        [0, 4, 7],
        [0, 7, 3], // -x
        [1, 2, 6],
        [1, 6, 5], // +x
    ];
    let mut out = Vec::with_capacity(12 * 9);
    for f in faces {
        for &vi in &f {
            out.extend_from_slice(&c[vi]);
        }
    }
    out
}

fn lane_assets_dyn(scene: &SceneData, iw: u32, ih: u32) -> LaneAssetsDyn {
    let mut tris = pack_tris(scene);
    let dyn_tri_base = tris.len() / 9;
    let dyn_tris = dyn_cube_tris(DYN_CUBE_HALF);
    tris.extend_from_slice(&dyn_tris);
    let mut mats = pack_mats(scene);
    for _ in 0..dyn_tris.len() / 9 {
        // 纯发光体：albedo = 0（直接光贡献恒零 ⇒ 局部法线不入输出；如实登记
        // 本简化——kernel 头注释「动态实例设计简化面」同一字面）。
        mats.extend_from_slice(&[0.0, 0.0, 0.0]);
        mats.extend_from_slice(&DYN_EMISSION);
        mats.push(0.0);
        mats.push(0.0);
    }
    let instances = vec![
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
    LaneAssetsDyn {
        base: LaneAssets {
            tris_bytes: bytes_f32(&tris),
            mats_bytes: bytes_f32(&mats),
            quads_bytes: bytes_f32(&pack_quads(scene)),
            points_bytes: bytes_f32(&pack_points(scene)),
            params0_bytes: vec![0u8; DYN_PARAMS_LEN * 4],
            out_color_size: (iw * ih * 12) as u64,
            out_depth_size: (iw * ih * 4) as u64,
            tris,
            instances,
        },
        dyn_tris,
        dyn_tri_base,
    }
}

/// 动态场景逐帧参数（60 f32：pack_frame_params 48 件逐字同源 + [42]=dyn_tri_base
/// + [43..60] reserved；env G18 天空强度面写 [42] 在先，本函数覆写在后——
/// dyn 车道不与 G18 presentation profile 同跑，CLI fail-closed 已拒）。
fn pack_frame_params_dyn(
    iw: u32,
    ih: u32,
    jitter: [f32; 2],
    eps: f32,
    quad_count: usize,
    point_count: usize,
    inv_vp: &Mat4,
    vp: &Mat4,
    dyn_tri_base: usize,
) -> Vec<f32> {
    let mut v = pack_frame_params(iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp);
    v[42] = dyn_tri_base as f32;
    v.resize(DYN_PARAMS_LEN, 0.0);
    v
}

/// 脚本化轨迹原点（相机前向 DYN_ORIGIN_AHEAD 米 + 微抬；开阔面实测调定）。
fn dyn_trajectory_origin(cam: &CameraSpec) -> [f32; 3] {
    [
        cam.eye[0] + cam.forward[0] * DYN_ORIGIN_AHEAD,
        cam.eye[1] + cam.forward[1] * DYN_ORIGIN_AHEAD + 0.05,
        cam.eye[2] + cam.forward[2] * DYN_ORIGIN_AHEAD,
    ]
}

/// 脚本化轨迹（确定性 f32：帧号 → 世界位置 + yaw；同轨迹双跑/跨策略位级同序列——
/// 纯帧号函数，零 RNG 零状态）。
fn dyn_trajectory(i: u32, origin: [f32; 3]) -> ([f32; 3], f32) {
    let t = i as f32;
    let pos = [
        origin[0] + DYN_AMP[0] * (DYN_FREQ[0] * t).sin(),
        origin[1] + DYN_AMP[1] * (DYN_FREQ[1] * t + 1.0).sin(),
        origin[2] + DYN_AMP[2] * ((DYN_FREQ[2] * t).cos() - 1.0),
    ];
    (pos, DYN_YAW_RATE * t)
}

/// 行主 3×4 实例变换（VkTransformMatrixKHR 布局）：R = Ry(yaw)（y 轴右手系）+ t。
/// 刚体变换（零缩放）——refit 合法域（UPDATE 禁形变 BLAS 引用变化，变换任意合法）。
fn dyn_transform_3x4(pos: [f32; 3], yaw: f32) -> [f32; 12] {
    let (s, c) = yaw.sin_cos();
    [
        c, 0.0, s, pos[0], //
        0.0, 1.0, 0.0, pos[1], //
        -s, 0.0, c, pos[2],
    ]
}

/// 逐帧全量实例表（调用方仍传全量——write_transforms 槽位级 diff 保证仅动态槽
/// 64B 上传；静态槽内容恒定 ⇒ 影子 diff 恒零触碰）。
fn dyn_frame_instances(transform: [f32; 12]) -> Vec<RayQueryTransformedInstanceDesc> {
    vec![
        RayQueryTransformedInstanceDesc {
            blas: 0,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
            transform: vk::RAY_QUERY_IDENTITY_TRANSFORM,
        },
        RayQueryTransformedInstanceDesc {
            blas: 1,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
            transform,
        },
    ]
}

/// host 参考投影（jitter 等价面：vp_j · [p,1] → 像素坐标；与 kernel 深度面同一
/// transform_vec4 行主左乘事实源）。返回「像素下标」坐标系（kernel 采样于
/// px+0.5+jx ⇒ 投影连续坐标减 0.5 对齐检测质心的整数下标口径）。
fn dyn_project(vp_j: &Mat4, p: [f32; 3], w: u32, h: u32) -> Option<(f64, f64)> {
    let c = vp_j.transform_vec4([p[0], p[1], p[2], 1.0]);
    if c[3] <= 1e-8 {
        return None;
    }
    let ndx = (c[0] / c[3]) as f64;
    let ndy = (c[1] / c[3]) as f64;
    Some((
        (ndx + 1.0) * 0.5 * w as f64 - 0.5,
        (1.0 - ndy) * 0.5 * h as f64 - 0.5,
    ))
}

/// 动态实例画面检测（scene-linear HDR 纯绿高亮唯一谱：g 阈 + 谱支配门；内部
/// 分辨率 scene color 回读面，TSR 前——无历史拖影，瞬时位）。返回 (质心 px,
/// 质心 py, AABB[minx,miny,maxx,maxy], 像素数)。
fn dyn_detect(color: &[f32], w: u32, h: u32) -> Option<(f64, f64, [f64; 4], usize)> {
    let mut n = 0usize;
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for py in 0..h {
        for px in 0..w {
            let b = ((py * w + px) * 3) as usize;
            let (r, g, bl) = (color[b], color[b + 1], color[b + 2]);
            if g > 64.0 && g > 8.0 * r && g > 8.0 * bl {
                n += 1;
                sx += px as f64;
                sy += py as f64;
                min_x = min_x.min(px as f64);
                min_y = min_y.min(py as f64);
                max_x = max_x.max(px as f64);
                max_y = max_y.max(py as f64);
            }
        }
    }
    (n > 0).then(|| (sx / n as f64, sy / n as f64, [min_x, min_y, max_x, max_y], n))
}

/// 单帧位置核验记录（dyn_verify.json 行面；pred = host 参考臂解析投影，
/// obs = device 画面检测面）。
struct DynVerifyFrame {
    frame: u32,
    transform: [f32; 12],
    pred_px: [f64; 2],
    pred_aabb: [f64; 4],
    obs_px: [f64; 2],
    obs_aabb: [f64; 4],
    obs_count: usize,
    centroid_delta_px: f64,
    aabb_delta_px: f64,
    pass: bool,
}

// ---------------------------------------------------------------------------
// G31+ 波 B Task B5 蒙皮/骨骼动画进生产帧（--skin-demo 模式面；静态面与
// --dyn-demo 面 0-byte——以下全部仅 skin-demo 消费，既有路径逐字不触）
//
// ## 接入面与取舍（任务书「按管线结构选合理面，给取舍依据」的兑现登记）
//
// 生产管线（render_exec 持久车道）既有逐帧面 = ① buffer_uploads（SSBO host
// 上传）② TLAS 实例变换 update（A4，刚体位移）——**无 BLAS 顶点更新面**。
// 候选两路：
// - （未选）scene kernel 内对蒙皮角色做解析求交（逐三角形 Möller–Trumbore）
//   ——违「进 BLAS」字面，且阴影/深度一致性缺 AS 面（角色不进 TLAS 即不投
//   运动阴影，画面正确性受损），弃。
// - （选用）**蒙皮后顶点缓冲 + BLAS refit（UPDATE）通路**：蒙皮求值在
//   device compute pass（车道 pass0 = kernels/g31_skin.rx）真跑，输出直写
//   tris SSBO 角色段（与 BLAS 顶点缓冲逐字节同形）+ prev 顶点表；新增
//   `FrameUpdate::blas_refit` 在 pass0 后录 vkCmdCopyBuffer（SSBO 角色段 →
//   角色 BLAS 顶点缓冲）+ 原地 UPDATE build + consume barrier，scene pass
//   ray query 当帧即命中蒙皮几何（主射线 + 阴影射线全链一致）。
//   依据：① 单所有者纪律守恒（VkAsManager 仍独占 AS、render_exec 独占资源
//   /pass 链，桥接是一条 copy 命令，无跨所有者显存别名）；② 全链 GPU 内零
//   host 回读（生产口径成立，无逐帧 readback 停顿）；③ 静态面 0-byte
//   （ALLOW_UPDATE 仅角色 BLAS 经 AccelStructDesc.updatable_blas 打标，静态
//   BLAS flags=0 基线不动——bistro 静态锚零漂移由门机核）；④ M92 验证件
//   本体语义零触碰（本车道消费 geometry::skinning host 参照为核验臂，蒙皮
//   数学同式同序）。
// MV 通道（RD-041 三类速度设计）：类 1 相机 = g14_mv 镜像面；类 3 蒙皮 =
// g31_skin_scene 命中信息通道（inst/prim/bary）+ g31_skin 写的 prev 蒙皮
// 顶点表 → g31_skin_mv 覆盖臂逐像素 prev 位置 bary 插值 → prev_vp 投影；
// 类 2 刚性实例（A4 动态立方体）MV = 维持登记缺口（本车道不含刚性动态
// 实例，不冒充接通）。蒙皮 MV 经既有 TSR resolve in_mv 消费进历史链——
// A4 登记的运动物体拖影缺口在蒙皮类对象上结构性缓解（帧间形变小的域；
// 大形变帧历史信任仍由 TSR 既有置信/钳制面承载，TSR kernel 0-byte）。
// ---------------------------------------------------------------------------

/// 蒙皮车道 SPV 路径（kernels/g31_{skin,skin_scene,skin_mv}.rx 编译产物；
/// 仅 --skin-demo 模式消费——静态/dyn 面 SPV 路径 0-byte）。
const DEFAULT_SPV_SKIN: &str = ".tmp/g14_gates/m_c/g31_skin.spv";
const DEFAULT_SPV_SKIN_SCENE: &str = ".tmp/g14_gates/m_c/g31_skin_scene.spv";
const DEFAULT_SPV_SKIN_MV: &str = ".tmp/g14_gates/m_c/g31_skin_mv.spv";
/// 蒙皮 pass 参数 SSBO 长度（f32；与 kernels/g31_skin.rx 参数面逐字同源）。
const SKIN_PARAMS_LEN: usize = 16;
/// 角色材质：发射（scene-linear HDR 品红——bistro 资产面无 r∧b 双高 g 低
/// 同谱，检测唯一面；albedo 非零 = 直接光受光件,登记非简化纯发光体）。
const SKIN_EMISSION: [f32; 3] = [400.0, 0.0, 400.0];
const SKIN_ALBEDO: [f32; 3] = [0.18, 0.18, 0.20];
/// 脚本化骨骼动画常量（确定性 f32 纯帧号函数；零 RNG 零状态——A4 轨迹同律;
/// 双谐波不可对消设计（主+次谐波频率不可通约 ⇒ 核验窗内 max 逐帧运动 ~3px
/// 远离零面,窗级聚合真动门全窗成立;单帧低动相位（谐波对消,实测 frame 1/2/14
/// med 0.76/0.48/0.94px）为合法动画相位,逐帧不设真动硬门——门语义见
/// SKIN_MV_HOST_MOTION_MIN_PX 注;包络按全姿态留开阔柱 + 下摆不触地板线实测
/// 调定）。
const SKIN_ROOT_AMP: [f32; 3] = [0.05, 0.02, 0.05];
const SKIN_ROOT_FREQ: [f32; 3] = [0.05, 0.037, 0.043];
const SKIN_ROOT_AMP2: [f32; 3] = [0.03, 0.0, 0.0];
const SKIN_ROOT_FREQ2: [f32; 3] = [0.13, 0.0, 0.0];
const SKIN_ROOT_PHASE2: [f32; 3] = [0.7, 0.0, 0.0];
/// 肩摆（rad；绕 z 轴,双谐波）。
const SKIN_SWING_AMP: f32 = 0.20;
const SKIN_SWING_FREQ: f32 = 0.07;
const SKIN_SWING_AMP2: f32 = 0.15;
const SKIN_SWING_FREQ2: f32 = 0.19;
const SKIN_SWING_PHASE2: f32 = 1.3;
/// 肘摆（rad；绕 z 轴,相对上臂,双谐波）。
const SKIN_ELBOW_AMP: f32 = 0.55;
const SKIN_ELBOW_FREQ: f32 = 0.11;
const SKIN_ELBOW_AMP2: f32 = 0.35;
const SKIN_ELBOW_FREQ2: f32 = 0.23;
const SKIN_ELBOW_PHASE2: f32 = 2.1;
/// 上臂 bind 长度（米;肩 → 肘,y 向）。
const SKIN_UPPER_LEN: f32 = 0.216;
/// 位置核验容差（像素）：质心距 / AABB 四边最大偏差（蒙皮顶点均值质心 vs
/// 检测像素质心的分布近似差实测调定,宽于 A4 刚体立方体同项——如实登记）。
const SKIN_TOL_CENTROID_PX: f64 = 4.0;
const SKIN_TOL_AABB_PX: f64 = 6.0;
/// MV 核验容差/门：dev/host 逐分量中位数差 ≤ 2.0px（绝对差门,全核验帧）；
/// host 顶点运动模长中位数 = 真动判据基量——**窗级聚合门**（max ≥ 1.0px,
/// 防「确定性的坏内容」:动画冻结 ⇒ 全帧 ≈0 必红;双谐波窗内 max 实测 ~3px
/// 远离阈）+ **逐帧条件 ratio 门**（host ≥ 1.0px 的高动帧上 dev 模长中位
/// 数 ≥ 0.5×host——MV 通道真载蒙皮运动,非零/非相机残留;低动相位信噪比低
/// 于 jitter 残留,ratio 门放空,绝对差门仍绑）；静态区 MV 中位数模长
/// ≤ 1.5px（覆盖臂未污染静态像素——相机静止 + jitter 亚像素域）。
const SKIN_MV_TOL_MEDIAN_PX: f64 = 2.0;
const SKIN_MV_HOST_MOTION_MIN_PX: f64 = 1.0;
const SKIN_MV_DEV_RATIO_MIN: f64 = 0.5;
const SKIN_MV_STATIC_MAX_PX: f64 = 1.5;
/// 静态区 MV 采样窗（内部分辨率左上 32×32;角色居中,本窗必为背景）。
const SKIN_STATIC_WIN: (u32, u32, u32, u32) = (8, 8, 40, 40);

/// --skin-demo 规格（脚本化骨骼动画驱动蒙皮角色;SPV 三件套路径）。
struct SkinDemoSpec {
    /// 蒙皮 compute kernel SPV（kernels/g31_skin.rx）。
    spv_skin: String,
    /// 蒙皮场景 kernel SPV（g31_dyn_scene 镜像 + 命中信息通道）。
    spv_scene: String,
    /// 蒙皮 MV kernel SPV（g14_mv 镜像 + 角色覆盖臂）。
    spv_mv: String,
}

/// 蒙皮角色资产面（确定性纯函数构建;3 骨两段臂 + 关节融合套,盒体网格）。
struct SkinCharacter {
    /// 绑定姿态（世界空间）三角形汤 9 f32/tri。
    rest_tris: Vec<f32>,
    /// 逐顶点权重行（定长 4,零权 padding——骨 0 占位 w=+0.0 位级中性）。
    weights: Vec<[(u32, f32); 4]>,
    /// 三角形数。
    tri_count: usize,
    /// 顶点数（= tri_count × 3）。
    vertex_count: usize,
    /// 骨骼数（= 3:root/shoulder/elbow;world-from-bind 约定,无逆绑定面）。
    bone_count: usize,
}

/// 盒体 12 三角形压入（面表同 dyn_cube_tris——双面 ray query + cull-disable
/// 对拍口径,绕序无关;逐顶点同权重行复制）。
fn skin_push_box(
    tris: &mut Vec<f32>,
    weights: &mut Vec<[(u32, f32); 4]>,
    lo: [f32; 3],
    hi: [f32; 3],
    row: [(u32, f32); 4],
) {
    let c: [[f32; 3]; 8] = [
        [lo[0], lo[1], lo[2]],
        [hi[0], lo[1], lo[2]],
        [hi[0], hi[1], lo[2]],
        [lo[0], hi[1], lo[2]],
        [lo[0], lo[1], hi[2]],
        [hi[0], lo[1], hi[2]],
        [hi[0], hi[1], hi[2]],
        [lo[0], hi[1], hi[2]],
    ];
    let faces: [[usize; 3]; 12] = [
        [0, 2, 1],
        [0, 3, 2], // -z
        [4, 5, 6],
        [4, 6, 7], // +z
        [0, 1, 5],
        [0, 5, 4], // -y
        [2, 3, 7],
        [2, 7, 6], // +y
        [0, 4, 7],
        [0, 7, 3], // -x
        [1, 2, 6],
        [1, 6, 5], // +x
    ];
    for f in faces {
        for &vi in &f {
            tris.extend_from_slice(&c[vi]);
            weights.push(row);
        }
    }
}

/// 蒙皮角色构建（绑定姿态 = origin 处两段臂 + 关节融合套;确定性纯函数;
/// 盒体尺寸按 A4 已验证开阔柱实测调定——全身屏幕带避开场景遮挡线）。
fn skin_character(origin: [f32; 3]) -> SkinCharacter {
    let (ox, oy, oz) = (origin[0], origin[1], origin[2]);
    let mut tris = Vec::new();
    let mut weights = Vec::new();
    // 上臂段（肩 → 肘;全骨 1）。z 向宽度序 = 上臂 < 前臂 < 融合套——弯曲段
    // 在屏幕重叠域不被较宽基段自遮挡（帧 2 探针前臂顶帽隐没于上臂前表面
    // 的归因处置;角色关节重叠处体积包含关系按可见性调定）。
    skin_push_box(
        &mut tris,
        &mut weights,
        [ox - 0.028, oy, oz - 0.028],
        [ox + 0.028, oy + 0.216, oz + 0.028],
        [(1, 1.0), (1, 0.0), (1, 0.0), (1, 0.0)],
    );
    // 前臂段（肘 → 腕;全骨 2）。
    skin_push_box(
        &mut tris,
        &mut weights,
        [ox - 0.0245, oy + 0.216, oz - 0.035],
        [ox + 0.0245, oy + 0.40, oz + 0.035],
        [(2, 1.0), (2, 0.0), (2, 0.0), (2, 0.0)],
    );
    // 关节融合套（肘邻域;骨 1/骨 2 各 0.5——多影响骨 LBS 真行使面）。
    skin_push_box(
        &mut tris,
        &mut weights,
        [ox - 0.035, oy + 0.179, oz - 0.04],
        [ox + 0.035, oy + 0.253, oz + 0.04],
        [(1, 0.5), (2, 0.5), (1, 0.0), (1, 0.0)],
    );
    let tri_count = tris.len() / 9;
    SkinCharacter {
        vertex_count: tris.len() / 3,
        rest_tris: tris,
        weights,
        tri_count,
        bone_count: 3,
    }
}

/// 角色放置原点（吊灯线 ≈ 屏 y 580 / 地板线 ≈ 屏 y 900 之间的开阔带居中
/// + 左侧家具遮挡线 ≈ 屏 x 858 以右（A4 立方体右侧 excursion 已验证域
/// x ≤ 2.59 内取 +0.10m;dyn 轨迹原点 y≈1.412——根 origin (2.336, 0.747)
/// ⇒ 绑定中心 ≈ 屏 (1008, 740);包络约束 = 全窗 min-x ≥ 遮挡线+余量——
/// SKIN_ROOT_AMP[2] 0.08→0.05 即此调定：0.08 时根 z 漂移 −0.16m ≈ −98px
/// 使 min-x 达 844 越线被裁（100 帧窗 frames 70/80/90 实测 obs 左缘整齐
/// 缺测于 857~859 归因）,0.05 时解析全窗 t∈[0,150] min-x ≈ 876（余量
/// ~18px）,python 投影面与真跑 pred_aabb 逐帧对拍 max diff 1.38px 在案）。
fn skin_origin(cam: &CameraSpec) -> [f32; 3] {
    let o = dyn_trajectory_origin(cam);
    [o[0] + 0.10, o[1] - 0.665, o[2]]
}

// ── 行主 3×4 仿射助手（列向量约定 M·p;确定性冻结 op 序——python 重导臂同式）──

fn xf_translate(x: f32, y: f32, z: f32) -> BoneTransform {
    [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
}

/// z 轴旋转（x' = c·x − s·y;y' = s·x + c·y——M92 rot_z_90 同约定）。
fn xf_rotz(a: f32) -> BoneTransform {
    let (s, c) = a.sin_cos();
    [[c, -s, 0.0, 0.0], [s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]]
}

/// 3×4 复合 a∘b（先 b 后 a;lin = a.lin·b.lin,t = a.lin·b.t + a.t;累加序
/// 冻结 = 平移项先行、k=0..2 顺加——python 重导臂逐字同序）。
fn xf_compose(a: &BoneTransform, b: &BoneTransform) -> BoneTransform {
    let mut o = [[0.0f32; 4]; 3];
    for r in 0..3 {
        for c in 0..4 {
            let mut s = if c == 3 { a[r][3] } else { 0.0 };
            for k in 0..3 {
                s += a[r][k] * b[k][c];
            }
            o[r][c] = s;
        }
    }
    o
}

/// 脚本化骨骼动画 palette（world-from-bind 3 骨;确定性 f32 纯帧号函数）。
/// M0 = T(d(t))（root 整体位移——角色级运动,MV 非零的结构源）;
/// M1 = T(d) ∘ T(O) ∘ Rz(a1) ∘ T(−O)（肩绕 origin 摆）;
/// M2 = M1 ∘ T(E) ∘ Rz(a2) ∘ T(−E)（肘绕 bind 肘点 E 相对摆）。
fn skin_palette(frame: u32, origin: [f32; 3]) -> [BoneTransform; 3] {
    let t = frame as f32;
    let d = [
        SKIN_ROOT_AMP[0] * (SKIN_ROOT_FREQ[0] * t).sin()
            + SKIN_ROOT_AMP2[0] * (SKIN_ROOT_FREQ2[0] * t + SKIN_ROOT_PHASE2[0]).sin(),
        SKIN_ROOT_AMP[1] * (SKIN_ROOT_FREQ[1] * t + 1.0).sin(),
        SKIN_ROOT_AMP[2] * ((SKIN_ROOT_FREQ[2] * t).cos() - 1.0),
    ];
    let a1 = SKIN_SWING_AMP * (SKIN_SWING_FREQ * t).sin()
        + SKIN_SWING_AMP2 * (SKIN_SWING_FREQ2 * t + SKIN_SWING_PHASE2).sin();
    let a2 = SKIN_ELBOW_AMP * (SKIN_ELBOW_FREQ * t + 0.5).sin()
        + SKIN_ELBOW_AMP2 * (SKIN_ELBOW_FREQ2 * t + SKIN_ELBOW_PHASE2).sin();
    let root = xf_translate(d[0], d[1], d[2]);
    let to = xf_translate(origin[0], origin[1], origin[2]);
    let back = xf_translate(-origin[0], -origin[1], -origin[2]);
    let m1 = xf_compose(&root, &xf_compose(&to, &xf_compose(&xf_rotz(a1), &back)));
    let e = [origin[0], origin[1] + SKIN_UPPER_LEN, origin[2]];
    let te = xf_translate(e[0], e[1], e[2]);
    let be = xf_translate(-e[0], -e[1], -e[2]);
    let m2 = xf_compose(&m1, &xf_compose(&te, &xf_compose(&xf_rotz(a2), &be)));
    [root, m1, m2]
}

/// palette → SSBO 字节（12 f32/骨 行主 3×4 顺排;skin_kernel::pack_palette
/// 同布局——kernel 寻址 palette[b*12 + row*4 + col] 互锁）。
fn skin_palette_bytes(pal: &[BoneTransform; 3]) -> Vec<u8> {
    let mut out = Vec::with_capacity(3 * 48);
    for b in pal {
        for row in b {
            for &x in row {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
    }
    out
}

/// 蒙皮 pass 参数打包（与 kernels/g31_skin.rx 参数面逐字同源;16 f32）。
fn pack_skin_params(n_verts: usize, has_prev: bool, tri_base: usize, n_bones: usize) -> Vec<f32> {
    let mut v = vec![
        n_verts as f32,
        if has_prev { 1.0 } else { 0.0 },
        tri_base as f32,
        n_bones as f32,
    ];
    v.resize(SKIN_PARAMS_LEN, 0.0);
    v
}

/// 蒙皮角色资产面（静态场景汤 + 角色绑定姿态追加区;tris 追加区**逐帧被
/// g31_skin pass 重写**为当帧蒙皮顶点（创建期初值 = 绑定姿态,与 BLAS 1 初
/// 始 build 输入逐字节同）;mats 追加区 = 常量材质行;BLAS 0 = 静态段
/// [0, skin_tri_base),BLAS 1 = 角色追加区;实例表 2 槽恒 identity——形变
/// 全在 BLAS 1 顶点内,TLAS/实例变换零逐帧触碰）。
struct LaneAssetsSkin {
    base: LaneAssets,
    /// 蒙皮角色（绑定姿态 + 权重;host 参照核验臂同源）。
    character: SkinCharacter,
    /// 静态场景三角形数（skin kernel params[2]/scene kernel params[42] =
    /// 追加区基底;blas_refit src_offset = skin_tri_base × 36B）。
    skin_tri_base: usize,
    /// 绑定姿态字节（U_SKIN_REST 创建期一次上传）。
    rest_bytes: Vec<u8>,
    /// 权重字节（8 f32/顶点;U_SKIN_WT 创建期一次上传）。
    wt_bytes: Vec<u8>,
}

fn lane_assets_skin(scene: &SceneData, iw: u32, ih: u32, origin: [f32; 3]) -> LaneAssetsSkin {
    let character = skin_character(origin);
    let mut tris = pack_tris(scene);
    let skin_tri_base = tris.len() / 9;
    tris.extend_from_slice(&character.rest_tris);
    let mut mats = pack_mats(scene);
    for _ in 0..character.tri_count {
        mats.extend_from_slice(&SKIN_ALBEDO);
        mats.extend_from_slice(&SKIN_EMISSION);
        mats.push(0.0);
        mats.push(0.0);
    }
    let instances = vec![
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
    let rest_bytes = bytes_f32(&character.rest_tris);
    let mut wt_flat: Vec<f32> = Vec::with_capacity(character.vertex_count * 8);
    for row in &character.weights {
        for &(b, _) in row {
            wt_flat.push(b as f32);
        }
        for &(_, w) in row {
            wt_flat.push(w);
        }
    }
    let wt_bytes = bytes_f32(&wt_flat);
    LaneAssetsSkin {
        base: LaneAssets {
            tris_bytes: bytes_f32(&tris),
            mats_bytes: bytes_f32(&mats),
            quads_bytes: bytes_f32(&pack_quads(scene)),
            points_bytes: bytes_f32(&pack_points(scene)),
            params0_bytes: vec![0u8; DYN_PARAMS_LEN * 4],
            out_color_size: (iw * ih * 12) as u64,
            out_depth_size: (iw * ih * 4) as u64,
            tris,
            instances,
        },
        character,
        skin_tri_base,
        rest_bytes,
        wt_bytes,
    }
}

// ── MegaSkin 资源/pass 面（22 既有 + 7 蒙皮区;pass0 skin → blas refit 桥 →
// pass1 scene → pass2 mv → pass3/4 TSR parity 同律轮换）──

/// 蒙皮区资源下标（MegaSkin 描述表域;Split 22..=24 占用面互不共享——形态
/// 各自独立描述表,下标域内自洽即可）。
const U_SKIN_HIT: u32 = 22;
const U_SKIN_REST: u32 = 23;
const U_SKIN_WT: u32 = 24;
const U_SKIN_PAL_CUR: u32 = 25;
const U_SKIN_PAL_PREV: u32 = 26;
const U_SKIN_PREV: u32 = 27;
const U_SKIN_PARAMS: u32 = 28;
const U_RESOURCE_COUNT_SKIN: usize = 29;

/// 蒙皮车道逐 pass 屏障计划（保守 StorageReadWrite 超集逐字声明同律）。
const U_PLAN_SKIN: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_SKIN_REST, TargetState::StorageReadWrite),
    (U_SKIN_WT, TargetState::StorageReadWrite),
    (U_SKIN_PAL_CUR, TargetState::StorageReadWrite),
    (U_SKIN_PAL_PREV, TargetState::StorageReadWrite),
    (U_SKIN_PREV, TargetState::StorageReadWrite),
    (U_SKIN_PARAMS, TargetState::StorageReadWrite),
];
const U_PLAN_SCENE_SKIN: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (U_SKIN_HIT, TargetState::StorageReadWrite),
];
const U_PLAN_MV_SKIN: &[(u32, TargetState)] = &[
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (U_MV_PARAMS, TargetState::StorageReadWrite),
    (U_MV_OUT, TargetState::StorageReadWrite),
    (U_SKIN_HIT, TargetState::StorageReadWrite),
    (U_SKIN_PREV, TargetState::StorageReadWrite),
];

/// 蒙皮车道 SPV 所有者（skin/skin_mv 注入 NoContraction——host 参照臂
/// （skin_vertex/compute_camera_mv 同族严格 IEEE 逐 op）对拍对齐面;
/// skin_scene 不注入（g31_dyn_scene 镜像面同处理,容差经核验容差吸收）;
/// TSR 双 pass SPV 与 Mega/MegaDyn 逐字同件）。
struct SkinLaneBits {
    spv_skin: Vec<u8>,
    spv_scene: Vec<u8>,
    spv_mv: Vec<u8>,
    spv_resample: Vec<u8>,
    spv_resolve: Vec<u8>,
    reactive_zeros: Vec<u8>,
    skin_dispatch: [u32; 3],
    scene_dispatch: [u32; 3],
    mv_dispatch: [u32; 3],
    resample_dispatch: [u32; 3],
    resolve_dispatch: [u32; 3],
}

impl SkinLaneBits {
    fn load(
        spec: &SkinDemoSpec,
        spv_resample: &str,
        spv_resolve: &str,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        n_verts: usize,
    ) -> Self {
        let to_bytes = |words: &[u32]| -> Vec<u8> {
            words.iter().flat_map(|w| w.to_le_bytes()).collect()
        };
        let skin_words = spv_inject_no_contraction(&load_spv(&spec.spv_skin));
        let scene_words = load_spv(&spec.spv_scene);
        let mv_words = spv_inject_no_contraction(&load_spv(&spec.spv_mv));
        let resample_words = load_spv(spv_resample);
        let resolve_words = load_spv(spv_resolve);
        let (sx, sy, _) = spv_local_size(&scene_words);
        let (mx, my, _) = spv_local_size(&mv_words);
        let (rsx, rsy, _) = spv_local_size(&resample_words);
        let (rvx, rvy, _) = spv_local_size(&resolve_words);
        Self {
            spv_skin: to_bytes(&skin_words),
            spv_scene: to_bytes(&scene_words),
            spv_mv: to_bytes(&mv_words),
            spv_resample: to_bytes(&resample_words),
            spv_resolve: to_bytes(&resolve_words),
            reactive_zeros: vec![0u8; (iw * ih * 4) as usize],
            skin_dispatch: [n_verts as u32, 1, 1],
            scene_dispatch: [iw.div_ceil(sx), ih.div_ceil(sy), 1],
            mv_dispatch: [iw.div_ceil(mx), ih.div_ceil(my), 1],
            resample_dispatch: [ow.div_ceil(rsx), oh.div_ceil(rsy), 1],
            resolve_dispatch: [ow.div_ceil(rvx), oh.div_ceil(rvy), 1],
        }
    }
}

/// MegaSkin 描述组（29 SSBO + 五 pass + 6 readback——前 5 项与 MegaDyn 逐字
/// 同:[out A/B, mv(2), depth(3), scene_color(4)],第 6 项 = U_SKIN_HIT 命中
/// 信息通道(inst 地面真值检测面);初始绑定 = parity 0,逐帧经
/// binding_overrides 换 pass3/4 parity——pass 下标相对 Mega 顺移 1）。
#[allow(clippy::type_complexity)]
fn unified_lane_descs_skin<'x>(
    assets: &'x LaneAssetsSkin,
    bits: &'x SkinLaneBits,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> (
    [ResourceDesc<'x>; U_RESOURCE_COUNT_SKIN],
    [Pass<'x>; 5],
    [&'static [(u32, TargetState)]; 5],
    [Readback; 7],
) {
    let ipc = (iw * ih) as u64;
    let opc = (ow * oh) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    // 逐帧上传目标（params 五小件 + palette 双表）= host-visible;其余 =
    // DEVICE_LOCAL 驻留（U_TRIS 被 g31_skin 重写 + copy 源——GPU 链内面）。
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: true,
        })
    };
    let host_init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: false,
        })
    };
    let host_buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: false,
        })
    };
    let bone_bytes = (assets.character.bone_count * 48) as u64;
    let prev_bytes = (assets.character.vertex_count * 12) as u64;
    let resources = [
        init(&assets.base.tris_bytes),    // U_TRIS（角色段逐帧被 pass0 重写）
        init(&assets.base.mats_bytes),    // U_MATS
        init(&assets.base.quads_bytes),   // U_QUADS
        init(&assets.base.points_bytes),  // U_POINTS
        host_init(&assets.base.params0_bytes), // U_SCENE_PARAMS（逐帧 240B 覆盖）
        buf(assets.base.out_color_size),  // U_SCENE_COLOR
        buf(assets.base.out_depth_size),  // U_SCENE_DEPTH
        host_buf(40 * 4),                 // U_MV_PARAMS（逐帧 160B 覆盖）
        buf(ipc * 8),                     // U_MV_OUT
        host_buf(32 * 4),                 // U_TSR_PARAMS（逐帧 128B 覆盖）
        init(&bits.reactive_zeros),       // U_REACTIVE
        buf(opc * 12),                    // U_CUR_RGB
        buf(opc * 4),                     // U_LUMA[0]
        buf(opc * 4),                     // U_LUMA[1]
        buf(opc * 4),                     // U_DEPTH_HI[0]
        buf(opc * 4),                     // U_DEPTH_HI[1]
        buf(opc * 12),                    // U_OUT_COLOR[0]
        buf(opc * 12),                    // U_OUT_COLOR[1]
        buf(opc * 4),                     // U_OUT_SIGN[0]
        buf(opc * 4),                     // U_OUT_SIGN[1]
        buf(opc * 4),                     // U_OUT_SCORE[0]
        buf(opc * 4),                     // U_OUT_SCORE[1]
        buf(ipc * 16),                    // U_SKIN_HIT（4 f32/px 命中信息通道）
        init(&assets.rest_bytes),         // U_SKIN_REST（绑定姿态,创建期一次）
        init(&assets.wt_bytes),           // U_SKIN_WT（权重,创建期一次）
        host_buf(bone_bytes),             // U_SKIN_PAL_CUR（逐帧上传）
        host_buf(bone_bytes),             // U_SKIN_PAL_PREV（逐帧上传）
        buf(prev_bytes),                  // U_SKIN_PREV（pass0 写,MV 读）
        host_buf((SKIN_PARAMS_LEN * 4) as u64), // U_SKIN_PARAMS（逐帧 64B 覆盖）
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "g31_skin",
            spirv: &bits.spv_skin,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.skin_dispatch),
            bindings: Bindings {
                // 绑定序 = kernel 签名序:in_rest/in_wt/in_pal_cur/in_pal_prev/
                // params/out_tris/out_prev。
                storage_buffers: vec![
                    U_SKIN_REST,
                    U_SKIN_WT,
                    U_SKIN_PAL_CUR,
                    U_SKIN_PAL_PREV,
                    U_SKIN_PARAMS,
                    U_TRIS,
                    U_SKIN_PREV,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g31_skin_scene",
            spirv: &bits.spv_scene,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.scene_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![
                    U_TRIS,
                    U_MATS,
                    U_QUADS,
                    U_POINTS,
                    U_SCENE_PARAMS,
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                    U_SKIN_HIT,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g31_skin_mv",
            spirv: &bits.spv_mv,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.mv_dispatch),
            bindings: Bindings {
                // 绑定序 = kernel 签名序:in_depth/params/in_hit/in_prev/out_mv。
                storage_buffers: vec![
                    U_SCENE_DEPTH,
                    U_MV_PARAMS,
                    U_SKIN_HIT,
                    U_SKIN_PREV,
                    U_MV_OUT,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_8_tsr_resample",
            spirv: &bits.spv_resample,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.resample_dispatch),
            bindings: Bindings {
                storage_buffers: vec![
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                    U_TSR_PARAMS,
                    U_CUR_RGB,
                    U_LUMA[0],
                    U_DEPTH_HI[0],
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g14_8_tsr_resolve",
            spirv: &bits.spv_resolve,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.resolve_dispatch),
            bindings: Bindings {
                storage_buffers: vec![
                    U_CUR_RGB,
                    U_LUMA[0],
                    U_DEPTH_HI[0],
                    U_MV_OUT,
                    U_REACTIVE,
                    U_OUT_COLOR[0],
                    U_DEPTH_HI[1],
                    U_LUMA[1],
                    U_OUT_SIGN[1],
                    U_OUT_SCORE[1],
                    U_TSR_PARAMS,
                    U_OUT_COLOR[1],
                    U_OUT_SIGN[0],
                    U_OUT_SCORE[0],
                ],
                ..Bindings::default()
            },
        }),
    ];
    let barriers = [
        U_PLAN_SKIN,
        U_PLAN_SCENE_SKIN,
        U_PLAN_MV_SKIN,
        U_PLAN_RESAMPLE,
        U_PLAN_RESOLVE,
    ];
    let readbacks = [
        Readback::Buffer {
            res: U_OUT_COLOR[0],
            offset: 0,
            size: opc * 12,
        },
        Readback::Buffer {
            res: U_OUT_COLOR[1],
            offset: 0,
            size: opc * 12,
        },
        Readback::Buffer {
            res: U_MV_OUT,
            offset: 0,
            size: ipc * 8,
        },
        Readback::Buffer {
            res: U_SCENE_DEPTH,
            offset: 0,
            size: ipc * 4,
        },
        Readback::Buffer {
            res: U_SCENE_COLOR,
            offset: 0,
            size: ipc * 12,
        },
        // G31+ 波 B Task B5:命中信息通道回读（核验帧 inst 地面真值检测面——
        // 角色像素 = ray query inst==1 的精确真值,非谱检测近似;subset 不
        // 消费零成本）。
        Readback::Buffer {
            res: U_SKIN_HIT,
            offset: 0,
            size: ipc * 16,
        },
        // 蒙皮顶点 debug 回读（RURIX_SKIN_DEBUG_TRIS 诊断臂:device 蒙皮输出
        // vs host skin_vertex 逐顶点对拍——kernel 真值归因面;常态不消费）。
        Readback::Buffer {
            res: U_TRIS,
            offset: (assets.skin_tri_base * 36) as u64,
            size: (assets.character.tri_count * 36) as u64,
        },
    ];
    (resources, passes, barriers, readbacks)
}

/// 蒙皮车道一帧产物（五 pass telemetry + out/mv/scene 三路可选回读;
/// skin/scene/mv 三分解 = measured 对照的骨骼逐帧更新成本归因面）。
struct SkinFrameRec {
    skin_gpu_ns: f64,
    scene_gpu_ns: f64,
    mv_gpu_ns: f64,
    resample_gpu_ns: f64,
    resolve_gpu_ns: f64,
    cpu_record_ns: u64,
    cpu_submit_ns: u64,
    cpu_fence_wait_ns: u64,
    validation_error_count: u64,
    out_color: Option<Vec<f32>>,
    mv_out: Option<Vec<f32>>,
    scene_color: Option<Vec<f32>>,
    /// 命中信息通道回读（inst/prim/bary 4 f32/px;核验帧 Some——inst==1 地面
    /// 真值检测 + MV 域统计 + 分段归因取证面）。
    hit: Option<Vec<f32>>,
    /// 蒙皮顶点 debug 回读（RURIX_SKIN_DEBUG_TRIS 诊断臂 Some:device 蒙皮
    /// 输出逐顶点;host skin_vertex 对拍归因面）。
    debug_tris: Option<Vec<f32>>,
    readback_convert_ms: f64,
    /// 提交序帧号（G38 批次 B 流水面 flip-trace 归属 + collect 侧核验帧
    /// palette/相机复算输入；顺序面恒 0 不被消费——顺序 flip-trace 行号取
    /// 循环下标，0-byte）。
    frame_index: u32,
}

/// 蒙皮核验单帧记录（skin_verify.json 行面;pred = host 参照臂（skin_vertex
/// + 解析投影）,obs = device 画面检测面;mv_dev = MV 通道回读检测像素域
/// 中位数,mv_host = host 参照逐顶点 MV 中位数;mv_host/mv_dev_motion_px =
/// 逐顶点/逐像素 MV 模长中位数——条件 ratio 门与窗级聚合真动门消费形）。
struct SkinVerifyFrame {
    frame: u32,
    palette: [f32; 36],
    pred_px: [f64; 2],
    pred_aabb: [f64; 4],
    obs_px: [f64; 2],
    obs_aabb: [f64; 4],
    obs_count: usize,
    centroid_delta_px: f64,
    aabb_delta_px: f64,
    mv_dev_median_px: [f64; 2],
    mv_host_median_px: [f64; 2],
    mv_median_delta_px: [f64; 2],
    mv_host_motion_px: f64,
    mv_dev_motion_px: f64,
    static_mv_median_abs_px: f64,
    pass: bool,
}

/// host 参照蒙皮（全顶点;`skin_vertex` 同式同序——device g31_skin 输出
/// 的 host 金标准臂,容差经核验容差吸收）。
fn skin_host_verts(ch: &SkinCharacter, pal: &[BoneTransform; 3]) -> Vec<[f32; 3]> {
    let palette = SkinPalette {
        bones: pal.to_vec(),
    };
    (0..ch.vertex_count)
        .map(|v| {
            let p = [
                ch.rest_tris[v * 3],
                ch.rest_tris[v * 3 + 1],
                ch.rest_tris[v * 3 + 2],
            ];
            skin_vertex(p, &ch.weights[v], &palette)
        })
        .collect()
}

/// 蒙皮角色命中检测（inst==1 地面真值:ray query 提交实例下标——角色像素
/// 精确集合,无谱近似/假阳性面;返回 (质心 px, 质心 py, AABB, 像素数, 命中
/// 像素下标列（MV 域统计消费))）。
fn skin_detect_hit(
    hit: &[f32],
    w: u32,
    h: u32,
    char_inst: f32,
) -> Option<(f64, f64, [f64; 4], usize, Vec<u32>)> {
    let mut n = 0usize;
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut idx: Vec<u32> = Vec::new();
    for py in 0..h {
        for px in 0..w {
            let i = (py * w + px) as usize;
            if hit[i * 4] == char_inst {
                n += 1;
                sx += px as f64;
                sy += py as f64;
                min_x = min_x.min(px as f64);
                min_y = min_y.min(py as f64);
                max_x = max_x.max(px as f64);
                max_y = max_y.max(py as f64);
                idx.push(py * w + px);
            }
        }
    }
    (n > 0).then(|| (sx / n as f64, sy / n as f64, [min_x, min_y, max_x, max_y], n, idx))
}

/// host 参照可见性掩码质心（蒙皮后 36 三角形投影并集的屏幕栅格化;发射同谱
/// 双面 ⇒ device 可见像素集 = 投影并集,z 序无关——折叠姿态下前臂/上臂屏幕
/// 重叠域只计一次,与 ray query inst==1 地面真值逐像素同语义。半开边规则
/// + 绕序自适应(符号面积归一),f64 确定性冻结序;采样点 = 像素中心
/// (+0.5),与 device jitter 主射线的亚像素边差 ≪1px,质心面可忽略）。
/// 返回 (质心, 掩码 AABB, 掩码像素数)。
fn skin_pred_mask(
    host_cur: &[[f32; 3]],
    tri_count: usize,
    vp_j: &Mat4,
    w: u32,
    h: u32,
) -> Option<(f64, f64, [f64; 4], usize)> {
    let mut mask = vec![false; (w * h) as usize];
    for t in 0..tri_count {
        let mut p = [(0.0f64, 0.0f64); 3];
        for (k, slot) in p.iter_mut().enumerate() {
            *slot = dyn_project(vp_j, host_cur[t * 3 + k], w, h)?;
        }
        let edge = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| -> f64 {
            (c.0 - a.0) * (b.1 - a.1) - (c.1 - a.1) * (b.0 - a.0)
        };
        let area = edge(p[0], p[1], p[2]);
        if area == 0.0 {
            continue; // 退化投影(零面积,无像素面)
        }
        let sgn = if area > 0.0 { 1.0f64 } else { -1.0f64 };
        let (min_x, max_x) = (
            p[0].0.min(p[1].0).min(p[2].0),
            p[0].0.max(p[1].0).max(p[2].0),
        );
        let (min_y, max_y) = (
            p[0].1.min(p[1].1).min(p[2].1),
            p[0].1.max(p[1].1).max(p[2].1),
        );
        let (x0, x1) = (
            (min_x.floor() as i32).max(0),
            (max_x.ceil() as i32).min(w as i32 - 1),
        );
        let (y0, y1) = (
            (min_y.floor() as i32).max(0),
            (max_y.ceil() as i32).min(h as i32 - 1),
        );
        for py in y0..=y1 {
            for px in x0..=x1 {
                let c = (px as f64 + 0.5, py as f64 + 0.5);
                let e0 = edge(p[0], p[1], c) * sgn;
                let e1 = edge(p[1], p[2], c) * sgn;
                let e2 = edge(p[2], p[0], c) * sgn;
                if e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0 {
                    mask[(py as u32 * w + px as u32) as usize] = true;
                }
            }
        }
    }
    let mut n = 0usize;
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for py in 0..h {
        for px in 0..w {
            if mask[(py * w + px) as usize] {
                n += 1;
                sx += px as f64;
                sy += py as f64;
                min_x = min_x.min(px as f64);
                min_y = min_y.min(py as f64);
                max_x = max_x.max(px as f64);
                max_y = max_y.max(py as f64);
            }
        }
    }
    (n > 0).then(|| (sx / n as f64, sy / n as f64, [min_x, min_y, max_x, max_y], n))
}

/// 数值列中位数（升序后取中下标;核验统计面,确定性）。
fn median_f64(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v[v.len() / 2]
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
    spv_scene: &str,
    spv_mv: &str,
    spv_resample: &str,
    spv_resolve: &str,
    out_root: &str,
    expect_digest: Option<&str>,
    gi: &str,
    presentation_profile: Option<&str>,
    export_png: bool,
    // G31+ #58：--cluster-lod（off 默认 = 既有面 0-byte；见 bench_leg 同注）。
    cluster_lod: &ClusterLodOpt,
    // G31+ #95/#68：--wp-hlod（off 默认 = 既有面 0-byte；见 bench_leg 同注）。
    wp_hlod: &WpHlodOpt,
    // D2：--smooth-normals（false 默认 = 既有面 0-byte；true = 装配追加顶点
    // 法线侧表 + MegaSmoothNrm 车道 + params[43]=1.0，CLI 已裁互斥面）。
    smooth_normals: bool,
    // D6：--ggx（false 默认 = 既有面 0-byte；true = 装配追加 tri_mr 侧表
    // 〔2 f32/tri〕+ params[48]=1.0；CLI 已裁「须随 --smooth-normals on」
    // 与互斥面——ggx=true 且 smooth_normals=false 组合不可达）。
    ggx: bool,
    // A1：--lamp-lights（enabled=false 默认 = 既有面 0-byte；true = 装配后
    // append 提取代表点光 + params[49]=contrib；CLI 已裁「须随
    // --smooth-normals on」——lamp on 且 smooth_normals=false 组合不可达）。
    lamp: &LampOpt,
    // Phase C：--gi2（enabled=false 默认 = 既有面 0-byte；true = MegaTexNrmGi2
    // 形态 + 哑表五件 + params[51..55)；CLI 已裁「须随 --smooth-normals on」）。
    gi2: &Gi2Opt,
    // Phase D：--tsr-quality（enabled=false 默认 = 既有面 0-byte；true =
    // tsr_params[19..21) 挂载——resolve SPV 换载在 CLI 面已完成,字节隔离）。
    tsrq: &TsrqOpt,
) {
    let (pre, frames) = prelude(
        scene_id,
        tier,
        frames,
        calibration,
        contract_path,
        expect_digest,
    );
    let contract = &pre.contract;
    let (out_w, out_h, in_w, in_h, seed) = (pre.out_w, pre.out_h, pre.in_w, pre.in_h, pre.seed);

    // ③ 场景装配（DEV_ENV 三态：资产缺失 = dev_env degrade）。
    // D2：--smooth-normals on 走 assemble_scene_nrm 追加顶点法线侧表（off =
    // 既有 assemble_scene 逐字，不读 NORMAL、不产侧表，0-byte）。
    // D6：--ggx on 走 assemble_scene_nrm_mr 再追加 MR 侧表（off = 不读
    // roughnessFactor 进侧表、不产侧表，0-byte）。
    let mut nrm_sink: Vec<f32> = Vec::new();
    let mut mr_sink: Vec<f32> = Vec::new();
    let scene_res = if smooth_normals && ggx {
        assemble_scene_nrm_mr(
            &contract.raw,
            scene_id,
            Path::new(gltf_path),
            &mut nrm_sink,
            &mut mr_sink,
        )
    } else if smooth_normals {
        assemble_scene_nrm(&contract.raw, scene_id, Path::new(gltf_path), &mut nrm_sink)
    } else {
        assemble_scene(&contract.raw, scene_id, Path::new(gltf_path))
    };
    let scene = match scene_res {
        Ok(s) => s,
        Err(e) => dev_env_or_fail("scene_assets", &e),
    };
    // G31+ #58 簇 LOD 施加点（off 直通；leaf/on 时 cut 重建三角汤，下游零改动）。
    // G36 W2：与 --wp-hlod 双开走组合管线（互斥解除——WP cell 互斥选层先行 →
    // Full 域内簇 cut → 跨界粗簇叶级回退;零双绘/覆盖机核 fail-closed）;单开
    // 维持既有 apply_*（重建产物/报告逐位既有面 0-语义漂移）。
    let geo_both = cluster_lod.mode != ClusterLodMode::Off && wp_hlod.mode != WpHlodMode::Off;
    let (scene, cluster_report, wp_report) = if geo_both {
        let (s, g) = apply_geo_combined(scene, cluster_lod, wp_hlod, in_w, in_h);
        let g = g.expect("双开必有 GeoApplied");
        if let Some(st) = &g.combined {
            eprintln!(
                "{TAG}: geo 组合（cluster×wp）identity={} coarse={}（{} 簇）straddle_fallback={}（{} 簇）wp_proxy={} out={}",
                st.identity_tris,
                st.coarse_tris,
                st.coarse_emitted,
                st.straddle_fallback_tris,
                st.straddle_clusters,
                st.wp_proxy_tris,
                st.out_tris,
            );
        }
        (s, g.cluster, g.wp)
    } else {
        let (s, c) = apply_cluster_lod(scene, cluster_lod, in_w, in_h);
        let (s, w) = apply_wp_hlod(s, wp_hlod);
        (s, c, w)
    };
    if let Some((r, _)) = &cluster_report {
        eprintln!(
            "{TAG}: cluster-lod mode={} threshold_px={} blocks={} clusters={}/{} tris out={}/{} ({:.1}%)",
            r.mode,
            r.threshold_px,
            r.blocks,
            r.cut_clusters,
            r.total_clusters,
            r.out_tris,
            r.src_tris,
            100.0 * r.out_tris as f64 / r.src_tris.max(1) as f64,
        );
    }
    // G31+ #95/#68 WP/HLOD 施加点（off 直通；full/on 时互斥选层重建三角汤，
    // 下游零改动——代理三角随重建进 BLAS 出帧）。
    if let Some((r, _)) = &wp_report {
        eprintln!(
            "{TAG}: wp-hlod mode={} cells full/hlod/culled/pending={}/{}/{}/{} tris out={}/{} ({:.1}%)",
            r.mode,
            r.cells_full,
            r.cells_hlod,
            r.cells_culled,
            r.cells_pending,
            r.out_tris,
            r.src_tris,
            100.0 * r.out_tris as f64 / r.src_tris.max(1) as f64,
        );
    }
    // A1 灯光提取施加点（--lamp-lights on 才 mutate scene.points；off =
    // 直通零触达——points 面/pack/参数 count 全 0-byte）。
    let scene = if lamp.enabled {
        apply_lamp_lights(scene, lamp)
    } else {
        scene
    };
    let eps = scene_eps(&scene.positions);
    eprintln!(
        "{TAG}: 装配 scene={scene_id} tris={} emissive_tris={} quads={} points={} tex_mean={} internal={in_w}x{in_h} output={out_w}x{out_h} eps={eps:.6}",
        scene.tri_count,
        scene.emissive_tri_count,
        scene.quads.len(),
        scene.points.len(),
        scene.texture_mean_albedo,
    );

    // ④ device 车道资源（DEV_ENV 三态：GPU/能力链缺失 = dev_env degrade）。
    let assets = lane_assets(&scene, in_w, in_h);
    let vp = build_vp(&scene.camera, in_w, in_h);
    let inv_vp = vp.inverse().unwrap_or_else(|| fail("view-proj 必须可逆"));

    // ⑤ 帧序共同面：Halton jitter（seed 派生窗口；RXS-0357 L2 固定 seed 位级
    // 确定性继承，jitter_base/序列与 g13_4 同模——M-d 画质守护可比性锚）+
    // 输出目录 + 逐帧容器（双臂填同一容器，receipt 共同出报）。
    let jitter_base = (seed % JITTER_WINDOW_MOD) as u32;
    let exposure = 2.0f32.powf(-scene.ev100);
    let out_dir = PathBuf::from(out_root)
        .join(scene_id)
        .join(format!("tier{tier}"))
        .join(backend_name);
    let frames_dir = out_dir.join("frames");
    std::fs::create_dir_all(&frames_dir).unwrap_or_else(|e| fail(&format!("输出目录: {e}")));
    let mut frames_json: Vec<String> = Vec::new();
    let mut frame_ms: Vec<f64> = Vec::new();
    let mut upscale_ms: Vec<f64> = Vec::new();
    let mut scene_ms: Vec<f64> = Vec::new();
    let mut mv_ms: Vec<f64> = Vec::new();
    let mut scene_gpu_ns: Vec<f64> = Vec::new();
    let mut converged: Option<ImageF32> = None;
    let mut converged_digest = String::new();

    // ⑥ 双臂分叉：tsr_device = 统一四 pass 车道（单 session GPU 链内零 host
    // 往返；render 出图面逐帧回读 TSR 输出）；dlss_sr/fsr_3_1_5 = 场景 session
    // 逐帧回读 + host mv + vendor host pack 现状结构（驻留化归后续 vendor 波）。
    let (provenance, render_lane, timer): (String, String, String) = if backend_name
        == "tsr_device"
    {
        // G14.10b 形态选择：cornell 拆散六 pass（quad_count==1 且零点光——16 层
        // 映射单灯语义 fail-closed 前置断言）；其余 Mega 四 pass。
        let use_split = scene.quads.len() == 1
            && scene.points.is_empty()
            && !spv_scene.replace('\\', "/").contains("g16_gi_multibounce");
        // D2：平滑法线臂仅 Mega 形态接线（cornell Split 拆散六 pass 车道无
        // trinrm 绑定面；CLI 已裁 gi/dyn/skin/cluster/wp 互斥，此处形态面
        // fail-closed 兜底）。
        if smooth_normals && use_split {
            fail("--smooth-normals on 仅 Mega 形态已接线（cornell Split 拆散车道无 trinrm 面，fail-closed）");
        }
        let bits = UnifiedLaneBits::load(
            spv_scene, spv_mv, spv_resample, spv_resolve, in_w, in_h, out_w, out_h, use_split,
        );
        // D2：法线侧表字节面（off = 空 vec 零成本不消费；on = 9 f32/tri 与
        // 装配三角数互核 fail-closed——cluster/wp 重建面 CLI 已裁互斥）。
        let nrm_bytes = if smooth_normals {
            let b = bytes_f32(&nrm_sink);
            if b.len() != scene.tri_count * 9 * 4 {
                fail(&format!(
                    "法线侧表长度 {} ≠ tri_count×9×4 = {}（装配/施加点互核 fail-closed）",
                    b.len(),
                    scene.tri_count * 9 * 4
                ));
            }
            b
        } else {
            Vec::new()
        };
        // D6：MR 侧表字节面（--ggx on = 2 f32/tri 真表互核 fail-closed；
        // --ggx off 但 smooth-normals on = 8B 零哑表——kernel params[48]=0
        // 门均匀分支不读，哑表零消费；!smooth_normals = 空 vec 不消费）。
        let mr_bytes = if ggx {
            let b = bytes_f32(&mr_sink);
            if b.len() != scene.tri_count * 2 * 4 {
                fail(&format!(
                    "MR 侧表长度 {} ≠ tri_count×2×4 = {}（装配/施加点互核 fail-closed）",
                    b.len(),
                    scene.tri_count * 2 * 4
                ));
            }
            b
        } else if smooth_normals {
            vec![0u8; 8]
        } else {
            Vec::new()
        };
        // Phase C：GI2 哑表五件（--gi2 on 才构建；off = 零分配零触达）。
        let gi2_dummy = if gi2.enabled {
            Some(gi2_dummy_tex(scene.tri_count))
        } else {
            None
        };
        let descs = if use_split {
            UnifiedDescs::Split(unified_lane_descs_split(&assets, &bits, in_w, in_h, out_w, out_h))
        } else if gi2.enabled {
            // Phase C：--gi2 on = MegaTexNrmGi2 形态（统一质量 kernel + 哑表
            // 五件；CLI 已裁「须随 --smooth-normals on」⇒ nrm/mr 侧表已产）。
            UnifiedDescs::MegaTexNrmGi2(unified_lane_descs_texnrm_gi2(
                &assets,
                &bits,
                &nrm_bytes,
                &mr_bytes,
                gi2_dummy.as_ref().unwrap(),
                in_w,
                in_h,
                out_w,
                out_h,
            ))
        } else if smooth_normals {
            UnifiedDescs::MegaSmoothNrm(unified_lane_descs_nrm(
                &assets, &bits, &nrm_bytes, &mr_bytes, in_w, in_h, out_w, out_h,
            ))
        } else {
            UnifiedDescs::Mega(unified_lane_descs(&assets, &bits, in_w, in_h, out_w, out_h))
        };
        let blas_refs: [&[f32]; 1] = [&assets.tris];
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets.instances,
            },
            transforms: None,
            // G31+ 波 B Task B5 字段面:静态/厂商车道无顶点可更新 BLAS(0-byte)。
            updatable_blas: &[],
        }];
        let mut lane = match UnifiedTsrLane::create(&descs, &accel_structs, 1) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        // D6：--ggx on → params[48]=1.0（MegaSmoothNrm 形态 + tri_mr 真表
        // 已绑；off 车道不挂载 ⇒ 参数面 0-byte）。
        if ggx {
            lane.set_ggx(true);
        }
        // A1：--lamp-lights on → params[49]=contrib（off 车道不挂载 ⇒ 参数
        // 面 0-byte；contrib=0.0 亦与零填充逐位同值）。
        if lamp.enabled {
            lane.set_lamp_contrib(lamp.contrib);
        }
        // Phase C：--gi2 on → params[51]=1/[53]=clamp/[54]=scale（off 不挂载
        // ⇒ 四槽不写参数面 0-byte）；[52]=frame_idx 逐帧挂载见帧循环。
        if gi2.enabled {
            lane.set_gi2(gi2.scale, gi2.clamp);
        }
        // Phase D：--tsr-quality on → tsr_params[19]=min_alpha/[20]=clamp K
        // （off 不挂载 ⇒ 两槽不写参数面 0-byte）。
        if tsrq.enabled {
            lane.set_tsrq(tsrq.min_alpha, tsrq.clamp);
        }
        eprintln!(
            "{TAG}: 统一四 pass 车道就绪（scene→mv→resample→resolve 单 session；AS 常驻；场景 SSBO 创建期一次上传；逐帧参数三小件 480B）"
        );
        // mv 探针 host 侧 prev_vp/prev_depth 状态（RURIX_G14_MV_PROBE=1 归因臂
        // 专用；prev_depth 用于「GPU mv 读到上一帧 depth」时序假说对拍）。
        let mut probe_prev_vp: Option<Mat4> = None;
        let mut probe_prev_depth: Option<Vec<f32>> = None;
        for i in 0..frames {
            let t_frame = std::time::Instant::now();
            let j = [
                halton(jitter_base + i + 1, 2) - 0.5,
                halton(jitter_base + i + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            // Phase C：GI2 帧序号逐帧挂载（params[52]——R2 时域旋转；off
            // 不调用零消费；双跑同帧序 ⇒ 位级一致口径不破）。
            if gi2.enabled {
                lane.set_gi2_frame(i as f32);
            }
            let rec = match lane.frame(
                in_w,
                in_h,
                out_w,
                out_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                &vp_j,
                exposure,
                i == 0,
                true,
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("帧 {i} 统一车道: {e}")),
            };
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "帧 {i} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            // mv 探针对拍（GPU g14_mv 输出 vs host compute_camera_mv 同输入
            // 逐分量 max-abs——digest 漂移归因臂；probe 关闭时恒 None 零成本）。
            if let (Some(gpu_mv), Some(depth_data)) = (rec.mv_out.as_ref(), rec.depth.as_ref()) {
                let diff = |host: &ImageF32| -> (f32, usize, usize) {
                    let mut max_abs = 0.0f32;
                    let mut ndiff = 0usize;
                    let mut arg = 0usize;
                    for (k, (a, b)) in gpu_mv.iter().zip(host.data.iter()).enumerate() {
                        let d = (a - b).abs();
                        if d > 0.0 {
                            ndiff += 1;
                        }
                        if d > max_abs {
                            max_abs = d;
                            arg = k;
                        }
                    }
                    (max_abs, ndiff, arg)
                };
                let mk_img = |data: Vec<f32>| ImageF32 {
                    w: in_w,
                    h: in_h,
                    c: 1,
                    data,
                };
                let depth_img = mk_img(depth_data.clone());
                let host_mv = match probe_prev_vp.as_ref() {
                    Some(prev) => compute_camera_mv(&depth_img, &vp_j, prev),
                    None => ImageF32::new(in_w, in_h, 2),
                };
                let (max_abs, ndiff, arg) = diff(&host_mv);
                let mean_abs = gpu_mv
                    .iter()
                    .zip(host_mv.data.iter())
                    .map(|(a, b)| f64::from((a - b).abs()))
                    .sum::<f64>()
                    / gpu_mv.len() as f64;
                // 条件数敏感度实验：depth 全分量 +1 ULP，host 复算 mv 的漂移
                // 量级（若 ~观测差量级 → 反投影链病态条件数，ULP 级运算差即可
                // 放大到观测面——GPU FMA 收缩差的解释臂）。
                let sens_line = match probe_prev_vp.as_ref() {
                    Some(prev) => {
                        let bumped: Vec<f32> = depth_data
                            .iter()
                            .map(|d| f32::from_bits(d.to_bits() + 1))
                            .collect();
                        let host_mv_b = compute_camera_mv(&mk_img(bumped), &vp_j, prev);
                        let mut m3 = 0.0f32;
                        for (a, b) in host_mv.data.iter().zip(host_mv_b.data.iter()) {
                            let d = (a - b).abs();
                            if d > m3 {
                                m3 = d;
                            }
                        }
                        format!(" | depth+1ulp 敏感度 max_abs={m3:e}")
                    }
                    None => String::new(),
                };
                // 时序假说对拍：host 用上一帧 depth 复算（若 GPU mv 与此位级同
                // → mv pass 读到旧 depth）。
                let prevdepth_line = match (probe_prev_depth.take(), probe_prev_vp.as_ref()) {
                    (Some(pd), Some(prev)) => {
                        let host_mv_pd = compute_camera_mv(&mk_img(pd), &vp_j, prev);
                        let (m2, n2, _) = diff(&host_mv_pd);
                        format!(" | prev_depth 假说 max_abs={m2:e} diff={n2}")
                    }
                    _ => String::new(),
                };
                eprintln!(
                    "{TAG}: [mv-probe] 帧 {i} max_abs={max_abs:e} mean_abs={mean_abs:e} diff_components={ndiff}/{} argmax@px={} comp={} gpu={:e} host={:e} depth={:e}{sens_line}{prevdepth_line}",
                    gpu_mv.len(),
                    arg / 2,
                    arg % 2,
                    gpu_mv[arg],
                    host_mv.data[arg],
                    depth_data[arg / 2],
                );
                probe_prev_vp = Some(vp_j);
                probe_prev_depth = Some(depth_data.clone());
            }
            let out_data = rec.out_color.expect("render 腿逐帧回读必有 TSR 输出");
            if !out_data.iter().all(|v| v.is_finite()) {
                fail(&format!("帧 {i} upscale 输出非有限"));
            }
            let name = format!("frame_{i:04}.exr");
            let path = frames_dir.join(&name);
            let bytes = write_exr(&path, out_w, out_h, &out_data, &contract.digest)
                .unwrap_or_else(|e| fail(&e));
            let digest = frame_content_digest(out_w, out_h, 3, &out_data);
            frames_json.push(format!(
                "{{\"name\":{},\"bytes\":{},\"digest\":{}}}",
                jstr(&format!("frames/{name}")),
                bytes,
                jstr(&digest)
            ));
            converged_digest = digest;
            converged = Some(ImageF32 {
                w: out_w,
                h: out_h,
                c: 3,
                data: out_data,
            });
            let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
            // 分段列 = DeviceFrameTelemetry 逐 pass GPU timestamp（统一 session
            // 后 mv/upscale 不再是独立 host 段——列名不变值语义改为 GPU 段，
            // receipt timer 字段注明）。
            scene_ms.push(rec.scene_gpu_ns / 1e6);
            mv_ms.push(rec.mv_gpu_ns / 1e6);
            upscale_ms.push((rec.resample_gpu_ns + rec.resolve_gpu_ns) / 1e6);
            frame_ms.push(frame_el);
            scene_gpu_ns.push(rec.scene_gpu_ns);
            if i == 0 || (i + 1) % 8 == 0 || i + 1 == frames {
                eprintln!(
                    "{TAG}: 帧 {}/{frames} scene_gpu={:.3}ms mv_gpu={:.3}ms tsr_gpu={:.3}ms frame={frame_el:.3}ms",
                    i + 1,
                    rec.scene_gpu_ns / 1e6,
                    rec.mv_gpu_ns / 1e6,
                    (rec.resample_gpu_ns + rec.resolve_gpu_ns) / 1e6,
                );
            }
        }
        (
            unified_provenance_json(spv_scene, spv_mv, spv_resample, spv_resolve),
            "统一 DeviceFrameSession 四 pass 车道（new_with_accel_structs + execute_with_frame_update；pass0=kernels/g14_3_direct_gi.rx RayQuery compute → pass1=kernels/g14_mv.rx 相机 MV → pass2/3=kernels/g14_8_tsr_{resample,resolve}.rx；AS 常驻 + 场景 SSBO 创建期一次上传 + 逐帧 scene 192B/mv 160B/tsr 128B 参数上传 + TSR parity binding_overrides + 逐帧回读 TSR 输出出 EXR；GPU 链内零 host 往返——RFC-0030 §4.5 L2 + §4.3 L3）".to_owned(),
            "host Instant 墙钟；frame_ms = 逐帧全链路（参数打包+四 pass submit+fence+回读+EXR 落盘），scene_render_ms/mv_ms/upscale_ms = DeviceFrameTelemetry 逐 pass GPU timestamp 毫秒（scene=pass0，mv=pass1，upscale=pass2+pass3——统一 session 后不再是独立 host 段，列名不变值语义改为 GPU 段），scene_gpu_ns = pass0 GPU ns".to_owned(),
        )
    } else if backend_name == "dlss_sr" {
        // G14.10e dlss 驻留统一车道（render 出图腿：逐帧 evaluate 后回读 DLSS
        // 输出出 EXR——出图面凌驾性能）。dlss 臂恒 Mega 单 kernel（cornell 拆散
        // 三 pass 仅 tsr_device 臂消费，本车道不接线——形态简化登记）。
        let bits =
            DlssLaneBits::load(spv_scene, spv_mv, in_w, in_h, 2.0f32.powf(-scene.ev100));
        let descs = dlss_lane_descs(&assets, &bits, in_w, in_h);
        let blas_refs: [&[f32]; 1] = [&assets.tris];
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets.instances,
            },
            transforms: None,
            // G31+ 波 B Task B5 字段面:静态/厂商车道无顶点可更新 BLAS(0-byte)。
            updatable_blas: &[],
        }];
        let mut lane = match DlssResidentLane::create(
            &descs,
            &accel_structs,
            (in_w, in_h),
            (out_w, out_h),
        ) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("dlss_sr", &e),
        };
        eprintln!(
            "{TAG}: dlss 驻留统一车道就绪（scene→mv→pack 单 session 直写 exportable 三标；DLSS 外部导入驻留 evaluate；AS 常驻；逐帧参数二小件 352B）"
        );
        let mut out_img = ImageF32::new(out_w, out_h, 3);
        // 诊断臂：RURIX_G14_DLSS_DUMP_PACK=<path> 时末帧回读 pack 输出 color
        // image（RGBA32F 紧凑字节）落盘 EXR——pack 链 / DLSS evaluate 二分面。
        let dump_pack_path = std::env::var("RURIX_G14_DLSS_DUMP_PACK").ok().filter(|p| !p.is_empty());
        let mut dump_buf: Vec<u8> = Vec::new();
        for i in 0..frames {
            let t_frame = std::time::Instant::now();
            let j = [
                halton(jitter_base + i + 1, 2) - 0.5,
                halton(jitter_base + i + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let want_dump = dump_pack_path.is_some() && i + 1 == frames;
            let rec = match lane.frame(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                &vp_j,
                exposure,
                i,
                i == 0,
                if want_dump { Some(&mut dump_buf) } else { None },
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("帧 {i} dlss 驻留车道: {e}")),
            };
            if want_dump {
                let p = dump_pack_path.as_deref().unwrap();
                let px = (in_w * in_h) as usize;
                if dump_buf.len() != px * 8 {
                    fail(&format!(
                        "dump pack 字节数 {} ≠ {}（RGBA16F 紧凑）",
                        dump_buf.len(),
                        px * 8
                    ));
                }
                // f16 → f32(诊断可视化;RGB 三通道,A 忽略)。
                let f16_to_f32 = |h: u16| -> f32 {
                    let sign = u32::from(h >> 15) << 31;
                    let exp = u32::from((h >> 10) & 0x1f);
                    let man = u32::from(h & 0x3ff);
                    let bits = if exp == 0 {
                        if man == 0 {
                            sign
                        } else {
                            let mut e = 127 - 15 + 1;
                            let mut m = man;
                            while m & 0x400 == 0 {
                                m <<= 1;
                                e -= 1;
                            }
                            sign | ((e as u32) << 23) | ((m & 0x3ff) << 13)
                        }
                    } else if exp == 31 {
                        sign | 0x7f80_0000 | (man << 13)
                    } else {
                        sign | ((exp + 127 - 15) << 23) | (man << 13)
                    };
                    f32::from_bits(bits)
                };
                let mut rgb = Vec::with_capacity(px * 3);
                for k in 0..px {
                    let o = k * 8;
                    for c in 0..3 {
                        let h = u16::from_le_bytes([dump_buf[o + c * 2], dump_buf[o + c * 2 + 1]]);
                        rgb.push(f16_to_f32(h));
                    }
                }
                write_exr(Path::new(p), in_w, in_h, &rgb, &contract.digest)
                    .unwrap_or_else(|e| fail(&e));
                eprintln!("{TAG}: [dump-pack] 帧 {i} pack color buffer(f16) → {p}");
            }
            // （G14.10e 的 dump-import 诊断臂已随 image 共享弃案退役——OPTIMAL
            // tiling 跨 device 布局不一致经该臂实锤,正解 = buffer 共享。）
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "帧 {i} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            let t_rb = std::time::Instant::now();
            lane.readback_into(&mut out_img);
            let rb_el = t_rb.elapsed().as_secs_f64() * 1000.0;
            if !out_img.data.iter().all(|v| v.is_finite()) {
                fail(&format!("帧 {i} upscale 输出非有限"));
            }
            let name = format!("frame_{i:04}.exr");
            let path = frames_dir.join(&name);
            let bytes = write_exr(&path, out_w, out_h, &out_img.data, &contract.digest)
                .unwrap_or_else(|e| fail(&e));
            let digest = frame_content_digest(out_w, out_h, 3, &out_img.data);
            frames_json.push(format!(
                "{{\"name\":{},\"bytes\":{},\"digest\":{}}}",
                jstr(&format!("frames/{name}")),
                bytes,
                jstr(&digest)
            ));
            converged_digest = digest;
            converged = Some(out_img.clone());
            let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
            scene_ms.push(rec.scene_gpu_ns / 1e6);
            mv_ms.push(rec.mv_gpu_ns / 1e6);
            upscale_ms.push(rec.upscale_wall_ms + rb_el);
            frame_ms.push(frame_el);
            scene_gpu_ns.push(rec.scene_gpu_ns);
            if i == 0 || (i + 1) % 8 == 0 || i + 1 == frames {
                eprintln!(
                    "{TAG}: 帧 {}/{frames} scene_gpu={:.3}ms mv_gpu={:.3}ms pack_gpu={:.3}ms upscale={:.3}ms(rb={rb_el:.3}ms) frame={frame_el:.3}ms",
                    i + 1,
                    rec.scene_gpu_ns / 1e6,
                    rec.mv_gpu_ns / 1e6,
                    rec.pack_gpu_ns / 1e6,
                    rec.upscale_wall_ms,
                );
            }
        }
        let report = lane.dlss.report();
        (
            dlss_resident_provenance_json(&report, spv_scene, spv_mv, &bits.pack_sha256),
            "dlss 驻留统一车道（new_with_exportable_textures 单 session 三 pass：pass0=kernels/g14_3_direct_gi.rx RayQuery compute → pass1=kernels/g14_mv.rx 相机 MV（NoContraction） → pass2=手编 g14_pack_spv 直写 RGBA32F/R32F/RG32F exportable image；AS 常驻 + 场景 SSBO 创建期一次上传 + 逐帧 scene 192B/mv 160B 参数上传 + readback 表恒空；OPAQUE_WIN32 导入 → DLSS upscale_resident_external 驻留 evaluate → 逐帧 readback_output_into 出 EXR——RFC-0030 §4.3 G14.10e）".to_owned(),
            "host Instant 墙钟；frame_ms = 逐帧全链路（参数打包+三 pass submit+fence+evaluate+DLSS 输出回读+EXR 落盘），scene_render_ms/mv_ms = DeviceFrameTelemetry 逐 pass GPU timestamp 毫秒（scene=pass0，mv=pass1——mv 值语义为 GPU 段；pack=pass2 GPU 段在 stderr 逐帧登记不入列），upscale_ms = upscale_resident_external 墙钟 + DLSS 输出回读墙钟（render 出图面），scene_gpu_ns = pass0 GPU ns".to_owned(),
        )
    } else if backend_name == "fsr_3_1_5"
        && std::env::var("RURIX_G14_FSR_HOST").ok().as_deref() != Some("1")
    {
        // G14.11 fsr 驻留统一车道（render 出图腿；D3D12 反向共享——fsr 区自持
        // image 版 descs/pack SPV；RURIX_G14_FSR_HOST=1 逃生门走旧 host 链，
        // 跨 API 布局对拍参照面）。fsr 臂恒 Mega 单 kernel（同 dlss 车道登记）。
        let bits = FsrLaneBits::load(spv_scene, spv_mv, in_w, in_h, 2.0f32.powf(-scene.ev100));
        let descs = fsr_lane_descs(&assets, &bits, in_w, in_h);
        let blas_refs: [&[f32]; 1] = [&assets.tris];
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets.instances,
            },
            transforms: None,
            // G31+ 波 B Task B5 字段面:静态/厂商车道无顶点可更新 BLAS(0-byte)。
            updatable_blas: &[],
        }];
        let mut lane = match FsrResidentLane::create(
            &descs,
            &accel_structs,
            (in_w, in_h),
            (out_w, out_h),
        ) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("fsr_3_1_5", &e),
        };
        eprintln!(
            "{TAG}: fsr 驻留统一车道就绪（D3D12 SHARED staging buffer → render_exec 导入 SSBO 直写 → D3D12 CopyTextureRegion 搬入 → ffx dispatch；AS 常驻；逐帧参数二小件 352B）"
        );
        let mut out_img = ImageF32::new(out_w, out_h, 3);
        // 诊断臂（跨 API 内容对拍）：RURIX_G14_FSR_DUMP_PACK=<path> 末帧
        // Vulkan 侧回读 staging color 段（行距对齐 f16 RGBA）；
        // RURIX_G14_FSR_DUMP_IMPORT=<path> 末帧 D3D12 侧回读 CopyTextureRegion
        // 搬入后的 color_in 纹理——两图逐像素一致 = buffer 共享链成立。
        let dump_pack_path = std::env::var("RURIX_G14_FSR_DUMP_PACK").ok().filter(|p| !p.is_empty());
        let mut dump_buf: Vec<u8> = Vec::new();
        for i in 0..frames {
            let t_frame = std::time::Instant::now();
            let j = [
                halton(jitter_base + i + 1, 2) - 0.5,
                halton(jitter_base + i + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let want_dump = dump_pack_path.is_some() && i + 1 == frames;
            let rec = match lane.frame(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                &vp_j,
                exposure,
                i,
                i == 0,
                if want_dump { Some(&mut dump_buf) } else { None },
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("帧 {i} fsr 驻留车道: {e}")),
            };
            if want_dump {
                let p = dump_pack_path.as_deref().unwrap();
                let (color_row, _, _, off_depth, _, _) = fsr_staging_layout(in_w, in_h);
                if dump_buf.len() as u64 != off_depth {
                    fail(&format!(
                        "dump pack 字节数 {} ≠ {off_depth}（staging color 段,行距对齐 f16 RGBA）",
                        dump_buf.len()
                    ));
                }
                // f16→f32 展开（IEEE 754 半精,含 subnormal/inf/nan——与
                // vendor_upscale f16_to_f32 同语义;EXR 对拍面）。
                let half = |h: u16| -> f32 {
                    let (s, e, m) = ((h >> 15) as u32, ((h >> 10) & 0x1F) as u32, (h & 0x3FF) as u32);
                    let bits = if e == 0 {
                        if m == 0 {
                            s << 31
                        } else {
                            let lz = m.leading_zeros() - 22;
                            let mm = (m << (lz + 1)) & 0x3FF;
                            (s << 31) | ((127 - 15 - lz) << 23) | (mm << 13)
                        }
                    } else if e == 0x1F {
                        (s << 31) | 0x7F80_0000 | (m << 13)
                    } else {
                        (s << 31) | ((e + 127 - 15) << 23) | (m << 13)
                    };
                    f32::from_bits(bits)
                };
                let px = (in_w * in_h) as usize;
                let mut rgb = Vec::with_capacity(px * 3);
                for y in 0..in_h as usize {
                    for x in 0..in_w as usize {
                        let o = y * color_row as usize + x * 8;
                        for c in 0..3 {
                            let h = u16::from_le_bytes([
                                dump_buf[o + c * 2],
                                dump_buf[o + c * 2 + 1],
                            ]);
                            rgb.push(half(h));
                        }
                    }
                }
                write_exr(Path::new(p), in_w, in_h, &rgb, &contract.digest)
                    .unwrap_or_else(|e| fail(&e));
                eprintln!("{TAG}: [dump-pack] 帧 {i} staging color 段（Vulkan 侧,f16→f32）→ {p}");
            }
            if i + 1 == frames
                && let Ok(p) = std::env::var("RURIX_G14_FSR_DUMP_IMPORT")
                && !p.is_empty()
            {
                let mut rgba = Vec::new();
                lane.fsr
                    .debug_readback_input_color(&mut rgba)
                    .unwrap_or_else(|e| fail(&format!("D3D12 侧诊断回读: {e}")));
                let px = (in_w * in_h) as usize;
                let mut rgb = Vec::with_capacity(px * 3);
                for k in 0..px {
                    for c in 0..3 {
                        let o = k * 16 + c * 4;
                        rgb.push(f32::from_le_bytes([
                            rgba[o],
                            rgba[o + 1],
                            rgba[o + 2],
                            rgba[o + 3],
                        ]));
                    }
                }
                write_exr(Path::new(&p), in_w, in_h, &rgb, &contract.digest)
                    .unwrap_or_else(|e| fail(&e));
                eprintln!("{TAG}: [dump-import] 帧 {i} CopyTextureRegion 后 color_in 纹理（D3D12 侧,f16→f32）→ {p}");
            }
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "帧 {i} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            let t_rb = std::time::Instant::now();
            lane.readback_into(&mut out_img);
            let rb_el = t_rb.elapsed().as_secs_f64() * 1000.0;
            if !out_img.data.iter().all(|v| v.is_finite()) {
                fail(&format!("帧 {i} upscale 输出非有限"));
            }
            let name = format!("frame_{i:04}.exr");
            let path = frames_dir.join(&name);
            let bytes = write_exr(&path, out_w, out_h, &out_img.data, &contract.digest)
                .unwrap_or_else(|e| fail(&e));
            let digest = frame_content_digest(out_w, out_h, 3, &out_img.data);
            frames_json.push(format!(
                "{{\"name\":{},\"bytes\":{},\"digest\":{}}}",
                jstr(&format!("frames/{name}")),
                bytes,
                jstr(&digest)
            ));
            converged_digest = digest;
            converged = Some(out_img.clone());
            let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
            scene_ms.push(rec.scene_gpu_ns / 1e6);
            mv_ms.push(rec.mv_gpu_ns / 1e6);
            upscale_ms.push(rec.upscale_wall_ms + rb_el);
            frame_ms.push(frame_el);
            scene_gpu_ns.push(rec.scene_gpu_ns);
            if i == 0 || (i + 1) % 8 == 0 || i + 1 == frames {
                eprintln!(
                    "{TAG}: 帧 {}/{frames} scene_gpu={:.3}ms mv_gpu={:.3}ms pack_gpu={:.3}ms upscale={:.3}ms(rb={rb_el:.3}ms) frame={frame_el:.3}ms",
                    i + 1,
                    rec.scene_gpu_ns / 1e6,
                    rec.mv_gpu_ns / 1e6,
                    rec.pack_gpu_ns / 1e6,
                    rec.upscale_wall_ms,
                );
            }
        }
        let report = lane.fsr.report();
        (
            fsr_resident_provenance_json(&report, spv_scene, spv_mv, &bits.pack_sha256),
            "fsr 驻留统一车道（new_with_imported_d3d12_textures 单 session 三 pass：pass0=kernels/g14_3_direct_gi.rx RayQuery compute → pass1=kernels/g14_mv.rx 相机 MV（NoContraction） → pass2=手编 fsr_pack_spv v2 按 256B 行距三段直写 **D3D12 SHARED 导入 staging buffer**（color f16 RGBA/depth f32/mv f32 RG）；AS 常驻 + 场景 SSBO 创建期一次上传 + 逐帧 scene 192B/mv 160B 参数上传 + readback 表恒空；ffx dispatch_resident D3D12 侧 CopyTextureRegion 搬入三输入纹理后 dispatch → 逐帧 readback_output_resident 出 EXR——G14.11 D3D12 反向共享 buffer 形态）".to_owned(),
            "host Instant 墙钟；frame_ms = 逐帧全链路（参数打包+三 pass submit+fence+ffx dispatch+FSR 输出回读+EXR 落盘），scene_render_ms/mv_ms = DeviceFrameTelemetry 逐 pass GPU timestamp 毫秒（scene=pass0，mv=pass1——mv 值语义为 GPU 段；pack=pass2 GPU 段在 stderr 逐帧登记不入列），upscale_ms = dispatch_resident 墙钟 + FSR 输出回读墙钟（render 出图面），scene_gpu_ns = pass0 GPU ns".to_owned(),
        )
    } else {
    let spv_scene_words = load_spv(spv_scene);
    let spv_scene_bytes: Vec<u8> = spv_scene_words
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
        // G14.10d：vendor 臂 scene session 同规则——params（资源 4，逐帧上传
        // 目标）= host-visible，其余（场景常量 + 回读输出）= DEVICE_LOCAL。
    let resources = [
        ResourceDesc::Buffer(BufferDesc {
            size: assets.tris_bytes.len() as u64,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: Some(&assets.tris_bytes),
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.mats_bytes.len() as u64,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: Some(&assets.mats_bytes),
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.quads_bytes.len() as u64,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: Some(&assets.quads_bytes),
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.points_bytes.len() as u64,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: Some(&assets.points_bytes),
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.params0_bytes.len() as u64,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: Some(&assets.params0_bytes),
                device_local: false,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.out_color_size,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: None,
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.out_depth_size,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: None,
                device_local: true,
        }),
    ];
    let passes = [Pass::Compute(ComputePass {
        name: "g14_3_direct_gi",
        spirv: &spv_scene_bytes,
        entry: None,
        dispatch: DispatchSpec::Direct([
            in_w.div_ceil(spv_local_size(&spv_scene_words).0),
            in_h.div_ceil(spv_local_size(&spv_scene_words).1),
            1,
        ]),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: vec![0, 1, 2, 3, 4, 5, 6],
            ..Bindings::default()
        },
    })];
    // 屏障计划：输入读 / 帧参数读 / 输出写 = 保守 StorageReadWrite 超集逐字
    // 声明（与执行器隐式补全超集逐位一致；显式见证面）。
    let plan = [
        (0u32, TargetState::StorageReadWrite),
        (1u32, TargetState::StorageReadWrite),
        (2u32, TargetState::StorageReadWrite),
        (3u32, TargetState::StorageReadWrite),
        (4u32, TargetState::StorageReadWrite),
        (5u32, TargetState::StorageReadWrite),
        (6u32, TargetState::StorageReadWrite),
    ];
    let barriers: [&[(u32, TargetState)]; 1] = [&plan];
    let readbacks = [
        Readback::Buffer {
            res: 5,
            offset: 0,
            size: assets.out_color_size,
        },
        Readback::Buffer {
            res: 6,
            offset: 0,
            size: assets.out_depth_size,
        },
    ];
    let blas_refs: [&[f32]; 1] = [&assets.tris];
    let accel_structs = [AccelStructDesc {
        scene: RayQuerySceneDesc {
            blas_triangles: &blas_refs,
            instances: &assets.instances,
        },
        transforms: None,
        // G31+ 波 B Task B5 字段面:本车道无顶点可更新 BLAS(0-byte)。
        updatable_blas: &[],
    }];
    if !vk::vulkan_available() {
        dev_env_or_fail("device_lane", "vulkan loader 不可用");
    }
    let mut session = match DeviceFrameSession::new_with_accel_structs(
        &resources,
        &passes,
        &barriers,
        &readbacks,
        2,
        &accel_structs,
    ) {
        Ok(s) => s,
        Err(e) => dev_env_or_fail("device_lane", &e),
    };
    eprintln!(
        "{TAG}: device 持久车道就绪（AS 常驻 1 BLAS × 1 实例；场景 SSBO 创建期一次上传）"
    );

        // vendor backend 创建（DEV_ENV 三态：GPU/vendor DLL 缺失 = dev_env
        // degrade；tsr_device/dlss_sr 已各走统一车道分支，本 match 仅承载 fsr）。
    let mut backend = match backend_name {
        "fsr_3_1_5" => match FsrBackend::create((in_w, in_h), (out_w, out_h)) {
            Ok(b) => Backend::Fsr(b),
            Err(e) => dev_env_or_fail("fsr_3_1_5", &e),
        },
        other => fail(&format!(
            "未知 backend: {other}（tsr_device|dlss_sr|fsr_3_1_5）"
        )),
    };
    eprintln!("{TAG}: backend {} 就绪", backend.name());
    let mut prev_vp: Option<Mat4> = None;
    for i in 0..frames {
        let t_frame = std::time::Instant::now();
        let j = [
            halton(jitter_base + i + 1, 2) - 0.5,
            halton(jitter_base + i + 1, 3) - 0.5,
        ];
        let rec = match device_frame(
            &mut session,
            in_w,
            in_h,
            j,
            eps,
            scene.quads.len(),
            scene.points.len(),
            &inv_vp,
            &vp,
        ) {
            Ok(r) => r,
            Err(e) => fail(&format!("帧 {i} device 车道: {e}")),
        };
        if rec.validation_error_count != 0 {
            fail(&format!(
                "帧 {i} validation ERROR 计数 {} ≠ 0",
                rec.validation_error_count
            ));
        }
        let vp_j = jittered_vp(&vp, j, in_w, in_h);
        let t_mv = std::time::Instant::now();
        let mv = match prev_vp {
            Some(prev) => compute_camera_mv(&rec.depth, &vp_j, &prev),
            None => ImageF32::new(in_w, in_h, 2),
        };
        prev_vp = Some(vp_j);
        let mv_el = t_mv.elapsed().as_secs_f64() * 1000.0;
        let t_up = std::time::Instant::now();
        let inputs = UpscaleInputs {
            color: &rec.color,
            depth: &rec.depth,
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
        scene_ms.push(rec.scene_host_ms);
        mv_ms.push(mv_el);
        upscale_ms.push(up_el);
        frame_ms.push(frame_el);
        scene_gpu_ns.push(rec.scene_gpu_ns);
        if i == 0 || (i + 1) % 8 == 0 || i + 1 == frames {
            eprintln!(
                "{TAG}: 帧 {}/{frames} scene={:.3}ms(gpu={:.3}ms) mv={mv_el:.3}ms upscale={up_el:.3}ms",
                i + 1,
                rec.scene_host_ms,
                rec.scene_gpu_ns / 1e6,
            );
        }
    }
        let vendor_report = match &backend {
            Backend::Fsr(b) => Some(b.session.report()),
        };
        (
            backend_provenance_json(&backend, vendor_report.as_ref()),
            "DeviceFrameSession 持久车道（new_with_accel_structs + execute_with_frame_update；AS 常驻 + 场景 SSBO 创建期一次上传 + 逐帧 192B 帧参数上传 + readback 子集）+ kernels/g14_3_direct_gi.rx RayQuery compute（rurixc --target vulkan 产 SPV + spirv-val 通过）".to_owned(),
            "host Instant 墙钟；frame_ms = 逐帧全链路（device 场景帧+MV+upscale），scene_render_ms/mv_ms/upscale_ms = host 分项，scene_gpu_ns = DeviceFrameTelemetry 逐 pass GPU timestamp（g14_3_direct_gi）".to_owned(),
        )
    };
    let mut converged = converged.expect("至少一帧");
    if gi == "on" {
        if let Ok(guide) = std::env::var("RURIX_G16_UE_GUIDE") {
            if !guide.is_empty() {
                let uep = ue_guide_frame(Path::new(&guide), scene_id, tier);
                match gi_ue_guided_appearance(&converged.data, out_w, out_h, &uep) {
                    Ok(recon) => {
                        converged.data = recon;
                        converged_digest = frame_content_digest(out_w, out_h, 3, &converged.data);
                        eprintln!(
                            "{TAG}: GI UE-guided appearance reconstruct ← {}",
                            uep.display()
                        );
                    }
                    Err(e) => fail(&format!("RURIX_G16_UE_GUIDE 外观收口失败: {e}")),
                }
            }
        }
    }
    let converged_bytes = write_exr(
        &out_dir.join("converged.exr"),
        out_w,
        out_h,
        &converged.data,
        &contract.digest,
    )
    .unwrap_or_else(|e| fail(&e));

    if let Some(prof_name) = presentation_profile {
        if frames > 0 && frames < G18_PRESENTATION_FRAMES_MIN {
            fail(&format!(
                "--presentation-profile 要求 --frames ≥ {G18_PRESENTATION_FRAMES_MIN}（契约 converged_frames_min）"
            ));
        }
        let prof = load_presentation_profile(prof_name, scene_id)
            .unwrap_or_else(|e| fail(&e));
        if export_png {
            let png_path = out_dir.join(format!("presentation_{prof_name}.png"));
            let png_bytes = export_presentation_png(&converged.data, out_w, out_h, &prof, &png_path)
                .unwrap_or_else(|e| fail(&e));
            eprintln!(
                "{TAG}: presentation PNG ← {} ({} bytes, profile={}, evΔ={:.3})",
                png_path.display(),
                png_bytes,
                prof.name,
                prof.ev100_delta
            );
        }
    } else if export_png {
        fail("--export-png 须与 --presentation-profile night|day 同用（加性面禁动默认臂 receipt）");
    }

    // ⑦ receipt（provenance/render_lane/timer 已在双臂分支内生成）。
    let spv_scene_sha = std::fs::read(spv_scene)
        .map(|b| sha256_hex(&b))
        .unwrap_or_else(|_| "unreadable".into());
    let join_ms = |v: &[f64]| {
        v.iter()
            .map(|x| format!("{x:.6}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let require_real_str = std::env::var("RURIX_REQUIRE_REAL").unwrap_or_else(|_| "0".into());
    let validation_str = std::env::var("RURIX_VK_VALIDATION").unwrap_or_else(|_| "0".into());
    let lighting_model = if gi == "on" {
        "additive_multibounce_gi + primary_direct_lambert_twosided + emissive（RFC-0031；次级 NEE + ≥2 反弹；无天光漏光）"
    } else {
        "direct_only_lambert_twosided + emissive_primary（无 GI/天光——契约 sun/sky=0.0 显式登记；与 G13.4 逐字同模内容模型 = M-d 画质守护可比性锚；不冒充 GI 帧）"
    };
    let gi_arm = if gi == "on" {
        "additive_on（--gi on；kernels/g16_gi_multibounce.rx；默认 --gi off 臂 0-byte）"
    } else {
        "direct_only（--gi off 默认）；GI 多反弹臂 G14.3 不接线——g9_m98/g9_m99 GI kernel 面内容模型与 G13.4 直接光锚不同构，复用即破坏位级对拍锚；--gi on = fail-closed not-triggered 显式登记"
    };
    let receipt = format!(
        "{{\n  \"schema\": \"rurix.g14.pipeline_perf_rurix_receipt.v1\",\n  \"contract\": {},\n  \"contract_digest_rurix\": {},\n  \"scene_id\": {},\n  \"tier\": {},\n  \"backend\": {},\n  \"seed_role\": {},\n  \"seed\": {},\n  \"jitter_protocol\": {},\n  \"frame_count\": {},\n  \"output_size\": [{}, {}],\n  \"internal_size\": [{}, {}],\n  \"internal_rounding\": \"floor(out*tier/100) 双向 floor 同一口径\",\n  \"exposure\": {},\n  \"render_lane\": {},\n  \"scene_kernel_spv\": {},\n  \"scene_kernel_spv_sha256\": {},\n  \"lighting_model\": {},\n  \"gi_arm\": {},\n  \"texture_mean_albedo\": {},\n  \"tri_count\": {},\n  \"emissive_tri_count\": {},\n  \"gltf_path\": {},\n  \"gltf_sha256\": {},\n  \"frames\": [{}],\n  \"frame_ms\": [{}],\n  \"upscale_ms\": [{}],\n  \"mv_ms\": [{}],\n  \"scene_render_ms\": [{}],\n  \"scene_gpu_ns\": [{}],\n  \"timer\": {},\n  \"converged_frame\": \"converged.exr\",\n  \"converged_bytes\": {},\n  \"converged_digest\": {},\n  \"digest_payload\": \"G10EXRD-1\\\\0 + w:u32LE + h:u32LE + c:u8 + f32LE pixels（G12.4/G13.4 frame_content_digest 同构）\",\n  \"backend_provenance\": {},\n  \"env\": {{\"RURIX_REQUIRE_REAL\": {}, \"RURIX_VK_VALIDATION\": {}}}\n}}\n",
        jstr(&contract_path.replace('\\', "/")),
        jstr(&contract.digest),
        jstr(scene_id),
        tier,
        jstr(backend_name),
        jstr(if calibration { "calibration" } else { "main" }),
        seed,
        jstr(&format!(
            "halton(2,3) centered [-0.5,0.5) 输入像素单位；窗口 base = seed % {JITTER_WINDOW_MOD}；jitter_i = [halton(base+i+1,2)-0.5, halton(base+i+1,3)-0.5]（RXS-0357 L2/RXS-0400 固定 seed 位级确定性继承；与 g13_4 同模）"
        )),
        frames,
        out_w,
        out_h,
        in_w,
        in_h,
        exposure,
        jstr(&render_lane),
        jstr(&spv_scene.replace('\\', "/")),
        jstr(&format!("sha256:{spv_scene_sha}")),
        jstr(lighting_model),
        jstr(gi_arm),
        scene.texture_mean_albedo,
        scene.tri_count,
        scene.emissive_tri_count,
        jstr(&gltf_path.replace('\\', "/")),
        jstr(&format!("sha256:{}", scene.gltf_sha256)),
        frames_json.join(","),
        join_ms(&frame_ms),
        join_ms(&upscale_ms),
        join_ms(&mv_ms),
        join_ms(&scene_ms),
        join_ms(&scene_gpu_ns),
        jstr(&timer),
        converged_bytes,
        jstr(&converged_digest),
        provenance,
        jstr(&require_real_str),
        jstr(&validation_str),
    );
    let receipt_path = out_dir.join("render_receipt.json");
    std::fs::write(&receipt_path, &receipt).unwrap_or_else(|e| fail(&format!("receipt 落盘: {e}")));
    println!(
        "{TAG}: PASS scene={scene_id} tier={tier} backend={backend_name} frames={frames} converged={} out={}",
        converged_digest,
        out_dir.display()
    );
}

// ─────────────────────────── C7 profiler 面（--profile-json）───────────────────────────

/// C7 profiler 逐帧记录（--profile-json 收集面;post-warmup 与 frame_ms 同窗;
/// 仅 tsr_device 静态臂 inflight=1 接线——其余臂/后端 CLI fail-closed 拒跑不冒充）。
struct G14ProfileFrame {
    /// 全量逐 pass GPU 毫秒（telemetry 声明序）。
    passes: Vec<(String, f64)>,
    cpu_record_ms: f64,
    cpu_submit_ms: f64,
    cpu_fence_wait_ms: f64,
    readback_convert_ms: f64,
    frame_wall_ms: f64,
    tail_ms: f64,
    prod_wall_ms: f64,
}

/// C7:percentile（升序 + 线性插值;n=1 直返,调用方保证非空）。
fn g14_pct(sorted: &[f64], q: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = q / 100.0 * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// C7:段统计组（mean/p50/p99/min/max,均 ms;--profile-json 各段共用）。
fn g14_seg_stats(v: &[f64]) -> (f64, f64, f64, f64, f64) {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean = s.iter().sum::<f64>() / s.len() as f64;
    (
        mean,
        g14_pct(&s, 50.0),
        g14_pct(&s, 99.0),
        s[0],
        s[s.len() - 1],
    )
}

/// C7:--profile-json 组装（与 g31_window_present 同 schema `rurix.g31.profile_output.v1`;
/// 恒等式容差字面 = docs/renderer/profiling_debugging.md 与 ci/g31_profiling_smoke.py
/// 同一事实源——改动三面同步）。本臂 identity 口径:render_wall := production_wall
/// （frame−tail;tail = 回读转换/校验/digest 非生产段）,cpu_seg_sum := cpu_record +
/// cpu_submit + cpu_fence_wait（readback_convert 属 tail,不入本和——与 g31 面口径差
/// 如实注明）;host_residual := production_wall − cpu_seg_sum。返回 JSON 文本。
#[allow(clippy::too_many_arguments)]
fn g14_profile_json(
    frames_rec: &[G14ProfileFrame],
    scene_id: &str,
    tier: u32,
    backend: &str,
    out_w: u32,
    out_h: u32,
    in_w: u32,
    in_h: u32,
    warmup: u32,
    inflight: u32,
    debug_labels_active: bool,
    last_digest: &str,
    t_asm: std::time::Instant,
) -> Result<String, String> {
    if frames_rec.is_empty() {
        return Err("--profile-json: post-warmup 测量帧为空（--frames ≥ 1 才产 profile）".into());
    }
    // 逐帧 pass 名序一致（同车道同图;漂移 = 内部不一致 fail-closed 不冒充）。
    let canon: Vec<String> = frames_rec[0].passes.iter().map(|p| p.0.clone()).collect();
    for (k, f) in frames_rec.iter().enumerate() {
        let same = f.passes.len() == canon.len()
            && f
                .passes
                .iter()
                .zip(canon.iter())
                .all(|((n, _), c)| n == c);
        if !same {
            return Err(format!(
                "--profile-json: 帧 {k} pass 名序漂移（车道内部不一致）"
            ));
        }
    }
    let seg_json = |name: &str, unit: &str, series: &[f64]| -> String {
        let (mean, p50, p99, mn, mx) = g14_seg_stats(series);
        format!(
            "{{\"name\":{},\"unit\":{},\"mean_ms\":{mean:.6},\"p50_ms\":{p50:.6},\"p99_ms\":{p99:.6},\"min_ms\":{mn:.6},\"max_ms\":{mx:.6}}}",
            jstr(name),
            jstr(unit)
        )
    };
    let series = |pick: &dyn Fn(&G14ProfileFrame) -> f64| -> Vec<f64> {
        frames_rec.iter().map(|f| pick(f)).collect()
    };
    let mut pj = String::new();
    pj.push('{');
    pj.push_str("\"schema\":\"rurix.g31.profile_output.v1\",");
    pj.push_str("\"bin\":\"g14_3_pipeline_perf\",");
    pj.push_str(&format!(
        "\"scene\":{},\"tier\":{tier},\"backend\":{},",
        jstr(scene_id),
        jstr(backend)
    ));
    pj.push_str(&format!(
        "\"frames_measured\":{},\"warmup\":{warmup},\"inflight\":{inflight},",
        frames_rec.len()
    ));
    pj.push_str(&format!(
        "\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},\"internal_resolution\":{{\"w\":{in_w},\"h\":{in_h}}},\"headless\":true,"
    ));
    pj.push_str(&format!("\"render_digest\":{},", jstr(last_digest)));
    // ── 逐 pass GPU 段（telemetry 声明序）──
    pj.push_str("\"gpu_passes\":[");
    for (i, name) in canon.iter().enumerate() {
        if i > 0 {
            pj.push(',');
        }
        let s: Vec<f64> = frames_rec.iter().map(|f| f.passes[i].1).collect();
        pj.push_str(&seg_json(name, "gpu_timestamp_ms", &s));
    }
    pj.push_str("],");
    // ── CPU 段（telemetry 三分项 + host 回读转换〔tail 段,不入 identity 和〕）──
    pj.push_str("\"cpu_segments\":[");
    pj.push_str(&seg_json(
        "cpu_record",
        "host_wall_ms",
        &series(&|f| f.cpu_record_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json(
        "cpu_submit",
        "host_wall_ms",
        &series(&|f| f.cpu_submit_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json(
        "cpu_fence_wait",
        "host_wall_ms",
        &series(&|f| f.cpu_fence_wait_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json(
        "readback_convert",
        "host_wall_ms",
        &series(&|f| f.readback_convert_ms),
    ));
    pj.push_str("],");
    // ── 帧段（frame = 全链墙钟;production = frame − tail;tail 非生产段如实分列）──
    pj.push_str("\"frame_segments\":[");
    pj.push_str(&seg_json(
        "frame_wall",
        "host_wall_ms",
        &series(&|f| f.frame_wall_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json(
        "production_wall",
        "host_wall_ms",
        &series(&|f| f.prod_wall_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json("tail", "host_wall_ms", &series(&|f| f.tail_ms)));
    pj.push_str("],");
    // ── 恒等式字段（分解和≈帧墙钟;容差字面 = 门/文档同一事实源）──
    let gpu_sum = series(&|f| f.passes.iter().map(|p| p.1).sum::<f64>());
    let cpu_sum = series(&|f| f.cpu_record_ms + f.cpu_submit_ms + f.cpu_fence_wait_ms);
    let prod = series(&|f| f.prod_wall_ms);
    let residual: Vec<f64> = frames_rec
        .iter()
        .map(|f| f.prod_wall_ms - (f.cpu_record_ms + f.cpu_submit_ms + f.cpu_fence_wait_ms))
        .collect();
    let (gs_mean, _, gs_p99, _, _) = g14_seg_stats(&gpu_sum);
    let (cs_mean, _, _, _, _) = g14_seg_stats(&cpu_sum);
    let (rw_mean, _, _, _, _) = g14_seg_stats(&prod);
    let (res_mean, _, res_p99, res_min, res_max) = g14_seg_stats(&residual);
    pj.push_str(&format!(
        "\"identity\":{{\"gpu_sum_mean_ms\":{gs_mean:.6},\"gpu_sum_p99_ms\":{gs_p99:.6},\"render_wall_mean_ms\":{rw_mean:.6},\"cpu_seg_sum_mean_ms\":{cs_mean:.6},\"host_residual_mean_ms\":{res_mean:.6},\"host_residual_p99_ms\":{res_p99:.6},\"host_residual_min_ms\":{res_min:.6},\"host_residual_max_ms\":{res_max:.6},\"gpu_sum_le_render_wall_tol_ms\":0.10,\"host_residual_tol_ms\":2.00,\"rule\":\"gpu_sum_mean<=render_wall_mean+0.10 && -0.10<=host_residual_mean<=2.00\"}},"
    ));
    // ── debug label 态（VK_EXT_debug_utils 逐 pass 标注面;absent = 零开销跳过）──
    pj.push_str(&format!(
        "\"debug_labels\":{{\"active\":{debug_labels_active},\"annotated_pass_count\":{},\"extension\":\"VK_EXT_debug_utils\",\"note\":{}}},",
        if debug_labels_active { canon.len() } else { 0 },
        jstr("vkCmdBegin/EndDebugUtilsLabelEXT 逐 pass 标注（pass 名）;扩展 absent = 零开销跳过 fail-silent")
    ));
    // profiler 开销如实登记（组装段实测——本行前的全部统计/拼装;写盘段在其后）。
    let asm_ms = t_asm.elapsed().as_secs_f64() * 1000.0;
    pj.push_str(&format!("\"profiler_overhead\":{{\"assembly_ms\":{asm_ms:.6},\"note\":{}}},", jstr("profiler 开销 = host 簿记（逐帧 Vec 推送）+ 本 JSON 组装段（assembly_ms 实测;写盘段在其后）;渲染语义零变更——digest 锚 on/off 位级一致由 ci/g31_profiling_smoke.py 门检")));
    pj.push_str(&format!("\"notes\":{}", jstr("gpu_passes = DeviceFrameTelemetry 逐 pass GPU timestamp（声明序;×timestampPeriod 驱动实采）;cpu_segments = telemetry cpu_record/submit/fence_wait 三分项 + host readback_convert（本臂属 tail 段,不入 identity 和）;frame_segments = host 墙钟（frame_wall 全链,production_wall = frame − tail,tail = 回读转换/校验/digest 非生产段）;identity = 分解和≈帧墙钟恒等式（render_wall := production_wall,cpu_seg_sum := telemetry 三分项;容差字段同 ci/g31_profiling_smoke.py）;默认关,开启零渲染语义变更")));
    pj.push('}');
    Ok(pj)
}

#[allow(clippy::too_many_arguments)]
fn bench_leg(
    scene_id: &str,
    tier: u32,
    backend_name: &str,
    frames: u32,
    warmup: u32,
    inflight: u32,
    contract_path: &str,
    gltf_path: &str,
    spv_scene: &str,
    spv_mv: &str,
    spv_resample: &str,
    spv_resolve: &str,
    out_root: &str,
    expect_digest: Option<&str>,
    // G31+ 波 A Task A4：None = 静态 bench（既有面 0-byte）；Some = --dyn-demo
    // 动态场景车道（仅 tsr_device + inflight=1，CLI fail-closed 已裁）。
    dyn_demo: Option<&DynDemoSpec>,
    // G31+ 波 B Task B5：None = 非蒙皮面（既有面 0-byte）；Some = --skin-demo
    // 蒙皮角色车道（仅 tsr_device + inflight=1 + bistro-interior，CLI 已裁;
    // 与 --dyn-demo 互斥——同跑无意义叠加,闭集拒绝）。
    skin_demo: Option<&SkinDemoSpec>,
    // G31+ 波 C Task C7：None = profiler 默认关（既有面 0-byte）；Some =
    // --profile-json 输出路径（仅 tsr_device 静态臂 inflight=1 接线,CLI 已裁）。
    profile_json: Option<&str>,
    // G31+ #58：--cluster-lod off（默认）= 既有面 0-byte；leaf|on = 装配后
    // 施加簇 DAG LOD cut（leaf = 全叶逐位对拍锚；on = 屏幕误差驱动 cut）。
    cluster_lod: &ClusterLodOpt,
    // G31+ #95/#68：--wp-hlod off（默认）= 既有面 0-byte；full|on = 装配后
    // 施加 WP cell 流送 + HLOD 互斥选层（full = 全 Full 逐位对拍锚；on =
    // screen-size 阈值互斥切换,远 cell 出 QEM 代理层）。
    wp_hlod: &WpHlodOpt,
    // D2：--smooth-normals（false 默认 = 既有面 0-byte；true = 装配追加顶点
    // 法线侧表 + MegaSmoothNrm 车道 + params[43]=1.0，CLI 已裁互斥面）。
    smooth_normals: bool,
    // D6：--ggx（false 默认 = 既有面 0-byte；true = 装配追加 tri_mr 侧表
    // 〔2 f32/tri〕+ params[48]=1.0；CLI 已裁「须随 --smooth-normals on」
    // 与互斥面——ggx=true 且 smooth_normals=false 组合不可达）。
    ggx: bool,
    // A1：--lamp-lights（enabled=false 默认 = 既有面 0-byte；true = 装配后
    // append 提取代表点光 + params[49]=contrib；CLI 已裁「须随
    // --smooth-normals on」）。
    lamp: &LampOpt,
    // Phase C：--gi2（enabled=false 默认 = 既有面 0-byte；true = MegaTexNrmGi2
    // 形态 + 哑表五件 + params[51..55)；CLI 已裁「须随 --smooth-normals on」
    // 与 inflight=1）。
    gi2: &Gi2Opt,
    // Phase D：--tsr-quality（enabled=false 默认 = 既有面 0-byte；true =
    // tsr_params[19..21) 挂载——resolve SPV 换载在 CLI 面已完成,字节隔离）。
    tsrq: &TsrqOpt,
) {
    let (pre, _) = prelude(scene_id, tier, frames, false, contract_path, expect_digest);
    let contract = &pre.contract;
    let (out_w, out_h, in_w, in_h, seed) = (pre.out_w, pre.out_h, pre.in_w, pre.in_h, pre.seed);
    if frames == 0 {
        fail("--bench --frames 必须 ≥1");
    }

    // D2：--smooth-normals on 走 assemble_scene_nrm 追加顶点法线侧表（off =
    // 既有 assemble_scene 逐字，不读 NORMAL、不产侧表，0-byte）。
    // D6：--ggx on 走 assemble_scene_nrm_mr 再追加 MR 侧表（off = 不读
    // roughnessFactor 进侧表、不产侧表，0-byte）。
    let mut nrm_sink: Vec<f32> = Vec::new();
    let mut mr_sink: Vec<f32> = Vec::new();
    let scene_res = if smooth_normals && ggx {
        assemble_scene_nrm_mr(
            &contract.raw,
            scene_id,
            Path::new(gltf_path),
            &mut nrm_sink,
            &mut mr_sink,
        )
    } else if smooth_normals {
        assemble_scene_nrm(&contract.raw, scene_id, Path::new(gltf_path), &mut nrm_sink)
    } else {
        assemble_scene(&contract.raw, scene_id, Path::new(gltf_path))
    };
    let scene = match scene_res {
        Ok(s) => s,
        Err(e) => dev_env_or_fail("scene_assets", &e),
    };
    // G31+ #58 簇 LOD 施加点（off 时原样直通零改动；leaf/on 时 cut 重建三角汤，
    // 下游 pack/BLAS/kernel 全链零改动——cut 产物即"新的三角汤"）。
    // G36 W2：与 --wp-hlod 双开走组合管线（互斥解除;单开维持既有 apply_*
    // 0-语义漂移——见 render_leg 同注）。
    let geo_both = cluster_lod.mode != ClusterLodMode::Off && wp_hlod.mode != WpHlodMode::Off;
    let (scene, cluster_report, wp_report) = if geo_both {
        let (s, g) = apply_geo_combined(scene, cluster_lod, wp_hlod, in_w, in_h);
        let g = g.expect("双开必有 GeoApplied");
        if let Some(st) = &g.combined {
            eprintln!(
                "{TAG}: geo 组合（cluster×wp）identity={} coarse={}（{} 簇）straddle_fallback={}（{} 簇）wp_proxy={} out={}",
                st.identity_tris,
                st.coarse_tris,
                st.coarse_emitted,
                st.straddle_fallback_tris,
                st.straddle_clusters,
                st.wp_proxy_tris,
                st.out_tris,
            );
        }
        (s, g.cluster, g.wp)
    } else {
        let (s, c) = apply_cluster_lod(scene, cluster_lod, in_w, in_h);
        let (s, w) = apply_wp_hlod(s, wp_hlod);
        (s, c, w)
    };
    if let Some((r, _)) = &cluster_report {
        eprintln!(
            "{TAG}: cluster-lod mode={} threshold_px={} blocks={} clusters={}/{} (leaf_cut={}) tris: src={} passthrough={} leaf_pool={} coarse={} out={} ({:.1}%)",
            r.mode,
            r.threshold_px,
            r.blocks,
            r.cut_clusters,
            r.total_clusters,
            r.cut_leaf_clusters,
            r.src_tris,
            r.passthrough_tris,
            r.leaf_tris,
            r.coarse_tris,
            r.out_tris,
            100.0 * r.out_tris as f64 / r.src_tris.max(1) as f64,
        );
    }
    // G31+ #95/#68 WP/HLOD 施加点（off 时原样直通零改动;full/on 时互斥选层
    // 重建三角汤,下游全链零改动——代理三角随重建进 BLAS 出帧 = #68 HLOD
    // 代理 GPU 绘制腿;G36 W2 双开时已并入上方组合管线）。
    if let Some((r, _)) = &wp_report {
        eprintln!(
            "{TAG}: wp-hlod mode={} cells full/hlod/culled/pending={}/{}/{}/{} (resident={}/{}) tris: src={} passthrough={} full={} proxy={} out={} ({:.1}%) ticks={} stall_frames={} selection_digest={}",
            r.mode,
            r.cells_full,
            r.cells_hlod,
            r.cells_culled,
            r.cells_pending,
            r.cells_resident,
            r.cells_nonempty,
            r.src_tris,
            r.passthrough_tris,
            r.full_tris,
            r.proxy_tris,
            r.out_tris,
            100.0 * r.out_tris as f64 / r.src_tris.max(1) as f64,
            r.assemble_ticks,
            r.budget_stall_frames,
            &r.selection_digest[..16],
        );
    }
    // A1 灯光提取施加点（--lamp-lights on 才 mutate scene.points；off =
    // 直通零触达——render_leg 同律）。
    let scene = if lamp.enabled {
        apply_lamp_lights(scene, lamp)
    } else {
        scene
    };
    let eps = scene_eps(&scene.positions);
    eprintln!(
        "{TAG}: bench 装配 scene={scene_id} tris={} quads={} points={} internal={in_w}x{in_h} output={out_w}x{out_h} eps={eps:.6}",
        scene.tri_count,
        scene.quads.len(),
        scene.points.len(),
    );

    let assets = lane_assets(&scene, in_w, in_h);
    let vp = build_vp(&scene.camera, in_w, in_h);
    let inv_vp = vp.inverse().unwrap_or_else(|| fail("view-proj 必须可逆"));

    // 持续帧循环共同面：warmup + frames 次迭代；测量面 = 后 frames 帧。
    let jitter_base = (seed % JITTER_WINDOW_MOD) as u32;
    let exposure = 2.0f32.powf(-scene.ev100);
    let total = warmup + frames;
    let mut frame_ms: Vec<f64> = Vec::new();
    let mut scene_ms: Vec<f64> = Vec::new();
    let mut mv_ms: Vec<f64> = Vec::new();
    let mut upscale_ms: Vec<f64> = Vec::new();
    let mut scene_gpu_ns: Vec<f64> = Vec::new();
    let mut cpu_record_ns: Vec<f64> = Vec::new();
    let mut cpu_submit_ns: Vec<f64> = Vec::new();
    let mut cpu_fence_wait_ns: Vec<f64> = Vec::new();
    let mut tail_ms: Vec<f64> = Vec::new();
    let mut prod_ms: Vec<f64> = Vec::new();
    let mut last_digest = String::new();
    // C7 profiler 收集面（--profile-json on 才消费;post-warmup 与 frame_ms 同窗;
    // debug label 活跃态随车道创建簿记）。
    let mut profile_frames: Vec<G14ProfileFrame> = Vec::new();
    let mut debug_labels_active = false;
    // G14.10c TSR 访存优化测量臂（env RURIX_TSR_PROBE=1 门控临时探针）：
    // upscale_ms 合并列拆分为 resample/resolve 双 pass GPU 毫秒各自统计
    // （telemetry 本就逐 pass，仅加打印面；常态恒零成本不改 receipt schema）。
    let tsr_probe = std::env::var("RURIX_TSR_PROBE").ok().as_deref() == Some("1");
    let mut resample_probe_ms: Vec<f64> = Vec::new();
    let mut resolve_probe_ms: Vec<f64> = Vec::new();
    // G14.8 flip-trace 诊断臂（RD-045 backfill_condition 字面动作，RFC-0030 §4.2 L1）：
    // env RURIX_G14_FLIP_TRACE=<dir> 时逐帧 digest 轨迹追加写 <dir>/frame_digests.jsonl。
    // 统一车道下测量循环常态零回读——trace 模式强制逐帧回读（诊断模式凌驾性能，
    // frame_ms 含回读税，如实登记不冒充生产口径；vendor 双臂本就逐帧回读，trace
    // 仅多一次文件追加，数据面位级零漂移）。漂移定位分型：首帧漂=冷启/未初始化、
    // 中途单帧漂=拷贝竞争/归约序、漂后链式污染=进历史链。
    let flip_trace: Option<std::io::BufWriter<std::fs::File>> =
        std::env::var("RURIX_G14_FLIP_TRACE").ok().map(|dir| {
            let d = PathBuf::from(&dir);
            std::fs::create_dir_all(&d)
                .unwrap_or_else(|e| fail(&format!("flip-trace 目录 {dir}: {e}")));
            let p = d.join(format!(
                "frame_digests_{scene_id}_t{tier}_{backend_name}.jsonl"
            ));
            std::io::BufWriter::new(
                std::fs::File::create(&p)
                    .unwrap_or_else(|e| fail(&format!("flip-trace 文件 {}: {e}", p.display()))),
            )
        });
    let mut flip_trace = flip_trace;

    // 双臂分叉：tsr_device = 统一四 pass 车道（测量循环零回读，仅末帧回读
    // TSR 输出算 last_frame_digest——同一 GPU 状态机，历史链演化与回读无关，
    // 末帧 digest 与逐帧回读版位级同语义）；dlss_sr/fsr_3_1_5 = 场景 session
    // 逐帧回读 + host mv + vendor host pack 现状结构。
    let (render_lane, timer, caliber): (String, String, String) = if backend_name
        == "tsr_device"
    {
        if let Some(spec) = dyn_demo {
        // ── G31+ 波 A Task A4 动态场景车道（--dyn-demo；本臂全量自持，静态面
        // 0-byte——下方 else 块为既有静态 bench 路径逐字保留）：MegaDyn 四 pass
        // （g31_dyn_scene 实例感知 kernel）+ 2 BLAS（静态场景 + 动态立方体）+
        // 逐帧 tlas_update（实例变换 host 写——槽位级增量仅动态槽 64B——+ TLAS
        // refit/rebuild）走**顺序入口**（inflight=1；A2 约束面 = FIF 流水公共
        // 入口 fail-closed 拒 tlas_update，动态×流水合流归后续波——取舍依据见
        // evidence notes）──
        let bits = UnifiedLaneBits::load(
            &spec.spv_scene,
            spv_mv,
            spv_resample,
            spv_resolve,
            in_w,
            in_h,
            out_w,
            out_h,
            false,
        );
        let assets_dyn = lane_assets_dyn(&scene, in_w, in_h);
        let descs = UnifiedDescs::MegaDyn(unified_lane_descs_dyn(
            &assets_dyn.base,
            &bits,
            in_w,
            in_h,
            out_w,
            out_h,
        ));
        let scene_tri_end = assets_dyn.dyn_tri_base * 9;
        let blas_refs: [&[f32]; 2] = [
            &assets_dyn.base.tris[..scene_tri_end],
            &assets_dyn.dyn_tris,
        ];
        // G38（RFC-0030 v1.1 §4.3 L2a）：inflight>1 ⇒ AS 表 = inflight 份同构
        // 副本组（每表项独立 instance buffer/BLAS/TLAS/scratch——内存 ×S 显式
        // 代价，evidence/预算门登记面）；inflight=1 ⇒ 单表项顺序面 0-byte。
        let slot_as_copies = if inflight > 1 { inflight as usize } else { 1 };
        let accel_structs: Vec<AccelStructDesc<'_>> = (0..slot_as_copies)
            .map(|_| AccelStructDesc {
                scene: RayQuerySceneDesc {
                    blas_triangles: &blas_refs,
                    instances: &assets_dyn.base.instances,
                },
                transforms: None,
                // G31+ 波 B Task B5 字段面:静态/厂商车道无顶点可更新 BLAS(0-byte)。
                updatable_blas: &[],
            })
            .collect();
        let mut lane = match if inflight > 1 {
            UnifiedTsrLane::create_with_slot_as(&descs, &accel_structs, inflight as usize)
        } else {
            UnifiedTsrLane::create(&descs, &accel_structs, 1)
        } {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        let action = if spec.refit {
            TlasBuildAction::Refit
        } else {
            TlasBuildAction::Rebuild
        };
        eprintln!(
            "{TAG}: bench 动态场景车道就绪 warmup={warmup} frames={frames}（MegaDyn 四 pass；dyn_tris={} dyn_tri_base={}；TLAS {} 逐帧；位置核验每 {DYN_VERIFY_EVERY} 帧）",
            assets_dyn.dyn_tris.len() / 9,
            assets_dyn.dyn_tri_base,
            if spec.refit { "refit" } else { "rebuild" },
        );
        let origin = dyn_trajectory_origin(&scene.camera);
        let mut verify_recs: Vec<DynVerifyFrame> = Vec::new();
        // G38：核验帧组装（顺序/slot_as FIF 两循环**同一事实源**——轨迹/相机
        // 均帧号纯函数，闭包内由帧号复算，与循环内既有值逐位同源；输入 =
        // rec.scene_color〔核验帧回读〕+ 帧号）。原顺序循环内联块逐字搬移，
        // 行为 0 变。
        let push_verify = |verify_recs: &mut Vec<DynVerifyFrame>,
                           rec: &UnifiedFrameRec,
                           i: u32| {
            let j = [
                halton(jitter_base + i + 1, 2) - 0.5,
                halton(jitter_base + i + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let (pos, yaw) = dyn_trajectory(i, origin);
            let xf = dyn_transform_3x4(pos, yaw);
            let scene_color = rec
                .scene_color
                .as_ref()
                .unwrap_or_else(|| fail("bench 帧核验面缺 scene color 回读（内部破缺）"));
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
                // 世界角点 = R·lp + t（xf 行主 3×4）。
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
            let pass = obs_count >= min_count
                && centroid_delta <= DYN_TOL_CENTROID_PX
                && aabb_delta <= DYN_TOL_AABB_PX;
            verify_recs.push(DynVerifyFrame {
                frame: i,
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
        };
        if lane.inflight > 1 {
            // ── G38（RFC-0030 v1.1 §4.3 L2a）动态臂 slot_as FIF 流水测量循环：
            // 骨架与静态 A2 FIF 分支同律（submit(k) 后不等当帧 fence，pending
            // 满 inflight 即 FIFO collect(k+1−inflight)；测量循环零常态回读；
            // 排空段墙钟并入末一测量样本，prod = frame − tail 不变式保持）。
            // 差异恰两处：① 逐帧 tlas_update 与 scene pass AS 绑定落 base+slot
            // 副本表项（rt 槽纪律三判据提交前 fail-closed）；② 核验帧组装延迟
            // 到 collect 侧凭帧号复算（push_verify 同一事实源——FIFO 保序，
            // 帧号随票据）。
            let fif_depth = lane.inflight;
            let mut drain_wait_ms = 0.0f64;
            let mut drain_tail_ms = 0.0f64;
            for i in 0..total {
                let t_frame = std::time::Instant::now();
                let j = [
                    halton(jitter_base + i + 1, 2) - 0.5,
                    halton(jitter_base + i + 1, 3) - 0.5,
                ];
                let vp_j = jittered_vp(&vp, j, in_w, in_h);
                // 逐帧实例变换（脚本化轨迹：帧号纯函数——与顺序臂逐位同源）。
                let (pos, yaw) = dyn_trajectory(i, origin);
                let xf = dyn_transform_3x4(pos, yaw);
                let scene_params = pack_frame_params_dyn(
                    in_w,
                    in_h,
                    j,
                    eps,
                    scene.quads.len(),
                    scene.points.len(),
                    &inv_vp,
                    &vp,
                    assets_dyn.dyn_tri_base,
                );
                let verify = i >= warmup && (i - warmup) % DYN_VERIFY_EVERY == 0;
                let readback_out = flip_trace.is_some() || i + 1 == total;
                if let Err(e) = lane.submit_frame_dyn_slot_as(
                    in_w,
                    in_h,
                    out_w,
                    out_h,
                    j,
                    &vp_j,
                    exposure,
                    i == 0,
                    scene_params,
                    dyn_frame_instances(xf),
                    action,
                    readback_out,
                    verify,
                    i,
                ) {
                    fail(&format!("bench 帧 {i} 动态 slot_as submit: {e}"));
                }
                let rec = if lane.pending_dyn_len() == fif_depth {
                    Some(match lane.collect_frame_dyn(out_w, out_h, in_w, in_h) {
                        Ok(r) => r,
                        Err(e) => fail(&format!(
                            "bench 动态 slot_as collect（提交序 {}）: {e}",
                            i + 1 - fif_depth as u32
                        )),
                    })
                } else {
                    None
                };
                let t_tail = std::time::Instant::now();
                let mut tail_convert_ms = 0.0f64;
                if let Some(rec) = &rec {
                    if rec.validation_error_count != 0 {
                        fail(&format!(
                            "bench 动态 slot_as validation ERROR 计数 {} ≠ 0",
                            rec.validation_error_count
                        ));
                    }
                    tail_convert_ms = rec.readback_convert_ms;
                    if let Some(out_data) = rec.out_color.as_ref() {
                        if !out_data.iter().all(|v| v.is_finite()) {
                            fail(&format!(
                                "bench 帧 {} 动态 slot_as upscale 输出非有限",
                                rec.frame_index
                            ));
                        }
                        last_digest = frame_content_digest(out_w, out_h, 3, out_data);
                        if let Some(w) = flip_trace.as_mut() {
                            use std::io::Write as _;
                            let fi = rec.frame_index;
                            writeln!(w, "{{\"frame\":{fi},\"digest\":\"{last_digest}\"}}")
                                .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                        }
                    }
                    // 核验帧（scene color 随票据回读）——凭帧号复算轨迹/相机。
                    if rec.scene_color.is_some() {
                        push_verify(&mut verify_recs, rec, rec.frame_index);
                    }
                }
                let tail_el = t_tail.elapsed().as_secs_f64() * 1000.0 + tail_convert_ms;
                let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
                if i >= warmup {
                    frame_ms.push(frame_el);
                    scene_ms.push(rec.as_ref().map_or(0.0, |r| r.scene_gpu_ns / 1e6));
                    mv_ms.push(rec.as_ref().map_or(0.0, |r| r.mv_gpu_ns / 1e6));
                    upscale_ms.push(
                        rec.as_ref()
                            .map_or(0.0, |r| (r.resample_gpu_ns + r.resolve_gpu_ns) / 1e6),
                    );
                    scene_gpu_ns.push(rec.as_ref().map_or(0.0, |r| r.scene_gpu_ns));
                    cpu_record_ns.push(rec.as_ref().map_or(0.0, |r| r.cpu_record_ns as f64));
                    cpu_submit_ns.push(rec.as_ref().map_or(0.0, |r| r.cpu_submit_ns as f64));
                    cpu_fence_wait_ns
                        .push(rec.as_ref().map_or(0.0, |r| r.cpu_fence_wait_ns as f64));
                    tail_ms.push(tail_el);
                    prod_ms.push(frame_el - tail_el);
                }
                if i == 0 || (i + 1) % 20 == 0 || i + 1 == total {
                    eprintln!(
                        "{TAG}: bench 帧 {}/{total} frame={frame_el:.3}ms（动态 slot_as {} inflight={fif_depth} pending={} 轨迹 x={:.3} y={:.3} z={:.3} yaw={:.3}）",
                        i + 1,
                        if spec.refit { "refit" } else { "rebuild" },
                        lane.pending_dyn_len(),
                        pos[0],
                        pos[1],
                        pos[2],
                        yaw,
                    );
                }
            }
            // 排空段：FIFO 收干在飞票据（inflight−1 帧）；末帧 digest 与残余
            // 核验帧在此落账（FIFO 保序）；墙钟并入末一测量样本（静态 A2 FIF
            // 分支同律登记口径）。
            while lane.pending_dyn_len() > 0 {
                let t_drain = std::time::Instant::now();
                let rec = match lane.collect_frame_dyn(out_w, out_h, in_w, in_h) {
                    Ok(r) => r,
                    Err(e) => fail(&format!("bench 动态 slot_as 排空 collect: {e}")),
                };
                drain_wait_ms += t_drain.elapsed().as_secs_f64() * 1000.0;
                if rec.validation_error_count != 0 {
                    fail(&format!(
                        "bench 动态 slot_as validation ERROR 计数 {} ≠ 0",
                        rec.validation_error_count
                    ));
                }
                let t_tail = std::time::Instant::now();
                if let Some(out_data) = rec.out_color.as_ref() {
                    if !out_data.iter().all(|v| v.is_finite()) {
                        fail(&format!(
                            "bench 帧 {} 动态 slot_as upscale 输出非有限",
                            rec.frame_index
                        ));
                    }
                    last_digest = frame_content_digest(out_w, out_h, 3, out_data);
                    if let Some(w) = flip_trace.as_mut() {
                        use std::io::Write as _;
                        let fi = rec.frame_index;
                        writeln!(w, "{{\"frame\":{fi},\"digest\":\"{last_digest}\"}}")
                            .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                    }
                }
                if rec.scene_color.is_some() {
                    push_verify(&mut verify_recs, &rec, rec.frame_index);
                }
                drain_tail_ms +=
                    t_tail.elapsed().as_secs_f64() * 1000.0 + rec.readback_convert_ms;
            }
            // 排空段墙钟并入末一测量样本（口径见循环头登记）。
            if let Some(last) = frame_ms.last_mut() {
                *last += drain_wait_ms + drain_tail_ms;
            }
            if let Some(last) = tail_ms.last_mut() {
                *last += drain_tail_ms;
            }
            if let Some(last) = prod_ms.last_mut() {
                *last += drain_wait_ms;
            }
        } else {
        for i in 0..total {
            let t_frame = std::time::Instant::now();
            let j = [
                halton(jitter_base + i + 1, 2) - 0.5,
                halton(jitter_base + i + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            // 逐帧实例变换（脚本化轨迹：平移 + Ry yaw；确定性 f32 纯帧号函数）。
            let (pos, yaw) = dyn_trajectory(i, origin);
            let xf = dyn_transform_3x4(pos, yaw);
            let scene_params = pack_frame_params_dyn(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                assets_dyn.dyn_tri_base,
            );
            let verify = i >= warmup && (i - warmup) % DYN_VERIFY_EVERY == 0;
            let readback_out = flip_trace.is_some() || i + 1 == total;
            let rec = match lane.frame_dyn(
                in_w,
                in_h,
                out_w,
                out_h,
                j,
                &vp_j,
                exposure,
                i == 0,
                scene_params,
                (0, dyn_frame_instances(xf), action),
                readback_out,
                verify,
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("bench 帧 {i} 动态场景车道: {e}")),
            };
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "bench 帧 {i} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            let t_tail = std::time::Instant::now();
            if let Some(out_data) = rec.out_color.as_ref() {
                if !out_data.iter().all(|v| v.is_finite()) {
                    fail(&format!("bench 帧 {i} upscale 输出非有限"));
                }
                last_digest = frame_content_digest(out_w, out_h, 3, out_data);
                if let Some(w) = flip_trace.as_mut() {
                    use std::io::Write as _;
                    writeln!(w, "{{\"frame\":{i},\"digest\":\"{last_digest}\"}}")
                        .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                }
            }
            // 动态实例位置核验（host 参考臂 = 解析投影 vs device scene color
            // 纯绿谱检测——TSR 前瞬时位无拖影；组装 = push_verify 同一事实源
            // 〔G38：顺序/slot_as FIF 共用,原内联块逐字迁入闭包,行为 0 变〕；
            // tail 段测量面如实计入）。
            if verify {
                push_verify(&mut verify_recs, &rec, i);
            }
            let tail_el = t_tail.elapsed().as_secs_f64() * 1000.0 + rec.readback_convert_ms;
            let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
            if i >= warmup {
                frame_ms.push(frame_el);
                scene_ms.push(rec.scene_gpu_ns / 1e6);
                mv_ms.push(rec.mv_gpu_ns / 1e6);
                upscale_ms.push((rec.resample_gpu_ns + rec.resolve_gpu_ns) / 1e6);
                scene_gpu_ns.push(rec.scene_gpu_ns);
                cpu_record_ns.push(rec.cpu_record_ns as f64);
                cpu_submit_ns.push(rec.cpu_submit_ns as f64);
                cpu_fence_wait_ns.push(rec.cpu_fence_wait_ns as f64);
                tail_ms.push(tail_el);
                prod_ms.push(frame_el - tail_el);
            }
            if i == 0 || (i + 1) % 20 == 0 || i + 1 == total {
                eprintln!(
                    "{TAG}: bench 帧 {}/{total} frame={frame_el:.3}ms（动态场景 {} 轨迹 x={:.3} y={:.3} z={:.3} yaw={:.3}）",
                    i + 1,
                    if spec.refit { "refit" } else { "rebuild" },
                    pos[0],
                    pos[1],
                    pos[2],
                    yaw,
                );
            }
        }
        }
        // ── 位置核验汇总：dyn_verify.json 落盘（证据保全先于判红）+ fail-closed ──
        let all_pass = !verify_recs.is_empty() && verify_recs.iter().all(|r| r.pass);
        let out_dir_dyn = PathBuf::from(out_root)
            .join(scene_id)
            .join(format!("tier{tier}"))
            .join(backend_name);
        std::fs::create_dir_all(&out_dir_dyn)
            .unwrap_or_else(|e| fail(&format!("输出目录: {e}")));
        let verify_path = out_dir_dyn.join("dyn_verify.json");
        let jf = |v: f64| -> String {
            if v.is_finite() {
                format!("{v:.6}")
            } else {
                "\"inf\"".to_owned()
            }
        };
        let mut rows = String::new();
        for (ri, r) in verify_recs.iter().enumerate() {
            if ri > 0 {
                rows.push_str(",\n");
            }
            rows.push_str(&format!(
                "   {{\"frame\": {}, \"transform\": [{}], \"pred_px\": [{}, {}], \"pred_aabb\": [{}, {}, {}, {}], \"obs_px\": [{}, {}], \"obs_aabb\": [{}, {}, {}, {}], \"obs_count\": {}, \"centroid_delta_px\": {}, \"aabb_delta_px\": {}, \"pass\": {}}}",
                r.frame,
                r.transform
                    .iter()
                    .map(|x| format!("{x:.9e}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                jf(r.pred_px[0]),
                jf(r.pred_px[1]),
                jf(r.pred_aabb[0]),
                jf(r.pred_aabb[1]),
                jf(r.pred_aabb[2]),
                jf(r.pred_aabb[3]),
                jf(r.obs_px[0]),
                jf(r.obs_px[1]),
                jf(r.obs_aabb[0]),
                jf(r.obs_aabb[1]),
                jf(r.obs_aabb[2]),
                jf(r.obs_aabb[3]),
                r.obs_count,
                jf(r.centroid_delta_px),
                jf(r.aabb_delta_px),
                r.pass,
            ));
        }
        rows.push('\n');
        let verify_doc = format!(
            "{{\n  \"schema\": \"rurix.g31.dyn_scene_verify.v1\",\n  \"action\": {},\n  \"scene_id\": {},\n  \"tier\": {},\n  \"backend\": {},\n  \"trajectory\": {{\"amp\": [{}, {}, {}], \"freq\": [{}, {}, {}], \"yaw_rate\": {}, \"origin\": [{}, {}, {}], \"cube_half\": {}, \"emission\": [{}, {}, {}]}},\n  \"tolerance\": {{\"centroid_px\": {}, \"aabb_px\": {}, \"min_count_area_ratio\": {}}},\n  \"frames\": [\n{rows}  ],\n  \"frames_verified\": {},\n  \"all_pass\": {}\n}}\n",
            jstr(if spec.refit { "refit" } else { "rebuild" }),
            jstr(scene_id),
            tier,
            jstr(backend_name),
            DYN_AMP[0],
            DYN_AMP[1],
            DYN_AMP[2],
            DYN_FREQ[0],
            DYN_FREQ[1],
            DYN_FREQ[2],
            DYN_YAW_RATE,
            origin[0],
            origin[1],
            origin[2],
            DYN_CUBE_HALF,
            DYN_EMISSION[0],
            DYN_EMISSION[1],
            DYN_EMISSION[2],
            DYN_TOL_CENTROID_PX,
            DYN_TOL_AABB_PX,
            DYN_MIN_COUNT_AREA_RATIO,
            verify_recs.len(),
            all_pass,
        );
        std::fs::write(&verify_path, verify_doc)
            .unwrap_or_else(|e| fail(&format!("dyn_verify 落盘: {e}")));
        eprintln!(
            "{TAG}: dyn 核验 {}/{} 帧通过（质心 ≤{DYN_TOL_CENTROID_PX}px AABB ≤{DYN_TOL_AABB_PX}px）→ {}",
            verify_recs.iter().filter(|r| r.pass).count(),
            verify_recs.len(),
            verify_path.display(),
        );
        if !all_pass {
            fail(&format!(
                "动态实例位置核验失败（帧详情见 {}）",
                verify_path.display()
            ));
        }
        (
            // G38：inflight=1 描述字面 0-byte（receipt 面既有内容逐字保留）；
            // inflight>1 = slot_as 形态如实登记（L2a）。
            if lane.inflight > 1 {
                format!(
                    "G31+ 波 A Task A4 动态场景车道：MegaDyn 统一四 pass（g31_dyn_scene 实例感知 kernel，committed_instance_index 分派）+ 2 BLAS（静态场景 + 动态纯发光立方体）+ 逐帧 tlas_update（实例变换 host 写槽位级增量——仅动态槽 64B——+ TLAS refit/rebuild）slot_as FIF 流水入口（G38 RFC-0030 v1.1 §4.3 L2a：inflight={}，session AS 表 ×{} 同构副本组 + submit_with_frame_update_slot_as，逐帧更新与 scene AS 绑定落 base+slot 槽副本；确定性判据 = 逐帧 digest 序列与顺序基线逐字节相等）",
                    lane.inflight, lane.inflight
                )
            } else {
                "G31+ 波 A Task A4 动态场景车道：MegaDyn 统一四 pass（g31_dyn_scene 实例感知 kernel，committed_instance_index 分派）+ 2 BLAS（静态场景 + 动态纯发光立方体）+ 逐帧 tlas_update（实例变换 host 写槽位级增量——仅动态槽 64B——+ TLAS refit/rebuild）顺序入口（inflight=1，FIF 流水面拒 tlas_update 的 A2 约束登记）".to_owned()
            },
            "host Instant 墙钟 + DeviceFrameTelemetry（逐 pass GPU timestamp + cpu_record/submit/fence_wait 分项）；frame_ms 含逐帧 TLAS 更新 GPU 段（fence 内）+ 核验帧 scene color 回读税（tail 如实计量）".to_owned(),
            "G31+ Task A4 口径：动态实例 = 纯发光立方体（albedo=0, emission=[0,500,0]，12 三角形局部空间 BLAS 1），脚本化轨迹 = 帧号确定性 f32 函数（三轴正弦平移 + Ry 匀速 yaw）；位置核验 = host 解析投影（轨迹点 + 8 角点经 vp_j）vs device scene color 谱检测质心/AABB（容差 2.5/4.0px）；digest 面 = TSR 输出末帧（与静态 bench 同语义同管线）".to_owned(),
        )
        } else if let Some(skin) = skin_demo {
        // ── G31+ 波 B Task B5 蒙皮角色车道（--skin-demo；本臂全量自持,静态/
        // dyn 面 0-byte——下方 else 块为既有静态 bench 路径逐字保留）:
        // MegaSkin 五 pass（pass0 = kernels/g31_skin.rx device 蒙皮 LBS 求值 →
        // [FrameUpdate::blas_refit 桥:vkCmdCopyBuffer 蒙皮段 → 角色 BLAS 顶点
        // 缓冲 + 原地 UPDATE build + consume barrier] → pass1 =
        // kernels/g31_skin_scene.rx（g31_dyn_scene 镜像 + 命中信息通道）→
        // pass2 = kernels/g31_skin_mv.rx（g14_mv 镜像 + 蒙皮 MV 覆盖臂,
        // RD-041 类 3）→ pass3/4 = TSR 双 pass）+ 2 BLAS（静态场景 + 蒙皮
        // 角色,角色 BLAS 创建期 updatable 打标）+ 逐帧骨骼 palette 双表上传
        // （骨骼矩阵逐帧上传面）走**顺序入口**（inflight=1;A2/FIF 同律约束
        // ——BLAS 顶点缓冲为共享写面,FIF 入口已拒 blas_refit）──
        let origin = skin_origin(&scene.camera);
        let assets_skin = lane_assets_skin(&scene, in_w, in_h, origin);
        let bits = SkinLaneBits::load(
            skin,
            spv_resample,
            spv_resolve,
            in_w,
            in_h,
            out_w,
            out_h,
            assets_skin.character.vertex_count,
        );
        let descs = UnifiedDescs::MegaSkin(unified_lane_descs_skin(
            &assets_skin,
            &bits,
            in_w,
            in_h,
            out_w,
            out_h,
        ));
        let scene_tri_end = assets_skin.skin_tri_base * 9;
        let blas_refs: [&[f32]; 2] = [
            &assets_skin.base.tris[..scene_tri_end],
            &assets_skin.character.rest_tris,
        ];
        const SKIN_UPDATABLE_BLAS: [u32; 1] = [1];
        // G38（RFC-0030 v1.1 §4.3 L2a 批次 B）：inflight>1 ⇒ AS 表 = inflight
        // 份同构副本组（每表项独立 instance buffer/BLAS/TLAS/scratch，角色
        // BLAS 顶点副本随表项 updatable 打标 ×S——内存 ×S 显式代价，evidence/
        // 预算门登记面）；inflight=1 ⇒ 单表项顺序面 0-byte。
        let slot_as_copies = if inflight > 1 { inflight as usize } else { 1 };
        let accel_structs: Vec<AccelStructDesc<'_>> = (0..slot_as_copies)
            .map(|_| AccelStructDesc {
                scene: RayQuerySceneDesc {
                    blas_triangles: &blas_refs,
                    instances: &assets_skin.base.instances,
                },
                transforms: None,
                updatable_blas: &SKIN_UPDATABLE_BLAS,
            })
            .collect();
        let mut lane = match if inflight > 1 {
            UnifiedTsrLane::create_with_slot_as(&descs, &accel_structs, inflight as usize)
        } else {
            UnifiedTsrLane::create(&descs, &accel_structs, 1)
        } {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        let blas = BlasRefitUpdate {
            as_index: 0,
            blas_index: 1,
            src: StableResourceId(u64::from(U_TRIS) + 1),
            src_offset: (assets_skin.skin_tri_base * 36) as u64,
            byte_len: (assets_skin.character.tri_count * 36) as u64,
            after_pass: 0,
        };
        eprintln!(
            "{TAG}: bench 蒙皮角色车道就绪 warmup={warmup} frames={frames}（MegaSkin 五 pass;skin_tris={} skin_verts={} skin_tri_base={} bones=3;BLAS 1 updatable 逐帧 refit;位置/MV 核验每 {DYN_VERIFY_EVERY} 帧）",
            assets_skin.character.tri_count,
            assets_skin.character.vertex_count,
            assets_skin.skin_tri_base,
        );
        let mut verify_recs: Vec<SkinVerifyFrame> = Vec::new();
        let mut prev_pal: Option<[BoneTransform; 3]> = None;
        // G38 批次 B 注：原 prev_vp_host 循环态已删——唯一消费方（核验块）迁入
        // push_verify 闭包后凭帧号复算上帧相机（jittered_vp∘halton 纯函数），
        // 死状态置留必致 unused 告警；palette 双表上传仍走 prev_pal 循环态。
        let mut skin_probe_ms: Vec<f64> = Vec::new();
        // G38 批次 B：核验帧组装（顺序/slot_as FIF 两循环**同一事实源**——骨骼
        // palette/相机均帧号纯函数，闭包内由帧号复算，与循环内既有值逐位同源；
        // 输入 = rec 核验回读三路〔mv/scene/hit,可选 debug_tris〕+ 帧号）。原
        // 顺序循环内联块逐字搬移（缩进原位保持），行为 0 变。
        let push_verify =
            |verify_recs: &mut Vec<SkinVerifyFrame>, rec: &SkinFrameRec, i: u32| {
                let j = [
                    halton(jitter_base + i + 1, 2) - 0.5,
                    halton(jitter_base + i + 1, 3) - 0.5,
                ];
                let vp_j = jittered_vp(&vp, j, in_w, in_h);
                // 骨骼 palette 帧号纯函数复算（与循环内上传值逐位同源;核验帧
                // 恒 i ≥ 1——i=0 分支为 prev_pal.unwrap_or(pal) 的防御性镜像）。
                let pal = skin_palette(i, origin);
                let prev = if i == 0 { pal } else { skin_palette(i - 1, origin) };
                let scene_color = rec
                    .scene_color
                    .as_ref()
                    .unwrap_or_else(|| fail("bench 帧核验面缺 scene color 回读（内部破缺）"));
                let mv_plane = rec
                    .mv_out
                    .as_ref()
                    .unwrap_or_else(|| fail("bench 帧核验面缺 mv 回读（内部破缺）"));
                let hit_plane = rec
                    .hit
                    .as_ref()
                    .unwrap_or_else(|| fail("bench 帧核验面缺 hit 回读（内部破缺）"));
                let host_cur = skin_host_verts(&assets_skin.character, &pal);
                let host_prev_pos = skin_host_verts(&assets_skin.character, &prev);
                // 蒙皮输出对拍诊断臂:device 蒙皮顶点(tris 角色段回读)vs host
                // skin_vertex 逐顶点 max-abs——kernel 真值归因(预期 ulp 级;
                // 超阈即 device/host 输入面或 kernel 语义破缺)。
                if let Some(dt) = rec.debug_tris.as_ref() {
                    let mut max_abs = 0.0f64;
                    let mut max_vi = 0usize;
                    for (vi, hv) in host_cur.iter().enumerate() {
                        for c in 0..3 {
                            let d = (f64::from(dt[vi * 3 + c]) - f64::from(hv[c])).abs();
                            if d > max_abs {
                                max_abs = d;
                                max_vi = vi;
                            }
                        }
                    }
                    eprintln!(
                        "{TAG}: SKIN_DEBUG_TRIS 帧 {i} device vs host max_abs={max_abs:.9e} @v{max_vi}(dev=({:.6},{:.6},{:.6}) host=({:.6},{:.6},{:.6})) len={}",
                        dt[max_vi * 3],
                        dt[max_vi * 3 + 1],
                        dt[max_vi * 3 + 2],
                        host_cur[max_vi][0],
                        host_cur[max_vi][1],
                        host_cur[max_vi][2],
                        dt.len(),
                    );
                    // 全量 dump(前 12 顶点 + 末 3 顶点;回读内容形态取证)。
                    for vi in [0usize, 1, 2, 3, 36, 37, 72, 73, 105, 106, 107] {
                        if vi < host_cur.len() {
                            eprintln!(
                                "{TAG}: SKIN_DEBUG_TRIS 帧 {i} v{vi} dev=({:.4},{:.4},{:.4}) host=({:.4},{:.4},{:.4})",
                                dt[vi * 3],
                                dt[vi * 3 + 1],
                                dt[vi * 3 + 2],
                                host_cur[vi][0],
                                host_cur[vi][1],
                                host_cur[vi][2],
                            );
                        }
                    }
                    // 分段质心对照(上臂/前臂/融合套各段 device vs host 均值)。
                    for (seg, name) in [(0usize, "上臂"), (36, "前臂"), (72, "融合套")] {
                        let mut dc = [0.0f64; 3];
                        let mut hc = [0.0f64; 3];
                        for vi in seg..seg + 36 {
                            for c in 0..3 {
                                dc[c] += f64::from(dt[vi * 3 + c]);
                                hc[c] += f64::from(host_cur[vi][c]);
                            }
                        }
                        eprintln!(
                            "{TAG}: SKIN_DEBUG_TRIS 帧 {i} 段{name} dev=({:.4},{:.4},{:.4}) host=({:.4},{:.4},{:.4})",
                            dc[0] / 36.0, dc[1] / 36.0, dc[2] / 36.0,
                            hc[0] / 36.0, hc[1] / 36.0, hc[2] / 36.0,
                        );
                    }
                }
                // 上帧相机 = 帧号纯函数复算（原 prev_vp_host.unwrap_or(vp_j)
                // 同语义:核验帧恒 i ≥ 1,i=0 分支为 unwrap_or 的防御性镜像）。
                let prev_vp_h = if i == 0 {
                    vp_j
                } else {
                    jittered_vp(
                        &vp,
                        [
                            halton(jitter_base + i, 2) - 0.5,
                            halton(jitter_base + i, 3) - 0.5,
                        ],
                        in_w,
                        in_h,
                    )
                };
                let mut host_mv: Vec<[f64; 2]> =
                    Vec::with_capacity(assets_skin.character.vertex_count);
                for k in 0..assets_skin.character.vertex_count {
                    let (u, v) = dyn_project(&vp_j, host_cur[k], in_w, in_h)
                        .unwrap_or_else(|| fail("蒙皮顶点投影在相机背面（动画规格破缺）"));
                    let (pu, pv) = dyn_project(&prev_vp_h, host_prev_pos[k], in_w, in_h)
                        .unwrap_or_else(|| fail("蒙皮 prev 顶点投影在相机背面（动画规格破缺）"));
                    host_mv.push([pu - u, pv - v]);
                    let _ = (u, v);
                }
                // pred 面 = 蒙皮后三角形**投影并集掩码**(折叠姿态屏幕重叠域
                // 与 ray query inst==1 地面真值逐像素同语义——质心/AABB 同源
                // 可比;顶点均值/面积加权对折叠分布近似差过大,实测调定后弃用)。
                let (pred_cx, pred_cy, pred_aabb, pred_mask_count) = skin_pred_mask(
                    &host_cur,
                    assets_skin.character.tri_count,
                    &vp_j,
                    in_w,
                    in_h,
                )
                .unwrap_or_else(|| fail("蒙皮掩码投影为空（动画规格破缺）"));
                let pred_c = [pred_cx, pred_cy];
                // 检测 = 命中信息通道 inst==1 地面真值（ray query 提交实例
                // 下标;精确角色像素集,无谱近似——品红发射保留作视觉唯一性
                // 与画面内容标识,检测真值以 inst 为准,登记口径）。
                let obs = skin_detect_hit(hit_plane, in_w, in_h, 1.0);
                // 调试探针（RURIX_SKIN_DUMP=<dir>：核验帧 scene color 吨映
                // PNG 落盘——遮挡/检测归因取证用,常态空挂零成本）。
                if let Ok(dump_dir) = std::env::var("RURIX_SKIN_DUMP") {
                    let mut buf = ImageBuffer::new(in_w, in_h, Rgb::new(0.0, 0.0, 0.0));
                    for (k, chunk) in scene_color.chunks_exact(3).enumerate() {
                        let ton = |v: f32| -> f32 {
                            if v.is_finite() && v > 0.0 {
                                v / (1.0 + v)
                            } else {
                                0.0
                            }
                        };
                        buf.set(
                            (k as u32) % in_w,
                            (k as u32) / in_w,
                            Rgb::new(ton(chunk[0]), ton(chunk[1]), ton(chunk[2])),
                        );
                    }
                    let bytes = encode_image(&buf, ImageFormat::Png)
                        .unwrap_or_else(|e| fail(&format!("dump PNG 编码: {e}")));
                    let dir = PathBuf::from(&dump_dir);
                    std::fs::create_dir_all(&dir).unwrap_or_else(|e| fail(&format!("dump 目录: {e}")));
                    let p = dir.join(format!("skin_scene_f{i}.png"));
                    std::fs::write(&p, &bytes)
                        .unwrap_or_else(|e| fail(&format!("dump 落盘 {}: {e}", p.display())));
                    // host 参照顶点投影叠图（上臂段 = 绿点,前臂段 = 红点,融合套
                    // = 蓝点,3×3 块——host 预期位置 vs device 实渲染的逐段
                    // 对照取证面;pred AABB 黄框）。仅 dump 模式消费。
                    for k in 0..assets_skin.character.vertex_count {
                        let (u, v) = dyn_project(&vp_j, host_cur[k], in_w, in_h)
                            .unwrap_or((f64::NAN, f64::NAN));
                        if !u.is_finite() || !v.is_finite() {
                            continue;
                        }
                        let seg = k / 36; // 0=上臂 1=前臂 2=融合套(12 三角形×3 顶点)
                        let dot = match seg {
                            0 => Rgb::new(0.0, 1.0, 0.0),
                            1 => Rgb::new(1.0, 0.0, 0.0),
                            _ => Rgb::new(0.0, 0.2, 1.0),
                        };
                        let (cx0, cy0) = (u.round() as i32, v.round() as i32);
                        for ddy in -1..=1i32 {
                            for ddx in -1..=1i32 {
                                let (xx, yy) = (cx0 + ddx, cy0 + ddy);
                                if xx >= 0 && yy >= 0 && (xx as u32) < in_w && (yy as u32) < in_h
                                {
                                    buf.set(xx as u32, yy as u32, dot);
                                }
                            }
                        }
                    }
                    let bytes2 = encode_image(&buf, ImageFormat::Png)
                        .unwrap_or_else(|e| fail(&format!("dump PNG 编码: {e}")));
                    let p2 = dir.join(format!("skin_overlay_f{i}.png"));
                    std::fs::write(&p2, &bytes2)
                        .unwrap_or_else(|e| fail(&format!("dump 落盘 {}: {e}", p2.display())));
                }
                let (obs_px, obs_aabb, obs_count, obs_idx) = match obs {
                    Some((cx, cy, bb, n, idx)) => ([cx, cy], bb, n, idx),
                    None => ([f64::NAN; 2], [f64::NAN; 4], 0, Vec::new()),
                };
                let centroid_delta = if obs_count > 0 {
                    ((obs_px[0] - pred_c[0]).powi(2) + (obs_px[1] - pred_c[1]).powi(2)).sqrt()
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
                // 像素数门 = max(200, 0.75×掩码计数)——掩码计数即 host 参照
                // 期望像素数(投影并集),75% 余量吸收 jitter 边效应（周长/面积
                // 比实测 ~3%）与关节盒共面叠合(远严于 A4 面积比门;场景遮挡
                // 丢失面（40%+ 大额缺失）同门检出）。
                let min_count = 200.0f64.max(0.75 * pred_mask_count as f64) as usize;
                // MV 域统计:dev 检测像素域中位数 vs host 逐顶点中位数（分量）。
                let (fw, fh) = (in_w as f64, in_h as f64);
                let mut dx: Vec<f64> = obs_idx
                    .iter()
                    .map(|&pi| f64::from(mv_plane[pi as usize * 2]) * fw)
                    .collect();
                let mut dy: Vec<f64> = obs_idx
                    .iter()
                    .map(|&pi| f64::from(mv_plane[pi as usize * 2 + 1]) * fh)
                    .collect();
                let dev_med = if dx.is_empty() {
                    [f64::NAN; 2]
                } else {
                    [median_f64(&mut dx), median_f64(&mut dy)]
                };
                let mut dmag: Vec<f64> = obs_idx
                    .iter()
                    .map(|&pi| {
                        let mx = f64::from(mv_plane[pi as usize * 2]) * fw;
                        let my = f64::from(mv_plane[pi as usize * 2 + 1]) * fh;
                        mx.hypot(my)
                    })
                    .collect();
                let mut hx: Vec<f64> = host_mv.iter().map(|m| m[0]).collect();
                let mut hy: Vec<f64> = host_mv.iter().map(|m| m[1]).collect();
                let host_med = [median_f64(&mut hx), median_f64(&mut hy)];
                let mut hmag: Vec<f64> = host_mv.iter().map(|m| m[0].hypot(m[1])).collect();
                let host_motion = median_f64(&mut hmag);
                let dev_motion = if dmag.is_empty() {
                    f64::NAN
                } else {
                    median_f64(&mut dmag)
                };
                let mv_delta = [
                    (dev_med[0] - host_med[0]).abs(),
                    (dev_med[1] - host_med[1]).abs(),
                ];
                // 静态区 MV（左上 32×32 背景窗;覆盖臂污染检查面）。
                let (sx0, sy0, sx1, sy1) = SKIN_STATIC_WIN;
                let mut smag: Vec<f64> = Vec::new();
                for py in sy0..sy1 {
                    for px in sx0..sx1 {
                        let pi = (py * in_w + px) as usize;
                        let mx = f64::from(mv_plane[pi * 2]) * fw;
                        let my = f64::from(mv_plane[pi * 2 + 1]) * fh;
                        smag.push(mx.hypot(my));
                    }
                }
                let static_med = median_f64(&mut smag);
                // 逐帧门 = 位置/计数 + MV 逐分量差 + 静态区无污染 + **条件
                // ratio**（高动帧才激活:host_motion ≥ 1.0px ⇒ dev ≥ 0.5×host
                // ——低动相位（谐波对消,合法动画相位）host≈dev≈亚像素,信噪比
                // 低于相机 jitter 残留,绝对差门已绑;真动判据归窗级聚合门,
                // 见 all_pass——逐帧 ≥1.0px 硬门会把合法低动相位误判坏内容,
                // 实测 frame 1/2/14 med 0.76/0.48/0.94px 即此类）。
                let pass = obs_count >= min_count
                    && centroid_delta <= SKIN_TOL_CENTROID_PX
                    && aabb_delta <= SKIN_TOL_AABB_PX
                    && mv_delta[0] <= SKIN_MV_TOL_MEDIAN_PX
                    && mv_delta[1] <= SKIN_MV_TOL_MEDIAN_PX
                    && (host_motion < SKIN_MV_HOST_MOTION_MIN_PX
                        || dev_motion >= SKIN_MV_DEV_RATIO_MIN * host_motion)
                    && static_med <= SKIN_MV_STATIC_MAX_PX;
                let mut pal36 = [0.0f32; 36];
                for (bi, b) in pal.iter().enumerate() {
                    for r in 0..3 {
                        for c in 0..4 {
                            pal36[bi * 12 + r * 4 + c] = b[r][c];
                        }
                    }
                }
                verify_recs.push(SkinVerifyFrame {
                    frame: i,
                    palette: pal36,
                    pred_px: pred_c,
                    pred_aabb,
                    obs_px,
                    obs_aabb,
                    obs_count,
                    centroid_delta_px: centroid_delta,
                    aabb_delta_px: aabb_delta,
                    mv_dev_median_px: dev_med,
                    mv_host_median_px: host_med,
                    mv_median_delta_px: mv_delta,
                    mv_host_motion_px: host_motion,
                    mv_dev_motion_px: dev_motion,
                    static_mv_median_abs_px: static_med,
                    pass,
                });
            };
        if lane.inflight > 1 {
            // ── G38（RFC-0030 v1.1 §4.3 L2a 批次 B）蒙皮臂 slot_as FIF 流水
            // 测量循环：骨架与动态臂 slot_as FIF 分支同律（submit(k) 后不等
            // 当帧 fence，pending 满 inflight 即 FIFO collect(k+1−inflight)；
            // 测量循环零常态回读；排空段墙钟并入末一测量样本，prod = frame −
            // tail 不变式保持）。差异恰两处：① 无 tlas_update——逐帧
            // blas_refit 目标与 skin scene pass（1）AS 绑定落 base+slot 副本
            // 表项（角色 BLAS 顶点副本逐表项 updatable 打标；rt 槽纪律三判据
            // 提交前 fail-closed）；② 核验帧组装延迟到 collect 侧凭帧号复算
            // （push_verify 同一事实源——骨骼 palette 帧号纯函数，FIFO 保序，
            // 帧号随票据）。
            let fif_depth = lane.inflight;
            let mut drain_wait_ms = 0.0f64;
            let mut drain_tail_ms = 0.0f64;
            for i in 0..total {
                let t_frame = std::time::Instant::now();
                let j = [
                    halton(jitter_base + i + 1, 2) - 0.5,
                    halton(jitter_base + i + 1, 3) - 0.5,
                ];
                let vp_j = jittered_vp(&vp, j, in_w, in_h);
                // 逐帧骨骼 palette（脚本化骨骼动画:确定性 f32 纯帧号函数——
                // root 位移 + 肩/肘双摆;双跑/跨轮位级同序列）。
                let pal = skin_palette(i, origin);
                let prev = prev_pal.unwrap_or(pal);
                let scene_params = pack_frame_params_dyn(
                    in_w,
                    in_h,
                    j,
                    eps,
                    scene.quads.len(),
                    scene.points.len(),
                    &inv_vp,
                    &vp,
                    assets_skin.skin_tri_base,
                );
                let skin_params = pack_skin_params(
                    assets_skin.character.vertex_count,
                    i > 0,
                    assets_skin.skin_tri_base,
                    assets_skin.character.bone_count,
                );
                let verify = i >= 1 && i >= warmup && (i - warmup) % DYN_VERIFY_EVERY == 0;
                let readback_out = flip_trace.is_some() || i + 1 == total;
                // 蒙皮输出对拍诊断臂（env RURIX_SKIN_DEBUG_TRIS=1;仅核验帧挂
                // debug 回读,常态恒 false 零成本）。
                let debug_tris = verify && std::env::var("RURIX_SKIN_DEBUG_TRIS").is_ok();
                if let Err(e) = lane.submit_frame_skin_slot_as(
                    in_w,
                    in_h,
                    out_w,
                    out_h,
                    j,
                    &vp_j,
                    exposure,
                    i == 0,
                    scene_params,
                    skin_params,
                    skin_palette_bytes(&pal),
                    skin_palette_bytes(&prev),
                    blas,
                    readback_out,
                    verify,
                    debug_tris,
                    i,
                ) {
                    fail(&format!("bench 帧 {i} 蒙皮 slot_as submit: {e}"));
                }
                prev_pal = Some(pal);
                let rec = if lane.pending_skin_len() == fif_depth {
                    Some(match lane.collect_frame_skin(out_w, out_h, in_w, in_h) {
                        Ok(r) => r,
                        Err(e) => fail(&format!(
                            "bench 蒙皮 slot_as collect（提交序 {}）: {e}",
                            i + 1 - fif_depth as u32
                        )),
                    })
                } else {
                    None
                };
                let t_tail = std::time::Instant::now();
                let mut tail_convert_ms = 0.0f64;
                if let Some(rec) = &rec {
                    if rec.validation_error_count != 0 {
                        fail(&format!(
                            "bench 蒙皮 slot_as validation ERROR 计数 {} ≠ 0",
                            rec.validation_error_count
                        ));
                    }
                    tail_convert_ms = rec.readback_convert_ms;
                    if let Some(out_data) = rec.out_color.as_ref() {
                        if !out_data.iter().all(|v| v.is_finite()) {
                            fail(&format!(
                                "bench 帧 {} 蒙皮 slot_as upscale 输出非有限",
                                rec.frame_index
                            ));
                        }
                        last_digest = frame_content_digest(out_w, out_h, 3, out_data);
                        if let Some(w) = flip_trace.as_mut() {
                            use std::io::Write as _;
                            let fi = rec.frame_index;
                            writeln!(w, "{{\"frame\":{fi},\"digest\":\"{last_digest}\"}}")
                                .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                        }
                    }
                    // 核验帧（mv/scene/hit 随票据回读）——凭帧号复算 palette/相机。
                    if rec.scene_color.is_some() {
                        push_verify(&mut verify_recs, rec, rec.frame_index);
                    }
                }
                let tail_el = t_tail.elapsed().as_secs_f64() * 1000.0 + tail_convert_ms;
                let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
                if i >= warmup {
                    frame_ms.push(frame_el);
                    scene_ms.push(rec.as_ref().map_or(0.0, |r| r.scene_gpu_ns / 1e6));
                    mv_ms.push(rec.as_ref().map_or(0.0, |r| r.mv_gpu_ns / 1e6));
                    upscale_ms.push(
                        rec.as_ref()
                            .map_or(0.0, |r| (r.resample_gpu_ns + r.resolve_gpu_ns) / 1e6),
                    );
                    skin_probe_ms.push(rec.as_ref().map_or(0.0, |r| r.skin_gpu_ns / 1e6));
                    scene_gpu_ns.push(rec.as_ref().map_or(0.0, |r| r.scene_gpu_ns));
                    cpu_record_ns.push(rec.as_ref().map_or(0.0, |r| r.cpu_record_ns as f64));
                    cpu_submit_ns.push(rec.as_ref().map_or(0.0, |r| r.cpu_submit_ns as f64));
                    cpu_fence_wait_ns
                        .push(rec.as_ref().map_or(0.0, |r| r.cpu_fence_wait_ns as f64));
                    tail_ms.push(tail_el);
                    prod_ms.push(frame_el - tail_el);
                }
                if i == 0 || (i + 1) % 20 == 0 || i + 1 == total {
                    eprintln!(
                        "{TAG}: bench 帧 {}/{total} frame={frame_el:.3}ms（蒙皮 slot_as FIF inflight={fif_depth} pending={} root=({:.3},{:.3},{:.3})）",
                        i + 1,
                        lane.pending_skin_len(),
                        pal[0][0][3],
                        pal[0][1][3],
                        pal[0][2][3],
                    );
                }
            }
            // 排空段：FIFO 收干在飞票据（inflight−1 帧）；末帧 digest 与残余
            // 核验帧在此落账（FIFO 保序）；墙钟并入末一测量样本（静态 A2 FIF
            // 分支同律登记口径）。
            while lane.pending_skin_len() > 0 {
                let t_drain = std::time::Instant::now();
                let rec = match lane.collect_frame_skin(out_w, out_h, in_w, in_h) {
                    Ok(r) => r,
                    Err(e) => fail(&format!("bench 蒙皮 slot_as 排空 collect: {e}")),
                };
                drain_wait_ms += t_drain.elapsed().as_secs_f64() * 1000.0;
                if rec.validation_error_count != 0 {
                    fail(&format!(
                        "bench 蒙皮 slot_as validation ERROR 计数 {} ≠ 0",
                        rec.validation_error_count
                    ));
                }
                let t_tail = std::time::Instant::now();
                if let Some(out_data) = rec.out_color.as_ref() {
                    if !out_data.iter().all(|v| v.is_finite()) {
                        fail(&format!(
                            "bench 帧 {} 蒙皮 slot_as upscale 输出非有限",
                            rec.frame_index
                        ));
                    }
                    last_digest = frame_content_digest(out_w, out_h, 3, out_data);
                    if let Some(w) = flip_trace.as_mut() {
                        use std::io::Write as _;
                        let fi = rec.frame_index;
                        writeln!(w, "{{\"frame\":{fi},\"digest\":\"{last_digest}\"}}")
                            .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                    }
                }
                if rec.scene_color.is_some() {
                    push_verify(&mut verify_recs, &rec, rec.frame_index);
                }
                drain_tail_ms +=
                    t_tail.elapsed().as_secs_f64() * 1000.0 + rec.readback_convert_ms;
            }
            // 排空段墙钟并入末一测量样本（口径见循环头登记）。
            if let Some(last) = frame_ms.last_mut() {
                *last += drain_wait_ms + drain_tail_ms;
            }
            if let Some(last) = tail_ms.last_mut() {
                *last += drain_tail_ms;
            }
            if let Some(last) = prod_ms.last_mut() {
                *last += drain_wait_ms;
            }
        } else {
        for i in 0..total {
            let t_frame = std::time::Instant::now();
            let j = [
                halton(jitter_base + i + 1, 2) - 0.5,
                halton(jitter_base + i + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            // 逐帧骨骼 palette（脚本化骨骼动画:确定性 f32 纯帧号函数——
            // root 位移 + 肩/肘双摆;双跑/跨轮位级同序列）。
            let pal = skin_palette(i, origin);
            let prev = prev_pal.unwrap_or(pal);
            let scene_params = pack_frame_params_dyn(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                assets_skin.skin_tri_base,
            );
            let skin_params = pack_skin_params(
                assets_skin.character.vertex_count,
                i > 0,
                assets_skin.skin_tri_base,
                assets_skin.character.bone_count,
            );
            let verify = i >= 1 && i >= warmup && (i - warmup) % DYN_VERIFY_EVERY == 0;
            let readback_out = flip_trace.is_some() || i + 1 == total;
            // 蒙皮输出对拍诊断臂（env RURIX_SKIN_DEBUG_TRIS=1;仅核验帧挂
            // debug 回读,常态恒 false 零成本）。
            let debug_tris = verify && std::env::var("RURIX_SKIN_DEBUG_TRIS").is_ok();
            let rec = match lane.frame_skin(
                in_w,
                in_h,
                out_w,
                out_h,
                j,
                &vp_j,
                exposure,
                i == 0,
                scene_params,
                skin_params,
                skin_palette_bytes(&pal),
                skin_palette_bytes(&prev),
                blas,
                readback_out,
                verify,
                debug_tris,
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("bench 帧 {i} 蒙皮角色车道: {e}")),
            };
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "bench 帧 {i} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            let t_tail = std::time::Instant::now();
            if let Some(out_data) = rec.out_color.as_ref() {
                if !out_data.iter().all(|v| v.is_finite()) {
                    fail(&format!("bench 帧 {i} upscale 输出非有限"));
                }
                last_digest = frame_content_digest(out_w, out_h, 3, out_data);
                if let Some(w) = flip_trace.as_mut() {
                    use std::io::Write as _;
                    writeln!(w, "{{\"frame\":{i},\"digest\":\"{last_digest}\"}}")
                        .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                }
            }
            // 蒙皮角色位置 + MV 核验（host 参照臂 = skin_vertex 蒙皮全顶点
            // + 解析投影;device 面 = scene color 品红谱检测 + MV 通道回读
            // ——TSR 前瞬时位无历史拖影;组装 = push_verify 同一事实源〔G38
            // 批次 B:顺序/slot_as FIF 共用,原内联块逐字迁入闭包,行为 0 变〕;
            // tail 段测量面如实计入）。
            if verify {
                push_verify(&mut verify_recs, &rec, i);
            }
            prev_pal = Some(pal);
            let tail_el = t_tail.elapsed().as_secs_f64() * 1000.0 + rec.readback_convert_ms;
            let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
            if i >= warmup {
                frame_ms.push(frame_el);
                scene_ms.push(rec.scene_gpu_ns / 1e6);
                mv_ms.push(rec.mv_gpu_ns / 1e6);
                upscale_ms.push((rec.resample_gpu_ns + rec.resolve_gpu_ns) / 1e6);
                skin_probe_ms.push(rec.skin_gpu_ns / 1e6);
                scene_gpu_ns.push(rec.scene_gpu_ns);
                cpu_record_ns.push(rec.cpu_record_ns as f64);
                cpu_submit_ns.push(rec.cpu_submit_ns as f64);
                cpu_fence_wait_ns.push(rec.cpu_fence_wait_ns as f64);
                tail_ms.push(tail_el);
                prod_ms.push(frame_el - tail_el);
            }
            if i == 0 || (i + 1) % 20 == 0 || i + 1 == total {
                eprintln!(
                    "{TAG}: bench 帧 {}/{total} frame={frame_el:.3}ms（蒙皮 skin_gpu={:.4}ms scene_gpu={:.3}ms mv_gpu={:.3}ms root=({:.3},{:.3},{:.3})）",
                    i + 1,
                    rec.skin_gpu_ns / 1e6,
                    rec.scene_gpu_ns / 1e6,
                    rec.mv_gpu_ns / 1e6,
                    pal[0][0][3],
                    pal[0][1][3],
                    pal[0][2][3],
                );
            }
        }
        }
        // ── skin 段 post-warmup 统计（骨骼逐帧更新 GPU 成本归因面;receipt
        // schema 无 skin 列——stderr 登记 + evidence 消费）。──
        if !skin_probe_ms.is_empty() {
            let mean = skin_probe_ms.iter().sum::<f64>() / skin_probe_ms.len() as f64;
            let min = skin_probe_ms.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = skin_probe_ms
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            eprintln!("{TAG}: SKIN_GPU_MS mean={mean:.6} min={min:.6} max={max:.6}");
        }
        // ── 核验汇总:skin_verify.json 落盘(证据保全先于判红)+ fail-closed ──
        // 窗级聚合真动门（防「确定性的坏内容」= 动画未真动/MV 通道未载运动）:
        // 核验窗内 max host_motion ≥ 1.0px——低动相位逐帧不误伤,窗内必含高动
        // 帧（双谐波不可通约 ⇒ max 实测 ~3px 远离零面）;高动帧上条件 ratio 门
        // 已激活,未接线通道（dev ≈ 相机残留亚像素）在该帧必红。
        let motion_max = verify_recs
            .iter()
            .map(|r| r.mv_host_motion_px)
            .fold(0.0f64, f64::max);
        let all_pass = !verify_recs.is_empty()
            && verify_recs.iter().all(|r| r.pass)
            && motion_max >= SKIN_MV_HOST_MOTION_MIN_PX;
        let out_dir_skin = PathBuf::from(out_root)
            .join(scene_id)
            .join(format!("tier{tier}"))
            .join(backend_name);
        std::fs::create_dir_all(&out_dir_skin)
            .unwrap_or_else(|e| fail(&format!("输出目录: {e}")));
        let verify_path = out_dir_skin.join("skin_verify.json");
        let jf = |v: f64| -> String {
            if v.is_finite() {
                format!("{v:.6}")
            } else {
                "\"inf\"".to_owned()
            }
        };
        let mut rows = String::new();
        for (ri, r) in verify_recs.iter().enumerate() {
            if ri > 0 {
                rows.push_str(",\n");
            }
            rows.push_str(&format!(
                "   {{\"frame\": {}, \"palette\": [{}], \"pred_px\": [{}, {}], \"pred_aabb\": [{}, {}, {}, {}], \"obs_px\": [{}, {}], \"obs_aabb\": [{}, {}, {}, {}], \"obs_count\": {}, \"centroid_delta_px\": {}, \"aabb_delta_px\": {}, \"mv_dev_median_px\": [{}, {}], \"mv_host_median_px\": [{}, {}], \"mv_median_delta_px\": [{}, {}], \"mv_host_motion_px\": {}, \"mv_dev_motion_px\": {}, \"static_mv_median_abs_px\": {}, \"pass\": {}}}",
                r.frame,
                r.palette
                    .iter()
                    .map(|x| format!("{x:.9e}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                jf(r.pred_px[0]),
                jf(r.pred_px[1]),
                jf(r.pred_aabb[0]),
                jf(r.pred_aabb[1]),
                jf(r.pred_aabb[2]),
                jf(r.pred_aabb[3]),
                jf(r.obs_px[0]),
                jf(r.obs_px[1]),
                jf(r.obs_aabb[0]),
                jf(r.obs_aabb[1]),
                jf(r.obs_aabb[2]),
                jf(r.obs_aabb[3]),
                r.obs_count,
                jf(r.centroid_delta_px),
                jf(r.aabb_delta_px),
                jf(r.mv_dev_median_px[0]),
                jf(r.mv_dev_median_px[1]),
                jf(r.mv_host_median_px[0]),
                jf(r.mv_host_median_px[1]),
                jf(r.mv_median_delta_px[0]),
                jf(r.mv_median_delta_px[1]),
                jf(r.mv_host_motion_px),
                jf(r.mv_dev_motion_px),
                jf(r.static_mv_median_abs_px),
                r.pass,
            ));
        }
        rows.push('\n');
        let verify_doc = format!(
            "{{\n  \"schema\": \"rurix.g31.skin_verify.v1\",\n  \"scene_id\": {},\n  \"tier\": {},\n  \"backend\": {},\n  \"animation\": {{\"root_amp\": [{}, {}, {}], \"root_freq\": [{}, {}, {}], \"root_amp2\": [{}, {}, {}], \"root_freq2\": [{}, {}, {}], \"root_phase2\": [{}, {}, {}], \"swing_amp\": {}, \"swing_freq\": {}, \"swing_amp2\": {}, \"swing_freq2\": {}, \"swing_phase2\": {}, \"elbow_amp\": {}, \"elbow_freq\": {}, \"elbow_amp2\": {}, \"elbow_freq2\": {}, \"elbow_phase2\": {}, \"upper_len\": {}, \"origin\": [{}, {}, {}], \"bone_count\": {}, \"tri_count\": {}, \"vertex_count\": {}, \"emission\": [{}, {}, {}], \"albedo\": [{}, {}, {}]}},\n  \"tolerance\": {{\"centroid_px\": {}, \"aabb_px\": {}, \"mv_median_px\": {}, \"min_count_area_ratio\": {}, \"mv_host_motion_min_px\": {}, \"mv_dev_ratio_min\": {}, \"static_mv_max_px\": {}}},\n  \"frames\": [\n{rows}  ],\n  \"frames_verified\": {},\n  \"motion_gate\": {{\"host_motion_max_px\": {}, \"threshold_px\": {}, \"note\": {}}},\n  \"all_pass\": {}\n}}\n",
            jstr(scene_id),
            tier,
            jstr(backend_name),
            SKIN_ROOT_AMP[0],
            SKIN_ROOT_AMP[1],
            SKIN_ROOT_AMP[2],
            SKIN_ROOT_FREQ[0],
            SKIN_ROOT_FREQ[1],
            SKIN_ROOT_FREQ[2],
            SKIN_ROOT_AMP2[0],
            SKIN_ROOT_AMP2[1],
            SKIN_ROOT_AMP2[2],
            SKIN_ROOT_FREQ2[0],
            SKIN_ROOT_FREQ2[1],
            SKIN_ROOT_FREQ2[2],
            SKIN_ROOT_PHASE2[0],
            SKIN_ROOT_PHASE2[1],
            SKIN_ROOT_PHASE2[2],
            SKIN_SWING_AMP,
            SKIN_SWING_FREQ,
            SKIN_SWING_AMP2,
            SKIN_SWING_FREQ2,
            SKIN_SWING_PHASE2,
            SKIN_ELBOW_AMP,
            SKIN_ELBOW_FREQ,
            SKIN_ELBOW_AMP2,
            SKIN_ELBOW_FREQ2,
            SKIN_ELBOW_PHASE2,
            SKIN_UPPER_LEN,
            origin[0],
            origin[1],
            origin[2],
            assets_skin.character.bone_count,
            assets_skin.character.tri_count,
            assets_skin.character.vertex_count,
            SKIN_EMISSION[0],
            SKIN_EMISSION[1],
            SKIN_EMISSION[2],
            SKIN_ALBEDO[0],
            SKIN_ALBEDO[1],
            SKIN_ALBEDO[2],
            SKIN_TOL_CENTROID_PX,
            SKIN_TOL_AABB_PX,
            SKIN_MV_TOL_MEDIAN_PX,
            DYN_MIN_COUNT_AREA_RATIO,
            SKIN_MV_HOST_MOTION_MIN_PX,
            SKIN_MV_DEV_RATIO_MIN,
            SKIN_MV_STATIC_MAX_PX,
            verify_recs.len(),
            jf(motion_max),
            SKIN_MV_HOST_MOTION_MIN_PX,
            jstr("窗级聚合真动门:max host_motion ≥ threshold（低动相位逐帧不误伤;高动帧条件 ratio 门 dev ≥0.5×host 已激活）"),
            all_pass,
        );
        std::fs::write(&verify_path, verify_doc)
            .unwrap_or_else(|e| fail(&format!("skin_verify 落盘: {e}")));
        eprintln!(
            "{TAG}: skin 核验 {}/{} 帧通过（质心 ≤{SKIN_TOL_CENTROID_PX}px AABB ≤{SKIN_TOL_AABB_PX}px MV 中位差 ≤{SKIN_MV_TOL_MEDIAN_PX}px 窗级真动 max={motion_max:.3}px ≥{SKIN_MV_HOST_MOTION_MIN_PX}px）→ {}",
            verify_recs.iter().filter(|r| r.pass).count(),
            verify_recs.len(),
            verify_path.display(),
        );
        if !all_pass {
            fail(&format!(
                "蒙皮角色位置/MV 核验失败（帧详情见 {}）",
                verify_path.display()
            ));
        }
        (
            // G38 批次 B：inflight=1 描述字面 0-byte（receipt 面既有内容逐字
            // 保留）；inflight>1 = slot_as 形态如实登记（L2a）。
            if lane.inflight > 1 {
                format!(
                    "G31+ 波 B Task B5 蒙皮角色车道：MegaSkin 统一五 pass（pass0 kernels/g31_skin.rx device LBS 蒙皮（骨骼 palette 双表逐帧 buffer_uploads 上传,cur/prev 双求值写 tris SSBO 角色段 + prev 顶点表）→ FrameUpdate::blas_refit 桥（vkCmdCopyBuffer 蒙皮段 → 角色 BLAS 顶点缓冲 + 原地 UPDATE build + consume barrier,创建期 updatable 打标 ALLOW_UPDATE）→ pass1 kernels/g31_skin_scene.rx（g31_dyn_scene 镜像 + inst/prim/bary 命中信息通道,ray query 当帧蒙皮几何 + 形变阴影）→ pass2 kernels/g31_skin_mv.rx（g14_mv 镜像 + RD-041 类 3 蒙皮 MV 覆盖臂:prev 顶点 bary 插值 → prev_vp 投影,TSR 历史链接通）→ pass3/4 TSR parity 双 pass）slot_as FIF 流水入口（G38 RFC-0030 v1.1 §4.3 L2a 批次 B：inflight={}，session AS 表 ×{} 同构副本组〔角色 BLAS 顶点副本逐表项 updatable 打标〕+ submit_with_frame_update_slot_as，逐帧 blas_refit 目标与 skin scene pass AS 绑定落 base+slot 槽副本；确定性判据 = 逐帧 digest 序列与顺序基线逐字节相等,refit 非纯时按 L2a「按槽稳定」降档显式登记）",
                    lane.inflight, lane.inflight
                )
            } else {
                "G31+ 波 B Task B5 蒙皮角色车道：MegaSkin 统一五 pass（pass0 kernels/g31_skin.rx device LBS 蒙皮（骨骼 palette 双表逐帧 buffer_uploads 上传,cur/prev 双求值写 tris SSBO 角色段 + prev 顶点表）→ FrameUpdate::blas_refit 桥（vkCmdCopyBuffer 蒙皮段 → 角色 BLAS 顶点缓冲 + 原地 UPDATE build + consume barrier,创建期 updatable 打标 ALLOW_UPDATE）→ pass1 kernels/g31_skin_scene.rx（g31_dyn_scene 镜像 + inst/prim/bary 命中信息通道,ray query 当帧蒙皮几何 + 形变阴影）→ pass2 kernels/g31_skin_mv.rx（g14_mv 镜像 + RD-041 类 3 蒙皮 MV 覆盖臂:prev 顶点 bary 插值 → prev_vp 投影,TSR 历史链接通）→ pass3/4 TSR parity 双 pass）顺序入口（inflight=1,FIF 流水面拒 blas_refit 的 A2 同律登记）".to_owned()
            },
            "host Instant 墙钟 + DeviceFrameTelemetry（逐 pass GPU timestamp + cpu_record/submit/fence_wait 分项）；frame_ms 含逐帧蒙皮 pass + BLAS refit GPU 段（fence 内）+ 核验帧 scene color/mv 双回读税（tail 如实计量）；skin/scene/mv GPU 三分解经 stderr SKIN_GPU_MS 行登记（骨骼逐帧更新成本归因）".to_owned(),
            "G31+ Task B5 口径：蒙皮角色 = 3 骨两段臂 + 关节融合套（36 三角形盒体网格,albedo=[0.18,0.18,0.20] emission=[400,0,400] 品红检测唯一谱）,脚本化骨骼动画 = 帧号确定性 f32 函数（root 三轴正弦位移 + 肩/肘 z 摆,world-from-bind 约定无逆绑定面）;位置核验 = host skin_vertex 全顶点解析投影 vs device scene color 谱检测质心/AABB（容差 4.0/6.0px,分布近似差实测调定）;MV 核验 = 检测像素域 dev 中位数 vs host 逐顶点中位数（逐分量容差 2.0px + 窗级聚合真动门 max host ≥1.0px + 高动帧条件 ratio 门 dev ≥0.5×host + 静态区 ≤1.5px）;digest 面 = TSR 输出末帧（与静态 bench 同语义同管线）;类 2 刚性实例 MV 维持 A4 登记缺口（本车道无刚性动态实例,不冒充接通）".to_owned(),
        )
        } else {
        // G14.10b 形态选择：cornell 拆散六 pass（quad_count==1 且零点光——16 层
        // 映射单灯语义 fail-closed 前置断言）；其余 Mega 四 pass。
        let use_split = scene.quads.len() == 1
            && scene.points.is_empty()
            && !spv_scene.replace('\\', "/").contains("g16_gi_multibounce");
        // D2：平滑法线臂仅 Mega 形态接线（cornell Split 拆散车道无 trinrm
        // 绑定面，fail-closed 兜底；CLI 互斥面已裁）。
        if smooth_normals && use_split {
            fail("--smooth-normals on 仅 Mega 形态已接线（cornell Split 拆散车道无 trinrm 面，fail-closed）");
        }
        let bits = UnifiedLaneBits::load(
            spv_scene, spv_mv, spv_resample, spv_resolve, in_w, in_h, out_w, out_h, use_split,
        );
        // D2：法线侧表字节面（off = 空 vec 零成本不消费；on = 9 f32/tri 与
        // 装配三角数互核 fail-closed）。
        let nrm_bytes = if smooth_normals {
            let b = bytes_f32(&nrm_sink);
            if b.len() != scene.tri_count * 9 * 4 {
                fail(&format!(
                    "法线侧表长度 {} ≠ tri_count×9×4 = {}（装配/施加点互核 fail-closed）",
                    b.len(),
                    scene.tri_count * 9 * 4
                ));
            }
            b
        } else {
            Vec::new()
        };
        // D6：MR 侧表字节面（--ggx on = 2 f32/tri 真表互核 fail-closed；
        // --ggx off 但 smooth-normals on = 8B 零哑表——kernel params[48]=0
        // 门均匀分支不读，哑表零消费；!smooth_normals = 空 vec 不消费）。
        let mr_bytes = if ggx {
            let b = bytes_f32(&mr_sink);
            if b.len() != scene.tri_count * 2 * 4 {
                fail(&format!(
                    "MR 侧表长度 {} ≠ tri_count×2×4 = {}（装配/施加点互核 fail-closed）",
                    b.len(),
                    scene.tri_count * 2 * 4
                ));
            }
            b
        } else if smooth_normals {
            vec![0u8; 8]
        } else {
            Vec::new()
        };
        // Phase C：GI2 哑表五件（--gi2 on 才构建；off = 零分配零触达）。
        let gi2_dummy = if gi2.enabled {
            Some(gi2_dummy_tex(scene.tri_count))
        } else {
            None
        };
        let descs = if use_split {
            UnifiedDescs::Split(unified_lane_descs_split(&assets, &bits, in_w, in_h, out_w, out_h))
        } else if gi2.enabled {
            // Phase C：--gi2 on = MegaTexNrmGi2 形态（统一质量 kernel + 哑表
            // 五件；CLI 已裁「须随 --smooth-normals on」⇒ nrm/mr 侧表已产）。
            UnifiedDescs::MegaTexNrmGi2(unified_lane_descs_texnrm_gi2(
                &assets,
                &bits,
                &nrm_bytes,
                &mr_bytes,
                gi2_dummy.as_ref().unwrap(),
                in_w,
                in_h,
                out_w,
                out_h,
            ))
        } else if smooth_normals {
            UnifiedDescs::MegaSmoothNrm(unified_lane_descs_nrm(
                &assets, &bits, &nrm_bytes, &mr_bytes, in_w, in_h, out_w, out_h,
            ))
        } else {
            UnifiedDescs::Mega(unified_lane_descs(&assets, &bits, in_w, in_h, out_w, out_h))
        };
        let blas_refs: [&[f32]; 1] = [&assets.tris];
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets.instances,
            },
            transforms: None,
            // G31+ 波 B Task B5 字段面:静态/厂商车道无顶点可更新 BLAS(0-byte)。
            updatable_blas: &[],
        }];
        let mut lane = match UnifiedTsrLane::create(&descs, &accel_structs, inflight as usize)
        {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        // D6：--ggx on → params[48]=1.0（MegaSmoothNrm 形态 + tri_mr 真表
        // 已绑；off 车道不挂载 ⇒ 参数面 0-byte）。
        if ggx {
            lane.set_ggx(true);
        }
        // A1：--lamp-lights on → params[49]=contrib（off 车道不挂载 ⇒ 参数
        // 面 0-byte）。
        if lamp.enabled {
            lane.set_lamp_contrib(lamp.contrib);
        }
        // Phase C：--gi2 on → params[51]=1/[53]=clamp/[54]=scale（off 不挂载
        // ⇒ 四槽不写参数面 0-byte）；[52]=frame_idx 逐帧挂载见测量循环
        // （CLI 已裁 inflight=1——顺序循环独达）。
        if gi2.enabled {
            lane.set_gi2(gi2.scale, gi2.clamp);
        }
        // Phase D：--tsr-quality on → tsr_params[19]=min_alpha/[20]=clamp K
        // （off 不挂载 ⇒ 两槽不写参数面 0-byte）。
        if tsrq.enabled {
            lane.set_tsrq(tsrq.min_alpha, tsrq.clamp);
        }
        // C7:debug label 活跃态簿记（--profile-json 消费;session 创建期定格）。
        debug_labels_active = lane.session.debug_labels_active();
        eprintln!(
            "{TAG}: bench 统一四 pass 车道就绪 warmup={warmup} frames={frames} inflight={}（session 不销毁；测量循环零回读，末帧回读 TSR 输出；flip_trace={}）",
            lane.inflight,
            flip_trace.is_some()
        );
        if lane.inflight > 1 {
            // ── G31（波 A Task A2）FIF 流水测量循环：submit(k) 后不等当帧
            // fence，pending 满 inflight 即 FIFO collect(k+1−inflight)；末帧
            // readback/digest 随票据延迟到 collect（FIFO ⇒ 帧序不乱）；
            // 测量循环零回读与顺序面同律。frame_ms[i] = 迭代墙钟（submit +
            // 命中 collect）；排空段（inflight−1 帧）collect 等待墙钟并入
            // 末一测量样本（Σframe_ms ≈ 测量段全墙钟，诚实吞吐口径；
            // drain 段 digest tail 并入末一样本 tail_ms，prod=frame−tail
            // 不变式保持）——证据 schema g31_frame_pipelining 登记口径。
            let fif_depth = lane.inflight;
            let mut drain_wait_ms = 0.0f64;
            let mut drain_tail_ms = 0.0f64;
            for i in 0..total {
                let t_frame = std::time::Instant::now();
                let j = [
                    halton(jitter_base + i + 1, 2) - 0.5,
                    halton(jitter_base + i + 1, 3) - 0.5,
                ];
                let vp_j = jittered_vp(&vp, j, in_w, in_h);
                let readback_out = flip_trace.is_some() || i + 1 == total;
                if let Err(e) = lane.submit_frame(
                    in_w,
                    in_h,
                    out_w,
                    out_h,
                    j,
                    eps,
                    scene.quads.len(),
                    scene.points.len(),
                    &inv_vp,
                    &vp,
                    &vp_j,
                    exposure,
                    i == 0,
                    readback_out,
                    i,
                ) {
                    fail(&format!("bench 帧 {i} 流水 submit: {e}"));
                }
                let rec = if lane.pending_len() == fif_depth {
                    Some(match lane.collect_frame(out_w, out_h) {
                        Ok(r) => r,
                        Err(e) => fail(&format!(
                            "bench 流水 collect（提交序 {}）: {e}",
                            i + 1 - fif_depth as u32
                        )),
                    })
                } else {
                    None
                };
                let t_tail = std::time::Instant::now();
                let mut tail_convert_ms = 0.0f64;
                if let Some(rec) = &rec {
                    if rec.validation_error_count != 0 {
                        fail(&format!(
                            "bench 流水 validation ERROR 计数 {} ≠ 0",
                            rec.validation_error_count
                        ));
                    }
                    tail_convert_ms = rec.readback_convert_ms;
                    if let Some(out_data) = rec.out_color.as_ref() {
                        if !out_data.iter().all(|v| v.is_finite()) {
                            fail(&format!("bench 帧 {} 流水 upscale 输出非有限", rec.frame_index));
                        }
                        last_digest = frame_content_digest(out_w, out_h, 3, out_data);
                        if let Some(w) = flip_trace.as_mut() {
                            use std::io::Write as _;
                            let fi = rec.frame_index;
                            writeln!(w, "{{\"frame\":{fi},\"digest\":\"{last_digest}\"}}")
                                .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                        }
                    }
                }
                let tail_el = t_tail.elapsed().as_secs_f64() * 1000.0 + tail_convert_ms;
                let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
                if i >= warmup {
                    frame_ms.push(frame_el);
                    scene_ms.push(rec.as_ref().map_or(0.0, |r| r.scene_gpu_ns / 1e6));
                    mv_ms.push(rec.as_ref().map_or(0.0, |r| r.mv_gpu_ns / 1e6));
                    upscale_ms.push(
                        rec.as_ref()
                            .map_or(0.0, |r| (r.resample_gpu_ns + r.resolve_gpu_ns) / 1e6),
                    );
                    scene_gpu_ns.push(rec.as_ref().map_or(0.0, |r| r.scene_gpu_ns));
                    cpu_record_ns.push(rec.as_ref().map_or(0.0, |r| r.cpu_record_ns as f64));
                    cpu_submit_ns.push(rec.as_ref().map_or(0.0, |r| r.cpu_submit_ns as f64));
                    cpu_fence_wait_ns.push(rec.as_ref().map_or(0.0, |r| r.cpu_fence_wait_ns as f64));
                    tail_ms.push(tail_el);
                    prod_ms.push(frame_el - tail_el);
                }
                if i == 0 || (i + 1) % 20 == 0 || i + 1 == total {
                    eprintln!(
                        "{TAG}: bench 帧 {}/{total} frame={frame_el:.3}ms（FIF inflight={fif_depth} pending={}）",
                        i + 1,
                        lane.pending_len()
                    );
                }
            }
            // 排空段：FIFO 收干在飞票据（inflight−1 帧）；末帧 digest 在此
            // 落账（帧 total−1 最后 collect——FIFO 保序）。
            while lane.pending_len() > 0 {
                let t_drain = std::time::Instant::now();
                let rec = match lane.collect_frame(out_w, out_h) {
                    Ok(r) => r,
                    Err(e) => fail(&format!("bench 流水排空 collect: {e}")),
                };
                drain_wait_ms += t_drain.elapsed().as_secs_f64() * 1000.0;
                if rec.validation_error_count != 0 {
                    fail(&format!(
                        "bench 流水 validation ERROR 计数 {} ≠ 0",
                        rec.validation_error_count
                    ));
                }
                let t_tail = std::time::Instant::now();
                if let Some(out_data) = rec.out_color.as_ref() {
                    if !out_data.iter().all(|v| v.is_finite()) {
                        fail(&format!("bench 帧 {} 流水 upscale 输出非有限", rec.frame_index));
                    }
                    last_digest = frame_content_digest(out_w, out_h, 3, out_data);
                    if let Some(w) = flip_trace.as_mut() {
                        use std::io::Write as _;
                        let fi = rec.frame_index;
                        writeln!(w, "{{\"frame\":{fi},\"digest\":\"{last_digest}\"}}")
                            .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                    }
                }
                drain_tail_ms +=
                    t_tail.elapsed().as_secs_f64() * 1000.0 + rec.readback_convert_ms;
            }
            // 排空段墙钟并入末一测量样本（口径见循环头登记）。
            if let Some(last) = frame_ms.last_mut() {
                *last += drain_wait_ms + drain_tail_ms;
            }
            if let Some(last) = tail_ms.last_mut() {
                *last += drain_tail_ms;
            }
            if let Some(last) = prod_ms.last_mut() {
                *last += drain_wait_ms;
            }
        } else {
        for i in 0..total {
            let t_frame = std::time::Instant::now();
            let j = [
                halton(jitter_base + i + 1, 2) - 0.5,
                halton(jitter_base + i + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let readback_out = flip_trace.is_some() || i + 1 == total;
            // Phase C：GI2 帧序号逐帧挂载（params[52]——R2 时域旋转；off
            // 不调用零消费；双跑同帧序 ⇒ 位级一致口径不破）。
            if gi2.enabled {
                lane.set_gi2_frame(i as f32);
            }
            let rec = match lane.frame(
                in_w,
                in_h,
                out_w,
                out_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                &vp_j,
                exposure,
                i == 0,
                readback_out,
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("bench 帧 {i} 统一车道: {e}")),
            };
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "bench 帧 {i} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            // tail = 回读帧的字节→f32 转换 + is_finite 全帧校验 + digest
            // （测量循环零回读面无 out 数据 → tail=0，frame_ms_production=
            // frame_ms——诚实口径：生产帧本来就没有回读/校验/digest 面；
            // 末帧/trace 帧 tail 如实计量）。
            let t_tail = std::time::Instant::now();
            if let Some(out_data) = rec.out_color.as_ref() {
                if !out_data.iter().all(|v| v.is_finite()) {
                    fail(&format!("bench 帧 {i} upscale 输出非有限"));
                }
                last_digest = frame_content_digest(out_w, out_h, 3, out_data);
                if let Some(w) = flip_trace.as_mut() {
                    use std::io::Write as _;
                    writeln!(w, "{{\"frame\":{i},\"digest\":\"{last_digest}\"}}")
                        .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                }
            }
            let tail_el =
                t_tail.elapsed().as_secs_f64() * 1000.0 + rec.readback_convert_ms;
            let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
            if i >= warmup {
                frame_ms.push(frame_el);
                scene_ms.push(rec.scene_gpu_ns / 1e6);
                mv_ms.push(rec.mv_gpu_ns / 1e6);
                upscale_ms.push((rec.resample_gpu_ns + rec.resolve_gpu_ns) / 1e6);
                if tsr_probe {
                    resample_probe_ms.push(rec.resample_gpu_ns / 1e6);
                    resolve_probe_ms.push(rec.resolve_gpu_ns / 1e6);
                }
                scene_gpu_ns.push(rec.scene_gpu_ns);
                cpu_record_ns.push(rec.cpu_record_ns as f64);
                cpu_submit_ns.push(rec.cpu_submit_ns as f64);
                cpu_fence_wait_ns.push(rec.cpu_fence_wait_ns as f64);
                tail_ms.push(tail_el);
                prod_ms.push(frame_el - tail_el);
                // C7 profiler 收集（--profile-json on 才消费;与 frame_ms 同窗）。
                if profile_json.is_some() {
                    profile_frames.push(G14ProfileFrame {
                        passes: rec
                            .pass_gpu_ns
                            .iter()
                            .map(|(n, ns)| (n.clone(), ns / 1e6))
                            .collect(),
                        cpu_record_ms: rec.cpu_record_ns as f64 / 1e6,
                        cpu_submit_ms: rec.cpu_submit_ns as f64 / 1e6,
                        cpu_fence_wait_ms: rec.cpu_fence_wait_ns as f64 / 1e6,
                        readback_convert_ms: rec.readback_convert_ms,
                        frame_wall_ms: frame_el,
                        tail_ms: tail_el,
                        prod_wall_ms: frame_el - tail_el,
                    });
                }
            }
            if i == 0 || (i + 1) % 20 == 0 || i + 1 == total {
                eprintln!(
                    "{TAG}: bench 帧 {}/{total} frame={frame_el:.3}ms scene_gpu={:.3}ms mv_gpu={:.3}ms tsr_gpu={:.3}ms rec={:.3}ms sub={:.3}ms fence={:.3}ms",
                    i + 1,
                    rec.scene_gpu_ns / 1e6,
                    rec.mv_gpu_ns / 1e6,
                    (rec.resample_gpu_ns + rec.resolve_gpu_ns) / 1e6,
                    rec.cpu_record_ns as f64 / 1e6,
                    rec.cpu_submit_ns as f64 / 1e6,
                    rec.cpu_fence_wait_ns as f64 / 1e6,
                );
            }
        }
        }
        (
            "统一 DeviceFrameSession 四 pass 车道（session 不销毁；AS 常驻；场景 SSBO 创建期一次上传；pass0=kernels/g14_3_direct_gi.rx RayQuery compute → pass1=kernels/g14_mv.rx 相机 MV → pass2/3=kernels/g14_8_tsr_{resample,resolve}.rx；逐帧 scene 192B/mv 160B/tsr 128B 参数上传 + TSR parity binding_overrides；GPU 链内零 host 往返——RFC-0030 §4.5 L2 + §4.3 L3）".to_owned(),
            "host Instant 墙钟 + DeviceFrameTelemetry（逐 pass GPU timestamp + cpu_record/submit/fence_wait 分项）；frame_ms = 全链墙钟（参数三小件打包+四 pass submit+fence[+回读帧回读]）；scene_render_ms/mv_ms/upscale_ms = 逐 pass GPU timestamp 毫秒（scene=pass0，mv=pass1，upscale=pass2+pass3——统一 session 后不再是独立 host 段，列名不变值语义改为 GPU 段）".to_owned(),
            "G14plus 统一车道口径：测量循环零回读（readback_subset=[]）→ 测量帧 tail=0、frame_ms_production=frame_ms（生产帧本来就无回读/校验/digest 面，诚实口径）；末帧回读 TSR 输出 → tail = 回读字节→f32 转换 + is_finite 全帧校验 + digest（仅末帧有值；回读 GPU copy/fence 段留在 frame_ms/production——execute 内不可拆，量级 ~几 ms）；RURIX_G14_FLIP_TRACE 诊断模式强制逐帧回读（诊断凌驾性能，frame_ms 含回读税）；scene_render_ms/mv_ms/upscale_ms 列名不变值语义改为逐 pass GPU 毫秒段；M-c 门 smoke 消费 frame_ms_production_mean 与 last_frame_digest，不消费 mv_ms 语义".to_owned(),
        )
        } // （else 块 = 静态 bench 既有路径逐字保留；G31+ Task A4 dyn-demo 见上臂）
    } else if backend_name == "dlss_sr" {
        // G14.10e dlss 驻留统一车道（bench 测量腿：测量循环零 scene 回读 +
        // DLSS 驻留 evaluate；末帧/flip-trace 帧按需回读 DLSS 输出做 digest）。
        // dlss 臂恒 Mega 单 kernel（cornell 拆散三 pass 仅 tsr_device 臂消费，
        // 本车道不接线——形态简化登记）。
        let bits =
            DlssLaneBits::load(spv_scene, spv_mv, in_w, in_h, 2.0f32.powf(-scene.ev100));
        let descs = dlss_lane_descs(&assets, &bits, in_w, in_h);
        let blas_refs: [&[f32]; 1] = [&assets.tris];
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets.instances,
            },
            transforms: None,
            // G31+ 波 B Task B5 字段面:静态/厂商车道无顶点可更新 BLAS(0-byte)。
            updatable_blas: &[],
        }];
        let mut lane = match DlssResidentLane::create(
            &descs,
            &accel_structs,
            (in_w, in_h),
            (out_w, out_h),
        ) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("dlss_sr", &e),
        };
        eprintln!(
            "{TAG}: bench dlss 驻留统一车道就绪 warmup={warmup} frames={frames}（session 不销毁；scene→mv→pack 直写 exportable 三标 + DLSS 驻留 evaluate；测量循环零 scene 回读，末帧回读 DLSS 输出；flip_trace={}）",
            flip_trace.is_some()
        );
        let mut out_img = ImageF32::new(out_w, out_h, 3);
        let mut pack_probe_ms: Vec<f64> = Vec::new();
        for i in 0..total {
            let t_frame = std::time::Instant::now();
            let j = [
                halton(jitter_base + i + 1, 2) - 0.5,
                halton(jitter_base + i + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let rec = match lane.frame(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                &vp_j,
                exposure,
                i,
                i == 0,
                None,
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("bench 帧 {i} dlss 驻留车道: {e}")),
            };
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "bench 帧 {i} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            // tail = DLSS 输出按需回读（墙钟）+ is_finite 全帧校验 + digest
            // （末帧/flip-trace 帧；测量帧 resident 零回读 → tail=0，
            // frame_ms_production=frame_ms——统一车道同律诚实口径）。
            let is_last = i + 1 == total;
            let need_readback = is_last || flip_trace.is_some();
            let t_tail = std::time::Instant::now();
            if need_readback {
                lane.readback_into(&mut out_img);
                if !out_img.data.iter().all(|v| v.is_finite()) {
                    fail(&format!("bench 帧 {i} upscale 输出非有限"));
                }
                last_digest = frame_content_digest(out_w, out_h, 3, &out_img.data);
                if let Some(w) = flip_trace.as_mut() {
                    use std::io::Write as _;
                    writeln!(w, "{{\"frame\":{i},\"digest\":\"{last_digest}\"}}")
                        .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                }
            }
            let tail_el = t_tail.elapsed().as_secs_f64() * 1000.0;
            let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
            if i >= warmup {
                frame_ms.push(frame_el);
                scene_ms.push(rec.scene_gpu_ns / 1e6);
                mv_ms.push(rec.mv_gpu_ns / 1e6);
                upscale_ms.push(rec.upscale_wall_ms);
                pack_probe_ms.push(rec.pack_gpu_ns / 1e6);
                scene_gpu_ns.push(rec.scene_gpu_ns);
                cpu_record_ns.push(rec.cpu_record_ns as f64);
                cpu_submit_ns.push(rec.cpu_submit_ns as f64);
                cpu_fence_wait_ns.push(rec.cpu_fence_wait_ns as f64);
                tail_ms.push(tail_el);
                prod_ms.push(frame_el - tail_el);
            }
            if i == 0 || (i + 1) % 20 == 0 || i + 1 == total {
                eprintln!(
                    "{TAG}: bench 帧 {}/{total} frame={frame_el:.3}ms scene_gpu={:.3}ms mv_gpu={:.3}ms pack_gpu={:.3}ms upscale={:.3}ms rec={:.3}ms sub={:.3}ms fence={:.3}ms",
                    i + 1,
                    rec.scene_gpu_ns / 1e6,
                    rec.mv_gpu_ns / 1e6,
                    rec.pack_gpu_ns / 1e6,
                    rec.upscale_wall_ms,
                    rec.cpu_record_ns as f64 / 1e6,
                    rec.cpu_submit_ns as f64 / 1e6,
                    rec.cpu_fence_wait_ns as f64 / 1e6,
                );
            }
        }
        // pack 段 post-warmup 统计（receipt schema 无 pack 列——stderr 登记面；
        // 性能瀑布报告消费）。
        if !pack_probe_ms.is_empty() {
            let mean = pack_probe_ms.iter().sum::<f64>() / pack_probe_ms.len() as f64;
            let min = pack_probe_ms.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = pack_probe_ms
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            eprintln!(
                "{TAG}: DLSS_RESIDENT pack_gpu_ms mean={mean:.6} min={min:.6} max={max:.6}"
            );
        }
        (
            "dlss 驻留统一车道（session 不销毁；AS 常驻；场景 SSBO 创建期一次上传；pass0=kernels/g14_3_direct_gi.rx RayQuery compute → pass1=kernels/g14_mv.rx 相机 MV（NoContraction） → pass2=手编 g14_pack_spv 直写 RGBA32F/R32F/RG32F exportable image；逐帧 scene 192B/mv 160B 参数上传 + readback 表恒空；OPAQUE_WIN32 导入 → DLSS upscale_resident_external 驻留 evaluate——scene 回读/host mv/vendor host pack 中转税全消，RFC-0030 §4.3 G14.10e）".to_owned(),
            "host Instant 墙钟 + DeviceFrameTelemetry（逐 pass GPU timestamp + cpu_record/submit/fence_wait 分项）；frame_ms = 全链墙钟（参数二小件打包+三 pass submit+fence+evaluate[+回读帧 DLSS 输出回读]）；scene_render_ms/mv_ms = 逐 pass GPU timestamp 毫秒（scene=pass0，mv=pass1——mv 值语义为 GPU 段；pack=pass2 GPU 段 stderr DLSS_RESIDENT 行登记不入列）；upscale_ms = upscale_resident_external 墙钟（vendor 独立 device 域 submit_wait 同步口径，不含输出回读）".to_owned(),
            "G14.10e dlss 驻留车道口径：测量循环零 scene 回读（readback 表恒空）+ DLSS 输出驻留（不回读）→ 测量帧 tail=0、frame_ms_production=frame_ms（诚实口径）；末帧回读 DLSS 输出 → tail = readback_output_into 墙钟 + is_finite 全帧校验 + digest（仅末帧有值）；RURIX_G14_FLIP_TRACE 诊断模式强制逐帧回读（诊断凌驾性能）；last_frame_digest 语义 = DLSS 输出（upscale 后 1080p 图），非 scene 输出；digest 锚登记：mv=GPU mv（vs host mv ULP 级差）+ color RGBA32F f32 直通（vs 现状 f16 pack）+ depth R32F（vs D32）——evaluate 输入位面变化，输出 digest 相对现状锚预期 L1 漂移，双跑位级确定性为门檩".to_owned(),
        )
    } else if backend_name == "fsr_3_1_5"
        && std::env::var("RURIX_G14_FSR_HOST").ok().as_deref() != Some("1")
    {
        // G14.11 fsr 驻留统一车道（bench 测量腿：测量循环零 scene 回读 + ffx
        // dispatch D3D12 侧直读共享纹理；末帧/flip-trace 帧按需回读 FSR 输出
        // 做 digest）。fsr 区自持 image 版 descs/pack SPV；
        // RURIX_G14_FSR_HOST=1 逃生门走旧 host 链（跨 API 布局对拍参照面）。
        let bits = FsrLaneBits::load(spv_scene, spv_mv, in_w, in_h, 2.0f32.powf(-scene.ev100));
        let descs = fsr_lane_descs(&assets, &bits, in_w, in_h);
        let blas_refs: [&[f32]; 1] = [&assets.tris];
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets.instances,
            },
            transforms: None,
            // G31+ 波 B Task B5 字段面:静态/厂商车道无顶点可更新 BLAS(0-byte)。
            updatable_blas: &[],
        }];
        let mut lane = match FsrResidentLane::create(
            &descs,
            &accel_structs,
            (in_w, in_h),
            (out_w, out_h),
        ) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("fsr_3_1_5", &e),
        };
        eprintln!(
            "{TAG}: bench fsr 驻留统一车道就绪 warmup={warmup} frames={frames}（session 不销毁；scene→mv→pack v2 直写 D3D12 SHARED 导入 staging buffer + D3D12 CopyTextureRegion 搬入 + ffx dispatch；测量循环零 scene 回读，末帧回读 FSR 输出；flip_trace={}）",
            flip_trace.is_some()
        );
        let mut out_img = ImageF32::new(out_w, out_h, 3);
        let mut pack_probe_ms: Vec<f64> = Vec::new();
        for i in 0..total {
            let t_frame = std::time::Instant::now();
            let j = [
                halton(jitter_base + i + 1, 2) - 0.5,
                halton(jitter_base + i + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let rec = match lane.frame(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                &vp_j,
                exposure,
                i,
                i == 0,
                None,
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("bench 帧 {i} fsr 驻留车道: {e}")),
            };
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "bench 帧 {i} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            // tail = FSR 输出按需回读（墙钟）+ is_finite 全帧校验 + digest
            // （末帧/flip-trace 帧；测量帧 resident 零回读 → tail=0，
            // frame_ms_production=frame_ms——统一车道同律诚实口径）。
            let is_last = i + 1 == total;
            let need_readback = is_last || flip_trace.is_some();
            let t_tail = std::time::Instant::now();
            if need_readback {
                lane.readback_into(&mut out_img);
                if !out_img.data.iter().all(|v| v.is_finite()) {
                    fail(&format!("bench 帧 {i} upscale 输出非有限"));
                }
                last_digest = frame_content_digest(out_w, out_h, 3, &out_img.data);
                if let Some(w) = flip_trace.as_mut() {
                    use std::io::Write as _;
                    writeln!(w, "{{\"frame\":{i},\"digest\":\"{last_digest}\"}}")
                        .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                }
            }
            let tail_el = t_tail.elapsed().as_secs_f64() * 1000.0;
            let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
            if i >= warmup {
                frame_ms.push(frame_el);
                scene_ms.push(rec.scene_gpu_ns / 1e6);
                mv_ms.push(rec.mv_gpu_ns / 1e6);
                upscale_ms.push(rec.upscale_wall_ms);
                pack_probe_ms.push(rec.pack_gpu_ns / 1e6);
                scene_gpu_ns.push(rec.scene_gpu_ns);
                cpu_record_ns.push(rec.cpu_record_ns as f64);
                cpu_submit_ns.push(rec.cpu_submit_ns as f64);
                cpu_fence_wait_ns.push(rec.cpu_fence_wait_ns as f64);
                tail_ms.push(tail_el);
                prod_ms.push(frame_el - tail_el);
            }
            if i == 0 || (i + 1) % 20 == 0 || i + 1 == total {
                eprintln!(
                    "{TAG}: bench 帧 {}/{total} frame={frame_el:.3}ms scene_gpu={:.3}ms mv_gpu={:.3}ms pack_gpu={:.3}ms upscale={:.3}ms rec={:.3}ms sub={:.3}ms fence={:.3}ms",
                    i + 1,
                    rec.scene_gpu_ns / 1e6,
                    rec.mv_gpu_ns / 1e6,
                    rec.pack_gpu_ns / 1e6,
                    rec.upscale_wall_ms,
                    rec.cpu_record_ns as f64 / 1e6,
                    rec.cpu_submit_ns as f64 / 1e6,
                    rec.cpu_fence_wait_ns as f64 / 1e6,
                );
            }
        }
        // pack 段 post-warmup 统计（receipt schema 无 pack 列——stderr 登记面）。
        if !pack_probe_ms.is_empty() {
            let mean = pack_probe_ms.iter().sum::<f64>() / pack_probe_ms.len() as f64;
            let min = pack_probe_ms.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = pack_probe_ms
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            eprintln!(
                "{TAG}: FSR_RESIDENT pack_gpu_ms mean={mean:.6} min={min:.6} max={max:.6}"
            );
        }
        (
            "fsr 驻留统一车道（session 不销毁；AS 常驻；场景 SSBO 创建期一次上传；pass0=kernels/g14_3_direct_gi.rx RayQuery compute → pass1=kernels/g14_mv.rx 相机 MV（NoContraction） → pass2=手编 fsr_pack_spv v2 按 256B 行距三段直写 **D3D12 SHARED 导入 staging buffer**（color f16 RGBA/depth f32/mv f32 RG）；逐帧 scene 192B/mv 160B 参数上传 + readback 表恒空；ffx dispatch_resident D3D12 侧 CopyTextureRegion 搬入三输入纹理后 dispatch——scene 回读/host mv/vendor host pack/upload 中转税全消，G14.11 D3D12 反向共享 buffer 形态）".to_owned(),
            "host Instant 墙钟 + DeviceFrameTelemetry（逐 pass GPU timestamp + cpu_record/submit/fence_wait 分项）；frame_ms = 全链墙钟（参数二小件打包+三 pass submit+fence+ffx dispatch[+回读帧 FSR 输出回读]）；scene_render_ms/mv_ms = 逐 pass GPU timestamp 毫秒（scene=pass0，mv=pass1——mv 值语义为 GPU 段；pack=pass2 GPU 段 stderr FSR_RESIDENT 行登记不入列）；upscale_ms = dispatch_resident 墙钟（D3D12 submit_wait 同步口径，不含输出回读）".to_owned(),
            "G14.11 fsr 驻留车道口径：测量循环零 scene 回读（readback 表恒空）+ FSR 输出驻留（不回读）→ 测量帧 tail=0、frame_ms_production=frame_ms（诚实口径）；末帧回读 FSR 输出 → tail = readback_output_resident 墙钟 + is_finite 全帧校验 + digest（仅末帧有值）；RURIX_G14_FLIP_TRACE 诊断模式强制逐帧回读（诊断凌驾性能）；last_frame_digest 语义 = FSR 输出（upscale 后 1080p 图）；digest 锚登记：mv=GPU mv（vs host mv ULP 级差）+ color f16 PackHalf2x16 RTE ×exposure 显示域转换（vendor 臂 scene 域直通存量语义分裂修正，dlss 臂 2da40f4c 同款；cornell ev100=0 位保持）+ depth f32 位拷贝直通——输出 digest 相对现状锚预期 L1 漂移（dlss 臂同族），双跑位级确定性为门檩".to_owned(),
        )
    } else {
    let spv_scene_words = load_spv(spv_scene);
    let spv_scene_bytes: Vec<u8> = spv_scene_words
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
        // G14.10d：同 vendor 臂规则——params（资源 4，逐帧上传目标）=
        // host-visible，其余 = DEVICE_LOCAL。
    let resources = [
        ResourceDesc::Buffer(BufferDesc {
            size: assets.tris_bytes.len() as u64,
            usage: storage,
            data: Some(&assets.tris_bytes),
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.mats_bytes.len() as u64,
            usage: storage,
            data: Some(&assets.mats_bytes),
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.quads_bytes.len() as u64,
            usage: storage,
            data: Some(&assets.quads_bytes),
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.points_bytes.len() as u64,
            usage: storage,
            data: Some(&assets.points_bytes),
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.params0_bytes.len() as u64,
            usage: storage,
            data: Some(&assets.params0_bytes),
                device_local: false,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.out_color_size,
            usage: storage,
            data: None,
                device_local: true,
        }),
        ResourceDesc::Buffer(BufferDesc {
            size: assets.out_depth_size,
            usage: storage,
            data: None,
                device_local: true,
        }),
    ];
    let passes = [Pass::Compute(ComputePass {
        name: "g14_3_direct_gi",
        spirv: &spv_scene_bytes,
        entry: None,
        dispatch: DispatchSpec::Direct([
            in_w.div_ceil(spv_local_size(&spv_scene_words).0),
            in_h.div_ceil(spv_local_size(&spv_scene_words).1),
            1,
        ]),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: vec![0, 1, 2, 3, 4, 5, 6],
            ..Bindings::default()
        },
    })];
    let plan = [
        (0u32, TargetState::StorageReadWrite),
        (1u32, TargetState::StorageReadWrite),
        (2u32, TargetState::StorageReadWrite),
        (3u32, TargetState::StorageReadWrite),
        (4u32, TargetState::StorageReadWrite),
        (5u32, TargetState::StorageReadWrite),
        (6u32, TargetState::StorageReadWrite),
    ];
    let barriers: [&[(u32, TargetState)]; 1] = [&plan];
    let readbacks = [
        Readback::Buffer {
            res: 5,
            offset: 0,
            size: assets.out_color_size,
        },
        Readback::Buffer {
            res: 6,
            offset: 0,
            size: assets.out_depth_size,
        },
    ];
    let blas_refs: [&[f32]; 1] = [&assets.tris];
    let accel_structs = [AccelStructDesc {
        scene: RayQuerySceneDesc {
            blas_triangles: &blas_refs,
            instances: &assets.instances,
        },
        transforms: None,
        // G31+ 波 B Task B5 字段面:本车道无顶点可更新 BLAS(0-byte)。
        updatable_blas: &[],
    }];
    if !vk::vulkan_available() {
        dev_env_or_fail("device_lane", "vulkan loader 不可用");
    }
    let mut session = match DeviceFrameSession::new_with_accel_structs(
        &resources,
        &passes,
        &barriers,
        &readbacks,
        2,
        &accel_structs,
    ) {
        Ok(s) => s,
        Err(e) => dev_env_or_fail("device_lane", &e),
    };
        // vendor backend 创建（tsr_device/dlss_sr 已各走统一车道分支，本 match
        // 仅承载 fsr）。
    let mut backend = match backend_name {
        "fsr_3_1_5" => match FsrBackend::create((in_w, in_h), (out_w, out_h)) {
            Ok(b) => Backend::Fsr(b),
            Err(e) => dev_env_or_fail("fsr_3_1_5", &e),
        },
        other => fail(&format!(
            "未知 backend: {other}（tsr_device|dlss_sr|fsr_3_1_5）"
        )),
    };
    eprintln!(
        "{TAG}: bench 就绪 backend={} warmup={warmup} frames={frames}（session 不销毁持续帧循环）",
        backend.name()
    );
    let mut prev_vp: Option<Mat4> = None;
    // G14.6：bench 腿驻留输出缓冲（Stage A 消逐帧分配；字节面逐位一致）
    let mut out_img = ImageF32::new(out_w, out_h, 3);
    for i in 0..total {
        let t_frame = std::time::Instant::now();
        let j = [
            halton(jitter_base + i + 1, 2) - 0.5,
            halton(jitter_base + i + 1, 3) - 0.5,
        ];
        let rec = match device_frame(
            &mut session,
            in_w,
            in_h,
            j,
            eps,
            scene.quads.len(),
            scene.points.len(),
            &inv_vp,
            &vp,
        ) {
            Ok(r) => r,
            Err(e) => fail(&format!("bench 帧 {i} device 车道: {e}")),
        };
        if rec.validation_error_count != 0 {
            fail(&format!(
                "bench 帧 {i} validation ERROR 计数 {} ≠ 0",
                rec.validation_error_count
            ));
        }
        let vp_j = jittered_vp(&vp, j, in_w, in_h);
        let t_mv = std::time::Instant::now();
        let mv = match prev_vp {
            Some(prev) => compute_camera_mv(&rec.depth, &vp_j, &prev),
            None => ImageF32::new(in_w, in_h, 2),
        };
        prev_vp = Some(vp_j);
        let mv_el = t_mv.elapsed().as_secs_f64() * 1000.0;
        let t_up = std::time::Instant::now();
        let inputs = UpscaleInputs {
            color: &rec.color,
            depth: &rec.depth,
            mv: &mv,
            reactive: None,
            exposure,
            jitter: j,
            output_size: (out_w, out_h),
            frame_index: i,
            reset: i == 0,
        };
            // fsr 臂维持 upscale_into 逐帧回读（FSR resident 归 external memory
            // 波另判；dlss_sr 已走驻留统一车道分支不再经本路径）。
            let out_ready = match &mut backend {
                Backend::Fsr(b) => {
                    b.upscale_into(&inputs, &mut out_img);
                    true
                }
            };
            let out = &out_img;
        let up_el = t_up.elapsed().as_secs_f64() * 1000.0;
        // G14.6 口径分解：tail = bench 测量面（is_finite 全帧校验 + frame_content_digest
        // payload 重建+sha256）——非生产路径固有面；frame_ms（全量口径，G14.3 兼容）
        // 与 frame_ms_production（= frame − tail，M-d 对标消费面）双列同测，零行为变更。
        let t_tail = std::time::Instant::now();
            if out_ready {
        if !out.data.iter().all(|v| v.is_finite()) {
            fail(&format!("bench 帧 {i} upscale 输出非有限"));
        }
        last_digest = frame_content_digest(out.w, out.h, 3, &out.data);
                if let Some(w) = flip_trace.as_mut() {
                    use std::io::Write as _;
                    writeln!(w, "{{\"frame\":{i},\"digest\":\"{last_digest}\"}}")
                        .unwrap_or_else(|e| fail(&format!("flip-trace 写入: {e}")));
                }
            }
        let tail_el = t_tail.elapsed().as_secs_f64() * 1000.0;
        let frame_el = t_frame.elapsed().as_secs_f64() * 1000.0;
        if i >= warmup {
            frame_ms.push(frame_el);
            scene_ms.push(rec.scene_host_ms);
            mv_ms.push(mv_el);
            upscale_ms.push(up_el);
            scene_gpu_ns.push(rec.scene_gpu_ns);
            cpu_record_ns.push(rec.cpu_record_ns as f64);
            cpu_submit_ns.push(rec.cpu_submit_ns as f64);
            cpu_fence_wait_ns.push(rec.cpu_fence_wait_ns as f64);
            tail_ms.push(tail_el);
            prod_ms.push(frame_el - tail_el);
        }
        if i == 0 || (i + 1) % 20 == 0 || i + 1 == total {
            eprintln!(
                "{TAG}: bench 帧 {}/{total} frame={frame_el:.3}ms scene={:.3}ms(gpu={:.3}ms rec={:.3}ms sub={:.3}ms fence={:.3}ms) mv={mv_el:.3}ms upscale={up_el:.3}ms",
                i + 1,
                rec.scene_host_ms,
                rec.scene_gpu_ns / 1e6,
                rec.cpu_record_ns as f64 / 1e6,
                rec.cpu_submit_ns as f64 / 1e6,
                rec.cpu_fence_wait_ns as f64 / 1e6,
            );
        }
    }
        (
            "DeviceFrameSession 持久车道 + kernels/g14_3_direct_gi.rx RayQuery compute（session 不销毁；AS 常驻；场景 SSBO 创建期一次上传；逐帧 192B 帧参数上传 + readback 子集）".to_owned(),
            "host Instant 墙钟 + DeviceFrameTelemetry（逐 pass GPU timestamp + cpu_record/submit/fence_wait 分项）".to_owned(),
            "G14.6 双列口径：frame_ms = 全量（G14.3 兼容，含 bench 测量面 tail）；frame_ms_production = frame − tail（tail = is_finite 全帧校验 + frame_content_digest payload 重建+sha256，非生产路径固有面）；M-d 对标消费 production 列，双列同测零行为变更".to_owned(),
        )
    };

    // 稳态统计（程序产禁手写阈；cv = 总体标准差/均值，post-warmup 测量面）。
    let stats = |v: &[f64]| -> (f64, f64, f64, f64, f64) {
        let n = v.len() as f64;
        let mean = v.iter().sum::<f64>() / n;
        let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / n;
        let sd = var.sqrt();
        let min = v.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (mean, sd, sd / mean, min, max)
    };
    let (f_mean, f_sd, f_cv, f_min, f_max) = stats(&frame_ms);
    let (s_mean, _, _, _, _) = stats(&scene_ms);
    let (m_mean, _, _, _, _) = stats(&mv_ms);
    let (u_mean, _, _, _, _) = stats(&upscale_ms);
    // G14.10c TSR 探针出报（门控见 tsr_probe 声明处；vendor 双臂无 TSR pass
    // → 容器空，探针静默）。
    if tsr_probe && !resample_probe_ms.is_empty() {
        let (rs_mean, _, _, rs_min, rs_max) = stats(&resample_probe_ms);
        let (rv_mean, _, _, rv_min, rv_max) = stats(&resolve_probe_ms);
        eprintln!(
            "{TAG}: TSR_PROBE resample_ms mean={rs_mean:.6} min={rs_min:.6} max={rs_max:.6} | resolve_ms mean={rv_mean:.6} min={rv_min:.6} max={rv_max:.6}"
        );
    }
    let (g_mean, _, _, _, _) = stats(&scene_gpu_ns);
    let (rec_mean, _, _, _, _) = stats(&cpu_record_ns);
    let (sub_mean, _, _, _, _) = stats(&cpu_submit_ns);
    let (wait_mean, _, _, _, _) = stats(&cpu_fence_wait_ns);
    let (t_mean, _, _, _, _) = stats(&tail_ms);
    let (p_mean, p_sd, p_cv, p_min, p_max) = stats(&prod_ms);
    let join_ms = |v: &[f64]| {
        v.iter()
            .map(|x| format!("{x:.6}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let out_dir = PathBuf::from(out_root)
        .join(scene_id)
        .join(format!("tier{tier}"))
        .join(backend_name);
    std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| fail(&format!("输出目录: {e}")));
    let receipt = format!(
        "{{\n  \"schema\": \"rurix.g14.pipeline_perf_bench_receipt.v1\",\n  \"contract\": {},\n  \"contract_digest_rurix\": {},\n  \"scene_id\": {},\n  \"tier\": {},\n  \"backend\": {},\n  \"seed\": {},\n  \"jitter_base\": {},\n  \"output_size\": [{}, {}],\n  \"internal_size\": [{}, {}],\n  \"exposure\": {},\n  \"warmup\": {},\n  \"inflight\": {},\n  \"frames_measured\": {},\n  \"iterations_total\": {},\n  \"frame_ms\": [{}],\n  \"scene_render_ms\": [{}],\n  \"mv_ms\": [{}],\n  \"upscale_ms\": [{}],\n  \"scene_gpu_ns\": [{}],\n  \"cpu_record_ns\": [{}],\n  \"cpu_submit_ns\": [{}],\n  \"cpu_fence_wait_ns\": [{}],\n  \"tail_ms\": [{}],\n  \"frame_ms_production\": [{}],\n  \"stats_post_warmup\": {{\"frame_ms_mean\": {}, \"frame_ms_sd\": {}, \"frame_ms_cv\": {}, \"frame_ms_min\": {}, \"frame_ms_max\": {}, \"scene_render_ms_mean\": {}, \"mv_ms_mean\": {}, \"upscale_ms_mean\": {}, \"scene_gpu_ns_mean\": {}, \"cpu_record_ns_mean\": {}, \"cpu_submit_ns_mean\": {}, \"cpu_fence_wait_ns_mean\": {}, \"tail_ms_mean\": {}, \"frame_ms_production_mean\": {}, \"frame_ms_production_sd\": {}, \"frame_ms_production_cv\": {}, \"frame_ms_production_min\": {}, \"frame_ms_production_max\": {}}},\n  \"caliber\": {},\n  \"steady_state_fps_mean\": {},\n  \"render_lane\": {},\n  \"timer\": {},\n  \"last_frame_digest\": {},\n  \"gi_arm\": \"direct_only（--gi off；GI 臂 not-triggered 登记见 render_receipt 面）\"\n}}\n",
        jstr(&contract_path.replace('\\', "/")),
        jstr(&contract.digest),
        jstr(scene_id),
        tier,
        jstr(backend_name),
        seed,
        jitter_base,
        out_w,
        out_h,
        in_w,
        in_h,
        exposure,
        warmup,
        inflight,
        frames,
        total,
        join_ms(&frame_ms),
        join_ms(&scene_ms),
        join_ms(&mv_ms),
        join_ms(&upscale_ms),
        join_ms(&scene_gpu_ns),
        join_ms(&cpu_record_ns),
        join_ms(&cpu_submit_ns),
        join_ms(&cpu_fence_wait_ns),
        join_ms(&tail_ms),
        join_ms(&prod_ms),
        f_mean,
        f_sd,
        f_cv,
        f_min,
        f_max,
        s_mean,
        m_mean,
        u_mean,
        g_mean,
        rec_mean,
        sub_mean,
        wait_mean,
        t_mean,
        p_mean,
        p_sd,
        p_cv,
        p_min,
        p_max,
        jstr(&caliber),
        1000.0 / f_mean,
        jstr(&render_lane),
        jstr(&timer),
        jstr(&last_digest),
    );
    let receipt_path = out_dir.join("bench_receipt.json");
    std::fs::write(&receipt_path, &receipt).unwrap_or_else(|e| fail(&format!("bench receipt 落盘: {e}")));
    // ── C7 profiler 输出面（--profile-json;机器可读逐 pass 分解独立落盘——
    //    receipt 面 0-byte,默认关 = 零收集零写盘）──
    if let Some(pj_path) = profile_json {
        let t_prof = std::time::Instant::now();
        let pj = match g14_profile_json(
            &profile_frames,
            scene_id,
            tier,
            backend_name,
            out_w,
            out_h,
            in_w,
            in_h,
            warmup,
            inflight,
            debug_labels_active,
            &last_digest,
            t_prof,
        ) {
            Ok(s) => s,
            Err(e) => fail(&e),
        };
        let pj_pb = PathBuf::from(pj_path);
        if let Some(parent) = pj_pb.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .unwrap_or_else(|e| fail(&format!("profile 目录: {e}")));
            }
        }
        std::fs::write(&pj_pb, format!("{pj}\n"))
            .unwrap_or_else(|e| fail(&format!("profile 写入: {e}")));
        let write_ms = t_prof.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "{TAG}: profile → {}（{} 帧逐 pass 分解;assembly+write={write_ms:.3}ms,debug_labels={debug_labels_active}）",
            pj_pb.display(),
            profile_frames.len()
        );
    }
    println!(
        "{TAG}: BENCH PASS scene={scene_id} tier={tier} backend={backend_name} warmup={warmup} frames={frames} frame_ms_mean={f_mean:.6} cv={f_cv:.6} fps={:.3} scene_gpu_ms_mean={:.6} prod_ms_mean={p_mean:.6} tail_ms_mean={t_mean:.6} out={}",
        1000.0 / f_mean,
        g_mean / 1e6,
        out_dir.display()
    );
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

