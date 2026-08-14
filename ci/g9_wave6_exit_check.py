#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G9.6 波次聚合门 g9.wave.6.exit(步骤 169;milestones/g9/CI_GATES §6 v1.18)。

只读汇总 G9.6 波五门最新 evidence——两 P0 完整期(M121/M122,步骤 136/137
同 step 双 phase:聚合核验最新件 status=="pass" 且 phase_g9_6_pass==true,
骨架期绿不替完整期充绿)+ 三 P1(M124 步骤 166 / M126 步骤 167 / M125
步骤 168)+ spec/physics.md 在树且 RXS-0374~0379 条款头齐(RXS-0376 条款头
「解析浮力走 Field 通道」字面核验)+ RFC-0024 v1.1 章 F1/F2 字面在树
+ M123 no-go counts_as_green=false 诚实登记字面(spec RXS-0379 +
G9_CANDIDATE_DECISIONS v1.5 校准注 + MAP M123 no-go 登记句且 §3 零 m123 key)
+ M125 verdict=maintain_5_3_default + 5.3 基线 0-byte 事实(g9_m125_jolt56_ab.json
verdict 机核 + src/rurix-physics-sys/VENDOR.md 5.3 pin 字面不动核验)
+ M126 RD-044 maintain_no_go 登记字面(g9_m126_rapier_benchmark.json rd044 面机核)
+ 门序 interlock(ci/g9_physics_interlock.py——M121 完整期 full pass 前 M122
完整期不得绿;M122 最新件 checks.gate_order_m121_full_passed==true)。
不重跑 smoke、不代绿、不设 RURIX_REQUIRE_REAL。聚合 PASS 不遮蔽任一子断言
FAIL/SKIP/DEV_ENV_DEGRADE。

用法:
  py -3 ci/g9_wave6_exit_check.py --gate g9.wave.6.exit
  py -3 ci/g9_wave6_exit_check.py --selftest
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# 允许同目录 import
sys.path.insert(0, str(Path(__file__).resolve().parent))

import g9_wave_exit_lib as wel  # noqa: E402
import g9_physics_interlock as pi  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g9.wave.6.exit"
NUMERIC_STEP = 169
SUBJECT = "g9_wave6_exit"
WAVE = "G9.6"
SOURCE_REF = (
    "milestones/g9/CI_GATES §6 v1.18;G9_CONTRACT §8.6;G9_ACCEPTANCE_MAP §2/§3;"
    "RFC-0024 v1.1 章 F1/F2;RXS-0374~0379 clause heads on tree;"
    "M123 no-go counts_as_green=false registered (not in MAP §3);"
    "M125 verdict=maintain_5_3_default + 5.3 baseline 0-byte;"
    "M126 RD-044 maintain_no_go registered;M121 full before M122 full interlock"
)
SCHEMA_PATH = ROOT / "milestones" / "g9" / "g9_wave6_exit_evidence_schema.json"
RFC0024 = ROOT / "rfcs" / "0024-physics-platform-revision.md"
SPEC_PHYS = ROOT / "spec" / "physics.md"
CANDIDATE = ROOT / "milestones" / "g9" / "G9_CANDIDATE_DECISIONS.md"
ACCEPTANCE_MAP = ROOT / "milestones" / "g9" / "G9_ACCEPTANCE_MAP.md"
M125_REPORT = ROOT / "milestones" / "g9" / "g9_m125_jolt56_ab.json"
M126_REPORT = ROOT / "milestones" / "g9" / "g9_m126_rapier_benchmark.json"
VENDOR_53 = ROOT / "src" / "rurix-physics-sys" / "VENDOR.md"

# 五个 G9.6 门:(symbolic_key, evidence subject_prefix, 完整期双 phase?)——
# 两 P0(M121/M122,步骤 136/137 同 step 双 phase:聚合核验最新件完整期绿)
# + 三 P1(§4A,G9_CONTRACT §8.1 裁决① P1 全进)。门 key 按 G9_ACCEPTANCE_MAP
# §2/§3 实记。聚合门只核各门最新一份 evidence 的 PASS,不重跑 smoke、不代绿。
REQUIRED_GATES: list[tuple[str, str, bool]] = [
    ("g9.p0.m121.physics_particle_view", "g9_m121_physics_particle_view", True),
    ("g9.p0.m122.gameplay_field", "g9_m122_gameplay_field", True),
    ("g9.p1.m124.buoyancy_field_channel", "g9_m124_buoyancy_field_channel", False),
    ("g9.p1.m125.jolt_56_ab_evaluation", "g9_m125_jolt_56_ab_evaluation", False),
    ("g9.p1.m126.rapier_benchmark_ab", "g9_m126_rapier_benchmark_ab", False),
]

