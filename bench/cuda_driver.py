"""CUDA Driver API 最小 ctypes 封装(M0 harness 专用,非 Rurix 运行时)。

直调 nvcuda.dll,无 CUDA Toolkit 依赖;PTX 经 cuModuleLoadDataEx 驱动内 JIT,
JIT 日志缓冲常开(08 §2.4),版本不匹配自动降版重试(协商序列)。
"""
from __future__ import annotations

import ctypes
import re

CUDA_SUCCESS = 0
CUDA_ERROR_UNSUPPORTED_PTX_VERSION = 222

CU_JIT_INFO_LOG_BUFFER = 3
CU_JIT_INFO_LOG_BUFFER_SIZE_BYTES = 4
CU_JIT_ERROR_LOG_BUFFER = 5
CU_JIT_ERROR_LOG_BUFFER_SIZE_BYTES = 6

PTX_VERSION_LADDER = ["8.0", "7.8", "7.0"]  # 协商降版序列

_cuda = ctypes.WinDLL("nvcuda.dll")

# 显式 argtypes:防 ctypes 把 64 位指针/size_t 截成 c_int(Windows LLP64 陷阱)
_CUdeviceptr = ctypes.c_uint64
_cuda.cuMemAlloc_v2.argtypes = [ctypes.POINTER(_CUdeviceptr), ctypes.c_size_t]
_cuda.cuMemFree_v2.argtypes = [_CUdeviceptr]
_cuda.cuMemAllocHost_v2.argtypes = [ctypes.POINTER(ctypes.c_void_p), ctypes.c_size_t]
_cuda.cuMemFreeHost.argtypes = [ctypes.c_void_p]
_cuda.cuMemcpyHtoD_v2.argtypes = [_CUdeviceptr, ctypes.c_void_p, ctypes.c_size_t]
_cuda.cuMemcpyDtoH_v2.argtypes = [ctypes.c_void_p, _CUdeviceptr, ctypes.c_size_t]
_cuda.cuMemcpyDtoD_v2.argtypes = [_CUdeviceptr, _CUdeviceptr, ctypes.c_size_t]
_cuda.cuMemsetD8_v2.argtypes = [_CUdeviceptr, ctypes.c_ubyte, ctypes.c_size_t]
_cuda.cuStreamSynchronize.argtypes = [ctypes.c_void_p]
_cuda.cuModuleLoadDataEx.argtypes = [
    ctypes.POINTER(ctypes.c_void_p), ctypes.c_char_p, ctypes.c_uint,
    ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_void_p),
]
_cuda.cuLaunchKernel.argtypes = [
    ctypes.c_void_p,
    ctypes.c_uint, ctypes.c_uint, ctypes.c_uint,
    ctypes.c_uint, ctypes.c_uint, ctypes.c_uint,
    ctypes.c_uint, ctypes.c_void_p,
    ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_void_p),
]


class CudaError(RuntimeError):
    def __init__(self, fn: str, code: int, extra: str = ""):
        self.code = code
        name = ctypes.c_char_p()
        _cuda.cuGetErrorName(code, ctypes.byref(name))
        super().__init__(f"{fn} -> {name.value.decode() if name.value else code}{(' | ' + extra) if extra else ''}")


def check(fn: str, code: int, extra: str = "") -> None:
    if code != CUDA_SUCCESS:
        raise CudaError(fn, code, extra)


class Context:
    """cuInit + cuCtxCreate 的 RAII 封装(harness 单 context)。"""

    def __init__(self, device_index: int = 0):
        check("cuInit", _cuda.cuInit(0))
        self.device = ctypes.c_int()
        check("cuDeviceGet", _cuda.cuDeviceGet(ctypes.byref(self.device), device_index))
        self.handle = ctypes.c_void_p()
        check("cuCtxCreate", _cuda.cuCtxCreate_v2(ctypes.byref(self.handle), 0, self.device))

    def destroy(self) -> None:
        if self.handle:
            _cuda.cuCtxSynchronize()
            _cuda.cuCtxDestroy_v2(self.handle)
            self.handle = None

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.destroy()


