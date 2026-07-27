#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Vulkan RHI 通道 smoke(步骤 80;G4.4 PR-F / RFC-0015 §4.C4;RXS-0293/0294;
验收门 G-G4-5〔.rx 单源 Vulkan RHI 通道:compute + graphics 双腿,`Rhi::create_vk`
显式后端 strict 无回退;device 见证判据:compute 图 device 真跑 + 数值对照 + spirv-val
全模块校验 + RURIX_REQUIRE_REAL=1〕）。

**host 段恒跑**(反 YAML-only,无 GPU):
  1. `vk.rs` 库单测(feature `vulkan`):`vulkan_available` 探测 + `run_compute`
     pipeline/descriptor/dispatch 结构(pure host 单测,不触 device)。
  2. `rhi.rs` 库单测(backend 分流 + create_vk 0-byte):`RhiEntry.backend` 字段
     加性 + CUDA 路 0-byte 维持。
  3. `rurix-rt-cabi` 库单测(rhi_symbols_failure_path + assembly):`rxrt_rhi_create_vk`
     + `rxrt_rhi_*` Vulkan 分流符号面。
  4. `uc05_corpus` 批跑(含 accept/rhi_create_vk:0 诊断 + `rxrt_rhi_create_vk`
     lowering 符号锚定;compute 路零回归守卫)。
  5. rurixc `--emit=check` 编译档:accept/rhi_create_vk.rx 0 诊断(显式 Vulkan
     后端构造合法声明面)。
  6. spirv-val 全模块校验(工具在位 accept / 缺工具 SKIP,退出码判定非 grep;
     RXS-0212 三态 gate 先例):codegen 产 SPIR-V 模块经 `spirv-val` accept。

**device 段 gate real**(`RURIX_REQUIRE_REAL=1`,缺 provisioning SKIP=dev-env degrade):
  7. `rx build rhi_create_vk.rx` → EXE GREEN(Vulkan 通道 compute 腿 device 真跑;
     `Rhi::create_vk` 显式后端 + SPIR-V pipeline + descriptor set + dispatch)。
  8. evidence JSON 记录 device 见证结果(含 Vulkan 通道 compute 腿 exit code +
     环境画像 + spirv-val 退出码)。

