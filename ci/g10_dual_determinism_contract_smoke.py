#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.2 波 materialize；G10.5a 波续 g10.5 腿）
"""G10.2/G10.5 M130 双端确定性契约硬门冒烟·双 phase（骨架期步骤 179 /
双端核验腿步骤 187；g10.p0.m130.dual_determinism_contract --phase g10.2|g10.5；
RFC-0026 §4.6 + v1.1 章 E errata；spec/visual_comparison.md RXS-0384 + 修订记录
v1.1 errata 行 + RXS-0390；G10_ACCEPTANCE_MAP §1 M130 行 + §3.3 双阶段口径）。

骨架期（--phase g10.2，host 纯 host 门，device_section_state=not_applicable）：
相机/光照/时间参数同 schema 双端各一份（Rurix 侧骨架参考解析器 = 本脚本内
独立实现，按 RXS-0384 L3 字节布局字面；UE 侧解析器 = harness
g10_param_contract.py，UE 进程内嵌 CPython 载体同一源文件）+ digest 比对面
就位 + 边界浮点语料 + 四 RED 臂；evidence phase_g10_2_pass=true、
phase_g10_5_pass=false（骨架期绿不替双端核验期充绿，MAP §3.3）。

双端核验腿（--phase g10.5，host+device 门，device_section_state=executed；
UE 真跑持 gpu_device_lock 串行）：
  骨架期回归 + 双场景真实契约（milestones/g10/corpus/contract_params_<scene>.json）
  三方 digest 实测相等（host 参考解析器 × Rurix Rust 第三实现
  g10_5_scene_render --contract-digest × UE 进程内嵌 CPython 实跑——
  RXS-0384 L4 双端核验期载体要求兑现）+ 应用层探针（RXS-0390：冻结标志物
  双端各自管线投影 pixel_delta ≤ 1e-3 px，UE 端 as-built 相机读回）+
  三重绑定字段登记（当次 param_digest + base_commit + session_run_id，
  M139 机器前置消费面）+ 确定性再生成核验（生成器复跑逐字节一致）+
  RED 四臂（参数漂移三方不等检出 / 错误共轭注入探针超差检出 / 标志物字面
  漂移检出 / 陈旧绑定冒充检出）。

用法：
  py -3 ci/g10_dual_determinism_contract_smoke.py --gate g10.p0.m130.dual_determinism_contract --phase g10.2
  py -3 ci/g10_dual_determinism_contract_smoke.py --gate g10.p0.m130.dual_determinism_contract --phase g10.5
  py -3 ci/g10_dual_determinism_contract_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import math
import os
import platform
import re
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g10" / "g10_m130_dual_determinism_contract_evidence_schema.json"
SPEC_PATH = ROOT / "spec" / "visual_comparison.md"
UE_PARSER = ROOT / "milestones" / "g10" / "harness" / "ue_python" / "g10_param_contract.py"
PARAMS_PATH = ROOT / "milestones" / "g10" / "harness" / "examples" / "contract_params_entry_smoke.json"

GATE_KEY = "g10.p0.m130.dual_determinism_contract"
NUMERIC_STEP = 179
SOURCE_REF = "RFC-0026 §4.6;spec/visual_comparison.md RXS-0384;G10_ACCEPTANCE_MAP §1 M130 + §3.3"
TAG = "g10_m130"
SUBJECT = "g10_m130_dual_determinism_contract"
MATRIX_ROW = "M130"

CHECK_KEYS = [
    "spec_rxs0384_clause_on_tree",
    "byte_layout_matches_spec",
    "rurix_side_parse_digest",
    "ue_side_parse_digest",
    "dual_digest_compare_face",
    "boundary_float_corpus_consistent",
    "red_param_drift_detected",
    "red_schema_extra_field_detected",
    "red_nonunit_quat_detected",
    "red_nan_injection_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


# ---------------------------------------------------------------------------
# Rurix 侧骨架参考解析器（RXS-0384 独立实现一份；UE 侧 = harness
# g10_param_contract.py，双端各一份、同 schema 版本互证）。
# ---------------------------------------------------------------------------

UNIT_NORM_TOL = 2.0 ** -40  # RXS-0384 L2 合法性谓词常量（不走 g10_budget）

BYTE_LAYOUT = {
    "version_prefix": b"G10DCP-1\x00",  # RXS-0384 L3 冻结字面
    "tag_f64": 0x01,
    "tag_u32": 0x02,
    "tag_u64": 0x03,
    "tag_str": 0x04,
    "tag_bool": 0x05,
    "tag_null": 0x06,
    "tag_obj_begin": 0x07,
    "tag_obj_end": 0x08,
    "tag_arr_begin": 0x09,
    "tag_arr_end": 0x0A,
}

SCHEMA = {
    "camera": {
        "position": ("arr_f64", 3),
        "orientation_quat": ("arr_f64", 4),
        "fov_y_deg": "f64",
        "near": "f64",
        "far": "f64",
        "resolution": {"w": "u32", "h": "u32"},
    },
    "lighting": {
        "sun": {
            "direction": ("arr_f64", 3),
            "intensity_lux": "f64",
            "color_linear_rgb": ("arr_f64", 3),
        },
        "sky": {"intensity": "f64", "cubemap_id": "str_or_null"},
        "exposure": {"mode": ("enum", ("manual",)), "ev100": "f64"},
    },
    "time": {
        "fixed_dt_s": "f64",
        "warmup_frames": "u32",
        "capture_frame_index": "u32",
        "random_seed": "u64",
        "jitter": {
            "sequence": ("enum", ("halton_2_3",)),
            "index_base": "u32",
            "scale": "f64",
        },
    },
    "post": {
        "view_transform": ("enum", ("aces13",)),
        "bloom": ("const", False),
        "vignette": ("const", False),
        "motion_blur": ("const", False),
        "dof": ("const", False),
    },
}


class ContractError(ValueError):
    """fail-closed：任何 schema 违例即抛出。"""


def _f64(name: str, v) -> float:
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        raise ContractError(f"{name}: expected f64, got {type(v).__name__}")
    v = float(v)
    if math.isnan(v) or math.isinf(v):
        raise ContractError(f"{name}: NaN/Inf forbidden")
    return v


def _uint(name: str, v, bits: int) -> int:
    if isinstance(v, bool) or not isinstance(v, int):
        raise ContractError(f"{name}: expected u{bits}, got {type(v).__name__}")
    if v < 0 or v >= 2 ** bits:
        raise ContractError(f"{name}: out of u{bits} range")
    return v


def _validate(name: str, spec, value):
    if isinstance(spec, dict):
        if not isinstance(value, dict):
            raise ContractError(f"{name}: expected object")
        extra = set(value) - set(spec)
        if extra:
            raise ContractError(f"{name}: unknown fields {sorted(extra)}")
        missing = set(spec) - set(value)
        if missing:
            raise ContractError(f"{name}: missing fields {sorted(missing)}")
        return {k: _validate(f"{name}.{k}", s, value[k]) for k, s in spec.items()}
    if isinstance(spec, tuple):
        kind = spec[0]
        if kind == "arr_f64":
            if not isinstance(value, list) or len(value) != spec[1]:
                raise ContractError(f"{name}: expected f64[{spec[1]}]")
            return [_f64(f"{name}[{i}]", x) for i, x in enumerate(value)]
        if kind == "enum":
            if value not in spec[1]:
                raise ContractError(f"{name}: value {value!r} not in {spec[1]}")
            return value
        if kind == "const":
            if value is not spec[1] and value != spec[1]:
                raise ContractError(f"{name}: must be {spec[1]!r}")
            return value
        raise ContractError(f"{name}: bad spec {spec!r}")
    if spec == "f64":
        return _f64(name, value)
    if spec == "u32":
        return _uint(name, value, 32)
    if spec == "u64":
        return _uint(name, value, 64)
    if spec == "str_or_null":
        if value is not None and not isinstance(value, str):
            raise ContractError(f"{name}: expected string|null")
        return value
    raise ContractError(f"{name}: bad spec {spec!r}")


def parse_contract_rurix(text: str) -> dict:
    """Rurix 侧骨架解析（strict fail-closed；CPython json correctly-rounded 口径）。"""
    data = json.loads(text)
    if not isinstance(data, dict):
        raise ContractError("top level must be object")
    extra = set(data) - set(SCHEMA)
    if extra:
        raise ContractError(f"unknown sections {sorted(extra)}")
    missing = set(SCHEMA) - set(data)
    if missing:
        raise ContractError(f"missing sections {sorted(missing)}")
    out = {k: _validate(k, SCHEMA[k], data[k]) for k in SCHEMA}
    q = out["camera"]["orientation_quat"]
    if abs(sum(x * x for x in q) - 1.0) > UNIT_NORM_TOL:
        raise ContractError("camera.orientation_quat: unit-norm violated (|q²−1| > 2^-40)")
    d = out["lighting"]["sun"]["direction"]
    if abs(sum(x * x for x in d) - 1.0) > UNIT_NORM_TOL:
        raise ContractError("lighting.sun.direction: unit-norm violated (|d²−1| > 2^-40)")
    return out


def _enc(buf: bytes, spec, value) -> bytes:
    L = BYTE_LAYOUT
    if isinstance(spec, dict):
        buf += bytes([L["tag_obj_begin"]])
        for k in sorted(spec, key=lambda s: [ord(c) for c in s]):  # Unicode code point 序
            kb = k.encode("utf-8")
            buf += struct.pack("<I", len(kb)) + kb
            buf = _enc(buf, spec[k], value[k])
        return buf + bytes([L["tag_obj_end"]])
    if isinstance(spec, tuple):
        kind = spec[0]
        if kind == "arr_f64":
            buf += bytes([L["tag_arr_begin"]])
            for x in value:
                buf += bytes([L["tag_f64"]]) + struct.pack("<d", x)
            return buf + bytes([L["tag_arr_end"]])
        if kind == "enum":
            sb = value.encode("utf-8")
            return buf + bytes([L["tag_str"]]) + struct.pack("<I", len(sb)) + sb
        if kind == "const":
            return buf + bytes([L["tag_bool"]]) + (b"\x01" if value else b"\x00")
        raise ContractError(f"bad spec {spec!r}")
    if spec == "f64":
        return buf + bytes([L["tag_f64"]]) + struct.pack("<d", value)
    if spec == "u32":
        return buf + bytes([L["tag_u32"]]) + struct.pack("<I", value)
    if spec == "u64":
        return buf + bytes([L["tag_u64"]]) + struct.pack("<Q", value)
    if spec == "str_or_null":
        if value is None:
            return buf + bytes([L["tag_null"]])
        sb = value.encode("utf-8")
        return buf + bytes([L["tag_str"]]) + struct.pack("<I", len(sb)) + sb
    raise ContractError(f"bad spec {spec!r}")


def canonical_preimage_rurix(contract: dict) -> bytes:
    return _enc(BYTE_LAYOUT["version_prefix"], SCHEMA, contract)


def param_digest_rurix(contract: dict) -> str:
    return hashlib.sha256(canonical_preimage_rurix(contract)).hexdigest()


def param_digest_ue5(params_file: Path) -> str:
    """UE 侧解析器（harness g10_param_contract.py）子进程实跑 digest。"""
    argv = [sys.executable, str(UE_PARSER), str(params_file)]
    r = subprocess.run(argv, capture_output=True, text=True, timeout=120)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(argv), "exit_code": r.returncode})
    if r.returncode != 0:
        raise ContractError(f"UE 侧解析器非零退出: {r.stderr[-300:]}")
    line = r.stdout.strip()
    prefix = "param_digest_ue5 = "
    if not line.startswith(prefix):
        raise ContractError(f"UE 侧解析器输出形态异常: {line[:120]!r}")
    return line[len(prefix):].strip()


def _expect_reject(text: str, arm: str) -> bool:
    try:
        parse_contract_rurix(text)
    except ContractError as e:
        note(f"RED 检出 {arm}: {e}")
        return True
    check(False, f"{arm}: 注入未被 Rurix 侧解析器拒绝（假绿口）")
    return False


def leg_byte_layout_matches_spec() -> bool:
    """spec 条款字面 ↔ 双端布局常量三方一致（RXS-0384 IR2）。"""
    ok = True
    text = SPEC_PATH.read_text(encoding="utf-8") if SPEC_PATH.is_file() else ""
    for literal in ('"G10DCP-1"', "47 31 30 44 43 50 2D 31 00"):
        if literal not in text:
            check(False, f"spec 缺冻结字面 {literal}")
            ok = False
    sys.path.insert(0, str(UE_PARSER.parent))
    try:
        import g10_param_contract as ue_pc  # noqa: E402
    finally:
        sys.path.remove(str(UE_PARSER.parent))
    ue_layout = ue_pc.SPEC_BYTE_LAYOUT
    if ue_layout["version_prefix"] != BYTE_LAYOUT["version_prefix"]:
        check(False, "版本前缀双端不一致")
        ok = False
    for k, v in BYTE_LAYOUT.items():
        if k == "version_prefix":
            continue
        if ue_layout[k] != bytes([v]):
            check(False, f"类型标签 {k} 双端不一致: ue={ue_layout[k]!r} rurix={bytes([v])!r}")
            ok = False
    if ue_pc.SPEC_VERSION_PREFIX_HEX != BYTE_LAYOUT["version_prefix"].hex():
        check(False, "版本前缀 hex 旁证不一致")
        ok = False
    return ok


def leg_boundary_corpus() -> bool:
    """边界浮点差分语料：-0.0 / 次正规 / 长十进制最短表示 / 1e-310 / u64 上界，
    双端解析逐位一致（digest 相等）。"""
    base = json.loads(PARAMS_PATH.read_text(encoding="utf-8"))
    base["camera"]["position"] = [-0.0, 1e-310, 0.30000000000000004]
    base["camera"]["fov_y_deg"] = 2.2250738585072014e-308
    base["camera"]["near"] = 5e-324
    base["camera"]["far"] = 1.7976931348623157e308
    base["lighting"]["sun"]["intensity_lux"] = 1e-310
    base["lighting"]["sun"]["color_linear_rgb"] = [0.1, 0.2, 0.30000000000000004]
    base["lighting"]["sky"]["intensity"] = -0.0
    base["lighting"]["exposure"]["ev100"] = -0.0
    base["time"]["fixed_dt_s"] = 1e-310
    base["time"]["random_seed"] = 18446744073709551615
    base["time"]["jitter"]["scale"] = 0.30000000000000004
    text = json.dumps(base)
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8") as tf:
        tf.write(text)
        tmp = Path(tf.name)
    try:
        d_rurix = param_digest_rurix(parse_contract_rurix(text))
        d_ue5 = param_digest_ue5(tmp)
    finally:
        tmp.unlink(missing_ok=True)
    if d_rurix != d_ue5:
        check(False, f"边界浮点语料跨端不一致: rurix={d_rurix[:16]}… ue5={d_ue5[:16]}…")
        return False
    note(f"边界浮点语料跨端逐位一致: {d_rurix[:16]}…")
    return True


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    text = PARAMS_PATH.read_text(encoding="utf-8")
    # 绿臂：双端 digest 相等（真跑 UE 侧解析器）。
    d_r = param_digest_rurix(parse_contract_rurix(text))
    d_u = param_digest_ue5(PARAMS_PATH)
    if d_r != d_u:
        print(f"[{TAG}] selftest FAIL: 双端 digest 不等 {d_r[:16]}… ≠ {d_u[:16]}…", file=sys.stderr)
        return 1
    # 红臂①：单端参数漂移 ⇒ digest 不等。
    drifted = json.loads(text)
    drifted["camera"]["fov_y_deg"] = drifted["camera"]["fov_y_deg"] + 1e-12
    if param_digest_rurix(parse_contract_rurix(json.dumps(drifted))) == d_u:
        print(f"[{TAG}] selftest FAIL: 参数漂移未引起 digest 不等", file=sys.stderr)
        return 1
    # 红臂②：schema 外字段注入必拒。
    extra = json.loads(text)
    extra["misc"] = {}
    try:
        parse_contract_rurix(json.dumps(extra))
        print(f"[{TAG}] selftest FAIL: schema 外字段未拒", file=sys.stderr)
        return 1
    except ContractError:
        pass
    # 红臂③（G10.5a 改接）：错误共轭注入 ⇒ 探针超差检出（host 纯计算，
    # 真实 bistro 契约——一般旋转面，缺陷式镜像必超差）。
    c_bi = parse_contract_rurix(
        (ROOT / "milestones" / "g10" / "corpus" / "contract_params_bistro_interior.json").read_text(encoding="utf-8")
    )
    px_def = _ue_project(c_bi, LANDMARKS_SPEC["bistro-interior"], _quat_contract_to_ue_defective)
    px_fix = _ue_project(c_bi, LANDMARKS_SPEC["bistro-interior"], _quat_contract_to_ue_errata)
    px_ref = project_landmarks_host(c_bi, LANDMARKS_SPEC["bistro-interior"])
    cmp_def = [(a, b) for a, b in zip(px_def, px_ref) if a is not None and b is not None]
    d_def = max(max(abs(a[0] - b[0]), abs(a[1] - b[1])) for a, b in cmp_def) if cmp_def else float("inf")
    d_fix = max(max(abs(a[0] - b[0]), abs(a[1] - b[1])) for a, b in zip(px_fix, px_ref))
    if not (d_def > PIXEL_DELTA_TOL and d_fix <= PIXEL_DELTA_TOL):
        print(f"[{TAG}] selftest FAIL: 错误共轭臂失效 d_def={d_def:.3e} d_fix={d_fix:.3e}", file=sys.stderr)
        return 1
    # 红臂④（G10.5a）：陈旧 session_run_id 冒充 ⇒ 三重绑定核验拒绝。
    synth = {
        "status": "pass",
        "phase_g10_5_pass": True,
        "base_commit": "abc",
        "contract_report": {
            "param_digest": "sha256:x",
            "param_digest_rurix": "sha256:x",
            "param_digest_ue5": "sha256:x",
            "session_run_id": "stale",
        },
    }
    if verify_three_binding(synth, "sha256:x", "current", "abc"):
        print(f"[{TAG}] selftest FAIL: 陈旧绑定冒充未被拒绝", file=sys.stderr)
        return 1
    if not verify_three_binding(synth, "sha256:x", "stale", "abc"):
        print(f"[{TAG}] selftest FAIL: 正当绑定被误拒（三重绑定核验过紧）", file=sys.stderr)
        return 1
    # 绿臂：schema anyOf 双支 checks.required 与双 phase CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    branches = schema.get("anyOf", [])
    if len(branches) != 2:
        print(f"[{TAG}] selftest FAIL: schema 非 anyOf 双支形态", file=sys.stderr)
        return 1
    req_v1 = set(branches[0].get("properties", {}).get("checks", {}).get("required", []))
    req_v2 = set(branches[1].get("properties", {}).get("checks", {}).get("required", []))
    if req_v1 != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: v1 支 required 与 CHECK_KEYS 不等 {req_v1 ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    if req_v2 != set(CHECK_KEYS_G10_5):
        print(f"[{TAG}] selftest FAIL: v2 支 required 与 CHECK_KEYS_G10_5 不等 {req_v2 ^ set(CHECK_KEYS_G10_5)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)}+{len(CHECK_KEYS_G10_5)} (6 RED + 3 GREEN)")
    return 0


# ---------------------------------------------------------------------------
# G10.5a 双端核验腿（--phase g10.5，步骤 187；RXS-0390 应用层探针 + 三重绑定登记）
# ---------------------------------------------------------------------------

NUMERIC_STEP_G10_5 = 187
PIXEL_DELTA_TOL = 1e-3  # RXS-0390 L1 合法性谓词常量（不走 g10_budget）

CORPUS = ROOT / "milestones" / "g10" / "corpus"
HARNESS = ROOT / "milestones" / "g10" / "harness"
UE_RUN = HARNESS / "g10_5_ue_run.py"
UE_BUILD = HARNESS / "ue_python" / "g10_5_build_scenes.py"
UE_PROBE = HARNESS / "ue_python" / "g10_5_probe_landmarks.py"
GEN_PARAMS = HARNESS / "g10_5_gen_contract_params.py"
RUST_BIN = ROOT / "target" / "debug" / "g10_5_scene_render.exe"
PROBE_OUT_DIR = Path(r"K:\rurix-ext\g10-frames\g10_5")

G10_5_SCENES = ("cornell-box", "bistro-interior")

# RXS-0390 L2 冻结标志物集（与 spec 条款逐值同字面；门内对账面之一）。
LANDMARKS_SPEC = {
    "cornell-box": [
        (0.0, 0.0, 558.8),
        (552.8, 0.0, 558.8),
        (552.8, 548.8, 558.8),
        (0.0, 548.8, 558.8),
        (276.4, 274.4, 558.8),
    ],
    "bistro-interior": [
        (2.0375248420941845, 1.3697032820278594, -1.6595583445401449),
        (2.1463398736291461, 1.6862064060565474, -0.82191749619001619),
        (1.9521887623639345, 1.6862064214520678, -2.4999157956664435),
        (2.1228609218244348, 1.053200142603651, -0.81920089341384617),
        (1.9287098105592226, 1.0532001579991714, -2.4971991928902737),
    ],
}

CHECK_KEYS_G10_5 = [
    "skeleton_phase_regression",
    "dual_scene_coverage",
    "contract_params_deterministic_regen",
    "rurix_rust_digest_real_run",
    "ue_inprocess_digest_real_run",
    "triple_digest_equal",
    "landmark_set_matches_spec",
    "application_probe_dual_end",
    "three_binding_fields_registered",
    "red_param_drift_detected",
    "red_wrong_conjugation_detected",
    "red_landmark_tamper_detected",
    "red_stale_binding_detected",
]


def _quat_rotate(q, v):
    w, x, y, z = q
    uv = (y * v[2] - z * v[1], z * v[0] - x * v[2], x * v[1] - y * v[0])
    uuv = (y * uv[2] - z * uv[1], z * uv[0] - x * uv[2], x * uv[1] - y * uv[0])
    return tuple(v[i] + 2.0 * (w * uv[i] + uuv[i]) for i in range(3))


def project_landmarks_host(contract: dict, landmarks) -> list:
    """host 侧 f64 独立参考投影（契约空间针孔；glTF 相机惯例 forward=R(q)·(0,0,−1)）。
    与 Rust 探针 f32 路径对账，并供 RED 臂（错误共轭注入）构造镜像投影。"""
    cam = contract["camera"]
    eye = cam["position"]
    q = cam["orientation_quat"]
    fwd = _quat_rotate(q, (0.0, 0.0, -1.0))
    up = _quat_rotate(q, (0.0, 1.0, 0.0))
    s = (
        fwd[1] * up[2] - fwd[2] * up[1],
        fwd[2] * up[0] - fwd[0] * up[2],
        fwd[0] * up[1] - fwd[1] * up[0],
    )
    n = math.sqrt(sum(a * a for a in s))
    s = tuple(a / n for a in s)
    u = (
        s[1] * fwd[2] - s[2] * fwd[1],
        s[2] * fwd[0] - s[0] * fwd[2],
        s[0] * fwd[1] - s[1] * fwd[0],
    )
    w, h = cam["resolution"]["w"], cam["resolution"]["h"]
    tan_v = math.tan(math.radians(cam["fov_y_deg"]) / 2.0)
    tan_h = tan_v * (w / h)
    out = []
    for p in landmarks:
        rel = tuple(p[i] - eye[i] for i in range(3))
        zc = sum(rel[i] * fwd[i] for i in range(3))
        if zc <= 0.0:
            out.append(None)
            continue
        xc = sum(rel[i] * s[i] for i in range(3))
        yc = sum(rel[i] * u[i] for i in range(3))
        ndx = xc / (zc * tan_h)
        ndy = yc / (zc * tan_v)
        out.append(((ndx + 1.0) * 0.5 * w, (1.0 - ndy) * 0.5 * h))
    return out


def _quat_contract_to_ue_errata(q):
    """errata 修订式（RFC-0026 v1.1 章 E）：q_ue = (w, z, −x, −y)。"""
    w, x, y, z = q
    return (w, z, -x, -y)


def _quat_contract_to_ue_defective(q):
    """RED 臂用缺陷式（errata 前实现）：(w, −z, x, y) = R(M·axis, +θ) 镜像。"""
    w, x, y, z = q
    return (w, -z, x, y)


def _ue_project(contract: dict, landmarks, quat_map) -> list:
    """UE 侧像素计算（host 镜像 g10_5_probe_landmarks.py 公式面，f64）：
    p_ue = M·p·100；相机三轴 = R(q_ue) 作用于 UE 基 (X fwd / Y right / Z up)；
    view = (rel·right, rel·up, rel·fwd)；ndc → px（UE 5.8 源树锚定链）。"""
    cam = contract["camera"]
    loc = (-cam["position"][2] * 100.0, cam["position"][0] * 100.0, cam["position"][1] * 100.0)
    q_ue = quat_map(cam["orientation_quat"])
    fwd = _quat_rotate(q_ue, (1.0, 0.0, 0.0))
    right = _quat_rotate(q_ue, (0.0, 1.0, 0.0))
    up = _quat_rotate(q_ue, (0.0, 0.0, 1.0))
    w, h = cam["resolution"]["w"], cam["resolution"]["h"]
    tan_h = math.tan(math.radians(cam["fov_y_deg"]) / 2.0) * (w / h)
    tan_v = tan_h / (w / h)
    out = []
    for p in landmarks:
        p_ue = (-p[2] * 100.0, p[0] * 100.0, p[1] * 100.0)
        rel = tuple(p_ue[i] - loc[i] for i in range(3))
        vz = sum(rel[i] * fwd[i] for i in range(3))
        if vz <= 0.0:
            out.append(None)
            continue
        vx = sum(rel[i] * right[i] for i in range(3))
        vy = sum(rel[i] * up[i] for i in range(3))
        ndx = vx / (vz * tan_h)
        ndy = vy / (vz * tan_v)
        out.append(((ndx / 2.0 + 0.5) * w, (0.5 - ndy / 2.0) * h))
    return out


def load_latest_g10_5_evidence() -> dict | None:
    """M130 双端核验期最新 evidence（「最新」= 顶层 timestamp UTC 最大、并列以
    落盘文件名 UTC stamp 次键、仍并列 fail-closed——RFC-0026 §4.6 门序字面）。"""
    cands = []
    for f in EVIDENCE_DIR.glob("g10_m130_dual_determinism_contract_*.json"):
        try:
            doc = json.loads(f.read_text(encoding="utf-8"))
        except Exception:
            continue
        if doc.get("phase") == "g10.5":
            cands.append((doc.get("timestamp", ""), f.name, doc))
    if not cands:
        return None
    cands.sort(key=lambda t: (t[0], t[1]))
    top = [c for c in cands if (c[0], c[1]) == (cands[-1][0], cands[-1][1])]
    if len(top) != 1:
        raise ContractError("最新 evidence 判定并列（fail-closed）")
    return top[0][2]


def verify_three_binding(evidence: dict, param_digest: str, session_run_id: str, base_commit: str) -> bool:
    """三重绑定机器核验（RFC-0026 §4.6 / §4.0 不变量 4；M139 机器前置消费面）：
    (a) evidence 双端 digest 相等（param_digest_rurix == param_digest_ue5）；
    (b) evidence status==pass 且 phase_g10_5_pass==true；
    (c) 同 base_commit 同 session_run_id（陈旧 pass 不得冒充当次一致）。
    双场景口径（G10.5b 修订）：登记面 param_digest = 双场景联合值（字典序拼接
    sha256），param_digest_rurix/ue5 = 首场景双端 digest——(a) 只断言双端相等
    （RFC「且二者相等」字面），不断言等于联合值；入参 param_digest 比对面 =
    evidence 登记的联合 param_digest（(b) 语义由 M139 门独立重算联合值后对账）。
    G10.5a 形态（rurix==ue5==入参）在联合值 ≠ 首场景 digest 时对本门自身
    g10.5 evidence 恒假——过严，本修订回 RFC 字面。"""
    if evidence.get("status") != "pass" or evidence.get("phase_g10_5_pass") is not True:
        return False
    rep = evidence.get("contract_report", {})
    if rep.get("param_digest") != param_digest or not param_digest:
        return False
    if rep.get("param_digest_rurix") != rep.get("param_digest_ue5") or not rep.get("param_digest_rurix"):
        return False
    if evidence.get("base_commit") != base_commit:
        return False
    if rep.get("session_run_id") != session_run_id or not session_run_id:
        return False
    return True


def _run(cmd, env_extra=None, timeout=1800) -> tuple[int, str]:
    import subprocess as _sp
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    r = _sp.run(cmd, capture_output=True, text=True, timeout=timeout, env=env)
    COMMANDS.append({"seq": len(COMMANDS) + 1, "command": " ".join(str(c) for c in cmd), "exit_code": r.returncode})
    return r.returncode, (r.stdout or "") + (r.stderr or "")


def _rust_digest(params_path: Path) -> str:
    rc, out = _run([str(RUST_BIN), "--contract-digest", str(params_path)], timeout=600)
    if rc != 0:
        raise ContractError(f"Rust 契约 digest 非零退出: {out[-300:]}")
    for line in out.splitlines():
        if line.startswith("param_digest_rust = "):
            return line.split("=", 1)[1].strip()
    raise ContractError(f"Rust 契约 digest 输出形态异常: {out[:160]!r}")


def _rust_probe_pixels(params_path: Path, landmarks_path: Path) -> list:
    rc, out = _run([str(RUST_BIN), "--project-landmarks", "--contract", str(params_path), "--landmarks", str(landmarks_path)], timeout=600)
    if rc != 0:
        raise ContractError(f"Rust 探针投影非零退出: {out[-300:]}")
    line = out.strip().splitlines()[-1]
    doc = json.loads(line)
    return doc["pixels"]


def _ue_build_and_probe(scene_id: str, params_path: Path) -> dict:
    """UE 真跑：关卡建设（幂等，含导入）+ 探针（进程内 digest + as-built 相机投影）。"""
    probe_out = PROBE_OUT_DIR / f"probe_{scene_id}.json"
    env = {
        "G10_5_SCENE": scene_id,
        "G10_5_CONTRACT": str(params_path),
        "G10_5_SKIP_IMPORT": "1",
    }
    rc, out = _run([sys.executable, str(UE_RUN), str(UE_BUILD)], env_extra=env, timeout=1800)
    if rc != 0:
        raise ContractError(f"UE 关卡建设非零退出（{scene_id}）: {out[-300:]}")
    env2 = dict(env)
    env2["G10_5_PROBE_OUT"] = str(probe_out)
    if probe_out.exists():
        probe_out.unlink()
    rc, out = _run([sys.executable, str(UE_RUN), str(UE_PROBE)], env_extra=env2, timeout=1800)
    if rc != 0 or not probe_out.exists():
        raise ContractError(f"UE 探针非零退出或无产物（{scene_id}）: rc={rc} {out[-300:]}")
    return json.loads(probe_out.read_text(encoding="utf-8"))


def _spec_landmark_literals() -> dict:
    """从 spec/visual_comparison.md RXS-0390 L2 抽取冻结标志物字面（对账面）。"""
    text = SPEC_PATH.read_text(encoding="utf-8")
    m = re.search(r"### RXS-0390.*?(?=\n### |\n## )", text, re.DOTALL)
    if not m:
        raise ContractError("spec 缺 RXS-0390 条款段")
    seg = m.group(0)
    out = {}
    for scene in G10_5_SCENES:
        sm = re.search(r"`" + scene + r"`（[^：]*?）：(.*?)；", seg, re.DOTALL)
        if not sm:
            raise ContractError(f"spec RXS-0390 L2 缺 {scene} 标志物段")
        tuples = re.findall(r"\(([^()]+)\)", sm.group(1))
        pts = []
        for t in tuples:
            vals = tuple(float(x.strip()) for x in t.split(","))
            if len(vals) == 3:
                pts.append(vals)
        out[scene] = pts
    return out


def _ue_script_landmark_literals() -> dict:
    """从 UE 探针脚本抽取内嵌标志物字面（ast 解析 LANDMARKS 赋值）。"""
    import ast as _ast
    tree = _ast.parse(UE_PROBE.read_text(encoding="utf-8"))
    for node in tree.body:
        if isinstance(node, _ast.Assign) and getattr(node.targets[0], "id", "") == "LANDMARKS":
            val = _ast.literal_eval(node.value)
            return {k: [tuple(p) for p in v] for k, v in val.items()}
    raise ContractError("UE 探针脚本缺 LANDMARKS 字面")


def run_g10_5() -> int:
    import os as _os  # noqa: F401（_run 内使用）
    checks: dict[str, bool] = {k: False for k in CHECK_KEYS_G10_5}

    # ---- 骨架期回归（g10.2 门面同段真跑复验）----
    text0 = PARAMS_PATH.read_text(encoding="utf-8")
    reg_ok = True
    try:
        d_r0 = param_digest_rurix(parse_contract_rurix(text0))
        d_u0 = param_digest_ue5(PARAMS_PATH)
        reg_ok = reg_ok and (d_r0 == d_u0)
    except ContractError as e:
        check(False, f"骨架期回归解析失败: {e}")
        reg_ok = False
    checks["skeleton_phase_regression"] = reg_ok
    check(reg_ok, "骨架期回归失败（双端 digest 不等或解析失败）")

    # ---- 双场景覆盖（anti-vacuous）----
    checks["dual_scene_coverage"] = len(G10_5_SCENES) == 2 and all(
        (CORPUS / f"contract_params_{s.replace('-', '_')}.json").is_file() for s in G10_5_SCENES
    )
    check(checks["dual_scene_coverage"], "双场景契约参数件不齐（vacuous 拦截）")

    # ---- 确定性再生成（生成器复跑逐字节一致）----
    before = {
        s: (CORPUS / f"contract_params_{s.replace('-', '_')}.json").read_bytes() for s in G10_5_SCENES
    }
    rc, out = _run([sys.executable, str(GEN_PARAMS)], timeout=300)
    after = {
        s: (CORPUS / f"contract_params_{s.replace('-', '_')}.json").read_bytes() for s in G10_5_SCENES
    }
    checks["contract_params_deterministic_regen"] = rc == 0 and all(before[s] == after[s] for s in G10_5_SCENES)
    check(checks["contract_params_deterministic_regen"], "契约参数生成器复跑非逐字节一致")

    # ---- Rust 第三实现 digest 真跑（cargo build 后）----
    rc, out = _run(
        ["cargo", "build", "-p", "rurix-asset", "--bin", "g10_5_scene_render"],
        timeout=3600,
    )
    digests = {}
    rust_ok = rc == 0 and RUST_BIN.is_file()
    if rust_ok:
        for s in G10_5_SCENES:
            p = CORPUS / f"contract_params_{s.replace('-', '_')}.json"
            try:
                digests[s] = {"rust": _rust_digest(p)}
            except ContractError as e:
                check(False, f"Rust digest 失败（{s}）: {e}")
                rust_ok = False
    checks["rurix_rust_digest_real_run"] = rust_ok
    check(rust_ok, "Rurix Rust digest 真跑失败")

    # ---- host 参考 digest + UE 进程内 digest + 探针 ----
    for s in G10_5_SCENES:
        if s not in digests:
            continue
        p = CORPUS / f"contract_params_{s.replace('-', '_')}.json"
        c = parse_contract_rurix(p.read_text(encoding="utf-8"))
        digests[s]["host"] = param_digest_rurix(c)
    ue_ok = True
    probes = {}
    # 串行纪律：UE 调用经 g10_5_ue_run.py 子进程自持 gpu_device_lock——门内
    # 不得再嵌套持锁（msvcrt 字节锁跨进程互斥 + 进程内 threading.Lock 不可重入，
    # 嵌套即死锁，G10.5a 首跑实测挂起定案）。
    for s in G10_5_SCENES:
        if s not in digests:
            ue_ok = False
            continue
        p = CORPUS / f"contract_params_{s.replace('-', '_')}.json"
        try:
            probes[s] = _ue_build_and_probe(s, p)
            digests[s]["ue5"] = probes[s]["param_digest_ue5_inprocess"]
        except (ContractError, Exception) as e:  # noqa: BLE001（门内 fail-closed 登记）
            check(False, f"UE 进程内 digest/探针失败（{s}）: {e}")
            ue_ok = False
    checks["ue_inprocess_digest_real_run"] = ue_ok
    check(ue_ok, "UE 进程内 digest 真跑失败")

    # ---- 三方 digest 相等 ----
    tri_ok = bool(digests) and all(
        d.get("rust") == d.get("host") == d.get("ue5") and d.get("rust") for d in digests.values()
    ) and len(digests) == 2
    checks["triple_digest_equal"] = tri_ok
    check(tri_ok, f"三方 digest 不等: {digests}")

    # ---- 标志物集三面对账（spec 条款字面 ↔ corpus JSON ↔ UE 探针脚本内嵌）----
    lm_ok = True
    try:
        spec_lm = _spec_landmark_literals()
        ue_lm = _ue_script_landmark_literals()
        for s in G10_5_SCENES:
            corpus_lm = [
                tuple(float(x) for x in p)
                for p in json.loads((CORPUS / f"landmarks_{s.replace('-', '_')}.json").read_text(encoding="utf-8"))["landmarks"]
            ]
            if list(LANDMARKS_SPEC[s]) != corpus_lm:
                check(False, f"corpus 标志物与门内冻结字面不符（{s}）")
                lm_ok = False
            if spec_lm.get(s) != list(LANDMARKS_SPEC[s]):
                check(False, f"spec RXS-0390 L2 标志物字面漂移（{s}）: {spec_lm.get(s)}")
                lm_ok = False
            if ue_lm.get(s) != list(LANDMARKS_SPEC[s]):
                check(False, f"UE 探针脚本内嵌标志物字面漂移（{s}）")
                lm_ok = False
    except ContractError as e:
        check(False, f"标志物对账失败: {e}")
        lm_ok = False
    checks["landmark_set_matches_spec"] = lm_ok
    check(lm_ok, "标志物集三面对账失败")

    # ---- 应用层探针（RXS-0390：双端各自管线 pixel_delta ≤ 1e-3 px）----
    probe_ok = True
    app_probes = []
    if ue_ok:
        for s in G10_5_SCENES:
            p = CORPUS / f"contract_params_{s.replace('-', '_')}.json"
            c = parse_contract_rurix(p.read_text(encoding="utf-8"))
            px_host = project_landmarks_host(c, LANDMARKS_SPEC[s])
            px_rust = _rust_probe_pixels(p, CORPUS / f"landmarks_{s.replace('-', '_')}.json")
            px_ue = probes[s]["pixels_ue5"]
            rows = []
            for i, lm in enumerate(LANDMARKS_SPEC[s]):
                pr = px_rust[i] if px_rust[i] is not None else None
                pu = px_ue[i] if px_ue[i] is not None else None
                ph = px_host[i] if px_host[i] is not None else None
                if pr is None or pu is None or ph is None:
                    check(False, f"探针点出界/缺失（{s}#{i}）")
                    probe_ok = False
                    continue
                d_ue_rust = max(abs(pu[0] - pr[0]), abs(pu[1] - pr[1]))
                d_ue_host = max(abs(pu[0] - ph[0]), abs(pu[1] - ph[1]))
                ok = d_ue_rust <= PIXEL_DELTA_TOL and d_ue_host <= PIXEL_DELTA_TOL
                if not ok:
                    check(False, f"探针超差（{s}#{i}）: ue-rust={d_ue_rust:.3e} ue-host={d_ue_host:.3e}")
                    probe_ok = False
                rows.append({
                    "index": i,
                    "world": list(lm),
                    "pixel_rurix": [float(pr[0]), float(pr[1])],
                    "pixel_ue5": [float(pu[0]), float(pu[1])],
                    "pixel_host_ref": [float(ph[0]), float(ph[1])],
                    "pixel_delta": float(max(d_ue_rust, d_ue_host)),
                    "pass": bool(ok),
                })
            app_probes.append({"scene_id": s, "landmarks": rows})
    checks["application_probe_dual_end"] = probe_ok
    check(probe_ok, "应用层探针超差或缺失")

    # ---- 三重绑定字段登记面 ----
    base_commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    ts_now = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    session_run_id = f"g10ab-{ts_now}"
    param_digest_joint = ""
    if tri_ok:
        joint = sorted(digests[s]["rust"] for s in G10_5_SCENES)
        param_digest_joint = hashlib.sha256("".join(joint).encode("ascii")).hexdigest()
    bind_ok = bool(param_digest_joint) and bool(base_commit) and bool(session_run_id)
    checks["three_binding_fields_registered"] = bind_ok
    check(bind_ok, "三重绑定字段登记面不齐")

    # ---- RED 臂①：单端参数漂移 ⇒ 三方不等检出（真实 bistro 契约）----
    red1 = False
    try:
        p = CORPUS / "contract_params_bistro_interior.json"
        c_doc = json.loads(p.read_text(encoding="utf-8"))
        c_doc["camera"]["fov_y_deg"] += 1e-12
        d_drift = param_digest_rurix(parse_contract_rurix(json.dumps(c_doc)))
        red1 = d_drift != digests.get("bistro-interior", {}).get("rust")
    except ContractError:
        red1 = False
    checks["red_param_drift_detected"] = red1
    check(red1, "RED 臂失效：参数漂移未引起 digest 不等")

    # ---- RED 臂②：错误共轭注入 ⇒ 探针超差检出（bistro 一般旋转面）----
    c_bi = parse_contract_rurix((CORPUS / "contract_params_bistro_interior.json").read_text(encoding="utf-8"))
    px_defective = _ue_project(c_bi, LANDMARKS_SPEC["bistro-interior"], _quat_contract_to_ue_defective)
    px_fixed = _ue_project(c_bi, LANDMARKS_SPEC["bistro-interior"], _quat_contract_to_ue_errata)
    px_host_bi = project_landmarks_host(c_bi, LANDMARKS_SPEC["bistro-interior"])
    cmp_def = [(a, b) for a, b in zip(px_defective, px_host_bi) if a is not None and b is not None]
    d_def = max(max(abs(a[0] - b[0]), abs(a[1] - b[1])) for a, b in cmp_def) if cmp_def else float("inf")
    d_fix = max(
        max(abs(a[0] - b[0]), abs(a[1] - b[1]))
        for a, b in zip(px_fixed, px_host_bi)
        if a is not None and b is not None
    )
    checks["red_wrong_conjugation_detected"] = d_def > PIXEL_DELTA_TOL and d_fix <= PIXEL_DELTA_TOL
    check(
        checks["red_wrong_conjugation_detected"],
        f"RED 臂失效：错误共轭注入未超差（d_def={d_def:.3e}）或修订式自身超差（d_fix={d_fix:.3e}）",
    )
    note(f"错误共轭注入实测超差 d_def={d_def:.6e} px（>1e-3 检出）；修订式 d_fix={d_fix:.6e} px")

    # ---- RED 臂③：标志物字面漂移注入 ⇒ 对账检出 ----
    tampered = list(LANDMARKS_SPEC["cornell-box"])
    tampered[0] = (tampered[0][0] + 0.1, tampered[0][1], tampered[0][2])
    spec_lm = _spec_landmark_literals()
    checks["red_landmark_tamper_detected"] = spec_lm.get("cornell-box") != tampered
    check(checks["red_landmark_tamper_detected"], "RED 臂失效：标志物漂移未被对账检出")

    # ---- RED 臂④：陈旧绑定冒充 ⇒ 三重绑定核验拒绝 ----
    synth = {
        "status": "pass",
        "phase_g10_5_pass": True,
        "base_commit": base_commit,
        "contract_report": {
            "param_digest": param_digest_joint,
            "param_digest_rurix": param_digest_joint,
            "param_digest_ue5": param_digest_joint,
            "session_run_id": session_run_id + "-stale",
        },
    }
    red4 = not verify_three_binding(synth, param_digest_joint, session_run_id, base_commit)
    checks["red_stale_binding_detected"] = red4
    check(red4, "RED 臂失效：陈旧 session_run_id 冒充未被三重绑定核验拒绝")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    first = G10_5_SCENES[0]
    contract_report = {
        "phase": "g10.5",
        "scenes": {
            s: {
                "params_path": f"milestones/g10/corpus/contract_params_{s.replace('-', '_')}.json",
                "param_digest": f"sha256:{digests[s]['rust']}" if s in digests and "rust" in digests[s] else "",
            }
            for s in G10_5_SCENES
            if s in digests
        },
        "param_digest_rurix": f"sha256:{digests[first]['rust']}" if tri_ok else "",
        "param_digest_ue5": f"sha256:{digests[first]['ue5']}" if tri_ok else "",
        "param_digest": f"sha256:{param_digest_joint}" if tri_ok else "",
        "param_digest_note": "param_digest = sha256(双场景各自 digest 字典序拼接)——联合登记值；逐场景 digest 见 scenes[]（逐场景三方相等实测）",
        "dual_digest_equal": tri_ok,
        "session_run_id": session_run_id,
        "application_probes": app_probes,
        "pixel_delta_tol": PIXEL_DELTA_TOL,
        "rurix_side_note": "Rurix 端 = Rust 第三实现 g10_5_scene_render（RXS-0384 L3 同字面；--contract-digest / --project-landmarks 真跑）",
        "ue_side_note": "UE 端 = 进程内嵌 CPython（g10_param_contract.py 同一源文件）实跑 digest + as-built 相机读回探针（g10_5_probe_landmarks.py，载体内嵌 CPython 钉死字面兑现）",
    }

    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G10.5",
        "phase": "g10.5",
        "phase_g10_2_pass": True,
        "phase_g10_5_pass": all_pass,
        "numeric_step": NUMERIC_STEP_G10_5,
        "source_ref": SOURCE_REF + ";spec/visual_comparison.md RXS-0390;RFC-0026 v1.1 章 E",
        "base_commit": base_commit,
        "host_section_pass": host_pass,
        "device_section_state": "executed",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS_G10_5},
        "commands": COMMANDS,
        "contract_report": contract_report,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts_now,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts_now}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS_G10_5)} device=executed phase=g10.5")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（双端核验期：双场景三方 digest 相等 param_digest=sha256:{param_digest_joint[:16]}… "
            f"+ 应用层探针逐点 ≤1e-3 px + 三重绑定字段登记 session_run_id={session_run_id} + RED 四臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--phase", choices=["g10.2", "g10.5"])
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    if args.phase is None:
        print(f"[{TAG}] FAIL: 缺 --phase（g10.2 骨架期 / g10.5 双端核验期）", file=sys.stderr)
        return 2
    if args.phase == "g10.5":
        return run_g10_5()

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    contract_report: dict = {}

    checks["spec_rxs0384_clause_on_tree"] = False
    if SPEC_PATH.is_file():
        import re as _re
        checks["spec_rxs0384_clause_on_tree"] = (
            _re.search(r"^###\s+RXS-0384\b", SPEC_PATH.read_text(encoding="utf-8"), _re.MULTILINE)
            is not None
        )
    check(checks["spec_rxs0384_clause_on_tree"], "spec/visual_comparison.md 缺 RXS-0384 条款头")

    checks["byte_layout_matches_spec"] = leg_byte_layout_matches_spec() if SPEC_PATH.is_file() else False

    text = PARAMS_PATH.read_text(encoding="utf-8")
    d_rurix = ""
    d_ue5 = ""
    try:
        d_rurix = param_digest_rurix(parse_contract_rurix(text))
        checks["rurix_side_parse_digest"] = True
    except ContractError as e:
        check(False, f"Rurix 侧解析失败: {e}")
    try:
        d_ue5 = param_digest_ue5(PARAMS_PATH)
        checks["ue_side_parse_digest"] = True
    except ContractError as e:
        check(False, f"UE 侧解析失败: {e}")

    dual_equal = bool(d_rurix) and d_rurix == d_ue5
    checks["dual_digest_compare_face"] = dual_equal
    check(dual_equal, f"双端 digest 不等: rurix={d_rurix[:16]}… ue5={d_ue5[:16]}…")
    if dual_equal:
        note(f"双端 digest 相等（解析一致）: sha256:{d_rurix}")

    checks["boundary_float_corpus_consistent"] = leg_boundary_corpus()

    # RED 臂①：单端参数漂移注入 ⇒ digest 不等必须检出。
    drifted = json.loads(text)
    drifted["camera"]["fov_y_deg"] = drifted["camera"]["fov_y_deg"] + 1e-12
    d_drift = param_digest_rurix(parse_contract_rurix(json.dumps(drifted)))
    checks["red_param_drift_detected"] = d_drift != d_ue5
    check(checks["red_param_drift_detected"], "单端参数漂移未引起 digest 不等（判定失效）")
    if checks["red_param_drift_detected"]:
        note(f"RED 检出 red_param_drift: {d_drift[:16]}… ≠ ue5 {d_ue5[:16]}…")

    # RED 臂②：schema 外字段注入必拒（双端各拒；Rurix 侧本脚本直验，UE 侧经子进程非零退出旁证）。
    extra = json.loads(text)
    extra["camera"]["hdr_enable"] = True
    checks["red_schema_extra_field_detected"] = _expect_reject(
        json.dumps(extra), "red_schema_extra_field"
    )

    # RED 臂③：非单位四元数注入必拒。
    badq = json.loads(text)
    badq["camera"]["orientation_quat"] = [1.0, 0.1, 0.0, 0.0]
    checks["red_nonunit_quat_detected"] = _expect_reject(
        json.dumps(badq), "red_nonunit_quat"
    )

    # RED 臂④：NaN 注入必拒（json NaN 字面 → f64 合法性谓词拒绝）。
    nan_text = text.replace('"ev100": 0.0', '"ev100": NaN', 1)
    check("NaN" in nan_text, "NaN 注入语料构造失效")
    checks["red_nan_injection_detected"] = "NaN" in nan_text and _expect_reject(
        nan_text, "red_nan_injection"
    )

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES
    contract_report = {
        "phase": "g10.2",
        "params_path": str(PARAMS_PATH.relative_to(ROOT)).replace("\\", "/"),
        "ue_parser_path": str(UE_PARSER.relative_to(ROOT)).replace("\\", "/"),
        "param_digest_rurix": f"sha256:{d_rurix}" if d_rurix else "",
        "param_digest_ue5": f"sha256:{d_ue5}" if d_ue5 else "",
        "param_digest": f"sha256:{d_rurix}" if dual_equal else "",
        "dual_digest_equal": dual_equal,
        "rurix_side_note": "Rurix 侧骨架 = host Python 参考解析器（本脚本内独立实现，RXS-0384 L3 同字面）；Rust 消费面归 G10.5 实现波（RFC-0026 §4.6 骨架期登记口径）",
    }

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G10.2",
        "phase": "g10.2",
        "phase_g10_2_pass": all_pass,
        "phase_g10_5_pass": False,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "contract_report": contract_report,
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
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable phase=g10.2")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS（骨架期：双端 schema 各一份 + digest 比对面就位 param_digest=sha256:{d_rurix[:16]}… + RED 四臂全检出；phase_g10_5_pass=false 不充双端核验期绿）")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
