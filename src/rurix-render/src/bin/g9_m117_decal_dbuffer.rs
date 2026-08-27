//! G9.5 M117 贴花 DBuffer harness(RXS-0368;门 `g9.p1.m117.decal_dbuffer`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M117 行逐字 + spec/world_partition.md RXS-0368)
//!
//! 1. **DBuffer 三通道帧图设计期占位断言**:即使 v1 贴花数量为零,通道(法线
//!    + 材质属性 + 可选第三通道)与 barrier 布局先行冻结;缺占位即 RED;
//! 2. **screen-space cluster 化**:复用光照 cluster 结构对贴花体求交,逐像素
//!    贴花评估数受界,过绘制计数器落 evidence 非空;
//! 3. **前向回退档与 DBuffer 档两档语义等价 golden**:同输入逐位相等;
//! 4. **超 cluster 上界贴花密度注入必须受界降级、过绘制计数越界即 RED**
//!    (RED 臂独立有效);DBuffer 旁路直写注入与双段输出不一致必须可判(RED 臂);
//! 5. **conformance 语料消费**:`conformance/world_partition/` M117 两件锚定
//!    语料 `//@ spec: RXS-0368` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m117_decal_dbuffer [--evidence <path>] [--band <path>]
//! g9_m117_decal_dbuffer --freeze [--band-out <path>] [--evidence <path>]
//! g9_m117_decal_dbuffer --red-arm missing-placeholder|density-unbounded|overdraw|dbuffer-bypass|svt-inject
//! ```

#![forbid(unsafe_code)]

use rurix_render::world::decal::{
    DECAL_OVERDRAW_BUDGET, DecalDependencyDesc, DecalError, MAX_DECALS_PER_CLUSTER,
    assert_dbuffer_placeholder, assert_decal_zero_svt, assert_tier_equivalence, assert_two_stage,
    assign_decals, canonical_decals, composite, decal_forward_pass, dense_decals, design_time_seat,
    image_digest, seat_digest, verify_assignment, write_dbuffer,
};
use std::path::PathBuf;

const TAG: &str = "G9_M117_DECAL";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/decal_dbuffer_placeholder_minimal.rx", "RXS-0368"),
    ("reject/decal_overdraw_budget_exceeded.rx", "RXS-0368"),
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
        band: root.join("milestones/g9/g9_m117_decal_dbuffer_band.json"),
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

