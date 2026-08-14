#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.6 M125 Jolt 5.3→5.6 升级 A/B 评估门冒烟(g9.p1.m125.jolt_56_ab_evaluation;
RFC-0024 §4.E1 + RFC-0021 §4.A4 七步程序;spec/physics.md RXS-0377;
G9_ACCEPTANCE_MAP §3 M125;G9_CONTRACT §8.1 裁决① P1 全进;两臂诚实登记——
5.3 基线维持生产默认,5.6 评估不升格,禁写 5.6 PASS 伪绿)。

host 纯 host 确定性门(device_section_state=not_applicable;harness evidence
实记 device_name=host-only〔Jolt 5.3 + Jolt 5.6 双臂同进程〕/validation=
not_applicable;jolt56 feature 默认 off 纪律维持——本门仅 feature on 构建档
产绿)。三段判据:

  host 段:rurix-physics ab_eval 2 单测逐名锚定(并存/双跑位级/各自 replay
    一致/逐字段分类 + vendor 覆盖·GPU 接权威·伪写 PASS 三 RED 面)+
    rurix-physics-sys56 单测(ffi_layout_anchors 编译期布局锚)+ 5.3 基线回归
    (cargo test 默认档全绿)+ conformance physics M125 双件语料锚定 +
    measured 报告 g9_m125_jolt56_ab.json provenance 机器核验(基线冻结面/
    vendor56 pin 与补丁集/偏差画像逐字段分类/摩擦模型专项/GPU compute 留档/
    七步记录/verdict 闭集诚实登记)。
  harness 段:持锁(gpu_device_lock)真跑 g9_m125_jolt56_ab --evidence
    (直出件落 .tmp 工作区不覆盖 evidence/ harness 直出件;schema/spec_anchor/
    assertion_id/status==pass + 18 判据闭集全真)+ --red-arm
    vendor-overwrite/gpu-authority/fake-pass 子模式独立复跑抽检。

用法:
  py -3 ci/g9_jolt_56_ab_evaluation_smoke.py --gate g9.p1.m125.jolt_56_ab_evaluation
  py -3 ci/g9_jolt_56_ab_evaluation_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_m125_jolt_56_ab_evaluation_evidence_schema.json"
REPORT_PATH = ROOT / "milestones" / "g9" / "g9_m125_jolt56_ab.json"
CORPUS_DIR = ROOT / "conformance" / "physics"
WORK_DIR = ROOT / ".tmp" / "g96_gates" / "m125"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p1.m125.jolt_56_ab_evaluation"
NUMERIC_STEP = 168
SOURCE_REF = "RFC-0024 §4.E1;spec/physics.md RXS-0377;G9_ACCEPTANCE_MAP §3 M125"
TAG = "g9_m125"
SUBJECT = "g9_m125_jolt_56_ab_evaluation"
MATRIX_ROW = "M125"

MODULE_TESTS = {
    "ab_eval": [
        "coexistence_replay_consistency_and_canonical_ab",
        "vendor_overwrite_gpu_authority_and_fake_pass_fail_closed",
    ],
}
CORPUS_FILES = [
    ("accept/jolt_ab_seven_step_minimal.rx", "RXS-0377"),
    ("reject/jolt_56_vendor_overwrite_baseline.rx", "RXS-0377"),
]
REPORT_SCHEMA = "rurix.g9m125.jolt56_ab.report.v1"
HARNESS_BIN = "g9_m125_jolt56_ab"
HARNESS_FEATURES = "physics-capture,jolt56"
HARNESS_SCHEMA = "rurix.g9m125.jolt56_ab.v1"
HARNESS_ASSERTION = "g9.p1.m125.jolt_56_ab_evaluation"
HARNESS_TAG = "G9_M125_JOLT56_AB"
HARNESS_CHECKS = [
    "conformance_corpus_anchored",
    "step1_baseline_corpus_digest_frozen",
    "step1_baseline_replay_corpus_regression_pass",
    "step1_baseline_measured_frozen",
    "step2_independent_vendor_coexistence",
    "step3_arm_53_replay_consistent",
    "step3_arm_56_replay_consistent",
    "step4_canonical_ab_same_input",
    "friction_model_classification_recorded",
    "step5_measured_budget_discipline",
    "gpu_compute_evaluated_not_authoritative",
    "layout_probe_checked_in",
    "seven_step_record_complete",
    "two_arms_honest_registration",
    "vendor_overwrite_red",
    "gpu_authority_red",
    "fake_pass_red",
    "measured_report_written",
]
RED_ARMS = ["vendor-overwrite", "gpu-authority", "fake-pass"]
FIELD_CLASS_SET = {"exact", "tolerance", "invariant"}
VERDICT_SET = {"maintain_5_3_default", "pinned_5_3_on_failure"}

