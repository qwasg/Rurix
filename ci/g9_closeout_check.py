#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.8b close-out 终审 g9.wave.8b.closeout(G9_CONTRACT G-G9-11;G9_PLAN §G9.8b;
G9_ACCEPTANCE_MAP §6;同构 ci/g8_closeout_check.py)。

只读汇总:34 key(15 P0 + 19 go P1)PASS + wave2~8a 七聚合门绿 + MAP 三向
+ P2 表 + budget --strict + 8a full-run 先行(立项裁决 6 同日放行:8a full-run
先行完成后允许同日 close-out)+ RD 最终状态逐字一致 + 最后新绿留痕。

输出 VERDICT = READY|BLOCKED。status flip 可与 READY 同波独立 commit。

用法:
  py -3 ci/g9_closeout_check.py --gate g9.wave.8b.closeout
  py -3 ci/g9_closeout_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g9_wave_exit_lib as wel  # noqa: E402
from g9_p2_decisions_check import FROZEN_IDS  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g9.wave.8b.closeout"
NUMERIC_STEP = 172
SUBJECT = "g9_wave8b_closeout"
WAVE = "G9.8b"
SOURCE_REF = (
    "G9_CONTRACT G-G9-11;G9_PLAN §G9.8b;G9_ACCEPTANCE_MAP §6;"
    "34 key + wave2~8a 聚合 + MAP 三向 + P2 + budget --strict + 8a 先行"
    "(同日放行立项裁决 6)+ RD 最终状态逐字一致"
)
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_wave8b_closeout_evidence_schema.json"
P2_TABLE_PATH = ROOT / "milestones" / "g9" / "G9_P2_DECISIONS.md"

# 15 P0 + 19 go P1(G9_ACCEPTANCE_MAP §2/§3 实记;key/prefix 与
# ci/g9_stabilization_soak.py REGRESSION_GATES 前 34 行同一闭集)。
P0_P1_KEYS = [
    ("g9.p0.m90.cluster_dag_deepening", "g9_m90_cluster_dag_deepening"),
    ("g9.p0.m91.page_format_v2_abi", "g9_m91_page_format_v2_abi"),
    ("g9.p0.m102.dgc_abstraction", "g9_m102_dgc_abstraction"),
    ("g9.p0.m103.descriptor_global_table", "g9_m103_descriptor_global_table"),
    ("g9.p0.m104.accesskind_indirect_edge", "g9_m104_accesskind_indirect_edge"),
    ("g9.p0.m121.physics_particle_view", "g9_m121_physics_particle_view"),
    ("g9.p0.m122.gameplay_field", "g9_m122_gameplay_field"),
    ("g9.p0.m93.visible_cluster_set", "g9_m93_visible_cluster_set"),
    ("g9.p0.m94.clas_rt_convergence", "g9_m94_clas_rt_convergence"),
    ("g9.p0.m95.single_source_truth", "g9_m95_single_source_truth"),
    ("g9.p0.m96.path_tracer_reference", "g9_m96_path_tracer_reference"),
    ("g9.p0.m97.surface_cache", "g9_m97_surface_cache"),
    ("g9.p0.m98.tracing_fallback_chain", "g9_m98_tracing_fallback_chain"),
    ("g9.p0.m110.world_partition", "g9_m110_world_partition"),
    ("g9.p0.m118.display_pipeline_view_transform", "g9_m118_display_pipeline_view_transform"),
    ("g9.p1.m92.gpu_skinning_lod_update", "g9_m92_gpu_skinning_lod_update"),
    ("g9.p1.m105.command_build_node", "g9_m105_command_build_node"),
    ("g9.p1.m106.execution_set_pso", "g9_m106_execution_set_pso"),
    ("g9.p1.m107.shader_library_ir_link", "g9_m107_shader_library_ir_link"),
    ("g9.p1.m99.spg_radiance_cache", "g9_m99_spg_radiance_cache"),
    ("g9.p1.m100.multi_light_low", "g9_m100_multi_light_low"),
    ("g9.p1.m101.if_tier_ladder", "g9_m101_if_tier_ladder"),
    ("g9.p1.m111.hlod_baking", "g9_m111_hlod_baking"),
    ("g9.p1.m112.atmosphere_froxel", "g9_m112_atmosphere_froxel"),
    ("g9.p1.m113.water_dual_pipeline", "g9_m113_water_dual_pipeline"),
    ("g9.p1.m114.hair_marschner", "g9_m114_hair_marschner"),
    ("g9.p1.m115.skin_burley_diffusion", "g9_m115_skin_burley_diffusion"),
    ("g9.p1.m116.terrain_chunk_cell", "g9_m116_terrain_chunk_cell"),
    ("g9.p1.m117.decal_dbuffer", "g9_m117_decal_dbuffer"),
    ("g9.p1.m119.post_processing_skeleton", "g9_m119_post_processing_skeleton"),
    ("g9.p1.m120.oit_benchmark_harness", "g9_m120_oit_benchmark_harness"),
    ("g9.p1.m124.buoyancy_field_channel", "g9_m124_buoyancy_field_channel"),
    ("g9.p1.m125.jolt_56_ab_evaluation", "g9_m125_jolt_56_ab_evaluation"),
    ("g9.p1.m126.rapier_benchmark_ab", "g9_m126_rapier_benchmark_ab"),
]

