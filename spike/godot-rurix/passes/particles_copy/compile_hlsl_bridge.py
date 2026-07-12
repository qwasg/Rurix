#!/usr/bin/env python3
"""GRX-013: compile the math-equivalent HLSL particles_copy kernel.

Compiles ``artifacts/hlsl_bridge/particles_copy_fill_instances.hlsl`` via DXC
(cs_6_0), validates it with DXV, emits the descriptor layout JSON (per-slot
structured_buffer / rwstructured_buffer binding kinds + the 128-byte / 32-dword
CopyPushConstant root-constant layout), and synthesizes a Rurix-owned RTS0 root
signature through ``cargo run --example emit_grx013_particles_copy_rts0``
(``rurixc::binding_layout::{infer_root_signature, pack_root_constants,
serialize_rts0}``).

Route rationale (particles_copy is an all raw-buffer / SSBO pass, so the
GRX-009 texture-intrinsic llc blocker does NOT apply): a rurixc-owned rx -> DXIL
compile of the in-scope subset is still infeasible because (1) Rurix's lang
subset models raw buffers as scalar ``View<f32>`` / ``ViewMut<f32>`` with no
``struct`` / ``vec4`` / ``mat4`` aggregate SSBO element types (ParticleData is a
mat4+vec3+uint+vec4+vec4 aggregate) and (2) the DXIL backend does not lower the
sin/cos/sqrt device-math intrinsics the ALIGN_BILLBOARD subset needs
(``DeviceMathFn`` lowers only on the NVPTX libdevice ``__nv_*`` path). The RTS0
is Rurix-owned; the DXIL/descriptor package is the owner-approved
``hlsl_bridge_workaround`` (precedent:
``../luminance_reduction/texture_artifact_provenance_policy.json``).

Tool discovery follows the GRX-009/010/011 template (``RURIX_DXC_DIR`` /
``RURIX_DXC_NEW_DIR`` env, then the default round-7 extraction dir, then PATH).

Fail-closed: this is an ``hlsl_bridge_workaround`` artifact set, NOT rurix_owned.
It never advances ``runtime_state``/``real_gpu_pass`` and the pass stays default
disabled; math parity stays pending until ``math_parity_evidence.json`` records
a measured GPU comparison.
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
HLSL_PATH = BRIDGE_DIR / "particles_copy_fill_instances.hlsl"
BRIDGE_DXIL_PATH = BRIDGE_DIR / "particles_copy_fill_instances.dxil"
CANONICAL_DXIL_PATH = ARTIFACTS_DIR / "particles_copy.dxil"
CANONICAL_RTS0_PATH = ARTIFACTS_DIR / "particles_copy.rts0.bin"
CANONICAL_DESCRIPTOR_PATH = ARTIFACTS_DIR / "particles_copy_descriptor_layout.json"
EVIDENCE_PATH = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"
DEFAULT_DXC_DIR = pathlib.Path(r"H:\dxc-round7\extracted\bin\x64")

TARGET_PROFILE = "cs_6_0"
ENTRY_POINT = "main"

MATH_PARITY_STATUS = "fill_instances_cpu_reference_proven_pending_gpu_dispatch"

# Canonical GRX-013 particles_copy root-constant layout: Godot's 128-byte
# ParticlesShader::CopyPushConstant = 32 dwords at root_parameter_index 0.
# Every field is a single dword; uint32 fields are carried as 1-dword slots
# (the RTS0 only encodes dword layout, the "type" here is the true semantic
# type for the descriptor JSON). Order/offsets match the struct byte layout.
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
    _rc("sort_direction_x", "f32", 0),
    _rc("sort_direction_y", "f32", 1),
    _rc("sort_direction_z", "f32", 2),
    _rc("total_particles", "u32", 3),
    _rc("trail_size", "u32", 4),
    _rc("trail_total", "u32", 5),
    _rc("frame_delta", "f32", 6),
    _rc("frame_remainder", "f32", 7),
    _rc("align_up_x", "f32", 8),
    _rc("align_up_y", "f32", 9),
    _rc("align_up_z", "f32", 10),
    _rc("align_mode", "u32", 11),
    _rc("lifetime_split", "u32", 12),
    _rc("lifetime_reverse", "u32", 13),
    _rc("motion_vectors_current_offset", "u32", 14),
    _rc("flags_bits", "u32", 15),
    _rc("inv_emission_transform_0", "f32", 16),
    _rc("inv_emission_transform_1", "f32", 17),
    _rc("inv_emission_transform_2", "f32", 18),
    _rc("inv_emission_transform_3", "f32", 19),
    _rc("inv_emission_transform_4", "f32", 20),
    _rc("inv_emission_transform_5", "f32", 21),
    _rc("inv_emission_transform_6", "f32", 22),
    _rc("inv_emission_transform_7", "f32", 23),
    _rc("inv_emission_transform_8", "f32", 24),
    _rc("inv_emission_transform_9", "f32", 25),
    _rc("inv_emission_transform_10", "f32", 26),
    _rc("inv_emission_transform_11", "f32", 27),
    _rc("align_channel_filter", "u32", 28),
    _rc("align_axis", "u32", 29),
    _rc("pad1", "u32", 30),
    _rc("pad2", "u32", 31),
]

RESOURCES = [
    {
        "name": "src_particles",
        "class": "t",
        "register": 0,
        "space": 0,
        "count": 1,
        "hlsl_type": "StructuredBuffer<ParticleData>",
        "binding_kind": "structured_buffer",
    },
    {
        "name": "dst_instances",
        "class": "u",
        "register": 0,
        "space": 0,
        "count": 1,
        "hlsl_type": "RWStructuredBuffer<float4>",
        "binding_kind": "rwstructured_buffer",
    },
]

KNOWN_GAPS = [
    "3D COPY_MODE_FILL_INSTANCES subset only; align_mode limited to ALIGN_DISABLED (0) and ALIGN_BILLBOARD (1)",
    "align_mode ALIGN_Y_TO_VELOCITY (2), ALIGN_Z_BILLBOARD_Y_TO_VELOCITY (3), ALIGN_LOCAL_BILLBOARD (4) unsupported",
    "2D copy mode (PARAMS_FLAG_COPY_MODE_2D + inv_emission_transform + 4-vec4 instance write) unsupported",
    "MODE_FILL_SORT_BUFFER and COPY_MODE_FILL_INSTANCES_WITH_SORT_BUFFER (VIEW_DEPTH sort) unsupported; no USE_SORT_BUFFER binding",
    "ORDER_BY_LIFETIME / REVERSE_LIFETIME draw-order reindex unsupported (kernel maps instance i <- particle i)",
    "trail interpolation and trail_bind_poses (trail_size > 1) unsupported; no set2 TrailBindPoses binding",
    "userdata channels (USERDATA_COUNT) unsupported; ParticleData stride fixed at 112 bytes",
    "motion_vectors_current_offset carried in b0 but exercised at 0 (motion-vector double-buffer offset is a runtime concern)",
    "rg / half-float / packed instance quantization not modelled (kernel and instance buffer are float4)",
    "the cull-stage hook (particles_set_view_axis / renderer_scene_cull.cpp) is a later patch slice; this slice is the offline face only",
    "GPU-side math parity observation pending a real dispatch (math_parity_evidence.json)",
]

DOES_NOT_IMPLY = [
    "Godot runtime particles_copy pass completion",
    "real_gpu_pass=true",
    "real_d3d12_dispatch_recorded=true",
    "visual success",
    "temporal stability success",
    "GPU timestamp success",
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
        "module": "particles_copy",
        "pass_id": "particles_copy",
        "artifact_kind": "hlsl_bridge_descriptor_layout",
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "entry_point": ENTRY_POINT,
        "target_profile": TARGET_PROFILE,
        "root_constants": 32,
        "root_constant_layout": [dict(entry) for entry in ROOT_CONSTANT_LAYOUT],
        "push_constant_note": (
            "128-byte / 32-dword mirror of Godot's ParticlesShader::CopyPushConstant "
            "(servers/rendering/renderer_rd/storage_rd/particles_storage.h L303-329); "
            "fields for out-of-scope features (inv_emission_transform, trail_*, "
            "lifetime_*, align_axis) are carried for byte-exact push-constant shape "
            "parity and are unused by the in-scope kernel."
        ),
        "resources": [dict(resource) for resource in RESOURCES],
        # RootConstants parameter at index 0 + single SRV/UAV descriptor table.
        "root_signature_parameters": 2,
        "grx013_mapping": {
            "status": "offline_compile_package",
            "requires_64bit_integer_shader_capability": False,
            "requires_64bit_integer_shader_capability_note": (
                "Unlike the GRX-009/010/011 texture passes, particles_copy carries no "
                "i64 push-constant fields (CopyPushConstant is all f32/u32), so the "
                "SHADER_INT64 device capability is NOT part of its preflight."
            ),
            "runtime_state": "fallback_only",
            "real_gpu_pass": False,
            "root_constant_bytes": 128,
            "root_constant_dwords": 32,
            "particle_data_stride_bytes": 112,
            "instance_stride_vec4": 5,
            "instance_stride_note": "3D: 3 transposed xform rows + color + custom = 5 vec4 per instance",
        },
    }


def finish(evidence: dict[str, object], status: str, blocker_category: str | None, blocker_summary: str | None, exit_code: int) -> int:
    evidence["status"] = status
    evidence["blocker_category"] = blocker_category
    evidence["blocker_summary"] = blocker_summary
    evidence["runtime_mappable"] = status == "success"
    write_json(EVIDENCE_PATH, evidence)
    print(f"[grx013-hlsl-bridge] status={status} blocker={blocker_category} evidence={EVIDENCE_PATH}")
    return exit_code


def main() -> int:
    if not HLSL_PATH.is_file():
        raise SystemExit(f"missing HLSL kernel source: {HLSL_PATH}")
    BRIDGE_DIR.mkdir(parents=True, exist_ok=True)

    dxc_path = find_tool("dxc.exe")
    dxv_path = find_tool("dxv.exe")

    evidence: dict[str, object] = {
        "pass_id": "particles_copy",
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
            "particles_copy is an all raw-buffer / SSBO pass, so the texture-intrinsic "
            "llc blocker in the precedent policy does NOT apply. The workaround is used "
            "for two different blockers: (1) no aggregate (struct/vec4/mat4) SSBO element "
            "types in the Rurix lang subset (ParticleData is a mat4+vec3+uint+vec4+vec4 "
            "aggregate); (2) the DXIL backend does not lower the sin/cos/sqrt device-math "
            "intrinsics the ALIGN_BILLBOARD subset needs (DeviceMathFn is NVPTX-libdevice-only)."
        ),
        "math_parity_evidence": rel(MATH_PARITY_EVIDENCE_PATH),
        "math_parity_status": MATH_PARITY_STATUS,
        "inputs": {
            "package_manifest": rel(PASS_DIR / "rurix.toml"),
            "entry_file": rel(HLSL_PATH),
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_reference_files": [
                "external/godot-master/servers/rendering/renderer_rd/shaders/particles_copy.glsl",
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/particles_storage.cpp",
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/particles_storage.h",
            ],
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
        "attempted_binding_kinds": ["structured_buffer", "rwstructured_buffer"],
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
        "--example", "emit_grx013_particles_copy_rts0",
        "--",
        rel(CANONICAL_DESCRIPTOR_PATH),
        rel(CANONICAL_RTS0_PATH),
    ]
    cargo_stdout = BRIDGE_DIR / "emit_rts0_stdout.txt"
    cargo_stderr = BRIDGE_DIR / "emit_rts0_stderr.txt"
    cargo_completed = run_command(cargo_argv, cargo_stdout, cargo_stderr)
    evidence["commands"].append({
        "label": "emit_grx013_particles_copy_rts0",
        "argv": cargo_argv,
        "exit_code": cargo_completed.returncode,
        "stdout_path": rel(cargo_stdout),
        "stderr_path": rel(cargo_stderr),
    })
    if cargo_completed.returncode != 0 or not CANONICAL_RTS0_PATH.is_file():
        return finish(evidence, "compile_failed", "rts0_emit_failed",
                      "emit_grx013_particles_copy_rts0 failed; see emit_rts0_stderr.txt", 1)
    evidence["root_signature_generator"] = {
        "kind": "rurixc_binding_layout_example",
        "command": cargo_argv,
        "source_api": "rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}",
        "root_constants": "128_bytes_32_dwords_at_root_parameter_index_0",
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
        "Runtime remains fallback_only: the real dispatch path (a later patch/bridge slice) is opt-in only and armed by RXGD_CAP_PARTICLES_COPY_REAL_PASS (1u<<7); the shipping feature-off bridge fails closed.",
        "The canonical artifacts/ paths carry the raw-buffer hlsl_bridge workaround package: a DXC cs_6_0 DXIL container (validated by dxv), the Rurix-owned RTS0 root signature serialized by rurixc::binding_layout::serialize_rts0 (128-byte / 32-dword CopyPushConstant root constants + SRV t0 structured buffer + UAV u0 structured buffer), and the descriptor layout with per-slot structured_buffer/rwstructured_buffer binding kinds.",
        "hlsl_bridge_workaround provenance (buffer-pass variant): NOT rurix_owned. Blockers differ from the texture passes: (1) no aggregate (struct/vec4/mat4) SSBO element types in the Rurix lang subset; (2) the DXIL backend does not lower sin/cos/sqrt (DeviceMathFn is NVPTX-libdevice-only), which ALIGN_BILLBOARD needs. The texture-intrinsic llc blocker does NOT apply to this raw-buffer pass.",
        "The kernel math mirrors the particles_copy.glsl COPY_MODE_FILL_INSTANCES 3D subset only (ALIGN_DISABLED + ALIGN_BILLBOARD, no trail, no sort, no 2D, no userdata); see known_gaps.",
    ]
    return finish(evidence, "success", None, None, 0)


if __name__ == "__main__":
    sys.exit(main())
