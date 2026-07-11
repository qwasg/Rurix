#!/usr/bin/env python3
"""Aggregate a full measured_local Godot bench run into GRX baseline evidence.

Reads the tracked runner summary (target/grx/godot_bench_runner_summary.json)
plus the per-scene raw JSON artifacts it references, cross-checks them, and
writes an aggregated baseline evidence document that conforms to
schemas/baseline_evidence.schema.json (kind=baseline, run_mode=full,
evidence_level=measured_local, seven fixed scenes in order, per-scene
avg_fps / p95_frame_time_ms / sample_count / raw_artifact_path).

This is BASELINE evidence only: it records what the unmodified tracked Godot
build measured on this machine. It makes NO comparison against any Rurix
configuration and NO performance-improvement claim of any kind. The aggregated
document is self-validated through perf_gate.py --kind baseline --strict
--validate-only before this tool reports success.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BENCH_DIR = Path(__file__).resolve().parent
PERF_GATE_SCRIPT = BENCH_DIR / "perf_gate.py"
DEFAULT_SUMMARY_PATH = ROOT / "target" / "grx" / "godot_bench_runner_summary.json"
BASELINE_DIR = BENCH_DIR / "baseline"

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


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


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


def validate_summary(summary: dict[str, object]) -> None:
    if summary.get("status") != "success":
        raise ValueError("runner summary status must be success")
    if summary.get("run_mode") != "full":
        raise ValueError(
            "runner summary run_mode must be 'full'; quick_smoke runs are smoke "
            "evidence only and cannot be aggregated into baseline evidence"
        )
    if summary.get("evidence_level") != EVIDENCE_LEVEL:
        raise ValueError(f"runner summary evidence_level must be '{EVIDENCE_LEVEL}'")
    if summary.get("target_backend") != TARGET_BACKEND:
        raise ValueError(f"runner summary target_backend must be '{TARGET_BACKEND}'")
    if summary.get("resolution") != RESOLUTION:
        raise ValueError("runner summary resolution must be [1920, 1080]")
    if summary.get("vsync") is not False:
        raise ValueError("runner summary vsync must be false")
    if summary.get("warmup_frames") != FULL_WARMUP_FRAMES:
        raise ValueError(f"full baseline requires warmup_frames={FULL_WARMUP_FRAMES}")
    if summary.get("sample_frames") != FULL_SAMPLE_FRAMES:
        raise ValueError(f"full baseline requires sample_frames={FULL_SAMPLE_FRAMES}")
    if summary.get("failure_count") != 0:
        raise ValueError("runner summary failure_count must be 0")
    if summary.get("scene_names") != EXPECTED_SCENES:
        raise ValueError(
            "runner summary scene_names must be the seven fixed scenes in order: "
            + ", ".join(EXPECTED_SCENES)
        )


def validate_raw_payload(
    payload: dict[str, object], scene_name: str, scene_result: dict[str, object]
) -> None:
    checks: list[tuple[str, object, object]] = [
        ("status", payload.get("status"), "success"),
        ("scene_name", payload.get("scene_name"), scene_name),
        ("run_mode", payload.get("run_mode"), "full"),
        ("evidence_level", payload.get("evidence_level"), EVIDENCE_LEVEL),
        ("target_backend", payload.get("target_backend"), TARGET_BACKEND),
        ("resolution", payload.get("resolution"), RESOLUTION),
        ("vsync", payload.get("vsync"), False),
        ("warmup_frames", payload.get("warmup_frames"), FULL_WARMUP_FRAMES),
        ("sample_frames", payload.get("sample_frames"), FULL_SAMPLE_FRAMES),
        ("sample_count", payload.get("sample_count"), FULL_SAMPLE_FRAMES),
        ("avg_fps", payload.get("avg_fps"), scene_result.get("avg_fps")),
        (
            "p95_frame_time_ms",
            payload.get("p95_frame_time_ms"),
            scene_result.get("p95_frame_time_ms"),
        ),
    ]
    for key, actual, expected in checks:
        if actual != expected:
            raise ValueError(
                f"{scene_name}: raw payload {key} mismatch "
                f"(raw={actual!r}, expected={expected!r})"
            )
    frame_times = payload.get("frame_times_ms")
    if not isinstance(frame_times, list) or len(frame_times) != FULL_SAMPLE_FRAMES:
        raise ValueError(
            f"{scene_name}: raw payload frame_times_ms must contain "
            f"{FULL_SAMPLE_FRAMES} samples"
        )


def aggregate(summary_path: Path) -> dict[str, object]:
    summary = load_json_object(summary_path)
    validate_summary(summary)

    per_scene_results = summary.get("per_scene_results")
    if not isinstance(per_scene_results, list) or len(per_scene_results) != len(
        EXPECTED_SCENES
    ):
        raise ValueError("runner summary per_scene_results must contain seven scenes")

    scenes: list[dict[str, object]] = []
    for expected_name, scene_result in zip(EXPECTED_SCENES, per_scene_results):
        if not isinstance(scene_result, dict):
            raise ValueError("each per_scene_results entry must be a JSON object")
        scene_name = scene_result.get("scene_name")
        if scene_name != expected_name:
            raise ValueError(
                f"per_scene_results order mismatch: expected {expected_name}, "
                f"got {scene_name!r}"
            )
        if scene_result.get("status") != "success":
            raise ValueError(f"{expected_name}: per-scene status must be success")
        raw_path = Path(str(scene_result.get("raw_json_path")))
        if not raw_path.is_file():
            raise FileNotFoundError(f"{expected_name}: raw artifact missing: {raw_path}")
        raw_payload = load_json_object(raw_path)
        validate_raw_payload(raw_payload, expected_name, scene_result)
        scenes.append(
            {
                "name": expected_name,
                "avg_fps": float(raw_payload["avg_fps"]),
                "p95_frame_time_ms": float(raw_payload["p95_frame_time_ms"]),
                "sample_count": int(raw_payload["sample_count"]),
                "raw_artifact_path": repo_relative_posix(raw_path),
                "raw_artifact_sha256": sha256_file(raw_path),
            }
        )

    return {
        "kind": "baseline",
        "run_mode": "full",
        "evidence_level": EVIDENCE_LEVEL,
        "target_backend": TARGET_BACKEND,
        "resolution": RESOLUTION,
        "vsync": False,
        "warmup_frames": FULL_WARMUP_FRAMES,
        "sample_frames": FULL_SAMPLE_FRAMES,
        "run_id": summary.get("run_id"),
        "runner_summary_path": repo_relative_posix(summary_path),
        "aggregator": "spike/godot-rurix/bench/aggregate_baseline_evidence.py",
        "note": (
            "GRX full baseline evidence aggregated from one measured_local run of "
            "the unmodified tracked Godot build (run_id above; raw per-scene JSON "
            "artifacts hash-pinned below). BASELINE ONLY: no Rurix configuration "
            "was measured, no comparison is recorded, and no performance claim of "
            "any kind is made by this document."
        ),
        "scenes": scenes,
    }


def run_id_date(run_id: object) -> str | None:
    if isinstance(run_id, str) and len(run_id) >= 8 and run_id[:8].isdigit():
        return run_id[:8]
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", type=Path, default=DEFAULT_SUMMARY_PATH)
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help=(
            "output baseline evidence path (default: "
            "spike/godot-rurix/bench/baseline/baseline_full_<run date>.json)"
        ),
    )
    args = parser.parse_args()

    try:
        document = aggregate(args.summary)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"[baseline-aggregate] FAIL {exc}", file=sys.stderr)
        return 1

    output_path = args.output
    if output_path is None:
        date = run_id_date(document.get("run_id")) or "unknown_date"
        output_path = BASELINE_DIR / f"baseline_full_{date}.json"

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(document, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[baseline-aggregate] wrote {output_path}")

    gate = subprocess.run(
        [
            sys.executable,
            str(PERF_GATE_SCRIPT),
            "--kind",
            "baseline",
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
    if gate.returncode != 0:
        print(
            "[baseline-aggregate] FAIL generated evidence did not pass "
            "perf_gate.py --kind baseline --strict --validate-only",
            file=sys.stderr,
        )
        return 1
    print(
        "[baseline-aggregate] PASS baseline evidence aggregated and validated "
        "(baseline only; no comparison, no performance claim)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
