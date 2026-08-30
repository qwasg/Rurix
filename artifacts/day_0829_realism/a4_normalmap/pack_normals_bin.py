# -*- coding: utf-8 -*-
"""Day 0829 臂④ 法线运行时容器打包（DDS 全 mip 链 → .rgba8bin;确定性可重跑）。

容器 = g31_rgba8bin_read 闭集同律:头 3×u32 LE [w,h,mips] + 逐级 RGBA8 行主
序紧凑,pow2 方图 ≤2048,完整链(w→1x1)。法线编码:R=X,G=Y(BC5 解码 8bit
原值),B=128 常量,A=255——kernel 侧 x=(R−127)/127,y=(G−127)/127,
z=sqrt(max(0,1−x²−y²))。零重采样:美术原始 DDS mip 逐级 BC5 解码直搬;
mips_raw < 完整链时缺级 box 降采样补齐(登记 synthesized_levels)。
非法常值检测(day_0830 W1;bake_normals.detect_illegal_const_rg 单一事实源):
mip0 整张常值且 ‖xy‖₁>1 ⇒ 全链替换平坦 (127,127),告警+manifest 逐条登记。

用法: python pack_normals_bin.py [--limit N]
产物: baked_normals_bin/slotNN.rgba8bin + manifest_bin.json
"""
from __future__ import annotations

import argparse
import json
import struct
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bake_normals import (  # noqa: E402
    DEFAULT_GLTF,
    FLAT_RG,
    decode_bc4_plane,
    detect_illegal_const_rg,
    flat_like,
    parse_dds_header,
    rank_slots,
    sha256,
)

OUT_DIR = Path(__file__).resolve().parent / "baked_normals_bin"


