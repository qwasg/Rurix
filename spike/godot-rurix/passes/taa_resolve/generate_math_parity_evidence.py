#!/usr/bin/env python3
"""GRX-012: taa_resolve math parity evidence generator (CPU reference vs GPU).

Computes a CPU float32 reference for the single-frame TAA resolve math subset
(Spartan-derived; see PASS_CONTRACT.md §5) and writes
``math_parity_evidence.json`` with three >=8-frame time-series cases
(static convergence / motion disocclusion / off-screen reset) plus a dedicated
single-frame dispatch fixture the standalone smoke re-verifies on a real GPU.

All arithmetic is rounded to IEEE-754 binary32 after every scalar operation so
the expected values are close to a D3D12 dispatch of the tracked
``artifacts/hlsl_bridge/taa_resolve.hlsl`` kernel (the comparison uses a small
absolute tolerance to absorb legal fp contraction / hardware transcendental
differences).

The reference reproduces the HLSL faithfully, including:
  * the groupshared 10x10 tile as direct clamped color/depth loads (equivalent),
  * the get_closest_pixel_velocity_3x3 border-offset quirk (velocity fetched at
    pos_screen - 1 + best_offset, Load out-of-bounds = 0),
  * the 9-tap Catmull-Rom history sampling with hardware bilinear reproduced as
    explicit float 4-tap Load bilinear + clamp addressing,
  * clip_aabb variance clipping, Reinhard-domain blend, inverse Reinhard.

GPU side: honest stub. Without a GPU results document the evidence records
``status=pending_gpu_dispatch`` with ``gpu_observed=null``. The standalone
dispatch smoke (ci/grx012_taa_resolve_d3d12_dispatch_smoke.py) separately
verifies every measured GPU texel against ``taa_resolve_frame`` on the dispatch
fixture. This evidence never implies real_gpu_pass=true or default enablement.
"""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import math
import pathlib
import struct
import sys


PASS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = PASS_DIR.parents[3]
BRIDGE_DIR = PASS_DIR / "artifacts" / "hlsl_bridge"
HLSL_PATH = BRIDGE_DIR / "taa_resolve.hlsl"
DXIL_PATH = PASS_DIR / "artifacts" / "taa_resolve.dxil"
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

# TAA resolve has division-heavy math (reinhard_inverse, clip_aabb ratios) plus
# a sqrt/transcendental; keep a modest tolerance that absorbs legal fp
# contraction and hardware differences. The measured smoke records the real gap.
MAX_ABS_ERROR_TOLERANCE = 3e-3

RPC_9 = 1.0 / 9.0
RPC_16 = 1.0 / 16.0
DISOCCLUSION_SCALE = 0.01
FLT_MIN_TAA = 1e-8
FLT_MAX_TAA = 32767.0
LUM = (0.299, 0.587, 0.114)


def f(x: float) -> float:
    """Round a Python float to IEEE-754 binary32."""
    return struct.unpack("<f", struct.pack("<f", float(x)))[0]


def f3(v) -> tuple[float, float, float]:
    return (f(v[0]), f(v[1]), f(v[2]))


def vadd(a, b):
    return (f(a[0] + b[0]), f(a[1] + b[1]), f(a[2] + b[2]))


def vsub(a, b):
    return (f(a[0] - b[0]), f(a[1] - b[1]), f(a[2] - b[2]))


def vmul(a, b):
    return (f(a[0] * b[0]), f(a[1] * b[1]), f(a[2] * b[2]))


def vscale(a, s):
    s = f(s)
    return (f(a[0] * s), f(a[1] * s), f(a[2] * s))


def clampf(x, lo, hi):
    x = f(x)
    if x < lo:
        x = lo
    if x > hi:
        x = hi
    return f(x)


def vclamp(a, lo, hi):
    return (clampf(a[0], lo, hi), clampf(a[1], lo, hi), clampf(a[2], lo, hi))


def lerpf(a, b, t):
    a, b, t = f(a), f(b), f(t)
    return f(a + f(f(b - a) * t))


def vlerp(a, b, t):
    return (lerpf(a[0], b[0], t), lerpf(a[1], b[1], t), lerpf(a[2], b[2], t))


