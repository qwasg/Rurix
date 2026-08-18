#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.6/G12.7 收口波）
"""G12.7a stabilization soak 聚合门 g12.wave.7a.soak(G12_CONTRACT G-G12-9;
G12_PLAN §2 G12.7a;CI_GATES §5 wave7a 行;同构 ci/g11_stabilization_soak.py)。

四腿:①全量回归(8 P0 + 1 go P1 = 9 key 逐门真跑 --gate——机器核验最新 evidence
顶层 status=="pass" 字面〔G12 证据形态统一,无豁免面——缺字段即红〕;wave2~wave5
exit + wave6 decisions 五聚合/决策门真跑核验;9 门 evidence base_commit 同值
=HEAD 且 14 门 evidence 文件名 UTC stamp ≥ run 起点 = 同一候选 close-out 基线,
沿 G10.8a/G11.7a MAP §7 口径)+ ②PT 生产化链路(出图→降噪→对标装配→吞吐基线)
连续复跑 soak(≥1800s 墙钟沿 G11.7a/G10.8a/G9.8a 继承;真实链路逐迭代连续复跑,
迭代计数与各环节计数非空、零失败;sleep_seconds 恒 0,active_chain_seconds
≈seconds,gate 外测墙钟交叉核验,谎报判红)+ ③budget_eval --strict 非空零
estimated/skip + ④纪律日期锚;G-G12-9 另含「G5~G11 既有判据 0-byte」独立 fact
(git status --porcelain 闭集面空集,与 M164 门 ③ 同闭集字面)。

诚实语义(沿 G11.7a/G10.8a/G9.8a/G8.8a 2026-08-08 清零假绿后口径,G12 无 legacy 兼容):
- soak 墙钟=真实链路复跑实测(active_chain_seconds 逐迭代计时求和),迭代间
  零 sleep(sleep_seconds 恒 0);gate 侧用外测墙钟交叉核验,谎报 seconds 判红。
- soak 载体 = G12 生产化链路四面(与 M158~M165 门数据面同构):
  出图腿 = g12_4_ue_pt_parity_render --render 双场景 spp16 device 真跑,
    契约 digest == 冻结注册值(不等拒出图)+ receipt frame_content_digest ==
    M163 Rurix 臂冻结锚(FROZEN_FRAME_DIGESTS,固定 seed 位级复现);
  降噪腿 = g12_pt_production --gate g12.p0.m162.denoise_pipeline_tsr 双 kernel
    device 真跑(时域累积 + firefly 预钳位 + A-trous 管线全档,pbrt 参照缓存面,
    SPV 循环外一次编译);
  对标装配腿 = 最新 M163 evidence parity 节重建 metrics → build_gap_registry
    当次重装配与在树 g12_ue_pt_gap_registry.json 幂等复核(逐字段全等 +
    evidence_digest 自证指针 == metrics 重算 digest);
  吞吐基线腿 = g12_4_ue_pt_parity_render --benchmark 轻量复跑(warmup 4 + timed
    8,场景逐迭代轮转——链复跑口径如实登记,不冒充 M165 冻结 50×3 协议),首帧
    digest == 冻结锚 + distinct==1 + timed_count==8;吞吐守护阈归 M165 门本体
    (①回归腿 50×3 全协议 ×1.5/÷1.5 守护——轻量链复跑冷启动面与冻结协议口径
    不同构,不以轻量腿帧时冒充守护判定,帧时样本进 notes 信息面不充判据)。
  device 腿持 gpu_device_lock 串行;UE 帧为帧区只读解码面(不重复 MRQ,与
  G11.7a 同口径——UE 臂真跑归 ①回归腿 M163 门本体)。
- 迭代计数/出图帧数/降噪管线次数/清单装配次数/吞吐复跑次数/吞吐计时帧数非空
  (空即红),failures 恒 0;evidence soak 块 seconds 与 iterations 双字段机器可核。

pr-smoke 默认 --verify-latest(秒级核最新 full-run evidence);
本地/workflow_dispatch 用 --gate 产 full-run。

用法:
  py -3 ci/g12_stabilization_soak.py --gate g12.wave.7a.soak
  py -3 ci/g12_stabilization_soak.py --verify-latest
  py -3 ci/g12_stabilization_soak.py --selftest
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g12_wave_exit_lib as wel  # noqa: E402
import g12_pt_prod_lib as gl  # noqa: E402
from g12_denoise_pipeline_tsr_smoke import (  # noqa: E402
    DENOISE_KERNEL,
    load_denoise_calibration_from_budget,
)
from g12_pt_throughput_baseline_smoke import (  # noqa: E402
    FROZEN_CONTRACT_DIGEST,
    FROZEN_FRAME_DIGESTS,
    GLTF_PATHS,
    SEED,
)
from g12_ue_pt_parity_smoke import CONTRACT_PATH, SCENES, build_gap_registry  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g12.wave.7a.soak"
NUMERIC_STEP = 231  # 落盘前实测 registry/number_ledger.json CI_step.next_free=231 顺位领取
SUBJECT = "g12_stabilization_soak"
WAVE = "G12.7a"
SOURCE_REF = (
    "G12_CONTRACT G-G12-9;G12_PLAN §2 G12.7a;CI_GATES §5 wave7a;"
    "8 P0 + 1 go P1 全量回归(顶层 status==pass 字面)+ wave2~wave5 exit + "
    "wave6 decisions 聚合/决策门 + PT 生产化链路(出图/降噪/对标装配/吞吐基线)"
    "连续复跑 soak ≥1800s + budget --strict + G5~G11 既有判据 0-byte"
)
SCHEMA_PATH = ROOT / "milestones" / "g12" / "g12_stabilization_soak_evidence_schema.json"
GAP_REGISTRY_PATH = ROOT / "milestones" / "g12" / "g12_ue_pt_gap_registry.json"
M162_GATE_KEY = "g12.p0.m162.denoise_pipeline_tsr"

# (symbolic_key, subject_prefix, smoke argv relative)
# 顺序即执行序:9 断言门(8 P0 + 1 go P1)先,波聚合/决策门后(只读汇总 14 门最新
# evidence)。G12 门 smoke 各自管理 device 腿/gpu_device_lock(pr-smoke 体例,
# 聚合门不设 RURIX_REQUIRE_REAL)。
REGRESSION_GATES: list[tuple[str, str, list[str]]] = [
    # ── G12.2 波(步骤 217~221)──
    ("g12.p1.m166.pt_production_calibration", "g12_pt_production_calibration",
     ["ci/g12_pt_production_calibration_smoke.py", "--gate", "g12.p1.m166.pt_production_calibration"]),
    ("g12.p0.m158.mis_full_surface", "g12_m158_mis_full_surface",
     ["ci/g12_mis_full_surface_smoke.py", "--gate", "g12.p0.m158.mis_full_surface"]),
    ("g12.p0.m159.russian_roulette_prod", "g12_m159_russian_roulette_prod",
     ["ci/g12_russian_roulette_prod_smoke.py", "--gate", "g12.p0.m159.russian_roulette_prod"]),
    ("g12.p0.m160.sampling_lds_upgrade", "g12_m160_sampling_lds_upgrade",
     ["ci/g12_sampling_lds_upgrade_smoke.py", "--gate", "g12.p0.m160.sampling_lds_upgrade"]),
    ("g12.p0.m161.convergence_criterion_prod", "g12_m161_convergence_criterion_prod",
     ["ci/g12_convergence_criterion_prod_smoke.py", "--gate", "g12.p0.m161.convergence_criterion_prod"]),
    # ── G12.3 波(步骤 223)──
    ("g12.p0.m162.denoise_pipeline_tsr", "g12_m162_denoise_pipeline_tsr",
     ["ci/g12_denoise_pipeline_tsr_smoke.py", "--gate", "g12.p0.m162.denoise_pipeline_tsr"]),
    # ── G12.4 波(步骤 225~226)──
    ("g12.p0.m163.ue_pt_parity", "g12_m163_ue_pt_parity",
     ["ci/g12_ue_pt_parity_smoke.py", "--gate", "g12.p0.m163.ue_pt_parity"]),
    ("g12.p0.m164.regression_guard", "g12_m164_regression_guard",
     ["ci/g12_regression_guard_smoke.py", "--gate", "g12.p0.m164.regression_guard"]),
    # ── G12.5 波(步骤 228)──
    ("g12.p0.m165.pt_throughput_baseline", "g12_m165_pt_throughput_baseline",
     ["ci/g12_pt_throughput_baseline_smoke.py", "--gate", "g12.p0.m165.pt_throughput_baseline"]),
    # ── 波聚合/决策门(步骤 222/224/227/229/230;只读汇总不重跑子门 smoke)──
    ("g12.wave.2.exit", "g12_wave2_exit",
     ["ci/g12_wave2_exit_check.py", "--gate", "g12.wave.2.exit"]),
    ("g12.wave.3.exit", "g12_wave3_exit",
     ["ci/g12_wave3_exit_check.py", "--gate", "g12.wave.3.exit"]),
    ("g12.wave.4.exit", "g12_wave4_exit",
     ["ci/g12_wave4_exit_check.py", "--gate", "g12.wave.4.exit"]),
    ("g12.wave.5.exit", "g12_wave5_exit",
     ["ci/g12_wave5_exit_check.py", "--gate", "g12.wave.5.exit"]),
    ("g12.wave.6.decisions", "g12_p2_decisions",
     ["ci/g12_p2_decisions_check.py", "--gate", "g12.wave.6.decisions"]),
]

MIN_SECONDS = 1800  # 沿 G11.7a/G10.8a/G9.8a 继承(≥30min;G-G12-9「或 measured 证明更短足够」未触)
MIN_ITERATIONS = 3  # 全链路迭代计数非空下界
# 9 门(8 P0 + 1 go P1)为门 evidence(顶层 status=="pass" 字面 + base_commit);
# 后 5 门为波聚合/决策 evidence(wel 口径,无顶层 status/base_commit 字段)。
N_ASSERTION_GATES = 9

SOAK_WORK = ROOT / ".tmp" / "g12_gates" / "soak_7a"
BENCH_WARMUP = 4  # 吞吐基线腿轻量链复跑口径(如实登记,不冒充 M165 冻结 50×3 协议)
BENCH_TIMED = 8
_BENCH_LAST_TIMES: dict[str, list[float]] = {}  # 轻量腿帧时信息面(notes 登记,不充判据)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def base_commit() -> str:
    r = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True,
    )
    return (r.stdout or "").strip() or "unknown"


def verify_assertion_gate(key: str, prefix: str) -> dict:
    """9 门(8 P0 + 1 go P1)最新 evidence 机器核验。

    在 wel.require_gate_pass 口径(symbolic_gate_key/host_section_pass/
    device_section_state/checks 全真)之上叠加顶层 status=="pass" 字面
    (MAP §1 evidence 必备字段,skip/estimated 不充绿;G12 证据形态统一,无豁免
    面——缺字段即红)。
    """
    row = wel.require_gate_pass(key, prefix)
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return row
    try:
        doc = wel.load_json(path)
    except (OSError, json.JSONDecodeError):
        return row
    problems: list[str] = []
    if "status" in doc:
        if doc.get("status") != "pass":
            problems.append(f"status={doc.get('status')!r} ≠ 'pass'")
    else:
        problems.append("缺顶层 status 字段(G12 证据形态统一,无豁免面)")
    if problems:
        row["status"] = "FAIL"
        row["detail"] = f"{row.get('detail', '')}; " + "; ".join(problems)
    return row


def run_regression(*, skip_rerun: bool = False) -> tuple[bool, list[dict], str, bool]:
    """全量回归(9 门 + 5 波聚合/决策门)。口径沿 G11.7a run_regression 同构。"""
    rows: list[dict] = []
    commit = base_commit()
    run_start_stamp = wel.utc_stamp()
    all_ok = True
    bases: list[str] = []
    no_base_field: list[str] = []
    stale: list[str] = []
    stamp_re = re.compile(r"_(\d{8}T\d{6}Z)\.json$")
    for idx, (key, prefix, argv) in enumerate(REGRESSION_GATES):
        is_aggregate = idx >= N_ASSERTION_GATES
        if not skip_rerun:
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
            print(f"[7a] regression {key}", flush=True)
            r = subprocess.run(
                [sys.executable, str(script), *argv[1:]],
                cwd=ROOT,
            )
            if r.returncode != 0:
                rows.append(
                    {
                        "symbolic_gate_key": key,
                        "subject_prefix": prefix,
                        "evidence_path": None,
                        "status": "FAIL",
                        "detail": f"smoke exit={r.returncode}",
                    }
                )
                all_ok = False
                continue
        if is_aggregate:
            row = wel.require_gate_pass(key, prefix)
        else:
            row = verify_assertion_gate(key, prefix)
            path = wel.load_latest_evidence(prefix)
            if path is not None:
                try:
                    doc = wel.load_json(path)
                    bc = doc.get("base_commit")
                    if bc is None:
                        no_base_field.append(key)
                    else:
                        bases.append(str(bc))
                except (OSError, json.JSONDecodeError):
                    bases.append("<unreadable>")
        if not skip_rerun and row.get("evidence_path"):
            m = stamp_re.search(str(row["evidence_path"]))
            if m is None or m.group(1) < run_start_stamp:
                stale.append(key)
                row["status"] = "FAIL"
                row["detail"] = (
                    f"{row.get('detail', '')}; evidence 非本 run 新鲜产出"
                    f"(stamp {m.group(1) if m else '?'} < run 起点 {run_start_stamp})"
                )
        rows.append(row)
        if row["status"] != "PASS":
            all_ok = False
    base_uniform = (
        not stale
        and not no_base_field
        and len(bases) == N_ASSERTION_GATES
        and len(set(bases)) == 1
        and bases[0] == commit
        and commit != "unknown"
    )
    return all_ok, rows, commit, base_uniform


def judge_chain_soak(
    doc: dict,
    *,
    outer_elapsed: float | None,
    min_seconds: int,
    min_iterations: int,
) -> tuple[bool, list[str]]:
    """诚实判定 PT 生产化链路 soak 输出。返回 (ok, problems)。口径沿 G11.7a 同构:
    honesty 字段必填 / sleep_seconds==0 / active≈seconds / 外测墙钟交叉核验 /
    双阈值 / failures==0 / 各环节计数非空。"""
    problems: list[str] = []
    iterations = int(doc.get("iterations") or 0)
    seconds = float(doc.get("seconds") or 0.0)
    if doc.get("soak_subject") != "chain-soak":
        problems.append(f"soak_subject={doc.get('soak_subject')!r} ≠ 'chain-soak'(缺 honesty 字段)")
    sleep_s = doc.get("sleep_seconds")
    if sleep_s is None:
        problems.append("缺 sleep_seconds 字段(伪造 → 红)")
    elif float(sleep_s) != 0.0:
        problems.append(f"sleep_seconds={sleep_s} ≠ 0(sleep 充墙钟)")
    active = doc.get("active_chain_seconds")
    if active is None:
        problems.append("缺 active_chain_seconds 字段")
    elif abs(float(active) - seconds) > 2.0:
        problems.append(
            f"active_chain_seconds={active} 与 seconds={seconds} 偏差 >2s(墙钟非链路复跑产出)"
        )
    if iterations < min_iterations:
        problems.append(f"iterations={iterations} < min_iterations={min_iterations}")
    if seconds < min_seconds:
        problems.append(f"seconds={seconds:.1f} < min_seconds={min_seconds}")
    if outer_elapsed is not None and outer_elapsed + 2.0 < seconds:
        problems.append(
            f"外测墙钟 {outer_elapsed:.1f}s < 自称 seconds={seconds:.1f}s(谎报时长)"
        )
    if int(doc.get("failures") or 0) != 0:
        problems.append(f"failures={doc.get('failures')} ≠ 0(链路迭代失败)")
    for k in (
        "pt_frames_rendered",
        "denoise_pipeline_runs",
        "gap_registry_assemblies",
        "throughput_baseline_reruns",
        "throughput_frames_timed",
    ):
        if int(doc.get(k) or 0) < 1:
            problems.append(f"{k} 计数面空")
    return (not problems), problems


# ---------------------------------------------------------------------------
# PT 生产化链路 soak(出图 → 降噪 → 对标装配 → 吞吐基线)
# ---------------------------------------------------------------------------


def _render_leg(bench_bin: Path, spv: Path, tau: float, work: Path) -> str | None:
    """出图腿:双场景 spp16 device 真跑,契约 digest 冻结 + receipt digest ==
    M163 Rurix 臂冻结锚(固定 seed 位级复现)。返回问题串(None=绿)。"""
    out_dir = work / "render"
    out_dir.mkdir(parents=True, exist_ok=True)
    env = gl.device_env()
    for scene in SCENES:
        r = gl.run(
            [
                str(bench_bin), "--render", "--scene", scene, "--spp", "16",
                "--seed", str(SEED), "--tau", repr(tau),
                "--contract", str(CONTRACT_PATH), "--gltf", GLTF_PATHS[scene],
                "--spv", str(spv), "--out-dir", str(out_dir),
                "--expect-digest", FROZEN_CONTRACT_DIGEST,
            ],
            env=env, timeout=3600,
        )
        out = r.stdout + r.stderr
        if "G12_4_PT: SKIP" in r.stdout:
            return f"出图腿 SKIP({scene};DEV_ENV_DEGRADE 不充绿)"
        if r.returncode != 0 or "G12_4_PT: PASS" not in r.stdout:
            return f"出图腿失败({scene}): rc={r.returncode} {out.strip()[-300:]}"
        receipt_path = out_dir / f"{scene}_spp16_receipt.json"
        if not receipt_path.is_file():
            return f"出图腿 receipt 缺失({scene})"
        try:
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            return f"出图腿 receipt 不可解析({scene}): {e}"
        want = FROZEN_FRAME_DIGESTS[(scene, 16)]
        got = receipt.get("frame_content_digest")
        if got != want:
            return f"出图腿帧 digest 漂移({scene}): {got} ≠ M163 冻结锚 {want}"
        if receipt.get("double_run_bitexact") is not True:
            return f"出图腿双跑位级面破缺({scene})"
    return None


def _denoise_leg(harness: Path, spv: Path, dn_spv: Path, cal: dict, work: Path) -> str | None:
    """降噪腿:M162 device 降噪管线全档真跑(时域累积 + firefly 预钳位 +
    A-trous 双帧管线,pbrt 参照缓存面)。返回问题串(None=绿)。"""
    ev_path = work / "m162_harness_evidence.json"
    r = gl.run(
        [
            str(harness), "--gate", M162_GATE_KEY,
            "--spv", str(spv), "--denoise-spv", str(dn_spv),
            "--evidence", str(ev_path),
            "--pbrt", str(gl.PBRT_EXE), "--imgtool", str(gl.IMGTOOL_EXE),
            "--work-dir", str(gl.WORK_DIR / "pbrt_work"),
            "--tau", repr(cal["tau"]),
            "--sampler", gl.winner_cli_name(cal["winner"]),
            "--hf-drop-min", repr(cal["hf_drop_min"]),
            "--mean-energy-tol", repr(cal["mean_energy_tol"]),
        ],
        env=gl.device_env(), timeout=3600,
    )
    out = r.stdout + r.stderr
    if "G12_PT_PROD: SKIP" in r.stdout:
        return "降噪腿 SKIP(DEV_ENV_DEGRADE 不充绿)"
    if r.returncode != 0 or "G12_PT_PROD: PASS" not in r.stdout:
        return f"降噪腿失败: rc={r.returncode} {out.strip()[-300:]}"
    return None


def _registry_leg() -> str | None:
    """对标装配腿:最新 M163 evidence parity 节重建 metrics → build_gap_registry
    当次重装配,与在树 g12_ue_pt_gap_registry.json 幂等复核(逐字段全等 +
    evidence_digest 自证指针 == metrics 重算 digest)。返回问题串(None=绿)。"""
    path = wel.load_latest_evidence("g12_m163_ue_pt_parity")
    if path is None:
        return "对标装配腿缺最新 M163 evidence"
    try:
        doc = wel.load_json(path)
    except (OSError, json.JSONDecodeError) as e:
        return f"对标装配腿 M163 evidence 不可解析: {e}"
    parity = doc.get("parity") or {}
    segs = parity.get("curve_segments") or []
    noise = parity.get("noise_spectrum_delta") or {}
    energy = parity.get("energy_conservation_delta") or {}
    if not segs or not noise or not energy:
        return "对标装配腿 parity 节缺面(curve_segments/noise/energy)"
    metrics: dict = {"scenes": {}}
    for scene in SCENES:
        metrics["scenes"][scene] = {
            "curve_segments": [
                {
                    "spp": s["spp"],
                    "rel_err_ue": s["rel_err_ue"],
                    "rel_err_rurix": s["rel_err_rurix"],
                    "delta": s["delta"],
                }
                for s in segs
                if s.get("scene") == scene
            ],
            "noise_spectrum": noise[scene],
            "energy": energy[scene],
        }
    budget = gl.load_budget()
    tolerances: dict[str, float] = {}
    for eid, key in (
        ("g12.pt.parity_curve_tol", "curve"),
        ("g12.pt.parity_noise_tol", "noise"),
        ("g12.pt.parity_energy_tol", "energy"),
    ):
        entry = gl.budget_entry(budget, eid)
        if entry is None or entry.get("evidence") != "measured_local":
            return f"对标装配腿 budget 标定条目缺失/非 measured: {eid}"
        tolerances[key] = float(entry["threshold"])
    ev_digest = hashlib.sha256(
        json.dumps(metrics, sort_keys=True).encode("utf-8")
    ).hexdigest()
    registry = build_gap_registry(metrics, tolerances, "sha256:" + ev_digest)
    if not GAP_REGISTRY_PATH.is_file():
        return "对标装配腿在树差距登记表缺失"
    try:
        on_tree = json.loads(GAP_REGISTRY_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        return f"对标装配腿在树登记表不可解析: {e}"
    digests = {d.get("evidence_digest") for it in on_tree.get("items", []) for d in it.get("measured_delta", [])}
    if digests != {"sha256:" + ev_digest}:
        return (
            f"对标装配腿 evidence_digest 自证指针失效: 在树 {sorted(digests)} "
            f"≠ metrics 重算 sha256:{ev_digest}"
        )
    if registry != on_tree:
        return "对标装配腿重装配与在树漂移(逐字段全等机核)"
    return None


def _benchmark_leg(bench_bin: Path, spv: Path, tau: float, scene: str) -> str | None:
    """吞吐基线腿:--benchmark 轻量链复跑(warmup 4 + timed 8,如实登记不冒充
    M165 冻结 50×3 协议),首帧 digest == 冻结锚 + distinct==1 + timed_count==8;
    吞吐守护阈归 M165 门本体(①回归腿 50×3 全协议)——轻量腿帧时不充守护判定。
    返回问题串(None=绿)。"""
    r = gl.run(
        [
            str(bench_bin), "--benchmark", "--scene", scene, "--spp", "16",
            "--seed", str(SEED), "--tau", repr(tau),
            "--contract", str(CONTRACT_PATH), "--gltf", GLTF_PATHS[scene],
            "--spv", str(spv), "--warmup", str(BENCH_WARMUP), "--frames", str(BENCH_TIMED),
            "--expect-digest", FROZEN_CONTRACT_DIGEST,
        ],
        env=gl.device_env(), timeout=3600,
    )
    out = r.stdout or ""
    if "G12_4_PT: SKIP" in out:
        return f"吞吐基线腿 SKIP({scene};DEV_ENV_DEGRADE 不充绿)"
    if r.returncode != 0:
        return f"吞吐基线腿失败({scene}): rc={r.returncode} {(r.stderr or out).strip()[-300:]}"
    try:
        doc = json.loads(out.strip().splitlines()[-1])
    except (json.JSONDecodeError, IndexError) as e:
        return f"吞吐基线腿输出解析失败({scene}): {e}"
    if doc.get("schema") != "rurix.g12.pt_throughput_bench.v1":
        return f"吞吐基线腿输出 schema 字面不符({scene})"
    if doc.get("contract_digest") != FROZEN_CONTRACT_DIGEST:
        return f"吞吐基线腿契约 digest 不等({scene})"
    want = FROZEN_FRAME_DIGESTS[(scene, 16)]
    if doc.get("first_frame_digest") != want:
        return f"吞吐基线腿首帧 digest 漂移({scene}): {doc.get('first_frame_digest')} ≠ 冻结锚 {want}"
    if doc.get("distinct_frame_digests") != 1:
        return f"吞吐基线腿全帧 digest 非单值({scene}): {doc.get('distinct_frame_digests')}"
    if doc.get("timed_count") != BENCH_TIMED:
        return f"吞吐基线腿计时帧数不符({scene}): {doc.get('timed_count')} ≠ {BENCH_TIMED}"
    samples = doc.get("frame_ms") or []
    if not samples:
        return f"吞吐基线腿帧时样本空({scene})"
    _BENCH_LAST_TIMES[scene] = [round(float(x), 3) for x in samples]
    return None


def _soak_iteration(ctx: dict, iteration: int) -> list[str]:
    """单轮 PT 生产化链路复跑(出图→降噪→对标装配→吞吐基线)。fail-fast。"""
    p = _render_leg(ctx["bench_bin"], ctx["spv"], ctx["tau"], ctx["work"])
    if p:
        return [p]
    p = _denoise_leg(ctx["harness"], ctx["spv"], ctx["dn_spv"], ctx["cal"], ctx["work"])
    if p:
        return [p]
    p = _registry_leg()
    if p:
        return [p]
    scene = SCENES[iteration % len(SCENES)]
    p = _benchmark_leg(ctx["bench_bin"], ctx["spv"], ctx["tau"], scene)
    if p:
        return [p]
    return []


def run_chain_soak(
    *,
    min_seconds: int = MIN_SECONDS,
    min_iterations: int = MIN_ITERATIONS,
) -> tuple[bool, dict]:
    """PT 生产化链路连续复跑 soak:≥min_seconds 墙钟内逐迭代真跑,零 sleep,零失败。"""
    counters = {
        "iterations": 0,
        "failures": 0,
        "pt_frames_rendered": 0,
        "denoise_pipeline_runs": 0,
        "gap_registry_assemblies": 0,
        "throughput_baseline_reruns": 0,
        "throughput_frames_timed": 0,
    }
    problems: list[str] = []
    # 构建一次(循环外;release 出图/吞吐 harness + rurixc SPV 双面 + debug 降噪
    # harness——与各门同 binary 面;bin 需 vulkan feature,与 M165 门构建面同字面)。
    rb = subprocess.run(
        ["cargo", "build", "--release", "-p", "rurix-asset", "--features", "vulkan",
         "--bin", "g12_4_ue_pt_parity_render"],
        cwd=ROOT, timeout=3600, capture_output=True, text=True,
    )
    bench_bin = gl.target_dir() / "release" / "g12_4_ue_pt_parity_render.exe"
    if rb.returncode != 0 or not bench_bin.is_file():
        tail = ((rb.stderr or "") + (rb.stdout or "")).strip()[-600:]
        problems.append(f"soak 前置构建失败(release g12_4_ue_pt_parity_render): rc={rb.returncode} {tail}")
        return False, {"ok": False, "detail": f"problems={problems}", "raw": {}, "counters": counters,
                       "seconds": 0.0, "active_chain_seconds": 0.0, "outer_elapsed": 0.0}
    with gl.gpu_device_lock(purpose="g12.7a chain-soak"):
        rurixc = gl.build_rurixc()
        spv = SOAK_WORK / "g12_pt_production.spv"
        dn_spv = SOAK_WORK / "g12_pt_denoise.spv"
        ok = rurixc is not None and gl.compile_spv(rurixc, spv)
        if ok:
            r = gl.run([str(rurixc), str(DENOISE_KERNEL), "--target", "vulkan", "-o", str(dn_spv)])
            ok = r.returncode == 0 and dn_spv.is_file()
        harness = gl.build_harness() if ok else None
        cal = load_denoise_calibration_from_budget() if harness else None
        if not (ok and harness and cal):
            problems.append("soak 前置产线失败(rurixc/SPV/降噪 harness/标定面)")
            return False, {"ok": False, "detail": f"problems={problems}", "raw": {}, "counters": counters,
                           "seconds": 0.0, "active_chain_seconds": 0.0, "outer_elapsed": 0.0}
        ctx = {
            "bench_bin": bench_bin,
            "harness": harness,
            "spv": spv,
            "dn_spv": dn_spv,
            "cal": cal,
            "tau": float(cal["tau"]),
            "work": SOAK_WORK,
        }
        SOAK_WORK.mkdir(parents=True, exist_ok=True)
        t0 = time.time()
        active = 0.0
        while time.time() - t0 < min_seconds and not problems:
            it0 = time.time()
            it_problems = _soak_iteration(ctx, counters["iterations"])
            active += time.time() - it0
            counters["iterations"] += 1
            if it_problems:
                counters["failures"] += 1
                problems.extend(it_problems)
                break
            counters["pt_frames_rendered"] += 2
            counters["denoise_pipeline_runs"] += 1
            counters["gap_registry_assemblies"] += 1
            counters["throughput_baseline_reruns"] += 1
            counters["throughput_frames_timed"] += BENCH_TIMED
            print(
                f"[7a] soak iter {counters['iterations']} ok "
                f"(elapsed {time.time() - t0:.1f}s/{min_seconds}s)",
                flush=True,
            )
    seconds = time.time() - t0
    raw = {
        "soak_subject": "chain-soak",
        "iterations": counters["iterations"],
        "seconds": seconds,
        "active_chain_seconds": active,
        "sleep_seconds": 0.0,
        "failures": counters["failures"],
        **{k: v for k, v in counters.items() if k not in ("iterations", "failures")},
    }
    ok, judge_problems = judge_chain_soak(
        raw, outer_elapsed=None, min_seconds=min_seconds, min_iterations=min_iterations
    )
    problems.extend(judge_problems)
    detail = (
        f"iterations={counters['iterations']} seconds={seconds:.1f} active={active:.1f} "
        f"sleep=0.0 failures={counters['failures']} subject='chain-soak'"
    )
    if problems:
        detail += f" problems={problems[:6]}"
    return ok and not problems, {
        "ok": ok and not problems,
        "iterations": counters["iterations"],
        "seconds": seconds,
        "active_chain_seconds": active,
        "detail": detail,
        "raw": raw,
        "counters": counters,
    }


def run_budget_strict() -> tuple[bool, str]:
    """budget_eval --strict:非空、零 estimated/skip(exit 0 且 PASS 且 0 skip)。"""
    r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "budget_eval.py"), "--strict"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    text = (r.stdout or "") + (r.stderr or "")
    m = re.search(r"\[budget_eval\] PASS \((\d+) pass, (\d+) skip, strict mode\)", text)
    ok = r.returncode == 0 and m is not None and int(m.group(1)) >= 1 and int(m.group(2)) == 0
    tail = (text.strip().splitlines() or [""])[-1]
    return ok, f"exit={r.returncode}; {tail}"


def run_legacy_0byte() -> tuple[bool, str]:
    """G5~G11 既有判据 0-byte(git status --porcelain 闭集面空集——与 M164 门 ③
    同闭集字面;异己 src/ 未提交面属立项裁决 1 登记面,不在本闭集)。"""
    r = subprocess.run(
        ["git", "status", "--porcelain", "--",
         "ci/g9_*.py", "ci/g10_*.py", "ci/g11_*.py",
         "milestones/g9", "milestones/g10", "milestones/g11", "spec", "conformance"],
        cwd=ROOT, capture_output=True, text=True,
    )
    dirty = [l for l in (r.stdout or "").splitlines() if l.strip()]
    ok = r.returncode == 0 and not dirty
    return ok, (f"exit={r.returncode}; 闭集面空集" if ok else f"dirty={dirty[:5]}")


def verify_latest() -> int:
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print("[7a] FAIL: missing soak evidence", file=sys.stderr)
        return 1
    data = wel.load_json(path)
    errs = wel.validate_schema(data, SCHEMA_PATH) if SCHEMA_PATH.is_file() else []
    if errs:
        print(f"[7a] schema FAIL: {errs}", file=sys.stderr)
        return 1
    if data.get("host_section_pass") is not True:
        print("[7a] FAIL: host_section_pass≠true", file=sys.stderr)
        return 1
    checks = data.get("checks") or {}
    need = [
        "regression_all_pass",
        "base_commit_uniform",
        "soak_dual_threshold",
        "soak_no_sleep_padding",
        "budget_strict_pass",
        "legacy_criteria_0byte",
        "date_anchor_recorded",
    ]
    bad = [k for k in need if checks.get(k) is not True]
    if bad:
        print(f"[7a] FAIL checks: {bad}", file=sys.stderr)
        return 1
    soak = data.get("soak") or {}
    ok, problems = judge_chain_soak(
        soak, outer_elapsed=None, min_seconds=MIN_SECONDS, min_iterations=MIN_ITERATIONS
    )
    if not ok:
        print(f"[7a] FAIL soak honesty: {problems}", file=sys.stderr)
        return 1
    print(f"[7a] verify-latest PASS(honest chain soak)← {path.relative_to(ROOT)}")
    return 0


def run_full_gate() -> int:
    if NUMERIC_STEP <= 0:
        print("[7a] NUMERIC_STEP unset (回填前草稿 → 红)", file=sys.stderr)
        return 1
    if not SCHEMA_PATH.is_file():
        print(f"[7a] schema missing: {SCHEMA_PATH}", file=sys.stderr)
        return 1

    reg_ok, reg_rows, commit, base_uniform = run_regression(skip_rerun=False)
    t0 = time.time()
    soak_ok, soak_info = run_chain_soak()
    outer_elapsed = time.time() - t0
    # 外测墙钟交叉核验(谎报时长判红)。
    if soak_info.get("seconds") and outer_elapsed + 2.0 < float(soak_info["seconds"]):
        soak_ok = False
        soak_info["detail"] += (
            f" 外测墙钟 {outer_elapsed:.1f}s < 自称 seconds={soak_info['seconds']:.1f}s(谎报)"
        )
    bud_ok, bud_detail = run_budget_strict()
    leg_ok, leg_detail = run_legacy_0byte()
    stamp = wel.utc_stamp()
    utc_date = stamp[:8]

    overall = reg_ok and base_uniform and soak_ok and bud_ok and leg_ok
    raw_soak = soak_info.get("raw") or {}
    no_sleep_ok, _ = judge_chain_soak(
        raw_soak, outer_elapsed=None, min_seconds=0, min_iterations=0
    )
    facts = [
        _fact(
            "regression_8p0_1p1_5wave",
            reg_ok,
            f"gates={len(reg_rows)} base_commit={commit}",
        ),
        _fact(
            "base_commit_uniform",
            base_uniform,
            "9 门 evidence base_commit 同值且=HEAD(同一候选 close-out 基线,沿 G11.7a/G10.8a MAP §7 口径)",
        ),
        _fact("soak_dual_threshold", soak_ok, soak_info["detail"]),
        _fact("budget_strict", bud_ok, bud_detail),
        _fact("legacy_criteria_0byte", leg_ok, leg_detail),
        _fact("date_anchor", True, f"utc_date={utc_date}"),
    ]
    # PT 生产化链路 soak 的 device 面由 ①回归腿 9 门本体承载;链复跑 device 腿
    # (出图/降噪/吞吐)为生产化链路稳定性面,validation/device_lost 字面量 0
    # 不作门亦不写实(沿 G11.7a/G10.8a/G9.8a 语义)。
    counters = soak_info.get("counters") or {}
    soak_block = {
        "iterations": int(soak_info.get("iterations") or 0),
        "seconds": float(soak_info.get("seconds") or 0.0),
        "min_iterations": MIN_ITERATIONS,
        "min_seconds": MIN_SECONDS,
        "soak_subject": "chain-soak",
        "active_chain_seconds": float(soak_info.get("active_chain_seconds") or 0.0),
        "sleep_seconds": 0.0,
        "outer_wall_seconds": round(float(outer_elapsed), 3),
        "failures": int(counters.get("failures") or 0),
        "pt_frames_rendered": int(counters.get("pt_frames_rendered") or 0),
        "denoise_pipeline_runs": int(counters.get("denoise_pipeline_runs") or 0),
        "gap_registry_assemblies": int(counters.get("gap_registry_assemblies") or 0),
        "throughput_baseline_reruns": int(counters.get("throughput_baseline_reruns") or 0),
        "throughput_frames_timed": int(counters.get("throughput_frames_timed") or 0),
    }
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
            "base_commit_uniform": base_uniform,
            "soak_dual_threshold": soak_ok,
            "soak_no_sleep_padding": no_sleep_ok,
            "budget_strict_pass": bud_ok,
            "legacy_criteria_0byte": leg_ok,
            "date_anchor_recorded": True,
        },
        "soak": soak_block,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "G12.7a full soak; four legs + G5~G11 既有判据 0-byte fact; honest semantics(沿 "
            "G11.7a/G10.8a/G9.8a/G8.8a 2026-08-08 口径): soak 墙钟=真实 PT 生产化链路复跑实测"
            "(禁 sleep 充时,sleep_seconds 恒 0,active_chain_seconds 逐迭代计时求和,gate 外测墙钟"
            "交叉核验);soak 载体=PT 生产化链路四面(出图 g12_4_ue_pt_parity_render --render 双场景"
            " spp16 receipt digest == M163 Rurix 臂冻结锚 → 降噪 g12_pt_production --gate "
            "g12.p0.m162.denoise_pipeline_tsr 双 kernel 全档真跑 → 对标装配 最新 M163 evidence "
            "parity 节重建 metrics 重装配与在树 g12_ue_pt_gap_registry.json 幂等复核"
            "(evidence_digest 自证指针 == metrics 重算 digest) → 吞吐基线 --benchmark 轻量链复跑"
            "(warmup 4 + timed 8 场景轮转,首帧 digest == 冻结锚 + distinct==1 + timed_count==8——"
            "链复跑口径如实登记,不冒充 M165 冻结 50×3 协议;吞吐守护阈归 M165 门本体〔①回归腿"
            " 50×3 全协议 ×1.5/÷1.5 守护〕,轻量腿帧时不充守护判定));subject=chain-soak 无 "
            "validation/device_lost 字面量 0 硬门(device 面归回归腿 9 门本体);9 门 evidence "
            "base_commit 同值一致(同一候选 close-out 基线)"
            + (
                f";轻量腿帧时信息面(notes 登记不充判据): {_BENCH_LAST_TIMES}"
                if _BENCH_LAST_TIMES
                else ""
            )
        ),
    }
    errs = wel.validate_schema(payload, SCHEMA_PATH)
    if errs:
        print(f"[7a] schema errors: {errs}", file=sys.stderr)
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


def selftest() -> int:
    """反假绿臂:谎报/sleep 充时/迭代不足/计数面空/缺 honesty 字段必红 + 诚实样本绿。"""
    arms: list[tuple[str, bool]] = []

    def _honest() -> dict:
        return {
            "soak_subject": "chain-soak",
            "iterations": 24,
            "seconds": 1830.5,
            "active_chain_seconds": 1830.1,
            "sleep_seconds": 0.0,
            "failures": 0,
            "pt_frames_rendered": 48,
            "denoise_pipeline_runs": 24,
            "gap_registry_assemblies": 24,
            "throughput_baseline_reruns": 24,
            "throughput_frames_timed": 96,
        }

    # A1 sleep 充墙钟:sleep_seconds>0 → 红。
    d = _honest()
    d["sleep_seconds"] = 900.0
    ok, probs = judge_chain_soak(d, outer_elapsed=1832.0, min_seconds=1800, min_iterations=3)
    arms.append(("A1 sleep_seconds>0(sleep充墙钟)→红", not ok))
    print(f"[selftest] A1 judge ok={ok} problems={probs}")

    # A2 外测墙钟戳穿谎报:自称 1900s 但外测只有 30s → 红。
    d = _honest()
    d["seconds"] = 1900.0
    d["active_chain_seconds"] = 1900.0
    ok, probs = judge_chain_soak(d, outer_elapsed=30.0, min_seconds=1800, min_iterations=3)
    arms.append(("A2 外测墙钟30s<自称1900s(谎报)→红", not ok))
    print(f"[selftest] A2 judge ok={ok} problems={probs}")

    # A3 迭代数不足:2 轮 < 3 → 红。
    d = _honest()
    d["iterations"] = 2
    ok, probs = judge_chain_soak(d, outer_elapsed=1832.0, min_seconds=1800, min_iterations=3)
    arms.append(("A3 iterations=2<3(迭代不足)→红", not ok))
    print(f"[selftest] A3 judge ok={ok} problems={probs}")

    # A4 计数面空/有失败:failures=1 且 pt_frames_rendered=0 → 红。
    d = _honest()
    d["failures"] = 1
    d["pt_frames_rendered"] = 0
    ok, probs = judge_chain_soak(d, outer_elapsed=1832.0, min_seconds=1800, min_iterations=3)
    arms.append(("A4 failures≠0/计数面空→红", not ok))
    print(f"[selftest] A4 judge ok={ok} problems={probs}")

    # A5 缺 honesty 字段(无 soak_subject/sleep_seconds/active)→ 红。
    d = _honest()
    del d["soak_subject"]
    del d["sleep_seconds"]
    del d["active_chain_seconds"]
    ok, probs = judge_chain_soak(d, outer_elapsed=1832.0, min_seconds=1800, min_iterations=3)
    arms.append(("A5 缺 honesty 字段→红", not ok))
    print(f"[selftest] A5 judge ok={ok} problems={probs}")

    # A6 诚实样本:24 轮/1830.5s/sleep=0/计数非空/零失败 → 绿(正臂)。
    ok, probs = judge_chain_soak(_honest(), outer_elapsed=1832.0, min_seconds=1800, min_iterations=3)
    arms.append(("A6 诚实 chain soak(24轮/1830.5s/sleep=0/计数非空)→绿", ok))
    print(f"[selftest] A6 judge ok={ok} problems={probs}")

    failed = [name for name, good in arms if not good]
    for name, good in arms:
        print(f"[selftest] {'PASS' if good else 'FAIL'}  {name}")
    if failed:
        print(f"[selftest] FAIL arms: {failed}", file=sys.stderr)
        return 1
    print("[selftest] PASS: 反假绿臂全部符合预期(5 红臂 + 1 绿臂)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G12.7a stabilization soak")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        if NUMERIC_STEP <= 0:
            print("[7a] selftest: NUMERIC_STEP=0 draft → expect red on --gate")
            code = run_full_gate()
            if code == 0:
                print("[selftest] FAIL: draft still green", file=sys.stderr)
                return 1
            print("[selftest] PASS: draft NUMERIC_STEP=0 → red")
            return 0
        return selftest()
    if args.verify_latest:
        return verify_latest()
    return run_full_gate()


if __name__ == "__main__":
    sys.exit(main())
