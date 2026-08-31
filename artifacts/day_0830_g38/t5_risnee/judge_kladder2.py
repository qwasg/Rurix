#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G38 T5:lamp-k 阶梯判读器(消费 run_kladder.py 产物,产 lamp_k_ladder.json)。

判读口径:
  - 预算 = 11.11ms(90fps 字面,full19 s02 同预算);budget_margin_ms
    = 11.11 − frame_ms_p50。
  - p50/max 主源 = 各档 --profile-json 的 frame_segments[render_wall]
    (host 墙钟,与 evidence real_render_frame_ms 同一 render 腿口径);
    profile 缺失回退 evidence(mean 代 p50 + stats.render_max_ms),
    p50_source 如实登记。
  - verdict 逻辑(交接单字面):
      提档 = kept > 13(超过 0.6m 网格簇总量,证明网格收细真起效);
      lamp_k_go_candidate = 预算内(margin ≥ 0)最高 kept 档;
      存在预算内提档 ⇒ verdict = "go_candidate"(Wave3 决策口);
      kept>13 的一切档均超预算 ⇒ verdict = "restir_precondition_confirmed"
      (逐盏 K 提档在预算内不存在,ReSTIR 大件开窗条件成立 measured)。
  - 旋钮生效性注记:grid<0.6 档 clusters_total 仍 ==13 ⇒
    grid_knob_suspect_not_wired(接口约定档,主 agent 接线前跑会落此注记)。

用法:
  py -3 judge_kladder2.py             # 判读 kladder/ 产物 → lamp_k_ladder.json
  py -3 judge_kladder2.py --selftest  # 伪 runs/profile 两情景断言两 verdict
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
KL_DIR = HERE / "kladder"
RUNS_JSON = KL_DIR / "kladder_runs.json"
OUT_JSON = HERE / "lamp_k_ladder.json"

BUDGET_MS = 11.11          # 90fps 帧预算字面(full19 同预算)
BASE_CLUSTERS = 13         # 0.6m 网格簇总量字面(day_0828 ACCEPTANCE_SUMMARY)

VERDICT_NOTE = (
    "提档 = kept>13(0.6m 网格簇总量);go_candidate = 存在 margin≥0 的提档,"
    "lamp_k_go_candidate = 预算内最高 kept 档;restir_precondition_confirmed = "
    "kept>13 的一切档均超预算(逐盏 K 提档预算内不存在,ReSTIR 开窗条件成立 "
    "measured)。预算 11.11ms(90fps);margin = 11.11 − p50。"
)


def frame_ms_of(row: dict) -> dict:
    """帧时抽取:主源 profile render_wall p50/max;缺失回退 evidence。"""
    prof_p = Path(row.get("profile_json") or "")
    if prof_p.is_file():
        prof = json.loads(prof_p.read_text(encoding="utf-8"))
        seg = next((s for s in prof.get("frame_segments", [])
                    if s.get("name") == "render_wall"), None)
        if seg is not None:
            return {"p50": seg["p50_ms"], "max": seg["max_ms"],
                    "mean": seg["mean_ms"], "p50_source": "profile_render_wall"}
    # 回退面(如实登记 mean 代 p50,不冒充)。
    return {"p50": row.get("real_render_frame_ms"),
            "max": row.get("render_max_ms"),
            "mean": row.get("real_render_frame_ms"),
            "p50_source": "evidence_mean_fallback"}


