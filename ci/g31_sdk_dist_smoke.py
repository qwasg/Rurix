#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C5 渲染器 SDK 分发打包）
# G37 W5 升级:16→24 组件闭集扩面(新 SPV 四件 + 许可义务四件)+ sdk-1.0.0→sdk-1.1.0,
# 判读随 schema v2(milestones/g31/g31_sdk_dist_v2_evidence_schema.json,门键
# g31.g37w5.dist,evidence 前缀 g31_sdk_dist_v2_);v1 schema/路由/既有 evidence 0-byte。
"""G31+ 波 C Task C5:渲染器 SDK 分发打包门冒烟(v1 门键 g31.waveC.dist,G37 W5
升级后 g31.g37w5.dist;G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #52「渲染器 SDK 分发
打包:预编译 bundle 进 rurixup 链 + 签名/SBOM 扩展」兑现面;交付判据 = SDK bundle
一键安装 + 示例工程离线可建)。

G37 W5 扩面(2026-08-30):
- 新 SPV 四件:g31_realism_transp.spv(工件谱系锚定——transp 链位源快照已被 RIS
  演进覆盖,自 artifacts/.../w5_commercial/bundle/inputs/ 取件 + sha256 锚硬核对)
  / g31_realism_ris.spv(源 = kernels/g31_realism.rx RIS 终态,现编)
  / g31_display_encode_lut.spv / g34_unified_primary_skin.spv(源在树,现编)。
- 许可义务四件:LICENSE-MIT / LICENSE-APACHE / THIRD_PARTY_NOTICES.md /
  third_party_embedded.cdx.json(GAP-01~03 闭合件,组件名/license 字面与
  release.yml 编排段同口径)。
- 组件 license 字面如实分件(第一方件 = MIT OR Apache-2.0 workspace 口径,GAP-02)。

链路(EA1 分发链复用,RXS-0214~0218 机制面;零新 RXS/RD/CI 数字步骤消费):
  C1 产物(sdk.rx 经 rurixc --emit=dll 产 rurix_renderer.dll/.lib/.h + 实现层
  rurix-renderer-sdk cdylib release sdk-device)+ canonical SPV 四件套 + bistro
  生产契约 + 示例工程源(renderer_sdk_host.cpp)+ 文档五件 → 16 组件扁平 staging
  → `rurixup release --channel stable` 产 bundle.json/channel_manifest.json/
  signing_manifest.json/sbom.spdx.json/sbom.cdx.json/SHA256SUMS/gate_decision.json
  → `rurixup install --from-dir` 四级校验真实物化(component_rel_path SDK 面纯
  追加:*.h→include/ *.spv→spv/ *.json→manifests/ *.md→docs/ *.cpp→examples/,
  既有 *.exe/*.lib/nvidia 路径 0-byte,spec/release.md RXS-0214 同条修订)→
  default 切换 + list --verify + 幂等再装 → hermetic 环回 HTTP 网络 install
  (零真实外呼)→ 干净目录仅 bundle+公开工具链(MSVC)离线构建示例 → 真跑
  canonical 160+10 末帧 digest 对拍 Stage A 锚。

判据闭集(milestones/g31/g31_sdk_dist_evidence_schema.json 描述段逐字;facts=9):
1. bundle_assembly_ok:16 组件齐 + 源字节 sha256 == bundle.json digest ==
   SHA256SUMS 行(一比一闭环)+ 同源两次 release 七产物逐字节一致(确定性)。
2. signing_sbom_extended:两 DLL 签名项 Valid+timestamped+verified;SBOM 双视图
   覆盖全 16 组件(名+版本);vendor 运行件技术对账三件——NGX/Streamline 与
   FSR 动态装载不捆绑(dynamic-load-not-bundled,许可清结文在树)+
   basis_universal 静态入 DLL(static-in-dll,SBOM.md 在树)(C6 许可矩阵
   协同面,本任务只做 SBOM 技术对账不做许可裁决)。
3. from_dir_install_ok:--from-dir GREEN(components=16 digest_levels_verified=4
   + 注册表 v2 + 布局 include/bin/lib/spv/manifests/docs/examples 逐字节==源)。
4. switch_and_idempotent_ok:default 切换 sdk-1.0.0 绿 + list --verify 绿 +
   幂等再装 registered=1 不增 + toolchains.json 逐字节一致 + 原子写零 .tmp 残渣。
5. network_install_ok:hermetic 环回 HTTP(stdlib http.server,127.0.0.1 随机
   端口)全链 install 绿 + 物化字节==源;pr-smoke 零真实外呼(fixture 唯一网络面)。
6. offline_build_ok:干净目录(仅已装 bundle + MSVC 公开工具链,无 rurixc/cargo)
   cl.exe 编译示例工程 → minimal_host.exe;毒化代理 env(HTTP_PROXY=127.0.0.1:9)
   见证本腿零网络依赖;耗时 measured。
7. offline_run_digest_ok:示例真跑 canonical 160+10(GPU 锁),末帧 digest ==
   Stage A 锚 bistro-interior_t100_tsr_device + 帧时 mean/p50>0 n=160;耗时 measured。
8. red_arms_closed:四红臂各自独立见证 + 复原绿——①签名错(--sign Unsigned →
   release exit 2 failed_gates=[signing] 发布阻断)②哈希错(组件坏一字节 →
   级④内容寻址拒 kind=integrity 零半装)③截断(环回半 body → curl 部分传输
   kind=network)④清单篡改(channel_manifest 坏字节 → 级①锚失配
   kind=integrity)+ 端点不可达(fixture 关 → kind=network)+ 复原绿。
9. ea1_regression_green:ci/rurixup_dist_smoke.py 复跑 exit 0(EA1 链既有门
   零破坏——component_rel_path 扩展对既有组件形态行为不变)。

三态:无 cargo/clang/MSVC/Vulkan/GPU/bistro 资产 → DEV_ENV_DEGRADE 退 0(不冒充
PASS);RURIX_REQUIRE_REAL=1(gate 缺省置 1)下降级翻硬 FAIL(禁 mock 充真跑)。

evidence 纪律:PASS 才落 evidence/g31_sdk_dist_<ts>.json(check_schemas 前缀路由
g31_sdk_dist_);FAIL 诊断件落 .tmp/g31_gates/sdk_dist/ 工作区不污染 evidence/
路由面(fail-closed:evidence/ 无件 = 门未过)。

用法:
  py -3 ci/g31_sdk_dist_smoke.py --selftest
  py -3 ci/g31_sdk_dist_smoke.py --gate g31.waveC.dist
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import http.server
import io
import json
import os
import re
import shutil
import socketserver
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.g37w5.dist"
SUBJECT = "g31_sdk_dist_v2"
WAVE = "G37.W5"
TAG = "g31_sdk_dist_v2"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_sdk_dist_v2_evidence_schema.json"
SCHEMA_ID = "rurix.g31.sdk_dist_evidence.v2"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
SDK_RX = ROOT / "apps" / "g31-renderer-sdk" / "src" / "sdk.rx"
HOST_CPP = ROOT / "apps" / "g31-renderer-sdk" / "host" / "renderer_sdk_host.cpp"
VERSIONING_MD = ROOT / "apps" / "g31-renderer-sdk" / "API_VERSIONING.md"
CONTRACT = ROOT / "milestones" / "g13" / "g13_ue_upscale_parity_contract.json"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
DOCS = {
    "integration_guide.md": ROOT / "docs" / "renderer" / "integration_guide.md",
    "feature_matrix.md": ROOT / "docs" / "renderer" / "feature_matrix.md",
    "performance_tuning.md": ROOT / "docs" / "renderer" / "performance_tuning.md",
    "compatibility_matrix.md": ROOT / "docs" / "renderer" / "compatibility_matrix.md",
}
WORK = ROOT / ".tmp" / "g31_gates" / "sdk_dist"
STAGE = WORK / "fromdir"

SDK_VERSION = "sdk-1.1.0"
SDK_CHANNEL = "stable"
SCENE, TIER, FRAMES, WARMUP = "bistro-interior", 100, 160, 10
KERNELS = {
    "g14_3_direct_gi.spv": ROOT / "src" / "rurix-render" / "kernels" / "g14_3_direct_gi.rx",
    "g14_mv.spv": ROOT / "src" / "rurix-render" / "kernels" / "g14_mv.rx",
    "g14_8_tsr_resample.spv": ROOT / "src" / "rurix-render" / "kernels" / "g14_8_tsr_resample.rx",
    "g14_8_tsr_resolve.spv": ROOT / "src" / "rurix-render" / "kernels" / "g14_8_tsr_resolve.rx",
    # G37 W5 新臂 kernel(源在树,现编;transp 链位见 PREBUILT_SPV 工件谱系锚定)。
    "g31_display_encode_lut.spv": ROOT / "src" / "rurix-render" / "kernels" / "g31_display_encode_lut.rx",
    "g34_unified_primary_skin.spv": ROOT / "src" / "rurix-render" / "kernels" / "g34_unified_primary_skin.rx",
    "g31_realism_ris.spv": ROOT / "src" / "rurix-render" / "kernels" / "g31_realism.rx",
}
# transp 链位工件谱系锚定:g31_realism.rx 已被 RIS/NEE 演进覆盖(链式超集律,
# day_0829 HANDOVER §C),transp 快照源不在树 ⇒ 自战役快照取件 + sha256 锚硬核对
# (缺件 → DEV_ENV_DEGRADE;锚失配 → FAIL fail-closed 不冒充)。
PREBUILT_SPV = {
    "g31_realism_transp.spv": (
        ROOT / "artifacts" / "day_0830_delivery" / "w5_commercial" / "bundle"
        / "inputs" / "g31_realism_transp.spv",
        "35983d0f405169ec84bf222f4a12ec8bf8dfd7d471eefb12488eea7dd34c4f8b",
    ),
}
# 许可义务四件(GAP-01~03 闭合;组件名/license 字面与 release.yml 编排段同口径)。
LICENSE_COMPONENTS = {
    "LICENSE-MIT": (ROOT / "LICENSE-MIT", "MIT"),
    "LICENSE-APACHE": (ROOT / "LICENSE-APACHE", "Apache-2.0"),
    "THIRD_PARTY_NOTICES.md": (ROOT / "dist" / "licenses" / "THIRD_PARTY_NOTICES.md",
                               "MIT OR Apache-2.0"),
    "third_party_embedded.cdx.json": (ROOT / "dist" / "sbom" / "third_party_embedded.cdx.json",
                                      "MIT OR Apache-2.0"),
}
SDK_RUNTIME = ["rurix_renderer.dll", "rurix_renderer.lib", "rurix_renderer.h",
               "rurix_renderer_sdk.dll", "rurix_renderer_sdk.lib"]
EXPECTED_COMPONENTS = sorted(
    SDK_RUNTIME
    + list(KERNELS)
    + list(PREBUILT_SPV)
    + [CONTRACT.name, HOST_CPP.name, VERSIONING_MD.name]
    + list(DOCS)
    + list(LICENSE_COMPONENTS)
)


def component_license(name: str) -> str:
    """组件 license 字面(release.yml 同口径:许可件如实分件,第一方件 workspace
    双许可 MIT OR Apache-2.0——GAP-02 闭合口径)。纯函数。"""
    if name in LICENSE_COMPONENTS:
        return LICENSE_COMPONENTS[name][1]
    return "MIT OR Apache-2.0"
SIGNED_DLLS = ["rurix_renderer.dll", "rurix_renderer_sdk.dll"]
# vendor 运行件技术对账(C6 许可矩阵协同面;本任务只做 SBOM 技术对账——引文在树机核,
# 许可裁决归 C6)。NGX/FSR 动态装载不捆绑(vendor_upscale.rs LoadLibraryExW 运行时装载,
# 许可边界:vendor SDK 二进制不入 git);basis_universal 静态入 rurix_renderer_sdk.dll。
VENDOR_RUNTIME = [
    {"name": "sl.interposer.dll+nvngx_dlss.dll (Streamline 2.10.3 / NGX)",
     "license": "NVIDIA-RTX-SDKs-LICENSE(owner 2026-08-18 清结)",
     "linkage": "dynamic-load-not-bundled",
     "ref": "milestones/g13/design/vendor_upscale_license_clearance.md"},
    {"name": "amd_fidelityfx_loader_dx12.dll+amd_fidelityfx_upscaler_dx12.dll (FSR 3.1.5)",
     "license": "MIT",
     "linkage": "dynamic-load-not-bundled",
     "ref": "milestones/g13/design/vendor_upscale_license_clearance.md"},
    {"name": "basis_universal 1.16.4",
     "license": "Apache-2.0",
     "linkage": "static-in-dll",
     "ref": "src/rurix-basis-sys/SBOM.md"},
]

# 工具链 pin(与 ci/g31_renderer_sdk_smoke.py 同源;RURIXC_CLANG 覆写 clang)。
CLANG = Path(r"C:/Program Files/LLVM/bin/clang.exe")
MSVC_ROOT = Path(r"C:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.44.35207")
MSVC_BIN = MSVC_ROOT / "bin" / "Hostx64" / "x64"
SDK_INC = Path(r"C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0")
SDK_LIB = Path(r"C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0")

FACT_IDS = [
    "bundle_assembly_ok",
    "signing_sbom_extended",
    "from_dir_install_ok",
    "switch_and_idempotent_ok",
    "network_install_ok",
    "offline_build_ok",
    "offline_run_digest_ok",
    "red_arms_closed",
    "ea1_regression_green",
]
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


def sha256_file(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面;全纯函数无 GPU/构建依赖)
# ---------------------------------------------------------------------------


def install_rejected_cleanly(exit_code: int, toolchains_exists: bool, registry_exists: bool) -> bool:
    """RED 判据:安装被拒(退出非 0)且零半装(无版本目录、无注册表)。纯函数(EA1 同型)。"""
    return exit_code != 0 and not toolchains_exists and not registry_exists


def install_succeeded(exit_code: int, toolchains_exists: bool, registry_exists: bool) -> bool:
    """GREEN 判据:安装成功(退出 0)且磁盘物化 + 注册表落地。纯函数(EA1 同型)。"""
    return exit_code == 0 and toolchains_exists and registry_exists


def has_kind(stdout: str, kind: str) -> bool:
    """机器 token 判据:stdout 含 `RURIXUP_INSTALL_ERROR: kind=<kind>`。纯函数(EA1 同型)。"""
    return f"RURIXUP_INSTALL_ERROR: kind={kind}" in (stdout or "")


def release_tokens(stdout: str) -> dict:
    out = {}
    for ln in (stdout or "").splitlines():
        if ln.startswith("RURIXUP_RELEASE:"):
            for tok in ln[len("RURIXUP_RELEASE:"):].split():
                if "=" in tok:
                    k, v = tok.split("=", 1)
                    out[k] = v
    return out


def release_blocked_with_gate(exit_code: int, tokens: dict, gate: str) -> bool:
    """签名错红臂判据:release 发布阻断(exit 2)+ failed_gates 恰含 <gate> 且
    allow_upload=false。纯函数。"""
    if exit_code != 2 or tokens.get("allow_upload") != "false":
        return False
    failed = [g for g in tokens.get("failed_gates", "").strip("[]").split(",") if g]
    return gate in failed


def digests_match(a: str, b: str) -> bool:
    """一比一内容寻址判据:两 digest 逐字符相等且非空。纯函数(EA1 同型)。"""
    return bool(a) and a == b


def component_set_ok(names) -> bool:
    """bundle 组件闭集判据:干名集 == EXPECTED_COMPONENTS(24,G37 W5 扩面)。纯函数。"""
    return sorted(names) == EXPECTED_COMPONENTS


def expected_rel_paths(names) -> dict:
    """组件干名 → toolchains/<ver>/ 相对路径(纯函数镜像 install.rs
    component_rel_path SDK 面扩展;*.h→include/ *.spv→spv/ *.json→manifests/
    *.md→docs/ *.cpp→examples/ *.lib→bin/lib/ 其余→bin/)。"""
    out = {}
    for n in names:
        if n.endswith(".lib"):
            out[n] = f"bin/lib/{n}"
        elif n.endswith(".h"):
            out[n] = f"include/{n}"
        elif n.endswith(".spv"):
            out[n] = f"spv/{n}"
        elif n.endswith(".json"):
            out[n] = f"manifests/{n}"
        elif n.endswith(".md"):
            out[n] = f"docs/{n}"
        elif n.endswith(".cpp"):
            out[n] = f"examples/{n}"
        else:
            out[n] = f"bin/{n}"
    return out


def sbom_covers(text: str, names, version: str) -> bool:
    """SBOM 覆盖判据:视图文本含全部组件干名 + bundle 版号。纯函数。"""
    return bool(text) and version in text and all(n in text for n in names)


def digest_matches(fresh: str | None, anchor: str | None) -> bool:
    return (
        isinstance(fresh, str)
        and isinstance(anchor, str)
        and DIGEST_RE.match(fresh) is not None
        and fresh == anchor
    )


def parse_host_tokens(out: str) -> dict:
    """示例宿主 stdout token 解析(ci/g31_renderer_sdk_smoke.py 同口径子集)。"""
    d: dict = {}
    m = re.search(r"^RXSDK_HOST_ABI=(0x[0-9a-f]{8})\s*$", out, re.MULTILINE)
    if m:
        d["abi"] = m.group(1)
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
    d["host_ok"] = "RXSDK_HOST_OK" in out
    return d


def frame_time_ok(tokens: dict, expect_n: int = FRAMES) -> bool:
    return (
        tokens.get("frame_ms_mean", 0.0) > 0.0
        and tokens.get("frame_ms_p50", 0.0) > 0.0
        and tokens.get("frame_samples") == expect_n
    )


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决:无降级 → None(续跑);有降级 + REQUIRE_REAL → 1(硬红);
    有降级无 REQUIRE_REAL → 0(SKIP 非 PASS 非 FAIL)。"""
    if not degrade:
        return None
    return 1 if require_real else 0


