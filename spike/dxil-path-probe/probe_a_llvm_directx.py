# SPIKE(RD-010) — A 路取证:LLVM DirectX 后端直接 emit DXIL(RFC-0003 §9 Q-D131 选项 A,结构首选)。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest:探到 DirectX target + dxc/dxv 则记实测,探不到如实 blocked + repro,绝不杜撰。
"""A 路探针 = LLVM DirectX target 直接从文本 LLVM IR emit DXIL。

与 NVPTX 后端同构(rurixc 产文本 LLVM IR → 外部 pin clang/llc 经 target 后端汇编;
D-205 外部 pin clang 22.1.x,toolchain.rs)。本探针只做可行性探测,不产生产 codegen:
  1. 定位 clang/llc(复刻 toolchain.rs 探测序),记版本。
  2. clang --print-targets / llc --version 检测 directx/dxil target 可用性。
     LLVM 官方 22.1.0 文档:DirectX 后端 experimental,不随发行版二进制 ship,
     须本地 LLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX 编译——故发行版 pin clang 极可能不含。
  3. target 可用 → 最小 LLVM IR → llc -mtriple=dxil-... emit DXIL → dxc/dxv validator 验证。
  4. target 不可用 → status=blocked + target_list_excerpt 实证 + repro 复现清单。
"""
from __future__ import annotations

import json
import sys

import _common as c

# 最小 LLVM IR(空 compute-shape 函数);仅用于 target 是否接受 dxil triple 的可行性探测,
# 非生产 codegen、非 spec 形态。target 不可用时根本不会用到。
MINIMAL_IR = """target triple = "dxil-unknown-shadermodel6.0-compute"
define void @main() {
entry:
  ret void
}
"""

DIRECTX_TOKENS = ("directx", "dxil")


def detect_directx_target(clang_path, llc_path) -> dict:
    """检测 DirectX/dxil target 是否编入工具链。返回 {available, excerpt, probe_command}。"""
    # 优先 llc --version 的 "Registered Targets" 段;补充 clang --print-targets。
    excerpts = []
    available = False
    probe_cmd = []
    if llc_path:
        res = c.run([llc_path, "--version"], timeout=15)
        out = res["stdout"] + res["stderr"]
        probe_cmd.append(f"{llc_path} --version")
        if out.strip():
            # 仅截取 Registered Targets 段,避免噪声
            lines = out.splitlines()
            cap = [l for l in lines if any(t in l.lower() for t in DIRECTX_TOKENS)]
            excerpts.append("llc --version registered-targets match: " + ("; ".join(s.strip() for s in cap) if cap else "(no directx/dxil target listed)"))
            if cap:
                available = True
    if clang_path:
        res = c.run([clang_path, "--print-targets"], timeout=15)
        out = res["stdout"] + res["stderr"]
        probe_cmd.append(f"{clang_path} --print-targets")
        if out.strip():
            lines = out.splitlines()
            cap = [l for l in lines if any(t in l.lower() for t in DIRECTX_TOKENS)]
            excerpts.append("clang --print-targets match: " + ("; ".join(s.strip() for s in cap) if cap else "(no directx/dxil target listed)"))
            if cap:
                available = True
    if not excerpts:
        excerpts.append("clang/llc 均不可用,无法枚举 target")
    return {
        "available": available,
        "excerpt": " || ".join(excerpts),
        "probe_command": " ; ".join(probe_cmd) if probe_cmd else "(no clang/llc located)",
    }


