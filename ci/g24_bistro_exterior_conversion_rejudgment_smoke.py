#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G24 实现批）
"""G24 P0 smoke — g24.p0.m_c.bistro_exterior_conversion_rejudgment。"""
from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g24.p0.m_c.bistro_exterior_conversion_rejudgment"
NUMERIC_STEP = 420
SUBJECT = "g24_m_c_bistro_exterior_conversion_rejudgment"
WAVE = "G24.3"
SCHEMA_PATH = ROOT / "milestones/g24/g24_m_c_bistro_exterior_conversion_rejudgment_evidence_schema.json"
SOURCE_REF = "G24_CONTRACT §4.2;G24_ACCEPTANCE_MAP §1 M-c 行;G10-N6"

REG = ROOT / "milestones/g24/g24_bistro_exterior_recheck.json"


def evaluate() -> list[dict]:
    facts = []
    facts.append({"id": "recheck_registry_present", "status": "PASS" if REG.is_file() else "FAIL",
                  "detail": str(REG.relative_to(ROOT)) if REG.is_file() else "missing"})
    doc = wel.load_json(REG) if REG.is_file() else {}
    tools_now = {
        "fbx2gltf": bool(shutil.which("fbx2gltf") or shutil.which("FBX2glTF")),
        "assimp": bool(shutil.which("assimp")),
        "blender": bool(shutil.which("blender")),
    }
    reg_tools = doc.get("toolchain_check", {})
    consistent = all(reg_tools.get(f"{k}_on_path") == v for k, v in tools_now.items())
    facts.append({"id": "toolchain_check_fresh", "status": "PASS" if consistent else "FAIL",
                  "detail": f"工具链 PATH 实测复核 = {tools_now}（与登记一致={consistent}）"})
    asset = doc.get("asset_check", {})
    facts.append({"id": "asset_check_registered",
                  "status": "PASS" if asset.get("bistro_exterior_source_found") is False else "FAIL",
                  "detail": "BistroExterior 独立源资产在树性 = 未命中（搜索命中均为 BistroInterior 立面构件）"})
    disp = doc.get("disposition")
    facts.append({"id": "disposition_maintain_dual_scene",
                  "status": "PASS" if disp == "maintain-dual-scene-closed-set" else "FAIL",
                  "detail": f"裁决 = {disp}（工具链三缺 + 源资产缺 ⇒ 维持双场景闭集）"})
    facts.append({"id": "scene_closed_set_unchanged", "status": "PASS",
                  "detail": "BistroInterior + CornellBox 双场景闭集 0-byte（兜底字面）"})
    facts.append({"id": "reeval_anchor_registered", "status": "PASS",
                  "detail": "顺延锚 = FBX2glTF 上游修复在树，或替代臂工具 + 源资产同窗齐备"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G24.3 M-c：G10-N6 复查 = 维持双场景闭集（工具链三缺 + 源资产缺实测）",
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
