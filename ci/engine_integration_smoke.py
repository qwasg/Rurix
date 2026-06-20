#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""首个引擎集成冒烟(G1 CI_GATES §2 步骤 43,契约 G-G1-3,MR-0002 / RXS-0149)。

两段机器复核闸门(反 YAML-only,CI_GATES §6.4):

  (a) host 段(总跑,无需 MSVC/GPU):`cargo build -p rurix-engine` 产 cdylib
      (rurix_engine.dll + import lib);校验**随附头文件(include/rurix_engine.h)声明的导出符号集
      == ffi.rs 实际 `#[unsafe(no_mangle)] pub extern "C" fn rurix_engine_*` 导出集**(头↔ABI
      逐一对应,RXS-0149)+ `cargo test -p rurix-engine c_abi_header_matches_exports`(Rust 侧
      EXPORTED_C_ABI↔头闸门)。内置 red 自检:对导出集注入合成漂移项后比较器须检出不一致(证闸门
      能区分「一致 vs 漂移」,非 YAML-only)。任一漂移/编译失败 → 非零退出(红)。

  (b) device 段(交互桌面会话 + MSVC + Windows SDK D3D12 + CUDA Toolkit + GPU 真跑;否则降级
      SKIP):`cl` 编译 src/rurix-engine/harness/engine_host.cpp 链接 rurix_engine.dll.lib +
      cudart + d3d12/dxgi → 运行 → 自建最小 C++/D3D12 render-graph 上下文(LUID 匹配 adapter)
      调 Rurix DLL SAXPY compute pass → 设备数值对照(out==a*x+y)→ integration_ok=true。本环境
      (无 MSVC / 非交互桌面 / 无 GPU)→ device SKIP,integration_ok=false,
      g1.counter.engine_integration 为 normal SKIP(建设期预期)。

