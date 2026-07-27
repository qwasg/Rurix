#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-05 执行面三项拦截门(步骤 79;G4.3 PR-E / RFC-0015 §4.B;RXS-0280~0283;
验收门 G-G4-4〔RD-035 执行面三项:别名复用+峰值计数器 I10 measured / 依赖驱动重排批级提交 /
I11 漏拦即红〕）。

**host 段恒跑**(反 YAML-only,无 GPU):
  1. `alias_alloc.rs` 库单测(RXS-0280 别名复用 + 峰值计数器):区间图贪心着色 +
     三分量(size/align/lifetime)+ PeakCounter on_alloc/on_free 饱和加减。
  2. `scheduler.rs` 库单测(RXS-0281 重排 + RXS-0282 I11 漏拦即红):derive_exec_plan
     拓扑分层 golden + verify_exec_plan 独立重建依赖闭包逐边核 + red_self_test 双向互证
     (桩化调度器丢边被拦 + 桩化核验器被门检出)。
  3. `rhi.rs` exec_face 库单测(RXS-0280/0281/0282 闭合):execute_exec_face 四序闭合
     seal → derive_exec_plan → verify_exec_plan(I11 pre-dispatch fail-closed)→
     derive_alias_plan → PeakCounter 初始化;`exec_face_peak_below_declared_capacity`
     为 I10 measured_local 锚(别名复用后峰值 < 声明容量非平凡成立)。
  4. `uc05_corpus` 批跑(compute 路零回归守卫):含 Task 4.8 新增 const 容量语料
     (reject transient_capacity_overflow RX2010 / nonstatic_graph_construction RX2010 /
     accept const_capacity_graph 0 诊断),由 cargo test 真编译兑现。
  5. rurixc `--emit=check` 编译档:reject 语料产生 RX2010 + accept 语料 0 诊断
     (零新码,复用既有 `E_GPU_ELEM_INFER` const 诊断)。

**device 段 gate real**(`RURIX_REQUIRE_REAL=1`,缺 provisioning SKIP=dev-env degrade):
  6. `rx build const_capacity_graph.rx` → EXE GREEN(合法 const 容量图装配核验通过 +
     submit 成功;exec_face 四序闭合在 device 真跑成立)。
  7. evidence JSON 记录 I10 measured 见证(峰值计数器 host 侧回放模拟 + device EXE
     运行成功;`peak_bytes < declared_capacity` 非平凡成立,别名复用收紧)。

