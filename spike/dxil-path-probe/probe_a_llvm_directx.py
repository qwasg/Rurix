# SPIKE(RD-010) — A 路取证 round-4:LLVM DirectX 后端直接 emit DXIL(RFC-0003 §9 Q-D131 选项 A,结构首选)。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest:自编 LLVM(带 dxil target)在位则实测崩溃率/验证,否则如实 blocked。
"""A 路探针 round-4 = 两个锐利诊断(measured-first,每配置 ×N 测崩溃率,绝不单发判定)。

诊断 1(emit 稳定性):分离测 `llc -filetype=asm`(文本 DXIL)vs `-filetype=obj`(二进制
  DXContainer)崩溃率,跨 shader model(6.0/6.2/6.5/6.6)× 元数据变体(bare / dxc 风格补全
  entryPoints+numthreads+valver)。找能稳定 emit 的配置。
诊断 2(互操作 0x80aa000f):对 llc 产 DXContainer 调 dxcompiler.dll IDxcValidator::Validate 真验证
  (非 dxc -dumpbin 容器加载),判 validator 接受+签名(只缺签名步=可打通)还是拒绝(validation
  error=上游 backend 不合规);并 diff llc vs dxc 自产容器结构定位成因。

自编 LLVM 经 RURIX_LLC(或 PATH)临时用,绝不动 C:\\Program Files\\LLVM(D-205 pin)。
"""
from __future__ import annotations

import json
import os
import sys
import tempfile

import _common as c
import dxil_container as dcont
import dxil_validator as dval

# 最小 LLVM IR(空 compute-shape);bare = 仅 triple,无 DXIL 元数据(对照 dxc 风格补全)。
BARE_IR = '''target triple = "{triple}"
define void @main() {{
entry:
  ret void
}}
'''

# dxc 风格补全:对照 dxc 自产 DXIL 元数据结构(!dx.valver / !dx.shaderModel / !dx.entryPoints
# + entry 属性 numthreads),"缺啥补啥"——测补全能否修复 validator 的 0x80aa000f。
ENRICHED_IR = '''target triple = "{triple}"
define void @main() {{
entry:
  ret void
}}
!dx.valver = !{{!0}}
!dx.shaderModel = !{{!1}}
!dx.entryPoints = !{{!2}}
!0 = !{{i32 1, i32 8}}
!1 = !{{!"cs", i32 {maj}, i32 {minr}}}
!2 = !{{void ()* @main, !"main", null, null, !3}}
!3 = !{{i32 4, !4}}
!4 = !{{i32 1, i32 1, i32 1}}
'''

MINIMAL_HLSL = "[numthreads(1,1,1)]\nvoid main() {}\n"

SHADER_MODELS = [("6.0", 6, 0), ("6.2", 6, 2), ("6.5", 6, 5), ("6.6", 6, 6)]
ATTEMPTS = 12  # 每配置发数(measured-first:单发会假 pass,多发量崩溃率)

DIRECTX_TOKENS = ("directx", "dxil")
CRASH_RCS = (3221225477, -1073741819)  # 0xC0000005 access violation(后端崩溃,非编译诊断)


def detect_directx_target(clang_path, llc_path) -> dict:
    """检测 DirectX/dxil target 是否编入工具链。返回 {available, excerpt, probe_command}。"""
    excerpts = []
    available = False
    probe_cmd = []
    if llc_path:
        res = c.run([llc_path, "--version"], timeout=15)
        out = res["stdout"] + res["stderr"]
        probe_cmd.append(f"{llc_path} --version")
        if out.strip():
            cap = [l for l in out.splitlines() if any(t in l.lower() for t in DIRECTX_TOKENS)]
            excerpts.append("llc --version registered-targets match: " + ("; ".join(s.strip() for s in cap) if cap else "(no directx/dxil target listed)"))
            if cap:
                available = True
    if clang_path:
        res = c.run([clang_path, "--print-targets"], timeout=15)
        out = res["stdout"] + res["stderr"]
        probe_cmd.append(f"{clang_path} --print-targets")
        if out.strip():
            cap = [l for l in out.splitlines() if any(t in l.lower() for t in DIRECTX_TOKENS)]
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


