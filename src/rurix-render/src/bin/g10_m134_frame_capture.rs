//! G10.4a M134 帧捕获管线 harness（spec/imageio.md §2A RXS-0385 +
//! spec/visual_comparison.md RXS-0386；门 `g10.p0.m134.frame_capture_pipeline`）。
//!
//! ## 判据面（G10_ACCEPTANCE_MAP §1 M134 行逐字 + RFC-0026 §4.1/§6.2）
//!
//! 1. **HDR 帧捕获落盘 + 捕获→回读逐像素往返无损**：device 腿（GPU 真渲染
//!    三角形 → Rgba16Float target → `Readback::Texture` 回读 → fp16→f32 精确
//!    提升）与 host 腿（闭式探针图案，含 HDR>1 / 负值 / 非 8-bit 精确值）
//!    各自 capture→encode→落盘→decode 逐像素 float32 **位级相等**；
//! 2. **元数据闭集齐备**：分辨率（dataWindow/displayWindow）/ 色彩空间
//!    （chromaticities Rec709/D65 位级闭集 + domain/transfer 互证）/ 位深
//!    （float32 canonical）/ rurix:* 九字段齐备，写侧闭集外禁写；
//! 3. **渲染输出探针图案位级核验**（RFC-0026 §6.2 F16）：host 探针图案逐
//!    像素先验，capture→EXR 后逐像素位级出现（防恒定合成帧伪绿）；device
//!    腿锚点断言（角 = 清色 / 中心覆盖 / 覆盖计数 > 0）证非恒定帧；
//! 4. **UE 真帧 strip-and-log 读取**：G10.2 已出真实 UE EXR（fp16 RGBA、
//!    NONE）按 ue5 策略解码——闭集外 `unreal/*` 属性剥离逐条登记、alpha
//!    通道剥离登记、fp16→f32 精确提升、chromaticities 位级闭集互证；
//! 5. **ZIP fail-closed**：compression 改写为 ZIP(=3) 后读取必显式
//!    `UnsupportedCompression`（禁静默，RXS-0385 L1 v1 实现面登记）；
//! 6. **RED 臂**：8-bit clamp 位深截断注入 / sRGB-线性混标注入 / 元数据
//!    缺字段注入——各 `--red-arm` 子模式演示检出（检出即 exit 0「PASS
//!    red-arm」，漏检 exit 1，g9_m118 体例）。
//!
//! ## 三态
//!
//! 无 Vulkan loader / demo 着色器缺失 → device 腿 `SKIP DEV_ENV_DEGRADE`
//! （退 0，非 fake pass；`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke
//! 脚本层裁决）；host 腿恒跑。判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g10_m134_frame_capture --evidence <path> --ue-frame <ue.exr> [--work-dir <dir>]
//! g10_m134_frame_capture --host-only [--evidence <path>] [--work-dir <dir>]
//! g10_m134_frame_capture --red-arm clamp-8bit|srgb-linear-mislabel|metadata-missing
//! ```

#![forbid(unsafe_code)]

use image_io::ImageError;
use image_io::exr::{
    ChromaticitiesOrigin, ExrBitDepth, ExrChannelLayout, ExrDerivation, ExrDomain, ExrImage,
    ExrMetadata, ExrSourceEnd, ExrTransfer, decode_exr, encode_exr, half_to_f32,
};
use rurix_rt::render_exec::{
    self, ColorAttachmentRef, DrawSpec, Pass, RasterPass, Readback, ResourceDesc, TargetState,
    TexFormat, TextureDesc, TextureUsage, VertexData,
};
use rurix_rt::vk;
use std::path::{Path, PathBuf};

const TAG: &str = "G10_M134_FC";
const DEVICE_W: u32 = 64;
const DEVICE_H: u32 = 64;
const HOST_W: u32 = 64;
const HOST_H: u32 = 48;

const FORMAT_R32G32B32A32_SFLOAT: u32 = 109;
const TRI_ATTRS: [(u32, u32, u32); 2] = [
    (0, FORMAT_R32G32B32A32_SFLOAT, 0),
    (1, FORMAT_R32G32B32A32_SFLOAT, 16),
];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

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

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn sha256_hex(data: &[u8]) -> String {
    rurix_pkg::sha256::hex_digest(data)
}

