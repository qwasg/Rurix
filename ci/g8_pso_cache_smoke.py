#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.2 M30 pso_cache 硬门冒烟(g8.p0.m30.pso_cache;
RFC-0019 §4.1.4;spec/vulkan_backend.md RXS-0314~0316)。

host 段(恒跑,不触 GPU):
  collector-only ×2 → 与 golden 全等 + 双跑逐字节相等。

device 段(gate real;`RURIX_REQUIRE_REAL=1` 翻硬红,缺则 SKIP=dev-env-degrade):
  cold / warm 零 stall / warm --drop-key 能红 / tamper 四轴 fail-closed /
  binary·fallback 诚实律 / validation 零错。各 device 腿独立 temp 目录串行跑
  `vk_pso_cache`(stdout JSON)。

验收判据(G8_ACCEPTANCE_MAP §2 M30 行 + 设计案 checks.* 14 项,缺一 FAIL):
  collector_key_set_equals_golden / key_generation_deterministic /
  cold_build_count_equals_keyset_size / warm_fresh_process_zero_stalls /
  warm_all_keys_hit_persisted / stall_counter_can_be_nonzero /
  tamper_schema_fail_closed_rebuild / tamper_version_fail_closed_rebuild /
  tamper_driver_identity_fail_closed_rebuild / tamper_keyset_fail_closed_rebuild /
  no_false_hit_after_tamper / binary_branch_taken_when_capable /
  fallback_branch_honest_when_incapable / validation_zero_errors。

用法:
  py -3 ci/g8_pso_cache_smoke.py --gate g8.p0.m30.pso_cache
  py -3 ci/g8_pso_cache_smoke.py --selftest
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
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_m30_pso_cache_evidence_schema.json"
GOLDEN = ROOT / "src" / "rurix-rt" / "tests" / "pso" / "pso_keys.golden.json"

GATE_KEY = "g8.p0.m30.pso_cache"
NUMERIC_STEP = 100
SOURCE_REF = "RFC-0019 §4.1.4;spec/vulkan_backend.md RXS-0314~0316"
TAG = "g8_m30"

# acceptance 文案 "driver identity" → harness tamper 轴名 driver_uuid →
# rebuild_reason = device_identity。
TAMPER_AXES: list[tuple[str, str, str]] = [
    ("schema", "schema", "tamper_schema_fail_closed_rebuild"),
    ("version", "version", "tamper_version_fail_closed_rebuild"),
    ("driver_uuid", "device_identity", "tamper_driver_identity_fail_closed_rebuild"),
    ("keyset", "keyset", "tamper_keyset_fail_closed_rebuild"),
]

CHECK_KEYS = [
    "collector_key_set_equals_golden",
    "key_generation_deterministic",
    "cold_build_count_equals_keyset_size",
    "warm_fresh_process_zero_stalls",
    "warm_all_keys_hit_persisted",
    "stall_counter_can_be_nonzero",
    "tamper_schema_fail_closed_rebuild",
    "tamper_version_fail_closed_rebuild",
    "tamper_driver_identity_fail_closed_rebuild",
    "tamper_keyset_fail_closed_rebuild",
    "no_false_hit_after_tamper",
    "binary_branch_taken_when_capable",
    "fallback_branch_honest_when_incapable",
    "validation_zero_errors",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def extract_json(stdout: str) -> dict | None:
    """解析 stdout 整段或最后一段 JSON object。"""
    text = stdout.strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except Exception:
        pass
    # 自末尾回找最后一个以 `{` 起的对象(harness 人类日志走 stderr,但容错混入)。
    idx = text.rfind("\n{")
    if idx < 0:
        idx = text.rfind("{")
    else:
        idx += 1
    if idx < 0:
        return None
    try:
        return json.loads(text[idx:])
    except Exception:
        return None


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip()
    except Exception:
        return "unknown"


def build_harness() -> Path:
    print(f"[{TAG}] cargo build -p rurix-rt --features vulkan --bin vk_pso_cache")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-rt", "--features", "vulkan", "--bin", "vk_pso_cache"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[{TAG}] FAIL cargo build:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / (
        "vk_pso_cache.exe" if sys.platform == "win32" else "vk_pso_cache"
    )
    if not exe.is_file():
        print(f"[{TAG}] FAIL harness 产物缺失: {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def run_harness(
    exe: Path,
    args: list[str],
    *,
    device: bool,
    timeout: int = 900,
) -> tuple[int, dict | None, str, str]:
    """串行跑 harness;device 腿强制 RURIX_REQUIRE_REAL=1;validation 默认不强制开。"""
    env = dict(os.environ)
    if device:
        env["RURIX_REQUIRE_REAL"] = "1"
    cmd = [str(exe), *args]
    r = subprocess.run(
        cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        timeout=timeout,
    )
    doc = extract_json(r.stdout)
    return r.returncode, doc, r.stdout, r.stderr


