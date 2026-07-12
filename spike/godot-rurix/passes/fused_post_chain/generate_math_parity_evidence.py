#!/usr/bin/env python3
"""GRX-019: fused_post_chain math parity evidence generator (CPU reference).

Computes a CPU float32 reference for the fused post-chain math subset (see
PASS_CONTRACT.md section 5) and writes ``math_parity_evidence.json`` with
four >=8-frame time-series cases covering the EMA sequence x tonemap
(LINEAR + sRGB) composite:

  * static scene EMA convergence,
  * brightness step adaptation,
  * clamp boundary at both ends,
  * first-frame reset (first_frame=1 then chained frames),

plus a dedicated single-frame dispatch fixture the (deferred S6) standalone
smoke re-verifies on a real GPU.

Per frame the previous 1x1 luminance is chained from the previous frame's
fused output (segment A), and the full-resolution LDR output (segment B) is
recorded as a SHA-256 over the little-endian f32 RGBA grid plus sample texels
- so the evidence demonstrates the EMA time series and the tonemap composite
together, exactly as the fused kernel computes them.

All arithmetic is rounded to IEEE-754 binary32 after every scalar operation so
the expected values are close to a D3D12 dispatch of the tracked
``artifacts/hlsl_bridge/fused_post_chain.hlsl`` kernel (the comparison uses a
small absolute tolerance to absorb legal fp contraction / hardware
transcendental differences).

The reference reproduces the HLSL faithfully, including:
  * segment A in the tracked member-kernel order (partial-tile-correct mean
    over the <= 8x8 lum_source with the member accumulation order, clamp then
    EMA; first_frame selects the clamped current value),
  * segment B in the native tonemap operation order (luminance_multiplier
    pre-scale, the tonemap.glsl L866-868 auto-exposure formula fed by the
    segment-A register value, TONEMAPPER_LINEAR identity, linear_to_srgb with
    the L230-233 coefficients, alpha passthrough).

A composition cross-check additionally verifies, texel for texel on the
dispatch fixture, that the fused segment-B output equals the GRX-010 member
tonemap reference evaluated with the fused effective exposure - documenting
that the fused kernel is the composition of the two member kernels.

GPU side: honest stub. Without a GPU results document the evidence records
``status=pending_gpu_dispatch`` with ``gpu_observed=null``. This evidence
never implies real_gpu_pass=true, default enablement, or any performance
claim.
"""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import pathlib
import struct
import sys


PASS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = PASS_DIR.parents[3]
BRIDGE_DIR = PASS_DIR / "artifacts" / "hlsl_bridge"
HLSL_PATH = BRIDGE_DIR / "fused_post_chain.hlsl"
DXIL_PATH = PASS_DIR / "artifacts" / "fused_post_chain.dxil"
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

# The fused chain has a divide (auto-exposure denominator) plus the sRGB pow;
# keep a modest tolerance that absorbs legal fp contraction and hardware
# transcendental differences. The (deferred S6) measured smoke records the
# real gap.
MAX_ABS_ERROR_TOLERANCE = 3e-3


def f(x: float) -> float:
    """Round a Python float to IEEE-754 binary32."""
    return struct.unpack("<f", struct.pack("<f", float(x)))[0]


def clampf(x: float, lo: float, hi: float) -> float:
    x = f(x)
    if x < lo:
        x = lo
    if x > hi:
        x = hi
    return f(x)


# ── segment A: luminance final WRITE_LUMINANCE level (member-kernel order) ──

def luminance_segment(lum_grid, lum_w, lum_h, prev, max_lum, min_lum,
                      exposure_adjust, first_frame):
    """Mirror of compute_luminance_current() in fused_post_chain.hlsl:
    member accumulation order (row-guarded outer, column-guarded inner over
    the single 8x8 tile), partial-tile-correct mean, clamp then EMA,
    first_frame selects the clamped current value."""
    accum = 0.0
    count = 0.0
    for dy in range(8):
        sy = dy
        if sy < lum_h:
            for dx in range(8):
                sx = dx
                if sx < lum_w:
                    accum = f(accum + lum_grid[sy][sx])
                    count = f(count + 1.0)
    avg = f(accum / count) if count > 0.0 else 0.0
    cur = clampf(avg, min_lum, max_lum)
    ema = f(prev + f(f(cur - prev) * f(exposure_adjust)))
    return cur if first_frame else ema, cur, avg


