//! G9.6 M126 Rapier 深造对标基准 A/B harness(RXS-0378;门
//! `g9.p1.m126.rapier_benchmark_ab`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M126 行逐字 + spec/physics.md RXS-0378)
//!
//! 1. **同场景同输入同 determinism 画像 A/B 夹具**:Jolt vs Rapier 同一
//!    canonical 大堆叠场景、同一输入 journal(input digest 两臂逐位相等)、
//!    同一 determinism 画像(固定 dt 锁死/单线程 declared/睡眠策略钉值/零
//!    IO)——三面测量:逐 tick world 状态摘要链 digest + 接触事件计数 +
//!    求解耗时(wall-clock measured_local 真实采样,禁 estimated)。
//! 2. **determinism 画像**:各自后端同后端双跑位级一致(各自确定性成立,
//!    硬断言);跨后端差异如实记录(跨 solver 不承诺逐位,只作不变量/容差
//!    对拍,RFC-0021 §7 备选 D;差异非判据,画像记录)。
//! 3. **measured 报告(evidence 非空)**:落
//!    `milestones/g9/g9_m126_rapier_benchmark.json`(measured + provenance:
//!    后端版本/feature/UTC/输入 digest)。
//! 4. **基准不作 replay oracle**(RED 臂独立有效):以基准输出冒充
//!    capture/replay 逐位对拍 oracle ⇒ fail-closed typed Err(夹具内自检)。
//! 5. **RD-044 字面不变**:「快路径被真实 workload 采用时」0-byte——裁决
//!    按 measured 优势 + 不变量/容差对拍登记(申请改判 or 维持 no-go);
//!    无 measured 数据的判档申请 fail-closed(RED 臂独立有效);本门只产
//!    基准报告,不升格深造、不作验收依赖与生产默认。
//! 6. **glam 迁移兼容留档**:Rapier 0.32+ glam 化 API 冲击评估与兼容层设
//!    计留档,不承诺 bitwise 不变。
//!
//! ## 三态
//!
//! host 纯确定性面(`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,validation
//! 不适用);feature `rapier`(默认 off 纪律维持,RFC-0017 §4.D1)未编译 ⇒
//! RapierBackendNotCompiled fail-closed(不静默单臂充绿);判据不符 / RED
//! 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m126_rapier_benchmark [--evidence <path>] [--report <path>]
//! g9_m126_rapier_benchmark --red-arm replay-oracle|rd044-without-measured
//! ```

#![forbid(unsafe_code)]

const TAG: &str = "G9_M126_RAPIER_BENCH";

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

#[cfg(all(feature = "physics-capture", feature = "rapier"))]
mod imp {
    use std::path::PathBuf;

    use rurix_physics::BackendKind;
    use rurix_physics::benchmark::{
        BenchmarkError, BenchmarkReport, CanonicalStackSpec, GLAM_MIGRATION_NOTE,
        RD044_CONDITION_LITERAL, assert_double_run_bitwise, compare_as_replay_oracle,
        cross_solver_deviation, rd044_verdict, validate_rd044_application,
    };

    use super::{TAG, fail};

    const CORPUS_RX: &[(&str, &str)] = &[
        ("accept/rapier_benchmark_ab_fixture_minimal.rx", "RXS-0378"),
        ("reject/rapier_benchmark_as_replay_oracle.rx", "RXS-0378"),
    ];

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

    struct Args {
        evidence: Option<PathBuf>,
        report: PathBuf,
        red_arm: Option<String>,
    }

