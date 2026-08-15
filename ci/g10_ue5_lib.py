#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.2 波 materialize）
"""G10.2 UE5 出图门共享判定层（ci/g10_ue5_capture_environment_smoke.py /
ci/g10_ue5_reference_frames_smoke.py 复用；同构 ci/g10_corpus_lib.py 先例）。

职责闭集：
  - UE 5.8 外部进程事实面解析（UnrealEditor-Cmd 路径 / Build.version →
    ue_build_id / uproject / MRQ 资产在位）；
  - GPU 环境画像采集（nvidia-smi 驱动/时钟/锁频状态，实测不手写）；
  - MRQ Phase B 真跑（gpu_device_lock 串行；subprocess list-form 禁 shell
    拼接；命令面沿 RXS-0380 L2 臂 A 形态 + -LevelSequence 实测参数）；
  - 新出帧收割（mtime ≥ run_start 的新鲜度机核，防预置假帧冒充）与 EXR
    canonical digest（复用 harness g10_determinism.exr_canonical_digest
    实测过的 14 属性剥离逻辑，单一事实源）；
  - provenance 七元组闭集构建（RXS-0380 L3；相机/光照 digest 经 harness
    g10_param_contract.section_param_digest 实算）；
  - spec 条款头在树机核（spec-first 门序）。

主机绝对路径、时间戳、用户名不进入签名面（RXS-0380 L3 字面）。
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HARNESS_UE_PYTHON = ROOT / "milestones" / "g10" / "harness" / "ue_python"

sys.path.insert(0, str(HARNESS_UE_PYTHON))
import g10_determinism as _det  # noqa: E402
import g10_param_contract as _pc  # noqa: E402

DEFAULT_UE_EDITOR_CMD = r"F:\UE_5.8\Engine\Binaries\Win64\UnrealEditor-Cmd.exe"
DEFAULT_UPROJECT = r"K:\rurix-ext\g10-ue\G10RefRender\G10RefRender.uproject"
DEFAULT_FRAMES_ROOT = r"K:\rurix-ext\g10-frames"
MRQ_MAP_ENTRY = "/Engine/Maps/Entry"
MRQ_SEQ = "/Game/Cinematics/G10_SmokeSeq"
MRQ_CFG = "/Game/Cinematics/G10_SmokeConfig"
CONTRACT_PARAMS_PATH = (
    ROOT / "milestones" / "g10" / "harness" / "examples" / "contract_params_entry_smoke.json"
)
PROVISIONAL_SCENE_SET_PATH = ROOT / "milestones" / "g10" / "g10_2_provisional_scene_set.json"

EXPECTED_UE_BUILD_ID = "5.8.1-56057345"
EXR_MAGIC = b"\x76\x2f\x31\x01"
MIN_REAL_EXR_BYTES = 1_000_000  # 1920×1080 float NONE 实测 ≈16.6 MB；下限防合成小文件冒充

PROVENANCE_SEPTUPLE = (
    "scene_id",
    "camera_params_digest",
    "lighting_params_digest",
    "ue_build_id",
    "gpu_driver_version",
    "clock_lock_state",
    "capture_arm",
)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def ue_editor_cmd() -> Path | None:
    p = Path(os.environ.get("RURIX_G10_UE_EDITOR_CMD", DEFAULT_UE_EDITOR_CMD))
    return p if p.is_file() else None


def uproject_path() -> Path | None:
    p = Path(os.environ.get("RURIX_G10_UPROJECT", DEFAULT_UPROJECT))
    return p if p.is_file() else None


def frames_root() -> Path | None:
    p = Path(os.environ.get("RURIX_G10_FRAMES_ROOT", DEFAULT_FRAMES_ROOT))
    return p if p.is_dir() else None


def read_ue_build_id(ue_exe: Path) -> str | None:
    """Build.version 实测 → '<Major>.<Minor>.<Patch>-<Changelist>'（文本，非目录哈希）。"""
    # ue_exe = <root>/Engine/Binaries/Win64/UnrealEditor-Cmd.exe → parents[2] = Engine
    bv = ue_exe.parents[2] / "Build" / "Build.version"
    if not bv.is_file():
        return None
    data = load_json(bv)
    return (
        f"{data['MajorVersion']}.{data['MinorVersion']}.{data['PatchVersion']}"
        f"-{data['Changelist']}"
    )


def mrq_assets_present() -> bool:
    proj = uproject_path()
    if proj is None:
        return False
    content = proj.parent / "Content" / "Cinematics"
    return (content / "G10_SmokeSeq.uasset").is_file() and (
        content / "G10_SmokeConfig.uasset"
    ).is_file()


def gpu_profile() -> dict | None:
    """nvidia-smi 实测环境画像（驱动/当前与最大 SM 时钟/锁频状态）。

    锁频判定：`clocks.applications.graphics` 查询 N/A ⇒ 未应用锁频（环境日志
    §6「时钟未锁（idle 210 MHz / max 3105 MHz）」同口径）。
    """
    def _q(fields: str) -> list[str] | None:
        try:
            r = subprocess.run(
                ["nvidia-smi", f"--query-gpu={fields}", "--format=csv,noheader"],
                capture_output=True,
                text=True,
                timeout=30,
            )
        except (OSError, subprocess.TimeoutExpired):
            return None
        if r.returncode != 0:
            return None
        return [c.strip() for c in r.stdout.strip().split(",")]

    base = _q("name,driver_version,clocks.sm,clocks.max.sm")
    if base is None:
        return None
    app = _q("clocks.applications.graphics") or ["N/A"]
    lock_state = "unlocked" if app[0].upper() in ("N/A", "") else "locked"
    return {
        "gpu_name": base[0],
        "gpu_driver_version": base[1],
        "clocks_sm_current": base[2],
        "clocks_sm_max": base[3],
        "clock_lock_state": lock_state,
    }


def build_mrq_argv(ue_exe: Path, uproject: Path) -> list[str]:
    """臂 A（MRQ 主路）命令形态（RXS-0380 L2 + Phase B spike 实证参数）。"""
    return [
        str(ue_exe),
        str(uproject),
        MRQ_MAP_ENTRY,
        "-game",
        f"-LevelSequence={MRQ_SEQ}",
        f"-MoviePipelineConfig={MRQ_CFG}",
        "-windowed",
        "-resx=1920",
        "-resy=1080",
        "-log",
        "-notexturestreaming",
        "-Unattended",
        "-FixedSeed",
    ]


def command_surface_digest(argv: list[str]) -> str:
    """命令面配置 digest（capture_arm 签名面组件；首位可执行文件绝对路径不入）。"""
    payload = "\x00".join(argv[1:]).encode("utf-8")
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def run_process(argv: list[str], timeout_s: int = 1500) -> dict:
    """subprocess list-form 真跑（禁 shell 拼接）。返回 exit_code/duration/output tail。"""
    import time

    t0 = time.time()
    r = subprocess.run(argv, capture_output=True, timeout=timeout_s)
    return {
        "exit_code": r.returncode,
        "duration_s": round(time.time() - t0, 3),
        "output_tail": (r.stdout + r.stderr).decode("utf-8", "replace")[-800:],
    }


def run_mrq_phase_b(ue_exe: Path, uproject: Path, gpu_lock, timeout_s: int = 1500) -> dict:
    """持 GPU 锁真跑 MRQ Phase B；返回 run receipt（exit_code/argv/duration/digest）。"""
    argv = build_mrq_argv(ue_exe, uproject)
    with gpu_lock(purpose="g10.2 UE5 MRQ Phase B 出帧"):
        import time

        started = time.time()
        res = run_process(argv, timeout_s=timeout_s)
    return {
        "argv": argv,
        "command_surface_digest": command_surface_digest(argv),
        "started_epoch": started,
        "exit_code": res["exit_code"],
        "duration_s": res["duration_s"],
        "output_tail": res["output_tail"],
    }


def harvest_new_frames(output_dir: Path, since_epoch: float, dest_dir: Path) -> list[dict]:
    """收割 output_dir 内 mtime ≥ since_epoch 的 .exr 到 dest_dir（新鲜度机核）。

    返回逐帧 {name, bytes, exr_magic_ok, canonical_digest}；canonical digest 经
    harness g10_determinism.exr_canonical_digest 实算（14 属性剥离单一事实源）。
    """
    import shutil

    dest_dir.mkdir(parents=True, exist_ok=True)
    frames: list[dict] = []
    for f in sorted(output_dir.iterdir()):
        if not f.is_file() or not f.name.lower().endswith(".exr"):
            continue
        if f.stat().st_mtime < since_epoch:
            continue
        dest = dest_dir / f.name
        shutil.copy2(f, dest)
        blob = dest.read_bytes()
        frames.append(
            {
                "name": f.name,
                "bytes": len(blob),
                "exr_magic_ok": blob[:4] == EXR_MAGIC,
                "canonical_digest": _det.exr_canonical_digest(str(dest)),
            }
        )
    return frames


def frames_are_real(frames: list[dict]) -> list[str]:
    """真帧判据：EXR magic + 体积下限 + canonical digest 可算（防恒定合成帧）。"""
    fails: list[str] = []
    for fr in frames:
        if not fr["exr_magic_ok"]:
            fails.append(f"{fr['name']}: EXR magic 不符（非真 EXR 帧）")
        if fr["bytes"] < MIN_REAL_EXR_BYTES:
            fails.append(f"{fr['name']}: 体积 {fr['bytes']} < 下限（疑似合成小文件）")
        if not re.fullmatch(r"[0-9a-f]{64}", fr.get("canonical_digest", "")):
            fails.append(f"{fr['name']}: canonical digest 不可算")
    return fails


def contract_digests(params_path: Path = CONTRACT_PARAMS_PATH) -> dict:
    """契约参数解析 → 相机/光照节 canonical digest + 全量 digest（UE 侧解析器实算）。"""
    contract = _pc.parse_contract(params_path.read_text(encoding="utf-8"))
    return {
        "camera_params_digest": _pc.section_param_digest(contract, "camera"),
        "lighting_params_digest": _pc.section_param_digest(contract, "lighting"),
        "param_digest": "sha256:" + _pc.param_digest(contract),
    }


def build_provenance(scene_id: str, ue_build_id: str, profile: dict, capture_arm: str) -> dict:
    """provenance 七元组闭集（RXS-0380 L3；顺序即闭集顺序）。"""
    cd = contract_digests()
    return {
        "scene_id": scene_id,
        "camera_params_digest": cd["camera_params_digest"],
        "lighting_params_digest": cd["lighting_params_digest"],
        "ue_build_id": ue_build_id,
        "gpu_driver_version": profile["gpu_driver_version"],
        "clock_lock_state": profile["clock_lock_state"],
        "capture_arm": capture_arm,
    }


def provenance_failures(prov: dict) -> list[str]:
    """七元组缺行/空值即失败（provenance 缺行即 RED）。"""
    fails: list[str] = []
    for k in PROVENANCE_SEPTUPLE:
        v = prov.get(k)
        if not isinstance(v, str) or not v.strip():
            fails.append(f"provenance 缺行/空值: {k}")
    extra = set(prov) - set(PROVENANCE_SEPTUPLE)
    if extra:
        fails.append(f"provenance 闭集外字段: {sorted(extra)}")
    return fails


def spec_clause_head_on_tree(spec_rel: str, clause_id: str) -> bool:
    p = ROOT / spec_rel
    if not p.is_file():
        return False
    return re.search(rf"^###\s+{clause_id}\b", p.read_text(encoding="utf-8"), re.MULTILINE) is not None


def exr_canonical_digest(path: Path) -> str:
    return _det.exr_canonical_digest(str(path))


def frame_sha256(path: Path) -> str:
    return _det.frame_sha256(str(path))
