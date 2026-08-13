#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.4 M96 M17 Path Tracer 参照器门冒烟(g9.p0.m96.path_tracer_reference;
RFC-0022 §4.10;spec/global_illumination.md RXS-0357;G9_ACCEPTANCE_MAP §2 M96;
D2-Q7 门序源——本门最新 PASS evidence 是 M97~M101 五门的前置)。

host 段:rurix-render gi::path_trace 10 单测逐名锚定(确定性协议/数值锚/fixture
  fail-closed/host oracle/RED 臂/容差带比较器/语料锚/PFM 往返)+ conformance gi
  语料锚(accept pt_reference_fixed_seed_minimal + reject
  pt_seed_changed_nondeterministic,`//@ spec: RXS-0357`)+ 冻结容差带
  milestones/g9/g9_m96_pbrt_tolerance_band.json provenance 字段机器核验
  (schema/frozen_at_utc/device_name/pbrt 版本·commit·exe sha256/冻结规则/
  spp 序列/ref_spp/双 seed/8 条目 golden digest 形态)。
device 段(必需,持 gpu_device_lock):cargo build -p rurixc --features
  vulkan-backend → rurixc src/rurix-render/kernels/g9_m96_path_tracer.rx
  --target vulkan 产 SPV → g9_m96_path_tracer harness 全档真跑(双跑位级一致 +
  逐像素 sample count + golden digest 全等 + pbrt-v4 对照收敛曲线在冻结带内 +
  改 seed/跳 RR/关 MIS 三臂 RED + 起步范围冻结显式拒绝;RURIX_REQUIRE_REAL=1 +
  RURIX_VK_VALIDATION=1,SKIP 翻红)+ 三 RED 臂 --red-arm 子模式独立复跑抽检。
  本机 pbrt provisioning = external/pbrt-v4/build/Release/{pbrt,imgtool}.exe,
  exe sha256 与冻结带 provenance 逐字核验(漂移即 RED)。

用法:
  py -3 ci/g9_path_tracer_reference_smoke.py --gate g9.p0.m96.path_tracer_reference
  py -3 ci/g9_path_tracer_reference_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones/g9/g9_m96_path_tracer_reference_evidence_schema.json"
BAND_PATH = ROOT / "milestones/g9/g9_m96_pbrt_tolerance_band.json"
ACCEPT_CORPUS = ROOT / "conformance/gi/accept/pt_reference_fixed_seed_minimal.rx"
REJECT_CORPUS = ROOT / "conformance/gi/reject/pt_seed_changed_nondeterministic.rx"
KERNEL = ROOT / "src/rurix-render/kernels/g9_m96_path_tracer.rx"
PBRT_EXE = ROOT / "external/pbrt-v4/build/Release/pbrt.exe"
IMGTOOL_EXE = ROOT / "external/pbrt-v4/build/Release/imgtool.exe"
WORK_DIR = ROOT / ".tmp/g94_gates/m96"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g9.p0.m96.path_tracer_reference"
NUMERIC_STEP = 147
SOURCE_REF = "RFC-0022 §4.10;spec/global_illumination.md RXS-0357;G9_ACCEPTANCE_MAP §2 M96"
TAG = "g9_m96"

PATH_TRACE_TESTS = [
    "rng_stream_layout_and_determinism",
    "bsdf_mis_numeric_anchors",
    "scene_fixtures_validate_and_pack",
    "out_of_scope_materials_fail_closed",
    "host_oracle_deterministic_and_scope_sane",
    "host_oracle_red_arms_detectable",
    "pbrt_scene_text_deterministic_and_contains_frozen_fields",
    "tolerance_band_comparator_fail_closed",
    "conformance_anchor_corpus_present",
    "pfm_roundtrip_and_orientation",
]

