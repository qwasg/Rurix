#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.4a 波）
"""G10.4a M134 帧捕获管线门冒烟（步骤 181；
g10.p0.m134.frame_capture_pipeline；RFC-0026 §4.1；spec/imageio.md RXS-0385 +
spec/visual_comparison.md RXS-0386；G10_ACCEPTANCE_MAP §1 M134 行）。

host+device 门（device 腿持 gpu_device_lock 串行，RURIX_REQUIRE_REAL=1 +
RURIX_VK_VALIDATION=1）。判据：HDR 帧捕获落盘（EXR 自研最小子集——
src/image-io/src/exr.rs，float32 RGB scanline NONE，零外部依赖全 safe）+
捕获→回读逐像素往返无损（device 腿 GPU 真渲染→Rgba16Float readback→
fp16→f32 精确提升→EXR→回读位级；host 腿闭式探针图案同判）+ 分辨率/色彩
空间/位深元数据闭集齐备（ci/g10_exr_lib.py 独立第二实现互核 + 跨实现
digest 互证）+ 渲染输出探针图案位级核验（RFC-0026 §6.2 F16）+ UE 真帧
strip-and-log 读取（G10.2 已出真实 UE EXR，fp16→f32，闭集外属性剥离登记）。

RED 臂（契约 §4.2 M134 字面 + MAP PLAN §3 草案补充）：位深截断（8-bit
clamp）注入即 RED；sRGB/线性混标注入即 RED；元数据缺字段注入即 RED——
三臂 harness --red-arm 子模式独立复跑抽检 + 主流程内联实测双保险。
ZIP 解码 fail-closed 显式 UnsupportedCompression 登记（RXS-0385 L1 v1
实现面；本波实测 UE 5.8.1 MRQ 帧 compression=NONE）。

用法：
  py -3 ci/g10_frame_capture_pipeline_smoke.py --gate g10.p0.m134.frame_capture_pipeline
  py -3 ci/g10_frame_capture_pipeline_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m134_frame_capture_pipeline_evidence_schema.json"
SPEC_IMAGEIO = ROOT / "spec" / "imageio.md"
SPEC_VC = ROOT / "spec" / "visual_comparison.md"
WORK_DIR = ROOT / ".tmp" / "g104_gates" / "m134"
HARNESS_EVIDENCE = WORK_DIR / "harness_evidence.json"

sys.path.insert(0, str(ROOT / "ci"))
import g10_exr_lib  # noqa: E402
import g10_ue5_lib  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g10.p0.m134.frame_capture_pipeline"
NUMERIC_STEP = 181
SOURCE_REF = "RFC-0026 §4.1;spec/imageio.md RXS-0385;spec/visual_comparison.md RXS-0386;G10_ACCEPTANCE_MAP §1 M134"
TAG = "g10_m134"
SUBJECT = "g10_m134_frame_capture_pipeline"
MATRIX_ROW = "M134"
BIN = "g10_m134_frame_capture"

CHECK_KEYS = [
    "spec_rxs0385_clause_on_tree",
    "spec_rxs0386_clause_on_tree",
    "imageio_exr_unit_tests",
    "harness_build_vulkan",
    "harness_checks_all_true",
    "device_section_executed",
    "metadata_closed_set_complete",
    "cross_impl_digest_match",
    "ue_frame_strip_and_log",
    "zip_decode_fail_closed",
    "red_bit_depth_truncation_detected",
    "red_srgb_linear_mislabel_detected",
    "red_metadata_missing_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def run_cmd(argv: list[str], timeout: int = 3600, env: dict | None = None) -> subprocess.CompletedProcess:
    print(f"[{TAG}] $ {' '.join(argv)}")
    r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(argv), "exit_code": r.returncode})
    return r


def spec_clause(spec: Path, clause: str) -> bool:
    return spec.is_file() and (
        re.search(rf"^###\s+{clause}\b", spec.read_text(encoding="utf-8"), re.MULTILINE) is not None
    )


def find_ue_frame() -> Path | None:
    """G10.2 已出真实 UE EXR（最新 m128 run 目录首帧；环境日志 §7.3 登记面）。"""
    root = g10_ue5_lib.frames_root()
    if root is None:
        return None
    runs = sorted(root.glob("g10_gate_runs/m128_*/"))
    if not runs:
        return None
    for run in reversed(runs):
        exrs = sorted(run.glob("*.exr"))
        if exrs:
            return exrs[0]
    return None


def build_harness() -> Path | None:
    r = run_cmd([
        "cargo", "build", "-p", "rurix-render", "--features", "vulkan",
        "--bin", BIN,
    ])
    if r.returncode != 0:
        check(False, f"harness 构建失败: {(r.stdout + r.stderr)[-600:]}")
        return None
    exe = ROOT / "target" / "debug" / f"{BIN}.exe"
    return exe if exe.is_file() else None


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    env["RURIX_BASE_COMMIT"] = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True
    ).stdout.strip()
    return env


def run_harness(exe: Path, ue_frame: Path) -> dict | None:
    argv = [
        str(exe),
        "--evidence", str(HARNESS_EVIDENCE),
        "--ue-frame", str(ue_frame),
        "--work-dir", str(WORK_DIR),
    ]
    with gpu_device_lock(purpose="g10_m134 frame capture device 腿"):
        r = run_cmd(argv, env=device_env())
    out = r.stdout + r.stderr
    if "SKIP DEV_ENV_DEGRADE" in out:
        if require_real():
            check(False, f"harness SKIP（RURIX_REQUIRE_REAL=1 不许 SKIP）: {out.strip()[-400:]}")
        return None
    if r.returncode != 0:
        check(False, f"harness 非零退出: {out.strip()[-800:]}")
        return None
    if not HARNESS_EVIDENCE.is_file():
        check(False, "harness evidence 未落盘")
        return None
    try:
        return json.loads(HARNESS_EVIDENCE.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        check(False, f"harness evidence 不可解析: {e}")
        return None


def run_red_arm(exe: Path, arm: str) -> bool:
    argv = [str(exe), "--red-arm", arm]
    r = run_cmd(argv, timeout=300)
    ok = r.returncode == 0 and "PASS red-arm" in r.stdout
    if not ok:
        check(False, f"RED 臂 {arm} 复跑未检出: {(r.stdout + r.stderr).strip()[-300:]}")
    else:
        note(f"RED 检出 {arm}（--red-arm 复跑）")
    return ok


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂：cargo test -p image-io（EXR 单测全绿）+ harness --host-only 快速通道。
    r = run_cmd(["cargo", "test", "-p", "image-io"], timeout=1800)
    if r.returncode != 0 or "test result: ok" not in r.stdout:
        print(f"[{TAG}] selftest FAIL: image-io 单测非全绿", file=sys.stderr)
        return 1
    exe = build_harness()
    if exe is None:
        print(f"[{TAG}] selftest FAIL: harness 构建失败", file=sys.stderr)
        return 1
    r = run_cmd([str(exe), "--host-only", "--work-dir", str(WORK_DIR / "selftest")], timeout=600)
    if r.returncode != 0 or "PASS" not in r.stdout:
        print(f"[{TAG}] selftest FAIL: harness --host-only 非绿: {(r.stdout + r.stderr)[-300:]}", file=sys.stderr)
        return 1
    # 红臂：三注入复跑全检出。
    for arm in ("clamp-8bit", "srgb-linear-mislabel", "metadata-missing"):
        if not run_red_arm(exe, arm):
            print(f"[{TAG}] selftest FAIL: RED 臂 {arm} 漏检", file=sys.stderr)
            return 1
    # 绿臂：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (3 RED + 2 GREEN)")
    return 0


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

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}

    checks["spec_rxs0385_clause_on_tree"] = spec_clause(SPEC_IMAGEIO, "RXS-0385")
    check(checks["spec_rxs0385_clause_on_tree"], "spec/imageio.md 缺 RXS-0385 条款头")
    checks["spec_rxs0386_clause_on_tree"] = spec_clause(SPEC_VC, "RXS-0386")
    check(checks["spec_rxs0386_clause_on_tree"], "spec/visual_comparison.md 缺 RXS-0386 条款头")

    # host 段：image-io EXR 单测（含往返无损/闭集/分端/ZIP fail-closed/fp16 锚定）。
    r = run_cmd(["cargo", "test", "-p", "image-io"], timeout=1800)
    anchored = all(
        name in r.stdout
        for name in (
            "rgb_roundtrip_bit_exact_and_deterministic",
            "ue5_strip_and_log_read",
            "zip_compression_fail_closed",
            "half_to_f32_exact",
            "metadata_closed_set_enforced",
            "per_end_policy_enforced",
        )
    )
    checks["imageio_exr_unit_tests"] = r.returncode == 0 and "test result: ok" in r.stdout and anchored
    check(checks["imageio_exr_unit_tests"], "image-io EXR 单测非全绿或锚定缺失")

    # harness 构建 + 全档真跑（device 腿持锁）。
    exe = build_harness()
    checks["harness_build_vulkan"] = exe is not None
    harness = None
    ue_frame = find_ue_frame()
    if ue_frame is None:
        check(False, "UE 真帧缺失（K:\\rurix-ext\\g10-frames 无 m128 run；G10.2 产物不在树外）")
    if exe is not None and ue_frame is not None:
        harness = run_harness(exe, ue_frame)
    hchecks = (harness or {}).get("checks", {})
    checks["harness_checks_all_true"] = bool(hchecks) and all(hchecks.values())
    check(
        checks["harness_checks_all_true"],
        f"harness checks 非全真: {[k for k, v in hchecks.items() if not v]}",
    )
    device_state = (harness or {}).get("device_section_state", "fail")
    checks["device_section_executed"] = device_state == "executed"
    check(checks["device_section_executed"], f"device 腿未 executed: {device_state}")

    # 独立第二实现互核：元数据闭集齐备 + 跨实现 digest 互证。
    meta_ok = False
    digest_ok = False
    if harness is not None:
        try:
            host_exr = g10_exr_lib.decode_exr_file(WORK_DIR / "host_probe_frame.exr", "rurix")
            md = host_exr["metadata"] or {}
            required = {
                "rurix:schema_version", "rurix:domain", "rurix:transfer",
                "rurix:bit_depth", "rurix:source_end", "rurix:capture_params_digest",
                "rurix:derivation",
            }
            meta_ok = (
                required <= set(md)
                and md.get("rurix:domain") == "scene-linear-hdr"
                and md.get("rurix:transfer") == "linear"
                and md.get("rurix:bit_depth") == "float32"
                and md.get("rurix:source_end") == "rurix"
                and host_exr["width"] == 64
                and host_exr["height"] == 48
                and host_exr["chromaticities_ok"]
                and host_exr["source_bit_depth"] == "float32"
            )
            host_digest = g10_exr_lib.frame_content_digest(
                host_exr["width"], host_exr["height"], 3, host_exr["pixels"]
            )
            digest_ok = host_digest == harness.get("host_frame_digest")
            if device_state == "executed":
                dev_exr = g10_exr_lib.decode_exr_file(WORK_DIR / "device_captured_frame.exr", "rurix")
                dev_digest = g10_exr_lib.frame_content_digest(
                    dev_exr["width"], dev_exr["height"], 3, dev_exr["pixels"]
                )
                digest_ok = digest_ok and dev_digest == harness.get("device_frame_digest")
                meta_ok = meta_ok and (dev_exr["metadata"] or {}).get("rurix:domain") == "scene-linear-hdr"
            if not digest_ok:
                note(f"跨实现 digest 对账: host py={host_digest[:24]}… rust={str(harness.get('host_frame_digest'))[:24]}…")
        except Exception as e:  # noqa: BLE001 — 独立复核面异常即判据失效（fail-closed）
            check(False, f"独立 EXR 复核异常: {e}")
    checks["metadata_closed_set_complete"] = meta_ok
    check(meta_ok, "元数据闭集齐备性独立互核失败（分辨率/色彩空间/位深/九字段）")
    checks["cross_impl_digest_match"] = digest_ok
    check(digest_ok, "跨实现帧 digest 互证不一致（Rust bin vs ci Python 独立解析器）")

    # UE 真帧 strip-and-log：harness 判据 + 独立第二实现复核（剥离计数/位深/digest）。
    ue_ok = bool(hchecks.get("ue_frame_strip_and_log_read"))
    if harness is not None and ue_frame is not None:
        try:
            ue = g10_exr_lib.decode_exr_file(ue_frame, "ue5")
            ue_digest = g10_exr_lib.frame_content_digest(ue["width"], ue["height"], 3, ue["pixels"])
            ue_ok = ue_ok and (
                ue["source_bit_depth"] == "float16"
                and len(ue["stripped"]) == harness.get("ue_stripped_attribute_count")
                and any(s["reason"] == "ue5-strip-and-log" and s["name"].startswith("unreal/") for s in ue["stripped"])
                and any(s["reason"] == "alpha-channel-strip" for s in ue["stripped"])
                and ue_digest == harness.get("ue_frame_digest")
            )
            note(
                f"UE 真帧: {ue_frame.name} {ue['width']}×{ue['height']} fp16→f32, "
                f"stripped={len(ue['stripped'])}（独立复核 digest 一致）"
            )
        except Exception as e:  # noqa: BLE001
            check(False, f"UE 真帧独立复核异常: {e}")
            ue_ok = False
    checks["ue_frame_strip_and_log"] = ue_ok
    check(ue_ok, "UE 真帧 strip-and-log 读取判据失效")

    checks["zip_decode_fail_closed"] = bool(hchecks.get("zip_decode_fail_closed"))
    check(checks["zip_decode_fail_closed"], "ZIP fail-closed 失效")

    # RED 臂：harness 内联实测 + 门侧 --red-arm 独立复跑双保险。
    red_inline = (
        bool(hchecks.get("red_bit_depth_truncation_detected"))
        and bool(hchecks.get("red_srgb_linear_mislabel_detected"))
        and bool(hchecks.get("red_metadata_missing_detected"))
    )
    check(red_inline, "harness RED 臂内联实测漏检")
    if exe is not None:
        checks["red_bit_depth_truncation_detected"] = (
            bool(hchecks.get("red_bit_depth_truncation_detected"))
            and run_red_arm(exe, "clamp-8bit")
        )
        checks["red_srgb_linear_mislabel_detected"] = (
            bool(hchecks.get("red_srgb_linear_mislabel_detected"))
            and run_red_arm(exe, "srgb-linear-mislabel")
        )
        checks["red_metadata_missing_detected"] = (
            bool(hchecks.get("red_metadata_missing_detected"))
            and run_red_arm(exe, "metadata-missing")
        )
    for k in ("red_bit_depth_truncation_detected", "red_srgb_linear_mislabel_detected", "red_metadata_missing_detected"):
        check(checks[k], f"{k} 未检出")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES and device_state == "executed"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G10.4",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": device_state if device_state in ("executed", "not_applicable") else "fail",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "capture_report": {
            "harness": BIN,
            "harness_evidence": str(HARNESS_EVIDENCE.relative_to(ROOT)).replace("\\", "/"),
            "host_frame_digest": (harness or {}).get("host_frame_digest", ""),
            "host_exr_file_digest": (harness or {}).get("host_exr_file_digest", ""),
            "device_frame_digest": (harness or {}).get("device_frame_digest", ""),
            "device_name": (harness or {}).get("device_name", ""),
            "ue_frame": str(ue_frame) if ue_frame else "",
            "ue_frame_digest": (harness or {}).get("ue_frame_digest", ""),
            "ue_frame_bit_depth": (harness or {}).get("ue_frame_bit_depth", ""),
            "ue_stripped_attribute_count": (harness or {}).get("ue_stripped_attribute_count", 0),
            "exr_subset": "scanline float32 RGB/Y; compression v1 = NONE 编+解; ZIP fail-closed 显式 UnsupportedCompression（禁静默）",
            "compression_narrowing": "harness 收窄登记 = {NONE}（UE 5.8.1 MRQ 帧 compression=NONE 实测）",
            "cross_impl_verification": "ci/g10_exr_lib.py 独立第二实现（Python）互核元数据闭集 + 帧 digest 互证",
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（device+host 双腿往返无损位级 + 元数据闭集齐备 + 探针位级核验 + "
            f"UE 真帧 strip-and-log〔fp16→f32〕+ ZIP fail-closed + RED 三臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
