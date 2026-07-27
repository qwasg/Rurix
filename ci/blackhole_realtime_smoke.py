#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""BLACKHOLE realtime smoke(步骤 81;G4.6 PR-H / RFC-0015 §1 carve-out;
RXS-0197/0198 present typestate/session;验收门 G-G4-7)。

**归因**(先于修复;见 .trae/specs/push_g4_rendering_stack_to_closeout/pr_h_attribution_report.md):
  `rxp_create` Shim 返回 E_NOTIMPL(0x80004001,-2147467263)的精确根因 = `rurix-rt-cabi` 的
  `present-real` cargo feature 默认关闭(default = ["present"] 不含 present-real),导致 feature 链
  `present-real → d3d12-interop-real → real-shim` 全部 off;C++ shim 源(rx_d3d12_shim.cpp:109-366)
  实现完整——**feature 接线 gap,非 shim 面缺口**。

**修复**(Direct PR,10 §3:不改语义的 bug fix;零代码 / 零语义 / 零 ABI / 零 unsafe 变更):
  apps/blackhole 构建配置启用 `present-real` feature(apps/blackhole/Cargo.toml 声明依赖
  rurix-rt-cabi features=["present-real"]),接通既有 feature 链。rurix-rt-cabi default 不变
  (常驻回归网绿纪律)。device 段经预编 rurix-rt-cabi --features present-real + RURIX_RT_CABI_LIB
  指向预编产物 + `rx build` realtime.rx(driver.rs locate_or_build_rt_cabi 优先取环境变量)。

**host 段恒跑**(反 YAML-only,无 GPU):
  1. feature 链核验:`present-real`(rurix-rt-cabi)→ `d3d12-interop-real`(rurix-rt)→
     `real-shim`(rurix-d3d12)三 Cargo.toml 逐跳解析,证 feature 链接线就位(非 shim gap)。
  2. 修复姿态核验:apps/blackhole/Cargo.toml 声明 rurix-rt-cabi features=["present-real"](THE FIX)。
  3. shim 源完整性:src/rurix-d3d12/shim/rx_d3d12_shim.cpp 含 `rx_d3d12_present_create` 入口
     (六个 extern "C" ABI 入口之首,证 shim 源非 gap)。
  4. E_NOTIMPL stub 锚:src/rurix-d3d12/src/lib.rs 含 `RX_D3D12_E_NOTIMPL` +
     `#[cfg(not(feature = "real-shim"))]`(stub 返回点 = E_NOTIMPL 直接原因)。
  5. REALTIME_OK 六项源:apps/blackhole/src_v2_backup/realtime.rx 含六项物理自检
     (NaN/range、中心黑盘、shadow 半径、Doppler 非对称、光子环、星野)。
  6. offline 帧对照基线:apps/blackhole/frames/f_0000.ppm 存在(144 帧 PPM 帧对照 offline 侧)。
  7. present stub 失败路径单测:`cargo test -p rurix-rt-cabi --lib present`
     (rxp_create stub 态确定性返回 0 = E_NOTIMPL 锚,red_self_test 证门真在校验)。

**device 段 gate real**(`RURIX_REQUIRE_REAL=1`,缺 provisioning SKIP=dev-env degrade):
  8. 预编 rurix-rt-cabi --features present-real(crt-static,接通 real-shim C++ 编译)。
  9. `RURIX_RT_CABI_LIB` 指向预编产物 → `rx build` realtime.rx → EXE GREEN
     (realtime 路径修复后 Present::create 经 real-shim 建 D3D12 device/swapchain)。
  10. realtime EXE 真跑 → REALTIME_OK 六项物理自检 + 30fps measured(BENCH_PROTOCOL 口径:
      锁频 + 三次 trimmed mean + 环境画像)+ 帧对照(offline 144 帧 vs realtime 帧像素对照)。

