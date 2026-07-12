#!/usr/bin/env python3
"""GRX-018 ``indirect_args`` gate module.

Exports ``evaluate() -> dict`` per the interface contract in
``ci/grx_gates/_common.py``. The probe registers this module seventh in
``GRX_GATE_SEQUENCE`` (after grx011..grx016) and walks it fail-closed.

GRX-018 indirect_args is at the GRX Wave 4 *bridge* stage: the contract trio, the
offline kernel package, the integer-exact CPU math-parity reference, the
fail-closed ``IndirectArgsGate`` (paired write + validate kernels with its
shim recording entry), and the standalone D3D12 dispatch smoke (four tracked
digests, exact GPU-vs-CPU match, INCLUDING the corrupted-staging RED leg where
validate must report a non-zero mismatch count) have all landed. The Godot patches (0033-0035), the gated
real-pass enablement strict success, and the owner default-enable decision are
DEFERRED to the next serial patch slice, so this gate honestly reports
``patch_applyability`` / ``enablement_ready`` / ``decision_ready`` as ``False``
and the probe leaves ``next_action`` unchanged (fail-closed stop). Every level
reports its readiness HONESTLY; a green level never implies default enablement, a
real Godot runtime pass, or any performance claim (the pass ships default
disabled and fallback-only).
"""
from __future__ import annotations

import pathlib

import _common

GATE_ID = "grx018"
# The action the probe advances to once THIS gate is fully ready: the next
# pass's contract start (fused_post_chain, GRX-019 in the milestone order).
NEXT_ACTION = "start_grx019_fused_post_chain_pass_contract"

ROOT = _common.ROOT
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "indirect_args"
PATCHES_DIR = ROOT / "spike" / "godot-rurix" / "patches"
EXTERNAL_GODOT = ROOT / "external" / "godot-master"

CONTRACT_TRIO = ("PASS_CONTRACT.md", "pass_manifest.json", "resource_mapping.md")

# The three indirect_args patches (PATCH_ALLOCATION.md §2 0033-0035). They are
# DEFERRED to the next serial patch slice; a missing file here is the expected,
# honest not-ready state (not a tampered stack).
INDIRECT_ARGS_PATCH_ORDINALS = ("0033", "0034", "0035")
PREREQ_ORDINALS = tuple(f"{n:04d}" for n in range(4, 34))  # 0004..0033

DISPATCH_SMOKE_EVIDENCE = PASS_DIR / "real_d3d12_dispatch_smoke.json"
ENABLEMENT_SUCCESS_EVIDENCE = PASS_DIR / "real_pass_enablement_success_evidence.json"
DEFAULT_ENABLE_DECISION = PASS_DIR / "real_pass_default_enable_decision.json"

PASS_ID = "indirect_args"
MATH_PARITY_STATUS = "indirect_args_cpu_reference_proven_pending_gpu_dispatch"
PASS_ENUM_MARKER = "RXGD_PASS_INDIRECT_ARGS"


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
    if manifest.get("pass_id") != PASS_ID:
        return False, f"pass_manifest.json pass_id is not {PASS_ID}"
    if manifest.get("default_enable_state") != "disabled":
        return False, "pass_manifest.json default_enable_state must be disabled"
    if manifest.get("math_parity_status") != MATH_PARITY_STATUS:
        return False, "pass_manifest.json math_parity_status does not match the gpu_culling contract"
    text = (PASS_DIR / "pass_manifest.json").read_text(encoding="utf-8")
    if PASS_ENUM_MARKER not in text:
        return False, f"pass_manifest.json does not reference {PASS_ENUM_MARKER}"
    return True, None


def _patch_applyability() -> tuple[bool, str | None]:
    # DEFERRED: the indirect_args patch block (0033-0035) is authored in the next
    # serial patch slice; this Wave 4 bridge slice landed the S4 gate + S6
    # dispatch smoke only. Report not-ready honestly until they exist.
    for ordinal in INDIRECT_ARGS_PATCH_ORDINALS:
        if _patch_file(ordinal) is None:
            return False, (
                f"indirect_args patches {'/'.join(INDIRECT_ARGS_PATCH_ORDINALS)} not yet authored "
                "(DEFERRED to the next serial patch slice; the GRX Wave 4 bridge slice landed "
                "the fail-closed IndirectArgsGate + standalone D3D12 dispatch smoke only)"
            )
    prereqs = []
    for ordinal in PREREQ_ORDINALS:
        path = _patch_file(ordinal)
        if path is None:
            return False, f"prerequisite patch {ordinal} not found in {PATCHES_DIR}"
        prereqs.append(path)
    top = _patch_file("0035")
    result = _common.evaluate_stacked_patch_applyability(ROOT, EXTERNAL_GODOT, prereqs, top, "0035")
    if result.get("ok") is not True:
        return False, f"indirect_args patch 0035 stacked applyability failed: {result.get('reason')}"
    return True, None


def _dispatch_smoke_ready() -> tuple[bool, str | None]:
    doc = _common.load_json_file(DISPATCH_SMOKE_EVIDENCE)
    if doc is None:
        return False, (
            "standalone D3D12 dispatch smoke evidence not present "
            f"({DISPATCH_SMOKE_EVIDENCE.name}); run "
            "ci/grx018_indirect_args_d3d12_dispatch_smoke.py"
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
            f"({ENABLEMENT_SUCCESS_EVIDENCE.name}); DEFERRED to the next serial patch slice"
        )
    if doc.get("strict_success") is not True:
        return False, "enablement evidence strict_success is not true"
    return True, None


def _decision_ready() -> tuple[bool, str | None]:
    doc = _common.load_json_file(DEFAULT_ENABLE_DECISION)
    if doc is None:
        return False, (
            "owner default-enable decision not recorded "
            f"({DEFAULT_ENABLE_DECISION.name}); DEFERRED to the next serial patch slice"
        )
    if not doc.get("default_enable_decision"):
        return False, "default-enable decision document has no default_enable_decision field"
    return True, None


def evaluate() -> dict:
    contract_ready, contract_issue = _contract_ready()
    if contract_ready:
        patch_applyability, patch_issue = _patch_applyability()
    else:
        patch_applyability, patch_issue = False, None
    dispatch_ready, dispatch_issue = _dispatch_smoke_ready()
    enablement_ready, enablement_issue = _enablement_ready()
    decision_ready, decision_issue = _decision_ready()

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
