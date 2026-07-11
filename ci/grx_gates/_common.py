#!/usr/bin/env python3
"""Shared helpers + interface contract for GRX per-pass gate modules.

Industrialized GRX-011+ scaffolding: from GRX-011 onward every pass ships one
gate module under ``ci/grx_gates/`` (``grx011_ssao_blur.py``,
``grx012_taa_resolve.py``, ...). The probe
(``ci/godot_rurix_toolchain_probe.py``) registers them, in order, in
``GRX_GATE_SEQUENCE`` and walks them fail-closed to decide whether to advance
``next_action`` past the grx010 hand-off.

Gate-module interface (v1)
==========================

Each gate module MUST export a top-level callable::

    def evaluate() -> dict

whose return value is a ``dict`` with EXACTLY these keys::

    gate_id             str            # e.g. "grx011"
    contract_ready      bool           # S1 pass-contract trio present + coherent
    patch_applyability  bool           # S5/S7 patches apply on the scratch stack
    dispatch_smoke_ready bool          # S6 standalone D3D12 dispatch smoke green
    enablement_ready    bool           # S8 real-pass enablement strict success
    decision_ready      bool           # S9 owner default-enable decision recorded
    first_issue         str | None     # first blocking issue, or None when clear
    next_action         str | None     # action to advance to once this gate is
                                        # fully ready (e.g. the NEXT pass's start)

Fail-closed contract enforced by the probe
------------------------------------------

* A gate is ``all_ready`` only when every ``*_ready`` / ``*_applyability`` key
  is ``True`` AND ``first_issue`` is ``None``.
* The probe advances ``next_action`` to a gate's ``next_action`` ONLY for an
  ``all_ready`` gate; then it continues to the next registered gate.
* ANY failure — module import error, missing/non-callable ``evaluate``,
  ``evaluate`` raising, a non-dict result, a missing/mistyped key, or a
  non-empty ``first_issue`` / a false readiness key — is recorded as a
  ``grx_gate_module_error`` and leaves ``next_action`` UNCHANGED (the walk stops
  at the first such gate). See ``walk_grx_gate_sequence`` in the probe.

Import convention
-----------------

Gate modules are loaded by file path, and the probe's loader guarantees this
directory is on ``sys.path`` before it runs a module, so a gate module imports
these helpers with a plain::

    import _common

Provenance / sync obligation
----------------------------

``sha256_of_file`` and ``load_json_file`` are deliberately kept byte-for-byte
behaviour-compatible with the same-named helpers in
``ci/godot_rurix_toolchain_probe.py`` (``sha256_of_file`` there, and the
JSON-loading idiom). If those probe helpers change semantics, mirror the change
here. The stacked-patch applyability entry points are re-exported directly from
``ci/godot_rurix_patch_stack.py`` (single source of truth — no copy).
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import sys

GATE_INTERFACE_VERSION = 1

# ci/grx_gates/_common.py -> parents[0]=grx_gates, parents[1]=ci, parents[2]=repo root.
GRX_GATES_DIR = pathlib.Path(__file__).resolve().parent
CI_DIR = GRX_GATES_DIR.parent
ROOT = CI_DIR.parent

# Make the standalone (non-package) ci/ scripts importable regardless of how the
# owning probe was launched (as a script from ci/, or in-process as ci.*).
if str(CI_DIR) not in sys.path:
    sys.path.insert(0, str(CI_DIR))

# Stacked-patch applyability entry points: reuse the probe's shared patch-stack
# module directly rather than copying its logic (single source of truth).
from godot_rurix_patch_stack import (  # noqa: E402
    evaluate_followup_patch_applyability,
    evaluate_stacked_patch_applyability,
    patch_touched_paths,
)

# Required evaluate() keys and the subset that must ALL be True for readiness.
REQUIRED_EVALUATE_KEYS = (
    "gate_id",
    "contract_ready",
    "patch_applyability",
    "dispatch_smoke_ready",
    "enablement_ready",
    "decision_ready",
    "first_issue",
    "next_action",
)
READINESS_KEYS = (
    "contract_ready",
    "patch_applyability",
    "dispatch_smoke_ready",
    "enablement_ready",
    "decision_ready",
)


def load_json_file(path: pathlib.Path) -> dict | None:
    """Load a JSON object from ``path``; return ``None`` on missing/invalid.

    Mirror of the probe's JSON-loading idiom (``load_json_report``): never
    raises, returns ``None`` for a missing file or malformed / non-object JSON.
    """
    try:
        text = pathlib.Path(path).read_text(encoding="utf-8")
    except (OSError, ValueError):
        return None
    try:
        value = json.loads(text)
    except json.JSONDecodeError:
        return None
    return value if isinstance(value, dict) else None


def sha256_of_file(path: pathlib.Path) -> str | None:
    """Hex SHA-256 of ``path``; ``None`` when the file is absent.

    Byte-for-byte behaviour parity with
    ``ci/godot_rurix_toolchain_probe.py::sha256_of_file`` (64 KiB chunks).
    """
    path = pathlib.Path(path)
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def file_contains_all(path: pathlib.Path, needles: list[str] | tuple[str, ...]) -> bool:
    """True when ``path`` exists and its text contains every needle.

    Equivalent of the probe test-helper ``assert_contains`` collapsed to a
    boolean over several needles (a common gate-evidence marker check).
    """
    try:
        text = pathlib.Path(path).read_text(encoding="utf-8")
    except OSError:
        return False
    return all(needle in text for needle in needles)


def make_evaluation(
    gate_id: str,
    *,
    contract_ready: bool = False,
    patch_applyability: bool = False,
    dispatch_smoke_ready: bool = False,
    enablement_ready: bool = False,
    decision_ready: bool = False,
    first_issue: str | None = None,
    next_action: str | None = None,
) -> dict:
    """Build a conforming ``evaluate()`` result dict.

    Gate authors should return ``make_evaluation("grxNNN", ...)`` so the shape
    always matches ``REQUIRED_EVALUATE_KEYS`` and the probe's interface check.
    """
    return {
        "gate_id": gate_id,
        "contract_ready": bool(contract_ready),
        "patch_applyability": bool(patch_applyability),
        "dispatch_smoke_ready": bool(dispatch_smoke_ready),
        "enablement_ready": bool(enablement_ready),
        "decision_ready": bool(decision_ready),
        "first_issue": first_issue,
        "next_action": next_action,
    }


__all__ = [
    "GATE_INTERFACE_VERSION",
    "GRX_GATES_DIR",
    "CI_DIR",
    "ROOT",
    "REQUIRED_EVALUATE_KEYS",
    "READINESS_KEYS",
    "load_json_file",
    "sha256_of_file",
    "file_contains_all",
    "make_evaluation",
    "evaluate_followup_patch_applyability",
    "evaluate_stacked_patch_applyability",
    "patch_touched_paths",
]
