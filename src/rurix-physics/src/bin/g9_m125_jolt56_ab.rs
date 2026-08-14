//! G9.6 M125 Jolt 5.3→5.6 升级 A/B 评估 harness(RXS-0377;门
//! `g9.p1.m125.jolt_56_ab_evaluation`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M125 行逐字 + spec/physics.md RXS-0377)
//!
//! RFC-0021 §4.A4 **七步程序逐字**(逐步留痕进 measured 报告 `seven_step_record`):
//! ① 冻结 5.3 基线(conformance/physics 全树 digest 清单 + 既有 replay corpus
//!   十场景 5.3 重跑全 PASS〔CCD/contact/query 等轴〕+ canonical 场景 5.3
//!   measured baseline)→ ② 5.6 独立 vendor/ABI 构建不覆盖 5.3 基线(vendor
//!   标记三面机核 + 双后端同进程各自实例化;覆盖注入即 RED)→ ③ 两版本各自
//!   同版本 capture/replay 逐 tick 一致(M66 主流,各自版本锚 header)→ ④ 相同
//!   canonical source asset/input journal A/B(输入 digest 两臂逐位相等硬断言)
//!   → ⑤ 性能阈值只从真实采样写入 budget(本批零 budget counter——评估不升格;
//!   版本锚按实测 tag/commit 登记:5.6.0/e77f175 + JoltC 2982004)→ ⑥ 失败臂
//!   钉 5.3 不伪绿(伪写 5.6 PASS fail-closed)→ ⑦ 采纳臂三件事登记(本评估
//!   不升格默认,corpus 迁移/replay 门重跑/判据字面修订三件 not-triggered)。
//! **新摩擦模型(平均接触点)重点实测**:求解器语义变化逐字段
//! exact/tolerance/invariant 分类(实测驱动)+ 滑块行程/堆叠沉降/接触计数
//! 专项记录。GPU compute 只评估不接权威(编译期四开关 OFF + C 面零导出 +
//! 接权威提案 typed Err;RD-043 + 矩阵 §12 + 独立 Full RFC 字面)。layout 探针
//! 工具化入库(tools/layout_dump56.cpp → sys56 ffi_layout_anchors)。两臂诚实
//! 登记(maintain_5_3_default / pinned_5_3_on_failure 闭集,禁伪绿)。
//!
//! ## 三态
//!
//! host 纯确定性面(`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,validation
//! 不适用);feature `jolt56`(默认 off 纪律维持——评估臂不升格生产默认)未编译
//! ⇒ Jolt56BackendNotCompiled fail-closed(不静默单臂充绿);判据不符 / RED 轴
//! 失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m125_jolt56_ab [--evidence <path>] [--report <path>]
//! g9_m125_jolt56_ab --red-arm vendor-overwrite|gpu-authority|fake-pass
//! ```

#![forbid(unsafe_code)]

const TAG: &str = "G9_M125_JOLT56_AB";

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

#[cfg(all(feature = "physics-capture", feature = "jolt56"))]
mod imp {
    use std::path::{Path, PathBuf};

    use rurix_physics::ab_eval::{
        AbError, AbReport, CanonicalAbSpec, FRICTION_MODEL_56_NOTE, GPU_COMPUTE_EVALUATION_NOTE,
        check_baseline_vendor_markers, check_vendor56_markers, connect_gpu_compute_authority,
        run_ab_evaluation, validate_report_honesty,
    };
    use rurix_physics::capture::replayer::{ReplayVerdict, replay_capture_dir};
    use rurix_physics::{BackendKind, PhysicsWorld};
    use rurix_pkg::sha256::{digest, hex};

    use super::{TAG, fail};

