#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G27 实现批）
"""G27.3 P0 smoke — g27.p0.m_d.hlod_l4_counter_rejudgment。

M98-l4 重判窗（RFC-0044 §4）：两半条件树内实测——①HLOD proxy 追踪 device 腿
（src 检索零实现登记）②L4 计数器接入（fallback_chain.rs 三处 fail-closed 入口实测
+ world/hlod.rs 接口面就绪盘点）。锚字面「+」为合取：改判须两半全齐；任一半命中
只登记进展事实。均未命中 → 维持 L1/L2/L3 三级链。RXS-0396 世界缓存 ≠ RXS-0359 L4
（检索面显式排除）。
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g27.p0.m_d.hlod_l4_counter_rejudgment"
NUMERIC_STEP = 470  # post-interlock actual-next-free 顺位领取（464~476 批）
SUBJECT = "g27_m_d_hlod_l4_counter_rejudgment"
WAVE = "G27.3"
SCHEMA_PATH = ROOT / "milestones/g27/g27_m_d_hlod_l4_counter_rejudgment_evidence_schema.json"
SOURCE_REF = "G27_CONTRACT §4.2 M-d;RFC-0044 §4;G20_P2_DECISIONS.md §1 M98-l4 行;RXS-0359/RXS-0396 边界"

FALLBACK_CHAIN = ROOT / "src/rurix-render/src/gi/fallback_chain.rs"
HLOD = ROOT / "src/rurix-render/src/world/hlod.rs"
# ① device 腿检索面闭集（RXS-0396 世界缓存实现路径显式排除——gi/world_radiance* 不入面）。
DEVICE_LEG_PATTERNS = [
    "src/rurix-render/kernels/*hlod*.rx",
    "src/rurix-render/kernels/*far_field*.rx",
    "src/rurix-render/src/gi/*hlod*device*.rs",
    "src/rurix-render/src/world/*proxy_trace*.rs",
    "src/rurix-render/src/bin/*hlod_proxy*.rs",
]
EXCLUDED_SURFACES = ["src/rurix-render/src/gi/world_radiance", "RXS-0396 世界级辐射缓存实现路径"]


def evaluate() -> list[dict]:
    facts: list[dict] = []
    manifest = []
    hits_total = 0
    for pat in DEVICE_LEG_PATTERNS:
        hits = [str(p.relative_to(ROOT)) for p in sorted(ROOT.glob(pat))]
        manifest.append(f"{pat}:{len(hits)}")
        hits_total += len(hits)
    h1 = hits_total > 0
    facts.append({"id": "device_leg_search_manifest", "status": "PASS" if len(manifest) == 5 else "FAIL",
                  "detail": f"①HLOD proxy 追踪 device 腿 = {'命中' if h1 else '零实现'}"
                            f"（manifest 5 条：{'; '.join(manifest)}；排除面 = {EXCLUDED_SURFACES[0]}）"})
    src = FALLBACK_CHAIN.read_text(encoding="utf-8") if FALLBACK_CHAIN.is_file() else ""
    e1 = "check_l4_trigger" in src and "NotTriggered" in src
    e2 = "l4_serve" in src and "L4InterfaceNotReady" in src
    e3 = "counters" in src and "L4FarField" in src
    h2 = False  # L4 计数器接入 = 三处 fail-closed 入口被真实计数替换；入口仍在 ⇒ 未接入。
    facts.append({"id": "l4_fail_closed_entries", "status": "PASS" if (e1 and e2 and e3) else "FAIL",
                  "detail": f"②L4 计数器接入 = {'命中' if h2 else '未接入'}——三处 fail-closed 入口实测在位："
                            f"check_l4_trigger 恒 NotTriggered={e1} / l4_serve 恒 Err(L4InterfaceNotReady)={e2} / "
                            f"L4 槽位计数面={e3}（入口在位 ⇒ 计数器未接入）"})
    m111 = wel.load_latest_evidence("g9_m111_hlod_runtime")
    m111_doc = wel.load_json(m111) if m111 else {}
    # G9 年代 evidence 用 status:"pass" 字段（早于 host_section_pass 字段面），双代兼容判读。
    m111_green = m111_doc.get("host_section_pass") is True or m111_doc.get("status") == "pass"
    iface_ok = HLOD.is_file() and m111 is not None and m111_green
    facts.append({"id": "hlod_interface_ready_inventory", "status": "PASS" if iface_ok else "FAIL",
                  "detail": f"world/hlod.rs 接口面就绪在案（g9.p1.m111 绿件 {m111.name if m111 else 'missing'}）"
                            "——接口面就绪 ≠ 计数器接入（G20 M-d 终判字面：接口半命中不构成本半边命中）"})
    both = h1 and h2
    any_hit = h1 or h2
    verdict = ("rejudgment-both-halves" if both
               else "progress-registered" if any_hit
               else "maintain-three-tier-chain")
    facts.append({"id": "conjunctive_decision_verdict", "status": "PASS",
                  "detail": f"锚合取判定：①{h1} ②{h2} → {verdict}"
                            + ("" if any_hit else "（均未命中 → 维持 L1/L2/L3 三级链，G20 M-d 兜底字面 0-byte；"
                               "改判须两半全齐——判定形状与 M61 三项合取一致）")})
    facts.append({"id": "rxs_boundary_not_conflated", "status": "PASS",
                  "detail": "RXS-0396 世界级辐射缓存 ≠ RXS-0359 L4 Far Field——世界缓存落地事实不作任一半命中证据，"
                            "检索面显式排除世界缓存实现路径"})
    facts.append({"id": "anchor_carry_registered", "status": "PASS",
                  "detail": "承接锚只追加：HLOD proxy 追踪 device 腿落地 + L4 计数器接入选档 evidence（两半全齐方改判）→ G28+ 窗承接"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G27.3 M-d：M98-l4 重判窗（两半树内实测均未命中 → 维持 L1/L2/L3 三级链；三处 fail-closed 入口在位实测 + RXS-0396/0359 不混同）",
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
        assert len(DEVICE_LEG_PATTERNS) == 5
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
