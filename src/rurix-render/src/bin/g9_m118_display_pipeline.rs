//! G9.5 M118 显示管线 view transform harness(RXS-0369;门
//! `g9.p0.m118.display_pipeline_view_transform`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §2 M118 行逐字 + spec/display_pipeline.md RXS-0369)
//!
//! 1. **SDR/scRGB/PQ 三交换链路径运行时切换证据齐备**:canonical HDR 帧经
//!    aces13 插件 SDR→scRGB→PQ→SDR 切换,同输入切同路径输出位级一致
//!    (digest 机核);切换序列日志非空;三路径 × 四插件输出 digest 全量记录;
//! 2. **ACES 1.3/2.0/AgX/中性四内置插件逐一对冻结 golden**:golden 输入集
//!    (155 条闭式生成)经四插件 SDR 路径输出 digest 对
//!    `milestones/g9/g9_m118_display_pipeline_golden.json`(host 参考公式逐字
//!    实现 + measured 冻结 + provenance,禁手写);**已知差异记录**(D4 R-D4-5):
//!    ACES 1.3↔2.0 版本间差(max/mean 逐通道)与 AgX↔ACES 2.0 hue-skew
//!    (max/mean 度)实测入带;AgX 对比度补偿参数随 view transform 资产化
//!    (Punchy vs 平直资产 digest 分叉机核);未注册插件名调用 → 拒录 RED;
//! 3. **非 HDR 交换链携带 PQ 输出即 RED**:SDR/scRGB + PQ 编码 → typed Err
//!    (fail-closed);PQ+PQ 合法;合法三组合全 Ok(sabotage 能红证明);
//! 4. **HDR 设备标定层条件未触发登记 SKIP=not-triggered**:能力查询面返回
//!    显式 NotTriggered 结构(evidence 字段可见,**不充绿**——checks 内该面
//!    独立登记且不计入绿色断言),强制消费 → typed Err;标定未触发不反向
//!    否决 SDR 验证面(1/2/3 照常全量验证);
//! 5. **conformance 语料消费**:`conformance/display_pipeline/` M118 两件锚定
//!    语料 `//@ spec: RXS-0369` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m118_display_pipeline [--evidence <path>] [--band <path>]
//! g9_m118_display_pipeline --freeze [--band-out <path>] [--evidence <path>]
//! g9_m118_display_pipeline --red-arm pq-on-non-hdr|unregistered-plugin
//! ```

#![forbid(unsafe_code)]

use rurix_render::display::swapchain::{
    DisplayPipeline, SwapchainPath, query_hdr_capability, require_hdr_calibration,
};
use rurix_render::display::view_transform::{
    DisplayParams, OutputEncoding, ViewTransformRegistry, canonical_hdr_frame, golden_input_set,
    rgb_set_digest,
};
use std::path::PathBuf;

const TAG: &str = "G9_M118_DP";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/view_transform_four_plugins_minimal.rx", "RXS-0369"),
    ("reject/non_hdr_swapchain_pq_output.rx", "RXS-0369"),
];
const PLUGIN_IDS: [&str; 4] = ["aces13", "aces20", "agx", "neutral"];
const PATHS: [SwapchainPath; 3] = [
    SwapchainPath::Sdr,
    SwapchainPath::ScRgb,
    SwapchainPath::PqRec2020,
];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
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

fn json_str<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\": \"");
    let start = text.find(&needle)? + needle.len();
    let end = text[start..].find('"')? + start;
    Some(&text[start..end])
}

/// f64 位级精确 JSON 面({:.17e} 十进制保证 roundtrip;位比较经 to_bits  hex)。
fn f64_hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

struct Args {
    evidence: Option<PathBuf>,
    band: PathBuf,
    freeze: bool,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let root = workspace_root();
    let mut out = Args {
        evidence: None,
        band: root.join("milestones/g9/g9_m118_display_pipeline_golden.json"),
        freeze: false,
        red_arm: None,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i)
                .unwrap_or_else(|| fail(&format!("{} 缺参数", args[*i - 1])))
                .clone()
        };
        match args[i].as_str() {
            "--evidence" => out.evidence = Some(PathBuf::from(take(&mut i))),
            "--band" | "--band-out" => out.band = PathBuf::from(take(&mut i)),
            "--freeze" => out.freeze = true,
            "--red-arm" => out.red_arm = Some(take(&mut i)),
            other => fail(&format!("未知参数: {other}")),
        }
        i += 1;
    }
    out
}

