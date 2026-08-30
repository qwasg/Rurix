#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D6W2 窗口车道 GGX 臂自验矩阵（GPU 锁内，RURIX_VK_VALIDATION=1）。

判据：
  off 臂（--smooth-normals on，不 --ggx）presented digest == b02b08b57…
    （D6W2 改后零漂移——哑表绑定 + params[48]=0 与 D6W1 回归面逐位一致）；
  on 臂（--smooth-normals on --ggx on）双跑位级一致 + on≠off（接线生效）；
  组合臂（smooth+ggx+bloom+dither）双跑 digest 稳定 + rc=0 + validation 静默；
  视觉臂 off/on 各 --dump-present-raw 落盘 → PNG（GGX 唯一变量对照）。
产物：artifacts/night_0828/d6_ggx/d6w2_summary.json + d6w2_off/on.png
"""
from __future__ import annotations

import json
import os
import re
import struct
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

OUT = ROOT / "artifacts" / "night_0828" / "d6_ggx"
BIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
ANCHOR_ON = "sha256:b02b08b57e4e3020097c16f10e6f5c757c05d4a50ad3a6fb138ca2cc50d091f4"
DITHER_ENCODE_SPV = "H:/rurix/.tmp/night_0828/spv/g31_display_encode.spv"

BASE = ["--frames", "8", "--warmup", "2", "--hidden"]

PASS_RE = re.compile(r'real_render_frame_ms=([\d.]+).*?digest="(sha256:[0-9a-f]+)"')
VAL_ERR_RE = re.compile(r"validation.*(error|ERROR)|VUID-", re.IGNORECASE)


def run_arm(tag: str, extra: list[str]) -> dict:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = "H:\\rurix\\target-night"
    env["RURIX_VK_VALIDATION"] = "1"
    ev = OUT / f"d6w2_{tag}_ev.json"
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
        "stderr_tail": (r.stderr or "").strip().splitlines()[-4:],
    }


def raw_to_png(raw_path: Path, png_path: Path) -> bool:
    from PIL import Image
    data = raw_path.read_bytes()
    w, h = struct.unpack_from("<II", data, 0)
    px = data[8:]
    if len(px) != w * h * 4:
        return False
    Image.frombytes("RGBA", (w, h), px, "raw", "BGRA").save(png_path)
    return png_path.exists()


def main() -> int:
    result: dict = {"anchor_on": ANCHOR_ON, "runs": []}
    with gpu_device_lock(purpose="d6w2 window lane ggx"):
        result["runs"].append(run_arm("off", ["--smooth-normals", "on"]))
        for tag in ("on_1", "on_2"):
            result["runs"].append(run_arm(tag, ["--smooth-normals", "on", "--ggx", "on"]))
        for tag in ("combo_1", "combo_2"):
            result["runs"].append(run_arm(tag, [
                "--smooth-normals", "on", "--ggx", "on",
                "--bloom", "on", "--dither", "on",
                "--spv-encode", DITHER_ENCODE_SPV,
            ]))
        # 视觉臂：GGX 唯一变量对照（off/on 各 dump presented raw）。
        result["runs"].append(run_arm("visual_off", [
            "--smooth-normals", "on",
            "--dump-present-raw", str(OUT / "d6w2_off.raw"),
        ]))
        result["runs"].append(run_arm("visual_on", [
            "--smooth-normals", "on", "--ggx", "on",
            "--dump-present-raw", str(OUT / "d6w2_on.raw"),
        ]))

    runs = {r["tag"]: r for r in result["runs"]}
    on_digests = {runs["on_1"]["digest"], runs["on_2"]["digest"]}
    combo_digests = {runs["combo_1"]["digest"], runs["combo_2"]["digest"]}
    png_off = raw_to_png(OUT / "d6w2_off.raw", OUT / "d6w2_off.png") if (OUT / "d6w2_off.raw").exists() else False
    png_on = raw_to_png(OUT / "d6w2_on.raw", OUT / "d6w2_on.png") if (OUT / "d6w2_on.raw").exists() else False
    result["verdict"] = {
        "off_matches_d2w_anchor": runs["off"]["digest"] == ANCHOR_ON,
        "on_double_run_bitexact": len(on_digests) == 1 and None not in on_digests,
        "on_differs_from_off": on_digests.isdisjoint({runs["off"]["digest"]}),
        "on_digest": list(on_digests),
        "combo_runs_green": all(runs[t]["rc"] == 0 for t in ("combo_1", "combo_2")),
        "combo_digest_stable": len(combo_digests) == 1 and None not in combo_digests,
        "combo_digest": list(combo_digests),
        "validation_silent_all": all(r["validation_hits"] == 0 for r in result["runs"]),
        "visual_pngs": {"off": png_off, "on": png_on},
        "frame_ms": {t: r["real_render_frame_ms"] for t, r in runs.items()},
    }
    v = result["verdict"]
    result["all_pass"] = (
        all(r["rc"] == 0 for r in result["runs"])
        and v["off_matches_d2w_anchor"]
        and v["on_double_run_bitexact"]
        and v["on_differs_from_off"]
        and v["combo_runs_green"]
        and v["combo_digest_stable"]
        and v["validation_silent_all"]
        and png_off and png_on
    )
    OUT.joinpath("d6w2_summary.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(result["verdict"], ensure_ascii=False, indent=1))
    print("ALL_PASS" if result["all_pass"] else "FAIL")
    return 0 if result["all_pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