CHECK_KEYS = [
    "host_module_tests_anchored",
    "baseline_53_default_regression",
    "conformance_corpus_anchored",
    "baseline_freeze_provenance",
    "vendor56_provenance_layout_probe",
    "ab_deviation_classification_recorded",
    "friction_model_special_recorded",
    "two_arms_honest_registration",
    "harness_full_pass",
    "harness_checks_closed_set_green",
    "harness_red_arm_submode_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_cmd(cmd: list[str], *, record: bool = True, timeout: int = 1800, env: dict | None = None) -> tuple[int, str]:
    print(f"[{TAG}] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    if record:
        COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(cmd), "exit_code": r.returncode})
    return r.returncode, r.stdout + r.stderr


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def is_hex(v: object, n: int) -> bool:
    return isinstance(v, str) and len(v) == n and all(c in "0123456789abcdef" for c in v)


def _load_report() -> dict | None:
    if not REPORT_PATH.is_file():
        check(False, f"缺 measured 报告 {REPORT_PATH.name}")
        return None
    try:
        return json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        check(False, f"measured 报告不可读: {e}")
        return None


# ═══════════════════════ host 段 ═══════════════════════


def host_module_tests() -> bool:
    """ab_eval 2 单测逐名锚定 + sys56 crate 单测(ffi_layout_anchors 锚)。"""
    ok_all = True
    for module, names in MODULE_TESTS.items():
        rc, blob = run_cmd(["cargo", "test", "-p", "rurix-physics", "--features", HARNESS_FEATURES, "--lib", module])
        ok = rc == 0 and "test result: ok" in blob
        for name in names:
            if not (ok and name in blob):
                check(False, f"{module} 单测 {name} 未锚定/失败")
                ok_all = False
        if not ok:
            check(False, f"cargo test -p rurix-physics --lib {module} 失败")
            ok_all = False
    rc, blob = run_cmd(["cargo", "test", "-p", "rurix-physics-sys56"])
    if not (rc == 0 and "test result: ok" in blob and "ffi_layout" not in blob.lower() or rc == 0):
        # ffi_layout_anchors 为编译期断言(测试二进制构建成功即锚定);运行时七测全绿。
        pass
    if not (rc == 0 and "test result: ok" in blob):
        check(False, "cargo test -p rurix-physics-sys56 失败(layout 锚/行为面)")
        ok_all = False
    return ok_all


def host_baseline_regression() -> bool:
    """5.3 基线门回归(未被扰动):cargo test -p rurix-physics 默认档全绿。"""
    rc, blob = run_cmd(["cargo", "test", "-p", "rurix-physics", "--lib"])
    ok = rc == 0 and "test result: ok" in blob
    if not ok:
        check(False, "5.3 基线回归:cargo test -p rurix-physics --lib 默认档失败")
    return ok


def host_conformance() -> bool:
    ok = True
    for rel, anchor in CORPUS_FILES:
        path = CORPUS_DIR / rel
        if not path.is_file():
            check(False, f"缺语料 conformance/physics/{rel}")
            ok = False
            continue
        text = path.read_text(encoding="utf-8")
        if f"//@ spec: {anchor}" not in text or GATE_KEY not in text:
            check(False, f"语料 {rel} 缺 `//@ spec: {anchor}` 锚或门 key 留痕")
            ok = False
    return ok


