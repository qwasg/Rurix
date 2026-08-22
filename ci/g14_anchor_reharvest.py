#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G14plus 延续波 digest 锚 18 格程序化重收割（RFC-0030 §4.7;M-h 门 ① 证载体）。

语义:G14.8~G14.11 优化波按 RFC-0030 授权改变渲染语义(mv GPU 化/vendor 驻留/
曝光显示域修正等),G14.3 stage_a 旧锚作废;本工具在优化定盘 commit 上对 18 格
(2 场景 × 3 tier × 3 后端)执行 **canonical 口径(160 帧 warmup 10)双收割**:
- 每格连续两次独立进程 bench,两跑 last_frame_digest **位级同值**才收录
  (任一格双跑不等 = RD-045 类非确定性面,硬退不写锚,如实报格位);
- 全 18 格收齐后原子重写 milestones/g14/g14_3_stage_a_digest_anchor.json:
  anchors 18 格新 digest + 顶层 reharvest{harvested_utc, source_gate_run,
  base_commit, double_harvest_bitexact}(M-h 门 anchor_reharvest_three_proofs
  字段闭集);旧锚字段结构 0-byte 保持(schema/description/harvested_utc/
  source_gate_run 顶层旧字段维持,历史可溯)。

时序纪律(M-h 门时间序机核):M-c 复跑绿 → 本工具收割 → M-d 复跑(新锚
drift guard)→ M-h。RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 恒双开。

用法: py -3 ci/g14_anchor_reharvest.py [--dry-run]
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
TAG = "g14_anchor_reharvest"

SCENES = ("cornell-box", "bistro-interior")
TIERS = (50, 67, 100)
BACKENDS = ("tsr_device", "dlss_sr", "fsr_3_1_5")
FRAMES = 160
WARMUP = 10


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def bench_digest(scene: str, tier: int, backend: str, out_root: Path) -> str:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    r = subprocess.run(
        [str(BIN), "--bench", "--scene", scene, "--tier", str(tier),
         "--backend", backend, "--frames", str(FRAMES), "--warmup", str(WARMUP),
         "--out-root", str(out_root)],
        cwd=ROOT, env=env, capture_output=True, text=True, timeout=1800,
    )
    if r.returncode != 0:
        tail = (r.stderr or "")[-800:]
        raise RuntimeError(f"bench {scene}/t{tier}/{backend} exit={r.returncode}: {tail}")
    receipt = out_root / scene / f"tier{tier}" / backend / "bench_receipt.json"
    doc = json.loads(receipt.read_text(encoding="utf-8"))
    dg = doc.get("last_frame_digest")
    if not isinstance(dg, str) or not dg.startswith("sha256:"):
        raise RuntimeError(f"bench {scene}/t{tier}/{backend} receipt 无 last_frame_digest")
    return dg


def base_commit() -> str:
    r = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True)
    return (r.stdout or "").strip() or "unknown"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true", help="只跑 2 格冒烟(首/末),不写锚")
    args = ap.parse_args()
    if not BIN.is_file():
        note(f"缺 {BIN}")
        return 1
    cells = [(s, t, b) for s in SCENES for t in TIERS for b in BACKENDS]
    if args.dry_run:
        cells = [cells[0], cells[-1]]
    t0 = time.monotonic()
    harvested: dict[str, dict] = {}
    with tempfile.TemporaryDirectory(prefix="g14_reharvest_") as td:
        run1_root = Path(td) / "run1"
        run2_root = Path(td) / "run2"
        for k, (scene, tier, backend) in enumerate(cells, 1):
            key = f"{scene}_t{tier}_{backend}"
            d1 = bench_digest(scene, tier, backend, run1_root)
            d2 = bench_digest(scene, tier, backend, run2_root)
            if d1 != d2:
                note(f"RED [{k}/{len(cells)}] {key} 双收割不等: {d1} ≠ {d2}"
                     "——非确定性面(RD-045 类)如实报,不写锚硬退")
                return 2
            harvested[key] = {"last_frame_digest": d1}
            note(f"[{k}/{len(cells)}] {key} 双收割位级同 {d1[:23]}… "
                 f"({time.monotonic() - t0:.0f}s)")
    if args.dry_run:
        note("dry-run 完成(不写锚)")
        return 0
    doc = json.loads(ANCHOR_PATH.read_text(encoding="utf-8"))
    old_n = len(doc.get("anchors") or {})
    if old_n != 18 or len(harvested) != 18:
        note(f"锚格数异常 old={old_n} new={len(harvested)}")
        return 1
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    doc["anchors"] = harvested
    doc["reharvest"] = {
        "harvested_utc": stamp,
        "source_gate_run": "ci/g14_anchor_reharvest.py（canonical 160 帧×双跑位级同值,18/18）",
        "base_commit": base_commit(),
        "double_harvest_bitexact": True,
    }
    ANCHOR_PATH.write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    note(f"锚重写完成 → {ANCHOR_PATH.relative_to(ROOT)} (reharvest.harvested_utc={stamp})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
