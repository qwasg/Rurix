#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G27 实现批）
"""G27.3 P0 smoke — g27.p0.m_c.cluster_p4_gap_rejudgment。

cluster P4 差距闭集重判（RFC-0044 §3）：四行逐行 reeval（现面实现痕迹树内实测）+
P4-2 依赖解除事实登记（M-a 绿件 ⇒ reeval_anchor 半命中；登记≠该行兑现）+ 产物
milestones/g27/g27_cluster_p4_rejudgment.json（g20 原表 0-byte 不回写）+ RD-039
history 只追加（append-only 机核 = 互锁同律 vs G27.0 ref）。
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from g27_interlock_check import G27_0_IMMUTABLE_REF, check_deferred_append_only, _git_show_file  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g27.p0.m_c.cluster_p4_gap_rejudgment"
NUMERIC_STEP = 468  # post-interlock actual-next-free 顺位领取（464~476 批）
SUBJECT = "g27_m_c_cluster_p4_gap_rejudgment"
WAVE = "G27.3"
SCHEMA_PATH = ROOT / "milestones/g27/g27_m_c_cluster_p4_gap_rejudgment_evidence_schema.json"
SOURCE_REF = "G27_CONTRACT §4.2 M-c;RFC-0044 §3;milestones/g20/g20_cluster_streaming_p4_gap.json"

GAP_TABLE = ROOT / "milestones/g20/g20_cluster_streaming_p4_gap.json"
OUT_JSON = ROOT / "milestones/g27/g27_cluster_p4_rejudgment.json"
DEFERRED = ROOT / "registry/deferred.json"

# 逐行现面检索面（RFC-0044 §3.3 字面：cluster 专属实现痕迹——既有纹理/资源页式流送
# 现面〔streaming/pool.rs 等，g20 差距表 current_surface 登记〕不构成 cluster 载荷兑现，
# 检索 pattern 一律 cluster 限定以防误判）。
ROW_SEARCH = {
    "P4-1": ["src/rurix-render/src/streaming/*cluster*.rs", "src/rurix-render/src/geometry/cluster_page*.rs"],
    "P4-2": ["src/rurix-render/src/streaming/*cluster_feedback*.rs", "src/rurix-render/src/geometry/*cluster*request*.rs"],
    "P4-3": ["src/rurix-render/src/geometry/*lod_residency*.rs", "src/rurix-render/src/streaming/*cluster_lod*.rs"],
    "P4-4": ["src/rurix-render/src/streaming/*cluster_priority*.rs", "src/rurix-render/src/streaming/*cluster_io*.rs"],
}
ROW_TOKENS = {
    "P4-1": "cluster_page",
    "P4-2": "cluster_feedback",
    "P4-3": "residency_lod_cut",
    "P4-4": "cluster_io_priority",
}


def _row_impl_found(rid: str) -> tuple[bool, list[str]]:
    manifest: list[str] = []
    found = False
    for pat in ROW_SEARCH[rid]:
        hits = [str(p.relative_to(ROOT)) for p in sorted(ROOT.glob(pat))]
        manifest.append(f"{pat}:{len(hits)}")
        if hits:
            found = True
    # token 检索：streaming 四模块内 cluster 载荷 token（现面 = 纹理/资源页式，cluster 载荷零实现预期）。
    token = ROW_TOKENS[rid]
    tok_hits = 0
    for f in sorted((ROOT / "src/rurix-render/src/streaming").glob("*.rs")):
        if token in f.read_text(encoding="utf-8"):
            tok_hits += 1
    manifest.append(f"token:{token}:{tok_hits}")
    return found or tok_hits > 0, manifest


def materialize_rejudgment() -> dict:
    gap = wel.load_json(GAP_TABLE)
    ma = wel.load_latest_evidence("g27_m_a_hzb_device_kernel")
    ma_doc = wel.load_json(ma) if ma else {}
    dep_release = (ma is not None and ma_doc.get("host_section_pass") is True
                   and ma_doc.get("device_section_state") != "skipped_dev_env")
    rows_out = []
    for r in gap.get("gap_rows", []):
        rid = r["id"]
        found, manifest = _row_impl_found(rid)
        rows_out.append({
            "id": rid,
            "gap": r.get("gap"),
            "g20_anchor": r.get("anchor"),
            "impl_surface_found": found,
            "search_manifest": manifest,
            "disposition": "closed-go" if found else "maintain-open",
            "basis": ("现面实现痕迹树内实测命中" if found
                      else "现面零实现树内实测（检索清单在档）——维持 open 如实登记不冒充"),
        })
    doc = {
        "schema": "rurix.g27.cluster_p4_rejudgment.v1",
        "source_table": "milestones/g20/g20_cluster_streaming_p4_gap.json（0-byte 只读，原始锚字面不动）",
        "dependency_release_fact": {
            "anchor": "HZB device 化落地 + 剔除 pass 反馈链出现（表级 reeval_anchor 字面）",
            "hzb_device_landed": dep_release,
            "note": ("M-a 绿件 ⇒ reeval_anchor 半边命中（HZB device 化落地）——P4-2 依赖面本期解除的事实登记；"
                     "登记≠该行兑现（剔除 pass 反馈链另半边本期不出现，生产接线 out-of-scope）；"
                     "「HZB device 化落地」= 金字塔构建与保守测试 device 化的单件事实，不构成 RD-039「HZB 两阶段」分项整体兑现"),
            "evidence": ma.name if ma else "missing",
        },
        "rows": rows_out,
        "verdict": ("all-cleared" if all(r["disposition"] == "closed-go" for r in rows_out)
                    else "maintain-open-four-rows" if all(r["disposition"] == "maintain-open" for r in rows_out)
                    else "partial"),
    }
    OUT_JSON.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    return doc


def evaluate() -> list[dict]:
    facts: list[dict] = []
    r = subprocess.run(["git", "diff", "--quiet", "g26-closed", "--",
                        "milestones/g20/g20_cluster_streaming_p4_gap.json"],
                       cwd=ROOT, capture_output=True)
    facts.append({"id": "gap_table_readonly_0byte", "status": "PASS" if r.returncode == 0 else "FAIL",
                  "detail": "g20 差距表 vs g26-closed 0-byte（原始锚不回写）"})
    doc = materialize_rejudgment()
    dep = doc["dependency_release_fact"]
    facts.append({"id": "dependency_release_registered", "status": "PASS" if dep["hzb_device_landed"] else "FAIL",
                  "detail": f"P4-2 依赖解除事实登记：hzb_device_landed={dep['hzb_device_landed']}（{dep['evidence']}；登记≠兑现）"})
    rows = doc["rows"]
    facts.append({"id": "four_rows_reeval_manifest", "status": "PASS" if len(rows) == 4 and all(
        r_["search_manifest"] for r_ in rows) else "FAIL",
        "detail": f"四行逐行 reeval + 检索清单在档（{[r_['id'] for r_ in rows]}）"})
    legal = all(r_["disposition"] in ("closed-go", "maintain-open") for r_ in rows)
    facts.append({"id": "rows_disposition_honest", "status": "PASS" if legal else "FAIL",
                  "detail": f"verdict={doc['verdict']}：" + "; ".join(f"{r_['id']}={r_['disposition']}" for r_ in rows)})
    rd = {}
    if DEFERRED.is_file():
        for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", []):
            if e.get("id") == "RD-039":
                rd = e
    hist_ok = any("G27.3" in (h.get("event") or "") for h in rd.get("history", []))
    facts.append({"id": "rd039_history_appended", "status": "PASS" if hist_ok else "FAIL",
                  "detail": "RD-039 history 含 G27.3 重判只追加登记（断档口径注明）"})
    base_text = _git_show_file(ROOT, G27_0_IMMUTABLE_REF, "registry/deferred.json")
    base_doc = json.loads(base_text) if base_text else None
    cur_doc = json.loads(DEFERRED.read_text(encoding="utf-8")) if DEFERRED.is_file() else None
    findings = check_deferred_append_only(base_doc, cur_doc)
    facts.append({"id": "rd039_append_only_mechanized", "status": "PASS" if findings == [] else "FAIL",
                  "detail": "append-only 机核（vs G27.0 ref：四字段 0-byte + history 前缀相等）"
                            + ("" if not findings else f"；违例 {findings[:2]}")})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G27.3 M-c：cluster P4 四行重判（依赖解除事实登记 + 逐行零实现实测维持 open + RD-039 history 只追加 + g20 原表 0-byte）",
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
        assert set(ROW_SEARCH) == {"P4-1", "P4-2", "P4-3", "P4-4"}
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
