#!/usr/bin/env python3
"""GRX-015: compile the math-equivalent HLSL gpu_culling kernel.

Compiles ``artifacts/hlsl_bridge/gpu_culling_frustum_count.hlsl`` via DXC
(cs_6_0), validates it with DXV, emits the descriptor layout JSON (per-slot
structured_buffer / rwstructured_buffer / rwstructured_buffer binding kinds +
the 144-byte / 36-dword Rurix-defined root-constant layout), and synthesizes a
Rurix-owned RTS0 root signature through ``cargo run --example
emit_grx015_gpu_culling_rts0`` (``rurixc::binding_layout::
{infer_root_signature, pack_root_constants, serialize_rts0}``).

Route rationale (gpu_culling is an all raw-buffer / SSBO pass, so the GRX-009
texture-intrinsic llc blocker does NOT apply): a rurixc-owned rx -> DXIL
compile of the culling kernel is infeasible because (1) the DXIL compute-body
lowering accepts only ``View<global, f32>`` / ``ViewMut<global, f32>`` buffer
views (src/rurixc/src/dxil_codegen.rs ~L1750-1790) while both outputs (command
dwords, visibility bitmask words) are u32 SSBOs whose bit patterns an f32 view
cannot carry bit-faithfully; (2) the Rurix lang subset has NO atomic intrinsic
(InterlockedAdd / InterlockedOr) on any backend, and the kernel's only writes
are atomics; (3) the integer bit operations the bitmask write needs (``<<``,
``&``, ``>>``) exist in MIR (``BinOp::BitAnd/BitOr/BitXor/Shl/Shr``) but have
no DXIL-backend lowering; and (4) the conservative-radius ``sqrt`` has no DXIL
lowering (``DeviceMathFn`` is NVPTX-libdevice-only). The RTS0 is Rurix-owned;
the DXIL/descriptor package is the owner-approved ``hlsl_bridge_workaround``
(precedent: ``../cluster_store/PASS_CONTRACT.md`` sec 5.3 /
``../particles_copy/PASS_CONTRACT.md`` sec 5.3 /
``../luminance_reduction/texture_artifact_provenance_policy.json``).

Tool discovery follows the GRX-009..014 template (``RURIX_DXC_DIR`` /
``RURIX_DXC_NEW_DIR`` env, then the default round-7 extraction dir, then PATH).

Fail-closed: this is an ``hlsl_bridge_workaround`` artifact set, NOT
rurix_owned. It never advances ``runtime_state``/``real_gpu_pass`` and the pass
stays default disabled; math parity stays pending until
``math_parity_evidence.json`` records a measured GPU comparison.
"""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys


PASS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = PASS_DIR.parents[3]
ARTIFACTS_DIR = PASS_DIR / "artifacts"
BRIDGE_DIR = ARTIFACTS_DIR / "hlsl_bridge"
HLSL_PATH = BRIDGE_DIR / "gpu_culling_frustum_count.hlsl"
BRIDGE_DXIL_PATH = BRIDGE_DIR / "gpu_culling_frustum_count.dxil"
CANONICAL_DXIL_PATH = ARTIFACTS_DIR / "gpu_culling.dxil"
CANONICAL_RTS0_PATH = ARTIFACTS_DIR / "gpu_culling.rts0.bin"
CANONICAL_DESCRIPTOR_PATH = ARTIFACTS_DIR / "gpu_culling_descriptor_layout.json"
EVIDENCE_PATH = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"
DEFAULT_DXC_DIR = pathlib.Path(r"H:\dxc-round7\extracted\bin\x64")

TARGET_PROFILE = "cs_6_0"
ENTRY_POINT = "main"

MATH_PARITY_STATUS = "gpu_culling_cpu_reference_proven_pending_gpu_dispatch"

