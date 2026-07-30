#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""虚拟化几何冒烟(步骤 84;G5.2-C+G5.3-C;RFC-0016 章 C;验收门 G-G5-5)。

host 段(**恒跑**,纯 rust test,无 GPU):
  1. `cargo test -p rurix-geom-build`——meshlet 化/层级 DAG/误差单调/边界锁定/序列化/CPU 参照剔除。
  2. `cargo test -p rurix-render geometry::`——两级剔除/VisBuffer 光栅/classify-resolve/GPU 编组。

device 段(**按能力分波 gate real**):
  3. W1/W2 几何内核经 Vulkan 真派发并与 host 金标准对拍；无 loader 时
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
    print(f"[renderer_visbuffer_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[renderer_visbuffer_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
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
    code, out, err = run(["cargo", "test", "-p", "rurix-geom-build", "--", "--nocapture"])
    blob = out + err
    if code != 0:
        print(blob[-2400:], file=sys.stderr)
        results["geom_build_pass"] = False
        return fail("host 段: rurix-geom-build 单测未过(meshlet/DAG/剔除回归)")
    m = re.findall(r"test result: ok\. (\d+) passed; 0 failed", blob)
    total = sum(int(x) for x in m) if m else 0
    results["geom_build_count"] = total
    if total < 20:
        return fail(f"host 段: geom-build 测试计数 {total} < 20(meshlet/DAG/剔除覆盖不全)")
    results["geom_build_pass"] = True
    print(f"[renderer_visbuffer_smoke] host 步骤 1 PASS: rurix-geom-build {total} 单测全过")

    code2, out2, err2 = run(["cargo", "test", "-p", "rurix-render", "geometry::", "--", "--nocapture"])
    blob2 = out2 + err2
    if code2 != 0:
        print(blob2[-2400:], file=sys.stderr)
        results["geometry_pass"] = False
        return fail("host 段: rurix-render geometry:: 单测未过(剔除/VisBuffer/classify 回归)")
    m2 = re.findall(r"test result: ok\. (\d+) passed; 0 failed", blob2)
    total2 = sum(int(x) for x in m2) if m2 else 0
    results["geometry_count"] = total2
    if total2 < 20:
        return fail(f"host 段: geometry:: 测试计数 {total2} < 20(剔除/VisBuffer/classify 覆盖不全)")
    results["geometry_pass"] = True
    print(f"[renderer_visbuffer_smoke] host 步骤 2 PASS: rurix-render geometry:: {total2} 单测全过")
    return True


def device_section(results: dict) -> int:
    expected = [
        "device_w1_cull_matches_host",
        "device_w2_visbuffer_u64_bitexact_host",
        "device_w1_classify_resolve_matches_host",
    ]
    code, out, err = run(
        ["cargo", "test", "-p", "uc06-renderer", "--features", "vulkan",
         "device_w", "--", "--nocapture"]
    )
    blob = out + err
    if "SKIP" in blob and code == 0 and "0 passed" in blob:
        results["toolchain_skip"] = "no-vulkan"
        return skip("[device W1/W2] 无 Vulkan loader(几何内核真跑归 gate real)")
    if code != 0:
        print(blob[-2400:], file=sys.stderr)
        return fail("[device W1/W2] uc06-renderer 几何内核对拍未过")
    missing = [name for name in expected if name not in blob]
    if missing:
        return fail(f"[device W1/W2] 测试不在集内: {missing}")
    results["device_wave_w1_pass"] = True
    results["device_wave_w2_pass"] = True
    results["device_wave_tests"] = expected
    print("[renderer_visbuffer_smoke] [device W1/W2] PASS: cull/classify + u64 VisBuffer 真跑对拍")
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "renderer_visbuffer_smoke",
        "milestone": "G5.2-C+G5.3-C / G-G5-5 (RFC-0016 章 C)",
        "step": 84,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "device_blocked": results.get("device_blocked"),
        "device_probe_note": results.get("device_probe_note"),
        **{k: results[k] for k in (
            "device_wave_w1_pass", "device_wave_w2_pass", "device_wave_tests",
        ) if k in results},
        "checks": {k: results.get(k) for k in (
            "geom_build_pass", "geom_build_count", "geometry_pass", "geometry_count",
        ) if results.get(k) is not None},
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": results.get("toolchain_skip") is not None,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"renderer_visbuffer_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[renderer_visbuffer_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    device_rc = device_section(results) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if not host_ok:
        return 1
    if device_rc != 0:
        return device_rc
    print("[renderer_visbuffer_smoke] PASS(host 恒跑 + device W1/W2 gate real)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