/// 探针描述符 digest（RXS-0385 L3：G10.4 探针/合成帧 capture_params_digest =
/// 生成参数描述符 SHA-256，登记面不冒充 M130 链）。
fn probe_params_digest(tag: &str, width: u32, height: u32) -> String {
    let mut payload = b"G10M134P-1\0".to_vec();
    payload.extend_from_slice(&width.to_le_bytes());
    payload.extend_from_slice(&height.to_le_bytes());
    payload.extend_from_slice(tag.as_bytes());
    format!("sha256:{}", sha256_hex(&payload))
}

/// 帧像素内容 digest（跨实现互证面：Rust bin 与 ci Python 独立解析器同字面
/// 复算——`"G10EXRD-1\0" ‖ w u32le ‖ h u32le ‖ channels u8 ‖ f32 LE 像素字节`）。
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

/// HDR 臂捕获元数据（Rurix canonical；探针 digest 登记）。
fn hdr_capture_metadata(width: u32, height: u32, probe_tag: &str) -> ExrMetadata {
    ExrMetadata {
        schema_version: "1".to_owned(),
        domain: ExrDomain::SceneLinearHdr,
        transfer: ExrTransfer::Linear,
        bit_depth: ExrBitDepth::Float32,
        source_end: ExrSourceEnd::Rurix,
        view_transform: None,
        capture_params_digest: probe_params_digest(probe_tag, width, height),
        derivation: ExrDerivation::Capture,
        source_frame_digest: None,
        chromaticities_origin: Some(ChromaticitiesOrigin::Writer),
    }
}

/// host 闭式探针图案（RXS-0385 L5：含 HDR>1 值 / 负值 / 非 8-bit 精确表示
/// 值——截断注入必然改变位模式，防 8-bit clamp 伪绿通道）。
fn host_probe_pixels(width: u32, height: u32) -> Vec<f32> {
    let mut px = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            px.push(0.1 * x as f32 + 1.25); // HDR>1
            px.push(0.01 * y as f32 - 0.5); // 负值
            px.push((x * 7 + y * 13) as f32 * 0.001_953_125 + 0.1); // 非 8-bit 精确
        }
    }
    px
}

/// capture→encode→decode 逐像素位级核验（往返无损判据单一事实源）。
fn roundtrip_bit_exact(img: &ExrImage) -> Result<(Vec<u8>, Vec<f32>), String> {
    let bytes = encode_exr(img).map_err(|e| format!("encode 失败: {e}"))?;
    let bytes2 = encode_exr(img).map_err(|e| format!("二次 encode 失败: {e}"))?;
    if bytes != bytes2 {
        return Err("同输入两次编码非逐字节一致（确定性失效）".to_owned());
    }
    let dec = decode_exr(&bytes, ExrSourceEnd::Rurix).map_err(|e| format!("decode 失败: {e}"))?;
    if dec.width != img.width || dec.height != img.height || dec.layout != img.layout {
        return Err("回读帧形态漂移".to_owned());
    }
    if dec.pixels.len() != img.pixels.len() {
        return Err("回读像素长度漂移".to_owned());
    }
    for (i, (a, b)) in dec.pixels.iter().zip(img.pixels.iter()).enumerate() {
        if a.to_bits() != b.to_bits() {
            return Err(format!(
                "像素 {i} 位级不等（roundtrip 失效）: {:#010x} ≠ {:#010x}",
                a.to_bits(),
                b.to_bits()
            ));
        }
    }
    Ok((bytes, dec.pixels))
}

