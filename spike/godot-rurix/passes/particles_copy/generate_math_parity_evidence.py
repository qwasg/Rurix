#!/usr/bin/env python3
"""GRX-013: particles_copy math parity evidence generator (CPU reference vs GPU).

Computes a CPU float32 reference for the particles_copy math subset
(COPY_MODE_FILL_INSTANCES, 3D, ALIGN_DISABLED + ALIGN_BILLBOARD, no trail, no
sort, no userdata; see PASS_CONTRACT.md sec 5) on deterministic synthetic
ParticleData and writes ``math_parity_evidence.json``.

All arithmetic is rounded to IEEE-754 binary32 after every operation so the
expected values are comparable with a D3D12 dispatch of the
``artifacts/hlsl_bridge/particles_copy_fill_instances.hlsl`` kernel. Because the
ALIGN_BILLBOARD path uses sin/cos and normalize (rsqrt), whose GPU
approximations differ from libm, the comparison uses a small absolute tolerance
(the ALIGN_DISABLED path is near-exact; the wider tolerance covers the
transcendental / rsqrt cases).

GPU side: honest stub. Without a GPU results document the evidence records
``status=pending_gpu_dispatch`` with ``gpu_observed=null`` per case. A later
standalone dispatch smoke (S6, host-exclusive) will fill the GPU-observed side
by verifying every measured instance against this same CPU formula. This
evidence never implies real_gpu_pass=true or default enablement.
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
HLSL_PATH = BRIDGE_DIR / "particles_copy_fill_instances.hlsl"
DXIL_PATH = BRIDGE_DIR / "particles_copy_fill_instances.dxil"
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

# DISABLED path is near-exact (add/mul only); BILLBOARD carries sin/cos/rsqrt so
# the GPU-vs-libm tolerance is wider.
MAX_ABS_ERROR_TOLERANCE = 3.0e-4

NEG_INF = struct.unpack("<f", struct.pack("<I", 0xFF800000))[0]

PARTICLE_FLAG_ACTIVE = 1
ALIGN_DISABLED = 0
ALIGN_BILLBOARD = 1

INPUT_PATTERN = {
    "xform_c0": "[1.0 + 0.01*p, 0.02*p, 0.0, 0.0]",
    "xform_c1": "[0.03*p, 1.0 - 0.01*p, 0.01*p, 0.0]",
    "xform_c2": "[0.0, 0.02*p, 1.0, 0.0]",
    "xform_c3": "[2.0*p, -1.0*p, 0.5*p, 1.0]  (translation column)",
    "velocity": "[0.5 - 0.1*p, 0.2*p, -0.3*p]",
    "flags": "PARTICLE_FLAG_ACTIVE(1) if p % 3 != 2 else 0 (inactive)",
    "color": "[0.125*p, 1.0 - 0.125*p, 0.25, 1.0]",
    "custom": "[0.7*p, 0.1*p, -0.2*p, 0.05*p]  (billboard angle channel source)",
    "note": "every value rounded to binary32; c*.w of the basis columns never reaches the 3D output (row 3 is not written)",
}

DOES_NOT_IMPLY = [
    "Godot runtime particles_copy pass completion",
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


# ---- synthetic ParticleData (binary32 per component) --------------------------

def xform_columns(p: int) -> list[list[float]]:
    c0 = [f32(1.0 + 0.01 * p), f32(0.02 * p), f32(0.0), f32(0.0)]
    c1 = [f32(0.03 * p), f32(1.0 - 0.01 * p), f32(0.01 * p), f32(0.0)]
    c2 = [f32(0.0), f32(0.02 * p), f32(1.0), f32(0.0)]
    c3 = [f32(2.0 * p), f32(-1.0 * p), f32(0.5 * p), f32(1.0)]
    return [c0, c1, c2, c3]


def velocity(p: int) -> list[float]:
    return [f32(0.5 - 0.1 * p), f32(0.2 * p), f32(-0.3 * p)]


def flags(p: int) -> int:
    return PARTICLE_FLAG_ACTIVE if (p % 3 != 2) else 0


def color(p: int) -> list[float]:
    return [f32(0.125 * p), f32(1.0 - 0.125 * p), f32(0.25), f32(1.0)]


def custom(p: int) -> list[float]:
    return [f32(0.7 * p), f32(0.1 * p), f32(-0.2 * p), f32(0.05 * p)]


# ---- vector helpers (binary32 per op) -----------------------------------------

def v_dot(a: list[float], b: list[float]) -> float:
    return f32(f32(f32(a[0] * b[0]) + f32(a[1] * b[1])) + f32(a[2] * b[2]))


def v_normalize(a: list[float]) -> list[float]:
    # HLSL normalize == v * rsqrt(dot(v,v)); CPU uses 1/sqrt rounded to f32.
    inv = f32(1.0 / f32(math.sqrt(v_dot(a, a))))
    return [f32(a[0] * inv), f32(a[1] * inv), f32(a[2] * inv)]


def v_cross(a: list[float], b: list[float]) -> list[float]:
    return [
        f32(f32(a[1] * b[2]) - f32(a[2] * b[1])),
        f32(f32(a[2] * b[0]) - f32(a[0] * b[2])),
        f32(f32(a[0] * b[1]) - f32(a[1] * b[0])),
    ]


def v_madd3(l0: list[float], sx: float, l1: list[float], sy: float, l2: list[float], sz: float) -> list[float]:
    # l0*sx + l1*sy + l2*sz, binary32 per op, matching the HLSL column-major
    # mat3*vec accumulation order.
    out = []
    for i in range(3):
        acc = f32(l0[i] * sx)
        acc = f32(acc + f32(l1[i] * sy))
        acc = f32(acc + f32(l2[i] * sz))
        out.append(acc)
    return out


# ---- per-instance reference ---------------------------------------------------

def transform_instance(p: int, consts: dict[str, object]) -> list[list[float]]:
    cols = xform_columns(p)
    c0, c1, c2, c3 = cols[0], cols[1], cols[2], cols[3]
    vel = velocity(p)
    align_mode = int(consts["align_mode"])
    align_channel_filter = int(consts["align_channel_filter"])
    sort_direction = [f32(x) for x in consts["sort_direction"]]  # type: ignore[index]
    align_up = [f32(x) for x in consts["align_up"]]              # type: ignore[index]
    frame_remainder = f32(consts["frame_remainder"])            # type: ignore[arg-type]

    active = (flags(p) & PARTICLE_FLAG_ACTIVE) != 0 or (flags(p) & 4) != 0

    if active:
        if align_mode == ALIGN_BILLBOARD:
            cst = custom(p)
            angle = 0.0
            if align_channel_filter == 1:
                angle = cst[0]
            elif align_channel_filter == 2:
                angle = cst[1]
            elif align_channel_filter == 3:
                angle = cst[2]
            elif align_channel_filter == 4:
                angle = cst[3]

            axis = v_normalize(sort_direction)
            s = f32(math.sin(angle))
            cN = f32(math.cos(angle))
            oc = f32(1.0 - cN)
            ax, ay, az = axis[0], axis[1], axis[2]

            rc0 = [
                f32(f32(f32(oc * ax) * ax) + cN),
                f32(f32(f32(oc * ax) * ay) - f32(az * s)),
                f32(f32(f32(oc * az) * ax) + f32(ay * s)),
            ]
            rc1 = [
                f32(f32(f32(oc * ax) * ay) + f32(az * s)),
                f32(f32(f32(oc * ay) * ay) + cN),
                f32(f32(f32(oc * ay) * az) - f32(ax * s)),
            ]
            rc2 = [
                f32(f32(f32(oc * az) * ax) - f32(ay * s)),
                f32(f32(f32(oc * ay) * az) + f32(ax * s)),
                f32(f32(f32(oc * az) * az) + cN),
            ]

            new_up = v_madd3(rc0, align_up[0], rc1, align_up[1], rc2, align_up[2])
            L0 = v_normalize(v_cross(new_up, sort_direction))
            L1 = new_up
            L2 = sort_direction

            nc0 = v_madd3(L0, c0[0], L1, c0[1], L2, c0[2])
            nc1 = v_madd3(L0, c1[0], L1, c1[1], L2, c1[2])
            nc2 = v_madd3(L0, c2[0], L1, c2[1], L2, c2[2])
            c0 = [nc0[0], nc0[1], nc0[2], c0[3]]
            c1 = [nc1[0], nc1[1], nc1[2], c1[3]]
            c2 = [nc2[0], nc2[1], nc2[2], c2[3]]
        # ALIGN_DISABLED: basis unchanged.

        c3 = [
            f32(c3[0] + f32(vel[0] * frame_remainder)),
            f32(c3[1] + f32(vel[1] * frame_remainder)),
            f32(c3[2] + f32(vel[2] * frame_remainder)),
            c3[3],
        ]
    else:
        c0 = [0.0, 0.0, 0.0, 0.0]
        c1 = [0.0, 0.0, 0.0, 0.0]
        c2 = [0.0, 0.0, 0.0, 0.0]
        c3 = [NEG_INF, NEG_INF, NEG_INF, 0.0]

    # transpose rows 0..2
    r0 = [c0[0], c1[0], c2[0], c3[0]]
    r1 = [c0[1], c1[1], c2[1], c3[1]]
    r2 = [c0[2], c1[2], c2[2], c3[2]]
    return [r0, r1, r2, color(p), custom(p)]  # 5 vec4


def build_case(case_id: str, n: int, consts: dict[str, object]) -> dict[str, object]:
    instances: list[list[list[float]]] = [transform_instance(p, consts) for p in range(n)]
    flat: list[float] = [v for inst in instances for vec in inst for v in vec]
    packed = struct.pack(f"<{len(flat)}f", *flat)
    sample_ids = sorted(set([0, n // 2, n - 1]))
    samples = []
    for p in sample_ids:
        samples.append({
            "instance": p,
            "active": bool((flags(p) & PARTICLE_FLAG_ACTIVE) != 0 or (flags(p) & 4) != 0),
            "rows_xform": instances[p][0:3],
            "color": instances[p][3],
            "custom": instances[p][4],
        })
    return {
        "case_id": case_id,
        "particle_count": n,
        "instance_count": n,
        "instance_stride_vec4": 5,
        "constants": {
            "total_particles": n,
            "align_mode": int(consts["align_mode"]),
            "align_mode_name": "ALIGN_BILLBOARD" if int(consts["align_mode"]) == ALIGN_BILLBOARD else "ALIGN_DISABLED",
            "align_channel_filter": int(consts["align_channel_filter"]),
            "sort_direction": [f32(x) for x in consts["sort_direction"]],  # type: ignore[index]
            "align_up": [f32(x) for x in consts["align_up"]],              # type: ignore[index]
            "frame_remainder": f32(consts["frame_remainder"]),            # type: ignore[arg-type]
            "trail_size": 1,
            "motion_vectors_current_offset": 0,
            "flags_bits": 0,
        },
        "cpu_expected_instances_f32_le_sha256": hashlib.sha256(packed).hexdigest(),
        "cpu_expected_sample_instances": samples,
        "gpu_observed": None,
    }


def main() -> int:
    sort_a = [0.6, 0.0, 0.8]        # unit
    sort_b = [0.48, 0.64, 0.6]      # unit
    up_y = [0.0, 1.0, 0.0]

    cases = [
        build_case("fill_instances_align_disabled", 8, {
            "align_mode": ALIGN_DISABLED, "align_channel_filter": 0,
            "sort_direction": sort_a, "align_up": up_y, "frame_remainder": 0.5,
        }),
        build_case("fill_instances_billboard_channel_x", 8, {
            "align_mode": ALIGN_BILLBOARD, "align_channel_filter": 1,
            "sort_direction": sort_a, "align_up": up_y, "frame_remainder": 0.0,
        }),
        build_case("fill_instances_billboard_channel_w", 6, {
            "align_mode": ALIGN_BILLBOARD, "align_channel_filter": 4,
            "sort_direction": sort_b, "align_up": up_y, "frame_remainder": 0.25,
        }),
        build_case("fill_instances_billboard_channel_none", 5, {
            "align_mode": ALIGN_BILLBOARD, "align_channel_filter": 0,
            "sort_direction": sort_a, "align_up": up_y, "frame_remainder": 0.0,
        }),
    ]
    evidence = {
        "pass_id": "particles_copy",
        "subject": "grx013_particles_copy_math_parity",
        "status": "pending_gpu_dispatch",
        "generated_at_utc": utc_now(),
        "math_status": "fill_instances_cpu_reference_proven_pending_gpu_dispatch",
        "cpu_reference": {
            "formula": (
                "active = (flags & ACTIVE) || (flags & TRAILED); "
                "ALIGN_DISABLED: txform = xform; ALIGN_BILLBOARD: axis=normalize(sort_direction), "
                "rotated=Rodrigues(axis, custom[channel]); new_up=rotated*align_up; "
                "local=mat3(normalize(cross(new_up, sort_direction)), new_up, sort_direction)*mat3(txform); "
                "then txform[3].xyz += velocity*frame_remainder. inactive: txform basis=0, "
                "translation column=(-inf,-inf,-inf,0). write (3D, transposed): "
                "instances[i*5+0..2]=transpose(txform) rows 0..2; [i*5+3]=color; [i*5+4]=custom; "
                "column-major mat algebra, every operation rounded to binary32"
            ),
            "hlsl_kernel": {"path": rel(HLSL_PATH), "sha256": sha256_of_file(HLSL_PATH)},
            "dxil": {"path": rel(DXIL_PATH), "sha256": sha256_of_file(DXIL_PATH)},
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_math_source": "external/godot-master/servers/rendering/renderer_rd/shaders/particles_copy.glsl (L109-347, MODE_FILL_INSTANCES)",
        },
        "input_pattern": INPUT_PATTERN,
        "max_abs_error_tolerance": MAX_ABS_ERROR_TOLERANCE,
        "tolerance_note": (
            "ALIGN_DISABLED instances are near-exact (add/mul only, ~1 ULP). "
            "ALIGN_BILLBOARD instances carry sin/cos and normalize (rsqrt), whose "
            "GPU approximations differ from libm; the tolerance covers that."
        ),
        "gpu_dispatch_kind": None,
        "cases": cases,
        "notes": [
            "GPU-observed values are pending a real dispatch; a later standalone smoke "
            "(S6, host-exclusive) will verify every measured instance against this CPU formula.",
            "Out-of-scope push-constant fields (inv_emission_transform, trail_*, lifetime_*, "
            "align_axis) are carried for CopyPushConstant shape parity and set to their neutral "
            "values (trail_size=1, motion_vectors_current_offset=0, flags_bits=0).",
            "The mat4 is column-major (Godot txform[i] = column i); all matrix algebra mirrors "
            "the GLSL column-major order, and the 3D output writes only transpose rows 0..2, so "
            "the basis columns' .w never reaches the output.",
        ],
        "does_not_imply": DOES_NOT_IMPLY,
    }
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[grx013-math-parity] status=pending_gpu_dispatch cases={len(cases)} evidence={EVIDENCE_PATH}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