/// 插件 × 路径 digest 行(类型别名,clippy type_complexity 面)。
type PluginDigestRow = (&'static str, [[u8; 32]; 3]);

/// 四插件 × 三路径 digest 矩阵(golden 面 + 路径覆盖证据)。
fn plugin_path_digests(
    registry: &ViewTransformRegistry,
    input: &[[f64; 3]],
) -> Vec<PluginDigestRow> {
    let mut rows = Vec::new();
    for id in PLUGIN_IDS {
        let plugin = registry.get(id).unwrap_or_else(|_| fail("内置插件缺失"));
        let mut digests = [[0u8; 32]; 3];
        for (pi, path) in PATHS.iter().enumerate() {
            let params = DisplayParams {
                peak_luminance_nits: 100.0,
                encoding: path.legal_encoding(),
            };
            let out: Vec<[f64; 3]> = input
                .iter()
                .map(|&px| plugin.transform(px, &params))
                .collect();
            digests[pi] = rgb_set_digest(&out);
        }
        rows.push((plugin.id(), digests));
    }
    rows
}

/// 已知差异实测(D4 R-D4-5;measured,入带):
/// - ACES 1.3 ↔ 2.0:SDR 编码域逐通道 max/mean 绝对差(版本间差异区间);
/// - AgX ↔ ACES 2.0 hue-skew:显示线性域几何色相角差(度,环绕 [-180,180])。
fn measure_known_differences(
    registry: &ViewTransformRegistry,
    input: &[[f64; 3]],
) -> ((f64, f64), (f64, f64)) {
    let a13 = registry.get("aces13").expect("aces13");
    let a20 = registry.get("aces20").expect("aces20");
    let agx = registry.get("agx").expect("agx");
    let sdr = DisplayParams {
        peak_luminance_nits: 100.0,
        encoding: OutputEncoding::SdrBt1886,
    };
    let mut max_abs = 0.0f64;
    let mut sum_abs = 0.0f64;
    let mut n = 0usize;
    let mut max_hue = 0.0f64;
    let mut sum_hue = 0.0f64;
    let mut n_hue = 0usize;
    for &px in input {
        let o13 = a13.transform(px, &sdr);
        let o20 = a20.transform(px, &sdr);
        for c in 0..3 {
            let d = (o13[c] - o20[c]).abs();
            max_abs = max_abs.max(d);
            sum_abs += d;
            n += 1;
        }
        // hue-skew(显示线性域;饱和色相才参与——中性色 hue 未定义)。
        let l_agx = agx.to_display_linear(px);
        let l_a20 = a20.to_display_linear(px);
        let sat = l_a20[0].max(l_a20[1]).max(l_a20[2]) - l_a20[0].min(l_a20[1]).min(l_a20[2]);
        if sat > 0.05 {
            let h1 = rurix_render::display::color::rgb_2_hue(l_agx);
            let h2 = rurix_render::display::color::rgb_2_hue(l_a20);
            if h1.is_finite() && h2.is_finite() {
                let mut dh = (h1 - h2).abs();
                if dh > 180.0 {
                    dh = 360.0 - dh;
                }
                max_hue = max_hue.max(dh);
                sum_hue += dh;
                n_hue += 1;
            }
        }
    }
    (
        (max_abs, sum_abs / n.max(1) as f64),
        (max_hue, sum_hue / n_hue.max(1) as f64),
    )
}

/// RED 臂:非 HDR 交换链携带 PQ 输出(RXS-0369 L3)。
fn red_arm_pq_on_non_hdr() -> Result<(), String> {
    let registry = ViewTransformRegistry::with_builtins();
    let plugin = registry.get("aces13").map_err(|e| e.to_string())?;
    let frame = canonical_hdr_frame();
    let pipe = DisplayPipeline::assemble(SwapchainPath::Sdr, 100.0).map_err(|e| e.to_string())?;
    // 负例两臂:SDR+PQ / scRGB+PQ 必须 typed Err。
    match pipe.present_explicit(
        &frame,
        plugin,
        SwapchainPath::Sdr,
        OutputEncoding::PqSt2084Rec2020,
    ) {
        Err(rurix_render::display::view_transform::DisplayError::PqOutputOnNonHdrSwapchain {
            ..
        }) => {}
        other => return Err(format!("SDR+PQ 未拒(漏检): {}", other.is_ok())),
    }
    match pipe.present_explicit(
        &frame,
        plugin,
        SwapchainPath::ScRgb,
        OutputEncoding::PqSt2084Rec2020,
    ) {
        Err(rurix_render::display::view_transform::DisplayError::PqOutputOnNonHdrSwapchain {
            ..
        }) => {}
        other => return Err(format!("scRGB+PQ 未拒(漏检): {}", other.is_ok())),
    }
    // sabotage 探针(能红证明):合法组合全 Ok。
    for path in PATHS {
        pipe.present_explicit(&frame, plugin, path, path.legal_encoding())
            .map_err(|e| format!("合法组合被误拒: {e}"))?;
    }
    // PQ 路径 + PQ 编码合法。
    pipe.present_explicit(
        &frame,
        plugin,
        SwapchainPath::PqRec2020,
        OutputEncoding::PqSt2084Rec2020,
    )
    .map_err(|e| format!("PQ+PQ 被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:未注册插件名调用拒录(RXS-0369 L2)。
fn red_arm_unregistered_plugin() -> Result<(), String> {
    let registry = ViewTransformRegistry::with_builtins();
    match registry.get("filmic-x") {
        Err(rurix_render::display::view_transform::DisplayError::UnregisteredPlugin(n)) => {
            if n != "filmic-x" {
                return Err("错误名不回显".into());
            }
        }
        other => return Err(format!("未注册名未拒录: {}", other.is_ok())),
    }
    // sabotage:已注册名可取。
    for id in PLUGIN_IDS {
        registry
            .get(id)
            .map_err(|e| format!("内置插件 {id} 取用失败: {e}"))?;
    }
    Ok(())
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "pq-on-non-hdr" => red_arm_pq_on_non_hdr(),
            "unregistered-plugin" => red_arm_unregistered_plugin(),
            other => fail(&format!(
                "未知 RED 臂: {other}(pq-on-non-hdr|unregistered-plugin)"
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

    // ── 步骤 1:conformance 语料锚定核验 ──
    let corpus_dir = root.join("conformance/display_pipeline");
    let mut corpus_ok = true;
    let mut anchors_json: Vec<String> = Vec::new();
    for (rel, expect) in CORPUS_FILES {
        let path = corpus_dir.join(rel);
        let anchor = std::fs::read_to_string(&path).ok().and_then(|t| {
            t.lines()
                .find(|l| l.contains("//@ spec:"))
                .map(|l| l.to_string())
        });
        let ok = anchor
            .as_ref()
            .map(|l| l.contains(&format!("//@ spec: {expect}")))
            .unwrap_or(false);
        if !ok {
            corpus_ok = false;
            failures.push(format!("语料 {rel} 缺 {expect} 锚"));
        }
        anchors_json.push(format!(
            "\"{}\": \"{}\"",
            rel.replace('\\', "/"),
            if ok { expect } else { "MISSING" }
        ));
    }
    if corpus_ok {
        println!("{TAG}: conformance 语料 2 件锚定核验通过");
    }

    // ── 步骤 2:注册表面(四内置并列 + 未注册拒录) ──
    let registry = ViewTransformRegistry::with_builtins();
    let registry_ok =
        registry.registered_names() == PLUGIN_IDS.to_vec() && red_arm_unregistered_plugin().is_ok();
    if !registry_ok {
        failures.push("插件注册表面失效".into());
    }

    // ── 步骤 3:golden 输入集 + 四插件 × 三路径 digest(双跑位级一致) ──
    let input = golden_input_set();
    let input_digest = rgb_set_digest(&input);
    let frame = canonical_hdr_frame();
    let frame_digest = rgb_set_digest(&frame);
    let matrix = plugin_path_digests(&registry, &input);
    let matrix2 = plugin_path_digests(&registry, &input);
    let double_run_ok = matrix.iter().zip(matrix2.iter()).all(|(a, b)| a.1 == b.1);
    if !double_run_ok {
        failures.push("四插件 golden 双跑位级不一致".into());
    }

    // ── 步骤 4:已知差异实测 ──
    let ((aces_max, aces_mean), (hue_max, hue_mean)) = measure_known_differences(&registry, &input);
    let differences_nonempty = aces_max > 0.0 && hue_max > 0.0;
    if !differences_nonempty {
        failures.push("已知差异测量退化(ACES 版本差/hue-skew 全零 = 测量面失效)".into());
    }

    // ── 步骤 5:三路径运行时切换确定性(aces13 canonical 帧) ──
    let mut pipe = DisplayPipeline::assemble(SwapchainPath::Sdr, 100.0).expect("装配");
    let plugin13 = registry.get("aces13").expect("aces13");
    let a1 = pipe.present(&frame, plugin13).expect("present SDR#1");
    pipe.switch_to(SwapchainPath::ScRgb).expect("切 scRGB");
    let b_out = pipe.present(&frame, plugin13).expect("present scRGB");
    pipe.switch_to(SwapchainPath::PqRec2020).expect("切 PQ");
    let c_out = pipe.present(&frame, plugin13).expect("present PQ");
    pipe.switch_to(SwapchainPath::Sdr).expect("切回 SDR");
    let a2 = pipe.present(&frame, plugin13).expect("present SDR#2");
    let switch_deterministic = a1.digest == a2.digest
        && a1.digest != b_out.digest
        && b_out.digest != c_out.digest
        && pipe.switch_log().len() == 3;
    let metadata_nonempty = a1.hdr_metadata.max_cll_nits > 0.0
        && b_out.hdr_metadata.max_cll_nits > 0.0
        && c_out.hdr_metadata.max_cll_nits > 0.0
        && c_out.hdr_metadata.max_fall_nits > 0.0;
    if !switch_deterministic || !metadata_nonempty {
        failures.push(format!(
            "三路径切换面: deterministic={switch_deterministic} metadata={metadata_nonempty}"
        ));
    }

    // ── 步骤 6:PQ RED 臂(主流程内联实测) ──
    let pq_arm_ok = red_arm_pq_on_non_hdr().is_ok();
    if !pq_arm_ok {
        failures.push("非 HDR 交换链携带 PQ 输出 RED 臂失效".into());
    }

    // ── 步骤 7:HDR 设备标定层登记(not-triggered 不充绿) ──
    let hdr_report = query_hdr_capability();
    let hdr_not_triggered = matches!(
        hdr_report.calibration,
        rurix_render::display::swapchain::HdrCalibrationStatus::NotTriggered { .. }
    ) && !hdr_report.display_hdr_capable
        && require_hdr_calibration(&hdr_report).is_err();
    if !hdr_not_triggered {
        failures.push("HDR 标定层 NotTriggered 登记面失效".into());
    }

    // ── 步骤 8:golden 带对照(freeze 自标定;PASS 逐字) ──
    let band_text = match std::fs::read_to_string(&args.band) {
        Ok(t) => Some(t),
        Err(_) if args.freeze => None,
        Err(_) => fail(&format!(
            "冻结带 {:?} 不存在——先跑 `--freeze` 产 measured 冻结(禁手写 golden)",
            args.band
        )),
    };
    let mut golden_ok = true;
    if !args.freeze {
        let t = band_text.as_deref().expect("PASS 模式冻结带必读");
        for (id, digests) in &matrix {
            for (k, tag) in ["sdr", "scrgb", "pq"].iter().enumerate() {
                let key = format!("{id}_{tag}_digest");
                let frozen = json_str(t, &key).unwrap_or_else(|| fail(&format!("冻结带缺 {key}")));
                if frozen != hex(&digests[k]) {
                    golden_ok = false;
                    failures.push(format!("golden 漂移: {key}"));
                }
            }
        }
        // 已知差异带内记录对照(位级)。
        for (key, val) in [
            ("aces13_vs_aces20_max_abs", aces_max),
            ("aces13_vs_aces20_mean_abs", aces_mean),
            ("agx_vs_aces20_hue_skew_max_deg", hue_max),
            ("agx_vs_aces20_hue_skew_mean_deg", hue_mean),
        ] {
            let frozen = json_str(t, key).unwrap_or_else(|| fail(&format!("冻结带缺 {key}")));
            if frozen != f64_hex(val) {
                golden_ok = false;
                failures.push(format!("已知差异记录漂移: {key}"));
            }
        }
    }

    // ── 步骤 9:freeze 落盘(measured 冻结 + provenance) ──
    if args.freeze {
        let mut plugins_json = String::new();
        for (id, digests) in &matrix {
            for (k, tag) in ["sdr", "scrgb", "pq"].iter().enumerate() {
                plugins_json.push_str(&format!(
                    "    \"{}_{}_digest\": \"{}\",\n",
                    id,
                    tag,
                    hex(&digests[k])
                ));
            }
        }
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m118.display_pipeline_golden.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖;M118 语义面 = view transform 数学 + 交换链路径状态机;窗口腿 D-130 C++ shim 0-byte)\"}},\n  \
             \"freeze_rule\": \"golden digest = golden 输入集(155 条闭式)经四插件参考公式 host 逐字实现输出的 SHA-256(双跑位级一致后冻结,禁手写);已知差异 = ACES 1.3↔2.0 SDR 编码域逐通道 max/mean 绝对差 + AgX↔ACES 2.0 显示线性域色相角差(度),实测入带(f64 位级 hex)\",\n  \
             \"spec_anchor\": \"RXS-0369\",\n  \
             \"golden_input_digest\": \"{}\",\n  \"golden_input_count\": {},\n  \
             \"canonical_frame_digest\": \"{}\",\n  \
             \"plugin_reference\": {{\"aces13\": \"ACES 1.3 RRT.a1.0.3+ODT.Rec709_100nits_dim.a1.0.3 host 逐字\", \"aces20\": \"ACES 2.0 aces-core Lib.Academy.OutputTransform.a2.v1 host 逐字(preset Rec709-D65_100nit_BT1886)\", \"agx\": \"AgX iolite minimal host 逐字(look=Punchy 资产化)\", \"neutral\": \"Khronos PBR Neutral host 逐字\"}},\n  \
             \"agx_look_asset\": {{\"slope\": [1.0, 1.0, 1.0], \"offset\": [0.0, 0.0, 0.0], \"power\": [1.35, 1.35, 1.35], \"saturation\": 1.4}},\n  \
             \"plugins\": {{\n{}}},\n  \
             \"known_differences\": {{\n    \"aces13_vs_aces20_max_abs\": \"{}\",\n    \"aces13_vs_aces20_mean_abs\": \"{}\",\n    \"aces13_vs_aces20_max_abs_dec\": \"{:.6e}\",\n    \"aces13_vs_aces20_mean_abs_dec\": \"{:.6e}\",\n    \"agx_vs_aces20_hue_skew_max_deg\": \"{}\",\n    \"agx_vs_aces20_hue_skew_mean_deg\": \"{}\",\n    \"agx_vs_aces20_hue_skew_max_dec\": \"{:.6}\",\n    \"agx_vs_aces20_hue_skew_mean_dec\": \"{:.6}\",\n    \"note\": \"AgX/ACES hue-skew 与 ACES 版本间差异为公认差异记录(D4 R-D4-5),不作 bug 返工\"\n  }},\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-m118-m120-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            hex(&input_digest),
            input.len(),
            hex(&frame_digest),
            plugins_json.trim_end_matches([',', '\n']),
            f64_hex(aces_max),
            f64_hex(aces_mean),
            aces_max,
            aces_mean,
            f64_hex(hue_max),
            f64_hex(hue_mean),
            hue_max,
            hue_mean,
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}", args.band);
    }

    // ── 步骤 10:evidence(rurix.g9m118.display_pipeline.v1) ──
    let checks: [(&str, bool); 9] = [
        ("conformance_corpus_anchored", corpus_ok),
        (
            "registry_four_builtins_and_unregistered_rejected",
            registry_ok,
        ),
        ("four_plugins_golden_double_run_bit_equal", double_run_ok),
        ("four_plugins_golden_frozen_equal", golden_ok || args.freeze),
        ("known_differences_recorded", differences_nonempty),
        (
            "three_paths_runtime_switch_deterministic",
            switch_deterministic,
        ),
        ("hdr_metadata_filled_by_output_transform", metadata_nonempty),
        ("pq_on_non_hdr_swapchain_red_arm", pq_arm_ok),
        (
            "hdr_calibration_not_triggered_registered",
            hdr_not_triggered,
        ),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let mut matrix_json = String::new();
    for (id, digests) in &matrix {
        matrix_json.push_str(&format!(
            "      \"{id}\": {{\"sdr\": \"{}\", \"scrgb\": \"{}\", \"pq\": \"{}\"}},\n",
            hex(&digests[0]),
            hex(&digests[1]),
            hex(&digests[2])
        ));
    }
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let switch_log_json: Vec<String> = pipe
        .switch_log()
        .iter()
        .map(|r| {
            format!(
                "{{\"seq\": {}, \"from\": \"{}\", \"to\": \"{}\"}}",
                r.seq,
                r.from.as_str(),
                r.to.as_str()
            )
        })
        .collect();
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m118.display_pipeline.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m118_display_pipeline\",\n  \"spec_anchor\": \"RXS-0369\",\n  \
         \"assertion_id\": \"g9.p0.m118.display_pipeline_view_transform\",\n  \"milestone\": \"M118\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;M118 语义面 = view transform 数学 + 交换链路径状态机,窗口腿 D-130 shim 0-byte)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"golden\": {{\"input_digest\": \"{}\", \"input_count\": {}, \"frame_digest\": \"{}\", \"freeze_band\": \"{}\", \"plugins\": {{\n    {}\n  }}}},\n  \
         \"known_differences\": {{\"aces13_vs_aces20_max_abs\": {:.6e}, \"aces13_vs_aces20_mean_abs\": {:.6e}, \"agx_vs_aces20_hue_skew_max_deg\": {:.6}, \"agx_vs_aces20_hue_skew_mean_deg\": {:.6}, \"counts_as_bug\": false}},\n  \
         \"swapchain\": {{\"paths\": [\"sdr\", \"scrgb\", \"pq_rec2020\"], \"switch_log\": [{}], \"switch_deterministic_bit_equal\": {}}},\n  \
         \"hdr_calibration\": {{\"status\": \"not-triggered\", \"display_hdr_capable\": false, \"query_surface\": \"{}\", \"counts_as_green\": false, \"does_not_veto_sdr_surface\": true, \"require_calibration_fail_closed\": {}}},\n  \
         \"red_arms\": {{\"pq_on_non_hdr_swapchain\": {}, \"unregistered_plugin\": {}}},\n  \
         \"conformance_corpus\": {{\"dir\": \"conformance/display_pipeline\", \"files\": {}, \"anchors\": {{{}}}}},\n  \
         \"checks\": {{{}}},\n  \
         \"commands\": [{}],\n  \
         \"failures\": [{}]\n}}",
        if args.freeze { "freeze" } else { "pass" },
        utc_now(),
        json_escape(&base_commit),
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        hex(&input_digest),
        input.len(),
        hex(&frame_digest),
        json_escape(&args.band.display().to_string()),
        matrix_json.trim_end_matches([',', '\n']),
        aces_max,
        aces_mean,
        hue_max,
        hue_mean,
        switch_log_json.join(", "),
        switch_deterministic,
        json_escape(hdr_report.query_surface),
        require_hdr_calibration(&hdr_report).is_err(),
        pq_arm_ok,
        red_arm_unregistered_plugin().is_ok(),
        CORPUS_FILES.len(),
        anchors_json.join(", "),
        checks_json.join(", "),
        std::env::args()
            .map(|a| format!("\"{}\"", json_escape(&a)))
            .collect::<Vec<_>>()
            .join(", "),
        failures_json.join(", "),
    );
    if let Some(p) = &args.evidence {
        std::fs::write(p, &json).unwrap_or_else(|e| fail(&format!("写 evidence {p:?}: {e}")));
        println!("{TAG}: evidence 已落盘 {p:?}");
    }
    println!("{json}");
    if failures.is_empty() {
        println!(
            "{TAG}: PASS 四插件逐一 golden + 三路径切换位级确定 + PQ-RED 臂 + HDR 标定 not-triggered 登记(host 确定性面)"
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
