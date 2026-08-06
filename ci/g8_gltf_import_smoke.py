#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.3 M81 gltf_import 硬门冒烟(步骤 106;g8.p0.m81.gltf_import;
RFC-0020 §4.1/§4.6;spec/asset_pipeline.md RXS-0332~0333)。

host 纯 host 门(device 段 not_applicable)。判据(G8_ACCEPTANCE_MAP M81):

  锁定扩展集内 accept fixtures 全部导入且 canonical 六表 count/digest == golden;
  越界扩展 / 非法 accessor 范围 / 缺失必需 buffer 三 reject fail-closed;
  不得静默丢字段或只验证 JSON 可解析。

checks.* ≥10 项布尔(缺一 FAIL)。退出码判定(非 grep stdout)。
evidence 仍如实落盘(红不充绿)。

用法:
  py -3 ci/g8_gltf_import_smoke.py --gate g8.p0.m81.gltf_import
  py -3 ci/g8_gltf_import_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
ACCEPT_DIR = ROOT / "conformance" / "asset" / "gltf" / "accept"
REJECT_DIR = ROOT / "conformance" / "asset" / "gltf" / "reject"

GATE_KEY = "g8.p0.m81.gltf_import"
NUMERIC_STEP = 106
TABLES = ("scenes", "nodes", "meshes", "primitives", "materials", "textures")

FAILURES: list[str] = []
NOTES: list[str] = []


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


