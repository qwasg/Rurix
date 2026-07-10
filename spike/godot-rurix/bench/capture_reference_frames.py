#!/usr/bin/env python3
"""Capture GRX-007 reference frames for the seven benchmark scenes.

Real capture backend (GRX-007 close-out): for every benchmark scene this tool
generates a deterministic capture runner (GDScript + scene) inside the
generated bench project, runs the tracked Godot console executable
(D3D12 Forward+, --fixed-fps for deterministic animation state), waits a fixed
number of frames, grabs the viewport image via
``get_viewport().get_texture().get_image()`` and stores it as a raw RGB8
artifact (+ PNG sibling + metadata JSON) under
``target/grx/godot-visual/reference/``.

Frames are captured at a reduced deterministic resolution
(``CAPTURE_RESOLUTION``) so the ``{"pixels": [[r,g,b], ...]}`` channel
documents consumed by ``visual_diff.py`` stay small. The capture summary
(hashes / dimensions / frame index per scene) is written to
``target/grx/godot-visual/capture_summary.json``.

This is REFERENCE CAPTURE ONLY. The visual evidence generated downstream
compares each captured reference frame against itself (reference == candidate)
purely to prove the capture pipeline and LDR diff tooling work end to end on
real frames. It is NOT a Rurix-pass-vs-baseline visual comparison and makes no
visual-verification or performance claim about any Rurix pass.
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
TARGET_GRX_DIR = ROOT / "target" / "grx"
VISUAL_DIR = TARGET_GRX_DIR / "godot-visual"
REFERENCE_DIR = VISUAL_DIR / "reference"
FRAMES_DIR = VISUAL_DIR / "frames"
CAPTURE_SUMMARY_PATH = VISUAL_DIR / "capture_summary.json"
VISUAL_EVIDENCE_PATH = VISUAL_DIR / "visual_reference_capture_evidence.json"
VISUAL_DIFF_SCRIPT = BENCH_DIR / "visual_diff.py"
PROJECT_SUMMARY_PATH = TARGET_GRX_DIR / "godot_bench_project_summary.json"
PROJECT_DIR = TARGET_GRX_DIR / "godot-bench-project"

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
# Benchmark target resolution (baseline runs render at this); capture frames
# use the reduced CAPTURE_RESOLUTION below so channel JSON stays small.
RESOLUTION = [1920, 1080]
CAPTURE_RESOLUTION = [256, 144]
# Fixed capture frame per scene: a single frame captured after warmup.
CAPTURE_FRAME_INDEX = 600
FRAME_FORMAT = "R8G8B8_raw"
GODOT_TIMEOUT_SECONDS = 300
GODOT_CONSOLE_EXE = (
    ROOT / "external" / "godot-master" / "bin" / "godot.windows.template_debug.x86_64.console.exe"
)

CAPTURE_RUNNER_SCRIPT_PATH = PROJECT_DIR / "scripts" / "capture_runner.gd"
CAPTURE_RUNNER_SCENE_PATH = PROJECT_DIR / "scenes" / "capture_runner.tscn"

# Aligned with run_benchmark_scenes.py log policy: only the known global
# script cache warning is tolerated; any other ERROR/failure marker fails.
FAILURE_MARKERS = (
    "SCRIPT ERROR:",
    "Parser Error:",
    "Parse Error:",
    "Failed loading resource:",
    "Failed loading script",
)
ALLOWLISTED_GLOBAL_SCRIPT_CACHE_ERROR = "ERROR: Could not load global script cache."
ALLOWLISTED_GLOBAL_SCRIPT_CACHE_CONTEXT = "at: ProjectSettings::get_global_class_list"

CAPTURE_RUNNER_SCENE = """\
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://scripts/capture_runner.gd" id="1"]

[node name="CaptureRunnerRoot" type="Node"]
script = ExtResource("1")

[node name="SceneContainer" type="Node" parent="."]
"""

CAPTURE_RUNNER_SCRIPT = f"""\
extends Node

const CAPTURE_WIDTH := {CAPTURE_RESOLUTION[0]}
const CAPTURE_HEIGHT := {CAPTURE_RESOLUTION[1]}
const FRAME_FORMAT := "{FRAME_FORMAT}"

