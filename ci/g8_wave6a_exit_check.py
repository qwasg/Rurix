#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.6a wave6a.exit 聚合门草稿(CI_GATES §5;NUMERIC_STEP=0 待 Gov 回填)。

只读汇总 M66 PASS + M73 A/B 诚实判档(钉 5.3 止损臂亦算波次 subject 完成)。
不重跑 smoke、不代绿、不碰治理热点。

用法:
  py -3 ci/g8_wave6a_exit_check.py --gate g8.wave.6a.exit
  py -3 ci/g8_wave6a_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.6a.exit"
NUMERIC_STEP = 121
SUBJECT = "g8_wave6a_exit"
WAVE = "G8.6a"
SOURCE_REF = (
    "CI_GATES §5;G8_CONTRACT G-G8-8A;design §2.6/§7;"
    "M66 PASS + Jolt 5.3 corpus + 5.6 A/B honest pin"
)
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave6a_exit_evidence_schema.json"

REQUIRED = [
    ("g8.p0.m66.physics_replay", "g8_m66_physics_replay"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def m73_honest_ab(evidence_dir=None) -> tuple[bool, str, dict]:
    """波次 subject g8.wave6a.m73.jolt_ab:只读 M66 evidence 内 jolt_ab 段。"""
    path = wel.load_latest_evidence("g8_m66_physics_replay", evidence_dir=evidence_dir)
    if path is None:
        return False, "missing M66 evidence", {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        return False, f"M66 evidence unreadable: {e}", {}
    ab = data.get("jolt_ab") or {}
    # 诚实边界:ab_pass 不得为 true(无双二进制 A/B);钉 5.3 止损臂为正式成功
    pinned = ab.get("jolt_version_pinned") == "5.3.0"
    not_fake = ab.get("ab_pass") is False
    verdict = ab.get("verdict") or ""
    ok = pinned and not_fake and bool(verdict)
    detail = f"verdict={verdict!r} probe={ab.get('probe')!r} ab_pass={ab.get('ab_pass')}"
    subject = {
        "id": "g8.wave6a.m73.jolt_ab",
        "status": "PASS" if ok else "FAIL",
        "detail": detail,
        "evidence_path": str(path.relative_to(ROOT)).replace("\\", "/"),
    }
    return ok, detail, subject


def collect_extra(evidence_dir=None) -> tuple[list[dict], list[dict]]:
    facts = []
    ok, detail, subj = m73_honest_ab(evidence_dir)
    facts.append(_fact("m73_jolt_ab_honest", ok, detail))
    # corpus 先于 5.6:M66 checks.corpus_scene_count_min
    m66 = wel.load_latest_evidence("g8_m66_physics_replay", evidence_dir=evidence_dir)
    corpus_ok = False
    cdetail = "missing m66"
    if m66:
        d = json.loads(m66.read_text(encoding="utf-8"))
        corpus_ok = bool((d.get("checks") or {}).get("corpus_scene_count_min"))
        cdetail = f"corpus_scene_count_min={corpus_ok}"
    facts.append(_fact("jolt_53_corpus_built_first", corpus_ok, cdetail))
    subjects = [subj] if subj else []
    return facts, subjects


def run_gate(*, evidence_dir=None) -> int:
    if NUMERIC_STEP <= 0:
        print(
            "[wave6a] NUMERIC_STEP unset (Gov materialize 回填前草稿 → 红)",
            file=sys.stderr,
        )
        return 1
    if not SCHEMA_PATH.is_file():
        print(f"[wave6a] schema missing: {SCHEMA_PATH}", file=sys.stderr)
        return 1
    rows = [wel.require_gate_pass(k, p, evidence_dir=evidence_dir) for k, p in REQUIRED]
    extras, subjects = collect_extra(evidence_dir)
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
        notes="wave6a: M66 PASS + M73 honest pin_5_3_stop_loss",
        host_section_pass=True,
    )
    return code


def main() -> int:
    ap = argparse.ArgumentParser(description="G8.6a wave6a.exit 聚合门")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        assert NUMERIC_STEP == 121
        print("[wave6a] selftest OK")
        return 0
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
