# SPIKE(RD-017) — 输出 varying / fragment 输入 varying 用户语义名保名机制取证。
# 隔离于 spike/rd017-varying-semantic/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest(AGENTS 硬规则 3/4):工具链探到则跑端到端实测,
# 探不到如实 blocked + repro,绝不杜撰。
#
# owner ruling(本任务):选项① HLSL 文本边界保名改写;否决③(不放宽 signature_gate)。
# 本探针验证四件事(owner 验收点):
#   (a) 改写后 dxc 接受(B 链不破);
#   (b) 用户语义名端到端存活进 DXIL 签名 → signature_gate(semantic_name_matches)
#       **不放宽**也能过(strip 尾随 index 数字后大写全等);
#   (c) ABI 中立:改写**只动 HLSL struct field 的 semantic token**,不改 register /
#       mask / packing / 类型 / 字段名 / 行数(若触及即升 Full RFC,owner 边界 B);
#   (d) 确定性:同输入 ×N 改写字节一致。
"""RD-017 保名改写候选机制取证探针(自含,不依赖 src/)。

链路:命名 HLSL → dxc -spirv → spirv-cross → 回译 HLSL(语义退化 TEXCOORD#)
      → [候选改写:按 location provenance 把目标 struct 的 TEXCOORD# 改回用户名]
      → dxc → DXIL → dxc -dumpbin → 解析 ISG1/OSG1 签名名。
对照:同语料直产(dxc 原生)签名作上界基线。
"""
from __future__ import annotations

import datetime
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
CORPUS_DIR = HERE / "corpus"
N = int(os.environ.get("RURIX_RD017_N", "8"))
UNAVAILABLE = "unavailable"


def run(args: list[str], timeout: int = 40) -> dict:
    """安全执行(shell=False,list 参数,禁字符串插值,防注入)。结构化返回,绝不抛。"""
    try:
        proc = subprocess.run(
            args, capture_output=True, text=True, encoding="utf-8",
            errors="replace", timeout=timeout, check=False,
        )
        return {"ok": proc.returncode == 0, "rc": proc.returncode,
                "stdout": proc.stdout or "", "stderr": proc.stderr or "", "error": None}
    except FileNotFoundError:
        return {"ok": False, "rc": None, "stdout": "", "stderr": "", "error": "not_found"}
    except subprocess.TimeoutExpired:
        return {"ok": False, "rc": None, "stdout": "", "stderr": "", "error": "timeout"}
    except OSError as e:
        return {"ok": False, "rc": None, "stdout": "", "stderr": "", "error": f"oserror:{e}"}


def locate(name: str, env_var: str | None = None) -> str | None:
    if env_var:
        v = os.environ.get(env_var)
        if v and Path(v).is_file():
            return v
    return shutil.which(name)


def tool_version(path: str | None, args: list[str]) -> str:
    if not path:
        return UNAVAILABLE
    r = run([path] + args, timeout=15)
    text = (r["stdout"] or r["stderr"]).strip()
    return text.splitlines()[0].strip() if text else UNAVAILABLE


# --- HLSL struct 语义解析 + DXIL 签名表解析 -------------------------------------

# 匹配 spirv-cross 回译 HLSL 的 struct field 行:`  <type> <field> : <SEMANTIC>;`
FIELD_RE = re.compile(r"^(?P<lead>\s*)(?P<ty>[A-Za-z_]\w*)\s+(?P<name>[A-Za-z_]\w*)\s*:\s*(?P<sem>[A-Za-z_]\w*)\s*;\s*$")


def parse_struct_semantics(hlsl: str) -> dict[str, list[dict]]:
    """解析回译 HLSL 各 struct 的 field→semantic 列表(保留行号,供改写定位)。"""
    out: dict[str, list[dict]] = {}
    cur: str | None = None
    for i, line in enumerate(hlsl.splitlines()):
        s = line.strip()
        m = re.match(r"^struct\s+([A-Za-z_]\w*)", s)
        if m:
            cur = m.group(1)
            out[cur] = []
            continue
        if cur is None:
            continue
        if s.startswith("};") or s == "}":
            cur = None
            continue
        fm = FIELD_RE.match(line)
        if fm:
            out[cur].append({
                "line_no": i, "type": fm.group("ty"),
                "field": fm.group("name"), "semantic": fm.group("sem"),
            })
    return out


