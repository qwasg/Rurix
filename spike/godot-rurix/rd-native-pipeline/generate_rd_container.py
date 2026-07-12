#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""generate_rd_container.py — Route B spike: build a Godot RenderingShaderContainerD3D12
container from existing Rurix offline artifacts (.dxil + .rts0.bin + descriptor_layout.json).

The output is byte-compatible with RenderingShaderContainer::from_bytes() +
RenderingShaderContainerD3D12 extra-data hooks as reverse-engineered in
container_format.md (all layout decisions are documented there with source
line references into external/godot-master).

Fail-closed: any mismatch between the descriptor layout JSON, the RTS0 blob
and the DXIL container aborts with a nonzero exit code. No GPU work is done.

Usage (defaults target the tonemap fixture):
  py -3 generate_rd_container.py [--pass-dir .../passes/tonemap/artifacts]
                                 [--dxil X.dxil] [--rts0 X.rts0.bin]
                                 [--layout X_descriptor_layout.json]
                                 [--name rurix_tonemap] [--out out/tonemap.rd_container.bin]
"""

import argparse
import json
import struct
import sys
import zlib
from pathlib import Path

# ---------------------------------------------------------------------------
# Constants mirrored from external/godot-master (see container_format.md)
# ---------------------------------------------------------------------------

CONTAINER_MAGIC = 0x43535247  # "GRSC" (rendering_shader_container.h:44)
CONTAINER_VERSION = 2  # rendering_shader_container.h:45
D3D12_FORMAT = 0x43443344  # "D3DC" (rendering_shader_container_d3d12.cpp:250-252)
D3D12_FORMAT_VERSION = 1  # rendering_shader_container_d3d12.h:80

PIPELINE_TYPE_COMPUTE = 1  # rendering_device_commons.h:694-698
SHADER_STAGE_COMPUTE = 4  # rendering_device_commons.h:589-600
SHADER_STAGE_COMPUTE_BIT = 1 << SHADER_STAGE_COMPUTE  # 0x10

# rendering_device_commons.h:650-665
UNIFORM_TYPE_SAMPLER = 0
UNIFORM_TYPE_SAMPLER_WITH_TEXTURE = 1
UNIFORM_TYPE_TEXTURE = 2
UNIFORM_TYPE_IMAGE = 3
UNIFORM_TYPE_UNIFORM_BUFFER = 7
UNIFORM_TYPE_STORAGE_BUFFER = 8

# rendering_shader_container_d3d12.h:55-60
RES_CLASS_INVALID = 0
RES_CLASS_CBV = 1
RES_CLASS_SRV = 2
RES_CLASS_UAV = 3

UINT32_MAX = 0xFFFFFFFF

# D3D12_ROOT_PARAMETER_TYPE
RP_DESCRIPTOR_TABLE = 0
RP_32BIT_CONSTANTS = 1
RP_CBV = 2
RP_SRV = 3
RP_UAV = 4

# D3D12_DESCRIPTOR_RANGE_TYPE
RANGE_SRV = 0
RANGE_UAV = 1
RANGE_CBV = 2
RANGE_SAMPLER = 3

RANGE_TYPE_TO_CLASS_CHAR = {RANGE_SRV: "t", RANGE_UAV: "u", RANGE_CBV: "b", RANGE_SAMPLER: "s"}

PSV_SHADER_KIND_COMPUTE = 5


def fail(msg):
    sys.stderr.write("[generate_rd_container] FAIL: %s\n" % msg)
    sys.exit(1)


def check(cond, msg):
    if not cond:
        fail(msg)


def align4(n):
    return (n + 3) & ~3


# ---------------------------------------------------------------------------
# DXBC / RTS0 / PSV0 parsing (read-only, offline)
# ---------------------------------------------------------------------------

def parse_dxbc_parts(blob, what):
    check(len(blob) >= 32, "%s: too small for a DXBC header" % what)
    check(blob[0:4] == b"DXBC", "%s: missing DXBC magic" % what)
    total = struct.unpack_from("<I", blob, 24)[0]
    check(total == len(blob), "%s: DXBC TotalSize %d != file size %d" % (what, total, len(blob)))
    count = struct.unpack_from("<I", blob, 28)[0]
    parts = []
    for i in range(count):
        off = struct.unpack_from("<I", blob, 32 + 4 * i)[0]
        check(off + 8 <= len(blob), "%s: part %d offset out of range" % (what, i))
        name = blob[off:off + 4].decode("ascii", "replace")
        size = struct.unpack_from("<I", blob, off + 4)[0]
        check(off + 8 + size <= len(blob), "%s: part %s payload out of range" % (what, name))
        parts.append((name, off + 8, size))
    return parts


def parse_rts0(rts0_blob):
    """Parse a serialized root signature blob (DXBC container wrapping one RTS0 part).

    Returns dict: version, flags, params = [
      {type, visibility, ...} with per-type payload; descriptor tables carry
      'ranges' = [{type,num,base_reg,space,slot}] where slot is the effective
      OffsetInDescriptorsFromTableStart with APPEND (0xFFFFFFFF) resolved.
    ]
    """
    parts = parse_dxbc_parts(rts0_blob, "rts0")
    rts0 = [p for p in parts if p[0] == "RTS0"]
    check(len(rts0) == 1, "rts0: expected exactly one RTS0 part, found %d" % len(rts0))
    base, size = rts0[0][1], rts0[0][2]
    p = base
    ver, num_params, params_off, num_samplers, samplers_off, flags = struct.unpack_from("<6I", rts0_blob, p)
    check(ver in (1, 2), "rts0: unsupported root signature version %d" % ver)
    check(num_samplers == 0, "rts0: static samplers unsupported by this spike generator")
    params = []
    for i in range(num_params):
        pd = p + params_off + 12 * i
        ptype, vis, data_off = struct.unpack_from("<3I", rts0_blob, pd)
        entry = {"index": i, "type": ptype, "visibility": vis}
        dd = p + data_off
        if ptype == RP_32BIT_CONSTANTS:
            reg, space, num32 = struct.unpack_from("<3I", rts0_blob, dd)
            entry.update(reg=reg, space=space, num32=num32)
        elif ptype == RP_DESCRIPTOR_TABLE:
            num_ranges, ranges_off = struct.unpack_from("<2I", rts0_blob, dd)
            ranges = []
            cursor = 0
            stride = 20 if ver == 1 else 24  # v1.1 ranges carry an extra Flags u32
            for j in range(num_ranges):
                rd = p + ranges_off + stride * j
                if ver == 1:
                    rtype, num, base_reg, space, tbl_off = struct.unpack_from("<5I", rts0_blob, rd)
                else:
                    rtype, num, base_reg, space, _flags, tbl_off = struct.unpack_from("<6I", rts0_blob, rd)
                slot = cursor if tbl_off == UINT32_MAX else tbl_off
                cursor = slot + num
                ranges.append({"type": rtype, "num": num, "base_reg": base_reg,
                               "space": space, "slot": slot})
            entry.update(ranges=ranges, descriptor_total=cursor)
        else:
            # Root descriptors (CBV/SRV/UAV): allowed in D3D12 but not used by
            # current Rurix artifacts; reject until a pass actually needs them.
            fail("rts0: root parameter %d has type %d (root descriptor) — unsupported by this spike" % (i, ptype))
        params.append(entry)
    check(size >= 24, "rts0: RTS0 payload truncated")
    return {"version": ver, "flags": flags, "params": params}


def parse_dxil_psv0(dxil_blob):
    """Extract (shader_kind, numthreads) from the PSV0 part. Fail-closed."""
    parts = parse_dxbc_parts(dxil_blob, "dxil")
    names = [p[0] for p in parts]
    check("DXIL" in names, "dxil: no DXIL part")
    check("RTS0" not in names, "dxil: embedded RTS0 part found — root signature source would be ambiguous")
    hash_bytes = dxil_blob[4:20]
    check(hash_bytes != b"\x00" * 16, "dxil: container hash is zero — blob is unsigned; PSO creation would fail")
    psv = [p for p in parts if p[0] == "PSV0"]
    check(len(psv) == 1, "dxil: expected exactly one PSV0 part")
    base, size = psv[0][1], psv[0][2]
    check(size >= 4, "dxil: PSV0 truncated")
    info_size = struct.unpack_from("<I", dxil_blob, base)[0]
    check(info_size >= 48, "dxil: PSV0 RuntimeInfoSize=%d < 48 (Info2); cannot extract numthreads" % info_size)
    check(4 + info_size <= size, "dxil: PSV0 RuntimeInfo out of range")
    s = base + 4
    shader_kind = dxil_blob[s + 24]
    ntx, nty, ntz = struct.unpack_from("<3I", dxil_blob, s + 36)
    return shader_kind, (ntx, nty, ntz), names


# ---------------------------------------------------------------------------
# Layout JSON -> reflection model
# ---------------------------------------------------------------------------

BINDING_KIND_MAP = {
    # binding_kind -> (uniform_type, writable, expected_range_type, res_class)
    "texture2d": (UNIFORM_TYPE_TEXTURE, 0, RANGE_SRV, RES_CLASS_SRV),
    "rwtexture2d": (UNIFORM_TYPE_IMAGE, 1, RANGE_UAV, RES_CLASS_UAV),
    # Raw buffer views (GRX-009 tracked-3a kernel model). The D3D12 driver
    # creates R32_TYPELESS RAW SRV/UAV buffer views for storage buffers
    # (rendering_device_driver_d3d12.cpp:3544-3565), matching Rurix raw views.
    "byteaddressbuffer": (UNIFORM_TYPE_STORAGE_BUFFER, 0, RANGE_SRV, RES_CLASS_SRV),
    "rwbyteaddressbuffer": (UNIFORM_TYPE_STORAGE_BUFFER, 1, RANGE_UAV, RES_CLASS_UAV),
    "cbuffer": (UNIFORM_TYPE_UNIFORM_BUFFER, 0, RANGE_CBV, RES_CLASS_CBV),
}


def build_reflection(layout, rts0):
    """Cross-check layout JSON against the parsed RTS0 and derive the per-set
    reflection rows. Single-set model (set 0) — matches all current Rurix passes."""
    params = rts0["params"]
    check(len(params) == int(layout.get("root_signature_parameters", len(params))),
          "layout root_signature_parameters=%s != rts0 param count %d"
          % (layout.get("root_signature_parameters"), len(params)))

    # --- push constants / root param 0 hard invariant --------------------
    # NOTE: layout "root_constants" is the number of named constants (entries),
    # NOT the dword count. The authoritative dword total is the sum of
    # root_constant_layout[].dword_size (tonemap: 2+2+1+1+1 = 7 dwords = 28B).
    rc_entries = layout.get("root_constant_layout", [])
    check(len(rc_entries) == int(layout.get("root_constants", len(rc_entries))),
          "root_constant_layout entry count != root_constants")
    root_constant_dwords = sum(int(e["dword_size"]) for e in rc_entries)
    for e in rc_entries:
        check(int(e.get("root_parameter_index", 0)) == 0,
              "root constant %r not on root parameter 0" % e.get("name"))
    # Cross-check against the pass mapping block if present (e.g. grx010_mapping).
    for key, val in layout.items():
        if key.endswith("_mapping") and isinstance(val, dict) and "root_constant_dwords" in val:
            check(int(val["root_constant_dwords"]) == root_constant_dwords,
                  "%s.root_constant_dwords=%s != derived %d" % (key, val["root_constant_dwords"], root_constant_dwords))
    push_constant_size = root_constant_dwords * 4
    param_cursor = 0
    if root_constant_dwords > 0:
        p0 = params[0]
        check(p0["type"] == RP_32BIT_CONSTANTS,
              "push constants declared but rts0 param[0] is not 32-bit constants; "
              "driver hardcodes root param 0 for push constants (rendering_device_driver_d3d12.cpp:4208)")
        check(p0["num32"] == root_constant_dwords,
              "rts0 param[0] num32=%d != layout root_constants=%d" % (p0["num32"], root_constant_dwords))
        check(p0["space"] == 0, "root constants must live in space0")
        param_cursor = 1
    else:
        check(all(p["type"] != RP_32BIT_CONSTANTS for p in params),
              "rts0 has 32-bit constants but layout declares none")

    # --- resource descriptor table ---------------------------------------
    tables = [p for p in params if p["type"] == RP_DESCRIPTOR_TABLE]
    check(len(tables) == 1, "expected exactly one descriptor table (Rurix single-table model), found %d" % len(tables))
    table = tables[0]
    check(table["index"] == param_cursor,
          "descriptor table at rts0 param %d, expected %d (contiguous after root constants)"
          % (table["index"], param_cursor))
    for r in table["ranges"]:
        check(r["type"] in (RANGE_SRV, RANGE_UAV, RANGE_CBV),
              "sampler ranges are unsupported by this spike generator")
        check(r["space"] == 0, "all Rurix resources must be space0")

    # --- match layout resources to ranges, assign RD bindings ------------
    resources = layout.get("resources", [])
    check(len(resources) == len(table["ranges"]),
          "layout resource count %d != rts0 range count %d" % (len(resources), len(table["ranges"])))

    by_key = {}
    for res in resources:
        key = (res["class"], int(res["register"]), int(res.get("space", 0)))
        check(key not in by_key, "duplicate resource %s" % (key,))
        by_key[key] = res

    uniforms = []
    for idx, r in enumerate(table["ranges"]):
        cls_char = RANGE_TYPE_TO_CLASS_CHAR[r["type"]]
        key = (cls_char, r["base_reg"], r["space"])
        check(key in by_key, "rts0 range %d (%s%d space%d) has no matching layout resource" % (idx, cls_char, r["base_reg"], r["space"]))
        res = by_key[key]
        kind = res.get("binding_kind")
        check(kind in BINDING_KIND_MAP, "unsupported binding_kind %r for %s" % (kind, res.get("name")))
        utype, writable, expect_range, res_class = BINDING_KIND_MAP[kind]
        check(expect_range == r["type"],
              "binding_kind %s expects range type %d but rts0 has %d" % (kind, expect_range, r["type"]))
        count = int(res.get("count", 1))
        check(count == r["num"], "resource %s count %d != range num %d" % (res.get("name"), count, r["num"]))
        # RD binding policy: binding index == range order == table slot order.
        uniforms.append({
            "name": res.get("name"),
            "binding": idx,
            "type": utype,
            "stages": SHADER_STAGE_COMPUTE_BIT,
            "length": count if utype in (UNIFORM_TYPE_TEXTURE, UNIFORM_TYPE_IMAGE) else 0,
            "writable": writable,
            "res_class": res_class,
            "resource_descriptor_offset": r["slot"],
            "register": "%s%d" % (cls_char, r["base_reg"]),
        })

    bindings_sorted = [u["binding"] for u in uniforms]
    check(bindings_sorted == sorted(bindings_sorted), "bindings must be strictly ascending within the set")

    return {
        "push_constant_size": push_constant_size,
        "push_root_param_idx": 0 if root_constant_dwords else UINT32_MAX,
        "resource_root_param_idx": table["index"],
        "resource_descriptor_count": table["descriptor_total"],
        "uniforms": uniforms,
    }


# ---------------------------------------------------------------------------
# Container writer (mirror of to_bytes(), see container_format.md section 1)
# ---------------------------------------------------------------------------

def build_container(shader_name, dxil, rts0_bytes, refl, local_size):
    name_bytes = shader_name.encode("utf-8")
    out = bytearray()

    # ContainerHeader (20B)
    out += struct.pack("<5I", CONTAINER_MAGIC, CONTAINER_VERSION, D3D12_FORMAT,
                       D3D12_FORMAT_VERSION, 1)

    # ReflectionData (64B incl. 4B tail padding)
    out += struct.pack(
        "<Q13Ii",
        0,                      # vertex_input_mask
        0,                      # fragment_output_mask
        0,                      # specialization_constants_count
        PIPELINE_TYPE_COMPUTE,  # pipeline_type
        0,                      # has_multiview
        0,                      # has_dynamic_buffers
        local_size[0], local_size[1], local_size[2],
        1,                      # set_count
        refl["push_constant_size"],
        SHADER_STAGE_COMPUTE_BIT if refl["push_constant_size"] else 0,
        1,                      # stage_count
        len(name_bytes),        # shader_name_len
        0,                      # struct tail padding
    )

    # ReflectionDataD3D12 (12B)
    out += struct.pack("<3I",
                       0,  # spirv_specialization_constants_ids_mask
                       SHADER_STAGE_COMPUTE_BIT if refl["push_constant_size"] else 0,
                       UINT32_MAX)  # nir_runtime_data_root_param_idx

    # ReflectionBindingSetDataD3D12 x set_count (16B each)
    out += struct.pack("<4I",
                       refl["resource_root_param_idx"],
                       refl["resource_descriptor_count"],
                       UINT32_MAX,  # sampler_root_param_idx (no samplers)
                       0)           # sampler_descriptor_count

    # shader name + align absolute offset to 4
    out += name_bytes
    out += b"\x00" * (align4(len(out)) - len(out))

    # set 0: uniforms_count + interleaved (base 20B + d3d12 24B) per uniform
    out += struct.pack("<I", len(refl["uniforms"]))
    for u in refl["uniforms"]:
        out += struct.pack("<5I", u["type"], u["binding"], u["stages"], u["length"], u["writable"])
        out += struct.pack("<6I", u["res_class"], 0, SHADER_STAGE_COMPUTE_BIT,
                           u["resource_descriptor_offset"], UINT32_MAX, UINT32_MAX)

    # specialization constants: none.

    # stages array
    out += struct.pack("<I", SHADER_STAGE_COMPUTE)

    # shader entry: ShaderHeader + raw DXIL (uncompressed), align to 4
    out += struct.pack("<4I", SHADER_STAGE_COMPUTE, len(dxil), 0, len(dxil))
    out += dxil
    out += b"\x00" * (align4(len(out)) - len(out))

    # footer: ContainerFooterD3D12 + root signature bytes verbatim
    crc = zlib.crc32(rts0_bytes) & 0xFFFFFFFF
    out += struct.pack("<2I", len(rts0_bytes), crc)
    out += rts0_bytes

    return bytes(out), crc


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main():
    here = Path(__file__).resolve().parent
    default_pass = here.parent / "passes" / "tonemap" / "artifacts"

    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--pass-dir", type=Path, default=default_pass,
                    help="pass artifacts dir (default: tonemap fixture)")
    ap.add_argument("--dxil", type=Path, default=None)
    ap.add_argument("--rts0", type=Path, default=None)
    ap.add_argument("--layout", type=Path, default=None)
    ap.add_argument("--name", default=None, help="shader name stored in the container")
    ap.add_argument("--out", type=Path, default=None)
    args = ap.parse_args()

    pass_dir = args.pass_dir
    stem = pass_dir.parent.name if pass_dir.name == "artifacts" else pass_dir.name
    dxil_path = args.dxil or (pass_dir / ("%s.dxil" % stem))
    rts0_path = args.rts0 or (pass_dir / ("%s.rts0.bin" % stem))
    layout_path = args.layout or (pass_dir / ("%s_descriptor_layout.json" % stem))
    out_path = args.out or (here / "out" / ("%s.rd_container.bin" % stem))

    for p in (dxil_path, rts0_path, layout_path):
        check(p.is_file(), "missing input: %s" % p)

    dxil = dxil_path.read_bytes()
    rts0_bytes = rts0_path.read_bytes()
    layout = json.loads(layout_path.read_bytes().decode("utf-8"))

    shader_kind, local_size, dxil_parts = parse_dxil_psv0(dxil)
    check(shader_kind == PSV_SHADER_KIND_COMPUTE,
          "dxil PSV0 shader kind %d != compute(%d)" % (shader_kind, PSV_SHADER_KIND_COMPUTE))

    rts0 = parse_rts0(rts0_bytes)
    refl = build_reflection(layout, rts0)

    name = args.name or ("rurix_%s" % layout.get("pass_id", stem))
    container, crc = build_container(name, dxil, rts0_bytes, refl, local_size)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(container)

    report = {
        "inputs": {
            "dxil": str(dxil_path), "dxil_size": len(dxil), "dxil_parts": dxil_parts,
            "rts0": str(rts0_path), "rts0_size": len(rts0_bytes),
            "rts0_version": rts0["version"],
            "layout": str(layout_path),
        },
        "container": {
            "path": str(out_path), "size": len(container), "shader_name": name,
            "format": "0x%08X" % D3D12_FORMAT, "format_version": D3D12_FORMAT_VERSION,
            "pipeline_type": "compute", "compute_local_size": list(local_size),
            "push_constant_size": refl["push_constant_size"],
            "root_signature_crc": "0x%08X" % crc,
        },
        "set0": {
            "resource_root_param_idx": refl["resource_root_param_idx"],
            "resource_descriptor_count": refl["resource_descriptor_count"],
            "sampler_root_param_idx": "UINT32_MAX",
            "uniforms": refl["uniforms"],
        },
    }
    report_path = out_path.with_suffix(".report.json")
    report_path.write_bytes((json.dumps(report, indent=2, ensure_ascii=False) + "\n").encode("utf-8"))

    print("[generate_rd_container] OK: %s (%d bytes)" % (out_path, len(container)))
    print("[generate_rd_container] report: %s" % report_path)
    for u in refl["uniforms"]:
        print("  set0 binding %d <- %s (%s) type=%d writable=%d slot=%d"
              % (u["binding"], u["register"], u["name"], u["type"], u["writable"],
                 u["resource_descriptor_offset"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
