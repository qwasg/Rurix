#!/usr/bin/env py -3
# SPIKE(G2.3) — 绑定布局推导可行性 spike 探针。不入 src/ 生产路径、spike 结束可弃。
#
# 目的：在现有图形=B 链（MIR→SPIR-V→spirv-cross→HLSL→dxc→DXIL，RFC-0004）上，
# 实测从 shader 资源使用（RXS-0156 Texture2D<F>/Sampler 句柄 + cbuffer/structured
# buffer）到 D3D12 root signature 的可推导路径：
#   SPIR-V DescriptorSet/Binding 装饰 → spirv-cross HLSL register(t#, space#)
#   → dxc 容器（RTS0 root signature part / PSV0 资源绑定）各产出什么、确定性如何、
#     register/space 分配是否可由编译器按声明（io_sig）顺序确定性导出。
#
# 严格诚实纪律（对齐 RD-010 spike 诚实纪律）：
#   - measured 与 assumed 严格分栏：本探针只 measure 「工具链给定输入产出什么」；
#     「Rurix 自有 MIR→SPIR-V 能否按 io_sig 顺序 emit 资源绑定装饰」当前结构上不可达
#     （io_sig/MirIoType 无资源种类，见 dxil_spirv.rs），属 assumed 不实测。
#   - 工具缺失为 SKIP，不伪造。
#   - 不声称未验证的推导路径「已打通」。
#
# 用法：
#   set RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64   (signed pin，HLSL→DXIL + dumpbin)
#   set RURIX_SPIRV_CROSS=...\spirv-cross.exe
#   set RURIX_SPIRV_DXC=...\vulkan-sdk\Bin\dxc.exe       (HLSL→SPIR-V producer)
#   set RURIX_SPIRV_DIS=...\spirv-dis.exe
#   py -3 spike/g2.3-binding-layout/probe_binding_layout.py
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
CORPUS = HERE / "corpus"
OUT_DIR = HERE / "_out"
EVIDENCE_DIR = ROOT / "evidence" / "g2.3-binding-layout"

# ── 工具定位（env override；缺失 → SKIP，不伪造）───────────────────────────────
def find_tool(env_key, *candidates):
    v = os.environ.get(env_key)
    if v and Path(v).exists():
        return Path(v)
    for c in candidates:
        p = Path(c)
        if p.exists():
            return p
    w = shutil.which(Path(candidates[0]).name if candidates else env_key)
    return Path(w) if w else None

DXC_DIR = os.environ.get("RURIX_DXC_DIR", r"H:\dxc-round7\extracted\bin\x64")
DXC_PIN = find_tool("RURIX_DXC_PIN", str(Path(DXC_DIR) / "dxc.exe"))
SPIRV_CROSS = find_tool(
    "RURIX_SPIRV_CROSS",
    r"C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-cross.exe",
)
SPIRV_DXC = find_tool(
    "RURIX_SPIRV_DXC",
    r"C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\dxc.exe",
)
SPIRV_DIS = find_tool(
    "RURIX_SPIRV_DIS",
    r"C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-dis.exe",
)


def run(cmd, **kw):
    p = subprocess.run(
        [str(c) for c in cmd],
        capture_output=True,
        text=True,
        timeout=120,
        **kw,
    )
    return p.returncode, p.stdout, p.stderr


def sha256_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def tool_version(tool):
    if tool is None:
        return None
    _, out, err = run([tool, "--version"])
    return (out + err).strip().splitlines()[0] if (out + err).strip() else "unknown"


# 语料 → (dxc profile, spirv-cross stage)
CORPUS_SPEC = {
    "ps_textured": ("ps_6_0", "frag"),
    "ps_mixed": ("ps_6_0", "frag"),
    "cs_structured": ("cs_6_0", "comp"),
    "ps_rootsig": ("ps_6_0", "frag"),
}


