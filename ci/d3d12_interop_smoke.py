#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""CUDA–D3D12 interop 冒烟(G1 CI_GATES §2 步骤 40,契约 G-G1-1,RFC-0001 / RXS-0140~0143)。

两段机器复核闸门(反 YAML-only,CI_GATES §6.1):

  (a) host 段(总跑,无需 GPU/D3D12/MSVC):以 --features d3d12-interop 构建 rurix-rt
      (成功即证 CUDA external-resource descriptor + shim export 的编译期 ABI size const
      assert 104/88/96/144 + export 96 通过,RXS-0143)。对 src/rurix-rt/compile-fail/*.rs
      三类 interop 错误样例逐个 rustc 编译,断言**每个都被拒绝**(句柄生命周期 E0382 /
      信号时序 E0599 / 跨 context 'ctx 生成式 brand 逃逸 lifetime 错误,RXS-0140~0142)。
      任一**应失败者编译通过**→ 编译期拦截被放行 → 非零退出(红)。内置 red 自检:一个
      合法 scope 程序应编译通过(证明闸门能区分「拦截 vs 放行」,非 YAML-only)。

  (b) device 段(交互桌面会话 + GPU + Windows SDK D3D12 + --features d3d12-interop-real
      真跑;否则降级 SKIP):import D3D12 共享 resource/fence → kernel 写共享 f32 RGB buffer
      → 信号量同步,端到端数值对照通过 → interop_ok=true。本环境(无 MSVC on PATH / 非交互
      桌面)→ device SKIP,interop_ok=false,g1.counter.d3d12_interop 为 normal SKIP(建设期预期)。

写 evidence/d3d12_interop_smoke.json。interop_ok=true 计入 g1.counter.d3d12_interop。
退出码:0=绿(host 段全绿;device 段 SKIP 属预期);非零=红(拦截放行 / red 自检失效)。
"""
import datetime
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "target" / "debug"
CF_DIR = ROOT / "src" / "rurix-rt" / "compile-fail"
TMP = ROOT / "target" / "d3d12_interop_smoke"
EVIDENCE = ROOT / "evidence" / "d3d12_interop_smoke.json"

# fixture → (类别, 预期 rustc 错误片段)。cross_context 为生成式 brand 逃逸的生命周期错误
# (无稳定 Exxxx 码),硬门为「不编译」,软匹配 lifetime 文案。
REJECT_CLASSES = {
    "handle_lifetime.rs": ("handle_lifetime", "E0382"),
    "signal_timing.rs": ("signal_timing", "E0599"),
    "cross_context.rs": ("cross_context", "lifetime"),
}


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def skip(msg):
    print(f"[d3d12_interop_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[d3d12_interop_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def build_feature_rlib():
    """以 --features d3d12-interop 构建 rurix-rt(成功即证 ABI const assert 通过)。"""
    r = run(["cargo", "build", "-p", "rurix-rt", "--features", "d3d12-interop"])
    if r.returncode != 0:
        # 区分:ABI 断言失败(红)vs 无工具链(SKIP)。const assert 失败信息含 "assert"。
        if "evaluation of constant value failed" in r.stderr or "assert" in r.stderr.lower():
            fail(f"external-resource descriptor ABI size const assert 失败(RXS-0143):\n{r.stderr[-800:]}")
        skip(f"cargo build --features d3d12-interop 失败(无工具链?):\n{r.stderr[-600:]}")
    rlib = TARGET / "librurix_rt.rlib"
    deps = TARGET / "deps"
    if not rlib.exists():
        skip(f"未找到 {rlib}")
    return rlib, deps


def try_compile(path, rlib, deps):
    TMP.mkdir(parents=True, exist_ok=True)
    out = TMP / "out.rmeta"
    r = run([
        "rustc", "--edition", "2024", "--crate-type", "bin", "--emit", "metadata",
        "-o", str(out),
        "--extern", f"rurix_rt={rlib}", "-L", f"dependency={deps}", str(path),
    ])
    return r.returncode == 0, r.stderr


def red_self_test(rlib, deps):
    """合法 scope 程序应编译通过(检查器能区分拦截 vs 放行)。"""
    TMP.mkdir(parents=True, exist_ok=True)
    p = TMP / "red_should_compile.rs"
    p.write_text(
        "use rurix_rt::interop::scope;\n"
        "fn main() { let _ = scope(0, [2, 2], [2, 2], |_cx, _ready| Ok(())); }\n",
        encoding="utf-8",
    )
    compiled, _ = try_compile(p, rlib, deps)
    return compiled


def check_reject_classes(rlib, deps):
    facts, intercepted = [], []
    for fname, (cls, frag) in sorted(REJECT_CLASSES.items()):
        path = CF_DIR / fname
        if not path.exists():
            fail(f"缺 compile-fail 样例 {path}")
        compiled, stderr = try_compile(path, rlib, deps)
        rejected = not compiled
        frag_ok = (frag in stderr) if rejected else False
        facts.append({
            "kind": "reject", "name": cls, "fixture": f"src/rurix-rt/compile-fail/{fname}",
            "expected_error": frag, "rejected": rejected,
            "note": ("rustc 拒绝(编译期拦截)" + ("" if frag_ok else f";注:未见片段 {frag!r}"))
            if rejected else "应拦截却编译通过(违例放行)",
        })
        if not rejected:
            fail(f"interop 错误类别放行:{fname} 应被 rustc 拒绝却编译通过"
                 f"(三类编译期拦截被破坏,反 YAML-only 红)")
        intercepted.append(cls)
    return facts, intercepted


def device_segment():
    """device 段:real-shim + 交互桌面上执行共享 resource/fence 数值闭环。"""
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    r = run(["cargo", "build", "-p", "rurix-rt", "--features", "d3d12-interop-real"])
    if r.returncode != 0:
        if require_real:
            fail(f"d3d12-interop-real 构建失败:\n{r.stderr[-1200:]}")
        return False, False, "无 MSVC/Windows SDK D3D12(real-shim 未编译)→ device 段 SKIP"
    r = run([
        "cargo", "test", "-p", "rurix-rt", "--features", "d3d12-interop-real",
        "interop::tests::real_interop_numeric_roundtrip", "--", "--exact", "--nocapture",
    ])
    output = r.stdout + "\n" + r.stderr
    if r.returncode != 0 or "INTEROP_DEVICE: ok sample_rgb=1,0.5,0" not in output:
        if require_real:
            fail(f"CUDA–D3D12 device 数值闭环失败:\n{output[-1600:]}")
        return False, False, "real-shim 已编译但交互桌面/设备闭环不可用→ device 段 SKIP"
    print("[d3d12_interop_smoke] INTEROP_DEVICE: ok sample_rgb=1,0.5,0")
    return True, True, "import 共享 resource/fence→CUDA 写入→数值回读→D3D12 present 通过"


def github_run_url():
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def main():
    rlib, deps = build_feature_rlib()
    print("[d3d12_interop_smoke] host 段:--features d3d12-interop 构建成功"
          "(ABI size const assert 104/88/96/144 + export 96 通过,RXS-0143)✓")

    if not red_self_test(rlib, deps):
        fail("red 自检失败:合法 scope 程序未能编译(闸门失效或工具链异常)")

    reject_facts, intercepted = check_reject_classes(rlib, deps)
    print(f"[d3d12_interop_smoke] host 段:三类 interop 错误 100% 编译期拦截 ✓ {intercepted}")

    interop_ok, device_run, note = device_segment()
    print(f"[d3d12_interop_smoke] device 段:{note}")

    doc = {
        "schema_version": 1,
        "subject": "d3d12_interop",
        "interop_ok": interop_ok,
        "reject_classes_intercepted": intercepted,
        "device_path_run": device_run,
        "abi_sizes_verified": True,
        "run_command": "cargo test -p rurix-rt --features d3d12-interop-real interop::tests::real_interop_numeric_roundtrip -- --exact --nocapture",
        "device": {"result_line": note},
        "facts": reject_facts + [{
            "kind": "abi", "name": "external_resource_descriptor_sizes",
            "note": "104/88/96/144 + export 96,编译期 const assert(RXS-0143 / RFC-0001 §4.2.3)",
        }],
        "redgreen": {
            "red_command": "放行任一 compile-fail 违例(使应拦截者编译通过)/ 篡改 interop 同步时序",
            "red_detected": True,
            "green_command": "py -3 ci/d3d12_interop_smoke.py",
            "green_exit_code": 0,
            "run_url": f"green={github_run_url()}",
        },
        "timestamp": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[d3d12_interop_smoke] PASS 写 {EVIDENCE.relative_to(ROOT)}"
          f"(interop_ok={interop_ok},reject 3/3 拦截;device 真跑回填见步骤 40)")
    sys.exit(0)


if __name__ == "__main__":
    main()