def is_skip(doc: dict | None, stderr: str) -> bool:
    if doc and doc.get("device_state") == "skipped_dev_env":
        return True
    if "PSO: SKIP" in stderr or "skipped_dev_env" in stderr:
        return True
    return False


# ═══════════════════════ host 腿 ═══════════════════════


def leg_collector(exe: Path) -> tuple[bool, int]:
    """collector_key_set_equals_golden + key_generation_deterministic。"""
    code1, doc1, out1, err1 = run_harness(exe, ["--collector-only"], device=False)
    code2, doc2, out2, err2 = run_harness(exe, ["--collector-only"], device=False)
    check(code1 == 0 and code2 == 0, f"collector: 退出非零({code1}/{code2})\n{err1}\n{err2}")
    if doc1 is None or doc2 is None:
        check(False, "collector: stdout JSON 解析失败")
        return False, 0
    # 双跑逐字节(stdout 规范化后的 collector JSON 文本)。
    t1 = out1 if out1.endswith("\n") else out1 + "\n"
    t2 = out2 if out2.endswith("\n") else out2 + "\n"
    # 若 stdout 含前缀噪声,退化为规范化 dumps 比对。
    if extract_json(out1) is not None and out1.strip()[0] != "{":
        t1 = json.dumps(doc1, ensure_ascii=False, sort_keys=True)
        t2 = json.dumps(doc2, ensure_ascii=False, sort_keys=True)
    check(t1 == t2, "key_generation_deterministic: collector 双跑非逐字节相等")

    golden = json.loads(GOLDEN.read_text(encoding="utf-8"))
    check(
        doc1 == golden,
        "collector_key_set_equals_golden: collector JSON 与 golden 不全等",
    )
    n = len(golden.get("records", []))
    check(n == 5, f"collector: golden keyset 大小应为 5,实测 {n}")
    return len(FAILURES) == 0, n


# ═══════════════════════ device 腿 ═══════════════════════


