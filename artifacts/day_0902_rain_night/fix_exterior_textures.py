#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0902 雨夜街景战役:BistroExterior「无纹理臂」glTF 回接真实 DDS。

背景:8/15 FBX2glTF v0.9.7 对 BistroExterior 的有纹理转换(工具找到 Textures 目录)
在写 .gltf 时报 "Couldn't open file for writing"(根因未定,见
milestones/g10/g10_asset_license_registry.json 登记);无纹理臂(工具找不到贴图)成功
落盘,但 274 个 images[].uri 全是 1×1 PNG 的 data-URI 占位,真实贴图名只留在
images[].name / textures[].name 里。本脚本按名字把 uri 回接到原始
Textures\\<name>.dds,连同 buffer.bin 与 274 张被引用 DDS 一起落到
derived\\BistroExterior,使产物与 BistroInterior 派生产物同形:
  images[i]   = {"name": "<n>.dds", "uri": "<n>.dds"}   (Interior 144/144 name==uri 且带 .dds)
  textures[i] = {"name": "<n>", "sampler": 0, "source": i}(原样不动)
  buffers[0]  = {"byteLength": N, "uri": "buffer.bin"}   (原样不动,如实报告 uri 名)

纪律:
  * 只读 .tmp 源与 extracted\\Textures(后者在 G10 资产缓存 digest 覆盖面内,严禁写入);
    --out 若落在 extracted 树内、或等于源 glTF 所在目录,直接拒绝。
  * fail-closed:任一硬校验不满足 → 打印中文原因、退出码 1;报告 JSON 仍写出(ok=false)。
    写模式下前置校验(名字映射 / buffer)若已红,不落任何产物。
  * 幂等:目标已存在且字节数相同的文件跳过拷贝;glTF 字节一致则不重写。
  * 不拷贝任何未被 images[] 引用的 DDS/TGA。

用法:
  py -3 fix_exterior_textures.py [--src <gltf>] [--textures <dir>] [--out <dir>]
                                 [--verify-only] [--report <json>]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import struct
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent

DEFAULT_SRC = ROOT / ".tmp" / "g10_conv_ext" / "BistroExterior.gltf"
DEFAULT_TEXTURES = Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\extracted\Bistro_v5_2\Textures")
DEFAULT_OUT = Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroExterior")
DEFAULT_REPORT = HERE / "exterior_asset_verify.json"
INTERIOR_REF_GLTF = Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf")

EXPECTED_TRIANGLES = 2_832_120
EXPECTED_IMAGES = 274
EXPECTED_EMISSIVE_MATERIAL_INDICES = [1, 2, 3, 4, 5, 6, 12, 13, 38, 39]
# Interior 参照的条目键集合(images 无 mimeType)——输出按此形状重建 images[]
INTERIOR_IMAGE_KEYS = ("name", "uri")
INTERIOR_TEXTURE_KEYS = ("name", "sampler", "source")
INTERIOR_BUFFER_KEYS = ("byteLength", "uri")
INTERIOR_BUFFER_URI = "buffer.bin"
# 加载器 texture_mean_albedo 面只接受字面 fourCC DXT1/DXT5(DX10 扩展头也不进该面)
ALBEDO_FOURCC_OK = ("DXT1", "DXT5")
MATERIAL_SLOTS = ("normalTexture", "emissiveTexture", "occlusionTexture")
PBR_SLOTS = ("baseColorTexture", "metallicRoughnessTexture")

DXGI_NAMES = {
    70: "BC1_TYPELESS", 71: "BC1_UNORM", 72: "BC1_UNORM_SRGB",
    73: "BC2_TYPELESS", 74: "BC2_UNORM", 75: "BC2_UNORM_SRGB",
    76: "BC3_TYPELESS", 77: "BC3_UNORM", 78: "BC3_UNORM_SRGB",
    79: "BC4_TYPELESS", 80: "BC4_UNORM", 81: "BC4_SNORM",
    82: "BC5_TYPELESS", 83: "BC5_UNORM", 84: "BC5_SNORM",
    94: "BC6H_TYPELESS", 95: "BC6H_UF16", 96: "BC6H_SF16",
    97: "BC7_TYPELESS", 98: "BC7_UNORM", 99: "BC7_UNORM_SRGB",
}
COMPONENT_BYTES = {5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4}
TYPE_COMPONENTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT2": 4, "MAT3": 9, "MAT4": 16}
DDPF_FOURCC = 0x4

NOTE = (
    "来源 = FBX2glTF v0.9.7 无纹理臂(工具找不到贴图时以 1×1 PNG data-URI 占位,几何/材质/动画数据完整)"
    "+ 本脚本按 images[].name(缺失则经 textures[].source 反查 textures[].name)把 uri 回接为 Textures\\<name>.dds,"
    "贴图字节自原始 DDS 原样拷贝、未做任何转码;images[] 重建为 Interior 同形 {name: <n>.dds, uri: <n>.dds}。"
    "8/15 有纹理臂(工具找到 Textures 目录)在写 .gltf 时报 'Couldn't open file for writing',"
    "K:/H: 双盘、有无扩展名、--fbx-temp-dir 四臂同失败,写盘失败根因未定(见 milestones/g10/g10_asset_license_registry.json)。"
)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(8 << 20), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()


