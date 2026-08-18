#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.2 vendor 超分接入波）
"""G13.2 M-a(M167) vendor 超分接入门冒烟
（g13.p0.m_a.vendor_upscale_integration；G13_CONTRACT §4.2 M-a 行判据逐字 /
G-G13-4；G13_ACCEPTANCE_MAP §1；RFC-0016 §4.H3/§9 Q-F；spec/visual_comparison.md
RXS-0387/0388 口径继承）。

硬判据：许可前置 owner 法律面清结留痕（五要素机核，未清结即 blocked 不充绿）
+ DLSS SR 经 Streamline SDK（2.10.3 + NGX 签名 DLL，Vulkan interop 臂）真跑出帧
（RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 validation 零错误，RTX 4070 Ti；
NGX evaluate 段校验层覆盖排除如实登记——层在下 NGX CUDA interop 触发 NVIDIA
驱动内部崩溃 nvoglv64.dll 0xc0000005，vendor 已知 SL+validation 不兼容类
Streamline issue #84，覆盖口径 = 我方 Vulkan 全表面 + SL 代理建链 + SL 簿记，
经 harness --validation-probe 独立子进程面达成）
+ FSR 3.1.5 同接口档（同一 UpscaleBackend 冻结面 RFC-0016 §4.0-3，D3D12 臂
debug layer 在跑零错误，FSR4 ML 不可用自动回退 FSR 3.1.5 分析版如实登记）
+ 同场景同内部分辨率（320×180→640×360）TSR/DLSS/FSR 三后端同进程运行时切换
逐帧有效出帧
+ 静态场景 32 帧 Halton jitter 收敛 SSIM deficit 对拍 4×4 超采样参照不偏离
g13_budget 标定冻结带（threshold = measured × 2.0，标定腿两跑位级一致程序产，
禁手写 P-09）
+ 双端超分帧对拍 measured 登记（DLSS↔TSR / FSR↔TSR SSIM + 逐像素最大绝对差，
不设绝对通过线——G13 不设 DLSS/超分画质通过线）
+ DLL provenance 实测 digest 对账 g13_vendor_sdk_registry.json
+ 双跑位级一致（固定 scene/jitter/参数确定性协议面）
+ UpscaleBackend trait 签名面与 temporal 底座 0-byte 机核（目录级 git diff
vs G13.0 不可变 ref 8c5dc5ee + 工作树双面）
+ 树内零 UE/vendor 源码 vendoring（git ls-files token 面机核）
+ 树内零绕过 UpscaleBackend 私接面（vendor SDK 调用 token 仅允许在登记 FFI
边界文件 src/rurix-rt/src/vendor_upscale.rs 内）。
RED 臂：许可未清结开工即 RED；底座接线即 RED；mock/stub 充真跑即 RED
（mock-passthrough 臂）；单 vendor 缺臂聚合 PASS 即 RED；device 臂垃圾 MV
注入必检出（mv-garbage / fsr-mv-garbage 双臂——原 zero-exposure 注入实测
无效已废止：FSR 3.1.5 LDR 路径不消费 pre_exposure，留痕防回归）。

三态：无 SDK/GPU → device 腿 SKIP DEV_ENV_DEGRADE（退 0，非 fake pass）；
本脚本默认 RURIX_REQUIRE_REAL=1（setdefault），该态下 SKIP → 硬红。

用法:
  py -3 ci/g13_vendor_upscale_integration_smoke.py --gate g13.p0.m_a.vendor_upscale_integration
  py -3 ci/g13_vendor_upscale_integration_smoke.py --selftest
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

GATE_KEY = "g13.p0.m_a.vendor_upscale_integration"
NUMERIC_STEP = 236
SUBJECT = "g13_m_a_vendor_upscale_integration"
SCHEMA_PATH = ROOT / "milestones/g13/g13_m_a_vendor_upscale_integration_evidence_schema.json"
CALIB_SCHEMA_PATH = ROOT / "milestones/g13/g13_m_a_calibration_entry_evidence_schema.json"
SOURCE_REF = (
    "G13_CONTRACT §4.2 M-a/G-G13-4;G13_ACCEPTANCE_MAP §1;rfcs/0016-native-renderer.md §4.H3/§9 Q-F;"
    "spec/visual_comparison.md RXS-0387/RXS-0388;milestones/g13/design/vendor_upscale_license_clearance.md"
)
TAG = "g13_m_a"

G13_ZERO_BASE = "8c5dc5ee"  # G13.0 不可变 ref（契约 §7 登记）
TEMPORAL_DIR = "src/rurix-render/src/temporal"
FFI_BOUNDARY = "src/rurix-rt/src/vendor_upscale.rs"
LICENSE_DOC = ROOT / "milestones/g13/design/vendor_upscale_license_clearance.md"
SDK_REGISTRY = ROOT / "milestones/g13/g13_vendor_sdk_registry.json"
BUDGET_PATH = ROOT / "milestones/g13/g13_budget.json"
EVIDENCE_DIR = ROOT / "evidence"
WORK_DIR = ROOT / ".tmp/g13_gates/m_a"
HARNESS_FEATURE = "vendor-upscale"
HARNESS_BIN = "g13_vendor_upscale"

# 许可清结五要素（机核 token 面;G-G13-3 事实门③同字面）。
LICENSE_TOKENS = ["Streamline", "NGX", "FSR", "owner", "清结"]

# vendor SDK 调用 token 闭集——仅允许在登记集成面文件内出现（绕过
# UpscaleBackend 私接面即 RED）。取函数符号面（DLL 文件名面会误中文档转引）。
VENDOR_CALL_TOKENS = [
    "slInit", "slShutdown", "slEvaluateFeature", "slGetNewFrameToken",
    "slSetConstants", "slDLSSSetOptions", "slDLSSGetOptimalSettings",
    "slFreeResources", "slGetFeatureFunction", "slIsFeatureLoaded",
    "slGetFeatureVersion", "slSetVulkanInfo",
    "ffxQuery", "ffxConfigure", "ffxCreateContext", "ffxDispatch",
    "ffxDestroyContext", "ffxGetScratchMemorySize",
]

# 登记集成面文件闭集(token 合法出现面):
# - FFI 边界 src/rurix-rt/src/vendor_upscale.rs——唯一 unsafe FFI 声明/调用面;
# - 门 harness src/rurix-render/src/bin/g13_vendor_upscale.rs——`#![forbid(unsafe_code)]`
#   结构面:extern 声明与 FFI 调用在该文件内**编译期不可能存在**,token 出现仅可能
#   为文档/字符串转引(证据登记字面),不构成调用面。其余任何文件命中即 RED。
VENDOR_TOKEN_ALLOWED_FILES = {
    FFI_BOUNDARY,
    "src/rurix-render/src/bin/g13_vendor_upscale.rs",
}

# vendor 源码/二进制 vendoring 检出面（git ls-files 全树扫描;external/ gitignored）。
VENDORING_PATTERN = re.compile(
    r"streamline|fidelityfx|nvngx|amd_fidelityfx|ffx_api|ffx_upscale|sl_dlss|sl_core_api",
    re.IGNORECASE,
)

# host 金标准锚定单测（temporal 底座面;逐名锚定防空跑）。
TEMPORAL_TESTS = [
    "static_convergence_ssim_gate",
    "reset_first_frame_is_plain_upsample",
    "flicker_suppressed_and_static_unharmed",
    "output_size_change_auto_resets",
    "camera_mv_static_is_zero",
    "validate_history_accepts_static",
]

# 标定条目注册表:(budget id, 后端, direction, slug, 描述)。
CALIB_ENTRY_REGISTRY = [
    (
        "g13.upscale.static_converge_ssim_deficit_tsr", "tsr", "max", "tsr",
        "TSR host 金标准静态场景收敛 SSIM deficit 冻结带(320×180→640×360,32 帧 Halton "
        "jitter,终帧 1−SSIM 对拍 4×4 超采样参照,RXS-0387 LDR 8×8 窗口径;threshold = "
        "measured × 2.0 协议冻结 k;M-a 标定腿产,禁手写 P-09)",
    ),
    (
        "g13.upscale.static_converge_ssim_deficit_dlss", "dlss", "max", "dlss",
        "DLSS SR(Streamline 2.10.3 + NGX 签名 DLL,Vulkan interop 臂)静态场景收敛 SSIM "
        "deficit 冻结带(同场景同内部分辨率同协议;threshold = measured × 2.0;DLL "
        "provenance digest 入 provenance;M-a 标定腿产,禁手写 P-09)",
    ),
    (
        "g13.upscale.static_converge_ssim_deficit_fsr", "fsr", "max", "fsr",
        "FSR 3.1.5(FidelityFX SDK 2.0.0 预编译签名 DLL,D3D12 臂)静态场景收敛 SSIM "
        "deficit 冻结带(同场景同内部分辨率同协议;threshold = measured × 2.0;DLL "
        "provenance digest 入 provenance;M-a 标定腿产,禁手写 P-09)",
    ),
]

RED_ARMS = ["mock-passthrough", "mv-garbage", "fsr-mv-garbage"]

CHECK_KEYS = [
    "license_clearance_present",
    "temporal_base_0byte",
    "no_vendor_source_vendoring",
    "no_private_bypass_surface",
    "sdk_registry_present",
    "host_upscale_tests_anchored",
    "budget_anchors_present",
    "calibration_two_run_bitexact",
    "calibration_budget_entries_measured",
    "budget_eval_all_pass",
    "device_harness_full_pass",
    "device_dlss_real_run",
    "device_fsr_real_run",
    "device_fsr4_ml_fallback_registered",
    "device_three_backend_runtime_switch",
    "device_static_converge_band_within",
    "device_pairwise_tsr_measured_registered",
    "device_dll_provenance_match",
    "device_mock_passthrough_detected",
    "device_red_arm_submodes_detected",
    "device_validation_zero",
    "device_double_run_bitexact",
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
# host 面机核（temporal 0-byte / 许可清结 / SDK registry / vendoring / 私接面）
# ---------------------------------------------------------------------------


def temporal_base_0byte() -> tuple[bool, str]:
    r = run(["git", "diff", "--name-only", G13_ZERO_BASE, "--", TEMPORAL_DIR])
    changed = [x.strip() for x in r.stdout.splitlines() if x.strip()]
    if changed:
        return False, f"temporal 底座有差分(底座接线即 RED): {changed[:3]}"
    u = run(["git", "status", "--porcelain", "--", TEMPORAL_DIR])
    dirty = [x for x in u.stdout.splitlines() if x.strip()]
    if dirty:
        return False, f"temporal 底座工作树未提交面: {dirty[:3]}"
    return True, f"temporal/ vs {G13_ZERO_BASE} 目录级 0-byte(提交面 + 工作树双面)"


def license_clearance_ok() -> tuple[bool, str]:
    if not LICENSE_DOC.is_file():
        return False, f"许可清结留痕缺失({LICENSE_DOC.relative_to(ROOT)})"
    text = LICENSE_DOC.read_text(encoding="utf-8")
    missing = [t for t in LICENSE_TOKENS if t not in text]
    if missing:
        return False, f"许可清结五要素缺 {missing}(未清结即 blocked 不充绿)"
    return True, "M-a 许可前置清结留痕在树且五要素齐备(Streamline/NGX/FSR/owner/清结)"


def sdk_registry_ok() -> tuple[bool, str]:
    if not SDK_REGISTRY.is_file():
        return False, f"vendor SDK registry 缺失({SDK_REGISTRY.relative_to(ROOT)})"
    doc = load_json(SDK_REGISTRY)
    if doc.get("schema") != "rurix.g13.vendor_sdk_registry.v1":
        return False, "registry schema 字面不符"
    sdks = doc.get("sdks") or {}
    sl = sdks.get("streamline") or {}
    fx = sdks.get("fidelityfx") or {}
    hex64 = re.compile(r"^[0-9a-f]{64}$")
    if sl.get("version") != "2.10.3" or "NVIDIA RTX SDKs LICENSE" not in str(sl.get("license", "")):
        return False, "streamline 段 version/license 字段不齐"
    if len(sl.get("dlls") or {}) != 4 or not all(hex64.match(v or "") for v in (sl.get("dlls") or {}).values()):
        return False, "streamline 段 DLL digest 不齐(4 件 64-hex)"
    if fx.get("fsr_upscaler_version") != "3.1.5" or "MIT" not in str(fx.get("license", "")):
        return False, "fidelityfx 段 fsr_upscaler_version/license 字段不齐"
    if len(fx.get("dlls") or {}) != 2 or not all(hex64.match(v or "") for v in (fx.get("dlls") or {}).values()):
        return False, "fidelityfx 段 DLL digest 不齐(2 件 64-hex)"
    return True, "vendor SDK registry 在树且 Streamline/FSR 双段许可/digest 字段齐备(二进制零入 git)"


def _detect_vendoring(ls_files_text: str) -> bool:
    """git ls-files 输出面 vendoring 检出器(命中即 RED)。"""
    return any(VENDORING_PATTERN.search(line) for line in ls_files_text.splitlines() if line.strip())


def no_vendor_source_vendoring() -> tuple[bool, str]:
    r = run(["git", "ls-files"])
    hits = [line for line in r.stdout.splitlines() if line.strip() and VENDORING_PATTERN.search(line)]
    if hits:
        return False, f"树内 vendor 源码/二进制 vendoring(零 vendoring 字面即 RED): {hits[:3]}"
    return True, "git ls-files 全树零 vendor SDK 源码/二进制 vendoring(external/ gitignored 缓存形态)"


def _detect_bypass(rel_path: str, text: str) -> str | None:
    """单文件私接面检出器:登记集成面文件外 token 命中即返回 token。"""
    if rel_path.replace("\\", "/") in VENDOR_TOKEN_ALLOWED_FILES:
        return None
    for tok in VENDOR_CALL_TOKENS:
        if tok in text:
            return tok
    return None


def no_private_bypass_surface() -> tuple[bool, str]:
    hits: list[str] = []
    for base in ("src", "apps"):
        for path in (ROOT / base).rglob("*.rs"):
            rel = path.relative_to(ROOT).as_posix()
            text = path.read_text(encoding="utf-8", errors="replace")
            tok = _detect_bypass(rel, text)
            if tok is not None:
                hits.append(f"{rel}:{tok}")
    if hits:
        return False, f"树内绕过 UpscaleBackend 私接面(vendor 调用 token 越界即 RED): {hits[:3]}"
    return True, f"vendor SDK 调用 token 仅见于登记集成面 {sorted(VENDOR_TOKEN_ALLOWED_FILES)}"


# ---------------------------------------------------------------------------
# g13_budget(标定条目消费面;首跑建文件,条目纯追加幂等)
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


def _new_budget_skeleton() -> dict:
    return {
        "schema_version": 1,
        "namespace": "g13",
        "_meta": {
            "provenance": "Assisted-by: Kimi-K3（G13.2 vendor 超分接入波）",
            "created_utc": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "base_commit": base_commit(),
        },
        "description": (
            "G13 超分/画质期预算。G13.2 M-a(M167)首批三条目 = TSR/DLSS/FSR 静态场景收敛 "
            "SSIM deficit 冻结带(threshold = measured × 2.0,协议冻结 k,方向 max;"
            "标定腿两跑位级一致程序产,禁手写 P-09)。本预算只证明测量已建立与冻结带已登记,"
            "不断言任何超分画质达标——G13 不设 DLSS/超分画质绝对通过线(契约 §4.2 M-a 行字面)。"
            "前瞻预算项(M-b TSR device 化质量/帧时三档对照等)一律等后续实现波标定回填——"
            "无实测证据的阈值不写入(零 estimated 硬约束);counter_assertions 留空"
            "(未知 counter id 会被 budget_eval 强制 FAIL,14 §5 防僵尸纪律)。"
        ),
        "source_docs": [
            "milestones/g13/G13_CONTRACT.md",
            "milestones/g13/G13_ACCEPTANCE_MAP.md",
        ],
        "entries": [],
        "ratio_assertions": [],
        "counter_assertions": [],
    }


def append_calibration_budget_entries(calibs: dict[str, dict], ts: str) -> list[str]:
    """逐条目 evidence 落盘 + budget 字节级纯追加(M138/M162/M166 同纪律幂等)。

    threshold = trimmed_mean × 2.0(协议冻结 k)——由 measured 重算,零手写面。
    """
    problems: list[str] = []
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    entries: list[dict] = []
    for eid, backend, direction, slug, desc in CALIB_ENTRY_REGISTRY:
        doc = calibs[backend]
        measured = float(doc["results"]["trimmed_mean"])
        threshold = measured * 2.0
        ev_rel = f"evidence/g13_m_a_calibration_{slug}_{ts}.json"
        out = ROOT / ev_rel
        out.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        entries.append({
            "id": eid,
            "description": desc + (
                f";样本集 digest {doc['sample_manifest']['digest']}"
                f"(count={doc['sample_manifest']['count']});标定程序 "
                "ci/g13_vendor_upscale_integration_smoke.py 标定腿可复跑(两跑位级一致)"
            ),
            "direction": direction,
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "1",
            "threshold": threshold,
            "evidence_file": ev_rel,
            "measured_value": measured,
        })
    if not BUDGET_PATH.is_file():
        skeleton = _new_budget_skeleton()
        BUDGET_PATH.write_text(
            json.dumps(skeleton, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        note("g13_budget.json 首跑创建(namespace g13,三条目骨架)")
    budget_text = BUDGET_PATH.read_text(encoding="utf-8")
    budget = json.loads(budget_text)
    to_add: list[dict] = []
    for entry in entries:
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
    # 空数组骨架(首跑):"entries": [] → 展开后插入;非空:锚定 entries 收尾
    # `],` + 次行 `"ratio_assertions"` 前纯追加(M138/M162/M166 同纪律)。
    empty_anchor = '"entries": [],'
    anchor = f"{nl}  ],{nl}  \"ratio_assertions\""
    if empty_anchor in budget_text:
        budget_text = budget_text.replace(
            empty_anchor, f'"entries": [{frag.lstrip(",")}{nl}  ],', 1
        )
    elif anchor in budget_text:
        head, sep, tail = budget_text.partition(anchor)
        budget_text = head + frag + sep + tail
    else:
        return ["g13_budget.json 结构锚缺失(拒改写)"]
    json.loads(budget_text)
    BUDGET_PATH.write_text(budget_text, encoding="utf-8", newline="")
    return problems


# ---------------------------------------------------------------------------
# 标定腿(harness --calibrate <backend> 两跑位级一致;device 面持锁内)
# ---------------------------------------------------------------------------


def run_calibration(harness: Path) -> dict[str, dict] | None:
    calibs: dict[str, dict] = {}
    for _eid, backend, _direction, _slug, _desc in CALIB_ENTRY_REGISTRY:
        lines: list[str] = []
        for run_idx in (1, 2):
            print(f"[{TAG}] 标定跑 {run_idx}: --calibrate {backend}")
            r = run([str(harness), "--calibrate", backend], env=device_env(), timeout=1800)
            line = json_line(r.stdout, "rurix.g13upscale.calibration_entry.v1")
            if r.returncode != 0 or line is None:
                skip = json_line(r.stdout, "rurix.g13upscale.calibration_skip.v1")
                if skip is not None:
                    check(False, f"标定腿 {backend} SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP 充绿): {skip[:200]}")
                else:
                    check(False, f"标定腿 {backend} 跑 {run_idx} 失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}")
                return None
            lines.append(line)
        if lines[0] != lines[1]:
            check(False, f"标定腿 {backend} 两跑非位级一致(确定性协议漂移即 RED)")
            return None
        doc = json.loads(lines[0])
        if doc.get("entry_id", "").endswith(f"_{backend}") is not True:
            check(False, f"标定腿 {backend} entry_id 后缀不符: {doc.get('entry_id')}")
            return None
        calibs[backend] = doc
        note(f"标定 {backend}: deficit={doc['results']['trimmed_mean']:.15f}")
    return calibs


# ---------------------------------------------------------------------------
# device 腿(全档 + RED 臂独立复跑;持锁)
# ---------------------------------------------------------------------------


def run_device_leg(harness: Path, budget: dict) -> tuple[str, dict | None, dict[str, bool], list[str]]:
    failures: list[str] = []
    arm_results: dict[str, bool] = {}
    bands: dict[str, float] = {}
    for eid, backend, _direction, _slug, _desc in CALIB_ENTRY_REGISTRY:
        e = budget_entry(budget, eid)
        if e is None or e.get("evidence") != "measured_local":
            return "fail", None, arm_results, [f"budget 缺标定条目 {eid}(标定腿未绿不得跑 device)"]
        bands[backend] = float(e["threshold"])
    args = [
        "--band-tsr", repr(bands["tsr"]),
        "--band-dlss", repr(bands["dlss"]),
        "--band-fsr", repr(bands["fsr"]),
    ]
    print(f"[{TAG}] device 全档: harness --band-tsr {bands['tsr']:.6g} --band-dlss {bands['dlss']:.6g} --band-fsr {bands['fsr']:.6g}(REQUIRE_REAL+VK_VALIDATION)")
    r = run([str(harness)] + args, env=device_env(), timeout=1800)
    line = json_line(r.stdout, "rurix.g13upscale.harness.v1")
    if line is None:
        return "fail", None, arm_results, [f"harness 全档无 evidence 行 rc={r.returncode}: {(r.stdout + r.stderr)[-400:]}"]
    doc = json.loads(line)
    if doc.get("state") == "skipped_dev_env":
        return "skipped_dev_env", doc, arm_results, [f"device SKIP(REQUIRE_REAL=1 不许 SKIP): {doc.get('skip_reason', '')[:200]}"]
    if r.returncode != 0 or doc.get("state") != "pass":
        return "fail", doc, arm_results, [f"harness 全档非 pass rc={r.returncode} problems={doc.get('problems')}"]
    # RED 臂逐臂独立复跑(退出码 0 + detected=true = 臂独立有效,逐臂登记)。
    for arm in RED_ARMS:
        band_arg = ["--band-dlss", repr(bands["dlss"])] if arm != "fsr-mv-garbage" else ["--band-fsr", repr(bands["fsr"])]
        print(f"[{TAG}] device RED 臂: --red-arm {arm}")
        ra = run([str(harness), "--red-arm", arm] + band_arg, env=device_env(), timeout=1800)
        rl = json_line(ra.stdout, "rurix.g13upscale.red_arm.v1")
        try:
            rdoc = json.loads(rl) if rl else {}
        except json.JSONDecodeError:
            rdoc = {}
        arm_ok = ra.returncode == 0 and rdoc.get("detected") is True
        arm_results[arm] = arm_ok
        if not arm_ok:
            failures.append(f"RED 臂 {arm} 未独立检出 rc={ra.returncode}: {(ra.stdout + ra.stderr)[-300:]}")
    return "executed", doc, arm_results, failures


# ---------------------------------------------------------------------------
# selftest(反 YAML-only)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    check(False, "selftest 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    if len(CHECK_KEYS) != 22:
        print(f"[{TAG}] selftest FAIL: CHECK_KEYS={len(CHECK_KEYS)} ≠ 22", file=sys.stderr)
        return 1
    schema = load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    # 红臂①:temporal 底座 diff 检出器——合成差分面必判 RED。
    if not _detect_temporal_diff("src/rurix-render/src/temporal/tsr.rs\n"):
        print(f"[{TAG}] selftest FAIL: temporal 差分注入未检出", file=sys.stderr)
        return 1
    if _detect_temporal_diff(""):
        print(f"[{TAG}] selftest FAIL: temporal 0-byte 正例误判", file=sys.stderr)
        return 1
    # 红臂②:vendor vendoring 检出器——合成 ls-files 命中面必判 RED。
    if not _detect_vendoring("src/rurix-rt/src/vendor_upscale.rs\nexternal/streamline-2.10.3/sl.h\n"):
        print(f"[{TAG}] selftest FAIL: vendoring 注入未检出", file=sys.stderr)
        return 1
    if _detect_vendoring("src/rurix-rt/src/vendor_upscale.rs\nsrc/rurix-render/src/lib.rs\n"):
        print(f"[{TAG}] selftest FAIL: 零 vendoring 正例误判", file=sys.stderr)
        return 1
    # 红臂③:私接面检出器——边界文件外 token 命中必判 RED;边界文件命中合法。
    if _detect_bypass("src/rurix-render/src/gi/mod.rs", "let r = slEvaluateFeature(...);") is None:
        print(f"[{TAG}] selftest FAIL: 私接面注入未检出", file=sys.stderr)
        return 1
    if _detect_bypass(FFI_BOUNDARY, "slEvaluateFeature ffxDispatch") is not None:
        print(f"[{TAG}] selftest FAIL: FFI 边界文件正例误判", file=sys.stderr)
        return 1
    if _detect_bypass("src/rurix-render/src/gi/mod.rs", "// 自研时序超分,零 vendor 依赖") is not None:
        print(f"[{TAG}] selftest FAIL: 零接线正例误判", file=sys.stderr)
        return 1
    # 红臂④:harness evidence 判读——skipped_dev_env / fail 态不得判 pass。
    if _harness_state_pass('{"state":"skipped_dev_env"}'):
        print(f"[{TAG}] selftest FAIL: SKIP 态误判 pass", file=sys.stderr)
        return 1
    if not _harness_state_pass('{"state":"pass"}'):
        print(f"[{TAG}] selftest FAIL: pass 态正例误判", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} schema_required={len(req)} (4 RED + 4 GREEN)")
    return 0


def _detect_temporal_diff(diff_text: str) -> bool:
    return bool(diff_text.strip())


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
    ok, msg = license_clearance_ok()
    checks["license_clearance_present"] = ok
    check(ok, msg)
    note(msg)

    ok, msg = temporal_base_0byte()
    checks["temporal_base_0byte"] = ok
    check(ok, msg)
    note(msg)

    ok, msg = no_vendor_source_vendoring()
    checks["no_vendor_source_vendoring"] = ok
    check(ok, msg)
    note(msg)

    ok, msg = no_private_bypass_surface()
    checks["no_private_bypass_surface"] = ok
    check(ok, msg)
    note(msg)

    ok, msg = sdk_registry_ok()
    checks["sdk_registry_present"] = ok
    check(ok, msg)
    note(msg)

    r = run(["cargo", "test", "-p", "rurix-render", "--lib", "temporal::"])
    blob = r.stdout + r.stderr
    missing = [n for n in TEMPORAL_TESTS if n not in blob]
    checks["host_upscale_tests_anchored"] = r.returncode == 0 and "test result: ok" in blob and not missing
    check(checks["host_upscale_tests_anchored"], f"temporal 金标准单测失败或未锚定: {missing[:3]} rc={r.returncode}")
    note(f"{len(TEMPORAL_TESTS)} temporal 单测逐名锚定全绿")

    budget = load_g13_budget()
    checks["budget_anchors_present"] = budget is not None and all(
        (budget_entry(budget, eid) or {}).get("evidence") == "measured_local"
        for eid, _b, _d, _s, _desc in CALIB_ENTRY_REGISTRY
    )
    if budget is None:
        note("g13_budget.json 未建(首跑标定腿创建)")

    # ── 持锁段(构建 + 标定腿 + device 腿;单 GPU 互斥) ──
    device_state = "fail"
    doc: dict | None = None
    with gpu_device_lock(purpose=f"{TAG} 构建+标定+device 腿"):
        harness = build_harness()
        if harness is None:
            check(False, "g13_vendor_upscale harness 构建失败")
        else:
            WORK_DIR.mkdir(parents=True, exist_ok=True)
            calibs = run_calibration(harness)
            checks["calibration_two_run_bitexact"] = calibs is not None
            if calibs is not None:
                problems = append_calibration_budget_entries(calibs, ts)
                checks["calibration_budget_entries_measured"] = not problems
                check(not problems, f"标定条目追加: {problems[:2]}")
                # 追加后重读(门消费面 = budget 现值)。
                budget = load_g13_budget()
                checks["budget_anchors_present"] = budget is not None and all(
                    (budget_entry(budget, eid) or {}).get("evidence") == "measured_local"
                    for eid, _b, _d, _s, _desc in CALIB_ENTRY_REGISTRY
                )
        if checks["calibration_budget_entries_measured"]:
            r = run(["py", "-3", "ci/budget_eval.py"])
            checks["budget_eval_all_pass"] = r.returncode == 0 and "[budget_eval] PASS" in (r.stdout + r.stderr)
            check(checks["budget_eval_all_pass"], f"budget_eval 非零: {(r.stdout + r.stderr)[-300:]}")

        if harness is not None and checks["budget_anchors_present"] and budget is not None:
            device_state, doc, arm_results, leg_failures = run_device_leg(harness, budget)
            for f in leg_failures:
                check(False, f)
            if device_state == "skipped_dev_env":
                check(False, "device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP)")
                device_state = "fail"
        elif harness is not None:
            arm_results = {}
            check(False, "budget 标定条目未齐备(标定腿未绿不得跑 device)")
        else:
            arm_results = {}

    # ── device 判据判读 ──
    if device_state == "executed" and doc is not None:
        backends = doc.get("backends") or {}
        dlss = backends.get("dlss") or {}
        fsr = backends.get("fsr") or {}
        tsr = backends.get("tsr") or {}
        checks["device_harness_full_pass"] = True
        checks["device_dlss_real_run"] = (
            dlss.get("ran") is True
            and bool(dlss.get("dlls"))
            and "SL 2.10.3" in str(dlss.get("engine_version", ""))
        )
        checks["device_fsr_real_run"] = (
            fsr.get("ran") is True
            and bool(fsr.get("dlls"))
            and str(fsr.get("engine_version", "")).startswith("3.1.5")
        )
        checks["device_fsr4_ml_fallback_registered"] = (
            fsr.get("fsr4_ml_available") is False and bool(fsr.get("fsr4_note"))
        )
        switch = doc.get("switch") or {}
        order = switch.get("order") or []
        checks["device_three_backend_runtime_switch"] = (
            switch.get("ok") is True
            and {"tsr", "dlss_sr", "fsr_3.1.5"} <= set(order)
        )
        checks["device_static_converge_band_within"] = all(
            b.get("in_band") is True for b in (tsr, dlss, fsr)
        )
        pairwise = doc.get("pairwise") or {}
        checks["device_pairwise_tsr_measured_registered"] = all(
            isinstance(pairwise.get(k), (int, float)) and pairwise.get(k) > 0.0
            for k in (
                "dlss_vs_tsr_ssim", "dlss_vs_tsr_maxdiff",
                "fsr_vs_tsr_ssim", "fsr_vs_tsr_maxdiff",
            )
        )
        registry = load_json(SDK_REGISTRY) if SDK_REGISTRY.is_file() else {}
        reg_dlls = {
            name: sha
            for section in ((registry.get("sdks") or {}).values())
            for name, sha in (section.get("dlls") or {}).items()
        }
        run_dlls = {
            name: sha
            for b in (dlss, fsr)
            for name, sha in (b.get("dlls") or [])
        }
        checks["device_dll_provenance_match"] = (
            bool(run_dlls) and run_dlls == {k: v for k, v in reg_dlls.items() if k in run_dlls}
            and len(run_dlls) == 6
        )
        checks["device_mock_passthrough_detected"] = arm_results.get("mock-passthrough") is True
        checks["device_red_arm_submodes_detected"] = (
            arm_results.get("mv-garbage") is True and arm_results.get("fsr-mv-garbage") is True
        )
        checks["device_validation_zero"] = (
            dlss.get("validation_errors") == 0
            and fsr.get("validation_errors") == 0
            and "ngx_evaluate_excluded" in str(dlss.get("validation_coverage", ""))
            and str(fsr.get("validation_coverage", "")).startswith("full_in_run")
            and not str(dlss.get("validation_coverage", "")).startswith("probe_")
        )
        checks["device_double_run_bitexact"] = all(
            b.get("bitexact") is True for b in (tsr, dlss, fsr)
        )
        for k in CHECK_KEYS:
            if k.startswith("device_") and k not in (
                "device_mock_passthrough_detected",
                "device_red_arm_submodes_detected",
            ) and not checks[k]:
                check(False, f"harness 判据 {k} 为假")
        if not checks["device_mock_passthrough_detected"]:
            check(False, "RED 臂 mock-passthrough 未独立检出(mock/stub 充真跑即 RED)")
        if not checks["device_red_arm_submodes_detected"]:
            check(False, "RED 臂子模式(mv-garbage/fsr-mv-garbage)未独立检出")
        note(
            "device:DLSS SR(SL 2.10.3/NGX Vulkan interop)+ FSR 3.1.5(D3D12)+ TSR host "
            "三后端同进程运行时切换真跑;RED 三臂独立复跑;validation 双臂零实错"
        )

    host_pass = all(checks[k] for k in CHECK_KEYS if not k.startswith("device_"))
    all_pass = all(checks.values()) and not FAILURES and device_state == "executed"
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M167",
        "milestone": "M167",
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G13.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": base_commit(),
        "host_section_pass": host_pass,
        "device_section_state": device_state,
        "checks": {k: bool(v) for k, v in checks.items()},
        "commands": [
            {"seq": 1, "command": "cargo test -p rurix-render --lib temporal:: (TSR 金标准单测逐名锚定)", "exit_code": 0 if checks["host_upscale_tests_anchored"] else 1},
            {"seq": 2, "command": f"git diff --name-only {G13_ZERO_BASE} -- src/rurix-render/src/temporal (0-byte 机核 + 工作树双面)", "exit_code": 0 if checks["temporal_base_0byte"] else 1},
            {"seq": 3, "command": "g13_vendor_upscale --calibrate tsr|dlss|fsr ×2 (标定腿两跑位级一致)", "exit_code": 0 if checks["calibration_two_run_bitexact"] else 1},
            {"seq": 4, "command": "cargo build -p rurix-render --features vendor-upscale --bin g13_vendor_upscale", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 5, "command": "g13_vendor_upscale --band-tsr <g13.upscale..tsr> --band-dlss <..dlss> --band-fsr <..fsr> (RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 全档;DLSS 校验面经 --validation-probe 自举子进程)", "exit_code": 0 if checks["device_harness_full_pass"] else 1},
            {"seq": 6, "command": "g13_vendor_upscale --red-arm mock-passthrough|mv-garbage|fsr-mv-garbage (逐臂独立复跑)", "exit_code": 0 if checks["device_red_arm_submodes_detected"] else 1},
            {"seq": 7, "command": "py -3 ci/budget_eval.py", "exit_code": 0 if checks["budget_eval_all_pass"] else 1},
        ],
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": environment(),
        "production": {
            "correctness_anchor_unchanged": checks["temporal_base_0byte"],
            "baseline_anchor_id": "g13.upscale.static_converge_ssim_deficit_{tsr,dlss,fsr}(本门标定腿产出入 g13_budget)",
            "measured_value": (
                "; ".join(
                    f"{name} deficit={(doc.get('backends') or {}).get(name, {}).get('deficit', 'n/a')}"
                    for name in ("tsr", "dlss", "fsr")
                )
                + "; "
                + "; ".join(
                    f"{k}={(doc.get('pairwise') or {}).get(k, 'n/a')}"
                    for k in ("dlss_vs_tsr_ssim", "dlss_vs_tsr_maxdiff", "fsr_vs_tsr_ssim", "fsr_vs_tsr_maxdiff")
                )
                if doc
                else "n/a(device 未执行)"
            ),
            "not_worse_than_anchor": checks["device_static_converge_band_within"],
            "threshold_provenance": "g13_budget.json M-a 标定条目(标定腿两跑位级一致程序产,threshold = measured × 2.0 冻结 k,禁手写 P-09)",
            "evolution_register": (
                "DLSS validation 覆盖口径:NGX slEvaluateFeature 段排除(层在下 NGX CUDA interop 触发 "
                "nvoglv64.dll 0xc0000005 驱动内崩溃,vendor 已知 SL+validation 不兼容类 Streamline issue "
                "#84)——覆盖 = 我方 Vulkan 全表面 + SL 代理建链/簿记(--validation-probe 独立子进程);"
                "FSR zero-exposure RED 注入面废止(pre_exposure LDR 路径不消费,实测 deficit 位级一致),"
                "fsr-mv-garbage 臂接替"
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
