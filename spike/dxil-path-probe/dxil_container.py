# SPIKE(RD-010) — round-4 A 路互操作诊断:DXContainer(DXBC)结构解析。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first:解析 llc 产 DXContainer 的 part 表/签名摘要,与 dxc 自产容器对照,
# 定位 dxc 1.8 validator 拒绝(0x80aa000f)的结构成因(缺 part / 未签名 / 顺序),绝不杜撰。
"""DXContainer(DXBC)二进制结构解析(只读,纯字节;无外部依赖)。

DXContainer 头布局(小端):
  magic[4]="DXBC" | HashDigest[16] | MajorVer(u16) | MinorVer(u16) |
  FileSize(u32) | PartCount(u32) | PartOffset[PartCount](u32 each)
每个 part:FourCC[4] | PartSize(u32) | data[PartSize]。
"""
from __future__ import annotations

import struct


def parse_dxbc(data: bytes) -> dict:
    """解析 DXContainer 字节,返回 {ok, magic, digest, version, file_size, part_count, parts:[{fourcc,size}]}。

    非 DXBC / 截断 → ok=False + reason,绝不抛(blocked-honest)。
    """
    if not data or len(data) < 36:
        return {"ok": False, "reason": "empty_or_too_short", "size": len(data) if data else 0}
    if data[:4] != b"DXBC":
        return {"ok": False, "reason": "magic_not_DXBC", "magic": data[:4].decode("ascii", "replace")}
    digest = data[4:20]
    major, minor = struct.unpack("<HH", data[20:24])
    file_size = struct.unpack("<I", data[24:28])[0]
    part_count = struct.unpack("<I", data[28:32])[0]
    parts = []
    bad = False
    for i in range(part_count):
        off_pos = 32 + 4 * i
        if off_pos + 4 > len(data):
            bad = True
            break
        off = struct.unpack("<I", data[off_pos:off_pos + 4])[0]
        if off + 8 > len(data):
            parts.append({"fourcc": f"BADOFF@{off}", "size": -1})
            bad = True
            continue
        fourcc = data[off:off + 4].decode("ascii", "replace")
        psz = struct.unpack("<I", data[off + 4:off + 8])[0]
        parts.append({"fourcc": fourcc, "size": psz})
    return {
        "ok": not bad,
        "size": len(data),
        "magic": "DXBC",
        "digest_hex": digest.hex(),
        "is_signed": digest != b"\x00" * 16,
        "version": [major, minor],
        "file_size": file_size,
        "part_count": part_count,
        "parts": parts,
        "part_fourccs": [p["fourcc"] for p in parts],
    }


def dxil_part_version(data: bytes) -> dict:
    """round-5 DXIL 版本子轴:从 DXContainer 的 DXIL part 提取 DxilProgramHeader 版本。

    DXIL part 内容布局(小端):
      ProgramVersion(u32) | SizeInUint32(u32) | BitcodeHeader{ DxilMagic[4]="DXIL" |
      DxilVersion(u32) | BitcodeOffset(u32) | BitcodeSize(u32) } | bitcode...
    ProgramVersion 解码(HLSL DxilContainer.h):Minor=v&0xF, Major=(v>>4)&0xF,
      ShaderKind=(v>>16)&0xFFFF。绝不杜撰:解析失败返回 ok=False + reason。
    """
    p = parse_dxbc(data)
    if not p.get("ok"):
        return {"ok": False, "reason": "container_parse_failed"}
    # 定位 DXIL part 的字节偏移(parse_dxbc 只记 fourcc/size,这里重扫 offset 取 data)
    part_count = p.get("part_count", 0)
    dxil_off = None
    for i in range(part_count):
        off_pos = 32 + 4 * i
        if off_pos + 4 > len(data):
            break
        off = struct.unpack("<I", data[off_pos:off_pos + 4])[0]
        if off + 8 > len(data):
            continue
        if data[off:off + 4] == b"DXIL":
            dxil_off = off
            break
    if dxil_off is None:
        return {"ok": False, "reason": "no_DXIL_part"}
    body = dxil_off + 8  # 跳过 FourCC + PartSize
    if body + 24 > len(data):
        return {"ok": False, "reason": "DXIL_part_truncated"}
    prog_ver = struct.unpack("<I", data[body:body + 4])[0]
    size_u32 = struct.unpack("<I", data[body + 4:body + 8])[0]
    dxil_magic = data[body + 8:body + 12]
    dxil_ver_raw = struct.unpack("<I", data[body + 12:body + 16])[0]
    sm_minor = prog_ver & 0xF
    sm_major = (prog_ver >> 4) & 0xF
    shader_kind = (prog_ver >> 16) & 0xFFFF
    return {
        "ok": True,
        "program_version_raw": prog_ver,
        "shader_model": f"{sm_major}.{sm_minor}",
        "shader_kind": shader_kind,
        "size_in_uint32": size_u32,
        "dxil_part_magic": dxil_magic.decode("ascii", "replace"),
        "dxil_version_raw": dxil_ver_raw,
        "dxil_version_hex": hex(dxil_ver_raw),
    }


def diff_parts(llc_parsed: dict, dxc_parsed: dict) -> dict:
    """对照 llc 容器与 dxc 自产容器的 part 集合/顺序/签名,定位结构差异。"""
    llc_fc = llc_parsed.get("part_fourccs", []) if llc_parsed.get("ok") else []
    dxc_fc = dxc_parsed.get("part_fourccs", []) if dxc_parsed.get("ok") else []
    missing = [fc for fc in dxc_fc if fc not in llc_fc]
    extra = [fc for fc in llc_fc if fc not in dxc_fc]
    return {
        "llc_parts": llc_fc,
        "dxc_parts": dxc_fc,
        "llc_missing_vs_dxc": missing,
        "llc_extra_vs_dxc": extra,
        "order_differs": llc_fc != dxc_fc,
        "llc_signed": llc_parsed.get("is_signed"),
        "dxc_signed": dxc_parsed.get("is_signed"),
    }


