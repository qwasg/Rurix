# -*- coding: utf-8 -*-
"""Phase F 灯具 emissive 贴图烘焙侧车（确定性可重跑）。

4 张 2048² RGBA PNG（K:/rurix-ext/g11-assets/bistro-interior-ue/*_Emissive.png）
→ 逐张 `.rgba8bin`（头 3×u32 LE [w,h,mips] + 逐级 RGBA8 行主序紧凑）+
manifest.json。

mip 链语义（与 BaseColor DDS mip 语义一致——kernel 采样后过 linlut）：
- mip0 = 源 PNG RGBA8 原字节（零重采样）;
- 级 ℓ+1 = 级 ℓ 的 **sRGB→linear 域** 2×2 box 平均（float64 运行链,不逐级
  重量化——均值保持精确）→ 存储时 linear→sRGB 编码 + (x·255+0.5).floor()
  量化（仓内 g31_display_encode/g31_tex_host_sample_srgb 同字面量化镜像）;
- A 通道非 sRGB：线性域直接 box 平均后同式量化（kernel 采样面不消费 A,
  registered for completeness）。

sRGB 传递函数 = 仓内 srgb_to_linear 逐字同式（g14_3_lane_body.rs L1309:
c ≤ 0.04045 ? c/12.92 : ((c+0.055)/1.055)^2.4）。

manifest 逐张登记：source_sha256（PNG 字节）/ output_sha256（.rgba8bin
字节）/ mip0_rgba8_sha256（mip0 RGBA8 平面,host 侧互核锚）/ 尺寸 / 级数 /
mip0 逐通道 **线性均值**（sRGB→linear 后 float64 均值,A 忽略——scale
标定分母:scale_c = 契约 Le_c / linear_mean_c）。
"""
from __future__ import annotations

import hashlib
import json
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image

SRC_DIR = Path("K:/rurix-ext/g11-assets/bistro-interior-ue")
OUT_DIR = Path(__file__).resolve().parent / "baked"

# F 相微调（小灯全白）：emissive 分布对比度 γ 重映射（线性域 dist^γ）。
# 动机：显示链总增益 ~×52.8（EV ×16 · AE ~×3.3）把 em_tex_linear >0.019 全推
# 白——Lantern 玻璃罩区 0.05-0.16 整体裁剪。γ>1 拉开灯泡:罩动态范围（罩压入
# 色阶带、灯泡仍白）；均值由 manifest 重标定（scale_c = Le_c / mean(tex^γ)）
# ⇒ 可见面均值仍 == 契约 Le，投光面（A1 提取/GI2 均值）架构解耦零影响。
# γ=1.0 位级恒等旧烘焙（mip0 源字节零重采样路径保留）。
GAMMA = 1.0

# (material_index, material_name) —— 契约 milestones/g13/
# g13_ue_upscale_parity_contract.json scenes[]/lighting/emissive_materials 段。
TEXTURES = [
    (38, "MASTER_Interior_01_Paris_Lantern"),
    (39, "Paris_Ceiling_Lamp"),
    (40, "Paris_CeilingFan"),
    (59, "Paris_Wall_Light_Interior"),
]


def srgb_to_linear(c: np.ndarray) -> np.ndarray:
    """仓内 srgb_to_linear 同式（float64 域）。"""
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c: np.ndarray) -> np.ndarray:
    """标准逆变换（float64 域）。"""
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * (c ** (1.0 / 2.4)) - 0.055)


def quant8(x: np.ndarray) -> np.ndarray:
    """(x·255+0.5).floor() 量化（仓内 8-bit 量化同字面）。"""
    return np.clip(np.floor(x * 255.0 + 0.5), 0.0, 255.0).astype(np.uint8)


def sha256(b: bytes) -> str:
    return "sha256:" + hashlib.sha256(b).hexdigest()


