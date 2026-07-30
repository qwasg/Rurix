#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""光照冒烟(步骤 85;G5.3-D/E/F;RFC-0016 章 D/E/F;验收门 G-G5-6)。

host 段(**恒跑**,纯 rust test,无 GPU):
  1. `cargo test -p rurix-render shadow::`——VSM clipmap 页表/失效/多视图深度/投影。
  2. `cargo test -p rurix-render gi::`——屏幕探针 GI(能量守恒/方向一致性/时域收敛/disocclusion)。
  3. `cargo test -p rurix-render rt::`——AS 管理/RTAO 硬阴影对拍/时域滤波。

device 段(**按能力分波**):
  4. W1 VSM page-mark 内核经 Vulkan 真派发并与 host 金标准对拍(gate real)。
  5. W3 GI/RTAO/硬阴影保留 blocked-honest：设备能力非阻塞项，工具链仍缺
     rurixc ray query 编码通道与 SPIR-V 1.4。
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
    print(f"[renderer_lighting_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[renderer_lighting_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
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
    for mod, min_count, label in [
        ("shadow::", 25, "VSM clipmap/页表/失效/投影"),
        ("gi::", 18, "屏幕探针 GI 闭环"),
        ("rt::", 25, "AS 管理/RTAO 硬阴影/时域滤波"),
    ]:
        code, out, err = run(["cargo", "test", "-p", "rurix-render", mod, "--", "--nocapture"])
        blob = out + err
        if code != 0:
            print(blob[-2400:], file=sys.stderr)
            results[f"{mod.rstrip(':')}_pass"] = False
            return fail(f"host 段: rurix-render {mod} 单测未过({label}回归)")
        m = re.findall(r"test result: ok\. (\d+) passed; 0 failed", blob)
        total = sum(int(x) for x in m) if m else 0
        results[f"{mod.rstrip(':')}_count"] = total
        if total < min_count:
            return fail(f"host 段: {mod} 测试计数 {total} < {min_count}({label}覆盖不全)")
        results[f"{mod.rstrip(':')}_pass"] = True
        print(f"[renderer_lighting_smoke] host 步骤 PASS: rurix-render {mod} {total} 单测全过")
    return True


def mark_w3_blocked(results: dict) -> None:
    results["device_blocked"] = "RD-038"
    results["device_probe_note"] = (
        "W3 gi_probe/rtao/hard_shadow blocked-honest；设备 ray query 五件链非阻塞项"
    )
    results["device_blocked_w3"] = "RD-038-W3"
    results["blocked_reason"] = (
        "工具链 rurixc compute ray query SPIR-V 编码通道未通，且须跨越 SPIR-V 1.4 升级门槛；"
        "设备能力非阻塞项"
    )
    results["missing_toolchain_caps"] = ["ray_query_codegen", "spirv_1_4"]
    print(
        "[renderer_lighting_smoke] [device W3 BLOCKED] RD-038-W3: "
        "gi_probe/rtao/hard_shadow 缺 ray_query_codegen + spirv_1_4；设备能力非阻塞项"
    )


def device_section(results: dict) -> int:
    expected = ["device_w1_vsm_page_mark_matches_host"]
    code, out, err = run(
        ["cargo", "test", "-p", "uc06-renderer", "--features", "vulkan",
         "device_w", "--", "--nocapture"]
    )
    blob = out + err
    if "SKIP" in blob and code == 0 and "0 passed" in blob:
        results["toolchain_skip"] = "no-vulkan"
        mark_w3_blocked(results)
        return skip("[device W1/W2] 无 Vulkan loader(VSM 内核真跑归 gate real)")
    if code != 0:
        print(blob[-2400:], file=sys.stderr)
        mark_w3_blocked(results)
        return fail("[device W1/W2] uc06-renderer VSM page-mark 对拍未过")
    missing = [name for name in expected if name not in blob]
    if missing:
        mark_w3_blocked(results)
        return fail(f"[device W1/W2] 测试不在集内: {missing}")
    results["device_wave_w1_pass"] = True
    results["device_wave_tests"] = expected
    print("[renderer_lighting_smoke] [device W1/W2] PASS: VSM page-mark 真跑对拍")
    mark_w3_blocked(results)
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "renderer_lighting_smoke",
        "milestone": "G5.3-D/E/F / G-G5-6 (RFC-0016 章 D/E/F)",
        "step": 85,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "device_blocked": results.get("device_blocked"),
        "device_probe_note": results.get("device_probe_note"),
        **{k: results[k] for k in (
            "device_wave_w1_pass", "device_wave_w2_pass", "device_wave_tests",
            "device_blocked_w3", "blocked_reason", "missing_toolchain_caps",
        ) if k in results},
        "checks": {k: results.get(k) for k in (
            "shadow_pass", "shadow_count", "gi_pass", "gi_count", "rt_pass", "rt_count",
        ) if results.get(k) is not None},
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": results.get("toolchain_skip") is not None,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"renderer_lighting_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[renderer_lighting_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    device_rc = device_section(results) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if not host_ok:
        return 1
    if device_rc != 0:
        return device_rc
    print("[renderer_lighting_smoke] PASS(host 恒跑 + device W1 gate real + W3 blocked-honest)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
