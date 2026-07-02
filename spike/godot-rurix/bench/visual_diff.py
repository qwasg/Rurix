#!/usr/bin/env python3
"""Validate GRX-007 visual diff evidence and compute LDR per-channel diffs.

GRX-007 is scaffold only. This tool validates a visual diff evidence document
against schemas/visual_diff_evidence.schema.json expectations and, when a
capture frame is marked ``pass`` with real reference and candidate frame files,
computes an LDR per-channel absolute diff. A capture frame marked status=pass
promises a real, comparable frame pair, so if its reference or candidate frame
file is missing, unreadable, not a valid channel document, or the two frames
disagree on channel count, the diff cannot be computed and the tool reports
DIFF FAIL with a non-zero exit; such a pass frame must not be silently downgraded
to SKIP. In --write-output mode, any pass frame that cannot compute a diff causes
the run to fail without writing evidence. Only status=skip frames report SKIP with
their recorded reason; no visual verification is claimed and no diff numbers are
fabricated. A capture frame with status=skip must not carry frame paths or any
diff numbers (reference/candidate paths, ldr/hdr/temporal diff must be null or
absent), so a skip cannot smuggle in fabricated diffs. HDR/temporal diffs are
declared in the schema but not produced at this stage.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys


EXPECTED_SCENES = [
    "clustered_lights",
    "many_mesh_instances",
    "material_variants",
    "post_fx_chain",
    "volumetric_fog",
    "particles",
    "mixed_forward_plus",
]
TARGET_BACKEND = "Godot 4.7-dev Windows D3D12 Forward+"
RESOLUTION = [1920, 1080]
VALID_STATUSES = ("pass", "skip")


def non_empty_string(value: object) -> bool:
    return isinstance(value, str) and value.strip() != ""


def is_non_negative_number_triple(value: object) -> bool:
    """True when value is a length-3 array of non-negative numbers (not bool)."""
    if not isinstance(value, list) or len(value) != 3:
        return False
    return all(
        isinstance(item, (int, float))
        and not isinstance(item, bool)
        and float(item) >= 0.0
        for item in value
    )


def scene_names_in_order(scenes: list[object]) -> list[str]:
    names: list[str] = []
    for scene in scenes:
        if isinstance(scene, dict) and isinstance(scene.get("name"), str):
            names.append(scene["name"])
    return names


def validate_capture_frame(scene_name: str, index: int, frame: object) -> list[str]:
    errors: list[str] = []
    label = f"{scene_name}[frame#{index}]"
    if not isinstance(frame, dict):
        return [f"{label}: capture frame must be a JSON object"]

    frame_index = frame.get("frame_index")
    if not isinstance(frame_index, int) or isinstance(frame_index, bool) or frame_index < 0:
        errors.append(f"{label}: frame_index must be a non-negative integer")

    status = frame.get("status")
    if status not in VALID_STATUSES:
        errors.append(f"{label}: status must be one of {VALID_STATUSES}")
        return errors

    if status == "skip":
        if not non_empty_string(frame.get("skip_reason")):
            errors.append(
                f"{label}: status=skip requires a non-empty skip_reason "
                "(e.g. missing capture backend / missing Godot full run / missing frame artifact)"
            )
        for key in (
            "reference_frame_path",
            "candidate_frame_path",
            "ldr_diff",
            "hdr_diff",
            "temporal_diff",
        ):
            if frame.get(key) is not None:
                errors.append(
                    f"{label}: status=skip forbids a non-null {key}; "
                    "skip frames must not carry fabricated frame paths or diff numbers"
                )
    else:  # status == "pass"
        for key in ("reference_frame_path", "candidate_frame_path"):
            if not non_empty_string(frame.get(key)):
                errors.append(f"{label}: status=pass requires a non-empty {key}")
        ldr_diff = frame.get("ldr_diff")
        if not isinstance(ldr_diff, dict):
            errors.append(f"{label}: status=pass requires an ldr_diff object")
        else:
            for key in ("per_channel_max_abs", "per_channel_mean_abs"):
                if key not in ldr_diff:
                    errors.append(
                        f"{label}: status=pass requires ldr_diff.{key}"
                    )
                elif not is_non_negative_number_triple(ldr_diff.get(key)):
                    errors.append(
                        f"{label}: ldr_diff.{key} must be a length-3 array of "
                        "non-negative numbers"
                    )
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
    if data.get("resolution") != RESOLUTION:
        errors.append("resolution must be [1920, 1080]")

    scenes = data.get("scenes")
    if not isinstance(scenes, list):
        errors.append("scenes must be a list")
        return (not errors), errors

    names = scene_names_in_order(scenes)
    if names != EXPECTED_SCENES:
        errors.append(
            "scenes must cover the seven fixed scenes in order: " + ", ".join(EXPECTED_SCENES)
        )

    for scene in scenes:
        if not isinstance(scene, dict):
            errors.append("each scene must be a JSON object")
            continue
        name = scene.get("name", "<unknown>")
        capture_frames = scene.get("capture_frames")
        if not isinstance(capture_frames, list) or not capture_frames:
            errors.append(f"{name}: capture_frames must be a non-empty list")
            continue
        for index, frame in enumerate(capture_frames):
            errors.extend(validate_capture_frame(str(name), index, frame))

    return (not errors), errors


def read_channels(path: pathlib.Path) -> list[list[int]] | None:
    """Best-effort read of frame channel data for LDR diff.

    Expects a small JSON frame artifact of the form
    {"pixels": [[r,g,b], ...]}. Returns None when the file cannot be read as
    such; the caller then reports SKIP instead of fabricating a diff.
    """
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(payload, dict):
        return None
    pixels = payload.get("pixels")
    if not isinstance(pixels, list) or not pixels:
        return None
    channels: list[list[int]] = []
    for pixel in pixels:
        if not isinstance(pixel, list) or len(pixel) != 3:
            return None
        if not all(isinstance(c, int) and not isinstance(c, bool) for c in pixel):
            return None
        channels.append([int(c) for c in pixel])
    return channels


def compute_ldr_diff(
    reference: list[list[int]], candidate: list[list[int]]
) -> dict[str, list[float]] | None:
    if len(reference) != len(candidate) or not reference:
        return None
    max_abs = [0.0, 0.0, 0.0]
    sum_abs = [0.0, 0.0, 0.0]
    for ref_pixel, cand_pixel in zip(reference, candidate):
        for channel in range(3):
            diff = abs(ref_pixel[channel] - cand_pixel[channel])
            if diff > max_abs[channel]:
                max_abs[channel] = float(diff)
            sum_abs[channel] += diff
    count = len(reference)
    mean_abs = [sum_abs[channel] / count for channel in range(3)]
    return {"per_channel_max_abs": max_abs, "per_channel_mean_abs": mean_abs}


def _triples_match(recorded: object, computed: list[float]) -> bool:
    if not isinstance(recorded, list) or len(recorded) != 3:
        return False
    for rec, comp in zip(recorded, computed):
        if not isinstance(rec, (int, float)) or isinstance(rec, bool):
            return False
        if abs(float(rec) - comp) > 1e-9:
            return False
    return True


def evaluate_frames(
    data: dict[str, object], *, write_output: bool
) -> tuple[int, int, int, int]:
    """Report per-frame pass/skip and compare recorded vs computed LDR diff.

    Returns (skip_count, diff_computed_count, mismatch_count,
    pass_uncomputable_count). A status=pass frame whose reference/candidate frame
    file is missing, unreadable, not a channel document, or channel-count
    mismatched is counted in pass_uncomputable_count and reported as DIFF FAIL;
    it is not downgraded to a skip. When write_output is True the computed
    ldr_diff overwrites the recorded value in-place (to generate evidence) and no
    mismatch is counted, but pass_uncomputable_count still blocks writing.
    """
    scenes = data.get("scenes")
    assert isinstance(scenes, list)
    skip_count = 0
    diff_computed = 0
    mismatch_count = 0
    pass_uncomputable_count = 0
    for scene in scenes:
        assert isinstance(scene, dict)
        name = scene.get("name", "<unknown>")
        for index, frame in enumerate(scene.get("capture_frames", [])):
            assert isinstance(frame, dict)
            label = f"{name}[frame#{index}]"
            if frame.get("status") == "skip":
                skip_count += 1
                reason = frame.get("skip_reason")
                print(f"[visual-diff] SKIP {label}: {reason}")
                continue
            ref_path = frame.get("reference_frame_path")
            cand_path = frame.get("candidate_frame_path")
            ref = pathlib.Path(str(ref_path))
            cand = pathlib.Path(str(cand_path))
            if not ref.exists() or not cand.exists():
                pass_uncomputable_count += 1
                missing = ref_path if not ref.exists() else cand_path
                print(
                    f"[visual-diff] DIFF FAIL {label}: status=pass frame artifact "
                    f"missing on disk ({missing}); cannot compute LDR diff",
                    file=sys.stderr,
                )
                continue
            reference = read_channels(ref)
            candidate = read_channels(cand)
            if reference is None or candidate is None:
                pass_uncomputable_count += 1
                print(
                    f"[visual-diff] DIFF FAIL {label}: status=pass frame artifact is "
                    "not a readable channel document; cannot compute LDR diff",
                    file=sys.stderr,
                )
                continue
            ldr = compute_ldr_diff(reference, candidate)
            if ldr is None:
                pass_uncomputable_count += 1
                print(
                    f"[visual-diff] DIFF FAIL {label}: status=pass reference/candidate "
                    "channel counts mismatch; cannot compute LDR diff",
                    file=sys.stderr,
                )
                continue
            diff_computed += 1
            print(
                f"[visual-diff] LDR {label}: per_channel_max_abs={ldr['per_channel_max_abs']} "
                f"per_channel_mean_abs={ldr['per_channel_mean_abs']}"
            )
            if write_output:
                frame["ldr_diff"] = ldr
                continue
            recorded = frame.get("ldr_diff")
            assert isinstance(recorded, dict)
            if not _triples_match(
                recorded.get("per_channel_max_abs"), ldr["per_channel_max_abs"]
            ) or not _triples_match(
                recorded.get("per_channel_mean_abs"), ldr["per_channel_mean_abs"]
            ):
                mismatch_count += 1
                print(
                    f"[visual-diff] DIFF FAIL {label}: recorded ldr_diff does not "
                    f"match computed per_channel_max_abs={ldr['per_channel_max_abs']} "
                    f"per_channel_mean_abs={ldr['per_channel_mean_abs']}",
                    file=sys.stderr,
                )
    return skip_count, diff_computed, mismatch_count, pass_uncomputable_count


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=pathlib.Path)
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="only validate the visual diff evidence format; do not compute diffs",
    )
    parser.add_argument(
        "--write-output",
        type=pathlib.Path,
        default=None,
        help=(
            "write an evidence JSON with the computed ldr_diff to this path "
            "(generation mode); overwrites recorded ldr_diff with computed values"
        ),
    )
    args = parser.parse_args()

    try:
        data = json.loads(args.results.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"[visual-diff] FORMAT FAIL could not read input: {exc}", file=sys.stderr)
        return 1

    ok, errors = validate_document(data)
    if not ok:
        for error in errors:
            print(f"[visual-diff] FORMAT FAIL {error}", file=sys.stderr)
        return 1
    print("[visual-diff] FORMAT PASS visual diff evidence document is valid")

    if args.validate_only:
        return 0

    assert isinstance(data, dict)
    write_output = args.write_output is not None
    skip_count, diff_computed, mismatch_count, pass_uncomputable_count = evaluate_frames(
        data, write_output=write_output
    )
    print(
        f"[visual-diff] frames_skipped={skip_count} frames_with_ldr_diff={diff_computed} "
        f"pass_diff_failed={pass_uncomputable_count}"
    )
    if write_output:
        if pass_uncomputable_count > 0:
            print(
                f"[visual-diff] DIFF FAIL {pass_uncomputable_count} pass frame(s) could "
                "not compute an LDR diff; refusing to write evidence",
                file=sys.stderr,
            )
            return 1
        args.write_output.parent.mkdir(parents=True, exist_ok=True)
        args.write_output.write_text(
            json.dumps(data, indent=2, ensure_ascii=True) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        print(f"[visual-diff] wrote evidence with computed ldr_diff to {args.write_output}")
        return 0
    if mismatch_count > 0 or pass_uncomputable_count > 0:
        if mismatch_count > 0:
            print(
                f"[visual-diff] DIFF FAIL recorded ldr_diff mismatched computed value "
                f"in {mismatch_count} frame(s)",
                file=sys.stderr,
            )
        if pass_uncomputable_count > 0:
            print(
                f"[visual-diff] DIFF FAIL {pass_uncomputable_count} status=pass frame(s) "
                "could not compute an LDR diff",
                file=sys.stderr,
            )
        return 1
    if diff_computed == 0:
        print(
            "[visual-diff] SCAFFOLD no LDR diff computed; visual verification is NOT done "
            "(no reference+candidate frame files present)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
