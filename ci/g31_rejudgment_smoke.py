#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C16 重判窗批量执行）
"""G31+ 波 C Task C16:重判窗批量执行门冒烟(g31.waveC.rejudgment;
G31_PLUS_COMMERCIAL_RENDERER_TODO §3 #24/#26/#27/#28/#29 + §4 #43 六窗;
判档登记表 = milestones/g31/g31_rejudgment_windows.json 唯一消费面)。

判据闭集(milestones/g31/g31_rejudgment_windows_evidence_schema.json 描述段):
1. registry_structure:六窗 id 闭集 + 逐窗九字段(anchor_literal/anchor_source/
   method/items/verdict/verdict_detail/evidence/followup)非空。
2. verdict_enum:三值闭集 + summary.window_verdicts 与逐窗 verdict 一致。
3. evidence_pointers_exist:逐窗 evidence 全指针在盘实文件。
4. mesh_bench_evidence_fresh:M61 ③ measured 门件在盘 + bench schema 校验 +
   digest_all_equal/double_run 位级 + 逐臂 median 有限正数 + parity 差登记。
5. deferred_history_appended:RD-039/RD-040/RD-026 各 2026-08-26 Task C16 行
   + status 维持 open + backfill_condition 锚句抽查 0-byte。
6. rfc0034_row_appended:G31+ C16 行在 + G18.5/G20.3/G27.2 三行原行在。
7. anchor_files_0byte:g20/g21/g27/g28 四件 tracked 锚 git porcelain 干净 +
   TODO 在飞未跟踪件在树登记。
8. window_items_consistency:partial 窗 items 半命中半未命中结构 + M61
   verdict_detail 含 maintain-no-go 字面。

三态:本门 = host 纯文件机核面,无 GPU 腿——登记表/bench 门件/锚文件缺失
即 FAIL(非 SKIP);DEV_ENV_DEGRADE 不适用(无 device 依赖面)如实登记。

evidence 纪律:PASS-only schema 面——PASS 才落
evidence/g31_rejudgment_windows_<ts>.json(check_schemas 前缀路由
g31_rejudgment_windows_);FAIL 诊断件落 .tmp/g31_gates/rejudgment/ 不污染
evidence/ 路由面。

用法:
  py -3 ci/g31_rejudgment_smoke.py --selftest
  py -3 ci/g31_rejudgment_smoke.py --gate g31.waveC.rejudgment
"""
from __future__ import annotations

import argparse
import datetime as _dt
import glob
import io
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

GATE_KEY = "g31.waveC.rejudgment"
SUBJECT = "g31_rejudgment_windows"
TAG = "g31_rejudgment_smoke"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_rejudgment_windows_evidence_schema.json"
SCHEMA_ID = "rurix.g31.rejudgment_windows_gate_evidence.v1"
REGISTRY_PATH = ROOT / "milestones" / "g31" / "g31_rejudgment_windows.json"
DEFERRED_PATH = ROOT / "registry" / "deferred.json"
RFC0034_PATH = ROOT / "rfcs" / "0034-virtualized-geometry-p3-mesh-shader.md"
BENCH_SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_mesh_vs_raster_bench_evidence_schema.json"
TODO_PATH = ROOT / "G31_PLUS_COMMERCIAL_RENDERER_TODO.md"
WORK = ROOT / ".tmp" / "g31_gates" / "rejudgment"

WINDOW_IDS = [
    "M61-mesh-shader",
    "RD-039-backfill-subitems",
    "SMRT",
    "WORLD-RC-evolution",
    "NRD-vendor-denoise",
    "RD-026-stdgpu",
]
VERDICT_ENUM = {"triggered", "not-triggered", "partial"}
WINDOW_REQUIRED_FIELDS = [
    "anchor_literal", "anchor_source", "method", "items",
    "verdict", "verdict_detail", "evidence", "followup",
]
TRACKED_ANCHORS = [
    "milestones/g20/g20_cluster_streaming_p4_gap.json",
    "milestones/g21/g21_rd040_subitem_registry.json",
    "milestones/g27/g27_cluster_p4_rejudgment.json",
    "milestones/g28/g28_m52_rd040_workload_rejudgment.json",
]
BACKFILL_ANCHOR_SENTENCES = {
    "RD-039": ["逐项独立判档", "Mega Geometry 在 RT 与虚拟几何合流需求出现时"],
    "RD-040": ["SMRT 在 VSM device 化", "NRD/vendor 降噪经 UpscaleBackend 同构输入契约接入"],
    "RD-026": ["AsyncBuffer", "硬需求时"],
}

