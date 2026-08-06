#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.2 M29 shader_permutation 硬门冒烟(g8.p0.m29.shader_permutation;
RFC-0019 §4.3;spec/rendering_platform.md RXS-0308~0310)。

host/compile 纯 host 门(host 恒跑,check_* 风格;device 段 not_applicable)。
验收判据(G8_ACCEPTANCE_MAP §2 M29 行逐字):

  对固定 domain golden:canonical key 两次生成逐字节相等;合法组合集合与
  golden 集合全等;静态不可能组合全部被裁剪;预算 `limit == legal_count`
  为 GREEN、`limit == legal_count - 1` 为 RED;报告中的
  `enumerated/pruned/emitted` 满足 `enumerated == pruned + emitted`。
  不得以 M30/M31/M32/M85 任一结果代替。

checks.* 13 项布尔(缺一 FAIL):
  double_key_generation_byte_identical / legal_set_equals_golden /
  pruned_combinations_all_absent / axis_declaration_order_invariant /
  budget_equal_green / budget_minus_one_red /
  report_identity_enumerated_eq_pruned_plus_emitted /
  axis_contribution_report_on_red / select_valid_key_fills_variant_key /
  select_missing_key_deterministic_error /
  empty_domain_reflection_zero_drift / accept_corpus_green /
  reject_corpus_red_with_codes。

预算边界腿的固定域 = conformance/permutation/accept/int_axis.rx(无 forbid,
legal_count == enumerated == emitted == 10,两种预算律读法在该域一致)。
退出码判定(非 grep stdout)。任一判据红 → 逐项打印定位后 exit 1
(evidence 仍如实落盘,红不充绿)。cargo test permutation 单测全绿为前置。

用法:
  py -3 ci/g8_shader_permutation_smoke.py --gate g8.p0.m29.shader_permutation
  py -3 ci/g8_shader_permutation_smoke.py --selftest
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
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
ACCEPT_DIR = ROOT / "conformance" / "permutation" / "accept"
REJECT_DIR = ROOT / "conformance" / "permutation" / "reject"
GOLDEN = ROOT / "conformance" / "permutation" / "golden" / "basic_domain_keys.json"
EXPECT_ERROR_RE = re.compile(r"//@\s*expect-error:\s*(RX\d{4})")

GATE_KEY = "g8.p0.m29.shader_permutation"
NUMERIC_STEP = 98

# M31 基线常量(RXS-0304 空编码;与 ci/g8_reflection_hash_smoke.py 腿⑤同一字面)。
EMPTY_DOMAIN_DIGEST = "160d241dc1681a927e8edbdd07a15e508f9f5aeb68da8bc92274332cb8541f31"

BASIC_RX = ACCEPT_DIR / "basic_domain.rx"
PERMUTED_RX = ACCEPT_DIR / "axis_order_permuted.rx"
INT_AXIS_RX = ACCEPT_DIR / "int_axis.rx"
EMPTY_MIX_RX = ACCEPT_DIR / "empty_domain_entry.rx"
SELECT_KEY = "FOG=true;QUALITY=med"

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def build_rurixc() -> Path:
    print("[g8_m29] cargo build -p rurixc")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[g8_m29] FAIL cargo build:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if not exe.is_file():
        print(f"[g8_m29] FAIL rurixc 产物缺失: {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def run_emit(exe: Path, rx_path: Path, emit: str, extra: list[str] | None = None,
             out_path: Path | None = None) -> tuple[int, str, str]:
    """rurixc <rx> --emit=<emit> [extra] [-o <json>];返回 (returncode, stdout, stderr)。"""
    cmd = [str(exe), str(rx_path), f"--emit={emit}"]
    if extra:
        cmd += extra
    if out_path is not None:
        cmd += ["-o", str(out_path)]
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def run_permutations(exe: Path, rx_path: Path, budget: int | None = None,
                     out_path: Path | None = None) -> tuple[int, str, str]:
    extra = [f"--permutation-budget={budget}"] if budget is not None else None
    return run_emit(exe, rx_path, "permutations", extra, out_path)


