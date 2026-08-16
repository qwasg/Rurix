#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.8 收口波）
"""G10.8a soak 聚合门 g10.wave.8a.soak(G10_CONTRACT G-G10-10;G10_PLAN §G10.8a;
G10_ACCEPTANCE_MAP §7;CI_GATES §5 wave8a 行;同构 ci/g9_stabilization_soak.py)。

四腿:①全量回归(12 P0 + 2 go P1 = 14 key 逐门真跑 --gate——M130 走
--phase g10.5 双端核验腿并核验 phase_g10_2_pass==true 且 phase_g10_5_pass==true,
骨架期绿不替双端核验期充绿;门序 M130 先于 M139——后机器核验最新 evidence
顶层 status=="pass"〔G10 证据形态统一,无 G9 M90/M91 缺字段豁免面〕+ wave2~wave5
exit + wave6 重评窗 + wave7 决策六聚合/决策门真跑核验;14 门 evidence base_commit
同值=HEAD 且 20 门 evidence 文件名 UTC stamp ≥ run 起点 = 同一候选 close-out
基线,MAP §7)→ ②出图→捕获→度量→差距清单全链路连续复跑 soak(≥1800s 墙钟,
沿 G9.8a 30min 继承;真实链路逐迭代连续复跑,迭代计数与各环节计数非空、零失败;
sleep_seconds 恒 0,active_chain_seconds≈soak_seconds,gate 外测墙钟交叉核验,
谎报判红)→ ③budget_eval --strict 非空零 estimated/skip → ④纪律日期锚。

诚实语义(沿 G9.8a/G8.8a 2026-08-08 清零假绿后口径,G10 无 legacy 兼容):
- soak 墙钟=真实链路复跑实测(active_chain_seconds 逐迭代计时求和),迭代间
  零 sleep(sleep_seconds 恒 0);gate 侧用外测墙钟交叉核验,谎报 seconds 判红。
- soak 载体 = G10.5 建立的 A/B 数据链(与 M139 门数据面同构,复用
  ci/g10_ab_comparison_smoke.py 单一事实源常量与函数):Rurix HDR release 重渲染
  双场景 digest 逐位复现库帧(出图)→ LDR 派生四臂逐字节复现 + 双端四组帧
  解码 + UE 帧 unreal/build == M128 登记 ue_build_id + 内容 digest == 注册常量
  (捕获)→ LDR 臂 FLIP/SSIM/PSNR + 亮度统计重算 == G10.5a golden 逐位(度量)
  → diff 报告重跑独立重算三面一致 → 探针 artifact 再生 → 差距清单 11 项装配 +
  gaplib 校验零错误 + 与在树 g10_gap_registry.json 逐字节相等幂等复核(差距清单)。
  host CPU 参考管线链,无 Vulkan device 面,不以字面量 0 充 device 零错门。
- 迭代计数/出图帧数/解码帧数/度量三元组/diff 报告/清单装配计数非空(空即红),
  failures 恒 0;evidence soak 块 seconds 与 iterations 双字段机器可核。

pr-smoke 默认 --verify-latest(秒级核最新 full-run evidence);
本地/workflow_dispatch 用 --gate 产 full-run。

用法:
  py -3 ci/g10_stabilization_soak.py --gate g10.wave.8a.soak
  py -3 ci/g10_stabilization_soak.py --verify-latest
  py -3 ci/g10_stabilization_soak.py --selftest
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
import g10_wave_exit_lib as wel  # noqa: E402
import g10_ab_comparison_smoke as m139  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g10.wave.8a.soak"
NUMERIC_STEP = 194
SUBJECT = "g10_stabilization_soak"
WAVE = "G10.8a"
SOURCE_REF = (
    "G10_CONTRACT G-G10-10;G10_PLAN §G10.8a;G10_ACCEPTANCE_MAP §7;CI_GATES §5 wave8a;"
    "12 P0 + 2 go P1 全量回归(M130 --phase g10.5 先于 M139)+ wave2~wave7 聚合/决策门 + "
    "出图→捕获→度量→差距清单全链路 soak ≥1800s + budget --strict"
)
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_stabilization_soak_evidence_schema.json"

# (symbolic_key, subject_prefix, smoke argv relative, dual_phase_full)
# 顺序即执行序:门序 M130(--phase g10.5 双端核验腿)先于 M139(G-G10-7 门序硬约束,
# M139 门内当次 session 亦真跑 M130 g10.5 腿);波聚合/决策门最后(只读汇总 20 门
# 最新 evidence)。G10 门 smoke 各自管理 device 腿/gpu_device_lock(pr-smoke 体例,
# 聚合门不设 RURIX_REQUIRE_REAL)。
REGRESSION_GATES: list[tuple[str, str, list[str], bool]] = [
    # ── G10.2 波(步骤 177~179)──
    ("g10.p0.m128.ue5_capture_environment", "g10_m128_ue5_capture_environment",
     ["ci/g10_ue5_capture_environment_smoke.py", "--gate", "g10.p0.m128.ue5_capture_environment"], False),
    ("g10.p0.m129.ue5_reference_frames", "g10_m129_ue5_reference_frames",
     ["ci/g10_ue5_reference_frames_smoke.py", "--gate", "g10.p0.m129.ue5_reference_frames"], False),
    # ── G10.3 波(步骤 173~175)──
    ("g10.p0.m131.asset_license_registry", "g10_m131_asset_license_registry",
     ["ci/g10_asset_license_registry_smoke.py", "--gate", "g10.p0.m131.asset_license_registry"], False),
    ("g10.p0.m132.corpus_loading", "g10_m132_corpus_loading",
     ["ci/g10_corpus_loading_smoke.py", "--gate", "g10.p0.m132.corpus_loading"], False),
    ("g10.p1.m133.corpus_list_freeze", "g10_m133_corpus_list_freeze",
     ["ci/g10_corpus_list_freeze_smoke.py", "--gate", "g10.p1.m133.corpus_list_freeze"], False),
    # ── G10.4 波(步骤 181~185)──
    ("g10.p0.m134.frame_capture_pipeline", "g10_m134_frame_capture_pipeline",
     ["ci/g10_frame_capture_pipeline_smoke.py", "--gate", "g10.p0.m134.frame_capture_pipeline"], False),
    ("g10.p0.m135.flip_metric", "g10_m135_flip_metric",
     ["ci/g10_flip_metric_smoke.py", "--gate", "g10.p0.m135.flip_metric"], False),
    ("g10.p0.m136.ssim_psnr_metric", "g10_m136_ssim_psnr_metric",
     ["ci/g10_ssim_psnr_metric_smoke.py", "--gate", "g10.p0.m136.ssim_psnr_metric"], False),
    ("g10.p0.m137.pixel_diff_report", "g10_m137_pixel_diff_report",
     ["ci/g10_pixel_diff_report_smoke.py", "--gate", "g10.p0.m137.pixel_diff_report"], False),
    ("g10.p1.m138.metric_threshold_calibration", "g10_m138_metric_threshold_calibration",
     ["ci/g10_metric_threshold_calibration_smoke.py", "--gate",
      "g10.p1.m138.metric_threshold_calibration"], False),
    # ── M130 双端核验腿(步骤 187;门序先于 M139)──
    ("g10.p0.m130.dual_determinism_contract", "g10_m130_dual_determinism_contract",
     ["ci/g10_dual_determinism_contract_smoke.py", "--gate", "g10.p0.m130.dual_determinism_contract",
      "--phase", "g10.5"], True),
    # ── G10.5 波(步骤 188~190)──
    ("g10.p0.m139.ab_comparison", "g10_m139_ab_comparison",
     ["ci/g10_ab_comparison_smoke.py", "--gate", "g10.p0.m139.ab_comparison"], False),
    ("g10.p0.m140.gap_registry", "g10_m140_gap_registry",
     ["ci/g10_gap_registry_smoke.py", "--gate", "g10.p0.m140.gap_registry"], False),
    ("g10.p0.m141.perf_baseline", "g10_m141_perf_baseline",
     ["ci/g10_perf_baseline_smoke.py", "--gate", "g10.p0.m141.perf_baseline"], False),
    # ── 波聚合/决策门(步骤 180/176/186/191/192/193;只读汇总不重跑子门 smoke)──
    ("g10.wave.2.exit", "g10_wave2_exit",
     ["ci/g10_wave2_exit_check.py", "--gate", "g10.wave.2.exit"], False),
    ("g10.wave.3.exit", "g10_wave3_exit",
     ["ci/g10_wave3_exit_check.py", "--gate", "g10.wave.3.exit"], False),
    ("g10.wave.4.exit", "g10_wave4_exit",
     ["ci/g10_wave4_exit_check.py", "--gate", "g10.wave.4.exit"], False),
    ("g10.wave.5.exit", "g10_wave5_exit",
     ["ci/g10_wave5_exit_check.py", "--gate", "g10.wave.5.exit"], False),
    ("g10.wave.6.reevaluation", "g10_wave6_reevaluation",
     ["ci/g10_wave6_reevaluation_check.py", "--gate", "g10.wave.6.reevaluation"], False),
    ("g10.wave.7.decisions", "g10_p2_decisions",
     ["ci/g10_p2_decisions_check.py", "--gate", "g10.wave.7.decisions"], False),
]

MIN_SECONDS = 1800  # 沿 G9.8a 继承(≥30min;契约「或 measured 证明更短足够」未触)
MIN_ITERATIONS = 3  # 全链路迭代计数非空下界(实测单轮约 1min 量级,1800s 远超)
# 14 门(12 P0 + 2 go P1)为门 evidence(顶层 status=="pass" 字面 + base_commit);
# 后 6 门为波聚合/决策 evidence(wel 口径,无顶层 status/base_commit 字段)。
N_ASSERTION_GATES = 14

SOAK_REPRO_DIR = m139.FRAMES / "soak_repro"


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


def verify_assertion_gate(key: str, prefix: str, dual_phase_full: bool) -> dict:
    """14 门(12 P0 + 2 go P1)最新 evidence 机器核验。

    在 wel.require_gate_pass 口径(symbolic_gate_key/host_section_pass/
    device_section_state/checks 全真)之上叠加:
    - 顶层 status=="pass" 字面(MAP §1 evidence 必备字段,skip/estimated 不充绿);
      G10 证据形态统一(全门携带顶层 status/base_commit),无 G9 M90/M91 缺字段
      豁免面——缺字段即红;
    - dual_phase_full=True(M130)按 MAP §3.3 完整期口径叠加
      phase_g10_2_pass==true 且 phase_g10_5_pass==true(骨架期绿不替双端核验期
      充绿)。
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
        problems.append("缺顶层 status 字段(G10 证据形态统一,无豁免面)")
    if dual_phase_full:
        if doc.get("phase_g10_2_pass") is not True:
            problems.append("phase_g10_2_pass≠true")
        if doc.get("phase_g10_5_pass") is not True:
            problems.append("phase_g10_5_pass≠true(骨架期绿不替双端核验期充绿)")
    if problems:
        row["status"] = "FAIL"
        row["detail"] = f"{row.get('detail', '')}; " + "; ".join(problems)
    return row