# Canonical GRX-015 gpu_culling root-constant layout: 144 bytes / 36 dwords at
# root_parameter_index 0. This layout is RURIX-DEFINED (gpu_culling is an
# additive pass; no Godot push constant exists to mirror — contrast with the
# GRX-013/014 CopyPushConstant / ClusterStore::PushConstant mirrors). uint32
# fields are carried as 1-dword slots (the RTS0 only encodes dword layout, the
# "type" here is the true semantic type for the descriptor JSON). The scalar
# tail packs tightly in HLSL cbuffer rules (no field straddles a 16-byte
# vector boundary), so the cbuffer dword offsets equal this table exactly.
def _rc(name: str, type_: str, order: int) -> dict[str, object]:
    return {
        "name": name,
        "type": type_,
        "order": order,
        "root_parameter_index": 0,
        "dword_offset": order,
        "dword_size": 1,
    }


ROOT_CONSTANT_LAYOUT = [
    entry
    for plane in range(6)
    for entry in (
        _rc(f"frustum_plane_{plane}_nx", "f32", plane * 4 + 0),
        _rc(f"frustum_plane_{plane}_ny", "f32", plane * 4 + 1),
        _rc(f"frustum_plane_{plane}_nz", "f32", plane * 4 + 2),
        _rc(f"frustum_plane_{plane}_d", "f32", plane * 4 + 3),
    )
] + [
    _rc("instance_count", "u32", 24),
    _rc("motion_vectors_current_offset", "u32", 25),
    _rc("transform_stride_floats", "u32", 26),
    _rc("surface_count", "u32", 27),
    _rc("command_stride_dwords", "u32", 28),
    _rc("instance_count_dword_index", "u32", 29),
    _rc("mesh_bound_center_local_x", "f32", 30),
    _rc("mesh_bound_center_local_y", "f32", 31),
    _rc("mesh_bound_center_local_z", "f32", 32),
    _rc("mesh_bound_radius_local", "f32", 33),
    _rc("pad1", "u32", 34),
    _rc("pad2", "u32", 35),
]

RESOURCES = [
    {
        "name": "src_transforms",
        "class": "t",
        "register": 0,
        "space": 0,
        "count": 1,
        "hlsl_type": "StructuredBuffer<float>",
        "binding_kind": "structured_buffer",
    },
    {
        "name": "dst_commands",
        "class": "u",
        "register": 0,
        "space": 0,
        "count": 1,
        "hlsl_type": "RWStructuredBuffer<uint>",
        "binding_kind": "rwstructured_buffer",
    },
    {
        "name": "dst_visibility",
        "class": "u",
        "register": 1,
        "space": 0,
        "count": 1,
        "hlsl_type": "RWStructuredBuffer<uint>",
        "binding_kind": "rwstructured_buffer",
    },
]

KNOWN_GAPS = [
    "conservative bounding-sphere test only (local AABB -> sphere, Frobenius-norm radius bound): may keep truly-invisible instances visible, never over-culls; precise OBB / transformed-AABB test out of scope",
    "occlusion culling, LOD selection, hierarchical/two-phase culling unsupported",
    "2D transform format (MULTIMESH_TRANSFORM_2D, 8-float stride) unsupported; color/custom stride variants carried via transform_stride_floats in b0 but exercised at 12 (bare 3D) only",
    "motion_vectors_current_offset carried in b0 but exercised at 0 (motion-vector double-buffer offset is a runtime concern)",
    "per-surface differing visibility unsupported (every surface command block receives the same count, matching the native CPU write loop)",
    "visible-instance compaction / transform remap is GRX-016; indirect-args generation beyond the instance-count dword is GRX-018 (separate milestones, separate PRs)",
    "Resource-layer use_indirect plumbing out of scope (scene/resources/multimesh.h has no such property); the later patch slices use the RS::multimesh_allocate_data(..., use_indirect=true) server-API bypass",
    "the CPU _scene_cull path is untouched: a MultiMesh occupies ONE InstanceData in the CPU cull, so this pass does not reduce the CPU cull O(N) over independent MeshInstance3D populations; the benefit surface is GPU draw-side only",
    "the instance-count dwords and the visibility bitmask are assumed zeroed before dispatch (runtime responsibility of the later patch slices); pre-dispatch zeroing is not wired here",
    "runtime hook / native-handle binding not wired (patches 0027-0029, later serial slices)",
    "GPU-observed math parity pending the standalone S6 dispatch smoke (math_parity_evidence.json stays pending_gpu_dispatch)",
]