def run_reflection(exe: Path, rx_path: Path, select: str | None = None,
                   out_path: Path | None = None) -> tuple[int, str, str]:
    extra = [f"--permutation-select={select}"] if select is not None else None
    return run_emit(exe, rx_path, "reflection", extra, out_path)


def cargo_permutation_tests() -> bool:
    """cargo test -p rurixc --lib permutation(14 单测全绿为前置)。"""
    r = subprocess.run(
        ["cargo", "test", "-p", "rurixc", "--lib", "permutation", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"cargo test permutation 单测失败:\n{r.stdout}\n{r.stderr}")
        return False
    if "14 passed" not in r.stdout and "test result: ok" not in r.stdout:
        check(False, f"cargo test permutation 单测结果异常:\n{r.stdout}")
        return False
    return True


def extract_json(stdout: str) -> dict | None:
    try:
        return json.loads(stdout)
    except Exception:
        return None


def find_entry(doc: dict, name: str) -> dict | None:
    for e in doc.get("entries", []):
        if e["name"] == name:
            return e
    return None


def report_for(exe: Path, rx_path: Path, budget: int | None = None) -> tuple[int, dict | None, str]:
    code, stdout, stderr = run_permutations(exe, rx_path, budget)
    return code, extract_json(stdout), stderr


# ═══════════════════════ 判据腿(经 rurixc CLI 端到端) ═══════════════════════


def leg_double_key_generation(exe: Path) -> None:
    """double_key_generation_byte_identical:报告(含 keys[]/domain_digest)
    两次生成逐字节相等。"""
    j1 = run_permutations(exe, BASIC_RX)[1]
    j2 = run_permutations(exe, BASIC_RX)[1]
    check(j1 == j2 and len(j1) > 0, "double_key: 双次 permutations 报告不逐字节相等(非确定性)")
    check("\r" not in j1 and j1.endswith("}\n"), "double_key: 报告须 LF 行尾无 CRLF")


def leg_legal_set_and_pruned(exe: Path) -> None:
    """legal_set_equals_golden + pruned_combinations_all_absent +
    report_identity_enumerated_eq_pruned_plus_emitted(固定 golden 域)。"""
    golden = json.loads(GOLDEN.read_text(encoding="utf-8"))
    code, doc, stderr = report_for(exe, BASIC_RX)
    check(code == 0, f"legal_set: basic_domain 应绿(0)却退出 {code}\n{stderr}")
    if not doc:
        check(False, "legal_set: 报告 JSON 解析失败")
        return
    e = find_entry(doc, "kmain")
    if not e:
        check(False, "legal_set: kmain entry 缺失")
        return
    check(e["keys"] == golden["keys"], "legal_set: 合法组合集合与 golden 集合不全等")
    check(
        [e["enumerated"], e["pruned"], e["emitted"]]
        == [golden["enumerated"], golden["pruned"], golden["emitted"]],
        "legal_set: 三计数与 golden 不符",
    )
    check(
        e["enumerated"] == e["pruned"] + e["emitted"],
        "identity: enumerated != pruned + emitted(恒等式破)",
    )
    # 静态不可能组合全部被裁剪:golden pruned_keys 不在 keys[],且被裁集合恰为
    # golden 声明(pruned 计数匹配)。
    for pk in golden["pruned_keys"]:
        check(pk not in e["keys"], f"pruned: 被裁组合 {pk} 出现在合法集合")
    # Python 侧独立重算笛卡尔全集:全集 - 合法 = 被裁集合,须与 golden pruned_keys 恰等。
    full = {
        f"FOG={f};QUALITY={q}"
        for f in ("false", "true")
        for q in ("low", "med", "high")
    }
    pruned_set = sorted(full - set(e["keys"]))
    check(
        pruned_set == sorted(golden["pruned_keys"]),
        f"pruned: 被裁集合 {pruned_set} 与 golden 声明 {golden['pruned_keys']} 不恰等(裁剪不完整或误裁)",
    )


def leg_axis_declaration_order_invariant(exe: Path) -> None:
    """axis_declaration_order_invariant:声明序置换 → key 集合与 domain_digest 全等。"""
    _, da, _ = report_for(exe, BASIC_RX)
    _, db, _ = report_for(exe, PERMUTED_RX)
    if not da or not db:
        check(False, "order_invariant: 报告 JSON 解析失败")
        return
    ea, eb = find_entry(da, "kmain"), find_entry(db, "kmain")
    if not ea or not eb:
        check(False, "order_invariant: kmain entry 缺失")
        return
    check(ea["keys"] == eb["keys"], "order_invariant: 声明序置换后 key 集合不等")
    check(
        ea["domain_digest"] == eb["domain_digest"],
        "order_invariant: 声明序置换后 domain_digest 不等",
    )


def leg_budget_boundary(exe: Path) -> None:
    """budget_equal_green / budget_minus_one_red / axis_contribution_report_on_red
    (固定域 int_axis.rx:legal_count == 10;limit==10 GREEN、limit==9 RED)。"""
    legal_count = None
    code, doc, stderr = report_for(exe, INT_AXIS_RX)
    if doc and (e := find_entry(doc, "kmain")):
        legal_count = e["emitted"]
        check(
            legal_count == e["enumerated"] == 10,
            f"budget: int_axis 固定域 legal_count 应为 10(实测 {legal_count})",
        )
    if legal_count is None:
        check(False, f"budget: int_axis 基线报告失败 exit={code}\n{stderr}")
        return

    code_g, _, stderr_g = run_permutations(exe, INT_AXIS_RX, budget=legal_count)
    check(code_g == 0, f"budget_equal_green: --permutation-budget={legal_count} 应 GREEN 却退出 {code_g}\n{stderr_g}")

    code_r, doc_r, stderr_r = report_for(exe, INT_AXIS_RX, budget=legal_count - 1)
    check(code_r != 0, f"budget_minus_one_red: --permutation-budget={legal_count - 1} 应 RED 却退出 0")
    check("RX7023" in stderr_r, f"budget_minus_one_red: 应发 RX7023,stderr 未见:\n{stderr_r}")
    check(doc_r is not None, "budget_minus_one_red: 报告不在位(RED 路径仍须产报告)")
    if doc_r and (er := find_entry(doc_r, "kmain")):
        check(er.get("budget_exceeded") is True, "budget_minus_one_red: budget_exceeded 标记缺失")
        contrib = er.get("axis_contribution", [])
        check(len(contrib) == 2, "axis_contribution_report_on_red: 两 axis contribution 缺失")
        check(
            all("domain_size" in c and "share_num" in c and "share_den" in c for c in contrib),
            "axis_contribution_report_on_red: contribution 缺 |axis|/占比字段",
        )
        check(
            er.get("keys") == [] and er.get("pruned") is None and er.get("emitted") is None,
            "axis_contribution_report_on_red: 超预算路径不得泄漏部分组合表(keys 空/计数 null)",
        )
        check(er.get("enumerated") == 10, "axis_contribution_report_on_red: enumerated 真值缺失")


def leg_select(exe: Path) -> None:
    """select_valid_key_fills_variant_key / select_missing_key_deterministic_error:
    选中后 variant_key=KEY、domain_digest 真值化、pipeline_key 分裂;非法 KEY =
    RX3019 类确定性错误(禁最接近回退)。"""
    _, out_a, _ = run_reflection(exe, BASIC_RX)
    doc_a = extract_json(out_a)
    _, out_b, _ = run_reflection(exe, BASIC_RX, select=SELECT_KEY)
    doc_b = extract_json(out_b)
    if not doc_a or not doc_b:
        check(False, "select: reflection JSON 解析失败")
        return
    ea, eb = find_entry(doc_a, "kmain"), find_entry(doc_b, "kmain")
    if not ea or not eb:
        check(False, "select: kmain entry 缺失")
        return
    check(eb["variant_key"] == SELECT_KEY, "select_valid: 选中后 variant_key 未填 KEY")
    check(
        eb["permutation_domain_digest"] != EMPTY_DOMAIN_DIGEST,
        "select_valid: 非空域 domain_digest 未真值化(仍空编码常量)",
    )
    check(
        ea["permutation_domain_digest"] == eb["permutation_domain_digest"],
        "select_valid: domain_digest 与 select 无关(真值化后应恒定)",
    )
    check(
        ea["pipeline_key"] != eb["pipeline_key"],
        "select_valid: 同 entry select 前后 pipeline_key 未分裂(preimage 含 variant_key 必变)",
    )
    check(ea["variant_key"] == "", "select_valid: 未选择时 variant_key 应空串(0 漂移)")

    code_bad, _, stderr_bad = run_reflection(exe, BASIC_RX, select="FOG=true;QUALITY=ultra")
    check(code_bad != 0, "select_missing: 非法 KEY 应确定性错误却退出 0")
    check("RX3019" in stderr_bad, f"select_missing: 应发 RX3019 类,stderr 未见:\n{stderr_bad}")
    code_pruned, _, stderr_pruned = run_reflection(exe, BASIC_RX, select="FOG=true;QUALITY=low")
    check(code_pruned != 0 and "RX3019" in stderr_pruned,
          "select_missing: 被裁剪组合的 KEY 同样须确定性拒(禁最接近回退)")


def leg_empty_domain_zero_drift(exe: Path) -> None:
    """empty_domain_reflection_zero_drift:空域 entry 的 reflection 与 M31 基线
    常量一致,且与同 entry 单独成单元的基线产物逐字段一致(0 字节漂移)。"""
    _, out_mix, _ = run_reflection(exe, EMPTY_MIX_RX)
    doc_mix = extract_json(out_mix)
    if not doc_mix:
        check(False, "zero_drift: 混合单元 reflection JSON 解析失败")
        return
    plain = find_entry(doc_mix, "plain")
    tagged = find_entry(doc_mix, "tagged")
    if not plain or not tagged:
        check(False, "zero_drift: plain/tagged entry 缺失")
        return
    check(
        plain["permutation_domain_digest"] == EMPTY_DOMAIN_DIGEST,
        "zero_drift: 空域 permutation_domain_digest 不等于 M31 基线常量",
    )
    check(plain["variant_key"] == "", "zero_drift: 空域 variant_key 应空串")
    check(
        tagged["permutation_domain_digest"] != EMPTY_DOMAIN_DIGEST,
        "zero_drift: 非空域 domain_digest 未真值化",
    )
    # M31 基线:同一 entry 单独成单元(无 #[permutation] 时代)的产物。
    with tempfile.TemporaryDirectory() as d:
        baseline_rx = Path(d) / "baseline.rx"
        baseline_rx.write_text("kernel fn plain() {}\nfn main() {}\n", encoding="utf-8")
        _, out_base, _ = run_reflection(exe, baseline_rx)
    doc_base = extract_json(out_base)
    if not doc_base or not (base := find_entry(doc_base, "plain")):
        check(False, "zero_drift: M31 基线单元 reflection 失败")
        return
    for field in ("permutation_domain_digest", "variant_key", "interface_hash",
                  "source_digest", "pipeline_key", "canonical_hex"):
        check(
            plain[field] == base[field],
            f"zero_drift: 空域 entry `{field}` 与 M31 基线不一致(0 漂移破)",
        )


def leg_report_identity_all_accept(exe: Path) -> None:
    """report_identity(全 accept 语料面):每 entry enumerated == pruned + emitted;
    报告不含路径/文件名。"""
    for rx in sorted(ACCEPT_DIR.glob("*.rx")):
        code, doc, stderr = report_for(exe, rx)
        check(code == 0, f"identity: {rx.name} 报告失败 exit={code}\n{stderr}")
        if not doc:
            check(False, f"identity: {rx.name} 报告 JSON 解析失败")
            continue
        check(rx.name not in json.dumps(doc), f"identity: {rx.name} 文件名泄入报告")
        for e in doc.get("entries", []):
            if e.get("budget_exceeded"):
                continue
            check(
                e["enumerated"] == e["pruned"] + e["emitted"],
                f"identity: {rx.name} entry {e['name']} 恒等式破",
            )


# ═══════════════════════ 语料批跑 ═══════════════════════


def accept_corpus(exe: Path) -> int:
    """conformance/permutation/accept/*.rx 逐件 --emit=permutations 与
    --emit=reflection 双通道退出 0、JSON 可解析。"""
    cases = sorted(ACCEPT_DIR.glob("*.rx"))
    check(len(cases) == 4, f"accept 语料应为 4 件,实测 {len(cases)}")
    for rx in cases:
        with tempfile.TemporaryDirectory() as d:
            out = Path(d) / (rx.stem + ".json")
            code, _, stderr = run_permutations(exe, rx, out_path=out)
            check(code == 0, f"accept: {rx.name} permutations 应绿(0)却退出 {code}\n{stderr}")
            if code == 0:
                check(out.is_file(), f"accept: {rx.name} permutations 报告未产出")
                if out.is_file():
                    try:
                        json.loads(out.read_text(encoding="utf-8"))
                    except Exception as e:
                        check(False, f"accept: {rx.name} 报告 JSON 解析失败: {e}")
            code2, stdout2, stderr2 = run_reflection(exe, rx)
            check(code2 == 0, f"accept: {rx.name} reflection 应绿(0)却退出 {code2}\n{stderr2}")
            if code2 == 0:
                check(extract_json(stdout2) is not None, f"accept: {rx.name} reflection JSON 解析失败")
    return len(cases)


def reject_corpus(exe: Path) -> int:
    """conformance/permutation/reject/*.rx 逐件必红且落头部声明的错误码。"""
    cases = sorted(REJECT_DIR.glob("*.rx"))
    check(len(cases) == 4, f"reject 语料应为 4 件,实测 {len(cases)}")
    for rx in cases:
        m = EXPECT_ERROR_RE.search(rx.read_text(encoding="utf-8"))
        if m is None:
            check(False, f"reject: {rx.name} 缺 `//@ expect-error: RX####` 头声明")
            continue
        want = m.group(1)
        code, stdout, stderr = run_permutations(exe, rx)
        check(code != 0, f"reject: {rx.name} 应红({want})却退出 0")
        combined = stdout + stderr
        check(want in combined, f"reject: {rx.name} 应发 {want},未见于输出:\n{combined}")
    return len(cases)


# ═══════════════════════ evidence 落盘 ═══════════════════════


def write_evidence(results: dict, host_ok: bool) -> None:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ev = {
        "schema_version": 1,
        "subject": "g8_m29_shader_permutation",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M29",
        "wave": "G8.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": "RFC-0019 §4.3;spec/rendering_platform.md RXS-0308~0310",
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
            "host/compile 纯 host 门;device 段 not_applicable(CI_GATES §6 host-only 行)。"
            "13 项判据经 rurixc --emit=permutations/--emit=reflection + "
            "--permutation-budget/--permutation-select CLI 端到端 + cargo test permutation "
            "14 单测前置。预算边界腿固定域 int_axis.rx(legal_count == 10:无 forbid,"
            "enumerated == emitted,两种预算律读法一致;上限含等号 GREEN)。"
        ),
    }
    path = EVIDENCE_DIR / f"g8_m29_shader_permutation_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m29] evidence 落盘: {path.relative_to(ROOT)}")


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip()
    except Exception:
        return "unknown"


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def selftest() -> None:
    """反 YAML-only:合成数据喂纯判定层,证明每组断言都能红(不跑 cargo、不写 evidence)。"""
    # check() 能正确记录失败
    check(False, "selftest: 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print("[g8_m29] selftest FAIL: check() 未正确记录合成失败", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    # extract_json 对非法 JSON 返回 None
    assert extract_json("not json") is None
    # find_entry 对缺失 entry 返回 None
    assert find_entry({"entries": []}, "x") is None
    # 恒等式判据能对合成违例判红(enumerated=6, pruned=1, emitted=4 → 破)
    synth = {"enumerated": 6, "pruned": 1, "emitted": 4}
    check(
        synth["enumerated"] == synth["pruned"] + synth["emitted"],
        "selftest: 合成恒等式违例(证明恒等式断言能红)",
    )
    if len(FAILURES) != 1 or "恒等式" not in FAILURES[0]:
        print("[g8_m29] selftest FAIL: 恒等式合成违例未被判红", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    # golden 集合比对能对合成错位判红
    golden_keys = ["A=false", "A=true"]
    check(golden_keys == ["A=true", "A=false"], "selftest: 合成 golden 错位(证明集合比对能红)")
    if len(FAILURES) != 1:
        print("[g8_m29] selftest FAIL: golden 合成错位未被判红", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    print("[g8_m29] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")


# ═══════════════════════ main ═══════════════════════


def main() -> int:
    parser = argparse.ArgumentParser(description="G8.2 M29 shader_permutation 硬门冒烟")
    parser.add_argument("--gate", default=GATE_KEY, help="symbolic gate key")
    parser.add_argument("--selftest", action="store_true", help="反 YAML-only 红绿自检")
    args = parser.parse_args()

    if args.selftest:
        selftest()
        return 0

    if args.gate != GATE_KEY:
        check(False, f"--gate `{args.gate}` ≠ canonical key `{GATE_KEY}`")

    exe = build_rurixc()

    # 前置:cargo test permutation 单测全绿
    tests_ok = cargo_permutation_tests()

    # 判据腿
    leg_double_key_generation(exe)
    leg_legal_set_and_pruned(exe)
    leg_axis_declaration_order_invariant(exe)
    leg_budget_boundary(exe)
    leg_select(exe)
    leg_empty_domain_zero_drift(exe)
    leg_report_identity_all_accept(exe)

    # 语料批跑
    n_accept = accept_corpus(exe)
    n_reject = reject_corpus(exe)

    # 汇总 checks(13 项,缺一 FAIL)
    results = {
        "double_key_generation_byte_identical": not any("double_key" in f for f in FAILURES),
        "legal_set_equals_golden": not any("legal_set" in f for f in FAILURES),
        "pruned_combinations_all_absent": not any("pruned" in f for f in FAILURES),
        "axis_declaration_order_invariant": not any("order_invariant" in f for f in FAILURES),
        "budget_equal_green": not any("budget_equal_green" in f for f in FAILURES),
        "budget_minus_one_red": not any("budget_minus_one_red" in f or ("budget:" in f) for f in FAILURES),
        "report_identity_enumerated_eq_pruned_plus_emitted": not any("identity" in f for f in FAILURES),
        "axis_contribution_report_on_red": not any("axis_contribution_report_on_red" in f for f in FAILURES),
        "select_valid_key_fills_variant_key": not any("select_valid" in f for f in FAILURES),
        "select_missing_key_deterministic_error": not any("select_missing" in f for f in FAILURES),
        "empty_domain_reflection_zero_drift": not any("zero_drift" in f for f in FAILURES),
        "accept_corpus_green": tests_ok and not any("accept" in f or "cargo test" in f for f in FAILURES),
        "reject_corpus_red_with_codes": not any("reject" in f for f in FAILURES),
    }

    host_ok = tests_ok and len(FAILURES) == 0

    write_evidence(results, host_ok)

    for m in NOTES:
        print(f"[g8_m29] NOTE {m}")
    if FAILURES:
        print(f"[g8_m29] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print(
        f"[g8_m29] PASS (host/compile 纯 host 门;"
        f"{n_accept} accept 语料绿 + {n_reject} reject 语料确定性拒;"
        f"cargo test permutation 14 单测全绿;13 checks 全真)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
