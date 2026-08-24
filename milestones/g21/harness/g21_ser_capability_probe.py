#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G21.2 M-b SER capability 实测波）
"""G21.2 M-b SER capability 探针：vulkaninfo 扩展枚举取证。

M52 重判条件 capability 半边 = 「capability rt.ser 设备面实测可用」的机器取证：
真跑 vulkaninfo，grep VK_NV_ray_tracing_invocation_reorder /
VK_EXT_ray_tracing_invocation_reorder / ReorderingHint 字面，全量 stdout 存档
.tmp/g21_mb/vulkaninfo.log，结论落 milestones/g21/g21_ser_capability_probe_results.json。

工具不可得 → capability = not-measurable 如实登记（不冒充）。
"""
from __future__ import annotations

import datetime as _dt
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT_JSON = ROOT / "milestones/g21/g21_ser_capability_probe_results.json"
LOG_DIR = ROOT / ".tmp/g21_mb"

TOKENS = (
    "VK_NV_ray_tracing_invocation_reorder",
    "VK_EXT_ray_tracing_invocation_reorder",
    "rayTracingInvocationReorderReorderingHint",
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
    hits = {t: [ln.strip()[:160] for ln in out.splitlines() if t in ln][:3] for t in TOKENS}
    found = {t: bool(v) for t, v in hits.items()}
    capability_available = tool_available and found["VK_NV_ray_tracing_invocation_reorder"]
    gpu_lines = [ln.strip()[:120] for ln in out.splitlines() if "deviceName" in ln][:2]
    results = {
        "schema": "rurix.g21.ser_capability_probe.v1",
        "started_utc": started,
        "tool_available": tool_available,
        "tokens_found": found,
        "token_lines": hits,
        "device_lines": gpu_lines,
        "capability_verdict": (
            "available" if capability_available
            else ("not-available" if tool_available else "not-measurable")
        ),
        "log_path": ".tmp/g21_mb/vulkaninfo.log",
    }
    OUT_JSON.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
    print(f"[g21_ser_probe] tool={tool_available} capability={results['capability_verdict']}")
    for t, v in found.items():
        print(f"  {t}: {v}")
    print(f"[g21_ser_probe] → {OUT_JSON.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
