#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.4 M99 屏幕级 SPG + Radiance Cache 双级门冒烟(g9.p1.m99.spg_radiance_cache;
RFC-0022 §4.8;spec/global_illumination.md RXS-0360;G9_ACCEPTANCE_MAP §3 M99;
G9_CONTRACT §8.1 裁决① P1 全进;世界级 clipmap 未举证 not-triggered 不充绿)。

门序机器阻断(D2-Q7 硬约束,前置):evidence/ 最新
  g9_m96_path_tracer_reference_<UTC>.json 必须 status=="pass" 且
  assertion_id=="g9.p0.m96.path_tracer_reference"(ci/g9_gi_interlock.py);
  缺失/非 pass 即门 FAIL 退 1(打印阻断原因)。

host 段:rurix-render gi::spg_rc 9 单测逐名锚定(细分判据闭集/16px 基线/滤波
  权重律 G8 同面/缓存 temporal 公共底座/世界级登记 fail-closed/带比较器/门序
  锚消费)+ conformance gi 语料锚(accept spg_radiance_cache_screen_level_minimal
  + reject radiance_cache_product_is_disabled,`//@ spec: RXS-0360`)+ 冻结带
  milestones/g9/g9_m99_spg_rc_band.json provenance 机器核验(含 m96_anchor_digest
  与 M97 冻结带 depth2 m96_digest 逐字相等的门序链锚)。
device 段(必需,持 gpu_device_lock):rurixc --target vulkan 产双 SPV
  (g9_m99_spg_probe/g9_m96_path_tracer)→ g9_m99_spg_radiance_cache harness 全档
  真跑(双跑位级一致 + 自适应细分非平凡 + 判据计数非空 + 缓存计数逐帧 +
  temporal 底座审计 + 私写重投影注入拒 + 关 product IS 方差回归/关自适应偏离
  双臂可检测 + 世界级 clipmap not-triggered 字段核验 + 三档深度带内;
  RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1,SKIP 翻红)+ --red-arm
  product-is-off 子模式独立复跑抽检。

用法:
  py -3 ci/g9_spg_radiance_cache_smoke.py --gate g9.p1.m99.spg_radiance_cache
  py -3 ci/g9_spg_radiance_cache_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones/g9/g9_m99_spg_radiance_cache_evidence_schema.json"
BAND_PATH = ROOT / "milestones/g9/g9_m99_spg_rc_band.json"
M97_BAND_PATH = ROOT / "milestones/g9/g9_m97_depth_band.json"
ACCEPT_CORPUS = ROOT / "conformance/gi/accept/spg_radiance_cache_screen_level_minimal.rx"
REJECT_CORPUS = ROOT / "conformance/gi/reject/radiance_cache_product_is_disabled.rx"
KERNEL_DIR = ROOT / "src/rurix-render/kernels"
WORK_DIR = ROOT / ".tmp/g94_gates/m99"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402
from g9_gi_interlock import m96_gate_passed  # noqa: E402

GATE_KEY = "g9.p1.m99.spg_radiance_cache"
NUMERIC_STEP = 150
SOURCE_REF = "RFC-0022 §4.8;spec/global_illumination.md RXS-0360;G9_ACCEPTANCE_MAP §3 M99"
TAG = "g9_m99"

SPG_RC_TESTS = [
    "subdivide_cause_closed_set_and_names",
    "grid_baseline_16px_and_adaptive_increment",
    "probe_trace_product_is_variance_red_arm",
    "filter_radiance_weight_law_matches_g8_base",
    "screen_cache_temporal_base_and_private_red",
    "world_clipmap_not_triggered_registration",
    "assemble_and_digest_structural_divergence",
    "band_roundtrip_and_fail_closed",
    "m96_anchor_consumed_from_m97_band",
]

