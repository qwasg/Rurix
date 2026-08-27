#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C1 渲染器 SDK 稳定 API 面）
"""G31+ 波 C Task C1:渲染器 SDK 稳定 API 面门冒烟(g31.waveC.sdk;
G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #48「渲染器 SDK 稳定 API 面」兑现面)。

架构(两层 DLL,export_c codegen 复用——RD-009 closed 机制第三消费方):
  `apps/g31-renderer-sdk/src/sdk.rx`(#[export(c)] 薄转发)→ `rurixc --emit=dll`
  → `rurix_renderer.dll` + import lib + **编译器生成头**(RXS-0253 单一事实源,
  自始生成不手写)→ 经 `#[link(name = "rurix_renderer_sdk")]` 绑定实现层
  `src/rurix-renderer-sdk` cdylib 的 `rxsdk_*` 会话面(u64 不透明句柄表,薄封装
  G14.3 统一四 pass TSR 生产车道)→ 外部 C++ 控制台宿主
  `apps/g31-renderer-sdk/host/renderer_sdk_host.cpp` 链接真跑。

判据闭集(milestones/g31/g31_renderer_sdk_evidence_schema.json 描述段逐字):
1. header_generated_and_idempotent:`--emit=dll` 三件齐 + 生成头声明集 == 期望
   导出集(9 符号)+ 幂等(同 -o 二次 emit 逐字节一致)+ 篡改一字节再生成 RED。
2. api_version_semver_policy:宿主见 ABI=0x00010000(1.0.0)+ 政策文件在树且含
   MAJOR/MINOR/PATCH 政策字面(破坏性变更走 RFC,同 MAJOR 只增)。
3. host_integration_ok:外部宿主全链真跑绿(RXSDK_HOST_OK + 七 token 齐)。
4. digest_matches_stage_a_anchor:canonical 160 帧 warmup 10 末帧 digest ==
   Stage A 锚 bistro-interior_t100_tsr_device(程序读锚文件,位级对拍生产管线)。
5. frame_time_measured:post-warmup 帧时 mean/p50 > 0 且样本 n == 160(真实数字)。
6. stable_snapshot_extended:stable_snapshot --check PASS + 快照 renderer_sdk_api
   段 9 导出 + abi_version=1.0.0(RD-008 机制渲染器面延伸,处置见 deferred.json)。
7. rd036_subset_v1_compliant:sdk.rx 导出签名类型机核全落 subset v1 闭集(标量 +
   *mut/*const 标量 + unit)——RD-036 超界四项逐项不触,判档不成立维持 open。
8. uc05_export_c_regression_green:ci/export_c_smoke.py(RURIX_REQUIRE_REAL=1)
   复跑 exit 0(EI1/UC-05 既有面回归不破坏)。

三态:无 clang/link/MSVC/Vulkan/GPU/资产 → DEV_ENV_DEGRADE 退 0(不冒充 PASS);
RURIX_REQUIRE_REAL=1 下降级翻硬 FAIL(禁 mock 充真跑)。

evidence 纪律:PASS 才落 evidence/g31_renderer_sdk_<ts>.json(check_schemas
前缀路由 g31_renderer_sdk_);FAIL 诊断件落 .tmp/g31_gates/renderer_sdk/
工作区不污染 evidence/ 路由面(fail-closed:evidence/ 无件 = 门未过)。

用法:
  py -3 ci/g31_renderer_sdk_smoke.py --selftest
  py -3 ci/g31_renderer_sdk_smoke.py --gate g31.waveC.sdk
"""
from __future__ import annotations

import argparse
import datetime as _dt
import io
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveC.sdk"
SUBJECT = "g31_renderer_sdk"
WAVE = "G31+.C"
TAG = "g31_renderer_sdk"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_renderer_sdk_evidence_schema.json"
SCHEMA_ID = "rurix.g31.renderer_sdk_evidence.v1"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
SDK_RX = ROOT / "apps" / "g31-renderer-sdk" / "src" / "sdk.rx"
HOST_CPP = ROOT / "apps" / "g31-renderer-sdk" / "host" / "renderer_sdk_host.cpp"
VERSIONING_MD = ROOT / "apps" / "g31-renderer-sdk" / "API_VERSIONING.md"
CONTRACT = ROOT / "milestones" / "g13" / "g13_ue_upscale_parity_contract.json"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
SNAPSHOT = ROOT / "tests" / "stable" / "stable_api.snapshot"
DEFERRED = ROOT / "registry" / "deferred.json"
WORK = ROOT / ".tmp" / "g31_gates" / "renderer_sdk"
SPV_DIR = WORK / "spv"

