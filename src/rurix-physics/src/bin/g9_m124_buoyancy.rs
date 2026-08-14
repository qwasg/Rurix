//! G9.6 M124 解析浮力走 Field 通道 harness(RXS-0376;门
//! `g9.p1.m124.buoyancy_field_channel`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M124 行逐字 + spec/physics.md RXS-0376)
//!
//! 1. **解析浮力走 Field 通道**(正例臂):水体区域 = persistent field
//!    (解析水面函数 = analytic-surface 基元),`FieldPhysicsType::Buoyancy`
//!    语义;每 tick 对 filter 内 RigidBody 计算 clipped 浸入体积/浸没质心 →
//!    浮力 impulse + 浮力矩 + 线性/角阻力 impulse,经 impulse/force 唯一写
//!    口进求解器主流;介质参数(密度/阻力)为场定义的一部分进 digest。
//! 2. **旁路 API 注入即 RED**(RED 臂独立有效):`buoyancy_set_velocity` /
//!    `buoyancy_teleport` 类旁路面一律 fail-closed typed Err(旁路即门红)。
//! 3. **场通道未接线即 RED**(RED 臂独立有效):Buoyancy 场缺介质参数 /
//!    capture 缺场 hash 注入 ⇒ fail-closed。
//! 4. **帧率敏感漂移注入即 RED**(RED 臂独立有效):帧率相关插值/墙钟相位
//!    标本注入 ⇒ 变帧率逐位一致破坏可检测。
//! 5. **细长体/翻滚体 corpus fixture**:`conformance/physics/buoyancy/`
//!    slender_body/tumbler_body 两 canonical 场景(场景 + 输入参数 + 预期
//!    行为特征),重算与 fixture 逐字一致(禁手写 golden)。
//! 6. **capture→replay 逐 tick hash 一致 + 变帧率输入同 tick 结果逐位一
//!    致**(determinism 断言;固定 dt + 解析水面函数)。
//! 7. **measured 冻结带**:digest 组落
//!    `milestones/g9/g9_m124_buoyancy_freeze.json`(measured 冻结 +
//!    provenance,`--emit-freeze` 生成,禁手写)。
//!
//! ## 三态
//!
//! host 纯确定性面(Jolt 5.3 lockstep 单线程;`RURIX_REQUIRE_REAL=1` 以 host
//! 确定性为准,validation 不适用);feature `physics-buoyancy`(R-7 🔒 冻
//! 结名)未编译 ⇒ FeatureNotCompiled fail-closed(不静默退化成视觉-only
//! 成功);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m124_buoyancy [--evidence <path>] [--freeze <path>]
//! g9_m124_buoyancy --emit-freeze [--freeze <path>] [--write-corpus]
//! g9_m124_buoyancy --red-arm bypass-api|field-unwired|framerate-drift
//! ```

#![forbid(unsafe_code)]

const TAG: &str = "G9_M124_BUOYANCY";

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

#[cfg(feature = "physics-buoyancy")]
mod imp {
    use std::path::PathBuf;

    use rurix_physics::field::buoyancy::{
        BuoyancyError, bypass_set_velocity, bypass_teleport, medium_from_field,
        reject_buoyancy_bypass,
    };
    use rurix_physics::field::buoyancy_capture::{
        CANONICAL_SCENARIO_NAMES, canonical_scenario, corpus_fixture_matches,
        inject_framerate_sensitive_drift, persist_corpus_fixture, record_buoyancy_capture,
        replay_buoyancy_capture, verify_variable_framerate_replay,
    };
    use rurix_physics::field::def::FieldPhysicsType;

    use super::{TAG, fail};

    const CORPUS_RX: &[(&str, &str)] = &[
        ("accept/buoyancy_field_channel_minimal.rx", "RXS-0376"),
        ("reject/buoyancy_bypass_api_injection.rx", "RXS-0376"),
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
        // src/rurix-physics → 仓库根(pop 两层:crate → src → root)。
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
        freeze: PathBuf,
        emit_freeze: bool,
        write_corpus: bool,
        red_arm: Option<String>,
    }