def tokens(stdout, prefix):
    out = {}
    for ln in (stdout or "").splitlines():
        if ln.startswith(prefix):
            for tok in ln[len(prefix):].split():
                if "=" in tok:
                    k, v = tok.split("=", 1)
                    out[k] = v
    return out


# ---------------------------------------------------------------------------
# dev-env / 构建层
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


def msvc_env(base: dict[str, str], include_dirs: list[Path], lib_dirs: list[Path]) -> dict[str, str]:
    env = dict(base)
    env["INCLUDE"] = os.pathsep.join([
        str(MSVC_ROOT / "include"),
        str(SDK_INC / "ucrt"),
        str(SDK_INC / "shared"),
        str(SDK_INC / "um"),
        str(SDK_INC / "winrt"),
    ] + [str(p) for p in include_dirs])
    env["LIB"] = os.pathsep.join([
        str(MSVC_ROOT / "lib" / "x64"),
        str(SDK_LIB / "ucrt" / "x64"),
        str(SDK_LIB / "um" / "x64"),
    ] + [str(p) for p in lib_dirs])
    env["PATH"] = str(MSVC_BIN) + os.pathsep + env.get("PATH", "")
    return env


def offline_env(base: dict[str, str]) -> dict[str, str]:
    """离线见证 env:毒化代理(127.0.0.1:9 = 不可达)+ 摘除环回例外——本腿任一
    网络外呼必失败,以此证明离线可建面零网络依赖。"""
    env = dict(base)
    for k in ("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"):
        env[k] = "http://127.0.0.1:9"
    env.pop("RURIXUP_TEST_ALLOW_LOOPBACK_HTTP", None)
    return env


