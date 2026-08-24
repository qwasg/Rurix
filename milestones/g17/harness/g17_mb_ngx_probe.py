#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G17.3 M-b NGX 版本演进面对齐评估波）
"""G17.3 M-b NGX 版本演进面双臂探针 driver（bistro-interior/t100/dlss_sr 单格）。

臂 A = 生产默认 310.5.2（external/streamline-2.10.3）；
臂 B = 评估缓存 310.6.0（external/streamline-2.10.3-ngx310.6.0，经
RURIX_STREAMLINE_SDK_DIR 显式 opt-in——生产默认目录 0-byte）。

每臂四类轮：
  ① no-timing ×3（A/B measured 三轮，frame_ms_production_mean 单变量对照）
  ② timing ×1（RURIX_VENDOR_TIMING=1，stderr 六段逐帧 → submit_wait 分布）
  ③ X2 ×1（RURIX_G17_DLSS_EVAL_X2=1 + timing，同 cmd 第二次 evaluate →
     submit_wait_x2 分布；边际 = median(x2) − median(x1) ≈ NGX in-stream 净成本）
  ④ SL verbose 日志捕获（stderr 全量存档 + NGX 实例化形态 token 提取——
     NGXCubinVulkan / PaddedWindowNetwork / DLTSS 等）
L0 digest 探针：每轮 receipt last_frame_digest 对 G14.12 冻结锚（B 臂新网络
预期 MISS——如实登记，超锚即拒绝换版门禁输入）。

输出 milestones/g17/g17_mb_ngx_probe_results.json（M-b 门唯一消费面）。

用法：py -3 milestones/g17/harness/g17_mb_ngx_probe.py
（GPU 独占窗执行；内部逐轮 gpu_device_lock）
"""
from __future__ import annotations

import datetime as _dt
import io
import json
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

BIN = ROOT / "target/release/g14_3_pipeline_perf.exe"
OUT_ROOT = Path(r"K:\rurix-ext\g14-frames\rurix_prod")
RECEIPT = OUT_ROOT / "bistro-interior" / "tier100" / "dlss_sr" / "bench_receipt.json"
ANCHOR_PATH = ROOT / "milestones/g14/g14_3_stage_a_digest_anchor.json"
EVAL_SDK_DIR = "external/streamline-2.10.3-ngx310.6.0"
OUT_JSON = ROOT / "milestones/g17/g17_mb_ngx_probe_results.json"
LOG_DIR = ROOT / ".tmp/g17_mb"
SCENE, TIER, BACKEND = "bistro-interior", 100, "dlss_sr"
TIMING_RE = re.compile(
    r"\[vendor-timing dlss-ext\] frame=(\d+) staging=([0-9.]+) sl_book=([0-9.]+) "
    r"record=([0-9.]+) evaluate=([0-9.]+) submit_wait=([0-9.]+) total=([0-9.]+)ms"
)
BENCH_RE = re.compile(r"BENCH PASS scene=\S+ tier=\d+ backend=\S+ warmup=\d+ frames=\d+"
                      r" frame_ms_mean=([0-9.]+) cv=([0-9.]+)")
NGX_TOKENS = ("NGXCubinVulkan", "PaddedWindowNetwork", "NGXCubinKernelMap",
              "DLTSS", "CreateDlssInstance", "NGXDLAA", "InternalHistory")
WARMUP_DROP = 20  # timing 分布弃 warmup 帧（G15plus-II 同口径 n=150 弃 20 前）


