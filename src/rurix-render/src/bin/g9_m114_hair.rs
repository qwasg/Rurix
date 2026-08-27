//! G9.5 M114 毛发 Marschner 三瓣 harness(RXS-0372;门 `g9.p1.m114.hair_marschner`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M114 行逐字 + spec/display_pipeline.md RXS-0372)
//!
//! 1. **Marschner R/TT/TRT 三瓣逐瓣对拍 golden**(纵向/方位角分离参数化为资产
//!    属性,经 §4.L 侧表通道按材质槽 ID 索引接入)+ **瓣能量守恒**(逐样本总
//!    瓣能 ≤ 1 机核,max measured 冻结);
//! 2. **单瓣系数置零的 RED 渲染独立有效**(缺 TT 瓣必须可见差异,无差异即管
//!    线未接通);
//! 3. **几何三档**(近 strand/中 card/远 mesh)档间切换距离 + strand→card 股
//!    替换映射离线烘焙确定性 golden(双构建逐位一致);card/mesh 档走默认半
//!    透明路径;
//! 4. **strand 档强制精确 OIT——分项 not-triggered 登记**:消费 M120 测量冻
//!    结带,数据可得性如实记录(承接锚「M120 精确档 benchmark 裁决数据落地后
//!    重判,兜底 G9.7 穷举」),`counts_as_green = false` 不充绿;strand 档请
//!    求排序 fallback/默认半透明路径即 RED(排序依赖缺失);
//! 5. **触 32B 经 RFC-0025 §4.L 修订行**(侧表越权即 RED);
//! 6. **conformance 语料消费**:`conformance/display_pipeline/` M114 两件锚定
//!    语料 `//@ spec: RXS-0372` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m114_hair [--evidence <path>] [--band <path>]
//! g9_m114_hair --freeze [--band-out <path>] [--evidence <path>]
//! g9_m114_hair --red-arm lobe-tt-zeroed|strand-sorted-fallback|side-table-overreach
//! ```

#![forbid(unsafe_code)]

use rurix_render::display::hair::{
    HairError, HairLobes, HairTier, STRAND_TIER_ANCHOR, StrandTierStatus, TranslucencyPath,
    ZeroLobe, assert_lobe_wired, bake_strand_replacement, canonical_marschner, canonical_sweep,
    canonical_switch_table, hair_params_from_side_table, lobe_digests, marschner_lobes_zeroed,
    max_total_lobe_energy, register_strand_tier, replacement_digest, request_strand_translucency,
    tier_for_distance, tier_translucency_path,
};
use rurix_render::material::side_table::{
    LobeExtension, MaterialSideTable, SideTableError, side_table_signature, verify_side_table,
};
use std::path::PathBuf;

const TAG: &str = "G9_M114_HAIR";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/hair_marschner_lobes_minimal.rx", "RXS-0372"),
    ("reject/hair_lobe_tt_zeroed_no_diff.rx", "RXS-0372"),
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
        band: root.join("milestones/g9/g9_m114_hair_band.json"),
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

/// 三瓣合并 digest(单瓣置零对拍载体)。
fn combined_digest(sweep: &[HairLobes]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(sweep.len() * 12);
    for l in sweep {
        buf.extend_from_slice(&l.r.to_le_bytes());
        buf.extend_from_slice(&l.tt.to_le_bytes());
        buf.extend_from_slice(&l.trt.to_le_bytes());
    }
    rurix_pkg::sha256::digest(&buf)
}

/// 置零扫描(harness 面;与模块单测同形态)。
fn sweep_zeroed(
    params: &rurix_render::material::side_table::MarschnerParams,
    zero: ZeroLobe,
) -> Vec<HairLobes> {
    let mut out = Vec::new();
    let mut ti = -0.6f32;
    while ti <= 0.6f32 {
        let mut k = 0u32;
        while k <= 36 {
            let phi = k as f32 * std::f32::consts::PI / 36.0;
            out.push(marschner_lobes_zeroed(params, 0.3, ti, phi, zero).expect("z"));
            k += 1;
        }
        ti += 0.05;
    }
    out
}