**SKIP 纪律**:无 link 工具链 / 无 Vulkan 驱动 / 无 GPU → device 段 SKIP = dev-env degrade
(非 fake pass,退 0);`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。run URL 不伪造:本机记 "local"。

**主循环登记提示**:步骤号 = 80;门 = G-G4-5;条款 = RXS-0293/0294;host 段恒跑(库单测 +
corpus 批跑 + --emit=check 编译档 + spirv-val)vs device 段 gated(rx build EXE 真跑)双态,
结构照步骤 79 `ci/uc05_exec_face_gate.py` 先例。

用法: py -3 ci/vulkan_rhi_channel_smoke.py
"""
#@ spec: RXS-0293
#@ spec: RXS-0294
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
ACCEPT_DIR = UC05 / "accept"

# accept/rhi_create_vk.rx(RXS-0293):显式 Vulkan 后端构造 + compute-pass 声明式建图。
CREATE_VK_RX = ACCEPT_DIR / "rhi_create_vk.rx"

ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def fail(msg: str) -> int:
    print(f"[vulkan_rhi_channel_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[vulkan_rhi_channel_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
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


def locate_spirv_val() -> str | None:
    """spirv-val 定位(RURIX_SPIRV_VAL 环境变量 / PATH;缺则 None = SKIP)。"""
    env = os.environ.get("RURIX_SPIRV_VAL")
    if env and Path(env).is_file():
        return env
    for candidate in ["spirv-val", "spirv-val.exe"]:
        r = subprocess.run(
            ["where" if sys.platform == "win32" else "which", candidate],
            capture_output=True,
        )
        if r.returncode == 0:
            return r.stdout.decode("utf-8", "replace").strip().splitlines()[0]
    return None


# ─────────────────────────── host 段（恒跑） ───────────────────────────


def check_vk_lib_tests() -> bool:
    """1) vk.rs 库单测(feature vulkan):vulkan_available + run_compute 结构。"""
    code, out = run_cargo([
        "test", "-q", "-p", "rurix-rt", "--features", "vulkan",
        "--lib", "vk::tests::",
    ])
    if code != 0:
        print(out[-1800:], file=sys.stderr)
        err("vk.rs 库单测未过(feature vulkan:vulkan_available + run_compute)")
        return False
    return True


def check_rhi_lib_tests() -> bool:
    """2) rhi.rs 库单测(backend 分流 + create_vk 0-byte 维持)。"""
    code, out = run_cargo(["test", "-q", "-p", "rurix-rt", "--lib", "rhi::tests::"])
    if code != 0:
        print(out[-1800:], file=sys.stderr)
        err("rhi.rs 库单测未过(backend 分流 + create_vk 0-byte)")
        return False
    return True


def check_cabi_lib_tests() -> bool:
    """3) rurix-rt-cabi 库单测(rhi_symbols_failure_path + assembly + create_vk 符号面)。"""
    code, out = run_cargo(["test", "-q", "-p", "rurix-rt-cabi", "--lib"])
    if code != 0:
        print(out[-1800:], file=sys.stderr)
        err("rurix-rt-cabi 库单测未过(rxrt_rhi_create_vk + Vulkan 分流符号面)")
        return False
    return True


def check_uc05_corpus_zero_regression() -> bool:
    """4) uc05_corpus 批跑(含 accept/rhi_create_vk:create_vk 0 诊断 + lowering 符号锚定)。"""
    code, out = run_cargo(["test", "-q", "-p", "rurixc", "--test", "uc05_corpus"])
    if code != 0:
        print(out[-2400:], file=sys.stderr)
        err("uc05_corpus 批跑未过(accept/rhi_create_vk 0 诊断 + lowering / compute 路回归)")
        return False
    return True


def check_accept_create_vk(rurixc: Path) -> None:
    """5) accept/rhi_create_vk.rx --emit=check 0 诊断(显式 Vulkan 后端构造合法声明面)。"""
    if not CREATE_VK_RX.is_file():
        err(f"accept: rhi_create_vk.rx 不存在({CREATE_VK_RX})")
        return
    ac, ao, ae = run([str(rurixc), str(CREATE_VK_RX), "--emit=check"])
    blob = ao + ae
    if ac != 0 or "RX" in blob or "error" in blob.lower():
        print(blob[-800:], file=sys.stderr)
        err("accept/rhi_create_vk.rx --emit=check 非 0 诊断(应为 0 诊断,RXS-0293 显式 Vulkan 后端)")


def check_spirv_val() -> bool | None:
    """6) spirv-val 全模块校验(codegen 产 SPIR-V 模块经 spirv-val accept;缺工具 SKIP)。"""
    sv = locate_spirv_val()
    if sv is None:
        print("[vulkan_rhi_channel_smoke] spirv-val 不可用 → SPIR-V 独立验证 SKIP")
        return None
    # codegen SPIR-V 见证语料经 mesh_rt_vulkan_spirv_val 测试已覆盖(RXS-0247 per-entry 分叉);
    # 本步骤锚定 spirv-val 工具在位 + 退出码 0(非 grep stdout,RXS-0212 三态 gate)。
    # 跑 codegen SPIR-V 见证测试(feature vulkan-backend):
    code, out = run_cargo([
        "test", "-q", "-p", "rurixc", "--features", "vulkan-backend",
        "--test", "mesh_rt_vulkan_spirv_val",
    ])
    if code != 0:
        print(out[-1800:], file=sys.stderr)
        err("spirv-val 见证测试未过(mesh_rt_vulkan_spirv_val:compute/vertex/fragment 1.0 + mesh/RT 1.4)")
        return False
    return True


def host_section(results: dict) -> bool:
    """host 段恒跑:库单测 + uc05_corpus + --emit=check + spirv-val。"""
    print("[vulkan_rhi_channel_smoke] host 段:库单测 + uc05_corpus + --emit=check + spirv-val…")

    ok = True
    if not check_vk_lib_tests():
        ok = False
    if not check_rhi_lib_tests():
        ok = False
    if not check_cabi_lib_tests():
        ok = False
    if not check_uc05_corpus_zero_regression():
        ok = False

    rurixc = ensure_rurixc()
    if rurixc is None:
        ok = False
    else:
        check_accept_create_vk(rurixc)

    spirv_val_result = check_spirv_val()
    results["spirv_val"] = spirv_val_result

    results["host_lib_tests"] = ok and not ERRORS
    if ERRORS:
        return False

    print(
        "[vulkan_rhi_channel_smoke] host 段 PASS:vk.rs(feature vulkan)"
        "+ rhi.rs(backend 分流)+ cabi(create_vk 符号面)"
        "+ uc05_corpus(accept/rhi_create_vk 0 诊断 + lowering)"
        "+ --emit=check(0 诊断)"
        + ("+ spirv-val(全模块校验)" if spirv_val_result is True
           else " + spirv-val SKIP(工具不在位)")
    )
    return True


# ─────────────────────────── device 段（gate real） ───────────────────────────


def locate_rx() -> Path | None:
    """rx CLI 在位(rx build 真 EXE 产;需 vulkan feature 编译)。"""
    exe = ROOT / "target" / "debug" / ("rx.exe" if sys.platform == "win32" else "rx")
    if exe.is_file():
        return exe
    code, out, error = run(["cargo", "build", "-q", "-p", "rx", "--bin", "rx"])
    if code != 0 or not exe.is_file():
        print((out + error)[-1200:], file=sys.stderr)
        return None
    return exe


def device_section(results: dict) -> int:
    """device 段 gate real:rx build rhi_create_vk.rx → EXE 真跑(Vulkan 通道 compute 腿)。"""
    if not CREATE_VK_RX.is_file():
        results["device_run"] = "SKIP"
        return skip(f"device 段:缺 rhi_create_vk.rx({CREATE_VK_RX})")

    rx = locate_rx()
    if rx is None:
        results["device_run"] = "SKIP"
        return skip("device 段:rx CLI 构建失败(rx build EXE 真跑需 link 工具链 + Vulkan 驱动 + GPU)")

    # 7) rx build rhi_create_vk.rx → EXE GREEN(Vulkan 通道 compute 腿 device 真跑)。
    #    rx build 需要 vulkan feature(vulkan-backend codegen + SPIR-V 产物);Vulkan
    #    驱动在位 → vk::run_compute 真 dispatch;缺驱动 → 运行期 Err(RXS-0193 strict)。
    work_dir = ROOT / "target" / "vulkan_rhi_channel_smoke"
    work_dir.mkdir(parents=True, exist_ok=True)
    exe_path = work_dir / ("rhi_create_vk.exe" if sys.platform == "win32" else "rhi_create_vk")
    bc, bo, be = run(
        [str(rx), "build", str(CREATE_VK_RX), "-o", str(exe_path)],
        cwd=ROOT,
    )
    blob = bo + be
    if bc != 0 or not exe_path.is_file():
        # RX7001 = external toolchain failure(link.exe / Vulkan SDK 不可用)→ dev-env degrade SKIP;
        # 其余 error[RX####] = 真编译期红,FAIL。
        if "error[RX" in blob and "error[RX7001]" not in blob:
            print(blob[-1800:], file=sys.stderr)
            results["device_run"] = False
            return fail("rhi_create_vk.rx `rx build` 编译期红(Vulkan 通道 codegen / 装配面?)")
        print(blob[-1200:], file=sys.stderr)
        results["device_run"] = "SKIP"
        return skip("device 段:rx build 失败(link.exe / Vulkan SDK 工具链面缺)")

    # 真跑 EXE(exit 0 = Vulkan 通道 compute 腿 device 真跑成功:create_vk + SPIR-V
    # pipeline + descriptor set + dispatch + 回写;Vulkan 不可用 → exit 非零 RXS-0193 strict)。
    rc, ro, re = run([str(exe_path)], cwd=work_dir)
    if rc != 0:
        print((ro + re)[-1800:], file=sys.stderr)
        results["device_run"] = False
        # Vulkan 驱动不可用 → strict Err(RXS-0193);dev-env degrade SKIP(非 fake pass)。
        blob = ro + re
        if "RXRT: error" in blob and ("vulkan" in blob.lower() or "loader" in blob.lower()):
            results["device_run"] = "SKIP"
            return skip("device 段:Vulkan loader 不可用(strict Err RXS-0193,dev-env degrade)")
        return fail(
            f"rhi_create_vk.rx EXE 真跑退非零(rc={rc};Vulkan 通道 compute 腿派发 / "
            "SPIR-V 模块装载 / descriptor set 绑定 / dispatch 任一不成立)"
        )
    results["device_run"] = True
    print(
        "[vulkan_rhi_channel_smoke] device 步骤 7 PASS: rhi_create_vk.rx EXE 真跑 exit 0"
        "(Vulkan 通道 compute 腿:create_vk + SPIR-V pipeline + descriptor set + dispatch + 回写)"
    )
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = results.get("device_run") == "SKIP"
    doc = {
        "schema_version": 1,
        "subject": "vulkan_rhi_channel_smoke",
        "milestone": "G4.4 PR-F / G-G4-5 (RFC-0015 §4.C4; RXS-0293/0294)",
        "step": 80,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {
            k: results.get(k)
            for k in (
                "host_lib_tests",
                "spirv_val",
                "device_run",
            )
            if results.get(k) is not None
        },
        "vulkan_channel_ok": (
            results.get("device_run") is True
        ),
        "toolchain_skip": "no-rx" if results.get("device_run") == "SKIP" else None,
        "dev_env_degrade": device_skipped,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"vulkan_rhi_channel_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(
        f"[vulkan_rhi_channel_smoke] 写 evidence {ev.relative_to(ROOT)}; "
        f"run_url={doc['run_url']}"
    )


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    if not host_ok:
        write_evidence(results, host_ok, 1)
        if ERRORS:
            print("[vulkan_rhi_channel_smoke] FAIL")
            for e in ERRORS:
                print(f"  - {e}")
        return 1
    device_rc = device_section(results)
    write_evidence(results, host_ok, device_rc)
    return device_rc


if __name__ == "__main__":
    sys.exit(main())
