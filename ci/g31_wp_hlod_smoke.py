#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G31+ #95/#68/#99 WP cell + HLOD 生产接线门（gate g31.wave95.wp_hlod）。

判据闭集（G31_PLUS_COMMERCIAL_RENDERER_TODO §7 #95/#68/#99 行交付判据方向;
计划 F 阶段验收字面 = 远景 draw/三角数下降 measured + 切换互斥机核 + 固定
轨迹 popping 指标进 evidence）：
① schema 在树 + required 闭集互核 + 构建必绿；
② 资产链三步真跑：`g14_3_pipeline_perf --dump-scene`（RXCS 装配 dump）→
   `g31_wp_hlod_bake --double-build`（XZ cell 网格 + 逐 cell 跨组件合并 +
   QEM 链 → RXWH cell 包,双 bake 字节相等 = 确定性门）→ 生产车道消费；
③ **全 Full 对拍锚**：`--wp-hlod full` 末帧 digest 与 `off`（既有三角汤）
   **位级一致**（bin 内嵌逐三角位级断言 + 端到端 GPU digest 双证）；
④ 互斥选层出帧（#68 HLOD 代理 GPU 绘制腿）：三档 t0 窗口臂 out_tris 严格
   单调降 + 全臂 out < src（远景三角数下降 measured）+ 混合臂 Full ≥1 且
   Hlod ≥1（同帧全量 XOR 代理并存,互斥机核 bin 内嵌 fail-closed）；
⑤ 确定性：bench on 双跑末帧 digest 位级一致 + 选层序列 digest 前缀一致；
⑥ 切换协议（#99 popping + #68 互斥切换）：g31 窗口 headless dolly 轨迹
   切换真实发生（switches ≥ 1）+ warmup 原子翻转协议逐事件机核
   （flip − request == warmup,bin 内嵌 fail-closed + sidecar 复核）+
   popping 指标（切换事件表/翻转三角跳变）进 evidence；
⑦ 四 RED 臂子进程独立检出（tamper-digest / event-order / double-draw /
   runtime-merge——机核能红证明）；
⑧ 画质差 measured 如实登记（off vs on 收敛帧 EXR 区域 diff,
   err_p95/err_max/超阈区域数;G6 纪律不设通过线）。

三态：无 Vulkan/SPV/bistro 资产 → SKIP DEV_ENV_DEGRADE 退 0（非 fake pass;
RURIX_REQUIRE_REAL=1 翻硬 FAIL）。PASS-only evidence：过门才落
evidence/g31_wp_hlod_<ts>.json,FAIL 诊断落 .tmp 不污染。

用法：python ci/g31_wp_hlod_smoke.py [--gate g31.wave95.wp_hlod]
      [--frames 8] [--warmup 2] [--window-frames 32] [--selftest]
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_wp_hlod_evidence_schema.json"
WORK_DIR = ROOT / ".tmp" / "g31_gates" / "wave95_wp_hlod"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.wave95.wp_hlod"
TAG = "g31_wp_hlod"
SCHEMA_ID = "rurix.g31.wp_hlod_evidence.v1"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
KERNEL_SRC = ROOT / "src" / "rurix-render" / "kernels"
SPV_FILES = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
)
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")

# 契约参数（本机 bistro-interior 实测标定:cell 4m × levels 4;三档 t0 =
# 混合/切换/激进——0.25 产 Full+Hlod 混合帧,4.0 在 dolly 轨迹产生切换,
# 8.0 验单调;--wp-warmup 4 预热协议）。
CELL_SIZE = "4.0"
LEVELS = "4"
T0_MIXED = 0.25
T0_SWITCH = 4.0
T0_AGGRESSIVE = 8.0
WARMUP_FRAMES = 4
RED_ARMS = ["tamper-digest", "event-order", "double-draw", "runtime-merge"]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []

