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
