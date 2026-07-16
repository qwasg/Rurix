# SPIKE(RD-010) — round-4 A 路互操作诊断:经 dxcompiler.dll IDxcValidator::Validate 真验证 llc 产 DXIL。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest:dxcompiler.dll/validator 可加载则记实测 status+错误原文,
# 不可加载(非 Windows / 缺 DLL / 接口缺)则如实 blocked,绝不杜撰。
"""IDxcValidator harness(纯 ctypes,无 comtypes 依赖)。

回答 round-4 互操作锐利诊断:dxc 1.8 的 validator 对 llc(LLVM DirectX 后端)产 DXIL 是
  - 接受+签名(只缺签名步 → A 互操作可打通),还是
  - 拒绝(validation error 非签名 → llc 产 DXIL 不合规 = 上游 backend 问题)。
经 DxcCreateInstance(CLSID_DxcValidator)→ IDxcValidator::Validate → IDxcOperationResult
读 HRESULT status + 错误 blob 原文。dxc -dumpbin 仅容器加载,非真 validator,故此处用真 API。
"""
from __future__ import annotations

import ctypes
from ctypes import POINTER, byref, c_size_t, c_uint32, c_void_p

UNAVAILABLE = "unavailable"


class _GUID(ctypes.Structure):
    _fields_ = [("a", c_uint32), ("b", ctypes.c_uint16), ("c", ctypes.c_uint16), ("d", ctypes.c_ubyte * 8)]


def _guid(a, b, c, *d) -> "_GUID":
    g = _GUID()
    g.a, g.b, g.c = a, b, c
    g.d = (ctypes.c_ubyte * 8)(*d)
    return g


CLSID_VALIDATOR = _guid(0x8CA3E215, 0xF728, 0x4CF3, 0x8C, 0xDD, 0x88, 0xAF, 0x91, 0x75, 0x87, 0xA1)
IID_VALIDATOR = _guid(0xA6E82BD2, 0x1FD7, 0x4826, 0x98, 0x11, 0x28, 0x57, 0xE7, 0x97, 0xF4, 0x9A)
CLSID_LIBRARY = _guid(0x6245D6AF, 0x66E0, 0x48FD, 0x80, 0xB4, 0x4D, 0x27, 0x17, 0x96, 0x74, 0x8C)
IID_LIBRARY = _guid(0xE5204DC7, 0xD18C, 0x4C3C, 0xBD, 0xFB, 0x85, 0x16, 0x73, 0x98, 0x0F, 0xE7)


def _vcall(obj, idx, restype, argtypes, *args):
    """经 COM vtable 索引调方法(idx:0=QI 1=AddRef 2=Release 3+=接口方法)。"""
    vtbl = ctypes.cast(obj, POINTER(c_void_p))[0]
    fnp = ctypes.cast(vtbl, POINTER(c_void_p))[idx]
    proto = ctypes.WINFUNCTYPE(restype, c_void_p, *argtypes)
    return proto(fnp)(obj, *args)


def validate_container(dll_path: str, container: bytes) -> dict:
    """对 DXContainer 字节调 IDxcValidator::Validate。返回结构化结果(不抛)。

    accepted=True 仅当 validator status HRESULT==S_OK(0);否则 rejected + 错误原文 + status 码。
    dxcompiler.dll / validator / 接口任一不可用 → status='blocked' + reason(blocked-honest)。
    """
    try:
        dll = ctypes.WinDLL(dll_path)
    except OSError as e:
        return {"status": "blocked", "reason": f"load_dll_failed:{e}"}

    try:
        create = dll.DxcCreateInstance
        create.restype = ctypes.c_long
        create.argtypes = [POINTER(_GUID), POINTER(_GUID), POINTER(c_void_p)]
    except AttributeError:
        return {"status": "blocked", "reason": "DxcCreateInstance_absent"}

    lib = c_void_p()
    hr = create(byref(CLSID_LIBRARY), byref(IID_LIBRARY), byref(lib))
    if hr != 0 or not lib:
        return {"status": "blocked", "reason": f"create_library_hr={hex(hr & 0xffffffff)}"}

    buf = ctypes.create_string_buffer(container, len(container))
    blob = c_void_p()
    # IDxcLibrary::CreateBlobWithEncodingFromPinned(vtbl idx 6): (ptr, size, codePage, IDxcBlobEncoding**)
    hr = _vcall(lib, 6, ctypes.c_long, [c_void_p, c_uint32, c_uint32, POINTER(c_void_p)],
                ctypes.cast(buf, c_void_p), len(container), 0, byref(blob))
    if hr != 0 or not blob:
        return {"status": "blocked", "reason": f"create_blob_hr={hex(hr & 0xffffffff)}"}

    val = c_void_p()
    hr = create(byref(CLSID_VALIDATOR), byref(IID_VALIDATOR), byref(val))
    if hr != 0 or not val:
        return {"status": "blocked", "reason": f"create_validator_hr={hex(hr & 0xffffffff)}(dxil.dll/validator 不可用)"}

    res = c_void_p()
    # IDxcValidator::Validate(vtbl idx 3): (IDxcBlob* shader, UINT32 flags, IDxcOperationResult**)
    hr = _vcall(val, 3, ctypes.c_long, [c_void_p, c_uint32, POINTER(c_void_p)], blob, 0, byref(res))
    if hr != 0 or not res:
        return {"status": "blocked", "reason": f"validate_call_hr={hex(hr & 0xffffffff)}"}

    # IDxcOperationResult::GetStatus(vtbl idx 3)
    op_status = ctypes.c_long(0)
    _vcall(res, 3, ctypes.c_long, [POINTER(ctypes.c_long)], byref(op_status))
    status_code = op_status.value & 0xFFFFFFFF

    # IDxcOperationResult::GetErrorBuffer(vtbl idx 5) → IDxcBlobEncoding
    err_msg = ""
    errblob = c_void_p()
    _vcall(res, 5, ctypes.c_long, [POINTER(c_void_p)], byref(errblob))
    if errblob:
        ptr = _vcall(errblob, 3, c_void_p, [])           # IDxcBlob::GetBufferPointer
        sz = _vcall(errblob, 4, c_size_t, [])            # IDxcBlob::GetBufferSize
        if ptr and sz:
            err_msg = ctypes.string_at(ptr, sz).decode("utf-8", "replace").rstrip("\x00").strip()

    accepted = status_code == 0
    return {
        "status": "measured",
        "accepted": accepted,
        "validation_status_hr": hex(status_code),
        "error_message": err_msg,
        "validator_dll": dll_path,
    }