def install_env(home: Path, loopback: bool = False) -> dict:
    env = dict(os.environ)
    env["RURIX_HOME"] = str(home)
    if loopback:
        env["RURIXUP_TEST_ALLOW_LOOPBACK_HTTP"] = "1"
    else:
        env.pop("RURIXUP_TEST_ALLOW_LOOPBACK_HTTP", None)
    return env


def build_rurixup() -> Path | None:
    r = run(["cargo", "build", "-q", "-p", "rurixup"], timeout=3600)
    if r.returncode != 0:
        print((r.stdout + r.stderr)[-1200:], file=sys.stderr)
        return None
    exe = ROOT / "target" / "debug" / ("rurixup.exe" if os.name == "nt" else "rurixup")
    return exe if exe.is_file() else None


def build_rurixc() -> Path | None:
    # vulkan-backend feature = SPV 编译面(--target vulkan 硬前置;与 C1 同源)。
    p = run(["cargo", "build", "-q", "-p", "rurixc", "--features", "vulkan-backend",
             "--bin", "rurixc"], timeout=3600)
    if p.returncode != 0:
        print((p.stdout + p.stderr)[-1600:], file=sys.stderr)
        return None
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if os.name == "nt" else "rurixc")
    return exe if exe.is_file() else None


# ---------------------------------------------------------------------------
# hermetic 环回 HTTP fixture(EA1.1b ci/rurixup_dist_smoke.py 同型复用)
# ---------------------------------------------------------------------------

_TRUNCATE: dict = {}


