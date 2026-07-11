#!/usr/bin/env python3

from __future__ import annotations

import datetime as dt
import hashlib
import json
import pathlib
import shutil


ROOT = pathlib.Path(__file__).resolve().parents[1]
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "luminance_reduction"
FEASIBILITY_EVIDENCE_PATH = PASS_DIR / "texture_dxc_feasibility_evidence.json"
SCAFFOLD_EVIDENCE_PATH = PASS_DIR / "dxc_texture_artifact_bridge_scaffold_evidence.json"
ARTIFACT_DIR = PASS_DIR / "artifacts" / "dxc_texture_bridge"
DXIL_PATH = ARTIFACT_DIR / "texture_bridge_scaffold.dxil"
DESCRIPTOR_LAYOUT_PATH = ARTIFACT_DIR / "descriptor_layout.json"
ROOT_SIGNATURE_SCAFFOLD_PATH = ARTIFACT_DIR / "root_signature_scaffold.json"


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


def write_json(path: pathlib.Path, value: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")


def load_json(path: pathlib.Path) -> dict[str, object] | None:
    if not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def evidence_path(path_text: object) -> pathlib.Path | None:
    if not isinstance(path_text, str) or not path_text:
        return None
    candidate = pathlib.Path(path_text)
    if candidate.is_absolute():
        return candidate
    return ROOT / candidate


def command_by_label(feasibility: dict[str, object], label: str) -> dict[str, object]:
    commands = feasibility.get("commands")
    if not isinstance(commands, list):
        return {}
    for command in commands:
        if isinstance(command, dict) and command.get("label") == label:
            return command
    return {}


def command_record(command: dict[str, object]) -> dict[str, object]:
    return {
        "argv": command.get("argv") if isinstance(command.get("argv"), list) else [],
        "exit_code": command.get("exit_code"),
        "stdout_path": command.get("stdout_path"),
        "stderr_path": command.get("stderr_path"),
    }


def base_evidence(issue: str | None = None) -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "segment": "4k_dxc_texture_artifact_bridge_scaffold",
        "status": "fail" if issue else "success",
        "scaffold_ready": issue is None,
        "issue": issue,
        "generated_at_utc": utc_now(),
        "runtime_mappable": False,
        "real_gpu_pass": False,
        "canonical_artifact_replaced": False,
        "offline_compile_status_changed": False,
        "artifact_dir": rel(ARTIFACT_DIR),
        "source_feasibility_evidence": rel(FEASIBILITY_EVIDENCE_PATH),
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "design_or_scaffold_only": True,
        "canonical_artifact_eligible": False,
        "does_not_imply": [
            "offline_compile_success",
            "runtime_mappable=true",
            "real_gpu_pass=true",
            "canonical artifact replacement",
            "visual success",
            "performance claim",
        ],
        "next_action_if_ready": "prepare_grx009_texture_artifact_rurix_provenance_or_rts0_integration",
    }


def descriptor_layout() -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "artifact_kind": "dxc_texture_bridge_descriptor_layout_scaffold",
        "root_constants": "none",
        "canonical_artifact_eligible": False,
        "resources": [
            {
                "name": "src_luminance",
                "binding": "t0 space0",
                "class": "SRV",
                "register": 0,
                "space": 0,
                "count": 1,
                "hlsl_type": "Texture2D<float>",
                "binding_kind": "texture2d",
            },
            {
                "name": "dst_luminance",
                "binding": "u0 space0",
                "class": "UAV",
                "register": 0,
                "space": 0,
                "count": 1,
                "hlsl_type": "RWTexture2D<float>",
                "binding_kind": "rwtexture2d",
            },
        ],
    }


def root_signature_scaffold() -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "artifact_kind": "dxc_texture_bridge_root_signature_scaffold",
        "root_signature_status": "scaffold_only",
        "rurix_owned_rts0_generated": False,
        "rts0_artifact": None,
        "unavailable_reason": "No direct Python or stable CLI entrypoint exists in this slice for Rurix RTS0 synthesis over the HLSL bridge descriptor; the HLSL DXC container is not a Rurix-owned RTS0 artifact.",
        "cross_check_status": "not_available",
        "root_constants": "none",
        "canonical_artifact_eligible": False,
    }


