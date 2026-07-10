#!/usr/bin/env python3
"""GRX-009 stage A2: compile the math-equivalent HLSL texture luminance kernel.

Compiles ``artifacts/hlsl_bridge/luminance_reduce_level.hlsl`` via DXC in two
compile-time variants (mirroring Godot's WRITE_LUMINANCE shader version in
``luminance_reduce.glsl``):

- base (no defines): level-N 8x8 tile reduction (partial-tile-correct mean)
  -> ``artifacts/hlsl_bridge/luminance_reduce_level.dxil``
- ``-D RX_WRITE_LUMINANCE=1``: final level clamp + EMA with prev luminance
  (t1) -> ``artifacts/hlsl_bridge/luminance_reduce_level_write_luminance.dxil``

Each variant is validated with DXV, gets a descriptor layout JSON
(texture2d/rwtexture2d bindings + the canonical 28-byte / 7-dword
root-constant layout because the kernel declares a cbuffer at b0), and a
Rurix-owned RTS0 root signature emitted through
``cargo run --example emit_grx009_texture_rts0`` (which reads the descriptor
and prepends the 28-byte root-constants parameter at root_parameter_index 0).

Tool discovery follows ci/grx009_texture_dxc_feasibility_smoke.py
(``RURIX_DXC_DIR`` / ``RURIX_DXC_NEW_DIR`` env, then the default round-7
extraction dir, then PATH).

Fail-closed: this is an ``hlsl_bridge_workaround`` artifact set, NOT
rurix_owned. It never replaces the canonical artifacts, never advances
``runtime_state``/``real_gpu_pass``, and math parity stays pending until
``math_parity_evidence.json`` records a measured GPU comparison
(see generate_math_parity_evidence.py).
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
BRIDGE_DIR = PASS_DIR / "artifacts" / "hlsl_bridge"
HLSL_PATH = BRIDGE_DIR / "luminance_reduce_level.hlsl"
EVIDENCE_PATH = PASS_DIR / "hlsl_bridge_compile_evidence.json"
MATH_PARITY_EVIDENCE_PATH = PASS_DIR / "math_parity_evidence.json"
DEFAULT_DXC_DIR = pathlib.Path(r"H:\dxc-round7\extracted\bin\x64")

TARGET_PROFILE = "cs_6_0"
ENTRY_POINT = "main"

# Canonical GRX-009 luminance root-constant layout (7 dwords = 28 bytes at
# root_parameter_index 0), matching
# artifacts/luminance_reduction_descriptor_layout.json. The i64 dims are
# carried as uint2 (low, high dword) in the HLSL cbuffer; the runtime must
# write 0 to the high dwords.
ROOT_CONSTANT_LAYOUT = [
    {"name": "source_width", "type": "i64", "order": 0, "root_parameter_index": 0, "dword_offset": 0, "dword_size": 2},
    {"name": "source_height", "type": "i64", "order": 1, "root_parameter_index": 0, "dword_offset": 2, "dword_size": 2},
    {"name": "max_luminance", "type": "f32", "order": 2, "root_parameter_index": 0, "dword_offset": 4, "dword_size": 1},
    {"name": "min_luminance", "type": "f32", "order": 3, "root_parameter_index": 0, "dword_offset": 5, "dword_size": 1},
    {"name": "exposure_adjust", "type": "f32", "order": 4, "root_parameter_index": 0, "dword_offset": 6, "dword_size": 1},
]

SRC_RESOURCE = {
    "name": "src_luminance",
    "binding": "t0 space0",
    "class": "SRV",
    "register": 0,
    "space": 0,
    "count": 1,
    "hlsl_type": "Texture2D<float>",
    "binding_kind": "texture2d",
}
PREV_RESOURCE = {
    "name": "prev_luminance",
    "binding": "t1 space0",
    "class": "SRV",
    "register": 1,
    "space": 0,
    "count": 1,
    "hlsl_type": "Texture2D<float>",
    "binding_kind": "texture2d",
}
DST_RESOURCE = {
    "name": "dst_luminance",
    "binding": "u0 space0",
    "class": "UAV",
    "register": 0,
    "space": 0,
    "count": 1,
    "hlsl_type": "RWTexture2D<float>",
    "binding_kind": "rwtexture2d",
}

VARIANTS = [
    {
        "variant": "base",
        "defines": [],
        "dxil": BRIDGE_DIR / "luminance_reduce_level.dxil",
        "descriptor_layout": BRIDGE_DIR / "descriptor_layout.json",
        "root_signature": BRIDGE_DIR / "root_signature.rts0.bin",
        "resources": [SRC_RESOURCE, DST_RESOURCE],
        "role": "level_reduction (8x8 tile partial-tile-correct mean; no clamp/exposure)",
    },
    {
        "variant": "write_luminance",
        "defines": ["RX_WRITE_LUMINANCE=1"],
        "dxil": BRIDGE_DIR / "luminance_reduce_level_write_luminance.dxil",
        "descriptor_layout": BRIDGE_DIR / "descriptor_layout_write_luminance.json",
        "root_signature": BRIDGE_DIR / "root_signature_write_luminance.rts0.bin",
        "resources": [SRC_RESOURCE, PREV_RESOURCE, DST_RESOURCE],
        "role": "final_level (clamp(min,max) + EMA prev + (cur-prev)*exposure_adjust)",
    },
]

DOES_NOT_IMPLY = [
    "Godot runtime luminance pass completion",
    "real_gpu_pass=true",
    "real_d3d12_dispatch_recorded=true",
    "visual success",
    "GPU timestamp success",
    "performance claim",
    "canonical luminance artifact replacement",
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


def descriptor_layout_doc(variant: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "artifact_kind": "hlsl_bridge_descriptor_layout",
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "canonical_artifact_eligible": False,
        "variant": variant["variant"],
        "defines": list(variant["defines"]),
        "entry_point": ENTRY_POINT,
        "target_profile": TARGET_PROFILE,
        # The kernel declares a cbuffer at b0, so the descriptor carries the
        # canonical 28-byte / 7-dword root-constant layout (stage A2 item 4).
        "root_constants": "28_bytes",
        "root_constant_bytes": 28,
        "root_constant_dwords": 7,
        "root_constant_layout": [dict(entry) for entry in ROOT_CONSTANT_LAYOUT],
        "i64_dims_note": (
            "source_width/source_height are carried as uint2 (low, high dword) in the HLSL "
            "cbuffer to keep plain cs_6_0 without the optional Int64 capability; only the low "
            "dword is consumed and the runtime must write 0 to the high dwords."
        ),
        "resources": [dict(resource) for resource in variant["resources"]],
        # RootConstants parameter at index 0 + single SRV/UAV descriptor table.
        "root_signature_parameters": 2,
    }


def evidence_skeleton(dxc_path: pathlib.Path | None, dxv_path: pathlib.Path | None) -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "segment": "stage_a2_hlsl_bridge_math_equivalent_kernel",
        "status": "skip",
        "ready": False,
        "issue": None,
        "generated_at_utc": utc_now(),
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "runtime_mappable": False,
        "real_gpu_pass": False,
        "canonical_artifact_replaced": False,
        "offline_compile_status_changed": False,
        "math_parity_status": "pending_gpu_dispatch",
        "math_parity_evidence": rel(MATH_PARITY_EVIDENCE_PATH),
        "math_target": {
            "level_reduction": rel(PASS_DIR / "src" / "lib_texture.rx"),
            "final_level": rel(PASS_DIR / "src" / "lib.rx"),
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
                "version_output": command_output(dxv_path, ["--help"]),
            },
        },
        "hlsl": {
            "path": rel(HLSL_PATH),
            "entry_point": ENTRY_POINT,
            "target_profile": TARGET_PROFILE,
            "sha256": sha256_of_file(HLSL_PATH),
        },
        "commands": [],
        "variants": [],
        "does_not_imply": DOES_NOT_IMPLY,
    }


def finish(evidence: dict[str, object], status: str, issue: str | None, exit_code: int) -> int:
    evidence["status"] = status
    evidence["issue"] = issue
    evidence["ready"] = status == "success"
    write_json(EVIDENCE_PATH, evidence)
    print(f"[grx009-hlsl-bridge] status={status} issue={issue} evidence={EVIDENCE_PATH}")
    return exit_code


def main() -> int:
    if not HLSL_PATH.is_file():
        raise SystemExit(f"missing HLSL kernel source: {HLSL_PATH}")
    BRIDGE_DIR.mkdir(parents=True, exist_ok=True)

    dxc_path = find_tool("dxc.exe")
    dxv_path = find_tool("dxv.exe")
    evidence = evidence_skeleton(dxc_path, dxv_path)
    if dxc_path is None:
        return finish(evidence, "skip", "dxc_missing", 0)

    overall_issue: str | None = None
    for variant in VARIANTS:
        name = str(variant["variant"])
        dxil_path: pathlib.Path = variant["dxil"]
        descriptor_path: pathlib.Path = variant["descriptor_layout"]
        rts0_path: pathlib.Path = variant["root_signature"]
        variant_report: dict[str, object] = {
            "variant": name,
            "defines": list(variant["defines"]),
            "role": variant["role"],
            "status": "fail",
        }
        evidence["variants"].append(variant_report)

        # 1) DXC compile.
        dxc_argv = [str(dxc_path), "-T", TARGET_PROFILE, "-E", ENTRY_POINT, "-Qstrip_debug"]
        for define in variant["defines"]:
            dxc_argv.extend(["-D", define])
        dxc_argv.extend(["-Fo", str(dxil_path), str(HLSL_PATH)])
        dxc_stdout = BRIDGE_DIR / f"dxc_stdout_{name}.txt"
        dxc_stderr = BRIDGE_DIR / f"dxc_stderr_{name}.txt"
        dxc_completed = run_command(dxc_argv, dxc_stdout, dxc_stderr)
        evidence["commands"].append(
            {
                "label": f"dxc_compile_{name}",
                "argv": dxc_argv,
                "exit_code": dxc_completed.returncode,
                "stdout_path": rel(dxc_stdout),
                "stderr_path": rel(dxc_stderr),
            }
        )
        variant_report["dxil"] = artifact_entry(dxil_path, "dxil_container", dxc_completed.returncode == 0)
        if dxc_completed.returncode != 0 or not dxil_path.is_file():
            variant_report["status"] = "fail"
            variant_report["issue"] = "dxc_compile_failed"
            overall_issue = overall_issue or "dxc_compile_failed"
            continue

        # 2) DXV validation.
        if dxv_path is None:
            variant_report["validation"] = {"tool": "dxv.exe", "status": "skip", "skip_reason": "dxv_missing"}
            variant_report["status"] = "skip"
            variant_report["issue"] = "dxv_missing"
            overall_issue = overall_issue or "dxv_missing"
        else:
            dxv_argv = [str(dxv_path), str(dxil_path)]
            dxv_stdout = BRIDGE_DIR / f"dxv_stdout_{name}.txt"
            dxv_stderr = BRIDGE_DIR / f"dxv_stderr_{name}.txt"
            dxv_completed = run_command(dxv_argv, dxv_stdout, dxv_stderr)
            evidence["commands"].append(
                {
                    "label": f"dxv_validate_{name}",
                    "argv": dxv_argv,
                    "exit_code": dxv_completed.returncode,
                    "stdout_path": rel(dxv_stdout),
                    "stderr_path": rel(dxv_stderr),
                }
            )
            variant_report["validation"] = {
                "tool": "dxv.exe",
                "status": "pass" if dxv_completed.returncode == 0 else "fail",
                "exit_code": dxv_completed.returncode,
                "stdout_path": rel(dxv_stdout),
                "stderr_path": rel(dxv_stderr),
            }
            if dxv_completed.returncode != 0:
                variant_report["status"] = "fail"
                variant_report["issue"] = "dxv_validation_failed"
                overall_issue = overall_issue or "dxv_validation_failed"
                continue

        # 3) Descriptor layout (texture2d/rwtexture2d + 28-byte root constants).
        write_json(descriptor_path, descriptor_layout_doc(variant))
        variant_report["descriptor_layout"] = artifact_entry(
            descriptor_path, "hlsl_bridge_descriptor_layout", True
        )

        # 4) Rurix-owned RTS0 via the binding-layout example.
        cargo_argv = [
            "cargo",
            "run",
            "-q",
            "-p",
            "rurixc",
            "--features",
            "dxil-backend shader-stages",
            "--example",
            "emit_grx009_texture_rts0",
            "--",
            rel(descriptor_path),
            rel(rts0_path),
        ]
        cargo_stdout = BRIDGE_DIR / f"emit_rts0_stdout_{name}.txt"
        cargo_stderr = BRIDGE_DIR / f"emit_rts0_stderr_{name}.txt"
        cargo_completed = run_command(cargo_argv, cargo_stdout, cargo_stderr)
        evidence["commands"].append(
            {
                "label": f"emit_grx009_texture_rts0_{name}",
                "argv": cargo_argv,
                "exit_code": cargo_completed.returncode,
                "stdout_path": rel(cargo_stdout),
                "stderr_path": rel(cargo_stderr),
            }
        )
        variant_report["root_signature"] = artifact_entry(
            rts0_path, "rurix_owned_rts0_root_signature", cargo_completed.returncode == 0
        )
        variant_report["root_signature_generator"] = {
            "kind": "rurixc_binding_layout_example",
            "command": cargo_argv,
            "source_api": "rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}",
            "root_constants": "28_bytes_at_root_parameter_index_0",
        }
        if cargo_completed.returncode != 0 or not rts0_path.is_file():
            variant_report["status"] = "fail"
            variant_report["issue"] = "emit_rts0_failed"
            overall_issue = overall_issue or "emit_rts0_failed"
            continue

        if variant_report["status"] != "skip":
            variant_report["status"] = "success"
            variant_report["issue"] = None

    statuses = {str(v["status"]) for v in evidence["variants"]}
    if statuses == {"success"}:
        return finish(evidence, "success", None, 0)
    if "fail" in statuses:
        return finish(evidence, "fail", overall_issue, 1)
    return finish(evidence, "skip", overall_issue, 0)


if __name__ == "__main__":
    sys.exit(main())
