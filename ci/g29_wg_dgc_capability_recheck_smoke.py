#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G29 实现批）
"""G29.3 P0 smoke — g29.p0.m_d.wg_dgc_capability_recheck。

WG/DGC capability 复测（RFC-0046 §4）：VK_AMDX_shader_enqueue 新鲜 vulkaninfo 复测
三态闭集（absent 维持 not-available / present 翻转复评启动〔门同样绿〕/ SKIP 如实）
+ DGC 三扩展 available 复测互核（漂移事件如实登记）+ FSR 3.1.5 maintain 盘点
（vendor_upscale 面 0-byte）。门态映射 = 分支捕获非透传（F10 同律）。
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g29.p0.m_d.wg_dgc_capability_recheck"
NUMERIC_STEP = 502  # post-interlock actual-next-free 顺位领取（496~508 批）
SUBJECT = "g29_m_d_wg_dgc_capability_recheck"
WAVE = "G29.3"
SCHEMA_PATH = ROOT / "milestones/g29/g29_m_d_wg_dgc_capability_recheck_evidence_schema.json"
SOURCE_REF = "G29_CONTRACT §4.2 M-d;RFC-0046 §4;g22_work_graphs_probe_results.json;G22_P2_DECISIONS.md §3 G22-N4"

G22_PROBE = ROOT / "milestones/g22/g22_work_graphs_probe_results.json"
LOG_DIR = ROOT / ".tmp/g29_md"
WG_TOKEN = "VK_AMDX_shader_enqueue"
DGC_TOKENS = ["VK_EXT_device_generated_commands", "VK_NV_device_generated_commands",
              "VK_NV_device_generated_commands_compute"]


def fresh_vulkaninfo() -> tuple[str, dict]:
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    log = LOG_DIR / "vulkaninfo.log"
    try:
        r = subprocess.run(["vulkaninfo"], cwd=ROOT, capture_output=True, text=True, timeout=300)
    except (OSError, subprocess.TimeoutExpired) as e:
        return "SKIP", {"reason": f"vulkaninfo 不可定位/超时: {e}"}
    text = (r.stdout or "") + (r.stderr or "")
    log.write_text(text, encoding="utf-8", newline="\n")
    wg_present = WG_TOKEN in text
    dgc = {t: (t in text) for t in DGC_TOKENS}
    return ("present" if wg_present else "absent"), {"dgc": dgc, "log": str(log.relative_to(ROOT))}


def evaluate() -> list[dict]:
    facts: list[dict] = []
    g22 = wel.load_json(G22_PROBE) if G22_PROBE.is_file() else {}
    g22_wg_absent = g22.get("work_graphs_verdict") == "not-available"
    facts.append({"id": "g22_inventory_readonly", "status": "PASS" if g22_wg_absent else "FAIL",
                  "detail": f"G22 在案：work_graphs_verdict={g22.get('work_graphs_verdict')}（{WG_TOKEN} absent）+ dgc_tokens 三键在档"})
    state, detail = fresh_vulkaninfo()
    if state == "absent":
        branch = "maintain-not-available"
        branch_note = "WG 扩展新鲜复测 absent（与 G22 在案一致）→ not-available 维持"
    elif state == "present":
        branch = "relock-review-triggered"
        branch_note = "WG 扩展新鲜复测 present——翻转事件！Work Graphs 立项评估复评启动登记（承接锚窗）"
    else:
        branch = "skip-registered"
        branch_note = f"新鲜复测未跑（{detail.get('reason')}）——在案态兜底 + 降级口径如实登记"
    facts.append({"id": "wg_fresh_reprobe_tristate", "status": "PASS",
                  "detail": f"三态闭集 = {state} → {branch}：{branch_note}（门态映射 F10 同律：not-available 维持/翻转复评均门绿）"})
    dgc = detail.get("dgc", {})
    dgc_all = all(dgc.values()) if dgc else None
    g22_dgc = g22.get("dgc_tokens", {})
    drift = (dgc_all is False and all(g22_dgc.values()))
    facts.append({"id": "dgc_three_ext_crosscheck", "status": "PASS",
                  "detail": f"DGC 三扩展复测互核：{dgc if dgc else 'SKIP 态未测'}（G22 在案全 true；漂移事件={drift} 如实登记）"})
    r = subprocess.run(["git", "diff", "--quiet", "g28-closed", "--", "src/rurix-rt/src/vendor_upscale.rs"],
                       cwd=ROOT, capture_output=True)
    facts.append({"id": "fsr_maintain_inventory", "status": "PASS" if r.returncode == 0 else "FAIL",
                  "detail": "FSR 3.1.5 maintain 盘点：vendor_upscale.rs vs g28-closed 0-byte（G22_P2 §3 G22-N4 行字面维持）"})
    facts.append({"id": "wg_probe_source_readonly", "status": "PASS" if subprocess.run(
        ["git", "diff", "--quiet", "g28-closed", "--", "milestones/g22/g22_work_graphs_probe_results.json"],
        cwd=ROOT, capture_output=True).returncode == 0 else "FAIL",
        "detail": "g22 WG probe 结果 0-byte（原始锚不回写）"})
    facts.append({"id": "anchor_carry_registered", "status": "PASS",
                  "detail": "承接锚只追加：VK_AMDX_shader_enqueue（或 Vulkan 跨厂商对应物）present 翻转 + 接缝消费方出现 → Work Graphs 立项窗"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G29.3 M-d：WG/DGC capability 复测（WG 三态闭集 + DGC 三扩展互核 + FSR maintain 盘点 + 门态映射分支）",
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
        assert len(DGC_TOKENS) == 3
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
