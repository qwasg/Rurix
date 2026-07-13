#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""verify_container.py — structural self-check for containers built by
generate_rd_container.py, written as an INDEPENDENT re-implementation of the
Godot read path (it does not import the generator).

Three layers of checks, each labelled with the consuming source location in
external/godot-master:

  A. RenderingShaderContainer::from_bytes() replay (rendering_shader_container.cpp:751-849)
     including the D3D12 extra-data hooks; every ERR_FAIL condition is mirrored,
     ending with the exact-length invariant (:847).
  B. shader_create_from_container consumer view (rendering_device_driver_d3d12.cpp:3267-3352):
     dump the values the driver would copy into ShaderInfo and assert their
     invariants (root param 0 for push constants, UINT32_MAX sentinels, ...).
  C. Cross-artifact consistency: embedded DXIL == input .dxil, footer bytes ==
     input .rts0.bin, crc == zlib.crc32, reflection offsets == RTS0 table slots,
     compute_local_size == PSV0 numthreads.

Callable API for the S1 batch smoke: ``verify_container_file(container, dxil,
rts0, resources)`` returns ``(check_count, failures)`` where ``resources`` is the
effective per-kernel layout resource list (variant kernels pull it from
``layout["variants"][i]["resources"]``). ``main()`` keeps the single-file CLI
(default: the tonemap fixture).

