#!/usr/bin/env python3
"""Plan GRX-007 reference/candidate frame captures for the benchmark scenes.

GRX-007 is scaffold only. This tool defines the capture plan (seven fixed
scenes, at least one capture frame each) and probes whether a capture backend,
a Godot full run, and frame artifacts are available. Nothing is captured yet, so
every scene frame is reported as SKIP with a concrete missing-input reason. No
reference frames are fabricated. The plan/skip evidence is written to
target/grx/godot-visual/capture_summary.json for downstream visual_diff.py.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
BENCH_DIR = Path(__file__).resolve().parent
TARGET_GRX_DIR = ROOT / "target" / "grx"
VISUAL_DIR = TARGET_GRX_DIR / "godot-visual"
CAPTURE_SUMMARY_PATH = VISUAL_DIR / "capture_summary.json"
RUNS_DIR = TARGET_GRX_DIR / "godot-bench-runs"

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
# Fixed capture frame per scene: a single frame captured after warmup.
CAPTURE_FRAME_INDEX = 600
GODOT_CONSOLE_EXE = (
    ROOT / "external" / "godot-master" / "bin" / "godot.windows.template_debug.x86_64.console.exe"
)


def capture_backend_available() -> bool:
    """No dedicated frame capture backend is wired up in the scaffold stage."""
    return False


def godot_full_run_available() -> bool:
    """A Godot full run is available only if the console executable exists."""
    return GODOT_CONSOLE_EXE.exists()


def frame_artifact_available(scene_name: str) -> bool:
    """No captured frame artifacts exist at the scaffold stage."""
    del scene_name
    return False


def determine_skip_reason(scene_name: str) -> str:
    if not capture_backend_available():
        return "missing capture backend"
    if not godot_full_run_available():
        return "missing Godot full run"
    if not frame_artifact_available(scene_name):
        return "missing frame artifact"
    return ""


def build_plan() -> dict[str, object]:
    scenes: list[dict[str, object]] = []
    for scene_name in EXPECTED_SCENES:
        skip_reason = determine_skip_reason(scene_name)
        scenes.append(
            {
                "name": scene_name,
                "capture_frames": [
                    {
                        "frame_index": CAPTURE_FRAME_INDEX,
                        "status": "skip",
                        "skip_reason": skip_reason,
                        "reference_frame_path": None,
                        "candidate_frame_path": None,
                        "ldr_diff": None,
                        "hdr_diff": None,
                        "temporal_diff": None,
                    }
                ],
            }
        )
    return {
        "run_mode": "scaffold",
        "evidence_level": "scaffold",
        "target_backend": TARGET_BACKEND,
        "resolution": RESOLUTION,
        "capture_backend_available": capture_backend_available(),
        "godot_full_run_available": godot_full_run_available(),
        "note": (
            "GRX-007 scaffold capture plan. No frames are captured; every scene is "
            "SKIP with a concrete missing-input reason. Not visual verification."
        ),
        "scenes": scenes,
    }


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="print the capture plan without writing the capture summary artifact",
    )
    args = parser.parse_args()

    plan = build_plan()
    for scene in plan["scenes"]:
        assert isinstance(scene, dict)
        frame = scene["capture_frames"][0]
        print(f"[visual-capture] SKIP {scene['name']}: {frame['skip_reason']}")

    if args.validate_only:
        print("[visual-capture] validate-only: capture summary not written")
        return 0

    write_json(CAPTURE_SUMMARY_PATH, plan)
    print(f"[visual-capture] capture_summary_path: {CAPTURE_SUMMARY_PATH}")
    print(
        "[visual-capture] all seven scenes are SKIP; no reference frames captured, "
        "no visual verification claimed"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