# ── segment B: tonemap LINEAR + sRGB (native operation order) ────────────────

def linear_to_srgb_f32(value: float) -> float:
    """Mirror of the HLSL linear_to_srgb (tonemap.glsl L230-233 coefficients):
    lo = 12.92*c; hi = 1.055*pow(max(c,0), 1/2.4) - 0.055; select on
    c < 0.0031308 (no extra negative clamp on the lo leg, matching the HLSL)."""
    v = f(value)
    lo = f(f(12.92) * v)
    powed = f(max(v, 0.0) ** f(1.0 / 2.4))
    hi = f(f(f(1.055) * powed) - f(0.055))
    return lo if v < f(0.0031308) else hi


def exposure_effective_f32(lum_current, exposure, luminance_multiplier,
                           auto_exposure_scale):
    """tonemap.glsl L866-868 operation order:
    exposure *= 1.0 / (lum * luminance_multiplier / auto_exposure_scale)."""
    denominator = f(f(f(lum_current) * f(luminance_multiplier)) / f(auto_exposure_scale))
    return f(f(exposure) * f(1.0 / denominator))


def tonemap_texel(rgba, exposure_eff, luminance_multiplier):
    """Mirror of the fused segment B per texel: rgb *= luminance_multiplier;
    rgb *= exposure_effective; LINEAR identity; linear_to_srgb; alpha
    passthrough."""
    out = []
    for c in range(3):
        scaled = f(f(rgba[c] * f(luminance_multiplier)) * exposure_eff)
        out.append(linear_to_srgb_f32(scaled))
    out.append(rgba[3])
    return out


def fused_frame(width, height, color, lum_grid, lum_w, lum_h, prev, constants,
                first_frame):
    """One fused dispatch on the CPU: returns (ldr_grid, lum_out, cur, avg,
    exposure_eff)."""
    lum_out, cur, avg = luminance_segment(
        lum_grid, lum_w, lum_h, prev,
        constants["max_luminance"], constants["min_luminance"],
        constants["exposure_adjust"], first_frame)
    exposure_eff = exposure_effective_f32(
        lum_out, constants["exposure"], constants["luminance_multiplier"],
        constants["auto_exposure_scale"])
    ldr = []
    for y in range(height):
        row = []
        for x in range(width):
            row.append(tonemap_texel(color[y][x], exposure_eff,
                                     constants["luminance_multiplier"]))
        ldr.append(row)
    return ldr, lum_out, cur, avg, exposure_eff


# ── GRX-010 member tonemap reference (composition cross-check) ───────────────

def member_tonemap_texel(rgba, exposure, white, luminance_multiplier):
    """The GRX-010 tonemap member CPU formula
    (spike/godot-rurix/passes/tonemap/generate_math_parity_evidence.py):
    rgb = linear_to_srgb(rgb * luminance_multiplier * exposure); alpha
    passthrough; white unused for TONEMAPPER_LINEAR."""
    _ = white
    out = []
    for c in range(3):
        scaled = f(f(rgba[c] * f(luminance_multiplier)) * f(exposure))
        out.append(linear_to_srgb_f32(scaled))
    out.append(rgba[3])
    return out


# ── deterministic synthetic inputs (replicable bit-for-bit in a C++ smoke) ──

def syn_color(x, y):
    # The GRX-010 member input pattern (HDR range [0, 2]).
    return tuple(
        f(f(float((x * 29 + y * 13 + c * 7) % 101)) / f(50.0)) for c in range(4)
    )


