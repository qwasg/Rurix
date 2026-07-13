#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-024 bench-scene visual parity evidence (2026-07-13, measured_local).

Fixed-frame RGB8 capture of the seven benchmark scenes under two rendering
legs on the SAME rb4 exe / v2.3 project as the terminal rd_native campaign:

  * baseline leg  = backend=0 (native passes), no override.cfg
  * all5 leg      = the five rd_native passes at backend=2 (override.cfg +
                    RURIX_DXC_DIR on PATH), --verbose so the one-shot
                    RXGD_RD_NATIVE_<pass> active markers can be scanned

For temporal / non-deterministic scenes a determinism-FLOOR control is also
captured: a SECOND baseline leg (A'), so each scene reports both
  diff(A, A')  = intrinsic run-to-run non-determinism floor, and
  diff(A, B)   = baseline-vs-all5 parity signal.
A parity diff at or below the determinism floor is honest evidence of parity;
a parity diff well above the floor is recorded as real divergence, not hidden.

Capture path is deterministic: --fixed-fps 60, a fixed post-warmup frame index,
viewport grabbed via get_viewport().get_texture().get_image() as RGB8. This is
VISUAL PARITY EVIDENCE ONLY (GRX-024 material); it makes NO performance claim.

Usage:
  py -3 grx024_visual_capture.py                 # all 7 scenes, 3 legs each
  py -3 grx024_visual_capture.py --scenes particles
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BENCH_DIR = HERE.parents[0]
ROOT = HERE.parents[3]
TARGET_GRX = ROOT / "target" / "grx"
PROJECT_DIR = TARGET_GRX / "godot-bench-project"
GODOT_EXE = (
    TARGET_GRX / "godot-scratch-rb4" / "bin"
    / "godot.windows.template_debug.x86_64.console.exe"
)
DXC_DIR = r"H:\dxc-round7\extracted\bin\x64"
ALL5_MATRIX = HERE / "rd_native_all5.json"
CAPTURES_DIR = HERE / "captures"
OVERRIDE_CFG = PROJECT_DIR / "override.cfg"

CAPTURE_RESOLUTION = [960, 540]
CAPTURE_FRAME_INDEX = 600
GODOT_TIMEOUT = 300
FRAME_FORMAT = "R8G8B8_raw"

SCENES = [
    "clustered_lights",
    "many_mesh_instances",
    "material_variants",
    "post_fx_chain",
    "volumetric_fog",
    "particles",
    "mixed_forward_plus",
]
RD_PASSES = ["tonemap", "ssao_blur", "taa_resolve", "particles_copy", "cluster_store"]

ALLOWLIST_ERR = "ERROR: Could not load global script cache."
ALLOWLIST_CTX = "at: ProjectSettings::get_global_class_list"
FAILURE_MARKERS = (
    "SCRIPT ERROR:", "Parser Error:", "Parse Error:",
    "Failed loading resource:", "Failed loading script",
)

CAPTURE_RUNNER_SCENE = """\
[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://scripts/grx024_capture_runner.gd" id="1"]

[node name="Grx024CaptureRoot" type="Node"]
script = ExtResource("1")

[node name="SceneContainer" type="Node" parent="."]
"""

CAPTURE_RUNNER_SCRIPT = f"""\
extends Node

const CAPTURE_WIDTH := {CAPTURE_RESOLUTION[0]}
const CAPTURE_HEIGHT := {CAPTURE_RESOLUTION[1]}

@onready var scene_container: Node = $SceneContainer

var scene_path := ""
var capture_prefix := ""
var capture_frame := {CAPTURE_FRAME_INDEX}
var processed_frames := 0
var capture_started := false


func _ready() -> void:
    var args := OS.get_cmdline_user_args()
    var index := 0
    while index + 1 < args.size():
        match args[index]:
            "--scene-path":
                scene_path = args[index + 1]
            "--capture-prefix":
                capture_prefix = args[index + 1]
            "--capture-frame":
                capture_frame = int(args[index + 1])
        index += 2
    if scene_path.is_empty() or capture_prefix.is_empty() or capture_frame <= 0:
        printerr("[grx024-capture] ERROR missing/invalid arguments")
        get_tree().quit(3)
        return
    var window := get_window()
    if window != null:
        window.size = Vector2i(CAPTURE_WIDTH, CAPTURE_HEIGHT)
    DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
    var packed := load(scene_path)
    if packed == null:
        printerr("[grx024-capture] ERROR failed to load scene: %s" % scene_path)
        get_tree().quit(3)
        return
    var loaded: Node = packed.instantiate()
    scene_container.add_child(loaded)
    print("[grx024-capture] scene=%s capture_frame=%d" % [scene_path, capture_frame])
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
        printerr("[grx024-capture] ERROR cannot open %s.rgb8" % capture_prefix)
        get_tree().quit(3)
        return
    raw.store_buffer(img.get_data())
    raw.close()
    img.save_png(capture_prefix + ".png")
    var meta := FileAccess.open(capture_prefix + ".meta.json", FileAccess.WRITE)
    meta.store_string(JSON.stringify({{
        "width": img.get_width(),
        "height": img.get_height(),
        "format": "{FRAME_FORMAT}",
        "capture_frame_index": processed_frames,
    }}))
    meta.close()
    print("[grx024-capture] captured frame=%d %dx%d" % [processed_frames, img.get_width(), img.get_height()])
    get_tree().quit(0)
"""


def _load_runner_module():
    spec = importlib.util.spec_from_file_location(
        "grx_bench_runner", BENCH_DIR / "run_benchmark_scenes.py"
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


RUNNER = _load_runner_module()


def now() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def install_runner() -> None:
    (PROJECT_DIR / "scripts").mkdir(parents=True, exist_ok=True)
    (PROJECT_DIR / "scenes").mkdir(parents=True, exist_ok=True)
    (PROJECT_DIR / "scripts" / "grx024_capture_runner.gd").write_text(
        CAPTURE_RUNNER_SCRIPT, encoding="utf-8", newline="\n")
    (PROJECT_DIR / "scenes" / "grx024_capture_runner.tscn").write_text(
        CAPTURE_RUNNER_SCENE, encoding="utf-8", newline="\n")


def scan_markers(output: str) -> tuple[list[str], list[str]]:
    lines = output.replace("\r\n", "\n").splitlines()
    fails, warns, i = [], [], 0
    while i < len(lines):
        ln = lines[i].strip()
        if not ln:
            i += 1
            continue
        if ln == ALLOWLIST_ERR:
            warns.append(ln)
            if i + 1 < len(lines) and lines[i + 1].strip().startswith(ALLOWLIST_CTX):
                warns.append(lines[i + 1].strip())
                i += 1
            i += 1
            continue
        if any(m in ln for m in FAILURE_MARKERS) or "ERROR:" in ln:
            fails.append(ln)
        i += 1
    return fails, warns


def scan_engagement(output: str) -> dict[str, bool]:
    """One-shot rd_native active markers, using the runner's own marker table
    (RXGD_RD_NATIVE_<PASS> active) so this scan never drifts from the runner."""
    text = output.replace("\r\n", "\n")
    markers = RUNNER.RD_NATIVE_ACTIVE_MARKERS
    return {p: (markers.get(p) is not None and markers[p] in text) for p in RD_PASSES}


def capture_leg(scene: str, leg: str, with_matrix: bool, verbose: bool) -> dict:
    prefix = CAPTURES_DIR / leg / scene
    prefix.parent.mkdir(parents=True, exist_ok=True)
    for suf in (".rgb8", ".png", ".meta.json"):
        Path(str(prefix) + suf).unlink(missing_ok=True)

    cmd = [
        str(GODOT_EXE),
        "--path", str(PROJECT_DIR),
        "--rendering-driver", "d3d12",
        "--rendering-method", "forward_plus",
        "--resolution", f"{CAPTURE_RESOLUTION[0]}x{CAPTURE_RESOLUTION[1]}",
        "--fixed-fps", "60",
    ]
    if verbose:
        cmd.append("--verbose")
    cmd += [
        "--scene", "res://scenes/grx024_capture_runner.tscn",
        "--",
        "--scene-path", f"res://scenes/{scene}.tscn",
        "--capture-prefix", str(prefix),
        "--capture-frame", str(CAPTURE_FRAME_INDEX),
    ]
    env = dict(os.environ)
    if with_matrix and Path(DXC_DIR).is_dir():
        env["PATH"] = DXC_DIR + os.pathsep + env.get("PATH", "")

    entry: dict = {"scene": scene, "leg": leg, "status": "fail"}
    try:
        proc = subprocess.run(cmd, cwd=str(PROJECT_DIR), capture_output=True,
                              text=True, timeout=GODOT_TIMEOUT, check=False, env=env)
    except subprocess.TimeoutExpired:
        entry["error"] = f"timeout {GODOT_TIMEOUT}s"
        return entry
    output = "\n".join(x for x in (proc.stdout, proc.stderr) if x)
    fails, warns = scan_markers(output)
    entry["exit_code"] = proc.returncode
    entry["failure_markers"] = fails
    entry["warning_count"] = len(warns)
    if with_matrix:
        entry["engagement"] = scan_engagement(output)
    if proc.returncode != 0:
        entry["error"] = f"exit {proc.returncode}"
        entry["stdout_tail"] = output[-1500:]
        return entry
    if fails:
        entry["error"] = f"failure marker: {fails[0]}"
        return entry
    raw = Path(str(prefix) + ".rgb8")
    meta_p = Path(str(prefix) + ".meta.json")
    if not raw.is_file() or not meta_p.is_file():
        entry["error"] = "capture artifacts missing"
        return entry
    meta = json.loads(meta_p.read_text(encoding="utf-8"))
    w, h = meta.get("width"), meta.get("height")
    data = raw.read_bytes()
    if [w, h] != CAPTURE_RESOLUTION or len(data) != w * h * 3:
        entry["error"] = f"dim/size mismatch {w}x{h} bytes={len(data)}"
        return entry
    entry.update({
        "status": "success",
        "width": w, "height": h,
        "frame_index": int(meta.get("capture_frame_index", 0)),
        "rgb8_path": str(raw.relative_to(ROOT).as_posix()),
        "rgb8_sha256": sha256_file(raw),
        "png_path": str(Path(str(prefix) + ".png").relative_to(ROOT).as_posix()),
    })
    return entry


def diff_rgb8(path_a: Path, path_b: Path) -> dict:
    import numpy as np
    a = np.frombuffer(path_a.read_bytes(), dtype=np.uint8)
    b = np.frombuffer(path_b.read_bytes(), dtype=np.uint8)
    if a.size != b.size:
        return {"error": f"size mismatch {a.size} vs {b.size}"}
    n = int(a.size)
    d = np.abs(a.astype(np.int16) - b.astype(np.int16))
    dp = d.reshape(-1, 3)
    npx = int(dp.shape[0])
    px_diff = int(np.count_nonzero(dp.any(axis=1)))
    hist = {
        "0": int(np.count_nonzero(d == 0)),
        "1": int(np.count_nonzero(d == 1)),
        "2_4": int(np.count_nonzero((d >= 2) & (d <= 4))),
        "5_16": int(np.count_nonzero((d >= 5) & (d <= 16))),
        "17plus": int(np.count_nonzero(d >= 17)),
    }
    return {
        "bytes": n,
        "pixels": npx,
        "max_abs": int(d.max()),
        "mean_abs": round(float(d.mean()), 5),
        "channel_max_abs": [int(x) for x in dp.max(axis=0)],
        "channel_mean_abs": [round(float(x), 5) for x in dp.mean(axis=0)],
        "pixels_differing": px_diff,
        "pct_pixels_differing": round(100.0 * px_diff / npx, 4),
        "abs_diff_histogram_bytes": hist,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--scenes", type=str, default=",".join(SCENES))
    args = ap.parse_args()
    scenes = [s.strip() for s in args.scenes.split(",") if s.strip()]

    for req in (GODOT_EXE, ALL5_MATRIX):
        if not Path(req).exists():
            print(f"missing required asset: {req}", file=sys.stderr)
            return 2
    if OVERRIDE_CFG.exists():
        print(f"stale override.cfg present, refusing: {OVERRIDE_CFG}", file=sys.stderr)
        return 2

    install_runner()
    matrix = RUNNER.load_pass_matrix(ALL5_MATRIX)

    summary = {
        "tool": "grx024_visual_capture.py",
        "generated_utc": now(),
        "evidence_level": "measured_local",
        "performance_claim": "none",
        "godot_exe": str(GODOT_EXE),
        "godot_exe_sha256": sha256_file(GODOT_EXE),
        "dll_sha256": sha256_file(ROOT / "target" / "debug" / "rurix_godot.dll"),
        "capture_resolution": CAPTURE_RESOLUTION,
        "capture_frame_index": CAPTURE_FRAME_INDEX,
        "fixed_fps": 60,
        "legs": ["baseline_a", "baseline_b(floor)", "all5"],
        "scenes": [],
    }

    for scene in scenes:
        print(f"[{now()}] === {scene} ===", flush=True)
        # A, A' baselines (no override.cfg), then all5 (override.cfg).
        a = capture_leg(scene, "baseline_a", with_matrix=False, verbose=False)
        print(f"  baseline_a: {a['status']} {a.get('error','')}", flush=True)
        ap_ = capture_leg(scene, "baseline_b", with_matrix=False, verbose=False)
        print(f"  baseline_b: {ap_['status']} {ap_.get('error','')}", flush=True)

        override_path = RUNNER.write_override_cfg(PROJECT_DIR, matrix)
        try:
            b = capture_leg(scene, "all5", with_matrix=True, verbose=True)
        finally:
            override_path.unlink(missing_ok=True)
        eng = b.get("engagement", {})
        engaged = [p for p, v in eng.items() if v]
        print(f"  all5: {b['status']} {b.get('error','')} engaged={engaged}", flush=True)

        entry = {"scene": scene, "baseline_a": a, "baseline_b": ap_, "all5": b,
                 "engaged_passes": engaged}
        if a["status"] == "success" and ap_["status"] == "success":
            entry["determinism_floor_AA"] = diff_rgb8(
                ROOT / a["rgb8_path"], ROOT / ap_["rgb8_path"])
        if a["status"] == "success" and b["status"] == "success":
            entry["parity_AB"] = diff_rgb8(
                ROOT / a["rgb8_path"], ROOT / b["rgb8_path"])
        summary["scenes"].append(entry)

    out = HERE / "grx024_visual_summary.json"
    out.write_text(json.dumps(summary, indent=2, ensure_ascii=True) + "\n",
                   encoding="utf-8", newline="\n")
    print(f"[{now()}] wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
