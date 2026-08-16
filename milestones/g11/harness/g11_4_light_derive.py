#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.4 波）
"""G11.4 R3 灯种子集派生链（spec/global_illumination.md RXS-0394 L2/L3：
光源参数唯一事实源 = 契约光照参数面 corpus/lighting_*.json；包内 glTF 字段
= 派生输入，经本脚本转入契约光照 JSON——语料修订走 M133 只追加修订程序）。

输入（只读）：
- bistro 包内 glTF（K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/
  BistroInterior.gltf——Rurix 消费面；G10.5 语料 0-byte）；
- G11.3 DDS 转码 manifest（milestones/g11/g11_3_dds_transcode_manifest.json——
  emissive 纹理线性均值取径：转码 PNG 产物与 bcdec rgba8 逐位同值，digest 链
  入 manifest）。

派生规则（RXS-0394 L3 冻结）：
- pointLight1~N 节点（包内实测 4 盏，世界位姿 = 节点树组合）→ 点光源；
- 关联灯具 = 最近 emissive 网格节点（ceiling/wall light fixtures）；
- Le = emissiveFactor × emissiveTexture 线性均值（sRGB→线性 IEC 分段逐
  texel）；A = 关联灯具 emissive 三角形世界表面积；发光轴向 = emissive 三角形
  面积加权平均法线；**轴向点强 I₀ = Le × A**（朗伯发射 I(θ)=I₀·cosθ，
  ∫I dΩ = Le·A·π = Φ）；
- 色 = Le 按亮度归一（hue 保真），强度载于 intensity_cd。

产出：
1. corpus/lighting_bistro_interior.json 修订（只追加 point_lights /
   emissive_surfaces / derived 三键——既有 lights[]/note 0-byte；
   gen_contract_params 仅消费 lights[type=directional] ⇒ 契约 digest 锁定
   值 0-byte 机核）；
2. g10_corpus_scene_manifest.json 只追加修订行（revision 2；scenes 行集
   0-byte ⇒ manifest digest 不变，修订行登记本次内容修订与光照文件 digest）；
3. milestones/g11/g11_4_light_derivation.json 派生报告（逐盏 provenance）。

用法：py -3 milestones/g11/harness/g11_4_light_derive.py [--check-only]
"""
from __future__ import annotations

import hashlib
import json
import math
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
CORPUS = ROOT / "milestones" / "g10" / "corpus"
GLTF_PATH = Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroInterior\BistroInterior.gltf")
TRANSCODE_MANIFEST = ROOT / "milestones" / "g11" / "g11_3_dds_transcode_manifest.json"
LIGHTING_JSON = CORPUS / "lighting_bistro_interior.json"
SCENE_MANIFEST = ROOT / "milestones" / "g10" / "g10_corpus_scene_manifest.json"
REPORT_PATH = ROOT / "milestones" / "g11" / "g11_4_light_derivation.json"