WAVE_EXITS = [
    ("g9.wave.2.exit", "g9_wave2_exit"),
    ("g9.wave.3.exit", "g9_wave3_exit"),
    ("g9.wave.4.exit", "g9_wave4_exit"),
    ("g9.wave.5.exit", "g9_wave5_exit"),
    ("g9.wave.6.exit", "g9_wave6_exit"),
    ("g9.wave.7.decisions", "g9_p2_decisions"),
    ("g9.wave.8a.soak", "g9_stabilization_soak"),
]

# G9_CONTRACT §6 Deferred 处置表字面:七条目总体 status 全维持 open
# (分项 closed/go 由候选决策表与验收面留痕,条目级 0-byte)。
RD_FINAL_OPEN_IDS = ["RD-034", "RD-039", "RD-040", "RD-041", "RD-042", "RD-043", "RD-044"]


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def evidence_utc_date(path: Path | None) -> str | None:
    if path is None:
        return None
    m = wel._UTC_STAMP_RE.search(path.name)
    if m:
        return m.group(1)[:8]
    doc = wel.load_json(path)
    ts = doc.get("timestamp") or doc.get("utc_date") or ""
    return str(ts)[:8] if ts else None


def max_first_pass_date() -> tuple[str | None, list[str]]:
    """对 34 key 取最新 PASS evidence 的 UTC 日期的 max(近似『最后新绿』)。"""
    dates: list[str] = []
    missing: list[str] = []
    for key, prefix in P0_P1_KEYS:
        p = wel.load_latest_evidence(prefix)
        if p is None:
            missing.append(key)
            continue
        doc = wel.load_json(p)
        ok, _ = wel.gate_pass_reason(doc, key)
        if not ok:
            missing.append(key)
            continue
        d = evidence_utc_date(p)
        if d:
            dates.append(d)
    if not dates:
        return None, missing
    return max(dates), missing


def check_rd_final_state() -> tuple[bool, str]:
    """G-G9-11「验收映射、候选决策、RD 最终状态逐字一致」机器化面:

    - deferred.json 七条 RD 条目级 status 逐字 == "open"(G9_CONTRACT §6
      处置表字面;分项历史只追加,条目级 0-byte);
    - G9_P2_DECISIONS.md 在树且 33 行 FROZEN_IDS 闭集逐 ID 在文
      (候选决策表最终状态无漂移的轻量核验;全表机核由 g9.wave.7.decisions
      门承载,本门不重复对账)。
    """
    problems: list[str] = []
    for rd in RD_FINAL_OPEN_IDS:
        st = wel.load_rd_status(rd)
        if st != "open":
            problems.append(f"{rd} status={st!r} ≠ 'open'")
    if not P2_TABLE_PATH.is_file():
        problems.append("G9_P2_DECISIONS.md 缺失")
    else:
        text = P2_TABLE_PATH.read_text(encoding="utf-8")
        absent = [i for i in FROZEN_IDS if i not in text]
        if absent:
            problems.append(f"P2 表缺 FROZEN_IDS: {absent}")
        if len(FROZEN_IDS) != 33:
            problems.append(f"FROZEN_IDS n={len(FROZEN_IDS)} ≠ 33(闭集口径漂移)")
    return (not problems), "; ".join(problems) if problems else "7 RD open 逐字一致 + P2 33 行闭集在树"


