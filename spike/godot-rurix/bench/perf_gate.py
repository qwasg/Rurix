#!/usr/bin/env python3
"""Evaluate the Godot/Rurix performance gate and validate evidence formats.

Two evidence document kinds are supported:

- ``baseline`` (schemas/baseline_evidence.schema.json): aggregated measured_local
  evidence for one benchmark run (baseline or rurix). ``quick_smoke`` documents
  are smoke evidence only and MUST NOT be used as strict perf_gate input.

- ``perf_gate`` (schemas/perf_gate_input.schema.json): strict close-out input
  comparing baseline vs rurix per scene, with raw artifact paths.

perf_gate input shape (default kind)::

    {
      "evidence_level": "measured_local",
      "run_mode": "full",
      "target_backend": "Godot 4.7-dev Windows D3D12 Forward+",
      "resolution": [1920, 1080],
      "vsync": false,
      "warmup_frames": 300,
      "sample_frames": 2000,
      "scenes": [
        {
          "name": "clustered_lights",
          "baseline_fps": 60.0,
          "rurix_fps": 95.0,
          "baseline_p95_ms": 22.0,
          "rurix_p95_ms": 14.0,
          "baseline_raw_artifact_path": "target/grx/.../baseline/clustered_lights.json",
          "rurix_raw_artifact_path": "target/grx/.../rurix/clustered_lights.json"
        }
      ]
    }

--strict rejects SKIP/estimated markers, quick_smoke, non-full run_mode,
missing scenes, and missing raw artifact paths.
"""

from __future__ import annotations

import argparse
import json
import math
import pathlib
import re
import sys


MIN_GEOMEAN_FPS_RATIO = 1.5
MIN_P95_REDUCTION = 0.30
MIN_SINGLE_SCENE_RATIO = 0.95

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
EVIDENCE_LEVEL = "measured_local"
FULL_WARMUP = 300
FULL_SAMPLE = 2000
# Standalone SKIP/skipped/estimated markers anywhere in a strict document are
# forbidden. Underscore is treated as a word character (via explicit lookarounds
# instead of \b) so path/word fragments like "spike", "skipper", or
# "quick_smoke" are not falsely flagged, while embedded markers such as
# "status=SKIP", "skip-reason", or "estimated:true" are caught.
FORBIDDEN_MARKER_RE = re.compile(
    r"(?<![0-9A-Za-z_])(?:skip|skipped|estimated)(?![0-9A-Za-z_])",
    re.IGNORECASE,
)
FORBIDDEN_MARKER_DESC = "SKIP/skipped/estimated"


def geomean(values: list[float]) -> float:
    return math.exp(sum(math.log(v) for v in values) / len(values))


def is_positive_number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and float(value) > 0.0


def non_empty_string(value: object) -> bool:
    return isinstance(value, str) and value.strip() != ""


def scene_names_in_order(scenes: list[object]) -> list[str]:
    names: list[str] = []
    for scene in scenes:
        if isinstance(scene, dict) and isinstance(scene.get("name"), str):
            names.append(scene["name"])
    return names


def contains_forbidden_token(data: object) -> str | None:
    """Return the first SKIP/estimated marker found anywhere in the document."""
    if isinstance(data, str):
        match = FORBIDDEN_MARKER_RE.search(data)
        return match.group(0) if match else None
    if isinstance(data, dict):
        for key, value in data.items():
            if isinstance(key, str):
                key_match = FORBIDDEN_MARKER_RE.search(key)
                if key_match:
                    return key_match.group(0)
            found = contains_forbidden_token(value)
            if found:
                return found
        return None
    if isinstance(data, list):
        for item in data:
            found = contains_forbidden_token(item)
            if found:
                return found
        return None
    return None


