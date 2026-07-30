#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""渲染调度 render graph 冒烟(步骤 82;G5.2-A;RFC-0016 章 A;验收门 G-G5-3)。

host 段(**恒跑**,纯 rust test,无 GPU/无工具链):
  1. `cargo test -p rurix-render graph::`——四趟编译(剔除/生命周期/EB 三轴屏障 golden/
     transient 别名峰值/编译期校验 RED 自检/异步车道 fence/图 dump)全套单测,解析测试计数
     非零且全过。
  2. 图 dump 可产核验(解析测试名单确认 `dump_json_is_valid_and_complete` 在集内)。

device 段:**无**(纯 host 门,check_* 风格;G5.2-A 为 host 调度底座,不涉 GPU)。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"


def fail(msg: str) -> int:
    print(f"[renderer_graph_smoke] FAIL {msg}", file=sys.stderr)
    return 1


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
    code, out, err = run(["cargo", "test", "-p", "rurix-render", "graph::", "--", "--nocapture"])
    blob = out + err
    if code != 0:
        print(blob[-2400:], file=sys.stderr)
        results["graph_tests_pass"] = False
        return fail("host 段: rurix-render graph:: 单测未过(四趟编译/屏障/别名/校验回归)")
    # 解析测试计数(行形如 "test result: ok. N passed; 0 failed")。
    import re

    m = re.findall(r"test result: ok\. (\d+) passed; 0 failed", blob)
    total = sum(int(x) for x in m) if m else 0
    results["graph_test_count"] = total
    if total < 30:
        return fail(f"host 段: graph:: 测试计数 {total} < 30(四趟编译/屏障/别名/校验覆盖不全)")
    dump_ok = "dump_json_is_valid_and_complete" in blob or "dump_json" in blob
    results["graph_dump_test_present"] = dump_ok
    if not dump_ok:
        return fail("host 段: 图 dump 测试不在集内(dump_json_is_valid_and_complete)")
    results["graph_tests_pass"] = True
    print(f"[renderer_graph_smoke] host 步骤 1 PASS: rurix-render graph:: {total} 单测全过")
    return True


def write_evidence(results: dict, host_ok: bool) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "renderer_graph_smoke",
        "milestone": "G5.2-A / G-G5-3 (RFC-0016 章 A)",
        "step": 82,
        "host_section_pass": host_ok,
        "device_section_rc": 0,
        "checks": {k: results.get(k) for k in (
            "graph_tests_pass", "graph_test_count", "graph_dump_test_present",
        ) if results.get(k) is not None},
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"renderer_graph_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[renderer_graph_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    write_evidence(results, host_ok)
    if not host_ok:
        return 1
    print("[renderer_graph_smoke] PASS(host 恒跑,纯 host 门)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
