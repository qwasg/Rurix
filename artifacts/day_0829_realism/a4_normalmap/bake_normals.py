# -*- coding: utf-8 -*-
"""Day 0829 臂④ 法线贴图烘焙侧车（预制;确定性可重跑;不触 src/kernels）。

bistro-interior glTF 的 materials[].normalTexture → textures[].source →
images[].uri（BC5/ATI2 DDS）→ mip0 解码为 RG 双通道 → RGB PNG（B=128 常量
占位;切线空间 Z 运行时由 sqrt(1−x²−y²) 重建,PNG 是人眼可核验中间产物,
非运行时消费容器——em 臂先例:运行时读 .rgba8bin 而非 PNG,仓内无 PNG 解码器）
+ manifest.json。

槽号律法 = g14_3_lane_body.rs g31_tex_load_heap top-70 同律：
  逐材质三角数降序,并列 material_index 升序（rank.sort_by(b.1.cmp(&a.1)
  .then(a.0.cmp(&b.0))）;三角数与 assemble_scene_ex_nrm 的 scene.tri_mat
  构建同源——按 nodes[] 数组序遍历所有带 mesh 的节点（非场景图可达性）,
  每 primitive 强制 mode==4 且必有 indices,三角数 = accessors[indices]
  .count/3,无 material 的 primitive 不计（SLAB_TRI_NONE 同义）。

BC5 解码律 = 标准 D3D/bcdec BC4 插值表 ×2 通道：
  每 16B 块 = R 通道 BC4 块（前 8B）+ G 通道 BC4 块（后 8B）;
  BC4 块 = e0,e1 两端点 byte + 48bit（16×3bit,LE 位序）索引;
  e0>e1: pal[n] = ((8−n)·e0 + (n−1)·e1)//7 （n∈2..7）
  e0≤e1: pal[n] = ((6−n)·e0 + (n−1)·e1)//5 （n∈2..5）, pal[6]=0, pal[7]=255
  （整数地板除,与 C 非负整除同值）。
【接线注意/诚实登记】lane_body 现行 bc4_alpha（仅 BC3-A 面,kernel 不消费
故一直休眠）系数族为 (8−(n−1))/7 与 (6−(n−1))/5,较标准表多 +e0/7（clamp
掩盖）——臂④运行时接线让 BC4 律法首次承重,必须用本文件的标准表,勿照搬。

DDS 头闭集（mirror lane_body g31_dds_decode_rgba8_mips 检查面）：
  magic "DDS " / header.size==124 / ddspf.size==32 / h@12 w@16 mips@28
  （0 视作 1）/ fourCC@84 ∈ {ATI2, BC5U} 或 DX10 扩展头 DXGI∈{82,83}
  （BC5_TYPELESS/UNORM;84=SNORM 显式拒绝）;mip0 体截断即异常。

非法常值检测（day_0830 W1 加性修复;HANDOVER §H slot14 登记项）:
  mip0 整张常值且 ‖xy‖₁>1（(byte−127)/127 解码）⇒ 全链替换平坦 (127,127),
  打印告警并入 manifest 逐条登记（detect_illegal_const_rg,pack 侧同源消费）。

用法: python bake_normals.py [--limit N] [--gltf PATH] [--out DIR]
产物: baked_normals/slotNN_<材质名>.png + baked_normals/manifest.json
退出码: 0 = 全部成功;2 = 存在异常（anomalies 进 manifest + stderr）。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image

DEFAULT_GLTF = Path(
    "K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf"
)
OUT_DIR = Path(__file__).resolve().parent / "baked_normals"

DXGI_BC5_OK = {82: "BC5_TYPELESS", 83: "BC5_UNORM"}
DXGI_BC5_SNORM = 84


def sha256(b: bytes) -> str:
    return "sha256:" + hashlib.sha256(b).hexdigest()


# ---------------------------------------------------------------------------
# 槽号律法（g31_tex_load_heap top-70 镜像）
# ---------------------------------------------------------------------------

def material_tri_counts(doc: dict) -> dict[int, int]:
    """scene.tri_mat 同源三角计数（nodes[] 数组序;mode==4/indices 强制,
    与 assemble_scene_ex_nrm fail-closed 同律——违规即 raise,槽序不允许
    与运行时装配漂移）。"""
    nodes = doc.get("nodes")
    meshes = doc.get("meshes")
    accessors = doc.get("accessors")
    if nodes is None or meshes is None or accessors is None:
        raise SystemExit("FAIL: glTF 缺 nodes/meshes/accessors（fail-closed）")
    counts: dict[int, int] = {}
    for ni, n in enumerate(nodes):
        mesh_idx = n.get("mesh")
        if mesh_idx is None:
            continue
        mesh = meshes[mesh_idx]
        for prim in mesh["primitives"]:
            if prim.get("mode", 4) != 4:
                raise SystemExit(f"FAIL: node {ni} 非三角形 primitive（mode≠4,装配同律 fail-closed）")
            idx_acc = prim.get("indices")
            if idx_acc is None:
                raise SystemExit(f"FAIL: node {ni} primitive 缺 indices（装配同律 fail-closed）")
            icount = accessors[idx_acc]["count"]
            if icount % 3 != 0:
                raise SystemExit(f"FAIL: node {ni} indices {icount} 非 3 整除")
            mi = prim.get("material")
            if mi is None:
                continue  # SLAB_TRI_NONE 同义,不入 rank
            counts[mi] = counts.get(mi, 0) + icount // 3
    return counts


def rank_slots(doc: dict) -> list[tuple[int, int, int]]:
    """→ [(slot, material_index, tris)] 按 heap 槽序（三角数降序,并列
    material_index 升序;top-70 与全集同长——bistro 70 材质全有三角）。"""
    counts = material_tri_counts(doc)
    rank = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
    n_map = min(70, len(rank))  # G31_TEX_N_MAPPED_HEAP = 70
    return [(k, mi, tris) for k, (mi, tris) in enumerate(rank[:n_map])]


# ---------------------------------------------------------------------------
# BC5 解码（标准 D3D/bcdec BC4 表;numpy 向量化）
# ---------------------------------------------------------------------------

def parse_dds_header(b: bytes, uri: str) -> tuple[int, int, int, str, int | None, int]:
    """→ (w, h, mips_raw, fourcc, dxgi|None, data_off);闭集破坏即 ValueError。"""
    if len(b) < 128 or b[0:4] != b"DDS ":
        raise ValueError(f"{uri}: 非 DDS magic/头截断")
    rd = lambda o: struct.unpack_from("<I", b, o)[0]
    if rd(4) != 124:
        raise ValueError(f"{uri}: DDS header.size ≠ 124")
    h, w = rd(12), rd(16)
    if w == 0 or h == 0:
        raise ValueError(f"{uri}: DDS 零尺寸")
    if rd(76) != 32:
        raise ValueError(f"{uri}: DDS ddspf.size ≠ 32")
    mips_raw = rd(28)
    fourcc = b[84:88].decode("ascii", errors="replace")
    dxgi: int | None = None
    data_off = 128
    if fourcc in ("ATI2", "BC5U"):
        pass
    elif fourcc == "DX10":
        if len(b) < 148:
            raise ValueError(f"{uri}: DDS DX10 扩展头截断")
        dxgi = rd(128)
        data_off = 148
        if dxgi == DXGI_BC5_SNORM:
            raise ValueError(f"{uri}: DXGI 84 = BC5_SNORM（本烘焙面只钉 UNORM,显式拒绝）")
        if dxgi not in DXGI_BC5_OK:
            raise ValueError(f"{uri}: DXGI 格式 {dxgi} 未入 BC5 消费闭集")
    else:
        raise ValueError(f"{uri}: fourCC {fourcc!r} 非 BC5（ATI2/BC5U/DX10 闭集,fail-closed）")
    return w, h, mips_raw, fourcc, dxgi, data_off


def decode_bc4_plane(blocks8: np.ndarray) -> np.ndarray:
    """(n,8) uint8 BC4 块 → (n,16) uint8 texel（标准插值表,整数地板除）。"""
    e0 = blocks8[:, 0].astype(np.int64)
    e1 = blocks8[:, 1].astype(np.int64)
    bits = np.zeros(len(blocks8), dtype=np.uint64)
    for i in range(6):
        bits |= blocks8[:, 2 + i].astype(np.uint64) << np.uint64(8 * i)
    idx = np.empty((len(blocks8), 16), dtype=np.int64)
    for t in range(16):
        idx[:, t] = ((bits >> np.uint64(3 * t)) & np.uint64(7)).astype(np.int64)
    pal = np.empty((len(blocks8), 8), dtype=np.int64)
    pal[:, 0] = e0
    pal[:, 1] = e1
    gt = e0 > e1
    for n in range(2, 8):
        v_gt = ((8 - n) * e0 + (n - 1) * e1) // 7
        if n <= 5:
            v_le = ((6 - n) * e0 + (n - 1) * e1) // 5
        elif n == 6:
            v_le = np.zeros_like(e0)
        else:
            v_le = np.full_like(e0, 255)
        pal[:, n] = np.where(gt, v_gt, v_le)
    return np.take_along_axis(pal, idx, axis=1).astype(np.uint8)


def decode_bc5_mip0(b: bytes, w: int, h: int, data_off: int, uri: str) -> np.ndarray:
    """DDS 体 mip0 → (h,w,2) uint8（X=R 块, Y=G 块）。"""
    bw = (w + 3) // 4
    bh = (h + 3) // 4
    need = bw * bh * 16
    if data_off + need > len(b):
        raise ValueError(f"{uri}: DDS mip0 体截断（需 {need}B,存 {len(b) - data_off}B）")
    blocks = np.frombuffer(b, dtype=np.uint8, count=need, offset=data_off).reshape(-1, 16)
    planes = []
    for half in (blocks[:, :8], blocks[:, 8:16]):
        texels = decode_bc4_plane(np.ascontiguousarray(half))
        img = texels.reshape(bh, bw, 4, 4).transpose(0, 2, 1, 3).reshape(bh * 4, bw * 4)
        planes.append(img[:h, :w])
    return np.stack(planes, axis=-1)


# ---------------------------------------------------------------------------
# 非法常值检测（day_0830 W1 修复面:HANDOVER §H slot14 源件损坏登记项）
# ---------------------------------------------------------------------------

FLAT_RG = (127, 127)  # 平坦法线 = +Z（(byte−127)/127 解码下 x=y=0）


def detect_illegal_const_rg(rg: np.ndarray) -> dict | None:
    """mip0 整张常值且 ‖xy‖₁=|x|+|y|>1（(byte−127)/127 解码）⇒ 登记 dict;否则 None。

    HANDOVER §H slot14（Paris_Table_cloth_01_Normal.dds）:整张 (53,53) →
    x=y≈−0.583,‖xy‖₁=1.165>1（‖xy‖₂=0.824<1——登记语句"‖xy‖>1"唯 L1
    自洽,判据如实取 L1）。判据域实测（day_0829 v1 全 70 张）:常值件 15 张,
    仅 slot14 非平坦,其余 14 张常值恰为 (127,127)（范数 0）不触发。
    修复律 = 全链替换 FLAT_RG 平坦（法线=+Z）,打印告警并入 manifest 登记。
    """
    r_lo, r_hi = int(rg[..., 0].min()), int(rg[..., 0].max())
    g_lo, g_hi = int(rg[..., 1].min()), int(rg[..., 1].max())
    if r_lo != r_hi or g_lo != g_hi:
        return None
    x = (r_lo - 127) / 127.0
    y = (g_lo - 127) / 127.0
    l1 = abs(x) + abs(y)
    if l1 <= 1.0:
        return None
    return {
        "reason": "illegal_const_normal",
        "const_rg": [r_lo, g_lo],
        "decoded_xy": [round(x, 6), round(y, 6)],
        "norm_l1": round(l1, 6),
        "norm_l2": round((x * x + y * y) ** 0.5, 6),
        "replaced_with_rg": list(FLAT_RG),
        "law": "mip0 整张常值且 ‖xy‖₁>1（(byte−127)/127 解码）⇒ 全链替换平坦 (127,127)（法线=+Z）;HANDOVER §H 登记修法",
    }


def flat_like(rg: np.ndarray) -> np.ndarray:
    """同形平坦 (127,127) RG 图（非法常值件替换用）。"""
    out = np.empty_like(rg)
    out[..., 0] = FLAT_RG[0]
    out[..., 1] = FLAT_RG[1]
    return out


# ---------------------------------------------------------------------------
# 烘焙主流程
# ---------------------------------------------------------------------------

def safe_name(name: str, mi: int) -> str:
    s = re.sub(r"[^0-9A-Za-z_\-]", "_", name)
    return s if s else f"mat{mi}"


def bake_one(slot: int, mi: int, tris: int, doc: dict, base: Path, out_dir: Path = OUT_DIR) -> dict:
    mat = doc["materials"][mi]
    name = mat.get("name", "")
    nt = mat.get("normalTexture")
    if nt is None:
        raise ValueError(f"材质 {mi}（{name}）缺 normalTexture")
    ti = nt["index"]
    src_idx = doc["textures"][ti]["source"]
    uri = doc["images"][src_idx]["uri"]
    raw = (base / uri).read_bytes()
    w, h, mips_raw, fourcc, dxgi, data_off = parse_dds_header(raw, uri)
    rg = decode_bc5_mip0(raw, w, h, data_off, uri)
    sanitized = detect_illegal_const_rg(rg)
    if sanitized is not None:
        print(
            f"WARN: slot{slot:02d} mat{mi} {name} 非法常值法线 rg={tuple(sanitized['const_rg'])}"
            f" |x|+|y|={sanitized['norm_l1']}>1 → 替换平坦 {FLAT_RG}（HANDOVER §H 修复律）"
        )
        rg = flat_like(rg)

    png = np.empty((h, w, 3), dtype=np.uint8)
    png[..., 0:2] = rg
    png[..., 2] = 128  # B 常量占位（Z 运行时重建,不入 PNG）
    out = out_dir / f"slot{slot:02d}_{safe_name(name, mi)}.png"
    Image.fromarray(png, mode="RGB").save(out)
    png_bytes = out.read_bytes()

    entry = {
        "slot": slot,
        "material_index": mi,
        "material_name": name,
        "tris": tris,
        "source_uri": uri.replace("\\", "/"),
        "width": w,
        "height": h,
        "mips_raw": mips_raw,
        "mips_effective": max(1, mips_raw),
        "fourcc": fourcc,
        "dxgi_format": dxgi,
        "source_sha256": sha256(raw),
        "file": out.name,
        "png_sha256": sha256(png_bytes),
        "mean_xy": [round(float(rg[..., 0].mean()), 3), round(float(rg[..., 1].mean()), 3)],
        "min_xy": [int(rg[..., 0].min()), int(rg[..., 1].min())],
        "max_xy": [int(rg[..., 0].max()), int(rg[..., 1].max())],
    }
    if sanitized is not None:
        entry["sanitized"] = sanitized
    return entry


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--limit", type=int, default=None, help="只烘焙前 N 个槽（试跑）")
    ap.add_argument("--gltf", type=Path, default=DEFAULT_GLTF)
    ap.add_argument("--out", type=Path, default=OUT_DIR, help="输出目录（默认 baked_normals/;v2 重烘不覆盖旧目录用）")
    args = ap.parse_args()

    gltf_bytes = args.gltf.read_bytes()
    doc = json.loads(gltf_bytes.decode("utf-8"))
    base = args.gltf.parent
    mats = doc.get("materials", [])
    with_nt = sum(1 for m in mats if "normalTexture" in m)
    print(f"glTF: {args.gltf}  materials={len(mats)}  with_normalTexture={with_nt}")

    slots = rank_slots(doc)
    if len(slots) != len(mats):
        print(f"WARN: rank 槽数 {len(slots)} ≠ 材质总数 {len(mats)}（存在零三角材质,槽序与 albedo heap 仍一致）")
    todo = slots[: args.limit] if args.limit is not None else slots
    args.out.mkdir(parents=True, exist_ok=True)

    entries: list[dict] = []
    anomalies: list[dict] = []
    for slot, mi, tris in todo:
        name = mats[mi].get("name", "")
        try:
            e = bake_one(slot, mi, tris, doc, base, args.out)
            entries.append(e)
            print(
                f"slot{slot:02d} mat{mi:>3} {name:<44} {e['width']}x{e['height']}"
                f" mips={e['mips_effective']:>2} {e['fourcc']}"
                f" meanXY=({e['mean_xy'][0]:7.3f},{e['mean_xy'][1]:7.3f})"
            )
        except (ValueError, OSError, KeyError) as ex:
            anomalies.append({"slot": slot, "material_index": mi, "material_name": name, "error": str(ex)})
            print(f"slot{slot:02d} mat{mi:>3} {name:<44} ANOMALY: {ex}", file=sys.stderr)

    manifest = {
        "schema": "rurix.day0829.a4_normalmap.bake_manifest.v1",
        "gltf": str(args.gltf).replace("\\", "/"),
        "gltf_sha256": sha256(gltf_bytes),
        "slot_law": "g31_tex_load_heap top-70 同律:逐材质三角数降序,并列 material_index 升序;三角数=nodes[]序×primitive accessors[indices].count/3（mode==4/indices 强制,无 material 不计）",
        "bc5_law": "BC5=R/G 两 BC4 块（各 8B:2 端点+16×3bit LE）;标准 D3D/bcdec 表 gt:((8-n)e0+(n-1)e1)//7, le:((6-n)e0+(n-1)e1)//5,idx6=0,idx7=255;整数地板除",
        "wiring_note": "lane_body bc4_alpha 现行系数族 (8-(n-1))/7 较标准多 +e0/7（BC3-A 休眠面 kernel 不消费）;臂④运行时接线必须用标准表",
        "png_law": "RGB8: R=X, G=Y, B=128 常量;mip0-only（源 DDS 自带完整 mip 链,运行时接线按 DDS 逐级解码零重采样或 .rgba8bin 容器路线,PNG 非运行时消费件）",
        "counts": {
            "materials_total": len(mats),
            "with_normal_texture": with_nt,
            "slots_ranked": len(slots),
            "baked": len(entries),
            "anomalies": len(anomalies),
        },
        "limit": args.limit,
        "entries": entries,
        "anomalies": anomalies,
    }
    (args.out / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=1), encoding="utf-8"
    )
    print(f"baked={len(entries)} anomalies={len(anomalies)} → {args.out / 'manifest.json'}")
    return 0 if not anomalies else 2


if __name__ == "__main__":
    sys.exit(main())
