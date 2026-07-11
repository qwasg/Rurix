#!/usr/bin/env python3

from __future__ import annotations

import datetime as dt
import hashlib
import json
import pathlib
import subprocess


ROOT = pathlib.Path(__file__).resolve().parents[1]
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "luminance_reduction"
SCAFFOLD_EVIDENCE_PATH = PASS_DIR / "dxc_texture_artifact_bridge_scaffold_evidence.json"
MANIFEST_PATH = PASS_DIR / "pass_manifest.json"
ARTIFACT_DIR = PASS_DIR / "artifacts" / "dxc_texture_bridge"
DESCRIPTOR_LAYOUT_PATH = ARTIFACT_DIR / "descriptor_layout.json"
ROOT_SIGNATURE_METADATA_PATH = ARTIFACT_DIR / "root_signature_scaffold.json"
RTS0_PATH = ARTIFACT_DIR / "root_signature.rts0.bin"
NEXT_ACTION = "prepare_grx009_texture_artifact_descriptor_rts0_crosscheck_or_provenance_policy"


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


def load_json(path: pathlib.Path) -> dict[str, object] | None:
    if not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: pathlib.Path, value: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(value, indent=2, ensure_ascii=True) + "\n")


def descriptor_issue(descriptor: dict[str, object] | None) -> str | None:
    if not isinstance(descriptor, dict):
        return "descriptor_layout_missing"
    resources = descriptor.get("resources")
    if not isinstance(resources, list):
        return "descriptor_resources_missing"
    by_name = {r.get("name"): r for r in resources if isinstance(r, dict)}
    src = by_name.get("src_luminance")
    dst = by_name.get("dst_luminance")
    if not isinstance(src, dict) or src.get("binding_kind") != "texture2d":
        return "descriptor_texture2d_binding_kind_missing"
    if not isinstance(dst, dict) or dst.get("binding_kind") != "rwtexture2d":
        return "descriptor_rwtexture2d_binding_kind_missing"
    return None


def blocked_root_signature(issue: str, descriptor_sha256: str | None) -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "artifact_kind": "dxc_texture_bridge_root_signature_scaffold",
        "root_signature_status": "blocked",
        "rts0_integration_status": issue,
        "rts0_integration_ready": False,
        "rurix_owned_rts0_generated": False,
        "rts0_artifact": None,
        "descriptor_layout_artifact": {
            "path": rel(DESCRIPTOR_LAYOUT_PATH),
            "sha256": descriptor_sha256,
            "size_bytes": DESCRIPTOR_LAYOUT_PATH.stat().st_size if DESCRIPTOR_LAYOUT_PATH.is_file() else None,
        },
        "cross_check_status": "not_available",
        "root_constants": "none",
        "canonical_artifact_eligible": False,
    }


def synthesized_root_signature(descriptor_sha256: str, command: list[str]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "artifact_kind": "dxc_texture_bridge_root_signature_scaffold",
        "root_signature_status": "rurix_synthesized",
        "rts0_integration_status": "success",
        "rts0_integration_ready": True,
        "rurix_owned_rts0_generated": True,
        "rts0_artifact": {
            "path": rel(RTS0_PATH),
            "size_bytes": RTS0_PATH.stat().st_size,
            "sha256": sha256_of_file(RTS0_PATH),
            "artifact_kind": "rurix_owned_rts0_root_signature",
        },
        "descriptor_layout_artifact": {
            "path": rel(DESCRIPTOR_LAYOUT_PATH),
            "size_bytes": DESCRIPTOR_LAYOUT_PATH.stat().st_size,
            "sha256": descriptor_sha256,
            "artifact_kind": "dxc_texture_bridge_descriptor_layout_scaffold",
        },
        "generator": {
            "kind": "rurixc_binding_layout_example",
            "command": command,
            "source_api": "rurixc::binding_layout::{infer_root_signature, serialize_rts0}",
        },
        "cross_check_status": "pending_descriptor_rts0_crosscheck",
        "root_constants": "none",
        "canonical_artifact_eligible": False,
    }


def update_scaffold_evidence(root_signature: dict[str, object], descriptor: dict[str, object]) -> None:
    evidence = load_json(SCAFFOLD_EVIDENCE_PATH)
    if not isinstance(evidence, dict):
        return
    evidence["generated_at_utc"] = utc_now()
    evidence["next_action_if_ready"] = NEXT_ACTION
    evidence["runtime_mappable"] = False
    evidence["real_gpu_pass"] = False
    evidence["canonical_artifact_replaced"] = False
    evidence["offline_compile_status_changed"] = False
    evidence["provenance"] = "hlsl_bridge_workaround"
    evidence["rurix_owned"] = False
    evidence["rts0_integration_ready"] = root_signature.get("rts0_integration_ready") is True
    evidence["rts0_integration_status"] = root_signature.get("rts0_integration_status")
    descriptor_entry = evidence.get("descriptor_layout_artifact")
    if isinstance(descriptor_entry, dict):
        descriptor_entry["path"] = rel(DESCRIPTOR_LAYOUT_PATH)
        descriptor_entry["size_bytes"] = DESCRIPTOR_LAYOUT_PATH.stat().st_size
        descriptor_entry["sha256"] = sha256_of_file(DESCRIPTOR_LAYOUT_PATH)
        descriptor_entry["resources"] = descriptor.get("resources") if isinstance(descriptor.get("resources"), list) else []
        descriptor_entry["canonical_artifact_eligible"] = False
    evidence["root_signature_scaffold"] = {
        "path": rel(ROOT_SIGNATURE_METADATA_PATH),
        "size_bytes": ROOT_SIGNATURE_METADATA_PATH.stat().st_size,
        "sha256": sha256_of_file(ROOT_SIGNATURE_METADATA_PATH),
        **root_signature,
    }
    write_json(SCAFFOLD_EVIDENCE_PATH, evidence)


