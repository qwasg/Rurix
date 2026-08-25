#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent(G28.2 实现波)
"""G28.2 M-b 空间重用加性臂兑现门冒烟
(g28.p0.m_b.restir_spatial_reuse_arm;G28_CONTRACT §4.2 M-b 行判据逐字;
rfcs/0045-lighting-device-realization.md §2 判据事实源;
G28_ACCEPTANCE_MAP §1 M-b 行)。

硬判据:g28_restir_device --spatial 纯 host 加性臂(bin-local,gi/ 冻结面
0-byte)——8×8 着色点网格(N=8 闭集)× fixture_lights(64),每点每 trial 单流
stream=(t·64+p)·4+3,本点 estimate_ris(16 候选)→ gather 合并前快照闭集 →
von Neumann 4 邻接字面固定序 (−1,0)(+1,0)(0,−1)(0,+1) → **受点重评快照变换后
直调冻结 merge(m_cap=60,禁第二实现)**——判据(RFC-0045 §2.4):
①聚合 3σ 硬门(逐 trial 64 点均值序列的 20000-trial 均值 vs exact_direct
64 点均值,dev < 3σ_mean + 1e-9)②逐点 5σ 结构兜底(任一点超 5σ 即 FAIL)
③逐点 3σ 诊断表 64 行全量入 evidence 如实登记(非门面——3σ×64 族期望假红
≈ 0.17,多重比较口径注明)④空间方差再收益 measured 登记(min/mean/max 如实
登记,**不设通过线**——收益/无收益/负收益均如实落 evidence,G6 无硬门纪律)
⑤双跑位级(固定 seed 全网格两跑 estimate 矩阵位级相等)⑥gi/ 两文件
(restir_reservoir.rs + multi_light.rs)vs g27-closed 0-byte 机核。

纯 host 确定性面:--spatial 臂零 GPU 依赖恒跑(构建段持 gpu_device_lock 防
并行 cargo 互覆盖;运行段无锁)。

用法:
  py -3 ci/g28_restir_spatial_reuse_arm_smoke.py --gate g28.p0.m_b.restir_spatial_reuse_arm
  py -3 ci/g28_restir_spatial_reuse_arm_smoke.py --verify-latest
  py -3 ci/g28_restir_spatial_reuse_arm_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g28.p0.m_b.restir_spatial_reuse_arm"
NUMERIC_STEP = 482
SUBJECT = "g28_m_b_restir_spatial_reuse_arm"
WAVE = "G28.2"
SCHEMA_PATH = ROOT / "milestones/g28/g28_m_b_restir_spatial_reuse_arm_evidence_schema.json"
SOURCE_REF = (
    "G28_CONTRACT §4.2 M-b;rfcs/0045-lighting-device-realization.md §2;"
    "G28_ACCEPTANCE_MAP §1 M-b 行"
)
TAG = "g28_m_b"

HARNESS_BIN = "g28_restir_device"
SPATIAL_EVIDENCE_REL = "evidence/g28_restir_spatial_arm.json"
FROZEN_BASE = "g27-closed"
# RFC-0045 §2.4 ④:host 参考臂 + M100 低档生产默认面双冻结(§1.8 机核覆盖)。
FROZEN_FILES = [
    "src/rurix-render/src/gi/restir_reservoir.rs",
    "src/rurix-render/src/gi/multi_light.rs",
]
N_POINTS = 64

# facts 闭集(≥6;schema extra_facts minItems 6)。
FACT_IDS = [
    "aggregate_unbiased_3sigma",
    "per_point_5sigma_structural",
    "per_point_3sigma_diagnostic_registered",
    "variance_gain_registered",
    "double_run_bitexact",
    "host_frozen_0byte",
]


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def target_dir() -> Path:
    import os

    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin {HARNESS_BIN}")
    r = run(["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", HARNESS_BIN])
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return None
    exe = target_dir() / "debug" / (f"{HARNESS_BIN}.exe" if sys.platform == "win32" else HARNESS_BIN)
    return exe if exe.is_file() else None


def json_line(stdout: str, schema_token: str) -> str | None:
    for line in stdout.splitlines():
        if schema_token in line:
            return line.strip()
    return None


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面)
# ---------------------------------------------------------------------------


def _aggregate_pass(doc: dict) -> bool:
    agg = doc.get("aggregate_3sigma") or {}
    return agg.get("pass") is True and doc.get("state") == "pass"


def _per_point_5sigma_pass(doc: dict) -> bool:
    pp = doc.get("per_point_5sigma") or {}
    return pp.get("all_within") is True


def _diagnostic_registered(doc: dict) -> tuple[bool, str]:
    """逐点 3σ 诊断表 64 行全量登记核验(登记面非门面——行数完整性是门,
    within 计数如实登记不阻断)。"""
    rows = doc.get("per_point_rows") or []
    diag = doc.get("per_point_3sigma_diagnostic") or {}
    if len(rows) != N_POINTS:
        return False, f"诊断表行数 {len(rows)} ≠ {N_POINTS}"
    if diag.get("gate") is not False:
        return False, "诊断块缺 gate=false 字面(3σ 逐点为诊断登记面非门面)"
    keys_ok = all(
        all(k in row for k in ("p", "mean", "dev", "sigma_mean", "within_3sigma", "within_5sigma"))
        for row in rows
    )
    if not keys_ok:
        return False, "诊断行字段不全"
    return True, (
        f"64 行全量入 {SPATIAL_EVIDENCE_REL};within_3sigma={diag.get('within_count')}/{N_POINTS}"
        "(3σ×64 族期望假红 ≈ 0.17,多重比较口径已注明,非门面)"
    )


def _variance_gain_registered(doc: dict) -> tuple[bool, str]:
    vg = doc.get("variance_gain") or {}
    if not all(k in vg for k in ("min", "mean", "max")):
        return False, "variance_gain 缺 min/mean/max"
    if vg.get("no_pass_line") is not True:
        return False, "缺 no_pass_line=true 字面(方差再收益不设通过线)"
    return True, (
        f"var(no-reuse)/var(reuse) measured 登记:min={vg['min']:.6} mean={vg['mean']:.6} "
        f"max={vg['max']:.6};no-pass-line(收益/无收益/负收益均如实登记,RFC-0045 §2.4 ②)"
    )


def _bitexact_pass(doc: dict) -> bool:
    return doc.get("double_run_bitexact") is True


# ---------------------------------------------------------------------------
# gi/ 冻结 0-byte 机核(RFC-0045 §1.8/§2.4 ④:vs g27-closed + 工作树双面)
# ---------------------------------------------------------------------------


def host_frozen_0byte() -> tuple[bool, str]:
    r = run(["git", "diff", "--quiet", FROZEN_BASE, "--", *FROZEN_FILES])
    if r.returncode != 0:
        d = run(["git", "diff", "--name-only", FROZEN_BASE, "--", *FROZEN_FILES])
        changed = [x.strip() for x in d.stdout.splitlines() if x.strip()]
        return False, f"gi/ 冻结面有差分 vs {FROZEN_BASE}(触碰即 RED): {changed[:3]}"
    u = run(["git", "status", "--porcelain", "--", *FROZEN_FILES])
    if u.stdout.strip():
        dirty = [x for x in u.stdout.splitlines() if x.strip()]
        return False, f"gi/ 冻结面工作树未提交面: {dirty[:3]}"
    return True, (
        f"git diff --quiet {FROZEN_BASE} -- restir_reservoir.rs+multi_light.rs 0-byte"
        "(提交面 + 工作树双面;空间臂全 bin-local 冻结文件零触碰)"
    )


# ---------------------------------------------------------------------------
# gate
# ---------------------------------------------------------------------------


def run_gate() -> int:
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}

    # 构建段持锁(并行 cargo 互覆盖防护);--spatial 纯 host 运行段无锁。
    with gpu_device_lock(purpose=f"{TAG} harness 构建"):
        harness = build_harness()

    if harness is None:
        set_fact("aggregate_unbiased_3sigma", False, "harness 构建失败")
    else:
        print(f"[{TAG}] 空间臂: --spatial --out {SPATIAL_EVIDENCE_REL}(纯 host)")
        r = run(
            [str(harness), "--spatial", "--out", str(ROOT / SPATIAL_EVIDENCE_REL)],
            timeout=3600,
        )
        line = json_line(r.stdout, "rurix.g28restir.spatial.v1")
        doc = json.loads(line) if line else {}
        if not doc:
            for fid in FACT_IDS[:5]:
                set_fact(fid, False, f"--spatial 无 evidence 行 rc={r.returncode}: {(r.stdout + r.stderr)[-200:]}")
        else:
            agg = doc.get("aggregate_3sigma") or {}
            win = doc.get("window") or {}
            set_fact(
                "aggregate_unbiased_3sigma",
                _aggregate_pass(doc) and r.returncode == 0,
                f"聚合硬门:逐 trial 64 点均值序列的 {win.get('n_trials')}-trial 均值 "
                f"{agg.get('mean')} vs exact_direct 64 点均值 {agg.get('reference')};"
                f"dev={agg.get('dev')} < 3σ_mean+1e-9={agg.get('bound_3sigma')};"
                f"窗长口径={win.get('n_trials')} trial(downgraded={win.get('downgraded')},"
                f"单跑 {win.get('single_run_seconds')}s)",
            )
            pp = doc.get("per_point_5sigma") or {}
            set_fact(
                "per_point_5sigma_structural",
                _per_point_5sigma_pass(doc),
                f"逐点 5σ 结构兜底:all_within={pp.get('all_within')};"
                f"worst dev/σ={pp.get('worst_dev_over_sigma')} @p{pp.get('worst_point')}"
                "(5σ×64 族假红 ≈ 3.7e-5 可忽略)",
            )
            diag_ok, diag_detail = _diagnostic_registered(doc)
            set_fact("per_point_3sigma_diagnostic_registered", diag_ok, diag_detail)
            vg_ok, vg_detail = _variance_gain_registered(doc)
            set_fact("variance_gain_registered", vg_ok, vg_detail)
            set_fact(
                "double_run_bitexact",
                _bitexact_pass(doc),
                "固定 seed 全网格两跑 estimate 矩阵(reuse+no-reuse)位级相等"
                if _bitexact_pass(doc) else "双跑矩阵位级不等",
            )

    ok, detail = host_frozen_0byte()
    set_fact("host_frozen_0byte", ok, detail)

    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=fact_rows,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes=(
            "G28.2 M-b:空间重用加性臂兑现(bin-local 纯 host)——8×8 网格 × 64 灯,"
            "流 (t·64+p)·4+3;gather 合并前快照闭集 + von Neumann 4 邻字面固定序 + "
            "受点重评快照变换后直调冻结 merge(m_cap=60,零合并判定复刻);聚合 3σ 硬门 "
            "+ 逐点 5σ 结构兜底 + 逐点 3σ 诊断表 64 行全量登记(非门面,多重比较口径"
            "注明)+ 方差再收益 min/mean/max measured 登记不设通过线 + 双跑位级 + "
            f"gi/ 两文件 0-byte(vs g27-closed);全量臂产物 {SPATIAL_EVIDENCE_REL}"
        ),
        host_section_pass=all_pass,
    )
    return 0 if (all_pass and code == 0) else 1


# ---------------------------------------------------------------------------
# selftest(反 YAML-only:判读器红绿两臂,无 GPU/无构建依赖)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    # facts 闭集 ≥6 且 schema 在树(extra_facts minItems 6 被满足)。
    expect(len(FACT_IDS) >= 6, f"facts 闭集 {len(FACT_IDS)} ≥ 6")
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    min_items = schema.get("properties", {}).get("extra_facts", {}).get("minItems", 99)
    expect(len(FACT_IDS) >= min_items, f"facts {len(FACT_IDS)} ≥ schema minItems {min_items}")

    good_row = {
        "p": 0, "mean": 1.0, "dev": 0.0, "sigma_mean": 1.0,
        "within_3sigma": True, "within_5sigma": True,
    }
    good = {
        "state": "pass",
        "aggregate_3sigma": {"pass": True},
        "per_point_5sigma": {"all_within": True},
        "per_point_3sigma_diagnostic": {"gate": False, "within_count": 64},
        "per_point_rows": [dict(good_row, p=i) for i in range(N_POINTS)],
        "variance_gain": {"min": 0.9, "mean": 2.0, "max": 2.7, "no_pass_line": True},
        "double_run_bitexact": True,
    }
    # 红臂①:聚合 3σ 假绿注入必拒。
    expect(not _aggregate_pass({**good, "aggregate_3sigma": {"pass": False}}), "RED:聚合 3σ fail 必拒")
    expect(not _aggregate_pass({**good, "state": "fail"}), "RED:state=fail 必拒")
    expect(_aggregate_pass(good), "GREEN:聚合 3σ 正例")
    # 红臂②:逐点 5σ 兜底漏检必拒。
    expect(not _per_point_5sigma_pass({**good, "per_point_5sigma": {"all_within": False}}), "RED:5σ 超限必拒")
    expect(_per_point_5sigma_pass(good), "GREEN:5σ 正例")
    # 红臂③:诊断表 64 行完整性——行缺失/gate 冒充门面必拒;within<64 如实登记不阻断。
    expect(not _diagnostic_registered({**good, "per_point_rows": good["per_point_rows"][:63]})[0], "RED:诊断表 63 行必拒")
    expect(
        not _diagnostic_registered(
            {**good, "per_point_3sigma_diagnostic": {"gate": True, "within_count": 64}}
        )[0],
        "RED:诊断块 gate=true 冒充门面必拒",
    )
    expect(
        _diagnostic_registered(
            {**good, "per_point_3sigma_diagnostic": {"gate": False, "within_count": 62}}
        )[0],
        "GREEN:within=62/64 如实登记不阻断(诊断面非门面)",
    )
    # 红臂④:方差再收益登记——缺字段/缺 no-pass-line 字面必拒;负收益如实登记过。
    expect(not _variance_gain_registered({**good, "variance_gain": {"min": 1.0, "mean": 2.0}})[0], "RED:缺 max 必拒")
    expect(
        not _variance_gain_registered(
            {**good, "variance_gain": {"min": 1.0, "mean": 2.0, "max": 3.0, "no_pass_line": False}}
        )[0],
        "RED:no_pass_line=false 必拒",
    )
    expect(
        _variance_gain_registered(
            {**good, "variance_gain": {"min": 0.5, "mean": 0.8, "max": 0.9, "no_pass_line": True}}
        )[0],
        "GREEN:负收益如实登记过(不设通过线)",
    )
    # 红臂⑤:双跑位级漂移必拒。
    expect(not _bitexact_pass({**good, "double_run_bitexact": False}), "RED:双跑漂移必拒")
    expect(_bitexact_pass(good), "GREEN:双跑位级正例")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts={len(FACT_IDS)};5 红臂组 + 正例组)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=None)
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    if args.gate is not None and args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
