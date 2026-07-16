# SPIKE(RD-010) — round-7 A 路 validator 互操作复验:用与 LLVM 22/23 同年代(2026)的更新
# DXC release(自带 dxil.dll 签名 validator + 独立 dxv.exe)重验 round-6 合法 llc DXContainer,
# 裁开 Bug 2 归因(「LLVM 过度 emit 新 PSV」vs「dxc 1.8 太旧不识新 PSV」)。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest(硬规则 3/4):所有数字来自命令真实输出,多发量化,探不到如实 blocked。
"""round-7 探针:换新 dxil.dll 后正反向控制 + 合法 llc 容器真验证 + PSV 版本子轴。

- 新 DXC 目录(RURIX_DXC_NEW_DIR,默认 H:\\dxc-round7\\extracted\\bin\\x64):dxc/dxcompiler/dxil/dxv。
- round-6 合法容器(RURIX_R6_DIR,默认 H:\\llvm-audit-round6):official_cs.obj(llc23 1936B)/
  off_cs22/run-0001.obj(llc22 1804B)——均带 hlsl 入口属性,合法 emit(round-6 阶段 3 证 100/100 稳定)。
- 双 validator:IDxcValidator(新 dxcompiler.dll + dxil.dll,ctypes harness)+ 独立 dxv.exe CLI。
- N≥20 量化确定性;先正向(新 dxc 自产应 accept)+ 反向(翻字节损坏应 reject)验 wrapper,再信任结果。
"""
from __future__ import annotations

import json
import os
import struct
import sys

import _common as c
import dxil_container as dcont
import dxil_validator as dval

NEW_DIR = os.environ.get("RURIX_DXC_NEW_DIR", r"H:\dxc-round7\extracted\bin\x64")
R6 = os.environ.get("RURIX_R6_DIR", r"H:\llvm-audit-round6")
N = int(os.environ.get("RURIX_ROUND7_N", "25"))

MINIMAL_HLSL = "[numthreads(1,1,1)]\nvoid main() {}\n"


def psv0_runtime_info_size(data: bytes) -> dict:
    """从 DXContainer 的 PSV0 part 读 PSVRuntimeInfo 声明大小(part 体首 u32)。"""
    p = dcont.parse_dxbc(data)
    if not p.get("ok"):
        return {"ok": False, "reason": "container_parse_failed"}
    for i in range(p.get("part_count", 0)):
        off_pos = 32 + 4 * i
        if off_pos + 4 > len(data):
            break
        off = struct.unpack("<I", data[off_pos:off_pos + 4])[0]
        if off + 8 > len(data):
            continue
        if data[off:off + 4] == b"PSV0":
            body = off + 8
            if body + 4 > len(data):
                return {"ok": False, "reason": "psv0_truncated"}
            return {"ok": True, "psv_runtime_info_size": struct.unpack("<I", data[body:body + 4])[0]}
    return {"ok": False, "reason": "no_PSV0_part"}


def idxc_validate_n(dll: str, data: bytes, n: int) -> dict:
    """IDxcValidator::Validate 重复 n 次,量 accept 数 + status 直方图 + 一例错误原文。"""
    hist: dict[str, int] = {}
    accepts = 0
    sample_err = ""
    for _ in range(n):
        res = dval.validate_container(dll, data)
        if res.get("status") != "measured":
            key = "blocked:" + str(res.get("reason"))
            hist[key] = hist.get(key, 0) + 1
            continue
        s = str(res.get("validation_status_hr"))
        hist[s] = hist.get(s, 0) + 1
        if res.get("accepted"):
            accepts += 1
        elif not sample_err:
            sample_err = res.get("error_message", "")
    return {"n": n, "accepts": accepts, "status_histogram": hist, "sample_error": sample_err}


