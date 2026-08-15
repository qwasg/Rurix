#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.4a 波）
"""G10.4a ci 侧独立 EXR 解析器（spec/imageio.md §2A RXS-0385 同语义第二实现）。

职责：M134/M137 门侧的**独立复核面**——与 `src/image-io/src/exr.rs`（Rust
实现）同字面语义、独立编码路径（双实现互证，防单实现口径漂移伪绿）：

- header 属性解析（name\\0 type\\0 size u32le value，\\0 终止）；
- 分端读取策略：rurix strict（白名单与 rurix:* 闭集外属性确定性拒绝）/
  ue5 strip-and-log（闭集外属性剥离逐条登记；rurix:* 冒充拒绝）；
- chromaticities Rec.709/D65 位级闭集互证；
- NONE 压缩 scanline 解码（fp32 直读 / fp16 精确提升 f32）；ZIP 及闭集外
  压缩值 fail-closed 显式 UnsupportedCompression；
- 帧内容 digest（与 Rust bin 同字面：`"G10EXRD-1\\0" ‖ w u32le ‖ h u32le
  ‖ channels u8 ‖ f32 LE 像素字节`）——跨实现 digest 相等为互证锚。

纪律：标准库零依赖；只读面（门侧复核不产 EXR 字节流）。
"""
from __future__ import annotations

import hashlib
import struct
from pathlib import Path

EXR_MAGIC = b"\x76\x2f\x31\x01"
CHROMATICITIES_REC709_D65 = (0.64, 0.33, 0.30, 0.60, 0.15, 0.06, 0.3127, 0.3290)

STD_WHITELIST = frozenset({
    "channels",
    "chromaticities",
    "compression",
    "dataWindow",
    "displayWindow",
    "lineOrder",
    "pixelAspectRatio",
    "screenWindowCenter",
    "screenWindowWidth",
})

RURIX_ATTRS = frozenset({
    "rurix:bit_depth",
    "rurix:capture_params_digest",
    "rurix:chromaticities_origin",
    "rurix:derivation",
    "rurix:domain",
    "rurix:schema_version",
    "rurix:source_end",
    "rurix:source_frame_digest",
    "rurix:transfer",
    "rurix:view_transform",
})

RURIX_REQUIRED = (
    "rurix:schema_version",
    "rurix:domain",
    "rurix:transfer",
    "rurix:bit_depth",
    "rurix:source_end",
    "rurix:capture_params_digest",
    "rurix:derivation",
)

DOMAIN_TRANSFER = {"scene-linear-hdr": "linear", "display-referred-ldr": "srgb"}


class ExrError(ValueError):
    """fail-closed：任何闭集/形态违例即抛出。"""


class UnsupportedCompression(ExrError):
    """compression ∈ v1 实现面外（ZIP 未接通 / 闭集外压缩）。"""


class MetadataViolation(ExrError):
    """元数据闭集违例。"""


def half_to_f32(bits: int) -> float:
    """fp16 位模式 → f32 精确提升（IEEE-754 binary16 → binary64 中间计算，
    值域与 Rust half_to_f32 逐值一致）。"""
    sign = -1.0 if (bits >> 15) & 1 else 1.0
    exp = (bits >> 10) & 0x1F
    frac = bits & 0x3FF
    if exp == 0:
        return sign * frac * (2.0 ** -24)
    if exp == 31:
        return sign * float("inf") if frac == 0 else float("nan")
    return sign * (1.0 + frac / 1024.0) * (2.0 ** (exp - 15))


def frame_content_digest(width: int, height: int, channels: int, pixels: list[float]) -> str:
    """帧像素内容 digest（与 Rust bin 同字面，跨实现互证锚）。"""
    h = hashlib.sha256()
    h.update(b"G10EXRD-1\x00")
    h.update(struct.pack("<I", width))
    h.update(struct.pack("<I", height))
    h.update(struct.pack("<B", channels))
    h.update(struct.pack(f"<{len(pixels)}f", *pixels))
    return "sha256:" + h.hexdigest()


def _read_cstr(buf: bytes, pos: int) -> tuple[str, int]:
    end = buf.index(b"\x00", pos)
    return buf[pos:end].decode("utf-8"), end + 1


def parse_header(buf: bytes) -> tuple[list[tuple[str, str, bytes]], int]:
    if len(buf) < 9 or buf[:4] != EXR_MAGIC:
        raise ExrError("EXR magic 不符（非真 EXR 帧）")
    if buf[4] != 2 or (buf[5] & ~0x04) or buf[6] != 0 or buf[7] != 0:
        # flags 字节仅放行 long names（0x04，UE 长属性名面）；tiled/deep/multi-part 子集外拒绝
        raise ExrError(f"EXR version 字段非 v2 scanline（tiled/deep/multi-part 子集外拒绝）: {buf[4:8].hex()}")
    pos = 8
    attrs: list[tuple[str, str, bytes]] = []
    while True:
        if pos >= len(buf):
            raise ExrError("header 截断")
        if buf[pos] == 0:
            pos += 1
            break
        name, pos = _read_cstr(buf, pos)
        attr_type, pos = _read_cstr(buf, pos)
        (size,) = struct.unpack_from("<I", buf, pos)
        pos += 4
        value = buf[pos:pos + size]
        if len(value) != size:
            raise ExrError(f"属性 {name} 值截断")
        pos += size
        attrs.append((name, attr_type, value))
    return attrs, pos


