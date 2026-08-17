#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G12.4 UE PT 对标波）
"""G12.4 harness — PT 双绕向派生语料生成器（UE 5.8.1 Interchange 导入面在 Path
Tracing 下对反射映射〔C:(x,z,y)·100,det=−1〕后的单绕向三角逐面剔除（RT 遍历
背面剔除;双面材质 MIC/父材质/网格槽位重挂三路实测不接线——G12.4 探针取证面〕,
双面几何补齐经内容恒等双绕向派生承载：逐三角 (a,b,c) 追加反向 (a,c,b),顶点/
UV/材质索引/节点变换逐字节不动;Rurix 臂续消费原语料〔同一表面集〕）。

派生物（K: 外部缓存,零入 git;digest 进 evidence/契约 provenance 登记）：
  K:/rurix-ext/g12-assets/cornell-box-pt2sided/cornell_box.gltf(+bin)
  K:/rurix-ext/g12-assets/bistro-interior-ue-pt2sided/BistroInterior.gltf(+bin+PNG 纹
  理按 URI 相对引用原样拷贝)

用法：py -3 milestones/g12/harness/g12_4_make_pt2sided.py
"""
from __future__ import annotations

import hashlib
import json
import shutil
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

SOURCES = {
    "cornell-box": (
        Path(r"K:\rurix_g10_cache\cornell-box-generated\v1\cornell_box.gltf"),
        Path(r"K:\rurix-ext\g12-assets\cornell-box-pt2sided"),
    ),
    "bistro-interior": (
        Path(r"K:\rurix-ext\g11-assets\bistro-interior-ue\BistroInterior.gltf"),
        Path(r"K:\rurix-ext\g12-assets\bistro-interior-ue-pt2sided"),
    ),
}


def sha256_file(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


def make_2sided(src_gltf: Path, dst_dir: Path) -> dict:
    doc = json.loads(src_gltf.read_text(encoding="utf-8"))
    base = src_gltf.parent
    # 外部 buffer 逐字节拷贝后改写索引段。
    buffers = doc.get("buffers", [])
    if len(buffers) != 1:
        raise RuntimeError("本生成器只消费单 buffer 语料")
    bin_src = base / buffers[0]["uri"]
    blob = bytearray(bin_src.read_bytes())
    accessors = doc["accessors"]
    views = doc["bufferViews"]
    # 逐 mesh.primitive 索引 accessor 双绕向扩写（新 accessor/bufferView 追加;
    # 原 accessor 集合 0-byte 不动——新增段追加于 bin 尾部）。
    new_accessors = list(accessors)
    new_views = list(views)
    for mesh in doc.get("meshes", []):
        for prim in mesh.get("primitives", []):
            ai = prim.get("indices")
            if ai is None:
                continue
            acc = accessors[ai]
            ct = acc.get("componentType")
            if ct == 5125:
                fmt, unit = "I", 4
            elif ct == 5123:
                fmt, unit = "H", 2
            else:
                raise RuntimeError(f"索引 componentType {ct} 不在消费面（5123/5125）")
            bv = views[acc["bufferView"]]
            start = bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
            count = acc["count"]
            idx = list(struct.unpack_from("<%d%s" % (count, fmt), blob, start))
            if count % 3 != 0:
                raise RuntimeError("索引数非 3 整除")
            out = []
            for t in range(0, count, 3):
                a, b, c = idx[t], idx[t + 1], idx[t + 2]
                out += [a, b, c]
                out += [a, c, b]
            payload = struct.pack("<%d%s" % (len(out), fmt), *out)
            new_view = {
                "buffer": 0,
                "byteOffset": len(blob),
                "byteLength": len(payload),
                "target": 34963,
            }
            blob += payload
            new_views.append(new_view)
            new_acc = {
                "bufferView": len(new_views) - 1,
                "componentType": ct,
                "count": len(out),
                "type": "SCALAR",
            }
            new_accessors.append(new_acc)
            prim["indices"] = len(new_accessors) - 1
    doc["accessors"] = new_accessors
    doc["bufferViews"] = new_views
    doc["buffers"][0]["byteLength"] = len(blob)
    dst_dir.mkdir(parents=True, exist_ok=True)
    dst_bin = dst_dir / buffers[0]["uri"]
    dst_bin.write_bytes(bytes(blob))
    dst_gltf = dst_dir / src_gltf.name
    dst_gltf.write_text(json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
    # 纹理等相对引用文件原样拷贝（URI 不透明保留;仅补缺不覆盖）。
    copied = 0
    for im in doc.get("images", []):
        uri = im.get("uri")
        if not uri:
            continue
        src_f = base / uri
        dst_f = dst_dir / uri
        if src_f.is_file() and not dst_f.is_file():
            dst_f.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src_f, dst_f)
            copied += 1
    return {
        "gltf": dst_gltf.name,
        "gltf_sha256": sha256_file(dst_gltf),
        "bin_sha256": sha256_file(dst_bin),
        "bytes": len(blob),
        "textures_copied": copied,
    }


def main() -> int:
    report = {}
    for scene_id, (src, dst) in SOURCES.items():
        if not src.is_file():
            print(f"缺源语料: {src}", file=sys.stderr)
            return 1
        report[scene_id] = make_2sided(src, dst)
        print(f"[g12_4_make_pt2sided] {scene_id}: {report[scene_id]['gltf_sha256'][:24]}… bytes={report[scene_id]['bytes']}")
    out = ROOT / "milestones" / "g12" / "g12_4_pt2sided_derivation.json"
    out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[g12_4_make_pt2sided] 派生报告 → {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
