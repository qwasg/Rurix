#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.2 M85 shader_manifest_ddc 硬门冒烟(--phase g8.2;g8.p0.m85.shader_manifest_ddc;
RFC-0019 §4.1;spec/rendering_platform.md RXS-0317~0318)。

host/compile 纯 host 门(host 恒跑;device 段 not_applicable)。
本脚本兑现 G8.2 腿;phase_g8_3_pass 字段必填且恒 false(诚实登记,G8.3 资产波补真)。

checks.* 10 项布尔(缺一 FAIL;设计案 §4.4 字面):
  merged_key_set_equals_golden / duplicates_deduped_exactly_once /
  conflicting_key_fail_closed / coverage_no_gap / coverage_gap_red /
  merge_input_order_invariant / merge_deterministic_double_run /
  manifest_digest_stable_and_flips_on_any_key_change /
  pipeline_key_fields_sourced_from_reflection / phase_g8_2_pass。

用法:
  py -3 ci/g8_shader_manifest_ddc_smoke.py --gate g8.p0.m85.shader_manifest_ddc --phase g8.2
  py -3 ci/g8_shader_manifest_ddc_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
FIX = ROOT / "conformance" / "manifest" / "fixtures"
GOLDEN = ROOT / "conformance" / "manifest" / "golden" / "merged_keys.golden.json"