def run_regression(*, skip_rerun: bool = False) -> tuple[bool, list[dict], str, bool]:
    """全量回归(14 门 + 6 波聚合/决策门)。

    skip_rerun=False(--gate):逐门真跑 smoke --gate 后核验其最新 evidence;
    skip_rerun=True(--verify-latest):只读最新 evidence。
    返回 (all_ok, rows, head_commit, base_uniform)。base_uniform 口径(MAP §7
    「同一候选 close-out 基线」):
    - 14 门 evidence base_commit 同值且=当前 HEAD(G10 证据形态统一携带
      base_commit,零豁免闭集——任一门缺字段即红);
    - --gate 模式追加新鲜度机核:20 门最新 evidence 文件名 UTC stamp 均 ≥
      本 run 起点 stamp(证明逐门真跑于本次 close-out 基线工作树)。
    """
    rows: list[dict] = []
    commit = base_commit()
    run_start_stamp = wel.utc_stamp()
    all_ok = True
    bases: list[str] = []
    no_base_field: list[str] = []
    stale: list[str] = []
    stamp_re = re.compile(r"_(\d{8}T\d{6}Z)\.json$")
    for idx, (key, prefix, argv, dual_phase_full) in enumerate(REGRESSION_GATES):
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
            print(f"[8a] regression {key}", flush=True)
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
            row = verify_assertion_gate(key, prefix, dual_phase_full)
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
    """诚实判定全链路 soak 输出。返回 (ok, problems)。

    反假绿(沿 G9.8a/G8.8a 2026-08-08 口径,G10 无 legacy 兼容):
    - 必须带 honesty 字段(soak_subject=chain-soak / sleep_seconds /
      active_chain_seconds);缺字段 = 伪造 → 红。
    - sleep_seconds 必须 == 0(禁 sleep 充墙钟)。
    - active_chain_seconds ≈ seconds(墙钟只能来自真实链路复跑)。
    - 外测墙钟(非 None 时)不得小于自称 seconds - 2s(谎报时长 → 红)。
    - 双阈值:iterations ≥ min_iterations 且 seconds ≥ min_seconds。
    - 迭代失败计数 failures == 0;链路各环节计数非空(rurix_frames_rendered/
      ldr_derivations/frames_decoded/metric_triplets/diff_reports/
      gap_registry_assemblies ≥1)。
    host CPU 参考管线链无 device 面:不以 validation/device_lost 作门。
    """
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
        "metric_triplets",
        "diff_reports",
        "gap_registry_assemblies",
    ):
        if int(doc.get(k) or 0) < 1:
            problems.append(f"{k} 计数面空")
    return (not problems), problems