def syn_lum(x, y):
    # Positive luminance texels in [0.05, ~1.55).
    base = f(f(float((x * 7 + y * 11) % 23)) / f(23.0))
    return f(f(0.05) + f(base * f(1.5)))


def build_color(width, height):
    return [[syn_color(x, y) for x in range(width)] for y in range(height)]


def build_lum(lum_w, lum_h, scale):
    return [[f(syn_lum(x, y) * f(scale)) for x in range(lum_w)] for y in range(lum_h)]


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


BASE_CONSTANTS = {
    "max_luminance": 1.2,
    "min_luminance": 0.05,
    "exposure_adjust": 0.4,
    "exposure": 1.0,
    "white": 1.0,
    "luminance_multiplier": 1.0,
    "auto_exposure_scale": 0.5,
}


def time_series_case(case_id, description, width, height, lum_w, lum_h,
                     prev0, lum_scale_per_frame, first_frame_flags,
                     constants=None, frames=8):
    """Chain `frames` fused dispatches: the segment-A output of frame k is the
    prev_luminance of frame k+1 (double-buffer mirror of the native SWAP); the
    static color scene isolates the EMA time series in the LDR output."""
    constants = dict(constants or BASE_CONSTANTS)
    color = build_color(width, height)
    prev = f(prev0)
    per_frame = []
    lum_trend = []
    for frame in range(frames):
        lum_grid = build_lum(lum_w, lum_h, lum_scale_per_frame[frame])
        first_frame = bool(first_frame_flags[frame])
        ldr, lum_out, cur, avg, exposure_eff = fused_frame(
            width, height, color, lum_grid, lum_w, lum_h, prev, constants,
            first_frame)
        samples = [{"x": px, "y": py, "rgba": ldr[py][px]}
                   for (px, py) in sample_points(width, height)]
        per_frame.append({
            "frame": frame,
            "first_frame": first_frame,
            "lum_source_scale": lum_scale_per_frame[frame],
            "prev_luminance": prev,
            "lum_avg": avg,
            "lum_clamped_current": cur,
            "lum_output": lum_out,
            "exposure_effective": exposure_eff,
            "abs_lum_output_minus_clamped_current": f(abs(lum_out - cur)),
            "ldr_output_sha256": flat_sha(ldr),
            "ldr_sample_texels": samples,
        })
        lum_trend.append(lum_out)
        prev = lum_out
    return {
        "case_id": case_id,
        "description": description,
        "width": width,
        "height": height,
        "lum_source_width": lum_w,
        "lum_source_height": lum_h,
        "frames": frames,
        "constants": constants,
        "prev_luminance_frame0": f(prev0),
        "lum_output_trend": lum_trend,
        "convergence_trend_abs_lum_minus_current": [
            pf["abs_lum_output_minus_clamped_current"] for pf in per_frame
        ],
        "per_frame": per_frame,
    }


DISPATCH_WIDTH = 16
DISPATCH_HEIGHT = 16
DISPATCH_LUM_W = 5
DISPATCH_LUM_H = 3
DISPATCH_PREV = 0.6


