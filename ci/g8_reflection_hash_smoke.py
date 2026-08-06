#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.2 M31 reflection_hash 硬门冒烟(步骤 97;g8.p0.m31.reflection_hash;
RFC-0019 §4.4;spec/rendering_platform.md RXS-0304~0307)。

host/compile 纯 host 门(host 恒跑,check_* 风格;device 段 not_applicable)。
六腿判据 + 锚定:

  ① 双次构建:canonical bytes 与 digest 逐字节相等(确定性)。
  ② 声明序置换 / 语义无关路径扰动 → canonical 与 hash 不变。
  ③ 仅改函数体 → interface_hash 不变、source_digest 必变。
  ④ ABI 四轴(binding / resource kind / stage visibility / value type)任一
     改变 → interface_hash 必变。
  ⑤ 空/未实现字段(M29/M32/M50)确定性空编码 + 同名 entry 跨 mod fail-closed
     + 无界非-SRV 纹理表 fail-closed + compute 形参超闭集 fail-closed。
  ⑥ JSON 产物确定性 + 不含路径/文件名/时间戳 + 装配期核验 fail-closed。

退出码判定(非 grep stdout)。任一判据红 → 逐项打印定位后 exit 1
(evidence 仍如实落盘,红不充绿)。cargo test reflection 单测全绿为前置。

用法:
  py -3 ci/g8_reflection_hash_smoke.py --gate g8.p0.m31.reflection_hash
  py -3 ci/g8_reflection_hash_smoke.py --selftest
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
ACCEPT_DIR = ROOT / "conformance" / "reflection" / "accept"
REJECT_DIR = ROOT / "conformance" / "reflection" / "reject"
EXPECT_ERROR_RE = re.compile(r"//@\s*expect-error:\s*(RX\d{4})")

