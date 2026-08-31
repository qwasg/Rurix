#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G38 T2:FIF×动态每槽 AS 副本组内存预算标定(TODO #90 / RFC-0030 §4.3 L2a)。

读 g31_fif_dyn_probe v2 evidence(rurix.g31.fif_dyn_probe.v2)→ 回填
milestones/g31/g31_budget.json 条目 `g31.fif_dyn.slot_as_group_mem_bytes`:
evidence estimated→measured_local、threshold = measured × 1.5(程序产禁手写,
方向 max)、evidence_file / measured_value 登记。ci/budget_eval.py 通用路
(results.trimmed_mean)随后判读,零新 evaluator、零新数字步骤号。

- 标定前 fail-closed 核验:schema v2 + verdict PASS + gates 七门全 true +
  镜像槽复算互核(trimmed_mean == max 组 group_total == Σ per_slot_bytes,
  标定不信任单值镜像)+ 值 >0;
- 回填 = **外科手术式字节级改写**:只动条目对象内 evidence/skip_reason/
  threshold/evidence_file/measured_value 五字段,条目外全文件字节 0 改动
  (前后缀逐字节核验,防重排噪声污染并行窗);
- 幂等:值已一致 → 零写盘退 0,可任意复跑;
- --check:只读核验条目与 evidence 互核(回填后验证 / CI 侧手动复核);
- --selftest:临时目录伪 evidence 双向(绿臂回填+幂等+check;红臂 RED 拒标 /
  镜像槽不一致拒 / 篡改 threshold 后 check 红),零真实文件触碰。

用法:
  py -3 ci/calibrate_fif_budget.py [--evidence <path>]   # 标定回填(缺省取
                                     evidence/g31_fif_dyn_probe_*.json 最新件)
  py -3 ci/calibrate_fif_budget.py --check               # 只读核验
  py -3 ci/calibrate_fif_budget.py --selftest            # 纯 host 自检
