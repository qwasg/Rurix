"""cublas v2 C API ctypes 薄封装(M8.2 bench;BENCH_PROTOCOL.md §3)。

直调 cublas runtime DLL(`cublas64_*.dll`,Attachment A 白名单最小集),供 cublas
GEMM/GEMV 性能基准 harness 复用——与 rurix-cublas crate(src/rurix-cublas)同一
cublas v2 调用与行主序 ↔ 列主序适配(RXS-0128),measure cublas kernel 吞吐作为
"绑定 kernel"的性能事实(01 §6 UC-01 判据:≥ 手写 CUDA C++ 90%)。

cublas 句柄绑定 current context——本模块在 bench/cuda_driver.py 的 cuCtxCreate
context 内创建句柄(句柄一次创建于 timed loop 外,仅计 cublasSgemm/Sgemv kernel 时间)。
"""
from __future__ import annotations

import ctypes
import os
from pathlib import Path

CUBLAS_OP_N = 0
CUBLAS_OP_T = 1
CUBLAS_STATUS_SUCCESS = 0

_CANDIDATES = ("cublas64_13.dll", "cublas64_12.dll", "cublas64_11.dll")


def _load() -> ctypes.CDLL:
    # 优先 CUDA_PATH\bin\x64(Attachment A 白名单最小集 runtime DLL),回落系统 PATH。
    cuda_path = os.environ.get("CUDA_PATH")
    for cand in _CANDIDATES:
        if cuda_path:
            for sub in ("bin/x64", "bin"):
                p = Path(cuda_path) / sub / cand
                if p.is_file():
                    return ctypes.CDLL(str(p))
        try:
            return ctypes.CDLL(cand)
        except OSError:
            continue
    raise OSError(f"找不到 cublas runtime DLL(候选 {_CANDIDATES};Attachment A 白名单)")


_lib = _load()

_lib.cublasCreate_v2.restype = ctypes.c_int
_lib.cublasCreate_v2.argtypes = [ctypes.c_void_p]
_lib.cublasDestroy_v2.restype = ctypes.c_int
_lib.cublasDestroy_v2.argtypes = [ctypes.c_void_p]
_lib.cublasSgemm_v2.restype = ctypes.c_int
_lib.cublasSgemm_v2.argtypes = [
    ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
    ctypes.c_void_p, ctypes.c_uint64, ctypes.c_int, ctypes.c_uint64, ctypes.c_int,
    ctypes.c_void_p, ctypes.c_uint64, ctypes.c_int,
]
_lib.cublasSgemv_v2.restype = ctypes.c_int
_lib.cublasSgemv_v2.argtypes = [
    ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int,
    ctypes.c_void_p, ctypes.c_uint64, ctypes.c_int, ctypes.c_uint64, ctypes.c_int,
    ctypes.c_void_p, ctypes.c_uint64, ctypes.c_int,
]


def loaded_dll_name() -> str:
    return getattr(_lib, "_name", "cublas64_?.dll")


class Handle:
    """cublasHandle_t RAII(创建于 current context)。"""

    def __init__(self) -> None:
        self.h = ctypes.c_void_p()
        st = _lib.cublasCreate_v2(ctypes.byref(self.h))
        if st != CUBLAS_STATUS_SUCCESS:
            raise RuntimeError(f"cublasCreate_v2 失败(status {st})")

    def destroy(self) -> None:
        if self.h:
            _lib.cublasDestroy_v2(self.h)
            self.h = ctypes.c_void_p()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.destroy()


_ALPHA = ctypes.c_float(1.0)
_BETA = ctypes.c_float(0.0)


def sgemm_row_major(h: Handle, c: int, a: int, b: int, m: int, n: int, k: int) -> int:
    """行主序 C[m,n]=A[m,k]·B[k,n]:cublasSgemm(OP_N,OP_N,n,m,k,B,n,A,k,C,n)(RXS-0128)。"""
    return _lib.cublasSgemm_v2(
        h.h, CUBLAS_OP_N, CUBLAS_OP_N, n, m, k,
        ctypes.byref(_ALPHA), b, n, a, k,
        ctypes.byref(_BETA), c, n,
    )


def sgemv_row_major(h: Handle, y: int, a: int, x: int, m: int, n: int) -> int:
    """行主序 y[m]=A[m,n]·x[n]:cublasSgemv(OP_T,n,m,A,n,x,1,y,1)(RXS-0128)。"""
    return _lib.cublasSgemv_v2(
        h.h, CUBLAS_OP_T, n, m,
        ctypes.byref(_ALPHA), a, n, x, 1,
        ctypes.byref(_BETA), y, 1,
    )