REQUIRED_KEYS = [
    "schema",
    "gate",
    "scene",
    "tier",
    "backend",
    "src_tris",
    "bake",
    "full_anchor",
    "window_arms",
    "mixed_arm",
    "determinism",
    "switch_protocol",
    "red_arms",
    "quality_diff",
    "frame_ms",
    "commands",
    "notes",
]

BAKE_RE = re.compile(
    r"bake OK grid=\[(-?\d+),(-?\d+)\]\.\.\[(-?\d+),(-?\d+)\] cells=(\d+)/(\d+) "
    r"cell_size_m=([0-9.]+) levels=(\d+) cell_tris=\[(\d+),(\d+)\] passthrough=(\d+) "
    r"proxy_tris=\[([0-9, ]+)\] bytes=\d+ sha256=([0-9a-f]{64}) bake_ms=([0-9.]+)"
)
WP_RE = re.compile(
    r"wp-hlod mode=(\w+) cells full/hlod/culled/pending=(\d+)/(\d+)/(\d+)/(\d+) "
    r"\(resident=(\d+)/(\d+)\) tris: src=(\d+) passthrough=(\d+) full=(\d+) proxy=(\d+) "
    r"out=(\d+) \([0-9.]+%\) ticks=(\d+) stall_frames=(\d+) selection_digest=([0-9a-f]{16})"
)
BENCH_MS_RE = re.compile(r"BENCH PASS .*? frame_ms_mean=([0-9.]+)")
DIFF_RE = re.compile(
    r"PASS regions=\d+ err_max=([0-9.eE+-]+) err_p95=([0-9.eE+-]+) over=(\d+)"
)


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def target_dir() -> Path:
    return Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target")))


def run_cmd(argv: list[str], timeout: int = 3600, env: dict | None = None) -> subprocess.CompletedProcess:
    print(f"[{TAG}] $ {' '.join(argv)}")
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(argv), "exit_code": r.returncode})
    return r


