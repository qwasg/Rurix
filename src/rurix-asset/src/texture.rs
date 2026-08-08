//! M83 纹理 cook 节点(`rurix.texture.cook.v1`)。
//!
//! source RGBA → 四腿产物(KTX2 / Basis / BCn / ASTC),全部由真实
//! `basis_universal`(BinomialLLC 1.16.4)驱动:
//!
//! - `texture.ktx2` = 真实 UASTC 4×4 KTX2 容器(supercompressionScheme=0);
//! - `texture.basis` = 真实 ETC1S `.basis` 码流(签名 `sB`;**不再**由 RXBS 冒充);
//! - `texture.bcn`  = KTX2 → 真 transcode → BC7(color)/BC5(normal)/BC4(mask),装 `RXBC`;
//! - `texture.astc` = KTX2 → 真 transcode → ASTC 4×4 实块,装 `RXAS`。
//!
//! `RXBC`/`RXAS` 是 Rurix 自有的 **GPU 块容器**(非 `.basis`/非 KTX2),
//! 承担「块字节 + 格式标注」的落盘角色,合法保留(RXS-0334 产物表)。
//! //@ spec: RXS-0334

use crate::bcdec::{
    alpha_coverage_delta, astc4x4_block_stats, decode_bc4_r8, decode_bc5_rg8, decode_bc7_rgba8,
    max_channel_delta, max_channel_delta_channels, normal_length_mean_abs_dev,
};
use crate::ktx2::{
    KTX2_MAGIC, RXAS_MAGIC, RXBC_FMT_BC4, RXBC_FMT_BC5, RXBC_FMT_BC7, RXBC_MAGIC, write_rxas,
    write_rxbc,
};
use rurix_basis_sys::{
    self as basis, BASIS_SIG, ContainerMode, SrcKind, TargetFormat, VENDOR_VERSION,
};
use rurix_pkg::sha256::Sha256;
use std::fs;
use std::path::{Path, PathBuf};