def sha256_bytes(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def dump_gltf(doc: dict) -> bytes:
    # FBX2glTF 输出 = 4 空格缩进、无尾换行、非 ASCII 原样;源与 Interior 均按此往返字节一致
    return json.dumps(doc, indent=4, ensure_ascii=False).encode("utf-8")


def load_gltf(path: Path) -> tuple[bytes, dict]:
    raw = path.read_bytes()
    return raw, json.loads(raw.decode("utf-8"))


def read_dds_header(path: Path) -> dict:
    """解析 DDS 头(magic + 124 字节头;fourCC@84;DX10 扩展头 DXGI 格式@128)。不合规抛 ValueError。"""
    with path.open("rb") as f:
        b = f.read(148)
    if len(b) < 128 or b[:4] != b"DDS ":
        raise ValueError("非 DDS magic 或头截断(<128 字节)")
    u32 = lambda off: struct.unpack_from("<I", b, off)[0]  # noqa: E731
    if u32(4) != 124:
        raise ValueError(f"DDS header.size = {u32(4)} ≠ 124")
    if u32(76) != 32:
        raise ValueError(f"DDS ddspf.size = {u32(76)} ≠ 32")
    pf_flags = u32(80)
    fourcc_raw = b[84:88]
    if pf_flags & DDPF_FOURCC:
        fourcc = fourcc_raw.decode("ascii") if all(32 <= c < 127 for c in fourcc_raw) else "0x%08x" % u32(84)
    else:
        fourcc = "(无 fourCC:未压缩像素格式)"
    dxgi = None
    if fourcc == "DX10":
        if len(b) < 148:
            raise ValueError("DX10 扩展头截断")
        dxgi = u32(128)
    return {
        "fourcc": fourcc,
        "dxgi_format": dxgi,
        "dxgi_name": DXGI_NAMES.get(dxgi) if dxgi is not None else None,
        "width": u32(16),
        "height": u32(12),
        "mip_count": u32(28),
    }


def fourcc_tag(row: dict) -> str:
    fc = row.get("fourcc")
    if fc is None:
        return "(头解析失败)"
    if row.get("dxgi_format") is not None:
        return f"{fc}/DXGI {row['dxgi_format']}({row.get('dxgi_name') or '未知'})"
    return fc


class Checks:
    """硬校验(红即退出码 1)与登记项(只记录,不判红)。"""

    def __init__(self) -> None:
        self.rows: list[dict] = []
        self.failures: list[str] = []

    def hard(self, cid: str, desc: str, ok: bool, detail=None) -> bool:
        self.rows.append({"id": cid, "desc": desc, "hard": True, "pass": bool(ok), "detail": detail})
        if not ok:
            self.failures.append(f"{cid}: {desc}" + (f" | {json.dumps(detail, ensure_ascii=False)[:600]}" if detail is not None else ""))
        return bool(ok)

    def soft(self, cid: str, desc: str, ok: bool, detail=None) -> bool:
        self.rows.append({"id": cid, "desc": desc, "hard": False, "pass": bool(ok), "detail": detail})
        return bool(ok)


def under(path: Path, ancestor: Path) -> bool:
    try:
        path.relative_to(ancestor)
        return True
    except ValueError:
        return False


def build_image_mapping(doc: dict, textures_dir: Path) -> tuple[list[dict], list[str]]:
    """逐 image 求真实贴图名 → <name>.dds;返回(映射表, 缺失清单)。"""
    names_by_source: dict[int, list[str]] = defaultdict(list)
    for t in doc.get("textures", []):
        if "source" in t and t.get("name"):
            names_by_source[t["source"]].append(t["name"])
    try:
        dir_listing = set(os.listdir(textures_dir))
    except OSError:
        dir_listing = set()
    rows: list[dict] = []
    missing: list[str] = []
    for i, im in enumerate(doc.get("images", [])):
        name = im.get("name") or None
        origin = "images[].name"
        if not name:
            cands = names_by_source.get(i, [])
            name = cands[0] if cands else None
            origin = "textures[].name(source 反查)"
        uri = im.get("uri", "")
        row = {
            "index": i,
            "fbx_texture_name": name,
            "name_origin": origin if name else None,
            "dds": None,
            "src_exists": False,
            "case_exact": False,
            "src_bytes": None,
            "had_data_uri": isinstance(uri, str) and uri.startswith("data:"),
            "dropped_keys": sorted(k for k in im if k not in INTERIOR_IMAGE_KEYS),
        }
        if not name:
            missing.append(f"images[{i}]: 无 name 且无 textures[].source 反查名")
            rows.append(row)
            continue
        stem = name[:-4] if name.lower().endswith(".dds") else name
        dds = f"{stem}.dds"
        src_file = textures_dir / dds
        row["dds"] = dds
        row["src_exists"] = src_file.is_file()
        row["case_exact"] = dds in dir_listing
        if row["src_exists"]:
            row["src_bytes"] = src_file.stat().st_size
        else:
            missing.append(dds)
        rows.append(row)
    return rows, missing


def copy_if_needed(src_file: Path, dst_file: Path) -> tuple[str, int]:
    """目标已存在且字节数相同即跳过(幂等);否则 copy2 保留 mtime。返回 (copied|skipped, 字节数)。"""
    size = src_file.stat().st_size
    if dst_file.is_file() and dst_file.stat().st_size == size:
        return "skipped", size
    dst_file.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src_file, dst_file)
    return "copied", size


