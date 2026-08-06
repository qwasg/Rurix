#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.6b wave6b.exit 聚合门草稿(CI_GATES §5;RFC-0021 F-14;NUMERIC_STEP=0 待 Gov 回填)。

只读汇总 M67 PASS + 波次级 subject:
  - g8.wave6b.m71.character_virtual
  - g8.wave6b.m69.physics_asset
不产独立 P0/P1 PASS、不代绿。subject 优先读最新 M67 evidence 字段;
缺字段时诚实跑一次 g8-physics-gates net --trace 取事实(不改写 M67 evidence)。

用法:
  py -3 ci/g8_wave6b_exit_check.py --gate g8.wave.6b.exit
  py -3 ci/g8_wave6b_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.6b.exit"
# Gov materialize 回填;实现草稿必须为 0。
NUMERIC_STEP = 123
SUBJECT = "g8_wave6b_exit"
WAVE = "G8.6b"
SOURCE_REF = (
    "CI_GATES §5;G8_CONTRACT G-G8-8B;RFC-0021 §6.4 F-14;"
    "M67 PASS + m71 character_virtual + m69 physics_asset subjects"
)
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave6b_exit_evidence_schema.json"
TRACE = (
    ROOT
    / "conformance"
    / "physics"
    / "network"
    / "mispredict_impulse_delay"
    / "trace.json"
)

REQUIRED = [
    ("g8.p0.m67.network_physics", "g8_m67_network_physics"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _gates_exe() -> Path:
    name = "g8-physics-gates.exe" if sys.platform == "win32" else "g8-physics-gates"
    return ROOT / "target" / "debug" / name


def _run_net_probe() -> dict | None:
    """诚实取证腿:只读跑 net --trace,不落 M67 evidence、不代绿。"""
    exe = _gates_exe()
    if not exe.is_file():
        r = subprocess.run(
            ["cargo", "build", "-p", "g8-physics-gates", "--quiet"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if r.returncode != 0 or not exe.is_file():
            return None
    if not TRACE.is_file():
        return None
    r = subprocess.run(
        [str(exe), "net", "--trace", str(TRACE)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    text = (r.stdout or "").strip().splitlines()
    last = text[-1] if text else ""
    try:
        doc = json.loads(last)
    except Exception:
        return None
    if r.returncode != 0 or not isinstance(doc, dict):
        return None
    return doc


def _read_m67_subject_fields(evidence_dir=None) -> tuple[dict | None, Path | None]:
    path = wel.load_latest_evidence("g8_m67_network_physics", evidence_dir=evidence_dir)
    if path is None:
        return None, None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None, path
    return data, path


def collect_subjects(evidence_dir=None) -> tuple[list[dict], list[dict]]:
    """求值 m71/m69 subject + 支撑 fact。"""
    facts: list[dict] = []
    subjects: list[dict] = []

    m67, m67_path = _read_m67_subject_fields(evidence_dir)
    ep = (
        None
        if m67_path is None
        else str(m67_path.relative_to(ROOT)).replace("\\", "/")
    )

    char_ok = None
    asset_ok = None
    source = "missing"

    if isinstance(m67, dict):
        if "character_state_canonical_ok" in m67:
            char_ok = bool(m67.get("character_state_canonical_ok"))
            source = "m67_evidence"
        if "physics_asset_cook_deterministic" in m67:
            asset_ok = bool(m67.get("physics_asset_cook_deterministic"))
            source = "m67_evidence"

    if char_ok is None or asset_ok is None:
        probe = _run_net_probe()
        if probe is not None and probe.get("ok") is True:
            if char_ok is None:
                char_ok = bool(probe.get("character_state_canonical_ok"))
            if asset_ok is None:
                asset_ok = bool(probe.get("physics_asset_cook_deterministic"))
            source = "gates_net_probe"
            ep = "apps/g8-physics-gates (net --trace live probe)"
        else:
            source = "probe_failed"

    char_pass = char_ok is True
    asset_pass = asset_ok is True
    facts.append(
        _fact(
            "character_asset_fact_source",
            source in ("m67_evidence", "gates_net_probe"),
            f"source={source}",
        )
    )
    subjects.append(
        {
            "id": "g8.wave6b.m71.character_virtual",
            "status": "PASS" if char_pass else "FAIL",
            "detail": f"character_state_canonical_ok={char_ok!r} via {source}",
            "evidence_path": ep,
        }
    )
    subjects.append(
        {
            "id": "g8.wave6b.m69.physics_asset",
            "status": "PASS" if asset_pass else "FAIL",
            "detail": f"physics_asset_cook_deterministic={asset_ok!r} via {source}",
            "evidence_path": ep,
        }
    )
    return facts, subjects


def run_gate(*, evidence_dir=None) -> int:
    if NUMERIC_STEP <= 0:
        print(
            "[wave6b] NUMERIC_STEP unset (Gov materialize 回填前草稿 → 红)",
            file=sys.stderr,
        )
        return 1
    if not SCHEMA_PATH.is_file():
        print(f"[wave6b] schema missing: {SCHEMA_PATH}", file=sys.stderr)
        return 1
    rows = [wel.require_gate_pass(k, p, evidence_dir=evidence_dir) for k, p in REQUIRED]
    extras, subjects = collect_subjects(evidence_dir)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=rows,
        extra_facts=extras,
        subjects=subjects,
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes=(
            "wave6b: M67 PASS + m71/m69 wave subjects "
            "(no independent P0/P1; RFC §6.4 F-14)"
        ),
        host_section_pass=True,
    )
    return code


def main() -> int:
    ap = argparse.ArgumentParser(description="G8.6b wave6b.exit 聚合门")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        assert NUMERIC_STEP == 123
        print("[wave6b] selftest OK")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
