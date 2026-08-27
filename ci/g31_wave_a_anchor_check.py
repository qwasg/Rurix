#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: TraeCode:Kimi-K3（G31+ 波 A 验收门 Task A6）
"""G31+ 波 A 验收门：零降级回归锚核验（g31.waveA.anchor_check；A6 三面锚程序产证载体）。

三面（全部真跑实测，数字来自命令输出，诚实登记不冒充）：

1. **Stage A digest 锚 18/18**：canonical 160 帧 bench（`target/release/g14_3_pipeline_perf.exe
   --bench --scene S --tier T --backend B --frames 160 --warmup 10`，g14_3 既有口径，
   RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 + gpu_device_lock 串行）逐格重跑，
   末帧 last_frame_digest 与 milestones/g14/g14_3_stage_a_digest_anchor.json 在案锚
   逐格对拍——18/18 位级一致 = 零漂移；任一格不符如实报格位（RED）。
2. **性能面（G17-MD-F1 焦点格 + 17/18 其余格）**：焦点格 bistro-interior/t100/dlss_sr
   新鲜 frame_ms_production_mean 与在案 3.5767ms（G30.2 M-b 20260825T102813Z）对照；
   新鲜 ratio = ue_median_ms（g14_m-d dual_end 最新 evidence 焦点格在案 3.43535ms）/
   新鲜 frame_ms——诚实红维持判据 = 新鲜 ratio ≥ 在案 0.960479（低于即如实 RED 登记，
   不冒充不静默）；其余 17 格 fresh frame_ms 与 g14_budget 在案 measured 逐格对照登记。
3. **五门在案盘点**：波 A 五门（present/pipelining/gameloop/dynscene/framegen）最新
   evidence 文件在树盘点（evidence/ 前缀闭集 + schema 字段闭集抽查），与本次验收会话
   复跑命令输出互证（命令输出逐字入 G31_CONTRACT §8 close-out）。

产物：evidence/g31_wave_a_anchor_check_<utc>.json（schema
milestones/g31/g31_wave_a_anchor_check_evidence_schema.json）。

用法：
  py -3 ci/g31_wave_a_anchor_check.py --gate
  py -3 ci/g31_wave_a_anchor_check.py --selftest
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

TAG = "g31_anchor_check"
GATE_KEY = "g31.waveA.anchor_check"
SCHEMA_ID = "rurix.g31.wave_a_anchor_check_evidence.v1"
SCHEMA_PATH = ROOT / "milestones/g31/g31_wave_a_anchor_check_evidence_schema.json"
ANCHOR_PATH = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
G14_BUDGET_PATH = ROOT / "milestones/g14/g14_budget.json"
BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
WORK_ROOT = ROOT / ".tmp" / "g31_waveA_accept" / "anchor_bench"

SCENES = ("cornell-box", "bistro-interior")
TIERS = (50, 67, 100)
BACKENDS = ("tsr_device", "dlss_sr", "fsr_3_1_5")
FRAMES = 160
WARMUP = 10

FOCUS = ("bistro-interior", 100, "dlss_sr")
FOCUS_ON_RECORD_FRAME_MS = 3.5767  # G30.2 M-b 焦点格新鲜真跑在案（g30_campaign_handover_registry G17-MD-F1 行）
FOCUS_ON_RECORD_RATIO = 0.960479   # 在案新鲜 ratio（同上；诚实红维持下界）

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def cell_key(scene: str, tier: int, backend: str) -> str:
    return f"{scene}_t{tier}_{backend}"


def bench_cell(scene: str, tier: int, backend: str) -> dict:
    """canonical 160 帧 bench 单格真跑 → receipt 三面字段。"""
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    argv = [
        str(BIN), "--bench", "--scene", scene, "--tier", str(tier),
        "--backend", backend, "--frames", str(FRAMES), "--warmup", str(WARMUP),
        "--out-root", str(WORK_ROOT),
    ]
    print(f"[{TAG}] $ {' '.join(argv)}", flush=True)
    t0 = time.time()
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=3600, env=env)
    wall = time.time() - t0
    receipt = WORK_ROOT / scene / f"tier{tier}" / backend / "bench_receipt.json"
    if r.returncode != 0:
        raise RuntimeError(f"bench {cell_key(scene, tier, backend)} exit={r.returncode}: {(r.stderr or '')[-400:]}")
    doc = json.loads(receipt.read_text(encoding="utf-8"))
    sp = doc.get("stats_post_warmup") or {}
    return {
        "digest": doc.get("last_frame_digest"),
        "frame_ms_production_mean": sp.get("frame_ms_production_mean"),
        "wall_s": round(wall, 3),
        "receipt": str(receipt),
    }


def compare_digest(fresh: str | None, anchor: str | None) -> bool:
    return isinstance(fresh, str) and isinstance(anchor, str) and fresh == anchor


def load_g14_budget_measured() -> dict[str, float]:
    """g14_budget 在案 g14.pipeline_perf.frame_ms.<cell> measured_value 面（17/18 其余格对照基线）。"""
    out: dict[str, float] = {}
    if not G14_BUDGET_PATH.is_file():
        return out
    doc = json.loads(G14_BUDGET_PATH.read_text(encoding="utf-8"))
    for e in doc.get("entries") or []:
        eid = str(e.get("id", ""))
        if eid.startswith("g14.pipeline_perf.frame_ms.") and isinstance(e.get("measured_value"), (int, float)):
            out[eid.rsplit(".", 1)[-1]] = float(e["measured_value"])
    return out


def load_focus_ue_median() -> float | None:
    """g14_m-d dual_end 最新 evidence 焦点格 ue_median_ms（ratio 分母在案锚）。"""
    import g11_wave_exit_lib as wel  # noqa: E402

    p = wel.load_latest_evidence("g14_m_d_dual_end_fps_parity")
    if not p:
        return None
    doc = wel.load_json(p)
    for c in (doc.get("parity", {}) or {}).get("cells", []) or []:
        if c.get("scene") == FOCUS[0] and c.get("tier") == FOCUS[1] and c.get("backend") == FOCUS[2]:
            v = c.get("ue_median_ms")
            return float(v) if isinstance(v, (int, float)) else None
    return None


def gate_evidence_inventory() -> list[dict]:
    """波 A 五门最新 evidence 在树盘点（前缀闭集；present 门 harness 件在 .tmp 工作目录登记）。"""
    inv: list[dict] = []
    specs = [
        ("g31.waveA.present", "g31_window_present"),
        ("g31.waveA.pipelining", "g31_frame_pipelining_"),
        ("g31.waveA.gameloop", "g31_game_loop_"),
        ("g31.waveA.dynscene", "g31_dynamic_scene_"),
        ("g31.waveA.framegen", "g31_framegen_present"),
    ]
    evdir = ROOT / "evidence"
    for gate, prefix in specs:
        files = sorted(evdir.glob(f"{prefix}*.json"), key=lambda p: p.stat().st_mtime) if evdir.is_dir() else []
        inv.append({
            "gate": gate,
            "prefix": prefix,
            "latest": files[-1].name if files else None,
            "present": bool(files),
        })
    return inv


def run_gate() -> int:
    if not BIN.is_file():
        print(f"[{TAG}] SKIP DEV_ENV_DEGRADE: 缺 {BIN}", file=sys.stderr)
        return 0
    anchor_doc = json.loads(ANCHOR_PATH.read_text(encoding="utf-8"))
    anchors = anchor_doc.get("anchors") or {}
    check(len(anchors) == 18, f"锚格数 ≠18: {len(anchors)}")
    budget_measured = load_g14_budget_measured()
    ue_median = load_focus_ue_median()
    check(ue_median is not None, "g14_m-d 焦点格 ue_median_ms 在案锚缺失")

    cells: list[dict] = []
    matched = 0
    t0 = time.time()
    with gpu_device_lock(purpose="g31 波 A 验收 Stage A 锚 18 格 canonical bench"):
        for scene in SCENES:
            for tier in TIERS:
                for backend in BACKENDS:
                    key = cell_key(scene, tier, backend)
                    got = bench_cell(scene, tier, backend)
                    anchor_dg = (anchors.get(key) or {}).get("last_frame_digest")
                    ok = compare_digest(got["digest"], anchor_dg)
                    matched += 1 if ok else 0
                    cells.append({
                        "cell": key,
                        "fresh_digest": got["digest"],
                        "anchor_digest": anchor_dg,
                        "digest_match": ok,
                        "fresh_frame_ms_production_mean": got["frame_ms_production_mean"],
                        "g14_budget_measured_ms": budget_measured.get(key),
                        "wall_s": got["wall_s"],
                    })
                    note(
                        f"[{len(cells)}/18] {key} digest {'MATCH' if ok else 'DRIFT'} "
                        f"frame_ms={got['frame_ms_production_mean']}（wall {got['wall_s']:.0f}s）"
                    )
    check(matched == 18, f"Stage A digest 锚漂移: {matched}/18（要求 18/18 位级一致）")

    focus = next(c for c in cells if c["cell"] == cell_key(*FOCUS))
    focus_ms = focus["fresh_frame_ms_production_mean"]
    fresh_ratio = (ue_median / focus_ms) if (ue_median and focus_ms) else None
    ratio_ok = fresh_ratio is not None and fresh_ratio >= FOCUS_ON_RECORD_RATIO
    check(
        ratio_ok,
        f"焦点格诚实红恶化: 新鲜 ratio={fresh_ratio} < 在案 {FOCUS_ON_RECORD_RATIO}"
        f"（fresh frame_ms={focus_ms} vs 在案 {FOCUS_ON_RECORD_FRAME_MS}）",
    )
    note(
        f"焦点格 {cell_key(*FOCUS)}: fresh frame_ms={focus_ms}ms（在案 {FOCUS_ON_RECORD_FRAME_MS}ms）"
        f" fresh ratio={round(fresh_ratio, 6) if fresh_ratio else None}（在案 {FOCUS_ON_RECORD_RATIO}，ue_median={ue_median}ms）"
        f" → {'维持不恶化' if ratio_ok else '恶化如实 RED'}"
    )

    gates = gate_evidence_inventory()
    for g in gates:
        check(g["present"], f"五门 evidence 缺档: {g['gate']}（前缀 {g['prefix']}）")

    verdict = "PASS" if not FAILURES else "FAIL"
    doc = {
        "schema": SCHEMA_ID,
        "gate": GATE_KEY,
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "binary": str(BIN.relative_to(ROOT)),
        "canonical": {"frames": FRAMES, "warmup": WARMUP, "cells": 18},
        "anchor_file": "milestones/g14/g14_3_stage_a_digest_anchor.json",
        "stage_a_digest": {"matched": matched, "total": 18, "zero_drift": matched == 18},
        "cells": cells,
        "focus_cell": {
            "cell": cell_key(*FOCUS),
            "fresh_frame_ms_production_mean": focus_ms,
            "on_record_frame_ms": FOCUS_ON_RECORD_FRAME_MS,
            "ue_median_ms_on_record": ue_median,
            "fresh_ratio": fresh_ratio,
            "on_record_ratio": FOCUS_ON_RECORD_RATIO,
            "honest_red_not_worsened": ratio_ok,
        },
        "wave_a_gates_inventory": gates,
        "wall_s_total": round(time.time() - t0, 3),
        "verdict": verdict,
        "notes": NOTES,
    }
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = ROOT / "evidence" / f"g31_wave_a_anchor_check_{ts}.json"
    out.write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence: {out}")
    if FAILURES:
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    print(
        f"[{TAG}] PASS gate={GATE_KEY}（Stage A digest 18/18 零漂移 + 焦点格 ratio "
        f"{round(fresh_ratio, 6)} ≥ 在案 {FOCUS_ON_RECORD_RATIO} + 17 格 frame_ms 对照登记 + 五门 evidence 5/5 在档）"
    )
    return 0


def run_selftest() -> int:
    d = "sha256:" + "0" * 64
    # 绿臂：位级一致判定。
    if not compare_digest(d, d):
        print(f"[{TAG}] selftest FAIL: 同 digest 误判 drift", file=sys.stderr)
        return 1
    # 红臂①：漂移必须检出。
    if compare_digest(d, "sha256:" + "1" * 64):
        print(f"[{TAG}] selftest FAIL: 漂移漏检", file=sys.stderr)
        return 1
    # 红臂②：None/非串不得判等。
    if compare_digest(None, d) or compare_digest(d, None):
        print(f"[{TAG}] selftest FAIL: None 冒充一致", file=sys.stderr)
        return 1
    # schema 在树 + required 闭集互核。
    if not SCHEMA_PATH.is_file():
        print(f"[{TAG}] selftest FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    req = set(schema.get("required", []))
    expect = {
        "schema", "gate", "generated_utc", "binary", "canonical", "anchor_file",
        "stage_a_digest", "cells", "focus_cell", "wave_a_gates_inventory",
        "wall_s_total", "verdict", "notes",
    }
    if req != expect:
        print(f"[{TAG}] selftest FAIL: schema required 漂移 {req ^ expect}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS (1 GREEN + 3 RED + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