def asset_census(rows: list[dict], buffer_uri: str, src_dir: Path, textures_dir: Path, out_dir: Path) -> dict:
    """输出目录 275 个资产文件(buffer + 274 DDS)与源逐个比字节数。"""
    identical = 0
    mismatched: list[str] = []
    pairs = [(src_dir / buffer_uri, out_dir / buffer_uri)]
    pairs += [(textures_dir / r["dds"], out_dir / r["dds"]) for r in rows if r["dds"]]
    for s, d in pairs:
        if s.is_file() and d.is_file() and s.stat().st_size == d.stat().st_size:
            identical += 1
        else:
            mismatched.append(d.name)
    return {"expected_files": len(pairs), "identical_size": identical, "missing_or_size_mismatch": mismatched}


def verify_output(gltf_path: Path, checks: Checks, textures_dir: Path) -> dict:
    """对输出 glTF 做全部硬校验与登记,返回 output/stats/fourcc/emissive/images 子报告。"""
    raw, doc = load_gltf(gltf_path)
    base = gltf_path.parent
    info: dict = {
        "dir": str(base),
        "gltf": str(gltf_path),
        "gltf_bytes": len(raw),
        "gltf_sha256": sha256_bytes(raw),
    }
    accessors = doc.get("accessors", [])
    buffer_views = doc.get("bufferViews", [])
    buffers = doc.get("buffers", [])
    textures = doc.get("textures", [])
    images = doc.get("images", [])
    materials = doc.get("materials", [])
    meshes = doc.get("meshes", [])

    # ---- buffers:恰一个、外链、文件存在、字节数 == byteLength
    buf_detail: dict = {"count": len(buffers)}
    buf_ok = len(buffers) == 1
    if buffers:
        b0 = buffers[0]
        uri = b0.get("uri")
        external = isinstance(uri, str) and not uri.startswith("data:")
        bpath = (base / uri) if external else None
        exists = bool(bpath and bpath.is_file())
        size = bpath.stat().st_size if exists else None
        buf_detail.update({
            "uri": uri,
            "external": external,
            "exists": exists,
            "bytes": size,
            "byteLength": b0.get("byteLength"),
            "keys": sorted(b0.keys()),
            "uri_same_as_interior": uri == INTERIOR_BUFFER_URI,
        })
        buf_ok = buf_ok and external and exists and size == b0.get("byteLength")
        if exists:
            info["buffer"] = str(bpath)
            info["buffer_bytes"] = size
            info["buffer_sha256"] = sha256_file(bpath)
    checks.hard("buffers", "buffers 恰一个且 uri 为外链、文件存在、字节数 == byteLength", buf_ok, buf_detail)

    # ---- bufferViews / accessors:无 sparse、有 bufferView、范围不越界
    bv_bad = []
    for j, bv in enumerate(buffer_views):
        bi = bv.get("buffer")
        if bi is None or bi >= len(buffers) or bv.get("byteLength") is None:
            bv_bad.append(j)
            continue
        if bv.get("byteOffset", 0) + bv["byteLength"] > buffers[bi].get("byteLength", -1):
            bv_bad.append(j)
    acc_sparse = [i for i, a in enumerate(accessors) if "sparse" in a]
    acc_nobv = [i for i, a in enumerate(accessors) if "bufferView" not in a]
    acc_oob = []
    for i, a in enumerate(accessors):
        if "bufferView" not in a or a["bufferView"] >= len(buffer_views):
            continue
        bv = buffer_views[a["bufferView"]]
        csz = COMPONENT_BYTES.get(a.get("componentType"))
        ncomp = TYPE_COMPONENTS.get(a.get("type"))
        count = a.get("count", 0)
        if csz is None or ncomp is None:
            acc_oob.append(i)
            continue
        elem = csz * ncomp
        stride = bv.get("byteStride", elem)
        end = a.get("byteOffset", 0) + ((count - 1) * stride + elem if count > 0 else 0)
        if end > bv.get("byteLength", 0):
            acc_oob.append(i)
    checks.hard("accessors", "全部 accessor 无 sparse 且有 bufferView;bufferView/accessor 范围不越界",
                not (acc_sparse or acc_nobv or acc_oob or bv_bad),
                {"accessors": len(accessors), "bufferViews": len(buffer_views), "sparse": acc_sparse[:20],
                 "no_bufferView": acc_nobv[:20], "accessor_out_of_range": acc_oob[:20], "bufferView_out_of_range": bv_bad[:20]})

    # ---- primitives:mode 4、有 indices 与 POSITION;三角总数
    tri = 0
    prims = 0
    bad_mode: list[list[int]] = []
    no_idx: list[list[int]] = []
    no_pos: list[list[int]] = []
    not_mult3: list[list[int]] = []
    no_material = 0
    for mi, m in enumerate(meshes):
        for pi, p in enumerate(m.get("primitives", [])):
            prims += 1
            if p.get("mode", 4) != 4:
                bad_mode.append([mi, pi])
            if "POSITION" not in p.get("attributes", {}):
                no_pos.append([mi, pi])
            if "material" not in p:
                no_material += 1
            if "indices" not in p:
                no_idx.append([mi, pi])
                continue
            cnt = accessors[p["indices"]].get("count", 0) if p["indices"] < len(accessors) else 0
            if cnt % 3:
                not_mult3.append([mi, pi])
            tri += cnt // 3
    checks.hard("primitives", "全部 primitive mode == 4(缺省即 4)且有 indices 与 attributes.POSITION",
                not (bad_mode or no_idx or no_pos),
                {"primitives": prims, "bad_mode": bad_mode[:20], "no_indices": no_idx[:20], "no_POSITION": no_pos[:20],
                 "indices_count_not_multiple_of_3": not_mult3[:20], "no_material": no_material})
    checks.hard("triangles", f"三角总数(Σ indices.count/3)== {EXPECTED_TRIANGLES:,}", tri == EXPECTED_TRIANGLES,
                {"triangles": tri, "expected": EXPECTED_TRIANGLES})

    # ---- images:uri 为纯相对 .dds 文件名、文件存在、DDS 头合规;按材质槽位归角色
    tex_name_by_source: dict[int, str] = {}
    for t in textures:
        if "source" in t:
            tex_name_by_source.setdefault(t["source"], t.get("name"))
    roles_by_image: dict[int, set] = defaultdict(set)
    role_refs: list[tuple[int, str, int]] = []  # (material_index, role, image_index)
    for mi, m in enumerate(materials):
        slots = [(k, m[k]) for k in MATERIAL_SLOTS if k in m]
        pbr = m.get("pbrMetallicRoughness", {})
        slots += [(k, pbr[k]) for k in PBR_SLOTS if k in pbr]
        for role, ref in slots:
            ti = ref.get("index")
            src_i = textures[ti].get("source") if isinstance(ti, int) and ti < len(textures) else None
            if src_i is not None:
                roles_by_image[src_i].add(role)
                role_refs.append((mi, role, src_i))
    image_rows: list[dict] = []
    missing_files: list[str] = []
    bad_uri: list[int] = []
    header_errors: list[str] = []
    size_mismatch: list[str] = []
    fourcc_by_role: dict[str, Counter] = defaultdict(Counter)
    fourcc_all: Counter = Counter()
    for i, im in enumerate(images):
        uri = im.get("uri")
        plain = (isinstance(uri, str) and uri and not uri.startswith("data:")
                 and "/" not in uri and "\\" not in uri and uri.lower().endswith(".dds"))
        if not plain:
            bad_uri.append(i)
        p = (base / uri) if plain else None
        exists = bool(p and p.is_file())
        if not exists:
            missing_files.append(uri if plain else f"images[{i}]")
        row: dict = {
            "index": i,
            "name": im.get("name"),
            "uri": uri if plain else (str(uri)[:40] + "..." if isinstance(uri, str) and len(uri) > 40 else uri),
            "texture_name": tex_name_by_source.get(i),
            "roles": sorted(roles_by_image.get(i, ())),
            "bytes": None,
            "fourcc": None,
            "dxgi_format": None,
            "dxgi_name": None,
            "width": None,
            "height": None,
            "mip_count": None,
            "same_bytes_as_textures_dir": None,
        }
        if exists:
            row["bytes"] = p.stat().st_size
            try:
                row.update(read_dds_header(p))
            except ValueError as e:
                header_errors.append(f"{uri}: {e}")
            src_p = textures_dir / uri
            row["same_bytes_as_textures_dir"] = src_p.is_file() and src_p.stat().st_size == row["bytes"]
            if not row["same_bytes_as_textures_dir"]:
                size_mismatch.append(uri)
            tag = fourcc_tag(row)
            fourcc_all[tag] += 1
            for r in (row["roles"] or ["(未被材质引用)"]):
                fourcc_by_role[r][tag] += 1
        image_rows.append(row)
    checks.hard("image_count", f"images 数 == {EXPECTED_IMAGES}", len(images) == EXPECTED_IMAGES,
                {"images": len(images), "textures": len(textures)})
    checks.hard("images_exist", f"{len(images) - len(missing_files)}/{len(images)} images[].uri 为纯相对 .dds 文件名且文件存在(glTF 所在目录.join(uri))",
                not (bad_uri or missing_files),
                {"bad_uri": bad_uri[:20], "missing": missing_files[:20], "missing_count": len(missing_files)})
    checks.hard("dds_headers", "全部被引用 DDS 头合规(magic 'DDS ' + header.size 124 + ddspf.size 32)",
                not header_errors, {"errors": header_errors[:20]})
    checks.hard("dds_same_size_as_textures_dir", "输出目录每张 DDS 与 Textures 源同字节数(原样拷贝)",
                not size_mismatch, {"mismatch": size_mismatch[:20]})
    checks.soft("images_all_referenced", "每个 image 至少被一个材质槽位引用", all(r["roles"] for r in image_rows),
                {"unreferenced": [r["index"] for r in image_rows if not r["roles"]][:20]})
    checks.soft("images_shape_interior", "images[] 键集合 == Interior 形状 {name, uri} 且 name == uri",
                all(tuple(sorted(im.keys())) == tuple(sorted(INTERIOR_IMAGE_KEYS)) and im.get("name") == im.get("uri") for im in images),
                {"key_sets": sorted({",".join(sorted(im.keys())) for im in images})})
    checks.soft("textures_shape_interior", "textures[] 键集合 == Interior 形状 {name, sampler, source}",
                all(tuple(sorted(t.keys())) == tuple(sorted(INTERIOR_TEXTURE_KEYS)) for t in textures),
                {"key_sets": sorted({",".join(sorted(t.keys())) for t in textures})})
    checks.soft("buffers_shape_interior", "buffers[0] 键集合 == Interior 形状 {byteLength, uri} 且 uri == 'buffer.bin'",
                bool(buffers) and tuple(sorted(buffers[0].keys())) == tuple(sorted(INTERIOR_BUFFER_KEYS)) and buffers[0].get("uri") == INTERIOR_BUFFER_URI,
                {"uri": buffers[0].get("uri") if buffers else None})

    # ---- baseColor fourCC 硬门(加载器 texture_mean_albedo 只吃 DXT1/DXT5)
    albedo_bad: list[dict] = []
    albedo_seen = 0
    mats_without_basecolor: list[int] = []
    for mi, m in enumerate(materials):
        ref = m.get("pbrMetallicRoughness", {}).get("baseColorTexture")
        if ref is None:
            mats_without_basecolor.append(mi)
            continue
        ti = ref.get("index")
        src_i = textures[ti].get("source") if isinstance(ti, int) and ti < len(textures) else None
        row = image_rows[src_i] if src_i is not None and src_i < len(image_rows) else None
        albedo_seen += 1
        if row is None or row.get("fourcc") not in ALBEDO_FOURCC_OK:
            albedo_bad.append({"material_index": mi, "name": m.get("name"),
                               "uri": row["uri"] if row else None, "fourcc": fourcc_tag(row) if row else None})
    checks.hard("basecolor_fourcc", "每个材质 baseColorTexture 对应 DDS fourCC ∈ {DXT1, DXT5}", not albedo_bad,
                {"materials_with_baseColorTexture": albedo_seen, "bad": albedo_bad[:20],
                 "materials_without_baseColorTexture": mats_without_basecolor[:20]})

    # ---- emissive 材质表(登记)
    emissive: list[dict] = []
    for mi, m in enumerate(materials):
        if "emissiveTexture" not in m and "emissiveFactor" not in m:
            continue
        row = {"material_index": mi, "name": m.get("name"), "emissiveFactor": m.get("emissiveFactor"),
               "emissive_texture": None, "uri": None, "fourcc": None, "dxgi_format": None}
        ref = m.get("emissiveTexture")
        if ref is not None:
            ti = ref.get("index")
            src_i = textures[ti].get("source") if isinstance(ti, int) and ti < len(textures) else None
            if src_i is not None and src_i < len(image_rows):
                ir = image_rows[src_i]
                row.update({"emissive_texture": textures[ti].get("name"), "uri": ir["uri"],
                            "fourcc": ir.get("fourcc"), "dxgi_format": ir.get("dxgi_format")})
        emissive.append(row)
    checks.soft("emissive_indices", "emissive 材质下标与预期 1–6/12/13/38/39 一致(登记)",
                [r["material_index"] for r in emissive] == EXPECTED_EMISSIVE_MATERIAL_INDICES,
                {"actual": [r["material_index"] for r in emissive], "expected": EXPECTED_EMISSIVE_MATERIAL_INDICES})
    checks.soft("emissive_fourcc", "emissive 贴图 fourCC 全部 ∈ {DXT1, DXT5}(登记,非硬门)",
                all(r["fourcc"] in ALBEDO_FOURCC_OK for r in emissive if r["emissive_texture"] is not None),
                {"fourcc": dict(Counter(r["fourcc"] for r in emissive if r["emissive_texture"] is not None))})
    checks.soft("normal_fourcc", "normal 贴图 fourCC 分布(预期 ATI2,仅登记)",
                set(fourcc_by_role.get("normalTexture", {}).keys()) <= {"ATI2"},
                {"fourcc": dict(fourcc_by_role.get("normalTexture", {}))})

    # ---- 输出目录普查:仅 glTF + buffer + 被引用 DDS,无多余文件
    entries = list(base.iterdir())
    files = [p for p in entries if p.is_file()]
    referenced = {gltf_path.name} | {b.get("uri") for b in buffers if isinstance(b.get("uri"), str)} | {r["uri"] for r in image_rows if r["uri"]}
    extra = sorted(p.name + ("/" if p.is_dir() else "") for p in entries if p.name not in referenced)
    info.update({
        "file_count": len(files),
        "total_bytes": sum(p.stat().st_size for p in files),
        "by_ext": dict(sorted(Counter(p.suffix.lower() for p in files).items())),
        "extra_entries": extra,
    })
    checks.soft("out_dir_no_extra", "输出目录仅含 glTF + buffer + 被引用 DDS(无多余条目)", not extra, {"extra": extra[:20]})

    stats = {
        "nodes": len(doc.get("nodes", [])),
        "meshes": len(meshes),
        "primitives": prims,
        "materials": len(materials),
        "textures": len(textures),
        "images": len(images),
        "accessors": len(accessors),
        "bufferViews": len(buffer_views),
        "animations": len(doc.get("animations", [])),
        "cameras": len(doc.get("cameras", [])),
        "triangles": tri,
        "generator": doc.get("asset", {}).get("generator"),
        "extensionsUsed": doc.get("extensionsUsed"),
        "extensionsRequired": doc.get("extensionsRequired"),
    }
    fourcc = {
        "by_role": {k: dict(v) for k, v in sorted(fourcc_by_role.items())},
        "all": dict(fourcc_all),
        "dx10_count": sum(1 for r in image_rows if r.get("dxgi_format") is not None),
        "mip_count_distribution": dict(sorted(Counter(r["mip_count"] for r in image_rows if r["mip_count"] is not None).items())),
        "size_distribution": {f"{w}x{h}": n for (w, h), n in sorted(Counter((r["width"], r["height"]) for r in image_rows if r["width"] is not None).items(), key=lambda kv: -kv[1])},
    }
    return {"output": info, "stats": stats, "fourcc": fourcc, "emissive_materials": emissive, "images": image_rows}


