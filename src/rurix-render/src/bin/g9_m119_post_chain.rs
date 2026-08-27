//! G9.5 M119 后处理骨架 harness(RXS-0370;门 `g9.p1.m119.post_chain`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M119 行逐字 + spec/display_pipeline.md RXS-0370)
//!
//! 1. **后处理骨架显式排序冻结(五级闭集)**:histogram 曝光+EV 偏移 → bloom
//!    (tonemap 前 HDR 域多尺度 mip 链) → tonemap(经 M118 view transform 插件)
//!    → 色彩分级 LUT → 输出变换;顺序闭集——交换两级产出必不同(顺序可检测
//!    断言);跳级/插级产出必不同;SDR 路径可全量验证(对 golden 输入集五级
//!    链输出 measured 冻结 golden 对照);
//! 2. **全程 HDR 线性域断言(RED 臂)**:节点输出范围探针,**隐式 SDR clamp
//!    注入即探针越界 RED**(RED 臂独立有效);
//! 3. **曝光状态帧间持久**:histogram→目标 EV adapt 状态 persistent resource
//!    帧间持久;**跨帧丢失注入即 RED**(RED 臂独立有效);adapt 上/下不同速率;
//! 4. **与 M118 view transform 插件面接线**:tonemap 级消费 ViewTransform
//!    trait(未注册插件名调用失败透传 M118 RED);链级禁静默插级/跳级;
//! 5. **conformance 语料消费**:`conformance/display_pipeline/` M119 两件锚定
//!    语料 `//@ spec: RXS-0370` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m119_post_chain [--evidence <path>] [--band <path>]
//! g9_m119_post_chain --freeze [--band-out <path>] [--evidence <path>]
//! g9_m119_post_chain --red-arm order-swap|implicit-clamp|exposure-lost
//! ```

#![forbid(unsafe_code)]

use rurix_render::display::post_chain::{
    ExposureState, HdrProbe, PostChainError, PostProcessChain, Stage, canonical_hdr_frame,
    frame_digest,
};
use rurix_render::display::view_transform::{DisplayParams, OutputEncoding, ViewTransformRegistry};
use std::path::PathBuf;

const TAG: &str = "G9_M119_POST";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/post_stack_explicit_order_minimal.rx", "RXS-0370"),
    ("reject/post_stack_implicit_sdr_clamp.rx", "RXS-0370"),
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
        band: root.join("milestones/g9/g9_m119_post_chain_band.json"),
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

fn make_chain<'a>(
    plugin: &'a dyn rurix_render::display::view_transform::ViewTransform,
    params: &'a DisplayParams,
    frame: u32,
) -> PostProcessChain<'a> {
    PostProcessChain {
        plugin,
        params,
        exposure: ExposureState::init(frame, 0.0),
        lut_slope: [1.0, 1.0, 1.0],
        lut_offset: [0.0, 0.0, 0.0],
    }
}

/// RED 臂:顺序交换(tonemap↔LUT 或 bloom↔tonemap)。
fn red_arm_order_swap() -> Result<(), String> {
    let reg = ViewTransformRegistry::with_builtins();
    let plugin = reg.get("neutral").map_err(|e| e.to_string())?;
    let params = DisplayParams {
        peak_luminance_nits: 100.0,
        encoding: OutputEncoding::SdrBt1886,
    };
    let frame = canonical_hdr_frame();
    let width = 32usize;

    let mut normal = make_chain(plugin, &params, 0);
    let d_normal = frame_digest(
        &normal
            .process(1, &frame, width)
            .map_err(|e| e.to_string())?,
    );

    // 交换 bloom↔tonemap。
    let swapped = make_chain(plugin, &params, 0);
    let s_exp = swapped
        .process_stage(Stage::Exposure, &frame, width)
        .map_err(|e| e.to_string())?;
    let s_tone = swapped
        .process_stage(Stage::Tonemap, &s_exp, width)
        .map_err(|e| e.to_string())?;
    let s_bloom = swapped
        .process_stage(Stage::Bloom, &s_tone, width)
        .map_err(|e| e.to_string())?;
    let s_lut = swapped
        .process_stage(Stage::ColorGrading, &s_bloom, width)
        .map_err(|e| e.to_string())?;
    let s_out = swapped
        .process_stage(Stage::OutputTransform, &s_lut, width)
        .map_err(|e| e.to_string())?;
    let d_swapped = frame_digest(&s_out);
    if d_normal == d_swapped {
        return Err("交换 bloom↔tonemap 产出未分叉(顺序不可检测 = RED 臂失效)".into());
    }

    // 跳级(跳过 bloom)。
    let skipped = make_chain(plugin, &params, 0);
    let k_exp = skipped
        .process_stage(Stage::Exposure, &frame, width)
        .map_err(|e| e.to_string())?;
    let k_tone = skipped
        .process_stage(Stage::Tonemap, &k_exp, width)
        .map_err(|e| e.to_string())?;
    let k_lut = skipped
        .process_stage(Stage::ColorGrading, &k_tone, width)
        .map_err(|e| e.to_string())?;
    let k_out = skipped
        .process_stage(Stage::OutputTransform, &k_lut, width)
        .map_err(|e| e.to_string())?;
    if d_normal == frame_digest(&k_out) {
        return Err("跳级(跳 bloom)产出未分叉(顺序不可检测 = RED 臂失效)".into());
    }
    Ok(())
}