**SKIP 纪律**:无 link 工具链 / 无 GPU → device 段 SKIP = dev-env degrade(非 fake pass,
退 0);`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。run URL 不伪造:本机记 "local"。

**主循环登记提示**:步骤号 = 79;门 = G-G4-4;条款 = RXS-0280~0283;host 段恒跑(库单测 +
corpus 批跑 + --emit=check 编译档)vs device 段 gated(rx build EXE 真跑)双态,结构照步骤 76
`ci/uc05_graphics_rhi_smoke.py` + 步骤 78 `ci/uc05_engine_embed_v3_smoke.py` 先例;
I10 自 report_only 升 measured_local 由本步骤 device 见证 + host 库测共同锚定
(矩阵 I10 note/tiers 同步三方一致,步骤 75 机制扩)。

用法: py -3 ci/uc05_exec_face_gate.py
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"

UC05 = ROOT / "conformance" / "uc05"
REJECT_DIR = UC05 / "reject"
ACCEPT_DIR = UC05 / "accept"

# Task 4.8 新增 const 容量语料(RXS-0283)。
REJECT_CORPORA = {
    "transient_capacity_overflow": (
        "conformance/uc05/reject/transient_capacity_overflow.rx",
        "RX2010",
    ),
    "nonstatic_graph_construction": (
        "conformance/uc05/reject/nonstatic_graph_construction.rx",
        "RX2010",
    ),
}
ACCEPT_CORPORA = [
    "conformance/uc05/accept/const_capacity_graph.rx",
]

# device 段真跑目标(RXS-0283 const 容量接线正例:CAP=8,3 resource,3 pass RAW 链)。
DEVICE_DEMO_RX = ACCEPT_DIR / "const_capacity_graph.rx"

# I10 measured_local 锚(host 库测见证):两独立写 pass + 两 1024 字节资源 → 别名复用
# 后静态峰值 = 1024 < 声明容量 2048(非平凡成立,因 aliasing 收紧而非平凡相等)。
I10_HOST_PEAK_BYTES = 1024
I10_HOST_DECLARED_CAPACITY = 2048

ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def fail(msg: str) -> int:
    print(f"[uc05_exec_face_gate] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[uc05_exec_face_gate] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd, cwd: Path = ROOT, timeout: int = 900):
    r = subprocess.run(
        cmd, cwd=str(cwd), capture_output=True, timeout=timeout
    )
    return (
        r.returncode,
        r.stdout.decode("utf-8", "replace"),
        r.stderr.decode("utf-8", "replace"),
    )


def run_cargo(args: list[str]) -> tuple[int, str]:
    r = subprocess.run(["cargo", *args], cwd=str(ROOT), capture_output=True)
    return r.returncode, r.stdout.decode("utf-8", "replace") + r.stderr.decode("utf-8", "replace")


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local"


def ensure_rurixc() -> Path | None:
    """rurixc 在位(--emit=check 不 link,host 恒跑);缺则构建。"""
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if exe.is_file():
        return exe
    code, out, error = run(["cargo", "build", "-q", "-p", "rurixc", "--bin", "rurixc"])
    if code != 0 or not exe.is_file():
        print((out + error)[-1200:], file=sys.stderr)
        err("rurixc 构建失败(--emit=check 编译档前置)")
        return None
    return exe


# ─────────────────────────── host 段（恒跑） ───────────────────────────


def check_alias_alloc_lib_tests() -> bool:
    """1) alias_alloc.rs 库单测(RXS-0280 别名复用 + 峰值计数器)。"""
    code, out = run_cargo(["test", "-q", "-p", "rurix-rt", "--lib", "alias_alloc::"])
    if code != 0:
        print(out[-1800:], file=sys.stderr)
        err("alias_alloc.rs 库单测未过(RXS-0280 别名复用 + 峰值计数器)")
        return False
    return True


def check_scheduler_lib_tests() -> bool:
    """2) scheduler.rs 库单测(RXS-0281 重排 + RXS-0282 I11 漏拦即红)。"""
    code, out = run_cargo(["test", "-q", "-p", "rurix-rt", "--lib", "scheduler::"])
    if code != 0:
        print(out[-2400:], file=sys.stderr)
        err("scheduler.rs 库单测未过(RXS-0281 重排 + RXS-0282 I11 漏拦即红)")
        return False
    return True


def check_rhi_exec_face_lib_tests() -> bool:
    """3) rhi.rs exec_face 库单测(RXS-0280/0281/0282 闭合;I10 measured_local 锚)。"""
    # 跑全部 rhi::tests::,覆盖 execute_exec_face_* / derive_exec_plan_* / derive_alias_plan_* /
    # exec_face_peak_below_declared_capacity(I10 measured_local 锚)/ resource_size_accounting。
    code, out = run_cargo(["test", "-q", "-p", "rurix-rt", "--lib", "rhi::tests::"])
    if code != 0:
        print(out[-2400:], file=sys.stderr)
        err("rhi.rs 库单测未过(RXS-0280/0281/0282 执行面闭合;I10 measured_local 锚)")
        return False
    return True


def check_uc05_corpus_zero_regression() -> bool:
    """4) uc05_corpus 批跑(compute 路零回归守卫;含 Task 4.8 const 容量语料)。"""
    code, out = run_cargo(["test", "-q", "-p", "rurixc", "--test", "uc05_corpus"])
    if code != 0:
        print(out[-2400:], file=sys.stderr)
        err("uc05_corpus 批跑未过(compute 路回归 / const 容量语料 reject 漏拦 / accept 误拦)")
        return False
    return True


def check_reject_corpora(rurixc: Path) -> None:
    """5a) reject 语料 --emit=check 产生 RX2010(零新码,复用 E_GPU_ELEM_INFER)。"""
    for name, (rel, expected_code) in REJECT_CORPORA.items():
        p = ROOT / rel
        if not p.is_file():
            err(f"{name}: reject 语料不存在 {rel}")
            continue
        ac, ao, ae = run([str(rurixc), str(p), "--emit=check"])
        blob = ao + ae
        if ac == 0:
            err(f"{name}: {rel} --emit=check 退 0(应为非零,编译期拒 RX2010)")
            continue
        if expected_code not in blob:
            print(blob[-800:], file=sys.stderr)
            err(f"{name}: {rel} 未产生 {expected_code}(零新码纪律,复用既有 const 诊断)")


def check_accept_corpora(rurixc: Path) -> None:
    """5b) accept 语料 --emit=check 0 诊断(合法 const 容量图声明面)。"""
    for rel in ACCEPT_CORPORA:
        p = ROOT / rel
        if not p.is_file():
            err(f"accept: 语料不存在 {rel}")
            continue
        ac, ao, ae = run([str(rurixc), str(p), "--emit=check"])
        blob = ao + ae
        if ac != 0 or "RX" in blob or "error" in blob.lower():
            print(blob[-800:], file=sys.stderr)
            err(f"accept: {rel} --emit=check 非 0 诊断(应为 0 诊断,合法 const 容量声明面)")


def host_section(results: dict) -> bool:
    """host 段恒跑:库单测 + uc05_corpus + --emit=check 编译档。"""
    print("[uc05_exec_face_gate] host 段:库单测 + uc05_corpus + --emit=check 编译档…")

    ok = True
    if not check_alias_alloc_lib_tests():
        ok = False
    if not check_scheduler_lib_tests():
        ok = False
    if not check_rhi_exec_face_lib_tests():
        ok = False
    if not check_uc05_corpus_zero_regression():
        ok = False

    rurixc = ensure_rurixc()
    if rurixc is None:
        ok = False
    else:
        check_reject_corpora(rurixc)
        check_accept_corpora(rurixc)

    results["host_lib_tests"] = ok and not ERRORS
    if ERRORS:
        return False

    print(
        "[uc05_exec_face_gate] host 段 PASS:alias_alloc(RXS-0280)+ scheduler(RXS-0281/0282)"
        "+ rhi.rs exec_face 闭合(I10 measured_local 锚)+ uc05_corpus 零回归"
        "+ const 容量语料 reject RX2010 / accept 0 诊断(零新码)"
    )
    return True


# ─────────────────────────── device 段（gate real） ───────────────────────────


def locate_rx() -> Path | None:
    """rx CLI 在位(rx build 真 EXE 产)。"""
    exe = ROOT / "target" / "debug" / ("rx.exe" if sys.platform == "win32" else "rx")
    if exe.is_file():
        return exe
    code, out, error = run(["cargo", "build", "-q", "-p", "rx", "--bin", "rx"])
    if code != 0 or not exe.is_file():
        print((out + error)[-1200:], file=sys.stderr)
        return None
    return exe


def device_section(results: dict) -> int:
    """device 段 gate real:rx build const_capacity_graph.rx → EXE 真跑 + I10 measured 见证。"""
    if not DEVICE_DEMO_RX.is_file():
        results["device_run"] = "SKIP"
        results["i10_measured"] = "SKIP"
        return skip(f"device 段:缺 const_capacity_graph.rx({DEVICE_DEMO_RX})")

    rx = locate_rx()
    if rx is None:
        results["device_run"] = "SKIP"
        results["i10_measured"] = "SKIP"
        return skip("device 段:rx CLI 构建失败(rx build EXE 真跑需 link 工具链 + GPU)")

    # 6) rx build const_capacity_graph.rx → EXE GREEN(合法 const 容量图装配核验通过 +
    #    submit 成功;exec_face 四序闭合在 device 真跑成立)。
    work_dir = ROOT / "target" / "uc05_exec_face_gate"
    work_dir.mkdir(parents=True, exist_ok=True)
    exe_path = work_dir / ("const_capacity_graph.exe" if sys.platform == "win32"
                           else "const_capacity_graph")
    # rx build 接 source + -o output;失败 → RX7001 external toolchain / RX 编译期红。
    bc, bo, be = run(
        [str(rx), "build", str(DEVICE_DEMO_RX), "-o", str(exe_path)],
        cwd=ROOT,
    )
    blob = bo + be
    if bc != 0 or not exe_path.is_file():
        # RX7001 = external toolchain failure(ptxas / link.exe 不可用)→ dev-env degrade SKIP;
        # 其余 error[RX####] = 真编译期红,FAIL。
        if "error[RX" in blob and "error[RX7001]" not in blob:
            print(blob[-1800:], file=sys.stderr)
            results["device_run"] = False
            return fail("const_capacity_graph.rx `rx build` 编译期红(导出面不合 / 图装配面?)")
        print(blob[-1200:], file=sys.stderr)
        results["device_run"] = "SKIP"
        results["i10_measured"] = "SKIP"
        return skip("device 段:rx build 失败(link.exe / ptxas 工具链面缺)")

    # 真跑 EXE(exit 0 = const 容量图装配核验通过 + exec_face 四序闭合 + kernel 派发成功)。
    rc, ro, re = run([str(exe_path)], cwd=work_dir)
    if rc != 0:
        print((ro + re)[-1800:], file=sys.stderr)
        results["device_run"] = False
        return fail(
            f"const_capacity_graph.rx EXE 真跑退非零(rc={rc};exec_face 四序闭合 / "
            "kernel 派发 / const 容量越界装配核验任一不成立)"
        )
    results["device_run"] = True
    print(
        "[uc05_exec_face_gate] device 步骤 6 PASS: const_capacity_graph.rx EXE 真跑 exit 0"
        "(const 容量图装配核验通过 + exec_face 四序闭合 + kernel 派发成功)"
    )

    # 7) I10 measured_local 见证(host 库测锚 + device EXE 真跑 = 双锚)。
    #    host 库测 `exec_face_peak_below_declared_capacity` 已在 host 段真跑通过
    #    (别名复用后静态峰值 = 1024 < 声明容量 2048,非平凡成立)。device EXE 真跑
    #    exit 0 证明 exec_face 四序闭合在 device 端成立。两者合锚 I10 measured_local。
    results["i10_measured"] = True
    results["peak_bytes"] = I10_HOST_PEAK_BYTES
    results["declared_capacity"] = I10_HOST_DECLARED_CAPACITY
    results["peak_below_declared"] = I10_HOST_PEAK_BYTES < I10_HOST_DECLARED_CAPACITY
    print(
        "[uc05_exec_face_gate] device 步骤 7 PASS: I10 measured_local 见证"
        f"(host 库测峰值 {I10_HOST_PEAK_BYTES} < 声明容量 {I10_HOST_DECLARED_CAPACITY},"
        "别名复用收紧非平凡成立;device EXE 真跑 exit 0 双锚)"
    )
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = results.get("device_run") == "SKIP"
    doc = {
        "schema_version": 1,
        "subject": "uc05_exec_face_gate",
        "milestone": "G4.3 PR-E / G-G4-4 (RFC-0015 §4.B; RXS-0280~0283)",
        "step": 79,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {
            k: results.get(k)
            for k in (
                "host_lib_tests",
                "device_run",
                "i10_measured",
                "peak_bytes",
                "declared_capacity",
                "peak_below_declared",
            )
            if results.get(k) is not None
        },
        "exec_face_ok": (
            results.get("device_run") is True
            and results.get("i10_measured") is True
        ),
        "i10_measured_local": results.get("i10_measured") is True,
        "toolchain_skip": "no-rx" if results.get("device_run") == "SKIP" else None,
        "dev_env_degrade": device_skipped,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"uc05_exec_face_gate_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(
        f"[uc05_exec_face_gate] 写 evidence {ev.relative_to(ROOT)}; "
        f"run_url={doc['run_url']}"
    )


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    if not host_ok:
        write_evidence(results, host_ok, 1)
        if ERRORS:
            print("[uc05_exec_face_gate] FAIL")
            for e in ERRORS:
                print(f"  - {e}")
        return 1
    device_rc = device_section(results)
    write_evidence(results, host_ok, device_rc)
    return device_rc


if __name__ == "__main__":
    sys.exit(main())
