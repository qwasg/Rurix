#!/usr/bin/env python3

from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import pathlib
import shutil
import subprocess


ROOT = pathlib.Path(__file__).resolve().parents[1]
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "luminance_reduction"
EVIDENCE_PATH = PASS_DIR / "texture_dxc_feasibility_evidence.json"
ARTIFACT_DIR = PASS_DIR / "artifacts" / "toolchain_probe" / "dxc_texture"
HLSL_PATH = ARTIFACT_DIR / "texture_feasibility.hlsl"
DXIL_PATH = ARTIFACT_DIR / "texture_feasibility.dxil"
DXC_STDOUT_PATH = ARTIFACT_DIR / "dxc_stdout.txt"
DXC_STDERR_PATH = ARTIFACT_DIR / "dxc_stderr.txt"
DXV_STDOUT_PATH = ARTIFACT_DIR / "dxv_stdout.txt"
DXV_STDERR_PATH = ARTIFACT_DIR / "dxv_stderr.txt"

HLSL_SOURCE = """Texture2D<float> src_luminance : register(t0, space0);
RWTexture2D<float> dst_luminance : register(u0, space0);

[numthreads(8, 8, 1)]
void main(uint3 dispatch_id : SV_DispatchThreadID)
{
    float value = src_luminance.Load(int3(dispatch_id.xy, 0));
    dst_luminance[dispatch_id.xy] = value;
}
"""


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
    path.write_text(text, encoding="utf-8")


def find_tool(name: str) -> pathlib.Path | None:
    for env_key in ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"):
        value = os.environ.get(env_key)
        if not value:
            continue
        candidate = pathlib.Path(value).expanduser() / name
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


def base_evidence(dxc_path: pathlib.Path | None, dxv_path: pathlib.Path | None) -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "segment": "4k_texture_dxc_feasibility",
        "status": "skip",
        "ready": False,
        "issue": None,
        "generated_at_utc": utc_now(),
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
        "commands": [],
        "hlsl": {
            "path": rel(HLSL_PATH),
            "entry_point": "main",
            "target_profile": "cs_6_0",
            "sha256": sha256_of_file(HLSL_PATH),
        },
        "dxil_container": {
            "path": rel(DXIL_PATH),
            "exists": DXIL_PATH.is_file(),
            "size_bytes": DXIL_PATH.stat().st_size if DXIL_PATH.is_file() else None,
            "sha256": sha256_of_file(DXIL_PATH),
            "artifact_kind": "dxil_container",
            "produced_by_current_run": False,
        },
        "validation": {
            "tool": "dxv.exe",
            "status": "not_run",
            "skip_reason": None,
        },
        "descriptor_binding_expectation": {
            "resources": [
                {
                    "name": "src_luminance",
                    "class": "t",
                    "register": 0,
                    "space": 0,
                    "hlsl_type": "Texture2D<float>",
                    "binding_kind": "texture2d",
                },
                {
                    "name": "dst_luminance",
                    "class": "u",
                    "register": 0,
                    "space": 0,
                    "hlsl_type": "RWTexture2D<float>",
                    "binding_kind": "rwtexture2d",
                },
            ]
        },
        "root_signature_expectation": {
            "rurix_owned_rts0_available": False,
            "issue": "dxc container does not provide a Rurix-owned .rts0.bin or descriptor layout contract without root signature extraction or synthesis",
        },
        "rurix_artifact_contract_comparison": {
            "satisfies_current_bridge_descriptor_layout_contract": False,
            "current_tracked_binding_kind": "raw_buffer_view",
            "missing_work": [
                "root_signature_extraction",
                "descriptor_layout_synthesis",
                "binding_kind_mapping",
                "DXIL_validation_integration",
                "Rurix_source_provenance",
            ],
        },
        "next_action_if_ready": "design_grx009_dxc_texture_artifact_bridge",
        "next_action_if_not_ready": "provide_grx009_runtime_mappable_luminance_kernel_artifact",
        "does_not_imply": [
            "Godot runtime luminance pass completion",
            "real_gpu_pass=true",
            "real_d3d12_dispatch_recorded=true",
            "visual success",
            "GPU timestamp success",
            "performance claim",
            "canonical luminance artifact replacement",
        ],
    }


