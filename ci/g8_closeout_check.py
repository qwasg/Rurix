#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.8b close-out 终审 g8.wave.8b.closeout(CI_GATES §5;design §7.3)。

只读汇总:21 key PASS + wave2~8a 聚合绿 + MAP 三向 + P2 表 + budget --strict
+ 不同日规则(最后新绿硬门 UTC 日 ≠ 本跑 UTC 日) + 8a 先行。

输出 VERDICT = READY|BLOCKED。status flip 由次日独立 PR 执行。

用法:
  py -3 ci/g8_closeout_check.py --gate g8.wave.8b.closeout
  py -3 ci/g8_closeout_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.8b.closeout"
NUMERIC_STEP = 130
SUBJECT = "g8_wave8b_closeout"
WAVE = "G8.8b"
SOURCE_REF = "CI_GATES §5;G8_CONTRACT close-out;design G8.6…§7.3"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave8b_closeout_evidence_schema.json"

P0_P1_KEYS = [
    ("g8.p0.m50.rt_pipeline_incremental", "g8_m50_rt_pipeline_incremental"),
    ("g8.p0.m89.single_source_gfx_submit", "g8_m89_single_source_gfx_submit"),
    ("g8.p0.m29.shader_permutation", "g8_m29_shader_permutation"),
    ("g8.p0.m30.pso_cache", "g8_m30_pso_cache"),
    ("g8.p0.m31.reflection_hash", "g8_m31_reflection_hash"),
    ("g8.p0.m32.capability_profile", "g8_m32_capability_profile"),
    ("g8.p0.m85.shader_manifest_ddc", "g8_m85_shader_manifest_ddc"),
    ("g8.p0.m79.asset_determinism", "g8_m79_asset_determinism"),
    ("g8.p0.m80.ddc_content_address", "g8_m80_ddc_content_address"),
    ("g8.p0.m81.gltf_import", "g8_m81_gltf_import"),
    ("g8.p0.m01.meshlet_page_builder", "g8_m01_meshlet_page_builder"),
    ("g8.p0.m04.page_format_abi", "g8_m04_page_format_abi"),
    ("g8.p0.m37.streaming_io", "g8_m37_streaming_io"),
    ("g8.p0.m19.vsm_page_cache", "g8_m19_vsm_page_cache"),
    ("g8.p0.m24.tsr_contract", "g8_m24_tsr_contract"),
    ("g8.p0.m66.physics_replay", "g8_m66_physics_replay"),
    ("g8.p0.m67.network_physics", "g8_m67_network_physics"),
    ("g8.p0.m68.fracture_pipeline", "g8_m68_fracture_pipeline"),
    ("g8.p1.m25.upscaler_input_abi", "g8_m25_upscaler_input_abi"),
    ("g8.p1.m72.cloth_product_chain", "g8_m72_cloth_product_chain"),
    ("g8.p1.m83.texture_transcode", "g8_m83_texture_transcode"),
]

WAVE_EXITS = [
    ("g8.wave.2.exit", "g8_wave2_exit"),
    ("g8.wave.3.exit", "g8_wave3_exit"),
    ("g8.wave.4.exit", "g8_wave4_exit"),
    ("g8.wave.5a.exit", "g8_wave5a_exit"),
    ("g8.wave.5b.exit", "g8_wave5b_exit"),
    ("g8.wave.6a.exit", "g8_wave6a_exit"),
    ("g8.wave.6b.exit", "g8_wave6b_exit"),
    ("g8.wave.6c.exit", "g8_wave6c_exit"),
    ("g8.wave.6d.exit", "g8_wave6d_exit"),
    ("g8.wave.7.decisions", "g8_wave7_decisions"),
    ("g8.wave.8a.soak", "g8_wave8a_soak"),
]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def evidence_utc_date(path: Path | None) -> str | None:
    if path is None:
        return None
    m = wel._UTC_STAMP_RE.search(path.name)
    if m:
        return m.group(1)[:8]
    doc = wel.load_json(path)
    ts = doc.get("timestamp") or doc.get("utc_date") or ""
    return str(ts)[:8] if ts else None


def max_first_pass_date() -> tuple[str | None, list[str]]:
    """对 21 key 取最新 PASS evidence 的 UTC 日期的 max(近似『最后新绿』)。"""
    dates: list[str] = []
    missing: list[str] = []
    for key, prefix in P0_P1_KEYS:
        p = wel.load_latest_evidence(prefix)
        if p is None:
            missing.append(key)
            continue
        doc = wel.load_json(p)
        ok, _ = wel.gate_pass_reason(doc, key)
        if not ok:
            missing.append(key)
            continue
        d = evidence_utc_date(p)
        if d:
            dates.append(d)
    if not dates:
        return None, missing
    return max(dates), missing