def decode_bc5_level(b: bytes, w: int, h: int, off: int, uri: str) -> tuple[np.ndarray, int]:
    """DDS 体某级 → ((h,w,2) uint8, 消费字节数)。"""
    bw = max(1, (w + 3) // 4)
    bh = max(1, (h + 3) // 4)
    need = bw * bh * 16
    if off + need > len(b):
        raise ValueError(f"{uri}: 级体截断（{w}x{h} 需 {need}B,存 {len(b) - off}B）")
    blocks = np.frombuffer(b, dtype=np.uint8, count=need, offset=off).reshape(-1, 16)
    planes = []
    for half in (blocks[:, :8], blocks[:, 8:16]):
        texels = decode_bc4_plane(np.ascontiguousarray(half))
        img = texels.reshape(bh, bw, 4, 4).transpose(0, 2, 1, 3).reshape(bh * 4, bw * 4)
        planes.append(img[:h, :w])
    return np.stack(planes, axis=-1), need


def box_half(rg: np.ndarray) -> np.ndarray:
    """(h,w,2) → (h/2,w/2,2) box 均值(合成缺级用;XY 分量域,kernel 重归一)。"""
    h, w = rg.shape[0], rg.shape[1]
    nh, nw = max(1, h // 2), max(1, w // 2)
    a = rg[: nh * 2 : 2, : nw * 2 : 2].astype(np.uint16)
    b = rg[1 : nh * 2 : 2, : nw * 2 : 2].astype(np.uint16) if h > 1 else a
    c = rg[: nh * 2 : 2, 1 : nw * 2 : 2].astype(np.uint16) if w > 1 else a
    d = rg[1 : nh * 2 : 2, 1 : nw * 2 : 2].astype(np.uint16) if (h > 1 and w > 1) else a
    return ((a + b + c + d + 2) // 4).astype(np.uint8)


def pack_one(slot: int, mi: int, tris: int, doc: dict, base: Path, out_dir: Path = OUT_DIR) -> dict:
    mat = doc["materials"][mi]
    name = mat.get("name", "")
    nt = mat.get("normalTexture")
    if nt is None:
        raise ValueError(f"材质 {mi}（{name}）缺 normalTexture")
    uri = doc["images"][doc["textures"][nt["index"]]["source"]]["uri"]
    raw = (base / uri).read_bytes()
    w, h, mips_raw, fourcc, dxgi, data_off = parse_dds_header(raw, uri)
    if w != h or (w & (w - 1)) != 0 or w > 2048:
        raise ValueError(f"{uri}: {w}x{h} 越 pow2 方图 ≤2048 容器闭集")
    full_chain = w.bit_length()  # w→1 完整链级数(w pow2)
    mips_have = max(1, mips_raw)
    # 逐级解码(有多少搬多少)。
    levels_rg: list[np.ndarray] = []
    off = data_off
    lw, lh = w, h
    for _ in range(min(mips_have, full_chain)):
        rg, used = decode_bc5_level(raw, lw, lh, off, uri)
        levels_rg.append(rg)
        off += used
        lw, lh = max(1, lw // 2), max(1, lh // 2)
    synthesized = 0
    while len(levels_rg) < full_chain:
        levels_rg.append(box_half(levels_rg[-1]))
        synthesized += 1
    # 非法常值检测(mip0 判定,命中即全链替换平坦;HANDOVER §H slot14 修复律)。
    sanitized = detect_illegal_const_rg(levels_rg[0])
    if sanitized is not None:
        print(
            f"WARN: slot{slot:02d} mat{mi} {name} 非法常值法线 rg={tuple(sanitized['const_rg'])}"
            f" |x|+|y|={sanitized['norm_l1']}>1 → 全链替换平坦 {FLAT_RG}（HANDOVER §H 修复律）"
        )
        levels_rg = [flat_like(rg) for rg in levels_rg]
    # 容器组装。
    blob = bytearray(struct.pack("<III", w, h, full_chain))
    mip0_rgba: bytes | None = None
    for rg in levels_rg:
        hh, ww = rg.shape[0], rg.shape[1]
        px = np.empty((hh, ww, 4), dtype=np.uint8)
        px[..., 0:2] = rg
        px[..., 2] = 128
        px[..., 3] = 255
        pb = px.tobytes()
        if mip0_rgba is None:
            mip0_rgba = pb
        blob.extend(pb)
    out = out_dir / f"slot{slot:02d}.rgba8bin"
    out.write_bytes(bytes(blob))
    entry = {
        "slot": slot,
        "material_index": mi,
        "material_name": name,
        "tris": tris,
        "source_uri": uri.replace("\\", "/"),
        "source_sha256": sha256(raw),
        "file": out.name,
        "output_sha256": sha256(bytes(blob)),
        "mip0_rgba8_sha256": sha256(mip0_rgba or b""),
        "width": w,
        "height": h,
        "mips": full_chain,
        "mips_from_dds": min(mips_have, full_chain),
        "synthesized_levels": synthesized,
        "fourcc": fourcc,
        "dxgi_format": dxgi,
    }
    if sanitized is not None:
        entry["sanitized"] = sanitized
    return entry


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=None)
    ap.add_argument("--gltf", type=Path, default=DEFAULT_GLTF)
    ap.add_argument("--out", type=Path, default=OUT_DIR, help="输出目录（默认 baked_normals_bin/;v2 重烘不覆盖旧目录用）")
    args = ap.parse_args()
    gltf_bytes = args.gltf.read_bytes()
    doc = json.loads(gltf_bytes.decode("utf-8"))
    base = args.gltf.parent
    slots = rank_slots(doc)
    todo = slots[: args.limit] if args.limit is not None else slots
    args.out.mkdir(parents=True, exist_ok=True)
    entries: list[dict] = []
    anomalies: list[dict] = []
    for slot, mi, tris in todo:
        name = doc["materials"][mi].get("name", "")
        try:
            e = pack_one(slot, mi, tris, doc, base, args.out)
            entries.append(e)
            print(
                f"slot{slot:02d} mat{mi:>3} {name:<44} {e['width']:>4}² mips={e['mips']:>2}"
                f" dds={e['mips_from_dds']:>2} synth={e['synthesized_levels']}"
            )
        except (ValueError, OSError, KeyError) as ex:
            anomalies.append({"slot": slot, "material_index": mi, "material_name": name, "error": str(ex)})
            print(f"slot{slot:02d} mat{mi:>3} {name:<44} ANOMALY: {ex}", file=sys.stderr)
    manifest = {
        "schema": "rurix.day0829.a4_normalmap.bin_manifest.v1",
        "gltf": str(args.gltf).replace("\\", "/"),
        "gltf_sha256": sha256(gltf_bytes),
        "container_law": "g31_rgba8bin_read 闭集:3×u32 LE [w,h,mips] + 逐级 RGBA8 行主序;pow2 方图 ≤2048 完整链;R=X,G=Y,B=128,A=255",
        "decode_law": "kernel 侧 x=(R−127)/127, y=(G−127)/127, z=sqrt(max(0,1−x²−y²));BC5 标准 D3D/bcdec 表(见 bake_normals.py bc5_law)",
        "counts": {"packed": len(entries), "anomalies": len(anomalies)},
        "limit": args.limit,
        "entries": entries,
        "anomalies": anomalies,
    }
    (args.out / "manifest_bin.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=1), encoding="utf-8"
    )
    print(f"packed={len(entries)} anomalies={len(anomalies)} → {args.out / 'manifest_bin.json'}")
    return 0 if not anomalies else 2


if __name__ == "__main__":
    sys.exit(main())
