#!/usr/bin/env python3
"""Validate GRX-008 fallback telemetry evidence documents.

GRX-008 is scaffold only. This tool validates a fallback telemetry document
against schemas/fallback_telemetry.schema.json expectations: each pass entry
must record an enable/disable state, a fallback reason from the fixed enum,
whether the Godot fallback path is active, and telemetry timestamp/frame/pass
id. It distinguishes scaffold from full/measured_local documents: scaffold
documents (run_mode=scaffold / evidence_level=scaffold) may leave
telemetry_timestamp/telemetry_frame null but every pass must be disabled with
godot_fallback_active=true, while full runs (run_mode=full or
evidence_level=measured_local) require a non-empty telemetry_timestamp and a
non-negative integer telemetry_frame, and measured_local documents may not use a
pass_id starting with placeholder_. At the scaffold stage every pass is a
placeholder; the presence of a pass entry here does NOT mean that pass has been
wired up, accelerated, or that a real fallback occurred. No acceleration pass is
implemented and no telemetry is measured.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys


TARGET_BACKEND = "Godot 4.7-dev Windows D3D12 Forward+"
VALID_ENABLE_STATES = ("enabled", "disabled")
VALID_FALLBACK_REASONS = (
    "compile_failed",
    "validation_failed",
    "unsupported_device",
    "visual_diff_failed",
    "manual_disabled",
)


def non_empty_string(value: object) -> bool:
    return isinstance(value, str) and value.strip() != ""


def validate_pass(
    index: int,
    entry: object,
    *,
    is_full: bool,
    is_measured_local: bool,
) -> list[str]:
    errors: list[str] = []
    label = f"pass#{index}"
    if not isinstance(entry, dict):
        return [f"{label}: pass entry must be a JSON object"]

    pass_id = entry.get("pass_id")
    if not non_empty_string(pass_id):
        errors.append(f"{label}: pass_id must be a non-empty string")
    else:
        label = f"pass#{index}({pass_id})"

    if entry.get("enable_state") not in VALID_ENABLE_STATES:
        errors.append(f"{label}: enable_state must be one of {VALID_ENABLE_STATES}")

    if entry.get("fallback_reason") not in VALID_FALLBACK_REASONS:
        errors.append(
            f"{label}: fallback_reason must be one of {VALID_FALLBACK_REASONS}"
        )

    if not isinstance(entry.get("godot_fallback_active"), bool):
        errors.append(f"{label}: godot_fallback_active must be a boolean")

    if "telemetry_timestamp" not in entry:
        errors.append(f"{label}: telemetry_timestamp is required (null placeholder allowed)")
    else:
        timestamp = entry.get("telemetry_timestamp")
        if timestamp is not None and not non_empty_string(timestamp):
            errors.append(
                f"{label}: telemetry_timestamp must be a non-empty string or null"
            )

    if "telemetry_frame" not in entry:
        errors.append(f"{label}: telemetry_frame is required (null placeholder allowed)")
    else:
        frame = entry.get("telemetry_frame")
        if frame is not None and (
            not isinstance(frame, int) or isinstance(frame, bool) or frame < 0
        ):
            errors.append(
                f"{label}: telemetry_frame must be a non-negative integer or null"
            )

    if is_full:
        if not non_empty_string(entry.get("telemetry_timestamp")):
            errors.append(
                f"{label}: run_mode=full/evidence_level=measured_local requires a "
                "non-empty telemetry_timestamp (null placeholder not allowed)"
            )
        frame = entry.get("telemetry_frame")
        if not isinstance(frame, int) or isinstance(frame, bool) or frame < 0:
            errors.append(
                f"{label}: run_mode=full/evidence_level=measured_local requires a "
                "non-negative integer telemetry_frame (null placeholder not allowed)"
            )
    if (
        is_measured_local
        and isinstance(pass_id, str)
        and pass_id.startswith("placeholder_")
    ):
        errors.append(
            f"{label}: evidence_level=measured_local forbids a pass_id starting with 'placeholder_'"
        )
    if not is_full:  # scaffold placeholder document
        if entry.get("enable_state") != "disabled":
            errors.append(f"{label}: scaffold telemetry requires enable_state=disabled")
        if entry.get("godot_fallback_active") is not True:
            errors.append(f"{label}: scaffold telemetry requires godot_fallback_active=true")
    return errors


def validate_document(data: object) -> tuple[bool, list[str]]:
    errors: list[str] = []
    if not isinstance(data, dict):
        return False, ["document root must be a JSON object"]

    if data.get("run_mode") not in ("scaffold", "full"):
        errors.append("run_mode must be 'scaffold' or 'full'")
    if data.get("evidence_level") not in ("scaffold", "measured_local"):
        errors.append("evidence_level must be 'scaffold' or 'measured_local'")
    if data.get("target_backend") != TARGET_BACKEND:
        errors.append(f"target_backend must be '{TARGET_BACKEND}'")
    if not non_empty_string(data.get("note")):
        errors.append("note must be a non-empty string")

    passes = data.get("passes")
    if not isinstance(passes, list) or not passes:
        errors.append("passes must be a non-empty list")
        return (not errors), errors

    is_full = data.get("run_mode") == "full" or data.get("evidence_level") == "measured_local"
    is_measured_local = data.get("evidence_level") == "measured_local"
    for index, entry in enumerate(passes):
        errors.extend(
            validate_pass(
                index, entry, is_full=is_full, is_measured_local=is_measured_local
            )
        )

    return (not errors), errors


def report_passes(data: dict[str, object]) -> None:
    passes = data.get("passes")
    assert isinstance(passes, list)
    for index, entry in enumerate(passes):
        assert isinstance(entry, dict)
        pass_id = entry.get("pass_id", f"pass#{index}")
        print(
            f"[fallback-telemetry] SKIP {pass_id}: enable_state={entry.get('enable_state')} "
            f"fallback_reason={entry.get('fallback_reason')} "
            f"godot_fallback_active={entry.get('godot_fallback_active')}"
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=pathlib.Path)
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="only validate the fallback telemetry evidence format",
    )
    args = parser.parse_args()

    try:
        data = json.loads(args.results.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(
            f"[fallback-telemetry] FORMAT FAIL could not read input: {exc}",
            file=sys.stderr,
        )
        return 1

    ok, errors = validate_document(data)
    if not ok:
        for error in errors:
            print(f"[fallback-telemetry] FORMAT FAIL {error}", file=sys.stderr)
        return 1
    print("[fallback-telemetry] FORMAT PASS fallback telemetry document is valid")

    if args.validate_only:
        return 0

    assert isinstance(data, dict)
    report_passes(data)
    print(
        "[fallback-telemetry] SCAFFOLD placeholder telemetry only; this does NOT mean "
        "any pass has been wired up or that a real fallback occurred"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