def judge(runs: dict) -> dict:
    """全链判读(纯数据入 → verdict 出;selftest 复用)。"""
    steps: list[dict] = []
    notes: list[str] = []
    for row in runs["rows"]:
        if not row.get("ok"):
            notes.append(f"档 {row['tag']} 未 ok(rc={row.get('rc')}),跳过判读")
            continue
        fm = frame_ms_of(row)
        p50 = fm["p50"]
        margin = round(BUDGET_MS - p50, 3) if p50 is not None else None
        steps.append({
            "tag": row["tag"],
            "grid": row["grid_env"] if row["grid_env"] is not None else "0.6(缺省)",
            "k_req": row["k_req"] if row["k_req"] is not None else 12,
            "clusters_total": row["clusters_total"],
            "kept": row["kept"],
            "frame_ms": {"p50": p50, "max": fm["max"], "mean": fm["mean"],
                         "p50_source": fm["p50_source"]},
            "budget_margin_ms": margin,
            "digest": row["digest"],
        })

    # 证伪档专项(0.6/24):kept 应与基线一致(现网格下 --lamp-k 提额无效)。
    base = next((s for s in steps if s["tag"].endswith("baseline")), None)
    falsify = next((s for s in steps if s["tag"].endswith("falsify")), None)
    if base and falsify and base["kept"] is not None:
        ok_f = falsify["kept"] == base["kept"]
        notes.append(f"证伪档 kept={falsify['kept']} vs 基线 {base['kept']}:"
                     + ("一致(现网格下 k 提额无效,符合预期)" if ok_f
                        else "不一致(预期外,须人工核查)"))

    # 旋钮生效性:收细档 clusters_total 仍 ==13 ⇒ 疑未接线(接口约定档)。
    fine = [s for s in steps
            if s["grid"] not in ("0.6", "0.6(缺省)") and s["clusters_total"] is not None]
    knob_suspect = bool(fine) and all(
        s["clusters_total"] == BASE_CLUSTERS for s in fine)
    if knob_suspect:
        notes.append(f"grid_knob_suspect_not_wired:全部收细档 clusters_total 仍 "
                     f"={BASE_CLUSTERS}(RURIX_G31_LAMP_GRID_M 旋钮疑未接线,"
                     "阶梯量测无效,待主 agent 接线后重跑)")

    # verdict 主逻辑。
    judged = [s for s in steps if s["kept"] is not None
              and s["budget_margin_ms"] is not None]
    in_budget = [s for s in judged if s["budget_margin_ms"] >= 0.0]
    upgrades_in_budget = [s for s in in_budget if s["kept"] > BASE_CLUSTERS]
    go_candidate = max(in_budget, key=lambda s: s["kept"]) if in_budget else None
    if upgrades_in_budget:
        verdict = "go_candidate"
    else:
        verdict = "restir_precondition_confirmed"

    return {
        "schema": "rurix.day0830.g38.t5.lamp_k_ladder.v1",
        "budget_ms": BUDGET_MS,
        "base_clusters_literal": BASE_CLUSTERS,
        "verdict_note": VERDICT_NOTE,
        "steps": steps,
        "verdict": verdict,
        "lamp_k_go_candidate": (
            {"tag": go_candidate["tag"], "grid": go_candidate["grid"],
             "k_req": go_candidate["k_req"], "kept": go_candidate["kept"],
             "budget_margin_ms": go_candidate["budget_margin_ms"]}
            if go_candidate else None),
        "grid_knob_suspect_not_wired": knob_suspect,
        "notes": notes,
    }


def _fake_runs(frame_ms_by_tag: dict[str, float],
               kept_by_tag: dict[str, int],
               clusters_by_tag: dict[str, int]) -> dict:
    """selftest 伪 runs 构造(profile_json 指向不存在路径 ⇒ 恒走 evidence
    回退腿,回退口径本身即被测面)。"""
    ladder = [
        ("s1_g060_k12_baseline", None, None),
        ("s2_g060_k24_falsify", "0.6", 24),
        ("s3_g030_k24", "0.3", 24),
        ("s4_g030_k48", "0.3", 48),
        ("s5_g015_k48", "0.15", 48),
        ("s6_g015_k96", "0.15", 96),
    ]
    rows = []
    for tag, grid, k in ladder:
        rows.append({
            "tag": tag, "grid_env": grid, "k_req": k, "ok": True,
            "digest": f"sha256:fake_{tag}",
            "real_render_frame_ms": frame_ms_by_tag[tag],
            "render_min_ms": frame_ms_by_tag[tag] - 0.3,
            "render_max_ms": frame_ms_by_tag[tag] + 0.8,
            "clusters_total": clusters_by_tag[tag],
            "kept": kept_by_tag[tag],
            "dropped": clusters_by_tag[tag] - kept_by_tag[tag],
            "profile_json": "Z:/不存在/prof.json",
        })
    return {"rows": rows}