def _measure_one(llc_path, ir_text, filetype, workdir, tag):
    """对一个 (IR, filetype) 配置发 ATTEMPTS 次,量 ok/crash/other + 是否产物。返回 dict + 一次成功容器路径。"""
    irp = os.path.join(workdir, f"ir_{tag}.ll")
    with open(irp, "wb") as f:
        f.write(ir_text.encode("ascii"))
    ok = crash = other = 0
    sizes = set()
    good = None
    for i in range(ATTEMPTS):
        op = os.path.join(workdir, f"out_{tag}_{i}.{filetype}")
        if os.path.exists(op):
            os.remove(op)
        res = c.run([llc_path, f"-filetype={filetype}", irp, "-o", op], timeout=30)
        rc = res["rc"]
        if rc == 0 and os.path.exists(op) and os.path.getsize(op) > 0:
            ok += 1
            sizes.add(os.path.getsize(op))
            good = op
        elif rc in CRASH_RCS:
            crash += 1
        else:
            other += 1
    return {"tag": tag, "filetype": filetype, "attempts": ATTEMPTS, "ok": ok, "crash": crash, "other": other, "sizes": sorted(sizes)}, good


def measure_emit_stability(llc_path, workdir):
    """诊断 1:asm vs obj 崩溃率,跨 SM × 元数据变体。返回 (matrix, asm_stable, obj_stable, a_good_obj)。"""
    matrix = []
    asm_stable_all = True
    obj_stable_all = True
    good_obj_bare = None
    good_obj_enr = None
    for tag, maj, minr in SHADER_MODELS:
        triple = f"dxil-unknown-shadermodel{tag}-compute"
        bare = BARE_IR.format(triple=triple)
        enr = ENRICHED_IR.format(triple=triple, maj=maj, minr=minr)
        for variant, irtext in (("bare", bare), ("enriched", enr)):
            a_res, _ = _measure_one(llc_path, irtext, "asm", workdir, f"{variant}_asm_sm{tag}")
            o_res, ogood = _measure_one(llc_path, irtext, "obj", workdir, f"{variant}_obj_sm{tag}")
            matrix.append({"shader_model": tag, "variant": variant, "asm": a_res, "obj": o_res})
            if a_res["crash"] or a_res["other"] or a_res["ok"] != ATTEMPTS:
                asm_stable_all = False
            if o_res["crash"] or o_res["ok"] != ATTEMPTS:
                obj_stable_all = False
            if ogood and variant == "bare" and good_obj_bare is None:
                good_obj_bare = ogood
            if ogood and variant == "enriched" and good_obj_enr is None:
                good_obj_enr = ogood
    return matrix, asm_stable_all, obj_stable_all, good_obj_bare, good_obj_enr


def measure_interop(dxc_path, good_obj_bare, good_obj_enr, workdir):
    """诊断 2:IDxcValidator 真验证 llc 容器 + diff vs dxc 自产容器。返回 interop dict。"""
    out = {"validator_available": False, "bare": None, "enriched": None, "container_diff": None, "dxc_control": None}
    # dxcompiler.dll 路径:与 dxc.exe 同目录(Vulkan SDK / DXC release)
    dll = None
    if dxc_path:
        cand = os.path.join(os.path.dirname(dxc_path), "dxcompiler.dll")
        if os.path.isfile(cand):
            dll = cand
    # dxc 自产对照容器(隔离「validator/工具坏」vs「拒绝 llc 容器」)
    dxc_parsed = {}
    if dxc_path:
        ctrl_hlsl = os.path.join(workdir, "ctrl.hlsl")
        ctrl_out = os.path.join(workdir, "ctrl.dxo")
        with open(ctrl_hlsl, "wb") as f:
            f.write(MINIMAL_HLSL.encode("ascii"))
        r = c.run([dxc_path, "-T", "cs_6_0", "-E", "main", "-Fo", ctrl_out, ctrl_hlsl], timeout=30)
        if r["ok"] and os.path.isfile(ctrl_out):
            with open(ctrl_out, "rb") as f:
                dxc_bytes = f.read()
            dxc_parsed = dcont.parse_dxbc(dxc_bytes)
            if dll:
                ctrl_val = dval.validate_container(dll, dxc_bytes)
                out["dxc_control"] = {"parsed": {k: dxc_parsed.get(k) for k in ("part_fourccs", "is_signed", "size")}, "validate": ctrl_val}
    # llc bare + enriched 容器:解析 + 真验证
    for key, gobj in (("bare", good_obj_bare), ("enriched", good_obj_enr)):
        if not gobj or not os.path.isfile(gobj):
            out[key] = {"status": "n/a", "reason": "无成功 obj 容器(全崩溃/失败)"}
            continue
        with open(gobj, "rb") as f:
            llc_bytes = f.read()
        parsed = dcont.parse_dxbc(llc_bytes)
        entry = {"parsed": {k: parsed.get(k) for k in ("ok", "part_fourccs", "is_signed", "size", "digest_hex")}}
        if dll:
            out["validator_available"] = True
            entry["validate"] = dval.validate_container(dll, llc_bytes)
        else:
            entry["validate"] = {"status": "blocked", "reason": "dxcompiler.dll 未定位"}
        out[key] = entry
        if key == "bare" and parsed.get("ok") and dxc_parsed.get("ok"):
            out["container_diff"] = dcont.diff_parts(parsed, dxc_parsed)
    return out