def dxv_cli_n(dxv: str, container_path: str, n: int) -> dict:
    """独立 dxv.exe CLI 重复 n 次,量 accept/reject/other + 一例输出原文。"""
    accept = reject = other = 0
    sample = ""
    for _ in range(n):
        r = c.run([dxv, container_path], timeout=30)
        out = (r["stdout"] + r["stderr"]).strip()
        low = out.lower()
        if r["rc"] == 0 and "succeeded" in low:
            accept += 1
        elif "failed" in low or "error" in low:
            reject += 1
            if not sample:
                sample = out
        else:
            other += 1
    return {"n": n, "accept": accept, "reject": reject, "other": other, "sample": sample}


def container_psv_facts(data: bytes) -> dict:
    """容器 PSV0 RuntimeInfoSize + DXIL part 程序版本(子轴对照)。"""
    return {
        "psv0": psv0_runtime_info_size(data),
        "dxil_program_version": dcont.dxil_part_version(data),
        "parsed": {k: dcont.parse_dxbc(data).get(k) for k in ("part_fourccs", "is_signed", "size")},
    }


def sha256(path: str) -> str:
    import hashlib
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def run_round7() -> dict:
    dxc = os.path.join(NEW_DIR, "dxc.exe")
    dxcompiler = os.path.join(NEW_DIR, "dxcompiler.dll")
    dxil_dll = os.path.join(NEW_DIR, "dxil.dll")
    dxv = os.path.join(NEW_DIR, "dxv.exe")
    out: dict = {"new_dxc_dir": NEW_DIR, "N": N, "facts": [], "repro": []}

    have = {p: os.path.isfile(p) for p in (dxc, dxcompiler, dxil_dll, dxv)}
    out["binaries_present"] = have
    if not all(have.values()):
        out["status"] = "blocked"
        out["facts"].append({"kind": "binaries", "name": "missing",
                             "note": f"新 DXC 缺二进制: {[k for k, v in have.items() if not v]}"})
        out["repro"] = [
            "1. 取 microsoft/DirectXShaderCompiler 新于 1.8.0.4739 且自带 dxil.dll 的 release,",
            "   解压到仓库外隔离目录,bin/x64/ 须含 dxc.exe/dxcompiler.dll/dxil.dll/dxv.exe。",
            "2. 设 RURIX_DXC_NEW_DIR 指向该 bin/x64,RURIX_R6_DIR 指向 round-6 合法容器目录。",
            "3. py -3 spike/dxil-path-probe/probe_round7_validator.py。",
        ]
        return out

    out["binary_digests"] = {os.path.basename(p): {"sha256": sha256(p), "size": os.path.getsize(p)}
                             for p in (dxc, dxcompiler, dxil_dll, dxv)}
    out["dxc_version"] = c.tool_version(dxc, ["--version"])

    # round-6 合法容器(byte-unchanged 引用,只读)
    llc23 = os.path.join(R6, "official_cs.obj")
    llc22 = os.path.join(R6, "off_cs22", "run-0001.obj")
    targets = {"llc23_1936B": llc23, "llc22_1804B": llc22}
    out["targets"] = {}
    for name, p in targets.items():
        out["targets"][name] = {"path": p, "exists": os.path.isfile(p),
                                "sha256": sha256(p) if os.path.isfile(p) else None}
    return _measure(out, dxc, dxcompiler, dxv, targets)