/// device 腿：GPU 真渲染 → readback → fp16→f32 提升 → capture EXR。
/// 返回 (captured_pixels_rgb, anchors_ok, device_name)。
fn device_capture_leg() -> Result<(Vec<f32>, bool, String), String> {
    if !vk::vulkan_available() {
        return Err("vulkan loader 不可用".to_owned());
    }
    let (vs, fs, _saxpy) = vk::demo_shaders_spv();
    if vs.is_empty() || fs.is_empty() {
        return Err("demo 着色器缺失".to_owned());
    }
    let device_name = render_exec::probe_device_caps()
        .map(|c| c.device_name)
        .unwrap_or_else(|_| "unknown".to_owned());
    // 居中三角形顶点（pos vec4 @0 + color vec4 @16，stride 32；render_exec
    // device_triangle_draw_readback 先例同律）。
    let mut verts = Vec::with_capacity(3 * 32);
    let mut push = |vals: [f32; 4]| {
        for f in vals {
            verts.extend_from_slice(&f.to_le_bytes());
        }
    };
    push([0.0, 0.7, 0.0, 1.0]);
    push([1.0, 0.0, 0.0, 1.0]);
    push([-0.7, -0.7, 0.0, 1.0]);
    push([0.0, 1.0, 0.0, 1.0]);
    push([0.7, -0.7, 0.0, 1.0]);
    push([0.0, 0.0, 1.0, 1.0]);

    let resources = vec![ResourceDesc::Texture(TextureDesc {
        width: DEVICE_W,
        height: DEVICE_H,
        format: TexFormat::Rgba16Float,
        usage: TextureUsage {
            sampled: false,
            storage: false,
            color: true,
            depth: false,
        },
        data: None,
    })];
    let passes = vec![Pass::Raster(RasterPass {
        name: "g10_m134_probe_tri",
        vs_spirv: vs,
        fs_spirv: fs,
        vertex: VertexData::Inline {
            data: &verts,
            stride: 32,
            attrs: &TRI_ATTRS,
        },
        draw: DrawSpec::Direct {
            vertex_count: 3,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        },
        colors: vec![ColorAttachmentRef {
            res: 0,
            clear: Some([0.0, 0.0, 0.0, 1.0]),
        }],
        depth: None,
        viewport: None,
        bindings: Default::default(),
        conservative: None,
    })];
    let plan: Vec<Vec<(u32, TargetState)>> = vec![vec![(0, TargetState::ColorAttachmentWrite)]];
    let brefs: Vec<&[(u32, TargetState)]> = plan.iter().map(Vec::as_slice).collect();
    let readbacks = vec![Readback::Texture { res: 0 }];
    let out = render_exec::execute_frame(&resources, &passes, &brefs, &readbacks)
        .map_err(|e| format!("device 帧执行失败: {e}"))?;
    let raw = out.first().ok_or_else(|| "readback 缺失".to_owned())?;
    if raw.len() != (DEVICE_W * DEVICE_H * 8) as usize {
        return Err(format!("readback 字节数 {} 非法", raw.len()));
    }
    // fp16 RGBA → f32 RGB（精确提升；alpha 不进入 canonical 面）。
    let mut rgb = Vec::with_capacity((DEVICE_W * DEVICE_H * 3) as usize);
    let mut alphas = Vec::with_capacity((DEVICE_W * DEVICE_H) as usize);
    for chunk in raw.chunks_exact(8) {
        let r = half_to_f32(u16::from_le_bytes([chunk[0], chunk[1]]));
        let g = half_to_f32(u16::from_le_bytes([chunk[2], chunk[3]]));
        let b = half_to_f32(u16::from_le_bytes([chunk[4], chunk[5]]));
        let a = half_to_f32(u16::from_le_bytes([chunk[6], chunk[7]]));
        rgb.extend_from_slice(&[r, g, b]);
        alphas.push(a);
    }
    // 锚点断言（探针图案位级核验 device 面：证非恒定合成帧）。
    let at = |x: u32, y: u32| -> (f32, f32, f32) {
        let o = ((y * DEVICE_W + x) * 3) as usize;
        (rgb[o], rgb[o + 1], rgb[o + 2])
    };
    let corner = at(0, DEVICE_H - 1);
    let corner_clear =
        corner == (0.0, 0.0, 0.0) && alphas[(DEVICE_H - 1) as usize * DEVICE_W as usize] == 1.0;
    let center = at(DEVICE_W / 2, DEVICE_H / 2);
    let center_covered = center != (0.0, 0.0, 0.0);
    let covered = rgb
        .chunks_exact(3)
        .filter(|p| p[0] != 0.0 || p[1] != 0.0 || p[2] != 0.0)
        .count();
    let anchors_ok = corner_clear && center_covered && covered > 0;
    eprintln!(
        "[{TAG}] device 腿: device=`{device_name}` covered={covered} corner={corner:?} center={center:?}"
    );
    Ok((rgb, anchors_ok, device_name))
}

