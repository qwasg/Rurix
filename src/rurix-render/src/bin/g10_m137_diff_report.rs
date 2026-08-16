//! G10.4a M137 逐像素 diff 报告器（spec/visual_comparison.md RXS-0388；门
//! `g10.p0.m137.pixel_diff_report`）。
//!
//! ## 判据面（G10_ACCEPTANCE_MAP §1 M137 行逐字 + RFC-0026 §4.4）
//!
//! 读两帧 EXR（image-io RXS-0385 strict 解码；同分辨率/同域/RGB 形态
//! fail-closed）→ 误差缓冲（G10.4 门内供给口径 = 逐像素 RGB 通道最大绝对
//! 差 `e = max(|Ra−Rb|,|Ga−Gb|,|Ba−Bb|)` 钳制 `[0,1]`，登记非 schema 语义
//! 本体；G10.5 起 FLIP 域误差图直接取 `error_map_output`）→ 同一误差缓冲
//! 的三面确定性投影：
//!
//! 1. **机器 canonical 面**：逐像素误差 EXR——float32 单通道 Y、NONE 无损、
//!    域随输入帧（`rurix:derivation="derived:diff-report-v1"` 加性登记）；
//! 2. **人读面**：灰度热区图——`e → [e,e,e]`（色彩映射闭集 v1 `{"gray"}`）
//!    经 RXS-0116 确定量化（clamp + 就近取整）落 8-bit 灰度 PPM P6；
//! 3. **逐区域统计**：固定网格 16×16（`region_grid={nx:16,ny:16}`），每区域
//!    字段闭集 `{x,y,w,h,pixel_count,err_max,err_mean,err_p95,
//!    over_threshold_count}`；p95 = nearest-rank（第 ceil(0.95·N) 个，
//!    1-based，禁插值）；末行/末列取实际剩余像素（pixel_count=w·h 对账）。
//!
//! evidence JSON 字段闭集（RXS-0388 L4；空场景行即 RED——scene_id /
//! camera_id 空串即 fail-closed）；thresholds 以 provisional 形态登记
//! （`source="provisional_pending_m138"`，M138 正式入 g10_budget 后翻转）。
//!
//! **G10.5b H1 修订**：evidence `domain` 字段由硬编码 `"scene-linear-hdr"`
//! 改为自输入帧元数据派生（`md_a.domain.as_str()`；双帧域不一致本已
//! fail-closed）——LDR 帧对消费面（M139 A/B 臂）报告域标签与输入域
//! 互证（RXS-0386 L1），G10.5a 预演 H1 标注兑现；误差数值口径 0-byte
//! 不变。
//!
//! ## 用法
//!
//! ```text
//! g10_m137_diff_report --synthetic-pair --out-dir <dir> --evidence <report.json> \
//!     --scene-id <id> --camera-id <id> --frame-index <n> --threshold <f32>
//! g10_m137_diff_report --frame-a <a.exr> --frame-b <b.exr> [--write-frames] ...
//! ```

#![forbid(unsafe_code)]

use image_io::exr::{
    ChromaticitiesOrigin, ExrBitDepth, ExrChannelLayout, ExrDerivation, ExrDomain, ExrImage,
    ExrMetadata, ExrSourceEnd, ExrTransfer, decode_exr, encode_exr,
};
use image_io::{ImageBuffer, ImageFormat, Rgb, encode};
use std::path::{Path, PathBuf};

const TAG: &str = "G10_M137_DR";
const GRID_NX: u32 = 16;
const GRID_NY: u32 = 16;
const SYN_W: u32 = 100; // 非 16 整除（末列剩余 4）——边缘规则实测面
const SYN_H: u32 = 70; // 非 16 整除（末行剩余 6）

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

/// 帧像素内容 digest（跨实现互证面：与 ci/g10_exr_lib.py 同字面——
/// `"G10EXRD-1\0" ‖ w u32le ‖ h u32le ‖ channels u8 ‖ f32 LE 像素字节`）。
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