def host_baseline_freeze() -> bool:
    """七步①基线冻结面 provenance(corpus 清单 digest + replay corpus 重跑 +
    measured baseline)。"""
    doc = _load_report()
    if doc is None:
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"基线冻结面: {msg}")
            ok = False

    fr = doc.get("baseline_freeze", {})
    need(is_hex(fr.get("corpus_manifest_digest"), 64), "corpus_manifest_digest 非 64-hex")
    need(isinstance(fr.get("corpus_file_count"), int) and fr["corpus_file_count"] > 0,
         "corpus_file_count 非正")
    need(isinstance(fr.get("replay_corpus_scenarios"), int) and fr["replay_corpus_scenarios"] >= 10,
         "replay corpus 场景数 < 10(CCD/contact/query 轴不全)")
    need(fr.get("replay_corpus_all_pass") is True, "replay_corpus_all_pass ≠ true(5.3 基线重跑)")
    need(isinstance(fr.get("measured_baseline_step_ns_median"), int)
         and fr["measured_baseline_step_ns_median"] > 0,
         "measured_baseline_step_ns_median 非正(measured 缺失)")
    scenario = doc.get("scenario", {})
    need(scenario.get("same_scene_same_input") is True, "scenario.same_scene_same_input ≠ true")
    need(is_hex(scenario.get("input_digest"), 64), "scenario.input_digest 非 64-hex")
    profile = scenario.get("determinism_profile", {})
    need("1/60" in str(profile.get("dt_fixed", "")), "determinism_profile.dt_fixed 未锁死")
    return ok


def host_vendor56_provenance() -> bool:
    """vendor56 pin/补丁集/符号隔离/GPU compute 编译期排除 + layout 探针入库。"""
    doc = _load_report()
    if doc is None:
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"vendor56 provenance: {msg}")
            ok = False

    v = doc.get("vendor56", {})
    need(v.get("jolt_tag") == "v5.6.0", "jolt_tag ≠ v5.6.0(版本锚按实测 tag 登记)")
    need(v.get("jolt_commit") == "e77f175595e64cb44218cc9d9d56fc365ad0e36a", "jolt_commit 非实测 pin")
    need(v.get("joltc_commit") == "2982004387a9e36ca89525a87d983709d3666da7", "joltc_commit 非实测 pin")
    patches = v.get("compat_patches", [])
    need(isinstance(patches, list) and len(patches) == 5, "5.6 适配补丁集 ≠ 5 件")
    need("JPC56_" in str(v.get("symbol_isolation", "")), "符号隔离字面缺失")
    need("OFF" in str(v.get("gpu_compute_build", "")), "GPU compute 编译期排除字面缺失")
    lp = doc.get("layout_probe", {})
    need(lp.get("path") == "src/rurix-physics-sys56/tools/layout_dump56.cpp",
         "layout 探针路径漂移(未入库)")
    need(lp.get("all_settings_rechecked") is True, "all_settings_rechecked ≠ true(*Settings 重跑面)")
    need(bool(lp.get("consumed_by")), "layout 探针消费面缺失")
    # 探针源文件在树 + sys56 build.rs 四开关 OFF 字面(结构性机核)。
    probe = ROOT / "src" / "rurix-physics-sys56" / "tools" / "layout_dump56.cpp"
    need(probe.is_file(), "tools/layout_dump56.cpp 不在树")
    build_rs = ROOT / "src" / "rurix-physics-sys56" / "build.rs"
    text = build_rs.read_text(encoding="utf-8") if build_rs.is_file() else ""
    for define in ("JPH_USE_DX12", "JPH_USE_VK", "JPH_USE_MTL", "JPH_USE_CPU_COMPUTE"):
        need(f'"{define}", "OFF"' in text, f"build.rs 缺 {define}=OFF 字面")
    # 5.3 基线 vendor 0-byte 面:VENDOR.md pin 字面不动。
    v53 = ROOT / "src" / "rurix-physics-sys" / "VENDOR.md"
    text53 = v53.read_text(encoding="utf-8") if v53.is_file() else ""
    need("2982004387a9e36ca89525a87d983709d3666da7" in text53
         and "0373ec0dd762e4bc2f6acdb08371ee84fa23c6db" in text53,
         "5.3 VENDOR.md pin 字面漂移(基线 0-byte 破坏)")
    return ok


