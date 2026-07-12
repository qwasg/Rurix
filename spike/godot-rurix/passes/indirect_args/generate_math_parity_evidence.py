#!/usr/bin/env python3
"""GRX-018: indirect_args math parity evidence generator (CPU reference vs GPU).

Computes a CPU **integer-exact** reference for the indirect_args kernel PAIR
(the 5-dword command-block WRITE kernel and the RESIDENT VALIDATION red-leg
kernel; see PASS_CONTRACT.md sec 5) on deterministic synthetic survivor-count /
command-template fixtures and writes ``math_parity_evidence.json``.

All arithmetic is pure u32 word math (``min``, compare, copy, bitmask), so the
expected values are **bit-exact**: a D3D12 dispatch of the
``artifacts/hlsl_bridge/indirect_args_write.hlsl`` /
``indirect_args_validate.hlsl`` kernels must reproduce every output word with
ZERO tolerance (no floats participate).

The synthetic fixtures deliberately cover every contract lane: single surface,
multi surface (distinct per-surface index counts), CLAMP TRIGGERED (survivors
> max_instance_count -> instance_count clamped + bit-6 red flags + nonzero
clamp_trigger_count), zero survivors, the MAX_SURFACES=8 boundary, nonzero
template statics (template-fidelity proof; natively dwords 2-4 are zero),
a nonzero survivor_count_word_offset, and a CORRUPTED-COMMAND-BUFFER
validation red case proving the validation kernel's expected output flags the
corruption (nonzero per-surface masks + mismatch_count). The generator FAILS
if any of those coverage counters is zero, so the fixture set cannot silently
degenerate — in particular a validation reference that reports clean on the
corrupted fixture fails the coverage gate.

GPU side: honest stub. Without a GPU results document the evidence records
``status=pending_gpu_dispatch`` with ``gpu_observed=null`` per case. The
standalone dispatch smoke (S6, later, host-exclusive) will import THIS
module's fixtures + reference and verify every measured output word exactly.
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
WRITE_HLSL_PATH = BRIDGE_DIR / "indirect_args_write.hlsl"
VALIDATE_HLSL_PATH = BRIDGE_DIR / "indirect_args_validate.hlsl"
WRITE_DXIL_PATH = BRIDGE_DIR / "indirect_args_write.dxil"
VALIDATE_DXIL_PATH = BRIDGE_DIR / "indirect_args_validate.dxil"
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

MAX_SURFACES = 8
COMMAND_STRIDE = 5  # Godot INDIRECT_MULTIMESH_COMMAND_STRIDE (mesh_storage.h L62-64)

MATH_PARITY_STATUS = "indirect_args_cpu_reference_proven_pending_gpu_dispatch"

# Validation bitmask bits (must match indirect_args_validate.hlsl).
BIT_INDEX_COUNT = 1 << 0
BIT_INSTANCE_COUNT = 1 << 1
BIT_FIRST_INDEX = 1 << 2
BIT_VERTEX_OFFSET = 1 << 3
BIT_FIRST_INSTANCE = 1 << 4
BIT_IN_BUFFER_CLAMP_VIOLATION = 1 << 5
BIT_PRODUCER_CLAMP_TRIGGER = 1 << 6
MISMATCH_MASK = 0x3F  # bits 0-5 count toward mismatch_count
CLAMP_MASK = 0x40     # bit 6 counts toward clamp_trigger_count

DOES_NOT_IMPLY = [
    "Godot runtime indirect_args pass completion",
    "real_gpu_pass=true",
    "real_d3d12_dispatch_recorded=true",
    "visual success",
    "temporal stability success",
    "GPU timestamp success",
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


def sha256_of_u32_le(words: list[int]) -> str:
    return hashlib.sha256(struct.pack(f"<{len(words)}I", *words)).hexdigest()


# ---- CPU reference (pure u32; mirrors the HLSL kernels word-for-word) ---------

def template_words(templates: list[dict[str, int]]) -> list[int]:
    """Flatten per-surface templates into the 40-dword b0 template region."""
    words = []
    for s in range(MAX_SURFACES):
        if s < len(templates):
            t = templates[s]
            words.extend([
                t["index_count"] & 0xFFFFFFFF,
                t.get("instance_count_reserved", 0) & 0xFFFFFFFF,
                t["first_index"] & 0xFFFFFFFF,
                t["vertex_offset"] & 0xFFFFFFFF,
                t["first_instance"] & 0xFFFFFFFF,
            ])
        else:
            words.extend([0, 0, 0, 0, 0])
    assert len(words) == MAX_SURFACES * COMMAND_STRIDE
    return words


def write_kernel_reference(consts: dict[str, object], survivor_buffer: list[int]) -> list[int]:
    """Expected dst_command_buffer contents (surface_count * 5 words)."""
    surface_count = int(consts["surface_count"])
    max_instances = int(consts["max_instance_count"])
    offset = int(consts["survivor_count_word_offset"])
    twords = template_words(consts["templates"])  # type: ignore[arg-type]

    survivors = survivor_buffer[offset] & 0xFFFFFFFF
    clamped = min(survivors, max_instances)

    out: list[int] = []
    for s in range(surface_count):
        t = s * COMMAND_STRIDE
        out.extend([
            twords[t + 0],  # index_count (b0 backfill)
            clamped,        # instance_count (GPU-dynamic)
            twords[t + 2],  # first_index (b0 backfill)
            twords[t + 3],  # vertex_offset (b0 backfill)
            twords[t + 4],  # first_instance (b0 backfill)
        ])
    return out


def validate_kernel_reference(
    consts: dict[str, object],
    survivor_buffer: list[int],
    command_buffer: list[int],
) -> list[int]:
    """Expected dst_validation contents (2 + surface_count words, zeroed first)."""
    surface_count = int(consts["surface_count"])
    max_instances = int(consts["max_instance_count"])
    offset = int(consts["survivor_count_word_offset"])
    twords = template_words(consts["templates"])  # type: ignore[arg-type]

    survivors = survivor_buffer[offset] & 0xFFFFFFFF
    expected_instance_count = min(survivors, max_instances)

    masks: list[int] = []
    mismatch_count = 0
    clamp_trigger_count = 0
    for s in range(surface_count):
        t = s * COMMAND_STRIDE
        base = s * COMMAND_STRIDE
        mask = 0
        if command_buffer[base + 0] != twords[t + 0]:
            mask |= BIT_INDEX_COUNT
        if command_buffer[base + 1] != expected_instance_count:
            mask |= BIT_INSTANCE_COUNT
        if command_buffer[base + 2] != twords[t + 2]:
            mask |= BIT_FIRST_INDEX
        if command_buffer[base + 3] != twords[t + 3]:
            mask |= BIT_VERTEX_OFFSET
        if command_buffer[base + 4] != twords[t + 4]:
            mask |= BIT_FIRST_INSTANCE
        if command_buffer[base + 1] > max_instances:
            mask |= BIT_IN_BUFFER_CLAMP_VIOLATION
        if survivors > max_instances:
            mask |= BIT_PRODUCER_CLAMP_TRIGGER
        masks.append(mask)
        if mask & MISMATCH_MASK:
            mismatch_count += 1
        if mask & CLAMP_MASK:
            clamp_trigger_count += 1
    return [mismatch_count, clamp_trigger_count, *masks]


# ---- fixtures ------------------------------------------------------------------

def make_templates(specs: list[tuple[int, int, int, int]]) -> list[dict[str, int]]:
    """specs: (index_count, first_index, vertex_offset, first_instance)."""
    return [
        {
            "index_count": ic,
            "instance_count_reserved": 0,
            "first_index": fi,
            "vertex_offset": vo,
            "first_instance": fin,
        }
        for (ic, fi, vo, fin) in specs
    ]


def build_case(
    case_id: str,
    consts: dict[str, object],
    survivor_buffer: list[int],
    corrupt: dict[int, int] | None = None,
    note: str | None = None,
) -> dict[str, object]:
    surface_count = int(consts["surface_count"])
    assert 1 <= surface_count <= MAX_SURFACES
    assert len(consts["templates"]) == surface_count  # type: ignore[arg-type]

    expected_command = write_kernel_reference(consts, survivor_buffer)
    # The validation input is the write kernel's own output unless the case
    # deliberately corrupts words (the validation red leg).
    validated_command = list(expected_command)
    corruption_records = []
    if corrupt:
        for word_index, bad_value in sorted(corrupt.items()):
            corruption_records.append({
                "word_index": word_index,
                "surface": word_index // COMMAND_STRIDE,
                "dword_in_block": word_index % COMMAND_STRIDE,
                "original_value": validated_command[word_index],
                "corrupted_value": bad_value & 0xFFFFFFFF,
            })
            validated_command[word_index] = bad_value & 0xFFFFFFFF
    expected_validation = validate_kernel_reference(consts, survivor_buffer, validated_command)

    survivors = survivor_buffer[int(consts["survivor_count_word_offset"])]
    case: dict[str, object] = {
        "case_id": case_id,
        "constants": {
            "surface_count": surface_count,
            "max_instance_count": int(consts["max_instance_count"]),
            "survivor_count_word_offset": int(consts["survivor_count_word_offset"]),
            "pad0": 0,
            "templates": consts["templates"],
        },
        "survivor_buffer": list(survivor_buffer),
        "survivors_read": survivors,
        "expected_instance_count": min(survivors, int(consts["max_instance_count"])),
        "clamp_triggered": survivors > int(consts["max_instance_count"]),
        "cpu_expected_command_buffer": expected_command,
        "cpu_expected_command_buffer_u32_le_sha256": sha256_of_u32_le(expected_command),
        "validation_input_command_buffer": validated_command,
        "validation_input_is_corrupted": bool(corrupt),
        "corrupted_words": corruption_records,
        "cpu_expected_validation": {
            "mismatch_count": expected_validation[0],
            "clamp_trigger_count": expected_validation[1],
            "per_surface_masks": expected_validation[2:],
            "words": expected_validation,
            "words_u32_le_sha256": sha256_of_u32_le(expected_validation),
        },
        "gpu_observed": None,
    }
    if note:
        case["note"] = note
    return case


def main() -> int:
    cases = [
        # 1) Single surface, plain lane: native-shaped statics (zeros), count
        #    below the clamp ceiling.
        build_case(
            "single_surface_basic",
            {
                "surface_count": 1,
                "max_instance_count": 64,
                "survivor_count_word_offset": 0,
                "templates": make_templates([(3000, 0, 0, 0)]),
            },
            survivor_buffer=[17],
            note="single 5-dword block; dword 1 = survivors (17), below the clamp ceiling",
        ),
        # 2) Multi surface: distinct per-surface index counts, one shared
        #    survivor count (native parity: same p_visible per surface block).
        build_case(
            "multi_surface_shared_count",
            {
                "surface_count": 4,
                "max_instance_count": 64,
                "survivor_count_word_offset": 0,
                "templates": make_templates([
                    (300, 0, 0, 0),
                    (4500, 0, 0, 0),
                    (36, 0, 0, 0),
                    (123456, 0, 0, 0),
                ]),
            },
            survivor_buffer=[9],
            note="4 blocks, distinct index_count per surface, shared instance_count",
        ),
        # 3) CLAMP TRIGGERED red lane: survivors (100) > max (64) -> written
        #    instance_count = 64, bit-6 flags on every surface, nonzero
        #    clamp_trigger_count, zero mismatch_count (the writer clamped).
        build_case(
            "clamp_triggered_producer_violation",
            {
                "surface_count": 2,
                "max_instance_count": 64,
                "survivor_count_word_offset": 0,
                "templates": make_templates([(600, 0, 0, 0), (900, 0, 0, 0)]),
            },
            survivor_buffer=[100],
            note="producer-interface violation lane: clamp fires, validation red-flags bit 6 per surface; runtime policy = fallback",
        ),
        # 4) Zero survivors: instance_count = 0 (a legal no-op indirect draw).
        build_case(
            "zero_survivors",
            {
                "surface_count": 3,
                "max_instance_count": 128,
                "survivor_count_word_offset": 0,
                "templates": make_templates([(60, 0, 0, 0), (90, 0, 0, 0), (120, 0, 0, 0)]),
            },
            survivor_buffer=[0],
            note="all blocks get instance_count=0; draw becomes a no-op, never invalid",
        ),
        # 5) MAX_SURFACES boundary + nonzero survivor word offset + nonzero
        #    template statics (template-fidelity proof; natively dwords 2-4
        #    are zero, see known_gaps).
        build_case(
            "max_surfaces_nonzero_statics_and_offset",
            {
                "surface_count": 8,
                "max_instance_count": 4096,
                "survivor_count_word_offset": 3,
                "templates": make_templates([
                    (16777216 + s * 7, 10 + s, 20 + s, 30 + s) for s in range(8)
                ]),
            },
            survivor_buffer=[111, 222, 333, 2048, 555],
            note="surface_count=8 boundary; survivor count read from word 3 (2048); index_count above 2^24 proves u32 (not f32) carriage; nonzero statics prove template backfill copies rather than hardcoding zero",
        ),
        # 6) VALIDATION RED CASE: deliberately corrupt the write kernel's
        #    output before validating; the CPU reference must flag it.
        build_case(
            "validation_detects_corruption",
            {
                "surface_count": 4,
                "max_instance_count": 64,
                "survivor_count_word_offset": 0,
                "templates": make_templates([
                    (300, 0, 0, 0),
                    (4500, 0, 0, 0),
                    (36, 0, 0, 0),
                    (123456, 0, 0, 0),
                ]),
            },
            survivor_buffer=[9],
            corrupt={
                # surface 1 dword 0: wrong index_count (a stride/offset bug shape)
                1 * COMMAND_STRIDE + 0: 4501,
                # surface 2 dword 1: unclamped foreign instance_count (999 > 64)
                2 * COMMAND_STRIDE + 1: 999,
            },
            note="resident red leg proof: surface 1 -> bit 0 (index_count), surface 2 -> bits 1+5 (instance_count mismatch + in-buffer clamp violation); mismatch_count=2 -> runtime policy = immediate fallback (GRX_PLAN GRX-018)",
        ),
    ]

    # ---- coverage gate (fail-closed: the fixture set cannot degenerate) -------
    def count(pred) -> int:
        return sum(1 for c in cases if pred(c))

    coverage = {
        "single_surface_cases": count(lambda c: c["constants"]["surface_count"] == 1),
        "multi_surface_cases": count(lambda c: c["constants"]["surface_count"] > 1),
        "max_surfaces_boundary_cases": count(lambda c: c["constants"]["surface_count"] == MAX_SURFACES),
        "clamp_triggered_cases": count(lambda c: c["clamp_triggered"]),
        "zero_survivor_cases": count(lambda c: c["survivors_read"] == 0),
        "nonzero_word_offset_cases": count(lambda c: c["constants"]["survivor_count_word_offset"] != 0),
        "nonzero_static_template_cases": count(
            lambda c: any(
                t["first_index"] or t["vertex_offset"] or t["first_instance"]
                for t in c["constants"]["templates"]
            )
        ),
        "corruption_red_cases": count(lambda c: c["validation_input_is_corrupted"]),
        "clean_validation_cases": count(
            lambda c: not c["validation_input_is_corrupted"]
            and c["cpu_expected_validation"]["mismatch_count"] == 0
        ),
        "red_validation_cases_with_nonzero_mismatch": count(
            lambda c: c["validation_input_is_corrupted"]
            and c["cpu_expected_validation"]["mismatch_count"] > 0
        ),
        "cases_with_index_count_above_2pow24": count(
            lambda c: any(t["index_count"] > (1 << 24) for t in c["constants"]["templates"])
        ),
    }
    degenerate = [key for key, value in coverage.items() if value == 0]
    if degenerate:
        raise SystemExit(f"[grx018-math-parity] coverage gate FAILED; zero-count lanes: {degenerate}")
    # A corrupted fixture whose expected validation reads clean would be a
    # broken red leg — the check above (red_validation_cases_with_nonzero_
    # mismatch) fails closed on it.

    evidence = {
        "pass_id": "indirect_args",
        "subject": "grx018_indirect_args_math_parity",
        "status": "pending_gpu_dispatch",
        "generated_at_utc": utc_now(),
        "math_status": MATH_PARITY_STATUS,
        "cpu_reference": {
            "write_formula": (
                "survivors = src_survivor_counts[survivor_count_word_offset]; "
                "clamped = min(survivors, max_instance_count); per surface s: "
                "block[s] = {template[s].index_count, clamped, "
                "template[s].first_index, template[s].vertex_offset, "
                "template[s].first_instance}; all five dwords written every "
                "dispatch; pure u32 (no floats)"
            ),
            "validate_formula": (
                "expected1 = min(survivors, max_instance_count); per surface "
                "s: bitmask bits 0-4 = generated dword c != expected dword c; "
                "bit 5 = in-buffer instance_count > max_instance_count; bit 6 "
                "= survivors > max_instance_count (clamp fired); "
                "dst_validation = [mismatch_count(bits0-5), "
                "clamp_trigger_count(bit6), mask_0..mask_{n-1}] with the "
                "counters accumulated via InterlockedAdd over a zeroed buffer"
            ),
            "write_hlsl_kernel": {"path": rel(WRITE_HLSL_PATH), "sha256": sha256_of_file(WRITE_HLSL_PATH)},
            "validate_hlsl_kernel": {"path": rel(VALIDATE_HLSL_PATH), "sha256": sha256_of_file(VALIDATE_HLSL_PATH)},
            "write_dxil": {"path": rel(WRITE_DXIL_PATH), "sha256": sha256_of_file(WRITE_DXIL_PATH)},
            "validate_dxil": {"path": rel(VALIDATE_DXIL_PATH), "sha256": sha256_of_file(VALIDATE_DXIL_PATH)},
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_math_source": (
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/"
                "mesh_storage.cpp (CPU producer: _multimesh_set_mesh L1674-1696 "
                "static fill + _multimesh_set_visible_instances L2210 dword-1 "
                "buffer_update; INDIRECT_MULTIMESH_COMMAND_STRIDE=5, "
                "mesh_storage.h L62-64) — NO native compute shader exists"
            ),
        },
        "max_abs_error_tolerance": 0,
        "tolerance_note": (
            "ZERO tolerance: the pass is pure u32 word math (min, compare, "
            "copy, bitmask, atomic count); a GPU dispatch must reproduce "
            "every command-buffer and validation word exactly."
        ),
        "command_stride_dwords": COMMAND_STRIDE,
        "max_surfaces": MAX_SURFACES,
        "validation_bitmask_legend": {
            "bit0": "index_count mismatch",
            "bit1": "instance_count mismatch",
            "bit2": "first_index mismatch",
            "bit3": "vertex_offset mismatch",
            "bit4": "first_instance mismatch",
            "bit5": "in-buffer instance_count > max_instance_count (unclamped writer)",
            "bit6": "survivors > max_instance_count (producer violation; clamp fired)",
            "mismatch_count": "surfaces with any of bits 0-5 (dst_validation word 0)",
            "clamp_trigger_count": "surfaces with bit 6 (dst_validation word 1)",
            "runtime_policy": "any nonzero counter -> staging copy skipped, fallback_reason=validation_failed (GRX_PLAN GRX-018: any validation mismatch -> immediate fallback)",
        },
        "coverage": coverage,
        "gpu_dispatch_kind": None,
        "cases": cases,
        "notes": [
            "GPU-observed values are pending a real dispatch; the standalone smoke "
            "(S6, later, host-exclusive) will import these fixtures and verify every "
            "measured word exactly (write kernel output AND validation kernel output, "
            "including a corrupted-staging red leg).",
            "The survivor buffer is synthetic: the GRX-015/016 producer interface is "
            "declared in PASS_CONTRACT.md section 4.1 but neither producer pass has "
            "landed; parity here binds the consumer side of that interface.",
            "Template slots at s >= surface_count are carried zeros; "
            "surface{s}_instance_count_reserved is carried 0 (dword 1 is the "
            "GPU-dynamic instance_count and is never sourced from b0).",
            "The corrupted-case validation input is NOT what the write kernel "
            "produces; it models a foreign/buggy writer so the resident red leg has "
            "a proven nonzero expectation (a clean report on it would be a broken "
            "red leg and fails the coverage gate).",
        ],
        "does_not_imply": DOES_NOT_IMPLY,
    }
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(
        f"[grx018-math-parity] status=pending_gpu_dispatch cases={len(cases)} "
        f"coverage_ok evidence={EVIDENCE_PATH}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
