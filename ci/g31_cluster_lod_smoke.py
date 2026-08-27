#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G31+ #58 簇 DAG LOD 生产接线门（gate g31.wave58.cluster_lod）。

判据闭集（G31_PLUS_COMMERCIAL_RENDERER_TODO §7 #58 行交付判据方向）：
① schema 在树 + required 闭集互核 + 构建必绿；
② 资产链三步真跑：`g14_3_pipeline_perf --dump-scene`（RXCS 装配 dump）→
   `g31_cluster_lod_bake --double-build`（RXCP 簇包,双 bake 字节相等 =
   确定性门）→ 生产车道消费；
③ **全叶对拍锚**：`--cluster-lod leaf` 末帧 digest 与 `off`（既有三角汤）
   **位级一致**（bin 内嵌逐三角位级断言 + 端到端 GPU digest 双证）；
④ 误差 cut 出帧：1px/4px 两档 out_tris < src_tris 且阈值单调（4px ≤ 1px）,
   cut 覆盖性机核（组共享 LOD 判定球）由 bin 内嵌 fail-closed；
⑤ 确定性：on 4px 双跑末帧 digest 位级一致；
⑥ 相机驱动 cut：g31 窗口 headless dolly 轨迹逐帧 host cut 统计
   cut_tris min < max（静态 cut 无法经本面产生）；
⑦ 画质差 measured 如实登记（off vs on 4px 收敛帧 EXR 区域 diff,
   err_p95/err_max/超阈区域数;G6 纪律不设通过线）。

三态：无 Vulkan/SPV/bistro 资产 → SKIP DEV_ENV_DEGRADE 退 0（非 fake pass;
RURIX_REQUIRE_REAL=1 翻硬 FAIL）。PASS-only evidence：过门才落
evidence/g31_cluster_lod_<ts>.json,FAIL 诊断落 .tmp 不污染。

用法：python ci/g31_cluster_lod_smoke.py [--gate g31.wave58.cluster_lod]
      [--frames 8] [--warmup 2] [--selftest]
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
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_cluster_lod_evidence_schema.json"
WORK_DIR = ROOT / ".tmp" / "g31_gates" / "wave58_cluster_lod"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.wave58.cluster_lod"
TAG = "g31_cluster_lod"
SCHEMA_ID = "rurix.g31.cluster_lod_evidence.v1"
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
    "leaf_anchor",
    "cut_arms",
    "determinism",
    "window_stats",
    "quality_diff",
    "commands",
    "notes",
]