# SPIKE(RD-010) — B 路图形签名取证:ISG1/OSG1/PSG1 签名 part 元素解析。
# 对照 round-8 A 路 elemcount=0 口径:证 SV_Position/SV_Target/插值 varying 是否端到端
# 存活进 DXIL 产物签名。纯字节只读,无外部依赖;解析失败 ok=False + reason,绝不杜撰。

# D3D_NAME(系统值语义)枚举 → 名(HLSL DxilConstants.h SemanticKind / d3d12.h D3D_NAME)。
_D3D_NAME = {
    0: "NONE", 1: "SV_Position", 2: "SV_ClipDistance", 3: "SV_CullDistance",
    4: "SV_RenderTargetArrayIndex", 5: "SV_ViewportArrayIndex", 6: "SV_VertexID",
    7: "SV_PrimitiveID", 8: "SV_InstanceID", 9: "SV_IsFrontFace", 10: "SV_SampleIndex",
    11: "SV_FinalQuadEdgeTessFactor", 12: "SV_FinalQuadInsideTessFactor",
    13: "SV_FinalTriEdgeTessFactor", 14: "SV_FinalTriInsideTessFactor",
    15: "SV_FinalLineDetailTessFactor", 16: "SV_FinalLineDensityTessFactor",
    23: "SV_Barycentrics", 24: "SV_ShadingRate", 25: "SV_CullPrimitive",
    64: "SV_Target", 65: "SV_Depth", 66: "SV_Coverage", 67: "SV_DepthGreaterEqual",
    68: "SV_DepthLessEqual", 69: "SV_StencilRef", 70: "SV_InnerCoverage",
}
# D3D_REGISTER_COMPONENT_TYPE → 名。
_COMP_TYPE = {0: "unknown", 1: "uint32", 2: "sint32", 3: "float32"}
# D3D_MIN_PRECISION → 名。
_MIN_PREC = {0: "default", 1: "float16", 2: "float2_8", 4: "uint16", 5: "sint16", 6: "any16", 7: "any10"}


def _find_part(data: bytes, fourcc: bytes) -> int | None:
    """返回指定 FourCC part 的数据起始偏移(part 头后,即 PartSize 后),无则 None。"""
    p = parse_dxbc(data)
    if not p.get("ok"):
        return None
    for i in range(p.get("part_count", 0)):
        off_pos = 32 + 4 * i
        if off_pos + 4 > len(data):
            break
        off = struct.unpack("<I", data[off_pos:off_pos + 4])[0]
        if off + 8 > len(data):
            continue
        if data[off:off + 4] == fourcc:
            return off + 8  # 跳过 FourCC(4) + PartSize(4) → part 数据起点
    return None


def _read_cstr(data: bytes, off: int) -> str:
    """从 off 读 null 结尾 ASCII 字符串(签名 SemanticName)。"""
    if off < 0 or off >= len(data):
        return ""
    end = data.find(b"\x00", off)
    if end < 0:
        end = len(data)
    return data[off:end].decode("ascii", "replace")


# DxilProgramSignatureElement(ISG1/OSG1/PSG1 v1)布局(小端,32 字节/元素):
#  Stream(u32) SemanticName_off(u32) SemanticIndex(u32) SystemValue(u32)
#  CompType(u32) Register(u32) Mask(u8) ExclusiveMask(u8) Pad(u16) MinPrecision(u32)
# 头:ParamCount(u32) ParamOffset(u32);off 均相对 part 数据起点。
_SIG_ELEM_SIZE = 32


def parse_signature_part(data: bytes, fourcc: str) -> dict:
    """解 DXContainer 的 ISG1/OSG1/PSG1 签名 part → elemcount + 各元素 SV 语义。

    对照 round-8 A 路 elemcount=0:elemcount>0 且元素 system_value 命名 SV_* → SV 真达产物。
    无该 part / 解析失败 → ok=False + reason(blocked-honest,绝不杜撰)。
    """
    base = _find_part(data, fourcc.encode("ascii"))
    if base is None:
        return {"ok": False, "reason": f"no_{fourcc}_part", "fourcc": fourcc}
    if base + 8 > len(data):
        return {"ok": False, "reason": "sig_header_truncated", "fourcc": fourcc}
    param_count = struct.unpack("<I", data[base:base + 4])[0]
    param_off = struct.unpack("<I", data[base + 4:base + 8])[0]
    elems = []
    for i in range(param_count):
        eo = base + param_off + i * _SIG_ELEM_SIZE
        if eo + _SIG_ELEM_SIZE > len(data):
            return {"ok": False, "reason": f"elem_{i}_truncated", "fourcc": fourcc}
        stream, name_off, sem_idx, sysval, comp, reg = struct.unpack("<IIIIII", data[eo:eo + 24])
        mask, excl = data[eo + 24], data[eo + 25]
        minprec = struct.unpack("<I", data[eo + 28:eo + 32])[0]
        elems.append({
            "semantic_name": _read_cstr(data, base + name_off),
            "semantic_index": sem_idx,
            "system_value": _D3D_NAME.get(sysval, f"0x{sysval:x}"),
            "system_value_raw": sysval,
            "comp_type": _COMP_TYPE.get(comp, str(comp)),
            "register": reg,
            "mask": mask,
            "exclusive_mask": excl,
            "min_precision": _MIN_PREC.get(minprec, str(minprec)),
            "stream": stream,
        })
    return {"ok": True, "fourcc": fourcc, "elemcount": param_count, "elements": elems}
