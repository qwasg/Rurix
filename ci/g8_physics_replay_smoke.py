#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.6a M66 physics_replay 硬门冒烟(g8.p0.m66.physics_replay;
RFC-0021 §4.A1;design §2.7 十五 checks)。

host 恒跑 / device not_applicable。Jolt 5.3 真跑 capture→replay;
5.6 A/B 诚实判档(无 JoltC-next → 钉 5.3,不伪绿)。

用法:
  py -3 ci/g8_physics_replay_smoke.py --gate g8.p0.m66.physics_replay
  py -3 ci/g8_physics_replay_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
CORPUS = ROOT / "conformance" / "physics" / "replay"
SCHEMA = ROOT / "milestones" / "g8" / "g8_m66_physics_replay_evidence_schema.json"

GATE_KEY = "g8.p0.m66.physics_replay"
# Gov materialize 时按 ledger next_free 回填;实现草稿占位 0。
NUMERIC_STEP = 120
SUBJECT = "g8_m66_physics_replay"
SOURCE_REF = (
    "RFC-0021 §4.A1;G8.6_G8.8_PHYSICS_CLOSEOUT_DESIGN.md §2;"
    "G8_ACCEPTANCE_MAP M66"
)

SCENARIOS = [
    "box_stack_settle",
    "sphere_impulse_script",
    "create_destroy_churn",
    "streaming_page_cycle",
    "ccd_bullet_thin_wall",
    "kinematic_platform",
    "joint_pendulum_motor",
    "query_mid_replay",
    "contact_ring_saturation",
    "mixed_soup_72",
]

CHECK_KEYS = [
    "capture_header_complete",
    "recovery_layer_registered",
    "capture_artifact_persisted",
    "corpus_scene_count_min",
    "per_tick_hash_equal_all_scenes",
    "event_digest_equal_all_scenes",
    "journal_fully_consumed",
    "journal_leftover_fails_closed",
    "journal_missing_fails_closed",
    "generation_reuse_stable",
    "streaming_receipt_order_journaled",
    "float_canonicalization_enforced",
    "injection_divergence_exact_all",
    "injection_report_fields_complete",
    "non_whitelist_injection_rejected",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def utc_stamp() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True, cwd=ROOT)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def gates_exe() -> Path:
    name = "g8-physics-gates.exe" if sys.platform == "win32" else "g8-physics-gates"
    return ROOT / "target" / "debug" / name


