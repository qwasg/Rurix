#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.2 修订与测量波）
"""G14.2 P0 硬门 M-a：M-c/M-d 门登记表 UE 方差带结构化对账修订
（g14.p0.m_a.registry_variance_band_reconciliation；G14_CONTRACT §4.2 M-a/G-G14-4；
G14_ACCEPTANCE_MAP §1 M-a 行；G13_CONTRACT §8.7 结构性修复承接锚兑现面）。

判据（契约 §4.2 M-a 逐字）：
1. **结构化对账修订在树**：gaplib 正典单源加性面
   `reconcile_registry_structured`（身份面逐字节 + Rurix 侧/结构常量位级 +
   UE 侧程序产方差带）在树且 selftest 全绿；M-c/M-d 双门脚本接线面在树
   （旧「在树非逐字节相等」字节冻结面移除字面机核）。
2. **修订后 M-c/M-d 全门复跑双绿**：子进程真跑 --gate（各自产新鲜 PASS
   evidence，stamp ≥ 本门起点），登记表在树态复跑不再误报厂商随机方差；
   双臂真跑面（UE MRQ + Rurix device）归双门本体判据。
3. **G13 锁定双登记表 8+2 行终态 0-byte 不回写**：复跑前后逐字节一致
   （内容 digest 比对）+ gap_id 闭集 == G13.5b 终审锁定清单逐字对账。
4. **UE 侧方差带程序产入 budget**：双门复跑 evidence notes 面解析
   band_rel（门内 UE 探针格三样本 max 两两相对差 ×2.0），入 g14_budget
   两条目 measured_local（阈 = measured ×2.0 守护带，禁手写 P-09）。
5. **RED 双臂门内真跑**：以在树真实登记表为底——UE 侧大方差注入（×1.5）
   必检出 / UE 侧带内小方差（×(1+band×0.1)）必吸收 / Rurix 侧 1e-12 级
   漂移必检出 / 身份面（title）篡改必检出——四臂独立有效。
6. **UE 确定性控制面调研结论登记**（cvar/收敛面压缩方差底调研面，notes
   入档；2026-08-19 调研报告主会话留痕转引）。

RED 字面：方差带手写冒充程序产即 RED；身份面漂移静默即 RED；修订后
M-c/M-d 复跑仍误报即 RED（门体 = 复跑面 exit/status/新鲜度三机核）。

pr-smoke 默认 --verify-latest（秒级核最新 full-run evidence）；
本地/workflow_dispatch 用 --gate 产 full-run。

用法：
  py -3 ci/g14_registry_variance_band_reconciliation_smoke.py --gate g14.p0.m_a.registry_variance_band_reconciliation
  py -3 ci/g14_registry_variance_band_reconciliation_smoke.py --verify-latest
  py -3 ci/g14_registry_variance_band_reconciliation_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import copy
import datetime as _dt
import json
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g10_wave_exit_lib as wel  # noqa: E402
import g10_gap_registry_lib as gaplib  # noqa: E402

GATE_KEY = "g14.p0.m_a.registry_variance_band_reconciliation"
NUMERIC_STEP = 250  # 落盘前实测 registry/number_ledger.json CI_step.next_free=250 顺位领取
SUBJECT = "g14_m_a_registry_variance_band_reconciliation"
WAVE = "G14.2"
TAG = "g14_m_a"
MATRIX_ROW = "M172"
SOURCE_REF = (
    "G14_CONTRACT §4.2 M-a/G-G14-4;G14_ACCEPTANCE_MAP §1;G13_CONTRACT §8.7 承接锚;"
    "RXS-0391 IR2（gaplib 正典单源）/P-09（程序产阈禁手写）"
)
SCHEMA_PATH = ROOT / "milestones" / "g14" / "g14_m_a_registry_variance_band_reconciliation_evidence_schema.json"
BUDGET_PATH = ROOT / "milestones" / "g14" / "g14_budget.json"

MC_SCRIPT = ROOT / "ci" / "g13_ue_upscale_parity_smoke.py"
MD_SCRIPT = ROOT / "ci" / "g13_ue_lumen_gi_parity_smoke.py"
UPSCALE_REGISTRY = ROOT / "milestones" / "g13" / "g13_ue_upscale_gap_registry.json"
LUMEN_REGISTRY = ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json"

MC_GATE = "g13.p0.m_c.ue_upscale_parity"
MD_GATE = "g13.p0.m_d.ue_lumen_gi_parity"
MC_PREFIX = "g13_m_c_ue_upscale_parity"
MD_PREFIX = "g13_m_d_ue_lumen_gi_parity"

# G13.5b 终审锁定清单（gap_id 身份五节派生与测量值解耦——G14 只消费不回写）。
FROZEN_UPSCALE_IDS = frozenset({
    "fda2892b148edc2f", "20f125548f145335", "58fd4c7e2ef98efe", "2631811751d63e0a",
    "d36e8cb107d579d9", "5b65327b903ac6bc", "bdf94acf4691fd74", "20e8950296211aae",
})
FROZEN_LUMEN_IDS = frozenset({"2f6331a41404dfcd", "b7527c980cdd1d46"})

BAND_NOTE_RE = re.compile(r"band_rel=([0-9]+\.[0-9]+)")

CHECK_KEYS = [
    "gaplib_structured_reconcile_present",
    "gate_amendments_present",
    "g13_registries_frozen_ids_locked",
    "m_c_rerun_pass",
    "m_d_rerun_pass",
    "registries_0byte_post_rerun",
    "ue_variance_bands_measured_into_budget",
    "red_arms_effective",
    "ue_determinism_research_registered",
    "budget_eval_all_pass",
]

NOTES: list[str] = []
FAILURES: list[str] = []


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout)


def _sha256(path: Path) -> str:
    import hashlib
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def _latest_stamp(prefix: str) -> str | None:
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return None
    m = re.search(r"_(\d{8}T\d{6}Z)\.json$", path.name)
    return m.group(1) if m else None


def _band_from_notes(prefix: str) -> float | None:
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return None
    doc = wel.load_json(path)
    m = BAND_NOTE_RE.search(str(doc.get("notes") or ""))
    return float(m.group(1)) if m else None


def _classify_by_a_ue(_metric: str, field: str, _value: float) -> str:
    """RED 臂用端侧归属（与 M-c 构造面同形：a=UE 侧 / b=Rurix 侧）。"""
    return gaplib.PROVENANCE_UE if field == "a_value" else gaplib.PROVENANCE_RURIX


# ---------------------------------------------------------------- RED 臂（门内真跑）
def red_arms(registry_path: Path, band_rel: float) -> dict[str, bool]:
    """以在树真实登记表为底的四臂（大方差检出/带内吸收/位级漂移检出/身份面检出）。"""
    base = wel.load_json(registry_path)
    results: dict[str, bool] = {}

    def _mut_first_delta(fn):
        doc = copy.deepcopy(base)
        fn(doc["items"][0]["measured_delta"][0])
        # delta 构造不变式修补（变异面只留目标轴）
        d = doc["items"][0]["measured_delta"][0]
        d["delta"] = float(d["b_value"]) - float(d["a_value"])
        return doc

    # 臂① UE 侧大方差注入（×1.5）→ 必检出
    big = _mut_first_delta(lambda d: d.update(a_value=float(d["a_value"]) * 1.5))
    results["red_ue_large_variance_detected"] = bool(
        gaplib.reconcile_registry_structured(base, big, band_rel, _classify_by_a_ue))
    # 臂② UE 侧带内小方差（×(1+band×0.1)）→ 必吸收
    small = _mut_first_delta(lambda d: d.update(a_value=float(d["a_value"]) * (1.0 + band_rel * 0.1)))
    results["green_ue_in_band_absorbed"] = not gaplib.reconcile_registry_structured(
        base, small, band_rel, _classify_by_a_ue)
    # 臂③ Rurix 侧 1e-12 级漂移 → 必检出（位级硬门）
    rurix = _mut_first_delta(lambda d: d.update(b_value=float(d["b_value"]) * (1.0 + 1e-12)))
    results["red_rurix_bit_drift_detected"] = bool(
        gaplib.reconcile_registry_structured(base, rurix, band_rel, _classify_by_a_ue))
    # 臂④ 身份面篡改（title 换字）→ 必检出
    ident = copy.deepcopy(base)
    ident["items"][0]["title"] = ident["items"][0]["title"] + "_tampered"
    results["red_identity_tamper_detected"] = bool(
        gaplib.reconcile_registry_structured(base, ident, band_rel, _classify_by_a_ue))
    return results


def _write_measured_entry(slug: str, entry_id: str, band: float, samples: list[float],
                          protocol: str, ts: str) -> str:
    """measured-entry evidence 落盘（results.run_variance_band_rel 供 budget_eval
    g14.ue_variance_band. 前缀分派判读）；返回仓库相对路径。"""
    import hashlib
    manifest_digest = "sha256:" + hashlib.sha256(
        json.dumps(samples, sort_keys=False).encode("utf-8")).hexdigest()
    doc = {
        "schema": "rurix.g14uevariance.measured_entry.v1",
        "entry_id": entry_id,
        "results": {"run_variance_band_rel": band},
        "protocol": protocol,
        "sample_manifest": {"count": len(samples), "digest": manifest_digest},
        "provenance": {
            "gpu": "device",
            "backend": "ue5.8.1-mrq-arm",
            "base_commit": (run(["git", "rev-parse", "HEAD"]).stdout or "").strip(),
        },
        "timestamp": ts,
    }
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"g14_m_a_band_{slug}_{ts}.json"
    out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return f"evidence/g14_m_a_band_{slug}_{ts}.json"


def _samples_from_notes(prefix: str) -> list[float]:
    path = wel.load_latest_evidence(prefix)
    if path is None:
        return []
    doc = wel.load_json(path)
    m = re.search(r"samples=([0-9./]+)", str(doc.get("notes") or ""))
    if not m:
        return []
    return [float(x) for x in m.group(1).split("/") if x]


def _write_budget(band_upscale: float, band_lumen: float, mc_ev: str, md_ev: str) -> None:
    """g14_budget 首建/幂等回写两条目（measured_local；阈 = measured ×2.0 守护带）。"""
    entries = [
        {
            "id": "g14.ue_variance_band.upscale_probe_rel",
            "description": (
                "M-c 门 UE 探针格（bistro-interior/tier67 末帧 HF share）运行间方差带"
                "（门内三样本 max 两两相对差 ×2.0 程序产，禁手写 P-09；G13 §8.7 承接锚"
                "兑现面——厂商随机方差吸收带，真实内容变更 ≫带 检出面维持）；本条目 = "
                "带的回归守护阈（阈 = 带实测 ×2.0），标定程序 "
                "ci/g13_ue_upscale_parity_smoke.py 探针格段可复跑"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": band_upscale * 2.0,
            "evidence_file": mc_ev,
            "measured_value": band_upscale,
        },
        {
            "id": "g14.ue_variance_band.lumen_probe_rel",
            "description": (
                "M-d 门 UE 探针格（bistro-interior/lumen-on 末帧平均亮度）运行间方差带"
                "（同口径 ×2.0 程序产）；回归守护阈 = 带实测 ×2.0；标定程序 "
                "ci/g13_ue_lumen_gi_parity_smoke.py 探针格段可复跑"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": band_lumen * 2.0,
            "evidence_file": md_ev,
            "measured_value": band_lumen,
        },
    ]
    if BUDGET_PATH.is_file():
        doc = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
        keep = [e for e in (doc.get("entries") or [])
                if e.get("id") not in {entries[0]["id"], entries[1]["id"]}]
        doc["entries"] = keep + entries
    else:
        doc = {
            "schema_version": 1,
            "namespace": "g14",
            "_meta": {
                "provenance": "Assisted-by: Kimi-K3（G14.2 修订与测量波）",
                "created_utc": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                "base_commit": (run(["git", "rev-parse", "HEAD"]).stdout or "").strip(),
            },
            "description": (
                "G14 帧率对标与管线性能期预算。G14.2 M-a 首批两条目 = UE 运行间方差带"
                "（M-c/M-d 门探针格标定程序产）回归守护面。本预算只证明测量已建立与守护带"
                "已登记，不断言任何帧率对标达标——M-d 通过线 = G-G14-6 契约面。前瞻预算项"
                "（M-b UE benchmark 臂帧时 / M-c 生产管线帧时等）一律等后续实现波标定回填"
                "——无实测证据的阈值不写入（零 estimated 硬约束）；counter_assertions 留空。"
            ),
            "source_docs": [
                "milestones/g14/G14_CONTRACT.md",
                "milestones/g14/G14_ACCEPTANCE_MAP.md",
            ],
            "entries": [],
        }
        doc["entries"] = entries
    BUDGET_PATH.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    started = _dt.datetime.now(_dt.timezone.utc)
    ts = started.strftime("%Y%m%dT%H%M%SZ")
    start_stamp = ts
    red_results: dict[str, bool] = {}
    band_upscale = None
    band_lumen = None

    # ── ① gaplib 正典面 + selftest ──
    lib_text = (ROOT / "ci" / "g10_gap_registry_lib.py").read_text(encoding="utf-8")
    present = "def reconcile_registry_structured(" in lib_text
    st = run([sys.executable, str(ROOT / "ci" / "g10_gap_registry_lib.py"), "--selftest"], timeout=300)
    checks["gaplib_structured_reconcile_present"] = present and st.returncode == 0
    check(checks["gaplib_structured_reconcile_present"],
          f"gaplib 结构化对账面/selftest 异常: present={present} selftest_exit={st.returncode}")
    note(f"gaplib 结构化对账面在树={present}，selftest exit={st.returncode}")

    # ── ② 双门脚本接线面在树（旧字节冻结面移除字面机核） ──
    mc_text = MC_SCRIPT.read_text(encoding="utf-8")
    md_text = MD_SCRIPT.read_text(encoding="utf-8")
    wired = (
        "reconcile_registry_structured(" in mc_text
        and "reconcile_registry_structured(" in md_text
        and "在树非逐字节相等" not in mc_text
        and "在树非逐字节相等" not in md_text
        and "UE 探针格" in mc_text
        and "UE 探针格" in md_text
    )
    checks["gate_amendments_present"] = wired
    check(wired, "M-c/M-d 门脚本结构化对账接线面缺失（或旧字节冻结面残留）")

    # ── ③ G13 锁定双登记表 gap_id 闭集对账 + 复跑前 digest 锚 ──
    pre_digests = {}
    ids_ok = True
    for path, frozen in ((UPSCALE_REGISTRY, FROZEN_UPSCALE_IDS), (LUMEN_REGISTRY, FROZEN_LUMEN_IDS)):
        if not path.is_file():
            ids_ok = False
            check(False, f"{path.name} 缺失")
            continue
        doc = wel.load_json(path)
        ids = {it.get("gap_id") for it in (doc.get("items") or [])}
        if ids != frozen:
            ids_ok = False
            check(False, f"{path.name} gap_id 闭集离 G13.5b 锁定清单: {sorted(ids ^ frozen)[:3]}")
        verrs = gaplib.validate_registry(doc, scene_set=list(doc.get("scene_set") or []),
                                         registry_name=doc.get("registry"))
        if verrs:
            ids_ok = False
            check(False, f"{path.name} schema 校验: {verrs[:2]}")
        pre_digests[path.name] = _sha256(path)
    checks["g13_registries_frozen_ids_locked"] = ids_ok

    # ── ④ 修订后 M-c/M-d 全门复跑双绿（子进程真跑） ──
    for key, prefix, script in ((MC_GATE, MC_PREFIX, MC_SCRIPT), (MD_GATE, MD_PREFIX, MD_SCRIPT)):
        note(f"复跑 {key}（子进程真跑，UE MRQ + Rurix device 双臂）…")
        r = run([sys.executable, str(script), "--gate", key], timeout=14400)
        ok = r.returncode == 0
        fresh = False
        top_pass = False
        ev_path = wel.load_latest_evidence(prefix)
        if ev_path is not None:
            doc = wel.load_json(ev_path)
            top_pass = doc.get("status") == "pass"
            stamp = _latest_stamp(prefix)
            fresh = stamp is not None and stamp >= start_stamp
        gate_ok = ok and top_pass and fresh
        checks[f"{'m_c' if prefix == MC_PREFIX else 'm_d'}_rerun_pass"] = gate_ok
        check(gate_ok, f"{key} 复跑面异常: exit={r.returncode} top={top_pass} fresh={fresh}")
        note(f"{key} 复跑 exit={r.returncode} status_pass={top_pass} fresh={fresh}")

    # ── ⑤ 复跑后登记表 0-byte（逐字节一致机核） ──
    post_ok = True
    for path in (UPSCALE_REGISTRY, LUMEN_REGISTRY):
        if not path.is_file() or _sha256(path) != pre_digests.get(path.name):
            post_ok = False
            check(False, f"{path.name} 复跑后非 0-byte（内容 digest 离复跑前锚）")
    checks["registries_0byte_post_rerun"] = post_ok
    note(f"双登记表复跑前后逐字节一致 = {post_ok}")

    # ── ⑥ UE 方差带程序产入 budget ──
    band_upscale = _band_from_notes(MC_PREFIX)
    band_lumen = _band_from_notes(MD_PREFIX)
    bands_ok = (
        band_upscale is not None and band_lumen is not None
        and band_upscale >= 0.0 and band_lumen >= 0.0
    )
    if bands_ok:
        samples_up = _samples_from_notes(MC_PREFIX)
        samples_lu = _samples_from_notes(MD_PREFIX)
        mc_ev = _write_measured_entry(
            "upscale", "g14.ue_variance_band.upscale_probe_rel", band_upscale, samples_up,
            "M-c 门内 UE 探针格（bistro-interior/tier67 末帧 HF share）三样本运行间方差底 "
            "max 两两相对差 ×2.0 程序产（禁手写 P-09；G13 §8.7 承接锚兑现面）",
            ts)
        md_ev = _write_measured_entry(
            "lumen", "g14.ue_variance_band.lumen_probe_rel", band_lumen, samples_lu,
            "M-d 门内 UE 探针格（bistro-interior/lumen-on 末帧平均亮度）三样本运行间方差底 "
            "max 两两相对差 ×2.0 程序产（同口径）",
            ts)
        _write_budget(band_upscale, band_lumen, mc_ev, md_ev)
        # 幂等复核：回读后条目在且 measured == 解析值
        doc = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
        got = {e["id"]: e for e in doc.get("entries") or []}
        bands_ok = (
            got.get("g14.ue_variance_band.upscale_probe_rel", {}).get("measured_value") == band_upscale
            and got.get("g14.ue_variance_band.lumen_probe_rel", {}).get("measured_value") == band_lumen
            and got.get("g14.ue_variance_band.upscale_probe_rel", {}).get("threshold") == band_upscale * 2.0
        )
    checks["ue_variance_bands_measured_into_budget"] = bool(bands_ok)
    check(not bands_ok is False, f"方差带入 budget 异常: upscale={band_upscale} lumen={band_lumen}")
    note(f"UE 方差带程序产：upscale band_rel={band_upscale} lumen band_rel={band_lumen}（入 g14_budget 两条目）")

    # ── ⑦ RED 臂（门内真跑，以在树真实登记表为底） ──
    if band_upscale is not None:
        red_results = red_arms(UPSCALE_REGISTRY, band_upscale)
    arms_ok = bool(red_results) and all(red_results.values())
    checks["red_arms_effective"] = arms_ok
    check(not arms_ok is False, f"RED 臂面: {red_results}")
    for k, v in red_results.items():
        note(f"RED 臂 {k}: {'有效' if v else '失效'}")

    # ── ⑧ UE 确定性控制面调研结论登记 ──
    research_note = (
        "UE 确定性控制面调研结论（2026-08-19 主会话留痕转引）：UE 5.8.1 deferred+Lumen+DLSS "
        "MRQ 臂 32 帧静态收敛序列末帧残余随机噪声（Lumen 随机采样面）为跨进程运行间方差根因"
        "（G13 §8.7 四跑取证 0.854056/0.851862/0.854550/0.852789 ≈0.32%，Rurix 侧位级一致）；"
        "cvar/收敛面（r.RandomSeed 固定、MRQ warm-up 帧、收敛帧数加严）可压缩方差底但不能消除"
        "至位级——厂商侧不承诺跨进程位级确定性（DLSS 内部历史/调度面）；故结构化对账（方差带"
        "吸收 + 身份面/位级面硬门）为稳健处置面，n≥3 样本 max 两两相对差 ×2.0 程序产带"
        "（P-09 禁手写）+ G15 画面终审期按只追加程序评估更大样本标定面"
    )
    checks["ue_determinism_research_registered"] = bool(research_note)
    note(f"调研结论登记：{research_note[:80]}…")

    # ── ⑨ budget_eval 全 PASS ──
    bud = run([sys.executable, str(ROOT / "ci" / "budget_eval.py")], timeout=600)
    checks["budget_eval_all_pass"] = bud.returncode == 0 and "[budget_eval] PASS" in (bud.stdout or "")
    check(checks["budget_eval_all_pass"], "budget_eval 非全 PASS")

    all_pass = all(checks.values()) and not FAILURES
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "status": "pass" if all_pass else "fail",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": (run(["git", "rev-parse", "HEAD"]).stdout or "").strip(),
        "host_section_pass": all_pass,
        "device_section_state": "executed" if (
            checks["m_c_rerun_pass"] and checks["m_d_rerun_pass"]
        ) else "fail",
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": "py -3 ci/g10_gap_registry_lib.py --selftest（gaplib 正典面）",
             "exit_code": 0 if checks["gaplib_structured_reconcile_present"] else 1},
            {"seq": 2, "command": f"py -3 ci/g13_ue_upscale_parity_smoke.py --gate {MC_GATE}（修订后复跑）",
             "exit_code": 0 if checks["m_c_rerun_pass"] else 1},
            {"seq": 3, "command": f"py -3 ci/g13_ue_lumen_gi_parity_smoke.py --gate {MD_GATE}（修订后复跑）",
             "exit_code": 0 if checks["m_d_rerun_pass"] else 1},
            {"seq": 4, "command": "RED 臂 ×4（ue-large-variance/ue-in-band/rurix-bit-drift/identity-tamper）",
             "exit_code": 0 if checks["red_arms_effective"] else 1},
            {"seq": 5, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "production": {
            "correctness_anchor_unchanged": checks["g13_registries_frozen_ids_locked"] and checks["registries_0byte_post_rerun"],
            "baseline_anchor_id": "g14.ue_variance_band.{upscale,lumen}_probe_rel（本门标定产出入 g14_budget）",
            "measured_value": f"upscale band_rel={band_upscale} lumen band_rel={band_lumen}",
            "not_worse_than_anchor": bool(red_results) and all(red_results.values()),
            "threshold_provenance": "门内 UE 探针格三样本 max 两两相对差 ×2.0 程序产（P-09 禁手写）；budget 守护阈 = 带实测 ×2.0",
            "evolution_register": (
                "G13 §8.7 承接锚兑现面：M-c/M-d 门「在树逐字节相等」字节冻结面 → gaplib 结构化对账"
                "（身份面逐字节 + Rurix 侧/结构常量位级 + UE 侧程序产方差带）；G13 锁定双登记表 "
                "8+2 行终态 0-byte（复跑前后逐字节一致机核在案）；真实内容变更检出面维持（RED 臂①④）"
            ),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]) + "；" + research_note,
    }
    errs = wel.validate_schema(evidence, SCHEMA_PATH) if SCHEMA_PATH.is_file() else []
    if errs:
        print(f"[{TAG}] schema errors: {errs}", file=sys.stderr)
        all_pass = False
        evidence["status"] = "fail"
        evidence["host_section_pass"] = False
    wel.EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = wel.EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if all_pass else 'FAIL'} checks={sum(1 for v in checks.values() if v)}/{len(checks)}")
    return 0 if all_pass else 1


def verify_latest() -> int:
    path = wel.load_latest_evidence(SUBJECT)
    if path is None:
        print(f"[{TAG}] FAIL: 缺最新 evidence（{SUBJECT}_*.json）", file=sys.stderr)
        return 1
    doc = wel.load_json(path)
    checks = doc.get("checks") or {}
    need = set(CHECK_KEYS)
    bad = [k for k in need if checks.get(k) is not True]
    if bad or doc.get("status") != "pass":
        print(f"[{TAG}] FAIL checks={bad} status={doc.get('status')!r}", file=sys.stderr)
        return 1
    print(f"[{TAG}] verify-latest PASS（{path.name}，checks {len(need)} 键全绿）")
    return 0


def selftest() -> int:
    """schema 闭集对账 + 结构化对账函数面 RED/GREEN（不依赖 device）。"""
    failures = 0
    schema = wel.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        failures += 1
    # 函数面 RED/GREEN（合成登记表）
    good_delta = {"metric": "m", "a_value": 1.0, "b_value": 2.5,
                  "delta": 1.5, "evidence_digest": "sha256:" + "0" * 64}
    good_item = {
        "gap_id": gaplib.derive_gap_id("s", "c", gaplib.MODULE_PREFIX + "Lumen", "quality_gap", "t"),
        "scene_id": "s", "camera_id": "c", "domain": "scene-linear-hdr", "kind": "quality_gap",
        "ue5_module_primary": gaplib.MODULE_PREFIX + "Lumen", "ue5_module_secondary": [],
        "measured_delta": [good_delta], "suggested_priority": "P2",
        "g11_anchor": "G15 承接锚", "title": "t", "description": "d", "attachments": [],
    }
    base = {"schema_version": 1, "registry": "selftest", "generated_by": "selftest",
            "scene_set": ["s"], "items": [good_item],
            "scene_summary": [{"scene_id": "s", "gap_count": 1, "no_gap_explicit": False}],
            "not_ready_scenes": []}
    mut = copy.deepcopy(base)
    mut["items"][0]["measured_delta"][0].update(a_value=1.5, delta=1.0)
    if not gaplib.reconcile_registry_structured(base, mut, 0.01, _classify_by_a_ue):
        print(f"[{TAG}] selftest FAIL: 大方差注入未检出", file=sys.stderr)
        failures += 1
    mut2 = copy.deepcopy(base)
    mut2["items"][0]["measured_delta"][0].update(a_value=1.001, delta=2.5 - 1.001)
    if gaplib.reconcile_registry_structured(base, mut2, 0.01, _classify_by_a_ue):
        print(f"[{TAG}] selftest FAIL: 带内小方差未吸收", file=sys.stderr)
        failures += 1
    if failures:
        return 1
    print(f"[{TAG}] selftest PASS（schema 闭集 + 2 函数面臂）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.verify_latest:
        return verify_latest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
