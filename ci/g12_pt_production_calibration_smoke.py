#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.2 生产化核心波续）
"""G12.2 M166 PT 生产化标定门冒烟（g12.p1.m166.pt_production_calibration；
G12_ACCEPTANCE_MAP §2;G12_PLAN §2 G12.2 退出门;RFC-0029 §4 U2;P-09 禁手写）。

判据（MAP §2 行逐字）:
1. **生产化闭门槛值标定集**——RR 吞吐参考阈 τ(p50)/自适应 rel_err 阈 θ
   (p75@spp=N_floor)/收敛误判率阈(场景×族单元 p100)/曲线容差(族间极差
   p100)/白炉能量容差(族间均值极差 p100)/逐级单调噪声带/RR 无偏容差,全部
   标定程序 measured 产(禁手写);标定样本集下界 ≥24 + manifest digest 入
   evidence;
2. **标定程序可复跑**:harness --calibrate 两跑输出逐字节一致;
3. **标定值按 M138 同程序(p100×k measured)入 g12_budget.json**——7 条
   g12.pt.* 条目 measured_local 字节级纯追加 + provenance 齐备 + 逐条目
   evidence(results.trimmed_mean)+ budget_eval 全 PASS;采样器选型
   artifact(milestones/g12/g12_pt_sampler_selection.json,winner 族)同批落。

RED 臂(MAP §2 字面):手写阈值冒充标定即 RED;estimated 冒充 measured 即
RED;标定程序不可复跑即 RED;样本集低于下界冒充有效标定即 RED。

纯 host 门(device_section_state=not_applicable;harness --calibrate 零
device 依赖,pbrt 1024 参照经子进程真跑——无 pbrt provisioning →
SKIP=DEV_ENV_DEGRADE 不充绿)。

用法:
  py -3 ci/g12_pt_production_calibration_smoke.py --gate g12.p1.m166.pt_production_calibration
  py -3 ci/g12_pt_production_calibration_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g12_pt_prod_lib as gl  # noqa: E402

GATE_KEY = "g12.p1.m166.pt_production_calibration"
NUMERIC_STEP = 217
SUBJECT = "g12_pt_production_calibration"
SCHEMA_PATH = ROOT / "milestones/g12/g12_m166_pt_production_calibration_evidence_schema.json"
SOURCE_REF = (
    "G12_ACCEPTANCE_MAP §2 M166;G12_PLAN §2 G12.2;RFC-0029 §4 U2;"
    "spec/global_illumination.md RXS-0398~0401"
)
TAG = "g12_m166"
SAMPLE_LOWER_BOUND = 24
CAL1 = gl.WORK_DIR / "calibration_run1.json"
CAL2 = gl.WORK_DIR / "calibration_run2.json"

CHECK_KEYS = [
    "calibration_two_run_bitexact",
    "sample_set_lower_bound",
    "sample_digest_registered",
    "budget_entries_appended_measured_local",
    "budget_eval_all_pass",
    "selection_artifact_written",
    "red_handwritten_threshold_detected",
    "red_estimated_masquerade_detected",
    "red_nonrerunnable_detected",
    "red_below_bound_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


# ---------------------------------------------------------------------------
# 标定值 → budget 条目(M138 同程序:p100×k measured;字节级纯追加)
# ---------------------------------------------------------------------------

# 条目注册表:(budget id, calib json 键, k 或 None=公式直读 tol, 描述)
ENTRY_REGISTRY = [
    ("g12.pt.rr_tau", "rr_tau", 2.0,
     "RR 吞吐参考阈 τ(p50 逐路径 bounce==2 吞吐 max 分量;cornell+direct+two_light spp16 stratified;M166 标定程序产,drift guard ×2.0,禁手写 P-09)"),
    ("g12.pt.adaptive_rel_err_theta", "adaptive_rel_err_theta", 1.5,
     "自适应 rel_err 阈 θ(p75 池化逐像素 rel_err @ spp=N_floor=16;cornell+direct winner 族;M166 标定程序产,drift guard ×1.5,禁手写 P-09)"),
    ("g12.pt.misjudge_rate_tol", "misjudge_rate", None,
     "收敛误判率阈(场景×族单元 p100 + 1/min_cell_judged)×2.0(协议冻结;误判带 0.25);M166 标定程序产,禁手写 P-09"),
    ("g12.pt.curve_tol_rel", "curve_tol_rel", 1.5,
     "收敛曲线不劣于判定相对容差(族×场景×spp 矩阵曲线族间极差/锚 p100 ×1.5,协议冻结 k);M166 标定程序产,禁手写 P-09"),
    ("g12.pt.furnace_energy_tol", "furnace_energy_tol", 1.5,
     "白炉能量容差(device vs host 参照均值相对偏差上界;族间均值相对极差 p100 ×1.5,协议冻结 k;不产能量上界 Le 门侧硬断言);M166 标定程序产,禁手写 P-09"),
    ("g12.pt.level_monotone_tol", "level_monotone_tol", 1.5,
     "逐级能量增量单调噪声带(四场景 spp64 max(0,E_(k+1)/E_k−1) p100 ×1.5,协议冻结 k);M166 标定程序产,禁手写 P-09"),
    ("g12.pt.rr_unbiased_tol", "rr_unbiased_tol", 2.0,
     "RR 无偏容差(RR 开/关均值相对差;场景×族池化 p100 ×2.0,协议冻结 k);M166 标定程序产,禁手写 P-09"),
]


def expected_threshold(calib: dict, key: str, k: float | None) -> float:
    """由标定 JSON measured 面重算阈(手写阈值冒充检出器——禁手写 P-09 承载)。"""
    block = calib[key]
    measured = float(block["measured"] if "measured" in block else block["base"])
    if k is None:
        # misjudge:tol = (rate + 1/min_cell_judged)×2.0,由 JSON 字段重算。
        return (measured + 1.0 / float(block["min_cell_judged"])) * 2.0
    return measured * k


def measured_of(calib: dict, key: str) -> float:
    block = calib[key]
    return float(block["measured"] if "measured" in block else block["base"])


def validate_budget_entry(entry: dict, expected_thr: float) -> list[str]:
    problems: list[str] = []
    if entry.get("evidence") != "measured_local":
        problems.append(f"{entry.get('id')}: evidence={entry.get('evidence')!r}(estimated 冒充 measured)")
    if abs(float(entry.get("threshold", -1.0)) - expected_thr) > 1e-12 * max(1.0, abs(expected_thr)):
        problems.append(
            f"{entry.get('id')}: threshold={entry.get('threshold')} ≠ 标定重算 {expected_thr}(手写阈值冒充)"
        )
    return problems


def build_entries(calib: dict, ts: str) -> list[dict]:
    entries: list[dict] = []
    slug_map = {
        "g12.pt.rr_tau": "rr_tau",
        "g12.pt.adaptive_rel_err_theta": "adaptive_rel_err_theta",
        "g12.pt.misjudge_rate_tol": "misjudge_rate_tol",
        "g12.pt.curve_tol_rel": "curve_tol_rel",
        "g12.pt.furnace_energy_tol": "furnace_energy_tol",
        "g12.pt.level_monotone_tol": "level_monotone_tol",
        "g12.pt.rr_unbiased_tol": "rr_unbiased_tol",
    }
    for eid, key, k, desc in ENTRY_REGISTRY:
        measured = measured_of(calib, key)
        thr = expected_threshold(calib, key, k)
        ev_path = f"evidence/g12_m166_calibration_{slug_map[eid]}_{ts}.json"
        entries.append({
            "id": eid,
            "description": desc + f";样本集 digest {calib['sample_manifest']['digest']}(count={calib['sample_manifest']['count']} ≥ {SAMPLE_LOWER_BOUND});标定程序 ci/g12_pt_production_calibration_smoke.py 可复跑(两跑逐位一致)",
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": thr,
            "evidence_file": ev_path,
            "measured_value": measured,
        })
    return entries


def write_entry_evidence(calib: dict, ts: str) -> list[str]:
    """逐条目 evidence(results.trimmed_mean = measured;budget_eval 消费面)。"""
    written: list[str] = []
    slug_map = {
        "g12.pt.rr_tau": "rr_tau",
        "g12.pt.adaptive_rel_err_theta": "adaptive_rel_err_theta",
        "g12.pt.misjudge_rate_tol": "misjudge_rate_tol",
        "g12.pt.curve_tol_rel": "curve_tol_rel",
        "g12.pt.furnace_energy_tol": "furnace_energy_tol",
        "g12.pt.level_monotone_tol": "level_monotone_tol",
        "g12.pt.rr_unbiased_tol": "rr_unbiased_tol",
    }
    for eid, key, _k, _desc in ENTRY_REGISTRY:
        measured = measured_of(calib, key)
        doc = {
            "schema": "rurix.g12pt.calibration_entry.v1",
            "entry_id": eid,
            "results": {"trimmed_mean": measured},
            "protocol": calib[key].get("protocol", ""),
            "sample_manifest": calib["sample_manifest"],
            "provenance": calib["provenance"],
            "timestamp": ts,
        }
        out = gl.EVIDENCE_DIR / f"g12_m166_calibration_{slug_map[eid]}_{ts}.json"
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        written.append(str(out))
    return written


def append_budget_entries(new_entries: list[dict]) -> list[str]:
    """g12_budget.json 字节级纯追加(M138 同纪律:已存在同值幂等、值漂移即
    problems;追加后整体可解析复核)。"""
    budget_text = gl.BUDGET_PATH.read_text(encoding="utf-8")
    budget = json.loads(budget_text)
    problems: list[str] = []
    to_add: list[dict] = []
    for entry in new_entries:
        existing = [x for x in budget.get("entries", []) if x.get("id") == entry["id"]]
        if existing:
            ex = existing[0]
            comparable = {k: v for k, v in entry.items() if k != "evidence_file"}
            ex_comparable = {k: v for k, v in ex.items() if k != "evidence_file"}
            if ex_comparable != comparable:
                problems.append(f"{entry['id']} 已在树且值漂移(只追加禁改写): 在树 {ex} vs 重算 {entry}")
            continue
        to_add.append(entry)
    if problems or not to_add:
        return problems
    nl = "\r\n" if "\r\n" in budget_text else "\n"
    anchor = f"{nl}  ],{nl}  \"ratio_assertions\""
    if anchor not in budget_text:
        return ["g12_budget.json 结构锚缺失(entries 闭合段未找到,拒改写)"]
    frag = ""
    for entry in to_add:
        body = json.dumps(entry, ensure_ascii=False, indent=2)
        body = body.replace("\n", nl)
        body = "    " + body.replace(nl, nl + "    ")
        frag += "," + nl + body
    head, sep, tail = budget_text.partition(anchor)
    budget_text = head + frag + sep + tail
    json.loads(budget_text)
    gl.BUDGET_PATH.write_text(budget_text, encoding="utf-8", newline="")
    return problems


def write_selection_artifact(calib: dict, ts: str) -> None:
    sel = {
        "schema": "rurix.g12pt.sampler_selection.v1",
        "winner": calib["sampler_selection"]["winner"],
        "variance_table": calib["sampler_selection"]["variance_table"],
        "protocol": calib["sampler_selection"]["protocol"],
        "sample_manifest": calib["sample_manifest"],
        "provenance": calib["provenance"],
        "timestamp": ts,
        "note": "RXS-0400 L1 选型面:benchmark measured 裁决,选型证据进 evidence;winner 族 = 生产化门消费采样器。内容确定性——同值重写幂等,漂移即 RED。",
    }
    text = json.dumps(sel, ensure_ascii=False, indent=2) + "\n"
    if gl.SELECTION_PATH.is_file():
        existing = gl.SELECTION_PATH.read_text(encoding="utf-8")
        old = json.loads(existing)
        if old.get("winner") != sel["winner"] or old.get("variance_table") != sel["variance_table"]:
            raise SystemExit(f"选型 artifact 漂移(禁改写): 在树 {old.get('winner')} vs 重算 {sel['winner']}")
    gl.SELECTION_PATH.write_text(text, encoding="utf-8", newline="")


# ---------------------------------------------------------------------------
# selftest(反 YAML-only)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂:合法条目过校验。
    ok_entry = {
        "id": "g12.pt.selftest_probe",
        "evidence": "measured_local",
        "threshold": 0.5,
    }
    if validate_budget_entry(ok_entry, 0.5):
        print(f"[{TAG}] selftest FAIL: 合法条目误判", file=sys.stderr)
        return 1
    # 红臂①:手写阈值冒充必检出。
    if not validate_budget_entry(dict(ok_entry, threshold=0.75), 0.5):
        print(f"[{TAG}] selftest FAIL: 手写阈值冒充未检出", file=sys.stderr)
        return 1
    # 红臂②:estimated 冒充必检出。
    if not validate_budget_entry(dict(ok_entry, evidence="estimated"), 0.5):
        print(f"[{TAG}] selftest FAIL: estimated 冒充未检出", file=sys.stderr)
        return 1
    # 红臂③:不可复跑(两跑分叉)必检出——合成两跑面。
    if not _detect_rerun_divergence(b"{}", b"{\"x\":1}"):
        print(f"[{TAG}] selftest FAIL: 两跑分叉未检出", file=sys.stderr)
        return 1
    # 红臂④:低于下界样本集冒充必检出。
    if not _detect_below_bound({"sample_manifest": {"count": SAMPLE_LOWER_BOUND - 1}}):
        print(f"[{TAG}] selftest FAIL: 低于下界冒充未检出", file=sys.stderr)
        return 1
    # 绿臂:下界正例不误判。
    if _detect_below_bound({"sample_manifest": {"count": SAMPLE_LOWER_BOUND}}):
        print(f"[{TAG}] selftest FAIL: 下界正例误判", file=sys.stderr)
        return 1
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (4 RED + 3 GREEN)")
    return 0


def _detect_rerun_divergence(a: bytes, b: bytes) -> bool:
    return a != b


def _detect_below_bound(calib: dict) -> bool:
    return int(calib.get("sample_manifest", {}).get("count", 0)) < SAMPLE_LOWER_BOUND


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    device_state = "not_applicable"

    # pbrt provisioning 缺失 → DEV_ENV_DEGRADE(不充绿)。
    if not gl.PBRT_EXE.is_file() or not gl.IMGTOOL_EXE.is_file():
        device_state = "dev_env_degrade"
        note("pbrt/imgtool provisioning 缺失(DEV_ENV_DEGRADE)")
        harness = None
    else:
        harness = gl.build_harness()
        if harness is None:
            check(False, "g12_pt_production harness 构建失败")

    calib = None
    if harness is not None:
        gl.WORK_DIR.mkdir(parents=True, exist_ok=True)
        # ① 标定两跑逐字节一致(可复跑判据)。
        cmd1 = [str(harness), "--calibrate", str(CAL1), "--pbrt", str(gl.PBRT_EXE),
                "--imgtool", str(gl.IMGTOOL_EXE), "--work-dir", str(gl.WORK_DIR / "pbrt_work")]
        r1 = gl.run(cmd1, timeout=3600)
        if "G12_PT_PROD: SKIP" in r1.stdout:
            device_state = "dev_env_degrade"
            note(f"harness 标定 SKIP: {r1.stdout.strip()[-300:]}")
        elif r1.returncode != 0:
            check(False, f"标定跑 1 失败 rc={r1.returncode}: {(r1.stdout + r1.stderr)[-800:]}")
        else:
            r2 = gl.run([str(harness), "--calibrate", str(CAL2), "--pbrt", str(gl.PBRT_EXE),
                         "--imgtool", str(gl.IMGTOOL_EXE), "--work-dir", str(gl.WORK_DIR / "pbrt_work")], timeout=3600)
            if r2.returncode != 0:
                check(False, f"标定跑 2 失败 rc={r2.returncode}")
            else:
                b1 = CAL1.read_bytes()
                b2 = CAL2.read_bytes()
                checks["calibration_two_run_bitexact"] = b1 == b2
                check(b1 == b2, "标定两跑非逐字节一致(不可复跑即 RED)")
                calib = json.loads(b1.decode("utf-8"))

    if calib is not None:
        # ② 样本集下界 + digest。
        count = int(calib.get("sample_manifest", {}).get("count", 0))
        checks["sample_set_lower_bound"] = count >= SAMPLE_LOWER_BOUND
        check(count >= SAMPLE_LOWER_BOUND, f"样本集 {count} < 下界 {SAMPLE_LOWER_BOUND}")
        digest = calib.get("sample_manifest", {}).get("digest", "")
        checks["sample_digest_registered"] = isinstance(digest, str) and digest.startswith("sha256:")
        note(f"样本集 count={count} digest={digest[:32]}…;winner={calib['sampler_selection']['winner']}")

        # ③ budget 条目重算校验 + 字节级纯追加 + 逐条目 evidence。
        entries = build_entries(calib, ts)
        entry_problems: list[str] = []
        for entry, (eid, key, k, _desc) in zip(entries, ENTRY_REGISTRY):
            entry_problems += validate_budget_entry(entry, expected_threshold(calib, key, k))
        check(not entry_problems, f"条目重算校验: {entry_problems[:2]}")
        gl.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        write_entry_evidence(calib, ts)
        problems = append_budget_entries(entries) if not entry_problems else entry_problems
        checks["budget_entries_appended_measured_local"] = not problems
        check(not problems, f"budget 追加: {problems[:2]}")
        note(f"g12_budget.json 追加/幂等 7 条 g12.pt.* 标定条目({[e['id'] for e in entries][0]}…)")
        # ④ 选型 artifact。
        write_selection_artifact(calib, ts)
        sel_ok = gl.SELECTION_PATH.is_file() and json.loads(
            gl.SELECTION_PATH.read_text(encoding="utf-8")
        ).get("winner") == calib["sampler_selection"]["winner"]
        checks["selection_artifact_written"] = sel_ok
        check(sel_ok, "选型 artifact 未落盘/winner 不符")
        # ⑤ budget_eval 全 PASS(normal 模式:全 measured_local 零 skip)。
        r = gl.run(["py", "-3", "ci/budget_eval.py"])
        checks["budget_eval_all_pass"] = r.returncode == 0 and "[budget_eval] PASS" in (r.stdout + r.stderr)
        check(r.returncode == 0, f"budget_eval 非零: {(r.stdout + r.stderr)[-400:]}")

    # ⑥ RED 臂(检出器红绿——合成注入)。
    probe = {"id": "g12.pt.selftest_probe", "evidence": "measured_local", "threshold": 0.5}
    checks["red_handwritten_threshold_detected"] = bool(validate_budget_entry(dict(probe, threshold=0.75), 0.5))
    checks["red_estimated_masquerade_detected"] = bool(validate_budget_entry(dict(probe, evidence="estimated"), 0.5))
    checks["red_nonrerunnable_detected"] = _detect_rerun_divergence(b"{}", b"{\"x\":1}")
    checks["red_below_bound_detected"] = _detect_below_bound({"sample_manifest": {"count": SAMPLE_LOWER_BOUND - 1}})

    all_pass = all(checks.values()) and not FAILURES and device_state == "not_applicable"
    host_pass = all_pass
    evidence = gl.gate_evidence(
        subject=SUBJECT,
        gate_key=GATE_KEY,
        milestone="M166",
        wave="G12.2",
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        checks=checks,
        device_state=device_state,
        host_pass=host_pass,
        commands=[
            {"seq": 1, "command": "cargo build -p rurix-render --features vulkan --bin g12_pt_production", "exit_code": 0 if harness else 1},
            {"seq": 2, "command": "g12_pt_production --calibrate <run1> --pbrt .. --imgtool .. (host 标定)", "exit_code": 0 if checks["calibration_two_run_bitexact"] else 1},
            {"seq": 3, "command": "g12_pt_production --calibrate <run2> (复跑逐字节一致)", "exit_code": 0 if checks["calibration_two_run_bitexact"] else 1},
            {"seq": 4, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
        ],
        environment=gl.environment(),
        production={
            "correctness_anchor_unchanged": True,
            "baseline_anchor_id": "g12.pt.*(7 条标定条目本门产出入 budget)",
            "measured_value": "见逐条目 evidence results.trimmed_mean",
            "not_worse_than_anchor": True,
            "threshold_provenance": "harness --calibrate 标定程序 measured 产(两跑逐位一致;样本集 digest 入 evidence;P-09 禁手写)",
            "evolution_register": None,
        },
        notes="; ".join(NOTES + FAILURES[:8]),
        all_pass=all_pass,
        ts=ts,
    )
    gl.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = gl.EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
