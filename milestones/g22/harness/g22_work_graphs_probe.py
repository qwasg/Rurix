#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G22.3 M-d Work Graphs 设备实测波）
"""G22.3 M-d Work Graphs / DGC 设备扩展探针：vulkaninfo 扩展枚举取证。

Work Graphs 的 Vulkan 车道载体 = VK_AMDX_shader_enqueue（AMD 实验扩展，NVIDIA
预期 absent）；GPU-driven 提交现役载体 = DGC 三扩展（VK_EXT/NV_device_generated_commands
+ NV compute）。全量 stdout 存档 .tmp/g22_md/vulkaninfo.log，结论落
milestones/g22/g22_work_graphs_probe_results.json。
"""
from __future__ import annotations

import datetime as _dt
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT_JSON = ROOT / "milestones/g22/g22_work_graphs_probe_results.json"
LOG_DIR = ROOT / ".tmp/g22_md"

WG_TOKENS = ("VK_AMDX_shader_enqueue",)
DGC_TOKENS = (
    "VK_EXT_device_generated_commands",
    "VK_NV_device_generated_commands",
    "VK_NV_device_generated_commands_compute",
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
    wg_found = {t: bool([ln for ln in out.splitlines() if t in ln]) for t in WG_TOKENS}
    dgc_found = {t: bool([ln for ln in out.splitlines() if t in ln]) for t in DGC_TOKENS}
    results = {
        "schema": "rurix.g22.work_graphs_probe.v1",
        "started_utc": started,
        "tool_available": tool_available,
        "work_graphs_tokens": wg_found,
        "dgc_tokens": dgc_found,
        "work_graphs_verdict": (
            "not-available" if tool_available and not any(wg_found.values())
            else ("available" if any(wg_found.values()) else "not-measurable")
        ),
        "dgc_verdict": (
            "available" if tool_available and all(dgc_found.values())
            else ("partial" if any(dgc_found.values()) else "not-measurable")
        ),
        "dgc_host_surface": "src/rurix-rt/src/dgc.rs（M102 DGC 抽象层 token 闭集 + 装配期核验）",
        "log_path": ".tmp/g22_md/vulkaninfo.log",
    }
    OUT_JSON.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
    print(f"[g22_wg_probe] tool={tool_available} work_graphs={results['work_graphs_verdict']} dgc={results['dgc_verdict']}")
    print(f"[g22_wg_probe] → {OUT_JSON.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
