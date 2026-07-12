#!/usr/bin/env python3
"""GRX-011 ``ssao_blur`` gate module.

Exports ``evaluate() -> dict`` per the interface contract in
``ci/grx_gates/_common.py``. The probe (``ci/godot_rurix_toolchain_probe.py``)
registers this module first in ``GRX_GATE_SEQUENCE`` and walks it fail-closed:
the probe only advances ``next_action`` past the grx010 hand-off when this gate
is fully ready (contract + patch applyability + standalone dispatch smoke +
real-pass enablement + owner default-enable decision all green).

Every level below reports its readiness HONESTLY. The pass ships default
disabled and fallback-only; a green level never implies ``real_gpu_pass=true``,
a real Godot runtime pass, or any performance claim.
"""
from __future__ import annotations

import pathlib

import _common

GATE_ID = "grx011"
# The action the probe advances to once this gate is fully ready: the next
# pass's contract start (mirrors GRX010_NEXT_ACTION="start_grx010_tonemap_pass_contract").
NEXT_ACTION = "start_grx012_taa_resolve_pass_contract"

ROOT = _common.ROOT
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "ssao_blur"
PATCHES_DIR = ROOT / "spike" / "godot-rurix" / "patches"
EXTERNAL_GODOT = ROOT / "external" / "godot-master"

# S1 pass-contract trio.
CONTRACT_TRIO = ("PASS_CONTRACT.md", "pass_manifest.json", "resource_mapping.md")

# The three ssao_blur patches this pass owns (PATCH_ALLOCATION.md 0014-0016).
SSAO_PATCH_ORDINALS = ("0014", "0015", "0016")
# Godot snapshot already carries 0001..0003; stacked applyability prereqs run
# from 0004 up to the patch under test (mirrors ci/godot_rurix_bridge_smoke.py).
PREREQ_ORDINALS = tuple(f"{n:04d}" for n in range(4, 16))  # 0004..0015

# Standalone dispatch smoke evidence (S6) + real-pass enablement (S8) + owner
# default-enable decision (S9). These are produced by the downstream smokes;
# until they land green this gate stays honestly not-ready and fail-closed.
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
    # Coherence: the manifest must reference the ssao_blur patch allocation
    # (0014-0016) and no longer the stale 0012 reference.
    text = (PASS_DIR / "pass_manifest.json").read_text(encoding="utf-8")
    if "0012-rurix-accel-ssao-blur" in text:
        return False, "pass_manifest.json still references the stale patch 0012 allocation"
    if "0014-rurix-accel-ssao-blur-pass-gate-and-callsite.patch" not in text:
        return False, "pass_manifest.json does not reference the ssao_blur 0014 gate patch"
    return True, None


def _patch_applyability() -> tuple[bool, str | None]:
    for ordinal in SSAO_PATCH_ORDINALS:
        if _patch_file(ordinal) is None:
            return False, f"ssao_blur patch {ordinal} not found in {PATCHES_DIR}"
    # Validate the top ssao_blur patch applies on the full 0004..0015 prereq
    # stack in a throwaway scratch copy (one stacked check exercises the whole
    # chain). external/godot-master is never mutated.
    prereqs = []
    for ordinal in PREREQ_ORDINALS:
        path = _patch_file(ordinal)
        if path is None:
            return False, f"prerequisite patch {ordinal} not found in {PATCHES_DIR}"
        prereqs.append(path)
    top = _patch_file("0016")
    result = _common.evaluate_stacked_patch_applyability(
        ROOT, EXTERNAL_GODOT, prereqs, top, "0016"
    )
    if result.get("ok") is not True:
        return False, f"ssao_blur patch 0016 stacked applyability failed: {result.get('reason')}"
    return True, None


def _dispatch_smoke_ready() -> tuple[bool, str | None]:
    doc = _common.load_json_file(DISPATCH_SMOKE_EVIDENCE)
    if doc is None:
        return False, (
            "standalone D3D12 dispatch smoke evidence not present "
            f"({DISPATCH_SMOKE_EVIDENCE.name}); run "
            "ci/grx011_ssao_blur_d3d12_dispatch_smoke.py"
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
            f"({ENABLEMENT_SUCCESS_EVIDENCE.name}); run "
            "ci/grx011_ssao_blur_real_pass_enablement_smoke.py"
        )
    if doc.get("strict_success") is not True:
        return False, "enablement evidence strict_success is not true"
    return True, None


def _decision_ready() -> tuple[bool, str | None]:
    doc = _common.load_json_file(DEFAULT_ENABLE_DECISION)
    if doc is None:
        return False, (
            "owner default-enable decision not recorded "
            f"({DEFAULT_ENABLE_DECISION.name})"
        )
    if not _common.load_json_file(DEFAULT_ENABLE_DECISION).get("default_enable_decision"):
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
