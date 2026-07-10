#!/usr/bin/env python3

from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import pathlib
import subprocess


ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "luminance_reduction"
PASS_DIR = pathlib.Path(os.environ.get("RURIX_GRX009_PASS_DIR", DEFAULT_PASS_DIR)).expanduser()
SCAFFOLD_EVIDENCE_PATH = PASS_DIR / "dxc_texture_artifact_bridge_scaffold_evidence.json"
CROSSCHECK_EVIDENCE_PATH = PASS_DIR / "dxc_texture_descriptor_rts0_crosscheck_evidence.json"
MANIFEST_PATH = PASS_DIR / "pass_manifest.json"
ARTIFACT_DIR = PASS_DIR / "artifacts" / "dxc_texture_bridge"
DESCRIPTOR_LAYOUT_PATH = ARTIFACT_DIR / "descriptor_layout.json"
ROOT_SIGNATURE_METADATA_PATH = ARTIFACT_DIR / "root_signature_scaffold.json"
RTS0_PATH = ARTIFACT_DIR / "root_signature.rts0.bin"
WORK_DIR = ROOT / "target" / "grx009_descriptor_rts0_crosscheck"
RESERIALIZED_RTS0_PATH = WORK_DIR / "root_signature.reserialized.rts0.bin"
NEXT_ACTION = "define_grx009_texture_artifact_provenance_policy"


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


def artifact_sha(doc: dict[str, object] | None, *keys: str) -> str | None:
    current: object = doc
    for key in keys:
        if not isinstance(current, dict):
            return None
        current = current.get(key)
    return current if isinstance(current, str) and current else None


def resource_issue(descriptor: dict[str, object] | None) -> str | None:
    if not isinstance(descriptor, dict):
        return "descriptor_layout_missing"
    if descriptor.get("root_constants") != "none":
        return "descriptor_root_constants_must_be_none"
    resources = descriptor.get("resources")
    if not isinstance(resources, list):
        return "descriptor_resources_missing"
    by_name = {resource.get("name"): resource for resource in resources if isinstance(resource, dict)}
    expected = {
        "src_luminance": {"class": "SRV", "register": 0, "space": 0, "count": 1, "binding_kind": "texture2d"},
        "dst_luminance": {"class": "UAV", "register": 0, "space": 0, "count": 1, "binding_kind": "rwtexture2d"},
    }
    for name, fields in expected.items():
        resource = by_name.get(name)
        if not isinstance(resource, dict):
            return f"descriptor_resource_missing:{name}"
        for key, value in fields.items():
            if resource.get(key) != value:
                return f"descriptor_resource_{name}_{key}_mismatch"
    if set(by_name) != set(expected):
        return "descriptor_resources_must_match_expected_set"
    return None


def fail_evidence(issue: str, descriptor_sha256: str | None, rts0_sha256: str | None, reserialized_sha256: str | None) -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "cross_check_status": issue,
        "descriptor_rts0_crosscheck_ready": False,
        "generated_at_utc": utc_now(),
        "descriptor_layout_artifact": artifact_entry(DESCRIPTOR_LAYOUT_PATH, descriptor_sha256, "dxc_texture_bridge_descriptor_layout_scaffold"),
        "rts0_artifact": artifact_entry(RTS0_PATH, rts0_sha256, "rurix_owned_rts0_root_signature"),
        "reserialized_rts0_artifact": artifact_entry(RESERIALIZED_RTS0_PATH, reserialized_sha256, "rurix_owned_rts0_root_signature_reserialized"),
        "byte_for_byte_match": False,
        "root_constants": "none",
        "runtime_mappable": False,
        "real_gpu_pass": False,
        "canonical_artifact_replaced": False,
        "offline_compile_status_changed": False,
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "next_action_if_ready": NEXT_ACTION,
    }


def artifact_entry(path: pathlib.Path, digest: str | None, artifact_kind: str) -> dict[str, object]:
    return {
        "path": rel(path),
        "size_bytes": path.stat().st_size if path.is_file() else None,
        "sha256": digest,
        "artifact_kind": artifact_kind,
    }


