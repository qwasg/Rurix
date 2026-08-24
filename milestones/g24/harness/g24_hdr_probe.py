#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G24.2 M-b HDR 设备实测波）
"""G24.2 M-b HDR 设备面探针：vulkaninfo 表面色彩空间枚举取证。

M118-hdr-cal 重判条件设备半 = 「HDR 显示设备」实测：真跑 vulkaninfo，grep 表面
色彩空间 HDR token（HDR10_ST2084/BT2020/EXTENDED_SRGB_LINEAR 等），全量 stdout
存档 .tmp/g24_mb/vulkaninfo.log，结论落 milestones/g24/g24_hdr_probe_results.json。
"""
from __future__ import annotations

import datetime as _dt
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT_JSON = ROOT / "milestones/g24/g24_hdr_probe_results.json"
LOG_DIR = ROOT / ".tmp/g24_mb"

HDR_TOKENS = (
    "VK_COLOR_SPACE_HDR10_ST2084_EXT",
    "VK_COLOR_SPACE_BT2020_LINEAR_EXT",
    "VK_COLOR_SPACE_HDR10_HLG_EXT",
)
AUX_TOKENS = (
    "VK_COLOR_SPACE_EXTENDED_SRGB_LINEAR_EXT",
    "VK_EXT_swapchain_colorspace",
)


def main() -> int:
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    started = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    try:
        r = subprocess.run(["vulkaninfo"], capture_output=True, text=True, timeout=300)
        out = (r.stdout or "") + (r.stderr or "")
        tool_available = r.returncode == 0 or bool(out.strip())
    except (OSError, subprocess.TimeoutExpired) as e:
        out = f"<vulkaninfo 不可得: {e}>"
        tool_available = False
    (LOG_DIR / "vulkaninfo.log").write_text(out, encoding="utf-8", newline="\n")
    hdr_found = {t: bool([ln for ln in out.splitlines() if t in ln]) for t in HDR_TOKENS}
    aux_found = {t: bool([ln for ln in out.splitlines() if t in ln]) for t in AUX_TOKENS}
    hdr_available = tool_available and any(hdr_found.values())
    results = {
        "schema": "rurix.g24.hdr_probe.v1",
        "started_utc": started,
        "tool_available": tool_available,
        "hdr_colorspace_tokens": hdr_found,
        "aux_tokens": aux_found,
        "device_half_verdict": (
            "available" if hdr_available
            else ("not-available" if tool_available else "not-measurable")
        ),
        "log_path": ".tmp/g24_mb/vulkaninfo.log",
    }
    OUT_JSON.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
    print(f"[g24_hdr_probe] tool={tool_available} device_half={results['device_half_verdict']}")
    for t, v in {**hdr_found, **aux_found}.items():
        print(f"  {t}: {v}")
    print(f"[g24_hdr_probe] → {OUT_JSON.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