/// canonical 侧表(槽 0 = 毛发 Marschner 参数集;材质表长 1)。
fn canonical_side_table() -> MaterialSideTable {
    let mut t = MaterialSideTable::new();
    t.insert(0, LobeExtension::Marschner(canonical_marschner()), 1)
        .expect("insert");
    t
}

/// RED 臂:单瓣置零无差异(缺 TT 瓣必须可见差异)。
fn red_arm_lobe_tt_zeroed() -> Result<(), String> {
    let p = canonical_marschner();
    let full_d = combined_digest(&canonical_sweep(&p));
    for zero in [ZeroLobe::R, ZeroLobe::Tt, ZeroLobe::Trt] {
        let zd = combined_digest(&sweep_zeroed(&p, zero));
        assert_lobe_wired(&full_d, &zd, zero)
            .map_err(|e| format!("{} 瓣置零无差异: {e}", zero.as_str()))?;
    }
    // sabotage:置零前后 digest 相同必须判管线未接通 RED。
    match assert_lobe_wired(&full_d, &full_d, ZeroLobe::Tt) {
        Err(HairError::LobeNotWired { lobe: "TT" }) => Ok(()),
        other => Err(format!("置零无差异注入未检出: {other:?}")),
    }
}

/// RED 臂:strand 档排序依赖缺失(请求排序 fallback / 默认半透明路径)。
fn red_arm_strand_sorted_fallback() -> Result<(), String> {
    match request_strand_translucency(TranslucencyPath::SortedFallback) {
        Err(HairError::StrandTierRequiresExactOit {
            requested: "sorted_fallback",
        }) => {}
        other => return Err(format!("strand 排序 fallback 未拒: {other:?}")),
    }
    match request_strand_translucency(TranslucencyPath::DefaultTranslucent) {
        Err(HairError::StrandTierRequiresExactOit { .. }) => {}
        other => return Err(format!("strand 默认半透明路径未拒: {other:?}")),
    }
    // sabotage:精确 linked-list(仅毛发 strand 作用域)必须合法。
    request_strand_translucency(TranslucencyPath::ExactLinkedList)
        .map_err(|e| format!("精确档被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:侧表越权(槽越界 / 毛发槽误挂 Burley 扩展)。
fn red_arm_side_table_overreach() -> Result<(), String> {
    let mut t = MaterialSideTable::new();
    match t.insert(3, LobeExtension::Marschner(canonical_marschner()), 1) {
        Err(SideTableError::UnknownMaterialSlot { .. }) => {}
        other => return Err(format!("槽越权未拒: {other:?}")),
    }
    let mut wrong = MaterialSideTable::new();
    wrong
        .insert(
            0,
            LobeExtension::Burley(rurix_render::material::side_table::BurleyProfile {
                falloff_rgb: [0.5, 0.3, 0.2],
            }),
            1,
        )
        .map_err(|e| format!("burley 注册: {e}"))?;
    match hair_params_from_side_table(&wrong, 0) {
        Err(HairError::SideTable(_)) => {}
        other => return Err(format!("误挂 Burley 扩展未拒: {other:?}")),
    }
    // sabotage:合法 Marschner 槽必须可取。
    hair_params_from_side_table(&canonical_side_table(), 0)
        .map_err(|e| format!("合法槽被误拒: {e}"))?;
    Ok(())
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "lobe-tt-zeroed" => red_arm_lobe_tt_zeroed(),
            "strand-sorted-fallback" => red_arm_strand_sorted_fallback(),
            "side-table-overreach" => red_arm_side_table_overreach(),
            other => fail(&format!(
                "未知 RED 臂: {other}(lobe-tt-zeroed|strand-sorted-fallback|side-table-overreach)"
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

    // ── 步骤 2:侧表通道消费 Marschner 参数集(§4.L) ──
    let table = canonical_side_table();
    let sig = side_table_signature(&table);
    let table_ok = verify_side_table(&table, &sig).is_ok()
        && hair_params_from_side_table(&table, 0)
            .map(|p| p == canonical_marschner())
            .unwrap_or(false);
    if !table_ok {
        failures.push("侧表通道消费失败".into());
    }

    // ── 步骤 3:三瓣逐瓣 golden + 瓣能量守恒 ──
    let params = canonical_marschner();
    let sweep = canonical_sweep(&params);
    let (dr, dtt, dtrt) = lobe_digests(&sweep);
    let max_energy = max_total_lobe_energy(&sweep);
    let energy_ok = max_energy <= 1.0 && max_energy > 0.0;
    if !energy_ok {
        failures.push(format!("瓣能量守恒违反: max {max_energy}"));
    }

    // ── 步骤 4:几何三档 + 股替换烘焙确定性 ──
    let switch = canonical_switch_table();
    let tiers_ok = tier_for_distance(&switch, 3.0) == Ok(HairTier::Strand)
        && tier_for_distance(&switch, 30.0) == Ok(HairTier::Card)
        && tier_for_distance(&switch, 300.0) == Ok(HairTier::Mesh)
        && tier_translucency_path(HairTier::Card) == TranslucencyPath::DefaultTranslucent
        && tier_translucency_path(HairTier::Mesh) == TranslucencyPath::DefaultTranslucent;
    if !tiers_ok {
        failures.push("几何三档闭集失效".into());
    }
    let bake_a = bake_strand_replacement(64).unwrap_or_else(|e| fail(&format!("烘焙: {e}")));
    let bake_b = bake_strand_replacement(64).expect("bake_b");
    let bake_d = replacement_digest(&bake_a);
    let bake_ok = bake_a == bake_b && bake_d == replacement_digest(&bake_b);
    if !bake_ok {
        failures.push("股替换烘焙双构建不一致".into());
    }

    // ── 步骤 5:strand 档 not-triggered 登记(消费 M120 测量冻结带) ──
    let m120_path = root.join("milestones/g9/g9_m120_oit_measurements.json");
    let m120_text = std::fs::read_to_string(&m120_path).ok();
    let reg = register_strand_tier(m120_text.as_deref());
    let strand_registered = reg.status == StrandTierStatus::NotTriggered
        && !reg.counts_as_green
        && reg.anchor == STRAND_TIER_ANCHOR;
    if !strand_registered {
        failures.push("strand 档 not-triggered 登记失效".into());
    }
    let m120_data_ok = reg.m120.measurements_present && reg.m120.linked_list_record_count > 0;
    if !m120_data_ok {
        failures.push("M120 测量带不可得(linked_list 记录缺失)".into());
    }

    // ── 步骤 6:双跑位级一致 ──
    let sweep2 = canonical_sweep(&params);
    let double_run_ok = (dr, dtt, dtrt) == lobe_digests(&sweep2);
    if !double_run_ok {
        failures.push("双跑位级不一致".into());
    }

    // ── 步骤 7:RED 臂内联实测 ──
    let red_lobe_ok = red_arm_lobe_tt_zeroed().is_ok();
    let red_strand_ok = red_arm_strand_sorted_fallback().is_ok();
    let red_table_ok = red_arm_side_table_overreach().is_ok();
    if !red_lobe_ok {
        failures.push("单瓣置零 RED 臂失效".into());
    }
    if !red_strand_ok {
        failures.push("strand 排序依赖缺失 RED 臂失效".into());
    }
    if !red_table_ok {
        failures.push("侧表越权 RED 臂失效".into());
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
        for (key, actual) in [
            ("lobe_r_digest", hex(&dr)),
            ("lobe_tt_digest", hex(&dtt)),
            ("lobe_trt_digest", hex(&dtrt)),
            ("max_total_lobe_energy", format!("{max_energy}")),
            ("replacement_digest", hex(&bake_d)),
        ] {
            let frozen = json_str(t, key).unwrap_or_else(|| fail(&format!("冻结带缺 {key}")));
            if frozen != actual {
                golden_ok = false;
                failures.push(format!(
                    "golden 漂移: {key}(frozen={frozen} actual={actual})"
                ));
            }
        }
    }

    // ── 步骤 9:freeze 落盘(measured 冻结 + provenance) ──
    if args.freeze {
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m114.hair_band.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖;M114 语义面 = Marschner 三瓣数学 + 几何三档 + 烘焙确定性)\"}},\n  \
             \"freeze_rule\": \"lobe_*_digest = canonical Marschner 参数集角度扫描(θ_r∈±0.6 步 0.05,φ∈0..π 步 π/36)逐瓣序列 SHA-256(逐瓣对拍 golden,双跑位级一致后冻结,禁手写);max_total_lobe_energy = 瓣能量守恒 measured(逐样本总瓣能上界 1);replacement_digest = 64 strand/聚类8 股替换映射 SHA-256(烘焙确定性 golden)\",\n  \
             \"spec_anchor\": \"RXS-0372\",\n  \
             \"lobe_r_digest\": \"{}\",\n  \
             \"lobe_tt_digest\": \"{}\",\n  \
             \"lobe_trt_digest\": \"{}\",\n  \
             \"max_total_lobe_energy\": \"{}\",\n  \
             \"replacement_digest\": \"{}\",\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-p1b-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            hex(&dr),
            hex(&dtt),
            hex(&dtrt),
            max_energy,
            hex(&bake_d),
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}", args.band);
    }

    // ── 步骤 10:evidence(rurix.g9m114.hair.v1;strand 档 not-triggered 登记可见) ──
    let checks: [(&str, bool); 12] = [
        ("conformance_corpus_anchored", corpus_ok),
        ("side_table_channel_consumed", table_ok),
        (
            "marschner_three_lobes_per_lobe_golden",
            golden_ok || args.freeze,
        ),
        ("lobe_energy_conservation", energy_ok),
        ("geometry_three_tiers_closed", tiers_ok),
        ("strand_replacement_bake_deterministic", bake_ok),
        ("strand_tier_not_triggered_registered", strand_registered),
        ("m120_measurements_availability_recorded", m120_data_ok),
        ("double_run_bit_equal", double_run_ok),
        ("golden_frozen_equal", golden_ok || args.freeze),
        (
            "red_arm_lobe_zeroed_and_strand_sorted",
            red_lobe_ok && red_strand_ok,
        ),
        ("red_arm_side_table_overreach", red_table_ok),
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
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m114.hair.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m114_hair\",\n  \"spec_anchor\": \"RXS-0372\",\n  \
         \"assertion_id\": \"g9.p1.m114.hair_marschner\",\n  \"milestone\": \"M114\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;M114 语义面 = Marschner 三瓣数学 + 三档 + 烘焙确定性)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"golden\": {{\"lobe_r_digest\": \"{}\", \"lobe_tt_digest\": \"{}\", \"lobe_trt_digest\": \"{}\", \"max_total_lobe_energy\": \"{}\", \"replacement_digest\": \"{}\", \"freeze_band\": \"{}\"}},\n  \
         \"strand_tier\": {{\"status\": \"not-triggered\", \"counts_as_green\": false, \"anchor\": \"{}\", \"m120\": {{\"measurements_present\": {}, \"measurements_digest\": \"{}\", \"linked_list_record_count\": {}, \"host_only_reference\": {}, \"verdict\": \"{}\"}}}},\n  \
         \"counters\": {{\"sweep_samples\": {}, \"replacement_entries\": {}, \"lobe_weights\": [0.35, 0.45, 0.20]}},\n  \
         \"red_arms\": {{\"lobe_tt_zeroed\": {}, \"strand_sorted_fallback\": {}, \"side_table_overreach\": {}}},\n  \
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
        hex(&dr),
        hex(&dtt),
        hex(&dtrt),
        max_energy,
        hex(&bake_d),
        json_escape(&args.band.display().to_string()),
        json_escape(reg.anchor),
        reg.m120.measurements_present,
        hex(&reg.m120.measurements_digest),
        reg.m120.linked_list_record_count,
        reg.m120.host_only_reference,
        json_escape(reg.m120.verdict),
        sweep.len(),
        bake_a.entries.len(),
        red_lobe_ok,
        red_strand_ok,
        red_table_ok,
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
            "{TAG}: PASS 三瓣逐瓣 golden + 能量守恒 + 三档 + 烘焙确定性 + strand not-triggered 登记 + 三 RED 臂(host 确定性面)"
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