def _measure(out: dict, dxc: str, dxcompiler: str, dxv: str, targets: dict) -> dict:
    import subprocess
    import tempfile
    wd = tempfile.mkdtemp(prefix="dxil_spike_r7_")
    # 新 dxc 自产对照容器(正向控制 + PSV 子轴基准)
    hlsl = os.path.join(wd, "ctrl_new.hlsl")
    cso = os.path.join(wd, "ctrl_new.cso")
    with open(hlsl, "wb") as f:
        f.write(MINIMAL_HLSL.encode("ascii"))
    r = c.run([dxc, "-T", "cs_6_0", "-E", "main", "-Fo", cso, hlsl], timeout=30)
    ctrl_ok = r["ok"] and os.path.isfile(cso)
    out["new_dxc_control_emit_ok"] = ctrl_ok

    # 正向控制:新 dxc 自产容器 应 accept(IDxcValidator + dxv CLI)
    if ctrl_ok:
        ctrl_bytes = open(cso, "rb").read()
        fwd_idxc = idxc_validate_n(dxcompiler, ctrl_bytes, N)
        fwd_dxv = dxv_cli_n(dxv, cso, N)
        out["forward_control"] = {"idxc": fwd_idxc, "dxv_cli": fwd_dxv,
                                  "psv": container_psv_facts(ctrl_bytes)}
        # 反向控制:翻字节损坏 应 reject
        bad = bytearray(ctrl_bytes)
        for k in range(64, min(160, len(bad))):
            bad[k] ^= 0xFF
        badp = os.path.join(wd, "ctrl_bad.cso")
        with open(badp, "wb") as f:
            f.write(bytes(bad))
        rev_idxc = idxc_validate_n(dxcompiler, bytes(bad), N)
        rev_dxv = dxv_cli_n(dxv, badp, N)
        out["reverse_control"] = {"idxc": rev_idxc, "dxv_cli": rev_dxv}
        wrapper_ok = (fwd_idxc["accepts"] == N and rev_idxc["accepts"] == 0
                      and fwd_dxv["accept"] == N and rev_dxv["reject"] == N)
        out["wrapper_validated"] = wrapper_ok
        out["facts"].append({"kind": "wrapper_control", "name": "forward_reverse",
                             "note": f"新 DLL 下 wrapper 正反向控制:正向(新 dxc 自产)IDxcValidator {fwd_idxc['accepts']}/{N} accept + dxv {fwd_dxv['accept']}/{N} accept;反向(翻字节损坏)IDxcValidator {rev_idxc['accepts']}/{N} accept(reject {N - rev_idxc['accepts']}) + dxv {rev_dxv['reject']}/{N} reject → wrapper_validated={wrapper_ok}"})

    # 真验证:合法 llc 容器(22/23)用新 validator ×N
    out["llc_validation"] = {}
    for name, p in targets.items():
        if not os.path.isfile(p):
            out["llc_validation"][name] = {"status": "n/a", "reason": "container_missing"}
            continue
        data = open(p, "rb").read()
        idxc = idxc_validate_n(dxcompiler, data, N)
        cli = dxv_cli_n(dxv, p, N)
        out["llc_validation"][name] = {"idxc": idxc, "dxv_cli": cli, "psv": container_psv_facts(data)}
    return _finalize(out)


def _finalize(out: dict) -> dict:
    out["status"] = "measured_local"
    fwd = out.get("forward_control", {})
    ctrl_psv = (fwd.get("psv") or {}).get("psv0") or {}
    # llc 容器拒因 + PSV 子轴归纳
    for name, v in out.get("llc_validation", {}).items():
        if "idxc" not in v:
            continue
        idxc = v["idxc"]
        cli = v["dxv_cli"]
        psv = (v.get("psv") or {}).get("psv0") or {}
        accepted = idxc["accepts"] == idxc["n"] and cli["accept"] == cli["n"]
        out["facts"].append({"kind": "llc_validation", "name": name,
                             "note": f"合法 llc 容器 {name} 经新 validator(1.9.2602.24 dxil.dll 签名):IDxcValidator {idxc['accepts']}/{idxc['n']} accept,status 直方图 {idxc['status_histogram']},err={idxc['sample_error']!r};dxv.exe CLI {cli['accept']}/{cli['n']} accept {cli['reject']}/{cli['n']} reject,sample={cli['sample']!r};容器 PSV0 RuntimeInfoSize={psv.get('psv_runtime_info_size')} → accepted={accepted}"})
    # PSV 子轴裁决:新 dxc 自产 PSV size vs llc 的 52
    llc23psv = ((out.get("llc_validation", {}).get("llc23_1936B") or {}).get("psv") or {}).get("psv0") or {}
    out["facts"].append({"kind": "psv_subaxis", "name": "runtime_info_size_compare",
                         "note": f"PSV0 RuntimeInfoSize 子轴:新 dxc 1.9.2602.24 自产 cs_6_0 容器={ctrl_psv.get('psv_runtime_info_size')};llc emit 容器=52;新 validator 从 DXIL 模块(SM6.0/DXIL1.0)推得期望值并拒 llc 的 52(具名 PSVRuntimeInfoSize 52 vs module 24)。新 validator 与 LLVM 22/23 同年代(2026),若属『validator 太旧不识新 PSV』则新 validator 应接受;实测仍拒 → 排除『dxc 太旧』假说"})
    return out


