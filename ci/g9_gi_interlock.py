#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.4 GI 波门序机器阻断(D2-Q7 硬约束)共享小库。

G9_ACCEPTANCE_MAP §2 M96 行:「门序硬约束(D2-Q7):本门未绿前 M97~M101 任何
画质门不得验收」。步骤 148~152 五门在 --gate 模式前置机器核验:evidence/ 内
最新 ``g9_m96_path_tracer_reference_<UTC>.json`` 必须 ``status=="pass"`` 且
``assertion_id=="g9.p0.m96.path_tracer_reference"``;缺失/不可读/非 pass/键
不符即门 FAIL 退 1(打印阻断原因)。harness 直出件(无 status/assertion_id
面)与他门件不充绿;非 UTC 文件名(如 ``*_voff.json``)不参与取最新。

用法(库,五门同构)::

    from g9_gi_interlock import m96_gate_passed
    ok, detail = m96_gate_passed()          # 默认 evidence/
    ok, detail = m96_gate_passed(Path(td))  # 指定目录(selftest 负样本)

用法(CLI)::

    py -3 ci/g9_gi_interlock.py            # 打印当前门序状态(退 0/1)
    py -3 ci/g9_gi_interlock.py --selftest # 3 RED + 1 GREEN 自检
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"

M96_GATE_KEY = "g9.p0.m96.path_tracer_reference"
M96_SUBJECT_PREFIX = "g9_m96_path_tracer_reference"

# UTC stamp in evidence filenames: g9_m96_path_tracer_reference_YYYYMMDDTHHMMSSZ.json
_UTC_STAMP_RE = re.compile(r"_(\d{8}T\d{6}Z)\.json$")


def latest_m96_evidence(evidence_dir: Path | None = None) -> Path | None:
    """evidence/ 内按 UTC 文件名取 M96 门最新一份(同构 g9_wave_exit_lib)。"""
    base = evidence_dir if evidence_dir is not None else EVIDENCE_DIR
    if not base.is_dir():
        return None
    candidates: list[tuple[str, Path]] = []
    for p in base.glob(f"{M96_SUBJECT_PREFIX}_*.json"):
        m = _UTC_STAMP_RE.search(p.name)
        if m is None:
            continue
        candidates.append((m.group(1), p))
    if not candidates:
        return None
    candidates.sort(key=lambda t: t[0])
    return candidates[-1][1]


def m96_gate_passed(evidence_dir: Path | None = None) -> tuple[bool, str]:
    """D2-Q7 机器核验:返回 (放行?, 说明)。阻断说明统一以「门序阻断」起头。"""
    path = latest_m96_evidence(evidence_dir)
    if path is None:
        return False, (
            f"门序阻断(D2-Q7):缺 M96 门 evidence({M96_SUBJECT_PREFIX}_*.json)——"
            f"{M96_GATE_KEY} 未绿前 M97~M101 画质门不得验收"
        )
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        return False, f"门序阻断(D2-Q7):M96 门 evidence 不可读({path.name}: {e})"
    assertion = doc.get("assertion_id")
    if assertion != M96_GATE_KEY:
        return False, (
            f"门序阻断(D2-Q7):最新 {path.name} assertion_id={assertion!r} ≠ "
            f"{M96_GATE_KEY!r}(harness 直出件/他门件不充绿)"
        )
    status = doc.get("status")
    if status != "pass":
        return False, (
            f"门序阻断(D2-Q7):最新 {path.name} status={status!r} ≠ 'pass'"
        )
    return True, f"M96 门最新 evidence {path.name} status=pass(门序前置满足)"


def run_selftest() -> int:
    """3 RED(缺件/status=fail/harness 直出件)+ 1 GREEN(合成 pass 件)。"""
    import tempfile

    reds = [
        ("缺件", None),
        ("status=fail", {"status": "fail", "assertion_id": M96_GATE_KEY}),
        (
            "harness 直出件(无 status/assertion_id)",
            {"schema": "rurix.g9m96.path_tracer.v1", "checks": {}},
        ),
    ]
    for name, payload in reds:
        with tempfile.TemporaryDirectory(prefix="g9_gi_interlock_") as td:
            if payload is not None:
                p = Path(td) / f"{M96_SUBJECT_PREFIX}_20260101T000000Z.json"
                p.write_text(json.dumps(payload), encoding="utf-8")
            ok, detail = m96_gate_passed(Path(td))
            if ok or "门序阻断" not in detail:
                print(f"[g9_gi_interlock] selftest FAIL: {name} 未阻断", file=sys.stderr)
                return 1
    with tempfile.TemporaryDirectory(prefix="g9_gi_interlock_") as td:
        p = Path(td) / f"{M96_SUBJECT_PREFIX}_20260101T000000Z.json"
        p.write_text(
            json.dumps({"status": "pass", "assertion_id": M96_GATE_KEY}),
            encoding="utf-8",
        )
        ok, detail = m96_gate_passed(Path(td))
        if not ok:
            print(
                f"[g9_gi_interlock] selftest FAIL: 合成 pass 件未放行({detail})",
                file=sys.stderr,
            )
            return 1
    print("[g9_gi_interlock] selftest PASS(3 RED + 1 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.4 GI 波门序机器阻断(D2-Q7)")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    ok, detail = m96_gate_passed()
    print(detail)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
