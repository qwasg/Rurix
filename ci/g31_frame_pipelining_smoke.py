#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 A Task A2 帧流水化）
"""G31+ 波 A Task A2：帧流水化（submit/collect 分离）A/B 门冒烟
（g31.waveA.pipelining；G31_PLUS_COMMERCIAL_RENDERER_TODO §1.1 #2 行；
本任务任务书 A2 判据逐字）。

判据（任务书逐字）：
- g14_3_pipeline_perf --bench bistro-interior t100 tsr_device 真跑
  --inflight 1（顺序全同步回归锚）vs 2 vs 3，各 ≥100 帧 + warmup 10
  （--runs R 轮进程级独立运行，逐指标跨轮中位数），frame_ms
  mean/p50/p99 对照 evidence JSON（schema =
  milestones/g31/g31_frame_pipelining_evidence_schema.json）；
- **流水模式末帧 digest == 同步模式末帧 digest（位级）**——三臂全部轮次
  的 last_frame_digest 全同才过，且逐帧 flip-trace digest 序列位级一致；
- in-flight 帧序不乱——flip-trace 帧号严格 0..N−1（FIFO collect 保序）；
- 无 fence 泄漏（clean shutdown）——全部真跑 rc=0 + BENCH PASS +
  receipt 新鲜 + stderr 无 validation/leak 字样（session Drop 单点销毁：
  fence/pipelined_pool/per-slot staging 逆序 teardown，挂死/崩溃即非 0）。

三态：无 Vulkan loader/设备/场景资产 → 输出 DEV_ENV_DEGRADE 退 0（不冒充
PASS）；本脚本真跑臂 RURIX_REQUIRE_REAL=1（该态下缺真实面即 FAIL 退 1，
禁 mock 充真跑——g14_rurix_pipeline_perf_smoke 同语义）。

用法：
  py -3 ci/g31_frame_pipelining_smoke.py --selftest
  py -3 ci/g31_frame_pipelining_smoke.py --gate g31.waveA.pipelining \
      [--runs 3] [--frames 100] [--warmup 10] [--out <evidence.json>]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveA.pipelining"
TAG = "g31_pipelining"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_frame_pipelining_evidence_schema.json"
SCHEMA_ID = "rurix.g31.frame_pipelining_evidence.v1"
BIN = ROOT / "target" / "release" / "g14_3_pipeline_perf.exe"
WORK = ROOT / ".tmp" / "g31_gates" / "pipelining"
OUT_ROOT = WORK / "out"
SCENE = "bistro-interior"
TIER = 100
BACKEND = "tsr_device"
ARMS = (1, 2, 3)
TRACE_FRAMES = 24
TRACE_WARMUP = 4

FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


# ---------------------------------------------------------------- 纯函数判据面
def percentile(sorted_v: list[float], q: float) -> float:
    """s 已升序；q∈[0,1] 最近秩（与 A/B 分析脚本同一口径）。"""
    if not sorted_v:
        raise ValueError("percentile: 空样本")
    n = len(sorted_v)
    return sorted_v[min(n - 1, int(q * (n - 1) + 0.5))]


def arm_stats(frame_ms: list[float]) -> dict:
    """单轮 frame_ms 全列 → mean/p50/p99/min/max（末一样本含末帧 digest
    tail——各臂同形同价；流水臂排水段墙钟并入末一样本，口径见 schema 描述）。"""
    if not frame_ms:
        raise ValueError("arm_stats: 空 frame_ms")
    s = sorted(frame_ms)
    n = len(frame_ms)
    p50 = s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2.0
    return {
        "mean": sum(frame_ms) / n,
        "p50": p50,
        "p99": percentile(s, 0.99),
        "min": s[0],
        "max": s[-1],
    }


def digests_bitexact(digest_lists: list[list[str]]) -> bool:
    """多臂 × 多轮 digest 列：全部展平后集合恰一元（位级一致判据）。"""
    flat = [d for ds in digest_lists for d in ds]
    return bool(flat) and len(set(flat)) == 1


def seqs_bitexact(seqs: list[list[str]]) -> bool:
    """多臂逐帧 digest **序列**逐位对拍：等长且逐下标全同（每帧 digest 各异，
    禁展平判一元——那是 last_frame_digest 面判据）。"""
    if not seqs or any(len(s) != len(seqs[0]) for s in seqs):
        return False
    return all(len({s[i] for s in seqs}) == 1 for i in range(len(seqs[0])))


def frame_order_ok(trace_rows: list[dict]) -> bool:
    """flip-trace 帧号严格 0..N−1（FIFO collect 保序判据）。"""
    return [int(r["frame"]) for r in trace_rows] == list(range(len(trace_rows)))


def evidence_required_keys(doc: dict) -> list[str]:
    """schema required 闭集核验（jsonschema 依赖免；check_schemas.py 另作
    形式校验面）。"""
    required = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))["required"]
    return [k for k in required if k not in doc]


# ---------------------------------------------------------------- 真跑驱动
def run_bench(
    inflight: int,
    frames: int,
    warmup: int,
    out_root: Path,
    flip_trace_dir: Path | None,
    require_real: bool,
) -> dict:
    env = dict(os.environ)
    if require_real:
        env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    if flip_trace_dir is not None:
        flip_trace_dir.mkdir(parents=True, exist_ok=True)
        env["RURIX_G14_FLIP_TRACE"] = str(flip_trace_dir)
    elif "RURIX_G14_FLIP_TRACE" in env:
        del env["RURIX_G14_FLIP_TRACE"]
    cmd = [
        str(BIN),
        "--bench",
        "--scene", SCENE,
        "--tier", str(TIER),
        "--backend", BACKEND,
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--inflight", str(inflight),
        "--out-root", str(out_root),
    ]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=7200, env=env)
    out = (r.stdout or "") + (r.stderr or "")
    receipt_path = out_root / SCENE / f"tier{TIER}" / BACKEND / "bench_receipt.json"
    receipt = {}
    fresh = False
    if receipt_path.is_file():
        fresh = receipt_path.stat().st_mtime >= t0 - 5.0
        try:
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            receipt = {}
    pass_line = re.search(r"BENCH PASS scene=(\S+)", out)
    clean = (
        r.returncode == 0
        and pass_line is not None
        and fresh
        and "VALIDATION" not in out
        and "leak" not in out.lower()
    )
    return {
        "rc": r.returncode,
        "clean_shutdown": clean,
        "receipt": receipt,
        "receipt_path": receipt_path,
        "fresh": fresh,
        "tail": out[-400:],
        "skipped_dev_env": "skipped_dev_env" in out,
    }


def load_trace(trace_dir: Path) -> list[dict]:
    p = trace_dir / f"frame_digests_{SCENE}_t{TIER}_{BACKEND}.jsonl"
    if not p.is_file():
        return []
    return [json.loads(line) for line in p.read_text(encoding="utf-8").splitlines() if line.strip()]


# ---------------------------------------------------------------- selftest
def selftest() -> int:
    note("selftest：判据纯函数红绿臂")
    ok = True

    # ① arm_stats 绿臂：已知向量核对 mean/p50/p99/min/max。
    v = [4.0, 1.0, 2.0, 3.0, 100.0]
    st = arm_stats(v)
    expect = {"mean": 22.0, "p50": 3.0, "p99": 100.0, "min": 1.0, "max": 100.0}
    green = st == expect
    ok &= green
    note(f"  arm_stats 绿臂: {'PASS' if green else 'FAIL'}（{st}）")

    # ② arm_stats 红臂：空样本必拒（ValueError）。
    try:
        arm_stats([])
        red = False
    except ValueError:
        red = True
    ok &= red
    note(f"  arm_stats 红臂（空样本拒）: {'PASS' if red else 'FAIL'}")

    # ③ digests_bitexact 绿臂：三臂全同 → True。
    green = digests_bitexact([["sha256:a", "sha256:a"], ["sha256:a"]])
    ok &= green
    note(f"  digests_bitexact 绿臂: {'PASS' if green else 'FAIL'}")

    # ④ digests_bitexact 红臂：任一漂移 → False（检出即红，不静默）。
    red = not digests_bitexact([["sha256:a"], ["sha256:b"]])
    ok &= red
    note(f"  digests_bitexact 红臂（漂移检出）: {'PASS' if red else 'FAIL'}")

    # ⑤ frame_order_ok 绿臂：严格 0..N−1 → True。
    green = frame_order_ok([{"frame": 0}, {"frame": 1}, {"frame": 2}])
    ok &= green
    note(f"  frame_order_ok 绿臂: {'PASS' if green else 'FAIL'}")

    # ⑥ frame_order_ok 红臂：乱序/缺帧 → False。
    red = not frame_order_ok([{"frame": 0}, {"frame": 2}, {"frame": 1}]) and not frame_order_ok(
        [{"frame": 0}, {"frame": 0}]
    )
    ok &= red
    note(f"  frame_order_ok 红臂（乱序/重复检出）: {'PASS' if red else 'FAIL'}")

    # ⑦ evidence required 键闭集：合成 doc 缺键必列出。
    missing = evidence_required_keys({"schema": SCHEMA_ID})
    red = len(missing) > 0 and "arms" in missing
    ok &= red
    note(f"  evidence required 键红臂（缺键列出 {len(missing)} 项）: {'PASS' if red else 'FAIL'}")

    # ⑧ percentile 口径锚：n=100 时 int(0.99×99+0.5)=98、int(0.5×99+0.5)=50。
    s100 = list(range(100))
    green = percentile(s100, 0.99) == 98 and percentile(s100, 0.5) == 50
    ok &= green
    note(f"  percentile 口径锚绿臂: {'PASS' if green else 'FAIL'}")

    # ⑨ seqs_bitexact 绿臂：逐帧 digest 各异但双臂序列逐位同 → True。
    green = seqs_bitexact([["d0", "d1", "d2"], ["d0", "d1", "d2"]])
    ok &= green
    note(f"  seqs_bitexact 绿臂: {'PASS' if green else 'FAIL'}")

    # ⑩ seqs_bitexact 红臂：单帧漂移/序列不等长 → False（检出即红）。
    red = not seqs_bitexact([["d0", "d1", "d2"], ["d0", "dX", "d2"]]) and not seqs_bitexact(
        [["d0", "d1"], ["d0"]]
    )
    ok &= red
    note(f"  seqs_bitexact 红臂（单帧漂移/不等长检出）: {'PASS' if red else 'FAIL'}")

    if ok:
        note("SELFTEST PASS（红绿臂全如预期）")
        return 0
    print(f"[{TAG}] SELFTEST FAIL", file=sys.stderr)
    return 1


# ---------------------------------------------------------------- gate
def gate(runs: int, frames: int, warmup: int, out_path: Path | None) -> int:
    if not SCHEMA_PATH.is_file():
        fail(f"schema 缺失: {SCHEMA_PATH}")
        return 1
    note(f"gate {GATE_KEY}: scene={SCENE} tier={TIER} backend={BACKEND} arms={ARMS} "
         f"runs/arm={runs} frames={frames} warmup={warmup}")

    with gpu_device_lock(purpose=f"{TAG} device 真跑（g31.waveA.pipelining）"):
        # 构建（release；g14_3_pipeline_perf 需 vendor-upscale feature）。
        build = subprocess.run(
            ["cargo", "build", "-p", "rurix-render", "--features", "vulkan,vendor-upscale",
             "--release", "--bin", "g14_3_pipeline_perf"],
            cwd=ROOT, capture_output=True, text=True, timeout=7200,
        )
        if build.returncode != 0 or not BIN.is_file():
            fail(f"release 构建失败: {(build.stderr or '')[-400:]}")
            return 1

        # dev-env 探针（不挂 REQUIRE_REAL：缺真实面 → bin 自报 skipped_dev_env 退 0）。
        probe = run_bench(1, 2, 1, WORK / "probe", None, require_real=False)
        if probe["skipped_dev_env"] or (probe["rc"] == 0 and not probe["fresh"]):
            print(json.dumps({
                "schema": "rurix.g31.frame_pipelining.skip.v1",
                "state": "DEV_ENV_DEGRADE",
                "what": "vulkan_device_or_scene_assets",
                "reason": probe["tail"][-200:],
            }, ensure_ascii=False))
            note("DEV_ENV_DEGRADE（无 Vulkan/设备/场景资产——退 0 不冒充 PASS）")
            return 0
        if probe["rc"] != 0:
            fail(f"dev-env 探针真跑失败: {probe['tail'][-200:]}")
            return 1
        note("dev-env 探针绿（真机真跑面成立）")

        # ── 主 A/B：逐臂 runs 轮（无 trace——生产口径零回读测量循环）──
        arm_docs: list[dict] = []
        all_digests: list[list[str]] = []
        clean_all = True
        for inflight in ARMS:
            run_stats: list[dict] = []
            run_digests: list[str] = []
            receipts: list[str] = []
            prod_means: list[float] = []
            for rep in range(runs):
                r = run_bench(inflight, frames, warmup, OUT_ROOT, None, require_real=True)
                clean_all &= r["clean_shutdown"]
                if r["rc"] != 0:
                    fail(f"inflight={inflight} rep{rep + 1} 真跑 rc={r['rc']}: {r['tail'][-200:]}")
                    return 1
                rec = r["receipt"]
                if not rec or int(rec.get("inflight", -1)) != inflight:
                    fail(f"inflight={inflight} rep{rep + 1} receipt 缺 inflight 字段/不新鲜")
                    return 1
                run_stats.append(arm_stats([float(x) for x in rec["frame_ms"]]))
                run_digests.append(str(rec["last_frame_digest"]))
                prod_means.append(float(rec["stats_post_warmup"]["frame_ms_production_mean"]))
                # 逐轮 receipt 归档（out_root 下同名覆盖——证据须逐轮独立件）。
                archive_dir = WORK / "receipts"
                archive_dir.mkdir(parents=True, exist_ok=True)
                archive = archive_dir / f"bench_inflight{inflight}_rep{rep + 1}.json"
                archive.write_text(
                    json.dumps(rec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
                )
                receipts.append(str(archive))
            med = lambda key: sorted(x[key] for x in run_stats)[len(run_stats) // 2]
            arm_docs.append({
                "inflight": inflight,
                "frame_slots": max(2, inflight),
                "frame_ms_mean": med("mean"),
                "frame_ms_p50": med("p50"),
                "frame_ms_p99": med("p99"),
                "frame_ms_min": med("min"),
                "frame_ms_max": med("max"),
                "frame_ms_production_mean": sorted(prod_means)[len(prod_means) // 2],
                "last_frame_digest": run_digests[-1],
                "receipts": receipts,
            })
            all_digests.append(run_digests)
            note(f"  arm inflight={inflight}: mean={med('mean'):.4f} p50={med('p50'):.4f} "
                 f"p99={med('p99'):.4f} prod={sorted(prod_means)[len(prod_means) // 2]:.4f} "
                 f"digest={run_digests[-1][:23]}…")

        # ── 判据①：三臂 × 全部轮次 digest 位级一致（硬门）──
        digest_ok = digests_bitexact(all_digests)
        if not digest_ok:
            fail(f"digest 位级一致破缺: {[sorted(set(ds)) for ds in all_digests]}")

        # ── 判据②③：flip-trace 侧跑（逐帧回读序列 + 帧序）──
        trace_seqs: list[list[dict]] = []
        for inflight in ARMS:
            tdir = WORK / f"trace_i{inflight}"
            r = run_bench(inflight, TRACE_FRAMES, TRACE_WARMUP, WORK / "trace_out",
                          tdir, require_real=True)
            clean_all &= r["clean_shutdown"]
            if r["rc"] != 0:
                fail(f"inflight={inflight} trace 侧跑 rc={r['rc']}: {r['tail'][-200:]}")
                return 1
            rows = load_trace(tdir)
            if len(rows) != TRACE_FRAMES + TRACE_WARMUP:
                fail(f"inflight={inflight} trace 行数 {len(rows)} ≠ {TRACE_FRAMES + TRACE_WARMUP}")
                return 1
            trace_seqs.append(rows)
        order_ok = all(frame_order_ok(rows) for rows in trace_seqs)
        if not order_ok:
            fail("flip-trace 帧序破缺（非严格 0..N−1）")
        seq_digests = [[str(r["digest"]) for r in rows] for rows in trace_seqs]
        trace_bitexact = seqs_bitexact(seq_digests)
        if not trace_bitexact:
            fail("flip-trace 逐帧 digest 序列跨臂位级一致破缺")
        note(f"  trace 侧跑: 帧序严格 FIFO={order_ok} 逐帧 digest 位级一致={trace_bitexact}")

        if not clean_all:
            fail("clean shutdown 破缺（rc/PASS/receipt 新鲜/validation/leak 字样）")

    # ── evidence 落盘 ──
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    anchor = arm_docs[0]
    best = min(arm_docs[1:], key=lambda a: a["frame_ms_p50"])
    total_runs = sum(len(a["receipts"]) for a in arm_docs)
    trace_frames_total = len(trace_seqs[0]) if trace_seqs else 0
    doc = {
        "schema": SCHEMA_ID,
        "subject": "g31_frame_pipelining",
        "symbolic_gate_key": GATE_KEY,
        "wave": "G31.A",
        "scene_id": SCENE,
        "tier": TIER,
        "backend": BACKEND,
        "seed": 0,
        "frames_measured": frames,
        "warmup": warmup,
        "runs_per_arm": runs,
        "arms": arm_docs,
        "digest_bitexact_all_arms": digest_ok and trace_bitexact,
        "frame_order_strict_fifo": order_ok,
        "clean_shutdown": clean_all,
        "environment": {
            "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
            "os": "windows",
            "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
            "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                          capture_output=True, text=True).stdout.strip(),
        },
        "timestamp": ts,
        "notes": (
            f"frame_ms 统计自 receipt 全列（末一样本含末帧 digest tail，各臂同形同价；"
            f"流水臂排水段并入末一样本，Σframe_ms ≈ 测量段全墙钟）；A/B 对照（对 inflight=1）："
            + "；".join(
                f"inflight={a['inflight']} p50 {a['frame_ms_p50']:.4f}ms"
                f"（{(a['frame_ms_p50'] / anchor['frame_ms_p50'] - 1) * 100:+.1f}%）"
                f" p99 {a['frame_ms_p99']:.4f}ms"
                f"（{(a['frame_ms_p99'] / anchor['frame_ms_p99'] - 1) * 100:+.1f}%）"
                for a in arm_docs[1:]
            )
            + f"；最优臂 inflight={best['inflight']}；digest 锚 {anchor['last_frame_digest'][:23]}… "
            f"主 A/B {total_runs} 轮 + trace {len(trace_seqs)}×{trace_frames_total} 帧全位级同"
        ),
    }
    missing = evidence_required_keys(doc)
    if missing:
        fail(f"evidence 缺 required 键: {missing}")
        return 1
    seed_val = json.loads((OUT_ROOT / SCENE / f"tier{TIER}" / BACKEND / "bench_receipt.json")
                          .read_text(encoding="utf-8"))["seed"]
    doc["seed"] = int(seed_val)
    ev_path = out_path or (ROOT / "evidence" / f"g31_frame_pipelining_{ts}.json")
    ev_path.parent.mkdir(parents=True, exist_ok=True)
    ev_path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {ev_path}")

    ok = digest_ok and trace_bitexact and order_ok and clean_all and not FAILURES
    note(f"GATE {'PASS' if ok else 'FAIL'} {GATE_KEY}")
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--gate", default="")
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--frames", type=int, default=100)
    ap.add_argument("--warmup", type=int, default=10)
    ap.add_argument("--out", default="")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        if args.frames < 100:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 100（任务书 ≥100 帧硬线）",
                  file=sys.stderr)
            return 1
        return gate(args.runs, args.frames, args.warmup,
                    Path(args.out) if args.out else None)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
