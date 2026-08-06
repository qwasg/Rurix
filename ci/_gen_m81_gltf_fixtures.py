#!/usr/bin/env python3
"""One-shot generator for conformance/asset/gltf fixtures (M81)."""
from __future__ import annotations

import base64
import json
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ACCEPT = ROOT / "conformance" / "asset" / "gltf" / "accept"
REJECT = ROOT / "conformance" / "asset" / "gltf" / "reject"


def b64(data: bytes) -> str:
    return base64.b64encode(data).decode("ascii")


def png_1x1_red() -> bytes:
    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(
            ">I", zlib.crc32(tag + data) & 0xFFFFFFFF
        )

    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0)
    raw = zlib.compress(b"\x00\xff\x00\x00")
    return sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", raw) + chunk(b"IEND", b"")


def write_json(path: Path, obj: object) -> None:
    path.write_text(json.dumps(obj, separators=(",", ":")) + "\n", encoding="utf-8")


def main() -> None:
    ACCEPT.mkdir(parents=True, exist_ok=True)
    REJECT.mkdir(parents=True, exist_ok=True)

    pos = struct.pack("<9f", 0, 0, 0, 1, 0, 0, 0, 1, 0)

    write_json(
        ACCEPT / "tri_min.gltf",
        {
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "mode": 4}]}],
            "accessors": [
                {
                    "bufferView": 0,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC3",
                    "max": [1, 1, 0],
                    "min": [0, 0, 0],
                }
            ],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}],
            "buffers": [
                {
                    "byteLength": 36,
                    "uri": "data:application/octet-stream;base64," + b64(pos),
                }
            ],
        },
    )

    interleaved = b""
    for v in (
        (0, 0, 0, 0, 0, 1, 0, 0),
        (1, 0, 0, 0, 0, 1, 1, 0),
        (0, 1, 0, 0, 0, 1, 0, 1),
    ):
        interleaved += struct.pack("<8f", *v)
    write_json(
        ACCEPT / "mesh_normal_uv.gltf",
        {
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0, "name": "n"}],
            "meshes": [
                {
                    "name": "m",
                    "primitives": [
                        {
                            "attributes": {
                                "POSITION": 0,
                                "NORMAL": 1,
                                "TEXCOORD_0": 2,
                            },
                            "mode": 4,
                        }
                    ],
                }
            ],
            "accessors": [
                {
                    "bufferView": 0,
                    "byteOffset": 0,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC3",
                    "byteStride": 32,
                },
                {
                    "bufferView": 0,
                    "byteOffset": 12,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC3",
                    "byteStride": 32,
                },
                {
                    "bufferView": 0,
                    "byteOffset": 24,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC2",
                    "byteStride": 32,
                },
            ],
            "bufferViews": [
                {"buffer": 0, "byteLength": len(interleaved), "byteStride": 32}
            ],
            "buffers": [
                {
                    "byteLength": len(interleaved),
                    "uri": "data:application/octet-stream;base64," + b64(interleaved),
                }
            ],
        },
    )

    png = png_1x1_red()
    uv = struct.pack("<6f", 0, 0, 1, 0, 0, 1)
    blob = pos + uv
    write_json(
        ACCEPT / "unlit_textured.gltf",
        {
            "asset": {"version": "2.0"},
            "extensionsUsed": ["KHR_materials_unlit"],
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [
                {
                    "primitives": [
                        {
                            "attributes": {"POSITION": 0, "TEXCOORD_0": 1},
                            "material": 0,
                            "mode": 4,
                        }
                    ]
                }
            ],
            "materials": [
                {
                    "name": "u",
                    "extensions": {"KHR_materials_unlit": {}},
                    "pbrMetallicRoughness": {
                        "baseColorTexture": {"index": 0},
                        "baseColorFactor": [1, 1, 1, 1],
                        "metallicFactor": 0,
                        "roughnessFactor": 1,
                    },
                }
            ],
            "textures": [{"sampler": 0, "source": 0}],
            "samplers": [{}],
            "images": [{"uri": "data:image/png;base64," + b64(png)}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2"},
            ],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 24},
            ],
            "buffers": [
                {
                    "byteLength": len(blob),
                    "uri": "data:application/octet-stream;base64," + b64(blob),
                }
            ],
        },
    )

    write_json(
        ACCEPT / "two_nodes_hierarchy.gltf",
        {
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [
                {"name": "root", "children": [1], "translation": [0, 1, 0]},
                {"name": "child", "mesh": 0},
            ],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "mode": 4}]}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"}
            ],
            "bufferViews": [{"buffer": 0, "byteLength": 36}],
            "buffers": [
                {
                    "byteLength": 36,
                    "uri": "data:application/octet-stream;base64," + b64(pos),
                }
            ],
        },
    )

    verts4 = struct.pack("<12f", 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0)
    idx = struct.pack("<6H", 0, 1, 2, 0, 2, 3)
    bin_data = verts4 + idx
    assert len(bin_data) % 4 == 0
    quad_json = {
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [
            {
                "primitives": [
                    {"attributes": {"POSITION": 0}, "indices": 1, "mode": 4}
                ]
            }
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3"},
            {"bufferView": 1, "componentType": 5123, "count": 6, "type": "SCALAR"},
        ],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": 48},
            {"buffer": 0, "byteOffset": 48, "byteLength": 12},
        ],
        "buffers": [{"byteLength": len(bin_data)}],
    }
    js = json.dumps(quad_json, separators=(",", ":")).encode("utf-8")
    while len(js) % 4:
        js += b" "
    total = 12 + 8 + len(js) + 8 + len(bin_data)
    glb = struct.pack("<III", 0x46546C67, 2, total)
    glb += struct.pack("<II", len(js), 0x4E4F534A) + js
    glb += struct.pack("<II", len(bin_data), 0x004E4942) + bin_data
    (ACCEPT / "quad_indexed.glb").write_bytes(glb)

    write_json(
        REJECT / "reject_ext_outside_allowlist.gltf",
        {
            "asset": {"version": "2.0"},
            "extensionsRequired": ["EXT_meshopt_compression"],
            "scenes": [{"nodes": []}],
        },
    )
    write_json(
        REJECT / "reject_accessor_oob.gltf",
        {
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "mode": 4}]}],
            "accessors": [
                {
                    "bufferView": 0,
                    "componentType": 5126,
                    "count": 100,
                    "type": "VEC3",
                }
            ],
            "bufferViews": [{"buffer": 0, "byteLength": 36}],
            "buffers": [
                {
                    "byteLength": 36,
                    "uri": "data:application/octet-stream;base64," + b64(pos),
                }
            ],
        },
    )
    write_json(
        REJECT / "reject_missing_buffer.gltf",
        {
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "mode": 4}]}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"}
            ],
            "bufferViews": [{"buffer": 0, "byteLength": 36}],
            "buffers": [{"byteLength": 36, "uri": "does_not_exist.bin"}],
        },
    )
    (REJECT / "reject_dup_json_key.gltf").write_text(
        '{"asset":{"version":"2.0"},"asset":{"version":"2.0"}}\n', encoding="utf-8"
    )
    write_json(
        REJECT / "reject_node_cycle.gltf",
        {
            "asset": {"version": "2.0"},
            "nodes": [{"children": [1]}, {"children": [0]}],
            "scenes": [{"nodes": [0]}],
        },
    )
    print("ok")


if __name__ == "__main__":
    main()
