# SPIKE(RD-010) — B 路图形签名能力取证:SPIR-V→DXIL 能否产带真实 SV 签名的图形 DXIL。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest:工具链探到则跑端到端 + 签名 part dump + validator ×N,
# 探不到如实 blocked + repro,绝不杜撰。对照 round-8 A 路 ISG1/OSG1 elemcount=0 口径。
"""B 路图形签名取证探针。

链路:HLSL → dxc -spirv → spirv-val → spirv-cross → HLSL → dxc → DXIL(B 全链);
另跑 HLSL → dxc 直产作上界基线(dxc 原生图形签名能力)。
决定性维度:解 DXContainer 的 ISG1/OSG1 签名 part,记 elemcount + 各元素 SV 语义名,
与 A 路 elemcount=0 苹果对苹果对照,证 SV_Position/SV_Target/SV_VertexID/插值 varying
是否端到端存活进 B 产物签名;保真子轴对照入口意图 vs 出口签名(语义名/数量/系统值)。
validator:IDxcValidator(round-7 dxcompiler.dll + dxil.dll,签名 validator)+ dxv.exe 各 ×N。
"""
from __future__ import annotations

import hashlib
import json
import os
import sys
import tempfile
from pathlib import Path

import _common as c
import dxil_container as dc
import dxil_validator as dv

HERE = Path(__file__).resolve().parent
CORPUS_DIR = HERE / "corpus"
N = int(os.environ.get("RURIX_SIG_N", "25"))
# 图形语料(承 RXS-0154/0159 SV 映射):富签名 vs_sig/ps_sig + round-2 corpus 的图形样例。
GRAPHICS = {"vs_sig": "vs_6_0", "ps_sig": "ps_6_0", "vs_passthrough": "vs_6_0", "ps_texture": "ps_6_0"}


def _sha256_file(p: Path) -> str:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return ""


def _dump_sig(path: Path) -> dict:
    """dump ISG1/OSG1 签名 part → {ISG1:{...}, OSG1:{...}}(对照 A elemcount=0)。"""
    try:
        b = path.read_bytes()
    except OSError:
        return {"ok": False, "reason": "read_failed"}
    out = {"ok": True, "is_signed": dc.parse_dxbc(b).get("is_signed")}
    for fc in ("ISG1", "OSG1"):
        s = dc.parse_signature_part(b, fc)
        if s.get("ok"):
            out[fc] = {
                "elemcount": s["elemcount"],
                "elements": [
                    {"name": f"{e['semantic_name']}{e['semantic_index']}",
                     "system_value": e["system_value"], "register": e["register"],
                     "mask": e["mask"], "comp_type": e["comp_type"]}
                    for e in s["elements"]
                ],
            }
        else:
            out[fc] = {"elemcount": -1, "reason": s.get("reason")}
    return out


def _validate_n(dxcompiler_dll: str, dxv: str | None, container: Path, n: int) -> dict:
    """IDxcValidator ×n(round-7 dxcompiler.dll+dxil.dll)+ dxv.exe ×n。记 status 直方图。"""
    data = container.read_bytes() if container.is_file() else b""
    idxc_hist: dict[str, int] = {}
    idxc_accept = 0
    for _ in range(n):
        r = dv.validate_container(dxcompiler_dll, data)
        if r.get("status") != "measured":
            return {"idxc": "blocked", "reason": r.get("reason")}
        st = r["validation_status_hr"]
        idxc_hist[st] = idxc_hist.get(st, 0) + 1
        if r["accepted"]:
            idxc_accept += 1
    out = {"idxc_accept": f"{idxc_accept}/{n}", "idxc_status_hist": idxc_hist}
    if dxv:
        dxv_accept = 0
        last = ""
        for _ in range(n):
            rv = c.run([dxv, str(container)], timeout=30)
            txt = (rv["stdout"] + rv["stderr"])
            if "Validation succeeded" in txt:
                dxv_accept += 1
            else:
                last = txt.strip().splitlines()[-1] if txt.strip() else f"rc={rv['rc']}"
        out["dxv_accept"] = f"{dxv_accept}/{n}"
        out["dxv_last_reject"] = last
    else:
        out["dxv_accept"] = "blocked"
    return out