def run_bench(arm: str, kind: str, extra_env: dict[str, str]) -> dict:
    env = dict(**__import__("os").environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    if arm == "b":
        env["RURIX_STREAMLINE_SDK_DIR"] = EVAL_SDK_DIR
    env.update(extra_env)
    t0 = time.time()
    r = subprocess.run(
        [str(BIN), "--bench", "--scene", SCENE, "--tier", str(TIER),
         "--backend", BACKEND, "--frames", "160", "--warmup", "10"],
        cwd=ROOT, capture_output=True, text=True, timeout=7200, env=env,
    )
    out = (r.stdout or "") + (r.stderr or "")
    m = BENCH_RE.search(out)
    rec = wel.load_json(RECEIPT) if RECEIPT.is_file() and RECEIPT.stat().st_mtime >= t0 - 5 else {}
    sp = rec.get("stats_post_warmup") or {}
    frames = TIMING_RE.findall(out)
    sw = [float(f[5]) for f in frames][WARMUP_DROP:]
    ev = [float(f[4]) for f in frames][WARMUP_DROP:]
    return {
        "arm": arm, "kind": kind, "exit": r.returncode, "ok": r.returncode == 0 and bool(m),
        "frame_ms_mean": float(m.group(1)) if m else None,
        "frame_ms_production_mean": float(sp["frame_ms_production_mean"]) if "frame_ms_production_mean" in sp else None,
        "last_frame_digest": str(rec.get("last_frame_digest", "")),
        "submit_wait_median_ms": statistics.median(sw) if sw else None,
        "submit_wait_mean_ms": statistics.fmean(sw) if sw else None,
        "evaluate_cpu_median_ms": statistics.median(ev) if ev else None,
        "timing_frames_n": len(sw),
        "_stderr": out,
    }


def main() -> int:
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    anchors = wel.load_json(ANCHOR_PATH).get("anchors", {})
    anchor_digest = (anchors.get(f"{SCENE}_t{TIER}_{BACKEND}") or {}).get("last_frame_digest", "")
    started = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    results: dict = {
        "schema": "rurix.g17.mb_ngx_probe.v1",
        "started_utc": started,
        "cell": f"{SCENE}/t{TIER}/{BACKEND}",
        "anchor_digest": anchor_digest,
        "eval_sdk_dir": EVAL_SDK_DIR,
        "g15_baseline_literal": {
            "in_stream_ms": 1.90, "submit_fixed_ms": 0.10,
            "source": "G15_CONTRACT §8.7 X2 边际探针分解字面（对照基线，非本窗实测）",
        },
        "arms": {},
    }
    for arm in ("a", "b"):
        arm_out: dict = {"rounds": []}
        sl_log_saved = False
        with gpu_device_lock(purpose=f"g17_mb_ngx_probe arm={arm}"):
            for kind, env in (("notiming_1", {}), ("notiming_2", {}), ("notiming_3", {}),
                              ("timing_x1", {"RURIX_VENDOR_TIMING": "1"}),
                              ("timing_x2", {"RURIX_VENDOR_TIMING": "1",
                                             "RURIX_G17_DLSS_EVAL_X2": "1"})):
                print(f"[g17_mb_probe] arm={arm} kind={kind} …", flush=True)
                rr = run_bench(arm, kind, env)
                stderr = rr.pop("_stderr")
                if not sl_log_saved and any(t in stderr for t in NGX_TOKENS):
                    log_path = LOG_DIR / f"arm_{arm}_sl.log"
                    io.open(log_path, "w", encoding="utf-8", newline="\n").write(stderr)
                    arm_out["sl_log_path"] = str(log_path.relative_to(ROOT)).replace("\\", "/")
                    arm_out["ngx_tokens_found"] = sorted(
                        {t for t in NGX_TOKENS if t in stderr}
                    )
                    sl_log_saved = True
                arm_out["rounds"].append(rr)
                if not rr["ok"]:
                    print(f"[g17_mb_probe] arm={arm} kind={kind} FAIL exit={rr['exit']}", flush=True)
        # 汇总
        nt = [r["frame_ms_production_mean"] for r in arm_out["rounds"]
              if r["kind"].startswith("notiming") and r["frame_ms_production_mean"]]
        x1 = next((r for r in arm_out["rounds"] if r["kind"] == "timing_x1"), {})
        x2 = next((r for r in arm_out["rounds"] if r["kind"] == "timing_x2"), {})
        digs = {r["last_frame_digest"] for r in arm_out["rounds"] if r.get("last_frame_digest")}
        arm_out["summary"] = {
            "notiming_prod_ms": sorted(nt),
            "notiming_prod_median_ms": statistics.median(nt) if nt else None,
            "submit_wait_x1_median_ms": x1.get("submit_wait_median_ms"),
            "submit_wait_x2_median_ms": x2.get("submit_wait_median_ms"),
            "in_stream_marginal_median_ms": (
                x2["submit_wait_median_ms"] - x1["submit_wait_median_ms"]
                if x1.get("submit_wait_median_ms") is not None and x2.get("submit_wait_median_ms") is not None
                else None
            ),
            "digests": sorted(digs),
            "digest_anchor_hit": digs == {anchor_digest} if digs else False,
            "all_rounds_ok": all(r["ok"] for r in arm_out["rounds"]),
        }
        results["arms"][arm] = arm_out
    a, b = results["arms"]["a"]["summary"], results["arms"]["b"]["summary"]
    verdict = "reject_version_swap" if not b["digest_anchor_hit"] else "candidate_adopt_pending_quality_band"
    results["adoption_verdict"] = {
        "verdict": verdict,
        "basis": (
            "画质守护双门禁第一门（Stage A digest 锚零漂移）："
            f"B 臂 digest_anchor_hit={b['digest_anchor_hit']}"
            + ("——310.6.0 新网络输出位面 ≠ 冻结锚，超锚即拒绝换版（如实登记，判据字面）；"
               "in-stream 分解对照数据留档为 M-c 决策树与 G18+ 锚重收割立项输入面"
               if not b["digest_anchor_hit"] else "——第一门禁 HIT，进入画质锚带复核第二门禁"),
        ),
    }
    OUT_JSON.write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
    print(f"[g17_mb_probe] 完成 → {OUT_JSON}")
    print(f"  A 臂: prod={a['notiming_prod_median_ms']} x1={a['submit_wait_x1_median_ms']} "
          f"x2={a['submit_wait_x2_median_ms']} 边际={a['in_stream_marginal_median_ms']} hit={a['digest_anchor_hit']}")
    print(f"  B 臂: prod={b['notiming_prod_median_ms']} x1={b['submit_wait_x1_median_ms']} "
          f"x2={b['submit_wait_x2_median_ms']} 边际={b['in_stream_marginal_median_ms']} hit={b['digest_anchor_hit']}")
    print(f"  verdict = {verdict}")
    return 0 if (a["all_rounds_ok"] and b["all_rounds_ok"]) else 1


if __name__ == "__main__":
    sys.exit(main())