def parse_spirv_bindings(disasm):
    """从 spirv-dis 文本抽取 (var, DescriptorSet, Binding) 装饰。"""
    sets, binds, names = {}, {}, {}
    for line in disasm.splitlines():
        m = re.search(r"OpDecorate (%\w+) DescriptorSet (\d+)", line)
        if m:
            sets[m.group(1)] = int(m.group(2))
        m = re.search(r"OpDecorate (%\w+) Binding (\d+)", line)
        if m:
            binds[m.group(1)] = int(m.group(2))
        m = re.search(r"OpName (%\w+) \"(\w+)\"", line)
        if m:
            names[m.group(1)] = m.group(2)
    out = []
    for var in sorted(set(sets) | set(binds)):
        out.append(
            {
                "var": var,
                "name": names.get(var, ""),
                "set": sets.get(var),
                "binding": binds.get(var),
            }
        )
    return out


def parse_hlsl_registers(hlsl):
    """从 spirv-cross 产出的 HLSL 抽取 `: register(x#, spaceN)` 分配。"""
    out = []
    for m in re.finditer(
        r"(\w+)\s*:\s*register\((\w)(\d+)(?:,\s*space(\d+))?\)", hlsl
    ):
        out.append(
            {
                "name": m.group(1),
                "class": m.group(2),  # t / s / b / u
                "index": int(m.group(3)),
                "space": int(m.group(4)) if m.group(4) else 0,
            }
        )
    return out


CONTAINER_FOURCC = ["DXIL", "PSV0", "RTS0", "RDAT", "ISG1", "OSG1", "ISG2",
                    "OSG2", "PSG1", "STAT", "SFI0", "HASH", "ILDN"]


def container_parts(dxil_path):
    """二进制扫描 DXIL 容器 part 四字符码（RTS0 = root signature；PSV0 = 资源绑定反射）。
    dxc -dumpbin 产反汇编文本而非 part 表，故直接扫容器字节里的 fourcc（measured 准确）。"""
    raw = Path(dxil_path).read_bytes()
    blob = raw.decode("latin-1")
    present = [fc for fc in CONTAINER_FOURCC if fc in blob]
    # dumpbin 反汇编摘要（含 PSVRuntimeInfo 资源绑定可读形态）留作人读佐证。
    rc, out, _ = run([DXC_PIN, "-dumpbin", str(dxil_path)])
    return {
        "container_bytes": len(raw),
        "parts_present": present,
        "has_root_signature_RTS0": "RTS0" in present,
        "has_psv0_reflection": "PSV0" in present,
        "dumpbin_rc": rc,
        "dumpbin_excerpt": "\n".join(out.splitlines()[:30]),
    }


