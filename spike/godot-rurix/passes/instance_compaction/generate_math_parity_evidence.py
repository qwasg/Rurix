#!/usr/bin/env python3
"""GRX-016: instance_compaction math parity evidence generator (CPU reference vs GPU).

Computes a CPU **integer-exact / byte-exact** reference for the three-dispatch
instance_compaction chain (scan_local -> scan_groups -> scatter; see
PASS_CONTRACT.md sec 5.1) on deterministic synthetic visibility-bitmask +
transform fixtures and writes ``math_parity_evidence.json``.

Nothing in the chain does float arithmetic: the two prefix scans are u32
addition on exact values and the scatter is a bit-preserving uint4 move of
the 12-float (3 x float4) 3D transform payload. The expected values are
therefore **bit-exact**: a D3D12 execution of the three
``artifacts/hlsl_bridge/instance_compaction_*.hlsl`` kernels must reproduce
every ``local_prefix`` / ``group_totals`` / ``group_offsets`` word
element-wise, the ``survivor_count`` word, and the whole ``dst_transforms``
buffer byte-for-byte with ZERO tolerance (dst is zero-initialized; elements
at rank >= survivor_count must stay zero because non-survivors write
nothing).

The synthetic fixtures deliberately cover: sparse survival spanning multiple
groups, all-survive, zero-survive, GARBAGE bits in the last mask word beyond
total_instances-1 (both the kernels and this reference must ignore them via
the p < total_instances bound; GRX-015 is not required to zero-pad), an
empty-leading-group case (nonzero group_offsets with zero group_totals ahead
of survivors), and non-multiples of both the 32-bit word and the 256-thread
group. The generator FAILS if any of those coverage counters is zero, so the
fixture set cannot silently degenerate. The capacity contract
(num_groups <= 256, i.e. N <= 65536) is asserted per case.

GPU side: honest stub. Without a GPU results document the evidence records
``status=pending_gpu_dispatch`` with ``gpu_observed=null`` per case. The
standalone dispatch smoke (S6, host-exclusive) will execute the real
three-dispatch chain with the sec-5.1 barrier contract and verify every
measured word/byte against this same CPU formula. This evidence never
implies real_gpu_pass=true or default enablement.
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
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

GROUP_SIZE = 256
MAX_GROUPS = 256  # scan_groups is a single 256-thread group
MAX_INSTANCES = GROUP_SIZE * MAX_GROUPS  # 65536
TRANSFORM_STRIDE_FLOATS = 12  # 3D transform-only MultiMesh stride (3 float4)

# ZERO tolerance: the whole chain is u32 adds + bit-preserving moves.
MAX_ABS_ERROR_TOLERANCE = 0

MATH_PARITY_STATUS = "instance_compaction_cpu_reference_proven_pending_gpu_dispatch"

VARIANT_HLSL = {
    "scan_local": BRIDGE_DIR / "instance_compaction_scan_local.hlsl",
    "scan_groups": BRIDGE_DIR / "instance_compaction_scan_groups.hlsl",
    "scatter": BRIDGE_DIR / "instance_compaction_scatter.hlsl",
}
VARIANT_DXIL = {
    "scan_local": BRIDGE_DIR / "instance_compaction_scan_local.dxil",
    "scan_groups": BRIDGE_DIR / "instance_compaction_scan_groups.dxil",
    "scatter": BRIDGE_DIR / "instance_compaction_scatter.dxil",
}

INPUT_PATTERN = {
    "visibility_mask": "per-case deterministic bit rule (see each case's survive_rule); packed LSB-first into u32 words (bit p = word p>>5, bit p&31)",
    "mask_tail": "bits of the last word beyond total_instances-1 are set to GARBAGE (all ones) in the tail-coverage case and MUST be ignored",
    "transform_lane": "src_transforms lane value = f32((p * 12 + lane) * 0.03125 - 100.0), exactly representable in binary32 for the fixture ranges (0.03125 = 2^-5); the payload is moved bit-preserving, never computed on",
    "note": "the transform payload layout mirrors mesh_storage.cpp L1900-1911: 3 rows of (basis row, origin component) = 3 float4 per instance",
}

DOES_NOT_IMPLY = [
    "Godot runtime instance_compaction pass completion",
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


def sha256_u32_le(words: list[int]) -> str:
    return hashlib.sha256(struct.pack(f"<{len(words)}I", *words)).hexdigest()


def sha256_f32_le(values: list[float]) -> str:
    return hashlib.sha256(struct.pack(f"<{len(values)}f", *values)).hexdigest()


# ---- deterministic fixtures -----------------------------------------------


def transform_lane(p: int, lane: int) -> float:
    # Exactly representable: (p*12+lane) <= ~8000 scaled by 2^-5, offset -100.
    return f32((p * TRANSFORM_STRIDE_FLOATS + lane) * 0.03125 - 100.0)


def src_transforms(n: int) -> list[float]:
    return [transform_lane(p, lane) for p in range(n) for lane in range(TRANSFORM_STRIDE_FLOATS)]


def pack_mask(n: int, survive, garbage_tail: bool) -> list[int]:
    words = [0] * ((n + 31) // 32)
    for p in range(n):
        if survive(p):
            words[p >> 5] |= 1 << (p & 31)
    if garbage_tail and n % 32 != 0:
        # Set every bit of the last word beyond n-1: the kernels and the CPU
        # reference must ignore them via the p < total_instances bound.
        tail_bits = ((1 << 32) - 1) ^ ((1 << (n % 32)) - 1)
        words[-1] |= tail_bits
    return words


# ---- CPU reference (mirrors the three kernels exactly) ---------------------


def reference_chain(n: int, mask_words: list[int]) -> dict[str, object]:
    """Integer-exact mirror of scan_local -> scan_groups -> scatter."""
    num_groups = (n + GROUP_SIZE - 1) // GROUP_SIZE
    assert 1 <= num_groups <= MAX_GROUPS, "capacity contract: num_groups <= 256"

    def bit(p: int) -> int:
        # Same read the kernels do, guarded by p < n at every consumer.
        return (mask_words[p >> 5] >> (p & 31)) & 1

    # D1 scan_local: per-group exclusive prefix + per-group totals.
    local_prefix = [0] * n
    group_totals = [0] * num_groups
    for g in range(num_groups):
        running = 0
        for tid in range(GROUP_SIZE):
            p = g * GROUP_SIZE + tid
            if p >= n:
                break
            local_prefix[p] = running
            running += bit(p)
        # The kernel's last lane holds the inclusive total (out-of-range lanes
        # contribute 0), which equals `running` here.
        group_totals[g] = running

    # D2 scan_groups: exclusive prefix over group totals + grand total.
    group_offsets = [0] * num_groups
    running = 0
    for g in range(num_groups):
        group_offsets[g] = running
        running += group_totals[g]
    survivor_count = running

    # D3 scatter: bit-preserving move of survivors to the front; dst is
    # zero-initialized and non-survivors write nothing.
    src = src_transforms(n)
    dst = [0.0] * (n * TRANSFORM_STRIDE_FLOATS)
    for p in range(n):
        if bit(p) == 0:
            continue
        rank = group_offsets[p // GROUP_SIZE] + local_prefix[p]
        for lane in range(TRANSFORM_STRIDE_FLOATS):
            dst[rank * TRANSFORM_STRIDE_FLOATS + lane] = src[p * TRANSFORM_STRIDE_FLOATS + lane]

    return {
        "num_groups": num_groups,
        "local_prefix": local_prefix,
        "group_totals": group_totals,
        "group_offsets": group_offsets,
        "survivor_count": survivor_count,
        "src": src,
        "dst": dst,
    }


def build_case(case_id: str, n: int, survive, survive_rule: str, garbage_tail: bool, note: str) -> dict[str, object]:
    mask_words = pack_mask(n, survive, garbage_tail)
    ref = reference_chain(n, mask_words)
    num_groups = int(ref["num_groups"])
    survivor_count = int(ref["survivor_count"])
    local_prefix = ref["local_prefix"]
    dst = ref["dst"]

    # Boundary samples: first/last survivor and the first untouched tail
    # element (must stay zero).
    survivors = [p for p in range(n) if ((mask_words[p >> 5] >> (p & 31)) & 1) == 1]
    samples = []
    sample_ranks = []
    if survivors:
        sample_ranks = sorted(set([0, survivor_count // 2, survivor_count - 1]))
    for rank in sample_ranks:
        p = survivors[rank]
        samples.append({
            "rank": rank,
            "source_instance": p,
            "dst_lanes_f32": [
                dst[rank * TRANSFORM_STRIDE_FLOATS + lane]
                for lane in range(TRANSFORM_STRIDE_FLOATS)
            ],
        })
    tail_sample = None
    if survivor_count < n:
        tail_sample = {
            "rank": survivor_count,
            "expect": "all 12 lanes zero (untouched; dst zero-initialized, non-survivors write nothing)",
            "dst_lanes_f32": [
                dst[survivor_count * TRANSFORM_STRIDE_FLOATS + lane]
                for lane in range(TRANSFORM_STRIDE_FLOATS)
            ],
        }

    return {
        "case_id": case_id,
        "note": note,
        "total_instances": n,
        "bitmask_words": len(mask_words),
        "num_groups": num_groups,
        "constants": {
            "total_instances": n,
            "bitmask_words": len(mask_words),
            "num_groups": num_groups,
            "transform_stride_vec4": TRANSFORM_STRIDE_FLOATS // 4,
            "pad0": 0,
            "pad1": 0,
            "pad2": 0,
            "pad3": 0,
        },
        "dispatch_chain": {
            "scan_local": [(n + GROUP_SIZE - 1) // GROUP_SIZE, 1, 1],
            "scan_groups": [1, 1, 1],
            "scatter": [(n + GROUP_SIZE - 1) // GROUP_SIZE, 1, 1],
        },
        "survive_rule": survive_rule,
        "mask_tail_garbage": garbage_tail,
        "visibility_mask_u32": mask_words,
        "survivor_count": survivor_count,
        "cpu_expected": {
            "local_prefix_u32_le_sha256": sha256_u32_le(ref["local_prefix"]),
            "group_totals_u32": ref["group_totals"],
            "group_offsets_u32": ref["group_offsets"],
            "survivor_count_u32": survivor_count,
            "src_transforms_f32_le_sha256": sha256_f32_le(ref["src"]),
            "dst_transforms_f32_le_sha256": sha256_f32_le(dst),
            "dst_compare": "WHOLE buffer byte-exact (survivor region + zero tail); dst zero-initialized before the chain",
            "local_prefix_samples": [
                {"instance": p, "local_prefix": local_prefix[p]}
                for p in sorted(set([0, n // 2, n - 1]))
            ],
            "dst_boundary_samples": samples,
            "dst_first_tail_element": tail_sample,
        },
        "gpu_observed": None,
    }


def main() -> int:
    def lcg_survive(p: int) -> bool:
        # Deterministic pseudo-random ~25% survival, holes across word and
        # group boundaries.
        return ((p * 1103515245 + 12345) >> 16) % 4 == 0

    cases = [
        build_case(
            "sparse_survival_multi_group", 600, lcg_survive,
            "((p * 1103515245 + 12345) >> 16) % 4 == 0  (~25%)", False,
            "sparse survivors spanning 3 groups (600 = 2*256 + 88); exercises nonzero group_offsets and stable rank order",
        ),
        build_case(
            "all_survive", 513, lambda p: True,
            "every instance survives", False,
            "compaction degenerates to the identity move; 513 crosses both the word (513 = 16*32+1) and group (2*256+1) boundaries",
        ),
        build_case(
            "zero_survive", 384, lambda p: False,
            "no instance survives", False,
            "survivor_count == 0; dst stays all-zero; prefix arrays all zero",
        ),
        build_case(
            "mask_tail_garbage_bits_ignored", 70, lambda p: p % 5 == 0,
            "p % 5 == 0, PLUS all bits of the last mask word beyond p=69 forced to 1", True,
            "proves the p < total_instances bound: garbage tail bits in word 2 (bits 70..95) must not create survivors (GRX-015 need not zero-pad)",
        ),
        build_case(
            "single_survivor_last_instance_empty_leading_group", 300, lambda p: p == 299,
            "only p == 299 survives", False,
            "group 0 is entirely empty while group 1 holds the sole survivor: rank 0 comes from group_offsets[1]=0 + local_prefix[299]=0 with group_totals=[0,1]",
        ),
    ]

    # Coverage counters: fail if the fixture set silently degenerates.
    coverage = {
        "multi_group_sparse_cases": sum(
            1 for c in cases
            if int(c["num_groups"]) >= 2 and 0 < int(c["survivor_count"]) < int(c["total_instances"])
        ),
        "all_survive_cases": sum(
            1 for c in cases if int(c["survivor_count"]) == int(c["total_instances"])
        ),
        "zero_survive_cases": sum(1 for c in cases if int(c["survivor_count"]) == 0),
        "garbage_tail_word_cases": sum(1 for c in cases if bool(c["mask_tail_garbage"])),
        "empty_leading_group_cases": sum(
            1 for c in cases
            if int(c["survivor_count"]) > 0 and int(c["cpu_expected"]["group_totals_u32"][0]) == 0
        ),
        "non_word_multiple_n_cases": sum(1 for c in cases if int(c["total_instances"]) % 32 != 0),
        "non_group_multiple_n_cases": sum(
            1 for c in cases if int(c["total_instances"]) % GROUP_SIZE != 0
        ),
    }
    degenerate = [key for key, count in coverage.items() if count == 0]
    if degenerate:
        raise SystemExit(f"fixture coverage degenerated: {degenerate}")

    evidence = {
        "pass_id": "instance_compaction",
        "subject": "grx016_instance_compaction_math_parity",
        "status": "pending_gpu_dispatch",
        "generated_at_utc": utc_now(),
        "math_status": MATH_PARITY_STATUS,
        "cpu_reference": {
            "formula": (
                "bit(p) = (mask[p>>5] >> (p&31)) & 1 guarded by p < total_instances; "
                "D1 scan_local: local_prefix[p] = exclusive prefix of bit within its "
                "256-thread group, group_totals[g] = survivors in group g; "
                "D2 scan_groups (single group, num_groups <= 256): group_offsets = "
                "exclusive prefix over group_totals, survivor_count = grand total; "
                "D3 scatter: surviving p moves its 12-float (3 x float4) payload "
                "bit-preserving to rank = group_offsets[p/256] + local_prefix[p]; "
                "non-survivors write nothing (zero-initialized dst tail stays zero). "
                "All integer math; STABLE compaction (rank monotone in p)."
            ),
            "hlsl_kernels": {
                variant: {"path": rel(path), "sha256": sha256_of_file(path)}
                for variant, path in VARIANT_HLSL.items()
            },
            "dxil": {
                variant: {"path": rel(path), "sha256": sha256_of_file(path)}
                for variant, path in VARIANT_DXIL.items()
            },
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_math_source": (
                "NONE — Godot has no native compaction kernel. The consumption "
                "contract is mesh_storage.h multimesh_get_instances_to_draw "
                "L721-728 ('draw the first N instances'); the transform layout "
                "is mesh_storage.cpp L1900-1911 (12 floats = 3 float4 rows)."
            ),
        },
        "group_size": GROUP_SIZE,
        "max_instances": MAX_INSTANCES,
        "transform_stride_floats": TRANSFORM_STRIDE_FLOATS,
        "input_pattern": INPUT_PATTERN,
        "max_abs_error_tolerance": MAX_ABS_ERROR_TOLERANCE,
        "tolerance_note": (
            "ZERO tolerance everywhere: the two prefix scans are u32 addition "
            "on exact values (element-wise integer equality required) and the "
            "scatter is a bit-preserving uint4 move (whole-buffer byte "
            "equality required, including the untouched zero tail). No float "
            "arithmetic exists anywhere in the chain."
        ),
        "fixture_coverage": coverage,
        "gpu_dispatch_kind": None,
        "cases": cases,
        "notes": [
            "GPU-observed values are pending a real three-dispatch chain; the later "
            "standalone smoke (S6, host-exclusive) must execute D1 -> UAV barrier -> "
            "D2 -> UAV barrier -> D3 (PASS_CONTRACT.md sec 5.1) and verify every "
            "measured word/byte against this CPU formula with zero tolerance.",
            "The visibility mask is the GRX-015 gpu_culling interface declared in "
            "PASS_CONTRACT.md sec 5.3 (u32[ceil(N/32)], bit p = word p>>5 bit p&31); "
            "tail bits beyond total_instances-1 are don't-care and one case proves "
            "they are ignored.",
            "Ordering contract: compaction is STABLE (rank = exclusive prefix by "
            "index) but absolute instance indices change; opaque-only applicability "
            "(PASS_CONTRACT.md sec 5.2).",
            "The capacity contract num_groups <= 256 (total_instances <= 65536) is "
            "asserted per fixture case and enforced fail-closed by the later S4 gate.",
        ],
        "does_not_imply": DOES_NOT_IMPLY,
    }
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[grx016-math-parity] status=pending_gpu_dispatch cases={len(cases)} evidence={EVIDENCE_PATH}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
