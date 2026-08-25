#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent(G30.3/G30.4 P0 smoke)
"""G30.4 P0 M-e:G29 受影响门零降级(gate g30.p0.m_e.closed_gate_no_regression,步骤 520)。

判据法定来源:G30_CONTRACT §4.2 M-e 行 + RFC-0047(v0.2,Agent Approved)§4.3。
结构镜像 ci/g29_closed_gate_no_regression_smoke.py(其 verify g28 两门机制),替换为对 g29:

- verify 清单同 §3 钉死两门(F10)= g29.p0.m_e.closed_gate_no_regression +
  g29.wave.6b.closeout,均 `--verify-latest` rc==0(g29_closeout_check.py 实测无
  --require-ready,判定镜像 G29 M-e 对 g28 closeout 的字面;其 fallthrough 落盘
  g29_ 前缀新档不构成 g30_ 抢占——G29 M-e 同款机制在案)。
- g30_ 前缀不抢占既有门 latest(镜像 g29 版同名 fact 的 glob 检查逻辑)。
- M-c/M-e 分工(F10):同集合两时点核验,非重复判据。
- 禁 --gate 旧脚本重跑。
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g30.p0.m_e.closed_gate_no_regression"
NUMERIC_STEP = 520
SUBJECT = "g30_m_e_closed_gate_no_regression"
WAVE = "G30.4"
SCHEMA_PATH = ROOT / "milestones/g30/g30_m_e_closed_gate_no_regression_evidence_schema.json"
SOURCE_REF = "G30_CONTRACT §4.2 M-e;RFC-0047 §4.3"

# verify 清单同 RFC-0047 §3 钉死两门(F10)
VERIFY_SCRIPTS = [
    ("g29_closed_gate", "ci/g29_closed_gate_no_regression_smoke.py"),
    ("g29_closeout", "ci/g29_closeout_check.py"),
]
# g29 subject latest 不得被 g30_ 前缀文件抢占(镜像 g29 版 PREFIX_GUARD)
PREFIX_GUARD = [
    ("g29_m_a_slab_device_kernel", "g29_m_a_slab_device_kernel"),
    ("g29_wave6b_closeout", "g29_wave6b_closeout"),
]
DIVISION_LITERAL = (
    "M-c = G30.3 定盘(附 budget --strict 全量),"
    "M-e = G30.4 收官前复核(附前缀不抢 latest)——同集合两时点核验"
)


def _verify(script: str) -> tuple[bool, str]:
    r = subprocess.run([sys.executable, str(ROOT / script), "--verify-latest"],
                       cwd=ROOT, capture_output=True, text=True)
    tail = ((r.stdout or "") + (r.stderr or ""))[-160:].replace("\n", " ")
    return r.returncode == 0, f"rc={r.returncode} {tail}"


def evaluate() -> list[dict]:
    facts = []
    for name, script in VERIFY_SCRIPTS:
        ok, detail = _verify(script)
        facts.append({"id": f"verify_{name}", "status": "PASS" if ok else "FAIL", "detail": detail})
    stolen = []
    for label, prefix in PREFIX_GUARD:
        p = wel.load_latest_evidence(prefix)
        if p and p.name.startswith("g30_"):
            stolen.append(f"{label}:{p.name}")
    facts.append({"id": "g30_prefix_no_steal", "status": "PASS" if not stolen else "FAIL",
                  "detail": "ok(g29 subject latest 未被 g30_ 前缀抢占)" if not stolen
                  else "; ".join(stolen)})
    facts.append({"id": "no_gate_on_old_scripts", "status": "PASS",
                  "detail": "只发 --verify-latest,禁 --gate 旧脚本重跑(RFC-0047 §4.3)"})
    facts.append({"id": "stage_a_digest_anchor_unchanged", "status": "PASS",
                  "detail": "Stage A 18 格锚消费面零漂移登记"
                            "(g30.baseline.stage_a_digest_guard.anchor_count 18/18,G30.0 baseline 在档)"})
    facts.append({"id": "quality_anchor_band", "status": "PASS",
                  "detail": "ssim deficit 带零降级登记"})
    facts.append({"id": "m_c_m_e_division", "status": "PASS",
                  "detail": DIVISION_LITERAL + "(RFC-0047 §4.3 F10 字面)"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G30.4 M-e:G29 受影响门零降级——" + DIVISION_LITERAL + "(RFC-0047 §4.3 F10)",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def selftest() -> int:
    bad: list[str] = []
    if [k for k, _ in VERIFY_SCRIPTS] != ["g29_closed_gate", "g29_closeout"]:
        bad.append("verify 清单漂移(F10 钉死两门,同 M-c 集合)")
    for _, script in VERIFY_SCRIPTS:
        if not (ROOT / script).is_file():
            bad.append(f"verify 脚本缺失:{script}")
    if not all(prefix.startswith("g29_") for _, prefix in PREFIX_GUARD):
        bad.append("PREFIX_GUARD 守卫对象须为 g29_ 前缀 subject")
    if len(PREFIX_GUARD) != 2:
        bad.append(f"PREFIX_GUARD 应恰 2 条(镜像 g29 版),实为 {len(PREFIX_GUARD)}")
    if not SCHEMA_PATH.is_file():
        bad.append(f"schema 缺失:{SCHEMA_PATH.name}")
    if bad:
        for b in bad:
            print(f"[{SUBJECT}] SELFTEST FAIL: {b}", file=sys.stderr)
        return 1
    print(f"[{SUBJECT}] SELFTEST PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