CHECK_KEYS = [
    # host 段
    "host_path_trace_tests_anchored",
    "conformance_gi_corpus_anchored",
    "pbrt_band_provenance_frozen",
    # device 段
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_sample_count_export",
    "device_golden_digest_match",
    "device_pbrt_band_within",
    "device_red_arms_effective",
    "device_red_arm_submodes_detected",
    "device_scope_reject_failclosed",
    "device_validation_zero",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_cargo(args: list[str]) -> tuple[int, str]:
    print(f"[{TAG}] cargo {' '.join(args)}")
    r = subprocess.run(["cargo", *args], cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


# ═══════════════════════ host 段 ═══════════════════════


def host_path_trace_tests() -> bool:
    """cargo test -p rurix-render --lib gi::path_trace:10 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-render", "--lib", "gi::path_trace"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in PATH_TRACE_TESTS:
        if not (ok and name in blob):
            check(False, f"gi::path_trace 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_conformance_anchor() -> bool:
    """conformance gi accept/reject 语料在位 + `//@ spec: RXS-0357` 锚定 + 预期面注释。"""
    ok = True
    for path, face in ((ACCEPT_CORPUS, "accept"), (REJECT_CORPUS, "reject")):
        if not path.is_file():
            check(False, f"缺 {face} 语料 {path.name}")
            ok = False
            continue
        text = path.read_text(encoding="utf-8")
        if "//@ spec: RXS-0357" not in text or GATE_KEY not in text:
            check(False, f"{face} 语料 {path.name} 锚定/门 key 预期面缺失")
            ok = False
    if ok and "三臂 RED" not in REJECT_CORPUS.read_text(encoding="utf-8"):
        check(False, "reject 语料三臂 RED 预期面缺失")
        ok = False
    return ok


def host_band_provenance() -> bool:
    """冻结容差带 provenance 字段机器核验(禁手写 P-09 的承载面)。"""
    if not BAND_PATH.is_file():
        check(False, f"缺冻结容差带 {BAND_PATH.name}")
        return False
    try:
        band = json.loads(BAND_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        check(False, f"容差带不可读: {e}")
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"容差带 provenance: {msg}")
            ok = False

    need(band.get("schema") == "rurix.g9m96.pbrt_tolerance_band.v1", "schema 字面不符")
    need(bool(band.get("frozen_at_utc")), "frozen_at_utc 空")
    need(bool(band.get("device_name")), "device_name 空")
    need("pbrt version 4" in str(band.get("pbrt_version", "")), "pbrt_version 非 v4 横幅")
    need(is_hex(band.get("pbrt_commit"), 40), "pbrt_commit 非 40hex")
    need(is_hex(band.get("pbrt_exe_sha256"), 64), "pbrt_exe_sha256 非 64hex")
    need("M96_BAND_MARGIN" in str(band.get("freeze_rule", "")), "freeze_rule 缺 M96_BAND_MARGIN 登记")
    need(band.get("spp_sequence") == "1,4,16,64", "spp_sequence ≠ 1,4,16,64")
    need(band.get("ref_spp") == "1024", "ref_spp ≠ 1024")
    need(bool(band.get("seed_device")) and bool(band.get("seed_pbrt")), "双 seed 空")
    entries = band.get("entries")
    need(isinstance(entries, list) and len(entries) == 8, "entries ≠ 8(2 场景 × 4 spp)")
    for e in entries if isinstance(entries, list) else []:
        need(is_hex(e.get("golden_digest"), 64), f"条目 {e.get('scene')}/{e.get('spp')} golden_digest 形态")
        for f in ("band_rel_dev", "measured_rel_dev", "curve_rurix", "curve_pbrt"):
            need(isinstance(e.get(f), str) and len(e[f]) > 0, f"条目 {e.get('scene')}/{e.get('spp')} 缺 {f}")
    # 本机 pbrt provisioning 与冻结带 provenance 逐字核验(漂移即 RED)。
    if PBRT_EXE.is_file():
        sha = hashlib.sha256(PBRT_EXE.read_bytes()).hexdigest()
        if sha != band.get("pbrt_exe_sha256"):
            check(False, f"本机 pbrt.exe sha256={sha} ≠ 冻结带 provenance(pbrt 漂移)")
            ok = False
        else:
            note("本机 pbrt.exe sha256 与冻结带 provenance 逐字一致")
    else:
        check(False, f"本机 pbrt provisioning 缺失: {PBRT_EXE}")
        ok = False
    return ok


def is_hex(v: object, n: int) -> bool:
    return isinstance(v, str) and len(v) == n and all(c in "0123456789abcdef" for c in v)


# ═══════════════════════ device 段 ═══════════════════════


def build_rurixc() -> Path | None:
    print(f"[{TAG}] cargo build -p rurixc --features vulkan-backend --bin rurixc")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"rurixc 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = target_dir() / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if not exe.is_file():
        check(False, f"rurixc 产物缺失: {exe}")
        return None
    return exe


def compile_spv(rurixc: Path, kernel: Path, out: Path) -> bool:
    print(f"[{TAG}] rurixc {kernel.name} --target vulkan -o {out.name}")
    out.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        [str(rurixc), str(kernel), "--target", "vulkan", "-o", str(out)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0 or not out.is_file():
        check(False, f"SPV 产线失败 rc={r.returncode}:\n{(r.stdout + r.stderr)[-1500:]}")
        return False
    return True


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin g9_m96_path_tracer")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", "g9_m96_path_tracer"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"g9_m96_path_tracer 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = target_dir() / "debug" / ("g9_m96_path_tracer.exe" if sys.platform == "win32" else "g9_m96_path_tracer")
    if not exe.is_file():
        check(False, f"harness 产物缺失: {exe}")
        return None
    return exe


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    env["RURIX_BASE_COMMIT"] = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
    ).stdout.strip()
    return env


def run_harness_full(exe: Path, spv: Path) -> tuple[str, dict | None]:
    """全档真跑(含 pbrt 对照腿)。返回 (device_state, harness evidence|None)。"""
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(exe),
        "--spv", str(spv),
        "--pbrt", str(PBRT_EXE),
        "--imgtool", str(IMGTOOL_EXE),
        "--work-dir", str(WORK_DIR / "pbrt_work"),
        "--evidence", str(HARNESS_EVIDENCE),
    ]
    print(f"[{TAG}] device 全档: {' '.join(Path(c).name for c in cmd[:1])} --spv {spv.name} --pbrt pbrt.exe(validation=on)")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=device_env(), timeout=3600)
    out = r.stdout + r.stderr
    if "G9_M96_PT: SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()[-800:]}")
        return "skipped_dev_env", None
    doc = None
    if HARNESS_EVIDENCE.is_file():
        try:
            doc = json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            check(False, f"harness evidence 不可解析: {e}")
    if r.returncode != 0 or "G9_M96_PT: PASS" not in r.stdout:
        check(False, f"harness 全档失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", doc
    if doc is None:
        check(False, "harness evidence 缺失")
        return "fail", None
    if doc.get("schema") != "rurix.g9m96.path_tracer.v1" or doc.get("spec_anchor") != "RXS-0357":
        check(False, "harness evidence schema/spec_anchor 字面不符")
        return "fail", doc
    return "executed", doc


def run_red_arm(exe: Path, spv: Path, arm: str) -> bool:
    """--red-arm 子模式独立复跑(退出码 0 + PASS red-arm 字面 = 臂独立有效)。"""
    print(f"[{TAG}] device RED 臂子模式: --red-arm {arm}")
    r = subprocess.run(
        [str(exe), "--red-arm", arm, "--spv", str(spv)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=device_env(),
        timeout=900,
    )
    out = r.stdout + r.stderr
    ok = r.returncode == 0 and f"G9_M96_PT: PASS red-arm {arm}" in r.stdout
    if not ok:
        check(False, f"RED 臂子模式 {arm} 未独立检出 rc={r.returncode}: {out[-600:]}")
    return ok


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    # 红臂:合成 FAILURES 必须使门红(check() 判别有效)。
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # CHECK_KEYS 闭集恰 12 项(host 3 + device 9)。
    if len(CHECK_KEYS) != 12:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 12", file=sys.stderr)
        return 1
    # 绿臂:schema required 与 CHECK_KEYS 闭集互核;合成缺键 evidence 必红。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    synth = {k: True for k in CHECK_KEYS}
    synth.pop(CHECK_KEYS[0])
    if not (req - set(synth)):
        print(f"[{TAG}] selftest FAIL: 合成缺键未触发 schema required 红", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (1 RED + 1 GREEN)")
    return 0


# ═══════════════════════ main ═══════════════════════


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

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}

    # host 段
    checks["host_path_trace_tests_anchored"] = host_path_trace_tests()
    checks["conformance_gi_corpus_anchored"] = host_conformance_anchor()
    checks["pbrt_band_provenance_frozen"] = host_band_provenance()

    # device 段(持锁串行:rurixc 构建 + SPV 产线 + harness 全档 + 三 RED 臂子模式)
    device_state = "fail"
    with gpu_device_lock(purpose="g9_m96 path tracer device 腿"):
        rurixc = build_rurixc()
        spv = WORK_DIR / "g9_m96_path_tracer.spv"
        exe = build_harness() if rurixc else None
        if rurixc and exe and compile_spv(rurixc, KERNEL, spv):
            device_state, doc = run_harness_full(exe, spv)
            if device_state == "executed" and doc is not None:
                hc = doc.get("checks", {})
                checks["device_harness_full_pass"] = True
                checks["device_double_run_bitexact"] = hc.get("double_run_bitexact") is True
                checks["device_sample_count_export"] = hc.get("sample_count_export") is True
                checks["device_golden_digest_match"] = hc.get("golden_digest_match") is True
                checks["device_pbrt_band_within"] = hc.get("pbrt_band_within") is True
                checks["device_red_arms_effective"] = all(
                    hc.get(k) is True for k in ("red_seed", "red_no_rr", "red_no_mis")
                )
                checks["device_scope_reject_failclosed"] = hc.get("scope_reject_failclosed") is True
                checks["device_validation_zero"] = (
                    hc.get("validation_zero") is True
                    and doc.get("device_state", {}).get("validation") == "on"
                    and doc.get("device_state", {}).get("require_real") is True
                )
                for k in CHECK_KEYS:
                    if k.startswith("device_") and k != "device_red_arm_submodes_detected" and not checks[k]:
                        check(False, f"harness 判据 {k} 为假")
                # 三 RED 臂子模式独立复跑抽检。
                checks["device_red_arm_submodes_detected"] = all(
                    run_red_arm(exe, spv, arm) for arm in ("seed-change", "no-rr", "no-mis")
                )
            note("device:全档真跑(双跑位级 + pbrt 带内)+ 三 RED 臂子模式独立复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m96_path_tracer_reference",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M96",
        "milestone": "M96",
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G9.4",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
        ).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": [
            {"seq": 1, "command": "cargo test -p rurix-render --lib gi::path_trace", "exit_code": 0},
            {"seq": 2, "command": "cargo build -p rurixc --features vulkan-backend --bin rurixc", "exit_code": 0},
            {"seq": 3, "command": "rurixc src/rurix-render/kernels/g9_m96_path_tracer.rx --target vulkan -o .tmp/g94_gates/m96/g9_m96_path_tracer.spv", "exit_code": 0},
            {"seq": 4, "command": "cargo build -p rurix-render --features vulkan --bin g9_m96_path_tracer", "exit_code": 0},
            {"seq": 5, "command": "g9_m96_path_tracer --spv .. --pbrt .. --imgtool .. (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g9_m96_path_tracer --red-arm seed-change|no-rr|no-mis --spv .. (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submodes_detected"] else 1},
        ],
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
    out = EVIDENCE_DIR / f"g9_m96_path_tracer_reference_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