/// 探针/报告描述符 digest（G10.4 探针帧 capture_params_digest 与
/// determinism_contract_digest 登记面，不冒充 M130 链）。
fn probe_params_digest(tag: &str, width: u32, height: u32) -> String {
    let mut payload = b"G10M137P-1\0".to_vec();
    payload.extend_from_slice(&width.to_le_bytes());
    payload.extend_from_slice(&height.to_le_bytes());
    payload.extend_from_slice(tag.as_bytes());
    format!("sha256:{}", sha256_hex(&payload))
}

/// 口径 digest（metric_caliber：门内供给口径 + 网格 + p95 口径版本互证）。
fn metric_caliber_digest() -> String {
    let payload = b"G10DIFFCAL-1\0err:maxchan-abs-clamp01\0grid:16x16\0p95:nearest-rank";
    format!("sha256:{}", sha256_hex(payload))
}

/// 闭式合成帧对（HDR 臂探针：A = 渐变 + 高亮区；B = A 施加位移/增益/噪声/
/// 截断四类扰动，误差缓冲非平凡且分布多峰）。
fn synthetic_pair(width: u32, height: u32) -> (Vec<f32>, Vec<f32>) {
    let mut a = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            a.push(0.02 * x as f32 + 0.5);
            a.push(0.03 * y as f32 + 0.25);
            a.push(0.001 * (x * y) as f32 + 0.1);
        }
    }
    let mut b = a.clone();
    // xorshift32 确定性噪声（固定 seed，零随机量）。
    let mut state: u32 = 0x9e37_79b9;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 3) as usize;
            // 扰动 1：右半区增益漂移。
            if x >= width / 2 {
                b[i] *= 1.05;
                b[i + 1] *= 0.97;
            }
            // 扰动 2：周期性条带加性偏移。
            if (x / 4) % 2 == 1 {
                b[i + 2] += 0.07;
            }
            // 扰动 3：稀疏噪声点。
            if next() % 977 == 0 {
                b[i] += 0.3;
            }
            // 扰动 4：顶部高亮截断带。
            if y < 6 {
                b[i] = b[i].min(0.55);
                b[i + 1] = b[i + 1].min(0.55);
                b[i + 2] = b[i + 2].min(0.55);
            }
        }
    }
    (a, b)
}

/// HDR 臂捕获元数据（合成帧对共用）。
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

/// 误差 EXR 元数据（域随输入帧；派生链 `derived:diff-report-v1` 加性登记）。
fn error_map_metadata(input: &ExrMetadata) -> ExrMetadata {
    ExrMetadata {
        schema_version: "1".to_owned(),
        domain: input.domain,
        transfer: input.transfer,
        bit_depth: ExrBitDepth::Float32,
        source_end: ExrSourceEnd::Rurix,
        view_transform: input.view_transform,
        capture_params_digest: input.capture_params_digest.clone(),
        derivation: ExrDerivation::DerivedDiffReportV1,
        source_frame_digest: None, // 双源 digest 在 evidence artifacts 闭集登记
        chromaticities_origin: Some(ChromaticitiesOrigin::Writer),
    }
}

/// 误差缓冲（G10.4 门内供给口径：通道最大绝对差钳制 [0,1]）。
fn error_buffer(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.chunks_exact(3)
        .zip(b.chunks_exact(3))
        .map(|(pa, pb)| {
            let d = (pa[0] - pb[0])
                .abs()
                .max((pa[1] - pb[1]).abs())
                .max((pa[2] - pb[2]).abs());
            d.clamp(0.0, 1.0)
        })
        .collect()
}

/// nearest-rank p95（RXS-0388 L2 冻结口径：第 ceil(0.95·N) 个，1-based；
/// ceil(0.95·N) < 1 时取 1；禁插值）。
fn nearest_rank_p95(sorted: &[f32]) -> f32 {
    let n = sorted.len();
    debug_assert!(n > 0);
    let rank = ((95u64 * n as u64) as f64 / 100.0).ceil().max(1.0) as usize;
    sorted[rank.min(n) - 1]
}

