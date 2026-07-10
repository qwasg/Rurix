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
