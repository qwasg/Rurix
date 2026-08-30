#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""D6 窗口车道回归自验（GPU 锁内，RURIX_VK_VALIDATION=1）。

背景：g18_smooth_nrm.rx D6 扩签名（+tri_mr 第 9 路 storage view）→
g31 nrm/nrm_bloom 两变体描述组 += 8B 零哑表绑定（params[48]=0 门不读）。
窗口车道无 --ggx 面（pack_frame_params_nrm ggx=false 恒置 params[48]=0）。

判据（锚 = d2_window/d2w_summary.json 在案值）：
  off 臂 presented digest == 5596a730…（零漂移机核）；
  on 臂 == b02b08b57…（kernel D6 扩面 + 哑表绑定后位级不变）；
  组合臂（smooth+bloom+dither）== 12d5dc91…；
  validation 全静默。
产物：artifacts/night_0828/d6_ggx/d6w_summary.json
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

OUT = ROOT / "artifacts" / "night_0828" / "d6_ggx"
BIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
ANCHOR_OFF = "sha256:5596a7308b1df26b681ead4d8c321f6c2be5a116ceb98f892aad110da156cd00"
ANCHOR_ON = "sha256:b02b08b57e4e3020097c16f10e6f5c757c05d4a50ad3a6fb138ca2cc50d091f4"
ANCHOR_COMBO = "sha256:12d5dc917ee3779bddcdafc7c965528d99302340318bb3b3cfdf67a799418b60"
DITHER_ENCODE_SPV = "H:/rurix/.tmp/night_0828/spv/g31_display_encode.spv"

BASE = ["--frames", "8", "--warmup", "2", "--hidden"]

PASS_RE = re.compile(r'real_render_frame_ms=([\d.]+).*?digest="(sha256:[0-9a-f]+)"')
VAL_ERR_RE = re.compile(r"validation.*(error|ERROR)|VUID-", re.IGNORECASE)


def run_arm(tag: str, extra: list[str]) -> dict:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = "H:\\rurix\\target-night"
    env["RURIX_VK_VALIDATION"] = "1"
    ev = OUT / f"d6w_{tag}_ev.json"
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


def main() -> int:
    result: dict = {"anchors": {"off": ANCHOR_OFF, "on": ANCHOR_ON, "combo": ANCHOR_COMBO}, "runs": []}
    with gpu_device_lock(purpose="d6 ggx window lane regression"):
        result["runs"].append(run_arm("off", ["--smooth-normals", "off"]))
        for tag in ("on_1", "on_2"):
            result["runs"].append(run_arm(tag, ["--smooth-normals", "on"]))
        result["runs"].append(run_arm("combo", [
            "--smooth-normals", "on", "--bloom", "on", "--dither", "on",
            "--spv-encode", DITHER_ENCODE_SPV,
        ]))

    runs = {r["tag"]: r for r in result["runs"]}
    on_digests = {runs["on_1"]["digest"], runs["on_2"]["digest"]}
    result["verdict"] = {
        "off_matches_anchor": runs["off"]["digest"] == ANCHOR_OFF,
        "on_matches_d2w_anchor": on_digests == {ANCHOR_ON},
        "combo_matches_d2w_anchor": runs["combo"]["digest"] == ANCHOR_COMBO,
        "validation_silent_all": all(r["validation_hits"] == 0 for r in result["runs"]),
        "all_rc0": all(r["rc"] == 0 for r in result["runs"]),
        "frame_ms": {t: r["real_render_frame_ms"] for t, r in runs.items()},
    }
    v = result["verdict"]
    result["all_pass"] = all(v[k] for k in (
        "off_matches_anchor", "on_matches_d2w_anchor", "combo_matches_d2w_anchor",
        "validation_silent_all", "all_rc0"))
    OUT.joinpath("d6w_summary.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=1), encoding="utf-8")
    print(json.dumps(result["verdict"], ensure_ascii=False, indent=1))
    print("ALL_PASS" if result["all_pass"] else "FAIL")
    return 0 if result["all_pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