def smoothstep(a, b, x):
    a, b, x = f(a), f(b), f(x)
    t = clampf(f(f(x - a) / f(b - a)), 0.0, 1.0)
    return f(f(t * t) * f(3.0 - f(2.0 * t)))


def length2(vx, vy):
    return f(math.sqrt(f(f(f(vx) * f(vx)) + f(f(vy) * f(vy)))))


def reinhard(v):
    return (f(v[0] / f(v[0] + 1.0)), f(v[1] / f(v[1] + 1.0)), f(v[2] / f(v[2] + 1.0)))


def reinhard_inverse(v):
    return (f(v[0] / f(1.0 - v[0])), f(v[1] / f(1.0 - v[1])), f(v[2] / f(1.0 - v[2])))


def luminance(v):
    d = f(f(f(v[0] * LUM[0]) + f(v[1] * LUM[1])) + f(v[2] * LUM[2]))
    return f(max(d, 0.0001))


def clip_aabb(aabb_min, aabb_max, p, q):
    r = [f(q[i] - p[i]) for i in range(3)]
    rmax = [f(aabb_max[i] - p[i]) for i in range(3)]
    rmin = [f(aabb_min[i] - p[i]) for i in range(3)]
    if r[0] > f(rmax[0] + FLT_MIN_TAA):
        r = [f(r[i] * f(rmax[0] / r[0])) for i in range(3)]
    if r[1] > f(rmax[1] + FLT_MIN_TAA):
        r = [f(r[i] * f(rmax[1] / r[1])) for i in range(3)]
    if r[2] > f(rmax[2] + FLT_MIN_TAA):
        r = [f(r[i] * f(rmax[2] / r[2])) for i in range(3)]
    if r[0] < f(rmin[0] - FLT_MIN_TAA):
        r = [f(r[i] * f(rmin[0] / r[0])) for i in range(3)]
    if r[1] < f(rmin[1] - FLT_MIN_TAA):
        r = [f(r[i] * f(rmin[1] / r[1])) for i in range(3)]
    if r[2] < f(rmin[2] - FLT_MIN_TAA):
        r = [f(r[i] * f(rmin[2] / r[2])) for i in range(3)]
    return (f(p[0] + r[0]), f(p[1] + r[1]), f(p[2] + r[2]))


# ── clamped loads over full-frame input arrays (LDS-equivalent) ──────────────

def _cx(x, w):
    return 0 if x < 0 else (w - 1 if x > w - 1 else x)


def load_color(color, x, y, w, h):
    return color[_cx(y, h)][_cx(x, w)]


def load_depth(depth, x, y, w, h):
    return f(depth[_cx(y, h)][_cx(x, w)])


def load_velocity_oob0(vel, x, y, w, h):
    # Native imageLoad returns 0 out of bounds; Texture2D.Load matches.
    if x < 0 or y < 0 or x > w - 1 or y > h - 1:
        return (0.0, 0.0)
    return vel[y][x]


def sample_history_bilinear(history, uvx, uvy, w, h):
    sx = f(f(uvx * w) - 0.5)
    sy = f(f(uvy * h) - 0.5)
    fx0 = math.floor(sx)
    fy0 = math.floor(sy)
    fracx = f(sx - fx0)
    fracy = f(sy - fy0)
    i0x, i0y = int(fx0), int(fy0)
    i1x, i1y = i0x + 1, i0y + 1
    i0x, i0y = _cx(i0x, w), _cx(i0y, h)
    i1x, i1y = _cx(i1x, w), _cx(i1y, h)
    c00 = history[i0y][i0x]
    c10 = history[i0y][i1x]
    c01 = history[i1y][i0x]
    c11 = history[i1y][i1x]
    top = vlerp(c00, c10, fracx)
    bot = vlerp(c01, c11, fracx)
    return vlerp(top, bot, fracy)


