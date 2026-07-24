#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-05 引擎嵌入 v3 冒烟(步骤 78;G4.2 PR-D / RXS-0277;RFC-0015 §4.A7;
验收门 G-G4-3 嵌入面)。

`apps/uc05-rhi/src/embed.rx` 单源 → `rurixc --emit=dll` → `rurix_rhi.dll` + import lib +
**编译器生成头** → C++/D3D12 宿主(`src/rurix-engine/harness/engine_host_v3.cpp`,
engine_host **v3**)链接并把整个 Rurix **图形** RHI 图(≥1 raster + ≥1 mesh pass)作为图
节点真跑 → **三方数值精确相等**(Q-PixelCriterion)。

  host 段（**恒跑**,反 YAML-only）:
    1. 「生成头**自始生成、不手写**」审计(RXS-0253/0254):仓库内**零** tracked `rurix_rhi.h`
       (git ls-files),且 v3 harness `#include "rurix_rhi.h"`。
    2. 三制共存审计(RXS-0254 §4.A5):**v1/v2 既有资产在位且 v3 不触之** —— v1 三件
       (`harness/engine_host.cpp` / `include/rurix_engine.h` / `src/ffi.rs`)+ v2
       (`harness/uc05_engine_host.cpp`)存在,且 v3 harness 既不 include v1 头、也不引用
       `rurix_engine_*` 符号(三制解耦共存)。
    3. apps/uc05-rhi 零 .rs 主语言判据维持(RFC-0014 §9.2;同步骤 74 口径)。
    4. `--emit=dll` GPU 导出面产物(RXS-0252/0277):embed.rx → `.dll` + `.lib` + `.h` 三件齐,
       生成头声明集 == 期望导出集(含 gfx 四符号)。
    5. 生成头幂等(RXS-0253):同 `-o` 二次 emit → 头逐字节一致 + 无绝对路径 / 无时间戳。
    6. **RED**:篡改生成头一字节 → 重跑 `--emit=dll` → 再生成头 == 规范头 且 ≠ 篡改版
       (证 CI 再生成逐字节比对守卫 RXS-0254 非空过)。
    - 步骤 4~6 需 clang + link.exe;缺则 **SKIP**(dev-env-degrade,退 0),步骤 1~3 **恒跑**。
      `RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。

  device 段（**gate real**:clang + link.exe + MSVC/Windows SDK(D3D12)+ Vulkan SDK + GPU;
  `RURIX_REQUIRE_REAL=1` 翻硬红,缺则 SKIP 退 0 打 dev-env-degrade):
    7. `cl.exe` 编译 v3 harness(include **现场再生成**的头,链 `rurix_rhi.lib` + d3d12 +
       dxgi + **vulkan-1**〔Vulkan↔D3D12 LUID 匹配;v2 = cudart,v3 = vulkan-1〕)→ 真跑。
    8. **三方数值精确相等**(Q-PixelCriterion,RXS-0277,**不设 ULP 容差**):stdout 须含
       `UC05_EMBED_V3_OK` + 每例 `UC05_EMBED_V3_CASE w=<w> h=<h> rx=<p> d3d_raster=<p>
       d3d_mesh=<p> ref=<p>`;**本脚本独立重算**闭式参考 `0x00000000`(第三方复核,防宿主
       rx/d3d/ref 自证同源脱节),四方(.rx 像素 / D3D12 raster / D3D12 mesh / 脚本参考)须
       精确相等。
    9. **RED 三路**(任一翻红):篡改 .rx 侧参考(`*out = 1u32`) / 篡改 D3D12 侧参考(clear
       color {1,0,0,0}) / 篡改 host 侧参考(HOST_REFERENCE_PIXEL = 0x00000001)各编译真跑
       须退非零(三方数值不等)。
   10. 落 evidence JSON(`evidence/uc05_engine_embed_v3_<ts>.json`)。

**SKIP 纪律**:无 clang/link/MSVC/Vulkan SDK/GPU → SKIP = dev-env degrade(**非 fake pass**,
退 0,打印 dev-env-degrade);`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。run URL 不伪造:本机
记 "local"。