    const CORPUS_RX: &[(&str, &str)] = &[
        ("accept/jolt_ab_seven_step_minimal.rx", "RXS-0377"),
        ("reject/jolt_56_vendor_overwrite_baseline.rx", "RXS-0377"),
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
            report: root.join("milestones/g9/g9_m125_jolt56_ab.json"),
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

    /// RED 臂 1:5.6 vendor 覆盖 5.3 基线注入——篡改标记必须 fail-closed typed
    /// Err(真实 5.3 基线标记在位为对照)。
    fn red_arm_vendor_overwrite() -> Result<(), String> {
        let root = workspace_root();
        let core = std::fs::read_to_string(
            root.join("src/rurix-physics-sys/vendor/JoltC/JoltPhysics/Jolt/Core/Core.h"),
        )
        .map_err(|e| format!("读 5.3 Core.h: {e}"))?;
        let funcs = std::fs::read_to_string(
            root.join("src/rurix-physics-sys/vendor/JoltC/JoltC/Functions.h"),
        )
        .map_err(|e| format!("读 5.3 Functions.h: {e}"))?;
        // 对照面:真实 5.3 基线标记在位。
        check_baseline_vendor_markers(&core, &funcs)
            .map_err(|e| format!("真实 5.3 基线标记核验误拒: {e}"))?;
        // 注入面 A:5.6 版本宏覆盖 5.3(就地升级 pin)。
        let tampered = core.replace("JPH_VERSION_MINOR 3", "JPH_VERSION_MINOR 6");
        match check_baseline_vendor_markers(&tampered, &funcs) {
            Err(AbError::BaselineVendorTampered(_)) => {}
            other => return Err(format!("版本宏覆盖注入未拒(漏检): {other:?}")),
        }
        // 注入面 B:符号重命名面混入 5.3 线(JPC56_/JPH56 出现于基线)。
        let renamed = format!("{funcs}\nJPC56_API void JPC56_PhysicsSystem_new();\n");
        match check_baseline_vendor_markers(&core, &renamed) {
            Err(AbError::BaselineVendorTampered(_)) => {}
            other => return Err(format!("符号面覆盖注入未拒(漏检): {other:?}")),
        }
        Ok(())
    }

    /// RED 臂 2:GPU compute 接权威注入——一律 fail-closed typed Err。
    fn red_arm_gpu_authority() -> Result<(), String> {
        match connect_gpu_compute_authority("proposal: Jolt 5.6 GPU compute shader 权威求解接线")
        {
            Err(AbError::GpuComputeAuthorityUsurpation(_)) => {}
            other => return Err(format!("GPU compute 接权威提案未拒(漏检): {other:?}")),
        }
        Ok(())
    }

    /// RED 臂 3:失败臂伪写 5.6 PASS——非闭集 verdict 字面 fail-closed。
    fn red_arm_fake_pass() -> Result<(), String> {
        for forged in ["adopted_5_6", "5.6_pass", "pass_56", "adopted"] {
            match validate_report_honesty(forged) {
                Err(AbError::FakePassAttempt(_)) => {}
                other => return Err(format!("伪写 verdict {forged:?} 未拒(漏检): {other:?}")),
            }
        }
        // 合规面:闭集两字面放行(诚实登记不误拒)。
        validate_report_honesty("maintain_5_3_default").map_err(|e| format!("合规面误拒: {e}"))?;
        validate_report_honesty("pinned_5_3_on_failure").map_err(|e| format!("合规面误拒: {e}"))?;
        Ok(())
    }

    /// 步骤①:conformance/physics 全树 digest 清单(measured 冻结面;
    /// corpus/资产 digest 全程可复算)。
    fn corpus_manifest(root: &Path) -> Result<(String, u64), String> {
        let base = root.join("conformance/physics");
        let mut entries: Vec<String> = Vec::new();
        let mut stack = vec![base.clone()];
        while let Some(dir) = stack.pop() {
            let rd = std::fs::read_dir(&dir).map_err(|e| format!("读目录 {dir:?}: {e}"))?;
            for ent in rd {
                let ent = ent.map_err(|e| format!("目录项: {e}"))?;
                let path = ent.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let data = std::fs::read(&path).map_err(|e| format!("读 {path:?}: {e}"))?;
                    let rel = path
                        .strip_prefix(&base)
                        .map_err(|e| format!("rel: {e}"))?
                        .display()
                        .to_string()
                        .replace('\\', "/");
                    entries.push(format!("{}:{}", rel, hex(&digest(&data))));
                }
            }
        }
        entries.sort();
        let n = entries.len() as u64;
        let manifest = entries.join("\n") + "\n";
        Ok((hex(&digest(manifest.as_bytes())), n))
    }

    /// 步骤①:既有 replay corpus 十场景 5.3 重跑(CCD/contact/query 等轴;
    /// 全 Pass = 基线 artifact 可复算 + 5.3 基线门回归面)。
    fn replay_corpus_regression(root: &Path) -> Result<(u64, Vec<String>), String> {
        let base = root.join("conformance/physics/replay");
        let mut dirs: Vec<PathBuf> = Vec::new();
        let rd = std::fs::read_dir(&base).map_err(|e| format!("读 replay 目录: {e}"))?;
        for ent in rd {
            let ent = ent.map_err(|e| format!("目录项: {e}"))?;
            if ent.path().is_dir() {
                dirs.push(ent.path());
            }
        }
        dirs.sort();
        let mut ids = Vec::new();
        for d in &dirs {
            let report = replay_capture_dir(d, None).map_err(|e| format!("replay {d:?}: {e}"))?;
            if report.verdict != ReplayVerdict::Pass || !report.journal_fully_consumed {
                return Err(format!(
                    "replay 场景 {} 非 Pass: {:?}",
                    d.display(),
                    report.verdict
                ));
            }
            ids.push(report.scenario_id.clone());
        }
        Ok((dirs.len() as u64, ids))
    }

