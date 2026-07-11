#!/usr/bin/env python3
"""Aggregate baseline evidence + a rurix-leg run summary into perf_gate input.

Reads:
  * a baseline evidence document (schemas/baseline_evidence.schema.json,
    kind=baseline, run_mode=full) produced by aggregate_baseline_evidence.py, and
  * a rurix-leg runner summary (target/grx/godot_bench_runner_summary.json,
    leg=rurix, run_mode=full) produced by run_benchmark_scenes.py --leg rurix.

Emits a strict perf_gate input document (schemas/perf_gate_input.schema.json)
comparing baseline vs rurix per scene. The output is self-validated through
perf_gate.py --kind perf_gate --strict --validate-only before success.

This tool records MEASURED numbers only; it makes no performance claim and does
NOT run the pass/fail threshold evaluation. It is fail-closed on the engagement
gate: if the rurix leg carries a non-empty pass matrix but the passes it enables
did not actually run for every sampled frame (pass_engagement null or recorded
short of sample_frames), no comparison document is emitted (invalid over fake).
Dev/iter runs (run_mode != full) are rejected outright.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BENCH_DIR = Path(__file__).resolve().parent
PERF_GATE_SCRIPT = BENCH_DIR / "perf_gate.py"
DEFAULT_RURIX_SUMMARY_PATH = ROOT / "target" / "grx" / "godot_bench_runner_summary.json"
DEFAULT_OUTPUT_PATH = ROOT / "target" / "grx" / "perf_gate_input.json"
DEFAULT_ENGAGEMENT_DIAGNOSTIC_PATH = ROOT / "target" / "grx" / "perf_gate_engagement_gate.json"

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
EVIDENCE_LEVEL = "measured_local"
RESOLUTION = [1920, 1080]
FULL_WARMUP_FRAMES = 300
FULL_SAMPLE_FRAMES = 2000


def load_json_object(path: Path) -> dict[str, object]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return payload


def repo_relative_posix(path: Path) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(ROOT).as_posix()
    except ValueError:
        return resolved.as_posix()


def validate_baseline(baseline: dict[str, object]) -> dict[str, dict[str, object]]:
    if baseline.get("kind") != "baseline":
        raise ValueError("baseline evidence kind must be 'baseline'")
    if baseline.get("run_mode") != "full":
        raise ValueError("baseline evidence run_mode must be 'full'")
    if baseline.get("evidence_level") != EVIDENCE_LEVEL:
        raise ValueError(f"baseline evidence_level must be '{EVIDENCE_LEVEL}'")
    scenes = baseline.get("scenes")
    if not isinstance(scenes, list) or [s.get("name") for s in scenes] != EXPECTED_SCENES:
        raise ValueError(
            "baseline evidence must cover the seven fixed scenes in order: "
            + ", ".join(EXPECTED_SCENES)
        )
    by_name: dict[str, dict[str, object]] = {}
    for scene in scenes:
        assert isinstance(scene, dict)
        by_name[str(scene["name"])] = scene
    return by_name


def validate_rurix_summary(summary: dict[str, object]) -> None:
    if summary.get("status") != "success":
        raise ValueError("rurix runner summary status must be success")
    if summary.get("leg") != "rurix":
        raise ValueError("rurix runner summary leg must be 'rurix'")
    if summary.get("run_mode") != "full":
        raise ValueError(
            "rurix runner summary run_mode must be 'full'; dev/iter/quick_smoke "
            "runs are not eligible for the strict perf gate"
        )
    if summary.get("evidence_level") != EVIDENCE_LEVEL:
        raise ValueError(f"rurix runner summary evidence_level must be '{EVIDENCE_LEVEL}'")
    if summary.get("target_backend") != TARGET_BACKEND:
        raise ValueError(f"rurix runner summary target_backend must be '{TARGET_BACKEND}'")
    if summary.get("resolution") != RESOLUTION:
        raise ValueError("rurix runner summary resolution must be [1920, 1080]")
    if summary.get("vsync") is not False:
        raise ValueError("rurix runner summary vsync must be false")
    if summary.get("warmup_frames") != FULL_WARMUP_FRAMES:
        raise ValueError(f"rurix runner summary warmup_frames must be {FULL_WARMUP_FRAMES}")
    if summary.get("sample_frames") != FULL_SAMPLE_FRAMES:
        raise ValueError(f"rurix runner summary sample_frames must be {FULL_SAMPLE_FRAMES}")
    if summary.get("failure_count") != 0:
        raise ValueError("rurix runner summary failure_count must be 0")
    if summary.get("scene_names") != EXPECTED_SCENES:
        raise ValueError(
            "rurix runner summary scene_names must be the seven fixed scenes in order"
        )
    pass_matrix = summary.get("pass_matrix")
    if not isinstance(pass_matrix, dict) or not pass_matrix:
        raise ValueError(
            "rurix runner summary pass_matrix must be non-empty (a rurix leg with "
            "no pass matrix has nothing to compare)"
        )


def expected_passes(pass_matrix: dict[str, object]) -> list[str]:
    if pass_matrix.get("rendering/rurix_accel/enabled") is not True:
        return []
    passes: list[str] = []
    for name in ("luminance_reduction", "tonemap"):
        if pass_matrix.get(f"rendering/rurix_accel/passes/{name}/enabled") is True:
            passes.append(name)
    return passes


def evaluate_engagement(
    pass_matrix: dict[str, object],
    rurix_raw_by_name: dict[str, dict[str, object]],
) -> dict[str, object]:
    """Return the engagement-gate verdict for the rurix leg."""
    passes = expected_passes(pass_matrix)
    reasons: list[str] = []
    per_scene: dict[str, object] = {}
    if not passes:
        reasons.append(
            "pass matrix enables no rurix pass "
            "(need rendering/rurix_accel/enabled + a passes/<name>/enabled)"
        )
    for scene_name in EXPECTED_SCENES:
        raw = rurix_raw_by_name.get(scene_name, {})
        engagement = raw.get("pass_engagement")
        per_scene[scene_name] = engagement
        if not passes:
            continue
        if engagement is None:
            reasons.append(f"{scene_name}: pass_engagement is null (no engagement markers observed)")
            continue
        if not isinstance(engagement, dict):
            reasons.append(f"{scene_name}: pass_engagement is not an object")
            continue
        for pass_name in passes:
            entry = engagement.get(pass_name)
            recorded = entry.get("recorded") if isinstance(entry, dict) else None
            if not isinstance(recorded, int) or recorded < FULL_SAMPLE_FRAMES:
                reasons.append(
                    f"{scene_name}/{pass_name}: recorded={recorded} "
                    f"< sample_frames={FULL_SAMPLE_FRAMES}"
                )
    return {
        "expected_passes": passes,
        "per_scene_engagement": per_scene,
        "engagement_valid": not reasons,
        "reasons": reasons,
    }


def build_perf_gate_input(
    baseline_by_name: dict[str, dict[str, object]],
    summary: dict[str, object],
    rurix_raw_by_name: dict[str, dict[str, object]],
    engagement: dict[str, object],
) -> dict[str, object]:
    per_scene_results = summary.get("per_scene_results")
    assert isinstance(per_scene_results, list)
    rurix_by_name = {str(item["scene_name"]): item for item in per_scene_results}

    scenes: list[dict[str, object]] = []
    for scene_name in EXPECTED_SCENES:
        base = baseline_by_name[scene_name]
        rurix = rurix_by_name[scene_name]
        rurix_raw_path = Path(str(rurix["raw_json_path"]))
        scenes.append(
            {
                "name": scene_name,
                "baseline_fps": float(base["avg_fps"]),
                "rurix_fps": float(rurix["avg_fps"]),
                "baseline_p95_ms": float(base["p95_frame_time_ms"]),
                "rurix_p95_ms": float(rurix["p95_frame_time_ms"]),
                "baseline_raw_artifact_path": str(base["raw_artifact_path"]),
                "rurix_raw_artifact_path": repo_relative_posix(rurix_raw_path),
            }
        )

    return {
        "kind": "perf_gate",
        "evidence_level": EVIDENCE_LEVEL,
        "run_mode": "full",
        "target_backend": TARGET_BACKEND,
        "resolution": RESOLUTION,
        "vsync": False,
        "warmup_frames": FULL_WARMUP_FRAMES,
        "sample_frames": FULL_SAMPLE_FRAMES,
        "aggregator": "spike/godot-rurix/bench/aggregate_perf_gate_input.py",
        "baseline_run_id": None,
        "rurix_run_id": summary.get("run_id"),
        "leg_pass_matrix": summary.get("pass_matrix"),
        "engagement_valid": engagement["engagement_valid"],
        "engagement_expected_passes": engagement["expected_passes"],
        "note": (
            "GRX perf_gate comparison input: measured_local full baseline vs a "
            "rurix leg (same seven scenes, same 300/2000 sampling). MEASURED "
            "numbers only; this document makes no performance claim and does not "
            "itself run the pass/fail threshold evaluation. The rurix pass "
            "engagement was validated (every enabled pass recorded on every "
            "sampled frame) before this input was emitted."
        ),
        "scenes": scenes,
    }


def self_validate(output_path: Path) -> bool:
    gate = subprocess.run(
        [
            sys.executable,
            str(PERF_GATE_SCRIPT),
            "--kind",
            "perf_gate",
            "--strict",
            "--validate-only",
            str(output_path),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    gate_output = (gate.stdout + gate.stderr).strip()
    if gate_output:
        print(gate_output)
    return gate.returncode == 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=Path, required=True, help="baseline evidence JSON")
    parser.add_argument("--rurix-summary", type=Path, default=DEFAULT_RURIX_SUMMARY_PATH)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT_PATH)
    parser.add_argument(
        "--engagement-diagnostic",
        type=Path,
        default=DEFAULT_ENGAGEMENT_DIAGNOSTIC_PATH,
        help="where to write the engagement-gate diagnostic when the gate fails",
    )
    args = parser.parse_args()

    try:
        baseline = load_json_object(args.baseline)
        summary = load_json_object(args.rurix_summary)
        baseline_by_name = validate_baseline(baseline)
        validate_rurix_summary(summary)

        per_scene_results = summary.get("per_scene_results")
        if not isinstance(per_scene_results, list) or len(per_scene_results) != len(EXPECTED_SCENES):
            raise ValueError("rurix runner summary per_scene_results must contain seven scenes")

        rurix_raw_by_name: dict[str, dict[str, object]] = {}
        for item in per_scene_results:
            if not isinstance(item, dict):
                raise ValueError("each per_scene_results entry must be a JSON object")
            if item.get("status") != "success":
                raise ValueError(f"{item.get('scene_name')}: rurix per-scene status must be success")
            raw_path = Path(str(item.get("raw_json_path")))
            if not raw_path.is_file():
                raise FileNotFoundError(f"rurix raw artifact missing: {raw_path}")
            rurix_raw_by_name[str(item["scene_name"])] = load_json_object(raw_path)

        pass_matrix = summary.get("pass_matrix")
        assert isinstance(pass_matrix, dict)
        engagement = evaluate_engagement(pass_matrix, rurix_raw_by_name)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"[perf-gate-input] FAIL {exc}", file=sys.stderr)
        return 1

    if not engagement["engagement_valid"]:
        diagnostic = {
            "status": "engagement_invalid",
            "leg": "rurix",
            "rurix_run_id": summary.get("run_id"),
            "pass_matrix": pass_matrix,
            "engagement": engagement,
            "note": (
                "rurix pass engagement gate failed: the pass matrix is non-empty "
                "but the passes it enables did not run for every sampled frame. No "
                "perf_gate comparison input was emitted (invalid over fake)."
            ),
        }
        args.engagement_diagnostic.parent.mkdir(parents=True, exist_ok=True)
        args.engagement_diagnostic.write_text(
            json.dumps(diagnostic, indent=2, ensure_ascii=True) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        print(
            "[perf-gate-input] FAIL rurix engagement gate invalid; no comparison "
            "input emitted (engagement_valid=false)",
            file=sys.stderr,
        )
        for reason in engagement["reasons"]:
            print(f"  - {reason}", file=sys.stderr)
        print(f"[perf-gate-input] engagement diagnostic: {args.engagement_diagnostic}")
        return 1

    document = build_perf_gate_input(baseline_by_name, summary, rurix_raw_by_name, engagement)
    document["baseline_run_id"] = baseline.get("run_id")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[perf-gate-input] wrote {args.output}")

    if not self_validate(args.output):
        print(
            "[perf-gate-input] FAIL emitted document did not pass "
            "perf_gate.py --kind perf_gate --strict --validate-only",
            file=sys.stderr,
        )
        return 1
    print(
        "[perf-gate-input] PASS perf_gate comparison input aggregated and validated "
        "(measured numbers only; no performance claim; threshold evaluation not run)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