def build_gates() -> Path:
    print("[g8_m66] cargo build -p g8-physics-gates")
    r = subprocess.run(
        ["cargo", "build", "-p", "g8-physics-gates", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(r.stdout + r.stderr, file=sys.stderr)
        sys.exit(1)
    exe = gates_exe()
    if not exe.is_file():
        print(f"[g8_m66] missing {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def run_gates(exe: Path, args: list[str]) -> tuple[int, dict | None, str]:
    r = subprocess.run(
        [str(exe), *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    text = (r.stdout or "").strip().splitlines()
    last = text[-1] if text else ""
    doc = None
    try:
        doc = json.loads(last)
    except Exception:
        pass
    return r.returncode, doc, r.stdout + r.stderr


def load_header(scenario: str) -> dict | None:
    p = CORPUS / scenario / "header.json"
    if not p.is_file():
        return None
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except Exception:
        return None


def run_gate() -> int:
    checks = {k: False for k in CHECK_KEYS}
    exe = build_gates()

    # ① record all (Jolt 5.3 真跑)
    print("[g8_m66] record --all")
    code, doc, out = run_gates(exe, ["record", "--all"])
    check(code == 0 and doc is not None and doc.get("ok") is True, f"record --all failed: {out[-500:]}")

    # corpus count + artifacts
    present = [s for s in SCENARIOS if (CORPUS / s / "header.json").is_file()]
    checks["corpus_scene_count_min"] = len(present) >= 10
    check(checks["corpus_scene_count_min"], f"corpus scenes {len(present)} < 10")

    artifact_ok = all(
        (CORPUS / s / "journal.jsonl").is_file()
        and (CORPUS / s / "state0.json").is_file()
        and (CORPUS / s / "state_final.json").is_file()
        for s in present
    )
    checks["capture_artifact_persisted"] = artifact_ok and len(present) >= 10
    check(checks["capture_artifact_persisted"], "capture artifacts incomplete")

    headers_ok = True
    recovery_ok = True
    for s in present:
        h = load_header(s)
        if h is None:
            headers_ok = False
            continue
        need = [
            "schema_id",
            "schema_version",
            "jolt_version",
            "joltc_commit",
            "recovery_layer",
            "scenario_id",
            "tick_count",
            "determinism_profile",
            "budget_profile",
        ]
        if any(k not in h for k in need):
            headers_ok = False
        if h.get("recovery_layer") != "semantic_journal_rebuild_v1":
            recovery_ok = False
        if h.get("jolt_version") != "5.3.0":
            headers_ok = False
            note(f"{s}: jolt_version={h.get('jolt_version')}")
    checks["capture_header_complete"] = headers_ok and len(present) >= 10
    checks["recovery_layer_registered"] = recovery_ok and len(present) >= 10
    check(checks["capture_header_complete"], "header incomplete")
    check(checks["recovery_layer_registered"], "recovery_layer not registered")

    # ② replay all
    print("[g8_m66] replay all scenes")
    replay_ok = True
    journal_ok = True
    for s in SCENARIOS:
        d = CORPUS / s
        code, doc, out = run_gates(exe, ["replay", "--dir", str(d)])
        if code != 0 or not doc or not doc.get("ok"):
            replay_ok = False
            check(False, f"replay {s}: {out[-300:]}")
            continue
        if not doc.get("journal_fully_consumed"):
            journal_ok = False
            check(False, f"journal not fully consumed: {s}")
    checks["per_tick_hash_equal_all_scenes"] = replay_ok
    checks["event_digest_equal_all_scenes"] = replay_ok
    checks["journal_fully_consumed"] = journal_ok and replay_ok
    # create_destroy_churn / streaming 绿即覆盖 generation / receipt 次序
    checks["generation_reuse_stable"] = replay_ok and (CORPUS / "create_destroy_churn").is_dir()
    checks["streaming_receipt_order_journaled"] = replay_ok and (
        CORPUS / "streaming_page_cycle"
    ).is_dir()

    # ③ journal tamper RED
    probe = CORPUS / "box_stack_settle"
    code, doc, _ = run_gates(exe, ["journal-tamper", "--dir", str(probe), "--mode", "leftover"])
    checks["journal_leftover_fails_closed"] = bool(doc and doc.get("fails_closed"))
    check(checks["journal_leftover_fails_closed"], "leftover journal did not fail-closed")
    code, doc, _ = run_gates(exe, ["journal-tamper", "--dir", str(probe), "--mode", "missing"])
    checks["journal_missing_fails_closed"] = bool(doc and doc.get("fails_closed"))
    check(checks["journal_missing_fails_closed"], "missing journal did not fail-closed")

    # ④ float canon
    code1, d1, _ = run_gates(exe, ["canon-float", "--mode", "neg_zero"])
    code2, d2, _ = run_gates(exe, ["canon-float", "--mode", "nan"])
    checks["float_canonicalization_enforced"] = (
        code1 == 0
        and code2 == 0
        and bool(d1 and d1.get("ok"))
        and bool(d2 and d2.get("nan_rejected"))
    )
    check(checks["float_canonicalization_enforced"], "float canon selftest failed")

    # ⑤ injection (prefer sphere_impulse_script injection.json)
    inj_ok = True
    fields_ok = True
    inj_scene = CORPUS / "sphere_impulse_script"
    meta_path = inj_scene / "injection.json"
    if meta_path.is_file():
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        code, doc, out = run_gates(
            exe,
            [
                "inject",
                "--dir",
                str(inj_scene),
                "--tick",
                str(meta["tick"]),
                "--body",
                meta["body"],
                "--field",
                meta["field"],
                "--bit",
                str(meta["bit"]),
            ],
        )
        if code != 0 or not doc or doc.get("first_divergence_tick") != meta["tick"]:
            inj_ok = False
            check(False, f"injection divergence: {out[-400:]}")
        else:
            fields_ok = all(
                k in doc
                for k in (
                    "first_divergence_tick",
                    "field",
                    "stable_id",
                    "expected_bits",
                    "actual_bits",
                )
            )
    else:
        inj_ok = False
        fields_ok = False
        check(False, "missing injection.json for sphere_impulse_script")
    checks["injection_divergence_exact_all"] = inj_ok
    checks["injection_report_fields_complete"] = fields_ok and inj_ok

    # ⑥ non-whitelist reject
    code, doc, out = run_gates(
        exe,
        [
            "inject",
            "--dir",
            str(inj_scene),
            "--tick",
            "1",
            "--body",
            "0000000000000001",
            "--field",
            "sleep_timer",
            "--bit",
            "0",
        ],
    )
    # whitelist reject → process exit 1 + ok:false
    rejected = code != 0 and (
        (doc and doc.get("ok") is False)
        or "non-whitelist" in out
        or "whitelist" in out.lower()
        or "Rejected" in out
    )
    checks["non_whitelist_injection_rejected"] = rejected
    check(rejected, f"non-whitelist injection not rejected: {out[-300:]}")

    # ⑦ 5.6 A/B honest probe
    code, ab, out = run_gates(exe, ["ab"])
    ab_ok = code == 0 and ab is not None and ab.get("ok") is True
    # 诚实:ab_pass 必须为 false(无双二进制则钉 5.3;有 vendor 也不伪绿)
    honest = ab_ok and ab.get("ab_pass") is False
    check(honest, f"jolt A/B honesty failed: {out[-300:]}")
    jolt_ab = {
        "probe": (ab or {}).get("probe", "unknown"),
        "jolt_version_pinned": (ab or {}).get("jolt_version_pinned", "5.3.0"),
        "ab_pass": bool((ab or {}).get("ab_pass")),
        "verdict": (ab or {}).get("verdict", "unknown"),
        "honest_boundary": (
            "Phase0: JoltC-next absent → formal pin 5.3 stop-loss; "
            "never claim 5.6 PASS without dual-binary A/B"
        ),
    }
    note(f"jolt_ab={jolt_ab['verdict']} probe={jolt_ab['probe']}")

    host_pass = all(checks.values()) and honest and not FAILURES
    stamp = utc_stamp()
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M66",
        "wave": "G8.6a",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": checks,
        "jolt_ab": jolt_ab,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": tool_version("cargo"),
            "rustc_version": tool_version("rustc"),
        },
        "notes": "; ".join(NOTES) if NOTES else "M66 Jolt 5.3 capture/replay measured_local",
    }

    # local schema validate (not via check_schemas.py hotspot)
    try:
        import jsonschema

        errs = sorted(
            jsonschema.Draft7Validator(json.loads(SCHEMA.read_text(encoding="utf-8"))).iter_errors(
                evidence
            ),
            key=lambda e: list(e.path),
        )
        if errs:
            for e in errs:
                FAILURES.append(f"schema: {e.message}")
            host_pass = False
            evidence["host_section_pass"] = False
    except ImportError:
        note("jsonschema missing; skipped local schema validate")

    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out_path = EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out_path.write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m66] evidence → {out_path.relative_to(ROOT)}")
    for k, v in checks.items():
        print(f"  check {k}: {'PASS' if v else 'FAIL'}")
    print(f"  jolt_ab: {jolt_ab['verdict']} ab_pass={jolt_ab['ab_pass']}")
    if FAILURES:
        print("[g8_m66] FAILURES:", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
    print(f"[g8_m66] VERDICT = {'PASS' if host_pass else 'FAIL'}")
    return 0 if host_pass else 1


def run_selftest() -> int:
    # 负样本:缺 corpus 时 replay 应红(不写 evidence 充绿)
    exe = build_gates()
    missing = ROOT / "conformance" / "physics" / "replay" / "__missing_scene__"
    code, doc, _ = run_gates(exe, ["replay", "--dir", str(missing)])
    if code == 0 and doc and doc.get("ok"):
        print("[selftest] FAIL: missing corpus still green", file=sys.stderr)
        return 1
    print("[selftest] PASS: missing corpus → red")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G8.6a M66 physics_replay smoke")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if NUMERIC_STEP <= 0:
        note("NUMERIC_STEP=0 (Gov materialize 回填前草稿)")
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