def run_closeout() -> int:
    if NUMERIC_STEP <= 0:
        print("[8b] NUMERIC_STEP unset → BLOCKED", file=sys.stderr)
        return 1
    today = wel.utc_stamp()[:8]
    facts: list[dict] = []
    gate_rows = [wel.require_gate_pass(k, p) for k, p in P0_P1_KEYS]
    gates_ok = all(r["status"] == "PASS" for r in gate_rows)
    facts.append(_fact("thirty_four_keys_pass", gates_ok, f"pass={sum(1 for r in gate_rows if r['status']=='PASS')}/34"))

    wave_rows = [wel.require_gate_pass(k, p) for k, p in WAVE_EXITS]
    waves_ok = all(r["status"] == "PASS" for r in wave_rows)
    facts.append(_fact("wave_exits_2_to_8a", waves_ok, f"pass={sum(1 for r in wave_rows if r['status']=='PASS')}/{len(WAVE_EXITS)}"))

    # MAP 三向
    map_r = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "check_g9_acceptance_map.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    map_ok = map_r.returncode == 0
    facts.append(_fact("acceptance_map_triple", map_ok, f"exit={map_r.returncode}"))

    # P2
    p2 = wel.load_latest_evidence("g9_p2_decisions")
    p2_ok = False
    if p2:
        d = wel.load_json(p2)
        p2_ok = d.get("host_section_pass") is True
    facts.append(_fact("p2_decisions_pass", p2_ok, str(p2.relative_to(ROOT)) if p2 else "missing"))

    # budget strict
    bud = subprocess.run(
        [sys.executable, str(ROOT / "ci" / "budget_eval.py"), "--strict"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    bud_ok = bud.returncode == 0 and "[budget_eval] PASS" in ((bud.stdout or "") + (bud.stderr or ""))
    facts.append(_fact("budget_strict", bud_ok, f"exit={bud.returncode}"))

    # 8a 先行
    e8a = wel.load_latest_evidence("g9_stabilization_soak")
    e8a_ok = False
    e8a_commit = None
    if e8a:
        d8 = wel.load_json(e8a)
        e8a_ok = d8.get("host_section_pass") is True
        e8a_commit = d8.get("base_commit")
    facts.append(_fact("soak_8a_precedes", e8a_ok, str(e8a.relative_to(ROOT)) if e8a else "missing"))

    # RD 最终状态逐字一致(G-G9-11)
    rd_ok, rd_detail = check_rd_final_state()
    facts.append(_fact("rd_final_state_consistent", rd_ok, rd_detail))

    # 留痕最后新绿 UTC 日(信息不阻断;同日 close-out 已放行——立项裁决 6)
    last_green, missing = max_first_pass_date()
    facts.append(
        _fact(
            "last_new_green_recorded",
            bool(last_green) and not missing,
            f"last_green_utc={last_green} today={today} missing={missing[:3]}",
        )
    )

    overall = all(f["status"] == "PASS" for f in facts)
    verdict = "READY" if overall else "BLOCKED"
    stamp = wel.utc_stamp()
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
        "verdict": verdict,
        "utc_date": today,
        "last_new_green_utc_date": last_green,
        "base_commit_8a": e8a_commit,
        "required_gates": gate_rows + wave_rows,
        "extra_facts": facts,
        "subjects": [],
        "checks": {
            "thirty_four_keys_pass": gates_ok,
            "wave_exits_pass": waves_ok,
            "acceptance_map_ok": map_ok,
            "p2_ok": p2_ok,
            "budget_strict_ok": bud_ok,
            "soak_8a_ok": e8a_ok,
            "rd_final_state_ok": rd_ok,
            "last_new_green_recorded": bool(last_green) and not missing,
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": stamp,
        "environment": wel.collect_environment(),
        "notes": "same-day closeout allowed after 8a full-run(立项裁决 6 继承 G8.8b 先例); status flip is a separate commit after READY",
    }
    if SCHEMA_PATH.is_file():
        errs = wel.validate_schema(payload, SCHEMA_PATH)
        if errs:
            print(f"[8b] schema: {errs}", file=sys.stderr)
            overall = False
            payload["host_section_pass"] = False
            payload["verdict"] = "BLOCKED"
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{stamp}.json"
    out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    for f in facts:
        print(f"  FACT  {f['status']:4}  {f['id']}  ({f['detail']})")
    print(f"  → evidence {out.relative_to(ROOT)}")
    print(f"  VERDICT = {payload['verdict']}")
    return 0 if overall else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        if NUMERIC_STEP <= 0:
            code = run_closeout()
            if code == 0:
                print("[selftest] FAIL: draft green", file=sys.stderr)
                return 1
            print("[selftest] PASS: draft → BLOCKED")
            return 0
        print("[selftest] OK materialized step", NUMERIC_STEP)
        return 0
    return run_closeout()


if __name__ == "__main__":
    sys.exit(main())
