# -*- coding: utf-8 -*-
"""Day 0829 臂④ 逐三角切线数学参考（CPU 纯 numpy;运行时接线的对拍锚）。

背景：bistro-interior glTF 无 TANGENT 顶点属性（census
primitives_with_tangent=0）,臂④接线须在装配侧按 UV 导数逐三角求切线。
本文件 = 该数学的确定性参考实现 + 单测,不进运行时。

UV 导数法（P = P0 + T·Δu + B·Δv 的最小二乘精确解,三角形上恰定）：
  dP1 = P1−P0, dP2 = P2−P0;  (du1,dv1) = UV1−UV0, (du2,dv2) = UV2−UV0
  det   = du1·dv2 − du2·dv1
  T_raw = (dP1·dv2 − dP2·dv1)/det       （∂P/∂u）
  B_raw = (dP2·du1 − dP1·du2)/det       （∂P/∂v）
  N     = normalize(cross(dP1, dP2))    （几何法线;可用平滑法线覆盖）
  T     = normalize(T_raw − N·dot(N,T_raw))   （Gram-Schmidt 对 N 正交化）
  w     = sign(dot(cross(N,T), B_raw))  （手性;glTF 约定 B = w·cross(N,T),
                                          dot==0 时取 +1 确定性缺省）
退化闭集（|det|≤eps / 几何零面积 / T_raw 投影后≈0）→ fallback：
  取 N 最小 |分量| 轴 a, T = normalize(cross(a,N)), w=+1
  （确定性任意正交基;全路径不产 NaN）。

单测：python tangent_ref.py（unittest,7 例）。
"""
from __future__ import annotations

import sys
import unittest

import numpy as np

EPS_DET = 1e-12
EPS_LEN = 1e-20


def _normalize(v: np.ndarray) -> tuple[np.ndarray, float]:
    n = float(np.linalg.norm(v))
    if n * n <= EPS_LEN:
        return np.zeros(3), 0.0
    return v / n, n


def _fallback_basis(n: np.ndarray) -> np.ndarray:
    """N 最小 |分量| 轴叉积 → 确定性正交单位切线。"""
    axis = np.zeros(3)
    axis[int(np.argmin(np.abs(n)))] = 1.0
    t, ln = _normalize(np.cross(axis, n))
    assert ln > 0.0, "fallback 轴选择保证非平行"
    return t


def triangle_tangent_frame(
    p0, p1, p2, uv0, uv1, uv2, n_override=None
) -> dict:
    """逐三角切线帧。

    返回 dict: T(单位,⊥N), B_raw(未归一 ∂P/∂v;退化时 = w·cross(N,T)),
    N(单位), w(±1.0), fallback(bool)。n_override = 平滑法线覆盖
    （运行时逐顶点平滑法线正交化路径;None 用几何法线）。
    """
    p0, p1, p2 = (np.asarray(x, dtype=np.float64) for x in (p0, p1, p2))
    uv0, uv1, uv2 = (np.asarray(x, dtype=np.float64) for x in (uv0, uv1, uv2))
    dp1 = p1 - p0
    dp2 = p2 - p0

    n_geo, area2 = _normalize(np.cross(dp1, dp2))
    if n_override is not None:
        n, ln = _normalize(np.asarray(n_override, dtype=np.float64))
        if ln == 0.0:
            n, area2 = n_geo, area2
    else:
        n = n_geo
    if area2 == 0.0 and n_override is None:
        # 几何零面积:法线不可得 → 全帧确定性 fallback（+Z 锚）。
        n = np.array([0.0, 0.0, 1.0])
        t = _fallback_basis(n)
        return {"T": t, "B_raw": np.cross(n, t), "N": n, "w": 1.0, "fallback": True}

    du1, dv1 = uv1 - uv0
    du2, dv2 = uv2 - uv0
    det = du1 * dv2 - du2 * dv1
    if abs(det) <= EPS_DET:
        t = _fallback_basis(n)
        return {"T": t, "B_raw": np.cross(n, t), "N": n, "w": 1.0, "fallback": True}

    t_raw = (dp1 * dv2 - dp2 * dv1) / det
    b_raw = (dp2 * du1 - dp1 * du2) / det
    t, tlen = _normalize(t_raw - n * float(np.dot(n, t_raw)))
    if tlen == 0.0:
        # T_raw ∥ N（病态 UV/覆盖法线正对切向）→ fallback。
        t = _fallback_basis(n)
        return {"T": t, "B_raw": np.cross(n, t), "N": n, "w": 1.0, "fallback": True}
    s = float(np.dot(np.cross(n, t), b_raw))
    w = 1.0 if s >= 0.0 else -1.0
    return {"T": t, "B_raw": b_raw, "N": n, "w": w, "fallback": False}


# ---------------------------------------------------------------------------
# 单测
# ---------------------------------------------------------------------------

