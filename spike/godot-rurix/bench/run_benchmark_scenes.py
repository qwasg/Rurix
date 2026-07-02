#!/usr/bin/env python3
"""Run the tracked GRX-005 Godot benchmark scenes and collect raw frame samples."""

from __future__ import annotations

import argparse
import json
import math
import subprocess
from datetime import UTC, datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BENCH_DIR = Path(__file__).resolve().parent
DEFAULT_MANIFEST_PATH = BENCH_DIR / "bench_manifest.json"
TARGET_GRX_DIR = ROOT / "target" / "grx"
DEFAULT_PROJECT_SUMMARY_PATH = TARGET_GRX_DIR / "godot_bench_project_summary.json"
RUNNER_SUMMARY_PATH = TARGET_GRX_DIR / "godot_bench_runner_summary.json"
RUNS_DIR = TARGET_GRX_DIR / "godot-bench-runs"
GODOT_CONSOLE_EXE = (
    ROOT / "external" / "godot-master" / "bin" / "godot.windows.template_debug.x86_64.console.exe"
)
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
QUICK_SMOKE_WARMUP_FRAMES = 30
QUICK_SMOKE_SAMPLE_FRAMES = 60
TIMEOUT_SECONDS = 1800

# Godot log failure-marker rules aligned with bench_project_smoke.py. The runner
# does not pass --verbose, so it only reuses marker detection and the global
# script cache allowlist; it does not require "Loading resource:" load evidence.
FAILURE_MARKERS = (
    "SCRIPT ERROR:",
    "Parser Error:",
    "Parse Error:",
    "Failed loading resource:",
    "Failed loading script",
)
ALLOWLISTED_GLOBAL_SCRIPT_CACHE_ERROR = "ERROR: Could not load global script cache."
ALLOWLISTED_GLOBAL_SCRIPT_CACHE_CONTEXT = "at: ProjectSettings::get_global_class_list"


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def load_json_object(path: Path) -> dict[str, object]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return payload


def normalize_output(text: str) -> str:
    return text.replace("\r\n", "\n")


def combined_output(proc: subprocess.CompletedProcess[str]) -> str:
    pieces: list[str] = []
    if proc.stdout:
        pieces.append(proc.stdout.rstrip())
    if proc.stderr:
        pieces.append(proc.stderr.rstrip())
    return "\n".join(piece for piece in pieces if piece).strip()


def scan_log_markers(output: str) -> dict[str, list[str]]:
    """Scan Godot output for failure markers, aligned with bench_project_smoke.py.

    The allowlisted global script cache error (and its `at:` context line) is
    recorded as a warning; any other bare ERROR or a known failure marker is
    treated as a failure marker.
    """
    lines = normalize_output(output).splitlines()
    failure_markers: list[str] = []
    warnings: list[str] = []

    index = 0
    while index < len(lines):
        line = lines[index].strip()
        if not line:
            index += 1
            continue

        if line == ALLOWLISTED_GLOBAL_SCRIPT_CACHE_ERROR:
            warning_lines = [line]
            if index + 1 < len(lines):
                next_line = lines[index + 1].strip()
                if next_line.startswith(ALLOWLISTED_GLOBAL_SCRIPT_CACHE_CONTEXT):
                    warning_lines.append(next_line)
                    index += 1
            warnings.extend(warning_lines)
            index += 1
            continue

        if any(marker in line for marker in FAILURE_MARKERS) or "ERROR:" in line:
            failure_markers.append(line)

        index += 1

    return {"failure_markers": failure_markers, "warnings": warnings}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST_PATH)
    parser.add_argument("--project-summary", type=Path, default=DEFAULT_PROJECT_SUMMARY_PATH)
    parser.add_argument("--project-dir", type=Path)
    parser.add_argument("--quick-smoke", action="store_true")
    return parser.parse_args()


def load_manifest(manifest_path: Path) -> dict[str, object]:
    manifest = load_json_object(manifest_path)
    scenes = manifest.get("scenes")
    if scenes != EXPECTED_SCENES:
        raise ValueError(
            "manifest.scenes must exactly match the fixed GRX-005 scene set: "
            + ", ".join(EXPECTED_SCENES)
        )
    resolution = manifest.get("resolution")
    if resolution != [1920, 1080]:
        raise ValueError("manifest.resolution must remain [1920, 1080]")
    if manifest.get("vsync") is not False:
        raise ValueError("manifest.vsync must remain false")
    for key in ("warmup_frames", "sample_frames"):
        value = manifest.get(key)
        if not isinstance(value, int) or value <= 0:
            raise ValueError(f"manifest.{key} must be a positive integer")
    return manifest