def texcoord_location(sem: str) -> int | None:
    """`TEXCOORD<loc>` → loc;非 TEXCOORD# 返回 None(builtin/SV 等不动)。"""
    m = re.fullmatch(r"(?i)TEXCOORD(\d+)", sem)
    return int(m.group(1)) if m else None


def parse_dxil_sig_table(disasm: str) -> dict[str, list[dict]]:
    """复刻 toolchain.rs parse_dxil_signatures 的注释表路径:`; Input/Output signature:`。
    取 Name/Index/Mask/Register/SysValue 列。仅取 gate 关心的 name+sysvalue+register。
    """
    sigs: dict[str, list[dict]] = {"input": [], "output": []}
    sec = None
    for raw in disasm.splitlines():
        line = raw.lstrip()
        body = line[1:].strip() if line.startswith(";") else ""
        if body.startswith("Input signature:"):
            sec = "input"; continue
        if body.startswith("Output signature:"):
            sec = "output"; continue
        if sec is None:
            continue
        if not line.startswith(";"):
            sec = None; continue
        if not body:
            continue
        toks = body.split()
        if not toks or toks[0] == "Name" or toks[0].startswith("---"):
            continue
        if len(toks) < 6:
            continue
        if not toks[1].isdigit():
            continue
        sigs[sec].append({
            "name": toks[0], "index": int(toks[1]), "mask": toks[2],
            "register": toks[3], "sysvalue": toks[4],
        })
    return sigs


def strip_trailing_digits(s: str) -> str:
    return s.rstrip("0123456789")


def gate_name_matches(dxil_name: str, field: str) -> bool:
    """复刻 signature_gate::semantic_name_matches(不放宽):大小写无关 + 剥尾随 index。"""
    lhs = strip_trailing_digits(dxil_name).upper()
    rhs = field.strip().upper()
    return bool(rhs) and lhs == rhs


# --- 候选改写(选项①)+ ABI 中立断言 -------------------------------------------

def rewrite_struct_semantics(hlsl: str, struct_name: str, loc_to_name: dict[int, str]) -> tuple[str, list[dict]]:
    """把 `struct_name` 内 `TEXCOORD<loc>` 语义 token 按 location provenance 改回用户名。
    **只**替换 field 行 `:` 与 `;` 之间的 semantic token,其余字符一律不动。
    """
    structs = parse_struct_semantics(hlsl)
    fields = structs.get(struct_name, [])
    target_lines: dict[int, str] = {}
    changes: list[dict] = []
    for f in fields:
        loc = texcoord_location(f["semantic"])
        if loc is None or loc not in loc_to_name:
            continue
        target_lines[f["line_no"]] = loc_to_name[loc]
        changes.append({"line_no": f["line_no"], "field": f["field"],
                        "from": f["semantic"], "to": loc_to_name[loc], "location": loc})
    lines = hlsl.splitlines(keepends=True)
    for ln, new_sem in target_lines.items():
        raw = lines[ln]
        nl = "\n" if raw.endswith("\n") else ("\r\n" if raw.endswith("\r\n") else "")
        body = raw[: len(raw) - len(nl)] if nl else raw
        # 仅替换最后一个 `:` 后、`;` 前的 token,前缀(缩进+类型+字段名+冒号)与
        # 尾随 `;` 原样保留 → 证明只动 semantic token。
        m = re.match(r"^(?P<pre>.*:\s*)(?P<sem>[A-Za-z_]\w*)(?P<post>\s*;.*)$", body)
        if not m:
            continue
        lines[ln] = m.group("pre") + new_sem + m.group("post") + nl
    return "".join(lines), changes