SCENE, TIER, FRAMES, WARMUP = "bistro-interior", 100, 160, 10
ABI_PACKED = "0x00010000"
SEMVER = "1.0.0"
KERNELS = {
    "g14_3_direct_gi.spv": ROOT / "src" / "rurix-render" / "kernels" / "g14_3_direct_gi.rx",
    "g14_mv.spv": ROOT / "src" / "rurix-render" / "kernels" / "g14_mv.rx",
    "g14_8_tsr_resample.spv": ROOT / "src" / "rurix-render" / "kernels" / "g14_8_tsr_resample.rx",
    "g14_8_tsr_resolve.spv": ROOT / "src" / "rurix-render" / "kernels" / "g14_8_tsr_resolve.rx",
}
EXPECTED_EXPORTS = {
    "rurix_renderer_abi_version",
    "rurix_renderer_caps_probe",
    "rurix_renderer_create",
    "rurix_renderer_destroy",
    "rurix_renderer_load_scene",
    "rurix_renderer_set_camera",
    "rurix_renderer_set_exposure_ev100",
    "rurix_renderer_render_frame",
    "rurix_renderer_present",
}
GENERATED_HEADER_NAME = "rurix_renderer.h"

# 工具链 pin(与 ci/uc05_engine_embed_v3_smoke.py 同源;RURIXC_CLANG 覆写 clang)。
CLANG = Path(r"C:/Program Files/LLVM/bin/clang.exe")
MSVC_ROOT = Path(r"C:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.44.35207")
MSVC_BIN = MSVC_ROOT / "bin" / "Hostx64" / "x64"
SDK_INC = Path(r"C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0")
SDK_LIB = Path(r"C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0")

FACT_IDS = [
    "header_generated_and_idempotent",
    "api_version_semver_policy",
    "host_integration_ok",
    "digest_matches_stage_a_anchor",
    "frame_time_measured",
    "stable_snapshot_extended",
    "rd036_subset_v1_compliant",
    "uc05_export_c_regression_green",
]

# subset v1 类型闭集(RXS-0251:标量 + *mut/*const 标量 + unit 返回)。
_SCALAR = {"i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "bool"}
EXPORT_SIG_RE = re.compile(
    r"#\[export\(c\)\]\s*pub fn\s+(\w+)\s*\((.*?)\)\s*(?:->\s*([^\{]+?))?\s*\{",
    re.DOTALL,
)
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None,
        cwd: Path = ROOT) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=str(cwd), capture_output=True, text=True,
                          timeout=timeout, env=env)


def base_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面;全纯函数无 GPU 依赖)
# ---------------------------------------------------------------------------


def type_in_subset_v1(ty: str) -> bool:
    """export_c subset v1 类型谓词(RXS-0251):标量 | *mut 标量 | *const 标量;
    返回位允许空(unit)。struct 按值/回调/数组按值/字符串按值 → False。"""
    t = ty.strip()
    if t in _SCALAR:
        return True
    for kw in ("*mut ", "*const "):
        if t.startswith(kw) and t[len(kw):].strip() in _SCALAR:
            return True
    return False


def check_sdk_rx_subset_v1(text: str) -> tuple[bool, list[str]]:
    """sdk.rx 导出签名面机核:全部参数/返回类型落 subset v1 闭集(RD-036 判档面)。
    返回 (全合规?, 违规描述列)。"""
    bad: list[str] = []
    names: list[str] = []
    for m in EXPORT_SIG_RE.finditer(text):
        name, params, ret = m.group(1), m.group(2), m.group(3)
        names.append(name)
        for p in params.split(","):
            p = p.strip()
            if not p:
                continue
            if ":" not in p:
                bad.append(f"{name}: 参数失类型标注 {p!r}")
                continue
            if not type_in_subset_v1(p.split(":", 1)[1]):
                bad.append(f"{name}: 参数类型越 subset v1 {p.split(':', 1)[1].strip()!r}")
        if ret is not None and not type_in_subset_v1(ret):
            bad.append(f"{name}: 返回类型越 subset v1 {ret.strip()!r}")
    if not names:
        bad.append("未解析到任何 #[export(c)] 导出")
    return (not bad, bad)


def parse_host_tokens(out: str) -> dict:
    """宿主 stdout token 解析(缺 token 的键缺席——判据侧拒判)。"""
    d: dict = {}
    m = re.search(r"^RXSDK_HOST_ABI=(0x[0-9a-f]{8})\s*$", out, re.MULTILINE)
    if m:
        d["abi"] = m.group(1)
    m = re.search(r"^RXSDK_HOST_CAPS=(\d+)\s*$", out, re.MULTILINE)
    if m:
        d["caps"] = int(m.group(1))
    m = re.search(
        r"^RXSDK_HOST_LOAD_OK tier=(\d+) frames=(\d+) warmup=(\d+)\s*$", out, re.MULTILINE
    )
    if m:
        d["tier"] = int(m.group(1))
        d["frames"] = int(m.group(2))
        d["warmup"] = int(m.group(3))
    m = re.search(
        r"^RXSDK_HOST_FRAME mean=([0-9.]+) p50=([0-9.]+) n=(\d+)\s*$", out, re.MULTILINE
    )
    if m:
        d["frame_ms_mean"] = float(m.group(1))
        d["frame_ms_p50"] = float(m.group(2))
        d["frame_samples"] = int(m.group(3))
    m = re.search(r"^RXSDK_HOST_DIGEST (sha256:[0-9a-f]{64})\s*$", out, re.MULTILINE)
    if m:
        d["digest"] = m.group(1)
    d["params_ok"] = "RXSDK_HOST_PARAMS_OK" in out
    d["present_ok"] = "RXSDK_HOST_PRESENT_OK" in out
    d["host_ok"] = "RXSDK_HOST_OK" in out
    return d