def bake_one(mat_idx: int, name: str) -> dict:
    src = SRC_DIR / f"{name}_Emissive.png"
    raw = src.read_bytes()
    img = Image.open(src)
    if img.mode != "RGBA":
        raise SystemExit(f"FAIL: {src} mode={img.mode} ≠ RGBA（fail-closed）")
    w, h = img.size
    if w != h or (w & (w - 1)) != 0:
        raise SystemExit(f"FAIL: {src} 尺寸 {w}x{h} 非 pow2 方图（fail-closed）")
    a8 = np.asarray(img, dtype=np.uint8)  # (H, W, 4)
    if a8.shape != (h, w, 4):
        raise SystemExit(f"FAIL: {src} 数组形状 {a8.shape} 异常")

    # 运行链（float64 线性域;A 线性直通）。
    f = a8.astype(np.float64) / 255.0
    lin = np.empty_like(f)
    lin[..., :3] = srgb_to_linear(f[..., :3])
    lin[..., 3] = f[..., 3]

    # γ 对比度重映射（线性域;γ=1.0 恒等 ⇒ 走 mip0 源字节零重采样路径）。
    if GAMMA != 1.0:
        lin[..., :3] = lin[..., :3] ** GAMMA

    mips = int(np.log2(w)) + 1  # 2048 → 12 级
    if GAMMA == 1.0:
        mip0_bytes = a8.tobytes()  # mip0 = 源字节零重采样
    else:
        m0 = np.empty(lin.shape, dtype=np.uint8)
        m0[..., :3] = quant8(linear_to_srgb(lin[..., :3]))
        m0[..., 3] = quant8(lin[..., 3])
        mip0_bytes = m0.tobytes()
    levels: list[bytes] = [mip0_bytes]
    level_dims = [(w, h)]
    cur = lin
    for _ in range(1, mips):
        # 2×2 box 平均（线性域;逐级折半,pow2 精确）。
        cur = (
            cur[0::2, 0::2] + cur[0::2, 1::2] + cur[1::2, 0::2] + cur[1::2, 1::2]
        ) * 0.25
        stored = np.empty(cur.shape, dtype=np.uint8)
        stored[..., :3] = quant8(linear_to_srgb(cur[..., :3]))
        stored[..., 3] = quant8(cur[..., 3])
        levels.append(stored.tobytes())
        level_dims.append((cur.shape[1], cur.shape[0]))

    header = struct.pack("<III", w, h, mips)
    blob = header + b"".join(levels)
    out = OUT_DIR / f"{name}.rgba8bin"
    out.write_bytes(blob)

    # mip0 线性均值（scale 标定分母;A 忽略）。
    mean_rgb = [float(lin[..., c].mean()) for c in range(3)]
    return {
        "material_index": mat_idx,
        "material_name": name,
        "source": str(src).replace("\\", "/"),
        "source_sha256": sha256(raw),
        "file": out.name,
        "output_sha256": sha256(blob),
        "mip0_rgba8_sha256": sha256(levels[0]),
        "width": w,
        "height": h,
        "mips": mips,
        "level_dims": [[dw, dh] for (dw, dh) in level_dims],
        "linear_mean_rgb": mean_rgb,
        "emissive_gamma": GAMMA,
    }


def main() -> int:
    global GAMMA
    if len(sys.argv) >= 3 and sys.argv[1] == "--gamma":
        GAMMA = float(sys.argv[2])
    print(f"emissive_gamma = {GAMMA}")
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    entries = [bake_one(mi, name) for mi, name in TEXTURES]
    manifest = {
        "schema": "rurix.day0828.f_emissive.bake_manifest.v1",
        "srgb_law": "srgb_to_linear: c<=0.04045 ? c/12.92 : ((c+0.055)/1.055)^2.4（仓内 g14_3_lane_body.rs 同式）; 量化 (x*255+0.5).floor()",
        "mip_law": "mip0=源字节零重采样; 级 l+1 = 级 l 线性域 2x2 box 平均（float64 运行链）→ linear→sRGB8 存储; A 线性平均",
        "container": ".rgba8bin = u32 LE [w,h,mips] ×3 头 + 逐级 RGBA8 行主序紧凑",
        "entries": entries,
    }
    (OUT_DIR / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=1), encoding="utf-8"
    )
    for e in entries:
        print(
            f"{e['material_index']:>3} {e['material_name']:<36} {e['width']}x{e['height']} mips={e['mips']} "
            f"mean_rgb=({e['linear_mean_rgb'][0]:.6f},{e['linear_mean_rgb'][1]:.6f},{e['linear_mean_rgb'][2]:.6f}) "
            f"out={e['output_sha256'][:15]}…"
        )
    print(f"OK → {OUT_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