FAILURES: list[str] = []
FACT_IDS = [
    "registry_structure",
    "verdict_enum",
    "evidence_pointers_exist",
    "mesh_bench_evidence_fresh",
    "deferred_history_appended",
    "rfc0034_row_appended",
    "anchor_files_0byte",
    "window_items_consistency",
]


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 600) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout)


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面)
# ---------------------------------------------------------------------------


def registry_structure_judge(doc: dict) -> list[str]:
    """① 结构判:六窗 id 闭集 + 逐窗九字段非空。"""
    fails: list[str] = []
    if not isinstance(doc, dict):
        return ["登记表非 object"]
    if doc.get("schema") != "rurix.g31.rejudgment_windows.v1":
        fails.append(f"schema 漂移: {doc.get('schema')!r}")
    windows = doc.get("windows")
    if not isinstance(windows, list) or [w.get("id") for w in windows if isinstance(w, dict)] != WINDOW_IDS:
        fails.append(f"windows id 闭集破: {[w.get('id') for w in windows] if isinstance(windows, list) else windows!r}"[:200])
        return fails
    for w in windows:
        wid = w.get("id")
        for f_ in WINDOW_REQUIRED_FIELDS:
            v = w.get(f_)
            if v is None or (isinstance(v, str) and not v.strip()) or (isinstance(v, list) and not v):
                fails.append(f"{wid}.{f_} 缺失/空")
        if not isinstance(w.get("items"), list) or not w.get("items"):
            fails.append(f"{wid}.items 非空数组破")
        if not isinstance(w.get("evidence"), list) or not w.get("evidence"):
            fails.append(f"{wid}.evidence 非空数组破")
    return fails


def verdict_enum_judge(doc: dict) -> list[str]:
    """② verdict 三值闭集 + summary 一致性判。"""
    fails: list[str] = []
    if not isinstance(doc, dict):
        return ["登记表非 object"]
    windows = doc.get("windows") or []
    per_window = {}
    for w in windows:
        v = w.get("verdict")
        if v not in VERDICT_ENUM:
            fails.append(f"{w.get('id')}.verdict 越三值闭集: {v!r}")
        per_window[w.get("id")] = v
    summary = (doc.get("summary") or {}).get("window_verdicts") or {}
    for wid in WINDOW_IDS:
        if summary.get(wid) != per_window.get(wid):
            fails.append(f"summary.window_verdicts[{wid}]={summary.get(wid)!r} ≠ windows[{wid}]={per_window.get(wid)!r}")
    return fails


def evidence_pointers_judge(doc: dict, root: Path) -> list[str]:
    """③ 证据指针完整性判(全指针在盘实文件;glob 星号展开须 ≥1 命中)。"""
    fails: list[str] = []
    if not isinstance(doc, dict):
        return ["登记表非 object"]
    for w in doc.get("windows") or []:
        for p in w.get("evidence") or []:
            if not isinstance(p, str) or not p.strip():
                fails.append(f"{w.get('id')} evidence 空指针")
                continue
            if "*" in p:
                hits = glob.glob(str(root / p))
                if not hits:
                    fails.append(f"{w.get('id')} glob 指针零命中: {p}")
            elif not (root / p).is_file():
                fails.append(f"{w.get('id')} 指针不在盘: {p}")
    return fails