def load_project_summary(summary_path: Path) -> dict[str, object]:
    summary = load_json_object(summary_path)
    if summary.get("status") != "success":
        raise ValueError("project summary status must be success")
    if summary.get("scene_count") != len(EXPECTED_SCENES):
        raise ValueError("project summary scene_count must be 7")
    if summary.get("scene_names") != EXPECTED_SCENES:
        raise ValueError("project summary scene_names must match manifest scenes")
    if not isinstance(summary.get("generated_project_dir"), str):
        raise ValueError("project summary must include generated_project_dir")
    if not isinstance(summary.get("runner_scene_path"), str):
        raise ValueError("project summary must include runner_scene_path")
    if not isinstance(summary.get("runner_script_path"), str):
        raise ValueError("project summary must include runner_script_path")
    return summary


def resolve_project_dir(
    cli_project_dir: Path | None,
    project_summary: dict[str, object],
) -> Path:
    if cli_project_dir is not None:
        return cli_project_dir
    return Path(str(project_summary["generated_project_dir"]))


def determine_run_settings(manifest: dict[str, object], quick_smoke: bool) -> dict[str, object]:
    resolution = manifest["resolution"]
    assert isinstance(resolution, list)
    if quick_smoke:
        return {
            "run_mode": "quick_smoke",
            "warmup_frames": QUICK_SMOKE_WARMUP_FRAMES,
            "sample_frames": QUICK_SMOKE_SAMPLE_FRAMES,
            "vsync": False,
            "resolution": resolution,
        }
    return {
        "run_mode": "full",
        "warmup_frames": manifest["warmup_frames"],
        "sample_frames": manifest["sample_frames"],
        "vsync": manifest["vsync"],
        "resolution": resolution,
    }


def make_run_id(run_mode: str) -> str:
    timestamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    return f"{timestamp}_{run_mode}"


def validate_runner_assets(project_summary: dict[str, object], project_dir: Path) -> tuple[Path, Path]:
    runner_scene_path = Path(str(project_summary["runner_scene_path"]))
    runner_script_path = Path(str(project_summary["runner_script_path"]))
    if runner_scene_path != project_dir / "scenes" / "benchmark_runner.tscn":
        raise ValueError("runner scene path does not match expected generated path")
    if runner_script_path != project_dir / "scripts" / "benchmark_runner.gd":
        raise ValueError("runner script path does not match expected generated path")
    if not runner_scene_path.exists():
        raise FileNotFoundError(f"missing runner scene: {runner_scene_path}")
    if not runner_script_path.exists():
        raise FileNotFoundError(f"missing runner script: {runner_script_path}")
    return runner_scene_path, runner_script_path


def build_scene_command(
    project_dir: Path,
    settings: dict[str, object],
    scene_name: str,
    raw_output_path: Path,
) -> list[str]:
    resolution = settings["resolution"]
    warmup_frames = settings["warmup_frames"]
    sample_frames = settings["sample_frames"]
    run_mode = settings["run_mode"]
    assert isinstance(resolution, list)
    assert isinstance(warmup_frames, int)
    assert isinstance(sample_frames, int)
    assert isinstance(run_mode, str)
    width, height = resolution
    return [
        str(GODOT_CONSOLE_EXE),
        "--path",
        str(project_dir),
        "--rendering-driver",
        "d3d12",
        "--rendering-method",
        "forward_plus",
        "--scene",
        "res://scenes/benchmark_runner.tscn",
        "--",
        "--scene-name",
        scene_name,
        "--scene-path",
        f"res://scenes/{scene_name}.tscn",
        "--raw-output-path",
        str(raw_output_path),
        "--warmup-frames",
        str(warmup_frames),
        "--sample-frames",
        str(sample_frames),
        "--vsync",
        "false",
        "--resolution-width",
        str(width),
        "--resolution-height",
        str(height),
        "--evidence-level",
        EVIDENCE_LEVEL,
        "--run-mode",
        run_mode,
    ]


def percentile_95(values: list[float]) -> float:
    ordered = sorted(values)
    index = max(math.ceil(len(ordered) * 0.95) - 1, 0)
    return ordered[index]