BAKE_RE = re.compile(
    r"bake OK blocks=(\d+) \(degraded=(\d+)\) clusters=(\d+) levels_max=(\d+) "
    r"leaf_tris=(\d+) root_tris=(\d+) passthrough=(\d+) pages=\d+ bytes=\d+ sha256=([0-9a-f]{64}) "
    r"bake_ms=([0-9.]+)"
)
CUT_RE = re.compile(
    r"cluster-lod mode=(\w+) threshold_px=([0-9.]+) blocks=(\d+) clusters=(\d+)/(\d+) "
    r"\(leaf_cut=(\d+)\) tris: src=(\d+) passthrough=(\d+) leaf_pool=(\d+) coarse=(\d+) out=(\d+)"
)
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
    # 沙盒/自定义 CARGO_TARGET_DIR 兼容（未设 = 仓内 target 既有惯例）。
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
        "bake OK blocks=187 (degraded=0) clusters=129709 levels_max=17 leaf_tris=1002585 "
        "root_tris=17208 passthrough=44024 pages=1712 bytes=47777508 sha256=" + "a" * 64
        + " bake_ms=814.1 simplifier=qem group_size=8 sibling_bias=true qem_stuck_groups=50117 -> x"
    )
    if not m or m.group(1) != "187" or m.group(9) != "814.1":
        fails.append("BAKE_RE 解析失败（E 页分配 pages= 字段同步面）")
    m = CUT_RE.search(
        "cluster-lod mode=on threshold_px=4 blocks=187 clusters=19294/129709 (leaf_cut=4357) "
        "tris: src=1046609 passthrough=44024 leaf_pool=1002585 coarse=214357 out=572727 (54.7%)"
    )
    if not m or m.group(11) != "572727":
        fails.append("CUT_RE 解析失败")
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
    print(f"[{TAG}] selftest PASS (3 正则 GREEN + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=8)
    ap.add_argument("--warmup", type=int, default=2)
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
        "--bin", "g31_cluster_lod_bake", "--quiet",
    ], timeout=7200)
    check(r.returncode == 0, f"g31_cluster_lod_bake 构建失败: {(r.stdout + r.stderr)[-600:]}")
    rel = target_dir() / "release"
    perf = rel / f"g14_3_pipeline_perf{EXE_SUFFIX}"
    bake = rel / f"g31_cluster_lod_bake{EXE_SUFFIX}"
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
        rxcp = WORK_DIR / "bistro.rxcp"
        env = dict(os.environ)

        # ④ 资产链三步之 1/2（纯 host,不占 GPU 锁）。
        rd = run_cmd([str(perf), "--dump-scene", "--scene", "bistro-interior", "--out", str(rxcs)], timeout=1800)
        check(rd.returncode == 0, f"dump-scene 非零退出: {(rd.stdout + rd.stderr)[-400:]}")
        rb = run_cmd([str(bake), "--scene-dump", str(rxcs), "--out", str(rxcp), "--double-build"], timeout=3600)
        outb = rb.stdout + rb.stderr
        check(rb.returncode == 0, f"bake 非零退出: {outb[-400:]}")
        check("double-build 字节相等 OK" in outb, "bake double-build 确定性门缺证")
        mb = BAKE_RE.search(outb)
        check(mb is not None, f"bake OK 行不可解析: {outb[-400:]}")
        bake_info: dict = {}
        if mb:
            bake_info = {
                "blocks": int(mb.group(1)),
                "degraded_blocks": int(mb.group(2)),
                "clusters": int(mb.group(3)),
                "levels_max": int(mb.group(4)),
                "leaf_tris": int(mb.group(5)),
                "root_tris": int(mb.group(6)),
                "passthrough_tris": int(mb.group(7)),
                "pack_sha256": mb.group(8),
                "double_build_equal": "double-build 字节相等 OK" in outb,
                "bake_ms": float(mb.group(9)),
            }
            check(bake_info["degraded_blocks"] == 0, f"降级块 {bake_info['degraded_blocks']} ≠ 0（bistro 全块须构建成功）")
            check(bake_info["root_tris"] < bake_info["leaf_tris"], "根层三角未少于叶层（DAG 无简化收益）")

        # ⑤ device 真跑（持锁）：off/leaf 锚 + 1px/4px 双档 + 4px 双跑 + 窗口统计。
        def bench(arm: str, extra: list[str], out_sub: str) -> tuple[subprocess.CompletedProcess, str, Path]:
            out_root = WORK_DIR / out_sub
            argv = [
                str(perf), "--bench", "--scene", "bistro-interior", "--tier", "100",
                "--backend", "tsr_device", "--frames", str(args.frames),
                "--warmup", str(args.warmup), "--out-root", str(out_root),
            ] + extra
            rr = run_cmd(argv, timeout=3600, env=env)
            return rr, rr.stdout + rr.stderr, out_root

        pack_args = ["--cluster-pack", str(rxcp)]
        src_tris = 0
        cut_arms: list[dict] = []
        leaf_anchor: dict = {}
        determinism: dict = {}
        window_stats: dict = {}
        quality: dict = {}
        with gpu_device_lock(purpose="g31 wave58 cluster-lod device 腿"):
            r_off, out_off, root_off = bench("off", [], "off")
            if "SKIP DEV_ENV_DEGRADE" in out_off or "skipped_dev_env" in out_off:
                degrade_reasons.append(f"bench off dev_env 降级: {out_off.strip()[-300:]}")
            else:
                check(r_off.returncode == 0, f"bench off 非零退出: {out_off[-500:]}")
                r_leaf, out_leaf, root_leaf = bench("leaf", ["--cluster-lod", "leaf"] + pack_args, "leaf")
                check(r_leaf.returncode == 0, f"bench leaf 非零退出: {out_leaf[-500:]}")
                r_on1, out_on1, root_on1 = bench("on1", ["--cluster-lod", "on", "--cluster-error-px", "1.0"] + pack_args, "on1")
                check(r_on1.returncode == 0, f"bench on(1px) 非零退出: {out_on1[-500:]}")
                r_on4a, out_on4a, root_on4a = bench("on4a", ["--cluster-lod", "on", "--cluster-error-px", "4.0"] + pack_args, "on4a")
                check(r_on4a.returncode == 0, f"bench on(4px)#1 非零退出: {out_on4a[-500:]}")
                r_on4b, out_on4b, root_on4b = bench("on4b", ["--cluster-lod", "on", "--cluster-error-px", "4.0"] + pack_args, "on4b")
                check(r_on4b.returncode == 0, f"bench on(4px)#2 非零退出: {out_on4b[-500:]}")

                # 全叶对拍锚（③）。
                d_off = bench_digest(root_off)
                d_leaf = bench_digest(root_leaf)
                check(d_off is not None and d_leaf is not None, "off/leaf receipt digest 缺失")
                if d_off and d_leaf:
                    check(d_off == d_leaf, f"全叶锚破坏: off={d_off} ≠ leaf={d_leaf}")
                    leaf_anchor = {"off_digest": d_off, "leaf_digest": d_leaf, "bitexact": d_off == d_leaf}

                # cut 双档（④）+ 确定性（⑤）。
                def cut_arm(out_text: str, root: Path, threshold: float) -> dict | None:
                    m = CUT_RE.search(out_text)
                    if not m:
                        check(False, f"cut 行不可解析（threshold {threshold}）")
                        return None
                    nonlocal src_tris
                    src_tris = int(m.group(7))
                    fm = re.search(r"frame_ms_mean=([0-9.]+)", out_text)
                    sm = re.search(r"scene_gpu_ms_mean=([0-9.]+)", out_text)
                    d = bench_digest(root)
                    if d is None:
                        check(False, f"receipt digest 缺失（threshold {threshold}）")
                        return None
                    return {
                        "threshold_px": threshold,
                        "out_tris": int(m.group(11)),
                        "ratio": int(m.group(11)) / max(1, int(m.group(7))),
                        "cut_clusters": int(m.group(4)),
                        "digest": d,
                        "frame_ms_mean": float(fm.group(1)) if fm else 0.0,
                        "scene_gpu_ms_mean": float(sm.group(1)) if sm else 0.0,
                    }

                a1 = cut_arm(out_on1, root_on1, 1.0)
                a4 = cut_arm(out_on4a, root_on4a, 4.0)
                if a1 and a4:
                    cut_arms = [a1, a4]
                    check(a1["out_tris"] < src_tris, f"1px 臂三角未下降: {a1['out_tris']} ≥ {src_tris}")
                    check(a4["out_tris"] < a1["out_tris"], f"阈值单调破坏: 4px {a4['out_tris']} ≥ 1px {a1['out_tris']}")
                d4b = bench_digest(root_on4b)
                if a4 and d4b:
                    check(a4["digest"] == d4b, f"on 4px 双跑 digest 漂移: {a4['digest']} ≠ {d4b}")
                    determinism = {"arm_threshold_px": 4.0, "double_run_digest_equal": a4["digest"] == d4b}

                # 窗口逐帧统计（⑥;headless dolly——出帧几何冻结,统计如实登记）。
                stats_path = WORK_DIR / "window_stats.json"
                rw = run_cmd([
                    str(window), "--frames", "24", "--warmup", "2", "--tier", "100",
                    "--headless-smoke", "--auto-move", "dolly",
                    "--cluster-lod", "on", "--cluster-error-px", "2.0",
                ] + pack_args + [
                    "--cluster-stats-out", str(stats_path),
                    "--evidence", str(WORK_DIR / "window_ev.json"),
                ], timeout=3600, env=env)
                outw = rw.stdout + rw.stderr
                check(rw.returncode == 0, f"窗口臂非零退出: {outw[-500:]}")
                if stats_path.is_file():
                    sj = json.loads(stats_path.read_text(encoding="utf-8"))
                    tris_seq = [f["cut_tris"] for f in sj.get("frames", [])]
                    check(len(tris_seq) >= 2, "窗口逐帧统计帧数 < 2")
                    if len(tris_seq) >= 2:
                        varies = min(tris_seq) < max(tris_seq)
                        check(varies, "dolly 轨迹下 cut_tris 恒定（相机未驱动 cut,静态 LOD 嫌疑）")
                        window_stats = {
                            "trajectory": "dolly",
                            "frames": len(tris_seq),
                            "cut_tris_min": min(tris_seq),
                            "cut_tris_max": max(tris_seq),
                            "camera_drives_cut": varies,
                            "note": "逐帧 host cut 重算 measured（每 16 帧覆盖性机核采样）;出帧几何冻结于装配期 cut,逐帧 AS 更新归 C/E 阶段——如实登记不冒充",
                        }
                else:
                    check(False, f"窗口统计 sidecar 未落盘: {stats_path}")

                # 画质差 measured（⑦;render 收敛帧 off vs on4 diff,不设通过线）。
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
                exr_on4 = render(["--cluster-lod", "on", "--cluster-error-px", "4.0"] + pack_args, "render_on4")
                if exr_off and exr_on4:
                    rq = run_cmd([
                        str(diff), "--frame-a", str(exr_off), "--frame-b", str(exr_on4),
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
                            "threshold_px": 4.0,
                            "err_max": float(mq.group(1)),
                            "err_p95": float(mq.group(2)),
                            "regions_over_1": int(mq.group(3)),
                            "note": "off vs on(4px) 收敛帧 HDR 线性域区域 diff（16×16 网格,阈 1.0）;measured 如实登记不设通过线（G6 纪律）",
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
                        "leaf_anchor": leaf_anchor,
                        "cut_arms": cut_arms,
                        "determinism": determinism,
                        "window_stats": window_stats,
                        "quality_diff": quality,
                        "commands": COMMANDS,
                        "notes": [
                            "三步文件交接：dump-scene（装配语义单源）→ g31_cluster_lod_bake（build_asset_dag 事实源构建 + 组共享 LOD 判定球派生）→ 生产车道 --cluster-lod 消费（select_lod_cut_grouped + verify_cut_coverage 生产金标准直调）",
                            "emissive 三角与 quad 灯面尾段恒 passthrough（光源几何面 0-byte）;粗簇属性 = 叶后代面积加权均值",
                            "默认 off = 既有面 0-byte（Stage A 锚零漂移;--cluster-lod 为加性闭集开关）",
                            "本门 = TODO #58 生产消费簇 DAG 最小面;逐帧 device cut/AS 更新归 #77（C 阶段）,页流送归 #20-23（E 阶段）",
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
    # PASS-only evidence 落盘。
    ts = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
    ev_path = ROOT / "evidence" / f"g31_cluster_lod_{ts}.json"
    ev_path.parent.mkdir(parents=True, exist_ok=True)
    ev_path.write_text(json.dumps(ev, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[{TAG}] PASS gate={GATE_KEY}（构建绿 + bake double-build 确定 + 全叶 digest 锚位级 "
        f"+ 1px/4px cut 单调下降 + 4px 双跑位级 + dolly 相机驱动 cut + 画质差 measured）"
        f" evidence={ev_path}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
