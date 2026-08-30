#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Phase E1 工作项 2：重建后预设等价证明批跑（单锁串行 11 臂）。

证明面：
- 窗口 --quality full == 显式九臂 == 锚 6bd3af63（位级）+ 预设双跑位级；
- 窗口 all-off == 55e4a92d；七臂显式 == 8b1c12f3（补 D 相中断复验）；
- bench 默认 == c1d28ad7；snrm==778f1dfc / snrm+tsrq==05532d5e
  （lane_body 环境光分支改写后的直接零漂移证）；
- bench --quality full == 显式质量腿(+env ambient) 双跑位级（新锚收割）。
env 纪律：RURIX_REQUIRE_REAL=1 必配 RURIX_VK_VALIDATION=1；
RURIX_G18_AMBIENT 仅显式臂设 0.004，预设臂不设（预设自注入为证明对象）。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
BENCH = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"
E = ROOT / "artifacts" / "day_0828" / "e_final"

A_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
A_COMBO7 = "sha256:8b1c12f34a76fd554496c9bcdcb45c9ba7d90291e10273b3766708521471abec"
A_COMBO9 = "sha256:6bd3af638be97715e1a24f1d260639c259aa181a9d0144a6b47809538ef0814d"
A_BENCH_DEF = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
A_SNRM = "sha256:778f1dfcd2e2c163af79879f8e0e804000674a2db291283b9f42569c03eaac76"
A_SNRM_TSRQ = "sha256:05532d5e940ff10fead1ea989c855bb0fb86dac873967b32666f0fe3541d292a"

NINE = ("--smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 "
        "--textures on --bloom on --dither on --auto-exposure on "
        "--gi2 on --gi2-clamp 0.01 --tsr-quality on").split()
SEVEN = ("--smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 "
         "--textures on --bloom on --dither on --auto-exposure on").split()
BENCH_QL = ("--smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 "
            "--gi2 on --gi2-clamp 0.01 --tsr-quality on").split()
RENDER_BASE = ("--render --scene bistro-interior --tier 100 --backend tsr_device "
               "--frames 128 --warmup 10").split()
BENCH_BASE = ("--bench --scene bistro-interior --tier 100 --backend tsr_device "
              "--frames 160 --warmup 10").split()

AMB = {"RURIX_G18_AMBIENT": "0.004"}


def win_run(tag: str, extra: list[str], amb: bool) -> list[str]:
    ev = E / "ev" / f"{tag}.json"
    return [str(WIN), "--frames", "32", "--warmup", "2", "--hidden",
            *extra, "--evidence", str(ev)]


