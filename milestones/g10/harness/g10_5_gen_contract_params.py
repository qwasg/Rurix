#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.5a 波）
"""G10.5 harness — 双场景契约参数 JSON 生成器（RFC-0026 §4.6 四节闭集；
spec/visual_comparison.md RXS-0384 schema）。

输入 = G10.3 冻结语料相机/光照登记（milestones/g10/corpus/camera_*.json /
lighting_*.json，M133 清单冻结面），输出 = 双端确定性契约参数 JSON
（milestones/g10/corpus/contract_params_<scene>.json，LF + 尾换行，确定性
逐字节——同输入同输出）。

约定（与 Rurix 侧渲染 harness g10_5_scene_render 同源登记）：
  - 契约世界系 = glTF 场景空间（右手系 / +Y up；CornellBox 沿用生成器
    毫米量级数值面、BistroInterior 为米——双端同数值消费，缩放口径一致，
    见 G10.5a preview caliber 登记）；
  - 相机约定：forward = R(q)·(0,0,-1)、up = R(q)·(0,1,0)（glTF 相机惯例），
    R 由 eye/target/up 经 look-at 右手基底 [s,u,−f] 构造（s=normalize(f×up)、
    u=cross(s,f)）；UE 侧经 g10_param_contract 冻结公式映射（quat M 相似变换 +
    fov_y→fov_h 换算）；
  - sun.direction = 光线**传播方向**（UE DirectionalLight 前向惯例）；Rurix 侧
    GiScene.sun_dir（指向光源）= −direction；
  - CornellBox 语料点光源不可由契约 lighting 节表达（schema 闭集仅 sun+sky），
    按契约降为 sun+sky 口径（sun 自开口侧射入盒内）——偏差如实登记进
    g10_5_ab_preview.md，不改 schema。

用法：py -3 milestones/g10/harness/g10_5_gen_contract_params.py
"""
from __future__ import annotations

import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CORPUS = ROOT / "milestones" / "g10" / "corpus"


def normalize(v):
    n = math.sqrt(sum(x * x for x in v))
    return tuple(x / n for x in v)


def mat3_to_quat_wxyz(m):
    """列主读取：m[c] = 第 c 列 (x,y,z)。返回 (w,x,y,z)。标准无分支算法。"""
    m00, m10, m20 = m[0]
    m01, m11, m21 = m[1]
    m02, m12, m22 = m[2]
    t = m00 + m11 + m22
    if t > 0.0:
        s = math.sqrt(t + 1.0) * 2.0
        w = 0.25 * s
        x = (m21 - m12) / s
        y = (m02 - m20) / s
        z = (m10 - m01) / s
    elif m00 > m11 and m00 > m22:
        s = math.sqrt(1.0 + m00 - m11 - m22) * 2.0
        w = (m21 - m12) / s
        x = 0.25 * s
        y = (m01 + m10) / s
        z = (m02 + m20) / s
    elif m11 > m22:
        s = math.sqrt(1.0 + m11 - m00 - m22) * 2.0
        w = (m02 - m20) / s
        x = (m01 + m10) / s
        y = 0.25 * s
        z = (m12 + m21) / s
    else:
        s = math.sqrt(1.0 + m22 - m00 - m11) * 2.0
        w = (m10 - m01) / s
        x = (m02 + m20) / s
        y = (m12 + m21) / s
        z = 0.25 * s
    n = math.sqrt(w * w + x * x + y * y + z * z)
    return (w / n, x / n, y / n, z / n)


def quat_rotate_wxyz(q, v):
    """主动旋转 v' = q·v·q*（列向量、右手定则正方向）。"""
    w, x, y, z = q
    ux, uy, uz = x, y, z
    uvx = uy * v[2] - uz * v[1]
    uvy = uz * v[0] - ux * v[2]
    uvz = ux * v[1] - uy * v[0]
    uuvx = uy * uvz - uz * uvy
    uuvy = uz * uvx - ux * uvz
    uuvz = ux * uvy - uy * uvx
    return (
        v[0] + 2.0 * (w * uvx + uuvx),
        v[1] + 2.0 * (w * uvy + uuvy),
        v[2] + 2.0 * (w * uvz + uuvz),
    )