def load_ptx(ptx_text: str) -> tuple[ctypes.c_void_p, str, str]:
    """装载 PTX,返回 (module, 实际使用的 .version, JIT info log)。

    协商序列(08 §2.4):UNSUPPORTED_PTX_VERSION 时按 PTX_VERSION_LADDER 降版重试。
    """
    last_error = None
    for version in PTX_VERSION_LADDER:
        text = re.sub(r"\.version\s+\d+\.\d+", f".version {version}", ptx_text, count=1)
        info_buf = ctypes.create_string_buffer(8192)
        err_buf = ctypes.create_string_buffer(8192)
        opt_keys = (ctypes.c_int * 4)(
            CU_JIT_INFO_LOG_BUFFER, CU_JIT_INFO_LOG_BUFFER_SIZE_BYTES,
            CU_JIT_ERROR_LOG_BUFFER, CU_JIT_ERROR_LOG_BUFFER_SIZE_BYTES,
        )
        opt_vals = (ctypes.c_void_p * 4)(
            ctypes.cast(info_buf, ctypes.c_void_p), 8192,
            ctypes.cast(err_buf, ctypes.c_void_p), 8192,
        )
        module = ctypes.c_void_p()
        code = _cuda.cuModuleLoadDataEx(
            ctypes.byref(module), text.encode(), 4, opt_keys, opt_vals
        )
        if code == CUDA_SUCCESS:
            return module, version, info_buf.value.decode(errors="replace")
        last_error = CudaError("cuModuleLoadDataEx", code,
                               f"version={version} jit_log={err_buf.value.decode(errors='replace')}")
        if code != CUDA_ERROR_UNSUPPORTED_PTX_VERSION:
            break
    raise last_error


def get_function(module: ctypes.c_void_p, name: str) -> ctypes.c_void_p:
    fn = ctypes.c_void_p()
    check("cuModuleGetFunction", _cuda.cuModuleGetFunction(ctypes.byref(fn), module, name.encode()))
    return fn


def mem_alloc(nbytes: int) -> ctypes.c_uint64:
    ptr = ctypes.c_uint64()
    check("cuMemAlloc", _cuda.cuMemAlloc_v2(ctypes.byref(ptr), ctypes.c_size_t(nbytes)))
    return ptr


def mem_free(ptr: ctypes.c_uint64) -> None:
    _cuda.cuMemFree_v2(ptr)


def mem_alloc_host(nbytes: int) -> ctypes.c_void_p:
    ptr = ctypes.c_void_p()
    check("cuMemAllocHost", _cuda.cuMemAllocHost_v2(ctypes.byref(ptr), ctypes.c_size_t(nbytes)))
    return ptr


def mem_free_host(ptr: ctypes.c_void_p) -> None:
    _cuda.cuMemFreeHost(ptr)


def memcpy_htod(dst: ctypes.c_uint64, src, nbytes: int) -> None:
    check("cuMemcpyHtoD", _cuda.cuMemcpyHtoD_v2(dst, src, ctypes.c_size_t(nbytes)))


def memcpy_dtoh(dst, src: ctypes.c_uint64, nbytes: int) -> None:
    check("cuMemcpyDtoH", _cuda.cuMemcpyDtoH_v2(dst, src, ctypes.c_size_t(nbytes)))


def memcpy_dtod(dst: ctypes.c_uint64, src: ctypes.c_uint64, nbytes: int) -> None:
    check("cuMemcpyDtoD", _cuda.cuMemcpyDtoD_v2(dst, src, ctypes.c_size_t(nbytes)))


def memset_d8(ptr: ctypes.c_uint64, value: int, nbytes: int) -> None:
    check("cuMemsetD8", _cuda.cuMemsetD8_v2(ptr, ctypes.c_ubyte(value), ctypes.c_size_t(nbytes)))


def stream_sync(stream: int = 0) -> None:
    check("cuStreamSynchronize", _cuda.cuStreamSynchronize(ctypes.c_void_p(stream)))


class EventPair:
    """CUDA Event 计时对(协议计时方式,r11 §1.4)。"""

    def __init__(self):
        self.start = ctypes.c_void_p()
        self.stop = ctypes.c_void_p()
        check("cuEventCreate", _cuda.cuEventCreate(ctypes.byref(self.start), 0))
        check("cuEventCreate", _cuda.cuEventCreate(ctypes.byref(self.stop), 0))

    def record_start(self) -> None:
        check("cuEventRecord", _cuda.cuEventRecord(self.start, None))

    def record_stop(self) -> None:
        check("cuEventRecord", _cuda.cuEventRecord(self.stop, None))

    def elapsed_ms(self) -> float:
        check("cuEventSynchronize", _cuda.cuEventSynchronize(self.stop))
        ms = ctypes.c_float()
        check("cuEventElapsedTime", _cuda.cuEventElapsedTime(ctypes.byref(ms), self.start, self.stop))
        return ms.value

    def destroy(self) -> None:
        _cuda.cuEventDestroy_v2(self.start)
        _cuda.cuEventDestroy_v2(self.stop)


def launch(fn: ctypes.c_void_p, grid: tuple[int, int, int], block: tuple[int, int, int],
           args: list) -> None:
    """cuLaunchKernel;args 为已构造的 ctypes 标量列表(按 kernel 参数顺序)。"""
    ptrs = (ctypes.c_void_p * len(args))(
        *[ctypes.cast(ctypes.byref(a), ctypes.c_void_p) for a in args]
    )
    check("cuLaunchKernel", _cuda.cuLaunchKernel(
        fn, grid[0], grid[1], grid[2], block[0], block[1], block[2],
        0, None, ptrs, None,
    ))
