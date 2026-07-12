#!/usr/bin/env python3
"""GRX-016 ``instance_compaction`` gate module.

Exports ``evaluate() -> dict`` per the interface contract in
``ci/grx_gates/_common.py``. The probe registers this module sixth in
``GRX_GATE_SEQUENCE`` (after grx011..grx015) and walks it fail-closed.

GRX-016 instance_compaction is at the GRX Wave 4 *bridge* stage: the contract trio, the
offline kernel package, the integer-exact CPU math-parity reference, the
fail-closed ``InstanceCompactionGate`` (a three-kernel scan/compaction chain
with its shim recording entry), and the standalone D3D12 dispatch smoke (seven
tracked digests, exact GPU-vs-CPU match over the full scan->scatter chain) have all landed. The Godot patches (0030-0032), the gated
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

GATE_ID = "grx016"
# The action the probe advances to once THIS gate is fully ready: the next
# pass's contract start (indirect_args, GRX-018 in the milestone order).
NEXT_ACTION = "start_grx018_indirect_args_pass_contract"

ROOT = _common.ROOT
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "instance_compaction"
PATCHES_DIR = ROOT / "spike" / "godot-rurix" / "patches"
EXTERNAL_GODOT = ROOT / "external" / "godot-master"

CONTRACT_TRIO = ("PASS_CONTRACT.md", "pass_manifest.json", "resource_mapping.md")

# The three instance_compaction patches (PATCH_ALLOCATION.md §2 0030-0032). They are
# DEFERRED to the next serial patch slice; a missing file here is the expected,
# honest not-ready state (not a tampered stack).
INSTANCE_COMPACTION_PATCH_ORDINALS = ("0030", "0031", "0032")
PREREQ_ORDINALS = tuple(f"{n:04d}" for n in range(4, 31))  # 0004..0030

DISPATCH_SMOKE_EVIDENCE = PASS_DIR / "real_d3d12_dispatch_smoke.json"
ENABLEMENT_SUCCESS_EVIDENCE = PASS_DIR / "real_pass_enablement_success_evidence.json"
DEFAULT_ENABLE_DECISION = PASS_DIR / "real_pass_default_enable_decision.json"

PASS_ID = "instance_compaction"
MATH_PARITY_STATUS = "instance_compaction_cpu_reference_proven_pending_gpu_dispatch"
PASS_ENUM_MARKER = "RXGD_PASS_INSTANCE_COMPACTION"


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
    # DEFERRED: the instance_compaction patch block (0030-0032) is authored in the next
    # serial patch slice; this Wave 4 bridge slice landed the S4 gate + S6
    # dispatch smoke only. Report not-ready honestly until they exist.
    for ordinal in INSTANCE_COMPACTION_PATCH_ORDINALS:
        if _patch_file(ordinal) is None:
            return False, (
                f"instance_compaction patches {'/'.join(INSTANCE_COMPACTION_PATCH_ORDINALS)} not yet authored "
                "(DEFERRED to the next serial patch slice; the GRX Wave 4 bridge slice landed "
                "the fail-closed InstanceCompactionGate + standalone D3D12 dispatch smoke only)"
            )
    prereqs = []
    for ordinal in PREREQ_ORDINALS:
        path = _patch_file(ordinal)
        if path is None:
            return False, f"prerequisite patch {ordinal} not found in {PATCHES_DIR}"
        prereqs.append(path)
    top = _patch_file("0032")
    result = _common.evaluate_stacked_patch_applyability(ROOT, EXTERNAL_GODOT, prereqs, top, "0032")
    if result.get("ok") is not True:
        return False, f"instance_compaction patch 0032 stacked applyability failed: {result.get('reason')}"
    return True, None


def _dispatch_smoke_ready() -> tuple[bool, str | None]:
    doc = _common.load_json_file(DISPATCH_SMOKE_EVIDENCE)
    if doc is None:
        return False, (
            "standalone D3D12 dispatch smoke evidence not present "
            f"({DISPATCH_SMOKE_EVIDENCE.name}); run "
            "ci/grx016_instance_compaction_d3d12_dispatch_smoke.py"
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