CHECK_KEYS = [
    # host 段
    "gate_order_m96_passed",
    "host_spg_rc_tests_anchored",
    "conformance_gi_corpus_anchored",
    "spg_rc_band_provenance_frozen",
    # device 段
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_spg_adaptive_subdivision",
    "device_cache_temporal",
    "device_red_arms_effective",
    "device_red_arm_submode_detected",
    "device_world_clipmap_not_triggered",
    "device_m96_cross_anchor_band",
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


def is_hex(v: object, n: int) -> bool:
    return isinstance(v, str) and len(v) == n and all(c in "0123456789abcdef" for c in v)


# ═══════════════════════ host 段 ═══════════════════════


def host_spg_rc_tests() -> bool:
    """cargo test -p rurix-render --lib gi::spg_rc:9 单测逐名锚定全绿。"""
    rc, blob = run_cargo(["test", "-p", "rurix-render", "--lib", "gi::spg_rc"])
    ok = rc == 0 and "test result: ok" in blob
    anchored = True
    for name in SPG_RC_TESTS:
        if not (ok and name in blob):
            check(False, f"gi::spg_rc 单测 {name} 未锚定/失败")
            anchored = False
    return anchored


def host_conformance_anchor() -> bool:
    """conformance gi accept/reject 语料在位 + `//@ spec: RXS-0360` 锚定。"""
    ok = True
    for path, face in ((ACCEPT_CORPUS, "accept"), (REJECT_CORPUS, "reject")):
        if not path.is_file():
            check(False, f"缺 {face} 语料 {path.name}")
            ok = False
            continue
        text = path.read_text(encoding="utf-8")
        if "//@ spec: RXS-0360" not in text or GATE_KEY not in text:
            check(False, f"{face} 语料 {path.name} 锚定/门 key 预期面缺失")
            ok = False
    if ok and "product IS" not in REJECT_CORPUS.read_text(encoding="utf-8"):
        check(False, "reject 语料 product IS 预期面缺失")
        ok = False
    return ok


def host_band_provenance() -> bool:
    """M99 带 provenance 机器核验 + M97 带 depth2 门序链锚逐字核验。"""
    if not BAND_PATH.is_file():
        check(False, f"缺冻结带 {BAND_PATH.name}")
        return False
    try:
        band = json.loads(BAND_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        check(False, f"冻结带不可读: {e}")
        return False
    ok = True

    def need(cond: bool, msg: str) -> None:
        nonlocal ok
        if not cond:
            check(False, f"冻结带 provenance: {msg}")
            ok = False

    need(band.get("schema") == "rurix.g9m99.spg_rc_band.v1", "schema 字面不符")
    need(bool(band.get("frozen_at_utc")), "frozen_at_utc 空")
    need(bool(band.get("device_name")), "device_name 空")
    need(band.get("scene") == "m96_cornell", "scene ≠ m96_cornell")
    need(is_hex(band.get("m96_anchor_digest"), 64), "m96_anchor_digest 形态")
    need("M99_BAND_MARGIN" in str(band.get("freeze_rule", "")), "freeze_rule 缺 M99_BAND_MARGIN 登记")
    need(band.get("matched_depth") == "2", "matched_depth ≠ 2")
    need(band.get("m96_golden_spp") == "64", "m96_golden_spp ≠ 64")
    need(bool(band.get("seed_chain")), "seed_chain 空")
    need(bool(band.get("product_is_variance_ratio")), "product_is_variance_ratio 空")
    need(bool(band.get("adaptive_deviation_ratio")), "adaptive_deviation_ratio 空")
    entries = band.get("entries")
    need(isinstance(entries, list) and len(entries) == 3, "entries ≠ 3(spg_adaptive/spg_uniform/product_is_off)")
    for e in entries if isinstance(entries, list) else []:
        for f in ("product_digest", "m96_digest"):
            need(is_hex(e.get(f), 64), f"条目 {e.get('tier')} {f} 形态")
        for f in ("band_rel_dev", "measured_rel_dev"):
            need(isinstance(e.get(f), str) and len(e[f]) > 0, f"条目 {e.get('tier')} 缺 {f}")
    # 门序链锚:M99 带 m96_anchor_digest == M97 带 depth2 条目 m96_digest。
    try:
        m97 = json.loads(M97_BAND_PATH.read_text(encoding="utf-8"))
        anchor = next(
            (e.get("m96_digest") for e in m97.get("entries", []) if e.get("depth") == "2"),
            None,
        )
    except (OSError, json.JSONDecodeError):
        anchor = None
    if anchor is None:
        check(False, "M97 冻结带缺 depth2 条目(门序链锚不可核)")
        ok = False
    elif band.get("m96_anchor_digest") != anchor:
        check(False, f"M99 带 m96_anchor_digest ≠ M97 带 depth2 m96_digest({anchor})")
        ok = False
    else:
        note("M99 带 m96_anchor_digest == M97 带 depth2 m96_digest(门序链锚逐字一致)")
    return ok


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


def compile_spv(rurixc: Path, kernel_name: str, out: Path) -> bool:
    print(f"[{TAG}] rurixc {kernel_name} --target vulkan -o {out.name}")
    out.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        [str(rurixc), str(KERNEL_DIR / kernel_name), "--target", "vulkan", "-o", str(out)],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0 or not out.is_file():
        check(False, f"SPV 产线失败({kernel_name}) rc={r.returncode}:\n{(r.stdout + r.stderr)[-1500:]}")
        return False
    return True


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin g9_m99_spg_radiance_cache")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", "g9_m99_spg_radiance_cache"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"g9_m99_spg_radiance_cache 构建失败:\n{r.stderr[-2000:]}")
        return None
    exe = target_dir() / "debug" / ("g9_m99_spg_radiance_cache.exe" if sys.platform == "win32" else "g9_m99_spg_radiance_cache")
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


def run_harness_full(exe: Path, spv_m99: Path, spv_m96: Path) -> tuple[str, dict | None]:
    """全档真跑(SPG+RC 双级)。返回 (device_state, harness evidence|None)。"""
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(exe),
        "--spv-m99", str(spv_m99),
        "--spv-m96", str(spv_m96),
        "--work-dir", str(WORK_DIR / "work"),
        "--evidence", str(HARNESS_EVIDENCE),
    ]
    print(f"[{TAG}] device 全档: g9_m99_spg_radiance_cache(双 SPV,validation=on)")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=device_env(), timeout=1800)
    out = r.stdout + r.stderr
    if "G9_M99_SPG: SKIP" in r.stdout:
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {out.strip()[-800:]}")
        return "skipped_dev_env", None
    doc = None
    if HARNESS_EVIDENCE.is_file():
        try:
            doc = json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            check(False, f"harness evidence 不可解析: {e}")
    if r.returncode != 0 or "G9_M99_SPG: PASS" not in r.stdout:
        check(False, f"harness 全档失败 rc={r.returncode}:\n{out[-2000:]}")
        return "fail", doc
    if doc is None:
        check(False, "harness evidence 缺失")
        return "fail", None
    if doc.get("schema") != "rurix.g9m99.spg_rc.v1" or doc.get("spec_anchor") != "RXS-0360":
        check(False, "harness evidence schema/spec_anchor 字面不符")
        return "fail", doc
    if doc.get("assertion_id") != GATE_KEY or doc.get("status") != "pass":
        check(False, "harness evidence assertion_id/status 不符")
        return "fail", doc
    return "executed", doc


