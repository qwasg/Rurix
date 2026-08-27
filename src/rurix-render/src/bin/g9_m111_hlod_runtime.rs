//! G9.5 M111 HLOD 运行时 harness(RXS-0364;门 `g9.p1.m111.hlod_runtime`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M111 行逐字 + spec/world_partition.md RXS-0364)
//!
//! 1. **HLOD 运行时选择/切换(screen-size 阈值互斥)**:同一视距序列产出确定性
//!    层级序列(双跑位级一致,digest 机核);层级序列对冻结 golden 逐字相等
//!    (measured 冻结,禁手写);互斥断言全真(同帧同 cell 只出一种内容);
//! 2. **运行时零合并断言(RED 臂)**:HLOD 资产来自离线烘焙,运行时合并/简化/
//!    重建调用尝试一律 fail-closed typed Err;sabotage 探针(正常选择/事件面)
//!    不被误拒(能红证明);
//! 3. **与 M110 cell 事件总线接线**:HLOD 层级随 cell 驻留状态切换(事件总线
//!    drain ⇒ HlodRuntime 驻留集迁移;乱序事件流注入必拒);
//! 4. **M110 烘焙产物双构建 hash 相等运行时核验臂**:实载产物 digest 与 cell
//!    元数据引用一致 ⇒ 登记成功;篡改 digest ⇒ DigestMismatch fail-closed;
//! 5. **conformance 语料消费**:`conformance/world_partition/` M111 两件锚定
//!    语料 `//@ spec: RXS-0364` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m111_hlod_runtime [--evidence <path>] [--band <path>]
//! g9_m111_hlod_runtime --freeze [--band-out <path>] [--evidence <path>]
//! g9_m111_hlod_runtime --red-arm runtime-merge|level-perturb|digest-mismatch
//! ```

#![forbid(unsafe_code)]

use rurix_render::world::hlod::{
    HlodError, HlodRuntime, ScreenSizeThresholds, SelectedContent, canonical_distance_path,
    canonical_thresholds, selection_log_digest,
};
use rurix_render::world::partition::{
    PartitionRuntime, canonical_budget, canonical_camera_path, canonical_world,
};
use std::path::PathBuf;

const TAG: &str = "G9_M111_HLOD";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/hlod_baking_double_build_minimal.rx", "RXS-0364"),
    ("reject/hlod_runtime_merge_forbidden.rx", "RXS-0364"),
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
        band: root.join("milestones/g9/g9_m111_hlod_runtime_band.json"),
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

/// 装配:canonical 世界 + 流送运行时 + HLOD 运行时,事件总线逐帧接线后返回。
fn assemble(frames: u32) -> (rurix_render::world::partition::PersistentWorld, HlodRuntime) {
    let world = canonical_world();
    let mut prt = PartitionRuntime::new(world.clone(), canonical_budget())
        .unwrap_or_else(|e| fail(&format!("PartitionRuntime 装配: {e}")));
    let mut rt = HlodRuntime::new();
    let path = canonical_camera_path(frames);
    for (f, s) in path.iter().enumerate() {
        prt.tick(f as u32, std::slice::from_ref(s))
            .unwrap_or_else(|e| fail(&format!("tick: {e}")));
        rt.apply_cell_events(&prt.drain_events())
            .unwrap_or_else(|e| fail(&format!("事件总线接线: {e}")));
    }
    (world, rt)
}

/// 产层级序列:对 resident HLOD cell 按 canonical 视距序列逐帧选择。
fn produce_selection_log(
    world: &rurix_render::world::partition::PersistentWorld,
    rt: &mut HlodRuntime,
) -> Vec<rurix_render::world::hlod::SelectionRecord> {
    let thresholds = canonical_thresholds();
    let hlod_cells: Vec<u32> = rt
        .resident()
        .iter()
        .copied()
        .filter(|&c| world.cells[c as usize].hlod.is_some())
        .collect();
    if hlod_cells.is_empty() {
        fail("canonical 场景无驻留 HLOD cell");
    }
    let path = canonical_distance_path(32);
    for (f, d) in path.iter().enumerate() {
        for &c in &hlod_cells {
            rt.select(world, c, *d, &thresholds, f as u32)
                .unwrap_or_else(|e| fail(&format!("select: {e}")));
        }
    }
    rt.records().to_vec()
}