def interior_reference_check(checks: Checks) -> dict:
    """可选:若 Interior 参照 glTF 在位,核对其 images/textures/buffers 形状与本脚本硬编码常量一致。"""
    if not INTERIOR_REF_GLTF.is_file():
        checks.soft("interior_reference", "Interior 参照 glTF 在位并与硬编码形状常量一致(缺位则跳过)", True,
                    {"path": str(INTERIOR_REF_GLTF), "skipped": True})
        return {"path": str(INTERIOR_REF_GLTF), "present": False}
    _, ref = load_gltf(INTERIOR_REF_GLTF)
    imgs = ref.get("images", [])
    texs = ref.get("textures", [])
    bufs = ref.get("buffers", [])
    detail = {
        "path": str(INTERIOR_REF_GLTF),
        "present": True,
        "images": len(imgs),
        "image_key_sets": sorted({",".join(sorted(im.keys())) for im in imgs}),
        "images_name_eq_uri": all(im.get("name") == im.get("uri") for im in imgs),
        "images_name_dds_suffix": all(str(im.get("name", "")).endswith(".dds") for im in imgs),
        "textures_name_plus_dds_eq_image_name": all(
            str(t.get("name")) + ".dds" == imgs[t["source"]].get("name") for t in texs if "source" in t and t["source"] < len(imgs)),
        "texture_key_sets": sorted({",".join(sorted(t.keys())) for t in texs}),
        "buffer_key_sets": sorted({",".join(sorted(b.keys())) for b in bufs}),
        "buffer_uri": [b.get("uri") for b in bufs],
    }
    ok = (detail["image_key_sets"] == [",".join(sorted(INTERIOR_IMAGE_KEYS))]
          and detail["images_name_eq_uri"] and detail["images_name_dds_suffix"]
          and detail["texture_key_sets"] == [",".join(sorted(INTERIOR_TEXTURE_KEYS))]
          and detail["buffer_key_sets"] == [",".join(sorted(INTERIOR_BUFFER_KEYS))]
          and detail["buffer_uri"] == [INTERIOR_BUFFER_URI])
    checks.soft("interior_reference", "Interior 参照 glTF 在位并与硬编码形状常量一致(缺位则跳过)", ok, detail)
    return detail