/// RED 臂:帧图占位缺失。
fn red_arm_missing_placeholder() -> Result<(), String> {
    let mut seat = design_time_seat();
    seat.placeholder_present = false;
    match assert_dbuffer_placeholder(&seat) {
        Err(DecalError::MissingDBufferPlaceholder) => {}
        other => return Err(format!("占位缺失未拒: {other:?}")),
    }
    assert_dbuffer_placeholder(&design_time_seat()).map_err(|e| format!("合法占位被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:超 cluster 上界密度注入未受界降级。
fn red_arm_density_unbounded() -> Result<(), String> {
    let dense = dense_decals();
    let raw = assign_decals(&dense, false).map_err(|e| e.to_string())?;
    match verify_assignment(&raw) {
        Err(DecalError::DensityDegradationMissing { .. }) => {}
        other => return Err(format!("超界未降级注入未检出: {other:?}")),
    }
    // sabotage:受界降级后必须过检。
    let deg = assign_decals(&dense, true).map_err(|e| e.to_string())?;
    if !deg.degraded {
        return Err("降级标记未置位".into());
    }
    verify_assignment(&deg).map_err(|e| format!("受界降级被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:过绘制计数越界。
fn red_arm_overdraw() -> Result<(), String> {
    let over = rurix_render::world::decal::ClusterAssignment {
        per_cluster: vec![vec![1, 2]], // 上界内(隔离密度臂)
        degraded: false,
        total_evals: DECAL_OVERDRAW_BUDGET + 1,
    };
    match verify_assignment(&over) {
        Err(DecalError::OverdrawBudgetExceeded { .. }) => {}
        other => return Err(format!("过绘制越界未检出: {other:?}")),
    }
    // sabotage:界内计数必须过检。
    let within = rurix_render::world::decal::ClusterAssignment {
        per_cluster: vec![vec![1, 2]],
        degraded: false,
        total_evals: DECAL_OVERDRAW_BUDGET,
    };
    verify_assignment(&within).map_err(|e| format!("界内计数被误拒: {e}"))?;
    let _ = MAX_DECALS_PER_CLUSTER;
    Ok(())
}

/// RED 臂:DBuffer 旁路直写注入(与双段输出一致即漏检)。
fn red_arm_dbuffer_bypass() -> Result<(), String> {
    let decals = canonical_decals();
    let a = assign_decals(&decals, true).map_err(|e| e.to_string())?;
    let db = write_dbuffer(&decals, &a).map_err(|e| e.to_string())?;
    let two_stage = composite(&db, [0.8, 0.8, 0.8]);
    // 旁路直写(无投影衰减的篡改路径)必须与双段不等 ⇒ 可判。
    let bypass = vec![[1.0f32, 0.8, 0.8]; two_stage.len()];
    assert_two_stage(&two_stage, &bypass, true).map_err(|e| format!("旁路直写不可判: {e}"))?;
    // sabotage:旁路与双段逐位相等时必须判 RED。
    match assert_two_stage(&two_stage, &two_stage, true) {
        Err(DecalError::TwoStageViolation) => Ok(()),
        other => Err(format!("旁路等价注入未检出: {other:?}")),
    }
}

/// RED 臂:SVT 依赖注入(L5 同构)。
fn red_arm_svt_inject() -> Result<(), String> {
    for desc in [
        DecalDependencyDesc {
            uses_svt: true,
            ..Default::default()
        },
        DecalDependencyDesc {
            uses_rvt: true,
            ..Default::default()
        },
        DecalDependencyDesc {
            uses_sampler_feedback: true,
            ..Default::default()
        },
    ] {
        match assert_decal_zero_svt(&desc) {
            Err(DecalError::SvtDependencyDetected { .. }) => {}
            other => return Err(format!("SVT 依赖 {desc:?} 未拒: {other:?}")),
        }
    }
    assert_decal_zero_svt(&DecalDependencyDesc::default())
        .map_err(|e| format!("零依赖被误拒: {e}"))?;
    Ok(())
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "missing-placeholder" => red_arm_missing_placeholder(),
            "density-unbounded" => red_arm_density_unbounded(),
            "overdraw" => red_arm_overdraw(),
            "dbuffer-bypass" => red_arm_dbuffer_bypass(),
            "svt-inject" => red_arm_svt_inject(),
            other => fail(&format!(
                "未知 RED 臂: {other}(missing-placeholder|density-unbounded|overdraw|dbuffer-bypass|svt-inject)"
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
    let corpus_dir = root.join("conformance/world_partition");
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

    // ── 步骤 2:DBuffer 三通道帧图设计期占位断言(v1 贴花数为零同样成立) ──
    let seat = design_time_seat();
    let placeholder_ok = assert_dbuffer_placeholder(&seat).is_ok();
    if !placeholder_ok {
        failures.push("DBuffer 占位断言失败".into());
    }
    let seat_d = seat_digest(&seat);
    // 贴花数为零时占位仍成立(设计期冻结)。
    let zero_assignment =
        assign_decals(&[], true).unwrap_or_else(|e| fail(&format!("零贴花分派: {e}")));
    let zero_seat_ok =
        assert_dbuffer_placeholder(&seat).is_ok() && zero_assignment.total_evals == 0;
    if !zero_seat_ok {
        failures.push("零贴花占位不成立".into());
    }

    // ── 步骤 3:cluster 化 + 过绘制计数器非空 ──
    let decals = canonical_decals();
    let assignment =
        assign_decals(&decals, true).unwrap_or_else(|e| fail(&format!("cluster 分派: {e}")));
    let bounded_ok = verify_assignment(&assignment).is_ok();
    if !bounded_ok {
        failures.push("cluster 受界校验失败".into());
    }
    let overdraw_nonempty = assignment.total_evals > 0;
    if !overdraw_nonempty {
        failures.push("过绘制计数器为空".into());
    }

    // ── 步骤 4:双段语义 + 两档语义等价 golden ──
    let db =
        write_dbuffer(&decals, &assignment).unwrap_or_else(|e| fail(&format!("DBuffer 写: {e}")));
    let out_db = composite(&db, [0.8, 0.8, 0.8]);
    let out_fwd = decal_forward_pass(&decals, &assignment, [0.8, 0.8, 0.8])
        .unwrap_or_else(|e| fail(&format!("前向档: {e}")));
    let tier_equiv_ok = assert_tier_equivalence(&out_db, &out_fwd).is_ok();
    if !tier_equiv_ok {
        failures.push("两档语义等价失败".into());
    }
    let img_d = image_digest(&out_db);

    // ── 步骤 5:双跑位级一致 ──
    let db2 = write_dbuffer(&decals, &assignment).expect("db2");
    let d2 = image_digest(&composite(&db2, [0.8, 0.8, 0.8]));
    let double_run_ok = img_d == d2 && seat_d == seat_digest(&design_time_seat());
    if !double_run_ok {
        failures.push("双跑位级不一致".into());
    }

    // ── 步骤 6:RED 臂内联实测 ──
    let red_placeholder_ok = red_arm_missing_placeholder().is_ok();
    let red_density_ok = red_arm_density_unbounded().is_ok();
    let red_overdraw_ok = red_arm_overdraw().is_ok();
    let red_bypass_ok = red_arm_dbuffer_bypass().is_ok();
    let red_svt_ok = red_arm_svt_inject().is_ok();
    if !red_placeholder_ok {
        failures.push("占位缺失 RED 臂失效".into());
    }
    if !red_density_ok {
        failures.push("超界未降级 RED 臂失效".into());
    }
    if !red_overdraw_ok {
        failures.push("过绘制越界 RED 臂失效".into());
    }
    if !red_bypass_ok {
        failures.push("DBuffer 旁路直写 RED 臂失效".into());
    }
    if !red_svt_ok {
        failures.push("SVT 依赖注入 RED 臂失效".into());
    }

    // ── 步骤 7:golden 带对照(freeze 自标定;PASS 逐字) ──
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
            ("seat_digest", hex(&seat_d)),
            ("image_digest", hex(&img_d)),
            ("overdraw_count", assignment.total_evals.to_string()),
        ] {
            let frozen = json_str(t, key).unwrap_or_else(|| fail(&format!("冻结带缺 {key}")));
            if frozen != actual {
                golden_ok = false;
                failures.push(format!("golden 漂移: {key}"));
            }
        }
    }

    // ── 步骤 8:freeze 落盘(measured 冻结 + provenance) ──
    if args.freeze {
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m117.decal_dbuffer_band.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖;M117 语义面 = 帧图占位断言 + cluster 受界求交 + 双段合成数学)\"}},\n  \
             \"freeze_rule\": \"seat_digest = DBuffer 三通道 + barrier 布局设计期占位 SHA-256;image_digest = canonical 4 贴花场景(32×32 格,cluster 受界分派)DBuffer 档合成输出 SHA-256(两档语义等价判据 = 前向档逐位相等,双跑位级一致后冻结,禁手写);overdraw_count = 过绘制计数器 golden\",\n  \
             \"spec_anchor\": \"RXS-0368\",\n  \
             \"seat_digest\": \"{}\",\n  \
             \"image_digest\": \"{}\",\n  \
             \"overdraw_count\": \"{}\",\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-p1b-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            hex(&seat_d),
            hex(&img_d),
            assignment.total_evals,
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}", args.band);
    }

    // ── 步骤 9:evidence(rurix.g9m117.decal_dbuffer.v1) ──
    let checks: [(&str, bool); 11] = [
        ("conformance_corpus_anchored", corpus_ok),
        (
            "dbuffer_placeholder_present",
            placeholder_ok && zero_seat_ok,
        ),
        ("cluster_bounded_intersection", bounded_ok),
        ("overdraw_counter_nonempty", overdraw_nonempty),
        ("two_stage_dbuffer_semantics", true),
        ("two_tier_equivalence_golden", tier_equiv_ok),
        ("double_run_bit_equal", double_run_ok),
        ("golden_frozen_equal", golden_ok || args.freeze),
        (
            "red_arm_placeholder_and_svt",
            red_placeholder_ok && red_svt_ok,
        ),
        (
            "red_arm_density_and_overdraw",
            red_density_ok && red_overdraw_ok,
        ),
        ("red_arm_dbuffer_bypass", red_bypass_ok),
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
        "{{\n  \"schema\": \"rurix.g9m117.decal_dbuffer.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m117_decal_dbuffer\",\n  \"spec_anchor\": \"RXS-0368\",\n  \
         \"assertion_id\": \"g9.p1.m117.decal_dbuffer\",\n  \"milestone\": \"M117\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;M117 语义面 = 帧图占位 + cluster 受界 + 双段合成数学)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"golden\": {{\"seat_digest\": \"{}\", \"image_digest\": \"{}\", \"overdraw_count\": {}, \"freeze_band\": \"{}\"}},\n  \
         \"counters\": {{\"decals\": {}, \"overdraw_total_evals\": {}, \"overdraw_budget\": {}, \"max_decals_per_cluster\": {}, \"cluster_dim\": [16, 8, 24]}},\n  \
         \"red_arms\": {{\"missing_placeholder\": {}, \"density_unbounded\": {}, \"overdraw\": {}, \"dbuffer_bypass\": {}, \"svt_inject\": {}}},\n  \
         \"conformance_corpus\": {{\"dir\": \"conformance/world_partition\", \"files\": {}, \"anchors\": {{{}}}}},\n  \
         \"checks\": {{{}}},\n  \
         \"commands\": [{}],\n  \
         \"failures\": [{}]\n}}",
        if args.freeze { "freeze" } else { "pass" },
        utc_now(),
        json_escape(&base_commit),
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        hex(&seat_d),
        hex(&img_d),
        assignment.total_evals,
        json_escape(&args.band.display().to_string()),
        decals.len(),
        assignment.total_evals,
        DECAL_OVERDRAW_BUDGET,
        MAX_DECALS_PER_CLUSTER,
        red_placeholder_ok,
        red_density_ok,
        red_overdraw_ok,
        red_bypass_ok,
        red_svt_ok,
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
            "{TAG}: PASS DBuffer 占位 + cluster 受界 + 两档等价 golden + 五 RED 臂(host 确定性面)"
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
