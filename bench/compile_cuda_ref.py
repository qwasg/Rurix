"""编译手写 CUDA C++ 对照 kernel → PTX(M5.3 WP8)。

nvcc -ptx -arch=sm_89 bench/cuda_ref/{reduce,scan,gemm_tile}.cu
→ bench/kernels/cuda_{name}.ptx

用法:
  py -3 bench/compile_cuda_ref.py

无 nvcc → SKIP(exit 0),不阻断 CI/开发环境。
"""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC_DIR = ROOT / "bench" / "cuda_ref"
OUT_DIR = ROOT / "bench" / "kernels"
KERNELS = ("reduce", "scan", "gemm_tile", "saxpy", "gemv")
ARCH = "sm_89"


def main() -> int:
    nvcc = shutil.which("nvcc")
    if not nvcc:
        print("[compile_cuda_ref] SKIP: nvcc 不在 PATH(无 CUDA Toolkit)")
        return 0

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for name in KERNELS:
        src = SRC_DIR / f"{name}.cu"
        dst = OUT_DIR / f"cuda_{name}.ptx"
        if not src.is_file():
            print(f"[compile_cuda_ref] FAIL: 源文件缺失 {src}", file=sys.stderr)
            return 1
        cmd = [nvcc, "-ptx", f"-arch={ARCH}", "-o", str(dst), str(src)]
        print(f"[compile_cuda_ref] $ {' '.join(cmd)}")
        proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
        if proc.returncode != 0:
            print(proc.stdout, file=sys.stderr)
            print(proc.stderr, file=sys.stderr)
            print(f"[compile_cuda_ref] FAIL: nvcc 编译 {name}.cu 失败", file=sys.stderr)
            return proc.returncode
        print(f"[compile_cuda_ref] OK: {dst.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
