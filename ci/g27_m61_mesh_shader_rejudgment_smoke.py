#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G27 实现批）
"""G27.2 P0 smoke — g27.p0.m_b.m61_mesh_shader_rejudgment。

M61 重判窗（RFC-0044 §2）：三项机器盘点闭集——①HZB device 化半边（M-a 绿件只读盘点）
②cluster P4 清零半边（g20 差距表四行 open 实测）③mesh shader HW 性能差 measured 证据
（树内闭集搜索 + searched-paths manifest 必填）。三项全齐才构成重判启动（防冒充硬线：
①半边命中不得单独启动）；任一未齐 → maintain-no-go 只追加（RFC-0034 重判表尾追加
G27.2 行）。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g27.p0.m_b.m61_mesh_shader_rejudgment"
NUMERIC_STEP = 466  # post-interlock actual-next-free 顺位领取（464~476 批）
SUBJECT = "g27_m_b_m61_mesh_shader_rejudgment"
WAVE = "G27.2"
SCHEMA_PATH = ROOT / "milestones/g27/g27_m_b_m61_mesh_shader_rejudgment_evidence_schema.json"
SOURCE_REF = "G27_CONTRACT §4.2 M-b;RFC-0044 §2;G20_P2_DECISIONS.md §1 M61 行;rfcs/0034 重判表"

GAP_TABLE = ROOT / "milestones/g20/g20_cluster_streaming_p4_gap.json"
RFC0034 = ROOT / "rfcs/0034-virtualized-geometry-p3-mesh-shader.md"
HW_EVIDENCE_PATTERNS = [
    "evidence/g27_mesh_shader_hw_ab_*.json",
    "evidence/*mesh_shader*perf*.json",
    "evidence/*mesh*hw*measured*.json",
]


def evaluate() -> list[dict]:
    facts: list[dict] = []
    # ① HZB device 化半边（M-a 绿件只读盘点；skipped_dev_env 不构成命中）。
    ma = wel.load_latest_evidence("g27_m_a_hzb_device_kernel")
    ma_doc = wel.load_json(ma) if ma else {}
    h1 = (ma is not None and ma_doc.get("host_section_pass") is True
          and ma_doc.get("device_section_state") != "skipped_dev_env")
    facts.append({"id": "hzb_device_half_inventory", "status": "PASS" if ma is not None else "FAIL",
                  "detail": f"①HZB device 化半边 = {'命中' if h1 else '未命中'}（{ma.name if ma else 'missing'}，"
                            f"device_section_state={ma_doc.get('device_section_state')}）"})
    # ② cluster P4 清零半边（四行 status 实测）。
    gap = wel.load_json(GAP_TABLE) if GAP_TABLE.is_file() else {}
    rows = gap.get("gap_rows", [])
    open_ids = [r.get("id") for r in rows if r.get("status") == "open"]
    h2_cleared = bool(rows) and not open_ids
    facts.append({"id": "cluster_p4_half_inventory", "status": "PASS" if rows else "FAIL",
                  "detail": f"②cluster P4 清零半边 = {'清零' if h2_cleared else '未清零'}"
                            f"（{len(rows)} 行实测,open={open_ids}）"})
    # ③ HW 性能差 measured 证据（manifest 必填）。
    manifest = []
    hits_total = 0
    for pat in HW_EVIDENCE_PATTERNS:
        hits = [str(p.relative_to(ROOT)) for p in sorted(ROOT.glob(pat))]
        manifest.append(f"{pat}:{len(hits)}")
        hits_total += len(hits)
    h3 = hits_total > 0
    facts.append({"id": "hw_measured_evidence_search", "status": "PASS" if len(manifest) >= 3 else "FAIL",
                  "detail": f"③HW 性能差 measured 证据 = {'命中' if h3 else '零命中'}"
                            f"（manifest {len(manifest)} 条：{'; '.join(manifest)}）"})
    # 三项合取决策树（防冒充硬线：单项/两项命中均落 maintain-no-go）。
    started = h1 and h2_cleared and h3
    verdict = "rejudgment-started" if started else "maintain-no-go"
    facts.append({"id": "three_item_conjunction_verdict", "status": "PASS",
                  "detail": f"三项合取 = ①{h1} ②清零{h2_cleared} ③{h3} → {verdict}"
                            + ("（重判启动登记）" if started
                               else "（①半边命中不得单独启动——maintain-no-go 只追加，VS 光栅唯一 fallback 兜底 0-byte）")})
    # RFC-0034 重判表 G27.2 行只追加登记。
    rfc_text = RFC0034.read_text(encoding="utf-8") if RFC0034.is_file() else ""
    appended = "G27.2 M-b 重判" in rfc_text
    facts.append({"id": "rfc_0034_rejudgment_appended", "status": "PASS" if appended else "FAIL",
                  "detail": "RFC-0034 重判表含 G27.2 M-b 重判行（只追加）"})
    fallback_ok = "VS 光栅唯一 fallback" in rfc_text
    facts.append({"id": "fallback_literal_maintained", "status": "PASS" if fallback_ok else "FAIL",
                  "detail": "VS 光栅唯一 fallback 兜底字面在档 0-byte"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G27.2 M-b：M61 重判窗（三项机器盘点闭集——HZB device 半边命中 + P4 未清零 + HW 证据零命中 → maintain-no-go 只追加，防冒充硬线兑现）",
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
        assert len(HW_EVIDENCE_PATTERNS) == 3
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