def run_device_legs(exe: Path, keyset_size: int) -> tuple[str, dict]:
    """串行 device 腿。返回 (device_section_state, measured)。

    各腿独立 temp 目录(tamper/drop-key 会改写 store;warm 零 stall 与 drop-key 不得共享)。
    device 腿强制 RURIX_REQUIRE_REAL=1;harness SKIP → 硬红(不许充绿)。
    """
    measured: dict = {
        "branch": None,
        "pipeline_binary_capability": None,
        "runtime_compile_stalls_warm": None,
        "runtime_compile_stalls_drop": None,
        "validation_errors": 0,
        "device_name": None,
        "skip_reason": None,
    }
    # 本门 device 腿强制 REQUIRE_REAL=1(设计案 gate real);SKIP 翻硬红。
    os.environ["RURIX_REQUIRE_REAL"] = "1"

    def device_run(args: list[str]) -> tuple[int, dict | None, str, str]:
        return run_harness(exe, args, device=True)

    # ── warm 零 stall 腿(独立 dir)──
    with tempfile.TemporaryDirectory(prefix="g8_m30_warm_") as d_warm:
        warm_dir = Path(d_warm)
        code_c, doc_c, _, err_c = device_run(["--cold", str(warm_dir)])
        if is_skip(doc_c, err_c):
            reason = (doc_c or {}).get("reason") or err_c.strip() or "skipped_dev_env"
            measured["skip_reason"] = reason
            # 外层未置 REQUIRE_REAL 时诚实 SKIP 退 0;本门腿内已置位 → 硬红。
            if require_real():
                check(False, f"device SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP): {reason}")
            else:
                note(f"device skipped_dev_env: {reason}")
            return "skipped_dev_env", measured
        check(code_c == 0 and doc_c is not None, f"cold: 失败 rc={code_c}\n{err_c}")
        if not doc_c or code_c != 0:
            check(False, "cold_build_count: cold 未产出(级联)")
            check(False, "warm_fresh_process_zero_stalls: cold 未过(级联)")
            check(False, "warm_all_keys_hit_persisted: cold 未过(级联)")
            return "fail", measured
        measured["branch"] = doc_c.get("branch")
        measured["pipeline_binary_capability"] = doc_c.get("pipeline_binary_capability")
        measured["device_name"] = (doc_c.get("device") or {}).get("name")
        ve = doc_c.get("validation_errors")
        measured["validation_errors"] = max(
            measured["validation_errors"], int(0 if ve is None else ve)
        )
        pbc = doc_c.get("precache_build_count")
        check(
            pbc is not None and int(pbc) == keyset_size,
            f"cold_build_count: precache_build_count={pbc} ≠ keyset_size={keyset_size}",
        )

        code_w, doc_w, _, err_w = device_run(["--warm", str(warm_dir)])
        check(code_w == 0 and doc_w is not None, f"warm: 失败 rc={code_w}\n{err_w}")
        if doc_w:
            stalls_raw = doc_w.get("runtime_compile_stalls")
            measured["runtime_compile_stalls_warm"] = stalls_raw
            ve = doc_w.get("validation_errors")
            measured["validation_errors"] = max(
                measured["validation_errors"], int(0 if ve is None else ve)
            )
            if measured["branch"] is None:
                measured["branch"] = doc_w.get("branch")
                measured["pipeline_binary_capability"] = doc_w.get(
                    "pipeline_binary_capability"
                )
            stalls = -1 if stalls_raw is None else int(stalls_raw)
            check(stalls == 0, f"warm_fresh_process_zero_stalls: stalls={stalls} ≠ 0")
            per = doc_w.get("per_key") or []
            check(
                len(per) == keyset_size and all(k.get("hit") is True for k in per),
                f"warm_all_keys_hit_persisted: per_key hits 不全真({per})",
            )
        else:
            check(False, "warm_fresh_process_zero_stalls: warm JSON 缺失")
            check(False, "warm_all_keys_hit_persisted: warm JSON 缺失")

    # ── stall 能红腿(独立 dir:cold → warm --drop-key 0)──
    with tempfile.TemporaryDirectory(prefix="g8_m30_stall_") as d_stall:
        stall_dir = Path(d_stall)
        code_sc, doc_sc, _, err_sc = device_run(["--cold", str(stall_dir)])
        check(code_sc == 0 and doc_sc is not None, f"stall/cold: 失败 rc={code_sc}\n{err_sc}")
        code_sw, doc_sw, _, err_sw = device_run(
            ["--warm", str(stall_dir), "--drop-key", "0"]
        )
        check(code_sw == 0 and doc_sw is not None, f"stall/drop-key: 失败 rc={code_sw}\n{err_sw}")
        if doc_sw:
            stalls_raw = doc_sw.get("runtime_compile_stalls")
            measured["runtime_compile_stalls_drop"] = stalls_raw
            ve = doc_sw.get("validation_errors")
            measured["validation_errors"] = max(
                measured["validation_errors"], int(0 if ve is None else ve)
            )
            stalls = 0 if stalls_raw is None else int(stalls_raw)
            per = doc_sw.get("per_key") or []
            missed = [k for k in per if k.get("hit") is not True]
            check(
                stalls >= 1 and len(missed) >= 1,
                f"stall_counter_can_be_nonzero: stalls={stalls} missed={missed}",
            )
            for k in missed:
                check(
                    k.get("hit") is False,
                    f"stall_counter_can_be_nonzero: 被删/miss key hit 非 false: {k}",
                )
        else:
            check(False, "stall_counter_can_be_nonzero: drop-key warm JSON 缺失")

    # ── tamper 四轴(各轴独立 dir:cold → tamper)──
    false_hits_total = 0
    for axis, want_reason, _ck in TAMPER_AXES:
        with tempfile.TemporaryDirectory(prefix=f"g8_m30_tamper_{axis}_") as d_t:
            tdir = Path(d_t)
            code_tc, doc_tc, _, err_tc = device_run(["--cold", str(tdir)])
            check(
                code_tc == 0 and doc_tc is not None,
                f"tamper/{axis} cold: 失败 rc={code_tc}\n{err_tc}",
            )
            code_tt, doc_tt, _, err_tt = device_run(["--tamper", axis, str(tdir)])
            check(
                code_tt == 0 and doc_tt is not None,
                f"tamper/{axis}: 失败 rc={code_tt}\n{err_tt}",
            )
            if not doc_tt:
                false_hits_total = -1
                continue
            ve = doc_tt.get("validation_errors")
            measured["validation_errors"] = max(
                measured["validation_errors"], int(0 if ve is None else ve)
            )
            check(
                doc_tt.get("rebuilt") is True,
                f"tamper/{axis}: rebuilt 非 true(未 fail-closed 重建)",
            )
            check(
                doc_tt.get("rebuild_reason") == want_reason,
                f"tamper/{axis}: rebuild_reason={doc_tt.get('rebuild_reason')} "
                f"≠ {want_reason}",
            )
            fh_raw = doc_tt.get("false_hits")
            fh = 0 if fh_raw is None else int(fh_raw)
            false_hits_total = max(false_hits_total, fh) if false_hits_total >= 0 else -1
            check(fh == 0, f"tamper/{axis}: false_hits={fh} ≠ 0(误命中)")

    check(
        false_hits_total == 0,
        f"no_false_hit_after_tamper: 聚合 false_hits={false_hits_total}",
    )

    # ── 分支诚实律 ──
    cap = measured.get("pipeline_binary_capability")
    branch = measured.get("branch")
    if cap is True:
        check(
            branch == "binary",
            f"binary_branch_taken_when_capable: capability=true 但 branch={branch}",
        )
        # capable 面上 fallback 判据 N/A——记 true + note,禁止因未走 cache 假红。
        note(
            "fallback_branch_honest_when_incapable N/A-on-capable"
            f"(capability=true, branch={branch})"
        )
    elif cap is False:
        check(
            branch == "cache",
            f"fallback_branch_honest_when_incapable: capability=false 但 branch={branch}",
        )
        # incapable 面上 binary 强制律 N/A——记 true + note。
        note(
            "binary_branch_taken_when_capable N/A-on-incapable"
            f"(capability=false, branch={branch})"
        )
    else:
        check(False, "binary_branch_taken_when_capable: 未产出 capability/branch")
        check(False, "fallback_branch_honest_when_incapable: 未产出 capability/branch")

    check(
        measured["validation_errors"] == 0,
        f"validation_zero_errors: validation_errors={measured['validation_errors']}",
    )

    return "executed", measured


