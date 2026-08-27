//! G31+ 波 C Task C14 KTX2-3 通用转码收益 A/B measured harness
//! (门 g31.waveC.ktx2;G31_PLUS_COMMERCIAL_RENDERER_TODO #39;RD-041 分项;
//! milestones/g22/g22_ktx2_disposition.json KTX2-3 行「资产分发面需求成立窗」兑现)。
//!
//! 职责闭集:bistro 12 槽 DDS 面(top-12 三角数映射律法 baseColor 集,uri 由 CI
//! 独立重算经 --textures 传入)逐纹理——
//!   ① 原始分发面:DDS 文件实测字节(全链 as-shipped)+ level0 等效字节(算式);
//!   ② KTX2-UASTC 分发面:DDS 真实解码(bcdec decode_dds)→ 确定性 box 滤波全 mip
//!      链 → 逐级真实 basis encoder UASTC → write_ktx2_multilevel 组装全链 KTX2
//!      (真实负载 + spec 容器)→ 实测字节 + 逐级真 transcode 耗时(同机 measured);
//!   ③ ETC1S `.basis` 参照腿(BasisU 家族超压缩代表;在树 ETC1S 产 `.basis` 非
//!      KTX2,如实登记):逐 level 真实 ETC1S 编码文件集合合计字节 + 转码耗时;
//!   ④ 质量对拍:UASTC→BC7 / ETC1S→BC7 回解码(bcdec)vs DDS 解码 RGBA(现行
//!      出货像素)max/mean 通道差(AP-TEX 冻结容差 48 判据面);
//!   ⑤ KTX2-1 消费证明:逐产出件 parse_ktx2 互核(levelCount/尺寸律法/scheme=0/
//!      uncompressed==length/KTXwriter 如实登记字面)+ 双解析位级一致 + 首张
//!      UASTC 双编码位级一致(确定性面)。
//!
//! 全部数字同机 measured_local;容器组装不构造像素/块数据(禁手写二进制冒充)。
//!
//! 用法:
//!   g31_ktx2_ab --dds-dir <dir> --textures uri1,uri2,... --out <evidence.json>
//!               [--limit N] [--no-etc1s]
//!
//! Assisted-by: Kimi-K3(G31+ 波 C Task C14)

#![forbid(unsafe_code)]

use rurix_asset::bcdec::{decode_bc7_rgba8, decode_dds};
use rurix_asset::ktx2::{KTX2_SS_NONE, parse_ktx2, write_ktx2_multilevel};
use rurix_asset::texture::COLOR_MAX_CHANNEL_DELTA;
use rurix_basis_sys::{self as basis, ContainerMode, SrcKind, TargetFormat, VENDOR_VERSION};
use std::path::PathBuf;
use std::time::Instant;

const TAG: &str = "G31_KTX2_AB";
/// alpha 量化容差界(UASTC mode 7 alpha = 5+1 bit 端点;语义翻转 = ≥128 级差,
/// 量化噪声 ≤ 16 带内——ktx2.rs punchthrough 机制锚同字面)。
const ALPHA_DELTA_BOUND: u8 = 16;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

/// 确定性 box 滤波 2×2 均值(四通道 round-half-up;奇数维钳制边缘)。
fn downsample_box(rgba: &[u8], w: u32, h: u32) -> (u32, u32, Vec<u8>) {
    let nw = (w / 2).max(1);
    let nh = (h / 2).max(1);
    let mut out = Vec::with_capacity((nw * nh * 4) as usize);
    for y in 0..nh {
        for x in 0..nw {
            let mut acc = [0u32; 4];
            for dy in 0..2u32 {
                for dx in 0..2u32 {
                    let sx = (x * 2 + dx).min(w - 1);
                    let sy = (y * 2 + dy).min(h - 1);
                    let i = ((sy * w + sx) * 4) as usize;
                    for (a, &px) in acc.iter_mut().zip(&rgba[i..i + 4]) {
                        *a += u32::from(px);
                    }
                }
            }
            for a in acc {
                out.push(((a + 2) / 4) as u8);
            }
        }
    }
    (nw, nh, out)
}

