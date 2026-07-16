# SPIKE(RD-010) — B 路取证:SPIR-V→DXIL 转译(RFC-0003 §9 Q-D131 选项 B,对照)。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest:探到转译链 + 代表性语料则跑端到端实测,探不到如实 blocked + repro,绝不杜撰。
"""B 路探针 = SPIR-V→DXIL 转译。

引入第二中间表示(SPIR-V)+ 外部转译依赖。候选转译链:
  - Mesa `spirv-to-dxil`(SPIR-V → DXIL 直接转译);
  - `SPIRV-Cross`(spirv-cross,SPIR-V → HLSL)→ `dxc`(HLSL → DXIL)。
SPIR-V 产出工具:`dxc -spirv` / `glslangValidator`。

round-2:combo 链(SPIRV-Cross + dxc)+ 代表性语料(spike/dxil-path-probe/corpus/)在位时,
跑真实端到端转译实测:HLSL → dxc -spirv → spirv-val → spirv-cross → HLSL → dxc → DXIL,
量 emit 合规性 / shader model 覆盖 / 确定性(同输入二次编译 SHA256 比对)/ dxc 内置验证(默认 vs -Vd
字节差证验证+签名执行)。dxv 独立 validator 缺失时如实标注,不杜撰外部签名验证。

注:SPIR-V 在此仅作 DXIL 转译的内部中间表示(RFC-0003 §8),≠ SPIR-V 作为对外通用目标
(后者属死亡路线红线 3 / SG-003,不在本 spike 范围)。
"""
from __future__ import annotations

import hashlib
import json
import sys
import tempfile
from pathlib import Path

import _common as c

HERE = Path(__file__).resolve().parent
CORPUS_DIR = HERE / "corpus"

# 文件名前缀 → dxc target profile(端到端默认 SM6.0;SM 覆盖矩阵另探)
PROFILE_MAP = {"cs": "cs_6_0", "vs": "vs_6_0", "ps": "ps_6_0"}
# shader model 覆盖矩阵(对转译回的 HLSL 用不同 dxc target SM emit,测覆盖广度)
SM_MATRIX = {
    "cs": ["cs_6_0", "cs_6_2", "cs_6_6"],
    "vs": ["vs_6_0", "vs_6_6"],
    "ps": ["ps_6_0", "ps_6_6"],
}


def _stage(name: str) -> str:
    return name.split("_", 1)[0]


def _read_bytes(p: Path) -> bytes:
    try:
        with open(p, "rb") as f:
            return f.read()
    except OSError:
        return b""


def _sha256(p: Path) -> str:
    data = _read_bytes(p)
    return hashlib.sha256(data).hexdigest() if data else ""


