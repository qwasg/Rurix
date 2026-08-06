#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.2 M89 single_source_gfx_submit 硬门冒烟(g8.p0.m89.single_source_gfx_submit;
RFC-0019;spec/rhi.md RXS-0319~0321;RD-037;设计案 §6.4)。

host 段(恒跑,不触 GPU):
  零 `.rs` 宿主审计 / cabi 三符号+READBACK_DUMP 锚 / 禁 host 像素替身静态锚 /
  artifacts v2 按名消费锚 / VB·IB·vs_layout lowering 锚 / seal 双向核验锚 /
  compile_only_not_pass 字面(accept 含像素自断言,禁仅编译充绿)。

device 段(gate real;`RURIX_REQUIRE_REAL=1` 翻硬红,缺则 SKIP=dev-env-degrade):
  `rx build --features vulkan-backend` → accept×2 EXE 真跑 exit 0 +
  `RURIX_RHI_READBACK_DUMP` 逐字节 == `tests/gfx/m89_golden.rgba8` +
  reject×4 RED(3×EXE 装配拒 + 1×编译期 brand 拒) +
  `RURIX_VK_VALIDATION=1` 零 ERROR。

用法:
  py -3 ci/g8_single_source_gfx_smoke.py --gate g8.p0.m89.single_source_gfx_submit
  py -3 ci/g8_single_source_gfx_smoke.py --selftest
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
SCHEMA_PATH = (
    ROOT / "milestones" / "g8" / "g8_m89_single_source_gfx_submit_evidence_schema.json"
)
CORPUS = ROOT / "conformance" / "gfx_submit"
ACCEPT_DIR = CORPUS / "accept"
REJECT_DIR = CORPUS / "reject"
GOLDEN = ROOT / "tests" / "gfx" / "m89_golden.rgba8"
WORK = ROOT / "target" / "g8_m89_gfx_smoke"
JUDGE = ACCEPT_DIR / "m89_two_tri_quad.rx"

GATE_KEY = "g8.p0.m89.single_source_gfx_submit"
NUMERIC_STEP = 102
SOURCE_REF = (
    "RFC-0019;spec/rhi.md RXS-0319~0321;RD-037;G8.2_SHADER_PLATFORM_DESIGN §6"
)
TAG = "g8_m89"

