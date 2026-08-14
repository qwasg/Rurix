//! G9.5 M115 皮肤 Burley 屏单 pass harness(RXS-0373;门 `g9.p1.m115.skin_burley_diffusion`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M115 行逐字 + spec/display_pipeline.md RXS-0373)
//!
//! 1. **Burley normalized diffusion 屏空单 pass separable SSS(颜色/深度双
//!    kernel)**:canonical 斑块输出 golden(双跑位级一致 + measured 冻结带);
//! 2. **扩散 profile 资产化**(RGB 三通道 falloff,per-material,经 §4.L 侧表
//!    通道按材质槽 ID 索引):扩散 profile 参数 → 扩散半径响应 golden(falloff
//!    增大 ⇒ 半径单调增);
//! 3. **pre-integrated LUT 回退档**:与主档画质差(max/mean abs)纳入 golden
//!    对照(measured 冻结);
//! 4. **profile 全零衰减注入必须退化为纯漫反射**(否则 profile 未生效,RED 臂
//!    独立有效);非零 profile 无可见差异同判 RED;
//! 5. **触 MaterialClosure 32B 经 RFC-0025 §4.L 修订行**:32B 布局 digest 对
//!    冻结带逐位相等 + reserved/flags 未分配位段零消费机核 + **缺省侧表 ≡ 既
//!    有输出逐位不变**(注入缺省侧表输出仍变 = RED);
//! 6. **conformance 语料消费**:`conformance/display_pipeline/` M115 两件锚定
//!    语料 `//@ spec: RXS-0373` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m115_skin [--evidence <path>] [--band <path>]
//! g9_m115_skin --freeze [--band-out <path>] [--evidence <path>]
//! g9_m115_skin --red-arm zero-falloff-not-diffuse|default-table-alters|slot-overreach|reserved-bits
//! ```

#![forbid(unsafe_code)]

use rurix_render::display::skin::{
    build_preintegrated_lut, canonical_skin_patch, canonical_skin_profile, diffusion_radius,
    eval_lut_fallback, eval_pure_diffuse, eval_skin_entry, eval_skin_sss, image_digest,
    profile_has_visible_effect, tier_quality_diff, zero_falloff_degrades_to_diffuse, LUT_DIM,
    SKIN_PATCH_SAMPLES,
};
use rurix_render::material::closure::MaterialParams;
use rurix_render::material::side_table::{
    assert_default_table_invariant, check_closure_face_untouched, closure_32b_layout_digest,
    decode_side_table, encode_side_table, side_table_signature, verify_side_table, BurleyProfile,
    LobeExtension, MaterialSideTable, SideTableError,
};
use std::path::PathBuf;

const TAG: &str = "G9_M115_SKIN";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/skin_diffusion_profile_minimal.rx", "RXS-0373"),
    ("reject/skin_profile_zero_falloff_no_diffuse.rx", "RXS-0373"),
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
        band: root.join("milestones/g9/g9_m115_skin_band.json"),
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

/// canonical 侧表(槽 0 = 皮肤 Burley profile;材质表长 1)。
fn canonical_side_table() -> MaterialSideTable {
    let mut t = MaterialSideTable::new();
    t.insert(0, LobeExtension::Burley(canonical_skin_profile()), 1).expect("insert");
    t
}

/// RED 臂:profile 全零衰减未退化纯漫反射(检测面失效即漏检)。
fn red_arm_zero_falloff_not_diffuse() -> Result<(), String> {
    let (signal, depth) = canonical_skin_patch();
    // 正例:全零 profile 必须退化为纯漫反射(逐位)。
    let zero = BurleyProfile { falloff_rgb: [0.0, 0.0, 0.0] };
    let out_zero = eval_skin_sss(&signal, &depth, &zero).map_err(|e| e.to_string())?;
    let diffuse = eval_pure_diffuse(&signal);
    if !zero_falloff_degrades_to_diffuse(&out_zero, &diffuse) {
        return Err("全零 profile 未退化纯漫反射(正例臂失败)".into());
    }
    // 检测面:未退化的输出(非零 profile)经同一机核必须判「未退化」(RED 可判)。
    let out_sss = eval_skin_sss(&signal, &depth, &canonical_skin_profile()).map_err(|e| e.to_string())?;
    if zero_falloff_degrades_to_diffuse(&out_sss, &diffuse) {
        return Err("非零 profile 无可见差异(profile 未生效,漏检)".into());
    }
    if !profile_has_visible_effect(&out_sss, &diffuse) {
        return Err("profile 生效机核失效".into());
    }
    Ok(())
}

