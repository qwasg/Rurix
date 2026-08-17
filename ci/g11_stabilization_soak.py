#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.6/G11.7 收口波）
"""G11.7a stabilization soak 聚合门 g11.wave.7a.soak(G11_CONTRACT G-G11-9;
G11_PLAN §2 G11.7a;CI_GATES §5 wave7a 行;同构 ci/g10_stabilization_soak.py)。

四腿:①全量回归(13 P0 + 1 go P1 = 14 key 逐门真跑 --gate——M147 走 --phase g11.3
登记面腿并叠加双 phase 机核:最新 evidence phase==g11.3 且顶层 status=="pass",
且最新 g11.5 phase 件 closure.verdict=="converged"〔definitive 收敛断言面不遮蔽,
契约 §8.3a〕;wave2~wave5 exit + wave6 decisions 五聚合/决策门真跑核验;14 门
evidence base_commit 同值=HEAD 且 19 门 evidence 文件名 UTC stamp ≥ run 起点 =
同一候选 close-out 基线,沿 G10.8a MAP §7 口径)+ ②修复链路(复测出图/度量/差距
清单装配)连续复跑 soak(≥1800s 墙钟沿 G10.8a/G9.8a 继承;真实链路逐迭代连续
复跑,迭代计数与各环节计数非空、零失败;sleep_seconds 恒 0,active_chain_seconds
≈seconds,gate 外测墙钟交叉核验,谎报判红)+ ③budget_eval --strict 非空零
estimated/skip + ④纪律日期锚;G-G11-9 另含「G5~G10 既有判据 0-byte」独立 fact
(git status --porcelain 闭集面空集,与 M156 门 ⑤ 同闭集字面)。

诚实语义(沿 G10.8a/G9.8a/G8.8a 2026-08-08 清零假绿后口径,G11 无 legacy 兼容):
- soak 墙钟=真实链路复跑实测(active_chain_seconds 逐迭代计时求和),迭代间
  零 sleep(sleep_seconds 恒 0);gate 侧用外测墙钟交叉核验,谎报 seconds 判红。
- soak 载体 = G11.5b 建立的修复复测链(与 M155 门数据面同构,复用
  ci/g11_5_retest_lib.py 单一事实源常量与函数 + milestones/g11/harness/
  g11_5b_ab_rerun.py 渲染旗标面/清单装配面):Rurix HDR release 全修复旗标面
  (--material-pbr/--light-seed-set/--gi-multibounce/--sky-ibl 等,双场景同消费)
  重渲染 digest 逐位复现 g11_5b 帧区库帧(复测出图)→ LDR 派生双臂 ×1.0 逐字节
  复现 + 双端帧解码(捕获面对账)→ 11 行复测 delta 门侧独立重算 ==
  g11_5b_rerun_report.json closure_faces 登记值逐位(度量)→ 复测差距清单
  stage_registry 当次重装配与在树 g11_5b_retest_gap_registry.json 幂等复核
  (rerun_report_digest 自证指针字段外逐字段全等——G11.5b 落盘时序既存差异
  如实登记,重装配值==当前报告 digest 机核,装配后在树回写 0-byte 恢复)
  (差距清单装配)。host CPU 参考管线链,无 Vulkan device 面,不以字面
  量 0 充 device 零错门;UE 帧为帧区只读解码面(不重复 MRQ,与 G10.8a 同口径)。
- 迭代计数/出图帧数/派生数/解码帧数/11 行度量重算/清单装配计数非空(空即红),
  failures 恒 0;evidence soak 块 seconds 与 iterations 双字段机器可核。

pr-smoke 默认 --verify-latest(秒级核最新 full-run evidence);
本地/workflow_dispatch 用 --gate 产 full-run。

用法:
  py -3 ci/g11_stabilization_soak.py --gate g11.wave.7a.soak
  py -3 ci/g11_stabilization_soak.py --verify-latest
  py -3 ci/g11_stabilization_soak.py --selftest
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
import g11_5_retest_lib as rl  # noqa: E402

ROOT = wel.ROOT
sys.path.insert(0, str(ROOT / "milestones" / "g11" / "harness"))
import g11_5b_ab_rerun as hr  # noqa: E402

GATE_KEY = "g11.wave.7a.soak"
NUMERIC_STEP = 215  # 落盘前实测 registry/number_ledger.json CI_step.next_free=215 顺位领取
SUBJECT = "g11_stabilization_soak"
WAVE = "G11.7a"
SOURCE_REF = (
    "G11_CONTRACT G-G11-9;G11_PLAN §2 G11.7a;CI_GATES §5 wave7a;"
    "13 P0 + 1 go P1 全量回归(M147 --phase g11.3 登记面腿 + 双 phase 机核)+ "
    "wave2~wave5 exit + wave6 decisions 聚合/决策门 + 修复链路(复测出图/度量/"
    "差距清单装配)连续复跑 soak ≥1800s + budget --strict + G5~G10 既有判据 0-byte"
)
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_stabilization_soak_evidence_schema.json"

# (symbolic_key, subject_prefix, smoke argv relative, dual_phase_m147)
# 顺序即执行序:M147(--phase g11.3 登记面腿,双 phase 机核)按其 key 序在位;
# 波聚合/决策门最后(只读汇总 19 门最新 evidence)。G11 门 smoke 各自管理 device
# 腿/gpu_device_lock(pr-smoke 体例,聚合门不设 RURIX_REQUIRE_REAL)。
REGRESSION_GATES: list[tuple[str, str, list[str], bool]] = [
    # ── G11.2 波(步骤 196~199)──
    ("g11.p0.m144.caliber_c1_indoor_luminance", "g11_m144_caliber_c1_indoor_luminance",
     ["ci/g11_caliber_c1_indoor_luminance_smoke.py", "--gate", "g11.p0.m144.caliber_c1_indoor_luminance"], False),
    ("g11.p0.m145.caliber_c2_exposure_chain", "g11_m145_caliber_c2_exposure_chain",
     ["ci/g11_caliber_c2_exposure_chain_smoke.py", "--gate", "g11.p0.m145.caliber_c2_exposure_chain"], False),
    ("g11.p0.m146.caliber_c3_exr_bit_depth", "g11_m146_caliber_c3_exr_bit_depth",
     ["ci/g11_caliber_c3_exr_bit_depth_smoke.py", "--gate", "g11.p0.m146.caliber_c3_exr_bit_depth"], False),
    ("g11.p1.m157.hdr_flip_calibration", "g11_m157_hdr_flip_calibration",
     ["ci/g11_hdr_flip_calibration_smoke.py", "--gate", "g11.p1.m157.hdr_flip_calibration"], False),
    # ── G11.3 波(步骤 201~206;M147 双 phase 登记面腿)──
    ("g11.p0.m147.fix_r1_material_subset", "g11_m147_fix_r1_material_subset",
     ["ci/g11_fix_r1_material_subset_smoke.py", "--gate", "g11.p0.m147.fix_r1_material_subset",
      "--phase", "g11.3"], True),
    ("g11.p0.m148.fix_r2_geometry_normals", "g11_m148_fix_r2_geometry_normals",
     ["ci/g11_fix_r2_geometry_normals_smoke.py", "--gate", "g11.p0.m148.fix_r2_geometry_normals"], False),
    ("g11.p0.m149.fix_r5_json_u64_seed", "g11_m149_fix_r5_json_u64_seed",
     ["ci/g11_fix_r5_json_u64_seed_smoke.py", "--gate", "g11.p0.m149.fix_r5_json_u64_seed"], False),
    ("g11.p0.m150.fix_u1_cornell_shell_radiance", "g11_m150_fix_u1_cornell_shell_radiance",
     ["ci/g11_fix_u1_cornell_shell_radiance_smoke.py", "--gate", "g11.p0.m150.fix_u1_cornell_shell_radiance"], False),
    ("g11.p0.m151.fix_u2_bistro_texture_dds", "g11_m151_fix_u2_bistro_texture_dds",
     ["ci/g11_fix_u2_bistro_texture_dds_smoke.py", "--gate", "g11.p0.m151.fix_u2_bistro_texture_dds"], False),
    ("g11.p0.m152.fix_u3_bistro_animation", "g11_m152_fix_u3_bistro_animation",
     ["ci/g11_fix_u3_bistro_animation_smoke.py", "--gate", "g11.p0.m152.fix_u3_bistro_animation"], False),
    # ── G11.4 波(步骤 208~209)──
    ("g11.p0.m153.fix_r3_light_subset", "g11_m153_fix_r3_light_subset",
     ["ci/g11_fix_r3_light_subset_smoke.py", "--gate", "g11.p0.m153.fix_r3_light_subset"], False),
    ("g11.p0.m154.fix_r4_gi_multibounce_world_cache", "g11_m154_fix_r4_gi_multibounce_world_cache",
     ["ci/g11_fix_r4_gi_multibounce_world_cache_smoke.py", "--gate",
      "g11.p0.m154.fix_r4_gi_multibounce_world_cache"], False),
    # ── G11.5 波(步骤 211~212)──
    ("g11.p0.m155.ab_retest_closure", "g11_m155_ab_retest_closure",
     ["ci/g11_ab_retest_closure_smoke.py", "--gate", "g11.p0.m155.ab_retest_closure"], False),
    ("g11.p0.m156.regression_guard", "g11_m156_regression_guard",
     ["ci/g11_regression_guard_smoke.py", "--gate", "g11.p0.m156.regression_guard"], False),
    # ── 波聚合/决策门(步骤 200/207/210/213/214;只读汇总不重跑子门 smoke)──
    ("g11.wave.2.exit", "g11_wave2_exit",
     ["ci/g11_wave2_exit_check.py", "--gate", "g11.wave.2.exit"], False),
    ("g11.wave.3.exit", "g11_wave3_exit",
     ["ci/g11_wave3_exit_check.py", "--gate", "g11.wave.3.exit"], False),
    ("g11.wave.4.exit", "g11_wave4_exit",
     ["ci/g11_wave4_exit_check.py", "--gate", "g11.wave.4.exit"], False),
    ("g11.wave.5.exit", "g11_wave5_exit",
     ["ci/g11_wave5_exit_check.py", "--gate", "g11.wave.5.exit"], False),
    ("g11.wave.6.decisions", "g11_p2_decisions",
     ["ci/g11_p2_decisions_check.py", "--gate", "g11.wave.6.decisions"], False),
]

MIN_SECONDS = 1800  # 沿 G10.8a/G9.8a 继承(≥30min;G-G11-9「或 measured 证明更短足够」未触)
MIN_ITERATIONS = 3  # 全链路迭代计数非空下界
# 14 门(13 P0 + 1 go P1)为门 evidence(顶层 status=="pass" 字面 + base_commit);
# 后 5 门为波聚合/决策 evidence(wel 口径,无顶层 status/base_commit 字段)。
N_ASSERTION_GATES = 14

SOAK_REPRO_DIR = rl.FRAMES_G11_5 / "soak_repro"
QUALITY_ROWS = ("R1", "R2", "R3", "R4", "R5", "U1", "U2", "U3")
ALL_ROWS = QUALITY_ROWS + ("C1", "C2", "C3")
# 单轮 EXR 解码精确计数口径:出图产物 decode 2 + lib rurix HDR 内容 digest 对账
# decode 2 + R1 ssim_ldr 2 + R2/U1 coverage_delta 各 2(共 4)+ R3/R4 hdr_lum 各 2
# (共 4)+ U2 ldr_lum 2 = 16(LDR 派生比对走 read_bytes 不计 EXR 解码;R5/U3/C 族
# 行自报告闭集块重算零解码)。
FRAMES_DECODED_PER_ITER = 16


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def base_commit() -> str:
    r = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True,
    )
    return (r.stdout or "").strip() or "unknown"


def _m147_g11_5_latest() -> Path | None:
    """M147 最新 phase==g11.5 evidence(definitive 收敛断言面)。"""
    best: tuple[str, Path] | None = None
    for p in wel.EVIDENCE_DIR.glob("g11_m147_fix_r1_material_subset_*.json"):
        m = re.search(r"_(\d{8}T\d{6}Z)\.json$", p.name)
        if m is None:
            continue
        try:
            doc = wel.load_json(p)
        except (OSError, json.JSONDecodeError):
            continue
        if doc.get("phase") != "g11.5":
            continue
        if best is None or m.group(1) > best[0]:
            best = (m.group(1), p)
    return best[1] if best else None


def verify_assertion_gate(key: str, prefix: str, dual_phase_m147: bool) -> dict:
    """14 门(13 P0 + 1 go P1)最新 evidence 机器核验。

    在 wel.require_gate_pass 口径(symbolic_gate_key/host_section_pass/
    device_section_state/checks 全真)之上叠加:
    - 顶层 status=="pass" 字面(MAP §1 evidence 必备字段,skip/estimated 不充绿;
      G11 证据形态统一,无 G9 M90/M91 缺字段豁免面——缺字段即红);
    - dual_phase_m147=True(M147)按契约 §8.3a 双 phase 口径叠加:最新 evidence
      phase=="g11.3"(登记面腿当次真跑)且最新 g11.5 phase 件
      closure.verdict=="converged"(definitive 收敛断言面不遮蔽——g11.3 登记面绿
      不替 g11.5 收敛断言充绿)。
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
        problems.append("缺顶层 status 字段(G11 证据形态统一,无豁免面)")
    if dual_phase_m147:
        if doc.get("phase") != "g11.3":
            problems.append(f"最新 evidence phase={doc.get('phase')!r} ≠ 'g11.3'(登记面腿未当次真跑)")
        p5 = _m147_g11_5_latest()
        if p5 is None:
            problems.append("缺 phase==g11.5 evidence(definitive 收敛断言面)")
        else:
            try:
                d5 = wel.load_json(p5)
            except (OSError, json.JSONDecodeError):
                d5 = {}
            verdict = (d5.get("closure") or {}).get("verdict")
            if verdict != "converged":
                problems.append(
                    f"最新 g11.5 phase 件 verdict={verdict!r} ≠ 'converged'"
                    "(g11.3 登记面绿不替 g11.5 收敛断言充绿)"
                )
    if problems:
        row["status"] = "FAIL"
        row["detail"] = f"{row.get('detail', '')}; " + "; ".join(problems)
    return row