def do_selftest() -> int:
    """两情景断言:①存在预算内提档 ⇒ go_candidate;②提档全超线 ⇒
    restir_precondition_confirmed;附旋钮未接线情景注记断言。"""
    # 情景①:0.3/24 档 10.8ms(margin +0.31)且 kept=24>13 ⇒ go_candidate。
    clusters = {"s1_g060_k12_baseline": 13, "s2_g060_k24_falsify": 13,
                "s3_g030_k24": 32, "s4_g030_k48": 32,
                "s5_g015_k48": 64, "s6_g015_k96": 128}
    kept = {"s1_g060_k12_baseline": 12, "s2_g060_k24_falsify": 12,
            "s3_g030_k24": 24, "s4_g030_k48": 32,
            "s5_g015_k48": 48, "s6_g015_k96": 96}
    ms_go = {"s1_g060_k12_baseline": 9.75, "s2_g060_k24_falsify": 9.75,
             "s3_g030_k24": 10.8, "s4_g030_k48": 13.2,
             "s5_g015_k48": 13.4, "s6_g015_k96": 20.1}
    r1 = judge(_fake_runs(ms_go, kept, clusters))
    assert r1["verdict"] == "go_candidate", r1["verdict"]
    assert r1["lamp_k_go_candidate"]["tag"] == "s3_g030_k24", r1["lamp_k_go_candidate"]
    assert r1["steps"][0]["frame_ms"]["p50_source"] == "evidence_mean_fallback"
    assert not r1["grid_knob_suspect_not_wired"]

    # 情景②:一切 kept>13 档均超预算(EVAL_RESTIR §2 斜率预判形态:
    # K=24 ≈ +1.9 贴线后越线、K≥48 崩)⇒ restir_precondition_confirmed;
    # 预算内最高 kept 档退化为基线。
    ms_restir = {"s1_g060_k12_baseline": 9.75, "s2_g060_k24_falsify": 9.75,
                 "s3_g030_k24": 11.7, "s4_g030_k48": 15.6,
                 "s5_g015_k48": 15.8, "s6_g015_k96": 23.9}
    r2 = judge(_fake_runs(ms_restir, kept, clusters))
    assert r2["verdict"] == "restir_precondition_confirmed", r2["verdict"]
    assert r2["lamp_k_go_candidate"]["tag"] == "s1_g060_k12_baseline"

    # 情景③:旋钮未接线(收细档 clusters 全 =13、kept 全 =12)⇒ 注记 +
    # 无提档 ⇒ restir_precondition_confirmed 但带 suspect 旗标(判读留人工)。
    flat13 = {t: 13 for t in clusters}
    flat12 = {t: 12 for t in kept}
    r3 = judge(_fake_runs(ms_go, flat12, flat13))
    assert r3["grid_knob_suspect_not_wired"] is True
    assert any("grid_knob_suspect_not_wired" in n for n in r3["notes"])

    print(json.dumps({
        "selftest": "judge_kladder2", "pass": True,
        "scenario_go": r1["verdict"],
        "scenario_restir": r2["verdict"],
        "scenario_knob_suspect": r3["grid_knob_suspect_not_wired"],
    }, ensure_ascii=False))
    return 0


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--selftest", action="store_true", help="伪数据两情景(零 GPU)")
    args = ap.parse_args()
    if args.selftest:
        return do_selftest()
    if not RUNS_JSON.is_file():
        raise SystemExit(f"FAIL: 缺 {RUNS_JSON}(先跑 run_kladder.py)")
    runs = json.loads(RUNS_JSON.read_text(encoding="utf-8"))
    res = judge(runs)
    txt = json.dumps(res, ensure_ascii=False, indent=1)
    OUT_JSON.write_text(txt + "\n", encoding="utf-8")
    print(txt)
    print(f"JUDGE_KLADDER2 verdict={res['verdict']} → {OUT_JSON}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