class TangentRefTests(unittest.TestCase):
    P0 = [0.0, 0.0, 0.0]
    P1 = [1.0, 0.0, 0.0]
    P2 = [0.0, 1.0, 0.0]

    def assert_finite_unit(self, f):
        for k in ("T", "B_raw", "N"):
            self.assertTrue(np.all(np.isfinite(f[k])), f"{k} 含 NaN/Inf")
        self.assertAlmostEqual(float(np.linalg.norm(f["T"])), 1.0, places=12)
        self.assertAlmostEqual(float(np.dot(f["T"], f["N"])), 0.0, places=12)
        self.assertIn(f["w"], (1.0, -1.0))

    def test_axis_aligned_identity_uv(self):
        """UV≡XY 平面坐标 → T=+X, B=+Y, N=+Z, w=+1。"""
        f = triangle_tangent_frame(self.P0, self.P1, self.P2, [0, 0], [1, 0], [0, 1])
        self.assertFalse(f["fallback"])
        np.testing.assert_allclose(f["T"], [1, 0, 0], atol=1e-12)
        np.testing.assert_allclose(f["B_raw"], [0, 1, 0], atol=1e-12)
        np.testing.assert_allclose(f["N"], [0, 0, 1], atol=1e-12)
        self.assertEqual(f["w"], 1.0)

    def test_v_flipped_handedness(self):
        """V 轴镜像（DirectX 风格 v 向下）→ T 不变, B=−Y, w=−1。"""
        f = triangle_tangent_frame(self.P0, self.P1, self.P2, [0, 1], [1, 1], [0, 0])
        self.assertFalse(f["fallback"])
        np.testing.assert_allclose(f["T"], [1, 0, 0], atol=1e-12)
        np.testing.assert_allclose(f["B_raw"], [0, -1, 0], atol=1e-12)
        self.assertEqual(f["w"], -1.0)

    def test_uniform_uv_scale_invariance(self):
        """UV 均匀 ×5：T/N/w 不变（T_raw 模长变,方向归一后同）。"""
        f1 = triangle_tangent_frame(self.P0, self.P1, self.P2, [0, 0], [1, 0], [0, 1])
        f5 = triangle_tangent_frame(self.P0, self.P1, self.P2, [0, 0], [5, 0], [0, 5])
        np.testing.assert_allclose(f5["T"], f1["T"], atol=1e-12)
        self.assertEqual(f5["w"], f1["w"])

    def test_rotation_equivariance(self):
        """位置整体旋转 R：T' = R·T, w 不变（帧随几何协变）。"""
        rng = np.random.default_rng(20260829)
        a, b, c = rng.uniform(0, 2 * np.pi, 3)
        rx = np.array([[1, 0, 0], [0, np.cos(a), -np.sin(a)], [0, np.sin(a), np.cos(a)]])
        ry = np.array([[np.cos(b), 0, np.sin(b)], [0, 1, 0], [-np.sin(b), 0, np.cos(b)]])
        rz = np.array([[np.cos(c), -np.sin(c), 0], [np.sin(c), np.cos(c), 0], [0, 0, 1]])
        r = rz @ ry @ rx
        uv = ([0.2, 0.7], [0.9, 0.7], [0.2, 0.1])
        f0 = triangle_tangent_frame(self.P0, self.P1, self.P2, *uv)
        fr = triangle_tangent_frame(r @ self.P0, r @ self.P1, r @ self.P2, *uv)
        self.assert_finite_unit(fr)
        np.testing.assert_allclose(fr["T"], r @ f0["T"], atol=1e-10)
        np.testing.assert_allclose(fr["N"], r @ f0["N"], atol=1e-10)
        self.assertEqual(fr["w"], f0["w"])

    def test_degenerate_uv_no_nan(self):
        """UV 全同点（det=0）→ fallback 正交基,不 NaN。"""
        f = triangle_tangent_frame(self.P0, self.P1, self.P2, [0.5, 0.5], [0.5, 0.5], [0.5, 0.5])
        self.assertTrue(f["fallback"])
        self.assert_finite_unit(f)
        np.testing.assert_allclose(f["N"], [0, 0, 1], atol=1e-12)
        self.assertEqual(f["w"], 1.0)

    def test_degenerate_geometry_no_nan(self):
        """共线三点（零面积）→ fallback,不 NaN。"""
        f = triangle_tangent_frame([0, 0, 0], [1, 1, 1], [2, 2, 2], [0, 0], [1, 0], [0, 1])
        self.assertTrue(f["fallback"])
        self.assert_finite_unit(f)

    def test_smooth_normal_orthogonalization(self):
        """平滑法线覆盖（N 倾斜）：T ⊥ N_override 且 w 仍由 B_raw 定向。

        几何 T_raw 恒在三角平面内（⊥几何 N,Gram-Schmidt 近恒等）;
        正交化真正承重的是运行时平滑法线路径——此处显式覆盖验证。
        """
        n_tilt = [0.3, -0.2, 1.0]
        f = triangle_tangent_frame(
            self.P0, self.P1, self.P2, [0, 0], [1, 0], [0, 1], n_override=n_tilt
        )
        self.assertFalse(f["fallback"])
        self.assert_finite_unit(f)
        n_unit = np.asarray(n_tilt) / np.linalg.norm(n_tilt)
        np.testing.assert_allclose(f["N"], n_unit, atol=1e-12)
        self.assertAlmostEqual(float(np.dot(f["T"], f["N"])), 0.0, places=12)
        self.assertEqual(f["w"], 1.0)


if __name__ == "__main__":
    sys.exit(unittest.main(verbosity=2))