DOES_NOT_IMPLY = [
    "Godot runtime gpu_culling pass completion",
    "real_gpu_pass=true",
    "real_d3d12_dispatch_recorded=true",
    "visual success",
    "temporal stability success",
    "GPU timestamp success",
    "draw/dispatch-count reduction claim",
    "performance claim",
    "default pass enablement",
    "math parity proven (see math_parity_evidence.json)",
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


def write_text(path: pathlib.Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)


def write_json(path: pathlib.Path, value: dict[str, object]) -> None:
    write_text(path, json.dumps(value, indent=2, ensure_ascii=True) + "\n")


def find_tool(name: str) -> pathlib.Path | None:
    for env_key in ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"):
        value = os.environ.get(env_key)
        if not value:
            continue
        candidate = pathlib.Path(value).expanduser() / name
        if candidate.is_file():
            return candidate
    candidate = DEFAULT_DXC_DIR / name
    if candidate.is_file():
        return candidate
    found = shutil.which(name)
    if found:
        return pathlib.Path(found)
    return None


def command_output(path: pathlib.Path | None, args: list[str]) -> str | None:
    if path is None:
        return None
    try:
        completed = subprocess.run(
            [str(path), *args],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=20,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return str(exc)
    return (completed.stdout or "").strip()[:4000]


def run_command(argv: list[str], stdout_path: pathlib.Path, stderr_path: pathlib.Path) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        argv,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    write_text(stdout_path, completed.stdout or "")
    write_text(stderr_path, completed.stderr or "")
    return completed


def artifact_entry(path: pathlib.Path, artifact_kind: str, produced: bool) -> dict[str, object]:
    return {
        "path": rel(path),
        "exists": path.is_file(),
        "size_bytes": path.stat().st_size if path.is_file() else None,
        "sha256": sha256_of_file(path),
        "artifact_kind": artifact_kind,
        "produced_by_current_run": produced,
    }


def descriptor_layout_doc() -> dict[str, object]:
    return {
        "module": "gpu_culling",
        "pass_id": "gpu_culling",
        "artifact_kind": "hlsl_bridge_descriptor_layout",
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "entry_point": ENTRY_POINT,
        "target_profile": TARGET_PROFILE,
        "root_constants": 36,
        "root_constant_layout": [dict(entry) for entry in ROOT_CONSTANT_LAYOUT],
        "push_constant_note": (
            "144-byte / 36-dword RURIX-DEFINED layout: gpu_culling is an "
            "additive pass with no native Godot compute shader, so no Godot "
            "push-constant struct exists to mirror (contrast with GRX-013 "
            "CopyPushConstant / GRX-014 ClusterStore::PushConstant). Six "
            "normalized inward-facing frustum planes (nx, ny, nz, d), the "
            "instance/stride/surface/command parameters, and the local "
            "bounding sphere (mesh local AABB center + half-diagonal radius). "
            "command_stride_dwords == 5 mirrors INDIRECT_MULTIMESH_COMMAND_"
            "STRIDE (mesh_storage.h L63) and instance_count_dword_index == 1 "
            "mirrors the +sizeof(uint32_t) byte offset of the CPU count write "
            "(mesh_storage.cpp L2210); both are carried as parameters, not "
            "hardcoded."
        ),
        "resources": [dict(resource) for resource in RESOURCES],
        # RootConstants parameter at index 0 + single SRV/UAV descriptor table
        # (SRV range t0 then UAV range u0-u1).
        "root_signature_parameters": 2,
        "grx015_mapping": {
            "status": "offline_compile_package",
            "requires_64bit_integer_shader_capability": False,
            "requires_64bit_integer_shader_capability_note": (
                "gpu_culling carries no i64 push-constant fields (planes/"
                "sphere are f32, parameters are u32), so the SHADER_INT64 "
                "device capability is NOT part of its binding preflight (same "
                "as GRX-013/014)."
            ),
            "runtime_state": "fallback_only",
            "real_gpu_pass": False,
            "root_constant_bytes": 144,
            "root_constant_dwords": 36,
            "transform_stride_floats_in_scope": 12,
            "transform_layout_note": (
                "row-major 3x4 float lanes per 3D instance (mesh_storage.cpp "
                "_multimesh_instance_set_transform L1880-1915): lanes 0-3 = "
                "(basis.rows[0], origin.x), 4-7 = (rows[1], origin.y), 8-11 = "
                "(rows[2], origin.z)"
            ),
            "command_stride_dwords": 5,
            "instance_count_dword_index": 1,
            "command_write_note": (
                "count-only: InterlockedAdd(+1) per visible instance into "
                "EACH surface's instance-count dword (the dword the CPU "
                "writes at mesh_storage.cpp L2210); all other command dwords "
                "untouched; count dwords assumed zeroed pre-dispatch"
            ),
            "visibility_bitmask_note": (
                "u32[ceil(instance_count/32)], bit (i & 31) of word (i >> 5) "
                "= instance i visible, InterlockedOr writes, zeroed "
                "pre-dispatch; the GRX-016/018 shared input interface"
            ),
            "dispatch_note": (
                "one thread per instance; local 64x1x1; dispatch "
                "(ceil(instance_count / 64), 1, 1)"
            ),
        },
    }


def finish(evidence: dict[str, object], status: str, blocker_category: str | None, blocker_summary: str | None, exit_code: int) -> int:
    evidence["status"] = status
    evidence["blocker_category"] = blocker_category
    evidence["blocker_summary"] = blocker_summary
    evidence["runtime_mappable"] = status == "success"
    write_json(EVIDENCE_PATH, evidence)
    print(f"[grx015-hlsl-bridge] status={status} blocker={blocker_category} evidence={EVIDENCE_PATH}")
    return exit_code


def main() -> int:
    if not HLSL_PATH.is_file():
        raise SystemExit(f"missing HLSL kernel source: {HLSL_PATH}")
    BRIDGE_DIR.mkdir(parents=True, exist_ok=True)

    dxc_path = find_tool("dxc.exe")
    dxv_path = find_tool("dxv.exe")

    evidence: dict[str, object] = {
        "pass_id": "gpu_culling",
        "segment": "offline_compile",
        "status": "skip",
        "runtime_state": "fallback_only",
        "attempted_at_utc": utc_now(),
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "rurix_owned_rts0": True,
        "canonical_switch_exception": "owner_approved_hlsl_bridge_workaround",
        "provenance_policy_precedent": "spike/godot-rurix/passes/luminance_reduction/texture_artifact_provenance_policy.json",
        "provenance_blocker_note": (
            "gpu_culling is an all raw-buffer / SSBO pass, so the texture-"
            "intrinsic llc blocker in the precedent policy does NOT apply. "
            "The workaround is used for four different blockers: (1) the DXIL "
            "compute-body lowering accepts only View<global, f32>/ViewMut<"
            "global, f32> buffer views while both outputs (command dwords, "
            "visibility bitmask words) are u32-word SSBOs whose bit patterns "
            "an f32 view cannot carry bit-faithfully; (2) the Rurix lang "
            "subset has NO atomic intrinsic (InterlockedAdd/InterlockedOr) on "
            "any backend and the kernel's only writes are atomics; (3) the "
            "integer bit operations the bitmask write needs (<<, &, >>) exist "
            "in MIR but have no DXIL-backend lowering; (4) the conservative-"
            "radius sqrt has no DXIL lowering (DeviceMathFn is NVPTX-"
            "libdevice-only)."
        ),
        "math_parity_evidence": rel(MATH_PARITY_EVIDENCE_PATH),
        "math_parity_status": MATH_PARITY_STATUS,
        "inputs": {
            "package_manifest": rel(PASS_DIR / "rurix.toml"),
            "entry_file": rel(HLSL_PATH),
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_reference_files": [
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp",
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/mesh_storage.h",
                "external/godot-master/servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp",
            ],
            "godot_reference_note": (
                "ADDITIVE pass: no native Godot compute shader exists for this "
                "math (the references document the indirect-MultiMesh command-"
                "buffer infrastructure and the CPU count write point the "
                "kernel aligns with, not a shader to port)"
            ),
        },
        "tools": {
            "dxc": {
                "found": dxc_path is not None,
                "path": str(dxc_path) if dxc_path else None,
                "version_output": command_output(dxc_path, ["--version"]),
            },
            "dxv": {
                "found": dxv_path is not None,
                "path": str(dxv_path) if dxv_path else None,
            },
        },
        "hlsl": {
            "path": rel(HLSL_PATH),
            "entry_point": ENTRY_POINT,
            "target_profile": TARGET_PROFILE,
            "sha256": sha256_of_file(HLSL_PATH),
        },
        "commands": [],
        "attempted_binding_kinds": ["structured_buffer", "rwstructured_buffer", "rwstructured_buffer"],
        "runtime_mappable": False,
        "known_gaps": list(KNOWN_GAPS),
        "does_not_imply": list(DOES_NOT_IMPLY),
    }

    if dxc_path is None:
        return finish(evidence, "skip", "dxc_missing", "dxc.exe not found (set RURIX_DXC_DIR)", 0)

    # 1) DXC compile.
    dxc_argv = [str(dxc_path), "-T", TARGET_PROFILE, "-E", ENTRY_POINT, "-Qstrip_debug",
                "-Fo", str(BRIDGE_DXIL_PATH), str(HLSL_PATH)]
    dxc_stdout = BRIDGE_DIR / "dxc_stdout.txt"
    dxc_stderr = BRIDGE_DIR / "dxc_stderr.txt"
    dxc_completed = run_command(dxc_argv, dxc_stdout, dxc_stderr)
    evidence["commands"].append({
        "label": "dxc_compile",
        "argv": dxc_argv,
        "exit_code": dxc_completed.returncode,
        "stdout_path": rel(dxc_stdout),
        "stderr_path": rel(dxc_stderr),
    })
    if dxc_completed.returncode != 0 or not BRIDGE_DXIL_PATH.is_file():
        return finish(evidence, "compile_failed", "dxil_container_missing",
                      "dxc compile failed; see dxc_stderr.txt", 1)

    # 2) DXV validation.
    if dxv_path is None:
        return finish(evidence, "skip", "dxv_missing", "dxv.exe not found (set RURIX_DXC_DIR)", 0)
    dxv_argv = [str(dxv_path), str(BRIDGE_DXIL_PATH)]
    dxv_stdout = BRIDGE_DIR / "dxv_stdout.txt"
    dxv_stderr = BRIDGE_DIR / "dxv_stderr.txt"
    dxv_completed = run_command(dxv_argv, dxv_stdout, dxv_stderr)
    evidence["commands"].append({
        "label": "dxv_validate",
        "argv": dxv_argv,
        "exit_code": dxv_completed.returncode,
        "stdout_path": rel(dxv_stdout),
        "stderr_path": rel(dxv_stderr),
    })
    evidence["dxil_provenance"] = {
        "produced_by": "dxc",
        "compiler_version": command_output(dxc_path, ["--version"]),
        "target_profile": TARGET_PROFILE,
        "entry_point": ENTRY_POINT,
        "hlsl_source": {"path": rel(HLSL_PATH), "sha256": sha256_of_file(HLSL_PATH)},
        "validation": {
            "tool": "dxv.exe",
            "status": "pass" if dxv_completed.returncode == 0 else "fail",
            "exit_code": dxv_completed.returncode,
        },
    }
    if dxv_completed.returncode != 0:
        return finish(evidence, "validation_failed", "dxv_validation_failed",
                      "dxv rejected the DXIL container; see dxv_stderr.txt", 1)

    # 3) Descriptor layout (canonical path).
    write_json(CANONICAL_DESCRIPTOR_PATH, descriptor_layout_doc())

    # 4) Rurix-owned RTS0 via the binding-layout example.
    cargo_argv = [
        "cargo", "run", "-q", "-p", "rurixc",
        "--features", "dxil-backend shader-stages",
        "--example", "emit_grx015_gpu_culling_rts0",
        "--",
        rel(CANONICAL_DESCRIPTOR_PATH),
        rel(CANONICAL_RTS0_PATH),
    ]
    cargo_stdout = BRIDGE_DIR / "emit_rts0_stdout.txt"
    cargo_stderr = BRIDGE_DIR / "emit_rts0_stderr.txt"
    cargo_completed = run_command(cargo_argv, cargo_stdout, cargo_stderr)
    evidence["commands"].append({
        "label": "emit_grx015_gpu_culling_rts0",
        "argv": cargo_argv,
        "exit_code": cargo_completed.returncode,
        "stdout_path": rel(cargo_stdout),
        "stderr_path": rel(cargo_stderr),
    })
    if cargo_completed.returncode != 0 or not CANONICAL_RTS0_PATH.is_file():
        return finish(evidence, "compile_failed", "rts0_emit_failed",
                      "emit_grx015_gpu_culling_rts0 failed; see emit_rts0_stderr.txt", 1)
    evidence["root_signature_generator"] = {
        "kind": "rurixc_binding_layout_example",
        "command": cargo_argv,
        "source_api": "rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}",
        "root_constants": "144_bytes_36_dwords_at_root_parameter_index_0",
    }

    # 5) Publish the canonical DXIL copy (byte-identical to the bridge output).
    shutil.copyfile(BRIDGE_DXIL_PATH, CANONICAL_DXIL_PATH)

    evidence["artifacts"] = {
        "dxil": {
            **artifact_entry(CANONICAL_DXIL_PATH, "dxil_container", True),
            "semantic_status": "lowered_compute_body",
        },
        "root_signature": artifact_entry(CANONICAL_RTS0_PATH, "rurix_owned_rts0_root_signature", True),
        "descriptor_layout": artifact_entry(CANONICAL_DESCRIPTOR_PATH, "descriptor_layout_json", True),
    }
    evidence["notes"] = [
        "Runtime remains fallback_only: the real dispatch path (a later patch/bridge slice) is opt-in only and armed by RXGD_CAP_GPU_CULLING_REAL_PASS (1u<<9); the shipping feature-off bridge fails closed.",
        "The canonical artifacts/ paths carry the raw-buffer hlsl_bridge workaround package: a DXC cs_6_0 DXIL container (validated by dxv), the Rurix-owned RTS0 root signature serialized by rurixc::binding_layout::serialize_rts0 (144-byte / 36-dword Rurix-defined root constants + SRV t0 structured buffer + UAV u0/u1 structured buffers), and the descriptor layout with per-slot structured_buffer/rwstructured_buffer/rwstructured_buffer binding kinds.",
        "hlsl_bridge_workaround provenance (buffer-pass variant): NOT rurix_owned. Blockers differ from the texture passes: (1) no u32 buffer views in the DXIL compute-body lowering; (2) no atomic intrinsics in the lang subset on any backend; (3) no DXIL-backend lowering for the integer bit operations; (4) no DXIL sqrt lowering. The texture-intrinsic llc blocker does NOT apply to this raw-buffer pass.",
        "The kernel math is Rurix-defined (additive pass, count-only conservative sphere cull): no native Godot compute shader is replaced; the aligned native behavior is the CPU count write at mesh_storage.cpp L2210 and the untouched remaining command dwords. See known_gaps.",
    ]
    return finish(evidence, "success", None, None, 0)


if __name__ == "__main__":
    sys.exit(main())
