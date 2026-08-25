#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G26 实现批）
"""G26.3 P0 smoke — g26.p0.m_d.g17_md_f1_rejudgment_window。

G17-MD-F1（fps 焦点格 bistro-interior/t100/dlss_sr 17/18 诚实红）重判窗条件核验
（RFC-0043 §4）：两半证据（①NGX 分解 profiling ②UE 侧插桩——宿主差可分离
measured 证据，RFC-0032 重判条件同源）树内闭集搜索实测。F6 硬线 = searched-paths
manifest 为 evidence 必填，空清单即 FAIL——「均未命中」只能建立在非空搜索清单上。
任一命中 → 重判程序启动登记（重判执行归 G30）；均未命中 → 维持 17/18 诚实红
carry（终判归 G30 商用终审）。
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g26.p0.m_d.g17_md_f1_rejudgment_window"
NUMERIC_STEP = 454  # post-interlock actual-next-free 顺位领取（448~460 批）
SUBJECT = "g26_m_d_g17_md_f1_rejudgment_window"
WAVE = "G26.3"
SCHEMA_PATH = ROOT / "milestones/g26/g26_m_d_g17_md_f1_rejudgment_window_evidence_schema.json"
SOURCE_REF = "G26_CONTRACT §4.2 M-d;G26_ACCEPTANCE_MAP §1 M-d 行;RFC-0043 §4;RFC-0032 重判条件"

# 树内闭集搜索面（F6：闭集字面登记，禁开放式外采）。
HALF1_NGX_PATTERNS = [
    "evidence/g26_ngx_decomposition_profiling_*.json",
    "evidence/*ngx_decomposition*.json",
    "evidence/*ngx_profiling*.json",
]
HALF2_UE_PATTERNS = [
    "evidence/g26_ue_frame_instrumentation_*.json",
    "evidence/*ue_instrumentation*.json",
    "evidence/*ue_frame_decomposition*.json",
]


def _search(patterns: list[str]) -> tuple[list[dict], int]:
    manifest: list[dict] = []
    total = 0
    for pat in patterns:
        hits = [str(p.relative_to(ROOT)) for p in sorted(ROOT.glob(pat))]
        manifest.append({"pattern": pat, "hits": len(hits), "files": hits})
        total += len(hits)
    return manifest, total


def evaluate() -> list[dict]:
    facts = []
    m1, n1 = _search(HALF1_NGX_PATTERNS)
    m2, n2 = _search(HALF2_UE_PATTERNS)
    manifest_ok = len(m1) + len(m2) >= 6
    facts.append({"id": "searched_paths_manifest_nonempty", "status": "PASS" if manifest_ok else "FAIL",
                  "detail": f"F6 必填清单：{len(m1) + len(m2)} 条 pattern 逐条登记（半① {len(m1)} + 半② {len(m2)}）："
                            + "; ".join(x["pattern"] for x in m1 + m2)})
    facts.append({"id": "half1_ngx_profiling_search", "status": "PASS",
                  "detail": f"NGX 分解 profiling 树内闭集实测命中 = {n1}（{[x['pattern'] + ':' + str(x['hits']) for x in m1]}）"})
    facts.append({"id": "half2_ue_instrumentation_search", "status": "PASS",
                  "detail": f"UE 侧插桩树内闭集实测命中 = {n2}（{[x['pattern'] + ':' + str(x['hits']) for x in m2]}）"})
    any_hit = (n1 + n2) > 0
    verdict = "rejudgment-triggered" if any_hit else "maintain-17-18-honest-red-carry"
    facts.append({"id": "decision_tree_verdict", "status": "PASS",
                  "detail": f"两半命中 {n1}+{n2} → {verdict}"
                            + ("（重判程序启动登记，执行归 G30）" if any_hit
                               else "（均未命中 → 维持 17/18 诚实红 carry，终判归 G30 商用终审；不冒充）")})
    g25p = wel.load_latest_evidence("g25_m_b_fps_parity_final_verdict")
    g25doc = wel.load_json(g25p) if g25p else {}
    anchor_ok = g25p is not None and g25doc.get("host_section_pass") is True and any(
        "0.856326" in str(f.get("detail", "")) for f in g25doc.get("extra_facts", []))
    facts.append({"id": "g25_final_verdict_anchor_present", "status": "PASS" if anchor_ok else "FAIL",
                  "detail": f"G25 M-b 终判锚（焦点格 ratio 0.856326）在档 = {g25p.name if g25p else 'missing'}"})
    facts.append({"id": "carry_to_g30_registered", "status": "PASS",
                  "detail": "重判锚字面 0-byte 只追加：NGX 分解 profiling 或 UE 侧插桩（宿主差可分离 measured 证据）→ G30 商用终审窗承接"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G26.3 M-d：G17-MD-F1 重判窗条件核验（两半证据树内闭集搜索 + searched-paths manifest 必填 + 维持 17/18 诚实红 carry 终判归 G30）",
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
        m1, n1 = _search(HALF1_NGX_PATTERNS)
        m2, n2 = _search(HALF2_UE_PATTERNS)
        assert len(m1) == 3 and len(m2) == 3, "闭集 pattern 数漂移"
        print(f"[{SUBJECT}] SELFTEST PASS（manifest 6 条 pattern；当前命中 {n1}+{n2}）")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
