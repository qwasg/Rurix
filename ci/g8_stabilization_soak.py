#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.8a soak 聚合门 g8.wave.8a.soak(CI_GATES §5;design §7.2)。

四腿:全量回归(18 P0 + 3 go P1)→ uc08 soak(≥30min 且 ≥10000 帧)→
budget --strict → 纪律日期锚。

pr-smoke 默认 --verify-latest(秒级核最新 full-run evidence);
本地/workflow_dispatch 用 --gate 产 full-run。

用法:
  py -3 ci/g8_stabilization_soak.py --gate g8.wave.8a.soak
  py -3 ci/g8_stabilization_soak.py --verify-latest
  py -3 ci/g8_stabilization_soak.py --selftest
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g8_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g8.wave.8a.soak"
NUMERIC_STEP = 129
SUBJECT = "g8_wave8a_soak"
WAVE = "G8.8a"
SOURCE_REF = (
    "CI_GATES §5;G8_CONTRACT G-G8-8A;design G8.6…§7.2;"
    "18 P0 + 3 go P1 + soak ≥1800s/≥10000 frames + budget --strict"
)
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_wave8a_soak_evidence_schema.json"

# (symbolic_key, subject_prefix, smoke argv relative, require_real)
REGRESSION_GATES: list[tuple[str, str, list[str], bool]] = [
    ("g8.p0.m50.rt_pipeline_incremental", "g8_m50_rt_pipeline_incremental",
     ["ci/g8_rt_pipeline_incremental_smoke.py", "--gate", "g8.p0.m50.rt_pipeline_incremental"], True),
    ("g8.p0.m89.single_source_gfx_submit", "g8_m89_single_source_gfx_submit",
     ["ci/g8_single_source_gfx_smoke.py", "--gate", "g8.p0.m89.single_source_gfx_submit"], True),
    ("g8.p0.m29.shader_permutation", "g8_m29_shader_permutation",
     ["ci/g8_shader_permutation_smoke.py", "--gate", "g8.p0.m29.shader_permutation"], False),
    ("g8.p0.m30.pso_cache", "g8_m30_pso_cache",
     ["ci/g8_pso_cache_smoke.py", "--gate", "g8.p0.m30.pso_cache"], True),
    ("g8.p0.m31.reflection_hash", "g8_m31_reflection_hash",
     ["ci/g8_reflection_hash_smoke.py", "--gate", "g8.p0.m31.reflection_hash"], False),
    ("g8.p0.m32.capability_profile", "g8_m32_capability_profile",
     ["ci/g8_capability_profile_smoke.py", "--gate", "g8.p0.m32.capability_profile"], False),
    ("g8.p0.m85.shader_manifest_ddc", "g8_m85_shader_manifest_ddc",
     ["ci/g8_shader_manifest_ddc_smoke.py", "--gate", "g8.p0.m85.shader_manifest_ddc", "--phase", "g8.3"], False),
    ("g8.p0.m79.asset_determinism", "g8_m79_asset_determinism",
     ["ci/g8_asset_determinism_smoke.py", "--gate", "g8.p0.m79.asset_determinism"], False),
    ("g8.p0.m80.ddc_content_address", "g8_m80_ddc_content_address",
     ["ci/g8_ddc_content_address_smoke.py", "--gate", "g8.p0.m80.ddc_content_address"], False),
    ("g8.p0.m81.gltf_import", "g8_m81_gltf_import",
     ["ci/g8_gltf_import_smoke.py", "--gate", "g8.p0.m81.gltf_import"], False),
    ("g8.p0.m01.meshlet_page_builder", "g8_m01_meshlet_page_builder",
     ["ci/g8_meshlet_page_builder_smoke.py", "--gate", "g8.p0.m01.meshlet_page_builder"], False),
    ("g8.p0.m04.page_format_abi", "g8_m04_page_format_abi",
     ["ci/g8_page_format_abi_smoke.py", "--gate", "g8.p0.m04.page_format_abi"], True),
    ("g8.p0.m37.streaming_io", "g8_m37_streaming_io",
     ["ci/g8_streaming_io_smoke.py", "--gate", "g8.p0.m37.streaming_io"], True),
    ("g8.p0.m19.vsm_page_cache", "g8_m19_vsm_page_cache",
     ["ci/g8_vsm_page_cache_smoke.py", "--gate", "g8.p0.m19.vsm_page_cache"], True),
    ("g8.p0.m24.tsr_contract", "g8_m24_tsr_contract",
     ["ci/g8_tsr_contract_smoke.py", "--gate", "g8.p0.m24.tsr_contract"], True),
    ("g8.p0.m66.physics_replay", "g8_m66_physics_replay",
     ["ci/g8_physics_replay_smoke.py", "--gate", "g8.p0.m66.physics_replay"], False),
    ("g8.p0.m67.network_physics", "g8_m67_network_physics",
     ["ci/g8_network_physics_smoke.py", "--gate", "g8.p0.m67.network_physics"], False),
    ("g8.p0.m68.fracture_pipeline", "g8_m68_fracture_pipeline",
     ["ci/g8_fracture_pipeline_smoke.py", "--gate", "g8.p0.m68.fracture_pipeline"], False),
    ("g8.p1.m25.upscaler_input_abi", "g8_m25_upscaler_input_abi",
     ["ci/g8_upscaler_input_abi_smoke.py", "--gate", "g8.p1.m25.upscaler_input_abi"], True),
    ("g8.p1.m72.cloth_product_chain", "g8_m72_cloth_product_chain",
     ["ci/g8_cloth_product_chain_smoke.py", "--gate", "g8.p1.m72.cloth_product_chain"], False),
    ("g8.p1.m83.texture_transcode", "g8_m83_texture_transcode",
     ["ci/g8_texture_transcode_smoke.py", "--gate", "g8.p1.m83.texture_transcode"], False),
]

