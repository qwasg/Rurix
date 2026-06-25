# SPIKE(RD-014) — B 路 strict-only 达标取证:SPIR-V→DXIL 名保真能否零静默降级。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest:对每语料跑 ① dxc 直产基线 ② 默认 B 链(基线 TEXCOORD 降级)
# ③ 保名 B 链(dxc -fspv-reflect 携 UserSemantic + spirv-cross --set-hlsl-named-vertex-input-
# semantic,经 SPIR-V 反射自动导出),三方签名 part dump 对照,证用户语义名能否端到端存活。
# 不裁 P-01 规范线 / 不裁 A/B(硬规则 1 / P-13);名/elemcount/SHA256/status 全来自命令输出。
"""B 路 strict-only 名保真取证探针(承 probe_b_graphics_sig.py 默认参数三类损耗基线)。

链路对照(每语料):
  direct : HLSL → dxc(round-7)→ DXIL            = 作者意图上界
  b_def  : HLSL → dxc -spirv → spirv-cross(默认)→ dxc → DXIL  = 默认参数(TEXCOORD 降级基线)
  b_keep : HLSL → dxc -spirv -fspv-reflect → spirv-cross(--set-hlsl-named-vertex-input-
           semantic,经 spirv_reflect 自动导出)→ dxc → DXIL       = 名保名配置
决定性维度:ISG1/OSG1 签名 part dump(elemcount + 各元素 SV / 用户语义名);
保真对照:b_def vs direct(基线降级)、b_keep vs direct(保名后)。validator ×N + 确定性 ×N。
"""
from __future__ import annotations

import datetime
import json
import os
import sys
import tempfile
from pathlib import Path

import _common as c
import dxil_validator as dv
import spirv_reflect as sr
from probe_b_graphics_sig import _dump_sig, _fidelity, _sha256_file, _validate_n

HERE = Path(__file__).resolve().parent
CORPUS_DIR = HERE / "corpus"
N = int(os.environ.get("RURIX_SIG_N", "25"))
GRAPHICS = {"vs_sig": "vs_6_0", "ps_sig": "ps_6_0", "vs_passthrough": "vs_6_0", "ps_texture": "ps_6_0"}


def _b_chain(dxc_spirv, cross, sval, dxc_final, src, profile, wd, name, keep):
    """跑 B 全链(keep=True 则保名配置)。返回 (步骤态 dict, 最终 DXIL 路径 或 None)。"""
    suffix = "keep" if keep else "def"
    spv = wd / f"{name}.{suffix}.spv"
    chl = wd / f"{name}.{suffix}.hlsl"
    dxil = wd / f"{name}.{suffix}.dxil"
    steps = {"keep_names": keep}
    emit = [dxc_spirv, "-T", profile, "-E", "main", "-spirv"]
    if keep:
        emit.append("-fspv-reflect")  # 携 OpName + UserSemantic(原 HLSL 语义串)
    emit += ["-Fo", str(spv), str(src)]
    r1 = c.run(emit, timeout=40)
    steps["spirv_emit"] = "pass" if (r1["ok"] and spv.is_file()) else "fail"
    if steps["spirv_emit"] != "pass":
        return steps, None
    rv = c.run([sval, str(spv)], timeout=30) if sval else {"ok": True}
    steps["spirv_val"] = "pass" if rv["ok"] else "fail"
    cross_args = [cross, "--hlsl", "--shader-model", "60", str(spv), "--output", str(chl)]
    if keep:
        refl = sr.parse_spirv(spv.read_bytes())
        flags = sr.vertex_input_semantic_flags(refl)
        steps["exec_model"] = refl.get("exec_model")
        steps["keep_flags"] = flags  # 经 SPIR-V 反射自动导出(非硬编码)
        cross_args += flags
    r3 = c.run(cross_args, timeout=40)
    steps["spirv_cross"] = "pass" if (r3["ok"] and chl.is_file()) else "fail"
    steps["spirv_cross_stderr_len"] = len((r3["stderr"] or "").strip())
    if steps["spirv_cross"] != "pass":
        return steps, None
    r4 = c.run([dxc_final, "-T", profile, "-E", "main", "-Fo", str(dxil), str(chl)], timeout=40)
    blob = dxil.read_bytes() if dxil.is_file() else b""
    steps["dxil_emit"] = "pass" if (r4["ok"] and blob[:4] == b"DXBC") else "fail"
    steps["dxc_stderr_len"] = len((r4["stderr"] or "").strip())
    return steps, (dxil if steps["dxil_emit"] == "pass" else None)


def _determinism(dxc_spirv, cross, sval, dxc_final, src, profile, wd, name, keep, n):
    """同输入 ×n 跑保名 B 全链,记最终 DXIL 容器 SHA256 一致性。"""
    hashes = []
    for i in range(n):
        sub = wd / f"det_{('keep' if keep else 'def')}_{i}"
        sub.mkdir(exist_ok=True)
        _, dxil = _b_chain(dxc_spirv, cross, sval, dxc_final, src, profile, sub, name, keep)
        hashes.append(_sha256_file(dxil) if dxil else "")
    uniq = sorted(set(h for h in hashes if h))
    return {"n": n, "unique_sha256": uniq, "consistent": len(uniq) == 1 and "" not in hashes,
            "sha256": uniq[0] if len(uniq) == 1 else ""}