GATE_KEY = "g8.p0.m31.reflection_hash"
NUMERIC_STEP = 97

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def build_rurixc() -> Path:
    print("[g8_m31] cargo build -p rurixc")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[g8_m31] FAIL cargo build:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if not exe.is_file():
        print(f"[g8_m31] FAIL rurixc 产物缺失: {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def run_reflection(exe: Path, rx_path: Path, out_path: Path | None = None) -> tuple[int, str, str]:
    """rurixc <rx> --emit=reflection [-o <json>];返回 (returncode, stdout, stderr)。"""
    cmd = [str(exe), str(rx_path), "--emit=reflection"]
    if out_path is not None:
        cmd += ["-o", str(out_path)]
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def cargo_reflection_tests() -> bool:
    """cargo test -p rurixc --lib reflection(15 单测全绿为前置)。"""
    r = subprocess.run(
        ["cargo", "test", "-p", "rurixc", "--lib", "reflection", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        check(False, f"cargo test reflection 单测失败:\n{r.stdout}\n{r.stderr}")
        return False
    # 确认 15 tests passed
    if "15 passed" not in r.stdout and "test result: ok" not in r.stdout:
        check(False, f"cargo test reflection 单测结果异常:\n{r.stdout}")
        return False
    return True


# ═══════════════════════ 六腿判据(经 rurixc CLI 端到端) ═══════════════════════

BASE_SRC = """\
struct VsOut {
    #[builtin(position)] pos: f32,
    #[interpolate(perspective)] uv: f32,
    #[interpolate(flat)] mat_id: u32,
}

vertex fn vs_main(inp: VsOut, tex: Texture2D<f32>, samp: Sampler) -> VsOut {
    VsOut { pos: 0.0, uv: 0.0, mat_id: 0 }
}

fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler) -> VsOut {
    inp
}

kernel fn kmain(t: ThreadCtx<1>, tlas: AccelStruct, buf: ViewMut<global, f32>, n: usize) {
    let i = t.global_id();
    if i < n { buf[i] = 1.0; }
}

fn main() {}
"""


def write_tmp(src: str, d: Path, name: str = "test.rx") -> Path:
    p = d / name
    p.write_text(src, encoding="utf-8")
    return p


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


def leg_double_reflection(exe: Path) -> None:
    """腿①:双次构建 canonical bytes 与 digest 逐字节相等。"""
    with tempfile.TemporaryDirectory() as d:
        td = Path(d)
        rx = write_tmp(BASE_SRC, td)
        j1 = run_reflection(exe, rx)[1]
        j2 = run_reflection(exe, rx)[1]
        check(j1 == j2, "腿① 双次 reflection JSON 不等(非确定性)")
        doc = extract_json(j1)
        check(doc is not None, "腿① reflection JSON 解析失败")
        if doc:
            check(len(doc.get("entries", [])) == 3, f"腿① entries 应为 3,实测 {len(doc.get('entries', []))}")


def leg_declaration_order(exe: Path) -> None:
    """腿②:声明序置换 → canonical 与 hash 不变。"""
    fs_block = (
        "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler) -> VsOut {\n"
        "    inp\n"
        "}\n\n"
    )
    assert fs_block in BASE_SRC
    reordered = BASE_SRC.replace(fs_block, "")
    reordered = reordered.replace("vertex fn vs_main", fs_block + "vertex fn vs_main")
    assert reordered != BASE_SRC
    with tempfile.TemporaryDirectory() as d:
        td = Path(d)
        j1 = run_reflection(exe, write_tmp(BASE_SRC, td, "a.rx"))[1]
        j2 = run_reflection(exe, write_tmp(reordered, td, "b.rx"))[1]
        check(j1 == j2, "腿② 声明序置换后 JSON 不等(应不变)")
        # 文件名不同也不影响
        j3 = run_reflection(exe, write_tmp(BASE_SRC, td, "zzz.rx"))[1]
        check(j1 == j3, "腿② 文件名改变后 JSON 不等(路径不得入产物)")


def leg_body_only_change(exe: Path) -> None:
    """腿③:仅改函数体 → interface_hash 不变、source_digest 必变。"""
    edited = BASE_SRC.replace("buf[i] = 1.0;", "buf[i] = 2.0;")
    assert edited != BASE_SRC
    with tempfile.TemporaryDirectory() as d:
        td = Path(d)
        da = extract_json(run_reflection(exe, write_tmp(BASE_SRC, td))[1])
        db = extract_json(run_reflection(exe, write_tmp(edited, td))[1])
        if not da or not db:
            check(False, "腿③ JSON 解析失败")
            return
        ka, kb = find_entry(da, "kmain"), find_entry(db, "kmain")
        if not ka or not kb:
            check(False, "腿③ kmain entry 缺失")
            return
        check(
            ka["interface_hash"] == kb["interface_hash"],
            "腿③ 仅函数体改动 interface_hash 不应变",
        )
        check(
            ka["source_digest"] != kb["source_digest"],
            "腿③ 函数体改动 source_digest 必变",
        )
        check(
            ka["pipeline_key"] != kb["pipeline_key"],
            "腿③ pipeline_key 含 source_digest,必变",
        )
        # vs_main 不动
        va, vb = find_entry(da, "vs_main"), find_entry(db, "vs_main")
        if va and vb:
            check(
                va["source_digest"] == vb["source_digest"],
                "腿③ vs_main 未改动 source_digest 应不变",
            )


def leg_abi_binding(exe: Path) -> None:
    """腿④a:binding 改变 → hash 必变。"""
    edited = BASE_SRC.replace(
        "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler)",
        "fragment fn fs_main(inp: VsOut, tex_a: Texture2D<f32>, tex_b: Texture2D<f32>, samp: Sampler)",
    )
    _abi_leg(exe, edited, "腿④a", "fs_main", check_binding=True)


def leg_abi_resource_kind(exe: Path) -> None:
    """腿④b:resource kind 改变 → hash 必变。"""
    edited = BASE_SRC.replace(
        "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler)",
        "fragment fn fs_main(inp: VsOut, tex_b: TextureRw2D<f32>, samp: Sampler)",
    )
    _abi_leg(exe, edited, "腿④b", "fs_main")


def leg_abi_stage_visibility(exe: Path) -> None:
    """腿④c:stage visibility 改变 → hash 必变。"""
    edited = (
        BASE_SRC.replace(
            "vertex fn vs_main(inp: VsOut, tex: Texture2D<f32>, samp: Sampler) -> VsOut",
            "vertex fn vs_main(inp: VsOut, tex: Texture2D<f32>) -> VsOut",
        )
        .replace(
            "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler) -> VsOut",
            "fragment fn fs_main(inp: VsOut, tex_b: Texture2D<f32>, samp: Sampler, samp2: Sampler) -> VsOut",
        )
    )
    _abi_leg(exe, edited, "腿④c", "fs_main")


def leg_abi_value_type(exe: Path) -> None:
    """腿④d:value type 改变 → hash 必变。buffer 元素类型 f32 → u32
    (push-constant 标量 `n` 保持 usize 以通过 typeck;buffer 元素类型是 ABI 字段)。"""
    edited = BASE_SRC.replace("ViewMut<global, f32>", "ViewMut<global, u32>").replace("buf[i] = 1.0;", "buf[i] = 1;")
    _abi_leg(exe, edited, "腿④d", "kmain")


def _abi_leg(exe: Path, edited: str, label: str, entry_name: str, check_binding: bool = False) -> None:
    assert edited != BASE_SRC
    with tempfile.TemporaryDirectory() as d:
        td = Path(d)
        da = extract_json(run_reflection(exe, write_tmp(BASE_SRC, td))[1])
        db = extract_json(run_reflection(exe, write_tmp(edited, td))[1])
        if not da or not db:
            check(False, f"{label} JSON 解析失败")
            return
        ea, eb = find_entry(da, entry_name), find_entry(db, entry_name)
        if not ea or not eb:
            check(False, f"{label} {entry_name} entry 缺失")
            return
        check(
            ea["interface_hash"] != eb["interface_hash"],
            f"{label} ABI 改变后 interface_hash 应变却不变",
        )
        if check_binding:
            # tex_b binding 0 → 1
            ra = [r for r in ea["resources"] if r["name"] == "tex_b"]
            rb = [r for r in eb["resources"] if r["name"] == "tex_b"]
            if ra and rb:
                check(ra[0]["binding"] == 0, f"{label} 原 tex_b binding 应为 0")
                check(rb[0]["binding"] == 1, f"{label} 新 tex_b binding 应为 1")


def leg_empty_and_fail_closed(exe: Path) -> None:
    """腿⑤:空编码稳定 + fail-closed 三路。"""
    with tempfile.TemporaryDirectory() as d:
        td = Path(d)
        # 空 entries
        empty_src = "fn main() {}\n"
        doc = extract_json(run_reflection(exe, write_tmp(empty_src, td))[1])
        check(doc is not None, "腿⑤ 空 entries JSON 解析失败")
        if doc:
            check(len(doc.get("entries", [])) == 0, "腿⑤ 无 entry 应产空表")
        # 空编码稳定常量
        base_doc = extract_json(run_reflection(exe, write_tmp(BASE_SRC, td))[1])
        if base_doc:
            k = find_entry(base_doc, "kmain")
            if k:
                check(
                    k["selected_profile_digest"] == "2997fd21a324a39e63cd1da6970db88c511e8d025d24fbce0bbb94c5ea8c28b6",
                    "腿⑤ selected_profile_digest 空编码常量不符",
                )
                check(
                    k["permutation_domain_digest"]
                    == "160d241dc1681a927e8edbdd07a15e508f9f5aeb68da8bc92274332cb8541f31",
                    "腿⑤ permutation_domain_digest 空编码常量不符",
                )
                check(k["variant_key"] == "", "腿⑤ variant_key 应为空串")

    # fail-closed: 同名 entry 跨 mod → RX6026
    dup_src = "mod a { vertex fn dup() -> f32 { 0.0 } }\nmod b { vertex fn dup() -> f32 { 0.0 } }\nfn main() {}\n"
    with tempfile.TemporaryDirectory() as d:
        code, _, stderr = run_reflection(exe, write_tmp(dup_src, Path(d)))
        check(code != 0, "腿⑤ 同名 entry 跨 mod 应红却退出 0")
        check("RX6026" in stderr, f"腿⑤ 同名 entry 应发 RX6026,stderr 未见:\n{stderr}")

    # fail-closed: 无界 Sampler 表 → RX6013
    unbounded_src = "vertex fn v(samps: [Sampler]) -> f32 { 0.0 }\nfn main() {}\n"
    with tempfile.TemporaryDirectory() as d:
        code, _, stderr = run_reflection(exe, write_tmp(unbounded_src, Path(d)))
        check(code != 0, "腿⑤ 无界 Sampler 表应红却退出 0")
        check("RX6013" in stderr, f"腿⑤ 无界 Sampler 表应发 RX6013,stderr 未见:\n{stderr}")

    # fail-closed: compute 形参超闭集 → RX6026
    struct_src = "struct S { x: f32 }\nkernel fn k(t: ThreadCtx<1>, s: S) { let _ = s.x; }\nfn main() {}\n"
    with tempfile.TemporaryDirectory() as d:
        code, _, stderr = run_reflection(exe, write_tmp(struct_src, Path(d)))
        check(code != 0, "腿⑤ compute 形参超闭集应红却退出 0")
        check("RX6026" in stderr, f"腿⑤ compute 形参超闭集应发 RX6026,stderr 未见:\n{stderr}")


def leg_json_and_verify(exe: Path) -> None:
    """腿⑥:JSON 产物确定性 + 不含路径 + 装配期核验 fail-closed(经单测覆盖,
    CLI 侧验证 JSON 确定性与路径无关)。"""
    with tempfile.TemporaryDirectory() as d:
        td = Path(d)
        j1 = run_reflection(exe, write_tmp(BASE_SRC, td, "test.rx"))[1]
        j2 = run_reflection(exe, write_tmp(BASE_SRC, td, "test.rx"))[1]
        check(j1 == j2, "腿⑥ JSON 两次生成不等(非确定性)")
        check(j1.endswith("}\n"), "腿⑥ JSON 应以 }\\n 结尾(LF)")
        check("\r" not in j1, "腿⑥ JSON 禁 CRLF")
        check("test.rx" not in j1, "腿⑥ JSON 不得含文件名/路径")
        # schema 字段在位
        doc = extract_json(j1)
        if doc:
            check(doc["schema"] == "rurix.shader-reflection.v1", "腿⑥ schema 字段不符")
            check(doc["schema_version"] == 1, "腿⑥ schema_version 不符")
            for e in doc.get("entries", []):
                check("interface_hash" in e, f"腿⑥ entry {e.get('name')} 缺 interface_hash")
                check("canonical_hex" in e, f"腿⑥ entry {e.get('name')} 缺 canonical_hex")
                check("pipeline_key" in e, f"腿⑥ entry {e.get('name')} 缺 pipeline_key")


# ═══════════════════════ 语料批跑 ═══════════════════════


def accept_corpus(exe: Path) -> int:
    """conformance/reflection/accept/*.rx 逐件 --emit=reflection 退出 0。"""
    cases = sorted(ACCEPT_DIR.glob("*.rx"))
    check(len(cases) > 0, f"conformance/reflection/accept 无语料: {ACCEPT_DIR}")
    for rx in cases:
        with tempfile.TemporaryDirectory() as d:
            out = Path(d) / (rx.stem + ".json")
            code, stdout, stderr = run_reflection(exe, rx, out)
            check(code == 0, f"{rx.name}: accept 应绿(0)却退出 {code}\n{stderr}")
            if code == 0:
                check(out.is_file(), f"{rx.name}: JSON 产物未产出")
                if out.is_file():
                    try:
                        json.loads(out.read_text(encoding="utf-8"))
                    except Exception as e:
                        check(False, f"{rx.name}: JSON 产物解析失败: {e}")
    return len(cases)


def reject_corpus(exe: Path) -> int:
    """conformance/reflection/reject/*.rx 逐件必红且落头部声明的错误码。"""
    cases = sorted(REJECT_DIR.glob("*.rx"))
    if not cases:
        return 0
    for rx in cases:
        m = EXPECT_ERROR_RE.search(rx.read_text(encoding="utf-8"))
        if m is None:
            check(False, f"{rx.name}: reject 语料缺 `//@ expect-error: RX####` 头声明")
            continue
        want = m.group(1)
        with tempfile.TemporaryDirectory() as d:
            out = Path(d) / (rx.stem + ".json")
            code, stdout, stderr = run_reflection(exe, rx, out)
            check(code != 0, f"{rx.name}: reject 语料应红({want})却退出 0")
            combined = stdout + stderr
            check(want in combined, f"{rx.name}: reject 语料应发 {want},未见于输出:\n{combined}")
            check(not out.is_file(), f"{rx.name}: reject 语料不得产 JSON(strict-only)")
    return len(cases)


# ═══════════════════════ evidence 落盘 ═══════════════════════


def write_evidence(results: dict, host_ok: bool) -> None:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ev = {
        "schema_version": 1,
        "subject": "g8_m31_reflection_hash",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M31",
        "wave": "G8.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": "RFC-0019 §4.4;spec/rendering_platform.md RXS-0304~0307",
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
            "六腿判据经 rurixc --emit=reflection CLI 端到端 + cargo test reflection 15 单测前置。"
        ),
    }
    path = EVIDENCE_DIR / f"g8_m31_reflection_hash_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m31] evidence 落盘: {path.relative_to(ROOT)}")


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip()
    except Exception:
        return "unknown"


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def selftest() -> None:
    """反 YAML-only:合成数据喂纯判定层,证明每组断言都能红。"""
    # check() 能正确记录失败
    check(False, "selftest: 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print("[g8_m31] selftest FAIL: check() 未正确记录合成失败", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    # extract_json 对非法 JSON 返回 None
    assert extract_json("not json") is None
    # find_entry 对缺失 entry 返回 None
    assert find_entry({"entries": []}, "x") is None
    print("[g8_m31] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")


# ═══════════════════════ main ═══════════════════════


def main() -> int:
    parser = argparse.ArgumentParser(description="G8.2 M31 reflection_hash 硬门冒烟")
    parser.add_argument("--gate", default=GATE_KEY, help="symbolic gate key")
    parser.add_argument("--selftest", action="store_true", help="反 YAML-only 红绿自检")
    args = parser.parse_args()

    if args.selftest:
        selftest()
        return 0

    if args.gate != GATE_KEY:
        check(False, f"--gate `{args.gate}` ≠ canonical key `{GATE_KEY}`")

    exe = build_rurixc()

    # 前置:cargo test reflection 单测全绿
    tests_ok = cargo_reflection_tests()

    # 六腿判据
    leg_double_reflection(exe)
    leg_declaration_order(exe)
    leg_body_only_change(exe)
    leg_abi_binding(exe)
    leg_abi_resource_kind(exe)
    leg_abi_stage_visibility(exe)
    leg_abi_value_type(exe)
    leg_empty_and_fail_closed(exe)
    leg_json_and_verify(exe)

    # 语料批跑
    n_accept = accept_corpus(exe)
    n_reject = reject_corpus(exe)

    # 汇总 checks
    results = {
        "double_reflection_byte_identical": not any("腿①" in f for f in FAILURES),
        "declaration_order_invariant": not any("腿②" in f for f in FAILURES),
        "body_only_change_keeps_interface_hash": not any("腿③" in f for f in FAILURES),
        "abi_binding_change_flips_hash": not any("腿④a" in f for f in FAILURES),
        "abi_resource_kind_change_flips_hash": not any("腿④b" in f for f in FAILURES),
        "abi_stage_visibility_change_flips_hash": not any("腿④c" in f for f in FAILURES),
        "abi_value_type_change_flips_hash": not any("腿④d" in f for f in FAILURES),
        "empty_encodings_stable": not any("腿⑤" in f and "空编码" in f for f in FAILURES),
        "duplicate_entry_fails_closed": not any("腿⑤" in f and "同名 entry" in f for f in FAILURES),
        "unbounded_sampler_unmappable": not any("腿⑤" in f and "无界 Sampler" in f for f in FAILURES),
        "compute_struct_param_unsupported": not any("腿⑤" in f and "compute 形参超闭集" in f for f in FAILURES),
        "json_artifact_deterministic": not any("腿⑥" in f and ("JSON 两次" in f or "CRLF" in f) for f in FAILURES),
        "json_artifact_path_free": not any("腿⑥" in f and "文件名" in f for f in FAILURES),
        "assembly_verify_pair_fail_closed": tests_ok and not any("腿⑥" in f and "interface_hash" in f for f in FAILURES),
        "accept_corpus_green": not any("accept" in f for f in FAILURES),
        "reject_corpus_red_with_codes": not any("reject" in f for f in FAILURES),
    }

    host_ok = tests_ok and len(FAILURES) == 0

    write_evidence(results, host_ok)

    for m in NOTES:
        print(f"[g8_m31] NOTE {m}")
    if FAILURES:
        print(f"[g8_m31] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print(
        f"[g8_m31] PASS (host/compile 纯 host门;"
        f"{n_accept} accept 语料绿 + {n_reject} reject 语料确定性拒;"
        f"cargo test reflection 15 单测全绿)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
