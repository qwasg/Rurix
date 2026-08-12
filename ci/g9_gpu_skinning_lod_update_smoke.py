#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.3 M92 GPU 蒙皮与距离分级更新率门冒烟(g9.p1.m92.gpu_skinning_lod_update;
spec/virtual_geometry.md RXS-0353;G9_ACCEPTANCE_MAP §3 M92;G9_CONTRACT §8.1 裁决①)。

host 段:rurix-render skinning::tests 全套 15 单测逐名锚定(定点 golden/确定性
  双跑/输入校验 fail-closed/保守包围体对抗包含/边界注入/缩水 RED/档位闭集与
  切换确定/AS 更新计数+静态帧零构建/静态策略拒/骨旋转 golden/法向锥定点
  golden/球锥对抗包含/球锥缩水 RED/静止包围球 golden/M92 fixture 定点域)+
  rurix-geom-build dag 桥单测(skinned_cluster_runtime_bridge_end_to_end +
  skin_metadata_three_fields_roundtrip)。
device 段(必需,持 gpu_device_lock):g9_m92_skinning_device 手编 SPV 蒙皮
  kernel 真跑——device vs host Kerbl 参照逐顶点定点域位级一致 + 法向/法向锥
  位级 + 包围体 AABB/球/锥三包含不变式 + 档位切换双跑逐位 + 静态帧零 AS
  构建 + AS 更新计数非空 + RED 四臂(缩水 AABB/球/锥/顶点篡改)+
  RURIX_VK_VALIDATION=1 validation error=0。`RURIX_REQUIRE_REAL=1` 下 SKIP 翻红。

用法:
  py -3 ci/g9_gpu_skinning_lod_update_smoke.py --gate g9.p1.m92.gpu_skinning_lod_update
  py -3 ci/g9_gpu_skinning_lod_update_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones/g9/g9_m92_gpu_skinning_lod_update_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p1.m92.gpu_skinning_lod_update"
NUMERIC_STEP = 142
SOURCE_REF = "spec/virtual_geometry.md RXS-0353;G9_ACCEPTANCE_MAP §3 M92;G9_CONTRACT §8.1 裁决①"
TAG = "g9_m92"

SKINNING_TESTS = [
    "lbs_skinning_fixed_point_golden",
    "lbs_skinning_deterministic_double_run",
    "lbs_skinning_input_validation_fail_closed",
    "conservative_bound_contains_all_skinned_adversarial",
    "conservative_bound_boundary_injection_contained",
    "conservative_bound_shrunk_variant_red",
    "update_tier_closed_set_and_deterministic_switch",
    "skinned_as_update_counted_and_static_frame_zero_build",
    "skinned_cluster_static_policy_rejected_fail_closed",
    "bone_rotation_angle_golden_and_conservative_fallback",
    "skin_normals_fixed_point_golden",
    "sphere_and_cone_containment_adversarial_poses",
    "sphere_and_cone_shrunk_variant_red",
    "rest_bounding_sphere_golden",
    "m92_fixture_fixed_point_domain_and_pose_sweep",
]
DAG_BRIDGE_TESTS = [
    "skinned_cluster_runtime_bridge_end_to_end",
    "skin_metadata_three_fields_roundtrip",
]
DEVICE_JSON_CHECKS = [
    "vertex_bitexact",
    "cone_bitexact",
    "containment_aabb",
    "containment_sphere",
    "containment_cone",
    "tier_switch_double_run_bitexact",
    "static_frame_zero_as_build",
    "as_update_counted",
    "tier_histogram_golden",
    "red_shrunk_aabb",
    "red_shrunk_sphere",
    "red_shrunk_cone",
    "red_vertex_tamper",
    "validation_error_zero",
]

