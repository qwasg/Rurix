# -*- coding: utf-8 -*-
"""入门教程示例端到端冒烟(guide/ 配套,conformance/tutorial/*.rx)。

用法:
    py -3 ci/tutorial_smoke.py            # 真跑 check + run,写 evidence
    py -3 ci/tutorial_smoke.py --no-emit  # 同上但不写 evidence(本地快速核对)

机制:构建 debug rx,对 conformance/tutorial/ 下每个教程示例逐个端到端真跑——
- rx check <ex>            → 0(全量前端静态检查通过,RXS-0086)
- rx run   <ex> -o <exe>   → 0(构建并执行 host 产物,退出码透传,RXS-0084/0085)

教程示例只用 `rx check` 可独立解析的语言面(host / device fn / const fn /
kernel 定义);launch、运行时资源(Context/Stream/Buffer)、stdlib 数学等需包
上下文,在 guide 正文以参考片段呈现,不入本冒烟集。

任一示例任一子命令端到端失败(非零退出)即整体 FAIL(反 YAML-only)。这道门
保证教程里展示的代码随工具链/语言面演进始终真实可编译(防 bit-rot,治理三角)。
成功示例集写 evidence/tutorial_smoke_<yyyymmdd_hhmmss>.json。
"""
from __future__ import annotations

import datetime
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")
TUTORIAL_DIR = ROOT / "conformance" / "tutorial"
OUT_DIR = ROOT / "build" / "tutorial_smoke"


def run(cmd):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)


def unique_evidence_path() -> Path:
    base = ROOT / "evidence"
    base.mkdir(parents=True, exist_ok=True)
    stem = f"tutorial_smoke_{datetime.datetime.now():%Y%m%d_%H%M%S}"
    out = base / f"{stem}.json"
    n = 1
    while out.exists():
        out = base / f"{stem}_{n}.json"
        n += 1
    return out


def main() -> int:
    emit = "--no-emit" not in sys.argv
    build = subprocess.run(
        ["cargo", "build", "-p", "rx"], cwd=ROOT, capture_output=True, text=True
    )
    if build.returncode != 0:
        print(f"[tutorial_smoke] FAIL: cargo build -p rx 失败:\n{build.stderr}")
        return 1
    if not RX.is_file():
        print(f"[tutorial_smoke] FAIL: rx 产物不存在: {RX}")
        return 1
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    examples = sorted(TUTORIAL_DIR.glob("*.rx"))
    if not examples:
        print(f"[tutorial_smoke] FAIL: 未发现教程示例: {TUTORIAL_DIR}")
        return 1

    passed: list[str] = []
    facts: list[dict] = []
    failures: list[str] = []
    for ex in examples:
        rel = ex.relative_to(ROOT).as_posix()
        stem = ex.stem
        cases = [
            ("check", [str(RX), "check", rel], 0),
            ("run", [str(RX), "run", rel, "-o", str(OUT_DIR / f"{stem}.exe")], 0),
        ]
        ex_ok = True
        for name, argv, want in cases:
            proc = run(argv)
            ok = proc.returncode == want
            facts.append({
                "example": rel,
                "subcommand": name,
                "exit_code": proc.returncode,
                "note": "PASS" if ok else (proc.stderr.strip() or proc.stdout.strip())[:300],
            })
            if not ok:
                ex_ok = False
                failures.append(
                    f"{rel} [{name}]: 期望退出 {want} 实际 {proc.returncode}\n"
                    f"  stdout: {proc.stdout.strip()[:200]}\n  stderr: {proc.stderr.strip()[:200]}"
                )
        if ex_ok:
            passed.append(rel)

    if emit:
        doc = {
            "schema_version": 1,
            "subject": "tutorial_examples_smoke",
            "examples_passed": sorted(passed),
            "rx_binary": str(RX.relative_to(ROOT)).replace("\\", "/"),
            "facts": facts,
            "timestamp": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        }
        out = unique_evidence_path()
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[tutorial_smoke] evidence 写入 {out.relative_to(ROOT)}")

    if failures:
        print(f"[tutorial_smoke] FAIL ({len(failures)} 个失败用例)")
        for line in failures:
            print(f"  - {line}")
        return 1
    print(f"[tutorial_smoke] PASS ({len(passed)}/{len(examples)} 个教程示例端到端真跑: "
          f"{', '.join(Path(p).name for p in passed)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