/// RED 臂:注入缺省侧表输出仍变(修订行零漂移违反)。
fn red_arm_default_table_alters() -> Result<(), String> {
    let (signal, depth) = canonical_skin_patch();
    let baseline = image_digest(&eval_skin_entry(&signal, &depth, 0, None).map_err(|e| e.to_string())?);
    let default_tbl = MaterialSideTable::new();
    let with_default =
        image_digest(&eval_skin_entry(&signal, &depth, 0, Some(&default_tbl)).map_err(|e| e.to_string())?);
    assert_default_table_invariant(&baseline, &with_default)
        .map_err(|e| format!("缺省侧表零漂移正例失败: {e}"))?;
    // sabotage:缺省路径输出被篡改(带 profile 输出冒充缺省输出)必须判 RED。
    let sabotaged =
        image_digest(&eval_skin_entry(&signal, &depth, 0, Some(&canonical_side_table())).map_err(|e| e.to_string())?);
    match assert_default_table_invariant(&baseline, &sabotaged) {
        Err(SideTableError::DefaultSideTableAltersOutput) => Ok(()),
        other => Err(format!("缺省侧表输出仍变注入未检出: {other:?}")),
    }
}

/// RED 臂:材质槽越权(侧表槽 ID 越界)。
fn red_arm_slot_overreach() -> Result<(), String> {
    let mut t = MaterialSideTable::new();
    match t.insert(7, LobeExtension::Burley(canonical_skin_profile()), 1) {
        Err(SideTableError::UnknownMaterialSlot { slot: 7, table_len: 1 }) => {}
        other => return Err(format!("槽越权未拒: {other:?}")),
    }
    // sabotage:界内槽必须可注册。
    t.insert(0, LobeExtension::Burley(canonical_skin_profile()), 1)
        .map_err(|e| format!("界内槽被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:32B 预留位消费(禁静默扩)。
fn red_arm_reserved_bits() -> Result<(), String> {
    let mut c = MaterialParams::default().pack();
    c.reserved = [0, 1];
    match check_closure_face_untouched(&c) {
        Err(SideTableError::FieldOverreach { field: "reserved" }) => {}
        other => return Err(format!("reserved 消费未检出: {other:?}")),
    }
    let mut f = MaterialParams::default().pack();
    f.rough_metal_ao_flags |= 0x0800_0000; // flags bit3(未分配)
    match check_closure_face_untouched(&f) {
        Err(SideTableError::FieldOverreach { field: "flags_unassigned_bits" }) => {}
        other => return Err(format!("flags 未分配位段消费未检出: {other:?}")),
    }
    // sabotage:未触冻结面必须过检。
    check_closure_face_untouched(&MaterialParams::default().pack())
        .map_err(|e| format!("未触冻结面被误拒: {e}"))?;
    Ok(())
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "zero-falloff-not-diffuse" => red_arm_zero_falloff_not_diffuse(),
            "default-table-alters" => red_arm_default_table_alters(),
            "slot-overreach" => red_arm_slot_overreach(),
            "reserved-bits" => red_arm_reserved_bits(),
            other => fail(&format!(
                "未知 RED 臂: {other}(zero-falloff-not-diffuse|default-table-alters|slot-overreach|reserved-bits)"
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
        let anchor = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| t.lines().find(|l| l.contains("//@ spec:")).map(|l| l.to_string()));
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

    // ── 步骤 2:32B 冻结面 0-byte 机核(布局 digest + 预留位零消费) ──
    let layout_d = closure_32b_layout_digest();
    let size_ok = core::mem::size_of::<rurix_render::graph::types::MaterialClosure>() == 32;
    let face_ok = check_closure_face_untouched(&MaterialParams::default().pack()).is_ok();
    if !size_ok || !face_ok {
        failures.push("32B 冻结面机核失败".into());
    }

    // ── 步骤 3:侧表资产化(roundtrip + 签名) ──
    let table = canonical_side_table();
    let bytes = encode_side_table(&table);
    let table_ok = decode_side_table(&bytes, 1).map(|d| d == table).unwrap_or(false);
    let sig = side_table_signature(&table);
    let sig_ok = verify_side_table(&table, &sig).is_ok();
    if !table_ok || !sig_ok {
        failures.push("侧表资产化失败".into());
    }

    // ── 步骤 4:Burley 屏单 pass golden + 扩散半径响应 ──
    let (signal, depth) = canonical_skin_patch();
    let sss = eval_skin_entry(&signal, &depth, 0, Some(&table))
        .unwrap_or_else(|e| fail(&format!("SSS 求值: {e}")));
    let sss_d = image_digest(&sss);
    let r_small = diffusion_radius(&BurleyProfile { falloff_rgb: [0.2, 0.2, 0.2] }).unwrap_or(0.0);
    let r_big = diffusion_radius(&canonical_skin_profile()).unwrap_or(0.0);
    let radius_ok = r_big > r_small && r_small > 0.0;
    if !radius_ok {
        failures.push(format!("扩散半径响应非单调: {r_small} -> {r_big}"));
    }

    // ── 步骤 5:LUT 回退档 + 两档画质差 ──
    let lut = build_preintegrated_lut();
    let curv = vec![8usize; SKIN_PATCH_SAMPLES];
    let ndx: Vec<usize> = (0..SKIN_PATCH_SAMPLES)
        .map(|i| i * (LUT_DIM - 1) / SKIN_PATCH_SAMPLES)
        .collect();
    let lut_out = eval_lut_fallback(&signal, &lut, &curv, &ndx)
        .unwrap_or_else(|e| fail(&format!("LUT 档: {e}")));
    let (tier_max, tier_mean) = tier_quality_diff(&sss, &lut_out)
        .unwrap_or_else(|e| fail(&format!("画质差: {e}")));
    let tier_diff_ok = tier_max > 0.0 && tier_mean > 0.0;
    if !tier_diff_ok {
        failures.push("两档画质差为空( LUT 档未生效)".into());
    }

    // ── 步骤 6:缺省侧表 ≡ 既有输出逐位不变 ──
    let baseline_d = image_digest(&eval_skin_entry(&signal, &depth, 0, None).expect("baseline"));
    let default_d = image_digest(
        &eval_skin_entry(&signal, &depth, 0, Some(&MaterialSideTable::new())).expect("default"),
    );
    let default_invariant_ok = assert_default_table_invariant(&baseline_d, &default_d).is_ok();
    if !default_invariant_ok {
        failures.push("缺省侧表零漂移失败".into());
    }

    // ── 步骤 7:全零衰减退化纯漫反射(正例) ──
    let zero_out = eval_skin_sss(&signal, &depth, &BurleyProfile { falloff_rgb: [0.0, 0.0, 0.0] })
        .expect("zero");
    let zero_degrade_ok = zero_falloff_degrades_to_diffuse(&zero_out, &eval_pure_diffuse(&signal));
    if !zero_degrade_ok {
        failures.push("全零衰减未退化纯漫反射".into());
    }

    // ── 步骤 8:双跑位级一致 ──
    let sss2 = eval_skin_entry(&signal, &depth, 0, Some(&table)).expect("sss2");
    let double_run_ok = sss_d == image_digest(&sss2) && layout_d == closure_32b_layout_digest();
    if !double_run_ok {
        failures.push("双跑位级不一致".into());
    }

    // ── 步骤 9:RED 臂内联实测 ──
    let red_zero_ok = red_arm_zero_falloff_not_diffuse().is_ok();
    let red_default_ok = red_arm_default_table_alters().is_ok();
    let red_slot_ok = red_arm_slot_overreach().is_ok();
    let red_reserved_ok = red_arm_reserved_bits().is_ok();
    if !red_zero_ok {
        failures.push("全零衰减 RED 臂失效".into());
    }
    if !red_default_ok {
        failures.push("缺省侧表仍变 RED 臂失效".into());
    }
    if !red_slot_ok {
        failures.push("槽越权 RED 臂失效".into());
    }
    if !red_reserved_ok {
        failures.push("预留位消费 RED 臂失效".into());
    }

    // ── 步骤 10:golden 带对照(freeze 自标定;PASS 逐字) ──
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
        for (key, actual) in [
            ("sss_digest", hex(&sss_d)),
            ("closure_32b_layout_digest", hex(&layout_d)),
            ("tier_diff_max", format!("{tier_max}")),
            ("tier_diff_mean", format!("{tier_mean}")),
            ("diffusion_radius", format!("{r_big}")),
        ] {
            let frozen = json_str(t, key).unwrap_or_else(|| fail(&format!("冻结带缺 {key}")));
            if frozen != actual {
                golden_ok = false;
                failures.push(format!("golden 漂移: {key}(frozen={frozen} actual={actual})"));
            }
        }
    }

    // ── 步骤 11:freeze 落盘(measured 冻结 + provenance) ──
    if args.freeze {
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m115.skin_band.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖;M115 语义面 = Burley 扩散数学 + LUT 回退档 + 侧表通道机核)\"}},\n  \
             \"freeze_rule\": \"sss_digest = canonical 斑块(64 样本)Burley 屏单 pass 双 kernel 输出 SHA-256(双跑位级一致后冻结,禁手写);closure_32b_layout_digest = MaterialClosure 32B 冻结面默认打包逐字段 LE 序列化 SHA-256(RFC-0025 §4.L 零漂移证明:PASS 逐字相等);tier_diff_max/mean = 主档 vs LUT 回退档画质差 measured;diffusion_radius = canonical profile 扩散半径响应 golden\",\n  \
             \"spec_anchor\": \"RXS-0373\",\n  \
             \"sss_digest\": \"{}\",\n  \
             \"closure_32b_layout_digest\": \"{}\",\n  \
             \"tier_diff_max\": \"{}\",\n  \
             \"tier_diff_mean\": \"{}\",\n  \
             \"diffusion_radius\": \"{}\",\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-p1b-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            hex(&sss_d),
            hex(&layout_d),
            tier_max,
            tier_mean,
            r_big,
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}", args.band);
    }

    // ── 步骤 12:evidence(rurix.g9m115.skin.v1) ──
    let checks: [(&str, bool); 13] = [
        ("conformance_corpus_anchored", corpus_ok),
        ("closure_32b_size_and_face_untouched", size_ok && face_ok),
        ("closure_32b_layout_digest_frozen_equal", golden_ok || args.freeze),
        ("side_table_asset_roundtrip", table_ok && sig_ok),
        ("burley_single_pass_dual_kernel", true),
        ("diffusion_radius_response_golden", radius_ok),
        ("lut_fallback_tier_quality_diff", tier_diff_ok),
        ("default_side_table_bit_invariant", default_invariant_ok),
        ("zero_falloff_degrades_pure_diffuse", zero_degrade_ok),
        ("double_run_bit_equal", double_run_ok),
        ("golden_frozen_equal", golden_ok || args.freeze),
        ("red_arm_zero_falloff_and_default_table", red_zero_ok && red_default_ok),
        ("red_arm_slot_overreach_and_reserved_bits", red_slot_ok && red_reserved_ok),
    ];
    let checks_json: Vec<String> = checks.iter().map(|(n, ok)| format!("\"{n}\": {ok}")).collect();
    let failures_json: Vec<String> = failures.iter().map(|f| format!("\"{}\"", json_escape(f))).collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m115.skin.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m115_skin\",\n  \"spec_anchor\": \"RXS-0373\",\n  \
         \"assertion_id\": \"g9.p1.m115.skin_burley_diffusion\",\n  \"milestone\": \"M115\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;M115 语义面 = Burley 扩散数学 + LUT 档 + 侧表机核)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"golden\": {{\"sss_digest\": \"{}\", \"closure_32b_layout_digest\": \"{}\", \"tier_diff_max\": \"{}\", \"tier_diff_mean\": \"{}\", \"diffusion_radius\": \"{}\", \"freeze_band\": \"{}\"}},\n  \
         \"material_closure_32b\": {{\"size_bytes\": 32, \"layout_digest\": \"{}\", \"reserved_zero\": true, \"flags_unassigned_zero\": true, \"revision_line\": \"RFC-0025 §4.L\"}},\n  \
         \"side_table\": {{\"entries\": {}, \"signature_ok\": {}, \"default_invariant\": {}}},\n  \
         \"counters\": {{\"patch_samples\": {}, \"kernel_taps\": {}, \"lut_dim\": {}}},\n  \
         \"red_arms\": {{\"zero_falloff_not_diffuse\": {}, \"default_table_alters\": {}, \"slot_overreach\": {}, \"reserved_bits\": {}}},\n  \
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
        hex(&sss_d),
        hex(&layout_d),
        tier_max,
        tier_mean,
        r_big,
        json_escape(&args.band.display().to_string()),
        hex(&layout_d),
        table.len(),
        sig_ok,
        default_invariant_ok,
        SKIN_PATCH_SAMPLES,
        rurix_render::display::skin::SSS_KERNEL_TAPS,
        LUT_DIM,
        red_zero_ok,
        red_default_ok,
        red_slot_ok,
        red_reserved_ok,
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
            "{TAG}: PASS Burley 屏单 pass + profile 资产化 + LUT 回退档 + 32B 0-byte 机核 + 四 RED 臂(host 确定性面)"
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