def update_manifest(root_signature: dict[str, object]) -> None:
    manifest = load_json(MANIFEST_PATH)
    if not isinstance(manifest, dict):
        return
    implementation = manifest.get("implementation_status")
    if not isinstance(implementation, dict):
        return
    implementation["runtime_state"] = "fallback_only"
    implementation["real_gpu_pass"] = False
    implementation["segment_4k_dxc_texture_artifact_rts0_integration"] = {
        "status": root_signature.get("root_signature_status"),
        "rts0_integration_status": root_signature.get("rts0_integration_status"),
        "rts0_integration_ready": root_signature.get("rts0_integration_ready") is True,
        "root_signature_metadata": rel(ROOT_SIGNATURE_METADATA_PATH),
        "descriptor_layout": rel(DESCRIPTOR_LAYOUT_PATH),
        "rts0_artifact": root_signature.get("rts0_artifact"),
        "runtime_mappable": False,
        "real_gpu_pass": False,
        "canonical_artifact_replaced": False,
        "offline_compile_status_changed": False,
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "next_action_when_ready": NEXT_ACTION,
        "does_not_imply": [
            "canonical artifact replacement",
            "offline compile success",
            "runtime_mappable=true",
            "real_gpu_pass=true",
            "visual success",
            "performance claim",
        ],
    }
    scaffold = implementation.get("segment_4k_dxc_texture_artifact_bridge_scaffold")
    if isinstance(scaffold, dict):
        scaffold["next_action_when_ready"] = NEXT_ACTION
    write_json(MANIFEST_PATH, manifest)


def main() -> int:
    descriptor = load_json(DESCRIPTOR_LAYOUT_PATH)
    descriptor_sha256 = sha256_of_file(DESCRIPTOR_LAYOUT_PATH)
    issue = descriptor_issue(descriptor)
    if issue is not None:
        root_signature = blocked_root_signature(issue, descriptor_sha256)
        write_json(ROOT_SIGNATURE_METADATA_PATH, root_signature)
        update_scaffold_evidence(root_signature, descriptor or {})
        update_manifest(root_signature)
        print(f"[grx009-dxc-texture-rts0] status=fail issue={issue} evidence={SCAFFOLD_EVIDENCE_PATH}")
        return 1

    assert isinstance(descriptor, dict)
    command = [
        "cargo",
        "run",
        "-p",
        "rurixc",
        "--features",
        "dxil-backend shader-stages",
        "--example",
        "emit_grx009_texture_rts0",
        "--",
        str(DESCRIPTOR_LAYOUT_PATH),
        str(RTS0_PATH),
    ]
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if completed.returncode != 0 or not RTS0_PATH.is_file():
        root_signature = blocked_root_signature("blocked_missing_stable_cli", descriptor_sha256)
        root_signature["generator_output"] = completed.stdout[-4000:]
        write_json(ROOT_SIGNATURE_METADATA_PATH, root_signature)
        update_scaffold_evidence(root_signature, descriptor)
        update_manifest(root_signature)
        print("[grx009-dxc-texture-rts0] status=fail issue=blocked_missing_stable_cli")
        print(completed.stdout)
        return 1

    actual_descriptor_sha = sha256_of_file(DESCRIPTOR_LAYOUT_PATH)
    if not isinstance(actual_descriptor_sha, str):
        root_signature = blocked_root_signature("descriptor_layout_missing_after_generation", None)
        write_json(ROOT_SIGNATURE_METADATA_PATH, root_signature)
        update_scaffold_evidence(root_signature, descriptor)
        update_manifest(root_signature)
        return 1
    root_signature = synthesized_root_signature(actual_descriptor_sha, command)
    write_json(ROOT_SIGNATURE_METADATA_PATH, root_signature)
    update_scaffold_evidence(root_signature, descriptor)
    update_manifest(root_signature)
    print(
        "[grx009-dxc-texture-rts0] "
        f"status=success rts0_integration_ready=true rts0_sha256={sha256_of_file(RTS0_PATH)} "
        f"descriptor_sha256={actual_descriptor_sha} evidence={SCAFFOLD_EVIDENCE_PATH}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