def update_dxil(evidence: dict[str, object], produced: bool) -> None:
    evidence["dxil_container"] = {
        "path": rel(DXIL_PATH),
        "exists": DXIL_PATH.is_file(),
        "size_bytes": DXIL_PATH.stat().st_size if DXIL_PATH.is_file() else None,
        "sha256": sha256_of_file(DXIL_PATH),
        "artifact_kind": "dxil_container",
        "produced_by_current_run": produced,
    }


def main() -> int:
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    write_text(HLSL_PATH, HLSL_SOURCE)
    dxc_path = find_tool("dxc.exe")
    dxv_path = find_tool("dxv.exe")
    evidence = base_evidence(dxc_path, dxv_path)
    evidence["hlsl"] = {
        "path": rel(HLSL_PATH),
        "entry_point": "main",
        "target_profile": "cs_6_0",
        "sha256": sha256_of_file(HLSL_PATH),
    }
    if dxc_path is None:
        evidence["status"] = "skip"
        evidence["issue"] = "dxc_missing"
        evidence["skip_reason"] = "dxc_missing"
        write_text(EVIDENCE_PATH, json.dumps(evidence, indent=2, ensure_ascii=True) + "\n")
        print(f"[grx009-texture-dxc] status=skip issue=dxc_missing evidence={EVIDENCE_PATH}")
        return 0

    dxc_argv = [
        str(dxc_path),
        "-T",
        "cs_6_0",
        "-E",
        "main",
        "-Qstrip_debug",
        "-Fo",
        str(DXIL_PATH),
        str(HLSL_PATH),
    ]
    dxc_completed = run_command(dxc_argv, DXC_STDOUT_PATH, DXC_STDERR_PATH)
    evidence["commands"].append(
        {
            "label": "dxc_texture_compute_compile",
            "argv": dxc_argv,
            "exit_code": dxc_completed.returncode,
            "stdout_path": rel(DXC_STDOUT_PATH),
            "stderr_path": rel(DXC_STDERR_PATH),
        }
    )
    if dxc_completed.returncode != 0 or not DXIL_PATH.is_file():
        evidence["status"] = "fail"
        evidence["issue"] = "dxc_compile_failed"
        update_dxil(evidence, False)
        write_text(EVIDENCE_PATH, json.dumps(evidence, indent=2, ensure_ascii=True) + "\n")
        print(f"[grx009-texture-dxc] status=fail issue=dxc_compile_failed evidence={EVIDENCE_PATH}")
        return 0

    update_dxil(evidence, True)
    if dxv_path is None:
        evidence["status"] = "skip"
        evidence["issue"] = "dxv_missing"
        evidence["skip_reason"] = "dxv_missing"
        evidence["validation"] = {
            "tool": "dxv.exe",
            "status": "skip",
            "skip_reason": "dxv_missing",
        }
        write_text(EVIDENCE_PATH, json.dumps(evidence, indent=2, ensure_ascii=True) + "\n")
        print(f"[grx009-texture-dxc] status=skip issue=dxv_missing evidence={EVIDENCE_PATH}")
        return 0

    dxv_argv = [str(dxv_path), str(DXIL_PATH)]
    dxv_completed = run_command(dxv_argv, DXV_STDOUT_PATH, DXV_STDERR_PATH)
    evidence["commands"].append(
        {
            "label": "dxv_texture_container_validation",
            "argv": dxv_argv,
            "exit_code": dxv_completed.returncode,
            "stdout_path": rel(DXV_STDOUT_PATH),
            "stderr_path": rel(DXV_STDERR_PATH),
        }
    )
    evidence["validation"] = {
        "tool": "dxv.exe",
        "status": "pass" if dxv_completed.returncode == 0 else "fail",
        "skip_reason": None,
        "exit_code": dxv_completed.returncode,
        "stdout_path": rel(DXV_STDOUT_PATH),
        "stderr_path": rel(DXV_STDERR_PATH),
    }
    if dxv_completed.returncode == 0:
        evidence["status"] = "success"
        evidence["ready"] = True
        evidence["issue"] = None
    else:
        evidence["status"] = "fail"
        evidence["ready"] = False
        evidence["issue"] = "dxv_validation_failed"
    write_text(EVIDENCE_PATH, json.dumps(evidence, indent=2, ensure_ascii=True) + "\n")
    print(f"[grx009-texture-dxc] status={evidence['status']} issue={evidence['issue']} evidence={EVIDENCE_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
