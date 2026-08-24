#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16.2 实现波）
"""G16 P0 四门共享判定层（不合并断言；各门独立 extra_facts / evidence subject）。"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

import g10_exr_lib as exr
import g11_wave_exit_lib as wel

ROOT = wel.ROOT
G13_FRAMES = Path(r"K:\rurix-ext\g13-frames")
UE_UPSCALE = G13_FRAMES / "ue_upscale"
UE_LUMEN = G13_FRAMES / "ue_lumen"
LUMA_THRESH = 1e-3
G13_REG_UPSCALE = ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json"
G13_REG_LUMEN = ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json"
G15_BUDGET = ROOT / "milestones" / "g15" / "g15_budget.json"
G15_DISP = ROOT / "milestones" / "g15" / "g15_quality_gap_disposition.json"
G15_RECORDS = ROOT / "milestones" / "g15" / "g15_m_c_ai_reading_records.json"


def fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def hdr_luma_max(path: Path, end: str = "ue5") -> float:
    doc = exr.decode_exr_file(path, end)
    px = doc["pixels"]
    n = doc["width"] * doc["height"]
    mx = 0.0
    for i in range(n):
        v = px[i * 3] * 0.2126 + px[i * 3 + 1] * 0.7152 + px[i * 3 + 2] * 0.0722
        if v > mx:
            mx = v
    return mx


def git_clean(rel: str) -> tuple[bool, str]:
    r = subprocess.run(
        ["git", "diff", "--exit-code", "--", rel],
        cwd=ROOT, capture_output=True, text=True,
    )
    return r.returncode == 0, (r.stdout + r.stderr)[-200:]


def last_frame(dir_path: Path) -> Path:
    return dir_path / ".0031.exr"


def receipt_ok(dir_path: Path) -> tuple[bool, str]:
    rp = dir_path / "render_receipt.json"
    if not rp.is_file():
        return False, "receipt 缺失"
    rec = json.loads(rp.read_text(encoding="utf-8"))
    frames = rec.get("frames") or []
    if rec.get("exit_code") != 0:
        return False, f"exit_code={rec.get('exit_code')}"
    if len(frames) != 32:
        return False, f"frames={len(frames)}≠32"
    if not last_frame(dir_path).is_file():
        return False, "末帧缺失"
    return True, f"exit=0 frames=32 started={rec.get('started_epoch')}"


def verify_latest_wave(subject: str, n_facts: int) -> int:
    path = wel.load_latest_evidence(subject)
    if path is None:
        print(f"[g16] FAIL: 缺最新 evidence（{subject}_*.json）")
        return 1
    doc = wel.load_json(path)
    facts = doc.get("extra_facts") or []
    bad = [f.get("id") for f in facts if f.get("status") != "PASS"]
    if bad or not doc.get("host_section_pass") or len(facts) != n_facts:
        print(f"[g16] FAIL {path.name} bad={bad} n={len(facts)}")
        return 1
    print(f"[g16] verify-latest PASS {path.name}")
    return 0


def emit(wave: str, subject: str, key: str, step: int, source_ref: str,
         schema: Path, facts: list[dict], notes: str) -> int:
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=wave,
        subject=subject,
        symbolic_gate_key=key,
        numeric_step=step,
        source_ref=source_ref,
        required_gate_rows=[],
        extra_facts=facts,
        subjects=[],
        schema_path=schema,
        evidence_basename=subject,
        notes=notes,
        host_section_pass=ok,
    )
    return code