def items_consistency_judge(doc: dict) -> list[str]:
    """⑧ 窗内一致性判(partial 窗半命中半未命中;M61 verdict_detail 字面)。"""
    fails: list[str] = []
    if not isinstance(doc, dict):
        return ["登记表非 object"]
    hit_states = {"hit", "triggered", "hit-measured-parity", "green-on-record"}
    miss_states = {"miss", "not-triggered"}
    for w in doc.get("windows") or []:
        states = [i.get("state") for i in w.get("items") or []]
        if any(s not in hit_states | miss_states for s in states):
            fails.append(f"{w.get('id')} items.state 越闭集: {states!r}")
            continue
        if w.get("verdict") == "partial":
            has_hit = any(s in hit_states and s != "green-on-record" for s in states)
            has_miss = any(s in miss_states for s in states)
            if not (has_hit and has_miss):
                fails.append(f"{w.get('id')} partial 窗 items 无半命中半未命中结构: {states!r}")
    m61 = next((w for w in (doc.get("windows") or []) if w.get("id") == "M61-mesh-shader"), {})
    if m61.get("verdict") == "not-triggered" and "maintain-no-go" not in (m61.get("verdict_detail") or ""):
        fails.append("M61 not-triggered 窗 verdict_detail 缺 maintain-no-go 字面")
    return fails


def deferred_judge(deferred: dict) -> list[str]:
    """⑤ deferred.json 只追加判(三行 history + status/backfill 0-byte 抽查)。"""
    fails: list[str] = []
    if not isinstance(deferred, dict):
        return ["deferred.json 非 object"]
    entries = {e.get("id"): e for e in deferred.get("entries", [])}
    for rid in ("RD-039", "RD-040", "RD-026"):
        e = entries.get(rid)
        if not e:
            fails.append(f"{rid} 条目缺失")
            continue
        if e.get("status") != "open":
            fails.append(f"{rid}.status ≠ open(0-byte 破): {e.get('status')!r}")
        rows = [h for h in e.get("history", []) if h.get("date") == "2026-08-26" and "Task C16" in h.get("event", "")]
        if not rows:
            fails.append(f"{rid} history 缺 2026-08-26 Task C16 行")
        bf = e.get("backfill_condition", "")
        for sent in BACKFILL_ANCHOR_SENTENCES[rid]:
            if sent not in bf:
                fails.append(f"{rid}.backfill_condition 锚句 0-byte 破(缺「{sent}」)")
    return fails


