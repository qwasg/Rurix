#!/usr/bin/env python3
"""GRX-018: compile the math-equivalent HLSL indirect_args kernel PAIR.

Compiles ``artifacts/hlsl_bridge/indirect_args_write.hlsl`` (the command-block
write kernel) AND ``artifacts/hlsl_bridge/indirect_args_validate.hlsl`` (the
RESIDENT validation red-leg kernel mandated by GRX_PLAN GRX-018) via DXC
(cs_6_0), validates both with DXV, emits the descriptor layout JSON (per-slot
structured_buffer / rwstructured_buffer binding kinds + the 176-byte /
44-dword Rurix-owned parameter block), and synthesizes ONE shared Rurix-owned
RTS0 root signature through ``cargo run --example
emit_grx018_indirect_args_rts0`` (``rurixc::binding_layout::
{infer_root_signature, pack_root_constants, serialize_rts0}``) — both PSOs
bind the same surface, so a single root signature serves the pair.

Route rationale (indirect_args is an all raw-buffer / SSBO pass, so the
GRX-009 texture-intrinsic llc blocker does NOT apply): a rurixc-owned
rx -> DXIL compile is infeasible because (1) the DXIL compute-body lowering
has no u32 buffer views (``View``/``ViewMut`` are f32-only,
``src/rurixc/src/dxil_codegen.rs`` L1754/L1786; f32 views cannot carry
arbitrary u32 payloads bit-faithfully), (2) integer bit ops are not lowered on
the DXIL path (MIR carries them, ``src/rurixc/src/mir.rs`` L643-647), and
(3) the lang subset has no atomic intrinsic (the validation counters need
``InterlockedAdd``). The RTS0 is Rurix-owned; the DXIL/descriptor package is
the owner-approved ``hlsl_bridge_workaround`` (precedent:
``../cluster_store/PASS_CONTRACT.md`` sec 5.3 /
``../luminance_reduction/texture_artifact_provenance_policy.json``).

Tool discovery follows the GRX-009..014 template (``RURIX_DXC_DIR`` /
``RURIX_DXC_NEW_DIR`` env, then the default round-7 extraction dir, then PATH).

Fail-closed: this is an ``hlsl_bridge_workaround`` artifact set, NOT
rurix_owned. It never advances ``runtime_state``/``real_gpu_pass`` and the
pass stays default disabled; math parity stays pending until
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
WRITE_HLSL_PATH = BRIDGE_DIR / "indirect_args_write.hlsl"
VALIDATE_HLSL_PATH = BRIDGE_DIR / "indirect_args_validate.hlsl"
WRITE_BRIDGE_DXIL_PATH = BRIDGE_DIR / "indirect_args_write.dxil"
VALIDATE_BRIDGE_DXIL_PATH = BRIDGE_DIR / "indirect_args_validate.dxil"
CANONICAL_DXIL_PATH = ARTIFACTS_DIR / "indirect_args.dxil"
CANONICAL_VALIDATE_DXIL_PATH = ARTIFACTS_DIR / "indirect_args_validate.dxil"
CANONICAL_RTS0_PATH = ARTIFACTS_DIR / "indirect_args.rts0.bin"
CANONICAL_DESCRIPTOR_PATH = ARTIFACTS_DIR / "indirect_args_descriptor_layout.json"
EVIDENCE_PATH = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"
DEFAULT_DXC_DIR = pathlib.Path(r"H:\dxc-round7\extracted\bin\x64")

TARGET_PROFILE = "cs_6_0"
ENTRY_POINT = "main"

MAX_SURFACES = 8
COMMAND_STRIDE_DWORDS = 5  # Godot INDIRECT_MULTIMESH_COMMAND_STRIDE

MATH_PARITY_STATUS = "indirect_args_cpu_reference_proven_pending_gpu_dispatch"


# Canonical GRX-018 indirect_args root-constant layout: a Rurix-owned 176-byte
# / 44-dword parameter block (there is NO native push constant to mirror; the
# native producer is CPU buffer_update code, not a dispatch). Every field is a
# u32 carried as a 1-dword slot (the RTS0 only encodes dword layout; the
# "type" here is the true semantic type for the descriptor JSON).
def _rc(name: str, type_: str, order: int) -> dict[str, object]:
    return {
        "name": name,
        "type": type_,
        "order": order,
        "root_parameter_index": 0,
        "dword_offset": order,
        "dword_size": 1,
    }


def _root_constant_layout() -> list[dict[str, object]]:
    layout = [
        _rc("surface_count", "u32", 0),
        _rc("max_instance_count", "u32", 1),
        _rc("survivor_count_word_offset", "u32", 2),
        _rc("pad0", "u32", 3),
    ]
    fields = [
        "index_count",
        "instance_count_reserved",
        "first_index",
        "vertex_offset",
        "first_instance",
    ]
    order = 4
    for s in range(MAX_SURFACES):
        for field in fields:
            layout.append(_rc(f"surface{s}_{field}", "u32", order))
            order += 1
    assert order == 44
    return layout


ROOT_CONSTANT_LAYOUT = _root_constant_layout()

RESOURCES = [
    {
        "name": "src_survivor_counts",
        "class": "t",
        "register": 0,
        "space": 0,
        "count": 1,
        "hlsl_type": "StructuredBuffer<uint>",
        "binding_kind": "structured_buffer",
    },
    {
        "name": "dst_command_buffer",
        "class": "u",
        "register": 0,
        "space": 0,
        "count": 1,
        "hlsl_type": "RWStructuredBuffer<uint>",
        "binding_kind": "rwstructured_buffer",
    },
    {
        "name": "dst_validation",
        "class": "u",
        "register": 1,
        "space": 0,
        "count": 1,
        "hlsl_type": "RWStructuredBuffer<uint>",
        "binding_kind": "rwstructured_buffer",
    },
]

KNOWN_GAPS = [
    "surface_count > 8 (MAX_SURFACES) unsupported -> fallback",
    "one multimesh per dispatch only; no cross-multimesh batching; no draw_count > 1 multi-draw blocks",
    "visible_instances == -1 all-visible sentinel not modeled (producer counts are absolute u32 in [0, max_instance_count])",
    "per-surface distinct survivor counts unsupported (native path shares one count across surfaces; kernel mirrors that)",
    "nonzero first_index/vertex_offset/first_instance never occur natively (CPU fill zero-inits dwords 2-4); template fidelity for nonzero statics is proven offline only",
    "GRX-015/016 producer interface declared but not landed (PASS_CONTRACT.md section 4.1); offline slices use synthetic survivor buffers",
    "render_forward_mobile indirect call-site twin out of scope",
    "the runtime hook (patches 0033-0035) is a later serial slice; this slice is the offline face only",
    "same-frame validation readback cost unmeasured (correctness-first design)",
    "GPU-side math parity observation pending a real dispatch (math_parity_evidence.json)",
]

DOES_NOT_IMPLY = [
    "Godot runtime indirect_args pass completion",
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
        "module": "indirect_args",
        "pass_id": "indirect_args",
        "artifact_kind": "hlsl_bridge_descriptor_layout",
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "entry_point": ENTRY_POINT,
        "target_profile": TARGET_PROFILE,
        "root_constants": 44,
        "root_constant_layout": [dict(entry) for entry in ROOT_CONSTANT_LAYOUT],
        "push_constant_note": (
            "176-byte / 44-dword Rurix-owned parameter block (NO native push "
            "constant exists: the native producer is CPU buffer_update code in "
            "mesh_storage.cpp, not a dispatch): surface_count / "
            "max_instance_count / survivor_count_word_offset / pad0 + 8 "
            "per-surface 5-dword command templates. The template array is "
            "carried as uint4[10] in the HLSL cbuffer so the 40 template "
            "dwords stay tightly packed. surface{s}_instance_count_reserved "
            "MUST be 0 (dword 1 is the GPU-dynamic instance_count)."
        ),
        "resources": [dict(resource) for resource in RESOURCES],
        # RootConstants parameter at index 0 + single SRV/UAV descriptor table.
        "root_signature_parameters": 2,
        "kernels": {
            "write": {
                "hlsl": rel(WRITE_HLSL_PATH),
                "canonical_dxil": rel(CANONICAL_DXIL_PATH),
                "entry_point": ENTRY_POINT,
                "role": "generate the complete 5-dword command block per surface (dword 1 = min(survivors, max_instance_count); dwords 0/2/3/4 = b0 template backfill)",
                "references_dst_validation": False,
            },
            "validate": {
                "hlsl": rel(VALIDATE_HLSL_PATH),
                "canonical_dxil": rel(CANONICAL_VALIDATE_DXIL_PATH),
                "entry_point": ENTRY_POINT,
                "role": "RESIDENT red leg (GRX_PLAN GRX-018): dword-by-dword compare vs b0 template + recomputed expected instance_count; per-surface bitmask + InterlockedAdd mismatch/clamp counters; any nonzero counter -> immediate fallback",
                "references_dst_validation": True,
            },
        },
        "shared_root_signature_note": (
            "both kernels declare the identical binding surface (t0/u0/u1/b0), "
            "so ONE Rurix-owned RTS0 serves both PSOs; the write kernel never "
            "references u1 (a root-signature superset is legal)."
        ),
        "grx018_mapping": {
            "status": "offline_compile_package",
            "requires_64bit_integer_shader_capability": False,
            "requires_64bit_integer_shader_capability_note": (
                "indirect_args carries no i64 b0 fields (all u32), so the "
                "SHADER_INT64 device capability is NOT part of its preflight "
                "(GRX-013/014 buffer-pass precedent)."
            ),
            "runtime_state": "fallback_only",
            "real_gpu_pass": False,
            "root_constant_bytes": 176,
            "root_constant_dwords": 44,
            "max_surfaces": MAX_SURFACES,
            "command_block_stride_dwords": COMMAND_STRIDE_DWORDS,
            "command_block_stride_note": (
                "Godot INDIRECT_MULTIMESH_COMMAND_STRIDE = 5 u32 dwords per "
                "surface (mesh_storage.h L62-64); dword layout in "
                "resource_mapping.md (indexed naming; non-indexed reading is "
                "value-equivalent because dwords 2-4 are natively zero)."
            ),
            "validation_buffer_layout": (
                "word 0 = mismatch_count, word 1 = clamp_trigger_count, "
                "word 2+s = per-surface bitmask (bits 0-4 dword mismatch, "
                "bit 5 in-buffer clamp violation, bit 6 producer clamp "
                "trigger); buffer zeroed before the validate dispatch."
            ),
            "dispatch_shape": "(1, 1, 1) groups, local 64x1x1; write -> UAV barrier -> validate -> readback",
        },
    }


def finish(evidence: dict[str, object], status: str, blocker_category: str | None, blocker_summary: str | None, exit_code: int) -> int:
    evidence["status"] = status
    evidence["blocker_category"] = blocker_category
    evidence["blocker_summary"] = blocker_summary
    evidence["runtime_mappable"] = status == "success"
    write_json(EVIDENCE_PATH, evidence)
    print(f"[grx018-hlsl-bridge] status={status} blocker={blocker_category} evidence={EVIDENCE_PATH}")
    return exit_code


def main() -> int:
    for hlsl_path in (WRITE_HLSL_PATH, VALIDATE_HLSL_PATH):
        if not hlsl_path.is_file():
            raise SystemExit(f"missing HLSL kernel source: {hlsl_path}")
    BRIDGE_DIR.mkdir(parents=True, exist_ok=True)

    dxc_path = find_tool("dxc.exe")
    dxv_path = find_tool("dxv.exe")

    evidence: dict[str, object] = {
        "pass_id": "indirect_args",
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
            "indirect_args is an all raw-buffer / SSBO pass, so the "
            "texture-intrinsic llc blocker in the precedent policy does NOT "
            "apply. The workaround is used for three different blockers: "
            "(1) the DXIL compute-body lowering has no u32 buffer views "
            "(View/ViewMut are f32-only, dxil_codegen.rs L1754/L1786); "
            "(2) integer bit ops are not lowered on the DXIL path (MIR "
            "carries BinOp::BitAnd/BitOr/Shl/Shr, mir.rs L643-647); (3) the "
            "lang subset has no atomic intrinsic (the resident validation "
            "kernel's counters need InterlockedAdd)."
        ),
        "math_parity_evidence": rel(MATH_PARITY_EVIDENCE_PATH),
        "math_parity_status": MATH_PARITY_STATUS,
        "inputs": {
            "package_manifest": rel(PASS_DIR / "rurix.toml"),
            "entry_files": [rel(WRITE_HLSL_PATH), rel(VALIDATE_HLSL_PATH)],
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_reference_files": [
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp",
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/mesh_storage.h",
                "external/godot-master/servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp",
            ],
            "godot_reference_note": (
                "NO native compute shader exists for this pass: the math "
                "target is the CPU command-block producer "
                "(_multimesh_set_mesh L1674-1696 static fill + "
                "_multimesh_set_visible_instances L2210 dword-1 buffer_update) "
                "and the draw_list_draw_indirect consumer contract "
                "(render_forward_clustered.cpp L602/L610)."
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
            "write": {
                "path": rel(WRITE_HLSL_PATH),
                "entry_point": ENTRY_POINT,
                "target_profile": TARGET_PROFILE,
                "sha256": sha256_of_file(WRITE_HLSL_PATH),
            },
            "validate": {
                "path": rel(VALIDATE_HLSL_PATH),
                "entry_point": ENTRY_POINT,
                "target_profile": TARGET_PROFILE,
                "sha256": sha256_of_file(VALIDATE_HLSL_PATH),
            },
        },
        "commands": [],
        "attempted_binding_kinds": ["structured_buffer", "rwstructured_buffer", "rwstructured_buffer"],
        "runtime_mappable": False,
        "known_gaps": list(KNOWN_GAPS),
        "does_not_imply": list(DOES_NOT_IMPLY),
    }

    if dxc_path is None:
        return finish(evidence, "skip", "dxc_missing", "dxc.exe not found (set RURIX_DXC_DIR)", 0)

    # 1) DXC compile + 2) DXV validation, for BOTH kernels.
    kernel_jobs = [
        ("write", WRITE_HLSL_PATH, WRITE_BRIDGE_DXIL_PATH),
        ("validate", VALIDATE_HLSL_PATH, VALIDATE_BRIDGE_DXIL_PATH),
    ]
    dxil_provenance: dict[str, object] = {}
    for label, hlsl_path, dxil_path in kernel_jobs:
        dxc_argv = [str(dxc_path), "-T", TARGET_PROFILE, "-E", ENTRY_POINT, "-Qstrip_debug",
                    "-Fo", str(dxil_path), str(hlsl_path)]
        dxc_stdout = BRIDGE_DIR / f"dxc_{label}_stdout.txt"
        dxc_stderr = BRIDGE_DIR / f"dxc_{label}_stderr.txt"
        dxc_completed = run_command(dxc_argv, dxc_stdout, dxc_stderr)
        evidence["commands"].append({
            "label": f"dxc_compile_{label}",
            "argv": dxc_argv,
            "exit_code": dxc_completed.returncode,
            "stdout_path": rel(dxc_stdout),
            "stderr_path": rel(dxc_stderr),
        })
        if dxc_completed.returncode != 0 or not dxil_path.is_file():
            return finish(evidence, "compile_failed", "dxil_container_missing",
                          f"dxc compile of the {label} kernel failed; see dxc_{label}_stderr.txt", 1)

        if dxv_path is None:
            return finish(evidence, "skip", "dxv_missing", "dxv.exe not found (set RURIX_DXC_DIR)", 0)
        dxv_argv = [str(dxv_path), str(dxil_path)]
        dxv_stdout = BRIDGE_DIR / f"dxv_{label}_stdout.txt"
        dxv_stderr = BRIDGE_DIR / f"dxv_{label}_stderr.txt"
        dxv_completed = run_command(dxv_argv, dxv_stdout, dxv_stderr)
        evidence["commands"].append({
            "label": f"dxv_validate_{label}",
            "argv": dxv_argv,
            "exit_code": dxv_completed.returncode,
            "stdout_path": rel(dxv_stdout),
            "stderr_path": rel(dxv_stderr),
        })
        dxil_provenance[label] = {
            "produced_by": "dxc",
            "compiler_version": command_output(dxc_path, ["--version"]),
            "target_profile": TARGET_PROFILE,
            "entry_point": ENTRY_POINT,
            "hlsl_source": {"path": rel(hlsl_path), "sha256": sha256_of_file(hlsl_path)},
            "validation": {
                "tool": "dxv.exe",
                "status": "pass" if dxv_completed.returncode == 0 else "fail",
                "exit_code": dxv_completed.returncode,
            },
        }
        if dxv_completed.returncode != 0:
            evidence["dxil_provenance"] = dxil_provenance
            return finish(evidence, "validation_failed", "dxv_validation_failed",
                          f"dxv rejected the {label} DXIL container; see dxv_{label}_stderr.txt", 1)
    evidence["dxil_provenance"] = dxil_provenance

    # 3) Descriptor layout (canonical path).
    write_json(CANONICAL_DESCRIPTOR_PATH, descriptor_layout_doc())

    # 4) ONE shared Rurix-owned RTS0 via the binding-layout example.
    cargo_argv = [
        "cargo", "run", "-q", "-p", "rurixc",
        "--features", "dxil-backend shader-stages",
        "--example", "emit_grx018_indirect_args_rts0",
        "--",
        rel(CANONICAL_DESCRIPTOR_PATH),
        rel(CANONICAL_RTS0_PATH),
    ]
    cargo_stdout = BRIDGE_DIR / "emit_rts0_stdout.txt"
    cargo_stderr = BRIDGE_DIR / "emit_rts0_stderr.txt"
    cargo_completed = run_command(cargo_argv, cargo_stdout, cargo_stderr)
    evidence["commands"].append({
        "label": "emit_grx018_indirect_args_rts0",
        "argv": cargo_argv,
        "exit_code": cargo_completed.returncode,
        "stdout_path": rel(cargo_stdout),
        "stderr_path": rel(cargo_stderr),
    })
    if cargo_completed.returncode != 0 or not CANONICAL_RTS0_PATH.is_file():
        return finish(evidence, "compile_failed", "rts0_emit_failed",
                      "emit_grx018_indirect_args_rts0 failed; see emit_rts0_stderr.txt", 1)
    evidence["root_signature_generator"] = {
        "kind": "rurixc_binding_layout_example",
        "command": cargo_argv,
        "source_api": "rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}",
        "root_constants": "176_bytes_44_dwords_at_root_parameter_index_0",
        "shared_by_both_kernels": True,
    }

    # 5) Publish the canonical DXIL copies (byte-identical to bridge outputs).
    shutil.copyfile(WRITE_BRIDGE_DXIL_PATH, CANONICAL_DXIL_PATH)
    shutil.copyfile(VALIDATE_BRIDGE_DXIL_PATH, CANONICAL_VALIDATE_DXIL_PATH)

    evidence["artifacts"] = {
        "dxil": {
            **artifact_entry(CANONICAL_DXIL_PATH, "dxil_container", True),
            "semantic_status": "lowered_compute_body",
            "kernel": "write",
        },
        "dxil_validate": {
            **artifact_entry(CANONICAL_VALIDATE_DXIL_PATH, "dxil_container", True),
            "semantic_status": "lowered_compute_body",
            "kernel": "validate",
        },
        "root_signature": artifact_entry(CANONICAL_RTS0_PATH, "rurix_owned_rts0_root_signature", True),
        "descriptor_layout": artifact_entry(CANONICAL_DESCRIPTOR_PATH, "descriptor_layout_json", True),
    }
    evidence["notes"] = [
        "Runtime remains fallback_only: the real dispatch path (a later patch/bridge slice) is opt-in only and armed by RXGD_CAP_INDIRECT_ARGS_REAL_PASS (1u<<12); the shipping feature-off bridge fails closed.",
        "The canonical artifacts/ paths carry the raw-buffer hlsl_bridge workaround package: TWO DXC cs_6_0 DXIL containers (write kernel + RESIDENT validation red-leg kernel, both validated by dxv), ONE shared Rurix-owned RTS0 root signature serialized by rurixc::binding_layout::serialize_rts0 (176-byte / 44-dword parameter block + SRV t0 + UAV u0 + UAV u1 structured buffers), and the descriptor layout with per-slot binding kinds. The (later) S4 bridge gate bakes FOUR digests.",
        "hlsl_bridge_workaround provenance (buffer-pass variant): NOT rurix_owned. Blockers: (1) no u32 buffer views on the DXIL path; (2) integer bit ops not lowered; (3) no atomic intrinsic. The texture-intrinsic llc blocker does NOT apply to this raw-buffer pass.",
        "The validation kernel is a RESIDENT red leg per GRX_PLAN GRX-018 (any validation mismatch -> immediate fallback); the runtime chain is write -> UAV barrier -> validate -> readback -> copy-if-clean (PASS_CONTRACT.md section 5.4).",
    ]
    return finish(evidence, "success", None, None, 0)


if __name__ == "__main__":
    sys.exit(main())
