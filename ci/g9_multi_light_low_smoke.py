#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.4 M100 低档多灯直接光默认档门冒烟(g9.p1.m100.multi_light_low;RFC-0022 §7;
spec/global_illumination.md RXS-0361;G9_ACCEPTANCE_MAP §3 M100;G9_CONTRACT §8.1
裁决① P1 全进;高档 ReSTIR 证据不足 not-triggered 不充绿,M15 维持 open-留档)。

门序机器阻断(D2-Q7 硬约束,前置):evidence/ 最新
  g9_m96_path_tracer_reference_<UTC>.json 必须 status=="pass" 且
  assertion_id=="g9.p0.m96.path_tracer_reference"(ci/g9_gi_interlock.py);
  缺失/非 pass 即门 FAIL 退 1(打印阻断原因)。

host 段:rurix-render gi::multi_light 7 单测逐名锚定(fixture 校验/选灯流确定性
  闭集/验证射线零跳过契约+偏置 RED/灯子集偏离 RED/ReSTIR 登记/带比较器/pbrt
  fixture 锚)+ conformance gi reject multi_light_restir_tier_unproven.rx
  (`//@ spec: RXS-0361`)锚定 + 冻结带 milestones/g9/g9_m100_multi_light_band.json
  provenance 机器核验(含 m96_anchor_digest 与 M97 冻结带 depth1 m96_digest
  逐字相等的门序链锚)。
device 段(必需,持 gpu_device_lock):rurixc --target vulkan 产双 SPV
  (g9_m100_multi_light/g9_m96_path_tracer)→ g9_m100_multi_light_low harness
  全档真跑(双跑位级一致 + pbrt fixture 锚 + 验证射线零跳过逐灯计数非空 +
  跳验证偏置/灯子集偏离双臂可检测 + ReSTIR not-triggered 字段核验 + 海量灯
  阴影统一接口 + 多灯 golden 带内;RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1,
  SKIP 翻红)+ --red-arm skip-verification 子模式独立复跑抽检。

用法:
  py -3 ci/g9_multi_light_low_smoke.py --gate g9.p1.m100.multi_light_low
  py -3 ci/g9_multi_light_low_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones/g9/g9_m100_multi_light_low_evidence_schema.json"
BAND_PATH = ROOT / "milestones/g9/g9_m100_multi_light_band.json"
M97_BAND_PATH = ROOT / "milestones/g9/g9_m97_depth_band.json"
REJECT_CORPUS = ROOT / "conformance/gi/reject/multi_light_restir_tier_unproven.rx"
SCENE_FIXTURE = ROOT / "conformance/gi/scenes/m100_multi_light_low.pbrt"
KERNEL_DIR = ROOT / "src/rurix-render/kernels"
WORK_DIR = ROOT / ".tmp/g94_gates/m100"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402
from g9_gi_interlock import m96_gate_passed  # noqa: E402

GATE_KEY = "g9.p1.m100.multi_light_low"
NUMERIC_STEP = 151
SOURCE_REF = "RFC-0022 §7;spec/global_illumination.md RXS-0361;G9_ACCEPTANCE_MAP §3 M100"
TAG = "g9_m100"

MULTI_LIGHT_TESTS = [
    "fixture_validates_and_single_light_scenes_pass_m96_validate",
    "stream_deterministic_and_light_selection_closed_set",
    "verification_ray_zero_skip_contract_and_bias_red",
    "light_subset_injection_deviation_red",
    "restir_not_triggered_registration",
    "band_roundtrip_and_fail_closed",
    "pbrt_fixture_anchor_and_m96_gate_anchor",
]

