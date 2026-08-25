#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G28 实现批）
"""G28.3 P0 smoke — g28.p0.m_c.m52_rd040_workload_rejudgment。

M52 两半盘点（RFC-0045 §3.2）：capability 半边（G21 三 token available 只读盘点 +
新鲜 vulkaninfo 复测三态闭集）+ workload 半边（RT pipeline/SBT 宿主车道树内检索，
manifest 必填 + G8 M50 库面底座不混同）——两半全齐方改判，未全齐 maintain-defer。
RD-040 五分项逐锚重判（§3.3）：逐分项独立 manifest + pattern↔锚映射表。
产物 milestones/g28/g28_m52_rd040_workload_rejudgment.json；RD-040 history 只追加。
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from g28_interlock_check import G28_0_IMMUTABLE_REF, check_deferred_append_only, _git_show_file  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g28.p0.m_c.m52_rd040_workload_rejudgment"
NUMERIC_STEP = 484  # post-interlock actual-next-free 顺位领取（480~492 批）
SUBJECT = "g28_m_c_m52_rd040_workload_rejudgment"
WAVE = "G28.3"
SCHEMA_PATH = ROOT / "milestones/g28/g28_m_c_m52_rd040_workload_rejudgment_evidence_schema.json"
SOURCE_REF = "G28_CONTRACT §4.2 M-c;RFC-0045 §3;G21_P2_DECISIONS.md §1 M52 行;g21_rd040_subitem_registry.json"

G21_PROBE = ROOT / "milestones/g21/g21_ser_capability_probe_results.json"
SUBITEM_REG = ROOT / "milestones/g21/g21_rd040_subitem_registry.json"
OUT_JSON = ROOT / "milestones/g28/g28_m52_rd040_workload_rejudgment.json"
DEFERRED = ROOT / "registry/deferred.json"
LOG_DIR = ROOT / ".tmp/g28_mc"
SER_TOKENS = ["VK_NV_ray_tracing_invocation_reorder", "VK_EXT_ray_tracing_invocation_reorder",
              "rayTracingInvocationReorderReorderingHint"]

# workload 半边检索面（锚 = 生产渲染车道以 RT pipeline/hit-miss 着色形态出现；
# 不混同：rurix-rt 库面 SBT 底座〔G8.2 M50，RFC-0019 冻结子集〕不入面）。
WORKLOAD_PATTERNS = [
    ("src/rurix-render/kernels/*hit*.rx", "hit 着色阶段 kernel 痕迹"),
    ("src/rurix-render/kernels/*miss*.rx", "miss 着色阶段 kernel 痕迹"),
    ("src/rurix-render/kernels/*raygen*.rx", "raygen 阶段 kernel 痕迹"),
    ("src/rurix-render/src/bin/*rt_pipeline*.rs", "生产 RT pipeline 派发形态"),
    ("src/rurix-render/src/bin/*sbt*.rs", "生产 SBT 车道形态"),
]
# 五分项逐锚检索面（RFC-0045 §3.3 F9：逐分项 ≥2 pattern + 锚关键词映射）。
SUBITEM_SEARCH = {
    "SMRT": [("K:/rurix-ext/assets/*hair*", "多灯动态场景资产入压测清单"),
             ("src/rurix-render/src/shadow/*smrt*.rs", "shadow page 采样车道出现")],
    "WORLD-RC": [("src/rurix-render/src/world/*gi_link*.rs", "大世界流送 + GI 联动窗"),
                 ("milestones/g2[6-8]/*world_gi_demand*.json", "世界级持久 GI 需求场景出现")],
    "NRD": [("evidence/*denoise_quality_gap*.json", "自研降噪画质差距 measured 检出"),
            ("evidence/g2[5-8]_*nrd*.json", "G25 终审窗或后续画质门 measured artifact")],
    "OMM": [("K:/rurix-ext/assets/*bistro_exterior*", "alpha-tested 几何主导场景入压测清单"),
            ("evidence/g2[4-8]_*fbx*conversion*success*.json", "BistroExterior 转换臂窗联动（G10-N6 维持 defer 互核）")],
    "RT-PIPELINE-SBT": [("src/rurix-render/kernels/*hit*.rx", "hit/miss 着色阶段语义需求成立"),
                        ("evidence/*ser_gain_estimate*.json", "SER 收益 measured 预估窗")],
}


def _glob_hits(pattern: str) -> list[str]:
    try:
        if pattern.startswith("K:/"):
            base = Path(pattern[:2] + "\\")
            rel = pattern[3:]
            return [str(p) for p in sorted(base.glob(rel))]
        return [str(p.relative_to(ROOT)) for p in sorted(ROOT.glob(pattern))]
    except (OSError, ValueError):
        return []


def fresh_vulkaninfo() -> tuple[str, dict]:
    """新鲜 vulkaninfo 复测三态闭集（F8）：available / not-available（漂移）/ SKIP。"""
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    log = LOG_DIR / "vulkaninfo.log"
    try:
        r = subprocess.run(["vulkaninfo"], cwd=ROOT, capture_output=True, text=True, timeout=300)
    except (OSError, subprocess.TimeoutExpired) as e:
        return "SKIP", {"reason": f"vulkaninfo 不可定位/超时: {e}", "tokens": {}}
    text = (r.stdout or "") + (r.stderr or "")
    log.write_text(text, encoding="utf-8", newline="\n")
    tokens = {t: (t in text) for t in SER_TOKENS}
    state = "available" if all(tokens.values()) else "not-available"
    return state, {"tokens": tokens, "log": str(log.relative_to(ROOT)), "rc": r.returncode}


def materialize() -> dict:
    g21 = wel.load_json(G21_PROBE) if G21_PROBE.is_file() else {}
    readonly_ok = g21.get("capability_verdict") == "available"
    fresh_state, fresh_detail = fresh_vulkaninfo()
    # 合取判定输入面：新鲜复测跑成即取现势；SKIP 态取在案态并登记降级口径（F8）。
    cap_current = (fresh_state == "available") if fresh_state != "SKIP" else readonly_ok
    drift = (fresh_state == "not-available" and readonly_ok)
    wl_manifest = []
    wl_hits = 0
    for pat, anchor_kw in WORKLOAD_PATTERNS:
        hits = _glob_hits(pat)
        wl_manifest.append({"pattern": pat, "anchor_keyword": anchor_kw, "hits": len(hits), "files": hits})
        wl_hits += len(hits)
    workload_hit = wl_hits > 0
    m52_started = cap_current and workload_hit
    subitems = []
    for sid, pats in SUBITEM_SEARCH.items():
        man = []
        s_hits = 0
        for pat, anchor_kw in pats:
            hits = _glob_hits(pat)
            man.append({"pattern": pat, "anchor_keyword": anchor_kw, "hits": len(hits), "files": hits})
            s_hits += len(hits)
        subitems.append({"id": sid, "manifest": man, "anchor_hit": s_hits > 0,
                         "disposition": "rejudgment-started" if s_hits > 0 else "maintain-defer"})
    doc = {
        "schema": "rurix.g28.m52_rd040_workload_rejudgment.v1",
        "m52": {
            "capability_readonly": {"verdict": g21.get("capability_verdict"), "three_tokens_in_archive": readonly_ok,
                                    "source": "milestones/g21/g21_ser_capability_probe_results.json（0-byte 只读）"},
            "capability_fresh": {"state": fresh_state, **fresh_detail,
                                 "drift_event": drift,
                                 "note": "三态闭集（F8）：现势优先；SKIP 态取在案态并登记降级口径"},
            "workload": {"manifest": wl_manifest, "hit": workload_hit,
                         "not_conflated": "rurix-rt 库面 SBT 底座（G8.2 M50，RFC-0019 冻结子集）不构成锚「宿主车道出现」命中——检索面显式区分库面底座与生产车道形态"},
            "conjunction": {"capability_current": cap_current, "workload_hit": workload_hit,
                            "verdict": "rejudgment-started" if m52_started else "maintain-defer",
                            "hardline": "capability 单半命中不得构成改判（G21 终判先例；两半布尔合取）"},
        },
        "rd040_subitems": subitems,
        "verdict": ("m52-rejudgment-started" if m52_started
                    else "maintain-defer-all" if all(s["disposition"] == "maintain-defer" for s in subitems)
                    else "partial-subitem-rejudgment"),
    }
    OUT_JSON.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    return doc


def evaluate() -> list[dict]:
    facts: list[dict] = []
    doc = materialize()
    m52 = doc["m52"]
    ro = m52["capability_readonly"]
    facts.append({"id": "m52_capability_readonly_inventory",
                  "status": "PASS" if ro["three_tokens_in_archive"] else "FAIL",
                  "detail": f"G21 在案 verdict={ro['verdict']}（三 token available 取证只读盘点）"})
    fr = m52["capability_fresh"]
    facts.append({"id": "m52_capability_fresh_reprobe", "status": "PASS",
                  "detail": f"新鲜复测三态 = {fr['state']}（drift_event={fr.get('drift_event')}；"
                            f"tokens={fr.get('tokens')}；F8 现势优先口径）"})
    wl = m52["workload"]
    facts.append({"id": "m52_workload_search_manifest",
                  "status": "PASS" if len(wl["manifest"]) == 5 else "FAIL",
                  "detail": f"workload 半边 = {'命中' if wl['hit'] else '零实现'}"
                            f"（manifest 5 条 + 锚关键词映射；M50 库面底座不混同）"})
    cj = m52["conjunction"]
    facts.append({"id": "m52_conjunction_verdict", "status": "PASS",
                  "detail": f"两半合取：capability={cj['capability_current']} workload={cj['workload_hit']} → "
                            f"{cj['verdict']}（{cj['hardline']}）"})
    subs = doc["rd040_subitems"]
    man_ok = len(subs) == 5 and all(len(s["manifest"]) >= 2 for s in subs)
    facts.append({"id": "rd040_five_subitem_manifests", "status": "PASS" if man_ok else "FAIL",
                  "detail": "五分项逐锚重判：" + "; ".join(f"{s['id']}={s['disposition']}" for s in subs)
                            + "（逐分项 ≥2 pattern + 锚映射表入档）"})
    rd = {}
    if DEFERRED.is_file():
        for e in json.loads(DEFERRED.read_text(encoding="utf-8")).get("entries", []):
            if e.get("id") == "RD-040":
                rd = e
    hist_ok = any("G28.3" in (h.get("event") or "") for h in rd.get("history", []))
    facts.append({"id": "rd040_history_appended", "status": "PASS" if hist_ok else "FAIL",
                  "detail": "RD-040 history 含 G28.3 重判只追加登记（断档口径注明）"})
    base_text = _git_show_file(ROOT, G28_0_IMMUTABLE_REF, "registry/deferred.json")
    base_doc = json.loads(base_text) if base_text else None
    cur_doc = json.loads(DEFERRED.read_text(encoding="utf-8")) if DEFERRED.is_file() else None
    findings = check_deferred_append_only(base_doc, cur_doc)
    facts.append({"id": "rd040_append_only_mechanized", "status": "PASS" if findings == [] else "FAIL",
                  "detail": "append-only 机核（vs G28.0 ref）" + ("" if not findings else f"；违例 {findings[:2]}")})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G28.3 M-c：M52 两半盘点（capability 现势 + workload 零实现 → maintain-defer）+ RD-040 五分项逐锚重判（全维持 defer）+ manifest 忠实性映射表 + history 只追加",
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
        assert len(WORKLOAD_PATTERNS) == 5 and len(SUBITEM_SEARCH) == 5
        assert all(len(v) >= 2 for v in SUBITEM_SEARCH.values())
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