def main() -> int:
    feasibility = load_json(FEASIBILITY_EVIDENCE_PATH)
    if not isinstance(feasibility, dict):
        evidence = base_evidence("texture_dxc_feasibility_evidence_missing")
        write_json(SCAFFOLD_EVIDENCE_PATH, evidence)
        print(f"[grx009-dxc-texture-bridge] status=fail issue={evidence['issue']} evidence={SCAFFOLD_EVIDENCE_PATH}")
        return 1

    dxil = feasibility.get("dxil_container")
    validation = feasibility.get("validation")
    if feasibility.get("status") != "success" or feasibility.get("ready") is not True:
        issue = "texture_dxc_feasibility_not_ready"
    elif not isinstance(dxil, dict):
        issue = "source_dxil_container_missing"
    elif not isinstance(validation, dict) or validation.get("status") != "pass":
        issue = "dxv_validation_metadata_missing"
    else:
        source_dxil = evidence_path(dxil.get("path"))
        recorded_sha = dxil.get("sha256")
        actual_sha = sha256_of_file(source_dxil) if source_dxil else None
        if source_dxil is None or not source_dxil.is_file():
            issue = "source_dxil_container_missing"
        elif not isinstance(recorded_sha, str) or recorded_sha != actual_sha:
            issue = "source_dxil_hash_mismatch"
        else:
            issue = None

    if issue is not None:
        evidence = base_evidence(issue)
        write_json(SCAFFOLD_EVIDENCE_PATH, evidence)
        print(f"[grx009-dxc-texture-bridge] status=fail issue={issue} evidence={SCAFFOLD_EVIDENCE_PATH}")
        return 1

    assert isinstance(dxil, dict)
    assert isinstance(validation, dict)
    source_dxil = evidence_path(dxil.get("path"))
    assert source_dxil is not None

    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source_dxil, DXIL_PATH)
    write_json(DESCRIPTOR_LAYOUT_PATH, descriptor_layout())
    root_signature = root_signature_scaffold()
    write_json(ROOT_SIGNATURE_SCAFFOLD_PATH, root_signature)

    compile_command = command_by_label(feasibility, "dxc_texture_compute_compile")
    validation_command = command_by_label(feasibility, "dxv_texture_container_validation")
    tools = feasibility.get("tools") if isinstance(feasibility.get("tools"), dict) else {}
    dxc_tool = tools.get("dxc") if isinstance(tools, dict) and isinstance(tools.get("dxc"), dict) else {}
    dxv_tool = tools.get("dxv") if isinstance(tools, dict) and isinstance(tools.get("dxv"), dict) else {}
    source = feasibility.get("hlsl") if isinstance(feasibility.get("hlsl"), dict) else {}

    evidence = base_evidence()
    evidence.update(
        {
            "dxil_container_metadata": {
                "dxc": {
                    "found": dxc_tool.get("found"),
                    "path": dxc_tool.get("path"),
                    "version_output": dxc_tool.get("version_output"),
                },
                "dxv": {
                    "found": dxv_tool.get("found"),
                    "path": dxv_tool.get("path"),
                    "version_output": dxv_tool.get("version_output"),
                },
                "compile": command_record(compile_command),
                "validation": {
                    **command_record(validation_command),
                    "status": validation.get("status"),
                },
                "container": {
                    "path": rel(DXIL_PATH),
                    "source_path": rel(source_dxil),
                    "size_bytes": DXIL_PATH.stat().st_size,
                    "sha256": sha256_of_file(DXIL_PATH),
                    "artifact_kind": "dxil_container",
                },
                "target_profile": "cs_6_0",
                "entry_point": "main",
                "source": {
                    "path": source.get("path"),
                    "entry_point": source.get("entry_point"),
                    "target_profile": source.get("target_profile"),
                    "sha256": source.get("sha256"),
                },
            },
            "descriptor_layout_artifact": {
                "path": rel(DESCRIPTOR_LAYOUT_PATH),
                "size_bytes": DESCRIPTOR_LAYOUT_PATH.stat().st_size,
                "sha256": sha256_of_file(DESCRIPTOR_LAYOUT_PATH),
                "root_constants": "none",
                "resources": descriptor_layout()["resources"],
                "canonical_artifact_eligible": False,
            },
            "root_signature_scaffold": {
                "path": rel(ROOT_SIGNATURE_SCAFFOLD_PATH),
                "size_bytes": ROOT_SIGNATURE_SCAFFOLD_PATH.stat().st_size,
                "sha256": sha256_of_file(ROOT_SIGNATURE_SCAFFOLD_PATH),
                **root_signature,
            },
            "binding_kind_mapping": {
                "RXGD_RESOURCE_TEXTURE": {
                    "src_luminance": "texture2d",
                    "dst_luminance": "rwtexture2d",
                    "rule": "by descriptor slot",
                },
                "RXGD_RESOURCE_BUFFER": "raw_buffer_view",
                "canonical_descriptor_binding_kind": "raw_buffer_view",
                "canonical_descriptor_replaced": False,
            },
        }
    )
    write_json(SCAFFOLD_EVIDENCE_PATH, evidence)
    print(f"[grx009-dxc-texture-bridge] status=success scaffold_ready=true evidence={SCAFFOLD_EVIDENCE_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