def rfc0034_judge(text: str) -> list[str]:
    """⑥ RFC-0034 重判记录只追加判(C16 行在 + 三行原行在)。"""
    fails: list[str] = []
    if not isinstance(text, str) or not text.strip():
        return ["RFC-0034 文本空"]
    if "G31+ C16 重判" not in text:
        fails.append("RFC-0034 缺 G31+ C16 行")
    for marker in ("G18.5 M-g 终态 = no-go", "G20.3 M-c 重判", "G27.2 M-b 重判"):
        if marker not in text:
            fails.append(f"RFC-0034 原行字面破(缺「{marker}」)")
    return fails


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def run_gate() -> int:
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not SCHEMA_PATH.is_file():
        fail(f"门 schema 缺失: {SCHEMA_PATH}")
        return 1
    if not REGISTRY_PATH.is_file():
        fail(f"判档登记表缺失: {REGISTRY_PATH}")
        return 1
    WORK.mkdir(parents=True, exist_ok=True)

    doc = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))

    # ①
    f1 = registry_structure_judge(doc)
    set_fact("registry_structure", not f1, f"六窗 id 闭集 + 逐窗九字段非空" + ("" if not f1 else f";红 {f1[:3]}"))
    # ②
    f2 = verdict_enum_judge(doc)
    set_fact("verdict_enum", not f2, "verdict 三值闭集 + summary 一致" + ("" if not f2 else f";红 {f2[:3]}"))
    # ③
    f3 = evidence_pointers_judge(doc, ROOT)
    set_fact("evidence_pointers_exist", not f3, "逐窗 evidence 全指针在盘" + ("" if not f3 else f";红 {f3[:3]}"))

    # ④ M61 ③ measured 新鲜面(bench 门件 schema 校验 + 数字健全 + parity 登记)
    bench_doc = None
    bench_ev_rel = ""
    bench_fails: list[str] = []
    m61 = next((w for w in doc.get("windows", []) if w.get("id") == "M61-mesh-shader"), {})
    bench_ptrs = [p for p in m61.get("evidence", []) if "g31_mesh_vs_raster_bench_" in p]
    if not bench_ptrs:
        bench_fails.append("M61 窗 evidence 缺 g31_mesh_vs_raster_bench 指针")
    else:
        bench_path = ROOT / bench_ptrs[0]
        if not bench_path.is_file():
            bench_fails.append(f"bench 门件不在盘: {bench_ptrs[0]}")
        else:
            bench_ev_rel = bench_ptrs[0].replace("\\", "/")
            bench_doc = json.loads(bench_path.read_text(encoding="utf-8"))
            import jsonschema
            berrs = list(jsonschema.Draft7Validator(
                json.loads(BENCH_SCHEMA_PATH.read_text(encoding="utf-8"))
            ).iter_errors(bench_doc))
            if berrs:
                bench_fails.append(f"bench 门件 schema 校验红: {['/'.join(str(p) for p in e.path) + ': ' + e.message for e in berrs[:2]]}")
            if bench_doc.get("digest_all_equal") is not True:
                bench_fails.append("bench digest_all_equal ≠ true")
            if bench_doc.get("double_run_digest_bitexact") is not True:
                bench_fails.append("bench double_run_digest_bitexact ≠ true")
    vs_med = mesh_med = float("nan")
    parity_pct = float("nan")
    if bench_doc and not bench_fails:
        arms = bench_doc.get("arms") or {}
        vs_med = float((arms.get("vs_fetch") or {}).get("gpu_ms_median", "nan"))
        mesh_med = float((arms.get("mesh_procedural") or {}).get("gpu_ms_median", "nan"))
        if not (vs_med == vs_med and vs_med > 0 and mesh_med == mesh_med and mesh_med > 0):
            bench_fails.append(f"逐臂 median 非有限正数: vs={vs_med} mesh={mesh_med}")
        else:
            parity_pct = (vs_med - mesh_med) / vs_med * 100.0
    set_fact(
        "mesh_bench_evidence_fresh",
        not bench_fails,
        f"M61 ③ measured 门件 {Path(bench_ev_rel).name if bench_ev_rel else '缺'}:"
        f"vs_fetch={vs_med:.4f}ms mesh={mesh_med:.4f}ms parity={parity_pct:+.3f}%"
        + ("" if not bench_fails else f";红 {bench_fails[:2]}"),
    )

    # ⑤
    deferred = json.loads(DEFERRED_PATH.read_text(encoding="utf-8"))
    f5 = deferred_judge(deferred)
    set_fact("deferred_history_appended", not f5, "RD-039/RD-040/RD-026 history + 0-byte 抽查" + ("" if not f5 else f";红 {f5[:3]}"))

    # ⑥
    rfc_text = RFC0034_PATH.read_text(encoding="utf-8")
    f6 = rfc0034_judge(rfc_text)
    set_fact("rfc0034_row_appended", not f6, "G31+ C16 行 + 三行原行在" + ("" if not f6 else f";红 {f6[:3]}"))

    # ⑦ 锚文件 0-byte(tracked 四件 git porcelain;TODO 在飞未跟踪件在树登记)
    u = run(["git", "status", "--porcelain", "--", *TRACKED_ANCHORS])
    tracked_clean = not u.stdout.strip()
    todo_ok = TODO_PATH.is_file()
    set_fact(
        "anchor_files_0byte",
        tracked_clean and todo_ok,
        f"tracked 锚四件 git 干净={tracked_clean};TODO 在飞未跟踪件在树={todo_ok}(本任务零触碰)"
        + ("" if tracked_clean else f";脏 {u.stdout.strip()[:120]}"),
    )

    # ⑧
    f8 = items_consistency_judge(doc)
    set_fact("window_items_consistency", not f8, "partial 窗结构 + M61 字面" + ("" if not f8 else f";红 {f8[:3]}"))

    # ── 门裁决 + evidence(PASS-only 面)──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f_["status"] == "PASS" for f_ in fact_rows) and not FAILURES
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    env_info = {
        "gpu": "host 纯文件机核面(无 GPU 腿;measured 数字转引 bench 门件 RTX 4070 Ti measured_local)",
        "os": "windows" if sys.platform == "win32" else sys.platform,
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": "G31+.C",
        "windows_checked": WINDOW_IDS,
        "window_verdicts": {w["id"]: w.get("verdict") for w in doc.get("windows", [])},
        "mesh_bench_measured": {
            "evidence": bench_ev_rel or "evidence/g31_mesh_vs_raster_bench_00000000T000000Z.json",
            "schema_valid": not bench_fails,
            "digest_all_equal": (bench_doc or {}).get("digest_all_equal") is True,
            "double_run_digest_bitexact": (bench_doc or {}).get("double_run_digest_bitexact") is True,
            "vs_fetch_median_ms": vs_med if vs_med == vs_med else -1.0,
            "mesh_procedural_median_ms": mesh_med if mesh_med == mesh_med else -1.0,
            "parity_delta_pct": parity_pct if parity_pct == parity_pct else -999.0,
        },
        "deferred_history": {
            "RD-039": not any("RD-039" in x for x in f5),
            "RD-040": not any("RD-040" in x for x in f5),
            "RD-026": not any("RD-026" in x for x in f5),
            "status_0byte": not any("status" in x for x in f5),
            "backfill_literal_0byte": not any("backfill_condition" in x for x in f5),
        },
        "rfc0034_table": {
            "c16_row_appended": "G31+ C16 重判" in rfc_text,
            "prior_rows_intact": all(m in rfc_text for m in ("G18.5 M-g 终态 = no-go", "G20.3 M-c 重判", "G27.2 M-b 重判")),
        },
        "anchor_files_0byte": {
            "tracked_anchors_clean": tracked_clean,
            "todo_registered": todo_ok,
        },
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C16 重判窗批量执行门(六窗 verdict 与证据指针完整性机核):"
            "判档登记表 milestones/g31/g31_rejudgment_windows.json 六窗逐行——"
            f"verdicts: {'; '.join(w.get('id', '?') + '=' + str(w.get('verdict')) for w in doc.get('windows', []))};"
            f"M61 ③ measured:vs_fetch={vs_med:.4f}ms vs mesh={mesh_med:.4f}ms(parity {parity_pct:+.3f}%,如实登记)。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in fact_rows)}"
        ),
    }
    import jsonschema  # 自校验硬门(schema 漂移即 RED;PASS-only 闭集面)

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        for e in errs[:5]:
            fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
        all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_rejudgment_windows_{ts}.json"
    else:
        gate_path = WORK / f"gate_fail_{ts}.json"
    with io.open(gate_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n")
    note(f"evidence: {gate_path.relative_to(ROOT)}")
    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest(判读器红绿两臂,无外部依赖)
# ---------------------------------------------------------------------------


def _good_doc() -> dict:
    def win(wid, verdict, states):
        return {
            "id": wid, "todo_ref": "#x", "title": "t",
            "anchor_literal": "a", "anchor_source": "s", "method": "m",
            "items": [{"item": f"i{k}", "state": st, "basis": "b"} for k, st in enumerate(states)],
            "verdict": verdict, "verdict_detail": "maintain-no-go" if wid == "M61-mesh-shader" else "d",
            "evidence": ["milestones/g31/g31_rejudgment_windows.json"],
            "followup": "f",
        }
    doc = {
        "schema": "rurix.g31.rejudgment_windows.v1",
        "windows": [
            win("M61-mesh-shader", "not-triggered", ["hit", "hit", "hit-measured-parity"]),
            win("RD-039-backfill-subitems", "partial", ["triggered", "not-triggered", "miss"]),
            win("SMRT", "partial", ["hit", "miss"]),
            win("WORLD-RC-evolution", "partial", ["hit", "miss"]),
            win("NRD-vendor-denoise", "not-triggered", ["green-on-record", "miss"]),
            win("RD-026-stdgpu", "not-triggered", ["miss"]),
        ],
        "summary": {"window_verdicts": {
            "M61-mesh-shader": "not-triggered",
            "RD-039-backfill-subitems": "partial",
            "SMRT": "partial",
            "WORLD-RC-evolution": "partial",
            "NRD-vendor-denoise": "not-triggered",
            "RD-026-stdgpu": "not-triggered",
        }},
    }
    return doc


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    good = _good_doc()
    expect(registry_structure_judge(good) == [], "GREEN:结构正例")
    expect(registry_structure_judge(None) != [], "RED:非 object 必红")
    bad = json.loads(json.dumps(good))
    bad["windows"] = bad["windows"][:5]
    expect(registry_structure_judge(bad) != [], "RED:窗闭集缺件必红")
    bad = json.loads(json.dumps(good))
    bad["windows"][0]["anchor_literal"] = ""
    expect(registry_structure_judge(bad) != [], "RED:字段空必红")
    expect(verdict_enum_judge(good) == [], "GREEN:verdict 正例")
    bad = json.loads(json.dumps(good))
    bad["windows"][0]["verdict"] = "go"
    expect(verdict_enum_judge(bad) != [], "RED:verdict 越闭集必红")
    bad = json.loads(json.dumps(good))
    bad["summary"]["window_verdicts"]["SMRT"] = "not-triggered"
    expect(verdict_enum_judge(bad) != [], "RED:summary 不一致必红")
    expect(evidence_pointers_judge(good, ROOT) == [], "GREEN:指针正例(在树文件)")
    bad = json.loads(json.dumps(good))
    bad["windows"][0]["evidence"] = ["evidence/nonexistent_zzz.json"]
    expect(evidence_pointers_judge(bad, ROOT) != [], "RED:指针不在盘必红")
    expect(items_consistency_judge(good) == [], "GREEN:窗内一致正例")
    bad = json.loads(json.dumps(good))
    bad["windows"][1]["items"] = [{"item": "a", "state": "triggered", "basis": "b"}]
    expect(items_consistency_judge(bad) != [], "RED:partial 窗全命中必红")
    bad = json.loads(json.dumps(good))
    bad["windows"][1]["items"][0]["state"] = "zzz"
    expect(items_consistency_judge(bad) != [], "RED:state 越闭集必红")
    bad = json.loads(json.dumps(good))
    bad["windows"][0]["verdict_detail"] = "d"
    expect(items_consistency_judge(bad) != [], "RED:M61 缺 maintain-no-go 字面必红")
    # deferred 判读器红绿。
    good_def = {
        "entries": [
            {"id": rid, "status": "open",
             "backfill_condition": "……".join(BACKFILL_ANCHOR_SENTENCES[rid]),
             "history": [{"date": "2026-08-26", "event": "G31+ 波 C Task C16 …", "evidence": "e"}]}
            for rid in ("RD-039", "RD-040", "RD-026")
        ]
    }
    expect(deferred_judge(good_def) == [], "GREEN:deferred 正例")
    bad = json.loads(json.dumps(good_def))
    bad["entries"][0]["history"] = []
    expect(deferred_judge(bad) != [], "RED:history 缺行必红")
    bad = json.loads(json.dumps(good_def))
    bad["entries"][1]["status"] = "closed"
    expect(deferred_judge(bad) != [], "RED:status 翻动必红")
    bad = json.loads(json.dumps(good_def))
    bad["entries"][0]["backfill_condition"] = "改写"
    expect(deferred_judge(bad) != [], "RED:backfill 锚句破必红")
    # rfc0034 判读器红绿。
    good_rfc = "……G18.5 M-g 终态 = no-go……G20.3 M-c 重判……G27.2 M-b 重判……G31+ C16 重判……"
    expect(rfc0034_judge(good_rfc) == [], "GREEN:rfc0034 正例")
    expect(rfc0034_judge("……G18.5 M-g 终态 = no-go……") != [], "RED:缺 C16 行必红")
    expect(rfc0034_judge(good_rfc.replace("G27.2 M-b 重判", "G27.2 改写")) != [], "RED:原行字面破必红")
    expect(rfc0034_judge("") != [], "RED:空文本必红")
    # schema 互核。
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
    expect(REGISTRY_PATH.is_file(), "判档登记表在树")
    if REGISTRY_PATH.is_file():
        live = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))
        expect(registry_structure_judge(live) == [], "在树登记表结构判绿(selftest 同窗复核)")
        expect(verdict_enum_judge(live) == [], "在树登记表 verdict 判绿")
        expect(items_consistency_judge(live) == [], "在树登记表窗内一致判绿")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    expect(len(WINDOW_IDS) == 6, "窗闭集 = 6")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=8;红臂 17 + 正例组 + 在树登记表复核 + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        return run_gate()
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