def validate_baseline_evidence(data: object) -> tuple[bool, list[str], bool]:
    """Validate a baseline evidence document.

    Returns (ok, errors, smoke_only). smoke_only is True for quick_smoke
    documents, which are parseable but not eligible for strict perf gate input.
    """
    errors: list[str] = []
    smoke_only = False
    if not isinstance(data, dict):
        return False, ["document root must be a JSON object"], smoke_only

    run_mode = data.get("run_mode")
    if run_mode not in ("full", "quick_smoke"):
        errors.append("run_mode must be 'full' or 'quick_smoke'")
    if run_mode == "quick_smoke":
        smoke_only = True

    if data.get("evidence_level") != EVIDENCE_LEVEL:
        errors.append(f"evidence_level must be '{EVIDENCE_LEVEL}'")
    if data.get("target_backend") != TARGET_BACKEND:
        errors.append(f"target_backend must be '{TARGET_BACKEND}'")
    if data.get("resolution") != RESOLUTION:
        errors.append("resolution must be [1920, 1080]")
    if data.get("vsync") is not False:
        errors.append("vsync must be false")

    warmup = data.get("warmup_frames")
    sample = data.get("sample_frames")
    if not isinstance(warmup, int) or isinstance(warmup, bool) or warmup <= 0:
        errors.append("warmup_frames must be a positive integer")
    if not isinstance(sample, int) or isinstance(sample, bool) or sample <= 0:
        errors.append("sample_frames must be a positive integer")
    if run_mode == "full":
        if warmup != FULL_WARMUP:
            errors.append(f"full baseline requires warmup_frames={FULL_WARMUP}")
        if sample != FULL_SAMPLE:
            errors.append(f"full baseline requires sample_frames={FULL_SAMPLE}")

    scenes = data.get("scenes")
    if not isinstance(scenes, list):
        errors.append("scenes must be a list")
    else:
        names = scene_names_in_order(scenes)
        if names != EXPECTED_SCENES:
            errors.append(
                "scenes must cover the seven fixed scenes in order: "
                + ", ".join(EXPECTED_SCENES)
            )
        for scene in scenes:
            if not isinstance(scene, dict):
                errors.append("each scene must be a JSON object")
                continue
            name = scene.get("name", "<unknown>")
            if not is_positive_number(scene.get("avg_fps")):
                errors.append(f"{name}: avg_fps must be a positive number")
            if not is_positive_number(scene.get("p95_frame_time_ms")):
                errors.append(f"{name}: p95_frame_time_ms must be a positive number")
            sample_count = scene.get("sample_count")
            if not isinstance(sample_count, int) or isinstance(sample_count, bool) or sample_count <= 0:
                errors.append(f"{name}: sample_count must be a positive integer")
            elif isinstance(sample, int) and not isinstance(sample, bool) and sample > 0 and sample_count != sample:
                errors.append(f"{name}: sample_count must equal sample_frames ({sample})")
            if not non_empty_string(scene.get("raw_artifact_path")):
                errors.append(f"{name}: raw_artifact_path must be a traceable non-empty string")

    return (not errors), errors, smoke_only


def validate_perf_gate_input(data: object, strict: bool) -> tuple[bool, list[str]]:
    """Validate strict perf gate input format."""
    errors: list[str] = []
    if not isinstance(data, dict):
        return False, ["document root must be a JSON object"]

    if strict:
        forbidden = contains_forbidden_token(data)
        if forbidden:
            errors.append(
                f"strict mode rejects {FORBIDDEN_MARKER_DESC} evidence (found '{forbidden}')"
            )
        thresholds = data.get("thresholds")
        if thresholds is not None:
            if not isinstance(thresholds, dict):
                errors.append("thresholds must be a JSON object")
            else:
                fixed_thresholds = {
                    "geomean_fps_ratio_min": MIN_GEOMEAN_FPS_RATIO,
                    "p95_frame_time_reduction_min": MIN_P95_REDUCTION,
                    "single_scene_fps_ratio_min": MIN_SINGLE_SCENE_RATIO,
                }
                for key, fixed in fixed_thresholds.items():
                    if key in thresholds and thresholds[key] != fixed:
                        errors.append(
                            f"strict mode requires thresholds.{key}={fixed} "
                            f"(got {thresholds[key]!r})"
                        )

    run_mode = data.get("run_mode")
    if run_mode != "full":
        errors.append("run_mode must be 'full' (quick_smoke is not eligible for strict perf gate)")
    if data.get("evidence_level") != EVIDENCE_LEVEL:
        errors.append(f"evidence_level must be '{EVIDENCE_LEVEL}'")
    if data.get("target_backend") != TARGET_BACKEND:
        errors.append(f"target_backend must be '{TARGET_BACKEND}'")
    if data.get("resolution") != RESOLUTION:
        errors.append("resolution must be [1920, 1080]")
    if data.get("vsync") is not False:
        errors.append("vsync must be false")
    if data.get("warmup_frames") != FULL_WARMUP:
        errors.append(f"warmup_frames must be {FULL_WARMUP}")
    if data.get("sample_frames") != FULL_SAMPLE:
        errors.append(f"sample_frames must be {FULL_SAMPLE}")

    scenes = data.get("scenes")
    if not isinstance(scenes, list) or not scenes:
        errors.append("scenes must be a non-empty list")
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
        for key in ("baseline_fps", "rurix_fps", "baseline_p95_ms", "rurix_p95_ms"):
            if not is_positive_number(scene.get(key)):
                errors.append(f"{name}: {key} must be a positive number")
        for key in ("baseline_raw_artifact_path", "rurix_raw_artifact_path"):
            if not non_empty_string(scene.get(key)):
                errors.append(f"{name}: {key} must be a traceable non-empty string")

    return (not errors), errors