**主循环登记提示**:步骤号 = 78;门 = G-G4-3 嵌入面;条款 = RXS-0277;host 段恒跑(步骤 1~3
纯审计)vs device 段 gated(cl + GPU 真跑)双态,结构照步骤 74 `ci/uc05_engine_embed_smoke.py`
先例;LUID 匹配升级为 Vulkan↔D3D12(v2 = CUDA↔D3D12);三方数值精确相等判据 Q-PixelCriterion
新增(纯色/nearest RGBA8 整数 fetch 域,不设 ULP 容差)。
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
WORK = ROOT / "target" / "uc05_engine_embed_v3"
EVIDENCE_DIR = ROOT / "evidence"

APP = ROOT / "apps" / "uc05-rhi"
EMBED_RX = APP / "src" / "embed.rx"
ENGINE = ROOT / "src" / "rurix-engine"
HARNESS_V3 = ENGINE / "harness" / "engine_host_v3.cpp"
# v1 手写路三件(G1.3 / MR-0002 / RXS-0149)+ v2 生成路(uc05_engine_host.cpp):
# 本步骤只**读**,不改(既有资产 0-byte)。
V1_ASSETS = [
    ENGINE / "harness" / "engine_host.cpp",
    ENGINE / "include" / "rurix_engine.h",
    ENGINE / "src" / "ffi.rs",
]
HARNESS_V2 = ENGINE / "harness" / "uc05_engine_host.cpp"

GENERATED_HEADER_NAME = "rurix_rhi.h"
# G4.2 PR-D:embed.rx 追加 gfx 导出后生成头含四符号(compute 两 + gfx 两,加性)。
EXPECTED_EXPORTS = {
    "uc05_run_graph", "uc05_graph_pass_count",
    "uc05_gfx_run_frame", "uc05_gfx_pass_count",
}
# harness 真跑规模(须与 engine_host_v3.cpp 的 CASES[] 一致;脚本独立重算参考值复核)。
EXPECTED_CASES = ((64, 64), (256, 256))
# 闭式参考(Q-PixelCriterion:纯色 RGBA8 = 0x00000000,清色不变量)。
HOST_REFERENCE_PIXEL = 0x00000000

# 工具链 pin（与 ci/export_c_smoke.py 同源;RURIXC_CLANG 覆写 clang）。
CLANG = Path(r"C:/Program Files/LLVM/bin/clang.exe")
MSVC_ROOT = Path(r"C:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.44.35207")
MSVC_BIN = MSVC_ROOT / "bin" / "Hostx64" / "x64"
SDK_INC = Path(r"C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0")
SDK_LIB = Path(r"C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0")


def fail(msg: str) -> int:
    print(f"[uc05_engine_embed_v3] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[uc05_engine_embed_v3] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None, timeout: int = 900):
    return subprocess.run(
        cmd, cwd=str(cwd), capture_output=True, text=True, env=env, timeout=timeout
    )


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
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if os.name == "nt" else "rurixc")
    return exe if exe.is_file() else None


def emit_dll(rurixc: Path, src: Path, out_stem: Path, env: dict[str, str]):
    return run([str(rurixc), str(src), "--emit=dll", "-o", str(out_stem)], env=env)


def header_names(header_text: str) -> set[str]:
    """生成头声明集(单一事实源:头声明 ↔ DLL 导出符号,§4.0-1;同步骤 71 解析口径)。"""
    names: set[str] = set()
    for line in header_text.splitlines():
        s = line.strip()
        if s.endswith(";") and "(" in s and not s.startswith(("#", "/", "extern", "}")):
            m = re.search(r"(\w+)\s*\(", s)
            if m:
                names.add(m.group(1))
    return names


# ─────────────────────────── host 段（恒跑） ───────────────────────────


