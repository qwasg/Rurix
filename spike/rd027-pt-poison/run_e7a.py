#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# SPIKE(RD-027):E7a compute-sanitizer memcheck 前置排除。
"""E7a:便宜且可能整案改道的前置排除(G-G3-1 ②)。

memcheck 包一个毒径档 + 一个对照档:
  - 对照档(ctrl_b2)memcheck clean → sanitizer 路径本身可用;
  - 毒径档(poison_b3)若报 OOB(如 cell 表异常 → je 垃圾巨值)→ 故事翻转为
    应用/数据缺陷(亦为合法归因,攻坚改道);真挂则拿部分输出如实记录。

sanitizer 大额度 1200s(proc_guard 分级);毒径档在 sanitizer 下可能:
  a) 依旧挂 → timeout 124 如实记录(sanitizer 拿不到完整报告);
  b) 被 sanitizer 扰动后完成 → 海森信号(指令布局敏感,后段嫌疑加权)。

用法:py -3 spike/rd027-pt-poison/run_e7a.py
"""
from __future__ import annotations

import json
import os
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from spike_common import (  # noqa: E402
    WORK, append_jsonl, campaign_header, canary, fail, log,
)
from bench.proc_guard import guarded_run  # noqa: E402

SANITIZER_TIMEOUT = 300  # spike 判定线:对照 memcheck 应 <60s,毒径 300s 足以判挂


def find_sanitizer() -> str | None:
    env = os.environ.get("COMPUTE_SANITIZER")
    if env and Path(env).is_file():
        return env
    cuda = os.environ.get("CUDA_PATH")
    if cuda:
        p = Path(cuda) / "compute-sanitizer" / "compute-sanitizer.exe"
        if p.is_file():
            return str(p)
    return shutil.which("compute-sanitizer")


def memcheck(tag: str, exe: Path, san: str) -> dict:
    rundir = WORK / "runs" / f"e7a_{tag}"
    rundir.mkdir(parents=True, exist_ok=True)
    r = guarded_run([san, "--tool", "memcheck", "--error-exitcode", "99", str(exe)],
                    timeout=SANITIZER_TIMEOUT, cwd=rundir,
                    quarantine_exe=exe, label=f"memcheck:{tag}")
    out = r.stdout + r.stderr
    errors_line = next((ln for ln in out.splitlines() if "ERROR SUMMARY" in ln), "")
    rec = {
        "kind": "memcheck", "name": tag, "exit_code": r.returncode,
        "timed_out": r.timed_out, "error_summary": errors_line.strip(),
        "oob_reported": ("Invalid" in out or "out of bounds" in out.lower()),
        "tail": out[-500:],
    }
    append_jsonl(rec)
    log(f"memcheck {tag}: exit={r.returncode} timed_out={r.timed_out} "
        f"summary={errors_line.strip() or 'n/a'}")
    return rec


def main() -> int:
    san = find_sanitizer()
    if san is None:
        fail("compute-sanitizer 不可定位(COMPUTE_SANITIZER/CUDA_PATH/PATH)")
    manifest = WORK / "e0a_built.json"
    if not manifest.is_file():
        fail("缺 e0a_built.json;先跑 run_e0a.py --build-only")
    built = {k: Path(v) for k, v in
             json.loads(manifest.read_bytes().decode("utf-8")).items()}
    for k, p in built.items():
        if not p.is_file():
            fail(f"变体 exe 缺失(可能已被隔离区收走): {p};先重跑 run_e0a.py --build-only")
    campaign_header("E7a", f"compute-sanitizer memcheck 前置排除(tool={san})")

    ctrl = memcheck("ctrl_b2", built["ctrl_b2"], san)
    if ctrl["timed_out"] or ctrl["exit_code"] not in (0,):
        log("WARN 对照档 memcheck 非 clean——sanitizer 路径存疑,毒径档结果仅供参考")
    poison = memcheck("poison_b3", built["poison_b3"], san)
    if poison["timed_out"]:
        canary(built["ctrl_b2"])
    verdict = ("app_data_suspect(OOB)" if poison["oob_reported"]
               else ("still_hangs_under_sanitizer" if poison["timed_out"]
                     else "completes_under_sanitizer(heisenbug_signal)"))
    append_jsonl({"kind": "e7a_summary", "verdict": verdict})
    log(f"E7a 判定:{verdict}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