def ensure_spv() -> list[str]:
    """五件 SPV 存在性保障（缺则经 rurixc --target vulkan 现编;.tmp 构建产物）。"""
    missing = [f for f in SPV_FILES if not (SPV_DIR / f).is_file()]
    if not missing:
        return []
    rurixc = target_dir() / "debug" / f"rurixc{EXE_SUFFIX}"
    if not rurixc.is_file():
        r = run_cmd(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"], timeout=7200)
        if r.returncode != 0 or not rurixc.is_file():
            return missing
    SPV_DIR.mkdir(parents=True, exist_ok=True)
    still = []
    for f in missing:
        src = KERNEL_SRC / f.replace(".spv", ".rx")
        if not src.is_file():
            still.append(f)
            continue
        r = run_cmd([str(rurixc), str(src), "--target", "vulkan", "-o", str(SPV_DIR / f)], timeout=1800)
        if r.returncode != 0 or not (SPV_DIR / f).is_file():
            still.append(f)
    return still


def bench_digest(out_root: Path) -> str | None:
    p = out_root / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
    if not p.is_file():
        return None
    try:
        return json.loads(p.read_text(encoding="utf-8")).get("last_frame_digest")
    except json.JSONDecodeError:
        return None


def run_selftest() -> int:
    """离线自证：正则四联 GREEN + schema required 互核（无 GPU 面）。"""
    fails = []
    m = BAKE_RE.search(
        "bake OK grid=[-1,-4]..[3,1] cells=20/30 cell_size_m=4 levels=4 cell_tris=[1,266643] "
        "passthrough=44024 proxy_tris=[1002585, 501277, 250628, 125310] bytes=71888387 "
        "sha256=" + "b" * 64 + " bake_ms=2763.7 -> x"
    )
    if not m or m.group(5) != "20" or m.group(12) != "1002585, 501277, 250628, 125310":
        fails.append("BAKE_RE 解析失败")
    m = WP_RE.search(
        "wp-hlod mode=on cells full/hlod/culled/pending=5/15/0/0 (resident=30/20) "
        "tris: src=1046609 passthrough=44024 full=224612 proxy=388975 out=657611 (62.8%) "
        "ticks=8 stall_frames=7 selection_digest=13268fe257df9f84"
    )
    if not m or m.group(2) != "5" or m.group(12) != "657611":
        fails.append("WP_RE 解析失败")
    m = BENCH_MS_RE.search(
        "BENCH PASS scene=bistro-interior tier=100 backend=tsr_device warmup=2 frames=8 "
        "frame_ms_mean=11.376600 cv=2.2 fps=87.9"
    )
    if not m or m.group(1) != "11.376600":
        fails.append("BENCH_MS_RE 解析失败")
    m = DIFF_RE.search("G10_M137_DR: PASS regions=256 err_max=0.716157 err_p95=0.000108 over=0 heat=x")
    if not m or m.group(3) != "0":
        fails.append("DIFF_RE 解析失败")
    if not SCHEMA_PATH.is_file():
        fails.append(f"schema 缺失 {SCHEMA_PATH}")
    else:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        if set(schema.get("required", [])) != set(REQUIRED_KEYS):
            fails.append("schema required 与校验键集不等")
    if fails:
        for f in fails:
            print(f"[{TAG}] selftest FAIL: {f}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS (4 正则 GREEN + schema 互核)")
    return 0


def parse_wp(out_text: str, what: str) -> dict | None:
    m = WP_RE.search(out_text)
    if not m:
        check(False, f"{what} wp-hlod 行不可解析")
        return None
    return {
        "mode": m.group(1),
        "full": int(m.group(2)),
        "hlod": int(m.group(3)),
        "culled": int(m.group(4)),
        "pending": int(m.group(5)),
        "src": int(m.group(8)),
        "out": int(m.group(12)),
        "proxy": int(m.group(11)),
        "sel16": m.group(15),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=8)
    ap.add_argument("--warmup", type=int, default=2)
    ap.add_argument("--window-frames", type=int, default=32)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    # ① schema 在树 + required 闭集互核。
    check(SCHEMA_PATH.is_file(), f"schema 文件缺失: {SCHEMA_PATH}")
    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        check(
            set(schema.get("required", [])) == set(REQUIRED_KEYS),
            f"schema required 与校验键集不等: {set(schema.get('required', [])) ^ set(REQUIRED_KEYS)}",
        )

    # ② 构建必绿（生产车道 + 窗口臂 + 离线 bake + EXR diff 四件）。
    r = run_cmd([
        "cargo", "build", "--release",
        "-p", "rurix-render", "--features", "vendor-upscale",
        "--bin", "g14_3_pipeline_perf", "--bin", "g31_window_present",
        "--bin", "g10_m137_diff_report", "--quiet",
    ], timeout=7200)
    check(r.returncode == 0, f"rurix-render 构建失败: {(r.stdout + r.stderr)[-600:]}")
    r = run_cmd([
        "cargo", "build", "--release", "-p", "rurix-asset",
        "--bin", "g31_wp_hlod_bake", "--quiet",
    ], timeout=7200)
    check(r.returncode == 0, f"g31_wp_hlod_bake 构建失败: {(r.stdout + r.stderr)[-600:]}")
    rel = target_dir() / "release"
    perf = rel / f"g14_3_pipeline_perf{EXE_SUFFIX}"
    bake = rel / f"g31_wp_hlod_bake{EXE_SUFFIX}"
    window = rel / f"g31_window_present{EXE_SUFFIX}"
    diff = rel / f"g10_m137_diff_report{EXE_SUFFIX}"
    for p in (perf, bake, window, diff):
        check(p.is_file(), f"产物缺失: {p}")

    # ③ device 前置面。
    degrade_reasons: list[str] = []
    still_missing = ensure_spv()
    if still_missing:
        degrade_reasons.append(f"SPV 缺失且现编失败 {still_missing}")
    if not BISTRO_GLTF.is_file():
        degrade_reasons.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    ran = False
    ev: dict = {}
    if not FAILURES and not degrade_reasons:
        WORK_DIR.mkdir(parents=True, exist_ok=True)
        rxcs = WORK_DIR / "bistro.rxcs"
        rxwh = WORK_DIR / "bistro.rxwh"
        env = dict(os.environ)

        # ④ 资产链三步之 1/2（纯 host,不占 GPU 锁）。
        rd = run_cmd([str(perf), "--dump-scene", "--scene", "bistro-interior", "--out", str(rxcs)], timeout=1800)
        check(rd.returncode == 0, f"dump-scene 非零退出: {(rd.stdout + rd.stderr)[-400:]}")
        rb = run_cmd([
            str(bake), "--scene-dump", str(rxcs), "--out", str(rxwh),
            "--cell-size", CELL_SIZE, "--levels", LEVELS, "--double-build",
        ], timeout=3600)
        outb = rb.stdout + rb.stderr
        check(rb.returncode == 0, f"bake 非零退出: {outb[-400:]}")
        check("double-build 字节相等 OK" in outb, "bake double-build 确定性门缺证")
        mb = BAKE_RE.search(outb)
        check(mb is not None, f"bake OK 行不可解析: {outb[-400:]}")
        bake_info: dict = {}
        if mb:
            proxy_per_level = [int(x.strip()) for x in mb.group(12).split(",")]
            bake_info = {
                "cells_total": int(mb.group(6)),
                "cells_nonempty": int(mb.group(5)),
                "cell_size_m": float(mb.group(7)),
                "levels": int(mb.group(8)),
                "cell_tris_min": int(mb.group(9)),
                "cell_tris_max": int(mb.group(10)),
                "passthrough_tris": int(mb.group(11)),
                "proxy_tris_per_level": proxy_per_level,
                "pack_sha256": mb.group(13),
                "double_build_equal": "double-build 字节相等 OK" in outb,
                "bake_ms": float(mb.group(14)),
            }
            # QEM 链减半律（L(i+1) ≤ L(i);#67 质量烘焙链存在性）。
            for i in range(1, len(proxy_per_level)):
                check(
                    proxy_per_level[i] < proxy_per_level[i - 1],
                    f"代理层链未递减: L{i} {proxy_per_level[i]} ≥ L{i - 1} {proxy_per_level[i - 1]}",
                )

        # ⑤ device 真跑（持锁）。
        pack_args = ["--wp-pack", str(rxwh)]
        src_tris = 0
        full_anchor: dict = {}
        window_arms: list[dict] = []
        mixed_arm: dict = {}
        determinism: dict = {}
        switch_protocol: dict = {}
        red_doc: dict = {}
        quality: dict = {}
        frame_ms_doc: dict = {}
        with gpu_device_lock(purpose="g31 wave95 wp-hlod device 腿"):
            def bench(arm: str, extra: list[str], out_sub: str) -> tuple[subprocess.CompletedProcess, str, Path]:
                out_root = WORK_DIR / out_sub
                argv = [
                    str(perf), "--bench", "--scene", "bistro-interior", "--tier", "100",
                    "--backend", "tsr_device", "--frames", str(args.frames),
                    "--warmup", str(args.warmup), "--out-root", str(out_root),
                ] + extra
                rr = run_cmd(argv, timeout=3600, env=env)
                return rr, rr.stdout + rr.stderr, out_root

            r_off, out_off, root_off = bench("off", [], "off")
            if "SKIP DEV_ENV_DEGRADE" in out_off or "skipped_dev_env" in out_off:
                degrade_reasons.append(f"bench off dev_env 降级: {out_off.strip()[-300:]}")
            else:
                check(r_off.returncode == 0, f"bench off 非零退出: {out_off[-500:]}")
                # 全 Full 对拍锚（③）。
                r_full, out_full, root_full = bench("full", ["--wp-hlod", "full"] + pack_args, "full")
                check(r_full.returncode == 0, f"bench full 非零退出: {out_full[-500:]}")
                wf = parse_wp(out_full, "bench full")
                if wf:
                    src_tris = wf["src"]
                    check(wf["mode"] == "full" and wf["hlod"] == 0 and wf["out"] == wf["src"],
                          f"full 极限非全量: {wf}")
                d_off = bench_digest(root_off)
                d_full = bench_digest(root_full)
                check(d_off is not None and d_full is not None, "off/full receipt digest 缺失")
                if d_off and d_full:
                    check(d_off == d_full, f"全 Full 锚破坏: off={d_off} ≠ full={d_full}")
                    full_anchor = {"off_digest": d_off, "full_digest": d_full, "bitexact": d_off == d_full}
                m_off_ms = BENCH_MS_RE.search(out_off)

                # bench on 双跑（⑤ 确定性:GPU 端到端 digest + 选层序列前缀）。
                on_args = ["--wp-hlod", "on", "--wp-threshold-l0", str(T0_SWITCH),
                           "--wp-warmup", str(WARMUP_FRAMES)] + pack_args
                r_on_a, out_on_a, root_on_a = bench("on_a", on_args, "on_a")
                check(r_on_a.returncode == 0, f"bench on#1 非零退出: {out_on_a[-500:]}")
                r_on_b, out_on_b, root_on_b = bench("on_b", on_args, "on_b")
                check(r_on_b.returncode == 0, f"bench on#2 非零退出: {out_on_b[-500:]}")
                wa = parse_wp(out_on_a, "bench on#1")
                wb = parse_wp(out_on_b, "bench on#2")
                d_a = bench_digest(root_on_a)
                d_b = bench_digest(root_on_b)
                if wa and wb and d_a and d_b:
                    check(d_a == d_b, f"on 双跑 digest 漂移: {d_a} ≠ {d_b}")
                    check(wa["sel16"] == wb["sel16"], f"选层序列前缀漂移: {wa['sel16']} ≠ {wb['sel16']}")
                    determinism = {
                        "arm_threshold_l0": T0_SWITCH,
                        "double_run_digest_equal": d_a == d_b,
                        "selection_digest_prefix_equal": wa["sel16"] == wb["sel16"],
                    }
                m_on_ms = BENCH_MS_RE.search(out_on_a)
                if m_off_ms and m_on_ms:
                    frame_ms_doc = {
                        "off_mean": float(m_off_ms.group(1)),
                        "on_mean": float(m_on_ms.group(1)),
                        "note": f"bistro-interior tier100 tsr_device --frames {args.frames} 同机同窗 measured_local(装配相机冻结出帧;如实登记不设通过线,G6 纪律)",
                    }
                else:
                    check(False, "bench frame_ms_mean 行不可解析")

                # 三档窗口臂（④ 单调 + 混合;⑥ 切换协议在 t0=4.0 臂）。
                def window_arm(t0: float, sub: str) -> tuple[dict | None, dict | None]:
                    stats_path = WORK_DIR / f"wp_stats_{sub}.json"
                    rw = run_cmd([
                        str(window), "--frames", str(args.window_frames), "--warmup", "2",
                        "--tier", "100", "--headless-smoke", "--auto-move", "dolly",
                        "--wp-hlod", "on", "--wp-threshold-l0", str(t0),
                        "--wp-warmup", str(WARMUP_FRAMES),
                    ] + pack_args + [
                        "--wp-stats-out", str(stats_path),
                        "--evidence", str(WORK_DIR / f"wp_window_ev_{sub}.json"),
                    ], timeout=3600, env=env)
                    outw = rw.stdout + rw.stderr
                    check(rw.returncode == 0, f"窗口臂 t0={t0} 非零退出: {outw[-500:]}")
                    w = parse_wp(outw, f"窗口 t0={t0}")
                    sj = None
                    if stats_path.is_file():
                        sj = json.loads(stats_path.read_text(encoding="utf-8"))
                    else:
                        check(False, f"窗口统计 sidecar 未落盘: {stats_path}")
                    return w, sj

                w_mixed, sj_mixed = window_arm(T0_MIXED, "mixed")
                w_switch, sj_switch = window_arm(T0_SWITCH, "switch")
                w_aggr, sj_aggr = window_arm(T0_AGGRESSIVE, "aggr")
                arms_raw = [(T0_MIXED, w_mixed, sj_mixed), (T0_SWITCH, w_switch, sj_switch),
                            (T0_AGGRESSIVE, w_aggr, sj_aggr)]
                for t0, w, sj in arms_raw:
                    if w is None or sj is None:
                        continue
                    sel = (sj.get("assembled") or {}).get("selection_digest", "")
                    check(re.fullmatch(r"[0-9a-f]{64}", sel) is not None,
                          f"t0={t0} sidecar selection_digest 形态非法: {sel[:32]!r}")
                    check(w["out"] < w["src"], f"t0={t0} 臂三角未下降: {w['out']} ≥ {w['src']}")
                    check(w["pending"] == 0, f"t0={t0} 稳态后仍有流送未达 cell: {w['pending']}")
                    window_arms.append({
                        "threshold_l0": t0,
                        "cells_full": w["full"],
                        "cells_hlod": w["hlod"],
                        "cells_culled": w["culled"],
                        "cells_pending": w["pending"],
                        "out_tris": w["out"],
                        "ratio": w["out"] / max(1, w["src"]),
                        "proxy_tris": w["proxy"],
                        "selection_digest": sel,
                    })
                if len(window_arms) == 3:
                    check(
                        window_arms[0]["out_tris"] > window_arms[1]["out_tris"] > window_arms[2]["out_tris"],
                        f"阈值单调破坏: {[a['out_tris'] for a in window_arms]}（t0 升 ⇒ 更激进代理 ⇒ out 应严格降）",
                    )
                # 混合臂（④:同帧 Full XOR 代理并存 = 互斥机核活性面）。
                if w_mixed:
                    check(w_mixed["full"] >= 1 and w_mixed["hlod"] >= 1,
                          f"混合臂非混合: full={w_mixed['full']} hlod={w_mixed['hlod']}")
                    mixed_arm = {
                        "threshold_l0": T0_MIXED,
                        "cells_full": w_mixed["full"],
                        "cells_hlod": w_mixed["hlod"],
                        "mutually_exclusive_mixed": w_mixed["full"] >= 1 and w_mixed["hlod"] >= 1,
                    }
                # 切换协议臂（⑥:switches ≥ 1 + warmup 协议 + popping 指标）。
                if sj_switch:
                    pop = sj_switch.get("popping") or {}
                    events = sj_switch.get("switch_events") or []
                    frames_arr = sj_switch.get("frames") or []
                    check(int(pop.get("total_switches", 0)) >= 1,
                          f"dolly 轨迹零切换（切换协议无活性证据）: {pop}")
                    check(pop.get("warmup_protocol_verified") is True, "warmup 协议 sidecar 未验证")
                    wf_n = int(sj_switch.get("warmup_frames", -1))
                    check(wf_n == WARMUP_FRAMES, f"sidecar warmup_frames {wf_n} ≠ {WARMUP_FRAMES}")
                    for e in events:
                        check(
                            int(e["flip_frame"]) - int(e["request_frame"]) == WARMUP_FRAMES,
                            f"切换事件间隔 ≠ warmup: {e}",
                        )
                    switch_protocol = {
                        "threshold_l0": T0_SWITCH,
                        "warmup_frames": WARMUP_FRAMES,
                        "frames": len(frames_arr),
                        "total_switches": int(pop.get("total_switches", 0)),
                        "max_switches_per_frame": int(pop.get("max_switches_per_frame", 0)),
                        "switch_delta_tris_max": int(pop.get("switch_delta_tris_max", 0)),
                        "warmup_protocol_verified": pop.get("warmup_protocol_verified") is True,
                        "events": [
                            {
                                "cell": int(e["cell"]),
                                "from": str(e["from"]),
                                "to": str(e["to"]),
                                "request_frame": int(e["request_frame"]),
                                "flip_frame": int(e["flip_frame"]),
                                "tris_before": int(e["tris_before"]),
                                "tris_after": int(e["tris_after"]),
                            }
                            for e in events
                        ],
                    }

                # ⑦ 四 RED 臂子进程独立检出（host 机核,无 GPU 依赖但保持锁内
                # 串行——窗口 bin 启动面统一口径）。
                red_rcs: dict[str, int] = {}
                for arm in RED_ARMS:
                    ra = run_cmd([
                        str(window), "--frames", "2", "--tier", "100", "--headless-smoke",
                        "--wp-hlod", "on", "--wp-red-arm", arm,
                    ] + pack_args, timeout=1800, env=env)
                    red_rcs[arm] = ra.returncode
                    outa = (ra.stdout or "") + (ra.stderr or "")
                    check(
                        ra.returncode == 0 and "WP_RED_ARM_DETECTED" in outa,
                        f"RED 臂 {arm} 未检出 rc={ra.returncode}: {outa[-200:]}",
                    )
                red_doc = {
                    "tamper_digest": red_rcs.get("tamper-digest") == 0,
                    "event_order": red_rcs.get("event-order") == 0,
                    "double_draw": red_rcs.get("double-draw") == 0,
                    "runtime_merge": red_rcs.get("runtime-merge") == 0,
                }

                # ⑧ 画质差 measured（render off vs on 收敛帧 diff,不设通过线）。
                def render(arm: list[str], sub: str) -> Path | None:
                    out_root = WORK_DIR / sub
                    rr = run_cmd([
                        str(perf), "--render", "--scene", "bistro-interior", "--tier", "100",
                        "--backend", "tsr_device", "--frames", "4", "--out-root", str(out_root),
                    ] + arm, timeout=3600, env=env)
                    check(rr.returncode == 0, f"render {sub} 非零退出: {(rr.stdout + rr.stderr)[-400:]}")
                    p = out_root / "bistro-interior" / "tier100" / "tsr_device" / "converged.exr"
                    return p if p.is_file() else None

                exr_off = render([], "render_off")
                exr_on = render(["--wp-hlod", "on", "--wp-threshold-l0", str(T0_SWITCH),
                                 "--wp-warmup", str(WARMUP_FRAMES)] + pack_args, "render_on")
                if exr_off and exr_on:
                    rq = run_cmd([
                        str(diff), "--frame-a", str(exr_off), "--frame-b", str(exr_on),
                        "--out-dir", str(WORK_DIR / "diff"), "--evidence", str(WORK_DIR / "diff" / "report.json"),
                        "--scene-id", "bistro-interior", "--camera-id", "contract",
                        "--frame-index", "0", "--threshold", "1.0",
                    ], timeout=1800)
                    outq = rq.stdout + rq.stderr
                    check(rq.returncode == 0, f"EXR diff 非零退出: {outq[-300:]}")
                    mq = DIFF_RE.search(outq)
                    check(mq is not None, f"diff 行不可解析: {outq[-300:]}")
                    if mq:
                        quality = {
                            "threshold_l0": T0_SWITCH,
                            "err_max": float(mq.group(1)),
                            "err_p95": float(mq.group(2)),
                            "regions_over_1": int(mq.group(3)),
                            "note": "off vs on(t0=4.0) 收敛帧 HDR 线性域区域 diff（16×16 网格,阈 1.0）;远 cell 出 QEM 代理层的画质差 measured 如实登记不设通过线（G6 纪律）",
                        }
                else:
                    check(False, "render 收敛帧缺失（diff 面不完整）")

                if not FAILURES:
                    ran = True
                    ev = {
                        "schema": SCHEMA_ID,
                        "gate": GATE_KEY,
                        "scene": "bistro-interior",
                        "tier": 100,
                        "backend": "tsr_device",
                        "src_tris": src_tris,
                        "bake": bake_info,
                        "full_anchor": full_anchor,
                        "window_arms": window_arms,
                        "mixed_arm": mixed_arm,
                        "determinism": determinism,
                        "switch_protocol": switch_protocol,
                        "red_arms": red_doc,
                        "quality_diff": quality,
                        "frame_ms": frame_ms_doc,
                        "commands": COMMANDS,
                        "notes": [
                            "三步文件交接：dump-scene（装配语义单源）→ g31_wp_hlod_bake（XZ cell 网格 + 逐 cell 跨组件合并 + bake_hlod_merged QEM 链事实源直调,#67/#97）→ 生产车道 --wp-hlod 消费（world::partition PartitionRuntime 距离环流送 + 三项预算排队 + world::hlod HlodRuntime 事件消费/digest 核验/screen-size 互斥选层生产机核直调,#95）",
                            "#68 HLOD 代理 GPU 绘制腿 = 代理三角随互斥重建进 BLAS 出帧;同 cell 全量 XOR 代理（互斥机核 + 源三角零重复断言 = 零双绘,bin 内嵌 fail-closed）;切换 = warmup 预热 N 帧后同帧原子翻转（UE bRequireWarmup 模式,flip−request==warmup 逐事件机核）",
                            "#99 popping 指标 = 固定 dolly 轨迹切换事件表 + 逐帧翻转数/三角跳变进 evidence（measured 如实登记）;geomorph/dither 过渡评估留窗",
                            "emissive 三角与 quad 灯面尾段恒 passthrough（光源几何面 0-byte）;代理三角属性 = cell 面积加权均值",
                            "默认 off = 既有面 0-byte（Stage A 锚零漂移;--wp-hlod 为加性闭集开关,与 --cluster-lod/--hzb/--textures/--slab-table 闭集互斥）",
                            "出帧几何冻结于装配期选层（逐帧 AS 更新归 #77/#89 合流窗）;g31 窗口逐帧 tick/选层/切换统计为 host 重算 measured 如实登记不冒充",
                        ],
                    }

    for m in NOTES:
        print(f"[{TAG}] NOTE {m}")
    if degrade_reasons:
        for d in degrade_reasons:
            print(f"[{TAG}] DEV_ENV_DEGRADE {d}")
        if require_real():
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        print(f"[{TAG}] SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL;构建/schema 面仍真跑）")
        return 0
    if FAILURES:
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    if not ran:
        print(f"[{TAG}] FAIL: device 腿未真跑（无 degrade 原因但无真跑证据）", file=sys.stderr)
        return 1
    # schema 自校验硬门（PASS-only 闭集面）。
    import jsonschema

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(ev))
    if errs:
        for e in errs[:5]:
            print(f"[{TAG}] FAIL evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}",
                  file=sys.stderr)
        return 1
    # PASS-only evidence 落盘。
    ts = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
    ev_path = ROOT / "evidence" / f"g31_wp_hlod_{ts}.json"
    ev_path.parent.mkdir(parents=True, exist_ok=True)
    ev_path.write_text(json.dumps(ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[{TAG}] PASS gate={GATE_KEY}（构建绿 + bake double-build 确定 + 全 Full digest 锚位级 "
        f"+ 三档阈值单调下降 + 混合互斥帧 + on 双跑位级 + dolly 切换 warmup 协议 + 四 RED 臂 + 画质差 measured）"
        f" evidence={ev_path}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