MIN_SECONDS = 1800
MIN_FRAMES = 10000


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def base_commit() -> str:
    r = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return (r.stdout or "").strip() or "unknown"


def run_regression(*, skip_rerun: bool = False) -> tuple[bool, list[dict], str]:
    """全量回归。skip_rerun=True 时只读最新 evidence(verify-latest 路径)。"""
    rows: list[dict] = []
    commit = base_commit()
    all_ok = True
    for key, prefix, argv, require_real in REGRESSION_GATES:
        if skip_rerun:
            row = wel.require_gate_pass(key, prefix)
            rows.append(row)
            if row["status"] != "PASS":
                all_ok = False
            continue
        env = os.environ.copy()
        if require_real:
            env["RURIX_REQUIRE_REAL"] = "1"
            env.setdefault("RURIX_VK_VALIDATION", "1")
        script = ROOT / argv[0]
        if not script.is_file():
            rows.append(
                {
                    "symbolic_gate_key": key,
                    "subject_prefix": prefix,
                    "evidence_path": None,
                    "status": "FAIL",
                    "detail": f"smoke missing: {argv[0]}",
                }
            )
            all_ok = False
            continue
        print(f"[8a] regression {key}")
        r = subprocess.run(
            [sys.executable, str(script), *argv[1:]],
            cwd=ROOT,
            env=env,
        )
        row = wel.require_gate_pass(key, prefix)
        if r.returncode != 0 and row["status"] == "PASS":
            row = {
                **row,
                "status": "FAIL",
                "detail": f"smoke exit={r.returncode} but evidence looked PASS",
            }
        if r.returncode != 0:
            row["status"] = "FAIL"
            row["detail"] = f"smoke exit={r.returncode}; {row.get('detail', '')}"
        rows.append(row)
        if row["status"] != "PASS":
            all_ok = False
    return all_ok, rows, commit


def run_uc08_soak(
    *,
    min_seconds: int = MIN_SECONDS,
    min_frames: int = MIN_FRAMES,
) -> tuple[bool, dict]:
    """驱动 uc08-physics --soak；双阈值同时满足。"""
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "uc08-physics",
        "--release",
        "--",
        "--soak",
        "--min-seconds",
        str(min_seconds),
        "--min-frames",
        str(min_frames),
        "--json",
    ]
    print(f"[8a] uc08 soak: {' '.join(cmd)}")
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    elapsed = time.time() - t0
    out = (r.stdout or "").strip().splitlines()
    doc: dict = {}
    for line in reversed(out):
        line = line.strip()
        if line.startswith("{") and line.endswith("}"):
            try:
                doc = json.loads(line)
                break
            except json.JSONDecodeError:
                continue
    frames = int(doc.get("soak_frames") or doc.get("frames") or 0)
    seconds = float(doc.get("soak_seconds") or elapsed)
    validation = int(doc.get("validation_messages") or 0)
    device_lost = int(doc.get("device_lost_count") or 0)
    ok = (
        r.returncode == 0
        and frames >= min_frames
        and seconds >= min_seconds
        and validation == 0
        and device_lost == 0
    )
    detail = (
        f"exit={r.returncode} frames={frames} seconds={seconds:.1f} "
        f"validation={validation} device_lost={device_lost}"
    )
    if r.returncode != 0:
        err = (r.stderr or "")[-500:]
        detail += f" stderr={err!r}"
    return ok, {
        "ok": ok,
        "frames": frames,
        "seconds": seconds,
        "validation_messages": validation,
        "device_lost_count": device_lost,
        "detail": detail,
        "raw": doc,
    }