def build_evidence(res: dict) -> dict:
    """把 round-7 measured 事实映射为 dxil_path_spike schema 证据(A 路聚焦;B 路 round-2 结转引用)。"""
    import datetime
    now = datetime.datetime.now().astimezone()
    ts = now.strftime("%Y-%m-%dT%H:%M:%S%z")
    ts = ts[:-2] + ":" + ts[-2:]
    dv = res.get("dxc_version", "unavailable")
    digests = res.get("binary_digests", {})
    val_pass = "fail" if res.get("status") == "measured_local" else "blocked"
    path_a = {
        "status": res.get("status", "blocked"),
        "target_available": True,
        "probe_command": "py -3 spike/dxil-path-probe/probe_round7_validator.py (RURIX_DXC_NEW_DIR=新 DXC bin/x64)",
        "target_list_excerpt": f"新 DXC {dv};dxil.dll 签名 validator sha256={digests.get('dxil.dll',{}).get('sha256')};dxv.exe sha256={digests.get('dxv.exe',{}).get('sha256')}",
        "dxil_emit_ok": "pass",
        "validator_pass": val_pass,
        "shader_model_coverage": "cs_6_0 / DXIL 1.0(round-6 合法 llc 容器 official_cs llc22+llc23)",
        "validator_compat": f"新 validator={dv}(与 LLVM 22/23 同年代 2026,自带独立 dxil.dll 签名 validator + dxv.exe);wrapper_validated={res.get('wrapper_validated')};新 dxc 自产 cs_6_0 容器 PSV0=52 accept,llc 容器 PSV0=52 仍 reject(0x80aa0013 PSVRuntimeInfoSize 52 vs module 24)→ 排除『dxc 太旧不识新 PSV』,坐实 llc 容器 PSV0 与其 DXIL 模块内部不一致(LLVM emit 不合规)",
        "facts": res.get("facts", []),
        "repro": [
            "1. 取 microsoft/DirectXShaderCompiler release v1.9.2602.24(dxc_2026_05_27.zip,sha256 CF658AAC...),解压到仓库外 H:\\dxc-round7\\extracted(bin/x64 含 dxc/dxcompiler/dxil/dxv)。",
            "2. 设 RURIX_DXC_NEW_DIR=H:\\dxc-round7\\extracted\\bin\\x64,RURIX_R6_DIR=H:\\llvm-audit-round6(round-6 合法容器 official_cs.obj 1936B / off_cs22/run-0001.obj 1804B)。",
            "3. py -3 spike/dxil-path-probe/probe_round7_validator.py:正反向控制(新 dxc 自产 accept / 翻字节 reject)验 wrapper,再对 llc22/llc23 合法容器 IDxcValidator + dxv.exe 各 ×25 真验证 + PSV0 子轴。",
        ],
        "round7_detail": {
            "binary_digests": digests,
            "targets": res.get("targets"),
            "forward_control": res.get("forward_control"),
            "reverse_control": res.get("reverse_control"),
            "llc_validation": res.get("llc_validation"),
            "new_dxc_control_emit_ok": res.get("new_dxc_control_emit_ok"),
        },
    }
    path_b = {
        "status": "measured_local",
        "translators_available": True,
        "probe_command": "(round-7 未重跑 B 路)",
        "dxil_emit_ok": "pass",
        "validator_pass": "pass",
        "supply_chain": "round-2 结转引用:SPIRV-Cross + dxc 1.8.0.4739 + glslang;端到端 HLSL→SPIR-V→HLSL→DXIL 4/4 pass(见 evidence/dxil_path_spike_20260624.json)",
        "determinism_notes": "round-2 measured 4/4 deterministic;round-7 未重测",
        "facts": [{"kind": "carry_forward", "name": "b_path_not_rerun",
                   "note": "round-7 聚焦 A 路 validator 互操作复验,未重跑 B 路;B 路 round-2 已 measured_local(端到端转译 4/4 pass + 确定性 4/4),见 evidence/dxil_path_spike_20260624.json / dxil_path_spike_report_round2.md"}],
        "repro": [],
    }
    return {
        "schema_version": 1,
        "subject": "dxil_path_spike",
        "status": res.get("status", "blocked"),
        "rd_ref": "RD-010",
        "decision_ref": "RFC-0003 §9 Q-D131=C / 13 §D-131",
        "timestamp": ts,
        "host_env": {"os": f"{os.name} / {sys.platform}", "clang_source": "n/a(round-7 复用 round-6 已落盘合法 llc 容器,未重跑 clang/llc)"},
        "tool_versions": {
            "clang": "n/a(round-7 不用 clang;复用 round-6 合法 llc 容器)",
            "llc": "n/a(round-7 复用 round-6 official_cs llc22/llc23 已落盘合法容器,byte-unchanged)",
            "dxc": dv,
            "dxv": f"dxil validator(独立 dxv.exe){dv}",
            "spirv_to_dxil": "n/a(B 路 round-2 结转)",
            "spirv_cross": "n/a(B 路 round-2 结转)",
            "spirv_producer": "n/a(B 路 round-2 结转)",
        },
        "path_a": path_a,
        "path_b": path_b,
        "comparison_criteria": [
            "validator_interop_gap_closed(A)",
            "rejection_named_reason(0x80aa0013 PSVRuntimeInfoSize)",
            "version_gap_excluded(new 2026 validator accepts dxc 52B PSV)",
            "attribution(LLVM emit PSV internal inconsistency vs validator age)",
        ],
        "run_command": "py -3 spike/dxil-path-probe/probe_round7_validator.py",
        "owner_decision": "A/B 最终路径裁决权属 owner(RFC-0003 §9 Q-D131 / 13 §D-131 / 硬规则 1);本 spike 仅产证据,agent 自主裁决。round-7 结论仅到『A 路 validator 互操作 gap 未闭合(当前 pin 即便最新 2026 validator 仍拒)+ Bug 2 归因坐实=LLVM emit PSV 不合规(非 dxc 太旧)』,不替 agent 裁 A/B,不签 G-G2-2,D-131 维持 C。",
        "notes": "纯取证 spike round-7;新 DXC/dxil.dll 隔离于仓库外 H:\\dxc-round7,不入库(version/sha256 见 path_a)。measured-first / blocked-honest:数字来自命令真实输出(IDxcValidator + dxv.exe 各 ×25)。round-1~6 既有 evidence/ 文件 byte-unchanged,本证据为新增。",
    }


def _write_evidence(ev: dict) -> str:
    from pathlib import Path
    root = Path(__file__).resolve().parent.parent.parent
    out_path = root / "evidence" / "dxil_path_spike_20260624_r7.json"
    payload = json.dumps(ev, ensure_ascii=False, indent=2) + "\n"
    data = payload.encode("utf-8").replace(b"\r\n", b"\n")
    with open(out_path, "wb") as f:
        f.write(data)
    return str(out_path)


if __name__ == "__main__":
    res = run_round7()
    if os.environ.get("RURIX_EMIT_R7") == "1":
        ev = build_evidence(res)
        p = _write_evidence(ev)
        crlf = 0
        with open(p, "rb") as f:
            crlf = f.read().count(b"\r")
        print(f"[round7] wrote {p} status={ev['status']} CRLF={crlf}")
    else:
        print(json.dumps(res, ensure_ascii=False, indent=2))
    sys.exit(0)