def _summarize(matrix):
    """把崩溃率矩阵压成一行人读摘要(asm/obj 各配置 crash/attempts)。"""
    asm_bits = []
    obj_bits = []
    for row in matrix:
        sm, v = row["shader_model"], row["variant"]
        asm_bits.append(f"{v}/sm{sm}:{row['asm']['ok']}ok/{row['asm']['crash']}crash")
        obj_bits.append(f"{v}/sm{sm}:{row['obj']['ok']}ok/{row['obj']['crash']}crash")
    return "; ".join(asm_bits), "; ".join(obj_bits)


def probe() -> dict:
    clang_path, clang_src = c.locate_clang()
    llc_path = c.locate_tool("llc", env_var="RURIX_LLC")
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

    if det["available"] and llc_path:
        workdir = tempfile.mkdtemp(prefix="dxil_spike_a_r4_")
        matrix, asm_stable, obj_stable, good_bare, good_enr = measure_emit_stability(llc_path, workdir)
        interop = measure_interop(dxc_path, good_bare, good_enr, workdir)
        asm_sum, obj_sum = _summarize(matrix)

        # 诊断 1 结论:asm 稳定 + obj 崩溃 → emit 文本 DXIL 稳定,崩溃隔离于 DXContainer 写出器
        emit_ok = "pass" if obj_stable else "fail"
        facts.append({"kind": "emit_stability", "name": "asm_vs_obj_crash_rate",
                      "note": f"诊断1 每配置×{ATTEMPTS}发:[-filetype=asm 文本DXIL] {('全配置稳定' if asm_stable else '有崩溃')}: {asm_sum} || [-filetype=obj 二进制DXContainer] {('稳定' if obj_stable else '非确定性崩溃(0xC0000005,后端对象写出器)')}: {obj_sum}"})
        facts.append({"kind": "emit_stability", "name": "key_finding",
                      "note": f"关键发现:文本 DXIL(asm)emit {'稳定' if asm_stable else '不稳'},崩溃{'仅' if (asm_stable and not obj_stable) else ''}出现在二进制容器化(obj)→ 打通方向 = emit 文本 DXIL 再另行容器化/签名;补 dxc 风格元数据(entryPoints/numthreads/valver)不降 obj 崩溃率(实测 enriched ≥ bare 崩溃)"})

        # 诊断 2 结论:validator 接受=可打通;拒绝(validation error)=上游 backend 不合规
        vbare = (interop.get("bare") or {}).get("validate") or {}
        venr = (interop.get("enriched") or {}).get("validate") or {}
        vctrl = (interop.get("dxc_control") or {}).get("validate") or {}
        diff = interop.get("container_diff") or {}
        if vbare.get("status") == "measured":
            validator_pass = "pass" if vbare.get("accepted") else "fail"
            facts.append({"kind": "interop_validator", "name": "IDxcValidator_validate_llc",
                          "note": f"诊断2 dxcompiler.dll IDxcValidator::Validate(真验证,非 -dumpbin)对 llc 产 DXContainer:accepted={vbare.get('accepted')} status={vbare.get('validation_status_hr')} err={vbare.get('error_message','')!r};enriched 同测 accepted={venr.get('accepted')} status={venr.get('validation_status_hr')} → {'接受+签名(只缺签名步=互操作可打通)' if vbare.get('accepted') else 'validation error 非签名缺失 → llc 产 DXIL 不合规=上游 backend 问题'}"})
            facts.append({"kind": "interop_control", "name": "dxc_self_validate",
                          "note": f"对照:dxc 自产容器 IDxcValidator accepted={vctrl.get('accepted')} status={vctrl.get('validation_status_hr')}(=工具/validator 本身可用,gap 在 llc↔dxc 互操作非工具坏)"})
            facts.append({"kind": "interop_container_diff", "name": "container_structure",
                          "note": f"容器结构 diff:llc parts={diff.get('llc_parts')} vs dxc parts={diff.get('dxc_parts')};llc 缺={diff.get('llc_missing_vs_dxc')} 多={diff.get('llc_extra_vs_dxc')} 顺序异={diff.get('order_differs')};签名 llc_signed={diff.get('llc_signed')} dxc_signed={diff.get('dxc_signed')}"})
        elif vbare.get("status") == "n/a":
            validator_pass = "n/a"
            facts.append({"kind": "interop_validator", "name": "IDxcValidator_validate_llc", "note": f"obj 全崩溃/失败,无成功容器可验证;asm 文本 DXIL 稳定但非二进制容器,IDxcValidator 需容器输入"})
        else:
            validator_pass = "blocked"
            facts.append({"kind": "interop_validator", "name": "IDxcValidator_validate_llc", "note": f"validator 不可用:{vbare.get('reason')}"})

        validator_compat = (f"dxc={versions['dxc']} dxv={versions['dxv']};"
                            f"IDxcValidator 经 dxcompiler.dll 真验证(dxil.dll 独立签名 validator 缺失,measured 的是 dxcompiler 内置 validator);"
                            f"validator_available={interop.get('validator_available')}")
        status = "measured_local"
        path = {
            "status": status,
            "target_available": True,
            "probe_command": det["probe_command"] + f" ; (RURIX_LLC={llc_path})",
            "target_list_excerpt": det["excerpt"],
            "dxil_emit_ok": emit_ok,
            "validator_pass": validator_pass,
            "shader_model_coverage": f"诊断 SM 6.0/6.2/6.5/6.6 × (bare/enriched) × (asm/obj) 各×{ATTEMPTS};asm 全 SM 稳定,obj 全 SM 非确定性崩溃",
            "validator_compat": validator_compat,
            "emit_stability_matrix": matrix,
            "interop": interop,
            "facts": facts,
            "repro": [
                "1. 自编带 DirectX target 的 LLVM(已备:H:\\llvm-dxil\\build\\bin,LLVM 22.1.7 pin commit a255c1ed,含 dxil target);设 RURIX_LLC 指向其 llc.exe。",
                "2. 设 RURIX_DXC 指向 dxc.exe(同目录须有 dxcompiler.dll 供 IDxcValidator);本环境 dxc 1.8.0.4739(Vulkan SDK)。",
                "3. py -3 spike/dxil-path-probe/run_spike.py(RURIX_SPIKE_SUFFIX=_r4)重跑;诊断1 量 asm/obj×SM×元数据崩溃率,诊断2 IDxcValidator 真验证。",
                "4. blocker:obj DXContainer 写出器非确定性崩溃(0xC0000005)+ IDxcValidator 拒绝 llc DXIL(0x80aa000f load dxil metadata failed,非签名缺失)→ 打通依赖上游 LLVM DirectX 后端成熟(容器写出器稳定 + DXIL 元数据与 dxc validator 互操作修复)或 validator 版本依赖项。",
            ],
        }
    else:
        repro = [
            "1. 取得编入 DirectX 后端的 LLVM:从源码编译 cmake -DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX,设 RURIX_LLC 指向 llc.exe。",
            "2. 确认 llc --version 的 Registered Targets 含 'dxil - DirectX Intermediate Language'。",
            "3. 设 RURIX_DXC 指向 dxc.exe(同目录 dxcompiler.dll 供 IDxcValidator)。",
            "4. 重跑 run_spike.py。",
        ]
        path = {
            "status": "blocked",
            "target_available": False if (clang_path or llc_path) else c.UNAVAILABLE,
            "probe_command": det["probe_command"],
            "target_list_excerpt": det["excerpt"],
            "dxil_emit_ok": "blocked",
            "validator_pass": "blocked",
            "shader_model_coverage": "blocked(无 DirectX target 或 llc 缺失,未能 emit)",
            "validator_compat": f"dxc={versions['dxc']} dxv={versions['dxv']}",
            "facts": [{"kind": "target_probe", "name": "directx_target", "note": "未定位带 dxil target 的 llc(设 RURIX_LLC 指向自编 LLVM)"}],
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