"""
from __future__ import annotations

import io
import json
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TAG = "[calibrate_fif_budget]"
ENTRY_ID = "g31.fif_dyn.slot_as_group_mem_bytes"
BUDGET_REL = "milestones/g31/g31_budget.json"
EVIDENCE_GLOB = "g31_fif_dyn_probe_*.json"
K = 1.5  # threshold = measured × 1.5(协议 k,程序产禁手写)

GATE_KEYS = (
    "b_eq_a_bytewise",
    "c_eq_a_bytewise",
    "double_run_bitlevel",
    "validation_zero",
    "dynamic_witness",
    "red_arms_rejected",
    "slot_as_mem_registered",
)


def validate_evidence(doc: dict) -> tuple[int, list[str]]:
    """v2 evidence 标定资格核验(纯函数;selftest 双向承载)。

    返回 (trimmed_mean, errors);errors 非空即不可标定。
    """
    errs: list[str] = []
    if doc.get("schema") != "rurix.g31.fif_dyn_probe.v2":
        errs.append(f"schema 非 v2: {doc.get('schema')!r}")
    if doc.get("verdict") != "PASS":
        errs.append(f"verdict 非 PASS: {doc.get('verdict')!r}(RED 件不得标定预算)")
    gates = doc.get("gates") or {}
    bad_gates = [k for k in GATE_KEYS if gates.get(k) is not True]
    if bad_gates:
        errs.append(f"gates 非全 true: {bad_gates}")
    results = doc.get("results") or {}
    tm = results.get("trimmed_mean")
    if not isinstance(tm, int) or isinstance(tm, bool) or tm <= 0:
        errs.append(f"results.trimmed_mean 非正整数: {tm!r}")
        tm = 0
    if results.get("unit") != "bytes":
        errs.append(f"results.unit 非 bytes: {results.get('unit')!r}")
    # 镜像槽复算互核:trimmed_mean == max 组 group_total == Σ per_slot_bytes。
    mem = doc.get("slot_as_mem") or {}
    totals: list[int] = []
    for arm, want_len in (("a_seq", 1), ("b_fif2", 2), ("c_fif3", 3)):
        blk = mem.get(arm) or {}
        per = blk.get("per_slot_bytes")
        total = blk.get("group_total_bytes")
        if not isinstance(per, list) or len(per) != want_len or not all(
            isinstance(b, int) and not isinstance(b, bool) and b > 0 for b in per
        ):
            errs.append(f"slot_as_mem.{arm}.per_slot_bytes 形状/正数破缺: {per!r}")
            continue
        if total != sum(per):
            errs.append(f"slot_as_mem.{arm} group_total {total!r} ≠ Σ per_slot {sum(per)}")
            continue
        totals.append(total)
    if totals and tm != max(totals):
        errs.append(f"镜像槽 trimmed_mean {tm} ≠ 最大组 group_total {max(totals)}(复算互核破)")
    return tm, errs


def find_latest_evidence() -> Path | None:
    cands = sorted(
        (ROOT / "evidence").glob(EVIDENCE_GLOB),
        key=lambda p: (p.stat().st_mtime, p.name),
    )
    return cands[-1] if cands else None


def entry_span(text: str) -> tuple[int, int]:
    """条目对象字节 span(id 锚 → 首个 '}';条目全扁平字段无嵌套对象)。"""
    anchor = f'"id": "{ENTRY_ID}"'
    start = text.index(anchor)
    end = text.index("}", start)
    return start, end


def surgical_backfill(budget_path: Path, measured: int, evidence_rel: str) -> bool:
    """字节级外科回填;返回是否发生写盘(False = 已一致幂等跳过)。"""
    with io.open(budget_path, "r", encoding="utf-8", newline="") as f:
        text = f.read()
    start, end = entry_span(text)
    seg = text[start:end]
    threshold = measured * K
    repl = [
        (r'"evidence": (?:null|"[^"]*")', '"evidence": "measured_local"'),
        (r'"skip_reason": (?:null|"(?:[^"\\]|\\.)*")', '"skip_reason": null'),
        (r'"threshold": (?:null|[0-9.eE+-]+)', f'"threshold": {json.dumps(threshold)}'),
        (r'"evidence_file": (?:null|"[^"]*")', f'"evidence_file": {json.dumps(evidence_rel)}'),
        (r'"measured_value": (?:null|[0-9.eE+-]+)', f'"measured_value": {measured}'),
    ]
    new_seg = seg
    for pat, sub in repl:
        new_seg, n = re.subn(pat, sub, new_seg, count=1)
        if n != 1:
            raise RuntimeError(f"条目字段锚缺失/不唯一: {pat!r}(拒改不猜)")
    if new_seg == seg:
        return False
    new_text = text[:start] + new_seg + text[end:]
    # fail-closed:改后必须仍是合法 JSON 且条目外前后缀逐字节不变。
    doc = json.loads(new_text)
    got = next(e for e in doc["entries"] if e["id"] == ENTRY_ID)
    assert got["evidence"] == "measured_local" and got["measured_value"] == measured
    assert got["threshold"] == threshold and got["evidence_file"] == evidence_rel
    if new_text[:start] != text[:start] or new_text[start + len(new_seg):] != text[end:]:
        raise RuntimeError("条目外字节漂移(外科纪律破)")
    with io.open(budget_path, "w", encoding="utf-8", newline="") as f:
        f.write(new_text)
    return True


def check_entry(budget_path: Path, evidence_root: Path) -> list[str]:
    """--check 只读核验:条目 measured_local 且与 evidence 位级互核。"""
    errs: list[str] = []
    doc = json.loads(budget_path.read_text(encoding="utf-8"))
    entry = next((e for e in doc.get("entries", []) if e.get("id") == ENTRY_ID), None)
    if entry is None:
        return [f"条目 {ENTRY_ID} 不存在"]
    if entry.get("evidence") == "estimated":
        return [f"条目仍为 estimated 占位(未标定;skip_reason: {entry.get('skip_reason')!r})"]
    if entry.get("evidence") != "measured_local":
        return [f"条目 evidence 非法: {entry.get('evidence')!r}"]
    ef = entry.get("evidence_file")
    if not ef or not (evidence_root / ef).is_file():
        return [f"evidence_file 缺失或不存在: {ef!r}"]
    ev = json.loads((evidence_root / ef).read_text(encoding="utf-8"))
    tm, ev_errs = validate_evidence(ev)
    errs.extend(f"evidence 件不合格: {e}" for e in ev_errs)
    if entry.get("measured_value") != tm:
        errs.append(f"measured_value {entry.get('measured_value')!r} ≠ evidence trimmed_mean {tm}(禁手写漂移)")
    want_thr = tm * K
    if entry.get("threshold") != want_thr:
        errs.append(f"threshold {entry.get('threshold')!r} ≠ measured × {K} = {want_thr}(程序产禁手写)")
    if entry.get("skip_reason") is not None:
        errs.append(f"measured_local 条目 skip_reason 应为 null: {entry.get('skip_reason')!r}")
    return errs


# ---------------------------------------------------------------------------
# selftest(伪 evidence 双向;临时目录,零真实文件触碰)
# ---------------------------------------------------------------------------

def _fake_evidence(tm: int = 300, tamper: str | None = None) -> dict:
    per = {"a_seq": [100], "b_fif2": [100, 100], "c_fif3": [100, 100, 100]}
    doc = {
        "schema": "rurix.g31.fif_dyn_probe.v2",
        "probe": "g31_fif_dyn_probe",
        "todo": 90,
        "args": {"frames": 48, "rays": "96x72", "action": "rebuild"},
        "gates": {k: True for k in GATE_KEYS},
        "verdict": "PASS",
        "failures": [],
        "measured_note": "selftest 伪件",
        "slot_as_mem": {
            "note": "selftest",
            **{
                arm: {"per_slot_bytes": p, "group_total_bytes": sum(p)}
                for arm, p in per.items()
            },
        },
        "results": {"trimmed_mean": tm, "unit": "bytes", "source": "selftest"},
        "arms": {},
    }
    if tamper == "red_verdict":
        doc["verdict"] = "RED"
        doc["gates"]["b_eq_a_bytewise"] = False
    elif tamper == "mirror_drift":
        doc["results"]["trimmed_mean"] = tm + 1
    return doc


_FAKE_BUDGET = """{
  "schema_version": 1,
  "namespace": "g31",
  "entries": [
    {
      "id": "g31.fif_dyn.slot_as_group_mem_bytes",
      "description": "selftest 占位",
      "direction": "max",
      "evidence": "estimated",
      "skip_reason": "待回填",
      "unit": "bytes",
      "threshold": null,
      "evidence_file": null,
      "measured_value": null
    }
  ]
}
"""


def run_selftest() -> int:
    ok = True

    def case(name: str, fn) -> None:
        nonlocal ok
        try:
            fn()
            print(f"{TAG} selftest {name}: PASS")
        except Exception as e:  # noqa: BLE001 - selftest 报告面
            print(f"{TAG} selftest {name}: FAIL — {e}", file=sys.stderr)
            ok = False

    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        (root / "evidence").mkdir()
        budget = root / "g31_budget.json"

        def green() -> None:
            budget.write_text(_FAKE_BUDGET, encoding="utf-8", newline="")
            ev_rel = "evidence/g31_fif_dyn_probe_selftest.json"
            (root / ev_rel).write_text(
                json.dumps(_fake_evidence(), ensure_ascii=False), encoding="utf-8"
            )
            ev = json.loads((root / ev_rel).read_text(encoding="utf-8"))
            tm, errs = validate_evidence(ev)
            assert not errs and tm == 300, f"绿件核验失败: {errs}"
            wrote = surgical_backfill(budget, tm, ev_rel)
            assert wrote, "首次回填应写盘"
            # 幂等:第二跑零改动。
            assert not surgical_backfill(budget, tm, ev_rel), "幂等破(第二跑写盘)"
            # 条目外字节不变:description 字面仍在 + check 绿。
            assert '"description": "selftest 占位"' in budget.read_text(encoding="utf-8")
            errs2 = check_entry(budget, root)
            assert not errs2, f"check 应绿: {errs2}"
            got = json.loads(budget.read_text(encoding="utf-8"))["entries"][0]
            assert got["threshold"] == 450.0 and got["measured_value"] == 300

        def red_verdict() -> None:
            _, errs = validate_evidence(_fake_evidence(tamper="red_verdict"))
            assert any("RED 件不得标定" in e for e in errs), f"RED 件未拒: {errs}"

        def red_mirror() -> None:
            _, errs = validate_evidence(_fake_evidence(tamper="mirror_drift"))
            assert any("复算互核破" in e for e in errs), f"镜像漂移未拒: {errs}"

        def red_check_tamper() -> None:
            # 篡改 threshold(手写漂移)→ check 必红。
            text = budget.read_text(encoding="utf-8")
            budget.write_text(
                text.replace('"threshold": 450.0', '"threshold": 451.0'),
                encoding="utf-8",
                newline="",
            )
            errs = check_entry(budget, root)
            assert any("程序产禁手写" in e for e in errs), f"篡改 threshold 未检出: {errs}"

        case("green(回填+幂等+check)", green)
        case("red(RED verdict 拒标定)", red_verdict)
        case("red(镜像槽复算互核拒)", red_mirror)
        case("red(篡改 threshold check 红)", red_check_tamper)

    if ok:
        print(f"{TAG}: PASS selftest 4/4(伪 evidence 双向,零真实文件触碰)")
        return 0
    return 1


def main() -> int:
    argv = sys.argv[1:]
    if "--selftest" in argv:
        return run_selftest()
    evidence_arg: str | None = None
    check_only = "--check" in argv
    if "--evidence" in argv:
        i = argv.index("--evidence")
        if i + 1 >= len(argv):
            print(f"{TAG}: FAIL --evidence 缺参数值", file=sys.stderr)
            return 2
        evidence_arg = argv[i + 1]
    known = {"--check", "--selftest", "--evidence"}
    extra = [a for a in argv if a not in known and (evidence_arg is None or a != evidence_arg)]
    if extra:
        print(f"{TAG}: FAIL 未知参数 {extra}(--evidence/--check/--selftest)", file=sys.stderr)
        return 2

    budget_path = ROOT / BUDGET_REL
    if check_only:
        errs = check_entry(budget_path, ROOT)
        if errs:
            for e in errs:
                print(f"{TAG}: CHECK FAIL — {e}", file=sys.stderr)
            return 1
        print(f"{TAG}: CHECK PASS — {ENTRY_ID} 与 evidence 位级互核绿")
        return 0

    if evidence_arg:
        ev_path = (ROOT / evidence_arg) if not Path(evidence_arg).is_absolute() else Path(evidence_arg)
    else:
        found = find_latest_evidence()
        if found is None:
            print(
                f"{TAG}: FAIL evidence/{EVIDENCE_GLOB} 无在档件——先跑 probe GPU 收割"
                "(cargo run -p rurix-render --features vulkan --bin g31_fif_dyn_probe -- "
                "--frames 48 --rays 96x72 --out evidence/g31_fif_dyn_probe_rebuild_<ts>.json)",
                file=sys.stderr,
            )
            return 1
        ev_path = found
    if not ev_path.is_file():
        print(f"{TAG}: FAIL evidence 件不存在: {ev_path}", file=sys.stderr)
        return 1
    doc = json.loads(ev_path.read_text(encoding="utf-8"))
    tm, errs = validate_evidence(doc)
    if errs:
        for e in errs:
            print(f"{TAG}: FAIL evidence 不合格 — {e}", file=sys.stderr)
        return 1
    ev_rel = ev_path.resolve().relative_to(ROOT).as_posix()
    wrote = surgical_backfill(budget_path, tm, ev_rel)
    verb = "回填完成" if wrote else "已标定一致(幂等零写盘)"
    print(
        f"{TAG}: PASS {verb} — {ENTRY_ID}: measured={tm} bytes,"
        f" threshold={tm * K}(× {K} 程序产),evidence_file={ev_rel}"
    )
    errs = check_entry(budget_path, ROOT)
    if errs:
        for e in errs:
            print(f"{TAG}: FAIL 回填后自核 — {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