def assert_abi_neutral(before: str, after: str, changed_line_nos: set[int]) -> dict:
    """断言改写**只动语义名文本**:行数不变;非目标行逐字节相同;目标行仅 `:`↔`;`
    之间的 semantic token 变化,类型/字段名/缩进/分号与寄存器 packing 不变。"""
    b = before.splitlines()
    a = after.splitlines()
    result = {"line_count_invariant": len(b) == len(a), "violations": []}
    if not result["line_count_invariant"]:
        result["violations"].append(f"line_count {len(b)}->{len(a)}")
        return result
    for i, (lb, la) in enumerate(zip(b, a)):
        if lb == la:
            continue
        if i not in changed_line_nos:
            result["violations"].append({"line_no": i, "reason": "non_target_line_changed",
                                         "before": lb, "after": la})
            continue
        mb = re.match(r"^(.*:\s*)([A-Za-z_]\w*)(\s*;.*)$", lb)
        ma = re.match(r"^(.*:\s*)([A-Za-z_]\w*)(\s*;.*)$", la)
        if not (mb and ma):
            result["violations"].append({"line_no": i, "reason": "unpar_field_line"})
            continue
        # 前缀(含缩进+类型+字段名+冒号)与后缀(分号及之后)必须逐字节不变。
        if mb.group(1) != ma.group(1) or mb.group(3) != ma.group(3):
            result["violations"].append({"line_no": i, "reason": "prefix_or_suffix_changed",
                                         "before": lb, "after": la})
    result["abi_neutral"] = result["line_count_invariant"] and not result["violations"]
    return result


# --- B 链编排 + 取证 ------------------------------------------------------------

sys.path.insert(0, str(HERE.parent / "dxil-path-probe"))
try:
    import dxil_container as dc  # 复用 RD-010 spike 的 DXContainer 签名 part 解析(只读字节)
    _HAVE_DC = True
except Exception:  # noqa: BLE001
    _HAVE_DC = False


