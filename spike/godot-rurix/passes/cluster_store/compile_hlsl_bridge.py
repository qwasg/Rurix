#!/usr/bin/env python3
"""GRX-014: compile the math-equivalent HLSL cluster_store kernel.

Compiles ``artifacts/hlsl_bridge/cluster_store_pack.hlsl`` via DXC (cs_6_0),
validates it with DXV, emits the descriptor layout JSON (per-slot
structured_buffer / structured_buffer / rwstructured_buffer binding kinds + the
32-byte / 8-dword ClusterStore::PushConstant root-constant layout), and
synthesizes a Rurix-owned RTS0 root signature through ``cargo run --example
emit_grx014_cluster_store_rts0`` (``rurixc::binding_layout::
{infer_root_signature, pack_root_constants, serialize_rts0}``).

Route rationale (cluster_store is an all raw-buffer / SSBO pass, so the GRX-009
texture-intrinsic llc blocker does NOT apply): a rurixc-owned rx -> DXIL compile
of the store kernel is infeasible because (1) the DXIL compute-body lowering
accepts only ``View<global, f32>`` / ``ViewMut<global, f32>`` buffer views
(src/rurixc/src/dxil_codegen.rs L1754/L1786) while all three cluster_store
buffers are u32-word SSBOs whose bit patterns an f32 view cannot carry
bit-faithfully; (2) the integer bit operations the kernel is built from
(``&``, ``|``, ``~``, ``<<``, ``>>``) exist in MIR (``BinOp::BitAnd/BitOr/
BitXor/Shl/Shr``) but have no DXIL-backend lowering; and (3) the Rurix lang
subset has no findLSB/findMSB (``firstbitlow``/``firstbithigh``) intrinsic on
any backend. The RTS0 is Rurix-owned; the DXIL/descriptor package is the
owner-approved ``hlsl_bridge_workaround`` (precedent:
``../particles_copy/PASS_CONTRACT.md`` sec 5.3 /
``../luminance_reduction/texture_artifact_provenance_policy.json``).

Tool discovery follows the GRX-009..013 template (``RURIX_DXC_DIR`` /
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
HLSL_PATH = BRIDGE_DIR / "cluster_store_pack.hlsl"
BRIDGE_DXIL_PATH = BRIDGE_DIR / "cluster_store_pack.dxil"
CANONICAL_DXIL_PATH = ARTIFACTS_DIR / "cluster_store.dxil"
CANONICAL_RTS0_PATH = ARTIFACTS_DIR / "cluster_store.rts0.bin"
CANONICAL_DESCRIPTOR_PATH = ARTIFACTS_DIR / "cluster_store_descriptor_layout.json"
EVIDENCE_PATH = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"
DEFAULT_DXC_DIR = pathlib.Path(r"H:\dxc-round7\extracted\bin\x64")

TARGET_PROFILE = "cs_6_0"
ENTRY_POINT = "main"

MATH_PARITY_STATUS = "cluster_store_cpu_reference_proven_pending_gpu_dispatch"

# Canonical GRX-014 cluster_store root-constant layout: Godot's 32-byte
# ClusterBuilderSharedDataRD::ClusterStore::PushConstant = 8 dwords at
# root_parameter_index 0. Every field is a single u32 dword; uint32 fields are
# carried as 1-dword slots (the RTS0 only encodes dword layout, the "type" here
# is the true semantic type for the descriptor JSON). Order/offsets match the
# struct byte layout (cluster_builder_rd.h L91-100).
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
    _rc("cluster_render_data_size", "u32", 0),
    _rc("max_render_element_count_div_32", "u32", 1),
    _rc("cluster_screen_size_x", "u32", 2),
    _rc("cluster_screen_size_y", "u32", 3),
    _rc("render_element_count_div_32", "u32", 4),
    _rc("max_cluster_element_count_div_32", "u32", 5),
    _rc("pad1", "u32", 6),
    _rc("pad2", "u32", 7),
]

RESOURCES = [
    {
        "name": "cluster_render",
        "class": "t",
        "register": 0,
        "space": 0,
        "count": 1,
        "hlsl_type": "StructuredBuffer<uint>",
        "binding_kind": "structured_buffer",
    },
    {
        "name": "render_elements",
        "class": "t",
        "register": 1,
        "space": 0,
        "count": 1,
        "hlsl_type": "StructuredBuffer<RenderElementData>",
        "binding_kind": "structured_buffer",
    },
    {
        "name": "cluster_store",
        "class": "u",
        "register": 0,
        "space": 0,
        "count": 1,
        "hlsl_type": "RWStructuredBuffer<uint>",
        "binding_kind": "rwstructured_buffer",
    },
]

KNOWN_GAPS = [
    "the bake_cluster rasterization segment (cluster_render.glsl proxy-mesh draw) that produces the cluster_render input is NOT replaced (graphics pipeline; native permanently)",
    "the native buffer_clear of cluster_buffer / cluster_render_buffer stays native; the kernel assumes a zeroed destination exactly like the native shader",
    "the render_element_count == 0 early-out stays native (no store dispatch happens in that frame)",
    "runtime hook / native-handle binding not wired (patches 0023-0025, later serial slices)",
    "synthetic parity fixtures use small cluster grids and 32-aligned element capacities; the Godot default max_clustered_elements = 512 deployment scale is a documented assumption, not exercised at full scale offline",
    "GPU-side math parity observation beyond the standalone S6 dispatch smoke (math_parity_evidence.json stays pending_gpu_dispatch until the smoke fills the GPU-observed side)",
]

DOES_NOT_IMPLY = [
    "Godot runtime cluster_store pass completion",
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
        "module": "cluster_store",
        "pass_id": "cluster_store",
        "artifact_kind": "hlsl_bridge_descriptor_layout",
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "entry_point": ENTRY_POINT,
        "target_profile": TARGET_PROFILE,
        "root_constants": 8,
        "root_constant_layout": [dict(entry) for entry in ROOT_CONSTANT_LAYOUT],
        "push_constant_note": (
            "32-byte / 8-dword mirror of Godot's "
            "ClusterBuilderSharedDataRD::ClusterStore::PushConstant "
            "(servers/rendering/renderer_rd/cluster_builder_rd.h L91-100); all "
            "eight fields are u32 and every non-pad field is consumed by the "
            "kernel."
        ),
        "resources": [dict(resource) for resource in RESOURCES],
        # RootConstants parameter at index 0 + single SRV/UAV descriptor table
        # (SRV range t0-t1 then UAV range u0).
        "root_signature_parameters": 2,
        "grx014_mapping": {
            "status": "offline_compile_package",
            "requires_64bit_integer_shader_capability": False,
            "requires_64bit_integer_shader_capability_note": (
                "cluster_store carries no i64 push-constant fields (all 8 dwords "
                "are u32), so the SHADER_INT64 device capability is NOT part of "
                "its binding preflight (same as GRX-013 particles_copy)."
            ),
            "runtime_state": "fallback_only",
            "real_gpu_pass": False,
            "root_constant_bytes": 32,
            "root_constant_dwords": 8,
            "cluster_render_word_stride_bytes": 4,
            "render_element_stride_bytes": 80,
            "cluster_store_word_stride_bytes": 4,
            "dispatch_note": (
                "one thread per cluster; local 8x8x1; dispatch "
                "(ceil(cluster_screen_size.x / 8), ceil(cluster_screen_size.y "
                "/ 8), 1); the store dispatch only runs when "
                "render_element_count > 0 (native guard)"
            ),
        },
    }


def finish(evidence: dict[str, object], status: str, blocker_category: str | None, blocker_summary: str | None, exit_code: int) -> int:
    evidence["status"] = status
    evidence["blocker_category"] = blocker_category
    evidence["blocker_summary"] = blocker_summary
    evidence["runtime_mappable"] = status == "success"
    write_json(EVIDENCE_PATH, evidence)
    print(f"[grx014-hlsl-bridge] status={status} blocker={blocker_category} evidence={EVIDENCE_PATH}")
    return exit_code


def main() -> int:
    if not HLSL_PATH.is_file():
        raise SystemExit(f"missing HLSL kernel source: {HLSL_PATH}")
    BRIDGE_DIR.mkdir(parents=True, exist_ok=True)

    dxc_path = find_tool("dxc.exe")
    dxv_path = find_tool("dxv.exe")

    evidence: dict[str, object] = {
        "pass_id": "cluster_store",
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
            "cluster_store is an all raw-buffer / SSBO pass, so the texture-"
            "intrinsic llc blocker in the precedent policy does NOT apply. The "
            "workaround is used for three different blockers: (1) the DXIL "
            "compute-body lowering accepts only View<global, f32>/ViewMut<global, "
            "f32> buffer views while all three cluster_store buffers are u32-word "
            "SSBOs whose bit patterns an f32 view cannot carry bit-faithfully; "
            "(2) the integer bit operations the kernel is built from (&, |, ~, "
            "<<, >>) exist in MIR but have no DXIL-backend lowering; (3) the "
            "Rurix lang subset has no findLSB/findMSB (firstbitlow/firstbithigh) "
            "intrinsic on any backend."
        ),
        "math_parity_evidence": rel(MATH_PARITY_EVIDENCE_PATH),
        "math_parity_status": MATH_PARITY_STATUS,
        "inputs": {
            "package_manifest": rel(PASS_DIR / "rurix.toml"),
            "entry_file": rel(HLSL_PATH),
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_reference_files": [
                "external/godot-master/servers/rendering/renderer_rd/shaders/cluster_store.glsl",
                "external/godot-master/servers/rendering/renderer_rd/cluster_builder_rd.cpp",
                "external/godot-master/servers/rendering/renderer_rd/cluster_builder_rd.h",
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
        "attempted_binding_kinds": ["structured_buffer", "structured_buffer", "rwstructured_buffer"],
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
        "--example", "emit_grx014_cluster_store_rts0",
        "--",
        rel(CANONICAL_DESCRIPTOR_PATH),
        rel(CANONICAL_RTS0_PATH),
    ]
    cargo_stdout = BRIDGE_DIR / "emit_rts0_stdout.txt"
    cargo_stderr = BRIDGE_DIR / "emit_rts0_stderr.txt"
    cargo_completed = run_command(cargo_argv, cargo_stdout, cargo_stderr)
    evidence["commands"].append({
        "label": "emit_grx014_cluster_store_rts0",
        "argv": cargo_argv,
        "exit_code": cargo_completed.returncode,
        "stdout_path": rel(cargo_stdout),
        "stderr_path": rel(cargo_stderr),
    })
    if cargo_completed.returncode != 0 or not CANONICAL_RTS0_PATH.is_file():
        return finish(evidence, "compile_failed", "rts0_emit_failed",
                      "emit_grx014_cluster_store_rts0 failed; see emit_rts0_stderr.txt", 1)
    evidence["root_signature_generator"] = {
        "kind": "rurixc_binding_layout_example",
        "command": cargo_argv,
        "source_api": "rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}",
        "root_constants": "32_bytes_8_dwords_at_root_parameter_index_0",
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
        "Runtime remains fallback_only: the real dispatch path (a later patch/bridge slice) is opt-in only and armed by RXGD_CAP_CLUSTER_STORE_REAL_PASS (1u<<8); the shipping feature-off bridge fails closed.",
        "The canonical artifacts/ paths carry the raw-buffer hlsl_bridge workaround package: a DXC cs_6_0 DXIL container (validated by dxv), the Rurix-owned RTS0 root signature serialized by rurixc::binding_layout::serialize_rts0 (32-byte / 8-dword ClusterStore::PushConstant root constants + SRV t0 structured buffer + SRV t1 structured buffer + UAV u0 structured buffer), and the descriptor layout with per-slot structured_buffer/structured_buffer/rwstructured_buffer binding kinds.",
        "hlsl_bridge_workaround provenance (buffer-pass variant): NOT rurix_owned. Blockers differ from the texture passes: (1) no u32 buffer views in the DXIL compute-body lowering; (2) no DXIL-backend lowering for the integer bit operations; (3) no findLSB/findMSB intrinsic in the lang subset. The texture-intrinsic llc blocker does NOT apply to this raw-buffer pass.",
        "The kernel math mirrors the complete cluster_store.glsl store segment (single kernel, no mode switches, so no subset cut); the rasterization segment that produces the input stays native. See known_gaps.",
    ]
    return finish(evidence, "success", None, None, 0)


if __name__ == "__main__":
    sys.exit(main())
