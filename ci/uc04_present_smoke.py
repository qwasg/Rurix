#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-04 可见窗口 present smoke（步骤 61;G3.2 / RFC-0013 §4.A;RXS-0220~0222;验收门 G-G3-2）。

G2.4 步骤 48（ci/dxil_uc04_device_smoke.py）证 offscreen deferred draw + readback;本 smoke 证
**G3.2 present 面**:UC-04 deferred 渲染器接可见 win32 窗口 D3D12 flip-model swapchain 逐帧
present + resize 重建 + 逐帧 backbuffer readback 数值校验——

  host 段（**恒跑**,反 YAML-only）:
    1. present 装配核验单测（`src/uc04-demo/src/present.rs` RXS-0220~0222 accept/reject）+
       swapchain 重建协商纯 host helper（`src/rurix-rt/src/vk.rs` swapchain_present_action,RXS-0221）;
    2. typestate 编译面（`cargo build -p uc04-demo --features d3d12-runtime`,present FFI 声明面）;
    3. **内建 red_self_test**:篡改/缺 PRESENT 态迁移锚点 → present 装配核验拒（RX6027)——
       证装配门非空过（device 段的 debug layer 报错翻红同构,此处以 host 装配门等价见证）。

  device 段（**gate real-shim + GPU + 显示环境**;present 真跑 = 交互桌面人工链路,**不进
  pr-smoke 硬门**,镜像 uc07_present / realtime_present 双态先例):
    4. Rurix 源 → 图形=B DXIL（4 着色器,非手写 HLSL/DXIL）→ real-shim → 可见窗口 flip-model
       swapchain present N 帧,每帧 backbuffer RENDER_TARGET→COPY_SOURCE→PRESENT → Present(sync)
       逐帧 S_OK;`SetWindowPos` 合成 WM_SIZE → ResizeBuffers 重建;三点 backbuffer 中心像素回读
       （首帧 / 重建后首帧 / 末帧,RXS-0222）数值断言。

**SKIP 纪律（RXS-0222 L2）**:无显示/无 GPU/无 real-shim/未 opt-in → device 段 SKIP =
dev-env degrade（**非 fake pass**,退 0,打印 dev-env-degrade）;`RURIX_REQUIRE_REAL=1` 把缺失
翻**硬红**。present 真跑默认不弹窗（避免干扰桌面 / CI 无显示）——须显式 opt-in
`RURIX_UC04_PRESENT_DEVICE=1`(或 REQUIRE_REAL=1)才尝试 device 真跑。

**offscreen 不被替代**:步骤 48 offscreen 硬门 0-byte 不动,present 不得替代 offscreen 真跑
（RD-019 backfill_condition 原文）。run URL 不伪造:本机记 "local interactive runner"。
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
WORK = ROOT / "target" / "uc04_present_smoke"
EVIDENCE_DIR = ROOT / "evidence"
KNOWN_DXC_DIR = Path(r"H:\dxc-round7\extracted\bin\x64")
KNOWN_SPIRV_CROSS = Path(r"C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-cross.exe")

SHADERS = ["uc04_gbuffer_vs", "uc04_gbuffer_fs", "uc04_lighting_vs", "uc04_lighting_fs"]
SRC_DIR = ROOT / "conformance" / "dxil" / "graphics" / "accept"

# 呈现循环参数（smoke 默认;present 真跑帧数小,逐帧 readback 成本可忽略）。
FRAMES = 6
SYNC_INTERVAL = 1
RESIZE_FRAME = 3       # 第 3 帧注入 resize（RXS-0221 WM_SIZE 经 SetWindowPos 合成）
RESIZE_W = 320
RESIZE_H = 240


