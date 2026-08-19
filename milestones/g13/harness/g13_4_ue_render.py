#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.4 UE 对拍波）
"""G13.4 harness — UE 超分对拍（M-c）/ Lumen GI 对照（M-d）UE 臂 host 编排器
（RXS-0405/RXS-0406 L3 双端出图 UE 腿；Phase A build + Phase B render 形态 +
gpu_device_lock 串行沿 G12.4 实证）。

职责闭集：
  1. Phase A（建设）：逐场景 UnrealEditor-Cmd -ExecutePythonScript=
     g13_4_build_scenes.py（地图/MRQ 资产配置幂等重建 + 探针 JSON 收割；
     环境变量 G13_4_SCENE/G13_4_CONTRACT/G13_4_PROBE_OUT/G13_4_OUT_ROOT）；
  2. Phase B（渲染）：逐 job MRQ 真跑（<map> -game -LevelSequence
     -MoviePipelineConfig -windowed -resx=<w> -resy=<h> -log
     -notexturestreaming -Unattended -FixedSeed 命令面闭集沿 G12.4）；
  3. 收割：output_dir 内 mtime ≥ run_start 的 .exr 新鲜度机核 + EXR magic +
     体积下限 + canonical digest（g10_determinism 单源）→ 帧库落盘
     K:/rurix-ext/g13-frames/ue_upscale/<scene>/tier<N> 与
     K:/rurix-ext/g13-frames/ue_lumen/<scene>/<on|off> + 收割 receipt JSON
     （含 contract_digest_ue〔build 探针转引，按臂取 M-c/M-d 键〕+ frames[] +
     exit_code + duration_s + output_tail + dlss_log_lines〔DLSS/Streamline/NGX
     日志行 ≤40——DLSS engagement 证据面，证 DLSS 真接管而非回退 TAAU〕）。

用法：
  py -3 milestones/g13/harness/g13_4_ue_render.py build [scene_id|--all] [--skip-import]
  py -3 milestones/g13/harness/g13_4_ue_render.py render upscale [scene_id|--all] [--tier N]
  py -3 milestones/g13/harness/g13_4_ue_render.py render lumen [scene_id|--all] [--mode on|off]
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
sys.path.insert(0, str(ROOT / "milestones" / "g10" / "harness" / "ue_python"))

from gpu_device_lock import gpu_device_lock  # noqa: E402
import g10_determinism as _det  # noqa: E402

UE_EXE = r"F:\UE_5.8\Engine\Binaries\Win64\UnrealEditor-Cmd.exe"
UPROJECT = r"K:\rurix-ext\g10-ue\G10RefRender\G10RefRender.uproject"
CONTRACT_UPSCALE = ROOT / "milestones" / "g13" / "g13_ue_upscale_parity_contract.json"
CONTRACT_LUMEN = ROOT / "milestones" / "g13" / "g13_ue_lumen_gi_parity_contract.json"
BUILD_SCRIPT = ROOT / "milestones" / "g13" / "harness" / "ue_python" / "g13_4_build_scenes.py"
OUT_ROOT = Path(r"K:\rurix-ext\g13-frames")
EXR_MAGIC = b"\x76\x2f\x31\x01"
# 512×512 RGBA f16 scanline NONE 实测 ≈2.1 MB/帧（G12.4 128×128 同形态 ≈130 KB
# 级 ×16 面积比推算；下限 100 KB 防合成小文件/空帧冒充，远低于真实帧量级）。
MIN_EXR_BYTES = 100_000
TIERS = (50, 67, 100)  # 契约 tier_sequence 闭集（与契约字面一致性由解析器机核）
DLSS_LOG_KEYS = ("DLSS", "Streamline", "NGX")
DLSS_LOG_MAX = 40
# UE 5.8.1 实测：-log 主日志落盘 Saved/Logs/<proj>.log，stdout 仅 bootstrap
# 初期行（dotnet/UBT/NGX 签名段）——DLSS engagement 证据面须双源收割
# （stdout ∪ 当次落盘日志，按 mtime ≥ run_start 新鲜度机核）。
UE_LOG = Path(r"K:\rurix-ext\g10-ue\G10RefRender\Saved\Logs\G10RefRender.log")

SCENES = {
    "cornell-box": {"map": "/Game/Maps/G13_CornellBox", "tag": "cornell_box", "w": 512, "h": 512},
    "bistro-interior": {"map": "/Game/Maps/G13_BistroInterior", "tag": "bistro_interior", "w": 1920, "h": 1080},
}


def run_editor(argv, timeout_s, env_extra=None, purpose="g13.4 ue"):
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    with gpu_device_lock(purpose=purpose):
        started = time.time()
        r = subprocess.run(argv, capture_output=True, timeout=timeout_s, env=env)
    out = (r.stdout + r.stderr).decode("utf-8", "replace")
    return started, r.returncode, out


def build(scene_id, skip_import=False):
    probe_out = OUT_ROOT / scene_id / "build_probe.json"
    probe_out.parent.mkdir(parents=True, exist_ok=True)
    env = {
        "G13_4_SCENE": scene_id,
        "G13_4_CONTRACT": str(CONTRACT_UPSCALE),
        "G13_4_PROBE_OUT": str(probe_out).replace("\\", "/"),
        "G13_4_OUT_ROOT": str(OUT_ROOT).replace("\\", "/"),
    }
    if skip_import:
        env["G13_4_SKIP_IMPORT"] = "1"
    argv = [UE_EXE, UPROJECT, f"-ExecutePythonScript={BUILD_SCRIPT}", "-unattended", "-log", "-nopause"]
    started, rc, out = run_editor(argv, 5400, env, purpose=f"g13.4 UE 建设 {scene_id}")
    lines = _log_lines(("G13_4_BUILD", "Error", "LogPython"), started, out)
    print("\n".join(lines[-40:]))
    _mt = UE_LOG.stat().st_mtime if UE_LOG.is_file() else None
    print(f"[g13_4_ue_render] log-dbg hits={len(lines)} ue_log_mtime={_mt} started={started:.3f}")
    print(f"[g13_4_ue_render] build scene={scene_id} exit={rc} duration_s={time.time()-started:.1f}")
    if rc != 0:
        return False
    if not probe_out.is_file():
        print(f"[g13_4_ue_render] build probe 缺失: {probe_out}")
        return False
    return True


def _contract_digest_ue(scene_id, arm):
    """build 探针转引（按臂取 M-c/M-d 键；探针缺失则 None——收割不阻塞）。"""
    probe = OUT_ROOT / scene_id / "build_probe.json"
    if not probe.is_file():
        return None
    try:
        doc = json.loads(probe.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None
    key = "contract_digest_ue" if arm == "upscale" else "contract_digest_ue_lumen"
    return doc.get(key)


def _log_lines(keys, started, out=None):
    """双源日志收割：stdout（若 out 给）∪ 当次 UE 落盘日志（mtime ≥ run_start
    新鲜度机核）；keys 子串命中，去重保序。UE 进程退出瞬间日志 flush/句柄释放
    有竞态（5.8.1 首跑实测收割空面）——落空时短退避重读（≤3 次 ×1s）。"""
    hits = []
    if out:
        hits.extend(l for l in out.splitlines() if any(k in l for k in keys))
    for attempt in range(3):
        file_hits = []
        if UE_LOG.is_file() and UE_LOG.stat().st_mtime >= started:
            try:
                text = UE_LOG.read_text(encoding="utf-8", errors="replace")
            except OSError:
                text = ""
            file_hits = [l for l in text.splitlines() if any(k in l for k in keys)]
        if file_hits or not UE_LOG.is_file() or attempt == 2:
            hits.extend(file_hits)
            break
        time.sleep(1.0)
    seen = set()
    uniq = []
    for l in hits:
        if l not in seen:
            seen.add(l)
            uniq.append(l)
    return uniq


def _dlss_log_lines(out, started):
    """DLSS engagement 证据面（最多 40 行，分层优先）：
    ① 核心 = "NGX DLSS Feature" Creating/Destroying 行（SrcRect/DestRect/
    NGXPerfQuality 档位+内部分辨率直证）+ "PaddedWindowNetwork" 张量分配行；
    ② 次级 = 其余 DLSS-SR/NGXPerfQuality 行；③ 初始化/签名行填充余量。
    实测面：单次 MRQ run engagement 51 行 > 40 限额，Creating 核心行时间序
    靠后会被裁——故分层保序（5.8.1 tier67 首跑 342×342 Creating 被裁实证）。"""
    hits = _log_lines(DLSS_LOG_KEYS, started, out)
    core = [l for l in hits if ("NGX DLSS Feature" in l) or ("PaddedWindowNetwork" in l)]
    core_set = set(core)
    sub = [l for l in hits if l not in core_set
           and any(k in l for k in ("DLSS-SR", "NGXPerfQuality"))]
    sub_set = set(sub)
    rest = [l for l in hits if l not in core_set and l not in sub_set]
    return (core + sub + rest)[:DLSS_LOG_MAX]


def render(scene_id, arm, variant):
    """variant：upscale 臂 = tier 整数；lumen 臂 = "on"|"off"。"""
    s = SCENES[scene_id]
    if arm == "upscale":
        cfg = f"/Game/Cinematics/G13_{s['tag']}_dlss_tier{variant}_Config"
        out_dir = OUT_ROOT / "ue_upscale" / scene_id / f"tier{variant}"
        job_desc = f"tier{variant}"
    else:
        cfg = f"/Game/Cinematics/G13_{s['tag']}_lumen_{variant}_Config"
        out_dir = OUT_ROOT / "ue_lumen" / scene_id / variant
        job_desc = f"lumen_{variant}"
    seq = f"/Game/Cinematics/G13_{s['tag']}_Seq"
    argv = [
        UE_EXE,
        UPROJECT,
        s["map"],
        "-game",
        f"-LevelSequence={seq}",
        f"-MoviePipelineConfig={cfg}",
        "-windowed",
        f"-resx={s['w']}",
        f"-resy={s['h']}",
        "-log",
        "-notexturestreaming",
        "-Unattended",
        "-FixedSeed",
    ]
    started, rc, out = run_editor(argv, 7200, purpose=f"g13.4 UE MRQ {scene_id} {arm} {job_desc}")
    tail = _log_lines(("MoviePipeline", "DLSS", "Lumen", "Error"), started, out)
    print("\n".join(tail[-25:]))
    dur = time.time() - started
    # 收割（新鲜度机核：mtime ≥ run_start）
    frames = []
    if out_dir.is_dir():
        for f in sorted(out_dir.rglob("*.exr")):
            if f.stat().st_mtime < started:
                continue
            blob = f.read_bytes()
            frames.append(
                {
                    "name": f.name,
                    "bytes": len(blob),
                    "exr_magic_ok": blob[:4] == EXR_MAGIC,
                    "canonical_digest": _det.exr_canonical_digest(
                        str(f), data_window=(s["h"], s["h"])
                    ),
                }
            )
    receipt = {
        "scene_id": scene_id,
        "arm": arm,
        "variant": job_desc,
        "config": cfg,
        "contract_digest_ue": _contract_digest_ue(scene_id, arm),
        "exit_code": rc,
        "duration_s": round(dur, 3),
        "started_epoch": started,
        "frames": frames,
        "dlss_log_lines": _dlss_log_lines(out, started),
        "output_tail": out[-1500:],
    }
    rp = out_dir / "render_receipt.json"
    rp.parent.mkdir(parents=True, exist_ok=True)
    rp.write_text(json.dumps(receipt, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    ok = rc == 0 and frames and all(
        f["exr_magic_ok"] and f["bytes"] >= MIN_EXR_BYTES for f in frames
    )
    print(
        f"[g13_4_ue_render] render scene={scene_id} arm={arm} variant={job_desc} exit={rc} "
        f"frames={len(frames)} dlss_log_lines={len(receipt['dlss_log_lines'])} "
        f"ok={bool(ok)} duration_s={dur:.1f}"
    )
    return bool(ok)


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    mode = sys.argv[1]
    if mode == "build":
        target = sys.argv[2]
        skip_import = "--skip-import" in sys.argv
        scenes = list(SCENES) if target == "--all" else [target]
        ok = all(build(s, skip_import) for s in scenes)
        return 0 if ok else 1
    if mode == "render":
        if len(sys.argv) < 4:
            print(__doc__)
            return 2
        arm = sys.argv[2]
        target = sys.argv[3]
        scenes = list(SCENES) if target == "--all" else [target]
        if arm == "upscale":
            tier_only = None
            if "--tier" in sys.argv:
                tier_only = int(sys.argv[sys.argv.index("--tier") + 1])
            ok = True
            for s in scenes:
                for tier in TIERS:
                    if tier_only is not None and tier != tier_only:
                        continue
                    ok = render(s, "upscale", tier) and ok
            return 0 if ok else 1
        if arm == "lumen":
            mode_only = None
            if "--mode" in sys.argv:
                mode_only = sys.argv[sys.argv.index("--mode") + 1]
            ok = True
            for s in scenes:
                for m in ("on", "off"):
                    if mode_only is not None and m != mode_only:
                        continue
                    ok = render(s, "lumen", m) and ok
            return 0 if ok else 1
        print("未知臂: " + arm)
        return 2
    print("未知模式: " + mode)
    return 2


if __name__ == "__main__":
    sys.exit(main())
