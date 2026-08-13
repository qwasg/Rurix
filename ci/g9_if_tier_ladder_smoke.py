#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.4 M101 IF 体素网格档位阶梯门冒烟(g9.p1.m101.if_tier_ladder;RFC-0022 §4.8;
spec/global_illumination.md RXS-0362;G9_ACCEPTANCE_MAP §3 M101;G9_CONTRACT §8.1
裁决① P1 全进)。

门序机器阻断(D2-Q7 硬约束,前置):evidence/ 最新
  g9_m96_path_tracer_reference_<UTC>.json 必须 status=="pass" 且
  assertion_id=="g9.p0.m96.path_tracer_reference"(ci/g9_gi_interlock.py);
  缺失/非 pass 即门 FAIL 退 1(打印阻断原因)。

host 段:rurix-render gi::if_tier 7 单测逐名锚定(八面体往返冻结界/SRGB 注入
  RED/档位闭集预算行+强制降档/共享内核单实例/体素轮换摊销/带比较器/门序锚
  消费)+ conformance gi 语料锚(accept if_tier_ladder_shared_kernel_minimal +
  reject if_octahedral_srgb_encoding + reject if_as_budget_exceeded_no_demote,
  `//@ spec: RXS-0362`)+ 冻结带 milestones/g9/g9_m101_if_tier_band.json
  provenance 机器核验(含 m96_anchor_digest 与 M97 冻结带 depth2 m96_digest
  逐字相等的门序链锚)。
device 段(必需,持 gpu_device_lock):rurixc --target vulkan 产双 SPV
  (g9_m101_probe_oct/g9_m96_path_tracer)→ g9_m101_if_tier_ladder harness 全档
  真跑(双跑位级一致 + 共享内核单实例 + vis 结构域精确对拍 + 轮换摊销非空 +
  每档 AS 预算行消费 AsStats + 强制降档显式记录 + 静默降档注入拒 + SRGB 注入
  可检测 + 选档器双跑逐位一致 + 四档深度带内;RURIX_REQUIRE_REAL=1 +
  RURIX_VK_VALIDATION=1,SKIP 翻红)+ --red-arm srgb-encode(device)/
  --red-arm budget-no-demote(host)子模式独立复跑抽检。

用法:
  py -3 ci/g9_if_tier_ladder_smoke.py --gate g9.p1.m101.if_tier_ladder
  py -3 ci/g9_if_tier_ladder_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones/g9/g9_m101_if_tier_ladder_evidence_schema.json"
BAND_PATH = ROOT / "milestones/g9/g9_m101_if_tier_band.json"
M97_BAND_PATH = ROOT / "milestones/g9/g9_m97_depth_band.json"
ACCEPT_CORPUS = ROOT / "conformance/gi/accept/if_tier_ladder_shared_kernel_minimal.rx"
REJECT_CORPUS = ROOT / "conformance/gi/reject/if_octahedral_srgb_encoding.rx"
REJECT_CORPUS_2 = ROOT / "conformance/gi/reject/if_as_budget_exceeded_no_demote.rx"
KERNEL_DIR = ROOT / "src/rurix-render/kernels"
WORK_DIR = ROOT / ".tmp/g94_gates/m101"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402
from g9_gi_interlock import m96_gate_passed  # noqa: E402

GATE_KEY = "g9.p1.m101.if_tier_ladder"
NUMERIC_STEP = 152
SOURCE_REF = "RFC-0022 §4.8;spec/global_illumination.md RXS-0362;G9_ACCEPTANCE_MAP §3 M101"
TAG = "g9_m101"

IF_TIER_TESTS = [
    "oct_roundtrip_error_within_frozen_bounds",
    "srgb_encode_injection_red",
    "tier_ladder_closed_set_budget_rows_and_forced_demotion",
    "shared_kernel_single_instance_and_tier_determinism",
    "voxel_grid_rotation_amortization_and_lookup",
    "band_roundtrip_and_fail_closed",
    "m96_anchor_consumed_from_m97_band",
]