/// 逐通道 max 绝对差 [R,G,B,A](对拍通道面诊断)。
fn per_channel_max_delta(src: &[u8], dec: &[u8]) -> [u8; 4] {
    let mut m = [0u8; 4];
    let n = src.len().min(dec.len()) / 4;
    for i in 0..n {
        for c in 0..4 {
            let d = src[i * 4 + c].abs_diff(dec[i * 4 + c]);
            if d > m[c] {
                m[c] = d;
            }
        }
    }
    m
}

/// premultiplied-aware 对拍(punchthrough 语义;ktx2.rs
/// `punchthrough_alpha_roundtrip_semantics` 机制锚同口径):
/// - RGB 仅在 mask(src.a>0 或 dec.a>0)像素内比对(透明像素 RGB = 自由域);
/// - alpha 全幅比对(codec 端点量化容差面;语义翻转 = ≥128 级差)。
/// 返回 (rgb_max_masked, rgb_mean_masked, alpha_max, punchthrough_px)。
fn masked_rgb_alpha_delta(src: &[u8], dec: &[u8]) -> (u8, f64, u8, u64) {
    let n = src.len().min(dec.len()) / 4;
    let mut rgb_max = 0u8;
    let mut alpha_max = 0u8;
    let mut sum = 0u128;
    let mut cnt = 0u64;
    let mut punch = 0u64;
    for i in 0..n {
        let sa = src[i * 4 + 3];
        let da = dec[i * 4 + 3];
        if sa == 0 {
            punch += 1;
        }
        let ad = sa.abs_diff(da);
        if ad > alpha_max {
            alpha_max = ad;
        }
        if sa > 0 || da > 0 {
            for c in 0..3 {
                let d = src[i * 4 + c].abs_diff(dec[i * 4 + c]);
                if d > rgb_max {
                    rgb_max = d;
                }
                sum += d as u128;
                cnt += 1;
            }
        }
    }
    let mean = if cnt > 0 {
        sum as f64 / cnt as f64
    } else {
        0.0
    };
    (rgb_max, mean, alpha_max, punch)
}

/// 生成全 mip 链(含 level 0)直至 1×1。
fn mip_chain(rgba0: &[u8], w: u32, h: u32) -> Vec<(u32, u32, Vec<u8>)> {
    let mut chain = vec![(w, h, rgba0.to_vec())];
    while chain.last().map(|(w, h, _)| *w > 1 || *h > 1) == Some(true) {
        let (w, h, rgba) = chain.last().expect("chain 非空");
        let (nw, nh, next) = downsample_box(rgba, *w, *h);
        chain.push((nw, nh, next));
    }
    chain
}

#[derive(Default)]
struct LegRow {
    // UASTC 腿
    ktx2_uastc_full_bytes: u64,
    ktx2_uastc_l0_bytes: u64,
    uastc_encode_ms: f64,
    uastc_transcode_full_ms: f64,
    uastc_transcode_l0_ms: f64,
    uastc_rgb_max_masked: u8,
    uastc_rgb_mean_masked: f64,
    uastc_alpha_max: u8,
    // ETC1S 参照腿
    etc1s_present: bool,
    etc1s_full_bytes: u64,
    etc1s_l0_bytes: u64,
    etc1s_encode_ms: f64,
    etc1s_transcode_full_ms: f64,
    etc1s_transcode_l0_ms: f64,
    etc1s_rgb_max_masked: u8,
    etc1s_rgb_mean_masked: f64,
    etc1s_alpha_max: u8,
}

fn f3(x: f64) -> String {
    format!("{x:.3}")
}

