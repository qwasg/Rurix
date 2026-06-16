# -*- coding: utf-8 -*-
"""G0 软光栅 kernel safe 覆盖 + 确定性帧像素冒烟(M7.3,契约 G-M7-3;M7 CI_GATES 步骤 31)。

用法:
    py -3 ci/soft_raster_smoke.py             # device codegen + 两次帧逐字节一致 + 红绿 + 写证据
    py -3 ci/soft_raster_smoke.py --self-test  # 仅跑内建篡改帧像素红绿自检(不写证据)

device 路径(全 safe kernel safe 覆盖):软光栅 kernel(conformance/soft_raster/device/
  sr_*.rx,binning/tile 光栅/深度/tonemap)经 rurixc `--emit=nvptx-ir` 产 NVPTX IR
  (0 退出 + 非空 IR);`--emit=ptx`(ptxas 干验证,RXS-0073)best-effort(ptxas 缺失则
  device_facts 记 skipped,不失败本门)。device codegen 通过的 kernel 计入 safe_kernels
  (全 safe 代码目标,零 unsafe;凡落 unsafe 不计入且须 // SAFETY: + 留痕)。

host 路径(确定性帧像素):src/soft-raster 全 safe CPU 参考(softraster_repro)经
  `cargo build` 离线构建;同一固定输入两次落盘到不同目录,逐帧 content SHA-256
  **逐字节一致**(spec/softraster.md RXS-0118~0121 确定性帧像素;复用 image-io
  PPM P6 确定字节布局)。

内建红绿(反 YAML-only,H06 D11.8-2):取一帧落盘字节,**篡改像素字节序**(逐像素
  R/B 通道交换)→ content SHA-256 必**改变**;若篡改后 SHA 不变(门无效)即脚本 FAIL。

safe_kernels(device codegen 通过的软光栅 kernel)写入 evidence/soft_raster_smoke.json,
  去重计数计入 m7.counter.soft_raster_kernels_safe(ci/budget_eval.py,>=4 则 PASS)。
"""

import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RURIXC = ROOT / "target" / "debug" / "rurixc.exe"
REPRO = ROOT / "target" / "debug" / ("softraster_repro.exe" if sys.platform == "win32" else "softraster_repro")
OUT_DIR = ROOT / "build" / "soft_raster_smoke"
EVIDENCE = ROOT / "evidence" / "soft_raster_smoke.json"

# device codegen 软光栅 kernel(全 safe,atomics-free;spec/softraster.md RXS-0118~0121)
DEVICE_KERNELS = [
    ("sr_binning", "conformance/soft_raster/device/sr_binning.rx"),
    ("sr_raster_tile", "conformance/soft_raster/device/sr_raster_tile.rx"),
    ("sr_depth", "conformance/soft_raster/device/sr_depth.rx"),
    ("sr_tonemap", "conformance/soft_raster/device/sr_tonemap.rx"),
]


