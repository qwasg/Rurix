#!/usr/bin/env python3
"""GRX-010: tonemap math parity evidence generator (CPU reference vs GPU).

Computes a CPU float32 reference for the tonemap math subset
(TONEMAPPER_LINEAR + linear_to_srgb; see PASS_CONTRACT.md §5) on
deterministic synthetic RGBA inputs and writes ``math_parity_evidence.json``.

All arithmetic is rounded to IEEE-754 binary32 after every operation so the
expected values are bit-comparable with a D3D12 dispatch of the
``artifacts/hlsl_bridge/tonemap_apply.hlsl`` kernel (the comparison still
uses a small absolute tolerance to absorb legal fp contraction and
pow-approximation differences).

GPU side: honest stub. Without a GPU results document the evidence records
``status=pending_gpu_dispatch`` with ``gpu_observed=null`` per case. The
standalone dispatch smoke (ci/grx010_tonemap_d3d12_dispatch_smoke.py)
separately verifies one measured GPU texel against this same CPU formula.
This evidence never implies real_gpu_pass=true or default enablement.
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
HLSL_PATH = BRIDGE_DIR / "tonemap_apply.hlsl"
DXIL_PATH = BRIDGE_DIR / "tonemap_apply.dxil"
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

MAX_ABS_ERROR_TOLERANCE = 2e-3  # absorbs GPU pow() approximation differences
INPUT_PATTERN = "f32(((x * 29 + y * 13 + c * 7) % 101)) / f32(50)  # HDR range [0, 2]"

DOES_NOT_IMPLY = [
    "Godot runtime tonemap pass completion",
    "real_gpu_pass=true",
    "real_d3d12_dispatch_recorded=true",
    "visual success",
    "GPU timestamp success",
    "performance claim",
    "default pass enablement",
]


def f32(value: float) -> float:
    return struct.unpack("<f", struct.pack("<f", value))[0]


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


def input_texel(x: int, y: int, c: int) -> float:
    return f32(f32(float((x * 29 + y * 13 + c * 7) % 101)) / f32(50.0))


def linear_to_srgb_f32(value: float) -> float:
    # tonemap.glsl L230-233, componentwise, binary32-rounded per op.
    v = f32(value)
    if v < 0.0:
        v = 0.0
    if v < f32(0.0031308):
        return f32(f32(12.92) * v)
    powed = f32(max(v, 0.0) ** f32(1.0 / 2.4))
    return f32(f32(f32(1.055) * powed) - f32(0.055))


def tonemap_texel(x: int, y: int, exposure: float, white: float, lum_mult: float) -> list[float]:
    out: list[float] = []
    for c in range(4):
        value = input_texel(x, y, c)
        if c < 3:
            scaled = f32(f32(value * f32(lum_mult)) * f32(exposure))
            out.append(linear_to_srgb_f32(scaled))
        else:
            out.append(value)  # alpha passthrough
    _ = white  # unused by TONEMAPPER_LINEAR (kept for shape parity)
    return out


def build_case(case_id: str, width: int, height: int, exposure: float, white: float, lum_mult: float) -> dict[str, object]:
    expected: list[list[float]] = []
    for y in range(height):
        row: list[float] = []
        for x in range(width):
            row.extend(tonemap_texel(x, y, exposure, white, lum_mult))
        expected.append(row)
    flat = struct.pack(f"<{width * height * 4}f", *[v for row in expected for v in row])
    return {
        "case_id": case_id,
        "width": width,
        "height": height,
        "constants": {
            "source_width": width,
            "source_height": height,
            "exposure": exposure,
            "white": white,
            "luminance_multiplier": lum_mult,
        },
        "input_pattern": INPUT_PATTERN,
        "cpu_expected_rgba_f32_le_sha256": hashlib.sha256(flat).hexdigest(),
        "cpu_expected_sample_texels": [
            {"x": 0, "y": 0, "rgba": expected[0][0:4]},
            {"x": width - 1, "y": height - 1, "rgba": expected[height - 1][(width - 1) * 4:(width - 1) * 4 + 4]},
        ],
        "gpu_observed": None,
    }


def main() -> int:
    cases = [
        build_case("tonemap_8x8_exposure1", 8, 8, 1.0, 1.0, 1.0),
        build_case("tonemap_8x8_exposure_half", 8, 8, 0.5, 1.0, 1.0),
        build_case("tonemap_16x9_lum_mult2", 16, 9, 1.0, 4.0, 2.0),
        build_case("tonemap_9x7_partial_tiles", 9, 7, 1.25, 1.0, 0.75),
    ]
    evidence = {
        "pass_id": "tonemap",
        "subject": "grx010_tonemap_math_parity",
        "status": "pending_gpu_dispatch",
        "generated_at_utc": utc_now(),
        "math_status": "linear_srgb_cpu_reference_proven_pending_gpu_dispatch",
        "cpu_reference": {
            "formula": (
                "rgb = linear_to_srgb(src.rgb * luminance_multiplier * exposure); "
                "alpha passthrough; linear_to_srgb per tonemap.glsl L230-233 "
                "(a=0.055, threshold 0.0031308); every operation rounded to binary32"
            ),
            "hlsl_kernel": {"path": rel(HLSL_PATH), "sha256": sha256_of_file(HLSL_PATH)},
            "dxil": {"path": rel(DXIL_PATH), "sha256": sha256_of_file(DXIL_PATH)},
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_math_source": "external/godot-master/servers/rendering/renderer_rd/shaders/effects/tonemap.glsl (L860, L870, L247-249, L230-233, L942-943)",
        },
        "max_abs_error_tolerance": MAX_ABS_ERROR_TOLERANCE,
        "gpu_dispatch_kind": None,
        "cases": cases,
        "notes": [
            "GPU-observed values are pending a real dispatch; the standalone smoke "
            "ci/grx010_tonemap_d3d12_dispatch_smoke.py independently verifies one "
            "measured GPU texel against the same CPU formula.",
            "white is intentionally unused by TONEMAPPER_LINEAR and carried only for "
            "push-constant shape parity.",
        ],
        "does_not_imply": DOES_NOT_IMPLY,
    }
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[grx010-math-parity] status=pending_gpu_dispatch cases={len(cases)} evidence={EVIDENCE_PATH}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
