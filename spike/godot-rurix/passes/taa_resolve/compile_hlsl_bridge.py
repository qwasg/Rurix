#!/usr/bin/env python3
"""GRX-012: compile the math-equivalent HLSL TAA resolve kernel.

Compiles ``artifacts/hlsl_bridge/taa_resolve.hlsl`` via DXC (cs_6_0), validates
it with DXV, emits the descriptor layout JSON (per-slot texture2d/rwtexture2d
binding kinds for the 5 SRVs + 1 UAV + the canonical 28-byte / 7-dword
root-constant layout), and synthesizes a Rurix-owned RTS0 root signature
through ``cargo run --example emit_grx012_taa_resolve_rts0``
(``rurixc::binding_layout::{infer_root_signature, pack_root_constants,
serialize_rts0}``).

On success the canonical package is published to the pass ``artifacts/`` paths
(``taa_resolve.dxil``, ``taa_resolve.rts0.bin``,
``taa_resolve_descriptor_layout.json``) under the owner-approved
``hlsl_bridge_workaround`` provenance policy (GRX-009
``texture_artifact_provenance_policy.json``, which applies to every texture
compute pass), and ``offline_compile_evidence.json`` is written.

Tool discovery follows the GRX-009..011 template (``RURIX_DXC_DIR`` /
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
HLSL_PATH = BRIDGE_DIR / "taa_resolve.hlsl"
BRIDGE_DXIL_PATH = BRIDGE_DIR / "taa_resolve.dxil"
CANONICAL_DXIL_PATH = ARTIFACTS_DIR / "taa_resolve.dxil"
CANONICAL_RTS0_PATH = ARTIFACTS_DIR / "taa_resolve.rts0.bin"
CANONICAL_DESCRIPTOR_PATH = ARTIFACTS_DIR / "taa_resolve_descriptor_layout.json"
EVIDENCE_PATH = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"
DEFAULT_DXC_DIR = pathlib.Path(r"H:\dxc-round7\extracted\bin\x64")

TARGET_PROFILE = "cs_6_0"
ENTRY_POINT = "main"

# Canonical GRX-012 taa_resolve root-constant layout (7 dwords = 28 bytes at
# root_parameter_index 0; the [i64, i64, f32, f32, f32] packing shape is shared
# with the GRX-009/GRX-010/GRX-011 canonical layouts). The i64 dims are carried
# as uint2 (low, high dword) in the HLSL cbuffer; the runtime must write 0 to
# the high dwords.
ROOT_CONSTANT_LAYOUT = [
    {"name": "source_width", "type": "i64", "order": 0, "root_parameter_index": 0, "dword_offset": 0, "dword_size": 2},
    {"name": "source_height", "type": "i64", "order": 1, "root_parameter_index": 0, "dword_offset": 2, "dword_size": 2},
    {"name": "disocclusion_threshold", "type": "f32", "order": 2, "root_parameter_index": 0, "dword_offset": 4, "dword_size": 1},
    {"name": "variance_dynamic", "type": "f32", "order": 3, "root_parameter_index": 0, "dword_offset": 5, "dword_size": 1},
    {"name": "reserved0", "type": "f32", "order": 4, "root_parameter_index": 0, "dword_offset": 6, "dword_size": 1},
]

RESOURCES = [
    {"name": "color_buffer", "class": "t", "register": 0, "space": 0, "count": 1, "hlsl_type": "Texture2D<float4>", "binding_kind": "texture2d"},
    {"name": "depth_buffer", "class": "t", "register": 1, "space": 0, "count": 1, "hlsl_type": "Texture2D<float>", "binding_kind": "texture2d"},
    {"name": "velocity_buffer", "class": "t", "register": 2, "space": 0, "count": 1, "hlsl_type": "Texture2D<float2>", "binding_kind": "texture2d"},
    {"name": "last_velocity_buffer", "class": "t", "register": 3, "space": 0, "count": 1, "hlsl_type": "Texture2D<float2>", "binding_kind": "texture2d"},
    {"name": "history_buffer", "class": "t", "register": 4, "space": 0, "count": 1, "hlsl_type": "Texture2D<float4>", "binding_kind": "texture2d"},
    {"name": "output_buffer", "class": "u", "register": 0, "space": 0, "count": 1, "hlsl_type": "RWTexture2D<float4>", "binding_kind": "rwtexture2d"},
]

KNOWN_GAPS = [
    "hardware bilinear (textureLod with a linear+clamp sampler) reproduced as explicit float 4-tap Load bilinear; interior UVs are texel-exact, real-sampler sub-texel fixed-point rounding is a recorded gap",
    "rgba16f / rg16f half storage quantization not modelled (kernel computes float32; native buffers are half)",
    "history physical maintenance chain (resolve->temp->internal->history via three copy_to_rect) not wired (single resolve dispatch only)",
    "one-frame latency (a self-queue dispatch reads the previous frame's color/velocity); a true replacement needs draw_graph integration (deferred)",
    "get_closest_pixel_velocity_3x3 border-offset quirk reproduced faithfully",
    "imageLoad out-of-bounds returns 0 (native Vulkan semantics) reproduced by Load OOB=0",
    "no multiview",
    "no temporal/noise stability evidence (GRX-012 plan note; 4g-level, later slice)",
    "GPU-side math parity observation pending a real dispatch (math_parity_evidence.json)",
]

DOES_NOT_IMPLY = [
    "Godot runtime TAA resolve pass completion",
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
        "module": "taa_resolve",
        "pass_id": "taa_resolve",
        "artifact_kind": "hlsl_bridge_descriptor_layout",
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "entry_point": ENTRY_POINT,
        "target_profile": TARGET_PROFILE,
        "root_constants": 5,
        "root_constant_layout": [dict(entry) for entry in ROOT_CONSTANT_LAYOUT],
        "i64_dims_note": (
            "source_width/source_height are carried as uint2 (low, high dword) in the HLSL "
            "cbuffer to keep plain cs_6_0 without the optional Int64 capability; only the low "
            "dword is consumed and the runtime must write 0 to the high dwords."
        ),
        "resources": [dict(resource) for resource in RESOURCES],
        # RootConstants parameter at index 0 + single SRV/UAV descriptor table
        # (SRV range t0..t4 precedes UAV range u0).
        "root_signature_parameters": 2,
        "grx012_mapping": {
            "status": "offline_compile_package",
            "requires_64bit_integer_shader_capability": True,
            "runtime_state": "fallback_only",
            "real_gpu_pass": False,
            "resource_count": 6,
            "srv_count": 5,
            "uav_count": 1,
            "root_constant_bytes": 28,
            "root_constant_dwords": 7,
            "dst_shape": "output extent == color extent (1:1 full-resolution resolve pass)",
        },
    }


def finish(evidence: dict[str, object], status: str, blocker_category: str | None, blocker_summary: str | None, exit_code: int) -> int:
    evidence["status"] = status
    evidence["blocker_category"] = blocker_category
    evidence["blocker_summary"] = blocker_summary
    evidence["runtime_mappable"] = status == "success"
    write_json(EVIDENCE_PATH, evidence)
    print(f"[grx012-hlsl-bridge] status={status} blocker={blocker_category} evidence={EVIDENCE_PATH}")
    return exit_code


def main() -> int:
    if not HLSL_PATH.is_file():
        raise SystemExit(f"missing HLSL kernel source: {HLSL_PATH}")
    BRIDGE_DIR.mkdir(parents=True, exist_ok=True)

    dxc_path = find_tool("dxc.exe")
    dxv_path = find_tool("dxv.exe")

    evidence: dict[str, object] = {
        "pass_id": "taa_resolve",
        "segment": "offline_compile",
        "status": "skip",
        "runtime_state": "fallback_only",
        "attempted_at_utc": utc_now(),
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "rurix_owned_rts0": True,
        "canonical_switch_exception": "owner_approved_hlsl_bridge_workaround",
        "provenance_policy": "spike/godot-rurix/passes/luminance_reduction/texture_artifact_provenance_policy.json",
        "math_parity_evidence": rel(MATH_PARITY_EVIDENCE_PATH),
        "math_parity_status": "taa_resolve_cpu_reference_proven_pending_gpu_dispatch",
        "inputs": {
            "package_manifest": rel(PASS_DIR / "rurix.toml"),
            "entry_file": rel(HLSL_PATH),
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_reference_files": [
                "external/godot-master/servers/rendering/renderer_rd/effects/taa.cpp",
                "external/godot-master/servers/rendering/renderer_rd/shaders/effects/taa_resolve.glsl",
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
        "attempted_binding_kinds": ["texture2d", "texture2d", "texture2d", "texture2d", "texture2d", "rwtexture2d"],
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
        "--example", "emit_grx012_taa_resolve_rts0",
        "--",
        rel(CANONICAL_DESCRIPTOR_PATH),
        rel(CANONICAL_RTS0_PATH),
    ]
    cargo_stdout = BRIDGE_DIR / "emit_rts0_stdout.txt"
    cargo_stderr = BRIDGE_DIR / "emit_rts0_stderr.txt"
    cargo_completed = run_command(cargo_argv, cargo_stdout, cargo_stderr)
    evidence["commands"].append({
        "label": "emit_grx012_taa_resolve_rts0",
        "argv": cargo_argv,
        "exit_code": cargo_completed.returncode,
        "stdout_path": rel(cargo_stdout),
        "stderr_path": rel(cargo_stderr),
    })
    if cargo_completed.returncode != 0 or not CANONICAL_RTS0_PATH.is_file():
        return finish(evidence, "compile_failed", "rts0_emit_failed",
                      "emit_grx012_taa_resolve_rts0 failed; see emit_rts0_stderr.txt", 1)
    evidence["root_signature_generator"] = {
        "kind": "rurixc_binding_layout_example",
        "command": cargo_argv,
        "source_api": "rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}",
        "root_constants": "28_bytes_at_root_parameter_index_0",
        "descriptor_table": "SRV range t0..t4 (5 descriptors) precedes UAV range u0 (1 descriptor)",
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
        "Runtime remains fallback_only: the real dispatch path is linked only under the d3d12-recording-shim feature and armed only by the opt-in RXGD_CAP_TAA_RESOLVE_REAL_PASS flag; the shipping feature-off bridge fails closed with real_dispatch_path_not_linked.",
        "The canonical artifacts/ paths carry the texture-capable hlsl_bridge workaround package: a DXC cs_6_0 DXIL container (validated by dxv), the Rurix-owned RTS0 root signature serialized by rurixc::binding_layout::serialize_rts0 (28-byte root constants + SRV range t0..t4 + UAV u0), and the descriptor layout with per-slot texture2d/rwtexture2d binding kinds.",
        "This canonical package uses the owner-approved hlsl_bridge_workaround provenance exception (GRX-009 texture_artifact_provenance_policy.json, which applies to every texture compute pass): the artifact is runtime-mappable but NOT rurix_owned; a rurixc-owned taa_resolve compile still requires a patched llc that supports texture intrinsics plus multi-channel texture element support in the Rurix lang items.",
        "The kernel math mirrors the taa_resolve.glsl single full-resolution resolve; hardware bilinear is reproduced as explicit float Load bilinear and half storage quantization is not modelled (see known_gaps).",
    ]
    return finish(evidence, "success", None, None, 0)


if __name__ == "__main__":
    sys.exit(main())