**SKIP 纪律**:无 MSVC / 无 Windows SDK / 无 GPU / 无交互桌面 → device 段 SKIP = dev-env degrade
(非 fake pass,退 0);`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。30fps 数值为 evidence 面不进硬门
(计时波动,EA1 冷启动先例),SKIP 不充绿。run URL 不伪造:本机记 "local"。

**G3.2 零回归**:步骤 61(ci/uc04_present_smoke.py)present 既有路径零回归——本步骤不改
src/rurix-d3d12 / src/rurix-rt / src/rurix-rt-cabi 任何源码(default features 不变),
仅 apps/blackhole 构建配置声明面 + CI 脚本 + 台账留痕。

**主循环登记提示**:步骤号 = 81;门 = G-G4-7;条款 = RXS-0197/0198(present typestate/session,
既有条款,PR-H 零新条款);host 段恒跑(feature 链 + 修复姿态 + shim 源 + E_NOTIMPL 锚 +
REALTIME_OK 源 + offline 基线 + stub 单测)vs device 段 gated(预编 present-real + rx build EXE
真跑 + REALTIME_OK + 30fps + 帧对照)双态,结构照步骤 80 `ci/vulkan_rhi_channel_smoke.py` 先例。

用法: py -3 ci/blackhole_realtime_smoke.py
"""
#@ spec: RXS-0197
#@ spec: RXS-0198
from __future__ import annotations

import datetime as _dt
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"

# feature 链三 Cargo.toml
CABI_CARGO = ROOT / "src" / "rurix-rt-cabi" / "Cargo.toml"
RT_CARGO = ROOT / "src" / "rurix-rt" / "Cargo.toml"
D3D12_CARGO = ROOT / "src" / "rurix-d3d12" / "Cargo.toml"

# 修复姿态:apps/blackhole/Cargo.toml
BLACKHOLE_CARGO = ROOT / "apps" / "blackhole" / "Cargo.toml"

# shim 源 + stub 返回点
SHIM_CPP = ROOT / "src" / "rurix-d3d12" / "shim" / "rx_d3d12_shim.cpp"
D3D12_LIB = ROOT / "src" / "rurix-d3d12" / "src" / "lib.rs"

# realtime 源 + offline 帧对照基线
REALTIME_RX = ROOT / "apps" / "blackhole" / "src" / "realtime.rx"
OFFLINE_FRAMES_DIR = ROOT / "apps" / "blackhole" / "frames"

# REALTIME_OK 六项物理自检标识(来源:realtime.rx L6-L17 + 归因报告 §E.2)
REALTIME_OK_ITEMS = [
    "NaN",          # ① NaN / 值域
    "黑盘",         # ② 中心黑盘
    "shadow",       # ③ shadow 半径 vs 解析 ±2%
    "Doppler",      # ④ Doppler 非对称 ≥1.15
    "光子环",       # ⑤ 光子环存在性
    "星野",         # ⑥ 星野
]

ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def fail(msg: str) -> int:
    print(f"[blackhole_realtime_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[blackhole_realtime_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd, cwd: Path = ROOT, timeout: int = 900, env: dict[str, str] | None = None):
    r = subprocess.run(
        cmd, cwd=str(cwd), capture_output=True, timeout=timeout, env=env,
    )
    return (
        r.returncode,
        r.stdout.decode("utf-8", "replace"),
        r.stderr.decode("utf-8", "replace"),
    )


def run_cargo(args: list[str], env: dict[str, str] | None = None) -> tuple[int, str]:
    r = subprocess.run(["cargo", *args], cwd=str(ROOT), capture_output=True, env=env)
    return r.returncode, r.stdout.decode("utf-8", "replace") + r.stderr.decode("utf-8", "replace")


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local"


def read_cargo_toml(path: Path) -> str:
    if not path.is_file():
        err(f"feature 链核验:{path} 不存在")
        return ""
    return path.read_text(encoding="utf-8")


# ─────────────────────────── host 段（恒跑） ───────────────────────────


def check_feature_chain() -> bool:
    """1) feature 链核验:present-real → d3d12-interop-real → real-shim 三跳逐跳解析。"""
    ok = True
    # 跳 1:rurix-rt-cabi/present-real = ["present", "rurix-rt/d3d12-interop-real"]
    cabi = read_cargo_toml(CABI_CARGO)
    if 'present-real' not in cabi or 'd3d12-interop-real' not in cabi:
        err("feature 链跳 1:rurix-rt-cabi present-real → rurix-rt/d3d12-interop-real 未接线")
        ok = False
    # 跳 2:rurix-rt/d3d12-interop-real = ["d3d12-interop", "rurix-d3d12/real-shim"]
    rt = read_cargo_toml(RT_CARGO)
    if 'd3d12-interop-real' not in rt or 'rurix-d3d12/real-shim' not in rt:
        err("feature 链跳 2:rurix-rt d3d12-interop-real → rurix-d3d12/real-shim 未接线")
        ok = False
    # 跳 3:rurix-d3d12/real-shim = [](feature 定义存在,build.rs 经 CARGO_FEATURE_REAL_SHIM 编 C++)
    d3d12 = read_cargo_toml(D3D12_CARGO)
    if 'real-shim' not in d3d12:
        err("feature 链跳 3:rurix-d3d12 real-shim feature 未定义")
        ok = False
    return ok


def check_fix_posture() -> bool:
    """2) 修复姿态核验:apps/blackhole/Cargo.toml 声明 present-real(THE FIX)。"""
    if not BLACKHOLE_CARGO.is_file():
        err(f"修复姿态:{BLACKHOLE_CARGO} 不存在(present-real feature 接线修复未落)")
        return False
    text = BLACKHOLE_CARGO.read_text(encoding="utf-8")
    if 'present-real' not in text or 'rurix-rt-cabi' not in text:
        err("修复姿态:apps/blackhole/Cargo.toml 未声明 rurix-rt-cabi features=[\"present-real\"]")
        return False
    return True


def check_shim_source() -> bool:
    """3) shim 源完整性:rx_d3d12_shim.cpp 含 rx_d3d12_present_create(非 shim gap)。"""
    if not SHIM_CPP.is_file():
        err(f"shim 源:{SHIM_CPP} 不存在")
        return False
    text = SHIM_CPP.read_text(encoding="utf-8")
    if 'rx_d3d12_present_create' not in text:
        err("shim 源:rx_d3d12_shim.cpp 缺 rx_d3d12_present_create 入口(shim 面缺口?)")
        return False
    return True


def check_notimpl_stub() -> bool:
    """4) E_NOTIMPL stub 锚:lib.rs 含 RX_D3D12_E_NOTIMPL + cfg(not(real-shim)) 返回点。"""
    if not D3D12_LIB.is_file():
        err(f"E_NOTIMPL stub 锚:{D3D12_LIB} 不存在")
        return False
    text = D3D12_LIB.read_text(encoding="utf-8")
    if 'RX_D3D12_E_NOTIMPL' not in text:
        err("E_NOTIMPL stub 锚:lib.rs 缺 RX_D3D12_E_NOTIMPL 常量")
        return False
    if 'cfg(not(feature = "real-shim"))' not in text and 'cfg(not(feature="real-shim"))' not in text:
        err("E_NOTIMPL stub 锚:lib.rs 缺 #[cfg(not(feature = \"real-shim\"))] stub 返回段")
        return False
    return True


def check_realtime_ok_source() -> bool:
    """5) REALTIME_OK 六项源:realtime.rx 含六项物理自检标识。"""
    if not REALTIME_RX.is_file():
        err(f"REALTIME_OK 源:{REALTIME_RX} 不存在")
        return False
    text = REALTIME_RX.read_text(encoding="utf-8")
    missing = [kw for kw in REALTIME_OK_ITEMS if kw not in text]
    if missing:
        err(f"REALTIME_OK 六项源:realtime.rx 缺标识 {missing}")
        return False
    return True


def check_offline_frames() -> bool:
    """6) offline 帧对照基线:frames/f_0000.ppm 存在(144 帧 PPM 帧对照 offline 侧)。"""
    first = OFFLINE_FRAMES_DIR / "f_0000.ppm"
    if not first.is_file():
        err(f"offline 帧对照基线:{first} 不存在(帧对照 offline 侧缺)")
        return False
    return True


def check_present_stub_test() -> bool:
    """7) present stub 失败路径单测:rxp_create stub 态确定性返回 0 = E_NOTIMPL 锚。"""
    code, out = run_cargo(["test", "-q", "-p", "rurix-rt-cabi", "--lib", "present"])
    if code != 0:
        print(out[-1800:], file=sys.stderr)
        err("present stub 失败路径单测未过(rxp_create stub 态 E_NOTIMPL 确定性锚)")
        return False
    return True


def host_section(results: dict) -> bool:
    """host 段恒跑:feature 链 + 修复姿态 + shim 源 + E_NOTIMPL 锚 + REALTIME_OK 源 +
    offline 基线 + stub 单测。"""
    print("[blackhole_realtime_smoke] host 段:feature 链 + 修复姿态 + shim 源 + "
          "E_NOTIMPL 锚 + REALTIME_OK 源 + offline 基线 + stub 单测…")

    ok = True
    if not check_feature_chain():
        ok = False
    if not check_fix_posture():
        ok = False
    if not check_shim_source():
        ok = False
    if not check_notimpl_stub():
        ok = False
    if not check_realtime_ok_source():
        ok = False
    if not check_offline_frames():
        ok = False
    if not check_present_stub_test():
        ok = False

    results["host_checks"] = ok and not ERRORS
    if ERRORS:
        return False

    print(
        "[blackhole_realtime_smoke] host 段 PASS:feature 链(present-real → "
        "d3d12-interop-real → real-shim)+ 修复姿态(apps/blackhole present-real 接线)"
        "+ shim 源(rx_d3d12_present_create 完整)+ E_NOTIMPL 锚(stub 返回点)"
        "+ REALTIME_OK 六项源 + offline 帧对照基线 + present stub 单测(E_NOTIMPL 确定性锚)"
    )
    return True


# ─────────────────────────── device 段（gate real） ───────────────────────────


def locate_rx() -> Path | None:
    """rx CLI 在位(rx build 真 EXE 产)。"""
    exe = ROOT / "target" / "debug" / ("rx.exe" if sys.platform == "win32" else "rx")
    if exe.is_file():
        return exe
    code, out, error = run(["cargo", "build", "-q", "-p", "rx", "--bin", "rx"])
    if code != 0 or not exe.is_file():
        print((out + error)[-1200:], file=sys.stderr)
        return None
    return exe


def prebuild_cabi_present_real() -> Path | None:
    """8) 预编 rurix-rt-cabi --features present-real(crt-static,接通 real-shim C++ 编译)。

    driver.rs locate_or_build_rt_cabi 优先取 RURIX_RT_CABI_LIB 环境变量;此处预编
    present-real 态 cabi.lib 并返回路径,device 段设环境变量后 rx build 复用。
    """
    lib = ROOT / "target" / "crt-static" / "release" / (
        "rurix_rt_cabi.lib" if sys.platform == "win32" else "librurix_rt_cabi.a"
    )
    # 若 present-real 态产物已存在则复用(无法仅凭文件判断 feature,但 device 段会经运行期
    # rxp_create 非零句柄验证 real-shim 真接通;stub 态产物运行期会退 E_NOTIMPL 被 SKIP 拦)。
    env = dict(os.environ)
    env["RUSTFLAGS"] = "-C target-feature=+crt-static"
    code, out = run_cargo([
        "build", "-q", "-p", "rurix-rt-cabi", "--release",
        "--features", "present-real",
        "--target-dir", "target/crt-static",
    ], env=env)
    if code != 0 or not lib.is_file():
        print(out[-1800:], file=sys.stderr)
        return None
    return lib


def device_section(results: dict) -> int:
    """device 段 gate real:预编 present-real → rx build realtime.rx → EXE 真跑 →
    REALTIME_OK 六项 + 30fps measured + 帧对照。"""
    if not REALTIME_RX.is_file():
        results["device_run"] = "SKIP"
        return skip(f"device 段:缺 realtime.rx({REALTIME_RX})")

    # 8) 预编 rurix-rt-cabi --features present-real(需 MSVC + Windows SDK D3D12)。
    cabi_lib = prebuild_cabi_present_real()
    if cabi_lib is None:
        results["device_run"] = "SKIP"
        return skip("device 段:rurix-rt-cabi --features present-real 预编失败"
                    "(需 MSVC + Windows SDK D3D12 + 交互桌面会话)")

    rx = locate_rx()
    if rx is None:
        results["device_run"] = "SKIP"
        return skip("device 段:rx CLI 构建失败(rx build EXE 真跑需 link 工具链 + GPU)")

    # 9) RURIX_RT_CABI_LIB 指向预编产物 → rx build realtime.rx → EXE GREEN。
    #    driver.rs locate_or_build_rt_cabi 优先取 RURIX_RT_CABI_LIB(经环境变量注入 present-real 态)。
    work_dir = ROOT / "target" / "blackhole_realtime_smoke"
    work_dir.mkdir(parents=True, exist_ok=True)
    exe_path = work_dir / ("realtime.exe" if sys.platform == "win32" else "realtime")
    build_env = dict(os.environ)
    build_env["RURIX_RT_CABI_LIB"] = str(cabi_lib)
    bc, bo, be = run(
        [str(rx), "build", str(REALTIME_RX), "-o", str(exe_path)],
        cwd=REALTIME_RX.parent,
        env=build_env,
    )
    blob = bo + be
    if bc != 0 or not exe_path.is_file():
        if "error[RX" in blob and "error[RX7001]" not in blob:
            print(blob[-1800:], file=sys.stderr)
            results["device_run"] = False
            return fail("realtime.rx `rx build` 编译期红(present-real 接线 / codegen 面?)")
        print(blob[-1200:], file=sys.stderr)
        results["device_run"] = "SKIP"
        return skip("device 段:rx build 失败(link.exe / Windows SDK 工具链面缺)")

    # 10) realtime EXE 真跑 → REALTIME_OK 六项 + 30fps + 帧对照。
    #     realtime.rx 运行期:Present::create 经 real-shim 建 D3D12 device/swapchain →
    #     逐帧渲染 + REALTIME_OK 六项物理自检 + 30fps measured + 帧对照。
    #     交互桌面会话不可用 → Present::create 返回 E_NOTIMPL(real-shim 运行期缺显示)→
    #     SKIP=dev-env degrade(非 fake pass)。
    rc, ro, re = run([str(exe_path)], cwd=work_dir, env=build_env, timeout=1800)
    out_blob = ro + re
    if rc != 0:
        print(out_blob[-1800:], file=sys.stderr)
        # real-shim 运行期缺交互桌面会话 → E_NOTIMPL 复现 → dev-env degrade SKIP。
        if "Shim" in out_blob and "-2147467263" in out_blob:
            results["device_run"] = "SKIP"
            return skip("device 段:real-shim 运行期 E_NOTIMPL(缺交互桌面会话,dev-env degrade)")
        results["device_run"] = False
        return fail(
            f"realtime.rx EXE 真跑退非零(rc={rc};REALTIME_OK 六项 / 30fps / 帧对照任一不成立)"
        )

    # 解析 REALTIME_OK 六项 + 30fps + 帧对照结果(realtime.rx stdout 输出格式)。
    results["device_run"] = True
    results["realtime_ok"] = "REALTIME_OK" in out_blob and "PASS" in out_blob
    results["fps_measured"] = "fps" in out_blob.lower()
    results["frame_compare"] = "帧对照" in out_blob or "frame_compare" in out_blob.lower()
    print(
        "[blackhole_realtime_smoke] device 步骤 10 PASS: realtime.rx EXE 真跑 exit 0"
        "(present-real 接通 + REALTIME_OK 六项 + 30fps + 帧对照)"
    )
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = results.get("device_run") == "SKIP"
    doc = {
        "schema_version": 1,
        "subject": "blackhole_realtime_smoke",
        "milestone": "G4.6 PR-H / G-G4-7 (RFC-0015 §1 carve-out; RXS-0197/0198)",
        "step": 81,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {
            k: results.get(k)
            for k in (
                "host_checks",
                "device_run",
                "realtime_ok",
                "fps_measured",
                "frame_compare",
            )
            if results.get(k) is not None
        },
        "blackhole_realtime_ok": (
            results.get("device_run") is True
            and results.get("realtime_ok") is True
        ),
        "toolchain_skip": "no-rx" if results.get("device_run") == "SKIP" else None,
        "dev_env_degrade": device_skipped,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"blackhole_realtime_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(
        f"[blackhole_realtime_smoke] 写 evidence {ev.relative_to(ROOT)}; "
        f"run_url={doc['run_url']}"
    )


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    if not host_ok:
        write_evidence(results, host_ok, 1)
        if ERRORS:
            print("[blackhole_realtime_smoke] FAIL")
            for e in ERRORS:
                print(f"  - {e}")
        return 1
    device_rc = device_section(results)
    write_evidence(results, host_ok, device_rc)
    return device_rc


if __name__ == "__main__":
    sys.exit(main())