def evaluate_perf_gate(data: dict[str, object]) -> tuple[bool, list[str], float, float]:
    """Run the three threshold checks on validated perf gate input."""
    scenes = data["scenes"]
    assert isinstance(scenes, list)
    fps_ratios: list[float] = []
    p95_reductions: list[float] = []
    failures: list[str] = []

    for scene in scenes:
        assert isinstance(scene, dict)
        name = scene["name"]
        baseline_fps = float(scene["baseline_fps"])
        rurix_fps = float(scene["rurix_fps"])
        baseline_p95 = float(scene["baseline_p95_ms"])
        rurix_p95 = float(scene["rurix_p95_ms"])
        fps_ratio = rurix_fps / baseline_fps
        p95_reduction = (baseline_p95 - rurix_p95) / baseline_p95
        fps_ratios.append(fps_ratio)
        p95_reductions.append(p95_reduction)
        if fps_ratio < MIN_SINGLE_SCENE_RATIO:
            failures.append(f"{name}: fps ratio {fps_ratio:.3f} < {MIN_SINGLE_SCENE_RATIO}")

    fps_geomean = geomean(fps_ratios)
    p95_mean_reduction = sum(p95_reductions) / len(p95_reductions)
    if fps_geomean < MIN_GEOMEAN_FPS_RATIO:
        failures.append(f"geomean fps ratio {fps_geomean:.3f} < {MIN_GEOMEAN_FPS_RATIO}")
    if p95_mean_reduction < MIN_P95_REDUCTION:
        failures.append(f"mean p95 reduction {p95_mean_reduction:.3f} < {MIN_P95_REDUCTION}")

    return (not failures), failures, fps_geomean, p95_mean_reduction


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=pathlib.Path)
    parser.add_argument(
        "--kind",
        choices=("perf_gate", "baseline"),
        default="perf_gate",
        help="which evidence schema to validate against (default: perf_gate)",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="enable strict rejection rules for close-out perf gate input",
    )
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="only validate schema/format; do not evaluate performance thresholds",
    )
    args = parser.parse_args()

    try:
        data = json.loads(args.results.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"[perf_gate] FORMAT FAIL could not read input: {exc}", file=sys.stderr)
        return 1

    if args.kind == "baseline":
        ok, errors, smoke_only = validate_baseline_evidence(data)
        if not ok:
            for error in errors:
                print(f"[perf_gate] FORMAT FAIL {error}", file=sys.stderr)
            return 1
        if smoke_only:
            print(
                "[perf_gate] baseline evidence is quick_smoke: smoke evidence only, "
                "NOT eligible for strict perf gate input"
            )
        print("[perf_gate] FORMAT PASS baseline evidence document is valid")
        return 0

    ok, errors = validate_perf_gate_input(data, args.strict)
    if not ok:
        for error in errors:
            print(f"[perf_gate] FORMAT FAIL {error}", file=sys.stderr)
        return 1
    print("[perf_gate] FORMAT PASS perf gate input document is valid")

    if args.validate_only:
        return 0

    assert isinstance(data, dict)
    passed, failures, fps_geomean, p95_mean_reduction = evaluate_perf_gate(data)
    print(
        f"[perf_gate] fps_geomean={fps_geomean:.3f} "
        f"p95_mean_reduction={p95_mean_reduction:.3f}"
    )
    if not passed:
        for failure in failures:
            print(f"[perf_gate] PERF FAIL {failure}", file=sys.stderr)
        return 1
    print("[perf_gate] PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
