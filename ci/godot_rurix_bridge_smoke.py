#!/usr/bin/env python3
"""Smoke checks for the Rurix Godot D3D12 Forward+ bridge scaffold."""

from __future__ import annotations

import pathlib
import subprocess
import sys

from godot_rurix_patch_stack import (
    evaluate_followup_patch_applyability,
    evaluate_patch_stack,
    evaluate_stacked_patch_applyability,
)


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
PATCH4 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0004-rurix-accel-luminance-resource-mapping-scaffold.patch"
)
PATCH5 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0005-rurix-accel-luminance-runtime-binding-preflight.patch"
)
PATCH6 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0006-rurix-accel-luminance-gated-dispatch-bringup.patch"
)
PATCH7 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0007-rurix-accel-luminance-native-resource-handle-mapping.patch"
)
PATCH8 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0008-rurix-accel-luminance-godot-runtime-bridge-recording-smoke.patch"
)
PATCH9 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0009-rurix-accel-luminance-real-pass-optin.patch"
)
PATCH10 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0010-rurix-accel-luminance-real-pass-result-writeback.patch"
)
PATCH11 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0011-rurix-accel-tonemap-pass-gate-and-callsite.patch"
)
PATCH12 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0012-rurix-accel-tonemap-runtime-resource-binding.patch"
)
PATCH13 = (
    ROOT
    / "spike"
    / "godot-rurix"
    / "patches"
    / "0013-rurix-accel-tonemap-recording-smoke-and-real-pass-optin.patch"
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
    if state == "0001+0002+0003":
        patch4 = evaluate_followup_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            PATCH4,
            "0004",
        )
        if patch4["ok"] is not True:
            details = patch4.get("details", {})
            raise SystemExit(
                f"{patch4['reason']}; fix "
                "0004-rurix-accel-luminance-resource-mapping-scaffold.patch "
                "so git apply --check passes.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0004 applyability: ready")
        patch5 = evaluate_stacked_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            [PATCH4],
            PATCH5,
            "0005",
        )
        if patch5["ok"] is not True:
            details = patch5.get("details", {})
            raise SystemExit(
                f"{patch5['reason']}; fix "
                "0005-rurix-accel-luminance-runtime-binding-preflight.patch "
                "so it applies after 0004 in a scratch copy.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0005 stacked applyability: ready")
        patch6 = evaluate_stacked_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            [PATCH4, PATCH5],
            PATCH6,
            "0006",
        )
        if patch6["ok"] is not True:
            details = patch6.get("details", {})
            raise SystemExit(
                f"{patch6['reason']}; fix "
                "0006-rurix-accel-luminance-gated-dispatch-bringup.patch "
                "so it applies after 0004+0005 in a scratch copy.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0006 stacked applyability: ready")
        patch7 = evaluate_stacked_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            [PATCH4, PATCH5, PATCH6],
            PATCH7,
            "0007",
        )
        if patch7["ok"] is not True:
            details = patch7.get("details", {})
            raise SystemExit(
                f"{patch7['reason']}; fix "
                "0007-rurix-accel-luminance-native-resource-handle-mapping.patch "
                "so it applies after 0004+0005+0006 in a scratch copy.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0007 stacked applyability: ready")
        patch8 = evaluate_stacked_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            [PATCH4, PATCH5, PATCH6, PATCH7],
            PATCH8,
            "0008",
        )
        if patch8["ok"] is not True:
            details = patch8.get("details", {})
            raise SystemExit(
                f"{patch8['reason']}; fix "
                "0008-rurix-accel-luminance-godot-runtime-bridge-recording-smoke.patch "
                "so it applies after 0004+0005+0006+0007 in a scratch copy.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0008 stacked applyability: ready")
        patch9 = evaluate_stacked_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            [PATCH4, PATCH5, PATCH6, PATCH7, PATCH8],
            PATCH9,
            "0009",
        )
        if patch9["ok"] is not True:
            details = patch9.get("details", {})
            raise SystemExit(
                f"{patch9['reason']}; fix "
                "0009-rurix-accel-luminance-real-pass-optin.patch "
                "so it applies after 0004+0005+0006+0007+0008 in a scratch copy.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0009 stacked applyability: ready")
        patch10 = evaluate_stacked_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            [PATCH4, PATCH5, PATCH6, PATCH7, PATCH8, PATCH9],
            PATCH10,
            "0010",
        )
        if patch10["ok"] is not True:
            details = patch10.get("details", {})
            raise SystemExit(
                f"{patch10['reason']}; fix "
                "0010-rurix-accel-luminance-real-pass-result-writeback.patch "
                "so it applies after 0004..0009 in a scratch copy.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0010 stacked applyability: ready")
        patch11 = evaluate_stacked_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            [PATCH4, PATCH5, PATCH6, PATCH7, PATCH8, PATCH9, PATCH10],
            PATCH11,
            "0011",
        )
        if patch11["ok"] is not True:
            details = patch11.get("details", {})
            raise SystemExit(
                f"{patch11['reason']}; fix "
                "0011-rurix-accel-tonemap-pass-gate-and-callsite.patch "
                "so it applies after 0004..0010 in a scratch copy.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0011 stacked applyability: ready")
        patch12 = evaluate_stacked_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            [PATCH4, PATCH5, PATCH6, PATCH7, PATCH8, PATCH9, PATCH10, PATCH11],
            PATCH12,
            "0012",
        )
        if patch12["ok"] is not True:
            details = patch12.get("details", {})
            raise SystemExit(
                f"{patch12['reason']}; fix "
                "0012-rurix-accel-tonemap-runtime-resource-binding.patch "
                "so it applies after 0004..0011 in a scratch copy.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0012 stacked applyability: ready")
        patch13 = evaluate_stacked_patch_applyability(
            ROOT,
            EXTERNAL_GODOT,
            [
                PATCH4,
                PATCH5,
                PATCH6,
                PATCH7,
                PATCH8,
                PATCH9,
                PATCH10,
                PATCH11,
                PATCH12,
            ],
            PATCH13,
            "0013",
        )
        if patch13["ok"] is not True:
            details = patch13.get("details", {})
            raise SystemExit(
                f"{patch13['reason']}; fix "
                "0013-rurix-accel-tonemap-recording-smoke-and-real-pass-optin.patch "
                "so it applies after 0004..0012 in a scratch copy.\n"
                f"details: {details}"
            )
        print("[godot-rurix] patch 0013 stacked applyability: ready")
    return state


