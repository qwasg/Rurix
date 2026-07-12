#!/usr/bin/env python3
"""Run the tracked GRX-005 Godot benchmark scenes and collect raw frame samples."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
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
# Environment fallback for the Godot console exe (below the explicit --godot-exe
# flag, above the tracked default). Lets both legs of a run point at the same
# fuller patch-stack build without repeating a long path on the command line.
ENV_GODOT_EXE = "RURIX_BENCH_GODOT_EXE"
# Tracked default: the scratch build committed under external/godot-master. It
# carries only patches 0001+0002+0003 (module scaffold + luminance pass-gate +
# core call-site wiring) and NO real-pass dispatch hook, so a rurix leg run
# against it can never engage a real pass (it always falls back). Point
# --godot-exe / RURIX_BENCH_GODOT_EXE at a fuller patch-stack build (and pass the
# matching --patch-stack-id) to exercise real passes.
DEFAULT_GODOT_CONSOLE_EXE = (
    ROOT / "external" / "godot-master" / "bin" / "godot.windows.template_debug.x86_64.console.exe"
)
DEFAULT_GODOT_EXE_NOTE = (
    "using tracked default Godot exe (external/godot-master scratch build, "
    "patches 0001+0002+0003 only, no real-pass hook); pass --godot-exe or set "
    "RURIX_BENCH_GODOT_EXE to a fuller patch-stack build for real passes"
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
ITER_WARMUP_FRAMES = 120
ITER_SAMPLE_FRAMES = 600
TIMEOUT_SECONDS = 1800

DLL_PATH = ROOT / "target" / "debug" / "rurix_godot.dll"
OVERRIDE_CFG_NAME = "override.cfg"

# Full set of override-able rurix_accel project settings, extracted from the
# GLOBAL_DEF_BASIC keys in the landed patch stack
# (spike/godot-rurix/patches/0001..0016) plus the reserved (not-yet-landed)
# taa_resolve / particles_copy blocks. A rurix-leg pass matrix may only set keys
# from this allowlist (fail-closed on typos so a mis-typed key can never silently
# disable a pass while claiming it ran).
VALID_PASS_MATRIX_KEYS = frozenset(
    {
        # Top-level gate (patch 0001 module scaffold).
        "rendering/rurix_accel/enabled",
        "rendering/rurix_accel/require_forward_plus",
        "rendering/rurix_accel/dll_path",
        # luminance_reduction (GRX-009, patches 0001..0010, landed).
        "rendering/rurix_accel/passes/luminance_reduction/enabled",
        "rendering/rurix_accel/passes/luminance_reduction/dispatch_bringup",
        "rendering/rurix_accel/passes/luminance_reduction/dispatch_recording_smoke",
        "rendering/rurix_accel/passes/luminance_reduction/dispatch_real_pass",
        "rendering/rurix_accel/passes/luminance_reduction/real_pass_force_capability_downgrade",
        # tonemap (GRX-010, patches 0011..0013, landed).
        "rendering/rurix_accel/passes/tonemap/enabled",
        "rendering/rurix_accel/passes/tonemap/dispatch_recording_smoke",
        "rendering/rurix_accel/passes/tonemap/dispatch_real_pass",
        "rendering/rurix_accel/passes/tonemap/real_pass_force_capability_downgrade",
        # ssao_blur (GRX-011, patches 0014..0016, landed). Key names verified
        # against GLOBAL_DEF_BASIC in patch 0014 (enabled) and patch 0016
        # (dispatch_recording_smoke / dispatch_real_pass /
        # real_pass_force_capability_downgrade).
        "rendering/rurix_accel/passes/ssao_blur/enabled",
        "rendering/rurix_accel/passes/ssao_blur/dispatch_recording_smoke",
        "rendering/rurix_accel/passes/ssao_blur/dispatch_real_pass",
        "rendering/rurix_accel/passes/ssao_blur/real_pass_force_capability_downgrade",
        # taa_resolve (GRX-012, patches 0017..0019 NOT yet landed). Reserved:
        # key names inferred from the per-pass template (isomorphic to
        # tonemap / ssao_blur); RE-VERIFY against GLOBAL_DEF_BASIC when the
        # patches land.
        "rendering/rurix_accel/passes/taa_resolve/enabled",
        "rendering/rurix_accel/passes/taa_resolve/dispatch_recording_smoke",
        "rendering/rurix_accel/passes/taa_resolve/dispatch_real_pass",
        "rendering/rurix_accel/passes/taa_resolve/real_pass_force_capability_downgrade",
        # particles_copy (GRX-013, patches 0020..0022 NOT yet landed). Reserved:
        # key names inferred from the per-pass template; RE-VERIFY against
        # GLOBAL_DEF_BASIC when the patches land.
        "rendering/rurix_accel/passes/particles_copy/enabled",
        "rendering/rurix_accel/passes/particles_copy/dispatch_recording_smoke",
        "rendering/rurix_accel/passes/particles_copy/dispatch_real_pass",
        "rendering/rurix_accel/passes/particles_copy/real_pass_force_capability_downgrade",
    }
)

# Per-frame or exit-summary pass-engagement markers. The current bridge prints
# per-frame record/blocked markers; a future "B0" refactor is expected to emit a
# single exit summary (RXGD_PASS_ENGAGEMENT pass=<name> recorded=<n>
# fallback=<m>). Both forms are parsed; when neither is present the runner
# records pass_engagement as null (honest: the tracked Godot exe carries no
# rurix_accel module, so no markers appear).
PASS_ENGAGEMENT_SUMMARY_RE = re.compile(
    r"RXGD_PASS_ENGAGEMENT\s+pass=([A-Za-z0-9_]+)\s+recorded=(\d+)\s+fallback=(\d+)"
)
PASS_RECORD_MARKERS = {
    "luminance_reduction": (
        "RXGD_GODOT_RUNTIME_LUMINANCE_REAL_PASS",
        "RXGD_GODOT_RUNTIME_LUMINANCE_RECORD",
    ),
    "tonemap": (
        "RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS",
        "RXGD_GODOT_RUNTIME_TONEMAP_RECORD",
    ),
}
PASS_FALLBACK_MARKERS = {
    "luminance_reduction": ("RXGD_REAL_PASS_BLOCKED",),
    "tonemap": ("RXGD_TONEMAP_REAL_PASS_BLOCKED",),
}

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


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def resolve_scene_names(scenes_arg: str | None) -> list[str]:
    """Resolve an optional --scenes subset, preserving the fixed manifest order."""
    if not scenes_arg:
        return EXPECTED_SCENES[:]
    requested = [name.strip() for name in scenes_arg.split(",") if name.strip()]
    if not requested:
        return EXPECTED_SCENES[:]
    unknown = [name for name in requested if name not in EXPECTED_SCENES]
    if unknown:
        raise ValueError(
            "unknown scene(s): "
            + ", ".join(unknown)
            + " (valid: "
            + ", ".join(EXPECTED_SCENES)
            + ")"
        )
    selected = set(requested)
    return [name for name in EXPECTED_SCENES if name in selected]


def load_pass_matrix(path: Path) -> dict[str, object]:
    """Load and validate a rurix-leg pass matrix (fail-closed on unknown keys)."""
    doc = load_json_object(path)
    raw_settings = doc.get("settings")
    settings = raw_settings if isinstance(raw_settings, dict) else doc
    if not isinstance(settings, dict) or not settings:
        raise ValueError(f"pass matrix {path} must contain a non-empty settings object")
    normalized: dict[str, object] = {}
    for key, value in settings.items():
        if key not in VALID_PASS_MATRIX_KEYS:
            raise ValueError(f"pass matrix key not in rurix_accel allowlist: {key}")
        if isinstance(value, bool) or isinstance(value, str):
            normalized[key] = value
        else:
            raise ValueError(f"pass matrix value for {key} must be a bool or string")
    return normalized


def render_override_cfg(settings: dict[str, object]) -> str:
    lines = [
        "; Auto-generated by run_benchmark_scenes.py for a rurix leg; deleted",
        "; after the run. Overrides rurix_accel project settings only.",
        "",
        "[rendering]",
        "",
    ]
    for key in sorted(settings):
        value = settings[key]
        sub_key = key[len("rendering/"):] if key.startswith("rendering/") else key
        if isinstance(value, bool):
            rendered = "true" if value else "false"
        else:
            # Godot config strings treat backslashes as escape sequences
            # (a Windows path like H:\rurix\target\... would be mangled into
            # CR/TAB bytes and can poison the whole override.cfg parse), so
            # escape them explicitly alongside double quotes.
            text = str(value).replace("\\", "\\\\").replace('"', '\\"')
            rendered = '"' + text + '"'
        lines.append(f"{sub_key}={rendered}")
    lines.append("")
    return "\n".join(lines)


def write_override_cfg(project_dir: Path, settings: dict[str, object]) -> Path:
    path = project_dir / OVERRIDE_CFG_NAME
    path.write_text(render_override_cfg(settings), encoding="utf-8", newline="\n")
    return path


def resolve_godot_exe(cli_exe: Path | None) -> tuple[Path, str, str | None]:
    """Resolve the Godot console exe both legs run against.

    Priority: explicit --godot-exe, then the RURIX_BENCH_GODOT_EXE environment
    variable, then the tracked default scratch build. Returns (path, source,
    note); note is set only for the tracked default so the run summary can flag
    its known (real-pass-less) patch state. Both the baseline and rurix legs of a
    comparison must be driven against the SAME resolved exe; the summary records
    godot_exe + godot_exe_sha256 so a cross-leg mismatch is auditable.
    """
    if cli_exe is not None:
        return cli_exe, "cli", None
    env_value = os.environ.get(ENV_GODOT_EXE)
    if env_value:
        return Path(env_value), "env", None
    return DEFAULT_GODOT_CONSOLE_EXE, "tracked_default", DEFAULT_GODOT_EXE_NOTE


def resolve_patch_stack_id(cli_id: str | None, godot_exe: Path) -> tuple[str | None, str | None]:
    """Best-effort patch-stack identity for the running Godot exe.

    Order: explicit --patch-stack-id, then an optional sidecar next to the exe
    or under target/grx. Returns (id, note); note is set only when the id could
    not be resolved so the raw payload can annotate the null.
    """
    if cli_id:
        return cli_id, None
    candidates = [
        godot_exe.parent / "rurix_patch_stack_id.txt",
        TARGET_GRX_DIR / "godot_patch_stack_id.json",
    ]
    for candidate in candidates:
        if not candidate.is_file():
            continue
        try:
            if candidate.suffix == ".json":
                data = json.loads(candidate.read_text(encoding="utf-8"))
                value = data.get("patch_stack_id") if isinstance(data, dict) else None
            else:
                value = candidate.read_text(encoding="utf-8").strip()
            if isinstance(value, str) and value:
                return value, None
        except (OSError, json.JSONDecodeError):
            continue
    return None, (
        "patch_stack_id unresolved: no --patch-stack-id and no sidecar "
        "(rurix_patch_stack_id.txt / godot_patch_stack_id.json); the tracked "
        "Godot exe carries no machine-readable patch-stack identity"
    )


def parse_pass_engagement(output: str) -> dict[str, dict[str, int]] | None:
    """Parse per-pass recorded/fallback frame counts from Godot stdout.

    Accepts either the future exit-summary form or the current per-frame marker
    form; returns None when no engagement markers are present.
    """
    summary: dict[str, dict[str, int]] = {}
    for match in PASS_ENGAGEMENT_SUMMARY_RE.finditer(output):
        summary[match.group(1)] = {
            "recorded": int(match.group(2)),
            "fallback": int(match.group(3)),
        }
    if summary:
        return summary
    lines = output.splitlines()
    counts: dict[str, dict[str, int]] = {}
    for pass_name, markers in PASS_RECORD_MARKERS.items():
        recorded = sum(1 for line in lines if any(mk in line for mk in markers))
        fallback = sum(
            1
            for line in lines
            if any(mk in line for mk in PASS_FALLBACK_MARKERS.get(pass_name, ()))
        )
        if recorded or fallback:
            counts[pass_name] = {"recorded": recorded, "fallback": fallback}
    return counts or None


def enrich_raw_payload(
    path: Path, context: dict[str, object], engagement: dict[str, dict[str, int]] | None
) -> None:
    """Inject run-identity/provenance fields the GD runner cannot compute."""
    payload = load_json_object(path)
    payload["leg"] = context["leg"]
    payload["pass_matrix"] = context["pass_matrix"]
    payload["dll_sha256"] = context["dll_sha256"]
    payload["godot_exe_sha256"] = context["godot_exe_sha256"]
    payload["patch_stack_id"] = context["patch_stack_id"]
    if context["patch_stack_id"] is None:
        payload["patch_stack_id_note"] = context["patch_stack_id_note"]
    payload["pass_engagement"] = engagement
    write_json(path, payload)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST_PATH)
    parser.add_argument("--project-summary", type=Path, default=DEFAULT_PROJECT_SUMMARY_PATH)
    parser.add_argument("--project-dir", type=Path)
    parser.add_argument("--quick-smoke", action="store_true")
    parser.add_argument(
        "--scenes",
        type=str,
        default=None,
        help="comma-separated subset of the seven fixed scenes (default: all)",
    )
    parser.add_argument(
        "--profile",
        choices=("full", "iter"),
        default="full",
        help="full = 300/2000 (strict-eligible); iter = 120/600 dev sampling",
    )
    parser.add_argument(
        "--leg",
        choices=("baseline", "rurix"),
        default="baseline",
        help="baseline = unmodified engine; rurix = pass matrix applied via override.cfg",
    )
    parser.add_argument(
        "--pass-matrix",
        type=Path,
        default=None,
        help="rurix-leg pass matrix JSON (required for --leg rurix, forbidden otherwise)",
    )
    parser.add_argument(
        "--godot-exe",
        type=Path,
        default=None,
        help=(
            "Godot console exe both legs run against (default: RURIX_BENCH_GODOT_EXE "
            "env, else the tracked external/godot-master scratch build). Pair with "
            "--patch-stack-id to record which patch stack the exe was built from."
        ),
    )
    parser.add_argument("--patch-stack-id", type=str, default=None)
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


def determine_run_settings(
    manifest: dict[str, object], quick_smoke: bool, profile: str
) -> dict[str, object]:
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
    if profile == "iter":
        # Dev/iteration sampling: run_mode is deliberately not "full" so this
        # evidence can never satisfy the strict close-out perf gate.
        return {
            "run_mode": "iter",
            "warmup_frames": ITER_WARMUP_FRAMES,
            "sample_frames": ITER_SAMPLE_FRAMES,
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
    context: dict[str, object],
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
    command = [
        str(context["godot_exe"]),
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
        "--leg",
        str(context["leg"]),
    ]
    pass_matrix_path = context.get("pass_matrix_path")
    if pass_matrix_path is not None:
        command.extend(["--pass-matrix-path", str(pass_matrix_path)])
    return command


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
    context: dict[str, object],
) -> dict[str, object]:
    raw_output_path = run_dir / "raw" / f"{scene_name}.json"
    log_path = run_dir / "logs" / f"{scene_name}.log"
    command = build_scene_command(project_dir, settings, scene_name, raw_output_path, context)
    result: dict[str, object] = {
        "scene_name": scene_name,
        "scene_path": f"res://scenes/{scene_name}.tscn",
        "command": command,
        "cwd": str(project_dir),
        "raw_json_path": str(raw_output_path),
        "log_path": str(log_path),
        "leg": context["leg"],
        "exit_code": None,
        "status": "fail",
        "error": None,
        "failure_markers": [],
        "warnings": [],
        "pass_engagement": None,
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
        engagement = parse_pass_engagement(output)
        enrich_raw_payload(raw_output_path, context, engagement)
        result["status"] = "success"
        result["sample_count"] = metrics["sample_count"]
        result["avg_fps"] = metrics["avg_fps"]
        result["p95_frame_time_ms"] = metrics["p95_frame_time_ms"]
        result["pass_engagement"] = engagement
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
        "leg": None,
        "profile": None,
        "scene_subset": False,
        "pass_matrix": {},
        "pass_matrix_path": None,
        "dll_sha256": None,
        "godot_exe": None,
        "godot_exe_source": None,
        "godot_exe_note": None,
        "godot_exe_sha256": None,
        "patch_stack_id": None,
        "patch_stack_id_note": None,
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
    override_cfg_path: Path | None = None
    try:
        manifest = load_manifest(args.manifest)
        project_summary = load_project_summary(args.project_summary)
        project_dir = resolve_project_dir(args.project_dir, project_summary)
        settings = determine_run_settings(manifest, args.quick_smoke, args.profile)
        validate_runner_assets(project_summary, project_dir)

        godot_exe, godot_exe_source, godot_exe_note = resolve_godot_exe(args.godot_exe)
        if not godot_exe.exists():
            raise FileNotFoundError(f"Godot console executable not found: {godot_exe}")
        if godot_exe_note is not None:
            print(f"[bench-runner] NOTE {godot_exe_note}")

        scene_names = resolve_scene_names(args.scenes)

        # Leg / pass-matrix resolution. The rurix leg writes a scoped override.cfg
        # (deleted in the finally below); the baseline leg must run against an
        # engine with no such override present.
        leg = args.leg
        pass_matrix_settings: dict[str, object] = {}
        pass_matrix_path: Path | None = None
        if leg == "rurix":
            if args.pass_matrix is None:
                raise ValueError("--leg rurix requires --pass-matrix <json path>")
            pass_matrix_path = args.pass_matrix
            pass_matrix_settings = load_pass_matrix(pass_matrix_path)
            override_cfg_path = write_override_cfg(project_dir, pass_matrix_settings)
        else:
            if args.pass_matrix is not None:
                raise ValueError("--leg baseline must not be given a --pass-matrix")
            existing_override = project_dir / OVERRIDE_CFG_NAME
            if existing_override.exists():
                raise ValueError(
                    "baseline leg requires no override.cfg, but one exists: "
                    f"{existing_override}"
                )

        patch_stack_id, patch_stack_id_note = resolve_patch_stack_id(
            args.patch_stack_id, godot_exe
        )
        context: dict[str, object] = {
            "leg": leg,
            "pass_matrix": pass_matrix_settings,
            "pass_matrix_path": pass_matrix_path,
            "dll_sha256": sha256_file(DLL_PATH),
            "godot_exe": godot_exe,
            "godot_exe_sha256": sha256_file(godot_exe),
            "patch_stack_id": patch_stack_id,
            "patch_stack_id_note": patch_stack_id_note,
        }

        run_mode = str(settings["run_mode"])
        run_id = make_run_id(run_mode)
        run_dir = RUNS_DIR / run_id
        raw_output_dir = run_dir / "raw"
        log_dir = run_dir / "logs"
        raw_output_dir.mkdir(parents=True, exist_ok=True)
        log_dir.mkdir(parents=True, exist_ok=True)

        summary.update(
            {
                "run_id": run_id,
                "run_mode": run_mode,
                "profile": args.profile,
                "leg": leg,
                "scene_subset": scene_names != EXPECTED_SCENES,
                "pass_matrix": pass_matrix_settings,
                "pass_matrix_path": str(pass_matrix_path) if pass_matrix_path is not None else None,
                "dll_sha256": context["dll_sha256"],
                "godot_exe": str(godot_exe),
                "godot_exe_source": godot_exe_source,
                "godot_exe_note": godot_exe_note,
                "godot_exe_sha256": context["godot_exe_sha256"],
                "patch_stack_id": patch_stack_id,
                "patch_stack_id_note": patch_stack_id_note,
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

        per_scene_results = [
            run_scene(project_dir, settings, run_dir, scene_name, context)
            for scene_name in scene_names
        ]
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
    finally:
        # The rurix-leg override.cfg is a transient run artifact; never leave it
        # behind (a stray override.cfg would fail the next baseline leg's guard).
        if override_cfg_path is not None:
            try:
                override_cfg_path.unlink(missing_ok=True)
            except OSError:
                pass


if __name__ == "__main__":
    raise SystemExit(main())
