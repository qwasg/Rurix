#!/usr/bin/env python3
"""Smoke checks for the Rurix Godot D3D12 Forward+ bridge scaffold."""

from __future__ import annotations

import pathlib
import subprocess
import sys

from godot_rurix_patch_stack import evaluate_patch_stack


ROOT = pathlib.Path(__file__).resolve().parents[1]
HEADER = ROOT / "src" / "rurix-godot" / "include" / "rurix_godot.h"
LIB = ROOT / "src" / "rurix-godot" / "src" / "lib.rs"
PATCH = ROOT / "spike" / "godot-rurix" / "patches" / "0001-rurix-accel-module-scaffold.patch"
PATCH2 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0002-rurix-accel-luminance-pass-gate.patch"
)
PATCH3 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0003-rurix-accel-luminance-core-callsite-wiring.patch"
)
EXTERNAL_GODOT = ROOT / "external" / "godot-master"

IDE_IGNORE_PROBES = [
    ".cursor/settings.json",
    ".kiro/state.json",
    ".kimi/state.json",
    ".trae/state.json",
    ".claude/state.json",
    ".vscode/settings.json",
    ".idea/workspace.xml",
    ".windsurf/state.json",
    ".zed/settings.json",
]


ABI_SYMBOLS = [
    "rxgd_abi_version",
    "rxgd_create_d3d12_session",
    "rxgd_register_texture",
    "rxgd_register_buffer",
    "rxgd_record_pass",
    "rxgd_collect_timestamps",
    "rxgd_destroy_session",
]


def run(cmd: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    print("[godot-rurix]", " ".join(cmd))
    return subprocess.run(cmd, cwd=ROOT, text=True, check=check)


def run_capture(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    print("[godot-rurix]", " ".join(cmd))
    return subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, check=False)


def require_text(path: pathlib.Path, needles: list[str]) -> None:
    text = path.read_text(encoding="utf-8")
    missing = [needle for needle in needles if needle not in text]
    if missing:
        raise SystemExit(f"{path} missing required text: {missing}")


def check_external_ignored() -> None:
    if not EXTERNAL_GODOT.exists():
        raise SystemExit(f"expected ignored Godot snapshot at {EXTERNAL_GODOT}")
    run(["git", "check-ignore", "-q", "external/godot-master"])
    status = subprocess.check_output(
        ["git", "status", "--porcelain", "--", "external/godot-master"],
        cwd=ROOT,
        text=True,
    ).strip()
    if status:
        raise SystemExit(f"external/godot-master must stay untracked, got: {status}")


def check_local_state_ignored() -> None:
    for probe in IDE_IGNORE_PROBES:
        run(["git", "check-ignore", "-q", "--no-index", probe])

    tracked = subprocess.check_output(
        [
            "git",
            "ls-files",
            ".cursor",
            ".kiro",
            ".kimi",
            ".trae",
            ".claude",
            ".vscode",
            ".idea",
            ".windsurf",
            ".zed",
        ],
        cwd=ROOT,
        text=True,
    ).strip()
    if tracked:
        raise SystemExit(f"local IDE/agent state must stay untracked, got: {tracked}")


def check_patch_state() -> str:
    result = evaluate_patch_stack(ROOT, EXTERNAL_GODOT, PATCH, PATCH2, PATCH3)
    if result["ok"] is not True:
        details = result.get("details", {})
        raise SystemExit(
            f"{result['reason']}.\n"
            f"details: {details}"
        )
    state = str(result["state"])
    print(f"[godot-rurix] patch state: {state}")
    return state


def main() -> int:
    require_text(HEADER, ABI_SYMBOLS + ["RXGD_BACKEND_D3D12", "RXGD_RENDER_METHOD_FORWARD_PLUS"])
    require_text(
        LIB,
        ABI_SYMBOLS
        + [
            "RXGD_PASS_FUSED_POST_CHAIN",
            "RXGD_STATUS_FALLBACK",
            "LuminanceReductionGate",
        ],
    )
    require_text(
        PATCH,
        [
            "modules/rurix_accel",
            "D3D12Hooks",
            "rxgd_create_d3d12_session",
            "module_rurix_accel_enabled",
        ],
    )
    require_text(
        PATCH2,
        [
            "modules/rurix_accel",
            "rendering/rurix_accel/passes/luminance_reduction/enabled",
            "RXGD_PASS_LUMINANCE_REDUCTION",
            "rxgd_record_pass",
            "try_record_luminance_reduction",
        ],
    )
    require_text(
        PATCH3,
        [
            "drivers/d3d12/d3d12_hooks.h",
            "renderer_scene_render_rd.cpp",
            "D3D12Hooks::get_singleton",
            "try_record_luminance_reduction",
            "override",
            "luminance_reduction",
        ],
    )
    check_external_ignored()
    check_local_state_ignored()
    check_patch_state()
    run(["cargo", "test", "-p", "rurix-godot"])
    print("[godot-rurix] PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