def _soak_iteration(work: Path, scene_param_digests: dict[str, str]) -> list[str]:
    """单轮全链路复跑(出图→捕获→度量→差距清单;复用 M139 门数据面单一事实源)。

    返回问题列表(空 = 本轮全绿)。任一环节失败即记录并中止本轮(fail-fast)。
    scene_param_digests = M130 g10.5 最新 evidence 登记的逐场景契约 digest
    (LDR 派生 --params-digest 消费面,与 M139 门 ⑤ 同一来源)。
    """
    problems: list[str] = []
    # ① 出图:Rurix HDR release 重渲染双场景,digest 逐位复现库帧。
    for scene in m139.SCENES:
        params = m139.CORPUS / f"contract_params_{scene.replace('-', '_')}.json"
        out_dir = work / scene
        out_dir.mkdir(parents=True, exist_ok=True)
        rr = m139.run_cmd([
            str(m139.RUST_RELEASE_BIN), "--render", "--gltf", str(m139.GLTF[scene]),
            "--contract", str(params), "--out-dir", str(out_dir), "--scene-id", scene,
        ], timeout=1800)
        repro_digest = ""
        if rr.returncode == 0:
            m = re.search(r'"frame_content_digest":"(sha256:[0-9a-f]{64})"', rr.stdout or "")
            repro_digest = m.group(1) if m else ""
        want = m139.LIB_HDR_DIGEST[(scene, "rurix")]
        if rr.returncode != 0 or repro_digest != want:
            problems.append(f"出图未逐位复现({scene}): exit={rr.returncode} {repro_digest} ≠ {want}")
            return problems
        # LDR 派生四臂逐字节复现(双端;--params-digest = 逐场景契约 digest,
        # M130 evidence contract_report.scenes[] 登记面,与 M139 门同字面)。
        lib_hdr_r = m139.FRAMES / "rurix" / f"{scene}.exr"
        lib_hdr_u = m139.FRAMES / "ue" / scene / ".0000.exr"
        scene_digest = scene_param_digests.get(scene, "")
        if not scene_digest:
            problems.append(f"缺逐场景契约 digest({scene})——M130 evidence 绑定面失效")
            return problems
        for end, hdr_path, scale, lib_ldr in (
            ("rurix", lib_hdr_r, m139.EXPOSURE_SCALE_RURIX[scene],
             m139.FRAMES / "ldr" / f"{scene}_rurix_ldr.exr"),
            ("ue5", lib_hdr_u, 1.0,
             m139.FRAMES / "ldr" / f"{scene}_ue5_ldr.exr"),
        ):
            out_ldr = out_dir / f"{scene}_{end}_ldr.exr"
            rd = m139.run_cmd([
                str(m139.RUST_RELEASE_BIN), "--derive-ldr", "--hdr", str(hdr_path),
                "--source-end", end, "--out", str(out_ldr),
                "--exposure-scale", str(scale), "--params-digest", scene_digest,
            ], timeout=900)
            if rd.returncode != 0 or not out_ldr.is_file() or out_ldr.read_bytes() != lib_ldr.read_bytes():
                problems.append(f"LDR 派生未逐字节复现({scene}/{end})")
                return problems
    # ② 捕获:双端四组帧解码 + UE 帧 provenance(build == M128 登记)+ 内容 digest。
    m128_path = wel.load_latest_evidence("g10_m128_ue5_capture_environment")
    ue_build_id = ""
    if m128_path is not None:
        try:
            ue_build_id = wel.load_json(m128_path).get("capture_report", {}).get("ue_build_id", "")
        except (OSError, json.JSONDecodeError):
            ue_build_id = ""
    if not ue_build_id:
        problems.append("M128 最新 evidence 缺 ue_build_id(捕获面 provenance 失效)")
        return problems
    frame_problems = m139.frame_set_problems(m139.FRAMES)
    if frame_problems:
        problems.append(f"双端帧齐备面问题: {frame_problems[:3]}")
        return problems
    measured: dict = {}
    metric_report_digests: dict[str, str] = {}
    artifact_digests: set[str] = set()
    for scene in m139.SCENES:
        p = m139.FRAMES / "ue" / scene / ".0000.exr"
        attrs, _ = m139.exr.parse_header(p.read_bytes())
        build_attr = next((a[2].decode("utf-8", "replace") for a in attrs if a[0] == "unreal/build"), "")
        if not build_attr.startswith(ue_build_id):
            problems.append(f"UE 帧 build 与 M128 登记不符({scene})")
            return problems
        d = m139.exr.decode_exr(p.read_bytes(), "ue5")
        content = m139.exr.frame_content_digest(d["width"], d["height"], 3, d["pixels"])
        if content != m139.LIB_HDR_DIGEST[(scene, "ue5")]:
            problems.append(f"UE HDR 库帧 digest 漂移({scene})")
            return problems
    # ③ 度量:LDR 臂 FLIP/SSIM/PSNR + 亮度统计重算 == G10.5a golden 逐位。
    for scene in m139.SCENES:
        fp = m139.frame_paths(m139.FRAMES, scene)
        hdr_r, arr_hdr_r = m139.load_pixels(fp["hdr_rurix"], "rurix")
        hdr_u, arr_hdr_u = m139.load_pixels(fp["hdr_ue5"], "ue5")
        ldr_r, arr_r = m139.load_pixels(fp["ldr_rurix"], "rurix")
        ldr_u, arr_u = m139.load_pixels(fp["ldr_ue5"], "rurix")
        ssim_v = m139.ssim_psnr.ssim_wang2004(arr_u, arr_r)
        psnr_v = m139.ssim_psnr.psnr_joint(arr_u, arr_r)
        _err_map, flip_v = m139.flip.flip_ldr(arr_u, arr_r)
        stats = {
            "hdr_rurix": m139.lum_stats(arr_hdr_r), "hdr_ue5": m139.lum_stats(arr_hdr_u),
            "ldr_rurix": m139.lum_stats(arr_r), "ldr_ue5": m139.lum_stats(arr_u),
        }
        metrics = {
            "flip_ldr": float(flip_v),
            "ssim": float(ssim_v),
            "psnr_db": m139.ssim_psnr.psnr_json_value(psnr_v),
        }
        g = m139.GOLDEN[scene]
        golden_ok = (
            metrics["flip_ldr"] == g["flip_ldr"]
            and metrics["ssim"] == g["ssim"]
            and metrics["psnr_db"] == g["psnr_db"]
            and all(stats[k][kk] == g[k][kk]
                    for k in ("hdr_rurix", "hdr_ue5", "ldr_rurix", "ldr_ue5")
                    for kk in ("median", "p90", "max", "nonzero_ratio"))
        )
        if not golden_ok:
            problems.append(f"度量重算 ≠ G10.5a golden({scene})")
            return problems
        measured[scene] = {**stats, "metrics": metrics}
        artifact = {
            "scene_id": scene,
            "camera_id": m139.CAMERA_ID,
            "frame_digests": {
                "hdr_rurix": m139.exr.frame_content_digest(hdr_r["width"], hdr_r["height"], 3, hdr_r["pixels"]),
                "hdr_ue5": m139.exr.frame_content_digest(hdr_u["width"], hdr_u["height"], 3, hdr_u["pixels"]),
                "ldr_rurix_source": ldr_r["metadata"].get("rurix:source_frame_digest"),
                "ldr_ue5_source": ldr_u["metadata"].get("rurix:source_frame_digest"),
            },
            "stats": stats,
            "metrics": metrics,
            "metric_caliber": {
                "flip_ldr": m139.flip.flip_ldr_caliber_literal(m139.flip.default_ppd()),
                "ssim_psnr": "SSIM Wang 2004（11×11 高斯 σ=1.5，K1=0.01，K2=0.03，data_range=1.0，总体协方差，逐通道均值）/ PSNR 联合 MSE（RXS-0387）",
                "domain": "display-referred-ldr",
            },
            "exposure_scale": {"rurix": m139.EXPOSURE_SCALE_RURIX[scene], "ue5": 1.0},
            "exr_source_bit_depth": {"rurix": 32, "ue5": 16},
        }
        apath = m139.REPORT_DIR / f"{scene}_metric_report.json"
        apath.write_text(json.dumps(artifact, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        adigest = m139.sha256_file(apath)
        metric_report_digests[scene] = adigest
        artifact_digests.add(adigest)
    # diff 报告重跑 + 独立重算三面一致(H1 修订面 domain==display-referred-ldr)。
    heatmap_digests: dict[str, str] = {}
    error_map_digests: dict[str, str] = {}
    for scene in m139.SCENES:
        fp = m139.frame_paths(m139.FRAMES, scene)
        diff_dir = m139.FRAMES / "diff" / scene
        diff_dir.mkdir(parents=True, exist_ok=True)
        ev_path = diff_dir / "diff_report.json"
        rr = m139.run_cmd([
            str(m139.DIFF_BIN),
            "--frame-a", str(fp["ldr_ue5"]), "--frame-b", str(fp["ldr_rurix"]),
            "--out-dir", str(diff_dir), "--evidence", str(ev_path),
            "--scene-id", scene, "--camera-id", m139.CAMERA_ID,
            "--frame-index", "0", "--threshold", "0.0",
        ], timeout=1800)
        if rr.returncode != 0 or not ev_path.is_file():
            problems.append(f"diff 报告跑失败({scene})")
            return problems
        report = json.loads(ev_path.read_text(encoding="utf-8"))
        cs_fails = m139.m137.closed_set_failures(report)
        if cs_fails or report.get("domain") != "display-referred-ldr" or len(report.get("regions", [])) != 256:
            problems.append(f"diff 报告闭集/domain/区域数机核失败({scene})")
            return problems
        em = m139.exr.decode_exr_file(diff_dir / "error_map.exr", "rurix")
        all_err = sorted(float(v) for v in em["pixels"])
        n = len(all_err)
        scal = report["scalars"]
        g = m139.GOLDEN[scene]["diff"]
        recompute_ok = (
            m139.m137.f32_eq(all_err[-1], scal["err_max"])
            and m139.m137.f32_eq(m139.exr.nearest_rank_p95(all_err), scal["err_p95"])
            and abs(sum(all_err) / n - scal["err_mean"]) <= 1e-12
            and abs(scal["over_threshold_ratio"] - g["over_threshold_ratio"]) <= 1e-12
            and abs(scal["err_mean"] - g["err_mean"]) <= 1e-12
            and m139.m137.f32_eq(scal["err_p95"], g["err_p95"])
            and m139.m137.f32_eq(scal["err_max"], g["err_max"])
        )
        arts = report["artifacts"]
        fa = m139.exr.decode_exr_file(fp["ldr_ue5"], "rurix")
        fb = m139.exr.decode_exr_file(fp["ldr_rurix"], "rurix")
        digest_ok = (
            arts["frame_a_digest"] == m139.exr.frame_content_digest(fa["width"], fa["height"], 3, fa["pixels"])
            and arts["frame_b_digest"] == m139.exr.frame_content_digest(fb["width"], fb["height"], 3, fb["pixels"])
            and arts["error_map_digest"] == m139.exr.frame_content_digest(em["width"], em["height"], 1, em["pixels"])
            and arts["heatmap_digest"] == m139.sha256_file(diff_dir / "heatmap.ppm")
        )
        if not (recompute_ok and digest_ok):
            problems.append(f"diff 独立重算/digest 对账失败({scene})")
            return problems
        heatmap_digests[scene] = arts["heatmap_digest"]
        error_map_digests[scene] = arts["error_map_digest"]
        artifact_digests.add(arts["error_map_digest"])
        artifact_digests.add(arts["heatmap_digest"])
    # 探针 artifact 再生(R5 seed u64 顶格拒绝 + U3 bistro 动画通道计数)。
    import hashlib
    import tempfile
    seed_probe_path = m139.REPORT_DIR / "rurix_seed_u64_max_probe.json"
    seed_doc: dict = {"scene_id": "cornell-box", "probe": "contract_seed_u64_max_rejection"}
    base_params = json.loads((m139.CORPUS / "contract_params_cornell_box.json").read_text(encoding="utf-8"))
    base_params["time"]["random_seed"] = 18446744073709551615
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8") as tf:
        json.dump(base_params, tf)
        tmp_params = Path(tf.name)
    try:
        rp = m139.run_cmd([str(m139.RUST_RELEASE_BIN), "--contract-digest", str(tmp_params)], timeout=300)
        seed_doc["rust_exit_code"] = rp.returncode
        seed_doc["stderr_has_i64_boundary_reject"] = "i64" in (rp.stderr or "")
    finally:
        tmp_params.unlink(missing_ok=True)
    seed_doc["rejected"] = seed_doc.get("rust_exit_code", 0) != 0
    if not seed_doc["rejected"]:
        problems.append("R5 探针失效:u64 顶格 seed 未被 Rust 端拒绝")
        return problems
    seed_probe_path.write_text(json.dumps(seed_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    seed_digest = m139.sha256_file(seed_probe_path)
    artifact_digests.add(seed_digest)
    gltf_text = m139.BISTRO_GLTF.read_text(encoding="utf-8")
    gltf_doc = json.loads(gltf_text)
    anims = gltf_doc.get("animations", [])
    bistro_anim_channels = sum(len(a.get("channels", [])) for a in anims)
    gltf_probe = {
        "gltf": str(m139.BISTRO_GLTF),
        "gltf_digest": "sha256:" + hashlib.sha256(gltf_text.encode("utf-8")).hexdigest(),
        "animations_count": len(anims),
        "animation_channels": bistro_anim_channels,
        "consumed_by_dual_end": 0,
    }
    gltf_probe_path = m139.REPORT_DIR / "bistro_gltf_animations_probe.json"
    gltf_probe_path.write_text(json.dumps(gltf_probe, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    gltf_digest = m139.sha256_file(gltf_probe_path)
    artifact_digests.add(gltf_digest)
    # ④ 差距清单:装配 + gaplib 校验零错误 + 与在树逐字节相等幂等复核。
    measured["digests"] = {
        "metric_report": metric_report_digests,
        "diff_report": {},
        "heatmap": heatmap_digests,
        "error_map": error_map_digests,
        "seed_probe": seed_digest,
        "gltf_probe": gltf_digest,
    }
    measured["bistro_anim_channels"] = bistro_anim_channels
    registry_doc = m139.build_gap_registry(measured, sorted(artifact_digests))
    verrs = m139.gaplib.validate_registry(registry_doc, scene_set=list(m139.SCENES))
    for it in registry_doc["items"]:
        for dd in it["measured_delta"]:
            if dd["evidence_digest"] not in artifact_digests:
                verrs.append(f"{it['gap_id']} evidence_digest 不可回溯: {dd['evidence_digest']}")
    if verrs:
        problems.append(f"差距清单校验失败: {verrs[:4]}")
        return problems
    new_text = json.dumps(registry_doc, ensure_ascii=False, indent=2) + "\n"
    old_text = m139.REGISTRY_PATH.read_text(encoding="utf-8") if m139.REGISTRY_PATH.is_file() else None
    if old_text != new_text:
        problems.append("差距清单在树内容与当次装配漂移(幂等复核 RED)")
        return problems
    return problems


def run_chain_soak(
    *,
    min_seconds: int = MIN_SECONDS,
    min_iterations: int = MIN_ITERATIONS,
) -> tuple[bool, dict]:
    """全链路连续复跑 soak:≥min_seconds 墙钟内逐迭代真跑,零 sleep,零失败。"""
    counters = {
        "iterations": 0,
        "failures": 0,
        "rurix_frames_rendered": 0,
        "ldr_derivations": 0,
        "frames_decoded": 0,
        "metric_triplets": 0,
        "diff_reports": 0,
        "gap_registry_assemblies": 0,
    }
    problems: list[str] = []
    # 构建一次(循环外;release 渲染器 + debug diff 报告器)。
    rb = m139.run_cmd(["cargo", "build", "--release", "-p", "rurix-asset", "--bin", "g10_5_scene_render"], timeout=3600)
    db = m139.run_cmd(["cargo", "build", "-p", "rurix-render", "--bin", "g10_m137_diff_report"], timeout=3600)
    if rb.returncode != 0 or not m139.RUST_RELEASE_BIN.is_file() or db.returncode != 0 or not m139.DIFF_BIN.is_file():
        problems.append("soak 前置构建失败(release g10_5_scene_render / debug g10_m137_diff_report)")
        return False, {"ok": False, "detail": f"problems={problems}", "raw": {}, "counters": counters,
                       "seconds": 0.0, "active_chain_seconds": 0.0, "outer_elapsed": 0.0}
    SOAK_REPRO_DIR.mkdir(parents=True, exist_ok=True)
    m139.REPORT_DIR.mkdir(parents=True, exist_ok=True)
    # M130 g10.5 最新 evidence 逐场景契约 digest(LDR 派生 --params-digest 消费面,
    # 与 M139 门 ⑤ 同一来源;回归腿已本 run 刷新)。
    scene_param_digests: dict[str, str] = {}
    try:
        _m130_path, m130_doc = m139.latest_m130_g10_5()
        for s in m139.SCENES:
            scene_param_digests[s] = (
                m130_doc.get("contract_report", {}).get("scenes", {})
                .get(s, {}).get("param_digest", "")
            ).replace("sha256:", "")
    except RuntimeError as e:
        problems.append(f"M130 g10.5 最新 evidence 装载失败: {e}")
    if any(not scene_param_digests.get(s) for s in m139.SCENES):
        problems.append("M130 g10.5 evidence 逐场景契约 digest 缺行")
    t0 = time.time()
    active = 0.0
    while time.time() - t0 < min_seconds and not problems:
        it0 = time.time()
        it_problems = _soak_iteration(SOAK_REPRO_DIR, scene_param_digests)
        active += time.time() - it0
        counters["iterations"] += 1
        if it_problems:
            counters["failures"] += 1
            problems.extend(it_problems)
            break
        counters["rurix_frames_rendered"] += 2
        counters["ldr_derivations"] += 4
        # 单轮 EXR 解码精确计数:齐备面 8 + UE provenance 2 + 度量 load_pixels 8
        # + diff 相(error_map/fa/fb)6 = 24。
        counters["frames_decoded"] += 24
        counters["metric_triplets"] += 2
        counters["diff_reports"] += 2
        counters["gap_registry_assemblies"] += 1
        print(
            f"[8a] soak iter {counters['iterations']} ok "
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
        "base_commit_uniform",
        "soak_dual_threshold",
        "soak_no_sleep_padding",
        "budget_strict_pass",
        "date_anchor_recorded",
    ]
    bad = [k for k in need if checks.get(k) is not True]
    if bad:
        print(f"[8a] FAIL checks: {bad}", file=sys.stderr)
        return 1
    soak = data.get("soak") or {}
    ok, problems = judge_chain_soak(
        soak, outer_elapsed=None, min_seconds=MIN_SECONDS, min_iterations=MIN_ITERATIONS
    )
    if not ok:
        print(f"[8a] FAIL soak honesty: {problems}", file=sys.stderr)
        return 1
    print(f"[8a] verify-latest PASS(honest chain soak)← {path.relative_to(ROOT)}")
    return 0


def run_full_gate() -> int:
    if NUMERIC_STEP <= 0:
        print("[8a] NUMERIC_STEP unset (回填前草稿 → 红)", file=sys.stderr)
        return 1
    if not SCHEMA_PATH.is_file():
        print(f"[8a] schema missing: {SCHEMA_PATH}", file=sys.stderr)
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
    stamp = wel.utc_stamp()
    utc_date = stamp[:8]

    overall = reg_ok and base_uniform and soak_ok and bud_ok
    raw_soak = soak_info.get("raw") or {}
    no_sleep_ok, _ = judge_chain_soak(
        raw_soak, outer_elapsed=None, min_seconds=0, min_iterations=0
    )
    facts = [
        _fact(
            "regression_12p0_2p1_6wave",
            reg_ok,
            f"gates={len(reg_rows)} base_commit={commit}",
        ),
        _fact(
            "base_commit_uniform",
            base_uniform,
            "14 门 evidence base_commit 同值且=HEAD(同一候选 close-out 基线,MAP §7)",
        ),
        _fact("soak_dual_threshold", soak_ok, soak_info["detail"]),
        _fact("budget_strict", bud_ok, bud_detail),
        _fact("date_anchor", True, f"utc_date={utc_date}"),
    ]
    # host 链路 soak 无 device 面,validation/device_lost 不作门亦不写实(沿 G9.8a 语义)。
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
        "metric_triplets": int(counters.get("metric_triplets") or 0),
        "diff_reports": int(counters.get("diff_reports") or 0),
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
            "date_anchor_recorded": True,
        },
        "soak": soak_block,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": (
            "G10.8a full soak; four legs; honest semantics(沿 G9.8a/G8.8a 2026-08-08 口径): "
            "soak 墙钟=真实链路复跑实测(禁 sleep 充时,sleep_seconds 恒 0,active_chain_seconds "
            "逐迭代计时求和,gate 外测墙钟交叉核验);soak 载体=出图→捕获→度量→差距清单全链路 "
            "连续复跑(与 M139 门数据面同构:Rurix HDR release 重渲染 digest 逐位复现 + LDR 派生 "
            "逐字节复现 + 双端帧解码/UE provenance + FLIP/SSIM/PSNR 重算==G10.5a golden + diff "
            "报告重跑三面一致 + 差距清单装配校验与在树逐字节幂等复核);subject=chain-soak 无 "
            "device 零错字面量门;14 门 evidence base_commit 同值一致(MAP §7 同一候选 close-out 基线)"
        ),
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
            "frames_decoded": 192,
            "metric_triplets": 48,
            "diff_reports": 48,
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
    ap = argparse.ArgumentParser(description="G10.8a stabilization soak")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        if NUMERIC_STEP <= 0:
            print("[8a] selftest: NUMERIC_STEP=0 draft → expect red on --gate")
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