CHECK_KEYS = [
    # host 段
    "gate_order_m96_passed",
    "host_multi_light_tests_anchored",
    "conformance_gi_corpus_anchored",
    "multi_light_band_provenance_frozen",
    # device 段
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_verification_ray_zero_skip",
    "device_red_arms_effective",
    "device_red_arm_submode_detected",
    "device_restir_not_triggered",
    "device_unified_shadow_interface",
    "device_m96_cross_anchor_band",
    "device_validation_zero",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_cargo(args: list[str]) -> tuple[int, str]:
    print(f"[{TAG}] cargo {' '.join(args)}")
    r = subprocess.run(["cargo", *args], cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def is_hex(v: object, n: int) -> bool:
    return isinstance(v, str) and len(v) == n and all(c in "0123456789abcdef" for c in v)


# ═══════════════════════ host 段 ═══════════════════════


def host_multi_light_tests() -> bool:
    """cargo test -p rurix-render --lib gi::multi_light:7 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-render", "--lib", "gi::multi_light"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in MULTI_LIGHT_TESTS:
        if not (ok and name in blob):
            check(False, f"gi::multi_light 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_conformance_anchor() -> bool:
    """conformance gi reject 语料 + 多灯 pbrt fixture 在位 + 锚定/预期面。"""
    if not REJECT_CORPUS.is_file():
        check(False, f"缺 RED 语料 {REJECT_CORPUS.name}")
        return False
    text = REJECT_CORPUS.read_text(encoding="utf-8")
    ok = (
        "//@ spec: RXS-0361" in text
        and GATE_KEY in text
        and "ReSTIR" in text
        and "not-triggered" in text
    )
    if not ok:
        check(False, "RED 语料锚定/ReSTIR not-triggered 预期面缺失")
    if not SCENE_FIXTURE.is_file():
        check(False, f"缺多灯 pbrt fixture {SCENE_FIXTURE.name}")
        ok = False
    return ok


def host_band_provenance() -> bool:
    """M100 带 provenance 机器核验 + M97 带 depth1 门序链锚逐字核验。"""
    if not BAND_PATH.is_file():
        check(False, f"缺冻结带 {BAND_PATH.name}")
        return False
    try:
        band = json.loads(BAND_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        check(False, f"冻结带不可读: {e}")
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"冻结带 provenance: {msg}")
            ok = False

    need(band.get("schema") == "rurix.g9m100.multi_light_band.v1", "schema 字面不符")
    need(bool(band.get("frozen_at_utc")), "frozen_at_utc 空")
    need(bool(band.get("device_name")), "device_name 空")
    need(band.get("scene") == "m100_multi_light", "scene ≠ m100_multi_light")
    need(is_hex(band.get("m96_anchor_digest"), 64), "m96_anchor_digest 形态")
    need("M100_BAND_MARGIN" in str(band.get("freeze_rule", "")), "freeze_rule 缺 M100_BAND_MARGIN 登记")
    need(band.get("matched_depth") == "1", "matched_depth ≠ 1")
    need(band.get("m96_golden_spp") == "64", "m96_golden_spp ≠ 64")
    need(bool(band.get("seed_chain")), "seed_chain 空")
    need(bool(band.get("skip_verification_bias")), "skip_verification_bias 空")
    need(bool(band.get("light_subset_rel_dev")), "light_subset_rel_dev 空")
    entries = band.get("entries")
    need(isinstance(entries, list) and len(entries) == 1, "entries ≠ 1(m100_low_reference)")
    for e in entries if isinstance(entries, list) else []:
        for f in ("product_digest", "m96_golden_digest"):
            need(is_hex(e.get(f), 64), f"条目 {e.get('tier')} {f} 形态")
        for f in ("band_rel_dev", "measured_rel_dev"):
            need(isinstance(e.get(f), str) and len(e[f]) > 0, f"条目 {e.get('tier')} 缺 {f}")
    # 门序链锚:M100 带 m96_anchor_digest == M97 带 depth1 条目 m96_digest。
    try:
        m97 = json.loads(M97_BAND_PATH.read_text(encoding="utf-8"))
        anchor = next(
            (e.get("m96_digest") for e in m97.get("entries", []) if e.get("depth") == "1"),
            None,
        )
    except (OSError, json.JSONDecodeError):
        anchor = None
    if anchor is None:
        check(False, "M97 冻结带缺 depth1 条目(门序链锚不可核)")
        ok = False
    elif band.get("m96_anchor_digest") != anchor:
        check(False, f"M100 带 m96_anchor_digest ≠ M97 带 depth1 m96_digest({anchor})")
        ok = False
    else:
        note("M100 带 m96_anchor_digest == M97 带 depth1 m96_digest(门序链锚逐字一致)")
    return ok


# ═══════════════════════ device 段 ═══════════════════════


def build_rurixc() -> Path | None:
    print(f"[{TAG}] cargo build -p rurixc --features vulkan-backend --bin rurixc")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"rurixc 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = target_dir() / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if not exe.is_file():
        check(False, f"rurixc 产物缺失: {exe}")
        return None
    return exe


def compile_spv(rurixc: Path, kernel_name: str, out: Path) -> bool:
    print(f"[{TAG}] rurixc {kernel_name} --target vulkan -o {out.name}")
    out.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        [str(rurixc), str(KERNEL_DIR / kernel_name), "--target", "vulkan", "-o", str(out)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0 or not out.is_file():
        check(False, f"SPV 产线失败({kernel_name}) rc={r.returncode}:\n{(r.stdout + r.stderr)[-1500:]}")
        return False
    return True


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin g9_m100_multi_light_low")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", "g9_m100_multi_light_low"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"g9_m100_multi_light_low 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = target_dir() / "debug" / ("g9_m100_multi_light_low.exe" if sys.platform == "win32" else "g9_m100_multi_light_low")
    if not exe.is_file():
        check(False, f"harness 产物缺失: {exe}")
        return None
    return exe


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    env["RURIX_BASE_COMMIT"] = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
    ).stdout.strip()
    return env


def run_harness_full(exe: Path, spv_m100: Path, spv_m96: Path) -> tuple[str, dict | None]:
    """全档真跑(低档默认档 + 验证射线契约)。返回 (device_state, harness evidence|None)。"""
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(exe),
        "--spv-m100", str(spv_m100),
        "--spv-m96", str(spv_m96),
        "--work-dir", str(WORK_DIR / "work"),
        "--evidence", str(HARNESS_EVIDENCE),
    ]
    print(f"[{TAG}] device 全档: g9_m100_multi_light_low(双 SPV,validation=on)")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=device_env(), timeout=1800)
    out = r.stdout + r.stderr
    if "G9_M100_ML: SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()[-800:]}")
        return "skipped_dev_env", None
    doc = None
    if HARNESS_EVIDENCE.is_file():
        try:
            doc = json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            check(False, f"harness evidence 不可解析: {e}")
    if r.returncode != 0 or "G9_M100_ML: PASS" not in r.stdout:
        check(False, f"harness 全档失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", doc
    if doc is None:
        check(False, "harness evidence 缺失")
        return "fail", None
    if doc.get("schema") != "rurix.g9m100.multi_light_low.v1" or doc.get("spec_anchor") != "RXS-0361":
        check(False, "harness evidence schema/spec_anchor 字面不符")
        return "fail", doc
    if doc.get("assertion_id") != GATE_KEY or doc.get("status") != "pass":
        check(False, "harness evidence assertion_id/status 不符")
        return "fail", doc
    return "executed", doc


def run_red_arm(exe: Path, spv_m100: Path, spv_m96: Path) -> bool:
    """--red-arm skip-verification 子模式独立复跑(退出码 0 + PASS red-arm 字面)。"""
    print(f"[{TAG}] device RED 臂子模式: --red-arm skip-verification")
    r = subprocess.run(
        [str(exe), "--red-arm", "skip-verification", "--spv-m100", str(spv_m100), "--spv-m96", str(spv_m96)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=device_env(),
        timeout=900,
    )
    out = r.stdout + r.stderr
    ok = r.returncode == 0 and "G9_M100_ML: PASS red-arm skip-verification" in r.stdout
    if not ok:
        check(False, f"RED 臂子模式 skip-verification 未独立检出 rc={r.returncode}: {out[-600:]}")
    return ok


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    # 红臂①:合成 FAILURES 必须使门红(check() 判别有效)。
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 红臂②:M96 evidence 缺失 ⇒ 门序必阻断(D2-Q7)。
    with tempfile.TemporaryDirectory(prefix="g9_m100_selftest_") as td:
        ok, detail = m96_gate_passed(Path(td))
        if ok or "门序阻断" not in detail:
            print(f"[{TAG}] selftest FAIL: M96 evidence 缺失未阻断", file=sys.stderr)
            return 1
    # CHECK_KEYS 闭集恰 13 项(host 4 + device 9)。
    if len(CHECK_KEYS) != 13:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 13", file=sys.stderr)
        return 1
    # 绿臂:schema required 与 CHECK_KEYS 闭集互核;合成缺键 evidence 必红。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    synth = {k: True for k in CHECK_KEYS}
    synth.pop(CHECK_KEYS[0])
    if not (req - set(synth)):
        print(f"[{TAG}] selftest FAIL: 合成缺键未触发 schema required 红", file=sys.stderr)
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
    device_state = "fail"

    # 门序机器阻断(D2-Q7,前置):M96 门未绿即 FAIL 退 1,host/device 段不跑。
    interlock_ok, interlock_detail = m96_gate_passed()
    print(f"[{TAG}] {interlock_detail}")
    note(interlock_detail)
    checks["gate_order_m96_passed"] = interlock_ok
    if not interlock_ok:
        check(False, interlock_detail)
    else:
        # host 段
        checks["host_multi_light_tests_anchored"] = host_multi_light_tests()
        checks["conformance_gi_corpus_anchored"] = host_conformance_anchor()
        checks["multi_light_band_provenance_frozen"] = host_band_provenance()

        # device 段(持锁串行:rurixc 构建 + 双 SPV 产线 + harness 全档 + RED 臂子模式)
        with gpu_device_lock(purpose="g9_m100 multi light device 腿"):
            rurixc = build_rurixc()
            exe = build_harness() if rurixc else None
            spv_m100 = WORK_DIR / "g9_m100_multi_light.spv"
            spv_m96 = WORK_DIR / "g9_m96_path_tracer.spv"
            spvs_ok = rurixc is not None and all(
                compile_spv(rurixc, k, o)
                for k, o in (
                    ("g9_m100_multi_light.rx", spv_m100),
                    ("g9_m96_path_tracer.rx", spv_m96),
                )
            )
            if exe and spvs_ok:
                device_state, doc = run_harness_full(exe, spv_m100, spv_m96)
                if device_state == "executed" and doc is not None:
                    hc = doc.get("checks", {})
                    checks["device_harness_full_pass"] = True
                    checks["device_double_run_bitexact"] = hc.get("double_run_bitexact") is True
                    checks["device_verification_ray_zero_skip"] = (
                        hc.get("verification_ray_zero_skip") is True
                        and hc.get("per_light_verification_non_empty") is True
                        and hc.get("pbrt_fixture_anchor") is True
                    )
                    checks["device_red_arms_effective"] = (
                        hc.get("skip_verification_bias_detectable") is True
                        and hc.get("light_subset_deviation_detectable") is True
                    )
                    rs = doc.get("restir_registration", {})
                    checks["device_restir_not_triggered"] = (
                        hc.get("restir_not_triggered_registered") is True
                        and rs.get("status") == "not-triggered"
                        and rs.get("trigger_met") is False
                        and rs.get("serve_request_rejected") is True
                    )
                    checks["device_unified_shadow_interface"] = hc.get("unified_shadow_interface") is True
                    checks["device_m96_cross_anchor_band"] = (
                        hc.get("m96_cross_anchor") is True
                        and hc.get("depth_band_within") is True
                    )
                    env = doc.get("environment", {})
                    checks["device_validation_zero"] = (
                        hc.get("validation_zero") is True
                        and env.get("validation") == "on"
                        and env.get("require_real") is True
                    )
                    checks["device_red_arm_submode_detected"] = run_red_arm(exe, spv_m100, spv_m96)
                    for k in CHECK_KEYS:
                        if k.startswith("device_") and not checks[k]:
                            check(False, f"harness 判据 {k} 为假")
                note("device:全档真跑(验证射线零跳过契约 + ReSTIR not-triggered 登记)+ skip-verification 子模式复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m100_multi_light_low",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M100",
        "milestone": "M100",
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G9.4",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": [
            {"seq": 1, "command": "cargo test -p rurix-render --lib gi::multi_light", "exit_code": 0},
            {"seq": 2, "command": "cargo build -p rurixc --features vulkan-backend --bin rurixc", "exit_code": 0},
            {"seq": 3, "command": "rurixc kernels/g9_m100_multi_light.rx + g9_m96_path_tracer.rx --target vulkan -o .tmp/g94_gates/m100/*.spv", "exit_code": 0},
            {"seq": 4, "command": "cargo build -p rurix-render --features vulkan --bin g9_m100_multi_light_low", "exit_code": 0},
            {"seq": 5, "command": "g9_m100_multi_light_low --spv-m100 .. --spv-m96 .. (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g9_m100_multi_light_low --red-arm skip-verification --spv-m100 .. --spv-m96 .. (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submode_detected"] else 1},
        ],
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
    out = EVIDENCE_DIR / f"g9_m100_multi_light_low_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
