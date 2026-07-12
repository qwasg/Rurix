#!/usr/bin/env python3
"""GRX-012 ``taa_resolve`` gate module.

Exports ``evaluate() -> dict`` per the interface contract in
``ci/grx_gates/_common.py``. The probe (``ci/godot_rurix_toolchain_probe.py``)
registers this module second in ``GRX_GATE_SEQUENCE`` (after grx011) and walks
it fail-closed: the probe only advances ``next_action`` off this gate once it is
fully ready (contract + patch applyability + standalone dispatch smoke +
real-pass enablement + owner default-enable decision all green).

GRX-012 taa_resolve is CLOSED OUT (mirroring GRX-011 ssao_blur): the contract
trio, the offline kernel, the fail-closed TaaResolveGate, the Godot patches
0017-0019, the standalone dispatch smoke, the gated real-pass enablement
(strict measured success with the GRX_PLAN temporal DoD), and the owner
default-enable decision (keep_default_disabled) are all present, so every
level reports ``True`` and the probe advances ``next_action`` to
``start_grx013_particles_copy_pass_contract``. Any missing/tampered artifact
flips the corresponding level ``False``, records a ``grx_gate_module_error``,
and keeps the base ``next_action`` unchanged (fail-closed).

Every level below reports its readiness HONESTLY. The pass ships default
disabled and fallback-only by default; a green level (including the measured
real-pass success) never implies default enablement, a real Godot runtime pass
on the default path, or any performance claim.
"""
from __future__ import annotations

import pathlib

import _common

GATE_ID = "grx012"
# The action the probe advances to once THIS gate is fully ready: the next
# pass's contract start (mirrors grx011's start_grx012_taa_resolve_pass_contract).
# Never applied in this slice — the gate is not fully ready.
NEXT_ACTION = "start_grx013_particles_copy_pass_contract"

ROOT = _common.ROOT
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "taa_resolve"
PATCHES_DIR = ROOT / "spike" / "godot-rurix" / "patches"
EXTERNAL_GODOT = ROOT / "external" / "godot-master"

# S1 pass-contract trio.
CONTRACT_TRIO = ("PASS_CONTRACT.md", "pass_manifest.json", "resource_mapping.md")

# The three taa_resolve patches this pass will own (PATCH_ALLOCATION.md
# 0017-0019); DEFERRED to a later serial slice — they do not exist yet.
TAA_PATCH_ORDINALS = ("0017", "0018", "0019")
# Godot snapshot carries 0001..0003; stacked applyability prereqs would run from
# 0004 up to the patch under test once the patches land.
PREREQ_ORDINALS = tuple(f"{n:04d}" for n in range(4, 19))  # 0004..0018

# Standalone dispatch smoke evidence (S6, produced this slice) + real-pass
# enablement (S8) + owner default-enable decision (S9). Until the downstream
# smokes land green this gate stays honestly not-ready and fail-closed.
DISPATCH_SMOKE_EVIDENCE = PASS_DIR / "real_d3d12_dispatch_smoke.json"
ENABLEMENT_SUCCESS_EVIDENCE = PASS_DIR / "real_pass_enablement_success_evidence.json"
DEFAULT_ENABLE_DECISION = PASS_DIR / "real_pass_default_enable_decision.json"


def _patch_file(ordinal: str) -> pathlib.Path | None:
    matches = sorted(PATCHES_DIR.glob(f"{ordinal}-*.patch"))
    return matches[0] if matches else None


def _contract_ready() -> tuple[bool, str | None]:
    for name in CONTRACT_TRIO:
        if not (PASS_DIR / name).is_file():
            return False, f"missing pass-contract file {name}"
    manifest = _common.load_json_file(PASS_DIR / "pass_manifest.json")
    if manifest is None:
        return False, "pass_manifest.json is missing or not a JSON object"
    if manifest.get("pass_id") != "taa_resolve":
        return False, "pass_manifest.json pass_id is not taa_resolve"
    # Fail-closed invariant: even after the real-pass close-out (implemented=true
    # / real_gpu_pass=true record the opt-in MEASURED arm only) the pass MUST
    # ship default disabled.
    if manifest.get("default_enable_state") != "disabled":
        return False, "pass_manifest.json default_enable_state must be disabled"
    if manifest.get("math_parity_status") != "taa_resolve_cpu_reference_proven_pending_gpu_dispatch":
        return False, "pass_manifest.json math_parity_status does not match the taa_resolve contract"
    text = (PASS_DIR / "pass_manifest.json").read_text(encoding="utf-8")
    if "RXGD_PASS_TAA_RESOLVE" not in text:
        return False, "pass_manifest.json does not reference RXGD_PASS_TAA_RESOLVE"
    # The manifest must reference the taa_resolve 0017-0019 patch allocation.
    if "0017-rurix-accel-taa-resolve-pass-gate-and-callsite.patch" not in text:
        return False, "pass_manifest.json does not reference the taa_resolve 0017 gate patch"
    return True, None


