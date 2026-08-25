#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent(G30.3/G30.4 P0 smoke)
"""G30.3 P0 M-c:战役全链零降级(gate g30.p0.m_c.campaign_full_chain_no_regression,步骤 516)。

判据法定来源:G30_CONTRACT §4.2 M-c 行 + RFC-0047(v0.2,Agent Approved)§3。

- verify 清单钉死(F10)= g29.p0.m_e.closed_gate_no_regression + g29.wave.6b.closeout 两门
  (G29 M-e 先例同构,争议时只追加扩表)。
- verify-latest 语义(F4 如实化):静态读档核验 = evidence 链完整性定盘,非现势重验;
  现势零回归由 M-b 表面 0-byte 机核 + 焦点格新鲜真跑 + G30.5 soak 承载。
- g29_closeout_check.py 实测无 --require-ready 参数(argparse 仅 --gate/--verify-latest/
  --selftest,--require-ready 会 argparse error rc=2),且其 --verify-latest 无独立读档分支
  (fallthrough 至 run_gate 只读汇总重评 + 落盘 g29_ 前缀新档);判定镜像 G29 M-e 对
  g28_closeout_check 的字面 = `--verify-latest` rc==0(closed 态输出 VERDICT=READY——
  G29 M-e 真跑同款机制在案,g29_ 前缀 subject 不构成 g30_ 抢占)。
- 预算面(F18):budget_eval --strict 全量零 skip 零 estimated,禁 --allow-pending
  (命令行字面不含该 flag;strict 下 estimated 违例必 rc≠0,skip 只可能来自 --allow-pending)。
- 禁 --gate 旧脚本重跑。
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g30.p0.m_c.campaign_full_chain_no_regression"
NUMERIC_STEP = 516
SUBJECT = "g30_m_c_campaign_full_chain_no_regression"
WAVE = "G30.3"
SCHEMA_PATH = ROOT / "milestones/g30/g30_m_c_campaign_full_chain_no_regression_evidence_schema.json"
SOURCE_REF = "G30_CONTRACT §4.2 M-c;RFC-0047 §3"

# 上游十一期收口 tag 闭集(RFC-0047 §3 前提:11/11,G30.0 baseline 在档)
CLOSED_TAGS = (
    "g19-closed", "g20-closed", "g21-closed", "g22-closed", "g23-closed",
    "g24-closed", "g25-closed", "g26-closed", "g27-closed", "g28-closed",
    "g29-closed",
)
# verify 清单钉死两门(F10)
VERIFY_SCRIPTS = [
    ("g29_closed_gate", "ci/g29_closed_gate_no_regression_smoke.py"),
    ("g29_closeout", "ci/g29_closeout_check.py"),
]
# 命令行字面禁含 --allow-pending(F18)
BUDGET_CMD = ("ci/budget_eval.py", "--strict")
CHAIN_SEMANTICS_LITERAL = (
    "verify-latest = 静态读档核验(evidence 链完整性),非现势重验;"
    "现势零回归由 M-b 表面 0-byte + 焦点格真跑 + G30.5 soak 承载"
)


def _tag_closure() -> tuple[bool, str]:
    r = subprocess.run(["git", "tag", "--list"], cwd=ROOT, capture_output=True, text=True)
    tags = set((r.stdout or "").split())
    present = [t for t in CLOSED_TAGS if t in tags]
    missing = [t for t in CLOSED_TAGS if t not in tags]
    ok = r.returncode == 0 and not missing
    detail = f"{len(present)}/{len(CLOSED_TAGS)} 在树:{','.join(present)}"
    if missing:
        detail += f";缺:{','.join(missing)}"
    return ok, detail


def _verify(script: str) -> tuple[bool, str]:
    r = subprocess.run([sys.executable, str(ROOT / script), "--verify-latest"],
                       cwd=ROOT, capture_output=True, text=True)
    tail = ((r.stdout or "") + (r.stderr or ""))[-160:].replace("\n", " ")
    return r.returncode == 0, f"rc={r.returncode} {tail}"


def _budget_strict() -> tuple[bool, str]:
    r = subprocess.run([sys.executable, *BUDGET_CMD], cwd=ROOT, capture_output=True, text=True)
    lines = [ln for ln in ((r.stdout or "") + (r.stderr or "")).splitlines() if ln.strip()]
    tail = lines[-1] if lines else "(无输出)"
    m = re.search(r"\[budget_eval\] PASS \((\d+) pass, (\d+) skip, strict mode\)", tail)
    skip_rows = [ln for ln in lines if ln.lstrip().startswith("SKIP")]
    ok = r.returncode == 0 and m is not None and int(m.group(2)) == 0 and not skip_rows
    detail = (f"cmd='py -3 {' '.join(BUDGET_CMD)}'(无 --allow-pending,F18) rc={r.returncode} "
              f"tail={tail!r} SKIP 行={len(skip_rows)};strict 下 estimated 违例必 rc≠0(rc=0 即零 estimated)")
    return ok, detail


def evaluate() -> list[dict]:
    facts: list[dict] = []
    ok, detail = _tag_closure()
    facts.append({"id": "tag_count_11", "status": "PASS" if ok else "FAIL", "detail": detail})
    for name, script in VERIFY_SCRIPTS:
        ok, detail = _verify(script)
        facts.append({"id": f"verify_{name}", "status": "PASS" if ok else "FAIL", "detail": detail})
    ok, detail = _budget_strict()
    facts.append({"id": "budget_strict_zero_skip_zero_estimated",
                  "status": "PASS" if ok else "FAIL", "detail": detail})
    facts.append({"id": "verify_chain_semantics_honest", "status": "PASS",
                  "detail": CHAIN_SEMANTICS_LITERAL + "(RFC-0047 §3 F4 如实化字面)"})
    facts.append({"id": "no_gate_on_old_scripts", "status": "PASS",
                  "detail": "只发 --verify-latest,禁 --gate 旧脚本重跑(RFC-0047 §3)"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G30.3 M-c:战役全链零降级——" + CHAIN_SEMANTICS_LITERAL + "(RFC-0047 §3 F4)",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def selftest() -> int:
    bad: list[str] = []
    if len(CLOSED_TAGS) != 11 or len(set(CLOSED_TAGS)) != 11:
        bad.append(f"CLOSED_TAGS 应恰 11 个,实为 {len(CLOSED_TAGS)}")
    if CLOSED_TAGS[0] != "g19-closed" or CLOSED_TAGS[-1] != "g29-closed":
        bad.append("CLOSED_TAGS 端点非 g19-closed~g29-closed")
    if [k for k, _ in VERIFY_SCRIPTS] != ["g29_closed_gate", "g29_closeout"]:
        bad.append("verify 清单漂移(F10 钉死两门)")
    for _, script in VERIFY_SCRIPTS:
        if not (ROOT / script).is_file():
            bad.append(f"verify 脚本缺失:{script}")
    if "--allow-pending" in BUDGET_CMD:
        bad.append("BUDGET_CMD 含 --allow-pending(F18 违例)")
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
