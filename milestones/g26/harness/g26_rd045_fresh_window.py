#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G26.3 M-c RD-045 backfill 重判波）
"""G26.3 M-c RD-045 backfill 重判 driver（新鲜观察窗 + 三件盘点，RFC-0043 §3）。

新鲜观察窗 = G19.3 长窗协议同模（bistro-interior/t50/tsr_device canonical 160 帧
bench 真跑，逐轮 receipt last_frame_digest 对 G14.12 冻结锚比对），默认 6 轮
（重判新鲜度窗口径，非 G19.3 ≥12 长窗复刻——窗长如实登记）。

三件盘点（backfill_condition 字面；F5 防冒充硬线）：
① 根因定位——唯一合法证据形态 = 候选面四项（首进程冷启动态/异步拷贝竞争/
   未初始化读取/浮点归约序）之一的逐字确证记录（指定落点
   evidence/g26_rd045_root_cause_confirmation.json，须含 candidate/repro_path/
   mechanism 三字段）；观察性证据（零复现）**永不充当**①件——本盘点对①件的
   判定输入面不含观察窗结果。
② 生产化缺陷修复——①未齐时修复无法确证（G14.10 结构性缓解在案 ≠ 确证修复；
   缓解事实如实登记不充②件）。
③ Full RFC 评估——指定落点 = 以 RD-045 确定性协议缺陷为主题的 Full RFC
   （rfcs/ 检索 rd045/digest-drift 主题文件）；RFC-0043 §3 只定重判程序，
   不自动构成③件。

决策树：三件全齐 → close；任一未齐 → maintain-open 只追加扩窗（零冒充）；
观察窗检出漂移 → drift-detected-escalate。

输出 milestones/g26/g26_rd045_fresh_window_results.json（M-c 门唯一消费面）。

用法：py -3 milestones/g26/harness/g26_rd045_fresh_window.py [--rounds 6]
（GPU 独占窗执行；内部 gpu_device_lock）
"""
from __future__ import annotations

import datetime as _dt
import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

BIN = ROOT / "target/release/g14_3_pipeline_perf.exe"
OUT_ROOT = Path(r"K:\rurix-ext\g14-frames\rurix_prod")
RECEIPT = OUT_ROOT / "bistro-interior" / "tier50" / "tsr_device" / "bench_receipt.json"
ANCHOR_PATH = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
OUT_JSON = ROOT / "milestones/g26/g26_rd045_fresh_window_results.json"
LOG_DIR = ROOT / ".tmp/g26_mc"
SCENE, TIER, BACKEND = "bistro-interior", 50, "tsr_device"

ROOT_CAUSE_CONFIRMATION = ROOT / "evidence/g26_rd045_root_cause_confirmation.json"
CANDIDATES = ("首进程冷启动态", "异步拷贝竞争", "未初始化读取", "浮点归约序")


def run_round(idx: int) -> dict:
    import os
    import subprocess

    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    t0 = time.time()
    r = subprocess.run(
        [str(BIN), "--bench", "--scene", SCENE, "--tier", str(TIER),
         "--backend", BACKEND, "--frames", "160", "--warmup", "10"],
        cwd=ROOT, capture_output=True, text=True, timeout=3600, env=env,
    )
    ok = r.returncode == 0
    rec = (
        wel.load_json(RECEIPT)
        if ok and RECEIPT.is_file() and RECEIPT.stat().st_mtime >= t0 - 5
        else {}
    )
    digest = str(rec.get("last_frame_digest", ""))
    if not ok:
        fl = LOG_DIR / f"round_{idx:02d}_fail.log"
        fl.write_text((r.stdout or "") + (r.stderr or ""), encoding="utf-8", newline="\n")
    return {"round": idx, "exit": r.returncode, "ok": ok, "last_frame_digest": digest,
            "wall_s": round(time.time() - t0, 2)}