def host_deviation_classification() -> bool:
    """七步④跨版本偏差画像 + L3 逐字段 exact/tolerance/invariant 分类闭集。"""
    doc = _load_report()
    if doc is None:
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"偏差画像/分类: {msg}")
            ok = False

    dev = doc.get("cross_version_deviation", {})
    need(isinstance(dev.get("world_chain_bitwise_equal"), bool), "world_chain_bitwise_equal 非 bool(须如实记录)")
    need(isinstance(dev.get("max_translation_abs_diff"), (int, float)), "max_translation_abs_diff 缺失")
    need(isinstance(dev.get("mean_translation_abs_diff"), (int, float)), "mean_translation_abs_diff 缺失")
    need(isinstance(dev.get("max_linvel_abs_diff"), (int, float)), "max_linvel_abs_diff 缺失")
    need(isinstance(dev.get("contact_events_abs_diff"), int), "contact_events_abs_diff 缺失")
    need(dev.get("rest_above_ground_invariant") is True, "rest_above_ground_invariant ≠ true")
    fc = doc.get("field_classification", {})
    for field in ("translation", "rotation", "linvel", "angvel", "contact_events", "world_chain"):
        need(fc.get(field) in FIELD_CLASS_SET,
             f"field_classification.{field} 出闭集(未分类不得默认同性): {fc.get(field)}")
    arms = doc.get("arms", {})
    for arm in ("jolt53", "jolt56"):
        a = arms.get(arm, {})
        need(a.get("double_run_bitwise") is True, f"arms.{arm}.double_run_bitwise ≠ true(双跑位级硬断言)")
        need(a.get("capture_replay") == "pass", f"arms.{arm}.capture_replay ≠ pass(七步③)")
        need(is_hex(a.get("world_digest"), 64), f"arms.{arm}.world_digest 非 64-hex")
        need(isinstance(a.get("step_ns_median"), int) and a["step_ns_median"] > 0,
             f"arms.{arm}.step_ns_median 非正(measured 计时缺失)")
    return ok


def host_friction_special() -> bool:
    """新摩擦模型(平均接触点)重点实测专项:上游语义留档 + 三分面实测值。"""
    doc = _load_report()
    if doc is None:
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"摩擦模型专项: {msg}")
            ok = False

    fm = doc.get("friction_model_56", {})
    note_text = str(fm.get("upstream_note", ""))
    need("平均接触点" in note_text, "上游语义留档缺「平均接触点」字面")
    need("序偏向" in note_text, "上游语义留档缺「序偏向」字面(消除接触点序偏向 = 重点项价值)")
    need(isinstance(fm.get("slider_travel_abs_diff_m"), (int, float)), "slider_travel_abs_diff_m 缺失")
    need(isinstance(fm.get("stack_z_abs_diff_m"), (int, float)), "stack_z_abs_diff_m 缺失")
    need(isinstance(fm.get("contact_events_abs_diff"), int), "contact_events_abs_diff 缺失")
    return ok


def host_two_arms_honest() -> bool:
    """两臂诚实登记:verdict 闭集 + 七步记录齐 + GPU compute 只评估不接权威 +
    伪写 5.6 PASS 拒绝面 + 零 budget 写入登记。"""
    doc = _load_report()
    if doc is None:
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"两臂诚实登记: {msg}")
            ok = False

    verdict = doc.get("verdict", {})
    need(verdict.get("verdict") in VERDICT_SET, f"verdict 出闭集: {verdict.get('verdict')}")
    need("baseline_default_maintained" in str(verdict.get("arm_53", "")), "5.3 臂登记字面缺失")
    need("evaluated_not_adopted" in str(verdict.get("arm_56", "")), "5.6 臂登记字面缺失(不升格默认)")
    need(verdict.get("budget_write") == "none", "budget_write ≠ none(评估期零 budget counter 写入)")
    need("伪绿" in str(verdict.get("honesty", "")), "honesty 字面缺失(禁写 5.6 PASS 伪绿)")
    steps = doc.get("seven_step_record", {})
    for i in range(1, 8):
        key = [k for k in steps if k.startswith(f"step{i}_")]
        need(len(key) == 1 and bool(steps.get(key[0])), f"seven_step_record step{i} 缺失/空")
    need("not-triggered" in str(steps.get("step7_adoption_items", "")),
         "step7 采纳三件 not-triggered 登记缺失(本评估不升格)")
    gpu = doc.get("gpu_compute", {})
    need(gpu.get("c_api_gpu_exports") == 0, "c_api_gpu_exports ≠ 0")
    need(isinstance(gpu.get("build_defines_off"), list) and len(gpu["build_defines_off"]) == 4,
         "build_defines_off 非四开关")
    need("RD-043" in str(gpu.get("evaluation_note", "")), "GPU compute 接入门槛字面(RD-043)缺失")
    need("rejected_typed_err" in str(gpu.get("authority_connection", "")),
         "authority_connection 未登记 typed Err 拒绝面")
    return ok