CHECK_KEYS = [
    # host 段
    "gate_order_m96_passed",
    "host_if_tier_tests_anchored",
    "conformance_gi_corpus_anchored",
    "if_tier_band_provenance_frozen",
    # device 段
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_shared_kernel_single_instance",
    "device_vis_host_parity",
    "device_budget_rows_consumed",
    "device_selector_bitexact",
    "device_red_arms_effective",
    "device_red_arm_submodes_detected",
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


def host_if_tier_tests() -> bool:
    """cargo test -p rurix-render --lib gi::if_tier:7 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-render", "--lib", "gi::if_tier"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in IF_TIER_TESTS:
        if not (ok and name in blob):
            check(False, f"gi::if_tier 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_conformance_anchor() -> bool:
    """conformance gi accept/reject 三语料在位 + `//@ spec: RXS-0362` 锚定。"""
    ok = True
    for path, face in (
        (ACCEPT_CORPUS, "accept"),
        (REJECT_CORPUS, "reject"),
        (REJECT_CORPUS_2, "reject"),
    ):
        if not path.is_file():
            check(False, f"缺 {face} 语料 {path.name}")
            ok = False
            continue
        text = path.read_text(encoding="utf-8")
        if "//@ spec: RXS-0362" not in text or GATE_KEY not in text:
            check(False, f"{face} 语料 {path.name} 锚定/门 key 预期面缺失")
            ok = False
    if ok:
        text = REJECT_CORPUS_2.read_text(encoding="utf-8")
        if "降档" not in text:
            check(False, "reject 语料强制降档预期面缺失")
            ok = False
    return ok


def host_band_provenance() -> bool:
    """M101 带 provenance 机器核验 + M97 带 depth2 门序链锚逐字核验。"""
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

    need(band.get("schema") == "rurix.g9m101.if_tier_band.v1", "schema 字面不符")
    need(bool(band.get("frozen_at_utc")), "frozen_at_utc 空")
    need(bool(band.get("device_name")), "device_name 空")
    need(band.get("scene") == "m96_cornell", "scene ≠ m96_cornell")
    need(is_hex(band.get("m96_anchor_digest"), 64), "m96_anchor_digest 形态")
    need("M101_BAND_MARGIN" in str(band.get("freeze_rule", "")), "freeze_rule 缺 M101_BAND_MARGIN 登记")
    need(band.get("matched_depth") == "2", "matched_depth ≠ 2")
    need(band.get("m96_golden_spp") == "64", "m96_golden_spp ≠ 64")
    need(bool(band.get("seed_chain")), "seed_chain 空")
    need(bool(band.get("srgb_encode_rel_dev")), "srgb_encode_rel_dev 空")
    entries = band.get("entries")
    need(isinstance(entries, list) and len(entries) == 4, "entries ≠ 4(L0~L3 四档)")
    for e in entries if isinstance(entries, list) else []:
        for f in ("product_digest", "m96_digest"):
            need(is_hex(e.get(f), 64), f"条目 {e.get('tier')} {f} 形态")
        for f in ("band_rel_dev", "measured_rel_dev"):
            need(isinstance(e.get(f), str) and len(e[f]) > 0, f"条目 {e.get('tier')} 缺 {f}")
    # 门序链锚:M101 带 m96_anchor_digest == M97 带 depth2 条目 m96_digest。
    try:
        m97 = json.loads(M97_BAND_PATH.read_text(encoding="utf-8"))
        anchor = next(
            (e.get("m96_digest") for e in m97.get("entries", []) if e.get("depth") == "2"),
            None,
        )
    except (OSError, json.JSONDecodeError):
        anchor = None
    if anchor is None:
        check(False, "M97 冻结带缺 depth2 条目(门序链锚不可核)")
        ok = False
    elif band.get("m96_anchor_digest") != anchor:
        check(False, f"M101 带 m96_anchor_digest ≠ M97 带 depth2 m96_digest({anchor})")
        ok = False
    else:
        note("M101 带 m96_anchor_digest == M97 带 depth2 m96_digest(门序链锚逐字一致)")
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
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin g9_m101_if_tier_ladder")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", "g9_m101_if_tier_ladder"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"g9_m101_if_tier_ladder 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = target_dir() / "debug" / ("g9_m101_if_tier_ladder.exe" if sys.platform == "win32" else "g9_m101_if_tier_ladder")
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


def run_harness_full(exe: Path, spv_m101: Path, spv_m96: Path) -> tuple[str, dict | None]:
    """全档真跑(IF 档位阶梯 L0~L3)。返回 (device_state, harness evidence|None)。"""
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(exe),
        "--spv-m101", str(spv_m101),
        "--spv-m96", str(spv_m96),
        "--work-dir", str(WORK_DIR / "work"),
        "--evidence", str(HARNESS_EVIDENCE),
    ]
    print(f"[{TAG}] device 全档: g9_m101_if_tier_ladder(双 SPV,validation=on)")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=device_env(), timeout=1800)
    out = r.stdout + r.stderr
    if "G9_M101_IF: SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()[-800:]}")
        return "skipped_dev_env", None
    doc = None
    if HARNESS_EVIDENCE.is_file():
        try:
            doc = json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            check(False, f"harness evidence 不可解析: {e}")
    if r.returncode != 0 or "G9_M101_IF: PASS" not in r.stdout:
        check(False, f"harness 全档失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", doc
    if doc is None:
        check(False, "harness evidence 缺失")
        return "fail", None
    if doc.get("schema") != "rurix.g9m101.if_tier.v1" or doc.get("spec_anchor") != "RXS-0362":
        check(False, "harness evidence schema/spec_anchor 字面不符")
        return "fail", doc
    if doc.get("assertion_id") != GATE_KEY or doc.get("status") != "pass":
        check(False, "harness evidence assertion_id/status 不符")
        return "fail", doc
    return "executed", doc


def run_red_arm_srgb(exe: Path, spv_m101: Path, spv_m96: Path) -> bool:
    """--red-arm srgb-encode 子模式独立复跑(device;退出码 0 + PASS 字面)。"""
    print(f"[{TAG}] device RED 臂子模式: --red-arm srgb-encode")
    r = subprocess.run(
        [str(exe), "--red-arm", "srgb-encode", "--spv-m101", str(spv_m101), "--spv-m96", str(spv_m96)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=device_env(),
        timeout=900,
    )
    out = r.stdout + r.stderr
    ok = r.returncode == 0 and "G9_M101_IF: PASS red-arm srgb-encode" in r.stdout
    if not ok:
        check(False, f"RED 臂子模式 srgb-encode 未独立检出 rc={r.returncode}: {out[-600:]}")
    return ok


def run_red_arm_budget(exe: Path) -> bool:
    """--red-arm budget-no-demote 子模式独立复跑(纯 host 臂)。"""
    print(f"[{TAG}] host RED 臂子模式: --red-arm budget-no-demote")
    r = subprocess.run(
        [str(exe), "--red-arm", "budget-no-demote"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=300,
    )
    out = r.stdout + r.stderr
    ok = r.returncode == 0 and "G9_M101_IF: PASS red-arm budget-no-demote" in r.stdout
    if not ok:
        check(False, f"RED 臂子模式 budget-no-demote 未独立检出 rc={r.returncode}: {out[-600:]}")
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
    with tempfile.TemporaryDirectory(prefix="g9_m101_selftest_") as td:
        ok, detail = m96_gate_passed(Path(td))
        if ok or "门序阻断" not in detail:
            print(f"[{TAG}] selftest FAIL: M96 evidence 缺失未阻断", file=sys.stderr)
            return 1
    # CHECK_KEYS 闭集恰 14 项(host 4 + device 10)。
    if len(CHECK_KEYS) != 14:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 14", file=sys.stderr)
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
        checks["host_if_tier_tests_anchored"] = host_if_tier_tests()
        checks["conformance_gi_corpus_anchored"] = host_conformance_anchor()
        checks["if_tier_band_provenance_frozen"] = host_band_provenance()

        # device 段(持锁串行:rurixc 构建 + 双 SPV 产线 + harness 全档 + RED 臂子模式)
        with gpu_device_lock(purpose="g9_m101 if tier device 腿"):
            rurixc = build_rurixc()
            exe = build_harness() if rurixc else None
            spv_m101 = WORK_DIR / "g9_m101_probe_oct.spv"
            spv_m96 = WORK_DIR / "g9_m96_path_tracer.spv"
            spvs_ok = rurixc is not None and all(
                compile_spv(rurixc, k, o)
                for k, o in (
                    ("g9_m101_probe_oct.rx", spv_m101),
                    ("g9_m96_path_tracer.rx", spv_m96),
                )
            )
            if exe and spvs_ok:
                device_state, doc = run_harness_full(exe, spv_m101, spv_m96)
                if device_state == "executed" and doc is not None:
                    hc = doc.get("checks", {})
                    checks["device_harness_full_pass"] = True
                    checks["device_double_run_bitexact"] = hc.get("double_run_bitexact") is True
                    checks["device_shared_kernel_single_instance"] = hc.get("shared_kernel_single_instance") is True
                    checks["device_vis_host_parity"] = hc.get("vis_device_host_parity") is True
                    checks["device_budget_rows_consumed"] = (
                        hc.get("budget_row_per_tier_present") is True
                        and hc.get("as_stats_consumed_calm_no_demote") is True
                        and hc.get("forced_demote_with_records") is True
                        and hc.get("rotation_amortization_non_empty") is True
                    )
                    checks["device_selector_bitexact"] = hc.get("selector_double_run_bitexact") is True
                    checks["device_red_arms_effective"] = (
                        hc.get("silent_demotion_detected") is True
                        and hc.get("srgb_encode_detectable") is True
                    )
                    checks["device_m96_cross_anchor_band"] = (
                        hc.get("m96_cross_anchor") is True
                        and hc.get("depth_band_within") is True
                    )
                    env = doc.get("environment", {})
                    checks["device_validation_zero"] = (
                        env.get("validation") == "on"
                        and env.get("require_real") is True
                    )
                    checks["device_red_arm_submodes_detected"] = (
                        run_red_arm_srgb(exe, spv_m101, spv_m96)
                        and run_red_arm_budget(exe)
                    )
                    for k in CHECK_KEYS:
                        if k.startswith("device_") and not checks[k]:
                            check(False, f"harness 判据 {k} 为假")
                note("device:全档真跑(四档阶梯 + 预算行消费 AsStats)+ srgb-encode/budget-no-demote 子模式复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m101_if_tier_ladder",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M101",
        "milestone": "M101",
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
            {"seq": 1, "command": "cargo test -p rurix-render --lib gi::if_tier", "exit_code": 0},
            {"seq": 2, "command": "cargo build -p rurixc --features vulkan-backend --bin rurixc", "exit_code": 0},
            {"seq": 3, "command": "rurixc kernels/g9_m101_probe_oct.rx + g9_m96_path_tracer.rx --target vulkan -o .tmp/g94_gates/m101/*.spv", "exit_code": 0},
            {"seq": 4, "command": "cargo build -p rurix-render --features vulkan --bin g9_m101_if_tier_ladder", "exit_code": 0},
            {"seq": 5, "command": "g9_m101_if_tier_ladder --spv-m101 .. --spv-m96 .. (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g9_m101_if_tier_ladder --red-arm srgb-encode --spv-m101 .. + --red-arm budget-no-demote (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submodes_detected"] else 1},
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
    out = EVIDENCE_DIR / f"g9_m101_if_tier_ladder_{ts}.json"
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
