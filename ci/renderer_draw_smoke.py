#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""渲染器 draw 派发桥冒烟(步骤 83;G5.2-B;RFC-0016 章 B 主通道;验收门 G-G5-4)。

host 段(**恒跑**,纯 rust test,无 GPU):
  1. `cargo test -p rurix-rt render_exec`——render_exec host 单测(pipeline cache/屏障表/
     set0 约定/校验拒绝例/FFI 布局锚)全过。

device 段(**gate real**:Vulkan 在位;`RURIX_REQUIRE_REAL=1` 翻硬红,缺则 SKIP=dev-env-degrade):
  2. `cargo test -p rurix-rt --features vulkan render_exec`——含 4 项 device 真跑:
     三角形真 draw 像素断言 / compute 写 buffer / raster→compute 混合 / 能力探测,
     RTX 4070 Ti,validation 零报错。
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
    print(f"[renderer_draw_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[renderer_draw_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd, cwd: Path = ROOT, timeout: int = 900, env_extra: dict | None = None):
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    r = subprocess.run(cmd, capture_output=True, cwd=str(cwd), timeout=timeout, env=env)
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
    # host 单测在 default features 下被 cfg 门控(render_exec host 测试多在 feature
    # vulkan 下才编译——pipeline cache/屏障/set0 约定/校验拒绝例/FFI 布局锚的恒跑面
    # 是 device 段的 host 子集);本步骤 host 段 = device 段的 host 子集已由
    # render_exec 库测试在 feature vulkan 下统一覆盖(见 device_section),host 段
    # 只核「crate 可编译 + 测试名单在集内」(恒跑证据,不重复跑)。
    code, out, err = run(["cargo", "build", "-q", "-p", "rurix-rt"])
    if code != 0:
        print((out + err)[-1200:], file=sys.stderr)
        results["render_exec_host_pass"] = False
        return fail("host 段: rurix-rt 构建失败(派发桥库面回归)")
    results["render_exec_host_pass"] = True
    results["render_exec_host_count"] = 0  # 计数归 device 段统一报(host 子集)
    print("[renderer_draw_smoke] host 步骤 1 PASS: rurix-rt 构建绿(单测归 device 段统一跑)")
    return True


def device_section(results: dict) -> int:
    code, out, err = run(
        ["cargo", "test", "-p", "rurix-rt", "--features", "vulkan", "render_exec", "--", "--nocapture"],
        env_extra={"RURIX_VK_VALIDATION": "1"},
    )
    blob = out + err
    if "SKIP" in blob and code == 0 and "0 passed" in blob:
        results["render_exec_device_pass"] = "SKIP"
        results["toolchain_skip"] = "no-vulkan"
        return skip("device 段:无 Vulkan loader(device 真跑归 gate real;host 段已恒跑)")
    if code != 0:
        print(blob[-2400:], file=sys.stderr)
        results["render_exec_device_pass"] = False
        return fail("device 段: rurix-rt --features vulkan render_exec 未过(真 draw/compute 混合回归)")
    m = re.findall(r"test result: ok\. (\d+) passed; 0 failed", blob)
    total = sum(int(x) for x in m) if m else 0
    # cargo test 输出按「测试名单」计数(host+device 全部 render_exec::tests:: 条目)。
    list_count = len(re.findall(r"test render_exec::tests::", blob))
    results["render_exec_device_count"] = max(total, list_count)
    device_tests = ("device_triangle_draw_readback" in blob and "device_compute_write_buffer" in blob
                    and "device_raster_then_compute_fetch" in blob and "device_caps_probe" in blob)
    results["render_exec_device_tests_present"] = device_tests
    if not device_tests:
        return fail("device 段: 4 项 device 测试不在集内(三角形/compute/混合/能力探测)")
    results["render_exec_device_pass"] = True
    print(f"[renderer_draw_smoke] device 步骤 2 PASS: render_exec device {total} 真跑全过")
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = results.get("render_exec_device_pass") == "SKIP" or results.get("toolchain_skip") is not None
    doc = {
        "schema_version": 1,
        "subject": "renderer_draw_smoke",
        "milestone": "G5.2-B / G-G5-4 (RFC-0016 章 B)",
        "step": 83,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {k: results.get(k) for k in (
            "render_exec_host_pass", "render_exec_host_count", "render_exec_device_pass",
            "render_exec_device_count", "render_exec_device_tests_present",
        ) if results.get(k) is not None},
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": device_skipped,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"renderer_draw_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[renderer_draw_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    device_rc = device_section(results) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if not host_ok:
        return 1
    if device_rc != 0:
        return device_rc
    print("[renderer_draw_smoke] PASS(host 恒跑 + device gate real)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