def finish(report: dict, checks: Checks, report_path: Path, t0: float) -> int:
    report["checks"] = checks.rows
    report["failures"] = checks.failures
    report["ok"] = not checks.failures
    report["run"]["wall_s"] = round(time.monotonic() - t0, 2)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"报告已写出: {report_path}")
    for row in checks.rows:
        if row["hard"]:
            tag = "PASS" if row["pass"] else "FAIL"
        else:
            tag = "登记 是" if row["pass"] else "登记 否"
        print(f"  [{tag}] {row['id']}: {row['desc']}")
    if checks.failures:
        print("FAIL: 以下硬校验不满足(fail-closed,退出码 1):")
        for f in checks.failures:
            print("  - " + f)
        return 1
    print("PASS: 全部硬校验通过")
    return 0


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    ap = argparse.ArgumentParser(description="BistroExterior 无纹理臂 glTF 回接真实 DDS(Interior 同形派生产物)")
    ap.add_argument("--src", default=str(DEFAULT_SRC), help="源 glTF(FBX2glTF 无纹理臂产物,只读)")
    ap.add_argument("--textures", default=str(DEFAULT_TEXTURES), help="原始 DDS 贴图目录(只读,G10 digest 覆盖面)")
    ap.add_argument("--out", default=str(DEFAULT_OUT), help="派生产物输出目录(不存在则创建)")
    ap.add_argument("--verify-only", action="store_true", help="只校验既有输出,不写任何资产")
    ap.add_argument("--report", default=str(DEFAULT_REPORT), help="校验报告 JSON 路径")
    args = ap.parse_args()

    t0 = time.monotonic()
    src = Path(args.src).resolve()
    textures_dir = Path(args.textures).resolve()
    out_dir = Path(args.out).resolve()
    report_path = Path(args.report).resolve()
    mode = "verify-only" if args.verify_only else "write"
    checks = Checks()
    report: dict = {
        "schema": "rurix.day0902.exterior_asset_verify.v1",
        "generated_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "tool": "artifacts/day_0902_rain_night/fix_exterior_textures.py",
        "asset": "Amazon Lumberyard Bistro v5_2(ORCA,CC-BY-4.0)BistroExterior",
        "run": {"mode": mode, "argv": sys.argv[1:], "src": str(src), "textures": str(textures_dir), "out": str(out_dir)},
        "ok": False,
        "note": NOTE,
    }
    print(f"模式: {mode}\n源 glTF: {src}\n贴图目录: {textures_dir}\n输出目录: {out_dir}")

    # ---- 前置守卫(fail-closed)
    if not checks.hard("src_exists", "源 glTF 存在", src.is_file(), {"src": str(src)}):
        return finish(report, checks, report_path, t0)
    if not checks.hard("textures_dir_exists", "贴图目录存在", textures_dir.is_dir(), {"textures": str(textures_dir)}):
        return finish(report, checks, report_path, t0)
    extracted_root = next((p for p in [textures_dir, *textures_dir.parents] if p.name.lower() == "extracted"), None)
    guard_ok = not (extracted_root and under(out_dir, extracted_root)) and out_dir != src.parent and out_dir != textures_dir
    if not checks.hard("out_dir_guard", "输出目录不在 extracted 树内(G10 digest 覆盖面)、不等于源目录/贴图目录",
                       guard_ok, {"out": str(out_dir), "extracted_root": str(extracted_root) if extracted_root else None}):
        return finish(report, checks, report_path, t0)

    # ---- 读源、名字映射
    raw_src, doc = load_gltf(src)
    report["source"] = {
        "gltf": str(src),
        "gltf_bytes": len(raw_src),
        "gltf_sha256": sha256_bytes(raw_src),
        "generator": doc.get("asset", {}).get("generator"),
    }
    checks.soft("source_json_roundtrip", "源 glTF 以 indent=4/ensure_ascii=False 往返字节一致(输出沿用同一序列化风格)",
                dump_gltf(doc) == raw_src)
    rows, missing = build_image_mapping(doc, textures_dir)
    report["missing_textures"] = missing
    checks.hard("source_image_count", f"源 images 数 == {EXPECTED_IMAGES}", len(rows) == EXPECTED_IMAGES, {"images": len(rows)})
    checks.hard("texture_mapping", "每个 image 均能按名在 Textures 目录找到 <name>.dds(缺失清单为空)", not missing,
                {"missing_count": len(missing), "missing": missing[:30],
                 "name_origin": dict(Counter(r["name_origin"] for r in rows)),
                 "had_data_uri": sum(1 for r in rows if r["had_data_uri"]),
                 "case_exact": sum(1 for r in rows if r["case_exact"])})
    checks.soft("texture_name_case_exact", "贴图文件名与目录列表大小写精确一致(跨平台卫生)",
                all(r["case_exact"] for r in rows if r["src_exists"]),
                {"case_inexact": [r["dds"] for r in rows if r["src_exists"] and not r["case_exact"]][:20]})
    dropped = Counter(k for r in rows for k in r["dropped_keys"])
    report["images_shape"] = {
        "policy": "images[i] 重建为 Interior 同形 {name: <n>.dds, uri: <n>.dds};textures[]/buffers[] 原样不动",
        "image_keys": list(INTERIOR_IMAGE_KEYS),
        "dropped_keys": dict(dropped),
        "data_uri_removed": sum(1 for r in rows if r["had_data_uri"]),
        "source_image_name_had_dds_suffix": sum(1 for r in rows if r["fbx_texture_name"] and r["fbx_texture_name"].lower().endswith(".dds")),
        "interior_reference": interior_reference_check(checks),
    }

    # ---- 源 buffer(拷贝前提)
    buffers = doc.get("buffers", [])
    buf_uri = buffers[0].get("uri") if len(buffers) == 1 else None
    buf_external = isinstance(buf_uri, str) and not buf_uri.startswith("data:")
    src_buf = (src.parent / buf_uri) if buf_external else None
    src_buf_ok = bool(src_buf and src_buf.is_file() and src_buf.stat().st_size == buffers[0].get("byteLength"))
    report["source"].update({
        "buffer_uri": buf_uri,
        "buffer_uri_same_as_interior": buf_uri == INTERIOR_BUFFER_URI,
        "buffer_bytes": src_buf.stat().st_size if src_buf and src_buf.is_file() else None,
        "buffer_sha256": sha256_file(src_buf) if src_buf and src_buf.is_file() else None,
    })
    checks.hard("source_buffer", "源 buffers 恰一个、uri 外链、文件存在且字节数 == byteLength", src_buf_ok,
                {"count": len(buffers), "uri": buf_uri, "byteLength": buffers[0].get("byteLength") if buffers else None,
                 "bytes": report["source"]["buffer_bytes"]})

    # ---- 重建 images[](Interior 同形)并序列化
    doc["images"] = [{"name": r["dds"], "uri": r["dds"]} for r in rows if r["dds"]] if not missing else doc["images"]
    new_bytes = dump_gltf(doc)
    out_gltf = out_dir / src.name

    if checks.failures:
        print("前置校验已红,不落任何产物。")
        return finish(report, checks, report_path, t0)

    if args.verify_only:
        if not checks.hard("out_gltf_exists", "输出 glTF 存在(--verify-only 需先有产物)", out_gltf.is_file(), {"out_gltf": str(out_gltf)}):
            return finish(report, checks, report_path, t0)
        existing = out_gltf.read_bytes()
        checks.hard("gltf_unchanged", "输出 glTF 字节 == 由源重建的字节(幂等;不一致即陈旧或被改动)",
                    existing == new_bytes, {"out_bytes": len(existing), "rebuilt_bytes": len(new_bytes)})
        report["copy"] = {"mode": "verify-only(未写任何文件)"}
    else:
        out_dir.mkdir(parents=True, exist_ok=True)
        copied = skipped = 0
        copied_bytes = skipped_bytes = 0
        plan = [(src_buf, out_dir / buf_uri)] + [(textures_dir / r["dds"], out_dir / r["dds"]) for r in rows]
        total = len(plan)
        t_copy = time.monotonic()
        for n, (s, d) in enumerate(plan, 1):
            state, size = copy_if_needed(s, d)
            if state == "copied":
                copied += 1
                copied_bytes += size
            else:
                skipped += 1
                skipped_bytes += size
            if n % 25 == 0 or n == total:
                print(f"  拷贝进度 {n}/{total}  新拷贝 {copied} 个 {copied_bytes / 2**20:,.1f} MiB  跳过(同字节数) {skipped} 个  "
                      f"{time.monotonic() - t_copy:,.1f}s", flush=True)
        gltf_written = not (out_gltf.is_file() and out_gltf.read_bytes() == new_bytes)
        if gltf_written:
            out_gltf.write_bytes(new_bytes)
        report["copy"] = {
            "mode": "write",
            "planned_files": total,
            "copied": copied,
            "copied_bytes": copied_bytes,
            "skipped_identical_size": skipped,
            "skipped_bytes": skipped_bytes,
            "gltf_written": gltf_written,
            "copy_wall_s": round(time.monotonic() - t_copy, 2),
        }
        print(f"glTF {'已写出' if gltf_written else '字节一致,未重写'}: {out_gltf}")

    # ---- 资产普查 + 输出 glTF 全量校验
    census = asset_census(rows, buf_uri, src.parent, textures_dir, out_dir)
    report["copy"].update(census)
    checks.hard("assets_same_size_as_source", f"输出目录 {census['expected_files']} 个资产文件(buffer + DDS)与源逐个同字节数",
                not census["missing_or_size_mismatch"],
                {"identical_size": census["identical_size"], "mismatch": census["missing_or_size_mismatch"][:20]})
    if out_gltf.is_file():
        sub = verify_output(out_gltf, checks, textures_dir)
        report.update(sub)
        report["output"]["buffer_sha256_equals_source"] = (
            report["output"].get("buffer_sha256") is not None
            and report["output"].get("buffer_sha256") == report["source"].get("buffer_sha256"))
        print(f"输出 glTF: {out_gltf}\n  {report['output']['gltf_sha256']}  ({report['output']['gltf_bytes']:,} B)")
        if report["output"].get("buffer_sha256"):
            print(f"buffer.bin: {report['output']['buffer_sha256']}  ({report['output']['buffer_bytes']:,} B)"
                  f"  与源一致: {report['output']['buffer_sha256_equals_source']}")
        print(f"输出目录: {report['output']['file_count']} 个文件, {report['output']['total_bytes']:,} B; 三角 {report['stats']['triangles']:,}")
        print(f"fourCC 按角色: {json.dumps(report['fourcc']['by_role'], ensure_ascii=False)}")
    return finish(report, checks, report_path, t0)


if __name__ == "__main__":
    sys.exit(main())
