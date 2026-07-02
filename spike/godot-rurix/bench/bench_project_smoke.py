#!/usr/bin/env python3
"""Smoke-test the generated GRX-004 Godot benchmark project skeleton."""

from __future__ import annotations

import json
import subprocess
import re
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[3]
BENCH_DIR = Path(__file__).resolve().parent
MANIFEST_PATH = BENCH_DIR / "bench_manifest.json"
TARGET_GRX_DIR = ROOT / "target" / "grx"
PROJECT_DIR = TARGET_GRX_DIR / "godot-bench-project"
SUMMARY_PATH = TARGET_GRX_DIR / "godot_bench_project_summary.json"
SMOKE_SUMMARY_PATH = TARGET_GRX_DIR / "godot_bench_project_smoke_summary.json"
LOG_DIR = PROJECT_DIR / "logs"
GODOT_CONSOLE_EXE = (
    ROOT / "external" / "godot-master" / "bin" / "godot.windows.template_debug.x86_64.console.exe"
)
TIMEOUT_SECONDS = 120
SCENE_COUNT = 7

SCRIPT_EXT_RESOURCE_RE = re.compile(
    r'\[ext_resource type="Script" path="(?P<path>[^"]+)" id="(?P<id>[^"]+)"\]'
)
SCRIPT_REF_RE = re.compile(r'script = ExtResource\("(?P<id>[^"]+)"\)')
FAILURE_MARKERS = (
    "SCRIPT ERROR:",
    "Parser Error:",
    "Parse Error:",
    "Failed loading resource:",
    "Failed loading script",
)
ALLOWLISTED_GLOBAL_SCRIPT_CACHE_ERROR = "ERROR: Could not load global script cache."
ALLOWLISTED_GLOBAL_SCRIPT_CACHE_CONTEXT = "at: ProjectSettings::get_global_class_list"
LOAD_MARKER_PREFIX = "Loading resource: "