def sha256_file(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def srgb_to_linear(c: np.ndarray) -> np.ndarray:
    c = c.astype(np.float64) / 255.0
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def quat_to_m3(q):
    x, y, z, w = q
    return np.array([
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ])


def compose_world(nodes):
    """节点树世界变换（与 bin 侧 compose 同语义：TRS 4×4，父链组合）。"""
    n = len(nodes)
    world = [None] * n
    parent = [None] * n
    for i, node in enumerate(nodes):
        for ch in node.get("children", []):
            parent[ch] = i

    def local(node):
        if "matrix" in node:
            return np.array(node["matrix"], dtype=np.float64).reshape(4, 4).T
        t = node.get("translation", [0, 0, 0])
        r = node.get("rotation", [0, 0, 0, 1])
        s = node.get("scale", [1, 1, 1])
        m = np.eye(4)
        m[:3, :3] = quat_to_m3(r) * np.array(s)
        m[:3, 3] = t
        return m

    def rec(i):
        if world[i] is not None:
            return world[i]
        m = local(nodes[i])
        p = parent[i]
        world[i] = rec(p) @ m if p is not None else m
        return world[i]

    for i in range(n):
        rec(i)
    return world


def read_positions(root, buffers, mesh_idx, prim_idx):
    mesh = root["meshes"][mesh_idx]
    prim = mesh["primitives"][prim_idx]
    acc = root["accessors"][prim["attributes"]["POSITION"]]
    bv = root["bufferViews"][acc["bufferView"]]
    buf = buffers[bv.get("buffer", 0)]
    base = bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
    stride = bv.get("byteStride", 12)
    out = np.zeros((acc["count"], 3), dtype=np.float64)
    for i in range(acc["count"]):
        off = base + i * stride
        out[i] = np.frombuffer(buf, dtype="<f4", count=3, offset=off)
    idx_acc = root["accessors"][prim["indices"]]
    bv2 = root["bufferViews"][idx_acc["bufferView"]]
    buf2 = buffers[bv2.get("buffer", 0)]
    base2 = bv2.get("byteOffset", 0) + idx_acc.get("byteOffset", 0)
    ctype = idx_acc["componentType"]
    dt = {5121: np.uint8, 5123: np.uint16, 5125: np.uint32}[ctype]
    indices = np.frombuffer(buf2, dtype=dt, count=idx_acc["count"], offset=base2).astype(np.int64)
    return out, indices.reshape(-1, 3)


def main() -> int:
    check_only = "--check-only" in sys.argv
    root = json.loads(GLTF_PATH.read_text(encoding="utf-8"))
    base = GLTF_PATH.parent
    buffers = [(base / b["uri"]).read_bytes() for b in root.get("buffers", [])]
    nodes = root["nodes"]
    world = compose_world(nodes)

    # ① pointLight 节点（闭集：名前缀 pointLight）。
    pl_nodes = [(i, n) for i, n in enumerate(nodes) if str(n.get("name", "")).startswith("pointLight")]
    if len(pl_nodes) < 4:
        raise SystemExit(f"pointLight 节点 < 4（实测 {len(pl_nodes)}）——包内实测面漂移")
    pl_positions = {i: (world[i] @ np.array([0, 0, 0, 1.0]))[:3] for i, _ in pl_nodes}

    # ② emissive 材质与纹理均值（转码 manifest digest 链）。
    man = json.loads(TRANSCODE_MANIFEST.read_text(encoding="utf-8"))
    out_dir = Path(man["output_dir"])
    by_uri = {e["source_uri"]: e for e in man["entries"]}
    images = root.get("images", [])
    emissive_mats = []
    for mi, m in enumerate(root.get("materials", [])):
        ef = m.get("emissiveFactor")
        et = m.get("emissiveTexture")
        if not ef and not et:
            continue
        le_tex = None
        tex_ref = None
        if et is not None:
            tex = root["textures"][et["index"]]
            img = images[tex["source"]]
            uri = img["uri"]
            entry = by_uri.get(uri)
            if entry is None:
                raise SystemExit(f"emissive 纹理 {uri} 未在转码 manifest（未登记资产即 RED）")
            png = out_dir / entry["product_png"]
            arr = np.asarray(Image.open(png).convert("RGB"), dtype=np.float64)
            lin = srgb_to_linear(arr.reshape(-1, 3))
            le_tex = lin.mean(axis=0)
            tex_ref = {"source_uri": uri, "source_digest": entry["source_digest"],
                       "product_digest": entry["product_digest"]}
        factor = np.array(ef[:3] if ef else [1.0, 1.0, 1.0], dtype=np.float64)
        le = factor * (le_tex if le_tex is not None else np.ones(3))
        emissive_mats.append({
            "material_index": mi, "name": m.get("name", f"mat{mi}"),
            "factor": factor, "le": le, "texture_ref": tex_ref,
        })
    if len(emissive_mats) != 4:
        raise SystemExit(f"emissive 材质数 {len(emissive_mats)} ≠ 4（包内实测面漂移）")

    # ③ emissive 灯具几何（网格节点 × emissive 材质图元）：面积 + 面积加权法线。
    fixtures = []
    for ni, node in enumerate(nodes):
        if "mesh" not in node:
            continue
        mesh = root["meshes"][node["mesh"]]
        for pi, prim in enumerate(mesh["primitives"]):
            mat = prim.get("material")
            em = next((e for e in emissive_mats if e["material_index"] == mat), None)
            if em is None:
                continue
            pos, tris = read_positions(root, buffers, node["mesh"], pi)
            w = world[ni]
            pts = (w[:3, :3] @ pos.T).T + w[:3, 3]
            a = pts[tris[:, 0]]
            b = pts[tris[:, 1]]
            c = pts[tris[:, 2]]
            cross = np.cross(b - a, c - a)
            area2 = np.linalg.norm(cross, axis=1)
            total_area = float(area2.sum() / 2.0)
            nrm = (cross / np.maximum(area2, 1e-30)[:, None])
            axis = (nrm * (area2 / 2.0)[:, None]).sum(axis=0)
            alen = float(np.linalg.norm(axis))
            axis = axis / alen if alen > 1e-12 else np.array([0.0, -1.0, 0.0])
            centroid = pts.reshape(-1, 3).mean(axis=0)
            fixtures.append({
                "node_index": ni, "node_name": node.get("name", f"node{ni}"),
                "material_index": mat, "area_m2": total_area,
                "emit_axis": axis, "centroid": centroid,
            })

    # ④ 点光源派生：最近灯具关联 + I₀ = Le × A。
    point_lights = []
    for ni, node in pl_nodes:
        p = pl_positions[ni]
        best = min(fixtures, key=lambda f: float(np.linalg.norm(f["centroid"] - p)))
        em = next(e for e in emissive_mats if e["material_index"] == best["material_index"])
        le = em["le"]
        lum = float(0.2126 * le[0] + 0.7152 * le[1] + 0.0722 * le[2])
        color = (le / lum).tolist() if lum > 0 else [1.0, 1.0, 1.0]
        intensity = lum * best["area_m2"]
        point_lights.append({
            "id": node.get("name"),
            "node_name": node.get("name"),
            "position": [float(v) for v in p],
            "color_linear_rgb": [float(v) for v in color],
            "intensity_cd": intensity,
            "emit_direction": [float(v) for v in best["emit_axis"]],
            "area_m2": best["area_m2"],
            "covers_material_index": best["material_index"],
            "derived_from": (
                f"glTF 节点 {node.get('name')} 位姿 + 关联灯具 {best['node_name']}"
                f"（最近质心关联）：Le={le.tolist()}（emissiveFactor×emissiveTexture 线性均值）"
                f" × A={best['area_m2']:.6f} m² ⇒ I₀=Le×A（朗伯轴向）；"
                f"covers_material_index={best['material_index']}（NEE 覆盖面——GI 面 Le 整零排除防双重计数）"
            ),
        })

    emissive_surfaces = []
    for em in emissive_mats:
        f_area = sum(f["area_m2"] for f in fixtures if f["material_index"] == em["material_index"])
        emissive_surfaces.append({
            "material_index": em["material_index"],
            "material_name": em["name"],
            "le_linear_rgb": [float(v) for v in em["le"]],
            "area_m2": f_area,
            "texture_ref": em["texture_ref"],
        })

    report = {
        "schema_version": 1,
        "report": "g11_4_light_derivation",
        "generated_by": "milestones/g11/harness/g11_4_light_derive.py",
        "source_gltf": str(GLTF_PATH),
        "source_gltf_digest": sha256_file(GLTF_PATH),
        "transcode_manifest_digest": sha256_file(TRANSCODE_MANIFEST),
        "derivation_rule": "RXS-0394 L3：I₀ = Le × A（Φ=Le·A·π 朗伯半球通量 ⇒ 轴向点强 I₀=Φ/π）；发光轴向 = emissive 三角形面积加权平均法线；近场钳制 d²_eff = max(d², A/π)",
        "point_lights": point_lights,
        "emissive_surfaces": emissive_surfaces,
        "area_lights": [],
        "area_lights_note": "bistro 包内无 area/spot 灯节点（缺类显式登记，不冒充空集）",
    }

    # ⑤ 契约光照 JSON 修订（只追加三键；lights[]/note 0-byte）。
    doc = json.loads(LIGHTING_JSON.read_text(encoding="utf-8"))
    doc["point_lights"] = [
        {k: pl[k] for k in ("id", "node_name", "position", "color_linear_rgb",
                            "intensity_cd", "emit_direction", "area_m2",
                            "covers_material_index", "derived_from")}
        for pl in point_lights
    ]
    doc["emissive_surfaces"] = emissive_surfaces
    doc["derived"] = {
        "by": "milestones/g11/harness/g11_4_light_derive.py",
        "at": "2026-08-16",
        "rule": "RXS-0394 L3（I₀=Le×A；朗伯余弦瓣；近场钳制 d²_eff=max(d²,A/π)）",
        "report": "milestones/g11/g11_4_light_derivation.json",
    }

    # ⑥ 清单只追加修订行（scenes 行集 0-byte ⇒ manifest digest 不变）。
    manifest = json.loads(SCENE_MANIFEST.read_text(encoding="utf-8"))
    new_digest = sha256_file(LIGHTING_JSON)  # 修订前 digest 留痕于 note（修订后 digest 由门复算）
    rev_ids = [r["revision"] for r in manifest.get("revisions", [])]
    latest_digest = manifest["revisions"][-1]["manifest_digest"]
    # 幂等面：本批修订行已在列（同 G11.4 标记）⇒ 不重复追加（复跑 0-byte）。
    already = any("G11.4 R3 灯种子集承接" in str(r.get("change_note", "")) for r in manifest.get("revisions", []))
    next_rev = max(rev_ids) + 1 if rev_ids else 1
    revision = {
        "revision": next_rev,
        "manifest_digest": latest_digest,
        "changed_at": "2026-08-16",
        "change_note": (
            "G11.4 R3 灯种子集承接（M133 只追加修订程序）：corpus/lighting_bistro_interior.json "
            "内容修订——只追加 point_lights（4 盏派生）/emissive_surfaces（4 件）/derived 三键，"
            "既有 lights[] 与 note 0-byte（gen_contract_params 仅消费 lights[type=directional]，"
            "契约 digest 锁定值 0-byte）；scenes 行集 0-byte ⇒ 清单 digest 维持 "
            f"{latest_digest[:24]}…；派生链 milestones/g11/harness/g11_4_light_derive.py + "
            "报告 milestones/g11/g11_4_light_derivation.json（逐盏 provenance）；"
            "cornell 灯面 0-byte（lighting_cornell_box.json 不动）"
        ),
    }

    if check_only:
        print(json.dumps({"point_lights": len(point_lights), "emissive": len(emissive_surfaces),
                          "revision": next_rev, "already": already}, ensure_ascii=False))
        return 0

    LIGHTING_JSON.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    if not already:
        manifest["revisions"].append(revision)
        SCENE_MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    report["lighting_json_digest_pre_revision"] = new_digest
    report["lighting_json_digest_post_revision"] = sha256_file(LIGHTING_JSON)
    REPORT_PATH.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[g11_4_light_derive] point_lights={len(point_lights)} emissive={len(emissive_surfaces)}")
    for pl in point_lights:
        print(f"  {pl['id']}: I={pl['intensity_cd']:.6f} cd area={pl['area_m2']:.4f} m² axis={[round(v,3) for v in pl['emit_direction']]}")
    print(f"[g11_4_light_derive] 修订行 revision={next_rev}（{'已在列跳过' if already else '落盘'}）+ 报告 {REPORT_PATH.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
