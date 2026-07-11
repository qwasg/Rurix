#!/usr/bin/env python3
"""GRX-009 stage A2: math parity evidence generator (CPU reference vs GPU).

Computes a CPU float32 reference for the luminance reduction math on
deterministic synthetic inputs and writes ``math_parity_evidence.json``
(schema: ``math_parity_evidence.schema.json``):

- ``base`` cases mirror ``src/lib_texture.rx`` level reduction: ceil-div 8x8
  destination extent, 8x8 tile accumulation in row-major (dy outer, dx inner)
  order, partial-tile-correct mean (divisor = valid pixel count).
- ``write_luminance`` cases mirror the final-level math (``src/lib.rx``
  clamp * exposure plus EMA): ``cur = clamp(avg, min, max)``,
  ``out = prev + (cur - prev) * exposure_adjust``.

GRX-009 Wave 2 additions (append-only; the four ``cases`` above are unchanged):

- ``pyramid_parity`` — the full multi-level reduce chain (spatial pyramid) for a
  256x144 source: level 0 reduces the input pattern, each later level reduces the
  previous level's output grid, ceil-div-8 cascade to 1x1 mirroring
  ``LuminanceReductionGate::plan_luminance_pyramid_levels``.
- ``ema_sequence`` — a >= 8 frame temporal EMA of the final 1x1 luminance,
  including the first-frame ``p_set`` (prev == 0) and clamp boundaries, modeling
  the WRITE_LUMINANCE kernel across frames.
- ``semantics`` — records the ``one_frame_latency`` declaration, the ``p_set``
  first-frame semantics, and where clamp applies (see ``hook_contract_v2.md``).

All arithmetic is rounded to IEEE-754 binary32 after every operation so the
expected values are bit-comparable with a D3D12 dispatch of the
``artifacts/hlsl_bridge/`` kernel (strict-IEEE fp; the comparison still uses
a small absolute tolerance to absorb legal fp contraction differences).

GPU side: this is a stub. When a GPU dispatch results document exists at
``RURIX_GRX009_GPU_RESULTS`` (JSON: ``{"dispatch_kind": ..., "cases":
{case_id: {"dst_f32_le_sha256": ..., "sample_texels": [{"x","y","value"}]}}}``),
it is compared case by case. Otherwise the evidence honestly records
``status=pending_gpu_dispatch`` with ``gpu_observed=null`` per case. This
evidence never implies real_gpu_pass=true or canonical artifact replacement.
"""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import pathlib
import struct
import sys


PASS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = PASS_DIR.parents[3]
BRIDGE_DIR = PASS_DIR / "artifacts" / "hlsl_bridge"
HLSL_PATH = BRIDGE_DIR / "luminance_reduce_level.hlsl"
DXIL_BASE_PATH = BRIDGE_DIR / "luminance_reduce_level.dxil"
DXIL_WRITE_LUMINANCE_PATH = BRIDGE_DIR / "luminance_reduce_level_write_luminance.dxil"
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

MAX_ABS_ERROR_TOLERANCE = 1e-6
INPUT_PATTERN = "f32(((x * 31 + y * 17) % 97)) / f32(97)"

# GRX-009 Wave 2 multi-level pyramid + EMA reference parameters.
# A 256x144 source cascades ceil-div-8 as 256x144 -> 32x18 -> 4x3 -> 1x1
# (three dispatches), mirroring plan_luminance_pyramid_levels. Kept modest so
# the per-op-f32 CPU reference stays fast.
PYRAMID_SOURCE = (256, 144)
# >= 8 frames of per-frame source brightness scales chosen so the clamp bites at
# both bounds. With the 8x8 pattern mean ~0.498, scales 2.5/3.0 exceed max=1.2
# (max bite) and scales 0.05/0.2 fall below min=0.1 (min bite).
EMA_SCALES = [0.2, 3.0, 1.0, 0.05, 2.5, 1.0, 0.5, 0.8]
EMA_CONSTANTS = {"max_luminance": 1.2, "min_luminance": 0.1, "exposure_adjust": 0.4}

DOES_NOT_IMPLY = [
    "Godot runtime luminance pass completion",
    "real_gpu_pass=true",
    "real_d3d12_dispatch_recorded=true",
    "visual success",
    "GPU timestamp success",
    "performance claim",
    "canonical luminance artifact replacement",
]