def digest_matches(fresh: str | None, anchor: str | None) -> bool:
    return (
        isinstance(fresh, str)
        and isinstance(anchor, str)
        and DIGEST_RE.match(fresh) is not None
        and fresh == anchor
    )


def frame_time_ok(tokens: dict, expect_n: int = FRAMES) -> bool:
    return (
        tokens.get("frame_ms_mean", 0.0) > 0.0
        and tokens.get("frame_ms_p50", 0.0) > 0.0
        and tokens.get("frame_samples") == expect_n
    )


def host_integration_ok(tokens: dict) -> bool:
    return bool(
        tokens.get("host_ok")
        and tokens.get("abi") == ABI_PACKED
        and isinstance(tokens.get("caps"), int)
        and tokens.get("tier") == TIER
        and tokens.get("frames") == FRAMES
        and tokens.get("warmup") == WARMUP
        and tokens.get("params_ok")
        and tokens.get("present_ok")
    )


def versioning_policy_ok(text: str) -> bool:
    """政策文件字面闭集:语义化版本三档 + 破坏性变更走 RFC + 版本字面。"""
    return all(k in text for k in ("MAJOR", "MINOR", "PATCH", "1.0.0", "破坏性变更", "RFC"))


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决:无降级 → None(续跑);有降级 + REQUIRE_REAL → 1(硬红);
    有降级无 REQUIRE_REAL → 0(SKIP 非 PASS 非 FAIL)。"""
    if not degrade:
        return None
    return 1 if require_real else 0


def header_names(header_text: str) -> set[str]:
    """生成头声明集(同 ci/uc05_engine_embed_v3_smoke.py 解析口径)。"""
    names: set[str] = set()
    for line in header_text.splitlines():
        s = line.strip()
        if s.endswith(";") and "(" in s and not s.startswith(("#", "/", "extern", "}")):
            m = re.search(r"(\w+)\s*\(", s)
            if m:
                names.add(m.group(1))
    return names


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def resolve_clang() -> Path | None:
    v = os.environ.get("RURIXC_CLANG")
    if v and Path(v).is_file():
        return Path(v)
    if CLANG.is_file():
        return CLANG
    from shutil import which

    w = which("clang")
    return Path(w) if w else None


def locate_msvc() -> Path | None:
    cl = MSVC_BIN / "cl.exe"
    if cl.is_file() and (MSVC_ROOT / "include").is_dir() and SDK_INC.is_dir():
        return cl
    return None


def msvc_env(base: dict[str, str], header_dir: Path, lib_dirs: list[Path]) -> dict[str, str]:
    env = dict(base)
    env["INCLUDE"] = os.pathsep.join([
        str(MSVC_ROOT / "include"),
        str(SDK_INC / "ucrt"),
        str(SDK_INC / "shared"),
        str(SDK_INC / "um"),
        str(SDK_INC / "winrt"),
        str(header_dir),
    ])
    env["LIB"] = os.pathsep.join(
        [
            str(MSVC_ROOT / "lib" / "x64"),
            str(SDK_LIB / "ucrt" / "x64"),
            str(SDK_LIB / "um" / "x64"),
        ]
        + [str(p) for p in lib_dirs]
    )
    env["PATH"] = str(MSVC_BIN) + os.pathsep + env.get("PATH", "")
    return env


def build_rurixc() -> Path | None:
    # vulkan-backend feature = SPV 编译面（--target vulkan 硬前置,RX6026;
    # 与 ci/g12_pt_prod_lib.py 等同源）；--emit=dll 面同二进制消费不受影响。
    p = run(["cargo", "build", "-q", "-p", "rurixc", "--features", "vulkan-backend",
             "--bin", "rurixc"], timeout=3600)
    if p.returncode != 0:
        print((p.stdout + p.stderr)[-1600:], file=sys.stderr)
        return None
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if os.name == "nt" else "rurixc")
    return exe if exe.is_file() else None


def emit_dll(rurixc: Path, src: Path, out_stem: Path, env: dict[str, str]):
    return run([str(rurixc), str(src), "--emit=dll", "-o", str(out_stem)], env=env)


def run_gate() -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not SCHEMA_PATH.is_file():
        fail(f"门 schema 缺失: {SCHEMA_PATH}")
        return 1

    # ── host 段①(恒跑):生成头不手写审计 + 政策文件 + RD-036 subset 机核 ──
    p = run(["git", "ls-files"])
    tracked = [ln for ln in p.stdout.splitlines() if ln.strip()] if p.returncode == 0 else []
    committed_hdr = [f for f in tracked if Path(f).name == GENERATED_HEADER_NAME]
    if committed_hdr:
        fail(f"仓库内存在 tracked `{GENERATED_HEADER_NAME}`(生成头须自始生成不手写,RXS-0253):{committed_hdr}")

    policy_text = VERSIONING_MD.read_text(encoding="utf-8") if VERSIONING_MD.is_file() else ""
    set_fact(
        "api_version_semver_policy",
        bool(policy_text) and versioning_policy_ok(policy_text),
        f"API_VERSIONING.md {'在树且含 MAJOR/MINOR/PATCH/1.0.0/破坏性变更走 RFC 字面' if versioning_policy_ok(policy_text) else '缺失或政策字面不全'}"
        f";abi 字面 {ABI_PACKED}(semver {SEMVER})",
    )

    sdk_text = SDK_RX.read_text(encoding="utf-8") if SDK_RX.is_file() else ""
    subset_ok, subset_bad = check_sdk_rx_subset_v1(sdk_text)
    set_fact(
        "rd036_subset_v1_compliant",
        bool(sdk_text) and subset_ok,
        "sdk.rx 9 导出签名全落 subset v1 闭集(标量+*mut/*const 标量+unit)——RD-036 超界四项逐项不触,"
        "判档不成立维持 open(deferred.json 2026-08-25 行)"
        if subset_ok else f"subset v1 违例: {'; '.join(subset_bad[:4])}",
    )

    # ── dev-env 前置面(缺 → DEV_ENV_DEGRADE 登记,不冒充 FAIL 也不 PASS)──
    degrade: list[str] = []
    clang = resolve_clang()
    if clang is None:
        degrade.append("未找到 clang(--emit=dll obj 通道需)")
    if locate_msvc() is None:
        degrade.append("未找到 MSVC cl.exe + Windows SDK(C++ 宿主编译需)")
    if not CONTRACT.is_file():
        degrade.append(f"bistro 生产契约缺失 {CONTRACT}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")
    if not ANCHOR_PATH.is_file():
        degrade.append(f"Stage A 锚缺失 {ANCHOR_PATH}")
    if not SDK_RX.is_file():
        degrade.append(f"sdk.rx 缺失 {SDK_RX}")
    if not HOST_CPP.is_file():
        degrade.append(f"宿主源缺失 {HOST_CPP}")
    for name, k in KERNELS.items():
        if not k.is_file():
            degrade.append(f"kernel 缺失 {name}: {k}")

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {"schema": "rurix.g31.renderer_sdk.skip.v1", "state": "DEV_ENV_DEGRADE",
               "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0

    WORK.mkdir(parents=True, exist_ok=True)
    SPV_DIR.mkdir(parents=True, exist_ok=True)
    clang_env = base_env()
    clang_env["RURIXC_CLANG"] = str(clang)

    # ── 构建:rurixc + 实现层 cdylib(release——帧时与生产 bench 腿同档位诚实口径;
    # sdk-device feature = device 执行面显式启用,常驻回归网空骨架纪律见 Cargo.toml)──
    rurixc = build_rurixc()
    if rurixc is None:
        fail("rurixc 构建失败")
        return 1
    b = run(["cargo", "build", "-q", "--release", "-p", "rurix-renderer-sdk",
             "--features", "sdk-device"], timeout=3600)
    sdk_dll = ROOT / "target" / "release" / "rurix_renderer_sdk.dll"
    sdk_implib = ROOT / "target" / "release" / "rurix_renderer_sdk.dll.lib"
    if b.returncode != 0 or not sdk_dll.is_file() or not sdk_implib.is_file():
        fail(f"rurix-renderer-sdk cdylib 构建失败: {(b.stdout + b.stderr)[-400:]}")
        return 1
    # #[link(name = "rurix_renderer_sdk")] 消费面:import lib 复制为 <name>.lib
    # (MSVC cdylib 副产 <name>.dll.lib;RXS-0195 链接段追加 <name>.lib)。
    link_lib = WORK / "rurix_renderer_sdk.lib"
    link_lib.write_bytes(sdk_implib.read_bytes())
    (WORK / "rurix_renderer_sdk.dll").write_bytes(sdk_dll.read_bytes())

    # ── host 段②:--emit=dll 三件齐 + 生成头声明集/幂等/篡改 RED ──
    stem = WORK / "rurix_renderer"
    dll, imp_lib, hdr = stem.with_suffix(".dll"), stem.with_suffix(".lib"), stem.with_suffix(".h")
    # rurixc 链接段 `rurix_renderer_sdk.lib` 定位:LIB env 注入 WORK(RXS-0195
    # 最小策略——/libpath 序仅 MSVC/SDK,bare 名经 LIB 环境解析)。
    emit_env = dict(clang_env)
    emit_env["LIB"] = str(WORK) + os.pathsep + emit_env.get("LIB", "")
    e1 = emit_dll(rurixc, SDK_RX, stem, emit_env)
    if e1.returncode != 0 or not dll.is_file():
        blob = (e1.stdout + e1.stderr)[-1600:]
        if "error[RX" in blob and "error[RX7001]" not in blob:
            print(blob, file=sys.stderr)
            fail("sdk.rx `--emit=dll` 编译期红(导出面不合 subset v1?)")
            return 1
        print(blob, file=sys.stderr)
        fail("`--emit=dll` 失败(link.exe / 工具链面)")
        return 1
    hdr_ok = imp_lib.is_file() and hdr.is_file()
    declared = header_names(hdr.read_text(encoding="utf-8")) if hdr.is_file() else set()
    hdr_set_ok = declared == EXPECTED_EXPORTS
    canonical = hdr.read_bytes() if hdr.is_file() else b""
    e2 = emit_dll(rurixc, SDK_RX, stem, emit_env)
    idem_ok = e2.returncode == 0 and hdr.read_bytes() == canonical and len(canonical) > 0
    htext = canonical.decode("utf-8", "replace")
    abs_path = bool(re.search(r"[A-Za-z]:[\\/]", htext)) or str(ROOT) in htext
    tampered = bytearray(canonical)
    tamper_ok = False
    if tampered:
        tampered[len(tampered) // 2] ^= 0x20
        hdr.write_bytes(bytes(tampered))
        e3 = emit_dll(rurixc, SDK_RX, stem, emit_env)
        regen = hdr.read_bytes()
        tamper_ok = (
            e3.returncode == 0 and regen == canonical and regen != bytes(tampered)
        )
    set_fact(
        "header_generated_and_idempotent",
        hdr_ok and hdr_set_ok and idem_ok and not abs_path and tamper_ok,
        f"--emit=dll 三件齐={hdr_ok} 声明集==期望 9 导出={hdr_set_ok} 幂等={idem_ok} "
        f"无绝对路径={not abs_path} 篡改再生成 RED={tamper_ok}",
    )
    if not (hdr_ok and hdr_set_ok):
        fail("生成头面破缺,后续 device 段无头可用")
        return 1

    # ── SPV 四件套编译(rurixc --target vulkan;canonical kernel 面)──
    for name, ksrc in KERNELS.items():
        out_spv = SPV_DIR / name
        ks = run([str(rurixc), str(ksrc), "--target", "vulkan", "-o", str(out_spv)],
                 env=clang_env, timeout=3600)
        if ks.returncode != 0 or not out_spv.is_file():
            fail(f"kernel {name} rurixc --target vulkan 编译失败: {(ks.stdout + ks.stderr)[-400:]}")
            return 1
    note("SPV 四件套编译绿(g14_3_direct_gi / g14_mv / g14_8_tsr_resample / g14_8_tsr_resolve)")

    # ── device 段:cl.exe 编宿主 + 真跑(GPU 独占窗)──
    cl = locate_msvc()
    host_env = msvc_env(clang_env, hdr.parent, [WORK])
    exe = WORK / "renderer_sdk_host.exe"
    pc = run(
        [
            str(cl), "/nologo", "/std:c++17", "/EHsc", str(HOST_CPP),
            f"/Fe:{exe}", f"/Fo:{WORK}\\",
            "/link", "rurix_renderer.lib", "rurix_renderer_sdk.lib",
        ],
        cwd=WORK,
        env=host_env,
    )
    if pc.returncode != 0 or not exe.is_file():
        fail(f"cl.exe 编译外部宿主失败: {(pc.stdout + pc.stderr)[-600:]}")
        return 1
    note("cl.exe 编译 renderer_sdk_host.exe 绿(include 现场再生成头 + 链两 import lib)")

    host_argv = [
        str(exe),
        "--contract", str(CONTRACT),
        "--gltf", str(BISTRO_GLTF),
        "--scene", SCENE,
        "--tier", str(TIER),
        "--spv-dir", str(SPV_DIR),
        "--frames", str(FRAMES),
        "--warmup", str(WARMUP),
    ]
    with gpu_device_lock(purpose=f"{TAG} 外部宿主 canonical 170 帧真跑"):
        hr = run(host_argv, cwd=WORK, env=base_env(), timeout=7200)
    host_out = (hr.stdout or "") + (hr.stderr or "")
    io.open(WORK / "host_run.log", "w", encoding="utf-8", newline="\n").write(host_out)
    if hr.returncode in (2, 3):
        doc = {"schema": "rurix.g31.renderer_sdk.skip.v1", "state": "DEV_ENV_DEGRADE",
               "reasons": [f"宿主 rc={hr.returncode}(create/load 面 dev-env 缺失)"]}
        print(json.dumps(doc, ensure_ascii=False))
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但宿主 dev-env 降级(rc={hr.returncode})",
                  file=sys.stderr)
            return 1
        note(f"SKIP DEV_ENV_DEGRADE(宿主 rc={hr.returncode},非 PASS 非 FAIL)")
        return 0
    tokens = parse_host_tokens(hr.stdout or "")
    if hr.returncode != 0:
        fail(f"外部宿主真跑失败 rc={hr.returncode}: {host_out.strip()[-400:]}")
    set_fact(
        "host_integration_ok",
        hr.returncode == 0 and host_integration_ok(tokens),
        f"RXSDK_HOST_OK={tokens.get('host_ok')} abi={tokens.get('abi')} caps={tokens.get('caps')} "
        f"params_ok={tokens.get('params_ok')} present_ok={tokens.get('present_ok')} rc={hr.returncode}",
    )

    anchors = json.loads(ANCHOR_PATH.read_text(encoding="utf-8")).get("anchors") or {}
    anchor_digest = (anchors.get(ANCHOR_CELL) or {}).get("last_frame_digest")
    digest_hit = digest_matches(tokens.get("digest"), anchor_digest)
    set_fact(
        "digest_matches_stage_a_anchor",
        digest_hit,
        f"canonical {FRAMES}+{WARMUP} 末帧 digest {str(tokens.get('digest'))[:23]}… vs "
        f"Stage A 锚 {ANCHOR_CELL} {str(anchor_digest)[:23]}… "
        f"{'位级 MATCH(SDK 面 ≡ 生产管线)' if digest_hit else 'DRIFT(RED)'}",
    )
    set_fact(
        "frame_time_measured",
        frame_time_ok(tokens),
        f"post-warmup 帧时 mean={tokens.get('frame_ms_mean')}ms p50={tokens.get('frame_ms_p50')}ms "
        f"n={tokens.get('frame_samples')}(要求 n=={FRAMES} 且均 >0;回读帧含回读税诚实口径)",
    )

    # ── stable 快照面(RD-008 机制渲染器面延伸机核)──
    snap_doc = json.loads(SNAPSHOT.read_text(encoding="utf-8")) if SNAPSHOT.is_file() else {}
    snap_sec = snap_doc.get("renderer_sdk_api") or {}
    sc = run(["py", "-3", "ci/stable_snapshot.py", "--check"], timeout=600)
    snap_ok = (
        sc.returncode == 0
        and snap_sec.get("export_count") == len(EXPECTED_EXPORTS)
        and snap_sec.get("abi_version") == SEMVER
        and len(snap_sec.get("exports") or []) == len(EXPECTED_EXPORTS)
    )
    set_fact(
        "stable_snapshot_extended",
        snap_ok,
        f"stable_snapshot --check exit={sc.returncode};renderer_sdk_api 段 export_count="
        f"{snap_sec.get('export_count')} abi_version={snap_sec.get('abi_version')}"
        "(bless_log 2026-08-25 行;RD-008 history 同日行处置登记)",
    )

    # ── EI1/UC-05 既有面回归(export_c_smoke 复跑)──
    reg = run(["py", "-3", "ci/export_c_smoke.py"], env=base_env(), timeout=7200)
    set_fact(
        "uc05_export_c_regression_green",
        reg.returncode == 0,
        f"ci/export_c_smoke.py(RURIX_REQUIRE_REAL=1)复跑 exit={reg.returncode}"
        "(EI1 export_c codegen 面零回归;UC-05 面由同 codegen 承载)",
    )

    # ── 门裁决(facts 全绿 + FAILURES 空)──
    all_pass = all(f["status"] == "PASS" for f in facts.values()) and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti(本机单卡 measured_local)",
        "os": "windows",
        "sdk_build_profile": "release(rurix-renderer-sdk cdylib;.rx rurix_renderer.dll 经 rurixc debug 编译器发射)",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "api": {
            "abi_version_packed": ABI_PACKED,
            "semver": SEMVER,
            "export_count": len(EXPECTED_EXPORTS),
            "version_policy": "apps/g31-renderer-sdk/API_VERSIONING.md",
            "breaking_change_policy": (
                "MAJOR 破坏性变更走 RFC + 新旧 MAJOR DLL 并存;MINOR 同 MAJOR 内只增不破坏"
                "(镜像 RXS-0180 L2);PATCH 语义不变修复"
            ),
        },
        "host_run": {
            "host": "apps/g31-renderer-sdk/host/renderer_sdk_host.cpp(C++ 控制台,"
                    "include 生成头 + 链 rurix_renderer.lib/rurix_renderer_sdk.lib)",
            "scene": SCENE,
            "tier": TIER,
            "frames": FRAMES,
            "warmup": WARMUP,
            "abi_seen": tokens.get("abi") or "0x00000000",
            "caps": tokens.get("caps") if isinstance(tokens.get("caps"), int) else 0,
            "frame_ms_mean": tokens.get("frame_ms_mean", -1.0),
            "frame_ms_p50": tokens.get("frame_ms_p50", -1.0),
            "frame_samples": tokens.get("frame_samples", -1),
            "last_frame_digest": tokens.get("digest") or ("sha256:" + "0" * 64),
            "stage_a_anchor_digest": anchor_digest or ("sha256:" + "0" * 64),
            "digest_matches_stage_a_anchor": bool(digest_hit),
            "params_update_ok": bool(tokens.get("params_ok")),
            "present_ok": bool(tokens.get("present_ok")),
            "exit_code": hr.returncode,
        },
        "stable_snapshot": {
            "section": "renderer_sdk_api",
            "export_count": snap_sec.get("export_count", -1),
            "abi_version": snap_sec.get("abi_version", ""),
            "check_pass": sc.returncode == 0,
            "bless_log_row": "tests/stable/bless_log.md 2026-08-25",
        },
        "rd036": {
            "subset_v1_compliant": bool(subset_ok),
            "disposition": "maintain_open",
            "upcall_trigger": False,
            "external_fixed_abi_trigger": False,
        },
        "regression": {"export_c_smoke_exit": reg.returncode},
        "facts": [facts[fid] for fid in FACT_IDS],
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C1 渲染器 SDK 稳定 API 面(G31_PLUS §5 #48):两层 DLL 架构——"
            ".rx stable API 面(sdk.rx 9 个 #[export(c)] 薄转发,export_c codegen 产 "
            "rurix_renderer.dll + import lib + 生成头)+ 实现层 rurix-renderer-sdk cdylib"
            "(rxsdk_* u64 句柄会话面,薄封装 G14.3 统一四 pass TSR 生产车道 include! 共享体,"
            "U-59 注册)。外部 C++ 宿主全链真跑:初始化(能力协商)→ bistro 契约场景提交 → "
            f"canonical {FRAMES}+{WARMUP} 帧循环(末帧 digest 对拍 Stage A 锚 {ANCHOR_CELL})→ "
            "参数更新见证(相机/曝光合法面+非法面拒+续渲)→ present 句柄 → 关闭。"
            "stable 快照 renderer_sdk_api 段纳入守卫(RD-008 机制延伸,处置登记 deferred.json "
            "2026-08-25);RD-036 判档不成立维持 open(签名面 subset v1 机核)。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in (facts[fid] for fid in FACT_IDS))}"
        ),
    }
    import jsonschema  # 自校验硬门(schema 漂移即 RED)

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        for e in errs[:5]:
            fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
        all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_renderer_sdk_{ts}.json"
    else:
        # FAIL 诊断件落 .tmp 工作区——fail-closed:evidence/ 无件 = 门未过。
        gate_path = WORK / f"gate_fail_{ts}.json"
    io.open(gate_path, "w", encoding="utf-8", newline="\n").write(
        json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n"
    )
    note(f"evidence: {gate_path.relative_to(ROOT)}")
    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest(判读器红绿两臂,无 GPU/无构建依赖)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    # 红绿臂①:subset v1 谓词。
    expect(type_in_subset_v1("u64"), "GREEN:标量 u64 合规")
    expect(type_in_subset_v1("*const u8"), "GREEN:*const u8 合规")
    expect(type_in_subset_v1("*mut f64"), "GREEN:*mut f64 合规")
    expect(type_in_subset_v1(" f32 "), "GREEN:空白规整合规")
    expect(not type_in_subset_v1("CameraSpec"), "RED:struct 按值必红")
    expect(not type_in_subset_v1("*mut CameraSpec"), "RED:*mut struct 必红(subset v1 指针 T∈标量)")
    expect(not type_in_subset_v1("extern \"C\" fn(u32) -> u32"), "RED:回调指针必红")
    expect(not type_in_subset_v1("[u8; 4]"), "RED:数组按值必红")
    expect(not type_in_subset_v1("*mut *const u8"), "RED:二级指针必红")
    # 红绿臂②:sdk.rx 签名面机核(真文件 + 合成违例)。
    ok_real, _ = check_sdk_rx_subset_v1(SDK_RX.read_text(encoding="utf-8"))
    expect(ok_real, "GREEN:真 sdk.rx 9 导出全合规")
    bad_src = (
        "#[export(c)]\npub fn bad(s: CameraSpec) -> u32 { 0 }\n"
        "#[export(c)]\npub fn ok(p: *mut i32) -> i32 { 0 }\n"
    )
    ok_bad, bad_list = check_sdk_rx_subset_v1(bad_src)
    expect(not ok_bad and any("CameraSpec" in b for b in bad_list), "RED:struct 按值签名检出")
    expect(check_sdk_rx_subset_v1("// 空文件\n")[0] is False, "RED:零导出必红")
    # 红绿臂③:宿主 token 解析。
    full = (
        "RXSDK_HOST_ABI=0x00010000\nRXSDK_HOST_CAPS=1\n"
        "RXSDK_HOST_LOAD_OK tier=100 frames=160 warmup=10\n"
        "RXSDK_HOST_FRAME mean=4.0123 p50=3.9876 n=160\n"
        f"RXSDK_HOST_DIGEST sha256:{'ab' * 32}\n"
        "RXSDK_HOST_PARAMS_OK\nRXSDK_HOST_PRESENT_OK\nRXSDK_HOST_OK\n"
    )
    tk = parse_host_tokens(full)
    expect(tk.get("host_ok") and tk.get("abi") == ABI_PACKED, "GREEN:全 token 解析")
    expect(host_integration_ok(tk), "GREEN:host_integration_ok 正例")
    expect(frame_time_ok(tk), "GREEN:frame_time_ok 正例")
    expect(digest_matches(tk.get("digest"), "sha256:" + "ab" * 32), "GREEN:digest 位级相等")
    missing = full.replace("RXSDK_HOST_OK\n", "")
    expect(not host_integration_ok(parse_host_tokens(missing)), "RED:缺终 token 拒判")
    no_digest = parse_host_tokens(full.replace(f"RXSDK_HOST_DIGEST sha256:{'ab' * 32}\n", ""))
    expect(not digest_matches(no_digest.get("digest"), "sha256:" + "ab" * 32), "RED:缺 digest 拒判")
    expect(not digest_matches(tk.get("digest"), "sha256:" + "cd" * 32), "RED:digest 不等必红")
    expect(not digest_matches(tk.get("digest"), None), "RED:锚缺失必红")
    bad_frames = parse_host_tokens(full.replace("n=160", "n=159"))
    expect(not frame_time_ok(bad_frames), "RED:样本数不足必红")
    zero_ms = parse_host_tokens(full.replace("mean=4.0123", "mean=0.0000"))
    expect(not frame_time_ok(zero_ms), "RED:帧时零值必红")
    bad_abi = parse_host_tokens(full.replace("0x00010000", "0x00020000"))
    expect(not host_integration_ok(bad_abi), "RED:ABI MAJOR 不符拒判")
    # 红绿臂④:政策字面 + 三态 + 生成头解析。
    expect(versioning_policy_ok("MAJOR 破坏性变更走 RFC;MINOR 只增;PATCH 修复;1.0.0"),
           "GREEN:政策字面正例")
    expect(not versioning_policy_ok("MAJOR only"), "RED:政策字面不全必红")
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")
    hdr_sample = (
        "/* Generated */\n#ifndef RURIX_RENDERER_H\n#include <stdint.h>\n"
        "#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n"
        "uint32_t rurix_renderer_abi_version();\n"
        "uint64_t rurix_renderer_create(uint32_t);\n"
        "\n#ifdef __cplusplus\n}\n#endif\n#endif\n"
    )
    expect(header_names(hdr_sample) == {"rurix_renderer_abi_version", "rurix_renderer_create"},
           "GREEN:生成头声明集解析")
    # schema 互核:在树 + 关键 const/required 逐字。
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        expect(
            sorted(gs.get("required", [])) == sorted([
                "schema", "subject", "symbolic_gate_key", "wave", "api", "host_run",
                "stable_snapshot", "rd036", "regression", "facts",
                "environment", "timestamp", "notes",
            ]),
            "schema required 闭集互核(13 字段)",
        )
        expect(gs["properties"]["api"]["properties"]["export_count"]["const"] == 9,
               "api.export_count const=9 互核")
        expect(gs["properties"]["host_run"]["properties"]["frames"]["const"] == FRAMES,
               "host_run.frames const 互核")
        expect(gs["properties"]["stable_snapshot"]["properties"]["section"]["const"] == "renderer_sdk_api",
               "stable_snapshot.section const 互核")
        expect(gs["properties"]["rd036"]["properties"]["disposition"]["const"] == "maintain_open",
               "rd036.disposition const 互核")
        fact_enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(fact_enum) == sorted(FACT_IDS), "facts id 枚举闭集互核(8)")
        expect(gs["properties"]["facts"]["minItems"] == 8
               and gs["properties"]["facts"]["maxItems"] == 8, "facts 基数 = 8 互核")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=8;4 红臂组 + 正例组 + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        return run_gate()
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