_RXS_HEAD_RE = re.compile(r"^###\s+RXS-(\d{4})\b", re.MULTILINE)


def _fact(fid: str, ok: bool, detail: str) -> dict:
    return {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}


def _rxs_heads(path: Path) -> set[int]:
    if not path.is_file():
        return set()
    return {int(m) for m in _RXS_HEAD_RE.findall(path.read_text(encoding="utf-8"))}


def _require_full_phase_gate_pass(
    key: str, subject_prefix: str, evidence_dir: Path | None = None
) -> dict:
    """M121/M122 完整期聚合核验:最新件 wel.gate_pass_reason 全过 + status=="pass"
    + phase_g9_6_pass==true(骨架期绿不替完整期充绿,双 phase 同 step 取最新件)。"""
    row = wel.require_gate_pass(key, subject_prefix, evidence_dir=evidence_dir)
    if row["status"] != "PASS":
        return row
    path = wel.load_latest_evidence(subject_prefix, evidence_dir=evidence_dir)
    try:
        doc = wel.load_json(path) if path is not None else {}
    except (OSError, ValueError) as e:
        row["status"] = "FAIL"
        row["detail"] = f"evidence 不可读: {e}"
        return row
    if doc.get("status") != "pass":
        row["status"] = "FAIL"
        row["detail"] = f"status={doc.get('status')!r} ≠ 'pass'"
        return row
    if doc.get("phase_g9_6_pass") is not True:
        row["status"] = "FAIL"
        row["detail"] = (
            f"phase_g9_6_pass={doc.get('phase_g9_6_pass')!r} ≠ true"
            "(骨架期绿不替完整期充绿)"
        )
        return row
    row["detail"] = "PASS(status=pass 且 phase_g9_6_pass=true,完整期)"
    return row