def fail(msg: str) -> int:
    print(f"[uc04_present_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg)
    print(f"[uc04_present_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd: list[str], *, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env)


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


# ─────────────────────────── host 段（恒跑） ───────────────────────────


def host_section() -> bool:
    """host 段恒跑:present 装配核验单测 + typestate 编译面 + red_self_test。全绿返回 True。"""
    # 1) present 装配/重建核验单测（RXS-0220~0222）+ vk.rs 重建协商纯 host helper（RXS-0221）。
    #    单 filter "present" 覆盖 present::tests::*（装配/重建 accept·reject）与
    #    device::tests::present_path_shim_unavailable_without_real_shim（均含子串 "present"）。
    p = run([
        "cargo", "test", "-p", "uc04-demo", "--features", "d3d12-runtime", "present",
    ])
    if p.returncode != 0:
        print((p.stdout + p.stderr)[-2000:], file=sys.stderr)
        print("[uc04_present_smoke] host 段 FAIL: uc04-demo present 单测未过", file=sys.stderr)
        return False
    p2 = run(["cargo", "test", "-p", "rurix-rt", "--features", "vulkan",
              "vk::tests::swapchain_present_action_classification"])
    if p2.returncode != 0:
        print((p2.stdout + p2.stderr)[-2000:], file=sys.stderr)
        print("[uc04_present_smoke] host 段 FAIL: vk.rs 重建协商 helper 单测未过", file=sys.stderr)
        return False

    # 2) typestate 编译面（present FFI 声明 + d3d12-runtime gate 面）。
    pb = run(["cargo", "build", "-p", "uc04-demo", "--features", "d3d12-runtime"])
    if pb.returncode != 0:
        print((pb.stdout + pb.stderr)[-2000:], file=sys.stderr)
        print("[uc04_present_smoke] host 段 FAIL: uc04-demo d3d12-runtime 编译面未过", file=sys.stderr)
        return False

    # 3) red_self_test:篡改/缺 PRESENT 态迁移锚点 → 装配门必须拒（RX6027）。
    #    以 present.rs `rejects_missing_present_transition` 为等价见证:该测构造缺
    #    COPY_SOURCE→PRESENT 锚点的请求,断言 assemble_present 拒 PresentAssembly（RX6027）。
    #    门若空过（不拒）→ 该测失败 → red_self_test 红（证门真在校验 PRESENT 迁移）。
    prt = run([
        "cargo", "test", "-p", "uc04-demo",
        "present::tests::rejects_missing_present_transition", "--", "--exact",
    ])
    if prt.returncode != 0 or "test result: ok. 1 passed" not in (prt.stdout + prt.stderr):
        print((prt.stdout + prt.stderr)[-2000:], file=sys.stderr)
        print("[uc04_present_smoke] host 段 FAIL: red_self_test（篡改 PRESENT 迁移 → 装配门须拒）未成立",
              file=sys.stderr)
        return False
    print("[uc04_present_smoke] host 段 PASS（present 装配/重建单测 + typestate 编译面 + "
          "red_self_test 篡改 PRESENT 迁移装配门拒）")
    return True


# ─────────────────────────── device 段（gated） ───────────────────────────


def locate_signed_dxc_dir() -> Path | None:
    dirs: list[Path] = []
    for key in ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"):
        v = os.environ.get(key)
        if v:
            dirs.append(Path(v))
    dirs.append(KNOWN_DXC_DIR)
    for d in dirs:
        if (d / "dxc.exe").is_file() and (d / "dxv.exe").is_file() and (d / "dxil.dll").is_file():
            return d
    return None


def locate_spirv_cross() -> Path | None:
    v = os.environ.get("RURIX_SPIRV_CROSS")
    if v and Path(v).is_file():
        return Path(v)
    if KNOWN_SPIRV_CROSS.is_file():
        return KNOWN_SPIRV_CROSS
    from shutil import which
    w = which("spirv-cross")
    return Path(w) if w else None


def tool_env(dxc_dir: Path, spirv_cross: Path) -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_DXC_DIR"] = str(dxc_dir)
    env["RURIX_DXC"] = str(dxc_dir / "dxc.exe")
    env["RURIX_SPIRV_CROSS"] = str(spirv_cross)
    return env


def emit_dxil(src: Path, out: Path, env: dict[str, str]) -> bool:
    p = run(
        ["cargo", "run", "-q", "-p", "rurixc", "--features", "dxil-backend shader-stages",
         "--example", "emit_uc04_dxil", "--", str(src), str(out)],
        env=env,
    )
    if p.returncode != 0 or not out.is_file():
        print((p.stdout + p.stderr)[-1600:], file=sys.stderr)
        return False
    return True


PRESENT_RE = re.compile(
    r"DXIL_UC04_PRESENT: ok adapter=\"(?P<adapter>[^\"]*)\" "
    r"frames_presented=(?P<fp>\d+) "
    r"first=(?P<fr>\d+),(?P<fg>\d+),(?P<fb>\d+),(?P<fa>\d+) "
    r"rebuilt=(?P<rr>\d+),(?P<rg>\d+),(?P<rb>\d+),(?P<ra>\d+) "
    r"last=(?P<lr>\d+),(?P<lg>\d+),(?P<lb>\d+),(?P<la>\d+) present=ok"
)


def device_section() -> int:
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    opt_in = require_real or os.environ.get("RURIX_UC04_PRESENT_DEVICE") == "1"
    if not opt_in:
        # present 真跑 = 交互桌面人工链路,默认不弹窗(不进 pr-smoke 硬门,镜像 uc07_present /
        # realtime_present 双态先例)。dev-env-degrade SKIP 退 0。
        print("[uc04_present_smoke] device 段 SKIP（present 真跑须显式 opt-in "
              "RURIX_UC04_PRESENT_DEVICE=1 或 RURIX_REQUIRE_REAL=1;present 交互桌面人工链路不进 "
              "pr-smoke 硬门,dev-env-degrade 退出 0）")
        return 0

    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip("未找到含 dxc.exe + dxv.exe + dxil.dll 的签名 DXC pin（图形=B 链必需）")
    spirv_cross = locate_spirv_cross()
    if spirv_cross is None:
        return skip("未找到 spirv-cross（图形=B 链必需）")
    env = tool_env(dxc_dir, spirv_cross)

    WORK.mkdir(parents=True, exist_ok=True)
    # 1) Rurix 源 → 图形=B DXIL 容器（4 着色器,非手写 HLSL/DXIL）。
    dxil: dict[str, Path] = {}
    for stem in SHADERS:
        out = WORK / f"{stem}.dxil"
        if not emit_dxil(SRC_DIR / f"{stem}.rx", out, env):
            return fail(f"cargo example emit_uc04_dxil 产 {stem} DXIL 失败（图形=B 链）")
        dxil[stem] = out

    # 2) 编译 real-shim（cc 编 D3D12 present shim;需 MSVC + Windows SDK D3D12）。
    pb = run(["cargo", "build", "-q", "-p", "uc04-demo", "--features", "real-shim"], env=env)
    if pb.returncode != 0:
        print((pb.stdout + pb.stderr)[-2400:], file=sys.stderr)
        return skip("cargo build -p uc04-demo --features real-shim 失败（需 MSVC + Windows SDK D3D12）")

    # 3) device present 真跑（可见窗口 flip-model swapchain + resize 重建 + 三点回读）。
    p = run(
        ["cargo", "run", "-q", "-p", "uc04-demo", "--features", "real-shim", "--",
         "present",
         str(dxil["uc04_gbuffer_vs"]), str(dxil["uc04_gbuffer_fs"]),
         str(dxil["uc04_lighting_vs"]), str(dxil["uc04_lighting_fs"]),
         str(FRAMES), str(SYNC_INTERVAL), str(RESIZE_FRAME), str(RESIZE_W), str(RESIZE_H)],
        env=env,
    )
    output = (p.stdout + p.stderr).strip()
    print(output)
    if "DXIL_UC04_PRESENT: skip ShimUnavailable" in output:
        return skip("present 入口 ShimUnavailable（无显示/real-shim 未编入;dev-env-degrade）")
    m = PRESENT_RE.search(output)
    if p.returncode != 0 or m is None:
        return fail("UC-04 可见窗口 present N 帧 / resize 重建 / readback 失败（green 路径）")

    fp = int(m.group("fp"))
    fr, rr, lr = int(m.group("fr")), int(m.group("rr")), int(m.group("lr"))
    adapter = m.group("adapter")
    # lighting FS 真采样 albedo（几何 FS 写常量 0.75 → gbuffer.R≈191 → backbuffer.R≈191）。
    if fp < FRAMES:
        return fail(f"frames_presented={fp} < 期望 {FRAMES}（Present 未逐帧 S_OK）")
    if not (185 <= fr <= 197):
        return fail(f"首帧 backbuffer 中心像素 R={fr} 不在期望 [185,197]")
    if not (185 <= rr <= 197):
        return fail(f"resize 重建后首帧 backbuffer 中心像素 R={rr} 不在期望 [185,197]（重建后再断言）")
    if not (185 <= lr <= 197):
        return fail(f"末帧 backbuffer 中心像素 R={lr} 不在期望 [185,197]")

    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    doc = {
        "schema_version": 1,
        "subject": "uc04_present_smoke",
        "present_ok": True,
        "milestone": "G3.2 / G-G3-2 (RFC-0013 §4.A; RXS-0220~0222)",
        "adapter": adapter,
        "frames_presented": fp,
        "present": {
            "swapchain": "flip-model FLIP_DISCARD, BufferCount=3, visible WS_OVERLAPPEDWINDOW window",
            "loop": "per-frame deferred (geom G-buffer MRT -> lighting sample albedo SRV -> backbuffer) "
                    "-> RENDER_TARGET->COPY_SOURCE->PRESENT -> Present(sync_interval)",
            "resize": f"SetWindowPos synth WM_SIZE @ frame {RESIZE_FRAME} -> ResizeBuffers -> rebuild RTV",
            "readback_points": ["first_frame", "after_rebuild", "last_frame"],
            "shaders_from_rurix_source": SHADERS,
            "offscreen_not_replaced": "step 48 (ci/dxil_uc04_device_smoke.py) offscreen hard gate 0-byte",
        },
        "pixels": {
            "first": [int(m.group("fr")), int(m.group("fg")), int(m.group("fb")), int(m.group("fa"))],
            "rebuilt": [int(m.group("rr")), int(m.group("rg")), int(m.group("rb")), int(m.group("ra"))],
            "last": [int(m.group("lr")), int(m.group("lg")), int(m.group("lb")), int(m.group("la"))],
        },
        "checks": {
            "visible_window_flip_model_present": True,
            "per_frame_present_s_ok": fp >= FRAMES,
            "resize_rebuild_readback_reasserted": 185 <= rr <= 197,
            "backbuffer_readback_numeric_compare": True,
            "readback_is_render_product_not_scanout": True,
        },
        "run_url": github_run_url(),
        "timestamp": _dt.datetime.now().astimezone().replace(microsecond=0).isoformat(),
        "stdout": output,
    }
    ev = EVIDENCE_DIR / "uc04_present_smoke.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[uc04_present_smoke] device 段 PASS adapter=\"{adapter}\" frames={fp} "
          f"first.R={fr} rebuilt.R={rr} last.R={lr}; 写 {ev.relative_to(ROOT)}; run_url={doc['run_url']}")
    return 0


def main() -> int:
    if not host_section():
        return 1
    return device_section()


if __name__ == "__main__":
    sys.exit(main())