def validate_raw_payload(
    payload: dict[str, object],
    scene_name: str,
    settings: dict[str, object],
) -> dict[str, object]:
    sample_frames = settings["sample_frames"]
    warmup_frames = settings["warmup_frames"]
    run_mode = settings["run_mode"]
    assert isinstance(sample_frames, int)
    assert isinstance(warmup_frames, int)
    assert isinstance(run_mode, str)

    if payload.get("status") != "success":
        raise ValueError(f"{scene_name}: raw payload status is not success")
    if payload.get("scene_name") != scene_name:
        raise ValueError(f"{scene_name}: raw payload scene_name mismatch")
    if payload.get("warmup_frames") != warmup_frames:
        raise ValueError(f"{scene_name}: raw payload warmup_frames mismatch")
    if payload.get("sample_frames") != sample_frames:
        raise ValueError(f"{scene_name}: raw payload sample_frames mismatch")
    if payload.get("gpu_timestamps_available") is not False:
        raise ValueError(f"{scene_name}: gpu_timestamps_available must be false")
    if payload.get("evidence_level") != EVIDENCE_LEVEL:
        raise ValueError(f"{scene_name}: evidence_level mismatch")
    if payload.get("run_mode") != run_mode:
        raise ValueError(f"{scene_name}: run_mode mismatch")

    frame_times_ms = payload.get("frame_times_ms")
    if not isinstance(frame_times_ms, list) or len(frame_times_ms) != sample_frames:
        raise ValueError(f"{scene_name}: frame_times_ms length mismatch")
    if not all(isinstance(value, (int, float)) for value in frame_times_ms):
        raise ValueError(f"{scene_name}: frame_times_ms must contain numbers")

    numeric_frame_times = [float(value) for value in frame_times_ms]
    if not all(value > 0.0 for value in numeric_frame_times):
        raise ValueError(f"{scene_name}: frame_times_ms must all be positive")

    avg_fps = payload.get("avg_fps")
    p95_frame_time_ms = payload.get("p95_frame_time_ms")
    if not isinstance(avg_fps, (int, float)) or float(avg_fps) <= 0.0:
        raise ValueError(f"{scene_name}: avg_fps must be positive")
    if not isinstance(p95_frame_time_ms, (int, float)) or float(p95_frame_time_ms) <= 0.0:
        raise ValueError(f"{scene_name}: p95_frame_time_ms must be positive")

    calculated_p95 = percentile_95(numeric_frame_times)
    if abs(float(p95_frame_time_ms) - calculated_p95) > 1e-6:
        raise ValueError(f"{scene_name}: p95_frame_time_ms does not match frame_times_ms")

    return {
        "sample_count": len(numeric_frame_times),
        "avg_fps": float(avg_fps),
        "p95_frame_time_ms": float(p95_frame_time_ms),
    }


def run_scene(
    project_dir: Path,
    settings: dict[str, object],
    run_dir: Path,
    scene_name: str,
) -> dict[str, object]:
    raw_output_path = run_dir / "raw" / f"{scene_name}.json"
    log_path = run_dir / "logs" / f"{scene_name}.log"
    command = build_scene_command(project_dir, settings, scene_name, raw_output_path)
    result: dict[str, object] = {
        "scene_name": scene_name,
        "scene_path": f"res://scenes/{scene_name}.tscn",
        "command": command,
        "cwd": str(project_dir),
        "raw_json_path": str(raw_output_path),
        "log_path": str(log_path),
        "exit_code": None,
        "status": "fail",
        "error": None,
        "failure_markers": [],
        "warnings": [],
    }

    try:
        completed = subprocess.run(
            command,
            cwd=project_dir,
            capture_output=True,
            text=True,
            timeout=TIMEOUT_SECONDS,
            check=False,
        )
        output = normalize_output(combined_output(completed))
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_path.write_text(output + ("\n" if output else ""), encoding="utf-8", newline="\n")
        result["exit_code"] = completed.returncode

        markers = scan_log_markers(output)
        result["failure_markers"] = markers["failure_markers"]
        result["warnings"] = markers["warnings"]

        if completed.returncode != 0:
            result["error"] = f"Godot exited with code {completed.returncode}"
            return result
        if not raw_output_path.exists():
            result["error"] = f"missing raw output: {raw_output_path}"
            return result

        raw_payload = load_json_object(raw_output_path)
        metrics = validate_raw_payload(raw_payload, scene_name, settings)
        if markers["failure_markers"]:
            result["error"] = f"godot log failure markers: {markers['failure_markers'][0]}"
            return result
        result["status"] = "success"
        result["sample_count"] = metrics["sample_count"]
        result["avg_fps"] = metrics["avg_fps"]
        result["p95_frame_time_ms"] = metrics["p95_frame_time_ms"]
        return result
    except subprocess.TimeoutExpired as exc:
        partial_output = normalize_output(
            combined_output(
                subprocess.CompletedProcess(
                    exc.cmd,
                    returncode=-1,
                    stdout=exc.stdout if isinstance(exc.stdout, str) else "",
                    stderr=exc.stderr if isinstance(exc.stderr, str) else "",
                )
            )
        )
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_path.write_text(
            partial_output + ("\n" if partial_output else ""),
            encoding="utf-8",
            newline="\n",
        )
        markers = scan_log_markers(partial_output)
        result["failure_markers"] = markers["failure_markers"] + [
            f"process timeout after {TIMEOUT_SECONDS} seconds"
        ]
        result["warnings"] = markers["warnings"]
        result["exit_code"] = -1
        result["error"] = f"process timeout after {TIMEOUT_SECONDS} seconds"
        return result
    except Exception as exc:  # pragma: no cover - surfaced by CLI status
        result["error"] = f"{type(exc).__name__}: {exc}"
        return result