def sample_catmull_rom_9(history, uvx, uvy, w, h):
    spx = f(uvx * w)
    spy = f(uvy * h)
    tp1x = f(math.floor(f(spx - 0.5)) + 0.5)
    tp1y = f(math.floor(f(spy - 0.5)) + 0.5)
    fx = f(spx - tp1x)
    fy = f(spy - tp1y)

    def weights(t):
        w0 = f(t * f(-0.5 + f(t * f(1.0 - f(0.5 * t)))))
        w1 = f(1.0 + f(f(t * t) * f(-2.5 + f(1.5 * t))))
        w2 = f(t * f(0.5 + f(t * f(2.0 - f(1.5 * t)))))
        w3 = f(f(t * t) * f(-0.5 + f(0.5 * t)))
        return w0, w1, w2, w3

    w0x, w1x, w2x, w3x = weights(fx)
    w0y, w1y, w2y, w3y = weights(fy)
    w12x = f(w1x + w2x)
    w12y = f(w1y + w2y)
    off12x = f(w2x / f(w1x + w2x))
    off12y = f(w2y / f(w1y + w2y))

    tp0x = f(f(tp1x - 1.0) / w)
    tp0y = f(f(tp1y - 1.0) / h)
    tp3x = f(f(tp1x + 2.0) / w)
    tp3y = f(f(tp1y + 2.0) / h)
    tp12x = f(f(tp1x + off12x) / w)
    tp12y = f(f(tp1y + off12y) / h)

    result = (0.0, 0.0, 0.0)
    rows = ((tp0y, w0y), (tp12y, w12y), (tp3y, w3y))
    cols = ((tp0x, w0x), (tp12x, w12x), (tp3x, w3x))
    for (ry, wy) in rows:
        for (rx, wx) in cols:
            s = sample_history_bilinear(history, rx, ry, w, h)
            result = vadd(result, vscale(s, f(wx * wy)))
    return (f(max(result[0], 0.0)), f(max(result[1], 0.0)), f(max(result[2], 0.0)))