def _patch_applyability() -> tuple[bool, str | None]:
    # Patches 0017-0019 are DEFERRED to a later serial slice; they do not exist
    # in this slice, so patch applyability is honestly not ready.
    for ordinal in TAA_PATCH_ORDINALS:
        if _patch_file(ordinal) is None:
            return False, (
                f"taa_resolve patch {ordinal} not found in {PATCHES_DIR} "
                "(patches 0017-0019 are deferred to a later serial slice)"
            )
    prereqs = []
    for ordinal in PREREQ_ORDINALS:
        path = _patch_file(ordinal)
        if path is None:
            return False, f"prerequisite patch {ordinal} not found in {PATCHES_DIR}"
        prereqs.append(path)
    top = _patch_file("0019")
    result = _common.evaluate_stacked_patch_applyability(
        ROOT, EXTERNAL_GODOT, prereqs, top, "0019"
    )
    if result.get("ok") is not True:
        return False, f"taa_resolve patch 0019 stacked applyability failed: {result.get('reason')}"
    return True, None


def _dispatch_smoke_ready() -> tuple[bool, str | None]:
    doc = _common.load_json_file(DISPATCH_SMOKE_EVIDENCE)
    if doc is None:
        return False, (
            "standalone D3D12 dispatch smoke evidence not present "
            f"({DISPATCH_SMOKE_EVIDENCE.name}); run "
            "ci/grx012_taa_resolve_d3d12_dispatch_smoke.py"
        )
    if doc.get("real_d3d12_dispatch_recorded") is not True:
        return False, "dispatch smoke evidence real_d3d12_dispatch_recorded is not true"
    if doc.get("cpu_reference_match") is False:
        return False, "dispatch smoke evidence cpu_reference_match is false"
    return True, None


def _enablement_ready() -> tuple[bool, str | None]:
    doc = _common.load_json_file(ENABLEMENT_SUCCESS_EVIDENCE)
    if doc is None:
        return False, (
            "real-pass enablement strict-success evidence not present "
            f"({ENABLEMENT_SUCCESS_EVIDENCE.name}); the enablement smoke is a "
            "later serial slice (requires patches 0017-0019 + a scratch rebuild)"
        )
    if doc.get("strict_success") is not True:
        return False, "enablement evidence strict_success is not true"
    return True, None


def _decision_ready() -> tuple[bool, str | None]:
    doc = _common.load_json_file(DEFAULT_ENABLE_DECISION)
    if doc is None:
        return False, (
            "owner default-enable decision not recorded "
            f"({DEFAULT_ENABLE_DECISION.name}); a later serial slice"
        )
    if not doc.get("default_enable_decision"):
        return False, "default-enable decision document has no default_enable_decision field"
    return True, None


def evaluate() -> dict:
    contract_ready, contract_issue = _contract_ready()
    # Patch applyability is only meaningful once the contract trio exists.
    if contract_ready:
        patch_applyability, patch_issue = _patch_applyability()
    else:
        patch_applyability, patch_issue = False, None
    dispatch_ready, dispatch_issue = _dispatch_smoke_ready()
    enablement_ready, enablement_issue = _enablement_ready()
    decision_ready, decision_issue = _decision_ready()

    # First blocking issue in level order: contract -> patch -> dispatch ->
    # enablement -> decision.
    first_issue = None
    for ready, issue in (
        (contract_ready, contract_issue),
        (patch_applyability, patch_issue),
        (dispatch_ready, dispatch_issue),
        (enablement_ready, enablement_issue),
        (decision_ready, decision_issue),
    ):
        if not ready:
            first_issue = issue
            break

    return _common.make_evaluation(
        GATE_ID,
        contract_ready=contract_ready,
        patch_applyability=patch_applyability,
        dispatch_smoke_ready=dispatch_ready,
        enablement_ready=enablement_ready,
        decision_ready=decision_ready,
        first_issue=first_issue,
        next_action=NEXT_ACTION,
    )


if __name__ == "__main__":
    import json

    print(json.dumps(evaluate(), indent=2))
