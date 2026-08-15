#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.2 波 materialize）
"""G10.2 M129 UE5 参考帧硬门冒烟（步骤 178；g10.p0.m129.ue5_reference_frames；
RFC-0027 §4.1/§4.4；spec/external_reference.md RXS-0380；G10_ACCEPTANCE_MAP §1
M129 行判据逐字；环境事实源 design/g10_2_environment_log.md §7.3）。

host+device 门：暂定场景集（g10_2_provisional_scene_set.json，RFC-0027 §4.4 F8
时序形态，单场景闭集 entry-empty-static + 偏差如实登记）逐场景参考帧落盘 +
同参数双跑 canonical digest 一致（复用 harness g10_determinism 实测过的 14
属性剥离逻辑，单一事实源）+ provenance 七元组登记闭集（场景/相机 digest/
光照 digest/build/驱动/锁频/臂）。

RED 臂（契约 §4.2 M129 字面 + PLAN §3 草案补充）：
  red_double_run_digest_unequal.json   双跑 digest 不等（Template_Default 动画
                                       帧 vs Entry 静态帧真实帧对，环境日志
                                       §7.3 实证其不等）
  red_provenance_missing_row.json      provenance 缺行
  red_frame_tampered.json              帧文件篡改（真帧像素区翻字节 ⇒ canonical
                                       digest 相对登记值漂移必须检出）

UE 出帧失败 = DEV_ENV_DEGRADE 诚实登记不充绿；HighResShot 臂与
-csvCaptureFrames 死路不复活作证据面（环境日志 §7.1 钉死）。

用法：
  py -3 ci/g10_ue5_reference_frames_smoke.py --gate g10.p0.m129.ue5_reference_frames
  py -3 ci/g10_ue5_reference_frames_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m129_ue5_reference_frames_evidence_schema.json"
RED_FIXTURE_DIR = ROOT / "milestones" / "g10" / "red_fixtures" / "m129"

sys.path.insert(0, str(ROOT / "ci"))
import g10_ue5_lib as lib  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g10.p0.m129.ue5_reference_frames"
NUMERIC_STEP = 178
SOURCE_REF = "RFC-0027 §4.1/§4.4;spec/external_reference.md RXS-0380;G10_ACCEPTANCE_MAP §1 M129"
TAG = "g10_m129"
SUBJECT = "g10_m129_ue5_reference_frames"
MATRIX_ROW = "M129"

CHECK_KEYS = [
    "spec_rxs0380_clause_on_tree",
    "provisional_scene_set_registered",
    "double_run_capture_exit_zero",
    "per_scene_reference_frames",
    "double_run_canonical_digest_equal",
    "provenance_septuple_closed",
    "red_fixtures_all_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def compare_double_run(frames_a: list[dict], frames_b: list[dict]) -> list[str]:
    """双跑 digest 判定层（纯函数）：同名帧 canonical digest 逐帧相等；不等即失败。"""
    fails: list[str] = []
    a = {f["name"]: f["canonical_digest"] for f in frames_a}
    b = {f["name"]: f["canonical_digest"] for f in frames_b}
    if not a:
        fails.append("run_a 零帧")
    if not b:
        fails.append("run_b 零帧")
    only_a = sorted(set(a) - set(b))
    only_b = sorted(set(b) - set(a))
    if only_a:
        fails.append(f"帧集不对账: 仅 run_a 有 {only_a}")
    if only_b:
        fails.append(f"帧集不对账: 仅 run_b 有 {only_b}")
    for name in sorted(set(a) & set(b)):
        if a[name] != b[name]:
            fails.append(f"双跑 digest 不等: {name}（{a[name][:16]}… ≠ {b[name][:16]}…）")
    return fails


def validate_scene_set(doc: dict) -> list[str]:
    """暂定场景集登记面核验：结构闭集 + 单场景 ready + 偏差登记非空。"""
    fails: list[str] = []
    if doc.get("kind") != "g10_2_provisional_scene_set":
        fails.append("kind ≠ g10_2_provisional_scene_set")
    scenes = doc.get("scenes") or []
    ready = [s for s in scenes if s.get("status") == "ready"]
    if not scenes:
        fails.append("场景集为空（vacuous 不充绿）")
    if len(ready) != len(scenes):
        fails.append("存在非 ready 场景行未显式登记")
    for s in scenes:
        for k in ("scene_id", "map", "status", "note"):
            if not str(s.get(k, "")).strip():
                fails.append(f"场景行缺字段 {k}")
    if not str(doc.get("deviation_note", "")).strip():
        fails.append("deviation_note 缺失（CornellBox/Bistro 缺口未如实登记）")
    return fails


def leg_red_fixtures(frames_root: Path | None) -> bool:
    """三件 RED 夹具逐件独立检出。"""
    ok_all = True
    fixtures = sorted(RED_FIXTURE_DIR.glob("*.json"))
    if len(fixtures) < 3:
        check(False, f"RED 夹具不足 3 件: {len(fixtures)}")
        return False
    for fp in fixtures:
        doc = lib.load_json(fp)
        name = fp.stem
        if name == "red_double_run_digest_unequal":
            if frames_root is None:
                check(False, f"{name}: 帧库根不可达，无法复算")
                ok_all = False
                continue
            fa = frames_root / doc["run_a_frame"]
            fb = frames_root / doc["run_b_frame"]
            da = [{"name": fa.name, "canonical_digest": lib.exr_canonical_digest(fa)}]
            db = [{"name": fa.name, "canonical_digest": lib.exr_canonical_digest(fb)}]
            fails = compare_double_run(da, db)
            if not any("不等" in f for f in fails):
                check(False, f"{name}: 真实不等帧对未被判定层检出（夹具失效）")
                ok_all = False
            else:
                note(f"RED 检出 {name}: {fails[0]}")
        elif name == "red_provenance_missing_row":
            rows = doc.get("provenance_rows") or []
            fails: list[str] = []
            for row in rows:
                fails.extend(lib.provenance_failures(row))
            if not fails:
                check(False, f"{name}: provenance 缺行未被检出（假绿口）")
                ok_all = False
            else:
                note(f"RED 检出 {name}: {fails[0]}")
        elif name == "red_frame_tampered":
            if frames_root is None:
                check(False, f"{name}: 帧库根不可达，无法复算")
                ok_all = False
                continue
            src = frames_root / doc["frame"]
            want = doc["expected_canonical_digest"]
            got = lib.exr_canonical_digest(src)
            if got != want:
                check(False, f"{name}: 夹具基准 digest 漂移 {got[:16]}… ≠ 登记 {want[:16]}…")
                ok_all = False
                continue
            with tempfile.TemporaryDirectory(prefix="g10_m129_tamper_") as td:
                tpath = Path(td) / src.name
                shutil.copy2(src, tpath)
                blob = bytearray(tpath.read_bytes())
                blob[len(blob) - int(doc.get("tamper_offset_from_end", 1024))] ^= 0xFF
                tpath.write_bytes(bytes(blob))
                tgot = lib.exr_canonical_digest(tpath)
            if tgot == want:
                check(False, f"{name}: 像素区篡改未引起 canonical digest 漂移（判定失效）")
                ok_all = False
            else:
                note(f"RED 检出 {name}: 篡改后 digest {tgot[:16]}… ≠ 登记 {want[:16]}…")
        else:
            check(False, f"未知 RED 夹具: {name}")
            ok_all = False
    return ok_all


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 红臂①：digest 不等必检出；②：帧集不对账必检出；③：空 run 必检出；绿臂：全等判零失败。
    fa = [{"name": "a.exr", "canonical_digest": "1" * 64}]
    fb = [{"name": "a.exr", "canonical_digest": "2" * 64}]
    if not any("不等" in f for f in compare_double_run(fa, fb)):
        print(f"[{TAG}] selftest FAIL: digest 不等臂未检出", file=sys.stderr)
        return 1
    if not compare_double_run(fa, []):
        print(f"[{TAG}] selftest FAIL: 零帧臂未检出", file=sys.stderr)
        return 1
    if compare_double_run(fa, fa):
        print(f"[{TAG}] selftest FAIL: 全等双跑被误判", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (3 RED + 2 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    device_state = "not_applicable"
    reference_report: dict = {}

    checks["spec_rxs0380_clause_on_tree"] = lib.spec_clause_head_on_tree(
        "spec/external_reference.md", "RXS-0380"
    )
    check(checks["spec_rxs0380_clause_on_tree"], "spec/external_reference.md 缺 RXS-0380 条款头")

    # 暂定场景集登记（RFC-0027 §4.4 F8 形态 + 偏差如实登记）。
    scene_doc = lib.load_json(lib.PROVISIONAL_SCENE_SET_PATH) if lib.PROVISIONAL_SCENE_SET_PATH.is_file() else {}
    ss_fails = validate_scene_set(scene_doc)
    checks["provisional_scene_set_registered"] = not ss_fails
    for f in ss_fails:
        check(False, f)
    ready_scenes = [s for s in (scene_doc.get("scenes") or []) if s.get("status") == "ready"]

    ue_exe = lib.ue_editor_cmd()
    ue_build_id = lib.read_ue_build_id(ue_exe) if ue_exe else None
    uproject = lib.uproject_path()
    profile = lib.gpu_profile()
    frames_root = lib.frames_root()
    env_ready = (
        ue_exe is not None and ue_build_id == lib.EXPECTED_UE_BUILD_ID
        and uproject is not None and lib.mrq_assets_present()
        and profile is not None and frames_root is not None and bool(ready_scenes)
    )

    if env_ready:
        stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        output_dir = frames_root / "smoke"
        receipts: list[dict] = []
        run_frames: dict[str, list[dict]] = {}
        for run_id in ("run_a", "run_b"):
            started = time.time()
            receipt = lib.run_mrq_phase_b(ue_exe, uproject, gpu_device_lock)
            COMMANDS.append(
                {"seq": len(COMMANDS) + 1, "command": " ".join(receipt["argv"]),
                 "exit_code": receipt["exit_code"]}
            )
            receipts.append(receipt)
            dest = frames_root / "g10_gate_runs" / f"m129_{stamp}" / run_id
            run_frames[run_id] = lib.harvest_new_frames(output_dir, started, dest)
        checks["double_run_capture_exit_zero"] = all(r["exit_code"] == 0 for r in receipts)
        check(checks["double_run_capture_exit_zero"],
              f"双跑出帧非零退出: {[r['exit_code'] for r in receipts]}")

        # 逐场景参考帧落盘（暂定场景集闭集逐场景）。
        scene_rows: list[dict] = []
        per_scene_ok = True
        for s in ready_scenes:
            ok = bool(run_frames["run_a"]) and bool(run_frames["run_b"])
            per_scene_ok = per_scene_ok and ok
            scene_rows.append({
                "scene_id": s["scene_id"], "map": s["map"],
                "run_a_frames": len(run_frames["run_a"]),
                "run_b_frames": len(run_frames["run_b"]),
                "reference_frames_on_disk": ok,
            })
        checks["per_scene_reference_frames"] = per_scene_ok
        check(per_scene_ok, "存在场景行参考帧未落盘")

        # 同参数双跑 canonical digest 一致。
        dr_fails = compare_double_run(run_frames["run_a"], run_frames["run_b"])
        checks["double_run_canonical_digest_equal"] = not dr_fails
        for f in dr_fails:
            check(False, f)
        if not dr_fails:
            note(f"双跑 canonical digest 一致: {len(run_frames['run_a'])}/{len(run_frames['run_a'])} 帧 MATCH")

        # provenance 七元组闭集（逐帧登记）。
        capture_arm = f"A(mrq):{receipts[0]['command_surface_digest']}"
        prov_rows: list[dict] = []
        prov_ok = True
        for s in ready_scenes:
            for fr in run_frames["run_a"]:
                prov = lib.build_provenance(s["scene_id"], ue_build_id, profile, capture_arm)
                pf = lib.provenance_failures(prov)
                prov_ok = prov_ok and not pf
                for x in pf:
                    check(False, x)
                prov_rows.append({"frame": fr["name"], "canonical_digest": fr["canonical_digest"],
                                  "provenance": prov})
        checks["provenance_septuple_closed"] = prov_ok

        reference_report = {
            "ue_build_id": ue_build_id,
            "scene_set_revision": scene_doc.get("revision"),
            "scenes": scene_rows,
            "capture_arm": capture_arm,
            "double_run_digests": {
                fr["name"]: fr["canonical_digest"] for fr in run_frames["run_a"]
            },
            "provenance_rows": prov_rows,
            "gpu_profile": profile,
            "durations_s": [r["duration_s"] for r in receipts],
        }
        both_ok = all(r["exit_code"] == 0 for r in receipts) and not dr_fails
        device_state = "executed" if both_ok else "dev_env_degrade"
        if device_state != "executed":
            note("UE 双跑出帧/digest 比对未全绿 → dev_env_degrade 诚实登记（不充绿）")
    else:
        device_state = "dev_env_degrade"
        note("UE 出图环境不齐 → dev_env_degrade 诚实登记（不充绿）")

    checks["red_fixtures_all_detected"] = leg_red_fixtures(frames_root)

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G10.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "reference_report": reference_report,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（双跑 {len(reference_report.get('double_run_digests', {}))} 帧 canonical digest 一致 + provenance 闭集 + RED 三夹具全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
