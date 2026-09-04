#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0902_rain_night：BistroExterior 场景事实分析（numpy 只读，memmap buffer）。

输出 exterior_scene_facts.json（全部为 **世界坐标**，根节点 BistroExterior 带 1.6 均匀缩放，
加载器按 parent×local 完整 TRS 累乘 ⇒ 世界坐标 = 1.6 × 局部坐标）：
  - 节点世界矩阵摘要 / 全场景世界 AABB / 地面 Y 估计（cobble·pavement·curb 类顶点 Y 中位数）
  - 10 个 emissive 材质逐材质统计（三角数 / 世界面积 / AABB / 面积加权质心 / emissive DDS 线性均值）
  - 材质 12（Emissive_StreetLight）按节点拆分：路灯玻璃罩（点光候选位 = 玻璃 AABB 底面中心再下移 0.35 m）
    / 吊灯笼 / 檐下小发光面；空间聚类（质心距 > 1.5 m 分簇）复核
  - 店招 38/39：AABB、PCA 最薄轴 = 法线、中心、朝街侧判定
  - 作者相机世界位姿（位置 / forward / up / fov_y_deg / 契约 orientation_quat(w,x,y,z)）
  - 机位候选 C1/C2/C3：eye/target/up/fov/near/far/quat、视锥内且射线未被遮挡的灯清单（Möller–Trumbore，
    AABB 粗筛 + 向量化）、粗略材质占比（96×54 深度缓冲光栅）、视点净空检查、树冠占比、发射器建议

用法：
  py -3 -B analyze_exterior_scene.py [--gltf <path>] [--out <json>] [--textures <dir>] [--preview-dir <dir>]

fail-closed：输入缺失 / 解析异常 / 数值异常 ⇒ 中文原因 + 非 0 退出。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent

DEFAULT_GLTF = Path(r"H:\rurix\.tmp\g10_conv_ext\BistroExterior.gltf")
DEFAULT_TEXTURES = Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\extracted\Bistro_v5_2\Textures")
DEFAULT_OUT = HERE / "exterior_scene_facts.json"

EMISSIVE_INDICES = (1, 2, 3, 4, 5, 6, 12, 13, 38, 39)
STREETLIGHT_MAT = 12
SHOPSIGN_MATS = (38, 39)

# 地面类材质关键字（cobblestone / wet pavement / curbstone）
GROUND_KEYWORDS = ("Pavement_Cobble", "Pavement_Ground_Wet", "Pavement_Curbstones", "Pavement_Brick",
                   "Pavement_Manhole")

# 点光候选位（任务口径）：玻璃 AABB 底面中心再向下 0.35 m（灯罩为闭合盒，点光须在盒外侧下方 0.3–0.5 m）
POINT_LIGHT_DROP_M = 0.35
# 逃逸测试：从点光位向下半球射线（长度 ESCAPE_RAY_M）未被遮挡的比例；口径位若被灯具下部包住则改选逃逸最优位
ESCAPE_RAY_M = 4.0
ESCAPE_ACCEPT = 0.6

# 机位候选公共参数
EYE_HEIGHT_M = 1.7
CAND_FOV_Y_DEG = 52.0
CAND_NEAR = 0.05
CAND_FAR = 500.0
CAND_RES = (1920, 1080)
CLEARANCE_RADIUS_M = 0.6

# 发射器建议参数（与命令行口径一致）
EMITTER_AHEAD_M = 12.0
EMITTER_DEPTH_MAX_M = 25.0
EMITTER_DEPTH_MIN_M = 1.0
EMITTER_HALF_HEIGHT_M = 1.5
EMITTER_CENTER_ABOVE_GROUND_M = 10.0
EMITTER_VEL = (0.4, -9.0, 0.2)
EMITTER_VEL_SPREAD = (0.3, 0.5, 0.3)
EMITTER_GRAVITY = -3.0

RASTER_W, RASTER_H = 96, 54


def fail(msg: str, code: int = 2) -> None:
    print(f"FAIL: {msg}", file=sys.stderr)
    sys.exit(code)


def r3(x):
    """数值保留 3 位小数（递归处理 list / tuple / ndarray / dict）。"""
    if isinstance(x, (bool, np.bool_)):
        return bool(x)
    if isinstance(x, (float, np.floating)):
        v = round(float(x), 3)
        return 0.0 if v == 0 else v
    if isinstance(x, (int, np.integer, str)) or x is None:
        return int(x) if isinstance(x, np.integer) else x
    if isinstance(x, np.ndarray):
        return [r3(v) for v in x.tolist()]
    if isinstance(x, (list, tuple)):
        return [r3(v) for v in x]
    if isinstance(x, dict):
        return {k: r3(v) for k, v in x.items()}
    return x


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 22), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()


# ---------------------------------------------------------------------------
# glTF 读取（只读 memmap）
# ---------------------------------------------------------------------------
COMP_DTYPE = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
TYPE_NCOMP = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT2": 4, "MAT3": 9, "MAT4": 16}


def read_accessor(g: dict, mm: np.memmap, idx: int) -> np.ndarray:
    a = g["accessors"][idx]
    if "bufferView" not in a:
        fail(f"accessor {idx} 无 bufferView（稀疏 accessor 不支持）")
    bv = g["bufferViews"][a["bufferView"]]
    if bv.get("buffer", 0) != 0:
        fail(f"accessor {idx} 引用非 0 号 buffer（本脚本仅支持单 buffer）")
    dt = np.dtype(COMP_DTYPE[a["componentType"]])
    nc = TYPE_NCOMP[a["type"]]
    cnt = a["count"]
    off = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
    stride = bv.get("byteStride")
    elem = dt.itemsize * nc
    if stride and stride != elem:
        raw = np.ndarray(shape=(cnt, nc), dtype=dt, buffer=mm, offset=off, strides=(stride, dt.itemsize))
        return np.array(raw)
    end = off + cnt * elem
    if end > mm.shape[0]:
        fail(f"accessor {idx} 越出 buffer（end={end} > {mm.shape[0]}）")
    return np.frombuffer(mm, dtype=dt, count=cnt * nc, offset=off).reshape(cnt, nc)


def quat_xyzw_to_mat3(q) -> np.ndarray:
    x, y, z, w = (float(v) for v in q)
    n = math.sqrt(x * x + y * y + z * z + w * w)
    x, y, z, w = x / n, y / n, z / n, w / n
    return np.array([
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ], dtype=np.float64)


def node_local_matrix(n: dict) -> np.ndarray:
    if "matrix" in n:
        return np.array(n["matrix"], dtype=np.float64).reshape(4, 4, order="F")
    t = np.array(n.get("translation", [0.0, 0.0, 0.0]), dtype=np.float64)
    r = n.get("rotation", [0.0, 0.0, 0.0, 1.0])  # glTF 存 x,y,z,w
    s = np.array(n.get("scale", [1.0, 1.0, 1.0]), dtype=np.float64)
    m = np.eye(4)
    m[:3, :3] = quat_xyzw_to_mat3(r) * s[None, :]
    m[:3, 3] = t
    return m


def node_world_matrices(g: dict) -> tuple[np.ndarray, np.ndarray]:
    """返回 (N,4,4) 世界矩阵与 (N,) 父节点索引（-1 = 根 / 未到达）。"""
    nodes = g["nodes"]
    n = len(nodes)
    world = np.full((n, 4, 4), np.nan, dtype=np.float64)
    parent = np.full(n, -1, dtype=np.int64)
    roots = g["scenes"][g.get("scene", 0)]["nodes"]
    stack = [(r, np.eye(4), -1) for r in roots]
    while stack:
        ni, pm, pi = stack.pop()
        m = pm @ node_local_matrix(nodes[ni])
        world[ni] = m
        parent[ni] = pi
        for c in nodes[ni].get("children", []):
            stack.append((c, m, ni))
    return world, parent


def gather_scene(g: dict, mm: np.memmap, world: np.ndarray):
    """收集全场景世界坐标三角：V(float64 Nv×3)、T(int32 Nt×3)、tri_mat(int16)、tri_node(int32)。"""
    Vs, Ts, Ms, Ns = [], [], [], []
    vbase = 0
    meshes = g["meshes"]
    for ni, n in enumerate(g["nodes"]):
        mi = n.get("mesh")
        if mi is None:
            continue
        if np.isnan(world[ni, 0, 0]):
            fail(f"节点 {ni} 含 mesh 但未从场景根到达")
        M = world[ni]
        for p in meshes[mi]["primitives"]:
            if p.get("mode", 4) != 4:
                fail(f"节点 {ni} primitive mode={p.get('mode')} 非 TRIANGLES，不支持")
            pos = read_accessor(g, mm, p["attributes"]["POSITION"]).astype(np.float64)
            wpos = pos @ M[:3, :3].T + M[:3, 3]
            if "indices" in p:
                idx = read_accessor(g, mm, p["indices"]).reshape(-1).astype(np.int64)
            else:
                idx = np.arange(pos.shape[0], dtype=np.int64)
            if idx.shape[0] % 3:
                fail(f"节点 {ni} 索引数 {idx.shape[0]} 非 3 的倍数")
            tri = idx.reshape(-1, 3) + vbase
            Vs.append(wpos)
            Ts.append(tri)
            Ms.append(np.full(tri.shape[0], p.get("material", -1), dtype=np.int16))
            Ns.append(np.full(tri.shape[0], ni, dtype=np.int32))
            vbase += pos.shape[0]
    V = np.concatenate(Vs)
    T = np.concatenate(Ts).astype(np.int32)
    return V, T, np.concatenate(Ms), np.concatenate(Ns)