def decode_exr(buf: bytes, expected_end: str) -> dict:
    """解码 EXR 帧（NONE；分端策略）。返回 dict:
    {width, height, layout("rgb"|"y"), pixels, source_bit_depth, metadata|None,
     stripped[], compression, chromaticities_ok}。"""
    if expected_end not in ("rurix", "ue5"):
        raise ExrError(f"expected_end 非法: {expected_end!r}")
    attrs, body = parse_header(buf)
    names = {a[0] for a in attrs}
    stripped: list[dict] = []

    # 分端策略。
    for name, attr_type, value in attrs:
        standard = name in STD_WHITELIST
        rurix = name.startswith("rurix:")
        if expected_end == "rurix":
            if rurix:
                if name not in RURIX_ATTRS:
                    raise MetadataViolation(f"rurix 帧 strict：rurix:* 闭集外属性 {name!r}")
            elif not standard:
                raise MetadataViolation(f"rurix 帧 strict：白名单外属性 {name!r}")
        else:
            if rurix:
                raise MetadataViolation(f"ue5 帧出现 rurix:* 属性 {name!r}（命名空间冒充）")
            if not standard:
                stripped.append({
                    "name": name,
                    "attr_type": attr_type,
                    "value_len": len(value),
                    "reason": "ue5-strip-and-log",
                })

    amap = {a[0]: a for a in attrs}

    # compression。
    comp = amap.get("compression")
    if comp is None or len(comp[2]) != 1:
        raise ExrError("缺 compression 属性或长度非法")
    compression = comp[2][0]
    if compression != 0:
        raise UnsupportedCompression(
            f"compression={compression}（闭集 {{NONE, ZIP}} 内 ZIP 解码 v1 未接通；其余压缩禁入）"
        )
    # lineOrder。
    lo = amap.get("lineOrder")
    if lo is None or lo[2][0] != 0:
        raise ExrError("lineOrder 非 INCREASING_Y（子集外）")
    # dataWindow / displayWindow。
    def _box2i(name: str) -> tuple[int, int, int, int]:
        a = amap.get(name)
        if a is None or a[1] != "box2i" or len(a[2]) != 16:
            raise ExrError(f"{name} 缺失或形态非法")
        return struct.unpack("<4i", a[2])

    dw = _box2i("dataWindow")
    disp = _box2i("displayWindow")
    if dw[0] != 0 or dw[1] != 0:
        raise ExrError("dataWindow 非零原点（子集外）")
    width, height = dw[2] + 1, dw[3] + 1
    if width <= 0 or height <= 0 or disp[2] != dw[2] or disp[3] != dw[3]:
        raise ExrError("dataWindow/displayWindow 尺寸非法或不一致")

    # chromaticities 位级闭集互证。
    chroma = amap.get("chromaticities")
    if chroma is None:
        raise MetadataViolation(
            "chromaticities 缺失（ue5 帧须经 harness backfill 先行，本面 fail-closed）"
        )
    if chroma[1] != "chromaticities" or len(chroma[2]) != 32:
        raise ExrError("chromaticities 形态非法")
    got_chroma = struct.unpack("<8f", chroma[2])
    for got, want in zip(got_chroma, CHROMATICITIES_REC709_D65):
        if struct.pack("<f", got) != struct.pack("<f", want):
            raise MetadataViolation(f"chromaticities 值 ≠ Rec.709/D65 闭集（{got} ≠ {want}）")

    # channels。
    ch_attr = amap.get("channels")
    if ch_attr is None or ch_attr[1] != "chlist":
        raise ExrError("channels 缺失或形态非法")
    chans: list[tuple[str, int]] = []  # (name, pixel_type)
    pos = 0
    blob = ch_attr[2]
    while True:
        if pos >= len(blob):
            raise ExrError("chlist 截断")
        if blob[pos] == 0:
            break
        end = blob.index(b"\x00", pos)
        cname = blob[pos:end].decode("utf-8")
        pos = end + 1
        ptype, = struct.unpack_from("<i", blob, pos)
        pos += 4
        pos += 4  # pLinear + reserved[3]
        xs, ys = struct.unpack_from("<2i", blob, pos)
        pos += 8
        if xs != 1 or ys != 1:
            raise ExrError(f"通道 {cname} 采样率非 1（子集外）")
        if ptype not in (1, 2):
            raise ExrError(f"通道 {cname} pixel_type={ptype}（子集外）")
        chans.append((cname, ptype))
    chan_names = [c for c, _ in chans]
    has_alpha = "A" in chan_names
    if has_alpha:
        stripped.append({
            "name": "A",
            "attr_type": "channel",
            "value_len": 0,
            "reason": "alpha-channel-strip",
        })
    base_names = [n for n in chan_names if n != "A"]
    if base_names == ["B", "G", "R"]:
        layout = "rgb"
    elif base_names == ["Y"]:
        layout = "y"
    else:
        raise ExrError(f"通道集 {base_names} 非 canonical（B,G,R / 单通道 Y）")
    ptypes = {p for _, p in chans}
    if len(ptypes) != 1:
        raise ExrError("通道位深混合（子集外）")
    source_bit_depth = "float16" if ptypes == {1} else "float32"

    # rurix 帧元数据闭集重构（strict 端齐备校验 + 位深互证）。
    metadata = None
    if expected_end == "rurix":
        metadata = {}
        for req in RURIX_REQUIRED:
            if req not in names:
                raise MetadataViolation(f"元数据缺字段: {req}")
        for req in RURIX_ATTRS & names:
            a = amap[req]
            if a[1] != "string":
                raise MetadataViolation(f"属性 {req} 类型须为 string")
            metadata[req] = a[2].decode("utf-8")
        if metadata["rurix:source_end"] != "rurix":
            raise MetadataViolation("rurix:source_end 须为 \"rurix\"")
        domain = metadata["rurix:domain"]
        transfer = metadata["rurix:transfer"]
        if DOMAIN_TRANSFER.get(domain) != transfer:
            raise MetadataViolation(f"域/transfer 混标: {domain} + {transfer}")
        if metadata["rurix:bit_depth"] != "float32" or source_bit_depth != "float32":
            raise MetadataViolation("rurix 帧位深非 float32 canonical")
        if domain == "display-referred-ldr" and "rurix:view_transform" not in metadata:
            raise MetadataViolation("LDR 臂 rurix:view_transform 必填")
        if (metadata["rurix:derivation"] == "derived:host-srgb-encoder-v1"
                and "rurix:source_frame_digest" not in metadata):
            raise MetadataViolation("派生帧 rurix:source_frame_digest 必填")

    # 像素体（NONE scanline）。
    pos = body
    offsets = []
    for _ in range(height):
        (off,) = struct.unpack_from("<Q", buf, pos)
        offsets.append(off)
        pos += 8
    out_ch = 3 if layout == "rgb" else 1
    pixels = [0.0] * (width * height * out_ch)
    bytes_per = 2 if source_bit_depth == "float16" else 4
    for y, off in enumerate(offsets):
        sy, packed = struct.unpack_from("<iI", buf, off)
        if sy != y:
            raise ExrError(f"扫描线 y={sy} 与偏移表序 {y} 不符")
        want = width * len(chans) * bytes_per
        if packed != want:
            raise ExrError(f"扫描线 {y} packed_size={packed} ≠ {want}")
        p = off + 8
        for cname, _ptype in chans:
            for x in range(width):
                if source_bit_depth == "float16":
                    (hb,) = struct.unpack_from("<H", buf, p)
                    v = half_to_f32(hb)
                    p += 2
                else:
                    (v,) = struct.unpack_from("<f", buf, p)
                    p += 4
                if cname == "A":
                    continue
                if layout == "rgb":
                    out_ci = {"R": 0, "G": 1, "B": 2}[cname]
                else:
                    out_ci = 0
                pixels[(y * width + x) * out_ch + out_ci] = v
    if any(v != v for v in pixels):
        raise ExrError("NaN 帧值禁入 canonical 面")
    return {
        "width": width,
        "height": height,
        "layout": layout,
        "pixels": pixels,
        "source_bit_depth": source_bit_depth,
        "metadata": metadata,
        "stripped": stripped,
        "compression": compression,
        "chromaticities_ok": True,
    }


def decode_exr_file(path: Path, expected_end: str) -> dict:
    return decode_exr(Path(path).read_bytes(), expected_end)


def quantize_u8(c: float) -> int:
    """RXS-0116 确定量化（clamp + 就近取整半值向上；NaN→0）——热区图复核面。"""
    if c != c:
        c = 0.0
    c = min(1.0, max(0.0, c))
    import math

    return int(math.floor(c * 255.0 + 0.5))


def heatmap_ppm_bytes(width: int, height: int, err: list[float]) -> bytes:
    """灰度热区图 PPM P6 字节流（e→[e,e,e]→RXS-0116 量化；与 Rust 侧同字面）。"""
    out = bytearray(f"P6\n{width} {height}\n255\n".encode("ascii"))
    for e in err:
        q = quantize_u8(e)
        out += bytes((q, q, q))
    return bytes(out)


def nearest_rank_p95(sorted_vals: list[float]) -> float:
    """nearest-rank p95（RXS-0388 L2 冻结口径：第 ceil(0.95·N) 个，1-based）。"""
    import math

    n = len(sorted_vals)
    rank = max(1, math.ceil(0.95 * n))
    return sorted_vals[min(rank, n) - 1]