struct RegionStat {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    pixel_count: u64,
    err_max: f32,
    err_mean: f64,
    err_p95: f32,
    over_threshold_count: u64,
}

/// 区域统计（固定 16×16 区域网格：常规格 = floor(W/16)×floor(H/16)，末行/
/// 末列区域 w/h 取实际剩余像素——网格恒为 16×16 区域（W≥16∧H≥16 时）；
/// pixel_count=w·h 对账，RXS-0388 L2 边缘规则）。
fn region_stats(err: &[f32], width: u32, height: u32, threshold: f32) -> Vec<RegionStat> {
    let mut out = Vec::new();
    let cell_w = (width / GRID_NX).max(1);
    let cell_h = (height / GRID_NY).max(1);
    for gy in 0..GRID_NY {
        for gx in 0..GRID_NX {
            let x = gx * cell_w;
            let y = gy * cell_h;
            if x >= width || y >= height {
                continue;
            }
            // 末列/末行区域吸收全部剩余像素（边缘规则）。
            let w = if gx + 1 == GRID_NX { width - x } else { cell_w };
            let h = if gy + 1 == GRID_NY {
                height - y
            } else {
                cell_h
            };
            let mut vals: Vec<f32> = Vec::with_capacity((w * h) as usize);
            for yy in y..y + h {
                for xx in x..x + w {
                    vals.push(err[(yy * width + xx) as usize]);
                }
            }
            let pixel_count = vals.len() as u64;
            debug_assert_eq!(pixel_count, w as u64 * h as u64);
            let err_max = vals.iter().copied().fold(0.0f32, f32::max);
            let err_mean = vals.iter().map(|v| *v as f64).sum::<f64>() / pixel_count as f64;
            vals.sort_by(|a, b| a.total_cmp(b));
            let err_p95 = nearest_rank_p95(&vals);
            let over = vals.iter().filter(|v| **v > threshold).count() as u64;
            out.push(RegionStat {
                x,
                y,
                w,
                h,
                pixel_count,
                err_max,
                err_mean,
                err_p95,
                over_threshold_count: over,
            });
        }
    }
    out
}

struct Args {
    frame_a: Option<PathBuf>,
    frame_b: Option<PathBuf>,
    synthetic_pair: bool,
    out_dir: PathBuf,
    evidence: PathBuf,
    scene_id: String,
    camera_id: String,
    frame_index: u32,
    threshold: f32,
}

fn parse_args() -> Args {
    let root = workspace_root();
    let mut out = Args {
        frame_a: None,
        frame_b: None,
        synthetic_pair: false,
        out_dir: root.join(".tmp/g104_gates/m137"),
        evidence: root.join(".tmp/g104_gates/m137/report.json"),
        scene_id: String::new(),
        camera_id: String::new(),
        frame_index: 0,
        threshold: 0.0,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--frame-a" => out.frame_a = Some(PathBuf::from(take(&mut i))),
            "--frame-b" => out.frame_b = Some(PathBuf::from(take(&mut i))),
            "--synthetic-pair" => out.synthetic_pair = true,
            "--out-dir" => out.out_dir = PathBuf::from(take(&mut i)),
            "--evidence" => out.evidence = PathBuf::from(take(&mut i)),
            "--scene-id" => out.scene_id = take(&mut i),
            "--camera-id" => out.camera_id = take(&mut i),
            "--frame-index" => {
                out.frame_index = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--frame-index 须 u32"))
            }
            "--threshold" => {
                out.threshold = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--threshold 须 f32"))
            }
            other => fail(&format!("未知参数: {other}")),
        }
        i += 1;
    }
    out
}