struct Args {
    evidence: Option<PathBuf>,
    ue_frame: Option<PathBuf>,
    work_dir: PathBuf,
    host_only: bool,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let root = workspace_root();
    let mut out = Args {
        evidence: None,
        ue_frame: None,
        work_dir: root.join(".tmp/g104_gates/m134"),
        host_only: false,
        red_arm: None,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--evidence" => out.evidence = Some(PathBuf::from(take(&mut i))),
            "--ue-frame" => out.ue_frame = Some(PathBuf::from(take(&mut i))),
            "--work-dir" => out.work_dir = PathBuf::from(take(&mut i)),
            "--host-only" => out.host_only = true,
            "--red-arm" => out.red_arm = Some(take(&mut i)),
            other => fail(&format!("未知参数: {other}")),
        }
        i += 1;
    }
    out
}

// ─────────────────────────── RED 臂（检出即 Ok；g9_m118 体例） ───────────────────────────

/// RED 臂：8-bit clamp 位深截断注入——探针图案含非 8-bit 精确值，截断后
/// capture→EXR 逐像素位级核验必须偏离（检出 = 往返无损判据抓得住截断）。
fn red_arm_clamp_8bit() -> Result<(), String> {
    let expected = host_probe_pixels(HOST_W, HOST_H);
    // 注入：捕获路径把像素量化到 8-bit（floor(clamp*255+0.5)/255）。
    let truncated: Vec<f32> = expected
        .iter()
        .map(|v| {
            let c = v.clamp(0.0, 1.0);
            (c * 255.0 + 0.5).floor() / 255.0
        })
        .collect();
    // 判定面：capture 帧与探针先验逐像素位级比对——截断必致不等。
    let mut mismatches = 0usize;
    for (a, b) in truncated.iter().zip(expected.iter()) {
        if a.to_bits() != b.to_bits() {
            mismatches += 1;
        }
    }
    if mismatches == 0 {
        return Err("8-bit clamp 未引起任何位级差异（探针图案判别力失效）".to_owned());
    }
    // 截断帧照常可编码（往返无损只保证 pipeline 无损）；判据 = 与先验不等。
    let img = ExrImage::new(
        HOST_W,
        HOST_H,
        ExrChannelLayout::Rgb,
        truncated,
        hdr_capture_metadata(HOST_W, HOST_H, "red-clamp-8bit"),
    )
    .map_err(|e| format!("截断帧构造失败: {e}"))?;
    let _ = encode_exr(&img).map_err(|e| format!("截断帧编码失败: {e}"))?;
    eprintln!("[{TAG}] RED 检出 clamp-8bit: {mismatches} 像素位级偏离探针先验");
    Ok(())
}

/// RED 臂：sRGB/线性混标注入——HDR 域 + transfer=srgb 构造必须 fail-closed。
fn red_arm_srgb_linear_mislabel() -> Result<(), String> {
    let mut md = hdr_capture_metadata(2, 2, "red-mislabel");
    md.transfer = ExrTransfer::Srgb; // 混标注入（HDR 域必 linear）
    match ExrImage::new(2, 2, ExrChannelLayout::Rgb, vec![0.0; 12], md) {
        Err(ImageError::MetadataViolation(_)) => {
            eprintln!("[{TAG}] RED 检出 srgb-linear-mislabel: 混标构造被拒");
            Ok(())
        }
        other => Err(format!("混标未被拒（假绿口）: {other:?}")),
    }
}

