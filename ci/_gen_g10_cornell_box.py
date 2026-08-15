#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.3 波）
"""G10.3 压测语料 Cornell Box 程序生成器（generated 类资产，RFC-0027 §4.2
generated 替代登记型；spec/external_reference.md RXS-0381/RXS-0382）。

纯自写几何/反射率公式，不读取/转换任何外部数据文件（generated 类 NONE
判定的成立前提，RFC-0027 §4.2 F14 修法字面）。几何/反射率数值参考
Cornell PCG「Public Use Data」页（https://www.graphics.cornell.edu/online/box/data.html
——页面无显式许可文本，仅作数值来源参考登记，不作资产摄入）。

产物（写入 <cache_root>/cornell-box-generated/v1/，缓存根解析见 RXS-0382）：
  cornell_box.gltf   glTF 2.0（外部 .bin + 外部 checker.png）
  cornell_box.bin    顶点/索引二进制（确定性布局，逐段 4 字节对齐）
  checker.png        8x8 程序生成棋盘格纹理（确定性 PNG 编码，floor 专用，
                     供 M132 加载门纹理计数非空判据）
  generator_params.json  生成参数（canonical 序列化，digest 实测登记）

确定性：同一参数两次运行逐字节一致（固定遍历序、固定浮点字面量、LF）。

用法：
  py -3 ci/_gen_g10_cornell_box.py [--out DIR] [--print-digests]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# ── 生成参数（冻结常量；canonical digest 实测登记进 M131 注册表） ──
PARAMS: dict = {
    "schema": "rurix.g10.cornell_box_params.v1",
    "box": {"width": 552.8, "height": 548.8, "depth": 558.8},
    "colors": {
        "white": [0.725, 0.71, 0.68, 1.0],
        "red": [0.63, 0.065, 0.05, 1.0],
        "green": [0.14, 0.45, 0.091, 1.0],
    },
    "tall_block": {"size": [165.0, 330.0, 165.0], "center": [350.0, 165.0, 350.0], "rot_y_deg": -15.0},
    "short_block": {"size": [165.0, 165.0, 165.0], "center": [185.0, 82.5, 170.0], "rot_y_deg": 15.0},
    "checker": {"size_px": 8, "cell_px": 4, "lo": [56, 56, 56], "hi": [224, 224, 224]},
    "outputs": {
        "gltf": "cornell_box.gltf",
        "bin": "cornell_box.bin",
        "png": "checker.png",
        "params": "generator_params.json",
    },
}


def canonical_json(doc: dict) -> bytes:
    return (json.dumps(doc, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def encode_png_rgb(w: int, h: int, px: bytes) -> bytes:
    """确定性 PNG（滤波 0，zlib level 9 固定）。"""

    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)
    raw = b"".join(b"\x00" + px[y * w * 3 : (y + 1) * w * 3] for y in range(h))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def checker_png(size: int, cell: int, lo: list[int], hi: list[int]) -> bytes:
    px = bytearray()
    for y in range(size):
        for x in range(size):
            c = hi if ((x // cell) + (y // cell)) % 2 == 0 else lo
            px += bytes(c)
    return encode_png_rgb(size, size, bytes(px))


def quad(ccw: list[list[float]]) -> list[list[float]]:
    """4 角（CCW）→ 2 三角形索引展开由调用侧统一处理；此处原样返回。"""
    return ccw


def rot_y(p: list[float], deg: float, center: list[float]) -> list[float]:
    r = math.radians(deg)
    c, s = math.cos(r), math.sin(r)
    x, z = p[0] - center[0], p[2] - center[2]
    return [center[0] + c * x + s * z, p[1], center[2] - s * x + c * z]


def box_faces(size: list[float], center: list[float], rot_deg: float) -> list[tuple[list[float], list[list[float]]]]:
    """返回 6 面 [(normal, 4 角 CCW 自外向看)]，角点已绕 center 旋转 rot_deg。"""
    hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2
    cx, cy, cz = center
    raw = [
        ([0.0, 1.0, 0.0], [[-hx, hy, -hz], [hx, hy, -hz], [hx, hy, hz], [-hx, hy, hz]]),
        ([0.0, -1.0, 0.0], [[-hx, -hy, hz], [hx, -hy, hz], [hx, -hy, -hz], [-hx, -hy, -hz]]),
        ([0.0, 0.0, 1.0], [[-hx, -hy, hz], [hx, -hy, hz], [hx, hy, hz], [-hx, hy, hz]]),
        ([0.0, 0.0, -1.0], [[hx, -hy, -hz], [-hx, -hy, -hz], [-hx, hy, -hz], [hx, hy, -hz]]),
        ([1.0, 0.0, 0.0], [[hx, -hy, hz], [hx, -hy, -hz], [hx, hy, -hz], [hx, hy, hz]]),
        ([-1.0, 0.0, 0.0], [[-hx, -hy, -hz], [-hx, -hy, hz], [-hx, hy, hz], [-hx, hy, -hz]]),
    ]
    out = []
    for n, corners in raw:
        rn = rot_y([n[0], n[1], n[2]], rot_deg, [0.0, 0.0, 0.0])
        rc = [rot_y([cx + p[0], cy + p[1], cz + p[2]], rot_deg, center) for p in corners]
        out.append((rn, rc))
    return out


def build() -> dict[str, bytes]:
    w, h, d = PARAMS["box"]["width"], PARAMS["box"]["height"], PARAMS["box"]["depth"]
    walls: list[tuple[str, list[float], list[list[float]], bool]] = [
        # (材质名, 法线, 四角, 是否贴图)
        ("white_tex", [0.0, 1.0, 0.0], [[0, 0, 0], [w, 0, 0], [w, 0, d], [0, 0, d]], True),  # floor
        ("white", [0.0, -1.0, 0.0], [[0, h, d], [w, h, d], [w, h, 0], [0, h, 0]], False),  # ceiling
        ("white", [0.0, 0.0, -1.0], [[0, 0, d], [w, 0, d], [w, h, d], [0, h, d]], False),  # back
        ("red", [1.0, 0.0, 0.0], [[0, 0, 0], [0, 0, d], [0, h, d], [0, h, 0]], False),  # left
        ("green", [-1.0, 0.0, 0.0], [[w, 0, d], [w, 0, 0], [w, h, 0], [w, h, d]], False),  # right
    ]
    for key, mat in (("tall_block", "white"), ("short_block", "white")):
        spec = PARAMS[key]
        for n, corners in box_faces(spec["size"], spec["center"], spec["rot_y_deg"]):
            walls.append((mat, n, corners, False))

    bin_blob = bytearray()
    accessors: list[dict] = []
    buffer_views: list[dict] = []
    primitives: list[dict] = []
    nodes: list[dict] = []

    def push_view(data: bytes) -> int:
        while len(bin_blob) % 4:
            bin_blob.append(0)
        off = len(bin_blob)
        bin_blob.extend(data)
        buffer_views.append({"buffer": 0, "byteOffset": off, "byteLength": len(data)})
        return len(buffer_views) - 1

    def push_accessor(view: int, comp: int, count: int, ty: str, mn=None, mx=None) -> int:
        a: dict = {"bufferView": view, "componentType": comp, "count": count, "type": ty}
        if mn is not None:
            a["min"] = mn
            a["max"] = mx
        accessors.append(a)
        return len(accessors) - 1

    for idx, (mat, normal, corners, textured) in enumerate(walls):
        pos = struct.pack("<" + "f" * 12, *[c for p in corners for c in p])
        nrm = struct.pack("<" + "f" * 12, *([c for _ in corners for c in normal]))
        ind = struct.pack("<6H", 0, 1, 2, 0, 2, 3)
        attrs: dict = {}
        pa = push_accessor(push_view(pos), 5126, 4, "VEC3", mn=[min(p[i] for p in corners) for i in range(3)], mx=[max(p[i] for p in corners) for i in range(3)])
        attrs["POSITION"] = pa
        attrs["NORMAL"] = push_accessor(push_view(nrm), 5126, 4, "VEC3")
        if textured:
            uv = struct.pack("<8f", 0, 0, 1, 0, 1, 1, 0, 1)
            attrs["TEXCOORD_0"] = push_accessor(push_view(uv), 5126, 4, "VEC2")
        ia = push_accessor(push_view(ind), 5123, 6, "SCALAR")
        primitives.append({"attributes": attrs, "indices": ia, "material": {"white": 0, "red": 1, "green": 2, "white_tex": 3}[mat], "mode": 4})
        nodes.append({"mesh": idx, "name": f"part_{idx:02d}_{mat}"})

    gltf = {
        "asset": {"version": "2.0", "generator": "ci/_gen_g10_cornell_box.py (rurix G10.3)"},
        "scene": 0,
        "scenes": [{"nodes": list(range(len(nodes)))}],
        "nodes": nodes,
        "meshes": [{"name": f"mesh_{i}", "primitives": [primitives[i]]} for i in range(len(primitives))],
        "materials": [
            {"name": "white", "pbrMetallicRoughness": {"baseColorFactor": PARAMS["colors"]["white"], "metallicFactor": 0.0, "roughnessFactor": 1.0}},
            {"name": "red", "pbrMetallicRoughness": {"baseColorFactor": PARAMS["colors"]["red"], "metallicFactor": 0.0, "roughnessFactor": 1.0}},
            {"name": "green", "pbrMetallicRoughness": {"baseColorFactor": PARAMS["colors"]["green"], "metallicFactor": 0.0, "roughnessFactor": 1.0}},
            {"name": "white_tex", "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}, "baseColorFactor": [1.0, 1.0, 1.0, 1.0], "metallicFactor": 0.0, "roughnessFactor": 1.0}},
        ],
        "textures": [{"sampler": 0, "source": 0}],
        "samplers": [{"magFilter": 9729, "minFilter": 9729, "wrapS": 10497, "wrapT": 10497}],
        "images": [{"uri": PARAMS["outputs"]["png"]}],
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{"byteLength": len(bin_blob), "uri": PARAMS["outputs"]["bin"]}],
    }
    ck = PARAMS["checker"]
    return {
        PARAMS["outputs"]["gltf"]: (json.dumps(gltf, ensure_ascii=False, separators=(",", ":")) + "\n").encode("utf-8"),
        PARAMS["outputs"]["bin"]: bytes(bin_blob),
        PARAMS["outputs"]["png"]: checker_png(ck["size_px"], ck["cell_px"], ck["lo"], ck["hi"]),
        PARAMS["outputs"]["params"]: canonical_json(PARAMS),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True, help="产物输出目录（缓存内 cornell-box-generated/v1/）")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    artifacts = build()
    total = 0
    for name in sorted(artifacts):
        data = artifacts[name]
        (out / name).write_bytes(data)
        total += len(data)
        print(f"[gen_cornell] {name} bytes={len(data)} sha256={sha256_hex(data)}")
    print(f"[gen_cornell] total_bytes={total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