@onready var scene_container: Node = $SceneContainer

var scene_name := ""
var scene_path := ""
var capture_prefix := ""
var capture_frame := {CAPTURE_FRAME_INDEX}
var processed_frames := 0
var capture_started := false


func _ready() -> void:
    var args := OS.get_cmdline_user_args()
    var index := 0
    while index + 1 < args.size():
        var key := args[index]
        var value := args[index + 1]
        match key:
            "--scene-name":
                scene_name = value
            "--scene-path":
                scene_path = value
            "--capture-prefix":
                capture_prefix = value
            "--capture-frame":
                capture_frame = int(value)
        index += 2

    if scene_name.is_empty() or scene_path.is_empty() or capture_prefix.is_empty():
        printerr("[capture-runner] ERROR missing required arguments")
        get_tree().quit(3)
        return
    if capture_frame <= 0:
        printerr("[capture-runner] ERROR capture_frame must be positive")
        get_tree().quit(3)
        return

    var window := get_window()
    if window != null:
        window.size = Vector2i(CAPTURE_WIDTH, CAPTURE_HEIGHT)
    DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)

    var packed_scene := load(scene_path)
    if packed_scene == null:
        printerr("[capture-runner] ERROR failed to load scene: %s" % scene_path)
        get_tree().quit(3)
        return
    var loaded_scene: Node = packed_scene.instantiate()
    if loaded_scene == null:
        printerr("[capture-runner] ERROR failed to instantiate scene: %s" % scene_path)
        get_tree().quit(3)
        return
    scene_container.add_child(loaded_scene)
    print("[capture-runner] scene_name=%s capture_frame=%d" % [scene_name, capture_frame])
    set_process(true)


func _process(_delta: float) -> void:
    if capture_started:
        return
    processed_frames += 1
    if processed_frames >= capture_frame:
        capture_started = true
        _capture()


func _capture() -> void:
    await RenderingServer.frame_post_draw
    var img: Image = get_viewport().get_texture().get_image()
    img.convert(Image.FORMAT_RGB8)
    var raw := FileAccess.open(capture_prefix + ".rgb8", FileAccess.WRITE)
    if raw == null:
        printerr("[capture-runner] ERROR cannot open output: %s.rgb8" % capture_prefix)
        get_tree().quit(3)
        return
    raw.store_buffer(img.get_data())
    raw.close()
    img.save_png(capture_prefix + ".png")
    var meta := FileAccess.open(capture_prefix + ".meta.json", FileAccess.WRITE)
    if meta == null:
        printerr("[capture-runner] ERROR cannot open output: %s.meta.json" % capture_prefix)
        get_tree().quit(3)
        return
    meta.store_string(JSON.stringify({{
        "scene_name": scene_name,
        "scene_path": scene_path,
        "width": img.get_width(),
        "height": img.get_height(),
        "format": FRAME_FORMAT,
        "capture_frame_index": processed_frames,
    }}))
    meta.close()
    print(
        "[capture-runner] captured scene=%s frame=%d width=%d height=%d"
        % [scene_name, processed_frames, img.get_width(), img.get_height()]
    )
    get_tree().quit(0)
