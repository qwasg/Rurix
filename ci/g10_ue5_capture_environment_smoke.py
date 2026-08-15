#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.2 波 materialize）
"""G10.2 M128 UE5 出图环境硬门冒烟（步骤 177；g10.p0.m128.ue5_capture_environment；
RFC-0027 §4.1；spec/external_reference.md RXS-0380；G10_ACCEPTANCE_MAP §1 M128 行
判据逐字；环境事实源 design/g10_2_environment_log.md）。

host 编排 + device（UE 侧 GPU 渲染）门：spike 裁决路径落地（②Launcher 安装
UE 5.8.1-56057345 @ F:\\UE_5.8，Build.version 实测）+ 固定场景（Entry 静态
空图，暂定场景集 g10_2_provisional_scene_set.json 登记）MRQ 臂真出帧 +
环境画像七元组（ue_build_id/驱动/锁频状态/场景/相机 digest/光照 digest/
capture_arm）随 evidence 存档。

RED 臂（契约 §4.2 M128 字面 + PLAN §3 草案补充）：
  red_nonzero_exit_masquerade.json   出帧进程非零退出冒充成功
  red_preplaced_fake_frames.json     预置假帧冒充真出帧
  red_profile_missing_field.json     环境画像缺字段
另：live RED 探针——UE 以不存在工程路径真调（token 合法）→ 非零退出 →
判定层必须检出（真子进程路径实证，非纯夹具）。

UE 安装缺失/出帧失败 = DEV_ENV_DEGRADE 诚实登记不充绿（Epic 人工接管点
本机已完成，环境日志 §2/§3.3）；HighResShot 臂时序不稳与 -csvCaptureFrames
死路已登记（环境日志 §7.1），本门不复活作证据面。

用法：
  py -3 ci/g10_ue5_capture_environment_smoke.py --gate g10.p0.m128.ue5_capture_environment
  py -3 ci/g10_ue5_capture_environment_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m128_ue5_capture_environment_evidence_schema.json"
RED_FIXTURE_DIR = ROOT / "milestones" / "g10" / "red_fixtures" / "m128"

sys.path.insert(0, str(ROOT / "ci"))
import g10_ue5_lib as lib  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g10.p0.m128.ue5_capture_environment"
NUMERIC_STEP = 177
SOURCE_REF = "RFC-0027 §4.1;spec/external_reference.md RXS-0380;G10_ACCEPTANCE_MAP §1 M128"
TAG = "g10_m128"
SUBJECT = "g10_m128_ue5_capture_environment"
MATRIX_ROW = "M128"
SCENE_ID = "entry-empty-static"

CHECK_KEYS = [
    "spec_rxs0380_clause_on_tree",
    "ue_install_build_id_measured",
    "uproject_and_mrq_assets_present",
    "gpu_profile_collected",
    "real_capture_exit_zero",
    "fresh_frames_produced",
    "frames_are_real_exr",
    "profile_septuple_complete",
    "red_fixtures_all_detected",
    "live_red_probe_detected",
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


def evaluate_capture_receipt(receipt: dict) -> list[str]:
    """出帧 receipt 判定层（纯函数，RED 夹具与真跑共用）。

    receipt = {exit_code, new_frames[], preplaced_frames[], frames[], provenance{}}。
    非零退出冒充成功 / 预置假帧冒充真出帧 / 画像缺字段各为独立失败。
    """
    fails: list[str] = []
    if receipt.get("exit_code") != 0:
        fails.append(f"出帧进程非零退出冒充成功: exit_code={receipt.get('exit_code')}")
    new_frames = receipt.get("new_frames") or []
    preplaced = receipt.get("preplaced_frames") or []
    if not new_frames and preplaced:
        fails.append(f"预置假帧冒充真出帧: 零新出帧而预置帧 {len(preplaced)} 件在场")
    if not new_frames and not preplaced and receipt.get("exit_code") == 0:
        fails.append("零新出帧（exit 0 而无新帧，不构成出帧成功）")
    for f in lib.frames_are_real(receipt.get("frames") or []):
        fails.append(f)
    for f in lib.provenance_failures(receipt.get("provenance") or {}):
        fails.append(f)
    return fails


def leg_red_fixtures() -> bool:
    """三件 RED 夹具逐件必须由判定层独立检出（RED 先行）。"""
    ok_all = True
    fixtures = sorted(RED_FIXTURE_DIR.glob("*.json"))
    if len(fixtures) < 3:
        check(False, f"RED 夹具不足 3 件: {len(fixtures)}")
        return False
    for fp in fixtures:
        doc = lib.load_json(fp)
        name = fp.stem
        want = doc.get("expect_failure_substring", "")
        fails = evaluate_capture_receipt(doc.get("receipt") or {})
        if not fails:
            check(False, f"{name}: RED 夹具未被判定层检出（假绿口）")
            ok_all = False
        elif want and not any(want in f for f in fails):
            check(False, f"{name}: 检出但未命中臂语义 {want!r}: {fails[:2]}")
            ok_all = False
        else:
            note(f"RED 检出 {name}: {fails[0]}")
    return ok_all


def leg_live_red_probe(ue_exe: Path) -> bool:
    """live RED 探针：不存在工程路径真调 UE → 非零退出 → 判定层检出。"""
    bogus = Path(r"K:\rurix-ext\g10-ue\NoSuchProject.uproject")
    argv = [str(ue_exe), str(bogus), lib.MRQ_MAP_ENTRY, "-game", "-unattended", "-log"]
    with gpu_device_lock(purpose="g10.2 M128 live RED 探针（非零退出臂）"):
        res = lib.run_process(argv, timeout_s=300)
    COMMANDS.append(
        {"seq": len(COMMANDS) + 1, "command": " ".join(argv), "exit_code": res["exit_code"]}
    )
    receipt = {"exit_code": res["exit_code"], "new_frames": [], "preplaced_frames": [],
               "frames": [], "provenance": {}}
    fails = evaluate_capture_receipt(receipt)
    if res["exit_code"] == 0:
        check(False, "live 探针: 不存在工程竟 exit 0（探针失效）")
        return False
    if not any("非零退出" in f for f in fails):
        check(False, f"live 探针: 非零退出未被判定层检出: {fails[:2]}")
        return False
    note(f"live RED 探针检出: exit={res['exit_code']}（{res['duration_s']}s）→ {fails[0]}")
    return True


def run_selftest() -> int:
    # 红臂①：合成 FAILURES 必须使门红。
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 红臂②③④：三类缺陷 receipt 必须各自检出；绿臂：合形 receipt 零失败。
    good_frames = [{"name": ".0000.exr", "bytes": 16_609_821, "exr_magic_ok": True,
                    "canonical_digest": "a" * 64}]
    good_prov = {k: "x" for k in lib.PROVENANCE_SEPTUPLE}
    good = {"exit_code": 0, "new_frames": [".0000.exr"], "preplaced_frames": [],
            "frames": good_frames, "provenance": good_prov}
    if evaluate_capture_receipt(good):
        print(f"[{TAG}] selftest FAIL: 合形 receipt 被误判: {evaluate_capture_receipt(good)}", file=sys.stderr)
        return 1
    if not any("非零退出" in f for f in evaluate_capture_receipt({**good, "exit_code": 1})):
        print(f"[{TAG}] selftest FAIL: 非零退出臂未检出", file=sys.stderr)
        return 1
    if not any("预置假帧" in f for f in evaluate_capture_receipt(
            {**good, "new_frames": [], "preplaced_frames": ["old.exr"], "frames": []})):
        print(f"[{TAG}] selftest FAIL: 预置假帧臂未检出", file=sys.stderr)
        return 1
    bad_prov = dict(good_prov)
    del bad_prov["capture_arm"]
    if not any("capture_arm" in f for f in evaluate_capture_receipt({**good, "provenance": bad_prov})):
        print(f"[{TAG}] selftest FAIL: 画像缺字段臂未检出", file=sys.stderr)
        return 1
    # 绿臂：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (4 RED + 2 GREEN)")
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
    capture_report: dict = {}

    # ① spec-first 门序：RXS-0380 条款头在树。
    checks["spec_rxs0380_clause_on_tree"] = lib.spec_clause_head_on_tree(
        "spec/external_reference.md", "RXS-0380"
    )
    check(checks["spec_rxs0380_clause_on_tree"], "spec/external_reference.md 缺 RXS-0380 条款头")

    # ② UE 安装实测：Build.version → ue_build_id。
    ue_exe = lib.ue_editor_cmd()
    ue_build_id = lib.read_ue_build_id(ue_exe) if ue_exe else None
    checks["ue_install_build_id_measured"] = ue_build_id == lib.EXPECTED_UE_BUILD_ID
    check(ue_build_id is not None, "UnrealEditor-Cmd 缺失或 Build.version 不可读（dev_env_degrade）")
    check(
        ue_build_id == lib.EXPECTED_UE_BUILD_ID,
        f"ue_build_id 实测 {ue_build_id!r} ≠ 登记 {lib.EXPECTED_UE_BUILD_ID!r}",
    )

    # ③ 工程与 MRQ 资产在位。
    uproject = lib.uproject_path()
    checks["uproject_and_mrq_assets_present"] = uproject is not None and lib.mrq_assets_present()
    check(checks["uproject_and_mrq_assets_present"], "uproject 或 G10_SmokeSeq/G10_SmokeConfig 资产缺失")

    # ④ GPU 环境画像采集。
    profile = lib.gpu_profile()
    checks["gpu_profile_collected"] = profile is not None
    check(profile is not None, "nvidia-smi 环境画像采集失败")

    frames_root = lib.frames_root()
    env_ready = (
        ue_exe is not None and ue_build_id == lib.EXPECTED_UE_BUILD_ID
        and uproject is not None and checks["uproject_and_mrq_assets_present"]
        and profile is not None and frames_root is not None
    )

    if env_ready:
        # ⑤ 真出帧：MRQ Phase B（GPU 锁串行；新鲜度机核防预置假帧）。
        run_started = time.time()
        receipt = lib.run_mrq_phase_b(ue_exe, uproject, gpu_device_lock)
        COMMANDS.append(
            {"seq": len(COMMANDS) + 1, "command": " ".join(receipt["argv"]),
             "exit_code": receipt["exit_code"]}
        )
        checks["real_capture_exit_zero"] = receipt["exit_code"] == 0
        check(checks["real_capture_exit_zero"],
              f"出帧进程非零退出: exit={receipt['exit_code']}; tail={receipt['output_tail'][-300:]}")

        output_dir = frames_root / "smoke"
        run_dir = frames_root / "g10_gate_runs" / f"m128_{_dt.datetime.now(_dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ')}"
        frames = lib.harvest_new_frames(output_dir, run_started, run_dir) if output_dir.is_dir() else []
        checks["fresh_frames_produced"] = len(frames) >= 1
        check(checks["fresh_frames_produced"], "零新出帧（mtime 新鲜度机核：预置帧不充数）")

        # ⑥ 真帧判据（EXR magic + 体积下限 + canonical digest 可算）。
        real_fails = lib.frames_are_real(frames)
        checks["frames_are_real_exr"] = not real_fails
        for f in real_fails:
            check(False, f)

        # ⑦ 环境画像七元组随 evidence 存档。
        capture_arm = f"A(mrq):{receipt['command_surface_digest']}"
        provenance = lib.build_provenance(SCENE_ID, ue_build_id, profile, capture_arm)
        prov_fails = lib.provenance_failures(provenance)
        checks["profile_septuple_complete"] = not prov_fails
        for f in prov_fails:
            check(False, f)

        capture_report = {
            "ue_build_id": ue_build_id,
            "scene_id": SCENE_ID,
            "map": lib.MRQ_MAP_ENTRY,
            "capture_arm": capture_arm,
            "provenance": provenance,
            "exit_code": receipt["exit_code"],
            "duration_s": receipt["duration_s"],
            "frames_dir": str(run_dir),
            "frames": frames,
            "gpu_profile": profile,
        }
        device_state = "executed" if receipt["exit_code"] == 0 and frames else "dev_env_degrade"
        if device_state != "executed":
            note("UE 出帧未成功 → dev_env_degrade 诚实登记（不充绿）")
        # ⑧ live RED 探针（真子进程非零退出检出）。
        checks["live_red_probe_detected"] = leg_live_red_probe(ue_exe)
    else:
        device_state = "dev_env_degrade"
        note("UE 出图环境不齐（安装/工程/画像/帧库任一缺失）→ dev_env_degrade 诚实登记（不充绿）")

    # ⑨ RED 夹具逐件检出（不依赖 UE 环境，恒跑）。
    checks["red_fixtures_all_detected"] = leg_red_fixtures()

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
        "capture_report": capture_report,
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
        print(f"[{TAG}] PASS（UE {ue_build_id} MRQ 臂真出帧 {len(capture_report.get('frames', []))} 帧 + 画像七元组存档 + RED 三夹具 + live 探针全检出）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
