#!/usr/bin/env python3
"""GRX-016: compile the three-variant HLSL instance_compaction kernel chain.

Compiles ``artifacts/hlsl_bridge/instance_compaction_{scan_local,scan_groups,
scatter}.hlsl`` via DXC (cs_6_0), validates each container with DXV, emits the
single descriptor layout JSON (three ``variants`` entries: per-slot
structured_buffer / rwstructured_buffer binding kinds + the shared Rurix-
defined 32-byte / 8-dword CompactionParams root-constant layout), and
synthesizes a Rurix-owned RTS0 root signature PER VARIANT through ``cargo run
--example emit_grx016_instance_compaction_rts0 <variant> ...``
(``rurixc::binding_layout::{infer_root_signature, pack_root_constants,
serialize_rts0}``).

Route rationale (instance_compaction is an all raw-buffer / SSBO pass, so the
GRX-009 texture-intrinsic llc blocker does NOT apply): a rurixc-owned
rx -> DXIL compile of the chain is infeasible because (1) the DXIL
compute-body lowering has f32-only buffer views (u32 bitmask/prefix words
cannot be carried bit-faithfully), (2) the integer bit ops the bitmask decode
needs have no DXIL-backend lowering, and (3) the DXIL compute-body lowering
rejects ``shared let`` and has no barrier lowering, so the groupshared prefix
scan cannot be expressed (``shared let``/``barrier()`` are NVPTX-only). The
RTS0 is Rurix-owned; the DXIL/descriptor package is the owner-approved
``hlsl_bridge_workaround`` (precedent:
``../luminance_reduction/texture_artifact_provenance_policy.json`` via the
GRX-013/014 buffer-pass variants).

Tool discovery follows the GRX-009..014 template (``RURIX_DXC_DIR`` /
``RURIX_DXC_NEW_DIR`` env, then the default round-7 extraction dir, then
PATH).

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
CANONICAL_DESCRIPTOR_PATH = ARTIFACTS_DIR / "instance_compaction_descriptor_layout.json"
EVIDENCE_PATH = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"
DEFAULT_DXC_DIR = pathlib.Path(r"H:\dxc-round7\extracted\bin\x64")

TARGET_PROFILE = "cs_6_0"
ENTRY_POINT = "main"
GROUP_SIZE = 256
MAX_INSTANCES = GROUP_SIZE * GROUP_SIZE  # 65536: single-level group hierarchy

MATH_PARITY_STATUS = "instance_compaction_cpu_reference_proven_pending_gpu_dispatch"

# Canonical GRX-016 root-constant layout: the Rurix-defined 32-byte / 8-dword
# CompactionParams block, shared byte-identical by all three variants at
# root_parameter_index 0. There is NO native Godot push constant to mirror
# (Godot has no native compaction pass); see resource_mapping.md. Every field
# is a 1-dword u32 (the RTS0 only encodes dword layout; the "type" here is
# the true semantic type for the descriptor JSON).
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
    _rc("total_instances", "u32", 0),
    _rc("bitmask_words", "u32", 1),
    _rc("num_groups", "u32", 2),
    _rc("transform_stride_vec4", "u32", 3),
    _rc("pad0", "u32", 4),
    _rc("pad1", "u32", 5),
    _rc("pad2", "u32", 6),
    _rc("pad3", "u32", 7),
]


def _res(name: str, cls: str, register: int, hlsl_type: str, kind: str) -> dict[str, object]:
    return {
        "name": name,
        "class": cls,
        "register": register,
        "space": 0,
        "count": 1,
        "hlsl_type": hlsl_type,
        "binding_kind": kind,
    }


# The three-dispatch chain (PASS_CONTRACT.md sec 5.1): D1 scan_local ->
# UAV barrier -> D2 scan_groups -> UAV barrier -> D3 scatter.
VARIANTS = [
    {
        "variant": "scan_local",
        "dispatch": "(ceil(N/256), 1, 1)",
        "resources": [
            _res("visibility_mask", "t", 0, "StructuredBuffer<uint>", "structured_buffer"),
            _res("local_prefix", "u", 0, "RWStructuredBuffer<uint>", "rwstructured_buffer"),
            _res("group_totals", "u", 1, "RWStructuredBuffer<uint>", "rwstructured_buffer"),
        ],
    },
    {
        "variant": "scan_groups",
        "dispatch": "(1, 1, 1)  [requires num_groups <= 256, i.e. N <= 65536]",
        "resources": [
            _res("group_totals", "t", 0, "StructuredBuffer<uint>", "structured_buffer"),
            _res("group_offsets", "u", 0, "RWStructuredBuffer<uint>", "rwstructured_buffer"),
            _res("survivor_count", "u", 1, "RWStructuredBuffer<uint>", "rwstructured_buffer"),
        ],
    },
    {
        "variant": "scatter",
        "dispatch": "(ceil(N/256), 1, 1)",
        "resources": [
            _res("visibility_mask", "t", 0, "StructuredBuffer<uint>", "structured_buffer"),
            _res("src_transforms", "t", 1, "StructuredBuffer<uint4>", "structured_buffer"),
            _res("local_prefix", "t", 2, "StructuredBuffer<uint>", "structured_buffer"),
            _res("group_offsets", "t", 3, "StructuredBuffer<uint>", "structured_buffer"),
            _res("dst_transforms", "u", 0, "RWStructuredBuffer<uint4>", "rwstructured_buffer"),
        ],
    },
]

KNOWN_GAPS = [
    "3D transform-only MultiMesh layout (stride 12 floats = 3 float4 per instance); colors/custom_data channels (stride 16/20) and the 2D layout (stride 8) unsupported",
    "motion-vector double-buffer MultiMesh layout unsupported (current/previous halves would compact to different ranks)",
    "opaque-only ordering contract: alpha-blended / transparent materials and absolute-instance-index-keyed consumers are out of scope (PASS_CONTRACT.md sec 5.2)",
    "capacity bound total_instances <= 65536 (single-level group hierarchy: num_groups <= 256); larger N needs a third scan level",
    "depends on the GRX-015 gpu_culling visibility bitmask (u32[ceil(N/32)]); GRX-015 had not landed its pass package when this slice was authored — the later S4 gate must re-verify the declared interface (PASS_CONTRACT.md sec 5.3)",
    "survivor-count consumption wiring (CPU readback multimesh_set_visible_instances vs GRX-018 indirect args) is a later-slice / owner decision",
    "runtime hook / native-handle binding not wired (patches 0030-0032, later serial slices; patch 0030 additionally blocked on GRX-015 patches 0027-0029)",
    "GPU-side math parity observation pending a real three-dispatch chain (math_parity_evidence.json)",
]

DOES_NOT_IMPLY = [
    "Godot runtime instance_compaction pass completion",
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
        "module": "instance_compaction",
        "pass_id": "instance_compaction",
        "artifact_kind": "hlsl_bridge_descriptor_layout",
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "entry_point": ENTRY_POINT,
        "target_profile": TARGET_PROFILE,
        "root_constants": 8,
        "root_constant_layout": [dict(entry) for entry in ROOT_CONSTANT_LAYOUT],
        "push_constant_note": (
            "Rurix-defined 32-byte / 8-dword CompactionParams block, shared "
            "byte-identical by all three variants at root_parameter_index 0. "
            "There is NO native Godot push constant to mirror: Godot has no "
            "native compaction pass; the pass plugs into the CPU-side 'draw "
            "the first N instances' contract (mesh_storage.h "
            "multimesh_get_instances_to_draw L721-728). No i64 fields, so "
            "SHADER_INT64 is NOT part of this pass's preflight."
        ),
        "group_size": GROUP_SIZE,
        "max_instances": MAX_INSTANCES,
        "dispatch_chain": [
            "D1 scan_local (ceil(N/256),1,1)",
            "UAV barrier: local_prefix + group_totals",
            "D2 scan_groups (1,1,1)  [gate: num_groups <= 256]",
            "UAV barrier: group_offsets + survivor_count",
            "D3 scatter (ceil(N/256),1,1)",
            "transition: dst_transforms -> draw read; survivor_count -> readback/indirect consumer",
        ],
        "variants": [
            {
                "variant": entry["variant"],
                "hlsl": rel(BRIDGE_DIR / f"instance_compaction_{entry['variant']}.hlsl"),
                "dispatch": entry["dispatch"],
                "resources": [dict(res) for res in entry["resources"]],
                # RootConstants parameter at index 0 + one SRV/UAV descriptor table.
                "root_signature_parameters": 2,
            }
            for entry in VARIANTS
        ],
        "grx016_mapping": {
            "status": "offline_compile_package",
            "requires_64bit_integer_shader_capability": False,
            "requires_64bit_integer_shader_capability_note": (
                "instance_compaction carries no i64 push-constant fields "
                "(CompactionParams is all u32), so the SHADER_INT64 device "
                "capability is NOT part of its preflight (same as GRX-013/014)."
            ),
            "runtime_state": "fallback_only",
            "real_gpu_pass": False,
            "root_constant_bytes": 32,
            "root_constant_dwords": 8,
            "transform_stride_floats": 12,
            "transform_stride_vec4": 3,
            "transform_stride_note": (
                "3D transform-only MultiMesh stride: 3 float4 rows of "
                "(basis row, origin component); mesh_storage.cpp "
                "_multimesh_allocate_data L1577-1580 / "
                "_multimesh_instance_set_transform L1900-1911"
            ),
            "grx015_dependency": (
                "visibility_mask is the GRX-015 gpu_culling output "
                "(u32[ceil(N/32)], bit p = word p>>5 bit p&31, 1 = survives; "
                "tail bits beyond N-1 are don't-care)"
            ),
            "ordering_contract": (
                "stable compaction (rank = exclusive prefix by index); "
                "opaque-only; alpha-blended and absolute-instance-index-keyed "
                "consumers out of scope (PASS_CONTRACT.md sec 5.2)"
            ),
        },
    }


def finish(evidence: dict[str, object], status: str, blocker_category: str | None, blocker_summary: str | None, exit_code: int) -> int:
    evidence["status"] = status
    evidence["blocker_category"] = blocker_category
    evidence["blocker_summary"] = blocker_summary
    evidence["runtime_mappable"] = status == "success"
    write_json(EVIDENCE_PATH, evidence)
    print(f"[grx016-hlsl-bridge] status={status} blocker={blocker_category} evidence={EVIDENCE_PATH}")
    return exit_code


def main() -> int:
    for entry in VARIANTS:
        hlsl_path = BRIDGE_DIR / f"instance_compaction_{entry['variant']}.hlsl"
        if not hlsl_path.is_file():
            raise SystemExit(f"missing HLSL kernel source: {hlsl_path}")
    BRIDGE_DIR.mkdir(parents=True, exist_ok=True)

    dxc_path = find_tool("dxc.exe")
    dxv_path = find_tool("dxv.exe")

    evidence: dict[str, object] = {
        "pass_id": "instance_compaction",
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
            "instance_compaction is an all raw-buffer / SSBO pass, so the "
            "texture-intrinsic llc blocker in the precedent policy does NOT "
            "apply. The workaround is used for three different blockers: (1) "
            "the DXIL compute-body lowering has f32-only buffer views "
            "(dxil_codegen.rs L1754/L1786) while the bitmask/prefix buffers "
            "are u32 words; (2) the integer bit ops the bitmask decode needs "
            "(BinOp::BitAnd/Shl/Shr, mir.rs L643-647) have no DXIL lowering; "
            "(3) the DXIL compute-body lowering rejects `shared let` "
            "(dxil_codegen.rs L921-924) and has no barrier lowering, so the "
            "groupshared prefix scan is NVPTX-only."
        ),
        "math_parity_evidence": rel(MATH_PARITY_EVIDENCE_PATH),
        "math_parity_status": MATH_PARITY_STATUS,
        "inputs": {
            "package_manifest": rel(PASS_DIR / "rurix.toml"),
            "entry_files": [
                rel(BRIDGE_DIR / f"instance_compaction_{entry['variant']}.hlsl")
                for entry in VARIANTS
            ],
            "rurix_math_source": rel(PASS_DIR / "src" / "lib.rx"),
            "godot_reference_files": [
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp",
                "external/godot-master/servers/rendering/renderer_rd/storage_rd/mesh_storage.h",
                "external/godot-master/servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp",
            ],
            "godot_reference_note": (
                "no native Godot compaction shader exists; the references pin "
                "the MultiMesh 12-float stride, the 3-float4-row transform "
                "layout, and the 'draw the first N instances' consumption "
                "contract this pass plugs into"
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
        "commands": [],
        "variants": [],
        "attempted_binding_kinds": ["structured_buffer", "rwstructured_buffer"],
        "runtime_mappable": False,
        "known_gaps": list(KNOWN_GAPS),
        "does_not_imply": list(DOES_NOT_IMPLY),
    }

    if dxc_path is None:
        return finish(evidence, "skip", "dxc_missing", "dxc.exe not found (set RURIX_DXC_DIR)", 0)
    if dxv_path is None:
        return finish(evidence, "skip", "dxv_missing", "dxv.exe not found (set RURIX_DXC_DIR)", 0)

    dxc_version = command_output(dxc_path, ["--version"])

    # 1+2) Per-variant DXC compile + DXV validation.
    variant_records: list[dict[str, object]] = []
    for entry in VARIANTS:
        variant = str(entry["variant"])
        hlsl_path = BRIDGE_DIR / f"instance_compaction_{variant}.hlsl"
        bridge_dxil_path = BRIDGE_DIR / f"instance_compaction_{variant}.dxil"

        dxc_argv = [str(dxc_path), "-T", TARGET_PROFILE, "-E", ENTRY_POINT, "-Qstrip_debug",
                    "-Fo", str(bridge_dxil_path), str(hlsl_path)]
        dxc_stdout = BRIDGE_DIR / f"dxc_stdout_{variant}.txt"
        dxc_stderr = BRIDGE_DIR / f"dxc_stderr_{variant}.txt"
        dxc_completed = run_command(dxc_argv, dxc_stdout, dxc_stderr)
        evidence["commands"].append({
            "label": f"dxc_compile_{variant}",
            "argv": dxc_argv,
            "exit_code": dxc_completed.returncode,
            "stdout_path": rel(dxc_stdout),
            "stderr_path": rel(dxc_stderr),
        })
        if dxc_completed.returncode != 0 or not bridge_dxil_path.is_file():
            return finish(evidence, "compile_failed", "dxil_container_missing",
                          f"dxc compile failed for variant {variant}; see dxc_stderr_{variant}.txt", 1)

        dxv_argv = [str(dxv_path), str(bridge_dxil_path)]
        dxv_stdout = BRIDGE_DIR / f"dxv_stdout_{variant}.txt"
        dxv_stderr = BRIDGE_DIR / f"dxv_stderr_{variant}.txt"
        dxv_completed = run_command(dxv_argv, dxv_stdout, dxv_stderr)
        evidence["commands"].append({
            "label": f"dxv_validate_{variant}",
            "argv": dxv_argv,
            "exit_code": dxv_completed.returncode,
            "stdout_path": rel(dxv_stdout),
            "stderr_path": rel(dxv_stderr),
        })
        variant_records.append({
            "variant": variant,
            "hlsl": {"path": rel(hlsl_path), "sha256": sha256_of_file(hlsl_path)},
            "dxil_provenance": {
                "produced_by": "dxc",
                "compiler_version": dxc_version,
                "target_profile": TARGET_PROFILE,
                "entry_point": ENTRY_POINT,
                "validation": {
                    "tool": "dxv.exe",
                    "status": "pass" if dxv_completed.returncode == 0 else "fail",
                    "exit_code": dxv_completed.returncode,
                },
            },
        })
        if dxv_completed.returncode != 0:
            evidence["variants"] = variant_records
            return finish(evidence, "validation_failed", "dxv_validation_failed",
                          f"dxv rejected the {variant} DXIL container; see dxv_stderr_{variant}.txt", 1)

    evidence["variants"] = variant_records

    # 3) Descriptor layout (single canonical JSON with the three variants).
    write_json(CANONICAL_DESCRIPTOR_PATH, descriptor_layout_doc())

    # 4) Rurix-owned RTS0 per variant via the binding-layout example.
    for entry in VARIANTS:
        variant = str(entry["variant"])
        canonical_rts0_path = ARTIFACTS_DIR / f"instance_compaction_{variant}.rts0.bin"
        cargo_argv = [
            "cargo", "run", "-q", "-p", "rurixc",
            "--features", "dxil-backend shader-stages",
            "--example", "emit_grx016_instance_compaction_rts0",
            "--",
            variant,
            rel(CANONICAL_DESCRIPTOR_PATH),
            rel(canonical_rts0_path),
        ]
        cargo_stdout = BRIDGE_DIR / f"emit_rts0_stdout_{variant}.txt"
        cargo_stderr = BRIDGE_DIR / f"emit_rts0_stderr_{variant}.txt"
        cargo_completed = run_command(cargo_argv, cargo_stdout, cargo_stderr)
        evidence["commands"].append({
            "label": f"emit_grx016_instance_compaction_rts0_{variant}",
            "argv": cargo_argv,
            "exit_code": cargo_completed.returncode,
            "stdout_path": rel(cargo_stdout),
            "stderr_path": rel(cargo_stderr),
        })
        if cargo_completed.returncode != 0 or not canonical_rts0_path.is_file():
            return finish(evidence, "compile_failed", "rts0_emit_failed",
                          f"emit_grx016_instance_compaction_rts0 {variant} failed; see emit_rts0_stderr_{variant}.txt", 1)
    evidence["root_signature_generator"] = {
        "kind": "rurixc_binding_layout_example",
        "example": "emit_grx016_instance_compaction_rts0 <variant> <descriptor_layout.json> <out.bin>",
        "source_api": "rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}",
        "root_constants": "32_bytes_8_dwords_at_root_parameter_index_0_shared_by_all_variants",
    }

    # 5) Publish the canonical DXIL copies (byte-identical to the bridge outputs).
    for entry in VARIANTS:
        variant = str(entry["variant"])
        shutil.copyfile(
            BRIDGE_DIR / f"instance_compaction_{variant}.dxil",
            ARTIFACTS_DIR / f"instance_compaction_{variant}.dxil",
        )

    artifacts: dict[str, object] = {}
    for entry in VARIANTS:
        variant = str(entry["variant"])
        artifacts[f"dxil_{variant}"] = {
            **artifact_entry(ARTIFACTS_DIR / f"instance_compaction_{variant}.dxil", "dxil_container", True),
            "semantic_status": "lowered_compute_body",
        }
        artifacts[f"root_signature_{variant}"] = artifact_entry(
            ARTIFACTS_DIR / f"instance_compaction_{variant}.rts0.bin",
            "rurix_owned_rts0_root_signature", True)
    artifacts["descriptor_layout"] = artifact_entry(
        CANONICAL_DESCRIPTOR_PATH, "descriptor_layout_json", True)
    evidence["artifacts"] = artifacts

    evidence["notes"] = [
        "Runtime remains fallback_only: the real dispatch path (a later patch/bridge slice) is opt-in only and armed by RXGD_CAP_INSTANCE_COMPACTION_REAL_PASS (1u<<10); the shipping feature-off bridge fails closed.",
        "The canonical artifacts/ paths carry the raw-buffer hlsl_bridge workaround package for the THREE-dispatch chain: per variant a DXC cs_6_0 DXIL container (validated by dxv) and a Rurix-owned RTS0 root signature serialized by rurixc::binding_layout::serialize_rts0 (shared 32-byte / 8-dword CompactionParams root constants + the per-variant structured-buffer SRV/UAV surface), plus ONE descriptor layout JSON with the three variants and the dispatch/barrier chain.",
        "hlsl_bridge_workaround provenance (buffer-pass variant): NOT rurix_owned. Blockers: (1) f32-only DXIL buffer views (u32 bitmask/prefix words cannot round-trip bit-faithfully); (2) integer bit ops have no DXIL-backend lowering; (3) `shared let`/`barrier()` lower only on NVPTX — the DXIL compute-body lowering rejects `shared let`, so the groupshared scan cannot be expressed. The texture-intrinsic llc blocker does NOT apply to this raw-buffer pass.",
        "Unlike GRX-009..014 there is no native Godot kernel being mirrored: Godot has no GPU compaction pass. The chain implements stable stream compaction against the CPU reference in generate_math_parity_evidence.py, and plugs into Godot's 'draw the first N instances' contract (mesh_storage.h multimesh_get_instances_to_draw L721-728); see known_gaps.",
        "GRX-015 dependency: visibility_mask is the gpu_culling output (u32[ceil(N/32)]); the interface is declared in PASS_CONTRACT.md sec 5.3 and must be re-verified by the later S4 gate once GRX-015 lands.",
    ]
    return finish(evidence, "success", None, None, 0)


if __name__ == "__main__":
    sys.exit(main())