KOFFS = [(-1, -1), (0, -1), (1, -1), (-1, 0), (0, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]


def get_closest_pixel_velocity(depth, vel, x, y, w, h):
    min_depth = 1.0
    best = (0, 0)
    for (ox, oy) in KOFFS:
        d = load_depth(depth, x + ox, y + oy, w, h)
        if d < min_depth:
            min_depth = d
            best = (ox, oy)
    # Native fetches velocity at group_top_left + min_pos = pos_screen - 1 + best.
    return load_velocity_oob0(vel, x - 1 + best[0], y - 1 + best[1], w, h)


def clip_history_3x3(color, x, y, w, h, color_history, velocity_closest, variance_dynamic):
    s = [load_color(color, x + ox, y + oy, w, h) for (ox, oy) in KOFFS]
    acc = (0.0, 0.0, 0.0)
    for v in s:
        acc = vadd(acc, v)
    color_avg = vscale(acc, RPC_9)
    acc2 = (0.0, 0.0, 0.0)
    for v in s:
        acc2 = vadd(acc2, vmul(v, v))
    color_avg2 = vscale(acc2, RPC_9)
    box_size = f(lerpf(0.0, variance_dynamic,
                       smoothstep(0.02, 0.0, length2(velocity_closest[0], velocity_closest[1]))))
    dev = []
    for i in range(3):
        diff = f(color_avg2[i] - f(color_avg[i] * color_avg[i]))
        dev.append(f(f(math.sqrt(f(abs(diff)))) * box_size))
    dev = tuple(dev)
    color_min = vsub(color_avg, dev)
    color_max = vadd(color_avg, dev)
    p = (clampf(color_avg[0], color_min[0], color_max[0]),
         clampf(color_avg[1], color_min[1], color_max[1]),
         clampf(color_avg[2], color_min[2], color_max[2]))
    color = clip_aabb(color_min, color_max, p, color_history)
    return vclamp(color, FLT_MIN_TAA, FLT_MAX_TAA)


def get_factor_disocclusion(last_velocity, uvrx, uvry, velocity, w, h, disocclusion_threshold):
    px = int(f(uvrx * w))
    py = int(f(uvry * h))
    vp = load_velocity_oob0(last_velocity, px, py, w, h)
    vtx = f(velocity[0] * w)
    vty = f(velocity[1] * h)
    ptx = f(vp[0] * w)
    pty = f(vp[1] * h)
    disocclusion = f(length2(f(ptx - vtx), f(pty - vty)) - disocclusion_threshold)
    return clampf(f(disocclusion * DISOCCLUSION_SCALE), 0.0, 1.0)


def taa_resolve_texel(x, y, w, h, color, depth, velocity, last_velocity, history,
                      disocclusion_threshold, variance_dynamic):
    velx, vely = velocity[y][x]
    uvx = f(f(x + 0.5) / w)
    uvy = f(f(y + 0.5) / h)
    uvrx = f(uvx + velx)
    uvry = f(uvy + vely)

    color_input = load_color(color, x, y, w, h)
    color_history = sample_catmull_rom_9(history, uvrx, uvry, w, h)

    velocity_closest = get_closest_pixel_velocity(depth, velocity, x, y, w, h)
    color_history = clip_history_3x3(color, x, y, w, h, color_history, velocity_closest, variance_dynamic)

    blend_factor = RPC_16
    factor_screen = 1.0 if (uvrx < 0.0 or uvry < 0.0 or uvrx > 1.0 or uvry > 1.0) else 0.0
    factor_disocclusion = get_factor_disocclusion(last_velocity, uvrx, uvry, (velx, vely), w, h, disocclusion_threshold)
    blend_factor = clampf(f(f(blend_factor + factor_screen) + factor_disocclusion), 0.0, 1.0)

    color_history = reinhard(color_history)
    color_input = reinhard(color_input)
    lum_color = luminance(color_input)
    lum_history = luminance(color_history)
    diff = f(f(abs(f(lum_color - lum_history))) / f(max(lum_color, f(max(lum_history, 1.001)))))
    diff = f(1.0 - diff)
    diff = f(diff * diff)
    blend_factor = lerpf(0.0, blend_factor, diff)
    color_resolved = vlerp(color_history, color_input, blend_factor)
    color_resolved = reinhard_inverse(color_resolved)
    return [color_resolved[0], color_resolved[1], color_resolved[2], 1.0]


def taa_resolve_frame(w, h, color, depth, velocity, last_velocity, history,
                      disocclusion_threshold, variance_dynamic):
    out = []
    for y in range(h):
        row = []
        for x in range(w):
            row.append(taa_resolve_texel(x, y, w, h, color, depth, velocity, last_velocity,
                                         history, disocclusion_threshold, variance_dynamic))
        out.append(row)
    return out


# ── deterministic synthetic inputs (replicable bit-for-bit in the C++ smoke) ──

def syn_color(x, y):
    return (f(((x * 7 + y * 13) % 97) / 96.0),
            f(((x * 11 + y * 5) % 89) / 88.0),
            f(((x * 3 + y * 17) % 83) / 82.0))


def syn_depth(x, y):
    return f(((x * 5 + y * 9) % 64) / 64.0)


def syn_velocity(x, y):
    return (f((((x * 2 + y) % 7) - 3) / 256.0),
            f((((x + y * 2) % 7) - 3) / 256.0))


def syn_last_velocity(x, y):
    return (f((((x + y * 3) % 5) - 2) / 256.0),
            f((((x * 3 + y) % 5) - 2) / 256.0))


def syn_history(x, y):
    return (f(((x * 13 + y * 7) % 91) / 90.0),
            f(((x * 17 + y * 3) % 79) / 78.0),
            f(((x * 5 + y * 11) % 73) / 72.0))


def build_dispatch_fixture(w, h):
    color = [[syn_color(x, y) for x in range(w)] for y in range(h)]
    depth = [[syn_depth(x, y) for x in range(w)] for y in range(h)]
    velocity = [[syn_velocity(x, y) for x in range(w)] for y in range(h)]
    last_velocity = [[syn_last_velocity(x, y) for x in range(w)] for y in range(h)]
    history = [[syn_history(x, y) for x in range(w)] for y in range(h)]
    return color, depth, velocity, last_velocity, history


DISPATCH_WIDTH = 16
DISPATCH_HEIGHT = 16
DISPATCH_VARIANCE_DYNAMIC = 1.0


def dispatch_disocclusion_threshold(w, h):
    return f(0.1 / max(w, h))


# ── evidence assembly ────────────────────────────────────────────────────────

def sha256_of_file(path: pathlib.Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def rel(path: pathlib.Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def utc_now() -> str:
    return dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def sample_points(w, h):
    return [(0, 0), (w // 2, h // 2), (w - 1, h - 1), (1, h - 2)]


def flat_sha(frame):
    values = [v for row in frame for texel in row for v in texel]
    return hashlib.sha256(struct.pack(f"<{len(values)}f", *values)).hexdigest()


def time_series_case(case_id, w, h, velocity, last_velocity, description, frames=8):
    """Feed output->history over `frames` frames on a static color/depth scene
    and record per-frame convergence + sample texels."""
    color = [[syn_color(x, y) for x in range(w)] for y in range(h)]
    depth = [[syn_depth(x, y) for x in range(w)] for y in range(h)]
    # history starts DIFFERENT from color (a shifted pattern) so convergence /
    # disocclusion / reset behaviour is observable over the frame sequence.
    history = [[syn_history(x, y) for x in range(w)] for y in range(h)]
    dth = dispatch_disocclusion_threshold(w, h)
    vd = 1.0
    per_frame = []
    for frame in range(frames):
        out = taa_resolve_frame(w, h, color, depth, velocity, last_velocity, history, dth, vd)
        # convergence metric: max abs channel diff of output vs the (static) color.
        max_to_color = 0.0
        for y in range(h):
            for x in range(w):
                for c in range(3):
                    max_to_color = max(max_to_color, abs(out[y][x][c] - color[y][x][c]))
        samples = [{"x": px, "y": py, "rgba": out[py][px]} for (px, py) in sample_points(w, h)]
        per_frame.append({
            "frame": frame,
            "max_abs_output_minus_current_color": f(max_to_color),
            "output_sha256": flat_sha(out),
            "sample_texels": samples,
        })
        history = [[(out[y][x][0], out[y][x][1], out[y][x][2]) for x in range(w)] for y in range(h)]
    return {
        "case_id": case_id,
        "description": description,
        "width": w,
        "height": h,
        "frames": frames,
        "constants": {
            "disocclusion_threshold": dth,
            "variance_dynamic": vd,
            "velocity_uv": list(velocity[0][0]),
            "last_velocity_uv": list(last_velocity[0][0]),
        },
        "convergence_trend_max_abs_to_color": [pf["max_abs_output_minus_current_color"] for pf in per_frame],
        "per_frame": per_frame,
    }


def dispatch_fixture_case():
    w, h = DISPATCH_WIDTH, DISPATCH_HEIGHT
    color, depth, velocity, last_velocity, history = build_dispatch_fixture(w, h)
    dth = dispatch_disocclusion_threshold(w, h)
    out = taa_resolve_frame(w, h, color, depth, velocity, last_velocity, history, dth, DISPATCH_VARIANCE_DYNAMIC)
    samples = [{"x": px, "y": py, "rgba": out[py][px]} for (px, py) in sample_points(w, h)]
    return {
        "case_id": "taa_resolve_dispatch_fixture_16x16",
        "description": "single-frame resolve fixture the standalone D3D12 smoke re-verifies on a real GPU",
        "width": w,
        "height": h,
        "constants": {
            "source_width": w,
            "source_height": h,
            "disocclusion_threshold": dth,
            "variance_dynamic": DISPATCH_VARIANCE_DYNAMIC,
        },
        "input_patterns": {
            "color": "r=((x*7+y*13)%97)/96, g=((x*11+y*5)%89)/88, b=((x*3+y*17)%83)/82",
            "depth": "((x*5+y*9)%64)/64",
            "velocity": "x=(((x*2+y)%7)-3)/256, y=(((x+y*2)%7)-3)/256",
            "last_velocity": "x=(((x+y*3)%5)-2)/256, y=(((x*3+y)%5)-2)/256",
            "history": "r=((x*13+y*7)%91)/90, g=((x*17+y*3)%79)/78, b=((x*5+y*11)%73)/72",
        },
        "cpu_expected_rgba_f32_le_sha256": flat_sha(out),
        "cpu_expected_sample_texels": samples,
        "gpu_observed": None,
    }


def main() -> int:
    w, h = 12, 10
    zero_vel = [[(0.0, 0.0) for _ in range(w)] for _ in range(h)]
    # Static: no motion, history converges to the (static) color.
    static = time_series_case(
        "static_convergence", w, h, zero_vel, zero_vel,
        "velocity=last_velocity=0: uv_reprojected stays in-screen, no disocclusion; "
        "history converges to the static color over the frame sequence "
        "(convergence_trend decreases toward 0)",
    )
    # Motion disocclusion: constant small in-screen velocity that differs from
    # last_velocity, so get_factor_disocclusion > 0 boosts the blend factor.
    motion_vel = [[(f(2.0 / 256.0), f(1.0 / 256.0)) for _ in range(w)] for _ in range(h)]
    motion_last = [[(f(-2.0 / 256.0), f(-1.0 / 256.0)) for _ in range(w)] for _ in range(h)]
    motion = time_series_case(
        "motion_disocclusion", w, h, motion_vel, motion_last,
        "small in-screen velocity that differs from last_velocity: "
        "get_factor_disocclusion > 0 raises the blend factor vs the static case "
        "(faster convergence to the current color)",
    )
    # Off-screen reset: velocity large enough that uv_reprojected leaves [0,1]
    # for most texels, so factor_screen=1 forces the maximum blend (reset to
    # current color).
    reset_vel = [[(f(1.5), f(1.5)) for _ in range(w)] for _ in range(h)]
    reset_last = [[(0.0, 0.0) for _ in range(w)] for _ in range(h)]
    reset = time_series_case(
        "offscreen_reset", w, h, reset_vel, reset_last,
        "velocity pushes uv_reprojected out of [0,1] for every texel: "
        "factor_screen=1 forces the maximum blend so the output resets toward "
        "the current color each frame (convergence_trend stays near 0)",
    )
    dispatch = dispatch_fixture_case()

    evidence = {
        "pass_id": "taa_resolve",
        "subject": "grx012_taa_resolve_math_parity",
        "status": "pending_gpu_dispatch",
        "generated_at_utc": utc_now(),
        "math_status": "taa_resolve_cpu_reference_proven_pending_gpu_dispatch",
        "cpu_reference": {
            "formula": (
                "single-frame Spartan TAA resolve: velocity(pos); uv_reprojected = uv + velocity; "
                "color_history = clip_history_3x3(catmull_rom_9(history, uv_reprojected), "
                "get_closest_pixel_velocity_3x3); blend = clamp(1/16 + offscreen + disocclusion, 0, 1); "
                "blend = mix(0, blend, flicker_diff); resolved = reinhard_inverse(mix(reinhard(history), "
                "reinhard(input), blend)); every operation rounded to binary32; hardware bilinear "
                "reproduced as explicit float 4-tap Load bilinear; imageLoad OOB = 0"
            ),
            "hlsl_kernel": {"path": rel(HLSL_PATH), "sha256": sha256_of_file(HLSL_PATH)},
            "dxil": {"path": rel(DXIL_PATH), "sha256": sha256_of_file(DXIL_PATH)},
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_math_source": "external/godot-master/servers/rendering/renderer_rd/shaders/effects/taa_resolve.glsl",
        },
        "max_abs_error_tolerance": MAX_ABS_ERROR_TOLERANCE,
        "gpu_dispatch_kind": None,
        "time_series_cases": [static, motion, reset],
        "dispatch_fixture": dispatch,
        "notes": [
            "GPU-observed values are pending a real dispatch; the standalone smoke "
            "ci/grx012_taa_resolve_d3d12_dispatch_smoke.py independently verifies every "
            "measured GPU output texel against taa_resolve_frame on the dispatch fixture.",
            "The three time-series cases are CPU-only demonstrations (static convergence, "
            "motion disocclusion, off-screen reset) over >=8 frames feeding output->history; "
            "only the single-frame dispatch fixture is GPU-verified in this slice.",
            "Interior UVs are texel-exact vs Godot's hardware bilinear; hardware sub-texel "
            "fixed-point rounding and half storage quantization are recorded gaps.",
        ],
        "does_not_imply": [
            "Godot runtime TAA resolve pass completion",
            "real_gpu_pass=true",
            "real_d3d12_dispatch_recorded=true",
            "visual success",
            "temporal stability success",
            "GPU timestamp success",
            "performance claim",
            "default pass enablement",
        ],
    }
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[grx012-math-parity] status=pending_gpu_dispatch time_series=3 dispatch_fixture=1 "
          f"evidence={EVIDENCE_PATH}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
