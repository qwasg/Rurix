#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D2 窗口车道平滑法线臂自验矩阵（GPU 锁内，RURIX_VK_VALIDATION=1）。

判据：
  off 臂 presented digest == 5596a730… 锚（零漂移机核）；
  on 臂双跑位级一致 + on≠off（接线生效）；
  组合臂（smooth+bloom+dither）双跑 digest 稳定 + rc=0 + validation 静默；
  视觉臂 on+ambient dump-present-raw 落盘（PNG 转换由 d2w_png.py 另跑）。
产物：artifacts/night_0828/d2_window/d2w_summary.json
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

OUT = ROOT / "artifacts" / "night_0828" / "d2_window"
BIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
ANCHOR = "sha256:5596a7308b1df26b681ead4d8c321f6c2be5a116ceb98f892aad110da156cd00"
DITHER_ENCODE_SPV = "H:/rurix/.tmp/night_0828/spv/g31_display_encode.spv"

BASE = ["--frames", "8", "--warmup", "2", "--hidden"]

PASS_RE = re.compile(r'real_render_frame_ms=([\d.]+).*?digest="(sha256:[0-9a-f]+)"')
VAL_ERR_RE = re.compile(r"validation.*(error|ERROR)|VUID-", re.IGNORECASE)


def run_arm(tag: str, extra: list[str], env_extra: dict[str, str] | None = None) -> dict:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = "H:\\rurix\\target-night"
    env["RURIX_VK_VALIDATION"] = "1"
    if env_extra:
        env.update(env_extra)
    ev = OUT / f"{tag}_ev.json"
    cmd = [str(BIN)] + BASE + extra + ["--evidence", str(ev)]
    t0 = time.monotonic()
    r = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True)
    dt = time.monotonic() - t0
    m = PASS_RE.search(r.stdout or "")
    val_hits = VAL_ERR_RE.findall(r.stderr or "") + VAL_ERR_RE.findall(r.stdout or "")
    return {
        "tag": tag,
        "rc": r.returncode,
        "wall_s": round(dt, 2),
        "digest": m.group(2) if m else None,
        "real_render_frame_ms": float(m.group(1)) if m else None,
        "validation_hits": len(val_hits),
        "stdout_tail": (r.stdout or "").strip().splitlines()[-2:],
        "stderr_tail": (r.stderr or "").strip().splitlines()[-4:],
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    result: dict = {"anchor": ANCHOR, "runs": []}
    with gpu_device_lock(purpose="night0828 D2 window lane"):
        # 构建（双 bin——共享体两臂兼容面）。
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = "H:\\rurix\\target-night"
        t0 = time.monotonic()
        b = subprocess.run(
            ["cargo", "build", "--release", "-p", "rurix-render", "--features",
             "vendor-upscale", "--bin", "g31_window_present", "--bin",
             "g14_3_pipeline_perf"],
            cwd=ROOT, env=env, capture_output=True, text=True,
        )
        result["build"] = {"rc": b.returncode, "wall_s": round(time.monotonic() - t0, 2)}
        if b.returncode != 0:
            result["build"]["stderr_tail"] = (b.stderr or "").strip().splitlines()[-12:]
            OUT.joinpath("d2w_summary.json").write_text(
                json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
            print("BUILD FAIL")
            return 1

        result["runs"].append(run_arm("off", ["--smooth-normals", "off"]))
        for tag in ("on_1", "on_2"):
            result["runs"].append(run_arm(tag, ["--smooth-normals", "on"]))
        for tag in ("combo_1", "combo_2"):
            result["runs"].append(run_arm(tag, [
                "--smooth-normals", "on", "--bloom", "on", "--dither", "on",
                "--spv-encode", DITHER_ENCODE_SPV,
            ]))
        # 视觉臂：smooth + 半球环境光（RURIX_G18_AMBIENT=0.08）+ presented raw dump。
        result["runs"].append(run_arm("visual_on", [
            "--smooth-normals", "on",
            "--dump-present-raw", str(OUT / "on_smooth_amb.raw"),
        ], env_extra={"RURIX_G18_AMBIENT": "0.08"}))

    runs = {r["tag"]: r for r in result["runs"]}
    off = runs["off"]
    ons = [runs["on_1"], runs["on_2"]]
    combos = [runs["combo_1"], runs["combo_2"]]
    on_digests = {r["digest"] for r in ons}
    combo_digests = {r["digest"] for r in combos}
    result["verdict"] = {
        "off_matches_anchor": off["digest"] == ANCHOR,
        "on_double_run_bitexact": len(on_digests) == 1 and None not in on_digests,
        "on_differs_from_off": on_digests.isdisjoint({off["digest"]}),
        "combo_runs_green": all(r["rc"] == 0 for r in combos),
        "combo_digest_stable": len(combo_digests) == 1 and None not in combo_digests,
        "validation_silent_all": all(r["validation_hits"] == 0 for r in result["runs"]),
        "visual_dump_exists": (OUT / "on_smooth_amb.raw").exists(),
        "frame_ms": {t: r["real_render_frame_ms"] for t, r in runs.items()},
    }
    v = result["verdict"]
    result["all_pass"] = (
        all(r["rc"] == 0 for r in result["runs"])
        and v["off_matches_anchor"]
        and v["on_double_run_bitexact"]
        and v["on_differs_from_off"]
        and v["combo_runs_green"]
        and v["combo_digest_stable"]
        and v["validation_silent_all"]
        and v["visual_dump_exists"]
    )
    OUT.joinpath("d2w_summary.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=1))
    return 0 if result["all_pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
