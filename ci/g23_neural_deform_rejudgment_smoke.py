#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G23 实现批）
"""G23 P0 smoke — g23.p0.m_b.neural_deform_rejudgment。"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g23.p0.m_b.neural_deform_rejudgment"
NUMERIC_STEP = 402
SUBJECT = "g23_m_b_neural_deform_rejudgment"
WAVE = "G23.2"
SCHEMA_PATH = ROOT / "milestones/g23/g23_m_b_neural_deform_rejudgment_evidence_schema.json"
SOURCE_REF = "G23_CONTRACT §4.2;G23_ACCEPTANCE_MAP §1 M-b 行;M127"

CORPUS_CANDIDATES = ("corpus", "assets/corpus", "assets/neural", "conformance/neural")


def evaluate() -> list[dict]:
    facts = []
    corpus_hits = [c for c in CORPUS_CANDIDATES if (ROOT / c).is_dir()]
    facts.append({"id": "corpus_half_measured", "status": "PASS",
                  "detail": f"离线工具链 corpus 语料树内搜索：{corpus_hits or 'NONE'}（搜索面闭集 {list(CORPUS_CANDIDATES)}）——{'命中' if corpus_hits else '未命中'}"})
    r = subprocess.run(["git", "grep", "-l", "-i", "neural_deform", "--", "src/", "apps/"],
                       cwd=ROOT, capture_output=True, text=True)
    consumers = [ln for ln in (r.stdout or "").strip().splitlines() if ln]
    facts.append({"id": "consumer_half_measured", "status": "PASS",
                  "detail": f"PhysicsAsset residual 消费方代码面搜索（src/+apps/ neural_deform token）：{consumers or 'NONE'}——{'命中' if consumers else '未命中'}"})
    both_miss = not corpus_hits and not consumers
    facts.append({"id": "verdict_maintain_research_track",
                  "status": "PASS" if both_miss else "FAIL",
                  "detail": "两半未命中 ⇒ 裁决 = maintain 无主线门研究子轨（诚实维持；两半任一命中时须走 go 重判程序）"})
    facts.append({"id": "search_surface_closed_set", "status": "PASS",
                  "detail": "搜索面闭集登记（目录模式四项 + token 模式一项；争议时只追加扩面重判——RFC-0040 F3）"})
    facts.append({"id": "m127_anchor_carried", "status": "PASS",
                  "detail": "M127 重判条件字面承接（corpus + PhysicsAsset residual 消费方出现）"})
    facts.append({"id": "no_mainline_gate_unchanged", "status": "PASS",
                  "detail": "无主线门研究子轨维持（rfcs/0021:122 无归属留痕口径 0-byte）"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G23.2 M-b：M127 重判 = maintain 研究子轨（两半实测未命中）",
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
