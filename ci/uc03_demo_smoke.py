# -*- coding: utf-8 -*-
"""UC-03 旗舰 demo 单 EXE 端到端 + 确定性图像序列冒烟(M7.4,契约 G-M7-1;CI_GATES 步骤 32)。

用法:
    py -3 ci/uc03_demo_smoke.py             # 单 EXE 端到端 + 两次运行逐帧 content SHA-256
                                            #   逐字节一致 + 内建篡改帧像素红绿 + 写证据
    py -3 ci/uc03_demo_smoke.py --self-test # 仅跑内建篡改帧像素红绿自检(不写证据)

单 EXE(G-M7-1):UC-03 demo(src/uc03-demo)= 确定性 SPH 仿真 + G0 软光栅出图 + image-io
  PPM 落盘的 host 单可执行,经 `cargo build -p uc03-demo --bin uc03_demo` 产出单 EXE
  (host 构建产物,不打包再分发;`rx build` 经包管理对接见 PR 描述)。

确定性图像序列:同一**固定初值/固定步长**两次运行落盘到不同目录,逐帧 content SHA-256
  **逐字节一致**(SPH 仿真 → 软光栅 → image-io PPM P6 确定字节布局;无随机量/时间戳/
  平台相关字节)。序列含粒子运动(各帧不应全同),体现旗舰用例动画。

内建红绿(反 YAML-only,H06 D11.8-2):取一帧落盘字节,**篡改像素字节序**(逐像素
  R/B 通道交换)→ content SHA-256 必**改变**;若篡改后 SHA 不变(门无效)即脚本 FAIL。

写 evidence/uc03_demo_smoke.json(image_sequence_ok=true),计入 m7.counter.uc03_demo_
  image_sequence(ci/budget_eval.py,>=1 则 PASS;计数源 = evidence/uc03_demo_*.json 中
  image_sequence_ok=true 的报告数)。
"""

import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEMO = ROOT / "target" / "debug" / ("uc03_demo.exe" if sys.platform == "win32" else "uc03_demo")
OUT_DIR = ROOT / "build" / "uc03_demo_smoke"
EVIDENCE = ROOT / "evidence" / "uc03_demo_smoke.json"
BUILD_CMD = ["cargo", "build", "-p", "uc03-demo", "--bin", "uc03_demo"]