    fn parse_args() -> Args {
        let root = workspace_root();
        let mut out = Args {
            evidence: None,
            freeze: root.join("milestones/g9/g9_m124_buoyancy_freeze.json"),
            emit_freeze: false,
            write_corpus: false,
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
                "--freeze" => out.freeze = PathBuf::from(take(&mut i)),
                "--emit-freeze" => out.emit_freeze = true,
                "--write-corpus" => out.write_corpus = true,
                "--red-arm" => out.red_arm = Some(take(&mut i)),
                other => fail(&format!("未知参数: {other}")),
            }
            i += 1;
        }
        out
    }

    /// RED 臂:旁路 API 注入(buoyancy_set_velocity / buoyancy_teleport 直写
    /// 速度/位置/transform 的旁路面)——一律 fail-closed typed Err。
    fn red_arm_bypass_api() -> Result<(), String> {
        for api in [bypass_set_velocity(), bypass_teleport()] {
            match reject_buoyancy_bypass(api) {
                Err(BuoyancyError::BypassApiRejected(_)) => {}
                other => return Err(format!("旁路 {api} 未拒(漏检): {other:?}")),
            }
        }
        Ok(())
    }

    /// RED 臂:场通道未接线(Buoyancy 场缺介质参数 ⇒ 求值 fail-closed;
    /// capture 缺场 hash ⇒ replay fail-closed)。
    fn red_arm_field_unwired() -> Result<(), String> {
        let dt = 1.0 / 60.0f32;
        let scene = canonical_scenario("slender_body", dt).ok_or("scenario")?;
        // 负例①:Buoyancy 场缺阻力子节点(介质参数缺失)。
        let mut bad = scene.field.clone();
        bad.root.children.clear();
        match medium_from_field(&bad, 9.81) {
            Err(BuoyancyError::FieldChannelMissingParams(_)) => {}
            other => return Err(format!("缺介质参数未拒(漏检): {other:?}")),
        }
        // 负例②:非 Buoyancy 语义场冒充浮力场。
        let mut bad2 = scene.field.clone();
        bad2.physics_type = FieldPhysicsType::LinearForce;
        match medium_from_field(&bad2, 9.81) {
            Err(BuoyancyError::FieldChannelMissingParams(_)) => {}
            other => return Err(format!("非 Buoyancy 语义未拒(漏检): {other:?}")),
        }
        // 负例③:capture 场 hash 缺失注入(场通道未接线)⇒ replay fail-closed。
        let out = record_buoyancy_capture(&scene).map_err(|e| e.to_string())?;
        let mut tampered = out.artifact.clone();
        for t in &mut tampered.ticks {
            t.post.field_semantic_hash = None;
        }
        match replay_buoyancy_capture(&scene, &tampered, 60) {
            Err(_) => Ok(()),
            Ok(_) => Err("场 hash 缺失注入 replay 未红(漏检)".into()),
        }
    }

    /// RED 臂:帧率敏感漂移注入(帧率相关插值/墙钟相位标本)⇒ 重算与记账
    /// 逐位分叉,fail-closed。
    fn red_arm_framerate_drift() -> Result<(), String> {
        let dt = 1.0 / 60.0f32;
        let scene = canonical_scenario("slender_body", dt).ok_or("scenario")?;
        let out = record_buoyancy_capture(&scene).map_err(|e| e.to_string())?;
        let mut tampered = out.artifact.clone();
        let mut injected = false;
        for cmd in &mut tampered.ticks[1].pre {
            if let rurix_physics::capture::journal::JournalCommand::ApplyImpulse {
                impulse, ..
            } = cmd
            {
                *impulse = inject_framerate_sensitive_drift(*impulse, 24);
                injected = true;
            }
        }
        if !injected {
            return Err("注入点缺失(tick1 无记账 impulse)".into());
        }
        match replay_buoyancy_capture(&scene, &tampered, 60) {
            Err(_) => Ok(()),
            Ok(_) => Err("帧率敏感漂移注入 replay 未红(漏检)".into()),
        }
    }

    pub fn main() {
        let args = parse_args();
        let root = workspace_root();

        // ── RED 臂子模式(臂面独立有效) ──
        if let Some(arm) = &args.red_arm {
            let r = match arm.as_str() {
                "bypass-api" => red_arm_bypass_api(),
                "field-unwired" => red_arm_field_unwired(),
                "framerate-drift" => red_arm_framerate_drift(),
                other => fail(&format!(
                    "未知 RED 臂: {other}(bypass-api|field-unwired|framerate-drift)"
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
        let dt = 1.0 / 60.0f32;

        // ── 步骤 1:conformance 语料锚定核验(.rx accept/reject + buoyancy corpus) ──
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

        // ── 步骤 2:canonical 场景录制(两场景)+ 正例判据 ──
        let mut outcomes = Vec::new();
        for name in CANONICAL_SCENARIO_NAMES {
            let scene = canonical_scenario(name, dt).unwrap_or_else(|| fail("scenario"));
            match record_buoyancy_capture(&scene) {
                Ok(o) => outcomes.push((name, scene, o)),
                Err(e) => fail(&format!("录制 {name}: {e}")),
            }
        }
        // 正例:走 Field 通道(介质参数自场定义消费 + impulse 非零 + 场 hash
        // 链进主流;Field 统一抽象第二个真实用户的消费面)。
        let field_channel_green = outcomes.iter().all(|(_, scene, o)| {
            medium_from_field(&scene.field, 9.81).is_ok()
                && o.applied_impulse_count > 0
                && o.field_chain_digest.len() == 64
        });
        if !field_channel_green {
            failures.push("走 Field 通道正例臂失效(介质参数/impulse/场 hash 链)".into());
        }

        // ── 步骤 3:capture→replay 逐 tick hash 一致 ──
        let mut replay_ok = true;
        for (name, scene, o) in &outcomes {
            match replay_buoyancy_capture(scene, &o.artifact, 60) {
                Ok(rep) => {
                    if !(rep.journal_fully_consumed
                        && rep.impulses_recomputed_equal
                        && rep.world_digest == o.world_digest
                        && rep.field_chain_digest == o.field_chain_digest)
                    {
                        replay_ok = false;
                        failures.push(format!("replay {name} 对拍不齐"));
                    }
                }
                Err(e) => {
                    replay_ok = false;
                    failures.push(format!("replay {name}: {e}"));
                }
            }
        }

        // ── 步骤 4:变帧率输入同 tick 结果逐位一致(采样粒度扰动注入仍一致) ──
        let mut vfr_ok = true;
        for (name, scene, o) in &outcomes {
            match verify_variable_framerate_replay(scene, &o.artifact) {
                Ok(reports) => {
                    if !reports.iter().all(|r| r.world_digest == o.world_digest) {
                        vfr_ok = false;
                        failures.push(format!("变帧率 {name} digest 分叉"));
                    }
                }
                Err(e) => {
                    vfr_ok = false;
                    failures.push(format!("变帧率 {name}: {e}"));
                }
            }
        }

        // ── 步骤 5:预期行为特征 + corpus fixture 消费断言 ──
        let mut behavior_ok = true;
        for (name, _, o) in &outcomes {
            let b = o.behavior;
            let ok = match *name {
                "slender_body" => {
                    b.final_submerged_fraction > 0.1 && b.final_submerged_fraction < 1.0
                }
                "tumbler_body" => b.final_submerged_fraction == 1.0 && b.final_linvel_z < 0.0,
                _ => false,
            };
            if !ok {
                behavior_ok = false;
                failures.push(format!("行为特征 {name} 不符: {b:?}"));
            }
        }
        // corpus fixture 重算逐字一致(--write-corpus 刷新;否则必须为在树 fixture)。
        let mut corpus_fixture_ok = true;
        for (name, _, o) in &outcomes {
            let dir = corpus_dir.join("buoyancy").join(name);
            if args.write_corpus {
                if let Err(e) = persist_corpus_fixture(&dir, o) {
                    corpus_fixture_ok = false;
                    failures.push(format!("写 corpus {name}: {e}"));
                }
            } else if !dir.join("input.json").exists() {
                corpus_fixture_ok = false;
                failures.push(format!("corpus fixture {name} 不在树(先跑 --write-corpus)"));
            } else {
                match corpus_fixture_matches(&dir, o) {
                    Ok(true) => {}
                    Ok(false) => {
                        corpus_fixture_ok = false;
                        failures.push(format!("corpus fixture {name} 与重算漂移"));
                    }
                    Err(e) => {
                        corpus_fixture_ok = false;
                        failures.push(format!("corpus fixture {name}: {e}"));
                    }
                }
            }
        }

        // ── 步骤 6:RED 臂内联实测(三臂独立) ──
        let bypass_red = red_arm_bypass_api().is_ok();
        let unwired_red = red_arm_field_unwired().is_ok();
        let drift_red = red_arm_framerate_drift().is_ok();
        if !bypass_red {
            failures.push("旁路 API RED 臂失效".into());
        }
        if !unwired_red {
            failures.push("场通道未接线 RED 臂失效".into());
        }
        if !drift_red {
            failures.push("帧率敏感漂移 RED 臂失效".into());
        }

        // ── 步骤 7:measured 冻结带(emit / 对拍) ──
        let mut freeze_rows = String::new();
        for (name, _, o) in &outcomes {
            freeze_rows.push_str(&format!(
                "    \"{name}_world_digest\": \"{}\",\n    \"{name}_journal_digest\": \"{}\",\n    \"{name}_field_chain_digest\": \"{}\",\n    \"{name}_input_digest\": \"{}\",\n    \"{name}_applied_impulse_count\": \"{}\",\n    \"{name}_final_z\": \"{:08x}\",\n    \"{name}_final_submerged_fraction\": \"{:08x}\",\n    \"{name}_final_linvel_z\": \"{:08x}\",\n",
                o.world_digest,
                o.journal_digest,
                o.field_chain_digest,
                o.input_digest,
                o.applied_impulse_count,
                o.behavior.final_z.to_bits(),
                o.behavior.final_submerged_fraction.to_bits(),
                o.behavior.final_linvel_z.to_bits(),
            ));
        }
        if args.emit_freeze {
            let band = format!(
                "{{\n  \"schema\": \"rurix.g9m124.buoyancy_freeze.v1\",\n  \"spec_anchor\": \"RXS-0376\",\n  \"frozen_at_utc\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"generator\": \"cargo build -p rurix-physics --features physics-buoyancy --bin g9_m124_buoyancy && g9_m124_buoyancy --emit-freeze --write-corpus\",\n  \"generator_host\": \"{} {}; Jolt 5.3.0 lockstep (job_threads=1, dt_fixed=1/60)\",\n  \"provenance\": \"G9.6 M124(RXS-0376)canonical 浮力场景首次真跑 measured_local 回填,此后字节冻结;禁手写(P-09)。场景 = 120 tick 细长体(箱 ρ=500 半浸漂浮收敛)/翻滚体(胶囊 ρ=1200 全浸下沉),persistent Buoyancy 场(解析水面 z=0 + 介质密度 1000 kg/m³ + 线性/角阻力 0.9/0.6,介质参数进场定义 digest);world/journal/field_chain digest = M66 主流 capture 三面锚;行为特征 = 末 tick 步进后快照 measured 观测。\",\n{}}}\n",
                utc_now(),
                std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string()),
                std::env::consts::OS,
                std::env::consts::ARCH,
                freeze_rows.trim_end_matches([',', '\n']),
            );
            if let Some(parent) = args.freeze.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&args.freeze, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
            println!("{TAG}: 冻结带已落盘 {:?}", args.freeze);
        }
        let mut freeze_ok = true;
        if !args.emit_freeze {
            let t = std::fs::read_to_string(&args.freeze).unwrap_or_else(|_| {
                fail(&format!(
                    "冻结带 {:?} 不存在——先跑 --emit-freeze(禁手写)",
                    args.freeze
                ))
            });
            for (name, _, o) in &outcomes {
                for (suffix, val) in [
                    ("world_digest", o.world_digest.clone()),
                    ("journal_digest", o.journal_digest.clone()),
                    ("field_chain_digest", o.field_chain_digest.clone()),
                    ("input_digest", o.input_digest.clone()),
                    ("applied_impulse_count", o.applied_impulse_count.to_string()),
                    ("final_z", format!("{:08x}", o.behavior.final_z.to_bits())),
                    (
                        "final_submerged_fraction",
                        format!("{:08x}", o.behavior.final_submerged_fraction.to_bits()),
                    ),
                    (
                        "final_linvel_z",
                        format!("{:08x}", o.behavior.final_linvel_z.to_bits()),
                    ),
                ] {
                    let key = format!("{name}_{suffix}");
                    match json_str(&t, &key) {
                        Some(frozen) if frozen == val => {}
                        Some(frozen) => {
                            freeze_ok = false;
                            failures.push(format!("冻结带漂移: {key}(冻结 {frozen} ≠ 实测 {val})"));
                        }
                        None => fail(&format!("冻结带缺 {key}")),
                    }
                }
            }
        }

        // ── 步骤 8:evidence(rurix.g9m124.buoyancy.v1) ──
        let checks: [(&str, bool); 9] = [
            ("conformance_corpus_anchored", corpus_ok),
            ("field_channel_buoyancy_green", field_channel_green),
            ("bypass_api_injection_red", bypass_red),
            ("field_channel_unwired_red", unwired_red),
            ("framerate_drift_injection_red", drift_red),
            ("capture_replay_tick_hash_equal", replay_ok),
            ("variable_framerate_bitwise_identical", vfr_ok),
            (
                "behavior_traits_and_corpus_fixture",
                behavior_ok && corpus_fixture_ok,
            ),
            (
                "measured_freeze_digest_match",
                freeze_ok || args.emit_freeze,
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
        let base_commit =
            std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
        let mut scenarios_json = String::new();
        for (name, _, o) in &outcomes {
            scenarios_json.push_str(&format!(
                "    {{\"scenario\": \"{name}\", \"world_digest\": \"{}\", \"journal_digest\": \"{}\", \"field_chain_digest\": \"{}\", \"input_digest\": \"{}\", \"applied_impulse_count\": {}, \"behavior\": {{\"final_z_bits\": \"{:08x}\", \"final_submerged_fraction_bits\": \"{:08x}\", \"final_linvel_z_bits\": \"{:08x}\"}}}},\n",
                o.world_digest,
                o.journal_digest,
                o.field_chain_digest,
                o.input_digest,
                o.applied_impulse_count,
                o.behavior.final_z.to_bits(),
                o.behavior.final_submerged_fraction.to_bits(),
                o.behavior.final_linvel_z.to_bits(),
            ));
        }
        let json = format!(
            "{{\n  \"schema\": \"rurix.g9m124.buoyancy.v1\",\n  \"schema_version\": 1,\n  \"subject\": \"g9_m124_buoyancy\",\n  \"spec_anchor\": \"RXS-0376\",\n  \"assertion_id\": \"g9.p1.m124.buoyancy_field_channel\",\n  \"milestone\": \"M124\",\n  \"wave\": \"G9.6\",\n  \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(Jolt 5.3.0 lockstep 单线程 host 确定性面)\", \"validation\": \"not_applicable\", \"require_real\": {}, \"build_debug_assertions\": {}}},\n  \"field_channel\": {{\"semantics\": \"FieldPhysicsType::Buoyancy(persistent field + analytic-surface 水面基元;Field 统一抽象第二个真实用户)\", \"medium_params_in_def_digest\": true, \"apply_path\": \"impulse/force 唯一写口(ParticleAdapter::set_force_impulse);零新 FFI;浮力权威不经 Taichi(M49 defer)\", \"shape_layering\": \"convex/primitive 解析 clip 闭集(Sphere/Box/Capsule);闭集外 fail-closed → voxelized volume table cooked 通道(版本化注册面)\"}},\n  \"scenarios\": [\n{}],\n  \"determinism\": {{\"capture_replay_tick_hash_equal\": {}, \"variable_framerate_bitwise_identical\": {}, \"fixed_dt\": \"1/60 锁死\", \"framerate_interpolation\": \"禁帧率相关插值/禁墙钟相位(采样粒度只影响 replay 调用节拍,不影响 tick 序列)\"}},\n  \"conformance_corpus\": {{\"dir\": \"conformance/physics\", \"rx_anchors\": {{{}}}, \"buoyancy_fixtures\": [\"buoyancy/slender_body\", \"buoyancy/tumbler_body\"]}},\n  \"checks\": {{{}}},\n  \"commands\": [{}],\n  \"failures\": [{}]\n}}",
            if args.emit_freeze {
                "emit-freeze"
            } else {
                "pass"
            },
            utc_now(),
            json_escape(&base_commit),
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
            cfg!(debug_assertions),
            scenarios_json.trim_end_matches([',', '\n']),
            replay_ok,
            vfr_ok,
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
                "{TAG}: PASS 走 Field 通道 + corpus fixture + capture/replay 逐 tick hash + 变帧率逐位一致 + 三 RED 臂(host 确定性面)"
            );
            std::process::exit(0);
        }
        fail(&format!("{failures:?}"));
    }
}

#[cfg(feature = "physics-buoyancy")]
fn main() {
    imp::main()
}

#[cfg(not(feature = "physics-buoyancy"))]
fn main() {
    // `physics-buoyancy` feature(RFC-0024 R-7 🔒 冻结名)未编译 ⇒
    // FeatureNotCompiled 类错误 fail-closed(RXS-0376 Implementation
    // Requirements:不静默退化成视觉-only 成功)。
    fail(
        "FeatureNotCompiled(physics-buoyancy):浮力面未编译进本构建——真跑面 = cargo run -p rurix-physics --features physics-buoyancy --bin g9_m124_buoyancy",
    );
}