def run_regression(*, skip_rerun: bool = False) -> tuple[bool, list[dict], str, bool]:
    """全量回归(14 门 + 5 波聚合/决策门)。口径沿 G10.8a run_regression 同构。"""
    rows: list[dict] = []
    commit = base_commit()
    run_start_stamp = wel.utc_stamp()
    all_ok = True
    bases: list[str] = []
    no_base_field: list[str] = []
    stale: list[str] = []
    stamp_re = re.compile(r"_(\d{8}T\d{6}Z)\.json$")
    for idx, (key, prefix, argv, dual_phase_m147) in enumerate(REGRESSION_GATES):
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
            row = verify_assertion_gate(key, prefix, dual_phase_m147)
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
    """诚实判定修复链路 soak 输出。返回 (ok, problems)。口径沿 G10.8a 同构:
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
        "rurix_frames_rendered",
        "ldr_derivations",
        "frames_decoded",
        "metric_row_recomputes",
        "gap_registry_assemblies",
    ):
        if int(doc.get(k) or 0) < 1:
            problems.append(f"{k} 计数面空")
    return (not problems), problems


def _render_repro(scene_id: str, work: Path) -> str | None:
    """复测出图腿:Rurix HDR release 全修复旗标面重渲染,digest 逐位复现
    g11_5b 帧区库帧。返回问题串(None=绿)。"""
    s = hr.SCENES[scene_id]
    scale = 2.0 ** (-s["ev100"])
    out_dir = work / scene_id
    out_dir.mkdir(parents=True, exist_ok=True)
    argv = [
        str(hr.RUST_RELEASE_BIN), "--render",
        "--gltf", str(s["gltf"]),
        "--contract", str(hr.CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"),
        "--out-dir", str(out_dir),
        "--scene-id", scene_id,
        "--exposure-scale", repr(scale),
        *s["rurix_flags"],
    ]
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=3600)
    frame = out_dir / f"{scene_id}.exr"
    if r.returncode != 0 or not frame.is_file():
        return f"出图失败({scene_id}): exit={r.returncode}"
    d = rl.decode(frame, "rurix")
    repro = hr.frame_content_digest(d["width"], d["height"], 3, d["pixels"])
    lib = rl.decode(rl.hdr_frame(scene_id, "rurix"), "rurix")
    want = hr.frame_content_digest(lib["width"], lib["height"], 3, lib["pixels"])
    if repro != want:
        return f"出图未逐位复现({scene_id}): {repro} ≠ {want}(g11_5b 库帧)"
    return None


def _ldr_repro(scene_id: str, end: str, work: Path) -> str | None:
    """LDR 派生腿:自 g11_5b 库 HDR 双臂 ×1.0 派生,逐字节复现库 LDR 帧。"""
    hdr = rl.hdr_frame(scene_id, end)
    lib_ldr = rl.ldr_frame(scene_id, end)
    out_ldr = work / scene_id / f"{scene_id}_{end}_ldr.exr"
    argv = [
        str(hr.RUST_RELEASE_BIN), "--derive-ldr",
        "--hdr", str(hdr),
        "--source-end", end,
        "--out", str(out_ldr),
        "--exposure-scale", "1.0",
        "--params-digest", rl.LOCKED_DIGEST[scene_id].split(":", 1)[1],
    ]
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=1800)
    if r.returncode != 0 or not out_ldr.is_file():
        return f"LDR 派生失败({scene_id}/{end}): exit={r.returncode}"
    if out_ldr.read_bytes() != lib_ldr.read_bytes():
        return f"LDR 派生未逐字节复现({scene_id}/{end})"
    return None


def _metric_rows_recompute(report: dict) -> list[str]:
    """度量腿:11 行复测 delta 门侧独立重算 == g11_5b_rerun_report.json
    closure_faces 登记值逐位(未复跑冒充判红面,与 M155 门同判定层)。"""
    problems: list[str] = []
    faces = report["results"]["metrics"]["closure_faces"]
    for prefix in ALL_ROWS:
        got = rl.recompute_row_retest(prefix, report)
        if prefix == "C1":
            want = faces["c1"]["retest_bistro_median_delta"]
        else:
            want = faces[prefix.lower()]["retest_delta"]
        if got != want:
            problems.append(f"{prefix} 重算 {got!r} ≠ 报告登记 {want!r}(逐位不等)")
    return problems


def _registry_assembly() -> str | None:
    """差距清单装配腿:stage_registry 当次重装配,与在树幂等复核。

    口径:重装配产物与在树逐字段全等,**唯一豁免字段 = rerun_report_digest**
    (报告当前 digest 自证指针——G11.5b 落盘时序既存差异:在树清单登记值
    6fb18192… ≠ 当前在树报告 digest 7ea9eb62…,即清单装配面与报告最终落盘面的
    时序差;本波不重写 G11.5b 终态 0-byte,差异如实登记于 soak evidence notes,
    重装配值 == 当前报告 digest 机核)。装配后在树回写 0-byte 恢复。"""
    path = hr.RETEST_REGISTRY_PATH
    before_text = path.read_text(encoding="utf-8")
    before_doc = json.loads(before_text)
    hr.stage_registry()
    after_text = path.read_text(encoding="utf-8")
    after_doc = json.loads(after_text)
    # 自证指针字段机核:重装配值必须 == 当前报告文件 digest。
    want_report_digest = hr.sha256_file(hr.REPORT_PATH)
    problems: list[str] = []
    if after_doc.get("rerun_report_digest") != want_report_digest:
        problems.append(
            f"重装配 rerun_report_digest={after_doc.get('rerun_report_digest')} "
            f"≠ 当前报告 digest {want_report_digest}(自证指针失效)"
        )
    b = dict(before_doc)
    a = dict(after_doc)
    b.pop("rerun_report_digest", None)
    a.pop("rerun_report_digest", None)
    if b != a:
        problems.append("复测差距清单重装配与在树漂移(rerun_report_digest 自证指针字段外)")
    # 在树 0-byte 恢复(G11.5b 终态不重写)。
    path.write_text(before_text, encoding="utf-8", newline="\n")
    if problems:
        return "; ".join(problems)
    return None


def _soak_iteration(work: Path, report: dict) -> list[str]:
    """单轮修复链路复跑(复测出图→度量→差距清单装配)。fail-fast。"""
    for scene_id in hr.SCENES:
        p = _render_repro(scene_id, work)
        if p:
            return [p]
    for scene_id in hr.SCENES:
        for end in ("rurix", "ue5"):
            p = _ldr_repro(scene_id, end, work)
            if p:
                return [p]
    problems = _metric_rows_recompute(report)
    if problems:
        return problems
    p = _registry_assembly()
    if p:
        return [p]
    return []


def run_chain_soak(
    *,
    min_seconds: int = MIN_SECONDS,
    min_iterations: int = MIN_ITERATIONS,
) -> tuple[bool, dict]:
    """修复链路连续复跑 soak:≥min_seconds 墙钟内逐迭代真跑,零 sleep,零失败。"""
    counters = {
        "iterations": 0,
        "failures": 0,
        "rurix_frames_rendered": 0,
        "ldr_derivations": 0,
        "frames_decoded": 0,
        "metric_row_recomputes": 0,
        "gap_registry_assemblies": 0,
    }
    problems: list[str] = []
    # 构建一次(循环外;release 渲染器——与 G11.5b 复跑同 binary)。
    rb = subprocess.run(
        ["cargo", "build", "--release", "-p", "rurix-asset", "--bin", "g10_5_scene_render"],
        cwd=ROOT, timeout=3600,
    )
    if rb.returncode != 0 or not hr.RUST_RELEASE_BIN.is_file():
        problems.append("soak 前置构建失败(release g10_5_scene_render)")
        return False, {"ok": False, "detail": f"problems={problems}", "raw": {}, "counters": counters,
                       "seconds": 0.0, "active_chain_seconds": 0.0, "outer_elapsed": 0.0}
    SOAK_REPRO_DIR.mkdir(parents=True, exist_ok=True)
    try:
        report = rl.load_report()
        faces = report["results"]["metrics"]["closure_faces"]
        for row in ALL_ROWS:
            faces[row.lower()]
    except (OSError, json.JSONDecodeError, KeyError) as e:
        problems.append(f"g11_5b 复跑报告装载/闭集块缺行: {e}")
    t0 = time.time()
    active = 0.0
    while time.time() - t0 < min_seconds and not problems:
        it0 = time.time()
        it_problems = _soak_iteration(SOAK_REPRO_DIR, report)
        active += time.time() - it0
        counters["iterations"] += 1
        if it_problems:
            counters["failures"] += 1
            problems.extend(it_problems)
            break
        counters["rurix_frames_rendered"] += 2
        counters["ldr_derivations"] += 4
        counters["frames_decoded"] += FRAMES_DECODED_PER_ITER
        counters["metric_row_recomputes"] += len(ALL_ROWS)
        counters["gap_registry_assemblies"] += 1
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
    """G5~G10 既有判据 0-byte(git status --porcelain 闭集面空集——与 M156 门 ⑤
    同闭集字面;异己 src/ 未提交面属立项裁决 1 登记面,不在本闭集)。"""
    r = subprocess.run(
        ["git", "status", "--porcelain", "--", "ci/g9_*.py", "ci/g10_*.py",
         "milestones/g9", "milestones/g10", "spec", "conformance"],
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
            "regression_13p0_1p1_5wave",
            reg_ok,
            f"gates={len(reg_rows)} base_commit={commit}",
        ),
        _fact(
            "base_commit_uniform",
            base_uniform,
            "14 门 evidence base_commit 同值且=HEAD(同一候选 close-out 基线,沿 G10.8a MAP §7 口径)",
        ),
        _fact("soak_dual_threshold", soak_ok, soak_info["detail"]),
        _fact("budget_strict", bud_ok, bud_detail),
        _fact("legacy_criteria_0byte", leg_ok, leg_detail),
        _fact("date_anchor", True, f"utc_date={utc_date}"),
    ]
    # host 修复链路 soak 无 device 面,validation/device_lost 不作门亦不写实(沿
    # G10.8a/G9.8a 语义)。
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
        "rurix_frames_rendered": int(counters.get("rurix_frames_rendered") or 0),
        "ldr_derivations": int(counters.get("ldr_derivations") or 0),
        "frames_decoded": int(counters.get("frames_decoded") or 0),
        "metric_row_recomputes": int(counters.get("metric_row_recomputes") or 0),
        "gap_registry_assemblies": int(counters.get("gap_registry_assemblies") or 0),
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
            "G11.7a full soak; four legs + G5~G10 既有判据 0-byte fact; honest semantics(沿 "
            "G10.8a/G9.8a/G8.8a 2026-08-08 口径): soak 墙钟=真实修复链路复跑实测(禁 sleep 充时,"
            "sleep_seconds 恒 0,active_chain_seconds 逐迭代计时求和,gate 外测墙钟交叉核验);"
            "soak 载体=G11.5b 修复复测链(与 M155 门数据面同构:Rurix HDR release 全修复旗标面"
            "重渲染 digest 逐位复现 g11_5b 帧区库帧 + LDR 派生双臂 ×1.0 逐字节复现 + 11 行复测 "
            "delta 门侧独立重算==g11_5b_rerun_report.json 登记值逐位 + 复测差距清单 stage_registry "
            "当次重装配与在树幂等复核(rerun_report_digest 自证指针字段外逐字段全等——G11.5b "
            "落盘时序既存差异〔在树登记 6fb18192… vs 当前报告 7ea9eb62…〕如实登记,重装配值==当前 "
            "报告 digest 机核,装配后在树回写 0-byte 恢复));subject=chain-soak 无 device 零错字面量门;"
            "14 门 evidence base_commit 同值一致(同一候选 close-out 基线);M147 双 phase 机核"
            "(g11.3 登记面当次真跑 + 最新 g11.5 phase 件 verdict==converged,契约 §8.3a 不遮蔽)"
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
            "rurix_frames_rendered": 48,
            "ldr_derivations": 96,
            "frames_decoded": 384,
            "metric_row_recomputes": 264,
            "gap_registry_assemblies": 24,
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

    # A4 计数面空/有失败:failures=1 且 frames_decoded=0 → 红。
    d = _honest()
    d["failures"] = 1
    d["frames_decoded"] = 0
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
    ap = argparse.ArgumentParser(description="G11.7a stabilization soak")
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
