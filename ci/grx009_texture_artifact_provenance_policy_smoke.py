#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import pathlib
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "luminance_reduction"
PASS_DIR = pathlib.Path(os.environ.get("RURIX_GRX009_PASS_DIR", DEFAULT_PASS_DIR)).expanduser()
POLICY_EVIDENCE_PATH = PASS_DIR / "texture_artifact_provenance_policy.json"
POLICY_DOC_PATH = PASS_DIR / "texture_artifact_provenance_policy.md"
BRIDGE_DOC_PATH = PASS_DIR / "dxc_texture_artifact_bridge.md"
CROSSCHECK_EVIDENCE_PATH = PASS_DIR / "dxc_texture_descriptor_rts0_crosscheck_evidence.json"
MANIFEST_PATH = PASS_DIR / "pass_manifest.json"
REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE = PASS_DIR / "real_pass_enablement_success_evidence.json"
NEXT_ACTION = "provide_grx009_runtime_mappable_luminance_kernel_artifact"
EXPECTED_OWNER_DECISION = (
    "approve_hlsl_bridge_workaround_as_temporary_runtime_mappable_canonical"
)
EXPECTED_CANONICAL_SWITCH_EXCEPTION = "owner_approved_hlsl_bridge_workaround"
REQUIRED_POLICY_DOC_SECTIONS = (
    "## Owner Decision",
    "## Exception to Canonical Switch Conditions",
    "## Revert / Re-cut Conditions",
    "## Fail-Closed Invariants",
)


def rel(path: pathlib.Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def load_json(path: pathlib.Path) -> dict[str, object] | None:
    if not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: pathlib.Path, value: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(value, indent=2, ensure_ascii=True) + "\n")


def normalize_string(value: object) -> str:
    return value.strip() if isinstance(value, str) else ""


def repo_path(path_text: str) -> pathlib.Path:
    candidate = pathlib.Path(path_text)
    if candidate.is_absolute():
        return candidate
    return ROOT / candidate


def real_pass_measured_success_active() -> bool:
    """Stage A5 fail-closed switch: the segment 4h strict measured success
    artifact may exist only when it passes the probe's full strict audit.
    A placeholder or tampered success document never relaxes this smoke."""
    if not REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE.exists():
        return False
    if str(ROOT) not in sys.path:
        sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    return probe.grx009_real_pass_measured_success_active()


