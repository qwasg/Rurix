#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.3 波次聚合门 g8.wave.3.exit(CI_GATES §5;步骤号合入时领取)。

只读汇总五个 G8.3 P0(M79/M80/M81/M01/M04)+ M83 go + M85 phase_g8_3_pass。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。

用法:
  py -3 ci/g8_wave3_exit_check.py --gate g8.wave.3.exit
  py -3 ci/g8_wave3_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.3.exit"
# 合入时按 ledger next_free 回填;脚手架先占位,Gov materialize 校准。
NUMERIC_STEP = 0
SUBJECT = "g8_wave3_exit"
WAVE = "G8.3"
SOURCE_REF = (
    "CI_GATES §5;G8_CONTRACT G-G8-5;G8.3_G8.4 design §4.5;"
    "five P0 + M83 + M85 phase_g8_3; M01/M04 ABI frozen anchors"
)
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave3_exit_evidence_schema.json"
SPEC_GEOM = ROOT / "spec" / "geometry_pages.md"
GOLDEN_DIR = ROOT / "tests" / "geom_pages" / "golden"

REQUIRED_GATES: list[tuple[str, str]] = [
    ("g8.p0.m79.asset_determinism", "g8_m79_asset_determinism"),
    ("g8.p0.m80.ddc_content_address", "g8_m80_ddc_content_address"),
    ("g8.p0.m81.gltf_import", "g8_m81_gltf_import"),
    ("g8.p0.m01.meshlet_page_builder", "g8_m01_meshlet_page_builder"),
    ("g8.p0.m04.page_format_abi", "g8_m04_page_format_abi"),
    ("g8.p1.m83.texture_transcode", "g8_m83_texture_transcode"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def m85_phase_g83_ok(evidence_dir: Path | None = None) -> tuple[bool, str]:
    ed = evidence_dir if evidence_dir is not None else wel.EVIDENCE_DIR
    latest = wel.load_latest_evidence("g8_m85_shader_manifest_ddc", evidence_dir=ed)
    if latest is None:
        return False, "missing g8_m85_shader_manifest_ddc evidence"
    try:
        data = json.loads(latest.read_text(encoding="utf-8"))
    except Exception as e:
        return False, f"m85 evidence unreadable: {e}"
    if data.get("symbolic_gate_key") != "g8.p0.m85.shader_manifest_ddc":
        return False, "m85 gate key mismatch"
    if not data.get("phase_g8_3_pass"):
        return False, f"phase_g8_3_pass={data.get('phase_g8_3_pass')!r} (require true)"
    # g8.2 腿亦须真(互不代绿)
    if not data.get("phase_g8_2_pass"):
        return False, "phase_g8_2_pass must remain true alongside g8.3"
    return True, f"m85 both phases pass ({latest.name})"


def abi_frozen_anchors() -> tuple[bool, str]:
    if not SPEC_GEOM.is_file():
        return False, "spec/geometry_pages.md missing"
    text = SPEC_GEOM.read_text(encoding="utf-8")
    has_m01 = "RXPL" in text or "AP-PAGE" in text or "logical" in text.lower()
    has_m04 = "RXPD" in text or "RXPM" in text or "AP-PAGE-DISK" in text or "AP-PAGE-MEM" in text
    if not (has_m01 and has_m04):
        return False, "geometry_pages.md missing M01/M04 ABI freeze anchors"
    if not GOLDEN_DIR.is_dir():
        return False, "tests/geom_pages/golden missing"
    goldens = list(GOLDEN_DIR.glob("*"))
    if not goldens:
        return False, "geom_pages golden dir empty"
    return True, f"ABI anchors + {len(goldens)} golden path(s)"


def collect_extra_facts(evidence_dir: Path | None = None) -> list[dict]:
    facts: list[dict] = []
    ok, detail = m85_phase_g83_ok(evidence_dir)
    facts.append(_fact("m85_phase_g8_3_pass", ok, detail))
    ok2, detail2 = abi_frozen_anchors()
    facts.append(_fact("m01_m04_abi_frozen_anchors", ok2, detail2))
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    if NUMERIC_STEP <= 0:
        print(
            "[g8_wave3] FAIL: NUMERIC_STEP 尚未由 Gov 校准(合入时回填)",
            file=sys.stderr,
        )
        return 1
    rows = [
        wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir)
        for key, prefix in REQUIRED_GATES
    ]
    extras = collect_extra_facts(evidence_dir)
    notes = (
        "implemented: M79/M80/M81/M01/M04 + M83 + M85.g8.3; "
        "aggregate read-only; ABI freeze anchors required"
    )
    code, _path = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=rows,
        extra_facts=extras,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes=notes,
        host_section_pass=True,
    )
    return code


def run_selftest() -> int:
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile

    # 临时绕过 NUMERIC_STEP 守卫做负样本(仅自检)
    global NUMERIC_STEP
    saved = NUMERIC_STEP
    NUMERIC_STEP = max(saved, 1)
    try:
        with tempfile.TemporaryDirectory(prefix="g8_wave3_selftest_") as td:
            code = run_gate(evidence_dir=Path(td))
            if code == 0:
                print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
                return 1
            print("[selftest] PASS: 缺 evidence → 红")
    finally:
        NUMERIC_STEP = saved
    print("[selftest] 负样本-only OK(正样本待五门+M83+M85.g8.3 齐后复跑)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G8.3 wave3.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