    fn parse_args() -> Args {
        let root = workspace_root();
        let mut out = Args {
            evidence: None,
            report: root.join("milestones/g9/g9_m126_rapier_benchmark.json"),
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
                "--report" => out.report = PathBuf::from(take(&mut i)),
                "--red-arm" => out.red_arm = Some(take(&mut i)),
                other => fail(&format!("未知参数: {other}")),
            }
            i += 1;
        }
        out
    }

    /// RED 臂:基准冒充 replay oracle(双向注入)——一律 fail-closed typed Err。
    fn red_arm_replay_oracle(report: &BenchmarkReport) -> Result<(), String> {
        match compare_as_replay_oracle(&report.jolt, &report.rapier) {
            Err(BenchmarkError::ReplayOracleUsurpation(_)) => {}
            other => return Err(format!("jolt→rapier 冒充未拒(漏检): {other:?}")),
        }
        match compare_as_replay_oracle(&report.rapier, &report.jolt) {
            Err(BenchmarkError::ReplayOracleUsurpation(_)) => {}
            other => return Err(format!("rapier→jolt 冒充未拒(漏检): {other:?}")),
        }
        Ok(())
    }

    /// RED 臂:无 measured 数据的 RD-044 深造判档申请——fail-closed。
    fn red_arm_rd044_without_measured() -> Result<(), String> {
        match validate_rd044_application(false) {
            Err(BenchmarkError::MeasuredDataMissing(_)) => {}
            other => return Err(format!("无数据申请未拒(漏检): {other:?}")),
        }
        validate_rd044_application(true).map_err(|e| format!("有数据合规面被误拒: {e}"))?;
        Ok(())
    }

    /// A/B 夹具真跑(两臂各自双跑位级断言 + 同输入断言 + 偏差统计 + 裁决)。
    fn run_ab() -> Result<BenchmarkReport, String> {
        let spec = CanonicalStackSpec::default();
        spec.validate().map_err(|e| e.to_string())?;
        let jolt =
            assert_double_run_bitwise(BackendKind::Jolt, &spec).map_err(|e| e.to_string())?;
        let rapier =
            assert_double_run_bitwise(BackendKind::Rapier, &spec).map_err(|e| e.to_string())?;
        if jolt.input_digest != rapier.input_digest {
            return Err("两臂输入 digest 不一致(同输入断言破裂)".into());
        }
        let deviation = cross_solver_deviation(&jolt, &rapier).map_err(|e| e.to_string())?;
        let verdict = rd044_verdict(&jolt, &rapier, &deviation);
        Ok(BenchmarkReport {
            spec,
            jolt,
            rapier,
            deviation,
            verdict,
        })
    }

    pub fn main() {
        let args = parse_args();
        let root = workspace_root();

        // ── RED 臂子模式 ──
        if let Some(arm) = &args.red_arm {
            let r = match arm.as_str() {
                "replay-oracle" => match run_ab() {
                    Ok(rep) => red_arm_replay_oracle(&rep),
                    Err(e) => Err(e),
                },
                "rd044-without-measured" => red_arm_rd044_without_measured(),
                other => fail(&format!(
                    "未知 RED 臂: {other}(replay-oracle|rd044-without-measured)"
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
        let corpus_dir = root.join("conformance/physics");
        let mut corpus_ok = true;
        let mut anchors_json: Vec<String> = Vec::new();
        for (rel, expect) in CORPUS_RX {
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
                if ok { *expect } else { "MISSING" }
            ));
        }

        // ── 步骤 2:A/B 夹具真跑(双臂各双跑位级断言 + 同输入同画像断言) ──
        let report = match run_ab() {
            Ok(r) => r,
            Err(e) => fail(&format!("A/B 夹具: {e}")),
        };
        // 到达此处 = 两臂各自双跑位级一致(assert_double_run_bitwise 内部
        // fail-closed)+ 输入 digest 逐位一致。
        let same_input = report.jolt.input_digest == report.rapier.input_digest;
        // 同 determinism 画像(除后端外 desc 全字段逐位)。
        let dj = report.spec.world_desc(BackendKind::Jolt);
        let dr = report.spec.world_desc(BackendKind::Rapier);
        let same_profile = dj.gravity == dr.gravity
            && dj.layer_count == dr.layer_count
            && dj.max_bodies == dr.max_bodies
            && dj.job_threads == dr.job_threads
            && dj.dt_fixed == dr.dt_fixed
            && dj.contact_capacity == dr.contact_capacity;
        if !same_input || !same_profile {
            failures.push("同场景同输入同 determinism 画像断言破裂".into());
        }

        // ── 步骤 3:跨 solver 偏差统计(不变量/容差对拍;差异如实记录非判据) ──
        let deviation_recorded = report.jolt.contact_events_total > 0
            && report.rapier.contact_events_total > 0
            && report.jolt.step_ns_median() > 0
            && report.rapier.step_ns_median() > 0;
        if !deviation_recorded {
            failures.push("measured 三面(状态链/接触计数/耗时)有空面".into());
        }
        let invariant_ok = report.deviation.rest_above_ground_invariant;
        if !invariant_ok {
            failures.push("物理不变量破坏(末态穿地)".into());
        }
        // 跨 solver 逐位相等非判据——如实记录(预期 false = 分叉画像)。
        let cross_bitwise_equal = report.deviation.world_chain_bitwise_equal;

        // ── 步骤 4:RED 臂内联实测(两臂独立) ──
        let oracle_red = red_arm_replay_oracle(&report).is_ok();
        let rd044_red = red_arm_rd044_without_measured().is_ok();
        if !oracle_red {
            failures.push("基准冒充 replay oracle RED 臂失效".into());
        }
        if !rd044_red {
            failures.push("无 measured 判档申请 RED 臂失效".into());
        }

        // ── 步骤 5:RD-044 字面不变核验(deferred.json 消费写明) ──
        let deferred_text = std::fs::read_to_string(root.join("registry/deferred.json"))
            .unwrap_or_else(|e| fail(&format!("读 deferred.json: {e}")));
        let rd044_literal_ok = deferred_text.contains(RD044_CONDITION_LITERAL);
        if !rd044_literal_ok {
            failures.push("RD-044 触发条件字面漂移(deferred.json 消费面)".into());
        }
        let verdict = report.verdict.canonical_name();

        // ── 步骤 6:measured 报告落盘(measured + provenance;不升格深造) ──
        let base_commit =
            std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
        let report_json = format!(
            "{{\n  \"schema\": \"rurix.g9m126.rapier_benchmark.report.v1\",\n  \"generated_at_utc\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"provenance\": {{\"generator\": \"cargo build -p rurix-physics --features 'physics-capture,rapier' --bin g9_m126_rapier_benchmark && g9_m126_rapier_benchmark\", \"host\": \"{} {}\", \"backends\": {{\"jolt\": \"Jolt 5.3.0 / JoltC 2982004387a9e36ca89525a87d983709d3666da7(feature jolt 默认 on)\", \"rapier\": \"rapier3d =0.33.0 pin(default-features=false + dim3/f32/std;parallel/simd-stable/serde-serialize/enhanced-determinism 维持 off,feature rapier 默认 off)\"}}, \"evidence_level\": \"measured_local(真实采样,禁 estimated)\"}},\n  \"scenario\": {{\"kind\": \"canonical 大堆叠(静态地面 + {} 层动态箱,半长 {} m,层缝 {} m)\", \"ticks\": {}, \"determinism_profile\": {{\"dt_fixed\": \"1/60 锁死\", \"job_threads\": 1, \"job_system\": \"ThreadPool(1)/rapier 单线程标量(job_threads 为 Jolt 专用,Rapier 臂忽略——rapier.rs 诚实登记)\", \"sleep_policy\": \"钉值(can_sleep=true 两臂同参)\", \"io\": \"零 IO\", \"fp_env\": \"无浮点环境变量依赖\"}}, \"input_digest\": \"{}\", \"same_scene_same_input\": {}}},\n  \"arms\": {{\n    \"jolt\": {{\"world_digest\": \"{}\", \"contact_events_total\": {}, \"step_ns_median\": {}, \"step_ns_min\": {}, \"step_ns_total\": {}, \"final_state_digest\": \"{}\", \"double_run_bitwise\": true}},\n    \"rapier\": {{\"world_digest\": \"{}\", \"contact_events_total\": {}, \"step_ns_median\": {}, \"step_ns_min\": {}, \"step_ns_total\": {}, \"final_state_digest\": \"{}\", \"double_run_bitwise\": true}}\n  }},\n  \"cross_solver_deviation\": {{\"world_chain_bitwise_equal\": {}, \"max_translation_abs_diff\": {:.9e}, \"mean_translation_abs_diff\": {:.9e}, \"max_linvel_abs_diff\": {:.9e}, \"contact_events_abs_diff\": {}, \"rest_above_ground_invariant\": {}, \"within_tolerance_0.05m\": {}, \"note\": \"跨 solver 不承诺逐位(RFC-0021 §7 备选 D),只作不变量/容差对拍;差异如实记录,非判据\"}},\n  \"rd044\": {{\"condition_literal\": \"{}\", \"condition_literal_unchanged\": {}, \"verdict\": \"{}\", \"verdict_basis\": \"rapier.step_ns_median({}) < jolt.step_ns_median({}) ⇒ measured 优势 = {};不变量/容差对拍 = {}/{}\", \"scope\": \"本门只产基准报告,不升格深造、不作验收依赖与生产默认;RD-044 字面不变\"}},\n  \"glam_migration\": \"{}\",\n  \"benchmark_not_replay_oracle\": \"replay 对拍唯一权威 = 同 solver 同版本 capture/replay 逐 tick hash(RFC-0021 §4.A1);基准输出冒充 replay oracle = RED(夹具内自检已实测)\"\n}}\n",
            utc_now(),
            json_escape(&base_commit),
            std::env::consts::OS,
            std::env::consts::ARCH,
            report.spec.layers,
            report.spec.box_half,
            report.spec.layer_gap,
            report.spec.ticks,
            report.jolt.input_digest,
            same_input && same_profile,
            report.jolt.world_digest,
            report.jolt.contact_events_total,
            report.jolt.step_ns_median(),
            report.jolt.step_ns_min(),
            report.jolt.step_ns_total(),
            report.jolt.final_state_digest,
            report.rapier.world_digest,
            report.rapier.contact_events_total,
            report.rapier.step_ns_median(),
            report.rapier.step_ns_min(),
            report.rapier.step_ns_total(),
            report.rapier.final_state_digest,
            cross_bitwise_equal,
            report.deviation.max_translation_abs_diff,
            report.deviation.mean_translation_abs_diff,
            report.deviation.max_linvel_abs_diff,
            report.deviation.contact_events_abs_diff,
            report.deviation.rest_above_ground_invariant,
            report.deviation.within_tolerance,
            json_escape(RD044_CONDITION_LITERAL),
            rd044_literal_ok,
            verdict,
            report.rapier.step_ns_median(),
            report.jolt.step_ns_median(),
            report.rapier.step_ns_median() < report.jolt.step_ns_median(),
            report.deviation.rest_above_ground_invariant,
            report.deviation.within_tolerance,
            json_escape(GLAM_MIGRATION_NOTE),
        );
        if let Some(parent) = args.report.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.report, &report_json)
            .unwrap_or_else(|e| fail(&format!("写 measured 报告: {e}")));
        println!("{TAG}: measured 报告已落盘 {:?}", args.report);

        // ── 步骤 7:evidence(rurix.g9m126.rapier_benchmark.v1) ──
        let checks: [(&str, bool); 10] = [
            ("conformance_corpus_anchored", corpus_ok),
            (
                "same_scene_same_input_same_profile",
                same_input && same_profile,
            ),
            ("jolt_double_run_bitwise", true),
            ("rapier_double_run_bitwise", true),
            ("cross_solver_deviation_recorded", deviation_recorded),
            ("rest_above_ground_invariant", invariant_ok),
            ("benchmark_as_replay_oracle_red", oracle_red),
            ("rd044_application_without_measured_red", rd044_red),
            ("rd044_condition_literal_unchanged", rd044_literal_ok),
            ("measured_report_written", args.report.exists()),
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
        let json = format!(
            "{{\n  \"schema\": \"rurix.g9m126.rapier_benchmark.v1\",\n  \"schema_version\": 1,\n  \"subject\": \"g9_m126_rapier_benchmark\",\n  \"spec_anchor\": \"RXS-0378\",\n  \"assertion_id\": \"g9.p1.m126.rapier_benchmark_ab\",\n  \"milestone\": \"M126\",\n  \"wave\": \"G9.6\",\n  \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \"mode\": \"pass\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(Jolt 5.3.0 + rapier3d 0.33.0 双臂 host 确定性面)\", \"validation\": \"not_applicable\", \"require_real\": {}, \"build_debug_assertions\": {}, \"features\": \"physics-capture,rapier(rapier 默认 off 纪律维持——本 harness 仅 feature on 构建档产绿)\"}},\n  \"ab_fixture\": {{\"scenario\": \"canonical 大堆叠 {} 层\", \"input_digest\": \"{}\", \"jolt_world_digest\": \"{}\", \"rapier_world_digest\": \"{}\", \"jolt_step_ns_median\": {}, \"rapier_step_ns_median\": {}, \"jolt_contact_events_total\": {}, \"rapier_contact_events_total\": {}}},\n  \"cross_solver_deviation\": {{\"world_chain_bitwise_equal\": {}, \"max_translation_abs_diff\": {:.9e}, \"mean_translation_abs_diff\": {:.9e}, \"max_linvel_abs_diff\": {:.9e}, \"contact_events_abs_diff\": {}, \"within_tolerance\": {}}},\n  \"rd044\": {{\"condition_literal_unchanged\": {}, \"verdict\": \"{}\", \"registration\": \"G9_CANDIDATE_DECISIONS 校准注 + ledger v1.95 只追加(RD 消费写明);不升格深造\"}},\n  \"glam_migration_note_recorded\": true,\n  \"measured_report\": \"{}\",\n  \"conformance_corpus\": {{\"dir\": \"conformance/physics\", \"rx_anchors\": {{{}}}}},\n  \"checks\": {{{}}},\n  \"commands\": [{}],\n  \"failures\": [{}]\n}}",
            utc_now(),
            json_escape(&base_commit),
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
            cfg!(debug_assertions),
            report.spec.layers,
            report.jolt.input_digest,
            report.jolt.world_digest,
            report.rapier.world_digest,
            report.jolt.step_ns_median(),
            report.rapier.step_ns_median(),
            report.jolt.contact_events_total,
            report.rapier.contact_events_total,
            cross_bitwise_equal,
            report.deviation.max_translation_abs_diff,
            report.deviation.mean_translation_abs_diff,
            report.deviation.max_linvel_abs_diff,
            report.deviation.contact_events_abs_diff,
            report.deviation.within_tolerance,
            rd044_literal_ok,
            verdict,
            json_escape(&args.report.display().to_string().replace('\\', "/")),
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
                "{TAG}: PASS A/B 同场景同输入同画像 + 双臂双跑位级一致 + measured 报告 + 双 RED 臂 + RD-044 字面不变(verdict={verdict})"
            );
            std::process::exit(0);
        }
        fail(&format!("{failures:?}"));
    }
}

#[cfg(all(feature = "physics-capture", feature = "rapier"))]
fn main() {
    imp::main()
}

#[cfg(not(all(feature = "physics-capture", feature = "rapier")))]
fn main() {
    // feature `rapier` 默认 off 纪律维持(RFC-0017 §4.D1):未编译档
    // fail-closed typed Err,不静默退化为单臂绿。
    fail(
        "RapierBackendNotCompiled(feature `rapier` 未编译——A/B 缺臂 fail-closed;真跑面 = cargo run -p rurix-physics --features 'physics-capture,rapier' --bin g9_m126_rapier_benchmark)",
    );
}