# ---------------------------------------------------------------------------
# DDS（复用 artifacts/day_0828/recon/material_census.py 的 numpy 解码；BC1/BC3 mip0 线性均值）
# ---------------------------------------------------------------------------
def srgb_to_linear(c: np.ndarray) -> np.ndarray:
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


LIN_LUT = srgb_to_linear(np.arange(256, dtype=np.float64) / 255.0)


def unpack565(c: np.ndarray) -> np.ndarray:
    r = ((c >> 11) & 31).astype(np.uint16)
    gg = ((c >> 5) & 63).astype(np.uint16)
    b = (c & 31).astype(np.uint16)
    return np.stack([(r << 3) | (r >> 2), (gg << 2) | (gg >> 4), (b << 3) | (b >> 2)], axis=-1).astype(np.uint8)


def dds_header(bytes_: bytes) -> dict:
    if len(bytes_) < 128 or bytes_[:4] != b"DDS ":
        raise ValueError("DDS magic 不符")
    h = struct.unpack_from("<I", bytes_, 12)[0]
    w = struct.unpack_from("<I", bytes_, 16)[0]
    mips = struct.unpack_from("<I", bytes_, 28)[0]
    fourcc = bytes_[84:88].decode("ascii", "replace")
    return {"w": w, "h": h, "mips": mips, "fourcc": fourcc}