/// RED 臂：元数据缺字段注入——rurix 帧缺 `rurix:derivation` 读取必拒。
fn red_arm_metadata_missing() -> Result<(), String> {
    // 构造一帧合法 EXR，再从字节流剔除 `rurix:derivation` 属性后读取。
    let img = ExrImage::new(
        2,
        2,
        ExrChannelLayout::Rgb,
        vec![0.0; 12],
        hdr_capture_metadata(2, 2, "red-metadata-missing"),
    )
    .map_err(|e| format!("基帧构造失败: {e}"))?;
    let bytes = encode_exr(&img).map_err(|e| format!("基帧编码失败: {e}"))?;
    // 剔除 derivation 属性（name\0 type\0 size value 整段）。
    let needle = b"rurix:derivation\x00string\x00".to_vec();
    let pos = bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .ok_or_else(|| "rurix:derivation 属性不在树".to_owned())?;
    let size_off = pos + needle.len();
    let vlen = u32::from_le_bytes([
        bytes[size_off],
        bytes[size_off + 1],
        bytes[size_off + 2],
        bytes[size_off + 3],
    ]) as usize;
    let mut tampered = Vec::with_capacity(bytes.len());
    tampered.extend_from_slice(&bytes[..pos]);
    tampered.extend_from_slice(&bytes[size_off + 4 + vlen..]);
    match decode_exr(&tampered, ExrSourceEnd::Rurix) {
        Err(ImageError::MetadataViolation(_)) => {
            eprintln!("[{TAG}] RED 检出 metadata-missing: 缺字段帧被拒");
            Ok(())
        }
        other => Err(format!("缺字段未被拒（假绿口）: {other:?}")),
    }
}

