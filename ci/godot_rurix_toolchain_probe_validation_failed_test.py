#!/usr/bin/env python3

from __future__ import annotations

import copy
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile


ROOT = pathlib.Path(__file__).resolve().parents[1]
REAL_PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "luminance_reduction"
REAL_CONTRACT_PATH = REAL_PASS_DIR / "PASS_CONTRACT.md"
REAL_MANIFEST_PATH = REAL_PASS_DIR / "pass_manifest.json"
REAL_EVIDENCE_PATH = REAL_PASS_DIR / "offline_compile_evidence.json"
REAL_SCHEMA_PATH = REAL_PASS_DIR / "compile_evidence.schema.json"
REAL_RESOURCE_MAPPING_PATH = REAL_PASS_DIR / "resource_mapping.md"
REAL_DESCRIPTOR_LAYOUT_PATH = (
    REAL_PASS_DIR / "artifacts" / "luminance_reduction_descriptor_layout.json"
)
REAL_RAW_BUFFER_EVIDENCE_PATH = REAL_PASS_DIR / "offline_compile_evidence_raw_buffer.json"
REAL_PASS_ENABLEMENT_EVIDENCE_PATH = REAL_PASS_DIR / "real_pass_enablement_evidence.json"
REAL_TEXTURE_DXC_FEASIBILITY_PATH = REAL_PASS_DIR / "texture_dxc_feasibility_evidence.json"
REAL_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC_PATH = (
    REAL_PASS_DIR / "dxc_texture_artifact_bridge.md"
)
REAL_DXC_TEXTURE_ARTIFACT_BRIDGE_DESIGN_PATH = (
    REAL_PASS_DIR / "dxc_texture_artifact_bridge_design.json"
)
REAL_DXC_TEXTURE_ARTIFACT_BRIDGE_SCAFFOLD_PATH = (
    REAL_PASS_DIR / "dxc_texture_artifact_bridge_scaffold_evidence.json"
)
REAL_DXC_TEXTURE_DESCRIPTOR_RTS0_CROSSCHECK_PATH = (
    REAL_PASS_DIR / "dxc_texture_descriptor_rts0_crosscheck_evidence.json"
)
REAL_TEXTURE_ARTIFACT_PROVENANCE_POLICY_PATH = (
    REAL_PASS_DIR / "texture_artifact_provenance_policy.json"
)
REAL_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC_PATH = (
    REAL_PASS_DIR / "texture_artifact_provenance_policy.md"
)
REAL_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR = REAL_PASS_DIR / "artifacts" / "dxc_texture_bridge"
STALE_SEGMENT4H_DXIL_SHA256 = "b5c5ea9be11be20184695a22b3ef8ac38055d227e4366a4d61db604a10564258"


def load_json(path: pathlib.Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: pathlib.Path, value: dict[str, object]) -> None:
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(value, indent=2) + "\n")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def pre_a5_fixture_manifest(base: dict[str, object]) -> dict[str, object]:
    """Normalize the real (stage A5) manifest back to the pre-A5 fail-closed
    shape for temp fixture pass dirs.

    Stage A5 flipped the REAL manifest to implemented=true,
    runtime_state=fallback_only_by_default_real_pass_optin_measured, and
    real_gpu_pass=true, which the probe only accepts while the strict measured
    success artifact real_pass_enablement_success_evidence.json exists AND
    passes the full 4h audit. Fixture pass dirs never carry that success
    artifact, so the probe's fail-closed manifest checks demand the old
    values there; this derivation restores them without touching the real
    manifest on disk."""
    candidate = copy.deepcopy(base)
    candidate["implemented"] = False
    candidate["status"] = "stage_a5_real_pass_dispatch_linked_pending_measured_success"
    implementation_status = candidate.get("implementation_status")
    if not isinstance(implementation_status, dict):
        raise AssertionError("manifest implementation_status must be an object")
    implementation_status["runtime_state"] = "fallback_only"
    implementation_status["real_gpu_pass"] = False
    implementation_status.pop("segment_4h_real_pass_measured_success", None)
    return candidate


def pre_close_out_tonemap_manifest(base: dict[str, object]) -> dict[str, object]:
    """Normalize the real (close-out) tonemap manifest back to the pre-close-out
    fail-closed shape.

    The GRX-010 close-out flipped the REAL tonemap manifest to implemented=true,
    runtime_state=fallback_only_by_default_real_pass_optin_measured,
    real_gpu_pass=true, and real_d3d12_dispatch_recorded=true. The probe accepts
    those new values only while the strict measured success artifact
    real_pass_enablement_success_evidence.json exists AND passes the full audit
    (grx010_real_pass_measured_success_active). This derivation restores the old
    false-valued shape so the pre-close-out assertions stay testable without
    touching the real manifest on disk."""
    candidate = copy.deepcopy(base)
    candidate["implemented"] = False
    candidate["status"] = (
        "segment_a_contract_offline_kernel_bridge_gate_default_disabled"
    )
    implementation_status = candidate.get("implementation_status")
    if not isinstance(implementation_status, dict):
        raise AssertionError("tonemap manifest implementation_status must be an object")
    implementation_status["runtime_state"] = "fallback_only"
    implementation_status["real_gpu_pass"] = False
    implementation_status["real_d3d12_dispatch_recorded"] = False
    implementation_status.pop("real_pass_measured_success", None)
    return candidate


def validation_failed_manifest(base: dict[str, object]) -> dict[str, object]:
    manifest = copy.deepcopy(base)
    manifest["offline_compile_status"] = "validation_failed"
    manifest["compile_blockers"] = [
        {
            "category": "validation_failed",
            "summary": "The offline compile attempt produced a DXIL container, but the dxc validator rejected it; fix the validator rejection before segment 3 resource mapping.",
        }
    ]
    implementation_status = manifest.get("implementation_status")
    if not isinstance(implementation_status, dict):
        raise AssertionError("manifest implementation_status must be an object")
    implementation_status["segment"] = 2
    implementation_status["real_gpu_pass"] = False
    implementation_status["segment_3a_last_result"] = "validation_failed"
    implementation_status["segment_3a_last_result_note"] = (
        "offline compile evidence records validation_failed; runtime remains fallback_only "
        "and manifest remains at segment 2 until the DXIL validator rejection is fixed"
    )
    return manifest


def validation_failed_evidence(base: dict[str, object]) -> dict[str, object]:
    evidence = copy.deepcopy(base)
    evidence["status"] = "validation_failed"
    evidence["runtime_state"] = "fallback_only"
    evidence["manifest_segment_after_run"] = 2
    evidence["blocker_category"] = "validation_failed"
    evidence["blocker_summary"] = (
        "The offline compile attempt produced a DXIL container, but the dxc validator rejected it; "
        "see compile stderr for validator details."
    )
    return evidence


def segment4b_success_evidence(
    canonical_evidence: dict[str, object],
    raw_buffer_evidence: dict[str, object],
) -> dict[str, object]:
    """Derive a success evidence for segment 4b probe testing.

    The canonical evidence honestly records the texture-capable compile attempt
    (status=compile_failed). The probe's new fail-closed elif intercepts that
    state and returns provide_grx009_runtime_mappable_luminance_kernel_artifact
    before the segment 4b branch can fire. To test the segment 4b branch in
    isolation, derive a success-shaped evidence from the raw-buffer historical
    evidence (status=success, manifest_segment_after_run=3,
    produced_by_current_run=true) but override its artifact paths to the
    canonical paths declared in the canonical evidence — the on-disk canonical
    artifacts carry the raw-buffer bytes (per the fail-closed copy in
    compile_offline.py), so the success-path file/hash checks resolve against
    real on-disk artifacts. This maintains honest fail-closed semantics: the
    probe's preflight still requires real success evidence for the segment 4b
    branch; only the test fixture is shaped to exercise that branch.
    """
    canonical_artifacts = canonical_evidence.get("artifacts")
    if not isinstance(canonical_artifacts, dict):
        raise AssertionError("canonical evidence artifacts must be an object")
    canonical_fallback = canonical_artifacts.get("bridge_tracked_fallback")
    if not isinstance(canonical_fallback, dict):
        # GRX-009 stage A3: the canonical evidence is success-shaped (the
        # owner-approved hlsl_bridge_workaround texture package) with flat
        # artifacts.{dxil,root_signature,descriptor_layout} entries that
        # already resolve against the real on-disk canonical files, so it IS
        # the segment-4b success fixture.
        if canonical_evidence.get("status") != "success":
            raise AssertionError(
                "canonical evidence without bridge_tracked_fallback must be "
                "success-shaped"
            )
        return copy.deepcopy(canonical_evidence)
    evidence = copy.deepcopy(raw_buffer_evidence)
    # Canonical compile_failed evidence stores its real on-disk artifacts under
    # artifacts.bridge_tracked_fallback.{dxil,root_signature,descriptor_layout}
    # (the raw-buffer bytes copied from raw_buffer_historical so the bridge
    # include_bytes! works). The attempted_texture_dxil sub-object records the
    # failed texture-capable compile (semantic_status=missing) and carries no
    # on-disk artifact paths. Read the canonical paths from
    # bridge_tracked_fallback so the synthetic segment-4b success fixture
    # resolves against real on-disk artifacts.
    artifacts = evidence.get("artifacts")
    if not isinstance(artifacts, dict):
        raise AssertionError("raw-buffer evidence artifacts must be an object")
    for key in ("dxil", "root_signature", "descriptor_layout"):
        canonical_artifact = canonical_fallback.get(key)
        if not isinstance(canonical_artifact, dict):
            raise AssertionError(
                f"canonical artifacts.bridge_tracked_fallback.{key} must be an object"
            )
        canonical_path = canonical_artifact.get("path")
        if not isinstance(canonical_path, str):
            raise AssertionError(
                f"canonical artifacts.bridge_tracked_fallback.{key} path must be a string"
            )
        artifact = artifacts.get(key)
        if not isinstance(artifact, dict):
            raise AssertionError(f"raw-buffer artifact for {key} must be an object")
        artifact["path"] = canonical_path
    # The raw-buffer historical compile ran without a signed DXC validator
    # suite, so its stderr records "validator gate SKIPPED". The probe's
    # grx009_compile_stderr_has_skip_marker rejects success evidence whose
    # stderr contains "skipped". This synthetic segment-4b fixture exercises
    # the segment 4b control-flow branch in isolation; clear the stderr_path
    # so the synthetic success evidence is not rejected for an environment-
    # specific validator-skip marker that is unrelated to the branch under
    # test. The real canonical evidence (compile_failed) still honestly
    # records the skip marker via the fail-closed path.
    commands = evidence.get("commands")
    if isinstance(commands, list):
        for command in commands:
            if isinstance(command, dict):
                command["stderr_path"] = ""
    return evidence


def compile_failed_runtime_mappable_true_evidence(
    base: dict[str, object],
) -> dict[str, object]:
    """Construct contradictory evidence: status=compile_failed but
    runtime_mappable=true.

    This must be rejected by the probe's
    ``grx009_compile_manifest_consistency_issue`` red-check (which fires before
    the fail-closed branch). The artifacts structure is inherited unchanged
    from ``base`` because the red-check returns before any artifact audit.
    Used to verify the probe fails closed on contradictory evidence.
    """
    evidence = copy.deepcopy(base)
    evidence["status"] = "compile_failed"
    evidence["runtime_state"] = "fallback_only"
    evidence["manifest_segment_after_run"] = 2
    evidence["blocker_category"] = "dxil_container_missing"
    evidence["blocker_summary"] = (
        "The offline compile attempt did not leave a real DXIL container artifact."
    )
    evidence["runtime_mappable"] = True  # contradiction: failed compile cannot be runtime-mappable
    evidence["attempted_binding_kinds"] = ["texture2d", "rwtexture2d"]
    return evidence


def assert_contains(text: str, needle: str) -> None:
    if needle not in text:
        raise AssertionError(f"expected output to contain {needle!r}")


def assert_not_contains(text: str, needle: str) -> None:
    if needle in text:
        raise AssertionError(f"expected output not to contain {needle!r}")