def _make_handler(served_dir: Path):
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_a):  # 静默(pr-smoke 干净输出)
            pass

        def do_GET(self):
            rel = self.path.lstrip("/").split("?", 1)[0]
            fp = served_dir / rel
            if not fp.is_file():
                self.send_error(404, "not found")
                return
            data = fp.read_bytes()
            if _TRUNCATE.get(rel):
                # 声明完整长度但只发一半 → 提前关闭 → curl 报部分传输(exit 18)。
                self.send_response(200)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                try:
                    self.wfile.write(data[: max(1, len(data) // 2)])
                except Exception:
                    pass
                return
            self.send_response(200)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

    return Handler


def start_fixture(served_dir: Path):
    httpd = socketserver.TCPServer(("127.0.0.1", 0), _make_handler(served_dir))
    httpd.timeout = 5
    port = httpd.server_address[1]
    t = threading.Thread(target=httpd.serve_forever, daemon=True)
    t.start()
    return httpd, f"http://127.0.0.1:{port}/"


def write_anchor(anchor_path: Path, ver: str, base_url: str, channel_manifest_bytes: bytes) -> None:
    """写本地锚(schema 同 repo channels/stable.json;base_url 指向 fixture;级① digest 真算)。"""
    digest = hashlib.sha256(channel_manifest_bytes).hexdigest()
    lines = ["{", '  "schema_version": 1,', '  "channel": "stable",', '  "releases": [', "    {",
             f'      "version": "{ver}",',
             f'      "channel_manifest_sha256": "{digest}",',
             f'      "base_url": "{base_url}"',
             "    }", "  ],", f'  "latest": "{ver}"', "}", ""]
    anchor_path.write_bytes("\n".join(lines).encode("utf-8"))
    assert json.loads(anchor_path.read_text(encoding="utf-8"))["releases"][0]["channel_manifest_sha256"] == digest


def net_install(rurixup: Path, ver: str, anchor: Path, home: Path, reg: Path, loopback: bool):
    return run(
        [str(rurixup), "install", ver, "--channel-file", str(anchor),
         "--registry", str(reg), "--home", str(home), "--max-time", "30"],
        env=install_env(home, loopback=loopback),
    )


def _assert_no_staging(home: Path) -> bool:
    tmp = home / "tmp"
    if tmp.is_dir():
        for e in tmp.iterdir():
            if e.name.startswith(".staging-") or e.name.startswith(".download-"):
                return False
    return True


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def do_release(rurixup: Path, out_dir: Path, sign_valid: bool = True):
    cmd = [str(rurixup), "release", "--version", SDK_VERSION, "--channel", SDK_CHANNEL,
           "--out-dir", str(out_dir)]
    for name in EXPECTED_COMPONENTS:
        cmd += ["--component", f"{name}|{SDK_VERSION}|{component_license(name)}|core|{STAGE / name}"]
    for dll in SIGNED_DLLS:
        st = "Valid|true" if sign_valid else "Unsigned|false"
        cmd += ["--sign", f"{dll}|{st}|selftest"]
    return run(cmd)


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

    # ── dev-env 前置面(缺 → DEV_ENV_DEGRADE 登记,不冒充 FAIL 也不 PASS)──
    degrade: list[str] = []
    clang = resolve_clang()
    if clang is None:
        degrade.append("未找到 clang(--emit=dll obj 通道需)")
    if locate_msvc() is None:
        degrade.append("未找到 MSVC cl.exe + Windows SDK(示例工程离线构建需)")
    if not CONTRACT.is_file():
        degrade.append(f"bistro 生产契约缺失 {CONTRACT}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")
    if not ANCHOR_PATH.is_file():
        degrade.append(f"Stage A 锚缺失 {ANCHOR_PATH}")
    if not SDK_RX.is_file():
        degrade.append(f"sdk.rx 缺失 {SDK_RX}")
    if not HOST_CPP.is_file():
        degrade.append(f"示例工程源缺失 {HOST_CPP}")
    for name, k in KERNELS.items():
        if not k.is_file():
            degrade.append(f"kernel 缺失 {name}: {k}")
    for name, (src_path, _anchor) in PREBUILT_SPV.items():
        if not src_path.is_file():
            degrade.append(f"PREBUILT SPV 缺失 {name}: {src_path}(工件谱系锚定件)")
    for name, (p, _lic) in LICENSE_COMPONENTS.items():
        if not p.is_file():
            degrade.append(f"许可组件缺失 {name}: {p}")
    for name, d in DOCS.items():
        if not d.is_file():
            degrade.append(f"文档缺失 {name}: {d}")
    for v in VENDOR_RUNTIME:
        if not (ROOT / v["ref"]).is_file():
            degrade.append(f"vendor 对账引文缺失 {v['ref']}")

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {"schema": "rurix.g31.sdk_dist.skip.v1", "state": "DEV_ENV_DEGRADE",
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
    if STAGE.is_dir():
        shutil.rmtree(STAGE)
    STAGE.mkdir(parents=True)
    clang_env = dict(os.environ)
    clang_env["RURIXC_CLANG"] = str(clang)

    # ── 构建:rurixup + rurixc + 实现层 cdylib(release sdk-device)──
    rurixup = build_rurixup()
    if rurixup is None:
        fail("rurixup 构建失败")
        return 1
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

    # ── emit dll 三件(.rx 导出面 → rurix_renderer.dll/.lib/.h;生成头自始生成不手写)──
    link_lib = WORK / "rurix_renderer_sdk.lib"
    link_lib.write_bytes(sdk_implib.read_bytes())
    stem = WORK / "rurix_renderer"
    dll, imp_lib, hdr = stem.with_suffix(".dll"), stem.with_suffix(".lib"), stem.with_suffix(".h")
    emit_env = dict(clang_env)
    emit_env["LIB"] = str(WORK) + os.pathsep + emit_env.get("LIB", "")
    e1 = run([str(rurixc), str(SDK_RX), "--emit=dll", "-o", str(stem)], env=emit_env, timeout=3600)
    if e1.returncode != 0 or not dll.is_file() or not imp_lib.is_file() or not hdr.is_file():
        fail(f"--emit=dll 三件不齐: {(e1.stdout + e1.stderr)[-600:]}")
        return 1
    note("emit dll 三件齐(rurix_renderer.dll/.lib/.h;生成头编译器单一事实源)")

    # ── SPV 编译(canonical 四件套 + G37 现编三件)──
    for name, ksrc in KERNELS.items():
        out_spv = STAGE / name
        ks = run([str(rurixc), str(ksrc), "--target", "vulkan", "-o", str(out_spv)],
                 env=clang_env, timeout=3600)
        if ks.returncode != 0 or not out_spv.is_file():
            fail(f"kernel {name} rurixc --target vulkan 编译失败: {(ks.stdout + ks.stderr)[-400:]}")
            return 1
    note(f"SPV 现编 {len(KERNELS)} 件绿(canonical 四件套 + G37 lut/skin/ris)")

    # ── PREBUILT SPV(transp 链位工件谱系锚定:sha256 硬核对,失配即红不冒充)──
    for name, (src_path, anchor) in PREBUILT_SPV.items():
        data = src_path.read_bytes()
        actual = hashlib.sha256(data).hexdigest()
        if actual != anchor:
            fail(f"PREBUILT {name} 工件谱系锚失配: {actual[:16]}… ≠ {anchor[:16]}…"
                 "(若 W6 重建已重造该链位,须同步更新 PREBUILT_SPV 锚)")
            return 1
        (STAGE / name).write_bytes(data)
    note(f"PREBUILT SPV {len(PREBUILT_SPV)} 件锚核对绿(transp 35983d0f…)")

    # ── bundle staging(24 组件扁平,干名布局)──
    for src, name in [(dll, "rurix_renderer.dll"), (imp_lib, "rurix_renderer.lib"),
                      (hdr, "rurix_renderer.h"), (sdk_dll, "rurix_renderer_sdk.dll"),
                      (sdk_implib, "rurix_renderer_sdk.lib"), (CONTRACT, CONTRACT.name),
                      (HOST_CPP, HOST_CPP.name), (VERSIONING_MD, VERSIONING_MD.name)]:
        (STAGE / name).write_bytes(src.read_bytes())
    for name, d in DOCS.items():
        (STAGE / name).write_bytes(d.read_bytes())
    for name, (p, _lic) in LICENSE_COMPONENTS.items():
        (STAGE / name).write_bytes(p.read_bytes())
    if not component_set_ok(p.name for p in STAGE.iterdir()):
        fail(f"staging 组件闭集不符: {sorted(p.name for p in STAGE.iterdir())}")
        return 1
    note(f"staging 24 组件齐({len(list(STAGE.iterdir()))} 件)")

    # ── release ×2(打包确定性;签名面 = 两 DLL selftest 验签状态)──
    r1 = do_release(rurixup, WORK / "rel1")
    tok1 = release_tokens(r1.stdout)
    if r1.returncode != 0 or tok1.get("allow_upload") != "true":
        fail(f"release 未放行(exit={r1.returncode}):{r1.stdout[-300:]}\n{r1.stderr[-300:]}")
        return 1
    r2 = do_release(rurixup, WORK / "rel2")
    if r2.returncode != 0:
        fail(f"二次 release 未放行(exit={r2.returncode})")
        return 1
    rel_files = ["bundle.json", "channel_manifest.json", "signing_manifest.json",
                 "sbom.spdx.json", "sbom.cdx.json", "SHA256SUMS", "gate_decision.json"]
    det_ok = all((WORK / "rel1" / n).read_bytes() == (WORK / "rel2" / n).read_bytes()
                 for n in rel_files)
    bundle = json.loads((WORK / "rel1" / "bundle.json").read_text(encoding="utf-8"))
    bundle_digests = {c["name"]: c["sha256"] for c in bundle["components"]}
    sums_rows = {}
    for ln in (WORK / "rel1" / "SHA256SUMS").read_text(encoding="utf-8").splitlines():
        if ln:
            d_, n_ = ln.split("  ", 1)
            sums_rows[n_] = d_
    closure_ok = True
    for name in EXPECTED_COMPONENTS:
        real = sha256_file(STAGE / name)
        if not (digests_match(real, bundle_digests.get(name, ""))
                and digests_match(real, sums_rows.get(name, ""))):
            closure_ok = False
    # bundle.json / channel_manifest.json 入 staging(from-dir / served 布局)。
    for n in ("bundle.json", "channel_manifest.json"):
        (STAGE / n).write_bytes((WORK / "rel1" / n).read_bytes())
    set_fact(
        "bundle_assembly_ok",
        det_ok and closure_ok and component_set_ok(bundle_digests),
        f"24 组件闭集={component_set_ok(bundle_digests)} digest 一比一闭环={closure_ok} "
        f"同源两次 release 七产物逐字节一致={det_ok}(SDK DLL+生成头+import lib+SPV 八件"
        f"〔canonical 四件套+G37 transp/ris/lut/skin〕+契约+示例工程+文档五件+许可四件)",
    )

    # ── 签名/SBOM 扩展面 + vendor 运行件技术对账 ──
    signing = json.loads((WORK / "rel1" / "signing_manifest.json").read_text(encoding="utf-8"))
    signed_ok = {}
    for a in signing.get("artifacts", []):
        signed_ok[a.get("name")] = (
            a.get("status") == "Valid" and a.get("timestamped") is True
            and a.get("verified") is True and a.get("backend") == "self-signed-test"
        )
    sign_ok = all(signed_ok.get(d) for d in SIGNED_DLLS) and signing.get("upload_permitted") is True
    spdx = (WORK / "rel1" / "sbom.spdx.json").read_text(encoding="utf-8")
    cdx = (WORK / "rel1" / "sbom.cdx.json").read_text(encoding="utf-8")
    spdx_ok = sbom_covers(spdx, EXPECTED_COMPONENTS, SDK_VERSION)
    cdx_ok = sbom_covers(cdx, EXPECTED_COMPONENTS, SDK_VERSION)
    vendor_refs_ok = all((ROOT / v["ref"]).is_file() for v in VENDOR_RUNTIME)
    set_fact(
        "signing_sbom_extended",
        sign_ok and spdx_ok and cdx_ok and vendor_refs_ok,
        f"两 DLL 签名 Valid+timestamped+verified={sign_ok};SBOM SPDX 覆盖 24 组件={spdx_ok} "
        f"CycloneDX 覆盖={cdx_ok};vendor 运行件对账三件(NGX/Streamline 与 FSR "
        f"dynamic-load-not-bundled + basis_universal static-in-dll)引文在树={vendor_refs_ok}"
        "(许可义务四件已入组件闭集,GAP-01~03;vendor 裁决引文维持 C6 口径)",
    )

    # ── GREEN:--from-dir 真实物化(四级校验 + 注册表 v2 + SDK 布局)──
    home = WORK / "home"
    reg = home / "toolchains.json"
    ri = run([str(rurixup), "install", "--from-dir", str(STAGE),
              "--registry", str(reg)], env=install_env(home))
    tdir = home / "toolchains" / SDK_VERSION
    s = tokens(ri.stdout, "RURIXUP_INSTALL:")
    layout_ok = False
    byte_ok = False
    if tdir.is_dir():
        rel_map = expected_rel_paths(EXPECTED_COMPONENTS)
        layout_ok = all((tdir / rel).is_file() for rel in rel_map.values())
        byte_ok = all((tdir / rel).read_bytes() == (STAGE / name).read_bytes()
                      for name, rel in rel_map.items())
    reg_v2 = False
    if reg.is_file():
        reg_doc = json.loads(reg.read_text(encoding="utf-8"))
        entry = next((t for t in reg_doc.get("installed", []) if t.get("version") == SDK_VERSION), None)
        reg_v2 = (reg_doc.get("schema_version") == 2 and bool(entry)
                  and bool(entry.get("install_path")) and bool(entry.get("tree_digest")))
    from_dir_ok = (install_succeeded(ri.returncode, tdir.is_dir(), reg.is_file())
                   and s.get("version") == SDK_VERSION and s.get("components") == "24"
                   and s.get("digest_levels_verified") == "4" and layout_ok and byte_ok and reg_v2)
    set_fact(
        "from_dir_install_ok",
        from_dir_ok,
        f"exit={ri.returncode} components={s.get('components')} digest_levels={s.get('digest_levels_verified')} "
        f"布局 include/bin/lib/spv/manifests/docs/examples 齐={layout_ok} 逐字节==源={byte_ok} 注册表 v2={reg_v2}",
    )
    if not from_dir_ok:
        fail(f"--from-dir 物化破缺: {ri.stdout[-300:]}\n{ri.stderr[-300:]}")
        return 1

    # ── 切换探针 + 幂等 ──
    rd = run([str(rurixup), "default", SDK_VERSION, "--registry", str(reg)], env=install_env(home))
    d_tok = tokens(rd.stdout, "RURIXUP_DEFAULT:")
    lv = run([str(rurixup), "list", "--registry", str(reg), "--verify"], env=install_env(home))
    first_reg = reg.read_bytes()
    ri2 = run([str(rurixup), "install", "--from-dir", str(STAGE),
               "--registry", str(reg)], env=install_env(home))
    s2 = tokens(ri2.stdout, "RURIXUP_INSTALL:")
    idem_ok = (ri2.returncode == 0 and s2.get("registered") == "1"
               and reg.read_bytes() == first_reg)
    atomic_clean = not sorted(p.name for p in reg.parent.glob("*.tmp"))
    switch_ok = (rd.returncode == 0 and d_tok.get("default") == SDK_VERSION
                 and lv.returncode == 0 and idem_ok and atomic_clean)
    set_fact(
        "switch_and_idempotent_ok",
        switch_ok,
        f"default 切换 exit={rd.returncode} default={d_tok.get('default')};list --verify "
        f"exit={lv.returncode}(tree_digest 复算零 corrupted);幂等再装 registered={s2.get('registered')} "
        f"注册表逐字节一致={idem_ok};原子写零残渣={atomic_clean}",
    )

    # ── hermetic 环回 HTTP 网络 install(零真实外呼)──
    nethome = WORK / "nethome"
    netreg = nethome / "toolchains.json"
    anchor = WORK / "stable_anchor.json"
    httpd, base_url = start_fixture(STAGE)
    net_ok = False
    net_byte_ok = False
    red_hash_ok = red_trunc_ok = red_mf_ok = red_unreach_ok = False
    try:
        write_anchor(anchor, SDK_VERSION, base_url, (STAGE / "channel_manifest.json").read_bytes())
        rn = net_install(rurixup, SDK_VERSION, anchor, nethome, netreg, loopback=True)
        ntd = nethome / "toolchains" / SDK_VERSION
        net_ok = install_succeeded(rn.returncode, ntd.is_dir(), netreg.is_file())
        if net_ok:
            net_byte_ok = ((ntd / "bin" / "rurix_renderer.dll").read_bytes()
                           == (STAGE / "rurix_renderer.dll").read_bytes())
        note(f"网络 install GREEN exit={rn.returncode} 物化字节==源={net_byte_ok}")

        # RED③ 截断传输(环回半 body → curl 部分传输)。
        _TRUNCATE["rurix_renderer.dll"] = True
        home3 = WORK / "nethome_r3"
        reg3 = home3 / "toolchains.json"
        r3 = net_install(rurixup, SDK_VERSION, anchor, home3, reg3, loopback=True)
        _TRUNCATE.pop("rurix_renderer.dll", None)
        red_trunc_ok = (install_rejected_cleanly(r3.returncode, (home3 / "toolchains" / SDK_VERSION).exists(), reg3.is_file())
                        and has_kind(r3.stdout, "network") and _assert_no_staging(home3))
        note(f"RED③ 截断 → kind=network 干净拒装={red_trunc_ok}")

        # RED④ 清单篡改(channel_manifest 坏一字节 → 级①锚失配)。
        good_mf = (STAGE / "channel_manifest.json").read_bytes()
        mf_text = good_mf.decode("utf-8")
        tampered_text = mf_text.replace('"stable"', '"stablx"', 1)
        if tampered_text == mf_text:
            fail("RED④ 构造失败:channel_manifest 未含预期 stable 令牌")
            return 1
        (STAGE / "channel_manifest.json").write_bytes(tampered_text.encode("utf-8"))
        home4 = WORK / "nethome_r4"
        reg4 = home4 / "toolchains.json"
        r4 = net_install(rurixup, SDK_VERSION, anchor, home4, reg4, loopback=True)
        red_mf_ok = (install_rejected_cleanly(r4.returncode, (home4 / "toolchains" / SDK_VERSION).exists(), reg4.is_file())
                     and has_kind(r4.stdout, "integrity"))
        (STAGE / "channel_manifest.json").write_bytes(good_mf)  # 复原
        note(f"RED④ 清单篡改 → 级①锚失配 kind=integrity 干净拒装={red_mf_ok}")
    finally:
        httpd.shutdown()
        httpd.server_close()
    # 端点不可达(fixture 已关)。
    home5 = WORK / "nethome_r5"
    reg5 = home5 / "toolchains.json"
    r5 = net_install(rurixup, SDK_VERSION, anchor, home5, reg5, loopback=True)
    red_unreach_ok = (install_rejected_cleanly(r5.returncode, (home5 / "toolchains" / SDK_VERSION).exists(), reg5.is_file())
                      and has_kind(r5.stdout, "network"))
    note(f"不可达 → kind=network 诚实报错={red_unreach_ok}")
    set_fact(
        "network_install_ok",
        net_ok and net_byte_ok,
        f"hermetic 环回 HTTP 全链 install GREEN={net_ok}(127.0.0.1 随机端口 fixture,零真实外呼)"
        f"物化 rurix_renderer.dll 逐字节==源={net_byte_ok}",
    )

    # ── RED① 签名错(发布阻断)+ RED② 哈希错(级④内容寻址拒)──
    rr = do_release(rurixup, WORK / "rel_red_sig", sign_valid=False)
    tok_r = release_tokens(rr.stdout)
    red_sig_ok = release_blocked_with_gate(rr.returncode, tok_r, "signing")
    note(f"RED① 签名错 → release exit={rr.returncode} failed_gates={tok_r.get('failed_gates')}(发布阻断)={red_sig_ok}")

    home_r = WORK / "home_red"
    reg_r = home_r / "toolchains.json"
    good_dll = (STAGE / "rurix_renderer.dll").read_bytes()
    bad = bytearray(good_dll)
    bad[100] ^= 0xFF
    (STAGE / "rurix_renderer.dll").write_bytes(bytes(bad))
    rh = run([str(rurixup), "install", "--from-dir", str(STAGE),
              "--registry", str(reg_r)], env=install_env(home_r))
    red_hash_ok = (install_rejected_cleanly(rh.returncode, (home_r / "toolchains" / SDK_VERSION).exists(), reg_r.is_file())
                   and has_kind(rh.stdout, "integrity") and _assert_no_staging(home_r))
    (STAGE / "rurix_renderer.dll").write_bytes(good_dll)  # 复原
    note(f"RED② 哈希错 → 级④内容寻址拒 kind=integrity 零半装={red_hash_ok}")

    # 复原绿(红绿闭合)。
    home_re = WORK / "home_restore"
    reg_re = home_re / "toolchains.json"
    rre = run([str(rurixup), "install", "--from-dir", str(STAGE),
               "--registry", str(reg_re)], env=install_env(home_re))
    restore_ok = install_succeeded(rre.returncode, (home_re / "toolchains" / SDK_VERSION).is_dir(), reg_re.is_file())
    set_fact(
        "red_arms_closed",
        red_sig_ok and red_hash_ok and red_trunc_ok and red_mf_ok and red_unreach_ok and restore_ok,
        f"①签名错 release 阻断 failed_gates=[signing]={red_sig_ok};②哈希错 kind=integrity "
        f"零半装={red_hash_ok};③截断 kind=network={red_trunc_ok};④清单篡改 kind=integrity={red_mf_ok};"
        f"不可达 kind=network={red_unreach_ok};复原绿={restore_ok}",
    )

    # ── 离线可建验证:干净目录(仅已装 bundle + MSVC 公开工具链)──
    t0 = time.monotonic()
    clean = Path(tempfile.mkdtemp(prefix="rurix_sdk_clean_"))
    build_seconds = -1.0
    run_seconds = -1.0
    cl_ok = False
    host_tokens: dict = {}
    run_rc = -1
    try:
        chome = clean / "rurix_home"
        creg = chome / "toolchains.json"
        rc = run([str(rurixup), "install", "--from-dir", str(STAGE),
                  "--registry", str(creg)], env=install_env(chome))
        ctd = chome / "toolchains" / SDK_VERSION
        if rc.returncode != 0 or not ctd.is_dir():
            fail(f"干净目录 install 未成功(exit={rc.returncode})——离线可建腿前置破缺")
            return 1
        cl = locate_msvc()
        example_src = ctd / "examples" / "renderer_sdk_host.cpp"
        exe = clean / "minimal_host.exe"
        # 公开工具链面 = MSVC cl.exe + Windows SDK;输入面 = 已装 bundle(include/
        # bin/lib)+ 示例源(examples/)。毒化代理 env 见证零网络依赖。
        build_env = offline_env(msvc_env(install_env(chome), [ctd / "include"], [ctd / "bin" / "lib"]))
        pc = run(
            [str(cl), "/nologo", "/std:c++17", "/EHsc", str(example_src),
             f"/Fe:{exe}", f"/Fo:{clean}\\",
             "/link", "rurix_renderer.lib", "rurix_renderer_sdk.lib"],
            cwd=clean,
            env=build_env,
        )
        build_seconds = time.monotonic() - t0
        cl_ok = pc.returncode == 0 and exe.is_file()
        if not cl_ok:
            fail(f"干净目录 cl.exe 编译示例失败: {(pc.stdout + pc.stderr)[-600:]}")
        else:
            for d_ in ("rurix_renderer.dll", "rurix_renderer_sdk.dll"):
                (clean / d_).write_bytes((ctd / "bin" / d_).read_bytes())
            note(f"离线可建 ✓ 干净目录仅 bundle+MSVC 编出 minimal_host.exe(耗时 {build_seconds:.1f}s,毒化代理 env)")
        # ── 真跑(离屏 digest 验证;GPU 锁)──
        if cl_ok:
            t1 = time.monotonic()
            host_argv = [
                str(exe),
                "--contract", str(ctd / "manifests" / CONTRACT.name),
                "--gltf", str(BISTRO_GLTF),
                "--scene", SCENE,
                "--tier", str(TIER),
                "--spv-dir", str(ctd / "spv"),
                "--frames", str(FRAMES),
                "--warmup", str(WARMUP),
            ]
            with gpu_device_lock(purpose=f"{TAG} 干净目录示例 canonical 170 帧真跑"):
                hr = run(host_argv, cwd=clean, env=offline_env(dict(os.environ)), timeout=3600)
            run_seconds = time.monotonic() - t1
            run_rc = hr.returncode
            io.open(WORK / "clean_run.log", "w", encoding="utf-8", newline="\n").write(
                (hr.stdout or "") + (hr.stderr or ""))
            if hr.returncode in (2, 3):
                doc = {"schema": "rurix.g31.sdk_dist.skip.v1", "state": "DEV_ENV_DEGRADE",
                       "reasons": [f"干净目录宿主 rc={hr.returncode}(create/load 面 dev-env 缺失)"]}
                print(json.dumps(doc, ensure_ascii=False))
                if os.environ.get("RURIX_REQUIRE_REAL") == "1":
                    print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但宿主 dev-env 降级(rc={hr.returncode})",
                          file=sys.stderr)
                    return 1
                note(f"SKIP DEV_ENV_DEGRADE(宿主 rc={hr.returncode},非 PASS 非 FAIL)")
                return 0
            host_tokens = parse_host_tokens(hr.stdout or "")
            note(f"干净目录示例真跑 exit={hr.returncode}(耗时 {run_seconds:.1f}s)")
    finally:
        shutil.rmtree(clean, ignore_errors=True)
    set_fact(
        "offline_build_ok",
        cl_ok and build_seconds >= 0.0,
        f"干净目录(仅已装 bundle + MSVC 公开工具链,无 rurixc/cargo)cl.exe 编示例 exit="
        f"{0 if cl_ok else '≠0'} 耗时 {build_seconds:.1f}s;毒化代理 env(HTTP_PROXY=127.0.0.1:9)"
        "全程零网络依赖",
    )

    anchors = json.loads(ANCHOR_PATH.read_text(encoding="utf-8")).get("anchors") or {}
    anchor_digest = (anchors.get(ANCHOR_CELL) or {}).get("last_frame_digest")
    digest_hit = digest_matches(host_tokens.get("digest"), anchor_digest)
    set_fact(
        "offline_run_digest_ok",
        run_rc == 0 and host_tokens.get("host_ok") and digest_hit and frame_time_ok(host_tokens),
        f"示例真跑 canonical {FRAMES}+{WARMUP} 末帧 digest {str(host_tokens.get('digest'))[:23]}… "
        f"vs Stage A 锚 {str(anchor_digest)[:23]}… {'位级 MATCH' if digest_hit else 'DRIFT(RED)'};"
        f"帧时 mean={host_tokens.get('frame_ms_mean')}ms p50={host_tokens.get('frame_ms_p50')}ms "
        f"n={host_tokens.get('frame_samples')};耗时 {run_seconds:.1f}s exit={run_rc}",
    )

    # ── EA1 链既有门回归(rurixup_dist_smoke 复跑)──
    reg1 = run(["py", "-3", "ci/rurixup_dist_smoke.py"], timeout=7200)
    set_fact(
        "ea1_regression_green",
        reg1.returncode == 0,
        f"ci/rurixup_dist_smoke.py 复跑 exit={reg1.returncode}(EA1 前半 --from-dir + 后半 "
        "hermetic 环回 HTTP 既有门零破坏;component_rel_path SDK 面扩展对既有组件形态行为不变)",
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
        "bundle": {
            "version": SDK_VERSION,
            "channel": SDK_CHANNEL,
            "component_count": len(EXPECTED_COMPONENTS),
            "components": EXPECTED_COMPONENTS,
            "deterministic_bytes_equal": bool(det_ok),
            "digest_closure": bool(closure_ok),
        },
        "signing_sbom": {
            "signed_dlls": SIGNED_DLLS,
            "sbom_spdx_covers": bool(spdx_ok),
            "sbom_cdx_covers": bool(cdx_ok),
            "vendor_runtime": VENDOR_RUNTIME,
        },
        "install": {
            "from_dir": {
                "exit_code": ri.returncode,
                "components": int(s.get("components", "-1")),
                "digest_levels_verified": int(s.get("digest_levels_verified", "-1")),
                "layout_verified": bool(layout_ok and byte_ok),
                "registry_v2": bool(reg_v2),
            },
            "switch": {
                "exit_code": rd.returncode,
                "default": d_tok.get("default", ""),
                "list_verify_exit": lv.returncode,
            },
            "idempotent": {
                "registered": int(s2.get("registered", "-1")),
                "registry_bytes_equal": bool(idem_ok),
                "atomic_write_clean": bool(atomic_clean),
            },
            "network": {
                "exit_code": 0 if net_ok else 1,
                "loopback_only": True,
                "byte_equal_source": bool(net_byte_ok),
            },
        },
        "offline_build": {
            "example": HOST_CPP.name,
            "cl_exit": 0 if cl_ok else 1,
            "build_seconds": round(build_seconds, 3),
            "network_isolation": "poisoned_proxy_env",
            "run_exit": run_rc,
            "run_seconds": round(run_seconds, 3),
            "scene": SCENE,
            "tier": TIER,
            "frames": FRAMES,
            "warmup": WARMUP,
            "frame_ms_mean": host_tokens.get("frame_ms_mean", -1.0),
            "frame_ms_p50": host_tokens.get("frame_ms_p50", -1.0),
            "last_frame_digest": host_tokens.get("digest") or ("sha256:" + "0" * 64),
            "digest_matches_stage_a_anchor": bool(digest_hit),
        },
        "red_arms": {
            "signature": {"release_exit": rr.returncode, "failed_gate": "signing"},
            "hash": {"kind": "integrity", "zero_residue": bool(red_hash_ok)},
            "truncation": {"kind": "network", "zero_residue": bool(red_trunc_ok)},
            "manifest_tamper": {"kind": "integrity"},
            "unreachable": {"kind": "network"},
            "restore_green": bool(restore_ok),
        },
        "regression": {"rurixup_dist_smoke_exit": reg1.returncode},
        "facts": [facts[fid] for fid in FACT_IDS],
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G37 W5 渲染器 SDK 分发打包 v2(G31+ C5 门 16→24 组件冻结面版本化扩面,"
            "sdk-1.0.0→sdk-1.1.0):24 组件预编译 bundle(SDK 两层 DLL + 生成头 + import "
            "lib ×2 + canonical SPV 四件套 + G37 新臂 SPV 四件〔transp 工件谱系锚定"
            "35983d0f/ris/lut/skin 现编〕+ bistro 生产契约 + 示例工程源 + 文档五件 + 许可"
            "义务四件〔LICENSE-MIT/LICENSE-APACHE/THIRD_PARTY_NOTICES/内嵌第三方 SBOM,"
            "GAP-01~03 闭合,release.yml 同口径〕)经 EA1 rurixup 链(channel=stable 既有面;"
            "component_rel_path 既有映射 0-byte,无后缀许可文本件按「其余」律落 bin/)——"
            "release 编排(签名两 DLL selftest + SBOM 双视图覆盖 24 组件 + vendor 运行件"
            "技术对账三件)→ --from-dir 四级校验物化 → default 切换 + list --verify + 幂等 "
            "→ hermetic 环回 HTTP 网络 install(零真实外呼)→ 干净目录仅 bundle+MSVC 离线"
            "构建示例(毒化代理 env 见证)→ 真跑 canonical 160+10 末帧 digest 对拍 Stage A "
            f"锚 {'MATCH' if digest_hit else 'DRIFT'}。红臂四路(签名错/哈希错/截断/清单篡改)"
            f"+ 不可达 + 复原绿闭合。facts: "
            + "; ".join(f["id"] + "=" + f["status"] for f in (facts[fid] for fid in FACT_IDS))
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
        gate_path = ROOT / "evidence" / f"g31_sdk_dist_v2_{ts}.json"
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

    # 红绿臂①:签名错红臂判据(release 发布阻断)。
    tok_sig = {"allow_upload": "false", "failed_gates": "[signing]"}
    expect(release_blocked_with_gate(2, tok_sig, "signing"), "GREEN:exit 2 + failed_gates=[signing] 判红")
    expect(not release_blocked_with_gate(0, {"allow_upload": "true", "failed_gates": "[]"}, "signing"),
           "RED:放行被误判为签名阻断(门过松吞绿)")
    expect(not release_blocked_with_gate(2, {"allow_upload": "false", "failed_gates": "[sbom]"}, "signing"),
           "RED:sbom 子门红被误判为签名红(子门串扰,门失效)")
    expect(not release_blocked_with_gate(1, tok_sig, "signing"), "RED:用法错误 exit 1 不算发布阻断")
    # 红绿臂②:哈希错/截断/清单篡改红臂判据(EA1 同型拒装 + kind token)。
    expect(install_rejected_cleanly(1, False, False), "GREEN:干净拒装(exit 1 + 零残留)判红")
    expect(not install_rejected_cleanly(0, True, True), "RED:成功安装被误判拒装(门过松)")
    expect(not install_rejected_cleanly(1, True, False), "RED:半装泄漏不算干净拒装")
    expect(install_succeeded(0, True, True), "GREEN:成功判据正例")
    expect(not install_succeeded(1, True, True), "RED:退出非 0 被误判成功")
    expect(has_kind("RURIXUP_INSTALL_ERROR: kind=integrity\n", "integrity"), "GREEN:integrity token(哈希错/清单篡改臂)")
    expect(has_kind("RURIXUP_INSTALL_ERROR: kind=network\n", "network"), "GREEN:network token(截断/不可达臂)")
    expect(not has_kind("RURIXUP_INSTALL_ERROR: kind=integrity\n", "network"), "RED:kind 串扰拒判")
    expect(not has_kind("RURIXUP_INSTALL: version=sdk-1.0.0\n", "network"), "RED:成功摘要被误判含 network token")
    # 红绿臂③:bundle 组件闭集 + digest 闭环 + SBOM 覆盖。
    expect(component_set_ok(EXPECTED_COMPONENTS), "GREEN:24 组件闭集正例")
    expect(not component_set_ok(EXPECTED_COMPONENTS[:-1]), "RED:缺一件必红(闭集判据)")
    expect(not component_set_ok(list(EXPECTED_COMPONENTS) + ["extra.dll"]), "RED:多一件必红")
    expect(digests_match("ab" * 32, "ab" * 32), "GREEN:digest 相等正例")
    expect(not digests_match("ab" * 32, "cd" * 32), "RED:digest 不等必红")
    expect(not digests_match("", ""), "RED:空 digest 拒判")
    spdx_all = " ".join(EXPECTED_COMPONENTS) + " " + SDK_VERSION
    expect(sbom_covers(spdx_all, EXPECTED_COMPONENTS, SDK_VERSION), "GREEN:SBOM 覆盖正例")
    expect(not sbom_covers(" ".join(EXPECTED_COMPONENTS[:-1]) + " " + SDK_VERSION,
                           EXPECTED_COMPONENTS, SDK_VERSION), "RED:SBOM 缺组件必红")
    expect(not sbom_covers(" ".join(EXPECTED_COMPONENTS), EXPECTED_COMPONENTS, "sdk-9.9.9"),
           "RED:SBOM 版号漂移必红")
    # 红绿臂④:SDK 布局映射(镜像 install.rs component_rel_path 扩展)。
    rel = expected_rel_paths(EXPECTED_COMPONENTS)
    expect(rel["rurix_renderer.dll"] == "bin/rurix_renderer.dll", "GREEN:DLL→bin/")
    expect(rel["rurix_renderer.lib"] == "bin/lib/rurix_renderer.lib", "GREEN:import lib→bin/lib/")
    expect(rel["rurix_renderer.h"] == "include/rurix_renderer.h", "GREEN:生成头→include/")
    expect(rel["g14_mv.spv"] == "spv/g14_mv.spv", "GREEN:SPV→spv/")
    expect(rel["g13_ue_upscale_parity_contract.json"] == "manifests/g13_ue_upscale_parity_contract.json",
           "GREEN:契约→manifests/")
    expect(rel["renderer_sdk_host.cpp"] == "examples/renderer_sdk_host.cpp", "GREEN:示例源→examples/")
    expect(rel["integration_guide.md"] == "docs/integration_guide.md", "GREEN:文档→docs/")
    expect(not rel["rurix_renderer.lib"].startswith("bin/rurix_renderer.lib"), "RED:.lib 不落 bin/ 干名(EA1 既有律)")
    # G37 W5 扩面映射(install.rs 既有后缀律 0-byte:新 SPV→spv/,NOTICES→docs/,
    # cdx→manifests/,无后缀许可文本按「其余」律→bin/——如实登记非理想落位)。
    expect(rel["g31_realism_transp.spv"] == "spv/g31_realism_transp.spv", "GREEN:G37 SPV→spv/")
    expect(rel["g31_realism_ris.spv"] == "spv/g31_realism_ris.spv", "GREEN:RIS SPV→spv/")
    expect(rel["THIRD_PARTY_NOTICES.md"] == "docs/THIRD_PARTY_NOTICES.md", "GREEN:NOTICES→docs/(.md 律)")
    expect(rel["third_party_embedded.cdx.json"] == "manifests/third_party_embedded.cdx.json",
           "GREEN:内嵌 SBOM→manifests/(.json 律)")
    expect(rel["LICENSE-MIT"] == "bin/LICENSE-MIT" and rel["LICENSE-APACHE"] == "bin/LICENSE-APACHE",
           "GREEN:无后缀许可文本→bin/(「其余」律,install.rs 0-byte 如实登记)")
    # G37 W5 组件 license 字面(release.yml 同口径)。
    expect(component_license("LICENSE-MIT") == "MIT"
           and component_license("LICENSE-APACHE") == "Apache-2.0"
           and component_license("rurix_renderer.dll") == "MIT OR Apache-2.0",
           "GREEN:license 字面分件(许可件如实 + 第一方 workspace 双许可)")
    # PREBUILT 锚形状(64 hex;锚破缺即工件谱系不可核)。
    expect(all(re.fullmatch(r"[0-9a-f]{64}", a) for _p, a in PREBUILT_SPV.values()),
           "GREEN:PREBUILT SPV 锚形状 64hex")
    # 红绿臂⑤:宿主 token + digest 对拍 + 三态。
    full = (
        "RXSDK_HOST_ABI=0x00010000\n"
        "RXSDK_HOST_FRAME mean=4.0123 p50=3.9876 n=160\n"
        f"RXSDK_HOST_DIGEST sha256:{'ab' * 32}\nRXSDK_HOST_OK\n"
    )
    tk = parse_host_tokens(full)
    expect(tk.get("host_ok") and frame_time_ok(tk), "GREEN:宿主 token 解析 + 帧时正例")
    expect(digest_matches(tk.get("digest"), "sha256:" + "ab" * 32), "GREEN:digest 位级相等")
    expect(not digest_matches(tk.get("digest"), "sha256:" + "cd" * 32), "RED:digest 不等必红(锚对拍)")
    expect(not digest_matches(None, "sha256:" + "ab" * 32), "RED:缺 digest 拒判")
    expect(not frame_time_ok(parse_host_tokens(full.replace("n=160", "n=159"))), "RED:样本数不足必红")
    expect(not frame_time_ok(parse_host_tokens(full.replace("mean=4.0123", "mean=0.0000"))),
           "RED:帧时零值必红")
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")
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
                "schema", "subject", "symbolic_gate_key", "wave", "bundle", "signing_sbom",
                "install", "offline_build", "red_arms", "regression", "facts",
                "environment", "timestamp", "notes",
            ]),
            "schema required 闭集互核(14 字段)",
        )
        expect(gs["properties"]["bundle"]["properties"]["version"]["const"] == SDK_VERSION,
               "bundle.version const 互核(sdk-1.1.0)")
        expect(gs["properties"]["bundle"]["properties"]["component_count"]["const"] == 24,
               "bundle.component_count const=24 互核")
        comp_enum = gs["properties"]["bundle"]["properties"]["components"]["items"]["enum"]
        expect(sorted(comp_enum) == EXPECTED_COMPONENTS, "bundle.components 枚举闭集互核(24)")
        expect(gs["properties"]["install"]["properties"]["from_dir"]["properties"]["components"]["const"] == 24,
               "from_dir.components const=24 互核")
        expect(gs["properties"]["install"]["properties"]["from_dir"]["properties"]["digest_levels_verified"]["const"] == 4,
               "from_dir.digest_levels_verified const=4 互核")
        expect(gs["properties"]["offline_build"]["properties"]["frames"]["const"] == FRAMES,
               "offline_build.frames const 互核")
        expect(gs["properties"]["offline_build"]["properties"]["network_isolation"]["const"] == "poisoned_proxy_env",
               "offline_build.network_isolation const 互核")
        expect(gs["properties"]["red_arms"]["properties"]["signature"]["properties"]["failed_gate"]["const"] == "signing",
               "red_arms.signature.failed_gate const 互核")
        expect(gs["properties"]["regression"]["properties"]["rurixup_dist_smoke_exit"]["const"] == 0,
               "regression exit const 互核")
        vr = gs["properties"]["signing_sbom"]["properties"]["vendor_runtime"]
        expect(vr["minItems"] == 3 and vr["maxItems"] == 3, "vendor_runtime 基数 = 3 互核")
        expect("dynamic-load-not-bundled" in vr["items"]["properties"]["linkage"]["enum"]
               and "static-in-dll" in vr["items"]["properties"]["linkage"]["enum"],
               "vendor_runtime.linkage 枚举互核(动态装载不捆绑/静态入 DLL)")
        fact_enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(fact_enum) == sorted(FACT_IDS), "facts id 枚举闭集互核(9)")
        expect(gs["properties"]["facts"]["minItems"] == 9
               and gs["properties"]["facts"]["maxItems"] == 9, "facts 基数 = 9 互核")
    expect(len(FACT_IDS) == 9, "facts 闭集 = 9")
    expect(len(EXPECTED_COMPONENTS) == 24, "组件闭集 = 24(16 v1 + 4 G37 SPV + 4 许可件)")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=9;5 红臂组 + 正例组 + schema v2 互核,组件闭集 24)")
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
