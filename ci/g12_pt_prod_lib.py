#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.2 生产化核心波续）
"""G12.2 生产化核心波共享判定层（ci/g12_*_smoke.py 五门复用）。

承载：rurixc/harness 构建 + SPV 产线、device env 双置、g12_budget 标定/锚
条目读取（M166 标定程序产，禁手写 P-09）、采样器选型 artifact 读取、
harness --gate 调用与直出件解析、M96 冻结面 0-byte 机核（band 文件 + M96
参照器既有面 git diff 闭集）、门 evidence 装配、schema/selftest 互核。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
KERNEL = ROOT / "src/rurix-render/kernels/g12_pt_production.rx"
BUDGET_PATH = ROOT / "milestones/g12/g12_budget.json"
SELECTION_PATH = ROOT / "milestones/g12/g12_pt_sampler_selection.json"
M96_BAND = ROOT / "milestones/g9/g9_m96_pbrt_tolerance_band.json"
M96_REFERENCE = ROOT / "src/rurix-render/src/gi/path_trace.rs"
PBRT_EXE = ROOT / "external/pbrt-v4/build/Release/pbrt.exe"
IMGTOOL_EXE = ROOT / "external/pbrt-v4/build/Release/imgtool.exe"
WORK_DIR = ROOT / ".tmp/g12_gates/pt_prod"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"
# G37 收官战役卫兵基线升级(day_0829 HANDOVER §G.4 兑现):path_trace.rs 自
# G12.0 ref 5ae83aa7 后经三次主线合法提交演进(526d4c4e G12.2 / 5388c30f G12.3 /
# 058f8e68 G31+ 合流),旧基线致卫兵必红;新不可变点 = 058f8e68(升级时点
# git diff 058f8e68 HEAD -- path_trace.rs + band 双 0-byte 机核在案),其后
# 纯追加 prod 守护语义不变。PT 功能面完好(selftest PASS + M158 device 14/15)。
G12_ZERO_BASE = "058f8e68"  # 不可变 ref 谱系:5ae83aa7(G12.0)→058f8e68(G31+ 合流后新基线)

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402,F401

TAG = "g12_pt_prod"

# 生产化 8 条参照器曲线基线锚 + 7 条 M166 标定条目的 budget id 闭集。
ANCHOR_IDS = [
    f"g12.pt.ref_curve_{scene}_spp{spp}"
    for scene in ("cornell", "direct")
    for spp in (1, 4, 16, 64)
]
CALIB_IDS = [
    "g12.pt.rr_tau",
    "g12.pt.adaptive_rel_err_theta",
    "g12.pt.misjudge_rate_tol",
    "g12.pt.curve_tol_rel",
    "g12.pt.furnace_energy_tol",
    "g12.pt.level_monotone_tol",
    "g12.pt.rr_unbiased_tol",
]


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def build_rurixc() -> Path | None:
    print(f"[{TAG}] cargo build -p rurixc --features vulkan-backend --bin rurixc")
    r = run(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"])
    if r.returncode != 0:
        return None
    exe = target_dir() / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    return exe if exe.is_file() else None


def compile_spv(rurixc: Path, out: Path) -> bool:
    print(f"[{TAG}] rurixc {KERNEL.name} --target vulkan -o {out.name}")
    out.parent.mkdir(parents=True, exist_ok=True)
    r = run([str(rurixc), str(KERNEL), "--target", "vulkan", "-o", str(out)])
    return r.returncode == 0 and out.is_file()


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin g12_pt_production")
    r = run(["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", "g12_pt_production"])
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return None
    exe = target_dir() / "debug" / ("g12_pt_production.exe" if sys.platform == "win32" else "g12_pt_production")
    return exe if exe.is_file() else None


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    env["RURIX_BASE_COMMIT"] = run(["git", "rev-parse", "HEAD"]).stdout.strip()
    return env


def base_commit() -> str:
    return run(["git", "rev-parse", "HEAD"]).stdout.strip()


# ---------------------------------------------------------------------------
# g12_budget 读取（标定值/容差/锚;M166 标定程序产,禁手写 P-09）
# ---------------------------------------------------------------------------


def load_budget() -> dict:
    return json.loads(BUDGET_PATH.read_text(encoding="utf-8"))


def budget_entry(budget: dict, eid: str) -> dict | None:
    for e in budget.get("entries", []):
        if e.get("id") == eid:
            return e
    return None


def load_calibration() -> dict | None:
    """读取生产化门消费面:τ/θ(winner measured_value)+ 容差(threshold)+
    8 锚(measured_value)+ 选型 winner。任一条目缺失/非 measured_local →
    None(门 fail-closed)。"""
    if not SELECTION_PATH.is_file() or not BUDGET_PATH.is_file():
        return None
    budget = load_budget()
    out: dict = {"anchors": {}}
    for eid in ANCHOR_IDS + CALIB_IDS:
        e = budget_entry(budget, eid)
        if e is None or e.get("evidence") != "measured_local":
            return None
        out[eid] = e
    for eid in ANCHOR_IDS:
        out["anchors"][eid] = float(budget_entry(budget, eid)["measured_value"])
    sel = json.loads(SELECTION_PATH.read_text(encoding="utf-8"))
    if sel.get("schema") != "rurix.g12pt.sampler_selection.v1" or sel.get("winner") not in (
        "pcg_independent",
        "stratified_per_dimension",
        "sobol_class_seed_perturbed",
    ):
        return None
    out["winner"] = sel["winner"]
    out["tau"] = float(out["g12.pt.rr_tau"]["measured_value"])
    out["theta"] = float(out["g12.pt.adaptive_rel_err_theta"]["measured_value"])
    for eid in CALIB_IDS[2:]:
        out[eid] = float(budget_entry(budget, eid)["threshold"])
    return out


def winner_cli_name(winner: str) -> str:
    return {
        "pcg_independent": "pcg",
        "stratified_per_dimension": "stratified",
        "sobol_class_seed_perturbed": "sobol",
    }[winner]


def harness_args(cal: dict, gate: str, spv: Path) -> list[str]:
    """harness 命令行(阈值全部自 g12_budget 标定条目读出,零手写)。"""
    a = cal["anchors"]
    args = [
        "--gate", gate,
        "--spv", str(spv),
        "--evidence", str(HARNESS_EVIDENCE),
        "--pbrt", str(PBRT_EXE),
        "--imgtool", str(IMGTOOL_EXE),
        "--work-dir", str(WORK_DIR / "pbrt_work"),
        "--tau", repr(cal["tau"]),
        "--theta", repr(cal["theta"]),
        "--sampler", winner_cli_name(cal["winner"]),
        "--curve-tol", repr(cal["g12.pt.curve_tol_rel"]),
        "--furnace-tol", repr(cal["g12.pt.furnace_energy_tol"]),
        "--level-tol", repr(cal["g12.pt.level_monotone_tol"]),
        "--rr-unbiased-tol", repr(cal["g12.pt.rr_unbiased_tol"]),
        "--misjudge-tol", repr(cal["g12.pt.misjudge_rate_tol"]),
        "--anchor-cornell",
        ",".join(repr(a[f"g12.pt.ref_curve_cornell_spp{s}"]) for s in (1, 4, 16, 64)),
        "--anchor-direct",
        ",".join(repr(a[f"g12.pt.ref_curve_direct_spp{s}"]) for s in (1, 4, 16, 64)),
    ]
    return args


# ---------------------------------------------------------------------------
# M96 冻结面 0-byte 机核(正确性锚:band 文件 + 参照器既有面 diff 闭集)
# ---------------------------------------------------------------------------


def m96_frozen_surface_unchanged() -> tuple[bool, str]:
    """vs 不可变基线 ref(G12_ZERO_BASE,现 = 058f8e68):m96 容差带/参照器既有面 0-byte。

    path_trace.rs 唯一合法差分 = 子模块注册块(纯追加行,全部含
    `prod` 字面;既有行零删除零改写)。基线谱系见 G12_ZERO_BASE 注释。"""
    r = run(["git", "diff", "--name-only", G12_ZERO_BASE, "--",
             "milestones/g9/g9_m96_pbrt_tolerance_band.json",
             "src/rurix-render/src/gi/path_trace.rs"])
    changed = [x.strip() for x in r.stdout.splitlines() if x.strip()]
    band_dirty = any("g9_m96_pbrt_tolerance_band" in c for c in changed)
    if band_dirty:
        return False, "g9_m96_pbrt_tolerance_band.json 有差分(冻结带须 0-byte)"
    if any("path_trace.rs" in c for c in changed):
        d = run(["git", "diff", G12_ZERO_BASE, "--", "src/rurix-render/src/gi/path_trace.rs"])
        for line in d.stdout.splitlines():
            if line.startswith("-") and not line.startswith("---"):
                return False, f"path_trace.rs 有删除行(参照器 0-byte 违例): {line[:80]}"
            if line.startswith("+") and not line.startswith("+++"):
                body = line[1:].strip()
                if body and "prod" not in body and not body.startswith("//"):
                    return False, f"path_trace.rs 追加行越 prod 模块注册面: {body[:80]}"
    return True, "band 0-byte + path_trace.rs 差分 ⊆ prod 模块注册块(纯追加)"


# ---------------------------------------------------------------------------
# 门 evidence 装配
# ---------------------------------------------------------------------------


def gate_evidence(
    *,
    subject: str,
    gate_key: str,
    milestone: str,
    wave: str,
    numeric_step: int,
    source_ref: str,
    checks: dict[str, bool],
    device_state: str,
    host_pass: bool,
    commands: list[dict],
    environment: dict,
    production: dict | None,
    notes: str,
    all_pass: bool,
    ts: str,
) -> dict:
    ev = {
        "schema_version": 1,
        "subject": subject,
        "symbolic_gate_key": gate_key,
        "matrix_row": milestone,
        "milestone": milestone,
        "assertion_id": gate_key,
        "status": "pass" if all_pass else "fail",
        "wave": wave,
        "numeric_step": numeric_step,
        "source_ref": source_ref,
        "base_commit": base_commit(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": commands,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": environment,
        "notes": notes,
    }
    if production is not None:
        ev["production"] = production
    return ev


def tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def environment() -> dict:
    import platform

    return {
        "os": platform.platform(),
        "python_version": sys.version.split()[0],
        "cargo_version": tool_version("cargo"),
        "rustc_version": tool_version("rustc"),
    }


# ---------------------------------------------------------------------------
# 四门公共驱动(host 锚定 + device 腿持锁真跑 + RED 臂子模式复跑)
# ---------------------------------------------------------------------------

PROD_TEST_MODULE = "gi::path_trace::prod"


def host_prod_tests(test_names: list[str], tag: str) -> tuple[bool, str]:
    """cargo test -p rurix-render --lib gi::path_trace::prod 逐名锚定全绿。"""
    r = run(["cargo", "test", "-p", "rurix-render", "--lib", PROD_TEST_MODULE])
    blob = r.stdout + r.stderr
    ok = r.returncode == 0 and "test result: ok" in blob
    missing = [n for n in test_names if n not in blob]
    if not ok or missing:
        return False, f"prod 单测失败或未锚定: {missing[:3]} rc={r.returncode}"
    return True, f"{len(test_names)} 单测逐名锚定全绿"


def conformance_anchor(files: list[tuple[str, str]], gate_key: str) -> tuple[bool, str]:
    """conformance/gi 语料在位 + `//@ spec: <clause>` 锚 + 门 key 预期面。"""
    for rel, clause in files:
        path = ROOT / "conformance/gi" / rel
        if not path.is_file():
            return False, f"缺语料 {rel}"
        text = path.read_text(encoding="utf-8")
        if f"//@ spec: {clause}" not in text or gate_key not in text:
            return False, f"{rel} 缺 {clause} 锚或门 key 预期面"
    return True, f"{len(files)} 件语料锚定在位"


def run_device_leg(
    gate_key: str,
    cal: dict,
    red_arms: list[str],
    tag: str,
) -> tuple[str, dict | None, bool, list[str]]:
    """device 腿(持锁):rurixc 构建 + SPV 产线 + harness --gate 全档真跑 +
    RED 臂 --red-arm 子模式独立复跑抽检。返回 (device_state, harness 直出件,
    submode_ok, failures)。"""
    failures: list[str] = []
    submode_ok = True
    doc: dict | None = None
    with gpu_device_lock(purpose=f"{tag} device 腿"):
        rurixc = build_rurixc()
        spv = WORK_DIR / "g12_pt_production.spv"
        harness = build_harness() if rurixc else None
        if not (rurixc and harness and compile_spv(rurixc, spv)):
            failures.append("rurixc/SPV/harness 产线失败")
            return "fail", None, False, failures
        args = harness_args(cal, gate_key, spv)
        cmd = [str(harness)] + args
        print(f"[{tag}] device 全档: harness --gate {gate_key}(validation=on)")
        r = run(cmd, env=device_env(), timeout=3600)
        out = r.stdout + r.stderr
        if "G12_PT_PROD: SKIP" in r.stdout:
            return "skipped_dev_env", None, False, [f"device SKIP: {out.strip()[-400:]}"]
        if HARNESS_EVIDENCE.is_file():
            try:
                doc = json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
            except json.JSONDecodeError as e:
                failures.append(f"harness evidence 不可解析: {e}")
        if r.returncode != 0 or "G12_PT_PROD: PASS" not in r.stdout:
            failures.append(f"harness 全档失败 rc={r.returncode}: {out[-1500:]}")
            return "fail", doc, False, failures
        if doc is None:
            failures.append("harness evidence 缺失")
            return "fail", None, False, failures
        if doc.get("schema") != "rurix.g12pt.production.v1" or doc.get("gate") != gate_key:
            failures.append("harness evidence schema/gate 字面不符")
            return "fail", doc, False, failures
        # RED 臂子模式独立复跑抽检(退出码 0 + PASS red-arm 字面 = 臂独立有效)。
        for arm in red_arms:
            print(f"[{tag}] device RED 臂子模式: --red-arm {arm}")
            ra = run(
                [
                    str(harness), "--red-arm", arm, "--spv", str(spv),
                    "--tau", repr(cal["tau"]), "--sampler", winner_cli_name(cal["winner"]),
                ],
                env=device_env(),
                timeout=900,
            )
            rout = ra.stdout + ra.stderr
            if ra.returncode != 0 or f"G12_PT_PROD: PASS red-arm {arm}" not in ra.stdout:
                failures.append(f"RED 臂子模式 {arm} 未独立检出 rc={ra.returncode}: {rout[-400:]}")
                submode_ok = False
    return "executed", doc, submode_ok, failures


def production_section(doc: dict | None, anchor_ok: bool) -> dict:
    """CI_GATES §7 生产化节字段闭集(materialize 硬化)。"""
    hp = (doc or {}).get("production", {})
    curves = (doc or {}).get("curves", {})
    worst = None
    all_pass = True
    for _k, c in curves.items():
        ratio = float(c["curve"]) / float(c["anchor"])
        worst = ratio if worst is None else max(worst, ratio)
        all_pass = all_pass and bool(c["pass"])
    return {
        "correctness_anchor_unchanged": anchor_ok,
        "baseline_anchor_id": "g12.pt.ref_curve_{cornell,direct}_spp{1,4,16,64}",
        "measured_value": "max(curve/anchor) = {:.6e}".format(worst) if worst is not None else "n/a(本门无曲线面)",
        "not_worse_than_anchor": all_pass,
        "threshold_provenance": hp.get("threshold_provenance", "") + ";阈值经 g12_budget 标定条目传入(M166)",
        "evolution_register": hp.get("evolution_register"),
    }