/// AP-TEX 冻结 tolerance(首批 measured 后字面冻结;RXS-0334)。
pub const COLOR_MAX_CHANNEL_DELTA: u8 = 48;
pub const NORMAL_LENGTH_MEAN_ABS_DEV: f64 = 0.15;
pub const ALPHA_COVERAGE_DELTA: f64 = 0.08;
pub const ALPHA_COVERAGE_THRESHOLD: u8 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureSemantics {
    Color,
    Normal,
    Mask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookProfile {
    /// win-vulkan-bcn-v1:BC7 color / 同路径 normal·mask(过渡期未分 BC5/BC4)。
    WinVulkanBcnV1,
    /// mobile-astc-v1:ASTC 4×4。
    MobileAstcV1,
}

impl CookProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            CookProfile::WinVulkanBcnV1 => "win-vulkan-bcn-v1",
            CookProfile::MobileAstcV1 => "mobile-astc-v1",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "win-vulkan-bcn-v1" => Some(Self::WinVulkanBcnV1),
            "mobile-astc-v1" => Some(Self::MobileAstcV1),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CookReport {
    pub codec_version: String,
    pub width: u32,
    pub height: u32,
    pub ktx2_path: PathBuf,
    pub basis_path: Option<PathBuf>,
    pub bcn_path: PathBuf,
    pub astc_path: PathBuf,
    pub ktx2_digest: String,
    pub bcn_digest: String,
    pub astc_digest: String,
    pub basis_present: bool,
    pub gpu_format_bcn: String,
    pub gpu_format_astc: String,
    pub color_max_delta: u8,
    pub normal_length_mad: f64,
    pub alpha_coverage_delta: f64,
    /// BCn 腿块数(== ceil(w/4)*ceil(h/4);防"改扩展名/截断"假腿)。
    pub bcn_block_count: u32,
    /// ASTC 腿块数与 void-extent 块数(void-extent 全覆盖 = 常色 fudge,判 FAIL)。
    pub astc_block_count: u32,
    pub astc_void_extent_blocks: u32,
    /// ASTC 真实权重块数(weighted > 0 证明非全 void-extent 常色 fudge)。
    pub astc_weighted_blocks: u32,
    /// `.basis` 腿真 transcode 回环:ETC1S → BC7 的块数与 digest
    /// (证 `.basis` 是可解码码流,而非仅 magic 对的空壳)。
    pub basis_transcode_block_count: u32,
    pub basis_transcode_digest: String,
}

#[derive(Debug)]
pub enum CookError {
    Io(String),
    Codec(String),
    Parse(String),
}

impl std::fmt::Display for CookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CookError::Io(s) | CookError::Codec(s) | CookError::Parse(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for CookError {}

fn digest_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let d = h.finalize();
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// 源图每个 4×4 分块是否都是常色。
///
/// 用途:ASTC 全 void-extent 只有在源图**逐块常色**时才是合法结果
/// (如 16×16 checker 的 4×4 单元);否则即为常色 fudge → FAIL。
fn all_blocks_constant_color(rgba: &[u8], width: u32, height: u32) -> bool {
    let bw = width.div_ceil(4);
    let bh = height.div_ceil(4);
    for by in 0..bh {
        for bx in 0..bw {
            let mut first: Option<[u8; 4]> = None;
            for ty in 0..4u32 {
                for tx in 0..4u32 {
                    let x = (bx * 4 + tx).min(width - 1);
                    let y = (by * 4 + ty).min(height - 1);
                    let i = (y as usize * width as usize + x as usize) * 4;
                    if i + 4 > rgba.len() {
                        return false;
                    }
                    let px = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
                    match first {
                        None => first = Some(px),
                        Some(f) if f != px => return false,
                        _ => {}
                    }
                }
            }
        }
    }
    true
}

/// 程序化 checker 16×16 RGBA(确定性 fixture)。
pub fn fixture_checker_rgba16() -> (u32, u32, Vec<u8>) {
    let w = 16u32;
    let h = 16u32;
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let on = ((x / 4) + (y / 4)) % 2 == 0;
            if on {
                v.extend_from_slice(&[220, 40, 40, 255]);
            } else {
                v.extend_from_slice(&[40, 40, 220, 200]);
            }
        }
    }
    (w, h, v)
}

/// 程序化渐变色图 32×32(设计案 §6 fixture 清单)。
///
/// 非常色块 —— 使 ASTC 腿必须产出**非 void-extent** 实块,
/// 是「禁 void-extent/均值敷衍」的判据载体。
pub fn fixture_gradient_rgba32() -> (u32, u32, Vec<u8>) {
    let w = 32u32;
    let h = 32u32;
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let r = (x * 255 / (w - 1)) as u8;
            let g = (y * 255 / (h - 1)) as u8;
            let b = ((x + y) * 255 / (w + h - 2)) as u8;
            v.extend_from_slice(&[r, g, b, 255]);
        }
    }
    (w, h, v)
}

/// alpha mask 16×16(设计案 §6 fixture 清单;圆形覆盖)。
pub fn fixture_mask_rgba16() -> (u32, u32, Vec<u8>) {
    let w = 16u32;
    let h = 16u32;
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = (x as f64 + 0.5) / w as f64 * 2.0 - 1.0;
            let fy = (y as f64 + 0.5) / h as f64 * 2.0 - 1.0;
            let inside = fx * fx + fy * fy <= 0.64;
            let m = if inside { 255u8 } else { 0u8 };
            v.extend_from_slice(&[m, m, m, m]);
        }
    }
    (w, h, v)
}