def dispatch_fixture_case():
    w, h = DISPATCH_WIDTH, DISPATCH_HEIGHT
    constants = dict(BASE_CONSTANTS)
    color = build_color(w, h)
    lum_grid = build_lum(DISPATCH_LUM_W, DISPATCH_LUM_H, 1.0)
    ldr, lum_out, cur, avg, exposure_eff = fused_frame(
        w, h, color, lum_grid, DISPATCH_LUM_W, DISPATCH_LUM_H,
        f(DISPATCH_PREV), constants, False)
    samples = [{"x": px, "y": py, "rgba": ldr[py][px]}
               for (px, py) in sample_points(w, h)]
    # Composition cross-check: the fused segment-B output must equal the
    # GRX-010 member tonemap reference evaluated with the fused effective
    # exposure (documents fused == composition of the member kernels).
    max_diff = 0.0
    for y in range(h):
        for x in range(w):
            member = member_tonemap_texel(
                color[y][x], exposure_eff, constants["white"],
                constants["luminance_multiplier"])
            for c in range(4):
                max_diff = max(max_diff, abs(ldr[y][x][c] - member[c]))
    return {
        "case_id": "fused_post_chain_dispatch_fixture_16x16",
        "description": ("single-frame fused dispatch fixture the (deferred S6) "
                        "standalone D3D12 smoke re-verifies on a real GPU"),
        "width": w,
        "height": h,
        "constants": {
            "source_width": w,
            "source_height": h,
            "lum_source_width": DISPATCH_LUM_W,
            "lum_source_height": DISPATCH_LUM_H,
            **constants,
            "first_frame": 0.0,
        },
        "input_patterns": {
            "color": "c(x,y,ch) = ((x*29 + y*13 + ch*7) % 101) / 50  (HDR [0,2], GRX-010 member pattern)",
            "lum_source": "l(x,y) = 0.05 + (((x*7 + y*11) % 23) / 23) * 1.5",
            "prev_luminance": "0.6 (single texel)",
        },
        "cpu_expected_lum_avg": avg,
        "cpu_expected_lum_clamped_current": cur,
        "cpu_expected_lum_output": lum_out,
        "cpu_expected_exposure_effective": exposure_eff,
        "cpu_expected_rgba_f32_le_sha256": flat_sha(ldr),
        "cpu_expected_sample_texels": samples,
        "composition_check": {
            "description": ("fused segment-B LDR output vs the GRX-010 member tonemap "
                            "reference fed with the fused effective exposure"),
            "max_abs_diff": max_diff,
            "equal": max_diff == 0.0,
        },
        "gpu_observed": None,
    }


