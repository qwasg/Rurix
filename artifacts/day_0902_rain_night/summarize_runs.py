#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0902 雨夜战役:把 evidence json + render_runs.jsonl 账本汇总成 DELIVERABLES.json(digest / 帧时 / 粒子数 / 交付件 sha256)。

只读 evidence 与账本,写一个汇总登记件;供 REPORT.md 引用与复核。
用法: py -3 -B summarize_runs.py
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

RUN_TAGS = [
    "regress_off_anchor", "regress_on_a", "regress_on_b", "regress_on_baseline_exe", "rb_proof_a", "rb_proof_b",
    "ext_load_smoke", "probe_cam_C1", "probe_cam_C1_cd040", "probe_C1_cd050_t100_compact", "probe_C2_cd050_t50",
    "probe_C3_cd050_t50", "bistro_rain_night_C2", "bistro_rain_night_C1", "clip_C1_dolly_a", "clip_C1_dolly_b",
]
# evidence 文件名 ≠ 账本 tag 的映射
EVIDENCE_OF = {"bistro_rain_night_C2": "bistro_rain_night_C2", "bistro_rain_night_C1": "bistro_rain_night_C1"}
TAG_OF_EVIDENCE = {"bistro_rain_night_C2": "still_C2_final", "bistro_rain_night_C1": "still_C1_final"}
FILES = [
    "bistro_rain_night_C2.png", "bistro_rain_night_C1.png", "bistro_rain_night_C1_dolly.mp4", "probe_C3_cd050_t50.png",
    "contract_rain_night_C1_cd050.json", "contract_rain_night_C2_cd050.json", "contract_rain_night_C3_cd050.json",
    "exterior_scene_facts.json", "exterior_asset_verify.json", "contracts_index.json",
]


def sha(p: Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for c in iter(lambda: f.read(1 << 22), b""):
            h.update(c)
    return "sha256:" + h.hexdigest()


def main() -> int:
    ledger = [json.loads(l) for l in (HERE / "render_runs.jsonl").read_text(encoding="utf-8").splitlines() if l.strip()]
    by_tag: dict[str, dict] = {}
    for r in ledger:
        by_tag[r["tag"]] = r  # 同 tag 多跑取最后一次
    runs = {}
    for t in RUN_TAGS:
        ep = HERE / f"{t}.json"
        if not ep.is_file():
            print(f"WARN: evidence 缺失 {ep.name}")
            continue
        e = json.loads(ep.read_text(encoding="utf-8"))
        ledger_tag = TAG_OF_EVIDENCE.get(t, t)
        rec = by_tag.get(ledger_tag)
        presented = None
        if rec:
            m = re.search(r"presented=(sha256:[0-9a-f]{64})", " ".join(rec["stderr_tail"]))
            presented = m.group(1) if m else None
        ps = e.get("particle_stats") or {}
        runs[t] = {
            "ledger_tag": ledger_tag, "rc": rec["rc"] if rec else None, "wall_s": rec["wall_s"] if rec else None,
            "presented": presented, "render_digest": e.get("render_digest"), "digest_seq_sha": e.get("digest_seq_sha"),
            "digest_seq_n": len(e.get("digest_seq") or []), "frames": e.get("frames"), "warmup": e.get("warmup"),
            "tier": e.get("tier"), "trajectory": e.get("trajectory"), "frame_ms": e.get("frame_ms"),
            "particles": {k: ps.get(k) for k in ("n_final", "pids_issued", "emit_max")},
            "showcase": e.get("showcase"), "gltf": e.get("gltf"), "contract": e.get("contract"),
        }
        fm = e.get("frame_ms") or {}
        print(f"{t:30s} rc={runs[t]['rc']} wall={runs[t]['wall_s']} presented={(presented or '-')[:23]} "
              f"render={(e.get('render_digest') or '-')[:23]} seq={(e.get('digest_seq_sha') or '-')[:23]} "
              f"n={ps.get('n_final')} ms={fm.get('real_render_frame_ms')}/{fm.get('particle_gpu_mean_ms')}")
    files = {}
    for f in FILES:
        p = HERE / f
        if p.is_file():
            files[f] = {"bytes": p.stat().st_size, "sha256": sha(p)}
            print(f"{f:40s} {files[f]['bytes']:>12,d}  {files[f]['sha256']}")
        else:
            print(f"WARN: 交付件缺失 {f}")
    out = {"schema": "rurix.day0902.deliverables.v1", "runs": runs, "files": files, "ledger_records": len(ledger),
           "note": "由 summarize_runs.py 从 evidence json + render_runs.jsonl 汇总;raw/帧/mp4 二进制不入库,sha256 登记于此"}
    (HERE / "DELIVERABLES.json").write_text(json.dumps(out, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"账本记录 {len(ledger)} 条 → DELIVERABLES.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())