def policy_issue(policy: dict[str, object] | None, crosscheck: dict[str, object] | None) -> str | None:
    if not isinstance(crosscheck, dict):
        return "descriptor_rts0_crosscheck_evidence_missing"
    if normalize_string(crosscheck.get("cross_check_status")) != "success":
        return "descriptor_rts0_crosscheck_status_must_be_success"
    if crosscheck.get("descriptor_rts0_crosscheck_ready") is not True:
        return "descriptor_rts0_crosscheck_ready_must_be_true"
    if not isinstance(policy, dict):
        return "texture_artifact_provenance_policy_evidence_missing"
    if normalize_string(policy.get("status")) != "success":
        return "provenance_policy_status_must_be_success"
    if policy.get("policy_ready") is not True:
        return "provenance_policy_ready_must_be_true"
    if normalize_string(policy.get("segment")) != "4l_texture_artifact_provenance_policy":
        return "provenance_policy_segment_mismatch"
    if policy.get("runtime_mappable") is not False:
        return "provenance_policy_runtime_mappable_must_be_false"
    if policy.get("real_gpu_pass") is not False:
        return "provenance_policy_real_gpu_pass_must_be_false"
    if policy.get("canonical_artifact_replaced") is not False:
        return "provenance_policy_canonical_artifact_replaced_must_be_false"
    if policy.get("offline_compile_status_changed") is not False:
        return "provenance_policy_offline_compile_status_changed_must_be_false"
    owner_decision = policy.get("owner_decision")
    if not isinstance(owner_decision, dict):
        return "provenance_policy_owner_decision_missing"
    if normalize_string(owner_decision.get("decision")) != EXPECTED_OWNER_DECISION:
        return "provenance_policy_owner_decision_mismatch"
    if not normalize_string(owner_decision.get("approved_by")):
        return "provenance_policy_owner_approved_by_missing"
    provenance_policy = policy.get("provenance_policy")
    if not isinstance(provenance_policy, dict):
        return "provenance_policy_block_missing"
    if normalize_string(provenance_policy.get("provenance")) != "hlsl_bridge_workaround":
        return "provenance_policy_provenance_must_be_hlsl_bridge_workaround"
    if provenance_policy.get("rurix_owned") is not False:
        return "provenance_policy_rurix_owned_must_be_false"
    if provenance_policy.get("rurix_owned_rts0") is not True:
        return "provenance_policy_rurix_owned_rts0_must_be_true"
    if normalize_string(provenance_policy.get("canonical_switch_exception")) != (
        EXPECTED_CANONICAL_SWITCH_EXCEPTION
    ):
        return "provenance_policy_canonical_switch_exception_mismatch"
    revert_conditions = provenance_policy.get("revert_to_rurix_owned_when")
    if not isinstance(revert_conditions, list) or not revert_conditions:
        return "provenance_policy_revert_conditions_missing"
    if repo_path(normalize_string(policy.get("policy_document"))) != POLICY_DOC_PATH:
        return "provenance_policy_document_path_mismatch"
    if not POLICY_DOC_PATH.is_file():
        return "provenance_policy_document_missing"
    policy_doc_text = POLICY_DOC_PATH.read_text(encoding="utf-8")
    for section in REQUIRED_POLICY_DOC_SECTIONS:
        if section not in policy_doc_text:
            return "provenance_policy_document_required_sections_missing"
    if repo_path(normalize_string(policy.get("bridge_contract_document"))) != BRIDGE_DOC_PATH:
        return "provenance_policy_bridge_contract_document_mismatch"
    if not BRIDGE_DOC_PATH.is_file():
        return "bridge_contract_document_missing"
    if "texture_artifact_provenance_policy.md" not in BRIDGE_DOC_PATH.read_text(encoding="utf-8"):
        return "bridge_contract_document_missing_owner_exception_reference"
    if normalize_string(policy.get("next_action_if_ready")) != NEXT_ACTION:
        return "provenance_policy_next_action_mismatch"
    if REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE.exists() and not real_pass_measured_success_active():
        return "real_pass_enablement_success_evidence_must_not_exist"
    return None


def sync_manifest(status: str, policy_ready: bool) -> None:
    manifest = load_json(MANIFEST_PATH)
    if not isinstance(manifest, dict):
        return
    implementation = manifest.get("implementation_status")
    if isinstance(implementation, dict):
        # Stage A5: when the audited 4h strict measured success is active the
        # manifest legitimately records the opt-in-measured runtime state and
        # real_gpu_pass=true, so this smoke must not clobber them; without
        # that audited success the old fail-closed values are enforced.
        if not real_pass_measured_success_active():
            implementation["runtime_state"] = "fallback_only"
            implementation["real_gpu_pass"] = False
        implementation["segment_4l_texture_artifact_provenance_policy"] = {
            "status": status,
            "policy_ready": policy_ready,
            "evidence": rel(POLICY_EVIDENCE_PATH),
            "policy_document": rel(POLICY_DOC_PATH),
            "bridge_contract_document": rel(BRIDGE_DOC_PATH),
            "owner_decision": EXPECTED_OWNER_DECISION,
            "canonical_switch_exception": EXPECTED_CANONICAL_SWITCH_EXCEPTION,
            "runtime_mappable": False,
            "real_gpu_pass": False,
            "canonical_artifact_replaced": False,
            "offline_compile_status_changed": False,
            "provenance": "hlsl_bridge_workaround",
            "rurix_owned": False,
            "next_action_when_ready": NEXT_ACTION,
        }
    write_json(MANIFEST_PATH, manifest)


def main() -> int:
    policy = load_json(POLICY_EVIDENCE_PATH)
    crosscheck = load_json(CROSSCHECK_EVIDENCE_PATH)
    issue = policy_issue(policy, crosscheck)
    if issue is not None:
        sync_manifest(issue, False)
        print(f"[grx009-texture-artifact-provenance-policy] status=fail issue={issue} evidence={POLICY_EVIDENCE_PATH}")
        return 1
    sync_manifest("success", True)
    print(
        "[grx009-texture-artifact-provenance-policy] "
        "status=success policy_ready=true "
        "canonical_switch_exception=owner_approved_hlsl_bridge_workaround "
        "provenance=hlsl_bridge_workaround rurix_owned=false runtime_mappable=false "
        f"next_action={NEXT_ACTION} evidence={POLICY_EVIDENCE_PATH}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