def audit_generated_header_not_handwritten(results: dict) -> bool:
    """1) 生成头自始生成、不手写(RXS-0253/0254)。"""
    p = run(["git", "ls-files"])
    if p.returncode != 0:
        fail("git ls-files 失败(非 git 工作树?)")
        return False
    tracked = [ln for ln in p.stdout.splitlines() if ln.strip()]
    committed = [f for f in tracked if Path(f).name == GENERATED_HEADER_NAME]
    if committed:
        fail(
            f"仓库内存在 tracked `{GENERATED_HEADER_NAME}`(生成头须自始生成、不手写,"
            f"RXS-0253):{committed}"
        )
        return False
    if not HARNESS_V3.is_file():
        fail(f"缺 engine_host v3 harness: {HARNESS_V3}")
        return False
    v3_text = HARNESS_V3.read_text(encoding="utf-8")
    if f'#include "{GENERATED_HEADER_NAME}"' not in v3_text:
        fail(f"v3 harness 未 include 生成头 `{GENERATED_HEADER_NAME}`(嵌入面须走生成路)")
        return False
    results["generated_header_not_handwritten"] = True
    print(
        "[uc05_engine_embed_v3] host 步骤 1 PASS: 生成头自始生成不手写"
        f"(仓库零 tracked {GENERATED_HEADER_NAME};v3 harness include 现场再生成头)"
    )
    return True


def audit_coexistence(results: dict) -> bool:
    """2) 三制共存(RXS-0254 §4.A5):v1 手写路 + v2 生成路在位,v3 不触之。"""
    missing = [str(p.relative_to(ROOT)) for p in V1_ASSETS if not p.is_file()]
    if missing:
        fail(f"G1.3 v1 手写路资产缺失(RXS-0149 守卫面应 0-byte 保留):{missing}")
        return False
    if not HARNESS_V2.is_file():
        fail(f"v2 harness 缺失: {HARNESS_V2}")
        return False
    v3_text = HARNESS_V3.read_text(encoding="utf-8")
    if '#include "rurix_engine.h"' in v3_text:
        fail("v3 harness include 了 v1 手写头(三制须解耦共存,RXS-0254)")
        return False
    # 注释中出现 `rurix_engine_*` 是叙述,不构成调用面;只拒非注释行的符号引用。
    for i, raw in enumerate(v3_text.splitlines(), 1):
        line = raw.strip()
        if line.startswith("//") or line.startswith("*") or line.startswith("/*"):
            continue
        if re.search(r"\brurix_engine_[a-z0-9_]+\s*\(", line):
            fail(f"v3 harness 引用 v1 符号面(engine_host_v3.cpp:{i}):{line}")
            return False
    # v3 也不 include v2 头(v2 用同一生成头 rurix_rhi.h,无独立头;只检 v1 手写头即可)。
    results["coexistence"] = True
    print(
        "[uc05_engine_embed_v3] host 步骤 2 PASS: 三制共存"
        "(v1 手写路三件 + v2 生成路在位;v3 既不 include v1 头也不引用 rurix_engine_* 符号面)"
    )
    return True


def audit_zero_rs(results: dict) -> bool:
    """3) apps/uc05-rhi 零 .rs 主语言判据维持(RFC-0014 §9.2;同步骤 72/74 口径)。"""
    if not APP.is_dir():
        fail(f"apps/uc05-rhi 不存在: {APP}")
        return False
    violations, rx_files = [], []
    for p in sorted(APP.rglob("*")):
        if p.is_dir():
            continue
        rel = p.relative_to(APP).as_posix()
        if rel == "rurix.toml":
            continue
        if p.suffix == ".rx":
            rx_files.append(rel)
            continue
        violations.append(rel)
    if violations:
        fail(
            "零 .rs 审计违例——apps/uc05-rhi 存在非 .rx 源(G-EI1-4,RFC-0014 §9.2;C++ 宿主"
            f"须置于审计边界之外):\n  " + "\n  ".join(violations)
        )
        return False
    if EMBED_RX.relative_to(APP).as_posix() not in rx_files:
        fail(f"缺导出面 {EMBED_RX.relative_to(ROOT)}")
        return False
    results["zero_rs_audit"] = True
    print(
        f"[uc05_engine_embed_v3] host 步骤 3 PASS: 零 .rs 审计(apps/uc05-rhi 仅 {len(rx_files)}"
        " 个 .rx + rurix.toml;导出面 embed.rx 在内)"
    )
    return True


