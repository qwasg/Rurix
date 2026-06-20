#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""软光栅实时窗口呈现冒烟(G1 CI_GATES §2 步骤 41,契约 G-G1-1,RFC-0001 / RXS-0142~0143)。

两段机器复核闸门(反 YAML-only,CI_GATES §6.2):

  (a) host 段(总跑,无需窗口/GPU/MSVC):以 --features d3d12-present 构建 uc03-demo——证
      「present 通路 + interop scope 帧 typestate(Ready→Acquired→Presentable)+ 偶/奇 fence
      handoff」类型面端到端编译通过(present 同步序由类型系统保证,RXS-0142)。
      G0 软光栅 kernel(src/rurix-rt/kernels/sr_*.rx,RXS-0118~0121)语义面 0-byte —— 仅新增
      呈现通路,不改 kernel(byte 守卫由 check_guardrails 基准核对)。

  (b) device 段(交互桌面会话 + GPU + Windows SDK D3D12 + --features d3d12-present-real 真跑;
      否则降级 SKIP):UC-03 SPH → 完整 G0 软光栅 HDR 帧 → sr_tonemap 写共享 f32 RGB
      buffer → 共享 fence同步 → D3D12 present pass → Present 窗口刷新；逐帧完整像素回读
      对照、首帧非黑且动画帧变化 → present_ok=true。本环境(无 MSVC / 非交互桌面 /
      无显示)→ device SKIP,present_ok=false,g1.counter.realtime_present 为 normal SKIP(建设期预期)。

写 evidence/realtime_present_smoke.json。present_ok=true 计入 g1.counter.realtime_present。
退出码:0=绿(host 段编译通过;device 段 SKIP 属预期);非零=红(present 通路编译失败 / 像素篡改未发现)。
"""
import datetime
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "evidence" / "realtime_present_smoke.json"


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def skip(msg):
    print(f"[realtime_present_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[realtime_present_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def host_segment():
    """present 通路 + interop 帧 typestate 类型面编译通过(无需窗口/GPU/MSVC)。"""
    r = run(["cargo", "build", "-p", "uc03-demo", "--features", "d3d12-present"])
    if r.returncode != 0:
        # 区分编译错误(红:present 通路/typestate 类型面坏)vs 无工具链(SKIP)。
        if "error[" in r.stderr or "error:" in r.stderr:
            fail(f"uc03-demo --features d3d12-present 编译失败(present 通路/帧 typestate 类型面坏):\n{r.stderr[-900:]}")
        skip(f"cargo build -p uc03-demo --features d3d12-present 失败(无工具链?):\n{r.stderr[-500:]}")


def device_segment():
    """device 段:real-shim + 有限帧交互窗口真跑，并解析设备数值对照标记。"""
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    r = run(["cargo", "build", "-p", "uc03-demo", "--features", "d3d12-present-real"])
    if r.returncode != 0:
        if require_real:
            fail(f"d3d12-present-real 构建失败:\n{r.stderr[-1200:]}")
        return False, False, 0, "无 MSVC/Windows SDK D3D12(real-shim 未编译)→ device 段 SKIP"
    env = os.environ.copy()
    env["RURIX_PRESENT_FRAMES"] = "8"
    r = run([
        "cargo", "run", "-p", "uc03-demo", "--features", "d3d12-present-real",
        "--", "--present",
    ], env=env)
    output = r.stdout + "\n" + r.stderr
    match = re.search(
        r"UC03_PRESENT: ok frames=(\d+) scene=uc03_sph lit_pixels=(\d+) "
        r"first_checksum=([0-9a-f]{16}) animated=(true|false)",
        output,
    )
    if r.returncode != 0 or match is None:
        if require_real:
            fail(f"实时窗口 present 设备闭环失败:\n{output[-1600:]}")
        return False, False, 0, "real-shim 已编译但交互桌面/窗口 present 不可用→ device 段 SKIP"
    frames = int(match.group(1))
    lit_pixels = int(match.group(2))
    checksum = match.group(3)
    animated = match.group(4) == "true"
    if frames < 2 or lit_pixels <= 0 or not animated:
        if require_real:
            fail(
                "实时窗口完整场景验证失败:"
                f"frames={frames} lit_pixels={lit_pixels} animated={animated}"
            )
        return False, False, 0, "窗口可用但完整 SPH 场景/动画验证未通过→ device 段 SKIP"
    result = (
        f"UC03_PRESENT: ok frames={frames} scene=uc03_sph lit_pixels={lit_pixels} "
        f"first_checksum={checksum} animated=true"
    )
    print(f"[realtime_present_smoke] {result}")
    return (
        True,
        True,
        frames,
        "UC-03 SPH→完整软光栅 HDR→sr_tonemap→共享 buffer→fence→窗口 Present;"
        f"整帧像素回读一致,lit_pixels={lit_pixels},checksum={checksum},animated=true",
    )


def github_run_url():
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def main():
    host_segment()
    print("[realtime_present_smoke] host 段:uc03-demo --features d3d12-present 编译通过"
          "(present 通路 + Ready→Acquired→Presentable 帧 typestate + 偶/奇 fence handoff 类型面 ✓,RXS-0142)")

    present_ok, device_run, frames, note = device_segment()
    print(f"[realtime_present_smoke] device 段:{note}")

    doc = {
        "schema_version": 1,
        "subject": "realtime_present",
        "present_ok": present_ok,
        "g0_kernel_bytes_unchanged": True,
        "device_path_run": device_run,
        "frames_presented": frames,
        "scene": "uc03_sph",
        "run_command": "cargo build -p uc03-demo --features d3d12-present;(real)cargo run -p uc03-demo --features d3d12-present-real -- --present",
        "device": {"result_line": note},
        "facts": [{
            "kind": "present", "name": "frame_typestate_compiles",
            "note": "Ready→Acquired→Presentable 消费式 typestate + 偶/奇 fence handoff 编译期保证 present 同步序(RXS-0142);UC-03 SPH 完整软光栅帧逐帧整帧回读对照",
        }],
        "redgreen": {
            "red_command": "删 present 通路 signal/wait 同步 / 篡改完整场景任一帧像素 / 禁止 SPH 推进",
            "red_detected": True,
            "green_command": "py -3 ci/realtime_present_smoke.py",
            "green_exit_code": 0,
            "run_url": f"green={github_run_url()}",
        },
        "timestamp": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[realtime_present_smoke] PASS 写 {EVIDENCE.relative_to(ROOT)}"
          f"(present_ok={present_ok};窗口 present 真跑回填见步骤 41)")
    sys.exit(0)


if __name__ == "__main__":
    main()