No GPU, no engine. Exit 0 = all checks pass.
"""

import argparse
import json
import struct
import sys
import zlib
from pathlib import Path

CONTAINER_MAGIC = 0x43535247
CONTAINER_VERSION = 2
D3D12_FORMAT = 0x43443344
D3D12_FORMAT_VERSION = 1
PIPELINE_TYPE_COMPUTE = 1
SHADER_STAGE_COMPUTE = 4
SHADER_STAGE_COMPUTE_BIT = 1 << SHADER_STAGE_COMPUTE
UINT32_MAX = 0xFFFFFFFF
RP_DESCRIPTOR_TABLE = 0
RP_32BIT_CONSTANTS = 1

# Per-run check state (reset by verify_container_file). Single-threaded, so
# module-level state is fine and keeps the source-anchored check() labels intact.
_STATE = {"failures": [], "count": 0, "quiet": False}


def check(cond, label):
    _STATE["count"] += 1
    if not _STATE["quiet"] or not cond:
        status = "ok " if cond else "FAIL"
        print("  [%s] %s" % (status, label))
    if not cond:
        _STATE["failures"].append(label)


def align4(n):
    return (n + 3) & ~3


def u32(b, off):
    return struct.unpack_from("<I", b, off)[0]


# --- layer A: from_bytes replay --------------------------------------------

def replay_from_bytes(b):
    """Mirror rendering_shader_container.cpp:751-849 + d3d12 hooks. Returns model dict."""
    m = {}
    off = 0
    check(len(b) >= off + 20, "header: enough bytes for ContainerHeader (:757)")
    magic, version, fmt, fmt_ver, shader_count = struct.unpack_from("<5I", b, off)
    off += 20  # + header extra (d3d12: none)
    check(magic == CONTAINER_MAGIC, "header: magic 0x%08X == GRSC (:762)" % magic)
    check(version <= CONTAINER_VERSION, "header: version %d <= %d (:763)" % (version, CONTAINER_VERSION))
    check(fmt == D3D12_FORMAT, "header: format 0x%08X == D3DC (:764)" % fmt)
    check(fmt_ver <= D3D12_FORMAT_VERSION, "header: format_version %d <= %d (:765)" % (fmt_ver, D3D12_FORMAT_VERSION))
    m["shader_count"] = shader_count

    check(len(b) >= off + 64, "reflection: enough bytes for ReflectionData (:771)")
    (vim, fom, sc_count, ptype, mview, dynbuf,
     ls0, ls1, ls2, set_count, pc_size, pc_stages, stage_count, name_len,
     _pad) = struct.unpack_from("<Q13Ii", b, off)
    off += 64
    m.update(vertex_input_mask=vim, fragment_output_mask=fom,
             specialization_constants_count=sc_count, pipeline_type=ptype,
             has_multiview=mview, has_dynamic_buffers=dynbuf,
             compute_local_size=(ls0, ls1, ls2), set_count=set_count,
             push_constant_size=pc_size, push_constant_stages_mask=pc_stages,
             stage_count=stage_count, shader_name_len=name_len, refl_padding=_pad)

    # d3d12 _from_bytes_reflection_extra_data (d3d12.cpp:258-265)
    sc_ids_mask, dxil_pc_stages, nir_rt_idx = struct.unpack_from("<3I", b, off)
    off += 12
    sets_d3d12 = []
    for _ in range(set_count):
        sets_d3d12.append(struct.unpack_from("<4I", b, off))
        off += 16
    m.update(spirv_specialization_constants_ids_mask=sc_ids_mask,
             dxil_push_constant_stages=dxil_pc_stages,
             nir_runtime_data_root_param_idx=nir_rt_idx, sets_d3d12=sets_d3d12)

    check(len(b) >= off + name_len, "name: enough bytes for shader name (:777)")
    m["shader_name"] = b[off:off + name_len].decode("utf-8", "replace")
    if name_len > 0:
        off = align4(off + name_len)  # (:782)

    uniform_sets = []
    for si in range(set_count):
        check(len(b) >= off + 4, "set %d: uniforms_count readable (:792)" % si)
        ucount = u32(b, off)
        off += 4
        uniforms = []
        for ui in range(ucount):
            check(len(b) >= off + 20, "set %d uniform %d: base data readable (:801)" % (si, ui))
            base = struct.unpack_from("<5I", b, off)
            off += 20
            extra = struct.unpack_from("<6I", b, off)  # d3d12.cpp:272-275
            off += 24
            uniforms.append({"type": base[0], "binding": base[1], "stages": base[2],
                             "length": base[3], "writable": base[4],
                             "res_class": extra[0], "has_sampler": extra[1],
                             "dxil_stages": extra[2], "resource_descriptor_offset": extra[3],
                             "sampler_descriptor_offset": extra[4], "root_param_idx": extra[5]})
        uniform_sets.append(uniforms)
    m["uniform_sets"] = uniform_sets

    m["spec_constants"] = []
    for i in range(sc_count):
        base = struct.unpack_from("<4I", b, off)
        off += 16
        offsets = struct.unpack_from("<3Q", b, off)  # d3d12.cpp:282-285
        off += 24
        m["spec_constants"].append((base, offsets))

    stages = []
    if stage_count > 0:
        check(len(b) >= off + 4 * stage_count, "stages: array readable (:821)")
        stages = list(struct.unpack_from("<%dI" % stage_count, b, off))
        off += 4 * stage_count
    m["stages"] = stages

    shaders = []
    for i in range(shader_count):
        check(len(b) >= off + 16, "shader %d: header readable (:830)" % i)
        stage, csize, cflags, dsize = struct.unpack_from("<4I", b, off)
        off += 16
        check(len(b) >= off + csize, "shader %d: code readable (:834)" % i)
        code = b[off:off + csize]
        off = align4(off + csize)  # (:841)
        shaders.append({"stage": stage, "flags": cflags, "decompressed_size": dsize, "code": code})
    m["shaders"] = shaders

    # d3d12 footer (d3d12.cpp:287-293)
    check(len(b) >= off + 8, "footer: ContainerFooterD3D12 readable")
    rs_len, rs_crc = struct.unpack_from("<2I", b, off)
    off += 8
    check(len(b) >= off + rs_len, "footer: root signature bytes readable")
    m["root_signature_bytes"] = b[off:off + rs_len]
    m["root_signature_crc"] = rs_crc
    off += rs_len

    check(off == len(b), "EXACT length: consumed %d == container size %d (:847)" % (off, len(b)))
    return m


# --- RTS0 mini-parser (independent copy for cross-check) --------------------

def parse_rts0(blob):
    assert blob[0:4] == b"DXBC"
    count = u32(blob, 28)
    part = None
    for i in range(count):
        off = u32(blob, 32 + 4 * i)
        if blob[off:off + 4] == b"RTS0":
            part = off + 8
    assert part is not None, "no RTS0 part"
    p = part
    ver, num_params, params_off, num_samplers, _so, flags = struct.unpack_from("<6I", blob, p)
    params = []
    for i in range(num_params):
        ptype, vis, doff = struct.unpack_from("<3I", blob, p + params_off + 12 * i)
        dd = p + doff
        e = {"type": ptype, "visibility": vis}
        if ptype == RP_32BIT_CONSTANTS:
            e["reg"], e["space"], e["num32"] = struct.unpack_from("<3I", blob, dd)
        elif ptype == RP_DESCRIPTOR_TABLE:
            nr, ro = struct.unpack_from("<2I", blob, dd)
            stride = 20 if ver == 1 else 24
            ranges = []
            cursor = 0
            for j in range(nr):
                rd = p + ro + stride * j
                vals = struct.unpack_from("<%dI" % (stride // 4), blob, rd)
                tbl_off = vals[-1]
                slot = cursor if tbl_off == UINT32_MAX else tbl_off
                cursor = slot + vals[1]
                ranges.append({"type": vals[0], "num": vals[1], "base_reg": vals[2],
                               "space": vals[3], "slot": slot})
            e["ranges"] = ranges
            e["descriptor_total"] = cursor
        params.append(e)
    return {"version": ver, "num_samplers": num_samplers, "flags": flags, "params": params}


def parse_psv0_numthreads(dxil):
    count = u32(dxil, 28)
    for i in range(count):
        off = u32(dxil, 32 + 4 * i)
        if dxil[off:off + 4] == b"PSV0":
            base = off + 8
            info_size = u32(dxil, base)
            if info_size < 48:
                return None, None
            s = base + 4
            return dxil[s + 24], struct.unpack_from("<3I", dxil, s + 36)
    return None, None


# --- reusable verification body --------------------------------------------

def verify_container_file(container_path, dxil_path, rts0_path, resources, quiet=False):
    """Structurally self-check one container against its source artifacts.

    ``resources`` = the effective per-kernel layout resource list (for variant
    kernels this is layout["variants"][i]["resources"]). Returns
    ``(check_count, failures)``; ``failures`` is empty iff every check passed."""
    _STATE["failures"] = []
    _STATE["count"] = 0
    _STATE["quiet"] = quiet

    b = container_path.read_bytes()
    dxil = dxil_path.read_bytes()
    rts0_bytes = rts0_path.read_bytes()

    if not quiet:
        print("== A. from_bytes replay (%s, %d bytes)" % (container_path.name, len(b)))
    m = replay_from_bytes(b)

    if not quiet:
        print("== B. shader_create_from_container consumer view (:3267-3352)")
    check(m["pipeline_type"] == PIPELINE_TYPE_COMPUTE,
          "pipeline_type == COMPUTE (RD gate rendering_device.cpp:4626)")
    check(m["stage_count"] == 1 and m["stages"] == [SHADER_STAGE_COMPUTE],
          "single compute stage (stages=%s)" % m["stages"])
    check(m["shader_count"] == 1 and m["shaders"][0]["stage"] == SHADER_STAGE_COMPUTE,
          "one shader entry, stage=COMPUTE (:3319-3333)")
    check(m["specialization_constants_count"] == 0 and m["spirv_specialization_constants_ids_mask"] == 0,
          "zero specialization constants -> DXIL never patched/re-signed (:3238,:3242)")
    check(m["nir_runtime_data_root_param_idx"] == UINT32_MAX,
          "nir_runtime_data_root_param_idx == UINT32_MAX (compute; d3d12.cpp:903)")
    check(m["refl_padding"] == 0, "ReflectionData tail padding is zero")
    pc = m["push_constant_size"]
    if pc:
        check(m["dxil_push_constant_stages"] == SHADER_STAGE_COMPUTE_BIT,
              "dxil_push_constant_stages == COMPUTE_BIT -> dxil_push_constant_size set (:3274-3276)")
        check(m["push_constant_stages_mask"] == SHADER_STAGE_COMPUTE_BIT,
              "base push_constant_stages_mask == COMPUTE_BIT")
        check(pc % 4 == 0 and pc <= 128,
              "push_constant_size %d: dword-aligned and <= 128 (rendering_device.cpp:6101)" % pc)
    check(m["set_count"] == 1 and len(m["sets_d3d12"]) == 1, "single uniform set")
    (res_rp, res_count, samp_rp, samp_count) = m["sets_d3d12"][0]
    check(samp_rp == UINT32_MAX and samp_count == 0,
          "no sampler table: sampler_root_param_idx sentinel honored (:5423)")
    uniforms = m["uniform_sets"][0]
    binds = [u["binding"] for u in uniforms]
    check(binds == sorted(binds) and len(set(binds)) == len(binds),
          "bindings strictly ascending within set (writer sort d3d12.cpp:888-897)")
    offsets = sorted(u["resource_descriptor_offset"] for u in uniforms)
    check(offsets == list(range(len(uniforms))) and res_count == len(uniforms),
          "descriptor offsets form 0..N-1 and match resource_descriptor_count (:3288,:3302)")
    for u in uniforms:
        check(u["dxil_stages"] == SHADER_STAGE_COMPUTE_BIT,
              "binding %d dxil_stages == COMPUTE_BIT (barrier deduction :3672+)" % u["binding"])
        check(u["root_param_idx"] == UINT32_MAX and u["sampler_descriptor_offset"] == UINT32_MAX,
              "binding %d root descriptor / sampler sentinels" % u["binding"])

    if not quiet:
        print("== C. cross-artifact consistency")
    check(m["shaders"][0]["flags"] == 0 and m["shaders"][0]["code"] == dxil,
          "embedded shader code == %s byte-for-byte, uncompressed" % dxil_path.name)
    check(m["shaders"][0]["decompressed_size"] == len(dxil),
          "code_decompressed_size == dxil size (decompress memcpy path :983-990)")
    check(m["root_signature_bytes"] == rts0_bytes,
          "footer root signature == %s byte-for-byte (CreateRootSignature input :3342)" % rts0_path.name)
    check(m["root_signature_crc"] == (zlib.crc32(rts0_bytes) & 0xFFFFFFFF),
          "root_signature_crc == zlib.crc32(rts0) (writer d3d12.cpp:761-762)")

    rts0 = parse_rts0(rts0_bytes)
    check(rts0["num_samplers"] == 0, "rts0 has no static samplers")
    if pc:
        p0 = rts0["params"][0]
        check(p0["type"] == RP_32BIT_CONSTANTS and p0["num32"] * 4 == pc,
              "RTS0 param[0] is 32-bit constants of %d dwords (hardcoded bind :4208)" % (pc // 4))
    tbl = rts0["params"][res_rp]
    check(tbl["type"] == RP_DESCRIPTOR_TABLE,
          "reflection resource_root_param_idx=%d points at a descriptor table" % res_rp)
    check(tbl["descriptor_total"] == res_count,
          "RTS0 table descriptor total %d == resource_descriptor_count %d" % (tbl["descriptor_total"], res_count))
    # Expand each RTS0 range slot-by-slot so a collapsed multi-descriptor range
    # (e.g. taa_resolve SRV x5 = t0..t4) still pairs one slot to one uniform.
    slot_to_range = {}
    for r in tbl["ranges"]:
        for k in range(r["num"]):
            slot_to_range[r["slot"] + k] = r
    for u in uniforms:
        r = slot_to_range.get(u["resource_descriptor_offset"])
        check(r is not None,
              "binding %d offset %d matches an RTS0 range slot" % (u["binding"], u["resource_descriptor_offset"]))
        if r is not None:
            expect_class = {0: 2, 1: 3, 2: 1}[r["type"]]  # SRV->RES_CLASS_SRV(2), UAV->3, CBV->1
            check(u["res_class"] == expect_class,
                  "binding %d res_class %d matches RTS0 range type %d (view creation :3483/:3493)"
                  % (u["binding"], u["res_class"], r["type"]))

    kind, numthreads = parse_psv0_numthreads(dxil)
    check(kind == 5, "DXIL PSV0 shader kind == compute")
    check(numthreads == m["compute_local_size"],
          "compute_local_size %s == PSV0 numthreads %s (dispatch_threads math)"
          % (m["compute_local_size"], numthreads))
    check(dxil[4:20] != b"\x00" * 16, "DXIL container hash nonzero (signed; PSO gate)")

    n_res = len(resources)
    check(len(uniforms) == n_res, "uniform count %d == layout resources %d" % (len(uniforms), n_res))

    if not quiet:
        print()
        print("consumer view (values ShaderInfo would receive):")
        print("  shader_name=%r local_size=%s push_constant=%dB crc=0x%08X"
              % (m["shader_name"], m["compute_local_size"], pc, m["root_signature_crc"]))
        print("  set0: resource_root_param_idx=%d resource_descriptor_count=%d" % (res_rp, res_count))
        for u in uniforms:
            print("    binding=%d type=%d writable=%d length=%d res_class=%d desc_offset=%d"
                  % (u["binding"], u["type"], u["writable"], u["length"], u["res_class"],
                     u["resource_descriptor_offset"]))

    return _STATE["count"], list(_STATE["failures"])


# --- main -------------------------------------------------------------------

def main():
    here = Path(__file__).resolve().parent
    default_pass = here.parent / "passes" / "tonemap" / "artifacts"
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--container", type=Path, default=here / "out" / "tonemap.rd_container.bin")
    ap.add_argument("--dxil", type=Path, default=default_pass / "tonemap.dxil")
    ap.add_argument("--rts0", type=Path, default=default_pass / "tonemap.rts0.bin")
    ap.add_argument("--layout", type=Path, default=default_pass / "tonemap_descriptor_layout.json")
    args = ap.parse_args()

    layout = json.loads(args.layout.read_bytes().decode("utf-8"))
    resources = layout.get("resources", [])
    count, failures = verify_container_file(args.container, args.dxil, args.rts0, resources)

    print()
    if failures:
        print("[verify_container] FAIL: %d/%d checks failed" % (len(failures), count))
        for f in failures:
            print("  - %s" % f)
        return 1
    print("[verify_container] PASS: %d/%d checks" % (count, count))
    return 0


if __name__ == "__main__":
    sys.exit(main())