def camera_quat(eye, target, up):
    """eye/target/up → 契约四元数（look-at 右手基底 [s,u,−f]，q·(0,0,−1)=f）。"""
    f = normalize(tuple(target[i] - eye[i] for i in range(3)))
    s = normalize((
        f[1] * up[2] - f[2] * up[1],
        f[2] * up[0] - f[0] * up[2],
        f[0] * up[1] - f[1] * up[0],
    ))
    u = (
        s[1] * f[2] - s[2] * f[1],
        s[2] * f[0] - s[0] * f[2],
        s[0] * f[1] - s[1] * f[0],
    )
    neg_f = (-f[0], -f[1], -f[2])
    q = mat3_to_quat_wxyz((s, u, neg_f))
    # 自证：q·(0,0,−1) == f、q·(0,1,0) == u（数值 1e-12 界）
    got_f = quat_rotate_wxyz(q, (0.0, 0.0, -1.0))
    got_u = quat_rotate_wxyz(q, (0.0, 1.0, 0.0))
    for a, b in zip(got_f, f):
        assert abs(a - b) < 1e-12, f"quat 自证失败 forward {got_f} != {f}"
    for a, b in zip(got_u, u):
        assert abs(a - b) < 1e-12, f"quat 自证失败 up {got_u} != {u}"
    return q


def build(scene_id: str) -> dict:
    cam = json.loads((CORPUS / f"camera_{scene_id.replace('-', '_')}.json").read_text(encoding="utf-8"))
    eye = tuple(float(v) for v in cam["eye"])
    target = tuple(float(v) for v in cam["target"])
    up = tuple(float(v) for v in cam["up"])
    q = camera_quat(eye, target, up)
    w, h = (int(v) for v in cam["resolution"])
    if scene_id == "cornell-box":
        # 盒体纵深 ~559（毫米量级数值面），相机距前墙 800：near/far 按场景尺度取。
        near, far = 10.0, 3000.0
        # 契约 sun+sky 口径：sun 自开口侧（−Z 侧）斜向射入盒内（传播方向 +Z 偏 −Y）。
        sun_dir = normalize((0.0, -0.5, 1.0))
        sun_lux = 5.0
        sun_rgb = [1.0, 1.0, 1.0]
        sky_i = 0.5
        ev100 = 2.0
    elif scene_id == "bistro-interior":
        near, far = 0.1, 200.0
        light = json.loads((CORPUS / "lighting_bistro_interior.json").read_text(encoding="utf-8"))
        d0 = next(l for l in light["lights"] if l["type"] == "directional")
        sun_dir = normalize(tuple(float(v) for v in d0["direction"]))
        sun_lux = float(d0["intensity"])
        sun_rgb = [float(v) for v in d0["color"][:3]]
        sky_i = 5.0
        ev100 = 1.0
    else:
        raise RuntimeError(f"未知场景 {scene_id}")
    return {
        "camera": {
            "position": [eye[0], eye[1], eye[2]],
            "orientation_quat": [q[0], q[1], q[2], q[3]],
            "fov_y_deg": float(cam["fov_y_deg"]),
            "near": near,
            "far": far,
            "resolution": {"w": w, "h": h},
        },
        "lighting": {
            "sun": {
                "direction": [sun_dir[0], sun_dir[1], sun_dir[2]],
                "intensity_lux": sun_lux,
                "color_linear_rgb": sun_rgb,
            },
            "sky": {"intensity": sky_i, "cubemap_id": None},
            "exposure": {"mode": "manual", "ev100": ev100},
        },
        "time": {
            "fixed_dt_s": 0.03333333333333333,
            "warmup_frames": 64,
            "capture_frame_index": 0,
            "random_seed": 42,
            "jitter": {"sequence": "halton_2_3", "index_base": 0, "scale": 1.0},
        },
        "post": {
            "view_transform": "aces13",
            "bloom": False,
            "vignette": False,
            "motion_blur": False,
            "dof": False,
        },
    }


def main() -> int:
    for scene_id in ("cornell-box", "bistro-interior"):
        doc = build(scene_id)
        out = CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        out.write_text(text, encoding="utf-8", newline="\n")
        print(f"[g10_5_params] {out.name} written")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