def success_evidence(descriptor_sha256: str, rts0_sha256: str, reserialized_sha256: str, command: list[str]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "cross_check_status": "success",
        "descriptor_rts0_crosscheck_ready": True,
        "generated_at_utc": utc_now(),
        "descriptor_layout_artifact": artifact_entry(DESCRIPTOR_LAYOUT_PATH, descriptor_sha256, "dxc_texture_bridge_descriptor_layout_scaffold"),
        "rts0_artifact": artifact_entry(RTS0_PATH, rts0_sha256, "rurix_owned_rts0_root_signature"),
        "reserialized_rts0_artifact": artifact_entry(RESERIALIZED_RTS0_PATH, reserialized_sha256, "rurix_owned_rts0_root_signature_reserialized"),
        "byte_for_byte_match": True,
        "root_constants": "none",
        "generator": {
            "kind": "rurixc_binding_layout_example",
            "command": command,
            "source_api": "rurixc::binding_layout::{infer_root_signature, serialize_rts0}",
        },
        "runtime_mappable": False,
        "real_gpu_pass": False,
        "canonical_artifact_replaced": False,
        "offline_compile_status_changed": False,
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "next_action_if_ready": NEXT_ACTION,
    }


def sync_metadata(evidence: dict[str, object]) -> None:
    root_signature = load_json(ROOT_SIGNATURE_METADATA_PATH)
    if isinstance(root_signature, dict):
        root_signature["cross_check_status"] = evidence.get("cross_check_status")
        root_signature["descriptor_rts0_crosscheck_ready"] = evidence.get("descriptor_rts0_crosscheck_ready") is True
        root_signature["cross_check_evidence"] = rel(CROSSCHECK_EVIDENCE_PATH)
        root_signature["descriptor_sha256"] = artifact_sha(evidence, "descriptor_layout_artifact", "sha256")
        root_signature["rts0_sha256"] = artifact_sha(evidence, "rts0_artifact", "sha256")
        root_signature["reserialized_rts0_sha256"] = artifact_sha(evidence, "reserialized_rts0_artifact", "sha256")
        root_signature["byte_for_byte_match"] = evidence.get("byte_for_byte_match") is True
        root_signature["runtime_mappable"] = False
        root_signature["real_gpu_pass"] = False
        root_signature["canonical_artifact_replaced"] = False
        write_json(ROOT_SIGNATURE_METADATA_PATH, root_signature)
    scaffold = load_json(SCAFFOLD_EVIDENCE_PATH)
    if isinstance(scaffold, dict):
        scaffold["generated_at_utc"] = utc_now()
        scaffold["runtime_mappable"] = False
        scaffold["real_gpu_pass"] = False
        scaffold["canonical_artifact_replaced"] = False
        scaffold["offline_compile_status_changed"] = False
        scaffold["provenance"] = "hlsl_bridge_workaround"
        scaffold["rurix_owned"] = False
        scaffold["next_action_if_ready"] = NEXT_ACTION if evidence.get("descriptor_rts0_crosscheck_ready") is True else "prepare_grx009_texture_artifact_descriptor_rts0_crosscheck_or_provenance_policy"
        nested = scaffold.get("root_signature_scaffold")
        if isinstance(nested, dict):
            nested["sha256"] = sha256_of_file(ROOT_SIGNATURE_METADATA_PATH)
            nested["size_bytes"] = ROOT_SIGNATURE_METADATA_PATH.stat().st_size if ROOT_SIGNATURE_METADATA_PATH.is_file() else None
            nested["cross_check_status"] = evidence.get("cross_check_status")
            nested["descriptor_rts0_crosscheck_ready"] = evidence.get("descriptor_rts0_crosscheck_ready") is True
            nested["cross_check_evidence"] = rel(CROSSCHECK_EVIDENCE_PATH)
            nested["descriptor_sha256"] = artifact_sha(evidence, "descriptor_layout_artifact", "sha256")
            nested["rts0_sha256"] = artifact_sha(evidence, "rts0_artifact", "sha256")
            nested["reserialized_rts0_sha256"] = artifact_sha(evidence, "reserialized_rts0_artifact", "sha256")
            nested["byte_for_byte_match"] = evidence.get("byte_for_byte_match") is True
            nested["runtime_mappable"] = False
            nested["real_gpu_pass"] = False
            nested["canonical_artifact_replaced"] = False
        write_json(SCAFFOLD_EVIDENCE_PATH, scaffold)
    manifest = load_json(MANIFEST_PATH)
    if isinstance(manifest, dict):
        implementation = manifest.get("implementation_status")
        if isinstance(implementation, dict):
            implementation["runtime_state"] = "fallback_only"
            implementation["real_gpu_pass"] = False
            implementation["segment_4k_dxc_texture_descriptor_rts0_crosscheck"] = {
                "status": evidence.get("cross_check_status"),
                "descriptor_rts0_crosscheck_ready": evidence.get("descriptor_rts0_crosscheck_ready") is True,
                "evidence": rel(CROSSCHECK_EVIDENCE_PATH),
                "descriptor_layout": evidence.get("descriptor_layout_artifact"),
                "rts0_artifact": evidence.get("rts0_artifact"),
                "reserialized_rts0_artifact": evidence.get("reserialized_rts0_artifact"),
                "byte_for_byte_match": evidence.get("byte_for_byte_match") is True,
                "runtime_mappable": False,
                "real_gpu_pass": False,
                "canonical_artifact_replaced": False,
                "offline_compile_status_changed": False,
                "provenance": "hlsl_bridge_workaround",
                "rurix_owned": False,
                "next_action_when_ready": NEXT_ACTION,
            }
        write_json(MANIFEST_PATH, manifest)