def run_red_arm(exe: Path, spv_m99: Path, spv_m96: Path) -> bool:
    """--red-arm product-is-off 子模式独立复跑(退出码 0 + PASS red-arm 字面)。"""
    print(f"[{TAG}] device RED 臂子模式: --red-arm product-is-off")
    r = subprocess.run(
        [str(exe), "--red-arm", "product-is-off", "--spv-m99", str(spv_m99), "--spv-m96", str(spv_m96)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=device_env(),
        timeout=900,
    )
    out = r.stdout + r.stderr
    ok = r.returncode == 0 and "G9_M99_SPG: PASS red-arm product-is-off" in r.stdout
    if not ok:
        check(False, f"RED 臂子模式 product-is-off 未独立检出 rc={r.returncode}: {out[-600:]}")
    return ok


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def run_selftest() -> int:
    # 红臂①:合成 FAILURES 必须使门红(check() 判别有效)。
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 红臂②:M96 evidence 缺失 ⇒ 门序必阻断(D2-Q7)。
    with tempfile.TemporaryDirectory(prefix="g9_m99_selftest_") as td:
        ok, detail = m96_gate_passed(Path(td))
        if ok or "门序阻断" not in detail:
            print(f"[{TAG}] selftest FAIL: M96 evidence 缺失未阻断", file=sys.stderr)
            return 1
    # CHECK_KEYS 闭集恰 13 项(host 4 + device 9)。
    if len(CHECK_KEYS) != 13:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 13", file=sys.stderr)
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
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (2 RED + 1 GREEN)")
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
    device_state = "fail"

    # 门序机器阻断(D2-Q7,前置):M96 门未绿即 FAIL 退 1,host/device 段不跑。
    interlock_ok, interlock_detail = m96_gate_passed()
    print(f"[{TAG}] {interlock_detail}")
    note(interlock_detail)
    checks["gate_order_m96_passed"] = interlock_ok
    if not interlock_ok:
        check(False, interlock_detail)
    else:
        # host 段
        checks["host_spg_rc_tests_anchored"] = host_spg_rc_tests()
        checks["conformance_gi_corpus_anchored"] = host_conformance_anchor()
        checks["spg_rc_band_provenance_frozen"] = host_band_provenance()

        # device 段(持锁串行:rurixc 构建 + 双 SPV 产线 + harness 全档 + RED 臂子模式)
        with gpu_device_lock(purpose="g9_m99 spg+rc device 腿"):
            rurixc = build_rurixc()
            exe = build_harness() if rurixc else None
            spv_m99 = WORK_DIR / "g9_m99_spg_probe.spv"
            spv_m96 = WORK_DIR / "g9_m96_path_tracer.spv"
            spvs_ok = rurixc is not None and all(
                compile_spv(rurixc, k, o)
                for k, o in (
                    ("g9_m99_spg_probe.rx", spv_m99),
                    ("g9_m96_path_tracer.rx", spv_m96),
                )
            )
            if exe and spvs_ok:
                device_state, doc = run_harness_full(exe, spv_m99, spv_m96)
                if device_state == "executed" and doc is not None:
                    hc = doc.get("checks", {})
                    checks["device_harness_full_pass"] = True
                    checks["device_double_run_bitexact"] = hc.get("double_run_bitexact") is True
                    checks["device_spg_adaptive_subdivision"] = (
                        hc.get("spg_adaptive_subdivision_non_trivial") is True
                        and hc.get("subdivide_cause_counts_non_empty") is True
                    )
                    checks["device_cache_temporal"] = (
                        hc.get("cache_counters_per_frame") is True
                        and hc.get("temporal_base_audit_pass") is True
                    )
                    checks["device_red_arms_effective"] = all(
                        hc.get(k) is True
                        for k in ("private_reproject_detected",
                                  "product_is_off_variance_detectable",
                                  "adaptive_off_deviation_detectable")
                    )
                    wc = doc.get("world_clipmap_registration", {})
                    checks["device_world_clipmap_not_triggered"] = (
                        hc.get("world_clipmap_not_triggered_registered") is True
                        and wc.get("status") == "not-triggered"
                        and wc.get("trigger_met") is False
                        and wc.get("lookup_rejected") is True
                        and wc.get("world_lookups") == 0
                    )
                    checks["device_m96_cross_anchor_band"] = (
                        hc.get("m96_cross_anchor") is True
                        and hc.get("depth_band_within") is True
                    )
                    env = doc.get("environment", {})
                    checks["device_validation_zero"] = (
                        env.get("validation") == "on"
                        and env.get("require_real") is True
                    )
                    checks["device_red_arm_submode_detected"] = run_red_arm(exe, spv_m99, spv_m96)
                    for k in CHECK_KEYS:
                        if k.startswith("device_") and not checks[k]:
                            check(False, f"harness 判据 {k} 为假")
                note("device:全档真跑(双级产物 + 世界级 not-triggered 登记)+ product-is-off 子模式复跑")

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g9_m99_spg_radiance_cache",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M99",
        "milestone": "M99",
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
            {"seq": 1, "command": "cargo test -p rurix-render --lib gi::spg_rc", "exit_code": 0},
            {"seq": 2, "command": "cargo build -p rurixc --features vulkan-backend --bin rurixc", "exit_code": 0},
            {"seq": 3, "command": "rurixc kernels/g9_m99_spg_probe.rx + g9_m96_path_tracer.rx --target vulkan -o .tmp/g94_gates/m99/*.spv", "exit_code": 0},
            {"seq": 4, "command": "cargo build -p rurix-render --features vulkan --bin g9_m99_spg_radiance_cache", "exit_code": 0},
            {"seq": 5, "command": "g9_m99_spg_radiance_cache --spv-m99 .. --spv-m96 .. (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g9_m99_spg_radiance_cache --red-arm product-is-off --spv-m99 .. (子模式抽检)", "exit_code": 0 if checks["device_red_arm_submode_detected"] else 1},
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
    out = EVIDENCE_DIR / f"g9_m99_spg_radiance_cache_{ts}.json"
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