def write_fixture_pass_dir(
    pass_dir: pathlib.Path,
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    (pass_dir / "PASS_CONTRACT.md").write_bytes(REAL_CONTRACT_PATH.read_bytes())
    write_json(pass_dir / "pass_manifest.json", validation_failed_manifest(manifest))
    write_json(pass_dir / "offline_compile_evidence.json", validation_failed_evidence(evidence))
    (pass_dir / "compile_evidence.schema.json").write_bytes(REAL_SCHEMA_PATH.read_bytes())


def write_segment4a_fixture_pass_dir(
    pass_dir: pathlib.Path,
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    (pass_dir / "artifacts").mkdir(parents=True, exist_ok=True)
    (pass_dir / "PASS_CONTRACT.md").write_bytes(REAL_CONTRACT_PATH.read_bytes())
    write_json(pass_dir / "pass_manifest.json", manifest)
    write_json(pass_dir / "offline_compile_evidence.json", evidence)
    (pass_dir / "compile_evidence.schema.json").write_bytes(REAL_SCHEMA_PATH.read_bytes())
    (pass_dir / "resource_mapping.md").write_bytes(REAL_RESOURCE_MAPPING_PATH.read_bytes())
    (pass_dir / "artifacts" / "luminance_reduction_descriptor_layout.json").write_bytes(
        REAL_DESCRIPTOR_LAYOUT_PATH.read_bytes()
    )
    # Segment 4i fail-closed manifests reference the raw-buffer historical
    # evidence (grx009_segment4i_raw_buffer_backing_ok reads it relative to
    # GRX009_PASS_DIR); copy it in so the fixture is self-contained whether
    # the probe runs in-process (monkeypatched GRX009_PASS_DIR) or as a
    # subprocess driven by RURIX_GRX009_PASS_DIR.
    if REAL_RAW_BUFFER_EVIDENCE_PATH.is_file():
        (pass_dir / "offline_compile_evidence_raw_buffer.json").write_bytes(
            REAL_RAW_BUFFER_EVIDENCE_PATH.read_bytes()
        )
    # GRX-009 stage A3: the owner-approved provenance policy evidence is one
    # half of the canonical texture switch
    # (grx009_canonical_texture_switch_active); copy it so fixtures derived
    # from the real success-shaped canonical evidence keep the switch active.
    if REAL_TEXTURE_ARTIFACT_PROVENANCE_POLICY_PATH.is_file():
        (pass_dir / "texture_artifact_provenance_policy.json").write_bytes(
            REAL_TEXTURE_ARTIFACT_PROVENANCE_POLICY_PATH.read_bytes()
        )


def make_segment4a_manifest(manifest: dict[str, object]) -> dict[str, object]:
    """Derive a coherent segment-4a manifest from the real manifest.

    The real luminance manifest has advanced to segment 4b; the segment-4a
    probe cases still need a coherent 4a fixture, so downgrade the segment
    detail and drop the 4b-only implementation fields."""
    candidate = copy.deepcopy(manifest)
    implementation_status = candidate.get("implementation_status")
    if not isinstance(implementation_status, dict):
        raise AssertionError("manifest implementation_status must be an object")
    implementation_status["segment_detail"] = "4a_runtime_binding_preflight"
    implementation_status.pop("segment_4b_gated_dispatch_bringup", None)
    implementation_status.pop("real_d3d12_dispatch_recorded", None)
    return candidate


def make_segment4b_manifest(manifest: dict[str, object]) -> dict[str, object]:
    """Derive a coherent segment-4b manifest from the real manifest.

    The real luminance manifest now records ``offline_compile_status=compile_failed``
    and ``segment_3a_last_result=compile_failed`` (segment 4i fail-closed: patched
    llc lacks ``llvm.dx.resource.load.texture.2d`` intrinsic support). The
    segment-4b probe cases exercise the segment 4b branch in isolation using a
    synthetic success-shaped evidence derived from the raw-buffer historical
    evidence (``segment4b_success_evidence``). For
    ``grx009_offline_compile_success_evidence_ok`` and
    ``grx009_compile_manifest_consistency_issue`` to pass under that synthetic
    success evidence (non-fail-closed path), the manifest's
    ``offline_compile_status`` and ``segment_3a_last_result`` must agree — i.e.,
    ``success``. This derivation does NOT alter the real manifest on disk; it
    only produces a fixture manifest used inside ``tempfile.TemporaryDirectory``.
    The real canonical evidence (``compile_failed``) is still honestly verified
    by the fail-closed red/green cases (``run_segment4i_contradiction_red_case``,
    ``run_segment4i_fail_closed_green_case``).
    """
    candidate = copy.deepcopy(manifest)
    candidate["offline_compile_status"] = "success"
    implementation_status = candidate.get("implementation_status")
    if not isinstance(implementation_status, dict):
        raise AssertionError("manifest implementation_status must be an object")
    implementation_status["segment_3a_last_result"] = "success"
    return candidate


def with_probe_pass_dir(probe: object, pass_dir: pathlib.Path, callback: object) -> object:
    saved = {
        "GRX009_PASS_DIR": probe.GRX009_PASS_DIR,
        "GRX009_PASS_CONTRACT": probe.GRX009_PASS_CONTRACT,
        "GRX009_PASS_MANIFEST": probe.GRX009_PASS_MANIFEST,
        "GRX009_RESOURCE_MAPPING": probe.GRX009_RESOURCE_MAPPING,
        "GRX009_DESCRIPTOR_LAYOUT": probe.GRX009_DESCRIPTOR_LAYOUT,
        "GRX009_COMPILE_EVIDENCE": probe.GRX009_COMPILE_EVIDENCE,
        "GRX009_TEXTURE_DXC_FEASIBILITY_EVIDENCE": (
            probe.GRX009_TEXTURE_DXC_FEASIBILITY_EVIDENCE
        ),
        "GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC": (
            probe.GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC
        ),
        "GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DESIGN": (
            probe.GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DESIGN
        ),
        "GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_SCAFFOLD_EVIDENCE": (
            probe.GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_SCAFFOLD_EVIDENCE
        ),
        "GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR": (
            probe.GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR
        ),
        "GRX009_DXC_TEXTURE_BRIDGE_DESCRIPTOR_LAYOUT": (
            probe.GRX009_DXC_TEXTURE_BRIDGE_DESCRIPTOR_LAYOUT
        ),
        "GRX009_DXC_TEXTURE_BRIDGE_ROOT_SIGNATURE_METADATA": (
            probe.GRX009_DXC_TEXTURE_BRIDGE_ROOT_SIGNATURE_METADATA
        ),
        "GRX009_DXC_TEXTURE_BRIDGE_RTS0_ARTIFACT": (
            probe.GRX009_DXC_TEXTURE_BRIDGE_RTS0_ARTIFACT
        ),
        "GRX009_DXC_TEXTURE_DESCRIPTOR_RTS0_CROSSCHECK_EVIDENCE": (
            probe.GRX009_DXC_TEXTURE_DESCRIPTOR_RTS0_CROSSCHECK_EVIDENCE
        ),
        "GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_EVIDENCE": (
            probe.GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_EVIDENCE
        ),
        "GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC": (
            probe.GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC
        ),
        "GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE": (
            probe.GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE
        ),
        "GRX009_COMPILE_SCHEMA": probe.GRX009_COMPILE_SCHEMA,
        "GRX009_RAW_BUFFER_COMPILE_EVIDENCE": probe.GRX009_RAW_BUFFER_COMPILE_EVIDENCE,
        "grx009_patch_stack_ready": probe.grx009_patch_stack_ready,
        "grx009_patch_0004_applyable": probe.grx009_patch_0004_applyable,
    }
    probe.GRX009_PASS_DIR = pass_dir
    probe.GRX009_PASS_CONTRACT = pass_dir / "PASS_CONTRACT.md"
    probe.GRX009_PASS_MANIFEST = pass_dir / "pass_manifest.json"
    probe.GRX009_RESOURCE_MAPPING = pass_dir / "resource_mapping.md"
    probe.GRX009_DESCRIPTOR_LAYOUT = (
        pass_dir / "artifacts" / "luminance_reduction_descriptor_layout.json"
    )
    probe.GRX009_COMPILE_EVIDENCE = pass_dir / "offline_compile_evidence.json"
    probe.GRX009_TEXTURE_DXC_FEASIBILITY_EVIDENCE = (
        pass_dir / "texture_dxc_feasibility_evidence.json"
    )
    probe.GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC = (
        pass_dir / "dxc_texture_artifact_bridge.md"
    )
    probe.GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_DESIGN = (
        pass_dir / "dxc_texture_artifact_bridge_design.json"
    )
    probe.GRX009_DXC_TEXTURE_ARTIFACT_BRIDGE_SCAFFOLD_EVIDENCE = (
        pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json"
    )
    probe.GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR = (
        pass_dir / "artifacts" / "dxc_texture_bridge"
    )
    probe.GRX009_DXC_TEXTURE_BRIDGE_DESCRIPTOR_LAYOUT = (
        probe.GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR / "descriptor_layout.json"
    )
    probe.GRX009_DXC_TEXTURE_BRIDGE_ROOT_SIGNATURE_METADATA = (
        probe.GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR / "root_signature_scaffold.json"
    )
    probe.GRX009_DXC_TEXTURE_BRIDGE_RTS0_ARTIFACT = (
        probe.GRX009_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR / "root_signature.rts0.bin"
    )
    probe.GRX009_DXC_TEXTURE_DESCRIPTOR_RTS0_CROSSCHECK_EVIDENCE = (
        pass_dir / "dxc_texture_descriptor_rts0_crosscheck_evidence.json"
    )
    probe.GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_EVIDENCE = (
        pass_dir / "texture_artifact_provenance_policy.json"
    )
    probe.GRX009_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC = (
        pass_dir / "texture_artifact_provenance_policy.md"
    )
    probe.GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE = (
        pass_dir / "real_pass_enablement_success_evidence.json"
    )
    probe.GRX009_COMPILE_SCHEMA = pass_dir / "compile_evidence.schema.json"
    probe.GRX009_RAW_BUFFER_COMPILE_EVIDENCE = (
        pass_dir / "offline_compile_evidence_raw_buffer.json"
    )
    probe.grx009_patch_stack_ready = lambda result=None: True
    probe.grx009_patch_0004_applyable = lambda result=None: True
    try:
        return callback()
    finally:
        for name, value in saved.items():
            setattr(probe, name, value)


def run_green_case(manifest: dict[str, object], evidence: dict[str, object]) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
    output = completed.stdout
    assert_contains(output, "grx009_segment3a_compile_ready: false")
    assert_contains(output, "grx009_compile_evidence_status: validation_failed")
    assert_contains(output, "grx009_compile_blocker_category: validation_failed")
    assert_contains(
        output,
        "next_action: fix_grx009_luminance_segment3a_dxil_container_body_lowering_blocker",
    )
    assert_not_contains(
        output,
        "next_action: start_grx009_luminance_reduction_real_gpu_pass",
    )
    assert_not_contains(
        output,
        "next_action: start_grx009_luminance_segment3_resource_mapping",
    )


def run_red_case(manifest: dict[str, object], evidence: dict[str, object]) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    mismatched_manifest = validation_failed_manifest(manifest)
    mismatched_manifest["compile_blockers"] = [
        {
            "category": "toolchain_missing",
            "summary": "different blocker category",
        }
    ]
    issue = probe.grx009_compile_manifest_consistency_issue(
        mismatched_manifest,
        validation_failed_evidence(evidence),
    )
    if issue is None:
        raise AssertionError("expected validation_failed blocker mismatch to be reported")
    if "latest evidence blocker=validation_failed" not in issue:
        raise AssertionError(f"unexpected mismatch message: {issue}")


def run_segment4i_contradiction_red_case(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    """Red: probe must reject compile_failed + runtime_mappable=true evidence.

    The probe's ``grx009_compile_manifest_consistency_issue`` red-check fires
    before the fail-closed branch and emits a contradiction mismatch warning;
    the contradictory evidence must NOT pass the segment 3a ready gate.
    """
    contradictory = compile_failed_runtime_mappable_true_evidence(evidence)
    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_segment4a_fixture_pass_dir(temp_pass_dir, manifest, contradictory)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
    # Red: the contradiction mismatch signal must be emitted as a warning.
    assert_contains(
        output,
        "[godot-toolchain] warning: GRX-009 evidence contradiction: "
        "status=compile_failed but runtime_mappable=true",
    )
    assert_contains(
        output,
        "a failed compile cannot be runtime-mappable",
    )
    # Red: the contradictory evidence must NOT pass the segment 3a ready gate
    # (grx009_offline_compile_success_evidence_ok returns False because the
    # consistency issue is non-None).
    assert_contains(output, "grx009_segment3a_compile_ready: false")
    # Red: the canonical evidence status/blocker are still surfaced honestly.
    assert_contains(output, "grx009_compile_evidence_status: compile_failed")
    assert_contains(
        output, "grx009_compile_blocker_category: dxil_container_missing"
    )


def run_segment4i_fail_closed_green_case(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    """Green (stage A3): the owner-approved success-shaped canonical evidence
    advances cleanly (no contradiction red-check, no fail-closed regression);
    with no 4c measured evidence in the fixture the probe requests the
    segment 4c real dispatch smoke next.
    """
    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_segment4a_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
    # Green: the canonical evidence advances to the 4c measured smoke request
    # (the fixture carries no 4c evidence).
    assert_contains(
        output,
        "next_action: provide_grx009_luminance_real_d3d12_dispatch_smoke",
    )
    assert_contains(output, "grx009_compile_evidence_status: success")
    assert_contains(output, "grx009_segment3a_compile_ready: true")
    assert_contains(output, "grx009_texture_dxc_feasibility_status: missing")
    assert_contains(output, "grx009_texture_dxc_feasibility_ready: false")
    assert_contains(output, "grx009_texture_dxc_feasibility_issue: missing")
    # Green: the canonical evidence must NOT trigger the contradiction warning.
    assert_not_contains(output, "GRX-009 evidence contradiction")


def texture_dxc_feasibility_evidence(pass_dir: pathlib.Path, sha256: str) -> dict[str, object]:
    dxil_path = pass_dir / "artifacts" / "toolchain_probe" / "dxc_texture" / "texture.dxil"
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "segment": "4k_texture_dxc_feasibility",
        "status": "success",
        "ready": True,
        "issue": None,
        "dxil_container": {
            "path": str(dxil_path.relative_to(ROOT)).replace("\\", "/"),
            "exists": True,
            "size_bytes": dxil_path.stat().st_size,
            "sha256": sha256,
            "artifact_kind": "dxil_container",
            "produced_by_current_run": True,
        },
        "validation": {
            "tool": "dxv.exe",
            "status": "pass",
        },
        "descriptor_binding_expectation": {
            "resources": [
                {"name": "src_luminance", "binding_kind": "texture2d"},
                {"name": "dst_luminance", "binding_kind": "rwtexture2d"},
            ]
        },
        "rurix_artifact_contract_comparison": {
            "satisfies_current_bridge_descriptor_layout_contract": False,
            "missing_work": [
                "root_signature_extraction",
                "descriptor_layout_synthesis",
                "binding_kind_mapping",
                "DXIL_validation_integration",
                "Rurix_source_provenance",
            ],
        },
    }


def run_texture_dxc_feasibility_probe_cases(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_segment4a_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        dxil_path = temp_pass_dir / "artifacts" / "toolchain_probe" / "dxc_texture" / "texture.dxil"
        dxil_path.parent.mkdir(parents=True, exist_ok=True)
        dxil_bytes = b"DXBC\x00texture-dxc-fixture"
        dxil_path.write_bytes(dxil_bytes)
        ready_doc = texture_dxc_feasibility_evidence(temp_pass_dir, sha256_bytes(dxil_bytes))
        write_json(temp_pass_dir / "texture_dxc_feasibility_evidence.json", ready_doc)

        def assert_ready() -> None:
            loaded = probe.grx009_texture_dxc_feasibility_evidence()
            if probe.grx009_texture_dxc_feasibility_status(loaded) != "success":
                raise AssertionError("expected texture dxc feasibility status success")
            if not probe.grx009_texture_dxc_feasibility_ready(loaded):
                raise AssertionError("expected texture dxc feasibility ready")
            if probe.grx009_texture_dxc_feasibility_issue(loaded) is not None:
                raise AssertionError("expected no texture dxc feasibility issue")

        with_probe_pass_dir(probe, temp_pass_dir, assert_ready)

        bad_doc = copy.deepcopy(ready_doc)
        bad_doc["dxil_container"]["sha256"] = "0" * 64
        write_json(temp_pass_dir / "texture_dxc_feasibility_evidence.json", bad_doc)

        def assert_hash_mismatch() -> None:
            loaded = probe.grx009_texture_dxc_feasibility_evidence()
            if probe.grx009_texture_dxc_feasibility_ready(loaded):
                raise AssertionError("hash mismatch must not be texture dxc ready")
            issue = probe.grx009_texture_dxc_feasibility_issue(loaded)
            if issue != "dxil_hash_mismatch":
                raise AssertionError(f"expected dxil_hash_mismatch, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_hash_mismatch)


def write_design_fixture_pass_dir(
    pass_dir: pathlib.Path,
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    write_segment4a_fixture_pass_dir(pass_dir, manifest, evidence)
    (pass_dir / "texture_dxc_feasibility_evidence.json").write_bytes(
        REAL_TEXTURE_DXC_FEASIBILITY_PATH.read_bytes()
    )
    (pass_dir / "dxc_texture_artifact_bridge.md").write_bytes(
        REAL_DXC_TEXTURE_ARTIFACT_BRIDGE_DOC_PATH.read_bytes()
    )
    (pass_dir / "dxc_texture_artifact_bridge_design.json").write_bytes(
        REAL_DXC_TEXTURE_ARTIFACT_BRIDGE_DESIGN_PATH.read_bytes()
    )
    # Keep the fixture policy evidence coherent with the fixture-local policy
    # document paths so the switch-active fixtures also exercise the policy
    # gate against fixture-local files.
    policy_doc_path = pass_dir / "texture_artifact_provenance_policy.md"
    policy_doc_path.write_bytes(
        REAL_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC_PATH.read_bytes()
    )
    policy_path = pass_dir / "texture_artifact_provenance_policy.json"
    if policy_path.is_file():
        policy = load_json(policy_path)
        policy["policy_document"] = str(policy_doc_path.relative_to(ROOT)).replace(
            "\\", "/"
        ) if policy_doc_path.is_relative_to(ROOT) else str(policy_doc_path)
        policy["bridge_contract_document"] = (
            str((pass_dir / "dxc_texture_artifact_bridge.md").relative_to(ROOT)).replace(
                "\\", "/"
            )
            if (pass_dir / "dxc_texture_artifact_bridge.md").is_relative_to(ROOT)
            else str(pass_dir / "dxc_texture_artifact_bridge.md")
        )
        write_json(policy_path, policy)


def rewrite_artifact_entry(entry: dict[str, object], path: pathlib.Path) -> None:
    entry["path"] = str(path.relative_to(ROOT)).replace("\\", "/")
    entry["size_bytes"] = path.stat().st_size
    entry["sha256"] = sha256_bytes(path.read_bytes())


def write_scaffold_fixture_pass_dir(
    pass_dir: pathlib.Path,
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    write_design_fixture_pass_dir(pass_dir, manifest, evidence)
    artifact_dir = pass_dir / "artifacts" / "dxc_texture_bridge"
    shutil.copytree(REAL_DXC_TEXTURE_BRIDGE_ARTIFACT_DIR, artifact_dir)
    scaffold = load_json(REAL_DXC_TEXTURE_ARTIFACT_BRIDGE_SCAFFOLD_PATH)
    scaffold["artifact_dir"] = str(artifact_dir.relative_to(ROOT)).replace("\\", "/")
    scaffold["source_feasibility_evidence"] = str(
        (pass_dir / "texture_dxc_feasibility_evidence.json").relative_to(ROOT)
    ).replace("\\", "/")
    dxil_metadata = scaffold.get("dxil_container_metadata")
    if not isinstance(dxil_metadata, dict):
        raise AssertionError("scaffold dxil metadata must be an object")
    container = dxil_metadata.get("container")
    if not isinstance(container, dict):
        raise AssertionError("scaffold container metadata must be an object")
    rewrite_artifact_entry(container, artifact_dir / "texture_bridge_scaffold.dxil")
    descriptor = scaffold.get("descriptor_layout_artifact")
    if not isinstance(descriptor, dict):
        raise AssertionError("scaffold descriptor metadata must be an object")
    rewrite_artifact_entry(descriptor, artifact_dir / "descriptor_layout.json")
    root_signature = scaffold.get("root_signature_scaffold")
    if not isinstance(root_signature, dict):
        raise AssertionError("scaffold root signature metadata must be an object")
    nested_descriptor = root_signature.get("descriptor_layout_artifact")
    if isinstance(nested_descriptor, dict):
        rewrite_artifact_entry(nested_descriptor, artifact_dir / "descriptor_layout.json")
    rts0 = root_signature.get("rts0_artifact")
    if isinstance(rts0, dict):
        rewrite_artifact_entry(rts0, artifact_dir / "root_signature.rts0.bin")
    rewrite_artifact_entry(root_signature, artifact_dir / "root_signature_scaffold.json")
    write_json(pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json", scaffold)


def crosscheck_evidence_for_pass_dir(pass_dir: pathlib.Path) -> dict[str, object]:
    artifact_dir = pass_dir / "artifacts" / "dxc_texture_bridge"
    descriptor_path = artifact_dir / "descriptor_layout.json"
    rts0_path = artifact_dir / "root_signature.rts0.bin"
    descriptor_sha = sha256_bytes(descriptor_path.read_bytes())
    rts0_sha = sha256_bytes(rts0_path.read_bytes())
    return {
        "schema_version": 1,
        "pass_id": "luminance_reduction",
        "cross_check_status": "success",
        "descriptor_rts0_crosscheck_ready": True,
        "descriptor_layout_artifact": {
            "path": str(descriptor_path.relative_to(ROOT)).replace("\\", "/"),
            "size_bytes": descriptor_path.stat().st_size,
            "sha256": descriptor_sha,
            "artifact_kind": "dxc_texture_bridge_descriptor_layout_scaffold",
        },
        "rts0_artifact": {
            "path": str(rts0_path.relative_to(ROOT)).replace("\\", "/"),
            "size_bytes": rts0_path.stat().st_size,
            "sha256": rts0_sha,
            "artifact_kind": "rurix_owned_rts0_root_signature",
        },
        "reserialized_rts0_artifact": {
            "path": str((artifact_dir / "root_signature.reserialized.rts0.bin").relative_to(ROOT)).replace("\\", "/"),
            "size_bytes": rts0_path.stat().st_size,
            "sha256": rts0_sha,
            "artifact_kind": "rurix_owned_rts0_root_signature_reserialized",
        },
        "byte_for_byte_match": True,
        "root_constants": "none",
        "runtime_mappable": False,
        "real_gpu_pass": False,
        "canonical_artifact_replaced": False,
        "offline_compile_status_changed": False,
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "next_action_if_ready": "define_grx009_texture_artifact_provenance_policy",
    }


def write_crosscheck_fixture_pass_dir(
    pass_dir: pathlib.Path,
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    write_scaffold_fixture_pass_dir(pass_dir, manifest, evidence)
    crosscheck = crosscheck_evidence_for_pass_dir(pass_dir)
    write_json(pass_dir / "dxc_texture_descriptor_rts0_crosscheck_evidence.json", crosscheck)
    artifact_dir = pass_dir / "artifacts" / "dxc_texture_bridge"
    root_signature_path = artifact_dir / "root_signature_scaffold.json"
    root_signature = load_json(root_signature_path)
    root_signature["cross_check_status"] = "success"
    root_signature["descriptor_rts0_crosscheck_ready"] = True
    root_signature["cross_check_evidence"] = str(
        (pass_dir / "dxc_texture_descriptor_rts0_crosscheck_evidence.json").relative_to(ROOT)
    ).replace("\\", "/")
    root_signature["descriptor_sha256"] = crosscheck["descriptor_layout_artifact"]["sha256"]
    root_signature["rts0_sha256"] = crosscheck["rts0_artifact"]["sha256"]
    root_signature["reserialized_rts0_sha256"] = crosscheck["reserialized_rts0_artifact"]["sha256"]
    root_signature["byte_for_byte_match"] = True
    write_json(root_signature_path, root_signature)
    scaffold = load_json(pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json")
    nested = scaffold.get("root_signature_scaffold")
    if not isinstance(nested, dict):
        raise AssertionError("nested root signature scaffold must be object")
    nested["cross_check_status"] = "success"
    nested["descriptor_rts0_crosscheck_ready"] = True
    nested["cross_check_evidence"] = root_signature["cross_check_evidence"]
    nested["descriptor_sha256"] = crosscheck["descriptor_layout_artifact"]["sha256"]
    nested["rts0_sha256"] = crosscheck["rts0_artifact"]["sha256"]
    nested["reserialized_rts0_sha256"] = crosscheck["reserialized_rts0_artifact"]["sha256"]
    nested["byte_for_byte_match"] = True
    rewrite_artifact_entry(nested, root_signature_path)
    write_json(pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json", scaffold)
    manifest_doc = load_json(pass_dir / "pass_manifest.json")
    implementation_status = manifest_doc.get("implementation_status")
    if not isinstance(implementation_status, dict):
        raise AssertionError("manifest implementation_status must be an object")
    implementation_status["segment_4k_dxc_texture_descriptor_rts0_crosscheck"] = {
        "status": "success",
        "descriptor_rts0_crosscheck_ready": True,
        "evidence": root_signature["cross_check_evidence"],
        "descriptor_layout": crosscheck["descriptor_layout_artifact"],
        "rts0_artifact": crosscheck["rts0_artifact"],
        "reserialized_rts0_artifact": crosscheck["reserialized_rts0_artifact"],
        "byte_for_byte_match": True,
        "runtime_mappable": False,
        "real_gpu_pass": False,
        "canonical_artifact_replaced": False,
        "offline_compile_status_changed": False,
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "next_action_when_ready": "define_grx009_texture_artifact_provenance_policy",
    }
    write_json(pass_dir / "pass_manifest.json", manifest_doc)


def write_provenance_policy_fixture_pass_dir(
    pass_dir: pathlib.Path,
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    write_crosscheck_fixture_pass_dir(pass_dir, manifest, evidence)
    policy_doc_path = pass_dir / "texture_artifact_provenance_policy.md"
    policy_doc_path.write_bytes(REAL_TEXTURE_ARTIFACT_PROVENANCE_POLICY_DOC_PATH.read_bytes())
    policy = load_json(REAL_TEXTURE_ARTIFACT_PROVENANCE_POLICY_PATH)
    policy["policy_document"] = str(policy_doc_path.relative_to(ROOT)).replace("\\", "/")
    policy["bridge_contract_document"] = str(
        (pass_dir / "dxc_texture_artifact_bridge.md").relative_to(ROOT)
    ).replace("\\", "/")
    write_json(pass_dir / "texture_artifact_provenance_policy.json", policy)
    manifest_doc = load_json(pass_dir / "pass_manifest.json")
    implementation_status = manifest_doc.get("implementation_status")
    if not isinstance(implementation_status, dict):
        raise AssertionError("manifest implementation_status must be an object")
    implementation_status["segment_4l_texture_artifact_provenance_policy"] = {
        "status": "success",
        "policy_ready": True,
        "evidence": str(
            (pass_dir / "texture_artifact_provenance_policy.json").relative_to(ROOT)
        ).replace("\\", "/"),
        "policy_document": policy["policy_document"],
        "bridge_contract_document": policy["bridge_contract_document"],
        "owner_decision": "approve_hlsl_bridge_workaround_as_temporary_runtime_mappable_canonical",
        "canonical_switch_exception": "owner_approved_hlsl_bridge_workaround",
        "runtime_mappable": False,
        "real_gpu_pass": False,
        "canonical_artifact_replaced": False,
        "offline_compile_status_changed": False,
        "provenance": "hlsl_bridge_workaround",
        "rurix_owned": False,
        "next_action_when_ready": "provide_grx009_runtime_mappable_luminance_kernel_artifact",
    }
    write_json(pass_dir / "pass_manifest.json", manifest_doc)


def run_dxc_texture_artifact_bridge_design_gate_cases(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_design_fixture_pass_dir(temp_pass_dir, manifest, evidence)

        def assert_ready() -> None:
            loaded = probe.grx009_dxc_texture_artifact_bridge_design_evidence()
            if not probe.grx009_dxc_texture_artifact_bridge_design_ready(
                loaded,
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
            ):
                issue = probe.grx009_dxc_texture_artifact_bridge_design_issue(
                    loaded,
                    probe.grx009_manifest(),
                    probe.grx009_compile_evidence(),
                    probe.grx009_texture_dxc_feasibility_evidence(),
                )
                raise AssertionError(f"expected design gate ready, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_ready)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
        report = load_json(ROOT / "target" / "grx" / "godot_toolchain_probe.json")
        # GRX-009 stage A3: the canonical evidence records the owner-approved
        # hlsl_bridge_workaround success, so the design-slice sub-ladder no
        # longer fires; the fixture (which carries no 4c measured evidence)
        # advances to the segment 4c real dispatch smoke request instead.
        assert_contains(
            output,
            "next_action: provide_grx009_luminance_real_d3d12_dispatch_smoke",
        )
        if report.get("grx009_dxc_texture_artifact_bridge_design_ready") is not True:
            raise AssertionError("expected design-ready gate true in JSON report")
        if report.get("next_action") != "provide_grx009_luminance_real_d3d12_dispatch_smoke":
            raise AssertionError(f"unexpected next_action in JSON report: {report.get('next_action')!r}")
        if report.get("grx009_compile_evidence_status") != "success":
            raise AssertionError(
                "offline compile evidence must record the owner-approved success"
            )

    red_cases: list[tuple[str, str]] = []

    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_design_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        write_json(
            temp_pass_dir / "real_pass_enablement_success_evidence.json",
            {"status": "success"},
        )

        def assert_real_pass_success_rejected() -> None:
            issue = probe.grx009_dxc_texture_artifact_bridge_design_issue(
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
            )
            if issue != "real_pass_enablement_success_evidence_must_not_exist":
                raise AssertionError(f"expected real-pass success rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_real_pass_success_rejected)

    # GRX-009 stage A3 red cases: the canonical success is accepted ONLY as
    # the owner-approved switch. Removing either half of the switch (the
    # policy evidence or the recorded workaround provenance), or claiming a
    # runtime-mappable FAILED compile, must still fail closed.
    def remove_policy_evidence(pass_dir: pathlib.Path) -> None:
        (pass_dir / "texture_artifact_provenance_policy.json").unlink()

    def strip_compile_provenance(pass_dir: pathlib.Path) -> None:
        compile_doc = load_json(pass_dir / "offline_compile_evidence.json")
        compile_doc.pop("provenance", None)
        write_json(pass_dir / "offline_compile_evidence.json", compile_doc)

    def failed_compile_claims_runtime_mappable(pass_dir: pathlib.Path) -> None:
        compile_doc = load_json(pass_dir / "offline_compile_evidence.json")
        compile_doc["status"] = "compile_failed"
        compile_doc["runtime_mappable"] = True
        write_json(pass_dir / "offline_compile_evidence.json", compile_doc)

    for mutate_fixture, expected_issue in (
        (remove_policy_evidence, "offline_compile_status_must_remain_compile_failed"),
        (strip_compile_provenance, "offline_compile_status_must_remain_compile_failed"),
        (
            failed_compile_claims_runtime_mappable,
            "offline_compile_runtime_mappable_must_remain_false",
        ),
    ):
        with tempfile.TemporaryDirectory() as tmp:
            temp_pass_dir = pathlib.Path(tmp)
            write_design_fixture_pass_dir(temp_pass_dir, manifest, evidence)
            mutate_fixture(temp_pass_dir)

            def assert_compile_rejected(expected: str = expected_issue) -> None:
                issue = probe.grx009_dxc_texture_artifact_bridge_design_issue(
                    probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                    probe.grx009_manifest(),
                    probe.grx009_compile_evidence(),
                    probe.grx009_texture_dxc_feasibility_evidence(),
                )
                if issue != expected:
                    raise AssertionError(f"expected {expected}, got {issue!r}")

            with_probe_pass_dir(probe, temp_pass_dir, assert_compile_rejected)

    for key, value, expected_issue in (
        ("runtime_state", "real_pass", "manifest_runtime_state_must_remain_fallback_only"),
        ("real_gpu_pass", True, "manifest_real_gpu_pass_must_remain_false"),
    ):
        with tempfile.TemporaryDirectory() as tmp:
            temp_pass_dir = pathlib.Path(tmp)
            bad_manifest = copy.deepcopy(manifest)
            implementation_status = bad_manifest.get("implementation_status")
            if not isinstance(implementation_status, dict):
                raise AssertionError("manifest implementation_status must be an object")
            implementation_status[key] = value
            write_design_fixture_pass_dir(temp_pass_dir, bad_manifest, evidence)

            def assert_manifest_rejected(expected: str = expected_issue) -> None:
                issue = probe.grx009_dxc_texture_artifact_bridge_design_issue(
                    probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                    probe.grx009_manifest(),
                    probe.grx009_compile_evidence(),
                    probe.grx009_texture_dxc_feasibility_evidence(),
                )
                if issue != expected:
                    raise AssertionError(f"expected {expected}, got {issue!r}")

            with_probe_pass_dir(probe, temp_pass_dir, assert_manifest_rejected)

    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_design_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        descriptor = load_json(
            temp_pass_dir / "artifacts" / "luminance_reduction_descriptor_layout.json"
        )
        resources = descriptor.get("resources")
        if not isinstance(resources, list):
            raise AssertionError("descriptor resources must be a list")
        # Stage A3: ["texture2d", "rwtexture2d"] is the approved canonical
        # shape; anything else (wrong slot order here) must still be rejected
        # even while the owner-approved switch is active.
        resources[0]["binding_kind"] = "rwtexture2d"
        resources[1]["binding_kind"] = "texture2d"
        write_json(
            temp_pass_dir / "artifacts" / "luminance_reduction_descriptor_layout.json",
            descriptor,
        )

        def assert_descriptor_rejected() -> None:
            issue = probe.grx009_dxc_texture_artifact_bridge_design_issue(
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
            )
            if issue != "canonical_descriptor_binding_kind_must_remain_raw_buffer_view":
                raise AssertionError(f"expected descriptor binding-kind rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_descriptor_rejected)


def run_dxc_texture_artifact_bridge_scaffold_gate_cases(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)

        def assert_ready() -> None:
            loaded = probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence()
            if not probe.grx009_dxc_texture_artifact_bridge_scaffold_ready(
                loaded,
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
            ):
                issue = probe.grx009_dxc_texture_artifact_bridge_scaffold_issue(
                    loaded,
                    probe.grx009_manifest(),
                    probe.grx009_compile_evidence(),
                    probe.grx009_texture_dxc_feasibility_evidence(),
                    probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                )
                raise AssertionError(f"expected scaffold gate ready, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_ready)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
        report = load_json(ROOT / "target" / "grx" / "godot_toolchain_probe.json")
        assert_contains(output, "grx009_dxc_texture_artifact_bridge_scaffold_ready: true")
        assert_contains(output, "grx009_dxc_texture_rts0_integration_ready: true")
        # GRX-009 stage A3: the design-slice sub-ladder no longer fires for the
        # owner-approved canonical success; the fixture (no 4c measured
        # evidence) advances to the segment 4c real dispatch smoke request.
        assert_contains(
            output,
            "next_action: provide_grx009_luminance_real_d3d12_dispatch_smoke",
        )
        assert_not_contains(output, "next_action: start_grx009_segment4h_real_pass_enablement_smoke")
        if report.get("grx009_dxc_texture_artifact_bridge_scaffold_ready") is not True:
            raise AssertionError("expected scaffold-ready gate true in JSON report")
        if report.get("grx009_dxc_texture_rts0_integration_ready") is not True:
            raise AssertionError("expected RTS0 integration-ready gate true in JSON report")
        if report.get("next_action") != "provide_grx009_luminance_real_d3d12_dispatch_smoke":
            raise AssertionError(f"unexpected scaffold next_action: {report.get('next_action')!r}")
        if report.get("grx009_compile_evidence_status") != "success":
            raise AssertionError(
                "offline compile evidence must record the owner-approved success"
            )

    red_cases: list[tuple[str, object, str]] = [
        ("runtime_mappable", True, "scaffold_runtime_mappable_must_be_false"),
        ("real_gpu_pass", True, "scaffold_real_gpu_pass_must_be_false"),
        (
            "canonical_artifact_replaced",
            True,
            "scaffold_canonical_artifact_replaced_must_be_false",
        ),
        ("rurix_owned", True, "scaffold_hlsl_workaround_rurix_owned_must_be_false"),
    ]
    for key, value, expected_issue in red_cases:
        with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
            temp_pass_dir = pathlib.Path(tmp)
            write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)
            scaffold = load_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json")
            scaffold[key] = value
            write_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json", scaffold)

            def assert_rejected(expected: str = expected_issue) -> None:
                issue = probe.grx009_dxc_texture_artifact_bridge_scaffold_issue(
                    probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                    probe.grx009_manifest(),
                    probe.grx009_compile_evidence(),
                    probe.grx009_texture_dxc_feasibility_evidence(),
                    probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                )
                if issue != expected:
                    raise AssertionError(f"expected {expected}, got {issue!r}")

            with_probe_pass_dir(probe, temp_pass_dir, assert_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        scaffold = load_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json")
        dxil_metadata = scaffold.get("dxil_container_metadata")
        if not isinstance(dxil_metadata, dict):
            raise AssertionError("dxil metadata must be object")
        dxil_metadata.pop("validation", None)
        write_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json", scaffold)

        def assert_validation_rejected() -> None:
            issue = probe.grx009_dxc_texture_artifact_bridge_scaffold_issue(
                probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
            )
            if issue != "scaffold_validation_metadata_missing":
                raise AssertionError(f"expected validation metadata rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_validation_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        scaffold = load_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json")
        descriptor = scaffold.get("descriptor_layout_artifact")
        if not isinstance(descriptor, dict):
            raise AssertionError("descriptor metadata must be object")
        resources = descriptor.get("resources")
        if not isinstance(resources, list) or not isinstance(resources[0], dict):
            raise AssertionError("descriptor resources must be objects")
        resources[0].pop("binding_kind", None)
        write_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json", scaffold)

        def assert_binding_kind_rejected() -> None:
            issue = probe.grx009_dxc_texture_artifact_bridge_scaffold_issue(
                probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
            )
            if issue != "scaffold_descriptor_binding_kind_missing_or_mismatch":
                raise AssertionError(f"expected binding-kind rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_binding_kind_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        descriptor = load_json(
            temp_pass_dir / "artifacts" / "luminance_reduction_descriptor_layout.json"
        )
        resources = descriptor.get("resources")
        if not isinstance(resources, list):
            raise AssertionError("canonical descriptor resources must be list")
        # Stage A3: only ["texture2d", "rwtexture2d"] (owner-approved) or the
        # historical ["raw_buffer_view", "raw_buffer_view"] are accepted; a
        # swapped slot order must still be rejected.
        resources[0]["binding_kind"] = "rwtexture2d"
        resources[1]["binding_kind"] = "texture2d"
        write_json(
            temp_pass_dir / "artifacts" / "luminance_reduction_descriptor_layout.json",
            descriptor,
        )

        def assert_canonical_descriptor_rejected() -> None:
            issue = probe.grx009_dxc_texture_artifact_bridge_scaffold_issue(
                probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
            )
            if issue not in {
                "canonical_descriptor_binding_kind_must_remain_raw_buffer_view",
                "design_gate_not_ready:canonical_descriptor_binding_kind_must_remain_raw_buffer_view",
            }:
                raise AssertionError(f"expected canonical descriptor rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_canonical_descriptor_rejected)


def run_dxc_texture_rts0_integration_gate_cases(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)

        def assert_ready() -> None:
            loaded = probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence()
            issue = probe.grx009_dxc_texture_rts0_integration_issue(
                loaded,
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
            )
            if issue is not None:
                raise AssertionError(f"expected RTS0 integration ready, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_ready)

    for descriptor_mutator, expected_issues in (
        (
            lambda resources: resources[0].pop("binding_kind", None),
            {
                "scaffold_gate_not_ready:scaffold_descriptor_binding_kind_missing_or_mismatch",
                "scaffold_gate_not_ready:scaffold_descriptor_layout_artifact_mismatch",
            },
        ),
        (
            lambda resources: resources[1].__setitem__("binding_kind", "texture2d"),
            {
                "scaffold_gate_not_ready:scaffold_descriptor_binding_kind_missing_or_mismatch",
                "scaffold_gate_not_ready:scaffold_descriptor_layout_artifact_mismatch",
            },
        ),
    ):
        with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
            temp_pass_dir = pathlib.Path(tmp)
            write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)
            descriptor_path = temp_pass_dir / "artifacts" / "dxc_texture_bridge" / "descriptor_layout.json"
            descriptor = load_json(descriptor_path)
            resources = descriptor.get("resources")
            if not isinstance(resources, list):
                raise AssertionError("descriptor resources must be list")
            descriptor_mutator(resources)
            write_json(descriptor_path, descriptor)
            scaffold = load_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json")
            descriptor_entry = scaffold.get("descriptor_layout_artifact")
            if not isinstance(descriptor_entry, dict):
                raise AssertionError("descriptor evidence must be object")
            rewrite_artifact_entry(descriptor_entry, descriptor_path)
            root_signature = scaffold.get("root_signature_scaffold")
            if not isinstance(root_signature, dict):
                raise AssertionError("root signature evidence must be object")
            nested_descriptor = root_signature.get("descriptor_layout_artifact")
            if not isinstance(nested_descriptor, dict):
                raise AssertionError("nested descriptor evidence must be object")
            rewrite_artifact_entry(nested_descriptor, descriptor_path)
            write_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json", scaffold)

            def assert_descriptor_rejected(expected: set[str] = expected_issues) -> None:
                issue = probe.grx009_dxc_texture_rts0_integration_issue(
                    probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                    probe.grx009_manifest(),
                    probe.grx009_compile_evidence(),
                    probe.grx009_texture_dxc_feasibility_evidence(),
                    probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                )
                if issue not in expected:
                    raise AssertionError(f"expected {expected}, got {issue!r}")

            with_probe_pass_dir(probe, temp_pass_dir, assert_descriptor_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        rts0_path = temp_pass_dir / "artifacts" / "dxc_texture_bridge" / "root_signature.rts0.bin"
        rts0_path.write_bytes(rts0_path.read_bytes() + b"tamper")

        def assert_rts0_hash_rejected() -> None:
            issue = probe.grx009_dxc_texture_rts0_integration_issue(
                probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
            )
            if issue not in {
                "scaffold_gate_not_ready:scaffold_rts0_artifact_mismatch",
                "rts0_artifact_hash_mismatch",
            }:
                raise AssertionError(f"expected RTS0 hash rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_rts0_hash_rejected)

    red_cases: list[tuple[str, object, str]] = [
        ("runtime_mappable", True, "scaffold_gate_not_ready:scaffold_runtime_mappable_must_be_false"),
        ("real_gpu_pass", True, "scaffold_gate_not_ready:scaffold_real_gpu_pass_must_be_false"),
        (
            "canonical_artifact_replaced",
            True,
            "scaffold_gate_not_ready:scaffold_canonical_artifact_replaced_must_be_false",
        ),
        (
            "rurix_owned",
            True,
            "scaffold_gate_not_ready:scaffold_hlsl_workaround_rurix_owned_must_be_false",
        ),
    ]
    for key, value, expected_issue in red_cases:
        with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
            temp_pass_dir = pathlib.Path(tmp)
            write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)
            scaffold = load_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json")
            scaffold[key] = value
            write_json(temp_pass_dir / "dxc_texture_artifact_bridge_scaffold_evidence.json", scaffold)

            def assert_fail_closed_rejected(expected: str = expected_issue) -> None:
                issue = probe.grx009_dxc_texture_rts0_integration_issue(
                    probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                    probe.grx009_manifest(),
                    probe.grx009_compile_evidence(),
                    probe.grx009_texture_dxc_feasibility_evidence(),
                    probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                )
                if issue != expected:
                    raise AssertionError(f"expected {expected}, got {issue!r}")

            with_probe_pass_dir(probe, temp_pass_dir, assert_fail_closed_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_scaffold_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
        report = load_json(ROOT / "target" / "grx" / "godot_toolchain_probe.json")
        assert_contains(output, "grx009_dxc_texture_rts0_integration_ready: true")
        # GRX-009 stage A3: the design-slice sub-ladder no longer fires for the
        # owner-approved canonical success; the fixture (no 4c measured
        # evidence) advances to the segment 4c real dispatch smoke request.
        assert_contains(
            output,
            "next_action: provide_grx009_luminance_real_d3d12_dispatch_smoke",
        )
        assert_not_contains(output, "next_action: start_grx009_segment4h_real_pass_enablement_smoke")
        if report.get("grx009_dxc_texture_rts0_integration_ready") is not True:
            raise AssertionError("expected RTS0 integration ready in JSON report")
        if report.get("grx009_compile_evidence_status") != "success":
            raise AssertionError(
                "offline compile evidence must record the owner-approved success"
            )


def run_dxc_texture_descriptor_rts0_crosscheck_gate_cases(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_crosscheck_fixture_pass_dir(temp_pass_dir, manifest, evidence)

        def assert_ready() -> None:
            issue = probe.grx009_dxc_texture_descriptor_rts0_crosscheck_issue(
                probe.grx009_dxc_texture_descriptor_rts0_crosscheck_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
            )
            if issue is not None:
                raise AssertionError(f"expected descriptor/RTS0 cross-check ready, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_ready)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
        report = load_json(ROOT / "target" / "grx" / "godot_toolchain_probe.json")
        assert_contains(output, "grx009_dxc_texture_descriptor_rts0_crosscheck_ready: true")
        # GRX-009 stage A3: the design-slice sub-ladder no longer fires for the
        # owner-approved canonical success; the fixture (no 4c measured
        # evidence) advances to the segment 4c real dispatch smoke request.
        assert_contains(
            output,
            "next_action: provide_grx009_luminance_real_d3d12_dispatch_smoke",
        )
        assert_not_contains(output, "next_action: start_grx009_segment4h_real_pass_enablement_smoke")
        if report.get("grx009_dxc_texture_descriptor_rts0_crosscheck_ready") is not True:
            raise AssertionError("expected descriptor/RTS0 cross-check ready in JSON report")
        if report.get("grx009_compile_evidence_status") != "success":
            raise AssertionError(
                "offline compile evidence must record the owner-approved success"
            )

    descriptor_mutators: list[tuple[object, set[str]]] = [
        (lambda resources: resources.pop(0), {"rts0_gate_not_ready:scaffold_gate_not_ready:scaffold_descriptor_layout_artifact_mismatch"}),
        (lambda resources: resources[0].__setitem__("register", 1), {"rts0_gate_not_ready:scaffold_gate_not_ready:scaffold_descriptor_layout_artifact_mismatch"}),
        (lambda resources: resources[0].__setitem__("space", 1), {"rts0_gate_not_ready:scaffold_gate_not_ready:scaffold_descriptor_layout_artifact_mismatch"}),
        (lambda resources: resources[0].__setitem__("class", "UAV"), {"rts0_gate_not_ready:scaffold_gate_not_ready:scaffold_descriptor_layout_artifact_mismatch"}),
        (lambda resources: resources[0].__setitem__("count", 2), {"rts0_gate_not_ready:scaffold_gate_not_ready:scaffold_descriptor_layout_artifact_mismatch"}),
        (lambda resources: resources[1].__setitem__("binding_kind", "texture2d"), {"rts0_gate_not_ready:scaffold_gate_not_ready:scaffold_descriptor_binding_kind_missing_or_mismatch", "rts0_gate_not_ready:scaffold_gate_not_ready:scaffold_descriptor_layout_artifact_mismatch"}),
    ]
    for mutator, expected_issues in descriptor_mutators:
        with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
            temp_pass_dir = pathlib.Path(tmp)
            write_crosscheck_fixture_pass_dir(temp_pass_dir, manifest, evidence)
            descriptor_path = temp_pass_dir / "artifacts" / "dxc_texture_bridge" / "descriptor_layout.json"
            descriptor = load_json(descriptor_path)
            resources = descriptor.get("resources")
            if not isinstance(resources, list):
                raise AssertionError("descriptor resources must be list")
            mutator(resources)
            write_json(descriptor_path, descriptor)
            crosscheck = load_json(temp_pass_dir / "dxc_texture_descriptor_rts0_crosscheck_evidence.json")
            rewrite_artifact_entry(crosscheck["descriptor_layout_artifact"], descriptor_path)
            write_json(temp_pass_dir / "dxc_texture_descriptor_rts0_crosscheck_evidence.json", crosscheck)

            def assert_descriptor_rejected(expected: set[str] = expected_issues) -> None:
                issue = probe.grx009_dxc_texture_descriptor_rts0_crosscheck_issue(
                    probe.grx009_dxc_texture_descriptor_rts0_crosscheck_evidence(),
                    probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                    probe.grx009_manifest(),
                    probe.grx009_compile_evidence(),
                    probe.grx009_texture_dxc_feasibility_evidence(),
                    probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                )
                if issue not in expected:
                    raise AssertionError(f"expected {expected}, got {issue!r}")

            with_probe_pass_dir(probe, temp_pass_dir, assert_descriptor_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_crosscheck_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        rts0_path = temp_pass_dir / "artifacts" / "dxc_texture_bridge" / "root_signature.rts0.bin"
        rts0_path.write_bytes(rts0_path.read_bytes() + b"tamper")

        def assert_rts0_hash_rejected() -> None:
            issue = probe.grx009_dxc_texture_descriptor_rts0_crosscheck_issue(
                probe.grx009_dxc_texture_descriptor_rts0_crosscheck_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
            )
            if issue not in {
                "rts0_gate_not_ready:scaffold_gate_not_ready:scaffold_rts0_artifact_mismatch",
                "rts0_gate_not_ready:rts0_artifact_hash_mismatch",
            }:
                raise AssertionError(f"expected RTS0 hash rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_rts0_hash_rejected)

    for key, value, expected_issue in (
        ("byte_for_byte_match", False, "descriptor_rts0_crosscheck_byte_for_byte_match_must_be_true"),
        ("runtime_mappable", True, "descriptor_rts0_crosscheck_runtime_mappable_must_be_false"),
        ("real_gpu_pass", True, "descriptor_rts0_crosscheck_real_gpu_pass_must_be_false"),
        ("canonical_artifact_replaced", True, "descriptor_rts0_crosscheck_canonical_artifact_replaced_must_be_false"),
        ("rurix_owned", True, "descriptor_rts0_crosscheck_hlsl_workaround_rurix_owned_must_be_false"),
    ):
        with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
            temp_pass_dir = pathlib.Path(tmp)
            write_crosscheck_fixture_pass_dir(temp_pass_dir, manifest, evidence)
            crosscheck = load_json(temp_pass_dir / "dxc_texture_descriptor_rts0_crosscheck_evidence.json")
            crosscheck[key] = value
            write_json(temp_pass_dir / "dxc_texture_descriptor_rts0_crosscheck_evidence.json", crosscheck)

            def assert_evidence_rejected(expected: str = expected_issue) -> None:
                issue = probe.grx009_dxc_texture_descriptor_rts0_crosscheck_issue(
                    probe.grx009_dxc_texture_descriptor_rts0_crosscheck_evidence(),
                    probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                    probe.grx009_manifest(),
                    probe.grx009_compile_evidence(),
                    probe.grx009_texture_dxc_feasibility_evidence(),
                    probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
                )
                if issue != expected:
                    raise AssertionError(f"expected {expected}, got {issue!r}")

            with_probe_pass_dir(probe, temp_pass_dir, assert_evidence_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_crosscheck_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        crosscheck = load_json(temp_pass_dir / "dxc_texture_descriptor_rts0_crosscheck_evidence.json")
        reserialized = crosscheck.get("reserialized_rts0_artifact")
        if not isinstance(reserialized, dict):
            raise AssertionError("reserialized RTS0 artifact must be object")
        reserialized["sha256"] = "0" * 64
        write_json(temp_pass_dir / "dxc_texture_descriptor_rts0_crosscheck_evidence.json", crosscheck)

        def assert_reserialized_hash_rejected() -> None:
            issue = probe.grx009_dxc_texture_descriptor_rts0_crosscheck_issue(
                probe.grx009_dxc_texture_descriptor_rts0_crosscheck_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
                probe.grx009_manifest(),
                probe.grx009_compile_evidence(),
                probe.grx009_texture_dxc_feasibility_evidence(),
                probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
            )
            if issue != "descriptor_rts0_crosscheck_reserialized_rts0_hash_mismatch":
                raise AssertionError(f"expected reserialized RTS0 hash rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_reserialized_hash_rejected)


def run_texture_artifact_provenance_policy_gate_cases(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    def current_issue() -> str | None:
        return probe.grx009_texture_artifact_provenance_policy_issue(
            probe.grx009_texture_artifact_provenance_policy_evidence(),
            probe.grx009_dxc_texture_descriptor_rts0_crosscheck_evidence(),
            probe.grx009_dxc_texture_artifact_bridge_scaffold_evidence(),
            probe.grx009_manifest(),
            probe.grx009_compile_evidence(),
            probe.grx009_texture_dxc_feasibility_evidence(),
            probe.grx009_dxc_texture_artifact_bridge_design_evidence(),
        )

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_provenance_policy_fixture_pass_dir(temp_pass_dir, manifest, evidence)

        def assert_ready() -> None:
            issue = current_issue()
            if issue is not None:
                raise AssertionError(f"expected provenance policy ready, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_ready)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
        report = load_json(ROOT / "target" / "grx" / "godot_toolchain_probe.json")
        assert_contains(output, "grx009_texture_artifact_provenance_policy_ready: true")
        # GRX-009 stage A3: the design-slice sub-ladder no longer fires for the
        # owner-approved canonical success; the fixture (no 4c measured
        # evidence) advances to the segment 4c real dispatch smoke request.
        assert_contains(
            output,
            "next_action: provide_grx009_luminance_real_d3d12_dispatch_smoke",
        )
        assert_not_contains(output, "next_action: start_grx009_segment4h_real_pass_enablement_smoke")
        if report.get("grx009_texture_artifact_provenance_policy_ready") is not True:
            raise AssertionError("expected provenance policy ready in JSON report")
        if report.get("grx009_compile_evidence_status") != "success":
            raise AssertionError(
                "offline compile evidence must record the owner-approved success"
            )

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_crosscheck_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        # Removing the policy evidence deactivates the owner-approved canonical
        # texture switch, so the whole prerequisite chain must fail closed on
        # the canonical success no longer being approved.
        (temp_pass_dir / "texture_artifact_provenance_policy.json").unlink()

        def assert_evidence_missing_rejected() -> None:
            issue = current_issue()
            if issue != (
                "crosscheck_gate_not_ready:rts0_gate_not_ready:"
                "scaffold_gate_not_ready:design_gate_not_ready:"
                "offline_compile_status_must_remain_compile_failed"
            ):
                raise AssertionError(f"expected missing policy evidence rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_evidence_missing_rejected)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
        assert_contains(output, "grx009_texture_artifact_provenance_policy_ready: false")
        assert_not_contains(output, "next_action: provide_grx009_runtime_mappable_luminance_kernel_artifact")
        assert_not_contains(output, "next_action: start_grx009_segment4h_real_pass_enablement_smoke")

    # Mutations that break the owner-approved switch itself (policy_ready,
    # status, owner decision) make the whole prerequisite chain fail closed on
    # the canonical success no longer being approved; the remaining mutations
    # keep the switch active and are rejected by the policy gate itself.
    switch_broken_issue = (
        "crosscheck_gate_not_ready:rts0_gate_not_ready:"
        "scaffold_gate_not_ready:design_gate_not_ready:"
        "offline_compile_status_must_remain_compile_failed"
    )
    for mutate, expected_issue in (
        (lambda policy: policy.__setitem__("policy_ready", False), switch_broken_issue),
        (lambda policy: policy.__setitem__("status", "pending"), switch_broken_issue),
        (lambda policy: policy.__setitem__("runtime_mappable", True), "provenance_policy_runtime_mappable_must_be_false"),
        (lambda policy: policy.__setitem__("real_gpu_pass", True), "provenance_policy_real_gpu_pass_must_be_false"),
        (lambda policy: policy.__setitem__("canonical_artifact_replaced", True), "provenance_policy_canonical_artifact_replaced_must_be_false"),
        (lambda policy: policy["provenance_policy"].__setitem__("rurix_owned", True), "provenance_policy_rurix_owned_must_be_false"),
        (lambda policy: policy["provenance_policy"].__setitem__("rurix_owned_rts0", False), "provenance_policy_rurix_owned_rts0_must_be_true"),
        (lambda policy: policy["provenance_policy"].__setitem__("canonical_switch_exception", "silent_switch"), "provenance_policy_canonical_switch_exception_mismatch"),
        (lambda policy: policy["provenance_policy"].__setitem__("revert_to_rurix_owned_when", []), "provenance_policy_revert_conditions_missing"),
        (lambda policy: policy["owner_decision"].__setitem__("decision", "approve_anything"), switch_broken_issue),
        (lambda policy: policy.__setitem__("next_action_if_ready", "enable_real_pass_now"), "provenance_policy_next_action_mismatch"),
    ):
        with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
            temp_pass_dir = pathlib.Path(tmp)
            write_provenance_policy_fixture_pass_dir(temp_pass_dir, manifest, evidence)
            policy_path = temp_pass_dir / "texture_artifact_provenance_policy.json"
            policy = load_json(policy_path)
            mutate(policy)
            write_json(policy_path, policy)

            def assert_policy_rejected(expected: str = expected_issue) -> None:
                issue = current_issue()
                if issue != expected:
                    raise AssertionError(f"expected {expected}, got {issue!r}")

            with_probe_pass_dir(probe, temp_pass_dir, assert_policy_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_provenance_policy_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        policy_doc_path = temp_pass_dir / "texture_artifact_provenance_policy.md"
        doc_text = policy_doc_path.read_text(encoding="utf-8")
        policy_doc_path.write_text(
            doc_text.replace("## Exception to Canonical Switch Conditions", "## Exception"),
            encoding="utf-8",
            newline="\n",
        )

        def assert_doc_sections_rejected() -> None:
            issue = current_issue()
            if issue != "provenance_policy_document_required_sections_missing":
                raise AssertionError(f"expected policy doc section rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_doc_sections_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_provenance_policy_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        bridge_doc_path = temp_pass_dir / "dxc_texture_artifact_bridge.md"
        bridge_text = bridge_doc_path.read_text(encoding="utf-8")
        bridge_doc_path.write_text(
            bridge_text.replace("texture_artifact_provenance_policy.md", "removed_policy_reference"),
            encoding="utf-8",
            newline="\n",
        )

        def assert_bridge_reference_rejected() -> None:
            issue = current_issue()
            if issue != "bridge_contract_document_missing_owner_exception_reference":
                raise AssertionError(f"expected bridge doc reference rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_bridge_reference_rejected)

    with tempfile.TemporaryDirectory(dir=ROOT) as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_provenance_policy_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        manifest_path = temp_pass_dir / "pass_manifest.json"
        manifest_doc = load_json(manifest_path)
        implementation_status = manifest_doc.get("implementation_status")
        if not isinstance(implementation_status, dict):
            raise AssertionError("manifest implementation_status must be an object")
        implementation_status.pop("segment_4l_texture_artifact_provenance_policy", None)
        write_json(manifest_path, manifest_doc)

        def assert_manifest_segment_rejected() -> None:
            issue = current_issue()
            if issue != "manifest_provenance_policy_status_missing":
                raise AssertionError(f"expected manifest segment rejection, got {issue!r}")

        with_probe_pass_dir(probe, temp_pass_dir, assert_manifest_segment_rejected)


def run_dxil_toolchain_preflight_cases() -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    empty_env: dict[str, str] = {}
    missing = probe.build_dxil_toolchain_preflight(empty_env)
    missing_reasons = missing.get("missing_reasons")
    if not isinstance(missing_reasons, list):
        raise AssertionError("preflight missing_reasons must be a list")
    if "RURIX_LLC_not_set" not in missing_reasons:
        raise AssertionError("expected RURIX_LLC_not_set missing reason")
    if "validator_suite_env_not_set" not in missing_reasons:
        raise AssertionError("expected validator_suite_env_not_set missing reason")

    with tempfile.TemporaryDirectory() as tmp:
        suite_dir = pathlib.Path(tmp) / "suite"
        suite_dir.mkdir()
        (suite_dir / "dxc.exe").write_text("fake dxc\n", encoding="utf-8")
        env = {
            "RURIX_DXC_DIR": str(suite_dir),
            "PATH": str(suite_dir),
        }
        partial = probe.probe_signed_dxc_validator_suite(env)
        if partial.get("status") == "PASS":
            raise AssertionError("partial validator suite must not pass")
        missing_files = partial.get("missing_files")
        if not isinstance(missing_files, list):
            raise AssertionError("validator suite missing_files must be a list")
        for filename in ("dxv.exe", "dxil.dll"):
            if filename not in missing_files:
                raise AssertionError(f"expected missing validator file: {filename}")

        (suite_dir / "dxv.exe").write_text("fake dxv\n", encoding="utf-8")
        (suite_dir / "dxil.dll").write_text("fake dxil\n", encoding="utf-8")
        fake_complete = probe.probe_signed_dxc_validator_suite(env)
        if fake_complete.get("status") == "PASS":
            raise AssertionError("fake text validator suite must not pass")
        fake_missing_reasons = fake_complete.get("missing_reasons")
        if not isinstance(fake_missing_reasons, list):
            raise AssertionError("fake validator suite missing_reasons must be a list")
        if not {"dxc_probe_failed", "dxv_probe_failed"}.intersection(fake_missing_reasons):
            raise AssertionError(f"expected fake executable probe failure: {fake_complete}")
        fake_files = fake_complete.get("files")
        if not isinstance(fake_files, dict):
            raise AssertionError("fake validator suite files must be an object")
        for filename in ("dxc.exe", "dxv.exe"):
            entry = fake_files.get(filename)
            if not isinstance(entry, dict):
                raise AssertionError(f"expected fake executable evidence for {filename}")
            for key in (
                "sha256",
                "probe_command",
                "probe_exit_code",
                "probe_output",
                "probe_timed_out",
                "probe_timeout_seconds",
            ):
                if key not in entry:
                    raise AssertionError(f"expected {key} for fake {filename}")
            if entry.get("probe_passed") is True:
                raise AssertionError(f"fake executable probe must not pass for {filename}")

    with tempfile.TemporaryDirectory() as tmp:
        suite_dir = pathlib.Path(tmp) / "suite"
        suite_dir.mkdir()
        shutil.copy2(sys.executable, suite_dir / "dxc.exe")
        shutil.copy2(sys.executable, suite_dir / "dxv.exe")
        (suite_dir / "dxil.dll").write_text("fake dxil\n", encoding="utf-8")
        env = {
            "RURIX_DXC_DIR": str(suite_dir),
            "PATH": str(suite_dir),
        }
        runnable_complete = probe.probe_signed_dxc_validator_suite(env)
        if runnable_complete.get("status") == "PASS":
            raise AssertionError(f"python-copy validator suite must not pass: {runnable_complete}")
        python_missing_reasons = runnable_complete.get("missing_reasons")
        if not isinstance(python_missing_reasons, list):
            raise AssertionError("python-copy validator suite missing_reasons must be a list")
        expected_reasons = {
            "dxc_probe_failed",
            "dxv_probe_failed",
            "dxc_identity_marker_missing",
            "dxv_identity_marker_missing",
        }
        if not expected_reasons.intersection(python_missing_reasons):
            raise AssertionError(
                f"expected python-copy identity/probe failure: {runnable_complete}"
            )
        files = runnable_complete.get("files")
        if not isinstance(files, dict):
            raise AssertionError("python-copy validator suite files must be an object")
        for filename in ("dxc.exe", "dxv.exe", "dxil.dll"):
            entry = files.get(filename)
            if not isinstance(entry, dict) or not entry.get("sha256"):
                raise AssertionError(f"expected sha256 for {filename}")
        for filename in ("dxc.exe", "dxv.exe"):
            entry = files.get(filename)
            if not isinstance(entry, dict):
                raise AssertionError(f"expected executable evidence for {filename}")
            if not entry.get("probe_command"):
                raise AssertionError(f"expected probe_command for {filename}")
            if entry.get("probe_exit_code") != 0:
                raise AssertionError(f"expected zero probe_exit_code for {filename}")
            if not isinstance(entry.get("probe_output"), str):
                raise AssertionError(f"expected probe_output for {filename}")
            if entry.get("probe_passed") is True:
                raise AssertionError(f"python-copy executable probe must not pass: {filename}")

    original_run = probe.subprocess.run

    def timeout_run(*args: object, **kwargs: object) -> object:
        raise subprocess.TimeoutExpired(
            cmd=kwargs.get("args", args[0] if args else "probe"),
            timeout=probe.PROBE_TIMEOUT_SECONDS,
            output="partial stdout",
            stderr="partial stderr",
        )

    try:
        probe.subprocess.run = timeout_run
        timeout_probe = probe.probe_validator_executable(
            pathlib.Path("timeout-dxc.exe"),
            ["--version"],
            probe.DXC_IDENTITY_MARKERS,
            "dxc_identity_marker_missing",
        )
        if timeout_probe.get("probe_timed_out") is not True:
            raise AssertionError(f"expected validator probe timeout: {timeout_probe}")
        if timeout_probe.get("probe_timeout_seconds") != probe.PROBE_TIMEOUT_SECONDS:
            raise AssertionError(f"expected validator probe timeout seconds: {timeout_probe}")
        if timeout_probe.get("probe_exit_code") is not None:
            raise AssertionError(f"expected null timeout exit code: {timeout_probe}")
        if timeout_probe.get("probe_passed") is not False:
            raise AssertionError(f"timeout validator probe must fail: {timeout_probe}")
        if "timed out" not in str(timeout_probe.get("probe_output")):
            raise AssertionError(f"expected timeout output text: {timeout_probe}")

        with tempfile.TemporaryDirectory() as tmp:
            llc_path = pathlib.Path(tmp) / "llc.exe"
            llc_path.write_text("fake llc\n", encoding="utf-8")
            llc_timeout = probe.probe_rurix_llc({"RURIX_LLC": str(llc_path)})
            if llc_timeout.get("missing_reason") != "RURIX_LLC_version_timeout":
                raise AssertionError(f"expected RURIX_LLC timeout reason: {llc_timeout}")
            if llc_timeout.get("version_timed_out") is not True:
                raise AssertionError(f"expected RURIX_LLC version_timed_out: {llc_timeout}")
    finally:
        probe.subprocess.run = original_run

    local_suite_dir = pathlib.Path(r"H:\dxc-round7\extracted\bin\x64")
    if local_suite_dir.exists():
        local_suite = probe.probe_signed_dxc_validator_suite(
            {"RURIX_DXC_DIR": str(local_suite_dir)}
        )
        if local_suite.get("status") != "PASS":
            raise AssertionError(f"local signed DXC validator suite did not pass: {local_suite}")
        local_files = local_suite.get("files")
        if not isinstance(local_files, dict):
            raise AssertionError("local signed DXC validator suite files must be an object")
        for filename in ("dxc.exe", "dxv.exe"):
            entry = local_files.get(filename)
            if not isinstance(entry, dict) or entry.get("probe_passed") is not True:
                raise AssertionError(f"expected local {filename} identity probe pass")
            if entry.get("probe_identity_passed") is not True:
                raise AssertionError(f"expected local {filename} identity marker pass")
        dxil_entry = local_files.get("dxil.dll")
        if not isinstance(dxil_entry, dict) or not dxil_entry.get("sha256"):
            raise AssertionError("expected local dxil.dll sha256")
    else:
        print(f"SKIP local signed DXC validator suite: {local_suite_dir} missing")


def run_segment4a_probe_cases(manifest: dict[str, object], evidence: dict[str, object]) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_segment4a_fixture_pass_dir(temp_pass_dir, manifest, evidence)

        def assert_ready() -> None:
            if not probe.grx009_segment4a_runtime_binding_preflight_inputs_ready():
                raise AssertionError("expected segment 4a inputs ready for coherent manifest")
            if not probe.grx009_segment4a_runtime_binding_preflight_ready(
                True,
                {"ok": True, "ready": True},
            ):
                raise AssertionError("expected segment 4a ready with applyable 0005")

        with_probe_pass_dir(probe, temp_pass_dir, assert_ready)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
        assert_contains(
            output,
            "grx009_segment4a_runtime_binding_preflight_inputs_ready: true",
        )
        assert_not_contains(
            output,
            "next_action: start_grx009_luminance_reduction_real_gpu_pass",
        )


def run_segment4a_runtime_state_reject_cases(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    for key, value in (("runtime_state", "dispatch_enabled"), ("real_gpu_pass", True)):
        candidate = copy.deepcopy(manifest)
        implementation_status = candidate.get("implementation_status")
        if not isinstance(implementation_status, dict):
            raise AssertionError("manifest implementation_status must be an object")
        implementation_status[key] = value
        with tempfile.TemporaryDirectory() as tmp:
            temp_pass_dir = pathlib.Path(tmp)
            write_segment4a_fixture_pass_dir(temp_pass_dir, candidate, evidence)

            def assert_rejected() -> None:
                if probe.grx009_segment4a_runtime_binding_preflight_inputs_ready():
                    raise AssertionError(f"expected segment 4a inputs to reject {key}={value!r}")

            with_probe_pass_dir(probe, temp_pass_dir, assert_rejected)


def run_segment4a_marker_reject_cases(
    manifest: dict[str, object],
    evidence: dict[str, object],
) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    missing_flag_manifest = copy.deepcopy(manifest)
    implementation_status = missing_flag_manifest.get("implementation_status")
    if not isinstance(implementation_status, dict):
        raise AssertionError("manifest implementation_status must be an object")
    implementation_status.pop("segment_4a_runtime_binding_preflight", None)
    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_segment4a_fixture_pass_dir(temp_pass_dir, missing_flag_manifest, evidence)

        def assert_missing_flag_rejected() -> None:
            if probe.grx009_segment4a_runtime_binding_preflight_inputs_ready():
                raise AssertionError("expected segment 4a inputs to reject missing 4a flag")

        with_probe_pass_dir(probe, temp_pass_dir, assert_missing_flag_rejected)

    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_segment4a_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        resource_mapping = temp_pass_dir / "resource_mapping.md"
        resource_mapping.write_text("resource mapping scaffold\n", encoding="utf-8")

        def assert_missing_marker_rejected() -> None:
            if probe.grx009_segment4a_runtime_binding_preflight_inputs_ready():
                raise AssertionError("expected segment 4a inputs to reject missing preflight marker")

        with_probe_pass_dir(probe, temp_pass_dir, assert_missing_marker_rejected)


def run_segment4b_probe_cases(manifest: dict[str, object], evidence: dict[str, object]) -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    # Coherent segment 4b manifest -> inputs ready and ready with applyable 0006.
    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_segment4a_fixture_pass_dir(temp_pass_dir, manifest, evidence)

        def assert_ready() -> None:
            saved_0005 = probe.grx009_patch_0005_applyable
            probe.grx009_patch_0005_applyable = lambda result=None: True
            try:
                if not probe.grx009_segment4b_gated_dispatch_bringup_inputs_ready():
                    raise AssertionError(
                        "expected segment 4b inputs ready for coherent manifest"
                    )
                if not probe.grx009_segment4b_gated_dispatch_bringup_ready(
                    True,
                    {"ok": True, "ready": True},
                ):
                    raise AssertionError("expected segment 4b ready with applyable 0006")
            finally:
                probe.grx009_patch_0005_applyable = saved_0005

        with_probe_pass_dir(probe, temp_pass_dir, assert_ready)

    # Missing segment_4b flag -> inputs must reject.
    missing_flag_manifest = copy.deepcopy(manifest)
    implementation_status = missing_flag_manifest.get("implementation_status")
    if not isinstance(implementation_status, dict):
        raise AssertionError("manifest implementation_status must be an object")
    implementation_status.pop("segment_4b_gated_dispatch_bringup", None)
    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_segment4a_fixture_pass_dir(temp_pass_dir, missing_flag_manifest, evidence)

        def assert_missing_flag_rejected() -> None:
            saved_0005 = probe.grx009_patch_0005_applyable
            probe.grx009_patch_0005_applyable = lambda result=None: True
            try:
                if probe.grx009_segment4b_gated_dispatch_bringup_inputs_ready():
                    raise AssertionError(
                        "expected segment 4b inputs to reject missing 4b flag"
                    )
            finally:
                probe.grx009_patch_0005_applyable = saved_0005

        with_probe_pass_dir(probe, temp_pass_dir, assert_missing_flag_rejected)

    # Full probe run over the coherent 4b fixture: next_action must SKIP the
    # real dispatch smoke and point at providing a real device dispatch smoke.
    with tempfile.TemporaryDirectory() as tmp:
        temp_pass_dir = pathlib.Path(tmp)
        write_segment4a_fixture_pass_dir(temp_pass_dir, manifest, evidence)
        env = os.environ.copy()
        env["RURIX_GRX009_PASS_DIR"] = str(temp_pass_dir)
        completed = subprocess.run(
            ["py", "-3", "ci\\godot_rurix_toolchain_probe.py"],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        output = completed.stdout
        assert_contains(
            output,
            "grx009_segment4b_gated_dispatch_bringup_inputs_ready: true",
        )
        assert_contains(
            output,
            "grx009_segment4b_gated_dispatch_bringup_ready: true",
        )
        assert_contains(
            output,
            "next_action: provide_grx009_luminance_real_d3d12_dispatch_smoke",
        )


def run_segment4f_sidecar_chain_parity_cases() -> None:
    """Smoke and probe must agree on the sidecar patch-application chain shape:
    patch_application_audit[-1].commit/tree must equal final_head/final_tree."""
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe
    from ci import grx009_godot_runtime_bridge_recording_smoke as smoke

    stack = smoke.patch_stack_identity()
    patches = stack.get("patches")
    if not isinstance(patches, list) or len(patches) != 8:
        raise AssertionError("smoke patch_stack_identity must record 8 patches")
    patch_audit = [
        {
            **entry,
            "order": index,
            "commit": f"{index}" * 40,
            "tree": f"{index + 1}" * 40,
        }
        for index, entry in enumerate(patches, start=1)
    ]
    final_entry = patch_audit[-1]
    sidecar = {
        "base_snapshot": "external/godot-master",
        "stack": "0001..0008",
        "patch_count": 8,
        "tracked_patch_stack_only": True,
        "expected_stack_ok": True,
        "final_status_clean": True,
        "actual_status_clean": True,
        "base_commit": "1" * 40,
        "base_tree": "2" * 40,
        "final_head": final_entry["commit"],
        "final_tree": final_entry["tree"],
        "actual_head": final_entry["commit"],
        "actual_tree": final_entry["tree"],
        "applied_patch_stack": {
            "stack": "0001..0008",
            "patches": copy.deepcopy(patches),
        },
        "patch_application_audit": patch_audit,
        "actual_source_root_at_generation": str(ROOT),
    }

    def fake_git_value(args: list[str], cwd: pathlib.Path) -> str | None:
        if args == ["rev-parse", "HEAD"]:
            return sidecar["final_head"]
        if args == ["rev-parse", "HEAD^{tree}"]:
            return sidecar["final_tree"]
        return None

    saved_git_value = smoke.git_value
    smoke.git_value = fake_git_value
    try:
        ok, errors, _audit = smoke.verify_source_provenance_sidecar(
            copy.deepcopy(sidecar), ROOT
        )
        if not ok or errors:
            raise AssertionError(f"valid sidecar chain fixture must pass smoke audit: {errors}")

        for key, expected_error in (
            ("commit", "patch_application_audit[-1].commit does not match final_head"),
            ("tree", "patch_application_audit[-1].tree does not match final_tree"),
        ):
            broken = copy.deepcopy(sidecar)
            broken["patch_application_audit"][-1][key] = "f" * 40
            ok, errors, _audit = smoke.verify_source_provenance_sidecar(broken, ROOT)
            if ok:
                raise AssertionError(
                    f"sidecar with broken last-audit {key} chain must NOT pass smoke audit"
                )
            if not any(expected_error in error for error in errors):
                raise AssertionError(
                    f"expected smoke chain error {expected_error!r}, got: {errors}"
                )
    finally:
        smoke.git_value = saved_git_value

    # Parity: the probe's success-evidence audit must reject the same broken
    # chain shape (last audit entry not matching final_head/final_tree).
    for key in ("commit", "tree"):
        parity = make_valid_segment4f_success_evidence(probe)
        parity["scratch_source_provenance"]["patch_application_audit"][-1][key] = "f" * 40
        if probe.grx009_segment4f_scratch_source_provenance_ok(parity):
            raise AssertionError(
                f"probe must reject a broken last-audit {key} chain like the smoke does"
            )

    print("segment 4f sidecar chain parity checks passed")


SEGMENT4G_FIXTURE_WIDTH = 64
SEGMENT4G_FIXTURE_HEIGHT = 64


def with_probe_segment4g_paths(
    probe: object, fixture_dir: pathlib.Path, callback: object
) -> object:
    saved = {
        name: getattr(probe, name)
        for name in (
            "GRX009_VISUAL_FALLBACK_SCHEMA",
            "GRX009_VISUAL_FALLBACK_EVIDENCE",
            "GRX009_VISUAL_FALLBACK_SUCCESS_EVIDENCE",
            "GRX009_MEASURED_FALLBACK_TELEMETRY",
            "GRX009_VISUAL_REFERENCE_FRAME",
            "GRX009_VISUAL_CANDIDATE_FRAME",
            "GRX009_VISUAL_DIFF_ARTIFACT",
        )
    }
    visual_dir = fixture_dir / "artifacts" / "visual"
    probe.GRX009_VISUAL_FALLBACK_SCHEMA = (
        fixture_dir / "visual_fallback_evidence.schema.json"
    )
    probe.GRX009_VISUAL_FALLBACK_EVIDENCE = fixture_dir / "visual_fallback_evidence.json"
    probe.GRX009_VISUAL_FALLBACK_SUCCESS_EVIDENCE = (
        fixture_dir / "visual_fallback_success_evidence.json"
    )
    probe.GRX009_MEASURED_FALLBACK_TELEMETRY = (
        fixture_dir / "measured_fallback_telemetry.json"
    )
    probe.GRX009_VISUAL_REFERENCE_FRAME = (
        visual_dir / "luminance_fallback_reference.rgb8"
    )
    probe.GRX009_VISUAL_CANDIDATE_FRAME = (
        visual_dir / "luminance_fallback_candidate.rgb8"
    )
    probe.GRX009_VISUAL_DIFF_ARTIFACT = visual_dir / "luminance_fallback_diff.rgb8"
    try:
        return callback()
    finally:
        for name, value in saved.items():
            setattr(probe, name, value)


def build_segment4g_fixture(
    probe: object,
    fixture_dir: pathlib.Path,
    *,
    reference_bytes: bytes,
    candidate_bytes: bytes,
) -> dict[str, object]:
    """Write real frame/diff/telemetry/schema fixture files and return a
    coherent segment 4g historical measured success evidence document."""
    visual_dir = fixture_dir / "artifacts" / "visual"
    visual_dir.mkdir(parents=True, exist_ok=True)
    diff_bytes = bytes(abs(a - b) for a, b in zip(reference_bytes, candidate_bytes))
    max_abs = max(diff_bytes) if diff_bytes else 0
    mean_abs = (sum(diff_bytes) / len(diff_bytes)) if diff_bytes else 0.0
    reference_path = visual_dir / "luminance_fallback_reference.rgb8"
    candidate_path = visual_dir / "luminance_fallback_candidate.rgb8"
    diff_path = visual_dir / "luminance_fallback_diff.rgb8"
    reference_path.write_bytes(reference_bytes)
    candidate_path.write_bytes(candidate_bytes)
    diff_path.write_bytes(diff_bytes)
    (fixture_dir / "visual_fallback_evidence.schema.json").write_bytes(
        (REAL_PASS_DIR / "visual_fallback_evidence.schema.json").read_bytes()
    )
    telemetry_doc = {
        "run_mode": "full",
        "evidence_level": "measured_local",
        "target_backend": "Godot 4.7-dev Windows D3D12 Forward+",
        "note": "segment 4g regression fixture: measured fallback telemetry",
        "passes": [
            {
                "pass_id": "luminance_reduction",
                "enable_state": "enabled",
                "fallback_reason": "validation_failed",
                "godot_fallback_active": True,
                "telemetry_timestamp": "2026-07-05T21:43:00+08:00",
                "telemetry_frame": 24,
            }
        ],
    }
    telemetry_path = fixture_dir / "measured_fallback_telemetry.json"
    write_json(telemetry_path, telemetry_doc)

    offline = probe.grx009_offline_artifact_digests(probe.grx009_compile_evidence())

    def fingerprint(path: pathlib.Path) -> dict[str, object]:
        return {
            "path": path.name,
            "sha256": probe.sha256_of_file(path),
            "size_bytes": path.stat().st_size,
        }

    matrix = {
        "disabled_default": {
            "role": "reference",
            "project_setting": "rendering/rurix_accel/passes/luminance_reduction/enabled=false",
            "exit_code": 0,
            "session_ready": True,
            "bridge_fallback_marker_observed": False,
            "bridge_fallback_marker_line": None,
        },
        "enabled_fallback": {
            "role": "candidate",
            "project_setting": "rendering/rurix_accel/passes/luminance_reduction/enabled=true",
            "exit_code": 0,
            "session_ready": True,
            "bridge_fallback_marker_observed": True,
            "bridge_fallback_marker_line": (
                "RurixAccel: luminance_reduction fallback rc=1; Godot native "
                "luminance path remains active."
            ),
        },
    }
    return {
        "schema_version": 1,
        "subject": "grx009_segment4g_luminance_visual_fallback_smoke",
        "pass_id": "luminance_reduction",
        "segment": "4g",
        "status": "success",
        "evidence_kind": "historical_measured_success",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "godot_runtime_luminance_path_enabled": False,
        "default_enable_state": "disabled",
        "gpu_timestamp_status": "not_yet",
        "performance_claim": "none",
        "artifact_hashes_match_offline_evidence": True,
        "artifacts": {
            "dxil": {"sha256": offline["dxil"]},
            "root_signature": {"sha256": offline["root_signature"]},
            "descriptor_layout": {"sha256": offline["descriptor_layout"]},
        },
        "dll_fingerprint": {
            "dll_path_at_run": "target/debug/rurix_godot.dll",
            "dll_sha256": "b" * 64,
            "dll_size_bytes": 65536,
            "build_profile": "debug",
            "features": [],
        },
        "checks": {name: True for name in probe.GRX009_SEGMENT4G_REQUIRED_CHECKS},
        "visual": {
            "measured_local": True,
            "metric_kind": probe.GRX009_SEGMENT4G_METRIC_KIND,
            "width": SEGMENT4G_FIXTURE_WIDTH,
            "height": SEGMENT4G_FIXTURE_HEIGHT,
            "format": probe.GRX009_SEGMENT4G_FRAME_FORMAT,
            "capture_frame_index": 24,
            "reference_frame": fingerprint(reference_path),
            "candidate_frame": fingerprint(candidate_path),
            "diff_artifact": fingerprint(diff_path),
            "max_abs_diff": max_abs,
            "mean_abs_diff": mean_abs,
            "max_abs_diff_threshold": probe.GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD,
            "mean_abs_diff_threshold": probe.GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD,
            "within_threshold": (
                max_abs <= probe.GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD
                and mean_abs <= probe.GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD
            ),
        },
        "fallback_telemetry": {
            "fallback_path_observed": True,
            "bridge_fallback_marker": "RurixAccel: luminance_reduction fallback rc=",
            "bridge_fallback_marker_line": matrix["enabled_fallback"][
                "bridge_fallback_marker_line"
            ],
            "pass_enable_matrix": matrix,
            "telemetry_document": fingerprint(telemetry_path),
            "no_fps_claim": True,
        },
        "pass_enable_matrix": matrix,
        "stdout_reference": (
            "GRX009Segment4g: scene ready\n"
            "GRX009Segment4g: captured frame=24 width=64 height=64\n"
            "ERROR: Could not load global script cache.\n"
        ),
        "stdout_candidate": (
            "GRX009Segment4g: scene ready\n"
            "RurixAccel: luminance_reduction fallback rc=1; Godot native "
            "luminance path remains active.\n"
            "GRX009Segment4g: captured frame=24 width=64 height=64\n"
            "ERROR: Could not load global script cache.\n"
        ),
        "runtime_log_audit": {
            leg: {
                "unexpected_rxgd_diag_count": 0,
                "rxgd_diag_allowed_by_tracked_patch_queue": False,
                "unexpected_godot_error_count": 0,
                "allowed_godot_errors": [
                    {
                        "message": probe.GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR,
                        "observed_count": 1,
                        "rationale": (
                            "Tolerated minimal smoke project cache warning after "
                            "clean fallback matrix and exit 0."
                        ),
                    }
                ],
                "unexpected_lines_tail": [],
            }
            for leg in ("reference", "candidate")
        },
    }


def run_segment4g_visual_fallback_cases() -> None:
    """Red/green coverage for the segment 4g visual/fallback readiness gate."""
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe
    from ci import grx009_segment4g_visual_fallback_smoke as smoke4g

    # The probe's pinned visual gate constants must match the harness pins.
    if probe.GRX009_SEGMENT4G_METRIC_KIND != smoke4g.METRIC_KIND:
        raise AssertionError("probe/harness metric_kind pins disagree")
    if probe.GRX009_SEGMENT4G_FRAME_FORMAT != smoke4g.FRAME_FORMAT:
        raise AssertionError("probe/harness frame format pins disagree")
    if probe.GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD != smoke4g.LDR_MAX_ABS_DIFF_THRESHOLD:
        raise AssertionError("probe/harness max_abs_diff threshold pins disagree")
    if probe.GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD != smoke4g.LDR_MEAN_ABS_DIFF_THRESHOLD:
        raise AssertionError("probe/harness mean_abs_diff threshold pins disagree")
    if probe.GRX009_SEGMENT4G_MIN_FRAME_DIMENSION != smoke4g.MIN_FRAME_DIMENSION:
        raise AssertionError("probe/harness minimum frame dimension pins disagree")
    if probe.GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR != smoke4g.ALLOWED_GODOT_ERROR:
        raise AssertionError("probe/harness allowed Godot error pins disagree")

    pixel_count = SEGMENT4G_FIXTURE_WIDTH * SEGMENT4G_FIXTURE_HEIGHT
    flat_frame = bytes((120, 130, 140)) * pixel_count

    def expect_issue(evidence: dict[str, object], needle: str, label: str) -> None:
        issue = probe.grx009_segment4g_visual_fallback_issue(evidence)
        if issue is None:
            raise AssertionError(f"{label}: expected the 4g gate to report an issue")
        if needle not in issue:
            raise AssertionError(f"{label}: expected issue containing {needle!r}, got {issue!r}")
        if probe.grx009_segment4g_visual_fallback_ready(evidence, True):
            raise AssertionError(f"{label}: gate must NOT be ready")

    with tempfile.TemporaryDirectory() as tmp:
        fixture_dir = pathlib.Path(tmp)
        valid = build_segment4g_fixture(
            probe,
            fixture_dir,
            reference_bytes=flat_frame,
            candidate_bytes=flat_frame,
        )

        def run_cases() -> None:
            # Green: valid measured_local visual evidence + fallback telemetry.
            # The gate re-verifies the tracked offline compile artifacts, so
            # only assert the green advance when they match in this env.
            offline = probe.grx009_offline_artifact_digests(probe.grx009_compile_evidence())
            current = {
                "dxil": probe.sha256_of_file(probe.GRX009_DXIL_ARTIFACT),
                "root_signature": probe.sha256_of_file(probe.GRX009_ROOT_SIGNATURE_ARTIFACT),
                "descriptor_layout": probe.sha256_of_file(probe.GRX009_DESCRIPTOR_LAYOUT),
            }
            artifacts_match = (
                all(v is not None for v in current.values()) and current == offline
            )
            if artifacts_match:
                issue = probe.grx009_segment4g_visual_fallback_issue(valid)
                if issue is not None:
                    raise AssertionError(f"valid 4g fixture must have no issue: {issue}")
                if not probe.grx009_segment4g_visual_fallback_ready(valid, True):
                    raise AssertionError("valid 4g fixture must advance readiness")
            else:
                print(
                    "SKIP segment 4g green readiness advance: on-disk artifacts do "
                    "not match offline compile evidence in this environment"
                )
            # Even valid evidence is not ready when segment 4f is not ready.
            if probe.grx009_segment4g_visual_fallback_ready(valid, False):
                raise AssertionError("4g gate must require segment 4f readiness")

            # Red: missing reference frame artifact.
            reference_path = probe.GRX009_VISUAL_REFERENCE_FRAME
            reference_bytes = reference_path.read_bytes()
            reference_path.unlink()
            expect_issue(valid, "reference_frame artifact is missing", "missing reference frame")
            reference_path.write_bytes(reference_bytes)

            # Red: missing candidate frame artifact.
            candidate_path = probe.GRX009_VISUAL_CANDIDATE_FRAME
            candidate_bytes = candidate_path.read_bytes()
            candidate_path.unlink()
            expect_issue(valid, "candidate_frame artifact is missing", "missing candidate frame")
            candidate_path.write_bytes(candidate_bytes)

            # Red: recorded frame hash mismatch.
            hash_tampered = copy.deepcopy(valid)
            hash_tampered["visual"]["reference_frame"]["sha256"] = "0" * 64
            expect_issue(hash_tampered, "sha256 does not match", "frame hash mismatch")
            hash_issue = probe.grx009_segment4g_visual_fallback_issue(hash_tampered)
            current_descriptor_sha = probe.sha256_of_file(probe.GRX009_DESCRIPTOR_LAYOUT)
            if current_descriptor_sha and current_descriptor_sha not in hash_issue:
                raise AssertionError(
                    "frame hash mismatch issue must report the current descriptor hash"
                )
            if "ci/grx009_segment4g_visual_fallback_smoke.py" not in hash_issue:
                raise AssertionError("frame hash mismatch issue must name the 4g smoke command")

            # Red: SKIP evidence is never ready.
            skipped = copy.deepcopy(valid)
            skipped["status"] = "skip"
            expect_issue(skipped, "not success", "skip evidence")

            # Red: placeholder/estimated evidence is never ready.
            placeholder = copy.deepcopy(valid)
            placeholder["visual"]["measured_local"] = False
            expect_issue(placeholder, "measured_local", "placeholder evidence")
            wrong_metric = copy.deepcopy(valid)
            wrong_metric["visual"]["metric_kind"] = "estimated"
            expect_issue(wrong_metric, "metric_kind", "estimated metric kind")

            # Red: telemetry claiming real_gpu_pass or a performance improvement.
            gpu_claim = copy.deepcopy(valid)
            gpu_claim["real_gpu_pass"] = True
            expect_issue(gpu_claim, "real_gpu_pass", "real_gpu_pass claim")
            perf_claim = copy.deepcopy(valid)
            perf_claim["performance_claim"] = "improved"
            expect_issue(perf_claim, "performance", "performance claim")

            # Red: malformed dimensions (below minimum / size mismatch).
            too_small = copy.deepcopy(valid)
            too_small["visual"]["width"] = 32
            expect_issue(too_small, "malformed or below", "below-minimum width")
            size_mismatch = copy.deepcopy(valid)
            size_mismatch["visual"]["width"] = 100
            expect_issue(size_mismatch, "width*height*3", "frame size/dimension mismatch")

            # Red: fallback telemetry without the observed fallback marker.
            no_marker = copy.deepcopy(valid)
            no_marker["fallback_telemetry"]["pass_enable_matrix"]["enabled_fallback"][
                "bridge_fallback_marker_observed"
            ] = False
            expect_issue(no_marker, "bridge fallback marker", "fallback marker missing")

            # Red: runtime log audit — missing section, recorded unexpected
            # error count, an unexpected ERROR line in the recorded stdout,
            # and an allowed-error entry other than the known cache warning.
            no_audit = copy.deepcopy(valid)
            no_audit.pop("runtime_log_audit", None)
            expect_issue(no_audit, "runtime_log_audit", "missing runtime_log_audit")
            dirty_audit = copy.deepcopy(valid)
            dirty_audit["runtime_log_audit"]["candidate"][
                "unexpected_godot_error_count"
            ] = 1
            expect_issue(
                dirty_audit, "unexpected Godot ERROR", "audit unexpected error count"
            )
            bad_stdout = copy.deepcopy(valid)
            bad_stdout["stdout_candidate"] += "ERROR: Unexpected rendering failure.\n"
            expect_issue(
                bad_stdout, "unexpected Godot ERROR line", "unexpected stdout ERROR"
            )
            foreign_allow = copy.deepcopy(valid)
            foreign_allow["runtime_log_audit"]["reference"]["allowed_godot_errors"][0][
                "message"
            ] = "Some other tolerated error"
            expect_issue(foreign_allow, "other", "foreign allowed error message")

            # Red: missing measured capture frame index.
            no_frame_index = copy.deepcopy(valid)
            no_frame_index["visual"].pop("capture_frame_index", None)
            expect_issue(
                no_frame_index, "capture_frame_index", "missing capture_frame_index"
            )

            # Red: telemetry entry mutations. The telemetry document lives on
            # disk and is hash-pinned by the evidence, so mutate the file and
            # the fingerprint coherently, then restore the original bytes.
            telemetry_path = fixture_dir / "measured_fallback_telemetry.json"
            original_telemetry = telemetry_path.read_bytes()

            def fingerprint_file(path: pathlib.Path) -> dict[str, object]:
                return {
                    "path": path.name,
                    "sha256": probe.sha256_of_file(path),
                    "size_bytes": path.stat().st_size,
                }

            def expect_telemetry_issue(
                key: str, value: object, needle: str, label: str
            ) -> None:
                doc = json.loads(original_telemetry.decode("utf-8"))
                doc["passes"][0][key] = value
                write_json(telemetry_path, doc)
                mutated = copy.deepcopy(valid)
                mutated["fallback_telemetry"]["telemetry_document"] = fingerprint_file(
                    telemetry_path
                )
                try:
                    expect_issue(mutated, needle, label)
                finally:
                    telemetry_path.write_bytes(original_telemetry)

            expect_telemetry_issue(
                "telemetry_frame", 7, "stale", "stale telemetry frame"
            )
            expect_telemetry_issue(
                "fallback_reason",
                "compile_failed",
                "validation_failed",
                "wrong fallback_reason",
            )
            expect_telemetry_issue(
                "enable_state", "disabled", "enable_state=enabled", "wrong enable_state"
            )

        with_probe_segment4g_paths(probe, fixture_dir, run_cases)

    # Red: a REAL over-threshold diff (files, hashes, and recorded numbers all
    # coherent) must still fail the pinned threshold.
    with tempfile.TemporaryDirectory() as tmp:
        fixture_dir = pathlib.Path(tmp)
        bright = bytearray(flat_frame)
        bright[0] = min(255, bright[0] + 200)
        over_threshold = build_segment4g_fixture(
            probe,
            fixture_dir,
            reference_bytes=flat_frame,
            candidate_bytes=bytes(bright),
        )
        # Keep the document internally consistent with the harness's honest
        # failure shape flipped to success, so ONLY the threshold check fires.
        over_threshold["visual"]["within_threshold"] = True
        over_threshold["checks"]["diff_within_threshold"] = True

        def run_threshold_case() -> None:
            issue = probe.grx009_segment4g_visual_fallback_issue(over_threshold)
            if issue is None:
                raise AssertionError("over-threshold diff must not pass the 4g gate")
            if "threshold" not in issue:
                raise AssertionError(f"expected a threshold issue, got {issue!r}")
            if probe.grx009_segment4g_visual_fallback_ready(over_threshold, True):
                raise AssertionError("over-threshold diff must NOT advance readiness")

        with_probe_segment4g_paths(probe, fixture_dir, run_threshold_case)

    print("segment 4g visual/fallback probe regression checks passed")


SEGMENT4H_FIXTURE_WIDTH = 64
SEGMENT4H_FIXTURE_HEIGHT = 64


def with_probe_segment4h_paths(
    probe: object, fixture_dir: pathlib.Path, callback: object
) -> object:
    saved = {
        name: getattr(probe, name)
        for name in (
            "GRX009_REAL_PASS_ENABLEMENT_SCHEMA",
            "GRX009_REAL_PASS_ENABLEMENT_TELEMETRY",
            "GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE",
            "GRX009_VISUAL_REAL_PASS_REFERENCE_FRAME",
            "GRX009_VISUAL_REAL_PASS_CANDIDATE_FRAME",
            "GRX009_VISUAL_REAL_PASS_DIFF_ARTIFACT",
        )
    }
    visual_dir = fixture_dir / "artifacts" / "visual"
    probe.GRX009_REAL_PASS_ENABLEMENT_SCHEMA = (
        fixture_dir / "real_pass_enablement_evidence.schema.json"
    )
    probe.GRX009_REAL_PASS_ENABLEMENT_TELEMETRY = (
        fixture_dir / "real_pass_enablement_telemetry.json"
    )
    # Redirect the historical success artifact into the fixture dir so the
    # "missing success evidence" red case stays deterministic even after the
    # real repository records a measured strict success on disk.
    probe.GRX009_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE = (
        fixture_dir / "real_pass_enablement_success_evidence.json"
    )
    probe.GRX009_VISUAL_REAL_PASS_REFERENCE_FRAME = (
        visual_dir / "luminance_real_pass_reference.rgb8"
    )
    probe.GRX009_VISUAL_REAL_PASS_CANDIDATE_FRAME = (
        visual_dir / "luminance_real_pass_candidate.rgb8"
    )
    probe.GRX009_VISUAL_REAL_PASS_DIFF_ARTIFACT = (
        visual_dir / "luminance_real_pass_diff.rgb8"
    )
    try:
        return callback()
    finally:
        for name, value in saved.items():
            setattr(probe, name, value)


def build_segment4h_fixture(
    probe: object,
    fixture_dir: pathlib.Path,
    *,
    reference_bytes: bytes,
    candidate_bytes: bytes,
) -> dict[str, object]:
    """Write frames/telemetry/schema fixture files and return a coherent
    HYPOTHETICAL segment 4h historical measured success document (a strict
    real-pass success is unreachable with the tracked segment 3a artifact by
    design; this fixture exists to red/green the gate's audit itself)."""
    visual_dir = fixture_dir / "artifacts" / "visual"
    visual_dir.mkdir(parents=True, exist_ok=True)
    diff_bytes = bytes(abs(a - b) for a, b in zip(reference_bytes, candidate_bytes))
    max_abs = max(diff_bytes) if diff_bytes else 0
    mean_abs = (sum(diff_bytes) / len(diff_bytes)) if diff_bytes else 0.0
    reference_path = visual_dir / "luminance_real_pass_reference.rgb8"
    candidate_path = visual_dir / "luminance_real_pass_candidate.rgb8"
    diff_path = visual_dir / "luminance_real_pass_diff.rgb8"
    reference_path.write_bytes(reference_bytes)
    candidate_path.write_bytes(candidate_bytes)
    diff_path.write_bytes(diff_bytes)
    (fixture_dir / "real_pass_enablement_evidence.schema.json").write_bytes(
        (REAL_PASS_DIR / "real_pass_enablement_evidence.schema.json").read_bytes()
    )
    telemetry_doc = {
        "run_mode": "full",
        "evidence_level": "measured_local",
        "target_backend": "Godot 4.7-dev Windows D3D12 Forward+",
        "note": "segment 4h regression fixture: forced-failure fallback telemetry",
        "passes": [
            {
                "pass_id": "luminance_reduction",
                "leg": "forced_capability_downgrade",
                "enable_state": "enabled",
                "fallback_reason": "unsupported_device",
                "godot_fallback_active": True,
                "telemetry_timestamp": "2026-07-06T12:00:00+08:00",
                "telemetry_frame": 24,
            }
        ],
    }
    telemetry_path = fixture_dir / "real_pass_enablement_telemetry.json"
    write_json(telemetry_path, telemetry_doc)

    offline = probe.grx009_offline_artifact_digests(probe.grx009_compile_evidence())

    def fingerprint(path: pathlib.Path) -> dict[str, object]:
        return {
            "path": path.name,
            "sha256": probe.sha256_of_file(path),
            "size_bytes": path.stat().st_size,
        }

    patches: list[dict[str, object]] = []
    patch_audit: list[dict[str, object]] = []
    for index, path in enumerate(probe.GRX009_SEGMENT4H_PATCH_STACK_FILES, start=1):
        patch_entry = {
            "patch": path.name,
            "path": str(path.relative_to(probe.ROOT)).replace("\\", "/"),
            "sha256": probe.sha256_of_file(path),
            "size_bytes": path.stat().st_size,
        }
        patches.append(patch_entry)
        patch_audit.append(
            {
                **patch_entry,
                "order": index,
                "commit": f"{index % 10}" * 40,
                "tree": f"{(index + 1) % 10}" * 40,
            }
        )
    final_patch_audit = patch_audit[-1]

    settings_prefix = "rendering/rurix_accel/passes/luminance_reduction"

    def leg(
        role: str,
        *,
        fallback: bool,
        blocked: bool,
        real_pass: bool,
        real_pass_optin: bool,
        downgrade: bool,
    ) -> dict[str, object]:
        return {
            "role": role,
            "project_settings": {
                f"{settings_prefix}/enabled": role != "reference",
                f"{settings_prefix}/dispatch_bringup": role != "reference",
                f"{settings_prefix}/dispatch_real_pass": real_pass_optin,
                f"{settings_prefix}/real_pass_force_capability_downgrade": downgrade,
            },
            "exit_code": 0,
            "session_ready": True,
            "bridge_fallback_marker_observed": fallback,
            "bridge_fallback_marker_line": (
                "RurixAccel: luminance_reduction native resource handle mapping "
                "fallback rc=1; Godot native luminance path remains active."
                if fallback
                else None
            ),
            "real_pass_blocked_marker_observed": blocked,
            "real_pass_blocked_marker_line": (
                "RXGD_REAL_PASS_BLOCKED first_missing_prerequisite="
                "runtime_binding_preflight_failed fallback_reason="
                "unsupported_device kernel_binding=raw_buffer_view "
                "default_enable_state=disabled"
                if blocked
                else None
            ),
            "real_pass_marker_observed": real_pass,
            "record_marker_observed": False,
            "capture_meta": {
                "width": SEGMENT4H_FIXTURE_WIDTH,
                "height": SEGMENT4H_FIXTURE_HEIGHT,
                "format": probe.GRX009_SEGMENT4G_FRAME_FORMAT,
                "capture_frame_index": 24,
            },
            "capture_error": None,
        }

    matrix = {
        "disabled_default": leg(
            "reference",
            fallback=False,
            blocked=False,
            real_pass=False,
            real_pass_optin=False,
            downgrade=False,
        ),
        "enabled_real_pass_optin": leg(
            "enabled_real_pass_optin",
            fallback=False,
            blocked=False,
            real_pass=True,
            real_pass_optin=True,
            downgrade=False,
        ),
        "forced_capability_downgrade": leg(
            "forced_capability_downgrade",
            fallback=True,
            blocked=True,
            real_pass=False,
            real_pass_optin=True,
            downgrade=True,
        ),
    }
    audit_leg = {
        "unexpected_rxgd_diag_count": 0,
        "rxgd_diag_allowed_by_tracked_patch_queue": False,
        "unexpected_godot_error_count": 0,
        "allowed_godot_errors": [
            {
                "message": probe.GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR,
                "observed_count": 1,
                "rationale": (
                    "Tolerated minimal smoke project cache warning after clean "
                    "enablement matrix and exit 0."
                ),
            }
        ],
        "unexpected_lines_tail": [],
    }
    return {
        "schema_version": 1,
        "subject": "grx009_segment4h_luminance_real_pass_enablement_smoke",
        "pass_id": "luminance_reduction",
        "segment": "4h",
        "status": "success",
        "evidence_kind": "historical_measured_success",
        "latest_evidence_path": probe.GRX009_SEGMENT4H_LATEST_EVIDENCE_REL_PATH,
        "runtime_state": "fallback_only",
        "real_gpu_pass": True,
        "real_d3d12_dispatch_recorded": True,
        "real_pass_marker_line": (
            "RXGD_GODOT_RUNTIME_LUMINANCE_REAL_PASS: "
            "pass=RXGD_PASS_LUMINANCE_REDUCTION dispatched=1"
        ),
        "godot_runtime_luminance_path_enabled": False,
        "default_enable_state": "disabled",
        "gpu_timestamp_status": "not_yet",
        "performance_claim": "none",
        "expected_first_missing_prerequisite": (
            probe.GRX009_SEGMENT4H_EXPECTED_FIRST_MISSING_PREREQUISITE
        ),
        "known_gaps": ["fixture"],
        "artifact_hashes_match_offline_evidence": True,
        "artifacts": {
            "dxil": {"sha256": offline["dxil"]},
            "root_signature": {"sha256": offline["root_signature"]},
            "descriptor_layout": {"sha256": offline["descriptor_layout"]},
        },
        "godot_exe_fingerprint": {
            "exe_path_at_run": "H:/scratch/godot.console.exe",
            "exe_sha256": "a" * 64,
            "exe_size_bytes": 123456,
            "committed": False,
        },
        "dll_fingerprint": {
            "dll_path_at_run": "target/debug/rurix_godot.dll",
            "dll_sha256": "b" * 64,
            "dll_size_bytes": 65536,
            "build_profile": "debug",
            "features": ["d3d12-recording-shim"],
        },
        "patch_stack_identity": {
            "patches_dir": "spike/godot-rurix/patches",
            "stack": probe.GRX009_SEGMENT4H_PATCH_STACK_ID,
            "patches": patches,
        },
        "scratch_source_provenance": {
            "base_snapshot": "external/godot-master",
            "base_commit": "1" * 40,
            "base_tree": "2" * 40,
            "final_head": final_patch_audit["commit"],
            "final_tree": final_patch_audit["tree"],
            "actual_head": final_patch_audit["commit"],
            "actual_tree": final_patch_audit["tree"],
            "source_root_at_run": "H:/rurix/target/grx009_segment4h_godot_build/godot",
            "source_clean": True,
            "source_status": [],
            "tracked_patch_stack_only": True,
            "source_audit_supported": True,
            "expected_stack_ok": True,
            "source_audit_errors": [],
            "source_provenance_sidecar_path": (
                "target/grx009_segment4h_godot_build/source_provenance.json"
            ),
            "applied_patch_stack": {
                "patches_dir": "spike/godot-rurix/patches",
                "stack": probe.GRX009_SEGMENT4H_PATCH_STACK_ID,
                "patches": copy.deepcopy(patches),
            },
            "patch_application_audit": copy.deepcopy(patch_audit),
            "godot_exe": {
                "path_at_run": "H:/scratch/godot.console.exe",
                "sha256": "a" * 64,
                "size_bytes": 123456,
                "mtime_utc": "2026-07-06T02:14:33+00:00",
            },
            "build": {
                "available": True,
                "command": (
                    "scons platform=windows target=template_debug "
                    "module_rurix_accel_enabled=yes d3d12=yes"
                ),
                "log_path": "target/grx009_segment4h_godot_build/build.log",
            },
        },
        "checks": {name: True for name in probe.GRX009_SEGMENT4H_REQUIRED_CHECKS},
        "pass_enable_matrix": matrix,
        "visual": {
            "measured_local": True,
            "metric_kind": probe.GRX009_SEGMENT4G_METRIC_KIND,
            "width": SEGMENT4H_FIXTURE_WIDTH,
            "height": SEGMENT4H_FIXTURE_HEIGHT,
            "format": probe.GRX009_SEGMENT4G_FRAME_FORMAT,
            "capture_frame_index": 24,
            "max_abs_diff_threshold": probe.GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD,
            "mean_abs_diff_threshold": probe.GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD,
            "reference_frame": fingerprint(reference_path),
            "candidate_frame": fingerprint(candidate_path),
            "diff_artifact": fingerprint(diff_path),
            "forced_fallback_frame": fingerprint(reference_path),
            "diffs": {
                "candidate": {
                    "max_abs_diff": max_abs,
                    "mean_abs_diff": mean_abs,
                    "within_threshold": (
                        max_abs <= probe.GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD
                        and mean_abs <= probe.GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD
                    ),
                },
                "forced_fallback": {
                    "max_abs_diff": 0,
                    "mean_abs_diff": 0.0,
                    "within_threshold": True,
                },
            },
        },
        "fallback_telemetry": {
            "fallback_path_observed": True,
            "bridge_fallback_marker": probe.GRX009_SEGMENT4H_FALLBACK_MARKER,
            "real_pass_blocked_marker": probe.GRX009_SEGMENT4H_BLOCKED_MARKER,
            "candidate_blocked_marker_line": None,
            "forced_blocked_marker_line": matrix["forced_capability_downgrade"][
                "real_pass_blocked_marker_line"
            ],
            "telemetry_document": fingerprint(telemetry_path),
            "no_fps_claim": True,
        },
        "runtime_log_audit": {
            name: copy.deepcopy(audit_leg)
            for name in ("reference", "candidate", "forced_fallback")
        },
        "stdout_reference": (
            "GRX009Segment4h: scene ready\n"
            "GRX009Segment4h: captured frame=24 width=64 height=64\n"
            "ERROR: Could not load global script cache.\n"
        ),
        "stdout_candidate": (
            "GRX009Segment4h: scene ready\n"
            "RXGD_GODOT_RUNTIME_LUMINANCE_REAL_PASS: "
            "pass=RXGD_PASS_LUMINANCE_REDUCTION dispatched=1\n"
            "GRX009Segment4h: captured frame=24 width=64 height=64\n"
            "ERROR: Could not load global script cache.\n"
        ),
        "stdout_forced_fallback": (
            "GRX009Segment4h: scene ready\n"
            "RXGD_REAL_PASS_BLOCKED first_missing_prerequisite="
            "runtime_binding_preflight_failed fallback_reason=unsupported_device "
            "kernel_binding=raw_buffer_view default_enable_state=disabled\n"
            "GRX009Segment4h: captured frame=24 width=64 height=64\n"
            "ERROR: Could not load global script cache.\n"
        ),
    }


def run_segment4h_real_pass_enablement_cases() -> None:
    """Red/green coverage for the segment 4h real-pass enablement gate."""
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe
    from ci import grx009_segment4h_real_pass_enablement_smoke as smoke4h

    # The probe's pinned gate constants must match the harness pins.
    if probe.GRX009_SEGMENT4G_METRIC_KIND != smoke4h.METRIC_KIND:
        raise AssertionError("probe/4h harness metric_kind pins disagree")
    if probe.GRX009_SEGMENT4G_FRAME_FORMAT != smoke4h.FRAME_FORMAT:
        raise AssertionError("probe/4h harness frame format pins disagree")
    if probe.GRX009_SEGMENT4G_MAX_ABS_DIFF_THRESHOLD != smoke4h.LDR_MAX_ABS_DIFF_THRESHOLD:
        raise AssertionError("probe/4h harness max_abs_diff threshold pins disagree")
    if probe.GRX009_SEGMENT4G_MEAN_ABS_DIFF_THRESHOLD != smoke4h.LDR_MEAN_ABS_DIFF_THRESHOLD:
        raise AssertionError("probe/4h harness mean_abs_diff threshold pins disagree")
    if probe.GRX009_SEGMENT4G_MIN_FRAME_DIMENSION != smoke4h.MIN_FRAME_DIMENSION:
        raise AssertionError("probe/4h harness minimum frame dimension pins disagree")
    if probe.GRX009_SEGMENT4G_ALLOWED_GODOT_ERROR != smoke4h.ALLOWED_GODOT_ERROR:
        raise AssertionError("probe/4h harness allowed Godot error pins disagree")
    if (
        probe.GRX009_SEGMENT4H_EXPECTED_FIRST_MISSING_PREREQUISITE
        != smoke4h.EXPECTED_FIRST_MISSING_PREREQUISITE
    ):
        raise AssertionError("probe/4h harness first-missing-prerequisite pins disagree")
    if probe.GRX009_SEGMENT4H_FALLBACK_MARKER != smoke4h.FALLBACK_MARKER:
        raise AssertionError("probe/4h harness fallback marker pins disagree")
    if probe.GRX009_SEGMENT4H_BLOCKED_MARKER != smoke4h.REAL_PASS_BLOCKED_MARKER:
        raise AssertionError("probe/4h harness blocked marker pins disagree")
    if probe.GRX009_SEGMENT4H_REAL_PASS_MARKER != smoke4h.REAL_PASS_MARKER:
        raise AssertionError("probe/4h harness real-pass marker pins disagree")
    if probe.GRX009_SEGMENT4H_PATCH_STACK_ID != smoke4h.PATCH_STACK_ID:
        raise AssertionError("probe/4h harness patch stack id pins disagree")
    probe_stack_names = tuple(
        path.name for path in probe.GRX009_SEGMENT4H_PATCH_STACK_FILES
    )
    if probe_stack_names != smoke4h.PATCH_STACK_4H:
        raise AssertionError("probe/4h harness patch stack file pins disagree")

    latest = load_json(REAL_PASS_ENABLEMENT_EVIDENCE_PATH)
    latest_issue = probe.grx009_segment4h_latest_evidence_hash_chain_issue(latest)
    if latest_issue is not None:
        raise AssertionError(f"current latest 4h evidence hash-chain must pass: {latest_issue}")
    stale_latest = copy.deepcopy(latest)
    artifacts = stale_latest.get("artifacts")
    offline_evidence = stale_latest.get("offline_evidence")
    if not isinstance(artifacts, dict) or not isinstance(offline_evidence, dict):
        raise AssertionError("latest 4h evidence fixture must carry artifacts and offline_evidence")
    dxil_entry = artifacts.get("dxil")
    if not isinstance(dxil_entry, dict):
        raise AssertionError("latest 4h evidence fixture must carry artifacts.dxil")
    stale_latest["status"] = "skip"
    stale_latest["skip_kind"] = "environment"
    stale_latest["artifact_hashes_match_offline_evidence"] = True
    dxil_entry["sha256"] = STALE_SEGMENT4H_DXIL_SHA256
    offline_evidence["dxil_sha256"] = STALE_SEGMENT4H_DXIL_SHA256
    stale_issue = probe.grx009_segment4h_latest_evidence_hash_chain_issue(stale_latest)
    if stale_issue is None:
        raise AssertionError("stale latest 4h evidence hash-chain must be rejected")
    if "dxil" not in stale_issue and "hash" not in stale_issue:
        raise AssertionError(f"stale latest 4h evidence issue should name hash drift, got {stale_issue!r}")

    pixel_count = SEGMENT4H_FIXTURE_WIDTH * SEGMENT4H_FIXTURE_HEIGHT
    flat_frame = bytes((120, 130, 140)) * pixel_count

    def expect_issue(evidence: dict[str, object], needle: str, label: str) -> None:
        issue = probe.grx009_segment4h_real_pass_enablement_issue(evidence)
        if issue is None:
            raise AssertionError(f"{label}: expected the 4h gate to report an issue")
        if needle not in issue:
            raise AssertionError(
                f"{label}: expected issue containing {needle!r}, got {issue!r}"
            )
        if probe.grx009_segment4h_real_pass_enablement_ready(evidence, True):
            raise AssertionError(f"{label}: gate must NOT be ready")

    with tempfile.TemporaryDirectory() as tmp:
        fixture_dir = pathlib.Path(tmp)
        valid = build_segment4h_fixture(
            probe,
            fixture_dir,
            reference_bytes=flat_frame,
            candidate_bytes=flat_frame,
        )

        def run_cases() -> None:
            # Green: the (hypothetical) coherent strict success advances only
            # when the tracked on-disk artifacts still match offline evidence.
            offline = probe.grx009_offline_artifact_digests(probe.grx009_compile_evidence())
            current = {
                "dxil": probe.sha256_of_file(probe.GRX009_DXIL_ARTIFACT),
                "root_signature": probe.sha256_of_file(probe.GRX009_ROOT_SIGNATURE_ARTIFACT),
                "descriptor_layout": probe.sha256_of_file(probe.GRX009_DESCRIPTOR_LAYOUT),
            }
            artifacts_match = (
                all(v is not None for v in current.values()) and current == offline
            )
            if artifacts_match:
                issue = probe.grx009_segment4h_real_pass_enablement_issue(valid)
                if issue is not None:
                    raise AssertionError(f"valid 4h fixture must have no issue: {issue}")
                if not probe.grx009_segment4h_real_pass_enablement_ready(valid, True):
                    raise AssertionError("valid 4h fixture must advance readiness")
            else:
                print(
                    "SKIP segment 4h green readiness advance: on-disk artifacts do "
                    "not match offline compile evidence in this environment"
                )
            # Even valid evidence is not ready when segment 4g is not ready.
            if probe.grx009_segment4h_real_pass_enablement_ready(valid, False):
                raise AssertionError("4h gate must require segment 4g readiness")

            # Red: missing/None success evidence never advances and names the
            # designed fail-closed state.
            missing_issue = probe.grx009_segment4h_real_pass_enablement_issue(None)
            if missing_issue is None or "runtime-mappable" not in missing_issue:
                raise AssertionError(
                    "missing 4h success evidence must name the runtime-mappable "
                    f"kernel prerequisite, got {missing_issue!r}"
                )

            # Red: SKIP evidence (including measured_prerequisite_blocked) is
            # never ready.
            skipped = copy.deepcopy(valid)
            skipped["status"] = "skip"
            skipped["skip_kind"] = "measured_prerequisite_blocked"
            expect_issue(skipped, "not success", "skip evidence")

            # Red: a success claim without the real-pass proof fields.
            no_gpu_pass = copy.deepcopy(valid)
            no_gpu_pass["real_gpu_pass"] = False
            expect_issue(no_gpu_pass, "real_gpu_pass=true", "real_gpu_pass false")
            no_marker_line = copy.deepcopy(valid)
            no_marker_line.pop("real_pass_marker_line", None)
            expect_issue(no_marker_line, "marker line", "missing real-pass marker line")

            # Red: performance claim / default enablement drift.
            perf_claim = copy.deepcopy(valid)
            perf_claim["performance_claim"] = "improved"
            expect_issue(perf_claim, "performance", "performance claim")
            default_flip = copy.deepcopy(valid)
            default_flip["default_enable_state"] = "enabled"
            expect_issue(default_flip, "default_enable_state", "default enable flip")

            # Red: contradictory candidate leg (blocked + success claim).
            contradictory = copy.deepcopy(valid)
            contradictory["pass_enable_matrix"]["enabled_real_pass_optin"][
                "real_pass_blocked_marker_observed"
            ] = True
            expect_issue(contradictory, "contradictory", "blocked + success")

            # Red: forced-failure red leg not measured.
            no_forced = copy.deepcopy(valid)
            no_forced["pass_enable_matrix"]["forced_capability_downgrade"][
                "bridge_fallback_marker_observed"
            ] = False
            expect_issue(no_forced, "forced-failure red leg", "forced leg missing")

            # Red: recorded frame hash mismatch.
            hash_tampered = copy.deepcopy(valid)
            hash_tampered["visual"]["candidate_frame"]["sha256"] = "0" * 64
            expect_issue(hash_tampered, "sha256 does not match", "frame hash mismatch")

            # Red: checks not all green.
            bad_checks = copy.deepcopy(valid)
            bad_checks["checks"]["real_pass_dispatched_and_completed"] = False
            expect_issue(bad_checks, "checks", "checks not green")

            # Red: patch stack identity tamper (9-stack).
            stack_tampered = copy.deepcopy(valid)
            stack_tampered["patch_stack_identity"]["patches"][8]["sha256"] = "0" * 64
            expect_issue(stack_tampered, "patch stack", "patch stack tamper")

            # Red: dirty scratch source provenance.
            dirty_source = copy.deepcopy(valid)
            dirty_source["scratch_source_provenance"]["source_clean"] = False
            expect_issue(dirty_source, "provenance", "dirty provenance")

            # Red: runtime log audit — missing section and unexpected stdout
            # ERROR in the forced leg.
            no_audit = copy.deepcopy(valid)
            no_audit.pop("runtime_log_audit", None)
            expect_issue(no_audit, "runtime_log_audit", "missing runtime_log_audit")
            bad_stdout = copy.deepcopy(valid)
            bad_stdout["stdout_forced_fallback"] += "ERROR: Unexpected rendering failure.\n"
            expect_issue(
                bad_stdout, "unexpected Godot ERROR line", "unexpected stdout ERROR"
            )

            # Red: telemetry entry mutations (doc on disk, hash-pinned).
            telemetry_path = fixture_dir / "real_pass_enablement_telemetry.json"
            original_telemetry = telemetry_path.read_bytes()

            def fingerprint_file(path: pathlib.Path) -> dict[str, object]:
                return {
                    "path": path.name,
                    "sha256": probe.sha256_of_file(path),
                    "size_bytes": path.stat().st_size,
                }

            def expect_telemetry_issue(
                mutate: object, needle: str, label: str
            ) -> None:
                doc = json.loads(original_telemetry.decode("utf-8"))
                mutate(doc)
                write_json(telemetry_path, doc)
                mutated = copy.deepcopy(valid)
                mutated["fallback_telemetry"]["telemetry_document"] = fingerprint_file(
                    telemetry_path
                )
                try:
                    expect_issue(mutated, needle, label)
                finally:
                    telemetry_path.write_bytes(original_telemetry)

            expect_telemetry_issue(
                lambda doc: doc["passes"][0].__setitem__("telemetry_frame", 7),
                "stale",
                "stale forced telemetry frame",
            )
            expect_telemetry_issue(
                lambda doc: doc["passes"][0].__setitem__(
                    "fallback_reason", "validation_failed"
                ),
                "unsupported_device",
                "wrong forced fallback_reason",
            )
            expect_telemetry_issue(
                lambda doc: doc["passes"].append(
                    {
                        "pass_id": "luminance_reduction",
                        "leg": "enabled_real_pass_optin",
                        "enable_state": "enabled",
                        "fallback_reason": "validation_failed",
                        "godot_fallback_active": True,
                        "telemetry_timestamp": "2026-07-06T12:00:00+08:00",
                        "telemetry_frame": 24,
                    }
                ),
                "contradictory",
                "candidate fallback entry on success",
            )

        with_probe_segment4h_paths(probe, fixture_dir, run_cases)

    # Red: a REAL over-threshold candidate diff must still fail the pinned
    # threshold even when files, hashes, and recorded numbers are coherent.
    with tempfile.TemporaryDirectory() as tmp:
        fixture_dir = pathlib.Path(tmp)
        bright = bytearray(flat_frame)
        bright[0] = min(255, bright[0] + 200)
        over_threshold = build_segment4h_fixture(
            probe,
            fixture_dir,
            reference_bytes=flat_frame,
            candidate_bytes=bytes(bright),
        )
        over_threshold["visual"]["diffs"]["candidate"]["within_threshold"] = True
        over_threshold["checks"]["diff_within_threshold_candidate"] = True

        def run_threshold_case() -> None:
            issue = probe.grx009_segment4h_real_pass_enablement_issue(over_threshold)
            if issue is None:
                raise AssertionError("over-threshold diff must not pass the 4h gate")
            if "threshold" not in issue:
                raise AssertionError(f"expected a threshold issue, got {issue!r}")
            if probe.grx009_segment4h_real_pass_enablement_ready(over_threshold, True):
                raise AssertionError("over-threshold diff must NOT advance readiness")

        with_probe_segment4h_paths(probe, fixture_dir, run_threshold_case)

    print("segment 4h real-pass enablement probe regression checks passed")


def run_patch_stack_git_config_order_case() -> None:
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_patch_stack as patch_stack

    recorded: list[list[str]] = []
    original_run_capture = patch_stack.run_capture

    def fake_run_capture(root: pathlib.Path, cmd: list[str]) -> subprocess.CompletedProcess[str]:
        recorded.append(cmd)
        return subprocess.CompletedProcess(cmd, 0, "", "")

    try:
        patch_stack.run_capture = fake_run_capture
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            external = root / "external" / "godot-master"
            external.mkdir(parents=True)
            prereq = root / "0004.patch"
            patch = root / "0005.patch"
            prereq.write_text(
                "diff --git a/modules/rurix_accel/rurix_accel.cpp b/modules/rurix_accel/rurix_accel.cpp\n",
                encoding="utf-8",
            )
            patch.write_text(
                "diff --git a/drivers/d3d12/d3d12_hooks.h b/drivers/d3d12/d3d12_hooks.h\n",
                encoding="utf-8",
            )
            result = patch_stack.evaluate_stacked_patch_applyability(
                root,
                external,
                [prereq],
                patch,
                "0005",
            )
    finally:
        patch_stack.run_capture = original_run_capture

    expected = [
        ["git", "-c", "core.autocrlf=false", "init", "--quiet", "."],
        ["git", "-c", "core.autocrlf=false", "apply", str(prereq.resolve())],
        ["git", "-c", "core.autocrlf=false", "apply", "--check", str(patch.resolve())],
    ]
    if recorded != expected:
        raise AssertionError(f"unexpected git command order: {recorded}")
    details = result.get("details")
    if not isinstance(details, dict):
        raise AssertionError(f"expected patch-stack details: {result}")
    if details.get("0005_stack_touched_paths") != [
        "modules/rurix_accel/rurix_accel.cpp",
        "drivers/d3d12/d3d12_hooks.h",
    ]:
        raise AssertionError(f"unexpected touched paths: {details}")
    if details.get("0005_stacked_check_exit_code") != 0:
        raise AssertionError(f"expected stacked check exit code in details: {details}")


def make_valid_segment4f_success_evidence(probe: object) -> dict[str, object]:
    """Build a well-formed historical measured success artifact whose audit
    provenance matches the current patch stack and offline artifact digests."""
    offline = probe.grx009_offline_artifact_digests(probe.grx009_compile_evidence())
    patches: list[dict[str, object]] = []
    patch_audit: list[dict[str, object]] = []
    for index, path in enumerate(probe.GRX009_SEGMENT4F_PATCH_STACK_FILES, start=1):
        patch_entry = {
            "patch": path.name,
            "path": str(path.relative_to(probe.ROOT)).replace("\\", "/"),
            "sha256": probe.sha256_of_file(path),
            "size_bytes": path.stat().st_size,
        }
        patches.append(patch_entry)
        patch_audit.append(
            {
                **patch_entry,
                "order": index,
                "commit": f"{index}" * 40,
                "tree": f"{index + 1}" * 40,
            }
        )
    final_patch_audit = patch_audit[-1]
    return {
        "schema_version": 1,
        "subject": "grx009_godot_runtime_luminance_bridge_dispatch_recording_smoke",
        "pass_id": "luminance_reduction",
        "segment": "4f",
        "status": "success",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "godot_runtime_bridge_recorded_dispatch": True,
        "godot_runtime_luminance_path_enabled": False,
        "default_enable_state": "disabled",
        "gpu_timestamp_status": "not_yet",
        "artifact_hashes_match_offline_evidence": True,
        "evidence_kind": "historical_measured_success",
        "latest_evidence_path": probe.GRX009_SEGMENT4F_LATEST_EVIDENCE_REL_PATH,
        "godot_exe_fingerprint": {
            "exe_path_at_run": "H:/scratch/godot.console.exe",
            "exe_sha256": "a" * 64,
            "exe_size_bytes": 123456,
            "committed": False,
        },
        "dll_fingerprint": {
            "dll_path_at_run": "target/debug/rurix_godot.dll",
            "dll_sha256": "b" * 64,
            "dll_size_bytes": 65536,
            "build_profile": "debug",
            "features": ["d3d12-recording-shim"],
            "snapshot_dll_sha256": "b" * 64,
        },
        "patch_stack_identity": {
            "patches_dir": "spike/godot-rurix/patches",
            "stack": "0001..0008",
            "patches": patches,
        },
        "scratch_source_provenance": {
            "base_snapshot": "external/godot-master",
            "base_commit": "1" * 40,
            "base_tree": "2" * 40,
            "final_head": final_patch_audit["commit"],
            "final_tree": final_patch_audit["tree"],
            "actual_head": final_patch_audit["commit"],
            "actual_tree": final_patch_audit["tree"],
            "source_root_at_run": "H:/rurix/target/grx009_segment4f_godot_build/godot",
            "source_clean": True,
            "source_status": [],
            "tracked_patch_stack_only": True,
            "source_audit_supported": True,
            "expected_stack_ok": True,
            "source_audit_errors": [],
            "source_provenance_sidecar_path": "target/grx009_segment4f_godot_build/source_provenance.json",
            "applied_patch_stack": {
                "patches_dir": "spike/godot-rurix/patches",
                "stack": "0001..0008",
                "patches": copy.deepcopy(patches),
            },
            "patch_application_audit": copy.deepcopy(patch_audit),
            "godot_exe": {
                "path_at_run": "H:/scratch/godot.console.exe",
                "sha256": "a" * 64,
                "size_bytes": 123456,
                "mtime_utc": "2026-07-05T02:14:33+00:00",
            },
            "build": {
                "available": True,
                "command": "scons platform=windows target=template_debug module_rurix_accel_enabled=yes d3d12=yes",
                "log_path": "target/grx009_segment4f_godot_build/build.log",
            },
        },
        "runtime_log_audit": {
            "unexpected_rxgd_diag_count": 0,
            "rxgd_diag_allowed_by_tracked_patch_queue": False,
            "unexpected_godot_error_count": 0,
            "allowed_godot_errors": [
                {
                    "message": "Could not load global script cache",
                    "observed_count": 1,
                    "rationale": "Tolerated minimal smoke project cache warning after clean recording and exit 0.",
                }
            ],
            "unexpected_lines_tail": [],
        },
        "artifacts": {
            "dxil": {"sha256": offline["dxil"]},
            "root_signature": {"sha256": offline["root_signature"]},
            "descriptor_layout": {"sha256": offline["descriptor_layout"]},
        },
        "checks": {
            "artifact_hashes_match_offline_evidence": True,
            "descriptor_layout_matches_resource_mapping": True,
            "recording_shim_linked": True,
            "godot_runtime_session_ready": True,
            "godot_runtime_call_site_recorded": True,
            "recorded_one_pass": True,
            "godot_exit_code_zero": True,
        },
        "recording": {"runtime_record_marker": "RXGD_GODOT_RUNTIME_LUMINANCE_RECORD", "recorded": "1"},
        "stdout": (
            "GRX009Segment4f: scene ready\n"
            "RXGD_GODOT_RUNTIME_LUMINANCE_RECORD: pass=RXGD_PASS_LUMINANCE_REDUCTION recorded=1\n"
            "ERROR: Could not load global script cache.\n"
        ),
    }


def run_segment4f_success_audit_cases() -> None:
    """Red/green coverage for the segment 4f historical-success audit gate."""
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    valid = make_valid_segment4f_success_evidence(probe)

    # Green: a fully-populated, hash-matched audit provenance passes.
    if not probe.grx009_segment4f_success_audit_ok(valid):
        raise AssertionError("valid segment 4f success audit provenance must pass")

    # Red: an old-style success JSON missing the audit fields must not pass.
    legacy = copy.deepcopy(valid)
    for key in (
        "evidence_kind",
        "latest_evidence_path",
        "godot_exe_fingerprint",
        "dll_fingerprint",
        "patch_stack_identity",
        "scratch_source_provenance",
        "runtime_log_audit",
    ):
        legacy.pop(key, None)
    if probe.grx009_segment4f_success_audit_ok(legacy):
        raise AssertionError("legacy success JSON missing audit fields must NOT pass audit")

    missing_provenance = copy.deepcopy(valid)
    missing_provenance.pop("scratch_source_provenance", None)
    if probe.grx009_segment4f_success_audit_ok(missing_provenance):
        raise AssertionError("missing scratch_source_provenance must NOT pass audit")

    missing_log_audit = copy.deepcopy(valid)
    missing_log_audit.pop("runtime_log_audit", None)
    if probe.grx009_segment4f_success_audit_ok(missing_log_audit):
        raise AssertionError("missing runtime_log_audit must NOT pass audit")

    dirty_source = copy.deepcopy(valid)
    dirty_source["scratch_source_provenance"]["source_clean"] = False
    if probe.grx009_segment4f_success_audit_ok(dirty_source):
        raise AssertionError("dirty scratch source provenance must NOT pass audit")

    extra_source_delta = copy.deepcopy(valid)
    extra_source_delta["scratch_source_provenance"]["tracked_patch_stack_only"] = False
    if probe.grx009_segment4f_success_audit_ok(extra_source_delta):
        raise AssertionError("non tracked-patch-stack-only provenance must NOT pass audit")

    extra_local_commit = copy.deepcopy(valid)
    extra_local_commit["scratch_source_provenance"]["actual_tree"] = "c" * 40
    if probe.grx009_segment4f_success_audit_ok(extra_local_commit):
        raise AssertionError("clean source with extra local commit/tree must NOT pass audit")

    clean_only_provenance = copy.deepcopy(valid)
    for key in (
        "source_audit_supported",
        "base_commit",
        "base_tree",
        "final_head",
        "final_tree",
        "actual_head",
        "actual_tree",
        "patch_application_audit",
    ):
        clean_only_provenance["scratch_source_provenance"].pop(key, None)
    if probe.grx009_segment4f_success_audit_ok(clean_only_provenance):
        raise AssertionError("clean-only provenance without source/tree/patch audit must NOT pass audit")

    missing_patch_audit = copy.deepcopy(valid)
    missing_patch_audit["scratch_source_provenance"].pop("patch_application_audit", None)
    if probe.grx009_segment4f_success_audit_ok(missing_patch_audit):
        raise AssertionError("missing patch_application_audit must NOT pass audit")

    forged_patch_audit = copy.deepcopy(valid)
    forged_patch_audit["scratch_source_provenance"]["patch_application_audit"] = forged_patch_audit["scratch_source_provenance"]["patch_application_audit"][:7]
    if probe.grx009_segment4f_success_audit_ok(forged_patch_audit):
        raise AssertionError("forged/incomplete patch_application_audit must NOT pass audit")

    final_tree_mismatch = copy.deepcopy(valid)
    final_tree_mismatch["scratch_source_provenance"]["final_tree"] = "d" * 40
    final_tree_mismatch["scratch_source_provenance"]["actual_tree"] = "d" * 40
    if probe.grx009_segment4f_success_audit_ok(final_tree_mismatch):
        raise AssertionError("final_tree mismatch with last patch audit tree must NOT pass audit")

    final_head_mismatch = copy.deepcopy(valid)
    final_head_mismatch["scratch_source_provenance"]["final_head"] = "e" * 40
    final_head_mismatch["scratch_source_provenance"]["actual_head"] = "e" * 40
    if probe.grx009_segment4f_success_audit_ok(final_head_mismatch):
        raise AssertionError("final_head mismatch with last patch audit commit must NOT pass audit")

    expected_stack_not_ok = copy.deepcopy(valid)
    expected_stack_not_ok["scratch_source_provenance"]["expected_stack_ok"] = False
    if probe.grx009_segment4f_success_audit_ok(expected_stack_not_ok):
        raise AssertionError("expected_stack_ok=false provenance must NOT pass audit")

    untracked_diag = copy.deepcopy(valid)
    untracked_diag["stdout"] += "RXGD_DIAG callsite rd=1\n"
    if probe.grx009_segment4f_success_audit_ok(untracked_diag):
        raise AssertionError("untracked RXGD_DIAG stdout must NOT pass audit")

    unexpected_error = copy.deepcopy(valid)
    unexpected_error["stdout"] += "ERROR: Unexpected rendering failure.\n"
    if probe.grx009_segment4f_success_audit_ok(unexpected_error):
        raise AssertionError("unexpected Godot ERROR stdout must NOT pass audit")

    # Red: a patch-hash mismatch must not pass.
    hash_tampered = copy.deepcopy(valid)
    hash_tampered["patch_stack_identity"]["patches"][0]["sha256"] = "0" * 64
    if probe.grx009_segment4f_success_audit_ok(hash_tampered):
        raise AssertionError("patch sha256 mismatch must NOT pass audit")

    source_hash_tampered = copy.deepcopy(valid)
    source_hash_tampered["scratch_source_provenance"]["patch_application_audit"][0]["sha256"] = "0" * 64
    if probe.grx009_segment4f_success_audit_ok(source_hash_tampered):
        raise AssertionError("source audit patch sha256 mismatch must NOT pass audit")

    # Red: a patch-size mismatch must not pass.
    size_tampered = copy.deepcopy(valid)
    size_tampered["patch_stack_identity"]["patches"][0]["size_bytes"] = 1
    if probe.grx009_segment4f_success_audit_ok(size_tampered):
        raise AssertionError("patch size mismatch must NOT pass audit")

    # Red: a committed scratch exe fingerprint must not pass.
    committed_exe = copy.deepcopy(valid)
    committed_exe["godot_exe_fingerprint"]["committed"] = True
    if probe.grx009_segment4f_success_audit_ok(committed_exe):
        raise AssertionError("committed exe fingerprint must NOT pass audit")

    # Red: an empty exe sha256 / zero exe size must not pass.
    empty_exe = copy.deepcopy(valid)
    empty_exe["godot_exe_fingerprint"]["exe_sha256"] = ""
    if probe.grx009_segment4f_success_audit_ok(empty_exe):
        raise AssertionError("empty exe sha256 must NOT pass audit")
    zero_exe = copy.deepcopy(valid)
    zero_exe["godot_exe_fingerprint"]["exe_size_bytes"] = 0
    if probe.grx009_segment4f_success_audit_ok(zero_exe):
        raise AssertionError("zero exe size must NOT pass audit")

    # Red: a recording-shim DLL without the feature must not pass.
    no_feature = copy.deepcopy(valid)
    no_feature["dll_fingerprint"]["features"] = ["default"]
    if probe.grx009_segment4f_success_audit_ok(no_feature):
        raise AssertionError("dll without d3d12-recording-shim feature must NOT pass audit")

    # Red: a wrong latest_evidence_path must not pass.
    wrong_latest = copy.deepcopy(valid)
    wrong_latest["latest_evidence_path"] = "spike/other/evidence.json"
    if probe.grx009_segment4f_success_audit_ok(wrong_latest):
        raise AssertionError("wrong latest_evidence_path must NOT pass audit")

    # Readiness gate: neutralize the harness/inputs preflight so the test
    # exercises the success-artifact gate, and pass segment4e_ready=True.
    saved_inputs = probe.grx009_segment4f_inputs_ready
    probe.grx009_segment4f_inputs_ready = lambda: True
    try:
        # Red: legacy / tampered success evidence must not advance readiness.
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(legacy, True):
            raise AssertionError(
                "legacy success evidence must NOT advance segment 4f readiness"
            )
        legacy_issue = probe.grx009_segment4f_godot_runtime_bridge_recording_issue(
            legacy, True
        )
        if not legacy_issue:
            raise AssertionError(
                "legacy success evidence must report a non-empty segment 4f issue"
            )
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(hash_tampered, True):
            raise AssertionError(
                "patch-hash-mismatch success evidence must NOT advance segment 4f readiness"
            )
        hash_issue = probe.grx009_segment4f_godot_runtime_bridge_recording_issue(
            hash_tampered, True
        )
        if not hash_issue:
            raise AssertionError(
                "patch-hash-mismatch success evidence must report a non-empty segment 4f issue"
            )
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(untracked_diag, True):
            raise AssertionError(
                "untracked RXGD_DIAG success evidence must NOT advance segment 4f readiness"
            )
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(dirty_source, True):
            raise AssertionError(
                "dirty scratch provenance must NOT advance segment 4f readiness"
            )
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(extra_local_commit, True):
            raise AssertionError(
                "clean source with extra local commit must NOT advance segment 4f readiness"
            )
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(missing_patch_audit, True):
            raise AssertionError(
                "missing source patch audit must NOT advance segment 4f readiness"
            )
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(source_hash_tampered, True):
            raise AssertionError(
                "source audit patch-hash mismatch must NOT advance segment 4f readiness"
            )
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(final_tree_mismatch, True):
            raise AssertionError(
                "final_tree mismatch with last patch audit tree must NOT advance segment 4f readiness"
            )
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(final_head_mismatch, True):
            raise AssertionError(
                "final_head mismatch with last patch audit commit must NOT advance segment 4f readiness"
            )
        if probe.grx009_segment4f_godot_runtime_bridge_recording_ready(expected_stack_not_ok, True):
            raise AssertionError(
                "expected_stack_ok=false provenance must NOT advance segment 4f readiness"
            )

        # The readiness gate re-verifies on-disk artifacts against the offline
        # evidence; only assert the green advance when they match in this env.
        offline = probe.grx009_offline_artifact_digests(probe.grx009_compile_evidence())
        current = {
            "dxil": probe.sha256_of_file(probe.GRX009_DXIL_ARTIFACT),
            "root_signature": probe.sha256_of_file(probe.GRX009_ROOT_SIGNATURE_ARTIFACT),
            "descriptor_layout": probe.sha256_of_file(probe.GRX009_DESCRIPTOR_LAYOUT),
        }
        artifacts_match = all(v is not None for v in current.values()) and current == offline
        if artifacts_match:
            if not probe.grx009_segment4f_godot_runtime_bridge_recording_ready(valid, True):
                raise AssertionError(
                    "valid historical success fixture with matching audit fields "
                    "must advance segment 4f readiness"
                )
        else:
            print(
                "SKIP segment 4f green readiness advance: on-disk artifacts do not "
                "match offline compile evidence in this environment"
            )
    finally:
        probe.grx009_segment4f_inputs_ready = saved_inputs

    print("segment 4f success-audit probe regression checks passed")


def run_grx010_tonemap_gate_cases() -> None:
    """GRX-010 tonemap gate red/green regression (kept to one key pair).

    Green: the tracked tonemap pass dir passes both the contract gate and the
    standalone dispatch smoke gate audits. Red: a fixture copy with a tampered
    offline artifact digest must be rejected with
    grx010_offline_artifact_hash_mismatch, and a fixture whose dispatch smoke
    evidence is not a strict success must not advance the smoke gate.
    """
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    real_pass_dir = ROOT / "spike" / "godot-rurix" / "passes" / "tonemap"

    # Green: the real tracked pass dir audits clean.
    issue = probe.grx010_tonemap_contract_issue(pass_dir=real_pass_dir)
    if issue is not None:
        raise AssertionError(f"expected GRX-010 contract gate green, got {issue!r}")
    smoke_issue = probe.grx010_tonemap_d3d12_dispatch_smoke_issue(pass_dir=real_pass_dir)
    if smoke_issue is not None:
        raise AssertionError(
            f"expected GRX-010 dispatch smoke gate green, got {smoke_issue!r}"
        )

    # Red: tampered offline artifact digest + non-success smoke status.
    with tempfile.TemporaryDirectory() as tmp:
        fixture = pathlib.Path(tmp) / "tonemap"
        shutil.copytree(real_pass_dir, fixture)

        evidence_path = fixture / "offline_compile_evidence.json"
        evidence = load_json(evidence_path)
        dxil_entry = evidence["artifacts"]["dxil"]
        dxil_entry["sha256"] = "0" * 64
        write_json(evidence_path, evidence)
        issue = probe.grx010_tonemap_contract_issue(pass_dir=fixture)
        if issue != "grx010_offline_artifact_hash_mismatch":
            raise AssertionError(
                "expected grx010_offline_artifact_hash_mismatch for a tampered "
                f"offline evidence digest, got {issue!r}"
            )

        smoke_path = fixture / "real_d3d12_dispatch_smoke.json"
        smoke = load_json(smoke_path)
        smoke["status"] = "skip"
        write_json(smoke_path, smoke)
        smoke_issue = probe.grx010_tonemap_d3d12_dispatch_smoke_issue(pass_dir=fixture)
        if smoke_issue != "grx010_dispatch_smoke_status_skip":
            raise AssertionError(
                "expected grx010_dispatch_smoke_status_skip for a SKIP smoke "
                f"evidence, got {smoke_issue!r}"
            )
    print("grx010 tonemap gate red/green cases passed")


def run_grx010_tonemap_close_out_cases() -> None:
    """GRX-010 tonemap stage-A5-equivalent close-out red/green regression.

    Mirrors the GRX-009 segment 4h / 4m coverage: the real close-out state is
    green (strict measured success active + owner default-enable decision
    ready); a pre-close-out fixture manifest (false-valued) stays acceptable;
    a tampered success artifact is rejected by the strict audit, drives a
    conflict, and makes the fail-closed manifest _ok helpers reject the new
    true-valued manifest; a SKIP success document never advances the gate."""
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    real_pass_dir = ROOT / "spike" / "godot-rurix" / "passes" / "tonemap"
    real_manifest_path = real_pass_dir / "pass_manifest.json"
    real_success_path = real_pass_dir / "real_pass_enablement_success_evidence.json"
    real_manifest_bytes = real_manifest_path.read_bytes()
    real_success_bytes = real_success_path.read_bytes()

    # Green: the real close-out state advances every gate.
    if not probe.grx010_real_pass_measured_success_active():
        raise AssertionError(
            "expected the real tonemap real-pass measured success to be active"
        )
    if probe.grx010_real_pass_enablement_issue() is not None:
        raise AssertionError(
            "expected the real tonemap real-pass enablement audit green, got "
            f"{probe.grx010_real_pass_enablement_issue()!r}"
        )
    if not probe.grx010_real_pass_default_enable_decision_ready():
        raise AssertionError(
            "expected the real tonemap owner default-enable decision gate ready, "
            f"got issue {probe.grx010_real_pass_default_enable_decision_issue()!r}"
        )
    if probe.grx010_real_pass_success_evidence_conflict():
        raise AssertionError("real success evidence must not report a conflict")

    real_manifest = load_json(real_manifest_path)
    real_success = load_json(real_success_path)

    # Green: a pre-close-out (false-valued) manifest stays acceptable — the
    # fail-closed _ok helpers always accept the old shape.
    pre = pre_close_out_tonemap_manifest(real_manifest)
    impl = pre["implementation_status"]
    if not probe.grx010_manifest_implemented_ok(pre):
        raise AssertionError("pre-close-out implemented=false must be accepted")
    if not probe.grx010_manifest_runtime_state_ok(impl):
        raise AssertionError("pre-close-out runtime_state=fallback_only must be accepted")
    if not probe.grx010_manifest_real_gpu_pass_ok(impl):
        raise AssertionError("pre-close-out real_gpu_pass=false must be accepted")
    if not probe.grx010_manifest_dispatch_recorded_ok(impl):
        raise AssertionError("pre-close-out dispatch_recorded=false must be accepted")

    # Red: a tampered success artifact fails the strict audit (in-memory).
    tampered = copy.deepcopy(real_success)
    tampered["artifacts"]["dxil"]["sha256"] = "0" * 64
    tampered_issue = probe.grx010_real_pass_enablement_issue(tampered)
    if tampered_issue is None:
        raise AssertionError(
            "a tampered tonemap success artifact must be rejected by the audit"
        )

    # Red: a SKIP success document never advances the gate.
    skipped = copy.deepcopy(real_success)
    skipped["status"] = "skip"
    skipped.pop("real_pass_marker_line", None)
    skip_issue = probe.grx010_real_pass_enablement_issue(skipped)
    if skip_issue is None or "not success" not in skip_issue:
        raise AssertionError(
            f"a SKIP tonemap success document must be rejected, got {skip_issue!r}"
        )

    # Red: point the probe at a tampered on-disk success evidence -> conflict
    # is reported AND the fail-closed manifest _ok helpers reject the
    # close-out (true-valued) manifest because the measured success is not
    # active.
    saved_path = probe.GRX010_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE
    with tempfile.TemporaryDirectory() as tmp:
        tampered_path = pathlib.Path(tmp) / "real_pass_enablement_success_evidence.json"
        write_json(tampered_path, tampered)
        probe.GRX010_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE = tampered_path
        probe._GRX010_REAL_PASS_SUCCESS_AUDIT_CACHE.clear()
        try:
            if probe.grx010_real_pass_measured_success_active():
                raise AssertionError(
                    "a tampered on-disk success evidence must not be active"
                )
            if not probe.grx010_real_pass_success_evidence_conflict():
                raise AssertionError(
                    "a tampered on-disk success evidence must report a conflict"
                )
            close_out_impl = real_manifest["implementation_status"]
            if probe.grx010_manifest_implemented_ok(real_manifest):
                raise AssertionError(
                    "close-out implemented=true must be rejected while the "
                    "measured success is not active"
                )
            if probe.grx010_manifest_runtime_state_ok(close_out_impl):
                raise AssertionError(
                    "close-out runtime_state must be rejected while the measured "
                    "success is not active"
                )
            if probe.grx010_manifest_real_gpu_pass_ok(close_out_impl):
                raise AssertionError(
                    "close-out real_gpu_pass=true must be rejected while the "
                    "measured success is not active"
                )
            if probe.grx010_manifest_dispatch_recorded_ok(close_out_impl):
                raise AssertionError(
                    "close-out real_d3d12_dispatch_recorded=true must be rejected "
                    "while the measured success is not active"
                )
        finally:
            probe.GRX010_REAL_PASS_ENABLEMENT_SUCCESS_EVIDENCE = saved_path
            probe._GRX010_REAL_PASS_SUCCESS_AUDIT_CACHE.clear()

    # The real manifest / success evidence on disk must be untouched.
    if real_manifest_path.read_bytes() != real_manifest_bytes:
        raise AssertionError("real tonemap pass_manifest.json changed during close-out cases")
    if real_success_path.read_bytes() != real_success_bytes:
        raise AssertionError(
            "real tonemap real_pass_enablement_success_evidence.json changed during close-out cases"
        )
    print("grx010 tonemap close-out cases passed")


def run_grx_gate_sequence_cases() -> None:
    """GRX gate sequence (table-driven per-pass registration) regression.

    (1) The gate sequence registers grx011 (ssao_blur), grx012 (taa_resolve),
        grx013 (particles_copy), then grx014 (cluster_store). An empty walk is
        still a pure no-op. Walking the REAL registered sequence: grx011,
        grx012, grx013 AND grx014 are all fully closed out (contract + patch
        applyability + standalone dispatch smoke + real-pass enablement strict
        success + owner default-enable decision all green — grx014 closed out
        in GRX Wave 4 with patches 0023-0025 and the 0001..0026 enablement
        smoke), so the walk records ZERO module errors and advances
        ``next_action`` to grx014's gate-provided value
        (``start_grx015_gpu_culling_pass_contract``).
    (2) A broken gate module injected into a temporary sequence is reported as a
        ``grx_gate_module_error`` and MUST NOT rewrite ``next_action``: covers a
        syntax-error module, a module without ``evaluate``, an ``evaluate`` that
        raises, a conforming-but-not-ready module, and a tampered COPY of the
        REAL grx011 module with a required key removed. A fully-ready module
        advances ``next_action`` to its gate-provided value.

    All fixtures are temp files; the real ``ci/grx_gates/`` directory is never
    mutated.
    """
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    base = probe.GRX011_NEXT_ACTION

    # (1) The gate sequence registers grx011 (ssao_blur), grx012 (taa_resolve),
    # grx013 (particles_copy), then grx014 (cluster_store), in that order, and
    # nothing else.
    expected_sequence = [
        {"gate_id": "grx011", "module": "grx011_ssao_blur"},
        {"gate_id": "grx012", "module": "grx012_taa_resolve"},
        {"gate_id": "grx013", "module": "grx013_particles_copy"},
        {"gate_id": "grx014", "module": "grx014_cluster_store"},
        {"gate_id": "grx015", "module": "grx015_gpu_culling"},
        {"gate_id": "grx016", "module": "grx016_instance_compaction"},
        {"gate_id": "grx018", "module": "grx018_indirect_args"},
        {"gate_id": "grx019", "module": "grx019_fused_post_chain"},
    ]
    if list(probe.GRX_GATE_SEQUENCE) != expected_sequence:
        raise AssertionError(
            "GRX_GATE_SEQUENCE must register exactly the grx011..grx014 closed-out "
            "gates followed by the GRX Wave 4 bridge gates grx015 gpu_culling, "
            "grx016 instance_compaction, grx018 indirect_args and grx019 "
            f"fused_post_chain, got {probe.GRX_GATE_SEQUENCE!r}"
        )

    # (1a) An empty walk is still a pure no-op: base preserved, nothing recorded.
    empty_walk = probe.walk_grx_gate_sequence([], base, "base reason", None)
    if empty_walk["next_action"] != base:
        raise AssertionError(
            f"empty gate walk must keep next_action {base!r}, got {empty_walk['next_action']!r}"
        )
    if empty_walk["module_errors"] or empty_walk["evaluations"]:
        raise AssertionError("empty gate walk must record no evaluations or errors")

    # (1b) Walking the REAL registered sequence. grx011..grx014 are fully closed
    # out (contract + patch applyability + standalone dispatch smoke + real-pass
    # enablement strict success + owner default-enable decision all green), so
    # they advance next_action in turn. grx015 (gpu_culling) is the GRX Wave 4
    # BRIDGE frontier: its contract, patch applyability (the Godot patches
    # 0027-0029 have LANDED and stack cleanly), and standalone D3D12 dispatch
    # smoke are green, but its real-pass enablement is MEASURED-BLOCKED (the
    # enablement evidence exists on disk yet strict_success is not yet true) and
    # the owner default-enable decision is not recorded, so it is NOT all_ready.
    # The walk therefore fail-closed STOPS at grx015 with a recorded
    # grx_gate_module_error and leaves next_action at grx014's advance
    # (start_grx015_gpu_culling_pass_contract); grx016/grx018/grx019 are never
    # consulted. This asserts the REAL, honest gate state, not a fabricated
    # all-green ((1b) lesson: gate-state assertions track reality — the earlier
    # fixture expected the 0027-0029 patch block DEFERRED, but the block has since
    # landed measured-blocked, so the first blocking issue is now the enablement
    # strict-success gap). Every gate keeps default_enable_state=disabled.
    grx014_next_action = "start_grx015_gpu_culling_pass_contract"
    real_walk = probe.walk_grx_gate_sequence(list(probe.GRX_GATE_SEQUENCE), base)
    if len(real_walk["evaluations"]) != 5:
        raise AssertionError(
            "the real gate walk must evaluate grx011..grx014 (ready) then stop at "
            f"grx015 (bridge frontier, not-ready); got {real_walk['evaluations']!r}"
        )
    ready_records = real_walk["evaluations"][:4]
    grx015_record = real_walk["evaluations"][4]
    for record, gate_id in zip(ready_records, ("grx011", "grx012", "grx013", "grx014")):
        if record.get("gate_id") != gate_id or record.get("all_ready") is not True:
            raise AssertionError(
                f"the real gate record for {gate_id} must be fully-ready: {record!r}"
            )
        if record.get("module_error"):
            raise AssertionError(
                f"the fully-ready {gate_id} gate must carry no module_error: {record!r}"
            )
    # grx015 is honestly not-ready: it must NOT be all_ready, MUST carry a
    # module_error, and its first_issue must name the measured-blocked real-pass
    # enablement gap (the 0027-0029 patch block has LANDED, so the first blocking
    # issue is no longer the deferred patch block).
    if grx015_record.get("gate_id") != "grx015" or grx015_record.get("all_ready") is True:
        raise AssertionError(
            f"the grx015 bridge gate must be not-ready (enablement measured-blocked / "
            f"decision deferred): {grx015_record!r}"
        )
    if not grx015_record.get("module_error"):
        raise AssertionError(
            f"the not-ready grx015 gate must record a grx_gate_module_error: {grx015_record!r}"
        )
    grx015_issue = grx015_record.get("first_issue") or ""
    if "enablement" not in grx015_issue or "strict_success" not in grx015_issue:
        raise AssertionError(
            "the grx015 first_issue must name the measured-blocked real-pass "
            "enablement strict-success gap (the gpu_culling patch block 0027-0029 "
            f"has landed measured-blocked); got {grx015_issue!r}"
        )
    if len(real_walk["module_errors"]) != 1 or real_walk["module_errors"][0].get("gate_id") != "grx015":
        raise AssertionError(
            "the bridge walk must record exactly one grx_gate_module_error, for "
            f"grx015; got {real_walk['module_errors']!r}"
        )
    if real_walk["next_action"] != grx014_next_action:
        raise AssertionError(
            "the walk must advance next_action to grx014's value and then stop at "
            f"grx015; expected {grx014_next_action!r}, got {real_walk['next_action']!r}"
        )
    if real_walk["next_action"] == base:
        raise AssertionError(
            f"the closed-out grx011..grx014 walk must advance next_action off the base {base!r}"
        )

    # (2) Broken / non-ready gate modules must fail closed.
    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = pathlib.Path(tmp)
        syntax_error = tmp_dir / "broken_syntax.py"
        syntax_error.write_text("def evaluate(:\n    pass\n", encoding="utf-8")
        missing_evaluate = tmp_dir / "no_evaluate.py"
        missing_evaluate.write_text("VALUE = 1\n", encoding="utf-8")
        raising_evaluate = tmp_dir / "raises.py"
        raising_evaluate.write_text(
            "def evaluate():\n    raise RuntimeError('boom')\n", encoding="utf-8"
        )
        not_ready = tmp_dir / "not_ready.py"
        not_ready.write_text(
            "def evaluate():\n"
            "    return {\n"
            "        'gate_id': 'grx011',\n"
            "        'contract_ready': True,\n"
            "        'patch_applyability': False,\n"
            "        'dispatch_smoke_ready': False,\n"
            "        'enablement_ready': False,\n"
            "        'decision_ready': False,\n"
            "        'first_issue': 'patch 0014 not applyable',\n"
            "        'next_action': 'start_grx011_ssao_blur_godot_patch_0014',\n"
            "    }\n",
            encoding="utf-8",
        )
        all_ready = tmp_dir / "all_ready.py"
        all_ready.write_text(
            "def evaluate():\n"
            "    return {\n"
            "        'gate_id': 'grx011',\n"
            "        'contract_ready': True,\n"
            "        'patch_applyability': True,\n"
            "        'dispatch_smoke_ready': True,\n"
            "        'enablement_ready': True,\n"
            "        'decision_ready': True,\n"
            "        'first_issue': None,\n"
            "        'next_action': 'start_grx012_taa_resolve_godot_patch_0017',\n"
            "    }\n",
            encoding="utf-8",
        )

        for broken in (syntax_error, missing_evaluate, raising_evaluate, not_ready):
            walk = probe.walk_grx_gate_sequence(
                [{"gate_id": "grx011", "module_path": str(broken)}], base
            )
            if walk["next_action"] != base:
                raise AssertionError(
                    f"broken gate {broken.name} must not rewrite next_action; "
                    f"got {walk['next_action']!r}"
                )
            if not walk["module_errors"]:
                raise AssertionError(
                    f"broken gate {broken.name} must record a grx_gate_module_error"
                )

        # A fully-ready gate advances next_action to its gate-provided value.
        ready_walk = probe.walk_grx_gate_sequence(
            [{"gate_id": "grx011", "module_path": str(all_ready)}], base
        )
        if ready_walk["module_errors"]:
            raise AssertionError(
                f"a fully-ready gate must not error: {ready_walk['module_errors']!r}"
            )
        if ready_walk["next_action"] != "start_grx012_taa_resolve_godot_patch_0017":
            raise AssertionError(
                "a fully-ready gate must advance next_action, got "
                f"{ready_walk['next_action']!r}"
            )

        # (2b) A tampered COPY of the REAL grx011 module (a required evaluate()
        # key removed) must also fail closed as a module error, proving the walk
        # validates the shipped module's shape and not just synthetic fixtures.
        # The real ci/grx_gates/grx011_ssao_blur.py is never mutated.
        real_gate_src = (
            ROOT / "ci" / "grx_gates" / "grx011_ssao_blur.py"
        ).read_text(encoding="utf-8")
        tampered = tmp_dir / "grx011_tampered.py"
        tampered.write_text(
            real_gate_src
            + "\n\n_rurix_original_evaluate = evaluate\n"
            "def evaluate():\n"
            "    result = dict(_rurix_original_evaluate())\n"
            "    result.pop('decision_ready', None)\n"
            "    return result\n",
            encoding="utf-8",
        )
        tampered_walk = probe.walk_grx_gate_sequence(
            [{"gate_id": "grx011", "module_path": str(tampered)}], base
        )
        if tampered_walk["next_action"] != base:
            raise AssertionError(
                "tampered real grx011 module must not rewrite next_action; "
                f"got {tampered_walk['next_action']!r}"
            )
        if not tampered_walk["module_errors"]:
            raise AssertionError(
                "tampered real grx011 module (missing required key) must record a "
                "grx_gate_module_error"
            )

    # The real gate package directory carries the shared helpers plus the
    # grx011..grx014 closed-out gate modules and the GRX Wave 4 bridge gate
    # modules grx015/grx016/grx018/grx019; these cases never mutate it.
    real_gates_dir = ROOT / "ci" / "grx_gates"
    tracked = sorted(path.name for path in real_gates_dir.glob("*.py"))
    if tracked != [
        "__init__.py",
        "_common.py",
        "grx011_ssao_blur.py",
        "grx012_taa_resolve.py",
        "grx013_particles_copy.py",
        "grx014_cluster_store.py",
        "grx015_gpu_culling.py",
        "grx016_instance_compaction.py",
        "grx018_indirect_args.py",
        "grx019_fused_post_chain.py",
    ]:
        raise AssertionError(f"unexpected files in ci/grx_gates/: {tracked}")
    print("grx gate sequence table-driven cases passed")


def run_grx_milestone_closeout_cases() -> None:
    """GRX milestone close-out marker (honest-ceiling close-out) regression.

    Lesson applied: the REAL gate state flips with close-out; the NOT-ready
    (pre-close-out frontier) state is asserted via a FIXTURE.

    (1) The REAL on-disk marker is valid, so ``grx_milestone_closeout_ready()``
        returns ``(True, None)`` and the marker's ``next_action_when_closed``
        equals the probe's terminal ``GRX_MILESTONE_CLOSED_NEXT_ACTION``. This
        is the real, honest close-out state after the owner's terminal ruling.
    (2) The per-pass gate walk stays honest and UNCHANGED: it still fail-closed
        STOPS at the mechanism-blocked grx015 frontier (asserted in
        ``run_grx_gate_sequence_cases`` (1b)). The close-out is an owner-ruling
        governance override applied AFTER the walk, not a gate readiness flip.
    (3) Fail-closed fixtures (temp markers): an ABSENT marker, a marker whose
        ``status`` is not ``closed``, one whose ``performance_claim`` is not
        ``none``, and one referencing a missing evidence file each return
        ``(False, reason)`` so a missing/tampered marker keeps the honest
        gate-walk next_action (the pre-close-out frontier). A well-formed temp
        marker returns ``(True, None)``.

    The real marker file is never mutated; all negative fixtures are temp files.
    """
    sys.path.insert(0, str(ROOT))
    from ci import godot_rurix_toolchain_probe as probe

    # (1) REAL marker is valid -> ready True (real close-out state).
    ready, reason = probe.grx_milestone_closeout_ready()
    if ready is not True or reason is not None:
        raise AssertionError(
            "the real GRX milestone close-out marker must be valid "
            f"(honest-ceiling close-out): got ready={ready!r} reason={reason!r}"
        )
    real_marker = json.loads(
        probe.GRX_MILESTONE_CLOSEOUT_MARKER.read_text(encoding="utf-8")
    )
    if real_marker.get("next_action_when_closed") != probe.GRX_MILESTONE_CLOSED_NEXT_ACTION:
        raise AssertionError(
            "the real marker's next_action_when_closed must equal the probe "
            f"terminal next_action {probe.GRX_MILESTONE_CLOSED_NEXT_ACTION!r}"
        )
    if probe.GRX_MILESTONE_CLOSED_NEXT_ACTION != "grx_milestone_closed_ceiling_archived":
        raise AssertionError(
            "the terminal close-out next_action must be "
            f"grx_milestone_closed_ceiling_archived, got "
            f"{probe.GRX_MILESTONE_CLOSED_NEXT_ACTION!r}"
        )

    # A well-formed temp marker (evidence pointing at an existing repo file).
    valid_marker = {
        "status": "closed",
        "decision": "honest_ceiling_close_out",
        "owner_ruling": "user terminal ruling 2026-07-13",
        "performance_claim": "none",
        "next_action_when_closed": probe.GRX_MILESTONE_CLOSED_NEXT_ACTION,
        "evidence": {
            "matrix": "spike/godot-rurix/passes/DEFAULT_ENABLE_MATRIX.md",
        },
    }

    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = pathlib.Path(tmp)

        # (3a) Absent marker -> not ready (fail closed to the honest frontier).
        absent = tmp_dir / "missing_closeout.json"
        ready, reason = probe.grx_milestone_closeout_ready(absent)
        if ready is not False or not reason or "absent" not in reason:
            raise AssertionError(
                f"an absent close-out marker must fail closed: {ready!r}/{reason!r}"
            )

        # (3b) Well-formed temp marker -> ready True.
        good = tmp_dir / "good_closeout.json"
        good.write_text(json.dumps(valid_marker), encoding="utf-8")
        ready, reason = probe.grx_milestone_closeout_ready(good)
        if ready is not True or reason is not None:
            raise AssertionError(
                f"a well-formed close-out marker must be ready: {ready!r}/{reason!r}"
            )

        # (3c) status not closed -> not ready.
        not_closed = dict(valid_marker, status="active")
        bad = tmp_dir / "not_closed.json"
        bad.write_text(json.dumps(not_closed), encoding="utf-8")
        ready, reason = probe.grx_milestone_closeout_ready(bad)
        if ready is not False or not reason or "status" not in reason:
            raise AssertionError(
                f"a non-closed marker must fail closed: {ready!r}/{reason!r}"
            )

        # (3d) performance_claim not none -> not ready (guards against a
        # close-out marker sneaking in a performance claim).
        with_claim = dict(valid_marker, performance_claim="1.5x achieved")
        claim = tmp_dir / "with_claim.json"
        claim.write_text(json.dumps(with_claim), encoding="utf-8")
        ready, reason = probe.grx_milestone_closeout_ready(claim)
        if ready is not False or not reason or "performance_claim" not in reason:
            raise AssertionError(
                f"a marker with a performance_claim must fail closed: {ready!r}/{reason!r}"
            )

        # (3e) evidence file missing on disk -> not ready.
        missing_ev = dict(
            valid_marker,
            evidence={"ghost": "spike/godot-rurix/passes/__does_not_exist__.md"},
        )
        ghost = tmp_dir / "missing_evidence.json"
        ghost.write_text(json.dumps(missing_ev), encoding="utf-8")
        ready, reason = probe.grx_milestone_closeout_ready(ghost)
        if ready is not False or not reason or "missing on disk" not in reason:
            raise AssertionError(
                f"a marker with missing evidence must fail closed: {ready!r}/{reason!r}"
            )

    print("grx milestone close-out marker cases passed")


def main() -> int:
    original_manifest_bytes = REAL_MANIFEST_PATH.read_bytes()
    original_evidence_bytes = REAL_EVIDENCE_PATH.read_bytes()
    # Fixture pass dirs carry no 4h strict measured success artifact, so the
    # probe's stage A5 fail-closed manifest checks require the pre-A5 shape
    # (implemented=false, runtime_state=fallback_only, real_gpu_pass=false).
    manifest = pre_a5_fixture_manifest(
        json.loads(original_manifest_bytes.decode("utf-8"))
    )
    evidence = json.loads(original_evidence_bytes.decode("utf-8"))
    raw_buffer_evidence = json.loads(
        REAL_RAW_BUFFER_EVIDENCE_PATH.read_bytes().decode("utf-8")
    )
    segment4b_evidence = segment4b_success_evidence(evidence, raw_buffer_evidence)
    segment4a_manifest = make_segment4a_manifest(manifest)
    segment4b_manifest = make_segment4b_manifest(manifest)
    run_dxil_toolchain_preflight_cases()
    run_segment4a_probe_cases(segment4a_manifest, evidence)
    run_segment4a_runtime_state_reject_cases(segment4a_manifest, evidence)
    run_segment4a_marker_reject_cases(segment4a_manifest, evidence)
    run_segment4b_probe_cases(segment4b_manifest, segment4b_evidence)
    run_segment4f_success_audit_cases()
    run_segment4f_sidecar_chain_parity_cases()
    run_segment4g_visual_fallback_cases()
    run_segment4h_real_pass_enablement_cases()
    run_patch_stack_git_config_order_case()
    run_red_case(manifest, evidence)
    run_green_case(manifest, evidence)
    run_segment4i_contradiction_red_case(manifest, evidence)
    run_segment4i_fail_closed_green_case(manifest, evidence)
    run_texture_dxc_feasibility_probe_cases(manifest, evidence)
    run_dxc_texture_artifact_bridge_design_gate_cases(manifest, evidence)
    run_dxc_texture_artifact_bridge_scaffold_gate_cases(manifest, evidence)
    run_dxc_texture_rts0_integration_gate_cases(manifest, evidence)
    run_dxc_texture_descriptor_rts0_crosscheck_gate_cases(manifest, evidence)
    run_texture_artifact_provenance_policy_gate_cases(manifest, evidence)
    run_grx010_tonemap_gate_cases()
    run_grx010_tonemap_close_out_cases()
    run_grx_gate_sequence_cases()
    run_grx_milestone_closeout_cases()
    if REAL_MANIFEST_PATH.read_bytes() != original_manifest_bytes:
        raise AssertionError("real pass_manifest.json changed during validation_failed test")
    if REAL_EVIDENCE_PATH.read_bytes() != original_evidence_bytes:
        raise AssertionError("real offline_compile_evidence.json changed during validation_failed test")
    print("validation_failed probe regression checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