def piece_inventory() -> dict:
    """三件盘点（①件判定输入面禁引观察窗结果——本函数不接收观察窗参数）。"""
    p1_doc = None
    p1_met = False
    p1_basis = "指定落点 evidence/g26_rd045_root_cause_confirmation.json 不存在（树内实测）"
    if ROOT_CAUSE_CONFIRMATION.is_file():
        try:
            p1_doc = json.loads(ROOT_CAUSE_CONFIRMATION.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            p1_doc = None
        if p1_doc and p1_doc.get("candidate") in CANDIDATES \
                and p1_doc.get("repro_path") and p1_doc.get("mechanism"):
            p1_met = True
            p1_basis = f"确证记录在档：candidate={p1_doc.get('candidate')}"
        else:
            p1_basis = "落点文件存在但三字段（candidate/repro_path/mechanism）未齐——不构成确证"
    # ② 修复确证：①未齐时无法确证（G14.10 结构性缓解为缓解事实，如实登记不充②）。
    p2_met = False
    p2_basis = (
        "①根因未确证 → 修复无法确证；G14.10 帧循环重构结构性缓解在案"
        "（registry/deferred.json RD-045 history 2026-08-22 行）——缓解事实 ≠ 确证修复，不充②件"
        if not p1_met else "①已确证——②按修复记录另行盘点（本窗未触达）"
    )
    # ③ Full RFC 评估：rfcs/ 树内检索 RD-045/digest-drift 主题 Full RFC。
    rfc_hits = [p.name for p in sorted((ROOT / "rfcs").glob("*.md"))
                if any(k in p.name for k in ("rd045", "digest-drift", "determinism-defect"))]
    p3_met = bool(rfc_hits)
    p3_basis = (f"rfcs/ 主题检索命中：{rfc_hits}" if rfc_hits
                else "rfcs/ 主题检索零命中（rd045/digest-drift/determinism-defect 文件名闭集）；"
                     "RFC-0043 §3 只定重判程序不自动构成③件")
    return {
        "piece1_root_cause_located": {"met": p1_met, "basis": p1_basis,
                                      "input_surface": "designated_confirmation_file_only（F5：不含观察窗结果）"},
        "piece2_production_fix": {"met": p2_met, "basis": p2_basis},
        "piece3_full_rfc_evaluation": {"met": p3_met, "basis": p3_basis},
        "met_count": sum(int(x) for x in (p1_met, p2_met, p3_met)),
    }


def main() -> int:
    rounds_n = 6
    args = sys.argv[1:]
    if "--rounds" in args:
        rounds_n = int(args[args.index("--rounds") + 1])
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    anchors = wel.load_json(ANCHOR_PATH).get("anchors", {})
    anchor = (anchors.get(f"{SCENE}_t{TIER}_{BACKEND}") or {}).get("last_frame_digest", "")
    started = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    rows: list[dict] = []
    with gpu_device_lock(purpose="g26_rd045_fresh_window"):
        for i in range(1, rounds_n + 1):
            rr = run_round(i)
            rows.append(rr)
            hit = rr["last_frame_digest"] == anchor
            print(f"[g26_rd045] round {i}/{rounds_n} exit={rr['exit']} "
                  f"digest_hit={hit} wall={rr['wall_s']}s", flush=True)
    hits = sum(1 for r in rows if r["ok"] and r["last_frame_digest"] == anchor)
    drift_rounds = [r["round"] for r in rows if r["ok"] and r["last_frame_digest"] != anchor]
    all_ok = all(r["ok"] for r in rows)
    zero_drift = all_ok and not drift_rounds
    inventory = piece_inventory()
    if not zero_drift:
        disposition = "drift-detected-escalate"
        basis = "新鲜观察窗内检出漂移轮——升级登记（漂移轮 stderr 在 .tmp/g26_mc/）"
    elif inventory["met_count"] == 3:
        disposition = "closed"
        basis = "backfill 三件全齐（确证记录/修复记录/Full RFC 评估均在档）→ close"
    else:
        disposition = "maintain-open-with-extended-zero-recurrence"
        basis = (
            f"三件盘点 {inventory['met_count']}/3 未全齐（①根因确证记录缺 + ②修复无法确证 + "
            "③Full RFC 评估缺）——新鲜窗零漂移只进累计观察面（F5：不充①件），"
            "maintain-open + history 只追加扩窗登记，不冒充 close"
        )
    results = {
        "schema": "rurix.g26.rd045_fresh_window.v1",
        "started_utc": started,
        "cell": f"{SCENE}/t{TIER}/{BACKEND}",
        "anchor_digest": anchor,
        "rounds_requested": rounds_n,
        "rounds": rows,
        "summary": {
            "rounds_ok": sum(1 for r in rows if r["ok"]),
            "digest_anchor_hits": hits,
            "drift_rounds": drift_rounds,
            "zero_drift": zero_drift,
        },
        "backfill_inventory": inventory,
        "disposition": disposition,
        "disposition_basis": basis,
        "window_note": "新鲜度重判窗 6 轮口径（G19.3 ≥12 长窗协议同模不同窗长，如实登记）",
    }
    OUT_JSON.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
    print(f"[g26_rd045] 完成 → {OUT_JSON}")
    print(f"  hits={hits}/{rounds_n} drift={drift_rounds} 三件={inventory['met_count']}/3 → {disposition}")
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