fn f6(x: f64) -> String {
    format!("{x:.6}")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut dds_dir: Option<PathBuf> = None;
    let mut textures: Vec<String> = vec![];
    let mut out_path: Option<PathBuf> = None;
    let mut limit: usize = 0;
    let mut with_etc1s = true;
    let mut dump_ktx2: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dds-dir" => {
                dds_dir = Some(PathBuf::from(
                    args.get(i + 1).unwrap_or_else(|| fail("--dds-dir 缺值")),
                ));
                i += 2;
            }
            "--textures" => {
                let v = args.get(i + 1).unwrap_or_else(|| fail("--textures 缺值"));
                textures = v
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                i += 2;
            }
            "--out" => {
                out_path = Some(PathBuf::from(
                    args.get(i + 1).unwrap_or_else(|| fail("--out 缺值")),
                ));
                i += 2;
            }
            "--dump-ktx2" => {
                dump_ktx2 = Some(PathBuf::from(
                    args.get(i + 1).unwrap_or_else(|| fail("--dump-ktx2 缺值")),
                ));
                i += 2;
            }
            "--limit" => {
                limit = args
                    .get(i + 1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| fail("--limit 缺值"));
                i += 2;
            }
            "--no-etc1s" => {
                with_etc1s = false;
                i += 1;
            }
            other => fail(&format!("未知参数 {other}")),
        }
    }
    let dds_dir = dds_dir.unwrap_or_else(|| fail("缺 --dds-dir"));
    let out_path = out_path.unwrap_or_else(|| fail("缺 --out"));
    if textures.is_empty() {
        fail("--textures 空集");
    }
    if limit > 0 {
        textures.truncate(limit);
    }
    let ver = basis::version_string();
    if ver != VENDOR_VERSION {
        fail(&format!("codec version drift: {ver} != {VENDOR_VERSION}"));
    }

    let t_wall = Instant::now();
    let mut rows: Vec<String> = Vec::new();
    let mut uris_json: Vec<String> = Vec::new();
    let mut files_parsed = 0u32;
    let mut parse_ok = true;
    let mut det_uastc_double_encode = true;
    let mut det_parse_double_read = true;
    // 总计面
    let mut tot = LegRow::default();
    let mut tot_dds_file = 0u64;
    let mut tot_dds_l0 = 0u64;
    let mut count = 0u32;
    let mut first = true;

    for uri in &textures {
        let path: PathBuf = dds_dir.join(uri);
        let raw = std::fs::read(&path).unwrap_or_else(|e| fail(&format!("读取 {uri} 失败: {e}")));
        let dds_file_bytes = raw.len() as u64;
        let img = decode_dds(&raw).unwrap_or_else(|e| fail(&format!("DDS 解码 {uri} 失败: {e}")));
        let (w, h) = (img.width, img.height);
        let dds_l0_bytes =
            (w.div_ceil(4) as u64) * (h.div_ceil(4) as u64) * img.format.block_bytes() as u64;
        let rgba8_digest = rurix_pkg::sha256::hex_digest(&img.rgba8);
        let chain = mip_chain(&img.rgba8, w, h);
        let levels_generated = chain.len() as u32;

        let mut row = LegRow::default();
        // ── UASTC 腿:逐级真实编码 → 全链组装 ──
        let mut payloads: Vec<Vec<u8>> = Vec::with_capacity(chain.len());
        let mut dfd_bytes: Vec<u8> = vec![];
        for (lv, (lw, lh, lrgba)) in chain.iter().enumerate() {
            let t0 = Instant::now();
            let k = basis::encode_container(lrgba, *lw, *lh, ContainerMode::UastcKtx2, false)
                .unwrap_or_else(|e| fail(&format!("UASTC 编码 {uri} level {lv} 失败: {e}")));
            row.uastc_encode_ms += t0.elapsed().as_secs_f64() * 1000.0;
            let fk =
                parse_ktx2(&k).unwrap_or_else(|e| fail(&format!("解析 UASTC 单级件失败: {e}")));
            if lv == 0 {
                dfd_bytes = fk
                    .dfd_slice(&k)
                    .unwrap_or_else(|| fail("DFD 区段缺失"))
                    .to_vec();
                // 确定性面:首纹理 level0 双编码位级一致。
                if first {
                    let k2 =
                        basis::encode_container(lrgba, *lw, *lh, ContainerMode::UastcKtx2, false)
                            .unwrap_or_else(|e| fail(&format!("UASTC 双编码 {uri} 失败: {e}")));
                    det_uastc_double_encode = k == k2;
                }
            }
            payloads.push(
                fk.level_slice(&k, 0)
                    .unwrap_or_else(|| fail("level 负载缺失"))
                    .to_vec(),
            );
        }
        let pl: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
        let ktx2_full = write_ktx2_multilevel(0, w, h, &dfd_bytes, &pl);
        let ktx2_l0 = write_ktx2_multilevel(0, w, h, &dfd_bytes, &pl[..1]);
        // CI 独立重解析互核面:首张 ≥1024 纹理的全链件落盘(--dump-ktx2)。
        if let Some(dp) = &dump_ktx2 {
            if w >= 1024 && !dp.exists() {
                if let Some(parent) = dp.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(dp, &ktx2_full)
                    .unwrap_or_else(|e| fail(&format!("dump-ktx2 落盘失败: {e}")));
            }
        }
        row.ktx2_uastc_full_bytes = ktx2_full.len() as u64;
        row.ktx2_uastc_l0_bytes = ktx2_l0.len() as u64;
        let ktx2_digest = rurix_pkg::sha256::hex_digest(&ktx2_full);

        // KTX2-1 消费证明:解析全链件逐字段互核 + 双解析位级一致。
        let fa = parse_ktx2(&ktx2_full).unwrap_or_else(|e| fail(&format!("解析全链件失败: {e}")));
        let fb =
            parse_ktx2(&ktx2_full).unwrap_or_else(|e| fail(&format!("解析全链件(双读)失败: {e}")));
        files_parsed += 1;
        det_parse_double_read &= fa == fb;
        parse_ok &= fa.header.level_count == levels_generated
            && fa.header.supercompression_scheme == KTX2_SS_NONE
            && fa.header.pixel_width == w
            && fa.header.pixel_height == h
            && fa.key_value("KTXwriter")
                == Some(b"rurix-asset ktx2.rs write_ktx2_multilevel".as_slice())
            && fa.levels.len() == chain.len()
            && chain.iter().enumerate().all(|(lv, (lw, lh, _))| {
                fa.level_dims(lv as u32) == Some((*lw, *lh, 1))
                    && fa.levels[lv].byte_length == payloads[lv].len() as u64
                    && fa.levels[lv].uncompressed_byte_length == payloads[lv].len() as u64
            })
            && fa.is_vendor_transcodable();

        // ── UASTC 逐级真 transcode(全链耗时 = 各级合计;level0 单列)──
        let mut bc7_l0: Option<Vec<u8>> = None;
        for lv in 0..chain.len() as u32 {
            let t0 = Instant::now();
            let t = basis::transcode_level(&ktx2_full, SrcKind::Ktx2, TargetFormat::Bc7Rgba, lv)
                .unwrap_or_else(|e| fail(&format!("UASTC 转码 {uri} level {lv} 失败: {e}")));
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            row.uastc_transcode_full_ms += ms;
            if lv == 0 {
                row.uastc_transcode_l0_ms = ms;
                if (t.width, t.height) != (w, h) {
                    fail(&format!(
                        "转码尺寸不符: {}x{} != {w}x{h}",
                        t.width, t.height
                    ));
                }
                bc7_l0 = Some(t.blocks);
            }
        }
        // 质量对拍:BC7 回解码 vs DDS 解码 RGBA(现行出货像素面;
        // premultiplied-aware 口径——透明像素 RGB 自由域,alpha 全幅量化容差)。
        let dec = decode_bc7_rgba8(bc7_l0.as_ref().expect("level0 块在"), w, h);
        let (urgb_max, urgb_mean, ualpha_max, punch_px) = masked_rgb_alpha_delta(&img.rgba8, &dec);
        row.uastc_rgb_max_masked = urgb_max;
        row.uastc_rgb_mean_masked = urgb_mean;
        row.uastc_alpha_max = ualpha_max;
        let uastc_ch = per_channel_max_delta(&img.rgba8, &dec);

        // ── ETC1S 参照腿:逐 level 真实 .basis 编码文件集合 ──
        let mut etc1s_ch = [0u8; 4];
        if with_etc1s {
            row.etc1s_present = true;
            let mut basis_levels: Vec<Vec<u8>> = Vec::with_capacity(chain.len());
            for (lv, (lw, lh, lrgba)) in chain.iter().enumerate() {
                let t0 = Instant::now();
                let b = basis::encode_container(lrgba, *lw, *lh, ContainerMode::Etc1sBasis, false)
                    .unwrap_or_else(|e| fail(&format!("ETC1S 编码 {uri} level {lv} 失败: {e}")));
                row.etc1s_encode_ms += t0.elapsed().as_secs_f64() * 1000.0;
                if lv == 0 {
                    row.etc1s_l0_bytes = b.len() as u64;
                }
                row.etc1s_full_bytes += b.len() as u64;
                basis_levels.push(b);
            }
            for (lv, b) in basis_levels.iter().enumerate() {
                let t0 = Instant::now();
                let t = basis::transcode(b, SrcKind::Basis, TargetFormat::Bc7Rgba)
                    .unwrap_or_else(|e| fail(&format!("ETC1S 转码 {uri} level {lv} 失败: {e}")));
                let ms = t0.elapsed().as_secs_f64() * 1000.0;
                row.etc1s_transcode_full_ms += ms;
                if lv == 0 {
                    row.etc1s_transcode_l0_ms = ms;
                    let dec = decode_bc7_rgba8(&t.blocks, w, h);
                    let (emax, emean, ealpha, _) = masked_rgb_alpha_delta(&img.rgba8, &dec);
                    row.etc1s_rgb_max_masked = emax;
                    row.etc1s_rgb_mean_masked = emean;
                    row.etc1s_alpha_max = ealpha;
                    etc1s_ch = per_channel_max_delta(&img.rgba8, &dec);
                }
            }
        }

        // 行 JSON
        uris_json.push(format!("\"{uri}\""));
        rows.push(format!(
            "    {{\"uri\":\"{uri}\",\"dds_format\":\"{fmt}\",\"width\":{w},\"height\":{h},\
\"mip_count_dds\":{mipd},\"levels_generated\":{lvg},\"rgba8_digest\":\"sha256:{dg}\",\
\"dds_file_bytes\":{ddf},\"dds_l0_bytes\":{ddl},\
\"ktx2_uastc_full_bytes\":{kuf},\"ktx2_uastc_l0_bytes\":{kul},\"ktx2_uastc_digest\":\"sha256:{kd}\",\
\"uastc_encode_ms\":{uem},\"uastc_transcode_full_ms\":{utf},\"uastc_transcode_l0_ms\":{utl},\
\"uastc_rgb_max_masked\":{umax},\"uastc_rgb_mean_masked\":{umean},\"uastc_alpha_max\":{ualpha},\
\"uastc_bc7_max_delta_ch\":[{uc0},{uc1},{uc2},{uc3}],\
\"etc1s_present\":{ep},\"etc1s_full_bytes\":{ef},\"etc1s_l0_bytes\":{el},\
\"etc1s_encode_ms\":{eem},\"etc1s_transcode_full_ms\":{etf},\"etc1s_transcode_l0_ms\":{etl},\
\"etc1s_rgb_max_masked\":{emax},\"etc1s_rgb_mean_masked\":{emean},\"etc1s_alpha_max\":{ealpha},\
\"etc1s_bc7_max_delta_ch\":[{ec0},{ec1},{ec2},{ec3}],\"punchthrough_px\":{punch}}}",
            fmt = img.format.as_str(),
            mipd = img.mip_count,
            lvg = levels_generated,
            dg = rgba8_digest,
            ddf = dds_file_bytes,
            ddl = dds_l0_bytes,
            kuf = row.ktx2_uastc_full_bytes,
            kul = row.ktx2_uastc_l0_bytes,
            kd = ktx2_digest,
            uem = f3(row.uastc_encode_ms),
            utf = f3(row.uastc_transcode_full_ms),
            utl = f3(row.uastc_transcode_l0_ms),
            umax = row.uastc_rgb_max_masked,
            umean = f6(row.uastc_rgb_mean_masked),
            ualpha = row.uastc_alpha_max,
            uc0 = uastc_ch[0], uc1 = uastc_ch[1], uc2 = uastc_ch[2], uc3 = uastc_ch[3],
            ep = row.etc1s_present,
            ef = row.etc1s_full_bytes,
            el = row.etc1s_l0_bytes,
            eem = f3(row.etc1s_encode_ms),
            etf = f3(row.etc1s_transcode_full_ms),
            etl = f3(row.etc1s_transcode_l0_ms),
            emax = row.etc1s_rgb_max_masked,
            emean = f6(row.etc1s_rgb_mean_masked),
            ealpha = row.etc1s_alpha_max,
            ec0 = etc1s_ch[0], ec1 = etc1s_ch[1], ec2 = etc1s_ch[2], ec3 = etc1s_ch[3],
            punch = punch_px,
        ));

        // 累计
        tot_dds_file += dds_file_bytes;
        tot_dds_l0 += dds_l0_bytes;
        tot.ktx2_uastc_full_bytes += row.ktx2_uastc_full_bytes;
        tot.ktx2_uastc_l0_bytes += row.ktx2_uastc_l0_bytes;
        tot.uastc_encode_ms += row.uastc_encode_ms;
        tot.uastc_transcode_full_ms += row.uastc_transcode_full_ms;
        tot.uastc_transcode_l0_ms += row.uastc_transcode_l0_ms;
        tot.uastc_rgb_max_masked = tot.uastc_rgb_max_masked.max(row.uastc_rgb_max_masked);
        tot.uastc_alpha_max = tot.uastc_alpha_max.max(row.uastc_alpha_max);
        tot.etc1s_present = row.etc1s_present;
        tot.etc1s_full_bytes += row.etc1s_full_bytes;
        tot.etc1s_l0_bytes += row.etc1s_l0_bytes;
        tot.etc1s_encode_ms += row.etc1s_encode_ms;
        tot.etc1s_transcode_full_ms += row.etc1s_transcode_full_ms;
        tot.etc1s_transcode_l0_ms += row.etc1s_transcode_l0_ms;
        tot.etc1s_rgb_max_masked = tot.etc1s_rgb_max_masked.max(row.etc1s_rgb_max_masked);
        tot.etc1s_alpha_max = tot.etc1s_alpha_max.max(row.etc1s_alpha_max);
        count += 1;
        first = false;
        println!(
            "[{TAG}] ({count}/{}) {uri}: dds={ddf}B uastc_full={kuf}B etc1s_full={ef}B uastc_enc={uem}ms etc1s_enc={eem}ms",
            textures.len(),
            ddf = dds_file_bytes,
            kuf = row.ktx2_uastc_full_bytes,
            ef = row.etc1s_full_bytes,
            uem = f3(row.uastc_encode_ms),
            eem = f3(row.etc1s_encode_ms),
        );
    }

    let wall_ms = t_wall.elapsed().as_secs_f64() * 1000.0;
    let json = format!(
        "{{\n  \"schema\": \"rurix.g31.ktx2_ab_evidence.v1\",\n  \"subject\": \"g31_ktx2_ab\",\n  \
\"codec_version\": \"{ver}\",\n  \
\"texture_set\": {{\"source\": \"bistro-interior top-N baseColor（G11.3 manifest 映射律法面;uri 由 CI 独立重算传入）\", \"count\": {count}, \"uris\": [{uris}]}},\n  \
\"bounds\": {{\"color_max_delta_bound\": {bound}, \"alpha_delta_bound\": {abound}, \"limit\": {limit}}},\n  \
\"textures\": [\n{rows}\n  ],\n  \
\"totals\": {{\"dds_file_bytes\": {tdf}, \"dds_l0_bytes\": {tdl}, \
\"ktx2_uastc_full_bytes\": {tkuf}, \"ktx2_uastc_l0_bytes\": {tkul}, \
\"uastc_encode_ms\": {tuem}, \"uastc_transcode_full_ms\": {tutf}, \"uastc_transcode_l0_ms\": {tutl}, \
\"uastc_rgb_max_masked\": {tumax}, \"uastc_alpha_max\": {tualpha}, \
\"etc1s_present\": {tep}, \"etc1s_full_bytes\": {tef}, \"etc1s_l0_bytes\": {tel}, \
\"etc1s_encode_ms\": {teem}, \"etc1s_transcode_full_ms\": {tetf}, \"etc1s_transcode_l0_ms\": {tetl}, \
\"etc1s_rgb_max_masked\": {temax}, \"etc1s_alpha_max\": {tealpha}}},\n  \
\"determinism\": {{\"uastc_encode_double_run_bitexact\": {d1}, \"parse_double_read_bitexact\": {d2}}},\n  \
\"parse_crosscheck\": {{\"files_parsed\": {fp}, \"ok\": {pok}}},\n  \
\"environment\": {{\"measured\": \"measured_local\", \"legs\": \"{legs}\"}},\n  \
\"wall_clock_ms\": {wall}\n}}\n",
        ver = ver,
        count = count,
        uris = uris_json.join(", "),
        bound = COLOR_MAX_CHANNEL_DELTA,
        abound = ALPHA_DELTA_BOUND,
        limit = limit,
        rows = rows.join(",\n"),
        tdf = tot_dds_file,
        tdl = tot_dds_l0,
        tkuf = tot.ktx2_uastc_full_bytes,
        tkul = tot.ktx2_uastc_l0_bytes,
        tuem = f3(tot.uastc_encode_ms),
        tutf = f3(tot.uastc_transcode_full_ms),
        tutl = f3(tot.uastc_transcode_l0_ms),
        tumax = tot.uastc_rgb_max_masked,
        tualpha = tot.uastc_alpha_max,
        tep = tot.etc1s_present,
        tef = tot.etc1s_full_bytes,
        tel = tot.etc1s_l0_bytes,
        teem = f3(tot.etc1s_encode_ms),
        tetf = f3(tot.etc1s_transcode_full_ms),
        tetl = f3(tot.etc1s_transcode_l0_ms),
        temax = tot.etc1s_rgb_max_masked,
        tealpha = tot.etc1s_alpha_max,
        d1 = det_uastc_double_encode,
        d2 = det_parse_double_read,
        fp = files_parsed,
        pok = parse_ok,
        legs = if with_etc1s { "uastc+etc1s" } else { "uastc" },
        wall = f3(wall_ms),
    );
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out_path, &json).unwrap_or_else(|e| fail(&format!("evidence 落盘失败: {e}")));
    if !parse_ok {
        fail("KTX2-1 消费证明 parse_crosscheck 判红");
    }
    if !det_uastc_double_encode {
        fail("UASTC 双编码位级漂移(确定性面判红)");
    }
    println!(
        "[{TAG}] PASS textures={count} wall={wall}ms dds_file={tdf}B uastc_full={tkuf}B etc1s_full={tef}B",
        wall = f3(wall_ms),
        tdf = tot_dds_file,
        tkuf = tot.ktx2_uastc_full_bytes,
        tef = tot.etc1s_full_bytes,
    );
}
