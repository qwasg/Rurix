#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G19.3 M-c RD-045 长窗观察波）
"""G19.3 M-c RD-045 长窗观察 driver（bistro-interior/t50/tsr_device 单格）。

RD-045 = 间歇性 digest 漂移生产化缺陷（M165 同型；G14.5a v5 检出 run1 末帧漂移，
G14.10 帧循环重构结构性消除候选根因面后未复现）。本窗字面动作 = backfill_condition
「flip-trace 诊断臂扩展至 g14_3_pipeline_perf TSR 车道逐帧 digest 轨迹取证」的
长窗延伸：受影响格连续 N 轮 bench 真跑，逐轮 receipt last_frame_digest 对 G14.12
冻结锚比对（G17.3 M-b 探针同模式）。

零漂移 → maintain-open + 长窗零复现证据 history 只追加（根因未逐字定位，close 须
backfill 三件齐：定位 + 修复 + Full RFC 评估——本窗不冒充 close）；
检出漂移 → 漂移轮全量 stderr 存档 + history 升级登记。两态均合法诚实终态。

输出 milestones/g19/g19_rd045_observation_results.json（M-c 门唯一消费面）。

用法：py -3 milestones/g19/harness/g19_rd045_observation.py [--rounds 12]
（GPU 独占窗执行；内部 gpu_device_lock）
"""
from __future__ import annotations

import datetime as _dt
import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

BIN = ROOT / "target/release/g14_3_pipeline_perf.exe"
OUT_ROOT = Path(r"K:\rurix-ext\g14-frames\rurix_prod")
RECEIPT = OUT_ROOT / "bistro-interior" / "tier50" / "tsr_device" / "bench_receipt.json"
ANCHOR_PATH = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
OUT_JSON = ROOT / "milestones/g19/g19_rd045_observation_results.json"
LOG_DIR = ROOT / ".tmp/g19_mc"
SCENE, TIER, BACKEND = "bistro-interior", 50, "tsr_device"


def run_round(idx: int) -> dict:
    import os
    import subprocess

    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"  # REQUIRE_REAL=1 硬要求（G17 探针同口径）
    t0 = time.time()
    # canonical 160 帧口径（锚 reharvest = canonical 160 帧×双跑；TSR 时域链
    # 末帧 digest 依赖帧数，非 canonical 帧数与锚不可比）。
    r = subprocess.run(
        [str(BIN), "--bench", "--scene", SCENE, "--tier", str(TIER),
         "--backend", BACKEND, "--frames", "160", "--warmup", "10"],
        cwd=ROOT, capture_output=True, text=True, timeout=3600, env=env,
    )
    ok = r.returncode == 0
    rec = (
        wel.load_json(RECEIPT)
        if ok and RECEIPT.is_file() and RECEIPT.stat().st_mtime >= t0 - 5
        else {}
    )
    digest = str(rec.get("last_frame_digest", ""))
    if not ok:
        fl = LOG_DIR / f"round_{idx:02d}_fail.log"
        fl.write_text((r.stdout or "") + (r.stderr or ""), encoding="utf-8", newline="\n")
    return {"round": idx, "exit": r.returncode, "ok": ok, "last_frame_digest": digest,
            "wall_s": round(time.time() - t0, 2)}


def main() -> int:
    rounds_n = 12
    args = sys.argv[1:]
    if "--rounds" in args:
        rounds_n = int(args[args.index("--rounds") + 1])
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    anchors = wel.load_json(ANCHOR_PATH).get("anchors", {})
    anchor = (anchors.get(f"{SCENE}_t{TIER}_{BACKEND}") or {}).get("last_frame_digest", "")
    started = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    rows: list[dict] = []
    with gpu_device_lock(purpose="g19_rd045_observation"):
        for i in range(1, rounds_n + 1):
            rr = run_round(i)
            rows.append(rr)
            hit = rr["last_frame_digest"] == anchor
            print(f"[g19_rd045] round {i}/{rounds_n} exit={rr['exit']} "
                  f"digest_hit={hit} wall={rr['wall_s']}s", flush=True)
    hits = sum(1 for r in rows if r["ok"] and r["last_frame_digest"] == anchor)
    drift_rounds = [r["round"] for r in rows if r["ok"] and r["last_frame_digest"] != anchor]
    all_ok = all(r["ok"] for r in rows)
    zero_drift = all_ok and not drift_rounds
    results = {
        "schema": "rurix.g19.rd045_observation.v1",
        "started_utc": started,
        "cell": f"{SCENE}/t{TIER}/{BACKEND}",
        "anchor_digest": anchor,
        "rounds_requested": rounds_n,
        "rounds": rows,
        "summary": {
            "rounds_ok": sum(1 for r in rows if r["ok"]),
            "digest_anchor_hits": hits,
            "drift_rounds": drift_rounds,
            "zero_drift": zero_drift,
        },
        "disposition": "maintain-open-with-extended-zero-recurrence" if zero_drift
        else "drift-detected-escalate",
        "disposition_basis": (
            "长窗零复现（G14.10 结构性消除后累计观察面扩展）；根因未逐字定位，"
            "backfill_condition 三件（定位+修复+Full RFC 评估）未全齐不冒充 close——"
            "maintain-open + history 只追加登记" if zero_drift else
            "观察窗内检出漂移轮——升级登记（漂移轮 stderr 在 .tmp/g19_mc/）"
        ),
    }
    OUT_JSON.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
    print(f"[g19_rd045] 完成 → {OUT_JSON}")
    print(f"  hits={hits}/{rounds_n} drift_rounds={drift_rounds} zero_drift={zero_drift}")
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