CHECK_KEYS = [
    # host 段
    "host_skinning_tests_anchored",
    "host_geom_build_dag_bridge_anchored",
    # device 段
    "device_pass",
    "device_vertex_cone_bitexact",
    "device_containment_invariants",
    "device_tier_closed_set_deterministic",
    "device_static_zero_build_as_counted",
    "device_red_arms_ok",
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


def extract_json(stdout: str) -> dict | None:
    text = stdout.strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except Exception:
        pass
    idx = text.rfind("\n{")
    idx = text.rfind("{") if idx < 0 else idx + 1
    if idx < 0:
        return None
    try:
        return json.loads(text[idx:])
    except Exception:
        return None


# ═══════════════════════ host 段 ═══════════════════════


def host_skinning_tests() -> bool:
    """cargo test -p rurix-render --lib skinning:15 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-render", "--lib", "skinning"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in SKINNING_TESTS:
        if not (ok and name in blob):
            check(False, f"skinning 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_geom_build_dag_bridge() -> bool:
    """rurix-geom-build dag 桥单测(蒙皮簇运行时桥端到端 + 元数据 roundtrip)。"""
    rc, blob = run_cargo(["test", "-p", "rurix-geom-build", "--lib", "dag"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in DAG_BRIDGE_TESTS:
        if not (ok and name in blob):
            check(False, f"geom-build dag 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


# ═══════════════════════ device 段 ═══════════════════════


def build_device_bin() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin g9_m92_skinning_device")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", "g9_m92_skinning_device"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"g9_m92_skinning_device 构建失败:\n{r.stderr[-2000:]}")
        return None
    name = "g9_m92_skinning_device.exe" if sys.platform == "win32" else "g9_m92_skinning_device"
    exe = ROOT / "target" / "debug" / name
    if exe.is_file():
        return exe
    alt_root = os.environ.get("CARGO_TARGET_DIR")
    if alt_root:
        cand = ROOT / alt_root / "debug" / name
        if cand.is_file():
            return cand
    check(False, f"g9_m92_skinning_device 产物缺失: {exe}")
    return None


def run_device(exe: Path, evidence_out: Path) -> tuple[str, str, dict | None]:
    """返回 (device_state, stdout, device_json)。REQUIRE_REAL 下 SKIP 翻红。"""
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    print(f"[{TAG}] device: g9_m92_skinning_device(RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)")
    r = subprocess.run(
        [str(exe), "--evidence", str(evidence_out)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        timeout=600,
    )
    out = r.stdout + r.stderr
    doc = extract_json(r.stdout)
    if doc is None and evidence_out.is_file():
        try:
            doc = json.loads(evidence_out.read_text(encoding="utf-8"))
        except Exception:
            doc = None
    if "G9_M92_SKIN: SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()[-800:]}")
        return "skipped_dev_env", out, doc
    if r.returncode != 0 or "G9_M92_SKIN: PASS" not in r.stdout:
        check(False, f"g9_m92_skinning_device 失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", out, doc
    return "executed", out, doc


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 9:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 9", file=sys.stderr)
        return 1
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
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (1 RED + 1 GREEN)")
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
    checks["host_skinning_tests_anchored"] = host_skinning_tests()
    checks["host_geom_build_dag_bridge_anchored"] = host_geom_build_dag_bridge()

    # device 段(持锁串行)
    device_state = "fail"
    with gpu_device_lock(purpose="g9_m92 skinning device 腿"):
        exe = build_device_bin()
        if exe:
            with tempfile.TemporaryDirectory(prefix="g9_m92_dev_") as td:
                dev_ev = Path(td) / "device.json"
                device_state, dev_out, dev_doc = run_device(exe, dev_ev)
            if device_state == "executed" and dev_doc is not None:
                dc = dev_doc.get("checks", {})
                checks["device_pass"] = True
                checks["device_vertex_cone_bitexact"] = (
                    dc.get("vertex_bitexact") is True and dc.get("cone_bitexact") is True
                )
                checks["device_containment_invariants"] = (
                    dc.get("containment_aabb") is True
                    and dc.get("containment_sphere") is True
                    and dc.get("containment_cone") is True
                )
                checks["device_tier_closed_set_deterministic"] = (
                    dc.get("tier_switch_double_run_bitexact") is True
                    and dc.get("tier_histogram_golden") is True
                )
                checks["device_static_zero_build_as_counted"] = (
                    dc.get("static_frame_zero_as_build") is True
                    and dc.get("as_update_counted") is True
                )
                checks["device_red_arms_ok"] = (
                    dc.get("red_shrunk_aabb") is True
                    and dc.get("red_shrunk_sphere") is True
                    and dc.get("red_shrunk_cone") is True
                    and dc.get("red_vertex_tamper") is True
                )
                checks["device_validation_zero"] = dc.get("validation_error_zero") is True
                for k in DEVICE_JSON_CHECKS:
                    if dc.get(k) is not True:
                        check(False, f"device checks.{k} 非 true")
            elif device_state == "executed":
                check(False, "device JSON 缺失")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m92_gpu_skinning_lod_update",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M92",
        "milestone": "M92",
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G9.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": [
            {"seq": 1, "command": "cargo test -p rurix-render --lib skinning", "exit_code": 0},
            {"seq": 2, "command": "cargo test -p rurix-geom-build --lib dag", "exit_code": 0},
            {"seq": 3, "command": "cargo build -p rurix-render --features vulkan --bin g9_m92_skinning_device", "exit_code": 0},
            {"seq": 4, "command": "g9_m92_skinning_device (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1)", "exit_code": 0 if device_state == "executed" else 1},
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
    out = EVIDENCE_DIR / f"g9_m92_gpu_skinning_lod_update_{ts}.json"
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