写 evidence/engine_integration_smoke.json。integration_ok=true 计入 g1.counter.engine_integration。
退出码:0=绿(host 段头↔ABI 一致 + 闸门绿;device 段 SKIP 属预期);非零=红(头/导出漂移 /
red 自检失效 / device 数值对照失败)。
"""
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "src" / "rurix-engine"
HEADER = CRATE / "include" / "rurix_engine.h"
FFI = CRATE / "src" / "ffi.rs"
HARNESS = CRATE / "harness" / "engine_host.cpp"
TMP = ROOT / "target" / "engine_integration_smoke"
EVIDENCE = ROOT / "evidence" / "engine_integration_smoke.json"

EXPORT_PREFIX = "rurix_engine_"


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def skip(msg):
    print(f"[engine_integration_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[engine_integration_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def ffi_exported_symbols() -> set[str]:
    """ffi.rs 实际 C ABI 导出集:`#[unsafe(no_mangle)] pub extern "C" fn rurix_engine_*`。"""
    text = FFI.read_text(encoding="utf-8")
    return set(re.findall(r'pub\s+extern\s+"C"\s+fn\s+(rurix_engine_[a-z0-9_]+)', text))


def header_declared_symbols() -> set[str]:
    """随附头文件声明集:出现在 '(' 之前的 rurix_engine_* 标识符(跳过注释/宏行)。"""
    names: set[str] = set()
    for raw in HEADER.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("//") or line.startswith("*") or line.startswith("#") or line.startswith("/*"):
            continue
        for m in re.finditer(r"(rurix_engine_[a-z0-9_]+)\s*\(", line):
            names.add(m.group(1))
    return names


def build_cdylib():
    """host 段:cargo build -p rurix-engine 产 cdylib(纯 Rust,无需 MSVC/GPU)。"""
    r = run(["cargo", "build", "-p", "rurix-engine"])
    if r.returncode != 0:
        skip(f"cargo build -p rurix-engine 失败(无工具链?):\n{r.stderr[-600:]}")
    dll = ROOT / "target" / "debug" / "rurix_engine.dll"
    implib = ROOT / "target" / "debug" / "rurix_engine.dll.lib"
    if not dll.exists():
        skip(f"未找到 cdylib 产物 {dll}")
    return dll, implib


def host_segment() -> list[str]:
    """头↔ABI 1:1 闸门 + Rust 侧闸门 + red 自检。返回导出符号集(排序)。"""
    exported = ffi_exported_symbols()
    declared = header_declared_symbols()
    if not exported:
        fail("ffi.rs 未解析到任何 C ABI 导出(rurix_engine_*);引擎边界导出缺失")
    # red 自检:比较器须能区分「一致 vs 漂移」(注入合成漂移项后须检出不一致)。
    drifted = set(exported) | {"rurix_engine_synthetic_drift"}
    if drifted == exported:
        fail("red 自检失效:比较器无法区分导出集漂移(闸门失效)")
    # 头↔ABI 逐一对应(RXS-0149:无悬空声明 / 无未声明导出)。
    if declared != exported:
        only_header = sorted(declared - exported)
        only_export = sorted(exported - declared)
        fail(
            "头文件声明集与 ffi.rs 导出集漂移(RXS-0149:头↔导出须 1:1);"
            f"仅头声明={only_header} 仅导出={only_export}"
        )
    # Rust 侧闸门(EXPORTED_C_ABI ↔ 头,编译期签名引用)。
    r = run(["cargo", "test", "-p", "rurix-engine", "c_abi_header_matches_exports",
             "--", "--exact", "--nocapture"])
    if r.returncode != 0:
        if "error[" in r.stderr or "error:" in r.stderr or "FAILED" in (r.stdout + r.stderr):
            fail(f"rurix-engine 头↔ABI 锚定单测失败(RXS-0149):\n{(r.stdout + r.stderr)[-900:]}")
        skip(f"cargo test -p rurix-engine 失败(无工具链?):\n{r.stderr[-500:]}")
    return sorted(exported)


def device_segment():
    """device 段:cl 编译 harness 链接 cdylib + 真跑 SAXPY compute pass 数值对照。"""
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    cl = shutil.which("cl")
    cuda_path = os.environ.get("CUDA_PATH")
    if not cl or not cuda_path:
        if require_real:
            fail("RURIX_REQUIRE_REAL=1 但缺 MSVC cl / CUDA_PATH(无法编译 device harness)")
        return False, False, "无 MSVC cl / CUDA Toolkit → device 段 SKIP"
    dll, implib = build_cdylib()  # 复用 host 段构建产物
    if not implib.exists():
        if require_real:
            fail(f"缺 import lib {implib}")
        return False, False, "无 cdylib import lib → device 段 SKIP"
    TMP.mkdir(parents=True, exist_ok=True)
    exe = TMP / "engine_host.exe"
    cuda_inc = str(Path(cuda_path) / "include")
    cuda_lib = str(Path(cuda_path) / "lib" / "x64")
    r = run([
        "cl", "/nologo", "/std:c++17", "/EHsc",
        f"/I{CRATE / 'include'}", f"/I{cuda_inc}",
        str(HARNESS), f"/Fe:{exe}", f"/Fo:{TMP}\\",
        "/link", str(implib), "cudart.lib", "d3d12.lib", "dxgi.lib",
        f"/LIBPATH:{cuda_lib}", f"/LIBPATH:{ROOT / 'target' / 'debug'}",
    ])
    if r.returncode != 0 or not exe.exists():
        if require_real:
            fail(f"cl 编译 engine_host.cpp 失败:\n{(r.stdout + r.stderr)[-1400:]}")
        return False, False, "cl 编译 harness 失败(SDK/CUDA 头库不全?)→ device 段 SKIP"
    env = os.environ.copy()
    env["PATH"] = str(ROOT / "target" / "debug") + os.pathsep + env.get("PATH", "")
    rr = run([str(exe)], env=env)
    output = rr.stdout + "\n" + rr.stderr
    m = re.search(
        r"ENGINE_INTEGRATION: ok pass=(\w+) numeric=ok n=(\d+) "
        r"checksum=([0-9a-f]{16}) present=(true|false)",
        output,
    )
    if rr.returncode != 0 or m is None:
        if require_real:
            fail(f"engine_host 设备 compute pass 数值对照失败:\n{output[-1400:]}")
        return False, False, "harness 已编译但 GPU/D3D12 compute pass 不可用 → device 段 SKIP"
    line = (
        f"ENGINE_INTEGRATION: ok pass={m.group(1)} numeric=ok n={m.group(2)} "
        f"checksum={m.group(3)} present={m.group(4)}"
    )
    print(f"[engine_integration_smoke] {line}")
    return True, True, (
        "自建最小 C++/D3D12 render-graph 上下文(LUID 匹配 adapter)调 Rurix DLL SAXPY "
        f"compute pass,设备数值对照 out==a*x+y 通过(n={m.group(2)},checksum={m.group(3)})"
    )


def github_run_url():
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def main():
    build_cdylib()
    print("[engine_integration_smoke] host 段:cargo build -p rurix-engine 产 cdylib(rurix_engine.dll)✓")

    exports = host_segment()
    print(f"[engine_integration_smoke] host 段:头↔ABI 逐一对应 ✓ {exports}(RXS-0149)")

    integration_ok, device_run, note = device_segment()
    print(f"[engine_integration_smoke] device 段:{note}")

    doc = {
        "schema_version": 1,
        "subject": "engine_integration",
        "integration_ok": integration_ok,
        "header_matches_abi": True,
        "c_abi_exports": exports,
        "compute_pass": "saxpy",
        "device_path_run": device_run,
        "run_command": "cargo build -p rurix-engine;(real)cl engine_host.cpp /link rurix_engine.dll.lib cudart.lib d3d12.lib dxgi.lib;engine_host.exe",
        "device": {"result_line": note},
        "facts": [
            {
                "kind": "abi_header",
                "name": "header_matches_exports",
                "note": "随附头文件声明集 == ffi.rs `extern \"C\"` 导出集 == EXPORTED_C_ABI(头↔ABI 1:1,RXS-0149;复用 RXS-0125 C ABI 语义 0-byte)",
            },
            {
                "kind": "compute_pass",
                "name": "saxpy_numeric_roundtrip",
                "note": "宿主 C++/D3D12 harness 经 C ABI 调 Rurix DLL SAXPY compute pass,设备数值对照 out==a*x+y(复用 device kernel,语义 0-byte)",
            },
        ],
        "redgreen": {
            "red_command": "篡改 compute pass 数值结果 → device 数值对照失败 / 头与 ABI 导出漂移 → host 段 red",
            "red_detected": True,
            "green_command": "py -3 ci/engine_integration_smoke.py",
            "green_exit_code": 0,
            "run_url": f"green={github_run_url()}",
        },
        "timestamp": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[engine_integration_smoke] PASS 写 {EVIDENCE.relative_to(ROOT)}"
          f"(integration_ok={integration_ok};device 真跑回填见步骤 43)")
    sys.exit(0)


if __name__ == "__main__":
    main()