def rel(path: pathlib.Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def utc_now() -> str:
    return dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def sha256_of_file(path: pathlib.Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def hashed_path(path: pathlib.Path) -> dict[str, object]:
    return {"path": rel(path), "sha256": sha256_of_file(path), "exists": path.is_file()}


def f32(value: float) -> float:
    """Round to IEEE-754 binary32 (matches per-op GPU float math)."""
    return struct.unpack("<f", struct.pack("<f", value))[0]


def input_texel(x: int, y: int) -> float:
    return f32(f32((x * 31 + y * 17) % 97) / f32(97.0))


def ceil_div8(dim: int) -> int:
    return (dim + 7) // 8 if dim > 1 else 1


def reduce_level_cpu(source_width: int, source_height: int) -> list[list[float]]:
    """Mirror of src/lib_texture.rx (and the base HLSL variant), f32 per op."""
    dst_width = ceil_div8(source_width)
    dst_height = ceil_div8(source_height)
    dst = [[0.0] * dst_width for _ in range(dst_height)]
    for y in range(dst_height):
        for x in range(dst_width):
            src_x = x * 8
            src_y = y * 8
            accum = 0.0
            count = 0.0
            for dy in range(8):
                sy = src_y + dy
                if sy < source_height:
                    for dx in range(8):
                        sx = src_x + dx
                        if sx < source_width:
                            accum = f32(accum + input_texel(sx, sy))
                            count = f32(count + 1.0)
            dst[y][x] = f32(accum / count) if count > 0.0 else 0.0
    return dst


def final_level_cpu(
    source_width: int,
    source_height: int,
    max_luminance: float,
    min_luminance: float,
    exposure_adjust: float,
    prev_luminance: float,
) -> list[list[float]]:
    """Mirror of the RX_WRITE_LUMINANCE HLSL variant: clamp then EMA."""
    dst = reduce_level_cpu(source_width, source_height)
    out = []
    for row in dst:
        out_row = []
        for avg in row:
            cur = min(max(avg, min_luminance), max_luminance)
            out_row.append(f32(prev_luminance + f32(f32(cur - prev_luminance) * exposure_adjust)))
        out.append(out_row)
    return out


def reduce_grid_cpu(src: list[list[float]]) -> list[list[float]]:
    """8x8 tile mean of an arbitrary source grid (f32 per op). Same math as
    reduce_level_cpu but over a provided grid instead of the input pattern, so it
    can be chained level to level for the multi-level pyramid."""
    source_height = len(src)
    source_width = len(src[0])
    dst_width = ceil_div8(source_width)
    dst_height = ceil_div8(source_height)
    dst = [[0.0] * dst_width for _ in range(dst_height)]
    for y in range(dst_height):
        for x in range(dst_width):
            src_x = x * 8
            src_y = y * 8
            accum = 0.0
            count = 0.0
            for dy in range(8):
                sy = src_y + dy
                if sy < source_height:
                    for dx in range(8):
                        sx = src_x + dx
                        if sx < source_width:
                            accum = f32(accum + src[sy][sx])
                            count = f32(count + 1.0)
            dst[y][x] = f32(accum / count) if count > 0.0 else 0.0
    return dst


def source_grid(width: int, height: int, scale: float = 1.0) -> list[list[float]]:
    """Build a source grid from the synthetic input pattern, optionally scaled
    (per-frame brightness for the EMA sequence)."""
    scale = f32(scale)
    return [[f32(input_texel(x, y) * scale) for x in range(width)] for y in range(height)]


def plan_levels(source_width: int, source_height: int) -> list[tuple[int, int, int, int, bool]]:
    """Mirror LuminanceReductionGate::plan_luminance_pyramid_levels: ceil-div-8
    cascade until 1x1. Returns (src_w, src_h, dst_w, dst_h, is_final) per level."""
    levels: list[tuple[int, int, int, int, bool]] = []
    w, h = max(source_width, 1), max(source_height, 1)
    while True:
        dst_width = ceil_div8(w)
        dst_height = ceil_div8(h)
        is_final = dst_width == 1 and dst_height == 1
        levels.append((w, h, dst_width, dst_height, is_final))
        if is_final:
            break
        w, h = dst_width, dst_height
    return levels


def pyramid_chain_cpu(source_width: int, source_height: int) -> list[dict[str, object]]:
    """Full multi-level reduce chain: level 0 reduces the input pattern, each
    later level reduces the previous level's output grid. The final level's grid
    is the raw 1x1 reduced luminance (pre-EMA); the temporal EMA is modeled
    separately in ema_sequence."""
    plan = plan_levels(source_width, source_height)
    cur = source_grid(source_width, source_height, 1.0)
    out: list[dict[str, object]] = []
    for i, (sw, sh, dw, dh, is_final) in enumerate(plan):
        dst = reduce_grid_cpu(cur)
        out.append(
            {
                "level_index": i,
                "variant": "write_luminance" if is_final else "base",
                "is_final": is_final,
                "src_width": sw,
                "src_height": sh,
                "dst_width": dw,
                "dst_height": dh,
                "cpu_expected": {
                    "dst_f32_le_sha256": grid_sha256(dst),
                    "sample_texels": sample_texels(dst),
                },
                "gpu_observed": None,
                "case_status": "pending",
            }
        )
        cur = dst
    return out


def reduce_to_final_cpu(grid: list[list[float]]) -> float:
    """Reduce a grid all the way to a single 1x1 value (chained 8x8 tile means)."""
    reduced = grid
    while len(reduced) > 1 or len(reduced[0]) > 1:
        reduced = reduce_grid_cpu(reduced)
    return reduced[0][0]


def ema_sequence_cpu(
    source_width: int,
    source_height: int,
    max_luminance: float,
    min_luminance: float,
    exposure_adjust: float,
    scales: list[float],
) -> list[dict[str, object]]:
    """>= 8 frame temporal EMA of the final 1x1 luminance, modeling the
    WRITE_LUMINANCE KERNEL: the first frame has prev == 0 (zero-cleared,
    Godot's p_set) so out = cur * adjust; later frames use out = prev +
    (cur - prev) * adjust with prev == the previous frame's output."""
    frames: list[dict[str, object]] = []
    prev = 0.0
    for f, scale in enumerate(scales):
        grid = source_grid(source_width, source_height, scale)
        raw_avg = reduce_to_final_cpu(grid)
        clamped = f32(min(max(raw_avg, min_luminance), max_luminance))
        p_set = f == 0
        prev_used = 0.0 if p_set else prev
        current_out = f32(prev_used + f32(f32(clamped - prev_used) * exposure_adjust))
        frames.append(
            {
                "frame_index": f,
                "p_set": p_set,
                "source_scale": f32(scale),
                "raw_avg": raw_avg,
                "clamped_cur": clamped,
                "clamp_active": bool(clamped != raw_avg),
                "prev_luminance": prev_used,
                "current_out": current_out,
            }
        )
        prev = current_out
    return frames


def grid_sha256(dst: list[list[float]]) -> str:
    payload = b"".join(struct.pack("<f", value) for row in dst for value in row)
    return hashlib.sha256(payload).hexdigest()


def sample_texels(dst: list[list[float]]) -> list[dict[str, object]]:
    height = len(dst)
    width = len(dst[0])
    coords = {(0, 0), (width - 1, 0), (0, height - 1), (width - 1, height - 1), (width // 2, height // 2)}
    return [{"x": x, "y": y, "value": dst[y][x]} for (x, y) in sorted(coords)]


def build_case(
    case_id: str,
    variant: str,
    source_width: int,
    source_height: int,
    input_pattern: str,
    constants: dict[str, object],
    dst: list[list[float]],
) -> dict[str, object]:
    return {
        "case_id": case_id,
        "variant": variant,
        "source_width": source_width,
        "source_height": source_height,
        "dst_width": len(dst[0]),
        "dst_height": len(dst),
        "input_pattern": input_pattern,
        "constants": constants,
        "cpu_expected": {
            "dst_f32_le_sha256": grid_sha256(dst),
            "sample_texels": sample_texels(dst),
        },
        "gpu_observed": None,
        "case_status": "pending",
    }


def build_cases() -> list[dict[str, object]]:
    cases: list[dict[str, object]] = []
    # base: exact tiles (16x16 -> 2x2), partial tiles (20x13 -> 3x2), and the
    # degenerate 1x1 -> 1x1 (exercises the `dim > 1` ceil-div branch).
    for case_id, w, h in (
        ("base_16x16_exact_tiles", 16, 16),
        ("base_20x13_partial_tiles", 20, 13),
        ("base_1x1_degenerate", 1, 1),
    ):
        constants = {
            "source_width": w,
            "source_height": h,
            "max_luminance": 8.0,
            "min_luminance": 0.0,
            "exposure_adjust": 1.0,
            "prev_luminance": None,
        }
        cases.append(build_case(case_id, "base", w, h, INPUT_PATTERN, constants, reduce_level_cpu(w, h)))
    # write_luminance: 8x8 -> 1x1 final level. Constants chosen so clamp and
    # EMA both bite: min_luminance above the raw tile mean forces the clamp,
    # exposure_adjust 0.5 with a nonzero prev forces a real EMA blend.
    w, h = 8, 8
    max_luminance, min_luminance, exposure_adjust, prev = 2.0, 0.6, 0.5, 0.25
    constants = {
        "source_width": w,
        "source_height": h,
        "max_luminance": max_luminance,
        "min_luminance": min_luminance,
        "exposure_adjust": exposure_adjust,
        "prev_luminance": prev,
    }
    dst = final_level_cpu(w, h, max_luminance, min_luminance, exposure_adjust, prev)
    cases.append(build_case("write_luminance_8x8_clamp_ema", "write_luminance", w, h, INPUT_PATTERN, constants, dst))
    return cases


def apply_gpu_results(cases: list[dict[str, object]], gpu_doc: dict[str, object]) -> tuple[bool, float | None]:
    """Compare GPU per-case results against the CPU reference. Returns
    (all_within_tolerance, max_abs_error_observed)."""
    gpu_cases = gpu_doc.get("cases")
    if not isinstance(gpu_cases, dict):
        raise SystemExit("GPU results document missing `cases` object")
    all_ok = True
    max_err: float | None = None
    for case in cases:
        observed = gpu_cases.get(case["case_id"])
        if not isinstance(observed, dict):
            case["case_status"] = "pending"
            all_ok = False
            continue
        case["gpu_observed"] = observed
        expected = case["cpu_expected"]
        hash_match = observed.get("dst_f32_le_sha256") == expected["dst_f32_le_sha256"]
        case_err = 0.0
        observed_samples = {
            (t["x"], t["y"]): t["value"]
            for t in observed.get("sample_texels", [])
            if isinstance(t, dict)
        }
        for texel in expected["sample_texels"]:
            got = observed_samples.get((texel["x"], texel["y"]))
            if got is None:
                case_err = float("inf")
                break
            case_err = max(case_err, abs(got - texel["value"]))
        within = hash_match or case_err <= MAX_ABS_ERROR_TOLERANCE
        case["case_status"] = "match" if within else "mismatch"
        case["gpu_observed"]["max_abs_error"] = None if case_err == float("inf") else case_err
        max_err = case_err if max_err is None else max(max_err, case_err)
        all_ok = all_ok and within
    return all_ok, max_err


def main() -> int:
    cases = build_cases()
    gpu_results_env = os.environ.get("RURIX_GRX009_GPU_RESULTS")
    gpu_results_path = pathlib.Path(gpu_results_env).expanduser() if gpu_results_env else None
    measured_gpu = gpu_results_path is not None and gpu_results_path.is_file()

    evidence: dict[str, object] = {
        "schema_version": 1,
        "subject": "grx009_stage_a2_hlsl_bridge_math_parity",
        "pass_id": "luminance_reduction",
        "status": "pending_gpu_dispatch",
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "measured_gpu": measured_gpu,
        "real_gpu_pass": False,
        "canonical_artifact_replaced": False,
        "generated_at_utc": utc_now(),
        "kernel": {
            "hlsl": hashed_path(HLSL_PATH),
            "dxil_base": hashed_path(DXIL_BASE_PATH),
            "dxil_write_luminance": hashed_path(DXIL_WRITE_LUMINANCE_PATH),
        },
        "cpu_reference": {
            "implementation": rel(pathlib.Path(__file__)),
            "math_source": {
                "level_reduction": rel(PASS_DIR / "src" / "lib_texture.rx"),
                "final_level": rel(PASS_DIR / "src" / "lib.rx")
                + " (clamp * exposure) + EMA prev + (cur - prev) * exposure_adjust",
            },
            "float_model": "IEEE-754 binary32 rounded after every operation (struct pack/unpack round-trip)",
        },
        "gpu_results": None,
        "comparison": {
            "max_abs_error_tolerance": MAX_ABS_ERROR_TOLERANCE,
            "measured": measured_gpu,
            "max_abs_error_observed": None,
            "all_cases_within_tolerance": None,
        },
        "cases": cases,
        "pyramid_parity": {
            "description": (
                "GRX-009 Wave 2 full multi-level reduce chain (spatial pyramid): "
                "level 0 reduces the input pattern, each later level reduces the "
                "previous level's output grid, ceil-div-8 cascade to 1x1 mirroring "
                "LuminanceReductionGate::plan_luminance_pyramid_levels. Each level "
                "grid is the partial-tile-correct 8x8 tile mean; the final level "
                "grid is the raw pre-EMA 1x1 luminance (the temporal EMA is modeled "
                "in ema_sequence). GPU side is pending a real multi-level dispatch."
            ),
            "source_width": PYRAMID_SOURCE[0],
            "source_height": PYRAMID_SOURCE[1],
            "input_pattern": INPUT_PATTERN,
            "levels": pyramid_chain_cpu(*PYRAMID_SOURCE),
        },
        "ema_sequence": {
            "description": (
                "GRX-009 Wave 2 temporal EMA of the final 1x1 luminance over "
                f"{len(EMA_SCALES)} frames including the first-frame p_set. Models "
                "the WRITE_LUMINANCE kernel out = prev + (cur - prev) * "
                "exposure_adjust with cur = clamp(avg, min, max); the first frame "
                "uses prev == 0 (zero-cleared, Godot p_set) so out = cur * adjust. "
                "Frame scales are chosen so the clamp bites at both bounds. GPU "
                "side is pending a real multi-frame dispatch."
            ),
            "source_width": 8,
            "source_height": 8,
            "input_pattern_base": INPUT_PATTERN,
            "constants": EMA_CONSTANTS,
            "frames": ema_sequence_cpu(
                8,
                8,
                EMA_CONSTANTS["max_luminance"],
                EMA_CONSTANTS["min_luminance"],
                EMA_CONSTANTS["exposure_adjust"],
                EMA_SCALES,
            ),
        },
        "semantics": {
            "one_frame_latency": {
                "declared": True,
                "description": (
                    "When the Godot runtime hook records these dispatches from "
                    "within a frame, Godot has not yet submitted that frame's "
                    "rendering to the queue, so a self-queue dispatch reading the "
                    "internal_texture (source) reads the PREVIOUS frame's content, "
                    "and prev is the previous frame's 1x1 luminance. The luminance "
                    "pass uses time-domain EMA feedback, so a 1-frame delay is "
                    "engineering-defensible; it is recorded here, not hidden. A "
                    "later enablement smoke may fingerprint the dispatch input in "
                    "test-only readback mode to demonstrate the ordering."
                ),
                "reference": "hook_contract_v2.md",
            },
            "p_set_first_frame": {
                "description": (
                    "Mirrors Godot's p_set: on the first frame there is no previous "
                    "luminance, so no EMA blend is applied. The fixed WRITE_LUMINANCE "
                    "kernel expresses this via a zero-cleared prev texture, for which "
                    "prev + (cur - prev) * adjust degenerates to cur * adjust. Native "
                    "p_set == false writes clamp(avg) directly (no exposure factor); "
                    "the fixed kernel differs by the exposure factor on the first "
                    "frame only, a bounded documented divergence. ema_sequence models "
                    "the kernel behavior."
                ),
            },
            "clamp": {
                "description": (
                    "clamp(avg, min_luminance, max_luminance) is applied only at the "
                    "final WRITE_LUMINANCE level (per Godot), before the EMA. Reduce "
                    "levels write the raw 8x8 tile mean with no clamp/exposure."
                ),
            },
        },
        "does_not_imply": DOES_NOT_IMPLY,
    }

    if measured_gpu:
        gpu_doc = json.loads(gpu_results_path.read_text(encoding="utf-8"))
        all_ok, max_err = apply_gpu_results(cases, gpu_doc)
        evidence["gpu_results"] = {
            "path": rel(gpu_results_path),
            "sha256": sha256_of_file(gpu_results_path),
            "dispatch_kind": gpu_doc.get("dispatch_kind", "unknown"),
        }
        evidence["comparison"]["max_abs_error_observed"] = max_err
        evidence["comparison"]["all_cases_within_tolerance"] = all_ok
        evidence["status"] = "success" if all_ok else "fail"
    else:
        evidence["pending_reason"] = (
            "no GPU dispatch results document available (RURIX_GRX009_GPU_RESULTS unset or missing); "
            "CPU float32 reference recorded, GPU side pending"
        )
        evidence["next_action_when_gpu_available"] = (
            "dispatch artifacts/hlsl_bridge/ DXIL variants against the recorded synthetic inputs, "
            "write a GPU results document, and re-run generate_math_parity_evidence.py with "
            "RURIX_GRX009_GPU_RESULTS pointing at it"
        )

    EVIDENCE_PATH.parent.mkdir(parents=True, exist_ok=True)
    with EVIDENCE_PATH.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(evidence, indent=2, ensure_ascii=True) + "\n")
    print(
        f"[grx009-math-parity] status={evidence['status']} measured_gpu={measured_gpu} "
        f"cases={len(cases)} evidence={EVIDENCE_PATH}"
    )
    return 0 if evidence["status"] != "fail" else 1


if __name__ == "__main__":
    sys.exit(main())