def probe_one(name, profile, stage):
    src = CORPUS / f"{name}.hlsl"
    rec = {"name": name, "profile": profile, "stage": stage, "steps": {}}

    # (1) HLSL → SPIR-V（dxc -spirv，Vulkan producer）。
    spv = OUT_DIR / f"{name}.spv"
    rc, out, err = run(
        [SPIRV_DXC, "-spirv", "-T", profile, "-E", "main",
         "-fspv-target-env=vulkan1.1", str(src), "-Fo", str(spv)]
    )
    rec["steps"]["hlsl_to_spirv"] = {"rc": rc, "ok": rc == 0 and spv.exists(),
                                     "stderr": err.strip()[:500]}
    if rc != 0 or not spv.exists():
        return rec

    # (2) spirv-dis → DescriptorSet/Binding 装饰。
    rc, dis, err = run([SPIRV_DIS, str(spv)])
    rec["steps"]["spirv_dis"] = {"rc": rc}
    rec["spirv_bindings"] = parse_spirv_bindings(dis) if rc == 0 else []
    (OUT_DIR / f"{name}.spvasm").write_bytes(dis.replace("\r\n", "\n").encode("utf-8"))

    # (3) spirv-cross → HLSL（默认：从 SPIR-V binding 派生 register）。
    hlsl_out = OUT_DIR / f"{name}.cross.hlsl"
    sc_stage = {"frag": "frag", "comp": "comp", "vert": "vert"}[stage]
    rc, out, err = run(
        [SPIRV_CROSS, "--hlsl", "--shader-model", "60", "--stage", sc_stage,
         str(spv), "--output", str(hlsl_out)]
    )
    rec["steps"]["spirv_cross"] = {"rc": rc, "ok": rc == 0 and hlsl_out.exists(),
                                   "stderr": err.strip()[:500]}
    if rc == 0 and hlsl_out.exists():
        cross_hlsl = hlsl_out.read_text(encoding="utf-8")
        rec["hlsl_registers"] = parse_hlsl_registers(cross_hlsl)
        rec["spirv_cross_sha256"] = sha256_file(hlsl_out)

    # (4) dxc（signed pin）HLSL→DXIL。对 ps_rootsig 用原始源（含 [RootSignature]）；
    #     其余用 spirv-cross 产出的 HLSL（贴合 B 链：dxc 编译的是 spirv-cross 输出）。
    compile_src = src if name == "ps_rootsig" else hlsl_out
    dxil = OUT_DIR / f"{name}.dxil"
    rc, out, err = run(
        [DXC_PIN, "-T", profile, "-E", "main", str(compile_src), "-Fo", str(dxil)]
    )
    rec["steps"]["dxc_to_dxil"] = {"rc": rc, "ok": rc == 0 and dxil.exists(),
                                   "compile_src": compile_src.name,
                                   "stderr": err.strip()[:500]}
    if rc == 0 and dxil.exists():
        rec["dxil_sha256"] = sha256_file(dxil)
        rec["container"] = container_parts(dxil)

        # (5) 确定性：再编译一次，比对容器 SHA256。
        dxil2 = OUT_DIR / f"{name}.2.dxil"
        rc2, _, _ = run(
            [DXC_PIN, "-T", profile, "-E", "main", str(compile_src), "-Fo", str(dxil2)]
        )
        if rc2 == 0 and dxil2.exists():
            rec["deterministic"] = sha256_file(dxil) == sha256_file(dxil2)

    return rec


def main():
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)

    tools = {
        "dxc_pin": str(DXC_PIN) if DXC_PIN else None,
        "dxc_pin_version": tool_version(DXC_PIN),
        "spirv_cross": str(SPIRV_CROSS) if SPIRV_CROSS else None,
        "spirv_cross_version": tool_version(SPIRV_CROSS),
        "spirv_dxc": str(SPIRV_DXC) if SPIRV_DXC else None,
        "spirv_dxc_version": tool_version(SPIRV_DXC),
        "spirv_dis": str(SPIRV_DIS) if SPIRV_DIS else None,
    }
    missing = [k for k in ("dxc_pin", "spirv_cross", "spirv_dxc", "spirv_dis")
               if tools[k] is None]

    result = {
        "schema": "g2.3-binding-layout-spike/v1",
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "host_note": "spike 隔离取证；measured=工具链产出，assumed=Rurix io_sig 资源绑定 emit（当前结构上不可达，不实测）",
        "tools": tools,
        "status": "SKIP" if missing else "MEASURED",
        "missing_tools": missing,
        "corpus": [],
    }

    if missing:
        print(f"[SKIP] 缺工具：{missing}（不伪造，evidence status=SKIP）")
    else:
        for name, (profile, stage) in CORPUS_SPEC.items():
            print(f"[probe] {name} ({profile})")
            result["corpus"].append(probe_one(name, profile, stage))

    stamp = datetime.now(timezone.utc).strftime("%Y%m%d")
    out_json = EVIDENCE_DIR / f"binding_layout_spike_{stamp}.json"
    # LF byte-exact：不用 Python 文本模式(Windows 会把 \n 转 \r\n)，显式写 LF 字节。
    payload = json.dumps(result, indent=2, ensure_ascii=False) + "\n"
    out_json.write_bytes(payload.encode("utf-8"))
    print(f"[evidence] {out_json}")
    return 0 if not missing else 0  # SKIP 不算失败


if __name__ == "__main__":
    sys.exit(main())