# ═══════════════════════ harness 段(持锁真跑) ═══════════════════════


def build_harness() -> Path | None:
    rc, blob = run_cmd(["cargo", "build", "-p", "rurix-physics", "--features", HARNESS_FEATURES, "--bin", HARNESS_BIN])
    if rc != 0:
        check(False, f"{HARNESS_BIN} 构建失败:\n{blob[-2000:]}")
        return None
    exe = target_dir() / "debug" / (HARNESS_BIN + (".exe" if sys.platform == "win32" else ""))
    if not exe.is_file():
        check(False, f"harness 产物缺失: {exe}")
        return None
    return exe


def harness_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_BASE_COMMIT"] = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
    ).stdout.strip()
    return env


def run_harness_full(exe: Path) -> dict | None:
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    rc, out = run_cmd([str(exe), "--evidence", str(HARNESS_EVIDENCE)], timeout=1800, env=harness_env())
    doc = None
    if HARNESS_EVIDENCE.is_file():
        try:
            doc = json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            check(False, f"harness evidence 不可解析: {e}")
    if rc != 0 or f"{HARNESS_TAG}: PASS" not in out:
        check(False, f"harness 全档失败 rc={rc}:\n{out[-2000:]}")
        return None
    if doc is None:
        check(False, "harness evidence 缺失")
        return None
    if doc.get("schema") != HARNESS_SCHEMA or doc.get("spec_anchor") != "RXS-0377":
        check(False, "harness evidence schema/spec_anchor 字面不符")
    if doc.get("assertion_id") != HARNESS_ASSERTION or doc.get("status") != "pass":
        check(False, "harness evidence assertion_id/status 不符")
    if doc.get("failures") != []:
        check(False, f"harness evidence failures 非空: {doc.get('failures')}")
    return doc


def run_red_arms(exe: Path) -> bool:
    ok_all = True
    for arm in RED_ARMS:
        rc, out = run_cmd([str(exe), "--red-arm", arm], timeout=1800, env=harness_env())
        ok = rc == 0 and f"{HARNESS_TAG}: PASS red-arm {arm}" in out
        if not ok:
            check(False, f"RED 臂子模式 {arm} 未独立检出 rc={rc}: {out[-600:]}")
            ok_all = False
    return ok_all


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 11:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 11", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    synth = {k: True for k in CHECK_KEYS}
    synth.pop(CHECK_KEYS[0])
    if not (req - set(synth)):
        print(f"[{TAG}] selftest FAIL: 合成缺键未触发 schema required 红", file=sys.stderr)
        return 1
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (2 RED + 1 GREEN)")
    return 0


# ═══════════════════════ main ═══════════════════════


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}

    # host 段
    checks["host_module_tests_anchored"] = host_module_tests()
    checks["baseline_53_default_regression"] = host_baseline_regression()
    checks["conformance_corpus_anchored"] = host_conformance()
    checks["baseline_freeze_provenance"] = host_baseline_freeze()
    checks["vendor56_provenance_layout_probe"] = host_vendor56_provenance()
    checks["ab_deviation_classification_recorded"] = host_deviation_classification()
    checks["friction_model_special_recorded"] = host_friction_special()
    checks["two_arms_honest_registration"] = host_two_arms_honest()

    # harness 段(持锁串行:cargo 构建 + 全档真跑 + RED 臂子模式抽检)
    with gpu_device_lock(purpose="g9_m125 jolt_56_ab_evaluation harness 腿"):
        exe = build_harness()
        if exe:
            doc = run_harness_full(exe)
            if doc is not None and not FAILURES:
                checks["harness_full_pass"] = True
                hc = doc.get("checks", {})
                green = True
                for k in HARNESS_CHECKS:
                    if hc.get(k) is not True:
                        check(False, f"harness 判据 {k} 非 true")
                        green = False
                checks["harness_checks_closed_set_green"] = green
            checks["harness_red_arm_submode_detected"] = run_red_arms(exe)
            note("harness:七步程序逐字 + 双臂各自 replay 一致 + canonical A/B + 摩擦模型逐字段分类 + GPU compute 不接权威 + 三 RED 臂")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G9.6",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS (host 纯 host 确定性门;harness 持锁真跑 + 七步程序 + 摩擦模型专项 + 两臂诚实登记 + 三 RED 臂全绿)")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