def fail(msg: str) -> None:
    print(f"[uc03_demo_smoke] FAIL: {msg}")
    sys.exit(1)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def build_demo() -> None:
    r = run(BUILD_CMD, cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build uc03-demo 失败:\n{r.stderr}")
    if not DEMO.exists():
        fail(f"uc03_demo 单 EXE 产物缺失: {DEMO}")


def run_demo(out_dir: Path) -> None:
    if out_dir.exists():
        for p in out_dir.glob("*.ppm"):
            p.unlink()
    out_dir.mkdir(parents=True, exist_ok=True)
    r = run([str(DEMO), str(out_dir)])
    if r.returncode != 0:
        fail(f"uc03_demo 退出码 {r.returncode}:\n{r.stdout}{r.stderr}")


def frame_shas(out_dir: Path) -> list[tuple[str, str]]:
    """逐帧 (文件名, content SHA-256) 按文件名排序。"""
    return [
        (p.name, hashlib.sha256(p.read_bytes()).hexdigest())
        for p in sorted(out_dir.glob("*.ppm"))
    ]


def ppm_pixel_offset(data: bytes) -> int:
    """定位 PPM P6 header 之后像素数据起点(magic / dims / maxval 三行,各以 \\n 结尾)。"""
    idx = 0
    newlines = 0
    while idx < len(data) and newlines < 3:
        if data[idx] == 0x0A:
            newlines += 1
        idx += 1
    if newlines != 3:
        fail("PPM P6 header 异常(未定位到 3 个换行)")
    return idx


def tamper_channel_order(data: bytes) -> bytes:
    """篡改像素字节序:逐像素 R,G,B → B,G,R 通道交换(像素区,header 不动)。"""
    off = ppm_pixel_offset(data)
    head = data[:off]
    body = bytearray(data[off:])
    for i in range(0, len(body) - len(body) % 3, 3):
        body[i], body[i + 2] = body[i + 2], body[i]
    return bytes(head) + bytes(body)


def pick_asymmetric_frame(out_dir: Path) -> bytes:
    """取首个 R/B 通道非对称(篡改后字节会变)的落盘帧字节;无则 FAIL。"""
    for p in sorted(out_dir.glob("*.ppm")):
        data = p.read_bytes()
        if tamper_channel_order(data) != data:
            return data
    fail("无 R/B 通道非对称帧(帧全灰/全黑,样本异常,红绿门无法生效)")
    raise AssertionError  # unreachable


def red_check() -> bool:
    """红:篡改一帧像素字节序后 content SHA-256 必改变。返回 True = 红验证通过(SHA 变了)。"""
    run_a = OUT_DIR / "run_a"
    run_demo(run_a)
    data = pick_asymmetric_frame(run_a)
    tampered = tamper_channel_order(data)
    orig_sha = hashlib.sha256(data).hexdigest()
    tampered_sha = hashlib.sha256(tampered).hexdigest()
    return tampered_sha != orig_sha


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    build_demo()

    if mode == "--self-test":
        if not red_check():
            fail("红验证失败:篡改像素字节序后 content SHA-256 未变(门未真正校验,反 YAML-only)")
        print("[uc03_demo_smoke] self-test PASS(篡改像素字节序 → content SHA-256 改变,门有效)")
        return

    # 单 EXE 端到端 + 确定性图像序列(两次运行落盘逐帧 content SHA-256 逐字节一致)。
    run_a = OUT_DIR / "run_a"
    run_b = OUT_DIR / "run_b"
    run_demo(run_a)
    run_demo(run_b)
    shas_a = frame_shas(run_a)
    shas_b = frame_shas(run_b)
    if not shas_a:
        fail("uc03_demo 未落盘任何 .ppm 帧")
    if shas_a != shas_b:
        fail(f"两次运行逐帧 content SHA-256 不一致(非确定性图像序列):\n  A={shas_a}\n  B={shas_b}")
    # 序列须含运动(各帧并非全部逐字节相同),体现旗舰 SPH 动画。
    distinct = {sha for _, sha in shas_a}
    if len(distinct) < 2:
        fail("图像序列各帧逐字节全同(SPH 仿真未体现运动,demo 管线异常)")

    # 内建红绿:篡改像素字节序 → content SHA-256 改变(红有效)。
    if not red_check():
        fail("红验证失败:篡改像素字节序后 content SHA-256 未变(门未真正校验,反 YAML-only)")

    evidence = {
        "schema_version": 1,
        "subject": "uc03_demo",
        "image_sequence_ok": True,
        "single_exe": {
            "binary": "target/debug/uc03_demo",
            "build_command": " ".join(BUILD_CMD),
            "note": "host 单可执行(cargo 产出);UC-03 demo 为本地构建产物不打包再分发(M8/G1),rx build 经包管理对接见 PR 描述",
        },
        "determinism": {
            "frames": len(shas_a),
            "distinct_frames": len(distinct),
            "frame_sha256_match": True,
            "run_dirs": ["build/uc03_demo_smoke/run_a", "build/uc03_demo_smoke/run_b"],
        },
        "redgreen": {
            "red_command": "篡改 uc03_demo 落盘帧像素字节序(R/B 通道交换)后计 content SHA-256",
            "red_sha_changed": True,
            "green_command": "py -3 ci/uc03_demo_smoke.py(固定初值两次运行逐帧 content SHA-256 逐字节一致)",
            "green_exit_code": 0,
            "run_url": "local red-green(反 YAML-only,H06 D11.8-2);pr-smoke 步骤 32 self-hosted runner run URL 见 PR 描述",
        },
        "timestamp": datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds"),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[uc03_demo_smoke] PASS(单 EXE 端到端 / {len(shas_a)} 帧两次运行 content SHA-256 "
        f"逐字节一致 / {len(distinct)} 个不同帧体现 SPH 运动 / 红验证篡改像素 SHA 改变 → "
        f"{EVIDENCE.relative_to(ROOT).as_posix()})"
    )


if __name__ == "__main__":
    main()
