#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.2 修订与测量波）
"""G14.2 harness — UE benchmark 臂正式帧率测量执行器（M-b；host 编排面）。

命令形态（G14.2 探针实证链 .tmp 探针 1~6 留痕——臂 B 命令面闭集 RXS-0380 L2 +
CsvProfile 双臂逗号分隔形态〔HandleCSVProfileCommand 每调用只处理 Args[0] 源码实证〕+
benchmark 虚拟步进 + Windows 原始命令行字符串传参〔list2cmdline 重引号陷阱实证〕）：

  UnrealEditor-Cmd.exe <proj> <map> -game -benchmark -seconds=<虚拟秒>
      -ResX=<w> -ResY=<h> -windowed -csvGpuStats
      -execcmds="CsvProfile startfile=<tag>, CsvProfile frames=<N>, <tier cvar 链>"
      -unattended -log -notexturestreaming -FixedSeed

收割 Saved/Profiling/CSV/<tag>.csv——逐帧 FrameTime/GameThreadTime/RenderThreadTime/
GPUTime/RHIThreadTime/MaxFrameTime + GPU 逐 pass 分解列（GPU/Streamline 非零 =
DLSS engagement 机核面之一）+ 元数据尾（engineversion == M128 登记面）。
稳态窗 = 弃 warmup 后末 150 帧（3 块 × 50，M141/M165 冻结统计口径复用
ci/g10_perf_baseline_smoke.py block_stats/recompute_check 单源）。

用法：
  py -3 milestones/g14/harness/g14_2_ue_bench.py <scene_id> --tier <50|67|100> --run-index <1..3>
  py -3 milestones/g14/harness/g14_2_ue_bench.py build [scene_id|--all]   # 契约相机 auto-activation 对齐步
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

UE_EXE = r"F:\UE_5.8\Engine\Binaries\Win64\UnrealEditor-Cmd.exe"
UPROJECT = r"K:\rurix-ext\g10-ue\G10RefRender\G10RefRender.uproject"
CSV_DIR = Path(r"K:\rurix-ext\g10-ue\G10RefRender\Saved\Profiling\CSV")
UE_LOG = Path(r"K:\rurix-ext\g10-ue\G10RefRender\Saved\Logs\G10RefRender.log")
OUT_ROOT = Path(r"K:\rurix-ext\g14-frames\ue_bench")
CAMERA_ALIGN_SCRIPT = ROOT / "milestones" / "g14" / "harness" / "ue_python" / "g14_2_bench_camera_align.py"

# 双场景闭集（M133 清单 digest 注册面；G13 对拍面继承——分辨率同 G13 对拍口径）。
SCENES = {
    "cornell-box": {"map": "/Game/Maps/G13_CornellBox", "w": 512, "h": 512},
    "bistro-interior": {"map": "/Game/Maps/G13_BistroInterior", "w": 1920, "h": 1080},
}
TIERS = (50, 67, 100)
# 档→DLSS 名义映射注入链（G13.4 M-c 契约 tier↔Performance/Quality/DLAA 名义档对拍口径
# 继承；-game 臂经 cvar 注入，engagement 以日志 NGX 窗口实测读回登记）。
TIER_CVARS = {
    50: "r.ScreenPercentage 50, r.NGX.DLSS.Enable 1",
    67: "r.ScreenPercentage 67, r.NGX.DLSS.Enable 1",
    100: "r.ScreenPercentage 100, r.NGX.DLSS.Enable 1",
}
# benchmark 虚拟步进：-seconds 计虚拟秒（固定 60Hz 步进）；frames 480 = 8 虚拟秒采集
# + seconds=40 留 processing thread 落盘窗（探针 2/3 早退/未落盘教训留痕）。
CAPTURE_FRAMES = 480
VIRTUAL_SECONDS = 40
WARMUP_DROP_HEAD = 300  # 稳态窗 = 480 帧弃首 300（启动/PSO 编译面）取末 180→150 协议窗
TIMED = 150


def build_camera_align(scene_id: str) -> bool:
    """契约相机 auto-activation 对齐（幂等；编辑器 Python 臂，子进程自持锁）。"""
    probe_out = OUT_ROOT / scene_id / "camera_align_probe.json"
    probe_out.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env["G14_2_SCENE"] = scene_id
    env["G14_2_PROBE_OUT"] = str(probe_out).replace("\\", "/")
    argv = [UE_EXE, UPROJECT, f"-ExecutePythonScript={CAMERA_ALIGN_SCRIPT}",
            "-unattended", "-log", "-nopause"]
    with gpu_device_lock(purpose=f"g14.2 契约相机对齐 {scene_id}"):
        r = subprocess.run(argv, capture_output=True, timeout=5400, env=env)
    ok = r.returncode == 0 and probe_out.is_file()
    if ok:
        doc = json.loads(probe_out.read_text(encoding="utf-8"))
        ok = doc.get("aligned") is True
    print(f"[g14_2_ue_bench] camera-align {scene_id} exit={r.returncode} aligned={ok}")
    return ok


def run_bench(scene_id: str, tier: int, run_index: int, timeout: int = 1800) -> dict:
    s = SCENES[scene_id]
    tag = f"g14_bench_{scene_id}_t{tier}_r{run_index}"
    cmdline = (
        f'"{UE_EXE}" {UPROJECT} {s["map"]} -game -benchmark -seconds={VIRTUAL_SECONDS} '
        f'-ResX={s["w"]} -ResY={s["h"]} -windowed -csvGpuStats '
        f'-execcmds="CsvProfile startfile={tag}, CsvProfile frames={CAPTURE_FRAMES}, {TIER_CVARS[tier]}" '
        f'-unattended -log -notexturestreaming -FixedSeed'
    )
    t0 = time.time()
    with gpu_device_lock(purpose=f"g14.2 UE benchmark 臂 {scene_id}/t{tier}/r{run_index}"):
        r = subprocess.run(cmdline, capture_output=True, timeout=timeout)
    dur = time.time() - t0
    csv_path = CSV_DIR / f"{tag}.csv"
    log_text = UE_LOG.read_text(encoding="utf-8", errors="replace") if UE_LOG.is_file() else ""
    log_fresh = UE_LOG.is_file() and UE_LOG.stat().st_mtime >= t0 - 5.0
    rec = {
        "scene_id": scene_id,
        "tier": tier,
        "run_index": run_index,
        "capture_arm": f"B-benchmark-csvprofile（RXS-0380 L2 臂 B + CsvProfile 双臂逗号链）",
        "command_digest": "",
        "started_epoch": t0,
        "duration_s": dur,
        "exit_code": r.returncode,
        "csv_file": str(csv_path),
        "csv_present": csv_path.is_file(),
        "csv_bytes": csv_path.stat().st_size if csv_path.is_file() else 0,
        "ue_log_fresh": bool(log_fresh),
        "dlss_log_tokens": [l for l in log_text.splitlines() if "NGX" in l or "DLSS" in l][-20:],
    }
    import hashlib
    # 命令面 digest 归一化：输出 tag 随轮次变化，digest 面以 <TAG> 归一占位（三轮
    # 进程级独立性机核消费——命令模板面一致，逐轮仅输出文件签名不同）。
    rec["command_digest"] = "sha256:" + hashlib.sha256(
        cmdline.replace(tag, "<TAG>").encode("utf-8")).hexdigest()
    out_dir = OUT_ROOT / scene_id / f"tier{tier}"
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / f"bench_receipt_r{run_index}.json").write_text(
        json.dumps(rec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[g14_2_ue_bench] {scene_id}/t{tier}/r{run_index} exit={r.returncode} "
          f"csv={'OK' if rec['csv_present'] else 'MISSING'} dur={dur:.1f}s")
    return rec


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: g14_2_ue_bench.py <cornell-box|bistro-interior> --tier N --run-index K\n"
              "       g14_2_ue_bench.py build [scene_id|--all]", file=sys.stderr)
        return 2
    if sys.argv[1] == "build":
        target = sys.argv[2] if len(sys.argv) > 2 else "--all"
        scenes = list(SCENES) if target == "--all" else [target]
        ok = all(build_camera_align(s) for s in scenes)
        return 0 if ok else 1
    scene_id = sys.argv[1]
    if scene_id not in SCENES:
        print(f"unknown scene {scene_id}", file=sys.stderr)
        return 2
    tier = int(sys.argv[sys.argv.index("--tier") + 1]) if "--tier" in sys.argv else 67
    run_index = int(sys.argv[sys.argv.index("--run-index") + 1]) if "--run-index" in sys.argv else 1
    if tier not in TIERS:
        print(f"tier {tier} 越闭集 {TIERS}", file=sys.stderr)
        return 2
    rec = run_bench(scene_id, tier, run_index)
    return 0 if rec["exit_code"] == 0 and rec["csv_present"] else 1


if __name__ == "__main__":
    sys.exit(main())
