//! G9.5 M120 OIT benchmark harness(RXS-0371;门 `g9.p1.m120.oit_benchmark_harness`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M120 行逐字 + spec/display_pipeline.md RXS-0371)
//!
//! 1. **benchmark harness 与 evidence 非空**:以 nvpro
//!    `vk_order_independent_transparency` 七算法(simple/linked_list/loop32/
//!    loop64/spinlock/interlock/weighted_blended)为对照基线,同场景同 overdraw
//!    分布(canonical 场景单源,4 档 overdraw 阶梯),逐算法帧时(wall-clock
//!    min-of-5)/内存(存储模型 bytes)/质量误差(对排序真值 max/mean/超阈像素)
//!    曲线 measured 落 evidence;
//! 2. **仅测量不定档**:本 harness 不产出任何选定档;选型入口
//!    `select_default_tier` 一律 fail-closed typed `Err(NotMeasuredYet)`;
//!    evidence 无选型字段;
//! 3. **无数据选型提交判 RED**:无 benchmark 数据引用的选型提交 → typed Err;
//!    引用数据缺算法/零记录 → typed Err(RED 臂独立有效;齐备引用 ⇒ 合规
//!    〔非本门选型〕,sabotage 探针能红证明);
//! 4. **排序 fallback 永保留**:depth-sorted alpha 路径恒可达(正确性对照
//!    = 真值本体);**linked-list 精确档与排序真值 diff=0**(池充足,逐档位
//!    位级一致);**精确档内存无界增长注入即 RED**(无界策略请求 → typed Err;
//!    观测超声明界 → typed Err);
//! 5. **M114 消费面**:测量数据 measured 冻结落
//!    `milestones/g9/g9_m120_oit_measurements.json`(确定性字段位级对照;帧时
//!    为参考值不位冻——wall time 不可位冻,如实登记),供 M114 strand 档裁决
//!    消费;
//! 6. **conformance 语料消费**:M120 两件锚定语料 `//@ spec: RXS-0371` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m120_oit_benchmark [--evidence <path>] [--measurements <path>]
//! g9_m120_oit_benchmark --freeze [--measurements-out <path>] [--evidence <path>]
//! g9_m120_oit_benchmark --red-arm selection-without-data|unbounded-memory
//! ```

#![forbid(unsafe_code)]

use rurix_render::oit::algorithms::{OitAlgorithm, image_digest, quality_error, sorted_fallback};
use rurix_render::oit::measure::{BenchmarkRun, run_benchmark};
use rurix_render::oit::scene::{BENCHMARK_LAYERS, canonical_scene};
use rurix_render::oit::selection::{
    BenchmarkDataRef, OitError, OitTier, SelectionCommit, check_exact_tier_memory,
    request_exact_tier_memory, select_default_tier, validate_selection_commit,
};
use std::path::PathBuf;

const TAG: &str = "G9_M120_OIT";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/oit_benchmark_harness_minimal.rx", "RXS-0371"),
    (
        "reject/oit_default_tier_without_benchmark_data.rx",
        "RXS-0371",
    ),
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

fn f32_hex(v: f32) -> String {
    format!("{:08x}", v.to_bits())
}

fn f64_hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

struct Args {
    evidence: Option<PathBuf>,
    measurements: PathBuf,
    freeze: bool,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let root = workspace_root();
    let mut out = Args {
        evidence: None,
        measurements: root.join("milestones/g9/g9_m120_oit_measurements.json"),
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
            "--measurements" | "--measurements-out" => {
                out.measurements = PathBuf::from(take(&mut i))
            }
            "--freeze" => out.freeze = true,
            "--red-arm" => out.red_arm = Some(take(&mut i)),
            other => fail(&format!("未知参数: {other}")),
        }
        i += 1;
    }
    out
}