def write_fail(issue: str) -> int:
    evidence = fail_evidence(issue, sha256_of_file(DESCRIPTOR_LAYOUT_PATH), sha256_of_file(RTS0_PATH), sha256_of_file(RESERIALIZED_RTS0_PATH))
    write_json(CROSSCHECK_EVIDENCE_PATH, evidence)
    sync_metadata(evidence)
    print(f"[grx009-descriptor-rts0-crosscheck] status=fail issue={issue} evidence={CROSSCHECK_EVIDENCE_PATH}")
    return 1


def main() -> int:
    descriptor = load_json(DESCRIPTOR_LAYOUT_PATH)
    root_signature = load_json(ROOT_SIGNATURE_METADATA_PATH)
    scaffold = load_json(SCAFFOLD_EVIDENCE_PATH)
    descriptor_sha256 = sha256_of_file(DESCRIPTOR_LAYOUT_PATH)
    rts0_sha256 = sha256_of_file(RTS0_PATH)
    if descriptor_sha256 is None:
        return write_fail("descriptor_layout_missing")
    if rts0_sha256 is None:
        return write_fail("rts0_artifact_missing")
    issue = resource_issue(descriptor)
    if issue is not None:
        return write_fail(issue)
    if descriptor_sha256 != artifact_sha(root_signature, "descriptor_layout_artifact", "sha256"):
        return write_fail("descriptor_hash_mismatch_root_signature_metadata")
    if descriptor_sha256 != artifact_sha(scaffold, "descriptor_layout_artifact", "sha256"):
        return write_fail("descriptor_hash_mismatch_scaffold_evidence")
    if rts0_sha256 != artifact_sha(root_signature, "rts0_artifact", "sha256"):
        return write_fail("rts0_hash_mismatch_root_signature_metadata")
    if rts0_sha256 != artifact_sha(scaffold, "root_signature_scaffold", "rts0_artifact", "sha256"):
        return write_fail("rts0_hash_mismatch_scaffold_evidence")
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    command = [
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
        str(DESCRIPTOR_LAYOUT_PATH),
        str(RESERIALIZED_RTS0_PATH),
    ]
    completed = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    if completed.returncode != 0 or not RESERIALIZED_RTS0_PATH.is_file():
        evidence = fail_evidence("reserialize_rts0_failed", descriptor_sha256, rts0_sha256, sha256_of_file(RESERIALIZED_RTS0_PATH))
        evidence["generator_output"] = completed.stdout[-4000:]
        write_json(CROSSCHECK_EVIDENCE_PATH, evidence)
        sync_metadata(evidence)
        print("[grx009-descriptor-rts0-crosscheck] status=fail issue=reserialize_rts0_failed")
        print(completed.stdout)
        return 1
    reserialized_sha256 = sha256_of_file(RESERIALIZED_RTS0_PATH)
    if reserialized_sha256 != rts0_sha256 or RESERIALIZED_RTS0_PATH.read_bytes() != RTS0_PATH.read_bytes():
        return write_fail("reserialized_rts0_bytes_mismatch")
    evidence = success_evidence(descriptor_sha256, rts0_sha256, reserialized_sha256, command)
    write_json(CROSSCHECK_EVIDENCE_PATH, evidence)
    sync_metadata(evidence)
    print(
        "[grx009-descriptor-rts0-crosscheck] "
        f"status=success ready=true descriptor_sha256={descriptor_sha256} "
        f"rts0_sha256={rts0_sha256} reserialized_rts0_sha256={reserialized_sha256} "
        f"byte_for_byte_match=true evidence={CROSSCHECK_EVIDENCE_PATH}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
