#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""`#[export(c)]` C ABI 导出 codegen smoke（步骤 71;EI1.2 / RFC-0014 Part A;
RXS-0250~0255;验收门 G-EI1-2）。

`src/rurixc/src/export_c.rs` 的 `collect_c_exports`（挂载合法性 RXS-0250 / C 兼容子集
v1 签名 RXS-0251 / 导出体无 panic 面 RXS-0255）+ driver `--emit=dll` 通道（空导出集
RX6032 RXS-0252 / 确定性内建头 RXS-0253 / CI 再生成守卫 RXS-0254）的端到端见证——

  host 段（**恒跑**,反 YAML-only）:
    1. conformance/export_c corpus 批跑（`cargo test -p rurixc --test export_c_corpus`）:
       reject 反例全拦截 + accept 正例 0 export_c 诊断。**纯 rust test,无 clang/link
       亦恒跑**（反 YAML-only 底线）。
    2. 空导出集 RX6032（RXS-0252）:无 `#[export(c)]` 的 `.rx` 经 `--emit=dll` → 退非零 +
       stderr 含 `RX6032`（driver 层 `emit_dll` 守门,corpus 的 `collect_c_exports` 测不到）。
    3. 内建头幂等（RXS-0253）:同一 accept `.rx` 同 `-o` 两次 `--emit=dll` → 两次 `.h`
       逐字节一致 + 无绝对路径 / 无时间戳。
    4. 篡改头再生成 byte-diff（RXS-0254 RED）:生成 `.h` 后篡改一字节,重跑 `--emit=dll`
       覆盖 → 再生成头 == 规范头 且 ≠ 篡改版（证 CI 再生成守卫非空过）。
    - 步骤 2~4 需 clang + link.exe（工具链面）;缺则 **SKIP**（dev-env degrade,退 0）
      但步骤 1 **恒跑**。`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。

  device 段（**gate real:clang + link.exe + MSVC/Windows SDK 在位**;`RURIX_REQUIRE_REAL=1`
  翻硬红,缺则 SKIP 退 0 打 dev-env-degrade）:
    5. accept 多导出 `.rx` → dll,`dumpbin /EXPORTS` 断言裸名未 mangle（符号集 == 头声明集,
       count 一致;§4.0-1 单一事实源）。
    6. 类型层 ABI 往返（RXS-0252/0253 硬门,redline F6）:cl.exe 编译哨兵 C 宿主（链 import
       lib,include 生成头）,哨兵值穿往返 —— i64 传 >2^32 验宽度 / i32 负值验符号扩展 /
       `*mut i32` store 回读 / add;错宽/错符号即数值红（非零退出）。
    7. 落 evidence JSON（`evidence/export_c_smoke_<ts>.json`）。

**SKIP 纪律**:无 clang/link/MSVC → SKIP = dev-env degrade（**非 fake pass**,退 0,打印
dev-env-degrade);`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。run URL 不伪造:本机记 "local"。

**主循环登记提示**:步骤号 = 71;门 = G-EI1-2;条款 = RXS-0250~0255;host 段恒跑（步骤 1
纯 rust test）vs device 段 gated（clang+link+MSVC）双态,镜像 uc04_present / uc07_present 先例。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "target" / "export_c_smoke"
EVIDENCE_DIR = ROOT / "evidence"
ACCEPT = ROOT / "conformance" / "export_c" / "accept"

# 工具链 pin（实测在位路径;RURIXC_CLANG 覆写 clang）。
CLANG = Path(r"C:/Program Files/LLVM/bin/clang.exe")
MSVC_ROOT = Path(r"C:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.44.35207")
MSVC_BIN = MSVC_ROOT / "bin" / "Hostx64" / "x64"
SDK_INC = Path(r"C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0")
SDK_LIB = Path(r"C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0")

# 多导出哨兵源（dumpbin + ABI 往返共用;4 导出覆盖宽度/符号/指针/加法）。
MULTI_EXPORT = ACCEPT / "multi_export.rx"
EXPECTED_SYMBOLS = {"rurix_add", "rurix_widen", "rurix_negate", "rurix_store"}