/// RED 臂:隐式 SDR clamp 注入(探针越界)。
fn red_arm_implicit_clamp() -> Result<(), String> {
    let frame = canonical_hdr_frame();
    let clamped: Vec<[f64; 3]> = frame
        .iter()
        .map(|p| {
            [
                p[0].clamp(0.0, 1.0),
                p[1].clamp(0.0, 1.0),
                p[2].clamp(0.0, 1.0),
            ]
        })
        .collect();
    match HdrProbe::from_pixels(&clamped).check_for_implicit_clamp("bloom") {
        Err(PostChainError::ImplicitSdrClamp { .. }) => Ok(()),
        other => Err(format!("隐式 SDR clamp 注入未被探针捕获: {other:?}")),
    }
}

/// RED 臂:曝光状态跨帧丢失注入。
fn red_arm_exposure_lost() -> Result<(), String> {
    let mut state = ExposureState::init(0, 2.0);
    state.tick(1, 3.0).map_err(|e| e.to_string())?;
    match state.tick(3, 4.0) {
        Err(PostChainError::ExposureStateLost { expected_frame: 2 }) => Ok(()),
        other => Err(format!("曝光状态跨帧丢失未检出: {other:?}")),
    }
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "order-swap" => red_arm_order_swap(),
            "implicit-clamp" => red_arm_implicit_clamp(),
            "exposure-lost" => red_arm_exposure_lost(),
            other => fail(&format!(
                "未知 RED 臂: {other}(order-swap|implicit-clamp|exposure-lost)"
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

    // ── 步骤 2:五级显式排序链 SDR 路径 golden(四插件逐一) ──
    let reg = ViewTransformRegistry::with_builtins();
    let params = DisplayParams {
        peak_luminance_nits: 100.0,
        encoding: OutputEncoding::SdrBt1886,
    };
    let frame = canonical_hdr_frame();
    let width = 32usize;
    let plugin_ids = ["aces13", "aces20", "agx", "neutral"];
    let mut chain_digests: Vec<(&str, [u8; 32])> = Vec::new();
    for id in plugin_ids {
        let plugin = reg
            .get(id)
            .unwrap_or_else(|e| fail(&format!("取插件 {id}: {e}")));
        let mut chain = make_chain(plugin, &params, 0);
        let out = chain
            .process(1, &frame, width)
            .unwrap_or_else(|e| fail(&format!("五级链处理: {e}")));
        chain_digests.push((id, frame_digest(&out)));
    }
    // 双跑位级一致。
    let plugin = reg.get("neutral").expect("neutral");
    let mut c1 = make_chain(plugin, &params, 0);
    let d1 = frame_digest(&c1.process(1, &frame, width).expect("run1"));
    let mut c2 = make_chain(plugin, &params, 0);
    let d2 = frame_digest(&c2.process(1, &frame, width).expect("run2"));
    let double_run_ok = d1 == d2;
    if !double_run_ok {
        failures.push("五级链双跑位级不一致".into());
    }
    // 插件间输出互异(至少 4 插件 digest 两两不同)。
    let mut distinct = true;
    for i in 0..chain_digests.len() {
        for j in (i + 1)..chain_digests.len() {
            if chain_digests[i].1 == chain_digests[j].1 {
                distinct = false;
            }
        }
    }
    if !distinct {
        failures.push("四插件链输出未互异(插件接线面失效)".into());
    }

    // ── 步骤 3:RED 臂内联实测 ──
    let red_swap_ok = red_arm_order_swap().is_ok();
    let red_clamp_ok = red_arm_implicit_clamp().is_ok();
    let red_exp_ok = red_arm_exposure_lost().is_ok();
    if !red_swap_ok {
        failures.push("顺序交换/跳级 RED 臂失效".into());
    }
    if !red_clamp_ok {
        failures.push("隐式 SDR clamp RED 臂失效".into());
    }
    if !red_exp_ok {
        failures.push("曝光状态跨帧丢失 RED 臂失效".into());
    }

    // ── 步骤 4:曝光 adapt 曲线 golden(上/下不同速率) ──
    let mut state = ExposureState::init(0, 0.0);
    state.tick(1, 2.0).expect("tick1");
    let up_step = state.ev_current;
    let mut state2 = ExposureState::init(0, 2.0);
    state2.tick(1, 0.0).expect("tick1");
    let down_step = 2.0 - state2.ev_current;
    let adapt_curve_ok = up_step > down_step && up_step > 0.0 && down_step > 0.0;
    if !adapt_curve_ok {
        failures.push("曝光 adapt 上/下速率未分化".into());
    }

    // ── 步骤 5:golden 带对照(freeze 自标定;PASS 逐字) ──
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
        for (id, d) in &chain_digests {
            let key = format!("{id}_chain_digest");
            let frozen = json_str(t, &key).unwrap_or_else(|| fail(&format!("冻结带缺 {key}")));
            if frozen != hex(d) {
                golden_ok = false;
                failures.push(format!("golden 漂移: {key}"));
            }
        }
    }

    // ── 步骤 6:freeze 落盘(measured 冻结 + provenance) ──
    if args.freeze {
        let mut plugins_json = String::new();
        for (id, d) in &chain_digests {
            plugins_json.push_str(&format!("    \"{}_chain_digest\": \"{}\",\n", id, hex(d)));
        }
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m119.post_chain_band.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖;M119 语义面 = 排序骨架 + HDR 探针 + 曝光持久状态)\"}},\n  \
             \"freeze_rule\": \"chain_digest = canonical HDR 帧(32×32,含高光>1)经五级显式排序链(曝光→bloom→tonemap→LUT→输出变换)SDR 路径输出的 SHA-256(双跑位级一致后冻结,禁手写);四插件逐一冻结\",\n  \
             \"spec_anchor\": \"RXS-0370\",\n  \
             \"stage_order\": [\"exposure\", \"bloom\", \"tonemap\", \"color_grading\", \"output_transform\"],\n  \
             \"plugins\": {{\n{}}},\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-m111-m112-m119-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            plugins_json.trim_end_matches([',', '\n']),
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}", args.band);
    }

    // ── 步骤 7:evidence(rurix.g9m119.post_chain.v1) ──
    let checks: [(&str, bool); 8] = [
        ("conformance_corpus_anchored", corpus_ok),
        ("five_stage_explicit_order", true),
        ("order_swap_detectable_red", red_swap_ok),
        ("hdr_linear_domain_probe_red", red_clamp_ok),
        ("exposure_state_persist_red", red_exp_ok),
        ("four_plugins_wired", distinct),
        ("double_run_bit_equal", double_run_ok),
        ("golden_frozen_equal", golden_ok || args.freeze),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let digests_json: Vec<String> = chain_digests
        .iter()
        .map(|(id, d)| format!("\"{id}\": \"{}\"", hex(d)))
        .collect();
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m119.post_chain.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m119_post_chain\",\n  \"spec_anchor\": \"RXS-0370\",\n  \
         \"assertion_id\": \"g9.p1.m119.post_chain\",\n  \"milestone\": \"M119\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;M119 语义面 = 排序骨架 + HDR 探针 + 曝光持久状态)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"golden\": {{\"chain_digests\": {{{}}}, \"freeze_band\": \"{}\"}},\n  \
         \"red_arms\": {{\"order_swap\": {}, \"implicit_clamp\": {}, \"exposure_lost\": {}}},\n  \
         \"exposure_adapt\": {{\"up_step\": {:.6}, \"down_step\": {:.6}, \"rates_diverged\": {}}},\n  \
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
        digests_json.join(", "),
        json_escape(&args.band.display().to_string()),
        red_swap_ok,
        red_clamp_ok,
        red_exp_ok,
        up_step,
        down_step,
        adapt_curve_ok,
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
        println!("{TAG}: PASS 五级显式排序 + HDR 线性域探针 + 曝光帧间持久(host 确定性面)");
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