fn load_frame(path: &Path) -> Result<(u32, u32, Vec<f32>, ExrMetadata), String> {
    let blob = std::fs::read(path).map_err(|e| format!("帧不可读 {}: {e}", path.display()))?;
    let dec = decode_exr(&blob, ExrSourceEnd::Rurix).map_err(|e| format!("帧解码失败: {e}"))?;
    if dec.layout != ExrChannelLayout::Rgb {
        return Err("diff 输入帧须 RGB 形态".to_owned());
    }
    let md = dec
        .metadata
        .ok_or_else(|| "rurix 帧元数据缺失".to_owned())?;
    Ok((dec.width, dec.height, dec.pixels, md))
}

fn f32j(v: f32) -> String {
    // JSON 数值字面 = Rust Display 最短十进制 round-trip（解析回同一位模式，
    // 门侧跨实现精确比对不依赖容差）。
    format!("{v}")
}

fn f64j(v: f64) -> String {
    format!("{v}")
}

fn main() {
    let args = parse_args();
    // 空场景行即 RED：scene 三元组 fail-closed。
    if args.scene_id.trim().is_empty() {
        fail("scene_id 空串（空场景行即 RED）");
    }
    if args.camera_id.trim().is_empty() {
        fail("camera_id 空串（空场景行即 RED）");
    }
    if !(args.threshold.is_finite() && args.threshold >= 0.0) {
        fail("threshold 非法（须有限非负）");
    }

    let (width, height, pixels_a, pixels_b, md_a) = if args.synthetic_pair {
        let (a, b) = synthetic_pair(SYN_W, SYN_H);
        (
            SYN_W,
            SYN_H,
            a,
            b,
            hdr_capture_metadata(SYN_W, SYN_H, "synthetic-pair-v1"),
        )
    } else {
        let (pa, pb) = match (&args.frame_a, &args.frame_b) {
            (Some(a), Some(b)) => (a.clone(), b.clone()),
            _ => fail("须 --synthetic-pair 或 --frame-a/--frame-b 双给"),
        };
        let (wa, ha, a, mda) = load_frame(&pa).unwrap_or_else(|e| fail(&e));
        let (wb, hb, b, mdb) = load_frame(&pb).unwrap_or_else(|e| fail(&e));
        if wa != wb || ha != hb {
            fail(&format!("帧分辨率不一致: {wa}×{ha} ≠ {wb}×{hb}"));
        }
        if mda.domain != mdb.domain || mda.transfer != mdb.transfer {
            fail("帧域/transfer 不一致（域标签错配 fail-closed）");
        }
        (wa, ha, a, b, mda)
    };

    // 帧落盘（synthetic-pair 模式：A/B 帧经 M134 管线同一编码面落 EXR，
    // 供门侧独立解码复核 artifacts digest）。
    let _ = std::fs::create_dir_all(&args.out_dir);
    let img_a = ExrImage::new(
        width,
        height,
        ExrChannelLayout::Rgb,
        pixels_a.clone(),
        md_a.clone(),
    )
    .unwrap_or_else(|e| fail(&format!("帧 A 构造失败: {e}")));
    let img_b = ExrImage::new(
        width,
        height,
        ExrChannelLayout::Rgb,
        pixels_b.clone(),
        md_a.clone(),
    )
    .unwrap_or_else(|e| fail(&format!("帧 B 构造失败: {e}")));
    let bytes_a = encode_exr(&img_a).unwrap_or_else(|e| fail(&format!("帧 A 编码失败: {e}")));
    let bytes_b = encode_exr(&img_b).unwrap_or_else(|e| fail(&format!("帧 B 编码失败: {e}")));
    let path_a = args.out_dir.join("frame_a.exr");
    let path_b = args.out_dir.join("frame_b.exr");
    if std::fs::write(&path_a, &bytes_a).is_err() || std::fs::write(&path_b, &bytes_b).is_err() {
        fail("A/B 帧落盘失败");
    }

    // 误差缓冲 → 三面投影。
    let err = error_buffer(&pixels_a, &pixels_b);
    let err_img = ExrImage::new(
        width,
        height,
        ExrChannelLayout::Y,
        err.clone(),
        error_map_metadata(&md_a),
    )
    .unwrap_or_else(|e| fail(&format!("误差帧构造失败: {e}")));
    let err_bytes = encode_exr(&err_img).unwrap_or_else(|e| fail(&format!("误差帧编码失败: {e}")));
    let err_path = args.out_dir.join("error_map.exr");
    if std::fs::write(&err_path, &err_bytes).is_err() {
        fail("误差 EXR 落盘失败");
    }
    // 热区图（gray 映射 + RXS-0116 量化 PPM P6）。
    let mut heat = ImageBuffer::new(width, height, Rgb::new(0.0, 0.0, 0.0));
    for y in 0..height {
        for x in 0..width {
            let e = err[(y * width + x) as usize];
            heat.set(x, y, Rgb::new(e, e, e));
        }
    }
    let heat_bytes =
        encode(&heat, ImageFormat::Ppm).unwrap_or_else(|e| fail(&format!("热区图编码失败: {e}")));
    let heat_path = args.out_dir.join("heatmap.ppm");
    if std::fs::write(&heat_path, &heat_bytes).is_err() {
        fail("热区图落盘失败");
    }

    // 区域统计 + 全图标量。
    let regions = region_stats(&err, width, height, args.threshold);
    let mut sorted = err.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let err_max = sorted.last().copied().unwrap_or(0.0);
    let err_mean = err.iter().map(|v| *v as f64).sum::<f64>() / err.len() as f64;
    let err_p95 = nearest_rank_p95(&sorted);
    let over_total = err.iter().filter(|v| **v > args.threshold).count() as u64;
    let over_ratio = over_total as f64 / err.len() as f64;

    // digest 闭集（artifacts 四 digest + end_pair 双帧 digest）。
    let frame_a_digest = frame_content_digest(width, height, 3, &pixels_a);
    let frame_b_digest = frame_content_digest(width, height, 3, &pixels_b);
    let error_map_digest = frame_content_digest(width, height, 1, &err);
    let heatmap_digest = format!("sha256:{}", sha256_hex(&heat_bytes));
    let error_exr_file_digest = format!("sha256:{}", sha256_hex(&err_bytes));

    // thresholds provisional（G10.4a：identity-pair 噪声底实测由门侧供给，
    // M138 正式入 g10_budget 后翻转 source）。
    let threshold_source_digest = format!(
        "sha256:{}",
        sha256_hex(format!("G10THRESH-1\0{}\0provisional_pending_m138", args.threshold).as_bytes())
    );

    // evidence JSON（RXS-0388 L4 字段闭集）。
    let mut j = String::from("{\n");
    j.push_str("  \"schema_version\": 1,\n");
    j.push_str(&format!(
        "  \"scene_id\": \"{}\",\n",
        json_escape(&args.scene_id)
    ));
    j.push_str(&format!(
        "  \"camera_id\": \"{}\",\n",
        json_escape(&args.camera_id)
    ));
    j.push_str(&format!("  \"frame_index\": {},\n", args.frame_index));
    j.push_str("  \"end_pair\": {\n");
    j.push_str(&format!(
        "    \"frame_a\": {{\"source_end\": \"rurix\", \"frame_id\": \"{}\", \"digest\": \"{}\"}},\n",
        json_escape(&path_a.file_name().unwrap().to_string_lossy()),
        frame_a_digest
    ));
    j.push_str(&format!(
        "    \"frame_b\": {{\"source_end\": \"rurix\", \"frame_id\": \"{}\", \"digest\": \"{}\"}}\n",
        json_escape(&path_b.file_name().unwrap().to_string_lossy()),
        frame_b_digest
    ));
    j.push_str("  },\n");
    j.push_str(&format!("  \"domain\": \"{}\",\n", md_a.domain.as_str()));
    j.push_str(&format!(
        "  \"metric_caliber\": \"{}\",\n",
        metric_caliber_digest()
    ));
    j.push_str("  \"thresholds\": {\n");
    j.push_str(&format!("    \"value\": {},\n", f32j(args.threshold)));
    j.push_str("    \"source\": \"provisional_pending_m138\",\n");
    j.push_str(&format!(
        "    \"source_digest\": \"{threshold_source_digest}\"\n"
    ));
    j.push_str("  },\n");
    j.push_str(&format!(
        "  \"region_grid\": {{\"nx\": {GRID_NX}, \"ny\": {GRID_NY}}},\n"
    ));
    j.push_str("  \"regions\": [\n");
    for (i, r) in regions.iter().enumerate() {
        let comma = if i + 1 == regions.len() { "" } else { "," };
        j.push_str(&format!(
            "    {{\"x\": {}, \"y\": {}, \"w\": {}, \"h\": {}, \"pixel_count\": {}, \"err_max\": {}, \"err_mean\": {}, \"err_p95\": {}, \"over_threshold_count\": {}}}{comma}\n",
            r.x, r.y, r.w, r.h, r.pixel_count, f32j(r.err_max), f64j(r.err_mean), f32j(r.err_p95), r.over_threshold_count
        ));
    }
    j.push_str("  ],\n");
    j.push_str("  \"scalars\": {\n");
    j.push_str("    \"flip\": null,\n");
    j.push_str(&format!("    \"err_max\": {},\n", f32j(err_max)));
    j.push_str(&format!("    \"err_mean\": {},\n", f64j(err_mean)));
    j.push_str(&format!("    \"err_p95\": {},\n", f32j(err_p95)));
    j.push_str(&format!(
        "    \"over_threshold_pixel_count\": {over_total},\n"
    ));
    j.push_str(&format!(
        "    \"over_threshold_ratio\": {}\n",
        f64j(over_ratio)
    ));
    j.push_str("  },\n");
    j.push_str("  \"artifacts\": {\n");
    j.push_str(&format!("    \"frame_a_digest\": \"{frame_a_digest}\",\n"));
    j.push_str(&format!("    \"frame_b_digest\": \"{frame_b_digest}\",\n"));
    j.push_str(&format!(
        "    \"error_map_digest\": \"{error_map_digest}\",\n"
    ));
    j.push_str(&format!("    \"heatmap_digest\": \"{heatmap_digest}\"\n"));
    j.push_str("  },\n");
    j.push_str(&format!(
        "  \"determinism_contract_digest\": \"{}\",\n",
        probe_params_digest("synthetic-pair-v1", width, height)
    ));
    j.push_str("  \"provenance\": {\n");
    j.push_str(&format!("    \"generated_by\": \"{TAG}\",\n"));
    j.push_str(&format!("    \"timestamp\": \"{}\",\n", utc_now()));
    j.push_str(&format!(
        "    \"error_exr_file_digest\": \"{error_exr_file_digest}\",\n"
    ));
    j.push_str("    \"error_supply\": \"maxchan-abs-clamp01 (G10.4 gate-supplied; FLIP error_map 归 G10.5)\",\n");
    j.push_str("    \"threshold_provenance\": \"identity-pair noise floor measured by gate; M138 正式入 g10_budget 后翻转 source\"\n");
    j.push_str("  }\n");
    j.push_str("}\n");
    if let Some(parent) = args.evidence.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&args.evidence, &j).is_err() {
        fail("report 落盘失败");
    }
    println!(
        "{TAG}: PASS regions={} err_max={:.6} err_p95={:.6} over={} heat={} err_exr={}",
        regions.len(),
        err_max,
        err_p95,
        over_total,
        &heatmap_digest[..20],
        &error_map_digest[..20]
    );
    eprintln!("[{TAG}] report → {}", args.evidence.display());
}