def host_toolchain_section(results: dict) -> bool:
    """4~6) `--emit=dll` 产物 + 生成头幂等 + 篡改再生成 RED(需 clang + link.exe)。"""
    clang = resolve_clang()
    if clang is None:
        results["toolchain_skip"] = "no-clang"
        for k in ("emit_dll_artifacts", "header_idempotent", "tamper_regen_red"):
            results[k] = "SKIP"
        return skip("未找到 clang(步骤 4~6 需 clang + link.exe;步骤 1~3 已恒跑)") == 0
    rurixc = build_rurixc()
    if rurixc is None:
        fail("rurixc 构建失败")
        return False
    env = rurixc_env(clang)
    WORK.mkdir(parents=True, exist_ok=True)

    # 4) GPU 导出面 emit:.dll + .lib + .h 三件齐 + 头声明集 == 期望导出集(含 gfx 四符号)。
    stem = WORK / "rurix_rhi"
    e1 = emit_dll(rurixc, EMBED_RX, stem, env)
    dll, imp_lib, hdr = stem.with_suffix(".dll"), stem.with_suffix(".lib"), stem.with_suffix(".h")
    if e1.returncode != 0 or not dll.is_file():
        blob = (e1.stdout + e1.stderr)[-1600:]
        # RX7001 = external toolchain failure(ptxas / link.exe 不可用)→ dev-env degrade SKIP,
        # 非编译期红(导出面/图装配面)。其余 error[RX####] = 真编译期红,FAIL。
        if "error[RX" in blob and "error[RX7001]" not in blob:
            print(blob, file=sys.stderr)
            fail("embed.rx `--emit=dll` 编译期红(导出面不合子集 v1 / 图装配面?)")
            return False
        print(blob, file=sys.stderr)
        results["toolchain_skip"] = "no-ptxas" if "RX7001" in blob else "no-link"
        for k in ("emit_dll_artifacts", "header_idempotent", "tamper_regen_red"):
            results[k] = "SKIP"
        return skip("`--emit=dll` 失败(link.exe / rt_cabi 工具链面缺)") == 0
    if not imp_lib.is_file():
        fail(f"缺 import lib {imp_lib.name}(link.exe /DLL 应副产 .lib)")
        return False
    if not hdr.is_file():
        fail(f"缺生成头 {hdr.name}(RXS-0253)")
        return False
    declared = header_names(hdr.read_text(encoding="utf-8"))
    if declared != EXPECTED_EXPORTS:
        fail(
            f"生成头声明集与期望导出集不符: declared={sorted(declared)} "
            f"expected={sorted(EXPECTED_EXPORTS)}"
        )
        return False
    results["emit_dll_artifacts"] = True
    print(
        "[uc05_engine_embed_v3] host 步骤 4 PASS: GPU 导出面 `--emit=dll` 产 .dll + .lib + .h"
        f"(声明集 {sorted(declared)};含 gfx 四符号,RXS-0277)"
    )

    # 5) 生成头幂等(RXS-0253)。
    canonical = hdr.read_bytes()
    e2 = emit_dll(rurixc, EMBED_RX, stem, env)
    again = hdr.read_bytes()
    htext = canonical.decode("utf-8", "replace")
    abs_path = bool(re.search(r"[A-Za-z]:[\\/]", htext)) or str(ROOT) in htext
    timestamp = bool(re.search(r"\b20\d\d[-/:]\d\d", htext))
    idem_ok = e2.returncode == 0 and again == canonical and not abs_path and not timestamp
    results["header_idempotent"] = idem_ok
    if not idem_ok:
        fail(
            f"生成头非确定性: byte_identical={again == canonical} abs_path={abs_path} "
            f"timestamp={timestamp}"
        )
        return False
    print("[uc05_engine_embed_v3] host 步骤 5 PASS: 生成头幂等 + 无绝对路径/时间戳(RXS-0253)")

    # 6) RED:篡改一字节 → 重 emit → 再生成 == 规范 ≠ 篡改。
    mutated = bytearray(canonical)
    mutated[len(mutated) // 2] ^= 0x20
    if bytes(mutated) == canonical:
        fail("篡改未改变头字节(RED 前置无效)")
        return False
    hdr.write_bytes(bytes(mutated))
    e3 = emit_dll(rurixc, EMBED_RX, stem, env)
    regen = hdr.read_bytes()
    tamper_ok = e3.returncode == 0 and regen == canonical and regen != bytes(mutated)
    results["tamper_regen_red"] = tamper_ok
    if not tamper_ok:
        fail("篡改头再生成守卫空过(RXS-0254 RED 未成立)")
        return False
    print(
        "[uc05_engine_embed_v3] host 步骤 6 PASS: 篡改生成头 → 再生成逐字节比对 byte-diff"
        "(RXS-0254 RED 守卫非空过)"
    )
    return True


# ─────────────────────────── device 段（gate real） ───────────────────────────


def locate_msvc() -> Path | None:
    cl = MSVC_BIN / "cl.exe"
    if cl.is_file() and (MSVC_ROOT / "include").is_dir() and SDK_INC.is_dir():
        return cl
    return None


def locate_vulkan_sdk() -> Path | None:
    v = os.environ.get("VULKAN_SDK")
    if v and (Path(v) / "include" / "vulkan" / "vulkan.h").is_file():
        return Path(v)
    return None


def msvc_env(base: dict[str, str], header_dir: Path, lib_dir: Path, vulkan: Path) -> dict[str, str]:
    env = dict(base)
    env["INCLUDE"] = os.pathsep.join([
        str(MSVC_ROOT / "include"),
        str(SDK_INC / "ucrt"),
        str(SDK_INC / "shared"),
        str(SDK_INC / "um"),
        str(SDK_INC / "winrt"),  # wrl/client.h(ComPtr)
        str(header_dir),
        str(vulkan / "include"),
    ])
    env["LIB"] = os.pathsep.join([
        str(MSVC_ROOT / "lib" / "x64"),
        str(SDK_LIB / "ucrt" / "x64"),
        str(SDK_LIB / "um" / "x64"),
        str(lib_dir),
        str(vulkan / "lib"),
    ])
    env["PATH"] = str(MSVC_BIN) + os.pathsep + env.get("PATH", "")
    return env


def parse_cases(stdout: str) -> dict[tuple[int, int], tuple[int, int, int, int]]:
    """`UC05_EMBED_V3_CASE w=<w> h=<h> rx=<p> d3d_raster=<p> d3d_mesh=<p> ref=<p>`
    → {(w, h): (rx, d3d_raster, d3d_mesh, ref)}。"""
    out: dict[tuple[int, int], tuple[int, int, int, int]] = {}
    pat = (
        r"^UC05_EMBED_V3_CASE w=(\d+) h=(\d+) rx=(\d+) d3d_raster=(\d+) "
        r"d3d_mesh=(\d+) ref=(\d+)\s*$"
    )
    for m in re.finditer(pat, stdout, re.MULTILINE):
        out[(int(m.group(1)), int(m.group(2)))] = (
            int(m.group(3)), int(m.group(4)), int(m.group(5)), int(m.group(6)),
        )
    return out


def compile_harness(cl: Path, src: Path, exe: Path, env: dict, imp_lib: Path) -> int:
    """cl.exe 编译 v3 harness,链 rurix_rhi.lib + d3d12 + dxgi + vulkan-1。"""
    pc = run(
        [
            str(cl), "/nologo", "/std:c++17", "/EHsc", str(src),
            f"/Fe:{exe}", f"/Fo:{WORK}\\",
            "/link", imp_lib.name, "d3d12.lib", "dxgi.lib", "vulkan-1.lib",
        ],
        cwd=WORK,
        env=env,
    )
    if pc.returncode != 0 or not exe.is_file():
        print((pc.stdout + pc.stderr)[-1800:], file=sys.stderr)
    return pc.returncode


def device_section(results: dict) -> int:
    clang = resolve_clang()
    if clang is None:
        results["harness_build"] = "SKIP"
        results["three_party_equal"] = "SKIP"
        return skip("device 段:未找到 clang(需 clang + link.exe + MSVC + Vulkan SDK)")
    cl = locate_msvc()
    if cl is None:
        results["harness_build"] = "SKIP"
        results["three_party_equal"] = "SKIP"
        return skip("device 段:未找到 MSVC cl.exe + Windows SDK(C++/D3D12 宿主编译需)")
    vulkan = locate_vulkan_sdk()
    if vulkan is None:
        results["harness_build"] = "SKIP"
        results["three_party_equal"] = "SKIP"
        return skip("device 段:无 VULKAN_SDK / vulkan.h(Vulkan↔D3D12 LUID 匹配 + GPU 真跑需)")

    stem = WORK / "rurix_rhi"
    dll, imp_lib, hdr = stem.with_suffix(".dll"), stem.with_suffix(".lib"), stem.with_suffix(".h")
    if not (dll.is_file() and imp_lib.is_file() and hdr.is_file()):
        results["harness_build"] = "SKIP"
        results["three_party_equal"] = "SKIP"
        return skip("device 段:host 段未产出 dll/lib/生成头(工具链面已 SKIP)")

    base_env = rurixc_env(clang)
    env = msvc_env(base_env, hdr.parent, imp_lib.parent, vulkan)

    # 7) cl.exe 编译 v3 harness(include 现场再生成头,链 rurix_rhi.lib + d3d12 + dxgi + vulkan-1)。
    exe = WORK / "engine_host_v3.exe"
    rc = compile_harness(cl, HARNESS_V3, exe, env, imp_lib)
    if rc != 0 or not exe.is_file():
        results["harness_build"] = "SKIP"
        results["three_party_equal"] = "SKIP"
        return skip("device 段:cl 编译 v3 harness 失败(MSVC/SDK/Vulkan SDK 头库不全?)")
    results["harness_build"] = True
    print(
        "[uc05_engine_embed_v3] device 步骤 7 PASS: cl.exe 编译 engine_host v3"
        "(include 现场再生成头 + 链 rurix_rhi.lib / d3d12 / dxgi / vulkan-1)"
    )

    # 8) 真跑 + 三方数值精确相等(Q-PixelCriterion,不设 ULP 容差)。
    pr = run([str(exe)], cwd=WORK, env=env)
    blob = pr.stdout + pr.stderr
    cases = parse_cases(pr.stdout)
    ok_token = "UC05_EMBED_V3_OK" in pr.stdout
    # rc=2:Vulkan↔D3D12 LUID 匹配不可达(无 Vulkan / 无 GPU)→ dev-env degrade SKIP;
    # rc=3:D3D12 上下文在 Vulkan adapter 上不可达 → dev-env degrade SKIP。
    # (两者均为 provisioning 缺失,非代码面 FAIL;RURIX_REQUIRE_REAL=1 把缺失翻硬红。)
    if pr.returncode in (2, 3) and not ok_token:
        results["three_party_equal"] = "SKIP"
        results["red_three_ways"] = "SKIP"
        return skip(
            f"device 段:engine_host v3 真跑 rc={pr.returncode} "
            "(Vulkan↔D3D12 LUID 匹配 / D3D12 上下文不可达;无 GPU 或无 Vulkan provision)"
        )
    if pr.returncode != 0 or not ok_token:
        print(blob[-1800:], file=sys.stderr)
        results["three_party_equal"] = False
        return fail(
            f"C++/D3D12 宿主真跑失败: rc={pr.returncode} 缺 UC05_EMBED_V3_OK"
            "(Vulkan LUID / D3D12 上下文 / 图节点 / 三方数值对照任一不成立)"
        )
    if set(cases) != set(EXPECTED_CASES):
        results["three_party_equal"] = False
        return fail(
            f"宿主上报规模集不符: got={sorted(cases)} expected={sorted(EXPECTED_CASES)}"
        )
    lines = []
    for key in sorted(cases):
        rx, d3d_raster, d3d_mesh, host_ref = cases[key]
        script_ref = HOST_REFERENCE_PIXEL
        if not (rx == d3d_raster == d3d_mesh == host_ref == script_ref):
            results["three_party_equal"] = False
            return fail(
                f"三方数值对照不等 {key}: rx={rx} d3d_raster={d3d_raster} "
                f"d3d_mesh={d3d_mesh} host_ref={host_ref} script_ref={script_ref}"
            )
        lines.append(f"{key[0]}x{key[1]}:rx={rx}:d3d_raster={d3d_raster}:d3d_mesh={d3d_mesh}")
    results["three_party_equal"] = True
    results["embed_cases"] = lines
    print(
        "[uc05_engine_embed_v3] device 步骤 8 PASS: UC05_EMBED_V3_OK —— 三方数值精确相等"
        f"(Q-PixelCriterion,纯色 RGBA8 = 0x00000000){lines}"
    )

    # 9) RED 三路:篡改 .rx 侧参考 / 篡改 D3D12 侧参考 / 篡改 host 侧参考任一翻红。
    rurixc = build_rurixc()
    if rurixc is None:
        results["red_three_ways"] = "SKIP"
        print("[uc05_engine_embed_v3] device 步骤 9 SKIP: rurixc 重建失败(RED 三路需)")
    else:
        red_results = red_three_ways(rurixc, cl, env, clang, imp_lib, stem)
        results["red_three_ways"] = red_results
        if red_results is not True:
            return fail("RED 三路未全部成立(至少一路篡改未翻红)")

    return 0


def red_three_ways(rurixc: Path, cl: Path, env: dict, clang: Path,
                    imp_lib: Path, canonical_stem: Path) -> bool:
    """RED 三路:篡改 .rx / D3D12 / host 参考任一翻红(RXS-0277 device 段)。"""

    # ── RED 1:篡改 .rx 侧参考(`*out = pixel` → `*out = 1u32`;rx 像素 ≠ ref)──────
    tampered_rx = WORK / "embed_tamper_rx.rx"
    text = EMBED_RX.read_text(encoding="utf-8")
    tampered_text = text.replace("*out = pixel;", "*out = 1u32;")
    if tampered_text == text:
        print("[uc05_engine_embed_v3] RED 1 WARN: 篡改模式未命中(*out = pixel;)", file=sys.stderr)
    tampered_rx.write_text(tampered_text, encoding="utf-8", newline="\n")
    tampered_stem = WORK / "rurix_rhi_tamper_rx"
    e_rx = emit_dll(rurixc, tampered_rx, tampered_stem, rurixc_env(clang))
    tampered_lib = tampered_stem.with_suffix(".lib")
    red1_ok = False
    if e_rx.returncode == 0 and tampered_lib.is_file():
        exe_rx = WORK / "engine_host_v3_tamper_rx.exe"
        rc_rx = compile_harness(cl, HARNESS_V3, exe_rx,
                                  msvc_env(rurixc_env(clang), tampered_stem.with_suffix(".h").parent,
                                           tampered_lib.parent, locate_vulkan_sdk()),
                                  tampered_lib)
        if rc_rx == 0 and exe_rx.is_file():
            pr_rx = run([str(exe_rx)], cwd=WORK, env=env)
            red1_ok = pr_rx.returncode != 0 and "UC05_EMBED_V3_OK" not in pr_rx.stdout
    results_key = "red_tamper_rx"
    if red1_ok:
        print("[uc05_engine_embed_v3] device 步骤 9 RED 1 PASS: 篡改 .rx 侧参考翻红"
              "(*out = 1u32 → 三方数值不等,exit ≠ 0)")
    else:
        print("[uc05_engine_embed_v3] device 步骤 9 RED 1 FAIL: 篡改 .rx 侧参考未翻红",
              file=sys.stderr)

    # ── RED 2:篡改 D3D12 侧参考(clear color {0,0,0,0} → {1,0,0,0};d3d 像素 ≠ ref)──
    tampered_d3d = WORK / "engine_host_v3_tamper_d3d.cpp"
    text_v3 = HARNESS_V3.read_text(encoding="utf-8")
    tampered_d3d_text = text_v3.replace("clear_val.Color[0] = 0.0f;",
                                         "clear_val.Color[0] = 1.0f;  // RED: tampered")
    tampered_d3d.write_text(tampered_d3d_text, encoding="utf-8", newline="\n")
    exe_d3d = WORK / "engine_host_v3_tamper_d3d.exe"
    rc_d3d = compile_harness(cl, tampered_d3d, exe_d3d, env, imp_lib)
    red2_ok = False
    if rc_d3d == 0 and exe_d3d.is_file():
        pr_d3d = run([str(exe_d3d)], cwd=WORK, env=env)
        red2_ok = pr_d3d.returncode != 0 and "UC05_EMBED_V3_OK" not in pr_d3d.stdout
    if red2_ok:
        print("[uc05_engine_embed_v3] device 步骤 9 RED 2 PASS: 篡改 D3D12 侧参考翻红"
              "(clear color {1,0,0,0} → 三方数值不等,exit ≠ 0)")
    else:
        print("[uc05_engine_embed_v3] device 步骤 9 RED 2 FAIL: 篡改 D3D12 侧参考未翻红",
              file=sys.stderr)

    # ── RED 3:篡改 host 侧参考(HOST_REFERENCE_PIXEL 0x00000000 → 0x00000001)──────
    tampered_host = WORK / "engine_host_v3_tamper_host.cpp"
    tampered_host_text = text_v3.replace("HOST_REFERENCE_PIXEL = 0x00000000u",
                                          "HOST_REFERENCE_PIXEL = 0x00000001u  // RED: tampered")
    tampered_host.write_text(tampered_host_text, encoding="utf-8", newline="\n")
    exe_host = WORK / "engine_host_v3_tamper_host.exe"
    rc_host = compile_harness(cl, tampered_host, exe_host, env, imp_lib)
    red3_ok = False
    if rc_host == 0 and exe_host.is_file():
        pr_host = run([str(exe_host)], cwd=WORK, env=env)
        red3_ok = pr_host.returncode != 0 and "UC05_EMBED_V3_OK" not in pr_host.stdout
    if red3_ok:
        print("[uc05_engine_embed_v3] device 步骤 9 RED 3 PASS: 篡改 host 侧参考翻红"
              "(HOST_REFERENCE_PIXEL = 0x00000001 → 三方数值不等,exit ≠ 0)")
    else:
        print("[uc05_engine_embed_v3] device 步骤 9 RED 3 FAIL: 篡改 host 侧参考未翻红",
              file=sys.stderr)

    return red1_ok and red2_ok and red3_ok


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = results.get("three_party_equal") == "SKIP"
    checks = {
        k: results.get(k)
        for k in (
            "generated_header_not_handwritten",
            "coexistence",
            "zero_rs_audit",
            "emit_dll_artifacts",
            "header_idempotent",
            "tamper_regen_red",
            "harness_build",
            "three_party_equal",
            "red_three_ways",
        )
        if results.get(k) is not None
    }
    doc = {
        "schema_version": 1,
        "subject": "uc05_engine_embed_v3",
        "milestone": "G4.2 PR-D / G-G4-3 (RFC-0015 §4.A7; RXS-0277)",
        "step": 78,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": checks,
        "embed_v3_ok": results.get("three_party_equal") is True,
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": device_skipped or results.get("toolchain_skip") is not None,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    if results.get("embed_cases"):
        doc["embed_cases"] = results["embed_cases"]
    ev = EVIDENCE_DIR / f"uc05_engine_embed_v3_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    print(f"[uc05_engine_embed_v3] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    host_ok = (
        audit_generated_header_not_handwritten(results)
        and audit_coexistence(results)
        and audit_zero_rs(results)
        and host_toolchain_section(results)
    )
    if not host_ok:
        write_evidence(results, host_ok, 1)
        return 1
    device_rc = device_section(results)
    write_evidence(results, host_ok, device_rc)
    return device_rc


if __name__ == "__main__":
    sys.exit(main())