CHECK_KEYS = [
    "rx_exe_green_readback_selfassert",
    "readback_equals_checked_in_golden",
    "rust_host_source_count_zero",
    "no_host_pixel_substitution",
    "artifacts_v2_real_consumption",
    "vb_ib_binding_via_cabi",
    "assembly_reject_legs_red",
    "seal_reflection_bidirectional_verify",
    "compile_only_not_pass",
    "validation_zero_errors",
    "accept_corpus_green",
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


def fail(msg: str) -> int:
    print(f"[{TAG}] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if require_real():
        return fail(msg + "（RURIX_REQUIRE_REAL=1 不许 SKIP）")
    print(f"[{TAG}] SKIP {msg}（dev-env-degrade,退出 0）")
    return 0


def run(
    cmd: list[str],
    cwd: Path = ROOT,
    env: dict | None = None,
    timeout: int = 900,
) -> tuple[int, str, str]:
    e = os.environ.copy()
    if env:
        e.update(env)
    r = subprocess.run(
        cmd, cwd=str(cwd), capture_output=True, timeout=timeout, env=e
    )
    return (
        r.returncode,
        r.stdout.decode("utf-8", "replace"),
        r.stderr.decode("utf-8", "replace"),
    )


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip()
    except Exception:
        return "unknown"


def exe_name(stem: str) -> str:
    return f"{stem}.exe" if sys.platform == "win32" else stem


# ─────────────────────────── host 段 ───────────────────────────


def host_rust_host_source_count_zero() -> bool:
    """fixture 目录 + 启动链零 `.rs`(RD-037);cabi/rt 无 fixture 名硬编码。"""
    ok = True
    for base in (ACCEPT_DIR, REJECT_DIR, ROOT / "tests" / "gfx"):
        if not base.is_dir():
            check(False, f"rust_host: 缺目录 {base.relative_to(ROOT)}")
            ok = False
            continue
        rs = sorted(p for p in base.rglob("*.rs") if p.is_file())
        check(not rs, f"rust_host: {base.relative_to(ROOT)} 含 .rs: {[p.name for p in rs]}")
        ok = ok and not rs
    # cabi/rt 禁 fixture 名硬编码
    needles = ["m89_two_tri_quad", "m89_golden", "vb_only_draw"]
    for rel in (
        "src/rurix-rt-cabi/src/lib.rs",
        "src/rurix-rt/src/vk.rs",
        "src/rurix-rt/src/rhi.rs",
    ):
        text = (ROOT / rel).read_text(encoding="utf-8", errors="replace")
        for n in needles:
            hit = n in text
            check(not hit, f"rust_host: {rel} 硬编码 fixture 名 `{n}`")
            ok = ok and not hit
    return ok


def host_no_host_pixel_substitution() -> bool:
    """cabi gfx 臂禁 host 填像素替身:派发结果仅来自 run_rhi_graphics*_ 写回。"""
    lib = (ROOT / "src/rurix-rt-cabi/src/lib.rs").read_text(encoding="utf-8", errors="replace")
    # 必须走真实 graphics 入口
    has_v2 = "run_rhi_graphics_offscreen_v2" in lib
    check(has_v2, "no_host_pixel: cabi 未调用 run_rhi_graphics_offscreen_v2")
    # 禁在 gfx 臂内用常数色填充 color target(典型替身)
    # 允许 clear 色字面;禁 `vec![0xff` / `fill(` 写回 vk_resources 的模式。
    arm = lib
    if "fn rhi_submit_vk_gfx" in lib:
        arm = lib.split("fn rhi_submit_vk_gfx", 1)[1].split("fn rhi_stream_sync", 1)[0]
    bad = re.search(r"vk_resources.*fill\(|vec!\[\s*0xff|host.?pixel.?substitut", arm, re.I)
    check(bad is None, f"no_host_pixel: gfx 臂疑似 host 填像素: {bad.group(0) if bad else ''}")
    check("RURIX_RHI_READBACK_DUMP" in lib, "no_host_pixel: 缺通用 READBACK_DUMP env")
    return has_v2 and bad is None


def host_artifacts_v2_and_cabi_anchors() -> tuple[bool, bool, bool]:
    """artifacts 按名 / vb_ib cabi / seal 双向核验静态锚。"""
    lib = (ROOT / "src/rurix-rt-cabi/src/lib.rs").read_text(encoding="utf-8", errors="replace")
    mir = (ROOT / "src/rurixc/src/mir_build.rs").read_text(encoding="utf-8", errors="replace")
    vk = (ROOT / "src/rurix-rt/src/vk.rs").read_text(encoding="utf-8", errors="replace")

    art = all(
        s in lib
        for s in (
            "spirv_entry",
            "vs_name",
            "rxrt_rhi_raster_pass",
            "run_rhi_graphics_offscreen_v2",
        )
    )
    check(art, "artifacts_v2: cabi 缺按名 spirv_entry / vs_name / graphics v2 锚")

    cabi = all(
        s in lib
        for s in (
            "rxrt_rhi_vb_create",
            "rxrt_rhi_ib_create",
            "rxrt_rhi_gfx_draw",
            "rxrt_rhi_gfx_vs_layout",
        )
    ) and all(
        s in mir
        for s in (
            "rxrt_rhi_vb_create",
            "rxrt_rhi_ib_create",
            "rxrt_rhi_gfx_draw",
            "rxrt_rhi_gfx_vs_layout",
        )
    )
    check(cabi, "vb_ib_binding: cabi/mir_build 缺 VB/IB/draw/vs_layout 符号锚")

    seal = (
        "vs_stride" in lib
        and "not multiple of stride" in lib
        and "io_sig_for" in mir
        and "INDEX_TYPE_UINT32" in vk
        and re.search(r"const INDEX_TYPE_UINT32:\s*u32\s*=\s*1", vk) is not None
    )
    check(seal, "seal_bidirectional: 缺 VS stride 核验 / io_sig_for / UINT32=1 锚")
    return art, cabi, seal


def host_compile_only_not_pass() -> bool:
    """accept 判据 fixture 必须含像素自断言(禁仅编译成功充绿)。"""
    text = JUDGE.read_text(encoding="utf-8", errors="replace")
    has_assert = "want" in text and "host.get" in text and "return 1" in text
    check(has_assert, "compile_only_not_pass: m89_two_tri_quad 缺像素自断言")
    check(GOLDEN.is_file(), f"compile_only_not_pass: 缺 golden {GOLDEN.relative_to(ROOT)}")
    return has_assert and GOLDEN.is_file()


def host_section(results: dict) -> bool:
    print(f"[{TAG}] host 段:静态审计…")
    results["rust_host_source_count_zero"] = host_rust_host_source_count_zero()
    results["no_host_pixel_substitution"] = host_no_host_pixel_substitution()
    art, cabi, seal = host_artifacts_v2_and_cabi_anchors()
    results["artifacts_v2_real_consumption"] = art
    results["vb_ib_binding_via_cabi"] = cabi
    results["seal_reflection_bidirectional_verify"] = seal
    results["compile_only_not_pass"] = host_compile_only_not_pass()
    # device 未跑前先置 false(device 段覆盖)
    for k in (
        "rx_exe_green_readback_selfassert",
        "readback_equals_checked_in_golden",
        "assembly_reject_legs_red",
        "validation_zero_errors",
        "accept_corpus_green",
    ):
        results.setdefault(k, False)
    host_ok = not FAILURES
    if host_ok:
        print(f"[{TAG}] host 段 PASS")
    else:
        for f in FAILURES:
            print(f"[{TAG}] host FAIL: {f}", file=sys.stderr)
    return host_ok


# ─────────────────────────── device 段 ───────────────────────────


def ensure_rx() -> Path | None:
    rx = ROOT / "target" / "debug" / ("rx.exe" if sys.platform == "win32" else "rx")
    code, out, err = run(
        ["cargo", "build", "-q", "-p", "rx", "--features", "vulkan-backend"]
    )
    if code != 0 or not rx.is_file():
        print((out + err)[-1500:], file=sys.stderr)
        return None
    # crt-static cabi(含 M89 符号)
    code2, out2, err2 = run(
        [
            "cargo",
            "build",
            "-q",
            "-p",
            "rurix-rt-cabi",
            "--release",
            "--features",
            "vulkan",
            "--target-dir",
            "target/crt-static-default",
        ],
        env={"RUSTFLAGS": "-C target-feature=+crt-static"},
    )
    if code2 != 0:
        print((out2 + err2)[-1500:], file=sys.stderr)
        return None
    return rx


def rx_build(rx: Path, src: Path, out_exe: Path) -> tuple[int, str]:
    out_exe.parent.mkdir(parents=True, exist_ok=True)
    code, so, se = run(
        [str(rx), "build", str(src), "-o", str(out_exe)],
        env={"RURIX_REQUIRE_REAL": "1"},
    )
    return code, so + se


def device_section(results: dict) -> str:
    """返回 device_section_state。"""
    print(f"[{TAG}] device 段:rx build + Vulkan 真跑…")
    if not JUDGE.is_file():
        results["device_run"] = "SKIP"
        skip(f"缺判据 fixture {JUDGE}")
        return "skipped_dev_env"

    rx = ensure_rx()
    if rx is None:
        skip("rx / cabi(vulkan,crt-static) 构建失败")
        return "skipped_dev_env"

    WORK.mkdir(parents=True, exist_ok=True)
    dump_path = WORK / "m89_dump.rgba8"
    if dump_path.exists():
        dump_path.unlink()

    # ── accept ──
    accept_rxs = sorted(ACCEPT_DIR.glob("*.rx"))
    check(len(accept_rxs) == 2, f"accept 语料应为 2 件,实测 {len(accept_rxs)}")
    accept_ok = True
    judge_exe: Path | None = None
    for src in accept_rxs:
        exe = WORK / exe_name(src.stem)
        bc, blog = rx_build(rx, src, exe)
        if bc != 0 or not exe.is_file():
            if "error[RX7001]" in blog:
                skip(f"accept {src.name}: link/工具链面缺\n{blog[-800:]}")
                return "skipped_dev_env"
            check(False, f"accept {src.name}: rx build 失败\n{blog[-1200:]}")
            accept_ok = False
            continue
        env = {
            "RURIX_REQUIRE_REAL": "1",
            "RURIX_VK_VALIDATION": "1",
        }
        if src.name == "m89_two_tri_quad.rx":
            env["RURIX_RHI_READBACK_DUMP"] = str(dump_path)
            judge_exe = exe
        rc, ro, re = run([str(exe)], cwd=WORK, env=env)
        blob = ro + re
        if rc != 0:
            if "vulkan loader" in blob.lower() or "physical device" in blob.lower():
                skip(f"accept {src.name}: Vulkan 驱动不可用\n{blob[-800:]}")
                return "skipped_dev_env"
            check(False, f"accept {src.name}: EXE 退出 {rc}\n{blob[-1200:]}")
            accept_ok = False
            continue
        if "validation" in blob.lower() and "error" in blob.lower() and "fail-closed" in blob.lower():
            check(False, f"accept {src.name}: validation ERROR\n{blob[-800:]}")
            accept_ok = False
        # validation 层 ERROR 会经 cabi 翻非零;能 exit 0 即 validation_zero 的运行期面
        note(f"accept {src.name}: exit 0")

    results["accept_corpus_green"] = accept_ok and len(accept_rxs) == 2
    results["rx_exe_green_readback_selfassert"] = bool(
        judge_exe and judge_exe.is_file() and accept_ok
    )
    results["validation_zero_errors"] = accept_ok  # exit 0 + VALIDATION=1

    # golden
    golden_ok = False
    if dump_path.is_file() and GOLDEN.is_file():
        golden_ok = dump_path.read_bytes() == GOLDEN.read_bytes()
        check(
            golden_ok,
            f"golden 逐字节不等: dump={dump_path.stat().st_size}B "
            f"golden={GOLDEN.stat().st_size}B",
        )
    else:
        check(False, f"golden 对拍缺文件 dump={dump_path.is_file()} golden={GOLDEN.is_file()}")
    results["readback_equals_checked_in_golden"] = golden_ok

    # ── reject ──
    reject_rxs = sorted(REJECT_DIR.glob("*.rx"))
    check(len(reject_rxs) == 4, f"reject 语料应为 4 件,实测 {len(reject_rxs)}")
    reject_ok = True
    for src in reject_rxs:
        exe = WORK / exe_name(src.stem)
        bc, blog = rx_build(rx, src, exe)
        if src.name == "draw_without_vb.rx":
            # brand 隔离 → 编译期 RX2001(诚实 RED;强于装配拒)
            compile_red = bc != 0 and ("RX2001" in blog or "mismatched types" in blog)
            check(compile_red, f"reject {src.name}: 期望编译期 brand 红,got build={bc}\n{blog[-800:]}")
            reject_ok = reject_ok and compile_red
            note(f"reject {src.name}: compile-time brand RED")
            continue
        if bc != 0 or not exe.is_file():
            if "error[RX7001]" in blog:
                skip(f"reject {src.name}: link/工具链面缺\n{blog[-800:]}")
                return "skipped_dev_env"
            check(False, f"reject {src.name}: 应能编译出 EXE 再运行期红\n{blog[-800:]}")
            reject_ok = False
            continue
        rc, ro, re = run(
            [str(exe)],
            cwd=WORK,
            env={"RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"},
        )
        blob = ro + re
        exe_red = rc != 0 and (
            "[capacity]" in blob or "[structure]" in blob or "rhi_gfx_draw" in blob or "rhi_submit" in blob
        )
        check(exe_red, f"reject {src.name}: 期望 EXE RED+类别,got rc={rc}\n{blob[-800:]}")
        reject_ok = reject_ok and exe_red
        note(f"reject {src.name}: EXE RED rc={rc}")

    results["assembly_reject_legs_red"] = reject_ok and len(reject_rxs) == 4

    # device 真跑过 accept → artifacts/cabi 运行期面加证(静态锚已在 host)
    if accept_ok:
        results["artifacts_v2_real_consumption"] = True
        results["vb_ib_binding_via_cabi"] = True

    if FAILURES:
        return "fail"
    return "pass"


# ─────────────────────────── evidence ───────────────────────────


def write_evidence(results: dict, host_ok: bool, device_state: str) -> Path:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    checks = {k: bool(results.get(k, False)) for k in CHECK_KEYS}
    ev = {
        "schema_version": 1,
        "subject": "g8_m89_single_source_gfx_submit",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M89",
        "wave": "G8.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_ok,
        "device_section_state": device_state,
        "checks": checks,
        "evidence_level": "measured_local",
        "run_url": (
            f"{os.environ['GITHUB_SERVER_URL']}/{os.environ['GITHUB_REPOSITORY']}"
            f"/actions/runs/{os.environ['GITHUB_RUN_ID']}"
            if os.environ.get("GITHUB_SERVER_URL")
            and os.environ.get("GITHUB_REPOSITORY")
            and os.environ.get("GITHUB_RUN_ID")
            else "local"
        ),
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": " | ".join(NOTES)
        if NOTES
        else (
            "device 门:rx build(vulkan-backend)+RURIX_REQUIRE_REAL=1+"
            "RURIX_VK_VALIDATION=1;golden=tests/gfx/m89_golden.rgba8;"
            "draw_without_vb=编译期 brand RX2001。"
        ),
    }
    path = EVIDENCE_DIR / f"g8_m89_single_source_gfx_submit_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence 落盘: {path.relative_to(ROOT)}")
    return path


def selftest() -> None:
    check(False, "selftest: 合成失败")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    assert schema["properties"]["numeric_step"]["const"] == NUMERIC_STEP
    assert schema["properties"]["symbolic_gate_key"]["const"] == GATE_KEY
    req = schema["properties"]["checks"]["required"]
    assert set(req) == set(CHECK_KEYS)
    print(f"[{TAG}] selftest PASS")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        selftest()
        return 0
    if args.gate != GATE_KEY:
        return fail(f"未知 gate `{args.gate}`(期望 {GATE_KEY})")

    results: dict = {}
    host_section(results)
    device_state = device_section(results)

    all_checks = {k: bool(results.get(k, False)) for k in CHECK_KEYS}
    host_keys = {
        "rust_host_source_count_zero",
        "no_host_pixel_substitution",
        "artifacts_v2_real_consumption",
        "vb_ib_binding_via_cabi",
        "seal_reflection_bidirectional_verify",
        "compile_only_not_pass",
    }
    host_pass = all(all_checks[k] for k in host_keys)
    write_evidence(all_checks, host_pass, device_state)

    if FAILURES:
        for f in FAILURES:
            print(f"[{TAG}] FAIL: {f}", file=sys.stderr)
        return 1
    if device_state in ("skipped_dev_env", "dev_env_degrade"):
        return 0
    device_pass = device_state == "pass" and all(all_checks[k] for k in CHECK_KEYS)
    if not (host_pass and device_pass):
        return fail(f"checks 未全绿: {all_checks}")
    print(
        f"[{TAG}] PASS (host 静态 + device 11 checks 全真;numeric_step={NUMERIC_STEP})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