def main() -> int:
    require_text(
        HEADER,
        ABI_SYMBOLS
        + [
            "RXGD_BACKEND_D3D12",
            "RXGD_RENDER_METHOD_FORWARD_PLUS",
            "RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP",
            "RXGD_CAP_LUMINANCE_REAL_PASS",
            "RXGD_CAP_TONEMAP_REAL_PASS",
        ],
    )
    require_text(
        LIB,
        ABI_SYMBOLS
        + [
            "RXGD_PASS_FUSED_POST_CHAIN",
            "RXGD_STATUS_FALLBACK",
            "LuminanceReductionGate",
            "record_runtime_binding_preflight",
            "RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP",
            "record_gated_dispatch_bringup",
            "check_dispatch_eligibility",
            "LuminanceDispatchPackage",
            "RXGD_CAP_LUMINANCE_REAL_PASS",
            "record_real_pass_attempt",
            "check_real_pass_binding_kind",
            "RXGD_REAL_PASS_BLOCKED",
            "TonemapGate",
            "TonemapDispatchPackage",
            "RXGD_CAP_TONEMAP_REAL_PASS",
            "RXGD_TONEMAP_REAL_PASS_BLOCKED",
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
    require_text(
        PATCH4,
        [
            "resource mapping scaffold",
            "RXGD_PASS_LUMINANCE_REDUCTION",
            "source_width",
            "source_height",
            "max_luminance",
            "min_luminance",
            "exposure_adjust",
            "src_luminance = t0",
            "dst_luminance = u0",
            "b0",
            "64-bit integer shader capability",
            "RXGD_STATUS_FALLBACK",
        ],
    )
    require_text(
        PATCH5,
        [
            "runtime binding preflight",
            "RXGD_PASS_LUMINANCE_REDUCTION",
            "RXGD_RESOURCE_TEXTURE",
            "try_record_luminance_reduction",
            "source_width",
            "source_height",
            "max_luminance",
            "min_luminance",
            "exposure_adjust",
            "src_luminance = t0",
            "dst_luminance = u0",
            "b0",
            "64-bit integer shader capability",
            "RXGD_STATUS_FALLBACK",
            "no D3D12 dispatch is recorded",
        ],
    )
    require_text(
        PATCH6,
        [
            "gated dispatch bring-up",
            "rendering/rurix_accel/passes/luminance_reduction/dispatch_bringup",
            "RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP",
            "caps.flags |= RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP",
            "GLOBAL_DEF_BASIC",
            "RXGD_STATUS_FALLBACK",
            "no D3D12 dispatch is recorded",
            "default",
        ],
    )
    require_text(
        PATCH7,
        [
            "native resource handle mapping",
            "RenderingDevice::get_driver_resource",
            "DRIVER_RESOURCE_TEXTURE",
            "ID3D12Resource*",
            "p_source_native_handle",
            "p_dest_native_handle",
            "rb->get_internal_texture()",
            "luminance_buffers->reduce[0]",
            "native Godot luminance path",
            "RXGD_STATUS_FALLBACK",
            "does not set RXGD_CAP_LUMINANCE_DISPATCH_RECORD",
        ],
    )
    require_text(
        PATCH8,
        [
            "rendering/rurix_accel/passes/luminance_reduction/dispatch_recording_smoke",
            "RXGD_CAP_LUMINANCE_DISPATCH_RECORD",
            "caps.flags |= RXGD_CAP_LUMINANCE_DISPATCH_RECORD",
            "d3d12-recording-shim",
            "RXGD_GODOT_RUNTIME_LUMINANCE_RECORD",
            "RXGD_STATUS_FALLBACK",
            "test-only",
            "default",
        ],
    )
    require_text(
        PATCH11,
        [
            "rendering/rurix_accel/passes/tonemap/enabled",
            "RXGD_PASS_TONEMAP",
            "try_record_tonemap",
            "tone_mapper->tonemapper",
            "RXGD_STATUS_FALLBACK",
            "default",
        ],
    )
    require_text(
        PATCH12,
        [
            "RenderingDevice::get_driver_resource",
            "ID3D12Resource*",
            "try_record_tonemap",
            "RXGD_PASS_TONEMAP",
            "RXGD_STATUS_FALLBACK",
        ],
    )
    require_text(
        PATCH13,
        [
            "rendering/rurix_accel/passes/tonemap/dispatch_real_pass",
            "rendering/rurix_accel/passes/tonemap/dispatch_recording_smoke",
            "rendering/rurix_accel/passes/tonemap/real_pass_force_capability_downgrade",
            "RXGD_CAP_TONEMAP_REAL_PASS",
            "RXGD_GODOT_RUNTIME_TONEMAP_RECORD",
            "d3d12-recording-shim",
            "RXGD_STATUS_FALLBACK",
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
