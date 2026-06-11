"""PR Smoke 步骤 1:仓库一等公民目录存在性核对(10 §4 / CI_GATES.md §3.1)。"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

REQUIRED_DIRS = [
    "spec",
    "rfcs",
    "conformance",
    "tests/ui",
    "unsafe-audit",
    "agents",
    "evidence",
    "registry",
    "milestones/m0",
    "ci",
    "bench",
]

REQUIRED_FILES = [
    "agents/AGENTS.md",
    "registry/deferred.json",
    "registry/spike_gating.json",
    "milestones/m0/M0_CONTRACT.md",
    "milestones/m0/m0_budget.json",
    "milestones/m0/evidence_schema.json",
]


def main() -> int:
    errors: list[str] = []
    for d in REQUIRED_DIRS:
        if not (ROOT / d).is_dir():
            errors.append(f"missing directory: {d}")
    for f in REQUIRED_FILES:
        if not (ROOT / f).is_file():
            errors.append(f"missing file: {f}")
    if errors:
        print("[check_structure] FAIL")
        for e in errors:
            print(f"  - {e}")
        return 1
    print(f"[check_structure] PASS ({len(REQUIRED_DIRS)} dirs, {len(REQUIRED_FILES)} files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