"""


def capture_backend_available() -> bool:
    """The real capture backend needs the tracked Godot console executable."""
    return GODOT_CONSOLE_EXE.exists()


def godot_full_run_available() -> bool:
    """A Godot full run is available only if the console executable exists."""
    return GODOT_CONSOLE_EXE.exists()


def bench_project_available() -> bool:
    if not PROJECT_SUMMARY_PATH.is_file():
        return False
    try:
        summary = json.loads(PROJECT_SUMMARY_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return False
    if not isinstance(summary, dict) or summary.get("status") != "success":
        return False
    return (PROJECT_DIR / "project.godot").is_file()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def normalize_output(text: str) -> str:
    return text.replace("\r\n", "\n")


def scan_log_markers(output: str) -> dict[str, list[str]]:
    """Failure-marker scan aligned with run_benchmark_scenes.py."""
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


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def install_capture_runner() -> None:
    CAPTURE_RUNNER_SCRIPT_PATH.parent.mkdir(parents=True, exist_ok=True)
    CAPTURE_RUNNER_SCENE_PATH.parent.mkdir(parents=True, exist_ok=True)
    CAPTURE_RUNNER_SCRIPT_PATH.write_text(
        CAPTURE_RUNNER_SCRIPT, encoding="utf-8", newline="\n"
    )
    CAPTURE_RUNNER_SCENE_PATH.write_text(
        CAPTURE_RUNNER_SCENE, encoding="utf-8", newline="\n"
    )


def repo_relative_posix(path: Path) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(ROOT).as_posix()
    except ValueError:
        return resolved.as_posix()


def capture_scene(scene_name: str) -> dict[str, object]:
    """Run one deterministic capture leg; returns the per-scene summary entry."""
    capture_prefix = REFERENCE_DIR / scene_name
    for suffix in (".rgb8", ".png", ".meta.json"):
        Path(str(capture_prefix) + suffix).unlink(missing_ok=True)

    command = [
        str(GODOT_CONSOLE_EXE),
        "--path",
        str(PROJECT_DIR),
        "--rendering-driver",
        "d3d12",
        "--rendering-method",
        "forward_plus",
        "--resolution",
        f"{CAPTURE_RESOLUTION[0]}x{CAPTURE_RESOLUTION[1]}",
        "--fixed-fps",
        "60",
        "--scene",
        "res://scenes/capture_runner.tscn",
        "--",
        "--scene-name",
        scene_name,
        "--scene-path",
        f"res://scenes/{scene_name}.tscn",
        "--capture-prefix",
        str(capture_prefix),
        "--capture-frame",
        str(CAPTURE_FRAME_INDEX),
    ]
    entry: dict[str, object] = {
        "name": scene_name,
        "status": "fail",
        "error": None,
        "command": command,
        "requested_capture_frame_index": CAPTURE_FRAME_INDEX,
        "requested_capture_resolution": CAPTURE_RESOLUTION,
    }
    try:
        proc = subprocess.run(
            command,
            cwd=PROJECT_DIR,
            capture_output=True,
            text=True,
            timeout=GODOT_TIMEOUT_SECONDS,
            check=False,
        )
    except subprocess.TimeoutExpired:
        entry["error"] = f"Godot capture run timed out after {GODOT_TIMEOUT_SECONDS}s"
        return entry
    output = "\n".join(part for part in (proc.stdout, proc.stderr) if part).strip()
    markers = scan_log_markers(output)
    entry["exit_code"] = proc.returncode
    entry["failure_markers"] = markers["failure_markers"]
    entry["warnings"] = markers["warnings"]
    if proc.returncode != 0:
        entry["error"] = f"Godot exited with code {proc.returncode}"
        entry["stdout_tail"] = output[-2000:]
        return entry
    if markers["failure_markers"]:
        entry["error"] = f"godot log failure markers: {markers['failure_markers'][0]}"
        return entry

    raw_path = Path(str(capture_prefix) + ".rgb8")
    png_path = Path(str(capture_prefix) + ".png")
    meta_path = Path(str(capture_prefix) + ".meta.json")
    if not raw_path.is_file() or not meta_path.is_file():
        entry["error"] = f"capture artifacts missing under {REFERENCE_DIR}"
        return entry
    try:
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        entry["error"] = f"capture metadata unreadable: {exc}"
        return entry
    width = meta.get("width")
    height = meta.get("height")
    frame_index = meta.get("capture_frame_index")
    if [width, height] != CAPTURE_RESOLUTION:
        entry["error"] = (
            f"captured frame dimensions {width}x{height} do not match the "
            f"deterministic capture resolution {CAPTURE_RESOLUTION[0]}x{CAPTURE_RESOLUTION[1]}"
        )
        return entry
    data = raw_path.read_bytes()
    if len(data) != width * height * 3:
        entry["error"] = (
            f"raw frame size {len(data)} does not equal width*height*3 "
            f"({width * height * 3})"
        )
        return entry
    if not isinstance(frame_index, (int, float)) or int(frame_index) < CAPTURE_FRAME_INDEX:
        entry["error"] = f"captured frame index {frame_index!r} is malformed or too early"
        return entry

    pixels_path = FRAMES_DIR / f"{scene_name}.pixels.json"
    write_pixels_document(pixels_path, data)

    entry.update(
        {
            "status": "success",
            "frame_index": int(frame_index),
            "width": int(width),
            "height": int(height),
            "format": FRAME_FORMAT,
            "reference_rgb8_path": repo_relative_posix(raw_path),
            "reference_rgb8_sha256": sha256_file(raw_path),
            "reference_rgb8_size_bytes": raw_path.stat().st_size,
            "reference_png_path": repo_relative_posix(png_path) if png_path.is_file() else None,
            "reference_pixels_json_path": repo_relative_posix(pixels_path),
            "reference_pixels_json_sha256": sha256_file(pixels_path),
        }
    )
    return entry


def write_pixels_document(path: Path, rgb8_data: bytes) -> None:
    """Convert raw RGB8 bytes into the {"pixels": [[r,g,b], ...]} channel
    document format read by visual_diff.py."""
    pixels = [
        [rgb8_data[i], rgb8_data[i + 1], rgb8_data[i + 2]]
        for i in range(0, len(rgb8_data), 3)
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps({"pixels": pixels}, separators=(",", ":")) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def build_summary(scenes: list[dict[str, object]]) -> dict[str, object]:
    success_count = sum(1 for scene in scenes if scene.get("status") == "success")
    return {
        "run_mode": "full",
        "evidence_level": "measured_local",
        "target_backend": TARGET_BACKEND,
        "resolution": RESOLUTION,
        "capture_resolution": CAPTURE_RESOLUTION,
        "capture_frame_index": CAPTURE_FRAME_INDEX,
        "frame_format": FRAME_FORMAT,
        "capture_backend_available": capture_backend_available(),
        "godot_full_run_available": godot_full_run_available(),
        "godot_console_exe": str(GODOT_CONSOLE_EXE),
        "capture_kind": "reference_capture_baseline",
        "note": (
            "GRX-007 real reference frame capture: one deterministic frame per "
            "benchmark scene captured from the tracked Godot build (D3D12 "
            "Forward+, fixed-fps, reduced capture resolution). REFERENCE "
            "CAPTURE ONLY: downstream visual evidence self-compares each "
            "reference frame (reference == candidate) to prove the capture/diff "
            "pipeline; this is NOT a Rurix pass visual comparison and no "
            "visual-verification or performance claim about any pass is made."
        ),
        "scene_success_count": success_count,
        "scene_count": len(scenes),
        "scenes": scenes,
    }


def build_visual_evidence_draft(scenes: list[dict[str, object]]) -> dict[str, object]:
    """Build the visual_diff.py evidence document for the captured frames.

    Every scene frame self-compares (reference == candidate == the captured
    reference pixels document). This proves the capture + LDR diff pipeline on
    real frames and pins the reference-capture baseline; it is explicitly NOT a
    Rurix-pass-vs-baseline visual comparison.
    """
    evidence_scenes: list[dict[str, object]] = []
    for scene in scenes:
        pixels_path = str(scene["reference_pixels_json_path"])
        evidence_scenes.append(
            {
                "name": scene["name"],
                "capture_frames": [
                    {
                        "frame_index": int(scene["frame_index"]),
                        "status": "pass",
                        "skip_reason": None,
                        "reference_frame_path": pixels_path,
                        "candidate_frame_path": pixels_path,
                        "capture_resolution": CAPTURE_RESOLUTION,
                        "reference_rgb8_path": scene["reference_rgb8_path"],
                        "reference_rgb8_sha256": scene["reference_rgb8_sha256"],
                        # Self-diff is exactly zero by construction; visual_diff.py
                        # --write-output recomputes and overwrites this value.
                        "ldr_diff": {
                            "per_channel_max_abs": [0.0, 0.0, 0.0],
                            "per_channel_mean_abs": [0.0, 0.0, 0.0],
                        },
                        "hdr_diff": None,
                        "temporal_diff": None,
                    }
                ],
            }
        )
    return {
        "run_mode": "full",
        "evidence_level": "measured_local",
        "target_backend": TARGET_BACKEND,
        "resolution": RESOLUTION,
        "capture_resolution": CAPTURE_RESOLUTION,
        "evidence_kind": "reference_capture_baseline_self_diff",
        "note": (
            "GRX-007 reference-capture baseline evidence: each scene's captured "
            "reference frame is diffed against ITSELF (reference == candidate) "
            "to prove the real capture backend + LDR diff pipeline end to end. "
            "This is a capture baseline document, NOT a Rurix pass vs baseline "
            "visual comparison; no pass visual verification or performance "
            "claim is made."
        ),
        "scenes": evidence_scenes,
    }


def generate_visual_evidence(scenes: list[dict[str, object]]) -> bool:
    """Write the self-diff visual evidence via visual_diff.py --write-output."""
    draft_path = VISUAL_DIR / "visual_reference_capture_evidence.draft.json"
    write_json(draft_path, build_visual_evidence_draft(scenes))
    proc = subprocess.run(
        [
            sys.executable,
            str(VISUAL_DIFF_SCRIPT),
            str(draft_path),
            "--write-output",
            str(VISUAL_EVIDENCE_PATH),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    output = (proc.stdout + proc.stderr).strip()
    if output:
        print(output)
    if proc.returncode != 0:
        print(
            "[visual-capture] FAIL visual_diff.py could not compute the "
            "self-diff evidence for the captured frames",
            file=sys.stderr,
        )
        return False
    check = subprocess.run(
        [sys.executable, str(VISUAL_DIFF_SCRIPT), str(VISUAL_EVIDENCE_PATH)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if check.returncode != 0:
        print((check.stdout + check.stderr).strip(), file=sys.stderr)
        print(
            "[visual-capture] FAIL written visual evidence failed "
            "visual_diff.py re-verification",
            file=sys.stderr,
        )
        return False
    print(f"[visual-capture] visual_evidence_path: {VISUAL_EVIDENCE_PATH}")
    return True


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="print the capture plan without running Godot or writing artifacts",
    )
    args = parser.parse_args()

    if args.validate_only:
        for scene_name in EXPECTED_SCENES:
            print(
                f"[visual-capture] PLAN {scene_name}: capture frame "
                f"{CAPTURE_FRAME_INDEX} at "
                f"{CAPTURE_RESOLUTION[0]}x{CAPTURE_RESOLUTION[1]} via "
                "res://scenes/capture_runner.tscn"
            )
        print("[visual-capture] validate-only: no Godot run, no artifacts written")
        return 0

    if not capture_backend_available():
        print(
            "[visual-capture] FAIL tracked Godot console exe missing at "
            f"{GODOT_CONSOLE_EXE}; cannot run the real capture backend",
            file=sys.stderr,
        )
        return 1
    if not bench_project_available():
        print(
            "[visual-capture] FAIL generated bench project unavailable; run "
            "py -3 spike/godot-rurix/bench/generate_benchmark_project.py first",
            file=sys.stderr,
        )
        return 1

    install_capture_runner()
    REFERENCE_DIR.mkdir(parents=True, exist_ok=True)
    FRAMES_DIR.mkdir(parents=True, exist_ok=True)

    scenes: list[dict[str, object]] = []
    for scene_name in EXPECTED_SCENES:
        entry = capture_scene(scene_name)
        scenes.append(entry)
        if entry["status"] == "success":
            print(
                f"[visual-capture] SUCCESS {scene_name}: frame "
                f"{entry['frame_index']} {entry['width']}x{entry['height']} "
                f"sha256={str(entry['reference_rgb8_sha256'])[:16]}..."
            )
        else:
            print(
                f"[visual-capture] FAIL {scene_name}: {entry['error']}",
                file=sys.stderr,
            )

    summary = build_summary(scenes)
    write_json(CAPTURE_SUMMARY_PATH, summary)
    print(f"[visual-capture] capture_summary_path: {CAPTURE_SUMMARY_PATH}")

    failure_count = sum(1 for scene in scenes if scene.get("status") != "success")
    if failure_count:
        print(
            f"[visual-capture] FAIL {failure_count} scene(s) failed to capture",
            file=sys.stderr,
        )
        return 1
    if not generate_visual_evidence(scenes):
        return 1
    print(
        "[visual-capture] all seven scenes captured (reference-capture baseline "
        "only; not a Rurix pass visual comparison)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