# (tag, argv, amb_env?, digest_kind, digest_path, expect|None)
RUNS: list[tuple[str, list[str], bool, str, str, str | None]] = [
    ("e_alloff",
     [str(WIN), "--frames", "8", "--warmup", "2", "--hidden",
      "--evidence", str(E / "ev" / "e_alloff.json")],
     False, "evidence", "ev/e_alloff.json", A_ALLOFF),
    ("e_combo9_explicit", win_run("e_combo9_explicit", NINE, True),
     True, "evidence", "ev/e_combo9_explicit.json", A_COMBO9),
    ("e_quality_full_1", win_run("e_quality_full_1", ["--quality", "full"], False),
     False, "evidence", "ev/e_quality_full_1.json", A_COMBO9),
    ("e_quality_full_2",
     win_run("e_quality_full_2", ["--quality", "full",
             "--dump-present-raw", str(E / "png" / "full.raw")], False),
     False, "evidence", "ev/e_quality_full_2.json", A_COMBO9),
    ("e_combo7_explicit", win_run("e_combo7_explicit", SEVEN, True),
     True, "evidence", "ev/e_combo7_explicit.json", A_COMBO7),
    ("e_bench_default",
     [str(BENCH), *BENCH_BASE, "--out-root", str(E / "arms" / "bench_default")],
     False, "bench", "arms/bench_default/bistro-interior/tier100/tsr_device/bench_receipt.json",
     A_BENCH_DEF),
    ("e_bench_snrm",
     [str(BENCH), *RENDER_BASE, "--smooth-normals", "on",
      "--out-root", str(E / "arms" / "bench_snrm")],
     False, "render", "arms/bench_snrm/bistro-interior/tier100/tsr_device/render_receipt.json",
     A_SNRM),
    ("e_bench_snrm_tsrq",
     [str(BENCH), *RENDER_BASE, "--smooth-normals", "on", "--tsr-quality", "on",
      "--out-root", str(E / "arms" / "bench_snrm_tsrq")],
     False, "render", "arms/bench_snrm_tsrq/bistro-interior/tier100/tsr_device/render_receipt.json",
     A_SNRM_TSRQ),
    ("e_bench_full_explicit",
     [str(BENCH), *RENDER_BASE, *BENCH_QL,
      "--out-root", str(E / "arms" / "bench_full_explicit")],
     True, "render",
     "arms/bench_full_explicit/bistro-interior/tier100/tsr_device/render_receipt.json", None),
    ("e_bench_full_preset_1",
     [str(BENCH), *RENDER_BASE, "--quality", "full",
      "--out-root", str(E / "arms" / "bench_full_preset_1")],
     False, "render",
     "arms/bench_full_preset_1/bistro-interior/tier100/tsr_device/render_receipt.json",
     "SAME_AS:e_bench_full_explicit"),
    ("e_bench_full_preset_2",
     [str(BENCH), *RENDER_BASE, "--quality", "full",
      "--out-root", str(E / "arms" / "bench_full_preset_2")],
     False, "render",
     "arms/bench_full_preset_2/bistro-interior/tier100/tsr_device/render_receipt.json",
     "SAME_AS:e_bench_full_explicit"),
]

DIGEST_FIELD = {"evidence": "digest", "bench": "last_frame_digest", "render": "converged_digest"}


def main() -> int:
    (E / "ev").mkdir(parents=True, exist_ok=True)
    (E / "png").mkdir(parents=True, exist_ok=True)
    (E / "arms").mkdir(parents=True, exist_ok=True)
    results: list[dict] = []
    got_by_tag: dict[str, str | None] = {}
    with gpu_device_lock(purpose="day0828 Phase E1 预设等价证明批跑", timeout_s=7200.0):
        for tag, argv, amb, kind, dpath, expect in RUNS:
            env = dict(os.environ)
            env.pop("RURIX_G18_AMBIENT", None)
            env["RURIX_REQUIRE_REAL"] = "1"
            env["RURIX_VK_VALIDATION"] = "1"
            if amb:
                env.update(AMB)
            t0 = time.time()
            r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True,
                               timeout=1800, env=env)
            wall = time.time() - t0
            got = None
            p = E / dpath
            if r.returncode == 0 and p.is_file():
                got = json.loads(p.read_text(encoding="utf-8")).get(DIGEST_FIELD[kind])
            got_by_tag[tag] = got
            exp = expect
            if isinstance(exp, str) and exp.startswith("SAME_AS:"):
                exp = got_by_tag.get(exp.split(":", 1)[1])
            vuid = (r.stderr or "").count("VUID-")
            ok = (got is not None) and (exp is None or got == exp)
            results.append({
                "tag": tag, "rc": r.returncode, "wall_s": round(wall, 1),
                "got": got, "expect": exp, "match": ok, "vuid_hits": vuid,
                "ambient_env": amb,
                "stderr_tail": (r.stderr or "").strip().splitlines()[-2:],
            })
            print(f"[{tag}] rc={r.returncode} {'MATCH' if ok else 'DRIFT/ERR'} "
                  f"vuid={vuid} ({wall:.0f}s) {got}", flush=True)
    passed = sum(1 for x in results if x["match"])
    summary = {"schema": "rurix.day0828.e1.equivalence_battery.v1",
               "passed": passed, "total": len(results),
               "all_green": passed == len(results), "runs": results}
    (E / "e2_equivalence_summary.json").write_text(
        json.dumps(summary, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"EQUIVALENCE {passed}/{len(results)}")
    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