/// RED 臂:运行时合并注入(RXS-0364 L3)。
fn red_arm_runtime_merge() -> Result<(), String> {
    let rt = HlodRuntime::new();
    for op in ["merge", "simplify", "rebuild"] {
        match rt.request_runtime_merge(op, &[0, 1]) {
            Err(HlodError::RuntimeMergeForbidden { .. }) => {}
            other => return Err(format!("运行时合并 {op} 未拒(漏检): {other:?}")),
        }
    }
    // sabotage:正常选择/事件面不被误拒(能红证明)。
    let (world, mut ok_rt) = assemble(4);
    let cell = *ok_rt.resident().iter().next().ok_or("无驻留 cell")?;
    ok_rt
        .select(&world, cell, 100.0, &canonical_thresholds(), 0)
        .map_err(|e| format!("正常选择被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:层级序列扰动(同一视距序列产出必须确定;扰动即分叉)。
fn red_arm_level_perturb() -> Result<(), String> {
    let (world, mut rt) = assemble(8);
    let a = produce_selection_log(&world, &mut rt);
    let d_a = selection_log_digest(&a);
    // 扰动:视距序列第 8 帧 +100m ⇒ 层级序列 digest 必分叉。
    let mut rt2 = HlodRuntime::new();
    let mut prt =
        PartitionRuntime::new(world.clone(), canonical_budget()).map_err(|e| e.to_string())?;
    let path = canonical_camera_path(8);
    for (f, s) in path.iter().enumerate() {
        prt.tick(f as u32, std::slice::from_ref(s))
            .map_err(|e| e.to_string())?;
        rt2.apply_cell_events(&prt.drain_events())
            .map_err(|e| e.to_string())?;
    }
    let thresholds = canonical_thresholds();
    let hlod_cells: Vec<u32> = rt2
        .resident()
        .iter()
        .copied()
        .filter(|&c| world.cells[c as usize].hlod.is_some())
        .collect();
    // 扰动:第 0 帧视距 40m→840m(跨 Full→Culled 全档,选择结果必变 ⇒ digest
    // 必分叉;扰动不足跨档 = RED 臂自身失效,harness 拒记绿)。
    let mut perturbed_path = canonical_distance_path(32);
    perturbed_path[0] = 840.0;
    for (f, d) in perturbed_path.iter().enumerate() {
        for &c in &hlod_cells {
            rt2.select(&world, c, *d, &thresholds, f as u32)
                .map_err(|e| e.to_string())?;
        }
    }
    let d_b = selection_log_digest(rt2.records());
    if d_a == d_b {
        return Err("层级序列扰动未分叉( digest 对扰动不敏感 = RED 臂失效)".into());
    }
    Ok(())
}

/// RED 臂:实载产物 digest 篡改(双构建 hash 相等运行时核验臂)。
fn red_arm_digest_mismatch() -> Result<(), String> {
    let (world, mut rt) = assemble(4);
    let cell = rt
        .resident()
        .iter()
        .copied()
        .find(|&c| world.cells[c as usize].hlod.is_some())
        .ok_or("无驻留 HLOD cell")?;
    let meta = world.cells[cell as usize].hlod.ok_or("无 hlod 引用")?;
    rt.register_loaded_asset(cell, &meta, meta.digest)
        .map_err(|e| format!("正常 digest 登记失败: {e}"))?;
    let mut forged = meta.digest;
    forged[0] ^= 0x5a;
    match rt.register_loaded_asset(cell, &meta, forged) {
        Err(HlodError::DigestMismatch { .. }) => Ok(()),
        other => Err(format!("篡改 digest 未拒(漏检): {other:?}")),
    }
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "runtime-merge" => red_arm_runtime_merge(),
            "level-perturb" => red_arm_level_perturb(),
            "digest-mismatch" => red_arm_digest_mismatch(),
            other => fail(&format!(
                "未知 RED 臂: {other}(runtime-merge|level-perturb|digest-mismatch)"
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

    // ── 步骤 2:装配(事件总线接线) ──
    let (world, mut rt) = assemble(8);
    let resident_count = rt.resident().len();
    if resident_count == 0 {
        failures.push("事件总线接线后无驻留 cell".into());
    }

    // ── 步骤 3:层级序列双跑位级一致 + 互斥断言 ──
    let log_a = produce_selection_log(&world, &mut rt);
    let digest_a = selection_log_digest(&log_a);
    rt.clear_records();
    let log_b = produce_selection_log(&world, &mut rt);
    let digest_b = selection_log_digest(&log_b);
    let double_run_ok = digest_a == digest_b && log_a == log_b;
    if !double_run_ok {
        failures.push("层级序列双跑位级不一致".into());
    }
    let exclusive_ok = rt.assert_mutually_exclusive().is_ok();
    if !exclusive_ok {
        failures.push("互斥断言失败(同帧同 cell 出现多种内容)".into());
    }
    // 序列非平凡(含 Full / Hlod / Culled 至少两种)。
    let has_full = log_a.iter().any(|r| r.content == SelectedContent::Full);
    let has_proxy = log_a.iter().any(|r| {
        matches!(
            r.content,
            SelectedContent::Hlod { .. } | SelectedContent::Culled
        )
    });
    if !(has_full && has_proxy) {
        failures.push("层级序列退化(未发生真实切换)".into());
    }

    // ── 步骤 4:切换距离表 golden ──
    let thresholds: ScreenSizeThresholds = canonical_thresholds();
    let distances = thresholds.switch_distances_m(45.2548);
    let distances_json: Vec<String> = distances.iter().map(|d| format!("{d:.6}")).collect();

    // ── 步骤 5:RED 臂内联实测 ──
    let red_merge_ok = red_arm_runtime_merge().is_ok();
    let red_perturb_ok = red_arm_level_perturb().is_ok();
    let red_digest_ok = red_arm_digest_mismatch().is_ok();
    if !red_merge_ok {
        failures.push("运行时合并注入 RED 臂失效".into());
    }
    if !red_perturb_ok {
        failures.push("层级序列扰动 RED 臂失效".into());
    }
    if !red_digest_ok {
        failures.push("digest 篡改 RED 臂失效".into());
    }

    // ── 步骤 6:golden 带对照(freeze 自标定;PASS 逐字) ──
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
        let frozen = json_str(t, "selection_log_digest")
            .unwrap_or_else(|| fail("冻结带缺 selection_log_digest"));
        if frozen != hex(&digest_a) {
            golden_ok = false;
            failures.push("golden 漂移: selection_log_digest".into());
        }
        let frozen_dist = json_str(t, "switch_distances_m")
            .unwrap_or_else(|| fail("冻结带缺 switch_distances_m"));
        let expect_dist = format!("[{}]", distances_json.join(", "));
        if frozen_dist != expect_dist {
            golden_ok = false;
            failures.push("golden 漂移: switch_distances_m".into());
        }
    }

    // ── 步骤 7:freeze 落盘(measured 冻结 + provenance) ──
    if args.freeze {
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m111.hlod_runtime_band.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖;M111 运行时语义面 = 选择/切换/零合并断言)\"}},\n  \
             \"freeze_rule\": \"selection_log_digest = canonical 场景 8 帧事件总线接线后,对驻留 HLOD cell 按 32 帧 canonical 视距序列逐帧选择的层级序列 SHA-256(双跑位级一致后冻结,禁手写);switch_distances_m = canonical 阈值表对 45.2548m 包围球半径的闭式切换距离表 golden\",\n  \
             \"spec_anchor\": \"RXS-0364\",\n  \
             \"selection_log_digest\": \"{}\",\n  \
             \"selection_record_count\": {},\n  \
             \"switch_distances_m\": \"[{}]\",\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-m111-m112-m119-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            hex(&digest_a),
            log_a.len(),
            distances_json.join(", "),
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}", args.band);
    }

    // ── 步骤 8:evidence(rurix.g9m111.hlod_runtime.v1) ──
    let checks: [(&str, bool); 8] = [
        ("conformance_corpus_anchored", corpus_ok),
        ("event_bus_wired", resident_count > 0),
        ("selection_double_run_bit_equal", double_run_ok),
        ("mutually_exclusive", exclusive_ok),
        ("sequence_nontrivial", has_full && has_proxy),
        ("golden_frozen_equal", golden_ok || args.freeze),
        ("red_arm_runtime_merge", red_merge_ok),
        (
            "red_arm_level_perturb_and_digest",
            red_perturb_ok && red_digest_ok,
        ),
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
        "{{\n  \"schema\": \"rurix.g9m111.hlod_runtime.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m111_hlod_runtime\",\n  \"spec_anchor\": \"RXS-0364\",\n  \
         \"assertion_id\": \"g9.p1.m111.hlod_runtime\",\n  \"milestone\": \"M111\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;M111 语义面 = 选择/切换/零合并断言)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"golden\": {{\"selection_log_digest\": \"{}\", \"record_count\": {}, \"switch_distances_m\": \"[{}]\", \"freeze_band\": \"{}\"}},\n  \
         \"red_arms\": {{\"runtime_merge\": {}, \"level_perturb\": {}, \"digest_mismatch\": {}}},\n  \
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
        hex(&digest_a),
        log_a.len(),
        distances_json.join(", "),
        json_escape(&args.band.display().to_string()),
        red_merge_ok,
        red_perturb_ok,
        red_digest_ok,
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
            "{TAG}: PASS screen-size 互斥切换 + 零合并断言 + cell 事件总线接线(host 确定性面)"
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