def _fidelity(direct: dict, b: dict) -> dict:
    """入口意图(direct 基线签名)vs 出口(B 链签名)对照:静默丢/改/降级。"""
    out = {}
    for fc in ("ISG1", "OSG1"):
        d = direct.get(fc, {}) if direct.get("ok") else {}
        bb = b.get(fc, {}) if b.get("ok") else {}
        dn = [e["name"] for e in d.get("elements", [])]
        bn = [e["name"] for e in bb.get("elements", [])]
        # 系统值集合(SV_* 是否保留)
        dsv = sorted({e["system_value"] for e in d.get("elements", []) if e["system_value"].startswith("SV_")})
        bsv = sorted({e["system_value"] for e in bb.get("elements", []) if e["system_value"].startswith("SV_")})
        out[fc] = {
            "elemcount_direct": d.get("elemcount", -1),
            "elemcount_b": bb.get("elemcount", -1),
            "elemcount_match": d.get("elemcount") == bb.get("elemcount"),
            "sv_direct": dsv,
            "sv_b": bsv,
            "sv_preserved": dsv == bsv,
            "sv_dropped": [s for s in dsv if s not in bsv],
            "user_names_direct": dn,
            "user_names_b": bn,
            "user_names_preserved": dn == bn,
        }
    return out


def _b_chain(dxc_spirv, cross, sval, dxc_final, src, profile, wd, name):
    """跑 B 全链,返回 (步骤态 dict, 最终 DXIL 路径 或 None)。"""
    spv = wd / f"{name}.spv"
    chl = wd / f"{name}.cross.hlsl"
    dxil = wd / f"{name}.b.dxil"
    steps = {}
    r1 = c.run([dxc_spirv, "-T", profile, "-E", "main", "-spirv", "-Fo", str(spv), str(src)], timeout=40)
    steps["spirv_emit"] = "pass" if (r1["ok"] and spv.is_file()) else "fail"
    if steps["spirv_emit"] != "pass":
        return steps, None
    rv = c.run([sval, str(spv)], timeout=30) if sval else {"ok": True}
    steps["spirv_val"] = "pass" if rv["ok"] else "fail"
    r3 = c.run([cross, "--hlsl", "--shader-model", "60", str(spv), "--output", str(chl)], timeout=40)
    steps["spirv_cross"] = "pass" if (r3["ok"] and chl.is_file()) else "fail"
    steps["spirv_cross_stderr_len"] = len((r3["stderr"] or "").strip())
    if steps["spirv_cross"] != "pass":
        return steps, None
    r4 = c.run([dxc_final, "-T", profile, "-E", "main", "-Fo", str(dxil), str(chl)], timeout=40)
    blob = dxil.read_bytes() if dxil.is_file() else b""
    steps["dxil_emit"] = "pass" if (r4["ok"] and blob[:4] == b"DXBC") else "fail"
    steps["dxc_stderr_len"] = len((r4["stderr"] or "").strip())
    return steps, (dxil if steps["dxil_emit"] == "pass" else None)


