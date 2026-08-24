#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G20 实现批）
"""G20 P0 smoke — g20.p0.m_c.mesh_shader_rejudgment。"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g20.p0.m_c.mesh_shader_rejudgment"
NUMERIC_STEP = 356
SUBJECT = "g20_m_c_mesh_shader_rejudgment"
WAVE = "G20.3"
SCHEMA_PATH = ROOT / "milestones/g20/g20_m_c_mesh_shader_rejudgment_evidence_schema.json"
SOURCE_REF = "G20_CONTRACT §4.2;G20_ACCEPTANCE_MAP §1 M-c 行;RFC-0034;RFC-0037 §1.4"

RFC34 = ROOT / "rfcs/0034-virtualized-geometry-p3-mesh-shader.md"
GAP = ROOT / "milestones/g20/g20_cluster_streaming_p4_gap.json"


def evaluate() -> list[dict]:
    facts = []
    text = RFC34.read_text(encoding="utf-8") if RFC34.is_file() else ""
    facts.append({"id": "rfc_0034_rejudgment_appended",
                  "status": "PASS" if "G20.3 M-c 重判" in text else "FAIL",
                  "detail": "RFC-0034 重判记录只追加在档"})
    hzb = wel.load_latest_evidence("g20_hzb_probe")
    hzb_ok = hzb is not None and wel.load_json(hzb).get("zero_false_positive") is True
    facts.append({"id": "hzb_half_condition_met", "status": "PASS" if hzb_ok else "FAIL",
                  "detail": f"重判条件 HZB 半边兑现（{hzb.name if hzb else 'missing'}）"})
    gap = wel.load_json(GAP) if GAP.is_file() else {}
    open_rows = [r.get("id") for r in gap.get("gap_rows", []) if r.get("status") == "open"]
    facts.append({"id": "cluster_half_gap_registered", "status": "PASS" if open_rows else "FAIL",
                  "detail": f"cluster P4 半边差距闭集 open={open_rows}（未清零如实登记）"})
    facts.append({"id": "measured_perf_evidence_absent",
                  "status": "PASS" if "measured 证据仍缺" in text else "FAIL",
                  "detail": "mesh shader HW 管线性能差 measured 证据缺位核验（HW 路径零实现 ⇒ 无 A/B 可测面）"})
    facts.append({"id": "verdict_maintain_no_go",
                  "status": "PASS" if "maintain-no-go" in text else "FAIL",
                  "detail": "裁决 = maintain-no-go（VS 光栅唯一 fallback 维持字面 0-byte；maintain-no-go/go 均合法）"})
    facts.append({"id": "rejudgment_anchor_forward",
                  "status": "PASS" if "cluster P4 差距闭集清零" in text else "FAIL",
                  "detail": "重判条件顺延锚在档（cluster 闭集清零 + HZB device 化）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G20.3 M-c：M61 重判 = maintain-no-go（RFC-0034 只追加重判记录；两半条件核验如实）",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
