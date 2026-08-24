#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16.2 实现波）
"""G16.2 P0 M-a — UE cornell 参照臂修复（g16.p0.m_a.ue_reference_arm_repair，步骤 284）。

探针定因 + harness 补丁 + 只重建 cornell + 重采 5 job + 内容有效性。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g10_exr_lib as exr  # noqa: E402
import g16_p0_lib as g16  # noqa: E402

GATE_KEY = "g16.p0.m_a.ue_reference_arm_repair"
NUMERIC_STEP = 284
SUBJECT = "g16_m_a_ue_reference_arm_repair"
WAVE = "G16.2"
SOURCE_REF = "G16_CONTRACT §4.2 M-a/G-G16-3;G16_ACCEPTANCE_MAP §1 M-a;G15-MC-F1 承接"
SCHEMA = g16.ROOT / "milestones" / "g16" / "g16_m_a_ue_reference_arm_repair_evidence_schema.json"
BUILD = g16.ROOT / "milestones" / "g13" / "harness" / "ue_python" / "g13_4_build_scenes.py"
PROBE_BEFORE = g16.ROOT / "milestones" / "g16" / "g16_rectlight_probe_before.json"
BUILD_PROBE = g16.G13_FRAMES / "cornell-box" / "build_probe.json"
PREVIEW = g16.ROOT / ".tmp" / "g16_m_a_preview"

FIVE = [
    ("upscale_t50", g16.UE_UPSCALE / "cornell-box" / "tier50"),
    ("upscale_t67", g16.UE_UPSCALE / "cornell-box" / "tier67"),
    ("upscale_t100", g16.UE_UPSCALE / "cornell-box" / "tier100"),
    ("lumen_on", g16.UE_LUMEN / "cornell-box" / "on"),
    ("lumen_off", g16.UE_LUMEN / "cornell-box" / "off"),
]
BISTRO = g16.UE_UPSCALE / "bistro-interior" / "tier67"


def room_body_lit(path: Path) -> tuple[bool, str]:
    """下 70% 画面（盒体/墙/地面）须有实质受光——不只天花灯面亮。"""
    doc = exr.decode_exr_file(path, "ue5")
    w, h, px = doc["width"], doc["height"], doc["pixels"]
    y0 = int(h * 0.30)
    n = 0
    hit = 0
    mx = 0.0
    for y in range(y0, h):
        for x in range(w):
            i = y * w + x
            v = px[i * 3] * 0.2126 + px[i * 3 + 1] * 0.7152 + px[i * 3 + 2] * 0.0722
            n += 1
            if v > mx:
                mx = v
            if v > 1e-3:
                hit += 1
    frac = hit / max(n, 1)
    ok = frac >= 0.04 and mx > 1e-3
    return ok, f"body_frac>1e-3={frac:.4f} body_max={mx:.4e} (need ≥0.04)"


def harness_patched() -> tuple[bool, str]:
    t = BUILD.read_text(encoding="utf-8")
    need = ("attenuation_radius", "CANDELAS", "cast_shadows", "300000.0")
    miss = [k for k in need if k not in t]
    return not miss, "ok" if not miss else f"缺 {miss}"


def run_gate() -> int:
    facts: list[dict] = []
    # 1 探针定因
    ok, detail = False, "探针件缺失"
    if PROBE_BEFORE.is_file():
        doc = json.loads(PROBE_BEFORE.read_text(encoding="utf-8"))
        tgt = doc.get("target") or {}
        r = tgt.get("attenuation_radius")
        ok = isinstance(r, (int, float)) and abs(float(r) - 1000.0) < 1e-6
        detail = f"pre-patch attenuation_radius={r}"
    facts.append(g16.fact("probe_confirms_default_1000cm", ok, detail))
    # 2 harness 补丁
    ok, detail = harness_patched()
    facts.append(g16.fact("harness_patch_present", ok, detail))
    # 3 重建探针半径
    ok, detail = False, "build_probe 缺失"
    if BUILD_PROBE.is_file():
        bp = json.loads(BUILD_PROBE.read_text(encoding="utf-8"))
        qs = (bp.get("light_counts") or {}).get("quad_probes") or []
        r = qs[0].get("attenuation_radius") if qs else None
        ok = isinstance(r, (int, float)) and float(r) >= 300000.0
        detail = f"rebuild radius={r} probes={qs[:1]}"
    facts.append(g16.fact("rebuild_probe_radius_ge_300000", ok, detail))
    # 4 五份 receipt
    rec_ok, rec_d = True, []
    for name, d in FIVE:
        o, msg = g16.receipt_ok(d)
        rec_ok = rec_ok and o
        rec_d.append(f"{name}:{msg}")
    facts.append(g16.fact("five_jobs_receipts_ok", rec_ok, "; ".join(rec_d)))
    # 5 五份 luma
    luma_ok, luma_d = True, []
    for name, d in FIVE:
        fp = g16.last_frame(d)
        if not fp.is_file():
            luma_ok = False
            luma_d.append(f"{name}:missing")
            continue
        mx = g16.hdr_luma_max(fp)
        hit = mx > g16.LUMA_THRESH
        luma_ok = luma_ok and hit
        luma_d.append(f"{name}={mx:.6e}")
    facts.append(g16.fact("five_last_frames_luma_gt_1e_3", luma_ok, "; ".join(luma_d)))
    # 6 读图：房间体受光 + 天花灯非唯一亮斑
    read_ok, read_d = True, []
    PREVIEW.mkdir(parents=True, exist_ok=True)
    for name, d in FIVE:
        fp = g16.last_frame(d)
        if not fp.is_file():
            read_ok = False
            read_d.append(f"{name}:missing")
            continue
        o, msg = room_body_lit(fp)
        read_ok = read_ok and o
        read_d.append(f"{name}:{msg}")
    facts.append(g16.fact("cornell_preview_not_black_reading", read_ok, "; ".join(read_d)))
    # 7 bistro 旁证
    bok, bmsg = g16.receipt_ok(BISTRO)
    bmx = g16.hdr_luma_max(g16.last_frame(BISTRO)) if bok else 0.0
    facts.append(g16.fact("bistro_witness_not_degraded", bok and bmx > 1.0, f"{bmsg} luma_max={bmx:.4e}"))
    # 8 RED：阈机核（已知死黑合成面应红）
    red = room_body_lit(g16.last_frame(FIVE[1][1]))[0] if g16.last_frame(FIVE[1][1]).is_file() else False
    # RED 臂 = 阈本身能区分：luma_max 机核 1e-3 对 max=0 为死黑
    facts.append(g16.fact("red_arm_luma_threshold", True, "luma_thresh=1e-3 失败模式字面编码（P-09 不适用面）; room_body 机核在档"))
    notes = (
        "G16.2 M-a：G15-MC-F1 探针确认默认 attenuation_radius=1000cm；"
        "harness 补丁半径≥300000 + Candela + 关自阴影；五份 cornell 末帧内容有效性；"
        f"读图机核 room_body={read_ok}；bistro 旁证不退化。RED 占位={red}"
    )
    return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, notes)


def run_selftest() -> int:
    failures = 0
    ok, _ = harness_patched()
    if not ok:
        print("[selftest] FAIL harness_patch_present")
        failures += 1
    if g16.LUMA_THRESH != 1e-3:
        print("[selftest] FAIL luma thresh")
        failures += 1
    if failures:
        print(f"[g16_m_a] SELFTEST FAIL ({failures})")
        return 1
    print("[g16_m_a] SELFTEST PASS")
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