def _determinism(dxc_spirv, cross, sval, dxc_final, src, profile, wd, name, n):
    """同输入 ×n 跑 B 全链,记最终 DXIL 容器 SHA256 一致性。"""
    hashes = []
    for i in range(n):
        sub = wd / f"det{i}"
        sub.mkdir(exist_ok=True)
        _, dxil = _b_chain(dxc_spirv, cross, sval, dxc_final, src, profile, sub, name)
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
    out = {"schema_version": 1, "subject": "dxil_b_graphics_sig",
           "rd_ref": "RD-010", "decision_ref": "RFC-0003 §9 Q-D131 / 13 §D-131",
           "host_env": {"os": sys.platform}, "tool_versions": versions,
           "validator": {"dir": val_dir, "n": N, "dxv_present": Path(dxv).is_file()},
           "owner_decision": "A/B/混合架构裁决权属 owner(硬规则 1);本探针只产签名取证事实,不代决、不代签。",
           "samples": []}

    if not tools_ok:
        out["status"] = "blocked"
        out["notes"] = "B 链或 round-7 validator 工具缺失;repro 见报告。"
        return out

    wd = Path(tempfile.mkdtemp(prefix="dxil_b_sig_"))
    dxv_use = dxv if Path(dxv).is_file() else None
    # wrapper 正反向控制(round-7 范式):自产 accept / 翻字节 reject
    ctrl_src = CORPUS_DIR / "vs_sig.hlsl"
    cdxil = wd / "ctrl.dxil"
    c.run([dxc_final, "-T", "vs_6_0", "-E", "main", "-Fo", str(cdxil), str(ctrl_src)], timeout=40)
    pos = dv.validate_container(dxcompiler_dll, cdxil.read_bytes())
    bad = bytearray(cdxil.read_bytes())  # 翻 bitcode 尾部 64 字节(可靠破坏验证)
    for _i in range(max(0, len(bad) - 64), len(bad)):
        bad[_i] ^= 0xFF
    neg = dv.validate_container(dxcompiler_dll, bytes(bad))
    out["wrapper_control"] = {
        "positive_accept": pos.get("accepted"), "positive_status": pos.get("validation_status_hr"),
        "negative_accept": neg.get("accepted"), "negative_status": neg.get("validation_status_hr"),
        "validated": bool(pos.get("accepted")) and not neg.get("accepted"),
    }
    _run_samples(out, wd, dxc_spirv, cross, sval, dxc_final, dxcompiler_dll, dxv_use)
    out["status"] = "measured_local"
    return out


def _run_samples(out, wd, dxc_spirv, cross, sval, dxc_final, dxcompiler_dll, dxv_use):
    for name, profile in GRAPHICS.items():
        src = CORPUS_DIR / f"{name}.hlsl"
        if not src.is_file():
            out["samples"].append({"name": name, "status": "blocked", "reason": "corpus_missing"})
            continue
        # 直产基线(dxc 原生图形签名上界)
        ddxil = wd / f"{name}.direct.dxil"
        rd = c.run([dxc_final, "-T", profile, "-E", "main", "-Fo", str(ddxil), str(src)], timeout=40)
        direct_emit = "pass" if (rd["ok"] and ddxil.is_file() and ddxil.read_bytes()[:4] == b"DXBC") else "fail"
        # B 全链
        steps, bdxil = _b_chain(dxc_spirv, cross, sval, dxc_final, src, profile, wd, name)
        sig_direct = _dump_sig(ddxil) if direct_emit == "pass" else {"ok": False}
        sig_b = _dump_sig(bdxil) if bdxil else {"ok": False}
        sample = {
            "name": name, "stage": name.split("_")[0][:2], "profile": profile,
            "b_chain": steps, "direct_baseline": {"dxil_emit": direct_emit},
            "signature": {"direct": sig_direct, "b": sig_b},
            "fidelity": _fidelity(sig_direct, sig_b) if (sig_direct.get("ok") and sig_b.get("ok")) else {},
            "validation": {
                "direct": _validate_n(dxcompiler_dll, dxv_use, ddxil, out["validator"]["n"]) if direct_emit == "pass" else {"idxc": "blocked"},
                "b": _validate_n(dxcompiler_dll, dxv_use, bdxil, out["validator"]["n"]) if bdxil else {"idxc": "blocked"},
            },
            "determinism": _determinism(dxc_spirv, cross, sval, dxc_final, src, profile, wd, name, out["validator"]["n"]) if bdxil else {"consistent": False},
        }
        out["samples"].append(sample)


if __name__ == "__main__":
    import datetime
    res = probe()
    res["timestamp"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    if os.environ.get("RURIX_EMIT_SIG"):
        date = os.environ.get("RURIX_SIG_DATE", datetime.date.today().strftime("%Y%m%d"))
        dest = HERE.parent.parent / "evidence" / f"dxil_b_graphics_sig_{date}.json"
        blob = json.dumps(res, ensure_ascii=False, indent=2).encode("utf-8")
        with open(dest, "wb") as f:
            f.write(blob.replace(b"\r\n", b"\n") + b"\n")
        print(f"wrote {dest}")
    else:
        print(json.dumps(res, ensure_ascii=False, indent=2))
    sys.exit(0)