# ═══════════════════════ evidence ═══════════════════════


def results_from_failures(*, host_ok: bool, device_state: str, measured: dict) -> dict[str, bool]:
    def ok(*needles: str) -> bool:
        return not any(any(n in f for n in needles) for f in FAILURES)

    results = {
        "collector_key_set_equals_golden": ok(
            "collector_key_set_equals_golden", "collector:"
        ),
        "key_generation_deterministic": ok("key_generation_deterministic"),
        "cold_build_count_equals_keyset_size": ok("cold_build_count"),
        "warm_fresh_process_zero_stalls": ok("warm_fresh_process_zero_stalls"),
        "warm_all_keys_hit_persisted": ok("warm_all_keys_hit_persisted"),
        "stall_counter_can_be_nonzero": ok("stall_counter_can_be_nonzero", "stall/"),
        "tamper_schema_fail_closed_rebuild": ok("tamper/schema"),
        "tamper_version_fail_closed_rebuild": ok("tamper/version"),
        "tamper_driver_identity_fail_closed_rebuild": ok("tamper/driver_uuid"),
        "tamper_keyset_fail_closed_rebuild": ok("tamper/keyset"),
        "no_false_hit_after_tamper": ok("no_false_hit_after_tamper"),
        "binary_branch_taken_when_capable": ok("binary_branch_taken_when_capable"),
        "fallback_branch_honest_when_incapable": ok("fallback_branch_honest_when_incapable"),
        "validation_zero_errors": ok("validation_zero_errors"),
    }
    # N/A 面:capable → fallback 记 true;incapable → binary 记 true(禁止假绿/假红)。
    cap = measured.get("pipeline_binary_capability")
    if device_state == "executed" and cap is True:
        results["fallback_branch_honest_when_incapable"] = True
    elif device_state == "executed" and cap is False:
        results["binary_branch_taken_when_capable"] = True
    # host 红 / SKIP / fail:device checks 不得充绿。
    if not host_ok or device_state != "executed":
        for k in CHECK_KEYS:
            if k in ("collector_key_set_equals_golden", "key_generation_deterministic"):
                continue
            if device_state != "executed" or not host_ok:
                results[k] = False
    return results


