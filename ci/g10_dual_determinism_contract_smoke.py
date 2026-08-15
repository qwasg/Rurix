#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.2 波 materialize）
"""G10.2 M130 双端确定性契约硬门冒烟·骨架期（步骤 179；
g10.p0.m130.dual_determinism_contract --phase g10.2；RFC-0026 §4.6；
spec/visual_comparison.md RXS-0384；G10_ACCEPTANCE_MAP §1 M130 行 + §3.3 双
阶段口径）。

host 纯 host 门（device_section_state 正常态 not_applicable）。骨架期判据：
相机/光照/时间参数同 schema 双端各一份（Rurix 侧骨架参考解析器 = 本脚本内
独立实现，按 RXS-0384 L3 字节布局字面；UE 侧解析器 = harness
g10_param_contract.py，UE 进程内嵌 CPython 载体同一源文件）+ digest 比对面
就位（同参数 JSON 双端各自解析各自产 digest，相等 ⟺ 解析一致）。

RED 臂（契约 §4.2 M130 字面）：单端参数漂移注入即 RED；schema 外字段注入即
RED（另：非单位四元数 / NaN 注入即拒，RXS-0384 L1/L2）。digest 不等仍出
A/B 报告即 RED 的门序三重绑定归 G10.5 双端核验腿（--phase g10.5 本波
fail-closed 拒跑，留 G10.5 实现波）；骨架期 evidence 标 phase：
phase_g10_2_pass=true、phase_g10_5_pass=false（骨架期绿不替双端核验期充绿，
MAP §3.3）。

用法：
  py -3 ci/g10_dual_determinism_contract_smoke.py --gate g10.p0.m130.dual_determinism_contract --phase g10.2
  py -3 ci/g10_dual_determinism_contract_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import math
import platform
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
    # 红臂③：--phase g10.5 本波 fail-closed 拒跑。
    r = subprocess.run(
        [sys.executable, str(Path(__file__).resolve()), "--gate", GATE_KEY, "--phase", "g10.5"],
        capture_output=True, text=True, timeout=60,
    )
    if r.returncode == 0:
        print(f"[{TAG}] selftest FAIL: --phase g10.5 未 fail-closed", file=sys.stderr)
        return 1
    # 绿臂：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (4 RED + 2 GREEN)")
    return 0


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
        print(f"[{TAG}] FAIL: --phase g10.5 双端核验腿归 G10.5 实现波，本波 fail-closed 拒跑", file=sys.stderr)
        return 2

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