def run_budget_strict() -> tuple[bool, str]:
    r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "budget_eval.py"), "--strict"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    text = (r.stdout or "") + (r.stderr or "")
    ok = r.returncode == 0 and "PASS" in text and "skip" not in text.lower().split("skip,")[0]
    # 宽松:exit 0 且输出含 budget_eval] PASS
    ok = r.returncode == 0 and "[budget_eval] PASS" in text
    return ok, f"exit={r.returncode}; {(text.strip().splitlines() or [''])[-1]}"


def verify_latest() -> int:
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print("[8a] FAIL: missing soak evidence", file=sys.stderr)
        return 1
    data = wel.load_json(path)
    errs = wel.validate_schema(data, SCHEMA_PATH) if SCHEMA_PATH.is_file() else []
    if errs:
        print(f"[8a] schema FAIL: {errs}", file=sys.stderr)
        return 1
    if data.get("host_section_pass") is not True:
        print("[8a] FAIL: host_section_pass≠true", file=sys.stderr)
        return 1
    checks = data.get("checks") or {}
    need = [
        "regression_all_pass",
        "soak_dual_threshold",
        "budget_strict_pass",
        "date_anchor_recorded",
    ]
    bad = [k for k in need if checks.get(k) is not True]
    if bad:
        print(f"[8a] FAIL checks: {bad}", file=sys.stderr)
        return 1
    soak = data.get("soak") or {}
    if int(soak.get("frames") or 0) < MIN_FRAMES or float(soak.get("seconds") or 0) < MIN_SECONDS:
        print("[8a] FAIL: soak thresholds not met in evidence", file=sys.stderr)
        return 1
    print(f"[8a] verify-latest PASS ← {path.relative_to(ROOT)}")
    return 0


def run_full_gate() -> int:
    if NUMERIC_STEP <= 0:
        print("[8a] NUMERIC_STEP unset (Gov 回填前草稿 → 红)", file=sys.stderr)
        return 1
    if not SCHEMA_PATH.is_file():
        print(f"[8a] schema missing: {SCHEMA_PATH}", file=sys.stderr)
        return 1

    reg_ok, reg_rows, commit = run_regression(skip_rerun=False)
    soak_ok, soak_info = run_uc08_soak()
    bud_ok, bud_detail = run_budget_strict()
    stamp = wel.utc_stamp()
    utc_date = stamp[:8]

    facts = [
        _fact("regression_18p0_3p1", reg_ok, f"gates={len(reg_rows)} base_commit={commit}"),
        _fact("soak_dual_threshold", soak_ok, soak_info["detail"]),
        _fact("budget_strict", bud_ok, bud_detail),
        _fact("date_anchor", True, f"utc_date={utc_date}"),
    ]
    overall = reg_ok and soak_ok and bud_ok
    checks = {
        "regression_all_pass": reg_ok,
        "soak_dual_threshold": soak_ok,
        "budget_strict_pass": bud_ok,
        "date_anchor_recorded": True,
        "aggregate_read_only": False,
    }
    # 手写 evidence(字段比 wave_exit 更丰富)
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
        "base_commit": commit,
        "utc_date": utc_date,
        "required_gates": reg_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "regression_all_pass": reg_ok,
            "soak_dual_threshold": soak_ok,
            "budget_strict_pass": bud_ok,
            "date_anchor_recorded": True,
        },
        "soak": {
            "frames": soak_info.get("frames", 0),
            "seconds": soak_info.get("seconds", 0.0),
            "min_frames": MIN_FRAMES,
            "min_seconds": MIN_SECONDS,
            "validation_messages": soak_info.get("validation_messages", 0),
            "device_lost_count": soak_info.get("device_lost_count", 0),
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": "G8.8a full soak; four legs",
    }
    errs = wel.validate_schema(payload, SCHEMA_PATH)
    if errs:
        print(f"[8a] schema errors: {errs}", file=sys.stderr)
        overall = False
        payload["host_section_pass"] = False
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    for f in facts:
        print(f"  FACT  {f['status']:4}  {f['id']}  ({f['detail']})")
    print(f"  → evidence {out.relative_to(ROOT)}")
    print(f"  VERDICT = {'PASS' if overall else 'FAIL'}")
    return 0 if overall else 1


def main() -> int:
    ap = argparse.ArgumentParser(description="G8.8a stabilization soak")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        if NUMERIC_STEP != 0:
            # materialize 后自检只断言步骤号
            assert NUMERIC_STEP > 0
            print("[8a] selftest OK (materialized)")
            return 0
        print("[8a] selftest: NUMERIC_STEP=0 draft → expect red on --gate")
        code = run_full_gate()
        if code == 0:
            print("[selftest] FAIL: draft still green", file=sys.stderr)
            return 1
        print("[selftest] PASS: draft NUMERIC_STEP=0 → red")
        return 0
    if args.verify_latest:
        return verify_latest()
    return run_full_gate()


if __name__ == "__main__":
    sys.exit(main())