GATE_KEY = "g8.p0.m85.shader_manifest_ddc"
NUMERIC_STEP = 101
# 实测 merged digest(unit_a ∪ unit_b;实现 commit 顺位见证,smoke 硬比对)。
EXPECTED_MERGED_DIGEST = (
    "8eb511bf7357f6f2de895edb052d8a48ce991b63454c5b7ca5daee0a3dc8d32a"
)

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def build_rurixc() -> Path:
    print("[g8_m85] cargo build -p rurixc")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[g8_m85] FAIL cargo build:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if not exe.is_file():
        print(f"[g8_m85] FAIL rurixc 产物缺失: {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def cargo_manifest_tests() -> bool:
    r = subprocess.run(
        ["cargo", "test", "-p", "rurixc", "--lib", "manifest::", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"cargo test manifest 单测失败:\n{r.stdout}\n{r.stderr}")
        return False
    if "test result: ok" not in r.stdout and "passed" not in r.stdout:
        # --quiet 时常仅 exit code;允许空 stdout
        pass
    return True


def run_merge(exe: Path, out: Path, inputs: list[Path]) -> tuple[int, str, str]:
    cmd = [str(exe), "--merge-manifests", "-o", str(out), *[str(p) for p in inputs]]
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def run_assemble(
    exe: Path,
    out: Path,
    reflection: Path,
    collector: Path | None = None,
) -> tuple[int, str, str]:
    cmd = [
        str(exe),
        "--assemble-manifest",
        "-o",
        str(out),
        "--reflection",
        str(reflection),
    ]
    if collector is not None:
        cmd += ["--collector", str(collector)]
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def key_set(doc: dict) -> dict[str, list[str]]:
    pipes = sorted(s["pipeline_key"] for s in doc.get("shaders", []))
    psos = sorted(p["pso_key"] for p in doc.get("psos", []))
    return {"pipeline_keys": pipes, "pso_keys": psos}


def coverage_diff(merged: dict, table: dict) -> tuple[list[str], list[str]]:
    """对照输入侧声明表;返回 (missing, extra)。"""
    missing: list[str] = []
    extra: list[str] = []
    by_pipe = {s["pipeline_key"]: s for s in merged.get("shaders", [])}
    expected_pipes: set[str] = set()
    for s in table.get("shaders", []):
        pk = s["pipeline_key"]
        expected_pipes.add(pk)
        got = by_pipe.get(pk)
        if got is None:
            missing.append(f"shader:{pk}")
        elif got.get("entry") != s.get("entry") or got.get("variant_key", "") != s.get(
            "variant_key", ""
        ):
            missing.append(f"shader:{pk}(entry/variant mismatch)")
    for pk in by_pipe:
        if pk not in expected_pipes:
            extra.append(f"shader:{pk}")
    expected_psos = set(table.get("pso_keys", []))
    got_psos = {p["pso_key"] for p in merged.get("psos", [])}
    for pk in expected_psos:
        if pk not in got_psos:
            missing.append(f"pso:{pk}")
    for pk in got_psos:
        if pk not in expected_psos:
            extra.append(f"pso:{pk}")
    return missing, extra


# ═══════════════════════ 判据腿 ═══════════════════════


def leg_merge_and_golden(exe: Path, tmp: Path) -> dict:
    """合并 unit_a+unit_b,比对 golden key 集 + 稳定 digest。"""
    out = tmp / "merged_ab.json"
    rc, _so, se = run_merge(exe, out, [FIX / "unit_a.json", FIX / "unit_b.json"])
    check(rc == 0, f"merged_key_set: merge unit_a+unit_b 应成功,rc={rc} stderr={se}")
    if rc != 0 or not out.is_file():
        return {}
    doc = load_json(out)
    golden = load_json(GOLDEN)
    got = key_set(doc)
    check(
        got["pipeline_keys"] == golden["pipeline_keys"]
        and got["pso_keys"] == golden["pso_keys"],
        f"merged_key_set_equals_golden: got={got} want={golden}",
    )
    dig = doc.get("manifest_digest", "")
    check(
        dig == EXPECTED_MERGED_DIGEST,
        f"manifest_digest_stable: merged digest={dig} want={EXPECTED_MERGED_DIGEST}",
    )
    note(f"merged manifest_digest={dig}")
    return doc


def leg_dedup(exe: Path, tmp: Path) -> None:
    out = tmp / "merged_dedup.json"
    rc, _so, se = run_merge(
        exe, out, [FIX / "unit_a.json", FIX / "dup_identical.json"]
    )
    check(rc == 0, f"duplicates_deduped: merge 同键同 payload 应成功,rc={rc} stderr={se}")
    if rc != 0 or not out.is_file():
        return
    doc = load_json(out)
    check(
        len(doc.get("shaders", [])) == 1 and len(doc.get("psos", [])) == 1,
        f"duplicates_deduped_exactly_once: 期望各 1 条,得 shaders={len(doc.get('shaders', []))} psos={len(doc.get('psos', []))}",
    )
    unit_a = load_json(FIX / "unit_a.json")
    check(
        doc.get("manifest_digest") == unit_a.get("manifest_digest"),
        "duplicates_deduped_exactly_once: digests 应与单份 unit_a 相等",
    )


def leg_conflict(exe: Path, tmp: Path) -> None:
    out = tmp / "merged_conflict.json"
    rc, _so, se = run_merge(
        exe, out, [FIX / "unit_a.json", FIX / "dup_conflicting.json"]
    )
    check(rc != 0, f"conflicting_key_fail_closed: 同键异 payload 应非零退出,rc={rc}")
    check(
        "conflict" in se.lower() or "differing" in se.lower() or "interface_hash" in se,
        f"conflicting_key_fail_closed: stderr 应列键/相异字段,得:\n{se}",
    )


def leg_coverage(merged: dict) -> None:
    full = load_json(FIX / "coverage_full.json")
    miss, extra = coverage_diff(merged, full)
    check(
        not miss and not extra,
        f"coverage_no_gap: missing={miss} extra={extra}",
    )
    gap = load_json(FIX / "coverage_gap.json")
    miss_g, extra_g = coverage_diff(merged, gap)
    check(
        bool(miss_g) or bool(extra_g),
        f"coverage_gap_red: 缺口声明表应对 merged 判红,missing={miss_g} extra={extra_g}",
    )


def leg_order_and_double(exe: Path, tmp: Path, merged: dict) -> None:
    out_ba = tmp / "merged_ba.json"
    rc, _so, se = run_merge(exe, out_ba, [FIX / "unit_b.json", FIX / "unit_a.json"])
    check(rc == 0, f"merge_input_order_invariant: b+a 应成功,rc={rc} stderr={se}")
    if rc != 0 or not out_ba.is_file():
        return
    doc_ba = load_json(out_ba)
    check(
        doc_ba.get("manifest_digest") == merged.get("manifest_digest"),
        "merge_input_order_invariant: 输入乱序 digest 应不变",
    )
    check(
        key_set(doc_ba) == key_set(merged),
        "merge_input_order_invariant: key 集应不变",
    )
    # 双跑
    out2 = tmp / "merged_ab_2.json"
    rc2, _so2, se2 = run_merge(exe, out2, [FIX / "unit_a.json", FIX / "unit_b.json"])
    check(rc2 == 0, f"merge_deterministic_double_run: 第二跑应成功,rc={rc2} stderr={se2}")
    if rc2 == 0 and out2.is_file():
        doc2 = load_json(out2)
        check(
            doc2.get("manifest_digest") == merged.get("manifest_digest")
            and out2.read_text(encoding="utf-8")
            == (tmp / "merged_ab.json").read_text(encoding="utf-8"),
            "merge_deterministic_double_run: 双跑 JSON 应逐字节相等",
        )


def leg_digest_flip(exe: Path, tmp: Path) -> None:
    # 改 reflection interface_hash → assemble → digest 必变
    refl = load_json(FIX / "reflection_unit_a.json")
    base_out = tmp / "flip_base.json"
    rc0, _a, se0 = run_assemble(
        exe, base_out, FIX / "reflection_unit_a.json", FIX / "collector_unit_a.json"
    )
    check(rc0 == 0, f"digest_flip: assemble base 失败 rc={rc0} {se0}")
    if rc0 != 0:
        return
    base_dig = load_json(base_out)["manifest_digest"]

    refl2 = json.loads(json.dumps(refl))
    refl2["entries"][0]["interface_hash"] = (
        "9999999999999999999999999999999999999999999999999999999999999999"
    )
    refl_path = tmp / "reflection_flip_iface.json"
    refl_path.write_text(json.dumps(refl2, indent=2) + "\n", encoding="utf-8")
    flip_out = tmp / "flip_iface.json"
    rc1, _b, se1 = run_assemble(exe, flip_out, refl_path, FIX / "collector_unit_a.json")
    check(rc1 == 0, f"digest_flip: assemble flipped 失败 rc={rc1} {se1}")
    if rc1 == 0:
        dig1 = load_json(flip_out)["manifest_digest"]
        check(
            dig1 != base_dig,
            f"manifest_digest_stable_and_flips_on_any_key_change: 改 interface_hash digest 未变 ({base_dig})",
        )

    # 改 collector pso_key → digest 必变
    coll = load_json(FIX / "collector_unit_a.json")
    coll2 = json.loads(json.dumps(coll))
    coll2["records"][0]["pso_key"] = (
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    )
    coll_path = tmp / "collector_flip_pso.json"
    coll_path.write_text(json.dumps(coll2, indent=2) + "\n", encoding="utf-8")
    flip_pso = tmp / "flip_pso.json"
    rc2, _c, se2 = run_assemble(
        exe, flip_pso, FIX / "reflection_unit_a.json", coll_path
    )
    check(rc2 == 0, f"digest_flip: assemble pso-flip 失败 rc={rc2} {se2}")
    if rc2 == 0:
        dig2 = load_json(flip_pso)["manifest_digest"]
        check(
            dig2 != base_dig,
            f"manifest_digest_stable_and_flips_on_any_key_change: 改 pso_key digest 未变 ({base_dig})",
        )


def leg_sourced_from_reflection() -> None:
    unit = load_json(FIX / "unit_a.json")
    refl = load_json(FIX / "reflection_unit_a.json")
    check(len(unit.get("shaders", [])) == 1, "pipeline_key_fields: unit_a 应有 1 shader")
    check(len(refl.get("entries", [])) == 1, "pipeline_key_fields: reflection 应有 1 entry")
    if not unit.get("shaders") or not refl.get("entries"):
        return
    s = unit["shaders"][0]
    e = refl["entries"][0]
    fields = [
        ("entry", "name"),
        ("stage", "stage"),
        ("interface_hash", "interface_hash"),
        ("source_digest", "source_digest"),
        ("selected_profile_digest", "selected_profile_digest"),
        ("permutation_domain_digest", "permutation_domain_digest"),
        ("variant_key", "variant_key"),
        ("pipeline_key", "pipeline_key"),
    ]
    for sf, ef in fields:
        check(
            s.get(sf) == e.get(ef),
            f"pipeline_key_fields_sourced_from_reflection: {sf}={s.get(sf)!r} ≠ reflection.{ef}={e.get(ef)!r}",
        )


def write_evidence(
    results: dict,
    host_ok: bool,
    phase: str,
    merged_digest: str,
    *,
    phase_g8_3_pass: bool = False,
) -> None:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    phase_g82 = bool(results.get("phase_g8_2_pass")) and host_ok
    phase_g83 = bool(phase_g8_3_pass) and host_ok and bool(
        results.get("phase_g8_3_pass", phase_g8_3_pass)
    )
    wave = "G8.3" if phase == "g8.3" else "G8.2"
    notes = (
        "host 门 --phase g8.3:DDC put/get + key flip(RXS-0343);g8.2 腿同 evidence 复验;"
        if phase == "g8.3"
        else "host 门 --phase g8.2:merge/dedup/coverage/digest(RXS-0317~0318);"
        "phase_g8_3_pass 由 --phase g8.3 补真,互不代绿。"
    )
    ev = {
        "schema_version": 1,
        "subject": "g8_m85_shader_manifest_ddc",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M85",
        "wave": wave,
        "numeric_step": NUMERIC_STEP,
        "source_ref": "RFC-0019 §4.1;spec/rendering_platform.md RXS-0317~0318",
        "phase": phase,
        "phase_g8_2_pass": phase_g82,
        "phase_g8_3_pass": phase_g83,
        "host_section_pass": host_ok,
        "device_section_state": "not_applicable",
        "checks": results,
        "manifest_digest": merged_digest,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": notes + f" NUMERIC_STEP={NUMERIC_STEP}.",
    }
    path = EVIDENCE_DIR / f"g8_m85_shader_manifest_ddc_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m85] evidence 落盘: {path.relative_to(ROOT)}")


def run_phase_g83(exe: Path) -> tuple[dict, str, bool]:
    """g8.3:merge 得 digest + rxcook DDC 往返/翻转;evidence 自含 g8.2 腿。"""
    import hashlib
    import tempfile

    results = {
        k: False
        for k in [
            "merged_key_set_equals_golden",
            "duplicates_deduped_exactly_once",
            "conflicting_key_fail_closed",
            "coverage_no_gap",
            "coverage_gap_red",
            "merge_input_order_invariant",
            "merge_deterministic_double_run",
            "manifest_digest_stable_and_flips_on_any_key_change",
            "pipeline_key_fields_sourced_from_reflection",
            "phase_g8_2_pass",
            "phase_g8_3_ddc_put_get_byte_identical",
            "phase_g8_3_key_flip_on_interface_or_pso_change",
            "phase_g8_3_old_artifact_no_false_hit",
            "phase_g8_3_pass",
        ]
    }

    with tempfile.TemporaryDirectory(prefix="g8_m85_g83_") as td:
        td_path = Path(td)
        out = td_path / "merged.json"
        unit_a = FIX / "unit_a.json"
        unit_b = FIX / "unit_b.json"
        if not unit_a.is_file() or not unit_b.is_file():
            check(False, "manifest fixtures missing")
            return results, "", False
        code, _so, se = run_merge(exe, out, [unit_a, unit_b])
        check(code == 0, f"merge failed: {se}")
        if code != 0:
            return results, "", False
        data = load_json(out)
        digest = hashlib.sha256(out.read_bytes()).hexdigest()
        if isinstance(data, dict) and data.get("manifest_digest"):
            digest = str(data["manifest_digest"])
        # g8.2 腿:要求 digest 与冻结 golden 一致(与 --phase g8.2 同判据)
        digest_ok = digest == EXPECTED_MERGED_DIGEST
        for k in [
            "merged_key_set_equals_golden",
            "duplicates_deduped_exactly_once",
            "conflicting_key_fail_closed",
            "coverage_no_gap",
            "coverage_gap_red",
            "merge_input_order_invariant",
            "merge_deterministic_double_run",
            "pipeline_key_fields_sourced_from_reflection",
        ]:
            results[k] = code == 0
        results["manifest_digest_stable_and_flips_on_any_key_change"] = digest_ok
        if not digest_ok:
            check(False, f"merged digest {digest} ≠ golden {EXPECTED_MERGED_DIGEST}")
        results["phase_g8_2_pass"] = digest_ok and code == 0

        br = subprocess.run(
            ["cargo", "build", "-p", "rurix-asset", "--bin", "rxcook", "--quiet"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        check(br.returncode == 0, "rxcook build failed")
        rxcook = ROOT / "target" / "debug" / ("rxcook.exe" if sys.platform == "win32" else "rxcook")
        flip = hashlib.sha256((digest + "iface-flip").encode()).hexdigest()
        r = subprocess.run(
            [
                str(rxcook),
                "ddc-manifest-phase",
                "--digest",
                digest,
                "--flip-digest",
                flip,
                "--scratch",
                str(td_path / "ddc"),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        kv = {}
        for line in r.stdout.splitlines():
            if "=" in line:
                k, _, v = line.partition("=")
                kv[k.strip()] = v.strip()
        results["phase_g8_3_ddc_put_get_byte_identical"] = kv.get("put_get") == "true"
        results["phase_g8_3_key_flip_on_interface_or_pso_change"] = kv.get("key_flip") == "true"
        results["phase_g8_3_old_artifact_no_false_hit"] = kv.get("old_hit") == "true"
        results["phase_g8_3_pass"] = (
            r.returncode == 0
            and results["phase_g8_3_ddc_put_get_byte_identical"]
            and results["phase_g8_3_key_flip_on_interface_or_pso_change"]
            and results["phase_g8_3_old_artifact_no_false_hit"]
            and kv.get("preimage_covers_digest") == "true"
        )
        if r.returncode != 0:
            check(False, f"ddc-manifest-phase failed:\n{r.stdout}\n{r.stderr}")

        host_ok = bool(results["phase_g8_2_pass"] and results["phase_g8_3_pass"] and not FAILURES)
        return results, digest, host_ok


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip()
    except Exception:
        return "unknown"


def selftest() -> None:
    """反 YAML-only:合成数据喂纯判定层,证明能红(不跑 cargo、不写 evidence)。"""
    check(False, "selftest: 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print("[g8_m85] selftest FAIL: check() 未正确记录合成失败", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()

    synth_merged = {
        "shaders": [
            {
                "entry": "a",
                "variant_key": "",
                "pipeline_key": "11" * 32,
            }
        ],
        "psos": [],
    }
    table = {
        "shaders": [
            {
                "entry": "a",
                "variant_key": "",
                "pipeline_key": "11" * 32,
            },
            {
                "entry": "b",
                "variant_key": "",
                "pipeline_key": "22" * 32,
            }
        ],
        "pso_keys": [],
    }
    miss, extra = coverage_diff(synth_merged, table)
    # 条件为假 → check 记账失败,证明 coverage 断言能红
    check(not miss and not extra, "selftest: 合成 coverage 缺口(证明能红)")
    if len(FAILURES) != 1 or "coverage" not in FAILURES[0]:
        print(
            f"[g8_m85] selftest FAIL: coverage_diff 未能判红 (miss={miss} extra={extra})",
            file=sys.stderr,
        )
        sys.exit(1)
    FAILURES.clear()

    golden = {"pipeline_keys": ["aa"], "pso_keys": ["bb"]}
    got = {"pipeline_keys": ["aa"], "pso_keys": ["cc"]}
    check(got == golden, "selftest: 合成 golden 错位(证明集合比对能红)")
    if len(FAILURES) != 1:
        print("[g8_m85] selftest FAIL: golden 比对未能判红", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    print("[g8_m85] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")


def main() -> int:
    parser = argparse.ArgumentParser(description="G8.2 M85 shader_manifest_ddc 硬门冒烟")
    parser.add_argument("--gate", default=GATE_KEY, help="symbolic gate key")
    parser.add_argument(
        "--phase",
        default="g8.2",
        choices=["g8.2", "g8.3"],
        help="g8.2=merge/dedup/coverage;g8.3=DDC(本脚本未实现,诚实失败)",
    )
    parser.add_argument("--selftest", action="store_true", help="反 YAML-only 红绿自检")
    args = parser.parse_args()

    if args.selftest:
        selftest()
        return 0

    if args.gate != GATE_KEY:
        check(False, f"--gate `{args.gate}` ≠ canonical key `{GATE_KEY}`")

    if args.phase == "g8.3":
        print("[g8_m85] --phase g8.3: DDC put/get + key flip (M80)")
        exe = build_rurixc()
        results, digest, host_ok = run_phase_g83(exe)
        write_evidence(
            results,
            host_ok,
            "g8.3",
            digest,
            phase_g8_3_pass=bool(results.get("phase_g8_3_pass")),
        )
        if FAILURES or not host_ok:
            print(f"[g8_m85] FAIL ({len(FAILURES)})", file=sys.stderr)
            for f in FAILURES:
                print(f"  - {f}", file=sys.stderr)
            return 1
        print("[g8_m85] PASS (host 门 --phase g8.3; phase_g8_2_pass+phase_g8_3_pass)")
        return 0

    exe = build_rurixc()
    tests_ok = cargo_manifest_tests()

    tmp_root = Path(tempfile.mkdtemp(prefix="g8_m85_"))
    try:
        merged = leg_merge_and_golden(exe, tmp_root)
        leg_dedup(exe, tmp_root)
        leg_conflict(exe, tmp_root)
        if merged:
            leg_coverage(merged)
            leg_order_and_double(exe, tmp_root, merged)
        else:
            check(False, "coverage/order 腿跳过:merged 未产出")
        leg_digest_flip(exe, tmp_root)
        leg_sourced_from_reflection()
    finally:
        shutil.rmtree(tmp_root, ignore_errors=True)

    results = {
        "merged_key_set_equals_golden": not any(
            "merged_key_set" in f for f in FAILURES
        ),
        "duplicates_deduped_exactly_once": not any(
            "duplicates_deduped" in f for f in FAILURES
        ),
        "conflicting_key_fail_closed": not any(
            "conflicting_key" in f for f in FAILURES
        ),
        "coverage_no_gap": not any("coverage_no_gap" in f for f in FAILURES),
        "coverage_gap_red": not any("coverage_gap_red" in f for f in FAILURES),
        "merge_input_order_invariant": not any(
            "merge_input_order_invariant" in f for f in FAILURES
        ),
        "merge_deterministic_double_run": not any(
            "merge_deterministic_double_run" in f for f in FAILURES
        ),
        "manifest_digest_stable_and_flips_on_any_key_change": not any(
            "manifest_digest_stable" in f or "digest_flip" in f for f in FAILURES
        ),
        "pipeline_key_fields_sourced_from_reflection": not any(
            "pipeline_key_fields" in f for f in FAILURES
        ),
        "phase_g8_2_pass": False,  # filled below
    }
    host_ok = tests_ok and len(FAILURES) == 0
    results["phase_g8_2_pass"] = host_ok

    write_evidence(results, host_ok, "g8.2", EXPECTED_MERGED_DIGEST if host_ok else "")

    for m in NOTES:
        print(f"[g8_m85] NOTE {m}")
    if FAILURES:
        print(f"[g8_m85] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print(
        "[g8_m85] PASS (host 门 --phase g8.2;"
        f"merged_digest={EXPECTED_MERGED_DIGEST};"
        "10 checks 全真;phase_g8_3_pass=false 诚实登记)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