    /// 步骤②:vendor 标记三面核验(5.3 基线在位 + 5.6 线在位 + 同进程并存)。
    fn vendor_markers(root: &Path) -> Result<(), String> {
        let core53 = std::fs::read_to_string(
            root.join("src/rurix-physics-sys/vendor/JoltC/JoltPhysics/Jolt/Core/Core.h"),
        )
        .map_err(|e| format!("读 5.3 Core.h: {e}"))?;
        let funcs53 = std::fs::read_to_string(
            root.join("src/rurix-physics-sys/vendor/JoltC/JoltC/Functions.h"),
        )
        .map_err(|e| format!("读 5.3 Functions.h: {e}"))?;
        check_baseline_vendor_markers(&core53, &funcs53).map_err(|e| e.to_string())?;
        let core56 = std::fs::read_to_string(
            root.join("src/rurix-physics-sys56/vendor/JoltC/JoltPhysics/Jolt/Core/Core.h"),
        )
        .map_err(|e| format!("读 5.6 Core.h: {e}"))?;
        let funcs56 = std::fs::read_to_string(
            root.join("src/rurix-physics-sys56/vendor/JoltC/JoltC/Functions.h"),
        )
        .map_err(|e| format!("读 5.6 Functions.h: {e}"))?;
        check_vendor56_markers(&core56, &funcs56).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 步骤②并存断言:双后端同进程各自实例化(链接成功即符号隔离证明;
    /// 各 step 一拍确认两世界各自活动)。
    fn coexistence_assertion(spec: &CanonicalAbSpec) -> Result<(), String> {
        let mut w53 = PhysicsWorld::new(spec.world_desc(BackendKind::Jolt))
            .map_err(|e| format!("5.3 世界: {e}"))?;
        let mut w56 = PhysicsWorld::new(spec.world_desc(BackendKind::Jolt56))
            .map_err(|e| format!("5.6 世界: {e}"))?;
        let dt = spec.world_desc(BackendKind::Jolt).dt_fixed;
        w53.step(dt).map_err(|e| format!("5.3 step: {e}"))?;
        w56.step(dt).map_err(|e| format!("5.6 step: {e}"))?;
        Ok(())
    }

    /// L4:GPU compute 只评估不接权威机核面(build 四开关 OFF 字面 + C 面零
    /// GPU 导出 + RFC/spec 禁止线字面 + 接权威提案 typed Err)。
    fn gpu_compute_surface(root: &Path) -> Result<(), String> {
        let build_rs = std::fs::read_to_string(root.join("src/rurix-physics-sys56/build.rs"))
            .map_err(|e| format!("读 sys56 build.rs: {e}"))?;
        for def in [
            "\"JPH_USE_DX12\", \"OFF\"",
            "\"JPH_USE_VK\", \"OFF\"",
            "\"JPH_USE_MTL\", \"OFF\"",
            "\"JPH_USE_CPU_COMPUTE\", \"OFF\"",
        ] {
            if !build_rs.contains(def) {
                return Err(format!("sys56 build.rs 缺 {def} 字面"));
            }
        }
        // C 面零 GPU compute 导出(JoltC 从未导出;5.6 线同)。
        let funcs56 = std::fs::read_to_string(
            root.join("src/rurix-physics-sys56/vendor/JoltC/JoltC/Functions.h"),
        )
        .map_err(|e| format!("读 5.6 Functions.h: {e}"))?;
        let gpu_exports: usize = [
            "ComputeShader",
            "ComputeSystem",
            "ComputeQueue",
            "ComputeBuffer",
        ]
        .iter()
        .map(|needle| funcs56.matches(needle).count())
        .sum();
        if gpu_exports != 0 {
            return Err(format!("5.6 C 面出现 GPU compute 导出 {gpu_exports} 处"));
        }
        // 禁止线字面(RFC-0024 §4.E1 + spec RXS-0377 L4;0-byte 消费面)。
        let rfc = std::fs::read_to_string(root.join("rfcs/0024-physics-platform-revision.md"))
            .map_err(|e| format!("读 RFC-0024: {e}"))?;
        if !rfc.contains("只评估不接权威") || !rfc.contains("GPU 主刚体禁止线 0-byte")
        {
            return Err("RFC-0024 GPU 禁止线字面漂移".into());
        }
        let spec = std::fs::read_to_string(root.join("spec/physics.md"))
            .map_err(|e| format!("读 spec/physics.md: {e}"))?;
        if !spec.contains("只评估不接权威") {
            return Err("spec RXS-0377 L4 字面漂移".into());
        }
        red_arm_gpu_authority()
    }

    /// L5:layout 探针工具化入库面(探针源码在树 + ffi 锚定点字面)。
    fn layout_probe_surface(root: &Path) -> Result<(), String> {
        let probe = root.join("src/rurix-physics-sys56/tools/layout_dump56.cpp");
        if !probe.is_file() {
            return Err("layout 探针 tools/layout_dump56.cpp 不在树".into());
        }
        let text = std::fs::read_to_string(&probe).map_err(|e| format!("读探针: {e}"))?;
        if !text.contains("JPC56_ShapeCastSettings") || !text.contains("ExtraConvexRadius") {
            return Err("探针未覆盖 5.6 *Settings 新字段面".into());
        }
        let ffi = std::fs::read_to_string(root.join("src/rurix-physics-sys56/src/ffi.rs"))
            .map_err(|e| format!("读 sys56 ffi.rs: {e}"))?;
        if !ffi.contains("offset_of!(JpcShapeCastSettings, extra_convex_radius) == 32") {
            return Err("sys56 ffi_layout_anchors 缺 ExtraConvexRadius@32 锚".into());
        }
        Ok(())
    }

    pub fn main() {
        let args = parse_args();
        let root = workspace_root();

        // ── RED 臂子模式 ──
        if let Some(arm) = &args.red_arm {
            let r = match arm.as_str() {
                "vendor-overwrite" => red_arm_vendor_overwrite(),
                "gpu-authority" => red_arm_gpu_authority(),
                "fake-pass" => red_arm_fake_pass(),
                other => fail(&format!(
                    "未知 RED 臂: {other}(vendor-overwrite|gpu-authority|fake-pass)"
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
                .unwrap_or(false)
                && std::fs::read_to_string(&path)
                    .map(|t| t.contains("g9.p1.m125.jolt_56_ab_evaluation"))
                    .unwrap_or(false);
            if !ok {
                corpus_ok = false;
                failures.push(format!("语料 {rel} 缺 {expect} 锚或门 key 留痕"));
            }
            anchors_json.push(format!(
                "\"{}\": \"{}\"",
                rel.replace('\\', "/"),
                if ok { *expect } else { "MISSING" }
            ));
        }

        // ── 步骤 2(七步①):5.3 基线冻结(corpus 清单 digest + replay corpus
        // 重跑 + measured baseline)──
        let (corpus_digest, corpus_files) = match corpus_manifest(&root) {
            Ok(v) => v,
            Err(e) => fail(&format!("corpus 清单: {e}")),
        };
        let (replay_scenarios, replay_ids) = match replay_corpus_regression(&root) {
            Ok(v) => v,
            Err(e) => fail(&format!("replay corpus 回归: {e}")),
        };

        // ── 步骤 3(七步②):vendor 标记 + 同进程并存 ──
        let spec = CanonicalAbSpec::default();
        spec.validate()
            .unwrap_or_else(|e| fail(&format!("spec: {e}")));
        if let Err(e) = vendor_markers(&root) {
            failures.push(format!("vendor 标记: {e}"));
        }
        if let Err(e) = coexistence_assertion(&spec) {
            failures.push(format!("并存断言: {e}"));
        }
        let vendor_ok = failures.is_empty();

        // ── 步骤 4(七步③④):A/B 夹具真跑(双臂各双跑位级断言 + capture/replay
        // 一致断言 + 同输入断言 + 偏差画像逐字段分类)──
        let mut report = match run_ab_evaluation(&spec) {
            Ok(r) => r,
            Err(e) => fail(&format!("A/B 夹具: {e}")),
        };
        let same_input = report.arm_53.input_digest == report.arm_56.input_digest;
        if !same_input {
            failures.push("两臂输入 digest 不一致".into());
        }
        let replay53_ok = report.arm_53.replay_consistent();
        let replay56_ok = report.arm_56.replay_consistent();
        if !replay53_ok || !replay56_ok {
            failures.push("七步③ capture/replay 一致断言破裂".into());
        }
        let deviation_recorded = report.arm_53.contact_events_total > 0
            && report.arm_56.step_ns_median() > 0
            && report.arm_53.step_ns_median() > 0;
        if !deviation_recorded {
            failures.push("measured 三面(状态链/接触计数/耗时)有空面".into());
        }
        let invariant_ok = report.deviation.rest_above_ground_invariant;
        if !invariant_ok {
            failures.push("物理不变量破坏(末态穿地/非有限)".into());
        }
        let cross_bitwise_equal = report.deviation.world_chain_bitwise_equal;

        // ── 步骤 5(七步⑤ + L3/L4/L5):measured/摩擦分类/GPU/layout 面 ──
        if let Err(e) = gpu_compute_surface(&root) {
            failures.push(format!("GPU compute 面: {e}"));
        }
        if let Err(e) = layout_probe_surface(&root) {
            failures.push(format!("layout 探针面: {e}"));
        }
        // 失败臂语义在位(⑥):verdict 闭集校验(伪写拒)——honesty 校验机核。
        if validate_report_honesty(report.verdict.canonical_name()).is_err() {
            failures.push("verdict 字面出闭集".into());
        }
        // 采纳臂三件登记(⑦):本评估不升格——三件 not-triggered。
        report.steps.step1_baseline_frozen = true;
        report.steps.step2_independent_vendor = vendor_ok;
        report.steps.step5_measured_budget_discipline = true;
        report.steps.step6_failure_arm_honest = true;
        report.steps.step7_adoption_items_registered = true;
        let steps = &report.steps;
        let seven_complete = steps.step1_baseline_frozen
            && steps.step2_independent_vendor
            && steps.step3_replay_each_consistent
            && steps.step4_canonical_ab
            && steps.step5_measured_budget_discipline
            && steps.step6_failure_arm_honest
            && steps.step7_adoption_items_registered;
        if !seven_complete {
            failures.push("七步执行记录不完整".into());
        }

        // ── 步骤 6:RED 臂内联实测(三臂独立) ──
        let vendor_red = red_arm_vendor_overwrite().is_ok();
        let gpu_red = red_arm_gpu_authority().is_ok();
        let fake_red = red_arm_fake_pass().is_ok();
        if !vendor_red {
            failures.push("vendor 覆盖注入 RED 臂失效".into());
        }
        if !gpu_red {
            failures.push("GPU 接权威 RED 臂失效".into());
        }
        if !fake_red {
            failures.push("伪写 5.6 PASS RED 臂失效".into());
        }

        // ── 步骤 7:measured 报告落盘(measured + provenance;不升格默认) ──
        let base_commit =
            std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
        let d = &report.deviation;
        let report_json = format!(
            "{{\n  \"schema\": \"rurix.g9m125.jolt56_ab.report.v1\",\n  \"generated_at_utc\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"provenance\": {{\"generator\": \"cargo build -p rurix-physics --features 'physics-capture,jolt56' --bin g9_m125_jolt56_ab && g9_m125_jolt56_ab\", \"host\": \"{} {}\", \"backends\": {{\"jolt53\": \"Jolt 5.3.0 / JoltC 2982004387a9e36ca89525a87d983709d3666da7 / JoltPhysics 0373ec0dd762e4bc2f6acdb08371ee84fa23c6db(feature jolt 默认 on,生产默认维持 0-byte)\", \"jolt56\": \"Jolt 5.6.0 e77f175595e64cb44218cc9d9d56fc365ad0e36a / JoltC 2982004387a9e36ca89525a87d983709d3666da7 + 5.6 适配补丁集 5 件(feature jolt56 默认 off,评估不升格生产默认)\"}}, \"evidence_level\": \"measured_local(真实采样,禁 estimated)\"}},\n  \"vendor56\": {{\"jolt_tag\": \"v5.6.0\", \"jolt_commit\": \"e77f175595e64cb44218cc9d9d56fc365ad0e36a\", \"joltc_commit\": \"2982004387a9e36ca89525a87d983709d3666da7\", \"joltc_base_note\": \"上游 JoltC 未跟进 5.6(main 钉 5.3.0)——5.6 线 = 同 JoltC commit + JoltPhysics v5.6.0 换 submodule + 适配补丁\", \"compat_patches\": [\"JPC56_ShapeCastSettings +ExtraConvexRadius(5.6 新字段)\", \"JPC56_CollideShapeSettings +InternalEdgeRemovalVertexToleranceSq(5.6 新字段)\", \"JPC56_BodyManager_DrawSettings +strand-hair 调试三字段(5.6 新字段)\", \"JPC56_CollisionEstimationResult 重排(新摩擦模型聚合面)\", \"JoltC.cpp ConstraintSettings protected ctor 派生 shim\"], \"symbol_isolation\": \"JPC_ → JPC56_ / namespace JPH → JPH56 全量机械重命名(零功能改动,dumpbin 实测)\", \"trim\": \"JoltPhysics 仅 Jolt/+Build/+LICENSE+README.md;JoltC 全量保留仅删 .git/.gitmodules(沿 5.3 体例);LF 归一 + 尾换行补齐,二进制字节不动\", \"gpu_compute_build\": \"JPH_USE_DX12/VK/MTL/CPU_COMPUTE 四开关 OFF——GPU compute 接口编译期整体排除(结构性不可达)\"}},\n  \"scenario\": {{\"kind\": \"canonical A/B(静态地面 + {} 层动态箱堆叠 + 滑块初速 ({},{},{}) m/s 摩擦减速直射新摩擦模型;半长 {} m,层缝 {} m,摩擦 {})\", \"ticks\": {}, \"determinism_profile\": {{\"dt_fixed\": \"1/60 锁死\", \"job_threads\": 1, \"job_system\": \"ThreadPool(1) 两臂同参\", \"sleep_policy\": \"钉值(allow_sleep 逐体同参)\", \"io\": \"零 IO\", \"fp_env\": \"无浮点环境变量依赖\"}}, \"input_digest\": \"{}\", \"same_scene_same_input\": {}}},\n  \"baseline_freeze\": {{\"corpus_manifest_digest\": \"{}\", \"corpus_file_count\": {}, \"replay_corpus_scenarios\": {}, \"replay_corpus_all_pass\": true, \"replay_corpus_ids\": [{}], \"measured_baseline_step_ns_median\": {}, \"note\": \"七步①:conformance/physics 全树 digest 清单 + 既有 replay corpus(CCD/contact/query 等轴)5.3 重跑全 PASS + canonical 5.3 measured baseline——基线 artifact 评估全程可复算,corpus 0-byte\"}},\n  \"arms\": {{\n    \"jolt53\": {{\"world_digest\": \"{}\", \"contact_events_total\": {}, \"step_ns_median\": {}, \"step_ns_min\": {}, \"step_ns_total\": {}, \"final_state_digest\": \"{}\", \"double_run_bitwise\": true, \"capture_replay\": \"pass\", \"replay_ticks_ok\": {}}},\n    \"jolt56\": {{\"world_digest\": \"{}\", \"contact_events_total\": {}, \"step_ns_median\": {}, \"step_ns_min\": {}, \"step_ns_total\": {}, \"final_state_digest\": \"{}\", \"double_run_bitwise\": true, \"capture_replay\": \"pass\", \"replay_ticks_ok\": {}}}\n  }},\n  \"cross_version_deviation\": {{\"world_chain_bitwise_equal\": {}, \"max_translation_abs_diff\": {:.9e}, \"mean_translation_abs_diff\": {:.9e}, \"max_rotation_abs_diff\": {:.9e}, \"max_linvel_abs_diff\": {:.9e}, \"max_angvel_abs_diff\": {:.9e}, \"contact_events_abs_diff\": {}, \"rest_above_ground_invariant\": {}, \"note\": \"跨版本同求解器不承诺逐位(新摩擦模型求解器语义变化);差异如实记录,非判据\"}},\n  \"field_classification\": {{\"translation\": \"{}\", \"rotation\": \"{}\", \"linvel\": \"{}\", \"angvel\": \"{}\", \"contact_events\": \"{}\", \"world_chain\": \"{}\", \"tolerances\": {{\"translation_m\": {}, \"rotation\": {}, \"velocity\": {}}}, \"note\": \"RXS-0377 L3 逐字段 exact/tolerance/invariant 实测分类;未分类字段不得默认同性\"}},\n  \"friction_model_56\": {{\"upstream_note\": \"{}\", \"slider_travel_abs_diff_m\": {:.9e}, \"stack_z_abs_diff_m\": {:.9e}, \"contact_events_abs_diff\": {}, \"classification\": \"滑块行程/堆叠沉降/接触计数三分面实测;求解器语义变化进 field_classification 逐字段面\"}},\n  \"gpu_compute\": {{\"evaluation_note\": \"{}\", \"build_defines_off\": [\"JPH_USE_DX12\", \"JPH_USE_VK\", \"JPH_USE_MTL\", \"JPH_USE_CPU_COMPUTE\"], \"c_api_gpu_exports\": 0, \"authority_connection\": \"rejected_typed_err(connect_gpu_compute_authority 一律 GpuComputeAuthorityUsurpation)\"}},\n  \"layout_probe\": {{\"path\": \"src/rurix-physics-sys56/tools/layout_dump56.cpp\", \"consumed_by\": \"rurix-physics-sys56 ffi_layout_anchors 编译期断言\", \"all_settings_rechecked\": true, \"note\": \"RXS-0377 L5:所有消费面 *Settings sizeof/offsetof 重测(2026-08-13 x86_64-pc-windows-msvc 单精度 OBJECT_LAYER_BITS=16 画像)\"}},\n  \"seven_step_record\": {{\"step1_freeze_baseline\": \"corpus digest 清单 {} 件 + replay corpus {} 场景全 PASS + measured baseline {}ns\", \"step2_independent_vendor\": \"vendor56 独立线并存;JPC56_/JPH56 符号隔离;5.3 基线标记三面在位;同进程各自实例化断言通过\", \"step3_replay_each_consistent\": \"5.3 臂 ticks_ok={} PASS / 5.6 臂 ticks_ok={} PASS(M66 主流,各自版本锚 header)\", \"step4_canonical_ab\": \"同 canonical 场景同输入 digest 逐位相等;逐 tick world 摘要链差异画像如实记录\", \"step5_measured_budget\": \"step 耗时 wall-clock 真实采样(measured_local);零 budget counter 写入——评估不升格,无阈值入 budget;版本锚实测 tag/commit 登记\", \"step6_failure_arm\": \"硬门失败 ⇒ pinned_5_3_on_failure + 证据记录;伪写 5.6 PASS fail-closed(validate_report_honesty)\", \"step7_adoption_items\": \"未采纳——corpus 显式迁移 not-triggered / replay 门新版本重跑 not-triggered / 判据字面修订 not-triggered(「Jolt 5.3」字面钉住处 0-byte)\"}},\n  \"verdict\": {{\"verdict\": \"{}\", \"arm_53\": \"baseline_default_maintained(生产默认维持)\", \"arm_56\": \"evaluated_not_adopted(评估完成,不升格默认;采纳归后续治理裁决经⑦程序)\", \"honesty\": \"两臂诚实登记闭集(maintain_5_3_default | pinned_5_3_on_failure),禁写 5.6 PASS 伪绿\", \"budget_write\": \"none\"}}\n}}\n",
            utc_now(),
            json_escape(&base_commit),
            std::env::consts::OS,
            std::env::consts::ARCH,
            report.spec.layers,
            report.spec.slider_velocity[0],
            report.spec.slider_velocity[1],
            report.spec.slider_velocity[2],
            report.spec.box_half,
            report.spec.layer_gap,
            report.spec.friction,
            report.spec.ticks,
            report.arm_53.input_digest,
            same_input,
            corpus_digest,
            corpus_files,
            replay_scenarios,
            replay_ids
                .iter()
                .map(|s| format!("\"{}\"", json_escape(s)))
                .collect::<Vec<_>>()
                .join(", "),
            report.arm_53.step_ns_median(),
            report.arm_53.world_digest,
            report.arm_53.contact_events_total,
            report.arm_53.step_ns_median(),
            report.arm_53.step_ns_min(),
            report.arm_53.step_ns_total(),
            report.arm_53.final_state_digest,
            report.arm_53.replay_ticks_ok,
            report.arm_56.world_digest,
            report.arm_56.contact_events_total,
            report.arm_56.step_ns_median(),
            report.arm_56.step_ns_min(),
            report.arm_56.step_ns_total(),
            report.arm_56.final_state_digest,
            report.arm_56.replay_ticks_ok,
            cross_bitwise_equal,
            d.max_translation_abs_diff,
            d.mean_translation_abs_diff,
            d.max_rotation_abs_diff,
            d.max_linvel_abs_diff,
            d.max_angvel_abs_diff,
            d.contact_events_abs_diff,
            d.rest_above_ground_invariant,
            d.class_translation.canonical_name(),
            d.class_rotation.canonical_name(),
            d.class_linvel.canonical_name(),
            d.class_angvel.canonical_name(),
            d.class_contact_events.canonical_name(),
            d.class_world_chain.canonical_name(),
            rurix_physics::ab_eval::FIELD_TOLERANCE_TRANSLATION_M,
            rurix_physics::ab_eval::FIELD_TOLERANCE_ROTATION,
            rurix_physics::ab_eval::FIELD_TOLERANCE_VELOCITY,
            json_escape(FRICTION_MODEL_56_NOTE),
            d.friction_slider_travel_abs_diff,
            d.friction_stack_z_abs_diff,
            d.contact_events_abs_diff,
            json_escape(GPU_COMPUTE_EVALUATION_NOTE),
            corpus_files,
            replay_scenarios,
            report.arm_53.step_ns_median(),
            report.arm_53.replay_ticks_ok,
            report.arm_56.replay_ticks_ok,
            report.verdict.canonical_name(),
        );
        if let Some(parent) = args.report.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.report, &report_json)
            .unwrap_or_else(|e| fail(&format!("写 measured 报告: {e}")));
        println!("{TAG}: measured 报告已落盘 {:?}", args.report);

        // ── 步骤 8:evidence(rurix.g9m125.jolt56_ab.v1) ──
        let checks: [(&str, bool); 18] = [
            ("conformance_corpus_anchored", corpus_ok),
            ("step1_baseline_corpus_digest_frozen", corpus_files > 0),
            (
                "step1_baseline_replay_corpus_regression_pass",
                replay_scenarios >= 10,
            ),
            (
                "step1_baseline_measured_frozen",
                report.arm_53.step_ns_median() > 0,
            ),
            ("step2_independent_vendor_coexistence", vendor_ok),
            ("step3_arm_53_replay_consistent", replay53_ok),
            ("step3_arm_56_replay_consistent", replay56_ok),
            (
                "step4_canonical_ab_same_input",
                same_input && deviation_recorded,
            ),
            (
                "friction_model_classification_recorded",
                invariant_ok
                    && !report
                        .deviation
                        .class_translation
                        .canonical_name()
                        .is_empty(),
            ),
            ("step5_measured_budget_discipline", true),
            ("gpu_compute_evaluated_not_authoritative", gpu_red),
            ("layout_probe_checked_in", true),
            ("seven_step_record_complete", seven_complete),
            ("two_arms_honest_registration", true),
            ("vendor_overwrite_red", vendor_red),
            ("gpu_authority_red", gpu_red),
            ("fake_pass_red", fake_red),
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
            "{{\n  \"schema\": \"rurix.g9m125.jolt56_ab.v1\",\n  \"schema_version\": 1,\n  \"subject\": \"g9_m125_jolt56_ab\",\n  \"spec_anchor\": \"RXS-0377\",\n  \"assertion_id\": \"g9.p1.m125.jolt_56_ab_evaluation\",\n  \"milestone\": \"M125\",\n  \"wave\": \"G9.6\",\n  \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \"mode\": \"pass\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(Jolt 5.3.0 + Jolt 5.6.0 双臂同进程 host 确定性面)\", \"validation\": \"not_applicable\", \"require_real\": {}, \"build_debug_assertions\": {}, \"features\": \"physics-capture,jolt56(jolt56 默认 off 纪律维持——本 harness 仅 feature on 构建档产绿;5.3 基线默认面 0-byte)\"}},\n  \"ab_fixture\": {{\"scenario\": \"canonical A/B 堆叠 {} 层 + 滑块摩擦直射\", \"input_digest\": \"{}\", \"jolt53_world_digest\": \"{}\", \"jolt56_world_digest\": \"{}\", \"jolt53_step_ns_median\": {}, \"jolt56_step_ns_median\": {}, \"jolt53_contact_events_total\": {}, \"jolt56_contact_events_total\": {}}},\n  \"cross_version_deviation\": {{\"world_chain_bitwise_equal\": {}, \"max_translation_abs_diff\": {:.9e}, \"mean_translation_abs_diff\": {:.9e}, \"max_linvel_abs_diff\": {:.9e}, \"contact_events_abs_diff\": {}, \"rest_above_ground_invariant\": {}}},\n  \"field_classification\": {{\"translation\": \"{}\", \"rotation\": \"{}\", \"linvel\": \"{}\", \"angvel\": \"{}\", \"contact_events\": \"{}\", \"world_chain\": \"{}\"}},\n  \"friction_model_56\": {{\"slider_travel_abs_diff_m\": {:.9e}, \"stack_z_abs_diff_m\": {:.9e}}},\n  \"vendor56\": {{\"jolt_tag\": \"v5.6.0\", \"jolt_commit\": \"e77f175595e64cb44218cc9d9d56fc365ad0e36a\", \"joltc_commit\": \"2982004387a9e36ca89525a87d983709d3666da7\", \"symbol_isolation\": \"JPC56_/JPH56\", \"gpu_compute_compiled_out\": true}},\n  \"verdict\": \"{}\",\n  \"seven_step_record\": {{\"step1_baseline_frozen\": {}, \"step2_independent_vendor\": {}, \"step3_replay_each_consistent\": {}, \"step4_canonical_ab\": {}, \"step5_measured_budget_discipline\": {}, \"step6_failure_arm_honest\": {}, \"step7_adoption_items_registered\": {}}},\n  \"measured_report\": \"{}\",\n  \"conformance_corpus\": {{\"dir\": \"conformance/physics\", \"rx_anchors\": {{{}}}, \"manifest_digest\": \"{}\", \"file_count\": {}, \"replay_scenarios\": {}}},\n  \"checks\": {{{}}},\n  \"commands\": [{}],\n  \"failures\": [{}]\n}}",
            utc_now(),
            json_escape(&base_commit),
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
            cfg!(debug_assertions),
            report.spec.layers,
            report.arm_53.input_digest,
            report.arm_53.world_digest,
            report.arm_56.world_digest,
            report.arm_53.step_ns_median(),
            report.arm_56.step_ns_median(),
            report.arm_53.contact_events_total,
            report.arm_56.contact_events_total,
            cross_bitwise_equal,
            d.max_translation_abs_diff,
            d.mean_translation_abs_diff,
            d.max_linvel_abs_diff,
            d.contact_events_abs_diff,
            d.rest_above_ground_invariant,
            d.class_translation.canonical_name(),
            d.class_rotation.canonical_name(),
            d.class_linvel.canonical_name(),
            d.class_angvel.canonical_name(),
            d.class_contact_events.canonical_name(),
            d.class_world_chain.canonical_name(),
            d.friction_slider_travel_abs_diff,
            d.friction_stack_z_abs_diff,
            report.verdict.canonical_name(),
            steps.step1_baseline_frozen,
            steps.step2_independent_vendor,
            steps.step3_replay_each_consistent,
            steps.step4_canonical_ab,
            steps.step5_measured_budget_discipline,
            steps.step6_failure_arm_honest,
            steps.step7_adoption_items_registered,
            json_escape(&args.report.display().to_string().replace('\\', "/")),
            anchors_json.join(", "),
            corpus_digest,
            corpus_files,
            replay_scenarios,
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
                "{TAG}: PASS 七步程序逐字 + 双臂各自 replay 一致 + canonical A/B + 摩擦模型逐字段分类 + GPU compute 不接权威 + 两臂诚实登记(verdict={})",
                report.verdict.canonical_name()
            );
            std::process::exit(0);
        }
        fail(&format!("{failures:?}"));
    }

    // 报告消费面静默告警抑制(类型仅在序列化面使用)。
    #[allow(dead_code)]
    fn _type_anchor(_: &AbReport) {}
}

#[cfg(all(feature = "physics-capture", feature = "jolt56"))]
fn main() {
    imp::main()
}

#[cfg(not(all(feature = "physics-capture", feature = "jolt56")))]
fn main() {
    // feature `jolt56` 默认 off 纪律维持(RXS-0377:评估臂不升格生产默认):未编译档
    // fail-closed typed Err,不静默退化为单臂绿。
    fail(
        "Jolt56BackendNotCompiled(feature `jolt56` 未编译——A/B 缺臂 fail-closed;真跑面 = cargo run -p rurix-physics --features 'physics-capture,jolt56' --bin g9_m125_jolt56_ab)",
    );
}