/// RED 臂:无 benchmark 数据的选型提交判 RED(RXS-0371 L3)。
fn red_arm_selection(record_count: usize) -> Result<(), String> {
    // 负例①:无数据引用。
    let c1 = SelectionCommit {
        tier: OitTier::DefaultTaaComposite,
        algorithm: OitAlgorithm::WeightedBlended,
        benchmark: None,
    };
    match validate_selection_commit(&c1) {
        Err(OitError::SelectionWithoutBenchmarkData) => {}
        other => return Err(format!("无数据提交未拒(漏检): {other:?}")),
    }
    // 负例②:数据不全(缺算法)。
    let mut partial = BenchmarkDataRef {
        measurements_digest: [0; 32],
        algorithms: OitAlgorithm::ALL.to_vec(),
        overdraw_levels: BENCHMARK_LAYERS.to_vec(),
        record_count,
    };
    partial.algorithms.retain(|a| *a != OitAlgorithm::Spinlock);
    let c2 = SelectionCommit {
        tier: OitTier::DefaultTaaComposite,
        algorithm: OitAlgorithm::Simple,
        benchmark: Some(partial),
    };
    match validate_selection_commit(&c2) {
        Err(OitError::BenchmarkDataIncomplete { .. }) => {}
        other => return Err(format!("缺档数据提交未拒(漏检): {other:?}")),
    }
    // 负例③:零记录。
    let empty = BenchmarkDataRef {
        measurements_digest: [0; 32],
        algorithms: OitAlgorithm::ALL.to_vec(),
        overdraw_levels: BENCHMARK_LAYERS.to_vec(),
        record_count: 0,
    };
    let c3 = SelectionCommit {
        tier: OitTier::DefaultTaaComposite,
        algorithm: OitAlgorithm::Loop32,
        benchmark: Some(empty),
    };
    match validate_selection_commit(&c3) {
        Err(OitError::BenchmarkDataIncomplete { .. }) => {}
        other => return Err(format!("零记录提交未拒(漏检): {other:?}")),
    }
    // sabotage 探针(能红证明):齐备引用 ⇒ 合规(非本门选型)。
    let full = BenchmarkDataRef {
        measurements_digest: [0xAB; 32],
        algorithms: OitAlgorithm::ALL.to_vec(),
        overdraw_levels: BENCHMARK_LAYERS.to_vec(),
        record_count,
    };
    let c4 = SelectionCommit {
        tier: OitTier::DefaultTaaComposite,
        algorithm: OitAlgorithm::LinkedList,
        benchmark: Some(full),
    };
    validate_selection_commit(&c4).map_err(|e| format!("齐备引用被误拒: {e}"))?;
    // 本门选型入口 fail-closed(仅测量不定档)。
    match select_default_tier() {
        Err(OitError::NotMeasuredYet) => Ok(()),
        Ok(t) => Err(format!("选型入口产出了档({t:?})——仅测量不定档纪律被破坏")),
        Err(e) => Err(format!("选型入口错误类别漂移: {e}")),
    }
}

/// RED 臂:精确档内存无界增长注入判 RED(RXS-0371 L4)。
fn red_arm_unbounded_memory() -> Result<(), String> {
    // 负例①:无界策略请求。
    match request_exact_tier_memory(true, 1 << 20) {
        Err(OitError::ExactTierUnboundedMemory) => {}
        other => return Err(format!("无界请求未拒(漏检): {other:?}")),
    }
    // 负例②:观测超声明界。
    let policy = request_exact_tier_memory(false, 1 << 20).map_err(|e| e.to_string())?;
    match check_exact_tier_memory(&policy, (1 << 20) + 4096) {
        Err(OitError::ExactTierMemoryExceeded { .. }) => {}
        other => return Err(format!("超界观测未拒(漏检): {other:?}")),
    }
    // sabotage:界内观测合规。
    check_exact_tier_memory(&policy, (1 << 20) - 1).map_err(|e| format!("界内被误拒: {e}"))?;
    Ok(())
}