def probe() -> dict:
    clang_path, clang_src = c.locate_clang()
    llc_path = c.locate_tool("llc")
    dxc_path = c.locate_tool("dxc", env_var="RURIX_DXC")
    dxv_path = c.locate_tool("dxv", env_var="RURIX_DXV")

    versions = {
        "clang": c.tool_version(clang_path, ["--version"]),
        "llc": c.tool_version(llc_path, ["--version"]),
        "dxc": c.tool_version(dxc_path, ["--version"]),
        "dxv": c.tool_version(dxv_path, ["--version"]),
    }

    det = detect_directx_target(clang_path, llc_path)
    facts = []
    repro = []

    if det["available"]:
        # target 可用:尝试最小 IR → DXIL emit → 验证(measured)
        emit_ok = "n/a"
        validator_pass = "n/a"
        import tempfile
        import os
        tmpdir = tempfile.mkdtemp(prefix="dxil_spike_a_")
        ir_path = os.path.join(tmpdir, "min.ll")
        out_path = os.path.join(tmpdir, "min.dxil")
        # 仅写 ASCII 临时目录(对齐 ptxas.rs 非 ASCII 路径防御)
        with open(ir_path, "wb") as f:
            f.write(MINIMAL_IR.encode("ascii"))
        if llc_path:
            res = c.run([llc_path, ir_path, "-o", out_path], timeout=30)
            emit_ok = "pass" if (res["ok"] and os.path.isfile(out_path)) else "fail"
            facts.append({"kind": "dxil_emit", "name": "llc_min_ir", "note": f"rc={res['rc']} err={res['error']} stderr_head={(res['stderr'] or '')[:200]}"})
        if dxv_path and emit_ok == "pass":
            res = c.run([dxv_path, out_path], timeout=30)
            validator_pass = "pass" if res["ok"] else "fail"
            facts.append({"kind": "validator", "name": "dxv", "note": f"rc={res['rc']} stderr_head={(res['stderr'] or '')[:200]}"})
        elif emit_ok == "pass":
            validator_pass = "blocked"
            facts.append({"kind": "validator", "name": "dxv", "note": "dxv(DXIL validator)不可用,无法验证 emit 产物合规性"})
        status = "measured_local"
        path = {
            "status": status,
            "target_available": True,
            "probe_command": det["probe_command"],
            "target_list_excerpt": det["excerpt"],
            "dxil_emit_ok": emit_ok,
            "validator_pass": validator_pass,
            "shader_model_coverage": "探测最小 IR(shadermodel6.0-compute);完整 SM 覆盖矩阵留生产 codegen 实现 PR",
            "validator_compat": f"dxc={versions['dxc']} dxv={versions['dxv']}",
            "facts": facts,
            "repro": repro,
        }
    else:
        # target 不可用(LLVM 文档预期:DirectX 后端不随发行版 ship)→ blocked-honest
        repro = [
            "1. 取得编入 DirectX 后端的 LLVM:从源码编译 cmake -DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX(LLVM 官方 DirectXUsage.rst:experimental,不随 release 二进制 ship)。",
            "2. 确认 llc --version 的 Registered Targets 含 'directx - DirectX' / 'dxil';clang --print-targets 同。",
            "3. 安装 DXIL validator:dxc/dxv(DirectX Shader Compiler;含 dxil.dll 签名/验证)。",
            "4. 设 RURIX_DXC / RURIX_DXV 指向 dxc/dxv,或置于 PATH;重跑 probe_a_llvm_directx.py。",
            "5. 仍须核实:从任意 LLVM IR(非 HLSL→clang 路径)emit 合规 DXIL 的成熟度 + dxc validator 接受率 + shader model 覆盖。",
        ]
        path = {
            "status": "blocked",
            "target_available": False if (clang_path or llc_path) else c.UNAVAILABLE,
            "probe_command": det["probe_command"],
            "target_list_excerpt": det["excerpt"],
            "dxil_emit_ok": "blocked",
            "validator_pass": "blocked",
            "shader_model_coverage": "blocked(无 DirectX target,未能 emit)",
            "validator_compat": f"dxc={versions['dxc']} dxv={versions['dxv']}",
            "facts": [
                {"kind": "target_probe", "name": "directx_target", "note": "本环境 pin clang/llc 未编入 DirectX/dxil target(LLVM 官方:experimental,不随 release 二进制 ship,须本地 LLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX 编译)"},
            ],
            "repro": repro,
        }

    return {
        "path": "A",
        "label": "LLVM DirectX 后端直接 emit DXIL(结构首选,与 NVPTX 同构)",
        "clang_source": clang_src,
        "versions_subset": versions,
        "result": path,
    }


if __name__ == "__main__":
    print(json.dumps(probe(), ensure_ascii=False, indent=2))
    sys.exit(0)