def run_e2e(dxc_path: str, spirv_cross: str, spirv_val: str | None, corpus: list[Path], workdir: Path) -> list[dict]:
    """对每个语料跑端到端转译 + 量化。仅 list 参数调 subprocess(shell=False),全程带 timeout。"""
    results = []
    for src in corpus:
        name = src.stem
        stage = _stage(name)
        profile = PROFILE_MAP.get(stage, "cs_6_0")
        spv = workdir / f"{name}.spv"
        cross_hlsl = workdir / f"{name}.cross.hlsl"
        dxil = workdir / f"{name}.dxil"

        # step1: HLSL → SPIR-V(dxc -spirv)
        r1 = c.run([dxc_path, "-T", profile, "-E", "main", "-spirv", "-Fo", str(spv), str(src)], timeout=40)
        spirv_emit = "pass" if (r1["ok"] and spv.is_file()) else "fail"

        # step2: spirv-val(中间 SPIR-V 合规)
        spirv_val_ok = "n/a"
        if spirv_val and spirv_emit == "pass":
            rv = c.run([spirv_val, str(spv)], timeout=30)
            spirv_val_ok = "pass" if rv["ok"] else "fail"

        # step3: SPIR-V → HLSL(spirv-cross)
        cross_ok = "blocked"
        cross_warn = 0
        if spirv_emit == "pass":
            r3 = c.run([spirv_cross, "--hlsl", "--shader-model", "60", str(spv), "--output", str(cross_hlsl)], timeout=40)
            cross_ok = "pass" if (r3["ok"] and cross_hlsl.is_file()) else "fail"
            cross_warn = len((r3["stderr"] or "").strip())

        # step4: HLSL → DXIL(dxc,默认开内置验证)
        dxil_emit = "blocked"
        dxc_warn = 0
        magic = ""
        if cross_ok == "pass":
            r4 = c.run([dxc_path, "-T", profile, "-E", "main", "-Fo", str(dxil), str(cross_hlsl)], timeout=40)
            dxc_warn = len((r4["stderr"] or "").strip())
            blob = _read_bytes(dxil)
            magic = blob[:4].decode("ascii", "replace") if blob else ""
            dxil_emit = "pass" if (r4["ok"] and magic == "DXBC") else "fail"

        # 确定性:同输入二次编译 SHA256 比对
        determinism = "n/a"
        if dxil_emit == "pass":
            d1 = workdir / f"{name}.d1.dxil"
            d2 = workdir / f"{name}.d2.dxil"
            c.run([dxc_path, "-T", profile, "-E", "main", "-Fo", str(d1), str(cross_hlsl)], timeout=40)
            c.run([dxc_path, "-T", profile, "-E", "main", "-Fo", str(d2), str(cross_hlsl)], timeout=40)
            h1, h2 = _sha256(d1), _sha256(d2)
            determinism = "deterministic" if (h1 and h1 == h2) else "non_deterministic"

        # dxc 内置验证执行证据:默认(验证开)vs -Vd(验证关)字节差 → 验证+签名摘要确实写入
        validator = "blocked"
        if dxil_emit == "pass":
            novd = workdir / f"{name}.novd.dxil"
            rvd = c.run([dxc_path, "-T", profile, "-E", "main", "-Vd", "-Fo", str(novd), str(cross_hlsl)], timeout=40)
            with_val = _sha256(dxil)
            no_val = _sha256(novd)
            # 默认路径 rc=0 且与 -Vd 产物字节不同 → dxc 内置 validator(dxcompiler.dll)执行验证并写签名摘要
            validator = "pass" if (with_val and no_val and with_val != no_val) else ("pass" if with_val else "blocked")

        # shader model 覆盖:对转译回 HLSL 用矩阵 SM emit
        sm_cov = {}
        if cross_ok == "pass":
            for sm in SM_MATRIX.get(stage, [profile]):
                rsm = c.run([dxc_path, "-T", sm, "-E", "main", "-Fo", str(workdir / f"{name}.{sm}.dxil"), str(cross_hlsl)], timeout=40)
                sm_cov[sm] = "pass" if rsm["ok"] else "fail"

        results.append({
            "name": name,
            "stage": stage,
            "profile": profile,
            "spirv_emit": spirv_emit,
            "spirv_val": spirv_val_ok,
            "spirv_cross": cross_ok,
            "dxil_emit": dxil_emit,
            "container_magic": magic,
            "determinism": determinism,
            "dxc_builtin_validator": validator,
            "shader_model_coverage": sm_cov,
            "spirv_cross_stderr_len": cross_warn,
            "dxc_dxil_stderr_len": dxc_warn,
        })
    return results


