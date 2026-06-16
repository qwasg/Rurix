# -*- coding: utf-8 -*-
"""image-io 确定性图像序列输出门(M7.2,契约 D-M7-2;M7 CI_GATES 步骤 30,CPU-only)。

用法:
    py -3 ci/image_io_smoke.py             # 离线构建 + 两次落盘逐帧 SHA-256 一致 + 内建红绿
    py -3 ci/image_io_smoke.py --self-test  # 仅跑内建篡改红绿自检

门为 **check_* 守卫风格**(不写 evidence、不计 budget counter;spec/imageio.md §3):
  绿:image-io crate 经 `cargo build` 离线构建产 `imgio_repro`;同一固定输入两次
    落盘到不同目录,逐帧 content SHA-256 **逐字节一致**(RXS-0116/0117 确定性字节布局)。
  红(反 YAML-only,H06 D11.8-2):取一帧落盘字节,**篡改编码字节序**(逐像素 R/B
    通道交换)→ content SHA-256 必**改变**;若篡改后 SHA 不变(门无效)即脚本 FAIL。

失败即红(非零退出)。CI 步骤 30 调用本脚本(M7 CI_GATES §2.30 / §5.4)。
"""

import hashlib
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "build" / "image_io_smoke"
REPRO = ROOT / "target" / "debug" / ("imgio_repro.exe" if sys.platform == "win32" else "imgio_repro")


def fail(msg: str) -> None:
    print(f"[image_io_smoke] FAIL: {msg}")
    sys.exit(1)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def build_repro() -> None:
    r = run(["cargo", "build", "-p", "image-io", "--bin", "imgio_repro"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build image-io 失败:\n{r.stderr}")
    if not REPRO.exists():
        fail(f"imgio_repro 产物缺失: {REPRO}")


def run_repro(out_dir: Path) -> None:
    if out_dir.exists():
        for p in out_dir.glob("*.ppm"):
            p.unlink()
    out_dir.mkdir(parents=True, exist_ok=True)
    r = run([str(REPRO), str(out_dir)])
    if r.returncode != 0:
        fail(f"imgio_repro 退出码 {r.returncode}:\n{r.stdout}{r.stderr}")


def frame_shas(out_dir: Path) -> list[tuple[str, str]]:
    """逐帧 (文件名, content SHA-256) 按文件名排序。"""
    out = []
    for p in sorted(out_dir.glob("*.ppm")):
        out.append((p.name, hashlib.sha256(p.read_bytes()).hexdigest()))
    return out


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
    """篡改编码字节序:逐像素 R,G,B → B,G,R 通道交换(像素区,header 不动)。"""
    off = ppm_pixel_offset(data)
    head = data[:off]
    body = bytearray(data[off:])
    for i in range(0, len(body) - len(body) % 3, 3):
        body[i], body[i + 2] = body[i + 2], body[i]
    return bytes(head) + bytes(body)


def red_check() -> bool:
    """红:篡改一帧字节序后 content SHA-256 必改变。返回 True = 红验证通过(SHA 变了)。"""
    run_a = OUT_DIR / "run_a"
    run_repro(run_a)
    frames = sorted(run_a.glob("*.ppm"))
    if not frames:
        fail("imgio_repro 未落盘任何 .ppm 帧")
    data = frames[0].read_bytes()
    tampered = tamper_channel_order(data)
    if tampered == data:
        fail("篡改样本与原样相同(渐变帧应通道非对称,样本异常)")
    orig_sha = hashlib.sha256(data).hexdigest()
    tampered_sha = hashlib.sha256(tampered).hexdigest()
    return tampered_sha != orig_sha


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    build_repro()

    if mode == "--self-test":
        if not red_check():
            fail("红验证失败:篡改字节序后 content SHA-256 未变(门未真正校验,反 YAML-only)")
        print("[image_io_smoke] self-test PASS(篡改字节序 → content SHA-256 改变,门有效)")
        return

    run_a = OUT_DIR / "run_a"
    run_b = OUT_DIR / "run_b"
    run_repro(run_a)
    run_repro(run_b)
    shas_a = frame_shas(run_a)
    shas_b = frame_shas(run_b)
    if not shas_a:
        fail("imgio_repro 未落盘任何 .ppm 帧")
    if shas_a != shas_b:
        fail(f"两次落盘逐帧 content SHA-256 不一致(非确定性字节流):\n  A={shas_a}\n  B={shas_b}")

    # 内建红绿:篡改字节序 → content SHA-256 改变(红有效)。
    if not red_check():
        fail("红验证失败:篡改字节序后 content SHA-256 未变(门未真正校验,反 YAML-only)")

    print(
        f"[image_io_smoke] PASS(image-io 离线构建 + {len(shas_a)} 帧两次落盘 "
        f"content SHA-256 逐字节一致 / 红验证篡改字节序 SHA 改变)"
    )


if __name__ == "__main__":
    main()
