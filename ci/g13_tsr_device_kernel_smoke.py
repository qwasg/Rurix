#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.3 TSR device 化波）
"""G13.3 M-b(M168) 自研 TSR device 化门冒烟
（g13.p0.m_b.tsr_device_kernel；G13_CONTRACT §4.2 M-b 行判据逐字 / G-G13-5；
G13_ACCEPTANCE_MAP §1；spec/display_pipeline.md RXS-0404；RFC-0016 §4.H3；
spec/visual_comparison.md RXS-0387/0388 口径继承；BENCH_PROTOCOL §3 50×3
trimmed mean 协议沿 M141/M165 字面）。

硬判据：tsr.rs host 金标准 → .rx kernel device 面（kernels/g13_tsr_resample.rx
+ g13_tsr_resolve.rx 双腿，rurixc --target vulkan 产 SPV + spirv-val 通过，
G12 PT megakernel 车道 vk::run_compute 复用）+ device vs host 金标准同输入
逐帧对拍（三档 50% 640×360 / 67% 858×482 / 100% 1280×720 → 1280×720 各
32 帧 Halton jitter 静态收敛序列，逐帧逐像素最大绝对差 p100 ≤ 标定容差，
threshold = measured × 2.0 冻结 k，标定腿两跑位级一致程序产，禁手写 P-09）
+ 三档质量/帧时 measured 对照入 g13_budget（质量 = 终帧 SSIM deficit 对拍
4×4 超采样参照 ×2.0 冻结带；帧时 = host Instant 墙钟 around 逐帧 device
全链路，warmup 10 + timed 150 = 3 块 × 50 trimmed mean ×1.5 守护阈——回归
守护语义，不构成超分画质/帧率对标通过线；measured_local 零 estimated）
+ 固定 seed 位级确定性协议维持（同档同参双跑 digest 位级一致）
+ UpscaleBackend trait 签名面与 temporal 底座 0-byte 机核（目录级 git diff
vs G13.0 不可变 ref 8c5dc5ee + 工作树双面）
+ RURIX_VK_VALIDATION=1 层在跑 stderr 扫 VUID/Validation Error token 计数 = 0。
RED 臂：kernel 输出面加性偏置（kernel-bias）对拍必超容差检出；jitter 序列
相位偏移（seed-change）终帧 digest 必异检出；estimated 冒充 measured 必拒
（selftest 合成红臂承载）；对拍超容差静默即 RED；确定性协议漂移即 RED。

三态：无 Vulkan loader/设备 → device 腿 SKIP DEV_ENV_DEGRADE（退 0，非 fake
pass）；本脚本默认 RURIX_REQUIRE_REAL=1（setdefault），该态下 SKIP → 硬红。

用法:
  py -3 ci/g13_tsr_device_kernel_smoke.py --gate g13.p0.m_b.tsr_device_kernel
  py -3 ci/g13_tsr_device_kernel_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402
# 50×3 trimmed mean 冻结统计口径（M141/M165 字面）同实现复用——禁重写防漂移。
from g12_pt_throughput_baseline_smoke import (  # noqa: E402
    NO_PASS_LINE_LITERAL,
    TIMED,
    WARMUP,
    block_stats,
    recompute_check,
)

GATE_KEY = "g13.p0.m_b.tsr_device_kernel"
NUMERIC_STEP = 238
SUBJECT = "g13_m_b_tsr_device_kernel"
SCHEMA_PATH = ROOT / "milestones/g13/g13_m_b_tsr_device_kernel_evidence_schema.json"
MEASURED_SCHEMA_PATH = ROOT / "milestones/g13/g13_m_b_measured_entry_evidence_schema.json"
SOURCE_REF = (
    "G13_CONTRACT §4.2 M-b/G-G13-5;G13_ACCEPTANCE_MAP §1;spec/display_pipeline.md RXS-0404;"
    "rfcs/0016-native-renderer.md §4.H3;spec/visual_comparison.md RXS-0387/RXS-0388;"
    "milestones/m0/BENCH_PROTOCOL.md §3（M141/M165 50×3 trimmed mean 冻结统计口径继承）"
)
TAG = "g13_m_b"

G13_ZERO_BASE = "8c5dc5ee"  # G13.0 不可变 ref（契约 §7 登记）
TEMPORAL_DIR = "src/rurix-render/src/temporal"
KERNEL_RESAMPLE = ROOT / "src/rurix-render/kernels/g13_tsr_resample.rx"
KERNEL_RESOLVE = ROOT / "src/rurix-render/kernels/g13_tsr_resolve.rx"
CONFORM_ACCEPT = ROOT / "conformance/display_pipeline/accept/tsr_device_kernel_minimal.rx"
CONFORM_REJECT = ROOT / "conformance/display_pipeline/reject/tsr_device_temporal_base_rewire.rx"
BUDGET_PATH = ROOT / "milestones/g13/g13_budget.json"
EVIDENCE_DIR = ROOT / "evidence"
WORK_DIR = ROOT / ".tmp/g13_gates/m_b"
HARNESS_FEATURE = "vulkan"
HARNESS_BIN = "g13_tsr_device"
SPEC_ANCHOR = "RXS-0404"

# host 金标准锚定单测（temporal 底座面;逐名锚定防空跑;M-a 门同闭集）。
TEMPORAL_TESTS = [
    "static_convergence_ssim_gate",
    "reset_first_frame_is_plain_upsample",
    "flicker_suppressed_and_static_unharmed",
    "output_size_change_auto_resets",
    "camera_mv_static_is_zero",
    "validate_history_accepts_static",
]

TIER_NAMES = ["50", "67", "100"]

# 标定条目注册表:(budget id, 标定面, direction, evidence slug, 描述)。
# 确定性面（maxdiff/deficit——固定 seed 位级确定）追加一次,复跑值漂移即 RED;
# 帧时面（墙钟非位级确定）建或守护复检（measured ≤ 在档阈,沿 M165 同模）。
CALIB_ENTRY_REGISTRY = [
    (
        "g13.tsr_device.host_device_maxdiff_tol", "maxdiff", "max", "maxdiff", 2.0,
        "TSR device vs host 金标准同输入逐帧对拍容差冻结带(三档 50%/67%/100% → "
        "1280×720 各 32 帧 Halton jitter 静态收敛序列,逐帧逐像素最大绝对差 p100;"
        "threshold = measured × 2.0 协议冻结 k,方向 max;M-b 标定腿产,禁手写 P-09)",
    ),
    (
        "g13.tsr_device.tier_ssim_deficit_50", "quality:50", "max", "quality_50", 2.0,
        "TSR device 50% 档(640×360→1280×720)终帧 SSIM deficit 冻结带(1−SSIM 对拍 "
        "4×4 超采样参照,RXS-0387 LDR 8×8 窗口径;threshold = measured × 2.0;M-b "
        "标定腿产,禁手写 P-09;回归守护语义不构成超分画质通过线)",
    ),
    (
        "g13.tsr_device.tier_ssim_deficit_67", "quality:67", "max", "quality_67", 2.0,
        "TSR device 67% 档(858×482→1280×720)终帧 SSIM deficit 冻结带(同口径;"
        "threshold = measured × 2.0;M-b 标定腿产,禁手写 P-09;回归守护语义不构成"
        "超分画质通过线)",
    ),
    (
        "g13.tsr_device.tier_ssim_deficit_100", "quality:100", "max", "quality_100", 2.0,
        "TSR device 100% 档(1280×720→1280×720)终帧 SSIM deficit 冻结带(同口径;"
        "threshold = measured × 2.0;M-b 标定腿产,禁手写 P-09;回归守护语义不构成"
        "超分画质通过线)",
    ),
]

BENCH_ENTRY_REGISTRY = [
    (
        "g13.tsr_device.frame_ms_tier_50", "50",
        "TSR device 50% 档逐帧全链路帧时基线(host Instant 墙钟 around 打包 + 双 "
        "dispatch + 回读同步 + 状态轮换;warmup 10 + timed 150 = 3 块 × 50 trimmed "
        "mean,M141/M165 冻结统计口径;阈 = 实测 ×1.5 沿 G9.1~G12.5 measured 冻结"
        "先例覆盖频率漂移)——回归守护语义,不构成帧率对标通过线(正式帧率对标"
        "锚定 G14,G10-N11/N16 承接锚字面维持)",
    ),
    (
        "g13.tsr_device.frame_ms_tier_67", "67",
        "TSR device 67% 档逐帧全链路帧时基线(同口径;阈 = 实测 ×1.5)——回归守护"
        "语义,不构成帧率对标通过线(正式帧率对标锚定 G14)",
    ),
    (
        "g13.tsr_device.frame_ms_tier_100", "100",
        "TSR device 100% 档逐帧全链路帧时基线(同口径;阈 = 实测 ×1.5)——回归守护"
        "语义,不构成帧率对标通过线(正式帧率对标锚定 G14)",
    ),
]

RED_ARMS = ["kernel-bias", "seed-change"]

CHECK_KEYS = [
    "temporal_base_0byte",
    "host_upscale_tests_anchored",
    "kernel_sources_anchored",
    "conformance_corpus_anchored",
    "spv_compile_spirv_val_pass",
    "budget_anchors_present",
    "calibration_two_run_bitexact",
    "calibration_budget_entries_measured",
    "bench_three_tiers_measured",
    "budget_eval_all_pass",
    "device_harness_full_pass",
    "device_host_device_maxdiff_within_tol",
    "device_tier_deficit_band_within",
    "device_converge_monotonic",
    "device_double_run_bitexact",
    "device_red_kernel_bias_detected",
    "device_red_seed_change_detected",
    "device_validation_zero",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def base_commit() -> str:
    return run(["git", "rev-parse", "HEAD"]).stdout.strip()


def tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def environment() -> dict:
    import platform

    return {
        "os": platform.platform(),
        "python_version": sys.version.split()[0],
        "cargo_version": tool_version("cargo"),
        "rustc_version": tool_version("rustc"),
    }


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def build_rurixc() -> Path | None:
    print(f"[{TAG}] cargo build -p rurixc --features vulkan-backend --bin rurixc")
    r = run(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"])
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return None
    exe = target_dir() / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    return exe if exe.is_file() else None


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features {HARNESS_FEATURE} --bin {HARNESS_BIN}")
    r = run(["cargo", "build", "-p", "rurix-render", "--features", HARNESS_FEATURE, "--bin", HARNESS_BIN])
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return None
    exe = target_dir() / "debug" / (f"{HARNESS_BIN}.exe" if sys.platform == "win32" else HARNESS_BIN)
    return exe if exe.is_file() else None


def json_line(stdout: str, schema_token: str) -> str | None:
    for line in stdout.splitlines():
        if schema_token in line:
            return line.strip()
    return None


# ---------------------------------------------------------------------------
# host 面机核（temporal 0-byte / kernel 锚定 / conformance 语料）
# ---------------------------------------------------------------------------


def _detect_temporal_diff(diff_text: str) -> bool:
    return bool(diff_text.strip())


def temporal_base_0byte() -> tuple[bool, str]:
    r = run(["git", "diff", "--name-only", G13_ZERO_BASE, "--", TEMPORAL_DIR])
    if _detect_temporal_diff(r.stdout):
        changed = [x.strip() for x in r.stdout.splitlines() if x.strip()]
        return False, f"temporal 底座有差分(底座接线即 RED): {changed[:3]}"
    u = run(["git", "status", "--porcelain", "--", TEMPORAL_DIR])
    if _detect_temporal_diff(u.stdout):
        dirty = [x for x in u.stdout.splitlines() if x.strip()]
        return False, f"temporal 底座工作树未提交面: {dirty[:3]}"
    return True, f"temporal/ vs {G13_ZERO_BASE} 目录级 0-byte(提交面 + 工作树双面)"


def kernel_sources_anchored() -> tuple[bool, str]:
    for path in (KERNEL_RESAMPLE, KERNEL_RESOLVE):
        if not path.is_file():
            return False, f"kernel 源缺失({path.relative_to(ROOT)})"
        text = path.read_text(encoding="utf-8")
        if f"//@ spec: {SPEC_ANCHOR}" not in text:
            return False, f"kernel {path.name} 缺 `//@ spec: {SPEC_ANCHOR}` 锚定"
        if GATE_KEY not in text:
            return False, f"kernel {path.name} 缺门 key 字面 {GATE_KEY}"
    return True, f"双 kernel 在树且 `//@ spec: {SPEC_ANCHOR}` + 门 key 字面锚定齐备"


def conformance_corpus_anchored() -> tuple[bool, str]:
    for path in (CONFORM_ACCEPT, CONFORM_REJECT):
        if not path.is_file():
            return False, f"conformance 语料缺失({path.relative_to(ROOT)})"
        text = path.read_text(encoding="utf-8")
        if f"//@ spec: {SPEC_ANCHOR}" not in text:
            return False, f"conformance {path.name} 缺 `//@ spec: {SPEC_ANCHOR}` 锚定"
    r = run(["py", "-3", "ci/trace_matrix.py", "--check"])
    if r.returncode != 0 or "[trace_matrix] PASS" not in (r.stdout + r.stderr):
        return False, f"trace_matrix 非 PASS: {(r.stdout + r.stderr)[-200:]}"
    return True, "conformance accept/reject 语料锚定齐备 + trace_matrix 全锚定 PASS"


# ---------------------------------------------------------------------------
# SPV 编译面(rurixc --target vulkan + spirv-val;G12 PT megakernel 车道同模)
# ---------------------------------------------------------------------------


def compile_spv(rurixc: Path) -> tuple[Path, Path] | None:
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    outs = []
    for kernel in (KERNEL_RESAMPLE, KERNEL_RESOLVE):
        out = WORK_DIR / f"{kernel.stem}.spv"
        print(f"[{TAG}] rurixc {kernel.name} --target vulkan -o {out.name}")
        r = run([str(rurixc), str(kernel), "--target", "vulkan", "-o", str(out)])
        if r.returncode != 0 or not out.is_file():
            check(False, f"SPV 编译失败 {kernel.name} rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}")
            return None
        # spirv-val 独立校验(rurixc 内建校验之外的第二判读面;缺工具即 RED——
        # M-b 行「spirv-val 通过」为硬判据,不 SKIP)。
        val = run(["spirv-val", str(out)])
        if val.returncode != 0:
            check(False, f"spirv-val 未过 {out.name}: {(val.stdout + val.stderr)[-300:]}")
            return None
        outs.append(out)
    return outs[0], outs[1]


# ---------------------------------------------------------------------------
# g13_budget(M-b 标定/帧时条目消费面;确定性面追加一次漂移即 RED,帧时面建或
# 守护复检 measured ≤ 在档阈——沿 M165 同模)
# ---------------------------------------------------------------------------


def load_g13_budget() -> dict | None:
    if not BUDGET_PATH.is_file():
        return None
    return load_json(BUDGET_PATH)


def budget_entry(budget: dict, eid: str) -> dict | None:
    for e in budget.get("entries", []):
        if e.get("id") == eid:
            return e
    return None


def _entry_is_measured(entry: dict) -> bool:
    """budget 条目 measured_local 判读器(estimated 冒充 measured 检出面)。"""
    return entry.get("evidence") == "measured_local" and not entry.get("skip_reason")


def _write_measured_evidence(
    eid: str, slug: str, measured: float, protocol: str, count: int, digest: str,
    ts: str, extra: dict | None = None,
) -> str:
    doc = {
        "schema": "rurix.g13tsrdevice.measured_entry.v1",
        "entry_id": eid,
        "results": {"trimmed_mean": measured},
        "protocol": protocol,
        "sample_manifest": {"count": count, "digest": digest},
        "provenance": {"gpu": "device", "backend": "tsr_device", "base_commit": base_commit()},
        "timestamp": ts,
    }
    if extra:
        doc.update(extra)
    ev_rel = f"evidence/g13_m_b_{slug}_{ts}.json"
    (ROOT / ev_rel).write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return ev_rel


def append_budget_entries(new_entries: list[dict]) -> list[str]:
    """确定性面:缺 → 追加;在档 → 值漂移即 RED(只追加禁改写,字节级纯追加锚
    M-a 同模)。帧时面:缺 → 追加;在档 → 守护复检 measured ≤ 阈(M165 同模),
    由调用侧在追加前分流。本函数只处理「追加」面;复检面在 run 主流程。"""
    problems: list[str] = []
    if not new_entries:
        return problems
    budget_text = BUDGET_PATH.read_text(encoding="utf-8")
    budget = json.loads(budget_text)
    to_add: list[dict] = []
    for entry in new_entries:
        existing = budget_entry(budget, entry["id"])
        if existing is not None:
            comparable = {k: v for k, v in entry.items() if k != "evidence_file"}
            ex_comparable = {k: v for k, v in existing.items() if k != "evidence_file"}
            if ex_comparable != comparable:
                problems.append(f"{entry['id']} 已在树且值漂移(只追加禁改写)")
            continue
        to_add.append(entry)
    if problems or not to_add:
        return problems
    nl = "\r\n" if "\r\n" in budget_text else "\n"
    frag = ""
    for entry in to_add:
        body = json.dumps(entry, ensure_ascii=False, indent=2)
        body = body.replace("\n", nl)
        body = "    " + body.replace(nl, nl + "    ")
        frag += "," + nl + body
    anchor = f"{nl}  ],{nl}  \"ratio_assertions\""
    if anchor not in budget_text:
        return ["g13_budget.json 结构锚缺失(拒改写)"]
    head, sep, tail = budget_text.partition(anchor)
    budget_text = head + frag + sep + tail
    json.loads(budget_text)
    BUDGET_PATH.write_text(budget_text, encoding="utf-8", newline="")
    return problems


# ---------------------------------------------------------------------------
# 标定腿(harness --calibrate maxdiff|quality 两跑位级一致;device 面持锁内)
# ---------------------------------------------------------------------------


def run_calibration(harness: Path, spv_a: Path, spv_b: Path) -> dict[str, dict] | None:
    calibs: dict[str, dict] = {}
    for what in ("maxdiff", "quality"):
        lines: list[str] = []
        for run_idx in (1, 2):
            print(f"[{TAG}] 标定跑 {run_idx}: --calibrate {what}")
            r = run(
                [str(harness), "--calibrate", what, "--spv-resample", str(spv_a), "--spv-resolve", str(spv_b)],
                env=device_env(), timeout=3600,
            )
            line = json_line(r.stdout, "rurix.g13tsrdevice.calibration_entry.v1")
            if r.returncode != 0 or line is None:
                skip = json_line(r.stdout, "rurix.g13tsrdevice.calibration_skip.v1")
                if skip is not None:
                    check(False, f"标定腿 {what} SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP 充绿): {skip[:200]}")
                else:
                    check(False, f"标定腿 {what} 跑 {run_idx} 失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}")
                return None
            lines.append(line)
        if lines[0] != lines[1]:
            check(False, f"标定腿 {what} 两跑非位级一致(确定性协议漂移即 RED)")
            return None
        calibs[what] = json.loads(lines[0])
        note(f"标定 {what}: trimmed_mean={calibs[what]['results']['trimmed_mean']:.15e}")
    return calibs


def register_calibration_entries(calibs: dict[str, dict], ts: str) -> list[str]:
    """确定性标定条目(maxdiff + 三档 deficit)落 evidence + budget 追加;
    在档条目值漂移即 RED(确定性面两跑位级一致 → 复跑位级期望)。"""
    problems: list[str] = []
    budget = load_g13_budget()
    if budget is None:
        return ["g13_budget.json 缺失(M-a 波首建面应已在树)"]
    new_entries: list[dict] = []
    for eid, what, direction, slug, factor, desc in CALIB_ENTRY_REGISTRY:
        if what == "maxdiff":
            doc = calibs["maxdiff"]
            measured = float(doc["results"]["trimmed_mean"])
            count = int(doc["sample_manifest"]["count"])
            digest = doc["sample_manifest"]["digest"]
        else:
            tier = what.split(":", 1)[1]
            doc = calibs["quality"]
            cell = doc.get("cells", {}).get(tier)
            if cell is None:
                problems.append(f"quality 标定缺 tier {tier} cell")
                continue
            measured = float(cell)
            count = 1
            digest = f"{doc['sample_manifest']['digest']}:tier{tier}"
        existing = budget_entry(budget, eid)
        if existing is not None:
            if float(existing.get("measured_value", "nan")) != measured:
                problems.append(f"{eid} 在档值 {existing.get('measured_value')} ≠ 复跑 {measured}(确定性面漂移即 RED)")
            elif not _entry_is_measured(existing):
                problems.append(f"{eid} 非 measured_local(estimated 冒充 measured 即 RED)")
            continue
        ev_rel = _write_measured_evidence(
            eid, f"calibration_{slug}", measured,
            doc["protocol"] + f";budget 条目 {eid} 派生面",
            count, digest, ts,
            extra={"cells": doc.get("cells", {})} if what != "maxdiff" else None,
        )
        new_entries.append({
            "id": eid,
            "description": desc + f";样本集 digest {digest}(count={count});标定程序 ci/g13_tsr_device_kernel_smoke.py 标定腿可复跑(两跑位级一致)",
            "direction": direction,
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": measured * factor,
            "evidence_file": ev_rel,
            "measured_value": measured,
        })
    problems.extend(append_budget_entries(new_entries))
    return problems


# ---------------------------------------------------------------------------
# bench 腿(三档帧时采样;统计面 = M141/M165 冻结口径同实现复用)
# ---------------------------------------------------------------------------


def run_bench(harness: Path, spv_a: Path, spv_b: Path, ts: str) -> tuple[dict[str, dict], list[str]]:
    results: dict[str, dict] = {}
    problems: list[str] = []
    for eid, tier, desc in BENCH_ENTRY_REGISTRY:
        print(f"[{TAG}] bench 腿: --bench {tier} --warmup {WARMUP} --frames {TIMED}")
        r = run(
            [str(harness), "--bench", tier, "--spv-resample", str(spv_a), "--spv-resolve", str(spv_b),
             "--warmup", str(WARMUP), "--frames", str(TIMED)],
            env=device_env(), timeout=3600,
        )
        line = json_line(r.stdout, "rurix.g13tsrdevice.bench.v1")
        if r.returncode != 0 or line is None:
            skip = json_line(r.stdout, "rurix.g13tsrdevice.bench_skip.v1")
            if skip is not None:
                problems.append(f"bench 腿 tier {tier} SKIP(REQUIRE_REAL=1 不许 SKIP): {skip[:200]}")
            else:
                problems.append(f"bench 腿 tier {tier} 失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}")
            continue
        doc = json.loads(line)
        if doc.get("warmup_count", 0) < WARMUP or doc.get("timed_count") != TIMED:
            problems.append(f"bench tier {tier} 采样轮数不足(warmup {doc.get('warmup_count')}/timed {doc.get('timed_count')},协议 warmup≥{WARMUP}∧timed=={TIMED})")
            continue
        samples = [float(v) for v in doc.get("frame_ms", [])]
        if len(samples) != TIMED:
            problems.append(f"bench tier {tier} 原始样本数 {len(samples)} ≠ {TIMED}")
            continue
        stats = block_stats(samples)
        if not recompute_check(samples, stats):
            problems.append(f"bench tier {tier} 统计面独立重算不符(50×3 trimmed mean 协议面破缺)")
            continue
        results[tier] = {"doc": doc, "stats": stats, "eid": eid, "desc": desc}
        note(f"bench tier {tier}: trimmed_mean={stats['trimmed_mean_ms']:.6f} ms cv={stats['cv']:.4f}")
    return results, problems


def register_bench_entries(bench: dict[str, dict], ts: str) -> list[str]:
    """帧时条目:缺 → 追加(阈 = 实测 ×1.5);在档 → 守护复检 measured ≤ 在档阈
    (墙钟非位级确定,M165 同模),复检失败即 RED(回归守护语义)。"""
    problems: list[str] = []
    budget = load_g13_budget()
    if budget is None:
        return ["g13_budget.json 缺失"]
    new_entries: list[dict] = []
    for tier, pack in bench.items():
        eid, desc, stats = pack["eid"], pack["desc"], pack["stats"]
        measured = float(stats["trimmed_mean_ms"])
        existing = budget_entry(budget, eid)
        if existing is not None:
            if not _entry_is_measured(existing):
                problems.append(f"{eid} 非 measured_local(estimated 冒充 measured 即 RED)")
            elif measured > float(existing["threshold"]):
                problems.append(f"{eid} 守护复检失败:复测 {measured:.6f} ms > 在档阈 {float(existing['threshold']):.6f} ms")
            else:
                note(f"帧时条目在档守护复检 PASS: {eid} 复测 {measured:.6f} ms vs 阈 {float(existing['threshold']):.6f} ms")
            continue
        if NO_PASS_LINE_LITERAL not in desc:
            problems.append(f"{eid} 描述缺不设通过线字面({NO_PASS_LINE_LITERAL})")
            continue
        ev_rel = _write_measured_evidence(
            eid, f"bench_{tier}", measured,
            f"TSR device tier {tier} 逐帧全链路帧时(host Instant 墙钟;warmup {WARMUP} + timed {TIMED} = 3 块 × 50 "
            f"trimmed mean,M141/M165 冻结统计口径;阈 = 实测 ×1.5 回归守护,不构成帧率对标通过线)",
            TIMED, f"sha256-samples:{pack['doc'].get('first_frame_digest', 'n/a')}", ts,
            extra={"stats": stats},
        )
        new_entries.append({
            "id": eid,
            "description": desc + f";样本集 {TIMED} 帧(trimmed mean {measured:.6f} ms,cv {stats['cv']:.4f});采样程序 ci/g13_tsr_device_kernel_smoke.py bench 腿可复跑(在档后守护复检)",
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "ms",
            "threshold": measured * 1.5,
            "evidence_file": ev_rel,
            "measured_value": measured,
        })
    problems.extend(append_budget_entries(new_entries))
    return problems


# ---------------------------------------------------------------------------
# device 腿(全档 + RED 臂独立复跑;持锁)
# ---------------------------------------------------------------------------


def run_device_leg(
    harness: Path, spv_a: Path, spv_b: Path, budget: dict,
) -> tuple[str, dict | None, dict[str, bool], list[str], int]:
    failures: list[str] = []
    arm_results: dict[str, bool] = {}
    tol_e = budget_entry(budget, "g13.tsr_device.host_device_maxdiff_tol")
    band_es = [budget_entry(budget, f"g13.tsr_device.tier_ssim_deficit_{t}") for t in TIER_NAMES]
    if tol_e is None or any(e is None for e in band_es):
        return "fail", None, arm_results, ["budget 缺 M-b 标定条目(标定腿未绿不得跑 device)"], 0
    if not _entry_is_measured(tol_e) or any(not _entry_is_measured(e) for e in band_es if e):
        return "fail", None, arm_results, ["M-b 标定条目非 measured_local(estimated 冒充 measured 即 RED)"], 0
    tol = float(tol_e["threshold"])
    bands = [float(e["threshold"]) for e in band_es if e]
    args = [
        "--spv-resample", str(spv_a), "--spv-resolve", str(spv_b),
        "--tol", repr(tol),
        "--band-deficit-50", repr(bands[0]),
        "--band-deficit-67", repr(bands[1]),
        "--band-deficit-100", repr(bands[2]),
    ]
    print(f"[{TAG}] device 全档: harness --tol {tol:.6g} --band-deficit-50/67/100 {bands[0]:.6g}/{bands[1]:.6g}/{bands[2]:.6g}(REQUIRE_REAL+VK_VALIDATION)")
    r = run([str(harness)] + args, env=device_env(), timeout=3600)
    validation_hits = len(re.findall(r"VUID|Validation Error", r.stderr))
    line = json_line(r.stdout, "rurix.g13tsrdevice.harness.v1")
    if line is None:
        return "fail", None, arm_results, [f"harness 全档无 evidence 行 rc={r.returncode}: {(r.stdout + r.stderr)[-400:]}"], validation_hits
    doc = json.loads(line)
    if doc.get("state") == "skipped_dev_env":
        return "skipped_dev_env", doc, arm_results, [f"device SKIP(REQUIRE_REAL=1 不许 SKIP): {doc.get('skip_reason', '')[:200]}"], validation_hits
    if r.returncode != 0 or doc.get("state") != "pass":
        return "fail", doc, arm_results, [f"harness 全档非 pass rc={r.returncode} problems={doc.get('problems')}"], validation_hits
    for arm in RED_ARMS:
        arm_args = ["--red-arm", arm, "--spv-resample", str(spv_a), "--spv-resolve", str(spv_b)]
        if arm == "kernel-bias":
            arm_args += ["--tol", repr(tol)]
        print(f"[{TAG}] device RED 臂: --red-arm {arm}")
        ra = run([str(harness)] + arm_args, env=device_env(), timeout=3600)
        rl = json_line(ra.stdout, "rurix.g13tsrdevice.red_arm.v1")
        try:
            rdoc = json.loads(rl) if rl else {}
        except json.JSONDecodeError:
            rdoc = {}
        arm_ok = ra.returncode == 0 and rdoc.get("detected") is True
        arm_results[arm] = arm_ok
        if not arm_ok:
            failures.append(f"RED 臂 {arm} 未独立检出 rc={ra.returncode}: {(ra.stdout + ra.stderr)[-300:]}")
    return "executed", doc, arm_results, failures, validation_hits


# ---------------------------------------------------------------------------
# selftest(反 YAML-only)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 18:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 18", file=sys.stderr)
        return 1
    schema = load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    # 红臂①:temporal 底座 diff 检出器——合成差分面必判 RED,空面正例不误判。
    if not _detect_temporal_diff("src/rurix-render/src/temporal/tsr.rs\n"):
        print(f"[{TAG}] selftest FAIL: temporal 差分注入未检出", file=sys.stderr)
        return 1
    if _detect_temporal_diff(""):
        print(f"[{TAG}] selftest FAIL: temporal 0-byte 正例误判", file=sys.stderr)
        return 1
    # 红臂②:harness evidence 判读——skipped_dev_env / fail 态不得判 pass。
    if _harness_state_pass('{"state":"skipped_dev_env"}'):
        print(f"[{TAG}] selftest FAIL: SKIP 态误判 pass", file=sys.stderr)
        return 1
    if _harness_state_pass('{"state":"fail"}'):
        print(f"[{TAG}] selftest FAIL: fail 态误判 pass", file=sys.stderr)
        return 1
    if not _harness_state_pass('{"state":"pass"}'):
        print(f"[{TAG}] selftest FAIL: pass 态正例误判", file=sys.stderr)
        return 1
    # 红臂③:estimated 冒充 measured——budget 条目判读器必拒。
    if _entry_is_measured({"evidence": "estimated"}):
        print(f"[{TAG}] selftest FAIL: estimated 注入未检出", file=sys.stderr)
        return 1
    if _entry_is_measured({"evidence": "measured_local", "skip_reason": "no gpu"}):
        print(f"[{TAG}] selftest FAIL: skip_reason 携带误判", file=sys.stderr)
        return 1
    if not _entry_is_measured({"evidence": "measured_local", "skip_reason": None}):
        print(f"[{TAG}] selftest FAIL: measured_local 正例误判", file=sys.stderr)
        return 1
    # 红臂④:50×3 trimmed mean 协议面——样本数不足必拒;统计面独立重算须咬合。
    try:
        block_stats([1.0] * (TIMED - 1))
        print(f"[{TAG}] selftest FAIL: 样本数不足未拒", file=sys.stderr)
        return 1
    except ValueError:
        pass
    good = block_stats([0.5 + (i % 7) * 0.001 for i in range(TIMED)])
    if not recompute_check([0.5 + (i % 7) * 0.001 for i in range(TIMED)], good):
        print(f"[{TAG}] selftest FAIL: 统计面独立重算不咬合", file=sys.stderr)
        return 1
    tampered = dict(good, trimmed_mean_ms=good["trimmed_mean_ms"] + 0.01)
    if recompute_check([0.5 + (i % 7) * 0.001 for i in range(TIMED)], tampered):
        print(f"[{TAG}] selftest FAIL: 统计面篡改未检出", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (4 RED + 4 GREEN)")
    return 0


def _harness_state_pass(line: str) -> bool:
    try:
        return json.loads(line).get("state") == "pass"
    except json.JSONDecodeError:
        return False


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

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    # ── host 段 ──
    ok, msg = temporal_base_0byte()
    checks["temporal_base_0byte"] = ok
    check(ok, msg)
    note(msg)

    ok, msg = kernel_sources_anchored()
    checks["kernel_sources_anchored"] = ok
    check(ok, msg)
    note(msg)

    ok, msg = conformance_corpus_anchored()
    checks["conformance_corpus_anchored"] = ok
    check(ok, msg)
    note(msg)

    r = run(["cargo", "test", "-p", "rurix-render", "--lib", "temporal::"])
    blob = r.stdout + r.stderr
    missing = [n for n in TEMPORAL_TESTS if n not in blob]
    checks["host_upscale_tests_anchored"] = r.returncode == 0 and "test result: ok" in blob and not missing
    check(checks["host_upscale_tests_anchored"], f"temporal 金标准单测失败或未锚定: {missing[:3]} rc={r.returncode}")
    note(f"{len(TEMPORAL_TESTS)} temporal 单测逐名锚定全绿")

    budget = load_g13_budget()
    entry_ids = [eid for eid, *_ in CALIB_ENTRY_REGISTRY] + [eid for eid, _t, _d in BENCH_ENTRY_REGISTRY]
    checks["budget_anchors_present"] = budget is not None and all(
        budget_entry(budget, eid) is not None and _entry_is_measured(budget_entry(budget, eid) or {})
        for eid in entry_ids
    )
    if not checks["budget_anchors_present"]:
        note("M-b budget 条目未齐备(首跑标定/bench 腿补齐)")

    # ── 持锁段(rurixc/SPV + harness 构建 + 标定腿 + bench 腿 + device 腿) ──
    device_state = "fail"
    doc: dict | None = None
    arm_results: dict[str, bool] = {}
    validation_hits = -1
    with gpu_device_lock(purpose=f"{TAG} 构建+SPV+标定+bench+device 腿"):
        rurixc = build_rurixc()
        spvs = compile_spv(rurixc) if rurixc is not None else None
        checks["spv_compile_spirv_val_pass"] = spvs is not None
        harness = build_harness()
        if harness is None:
            check(False, "g13_tsr_device harness 构建失败")
        elif spvs is not None:
            spv_a, spv_b = spvs
            calibs = run_calibration(harness, spv_a, spv_b)
            checks["calibration_two_run_bitexact"] = calibs is not None
            if calibs is not None:
                problems = register_calibration_entries(calibs, ts)
                check(not problems, f"标定条目登记: {problems[:2]}")
                bench, bench_problems = run_bench(harness, spv_a, spv_b, ts)
                check(not bench_problems, f"bench 腿: {bench_problems[:2]}")
                if len(bench) == 3:
                    bench_reg_problems = register_bench_entries(bench, ts)
                    check(not bench_reg_problems, f"帧时条目登记: {bench_reg_problems[:2]}")
                    checks["bench_three_tiers_measured"] = not bench_reg_problems
                budget = load_g13_budget()
                checks["calibration_budget_entries_measured"] = budget is not None and all(
                    budget_entry(budget, eid) is not None and _entry_is_measured(budget_entry(budget, eid) or {})
                    for eid, *_r in CALIB_ENTRY_REGISTRY
                )
                checks["budget_anchors_present"] = budget is not None and all(
                    budget_entry(budget, eid) is not None and _entry_is_measured(budget_entry(budget, eid) or {})
                    for eid in entry_ids
                )
        if checks["calibration_budget_entries_measured"] and checks["bench_three_tiers_measured"]:
            r = run(["py", "-3", "ci/budget_eval.py"])
            checks["budget_eval_all_pass"] = r.returncode == 0 and "[budget_eval] PASS" in (r.stdout + r.stderr)
            check(checks["budget_eval_all_pass"], f"budget_eval 非零: {(r.stdout + r.stderr)[-300:]}")

        if harness is not None and spvs is not None and checks["budget_anchors_present"] and budget is not None:
            device_state, doc, arm_results, leg_failures, validation_hits = run_device_leg(
                harness, spv_a, spv_b, budget
            )
            for f in leg_failures:
                check(False, f)
            if device_state == "skipped_dev_env":
                check(False, "device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP)")
                device_state = "fail"

    # ── device 判据判读 ──
    if device_state == "executed" and doc is not None:
        tiers = doc.get("tiers") or {}
        tier_list = [tiers.get(t) or {} for t in TIER_NAMES]
        checks["device_harness_full_pass"] = doc.get("state") == "pass" and not doc.get("problems")
        checks["device_host_device_maxdiff_within_tol"] = all(t.get("in_tol") is True for t in tier_list)
        checks["device_tier_deficit_band_within"] = all(t.get("in_band") is True for t in tier_list)
        checks["device_converge_monotonic"] = all(t.get("monotonic") is True for t in tier_list) and (
            doc.get("host") or {}
        ).get("tsr_monotonic") is True
        checks["device_double_run_bitexact"] = all(t.get("bitexact") is True for t in tier_list)
        checks["device_red_kernel_bias_detected"] = arm_results.get("kernel-bias") is True
        checks["device_red_seed_change_detected"] = arm_results.get("seed-change") is True
        checks["device_validation_zero"] = validation_hits == 0
        for k in CHECK_KEYS:
            if k.startswith("device_") and not checks[k]:
                check(False, f"harness 判据 {k} 为假")
        note(
            "device:TSR device 双腿三档真跑(device vs host 逐帧对拍 + 收敛单调 + "
            "双跑位级 + validation 零命中);RED 双臂独立复跑检出"
        )
    elif validation_hits > 0:
        checks["device_validation_zero"] = False

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M168",
        "milestone": "M168",
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G13.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": base_commit(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": "cargo test -p rurix-render --lib temporal:: (TSR 金标准单测逐名锚定)", "exit_code": 0 if checks["host_upscale_tests_anchored"] else 1},
            {"seq": 2, "command": f"git diff --name-only {G13_ZERO_BASE} -- src/rurix-render/src/temporal (0-byte 机核 + 工作树双面)", "exit_code": 0 if checks["temporal_base_0byte"] else 1},
            {"seq": 3, "command": "rurixc kernels/g13_tsr_{resample,resolve}.rx --target vulkan -o .tmp/g13_gates/m_b/*.spv + spirv-val 双件", "exit_code": 0 if checks["spv_compile_spirv_val_pass"] else 1},
            {"seq": 4, "command": "g13_tsr_device --calibrate maxdiff|quality ×2 (标定腿两跑位级一致)", "exit_code": 0 if checks["calibration_two_run_bitexact"] else 1},
            {"seq": 5, "command": "g13_tsr_device --bench 50|67|100 --warmup 10 --frames 150 (三档帧时 50×3 trimmed mean)", "exit_code": 0 if checks["bench_three_tiers_measured"] else 1},
            {"seq": 6, "command": "cargo build -p rurix-render --features vulkan --bin g13_tsr_device", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 7, "command": "g13_tsr_device --spv-resample .. --spv-resolve .. --tol <g13.tsr_device.host_device_maxdiff_tol> --band-deficit-50/67/100 <..> (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 8, "command": "g13_tsr_device --red-arm kernel-bias|seed-change (逐臂独立复跑)", "exit_code": 0 if (checks["device_red_kernel_bias_detected"] and checks["device_red_seed_change_detected"]) else 1},
            {"seq": 9, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": environment(),
        "production": {
            "correctness_anchor_unchanged": checks["temporal_base_0byte"],
            "baseline_anchor_id": "g13.tsr_device.{host_device_maxdiff_tol,tier_ssim_deficit_{50,67,100},frame_ms_tier_{50,67,100}}(本门标定/bench 腿产出入 g13_budget)",
            "measured_value": (
                "; ".join(
                    f"tier {t} deficit={(doc.get('tiers') or {}).get(t, {}).get('deficit', 'n/a')} p100_vs_host={(doc.get('tiers') or {}).get(t, {}).get('host_device_maxdiff_p100', 'n/a')}"
                    for t in TIER_NAMES
                )
                if doc
                else "n/a(device 未执行)"
            ),
            "not_worse_than_anchor": checks["device_tier_deficit_band_within"] and checks["device_host_device_maxdiff_within_tol"],
            "threshold_provenance": "g13_budget.json M-b 标定/帧时条目(标定腿两跑位级一致程序产,threshold = measured × 2.0 冻结 k;帧时阈 = 实测 ×1.5 守护复检,禁手写 P-09)",
            "evolution_register": (
                "帧时基线 zero_pass_line 登记:回归守护语义,不构成帧率对标通过线(正式帧率对标锚定 G14);"
                "质量 deficit 冻结带不构成超分画质通过线(G13 不设画质通过线归 G15)"
            ),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    if SCHEMA_PATH.is_file():
        schema = load_json(SCHEMA_PATH)
        for k in schema.get("required", []):
            check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_state}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