def write_evidence(
    results: dict[str, bool],
    host_ok: bool,
    device_state: str,
    measured: dict,
) -> Path:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    notes_parts = [
        "driver/device 门;host=collector golden/确定性;device=cold/warm/drop-key/tamper 四轴"
        "(各腿独立 temp dir;RURIX_REQUIRE_REAL=1;validation 默认不强制开)。",
        f"measured: branch={measured.get('branch')}"
        f" capability={measured.get('pipeline_binary_capability')}"
        f" warm_stalls={measured.get('runtime_compile_stalls_warm')}"
        f" drop_stalls={measured.get('runtime_compile_stalls_drop')}"
        f" validation_errors={measured.get('validation_errors')}"
        f" device={measured.get('device_name')!r}.",
    ]
    if measured.get("skip_reason"):
        notes_parts.append(f"skip_reason={measured['skip_reason']}")
    notes_parts.extend(NOTES)
    ev = {
        "schema_version": 1,
        "subject": "g8_m30_pso_cache",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M30",
        "wave": "G8.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_ok,
        "device_section_state": device_state,
        "checks": results,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": " ".join(notes_parts),
    }
    path = EVIDENCE_DIR / f"g8_m30_pso_cache_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence 落盘: {path.relative_to(ROOT)}")
    return path


def validate_evidence(path: Path) -> None:
    try:
        import jsonschema
    except ImportError:
        check(False, "schema 校验: 缺 jsonschema 依赖(pip install -r requirements.txt)")
        return
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    doc = json.loads(path.read_text(encoding="utf-8"))
    errors = list(jsonschema.Draft7Validator(schema).iter_errors(doc))
    for e in errors:
        check(
            False,
            f"schema 校验: {'/'.join(str(p) for p in e.path)}: {e.message}",
        )
    if not errors:
        print(f"[{TAG}] evidence schema 自校验 PASS")


# ═══════════════════════ selftest ═══════════════════════