/// 半球 normal map 16×16。
pub fn fixture_normal_rgba16() -> (u32, u32, Vec<u8>) {
    let w = 16u32;
    let h = 16u32;
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = (x as f64 + 0.5) / w as f64 * 2.0 - 1.0;
            let fy = (y as f64 + 0.5) / h as f64 * 2.0 - 1.0;
            let z2 = (1.0 - fx * fx - fy * fy).max(0.0);
            let fz = z2.sqrt();
            let r = ((fx * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
            let g = ((fy * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
            let b = ((fz * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
            v.extend_from_slice(&[r, g, b, 255]);
        }
    }
    (w, h, v)
}

/// 解码 PPM P6(镜像 image-io 编码布局;本切片不引第二解码器依赖面之外的路径)。
pub fn decode_ppm_p6(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), CookError> {
    if !bytes.starts_with(b"P6\n") {
        return Err(CookError::Parse("PPM: magic 非 P6".into()));
    }
    let mut i = 3usize;
    // skip comments
    while i < bytes.len() && bytes[i] == b'#' {
        while i < bytes.len() && bytes[i] != b'\n' {
            i += 1;
        }
        i += 1;
    }
    let rest = std::str::from_utf8(&bytes[i..]).map_err(|e| CookError::Parse(e.to_string()))?;
    let mut parts = rest.split_whitespace();
    let w: u32 = parts
        .next()
        .ok_or_else(|| CookError::Parse("PPM: 缺 width".into()))?
        .parse()
        .map_err(|e| CookError::Parse(format!("{e}")))?;
    let h: u32 = parts
        .next()
        .ok_or_else(|| CookError::Parse("PPM: 缺 height".into()))?
        .parse()
        .map_err(|e| CookError::Parse(format!("{e}")))?;
    let maxv: u32 = parts
        .next()
        .ok_or_else(|| CookError::Parse("PPM: 缺 maxval".into()))?
        .parse()
        .map_err(|e| CookError::Parse(format!("{e}")))?;
    if maxv != 255 {
        return Err(CookError::Parse("PPM: 仅支持 maxval=255".into()));
    }
    // binary payload starts after the header newline following maxval.
    let header_prefix = format!("P6\n{w} {h}\n255\n");
    // Robust: find last header newline by scanning for "\n255\n"
    let Some(pos) = find_ppm_payload(bytes) else {
        return Err(CookError::Parse("PPM: 无法定位 payload".into()));
    };
    let rgb = &bytes[pos..];
    let need = (w as usize) * (h as usize) * 3;
    if rgb.len() < need {
        return Err(CookError::Parse("PPM: payload 截断".into()));
    }
    let mut rgba = Vec::with_capacity(need / 3 * 4);
    for pix in rgb[..need].chunks_exact(3) {
        rgba.extend_from_slice(&[pix[0], pix[1], pix[2], 255]);
    }
    let _ = header_prefix;
    Ok((w, h, rgba))
}

fn find_ppm_payload(bytes: &[u8]) -> Option<usize> {
    // After "P6\n", parse ASCII header until a single whitespace-separated maxval then one newline.
    if !bytes.starts_with(b"P6\n") {
        return None;
    }
    let mut i = 3;
    let mut vals = 0u8;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            i += 1;
            continue;
        }
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // number
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        vals += 1;
        if vals == 3 {
            // maxval consumed; expect single newline then payload
            if i < bytes.len() && bytes[i] == b'\n' {
                return Some(i + 1);
            }
            // skip spaces then newline
            while i < bytes.len() && bytes[i].is_ascii_whitespace() && bytes[i] != b'\n' {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'\n' {
                return Some(i + 1);
            }
            return None;
        }
    }
    None
}

pub fn encode_ppm_p6(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let header = format!("P6\n{width} {height}\n255\n");
    let mut out = Vec::with_capacity(header.len() + (width * height * 3) as usize);
    out.extend_from_slice(header.as_bytes());
    for pix in rgba.chunks_exact(4) {
        out.extend_from_slice(&pix[..3]);
    }
    out
}

/// Cook 纹理到 `out_dir`,写四腿文件名约定:`texture.ktx2` / `texture.basis`(可选) /
/// `texture.bcn` / `texture.astc` + `cook_report.json`。
pub fn cook_texture(
    rgba: &[u8],
    width: u32,
    height: u32,
    semantics: TextureSemantics,
    profile: CookProfile,
    out_dir: &Path,
) -> Result<CookReport, CookError> {
    fs::create_dir_all(out_dir).map_err(|e| CookError::Io(e.to_string()))?;
    let ver = basis::version_string();
    if ver != VENDOR_VERSION {
        return Err(CookError::Codec(format!(
            "codec version drift: got {ver}, want {VENDOR_VERSION}"
        )));
    }

    // normal 语义按 XY 流铺陈(R→RGB、G→A),使 BC5 腿 X=R / Y=G 成立。
    let swizzle_rg = semantics == TextureSemantics::Normal;

    // ① 真实 UASTC 4×4 KTX2 容器(无 supercompression)。
    let ktx2 = basis::encode_container(rgba, width, height, ContainerMode::UastcKtx2, swizzle_rg)
        .map_err(|e| CookError::Codec(e.to_string()))?;
    if !ktx2.starts_with(KTX2_MAGIC) {
        return Err(CookError::Codec(
            "KTX2 容器 magic 非法(非真实 basisu 产物)".into(),
        ));
    }

    // ② 真实 ETC1S `.basis` 码流。
    let basis_file =
        basis::encode_container(rgba, width, height, ContainerMode::Etc1sBasis, swizzle_rg)
            .map_err(|e| CookError::Codec(e.to_string()))?;
    if !basis_file.starts_with(&BASIS_SIG) {
        return Err(CookError::Codec(
            "`.basis` 签名非法:必须为真实 basis_universal 码流(禁 RXBS 冒充)".into(),
        ));
    }

    // ③ BCn 腿:按语义选目标格式,真 transcode(非重打包)。
    let (bcn_target, bcn_fmt_id, bcn_format_name) = match semantics {
        TextureSemantics::Color => (TargetFormat::Bc7Rgba, RXBC_FMT_BC7, "BC7_UNORM"),
        TextureSemantics::Normal => (TargetFormat::Bc5Rg, RXBC_FMT_BC5, "BC5_UNORM"),
        TextureSemantics::Mask => (TargetFormat::Bc4R, RXBC_FMT_BC4, "BC4_UNORM"),
    };
    let bcn_blocks = basis::transcode(&ktx2, SrcKind::Ktx2, bcn_target)
        .map_err(|e| CookError::Codec(e.to_string()))?;
    if bcn_blocks.blocks.iter().all(|&b| b == 0) {
        return Err(CookError::Codec("BCn transcode 全零占位".into()));
    }

    // ④ ASTC 4×4 腿:真 transcode 实块。
    let astc = basis::transcode(&ktx2, SrcKind::Ktx2, TargetFormat::Astc4x4)
        .map_err(|e| CookError::Codec(e.to_string()))?;
    if astc.blocks.iter().all(|&b| b == 0) {
        return Err(CookError::Codec("ASTC transcode 全零占位".into()));
    }

    let bcn = write_rxbc(bcn_fmt_id, width, height, &bcn_blocks.blocks);
    let astc_file = write_rxas(width, height, &astc.blocks);
    let _ = profile;

    let ktx2_path = out_dir.join("texture.ktx2");
    let bcn_path = out_dir.join("texture.bcn");
    let astc_path = out_dir.join("texture.astc");
    let basis_path = out_dir.join("texture.basis");
    fs::write(&ktx2_path, &ktx2).map_err(|e| CookError::Io(e.to_string()))?;
    fs::write(&bcn_path, &bcn).map_err(|e| CookError::Io(e.to_string()))?;
    fs::write(&astc_path, &astc_file).map_err(|e| CookError::Io(e.to_string()))?;
    fs::write(&basis_path, &basis_file).map_err(|e| CookError::Io(e.to_string()))?;
    let basis_present = true;

    // 独立解码对拍(bcdec 不引 rurix-basis-sys):按腿的真实格式解码。
    // color → BC7 全通道;normal → BC5 的 XY;mask → BC4 的 R。
    let decoded = match semantics {
        TextureSemantics::Color => decode_bc7_rgba8(&bcn_blocks.blocks, width, height),
        TextureSemantics::Normal => decode_bc5_rg8(&bcn_blocks.blocks, width, height),
        TextureSemantics::Mask => decode_bc4_r8(&bcn_blocks.blocks, width, height),
    };
    let color_max_delta = match semantics {
        // normal/mask 腿只承载 XY / R 通道,整幅 RGBA 比对无意义:
        // 仅对参与编码的通道计误差。
        TextureSemantics::Color => max_channel_delta(rgba, &decoded),
        TextureSemantics::Normal => max_channel_delta_channels(rgba, &decoded, &[0, 1]),
        TextureSemantics::Mask => max_channel_delta_channels(rgba, &decoded, &[0]),
    };
    let normal_length_mad = normal_length_mean_abs_dev(&decoded);
    let alpha_coverage_delta = alpha_coverage_delta(rgba, &decoded, ALPHA_COVERAGE_THRESHOLD);

    // 块计量:BCn/ASTC 各腿块数须 == ceil(w/4)*ceil(h/4)。
    let expect_blocks = width.div_ceil(4) * height.div_ceil(4);
    // 每块字节数按格式区分:BC4 = 8B/块(单通道),BC5/BC7/ASTC 4×4 = 16B/块。
    let bcn_bytes_per_block: usize = match bcn_target {
        TargetFormat::Bc4R => 8,
        TargetFormat::Bc5Rg | TargetFormat::Bc7Rgba | TargetFormat::Astc4x4 => 16,
    };
    let bcn_block_count = (bcn_blocks.blocks.len() / bcn_bytes_per_block) as u32;
    let astc_block_count = (astc.blocks.len() / 16) as u32;
    if bcn_block_count != expect_blocks || astc_block_count != expect_blocks {
        return Err(CookError::Codec(format!(
            "块数不符: bcn={bcn_block_count} astc={astc_block_count} expect={expect_blocks}"
        )));
    }
    let astc_stats = astc4x4_block_stats(&astc.blocks);
    let astc_void_extent_blocks = astc_stats.void_extent as u32;
    let astc_weighted_blocks = astc_stats.weighted as u32;
    if astc_stats.all_zero > 0 {
        return Err(CookError::Codec(format!(
            "ASTC 出现全零占位块: {} 块",
            astc_stats.all_zero
        )));
    }
    if astc_block_count > 0 && astc_void_extent_blocks == astc_block_count {
        // 全 void-extent = 常色 fudge,设计案 §3.6 显式禁止。
        // 例外:源本身每个 4×4 分块皆为常色时,void-extent 是**正确**编码结果
        // (16×16 checker 的 4×4 单元恰属此类),不构成 fudge。
        if !all_blocks_constant_color(rgba, width, height) {
            return Err(CookError::Codec(
                "ASTC 全块 void-extent 但源非常色分块:疑常色 fudge".into(),
            ));
        }
    }

    // `.basis` 腿真 transcode 回环:证其为可解码 ETC1S 码流。
    let basis_rt = basis::transcode(&basis_file, SrcKind::Basis, TargetFormat::Bc7Rgba)
        .map_err(|e| CookError::Codec(format!("`.basis` 回环 transcode 失败: {e}")))?;
    let basis_transcode_block_count = (basis_rt.blocks.len() / 16) as u32;
    if basis_transcode_block_count != expect_blocks {
        return Err(CookError::Codec(format!(
            "`.basis` 回环块数不符: {basis_transcode_block_count} != {expect_blocks}"
        )));
    }
    if basis_rt.blocks.iter().all(|&b| b == 0) {
        return Err(CookError::Codec("`.basis` 回环产出全零".into()));
    }
    let basis_transcode_digest = digest_hex(&basis_rt.blocks);

    let report = CookReport {
        codec_version: ver.to_string(),
        width,
        height,
        ktx2_path: ktx2_path.clone(),
        basis_path: Some(basis_path.clone()),
        bcn_path: bcn_path.clone(),
        astc_path: astc_path.clone(),
        ktx2_digest: digest_hex(&ktx2),
        bcn_digest: digest_hex(&bcn),
        astc_digest: digest_hex(&astc_file),
        basis_present,
        gpu_format_bcn: bcn_format_name.to_string(),
        gpu_format_astc: "ASTC_4x4_UNORM".into(),
        color_max_delta,
        normal_length_mad,
        alpha_coverage_delta,
        bcn_block_count,
        astc_block_count,
        astc_void_extent_blocks,
        astc_weighted_blocks,
        basis_transcode_block_count,
        basis_transcode_digest,
    };

    let json = format!(
        "{{\n  \"codec_version\": \"{}\",\n  \"upstream_tag\": \"{}\",\n  \"upstream_commit\": \"{}\",\n  \"width\": {},\n  \"height\": {},\n  \"ktx2_digest\": \"{}\",\n  \"bcn_digest\": \"{}\",\n  \"astc_digest\": \"{}\",\n  \"basis_present\": {},\n  \"basis_signature\": \"{}\",\n  \"basis_transcode_block_count\": {},\n  \"basis_transcode_digest\": \"{}\",\n  \"gpu_format_bcn\": \"{}\",\n  \"gpu_format_astc\": \"{}\",\n  \"bcn_block_count\": {},\n  \"astc_block_count\": {},\n  \"astc_void_extent_blocks\": {},\n  \"astc_weighted_blocks\": {},\n  \"expected_block_count\": {},\n  \"color_max_delta\": {},\n  \"normal_length_mad\": {:.6},\n  \"alpha_coverage_delta\": {:.6},\n  \"ktx2_magic_ok\": {},\n  \"bcn_magic_ok\": {},\n  \"astc_magic_ok\": {},\n  \"supercompression\": 0\n}}\n",
        report.codec_version,
        basis::UPSTREAM_TAG,
        basis::UPSTREAM_COMMIT,
        report.width,
        report.height,
        report.ktx2_digest,
        report.bcn_digest,
        report.astc_digest,
        report.basis_present,
        // `.basis` 签名字面(ASCII);真实上游 = "sB"(packed_uint<2> LE of 0x4273)。
        String::from_utf8_lossy(&basis_file[..2]),
        report.basis_transcode_block_count,
        report.basis_transcode_digest,
        report.gpu_format_bcn,
        report.gpu_format_astc,
        report.bcn_block_count,
        report.astc_block_count,
        report.astc_void_extent_blocks,
        report.astc_weighted_blocks,
        expect_blocks,
        report.color_max_delta,
        report.normal_length_mad,
        report.alpha_coverage_delta,
        ktx2.starts_with(KTX2_MAGIC),
        bcn.starts_with(RXBC_MAGIC),
        astc_file.starts_with(RXAS_MAGIC),
    );
    fs::write(out_dir.join("cook_report.json"), json).map_err(|e| CookError::Io(e.to_string()))?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cook_twice_byte_equal() {
        let (w, h, rgba) = fixture_checker_rgba16();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rurix_m83_cook_{nanos}"));
        let a = root.join("a");
        let b = root.join("b");
        let ra = cook_texture(
            &rgba,
            w,
            h,
            TextureSemantics::Color,
            CookProfile::WinVulkanBcnV1,
            &a,
        )
        .unwrap();
        let rb = cook_texture(
            &rgba,
            w,
            h,
            TextureSemantics::Color,
            CookProfile::WinVulkanBcnV1,
            &b,
        )
        .unwrap();
        assert_eq!(ra.ktx2_digest, rb.ktx2_digest);
        assert_eq!(ra.bcn_digest, rb.bcn_digest);
        assert_eq!(
            fs::read(&ra.bcn_path).unwrap(),
            fs::read(&rb.bcn_path).unwrap()
        );
        assert!(ra.color_max_delta <= COLOR_MAX_CHANNEL_DELTA);
        let _ = fs::remove_dir_all(&root);
    }
}
