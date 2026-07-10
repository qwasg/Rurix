#!/usr/bin/env python3
"""GRX-011: ssao_blur math parity evidence generator (CPU reference vs GPU).

Computes a CPU float32 reference for the ssao_blur math subset (MODE_SMART
edge-aware cross blur, single pass, single slice; see PASS_CONTRACT.md §5)
on deterministic synthetic (value, packed_edges) inputs and writes
``math_parity_evidence.json``.

All arithmetic is rounded to IEEE-754 binary32 after every operation so the
expected values are bit-comparable with a D3D12 dispatch of the
``artifacts/hlsl_bridge/ssao_blur_smart.hlsl`` kernel (the comparison still
uses a small absolute tolerance to absorb legal fp contraction).

GPU side: honest stub. Without a GPU results document the evidence records
``status=pending_gpu_dispatch`` with ``gpu_observed=null`` per case. The
standalone dispatch smoke (ci/grx011_ssao_blur_d3d12_dispatch_smoke.py)
separately verifies every measured GPU texel against this same CPU formula.
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
HLSL_PATH = BRIDGE_DIR / "ssao_blur_smart.hlsl"
DXIL_PATH = BRIDGE_DIR / "ssao_blur_smart.dxil"
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

MAX_ABS_ERROR_TOLERANCE = 1e-5  # pure add/mul/div chain; no pow approximation
VALUE_PATTERN = "f32(((x * 29 + y * 13) % 101)) / f32(100)  # ssao value in [0, 1]"
EDGES_PATTERN = "f32((x * 7 + y * 3 + 11) % 256) / f32(255)  # packed LRTB edges byte as unorm"

DOES_NOT_IMPLY = [
    "Godot runtime SSAO blur pass completion",
    "real_gpu_pass=true",
    "real_d3d12_dispatch_recorded=true",
    "visual success",
    "temporal stability success",
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


def input_value(x: int, y: int) -> float:
    return f32(f32(float((x * 29 + y * 13) % 101)) / f32(100.0))


def input_edges(x: int, y: int) -> float:
    return f32(f32(float((x * 7 + y * 3 + 11) % 256)) / f32(255.0))


def unpack_edges(packed_val_f: float, edge_sharpness: float) -> list[float]:
    # ssao_blur.glsl L39-48, binary32-rounded per op.
    packed_val = int(f32(packed_val_f) * f32(255.5))
    edges = []
    for shift in (6, 4, 2, 0):
        e = f32(f32(float((packed_val >> shift) & 0x03)) / f32(3.0))
        e = f32(e + f32(edge_sharpness))
        if e < 0.0:
            e = 0.0
        if e > 1.0:
            e = 1.0
        edges.append(e)
    return edges  # [L, R, T, B]


def blur_texel(x: int, y: int, width: int, height: int, edge_sharpness: float) -> list[float]:
    # Clamp border addressing (interior texels match Godot's gather).
    xl = max(x - 1, 0)
    xr = min(x + 1, width - 1)
    yt = max(y - 1, 0)
    yb = min(y + 1, height - 1)

    center = input_value(x, y)
    packed_edges = input_edges(x, y)
    value_l = input_value(xl, y)
    value_r = input_value(xr, y)
    value_t = input_value(x, yt)
    value_b = input_value(x, yb)

    edge_l, edge_r, edge_t, edge_b = unpack_edges(packed_edges, edge_sharpness)

    # sample_blurred (ssao_blur.glsl L95-122): center weight 0.5, then
    # add_sample for L, R, T, B in that order, binary32-rounded per op.
    sum_value = f32(center * f32(0.5))
    sum_weight = f32(0.5)
    for value, edge in ((value_l, edge_l), (value_r, edge_r), (value_t, edge_t), (value_b, edge_b)):
        sum_value = f32(sum_value + f32(edge * value))
        sum_weight = f32(sum_weight + edge)
    avg = f32(sum_value / sum_weight)
    return [avg, packed_edges, 0.0, 0.0]


def build_case(case_id: str, width: int, height: int, edge_sharpness: float) -> dict[str, object]:
    expected: list[list[float]] = []
    for y in range(height):
        row: list[float] = []
        for x in range(width):
            row.extend(blur_texel(x, y, width, height, edge_sharpness))
        expected.append(row)
    flat = struct.pack(f"<{width * height * 4}f", *[v for row in expected for v in row])
    return {
        "case_id": case_id,
        "width": width,
        "height": height,
        "constants": {
            "source_width": width,
            "source_height": height,
            "edge_sharpness": edge_sharpness,
            "half_screen_pixel_size_x": f32(1.0 / width),
            "half_screen_pixel_size_y": f32(1.0 / height),
        },
        "input_pattern": {"value": VALUE_PATTERN, "packed_edges": EDGES_PATTERN},
        "cpu_expected_rgba_f32_le_sha256": hashlib.sha256(flat).hexdigest(),
        "cpu_expected_sample_texels": [
            {"x": 0, "y": 0, "rgba": expected[0][0:4]},
            {"x": width // 2, "y": height // 2, "rgba": expected[height // 2][(width // 2) * 4:(width // 2) * 4 + 4]},
            {"x": width - 1, "y": height - 1, "rgba": expected[height - 1][(width - 1) * 4:(width - 1) * 4 + 4]},
        ],
        "gpu_observed": None,
    }


def main() -> int:
    cases = [
        build_case("ssao_blur_8x8_sharp0", 8, 8, 0.0),
        build_case("ssao_blur_8x8_sharp02", 8, 8, 0.02),
        build_case("ssao_blur_16x9_sharp0", 16, 9, 0.0),
        build_case("ssao_blur_9x7_partial_tiles", 9, 7, 0.05),
    ]
    evidence = {
        "pass_id": "ssao_blur",
        "subject": "grx011_ssao_blur_math_parity",
        "status": "pending_gpu_dispatch",
        "generated_at_utc": utc_now(),
        "math_status": "smart_blur_cpu_reference_proven_pending_gpu_dispatch",
        "cpu_reference": {
            "formula": (
                "edges = clamp(unpack_edges(center.y) + edge_sharpness, 0, 1); "
                "sum = 0.5*center.x + edges.L*L + edges.R*R + edges.T*T + edges.B*B; "
                "w = 0.5 + edges.L + edges.R + edges.T + edges.B; "
                "dst = (sum/w, center.y, 0, 0); unpack_edges per ssao_blur.glsl L39-48 "
                "(packed byte -> 4x 2-bit LRTB / 3.0); clamp border addressing; "
                "every operation rounded to binary32"
            ),
            "hlsl_kernel": {"path": rel(HLSL_PATH), "sha256": sha256_of_file(HLSL_PATH)},
            "dxil": {"path": rel(DXIL_PATH), "sha256": sha256_of_file(DXIL_PATH)},
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_math_source": "external/godot-master/servers/rendering/renderer_rd/shaders/effects/ssao_blur.glsl (L39-48, L50-55, L95-122, L153)",
        },
        "max_abs_error_tolerance": MAX_ABS_ERROR_TOLERANCE,
        "gpu_dispatch_kind": None,
        "cases": cases,
        "notes": [
            "GPU-observed values are pending a real dispatch; the standalone smoke "
            "ci/grx011_ssao_blur_d3d12_dispatch_smoke.py independently verifies every "
            "measured GPU texel against the same CPU formula.",
            "half_screen_pixel_size_x/y are intentionally unused by the Load-addressed "
            "kernel and carried only for SSAOBlurPushConstant shape parity.",
            "Interior texels are texel-exact vs Godot's half-pixel gather addressing; "
            "border texels use clamp addressing (mirror-sampler difference is a "
            "recorded gap).",
        ],
        "does_not_imply": DOES_NOT_IMPLY,
    }
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[grx011-math-parity] status=pending_gpu_dispatch cases={len(cases)} evidence={EVIDENCE_PATH}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