def collect_extra_facts(evidence_dir: Path | None = None) -> list[dict]:
    facts: list[dict] = []

    # ① spec/physics.md 在树且 RXS-0374~0379 条款头齐 + RXS-0376 条款头
    # 「解析浮力走 Field 通道」字面(M124 浮力 Field 通道字面核验)。
    heads = _rxs_heads(SPEC_PHYS)
    want = {374, 375, 376, 377, 378, 379}
    missing = sorted(want - heads)
    spec_text = SPEC_PHYS.read_text(encoding="utf-8") if SPEC_PHYS.is_file() else ""
    buoyancy_literal = "解析浮力走 Field 通道" in spec_text
    bad_1: list[str] = []
    if not SPEC_PHYS.is_file():
        bad_1.append("spec/physics.md 不在树")
    if missing:
        bad_1.append(f"缺条款头: {missing}")
    if not buoyancy_literal:
        bad_1.append("RXS-0376 缺「解析浮力走 Field 通道」字面")
    facts.append(
        _fact(
            "rxs0374_0379_clause_heads_on_tree",
            not bad_1,
            (
                "spec/physics.md 在树,RXS-0374~0379 条款头全在树(共 6 枚),"
                "RXS-0376 条款头「解析浮力走 Field 通道」字面在树(M124 Field 通道)"
                if not bad_1
                else "; ".join(bad_1)
            ),
        )
    )

    # ② RFC-0024 v1.1 章 F1/F2 字面在树。
    rfc_text = RFC0024.read_text(encoding="utf-8") if RFC0024.is_file() else ""
    f1 = "### F1 🔒 M123 双通道判档落定" in rfc_text
    f2 = "### F2 🔒 World-Field GpuScene 只读扩面显式修订行" in rfc_text
    facts.append(
        _fact(
            "rfc0024_v1_1_chapter_f_literals",
            f1 and f2,
            (
                "RFC-0024 v1.1 章 F1(M123 双通道判档落定 no-go)/F2(World-Field "
                "GpuScene 只读扩面显式修订行)字面在树"
                if f1 and f2
                else f"章 F 字面缺失: F1={f1} F2={f2}"
            ),
        )
    )

    # ③ M123 no-go counts_as_green=false 诚实登记字面:spec RXS-0379 L1
    # 「counts_as_green=false」+ CANDIDATE v1.5 校准注「M123 双通道判档 =
    # no-go 不充绿」+ MAP M123 no-go 登记句 + MAP §3 零 m123 key(no-go 不入表)。
    cand_text = CANDIDATE.read_text(encoding="utf-8") if CANDIDATE.is_file() else ""
    map_text = ACCEPTANCE_MAP.read_text(encoding="utf-8") if ACCEPTANCE_MAP.is_file() else ""
    bad_3: list[str] = []
    if "counts_as_green=false" not in spec_text:
        bad_3.append("spec/physics.md RXS-0379 缺 counts_as_green=false 字面")
    if "M123 双通道判档 = no-go 不充绿" not in cand_text:
        bad_3.append("G9_CANDIDATE_DECISIONS 缺 v1.5 校准注 M123 no-go 字面")
    if "M123 双通道判档 = no-go 不充绿" not in map_text:
        bad_3.append("G9_ACCEPTANCE_MAP 缺 M123 no-go 登记句")
    if "g9.p0.m123" in map_text or "g9.p1.m123" in map_text:
        bad_3.append("MAP 出现 m123 gate key(no-go 不入 §3 纪律破坏)")
    facts.append(
        _fact(
            "m123_no_go_counts_as_green_false_registered",
            not bad_3,
            (
                "M123 no-go 诚实登记三面一致: spec RXS-0379 counts_as_green=false"
                "(证据非空不充绿)+ CANDIDATE v1.5 校准注 + MAP 登记句;"
                "no-go 不入 MAP §3(零 m123 key),承接锚 G9.7 穷举"
                if not bad_3
                else "; ".join(bad_3)
            ),
        )
    )

    # ④ M125 verdict=maintain_5_3_default + 5.3 基线 0-byte 事实:
    # g9_m125_jolt56_ab.json verdict 机核 + src/rurix-physics-sys/VENDOR.md
    # 5.3 pin 字面(JoltC/JoltPhysics 双 commit)不动核验。
    bad_4: list[str] = []
    verdict_125: str | None = None
    if not M125_REPORT.is_file():
        bad_4.append(f"缺 {M125_REPORT.name}")
    else:
        try:
            rep = wel.load_json(M125_REPORT)
        except (OSError, ValueError):
            rep = {}
            bad_4.append(f"{M125_REPORT.name} 不可读")
        v = rep.get("verdict")
        verdict_125 = v.get("verdict") if isinstance(v, dict) else None
        if verdict_125 != "maintain_5_3_default":
            bad_4.append(f"verdict={verdict_125!r} ≠ maintain_5_3_default")
    v53_text = VENDOR_53.read_text(encoding="utf-8") if VENDOR_53.is_file() else ""
    pin_ok = (
        "2982004387a9e36ca89525a87d983709d3666da7" in v53_text
        and "0373ec0dd762e4bc2f6acdb08371ee84fa23c6db" in v53_text
    )
    if not pin_ok:
        bad_4.append("5.3 VENDOR.md pin 字面漂移(基线 0-byte 破坏)")
    facts.append(
        _fact(
            "m125_verdict_maintain_5_3_default_baseline_0byte",
            not bad_4,
            (
                "g9_m125_jolt56_ab.json verdict=maintain_5_3_default(评估完成不升格"
                "默认)+ src/rurix-physics-sys/VENDOR.md 5.3 基线 pin 字面不动"
                "(5.6 独立 vendor 并存不覆盖 5.3 基线)"
                if not bad_4
                else "; ".join(bad_4)
            ),
        )
    )

    # ⑤ M126 RD-044 maintain_no_go 登记字面:g9_m126_rapier_benchmark.json
    # rd044.verdict==maintain_no_go 且 condition_literal_unchanged==true。
    bad_5: list[str] = []
    if not M126_REPORT.is_file():
        bad_5.append(f"缺 {M126_REPORT.name}")
    else:
        try:
            rep = wel.load_json(M126_REPORT)
        except (OSError, ValueError):
            rep = {}
            bad_5.append(f"{M126_REPORT.name} 不可读")
        rd = rep.get("rd044")
        if not isinstance(rd, dict):
            bad_5.append("report 缺 rd044 面")
        else:
            if rd.get("verdict") != "maintain_no_go":
                bad_5.append(f"rd044.verdict={rd.get('verdict')!r} ≠ maintain_no_go")
            if rd.get("condition_literal_unchanged") is not True:
                bad_5.append("rd044.condition_literal_unchanged ≠ true(RD-044 字面漂移)")
    facts.append(
        _fact(
            "m126_rd044_maintain_no_go_registered",
            not bad_5,
            (
                "g9_m126_rapier_benchmark.json rd044.verdict=maintain_no_go 且 "
                "condition_literal_unchanged=true(RD-044 字面不变维持 open-留档,"
                "不升格深造、不作验收依赖与生产默认)"
                if not bad_5
                else "; ".join(bad_5)
            ),
        )
    )

    # ⑥ 门序 interlock(RXS-0375):ci/g9_physics_interlock.py 机器核验 M121
    # 完整期最新件 status==pass 且 phase_g9_6_pass==true + M122 最新件
    # checks.gate_order_m121_full_passed==true。
    ok_il, detail_il = pi.m121_full_gate_passed(evidence_dir)
    m122_checks = False
    path_122 = wel.load_latest_evidence("g9_m122_gameplay_field", evidence_dir=evidence_dir)
    if path_122 is not None:
        try:
            doc_122 = wel.load_json(path_122)
        except (OSError, ValueError):
            doc_122 = {}
        checks_122 = doc_122.get("checks")
        m122_checks = isinstance(checks_122, dict) and checks_122.get("gate_order_m121_full_passed") is True
    facts.append(
        _fact(
            "gate_order_m121_full_before_m122_full",
            ok_il and m122_checks,
            (
                f"门序 interlock 留痕: {detail_il};M122 门最新 evidence "
                "checks.gate_order_m121_full_passed=true(RXS-0375:M121 完整期未绿 "
                "M122 完整期不得验收)"
                if ok_il and m122_checks
                else f"interlock={detail_il};M122 checks.gate_order_m121_full_passed={m122_checks}"
            ),
        )
    )
    return facts


