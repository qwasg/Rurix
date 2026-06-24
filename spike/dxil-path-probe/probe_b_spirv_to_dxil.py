# SPIKE(RD-010) — B 路取证:SPIR-V→DXIL 转译(RFC-0003 §9 Q-D131 选项 B,对照)。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest:探到转译链 + dxc/dxv 则记实测,探不到如实 blocked + repro,绝不杜撰。
"""B 路探针 = SPIR-V→DXIL 转译。

引入第二中间表示(SPIR-V)+ 外部转译依赖。候选转译链:
  - Mesa `spirv-to-dxil`(SPIR-V → DXIL 直接转译);
  - `SPIRV-Cross`(spirv-cross,SPIR-V → HLSL)→ `dxc`(HLSL → DXIL)。
SPIR-V 产出工具:`dxc -spirv` / `glslangValidator`。
本探针只做可用性 + 供应链成本取证,不产生产 codegen。

注:SPIR-V 在此仅作 DXIL 转译的内部中间表示(RFC-0003 §8),≠ SPIR-V 作为对外通用目标
(后者属死亡路线红线 3 / SG-003,不在本 spike 范围)。
"""
from __future__ import annotations

import json
import sys

import _common as c


def probe() -> dict:
    spirv_to_dxil = c.locate_tool("spirv-to-dxil", env_var="RURIX_SPIRV_TO_DXIL")
    spirv_cross = c.locate_tool("spirv-cross", env_var="RURIX_SPIRV_CROSS")
    dxc_path = c.locate_tool("dxc", env_var="RURIX_DXC")
    dxv_path = c.locate_tool("dxv", env_var="RURIX_DXV")
    glslang = c.locate_tool("glslangValidator") or c.locate_tool("glslang")

    versions = {
        "spirv_to_dxil": c.tool_version(spirv_to_dxil, ["--version"]),
        "spirv_cross": c.tool_version(spirv_cross, ["--version"]),
        "dxc": c.tool_version(dxc_path, ["--version"]),
        "dxv": c.tool_version(dxv_path, ["--version"]),
        "spirv_producer": c.tool_version(glslang, ["--version"]) if glslang else c.UNAVAILABLE,
    }

    # 转译链可用性:直接链(spirv-to-dxil)或组合链(spirv-cross + dxc)其一即可。
    direct_chain = spirv_to_dxil is not None
    combo_chain = (spirv_cross is not None) and (dxc_path is not None)
    available = direct_chain or combo_chain

    # 供应链成本实记(外部依赖来源/数量/第二中间表示)
    present = []
    if spirv_to_dxil:
        present.append("Mesa spirv-to-dxil")
    if spirv_cross:
        present.append("SPIRV-Cross")
    if dxc_path:
        present.append("dxc")
    if glslang:
        present.append("glslang(SPIR-V producer)")
    supply_chain = (
        "引入第二中间表示 SPIR-V + 外部转译依赖。"
        f"本环境探到:{', '.join(present) if present else '无'}。"
        "供应链长尾:Mesa(spirv-to-dxil)/ Khronos(SPIRV-Cross)/ Microsoft(dxc) 三独立来源,各自版本/许可/合规性需独立 pin 与审计(对齐 D-205 LLVM 单栈 pin 纪律的对照成本)。"
    )

    probe_cmd = "which spirv-to-dxil ; which spirv-cross ; which dxc ; which dxv ; which glslangValidator"
    facts = []
    repro = []

    if available:
        # 转译链可用:做最小可行性(此处仅探到工具即记 measured;最小端到端转译留实现 PR
        # 视具体 SPIR-V 输入而定,blocked-honest 不杜撰转译成功率数字)
        chain = "spirv-to-dxil(direct)" if direct_chain else "spirv-cross→dxc(combo)"
        facts.append({"kind": "translator_chain", "name": chain, "note": "转译链工具已就位;端到端转译合规率/确定性须以真实 SPIR-V 语料实测(留实现 PR,本 spike 不杜撰)"})
        emit_ok = "blocked"  # 工具在位但未跑真实语料端到端,诚实标 blocked 而非杜撰 pass
        validator_pass = "blocked"
        repro = [
            "工具链已就位,完成端到端实测还需:",
            "1. 准备代表性 SPIR-V 语料(经 dxc -spirv / glslangValidator 从 HLSL/GLSL 产)。",
            f"2. 跑 {chain} 转译为 DXIL。",
            "3. dxc/dxv 验证 DXIL 合规性,记录通过率 + shader model 覆盖。",
            "4. 评估转译层确定性(同输入同输出)与 strict-only(P-01)保真:无静默降级/回退。",
        ]
        determinism = "工具在位;确定性与 strict-only 保真须以真实语料实测(转译层是否引入非确定性/静默降级未验,blocked-honest 不预判)"
        status = "measured_local"  # 工具可用性/版本/供应链为实测;转译成功率诚实留 blocked
    else:
        emit_ok = "blocked"
        validator_pass = "blocked"
        determinism = "blocked(转译链不可用,未能评估)"
        status = "blocked"
        repro = [
            "1. 安装直接转译链:Mesa `spirv-to-dxil`(随 Mesa 发行;或自 mesa3d 源码构建),设 RURIX_SPIRV_TO_DXIL 或置 PATH。",
            "2. 或安装组合链:`SPIRV-Cross`(Khronos,spirv-cross 可执行)+ DirectX Shader Compiler `dxc`,分设 RURIX_SPIRV_CROSS / RURIX_DXC 或置 PATH。",
            "3. 安装 DXIL validator dxc/dxv;SPIR-V producer(dxc -spirv 或 glslangValidator)。",
            "4. 重跑 probe_b_spirv_to_dxil.py;再以真实 SPIR-V 语料实测端到端转译合规率/确定性/strict-only 保真。",
        ]
        facts.append({"kind": "translator_probe", "name": "spirv_to_dxil_chain", "note": "本环境未探到 Mesa spirv-to-dxil,亦无 SPIRV-Cross+dxc 组合链"})

    path = {
        "status": status,
        "translators_available": True if available else (False if present else c.UNAVAILABLE),
        "probe_command": probe_cmd,
        "translator_list_excerpt": f"present={present}; direct_chain={direct_chain}; combo_chain={combo_chain}",
        "dxil_emit_ok": emit_ok,
        "validator_pass": validator_pass,
        "supply_chain": supply_chain,
        "determinism_notes": determinism,
        "facts": facts,
        "repro": repro,
    }

    return {
        "path": "B",
        "label": "SPIR-V→DXIL 转译(对照;第二中间表示 + 外部转译依赖)",
        "versions_subset": versions,
        "result": path,
    }


if __name__ == "__main__":
    print(json.dumps(probe(), ensure_ascii=False, indent=2))
    sys.exit(0)