def dds_mean_linear_rgb(bytes_: bytes) -> list[float]:
    hd = dds_header(bytes_)
    w, h, fourcc = hd["w"], hd["h"], hd["fourcc"]
    block_bytes = {"DXT1": 8, "DXT5": 16}.get(fourcc)
    if block_bytes is None:
        raise ValueError(f"fourCC {fourcc} 非 BC1/BC3")
    if w % 4 or h % 4:
        raise ValueError("非 4 倍数尺寸不处理")
    bw, bh = (w + 3) // 4, (h + 3) // 4
    n = bw * bh
    raw = np.frombuffer(bytes_, dtype=np.uint8, count=n * block_bytes, offset=128).reshape(n, block_bytes)
    cb = raw[:, block_bytes - 8:]
    c0 = cb[:, 0].astype(np.uint16) | (cb[:, 1].astype(np.uint16) << 8)
    c1 = cb[:, 2].astype(np.uint16) | (cb[:, 3].astype(np.uint16) << 8)
    lut = (cb[:, 4].astype(np.uint32) | (cb[:, 5].astype(np.uint32) << 8)
           | (cb[:, 6].astype(np.uint32) << 16) | (cb[:, 7].astype(np.uint32) << 24))
    p0 = unpack565(c0).astype(np.uint32)
    p1 = unpack565(c1).astype(np.uint32)
    four = c0 > c1
    pal = np.zeros((n, 4, 3), dtype=np.uint8)
    pal[:, 0] = p0
    pal[:, 1] = p1
    pal[:, 2] = np.where(four[:, None], (2 * p0 + p1) // 3, (p0 + p1) // 2).astype(np.uint8)
    pal[:, 3] = np.where(four[:, None], (p0 + 2 * p1) // 3, 0).astype(np.uint8)
    idx = np.stack([(lut >> (2 * k)) & 3 for k in range(16)], axis=1)
    counts = np.zeros((n, 4), dtype=np.uint32)
    for k in range(4):
        counts[:, k] = (idx == k).sum(axis=1)
    lin_pal = LIN_LUT[pal]
    acc = (lin_pal * counts[:, :, None]).sum(axis=(0, 1))
    npx = w * h
    return [float(acc[0] / npx), float(acc[1] / npx), float(acc[2] / npx)]


def emissive_dds_mean(g: dict, mat_idx: int, tex_dir: Path) -> dict:
    """由材质 emissiveTexture → images[].name → <name>.dds（只读原始贴图目录）求线性均值。"""
    m = g["materials"][mat_idx]
    et = m.get("emissiveTexture")
    out = {"dds_path": None, "mean_linear_rgb": None, "header": None, "error": None}
    if et is None:
        out["error"] = "材质无 emissiveTexture"
        return out
    src = g["textures"][et["index"]].get("source")
    img = g["images"][src] if src is not None else {}
    name = img.get("name")
    uri = img.get("uri", "")
    cand = None
    if uri and not uri.startswith("data:"):
        p = Path(uri)
        cand = p if p.is_absolute() else (tex_dir / p.name)
    if (cand is None or not cand.exists()) and name:
        cand = tex_dir / f"{name}.dds"
    if cand is None or not cand.exists():
        out["error"] = f"贴图文件不可得（image name={name!r}, uri 前缀={uri[:16]!r}）"
        return out
    try:
        raw = cand.read_bytes()
        out["header"] = dds_header(raw)
        out["mean_linear_rgb"] = dds_mean_linear_rgb(raw)
        out["dds_path"] = str(cand)
    except ValueError as e:
        out["error"] = f"DDS 解码失败：{e}"
        out["dds_path"] = str(cand)
    return out


# ---------------------------------------------------------------------------
# 相机数学（右手系；forward = q·(0,0,−1)，up = q·(0,1,0)；契约四元数顺序 w,x,y,z）
# ---------------------------------------------------------------------------
def mat3_to_quat_wxyz(R: np.ndarray) -> np.ndarray:
    m = R
    tr = m[0, 0] + m[1, 1] + m[2, 2]
    if tr > 0:
        s = math.sqrt(tr + 1.0) * 2
        w, x, y, z = 0.25 * s, (m[2, 1] - m[1, 2]) / s, (m[0, 2] - m[2, 0]) / s, (m[1, 0] - m[0, 1]) / s
    elif m[0, 0] > m[1, 1] and m[0, 0] > m[2, 2]:
        s = math.sqrt(1.0 + m[0, 0] - m[1, 1] - m[2, 2]) * 2
        w, x, y, z = (m[2, 1] - m[1, 2]) / s, 0.25 * s, (m[0, 1] + m[1, 0]) / s, (m[0, 2] + m[2, 0]) / s
    elif m[1, 1] > m[2, 2]:
        s = math.sqrt(1.0 + m[1, 1] - m[0, 0] - m[2, 2]) * 2
        w, x, y, z = (m[0, 2] - m[2, 0]) / s, (m[0, 1] + m[1, 0]) / s, 0.25 * s, (m[1, 2] + m[2, 1]) / s
    else:
        s = math.sqrt(1.0 + m[2, 2] - m[0, 0] - m[1, 1]) * 2
        w, x, y, z = (m[1, 0] - m[0, 1]) / s, (m[0, 2] + m[2, 0]) / s, (m[1, 2] + m[2, 1]) / s, 0.25 * s
    q = np.array([w, x, y, z], dtype=np.float64)
    if q[0] < 0:
        q = -q
    return q / np.linalg.norm(q)


def quat_wxyz_to_mat3(q) -> np.ndarray:
    w, x, y, z = (float(v) for v in q)
    return quat_xyzw_to_mat3((x, y, z, w))


def lookat_quat_wxyz(eye, target, up) -> np.ndarray:
    """look-at → 契约四元数（w,x,y,z）：旋转矩阵列 = [right, up, −forward]。"""
    eye = np.asarray(eye, dtype=np.float64)
    f = np.asarray(target, dtype=np.float64) - eye
    fn = np.linalg.norm(f)
    if fn < 1e-12:
        fail("look-at：target 与 eye 重合")
    f = f / fn
    upv = np.asarray(up, dtype=np.float64)
    r = np.cross(f, upv)
    rn = np.linalg.norm(r)
    if rn < 1e-12:
        fail("look-at：up 与 forward 共线")
    r = r / rn
    u = np.cross(r, f)
    R = np.stack([r, u, -f], axis=1)
    return mat3_to_quat_wxyz(R)


def camera_basis(q_wxyz):
    R = quat_wxyz_to_mat3(q_wxyz)
    return R[:, 0].copy(), R[:, 1].copy(), (-R[:, 2]).copy()  # right, up, forward


def project_points(eye, q_wxyz, fov_y_deg, res_wh, P):
    """世界点 → (in_frustum, px, py, depth)；像素坐标以 res_wh 为画幅，原点左上。"""
    right, up, fwd = camera_basis(q_wxyz)
    d = np.atleast_2d(P) - np.asarray(eye, dtype=np.float64)
    z = d @ fwd
    x = d @ right
    y = d @ up
    tan_y = math.tan(math.radians(fov_y_deg) / 2)
    tan_x = tan_y * res_wh[0] / res_wh[1]
    with np.errstate(divide="ignore", invalid="ignore"):
        nx = np.where(z > 1e-9, x / z / tan_x, np.nan)
        ny = np.where(z > 1e-9, y / z / tan_y, np.nan)
    inside = (z > 1e-9) & (np.abs(nx) <= 1.0) & (np.abs(ny) <= 1.0)
    px = (nx + 1.0) * 0.5 * res_wh[0]
    py = (1.0 - ny) * 0.5 * res_wh[1]
    return inside, px, py, z


# ---------------------------------------------------------------------------
# 几何查询：射线遮挡（Möller–Trumbore）、点-三角距离、粗光栅
# ---------------------------------------------------------------------------
class SceneGeom:
    def __init__(self, V, T, tri_mat, tri_node):
        self.V, self.T, self.tri_mat, self.tri_node = V, T, tri_mat, tri_node
        A, B, C = V[T[:, 0]], V[T[:, 1]], V[T[:, 2]]
        self.tri_min = np.minimum(np.minimum(A, B), C).astype(np.float32)
        self.tri_max = np.maximum(np.maximum(A, B), C).astype(np.float32)
        cr = np.cross(B - A, C - A)
        self.area = 0.5 * np.linalg.norm(cr, axis=1)
        self.centroid = (A + B + C) / 3.0

    def _box_candidates(self, lo, hi):
        lo = np.asarray(lo, dtype=np.float32)
        hi = np.asarray(hi, dtype=np.float32)
        m = ((self.tri_max >= lo) & (self.tri_min <= hi)).all(axis=1)
        return np.nonzero(m)[0]

    def segment_hit(self, o, p, exclude_node: int | None = None, eps: float = 1e-3) -> bool:
        """线段 o→p 是否被任意三角遮挡（t∈(eps,1−eps)）；可排除指定节点的三角。"""
        o = np.asarray(o, dtype=np.float64)
        p = np.asarray(p, dtype=np.float64)
        cand = self._box_candidates(np.minimum(o, p) - 1e-3, np.maximum(o, p) + 1e-3)
        if exclude_node is not None:
            cand = cand[self.tri_node[cand] != exclude_node]
        if cand.size == 0:
            return False
        T = self.T[cand]
        A, B, C = self.V[T[:, 0]], self.V[T[:, 1]], self.V[T[:, 2]]
        d = p - o
        e1, e2 = B - A, C - A
        pv = np.cross(d[None, :], e2)
        det = np.einsum("ij,ij->i", e1, pv)
        ok = np.abs(det) > 1e-14
        inv = np.zeros_like(det)
        inv[ok] = 1.0 / det[ok]
        s = o[None, :] - A
        u = np.einsum("ij,ij->i", s, pv) * inv
        qv = np.cross(s, e1)
        v = (qv @ d) * inv
        t = np.einsum("ij,ij->i", e2, qv) * inv
        hit = ok & (u >= -1e-9) & (v >= -1e-9) & (u + v <= 1 + 1e-9) & (t > eps) & (t < 1 - eps)
        return bool(hit.any())

    def segments_hit_batch(self, o, P, cand: np.ndarray, eps: float = 1e-4, chunk: int = 16384) -> np.ndarray:
        """同一原点 o 到多个终点 P(M,3) 的线段是否被 cand 三角子集遮挡（返回 bool(M,)）。

        性能口径：cand 由调用方**按盒粗筛一次**后复用（避免每条射线都对 2.83M 三角做 AABB 全扫——
        上一版 61 条/位 × 37 位 × 30 盏 ≈ 6.8 万次全扫 ≈ 半小时以上）；这里对子集按 chunk 分块做向量化
        Möller–Trumbore（qv = s × e1 与射线无关，只算一次）。"""
        o = np.asarray(o, dtype=np.float64)
        P = np.asarray(P, dtype=np.float64)
        M = P.shape[0]
        hit_any = np.zeros(M, dtype=bool)
        if cand.size == 0 or M == 0:
            return hit_any
        D = P - o[None, :]  # (M,3)
        for c0 in range(0, cand.size, chunk):
            cc = cand[c0:c0 + chunk]
            T = self.T[cc]
            A, B, C = self.V[T[:, 0]], self.V[T[:, 1]], self.V[T[:, 2]]
            e1, e2 = B - A, C - A  # (N,3)
            s = o[None, :] - A  # (N,3)
            qv = np.cross(s, e1)  # (N,3) 与射线无关
            t_num = np.einsum("ij,ij->i", e2, qv)  # (N,)
            pv = np.cross(D[:, None, :], e2[None, :, :])  # (M,N,3)
            det = np.einsum("nj,mnj->mn", e1, pv)  # (M,N)
            ok = np.abs(det) > 1e-14
            with np.errstate(divide="ignore", invalid="ignore"):
                inv = np.where(ok, 1.0 / det, 0.0)
            u = np.einsum("nj,mnj->mn", s, pv) * inv
            v = (D @ qv.T) * inv  # (M,N)
            t = t_num[None, :] * inv
            hit = ok & (u >= -1e-9) & (v >= -1e-9) & (u + v <= 1 + 1e-9) & (t > eps) & (t < 1 - eps)
            hit_any |= hit.any(axis=1)
            if hit_any.all():
                break
        return hit_any

    def min_distance(self, pnt, radius: float) -> tuple[float, int]:
        """点到 radius 邻域内三角的最小距离（Ericson 最近点，向量化）与邻域内三角数。"""
        pnt = np.asarray(pnt, dtype=np.float64)
        cand = self._box_candidates(pnt - radius, pnt + radius)
        if cand.size == 0:
            return float("inf"), 0
        T = self.T[cand]
        a, b, c = self.V[T[:, 0]], self.V[T[:, 1]], self.V[T[:, 2]]
        ab, ac, ap = b - a, c - a, pnt - a
        d1 = np.einsum("ij,ij->i", ab, ap)
        d2 = np.einsum("ij,ij->i", ac, ap)
        bp = pnt - b
        d3 = np.einsum("ij,ij->i", ab, bp)
        d4 = np.einsum("ij,ij->i", ac, bp)
        cp = pnt - c
        d5 = np.einsum("ij,ij->i", ab, cp)
        d6 = np.einsum("ij,ij->i", ac, cp)
        vc = d1 * d4 - d3 * d2
        vb = d5 * d2 - d1 * d6
        va = d3 * d6 - d5 * d4
        n = cand.size
        Q = np.empty((n, 3))
        done = np.zeros(n, dtype=bool)
        m = (d1 <= 0) & (d2 <= 0)
        Q[m] = a[m]
        done |= m
        m = ~done & (d3 >= 0) & (d4 <= d3)
        Q[m] = b[m]
        done |= m
        m = ~done & (vc <= 0) & (d1 >= 0) & (d3 <= 0)
        with np.errstate(divide="ignore", invalid="ignore"):
            vv = np.where(d1 - d3 != 0, d1 / (d1 - d3), 0.0)
        Q[m] = a[m] + vv[m, None] * ab[m]
        done |= m
        m = ~done & (d6 >= 0) & (d5 <= d6)
        Q[m] = c[m]
        done |= m
        m = ~done & (vb <= 0) & (d2 >= 0) & (d6 <= 0)
        with np.errstate(divide="ignore", invalid="ignore"):
            ww = np.where(d2 - d6 != 0, d2 / (d2 - d6), 0.0)
        Q[m] = a[m] + ww[m, None] * ac[m]
        done |= m
        m = ~done & (va <= 0) & (d4 - d3 >= 0) & (d5 - d6 >= 0)
        with np.errstate(divide="ignore", invalid="ignore"):
            den = (d4 - d3) + (d5 - d6)
            ww2 = np.where(den != 0, (d4 - d3) / den, 0.0)
        Q[m] = b[m] + ww2[m, None] * (c[m] - b[m])
        done |= m
        m = ~done
        with np.errstate(divide="ignore", invalid="ignore"):
            denom = va + vb + vc
            denom = np.where(denom != 0, denom, 1.0)
            v_ = vb / denom
            w_ = vc / denom
        Q[m] = a[m] + ab[m] * v_[m, None] + ac[m] * w_[m, None]
        dist = np.linalg.norm(Q - pnt, axis=1)
        within = int((dist <= radius).sum())
        return float(dist.min()), within

    def coarse_raster(self, eye, q_wxyz, fov_y_deg, aspect, W, H, tri_class, near=0.05,
                      oversample=3.0, cap=60000, seed=1):
        """按投影面积采样三角到 W×H 深度缓冲，返回像素类别图（-1 = 天空/空）。"""
        rng = np.random.default_rng(seed)
        right, up, fwd = camera_basis(q_wxyz)
        eye = np.asarray(eye, dtype=np.float64)
        Vc = (self.V - eye) @ np.stack([right, up, fwd], axis=1)  # (Nv,3) 相机坐标，z 前向
        Vc = Vc.astype(np.float32)
        T = self.T
        cz = Vc[:, 2]
        z0, z1, z2 = cz[T[:, 0]], cz[T[:, 1]], cz[T[:, 2]]
        zmax = np.maximum(np.maximum(z0, z1), z2)
        tan_y = math.tan(math.radians(fov_y_deg) / 2)
        tan_x = tan_y * aspect
        cx = Vc[:, 0]
        cy = Vc[:, 1]
        xmin = np.minimum(np.minimum(cx[T[:, 0]], cx[T[:, 1]]), cx[T[:, 2]])
        xmax = np.maximum(np.maximum(cx[T[:, 0]], cx[T[:, 1]]), cx[T[:, 2]])
        ymin = np.minimum(np.minimum(cy[T[:, 0]], cy[T[:, 1]]), cy[T[:, 2]])
        ymax = np.maximum(np.maximum(cy[T[:, 0]], cy[T[:, 1]]), cy[T[:, 2]])
        lim = np.maximum(zmax, 0.0)
        keep = (zmax > near) & (xmin <= lim * tan_x) & (xmax >= -lim * tan_x) & (ymin <= lim * tan_y) & (ymax >= -lim * tan_y)
        kidx = np.nonzero(keep)[0]
        if kidx.size == 0:
            return np.full((H, W), -1, dtype=np.int16), 0
        zc = np.maximum((z0[kidx] + z1[kidx] + z2[kidx]) / 3.0, 0.5)
        f_px = (H / 2.0) / tan_y
        a_px = self.area[kidx] * (f_px * f_px) / (zc * zc)
        n_s = np.clip(np.ceil(a_px * oversample), 1, cap).astype(np.int64)
        total = int(n_s.sum())
        rep = np.repeat(kidx, n_s)
        u = rng.random(total, dtype=np.float32)
        v = rng.random(total, dtype=np.float32)
        flip = u + v > 1
        u[flip] = 1 - u[flip]
        v[flip] = 1 - v[flip]
        P0 = Vc[T[rep, 0]]
        P = P0 + u[:, None] * (Vc[T[rep, 1]] - P0) + v[:, None] * (Vc[T[rep, 2]] - P0)
        z = P[:, 2]
        good = z > near
        P, z, rep = P[good], z[good], rep[good]
        nx = P[:, 0] / z / tan_x
        ny = P[:, 1] / z / tan_y
        good = (np.abs(nx) < 1) & (np.abs(ny) < 1)
        nx, ny, z, rep = nx[good], ny[good], z[good], rep[good]
        col = np.clip(((nx + 1) * 0.5 * W).astype(np.int64), 0, W - 1)
        row = np.clip(((1 - ny) * 0.5 * H).astype(np.int64), 0, H - 1)
        pix = row * W + col
        order = np.lexsort((z, pix))
        pix_s = order[np.searchsorted(pix[order], np.unique(pix))]  # 每像素最小深度样本
        img = np.full(W * H, -1, dtype=np.int16)
        img[pix[pix_s]] = tri_class[rep[pix_s]]
        return img.reshape(H, W), total


def lower_hemisphere_dirs() -> np.ndarray:
    """下半球采样方向：仰角 −80/−60/−40/−20/−5 × 12 方位 + 正下方 = 61 条。"""
    dirs = [[0.0, -1.0, 0.0]]
    for el in (-80, -60, -40, -20, -5):
        for az in range(0, 360, 30):
            e, a = math.radians(el), math.radians(az)
            dirs.append([math.cos(e) * math.cos(a), math.sin(e), math.cos(e) * math.sin(a)])
    return np.array(dirs, dtype=np.float64)


ESCAPE_DIRS = lower_hemisphere_dirs()


def escape_fraction(geom: "SceneGeom", p: np.ndarray, ray_m: float = ESCAPE_RAY_M, cand: np.ndarray | None = None) -> float:
    """点光位向下半球 61 条射线在 ray_m 内未被遮挡的比例（1.0 = 完全开放）。

    cand = 调用方按盒粗筛一次的三角子集（复用）；缺省 None 时退回逐射线全扫（仅调试用，慢）。"""
    P = p[None, :] + ESCAPE_DIRS * ray_m
    if cand is None:
        cand = geom._box_candidates(np.minimum(p, P.min(axis=0)) - 1e-3, np.maximum(p, P.max(axis=0)) + 1e-3)
    hits = geom.segments_hit_batch(p, P, cand, eps=1e-4)
    return float((~hits).sum() / len(ESCAPE_DIRS))


def choose_point_light_pos(geom: "SceneGeom", lo: np.ndarray, hi: np.ndarray, cen: np.ndarray) -> dict:
    """口径位（底面中心 −0.35 m）逃逸比 ≥ ESCAPE_ACCEPT 则采用；否则在候选集合中取逃逸比最高者。

    候选：轴线下方 drop ∈ {0.35,0.5,0.7,0.9,1.1}；玻璃质心高 / 玻璃底高的 8 方位水平偏移 r ∈ {0.5,0.7}。
    性能：每盏灯只做一次 AABB 粗筛（玻璃盒外扩 ESCAPE_RAY_M + 1.2 m，覆盖全部候选位与 4 m 射线终点），
    全部候选位 × 61 条射线只对该子集做批量 Möller–Trumbore。"""
    axis = np.array([(lo[0] + hi[0]) / 2, 0.0, (lo[2] + hi[2]) / 2])
    pad = ESCAPE_RAY_M + 1.2
    cand = geom._box_candidates(np.asarray(lo) - pad, np.asarray(hi) + pad)
    spec = axis + np.array([0.0, lo[1] - POINT_LIGHT_DROP_M, 0.0])
    spec_esc = escape_fraction(geom, spec, cand=cand)
    tested = [{"rule": f"below_axis_drop_{POINT_LIGHT_DROP_M:.2f}", "pos": spec, "escape": spec_esc}]
    if spec_esc >= ESCAPE_ACCEPT:
        return {"pos": spec, "rule": tested[0]["rule"], "escape": spec_esc, "spec_pos": spec, "spec_escape": spec_esc,
                "tested": tested}
    for drop in (0.5, 0.7, 0.9, 1.1):
        p = axis + np.array([0.0, lo[1] - drop, 0.0])
        tested.append({"rule": f"below_axis_drop_{drop:.2f}", "pos": p, "escape": escape_fraction(geom, p, cand=cand)})
    for yname, yv in (("glass_centroid_y", cen[1]), ("glass_bottom_y", lo[1] + 0.05)):
        for r in (0.5, 0.7):
            for az in range(0, 360, 45):
                a = math.radians(az)
                p = axis + np.array([r * math.cos(a), yv, r * math.sin(a)])
                tested.append({"rule": f"side_{yname}_r{r:.1f}_az{az}", "pos": p, "escape": escape_fraction(geom, p, cand=cand)})
    best = max(tested, key=lambda t: (t["escape"], -tested.index(t)))
    # 最优位若仍嵌在几何内（距最近三角 < 5 cm）则标记
    mind, _ = geom.min_distance(best["pos"], 0.3)
    return {"pos": best["pos"], "rule": best["rule"], "escape": best["escape"], "spec_pos": spec, "spec_escape": spec_esc,
            "min_tri_dist_m": mind if math.isfinite(mind) else None, "tested": tested}


# ---------------------------------------------------------------------------
# 材质分类（粗光栅占比用）
# ---------------------------------------------------------------------------
CLASS_NAMES = ["ground", "building", "vegetation", "lamp_emissive", "streetlight_fixture", "props"]
CLASS_ID = {n: i for i, n in enumerate(CLASS_NAMES)}


def classify_material(idx: int, name: str) -> int:
    if idx in EMISSIVE_INDICES or name in ("LanternEmissive", "Spotlight_Emissive", "MASTER_Light_Bulb"):
        return CLASS_ID["lamp_emissive"]
    if name.startswith("Pavement_"):
        return CLASS_ID["ground"]
    if name.startswith("Foliage_"):
        return CLASS_ID["vegetation"]
    if name.startswith("Streetlight_"):
        return CLASS_ID["streetlight_fixture"]
    if (name.startswith("MASTER_") or name.startswith("Balcony_") or name.startswith("Awnings_")
            or name.startswith("Shopsign_") or name.startswith("Bistro_Sign") or name.startswith("Concrete")
            or name in ("Plaster", "Chimneys_Metal")):
        return CLASS_ID["building"]
    return CLASS_ID["props"]


# ---------------------------------------------------------------------------
# 机位候选（世界坐标；由场景事实人工选定，见 note）
# ---------------------------------------------------------------------------
# 北街走廊（眼高 y=2.0 净空扫描实测，世界坐标）：z → 可通行 x 区间；街轴由此估算
# z=-52: x 16..26 | z=-48: 10..24 | z=-44: 8..22 | z=-40: 6..19 | z=-36: 9..17 | z=-32: 6..15 | z=-28: 0..11 | z=-24: -3..8 | z=-20: ≤4
def candidate_specs(ground_y_fn, author_cam: dict) -> list[dict]:
    """返回候选机位列表（eye/target 世界坐标；人工按走廊扫描 + 灯/店招位置选定，脚本随后做射线/光栅校核）。

    C0 = 作者相机静态位姿（参照/回退）；C1/C2/C3 人眼高 1.7 m（当地地面 Y + 1.7）。"""
    specs = []
    a_pos = np.asarray(author_cam["world_position"], dtype=np.float64)
    a_fwd = np.asarray(author_cam["world_forward"], dtype=np.float64)
    specs.append({"id": "C0", "eye": a_pos, "target": a_pos + a_fwd * 20.0, "fov_y_deg": float(author_cam["fov_y_deg"]),
                  "up": np.asarray(author_cam["world_up"], dtype=np.float64), "eye_is_author": True,
                  "desc": "作者相机静态位姿（glTF node 'Camera'，动画剥离）：北街上空 4.29 m 顺街朝 SSW 望 bistro 北立面；作参照/回退"})
    # C1：北街街心 (22, ·, −52)（走廊 16..26 的中点附近），人眼高，顺街轴（约 (−0.51,0,0.86)）望 bistro 北转角 (3, ·, −20)；
    #     书店招牌 (20.27,4.98,−40.96) 在前方 10 m 左上，SL136 / SL151 分居中线两侧（前方 21–24 m）
    eye_xz = np.array([22.0, 0.0, -52.0])
    gy = ground_y_fn(eye_xz)
    eye = np.array([22.0, gy + EYE_HEIGHT_M, -52.0])
    target = np.array([3.0, eye[1] + 1.0, -20.0])
    specs.append({"id": "C1", "eye": eye, "target": target, "fov_y_deg": CAND_FOV_Y_DEG,
                  "desc": "北街街心 (22,·,−52)、人眼高 1.7 m，顺街朝 SSW 望 bistro 北转角；书店招牌左上入画，SL136/SL151 分居中线两侧"})
    # C2：北街南口 (2, ·, −24)（走廊 −3..8），人眼高，回望 NNE 顺街轴朝药店招牌 (17.5,6.7,−54)；
    #     书店招牌右侧、SL151 左近 / SL136 右近、SL137/SL138 远端；fov 55 给灯留顶部余量
    eye2_xz = np.array([2.0, 0.0, -24.0])
    gy2 = ground_y_fn(eye2_xz)
    eye2 = np.array([2.0, gy2 + EYE_HEIGHT_M, -24.0])
    tgt2 = np.array([21.0, eye2[1] + 2.5, -52.0])
    specs.append({"id": "C2", "eye": eye2, "target": tgt2, "fov_y_deg": 55.0,
                  "desc": "北街南口 (2,·,−24)、人眼高 1.7 m，回望 NNE 顺街朝药店/书店两块店招；SL151/SL136 近、SL137/SL138 远"})
    # C3：广场 (−14, ·, −3) 望 ENE bistro 西立面：Bistro 招牌 (−1.9,5.8,−1.2)、吊灯笼 LANT23/24、头顶彩灯串；
    #     Vespa (−8,·,0..1) 落在右缘外；绿篱在 x≈−4
    eye3_xz = np.array([-14.0, 0.0, -3.0])
    gy3 = ground_y_fn(eye3_xz)
    eye3 = np.array([-14.0, gy3 + EYE_HEIGHT_M, -3.0])
    tgt3 = np.array([-1.5, gy3 + 3.5, -6.0])
    specs.append({"id": "C3", "eye": eye3, "target": tgt3, "fov_y_deg": CAND_FOV_Y_DEG,
                  "desc": "广场 (−14,·,−3) 望 ENE bistro 西立面（Bistro 招牌 + 吊灯笼 + 彩灯串）；含视点净空与树冠占比检查"})
    return specs


def _emitter_box(eye, fh, rh, tan_x, ground_y, ahead_m, depth_max_m) -> dict:
    eye_h = np.array([eye[0], 0.0, eye[2]])
    center = eye_h + fh * ahead_m
    pts = np.array([eye_h + fh * d + rh * (s * d * tan_x) for d in (EMITTER_DEPTH_MIN_M, depth_max_m) for s in (-1, 1)])
    half_x = float(np.max(np.abs(pts[:, 0] - center[0])))
    half_z = float(np.max(np.abs(pts[:, 2] - center[2])))
    cy = ground_y + EMITTER_CENTER_ABOVE_GROUND_M
    pos = np.array([center[0], cy, center[2]])
    spread = np.array([half_x, EMITTER_HALF_HEIGHT_M, half_z])
    vy = EMITTER_VEL[1]
    gmag = -EMITTER_GRAVITY

    def fall_time(h):
        # y(t) = h + vy·t + ½·g·t² = 0（vy<0, g<0）⇒ ½|g|t² + |vy|t − h = 0
        return (-abs(vy) + math.sqrt(vy * vy + 2 * gmag * h)) / gmag

    h_bottom = (cy - EMITTER_HALF_HEIGHT_M) - ground_y
    h_center = cy - ground_y
    h_top = (cy + EMITTER_HALF_HEIGHT_M) - ground_y
    T = fall_time(h_bottom)
    life = 2.0 * T
    cmd = (f"--emitter-pos {pos[0]:.3f},{pos[1]:.3f},{pos[2]:.3f} "
           f"--emitter-spread {spread[0]:.3f},{spread[1]:.3f},{spread[2]:.3f} "
           f"--emitter-vel {EMITTER_VEL[0]},{EMITTER_VEL[1]},{EMITTER_VEL[2]} "
           f"--emitter-vel-spread {EMITTER_VEL_SPREAD[0]},{EMITTER_VEL_SPREAD[1]},{EMITTER_VEL_SPREAD[2]} "
           f"--emitter-gravity {EMITTER_GRAVITY:g} --emitter-life {life:.3f}")
    return {
        "emitter_pos": pos, "emitter_spread": spread,
        "emitter_vel": list(EMITTER_VEL), "emitter_vel_spread": list(EMITTER_VEL_SPREAD),
        "emitter_gravity": EMITTER_GRAVITY,
        "ahead_m": ahead_m, "depth_range_m": [EMITTER_DEPTH_MIN_M, depth_max_m],
        "frustum_footprint_xz": pts[:, [0, 2]],
        "box_volume_m3": float(8.0 * spread[0] * spread[1] * spread[2]),
        "ground_y_used": ground_y,
        "drop_height_bottom_m": h_bottom, "drop_height_center_m": h_center, "drop_height_top_m": h_top,
        "T_fall_bottom_s": T, "T_fall_center_s": fall_time(h_center), "T_fall_top_s": fall_time(h_top),
        "emitter_life_s": life,
        "cmdline": cmd,
    }


def emitter_suggestion(eye, q_wxyz, fov_y_deg, aspect, ground_y) -> dict:
    """发射器盒（世界空间轴对齐）：盒心 = 相机水平前方 12 m、地面 + 10 m；半宽覆盖 1–25 m 视锥水平足迹。"""
    right, up, fwd = camera_basis(q_wxyz)
    fh = np.array([fwd[0], 0.0, fwd[2]])
    fh /= np.linalg.norm(fh)
    rh = np.array([right[0], 0.0, right[2]])
    rh /= np.linalg.norm(rh)
    tan_x = math.tan(math.radians(fov_y_deg) / 2) * aspect
    main = _emitter_box(eye, fh, rh, tan_x, ground_y, EMITTER_AHEAD_M, EMITTER_DEPTH_MAX_M)
    compact = _emitter_box(eye, fh, rh, tan_x, ground_y, 8.0, 15.0)
    main["derivation"] = ("盒底距地 h=(盒心 Y − 1.5) − 地面 Y；y(t)=h−9t−1.5t²=0 ⇒ T=(−9+√(81+6h))/3；"
                          "寿命 ∈ [0.5,1)·life，取 life=2T 使最短寿命 = T（盒底出生的粒子恰好落地，"
                          "更高出生者需 T_top，故 ≥ 半数粒子落地前存活为近似保证）；"
                          "盒为世界轴对齐，相机斜向取景时半宽按视锥足迹 AABB 膨胀（体积见 box_volume_m3）")
    main["compact_alternative_8m_ahead_1_to_15m"] = compact
    return main


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------
def main() -> int:
    ap = argparse.ArgumentParser(description="BistroExterior 场景事实分析（numpy 只读）")
    ap.add_argument("--gltf", type=Path, default=DEFAULT_GLTF, help="glTF 路径（缺省 .tmp 无纹理占位版，几何字节与 K: 派生版相同）")
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT, help="输出事实 JSON")
    ap.add_argument("--textures", type=Path, default=DEFAULT_TEXTURES, help="原始 DDS 贴图目录（只读）")
    ap.add_argument("--preview-dir", type=Path, default=None, help="可选：写低分辨率材质类别预览 PNG 的目录")
    ap.add_argument("--preview-res", type=int, default=384, help="预览宽度（像素）")
    ap.add_argument("--time-budget-s", type=float, default=300.0,
                    help="总耗时预算（秒）；任一阶段结束时超预算即 fail 并打印卡在哪一步（防失控重跑）")
    args = ap.parse_args()

    t0 = time.time()

    def budget_check(stage: str) -> None:
        el = time.time() - t0
        if el > args.time_budget_s:
            fail(f"时间预算 {args.time_budget_s:.0f}s 超限（{el:.1f}s）：卡在阶段「{stage}」", code=3)
        print(f"[预算] {stage} 完成 @ {el:.1f}s / {args.time_budget_s:.0f}s")

    if not args.gltf.exists():
        fail(f"glTF 不存在：{args.gltf}")
    g = json.loads(args.gltf.read_text(encoding="utf-8"))
    if len(g.get("buffers", [])) != 1:
        fail(f"仅支持单 buffer glTF，实际 {len(g.get('buffers', []))}")
    buf_uri = g["buffers"][0].get("uri")
    if not buf_uri or buf_uri.startswith("data:"):
        fail("buffer 非外部文件 uri")
    buf_path = args.gltf.parent / buf_uri
    if not buf_path.exists():
        fail(f"buffer 不存在：{buf_path}")
    mm = np.memmap(buf_path, dtype=np.uint8, mode="r")
    if mm.shape[0] != g["buffers"][0]["byteLength"]:
        fail(f"buffer 字节数 {mm.shape[0]} ≠ 声明 {g['buffers'][0]['byteLength']}")
    if not args.textures.exists():
        print(f"WARN: 贴图目录不存在，emissive DDS 均值将不可得：{args.textures}")

    mats = g["materials"]
    nodes = g["nodes"]
    world, parent = node_world_matrices(g)
    root_scale = None
    for ni, n in enumerate(nodes):
        if n.get("name") == "BistroExterior":
            root_scale = n.get("scale")
    print(f"[载入] 节点 {len(nodes)} / 网格 {len(g['meshes'])} / 材质 {len(mats)} / buffer {mm.shape[0]} B；BistroExterior scale={root_scale}")

    V, T, tri_mat, tri_node = gather_scene(g, mm, world)
    geom = SceneGeom(V, T, tri_mat, tri_node)
    print(f"[几何] 顶点 {V.shape[0]} / 三角 {T.shape[0]}，耗时 {time.time() - t0:.1f}s")

    scene_min = V.min(axis=0)
    scene_max = V.max(axis=0)

    # 节点世界矩阵摘要（全量 1300×16 数值太大，仅登记关键节点，其余给统计）
    key_nodes = {}
    for ni, n in enumerate(nodes):
        nm = n.get("name", "")
        if ni in (0, 1, 2, 3) or "camera" in n or nm.startswith("Lantern_Wind") or "ShopSign_01D" in nm or "ShopSign_01E" in nm:
            key_nodes[str(ni)] = {"name": nm, "parent": int(parent[ni]), "world_matrix_rows": world[ni].tolist()}
    reached = int((~np.isnan(world[:, 0, 0])).sum())

    # 地面 Y 估计
    tri_names = [m.get("name", "") for m in mats]
    ground_mat_ids = [i for i, nm in enumerate(tri_names) if any(k in nm for k in GROUND_KEYWORDS)]
    gmask = np.isin(tri_mat, ground_mat_ids)
    gverts = np.unique(T[gmask].reshape(-1))
    gy_all = V[gverts, 1]
    ground_stats = {
        "materials_used": [{"index": i, "name": tri_names[i]} for i in ground_mat_ids],
        "vertex_count": int(gverts.size),
        "y_median": float(np.median(gy_all)),
        "y_p10": float(np.percentile(gy_all, 10)), "y_p90": float(np.percentile(gy_all, 90)),
        "y_min": float(gy_all.min()), "y_max": float(gy_all.max()),
        "basis": "cobblestone / wet pavement / curbstone / brick / manhole 类材质顶点 Y 中位数（实测）",
    }
    gV = V[gverts]

    def ground_y_local(xz, radius=4.0):
        d2 = (gV[:, 0] - xz[0]) ** 2 + (gV[:, 2] - xz[2]) ** 2
        sel = gV[d2 <= radius * radius, 1]
        if sel.size < 20:
            return float(ground_stats["y_median"])
        return float(np.median(sel))

    # emissive 材质统计
    tex_dir = args.textures
    emissive_rows = []
    for mi in EMISSIVE_INDICES:
        m = tri_mat == mi
        if not m.any():
            fail(f"材质 {mi} 无三角")
        ar = geom.area[m]
        ce = geom.centroid[m]
        Ttri = T[m]
        pts = V[np.unique(Ttri.reshape(-1))]
        dds = emissive_dds_mean(g, mi, tex_dir)
        emissive_rows.append({
            "material_index": mi, "name": tri_names[mi],
            "tri_count": int(m.sum()), "node_count": int(np.unique(tri_node[m]).size),
            "area_m2": float(ar.sum()),
            "aabb_min": pts.min(axis=0), "aabb_max": pts.max(axis=0),
            "centroid_area_weighted": (ce * ar[:, None]).sum(axis=0) / ar.sum(),
            "gltf_emissive_factor": mats[mi].get("emissiveFactor"),
            "emissive_dds": dds,
        })
        mr = dds["mean_linear_rgb"]
        print(f"[emissive] {mi:3d} {tri_names[mi]:48s} tris={int(m.sum()):6d} area={ar.sum():9.3f} m² "
              f"dds_mean={'/'.join(f'{c:.4f}' for c in mr) if mr else dds['error']}")

    # 材质 12 按节点拆分
    m12 = tri_mat == STREETLIGHT_MAT
    # 路灯金属灯具节点（*_StreetLight_01B_* / *_StreetLight_01a_*，不含 Glass）→ 与玻璃按水平距离配对
    fixture_nodes = []
    for ni, n in enumerate(nodes):
        nm = n.get("name", "")
        if "streetlight_01" in nm.lower() and "glass" not in nm.lower() and n.get("mesh") is not None:
            sel = tri_node == ni
            pts = V[np.unique(T[sel].reshape(-1))]
            fixture_nodes.append((ni, nm, pts.min(axis=0), pts.max(axis=0)))
    lamp_rows, lantern_rows, eave_rows = [], [], []
    for ni in np.unique(tri_node[m12]):
        sel = m12 & (tri_node == ni)
        Ttri = T[sel]
        pts = V[np.unique(Ttri.reshape(-1))]
        ar = geom.area[sel]
        ce = geom.centroid[sel]
        cen = (ce * ar[:, None]).sum(axis=0) / ar.sum()
        lo, hi = pts.min(axis=0), pts.max(axis=0)
        nm = nodes[ni].get("name", "")
        row = {"node": int(ni), "node_name": nm, "tri_count": int(sel.sum()), "area_m2": float(ar.sum()),
               "aabb_min": lo, "aabb_max": hi, "aabb_size": hi - lo, "glass_centroid": cen}
        low = nm.lower()
        if "streetlight_glass" in low:
            row["id"] = f"SL{int(ni)}"
            row["fbx_tail"] = nm.rsplit("_", 1)[-1]
            row["lamp_type"] = "01B_wall_bracket" if "_01b" in low else ("01a_pole" if "_01a" in low else "unknown")
            bottom_center = np.array([(lo[0] + hi[0]) / 2, lo[1], (lo[2] + hi[2]) / 2])
            row["glass_bottom_center"] = bottom_center
            row["centroid_minus_bottom_m"] = float(cen[1] - lo[1])
            row["ground_y_local"] = ground_y_local(cen)
            # 配对灯具节点
            best = None
            for fni, fnm, flo, fhi in fixture_nodes:
                fc = np.array([(flo[0] + fhi[0]) / 2, (flo[2] + fhi[2]) / 2])
                d = float(np.hypot(fc[0] - cen[0], fc[1] - cen[2]))
                if best is None or d < best[0]:
                    best = (d, fni, fnm, flo, fhi)
            if best is not None and best[0] < 1.0:
                row["fixture_node"] = best[1]
                row["fixture_node_name"] = best[2]
                row["fixture_aabb_min"] = best[3]
                row["fixture_aabb_max"] = best[4]
                row["fixture_extends_below_glass_m"] = float(lo[1] - best[3][1])
            pl = choose_point_light_pos(geom, lo, hi, cen)
            row["point_light_pos_spec"] = pl["spec_pos"]
            row["point_light_pos_spec_escape"] = pl["spec_escape"]
            row["point_light_pos"] = pl["pos"]
            row["point_light_rule"] = pl["rule"]
            row["point_light_escape"] = pl["escape"]
            if "min_tri_dist_m" in pl:
                row["point_light_min_tri_dist_m"] = pl["min_tri_dist_m"]
            row["point_light_candidates_tested"] = [{"rule": t["rule"], "pos": t["pos"], "escape": t["escape"]} for t in pl["tested"]]
            row["height_above_ground_m"] = float(row["point_light_pos"][1] - row["ground_y_local"])
            lamp_rows.append(row)
        elif low.startswith("lantern_wind"):
            row["id"] = f"LANT{nm.rsplit('_', 1)[-1]}"
            pl = choose_point_light_pos(geom, lo, hi, cen)
            row["point_light_pos_spec"] = pl["spec_pos"]
            row["point_light_pos_spec_escape"] = pl["spec_escape"]
            row["point_light_pos"] = pl["pos"]
            row["point_light_rule"] = pl["rule"]
            row["point_light_escape"] = pl["escape"]
            row["ground_y_local"] = ground_y_local(cen)
            row["height_above_ground_m"] = float(row["point_light_pos"][1] - row["ground_y_local"])
            lantern_rows.append(row)
        else:
            row["id"] = f"EAVE{int(ni)}"
            eave_rows.append(row)
    lamp_rows.sort(key=lambda r: r["node"])
    budget_check("材质12 拆分 + 点光位逃逸测试")
    for r in lamp_rows:
        print(f"[路灯] {r['id']} {r['lamp_type']:16s} glass_cen={np.round(r['glass_centroid'], 3).tolist()} "
              f"spec_pl_escape={r['point_light_pos_spec_escape']:.2f} -> {r['point_light_rule']} pl={np.round(r['point_light_pos'], 3).tolist()} escape={r['point_light_escape']:.2f}")
    # 空间聚类复核（质心距 > 1.5 m 分簇）
    cents = np.array([r["glass_centroid"] for r in lamp_rows + lantern_rows + eave_rows])
    labels = -np.ones(len(cents), dtype=int)
    nclu = 0
    for i in range(len(cents)):
        if labels[i] >= 0:
            continue
        labels[i] = nclu
        stack = [i]
        while stack:
            k = stack.pop()
            d = np.linalg.norm(cents - cents[k], axis=1)
            for j in np.nonzero((d <= 1.5) & (labels < 0))[0]:
                labels[j] = nclu
                stack.append(int(j))
        nclu += 1
    print(f"[材质12] 节点 {len(lamp_rows) + len(lantern_rows) + len(eave_rows)} = 路灯玻璃 {len(lamp_rows)} + 吊灯笼 {len(lantern_rows)} + 檐下面 {len(eave_rows)}；1.5 m 聚类簇数 {nclu}")
    lamps_by_id = {r["id"]: r for r in lamp_rows}
    for want in ("SL136", "SL151"):
        if want not in lamps_by_id:
            fail(f"未找到路灯 {want}（节点名 id 规则 SL<node>）")

    # 店招 38/39：AABB、PCA 最薄轴、中心、朝街侧
    signs = {}
    for mi in SHOPSIGN_MATS:
        sel = tri_mat == mi
        Ttri = T[sel]
        pts = V[np.unique(Ttri.reshape(-1))]
        c = pts.mean(axis=0)
        cov = np.cov((pts - c).T)
        evals, evecs = np.linalg.eigh(cov)
        nrm = evecs[:, 0]
        # 朝街侧：法线正负两侧 1.5 m 处，1.2 m 邻域内三角更少者为开放侧
        cnt_pos = geom.min_distance(c + nrm * 1.5, 1.2)[1]
        cnt_neg = geom.min_distance(c - nrm * 1.5, 1.2)[1]
        facing = nrm if cnt_pos <= cnt_neg else -nrm
        A, B, C = V[Ttri[:, 0]], V[Ttri[:, 1]], V[Ttri[:, 2]]
        fn = np.cross(B - A, C - A)
        fn_sum = fn.sum(axis=0)
        signs[str(mi)] = {
            "material_index": mi, "name": tri_names[mi], "tri_count": int(sel.sum()),
            "node": int(np.unique(tri_node[sel])[0]), "node_name": nodes[int(np.unique(tri_node[sel])[0])].get("name"),
            "area_m2": float(geom.area[sel].sum()),
            "aabb_min": pts.min(axis=0), "aabb_max": pts.max(axis=0), "center": c,
            "pca_eigenvalues": evals, "pca_thin_axis": nrm, "facing_normal_open_side": facing,
            "open_side_tri_counts": {"plus_axis": cnt_pos, "minus_axis": cnt_neg},
            "winding_normal_sum": fn_sum,
        }
        print(f"[店招] {mi} {tri_names[mi]} center={np.round(c, 3).tolist()} thin_axis={np.round(nrm, 3).tolist()} open_side={np.round(facing, 3).tolist()}")

    # 作者相机
    cam_node = None
    for ni, n in enumerate(nodes):
        if "camera" in n:
            cam_node = ni
            break
    if cam_node is None:
        fail("未找到 camera 节点")
    Mc = world[cam_node]
    R = Mc[:3, :3].copy()
    scales = np.linalg.norm(R, axis=0)
    R = R / scales[None, :]
    q_author = mat3_to_quat_wxyz(R)
    a_right, a_up, a_fwd = camera_basis(q_author)
    cam_def = g["cameras"][nodes[cam_node]["camera"]]["perspective"]
    yfov = float(cam_def["yfov"])
    author_cam = {
        "node": cam_node, "node_name": nodes[cam_node].get("name"), "animated": True,
        "local_translation": nodes[cam_node].get("translation"), "local_rotation_xyzw": nodes[cam_node].get("rotation"),
        "world_position": Mc[:3, 3], "world_forward": a_fwd, "world_up": a_up, "world_right": a_right,
        "pitch_deg": math.degrees(math.asin(float(np.clip(a_fwd[1], -1, 1)))),
        "yaw_deg_from_minus_z": math.degrees(math.atan2(float(a_fwd[0]), float(-a_fwd[2]))),
        "yfov_rad": yfov, "fov_y_deg": math.degrees(yfov), "aspect_ratio": cam_def.get("aspectRatio"),
        "znear": cam_def.get("znear"), "zfar": cam_def.get("zfar"),
        "orientation_quat_wxyz": q_author.tolist(),
        "parent_scale_removed": scales.tolist(),
        "note": "静态节点位姿（动画 Take 剥离登记，沿用 G10.5 口径）；世界位置 = 1.6 × 局部",
    }
    print(f"[作者相机] pos={np.round(Mc[:3, 3], 3).tolist()} fwd={np.round(a_fwd, 3).tolist()} fov_y={math.degrees(yfov):.3f}°")

    # 机位候选
    tri_class = np.array([classify_material(i, nm) for i, nm in enumerate(tri_names)], dtype=np.int16)
    tri_class_per_tri = tri_class[tri_mat]
    aspect = CAND_RES[0] / CAND_RES[1]
    lamp_targets = lamp_rows + lantern_rows
    sign_targets = [signs[str(mi)] for mi in SHOPSIGN_MATS]
    candidates = []
    if args.preview_dir is not None:
        args.preview_dir.mkdir(parents=True, exist_ok=True)
    # 北街走廊扫描（眼高 y = 地面中位 + 1.7 ≈ 2.07；每 4 m 一个 z 切片，x 步 1 m，0.45 m 邻域有三角即占用）
    corridor = []
    y_scan = ground_stats["y_median"] + EYE_HEIGHT_M
    for z in range(-56, -19, 4):
        occ = []
        for x in range(-8, 33):
            _, n = geom.min_distance(np.array([float(x), y_scan, float(z)]), 0.45)
            occ.append(n > 0)
        # 取包含预期街心的最长空闲区间
        free_runs = []
        start = None
        for i, o in enumerate(occ + [True]):
            if not o and start is None:
                start = i
            if o and start is not None:
                free_runs.append((start - 8, i - 1 - 8))
                start = None
        longest = max(free_runs, key=lambda r: r[1] - r[0]) if free_runs else None
        corridor.append({"z": z, "free_x_runs": free_runs, "widest_run": longest,
                         "center_x": (longest[0] + longest[1]) / 2 if longest else None})
    print("[北街走廊] " + "; ".join(f"z={c['z']}: x {c['widest_run']}" for c in corridor))
    budget_check("店招 / 作者相机 / 北街走廊扫描")

    for spec in candidate_specs(ground_y_local, author_cam):
        eye = np.asarray(spec["eye"], dtype=np.float64)
        target = np.asarray(spec["target"], dtype=np.float64)
        upv = np.asarray(spec.get("up", [0.0, 1.0, 0.0]), dtype=np.float64)
        if spec.get("eye_is_author"):
            q = np.asarray(author_cam["orientation_quat_wxyz"], dtype=np.float64)
        else:
            q = lookat_quat_wxyz(eye, target, upv)
        right, up, fwd = camera_basis(q)
        fov = spec["fov_y_deg"]
        # 灯可见性
        vis = []
        for r in lamp_targets:
            cen = np.asarray(r["glass_centroid"])
            inside, px, py, z = project_points(eye, q, fov, CAND_RES, cen[None, :])
            probes = [cen, cen + np.array([0.2, 0, 0]), cen - np.array([0.2, 0, 0]), cen + np.array([0, 0, 0.2]), cen - np.array([0, 0, 0.2])]
            unocc = sum(0 if geom.segment_hit(eye, p, exclude_node=r["node"]) else 1 for p in probes)
            plp = np.asarray(r["point_light_pos"])
            pl_unocc = not geom.segment_hit(eye, plp)
            vis.append({
                "id": r["id"], "kind": "streetlight" if r["id"].startswith("SL") else "lantern",
                "in_frustum": bool(inside[0]), "distance_m": float(np.linalg.norm(cen - eye)),
                "screen_px": [float(px[0]), float(py[0])] if inside[0] else None,
                "screen_uv": [float(px[0] / CAND_RES[0]), float(py[0] / CAND_RES[1])] if inside[0] else None,
                "glass_probe_unoccluded": f"{unocc}/5", "glass_visible": bool(inside[0]) and unocc >= 1,
                "point_light_pos_unoccluded_from_eye": bool(pl_unocc),
            })
        visible = [v for v in vis if v["glass_visible"]]
        visible.sort(key=lambda v: v["distance_m"])
        # 店招
        sgn = []
        for s in sign_targets:
            c = np.asarray(s["center"])
            inside, px, py, z = project_points(eye, q, fov, CAND_RES, c[None, :])
            occl = geom.segment_hit(eye, c, exclude_node=s["node"])
            cos_face = float(np.dot(np.asarray(s["facing_normal_open_side"]), (eye - c) / np.linalg.norm(eye - c)))
            sgn.append({"material_index": s["material_index"], "name": s["name"], "in_frustum": bool(inside[0]),
                        "unoccluded": not occl, "distance_m": float(np.linalg.norm(c - eye)),
                        "screen_px": [float(px[0]), float(py[0])] if inside[0] else None,
                        "cos_facing_to_eye": cos_face, "faces_camera": cos_face > 0.15})
        # 视点净空（0.6 m 硬门 + 1.5 m 开阔度参考）
        mind, within = geom.min_distance(eye, CLEARANCE_RADIUS_M)
        mind15, within15 = geom.min_distance(eye, 1.5)
        clearance = {"radius_m": CLEARANCE_RADIUS_M, "min_tri_distance_m": mind if math.isfinite(mind) else None,
                     "tris_within_radius": within, "pass": within == 0,
                     "open_space_1p5m": {"tris_within": within15, "min_tri_distance_m": mind15 if math.isfinite(mind15) else None}}
        # 粗光栅
        img, nsamp = geom.coarse_raster(eye, q, fov, aspect, RASTER_W, RASTER_H, tri_class_per_tri, near=CAND_NEAR)
        tot = img.size
        frac = {name: float((img == cid).sum() / tot) for name, cid in CLASS_ID.items()}
        frac["sky_or_empty"] = float((img < 0).sum() / tot)
        h3, w3 = RASTER_H // 3, RASTER_W // 3
        center_img = img[h3:2 * h3, w3:2 * w3]
        veg_center = float((center_img == CLASS_ID["vegetation"]).sum() / center_img.size)
        gy_local = ground_y_local(eye)
        em = emitter_suggestion(eye, q, fov, aspect, gy_local)
        cand = {
            "id": spec["id"], "desc": spec["desc"],
            "eye": eye, "target": target, "up": upv, "fov_y_deg": fov, "near": CAND_NEAR, "far": CAND_FAR,
            "resolution": {"w": CAND_RES[0], "h": CAND_RES[1]},
            "quat_source": "author_node_rotation" if spec.get("eye_is_author") else "lookat(eye,target,up)",
            "orientation_quat_wxyz": q.tolist(), "forward": fwd, "right": right, "camera_up": up,
            "pitch_deg": math.degrees(math.asin(float(np.clip(fwd[1], -1, 1)))),
            "yaw_deg_from_minus_z": math.degrees(math.atan2(float(fwd[0]), float(-fwd[2]))),
            "ground_y_local": gy_local, "eye_height_above_ground_m": float(eye[1] - gy_local),
            "clearance": clearance,
            "raster_96x54_class_fraction": frac,
            "vegetation_center_third_fraction": veg_center,
            "raster_samples": nsamp,
            "lamps_all": vis,
            "lamps_visible_sorted": visible,
            "visible_streetlight_ids": [v["id"] for v in visible if v["kind"] == "streetlight"],
            "visible_lantern_ids": [v["id"] for v in visible if v["kind"] == "lantern"],
            "shop_signs": sgn,
            "emitter": em,
        }
        candidates.append(cand)
        budget_check(f"机位 {spec['id']} 校核")
        print(f"[机位 {spec['id']}] eye={np.round(eye, 3).tolist()} target={np.round(target, 3).tolist()} "
              f"visible_SL={cand['visible_streetlight_ids']} lanterns={cand['visible_lantern_ids']} "
              f"clearance={'PASS' if clearance['pass'] else 'FAIL'}(min {mind:.2f} m) veg={frac['vegetation']:.2f} sky={frac['sky_or_empty']:.2f}")
        if args.preview_dir is not None:
            try:
                from PIL import Image
                pw = args.preview_res
                ph = int(round(pw * CAND_RES[1] / CAND_RES[0]))
                pimg, _ = geom.coarse_raster(eye, q, fov, aspect, pw, ph, tri_class_per_tri, near=CAND_NEAR, oversample=4.0, cap=400000)
                palette = np.array([[70, 70, 80], [160, 130, 100], [40, 120, 50], [255, 230, 120], [120, 120, 140], [190, 90, 160], [10, 10, 30]], dtype=np.uint8)
                rgb = palette[np.where(pimg < 0, 6, pimg)]
                # 标注可见路灯屏幕位置（白色十字）
                for v in visible:
                    ux, uy = v["screen_uv"]
                    cx, cy = int(ux * pw), int(uy * ph)
                    rgb[max(cy - 3, 0):cy + 4, max(cx, 0):cx + 1] = 255
                    rgb[max(cy, 0):cy + 1, max(cx - 3, 0):cx + 4] = 255
                Image.fromarray(rgb).save(args.preview_dir / f"preview_{spec['id']}.png")
            except ImportError:
                print("WARN: PIL 不可用，跳过预览")

    facts = {
        "schema": "rurix.day0902.exterior_scene_facts.v1",
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "note": ("BistroExterior（ORCA CC-BY-4.0）场景事实；全部坐标为**世界坐标**：根节点 BistroExterior（node 1）"
                 "带 1.6 均匀缩放，加载器按 parent×local 完整 TRS 累乘 ⇒ 世界 = 1.6 × 局部。"
                 "『实测』= 由 glTF 几何 numpy 直接计算（面积/AABB/质心/射线遮挡/光栅占比）；"
                 "『估算』= 地面 Y（顶点 Y 中位数）、机位候选（人工选定 + 数据校核）、发射器盒/落地时间（解析公式）、"
                 "材质占比（96×54 采样光栅，非精确渲染）。数值保留 3 位小数；orientation_quat_wxyz 例外保留全精度以维持单位范数"
                 "（契约脚本按 eye/target/up 重算并归一化）。"),
        "inputs": {
            "gltf": str(args.gltf), "gltf_sha256": sha256_file(args.gltf),
            "buffer": str(buf_path), "buffer_bytes": int(mm.shape[0]), "buffer_sha256": sha256_file(buf_path),
            "textures_dir": str(tex_dir), "generator": g.get("asset", {}).get("generator"),
        },
        "counts": {"nodes": len(nodes), "nodes_reached": reached, "meshes": len(g["meshes"]), "materials": len(mats),
                   "vertices": int(V.shape[0]), "triangles": int(T.shape[0]),
                   "alpha_modes": sorted(set(m.get("alphaMode", "OPAQUE") for m in mats))},
        "root_scale": root_scale,
        "scene_aabb_world": {"min": scene_min, "max": scene_max, "size": scene_max - scene_min},
        "ground": ground_stats,
        "key_node_world_matrices": key_nodes,
        "emissive_materials": emissive_rows,
        "streetlights": {
            "count": len(lamp_rows), "id_rule": "SL<glTF 节点下标>（节点名 *_StreetLight_Glass_*；fbx_tail = 节点名尾号）",
            "point_light_rule_spec": f"任务口径：玻璃 AABB 底面中心再向下 {POINT_LIGHT_DROP_M} m（point_light_pos_spec）",
            "point_light_rule_used": (f"逃逸测试：口径位向下半球 61 条 {ESCAPE_RAY_M} m 射线未遮比例 ≥ {ESCAPE_ACCEPT} 则采用，"
                                      "否则在轴下 0.5–1.1 m / 玻璃质心高·底高 8 方位 0.5–0.7 m 侧偏候选中取逃逸比最高者（point_light_pos）。"
                                      "实测：01B 壁挂灯的下部饰件延伸到玻璃底下方 ≈0.5 m，01a 杆灯的玻璃坐在杆头上——口径位对两型均被灯具包住"),
            "lamps": lamp_rows,
        },
        "lanterns": {"count": len(lantern_rows), "note": "Lantern_Wind_* 节点中属材质 12 的部分（灯笼内发光体；材质 14 LanternEmissive 为灯笼罩）", "items": lantern_rows},
        "eave_faces": {"count": len(eave_rows), "note": "paris_building_01_bottom_* 节点中属材质 12 的 2 三角小面（檐下 0.33×0.34 m 水平发光面）", "items": eave_rows},
        "material12_cluster_check": {"threshold_m": 1.5, "clusters": nclu, "nodes": len(cents)},
        "shop_signs": signs,
        "north_street_corridor": {"note": f"眼高 y={y_scan:.3f} 净空扫描（估算）：各 z 切片可通行 x 区间（世界坐标，1 m 步）", "slices": corridor},
        "author_camera": author_cam,
        "camera_candidates": candidates,
        "material_classes": {"names": CLASS_NAMES, "per_material": [{"index": i, "name": nm, "class": CLASS_NAMES[tri_class[i]]} for i, nm in enumerate(tri_names)]},
        "elapsed_s": time.time() - t0,
    }
    out = r3(facts)
    # 四元数保留全精度（单位范数；其余数值 3 位小数）
    by_id = {c["id"]: c for c in candidates}
    for c in out["camera_candidates"]:
        c["orientation_quat_wxyz"] = [float(v) for v in by_id[c["id"]]["orientation_quat_wxyz"]]
    out["author_camera"]["orientation_quat_wxyz"] = [float(v) for v in author_cam["orientation_quat_wxyz"]]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(out, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"[完成] 写出 {args.out}，总耗时 {time.time() - t0:.1f}s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