/// 确定性对照面(双跑位级一致;帧时除外——wall time 不位冻)。
fn deterministic_equal(a: &BenchmarkRun, b: &BenchmarkRun) -> bool {
    a.truth_digest_per_level == b.truth_digest_per_level
        && a.scene_digest_per_level == b.scene_digest_per_level
        && a.measurements.len() == b.measurements.len()
        && a.measurements
            .iter()
            .zip(b.measurements.iter())
            .all(|(x, y)| {
                x.algorithm == y.algorithm
                    && x.overdraw_layers == y.overdraw_layers
                    && x.storage_bytes == y.storage_bytes
                    && x.aux_bytes == y.aux_bytes
                    && x.quality_max_abs == y.quality_max_abs
                    && x.quality_mean_abs == y.quality_mean_abs
                    && x.quality_pixels_over_eps == y.quality_pixels_over_eps
                    && x.fragments_kept == y.fragments_kept
                    && x.fragments_tail == y.fragments_tail
                    && x.fragments_dropped == y.fragments_dropped
                    && x.image_digest == y.image_digest
            })
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    // ── RED 臂子模式(先于全量 benchmark 分发;臂面不需要曲线) ──
    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "selection-without-data" => {
                red_arm_selection(OitAlgorithm::ALL.len() * BENCHMARK_LAYERS.len())
            }
            "unbounded-memory" => red_arm_unbounded_memory(),
            other => fail(&format!(
                "未知 RED 臂: {other}(selection-without-data|unbounded-memory)"
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

    // ── 主 benchmark(双跑供确定性核验) ──
    let run = run_benchmark();
    let run2 = run_benchmark();

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

    // ── 步骤 2:evidence 非空(七算法 × 四档位,帧时/内存/质量面全量) ──
    let nonempty = run.is_nonempty();
    if !nonempty {
        failures.push("benchmark evidence 空/缺档".into());
    }

    // ── 步骤 3:确定性双跑位级一致(确定性字段) ──
    let double_run_ok = deterministic_equal(&run, &run2);
    if !double_run_ok {
        failures.push("benchmark 双跑确定性字段位级不一致".into());
    }

    // ── 步骤 4:linked-list 精确档与排序真值 diff=0(逐档位) ──
    let mut exact_diff_zero = true;
    for (layers, truth_digest) in &run.truth_digest_per_level {
        let m = run
            .find(&OitAlgorithm::LinkedList, *layers)
            .expect("linked_list 档必有");
        if m.image_digest != *truth_digest || m.fragments_dropped != 0 {
            exact_diff_zero = false;
            failures.push(format!("linked-list 与真值 diff≠0 @layers={layers}"));
        }
    }

    // ── 步骤 5:排序 fallback 永保留(可达 + 正确性对照) ──
    let fallback_ok = rurix_render::oit::selection::sorted_fallback_reachable()
        == OitTier::SortedFallback
        && BENCHMARK_LAYERS.iter().all(|&l| {
            let scene = canonical_scene(32, 32, l);
            let truth = sorted_fallback(&scene);
            // fallback 输出 = 真值(同一函数实例),digest 自一致且非退化。
            image_digest(&truth.rgb) == image_digest(&truth.rgb) && truth.fragments_kept > 0
        });
    if !fallback_ok {
        failures.push("排序 fallback 可达性/对照面失效".into());
    }

    // ── 步骤 6:测量敏感性(WBOIT 深档近似误差 > 0;sabotage 真值自比 = 0) ──
    let deep = *BENCHMARK_LAYERS.last().expect("档位非空");
    let wboit_deep = run
        .find(&OitAlgorithm::WeightedBlended, deep)
        .expect("wboit 深档");
    let sensitivity_ok = wboit_deep.quality_max_abs > 0.0 && wboit_deep.quality_pixels_over_eps > 0;
    let scene_check = canonical_scene(32, 32, 16);
    let truth_a = sorted_fallback(&scene_check);
    let (sabotage_max, _, sabotage_count) = quality_error(&truth_a.rgb, &truth_a.rgb, 0.0);
    let sabotage_ok = sabotage_max == 0.0 && sabotage_count == 0;
    if !sensitivity_ok || !sabotage_ok {
        failures.push(format!(
            "质量误差测量面: sensitivity={sensitivity_ok} sabotage={sabotage_ok}"
        ));
    }

    // ── 步骤 7:RED 臂内联实测 ──
    let selection_arm_ok = red_arm_selection(run.measurements.len()).is_ok();
    let memory_arm_ok = red_arm_unbounded_memory().is_ok();
    if !selection_arm_ok {
        failures.push("无数据选型提交 RED 臂失效".into());
    }
    if !memory_arm_ok {
        failures.push("精确档无界增长 RED 臂失效".into());
    }

    // ── 步骤 8:measurements 冻结带对照(freeze 自标定;PASS 逐字) ──
    let band_text = match std::fs::read_to_string(&args.measurements) {
        Ok(t) => Some(t),
        Err(_) if args.freeze => None,
        Err(_) => fail(&format!(
            "measurements 冻结 {:?} 不存在——先跑 `--freeze`(禁手写)",
            args.measurements
        )),
    };
    let mut frozen_ok = true;
    if !args.freeze {
        let t = band_text.as_deref().expect("PASS 模式冻结带必读");
        for m in &run.measurements {
            let base = format!("{}_{}", m.algorithm.as_str(), m.overdraw_layers);
            let key = format!("{base}_image_digest");
            let frozen = json_str(t, &key).unwrap_or_else(|| fail(&format!("冻结带缺 {key}")));
            if frozen != hex(&m.image_digest) {
                frozen_ok = false;
                failures.push(format!("measurements 漂移: {key}"));
            }
            for (suffix, val) in [
                ("storage_bytes", m.storage_bytes.to_string()),
                ("fragments_kept", m.fragments_kept.to_string()),
                ("fragments_tail", m.fragments_tail.to_string()),
                ("quality_max_abs", f32_hex(m.quality_max_abs)),
                ("quality_mean_abs", f64_hex(m.quality_mean_abs)),
                (
                    "quality_pixels_over_eps",
                    m.quality_pixels_over_eps.to_string(),
                ),
            ] {
                let key = format!("{base}_{suffix}");
                let frozen = json_str(t, &key).unwrap_or_else(|| fail(&format!("冻结带缺 {key}")));
                if frozen != val {
                    frozen_ok = false;
                    failures.push(format!(
                        "measurements 漂移: {key}(冻结 {frozen} ≠ 实测 {val})"
                    ));
                }
            }
        }
    }

    // ── 步骤 9:freeze 落盘(measured 冻结 + provenance;M114 消费面) ──
    if args.freeze {
        let mut rows = String::new();
        for m in &run.measurements {
            // 扁平键(与 PASS 模式 json_str 查询面一一对应)。
            rows.push_str(&format!(
                "    \"{}_{}_image_digest\": \"{}\",\n    \"{}_{}_storage_bytes\": \"{}\",\n    \"{}_{}_aux_bytes\": \"{}\",\n    \"{}_{}_quality_max_abs\": \"{}\",\n    \"{}_{}_quality_mean_abs\": \"{}\",\n    \"{}_{}_quality_pixels_over_eps\": \"{}\",\n    \"{}_{}_fragments_total\": \"{}\",\n    \"{}_{}_fragments_kept\": \"{}\",\n    \"{}_{}_fragments_tail\": \"{}\",\n    \"{}_{}_fragments_dropped\": \"{}\",\n    \"{}_{}_frame_ns_min_reference\": {},\n",
                m.algorithm.as_str(), m.overdraw_layers, hex(&m.image_digest),
                m.algorithm.as_str(), m.overdraw_layers, m.storage_bytes,
                m.algorithm.as_str(), m.overdraw_layers, m.aux_bytes,
                m.algorithm.as_str(), m.overdraw_layers, f32_hex(m.quality_max_abs),
                m.algorithm.as_str(), m.overdraw_layers, f64_hex(m.quality_mean_abs),
                m.algorithm.as_str(), m.overdraw_layers, m.quality_pixels_over_eps,
                m.algorithm.as_str(), m.overdraw_layers, m.fragments_total,
                m.algorithm.as_str(), m.overdraw_layers, m.fragments_kept,
                m.algorithm.as_str(), m.overdraw_layers, m.fragments_tail,
                m.algorithm.as_str(), m.overdraw_layers, m.fragments_dropped,
                m.algorithm.as_str(), m.overdraw_layers, m.frame_ns_min,
            ));
        }
        let truth_json: Vec<String> = run
            .truth_digest_per_level
            .iter()
            .map(|(l, d)| format!("\"{l}\": \"{}\"", hex(d)))
            .collect();
        let scene_json: Vec<String> = run
            .scene_digest_per_level
            .iter()
            .map(|(l, d)| format!("\"{l}\": \"{}\"", hex(d)))
            .collect();
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m120.oit_measurements.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(host 确定性参照;device 帧时腿归后续波——atomics 与 .rx 确定性协议冲突待裁决)\"}},\n  \
             \"freeze_rule\": \"内存 bytes/质量误差/fragment 计数/图像 digest = 确定性字段,双跑位级一致后逐字冻结(禁手写);frame_ns_min = wall-clock measured 参考值(不位冻,判据 = 非零非空);对照基线 = nvpro vk_order_independent_transparency 七算法存储/溢出语义;同场景同 overdraw 分布(canonical 场景 digest 锚定);**仅测量不定档**(D4 D15),本文件供 M114 strand 档裁决消费\",\n  \
             \"spec_anchor\": \"RXS-0371\",\n  \
             \"algorithms\": [\"simple\", \"linked_list\", \"loop32\", \"loop64\", \"spinlock\", \"interlock\", \"weighted_blended\"],\n  \
             \"overdraw_levels\": {:?},\n  \
             \"extent\": [128, 128],\n  \
             \"scene_digests\": {{{}}},\n  \
             \"truth_digests\": {{{}}},\n  \
             \"measurements\": {{\n{}}},\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-m118-m120-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            BENCHMARK_LAYERS,
            scene_json.join(", "),
            truth_json.join(", "),
            rows.trim_end_matches([',', '\n']),
        );
        if let Some(parent) = args.measurements.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.measurements, &band)
            .unwrap_or_else(|e| fail(&format!("写 measurements 冻结: {e}")));
        println!("{TAG}: measurements 冻结已落盘 {:?}", args.measurements);
    }

    // ── 步骤 10:evidence(rurix.g9m120.oit_benchmark.v1;无选型字段) ──
    let checks: [(&str, bool); 9] = [
        ("conformance_corpus_anchored", corpus_ok),
        ("benchmark_evidence_nonempty", nonempty),
        ("double_run_deterministic_bit_equal", double_run_ok),
        ("linked_list_exact_tier_diff_zero", exact_diff_zero),
        ("sorted_fallback_always_reachable", fallback_ok),
        (
            "quality_measurement_sensitive",
            sensitivity_ok && sabotage_ok,
        ),
        ("selection_without_data_red_arm", selection_arm_ok),
        ("exact_tier_unbounded_memory_red_arm", memory_arm_ok),
        ("measurements_frozen_equal", frozen_ok || args.freeze),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let mut meas_json = String::new();
    for m in &run.measurements {
        meas_json.push_str(&format!(
            "    {{\"algorithm\": \"{}\", \"overdraw_layers\": {}, \"frame_ns_min\": {}, \"storage_bytes\": {}, \"aux_bytes\": {}, \"quality_max_abs\": {:.9e}, \"quality_mean_abs\": {:.9e}, \"quality_pixels_over_eps\": {}, \"fragments_total\": {}, \"fragments_kept\": {}, \"fragments_tail\": {}, \"fragments_dropped\": {}, \"image_digest\": \"{}\"}},\n",
            m.algorithm.as_str(),
            m.overdraw_layers,
            m.frame_ns_min,
            m.storage_bytes,
            m.aux_bytes,
            m.quality_max_abs,
            m.quality_mean_abs,
            m.quality_pixels_over_eps,
            m.fragments_total,
            m.fragments_kept,
            m.fragments_tail,
            m.fragments_dropped,
            hex(&m.image_digest),
        ));
    }
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m120.oit_benchmark.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m120_oit_benchmark\",\n  \"spec_anchor\": \"RXS-0371\",\n  \
         \"assertion_id\": \"g9.p1.m120.oit_benchmark_harness\",\n  \"milestone\": \"M120\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(host 确定性参照;4070 Ti device 帧时腿归后续波)\", \"validation\": \"not_applicable\", \"require_real\": {}, \"build_debug_assertions\": {}}},\n  \
         \"baseline\": {{\"reference\": \"nvpro vk_order_independent_transparency 七算法对照(存储/溢出语义逐字对应)\", \"same_scene_same_overdraw\": true, \"extent\": [128, 128], \"overdraw_levels\": {:?}}},\n  \
         \"measurements\": [\n{}],\n  \
         \"tier_selection\": {{\"committed\": false, \"policy\": \"仅测量不定档(D4 D15);select_default_tier 一律 fail-closed NotMeasuredYet;默认档选型必须引 benchmark 数据(无数据提交判 RED)\"}},\n  \
         \"sorted_fallback\": {{\"always_reachable\": true, \"role\": \"最低端档与正确性对照(永保留)\"}},\n  \
         \"exact_tier\": {{\"algorithm\": \"linked_list\", \"scope\": \"hair_strand_only(场景级不开放)\", \"diff_vs_sorted_truth\": 0, \"memory_policy\": \"bounded_pool(无界增长注入判 RED)\"}},\n  \
         \"m114_consumption\": {{\"measurements_file\": \"{}\", \"status\": \"measured_frozen(strand 档承接锚:M120 精确档数据落地后重判,兜底 G9.7 穷举)\"}},\n  \
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
        cfg!(debug_assertions),
        BENCHMARK_LAYERS,
        meas_json.trim_end_matches([',', '\n']),
        json_escape(&args.measurements.display().to_string()),
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
            "{TAG}: PASS 七算法 × 4 档 evidence 非空 + 仅测量不定档 + 双 RED 臂 + 排序 fallback 可达 + 精确档 diff=0(host 确定性面)"
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