def build_rxcook() -> Path:
    print("[g8_m81] cargo build -p rurix-asset --bin rxcook")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurix-asset", "--bin", "rxcook", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[g8_m81] FAIL cargo build:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / ("rxcook.exe" if sys.platform == "win32" else "rxcook")
    if not exe.is_file():
        print(f"[g8_m81] FAIL rxcook 产物缺失: {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def cargo_gltf_tests() -> bool:
    r = subprocess.run(
        ["cargo", "test", "-p", "rurix-asset", "--lib", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"cargo test -p rurix-asset --lib 失败:\n{r.stdout}\n{r.stderr}")
        return False
    if "test result: ok" not in r.stdout and "passed" not in r.stdout:
        # --quiet 可能几乎无 stdout;以 returncode 为准。
        note("cargo test quiet: rc=0")
    return True


def run_import(exe: Path, path: Path) -> tuple[int, str, str]:
    r = subprocess.run(
        [str(exe), "import-gltf", str(path), "--emit-digest"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return r.returncode, r.stdout, r.stderr


def extract_tables(stdout: str) -> dict | None:
    """从 --emit-digest 输出中提取六表 JSON(忽略尾部 status= 行)。"""
    lines: list[str] = []
    for line in stdout.splitlines():
        if line.startswith("status=") or line.startswith("coverage_"):
            break
        lines.append(line)
    blob = "\n".join(lines).strip()
    if not blob:
        return None
    try:
        return json.loads(blob)
    except json.JSONDecodeError:
        return None


def accept_fixtures() -> list[Path]:
    files = sorted(
        [p for p in ACCEPT_DIR.iterdir() if p.suffix.lower() in (".gltf", ".glb")]
    )
    return files


def leg_accept_and_golden(exe: Path) -> None:
    files = accept_fixtures()
    check(len(files) >= 3, f"accept fixtures < 3: {len(files)}")
    for p in files:
        rc1, out1, err1 = run_import(exe, p)
        check(rc1 == 0, f"accept_import: {p.name} rc={rc1} err={err1.strip()}")
        doc1 = extract_tables(out1)
        check(doc1 is not None, f"accept_parse: {p.name} digest JSON 不可解析")
        if doc1 is None:
            continue
        for t in TABLES:
            check(t in doc1, f"accept_tables: {p.name} missing {t}")
        golden_path = p.with_suffix(".golden.json")
        check(golden_path.is_file(), f"golden_missing: {golden_path.name}")
        if not golden_path.is_file():
            continue
        golden = json.loads(golden_path.read_text(encoding="utf-8"))
        for t in TABLES:
            if t not in doc1 or t not in golden:
                continue
            check(
                doc1[t].get("count") == golden[t].get("count"),
                f"counts_golden: {p.name}.{t} got={doc1[t].get('count')} exp={golden[t].get('count')}",
            )
            check(
                doc1[t].get("digest") == golden[t].get("digest"),
                f"digests_golden: {p.name}.{t} mismatch",
            )
        rc2, out2, _ = run_import(exe, p)
        check(rc2 == 0, f"double_import: {p.name} second rc={rc2}")
        doc2 = extract_tables(out2)
        check(doc1 == doc2, f"double_import_stable: {p.name} digests differ")
        check(
            "coverage_complete=true" in out1,
            f"no_silent_field_drop: {p.name} coverage_complete!=true",
        )


def leg_rejects(exe: Path) -> None:
    cases = [
        ("reject_ext_outside_allowlist.gltf", "extension_not_allowed", "reject_extension"),
        ("reject_accessor_oob.gltf", "accessor_oob", "reject_accessor"),
        ("reject_missing_buffer.gltf", "missing_buffer", "reject_missing_buffer"),
    ]
    for name, kind, tag in cases:
        p = REJECT_DIR / name
        check(p.is_file(), f"{tag}: fixture missing {name}")
        if not p.is_file():
            continue
        rc, out, err = run_import(exe, p)
        check(rc != 0, f"{tag}: {name} unexpectedly ok")
        combined = out + "\n" + err
        check(
            f"error_kind={kind}" in combined or f'"error_kind":"{kind}"' in combined,
            f"{tag}: {name} expected kind={kind}, got:\n{combined}",
        )

    # 追加 reject 语料全拒
    extras = [
        p
        for p in REJECT_DIR.iterdir()
        if p.suffix == ".gltf"
        and p.name
        not in {
            "reject_ext_outside_allowlist.gltf",
            "reject_accessor_oob.gltf",
            "reject_missing_buffer.gltf",
        }
    ]
    for p in extras:
        rc, _, err = run_import(exe, p)
        check(rc != 0, f"reject_extras: {p.name} unexpectedly ok ({err})")


def leg_not_json_parse_only(exe: Path) -> None:
    """红自检:JSON 可解析但语义非法的 fixture 必须被 validate 层拒。"""
    # accessor OOB 文档是合法 JSON,但必须 RED。
    p = REJECT_DIR / "reject_accessor_oob.gltf"
    text = p.read_text(encoding="utf-8")
    try:
        json.loads(text)
        parseable = True
    except json.JSONDecodeError:
        parseable = False
    check(parseable, "not_json_parse_only: accessor_oob fixture should be parseable JSON")
    rc, out, err = run_import(exe, p)
    check(rc != 0, "not_json_parse_only: accessor_oob must fail closed at validate layer")
    combined = out + err
    check(
        "accessor_oob" in combined,
        f"not_json_parse_only: expected accessor_oob kind, got:\n{combined}",
    )


def write_evidence(results: dict, host_ok: bool) -> Path:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ev = {
        "schema_version": 1,
        "subject": "g8_m81_gltf_import",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M81",
        "wave": "G8.3",
        "numeric_step": NUMERIC_STEP,
        "source_ref": "RFC-0020 §4.1/§4.6;spec/asset_pipeline.md RXS-0332~0333",
        "host_section_pass": host_ok,
        "device_section_state": "not_applicable",
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
        "notes": (
            "host 纯 host 门;device 段 not_applicable。"
            "11 项判据经 rxcook import-gltf --emit-digest 端到端 +"
            " cargo test -p rurix-asset --lib 前置。"
            "三 reject(extension/accessor_oob/missing_buffer)独立 fail-closed;"
            "追加 reject(dup key/node cycle)计入 reject_extras。"
            "治理热点(ledger/pr-smoke/check_schemas/budget/CI_GATES/CONTRACT/"
            "CAPABILITY_MATRIX/traceability)本批未接线,另批合入。"
            + (("; " + "; ".join(NOTES)) if NOTES else "")
        ),
    }
    path = EVIDENCE_DIR / f"g8_m81_gltf_import_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m81] evidence 落盘: {path.relative_to(ROOT)}")
    return path


def selftest() -> None:
    """反 YAML-only:合成数据喂纯判定层,证明关键断言能红。"""
    check(False, "selftest: 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print("[g8_m81] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()

    bogus = '{"scenes": {"count": 1, "digest": "aa"}}'
    doc = json.loads(bogus)
    check(
        doc.get("scenes", {}).get("digest")
        == "0000000000000000000000000000000000000000000000000000000000000000",
        "selftest: 合成 digest 不等(证明 golden 比对能红)",
    )
    if len(FAILURES) != 1:
        print("[g8_m81] selftest FAIL: digest 比对未能判红", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()

    # extract_tables 对非 JSON 返回 None
    assert extract_tables("not json\nstatus=ok") is None
    # gate key 字面
    assert GATE_KEY == "g8.p0.m81.gltf_import"
    assert NUMERIC_STEP == 106
    print("[g8_m81] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")


def main() -> int:
    parser = argparse.ArgumentParser(description="G8.3 M81 gltf_import 硬门冒烟")
    parser.add_argument("--gate", default=GATE_KEY, help="symbolic gate key")
    parser.add_argument("--selftest", action="store_true", help="反 YAML-only 红绿自检")
    args = parser.parse_args()

    if args.selftest:
        selftest()
        return 0

    if args.gate != GATE_KEY:
        check(False, f"--gate `{args.gate}` ≠ canonical key `{GATE_KEY}`")

    exe = build_rxcook()
    tests_ok = cargo_gltf_tests()
    leg_accept_and_golden(exe)
    leg_rejects(exe)
    leg_not_json_parse_only(exe)

    results = {
        "cargo_gltf_tests_green": tests_ok and not any("cargo test" in f for f in FAILURES),
        "accept_all_import_green": not any("accept_import" in f for f in FAILURES),
        "counts_equal_golden": not any("counts_golden" in f for f in FAILURES),
        "digests_equal_golden": not any("digests_golden" in f for f in FAILURES),
        "double_import_digest_stable": not any("double_import" in f for f in FAILURES),
        "reject_extension_fail_closed": not any("reject_extension" in f for f in FAILURES),
        "reject_accessor_fail_closed": not any("reject_accessor" in f for f in FAILURES),
        "reject_missing_buffer_fail_closed": not any(
            "reject_missing_buffer" in f for f in FAILURES
        ),
        "reject_extras_fail_closed": not any("reject_extras" in f for f in FAILURES),
        "no_silent_field_drop": not any("no_silent_field_drop" in f for f in FAILURES),
        "not_json_parse_only": not any("not_json_parse_only" in f for f in FAILURES),
    }

    # gate key 错也计入失败
    if any("canonical key" in f for f in FAILURES):
        for k in results:
            results[k] = False

    host_ok = all(results.values()) and not FAILURES
    write_evidence(results, host_ok)

    if FAILURES:
        print("[g8_m81] FAIL:", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        return 1

    n = sum(1 for v in results.values() if v)
    print(f"[g8_m81] PASS ({n}/{len(results)} checks; numeric_step={NUMERIC_STEP})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