def main() -> int:
    w, h = 8, 6
    lum_w, lum_h = 5, 3

    static = time_series_case(
        "static_scene_ema_convergence", (
            "static color scene + static lum_source, prev starts away from the "
            "clamped current: the EMA converges toward clamp(avg) over the frame "
            "sequence (convergence_trend decreases toward 0) and the LDR output "
            "tracks the exposure_effective trajectory"),
        w, h, lum_w, lum_h,
        prev0=0.85,
        lum_scale_per_frame=[1.0] * 8,
        first_frame_flags=[0] * 8,
    )
    step = time_series_case(
        "brightness_step_adaptation", (
            "lum_source scales x4 from frame 4 (scene brightens): the EMA adapts "
            "upward over the remaining frames and exposure_effective drops "
            "accordingly (auto-exposure dimming), all visible in the chained "
            "LDR sequence"),
        w, h, lum_w, lum_h,
        prev0=0.5,
        lum_scale_per_frame=[1.0, 1.0, 1.0, 1.0, 4.0, 4.0, 4.0, 4.0],
        first_frame_flags=[0] * 8,
    )
    clamp_case = time_series_case(
        "clamp_boundary_both_ends", (
            "frames 0-3 drive the raw tile mean far above max_luminance (clamp "
            "pins cur at the max), frames 4-7 far below min_luminance (clamp "
            "pins cur at the min): the EMA walks between the two clamp "
            "boundaries and the LDR output follows"),
        w, h, lum_w, lum_h,
        prev0=0.6,
        lum_scale_per_frame=[8.0, 8.0, 8.0, 8.0, 0.01, 0.01, 0.01, 0.01],
        first_frame_flags=[0] * 8,
    )
    first_frame_case = time_series_case(
        "first_frame_reset", (
            "frame 0 runs with first_frame=1: segment A outputs cur = "
            "clamp(avg) directly and the (deliberately nonsensical) prev value "
            "is ignored; frames 1-7 chain normally with first_frame=0 "
            "(abs_lum_output_minus_clamped_current is exactly 0 at frame 0)"),
        w, h, lum_w, lum_h,
        prev0=7.5,
        lum_scale_per_frame=[1.0] * 8,
        first_frame_flags=[1, 0, 0, 0, 0, 0, 0, 0],
    )
    dispatch = dispatch_fixture_case()

    evidence = {
        "pass_id": "fused_post_chain",
        "subject": "grx019_fused_post_chain_math_parity",
        "status": "pending_gpu_dispatch",
        "generated_at_utc": utc_now(),
        "math_status": "fused_post_chain_cpu_reference_proven_pending_gpu_dispatch",
        "cpu_reference": {
            "formula": (
                "ONE fused dispatch: segment A (luminance final WRITE_LUMINANCE level, "
                "member-kernel order) avg = partial-tile mean(lum_source <= 8x8); "
                "cur = clamp(avg, min_luminance, max_luminance); lum = first_frame ? cur : "
                "prev + (cur - prev) * exposure_adjust; segment B (tonemap LINEAR+sRGB) "
                "exposure_effective = exposure * (1 / (lum * luminance_multiplier / "
                "auto_exposure_scale)) per tonemap.glsl L866-868; rgb = linear_to_srgb("
                "rgb * luminance_multiplier * exposure_effective); alpha passthrough; "
                "every operation rounded to binary32"
            ),
            "hlsl_kernel": {"path": rel(HLSL_PATH), "sha256": sha256_of_file(HLSL_PATH)},
            "dxil": {"path": rel(DXIL_PATH), "sha256": sha256_of_file(DXIL_PATH)},
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "member_kernels": {
                "luminance_reduction": "spike/godot-rurix/passes/luminance_reduction/artifacts/hlsl_bridge/luminance_reduce_level.hlsl (-D RX_WRITE_LUMINANCE)",
                "tonemap": "spike/godot-rurix/passes/tonemap/artifacts/hlsl_bridge/tonemap_apply.hlsl",
            },
            "godot_math_sources": [
                "external/godot-master/servers/rendering/renderer_rd/shaders/effects/luminance_reduce.glsl (WRITE_LUMINANCE L76-79)",
                "external/godot-master/servers/rendering/renderer_rd/shaders/effects/tonemap.glsl (L860, L864-870, L866-868, L893, L230-233, L942-943)",
            ],
        },
        "max_abs_error_tolerance": MAX_ABS_ERROR_TOLERANCE,
        "gpu_dispatch_kind": None,
        "time_series_cases": [static, step, clamp_case, first_frame_case],
        "dispatch_fixture": dispatch,
        "notes": [
            "GPU-observed values are pending a real dispatch; the standalone smoke "
            "(deferred S6, ci/grx019_fused_post_chain_d3d12_dispatch_smoke.py) will "
            "independently verify every measured GPU output texel plus the 1x1 "
            "dst_luminance against fused_frame on the dispatch fixture.",
            "The four time-series cases are CPU-only demonstrations of the EMA time "
            "series x tonemap composite (>= 8 frames each, the segment-A output of "
            "frame k chained as the prev_luminance of frame k+1, mirroring the native "
            "double-buffer SWAP); only the single-frame dispatch fixture is reserved "
            "for GPU verification in a later slice.",
            "Segment A mirrors the tracked GRX-009 member kernel (clamp then EMA); the "
            "native WRITE_LUMINANCE order (EMA inside the clamp) and the native p_set "
            "raw-avg first frame are recorded known gaps, as is the lum_current == 0 "
            "divide (all fixtures use min_luminance > 0).",
            "The composition cross-check on the dispatch fixture verifies texel for "
            "texel that the fused segment-B output equals the GRX-010 member tonemap "
            "reference fed with the fused effective exposure.",
        ],
        "does_not_imply": [
            "Godot runtime fused post-chain pass completion",
            "real_gpu_pass=true",
            "real_d3d12_dispatch_recorded=true",
            "visual success",
            "GPU timestamp success",
            "dispatch/barrier/VRAM traffic reduction",
            "performance claim",
            "default pass enablement",
        ],
    }
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[grx019-math-parity] status=pending_gpu_dispatch time_series=4 dispatch_fixture=1 "
          f"composition_check_equal={dispatch['composition_check']['equal']} "
          f"evidence={EVIDENCE_PATH}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
