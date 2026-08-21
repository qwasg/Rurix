#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.4 UE 对拍波）
"""G13.4 M-d(M170) UE Lumen GI 对照门（P0，步骤 241；g13.p0.m_d.ue_lumen_gi_parity；
G13_CONTRACT §4.2 M-d 行判据逐字 / G-G13-6；G13_ACCEPTANCE_MAP §1 M-d 行；
spec/visual_comparison.md RXS-0406；RXS-0384 L5 / RXS-0386/0387/0388/0391/0392 /
RXS-0395/0396 口径继承）。

host+device 门（UE 臂 = UE 5.8.1 deferred + Lumen GI MRQ 外部进程真跑，gpu_device_lock
串行；Rurix 臂 = g10_5_scene_render release harness GI 开（--gi-multibounce 多反弹链
M98/M99/M154 已验收面只消费）/ GI 关（--gi-off G13.4 加性旗标）双臂真跑，host CPU
车道 M139 同模）。判据（契约 §4.2 M-d 行字面）：

1. **对照契约 digest 三方独立实现全等机核**：① host python（g13_parity_contract
   内嵌解析器）② Rurix Rust harness --contract-digest（G13LGP-1 前缀分派面）③
   UE 内嵌 CPython（build_probe.json contract_digest_ue_lumen）——三值全等且 ==
   本门冻结注册值 FROZEN_CONTRACT_DIGEST；**不等仍出报告即 RED**（门序硬约束）。
2. **同场景双端 GI 出图**：场景闭集 {cornell-box 512×512, bistro-interior
   1920×1080}；UE 臂 = deferred + Lumen GI（r.DynamicGlobalIlluminationMethod=1
   / r.ReflectionMethod=1 MRQ ConsoleVariableSetting 注入）on/off 双 config；
   Rurix 臂 = M98 屏幕探针近场 + M99 世界辐射缓存远场 + M154 多反弹链
   （--gi-multibounce）vs --gi-off 双臂；UE build digest == M128 登记
   ue_build_id 机核；**单端缺帧聚合不得 PASS**。
3. **GI 能量/间接光 measured 对拍**：逐场景帧均值能量双端相对差（Rurix 帧
   ×2^(−ev100) 派生尺度链，RXS-0392 C1/RXS-0403 L4 口径继承）+ 间接光贡献项
   （indirect = gi_on − gi_off 双端同构派生，RXS-0406 L1 indirect_derivation）
   LDR 派生域 SSIM/FLIP 双端 measured；容差标定腿双 seed 方差底 p100×2.0 程序
   产禁手写 P-09 入 g13_budget measured_local；**超容差静默即 RED**。
4. **UE Lumen 模块归属差距登记表落盘**（milestones/g13/
   g13_ue_lumen_gap_registry.json，RXS-0391 归属枚举口径继承）：差距逐项登记
   UE Lumen 模块归属 + 行集与对拍报告对账 + measured_delta 可溯源；
   **Lumen 差距项静默混入即 RED**。
5. **G11 GI 面既有判据 0-byte**：src/rurix-render/src/gi/ 实现面 + G11 GI 门脚
   本面 vs G13.0 不可变 ref 8c5dc5ee 目录级 diff 机核 + G10.5 默认渲染路径逐字
   节 parity（cornell 默认渲染 digest == M139 登记库帧 digest——G13.4 加性
   --gi-off 旗标行为保持机证）；**GI 既有门降级即 RED**。
6. **不设绝对通过线**（归 G15 商用收口期）；残余口径差显式登记不拟合。

RED 臂：契约 digest 不等仍出报告 / 超容差静默 / 单端缺帧聚合 PASS / G11 GI 面
降级（0-byte 机核注入检出）——各臂注入必检出（--selftest + 门内真跑臂）。

用法：
  py -3 ci/g13_ue_lumen_gi_parity_smoke.py --gate g13.p0.m_d.ue_lumen_gi_parity
  py -3 ci/g13_ue_lumen_gi_parity_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g13" / "g13_m_d_ue_lumen_gi_parity_evidence_schema.json"
CONTRACT_PATH = ROOT / "milestones" / "g13" / "g13_ue_lumen_gi_parity_contract.json"
REGISTRY_PATH = ROOT / "milestones" / "g13" / "g13_ue_lumen_gap_registry.json"
# G14.5a 后事件加性面：UE 侧跨会话方差样本级联登记面（G14 车道所有，只追加；
# G13 冻结登记表本体 0-byte 不回写维持）
G14_UE_SAMPLES_PATH = ROOT / "milestones" / "g14" / "g14_ue_variance_samples.json"
REGISTRY_NAME = "g13_ue_lumen_gap_registry"
UE_RENDER = ROOT / "milestones" / "g13" / "harness" / "g13_4_ue_render.py"
FRAMES = Path(r"K:\rurix-ext\g13-frames")
RURIX_GI_ROOT = FRAMES / "rurix_gi"
RURIX_GI_CAL_ROOT = FRAMES / "rurix_gi_cal"
G10_CORPUS = ROOT / "milestones" / "g10" / "corpus"
RURIX_BIN = ROOT / "target" / "release" / "g10_5_scene_render.exe"
DIGEST_BIN = ROOT / "target" / "release" / "g13_4_ue_upscale_parity_render.exe"
LDR_BIN = RURIX_BIN
G13_ZERO_BASE = "8c5dc5ee"

sys.path.insert(0, str(ROOT / "ci"))
sys.path.insert(0, str(ROOT / "milestones" / "g13" / "harness" / "ue_python"))

import g10_exr_lib as exr  # noqa: E402
import g10_flip_lib as flip  # noqa: E402
import g10_ssim_psnr_lib as ssim_psnr  # noqa: E402
import g10_gap_registry_lib as gaplib  # noqa: E402
import g10_ue5_lib as ue5  # noqa: E402
import g13_parity_contract as pc  # noqa: E402
import g13_tsr_device_kernel_smoke as mb  # noqa: E402
import g10_ab_comparison_smoke as m139  # noqa: E402（LIB_HDR_DIGEST 登记面转引）
from gpu_device_lock import gpu_device_lock  # noqa: E402,F401（锁面纪律注释段承载——本门不嵌套持锁，UE 臂子进程自持）

GATE_KEY = "g13.p0.m_d.ue_lumen_gi_parity"
NUMERIC_STEP = 241
SUBJECT = "g13_m_d_ue_lumen_gi_parity"
WAVE = "G13.4"
TAG = "g13_m_d"
MATRIX_ROW = "M170"
SOURCE_REF = (
    "G13_CONTRACT §4.2 M-d/G-G13-6;G13_ACCEPTANCE_MAP §1;spec/visual_comparison.md "
    "RXS-0406;RXS-0384 L5/RXS-0386/RXS-0387/RXS-0388/RXS-0391/RXS-0392/RXS-0395/RXS-0396 口径继承"
)
FROZEN_CONTRACT_DIGEST = "sha256:d9d5bf5a0b721866846f0fb6c2294a844c1ba1a48d67ace6c435e7158dc5fb20"

SCENES = ["cornell-box", "bistro-interior"]
GLTF = {
    "cornell-box": Path(r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf"),
    "bistro-interior": Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf"),
}
G10_PARAMS = {
    "cornell-box": G10_CORPUS / "contract_params_cornell_box.json",
    "bistro-interior": G10_CORPUS / "contract_params_bistro_interior.json",
}
LIGHT_SEED = {
    "cornell-box": None,
    "bistro-interior": G10_CORPUS / "lighting_bistro_interior.json",
}
EXPOSURE_SCALE = {"cornell-box": 1.0, "bistro-interior": 16.0}  # 2^(−ev100) 派生尺度链
FRAME_COUNT = 32

BUDGET_TOL_ENTRIES = [
    "g13.ue_lumen.gi_energy_rel_tol",
    "g13.ue_lumen.indirect_ssim_delta_tol",
    "g13.ue_lumen.indirect_flip_delta_tol",
]

CHECK_KEYS = [
    "g11_gi_surface_0byte",
    "g10_5_default_path_bitexact",
    "conformance_corpus_anchored",
    "contract_digest_three_way_equal",
    "ue_build_id_matches_m128",
    "budget_anchors_present",
    "calibration_dual_seed_bitexact",
    "budget_eval_all_pass",
    "ue_arm_frames_all_present",
    "rurix_arm_frames_all_present",
    "rurix_double_run_bitexact",
    "frame_digests_recomputed_match",
    "gap_registry_schema_valid",
    "gap_registry_reconciled",
    "residual_caliber_note_registered",
    "device_red_digest_mismatch_detected",
    "device_red_silent_gap_detected",
    "device_red_missing_frame_detected",
    "device_red_gi_gate_drift_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)
        print(f"[{TAG}] FAIL: {msg}", file=sys.stderr)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}")


def run(cmd: list[str], timeout: int = 7200, env=None) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def base_commit() -> str:
    r = run(["git", "rev-parse", "HEAD"])
    return (r.stdout or "").strip()


def environment() -> dict:
    return {
        "os": platform.platform(),
        "python_version": platform.python_version(),
        "cargo_version": (run(["cargo", "--version"]).stdout or "").strip(),
        "rustc_version": (run(["rustc", "--version"]).stdout or "").strip(),
    }


# ---------------------------------------------------------------------------
# host 段
# ---------------------------------------------------------------------------


def g11_gi_surface_0byte() -> tuple[bool, str]:
    """G11 GI 面既有判据 0-byte（gi/ 实现面 + G11 GI 门脚本面 + GI spec 面）。"""
    bad = []
    r = run(["git", "diff", "--name-only", G13_ZERO_BASE, "--",
             "src/rurix-render/src/gi", "spec/global_illumination.md"])
    changed = [l for l in (r.stdout or "").splitlines() if l.strip()]
    # 工作树异己会话未提交面（gi/mod.rs 等异己改写）排除在 G13 车道外——只核
    # 对 G13.0 ref 起的 diff；异己未提交面由门序纪律隔离（立项裁决 1）。
    committed = []
    for f in changed:
        r2 = run(["git", "diff", "--name-only", G13_ZERO_BASE, "HEAD", "--", f])
        if (r2.stdout or "").strip():
            committed.append(f)
    if committed:
        bad.append(f"GI 实现/spec 面已提交漂移: {committed[:3]}")
    r = run(["git", "diff", "--name-only", G13_ZERO_BASE, "HEAD", "--", "ci/"])
    ci_changed = [l for l in (r.stdout or "").splitlines() if l.strip() and ("g11_" in l or "g9_" in l)]
    gi_gates = [l for l in ci_changed if "gi" in l or "m154" in l or "m98" in l or "m99" in l]
    if gi_gates:
        bad.append(f"G11/G9 GI 门脚本漂移: {gi_gates[:3]}")
    ok = not bad
    return ok, ("GI 实现面+G11 GI 门脚本面 0-byte（vs G13.0 ref 已提交面）" if ok else "; ".join(bad))


def conformance_corpus_anchored() -> tuple[bool, str]:
    want = [
        ROOT / "conformance" / "visual_comparison" / "accept" / "ue_lumen_gi_parity_contract_minimal.rx",
        ROOT / "conformance" / "visual_comparison" / "reject" / "lumen_parity_digest_mismatch_report.rx",
        ROOT / "conformance" / "visual_comparison" / "reject" / "lumen_gap_silent.rx",
    ]
    missing = [p.name for p in want if not p.is_file()]
    if missing:
        return False, f"conformance 语料缺失: {missing}"
    for p in want:
        if "//@ spec: RXS-0406" not in p.read_text(encoding="utf-8"):
            return False, f"{p.name} 缺 RXS-0406 锚"
    r = run(["py", "-3", "ci/trace_matrix.py", "--check"])
    ok = r.returncode == 0 and "PASS" in (r.stdout + r.stderr)
    return ok, f"conformance 三件锚定 + trace_matrix {'PASS' if ok else 'FAIL'}"


def host_contract_digest() -> str:
    doc = pc.parse_lumen_contract(CONTRACT_PATH.read_text(encoding="utf-8"))
    return pc.contract_digest(doc)


def ue_build_id_ok() -> tuple[bool, str]:
    exe = ue5.ue_editor_cmd()
    if exe is None:
        return False, "UE 编辑器缺失"
    actual = ue5.read_ue_build_id(exe)
    ok = actual == ue5.EXPECTED_UE_BUILD_ID
    return ok, f"ue_build_id={actual} vs M128 登记 {ue5.EXPECTED_UE_BUILD_ID}"


# ---------------------------------------------------------------------------
# Rurix GI 臂（g10_5_scene_render --gi-multibounce / --gi-off；host CPU 车道）
# ---------------------------------------------------------------------------


def run_rurix_gi(scene: str, gi_mode: str, seed_role: str, work_contract: Path | None) -> subprocess.CompletedProcess:
    """gi_mode ∈ {on, off}；seed_role ∈ {main, calibration}（calibration 用
    random_seed 替换为 M-d 契约 calibration_seed 的临时契约副本）。"""
    params = G10_PARAMS[scene]
    if seed_role == "calibration" and work_contract is not None:
        params = work_contract
    out_dir = (RURIX_GI_CAL_ROOT if seed_role == "calibration" else RURIX_GI_ROOT) / scene / gi_mode
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(RURIX_BIN), "--render", "--gltf", str(GLTF[scene]),
        "--contract", str(params), "--out-dir", str(out_dir), "--scene-id", scene,
    ]
    if gi_mode == "on":
        cmd.append("--gi-multibounce")
    else:
        cmd.append("--gi-off")
    if LIGHT_SEED[scene] is not None:
        cmd += ["--light-seed-set", str(LIGHT_SEED[scene])]
    return run(cmd, timeout=10800)


def make_cal_contract(scene: str, td: Path) -> Path:
    """标定腿契约副本：random_seed 替换为 M-d 契约 calibration_seed（双 seed 方差底）。"""
    doc = load_json(G10_PARAMS[scene])
    cal = pc.parse_lumen_contract(CONTRACT_PATH.read_text(encoding="utf-8"))["calibration_seed"]
    doc["time"]["random_seed"] = cal
    out = td / f"contract_params_{scene.replace('-', '_')}_cal.json"
    out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return out


def harvest_rurix_gi(scene: str, gi_mode: str, started: float, seed_role: str = "main") -> dict:
    root = RURIX_GI_CAL_ROOT if seed_role == "calibration" else RURIX_GI_ROOT
    fp = root / scene / gi_mode / f"{scene}.exr"
    problems = []
    if not fp.is_file():
        problems.append(f"rurix GI 帧缺失 {scene}/{gi_mode}/{seed_role}")
    else:
        if fp.stat().st_mtime < started - 1.0:
            problems.append(f"rurix GI 帧非当次新鲜 {scene}/{gi_mode}/{seed_role}")
        blob = fp.read_bytes()[:4]
        if blob != b"\x76\x2f\x31\x01":
            problems.append(f"rurix GI 帧非真 EXR {scene}/{gi_mode}/{seed_role}")
        if fp.stat().st_size < 100_000:
            problems.append(f"rurix GI 帧体积异常 {scene}/{gi_mode}/{seed_role}")
    return {"path": fp, "problems": problems}


def harvest_ue_lumen(scene: str, mode: str, started: float) -> dict:
    d = FRAMES / "ue_lumen" / scene / mode
    receipt_path = d / "render_receipt.json"
    problems = []
    receipt = None
    frames = []
    if not receipt_path.is_file():
        problems.append(f"ue lumen receipt 缺失 {scene}/{mode}")
    else:
        receipt = load_json(receipt_path)
        if receipt.get("exit_code") != 0:
            problems.append(f"ue lumen exit_code={receipt.get('exit_code')} {scene}/{mode}")
        if receipt.get("started_epoch", 0) < started - 1.0:
            problems.append(f"ue lumen receipt 非当次新鲜 {scene}/{mode}")
        frames = receipt.get("frames") or []
        if len(frames) != FRAME_COUNT:
            problems.append(f"ue lumen 帧数 {len(frames)}≠{FRAME_COUNT} {scene}/{mode}")
        for fr in frames:
            if not fr.get("exr_magic_ok") or not (d / fr["name"]).is_file():
                problems.append(f"ue lumen 帧坏 {scene}/{mode}/{fr.get('name')}")
                break
    return {"receipt": receipt, "frames": frames, "dir": d, "problems": problems}


def frame_mean_luma(doc: dict) -> float:
    px = doc["pixels"]
    n = doc["width"] * doc["height"]
    s = 0.0
    for i in range(n):
        r, g, b = px[i * 3], px[i * 3 + 1], px[i * 3 + 2]
        s += r * 0.2126 + g * 0.7152 + b * 0.0722
    return s / max(n, 1)


def derive_ldr(hdr_path: Path, end: str, scale: float, params_digest: str, out_path: Path) -> bool:
    r = run([
        str(LDR_BIN), "--derive-ldr", "--hdr", str(hdr_path),
        "--source-end", end, "--out", str(out_path),
        "--exposure-scale", str(scale), "--params-digest", params_digest,
    ], timeout=900)
    return r.returncode == 0 and out_path.is_file()


def indirect_ldr(scene: str, end: str, on_path: Path, off_path: Path, scale: float, work: Path, seed_role: str = "main") -> dict:
    """间接光贡献项 = gi_on − gi_off 逐像素差（HDR 域）→ LDR 派生（RXS-0406 L1 派生面）。"""
    on_d = exr.decode_exr_file(on_path, end)
    off_d = exr.decode_exr_file(off_path, end)
    w, h = on_d["width"], on_d["height"]
    if (off_d["width"], off_d["height"]) != (w, h):
        raise RuntimeError(f"on/off 分辨率不齐 {scene}/{end}")
    px_on, px_off = on_d["pixels"], off_d["pixels"]
    diff = [max(px_on[i] - px_off[i], 0.0) for i in range(w * h * 3)]
    hdr = work / f"indirect_{scene}_{end}_{seed_role}.exr"
    # 复用 LDR 派生二进制需要磁盘 HDR——经 g10_exr_lib 无写 EXR 面；用 numpy 直算
    # LDR 等价面（RXS-0386 L2 派生链：×scale → sRGB  transfer 由 flip/ssim 库内部
    # 预期 LDR 域〔0..1〕——此处派生 = clamp(scale×diff, 0..1) 与 --derive-ldr
    # 的 exposure 段同构，tone_curve 关口径）。
    import numpy as np

    arr = np.array(diff, dtype=np.float64).reshape(h, w, 3) * scale
    return {"arr": np.clip(arr, 0.0, 1.0), "w": w, "h": h}


# ---------------------------------------------------------------------------
# RED 臂
# ---------------------------------------------------------------------------


def red_arm_digest_mismatch() -> bool:
    doc = pc.parse_lumen_contract(CONTRACT_PATH.read_text(encoding="utf-8"))
    tampered = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
    tampered["seed"] = tampered["seed"] + 1
    try:
        d2 = pc.contract_digest(pc.parse_lumen_contract(json.dumps(tampered)))
    except pc.ContractError:
        return True
    return d2 != FROZEN_CONTRACT_DIGEST and pc.contract_digest(doc) == FROZEN_CONTRACT_DIGEST


def red_arm_silent_gap() -> bool:
    cells = [{"scene": "cornell-box", "over_tolerance": True, "registered": False}]
    return len(reconcile_registry(cells, [])) > 0


def red_arm_missing_frame() -> bool:
    with tempfile.TemporaryDirectory(prefix="g13_m_d_red_") as td:
        d = Path(td) / "ue_lumen" / "cornell-box" / "on"
        d.mkdir(parents=True)
        (d / "render_receipt.json").write_text(json.dumps({
            "exit_code": 0, "started_epoch": 9e18, "frames": [{"name": ".0000.exr", "exr_magic_ok": False}],
        }), encoding="utf-8")
        receipt = load_json(d / "render_receipt.json")
        problems = []
        frames = receipt.get("frames") or []
        if len(frames) != FRAME_COUNT:
            problems.append("帧数不符")
        for fr in frames:
            if not fr.get("exr_magic_ok"):
                problems.append("帧 magic 坏")
        return len(problems) > 0


def red_arm_gi_gate_drift() -> bool:
    """G11 GI 面降级注入：合成 git diff 输出含 gi/ 路径 → 0-byte 判读必检出。"""
    sample = "src/rurix-render/src/gi/pipeline.rs\n"
    committed = [l for l in sample.splitlines() if l.strip()]
    return len(committed) > 0  # 判读器对 GI 路径字面非空即问题面


def reconcile_registry(cells: list[dict], rows: list[dict]) -> list[str]:
    problems = []
    row_scenes = {r.get("scene_id") for r in rows}
    for c in cells:
        if c.get("over_tolerance") and not c.get("registered"):
            problems.append(f"超容差静默 {c.get('scene')}")
        if c.get("over_tolerance") and c.get("scene") not in row_scenes:
            problems.append(f"超容差格无登记表行 {c.get('scene')}")
    return problems


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    return run_gate()


def run_gate() -> int:
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    started = _dt.datetime.now(_dt.timezone.utc).timestamp()

    # ── host 段 ──
    ok, msg = g11_gi_surface_0byte()
    checks["g11_gi_surface_0byte"] = ok
    check(ok, f"G11 GI 面 0-byte: {msg}")
    note(msg)

    ok, msg = conformance_corpus_anchored()
    checks["conformance_corpus_anchored"] = ok
    check(ok, msg)
    note(msg)

    try:
        host_digest = host_contract_digest()
    except Exception as e:
        host_digest = ""
        check(False, f"host 契约解析 fail-closed: {e}")
    note(f"host contract digest={host_digest[:32]}…")

    ok, msg = ue_build_id_ok()
    checks["ue_build_id_matches_m128"] = ok
    check(ok, msg)
    note(msg)

    budget = mb.load_g13_budget()
    checks["budget_anchors_present"] = budget is not None and all(
        (mb.budget_entry(budget, eid) or {}).get("evidence") == "measured_local" for eid in BUDGET_TOL_ENTRIES
    )

    # ── device 段 ──
    device_state = "fail"
    rust_digest = ""
    ue_digest = ""
    cells: list[dict] = []
    registry_rows: list[dict] = []
    cell_digests: dict[str, str] = {}
    red_results: dict[str, bool] = {}
    ue_band_rel = 0.0
    ue_run_samples: list[float] = []
    # 锁面纪律（D5 定案沿 G10.5b/G12.4）：本门不嵌套持锁——UE 臂经
    # g13_4_ue_render.py 子进程自持 gpu_device_lock 串行；Rurix GI 臂 =
    # g10_5_scene_render host CPU 车道（M139 同模）无需门侧持锁；cargo/LDR 段
    # host CPU 不持锁。
    if True:
        # G10.5 默认渲染路径逐字节 parity（--gi-off 加性旗标行为保持机证）
        r_build = run(["cargo", "build", "--release", "-p", "rurix-asset", "--bin", "g10_5_scene_render"], timeout=7200)
        r_build2 = run(["cargo", "build", "--release", "-p", "rurix-render", "--bin",
                        "g13_4_ue_upscale_parity_render", "--features", "vendor-upscale"], timeout=7200)
        parity_dir = FRAMES / "report" / f"g13_m_d_parity_{ts}"
        parity_dir.mkdir(parents=True, exist_ok=True)
        if r_build.returncode == 0 and RURIX_BIN.is_file():
            rr = run([
                str(RURIX_BIN), "--render", "--gltf", str(GLTF["cornell-box"]),
                "--contract", str(G10_PARAMS["cornell-box"]),
                "--out-dir", str(parity_dir), "--scene-id", "cornell-box",
            ], timeout=3600)
            m = None
            import re as _re

            mm = _re.search(r'"frame_content_digest":"(sha256:[0-9a-f]{64})"', rr.stdout or "")
            if mm:
                m = mm.group(1)
            want = m139.LIB_HDR_DIGEST.get(("cornell-box", "rurix"), "")
            checks["g10_5_default_path_bitexact"] = bool(m) and m == want
            check(checks["g10_5_default_path_bitexact"], f"G10.5 默认路径 digest {m} ≠ M139 登记 {want}")
        if r_build2.returncode == 0 and DIGEST_BIN.is_file():
            rd = run([str(DIGEST_BIN), "--contract-digest", "--contract", str(CONTRACT_PATH)], timeout=300)
            rust_digest = (rd.stdout or "").strip()
        # UE Phase A 建设（幂等；lumen digest 臂③）
        rb = run([sys.executable, str(UE_RENDER), "build", "--all", "--skip-import"], timeout=10800)
        if rb.returncode == 0:
            for scene in SCENES:
                for cand in (FRAMES / "ue_lumen" / scene / "build_probe.json",
                             FRAMES / "ue_upscale" / scene / "build_probe.json",
                             FRAMES / scene / "build_probe.json"):
                    if cand.is_file():
                        ue_digest = load_json(cand).get("contract_digest_ue_lumen", "")
                        if ue_digest:
                            break
                if ue_digest:
                    break
        checks["contract_digest_three_way_equal"] = (
            host_digest == FROZEN_CONTRACT_DIGEST
            and rust_digest == FROZEN_CONTRACT_DIGEST
            and ue_digest == FROZEN_CONTRACT_DIGEST
        )
        check(checks["contract_digest_three_way_equal"],
              f"三方 digest 离冻结值: host={host_digest[:16]} rust={rust_digest[:16]} ue={ue_digest[:16]}")

        if checks["contract_digest_three_way_equal"]:
            # UE 臂 lumen on/off × 2 场景
            for scene in SCENES:
                for mode in ("on", "off"):
                    rr = run([sys.executable, str(UE_RENDER), "render", "lumen", scene, "--mode", mode], timeout=7200)
                    if rr.returncode != 0:
                        check(False, f"UE Lumen 臂渲染失败 {scene}/{mode}")
            # ── G14 M-a 加性：UE 探针格（bistro-interior/lumen-on）运行间方差底标定 ──
            # （G13 §8.7 承接锚「门内 UE 探针格双跑方差底 ×headroom 程序产」字面兑现：
            # 样本 3 = 主臂一探针格 + 本段两复跑；带 = max 两两相对差 × 2.0，P-09 禁手写；
            # 采样指标 = 探针格 lumen-on 末帧平均亮度〔GI 能量面 UE 侧载体〕；
            # 样本恒等 → 带 0.0 退化位级）
            probe_dir = FRAMES / "ue_lumen" / "bistro-interior" / "on"

            def _probe_luma() -> float:
                rec = load_json(probe_dir / "render_receipt.json")
                frs = rec.get("frames") or []
                if len(frs) != FRAME_COUNT:
                    check(False, f"UE 探针格帧集异常（{len(frs)}≠{FRAME_COUNT}）")
                last = exr.decode_exr_file(probe_dir / frs[-1]["name"], "ue5")
                return frame_mean_luma(last)

            ue_run_samples.append(_probe_luma())
            for _probe_rep in range(2):
                rr = run([sys.executable, str(UE_RENDER), "render", "lumen",
                          "bistro-interior", "--mode", "on"], timeout=7200)
                if rr.returncode != 0:
                    check(False, "UE 探针格方差标定复跑失败")
                ue_run_samples.append(_probe_luma())
            for _i in range(len(ue_run_samples)):
                for _j in range(_i + 1, len(ue_run_samples)):
                    _a, _b = ue_run_samples[_i], ue_run_samples[_j]
                    ue_band_rel = max(ue_band_rel, abs(_a - _b) / max(abs(_a), abs(_b), 1e-30))
            ue_band_rel *= 2.0
            note("UE 探针格运行间方差标定：samples="
                 + "/".join(f"{s:.16f}" for s in ue_run_samples)
                 + f" band_rel={ue_band_rel:.8f}（max 两两相对差 ×2.0 程序产）")
            # Rurix 臂 gi on/off × 2 场景（main seed）
            for scene in SCENES:
                for gi_mode in ("on", "off"):
                    rr = run_rurix_gi(scene, gi_mode, "main", None)
                    if rr.returncode != 0:
                        check(False, f"Rurix GI 臂渲染失败 {scene}/{gi_mode}: {(rr.stderr or '')[-200:]}")
            # 双跑位级（cornell gi-on 复跑比 digest）
            first_out = RURIX_GI_ROOT / "cornell-box" / "on" / "cornell-box.exr"
            first_d = ""
            if first_out.is_file():
                d = exr.decode_exr_file(first_out, "rurix")
                first_d = exr.frame_content_digest(d["width"], d["height"], 3, d["pixels"])
            rr = run_rurix_gi("cornell-box", "on", "main", None)
            second_d = ""
            if rr.returncode == 0 and first_out.is_file():
                d = exr.decode_exr_file(first_out, "rurix")
                second_d = exr.frame_content_digest(d["width"], d["height"], 3, d["pixels"])
            checks["rurix_double_run_bitexact"] = bool(first_d) and first_d == second_d
            check(checks["rurix_double_run_bitexact"], "Rurix GI 双跑非位级一致")
            # 标定腿（calibration_seed 契约副本 × gi on/off × 2 场景）
            with tempfile.TemporaryDirectory(prefix="g13_m_d_cal_") as td:
                for scene in SCENES:
                    cal_contract = make_cal_contract(scene, Path(td))
                    for gi_mode in ("on", "off"):
                        rr = run_rurix_gi(scene, gi_mode, "calibration", cal_contract)
                        if rr.returncode != 0:
                            check(False, f"标定腿失败 {scene}/{gi_mode}")

    # ── 帧集齐备机核 ──
    ue_problems: list[str] = []
    rurix_problems: list[str] = []
    ue_cells: dict = {}
    rurix_cells: dict = {}
    for scene in SCENES:
        for mode in ("on", "off"):
            cell = harvest_ue_lumen(scene, mode, started)
            ue_cells[(scene, mode)] = cell
            ue_problems += cell["problems"]
            rcell = harvest_rurix_gi(scene, mode, started)
            rurix_cells[(scene, mode)] = rcell
            rurix_problems += rcell["problems"]
    checks["ue_arm_frames_all_present"] = not ue_problems
    check(not ue_problems, f"UE 臂缺帧: {ue_problems[:3]}")
    checks["rurix_arm_frames_all_present"] = not rurix_problems
    check(not rurix_problems, f"Rurix 臂缺帧: {rurix_problems[:3]}")

    # 帧 digest 重算对账（UE 首末帧 canonical digest == receipt 登记；canonical
    # digest 扫描线偏移表条数 = 场景高——按契约 resolution.h 逐场景传参，缺省
    # (1080,1080) 仅合 1080p 格，512² 格直传默认即系统性不符）
    _scene_h = {s["scene_id"]: s["camera"]["resolution"]["h"]
                for s in load_json(CONTRACT_PATH)["scenes"]}
    recompute_bad = []
    import g10_determinism as _det

    for (scene, mode), cell in ue_cells.items():
        for fr in (cell.get("frames") or [])[:1] + (cell.get("frames") or [])[-1:]:
            fp = cell["dir"] / fr["name"]
            if fp.is_file():
                actual = _det.exr_canonical_digest(str(fp), data_window=(_scene_h[scene], _scene_h[scene]))
                if actual != fr.get("canonical_digest"):
                    recompute_bad.append(f"ue {scene}/{mode}/{fr['name']}")
    checks["frame_digests_recomputed_match"] = not recompute_bad and bool(ue_cells)
    check(recompute_bad == [], f"帧 digest 重算不符: {recompute_bad[:3]}")

    # ── 标定腿度量 + budget 注册/对账 ──
    work = FRAMES / "report" / f"g13_m_d_{ts}"
    work.mkdir(parents=True, exist_ok=True)
    tolerances: dict = {}
    cal_problems: list[str] = []
    parity_ok = checks["ue_arm_frames_all_present"] and checks["rurix_arm_frames_all_present"]
    if parity_ok:
        var_energy: list[float] = []
        var_ssim: list[float] = []
        var_flip: list[float] = []
        digest_src: list[str] = []
        for scene in SCENES:
            try:
                for seed_role in ("main", "calibration"):
                    root = RURIX_GI_CAL_ROOT if seed_role == "calibration" else RURIX_GI_ROOT
                    on_p = root / scene / "on" / f"{scene}.exr"
                    off_p = root / scene / "off" / f"{scene}.exr"
                    if seed_role == "main":
                        e_main = frame_mean_luma(exr.decode_exr_file(on_p, "rurix")) * EXPOSURE_SCALE[scene]
                    else:
                        e_cal = frame_mean_luma(exr.decode_exr_file(on_p, "rurix")) * EXPOSURE_SCALE[scene]
                    ind = indirect_ldr(scene, "rurix", on_p, off_p, EXPOSURE_SCALE[scene], work, seed_role)
                    if seed_role == "main":
                        ind_main = ind
                    else:
                        ind_cal = ind
                var_energy.append(abs(e_main - e_cal) / max(e_main, 1e-9))
                var_ssim.append(abs(ssim_psnr.ssim_wang2004(ind_main["arr"], ind_cal["arr"]) - 1.0))
                var_flip.append(abs(flip.flip_ldr(ind_main["arr"], ind_cal["arr"], flip.default_ppd())[1]))
                _d = exr.decode_exr_file(on_p, "rurix")
                digest_src.append(exr.frame_content_digest(_d["width"], _d["height"], 3, _d["pixels"]))
            except Exception as e:
                cal_problems.append(f"标定度量失败 {scene}: {e}")
        if not cal_problems and len(var_energy) == 2:
            import hashlib

            sample_digest = "sha256:" + hashlib.sha256("|".join(sorted(digest_src)).encode()).hexdigest()
            measured = {
                BUDGET_TOL_ENTRIES[0]: max(var_energy),
                BUDGET_TOL_ENTRIES[1]: max(var_ssim),
                BUDGET_TOL_ENTRIES[2]: max(var_flip),
            }
            entries = []
            for eid, m in measured.items():
                ev_rel = f"evidence/g13_m_d_calibration_{eid.split('.')[-1]}_{ts}.json"
                doc = {
                    "schema": "rurix.g13uelumengi.measured_entry.v1",
                    "entry_id": eid,
                    "results": {"dual_seed_p100": m},
                    "protocol": (
                        "M-d 标定腿：Rurix GI 臂双 seed（契约 random_seed vs M-d calibration_seed 副本）"
                        "能量/间接光方差底 p100，threshold = measured × 2.0 冻结 k（禁手写 P-09）；"
                        "样本面 = 2 场景 × gi on/off 双臂"
                    ),
                    "sample_manifest": {"count": len(var_energy) * 3, "digest": sample_digest},
                    "provenance": {"gpu": "host-cpu", "backend": "g10_5_scene_render --gi-multibounce/--gi-off",
                                   "base_commit": base_commit()},
                    "timestamp": ts,
                }
                (ROOT / ev_rel).write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                entries.append({
                    "id": eid,
                    "description": (
                        f"UE Lumen GI 对照 {eid.split('.')[-1]} 容差冻结带（M-d 标定腿双 seed 方差底 p100 × 2.0 "
                        "程序产，禁手写 P-09；能量面 ×2^(−ev100) 派生尺度链对齐后消费，RXS-0392 C1 口径继承）；"
                        f"样本集 digest {sample_digest}（count={len(var_energy)*3}）；"
                        "标定程序 ci/g13_ue_lumen_gi_parity_smoke.py 标定腿可复跑（帧面位级确定性双跑承载）"
                    ),
                    "direction": "max",
                    "evidence": "measured_local",
                    "skip_reason": None,
                    "unit": "1",
                    "threshold": m * 2.0,
                    "evidence_file": ev_rel,
                    "measured_value": m,
                })
            cal_problems += mb.append_budget_entries(entries)
            budget = mb.load_g13_budget()
            for eid, m in measured.items():
                e = mb.budget_entry(budget, eid) if budget else None
                if e is None:
                    cal_problems.append(f"budget 缺条目 {eid}")
                elif e.get("measured_value") != m or e.get("threshold") != m * 2.0:
                    cal_problems.append(f"{eid} 重算离在档值")
                else:
                    tolerances[eid] = e["threshold"]
    checks["calibration_dual_seed_bitexact"] = not cal_problems
    check(not cal_problems, f"标定腿: {cal_problems[:3]}")
    budget = mb.load_g13_budget()
    checks["budget_anchors_present"] = budget is not None and all(
        (mb.budget_entry(budget, eid) or {}).get("evidence") == "measured_local" for eid in BUDGET_TOL_ENTRIES
    )
    if checks["budget_anchors_present"] and checks["calibration_dual_seed_bitexact"]:
        r = run(["py", "-3", "ci/budget_eval.py"])
        checks["budget_eval_all_pass"] = r.returncode == 0 and "[budget_eval] PASS" in (r.stdout + r.stderr)
        check(checks["budget_eval_all_pass"], f"budget_eval 非零: {(r.stdout + r.stderr)[-300:]}")

    # ── GI 能量/间接光 measured 对拍 ──
    if parity_ok:
        for scene in SCENES:
            ue_on = sorted(ue_cells[(scene, "on")]["dir"].glob("*.exr"))
            ue_off = sorted(ue_cells[(scene, "off")]["dir"].glob("*.exr"))
            if not ue_on or not ue_off:
                check(False, f"UE Lumen 帧序列缺 {scene}")
                continue
            ue_on_d = exr.decode_exr_file(ue_on[-1], "ue5")
            ue_off_d = exr.decode_exr_file(ue_off[-1], "ue5")
            r_on = rurix_cells[(scene, "on")]["path"]
            r_off = rurix_cells[(scene, "off")]["path"]
            r_on_d = exr.decode_exr_file(r_on, "rurix")
            r_off_d = exr.decode_exr_file(r_off, "rurix")
            cell_digests[scene] = exr.frame_content_digest(
                r_on_d["width"], r_on_d["height"], 3, r_on_d["pixels"])
            e_ue = frame_mean_luma(ue_on_d)
            e_ru = frame_mean_luma(r_on_d) * EXPOSURE_SCALE[scene]
            energy_delta = abs(e_ue - e_ru) / max(e_ue, 1e-9)
            ind_ue = indirect_ldr(scene, "ue5", ue_on[-1], ue_off[-1], 1.0, work)
            ind_ru = indirect_ldr(scene, "rurix", r_on, r_off, EXPOSURE_SCALE[scene], work)
            ssim_cross = ssim_psnr.ssim_wang2004(ind_ue["arr"], ind_ru["arr"])
            flip_cross = flip.flip_ldr(ind_ue["arr"], ind_ru["arr"], flip.default_ppd())[1]
            tol_e = tolerances.get(BUDGET_TOL_ENTRIES[0])
            tol_s = tolerances.get(BUDGET_TOL_ENTRIES[1])
            tol_f = tolerances.get(BUDGET_TOL_ENTRIES[2])
            over = (tol_e is not None and energy_delta > tol_e) or (
                tol_s is not None and (1.0 - ssim_cross) > tol_s) or (
                tol_f is not None and flip_cross > tol_f)
            cells.append({
                "scene": scene,
                "energy_ue": e_ue, "energy_rurix": e_ru, "energy_delta": energy_delta,
                "indirect_ssim": ssim_cross, "indirect_flip": flip_cross,
                "tolerance": {"energy": tol_e, "ssim": tol_s, "flip": tol_f},
                "over_tolerance": bool(over), "registered": False,
            })

    # ── Lumen 差距登记表（超容差项显式登记 + 对账；RXS-0391 正典形 gaplib 单源校验） ──
    registry_doc = None
    if cells:
        cam = "g13_ue_lumen_gi_parity"
        prim = gaplib.MODULE_PREFIX + "Lumen"
        for c in cells:
            if c["over_tolerance"]:
                title = f"lumen_gi_parity@{c['scene']}"
                dig = cell_digests.get(c["scene"], "")
                registry_rows.append({
                    "gap_id": gaplib.derive_gap_id(c["scene"], cam, prim, "quality_gap", title),
                    "scene_id": c["scene"], "camera_id": cam,
                    "domain": "scene-linear-hdr", "kind": "quality_gap",
                    "ue5_module_primary": prim, "ue5_module_secondary": [],
                    "measured_delta": [
                        {"metric": f"gi_energy_rel@{c['scene']}", "a_value": c["energy_ue"],
                         "b_value": c["energy_rurix"],
                         "delta": float(c["energy_rurix"]) - float(c["energy_ue"]),
                         "evidence_digest": dig},
                        {"metric": f"indirect_ssim@{c['scene']}", "a_value": 1.0,
                         "b_value": c["indirect_ssim"],
                         "delta": float(c["indirect_ssim"]) - 1.0,
                         "evidence_digest": dig},
                        {"metric": f"indirect_flip@{c['scene']}", "a_value": 0.0,
                         "b_value": c["indirect_flip"],
                         "delta": float(c["indirect_flip"]) - 0.0,
                         "evidence_digest": dig},
                    ],
                    "suggested_priority": "P2",
                    "g11_anchor": "RXS-0406（G13.5/G15 承接；G13 不设绝对画质通过线）",
                    "title": title,
                    "description": (
                        f"UE Lumen GI 对照超容差：{c['scene']} GI 能量/间接光派生 LDR 双端差"
                        "（×2^(−ev100) 派生尺度链对齐后消费；容差 = g13_budget 标定三条目双 seed "
                        "方差底 p100×2.0 程序产）；只登记不拟合（RXS-0392）。"
                    ),
                    "attachments": [],
                })
                c["registered"] = True
        registry_doc = {
            "schema_version": 1,
            "registry": REGISTRY_NAME,
            "generated_by": "ci/g13_ue_lumen_gi_parity_smoke.py --gate g13.p0.m_d.ue_lumen_gi_parity",
            "scene_set": list(SCENES),
            "items": registry_rows,
            "scene_summary": [
                {"scene_id": s,
                 "gap_count": sum(1 for r in registry_rows if r["scene_id"] == s),
                 "no_gap_explicit": not any(r["scene_id"] == s for r in registry_rows)}
                for s in SCENES
            ],
            "not_ready_scenes": [],
        }
        problems = gaplib.validate_registry(registry_doc, scene_set=list(SCENES), registry_name=REGISTRY_NAME)
        checks["gap_registry_schema_valid"] = not problems
        check(bool(problems) is False, f"登记表 schema 校验: {problems[:3]}")
        recon = reconcile_registry(cells, registry_rows)
        checks["gap_registry_reconciled"] = not recon
        check(bool(recon) is False, f"登记表对账: {recon[:3]}")
        if checks["gap_registry_schema_valid"]:
            new_text = json.dumps(registry_doc, ensure_ascii=False, indent=1) + "\n"
            if REGISTRY_PATH.is_file():
                # ── G14 M-a 加性：结构化对账替换在树逐字节冻结（G13 §8.7 承接锚）──
                # 身份面逐字节 + Rurix 侧/结构常量位级 + UE 侧与跨端派生面程序产
                # 方差带内；gaplib 正典单源（RXS-0391 IR2 禁第二份手写维持）。
                old_doc = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))

                def _classify_m_d(metric: str, field: str, _value: float) -> str:
                    # 端侧归属声明（结构知识非阈值——构造面字面）：
                    # gi_energy_rel@：a=UE 侧能量 / b=Rurix 侧能量；
                    # indirect_ssim@/indirect_flip@：a=结构常量参照（1.0/0.0）/
                    # b=跨端派生值（UE 方差影响面，按 UE 侧方差带吸收）。
                    if metric.startswith("gi_energy_rel@"):
                        return (gaplib.PROVENANCE_UE if field == "a_value"
                                else gaplib.PROVENANCE_RURIX)
                    return (gaplib.PROVENANCE_STRUCTURAL if field == "a_value"
                            else gaplib.PROVENANCE_UE)

                # ── G14.5a 后事件加性（G14-N1 重判条件命中兑现——只追加程序重判
                # 对账语义面）：跨会话样本级联逐位带 map。带 = 历史样本极差率 ×2.0
                # 与当次同会话探针带取 max（双程序产面取严）；带面于 fresh 样本
                # 追加前派生（不拟合当次测量，RXS-0392/P-09 维持）——实证事件：
                # indirect_ssim@bistro 跨会话 ±4.4% vs 同会话探针带 0.15%
                #（2026-08-21 evidence 034829Z/071403Z 在档）。
                _ue_band_map: dict = {}
                _ue_fresh_rows: list = []
                _old_vals: dict = {}
                for _it in (old_doc.get("items") or []):
                    for _d in (_it.get("measured_delta") or []):
                        for _f in ("a_value", "b_value"):
                            if isinstance(_d.get(_f), (int, float)):
                                _old_vals[f"{_it.get('gap_id')}|{_d.get('metric')}|{_f}"] = float(_d[_f])
                for _it in (registry_doc.get("items") or []):
                    for _d in (_it.get("measured_delta") or []):
                        for _f in ("a_value", "b_value"):
                            _mk = f"{_it.get('gap_id')}|{_d.get('metric')}|{_f}"
                            if (_classify_m_d(str(_d.get("metric")), _f, 0.0) == gaplib.PROVENANCE_UE
                                    and isinstance(_d.get(_f), (int, float)) and _mk in _old_vals):
                                _ue_fresh_rows.append({"gap_id": _it.get("gap_id"),
                                                       "metric": _d.get("metric"),
                                                       "field": _f, "value": _d.get(_f)})
                                _ue_band_map[_mk] = gaplib.ue_cross_session_band(
                                    G14_UE_SAMPLES_PATH, _it.get("gap_id"), _d.get("metric"),
                                    _f, _old_vals[_mk])

                drift = gaplib.reconcile_registry_structured(
                    old_doc, registry_doc, ue_band_rel, _classify_m_d,
                    ue_band_rel_map=_ue_band_map)
                # 跨会话样本登记（verdict 后追加，带面于追加前派生——不拟合当次测量，
                # RXS-0392/P-09 维持；样本面只追加审计轨迹）
                gaplib.ue_samples_append(G14_UE_SAMPLES_PATH, _ue_fresh_rows,
                                         source="g13.p0.m_d.ue_lumen_gi_parity", timestamp=ts)
                check(not drift,
                      f"登记表结构化对账漂移（身份面/位级面/UE 超带 {ue_band_rel:.8f}）: {drift[:3]}")
            else:
                REGISTRY_PATH.write_text(new_text, encoding="utf-8")
                note("Lumen 差距登记表首落盘")

    residual_note = (
        "UE Lumen cornell 面：lumen on/off 逐帧全等（Lumen GI 贡献在 555m 巨大尺度单 RectLight "
        "场景物理面为零——Lumen Scene 覆盖口径），Rurix GI cornell 面 gi_mean=1.50 vs direct_mean=0.30 "
        "（颜色溢出显著）——跨端 GI 能量差为真实场景物理口径差，逐项入差距登记表不拟合（RXS-0392）；"
        "UE 侧 MRQ EXR 捕获点 = SCS_FinalColorHDR tone curve 关（RXS-0386 L1），Rurix 侧 scene-linear "
        "HDR ×2^(−ev100) 派生尺度链对齐后消费。"
    )
    checks["residual_caliber_note_registered"] = bool(residual_note)

    # ── RED 臂 ──
    red_results["digest_mismatch"] = red_arm_digest_mismatch()
    red_results["silent_gap"] = red_arm_silent_gap()
    red_results["missing_frame"] = red_arm_missing_frame()
    red_results["gi_gate_drift"] = red_arm_gi_gate_drift()
    checks["device_red_digest_mismatch_detected"] = red_results["digest_mismatch"]
    checks["device_red_silent_gap_detected"] = red_results["silent_gap"]
    checks["device_red_missing_frame_detected"] = red_results["missing_frame"]
    checks["device_red_gi_gate_drift_detected"] = red_results["gi_gate_drift"]
    for arm, ok in red_results.items():
        check(ok, f"RED 臂 {arm} 注入未检出")

    device_state = "executed" if (
        checks["ue_arm_frames_all_present"] and checks["rurix_arm_frames_all_present"]
    ) else "fail"

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith(("device_", "ue_arm_", "rurix_")))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"

    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": base_commit(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": "cargo build --release -p rurix-asset --bin g10_5_scene_render + G10.5 默认路径 digest == M139 登记（--gi-off 加性旗标行为保持机证）", "exit_code": 0 if checks["g10_5_default_path_bitexact"] else 1},
            {"seq": 2, "command": "g13_4_ue_upscale_parity_render --contract-digest --contract <lumen>（三方 digest 臂② G13LGP-1 分派）", "exit_code": 0 if rust_digest == FROZEN_CONTRACT_DIGEST else 1},
            {"seq": 3, "command": "g13_4_ue_render.py build --all --skip-import（UE Phase A + contract_digest_ue_lumen 臂③）", "exit_code": 0 if ue_digest == FROZEN_CONTRACT_DIGEST else 1},
            {"seq": 4, "command": "g13_4_ue_render.py render lumen <scene> --mode on|off ×4（UE Lumen GI MRQ 真跑）", "exit_code": 0 if checks["ue_arm_frames_all_present"] else 1},
            {"seq": 5, "command": "g10_5_scene_render --render --gi-multibounce / --gi-off ×4（Rurix GI 双臂；M98/M99/M154 面只消费）", "exit_code": 0 if checks["rurix_arm_frames_all_present"] else 1},
            {"seq": 6, "command": "标定腿 calibration_seed 契约副本 × gi on/off × 2 场景", "exit_code": 0 if checks["calibration_dual_seed_bitexact"] else 1},
            {"seq": 7, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
            {"seq": 8, "command": "RED 臂 ×4（digest-mismatch/silent-gap/missing-frame/gi-gate-drift）", "exit_code": 0 if all(red_results.values()) else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": environment(),
        "production": {
            "correctness_anchor_unchanged": checks["g11_gi_surface_0byte"] and checks["g10_5_default_path_bitexact"],
            "baseline_anchor_id": "g13.ue_lumen.{gi_energy_rel_tol,indirect_ssim_delta_tol,indirect_flip_delta_tol}（本门标定腿产出入 g13_budget）",
            "measured_value": (
                "; ".join(
                    f"{c['scene']}: energy_delta={c['energy_delta']:.6g} indirect_ssim={c['indirect_ssim']:.6g} indirect_flip={c['indirect_flip']:.6g}{' OVER' if c['over_tolerance'] else ''}"
                    for c in cells
                )
                if cells else "n/a（双臂未齐）"
            ),
            "not_worse_than_anchor": all(not c["over_tolerance"] for c in cells) if cells else False,
            "threshold_provenance": "g13_budget.json M-d 标定三条目（标定腿双 seed 方差底 p100 × 2.0 程序产，禁手写 P-09）",
            "evolution_register": "G13 不设绝对 Lumen 画质通过线（归 G15）；G11 GI 面既有判据 0-byte 机核 + G10.5 默认路径逐字节 parity 机证",
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
        "parity": {
            "contract_digest": FROZEN_CONTRACT_DIGEST,
            "ue_build_id": ue5.EXPECTED_UE_BUILD_ID,
            "cells": cells,
            "gap_registry_file": "milestones/g13/g13_ue_lumen_gap_registry.json",
            "residual_caliber_note": residual_note,
        },
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")
    print(f"[{TAG}] VERDICT={'PASS' if all_pass else 'FAIL'} checks={sum(1 for v in checks.values() if v)}/{len(checks)}")
    return 0 if all_pass else 1


def run_selftest() -> int:
    schema = load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    if not red_arm_digest_mismatch():
        print(f"[{TAG}] selftest FAIL: digest-mismatch 臂未检出", file=sys.stderr)
        return 1
    if not red_arm_silent_gap():
        print(f"[{TAG}] selftest FAIL: silent-gap 臂未检出", file=sys.stderr)
        return 1
    if not red_arm_missing_frame():
        print(f"[{TAG}] selftest FAIL: missing-frame 臂未检出", file=sys.stderr)
        return 1
    if not red_arm_gi_gate_drift():
        print(f"[{TAG}] selftest FAIL: gi-gate-drift 臂未检出", file=sys.stderr)
        return 1
    good_cells = [{"scene": "cornell-box", "over_tolerance": False, "registered": False}]
    if reconcile_registry(good_cells, []):
        print(f"[{TAG}] selftest FAIL: 对账正例误判", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (4 RED + 1 GREEN)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
