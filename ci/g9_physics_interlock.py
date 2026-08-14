#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.6 物理波门序机器阻断共享小库(spec/physics.md RXS-0375;RFC-0024 §4.B)。

RXS-0375 门序硬约束:「M121 完整期未绿,M122 完整期不得验收」。M122 完整期
门(ci/g9_gameplay_field_smoke.py --phase g9.6)前置机器核验:evidence/ 内
最新 ``g9_m121_physics_particle_view_<UTC>.json`` 必须
``assertion_id=="g9.p0.m121.physics_particle_view"`` 且 ``status=="pass"``
且 ``phase_g9_6_pass is True``——骨架期件(phase_g9_6_pass=false)不充绿,
缺失/不可读/非 pass/键不符即门 FAIL 退 1(打印阻断原因)。harness 直出件
(无 status/assertion_id 面)与他门件不充绿;非 UTC 文件名不参与取最新。

用法(库)::

    from g9_physics_interlock import m121_full_gate_passed
    ok, detail = m121_full_gate_passed()          # 默认 evidence/
    ok, detail = m121_full_gate_passed(Path(td))  # 指定目录(selftest 负样本)

用法(CLI)::

    py -3 ci/g9_physics_interlock.py            # 打印当前门序状态(退 0/1)
    py -3 ci/g9_physics_interlock.py --selftest # 4 RED + 1 GREEN 自检
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"

M121_GATE_KEY = "g9.p0.m121.physics_particle_view"
M121_SUBJECT_PREFIX = "g9_m121_physics_particle_view"

# UTC stamp in evidence filenames: g9_m121_physics_particle_view_YYYYMMDDTHHMMSSZ.json
_UTC_STAMP_RE = re.compile(r"_(\d{8}T\d{6}Z)\.json$")


def latest_m121_evidence(evidence_dir: Path | None = None) -> Path | None:
    """evidence/ 内按 UTC 文件名取 M121 门最新一份(同构 g9_gi_interlock)。"""
    base = evidence_dir if evidence_dir is not None else EVIDENCE_DIR
    if not base.is_dir():
        return None
    candidates: list[tuple[str, Path]] = []
    for p in base.glob(f"{M121_SUBJECT_PREFIX}_*.json"):
        m = _UTC_STAMP_RE.search(p.name)
        if m is None:
            continue
        candidates.append((m.group(1), p))
    if not candidates:
        return None
    candidates.sort(key=lambda t: t[0])
    return candidates[-1][1]


def m121_full_gate_passed(evidence_dir: Path | None = None) -> tuple[bool, str]:
    """RXS-0375 门序机器核验:返回 (放行?, 说明)。阻断说明统一以「门序阻断」起头。"""
    path = latest_m121_evidence(evidence_dir)
    if path is None:
        return False, (
            f"门序阻断(RXS-0375):缺 M121 门 evidence({M121_SUBJECT_PREFIX}_*.json)——"
            f"{M121_GATE_KEY} 完整期未绿前 M122 完整期不得验收"
        )
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        return False, f"门序阻断(RXS-0375):M121 门 evidence 不可读({path.name}: {e})"
    assertion = doc.get("assertion_id")
    if assertion != M121_GATE_KEY:
        return False, (
            f"门序阻断(RXS-0375):最新 {path.name} assertion_id={assertion!r} ≠ "
            f"{M121_GATE_KEY!r}(harness 直出件/他门件不充绿)"
        )
    status = doc.get("status")
    if status != "pass":
        return False, (
            f"门序阻断(RXS-0375):最新 {path.name} status={status!r} ≠ 'pass'"
        )
    if doc.get("phase_g9_6_pass") is not True:
        return False, (
            f"门序阻断(RXS-0375):最新 {path.name} phase_g9_6_pass="
            f"{doc.get('phase_g9_6_pass')!r} ≠ true(骨架期绿不替完整期充绿)"
        )
    return True, (
        f"M121 门最新 evidence {path.name} status=pass 且 phase_g9_6_pass=true"
        "(完整期门序前置满足)"
    )


def run_selftest() -> int:
    """4 RED(缺件/status=fail/harness 直出件/骨架期件)+ 1 GREEN(完整期 pass 件)。"""
    import tempfile

    reds = [
        ("缺件", None),
        (
            "status=fail",
            {"status": "fail", "assertion_id": M121_GATE_KEY, "phase_g9_6_pass": True},
        ),
        (
            "harness 直出件(无 status/assertion_id)",
            {"schema": "rurix.g9m121.field_solver_coupling.v1", "checks": {}},
        ),
        (
            "骨架期件(phase_g9_6_pass=false 不充绿)",
            {"status": "pass", "assertion_id": M121_GATE_KEY, "phase_g9_6_pass": False},
        ),
    ]
    for name, payload in reds:
        with tempfile.TemporaryDirectory(prefix="g9_physics_interlock_") as td:
            if payload is not None:
                p = Path(td) / f"{M121_SUBJECT_PREFIX}_20260101T000000Z.json"
                p.write_text(json.dumps(payload), encoding="utf-8")
            ok, detail = m121_full_gate_passed(Path(td))
            if ok or "门序阻断" not in detail:
                print(f"[g9_physics_interlock] selftest FAIL: {name} 未阻断", file=sys.stderr)
                return 1
    with tempfile.TemporaryDirectory(prefix="g9_physics_interlock_") as td:
        p = Path(td) / f"{M121_SUBJECT_PREFIX}_20260101T000000Z.json"
        p.write_text(
            json.dumps(
                {"status": "pass", "assertion_id": M121_GATE_KEY, "phase_g9_6_pass": True}
            ),
            encoding="utf-8",
        )
        ok, detail = m121_full_gate_passed(Path(td))
        if not ok:
            print(
                f"[g9_physics_interlock] selftest FAIL: 完整期 pass 件未放行({detail})",
                file=sys.stderr,
            )
            return 1
    print("[g9_physics_interlock] selftest PASS(4 RED + 1 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.6 物理波门序机器阻断(RXS-0375)")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    ok, detail = m121_full_gate_passed()
    print(detail)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