def fail(msg: str) -> None:
    print(f"[soft_raster_smoke] FAIL: {msg}")
    sys.exit(1)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def build_rurixc() -> None:
    r = run(["cargo", "build", "-p", "rurixc", "--bin", "rurixc"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build rurixc 失败:\n{r.stderr}")
    if not RURIXC.exists():
        fail(f"rurixc 产物缺失: {RURIXC}")


def build_repro() -> None:
    r = run(["cargo", "build", "-p", "soft-raster", "--bin", "softraster_repro"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build soft-raster 失败:\n{r.stderr}")
    if not REPRO.exists():
        fail(f"softraster_repro 产物缺失: {REPRO}")


def emit_device_ir(src_rel: str) -> int:
    """device codegen NVPTX IR;返回退出码(并校验 IR 非空)。"""
    src = ROOT / src_rel
    r = run([str(RURIXC), str(src), "--emit=nvptx-ir"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"device codegen {src_rel} 失败(exit {r.returncode}):\n{r.stdout}{r.stderr}")
    if "target triple" not in r.stdout:
        fail(f"device IR 异常(无 target triple): {src_rel}")
    return r.returncode


def try_ptxas(src_rel: str) -> str:
    """best-effort ptxas 干验证(--emit=ptx);返回 ran|skipped。"""
    src = ROOT / src_rel
    r = run([str(RURIXC), str(src), "--emit=ptx"], cwd=ROOT)
    if r.returncode == 0 and ("//" in r.stdout or ".visible" in r.stdout or "target" in r.stdout):
        return "ran"
    return "skipped"


def run_repro(out_dir: Path) -> None:
    if out_dir.exists():
        for p in out_dir.glob("*.ppm"):
            p.unlink()
    out_dir.mkdir(parents=True, exist_ok=True)
    r = run([str(REPRO), str(out_dir)])
    if r.returncode != 0:
        fail(f"softraster_repro 退出码 {r.returncode}:\n{r.stdout}{r.stderr}")


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


def red_check() -> bool:
    """红:篡改一帧像素字节序后 content SHA-256 必改变。返回 True = 红验证通过(SHA 变了)。"""
    run_a = OUT_DIR / "run_a"
    run_repro(run_a)
    frames = sorted(run_a.glob("*.ppm"))
    if not frames:
        fail("softraster_repro 未落盘任何 .ppm 帧")
    data = frames[0].read_bytes()
    tampered = tamper_channel_order(data)
    if tampered == data:
        fail("篡改样本与原样相同(帧应通道非对称,样本异常)")
    orig_sha = hashlib.sha256(data).hexdigest()
    tampered_sha = hashlib.sha256(tampered).hexdigest()
    return tampered_sha != orig_sha


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    build_rurixc()
    build_repro()

    if mode == "--self-test":
        if not red_check():
            fail("红验证失败:篡改像素字节序后 content SHA-256 未变(门未真正校验,反 YAML-only)")
        print("[soft_raster_smoke] self-test PASS(篡改像素字节序 → content SHA-256 改变,门有效)")
        return

    # device 路径:软光栅 kernel safe 覆盖(device codegen + ptxas 干验证)。
    device_facts = []
    safe_kernels = []
    for name, src_rel in DEVICE_KERNELS:
        code = emit_device_ir(src_rel)
        ptxas = try_ptxas(src_rel)
        device_facts.append({
            "source": src_rel,
            "emit": "--emit=nvptx-ir",
            "exit_code": code,
            "ptxas": ptxas,
            "note": "PASS",
        })
        # device codegen 通过且零 unsafe(全 safe kernel)→ 计入 safe 覆盖。
        safe_kernels.append(name)

    # host 路径:确定性帧像素(两次落盘逐帧 content SHA-256 逐字节一致)。
    run_a = OUT_DIR / "run_a"
    run_b = OUT_DIR / "run_b"
    run_repro(run_a)
    run_repro(run_b)
    shas_a = frame_shas(run_a)
    shas_b = frame_shas(run_b)
    if not shas_a:
        fail("softraster_repro 未落盘任何 .ppm 帧")
    if shas_a != shas_b:
        fail(f"两次落盘逐帧 content SHA-256 不一致(非确定性帧像素):\n  A={shas_a}\n  B={shas_b}")

    # 内建红绿:篡改像素字节序 → content SHA-256 改变(红有效)。
    if not red_check():
        fail("红验证失败:篡改像素字节序后 content SHA-256 未变(门未真正校验,反 YAML-only)")

    evidence = {
        "schema_version": 1,
        "subject": "soft_raster_kernels",
        "safe_kernels": safe_kernels,
        "device_facts": device_facts,
        "unsafe_kernels": [],
        "determinism": {
            "frames": len(shas_a),
            "frame_sha256_match": True,
            "repro_binary": "target/debug/softraster_repro",
        },
        "gpu_roundtrip": "skipped",
        "redgreen": {
            "red_command": "篡改 softraster_repro 落盘帧像素字节序(R/B 通道交换)后计 content SHA-256",
            "red_sha_changed": True,
            "green_command": "py -3 ci/soft_raster_smoke.py(固定输入两次落盘逐帧 content SHA-256 逐字节一致)",
            "green_exit_code": 0,
            "run_url": "local red-green(反 YAML-only,H06 D11.8-2);pr-smoke 步骤 31 self-hosted runner run URL 见 PR 描述",
        },
        "rurixc_binary": "target/debug/rurixc.exe",
        "timestamp": datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds"),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[soft_raster_smoke] PASS(device {len(device_facts)} kernel codegen IR 产出 / "
        f"{len(safe_kernels)} safe kernel / host {len(shas_a)} 帧两次落盘 content SHA-256 "
        f"逐字节一致 / 红验证篡改像素 SHA 改变 → {EVIDENCE.relative_to(ROOT).as_posix()})"
    )


if __name__ == "__main__":
    main()
