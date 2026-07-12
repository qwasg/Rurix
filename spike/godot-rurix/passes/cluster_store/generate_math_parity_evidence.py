#!/usr/bin/env python3
"""GRX-014: cluster_store math parity evidence generator (CPU reference vs GPU).

Computes a CPU **integer-exact** reference for the cluster_store math (the
complete store segment of ``bake_cluster()``; see PASS_CONTRACT.md sec 5 — a
single kernel with no mode switches, so no subset cut) on deterministic
synthetic ``cluster_render`` / ``element_buffer`` fixtures and writes
``math_parity_evidence.json``.

All arithmetic is pure u32 word math (bitmap scans, findLSB/findMSB, packed
u16 min/max, bitwise or), so the expected values are **bit-exact**: a D3D12
dispatch of the ``artifacts/hlsl_bridge/cluster_store_pack.hlsl`` kernel must
reproduce every output word with ZERO tolerance (no floats participate).

The synthetic fixtures deliberately cover every branch of the kernel:
``touches_near`` / ``touches_far`` overrides, single- and multi-slice z
ranges, the ``minmax == 0 -> 0xFFFF`` slice initialization branch, same-slice
min/max merging across elements, the ``z_range == 0`` guard ("should always be
> 0"), empty clusters, and the existence bitmap. The generator FAILS if any of
those coverage counters is zero, so the fixture cannot silently degenerate.
The Godot deployment default ``max_clustered_elements = 512`` is a documented
assumption only; fixtures use small 32-aligned capacities and carry every size
through the b0 mirror so nothing is hardcoded.

GPU side: honest stub. Without a GPU results document the evidence records
``status=pending_gpu_dispatch`` with ``gpu_observed=null`` per case. The
standalone dispatch smoke (S6, ``ci/grx014_cluster_store_d3d12_dispatch_smoke
.py``) imports THIS module's fixtures + reference and verifies every measured
output word exactly. This evidence never implies real_gpu_pass=true or default
enablement.
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
HLSL_PATH = BRIDGE_DIR / "cluster_store_pack.hlsl"
DXIL_PATH = BRIDGE_DIR / "cluster_store_pack.dxil"
EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"

ELEMENT_TYPE_MAX = 4
ELEMENT_STRIDE_BYTES = 80
U32_MASK = 0xFFFFFFFF

MATH_PARITY_STATUS = "cluster_store_cpu_reference_proven_pending_gpu_dispatch"

DOES_NOT_IMPLY = [
    "Godot runtime cluster_store pass completion",
    "real_gpu_pass=true",
    "manifest real_d3d12_dispatch_recorded=true",
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


def find_lsb(word: int) -> int:
    """glsl findLSB / HLSL firstbitlow for a nonzero u32."""
    assert word != 0
    return (word & -word).bit_length() - 1


def find_msb(word: int) -> int:
    """glsl findMSB / HLSL firstbithigh for a nonzero u32."""
    assert word != 0
    return word.bit_length() - 1


# ---- fixture cases --------------------------------------------------------


def parity_cases() -> list[dict]:
    """The tracked deterministic fixture cases. Every count is 32-aligned where
    the native setup() would align it; each case exercises distinct dims/params
    so a kernel with a hardcoded size cannot pass all of them."""
    return [
        {
            "case_id": "store_pack_grid_4x3_e64",
            "cluster_screen_size": (4, 3),
            "max_elements_by_type": 64,
            "render_element_count": 40,
            "note": "formula-driven usage/z_range grid with an empty cluster (c=7) and z_range==0 injections",
        },
        {
            "case_id": "store_pack_grid_2x2_minmax_merge",
            "cluster_screen_size": (2, 2),
            "max_elements_by_type": 32,
            "render_element_count": 16,
            "note": "shared z slices with descending/ascending original_index to force min- and max-side merges",
        },
        {
            "case_id": "store_pack_grid_3x1_touch_overrides",
            "cluster_screen_size": (3, 1),
            "max_elements_by_type": 32,
            "render_element_count": 12,
            "note": "explicit table: all near/far combos, single-bit + full-range + zero z_range, all 4 element types, empty cluster (c=2)",
        },
    ]


def case_constants(case: dict) -> dict:
    """Derive the full 8-dword b0 field set exactly like bake_cluster() does
    (cluster_builder_rd.cpp L522-531 / setup() L293-305)."""
    w, h = case["cluster_screen_size"]
    max_by_type = case["max_elements_by_type"]
    assert max_by_type % 32 == 0, "fixture capacities are pre-aligned like setup()"
    render_element_max = max_by_type * ELEMENT_TYPE_MAX
    count = case["render_element_count"]
    assert 0 < count <= render_element_max
    return {
        "cluster_render_data_size": render_element_max // 32 + render_element_max,
        "max_render_element_count_div_32": render_element_max // 32,
        "cluster_screen_size": (w, h),
        "render_element_count_div_32": (count + 31) // 32,
        "max_cluster_element_count_div_32": max_by_type // 32,
        "render_element_count": count,
        "render_element_max": render_element_max,
    }


def build_elements(case: dict) -> list[dict]:
    """Deterministic RenderElementData records. The store kernel reads only the
    four leading u32 fields; the float carry payload is nonzero garbage that a
    correct kernel must ignore."""
    case_id = case["case_id"]
    count = case["render_element_count"]
    max_by_type = case["max_elements_by_type"]
    elements: list[dict] = []
    if case_id == "store_pack_grid_3x1_touch_overrides":
        # Explicit table: (type, near, far, original_index).
        table = [
            (0, 0, 0, 5),
            (1, 1, 0, 9),
            (2, 0, 1, 2),
            (3, 1, 1, 30),
            (0, 0, 0, 0),
            (1, 0, 0, 31),
            (2, 1, 0, 17),
            (3, 0, 1, 4),
            (0, 1, 1, 11),
            (1, 0, 0, 9),
            (2, 0, 0, 2),
            (3, 0, 0, 30),
        ]
        assert len(table) == count
        for e, (etype, near, far, orig) in enumerate(table):
            elements.append({
                "type": etype,
                "touches_near": near,
                "touches_far": far,
                "original_index": orig,
                "seed": e,
            })
    elif case_id == "store_pack_grid_2x2_minmax_merge":
        for e in range(count):
            elements.append({
                "type": e % 2,
                "touches_near": 1 if e % 4 == 2 else 0,
                "touches_far": 1 if e % 8 == 5 else 0,
                # Even elements descend from the top, odd elements ascend: the
                # shared slices see both min-side and max-side merges.
                "original_index": (max_by_type - 1 - e) if e % 2 == 0 else e,
                "seed": e,
            })
    else:
        for e in range(count):
            elements.append({
                "type": e % ELEMENT_TYPE_MAX,
                "touches_near": 1 if e % 5 == 1 else 0,
                "touches_far": 1 if e % 7 == 3 else 0,
                "original_index": (e * 7 + 3) % max_by_type,
                "seed": e,
            })
    for record in elements:
        assert 0 <= record["original_index"] < max_by_type
        assert 0 <= record["type"] < ELEMENT_TYPE_MAX
    return elements


def pack_elements(elements: list[dict], render_element_max: int) -> bytes:
    """Pack the element table as 80-byte RenderElementData structs (std430 /
    C++ layout; see resource_mapping.md). The buffer is sized to
    render_element_max like the native element_buffer; slots past
    render_element_count carry zeroed structs exactly like a fresh native
    buffer region that was never written this frame."""
    out = bytearray()
    for record in elements:
        e = record["seed"]
        out += struct.pack(
            "<4I",
            record["type"],
            record["touches_near"],
            record["touches_far"],
            record["original_index"],
        )
        # Carry payload (raster-segment fields; ignored by the store kernel).
        transform_inv = [0.25 * (e + 1) + 0.125 * k for k in range(12)]
        scale = [1.0 + 0.5 * e, 2.0 + 0.25 * e, 3.0 + 0.125 * e]
        out += struct.pack("<12f", *transform_inv)
        out += struct.pack("<3f", *scale)
        out += struct.pack("<I", e)  # pad / has_wide_spot_angle
    out += bytes(80 * (render_element_max - len(elements)))
    assert len(out) == render_element_max * ELEMENT_STRIDE_BYTES
    return bytes(out)


def _usage_pairs(case: dict, consts: dict) -> list[tuple[int, int]]:
    """Deterministic (cluster, element) usage pairs for a case."""
    w, h = consts["cluster_screen_size"]
    count = consts["render_element_count"]
    case_id = case["case_id"]
    pairs: list[tuple[int, int]] = []
    for c in range(w * h):
        for e in range(count):
            if case_id == "store_pack_grid_4x3_e64":
                used = (e + 2 * c) % 3 == 0 and c != 7  # cluster 7 stays empty
            elif case_id == "store_pack_grid_2x2_minmax_merge":
                used = (e + c) % 2 == 0
            else:  # store_pack_grid_3x1_touch_overrides
                used = c != 2 and (e + c) % 2 == 0  # cluster 2 stays empty
            if used:
                pairs.append((c, e))
    return pairs


def _z_range(case: dict, c: int, e: int) -> int:
    """Deterministic z_range word for a used (cluster, element) pair."""
    case_id = case["case_id"]
    if case_id == "store_pack_grid_4x3_e64":
        if (3 * c + e) % 13 == 4:
            return 0  # exercise the z_range == 0 guard
        lo = (e + c) % 32
        hi = min(31, lo + ((e * 3 + c * 5) % 9))
        return (((1 << (hi + 1)) - 1) & ~((1 << lo) - 1)) & U32_MASK
    if case_id == "store_pack_grid_2x2_minmax_merge":
        # Shared slice band 8..15 for every element: same slices get many
        # min/max merges.
        return 0x0000FF00
    # store_pack_grid_3x1_touch_overrides: explicit per-element shapes.
    shapes = [
        1 << 0,          # single lowest slice
        1 << 31,         # single highest slice
        0xFFFFFFFF,      # full range
        1 << 15,         # single middle slice
        0,               # z_range == 0 guard
        0x00FF0000,      # mid band
        (1 << 4) | (1 << 20),  # sparse bits: from_z=4, to_z=21
        0x3,             # slices 0..1
        1 << 7,
        0xF0000000,      # slices 28..31
        0,               # another z_range == 0
        (1 << 10) | (1 << 11),
    ]
    return shapes[e % len(shapes)]


def build_cluster_render_words(case: dict, consts: dict) -> list[int]:
    """Build the synthetic cluster_render buffer: per cluster,
    max_render_element_count_div_32 usage-bitmap words followed by
    render_element_max z_range words (resource_mapping.md input layout)."""
    w, h = consts["cluster_screen_size"]
    crds = consts["cluster_render_data_size"]
    mrec_div_32 = consts["max_render_element_count_div_32"]
    words = [0] * (w * h * crds)
    for c, e in _usage_pairs(case, consts):
        src_offset = c * crds
        words[src_offset + (e >> 5)] |= 1 << (e & 31)
        words[src_offset + mrec_div_32 + e] = _z_range(case, c, e) & U32_MASK
    return words


def pack_words(words: list[int]) -> bytes:
    return struct.pack(f"<{len(words)}I", *words)


def build_b0(consts: dict) -> bytes:
    """The 8-dword (32-byte) ClusterStore::PushConstant mirror (descriptor
    layout dword order; see resource_mapping.md)."""
    w, h = consts["cluster_screen_size"]
    return struct.pack(
        "<8I",
        consts["cluster_render_data_size"],         # dword 0
        consts["max_render_element_count_div_32"],  # dword 1
        w,                                          # dword 2
        h,                                          # dword 3
        consts["render_element_count_div_32"],      # dword 4
        consts["max_cluster_element_count_div_32"], # dword 5
        0,                                          # dword 6 pad1
        0,                                          # dword 7 pad2
    )


def dst_word_count(consts: dict) -> int:
    w, h = consts["cluster_screen_size"]
    return w * h * (consts["max_cluster_element_count_div_32"] + 32) * ELEMENT_TYPE_MAX


# ---- CPU reference (pure u32 word math; bit-exact, zero tolerance) ---------


def cluster_store_reference(
    consts: dict, cluster_render_words: list[int], elements: list[dict]
) -> tuple[list[int], dict]:
    """Word-for-word port of cluster_store.glsl main (L46-119) over a zeroed
    destination (the native bake_cluster buffer_clear). Returns the expected
    cluster_store words plus branch-coverage counters."""
    w, h = consts["cluster_screen_size"]
    crds = consts["cluster_render_data_size"]
    mrec_div_32 = consts["max_render_element_count_div_32"]
    mcec_div_32 = consts["max_cluster_element_count_div_32"]
    count_div_32 = consts["render_element_count_div_32"]
    out = [0] * dst_word_count(consts)
    coverage = {
        "near_overrides": 0,
        "far_overrides": 0,
        "zero_z_range_skips": 0,
        "minmax_init_writes": 0,
        "minmax_merge_writes": 0,
        "single_slice_elements": 0,
        "multi_slice_elements": 0,
        "bitmap_bits_set": 0,
        "empty_clusters": 0,
        "elements_visited": 0,
    }
    for base_offset in range(w * h):
        src_offset = base_offset * crds
        cluster_empty = True
        for render_element_offset in range(count_div_32):
            bits = cluster_render_words[src_offset + render_element_offset]
            while bits != 0:
                cluster_empty = False
                index_bit = find_lsb(bits)
                index = render_element_offset * 32 + index_bit
                element = elements[index]
                coverage["elements_visited"] += 1
                z_range = cluster_render_words[src_offset + mrec_div_32 + index]
                if z_range != 0:
                    from_z = find_lsb(z_range)
                    to_z = find_msb(z_range) + 1
                    if element["touches_near"]:
                        from_z = 0
                        coverage["near_overrides"] += 1
                    if element["touches_far"]:
                        to_z = 32
                        coverage["far_overrides"] += 1
                    if to_z - from_z > 1:
                        coverage["multi_slice_elements"] += 1
                    else:
                        coverage["single_slice_elements"] += 1
                    dst_offset = (base_offset + element["type"] * (w * h)) * (mcec_div_32 + 32)
                    orig = element["original_index"]
                    for i in range(from_z, to_z):
                        slice_ofs = dst_offset + mcec_div_32 + i
                        minmax = out[slice_ofs]
                        if minmax == 0:
                            minmax = 0xFFFF  # min 0xFFFF, max 0
                            coverage["minmax_init_writes"] += 1
                        else:
                            coverage["minmax_merge_writes"] += 1
                        elem_min = min(orig, minmax & 0xFFFF)
                        elem_max = max(orig + 1, minmax >> 16)
                        out[slice_ofs] = (elem_min | (elem_max << 16)) & U32_MASK
                    word = dst_offset + (orig >> 5)
                    bit = 1 << (orig & 0x1F)
                    if out[word] & bit == 0:
                        coverage["bitmap_bits_set"] += 1
                    out[word] |= bit
                else:
                    coverage["zero_z_range_skips"] += 1
                bits &= ~(1 << index_bit) & U32_MASK
        if cluster_empty:
            coverage["empty_clusters"] += 1
    return out, coverage


REQUIRED_COVERAGE_KEYS = (
    "near_overrides",
    "far_overrides",
    "zero_z_range_skips",
    "minmax_init_writes",
    "minmax_merge_writes",
    "single_slice_elements",
    "multi_slice_elements",
    "bitmap_bits_set",
    "empty_clusters",
)


def build_case_doc(case: dict) -> tuple[dict, dict]:
    consts = case_constants(case)
    elements = build_elements(case)
    words = build_cluster_render_words(case, consts)
    expected, coverage = cluster_store_reference(consts, words, elements)
    element_bytes = pack_elements(elements, consts["render_element_max"])
    input_bytes = pack_words(words)
    expected_bytes = pack_words(expected)
    w, h = consts["cluster_screen_size"]
    samples = []
    for idx, word in enumerate(expected):
        if word != 0:
            samples.append({"word_index": idx, "value_hex": f"0x{word:08X}"})
        if len(samples) >= 8:
            break
    doc = {
        "case_id": case["case_id"],
        "note": case["note"],
        "constants": {
            "cluster_render_data_size": consts["cluster_render_data_size"],
            "max_render_element_count_div_32": consts["max_render_element_count_div_32"],
            "cluster_screen_size": [w, h],
            "render_element_count_div_32": consts["render_element_count_div_32"],
            "max_cluster_element_count_div_32": consts["max_cluster_element_count_div_32"],
            "render_element_count": consts["render_element_count"],
            "render_element_max": consts["render_element_max"],
        },
        "dispatch": [(w + 7) // 8, (h + 7) // 8, 1],
        "input_cluster_render_word_count": len(words),
        "input_cluster_render_u32_le_sha256": hashlib.sha256(input_bytes).hexdigest(),
        "input_elements_80b_le_sha256": hashlib.sha256(element_bytes).hexdigest(),
        "b0_sha256": hashlib.sha256(build_b0(consts)).hexdigest(),
        "expected_dst_word_count": len(expected),
        "expected_dst_nonzero_words": sum(1 for word in expected if word != 0),
        "cpu_expected_dst_u32_le_sha256": hashlib.sha256(expected_bytes).hexdigest(),
        "cpu_expected_sample_words": samples,
        "branch_coverage": coverage,
        "gpu_observed": None,
    }
    return doc, coverage


def main() -> int:
    cases = []
    total_coverage: dict[str, int] = {}
    for case in parity_cases():
        doc, coverage = build_case_doc(case)
        cases.append(doc)
        for key, value in coverage.items():
            total_coverage[key] = total_coverage.get(key, 0) + value
    missing = [key for key in REQUIRED_COVERAGE_KEYS if total_coverage.get(key, 0) == 0]
    if missing:
        raise SystemExit(
            f"fixture degenerated: required branch coverage is zero for {missing}"
        )
    evidence = {
        "pass_id": "cluster_store",
        "subject": "grx014_cluster_store_math_parity",
        "status": "pending_gpu_dispatch",
        "generated_at_utc": utc_now(),
        "math_status": MATH_PARITY_STATUS,
        "cpu_reference": {
            "formula": (
                "per cluster: src_offset = base_offset * cluster_render_data_size; "
                "scan render_element_count_div_32 usage words; per set bit "
                "(findLSB, cleared with bits &= ~(1 << bit)): read type + z_range "
                "word at src_offset + max_render_element_count_div_32 + index; if "
                "z_range != 0: from_z = findLSB(z_range), to_z = findMSB(z_range) "
                "+ 1, touches_near -> from_z = 0, touches_far -> to_z = 32; "
                "dst_offset = (base_offset + type * (w * h)) * "
                "(max_cluster_element_count_div_32 + 32); per slice: minmax == 0 "
                "-> 0xFFFF, elem_min = min(orig, minmax & 0xFFFF), elem_max = "
                "max(orig + 1, minmax >> 16), store elem_min | (elem_max << 16); "
                "then dst[dst_offset + (orig >> 5)] |= 1 << (orig & 0x1F). Pure "
                "u32 word math over a zeroed destination; bit-exact, zero "
                "tolerance"
            ),
            "hlsl_kernel": {"path": rel(HLSL_PATH), "sha256": sha256_of_file(HLSL_PATH)},
            "dxil": {"path": rel(DXIL_PATH), "sha256": sha256_of_file(DXIL_PATH)},
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_math_source": "external/godot-master/servers/rendering/renderer_rd/shaders/cluster_store.glsl (main, L46-119)",
        },
        "value_tolerance": 0,
        "tolerance_note": (
            "the kernel is pure u32 integer word math (no floats), so the GPU "
            "output must match the CPU reference EXACTLY, word for word"
        ),
        "gpu_dispatch_kind": None,
        "cases": cases,
        "coverage_totals": total_coverage,
        "notes": [
            "GPU-observed values are pending a real dispatch; the standalone S6 smoke "
            "(ci/grx014_cluster_store_d3d12_dispatch_smoke.py) imports this module's "
            "fixtures + reference and verifies every measured output word exactly.",
            "The destination is assumed zero-cleared (the native bake_cluster "
            "buffer_clear); the harness must upload explicit zeros before the dispatch.",
            "Fixtures carry every size/count through the 8-dword b0; the Godot default "
            "max_clustered_elements = 512 is a deployment assumption only (fixtures use "
            "smaller 32-aligned capacities), so a kernel with a hardcoded size cannot "
            "pass all three cases.",
            "The element buffer's carry payload (transform_inv/scale/pad) is nonzero "
            "garbage the store kernel must ignore; slots past render_element_count are "
            "zeroed like an unwritten native buffer region.",
        ],
        "does_not_imply": DOES_NOT_IMPLY,
    }
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(
        f"[grx014-math-parity] status=pending_gpu_dispatch cases={len(cases)} "
        f"coverage={total_coverage} evidence={EVIDENCE_PATH}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
