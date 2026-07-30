#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""时域重建冒烟(步骤 86;G5.2-H+G5.3-H;RFC-0016 章 H;验收门 G-G5-7)。

host 段(**恒跑**,纯 rust test,无 GPU):
  1. `cargo test -p rurix-render temporal::`——MV/jitter/历史验证/邻域裁剪/TAA 收敛/
     TSR SSIM 门/闪烁抑制/reactive/SSIM 门禁全套。

device 段(**按能力分波 gate real**):
  2. W1 TAA 内核经 Vulkan 真派发并与 host 金标准对拍；无 loader 时
     SKIP=dev-env-degrade，`RURIX_REQUIRE_REAL=1` 翻硬红。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"


def fail(msg: str) -> int:
    print(f"[renderer_temporal_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[renderer_temporal_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd, cwd: Path = ROOT, timeout: int = 900):
    r = subprocess.run(cmd, capture_output=True, cwd=str(cwd), timeout=timeout)
    return (
        r.returncode,
        r.stdout.decode("utf-8", "replace"),
        r.stderr.decode("utf-8", "replace"),
    )


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local"


def host_section(results: dict) -> bool:
    code, out, err = run(["cargo", "test", "-p", "rurix-render", "temporal::", "--", "--nocapture"])
    blob = out + err
    if code != 0:
        print(blob[-2400:], file=sys.stderr)
        results["temporal_pass"] = False
        return fail("host 段: rurix-render temporal:: 单测未过(时域底座/TAA/TSR 回归)")
    m = re.findall(r"test result: ok\. (\d+) passed; 0 failed", blob)
    total = sum(int(x) for x in m) if m else 0
    results["temporal_count"] = total
    if total < 40:
        return fail(f"host 段: temporal:: 测试计数 {total} < 40(时域底座/TAA/TSR 覆盖不全)")
    taa_conv = "taa_converges_static_scene" in blob
    tsr_ssim = "tsr" in blob and "ssim" in blob.lower()
    results["taa_convergence_present"] = taa_conv
    results["tsr_ssim_gate_present"] = tsr_ssim
    if not taa_conv:
        return fail("host 段: TAA 静态收敛测试不在集内(G-G5-7)")
    results["temporal_pass"] = True
    print(f"[renderer_temporal_smoke] host 步骤 1 PASS: rurix-render temporal:: {total} 单测全过")
    return True


def device_section(results: dict) -> int:
    expected = ["device_w1_taa_matches_host"]
    code, out, err = run(
        ["cargo", "test", "-p", "uc06-renderer", "--features", "vulkan",
         "device_w", "--", "--nocapture"]
    )
    blob = out + err
    if "SKIP" in blob and code == 0 and "0 passed" in blob:
        results["toolchain_skip"] = "no-vulkan"
        return skip("[device W1/W2] 无 Vulkan loader(TAA 内核真跑归 gate real)")
    if code != 0:
        print(blob[-2400:], file=sys.stderr)
        return fail("[device W1/W2] uc06-renderer TAA 对拍未过")
    missing = [name for name in expected if name not in blob]
    if missing:
        return fail(f"[device W1/W2] 测试不在集内: {missing}")
    results["device_wave_w1_pass"] = True
    results["device_wave_tests"] = expected
    print("[renderer_temporal_smoke] [device W1/W2] PASS: TAA 真跑对拍")
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "renderer_temporal_smoke",
        "milestone": "G5.2-H+G5.3-H / G-G5-7 (RFC-0016 章 H)",
        "step": 86,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "device_blocked": results.get("device_blocked"),
        "device_probe_note": results.get("device_probe_note"),
        **{k: results[k] for k in (
            "device_wave_w1_pass", "device_wave_w2_pass", "device_wave_tests",
        ) if k in results},
        "checks": {k: results.get(k) for k in (
            "temporal_pass", "temporal_count", "taa_convergence_present", "tsr_ssim_gate_present",
        ) if results.get(k) is not None},
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": results.get("toolchain_skip") is not None,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"renderer_temporal_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[renderer_temporal_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    device_rc = device_section(results) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if not host_ok:
        return 1
    if device_rc != 0:
        return device_rc
    print("[renderer_temporal_smoke] PASS(host 恒跑 + device W1 gate real)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
