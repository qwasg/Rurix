#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 B Task B2 ReSTIR 高档 reservoir 车道集成）
"""G31+ 波 B Task B2：ReSTIR 高档 reservoir 车道集成门冒烟
（g31.waveB.restir；G30 收官承接锚 M100-high 行 g31_anchor「M100 车道集成窗」
第三件兑现面；RFC-0038 out-of-scope 锚车道集成项）。

硬判据（任务书逐字）：
1. 车道集成：kernels/g28_restir.rx（本体 0-byte）接进 direct GI/多灯采样链——
   生产管线 direct GI pass 现有灯光表（bistro-interior 契约 lighting JSON
   point_lights）喂入 reservoir 采样高档车道；--restir <off|on> 开关，默认档
   维持 MegaLights/现有多灯路径 0-byte（off = RIS M=1 ≡ estimate_uniform
   代数恒等语义镜像，同一 kernel 同一灯表真跑）。
2. 门维持：G28 harness 全档验证接线态复跑全绿——y 整数锚 20000/20000 +
   无偏 3σ + host 参考臂对拍 p100 ≤ 冻结容差（g28_budget 程序产条目
   g28.restir_device.host_device_estimate_tol，measured×2.0 冻结 k 口径，
   本脚本读档传 --tol 不手写）。
3. measured 对照：bistro 多灯场景 ReSTIR on/off 画质（逐 trial 对拍 p100 +
   双臂无偏 3σ + 方差 var(off)/var(on) >1 方向硬门数值如实登记）+ dispatch
   墙钟 ms 对照进 evidence（数字来自真实命令输出）。
4. 确定性：固定 seed ReSTIR on 双跑 digest 位级一致；off 面静态锚零漂移
   （milestones/g31/g31_restir_wiring_off_anchor.json 首跑程序产追加，复跑
   位级期望，漂移即 RED）。
5. 空间重用加性臂：8×8 网格 × bistro 灯表，受点重评快照变换后直调冻结
   merge（禁第二实现），聚合 3σ + 逐点 5σ + 双跑位级。
6. 冻结面机核：gi/restir_reservoir.rs + gi/multi_light.rs vs g27-closed
   0-byte（提交面 + 工作树双面）+ kernels/g28_restir.rx vs g28-closed
   0-byte 同律。

三态：无 Vulkan loader/设备 → DEV_ENV_DEGRADE 退 0（不冒充 PASS）；真跑臂
RURIX_REQUIRE_REAL=1（该态下 SKIP → 硬红如实 FAIL，禁 mock 充真跑——
g31_dynamic_scene_smoke 同语义）。

用法：
  py -3 ci/g31_restir_wiring_smoke.py --selftest
  py -3 ci/g31_restir_wiring_smoke.py --gate g31.waveB.restir [--out <evidence.json>]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveB.restir"
TAG = "g31_restir_wire"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_restir_wiring_evidence_schema.json"
SCHEMA_ID = "rurix.g31.restir_wiring_evidence.v1"
ANCHOR_PATH = ROOT / "milestones" / "g31" / "g31_restir_wiring_off_anchor.json"
G28_BUDGET = ROOT / "milestones" / "g28" / "g28_budget.json"
G28_TOL_ENTRY = "g28.restir_device.host_device_estimate_tol"
KERNEL = ROOT / "src" / "rurix-render" / "kernels" / "g28_restir.rx"
WORK = ROOT / ".tmp" / "g31_gates" / "restir"
SPV_PATH = WORK / "g28_restir.spv"
HARNESS_G28 = "g28_restir_device"
HARNESS_WIRE = "g31_restir_wiring"
SCENE = "bistro-interior"
N_TRIALS = 20000
TIMING_RUNS = 20
# 车道夹具冻结 seed（bin 侧 SEED 字面同源；独立 G28/M100 流）。
WIRE_SEED = 0xB261_0007_2026_0825
FROZEN_GI_BASE = "g27-closed"
FROZEN_GI_FILES = [
    "src/rurix-render/src/gi/restir_reservoir.rs",
    "src/rurix-render/src/gi/multi_light.rs",
]
FROZEN_KERNEL_BASE = "g28-closed"
FROZEN_KERNEL_FILES = ["src/rurix-render/kernels/g28_restir.rx"]

FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def device_env(require_real: bool) -> dict[str, str]:
    env = dict(os.environ)
    if require_real:
        env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def json_line(stdout: str, schema_token: str) -> str | None:
    for line in stdout.splitlines():
        if schema_token in line:
            return line.strip()
    return None


# ---------------------------------------------------------------- 纯函数判据面
def budget_frozen_tol(budget: dict | None) -> float | None:
    """g28_budget 冻结容差判读器：条目在档 + measured_local + 无 skip_reason +
    threshold 正有限 ⇒ 返回 threshold（禁手写口径）；否则 None。"""
    if budget is None:
        return None
    for e in budget.get("entries", []):
        if e.get("id") == G28_TOL_ENTRY:
            if e.get("evidence") != "measured_local" or e.get("skip_reason"):
                return None
            tol = e.get("threshold")
            if isinstance(tol, (int, float)) and math.isfinite(float(tol)) and float(tol) > 0.0:
                return float(tol)
            return None
    return None


DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")


def digest_fmt(s: object) -> bool:
    return isinstance(s, str) and DIGEST_RE.match(s) is not None


def arm_green(arm: dict, tol: float) -> list[str]:
    """车道单臂判据闭集（off/on 同形）：y 整数锚 + 消费计数锚 + p100 ≤ tol +
    无偏 3σ + 双跑位级 + digest 形态 + problems 空。"""
    fails: list[str] = []
    tier = arm.get("tier", "?")
    if arm.get("y_anchor_all_equal") is not True:
        fails.append(f"{tier}: y 整数锚非全等")
    if arm.get("dec_consumed_all_equal") is not True:
        fails.append(f"{tier}: 判定带消费计数锚非全等")
    p100 = float(arm.get("p100_vs_host", "nan"))
    if not (math.isfinite(p100) and p100 <= tol):
        fails.append(f"{tier}: p100 {p100:.6e} 超冻结容差 {tol:.6e}")
    if arm.get("in_tol") is not True:
        fails.append(f"{tier}: in_tol ≠ true")
    if arm.get("unbiased_3sigma_pass") is not True:
        fails.append(f"{tier}: 无偏 3σ 失败")
    if arm.get("double_run_bitexact") is not True:
        fails.append(f"{tier}: 双跑非位级一致")
    if not digest_fmt(arm.get("digest")):
        fails.append(f"{tier}: digest 形态非法")
    if arm.get("problems"):
        fails.append(f"{tier}: problems 非空 {arm.get('problems')}")
    return fails


def variance_gate(off: dict, on: dict) -> tuple[bool, float]:
    """方差对照（方向硬门）：var(off)/var(on) > 1 且两臂方差正有限；
    比值 measured 如实登记（数值不设伪造通过线）。"""
    v_off = float(off.get("variance", "nan"))
    v_on = float(on.get("variance", "nan"))
    if not (math.isfinite(v_off) and math.isfinite(v_on) and v_off > 0.0 and v_on > 0.0):
        return False, float("nan")
    reduction = v_off / v_on
    return reduction > 1.0, reduction


def frame_ms_registered(arm: dict) -> bool:
    """dispatch 墙钟 measured 登记判读：mean/min/max 正有限 + runs ≥ 1。"""
    ms = arm.get("dispatch_ms") or {}
    mean = float(ms.get("mean", "nan"))
    mn = float(ms.get("min", "nan"))
    mx = float(ms.get("max", "nan"))
    runs = int(ms.get("runs", 0))
    return (
        math.isfinite(mean)
        and math.isfinite(mn)
        and math.isfinite(mx)
        and mean > 0.0
        and mn > 0.0
        and mx >= mn
        and runs >= 1
    )


def spatial_green(sp: dict) -> list[str]:
    """空间重用加性臂判据：聚合 3σ + 逐点 5σ 结构兜底 + 双跑位级。"""
    fails: list[str] = []
    agg = sp.get("aggregate_3sigma") or {}
    if agg.get("pass") is not True:
        fails.append("空间臂聚合 3σ 失败")
    if sp.get("per_point_5sigma_all_within") is not True:
        fails.append("空间臂逐点 5σ 结构兜底失败")
    if sp.get("double_run_bitexact") is not True:
        fails.append("空间臂双跑非位级一致")
    return fails


def anchor_zero_drift(anchor: dict, off_digest: str, table_digest: str) -> list[str]:
    """off 面静态锚零漂移判读（锚在档后的复跑面）：digest/灯表/seed/窗长
    逐字段位级期望。"""
    fails: list[str] = []
    if anchor.get("off_digest") != off_digest:
        fails.append(f"off digest 漂移：{anchor.get('off_digest')} ≠ {off_digest}")
    if anchor.get("light_table_digest") != table_digest:
        fails.append("灯表 digest 漂移（契约灯表变更即锚重订程序，禁静默）")
    if int(anchor.get("seed", -1)) != WIRE_SEED:
        fails.append("锚 seed 漂移")
    if int(anchor.get("n_trials", -1)) != N_TRIALS:
        fails.append("锚 n_trials 漂移")
    return fails


def evidence_required_keys(doc: dict) -> list[str]:
    """schema required 闭集核验（jsonschema 依赖免；check_schemas.py 另作
    形式校验面）。"""
    required = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))["required"]
    return [k for k in required if k not in doc]


# ---------------------------------------------------------------- 真跑驱动
def build_bin(name: str) -> Path | None:
    note(f"cargo build -p rurix-render --features vulkan --bin {name}")
    r = run(["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", name],
            timeout=7200)
    if r.returncode != 0:
        print((r.stderr or "")[-2000:])
        return None
    exe = target_dir() / "debug" / (f"{name}.exe" if sys.platform == "win32" else name)
    return exe if exe.is_file() else None


def ensure_rurixc() -> Path | None:
    exe = target_dir() / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if exe.is_file():
        return exe
    note("cargo build -p rurixc --features vulkan-backend --bin rurixc")
    r = run(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"],
            timeout=7200)
    if r.returncode != 0:
        print((r.stderr or "")[-2000:])
        return None
    return exe if exe.is_file() else None


def compile_spv() -> tuple[bool, str]:
    rurixc = ensure_rurixc()
    if rurixc is None:
        return False, "rurixc 构建失败"
    WORK.mkdir(parents=True, exist_ok=True)
    note(f"rurixc {KERNEL.name} --target vulkan -o {SPV_PATH.relative_to(ROOT)}")
    r = run([str(rurixc), str(KERNEL), "--target", "vulkan", "-o", str(SPV_PATH)], timeout=1800)
    if r.returncode != 0 or not SPV_PATH.is_file():
        return False, f"SPV 编译失败 rc={r.returncode}: {((r.stdout or '') + (r.stderr or ''))[-300:]}"
    # spirv-val 独立校验（rurixc 内建校验之外的第二判读面；缺工具即 RED——
    # G28 门同律，不 SKIP）。
    val = run(["spirv-val", str(SPV_PATH)], timeout=600)
    if val.returncode != 0:
        return False, f"spirv-val 未过: {((val.stdout or '') + (val.stderr or ''))[-300:]}"
    return True, "rurixc --target vulkan 产 SPV + spirv-val 独立校验通过"


def run_compare(harness: Path, trials: int, timing_runs: int, tol: float,
                require_real: bool, out: Path | None) -> dict:
    """单轮 compare 腿真跑；返回 {rc, doc, stdout_tail, skipped}。"""
    cmd = [str(harness), "--compare", "--spv", str(SPV_PATH), "--tol", repr(tol),
           "--trials", str(trials), "--timing-runs", str(timing_runs)]
    if out is not None:
        cmd += ["--out", str(out)]
    r = run(cmd, env=device_env(require_real), timeout=7200)
    text = (r.stdout or "") + (r.stderr or "")
    line = json_line(r.stdout or "", "rurix.g31restir.compare.v1")
    doc = {}
    if line is not None:
        try:
            doc = json.loads(line)
        except json.JSONDecodeError:
            doc = {}
    return {
        "rc": r.returncode,
        "doc": doc,
        "tail": text[-400:],
        "skipped": doc.get("state") == "skipped_dev_env" or "skipped_dev_env" in text,
    }


def run_g28_maintenance(harness: Path, tol: float) -> dict:
    """G28 门维持腿：harness 全档验证（y 整数锚 + p100 + 3σ + 双跑位级）。"""
    r = run([str(harness), "--spv", str(SPV_PATH), "--tol", repr(tol)],
            env=device_env(True), timeout=7200)
    line = json_line(r.stdout or "", "rurix.g28restir.harness.v1")
    doc = {}
    if line is not None:
        try:
            doc = json.loads(line)
        except json.JSONDecodeError:
            doc = {}
    return {"rc": r.returncode, "doc": doc, "tail": ((r.stdout or "") + (r.stderr or ""))[-400:]}


def frozen_0byte(base: str, files: list[str]) -> tuple[bool, str]:
    r = run(["git", "diff", "--quiet", base, "--", *files])
    if r.returncode != 0:
        d = run(["git", "diff", "--name-only", base, "--", *files])
        changed = [x.strip() for x in d.stdout.splitlines() if x.strip()]
        return False, f"冻结面有差分 vs {base}（触碰即 RED）: {changed[:3]}"
    u = run(["git", "status", "--porcelain", "--", *files])
    if u.stdout.strip():
        dirty = [x for x in u.stdout.splitlines() if x.strip()]
        return False, f"冻结面工作树未提交面: {dirty[:3]}"
    return True, f"git diff --quiet {base} 提交面 + 工作树双面 0-byte"


# ---------------------------------------------------------------- selftest
def selftest() -> int:
    note("selftest：判据纯函数红绿臂")
    ok = True

    def expect(cond: bool, name: str) -> None:
        nonlocal ok
        if cond:
            note(f"  ok   — {name}")
        else:
            print(f"[{TAG}] MISS — {name}", file=sys.stderr)
            ok = False

    # ① budget_frozen_tol 绿臂：在档 measured_local 条目 → threshold。
    tol = budget_frozen_tol({"entries": [{
        "id": G28_TOL_ENTRY, "evidence": "measured_local", "skip_reason": None,
        "threshold": 5.66244125366211e-06}]})
    expect(tol == 5.66244125366211e-06, "budget 冻结容差绿臂（在档 measured → threshold）")
    # 红臂 ×4：缺条目 / estimated 冒充 / skip_reason 携带 / 非正 threshold。
    expect(budget_frozen_tol({"entries": []}) is None, "RED: 缺条目必拒")
    expect(budget_frozen_tol({"entries": [{
        "id": G28_TOL_ENTRY, "evidence": "estimated", "skip_reason": None,
        "threshold": 1e-6}]}) is None, "RED: estimated 冒充 measured 必拒")
    expect(budget_frozen_tol({"entries": [{
        "id": G28_TOL_ENTRY, "evidence": "measured_local", "skip_reason": "no gpu",
        "threshold": 1e-6}]}) is None, "RED: skip_reason 携带必拒")
    expect(budget_frozen_tol({"entries": [{
        "id": G28_TOL_ENTRY, "evidence": "measured_local", "skip_reason": None,
        "threshold": 0.0}]}) is None, "RED: 非正 threshold 必拒")

    # ② arm_green 绿臂：合成全绿臂零失败。
    good_arm = {
        "tier": "on", "m_candidates": 16, "y_anchor_all_equal": True,
        "dec_consumed_all_equal": True, "p100_vs_host": 1.75e-9, "in_tol": True,
        "unbiased_3sigma_pass": True, "double_run_bitexact": True,
        "digest": "sha256:" + "a" * 64, "problems": [],
    }
    expect(arm_green(good_arm, 5.66e-6) == [], "arm_green 绿臂（全绿臂零失败）")
    # 红臂 ×5：y 锚破 / p100 超容差 / 3σ 破 / 双跑破 / problems 非空。
    for name, mut in [
        ("y 锚破", {"y_anchor_all_equal": False}),
        ("p100 超容差", {"p100_vs_host": 1e-4}),
        ("3σ 破", {"unbiased_3sigma_pass": False}),
        ("双跑破", {"double_run_bitexact": False}),
        ("problems 非空", {"problems": ["x"]}),
    ]:
        bad = dict(good_arm)
        bad.update(mut)
        expect(arm_green(bad, 5.66e-6) != [], f"RED: {name} 必拒")

    # ③ variance_gate 绿/红臂。
    green, red_v = variance_gate({"variance": 1.8e-5}, {"variance": 1.1e-6})
    expect(green and abs(red_v - 1.8e-5 / 1.1e-6) < 1e-3, "variance_gate 绿臂（>1 方向 + 比值口径）")
    red, _ = variance_gate({"variance": 1.1e-6}, {"variance": 1.8e-5})
    expect(not red, "RED: 方向反转（reduction<1）必拒")
    red, _ = variance_gate({"variance": 1.0}, {"variance": 1.0})
    expect(not red, "RED: 等方差（reduction=1）必拒")
    red, _ = variance_gate({"variance": 0.0}, {"variance": 1.0})
    expect(not red, "RED: 零方差非正有限必拒")

    # ④ frame_ms_registered 绿/红臂。
    expect(frame_ms_registered({"dispatch_ms": {"mean": 1.5, "min": 1.2, "max": 2.0, "runs": 20}}),
           "frame_ms 绿臂（正有限 + runs≥1）")
    expect(not frame_ms_registered({"dispatch_ms": {"mean": 0.0, "min": 0.0, "max": 0.0, "runs": 20}}),
           "RED: 零值墙钟必拒")
    expect(not frame_ms_registered({"dispatch_ms": {"mean": float("nan"), "min": 1.0, "max": 2.0, "runs": 20}}),
           "RED: NaN 墙钟必拒")
    expect(not frame_ms_registered({}), "RED: 缺 dispatch_ms 必拒")

    # ⑤ spatial_green 绿/红臂。
    good_sp = {"aggregate_3sigma": {"pass": True}, "per_point_5sigma_all_within": True,
               "double_run_bitexact": True}
    expect(spatial_green(good_sp) == [], "spatial_green 绿臂")
    expect(spatial_green({"aggregate_3sigma": {"pass": False},
                          "per_point_5sigma_all_within": True,
                          "double_run_bitexact": True}) != [], "RED: 聚合 3σ 破必拒")
    expect(spatial_green({"aggregate_3sigma": {"pass": True},
                          "per_point_5sigma_all_within": False,
                          "double_run_bitexact": True}) != [], "RED: 逐点 5σ 破必拒")

    # ⑥ anchor_zero_drift 绿/红臂。
    anchor = {"off_digest": "sha256:" + "b" * 64, "light_table_digest": "sha256:" + "c" * 64,
              "seed": WIRE_SEED, "n_trials": N_TRIALS}
    expect(anchor_zero_drift(anchor, "sha256:" + "b" * 64, "sha256:" + "c" * 64) == [],
           "anchor 零漂移绿臂")
    expect(anchor_zero_drift(anchor, "sha256:" + "d" * 64, "sha256:" + "c" * 64) != [],
           "RED: off digest 漂移必拒")
    expect(anchor_zero_drift(anchor, "sha256:" + "b" * 64, "sha256:" + "d" * 64) != [],
           "RED: 灯表 digest 漂移必拒")

    # ⑦ digest_fmt / 三态判读。
    expect(digest_fmt("sha256:" + "0" * 64), "digest_fmt 绿臂")
    expect(not digest_fmt("sha256:xyz"), "RED: digest 形态非法必拒")

    # ⑧ evidence required 键闭集：合成 doc 缺键必列出。
    missing = evidence_required_keys({"schema": SCHEMA_ID})
    expect(len(missing) > 0 and "arms" in missing and "gate_maintenance" in missing,
           f"evidence required 键红臂（缺键列出 {len(missing)} 项）")

    if ok:
        note("SELFTEST PASS（红绿臂全如预期）")
        return 0
    print(f"[{TAG}] SELFTEST FAIL", file=sys.stderr)
    return 1


# ---------------------------------------------------------------- gate
def gate(out_path: Path | None) -> int:
    if not SCHEMA_PATH.is_file():
        fail(f"schema 缺失: {SCHEMA_PATH}")
        return 1
    budget = json.loads(G28_BUDGET.read_text(encoding="utf-8")) if G28_BUDGET.is_file() else None
    tol = budget_frozen_tol(budget)
    if tol is None:
        fail(f"g28_budget 冻结容差条目 {G28_TOL_ENTRY} 缺失/非 measured_local（禁手写容差）")
        return 1
    note(f"gate {GATE_KEY}: scene={SCENE} trials={N_TRIALS} tol={tol:.6e}（g28_budget 程序产条目转引）")

    with gpu_device_lock(purpose=f"{TAG} 构建+SPV+G28 门维持+双臂真跑"):
        g28 = build_bin(HARNESS_G28)
        if g28 is None:
            fail("g28_restir_device 构建失败")
            return 1
        wire = build_bin(HARNESS_WIRE)
        if wire is None:
            fail("g31_restir_wiring 构建失败")
            return 1
        ok, detail = compile_spv()
        if not ok:
            fail(detail)
            return 1
        note(detail)

        # ── dev-env 探针（不挂 REQUIRE_REAL：缺真实面 → bin 自报
        # skipped_dev_env 退 0；小窗快进——探针只裁 dev-env 不裁判据）──
        probe = run_compare(wire, 64, 1, tol, require_real=False, out=None)
        if probe["skipped"]:
            print(json.dumps({
                "schema": "rurix.g31.restir_wiring.skip.v1",
                "state": "DEV_ENV_DEGRADE",
                "what": "vulkan_loader_or_device",
                "reason": probe["tail"][-200:],
            }, ensure_ascii=False))
            note("DEV_ENV_DEGRADE（无 Vulkan loader/设备——退 0 不冒充 PASS）")
            return 0
        note("dev-env 探针绿（真机真跑面成立）")

        # ── 判据②：G28 门维持腿（fixture 20000 trial 接线态复跑）──
        gm = run_g28_maintenance(g28, tol)
        gmd = gm["doc"]
        if gmd.get("state") == "skipped_dev_env":
            fail("G28 门维持腿 SKIP（RURIX_REQUIRE_REAL=1 下 SKIP 不许充绿）")
            return 1
        if gm["rc"] != 0 or gmd.get("state") != "pass":
            fail(f"G28 门维持腿非绿 rc={gm['rc']} state={gmd.get('state')}: {gm['tail'][-200:]}")
            return 1
        gm_ok = (
            gmd.get("y_anchor_all_equal") is True
            and gmd.get("dec_consumed_all_equal") is True
            and gmd.get("in_tol") is True
            and float(gmd.get("p100_vs_host", "nan")) <= tol
            and (gmd.get("unbiased") or {}).get("pass") is True
            and gmd.get("bitexact") is True
        )
        if not gm_ok:
            fail(f"G28 门维持判据破缺: {json.dumps(gmd, ensure_ascii=False)[:300]}")
            return 1
        note(
            f"  G28 门维持复跑全绿: y 锚 20000/20000 p100={float(gmd['p100_vs_host']):.3e} "
            f"≤ tol={tol:.3e} 3σ dev={float((gmd.get('unbiased') or {}).get('dev', 'nan')):.3e} 双跑位级"
        )

        # ── 判据①③④⑤：车道双臂真跑（REQUIRE_REAL=1 + VK_VALIDATION=1）──
        cmp_out = WORK / "compare.json"
        res = run_compare(wire, N_TRIALS, TIMING_RUNS, tol, require_real=True, out=cmp_out)
        doc = res["doc"]
        if res["skipped"]:
            fail("车道 compare 腿 SKIP（RURIX_REQUIRE_REAL=1 下 SKIP 不许充绿）")
            return 1
        if res["rc"] != 0 or doc.get("state") != "pass":
            fail(f"车道 compare 腿非绿 rc={res['rc']} state={doc.get('state')}: {res['tail'][-200:]}")
            return 1
        off_arm = doc.get("off") or {}
        on_arm = doc.get("on") or {}
        for f in arm_green(off_arm, tol) + arm_green(on_arm, tol):
            fail(f)
        v_ok, v_red = variance_gate(off_arm, on_arm)
        if not v_ok:
            fail(f"方差对照方向破缺: var(off)/var(on)={v_red:.6f} 未 >1")
        for f in spatial_green(doc.get("spatial") or {}):
            fail(f)
        if not (frame_ms_registered(off_arm) and frame_ms_registered(on_arm)):
            fail("dispatch 墙钟 measured 登记破缺（正有限 + runs≥1）")
        if int(doc.get("seed", -1)) != WIRE_SEED:
            fail("车道 seed 漂移")
        if int(doc.get("n_trials", -1)) != N_TRIALS:
            fail("车道窗长漂移")
        if FAILURES:
            return 1
        note(
            f"  车道双臂绿: var {float(off_arm['variance']):.3e}→{float(on_arm['variance']):.3e} "
            f"（reduction {v_red:.3f}）dispatch_ms "
            f"{float(off_arm['dispatch_ms']['mean']):.4f}→{float(on_arm['dispatch_ms']['mean']):.4f}"
        )

        # ── 判据④：off 面静态锚零漂移（首跑程序产追加；复跑位级期望）──
        off_digest = str(off_arm["digest"])
        table_digest = str(doc["light_table_digest"])
        anchor_first_run = False
        if not ANCHOR_PATH.is_file():
            anchor_doc = {
                "schema": "rurix.g31.restir_wiring_off_anchor.v1",
                "scene_id": SCENE,
                "seed": WIRE_SEED,
                "n_trials": N_TRIALS,
                "m_candidates_off": 1,
                "light_table_digest": table_digest,
                "off_digest": off_digest,
                "frozen_at_utc": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                "note": ("G31+ 波 B Task B2 off 面（--restir off 默认档 = MegaLights 式均匀选灯语义"
                         "镜像，RIS M=1 代数恒等）静态锚：首跑程序产追加（budget 追加同律），复跑"
                         "位级期望零漂移，漂移即 RED；bistro-interior 契约 point_lights 灯表 digest"
                         " 入键（契约灯表变更 ⇒ 锚重订程序，禁静默改写）"),
            }
            ANCHOR_PATH.write_text(
                json.dumps(anchor_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )
            anchor_first_run = True
            note(f"  off 静态锚程序产追加: {ANCHOR_PATH.relative_to(ROOT)}")
        else:
            anchor_doc = json.loads(ANCHOR_PATH.read_text(encoding="utf-8"))
            drift = anchor_zero_drift(anchor_doc, off_digest, table_digest)
            if drift:
                for m in drift:
                    fail(f"off 面静态锚零漂移破缺: {m}")
                return 1
            note(f"  off 静态锚零漂移: digest={off_digest[:23]}… == 在档锚")

        # ── 判据⑥：冻结面 0-byte 机核（gi/ 两文件 + kernel，提交面+工作树双面）──
        gi_ok, gi_detail = frozen_0byte(FROZEN_GI_BASE, FROZEN_GI_FILES)
        if not gi_ok:
            fail(gi_detail)
        k_ok, k_detail = frozen_0byte(FROZEN_KERNEL_BASE, FROZEN_KERNEL_FILES)
        if not k_ok:
            fail(k_detail)
        if FAILURES:
            return 1
        note(f"  冻结面 0-byte: {gi_detail}；{k_detail}")

    # ── evidence 落盘 ──
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    spatial = doc.get("spatial") or {}
    ev = {
        "schema": SCHEMA_ID,
        "subject": "g31_restir_wiring",
        "symbolic_gate_key": GATE_KEY,
        "wave": "G31+.B",
        "scene_id": SCENE,
        "seed": WIRE_SEED,
        "n_trials": N_TRIALS,
        "light_table": {
            "n_lights": int(doc["n_lights"]),
            "digest": table_digest,
            "contract": "milestones/g13/g13_ue_upscale_parity_contract.json",
            "source": ("bistro-interior lighting.point_lights（生产管线 direct GI pass 现有灯表；"
                       "I_rgb=color_linear_rgb×intensity_cd 同口径标量投影）"),
        },
        "gate_maintenance": {
            "state": "pass",
            "y_anchor_all_equal": True,
            "dec_consumed_all_equal": True,
            "p100_vs_host": float(gmd["p100_vs_host"]),
            "tol": tol,
            "in_tol": True,
            "unbiased_3sigma_pass": True,
            "device_double_run_bitexact": True,
            "harness_digest": f"sha256:{gmd['digest']}",
        },
        "arms": [
            {
                "tier": "off",
                "m_candidates": 1,
                "mean": float(off_arm["mean"]),
                "reference": float(off_arm["reference"]),
                "variance": float(off_arm["variance"]),
                "dev": float(off_arm["dev"]),
                "bound_3sigma": float(off_arm["bound_3sigma"]),
                "unbiased_3sigma_pass": bool(off_arm["unbiased_3sigma_pass"]),
                "y_anchor_all_equal": bool(off_arm["y_anchor_all_equal"]),
                "dec_consumed_all_equal": bool(off_arm["dec_consumed_all_equal"]),
                "p100_vs_host": float(off_arm["p100_vs_host"]),
                "in_tol": bool(off_arm["in_tol"]),
                "digest": str(off_arm["digest"]),
                "double_run_bitexact": bool(off_arm["double_run_bitexact"]),
                "dispatch_ms": {
                    "mean": float(off_arm["dispatch_ms"]["mean"]),
                    "min": float(off_arm["dispatch_ms"]["min"]),
                    "max": float(off_arm["dispatch_ms"]["max"]),
                    "runs": int(off_arm["dispatch_ms"]["runs"]),
                },
            },
            {
                "tier": "on",
                "m_candidates": 16,
                "mean": float(on_arm["mean"]),
                "reference": float(on_arm["reference"]),
                "variance": float(on_arm["variance"]),
                "dev": float(on_arm["dev"]),
                "bound_3sigma": float(on_arm["bound_3sigma"]),
                "unbiased_3sigma_pass": bool(on_arm["unbiased_3sigma_pass"]),
                "y_anchor_all_equal": bool(on_arm["y_anchor_all_equal"]),
                "dec_consumed_all_equal": bool(on_arm["dec_consumed_all_equal"]),
                "p100_vs_host": float(on_arm["p100_vs_host"]),
                "in_tol": bool(on_arm["in_tol"]),
                "digest": str(on_arm["digest"]),
                "double_run_bitexact": bool(on_arm["double_run_bitexact"]),
                "dispatch_ms": {
                    "mean": float(on_arm["dispatch_ms"]["mean"]),
                    "min": float(on_arm["dispatch_ms"]["min"]),
                    "max": float(on_arm["dispatch_ms"]["max"]),
                    "runs": int(on_arm["dispatch_ms"]["runs"]),
                },
            },
        ],
        "spatial_reuse_arm": {
            "grid": "8x8",
            "n_points": 64,
            "n_trials": int(spatial.get("n_trials", N_TRIALS)),
            "aggregate_3sigma": {
                "mean": float(spatial["aggregate_3sigma"]["mean"]),
                "reference": float(spatial["aggregate_3sigma"]["reference"]),
                "dev": float(spatial["aggregate_3sigma"]["dev"]),
                "bound_3sigma": float(spatial["aggregate_3sigma"]["bound_3sigma"]),
                "pass": bool(spatial["aggregate_3sigma"]["pass"]),
            },
            "per_point_5sigma_all_within": bool(spatial["per_point_5sigma_all_within"]),
            "worst_dev_over_sigma": float(spatial["worst_dev_over_sigma"]),
            "variance_gain": {
                "min": float(spatial["variance_gain"]["min"]),
                "mean": float(spatial["variance_gain"]["mean"]),
                "max": float(spatial["variance_gain"]["max"]),
                "no_pass_line": True,
            },
            "double_run_bitexact": bool(spatial["double_run_bitexact"]),
            "digest": str(spatial["digest"]),
        },
        "variance_reduction": v_red,
        "frame_ms_compare": {
            "off_mean": float(off_arm["dispatch_ms"]["mean"]),
            "on_mean": float(on_arm["dispatch_ms"]["mean"]),
            "on_over_off": float(on_arm["dispatch_ms"]["mean"]) / float(off_arm["dispatch_ms"]["mean"]),
            "note": ("dispatch+回读墙钟（20000 trial 单批 = 单帧采样批口径；含上传/回读税，"
                     "双臂同形同价；measured 如实登记不设通过线——G6 无硬门纪律）"),
        },
        "determinism": {
            "on_double_run_bitexact": bool(on_arm["double_run_bitexact"]),
            "off_double_run_bitexact": bool(off_arm["double_run_bitexact"]),
        },
        "off_static_anchor": {
            "path": "milestones/g31/g31_restir_wiring_off_anchor.json",
            "digest": off_digest,
            "zero_drift": True,
            "program_produced_first_run": anchor_first_run,
        },
        "frozen_surfaces": {
            "gi_files_0byte_vs_g27_closed": gi_ok,
            "g28_kernel_0byte_vs_g28_closed": k_ok,
        },
        "environment": {
            "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
            "os": "windows",
            "rustc": run(["rustc", "--version"]).stdout.strip(),
            "base_commit": run(["git", "rev-parse", "HEAD"]).stdout.strip(),
        },
        "timestamp": ts,
        "notes": (
            f"ReSTIR 车道集成 measured（bistro-interior 契约 point_lights {int(doc['n_lights'])} 灯，"
            f"{N_TRIALS} trial × 双臂同批）：off（M=1，MegaLights 语义镜像）var="
            f"{float(off_arm['variance']):.6e} vs on（M=16 reservoir）var={float(on_arm['variance']):.6e}，"
            f"variance_reduction={v_red:.3f}（>1 方向硬门，数值如实登记）；dispatch_ms mean "
            f"off={float(off_arm['dispatch_ms']['mean']):.4f} → on={float(on_arm['dispatch_ms']['mean']):.4f}"
            f"（on/off={float(on_arm['dispatch_ms']['mean']) / float(off_arm['dispatch_ms']['mean']):.3f}，"
            f"如实登记不设通过线）；双臂 y 整数锚 20000/20000 全等 + p100 off="
            f"{float(off_arm['p100_vs_host']):.3e}/on={float(on_arm['p100_vs_host']):.3e} ≤ G28 冻结容差 "
            f"{tol:.3e} + 双臂无偏 3σ 过 + 双跑位级；空间重用加性臂聚合 3σ dev="
            f"{float(spatial['aggregate_3sigma']['dev']):.3e} + 逐点 5σ 全过 + 方差再收益 "
            f"{float(spatial['variance_gain']['min']):.3f}/{float(spatial['variance_gain']['mean']):.3f}/"
            f"{float(spatial['variance_gain']['max']):.3f} 如实登记（no_pass_line）；G28 门维持腿复跑全绿"
            f"（fixture 20000 trial y 锚 + p100={float(gmd['p100_vs_host']):.3e} + 3σ + 双跑位级）；"
            f"off 静态锚{'首跑程序产追加' if anchor_first_run else '复跑零漂移'}；"
            f"gi/ 两文件 vs g27-closed + kernel vs g28-closed 三处 0-byte 机核绿"
        ),
    }
    missing = evidence_required_keys(ev)
    if missing:
        fail(f"evidence 缺 required 键: {missing}")
        return 1
    ev_path = out_path or (ROOT / "evidence" / f"g31_restir_wiring_{ts}.json")
    ev_path.parent.mkdir(parents=True, exist_ok=True)
    ev_path.write_text(json.dumps(ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {ev_path}")

    ok = not FAILURES
    note(f"GATE {'PASS' if ok else 'FAIL'} {GATE_KEY}")
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--gate", default="")
    ap.add_argument("--out", default="")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        return gate(Path(args.out) if args.out else None)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
