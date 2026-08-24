#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16.3 实现波）
"""G16.3 P0 M-b — 双端重收割（g16.p0.m_b.dual_end_reharvest，步骤 285）。

import G13 度量函数，不写 G13 两张登记表。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g10_exr_lib as exr  # noqa: E402
import g13_ue_lumen_gi_parity_smoke as g13md  # noqa: E402
import g13_ue_upscale_parity_smoke as g13mc  # noqa: E402
import g16_p0_lib as g16  # noqa: E402

GATE_KEY = "g16.p0.m_b.dual_end_reharvest"
NUMERIC_STEP = 285
SUBJECT = "g16_m_b_dual_end_reharvest"
WAVE = "G16.3"
SOURCE_REF = "G16_CONTRACT §4.2 M-b/G-G16-4;G16_ACCEPTANCE_MAP §1 M-b"
SCHEMA = g16.ROOT / "milestones" / "g16" / "g16_m_b_dual_end_reharvest_evidence_schema.json"
DISP = g16.ROOT / "milestones" / "g16" / "g16_quality_gap_disposition.json"
WORK = g16.G13_FRAMES / "report" / "g16_m_b"
CONTRACT = g16.ROOT / "milestones" / "g13" / "g13_ue_upscale_parity_contract.json"


def _direction(old: float, new: float, kind: str) -> str:
    if abs(new - old) <= 1e-12:
        return "maintained"
    better = (new > old) if "ssim" in kind else (new < old)
    return "converged" if better else "degraded"


def recompute_upscale(ev100: dict) -> tuple[list[dict], list[str]]:
    problems: list[str] = []
    items: list[dict] = []
    WORK.mkdir(parents=True, exist_ok=True)
    reg = json.loads(g16.G13_REG_UPSCALE.read_text(encoding="utf-8"))
    for it in reg.get("items") or []:
        fresh = []
        for d in it.get("measured_delta") or []:
            metric = str(d.get("metric"))
            parts = metric.split("@", 1)
            if len(parts) != 2:
                problems.append(f"metric 不可解析 {metric}")
                continue
            kind, loc = parts
            scene, tpart, backend = loc.split("/")
            tier = int(tpart[1:])
            ue_dir = g16.UE_UPSCALE / scene / f"tier{tier}"
            ue_ref = g16.UE_UPSCALE / scene / "tier100" / ".0031.exr"
            ru_dir = g16.G13_FRAMES / "rurix_upscale" / scene / f"tier{tier}" / backend
            ru_ref = g16.G13_FRAMES / "rurix_upscale" / scene / "tier100" / backend / "converged.exr"
            ue_frames = sorted(str(p) for p in ue_dir.glob("*.exr") if p.name.startswith("."))
            if len(ue_frames) < 32 or not ru_dir.joinpath("converged.exr").is_file():
                problems.append(f"帧缺 {metric}")
                continue
            if kind in ("ssim_deficit_delta", "flip_deficit_delta"):
                m = g13mc.cell_metrics(
                    scene, tier, backend, ue_frames, ru_dir / "converged.exr",
                    ue_ref, ru_ref, ev100[scene], WORK,
                )
                a = m["ssim_ue"] if kind.startswith("ssim") else m["flip_ue"]
                b = m["ssim_rurix"] if kind.startswith("ssim") else m["flip_rurix"]
            else:
                ue_paths = [Path(p) for p in ue_frames]
                ru_paths = sorted((ru_dir / "frames").glob("*.exr"))
                if len(ru_paths) != 32:
                    ru_paths = sorted(ru_dir.glob(".*.exr"))
                a = g13mc.noise_hf(ue_paths, "ue5")
                b = g13mc.noise_hf(ru_paths, "rurix") if len(ru_paths) == 32 else float("nan")
            fresh.append({
                "metric": metric, "a_value": a, "b_value": b,
                "delta": (b - a) if a == a and b == b else None,
                "tolerance": d.get("tolerance"),
            })
        items.append({
            "gap_id": it.get("gap_id"),
            "source_registry": "g13_ue_upscale_gap_registry",
            "scene_id": it.get("scene_id"),
            "kind": it.get("kind"),
            "title": it.get("title"),
            "registered_delta": it.get("measured_delta"),
            "fresh_measured_delta": fresh,
            "direction": _direction(
                float((it.get("measured_delta") or [{}])[0].get("a_value") or 0),
                float((fresh or [{}])[0].get("a_value") or 0),
                str((fresh or [{}])[0].get("metric") or ""),
            ),
        })
    return items, problems


def recompute_lumen() -> tuple[list[dict], list[str]]:
    problems: list[str] = []
    items: list[dict] = []
    WORK.mkdir(parents=True, exist_ok=True)
    reg = json.loads(g16.G13_REG_LUMEN.read_text(encoding="utf-8"))
    for it in reg.get("items") or []:
        scene = it.get("scene_id")
        ue_on = g16.UE_LUMEN / scene / "on"
        ue_off = g16.UE_LUMEN / scene / "off"
        ru_on = g16.G13_FRAMES / "rurix_gi" / scene / "on" / f"{scene}.exr"
        ru_off = g16.G13_FRAMES / "rurix_gi" / scene / "off" / f"{scene}.exr"
        ue_on_f = g16.last_frame(ue_on)
        ue_off_f = g16.last_frame(ue_off)
        if not (ue_on_f.is_file() and ue_off_f.is_file() and ru_on.is_file() and ru_off.is_file()):
            problems.append(f"lumen 帧缺 {scene}")
            continue
        e_ue = g13md.frame_mean_luma(exr.decode_exr_file(ue_on_f, "ue5"))
        e_ru = g13md.frame_mean_luma(exr.decode_exr_file(ru_on, "rurix")) * g13md.EXPOSURE_SCALE[scene]
        ind_ue = g13md.indirect_ldr(scene, "ue5", ue_on_f, ue_off_f, 1.0, WORK)
        ind_ru = g13md.indirect_ldr(scene, "rurix", ru_on, ru_off, g13md.EXPOSURE_SCALE[scene], WORK)
        import g10_flip_lib as flip
        import g10_ssim_psnr_lib as ssim_psnr
        ssim_x = ssim_psnr.ssim_wang2004(ind_ue["arr"], ind_ru["arr"])
        flip_x = flip.flip_ldr(ind_ue["arr"], ind_ru["arr"], flip.default_ppd())[1]
        fresh_map = {
            f"gi_energy_rel@{scene}": (e_ue, e_ru),
            f"indirect_ssim@{scene}": (1.0, ssim_x),
            f"indirect_flip@{scene}": (0.0, flip_x),
        }
        fresh = []
        for d in it.get("measured_delta") or []:
            metric = str(d.get("metric"))
            a, b = fresh_map.get(metric, (None, None))
            if a is None:
                problems.append(f"lumen metric 未覆盖 {metric}")
                continue
            fresh.append({"metric": metric, "a_value": a, "b_value": b, "delta": b - a, "tolerance": d.get("tolerance")})
        items.append({
            "gap_id": it.get("gap_id"),
            "source_registry": "g13_ue_lumen_gap_registry",
            "scene_id": scene,
            "kind": it.get("kind"),
            "title": it.get("title"),
            "registered_delta": it.get("measured_delta"),
            "fresh_measured_delta": fresh,
            "direction": "measured",
            "lumen_gi_honest_note": (
                f"cornell energy_ue={e_ue:.6e} indirect_ssim={ssim_x:.6f} "
                f"（直接光复绿 ≠ GI 差分复绿；如实登记不宣称 GI 达标）"
                if scene == "cornell-box" else f"bistro energy_ue={e_ue:.6e}"
            ),
        })
    return items, problems


def run_gate() -> int:
    facts: list[dict] = []
    u_ok, u_d = g16.git_clean("milestones/g13/g13_ue_upscale_gap_registry.json")
    l_ok, l_d = g16.git_clean("milestones/g13/g13_ue_lumen_gap_registry.json")
    facts.append(g16.fact("g13_registries_0byte", u_ok and l_ok, f"upscale={u_d!r} lumen={l_d!r}"))
    ev100 = {s["scene_id"]: s["exposure"]["ev100"] for s in json.loads(CONTRACT.read_text(encoding="utf-8"))["scenes"]}
    up_items, up_prob = recompute_upscale(ev100)
    facts.append(g16.fact("upscale_metrics_recomputed", not up_prob and len(up_items) >= 1, f"n={len(up_items)} {up_prob[:3]}"))
    lu_items, lu_prob = recompute_lumen()
    facts.append(g16.fact("lumen_metrics_recomputed", not lu_prob and len(lu_items) >= 1, f"n={len(lu_items)} {lu_prob[:3]}"))
    all_items = up_items + lu_items
    cornell_a = []
    for it in up_items:
        if it.get("scene_id") != "cornell-box":
            continue
        for d in it.get("fresh_measured_delta") or []:
            if "ssim" in d["metric"] or "flip" in d["metric"]:
                cornell_a.append((d["metric"], d["a_value"]))
    not_black = any(
        ("ssim" in m and abs(float(a) - 1.0) > 1e-6) or ("flip" in m and abs(float(a) - 0.0) > 1e-6)
        for m, a in cornell_a if a == a
    )
    facts.append(g16.fact("cornell_a_value_not_black_perfect", not_black, f"cornell a_values={cornell_a[:6]}"))
    gi_note = "; ".join(it.get("lumen_gi_honest_note", "") for it in lu_items)
    facts.append(g16.fact("lumen_gi_delta_honest", bool(lu_items), gi_note or "无 Lumen 行"))
    doc = {
        "schema_version": 1,
        "registry": "g16_quality_gap_disposition",
        "generated_by": "ci/g16_dual_end_reharvest_smoke.py --gate g16.p0.m_b.dual_end_reharvest",
        "wave": WAVE,
        "scene_set": ["cornell-box", "bistro-interior"],
        "items": all_items,
        "note": "不写 G13 两张冻结登记表；G12 PT 不在本波。",
    }
    DISP.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    u2, _ = g16.git_clean("milestones/g13/g13_ue_upscale_gap_registry.json")
    l2, _ = g16.git_clean("milestones/g13/g13_ue_lumen_gap_registry.json")
    facts.append(g16.fact("disposition_written", DISP.is_file() and u2 and l2, f"items={len(all_items)} path={DISP}"))
    notes = "G16.3 M-b：G13 M-c/M-d 同口径重算入 G16 处置表；G13 登记表 git 0-byte。"
    return g16.emit(WAVE, SUBJECT, GATE_KEY, NUMERIC_STEP, SOURCE_REF, SCHEMA, facts, notes)


def run_selftest() -> int:
    if not g16.G13_REG_UPSCALE.is_file() or not g16.G13_REG_LUMEN.is_file():
        print("[selftest] FAIL 缺 G13 登记表")
        return 1
    print("[g16_m_b] SELFTEST PASS")
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
        return g16.verify_latest_wave(SUBJECT, 6)
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