def probe() -> dict:
    val_dir = os.environ.get("RURIX_DXC_NEW_DIR", r"H:\dxc-round7\extracted\bin\x64")
    dxc_final = str(Path(val_dir) / "dxc.exe")
    dxcompiler_dll = str(Path(val_dir) / "dxcompiler.dll")
    dxv = str(Path(val_dir) / "dxv.exe")
    dxc_spirv = c.locate_tool("dxc", env_var="RURIX_DXC_SPIRV")
    cross = c.locate_tool("spirv-cross", env_var="RURIX_SPIRV_CROSS")
    sval = c.locate_tool("spirv-val", env_var="RURIX_SPIRV_VAL")
    tools_ok = all(Path(p).is_file() for p in (dxc_final, dxcompiler_dll)) and dxc_spirv and cross
    versions = {
        "dxc_spirv_producer": c.tool_version(dxc_spirv, ["--version"]),
        "spirv_cross": c.tool_version(cross, ["--version"]) or "(no --version)",
        "spirv_val": c.tool_version(sval, ["--version"]),
        "dxc_validator": c.tool_version(dxc_final, ["--version"]),
    }
    out = {"schema_version": 1, "subject": "dxil_b_strict_only",
           "rd_ref": "RD-014", "decision_ref": "RFC-0004 §4.4 / 04 P-01 / P-13",
           "host_env": {"os": sys.platform}, "tool_versions": versions,
           "validator": {"dir": val_dir, "n": N, "dxv_present": Path(dxv).is_file()},
           "keep_mechanism": "dxc -spirv -fspv-reflect(携 UserSemantic)+ spirv-cross "
                             "--set-hlsl-named-vertex-input-semantic(经 spirv_reflect 自动导出);"
                             "varying(vs-out/ps-in)语义 spirv-cross 硬绑 TEXCOORD#,无输出语义保名 flag。",
           "owner_decision": "P-01 strict-only 规范线 + ②③契约线归属裁断权属 owner(硬规则 1/P-13);"
                             "本探针只产名保真机器事实,不代决、不代签。",
           "samples": []}
    if not tools_ok:
        out["status"] = "blocked"
        out["notes"] = "B 链或 round-7 validator 工具缺失;repro 见报告。"
        return out
    wd = Path(tempfile.mkdtemp(prefix="dxil_b_strict_"))
    dxv_use = dxv if Path(dxv).is_file() else None
    _run_samples(out, wd, dxc_spirv, cross, sval, dxc_final, dxcompiler_dll, dxv_use)
    out["status"] = "measured_local"
    return out


def _run_samples(out, wd, dxc_spirv, cross, sval, dxc_final, dxcompiler_dll, dxv_use):
    n = out["validator"]["n"]
    for name, profile in GRAPHICS.items():
        src = CORPUS_DIR / f"{name}.hlsl"
        if not src.is_file():
            out["samples"].append({"name": name, "status": "blocked", "reason": "corpus_missing"})
            continue
        ddxil = wd / f"{name}.direct.dxil"
        rd = c.run([dxc_final, "-T", profile, "-E", "main", "-Fo", str(ddxil), str(src)], timeout=40)
        direct_emit = "pass" if (rd["ok"] and ddxil.is_file() and ddxil.read_bytes()[:4] == b"DXBC") else "fail"
        steps_def, bdef = _b_chain(dxc_spirv, cross, sval, dxc_final, src, profile, wd, name, False)
        steps_keep, bkeep = _b_chain(dxc_spirv, cross, sval, dxc_final, src, profile, wd, name, True)
        sig_direct = _dump_sig(ddxil) if direct_emit == "pass" else {"ok": False}
        sig_def = _dump_sig(bdef) if bdef else {"ok": False}
        sig_keep = _dump_sig(bkeep) if bkeep else {"ok": False}
        sample = {
            "name": name, "stage": name.split("_")[0][:2], "profile": profile,
            "b_chain_default": steps_def, "b_chain_keep": steps_keep,
            "direct_baseline": {"dxil_emit": direct_emit},
            "signature": {"direct": sig_direct, "b_default": sig_def, "b_keep": sig_keep},
            "fidelity_default": _fidelity(sig_direct, sig_def) if (sig_direct.get("ok") and sig_def.get("ok")) else {},
            "fidelity_keep": _fidelity(sig_direct, sig_keep) if (sig_direct.get("ok") and sig_keep.get("ok")) else {},
            "validation": {
                "direct": _validate_n(dxcompiler_dll, dxv_use, ddxil, n) if direct_emit == "pass" else {"idxc": "blocked"},
                "b_keep": _validate_n(dxcompiler_dll, dxv_use, bkeep, n) if bkeep else {"idxc": "blocked"},
            },
            "determinism_keep": _determinism(dxc_spirv, cross, sval, dxc_final, src, profile, wd, name, True, n) if bkeep else {"consistent": False},
        }
        out["samples"].append(sample)


if __name__ == "__main__":
    res = probe()
    res["timestamp"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    if os.environ.get("RURIX_EMIT_SIG"):
        date = os.environ.get("RURIX_SIG_DATE", datetime.date.today().strftime("%Y%m%d"))
        dest = HERE.parent.parent / "evidence" / f"dxil_b_strict_only_{date}.json"
        blob = json.dumps(res, ensure_ascii=False, indent=2).encode("utf-8")
        with open(dest, "wb") as f:
            f.write(blob.replace(b"\r\n", b"\n") + b"\n")
        print(f"wrote {dest}")
    else:
        print(json.dumps(res, ensure_ascii=False, indent=2))
    sys.exit(0)