def write_smoke_summary(payload: dict[str, object]) -> None:
    SMOKE_SUMMARY_PATH.write_text(
        json.dumps(payload, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def load_json(path: Path) -> dict[str, object]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return data


def combined_output(proc: subprocess.CompletedProcess[str]) -> str:
    parts: list[str] = []
    if proc.stdout:
        parts.append(proc.stdout.rstrip())
    if proc.stderr:
        parts.append(proc.stderr.rstrip())
    return "\n".join(part for part in parts if part).strip()


def normalize_output(text: str) -> str:
    return text.replace("\r\n", "\n")


def resolve_res_path(res_path: str) -> Path:
    if not res_path.startswith("res://"):
        raise ValueError(f"unsupported resource path: {res_path}")
    relative = PurePosixPath(res_path.removeprefix("res://"))
    return PROJECT_DIR.joinpath(*relative.parts)


def validate_manifest_scenes(manifest: dict[str, object]) -> list[str]:
    manifest_scenes = manifest.get("scenes")
    if not isinstance(manifest_scenes, list) or not all(
        isinstance(name, str) for name in manifest_scenes
    ):
        raise ValueError("manifest scenes must be a string list")
    if len(manifest_scenes) != SCENE_COUNT:
        raise ValueError(f"manifest scenes must contain exactly {SCENE_COUNT} entries")
    return list(manifest_scenes)


def validate_scene_script_reference(scene_path: Path) -> dict[str, object]:
    content = scene_path.read_text(encoding="utf-8")
    script_resources = {
        match.group("id"): match.group("path")
        for match in SCRIPT_EXT_RESOURCE_RE.finditer(content)
    }
    script_ref_match = SCRIPT_REF_RE.search(content)
    if script_ref_match is None:
        raise ValueError(f"{scene_path} is missing a script = ExtResource(...) reference")
    resource_id = script_ref_match.group("id")
    if resource_id not in script_resources:
        raise ValueError(f"{scene_path} references unknown script resource id {resource_id}")
    script_res_path = script_resources[resource_id]
    script_path = resolve_res_path(script_res_path)
    if not script_path.exists():
        raise FileNotFoundError(f"missing script for {scene_path.name}: {script_path}")
    return {
        "scene_path": str(scene_path),
        "script_res_path": script_res_path,
        "script_path": str(script_path),
    }


def run_static_checks() -> dict[str, object]:
    if not PROJECT_DIR.exists():
        raise FileNotFoundError(f"generated project directory does not exist: {PROJECT_DIR}")
    if not (PROJECT_DIR / "project.godot").exists():
        raise FileNotFoundError(f"generated project file does not exist: {PROJECT_DIR / 'project.godot'}")
    if not SUMMARY_PATH.exists():
        raise FileNotFoundError(f"generator summary does not exist: {SUMMARY_PATH}")

    manifest = load_json(MANIFEST_PATH)
    summary = load_json(SUMMARY_PATH)

    manifest_scenes = validate_manifest_scenes(manifest)

    summary_scene_names = summary.get("scene_names")
    summary_scene_paths = summary.get("scene_paths")
    runner_scene_path_text = summary.get("runner_scene_path")
    runner_script_path_text = summary.get("runner_script_path")
    if not isinstance(summary_scene_names, list) or not all(
        isinstance(name, str) for name in summary_scene_names
    ):
        raise ValueError("summary scene_names must be a string list")
    if not isinstance(summary_scene_paths, list) or not all(
        isinstance(path, str) for path in summary_scene_paths
    ):
        raise ValueError("summary scene_paths must be a string list")
    if not isinstance(runner_scene_path_text, str) or not runner_scene_path_text:
        raise ValueError("summary runner_scene_path must be a non-empty string")
    if not isinstance(runner_script_path_text, str) or not runner_script_path_text:
        raise ValueError("summary runner_script_path must be a non-empty string")
    if summary.get("status") != "success":
        raise ValueError(f"generator summary status is not success: {summary.get('status')!r}")

    if manifest_scenes != summary_scene_names:
        raise ValueError("summary scene_names do not match manifest scenes")
    if summary.get("scene_count") != len(manifest_scenes):
        raise ValueError("summary scene_count does not match manifest scenes length")

    validated_scenes: list[dict[str, object]] = []
    for scene_name, scene_path_text in zip(summary_scene_names, summary_scene_paths, strict=True):
        scene_path = Path(scene_path_text)
        if not scene_path.exists():
            raise FileNotFoundError(f"scene file is missing: {scene_path}")
        expected_scene_path = PROJECT_DIR / "scenes" / f"{scene_name}.tscn"
        if scene_path != expected_scene_path:
            raise ValueError(
                f"scene path for {scene_name} does not match expected generated path: "
                f"{scene_path} != {expected_scene_path}"
            )
        validated_scenes.append(validate_scene_script_reference(scene_path))

    runner_scene_path = Path(runner_scene_path_text)
    runner_script_path = Path(runner_script_path_text)
    if runner_scene_path != PROJECT_DIR / "scenes" / "benchmark_runner.tscn":
        raise ValueError("runner scene path does not match expected generated path")
    if runner_script_path != PROJECT_DIR / "scripts" / "benchmark_runner.gd":
        raise ValueError("runner script path does not match expected generated path")
    if not runner_scene_path.exists():
        raise FileNotFoundError(f"runner scene is missing: {runner_scene_path}")
    if not runner_script_path.exists():
        raise FileNotFoundError(f"runner script is missing: {runner_script_path}")

    return {
        "status": "pass",
        "manifest_path": str(MANIFEST_PATH),
        "generator_summary_path": str(SUMMARY_PATH),
        "project_dir": str(PROJECT_DIR),
        "project_file": str(PROJECT_DIR / "project.godot"),
        "scene_count": len(manifest_scenes),
        "scene_names": manifest_scenes,
        "validated_scenes": validated_scenes,
        "runner_scene_path": str(runner_scene_path),
        "runner_script_path": str(runner_script_path),
    }


def build_scene_command(scene_name: str) -> list[str]:
    return [
        str(GODOT_CONSOLE_EXE),
        "--path",
        str(PROJECT_DIR),
        "--headless",
        "--verbose",
        "--quit-after",
        "2",
        "--scene",
        f"res://scenes/{scene_name}.tscn",
    ]


def analyze_scene_output(output: str, scene_name: str) -> dict[str, list[str]]:
    normalized = normalize_output(output)
    lines = normalized.splitlines()
    failure_markers: list[str] = []
    warnings: list[str] = []
    loaded_markers: list[str] = []
    scene_load_marker = f"{LOAD_MARKER_PREFIX}res://scenes/{scene_name}.tscn"

    index = 0
    while index < len(lines):
        line = lines[index].strip()
        if not line:
            index += 1
            continue

        if line.startswith(LOAD_MARKER_PREFIX):
            if f"res://scenes/{scene_name}.tscn" in line or f"res://scripts/{scene_name}.gd" in line:
                loaded_markers.append(line)
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

    if scene_load_marker not in loaded_markers:
        failure_markers.append(f"missing load evidence: {scene_load_marker}")

    return {
        "failure_markers": failure_markers,
        "warnings": warnings,
        "loaded_markers": loaded_markers,
    }


def run_scene_smoke(scene_name: str) -> dict[str, object]:
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    scene_path = PROJECT_DIR / "scenes" / f"{scene_name}.tscn"
    log_path = LOG_DIR / f"{scene_name}.log"
    command = build_scene_command(scene_name)
    result: dict[str, object] = {
        "scene_name": scene_name,
        "scene_path": str(scene_path),
        "command": command,
        "cwd": str(PROJECT_DIR),
        "status": "fail",
        "exit_code": None,
        "log_path": str(log_path),
        "failure_markers": [],
        "warnings": [],
        "loaded_markers": [],
    }

    try:
        completed = subprocess.run(
            command,
            cwd=PROJECT_DIR,
            capture_output=True,
            text=True,
            timeout=TIMEOUT_SECONDS,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        output = normalize_output(
            combined_output(
                subprocess.CompletedProcess(
                    exc.cmd,
                    returncode=-1,
                    stdout=exc.stdout if isinstance(exc.stdout, str) else "",
                    stderr=exc.stderr if isinstance(exc.stderr, str) else "",
                )
            )
        )
        log_path.write_text(output + ("\n" if output else ""), encoding="utf-8", newline="\n")
        analysis = analyze_scene_output(output, scene_name)
        result["exit_code"] = -1
        result["status"] = "fail"
        result["failure_markers"] = analysis["failure_markers"] + [
            f"process timeout after {TIMEOUT_SECONDS} seconds"
        ]
        result["warnings"] = analysis["warnings"]
        result["loaded_markers"] = analysis["loaded_markers"]
        return result

    output = normalize_output(combined_output(completed))
    log_path.write_text(output + ("\n" if output else ""), encoding="utf-8", newline="\n")
    analysis = analyze_scene_output(output, scene_name)
    result["exit_code"] = completed.returncode
    result["failure_markers"] = analysis["failure_markers"]
    result["warnings"] = analysis["warnings"]
    result["loaded_markers"] = analysis["loaded_markers"]
    if completed.returncode == 0 and not analysis["failure_markers"]:
        result["status"] = "pass"
    else:
        result["status"] = "fail"
    return result


def summarize_failures(per_scene_results: list[dict[str, object]]) -> list[str]:
    return [
        str(result["scene_name"])
        for result in per_scene_results
        if str(result.get("status")) != "pass"
    ]


def main() -> int:
    smoke_summary: dict[str, object] = {
        "smoke": "spike/godot-rurix/bench/bench_project_smoke.py",
        "status": "fail",
        "manifest_path": str(MANIFEST_PATH),
        "project_dir": str(PROJECT_DIR),
        "generator_summary_path": str(SUMMARY_PATH),
        "scene_count": 0,
        "scene_names": [],
        "warning_count": 0,
        "failure_count": 0,
        "per_scene_results": [],
    }
    try:
        static_checks = run_static_checks()
        scene_names = static_checks["scene_names"]
        assert isinstance(scene_names, list)
        if not GODOT_CONSOLE_EXE.exists():
            smoke_summary.update(
                {
                    "status": "fail",
                    "scene_count": len(scene_names),
                    "scene_names": scene_names,
                    "static_checks": static_checks,
                    "failure_count": len(scene_names),
                    "reason": f"Godot console executable not found: {GODOT_CONSOLE_EXE}",
                }
            )
            write_smoke_summary(smoke_summary)
            print("[bench-smoke] status: fail")
            print(f"[bench-smoke] summary_path: {SMOKE_SUMMARY_PATH}")
            print(f"[bench-smoke] reason: {smoke_summary['reason']}")
            return 1

        per_scene_results = [run_scene_smoke(scene_name) for scene_name in scene_names]
        failed_scenes = summarize_failures(per_scene_results)
        warning_count = sum(
            len(result["warnings"])
            for result in per_scene_results
            if isinstance(result.get("warnings"), list)
        )
        status = "success" if not failed_scenes else "fail"
        smoke_summary.update(
            {
                "status": status,
                "scene_count": len(scene_names),
                "scene_names": scene_names,
                "static_checks": static_checks,
                "warning_count": warning_count,
                "failure_count": len(failed_scenes),
                "per_scene_results": per_scene_results,
            }
        )
        write_smoke_summary(smoke_summary)
        print(f"[bench-smoke] status: {status}")
        print(f"[bench-smoke] summary_path: {SMOKE_SUMMARY_PATH}")
        if failed_scenes:
            print(f"[bench-smoke] failed_scenes: {', '.join(failed_scenes)}")
            return 1
        return 0
    except Exception as exc:  # pragma: no cover - surfaced by CLI status
        error_message = f"{type(exc).__name__}: {exc}"
        smoke_summary["error"] = error_message
        write_smoke_summary(smoke_summary)
        print(f"[bench-smoke] ERROR {error_message}")
        print(f"[bench-smoke] summary_path: {SMOKE_SUMMARY_PATH}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