def build_initial_summary(
    manifest_path: Path,
    project_summary_path: Path,
    project_dir: Path | None,
) -> dict[str, object]:
    return {
        "runner": "spike/godot-rurix/bench/run_benchmark_scenes.py",
        "status": "fail",
        "manifest_path": str(manifest_path),
        "project_summary_path": str(project_summary_path),
        "project_dir": str(project_dir) if project_dir is not None else None,
        "run_id": None,
        "run_mode": None,
        "scene_count": 0,
        "scene_names": [],
        "warmup_frames": None,
        "sample_frames": None,
        "vsync": None,
        "resolution": None,
        "target_backend": TARGET_BACKEND,
        "evidence_level": EVIDENCE_LEVEL,
        "raw_output_dir": None,
        "log_dir": None,
        "per_scene_results": [],
        "failure_count": 0,
        "warning_count": 0,
    }


def main() -> int:
    args = parse_args()
    summary = build_initial_summary(
        args.manifest,
        args.project_summary,
        args.project_dir,
    )
    try:
        manifest = load_manifest(args.manifest)
        project_summary = load_project_summary(args.project_summary)
        project_dir = resolve_project_dir(args.project_dir, project_summary)
        settings = determine_run_settings(manifest, args.quick_smoke)
        validate_runner_assets(project_summary, project_dir)

        if not GODOT_CONSOLE_EXE.exists():
            raise FileNotFoundError(f"Godot console executable not found: {GODOT_CONSOLE_EXE}")

        run_mode = str(settings["run_mode"])
        run_id = make_run_id(run_mode)
        run_dir = RUNS_DIR / run_id
        raw_output_dir = run_dir / "raw"
        log_dir = run_dir / "logs"
        raw_output_dir.mkdir(parents=True, exist_ok=True)
        log_dir.mkdir(parents=True, exist_ok=True)

        scene_names = EXPECTED_SCENES[:]
        summary.update(
            {
                "run_id": run_id,
                "run_mode": run_mode,
                "project_dir": str(project_dir),
                "scene_count": len(scene_names),
                "scene_names": scene_names,
                "warmup_frames": settings["warmup_frames"],
                "sample_frames": settings["sample_frames"],
                "vsync": settings["vsync"],
                "resolution": settings["resolution"],
                "raw_output_dir": str(raw_output_dir),
                "log_dir": str(log_dir),
            }
        )

        per_scene_results = [run_scene(project_dir, settings, run_dir, scene_name) for scene_name in scene_names]
        failure_count = sum(1 for item in per_scene_results if item.get("status") != "success")
        warning_count = sum(
            len(item["warnings"])
            for item in per_scene_results
            if isinstance(item.get("warnings"), list)
        )
        summary["per_scene_results"] = per_scene_results
        summary["failure_count"] = failure_count
        summary["warning_count"] = warning_count
        summary["status"] = "success" if failure_count == 0 else "fail"
        write_json(RUNNER_SUMMARY_PATH, summary)

        print(f"[bench-runner] status: {summary['status']}")
        print(f"[bench-runner] run_id: {run_id}")
        print(f"[bench-runner] summary_path: {RUNNER_SUMMARY_PATH}")
        if failure_count:
            failed_scenes = [
                str(item["scene_name"])
                for item in per_scene_results
                if item.get("status") != "success"
            ]
            print(f"[bench-runner] failed_scenes: {', '.join(failed_scenes)}")
            return 1
        return 0
    except Exception as exc:  # pragma: no cover - surfaced by CLI status
        summary["error"] = f"{type(exc).__name__}: {exc}"
        write_json(RUNNER_SUMMARY_PATH, summary)
        print(f"[bench-runner] ERROR {summary['error']}")
        print(f"[bench-runner] summary_path: {RUNNER_SUMMARY_PATH}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