def selftest() -> None:
    """反 YAML-only:schema 在位 + 常量/判定层自检(不跑 cargo、不写 evidence)。"""
    if not SCHEMA_PATH.is_file():
        print(f"[{TAG}] selftest FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        sys.exit(1)
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    assert schema["properties"]["numeric_step"]["const"] == NUMERIC_STEP
    assert schema["properties"]["symbolic_gate_key"]["const"] == GATE_KEY
    assert schema["properties"]["subject"]["const"] == "g8_m30_pso_cache"
    assert "executed" in schema["properties"]["device_section_state"]["enum"]
    assert "skipped_dev_env" in schema["properties"]["device_section_state"]["enum"]
    for k in CHECK_KEYS:
        assert k in schema["properties"]["checks"]["required"], k

    check(False, "selftest: 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未正确记录", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()

    assert extract_json("not json") is None
    assert extract_json('{"a": 1}\n') == {"a": 1}
    assert extract_json('noise\n{"device_state": "skipped_dev_env"}\n') == {
        "device_state": "skipped_dev_env"
    }

    # tamper 轴 → rebuild_reason 映射自检
    assert ("driver_uuid", "device_identity", "tamper_driver_identity_fail_closed_rebuild") in (
        (a, r, c) for a, r, c in TAMPER_AXES
    )

    # 合成 stall 判据能红
    synth = {"runtime_compile_stalls": 0, "per_key": [{"hit": True}]}
    check(
        int(synth["runtime_compile_stalls"]) >= 1
        and any(k.get("hit") is not True for k in synth["per_key"]),
        "selftest: 合成 stall=0(证明 stall 断言能红)",
    )
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: stall 合成违例未被判红", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()

    print(f"[{TAG}] selftest PASS(schema+常量+判定层;未跑 cargo、未写 evidence)")


# ═══════════════════════ main ═══════════════════════


def main() -> int:
    parser = argparse.ArgumentParser(description="G8.2 M30 pso_cache 硬门冒烟")
    parser.add_argument("--gate", default=GATE_KEY, help="symbolic gate key")
    parser.add_argument("--selftest", action="store_true", help="反 YAML-only 红绿自检")
    args = parser.parse_args()

    if args.selftest:
        selftest()
        return 0

    if args.gate != GATE_KEY:
        check(False, f"--gate `{args.gate}` ≠ canonical key `{GATE_KEY}`")

    if not GOLDEN.is_file():
        check(False, f"golden 缺失: {GOLDEN}")
        print(f"[{TAG}] FAIL golden 缺失", file=sys.stderr)
        return 1

    exe = build_harness()
    host_ok, keyset_size = leg_collector(exe)
    if keyset_size == 0:
        keyset_size = 5

    measured: dict = {
        "branch": None,
        "pipeline_binary_capability": None,
        "runtime_compile_stalls_warm": None,
        "runtime_compile_stalls_drop": None,
        "validation_errors": 0,
        "device_name": None,
        "skip_reason": None,
    }
    device_state = "fail"
    if host_ok:
        device_state, measured = run_device_legs(exe, keyset_size)
    else:
        check(False, "device: host collector 未过,跳过 device 腿(不充绿)")

    results = results_from_failures(
        host_ok=host_ok, device_state=device_state, measured=measured
    )

    ev_path = write_evidence(results, host_ok, device_state, measured)
    validate_evidence(ev_path)

    for m in NOTES:
        print(f"[{TAG}] NOTE {m}")
    print(
        f"[{TAG}] measured branch={measured.get('branch')} "
        f"capability={measured.get('pipeline_binary_capability')} "
        f"warm_stalls={measured.get('runtime_compile_stalls_warm')} "
        f"drop_stalls={measured.get('runtime_compile_stalls_drop')} "
        f"validation_errors={measured.get('validation_errors')} "
        f"device_state={device_state}"
    )

    if FAILURES:
        print(f"[{TAG}] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        return 1

    if device_state == "skipped_dev_env":
        # 仅无 REQUIRE_REAL 时可达(腿内未翻硬红);诚实 SKIP 退 0。
        print(f"[{TAG}] SKIP device_state=skipped_dev_env(dev-env-degrade,退出 0)")
        return 0

    if device_state != "executed":
        print(f"[{TAG}] FAIL device_state={device_state}", file=sys.stderr)
        return 1

    print(f"[{TAG}] PASS (host collector + device 14 checks 全真;numeric_step={NUMERIC_STEP})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
