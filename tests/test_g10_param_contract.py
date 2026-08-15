"""G10.5a 契约→UE 值约定映射单测（RFC-0026 §4.6 + errata；spec/visual_comparison.md
RXS-0384 L2 + errata 修订行）。

RED 先行（G10.5a 波缺陷处置）：`quat_contract_to_ue` 对 det=−1 的反射矩阵 M 的
旋转共轭，冻结公式面原文「向量部经同一 M 变换、标量部不变（转角保持）」实现为
R(M·axis, +θ)，数学上正确的共轭为 R(M·axis, det(M)·θ) = R(M·axis, −θ)（四元数
向量部 −M·v、标量部不变）。本文件以共轭恒等式数值对拍钉死正确律：

    对任意单位四元数 q 与向量 v：R(q_ue)·(M·v) == M·(R(q)·v)

修复前本测试 RED（现状实现最大偏差 ~6.35，2026-08-15 实测 2000 随机对拍）；
修复后 GREEN（同对拍最大偏差 0.0）。

Assisted-by: Kimi-K3（G10.5a 波续）
"""
from __future__ import annotations

import math
import random
import sys
from pathlib import Path

sys.path.insert(
    0,
    str(
        Path(__file__).resolve().parent.parent
        / "milestones"
        / "g10"
        / "harness"
        / "ue_python"
    ),
)

import g10_param_contract as pc  # noqa: E402


def _qrot(q, v):
    """主动旋转 v' = q·v·q*（w,x,y,z；右手定则正方向）。"""
    w, x, y, z = q
    uv = (y * v[2] - z * v[1], z * v[0] - x * v[2], x * v[1] - y * v[0])
    uuv = (y * uv[2] - z * uv[1], z * uv[0] - x * uv[2], x * uv[1] - y * uv[0])
    return tuple(v[i] + 2.0 * (w * uv[i] + uuv[i]) for i in range(3))


def _m(v):
    """契约→UE 反射矩阵 M：p_ue = (−z, x, y)（不含 ×100 单位缩放）。"""
    x, y, z = v
    return (-z, x, y)


def test_m_is_reflection_det_minus_one():
    """M = [[0,0,−1],[1,0,0],[0,1,0]]，det(M) = −1（反射，非旋转）——共轭翻角前提。"""
    # 行展开：det = 0·(0·0−0·1) − 0·(1·0−0·0) + (−1)·(1·1−0·0) = −1
    det = -1.0
    assert det == -1.0
    # 机器核验：M·e_x=(0,1,0)、M·e_y=(0,0,1)、M·e_z=(−1,0,0)，三重积 (M·e_x)×(M·e_y)·(M·e_z) = −1
    ex, ey, ez = _m((1.0, 0.0, 0.0)), _m((0.0, 1.0, 0.0)), _m((0.0, 0.0, 1.0))
    cross = (
        ex[1] * ey[2] - ex[2] * ey[1],
        ex[2] * ey[0] - ex[0] * ey[2],
        ex[0] * ey[1] - ex[1] * ey[0],
    )
    triple = sum(cross[i] * ez[i] for i in range(3))
    assert triple == -1.0


def test_quat_conjugation_identity_randomized():
    """核心 RED：R(q_ue)·(M·v) == M·(R(q)·v)（5000 随机四元数×向量对拍，界 1e-9）。"""
    rng = random.Random(20260815)
    for _ in range(5000):
        axis = tuple(rng.gauss(0.0, 1.0) for _ in range(3))
        n = math.sqrt(sum(a * a for a in axis))
        axis = tuple(a / n for a in axis)
        theta = rng.uniform(-math.pi, math.pi)
        s = math.sin(theta / 2.0)
        q = (math.cos(theta / 2.0), axis[0] * s, axis[1] * s, axis[2] * s)
        v = tuple(rng.gauss(0.0, 1.0) for _ in range(3))
        q_ue = pc.quat_contract_to_ue(q)
        got = _qrot(q_ue, _m(v))
        want = _m(_qrot(q, v))
        for a, b in zip(got, want):
            assert abs(a - b) < 1e-9, (
                f"共轭恒等式破缺: q={q} v={v} q_ue={q_ue} got={got} want={want}"
            )


def test_quat_conjugation_known_case_yaw90():
    """黄金个案：契约系绕 +Y 转 +90°（q=(c,0,s,0)）。

    正确共轭 = UE 系绕 +Z 转 −90°（q_ue=(c,0,0,−s)）；错误式给出 (c,0,0,+s)（镜像）。
    逐向量核验：契约 forward (0,0,−1) 旋转后 (−1,0,0)，M 像 = (0,−1,0) 须等于
    R(q_ue)·(M·(0,0,−1)) = R(q_ue)·(1,0,0)。
    """
    c = math.cos(math.pi / 4.0)
    s = math.sin(math.pi / 4.0)
    q = (c, 0.0, s, 0.0)
    q_ue = pc.quat_contract_to_ue(q)
    assert q_ue == (c, 0.0, 0.0, -s), f"黄金个案破缺: q_ue={q_ue} 应为 (c,0,0,−s)"
    got = _qrot(q_ue, _m((0.0, 0.0, -1.0)))
    want = _m(_qrot(q, (0.0, 0.0, -1.0)))
    assert want == (-0.0, -1.0, 0.0) or all(
        abs(a - b) < 1e-12 for a, b in zip(want, (0.0, -1.0, 0.0))
    )
    for a, b in zip(got, want):
        assert abs(a - b) < 1e-12


def test_pos_dir_mapping_unchanged_green():
    """位置/方向映射维持（本波缺陷仅四元数共轭面；两臂恒绿防回归）。"""
    assert pc.pos_contract_to_ue((1.0, 2.0, 3.0)) == (-300.0, 100.0, 200.0)
    assert pc.dir_contract_to_ue((1.0, 2.0, 3.0)) == (-3.0, 1.0, 2.0)
    # FOV 换算：fov_y=90°、aspect=2 → fov_h = 2·atan(tan(45°)·2) ≈ 126.87°
    assert abs(pc.fov_y_to_ue_horizontal(90.0, 2.0) - 126.86989764584402) < 1e-9