def run_gate(*, evidence_dir: Path | None = None) -> int:
    rows = [
        (
            _require_full_phase_gate_pass(key, prefix, evidence_dir=evidence_dir)
            if full_phase
            else wel.require_gate_pass(key, prefix, evidence_dir=evidence_dir)
        )
        for key, prefix, full_phase in REQUIRED_GATES
    ]
    extras = collect_extra_facts(evidence_dir=evidence_dir)
    notes_parts = [
        "implemented: five G9.6 gates (P0 M121/M122 full-phase dual-leg + P1 M124/M125/M126)",
        "M121/M122 aggregate ruling: latest evidence status==pass and phase_g9_6_pass==true "
        "(skeleton-phase green does not substitute full phase; dual phase same numeric step)",
        "aggregate read-only: no smoke re-run, no substitute green, no RURIX_REQUIRE_REAL",
        "facts: RXS-0374~0379 clause heads + RFC-0024 v1.1 ch.F1/F2 literals + "
        "M123 no-go counts_as_green=false registered (not in MAP §3) + "
        "M125 verdict=maintain_5_3_default + 5.3 baseline 0-byte + "
        "M126 RD-044 maintain_no_go + gate-order interlock (M121 full before M122 full)",
        "aggregate PASS does not mask any child FAIL/SKIP/DEV_ENV_DEGRADE",
    ]
    code, _path = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=rows,
        extra_facts=extras,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes="; ".join(notes_parts),
        host_section_pass=True,  # 由 emit 内 overall 覆盖
    )
    return code


def run_selftest() -> int:
    """① 缺五门 evidence → 红;② 真树五门绿 + 事实核验 → 绿。"""
    print("[selftest] 负样本:空 evidence 目录")
    import tempfile
    from pathlib import Path as P

    with tempfile.TemporaryDirectory(prefix="g9_wave6_selftest_") as td:
        code = run_gate(evidence_dir=P(td))
        if code == 0:
            print("[selftest] FAIL: 缺 evidence 仍绿", file=sys.stderr)
            return 1
        print("[selftest] PASS: 缺 evidence → 红")

    print("[selftest] 正样本:仓库最新五门 evidence")
    code = run_gate(evidence_dir=None)
    if code != 0:
        print("[selftest] FAIL: 真树聚合未绿(前置五门/事实核验未满足)", file=sys.stderr)
        return 1
    print("[selftest] PASS: 真树聚合绿")
    print("[selftest] ALL PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="G9.6 wave6.exit 聚合门(只读汇总)")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY], help="跑聚合门")
    g.add_argument("--selftest", action="store_true", help="负/正样本自检")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
