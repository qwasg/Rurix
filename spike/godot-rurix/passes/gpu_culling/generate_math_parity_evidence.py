#!/usr/bin/env python3
"""GRX-015: gpu_culling math parity evidence generator (CPU reference vs GPU).

Computes a CPU reference for the count-only conservative frustum-cull kernel
(see PASS_CONTRACT.md sec 5) on deterministic synthetic transform / command /
frustum fixtures and writes ``math_parity_evidence.json``.

The kernel's OUTPUTS are pure u32 (visibility bitmask words + per-surface
instance counts + untouched command dwords), so the GPU comparison is
**zero tolerance** (exact word equality). The float intermediates (world
center, Frobenius-norm radius, plane distances) feed ONLY comparisons; to make
the zero-tolerance claim honest, this generator mirrors the HLSL arithmetic
binary32 per op AND **asserts a classification-margin floor**: for every
instance x plane, ``|dist + world_radius| >= 1e-3`` — so ULP-level GPU
reassociation / FMA / sqrt differences (~1e-7 relative) cannot flip any
per-plane comparison, and therefore cannot change any u32 output word.
Generation FAILS if the margin or any required branch coverage degenerates.

The three cases are the contract-required set: all-visible / all-culled /
boundary-mixed (spheres crossing frustum planes stay visible), and together
they also exercise: a tail bitmask word (N % 32 != 0), a multi-group dispatch
(N > 64), multi-surface command blocks, every one of the 6 planes as a culling
plane, rotated and non-uniform-scale bases, and untouched non-count command
dwords (nonzero sentinels; the native values are zeros — sentinel values are
a fixture-hardening choice to catch a clobbering kernel).

GPU side: honest stub. Without a GPU results document the evidence records
``status=pending_gpu_dispatch`` with ``gpu_observed=null`` per case. The
standalone dispatch smoke (S6, a later slice) imports THIS module's fixtures +
reference and verifies every measured output word exactly. This evidence never
implies real_gpu_pass=true or default enablement.
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
HLSL_PATH = BRIDGE_DIR / "gpu_culling_frustum_count.hlsl"
DXIL_PATH = BRIDGE_DIR / "gpu_culling_frustum_count.dxil"
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

MATH_PARITY_STATUS = "gpu_culling_cpu_reference_proven_pending_gpu_dispatch"

TRANSFORM_STRIDE_FLOATS = 12   # bare 3D (known gap: 16/20 stride variants)
COMMAND_STRIDE_DWORDS = 5      # mirrors INDIRECT_MULTIMESH_COMMAND_STRIDE
INSTANCE_COUNT_DWORD_INDEX = 1  # mirrors mesh_storage.cpp L2210 (+sizeof(u32))
THREADGROUP_X = 64

# Local bounding sphere (host precompute from the mesh local AABB).
BOUND_CENTER_LOCAL = (0.25, -0.5, 0.125)
BOUND_RADIUS_LOCAL = 0.75

# Every per-plane comparison must sit at least this far from the
# dist == -world_radius threshold (asserted; makes zero tolerance honest).
MARGIN_FLOOR = 1.0e-3

DOES_NOT_IMPLY = [
    "Godot runtime gpu_culling pass completion",
    "real_gpu_pass=true",
    "manifest real_d3d12_dispatch_recorded=true",
    "visual success",
    "temporal stability success",
    "GPU timestamp success",
    "draw/dispatch-count reduction claim",
    "performance claim",
    "default pass enablement",
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


def f32(value: float) -> float:
    return struct.unpack("<f", struct.pack("<f", value))[0]


# ---- basis variants (all entries exact in binary32) -------------------------

BASIS_KINDS = ("identity", "roty_scaled", "diag_scaled")


def basis_rows(kind: str) -> list[list[float]]:
    """3x3 basis rows. roty_scaled = rotY(cos 0.6, sin 0.8) * diag(0.5, 2, 1)
    (rotated + non-uniform); diag_scaled = diag(2, 0.25, 0.5) (non-uniform)."""
    if kind == "identity":
        return [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    if kind == "roty_scaled":
        return [[0.3, 0.0, 0.8], [0.0, 2.0, 0.0], [-0.4, 0.0, 0.6]]
    if kind == "diag_scaled":
        return [[2.0, 0.0, 0.0], [0.0, 0.25, 0.0], [0.0, 0.0, 0.5]]
    raise ValueError(kind)


# ---- fixture cases -----------------------------------------------------------


def box_planes(half_extent: float) -> list[tuple[float, float, float, float]]:
    """Six normalized inward-facing planes of the axis-aligned box
    [-half_extent, half_extent]^3 (dist = dot(n, p) + d >= 0 inside)."""
    d = float(half_extent)
    return [
        (1.0, 0.0, 0.0, d),   # plane 0: x >= -d face
        (-1.0, 0.0, 0.0, d),  # plane 1: x <= +d face
        (0.0, 1.0, 0.0, d),   # plane 2: y >= -d face
        (0.0, -1.0, 0.0, d),  # plane 3: y <= +d face
        (0.0, 0.0, 1.0, d),   # plane 4: z >= -d face
        (0.0, 0.0, -1.0, d),  # plane 5: z <= +d face
    ]


def small_coord(i: int, mul: int, half: float) -> float:
    """Deterministic small coordinate in [-half, half]."""
    return f32((((i * mul) % 11) - 5) * (half / 5.0))


def build_instances(case_id: str, count: int) -> list[dict]:
    """Deterministic per-instance records: basis kind + origin + intent tag."""
    instances: list[dict] = []
    if case_id == "cull_all_visible_n40":
        # Everything well inside the [-100, 100]^3 box.
        for i in range(count):
            instances.append({
                "kind": BASIS_KINDS[i % 3],
                "origin": (f32(-30.0 + 1.5 * i), f32(10.0 - 0.75 * i), f32(0.5 * i - 12.0)),
                "intent": "inside",
            })
    elif case_id == "cull_all_culled_n33":
        # Every instance fully beyond one of the six [-10, 10]^3 faces
        # (face = i % 6 -> each plane culls >= 5 instances).
        for i in range(count):
            face = i % 6
            a = small_coord(i, 7, 2.5)
            b = small_coord(i, 3, 2.0)
            far = 40.0
            origin = {
                0: (-far, a, b),
                1: (far, a, b),
                2: (a, -far, b),
                3: (a, far, b),
                4: (a, b, -far),
                5: (a, b, far),
            }[face]
            instances.append({
                "kind": BASIS_KINDS[i % 3],
                "origin": tuple(f32(v) for v in origin),
                "intent": f"culled_face_{face}",
            })
    elif case_id == "cull_boundary_mixed_n96":
        # [-20, 20]^3 box; i % 4 == 0 inside / 1 crossing +x / 2 culled / 3
        # crossing -z with a rotated non-uniform basis.
        for i in range(count):
            lane = i % 4
            if lane == 0:
                instances.append({
                    "kind": BASIS_KINDS[(i // 4) % 3],
                    "origin": (
                        f32(((i * 5) % 17) - 8.0),
                        f32(((i * 3) % 15) - 7.0),
                        f32(((i * 11) % 13) - 6.0),
                    ),
                    "intent": "inside",
                })
            elif lane == 1:
                # Sphere straddles the x = +20 face: world center x = 20.75,
                # dist(plane 1) = -0.75, world_radius(identity) ~= 1.299 ->
                # visible, crossing.
                instances.append({
                    "kind": "identity",
                    "origin": (f32(20.5), small_coord(i, 7, 5.0), small_coord(i, 3, 4.0)),
                    "intent": "crossing_plane_1",
                })
            elif lane == 2:
                face = (i // 4) % 6
                a = small_coord(i, 7, 2.5)
                b = small_coord(i, 3, 2.0)
                far = 60.0
                origin = {
                    0: (-far, a, b),
                    1: (far, a, b),
                    2: (a, -far, b),
                    3: (a, far, b),
                    4: (a, b, -far),
                    5: (a, b, far),
                }[face]
                instances.append({
                    "kind": BASIS_KINDS[(i // 4) % 3],
                    "origin": tuple(f32(v) for v in origin),
                    "intent": f"culled_face_{face}",
                })
            else:
                # Sphere straddles the z = -20 face with the rotated
                # non-uniform basis: world z ~= -20.625, dist(plane 4) =
                # -0.625, world_radius ~= 1.718 -> visible, crossing.
                instances.append({
                    "kind": "roty_scaled",
                    "origin": (small_coord(i, 5, 5.0), small_coord(i, 9, 3.0), f32(-20.6)),
                    "intent": "crossing_plane_4",
                })
    else:
        raise ValueError(case_id)
    assert len(instances) == count
    return instances


def parity_cases() -> list[dict]:
    return [
        {
            "case_id": "cull_all_visible_n40",
            "instance_count": 40,
            "surface_count": 1,
            "planes": box_planes(100.0),
            "note": "all instances well inside a [-100,100]^3 box frustum; N=40 exercises a tail bitmask word (word 1 uses 8 bits)",
        },
        {
            "case_id": "cull_all_culled_n33",
            "instance_count": 33,
            "surface_count": 2,
            "planes": box_planes(10.0),
            "note": "every instance fully beyond one of the six faces (face = i % 6, all 6 planes cull); counts must stay 0 and every non-count command dword (incl. sentinels) must survive untouched",
        },
        {
            "case_id": "cull_boundary_mixed_n96",
            "instance_count": 96,
            "surface_count": 3,
            "planes": box_planes(20.0),
            "note": "mixed inside / plane-crossing (visible) / fully-culled; N=96 needs 2 thread groups (ceil(96/64)); 3 surfaces exercise the per-surface count replication",
        },
    ]


def pack_transforms(instances: list[dict]) -> tuple[bytes, list[float]]:
    """12 row-major 3x4 lanes per instance (mesh_storage.cpp L1880-1915)."""
    lanes: list[float] = []
    for record in instances:
        rows = basis_rows(record["kind"])
        ox, oy, oz = record["origin"]
        lanes += [rows[0][0], rows[0][1], rows[0][2], ox]
        lanes += [rows[1][0], rows[1][1], rows[1][2], oy]
        lanes += [rows[2][0], rows[2][1], rows[2][2], oz]
    lanes = [f32(v) for v in lanes]
    return struct.pack(f"<{len(lanes)}f", *lanes), lanes


def initial_command_words(surface_count: int) -> list[int]:
    """5-dword command block per surface. Dword 0 = a deterministic
    vertices-drawn count (the native _multimesh_set_mesh init); dword 1 = 0
    (the count dword, zeroed pre-dispatch); dwords 2-4 carry nonzero sentinels
    (native values are ZERO — sentinels are fixture hardening: a kernel that
    clobbers any non-count dword fails the exact comparison)."""
    words: list[int] = []
    for s in range(surface_count):
        words += [36 * (s + 1), 0, 1000 + s, 2000 + s, 3000 + s]
    return words


def build_b0(consts: dict) -> bytes:
    planes = consts["planes"]
    flat_planes = [f32(v) for plane in planes for v in plane]
    return struct.pack(
        "<24f6I4f2I",
        *flat_planes,                          # dwords 0-23
        consts["instance_count"],              # dword 24
        consts["motion_vectors_current_offset"],  # dword 25
        consts["transform_stride_floats"],     # dword 26
        consts["surface_count"],               # dword 27
        consts["command_stride_dwords"],       # dword 28
        consts["instance_count_dword_index"],  # dword 29
        f32(BOUND_CENTER_LOCAL[0]),            # dword 30
        f32(BOUND_CENTER_LOCAL[1]),            # dword 31
        f32(BOUND_CENTER_LOCAL[2]),            # dword 32
        f32(BOUND_RADIUS_LOCAL),               # dword 33
        0,                                     # dword 34 pad1
        0,                                     # dword 35 pad2
    )


# ---- CPU reference (binary32 per op, mirroring the HLSL exactly) ------------


def instance_classification(lanes: list[float], instance: int, planes: list) -> dict:
    """Mirror of the HLSL per-instance math: world center (rows * (c,1)),
    Frobenius-norm conservative radius, 6 plane distances. Returns the
    classification plus the per-plane margins |dist + world_radius|."""
    base = instance * TRANSFORM_STRIDE_FLOATS
    r = lanes[base:base + 12]
    r0, r1, r2 = r[0:4], r[4:8], r[8:12]
    cx, cy, cz = (f32(v) for v in BOUND_CENTER_LOCAL)

    def world(row: list[float]) -> float:
        # ((row.x*cx + row.y*cy) + row.z*cz) + row.w, binary32 per op.
        acc = f32(f32(row[0] * cx) + f32(row[1] * cy))
        acc = f32(acc + f32(row[2] * cz))
        return f32(acc + row[3])

    wx, wy, wz = world(r0), world(r1), world(r2)

    # Left-to-right Frobenius accumulation, binary32 per op.
    squares = [r0[0], r0[1], r0[2], r1[0], r1[1], r1[2], r2[0], r2[1], r2[2]]
    fro2 = f32(squares[0] * squares[0])
    for v in squares[1:]:
        fro2 = f32(fro2 + f32(v * v))
    world_radius = f32(f32(BOUND_RADIUS_LOCAL) * f32(math.sqrt(fro2)))

    dists: list[float] = []
    margins: list[float] = []
    culled_planes: list[int] = []
    visible = True
    for p, (nx, ny, nz, d) in enumerate(planes):
        nx, ny, nz, d = f32(nx), f32(ny), f32(nz), f32(d)
        acc = f32(f32(nx * wx) + f32(ny * wy))
        acc = f32(acc + f32(nz * wz))
        dist = f32(acc + d)
        dists.append(dist)
        margins.append(abs(dist + world_radius))
        if dist < -world_radius:
            visible = False
            culled_planes.append(p)
    return {
        "world_center": [wx, wy, wz],
        "world_radius": world_radius,
        "dists": dists,
        "margins": margins,
        "visible": visible,
        "culled_planes": culled_planes,
        "min_dist": min(dists),
    }


def gpu_culling_reference(consts: dict, lanes: list[float], instances: list[dict]) -> dict:
    n = consts["instance_count"]
    surface_count = consts["surface_count"]
    bitmask = [0] * ((n + 31) // 32)
    commands = list(initial_command_words(surface_count))
    coverage = {
        "fully_inside_instances": 0,
        "crossing_instances": 0,
        "culled_instances": 0,
        "identity_basis_instances": 0,
        "rotated_basis_instances": 0,
        "nonuniform_scale_instances": 0,
    }
    for p in range(6):
        coverage[f"culled_by_plane_{p}"] = 0
    visible_count = 0
    min_margin = None
    diag = []
    for i in range(n):
        cls = instance_classification(lanes, i, consts["planes"])
        case_min = min(cls["margins"])
        min_margin = case_min if min_margin is None else min(min_margin, case_min)
        kind = instances[i]["kind"]
        if kind == "identity":
            coverage["identity_basis_instances"] += 1
        if kind == "roty_scaled":
            coverage["rotated_basis_instances"] += 1
            coverage["nonuniform_scale_instances"] += 1
        if kind == "diag_scaled":
            coverage["nonuniform_scale_instances"] += 1
        if cls["visible"]:
            visible_count += 1
            bitmask[i >> 5] |= 1 << (i & 31)
            if cls["min_dist"] >= 0.0:
                coverage["fully_inside_instances"] += 1
            else:
                coverage["crossing_instances"] += 1
        else:
            coverage["culled_instances"] += 1
            for p in cls["culled_planes"]:
                coverage[f"culled_by_plane_{p}"] += 1
        if i in (0, n // 2, n - 1):
            diag.append({
                "instance": i,
                "basis_kind": kind,
                "intent": instances[i]["intent"],
                "world_center": cls["world_center"],
                "world_radius": cls["world_radius"],
                "min_dist": cls["min_dist"],
                "min_margin": case_min,
                "visible": cls["visible"],
            })
    for s in range(surface_count):
        commands[s * COMMAND_STRIDE_DWORDS + INSTANCE_COUNT_DWORD_INDEX] += visible_count
    # Tail bits past instance_count - 1 must stay zero by construction.
    if n % 32 != 0:
        tail = bitmask[-1] >> (n % 32)
        assert tail == 0, "tail bitmask bits must stay zero"
    return {
        "visible_count": visible_count,
        "bitmask_words": bitmask,
        "expected_command_words": commands,
        "coverage": coverage,
        "min_classification_margin": min_margin,
        "sample_instances": diag,
    }


REQUIRED_COVERAGE_KEYS = (
    "fully_inside_instances",
    "crossing_instances",
    "culled_instances",
    "identity_basis_instances",
    "rotated_basis_instances",
    "nonuniform_scale_instances",
    "culled_by_plane_0",
    "culled_by_plane_1",
    "culled_by_plane_2",
    "culled_by_plane_3",
    "culled_by_plane_4",
    "culled_by_plane_5",
)


def build_case_doc(case: dict) -> tuple[dict, dict, float]:
    n = case["instance_count"]
    consts = {
        "instance_count": n,
        "motion_vectors_current_offset": 0,
        "transform_stride_floats": TRANSFORM_STRIDE_FLOATS,
        "surface_count": case["surface_count"],
        "command_stride_dwords": COMMAND_STRIDE_DWORDS,
        "instance_count_dword_index": INSTANCE_COUNT_DWORD_INDEX,
        "planes": case["planes"],
    }
    instances = build_instances(case["case_id"], n)
    transform_bytes, lanes = pack_transforms(instances)
    result = gpu_culling_reference(consts, lanes, instances)
    initial_commands = initial_command_words(case["surface_count"])
    initial_command_bytes = struct.pack(f"<{len(initial_commands)}I", *initial_commands)
    expected_command_bytes = struct.pack(
        f"<{len(result['expected_command_words'])}I", *result["expected_command_words"]
    )
    bitmask_bytes = struct.pack(f"<{len(result['bitmask_words'])}I", *result["bitmask_words"])
    doc = {
        "case_id": case["case_id"],
        "note": case["note"],
        "constants": {
            "instance_count": n,
            "motion_vectors_current_offset": 0,
            "transform_stride_floats": TRANSFORM_STRIDE_FLOATS,
            "surface_count": case["surface_count"],
            "command_stride_dwords": COMMAND_STRIDE_DWORDS,
            "instance_count_dword_index": INSTANCE_COUNT_DWORD_INDEX,
            "frustum_planes": [[f32(v) for v in plane] for plane in case["planes"]],
            "mesh_bound_center_local": [f32(v) for v in BOUND_CENTER_LOCAL],
            "mesh_bound_radius_local": f32(BOUND_RADIUS_LOCAL),
        },
        "dispatch": [(n + THREADGROUP_X - 1) // THREADGROUP_X, 1, 1],
        "input_transforms_f32_le_sha256": hashlib.sha256(transform_bytes).hexdigest(),
        "input_initial_command_words": initial_commands,
        "input_initial_commands_u32_le_sha256": hashlib.sha256(initial_command_bytes).hexdigest(),
        "b0_sha256": hashlib.sha256(build_b0(consts)).hexdigest(),
        "visibility_word_count": len(result["bitmask_words"]),
        "cpu_expected_visible_count": result["visible_count"],
        "cpu_expected_bitmask_words_hex": [f"0x{w:08X}" for w in result["bitmask_words"]],
        "cpu_expected_bitmask_u32_le_sha256": hashlib.sha256(bitmask_bytes).hexdigest(),
        "cpu_expected_command_words": result["expected_command_words"],
        "cpu_expected_commands_u32_le_sha256": hashlib.sha256(expected_command_bytes).hexdigest(),
        "min_classification_margin": result["min_classification_margin"],
        "branch_coverage": result["coverage"],
        "cpu_expected_sample_instances": result["sample_instances"],
        "gpu_observed": None,
    }
    return doc, result["coverage"], result["min_classification_margin"]


def main() -> int:
    cases = []
    total_coverage: dict[str, int] = {}
    global_min_margin = None
    for case in parity_cases():
        doc, coverage, min_margin = build_case_doc(case)
        cases.append(doc)
        for key, value in coverage.items():
            total_coverage[key] = total_coverage.get(key, 0) + value
        global_min_margin = (
            min_margin if global_min_margin is None else min(global_min_margin, min_margin)
        )
    missing = [key for key in REQUIRED_COVERAGE_KEYS if total_coverage.get(key, 0) == 0]
    if missing:
        raise SystemExit(
            f"fixture degenerated: required branch coverage is zero for {missing}"
        )
    if global_min_margin is None or global_min_margin < MARGIN_FLOOR:
        raise SystemExit(
            "fixture degenerated: classification margin "
            f"{global_min_margin} below floor {MARGIN_FLOOR} — zero-tolerance "
            "u32 comparison would not be robust against GPU float ULP differences"
        )
    # Structural expectations of the contract-required case set.
    assert cases[0]["cpu_expected_visible_count"] == cases[0]["constants"]["instance_count"], \
        "case 1 must be all-visible"
    assert cases[1]["cpu_expected_visible_count"] == 0, "case 2 must be all-culled"
    n3 = cases[2]["constants"]["instance_count"]
    assert 0 < cases[2]["cpu_expected_visible_count"] < n3, "case 3 must be mixed"
    assert any(c["constants"]["instance_count"] % 32 != 0 for c in cases), "need a tail bitmask word"
    assert any(c["constants"]["instance_count"] > THREADGROUP_X for c in cases), "need a multi-group dispatch"
    assert any(c["constants"]["surface_count"] > 1 for c in cases), "need a multi-surface case"

    evidence = {
        "pass_id": "gpu_culling",
        "subject": "grx015_gpu_culling_math_parity",
        "status": "pending_gpu_dispatch",
        "generated_at_utc": utc_now(),
        "math_status": MATH_PARITY_STATUS,
        "cpu_reference": {
            "formula": (
                "per instance i: base = (motion_vectors_current_offset + i) * "
                "transform_stride_floats; rows r0/r1/r2 = 12 row-major 3x4 f32 "
                "lanes; world_center = rows * (mesh_bound_center_local, 1); "
                "world_radius = mesh_bound_radius_local * sqrt(sum of 9 "
                "squared basis entries) (Frobenius >= spectral norm: "
                "conservative, never over-culls); for each of 6 normalized "
                "inward-facing planes: dist = dot(n, world_center) + d; "
                "culled iff any dist < -world_radius; visible -> set bit "
                "(i & 31) of visibility word (i >> 5) (InterlockedOr) and add "
                "+1 to EACH surface's command dword s * command_stride_dwords "
                "+ instance_count_dword_index (InterlockedAdd; the dword the "
                "CPU writes at mesh_storage.cpp L2210); count dwords and "
                "bitmask assumed zeroed pre-dispatch; all other command "
                "dwords untouched. Float math binary32 per op; u32 outputs "
                "compared exactly"
            ),
            "hlsl_kernel": {"path": rel(HLSL_PATH), "sha256": sha256_of_file(HLSL_PATH)},
            "dxil": {"path": rel(DXIL_PATH), "sha256": sha256_of_file(DXIL_PATH)},
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_math_source": (
                "none — ADDITIVE pass (no native Godot compute shader); the "
                "aligned native behavior is the CPU instance-count write at "
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/"
                "mesh_storage.cpp L2210 and the untouched remaining command dwords"
            ),
        },
        "value_tolerance": 0,
        "margin_floor": MARGIN_FLOOR,
        "tolerance_note": (
            "the kernel's outputs are pure u32 (bitmask words + counts + "
            "untouched command dwords), so the GPU output must match the CPU "
            "reference EXACTLY, word for word (zero tolerance). The float "
            "intermediates feed only comparisons; every instance x plane "
            "comparison in these fixtures sits at least margin_floor away "
            "from the dist == -world_radius threshold (asserted at "
            "generation), so ULP-level GPU reassociation / FMA / sqrt "
            "differences (~1e-7 relative) cannot flip any classification"
        ),
        "gpu_dispatch_kind": None,
        "cases": cases,
        "coverage_totals": total_coverage,
        "min_classification_margin": global_min_margin,
        "notes": [
            "GPU-observed values are pending a real dispatch; the standalone S6 smoke "
            "(a later slice) imports this module's fixtures + reference and verifies "
            "every measured output word exactly.",
            "The instance-count dwords and the visibility bitmask are assumed zeroed "
            "before the dispatch (the runtime patch slices own that zeroing); the "
            "harness must upload the initial command words and explicit bitmask zeros.",
            "Non-count command dwords carry nonzero sentinels (dword 0 mirrors the "
            "native vertices-drawn init; dwords 2-4 are zero in native Godot) so a "
            "kernel that clobbers any non-count dword fails the exact comparison.",
            "Fixtures carry every parameter through the 36-dword b0 (stride 12, "
            "command stride 5, count dword index 1 are parameters, not hardcoded); "
            "distinct instance counts / surface counts / frustums per case mean a "
            "kernel with any hardcoded size cannot pass all three cases.",
            "The conservative sphere test may keep truly-invisible instances visible "
            "(Frobenius radius upper bound) but can never cull a visible one; "
            "crossing-sphere fixtures pin the visible side of that behavior.",
        ],
        "does_not_imply": DOES_NOT_IMPLY,
    }
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(
        f"[grx015-math-parity] status=pending_gpu_dispatch cases={len(cases)} "
        f"min_margin={global_min_margin:.6f} coverage={total_coverage} evidence={EVIDENCE_PATH}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