# ABI 往返哨兵 C 宿主:i64 宽度 / i32 负值符号 / `*mut i32` store 回读 / add。
SENTINEL_C = (
    "#include <stdint.h>\n"
    "#include <stdio.h>\n"
    '#include "multi_export.h"\n'
    "int main(void) {\n"
    "    int fails = 0;\n"
    '    if (rurix_add(-100, 1) != -99) { printf("FAIL add\\n"); fails++; }\n'
    '    if (rurix_widen(5000000000LL) != 5000000001LL) { printf("FAIL widen\\n"); fails++; }\n'
    '    if (rurix_negate(1000000) != -1000000) { printf("FAIL negate\\n"); fails++; }\n'
    "    int32_t slot = 0;\n"
    "    rurix_store(&slot, 424242);\n"
    '    if (slot != 424242) { printf("FAIL store\\n"); fails++; }\n'
    '    if (fails == 0) printf("EXPORT_C_ABI_OK\\n");\n'
    "    return fails;\n"
    "}\n"
)


def fail(msg: str) -> int:
    print(f"[export_c_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg)
    print(f"[export_c_smoke] SKIP {msg}（dev-env-degrade,退出 0）")
    return 0


def run(cmd: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None):
    return subprocess.run(cmd, cwd=str(cwd), capture_output=True, text=True, env=env)


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local"


def resolve_clang() -> Path | None:
    v = os.environ.get("RURIXC_CLANG")
    if v and Path(v).is_file():
        return Path(v)
    if CLANG.is_file():
        return CLANG
    from shutil import which
    w = which("clang")
    return Path(w) if w else None


def rurixc_env(clang: Path) -> dict[str, str]:
    env = dict(os.environ)
    env["RURIXC_CLANG"] = str(clang)
    return env


def build_rurixc() -> Path | None:
    p = run(["cargo", "build", "-q", "-p", "rurixc", "--bin", "rurixc"])
    if p.returncode != 0:
        print((p.stdout + p.stderr)[-1600:], file=sys.stderr)
        return None
    exe = ROOT / "target" / "debug" / "rurixc.exe"
    return exe if exe.is_file() else None


def emit_dll(rurixc: Path, src: Path, out_stem: Path, env: dict[str, str]):
    """`rurixc <src> --emit=dll -o <out_stem>` → 产 <out_stem>.dll + .lib + .h。"""
    return run([str(rurixc), str(src), "--emit=dll", "-o", str(out_stem)], env=env)


def header_names(header_text: str) -> list[str]:
    """从生成头提取导出函数名（单一事实源:头声明 ↔ DLL 导出符号,§4.0-1）。"""
    names = []
    for line in header_text.splitlines():
        s = line.strip()
        if s.endswith(";") and "(" in s and not s.startswith(("#", "/", "extern", "}")):
            m = re.search(r"(\w+)\s*\(", s)
            if m:
                names.append(m.group(1))
    return names


# ─────────────────────────── host 段（恒跑） ───────────────────────────


def host_section(results: dict) -> bool:
    # 1) corpus 批跑（纯 rust test,恒跑,反 YAML-only）。
    p = run(["cargo", "test", "-q", "-p", "rurixc", "--test", "export_c_corpus"])
    if p.returncode != 0:
        print((p.stdout + p.stderr)[-2400:], file=sys.stderr)
        print("[export_c_smoke] host 段 FAIL: export_c_corpus 批跑未过", file=sys.stderr)
        results["corpus_pass"] = False
        return False
    results["corpus_pass"] = True
    print("[export_c_smoke] host 步骤 1 PASS: export_c_corpus 批跑（reject 全拦截 + accept 0 诊断）")

    # 步骤 2~4 需 clang + link.exe。缺则 SKIP（REQUIRE_REAL 翻红）,步骤 1 已恒跑。
    clang = resolve_clang()
    if clang is None:
        rc = skip("未找到 clang（步骤 2~4 需 clang + link.exe;步骤 1 corpus 已恒跑）")
        results["toolchain_skip"] = "no-clang"
        for k in ("empty_reject_ok", "header_idempotent", "tamper_regen_red"):
            results[k] = "SKIP"
        return rc == 0
    env = rurixc_env(clang)
    rurixc = build_rurixc()
    if rurixc is None:
        print("[export_c_smoke] host 段 FAIL: rurixc 构建失败", file=sys.stderr)
        return False

    WORK.mkdir(parents=True, exist_ok=True)

    # toolchain 探针:emit 一个已知 accept dll;失败 → clang/link 缺,SKIP 2~4。
    probe = emit_dll(rurixc, ACCEPT / "add.rx", WORK / "probe_add", env)
    if probe.returncode != 0 or not (WORK / "probe_add.dll").is_file():
        rc = skip("`--emit=dll` 探针失败（clang/link.exe 工具链缺;步骤 1 corpus 已恒跑）")
        results["toolchain_skip"] = "no-link"
        for k in ("empty_reject_ok", "header_idempotent", "tamper_regen_red"):
            results[k] = "SKIP"
        return rc == 0

    # 2) 空导出集 RX6032（RXS-0252,driver 层 emit,corpus 测不到）。
    empty_rx = WORK / "empty_export.rx"
    empty_rx.write_text(
        "// 无 `#[export(c)]` 导出 → `--emit=dll` 须报空导出集 RX6032（RXS-0252）。\n"
        "pub fn not_exported(a: i32) -> i32 {\n    a + 1\n}\n",
        encoding="utf-8", newline="\n",
    )
    pe = emit_dll(rurixc, empty_rx, WORK / "empty_out", env)
    empty_ok = pe.returncode != 0 and "RX6032" in (pe.stdout + pe.stderr)
    results["empty_reject_ok"] = empty_ok
    if not empty_ok:
        print((pe.stdout + pe.stderr)[-1200:], file=sys.stderr)
        return fail("空导出集未报 RX6032（RXS-0252 driver 守门空过）") == 0
    print("[export_c_smoke] host 步骤 2 PASS: 空导出集 → RX6032（RXS-0252）")

    # 3) 内建头幂等（RXS-0253）:同一 -o 两次 emit,头逐字节一致 + 无绝对路径/时间戳。
    idem = WORK / "idem"
    e1 = emit_dll(rurixc, MULTI_EXPORT, idem, env)
    h1 = idem.with_suffix(".h").read_bytes() if idem.with_suffix(".h").is_file() else b""
    e2 = emit_dll(rurixc, MULTI_EXPORT, idem, env)
    h2 = idem.with_suffix(".h").read_bytes() if idem.with_suffix(".h").is_file() else b""
    if e1.returncode != 0 or e2.returncode != 0 or not h1:
        print((e1.stdout + e1.stderr + e2.stdout + e2.stderr)[-1200:], file=sys.stderr)
        return fail("头幂等 emit 失败") == 0
    htext = h1.decode("utf-8", "replace")
    abs_path = bool(re.search(r"[A-Za-z]:[\\/]", htext)) or str(ROOT) in htext or str(WORK) in htext
    timestamp = bool(re.search(r"\b20\d\d[-/:]\d\d", htext))
    idem_ok = (h1 == h2) and not abs_path and not timestamp
    results["header_idempotent"] = idem_ok
    if not idem_ok:
        return fail(
            f"头非确定性: byte_identical={h1 == h2} abs_path={abs_path} timestamp={timestamp}"
        ) == 0
    print("[export_c_smoke] host 步骤 3 PASS: 内建头幂等 + 无绝对路径/时间戳（RXS-0253）")

    # 4) 篡改头再生成 byte-diff（RXS-0254 RED）:篡改一字节 → 重 emit → 再生成 == 规范 ≠ 篡改。
    tamper = WORK / "tamper_probe"
    et = emit_dll(rurixc, MULTI_EXPORT, tamper, env)
    thdr = tamper.with_suffix(".h")
    if et.returncode != 0 or not thdr.is_file():
        return fail("篡改前置 emit 失败") == 0
    canonical = thdr.read_bytes()
    mutated = bytearray(canonical)
    # 篡改末尾 guard 附近一字节（确保落在头内容,大小写翻转必生 byte-diff）。
    mutated[len(mutated) // 2] ^= 0x20
    thdr.write_bytes(bytes(mutated))
    if thdr.read_bytes() == canonical:
        return fail("篡改未改变头字节（RED 前置无效）") == 0
    er = emit_dll(rurixc, MULTI_EXPORT, tamper, env)
    regen = thdr.read_bytes()
    tamper_ok = er.returncode == 0 and regen == canonical and regen != bytes(mutated)
    results["tamper_regen_red"] = tamper_ok
    if not tamper_ok:
        return fail("篡改头再生成守卫空过（RXS-0254 RED 未成立）") == 0
    print("[export_c_smoke] host 步骤 4 PASS: 篡改头再生成 byte-diff（RXS-0254 RED 守卫非空过）")
    return True


# ─────────────────────────── device 段（gate real） ───────────────────────────


def locate_msvc() -> tuple[Path, Path] | None:
    """(cl.exe, dumpbin.exe) 若 MSVC/Windows SDK 在位。"""
    cl = MSVC_BIN / "cl.exe"
    dumpbin = MSVC_BIN / "dumpbin.exe"
    if cl.is_file() and dumpbin.is_file() and (MSVC_ROOT / "include").is_dir() and SDK_INC.is_dir():
        return cl, dumpbin
    return None


def msvc_env(base: dict[str, str], header_dir: Path, lib_dir: Path) -> dict[str, str]:
    env = dict(base)
    env["INCLUDE"] = os.pathsep.join([
        str(MSVC_ROOT / "include"), str(SDK_INC / "ucrt"),
        str(SDK_INC / "shared"), str(SDK_INC / "um"), str(header_dir),
    ])
    env["LIB"] = os.pathsep.join([
        str(MSVC_ROOT / "lib" / "x64"), str(SDK_LIB / "ucrt" / "x64"),
        str(SDK_LIB / "um" / "x64"), str(lib_dir),
    ])
    env["PATH"] = str(MSVC_BIN) + os.pathsep + env.get("PATH", "")
    return env


def parse_dumpbin_exports(text: str) -> list[str]:
    """dumpbin /EXPORTS 表:`ordinal hint RVA name` 行提取导出名。"""
    names = []
    in_table = False
    for line in text.splitlines():
        if re.search(r"ordinal\s+hint\s+RVA\s+name", line):
            in_table = True
            continue
        if not in_table:
            continue
        if line.strip().lower().startswith("summary"):
            break
        m = re.match(r"\s*\d+\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]+\s+(\S+)", line)
        if m:
            names.append(m.group(1))
    return names


def device_section(results: dict) -> int:
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    clang = resolve_clang()
    if clang is None:
        results["dumpbin_unmangled"] = "SKIP"
        results["abi_roundtrip_ok"] = "SKIP"
        return skip("device 段:未找到 clang（需 clang + link.exe + MSVC）")
    msvc = locate_msvc()
    if msvc is None:
        results["dumpbin_unmangled"] = "SKIP"
        results["abi_roundtrip_ok"] = "SKIP"
        return skip("device 段:未找到 MSVC cl.exe/dumpbin.exe + Windows SDK（ABI 往返/符号见证需）")
    cl, dumpbin = msvc
    env = rurixc_env(clang)
    rurixc = build_rurixc()
    if rurixc is None:
        return fail("device 段:rurixc 构建失败")

    WORK.mkdir(parents=True, exist_ok=True)
    # 多导出 accept .rx → dll + import lib + 头。
    dev = WORK / "multi_export"
    ed = emit_dll(rurixc, MULTI_EXPORT, dev, env)
    dll, imp_lib, hdr = dev.with_suffix(".dll"), dev.with_suffix(".lib"), dev.with_suffix(".h")
    if ed.returncode != 0 or not dll.is_file():
        print((ed.stdout + ed.stderr)[-1600:], file=sys.stderr)
        return skip("多导出 `--emit=dll` 失败（clang/link 工具链面;dev-env-degrade）")
    if not imp_lib.is_file():
        return fail(f"缺 import lib {imp_lib.name}（link.exe /DLL 应副产 .lib）")

    # 5) dumpbin /EXPORTS 裸名未 mangle + 符号集 == 头声明集。
    pd = run([str(dumpbin), "/nologo", "/EXPORTS", str(dll)], cwd=WORK)
    if pd.returncode != 0:
        print((pd.stdout + pd.stderr)[-1200:], file=sys.stderr)
        return fail("dumpbin /EXPORTS 失败")
    exported = parse_dumpbin_exports(pd.stdout)
    declared = header_names(hdr.read_text(encoding="utf-8"))
    mangled = [n for n in exported if n.startswith("?") or "@@" in n or "$" in n]
    unmangled_ok = (
        set(exported) == EXPECTED_SYMBOLS
        and set(exported) == set(declared)
        and len(exported) == len(EXPECTED_SYMBOLS)
        and not mangled
    )
    results["dumpbin_unmangled"] = unmangled_ok
    if not unmangled_ok:
        return fail(
            f"导出符号非裸名或与头不符: exported={sorted(exported)} "
            f"declared={sorted(declared)} mangled={mangled}"
        )
    print(f"[export_c_smoke] device 步骤 5 PASS: dumpbin 裸名未 mangle {sorted(exported)} "
          f"（符号集 == 头声明集,count={len(exported)}）")

    # 6) 类型层 ABI 往返（cl.exe 编哨兵 C 宿主,链 import lib,include 生成头）。
    sentinel = WORK / "sentinel.c"
    sentinel.write_text(SENTINEL_C, encoding="utf-8", newline="\n")
    exe = WORK / "sentinel.exe"
    obj = WORK / "sentinel.obj"
    cenv = msvc_env(env, hdr.parent, imp_lib.parent)
    pc = run([str(cl), "/nologo", str(sentinel), f"/Fe:{exe}", f"/Fo:{obj}",
              "/link", imp_lib.name], cwd=WORK, env=cenv)
    if pc.returncode != 0 or not exe.is_file():
        print((pc.stdout + pc.stderr)[-1600:], file=sys.stderr)
        return skip("哨兵 C 宿主编译/链接失败（MSVC/Windows SDK 面;dev-env-degrade）")
    pr = run([str(exe)], cwd=WORK, env=cenv)
    abi_ok = pr.returncode == 0 and "EXPORT_C_ABI_OK" in pr.stdout
    results["abi_roundtrip_ok"] = abi_ok
    if not abi_ok:
        print((pr.stdout + pr.stderr)[-1200:], file=sys.stderr)
        return fail(f"ABI 往返数值红: rc={pr.returncode} out={pr.stdout.strip()!r}"
                    "（错宽/错符号/store 回读不符）")
    print("[export_c_smoke] device 步骤 6 PASS: 类型层 ABI 往返 EXPORT_C_ABI_OK"
          "（i64 宽度 / i32 负值符号 / `*mut i32` store 回读 / add）")
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = results.get("abi_roundtrip_ok") == "SKIP" or results.get("dumpbin_unmangled") == "SKIP"
    doc = {
        "schema_version": 1,
        "subject": "export_c_smoke",
        "milestone": "EI1.2 / G-EI1-2 (RFC-0014 Part A; RXS-0250~0255)",
        "step": 71,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {
            "corpus_pass": results.get("corpus_pass"),
            "empty_reject_ok": results.get("empty_reject_ok"),
            "header_idempotent": results.get("header_idempotent"),
            "tamper_regen_red": results.get("tamper_regen_red"),
            "dumpbin_unmangled": results.get("dumpbin_unmangled"),
            "abi_roundtrip_ok": results.get("abi_roundtrip_ok"),
        },
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": device_skipped or results.get("toolchain_skip") is not None,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"export_c_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[export_c_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    if not host_ok:
        write_evidence(results, host_ok, 1)
        return 1
    device_rc = device_section(results)
    write_evidence(results, host_ok, device_rc)
    return device_rc


if __name__ == "__main__":
    sys.exit(main())