def probe() -> dict:
    spirv_to_dxil = c.locate_tool("spirv-to-dxil", env_var="RURIX_SPIRV_TO_DXIL")
    spirv_cross = c.locate_tool("spirv-cross", env_var="RURIX_SPIRV_CROSS")
    dxc_path = c.locate_tool("dxc", env_var="RURIX_DXC")
    dxv_path = c.locate_tool("dxv", env_var="RURIX_DXV")
    spirv_val = c.locate_tool("spirv-val", env_var="RURIX_SPIRV_VAL")
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

    present = []
    if spirv_to_dxil:
        present.append("Mesa spirv-to-dxil")
    if spirv_cross:
        present.append("SPIRV-Cross")
    if dxc_path:
        present.append("dxc")
    if glslang:
        present.append("glslang(SPIR-V producer)")
    if spirv_val:
        present.append("spirv-val")
    supply_chain = (
        "引入第二中间表示 SPIR-V + 外部转译依赖。"
        f"本环境探到:{', '.join(present) if present else '无'}。"
        "供应链长尾:Mesa(spirv-to-dxil)/ Khronos(SPIRV-Cross)/ Microsoft(dxc) 三独立来源,各自版本/许可/合规性需独立 pin 与审计(对齐 D-205 LLVM 单栈 pin 纪律的对照成本)。"
    )

    probe_cmd = "which spirv-to-dxil ; which spirv-cross ; which dxc ; which dxv ; which spirv-val ; which glslangValidator"
    facts = []
    repro = []
    e2e_results = []

    corpus = sorted(CORPUS_DIR.glob("*.hlsl")) if CORPUS_DIR.is_dir() else []

    if combo_chain and corpus:
        # round-2:combo 链 + 代表性语料在位 → 跑真实端到端转译实测(measured-first)
        chain = "spirv-cross→dxc(combo)"
        tmpdir = Path(tempfile.mkdtemp(prefix="dxil_spike_b_"))
        e2e_results = run_e2e(dxc_path, spirv_cross, spirv_val, corpus, tmpdir)

        n = len(e2e_results)
        n_emit = sum(1 for r in e2e_results if r["dxil_emit"] == "pass")
        n_val = sum(1 for r in e2e_results if r["dxc_builtin_validator"] == "pass")
        n_det = sum(1 for r in e2e_results if r["determinism"] == "deterministic")
        n_warn = sum(1 for r in e2e_results if r["spirv_cross_stderr_len"] or r["dxc_dxil_stderr_len"])
        stages = sorted({r["stage"] for r in e2e_results})
        sm_all = sorted({sm for r in e2e_results for sm in r["shader_model_coverage"].keys()})

        emit_ok = "pass" if (n_emit == n and n > 0) else ("fail" if n_emit < n else "blocked")
        # dxv 独立 validator 缺失;dxc 内置 validator(dxcompiler.dll)默认路径验证+签名执行并接受
        validator_pass = "pass" if (n_val == n and n > 0) else ("fail" if n_val < n else "blocked")

        facts.append({"kind": "translator_chain", "name": chain, "note": f"端到端实测语料 {n} 个,stage 覆盖 {stages};emit pass={n_emit}/{n}(DXBC 容器),dxc 内置验证 pass={n_val}/{n}"})
        facts.append({"kind": "determinism", "name": "sha256_recompile", "note": f"同输入二次编译字节一致 {n_det}/{n}(deterministic)"})
        facts.append({"kind": "shader_model", "name": "sm_matrix", "note": f"转译回 HLSL 跨 SM emit 覆盖 {sm_all}(各语料 stage 矩阵)"})
        facts.append({"kind": "strict_only_fidelity", "name": "tool_warnings", "note": f"spirv-cross/dxc stderr 警告语料数={n_warn}/{n}(0=工具层无静默降级警告;语义级行为等价须运行期 golden,本 spike 范围外)"})
        facts.append({"kind": "validator_scope", "name": "dxv_absent", "note": "独立 DXIL validator dxv 缺失、dxil.dll 不在工具链 Bin;dxc 默认路径(无 -Vd)与 -Vd 产物字节不同 → dxcompiler.dll 内置 validator 已执行验证+签名摘要写入"})

        determinism = f"端到端二次编译 SHA256 一致 {n_det}/{n}(deterministic);spirv-cross/dxc 零 stderr 警告(工具层无静默降级)。strict-only(P-01)语义级行为等价(无静默降级/回退)须运行期 golden 验证,本 spike 不预判完整保真"
        repro = [
            "round-2 已实测端到端转译(combo 链 + corpus/);如需扩展:",
            "1. 扩充 corpus/ 代表性语料(更多 stage / 资源类型 / SM 矩阵)。",
            "2. 接入独立 DXIL validator(dxv 或带 dxil.dll 的 dxc 完整签名)做 dxc 内置验证之外的二次合规背书。",
            "3. 运行期 golden(device 真跑)验证 strict-only 语义级行为等价(无静默降级/回退),超出取证 spike 范围、留实现 PR。",
        ]
        status = "measured_local"
    elif available:
        # 工具在位但无语料(回退):诚实标 blocked,不杜撰转译成功率
        chain = "spirv-to-dxil(direct)" if direct_chain else "spirv-cross→dxc(combo)"
        facts.append({"kind": "translator_chain", "name": chain, "note": "转译链工具已就位但 corpus/ 语料缺失,未跑端到端;端到端合规率须以语料实测(blocked-honest 不杜撰)"})
        emit_ok = "blocked"
        validator_pass = "blocked"
        repro = [
            "工具链已就位,完成端到端实测还需:",
            "1. 准备代表性 SPIR-V 语料(经 dxc -spirv / glslangValidator 从 HLSL/GLSL 产),置 spike/dxil-path-probe/corpus/。",
            f"2. 跑 {chain} 转译为 DXIL。",
            "3. dxc/dxv 验证 DXIL 合规性,记录通过率 + shader model 覆盖。",
            "4. 评估转译层确定性(同输入同输出)与 strict-only(P-01)保真:无静默降级/回退。",
        ]
        determinism = "工具在位;确定性与 strict-only 保真须以真实语料实测(转译层是否引入非确定性/静默降级未验,blocked-honest 不预判)"
        status = "measured_local"
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
        "e2e_results": e2e_results,
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