def run_closeout() -> int:
    if NUMERIC_STEP <= 0:
        print("[8b] NUMERIC_STEP unset → BLOCKED", file=sys.stderr)
        return 1
    today = wel.utc_stamp()[:8]
    facts: list[dict] = []
    gate_rows = [wel.require_gate_pass(k, p) for k, p in P0_P1_KEYS]
    gates_ok = all(r["status"] == "PASS" for r in gate_rows)
    facts.append(_fact("twenty_one_keys_pass", gates_ok, f"pass={sum(1 for r in gate_rows if r['status']=='PASS')}/21"))

    wave_rows = [wel.require_gate_pass(k, p) for k, p in WAVE_EXITS]
    waves_ok = all(r["status"] == "PASS" for r in wave_rows)
    facts.append(_fact("wave_exits_2_to_8a", waves_ok, f"pass={sum(1 for r in wave_rows if r['status']=='PASS')}/{len(WAVE_EXITS)}"))

    # MAP 三向
    map_r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "check_g8_acceptance_map.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    map_ok = map_r.returncode == 0
    facts.append(_fact("acceptance_map_triple", map_ok, f"exit={map_r.returncode}"))

    # P2
    p2 = wel.load_latest_evidence("g8_wave7_decisions")
    p2_ok = False
    if p2:
        d = wel.load_json(p2)
        p2_ok = d.get("host_section_pass") is True
    facts.append(_fact("p2_decisions_pass", p2_ok, str(p2.relative_to(ROOT)) if p2 else "missing"))

    # budget strict
    bud = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "budget_eval.py"), "--strict"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    bud_ok = bud.returncode == 0 and "[budget_eval] PASS" in ((bud.stdout or "") + (bud.stderr or ""))
    facts.append(_fact("budget_strict", bud_ok, f"exit={bud.returncode}"))

    # 8a 先行
    e8a = wel.load_latest_evidence("g8_wave8a_soak")
    e8a_ok = False
    e8a_commit = None
    if e8a:
        d8 = wel.load_json(e8a)
        e8a_ok = d8.get("host_section_pass") is True
        e8a_commit = d8.get("base_commit")
    facts.append(_fact("soak_8a_precedes", e8a_ok, str(e8a.relative_to(ROOT)) if e8a else "missing"))

    # 不同日
    last_green, missing = max_first_pass_date()
    different_day = bool(last_green) and last_green != today and not missing
    facts.append(
        _fact(
            "new_green_different_day",
            different_day,
            f"last_green_utc={last_green} today={today} missing={missing[:3]}",
        )
    )

    overall = all(f["status"] == "PASS" for f in facts)
    verdict = "READY" if overall else "BLOCKED"
    stamp = wel.utc_stamp()
    payload = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": WAVE,
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": overall,
        "device_section_state": "not_applicable",
        "verdict": verdict,
        "utc_date": today,
        "last_new_green_utc_date": last_green,
        "base_commit_8a": e8a_commit,
        "required_gates": gate_rows + wave_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "twenty_one_keys_pass": gates_ok,
            "wave_exits_pass": waves_ok,
            "acceptance_map_ok": map_ok,
            "p2_ok": p2_ok,
            "budget_strict_ok": bud_ok,
            "soak_8a_ok": e8a_ok,
            "new_green_different_day": different_day,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": "status flip is a separate next-day PR after READY",
    }
    if SCHEMA_PATH.is_file():
        errs = wel.validate_schema(payload, SCHEMA_PATH)
        if errs:
            print(f"[8b] schema: {errs}", file=sys.stderr)
            overall = False
            payload["host_section_pass"] = False
            payload["verdict"] = "BLOCKED"
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    for f in facts:
        print(f"  FACT  {f['status']:4}  {f['id']}  ({f['detail']})")
    print(f"  → evidence {out.relative_to(ROOT)}")
    print(f"  VERDICT = {payload['verdict']}")
    return 0 if overall else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        if NUMERIC_STEP <= 0:
            code = run_closeout()
            if code == 0:
                print("[selftest] FAIL: draft green", file=sys.stderr)
                return 1
            print("[selftest] PASS: draft → BLOCKED")
            return 0
        print("[selftest] OK materialized step", NUMERIC_STEP)
        return 0
    return run_closeout()


if __name__ == "__main__":
    sys.exit(main())