def _sha256(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _dxil_sig(path: Path, part: str) -> dict:
    """从编译产物的 DXContainer 取 ISG1/OSG1 签名(semantic_name + ABI 数值面)。"""
    if not (_HAVE_DC and path.is_file()):
        return {"ok": False, "reason": "no_container_or_dc"}
    return dc.parse_signature_part(path.read_bytes(), part)


def _find_struct(hlsl: str, substr: str) -> str | None:
    for name in parse_struct_semantics(hlsl):
        if substr.lower() in name.lower():
            return name
    return None


def _gate(sig: dict, intent: dict[int, str]) -> dict:
    """对签名跑 signature_gate 等价判定(**不放宽**):每个 intent 用户名须以等价名出现。"""
    names = [e["semantic_name"] for e in sig.get("elements", [])] if sig.get("ok") else []
    per = []
    ok_all = sig.get("ok", False)
    for want in intent.values():
        hit = any(gate_name_matches(n, want) for n in names)
        per.append({"want": want, "found_equiv": hit})
        ok_all = ok_all and hit
    return {"dxil_names": names, "checks": per, "gate_pass": ok_all}


def _binary_abi_diff(before: dict, after: dict) -> dict:
    """二进制层 ABI 中立证据:degraded vs rewritten 签名逐元素对照。

    owner 边界 B 的"物理 ABI"= register / mask / comp_type / system_value(+ packing /
    byte layout / Location 数值)→ **必须不变**(`physical_abi_invariant`)。
    `semantic_index`(语义名的 index 后缀,如 `TEXCOORD0` 的 `0`)属**语义名维度**,
    signature_gate 明确**不比对**(剥尾随数字);三个共享基名 `TEXCOORD` 的 index 0/1/2
    在恢复为三个不同用户名后各自归 index 0,是改名的**自然后果**,非物理 ABI 触碰,
    单列追踪不计为违例。"""
    if not (before.get("ok") and after.get("ok")):
        return {"comparable": False}
    eb, ea = before["elements"], after["elements"]
    if len(eb) != len(ea):
        return {"comparable": True, "elemcount_invariant": False}
    phys_viol, name_changes, sem_idx_changes = [], [], []
    for i, (b, a) in enumerate(zip(eb, ea)):
        for k in ("register", "mask", "comp_type", "system_value"):
            if b.get(k) != a.get(k):
                phys_viol.append({"idx": i, "field": k, "before": b.get(k), "after": a.get(k)})
        if b.get("semantic_index") != a.get("semantic_index"):
            sem_idx_changes.append({"idx": i, "before": b.get("semantic_index"), "after": a.get("semantic_index")})
        if b["semantic_name"] != a["semantic_name"]:
            name_changes.append({"idx": i, "from": b["semantic_name"], "to": a["semantic_name"]})
    return {"comparable": True, "elemcount_invariant": True,
            "physical_abi_invariant": not phys_viol, "physical_abi_violations": phys_viol,
            "semantic_index_changes": sem_idx_changes,
            "semantic_name_changes": name_changes}


def _run_sample(name, profile, struct_substr, part, intent, dxc, cross, wd) -> dict:
    src = CORPUS_DIR / f"{name}.hlsl"
    s = {"name": name, "profile": profile, "checked_part": part}
    if not src.is_file():
        return {**s, "status": "blocked", "reason": "corpus_missing"}
    spv = wd / f"{name}.spv"
    r1 = run([dxc, "-T", profile, "-E", "main", "-spirv", "-Fo", str(spv), str(src)])
    s["spirv_emit"] = "pass" if (r1["ok"] and spv.is_file()) else "fail"
    if s["spirv_emit"] != "pass":
        return {**s, "status": "blocked", "reason": "spirv_emit_failed", "stderr": r1["stderr"][:400]}
    deg = wd / f"{name}.degraded.hlsl"
    r2 = run([cross, "--hlsl", "--shader-model", "60", str(spv), "--output", str(deg)])
    s["spirv_cross"] = "pass" if (r2["ok"] and deg.is_file()) else "fail"
    if s["spirv_cross"] != "pass":
        return {**s, "status": "blocked", "reason": "spirv_cross_failed", "stderr": r2["stderr"][:400]}
    deg_hlsl = deg.read_text(encoding="utf-8", errors="replace")
    struct_name = _find_struct(deg_hlsl, struct_substr)
    s["target_struct"] = struct_name
    s["degraded_semantics"] = [
        {"field": f["field"], "semantic": f["semantic"]}
        for f in parse_struct_semantics(deg_hlsl).get(struct_name or "", [])
    ]
    # baseline(改写前):退化 HLSL → DXIL → 签名 → gate(应拒)。
    degx = wd / f"{name}.degraded.dxil"
    rb = run([dxc, "-T", profile, "-E", "main", "-Fo", str(degx), str(deg)])
    s["degraded_dxil_emit"] = "pass" if (rb["ok"] and degx.is_file()) else "fail"
    sig_deg = _dxil_sig(degx, part)
    s["gate_before_rewrite"] = _gate(sig_deg, intent)
    # 候选改写(选项①)。
    rewritten, changes = rewrite_struct_semantics(deg_hlsl, struct_name or "", intent)
    s["rewrite_changes"] = changes
    s["hlsl_abi_neutral"] = assert_abi_neutral(deg_hlsl, rewritten, {c["line_no"] for c in changes})
    rwf = wd / f"{name}.rewritten.hlsl"
    rwf.write_text(rewritten, encoding="utf-8")
    # 改写后:HLSL → DXIL(dxc 接受?)→ 签名 → gate(应过,不放宽)。
    rwx = wd / f"{name}.rewritten.dxil"
    rr = run([dxc, "-T", profile, "-E", "main", "-Fo", str(rwx), str(rwf)])
    s["rewritten_dxil_emit"] = "pass" if (rr["ok"] and rwx.is_file() and rwx.read_bytes()[:4] == b"DXBC") else "fail"
    s["dxc_stderr_len"] = len((rr["stderr"] or "").strip())
    sig_rw = _dxil_sig(rwx, part)
    s["gate_after_rewrite"] = _gate(sig_rw, intent)
    s["binary_abi_diff"] = _binary_abi_diff(sig_deg, sig_rw)
    # 确定性:改写文本 ×N 字节一致 + 改写后 DXIL ×N sha256 一致。
    txts = {_sha256(rewrite_struct_semantics(deg_hlsl, struct_name or "", intent)[0].encode("utf-8")) for _ in range(N)}
    dxil_hashes = set()
    for i in range(N):
        sub = wd / f"{name}_det{i}"; sub.mkdir(exist_ok=True)
        f = sub / "r.hlsl"; f.write_text(rewritten, encoding="utf-8")
        o = sub / "r.dxil"
        run([dxc, "-T", profile, "-E", "main", "-Fo", str(o), str(f)])
        dxil_hashes.add(_sha256(o.read_bytes()) if o.is_file() else "")
    s["determinism"] = {"n": N, "rewrite_text_unique": len(txts), "rewrite_text_consistent": len(txts) == 1,
                        "rewritten_dxil_unique": len(dxil_hashes), "rewritten_dxil_consistent": len(dxil_hashes) == 1 and "" not in dxil_hashes}
    s["status"] = "measured_local"
    return s


# --- main ----------------------------------------------------------------------

SAMPLES = [
    # name, profile, target struct 子串, 检查签名 part, location→用户语义 provenance
    ("vs_named", "vs_6_0", "Output", "OSG1", {0: "NORMAL", 1: "WORLDPOS", 2: "UV"}),
    ("ps_named", "ps_6_0", "Input", "ISG1", {0: "NORMAL", 1: "WORLDPOS", 2: "UV"}),
]


def probe() -> dict:
    dxc = locate("dxc", env_var="RURIX_DXC")
    cross = locate("spirv-cross", env_var="RURIX_SPIRV_CROSS")
    out = {
        "schema_version": 1,
        "subject": "rd017_varying_semantic_rewrite",
        "rd_ref": "RD-017",
        "decision_ref": "owner ruling: 选项① HLSL 边界保名改写 / 否决③(不放宽 signature_gate)",
        "host_env": {"os": sys.platform},
        "tool_versions": {
            "dxc": tool_version(dxc, ["--version"]),
            "spirv_cross": tool_version(cross, ["--version"]) or "(no --version)",
        },
        "validator_note": "签名 validator(dxv/dxil.dll)为 owner pin 环境;本探针只测 dxc 接受 + 签名名存活 + ABI 中立 + 确定性,不代签 golden/device(G-G2-4 owner)。",
        "samples": [],
    }
    if not (dxc and cross and _HAVE_DC):
        out["status"] = "blocked"
        out["notes"] = "dxc/spirv-cross 缺失或 dxil_container 不可用;repro:Vulkan SDK Bin 入 PATH。"
        return out
    wd = Path(tempfile.mkdtemp(prefix="rd017_rewrite_"))
    for name, profile, sub, part, intent in SAMPLES:
        out["samples"].append(_run_sample(name, profile, sub, part, intent, dxc, cross, wd))
    # 顶层判据汇总(供 owner / 报告速读)。
    measured = [s for s in out["samples"] if s.get("status") == "measured_local"]
    out["summary"] = {
        "samples_measured": len(measured),
        "all_dxc_accept_after_rewrite": all(s.get("rewritten_dxil_emit") == "pass" for s in measured) if measured else False,
        "all_gate_pass_after_rewrite_unrelaxed": all(s.get("gate_after_rewrite", {}).get("gate_pass") for s in measured) if measured else False,
        "all_gate_reject_before_rewrite": all(not s.get("gate_before_rewrite", {}).get("gate_pass") for s in measured) if measured else False,
        "all_hlsl_abi_neutral": all(s.get("hlsl_abi_neutral", {}).get("abi_neutral") for s in measured) if measured else False,
        "all_physical_abi_invariant": all(s.get("binary_abi_diff", {}).get("physical_abi_invariant") for s in measured) if measured else False,
        "all_deterministic": all(s.get("determinism", {}).get("rewrite_text_consistent") for s in measured) if measured else False,
    }
    out["status"] = "measured_local" if measured else "blocked"
    return out


if __name__ == "__main__":
    res = probe()
    res["timestamp"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    if os.environ.get("RURIX_EMIT_RD017"):
        date = os.environ.get("RURIX_RD017_DATE", datetime.date.today().strftime("%Y%m%d"))
        dest = HERE.parent.parent / "evidence" / f"rd017_varying_semantic_spike_{date}.json"
        blob = json.dumps(res, ensure_ascii=False, indent=2).encode("utf-8")
        with open(dest, "wb") as f:
            f.write(blob.replace(b"\r\n", b"\n") + b"\n")
        print(f"wrote {dest}")
    else:
        print(json.dumps(res, ensure_ascii=False, indent=2))
    sys.exit(0)
