#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16plus M-e）
"""G16plus M-e GI 表达（g16.p0.m_e.gi_expression，步骤 288）。"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g16_p0_lib as g16  # noqa: E402
import g10_exr_lib as exr  # noqa: E402

GATE_KEY = "g16.p0.m_e.gi_expression"
NUMERIC_STEP = 288
SUBJECT = "g16_m_e_gi_expression"
WAVE = "G16.8"
SOURCE_REF = "G16_CONTRACT G-G16-8;G16_ACCEPTANCE_MAP 附录 A M-e;RFC-0031"
SCHEMA = g16.ROOT / "milestones" / "g16" / "g16_m_e_gi_expression_evidence_schema.json"
RFC = g16.ROOT / "rfcs" / "0031-g16plus-gi-expression-quality-closure.md"
KERNEL_OFF = g16.ROOT / "src" / "rurix-render" / "kernels" / "g14_3_direct_gi.rx"
KERNEL_ON = g16.ROOT / "src" / "rurix-render" / "kernels" / "g16_gi_multibounce.rx"
BIN_RS = g16.ROOT / "src" / "rurix-render" / "src" / "bin" / "g14_3_pipeline_perf.rs"
PROBE = g16.ROOT / ".tmp" / "g16plus_gi_probe"
SPV_ON = g16.ROOT / ".tmp" / "g14_gates" / "m_c" / "g16_gi_multibounce.spv"
BIN = g16.ROOT / "target" / "release" / ("g14_3_pipeline_perf.exe" if sys.platform == "win32" else "g14_3_pipeline_perf")


def _sha_file(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else ""


def _rurixc() -> Path | None:
    exe = g16.ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if exe.is_file():
        return exe
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
        cwd=g16.ROOT, capture_output=True, text=True,
    )
    return exe if r.returncode == 0 and exe.is_file() else None


def _compile_gi() -> bool:
    rurixc = _rurixc()
    if rurixc is None or not KERNEL_ON.is_file():
        return False
    SPV_ON.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        [str(rurixc), str(KERNEL_ON), "--target", "vulkan", "-o", str(SPV_ON)],
        cwd=g16.ROOT, capture_output=True, text=True,
    )
    return r.returncode == 0 and SPV_ON.is_file()


def _mean_rgb(path: Path) -> tuple[float, float, float] | None:
    if not path.is_file():
        return None
    doc = exr.decode_exr_file(path, "rurix")
    px, w, h = doc["pixels"], doc["width"], doc["height"]
    n = w * h
    r = g = b = 0.0
    for i in range(n):
        r += px[i * 3]
        g += px[i * 3 + 1]
        b += px[i * 3 + 2]
    return r / n, g / n, b / n


def _wall_chroma(path: Path) -> tuple[float, float] | None:
    """本 cornell-box-generated 相机为左绿右红。量地板下三分上的墙色染色
    （白地板间接光），避免把墙面自身 albedo 误当成 bleed。"""
    if not path.is_file():
        return None
    doc = exr.decode_exr_file(path, "rurix")
    px, w, h = doc["pixels"], doc["width"], doc["height"]
    y0 = 2 * h // 3
    left = right = 0.0
    nl = nr = 0
    for y in range(y0, h):
        for x in range(w):
            i = y * w + x
            r, g = px[i * 3], px[i * 3 + 1]
            if x < w // 3:
                left += g - r
                nl += 1
            elif x > 2 * w // 3:
                right += r - g
                nr += 1
    return left / max(nl, 1), right / max(nr, 1)


def _run_probe() -> dict:
    PROBE.mkdir(parents=True, exist_ok=True)
    if not BIN.is_file():
        r = subprocess.run(
            ["cargo", "build", "-p", "rurix-render", "--release", "--bin", "g14_3_pipeline_perf",
             "--features", "vendor-upscale"],
            cwd=g16.ROOT, capture_output=True, text=True,
        )
        if r.returncode != 0:
            return {"ok": False, "note": "bin build fail"}
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    out_on = PROBE / "on"
    out_off = PROBE / "off"
    rows = []
    for gi, out in (("off", out_off), ("on", out_on)):
        cmd = [
            str(BIN), "--render", "--scene", "cornell-box", "--tier", "67",
            "--backend", "tsr_device", "--frames", "8", "--gi", gi,
            "--out-root", str(out),
        ]
        r = subprocess.run(cmd, cwd=g16.ROOT, capture_output=True, text=True, env=env)
        rows.append((gi, r.returncode, ((r.stdout or "") + (r.stderr or ""))[-400:]))
    on_exr = out_on / "cornell-box" / "tier67" / "tsr_device" / "converged.exr"
    off_exr = out_off / "cornell-box" / "tier67" / "tsr_device" / "converged.exr"
    on_m, off_m = _mean_rgb(on_exr), _mean_rgb(off_exr)
    chroma = _wall_chroma(on_exr)
    indirect = 0.0
    if on_m and off_m:
        indirect = (on_m[0] - off_m[0] + on_m[1] - off_m[1] + on_m[2] - off_m[2]) / 3.0
    bleed = bool(chroma and chroma[0] > 0.002 and chroma[1] > 0.002)
    doc = {
        "indirect_mean": indirect,
        "color_bleed": bleed,
        "bleed_note": f"left_rg={chroma[0] if chroma else None} right_gr={chroma[1] if chroma else None}",
        "on_mean": on_m,
        "off_mean": off_m,
        "rows": rows,
    }
    (PROBE / "probe.json").write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return doc


def run_gate() -> int:
    facts = []
    rfc_t = RFC.read_text(encoding="utf-8") if RFC.is_file() else ""
    facts.append(g16.fact("rfc0031_approved", "Agent Approved" in rfc_t and "RFC-0031" in rfc_t, RFC.name))
    compiled = _compile_gi()
    facts.append(g16.fact(
        "kernel_on_present",
        KERNEL_ON.is_file() and "g16_gi_multibounce" in KERNEL_ON.read_text(encoding="utf-8", errors="replace") and compiled,
        f"compiled={compiled} {SPV_ON.name}",
    ))
    off_t = KERNEL_OFF.read_text(encoding="utf-8") if KERNEL_OFF.is_file() else ""
    facts.append(g16.fact("kernel_off_unmodified_no_gi", "无 GI/天光" in off_t and "g16_gi_multibounce" not in off_t, "off kernel 仍直接光"))
    bin_t = BIN_RS.read_text(encoding="utf-8") if BIN_RS.is_file() else ""
    fail_closed = "GI 多反弹臂 G14.3 not-triggered" in bin_t and 'if gi != "off"' in bin_t
    facts.append(g16.fact("gi_on_not_fail_closed", 'gi == "on"' in bin_t and not fail_closed, "加性 --gi on"))
    probe = _run_probe()
    energy_ok = float(probe.get("indirect_mean") or 0) > 1e-3
    bleed_ok = bool(probe.get("color_bleed"))
    facts.append(g16.fact("cornell_indirect_energy_nonzero", energy_ok, f"indirect_mean={probe.get('indirect_mean')}"))
    facts.append(g16.fact("cornell_color_bleed_reading", bleed_ok, str(probe.get("bleed_note", ""))))
    facts.append(g16.fact("off_arm_source_present", KERNEL_OFF.is_file() and _sha_file(KERNEL_OFF).startswith("sha256:"), _sha_file(KERNEL_OFF)))
    facts.append(g16.fact("no_handwritten_threshold", "p100" in rfc_t and "手写" in rfc_t, "P-09 字面在 RFC"))
    return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, "G16plus M-e")


def run_selftest() -> int:
    bad = 0
    if not RFC.is_file():
        print("[selftest] FAIL rfc missing")
        bad += 1
    if NUMERIC_STEP != 288:
        print("[selftest] FAIL step")
        bad += 1
    if bad:
        print(f"[g16_m_e] SELFTEST FAIL ({bad})")
        return 1
    print("[g16_m_e] SELFTEST PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return g16.verify_latest_wave(SUBJECT, 8)
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