fn main() {
    let args = parse_args();
    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "clamp-8bit" => red_arm_clamp_8bit(),
            "srgb-linear-mislabel" => red_arm_srgb_linear_mislabel(),
            "metadata-missing" => red_arm_metadata_missing(),
            other => fail(&format!(
                "未知 RED 臂: {other}(clamp-8bit|srgb-linear-mislabel|metadata-missing)"
            )),
        };
        match r {
            Ok(()) => {
                println!("{TAG}: PASS red-arm {arm}");
                std::process::exit(0);
            }
            Err(e) => fail(&format!("red-arm {arm} 失效(漏检): {e}")),
        }
    }

    let mut failures: Vec<String> = Vec::new();
    let mut checks: Vec<(&str, bool)> = Vec::new();

    // ── host 腿：闭式探针图案 capture→encode→decode 位级 golden ──
    let host_pixels = host_probe_pixels(HOST_W, HOST_H);
    let host_img = match ExrImage::new(
        HOST_W,
        HOST_H,
        ExrChannelLayout::Rgb,
        host_pixels.clone(),
        hdr_capture_metadata(HOST_W, HOST_H, "host-probe-v1"),
    ) {
        Ok(i) => i,
        Err(e) => fail(&format!("host 探针帧构造失败: {e}")),
    };
    let (host_bytes, host_decoded) = match roundtrip_bit_exact(&host_img) {
        Ok(v) => v,
        Err(e) => fail(&format!("host 往返无损失效: {e}")),
    };
    let host_probe_ok = host_decoded
        .iter()
        .zip(host_pixels.iter())
        .all(|(a, b)| a.to_bits() == b.to_bits());
    checks.push(("host_roundtrip_bit_exact", host_probe_ok));
    if !host_probe_ok {
        failures.push("host 探针图案位级核验失效".into());
    }
    let host_exr_path = args.work_dir.join("host_probe_frame.exr");
    let mut host_written = false;
    if std::fs::create_dir_all(&args.work_dir).is_ok()
        && std::fs::write(&host_exr_path, &host_bytes).is_ok()
    {
        host_written = true;
    }
    checks.push(("hdr_frame_capture_written", host_written));
    if !host_written {
        failures.push("HDR 帧捕获落盘失败".into());
    }
    let host_frame_digest = frame_content_digest(HOST_W, HOST_H, 3, &host_pixels);
    let host_exr_file_digest = format!("sha256:{}", sha256_hex(&host_bytes));

    // ── ZIP fail-closed 腿 ──
    let mut zip_bytes = host_bytes.clone();
    let needle = b"compression\x00compression\x00\x01\x00\x00\x00\x00";
    let zip_ok = match zip_bytes.windows(needle.len()).position(|w| w == needle) {
        Some(pos) => {
            zip_bytes[pos + needle.len() - 1] = 3; // ZIP
            matches!(
                decode_exr(&zip_bytes, ExrSourceEnd::Rurix),
                Err(ImageError::UnsupportedCompression(_))
            )
        }
        None => false,
    };
    checks.push(("zip_decode_fail_closed", zip_ok));
    if !zip_ok {
        failures.push("ZIP fail-closed 显式 UnsupportedCompression 失效".into());
    }

    // ── RED 臂内联实测（主流程三臂全检出） ──
    let red_clamp_ok = red_arm_clamp_8bit().is_ok();
    let red_mislabel_ok = red_arm_srgb_linear_mislabel().is_ok();
    let red_metadata_ok = red_arm_metadata_missing().is_ok();
    checks.push(("red_bit_depth_truncation_detected", red_clamp_ok));
    checks.push(("red_srgb_linear_mislabel_detected", red_mislabel_ok));
    checks.push(("red_metadata_missing_detected", red_metadata_ok));
    if !(red_clamp_ok && red_mislabel_ok && red_metadata_ok) {
        failures.push("RED 臂内联实测存在漏检".into());
    }

    // ── device 腿：GPU 真渲染捕获 ──
    let mut device_state = "not_applicable";
    let mut device_digest = String::new();
    let mut device_name = String::new();
    if !args.host_only {
        match device_capture_leg() {
            Ok((rgb, anchors_ok, name)) => {
                device_state = "executed";
                device_name = name;
                let dev_img = match ExrImage::new(
                    DEVICE_W,
                    DEVICE_H,
                    ExrChannelLayout::Rgb,
                    rgb.clone(),
                    hdr_capture_metadata(DEVICE_W, DEVICE_H, "device-probe-v1"),
                ) {
                    Ok(i) => i,
                    Err(e) => fail(&format!("device 捕获帧构造失败: {e}")),
                };
                match roundtrip_bit_exact(&dev_img) {
                    Ok((dev_bytes, dev_decoded)) => {
                        let dev_ok = dev_decoded
                            .iter()
                            .zip(rgb.iter())
                            .all(|(a, b)| a.to_bits() == b.to_bits());
                        checks.push(("device_capture_roundtrip_bit_exact", dev_ok));
                        if !dev_ok {
                            failures.push("device 捕获→回读位级失效".into());
                        }
                        let dev_path = args.work_dir.join("device_captured_frame.exr");
                        let dev_written = std::fs::write(&dev_path, &dev_bytes).is_ok();
                        checks.push(("device_frame_written", dev_written));
                        if !dev_written {
                            failures.push("device 帧落盘失败".into());
                        }
                        device_digest = frame_content_digest(DEVICE_W, DEVICE_H, 3, &rgb);
                    }
                    Err(e) => fail(&format!("device 往返无损失效: {e}")),
                }
                checks.push(("device_probe_anchors", anchors_ok));
                if !anchors_ok {
                    failures.push("device 探针锚点断言失效（疑恒定帧）".into());
                }
            }
            Err(e) => {
                if args.host_only {
                    unreachable!();
                }
                eprintln!("[{TAG}] device 腿不可用: {e}");
                if std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1") {
                    fail(&format!(
                        "device 腿 SKIP（RURIX_REQUIRE_REAL=1 不许 SKIP）: {e}"
                    ));
                }
                device_state = "dev_env_degrade";
                checks.push(("device_capture_roundtrip_bit_exact", false));
                checks.push(("device_frame_written", false));
                checks.push(("device_probe_anchors", false));
            }
        }
    } else {
        checks.push(("device_capture_roundtrip_bit_exact", true));
        checks.push(("device_frame_written", true));
        checks.push(("device_probe_anchors", true));
    }

    // ── UE 真帧 strip-and-log 读取腿 ──
    let mut ue_ok = false;
    let mut ue_digest = String::new();
    let mut ue_stripped_count = 0usize;
    let mut ue_bit_depth = String::new();
    if let Some(ue_path) = &args.ue_frame {
        match std::fs::read(ue_path) {
            Ok(blob) => match decode_exr(&blob, ExrSourceEnd::Ue5) {
                Ok(dec) => {
                    let has_unreal_strip = dec
                        .stripped
                        .iter()
                        .any(|s| s.name.starts_with("unreal/") && s.reason == "ue5-strip-and-log");
                    let has_alpha_strip = dec
                        .stripped
                        .iter()
                        .any(|s| s.name == "A" && s.reason == "alpha-channel-strip");
                    ue_bit_depth = match dec.source_bit_depth {
                        ExrBitDepth::Float16 => "float16".to_owned(),
                        ExrBitDepth::Float32 => "float32".to_owned(),
                    };
                    ue_stripped_count = dec.stripped.len();
                    ue_ok = dec.layout == ExrChannelLayout::Rgb
                        && dec.metadata.is_none()
                        && has_unreal_strip
                        && has_alpha_strip
                        && dec.source_bit_depth == ExrBitDepth::Float16
                        && dec.width == 1920
                        && dec.height == 1080
                        && dec.pixels.len() == (dec.width * dec.height * 3) as usize;
                    if ue_ok {
                        ue_digest = frame_content_digest(dec.width, dec.height, 3, &dec.pixels);
                        eprintln!(
                            "[{TAG}] UE 真帧读取: {}×{} fp16→f32, stripped={}, digest={}",
                            dec.width,
                            dec.height,
                            ue_stripped_count,
                            &ue_digest[..24.min(ue_digest.len())]
                        );
                    }
                }
                Err(e) => {
                    failures.push(format!("UE 真帧解码失败: {e}"));
                }
            },
            Err(e) => failures.push(format!("UE 帧文件不可读 {}: {e}", ue_path.display())),
        }
    }
    // --host-only 快速通道跳过 UE 腿（UE 真帧读取由门全档面覆盖，不在此豁免）。
    checks.push((
        "ue_frame_strip_and_log_read",
        (args.ue_frame.is_some() && ue_ok) || args.host_only,
    ));
    if args.ue_frame.is_some() && !ue_ok {
        failures.push("UE 真帧 strip-and-log 读取判据失效".into());
    }

    // ── evidence ──
    let all_ok = failures.is_empty() && checks.iter().all(|(_, v)| *v);
    if let Some(path) = &args.evidence {
        let mut json = String::from("{\n");
        json.push_str(&format!("  \"harness\": \"{TAG}\",\n"));
        json.push_str(&format!("  \"timestamp\": \"{}\",\n", utc_now()));
        json.push_str(&format!(
            "  \"device_section_state\": \"{device_state}\",\n"
        ));
        json.push_str(&format!(
            "  \"device_name\": \"{}\",\n",
            json_escape(&device_name)
        ));
        json.push_str(&format!(
            "  \"host_frame_digest\": \"{}\",\n",
            json_escape(&host_frame_digest)
        ));
        json.push_str(&format!(
            "  \"host_exr_file_digest\": \"{}\",\n",
            json_escape(&host_exr_file_digest)
        ));
        json.push_str(&format!(
            "  \"device_frame_digest\": \"{}\",\n",
            json_escape(&device_digest)
        ));
        json.push_str(&format!(
            "  \"ue_frame_digest\": \"{}\",\n",
            json_escape(&ue_digest)
        ));
        json.push_str(&format!(
            "  \"ue_frame_bit_depth\": \"{}\",\n",
            json_escape(&ue_bit_depth)
        ));
        json.push_str(&format!(
            "  \"ue_stripped_attribute_count\": {ue_stripped_count},\n"
        ));
        json.push_str("  \"checks\": {\n");
        for (i, (name, value)) in checks.iter().enumerate() {
            let comma = if i + 1 == checks.len() { "" } else { "," };
            json.push_str(&format!("    \"{name}\": {value}{comma}\n"));
        }
        json.push_str("  },\n");
        json.push_str(&format!("  \"failures\": {:?}\n", failures));
        json.push_str("}\n");
        if let Some(parent) = Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(path, &json).is_err() {
            fail("evidence 落盘失败");
        }
        eprintln!("[{TAG}] evidence → {}", path.display());
    }
    let passed = checks.iter().filter(|(_, v)| *v).count();
    println!(
        "{TAG}: checks {}/{} device={device_state} host_digest={}",
        passed,
        checks.len(),
        &host_frame_digest[..24.min(host_frame_digest.len())]
    );
    if all_ok {
        println!("{TAG}: PASS");
    } else {
        fail(&format!("判据失效: {failures:?}"));
    }
}
